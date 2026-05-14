use astraweave_terrain::{
    elevation_to_biome_weights, smooth_shared_vertices, BiomeConfig, BiomePack, BiomePackAsset,
    BiomeType, ChunkId, ClimateBias, Heightmap, HeightmapPatch, ScatterConfig, SplatConfig,
    SplatMapGenerator, SplatRule, SplatWeights, TerrainChunk, VegetationInstance,
    VegetationScatter, WorldConfig, WorldGenerator, SEA_LEVEL,
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

// Phase 1.6-F.4.B.3.D.3c: `BiomeNoisePreset` REMOVED. Replaced by the
// climate-field architecture (D.1) + Whittaker biome lookup (D.2) +
// per-biome `BiomeParameters` (D.3a) which the terrain crate's
// `WorldGenerator::generate_chunk_with_climate` consumes per-vertex.
// Editor terrain generation now relies on `WorldConfig::default()`
// (Continental Temperate baseline); D.5 will replace the "Primary
// Biome" dropdown with a "World Archetype" dropdown that drives the
// climate field's archetype envelope per-vertex.

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

/// Terrain vertex (CPU-side, mirroring viewport::types::TerrainVertex layout).
///
/// Real-Fix.C 2026-05-08: unified `biome_weights_0/1` and `material_ids/
/// material_weights` into a single canonical material attribute set
/// (Option C per Andrew-gate decision). Resolves §7.7 sibling-attribute
/// drift trap at texture-data layer (Round 7 evidence). Splat textures are
/// rebuilt directly from `material_ids/material_weights`; biome blending
/// at higher abstraction layers (astraweave-terrain) preserved per Model A.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TerrainVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    /// Material texture layer indices (0-7 valid; mapped to 8-channel splat)
    /// packed as f32 for vertex attribute compatibility.
    pub material_ids: [f32; 4],
    /// Blend weights for each material slot (sum to 1.0).
    pub material_weights: [f32; 4],
}

impl TerrainVertex {
    pub fn new(
        position: [f32; 3],
        normal: [f32; 3],
        uv: [f32; 2],
        material_ids: [f32; 4],
        material_weights: [f32; 4],
    ) -> Self {
        Self {
            position,
            normal,
            uv,
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

    // Phase 1.6-F.4.B.3.D.3c: `apply_biome_noise_preset` REMOVED. Per-vertex
    // biome assignment now lives in `WorldGenerator::generate_chunk_with_climate`
    // (terrain crate). Editor's `regenerate_terrain` no longer mutates
    // `WorldConfig.noise` based on a single picked biome — instead, every
    // vertex looks up its own `BiomeId` from the climate field and applies
    // per-biome `BiomeParameters`. D.5 replaces the "Primary Biome" dropdown
    // with a "World Archetype" dropdown.

    /// Phase 1.6-F.4.B.3.D.5b: set the climate field's `WorldArchetype`
    /// envelope. Drives per-vertex climate sampling → biome lookup →
    /// per-biome parameter selection.
    pub fn set_world_archetype(
        &mut self,
        archetype: astraweave_terrain::climate::WorldArchetype,
    ) {
        if self.config.climate.archetype != archetype {
            self.config.climate.archetype = archetype;
            self.terrain_dirty = true;
        }
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
        // Phase 1.6-F.3-phase-1: derive ClimateBias from the primary_biome
        // string for the new `generate_chunk_with_climate` path, which
        // populates pre-erosion biome_weights on the chunk.
        let climate_bias =
            ClimateBias::from_primary_biome_str(primary_biome.as_str());

        let generator = self
            .generator
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Generator not initialized"))?;

        let chunk_size = self.config.chunk_size;

        // Phase 1.6-F.3-phase-4.B: split chunk generation from mesh assembly
        // so the shared-vertex averaging pass can reconcile boundary heights
        // across all chunks before any mesh is built.
        //
        // Phase 1.6-F.4.B.2.D: rayon-parallelize Pass 1. Target B radius-10
        // generates 441 chunks; single-threaded at 512 WU × 96² chunks takes
        // 10-20 minutes. Per-chunk generation is thread-safe: `&self` on
        // `WorldGenerator::generate_chunk_with_climate`, no shared mutable
        // state. `TerrainNoise` uses `Box<dyn NoiseFn + Send + Sync>` per
        // F.3-phase-2.E audit. Each chunk's halo seed is deterministic from
        // `(world_seed, chunk_id)` (phase-3.C world-coord seeding), so output
        // is determinism-preserving regardless of thread scheduling order.
        //
        // Pass 1: generate all chunks in parallel, apply primary-biome
        // override.
        use rayon::prelude::*;
        let chunk_ids: Vec<ChunkId> = (-chunk_radius..=chunk_radius)
            .flat_map(|x| (-chunk_radius..=chunk_radius).map(move |z| ChunkId { x, z }))
            .collect();

        let gen_ref = &*generator;
        let chunks_vec: anyhow::Result<Vec<(ChunkId, TerrainChunk)>> = chunk_ids
            .into_par_iter()
            .map(|chunk_id| {
                let mut chunk =
                    gen_ref.generate_chunk_with_climate(chunk_id, climate_bias)?;
                for b in chunk.biome_map_mut() {
                    *b = primary_biome;
                }
                Ok((chunk_id, chunk))
            })
            .collect();

        let raw_chunks: HashMap<ChunkId, TerrainChunk> = chunks_vec?.into_iter().collect();
        let mut raw_chunks = raw_chunks;

        // Phase 1.6-F.3-phase-4.B: reconcile shared-edge vertices across all
        // generated chunks. Biome weights are already byte-identical at
        // shared edges (Shape A invariant, verified by phase-4.A diagnostic);
        // only heights need averaging. Normals recompute naturally in
        // `generate_heightmap_mesh` from the updated heights.
        smooth_shared_vertices(&mut raw_chunks);

        // Pass 2: build meshes from smoothed chunks and populate
        // `generated_chunks`.
        let mut count = 0;
        // Drain in a deterministic order so index ordering is stable.
        let mut chunk_ids: Vec<ChunkId> = raw_chunks.keys().copied().collect();
        chunk_ids.sort_by(|a, b| a.x.cmp(&b.x).then(a.z.cmp(&b.z)));
        for chunk_id in chunk_ids {
            let chunk = raw_chunks.remove(&chunk_id).expect("present");

            let world_pos = chunk_id.to_world_pos(chunk_size);
            let world_offset = Vec3::new(world_pos.x, 0.0, world_pos.z);

            // Phase 1.6-F.3-phase-1: pass the pre-erosion biome_weights
            // captured in the chunk during generation. Shape A invariant
            // survives phase-4.B's height averaging because weights are
            // unchanged.
            let pre_erosion_biome_weights =
                chunk.biome_weights().map(|slice| slice.to_vec());

            let (vertices, indices) = Self::generate_heightmap_mesh(
                chunk.heightmap(),
                chunk.biome_map(),
                chunk_size,
                world_offset,
                self.config.seed,
                primary_biome,
                pre_erosion_biome_weights.as_deref(),
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
                // Regenerate mesh vertices for this chunk with stamped heights.
                // Phase 1.6-F.3-phase-1: re-use the chunk's pre-erosion
                // biome_weights so §2.5 authorial-intent stability is
                // preserved across stamping (painted biomes stay put even
                // though heightmap changed).
                let world_offset = Vec3::new(chunk_origin.x, 0.0, chunk_origin.z);
                let stamped_weights =
                    gen_chunk.chunk.biome_weights().map(|slice| slice.to_vec());
                let (vertices, indices) = Self::generate_heightmap_mesh(
                    gen_chunk.chunk.heightmap(),
                    gen_chunk.chunk.biome_map(),
                    chunk_size,
                    world_offset,
                    seed,
                    primary_biome,
                    stamped_weights.as_deref(),
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
        // Phase 1.6-F.3-phase-1: optional pre-erosion biome_weights. When
        // `Some`, each vertex's weight vector comes straight from this slice
        // (in row-major order matching heightmap indexing). When `None`, the
        // function falls back to computing weights from the current
        // (post-erosion) heightmap — preserves pre-F.3 behavior for any
        // caller that doesn't populate biome_weights upstream.
        pre_erosion_biome_weights: Option<&[[f32; 8]]>,
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
                // climate derived from `primary_biome`. The 8-slot output
                // feeds the canonical `material_ids/material_weights` vertex
                // attribute set post-Real-Fix.C: top-4 slot extraction via
                // `splat_weights_to_material_slots_from_array`.
                //
                // Phase 1.6-F.3-phase-1: when `pre_erosion_biome_weights` is
                // provided, use those directly — they were computed from the
                // PRE-erosion heightmap in `WorldGenerator::generate_chunk_with_climate`.
                // Otherwise fall back to computing from the current heights
                // (pre-F.3 behavior for any caller that doesn't populate them).
                //
                // Real-Fix.C 2026-05-08: when splat_map source is absent,
                // derive material_* from biome_weights_8 directly via top-4
                // slot extraction (preserves visual biome blending; pre-fix
                // fallback defaulted to material[0] which would have lost
                // blending entirely post-unification).
                let biome_weights_8: [f32; 8] = pre_erosion_biome_weights
                    .and_then(|slice| slice.get(biome_idx).copied())
                    .unwrap_or_else(|| {
                        elevation_to_biome_weights(height, SEA_LEVEL, climate)
                    });
                let (material_ids, material_weights) = splat_map
                    .get(biome_idx)
                    .copied()
                    .map(Self::splat_weights_to_material_slots)
                    .unwrap_or_else(|| {
                        Self::biome_weights_8_to_material_slots(biome_weights_8)
                    });

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

    /// Cleanup-D fix-pass (M-SK): map an edge surface-vertex grid position
    /// `(gx, gz)` in a chunk of resolution × resolution surface vertices to
    /// the corresponding skirt-vertex indices in `gen_chunk.vertices`.
    ///
    /// Skirt layout (must mirror `generate_heightmap_mesh` skirt order at
    /// lines 861-919): four edges added in sequence — bottom (z=0), top
    /// (z=res-1), left (x=0), right (x=res-1) — each contributing
    /// `resolution` skirt vertices in the same per-edge order as the
    /// surface edge_indices vector. Total skirt vertex count: 4 × resolution.
    ///
    /// Returns up to two indices:
    /// - Interior vertex (not on any edge): both slots `None`.
    /// - Single-edge vertex: first slot `Some(idx)`, second `None`.
    /// - Corner vertex: both slots `Some(idx)` (corner participates in two
    ///   adjacent edges' skirts; e.g. (0, 0) is in bottom AND left skirts).
    ///
    /// A vertex can be in AT MOST 2 skirts: only edges (1) or corners (2).
    fn compute_skirt_indices(gx: u32, gz: u32, resolution: u32) -> [Option<usize>; 2] {
        let res = resolution as usize;
        let surface = res * res;
        let mut out: [Option<usize>; 2] = [None, None];
        let mut slot = 0usize;

        // Bottom skirt: indices [surface .. surface + res), in x order
        if gz == 0 && slot < 2 {
            out[slot] = Some(surface + gx as usize);
            slot += 1;
        }
        // Top skirt: indices [surface + res .. surface + 2*res), in x order
        if gz == resolution.saturating_sub(1) && resolution > 1 && slot < 2 {
            out[slot] = Some(surface + res + gx as usize);
            slot += 1;
        }
        // Left skirt: indices [surface + 2*res .. surface + 3*res), in z order
        if gx == 0 && slot < 2 {
            out[slot] = Some(surface + 2 * res + gz as usize);
            slot += 1;
        }
        // Right skirt: indices [surface + 3*res .. surface + 4*res), in z order
        if gx == resolution.saturating_sub(1) && resolution > 1 && slot < 2 {
            out[slot] = Some(surface + 3 * res + gz as usize);
            // slot += 1; // not needed; loop ends here
        }

        out
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
        Self::biome_weights_8_to_material_slots([
            weights.weights_0.x,
            weights.weights_0.y,
            weights.weights_0.z,
            weights.weights_0.w,
            weights.weights_1.x,
            weights.weights_1.y,
            weights.weights_1.z,
            weights.weights_1.w,
        ])
    }

    /// Convert dense 8-slot biome weight array into top-4 material slots
    /// (ids + weights). Slot index IS the material/layer ID (0-7).
    /// Real-Fix.C 2026-05-08: shared core logic between splat-map-driven
    /// generation (via splat_weights_to_material_slots) and elevation-derived
    /// fallback path (when splat_map source is absent). Resolves §7.7 trap:
    /// vertex storage now exclusively uses material_*; this conversion at
    /// generation time replaces the prior `biome_weights_0/1` direct write.
    fn biome_weights_8_to_material_slots(weights_8: [f32; 8]) -> ([f32; 4], [f32; 4]) {
        // Collect all (channel_as_layer_id, weight) pairs.
        let mut entries: [(f32, f32); 8] = [
            (0.0, weights_8[0]),
            (1.0, weights_8[1]),
            (2.0, weights_8[2]),
            (3.0, weights_8[3]),
            (4.0, weights_8[4]),
            (5.0, weights_8[5]),
            (6.0, weights_8[6]),
            (7.0, weights_8[7]),
        ];

        // Sort by weight descending to find top 4.
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top 4.
        let mut top4: [(f32, f32); 4] = [entries[0], entries[1], entries[2], entries[3]];

        // Sort top 4 by material_id ascending for consistent slot assignment
        // across adjacent vertices (prevents interpolation artifacts).
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

        // Cleanup-D fix-pass (M-D5): compute avg_height GLOBALLY across all
        // chunks within brush radius BEFORE per-chunk dispatch. Pre-fix
        // computed avg_height per-chunk inside the dispatch loop, so when a
        // brush spanned two chunks each chunk settled toward a different
        // average — erosion's 10% settling produced a height step at the
        // shared boundary (Andrew-gate observation 2026-05-07: "when using
        // the erosion tool it exposes stitching seams in the chunk
        // boundaries"). Computing globally first unifies the settling
        // target across all affected chunks. Affects Smooth + Erode only;
        // other brush modes don't reference avg_height.
        let global_avg_height = if matches!(brush_mode, BrushMode::Smooth | BrushMode::Erode) {
            let mut sum = 0.0f32;
            let mut count = 0u32;
            for chunk_id in &chunk_ids {
                let chunk_origin_x = chunk_id.x as f32 * chunk_size;
                let chunk_origin_z = chunk_id.z as f32 * chunk_size;
                // Same AABB rejection as PASS 2 to skip non-overlapping chunks.
                let closest_x = world_x.clamp(chunk_origin_x, chunk_origin_x + chunk_size);
                let closest_z = world_z.clamp(chunk_origin_z, chunk_origin_z + chunk_size);
                let cdx = world_x - closest_x;
                let cdz = world_z - closest_z;
                if cdx * cdx + cdz * cdz > radius * radius {
                    continue;
                }
                if let Some(gen_chunk) = self.generated_chunks.get(chunk_id) {
                    let resolution = gen_chunk.chunk.heightmap().resolution();
                    let cell_size = chunk_size / (resolution - 1) as f32;
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

                // Cleanup-D fix-pass (M-D5): use the global avg_height computed
                // above instead of recomputing per-chunk. Pathway-equivalence
                // with the initial-generation pathway: erosion/smooth settling
                // is uniform across chunk boundaries.
                let avg_height = global_avg_height;

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
                    // Fast path: patch vertex heights and normals in-place.
                    // Cleanup-D fix-pass (M-SK): also update skirt vertex heights
                    // for edge surface vertices so the skirt curtain follows
                    // brush mutations instead of staying at the pre-stroke
                    // height (pathway-equivalence with the initial-generation
                    // pathway in generate_heightmap_mesh at lines 861-919).
                    let cell_size_patch = chunk_size / (resolution - 1) as f32;
                    let skirt_drop = chunk_size * 0.015; // matches generate_heightmap_mesh
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

                                // M-SK: propagate height change to corresponding
                                // skirt vertex/vertices (up to two — corners
                                // participate in two edges' skirts).
                                let skirt_idxs = Self::compute_skirt_indices(
                                    gx as u32,
                                    gz as u32,
                                    resolution,
                                );
                                for maybe in skirt_idxs.iter() {
                                    if let Some(skirt_idx) = *maybe {
                                        if skirt_idx < gen_chunk.vertices.len() {
                                            gen_chunk.vertices[skirt_idx].position[1] =
                                                new_h - skirt_drop;
                                        }
                                    }
                                }
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

        // Cleanup-D fix-pass (M-D9): if any chunk was modified, recompute
        // edge-vertex normals across all chunk boundaries via the existing
        // cross-chunk stitcher. Pre-fix, stitch_edge_normals was only called
        // at initial generation (line 423), so post-brush edge normals
        // diverged between adjacent chunks (calculate_normal's clamped
        // half-gradient at chunk edges) → lighting seam. Pathway-equivalence
        // with the initial-generation pathway: brush dispatch now produces
        // the same edge-normal end-state as generate_terrain.
        if modified {
            self.stitch_edge_normals(chunk_size);
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
                    // Phase 1.6-F.3-phase-1: preserve pre-erosion biome_weights
                    // across biome-paint edits per §2.5 authorial-intent
                    // stability.
                    let painted_weights =
                        gen_chunk.chunk.biome_weights().map(|slice| slice.to_vec());
                    let (vertices, indices) = Self::generate_heightmap_mesh(
                        gen_chunk.chunk.heightmap(),
                        gen_chunk.chunk.biome_map(),
                        chunk_size,
                        world_offset,
                        self.config.seed,
                        primary_biome,
                        painted_weights.as_deref(),
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

    /// Real-Fix.E 2026-05-08: ZoneBlend brush — biome boundary feathering.
    ///
    /// Active brush mode per Andrew-gate (l) l-1. Blends a vertex's biome
    /// composition with its neighborhood by:
    ///   1. expanding sparse `(material_ids, material_weights)` to dense
    ///      32-channel weights (Real-Fix.D MAX_TERRAIN_LAYERS capacity),
    ///   2. computing a distance-weighted average across neighborhood
    ///      vertices (sample radius = `radius * 1.5` per Smooth brush
    ///      analog),
    ///   3. re-sparsifying back to top-4 via the same logic as
    ///      `biome_weights_8_to_material_slots` (Real-Fix.C precedent,
    ///      extended to 32 channels),
    ///   4. lerping current vertex slots toward the blended target by
    ///      `strength * falloff_curve.eval(t)`.
    ///
    /// Does NOT modify height (biome layer only). Material assignments
    /// flow naturally from the blended sparse data via the existing
    /// splat-build pipeline (Real-Fix.D 32-channel splats).
    pub fn apply_brush_zoneblend(
        &mut self,
        world_x: f32,
        world_z: f32,
        radius: f32,
        strength: f32,
        falloff_curve: crate::panels::terrain_panel::FalloffCurve,
    ) -> bool {
        let chunk_size = self.config.chunk_size;
        let sample_radius = radius * 1.5;
        let sample_radius_sq = sample_radius * sample_radius;
        let mut modified = false;

        let chunk_ids: Vec<ChunkId> = self.generated_chunks.keys().cloned().collect();

        // First pass: collect neighborhood samples from all overlapping chunks.
        // Each sample is `(position_xz, dense_32_channels)`.
        let mut samples: Vec<([f32; 2], [f32; 32])> = Vec::new();
        for chunk_id in &chunk_ids {
            let chunk_origin_x = chunk_id.x as f32 * chunk_size;
            let chunk_origin_z = chunk_id.z as f32 * chunk_size;
            let closest_x = world_x.clamp(chunk_origin_x, chunk_origin_x + chunk_size);
            let closest_z = world_z.clamp(chunk_origin_z, chunk_origin_z + chunk_size);
            let dx = world_x - closest_x;
            let dz = world_z - closest_z;
            if dx * dx + dz * dz > sample_radius_sq {
                continue;
            }
            if let Some(gen_chunk) = self.generated_chunks.get(chunk_id) {
                for vertex in &gen_chunk.vertices {
                    let vx = vertex.position[0];
                    let vz = vertex.position[2];
                    let ddx = vx - world_x;
                    let ddz = vz - world_z;
                    if ddx * ddx + ddz * ddz > sample_radius_sq {
                        continue;
                    }
                    samples.push((
                        [vx, vz],
                        Self::expand_sparse_to_dense_32(
                            vertex.material_ids,
                            vertex.material_weights,
                        ),
                    ));
                }
            }
        }

        if samples.is_empty() {
            return false;
        }

        // Second pass: blend affected vertices toward the distance-weighted
        // average of the neighborhood samples.
        let radius_sq = radius * radius;
        for chunk_id in chunk_ids {
            let chunk_origin_x = chunk_id.x as f32 * chunk_size;
            let chunk_origin_z = chunk_id.z as f32 * chunk_size;
            let closest_x = world_x.clamp(chunk_origin_x, chunk_origin_x + chunk_size);
            let closest_z = world_z.clamp(chunk_origin_z, chunk_origin_z + chunk_size);
            let dx = world_x - closest_x;
            let dz = world_z - closest_z;
            if dx * dx + dz * dz > radius_sq {
                continue;
            }

            if let Some(gen_chunk) = self.generated_chunks.get_mut(&chunk_id) {
                let mut chunk_modified = false;
                for vertex in gen_chunk.vertices.iter_mut() {
                    let vx = vertex.position[0];
                    let vz = vertex.position[2];
                    let ddx = vx - world_x;
                    let ddz = vz - world_z;
                    let dist_sq = ddx * ddx + ddz * ddz;
                    if dist_sq > radius_sq {
                        continue;
                    }
                    let dist = dist_sq.sqrt();
                    let t = (dist / radius).clamp(0.0, 1.0);
                    let influence = strength * falloff_curve.eval(t);
                    if influence <= 0.0 {
                        continue;
                    }

                    // Compute distance-weighted average of neighborhood
                    // dense-32 channels around this vertex.
                    let target = Self::weighted_average_dense_32(&samples, vx, vz, sample_radius);
                    let current = Self::expand_sparse_to_dense_32(
                        vertex.material_ids,
                        vertex.material_weights,
                    );
                    let blended = Self::lerp_dense_32(&current, &target, influence);
                    let (ids, ws) = Self::top4_from_dense_32(blended);
                    vertex.material_ids = ids;
                    vertex.material_weights = ws;
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

    /// Real-Fix.E 2026-05-08: passive global ZoneBlend pass per Andrew-gate
    /// (l) l-1. One-shot operation: iterates all terrain vertices, detects
    /// vertices on/near biome boundaries (via dense-channel variance across
    /// a small neighborhood), and applies a modest blend with configurable
    /// `strength` (default 0.3 per Andrew-gate (m) m-3). Non-boundary
    /// vertices are skipped.
    ///
    /// Returns true if any vertices were modified.
    pub fn apply_zoneblend_pass(&mut self, strength: f32) -> bool {
        // Tuned defaults: small sample radius keeps the variance check
        // local-only (so the pass affects only boundary vertices, not the
        // entire terrain). Variance threshold chosen so a vertex whose
        // top-4 sparse representation differs from at least one neighbor
        // by ≥ ~10% per dominant channel triggers the blend.
        let sample_radius = 8.0_f32;
        let variance_threshold = 0.05_f32;
        let mut modified = false;

        let chunk_ids: Vec<ChunkId> = self.generated_chunks.keys().cloned().collect();

        // Pre-compute every vertex's dense representation + world position.
        // Two-pass design avoids borrow conflicts: read-only collect first,
        // then mutate during second pass.
        let mut all_samples: Vec<([f32; 2], [f32; 32])> = Vec::new();
        for chunk_id in &chunk_ids {
            if let Some(gen_chunk) = self.generated_chunks.get(chunk_id) {
                for vertex in &gen_chunk.vertices {
                    all_samples.push((
                        [vertex.position[0], vertex.position[2]],
                        Self::expand_sparse_to_dense_32(
                            vertex.material_ids,
                            vertex.material_weights,
                        ),
                    ));
                }
            }
        }

        if all_samples.is_empty() {
            return false;
        }

        let sample_radius_sq = sample_radius * sample_radius;
        for chunk_id in chunk_ids {
            if let Some(gen_chunk) = self.generated_chunks.get_mut(&chunk_id) {
                let mut chunk_modified = false;
                for vertex in gen_chunk.vertices.iter_mut() {
                    let vx = vertex.position[0];
                    let vz = vertex.position[2];

                    // Find neighbors within sample_radius (linear scan;
                    // acceptable for one-shot passive pass).
                    let neighbors: Vec<&([f32; 2], [f32; 32])> = all_samples
                        .iter()
                        .filter(|(pos, _)| {
                            let ddx = pos[0] - vx;
                            let ddz = pos[1] - vz;
                            ddx * ddx + ddz * ddz <= sample_radius_sq
                        })
                        .collect();
                    if neighbors.len() < 2 {
                        continue;
                    }

                    // Boundary detection: per-channel variance across
                    // neighborhood. If max variance exceeds threshold,
                    // vertex is on a biome boundary.
                    let neighbor_densities: Vec<&[f32; 32]> =
                        neighbors.iter().map(|(_, dense)| dense).collect();
                    let variance =
                        Self::dense_32_max_channel_variance(&neighbor_densities);
                    if variance < variance_threshold {
                        continue;
                    }

                    // Compute the dense average of all neighbors (uniform
                    // weighting for passive pass; the variance check
                    // already filters to boundary vertices).
                    let mut sum = [0.0f32; 32];
                    for (_, dense) in &neighbors {
                        for i in 0..32 {
                            sum[i] += dense[i];
                        }
                    }
                    let inv_n = 1.0 / neighbors.len() as f32;
                    for v in sum.iter_mut() {
                        *v *= inv_n;
                    }

                    let current = Self::expand_sparse_to_dense_32(
                        vertex.material_ids,
                        vertex.material_weights,
                    );
                    let blended = Self::lerp_dense_32(&current, &sum, strength);
                    let (ids, ws) = Self::top4_from_dense_32(blended);
                    vertex.material_ids = ids;
                    vertex.material_weights = ws;
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

    /// Expand sparse `(material_ids, material_weights)` top-4 representation
    /// to a dense 32-channel weight array. Channel index `i` accumulates the
    /// weight of any slot whose `material_id == i`. Out-of-range IDs
    /// (≥ 32 or negative) are dropped. Real-Fix.E 2026-05-08.
    fn expand_sparse_to_dense_32(ids: [f32; 4], weights: [f32; 4]) -> [f32; 32] {
        let mut dense = [0.0f32; 32];
        for i in 0..4 {
            let id = ids[i];
            if !id.is_finite() {
                continue;
            }
            let id_int = id.round() as i32;
            if (0..32).contains(&id_int) {
                let w = weights[i];
                if w.is_finite() && w > 0.0 {
                    dense[id_int as usize] += w;
                }
            }
        }
        dense
    }

    /// Re-sparsify a dense 32-channel weight array back to top-4 sparse
    /// `(material_ids, material_weights)`. Normalizes weights so the
    /// returned slots sum to 1.0; falls back to layer-0 with weight 1.0 if
    /// the dense array is all-zero. Mirrors
    /// `biome_weights_8_to_material_slots` semantics extended to 32
    /// channels. Real-Fix.E 2026-05-08.
    fn top4_from_dense_32(dense: [f32; 32]) -> ([f32; 4], [f32; 4]) {
        // Collect all (channel_as_layer_id, weight) pairs.
        let mut entries: [(f32, f32); 32] = [(0.0, 0.0); 32];
        for (i, w) in dense.iter().enumerate() {
            entries[i] = (i as f32, *w);
        }
        // Sort by weight descending to find top 4.
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut top4: [(f32, f32); 4] = [entries[0], entries[1], entries[2], entries[3]];
        // Sort top 4 by id ascending for consistent slot assignment across
        // adjacent vertices (prevents interpolation artifacts).
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
            // Fallback: all-zero dense → layer 0 with full weight.
            ids[0] = 0.0;
            ws[0] = 1.0;
        }
        (ids, ws)
    }

    /// Distance-weighted average of dense-32 channels across a sample set.
    /// Weight per sample = 1 - (dist / sample_radius), clamped to [0, 1].
    /// Real-Fix.E 2026-05-08.
    fn weighted_average_dense_32(
        samples: &[([f32; 2], [f32; 32])],
        center_x: f32,
        center_z: f32,
        sample_radius: f32,
    ) -> [f32; 32] {
        let mut sum = [0.0f32; 32];
        let mut total_weight = 0.0f32;
        for (pos, dense) in samples {
            let ddx = pos[0] - center_x;
            let ddz = pos[1] - center_z;
            let dist = (ddx * ddx + ddz * ddz).sqrt();
            let w = (1.0 - dist / sample_radius).max(0.0);
            if w <= 0.0 {
                continue;
            }
            for i in 0..32 {
                sum[i] += dense[i] * w;
            }
            total_weight += w;
        }
        if total_weight > 0.0001 {
            for v in sum.iter_mut() {
                *v /= total_weight;
            }
        }
        sum
    }

    /// Linear interpolation between two dense-32 channel arrays.
    /// Result = (1 - t) * a + t * b per channel. Real-Fix.E 2026-05-08.
    fn lerp_dense_32(a: &[f32; 32], b: &[f32; 32], t: f32) -> [f32; 32] {
        let t = t.clamp(0.0, 1.0);
        let mut out = [0.0f32; 32];
        let one_minus_t = 1.0 - t;
        for i in 0..32 {
            out[i] = a[i] * one_minus_t + b[i] * t;
        }
        out
    }

    /// Maximum per-channel variance across a neighborhood of dense-32 arrays.
    /// Used by the passive pass to detect biome boundaries: a vertex whose
    /// neighborhood has high per-channel variance is on a transition; a
    /// vertex whose neighborhood is uniform is in a biome interior.
    /// Real-Fix.E 2026-05-08.
    fn dense_32_max_channel_variance(samples: &[&[f32; 32]]) -> f32 {
        let n = samples.len() as f32;
        if n < 2.0 {
            return 0.0;
        }
        let mut max_variance = 0.0f32;
        for channel in 0..32 {
            let mut mean = 0.0f32;
            for s in samples {
                mean += s[channel];
            }
            mean /= n;
            let mut sum_sq = 0.0f32;
            for s in samples {
                let d = s[channel] - mean;
                sum_sq += d * d;
            }
            let variance = sum_sq / n;
            if variance > max_variance {
                max_variance = variance;
            }
        }
        max_variance
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
        // Per-type world-scale multiplier: Nature Kit models are authored at
        // ~real-world scale (e.g. `tree_small_02_a.glb` is 3.69 Blender
        // units = 3.69 m per native Blender convention). Multiplier brings
        // effective rendered height into realistic ranges at the 1 WU = 1 m
        // convention documented in `docs/supplemental/WORLD_SCALE_CONVENTIONS.md`.
        //
        // Phase 1.6-F.4.B.2.E: tree multiplier 14 → 4. Previously 14× was a
        // hack compensating for small terrain scale pre-F.4.B.2 — with
        // Target B's ~500 WU peaks and 1 WU = 1 m, trees now need to be
        // realistic forest scale (15-25 m mature) rather than oversized to
        // match mountains. At 4× × scatter-jitter (0.8-1.4), rendered tree
        // height is ~11.8-20.6 m, with peak-to-tree ratio ~25-30× against
        // ~500 m mountain peaks.
        let (type_multiplier, base_tint) = match vi.vegetation_type.as_str() {
            // Trees — raw asset ~3.7 units; ×4 → ~12-21 m rendered.
            s if s.contains("tree") || s.contains("pine") => (4.0, [0.90, 1.00, 0.85, 1.0]),
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

    // Phase 1.6-F.4.B.3.D.3c: `phase_1_6_f2_apply_preset_sets_noise_type_and_continental`
    // RETIRED. Tested the legacy `apply_biome_noise_preset` method which D.3c
    // removes. The replacement architecture (per-vertex BiomeId + per-biome
    // BiomeParameters) doesn't have a single "apply this preset to the whole
    // world" operation, so the test has no clean architecture-equivalent.
    // Per the D.3 plan §1.5: "Some preset-specific tests may not have a
    // clean architecture-equivalent. Document and remove."

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

    // Phase 1.6-F.4.B.3.D.3c: `test_mountain_generation_full_flow` and
    // `test_all_biomes_generate_terrain` RETIRED. Both tests exercised the
    // legacy `apply_biome_noise_preset` path which D.3c removes. The new
    // architecture (climate-field per-vertex BiomeId + per-biome
    // BiomeParameters) is exercised by:
    //  - `phase_1_6_f4_b_3_d_3_diagnostic.rs` integration tests (mixed-climate
    //    chunks produce varied biome IDs, per-vertex amplitude varies).
    //  - Existing `WorldGenerator::generate_chunk_with_climate` tests.
    // Per D.3 plan §1.5: "Some preset-specific tests may not have a clean
    // architecture-equivalent. Document and remove."

    // ====================================================================
    // Real-Fix.E 2026-05-08: ZoneBlend brush helpers (pure CPU; no GPU /
    // chunk machinery needed). Exercises the sparse↔dense round-trip,
    // weighted averaging, and variance-based boundary detection that the
    // active brush + passive global pass both rely on.
    // ====================================================================

    #[test]
    fn zoneblend_expand_then_top4_round_trip_preserves_dominant_layer() {
        // Sparse representation: layer 5 with weight 1.0.
        let (ids, ws) = TerrainState::top4_from_dense_32(
            TerrainState::expand_sparse_to_dense_32(
                [5.0, 0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0, 0.0],
            ),
        );
        // After re-sparsify: layer 5 should still be the dominant slot.
        let dominant_slot = ws
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        assert!(ids[dominant_slot].round() as i32 == 5, "dominant layer lost");
        assert!((ws[dominant_slot] - 1.0).abs() < 1e-4, "weight not preserved");
    }

    #[test]
    fn zoneblend_top4_renormalizes_sum_to_one() {
        // Mix of layers; top4_from_dense_32 should normalize to sum=1.0.
        let mut dense = [0.0f32; 32];
        dense[0] = 0.3;
        dense[5] = 0.5;
        dense[10] = 0.1;
        dense[21] = 0.1;
        let (_, ws) = TerrainState::top4_from_dense_32(dense);
        let sum: f32 = ws.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4, "sum != 1.0: {sum}");
    }

    #[test]
    fn zoneblend_top4_falls_back_to_layer_zero_on_all_zero() {
        let dense = [0.0f32; 32];
        let (ids, ws) = TerrainState::top4_from_dense_32(dense);
        // Fallback: layer 0 with full weight.
        assert_eq!(ids[0], 0.0);
        assert!((ws[0] - 1.0).abs() < 1e-4);
    }

    #[test]
    fn zoneblend_expand_drops_out_of_range_ids() {
        // ID 32 is out of range (Real-Fix.D MAX_TERRAIN_LAYERS=32 cap).
        let dense = TerrainState::expand_sparse_to_dense_32(
            [0.0, 32.0, 50.0, -1.0],
            [0.5, 1.0, 1.0, 1.0],
        );
        // Only layer 0 should have contribution.
        assert!((dense[0] - 0.5).abs() < 1e-6);
        for v in dense.iter().skip(1) {
            assert_eq!(*v, 0.0);
        }
    }

    #[test]
    fn zoneblend_expand_drops_nan_and_infinity() {
        let dense = TerrainState::expand_sparse_to_dense_32(
            [0.0, 1.0, 2.0, 3.0],
            [1.0, f32::NAN, f32::INFINITY, -0.5],
        );
        assert_eq!(dense[0], 1.0);
        assert_eq!(dense[1], 0.0); // NaN dropped
        assert_eq!(dense[2], 0.0); // +inf dropped
        assert_eq!(dense[3], 0.0); // negative dropped
    }

    #[test]
    fn zoneblend_lerp_interpolates_per_channel() {
        let mut a = [0.0f32; 32];
        a[0] = 1.0;
        let mut b = [0.0f32; 32];
        b[5] = 1.0;
        let mid = TerrainState::lerp_dense_32(&a, &b, 0.5);
        assert!((mid[0] - 0.5).abs() < 1e-6);
        assert!((mid[5] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn zoneblend_lerp_clamps_t_to_unit_range() {
        let a = [1.0f32; 32];
        let b = [0.0f32; 32];
        // t > 1.0 should clamp to 1.0 → returns b unchanged.
        let out = TerrainState::lerp_dense_32(&a, &b, 5.0);
        for v in out.iter() {
            assert!((*v - 0.0).abs() < 1e-6, "t>1 should clamp; got {v}");
        }
        // t < 0.0 should clamp to 0.0 → returns a unchanged.
        let out2 = TerrainState::lerp_dense_32(&a, &b, -2.0);
        for v in out2.iter() {
            assert!((*v - 1.0).abs() < 1e-6, "t<0 should clamp; got {v}");
        }
    }

    #[test]
    fn zoneblend_weighted_average_respects_distance() {
        // Two samples: one nearby (layer 0), one far (layer 5). The
        // weighted average at the center should be dominated by the
        // near sample.
        let mut near = [0.0f32; 32];
        near[0] = 1.0;
        let mut far = [0.0f32; 32];
        far[5] = 1.0;
        let samples = vec![([0.0, 0.0], near), ([9.0, 0.0], far)];
        let avg = TerrainState::weighted_average_dense_32(&samples, 0.0, 0.0, 10.0);
        // Near sample has weight ~1.0 (dist=0); far sample has weight ~0.1
        // (dist=9, sample_radius=10). After normalization: near ~0.91,
        // far ~0.09.
        assert!(avg[0] > avg[5], "near sample should dominate");
        assert!(avg[0] > 0.5, "near contribution too small: {}", avg[0]);
    }

    #[test]
    fn zoneblend_variance_low_for_uniform_neighborhood() {
        let mut sample = [0.0f32; 32];
        sample[3] = 1.0;
        let samples = vec![&sample, &sample, &sample];
        let var = TerrainState::dense_32_max_channel_variance(&samples);
        assert!(var < 1e-6, "uniform neighborhood should have zero variance");
    }

    #[test]
    fn zoneblend_variance_high_at_biome_boundary() {
        // Two distinct biomes mixed in a neighborhood.
        let mut a = [0.0f32; 32];
        a[0] = 1.0; // grass
        let mut b = [0.0f32; 32];
        b[3] = 1.0; // mountain_rock
        let samples = vec![&a, &b, &a, &b];
        let var = TerrainState::dense_32_max_channel_variance(&samples);
        assert!(var > 0.1, "biome boundary should have high variance: got {var}");
    }

    #[test]
    fn zoneblend_blend_of_two_layers_produces_intermediate_slots() {
        // Two pure-biome vertices: layer 0 and layer 5. Average their
        // dense representations, re-sparsify. Result should occupy both
        // slot 0 and slot 5 with equal-ish weights.
        let a = TerrainState::expand_sparse_to_dense_32(
            [0.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
        );
        let b = TerrainState::expand_sparse_to_dense_32(
            [5.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
        );
        let blended = TerrainState::lerp_dense_32(&a, &b, 0.5);
        let (ids, ws) = TerrainState::top4_from_dense_32(blended);
        // After re-sparsify, layers 0 and 5 should both have weight ~0.5.
        let mut has_layer_0 = false;
        let mut has_layer_5 = false;
        for i in 0..4 {
            if ids[i] == 0.0 && ws[i] > 0.4 {
                has_layer_0 = true;
            }
            if ids[i] == 5.0 && ws[i] > 0.4 {
                has_layer_5 = true;
            }
        }
        assert!(has_layer_0, "missing blended layer 0: ids={ids:?}, ws={ws:?}");
        assert!(has_layer_5, "missing blended layer 5: ids={ids:?}, ws={ws:?}");
    }

    #[test]
    fn zoneblend_round_trip_preserves_normalization() {
        // Start with normalized sparse representation; round-trip
        // through dense + re-sparsify; verify still normalized.
        let dense = TerrainState::expand_sparse_to_dense_32(
            [0.0, 5.0, 10.0, 21.0],
            [0.4, 0.3, 0.2, 0.1],
        );
        let (_, ws) = TerrainState::top4_from_dense_32(dense);
        let sum: f32 = ws.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4, "round-trip lost normalization: sum={sum}");
    }

    // ====================================================================
    // Cleanup-D fix-pass tests (M-D5 + M-D9 + M-SK)
    //
    // Predecessor: Cleanup-D research-pass audit at
    // `docs/audits/editor_multi_tool_architecture_subphase_3_mediator_
    // brush_cleanup_d_research_pass_2026-05-08.md` identified three
    // chunk-boundary state-propagation pathway equivalence failures:
    //   M-D5: per-chunk avg_height → erosion seam at chunk boundary
    //   M-D9: missing post-brush stitch_edge_normals → lighting seam
    //   M-SK: skirt vertex heights frozen after brush → skirt decouple
    //
    // These tests verify the structural fix: compute_skirt_indices
    // mapping (M-SK; pure function), plus stitch_edge_normals re-entrant
    // safety + idempotence (M-D9 supporting). M-D5 + M-D9 visual
    // verdicts are produced via Andrew-gate runtime observation per §7.
    // ====================================================================

    #[test]
    fn cleanup_d_skirt_indices_interior_vertex_returns_none() {
        // (5, 5) in a 10×10 mesh is interior: neither edge of x nor z.
        let out = TerrainState::compute_skirt_indices(5, 5, 10);
        assert_eq!(out, [None, None], "interior vertex must not map to any skirt");
    }

    #[test]
    fn cleanup_d_skirt_indices_bottom_edge_maps_single() {
        // (5, 0) → bottom skirt only. surface = 100; bottom base = 100.
        let out = TerrainState::compute_skirt_indices(5, 0, 10);
        assert_eq!(out[0], Some(100 + 5));
        assert_eq!(out[1], None);
    }

    #[test]
    fn cleanup_d_skirt_indices_top_edge_maps_single() {
        // (5, 9) → top skirt only. top base = 100 + 10 = 110.
        let out = TerrainState::compute_skirt_indices(5, 9, 10);
        assert_eq!(out[0], Some(110 + 5));
        assert_eq!(out[1], None);
    }

    #[test]
    fn cleanup_d_skirt_indices_left_edge_maps_single() {
        // (0, 5) → left skirt only. left base = 100 + 20 = 120.
        let out = TerrainState::compute_skirt_indices(0, 5, 10);
        assert_eq!(out[0], Some(120 + 5));
        assert_eq!(out[1], None);
    }

    #[test]
    fn cleanup_d_skirt_indices_right_edge_maps_single() {
        // (9, 5) → right skirt only. right base = 100 + 30 = 130.
        let out = TerrainState::compute_skirt_indices(9, 5, 10);
        assert_eq!(out[0], Some(130 + 5));
        assert_eq!(out[1], None);
    }

    #[test]
    fn cleanup_d_skirt_indices_bottom_left_corner_maps_two() {
        // (0, 0) is in BOTH bottom AND left skirts.
        let out = TerrainState::compute_skirt_indices(0, 0, 10);
        // Bottom first (gz check evaluated first), then left.
        assert_eq!(out[0], Some(100 + 0), "bottom skirt for (0,0) slot 0");
        assert_eq!(out[1], Some(120 + 0), "left skirt for (0,0) slot 1");
    }

    #[test]
    fn cleanup_d_skirt_indices_bottom_right_corner_maps_two() {
        // (9, 0) is in BOTH bottom AND right skirts.
        let out = TerrainState::compute_skirt_indices(9, 0, 10);
        assert_eq!(out[0], Some(100 + 9), "bottom skirt for (9,0) slot 0");
        assert_eq!(out[1], Some(130 + 0), "right skirt for (9,0) slot 1");
    }

    #[test]
    fn cleanup_d_skirt_indices_top_left_corner_maps_two() {
        // (0, 9) is in BOTH top AND left skirts.
        let out = TerrainState::compute_skirt_indices(0, 9, 10);
        assert_eq!(out[0], Some(110 + 0), "top skirt for (0,9) slot 0");
        assert_eq!(out[1], Some(120 + 9), "left skirt for (0,9) slot 1");
    }

    #[test]
    fn cleanup_d_skirt_indices_top_right_corner_maps_two() {
        // (9, 9) is in BOTH top AND right skirts.
        let out = TerrainState::compute_skirt_indices(9, 9, 10);
        assert_eq!(out[0], Some(110 + 9), "top skirt for (9,9) slot 0");
        assert_eq!(out[1], Some(130 + 9), "right skirt for (9,9) slot 1");
    }

    #[test]
    fn cleanup_d_skirt_indices_all_in_bounds_for_resolution_10() {
        // Every mapped skirt index for a 10×10 mesh must be < total
        // vertex count (surface 100 + 4 skirts × 10 = 140).
        let total = 10 * 10 + 4 * 10;
        for gz in 0..10u32 {
            for gx in 0..10u32 {
                let out = TerrainState::compute_skirt_indices(gx, gz, 10);
                for maybe in out.iter() {
                    if let Some(idx) = *maybe {
                        assert!(
                            idx < total,
                            "compute_skirt_indices({gx},{gz},10) -> {idx} >= total {total}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn cleanup_d_skirt_indices_corners_produce_distinct_indices() {
        // Each corner's two mapped indices must be distinct (corner is
        // in two different skirts, never the same one twice).
        let corners = [(0, 0), (9, 0), (0, 9), (9, 9)];
        for (gx, gz) in corners.iter() {
            let out = TerrainState::compute_skirt_indices(*gx, *gz, 10);
            let a = out[0].expect("corner must have skirt 1");
            let b = out[1].expect("corner must have skirt 2");
            assert_ne!(a, b, "corner ({gx},{gz}) maps to same skirt twice");
        }
    }
}
