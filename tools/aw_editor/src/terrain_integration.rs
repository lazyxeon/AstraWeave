#![allow(dead_code)]

use astraweave_terrain::{
    BiomeBlendConfig, BiomeBlender, BiomeConfig, BiomePack, BiomeType, ChunkId, Heightmap,
    HeightmapPatch, PackedBiomeBlend, ScatterConfig, SplatConfig, SplatMapGenerator, SplatRule,
    SplatWeights, TerrainChunk, VegetationInstance, VegetationScatter, WorldConfig, WorldGenerator,
};
use glam::{Vec2, Vec3};
use std::collections::HashMap;

/// Full noise preset for a biome — configures all three noise layers.
pub struct BiomeNoisePreset {
    // Base elevation (Perlin)
    pub base_scale: f64,
    pub base_amplitude: f32,
    pub base_octaves: usize,
    pub base_persistence: f64,
    pub base_lacunarity: f64,
    // Mountains (RidgedMulti)
    pub mountains_enabled: bool,
    pub mountains_scale: f64,
    pub mountains_amplitude: f32,
    pub mountains_octaves: usize,
    // Detail (Billow)
    pub detail_enabled: bool,
    pub detail_scale: f64,
    pub detail_amplitude: f32,
    // Hydraulic erosion
    pub erosion_enabled: bool,
    pub erosion_strength: f32,
}

pub struct TerrainState {
    generator: Option<WorldGenerator>,
    config: WorldConfig,
    generated_chunks: HashMap<ChunkId, GeneratedChunk>,
    terrain_dirty: bool,
    last_seed: u64,
    last_biome: String,
    /// Stable ordering of chunk keys for consistent GPU index mapping
    chunk_order: Vec<ChunkId>,
    /// Indices of chunks modified by the last brush stroke (into chunk_order)
    dirty_chunk_indices: Vec<usize>,
    /// Whether we're in the middle of a brush stroke (for undo snapshot tracking)
    is_stroking: bool,
    /// Pre-stroke heightmap snapshots keyed by ChunkId (captured lazily on first modification)
    stroke_pre_snapshots: HashMap<ChunkId, Vec<f32>>,
    /// Cached BiomePack for pack: selections (avoids re-loading on every configure call)
    cached_pack: Option<(std::path::PathBuf, astraweave_terrain::BiomePack)>,
}

pub struct GeneratedChunk {
    pub chunk: TerrainChunk,
    pub vertices: Vec<TerrainVertex>,
    pub indices: Vec<u32>,
    pub world_position: Vec3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub biome_weights_0: [f32; 4],
    pub biome_weights_1: [f32; 4],
    /// Material texture layer indices (0-21) packed as f32 for vertex attr compat
    pub material_ids: [f32; 4],
    /// Blend weights for each material slot (sum to 1.0)
    pub material_weights: [f32; 4],
}

impl TerrainVertex {
    pub fn new(
        position: [f32; 3],
        normal: [f32; 3],
        uv: [f32; 2],
        biome_weights_0: [f32; 4],
        biome_weights_1: [f32; 4],
        material_ids: [f32; 4],
        material_weights: [f32; 4],
    ) -> Self {
        Self {
            position,
            normal,
            uv,
            biome_weights_0,
            biome_weights_1,
            material_ids,
            material_weights,
        }
    }
}

impl Default for TerrainState {
    fn default() -> Self {
        Self {
            generator: None,
            config: WorldConfig::default(),
            generated_chunks: HashMap::new(),
            terrain_dirty: true,
            last_seed: 0,
            last_biome: String::new(),
            chunk_order: Vec::new(),
            dirty_chunk_indices: Vec::new(),
            is_stroking: false,
            stroke_pre_snapshots: HashMap::new(),
            cached_pack: None,
        }
    }
}

impl TerrainState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the currently cached BiomePack (if any).
    pub fn cached_biome_pack(&self) -> Option<&astraweave_terrain::BiomePack> {
        self.cached_pack.as_ref().map(|(_, pack)| pack)
    }

    pub fn configure(&mut self, seed: u64, primary_biome: &str) {
        if self.last_seed != seed || self.last_biome != primary_biome {
            self.terrain_dirty = true;
            self.last_seed = seed;
            self.last_biome = primary_biome.to_string();
        }

        self.config.seed = seed;
        self.config.biomes = self.biomes_for_primary(primary_biome);
    }

    /// Update the noise generation parameters (octaves, lacunarity, persistence, amplitude).
    pub fn set_noise_params(
        &mut self,
        octaves: usize,
        lacunarity: f64,
        persistence: f64,
        amplitude: f32,
    ) {
        self.config.noise.base_elevation.octaves = octaves;
        self.config.noise.base_elevation.lacunarity = lacunarity;
        self.config.noise.base_elevation.persistence = persistence;
        self.config.noise.base_elevation.amplitude = amplitude;
        self.terrain_dirty = true;
    }

    /// Apply a full biome noise preset that configures all three noise layers.
    pub fn apply_biome_noise_preset(&mut self, preset: &BiomeNoisePreset) {
        // Base elevation
        self.config.noise.base_elevation.scale = preset.base_scale;
        self.config.noise.base_elevation.amplitude = preset.base_amplitude;
        self.config.noise.base_elevation.octaves = preset.base_octaves;
        self.config.noise.base_elevation.persistence = preset.base_persistence;
        self.config.noise.base_elevation.lacunarity = preset.base_lacunarity;

        // Mountains
        self.config.noise.mountains.enabled = preset.mountains_enabled;
        self.config.noise.mountains.scale = preset.mountains_scale;
        self.config.noise.mountains.amplitude = preset.mountains_amplitude;
        self.config.noise.mountains.octaves = preset.mountains_octaves;

        // Detail
        self.config.noise.detail.enabled = preset.detail_enabled;
        self.config.noise.detail.scale = preset.detail_scale;
        self.config.noise.detail.amplitude = preset.detail_amplitude;

        // Hydraulic erosion
        self.config.noise.erosion_enabled = preset.erosion_enabled;
        self.config.noise.erosion_strength = preset.erosion_strength;

        self.terrain_dirty = true;
    }

    fn biomes_for_primary(&mut self, primary: &str) -> Vec<BiomeConfig> {
        // Check if this is a biome-pack reference ("pack:/path/to/file.biomepack.json")
        if let Some(pack_path) = primary.strip_prefix("pack:") {
            let path = std::path::PathBuf::from(pack_path);

            // Use cached pack if same path, otherwise load and cache
            let pack = if self.cached_pack.as_ref().is_some_and(|(p, _)| *p == path) {
                &self.cached_pack.as_ref().unwrap().1
            } else {
                match astraweave_terrain::BiomePack::load(&path) {
                    Ok(loaded) => {
                        self.cached_pack = Some((path.clone(), loaded));
                        &self.cached_pack.as_ref().unwrap().1
                    }
                    Err(_) => {
                        return vec![Self::biome_config_for_type(BiomeType::Grassland)];
                    }
                }
            };

            // Use Desert as base type since most packs are nature/desert scenes;
            // the pack's vegetation overrides the biome anyway.
            let base = pack.name.to_lowercase();
            let biome_type = if base.contains("forest") {
                BiomeType::Forest
            } else if base.contains("tundra") || base.contains("snow") {
                BiomeType::Tundra
            } else if base.contains("mountain") {
                BiomeType::Mountain
            } else if base.contains("swamp") {
                BiomeType::Swamp
            } else {
                BiomeType::Desert
            };
            return vec![pack.to_biome_config(biome_type)];
        }

        let primary_type = primary.parse::<BiomeType>().unwrap_or(BiomeType::Grassland);

        let mut biomes = vec![Self::biome_config_for_type(primary_type)];

        // Only include biomes that are compatible with the primary biome.
        // This prevents snow/tundra textures from appearing in grassland,
        // and grassland textures from appearing in tundra, etc.
        let compatible = Self::compatible_biomes(primary_type);
        for bt in compatible {
            if bt != primary_type {
                biomes.push(Self::biome_config_for_type(bt));
            }
        }
        biomes
    }

    /// Return the set of biome types that make sense alongside the given primary biome.
    fn compatible_biomes(primary: BiomeType) -> Vec<BiomeType> {
        match primary {
            BiomeType::Grassland => vec![
                BiomeType::Grassland,
                BiomeType::Forest,
                BiomeType::River,
                BiomeType::Swamp,
            ],
            BiomeType::Desert => vec![BiomeType::Desert, BiomeType::Beach],
            BiomeType::Forest => vec![
                BiomeType::Forest,
                BiomeType::Grassland,
                BiomeType::River,
                BiomeType::Swamp,
                BiomeType::Mountain,
            ],
            BiomeType::Mountain => vec![
                BiomeType::Mountain,
                BiomeType::Tundra,
                BiomeType::Forest,
                BiomeType::Grassland,
            ],
            BiomeType::Tundra => vec![BiomeType::Tundra, BiomeType::Mountain, BiomeType::River],
            BiomeType::Swamp => vec![
                BiomeType::Swamp,
                BiomeType::River,
                BiomeType::Forest,
                BiomeType::Grassland,
            ],
            BiomeType::Beach => vec![
                BiomeType::Beach,
                BiomeType::Desert,
                BiomeType::Grassland,
                BiomeType::River,
            ],
            BiomeType::River => vec![
                BiomeType::River,
                BiomeType::Grassland,
                BiomeType::Forest,
                BiomeType::Swamp,
            ],
            _ => vec![primary],
        }
    }

    fn biome_config_for_type(bt: BiomeType) -> BiomeConfig {
        match bt {
            BiomeType::Grassland => BiomeConfig::grassland(),
            BiomeType::Desert => BiomeConfig::desert(),
            BiomeType::Forest => BiomeConfig::forest(),
            BiomeType::Mountain => BiomeConfig::mountain(),
            BiomeType::Tundra => BiomeConfig::tundra(),
            BiomeType::Swamp => BiomeConfig::swamp(),
            BiomeType::Beach => BiomeConfig::beach(),
            BiomeType::River => BiomeConfig::river(),
            _ => BiomeConfig::grassland(), // Fallback for future biome types
        }
    }

    pub fn generate_terrain(&mut self, chunk_radius: i32) -> anyhow::Result<usize> {
        self.generator = Some(WorldGenerator::new(self.config.clone()));
        self.generated_chunks.clear();
        let primary_biome = self.primary_biome_type();

        let generator = self
            .generator
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Generator not initialized"))?;

        let chunk_size = self.config.chunk_size;
        let mut count = 0;

        for x in -chunk_radius..=chunk_radius {
            for z in -chunk_radius..=chunk_radius {
                let chunk_id = ChunkId { x, z };

                // Use generate_chunk (heightmap only) instead of
                // generate_chunk_with_scatter which runs extremely expensive
                // O(n²) Poisson disk sampling for vegetation/resources that
                // the editor doesn't render anyway.
                let mut chunk = generator.generate_chunk(chunk_id)?;

                // Override biome map to match the primary biome type.
                // The WorldGenerator scores biomes by climate conditions which
                // may default to Grassland even when Desert is configured.
                // This ensures biome weights in vertices match the splat rules.
                for b in chunk.biome_map_mut() {
                    *b = primary_biome;
                }

                let world_pos = chunk_id.to_world_pos(chunk_size);
                let world_offset = Vec3::new(world_pos.x, 0.0, world_pos.z);

                let (vertices, indices) = Self::generate_heightmap_mesh(
                    chunk.heightmap(),
                    chunk.biome_map(),
                    chunk_size,
                    world_offset,
                    self.config.seed,
                    primary_biome,
                );

                self.generated_chunks.insert(
                    chunk_id,
                    GeneratedChunk {
                        chunk,
                        vertices,
                        indices,
                        world_position: world_offset,
                    },
                );

                count += 1;
            }
        }

        // Build stable chunk ordering for GPU index mapping
        self.chunk_order = self.generated_chunks.keys().copied().collect();
        self.chunk_order
            .sort_by(|a, b| a.x.cmp(&b.x).then(a.z.cmp(&b.z)));
        self.dirty_chunk_indices.clear();

        self.terrain_dirty = false;
        Ok(count)
    }

    fn generate_heightmap_mesh(
        heightmap: &Heightmap,
        biome_map: &[BiomeType],
        chunk_size: f32,
        world_offset: Vec3,
        seed: u64,
        primary_biome: BiomeType,
    ) -> (Vec<TerrainVertex>, Vec<u32>) {
        let resolution = heightmap.resolution() as usize;
        let cell_size = chunk_size / (resolution - 1) as f32;
        let blender = BiomeBlender::new(BiomeBlendConfig::default(), seed);
        let biome_blends = blender.blend_chunk(
            heightmap,
            biome_map,
            chunk_size,
            Vec2::new(world_offset.x, world_offset.z),
        );
        let mut heights = Vec::with_capacity(resolution * resolution);
        let mut normals = Vec::with_capacity(resolution * resolution);
        for z in 0..resolution {
            for x in 0..resolution {
                heights.push(heightmap.get_height(x as u32, z as u32));
                normals.push(Self::calculate_normal(heightmap, x, z, cell_size));
            }
        }
        let splat_generator = Self::create_local_splat_generator(seed, primary_biome);
        let splat_map = splat_generator.generate_splat_map(&heights, &normals, resolution as u32);

        let mut vertices = Vec::with_capacity(resolution * resolution);
        let mut indices = Vec::with_capacity((resolution - 1) * (resolution - 1) * 6);

        for z in 0..resolution {
            for x in 0..resolution {
                let biome_idx = z * resolution + x;
                let height = heights[biome_idx];

                let world_x = world_offset.x + x as f32 * cell_size;
                let world_z = world_offset.z + z as f32 * cell_size;

                let normal = normals[biome_idx];

                let (biome_weights_0, biome_weights_1) = biome_blends
                    .get(biome_idx)
                    .copied()
                    .map(Self::packed_biome_to_weight_sets)
                    .unwrap_or(([1.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0]));
                let (material_ids, material_weights) = splat_map
                    .get(biome_idx)
                    .copied()
                    .map(Self::splat_weights_to_material_slots)
                    .unwrap_or(([0.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]));

                vertices.push(TerrainVertex::new(
                    [world_x, height, world_z],
                    [normal.x, normal.y, normal.z],
                    [x as f32 / resolution as f32, z as f32 / resolution as f32],
                    biome_weights_0,
                    biome_weights_1,
                    material_ids,
                    material_weights,
                ));
            }
        }

        for z in 0..(resolution - 1) {
            for x in 0..(resolution - 1) {
                let top_left = (z * resolution + x) as u32;
                let top_right = top_left + 1;
                let bottom_left = ((z + 1) * resolution + x) as u32;
                let bottom_right = bottom_left + 1;

                indices.push(top_left);
                indices.push(bottom_left);
                indices.push(top_right);

                indices.push(top_right);
                indices.push(bottom_left);
                indices.push(bottom_right);
            }
        }

        (vertices, indices)
    }

    fn calculate_normal(heightmap: &Heightmap, x: usize, z: usize, cell_size: f32) -> Vec3 {
        let resolution = heightmap.resolution() as usize;

        let h_center = heightmap.get_height(x as u32, z as u32);
        let h_left = if x > 0 {
            heightmap.get_height((x - 1) as u32, z as u32)
        } else {
            h_center
        };
        let h_right = if x < resolution - 1 {
            heightmap.get_height((x + 1) as u32, z as u32)
        } else {
            h_center
        };
        let h_up = if z > 0 {
            heightmap.get_height(x as u32, (z - 1) as u32)
        } else {
            h_center
        };
        let h_down = if z < resolution - 1 {
            heightmap.get_height(x as u32, (z + 1) as u32)
        } else {
            h_center
        };

        let dx = (h_right - h_left) / (2.0 * cell_size);
        let dz = (h_down - h_up) / (2.0 * cell_size);

        Vec3::new(-dx, 1.0, -dz).normalize()
    }

    fn biome_to_id(biome: BiomeType) -> u32 {
        match biome {
            BiomeType::Grassland => 0,
            BiomeType::Desert => 1,
            BiomeType::Forest => 2,
            BiomeType::Mountain => 3,
            BiomeType::Tundra => 4,
            BiomeType::Swamp => 5,
            BiomeType::Beach => 6,
            BiomeType::River => 7,
            _ => 0, // Fallback for future biome types
        }
    }

    fn packed_biome_to_weight_sets(blend: PackedBiomeBlend) -> ([f32; 4], [f32; 4]) {
        let mut weights_0 = [0.0; 4];
        let mut weights_1 = [0.0; 4];

        for index in 0..4 {
            let biome_id = blend.biome_ids[index] as usize;
            let weight = blend.weights[index];
            if biome_id < 4 {
                weights_0[biome_id] += weight;
            } else if biome_id < 8 {
                weights_1[biome_id - 4] += weight;
            }
        }

        let total: f32 =
            weights_0.iter().copied().sum::<f32>() + weights_1.iter().copied().sum::<f32>();
        if total > 0.0001 {
            for weight in &mut weights_0 {
                *weight /= total;
            }
            for weight in &mut weights_1 {
                *weight /= total;
            }
        } else {
            weights_0[0] = 1.0;
        }

        (weights_0, weights_1)
    }

    /// Convert 8-channel SplatWeights into top-4 material slots (ids + weights).
    /// Channel indices map 1:1 to texture layer indices for channels 0-7.
    fn splat_weights_to_material_slots(weights: SplatWeights) -> ([f32; 4], [f32; 4]) {
        // Collect all non-zero (channel_as_layer_id, weight) pairs
        let mut entries: [(f32, f32); 8] = [
            (0.0, weights.weights_0.x),
            (1.0, weights.weights_0.y),
            (2.0, weights.weights_0.z),
            (3.0, weights.weights_0.w),
            (4.0, weights.weights_1.x),
            (5.0, weights.weights_1.y),
            (6.0, weights.weights_1.z),
            (7.0, weights.weights_1.w),
        ];

        // Sort by weight descending to find top 4
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top 4
        let mut top4: [(f32, f32); 4] = [entries[0], entries[1], entries[2], entries[3]];

        // Sort top 4 by material_id ascending for consistent slot assignment
        // across adjacent vertices (prevents interpolation artifacts)
        top4.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut ids = [0.0f32; 4];
        let mut ws = [0.0f32; 4];
        let mut total = 0.0f32;
        for i in 0..4 {
            ids[i] = top4[i].0;
            ws[i] = top4[i].1;
            total += ws[i];
        }
        if total > 0.0001 {
            for w in &mut ws {
                *w /= total;
            }
        } else {
            ids[0] = 0.0;
            ws[0] = 1.0; // fallback to grass (layer 0)
        }

        (ids, ws)
    }

    fn create_local_splat_generator(seed: u64, primary_biome: BiomeType) -> SplatMapGenerator {
        let mut generator = SplatMapGenerator::new(SplatConfig::default(), seed);
        match primary_biome {
            BiomeType::Grassland => {
                generator.add_rule(SplatRule::grass());
                generator.add_rule(SplatRule::rock());
                generator.add_rule(SplatRule {
                    material_id: 2,
                    min_height: 0.0,
                    max_height: 92.0,
                    min_slope: 3.0,
                    max_slope: 34.0,
                    priority: 14,
                    weight: 0.95,
                    height_falloff: 0.025,
                    slope_falloff: 0.05,
                });
                generator.add_rule(SplatRule {
                    material_id: 1,
                    min_height: -4.0,
                    max_height: 10.0,
                    min_slope: 0.0,
                    max_slope: 16.0,
                    priority: 18,
                    weight: 0.75,
                    height_falloff: 0.10,
                    slope_falloff: 0.07,
                });
            }
            BiomeType::Desert => {
                // Sand covers ALL heights on flat terrain. The built-in
                // SplatRule::sand() only spans -5..8 (beach-level), which
                // leaves heights >11 with zero weight → fallback to grass.
                generator.add_rule(SplatRule {
                    material_id: 1, // sand
                    min_height: -10.0,
                    max_height: 200.0,
                    min_slope: 0.0,
                    max_slope: 30.0,
                    priority: 15,
                    weight: 2.0,
                    height_falloff: 0.005,
                    slope_falloff: 0.05,
                });
                generator.add_rule(SplatRule::rock());
                generator.add_rule(SplatRule {
                    material_id: 3, // mountain rock on moderate slopes
                    min_height: -2.0,
                    max_height: 120.0,
                    min_slope: 8.0,
                    max_slope: 35.0,
                    priority: 13,
                    weight: 0.55,
                    height_falloff: 0.02,
                    slope_falloff: 0.05,
                });
                generator.add_rule(SplatRule {
                    material_id: 5, // mud — exposed hardpan in depressions (was 9/dirt, but MAX_SPLAT_LAYERS=8 drops ids≥8)
                    min_height: -8.0,
                    max_height: 15.0,
                    min_slope: 0.0,
                    max_slope: 12.0,
                    priority: 8,
                    weight: 0.30,
                    height_falloff: 0.06,
                    slope_falloff: 0.08,
                });
                generator.add_rule(SplatRule {
                    material_id: 7, // stone — mid-elevation breakup (was 13/gravel, but MAX_SPLAT_LAYERS=8 drops ids≥8)
                    min_height: 10.0,
                    max_height: 80.0,
                    min_slope: 5.0,
                    max_slope: 25.0,
                    priority: 10,
                    weight: 0.25,
                    height_falloff: 0.03,
                    slope_falloff: 0.06,
                });
            }
            BiomeType::Beach => {
                // Beach uses the built-in low-level sand rule
                generator.add_rule(SplatRule::sand());
                generator.add_rule(SplatRule::rock());
                generator.add_rule(SplatRule {
                    material_id: 3, // rock on slopes
                    min_height: -2.0,
                    max_height: 55.0,
                    min_slope: 8.0,
                    max_slope: 35.0,
                    priority: 13,
                    weight: 0.55,
                    height_falloff: 0.05,
                    slope_falloff: 0.05,
                });
                generator.add_rule(SplatRule {
                    material_id: 5, // mud (was 9/dirt, but MAX_SPLAT_LAYERS=8 drops ids≥8)
                    min_height: -4.0,
                    max_height: 6.0,
                    min_slope: 0.0,
                    max_slope: 10.0,
                    priority: 8,
                    weight: 0.15,
                    height_falloff: 0.10,
                    slope_falloff: 0.08,
                });
            }
            BiomeType::Forest => {
                generator.add_rule(SplatRule::grass());
                generator.add_rule(SplatRule {
                    material_id: 2,
                    min_height: 1.0,
                    max_height: 110.0,
                    min_slope: 0.0,
                    max_slope: 42.0,
                    priority: 16,
                    weight: 1.10,
                    height_falloff: 0.025,
                    slope_falloff: 0.04,
                });
                generator.add_rule(SplatRule::rock());
                generator.add_rule(SplatRule {
                    material_id: 5,
                    min_height: -8.0,
                    max_height: 12.0,
                    min_slope: 0.0,
                    max_slope: 18.0,
                    priority: 17,
                    weight: 0.70,
                    height_falloff: 0.10,
                    slope_falloff: 0.06,
                });
            }
            BiomeType::Mountain => {
                generator.add_rule(SplatRule::rock());
                generator.add_rule(SplatRule::snow());
                // Stone at mid-to-high altitudes on moderate slopes
                generator.add_rule(SplatRule {
                    material_id: 7,
                    min_height: 30.0,
                    max_height: 350.0,
                    min_slope: 0.0,
                    max_slope: 35.0,
                    priority: 14,
                    weight: 0.75,
                    height_falloff: 0.02,
                    slope_falloff: 0.05,
                });
                // Forest-floor at lower mountain altitudes
                generator.add_rule(SplatRule {
                    material_id: 2,
                    min_height: 0.0,
                    max_height: 80.0,
                    min_slope: 0.0,
                    max_slope: 26.0,
                    priority: 12,
                    weight: 0.45,
                    height_falloff: 0.03,
                    slope_falloff: 0.07,
                });
                // Grass at base of mountains
                generator.add_rule(SplatRule {
                    material_id: 0,
                    min_height: -4.0,
                    max_height: 40.0,
                    min_slope: 0.0,
                    max_slope: 18.0,
                    priority: 11,
                    weight: 0.35,
                    height_falloff: 0.06,
                    slope_falloff: 0.10,
                });
            }
            BiomeType::Tundra => {
                generator.add_rule(SplatRule::snow());
                generator.add_rule(SplatRule::rock());
                generator.add_rule(SplatRule {
                    material_id: 4,
                    min_height: -4.0,
                    max_height: 58.0,
                    min_slope: 0.0,
                    max_slope: 24.0,
                    priority: 16,
                    weight: 0.60,
                    height_falloff: 0.05,
                    slope_falloff: 0.06,
                });
                generator.add_rule(SplatRule {
                    material_id: 5,
                    min_height: -12.0,
                    max_height: 4.0,
                    min_slope: 0.0,
                    max_slope: 14.0,
                    priority: 12,
                    weight: 0.35,
                    height_falloff: 0.12,
                    slope_falloff: 0.08,
                });
            }
            BiomeType::Swamp | BiomeType::River => {
                generator.add_rule(SplatRule {
                    material_id: 5,
                    min_height: -14.0,
                    max_height: 16.0,
                    min_slope: 0.0,
                    max_slope: 20.0,
                    priority: 22,
                    weight: 1.25,
                    height_falloff: 0.10,
                    slope_falloff: 0.06,
                });
                generator.add_rule(SplatRule::sand());
                generator.add_rule(SplatRule {
                    material_id: 2,
                    min_height: -2.0,
                    max_height: 44.0,
                    min_slope: 0.0,
                    max_slope: 26.0,
                    priority: 14,
                    weight: 0.65,
                    height_falloff: 0.05,
                    slope_falloff: 0.05,
                });
                generator.add_rule(SplatRule::rock());
            }
            _ => {
                generator.add_rule(SplatRule::grass());
                generator.add_rule(SplatRule::rock());
                generator.add_rule(SplatRule::sand());
                generator.add_rule(SplatRule::snow());
            }
        }
        generator
    }

    fn primary_biome_type(&self) -> BiomeType {
        self.config
            .biomes
            .first()
            .map(|b| b.biome_type)
            .unwrap_or(BiomeType::Grassland)
    }

    fn id_to_biome(id: u32) -> BiomeType {
        match id {
            0 => BiomeType::Grassland,
            1 => BiomeType::Desert,
            2 => BiomeType::Forest,
            3 => BiomeType::Mountain,
            4 => BiomeType::Tundra,
            5 => BiomeType::Swamp,
            6 => BiomeType::Beach,
            7 => BiomeType::River,
            _ => BiomeType::Grassland,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.terrain_dirty
    }

    pub fn chunk_count(&self) -> usize {
        self.generated_chunks.len()
    }

    pub fn get_all_vertices(&self) -> Vec<TerrainVertex> {
        let mut all_vertices = Vec::new();
        for gen_chunk in self.generated_chunks.values() {
            all_vertices.extend_from_slice(&gen_chunk.vertices);
        }
        all_vertices
    }

    pub fn get_all_indices(&self, vertex_offset: u32) -> Vec<u32> {
        let mut all_indices = Vec::new();
        let mut current_offset = vertex_offset;

        for gen_chunk in self.generated_chunks.values() {
            for &idx in &gen_chunk.indices {
                all_indices.push(idx + current_offset);
            }
            current_offset += gen_chunk.vertices.len() as u32;
        }
        all_indices
    }

    pub fn get_height_at(&self, world_x: f32, world_z: f32) -> Option<f32> {
        let chunk_size = self.config.chunk_size;
        let chunk_x = (world_x / chunk_size).floor() as i32;
        let chunk_z = (world_z / chunk_size).floor() as i32;
        let chunk_id = ChunkId {
            x: chunk_x,
            z: chunk_z,
        };

        if let Some(gen_chunk) = self.generated_chunks.get(&chunk_id) {
            let world_pos = Vec3::new(world_x, 0.0, world_z);
            gen_chunk
                .chunk
                .get_height_at_world_pos(world_pos, chunk_size)
        } else {
            None
        }
    }

    pub fn seed(&self) -> u64 {
        self.config.seed
    }

    pub fn primary_biome(&self) -> &str {
        if let Some(first) = self.config.biomes.first() {
            first.biome_type.as_str()
        } else {
            "grassland"
        }
    }

    pub fn chunks(&self) -> impl Iterator<Item = (&ChunkId, &GeneratedChunk)> {
        self.generated_chunks.iter()
    }

    /// Regenerate splatmap material weights for all chunks using the given parameters.
    /// Updates vertex `material_ids` and `material_weights` in-place and marks all
    /// chunks as dirty for GPU re-upload.
    pub fn regenerate_splatmaps(
        &mut self,
        rock_slope_threshold: f32,
        snow_height_threshold: f32,
        sand_height_max: f32,
    ) {
        let seed = self.config.seed;
        let primary_biome = self.primary_biome_type();
        let chunk_size = self.config.chunk_size;

        self.dirty_chunk_indices.clear();
        for (order_idx, chunk_id) in self.chunk_order.iter().enumerate() {
            let Some(gen_chunk) = self.generated_chunks.get_mut(chunk_id) else {
                continue;
            };
            let hm = gen_chunk.chunk.heightmap();
            let res = hm.resolution() as usize;
            let cell_size = chunk_size / (res - 1) as f32;

            let mut heights = Vec::with_capacity(res * res);
            let mut normals = Vec::with_capacity(res * res);
            for z in 0..res {
                for x in 0..res {
                    heights.push(hm.get_height(x as u32, z as u32));
                    normals.push(Self::calculate_normal(hm, x, z, cell_size));
                }
            }

            // Create splat generator with modified thresholds
            let mut generator = Self::create_local_splat_generator(seed, primary_biome);
            // Override rules based on UI parameters: adjust rock slope and snow height
            for rule in generator.rules_mut() {
                // Rock: adjust slope threshold
                if rule.material_id == 3 || rule.material_id == 7 || rule.material_id == 8 {
                    rule.min_slope = rock_slope_threshold * 0.5;
                    rule.max_slope = rock_slope_threshold + 20.0;
                }
                // Snow: adjust height threshold
                if rule.material_id == 4 {
                    let scaled = snow_height_threshold * 200.0;
                    rule.min_height = scaled - 20.0;
                }
                // Sand: adjust max height
                if rule.material_id == 1 {
                    rule.max_height = sand_height_max * 200.0;
                }
            }

            let splat_map = generator.generate_splat_map(&heights, &normals, res as u32);

            for (i, splat) in splat_map.iter().enumerate() {
                if i >= gen_chunk.vertices.len() {
                    break;
                }
                let (ids, weights) = Self::splat_weights_to_material_slots(*splat);
                gen_chunk.vertices[i].material_ids = ids;
                gen_chunk.vertices[i].material_weights = weights;
            }

            self.dirty_chunk_indices.push(order_idx);
        }

        tracing::info!(
            "Splatmaps regenerated for {} chunks (rock_slope={:.1}, snow_h={:.2}, sand_max={:.2})",
            self.dirty_chunk_indices.len(),
            rock_slope_threshold,
            snow_height_threshold,
            sand_height_max,
        );
    }

    pub fn get_gpu_chunks(&self) -> Vec<(Vec<TerrainVertex>, Vec<u32>)> {
        // Use stable ordering so GPU chunk indices stay consistent
        self.chunk_order
            .iter()
            .filter_map(|id| self.generated_chunks.get(id))
            .map(|chunk| (chunk.vertices.clone(), chunk.indices.clone()))
            .collect()
    }

    /// Take dirty chunk data after a brush stroke.
    /// Returns (gpu_index, vertices, indices) for each modified chunk.
    /// Clears the dirty list.
    pub fn take_dirty_chunks(&mut self) -> Vec<(usize, Vec<TerrainVertex>)> {
        let dirty: Vec<(usize, Vec<TerrainVertex>)> = self
            .dirty_chunk_indices
            .iter()
            .filter_map(|&idx| {
                let chunk_id = self.chunk_order.get(idx)?;
                let gen_chunk = self.generated_chunks.get(chunk_id)?;
                Some((idx, gen_chunk.vertices.clone()))
            })
            .collect();
        self.dirty_chunk_indices.clear();
        dirty
    }

    /// Apply heightmap patches from zone scatter generation.
    ///
    /// Mutates the heightmap in each affected chunk, regenerates vertex positions
    /// and normals, and marks the chunks dirty for GPU re-upload.
    pub fn apply_zone_heightmap_patches(&mut self, patches: &[HeightmapPatch]) {
        let chunk_size = self.config.chunk_size;

        for patch in patches {
            if patch.heights.is_empty() {
                continue;
            }
            let gen_chunk = match self.generated_chunks.get_mut(&patch.chunk_id) {
                Some(c) => c,
                None => continue,
            };

            let res = gen_chunk.chunk.heightmap().resolution();
            let cell_size = chunk_size / (res - 1) as f32;

            // Write patched heights into the chunk heightmap
            for (&(gx, gz), &height) in &patch.heights {
                gen_chunk.chunk.heightmap_mut().set_height(gx, gz, height);
            }

            // Regenerate vertex positions and normals for the entire chunk
            // (patches may affect normals of neighboring vertices)
            for gz in 0..res as usize {
                for gx in 0..res as usize {
                    let idx = gz * res as usize + gx;
                    if idx < gen_chunk.vertices.len() {
                        let new_h = gen_chunk.chunk.heightmap().get_height(gx as u32, gz as u32);
                        gen_chunk.vertices[idx].position[1] = new_h;
                        let normal =
                            Self::calculate_normal(gen_chunk.chunk.heightmap(), gx, gz, cell_size);
                        gen_chunk.vertices[idx].normal = [normal.x, normal.y, normal.z];
                    }
                }
            }

            // Mark dirty for GPU re-upload
            if let Some(gpu_idx) = self.chunk_order.iter().position(|id| *id == patch.chunk_id) {
                if !self.dirty_chunk_indices.contains(&gpu_idx) {
                    self.dirty_chunk_indices.push(gpu_idx);
                }
            }
        }
    }

    /// Begin a new brush stroke — enables heightmap snapshotting for undo.
    pub fn begin_stroke(&mut self) {
        self.is_stroking = true;
        self.stroke_pre_snapshots.clear();
    }

    /// Returns true if currently in a brush stroke.
    pub fn is_stroking(&self) -> bool {
        self.is_stroking
    }

    /// End a brush stroke — returns the undo data (ChunkId → (pre, post) heightmaps).
    /// Returns None if no chunks were modified.
    pub fn end_stroke(&mut self) -> Option<Vec<(ChunkId, Vec<f32>, Vec<f32>)>> {
        self.is_stroking = false;
        if self.stroke_pre_snapshots.is_empty() {
            return None;
        }

        let mut deltas = Vec::new();
        for (chunk_id, pre_heights) in self.stroke_pre_snapshots.drain() {
            if let Some(gen_chunk) = self.generated_chunks.get(&chunk_id) {
                let hm = gen_chunk.chunk.heightmap();
                let res = hm.resolution();
                let mut post_heights = Vec::with_capacity((res * res) as usize);
                for gz in 0..res {
                    for gx in 0..res {
                        post_heights.push(hm.get_height(gx, gz));
                    }
                }
                deltas.push((chunk_id, pre_heights, post_heights));
            }
        }

        if deltas.is_empty() {
            None
        } else {
            Some(deltas)
        }
    }

    /// Apply a heightmap snapshot (used for undo/redo).
    /// Updates heightmap, patches vertices, and marks chunks dirty.
    pub fn apply_height_snapshot(&mut self, snapshot: &[(ChunkId, Vec<f32>)]) {
        let chunk_size = self.config.chunk_size;
        for (chunk_id, heights) in snapshot {
            if let Some(gen_chunk) = self.generated_chunks.get_mut(chunk_id) {
                let res = gen_chunk.chunk.heightmap().resolution();
                let cell_size = chunk_size / (res - 1) as f32;
                // Write heights
                for (i, &h) in heights.iter().enumerate() {
                    let gx = (i as u32) % res;
                    let gz = (i as u32) / res;
                    gen_chunk.chunk.heightmap_mut().set_height(gx, gz, h);
                }
                // Patch vertices
                for gz in 0..res as usize {
                    for gx in 0..res as usize {
                        let idx = gz * res as usize + gx;
                        if idx < gen_chunk.vertices.len() {
                            let new_h =
                                gen_chunk.chunk.heightmap().get_height(gx as u32, gz as u32);
                            gen_chunk.vertices[idx].position[1] = new_h;
                            let normal = Self::calculate_normal(
                                gen_chunk.chunk.heightmap(),
                                gx,
                                gz,
                                cell_size,
                            );
                            gen_chunk.vertices[idx].normal = [normal.x, normal.y, normal.z];
                        }
                    }
                }
                // Mark dirty
                if let Some(gpu_idx) = self.chunk_order.iter().position(|id| id == chunk_id) {
                    if !self.dirty_chunk_indices.contains(&gpu_idx) {
                        self.dirty_chunk_indices.push(gpu_idx);
                    }
                }
            }
        }
    }

    /// Get total vertex count across all chunks
    pub fn total_vertex_count(&self) -> usize {
        self.generated_chunks
            .values()
            .map(|c| c.vertices.len())
            .sum()
    }

    /// Get total index/triangle count across all chunks
    pub fn total_index_count(&self) -> usize {
        self.generated_chunks
            .values()
            .map(|c| c.indices.len())
            .sum()
    }

    /// Get total triangle count
    pub fn total_triangle_count(&self) -> usize {
        self.total_index_count() / 3
    }

    /// Compute min, max, and average terrain height across all generated chunks.
    pub fn height_stats(&self) -> (f32, f32, f32) {
        let mut min_h = f32::MAX;
        let mut max_h = f32::MIN;
        let mut sum = 0.0f64;
        let mut count = 0u64;
        for gen_chunk in self.generated_chunks.values() {
            for v in &gen_chunk.vertices {
                let h = v.position[1];
                if h < min_h {
                    min_h = h;
                }
                if h > max_h {
                    max_h = h;
                }
                sum += h as f64;
                count += 1;
            }
        }
        if count == 0 {
            return (0.0, 0.0, 0.0);
        }
        (min_h, max_h, (sum / count as f64) as f32)
    }

    /// Check if terrain has been generated
    pub fn has_terrain(&self) -> bool {
        !self.generated_chunks.is_empty()
    }

    /// Get chunk IDs as a list
    pub fn chunk_ids(&self) -> Vec<ChunkId> {
        self.generated_chunks.keys().cloned().collect()
    }

    /// Apply a sculpting brush at the given world-space position.
    ///
    /// `brush_mode`: 0=Raise, 1=Smooth, 2=Flatten, 3=Lower, 4=Erode
    /// Returns true if any terrain was modified.
    /// Sample the heightmap height at a world position (bilinear between nearest vertices).
    /// Returns None if no chunk contains the position.
    pub fn sample_height_at(&self, world_x: f32, world_z: f32) -> Option<f32> {
        let chunk_size = self.config.chunk_size;
        for (chunk_id, gen_chunk) in &self.generated_chunks {
            let ox = chunk_id.x as f32 * chunk_size;
            let oz = chunk_id.z as f32 * chunk_size;
            if world_x >= ox
                && world_x <= ox + chunk_size
                && world_z >= oz
                && world_z <= oz + chunk_size
            {
                let res = gen_chunk.chunk.heightmap().resolution();
                let cell = chunk_size / (res - 1) as f32;
                let gx = ((world_x - ox) / cell).round() as u32;
                let gz = ((world_z - oz) / cell).round() as u32;
                let gx = gx.min(res - 1);
                let gz = gz.min(res - 1);
                return Some(gen_chunk.chunk.heightmap().get_height(gx, gz));
            }
        }
        None
    }

    pub fn apply_brush(
        &mut self,
        world_x: f32,
        world_z: f32,
        radius: f32,
        strength: f32,
        brush_mode: crate::panels::terrain_panel::BrushMode,
        falloff_curve: crate::panels::terrain_panel::FalloffCurve,
        flatten_target: Option<f32>,
        noise_scale: f32,
    ) -> bool {
        use crate::panels::terrain_panel::BrushMode;

        let chunk_size = self.config.chunk_size;
        let mut modified = false;

        // Collect chunk IDs that might be affected
        let chunk_ids: Vec<ChunkId> = self.generated_chunks.keys().cloned().collect();

        for chunk_id in chunk_ids {
            let chunk_origin_x = chunk_id.x as f32 * chunk_size;
            let chunk_origin_z = chunk_id.z as f32 * chunk_size;

            // Quick AABB check: does the brush circle overlap this chunk?
            let closest_x = world_x.clamp(chunk_origin_x, chunk_origin_x + chunk_size);
            let closest_z = world_z.clamp(chunk_origin_z, chunk_origin_z + chunk_size);
            let dx = world_x - closest_x;
            let dz = world_z - closest_z;
            if dx * dx + dz * dz > radius * radius {
                continue;
            }

            if let Some(gen_chunk) = self.generated_chunks.get_mut(&chunk_id) {
                // Snapshot heightmap before first modification in this stroke (for undo)
                if self.is_stroking && !self.stroke_pre_snapshots.contains_key(&chunk_id) {
                    let hm = gen_chunk.chunk.heightmap();
                    let res = hm.resolution();
                    let mut snapshot = Vec::with_capacity((res * res) as usize);
                    for gz in 0..res {
                        for gx in 0..res {
                            snapshot.push(hm.get_height(gx, gz));
                        }
                    }
                    self.stroke_pre_snapshots.insert(chunk_id, snapshot);
                }

                let resolution = gen_chunk.chunk.heightmap().resolution();
                let cell_size = chunk_size / (resolution - 1) as f32;
                let mut chunk_modified = false;

                // Pre-compute average height for Smooth/Erode modes
                let avg_height = if matches!(brush_mode, BrushMode::Smooth | BrushMode::Erode) {
                    let mut sum = 0.0f32;
                    let mut count = 0u32;
                    for gz in 0..resolution {
                        for gx in 0..resolution {
                            let px = chunk_origin_x + gx as f32 * cell_size;
                            let pz = chunk_origin_z + gz as f32 * cell_size;
                            let d = ((px - world_x).powi(2) + (pz - world_z).powi(2)).sqrt();
                            if d <= radius {
                                sum += gen_chunk.chunk.heightmap().get_height(gx, gz);
                                count += 1;
                            }
                        }
                    }
                    if count > 0 {
                        sum / count as f32
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                for gz in 0..resolution {
                    for gx in 0..resolution {
                        let px = chunk_origin_x + gx as f32 * cell_size;
                        let pz = chunk_origin_z + gz as f32 * cell_size;
                        let dist = ((px - world_x).powi(2) + (pz - world_z).powi(2)).sqrt();
                        if dist > radius {
                            continue;
                        }

                        let t = dist / radius; // 0 at center, 1 at edge
                        let falloff = falloff_curve.eval(t);
                        let current_h = gen_chunk.chunk.heightmap().get_height(gx, gz);

                        let new_h = match brush_mode {
                            BrushMode::Sculpt => current_h + strength * falloff * 5.0,
                            BrushMode::Lower => current_h - strength * falloff * 5.0,
                            BrushMode::Smooth => {
                                let blend = strength * falloff * 0.3;
                                current_h * (1.0 - blend) + avg_height * blend
                            }
                            BrushMode::Flatten => {
                                let target = flatten_target.unwrap_or(current_h);
                                let blend = strength * falloff;
                                current_h * (1.0 - blend) + target * blend
                            }
                            BrushMode::Erode => {
                                let erode = current_h - strength * falloff * 3.0;
                                erode * (1.0 - falloff * 0.1) + avg_height * falloff * 0.1
                            }
                            BrushMode::Noise => {
                                // Simple deterministic noise displacement based on position
                                let nx = px * noise_scale;
                                let nz = pz * noise_scale;
                                let noise_val = (nx.sin() * 2.0
                                    + nz.cos() * 3.0
                                    + (nx * 2.7 + nz * 1.3).sin() * 1.5)
                                    / 6.5; // normalized roughly -1..1
                                current_h + strength * falloff * noise_val * 5.0
                            }
                            BrushMode::Paint => current_h, // Handled separately
                            BrushMode::ZoneBlend => current_h, // Zone blend is handled at a higher level via blend masks
                        };

                        gen_chunk.chunk.heightmap_mut().set_height(gx, gz, new_h);
                        chunk_modified = true;
                    }
                }

                if chunk_modified {
                    // Fast path: patch vertex heights and normals in-place
                    let cell_size_patch = chunk_size / (resolution - 1) as f32;
                    for gz in 0..resolution as usize {
                        for gx in 0..resolution as usize {
                            let idx = gz * resolution as usize + gx;
                            if idx < gen_chunk.vertices.len() {
                                let new_h =
                                    gen_chunk.chunk.heightmap().get_height(gx as u32, gz as u32);
                                gen_chunk.vertices[idx].position[1] = new_h;
                                let normal = Self::calculate_normal(
                                    gen_chunk.chunk.heightmap(),
                                    gx,
                                    gz,
                                    cell_size_patch,
                                );
                                gen_chunk.vertices[idx].normal = [normal.x, normal.y, normal.z];
                            }
                        }
                    }
                    // Mark this chunk dirty for incremental GPU upload
                    if let Some(gpu_idx) = self.chunk_order.iter().position(|id| *id == chunk_id) {
                        if !self.dirty_chunk_indices.contains(&gpu_idx) {
                            self.dirty_chunk_indices.push(gpu_idx);
                        }
                    }
                    modified = true;
                }
            }
        }

        modified
    }

    /// Paint a biome material at the given world-space position.
    ///
    /// `biome_id`: 0-7 corresponding to the shader biome IDs.
    /// Returns true if any terrain was modified.
    pub fn apply_brush_paint(
        &mut self,
        world_x: f32,
        world_z: f32,
        radius: f32,
        biome_id: u32,
    ) -> bool {
        let chunk_size = self.config.chunk_size;
        let primary_biome = self.primary_biome_type();
        let target_biome = Self::id_to_biome(biome_id);
        let mut modified = false;

        let chunk_ids: Vec<ChunkId> = self.generated_chunks.keys().cloned().collect();

        for chunk_id in chunk_ids {
            let chunk_origin_x = chunk_id.x as f32 * chunk_size;
            let chunk_origin_z = chunk_id.z as f32 * chunk_size;

            // Quick AABB check
            let closest_x = world_x.clamp(chunk_origin_x, chunk_origin_x + chunk_size);
            let closest_z = world_z.clamp(chunk_origin_z, chunk_origin_z + chunk_size);
            let dx = world_x - closest_x;
            let dz = world_z - closest_z;
            if dx * dx + dz * dz > radius * radius {
                continue;
            }

            if let Some(gen_chunk) = self.generated_chunks.get_mut(&chunk_id) {
                let resolution = gen_chunk.chunk.heightmap().resolution() as usize;
                let cell_size = chunk_size / (resolution - 1) as f32;
                let mut chunk_modified = false;

                let biome_map = gen_chunk.chunk.biome_map_mut();
                for gz in 0..resolution {
                    for gx in 0..resolution {
                        let px = chunk_origin_x + gx as f32 * cell_size;
                        let pz = chunk_origin_z + gz as f32 * cell_size;
                        let dist = ((px - world_x).powi(2) + (pz - world_z).powi(2)).sqrt();
                        if dist > radius {
                            continue;
                        }
                        let idx = gz * resolution + gx;
                        if idx < biome_map.len() {
                            biome_map[idx] = target_biome;
                            chunk_modified = true;
                        }
                    }
                }

                if chunk_modified {
                    let world_offset = Vec3::new(chunk_origin_x, 0.0, chunk_origin_z);
                    let (vertices, indices) = Self::generate_heightmap_mesh(
                        gen_chunk.chunk.heightmap(),
                        gen_chunk.chunk.biome_map(),
                        chunk_size,
                        world_offset,
                        self.config.seed,
                        primary_biome,
                    );
                    gen_chunk.vertices = vertices;
                    gen_chunk.indices = indices;
                    // Mark this chunk dirty for incremental GPU upload
                    if let Some(gpu_idx) = self.chunk_order.iter().position(|id| *id == chunk_id) {
                        if !self.dirty_chunk_indices.contains(&gpu_idx) {
                            self.dirty_chunk_indices.push(gpu_idx);
                        }
                    }
                    modified = true;
                }
            }
        }

        modified
    }

    /// Paint a material directly onto vertex material_ids/material_weights slots.
    ///
    /// Unlike `apply_brush_paint` (which modifies the biome map and regenerates
    /// the mesh), this method edits vertex data in-place — no mesh regeneration
    /// needed.  The falloff curve and strength control how aggressively the
    /// target material replaces existing materials at each vertex.
    pub fn apply_brush_paint_material(
        &mut self,
        world_x: f32,
        world_z: f32,
        radius: f32,
        strength: f32,
        material_id: u32,
        falloff_curve: crate::panels::terrain_panel::FalloffCurve,
    ) -> bool {
        let chunk_size = self.config.chunk_size;
        let mat_id_f32 = material_id as f32;
        let mut modified = false;

        let chunk_ids: Vec<ChunkId> = self.generated_chunks.keys().cloned().collect();

        for chunk_id in chunk_ids {
            let chunk_origin_x = chunk_id.x as f32 * chunk_size;
            let chunk_origin_z = chunk_id.z as f32 * chunk_size;

            // Quick AABB rejection
            let closest_x = world_x.clamp(chunk_origin_x, chunk_origin_x + chunk_size);
            let closest_z = world_z.clamp(chunk_origin_z, chunk_origin_z + chunk_size);
            let dx = world_x - closest_x;
            let dz = world_z - closest_z;
            if dx * dx + dz * dz > radius * radius {
                continue;
            }

            if let Some(gen_chunk) = self.generated_chunks.get_mut(&chunk_id) {
                let mut chunk_modified = false;

                for vertex in gen_chunk.vertices.iter_mut() {
                    let vx = vertex.position[0];
                    let vz = vertex.position[2];
                    let dist = ((vx - world_x).powi(2) + (vz - world_z).powi(2)).sqrt();
                    if dist > radius {
                        continue;
                    }

                    let t = dist / radius;
                    let influence = strength * falloff_curve.eval(t);

                    // Find if this material already occupies one of the 4 slots
                    let mut slot = None;
                    for i in 0..4 {
                        if (vertex.material_ids[i] - mat_id_f32).abs() < 0.5 {
                            slot = Some(i);
                            break;
                        }
                    }

                    // If not present, evict the slot with the lowest weight
                    let slot = slot.unwrap_or_else(|| {
                        let mut min_idx = 0;
                        let mut min_w = vertex.material_weights[0];
                        for i in 1..4 {
                            if vertex.material_weights[i] < min_w {
                                min_w = vertex.material_weights[i];
                                min_idx = i;
                            }
                        }
                        vertex.material_ids[min_idx] = mat_id_f32;
                        vertex.material_weights[min_idx] = 0.0;
                        min_idx
                    });

                    // Add influence to the target slot
                    vertex.material_weights[slot] += influence;

                    // Renormalize so weights sum to 1.0
                    let sum: f32 = vertex.material_weights.iter().sum();
                    if sum > 0.0 {
                        for w in vertex.material_weights.iter_mut() {
                            *w /= sum;
                        }
                    }

                    chunk_modified = true;
                }

                if chunk_modified {
                    if let Some(gpu_idx) = self.chunk_order.iter().position(|id| *id == chunk_id) {
                        if !self.dirty_chunk_indices.contains(&gpu_idx) {
                            self.dirty_chunk_indices.push(gpu_idx);
                        }
                    }
                    modified = true;
                }
            }
        }

        modified
    }

    /// Get terrain statistics
    pub fn stats(&self) -> TerrainStats {
        TerrainStats {
            chunk_count: self.generated_chunks.len(),
            total_vertices: self.total_vertex_count(),
            total_indices: self.total_index_count(),
            total_triangles: self.total_triangle_count(),
            seed: self.last_seed,
            is_dirty: self.terrain_dirty,
        }
    }

    /// Generate scatter placements for all generated terrain chunks.
    ///
    /// Uses the existing VegetationScatter system from astraweave-terrain
    /// to place vegetation/rocks with Poisson disk sampling and biome-aware
    /// density rules. Returns a flat list of placements ready for the
    /// scatter renderer's GPU instancing pipeline.
    pub fn generate_scatter_placements(&self) -> Vec<ScatterPlacement> {
        let chunk_size = self.config.chunk_size;

        let mut placements = Vec::new();

        for (chunk_id, gen_chunk) in &self.generated_chunks {
            let chunk = &gen_chunk.chunk;

            // Sample biome at chunk center
            let chunk_center = chunk_id.to_center_pos(chunk_size);
            let center_biome = chunk
                .get_biome_at_world_pos(chunk_center, chunk_size)
                .unwrap_or(BiomeType::Grassland);

            // Biome-dependent minimum distance: open biomes get wider spacing,
            // dense biomes (forest, swamp) get tighter packing.
            let min_dist = match center_biome {
                BiomeType::Forest => 18.0,
                BiomeType::Swamp => 14.0,
                BiomeType::River => 14.0,
                BiomeType::Grassland => 18.0,
                BiomeType::Mountain => 22.0,
                BiomeType::Desert => 24.0,
                BiomeType::Tundra => 22.0,
                BiomeType::Beach => 20.0,
                _ => 16.0,
            };
            let scatter = VegetationScatter::new(ScatterConfig {
                min_distance: min_dist,
                ..ScatterConfig::default()
            });

            let primary_biome_config = self
                .config
                .biomes
                .first()
                .cloned()
                .unwrap_or_else(BiomeConfig::grassland);
            let biome_config = self
                .config
                .biomes
                .iter()
                .find(|b| b.biome_type == center_biome)
                .cloned()
                .unwrap_or(primary_biome_config);

            let seed = self
                .config
                .seed
                .wrapping_add((chunk_id.x as u64).wrapping_mul(1000))
                .wrapping_add(chunk_id.z as u64);

            // Generate vegetation instances via the terrain scatter system
            let vegetation =
                match scatter.scatter_vegetation(chunk, chunk_size, &biome_config, seed) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("Scatter failed for chunk {:?}: {e}", chunk_id);
                        continue;
                    }
                };

            for vi in vegetation {
                placements.push(ScatterPlacement::from_vegetation_instance(&vi));
            }
        }

        tracing::info!(
            "Generated {} scatter placements across {} chunks",
            placements.len(),
            self.generated_chunks.len()
        );

        // Log a sample placement to help debug mesh path resolution
        if let Some(sample) = placements.first() {
            tracing::info!(
                "Scatter sample: key='{}' path='{}' pos=({:.1},{:.1},{:.1}) scale={:.2}",
                sample.mesh_key,
                sample.mesh_path,
                sample.position.x,
                sample.position.y,
                sample.position.z,
                sample.scale,
            );
        }

        placements
    }
}

/// Statistics for terrain state
#[derive(Debug, Clone)]
pub struct TerrainStats {
    /// Number of chunks generated
    pub chunk_count: usize,
    /// Total vertex count
    pub total_vertices: usize,
    /// Total index count
    pub total_indices: usize,
    /// Total triangle count
    pub total_triangles: usize,
    /// Seed used for generation
    pub seed: u64,
    /// Whether terrain needs regeneration
    pub is_dirty: bool,
}

impl TerrainStats {
    /// Check if any terrain has been generated
    pub fn has_terrain(&self) -> bool {
        self.chunk_count > 0
    }

    /// Get average vertices per chunk
    pub fn avg_vertices_per_chunk(&self) -> f32 {
        if self.chunk_count == 0 {
            0.0
        } else {
            self.total_vertices as f32 / self.chunk_count as f32
        }
    }

    /// Get average triangles per chunk
    pub fn avg_triangles_per_chunk(&self) -> f32 {
        if self.chunk_count == 0 {
            0.0
        } else {
            self.total_triangles as f32 / self.chunk_count as f32
        }
    }
}

impl TerrainVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TerrainVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

/// CPU-side scatter placement (bridges terrain VegetationInstance to GPU renderer).
#[derive(Debug, Clone)]
pub struct ScatterPlacement {
    pub position: Vec3,
    pub rotation: f32,
    pub scale: f32,
    pub mesh_key: String,
    pub mesh_path: String,
    pub bounding_radius: f32,
    pub tint: [f32; 4],
    pub terrain_normal: Vec3,
}

impl ScatterPlacement {
    pub fn from_vegetation_instance(vi: &VegetationInstance) -> Self {
        // Per-type world-scale multiplier: Nature Kit models are tiny (~1 unit),
        // terrain spans hundreds of units. Trees need a much larger multiplier
        // than grass/flowers to look proportional.
        let (type_multiplier, sink_factor, tint) = match vi.vegetation_type.as_str() {
            // Trees — models are ~1.2–1.7 units tall, need to reach 15–30 units
            s if s.contains("tree") || s.contains("pine") => (14.0, 0.06, [0.35, 0.55, 0.25, 1.0]),
            // Cacti — models ~0.75 units, need to be ~5–8 units
            s if s.contains("cactus") => (8.0, 0.04, [0.45, 0.60, 0.30, 1.0]),
            // Bushes — models ~0.25 units, need to be ~2–3 units
            s if s.contains("bush") => (7.0, 0.04, [0.30, 0.58, 0.22, 1.0]),
            // Rocks/boulders/cliffs — models ~0.26 units, need to be ~2–4 units
            s if s.contains("rock")
                || s.contains("stone")
                || s.contains("boulder")
                || s.contains("cliff") =>
            {
                (8.0, 0.03, [0.60, 0.58, 0.55, 1.0])
            }
            // Mushrooms — small ground detail
            s if s.contains("mushroom") => (5.0, 0.01, [0.70, 0.55, 0.40, 1.0]),
            // Flowers — keep their original colors mostly
            s if s.contains("flower") => (3.5, 0.02, [0.90, 0.85, 0.80, 1.0]),
            // Grass, ground cover — models ~0.2 units, OK at ~1–2 units
            _ => (3.5, 0.02, [0.40, 0.68, 0.28, 1.0]),
        };
        let world_scale = vi.scale * type_multiplier;
        // Sink base slightly below terrain to counteract bilinear-vs-triangle
        // height mismatch that causes objects to float above the rendered surface.
        // The base AO darkening in the shader hides the minor terrain intersection.
        // Per-type sinking: small assets sink less, large assets sink more.
        let mut pos = vi.position;
        pos.y -= sink_factor * world_scale;
        Self {
            position: pos,
            rotation: vi.rotation,
            scale: world_scale,
            mesh_key: vi.vegetation_type.clone(),
            mesh_path: vi.model_path.clone(),
            bounding_radius: world_scale * 2.0,
            tint,
            terrain_normal: vi.terrain_normal,
        }
    }

    /// Create a ScatterPlacement from a zone-generated VegetationInstance with
    /// BiomePack context for physically-correct scaling.
    ///
    /// When a BiomePack is provided, asset dimensions from the pack drive the
    /// bounding radius and the heuristic type-multiplier is bypassed (scale 1:1).
    /// Mesh paths are resolved to absolute paths via `pack.root_dir`.
    pub fn from_zone_placement(vi: &VegetationInstance, pack: Option<&BiomePack>) -> Self {
        // Try to find matching asset in the pack for dimension-based scaling
        let pack_asset = pack.and_then(|p| {
            // Match by mesh_path (relative) — the zone scatter uses the same paths
            p.assets.iter().find(|a| {
                vi.model_path.ends_with(&a.mesh_path)
                    || a.mesh_path.ends_with(
                        std::path::Path::new(&vi.model_path)
                            .file_name()
                            .unwrap_or_default()
                            .to_str()
                            .unwrap_or_default(),
                    )
            })
        });

        // Resolve mesh path to absolute
        let resolved_path = match pack {
            Some(p) if !std::path::Path::new(&vi.model_path).is_absolute() => p
                .root_dir
                .join(&vi.model_path)
                .to_string_lossy()
                .to_string(),
            _ => vi.model_path.clone(),
        };

        // Use pack dimensions for correct scaling, fall back to heuristics
        let (world_scale, sink_factor, tint, bounding_radius) = if let Some(asset) = pack_asset {
            // Use actual Blender dimensions — no heuristic multiplier needed.
            // The zone scatter system already handles adaptive scaling via
            // AdaptiveScaleParams, so we use scale 1:1 here.
            let dims = asset.dimensions.unwrap_or([1.0, 1.0, 1.0]);
            let max_dim = dims[0].max(dims[1]).max(dims[2]) as f32;
            let radius = max_dim * vi.scale * 0.5;
            let sink = 0.02 * vi.scale;
            // Category-aware tint for visual variety
            let tint = match asset.category.as_str() {
                "vegetation" => [0.35, 0.55, 0.25, 1.0],
                "rock" => [0.60, 0.58, 0.55, 1.0],
                "structure" | "furniture" => [0.70, 0.65, 0.60, 1.0],
                _ => [0.50, 0.50, 0.50, 1.0],
            };
            (vi.scale, sink, tint, radius)
        } else {
            // Fall back to heuristic scaling for non-pack placements
            let (type_multiplier, sink, tint) = match vi.vegetation_type.as_str() {
                s if s.contains("tree") || s.contains("pine") => {
                    (14.0, 0.06, [0.35, 0.55, 0.25, 1.0])
                }
                s if s.contains("cactus") => (8.0, 0.04, [0.45, 0.60, 0.30, 1.0]),
                s if s.contains("bush") => (7.0, 0.04, [0.30, 0.58, 0.22, 1.0]),
                s if s.contains("rock")
                    || s.contains("stone")
                    || s.contains("boulder")
                    || s.contains("cliff") =>
                {
                    (8.0, 0.03, [0.60, 0.58, 0.55, 1.0])
                }
                s if s.contains("mushroom") => (5.0, 0.01, [0.70, 0.55, 0.40, 1.0]),
                s if s.contains("flower") => (3.5, 0.02, [0.90, 0.85, 0.80, 1.0]),
                _ => (3.5, 0.02, [0.40, 0.68, 0.28, 1.0]),
            };
            let ws = vi.scale * type_multiplier;
            (ws, sink, tint, ws * 2.0)
        };

        let mut pos = vi.position;
        pos.y -= sink_factor * world_scale;

        Self {
            position: pos,
            rotation: vi.rotation,
            scale: world_scale,
            mesh_key: vi.vegetation_type.clone(),
            mesh_path: resolved_path,
            bounding_radius,
            tint,
            terrain_normal: vi.terrain_normal,
        }
    }
}

pub fn biome_display_name(biome_str: &str) -> &'static str {
    match biome_str {
        "grassland" => "Grassland",
        "desert" => "Desert",
        "forest" => "Forest",
        "mountain" => "Mountain",
        "tundra" => "Tundra",
        "swamp" => "Swamp",
        "beach" => "Beach",
        "river" => "River",
        "temperate_forest" => "Forest",
        _ => "Unknown",
    }
}

/// Built-in biome options (always available).
const BUILTIN_BIOME_OPTIONS: &[(&str, &str)] = &[
    ("grassland", "Grassland"),
    ("desert", "Desert"),
    ("forest", "Forest"),
    ("mountain", "Mountain"),
    ("tundra", "Tundra"),
    ("swamp", "Swamp"),
    ("beach", "Beach"),
    ("river", "River"),
];

/// Discovered biome pack entry (value key, display name, path to .biomepack.json).
#[derive(Debug, Clone)]
pub struct BiomeOption {
    /// Key used as the selected value (e.g. "grassland" or "pack:namaqualand")
    pub value: String,
    /// Display name in the dropdown
    pub display: String,
}

/// Module-level cache for biome options to avoid filesystem I/O every frame.
static BIOME_OPTIONS_CACHE: std::sync::Mutex<Option<Vec<BiomeOption>>> =
    std::sync::Mutex::new(None);

/// Return cached biome options. Populates the cache on first call.
pub fn cached_biome_options() -> Vec<BiomeOption> {
    let mut guard = BIOME_OPTIONS_CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(ref cached) = *guard {
        return cached.clone();
    }
    let options = all_biome_options();
    *guard = Some(options.clone());
    options
}

/// Invalidate the biome options cache so the next call re-scans the filesystem.
pub fn refresh_biome_options_cache() {
    let mut guard = BIOME_OPTIONS_CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    *guard = None;
}

/// Return built-in biomes plus any `.biomepack.json` files found under `assets/imported/`.
pub fn all_biome_options() -> Vec<BiomeOption> {
    let mut options: Vec<BiomeOption> = BUILTIN_BIOME_OPTIONS
        .iter()
        .map(|(v, d)| BiomeOption {
            value: v.to_string(),
            display: d.to_string(),
        })
        .collect();

    // Scan for generated biome packs
    let import_dir = std::path::Path::new("assets/imported");
    if let Ok(entries) = std::fs::read_dir(import_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            // Look for *.biomepack.json inside each subdirectory
            if let Ok(files) = std::fs::read_dir(&path) {
                for file in files.flatten() {
                    let fp = file.path();
                    if fp.extension().is_some_and(|e| e == "json") {
                        if let Some(name) = fp.file_name().and_then(|n| n.to_str()) {
                            if name.ends_with(".biomepack.json") {
                                // Derive display name from pack contents or filename
                                let display = read_pack_name(&fp).unwrap_or_else(|| {
                                    name.trim_end_matches(".biomepack.json").to_string()
                                });
                                options.push(BiomeOption {
                                    value: format!("pack:{}", fp.display()),
                                    display: format!("{} (Pack)", display),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    options
}

/// Read just the "name" field from a biomepack.json without loading everything.
fn read_pack_name(path: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let val: serde_json::Value = serde_json::from_str(&content).ok()?;
    val.get("name")
        .and_then(|n| n.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_state_creation() {
        let state = TerrainState::new();
        assert_eq!(state.chunk_count(), 0);
        assert!(!state.has_terrain());
    }

    #[test]
    fn test_terrain_state_has_terrain() {
        let state = TerrainState::new();
        assert!(!state.has_terrain());
    }

    #[test]
    fn test_terrain_state_total_vertex_count() {
        let state = TerrainState::new();
        assert_eq!(state.total_vertex_count(), 0);
    }

    #[test]
    fn test_terrain_state_total_index_count() {
        let state = TerrainState::new();
        assert_eq!(state.total_index_count(), 0);
    }

    #[test]
    fn test_terrain_state_total_triangle_count() {
        let state = TerrainState::new();
        assert_eq!(state.total_triangle_count(), 0);
    }

    #[test]
    fn test_terrain_state_chunk_ids() {
        let state = TerrainState::new();
        assert!(state.chunk_ids().is_empty());
    }

    #[test]
    fn test_terrain_state_stats() {
        let state = TerrainState::new();
        let stats = state.stats();
        assert_eq!(stats.chunk_count, 0);
        assert_eq!(stats.total_vertices, 0);
        // New terrain states start as dirty (needing generation)
        assert!(stats.is_dirty);
    }

    // ====================================================================
    // TerrainStats Tests
    // ====================================================================

    #[test]
    fn test_terrain_stats_has_terrain() {
        let no_terrain = TerrainStats {
            chunk_count: 0,
            total_vertices: 0,
            total_indices: 0,
            total_triangles: 0,
            seed: 0,
            is_dirty: false,
        };
        assert!(!no_terrain.has_terrain());

        let with_terrain = TerrainStats {
            chunk_count: 4,
            total_vertices: 1000,
            total_indices: 3000,
            total_triangles: 1000,
            seed: 12345,
            is_dirty: false,
        };
        assert!(with_terrain.has_terrain());
    }

    #[test]
    fn test_terrain_stats_avg_vertices_per_chunk() {
        let stats = TerrainStats {
            chunk_count: 4,
            total_vertices: 1000,
            total_indices: 3000,
            total_triangles: 1000,
            seed: 0,
            is_dirty: false,
        };
        assert!((stats.avg_vertices_per_chunk() - 250.0).abs() < 0.1);
    }

    #[test]
    fn test_terrain_stats_avg_vertices_per_chunk_empty() {
        let stats = TerrainStats {
            chunk_count: 0,
            total_vertices: 0,
            total_indices: 0,
            total_triangles: 0,
            seed: 0,
            is_dirty: false,
        };
        assert!((stats.avg_vertices_per_chunk() - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_terrain_stats_avg_triangles_per_chunk() {
        let stats = TerrainStats {
            chunk_count: 5,
            total_vertices: 1000,
            total_indices: 3000,
            total_triangles: 500,
            seed: 0,
            is_dirty: false,
        };
        assert!((stats.avg_triangles_per_chunk() - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_terrain_stats_avg_triangles_per_chunk_empty() {
        let stats = TerrainStats {
            chunk_count: 0,
            total_vertices: 0,
            total_indices: 0,
            total_triangles: 0,
            seed: 0,
            is_dirty: false,
        };
        assert!((stats.avg_triangles_per_chunk() - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_biome_display_name() {
        assert_eq!(biome_display_name("grassland"), "Grassland");
        assert_eq!(biome_display_name("forest"), "Forest");
        assert_eq!(biome_display_name("unknown"), "Unknown");
    }

    #[test]
    fn test_all_biome_options() {
        let options = all_biome_options();
        assert!(options.len() >= 8);
        assert_eq!(options[0].value, "grassland");
        assert_eq!(options[0].display, "Grassland");
    }

    /// Reproduce the exact editor flow for mountain terrain generation.
    /// This test replicates regenerate_terrain() from terrain_panel.rs.
    #[test]
    #[ignore] // ~7 min in debug mode — run explicitly with --ignored
    fn test_mountain_generation_full_flow() {
        let seed = 42u64;
        let chunk_radius = 2i32;

        // Step 1: Create fresh TerrainState (same as regenerate_terrain)
        let mut state = TerrainState::new();

        // Step 2: Configure for mountain biome
        state.configure(seed, "mountain");

        // Step 3: set_noise_params with defaults (slider values)
        state.set_noise_params(6, 2.0, 0.5, 50.0);

        // Step 4: Apply mountain noise preset (overrides set_noise_params)
        let preset = BiomeNoisePreset {
            base_scale: 0.003,
            base_amplitude: 55.0,
            base_octaves: 6,
            base_persistence: 0.55,
            base_lacunarity: 2.2,
            mountains_enabled: true,
            mountains_scale: 0.002,
            mountains_amplitude: 210.0,
            mountains_octaves: 8,
            detail_enabled: true,
            detail_scale: 0.03,
            detail_amplitude: 8.0,
            erosion_enabled: false,
            erosion_strength: 0.0,
        };
        state.apply_biome_noise_preset(&preset);

        // Step 5: Generate terrain (this is what runs on the background thread)
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            state.generate_terrain(chunk_radius)
        }));

        match &result {
            Ok(Ok(count)) => {
                eprintln!("Mountain generation OK: {count} chunks");
            }
            Ok(Err(e)) => {
                panic!("Mountain generation returned error: {e}");
            }
            Err(panic_info) => {
                panic!("Mountain generation PANICKED: {panic_info:?}");
            }
        }

        let count = result.unwrap().unwrap();
        assert!(count > 0, "Should generate at least 1 chunk");

        // Step 6: Check height stats
        let (min_h, max_h, avg_h) = state.height_stats();
        eprintln!("Heights: min={min_h:.1}, max={max_h:.1}, avg={avg_h:.1}");
        assert!(!min_h.is_nan(), "min height is NaN");
        assert!(!max_h.is_nan(), "max height is NaN");
        assert!(!avg_h.is_nan(), "avg height is NaN");
        assert!(
            max_h > 0.0,
            "Max height should be positive for mountain terrain"
        );

        // Step 7: Check GPU chunks (what gets uploaded to renderer)
        let gpu_chunks = state.get_gpu_chunks();
        eprintln!("GPU chunks: {}", gpu_chunks.len());
        assert!(!gpu_chunks.is_empty(), "GPU chunks should not be empty");

        let total_verts: usize = gpu_chunks.iter().map(|(v, _)| v.len()).sum();
        let total_indices: usize = gpu_chunks.iter().map(|(_, i)| i.len()).sum();
        eprintln!("Total vertices: {total_verts}, indices: {total_indices}");
        assert!(total_verts > 0, "Should have vertices");
        assert!(total_indices > 0, "Should have indices");

        // Step 8: Verify no NaN in vertex positions
        for (chunk_idx, (verts, _)) in gpu_chunks.iter().enumerate() {
            for (v_idx, v) in verts.iter().enumerate() {
                assert!(
                    !v.position[0].is_nan() && !v.position[1].is_nan() && !v.position[2].is_nan(),
                    "NaN position in chunk {chunk_idx}, vertex {v_idx}: {:?}",
                    v.position
                );
                assert!(
                    !v.normal[0].is_nan() && !v.normal[1].is_nan() && !v.normal[2].is_nan(),
                    "NaN normal in chunk {chunk_idx}, vertex {v_idx}: {:?}",
                    v.normal
                );
            }
        }

        // Step 9: Generate scatter (also runs on the thread)
        let scatter_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            state.generate_scatter_placements()
        }));
        match &scatter_result {
            Ok(placements) => {
                eprintln!("Scatter OK: {} placements", placements.len());
            }
            Err(panic_info) => {
                panic!("Scatter generation PANICKED: {panic_info:?}");
            }
        }

        eprintln!("=== Mountain full flow test PASSED ===");
    }

    /// Test ALL biomes generate terrain successfully (not just mountain)
    #[test]
    #[ignore] // ~18 min in debug mode — run explicitly with --ignored
    fn test_all_biomes_generate_terrain() {
        for opt in all_biome_options() {
            let biome_name = &opt.value;
            let mut state = TerrainState::new();
            state.configure(42, biome_name);
            state.set_noise_params(6, 2.0, 0.5, 50.0);

            // Use the same preset logic as the editor
            let preset = match biome_name.as_str() {
                "mountain" => BiomeNoisePreset {
                    base_scale: 0.003,
                    base_amplitude: 55.0,
                    base_octaves: 6,
                    base_persistence: 0.55,
                    base_lacunarity: 2.2,
                    mountains_enabled: true,
                    mountains_scale: 0.002,
                    mountains_amplitude: 210.0,
                    mountains_octaves: 8,
                    detail_enabled: true,
                    detail_scale: 0.03,
                    detail_amplitude: 8.0,
                    erosion_enabled: false,
                    erosion_strength: 0.0,
                },
                _ => BiomeNoisePreset {
                    base_scale: 0.005,
                    base_amplitude: 50.0,
                    base_octaves: 6,
                    base_persistence: 0.5,
                    base_lacunarity: 2.0,
                    mountains_enabled: false,
                    mountains_scale: 0.002,
                    mountains_amplitude: 0.0,
                    mountains_octaves: 4,
                    detail_enabled: true,
                    detail_scale: 0.03,
                    detail_amplitude: 5.0,
                    erosion_enabled: false,
                    erosion_strength: 0.0,
                },
            };
            state.apply_biome_noise_preset(&preset);

            let result = state.generate_terrain(1);
            match result {
                Ok(count) => {
                    let (min_h, max_h, avg_h) = state.height_stats();
                    let gpu = state.get_gpu_chunks();
                    eprintln!(
                        "{biome_name}: {count} chunks, heights=({min_h:.1}, {max_h:.1}, {avg_h:.1}), gpu_chunks={}",
                        gpu.len()
                    );
                    assert!(count > 0, "{biome_name}: no chunks generated");
                    assert!(!gpu.is_empty(), "{biome_name}: no GPU chunks");
                }
                Err(e) => {
                    panic!("{biome_name}: generation failed: {e}");
                }
            }
        }
    }
}
