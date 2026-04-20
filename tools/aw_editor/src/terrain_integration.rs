use astraweave_terrain::{
    elevation_to_biome_weights, BiomeConfig, BiomePack, BiomePackAsset, BiomeType, ChunkId,
    ClimateBias, Heightmap, HeightmapPatch, ScatterConfig, SplatConfig, SplatMapGenerator,
    SplatRule, SplatWeights, TerrainChunk, VegetationInstance, VegetationScatter, WorldConfig,
    WorldGenerator, SEA_LEVEL,
};
use glam::Vec3;
use std::collections::HashMap;
use tracing::{debug, info};

/// Transform data for a terrain asset from fixed_placements.json.
#[allow(dead_code)]
struct TerrainAssetTransform {
    position: [f32; 3],
    scale: [f32; 3],
}

impl TerrainAssetTransform {
    fn identity() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

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

            // Detect biome type from pack name for terrain material selection.
            // This affects splat rules (ground color), noise preset, and scatter spacing.
            let base = pack.name.to_lowercase();
            let biome_type = if base.contains("forest")
                || base.contains("verdant")
                || base.contains("trail")
                || base.contains("jungle")
                || base.contains("woodland")
            {
                BiomeType::Forest
            } else if base.contains("tundra") || base.contains("snow") || base.contains("arctic") {
                BiomeType::Tundra
            } else if base.contains("mountain") || base.contains("alpine") {
                BiomeType::Mountain
            } else if base.contains("swamp") || base.contains("marsh") {
                BiomeType::Swamp
            } else if base.contains("desert")
                || base.contains("dune")
                || base.contains("namaqualand")
                || base.contains("arid")
                || base.contains("savanna")
            {
                BiomeType::Desert
            } else {
                BiomeType::Grassland
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

        // ── Stamp .blend terrain heightmap onto procedural chunks ─────
        // NOTE: stamp_blend_heightmap is disabled for now.
        // The .blend scene (~60m) is far smaller than the procedural terrain
        // (2560m at radius=5). Stamping the sculpted heightmap at 1:1 scale
        // creates a crater where blend heights (~0-12m) replace procedural
        // noise. A future version can re-enable this when scene-scale
        // matching is implemented.
        // self.stamp_blend_heightmap(chunk_size);

        // ── Cross-chunk normal stitching ──────────────────────────────
        // Edge vertices previously used clamped (halved) gradients because
        // no neighbor data was available. Now that all chunks are generated
        // we can fetch the actual neighbor heights and recompute full
        // central-difference normals, eliminating lighting seams.
        self.stitch_edge_normals(chunk_size);

        self.terrain_dirty = false;
        info!(target: "aw_editor::terrain_integration", "Terrain generated: {} chunks (radius={})", count, chunk_radius);
        Ok(count)
    }

    /// Stamp rasterized .blend terrain meshes onto procedural heightmap chunks.
    ///
    /// Loads terrain-category GLB meshes from the BiomePack (e.g. `ground.glb`),
    /// rasterizes them into a heightmap grid, and overwrites procedural height
    /// values in overlapping chunks. A smooth blend zone at the edges prevents
    /// harsh seams between sculpted and procedural terrain.
    ///
    /// After stamping, affected chunks get their mesh vertices regenerated.
    #[allow(dead_code)]
    fn stamp_blend_heightmap(&mut self, chunk_size: f32) {
        let pack = match &self.cached_pack {
            Some((_, p)) => p,
            None => return,
        };

        // Collect terrain-category assets from the BiomePack
        let terrain_assets: Vec<&BiomePackAsset> = pack
            .assets
            .iter()
            .filter(|a| a.category == "terrain" && a.mesh_path.ends_with(".glb"))
            .collect();

        if terrain_assets.is_empty() {
            return;
        }

        // Load GLB files and extract vertex/face data
        let mut extracted_meshes = Vec::new();
        for asset in &terrain_assets {
            let glb_path = pack.root_dir.join(&asset.mesh_path);
            if !glb_path.exists() {
                tracing::warn!("Terrain stamp: GLB not found: {}", glb_path.display());
                continue;
            }
            let bytes = match std::fs::read(&glb_path) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!("Terrain stamp: cannot read {}: {e}", glb_path.display());
                    continue;
                }
            };
            let mesh_data = match astraweave_asset::gltf_loader::load_all_meshes_merged(&bytes) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!("Terrain stamp: failed to load {}: {e}", glb_path.display());
                    continue;
                }
            };

            if mesh_data.positions.is_empty() || mesh_data.indices.is_empty() {
                continue;
            }

            // Convert indices to triangle faces [u32; 3]
            let faces: Vec<[u32; 3]> = mesh_data
                .indices
                .chunks_exact(3)
                .map(|tri| [tri[0], tri[1], tri[2]])
                .collect();

            // Early-skip meshes with no vertices
            let _bounds = match astraweave_blend::heightmap_raster::TerrainBounds::from_vertices(
                &mesh_data.positions,
            ) {
                Some(b) => b,
                None => continue,
            };

            // Apply the fixed placement transform if available.
            // The .blend scene positions terrain meshes at specific locations
            // with specific scales that define the scene layout.
            let transform = self.get_terrain_asset_transform(asset);

            let transformed_positions: Vec<[f32; 3]> = mesh_data
                .positions
                .iter()
                .map(|p| {
                    let scaled = [
                        p[0] * transform.scale[0],
                        p[1] * transform.scale[1],
                        p[2] * transform.scale[2],
                    ];
                    [
                        scaled[0] + transform.position[0],
                        scaled[1] + transform.position[1],
                        scaled[2] + transform.position[2],
                    ]
                })
                .collect();

            let transformed_bounds =
                match astraweave_blend::heightmap_raster::TerrainBounds::from_vertices(
                    &transformed_positions,
                ) {
                    Some(b) => b,
                    None => continue,
                };

            extracted_meshes.push(astraweave_blend::heightmap_raster::ExtractedTerrainMesh {
                name: asset.name.clone(),
                vertices: transformed_positions,
                faces,
                bounds: transformed_bounds,
            });

            info!(
                target: "aw_editor::terrain_integration",
                "Terrain stamp: loaded '{}' ({} verts, bounds X:{:.1}..{:.1} Z:{:.1}..{:.1} Y:{:.1}..{:.1})",
                asset.name,
                mesh_data.positions.len(),
                transformed_bounds.min[0], transformed_bounds.max[0],
                transformed_bounds.min[2], transformed_bounds.max[2],
                transformed_bounds.min[1], transformed_bounds.max[1],
            );
        }

        if extracted_meshes.is_empty() {
            return;
        }

        // Rasterize terrain meshes into a heightmap grid.
        // Use 512 resolution for good fidelity from the sculpted meshes.
        let rasterized = match astraweave_blend::heightmap_raster::rasterize_terrain_meshes(
            &extracted_meshes,
            512,
        ) {
            Ok(hm) => hm,
            Err(e) => {
                tracing::warn!("Terrain stamp: rasterization failed: {e}");
                return;
            }
        };

        info!(
            target: "aw_editor::terrain_integration",
            "Terrain stamp: rasterized {}×{} heightmap, world X:{:.1}..{:.1} Z:{:.1}..{:.1} height:{:.1}..{:.1}",
            rasterized.resolution, rasterized.resolution,
            rasterized.world_min[0], rasterized.world_max[0],
            rasterized.world_min[1], rasterized.world_max[1],
            rasterized.min_height, rasterized.max_height,
        );

        // Blend radius: smooth transition zone (in world units) at the edge
        // of the rasterized region to avoid harsh seams.
        let blend_margin = chunk_size * 0.15;
        let primary_biome = self.primary_biome_type();
        let seed = self.config.seed;
        let mut stamped_count = 0u32;

        // Stamp rasterized heights onto each chunk that overlaps the region
        let chunk_ids: Vec<ChunkId> = self.generated_chunks.keys().copied().collect();
        for chunk_id in chunk_ids {
            let chunk_origin = chunk_id.to_world_pos(chunk_size);
            let chunk_end_x = chunk_origin.x + chunk_size;
            let chunk_end_z = chunk_origin.z + chunk_size;

            // Check if this chunk overlaps the rasterized region
            if chunk_end_x < rasterized.world_min[0]
                || chunk_origin.x > rasterized.world_max[0]
                || chunk_end_z < rasterized.world_min[1]
                || chunk_origin.z > rasterized.world_max[1]
            {
                continue;
            }

            let gen_chunk = match self.generated_chunks.get_mut(&chunk_id) {
                Some(gc) => gc,
                None => continue,
            };

            let hm = gen_chunk.chunk.heightmap_mut();
            let resolution = hm.resolution();
            let cell_size = chunk_size / (resolution - 1) as f32;
            let mut any_modified = false;

            for gz in 0..resolution {
                for gx in 0..resolution {
                    let world_x = chunk_origin.x + gx as f32 * cell_size;
                    let world_z = chunk_origin.z + gz as f32 * cell_size;

                    // Check if this point is within the rasterized region
                    let in_x =
                        world_x >= rasterized.world_min[0] && world_x <= rasterized.world_max[0];
                    let in_z =
                        world_z >= rasterized.world_min[1] && world_z <= rasterized.world_max[1];

                    if !in_x || !in_z {
                        continue;
                    }

                    // Sample the rasterized heightmap
                    let blend_height = rasterized.sample(world_x, world_z);

                    // Compute blend factor based on distance from edge
                    let dist_to_edge = [
                        world_x - rasterized.world_min[0],
                        rasterized.world_max[0] - world_x,
                        world_z - rasterized.world_min[1],
                        rasterized.world_max[1] - world_z,
                    ]
                    .into_iter()
                    .fold(f32::INFINITY, f32::min);

                    let blend_factor = if blend_margin > 0.0 {
                        (dist_to_edge / blend_margin).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };

                    // Smoothstep for natural transition
                    let t = blend_factor * blend_factor * (3.0 - 2.0 * blend_factor);

                    let procedural_height = hm.get_height(gx, gz);
                    let final_height = procedural_height * (1.0 - t) + blend_height * t;

                    hm.set_height(gx, gz, final_height);
                    any_modified = true;
                }
            }

            if any_modified {
                stamped_count += 1;
                // Regenerate mesh vertices for this chunk with stamped heights
                let world_offset = Vec3::new(chunk_origin.x, 0.0, chunk_origin.z);
                let (vertices, indices) = Self::generate_heightmap_mesh(
                    gen_chunk.chunk.heightmap(),
                    gen_chunk.chunk.biome_map(),
                    chunk_size,
                    world_offset,
                    seed,
                    primary_biome,
                );
                gen_chunk.vertices = vertices;
                gen_chunk.indices = indices;
            }
        }

        if stamped_count > 0 {
            info!(
                target: "aw_editor::terrain_integration",
                "Terrain stamp: stamped .blend heightmap onto {} chunks",
                stamped_count,
            );
        }
    }

    /// Look up the fixed placement transform for a terrain asset.
    #[allow(dead_code)]
    fn get_terrain_asset_transform(&self, asset: &BiomePackAsset) -> TerrainAssetTransform {
        let pack = match &self.cached_pack {
            Some((_, p)) => p,
            None => return TerrainAssetTransform::identity(),
        };

        // Try to load fixed placements and find the matching terrain object
        let fp_path = match &pack.fixed_placements_path {
            Some(p) => pack.root_dir.join(p),
            None => return TerrainAssetTransform::identity(),
        };

        let content = match std::fs::read_to_string(&fp_path) {
            Ok(c) => c,
            Err(_) => return TerrainAssetTransform::identity(),
        };

        let placements: Vec<serde_json::Value> = match serde_json::from_str(&content) {
            Ok(p) => p,
            Err(_) => return TerrainAssetTransform::identity(),
        };

        for placement in &placements {
            let name = placement.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if name == asset.name {
                let raw_pos = Self::json_array_to_f32_3(placement.get("position"));
                let raw_scl =
                    Self::json_array_to_f32_3(placement.get("scale")).unwrap_or([1.0, 1.0, 1.0]);
                let bp = raw_pos.unwrap_or([0.0, 0.0, 0.0]);
                // Convert from Blender Z-up [X, Y, Z] to engine Y-up [X, Z, -Y]:
                // Blender X → engine X, Blender Z (up) → engine Y (up),
                // Blender Y (forward) → engine -Z (forward).
                // GLB mesh vertices are already Y-up (glTF exporter converts).
                return TerrainAssetTransform {
                    position: [bp[0], bp[2], -bp[1]],
                    scale: [raw_scl[0], raw_scl[2], raw_scl[1]],
                };
            }
        }

        TerrainAssetTransform::identity()
    }

    #[allow(dead_code)]
    fn json_array_to_f32_3(val: Option<&serde_json::Value>) -> Option<[f32; 3]> {
        let arr = val?.as_array()?;
        if arr.len() >= 3 {
            Some([
                arr[0].as_f64()? as f32,
                arr[1].as_f64()? as f32,
                arr[2].as_f64()? as f32,
            ])
        } else {
            None
        }
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
        // Phase 1.5 (Terrain Material System Campaign §3.5): `primary_biome`
        // now acts as a climate bias for heightmap-driven biome assignment.
        // Set to "grassland" for temperate, "tundra" for cold, "desert" for
        // arid, etc. Replaces the prior single-biome-selector behavior —
        // biome weights are now elevation-driven per-vertex (see
        // `astraweave_terrain::elevation_to_biome_weights`) and `biome_map`
        // is retained only for downstream splat-map generation (material
        // rules rather than biome weights).
        let _ = biome_map; // reserved for future multi-biome-per-chunk work
        let _ = seed; // reserved; biome bands are seed-independent in Phase 1.5
        let climate = ClimateBias::from_primary_biome_str(primary_biome.as_str());
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
                // Bounds-safe access: fall back to 0 height / up normal
                // if vectors are unexpectedly short (defensive against biome edge cases).
                let height = heights.get(biome_idx).copied().unwrap_or(0.0);

                let world_x = world_offset.x + x as f32 * cell_size;
                let world_z = world_offset.z + z as f32 * cell_size;

                let normal = normals.get(biome_idx).copied().unwrap_or(Vec3::Y);

                // Phase 1.5: per-vertex biome weights are driven by vertex
                // world-space elevation relative to sea level, biased by the
                // climate derived from `primary_biome`. The 8-slot output is
                // split slots [0..4] → biome_weights_0, [4..8] → biome_weights_1
                // matching the TerrainVertex packing consumed by the Phase 1
                // forward-lit splat pipeline.
                let biome_weights_8 =
                    elevation_to_biome_weights(height, SEA_LEVEL, climate);
                let mut biome_weights_0 = [0.0f32; 4];
                let mut biome_weights_1 = [0.0f32; 4];
                biome_weights_0.copy_from_slice(&biome_weights_8[0..4]);
                biome_weights_1.copy_from_slice(&biome_weights_8[4..8]);
                let (material_ids, material_weights) = splat_map
                    .get(biome_idx)
                    .copied()
                    .map(Self::splat_weights_to_material_slots)
                    .unwrap_or(([0.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]));

                // One detail texture tile per chunk (256 units). This eliminates
                // visible tiling/crosshatch from any camera height while the
                // 512px multi-octave detail texture retains close-up detail.
                const TERRAIN_UV_TILE_SIZE: f32 = 256.0;
                let tiled_u = world_x / TERRAIN_UV_TILE_SIZE;
                let tiled_v = world_z / TERRAIN_UV_TILE_SIZE;

                vertices.push(TerrainVertex::new(
                    [world_x, height, world_z],
                    [normal.x, normal.y, normal.z],
                    [tiled_u, tiled_v],
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

        // ── Edge skirts ────────────────────────────────────────────────
        // Drop each boundary vertex downward to create a "curtain" that
        // hides gaps between adjacent chunks and prevents the sky dome
        // from showing through the thin heightmap surface edges.
        let skirt_drop = chunk_size * 0.015; // tiny skirt — just enough to hide inter-chunk gaps
        let surface_vert_count = vertices.len() as u32;

        // Helper: duplicate a surface vertex with Y lowered by skirt_drop
        // and normal pointing outward along the edge.
        let add_skirt = |vertices: &mut Vec<TerrainVertex>,
                         indices: &mut Vec<u32>,
                         edge_indices: &[u32],
                         outward_normal: [f32; 3]| {
            let base = vertices.len() as u32;
            for &ei in edge_indices {
                let sv = vertices[ei as usize];
                vertices.push(TerrainVertex::new(
                    [sv.position[0], sv.position[1] - skirt_drop, sv.position[2]],
                    outward_normal,
                    sv.uv,
                    sv.biome_weights_0,
                    sv.biome_weights_1,
                    sv.material_ids,
                    sv.material_weights,
                ));
            }
            // Create quad strip: surface[i] → skirt[i] → surface[i+1] → skirt[i+1]
            for i in 0..(edge_indices.len() - 1) {
                let s0 = edge_indices[i];
                let s1 = edge_indices[i + 1];
                let k0 = base + i as u32;
                let k1 = base + i as u32 + 1;
                // Two triangles per quad segment
                indices.push(s0);
                indices.push(k0);
                indices.push(s1);
                indices.push(s1);
                indices.push(k0);
                indices.push(k1);
            }
        };

        // Bottom edge (z=0): normal points downward so skirt is in shadow
        let bottom_edge: Vec<u32> = (0..resolution).map(|x| x as u32).collect();
        add_skirt(&mut vertices, &mut indices, &bottom_edge, [0.0, -1.0, 0.0]);

        // Top edge (z=resolution-1)
        let top_edge: Vec<u32> = (0..resolution)
            .map(|x| ((resolution - 1) * resolution + x) as u32)
            .collect();
        add_skirt(&mut vertices, &mut indices, &top_edge, [0.0, -1.0, 0.0]);

        // Left edge (x=0)
        let left_edge: Vec<u32> = (0..resolution).map(|z| (z * resolution) as u32).collect();
        add_skirt(&mut vertices, &mut indices, &left_edge, [0.0, -1.0, 0.0]);

        // Right edge (x=resolution-1)
        let right_edge: Vec<u32> = (0..resolution)
            .map(|z| (z * resolution + resolution - 1) as u32)
            .collect();
        add_skirt(&mut vertices, &mut indices, &right_edge, [0.0, -1.0, 0.0]);

        let _ = surface_vert_count; // suppress unused warning

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

    /// Re-compute vertex normals at chunk boundaries using neighbor chunk
    /// height data so the gradient is a full central-difference rather than
    /// a clamped half-gradient. This eliminates the visible lighting seam
    /// between adjacent chunks.
    fn stitch_edge_normals(&mut self, chunk_size: f32) {
        // Collect chunk IDs so we can iterate without holding a mutable borrow
        let ids: Vec<ChunkId> = self.generated_chunks.keys().copied().collect();

        for id in &ids {
            let resolution = {
                let gc = &self.generated_chunks[id];
                gc.chunk.heightmap().resolution() as usize
            };
            let cell_size = chunk_size / (resolution - 1) as f32;

            // Fetch neighbor edge heights (Option<Vec<f32>>)
            // left neighbor  (x-1) → its right edge column  (x = res-1)
            // right neighbor (x+1) → its left edge column   (x = 0)
            // up neighbor    (z-1) → its bottom edge row     (z = res-1)
            // down neighbor  (z+1) → its top edge row        (z = 0)
            let left_heights: Option<Vec<f32>> = self
                .generated_chunks
                .get(&ChunkId {
                    x: id.x - 1,
                    z: id.z,
                })
                .map(|gc| {
                    let res = gc.chunk.heightmap().resolution();
                    (0..res)
                        .map(|z| gc.chunk.heightmap().get_height(res - 1, z))
                        .collect()
                });
            let right_heights: Option<Vec<f32>> = self
                .generated_chunks
                .get(&ChunkId {
                    x: id.x + 1,
                    z: id.z,
                })
                .map(|gc| {
                    let res = gc.chunk.heightmap().resolution();
                    (0..res)
                        .map(|z| gc.chunk.heightmap().get_height(0, z))
                        .collect()
                });
            let up_heights: Option<Vec<f32>> = self
                .generated_chunks
                .get(&ChunkId {
                    x: id.x,
                    z: id.z - 1,
                })
                .map(|gc| {
                    let res = gc.chunk.heightmap().resolution();
                    (0..res)
                        .map(|x| gc.chunk.heightmap().get_height(x, res - 1))
                        .collect()
                });
            let down_heights: Option<Vec<f32>> = self
                .generated_chunks
                .get(&ChunkId {
                    x: id.x,
                    z: id.z + 1,
                })
                .map(|gc| {
                    let res = gc.chunk.heightmap().resolution();
                    (0..res)
                        .map(|x| gc.chunk.heightmap().get_height(x, 0))
                        .collect()
                });

            // Now mutably update the vertex normals for boundary vertices
            let gc = self.generated_chunks.get_mut(id).unwrap();
            let hm = gc.chunk.heightmap();

            for z in 0..resolution {
                for x in 0..resolution {
                    let is_edge_x = x == 0 || x == resolution - 1;
                    let is_edge_z = z == 0 || z == resolution - 1;
                    if !is_edge_x && !is_edge_z {
                        continue; // interior vertex, already correct
                    }

                    let h_center = hm.get_height(x as u32, z as u32);

                    // X gradient: use neighbor chunk data at boundaries
                    let h_left = if x > 0 {
                        hm.get_height((x - 1) as u32, z as u32)
                    } else if let Some(ref lh) = left_heights {
                        lh.get(z).copied().unwrap_or(h_center)
                    } else {
                        h_center
                    };
                    let h_right = if x < resolution - 1 {
                        hm.get_height((x + 1) as u32, z as u32)
                    } else if let Some(ref rh) = right_heights {
                        rh.get(z).copied().unwrap_or(h_center)
                    } else {
                        h_center
                    };

                    // Z gradient: use neighbor chunk data at boundaries
                    let h_up = if z > 0 {
                        hm.get_height(x as u32, (z - 1) as u32)
                    } else if let Some(ref uh) = up_heights {
                        uh.get(x).copied().unwrap_or(h_center)
                    } else {
                        h_center
                    };
                    let h_down = if z < resolution - 1 {
                        hm.get_height(x as u32, (z + 1) as u32)
                    } else if let Some(ref dh) = down_heights {
                        dh.get(x).copied().unwrap_or(h_center)
                    } else {
                        h_center
                    };

                    let dx = (h_right - h_left) / (2.0 * cell_size);
                    let dz = (h_down - h_up) / (2.0 * cell_size);
                    let normal = Vec3::new(-dx, 1.0, -dz).normalize();

                    let vi = z * resolution + x;
                    gc.vertices[vi].normal = [normal.x, normal.y, normal.z];
                }
            }
        }
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
            ids[0] = 1.0;
            ws[0] = 1.0; // fallback to sand (layer 1) — safer default for all biomes
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
                // Catch-all sand base: covers ALL heights/slopes at low priority
                // so that the fallback is always sand (layer 1), never grass (layer 0).
                generator.add_rule(SplatRule {
                    material_id: 1, // sand
                    min_height: f32::MIN,
                    max_height: f32::MAX,
                    min_slope: 0.0,
                    max_slope: 90.0,
                    priority: 1, // lowest priority — overridden by all specific rules
                    weight: 0.5, // moderate base weight
                    height_falloff: 0.0,
                    slope_falloff: 0.0,
                });
                // Sand covers terrain aggressively — arid biomes are sand-dominated.
                // Very high weight (4.0) and wide slope tolerance (0..50) ensures
                // sand is the dominant material even on moderate slopes.
                generator.add_rule(SplatRule {
                    material_id: 1, // sand
                    min_height: -10.0,
                    max_height: 300.0,
                    min_slope: 0.0,
                    max_slope: 50.0, // sand even on moderate slopes in arid biomes
                    priority: 18,    // high priority to dominate
                    weight: 4.0,     // very strong
                    height_falloff: 0.002,
                    slope_falloff: 0.03,
                });
                // Rock only on very steep cliffs (>45°)
                generator.add_rule(SplatRule {
                    material_id: 7, // stone
                    min_height: f32::MIN,
                    max_height: f32::MAX,
                    min_slope: 45.0, // only very steep slopes
                    max_slope: 90.0,
                    priority: 20,
                    weight: 1.0,
                    height_falloff: 0.0,
                    slope_falloff: 0.08,
                });
                generator.add_rule(SplatRule {
                    material_id: 3, // mountain rock on steep slopes
                    min_height: -2.0,
                    max_height: 120.0,
                    min_slope: 30.0, // increased from 8.0 — less rock in desert
                    max_slope: 50.0,
                    priority: 13,
                    weight: 0.35, // reduced from 0.55
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
        debug!(target: "aw_editor::terrain_integration", "Brush stroke begin");
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
            debug!(target: "aw_editor::terrain_integration", "Brush stroke end: no chunks modified");
            None
        } else {
            info!(target: "aw_editor::terrain_integration", "Brush stroke end: {} chunks modified", deltas.len());
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

        // Diagnostic: log biome config vegetation info
        if let Some(primary) = self.config.biomes.first() {
            let veg_count = primary.vegetation.vegetation_types.len();
            let density = primary.vegetation.density;
            eprintln!(
                "SCATTER: generating for {} chunks, biome='{}' veg_types={} density={:.4}",
                self.generated_chunks.len(),
                primary.name,
                veg_count,
                density,
            );
            if veg_count > 0 {
                let sample = &primary.vegetation.vegetation_types[0];
                eprintln!(
                    "SCATTER: first veg type: name='{}' path='{}' weight={:.3}",
                    sample.name, sample.model_path, sample.weight,
                );
            }
        } else {
            eprintln!("SCATTER: WARNING — no biome configs available, scatter will be empty!");
        }

        let mut placements = Vec::new();

        for (chunk_id, gen_chunk) in &self.generated_chunks {
            let chunk = &gen_chunk.chunk;

            // Sample biome at chunk center
            let chunk_center = chunk_id.to_center_pos(chunk_size);
            let center_biome = chunk
                .get_biome_at_world_pos(chunk_center, chunk_size)
                .unwrap_or(BiomeType::Grassland);

            // Biome-dependent minimum distance fallback for the scatter
            // Poisson disk.  In hierarchical mode this is the INITIAL value
            // for the fold that computes per-tier spacing; per-species
            // min_distance values (from BiomeConfig vegetation_types) will
            // take precedence when they are smaller.  Kept at a moderate
            // value so that any species missing an explicit min_distance
            // doesn't collapse to zero spacing.
            let min_dist = match center_biome {
                BiomeType::Forest => 6.0,
                BiomeType::Swamp => 5.0,
                BiomeType::River => 5.0,
                BiomeType::Grassland => 6.0,
                BiomeType::Mountain => 8.0,
                BiomeType::Desert => 10.0,
                BiomeType::Tundra => 8.0,
                BiomeType::Beach => 8.0,
                _ => 6.0,
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

            // Generate vegetation instances via hierarchical scatter system.
            // Hierarchical mode respects per-species placement_priority
            // (trees first, then shrubs, then grass) with separate Poisson
            // disk passes per tier.  Earlier tiers create exclusion zones
            // preventing later placements from overlapping tree trunks.
            let veg_count = biome_config.vegetation.vegetation_types.len();
            let density = biome_config.vegetation.density;
            let vegetation = match scatter.scatter_vegetation_hierarchical(
                chunk,
                chunk_size,
                &biome_config,
                seed,
            ) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("Scatter failed for chunk {:?}: {e}", chunk_id);
                    continue;
                }
            };

            if placements.is_empty() && !vegetation.is_empty() {
                // Log first successful chunk for diagnostics
                tracing::debug!(
                    "=== SCATTER GEN: chunk {:?}: {} instances from {} veg types (density={:.4})",
                    chunk_id,
                    vegetation.len(),
                    veg_count,
                    density,
                );
                if let Some(vi) = vegetation.first() {
                    tracing::debug!(
                        "=== SCATTER INSTANCE: type='{}' path='{}' pos=({:.1},{:.1},{:.1})",
                        vi.vegetation_type,
                        vi.model_path,
                        vi.position.x,
                        vi.position.y,
                        vi.position.z,
                    );
                }
            } else if vegetation.is_empty() && placements.is_empty() {
                tracing::debug!(
                    "=== SCATTER GEN: chunk {:?}: 0 instances (veg_types={}, density={:.4})",
                    chunk_id,
                    veg_count,
                    density,
                );
            }

            // Use pack-aware placement when a BiomePack is loaded.
            // BiomePack assets are modeled at real-world Blender scale, but the
            // procedural terrain spans hundreds of units per chunk. We apply a
            // dimension-aware scale boost: small assets (bushes ~0.5m) get a
            // large boost (~12x) to be visible, while large assets (cliffs ~20m)
            // get a minimal boost (~1x) to stay reasonably sized.
            let pack_ref = self.cached_pack.as_ref().map(|(_, p)| p);

            // Build per-species cull_distance lookup from biome config
            let cull_map: std::collections::HashMap<&str, f32> = biome_config
                .vegetation
                .vegetation_types
                .iter()
                .filter(|vt| vt.cull_distance > 0.0)
                .map(|vt| (vt.name.as_str(), vt.cull_distance))
                .collect();

            for vi in vegetation {
                let mut placement = ScatterPlacement::from_zone_placement(&vi, pack_ref);

                // Apply per-species cull distance from biome config
                if let Some(&cd) = cull_map.get(vi.vegetation_type.as_str()) {
                    placement.cull_distance = cd;
                }

                if pack_ref.is_some() {
                    // BiomePack assets use Blender meters, terrain uses world
                    // units where 1 unit ≈ 1 meter.  The `from_zone_placement`
                    // path already returns `vi.scale` (~0.7-1.3) without a
                    // boost, so at this point the asset renders at roughly its
                    // natural Blender size.  That's correct for 1:1 worlds,
                    // but the procedural terrain spans 256 units per chunk and
                    // the camera sits at Y ≈ 100-200 units, so assets at their
                    // natural Blender scale appear as tiny dots.
                    //
                    // Apply a uniform scale boost that makes assets visible at
                    // typical editor camera distances.  The value is tuned so
                    // that a 1-meter bush becomes ~8 units and a 6-meter tree
                    // becomes ~48 units — clearly visible from camera heights
                    // of 100-500 units.
                    const PACK_SCALE_BOOST: f32 = 8.0;
                    placement.scale *= PACK_SCALE_BOOST;
                    placement.bounding_radius *= PACK_SCALE_BOOST;
                }

                // Tag with source chunk for streaming
                placement.chunk_id = *chunk_id;

                placements.push(placement);
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

/// Resolve a potentially relative mesh path to absolute using the project root (CWD).
/// If the path is already absolute, returns it unchanged.
fn resolve_mesh_path(path: &str) -> String {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        return path.to_string();
    }
    // Try resolving relative to CWD (project root)
    match std::env::current_dir() {
        Ok(cwd) => {
            let resolved = cwd.join(p);
            if resolved.exists() {
                return resolved.to_string_lossy().into_owned();
            }
            // Also try under assets/ subdirectory
            let assets_resolved = cwd.join("assets").join(p);
            if assets_resolved.exists() {
                return assets_resolved.to_string_lossy().into_owned();
            }
            // Return the CWD-joined path even if it doesn't exist yet
            // (it might be loaded later or the file might be missing)
            resolved.to_string_lossy().into_owned()
        }
        Err(_) => path.to_string(),
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
    /// Per-species cull distance. 0.0 = use global max_draw_distance.
    pub cull_distance: f32,
    /// Source chunk for streaming: identifies which terrain chunk this
    /// placement belongs to, enabling per-chunk load/unload as the camera moves.
    pub chunk_id: ChunkId,
}

impl ScatterPlacement {
    pub fn from_vegetation_instance(vi: &VegetationInstance) -> Self {
        // Per-type world-scale multiplier: Nature Kit models are tiny (~1 unit),
        // terrain spans hundreds of units. Trees need a much larger multiplier
        // than grass/flowers to look proportional.
        let (type_multiplier, base_tint) = match vi.vegetation_type.as_str() {
            // Trees — models are ~1.2–1.7 units tall, need to reach 15–30 units
            s if s.contains("tree") || s.contains("pine") => (14.0, [0.90, 1.00, 0.85, 1.0]),
            // Cacti — models ~0.75 units, need to be ~5–8 units
            s if s.contains("cactus") => (8.0, [0.92, 1.00, 0.88, 1.0]),
            // Bushes — models ~0.25 units, need to be ~2–3 units
            s if s.contains("bush") => (7.0, [0.88, 1.00, 0.85, 1.0]),
            // Rocks/boulders/cliffs — models ~0.26 units, need to be ~2–4 units
            s if s.contains("rock")
                || s.contains("stone")
                || s.contains("boulder")
                || s.contains("cliff") =>
            {
                (8.0, [0.95, 0.93, 0.90, 1.0])
            }
            // Mushrooms — small ground detail
            s if s.contains("mushroom") => (5.0, [0.95, 0.90, 0.88, 1.0]),
            // Flowers — keep their original colors mostly
            s if s.contains("flower") => (3.5, [1.00, 0.98, 0.95, 1.0]),
            // Grass, ground cover — models ~0.2 units, OK at ~1–2 units
            _ => (3.5, [0.92, 0.95, 0.88, 1.0]),
        };

        // Per-instance tint variation: derive a deterministic hash from
        // position to add ±12% hue/value jitter, breaking the uniform
        // "plantation" look without requiring a persistent RNG.
        let tint = Self::jitter_tint(base_tint, vi.position);

        // Force per-instance yaw + scale jitter. When the source rotation
        // is zero (a biome with `vegetation.random_rotation` disabled), use
        // the full ±π yaw to break clone lattices. When non-zero, apply an
        // additive ±0.25 rad nudge on top so scale variance still kicks in
        // without fighting the biome's intended orientation distribution.
        let (rotation, scale_jitter) = if vi.rotation.abs() < f32::EPSILON {
            Self::jitter_yaw_scale(vi.position)
        } else {
            let (yaw_delta, scale_mul) = Self::jitter_yaw_scale_additive(vi.position);
            (vi.rotation + yaw_delta, scale_mul)
        };
        let world_scale = vi.scale * type_multiplier * scale_jitter;

        // NOTE: The old sink_factor hack has been removed. Pivot correction
        // is now done at render time in upload_scatter_placements() using the
        // actual model AABB, which provides accurate grounding regardless of
        // model origin placement.
        Self {
            position: vi.position,
            rotation,
            scale: world_scale,
            mesh_key: vi.vegetation_type.clone(),
            mesh_path: resolve_mesh_path(&vi.model_path),
            bounding_radius: world_scale * 2.0,
            tint,
            terrain_normal: vi.terrain_normal,
            cull_distance: 0.0,
            chunk_id: ChunkId::new(0, 0),
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
        let (world_scale, tint, bounding_radius) = if let Some(asset) = pack_asset {
            // Use actual Blender dimensions — no heuristic multiplier needed.
            // The zone scatter system already handles adaptive scaling via
            // AdaptiveScaleParams, so we use scale 1:1 here.
            let dims = asset.dimensions.unwrap_or([1.0, 1.0, 1.0]);
            let max_dim = dims[0].max(dims[1]).max(dims[2]) as f32;
            let radius = max_dim * vi.scale * 0.5;
            // Category-aware tint for visual variety
            let tint = match asset.category.as_str() {
                "vegetation" => [0.90, 1.00, 0.85, 1.0],
                "rock" => [0.95, 0.93, 0.90, 1.0],
                "structure" | "furniture" => [1.00, 0.98, 0.95, 1.0],
                _ => [0.95, 0.95, 0.95, 1.0],
            };
            (vi.scale, tint, radius)
        } else {
            // Fall back to heuristic scaling for non-pack placements
            let (type_multiplier, tint) = match vi.vegetation_type.as_str() {
                s if s.contains("tree") || s.contains("pine") => (14.0, [0.90, 1.00, 0.85, 1.0]),
                s if s.contains("cactus") => (8.0, [0.92, 1.00, 0.88, 1.0]),
                s if s.contains("bush") => (7.0, [0.88, 1.00, 0.85, 1.0]),
                s if s.contains("rock")
                    || s.contains("stone")
                    || s.contains("boulder")
                    || s.contains("cliff") =>
                {
                    (8.0, [0.95, 0.93, 0.90, 1.0])
                }
                s if s.contains("mushroom") => (5.0, [0.95, 0.90, 0.88, 1.0]),
                s if s.contains("flower") => (3.5, [1.00, 0.98, 0.95, 1.0]),
                _ => (3.5, [0.92, 0.95, 0.88, 1.0]),
            };
            let ws = vi.scale * type_multiplier;
            (ws, tint, ws * 2.0)
        };

        // NOTE: No sink_factor hack. Pivot correction is done at render time
        // using the actual model AABB in upload_scatter_placements().

        // Apply deterministic per-instance yaw + scale jitter. Zero source
        // rotation → full ±π yaw; non-zero → additive ±0.25 rad nudge. Scale
        // jitter always applies so identical clones break up even when the
        // biome already randomises rotation.
        let (rotation, final_scale) = if vi.rotation.abs() < f32::EPSILON {
            let (yaw, scale_mul) = Self::jitter_yaw_scale(vi.position);
            (yaw, world_scale * scale_mul)
        } else {
            let (yaw_delta, scale_mul) = Self::jitter_yaw_scale_additive(vi.position);
            (vi.rotation + yaw_delta, world_scale * scale_mul)
        };
        let final_radius = bounding_radius * (final_scale / world_scale.max(f32::EPSILON));

        Self {
            position: vi.position,
            rotation,
            scale: final_scale,
            mesh_key: vi.vegetation_type.clone(),
            mesh_path: resolved_path,
            bounding_radius: final_radius,
            tint,
            terrain_normal: vi.terrain_normal,
            cull_distance: 0.0,
            chunk_id: ChunkId::new(0, 0),
        }
    }

    /// Deterministic per-instance tint jitter derived from world position.
    ///
    /// Applies ±12% variation to each RGB channel, breaking the uniform
    /// "plantation" look without requiring a persistent RNG.
    fn jitter_tint(base: [f32; 4], pos: Vec3) -> [f32; 4] {
        // Simple spatial hash: bit-mix the quantised position coordinates
        let ix = (pos.x * 73.13) as i32;
        let iz = (pos.z * 119.97) as i32;
        let hash =
            ((ix.wrapping_mul(374761393)) ^ (iz.wrapping_mul(668265263))).wrapping_add(1013904223);

        // Extract three independent ±1 float channels from the hash bits
        let r_off = ((hash & 0xFF) as f32 / 255.0 - 0.5) * 0.24; // ±12%
        let g_off = (((hash >> 8) & 0xFF) as f32 / 255.0 - 0.5) * 0.24;
        let b_off = (((hash >> 16) & 0xFF) as f32 / 255.0 - 0.5) * 0.24;

        [
            (base[0] + r_off).clamp(0.0, 1.5),
            (base[1] + g_off).clamp(0.0, 1.5),
            (base[2] + b_off).clamp(0.0, 1.5),
            base[3],
        ]
    }

    /// Deterministic per-instance yaw + scale jitter derived from world
    /// position. Used when the source `VegetationInstance.rotation` is
    /// zero (a common default when `biome_config.vegetation.random_rotation`
    /// is off), which would otherwise produce a visible lattice of identical
    /// clones at oblique viewing angles.
    ///
    /// - Yaw: full ±π range (uniform over the unit circle).
    /// - Scale: ±10 % multiplicative variation (guarantees silhouette break-up
    ///   even when a biome species uses a near-constant `scale_range`).
    fn jitter_yaw_scale(pos: Vec3) -> (f32, f32) {
        // Distinct mixing constants from `jitter_tint` so yaw and tint are
        // uncorrelated.
        let ix = (pos.x * 91.37) as i32;
        let iz = (pos.z * 147.53) as i32;
        let hash =
            ((ix.wrapping_mul(2654435761u32 as i32)) ^ (iz.wrapping_mul(40503))).wrapping_add(2166136261u32 as i32);

        let yaw_unit = ((hash as u32) & 0xFFFF) as f32 / 65535.0; // 0..1
        let yaw = (yaw_unit - 0.5) * std::f32::consts::TAU; // -π..π
        let scale_unit = (((hash as u32) >> 16) & 0xFFFF) as f32 / 65535.0; // 0..1
        let scale_mul = 0.90 + scale_unit * 0.20; // 0.90..1.10
        (yaw, scale_mul)
    }

    /// Deterministic additive yaw perturbation and scale multiplier to apply
    /// *on top of* a non-zero source rotation.
    ///
    /// Used when the biome config already randomised rotation: the source
    /// yaw gives orientation diversity but assets often still share scales,
    /// so identical silhouettes read as a grid at oblique angles. This
    /// function contributes a narrow ±0.25 rad (~±14°) yaw nudge plus the
    /// same ±10 % scale variation used when no source rotation is present.
    fn jitter_yaw_scale_additive(pos: Vec3) -> (f32, f32) {
        let (full_yaw, scale_mul) = Self::jitter_yaw_scale(pos);
        // Compress the full-range yaw into a narrow delta so we don't undo
        // the biome's intentional orientation.
        let yaw_delta = full_yaw * (0.25 / std::f32::consts::PI); // ±0.25 rad
        (yaw_delta, scale_mul)
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
                tracing::debug!("Mountain generation OK: {count} chunks");
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
        tracing::debug!("Heights: min={min_h:.1}, max={max_h:.1}, avg={avg_h:.1}");
        assert!(!min_h.is_nan(), "min height is NaN");
        assert!(!max_h.is_nan(), "max height is NaN");
        assert!(!avg_h.is_nan(), "avg height is NaN");
        assert!(
            max_h > 0.0,
            "Max height should be positive for mountain terrain"
        );

        // Step 7: Check GPU chunks (what gets uploaded to renderer)
        let gpu_chunks = state.get_gpu_chunks();
        tracing::debug!("GPU chunks: {}", gpu_chunks.len());
        assert!(!gpu_chunks.is_empty(), "GPU chunks should not be empty");

        let total_verts: usize = gpu_chunks.iter().map(|(v, _)| v.len()).sum();
        let total_indices: usize = gpu_chunks.iter().map(|(_, i)| i.len()).sum();
        tracing::debug!("Total vertices: {total_verts}, indices: {total_indices}");
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
                tracing::debug!("Scatter OK: {} placements", placements.len());
            }
            Err(panic_info) => {
                panic!("Scatter generation PANICKED: {panic_info:?}");
            }
        }

        tracing::debug!("=== Mountain full flow test PASSED ===");
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
                    tracing::debug!(
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
