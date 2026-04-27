#![forbid(unsafe_code)]
//! AstraWeave Terrain Generation Module
//!
//! This module provides procedural terrain generation using noise functions,
//! heightmaps, and biome classification for the AstraWeave engine.

pub mod advanced_erosion; // Production-ready erosion simulation
pub mod background_loader; // Week 4 Action 14: Async chunk streaming
pub mod biome;
pub mod biome_blending; // Production-ready biome blending
pub mod biome_pack;
pub mod blueprint_zone;
pub mod chunk;
pub mod climate;
pub mod elevation_biome; // Phase 1.5: heightmap-driven multi-biome generation
pub mod collision;
pub mod compressed_voxels; // P4-8: Palette compression + RLE for voxel chunks
pub mod erosion;
pub mod gpu_bridge; // GPU acceleration bridge (TerrainGpuAccelerator trait)
pub mod heightmap;
pub mod lod_blending;
pub mod lod_manager; // Week 4 Action 14: LOD with hysteresis
pub mod marching_cubes_tables;
pub mod meshing;
pub mod noise_gen;
pub mod noise_simd; // SIMD-optimized noise generation (Week 3 Action 8)
pub mod perlin_gradient; // Phase 1.6-F.2-T-4: analytical-derivative Perlin + derivative-weighted fBm
pub mod runevision_erosion; // Phase 1.6-F.4.B.3.C: gradient-aligned gully extrusion filter (Skovbo Johansen)
pub mod biome_lookup; // Phase 1.6-F.4.B.3.D.2: Whittaker biome lookup (climate × elevation → BiomeId)
pub mod biome_parameters; // Phase 1.6-F.4.B.3.D.3: per-BiomeId terrain parameters (replaces BiomeNoisePreset)
pub mod partition_integration;
pub mod scatter;
pub mod solver; // Phase 10: AI-Orchestrated Dynamic Terrain
pub mod streaming_diagnostics; // Week 4 Action 14: Diagnostics overlay
pub mod structures;
pub mod terrain_modifier; // Phase 10: Batched voxel updates
pub mod terrain_persistence; // Phase 10: Terrain save/load
pub mod texture_splatting; // Production-ready terrain texture splatting
pub mod voxel_data;
pub mod zone_scatter;

pub use advanced_erosion::{
    erosion_preset_for_climate, AdvancedErosionSimulator, ErosionPreset, ErosionStats,
    HydraulicErosionConfig, ThermalErosionConfig, WindErosionConfig,
}; // Advanced erosion
pub use background_loader::{BackgroundChunkLoader, StreamingConfig, StreamingStats}; // Week 4
pub use biome::{Biome, BiomeConfig, BiomeType, BiomeVegetation, VegetationType};
pub use biome_blending::{BiomeBlendConfig, BiomeBlender, BiomeWeight, PackedBiomeBlend}; // Biome blending
pub use biome_pack::{BiomePack, BiomePackAsset, BiomePackScatter};
pub use blueprint_zone::{
    AdaptiveScaleParams, BlendMask, BlueprintZone, PlacementMode, ZoneId, ZoneRegistry, ZoneSource,
};
pub use chunk::{smooth_shared_vertices, ChunkId, ChunkManager, TerrainChunk};
pub use collision::{collision_mesh_from_chunk, collision_mesh_from_heightmap, CollisionMesh};
pub use climate::{ClimateConfig, ClimateMap};
pub use elevation_biome::{elevation_to_biome_weights, ClimateBias, SEA_LEVEL}; // Phase 1.5
pub use gpu_bridge::{
    GpuErosionRequest, GpuHeightmapRequest, GpuHeightmapResult, GpuNoiseRequest,
    TerrainGpuAccelerator,
};
pub use compressed_voxels::{
    CompressedVoxelChunk, PaletteEntry, RleRun, VoxelPalette, CHUNK_VOLUME,
};
pub use heightmap::{Heightmap, HeightmapConfig};
pub use lod_blending::{LodBlender, MorphConfig, MorphedMesh, MorphingLodManager};
pub use lod_manager::{
    compute_pixel_error, ChunkLodState, LodConfig as LodHysteresisConfig, LodLevel, LodManager,
    LodStats, ViewParams,
}; // Week 4
pub use meshing::{
    AsyncMeshGenerator, ChunkMesh, DualContouring, LodConfig, LodMeshGenerator, MeshVertex,
};
pub use noise_gen::{domain_warped_fbm, DomainWarpConfig, NoiseConfig, NoiseType, TerrainNoise};
pub use noise_simd::SimdHeightmapGenerator; // Week 3 Action 8: SIMD optimization
pub use partition_integration::{
    PartitionCoord, VoxelPartitionConfig, VoxelPartitionEvent, VoxelPartitionManager,
    VoxelPartitionStats,
};
pub use scatter::{
    ScatterConfig, ScatterResult, VegetationInstance, VegetationLodConfig, VegetationScatter,
    density_at_distance,
};
pub use solver::{ResolvedLocation, SolverError, TerrainSolver, ValidationStatus};
pub use streaming_diagnostics::{
    ChunkLoadState, DiagnosticReport, FrameStats, HitchDetector, MemoryStats, StreamingDiagnostics,
}; // Week 4
pub use structures::{
    StructureConfig, StructureGenerator, StructureInstance, StructureResult, StructureType,
};
pub use terrain_modifier::{
    ModifierStats, NavMeshRegion, TerrainModifier, TerrainModifierConfig, VoxelOp, VoxelOpType,
}; // Phase 10
pub use texture_splatting::{
    SplatConfig, SplatMapGenerator, SplatRule, SplatWeights, TerrainMaterial, TerrainSplatVertex,
    TriplanarWeights, MAX_SPLAT_LAYERS,
}; // Texture splatting
pub use voxel_data::{ChunkCoord, Density, MaterialId, Voxel, VoxelChunk, VoxelGrid, CHUNK_SIZE};
pub use zone_scatter::{
    apply_heightmap_patches, generate_multi_zone_scatter, HeightmapPatch, SourceHeightmap,
    ZoneGenerationResult, ZoneScatterGenerator,
};

use glam::Vec3;
use serde::{Deserialize, Serialize};

// Phase 5: Comprehensive test modules
#[cfg(test)]
mod chunk_tests;
#[cfg(test)]
mod mutation_tests;
#[cfg(test)]
mod voxel_data_tests; // Phase 10B: Comprehensive mutation-killing tests

/// Configuration for the world generator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConfig {
    /// Random seed for generation
    pub seed: u64,
    /// Size of terrain chunks in world units
    pub chunk_size: f32,
    /// Resolution of heightmaps (vertices per chunk edge)
    pub heightmap_resolution: u32,
    /// Noise configuration for terrain generation
    pub noise: NoiseConfig,
    /// Climate configuration for biome assignment
    pub climate: ClimateConfig,
    /// Available biome configurations
    pub biomes: Vec<BiomeConfig>,
    /// Structure generation configuration
    pub structures: structures::StructureConfig,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            seed: 12345,
            // Phase 1.6-F.4.B.2.A: Target B scale (Enshrouded-class).
            // Chunk extent 256 → 512 WU (1 WU = 1 m convention per
            // `docs/supplemental/WORLD_SCALE_CONVENTIONS.md`); vertex density
            // 64 → 96 per side → vertex spacing 5.39 m (was 4.06 m). Halo=1
            // region becomes 288 verts covering 1536 m. F.4.B.2.C raises
            // radius default 5 → 10 in the editor UI (115 km² at radius 10).
            chunk_size: 512.0,
            heightmap_resolution: 96,
            noise: NoiseConfig::default(),
            climate: ClimateConfig::default(),
            biomes: vec![
                BiomeConfig::grassland(),
                BiomeConfig::desert(),
                BiomeConfig::forest(),
                BiomeConfig::mountain(),
            ],
            structures: structures::StructureConfig::default(),
        }
    }
}

/// Main world generator that coordinates terrain, climate, and biome generation
#[derive(Debug)]
pub struct WorldGenerator {
    config: WorldConfig,
    noise: TerrainNoise,
    climate: ClimateMap,
    chunk_manager: ChunkManager,
    structure_generator: structures::StructureGenerator,
}

impl WorldGenerator {
    /// Create a new world generator with the given configuration
    pub fn new(config: WorldConfig) -> Self {
        let noise = TerrainNoise::new(&config.noise, config.seed);
        let climate = ClimateMap::new(&config.climate, config.seed + 1);
        let chunk_manager = ChunkManager::new(config.chunk_size, config.heightmap_resolution);
        let mut structure_config = config.structures.clone();
        structure_config.seed = config.seed + 2; // Offset seed for structures
        let structure_generator = structures::StructureGenerator::new(structure_config);

        Self {
            config,
            noise,
            climate,
            chunk_manager,
            structure_generator,
        }
    }

    /// Generate a terrain chunk at the given world position with vegetation and resources
    pub fn generate_chunk_with_scatter(
        &mut self,
        chunk_id: ChunkId,
    ) -> anyhow::Result<(TerrainChunk, ScatterResult)> {
        // Generate the basic terrain chunk (lock-free)
        let chunk = self.generate_chunk(chunk_id)?;

        // Register with chunk manager
        self.chunk_manager.add_chunk(chunk.clone());

        // Generate scatter for the chunk
        let scatter_result = self.scatter_chunk_content(&chunk)?;

        Ok((chunk, scatter_result))
    }

    /// Generate scatter content (vegetation and resources) for an existing chunk
    pub fn scatter_chunk_content(&mut self, chunk: &TerrainChunk) -> anyhow::Result<ScatterResult> {
        let mut result = ScatterResult::new(chunk.id());

        // Create scatter system
        let scatter_config = ScatterConfig::default();
        let scatter = VegetationScatter::new(scatter_config);

        // Sample the biome at the chunk center to determine configuration
        let chunk_center = chunk.id().to_center_pos(self.config.chunk_size);
        let center_biome = chunk
            .get_biome_at_world_pos(chunk_center, self.config.chunk_size)
            .unwrap_or(BiomeType::Grassland);

        // Find the biome configuration
        let biome_config = self
            .config
            .biomes
            .iter()
            .find(|b| b.biome_type == center_biome)
            .unwrap_or(&self.config.biomes[0]);

        // Generate vegetation
        result.vegetation = scatter.scatter_vegetation(
            chunk,
            self.config.chunk_size,
            biome_config,
            self.config.seed + chunk.id().x as u64 * 1000 + chunk.id().z as u64,
        )?;

        // Generate resources
        result.resources = scatter.scatter_resources(
            chunk,
            self.config.chunk_size,
            biome_config,
            self.config.seed + chunk.id().x as u64 * 2000 + chunk.id().z as u64,
        )?;

        // Generate structures
        let structure_result = self.structure_generator.generate_structures(
            chunk,
            self.config.chunk_size,
            center_biome,
        )?;
        result.structures = structure_result.structures;

        Ok(result)
    }

    /// Generate a terrain chunk at the given position (lock-free, parallel-safe)
    /// NOTE: Does NOT add to chunk_manager - caller must handle that separately
    pub fn generate_chunk(&self, chunk_id: ChunkId) -> anyhow::Result<TerrainChunk> {
        // Generate heightmap for this chunk (using SIMD if enabled)
        #[cfg(feature = "simd-noise")]
        let heightmap = noise_simd::SimdHeightmapGenerator::generate_heightmap_simd(
            &self.noise,
            chunk_id,
            self.config.chunk_size,
            self.config.heightmap_resolution,
        )?;

        #[cfg(not(feature = "simd-noise"))]
        let heightmap = self.noise.generate_heightmap(
            chunk_id,
            self.config.chunk_size,
            self.config.heightmap_resolution,
        )?;

        // Generate climate data for biome assignment
        let climate_data = self.climate.sample_chunk(
            chunk_id,
            self.config.chunk_size,
            self.config.heightmap_resolution,
        )?;

        // Assign biomes based on height and climate
        let biome_map = self.assign_biomes(&heightmap, &climate_data)?;

        // Create the terrain chunk
        let mut chunk = TerrainChunk::new(chunk_id, heightmap, biome_map);

        // Apply erosion if enabled
        if self.config.noise.erosion_enabled {
            chunk.apply_erosion(self.config.noise.erosion_strength)?;
        }

        // NOTE: chunk_manager.add_chunk() removed - caller handles registration
        Ok(chunk)
    }

    /// Phase 1.6-F.3-phase-1: generate a chunk with pre-erosion biome_weights
    /// computed from the given climate bias. The returned chunk has
    /// `biome_weights: Some(_)` populated from the PRE-erosion heightmap,
    /// satisfying §2.5's authorial-intent invariant. The heightmap itself is
    /// post-erosion (simple CA for phase 1; `AdvancedErosionSimulator` for
    /// phase 2).
    ///
    /// Callers not requiring pre-erosion biome_weights should use the simpler
    /// `generate_chunk` (biome_weights stays `None`, editor/consumer computes
    /// them on-the-fly from post-erosion heights — current behavior).
    pub fn generate_chunk_with_climate(
        &self,
        chunk_id: ChunkId,
        climate_bias: crate::ClimateBias,
    ) -> anyhow::Result<TerrainChunk> {
        // Phase 1.6-F.3-phase-2.C: full pipeline wired.
        //
        // 1. Generate halo heightmap (3×3 chunks centered on target).
        // 2. Crop the halo to the target chunk for PRE-erosion biome_weights
        //    computation per §2.5 (authorial-intent invariant).
        // 3. Run AdvancedErosionSimulator::apply_preset on the FULL halo —
        //    droplets travel freely across chunk boundaries within the halo
        //    region. §2.3 halo=1 is sufficient per phase-0's p95 = 120 world
        //    units < 256 (one chunk width).
        // 4. Crop AFTER erosion. This becomes the chunk's final (post-erosion)
        //    heightmap.
        // 5. Construct chunk with post-erosion cropped heightmap + pre-erosion
        //    biome_weights (decoupled per §2.5).
        //
        // Legacy `chunk.apply_erosion(strength)` simple CA call is replaced by
        // `AdvancedErosionSimulator` per §2.2 climate → preset mapping.
        const HALO_CHUNKS: u32 = 1;

        let mut halo = self.generate_halo_heightmap(chunk_id, HALO_CHUNKS)?;

        // Phase 1.6-F.4.B.3.D.3b: per-vertex biome lookup + per-biome
        // mountain-amplitude modulation. Iterates the freshly generated halo
        // (each vertex currently has the default-amplitude `sample_height`
        // value), samples the climate field at that vertex's raw elevation,
        // looks up the dominant `BiomeId` via D.2's `lookup_biome`, looks up
        // per-biome `BiomeParameters` from D.3a, and re-samples the height
        // with `sample_height_with_mountain_amplitude` using
        // `params.mountains_amplitude` as the multiplier.
        //
        // This is the structural replacement for the legacy
        // `BiomeNoisePreset` whole-world configuration: every vertex gets
        // its own biome assignment from the climate field, and the noise
        // pipeline applies that biome's amplitude per-vertex.
        //
        // The pre-erosion biome IDs are computed from raw (default-amplitude)
        // heights so that biome assignment is invariant across re-runs with
        // different per-biome amplitudes — biome assignment shapes the world,
        // not the other way around. Per §2.5, biome assignment uses
        // pre-erosion heights for authorial-intent stability.
        let halo_biome_ids = self
            .apply_per_biome_modulation_to_halo(&mut halo, chunk_id, HALO_CHUNKS);

        // Pre-erosion cropped heightmap — input to biome_weights computation.
        // After per-biome modulation, this reflects per-vertex amplitude.
        let pre_erosion_heightmap = self.crop_halo_to_chunk(&halo, chunk_id)?;

        // §2.5: biome_weights are computed from PRE-erosion heights. Once
        // phase 2 lands real erosion, a vertex that drops from Y=50 to Y=30
        // keeps its pre-erosion Mountain weighting — authorial intent over
        // geological reclassification.
        let resolution = pre_erosion_heightmap.resolution() as usize;
        let mut biome_weights = Vec::with_capacity(resolution * resolution);
        for z in 0..resolution {
            for x in 0..resolution {
                let y = pre_erosion_heightmap.get_height(x as u32, z as u32);
                biome_weights.push(crate::elevation_to_biome_weights(
                    y,
                    crate::SEA_LEVEL,
                    climate_bias,
                ));
            }
        }

        // D.3b: crop the per-vertex BiomeId array to the chunk-sized region
        // matching `pre_erosion_heightmap`. Uses the same crop offsets as
        // `crop_halo_to_chunk`.
        let chunk_biome_ids = self.crop_halo_biome_ids_to_chunk(
            &halo_biome_ids,
            halo.resolution() as usize,
            chunk_id,
        );

        // §2.2 climate → ErosionPreset mapping.
        let preset = crate::advanced_erosion::erosion_preset_for_climate(climate_bias);

        // Phase 1.6-F.3-phase-3.C: use `apply_preset_at_world_offset` for
        // seamless chunk boundaries. Droplet spawn positions are derived
        // from world-aligned spatial cells, so adjacent halos iterate the
        // SAME cells in overlap → identical droplets → near-identical
        // erosion output in overlap region.
        //
        // The simulator's `new(seed)` argument is no longer the primary
        // determinism driver — per-droplet RNG comes from world-cell hash
        // inside the new API. We pass `halo_seed` for any ancillary RNG use
        // (currently unused inside the world-coord path).
        let seed = Self::halo_seed(self.config.seed, chunk_id, HALO_CHUNKS);
        let mut simulator = crate::advanced_erosion::AdvancedErosionSimulator::new(seed);

        if self.config.noise.erosion_enabled {
            // Halo's world origin (target chunk origin minus halo_chunks * chunk_size).
            let target_origin = chunk_id.to_world_pos(self.config.chunk_size);
            let halo_origin_x = (target_origin.x - HALO_CHUNKS as f32 * self.config.chunk_size) as f64;
            let halo_origin_z = (target_origin.z - HALO_CHUNKS as f32 * self.config.chunk_size) as f64;
            // Vertex spacing: chunk_size per (resolution - 1) vertices.
            let vertex_spacing =
                self.config.chunk_size as f64 / (self.config.heightmap_resolution - 1) as f64;

            let _stats = simulator.apply_preset_at_world_offset(
                &mut halo,
                &preset,
                halo_origin_x,
                halo_origin_z,
                vertex_spacing,
                self.config.seed,
            );
            halo.recalculate_bounds();
        }

        // Post-erosion cropped heightmap — final chunk output.
        let heightmap = self.crop_halo_to_chunk(&halo, chunk_id)?;

        // biome_map still computed from PRE-erosion heights + climate data
        // (unchanged path). Phase 4 will route climate through biome_weights;
        // biome_map currently feeds splat-rule selection elsewhere.
        let climate_data = self.climate.sample_chunk(
            chunk_id,
            self.config.chunk_size,
            self.config.heightmap_resolution,
        )?;
        let biome_map = self.assign_biomes(&pre_erosion_heightmap, &climate_data)?;

        // Construct chunk: post-erosion heights + pre-erosion biome_weights
        // + per-vertex BiomeId array (Phase 1.6-F.4.B.3.D.3b).
        // No further `apply_erosion` — phase 2 replaces simple CA with
        // AdvancedErosionSimulator.
        let chunk = TerrainChunk::new_with_climate_field(
            chunk_id,
            heightmap,
            biome_map,
            biome_weights,
            chunk_biome_ids,
        );

        Ok(chunk)
    }

    /// Phase 1.6-F.4.B.3.D.3b: apply per-vertex biome modulation to a
    /// halo heightmap.
    ///
    /// For each vertex in the halo, the function:
    /// 1. Samples the climate field at the vertex's current (raw) elevation.
    /// 2. Looks up the dominant `BiomeId` via `lookup_biome`.
    /// 3. Looks up the biome's `BiomeParameters` via `for_biome`.
    /// 4. Re-samples the noise pipeline using
    ///    `TerrainNoise::sample_height_with_mountain_amplitude` with the
    ///    biome's `mountains_amplitude` multiplier.
    /// 5. Replaces the halo's height value with the per-biome-modulated
    ///    sample and records the biome ID.
    ///
    /// Returns a flat `Vec<BiomeId>` in row-major `(z, x)` order matching
    /// the halo's vertex layout. Length: `halo_res × halo_res`.
    ///
    /// Cost analysis: 2 noise samples (one for biome-lookup elevation, one
    /// for per-biome height) + 1 climate sample + 1 biome lookup per vertex.
    /// The 2x noise cost is the per-vertex price of the architectural
    /// correction; erosion remains the dominant cost (60s+ per radius-5
    /// chunk for Temperate climate).
    fn apply_per_biome_modulation_to_halo(
        &self,
        halo: &mut heightmap::Heightmap,
        target_chunk_id: ChunkId,
        halo_chunks: u32,
    ) -> Vec<crate::biome_lookup::BiomeId> {
        let halo_res = halo.resolution() as usize;
        let chunk_res = self.config.heightmap_resolution as usize;
        // CRITICAL: must mirror `generate_halo_heightmap`'s f32 arithmetic
        // so that vertex world coordinates at the shared edge between
        // adjacent chunks match exactly. f64 arithmetic produces a
        // slightly different `step` value than f32, breaking the
        // shared-edge invariant under per-biome modulation. Per
        // F.4.B.3.D.3b regression diagnosis: f64 path produced 125 WU
        // divergence at chunk borders for the mountain test.
        let chunk_size = self.config.chunk_size;
        let step = chunk_size / (chunk_res as f32 - 1.0);

        let target_origin = target_chunk_id.to_world_pos(chunk_size);
        let halo_origin_x = target_origin.x - halo_chunks as f32 * chunk_size;
        let halo_origin_z = target_origin.z - halo_chunks as f32 * chunk_size;

        let mut biome_ids = Vec::with_capacity(halo_res * halo_res);
        for z_idx in 0..halo_res {
            for x_idx in 0..halo_res {
                let wx_f32 = halo_origin_x + x_idx as f32 * step;
                let wz_f32 = halo_origin_z + z_idx as f32 * step;
                let wx = wx_f32 as f64;
                let wz = wz_f32 as f64;

                let raw_height = halo.get_height(x_idx as u32, z_idx as u32);

                // Climate sample at the raw height. Climate field uses
                // archetype-driven temperature/moisture/continentalness;
                // raw height feeds the lapse-rate modulator.
                let climate = self.climate.sample(wx, wz, raw_height);

                // Whittaker biome lookup. Pure function of climate + elevation.
                let biome_id = crate::biome_lookup::lookup_biome(
                    climate.temperature_c,
                    climate.moisture_mm,
                    raw_height,
                );

                // Per-biome parameters. Currently wires only
                // `mountains_amplitude` into the noise pipeline; other fields
                // (ridge_strength, runevision_config, scatter, surface) are
                // forward-compatible defaults consumed by downstream
                // subsystems (D.5+).
                let params = crate::biome_parameters::BiomeParameters::for_biome(biome_id);

                let modulated_height = self.noise.sample_height_with_mountain_amplitude(
                    wx,
                    wz,
                    params.mountains_amplitude as f32,
                );
                halo.set_height(x_idx as u32, z_idx as u32, modulated_height);
                biome_ids.push(biome_id);
            }
        }
        halo.recalculate_bounds();
        biome_ids
    }

    /// Phase 1.6-F.4.B.3.D.3b: crop the per-vertex `BiomeId` halo array to
    /// the chunk-sized center region. Mirrors `crop_halo_to_chunk` for the
    /// heightmap.
    fn crop_halo_biome_ids_to_chunk(
        &self,
        halo_biome_ids: &[crate::biome_lookup::BiomeId],
        halo_res: usize,
        _target_chunk_id: ChunkId,
    ) -> Vec<crate::biome_lookup::BiomeId> {
        let chunk_res = self.config.heightmap_resolution as usize;
        // halo_res = chunks_per_side * (chunk_res - 1) + 1
        // chunks_per_side = (halo_res - 1) / (chunk_res - 1)
        // halo_chunks = (chunks_per_side - 1) / 2
        // crop_offset = halo_chunks * (chunk_res - 1)
        let chunks_per_side = (halo_res - 1) / (chunk_res - 1);
        let halo_chunks = (chunks_per_side - 1) / 2;
        let crop_offset = halo_chunks * (chunk_res - 1);

        let mut chunk_ids = Vec::with_capacity(chunk_res * chunk_res);
        for z in 0..chunk_res {
            for x in 0..chunk_res {
                let halo_idx =
                    (crop_offset + z) * halo_res + (crop_offset + x);
                chunk_ids.push(halo_biome_ids[halo_idx]);
            }
        }
        chunk_ids
    }

    /// Phase 1.6-F.3-phase-1.B: generate a heightmap covering a 3×3 chunk
    /// region centered on `target_chunk_id` (halo_chunks=1). The returned
    /// heightmap has `(1 + 2*halo_chunks) × (heightmap_resolution - 1) + 1`
    /// vertices per side at the same per-vertex spacing as single-chunk
    /// generation, so the center third (sampled at the target chunk's world
    /// coords) is byte-identical to what `generate_chunk` produces via the
    /// SIMD heightmap generator.
    pub(crate) fn generate_halo_heightmap(
        &self,
        target_chunk_id: ChunkId,
        halo_chunks: u32,
    ) -> anyhow::Result<heightmap::Heightmap> {
        let chunk_res = self.config.heightmap_resolution;
        let chunk_size = self.config.chunk_size;
        // Per-vertex step in world units. Must match single-chunk generation.
        let step = chunk_size / (chunk_res - 1) as f32;

        // Halo region: (1 + 2*halo_chunks) sub-chunks per axis. Adjacent
        // sub-chunks share their edge vertex so total vertex count per side is
        // `chunks_per_side * (chunk_res - 1) + 1`.
        let chunks_per_side = 1 + 2 * halo_chunks;
        let halo_res = chunks_per_side * (chunk_res - 1) + 1;
        let halo_size_world = chunks_per_side as f32 * chunk_size;

        // Origin world coordinates = target chunk origin minus (halo_chunks × chunk_size).
        let target_origin = target_chunk_id.to_world_pos(chunk_size);
        let halo_origin_x = target_origin.x - halo_chunks as f32 * chunk_size;
        let halo_origin_z = target_origin.z - halo_chunks as f32 * chunk_size;

        // Sample the noise field directly at per-vertex world coordinates. This
        // path uses `TerrainNoise::sample_height` rather than the SIMD
        // heightmap generator because the SIMD path is tied to a ChunkId. For
        // byte-identity with the SIMD path at the center crop, both routes
        // must produce the same Y at the same world (x, z) — confirmed by the
        // determinism invariant of `TerrainNoise`.
        let heightmap_config = heightmap::HeightmapConfig {
            resolution: halo_res,
            ..Default::default()
        };
        let mut halo_map = heightmap::Heightmap::new(heightmap_config)?;
        for z_idx in 0..halo_res {
            for x_idx in 0..halo_res {
                let wx = halo_origin_x + x_idx as f32 * step;
                let wz = halo_origin_z + z_idx as f32 * step;
                let y = self.noise.sample_height(wx as f64, wz as f64);
                halo_map.set_height(x_idx, z_idx, y);
            }
        }

        // Silence unused warning for halo_size_world — useful for phase 2
        // perf logging and halo-overlap sanity assertions.
        let _ = halo_size_world;

        Ok(halo_map)
    }

    /// Phase 1.6-F.3-phase-1.B: crop the center chunk-sized region out of a
    /// halo-expanded heightmap. Assumes the halo was built by
    /// `generate_halo_heightmap` with the same `target_chunk_id` and halo
    /// size. Returns a single-chunk heightmap (resolution matching
    /// `WorldConfig::heightmap_resolution`).
    pub(crate) fn crop_halo_to_chunk(
        &self,
        halo: &heightmap::Heightmap,
        target_chunk_id: ChunkId,
    ) -> anyhow::Result<heightmap::Heightmap> {
        let _ = target_chunk_id; // centered crop — target id not needed
        let chunk_res = self.config.heightmap_resolution;
        let halo_res = halo.resolution();
        // Infer halo_chunks from resolutions.
        //   halo_res = chunks_per_side * (chunk_res - 1) + 1
        //   chunks_per_side = (halo_res - 1) / (chunk_res - 1)
        //   halo_chunks = (chunks_per_side - 1) / 2
        if chunk_res == 0 || (halo_res - 1) % (chunk_res - 1) != 0 {
            anyhow::bail!(
                "halo resolution {} not a multiple of chunk resolution {}",
                halo_res,
                chunk_res
            );
        }
        let chunks_per_side = (halo_res - 1) / (chunk_res - 1);
        if chunks_per_side < 1 || chunks_per_side % 2 != 1 {
            anyhow::bail!(
                "halo has {} chunks per side; expected odd >= 1",
                chunks_per_side
            );
        }
        let halo_chunks = (chunks_per_side - 1) / 2;
        let start = halo_chunks * (chunk_res - 1);

        let chunk_cfg = heightmap::HeightmapConfig {
            resolution: chunk_res,
            ..Default::default()
        };
        let mut cropped = heightmap::Heightmap::new(chunk_cfg)?;
        for z in 0..chunk_res {
            for x in 0..chunk_res {
                let y = halo.get_height(start + x, start + z);
                cropped.set_height(x, z, y);
            }
        }
        Ok(cropped)
    }

    /// Phase 1.6-F.3-phase-1.B / §2.3: derive a deterministic seed for the
    /// halo region centered on `target_chunk_id`. Phase 2 feeds this seed to
    /// `AdvancedErosionSimulator::new` so adjacent chunks' halos that overlap
    /// in world space produce identical droplet trajectories in the overlap
    /// region.
    #[allow(dead_code)] // Wired in phase 2; validated by unit tests here.
    pub(crate) fn halo_seed(
        world_seed: u64,
        target_chunk_id: ChunkId,
        halo_chunks: u32,
    ) -> u64 {
        let halo_origin_x = target_chunk_id.x.wrapping_sub(halo_chunks as i32);
        let halo_origin_z = target_chunk_id.z.wrapping_sub(halo_chunks as i32);
        // Wang-style integer hash, deterministic across runs / platforms.
        let mut h = world_seed;
        h = h
            .wrapping_add(halo_origin_x as u64)
            .wrapping_mul(0x9E3779B97F4A7C15);
        h ^= h >> 32;
        h = h
            .wrapping_add(halo_origin_z as u64)
            .wrapping_mul(0x85EBCA6BE11ECC0D);
        h ^= h >> 32;
        h
    }

    /// Generate and register a chunk (mutable version for compatibility)
    pub fn generate_and_register_chunk(
        &mut self,
        chunk_id: ChunkId,
    ) -> anyhow::Result<TerrainChunk> {
        let chunk = self.generate_chunk(chunk_id)?;
        self.chunk_manager.add_chunk(chunk.clone());
        Ok(chunk)
    }

    /// Get an existing chunk if it's loaded
    pub fn get_chunk(&self, chunk_id: ChunkId) -> Option<&TerrainChunk> {
        self.chunk_manager.get_chunk(chunk_id)
    }

    /// Stream chunks around a center position, loading/unloading as needed
    pub fn stream_chunks(&mut self, center: Vec3, radius: u32) -> anyhow::Result<Vec<ChunkId>> {
        let chunks_to_load = self.chunk_manager.get_chunks_in_radius(center, radius);
        let mut loaded = Vec::new();

        for chunk_id in chunks_to_load {
            if !self.chunk_manager.has_chunk(chunk_id) {
                self.generate_and_register_chunk(chunk_id)?;
                loaded.push(chunk_id);
            }
        }

        // Unload chunks that are too far away
        let unload_radius = radius + 2; // Keep a buffer
        self.chunk_manager
            .unload_distant_chunks(center, unload_radius);

        Ok(loaded)
    }

    /// Assign biomes to heightmap points based on climate data
    fn assign_biomes(
        &self,
        heightmap: &Heightmap,
        climate_data: &[(f32, f32)], // (temperature, moisture) pairs
    ) -> anyhow::Result<Vec<BiomeType>> {
        let mut biome_map = Vec::with_capacity(climate_data.len());

        for (i, &(temperature, moisture)) in climate_data.iter().enumerate() {
            let height = heightmap.get_height_at_index(i);
            let biome = self.find_best_biome(height, temperature, moisture);
            biome_map.push(biome);
        }

        Ok(biome_map)
    }

    /// Find the best biome for given environmental conditions
    fn find_best_biome(&self, height: f32, temperature: f32, moisture: f32) -> BiomeType {
        let mut best_biome = BiomeType::Grassland;
        let mut best_score = f32::NEG_INFINITY;

        for biome_config in &self.config.biomes {
            let score = biome_config.score_conditions(height, temperature, moisture);
            if score > best_score {
                best_score = score;
                best_biome = biome_config.biome_type;
            }
        }

        best_biome
    }

    /// Get the world configuration
    pub fn config(&self) -> &WorldConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_generator_creation() {
        let config = WorldConfig::default();
        let generator = WorldGenerator::new(config);
        assert_eq!(generator.config.seed, 12345);
    }

    /// Phase 1.6-F.3-phase-1.B: `halo_seed` must be deterministic for the same
    /// (world_seed, target_chunk_id, halo_chunks) triple, and must produce
    /// different seeds for different target chunks (so adjacent halos have
    /// distinct PRNG streams).
    #[test]
    fn phase_1_6_f3_phase_1_halo_seed_deterministic() {
        let s1 = WorldGenerator::halo_seed(12345, ChunkId::new(5, 3), 1);
        let s2 = WorldGenerator::halo_seed(12345, ChunkId::new(5, 3), 1);
        assert_eq!(s1, s2, "halo_seed should be deterministic");
    }

    #[test]
    fn phase_1_6_f3_phase_1_halo_seed_differs_per_chunk() {
        let s00 = WorldGenerator::halo_seed(12345, ChunkId::new(0, 0), 1);
        let s10 = WorldGenerator::halo_seed(12345, ChunkId::new(1, 0), 1);
        let s01 = WorldGenerator::halo_seed(12345, ChunkId::new(0, 1), 1);
        assert_ne!(s00, s10, "adjacent chunks should get different halo seeds");
        assert_ne!(s00, s01, "adjacent chunks should get different halo seeds");
        assert_ne!(s10, s01, "non-identical chunks should get different halo seeds");
    }

    #[test]
    fn phase_1_6_f3_phase_1_halo_seed_differs_per_world_seed() {
        let s1 = WorldGenerator::halo_seed(12345, ChunkId::new(0, 0), 1);
        let s2 = WorldGenerator::halo_seed(67890, ChunkId::new(0, 0), 1);
        assert_ne!(s1, s2, "different world seeds should yield different halo seeds");
    }

    #[test]
    fn test_chunk_generation() -> anyhow::Result<()> {
        let config = WorldConfig::default();
        let generator = WorldGenerator::new(config);

        let chunk_id = ChunkId::new(0, 0);
        let chunk = generator.generate_chunk(chunk_id)?;

        assert_eq!(chunk.id(), chunk_id);
        assert!(chunk.heightmap().max_height() >= chunk.heightmap().min_height());

        Ok(())
    }

    #[test]
    fn test_chunk_streaming() -> anyhow::Result<()> {
        let config = WorldConfig::default();
        let mut generator = WorldGenerator::new(config);

        let center = Vec3::new(128.0, 0.0, 128.0);
        let loaded_chunks = generator.stream_chunks(center, 2)?;

        assert!(!loaded_chunks.is_empty());

        Ok(())
    }
}
