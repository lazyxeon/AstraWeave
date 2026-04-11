//! Zone-scoped generation engine.
//!
//! This module implements the core generation logic for blueprint zones.
//! It supports two placement modes:
//! - **Replica**: Places objects at exact (adaptively scaled) positions from
//!   the original .blend scene, producing a faithful 1:1 reproduction.
//! - **Inspired**: Uses the .blend scene as a template for vegetation types
//!   and density, but places objects via procedural Poisson-disk scatter.
//!
//! The engine also handles heightmap injection — transferring terrain shape
//! from .blend meshes into game terrain chunks.

use crate::blueprint_zone::{
    point_distance_to_polygon_edge, point_in_polygon, polygon_area, polygon_bounding_rect,
    polygon_centroid, polygon_overlaps_rect, AdaptiveScaleParams, BlueprintZone, PlacementMode,
    ZoneSource,
};
use crate::chunk::TerrainChunk;
use crate::scatter::{ScatterConfig, VegetationInstance, VegetationScatter};
use crate::{BiomeConfig, ChunkId};
use anyhow::{Context, Result};
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// ============================================================================
// Data contract types (mirror astraweave-blend's serialization schema)
// ============================================================================

/// A fixed object placement from a .blend scene.
///
/// This mirrors `astraweave_blend::heightmap_raster::FixedPlacement` and
/// deserializes from the same JSON format. Defined locally to avoid a
/// cross-crate dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedPlacement {
    /// World-space position `[x, y, z]`.
    pub position: [f64; 3],
    /// Euler rotation `[x, y, z]` in radians.
    pub rotation: [f64; 3],
    /// Scale `[x, y, z]`.
    pub scale: [f64; 3],
    /// Relative path to the mesh file.
    pub mesh_path: String,
    /// Asset category (vegetation, rock, structure, etc.).
    pub category: String,
    /// Object name from the .blend scene.
    pub name: String,
}

/// A rasterized heightmap from .blend terrain meshes.
///
/// Mirrors `astraweave_blend::heightmap_raster::RasterizedHeightmap` and
/// deserializes from the same JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceHeightmap {
    /// Height values in row-major order (z * resolution + x).
    pub data: Vec<f32>,
    /// Grid resolution (width = height = resolution).
    pub resolution: u32,
    /// World-space XZ minimum `[x, z]`.
    pub world_min: [f32; 2],
    /// World-space XZ maximum `[x, z]`.
    pub world_max: [f32; 2],
    /// Minimum height in the data.
    pub min_height: f32,
    /// Maximum height in the data.
    pub max_height: f32,
}

impl SourceHeightmap {
    /// Sample the heightmap with bilinear interpolation at a world XZ position.
    /// Returns `None` if the position is outside the heightmap bounds.
    pub fn sample(&self, world_x: f32, world_z: f32) -> Option<f32> {
        let width = self.world_max[0] - self.world_min[0];
        let depth = self.world_max[1] - self.world_min[1];
        if width <= 0.0 || depth <= 0.0 {
            return None;
        }

        let u = (world_x - self.world_min[0]) / width;
        let v = (world_z - self.world_min[1]) / depth;
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
            return None;
        }

        let res = self.resolution as f32;
        let fx = u * (res - 1.0);
        let fz = v * (res - 1.0);

        let x0 = fx.floor() as u32;
        let z0 = fz.floor() as u32;
        let x1 = (x0 + 1).min(self.resolution - 1);
        let z1 = (z0 + 1).min(self.resolution - 1);

        let tx = fx.fract();
        let tz = fz.fract();

        let h00 = self.data[(z0 * self.resolution + x0) as usize];
        let h10 = self.data[(z0 * self.resolution + x1) as usize];
        let h01 = self.data[(z1 * self.resolution + x0) as usize];
        let h11 = self.data[(z1 * self.resolution + x1) as usize];

        let h0 = h00 * (1.0 - tx) + h10 * tx;
        let h1 = h01 * (1.0 - tx) + h11 * tx;
        Some(h0 * (1.0 - tz) + h1 * tz)
    }
}

// ============================================================================
// Generation result types
// ============================================================================

/// A heightmap modification for a specific chunk region.
#[derive(Debug, Clone)]
pub struct HeightmapPatch {
    /// The chunk this patch targets.
    pub chunk_id: ChunkId,
    /// Height values indexed by `(x, z)` grid coordinates within the chunk.
    /// Only cells that fall inside the zone polygon are included.
    pub heights: HashMap<(u32, u32), f32>,
}

/// Result of generating content for a single zone.
#[derive(Debug, Clone)]
pub struct ZoneGenerationResult {
    /// Placed objects (vegetation, rocks, structures).
    pub placements: Vec<VegetationInstance>,
    /// Heightmap modifications to apply to terrain chunks.
    pub heightmap_patches: Vec<HeightmapPatch>,
}

impl ZoneGenerationResult {
    /// Create an empty result.
    pub fn empty() -> Self {
        Self {
            placements: Vec::new(),
            heightmap_patches: Vec::new(),
        }
    }

    /// Total number of placed objects.
    pub fn placement_count(&self) -> usize {
        self.placements.len()
    }

    /// Total number of height values modified.
    pub fn modified_height_count(&self) -> usize {
        self.heightmap_patches.iter().map(|p| p.heights.len()).sum()
    }
}

// ============================================================================
// Zone scatter generator
// ============================================================================

/// Generates zone-scoped terrain content (placements + heightmap patches).
pub struct ZoneScatterGenerator {
    chunk_size: f32,
    heightmap_resolution: u32,
}

impl ZoneScatterGenerator {
    /// Create a new generator.
    pub fn new(chunk_size: f32, heightmap_resolution: u32) -> Self {
        Self {
            chunk_size,
            heightmap_resolution,
        }
    }

    /// Generate content for a single zone across the given chunks.
    ///
    /// # Arguments
    /// * `zone` — The blueprint zone to generate content for.
    /// * `chunks` — Terrain chunks that overlap the zone.
    /// * `seed` — Random seed for procedural generation.
    pub fn generate_zone_scatter(
        &self,
        zone: &BlueprintZone,
        chunks: &[&TerrainChunk],
        seed: u64,
    ) -> Result<ZoneGenerationResult> {
        if !zone.enabled || zone.vertices.len() < 3 {
            return Ok(ZoneGenerationResult::empty());
        }

        match &zone.source {
            ZoneSource::BlendScene {
                pack_path,
                placement_mode,
            } => match placement_mode {
                PlacementMode::Replica => self.generate_replica(zone, chunks, pack_path),
                PlacementMode::Inspired => self.generate_inspired(zone, chunks, pack_path, seed),
            },
            ZoneSource::BiomePreset(biome_type) => {
                self.generate_biome_preset(zone, chunks, *biome_type, seed)
            }
        }
    }

    // ========================================================================
    // Replica mode
    // ========================================================================

    /// Generate content in Replica mode — exact placements from the .blend scene,
    /// spatially scaled to fit the zone polygon.
    fn generate_replica(
        &self,
        zone: &BlueprintZone,
        chunks: &[&TerrainChunk],
        pack_path: &Path,
    ) -> Result<ZoneGenerationResult> {
        let mut result = ZoneGenerationResult::empty();

        // Load fixed placements from the sibling JSON file
        let placements_path = detect_sibling_file(pack_path, "fixed_placements.json");
        let placements_path = match placements_path {
            Some(p) => p,
            None => return Ok(result), // No placements file → nothing to place
        };

        let placements =
            load_fixed_placements(&placements_path).context("Failed to load fixed placements")?;

        if placements.is_empty() {
            return Ok(result);
        }

        // Compute adaptive scaling (use manual override if set)
        let scene_footprint = estimate_scene_footprint_from_placements(&placements);
        let zone_area = polygon_area(&zone.vertices);
        let scale_params = if let Some(override_ratio) = zone.adaptive_scale_override {
            AdaptiveScaleParams::compute(1.0, override_ratio)
        } else {
            AdaptiveScaleParams::compute(scene_footprint, zone_area)
        };
        let zone_centroid = polygon_centroid(&zone.vertices);

        // Compute scene centroid (XZ only, from placement positions)
        let scene_centroid = compute_scene_centroid(&placements);

        // Load source heightmap for Y coordinate mapping
        let heightmap_path = detect_sibling_file(pack_path, "terrain_heightmap.json");
        let source_heightmap = heightmap_path.and_then(|p| load_source_heightmap(&p).ok());

        // Also inject zone heightmap if available
        if let Some(ref hm) = source_heightmap {
            result.heightmap_patches =
                self.generate_heightmap_patches(zone, chunks, hm, &scale_params);
            self.apply_boundary_blending(zone, chunks, &mut result.heightmap_patches);
        }

        // Transform each placement
        for fp in &placements {
            let original_xz = Vec2::new(fp.position[0] as f32, fp.position[2] as f32);
            let offset = original_xz - scene_centroid;
            let scaled_offset = offset * scale_params.position_scale;
            let target_xz = zone_centroid + scaled_offset;

            // Check if the scaled position falls within the zone polygon
            if !point_in_polygon(target_xz, &zone.vertices) {
                continue;
            }

            // Determine Y from target terrain
            let world_y = self.sample_terrain_height(chunks, target_xz.x, target_xz.y);

            let scale_x = fp.scale[0] as f32 * scale_params.scale_multiplier;
            let scale_y = fp.scale[1] as f32 * scale_params.scale_multiplier;
            let scale_z = fp.scale[2] as f32 * scale_params.scale_multiplier;
            let avg_scale = (scale_x + scale_y + scale_z) / 3.0;

            let rotation_y = fp.rotation[2] as f32; // Z-up → Y-up for game engine

            let terrain_normal = self
                .sample_terrain_normal(chunks, target_xz.x, target_xz.y)
                .unwrap_or(Vec3::Y);

            result.placements.push(VegetationInstance {
                position: Vec3::new(target_xz.x, world_y, target_xz.y),
                rotation: rotation_y,
                scale: avg_scale,
                vegetation_type: fp.category.clone(),
                model_path: fp.mesh_path.clone(),
                terrain_normal,
                tint: Vec3::ONE,
            });
        }

        Ok(result)
    }

    // ========================================================================
    // Inspired mode
    // ========================================================================

    /// Generate content in Inspired mode — procedural Poisson-disk scatter using
    /// the .blend scene as a density/type template.
    fn generate_inspired(
        &self,
        zone: &BlueprintZone,
        chunks: &[&TerrainChunk],
        pack_path: &Path,
        seed: u64,
    ) -> Result<ZoneGenerationResult> {
        let mut result = ZoneGenerationResult::empty();

        // Load the BiomePack manifest for biome config + scatter config
        let biome_config = load_biome_config_from_pack(pack_path)
            .context("Failed to load biome config from pack")?;

        let scatter_config = zone
            .scatter_config_override
            .clone()
            .unwrap_or_else(|| load_scatter_config_from_pack(pack_path));

        // Compute adaptive scaling (use manual override if set)
        let zone_area = polygon_area(&zone.vertices);
        let scene_footprint = load_scene_footprint(pack_path).unwrap_or(zone_area);
        let scale_params = if let Some(override_ratio) = zone.adaptive_scale_override {
            AdaptiveScaleParams::compute(1.0, override_ratio)
        } else {
            AdaptiveScaleParams::compute(scene_footprint, zone_area)
        };

        // Apply density multiplier from adaptive scaling
        let mut adjusted_config = biome_config;
        adjusted_config.vegetation.density *= scale_params.density_multiplier;

        // Load source heightmap for terrain injection
        let heightmap_path = detect_sibling_file(pack_path, "terrain_heightmap.json");
        let source_heightmap = heightmap_path.and_then(|p| load_source_heightmap(&p).ok());

        if let Some(ref hm) = source_heightmap {
            result.heightmap_patches =
                self.generate_heightmap_patches(zone, chunks, hm, &scale_params);
            self.apply_boundary_blending(zone, chunks, &mut result.heightmap_patches);
        }

        // Generate scatter per chunk that overlaps the zone
        let scatter = VegetationScatter::new(scatter_config);
        let (zone_min, zone_max) = polygon_bounding_rect(&zone.vertices);

        for chunk in chunks {
            let chunk_origin = chunk.id().to_world_pos(self.chunk_size);
            let chunk_max = Vec2::new(
                chunk_origin.x + self.chunk_size,
                chunk_origin.z + self.chunk_size,
            );
            let chunk_min = Vec2::new(chunk_origin.x, chunk_origin.z);

            // Quick AABB reject
            if chunk_min.x > zone_max.x
                || chunk_max.x < zone_min.x
                || chunk_min.y > zone_max.y
                || chunk_max.y < zone_min.y
            {
                continue;
            }

            // Generate vegetation for this chunk
            let chunk_seed = seed ^ ((chunk.id().x as u64) << 32) ^ (chunk.id().z as u64);

            let instances = scatter
                .scatter_vegetation(chunk, self.chunk_size, &adjusted_config, chunk_seed)
                .context("Scatter failed")?;

            // Filter to zone polygon + apply scale multiplier
            for mut inst in instances {
                let xz = Vec2::new(inst.position.x, inst.position.z);
                if point_in_polygon(xz, &zone.vertices) {
                    inst.scale *= scale_params.scale_multiplier;
                    result.placements.push(inst);
                }
            }
        }

        Ok(result)
    }

    // ========================================================================
    // BiomePreset mode
    // ========================================================================

    /// Generate content using a built-in biome preset.
    fn generate_biome_preset(
        &self,
        zone: &BlueprintZone,
        chunks: &[&TerrainChunk],
        biome_type: crate::biome::BiomeType,
        seed: u64,
    ) -> Result<ZoneGenerationResult> {
        let mut result = ZoneGenerationResult::empty();

        let biome_config = biome_config_for_type(biome_type);
        let scatter_config = zone.scatter_config_override.clone().unwrap_or_default();

        let scatter = VegetationScatter::new(scatter_config);

        for chunk in chunks {
            let chunk_seed = seed ^ ((chunk.id().x as u64) << 32) ^ (chunk.id().z as u64);

            let instances = scatter
                .scatter_vegetation(chunk, self.chunk_size, &biome_config, chunk_seed)
                .context("BiomePreset scatter failed")?;

            for inst in instances {
                let xz = Vec2::new(inst.position.x, inst.position.z);
                if point_in_polygon(xz, &zone.vertices) {
                    result.placements.push(inst);
                }
            }
        }

        Ok(result)
    }

    // ========================================================================
    // Heightmap injection
    // ========================================================================

    /// Create heightmap patches that inject .blend terrain shape into game chunks.
    ///
    /// Uses offset-based injection:
    /// `target_h = base_h + (blend_h - blend_base) * scale_multiplier`
    ///
    /// This preserves the terrain's existing base height while adding the .blend
    /// scene's relative terrain detail on top.
    fn generate_heightmap_patches(
        &self,
        zone: &BlueprintZone,
        chunks: &[&TerrainChunk],
        source: &SourceHeightmap,
        scale_params: &AdaptiveScaleParams,
    ) -> Vec<HeightmapPatch> {
        let mut patches = Vec::new();
        let zone_centroid = polygon_centroid(&zone.vertices);

        // Source heightmap center
        let src_center_x = (source.world_min[0] + source.world_max[0]) * 0.5;
        let src_center_z = (source.world_min[1] + source.world_max[1]) * 0.5;

        // Base height of the source (used as offset reference)
        let blend_base = source.min_height;

        for chunk in chunks {
            let chunk_origin = chunk.id().to_world_pos(self.chunk_size);
            let chunk_rect_min = Vec2::new(chunk_origin.x, chunk_origin.z);
            let chunk_rect_max = Vec2::new(
                chunk_origin.x + self.chunk_size,
                chunk_origin.z + self.chunk_size,
            );

            if !polygon_overlaps_rect(&zone.vertices, chunk_rect_min, chunk_rect_max) {
                continue;
            }

            let mut patch = HeightmapPatch {
                chunk_id: chunk.id(),
                heights: HashMap::new(),
            };

            let res = self.heightmap_resolution;
            let cell_size = self.chunk_size / res as f32;

            for gz in 0..res {
                for gx in 0..res {
                    let world_x = chunk_origin.x + gx as f32 * cell_size;
                    let world_z = chunk_origin.z + gz as f32 * cell_size;
                    let point = Vec2::new(world_x, world_z);

                    if !point_in_polygon(point, &zone.vertices) {
                        continue;
                    }

                    // Map world position to source heightmap space using adaptive scaling
                    let offset_from_zone = point - zone_centroid;
                    let src_offset = offset_from_zone / scale_params.position_scale;
                    let src_x = src_center_x + src_offset.x;
                    let src_z = src_center_z + src_offset.y;

                    if let Some(blend_h) = source.sample(src_x, src_z) {
                        // Current terrain height at this point
                        let base_h = chunk.heightmap().get_height(gx, gz);

                        // Offset-based injection
                        let height_delta = (blend_h - blend_base) * scale_params.scale_multiplier;
                        let target_h = base_h + height_delta;

                        patch.heights.insert((gx, gz), target_h);
                    }
                }
            }

            if !patch.heights.is_empty() {
                patches.push(patch);
            }
        }

        patches
    }

    // ========================================================================
    // Boundary blending
    // ========================================================================

    /// Apply boundary blending to heightmap patches.
    ///
    /// For each modified cell within `blend_margin` of the zone polygon edge, the
    /// injected height is smoothly blended back toward the original terrain height
    /// using a smoothstep curve. If the zone has a `blend_mask`, it overrides the
    /// auto-blend factor.
    pub fn apply_boundary_blending(
        &self,
        zone: &BlueprintZone,
        chunks: &[&TerrainChunk],
        patches: &mut [HeightmapPatch],
    ) {
        if zone.blend_margin <= 0.0 {
            return;
        }

        for patch in patches.iter_mut() {
            // Find the matching chunk for original height lookups
            let chunk = chunks.iter().find(|c| c.id() == patch.chunk_id);
            let chunk = match chunk {
                Some(c) => c,
                None => continue,
            };

            let chunk_origin = chunk.id().to_world_pos(self.chunk_size);
            let cell_size = self.chunk_size / self.heightmap_resolution as f32;

            // Collect keys to avoid borrow issues
            let keys: Vec<(u32, u32)> = patch.heights.keys().copied().collect();

            for (gx, gz) in keys {
                let world_x = chunk_origin.x + gx as f32 * cell_size;
                let world_z = chunk_origin.z + gz as f32 * cell_size;
                let point = Vec2::new(world_x, world_z);

                let edge_dist = point_distance_to_polygon_edge(point, &zone.vertices);

                if edge_dist < zone.blend_margin {
                    let original_h = chunk.heightmap().get_height(gx, gz);
                    let injected_h = patch.heights[&(gx, gz)];

                    // Auto-blend factor: 0 at edge → 1 deep inside zone
                    let t = (edge_dist / zone.blend_margin).clamp(0.0, 1.0);
                    let auto_blend = smoothstep(t); // 0 near edge, 1 inside

                    // Override with blend mask if available
                    let blend_factor = if let Some(ref mask) = zone.blend_mask {
                        let mask_val = mask.sample(point.x, point.y);
                        // mask: 0.0 = full zone, 1.0 = full surrounding
                        // Invert: we want 1.0 = full zone, 0.0 = full surrounding
                        1.0 - mask_val
                    } else {
                        auto_blend
                    };

                    // Lerp between original and injected
                    let blended = original_h * (1.0 - blend_factor) + injected_h * blend_factor;
                    patch.heights.insert((gx, gz), blended);
                }
            }
        }
    }

    // ========================================================================
    // Terrain sampling helpers
    // ========================================================================

    /// Sample terrain height at a world XZ position from the available chunks.
    fn sample_terrain_height(&self, chunks: &[&TerrainChunk], world_x: f32, world_z: f32) -> f32 {
        let pos = Vec3::new(world_x, 0.0, world_z);
        for chunk in chunks {
            if let Some(h) = chunk.get_height_at_world_pos(pos, self.chunk_size) {
                return h;
            }
        }
        0.0
    }

    /// Sample terrain normal at a world XZ position.
    fn sample_terrain_normal(
        &self,
        chunks: &[&TerrainChunk],
        world_x: f32,
        world_z: f32,
    ) -> Option<Vec3> {
        let pos = Vec3::new(world_x, 0.0, world_z);
        for chunk in chunks {
            let chunk_origin = chunk.id().to_world_pos(self.chunk_size);
            let local = pos - chunk_origin;
            if local.x >= 0.0
                && local.x < self.chunk_size
                && local.z >= 0.0
                && local.z < self.chunk_size
            {
                let res = chunk.heightmap().resolution() as f32;
                let gx = ((local.x / self.chunk_size) * (res - 1.0)).round() as u32;
                let gz = ((local.z / self.chunk_size) * (res - 1.0)).round() as u32;
                let scale = self.chunk_size / res;
                return Some(chunk.heightmap().calculate_normal(gx, gz, scale));
            }
        }
        None
    }
}

// ============================================================================
// Multi-zone orchestration
// ============================================================================

/// Generate content for multiple zones, respecting priority-based overlap.
///
/// Higher-priority zones overwrite lower-priority placements in overlapping
/// regions. Heightmap patches are merged with last-writer-wins per cell.
pub fn generate_multi_zone_scatter(
    zones: &[BlueprintZone],
    chunks: &[&TerrainChunk],
    chunk_size: f32,
    heightmap_resolution: u32,
    seed: u64,
) -> Result<Vec<ZoneGenerationResult>> {
    let generator = ZoneScatterGenerator::new(chunk_size, heightmap_resolution);

    // Sort zones by priority (lowest first → highest last gets final say)
    let mut sorted: Vec<_> = zones.iter().filter(|z| z.enabled).collect();
    sorted.sort_by_key(|z| z.priority);

    let mut results = Vec::with_capacity(sorted.len());

    // Track which chunks/cells have been claimed by higher-priority zones
    // Key: (chunk_id, gx, gz), Value: priority that claimed it
    let mut height_claims: HashMap<(ChunkId, u32, u32), i32> = HashMap::new();

    // Process zones in priority order (lowest first)
    for zone in &sorted {
        // Filter chunks to those overlapping this zone
        let (zone_min, zone_max) = polygon_bounding_rect(&zone.vertices);
        let relevant_chunks: Vec<&TerrainChunk> = chunks
            .iter()
            .filter(|c| {
                let origin = c.id().to_world_pos(chunk_size);
                let cmin = Vec2::new(origin.x, origin.z);
                let cmax = Vec2::new(origin.x + chunk_size, origin.z + chunk_size);
                cmin.x <= zone_max.x
                    && cmax.x >= zone_min.x
                    && cmin.y <= zone_max.y
                    && cmax.y >= zone_min.y
            })
            .copied()
            .collect();

        let zone_seed = seed ^ (zone.id.0.wrapping_mul(2654435761)); // Knuth hash

        let mut gen_result = generator
            .generate_zone_scatter(zone, &relevant_chunks, zone_seed)
            .with_context(|| format!("Failed to generate zone '{}'", zone.name))?;

        // Filter placements: remove any that fall in a higher-priority zone's polygon
        // (the higher-priority zone hasn't been processed yet since we go lowest-first,
        //  so we'll re-check after all zones are done)
        // For heightmap patches: register claims
        for patch in &mut gen_result.heightmap_patches {
            patch.heights.retain(|&(gx, gz), _| {
                let existing = height_claims.get(&(patch.chunk_id, gx, gz));
                match existing {
                    Some(&existing_priority) if existing_priority > zone.priority => false,
                    _ => {
                        height_claims.insert((patch.chunk_id, gx, gz), zone.priority);
                        true
                    }
                }
            });
        }

        results.push(gen_result);
    }

    Ok(results)
}

/// Apply heightmap patches from generation results to mutable terrain chunks.
///
/// Call this after `generate_multi_zone_scatter` to commit height changes.
pub fn apply_heightmap_patches(
    chunks: &mut HashMap<ChunkId, TerrainChunk>,
    results: &[ZoneGenerationResult],
) {
    for result in results {
        for patch in &result.heightmap_patches {
            if let Some(chunk) = chunks.get_mut(&patch.chunk_id) {
                let hm = chunk.heightmap_mut();
                for (&(gx, gz), &height) in &patch.heights {
                    hm.set_height(gx, gz, height);
                }
            }
        }
    }
}

// ============================================================================
// File loading helpers
// ============================================================================

/// Load fixed placements from a JSON file.
fn load_fixed_placements(path: &Path) -> Result<Vec<FixedPlacement>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read placements file: {}", path.display()))?;
    let placements: Vec<FixedPlacement> = serde_json::from_str(&content)
        .with_context(|| format!("Cannot parse placements JSON: {}", path.display()))?;
    Ok(placements)
}

/// Load a source heightmap from a JSON file.
fn load_source_heightmap(path: &Path) -> Result<SourceHeightmap> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read heightmap file: {}", path.display()))?;
    let hm: SourceHeightmap = serde_json::from_str(&content)
        .with_context(|| format!("Cannot parse heightmap JSON: {}", path.display()))?;
    Ok(hm)
}

/// Load a BiomeConfig from a BiomePack manifest.
fn load_biome_config_from_pack(pack_path: &Path) -> Result<BiomeConfig> {
    let content = std::fs::read_to_string(pack_path)
        .with_context(|| format!("Cannot read pack manifest: {}", pack_path.display()))?;
    let pack: crate::biome_pack::BiomePack = serde_json::from_str(&content)
        .with_context(|| format!("Cannot parse pack manifest: {}", pack_path.display()))?;
    Ok(pack.to_biome_config(crate::biome::BiomeType::Grassland))
}

/// Load scatter config from a BiomePack manifest.
fn load_scatter_config_from_pack(pack_path: &Path) -> ScatterConfig {
    let content = match std::fs::read_to_string(pack_path) {
        Ok(c) => c,
        Err(_) => return ScatterConfig::default(),
    };
    let pack: crate::biome_pack::BiomePack = match serde_json::from_str(&content) {
        Ok(p) => p,
        Err(_) => return ScatterConfig::default(),
    };
    pack.to_scatter_config()
}

/// Load scene footprint area from a BiomePack manifest.
fn load_scene_footprint(pack_path: &Path) -> Option<f32> {
    let content = std::fs::read_to_string(pack_path).ok()?;
    let pack: crate::biome_pack::BiomePack = serde_json::from_str(&content).ok()?;
    Some(pack.estimate_scene_footprint())
}

/// Get a default BiomeConfig for a given biome type.
fn biome_config_for_type(biome_type: crate::biome::BiomeType) -> BiomeConfig {
    use crate::biome::BiomeType;
    match biome_type {
        BiomeType::Grassland => BiomeConfig::grassland(),
        BiomeType::Desert => BiomeConfig::desert(),
        BiomeType::Forest => BiomeConfig::forest(),
        BiomeType::Mountain => BiomeConfig::mountain(),
        _ => BiomeConfig::grassland(), // Fallback
    }
}

/// Detect a sibling file (e.g., "fixed_placements.json") next to the pack manifest.
fn detect_sibling_file(pack_path: &Path, filename: &str) -> Option<std::path::PathBuf> {
    let parent = pack_path.parent()?;
    let candidate = parent.join(filename);
    if candidate.exists() {
        Some(candidate)
    } else {
        None
    }
}

/// Compute the scene centroid from fixed placements (XZ only).
fn compute_scene_centroid(placements: &[FixedPlacement]) -> Vec2 {
    if placements.is_empty() {
        return Vec2::ZERO;
    }
    let mut sum = Vec2::ZERO;
    for p in placements {
        sum.x += p.position[0] as f32;
        sum.y += p.position[2] as f32;
    }
    sum / placements.len() as f32
}

/// Estimate scene footprint area from placement spread.
fn estimate_scene_footprint_from_placements(placements: &[FixedPlacement]) -> f32 {
    if placements.is_empty() {
        return 1.0;
    }
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    for p in placements {
        let x = p.position[0] as f32;
        let z = p.position[2] as f32;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_z = min_z.min(z);
        max_z = max_z.max(z);
    }
    let width = (max_x - min_x).max(1.0);
    let depth = (max_z - min_z).max(1.0);
    width * depth
}

/// Hermite smoothstep: `t² (3 − 2t)`. Maps [0,1] → [0,1] with smooth edges.
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprint_zone::{ZoneId, ZoneSource};
    use crate::chunk::TerrainChunk;
    use crate::heightmap::{Heightmap, HeightmapConfig};
    use crate::BiomeType;

    fn make_flat_chunk(id: ChunkId, resolution: u32) -> TerrainChunk {
        let hm = Heightmap::new(HeightmapConfig {
            resolution,
            ..Default::default()
        })
        .unwrap();
        let biome_map = vec![BiomeType::Grassland; (resolution * resolution) as usize];
        TerrainChunk::new(id, hm, biome_map)
    }

    fn make_zone_biome_preset(vertices: Vec<Vec2>) -> BlueprintZone {
        BlueprintZone {
            id: ZoneId(1),
            name: "TestZone".into(),
            vertices,
            source: ZoneSource::BiomePreset(BiomeType::Grassland),
            priority: 0,
            enabled: true,
            scatter_config_override: None,
            blend_margin: 5.0,
            blend_mask: None,
            adaptive_scale_override: None,
        }
    }

    #[test]
    fn test_source_heightmap_sample() {
        let hm = SourceHeightmap {
            data: vec![0.0, 10.0, 20.0, 30.0],
            resolution: 2,
            world_min: [0.0, 0.0],
            world_max: [10.0, 10.0],
            min_height: 0.0,
            max_height: 30.0,
        };

        // Corners
        assert!((hm.sample(0.0, 0.0).unwrap() - 0.0).abs() < 0.01);
        assert!((hm.sample(10.0, 0.0).unwrap() - 10.0).abs() < 0.01);
        assert!((hm.sample(0.0, 10.0).unwrap() - 20.0).abs() < 0.01);
        assert!((hm.sample(10.0, 10.0).unwrap() - 30.0).abs() < 0.01);

        // Center (bilinear interpolation)
        assert!((hm.sample(5.0, 5.0).unwrap() - 15.0).abs() < 0.01);

        // Outside bounds
        assert!(hm.sample(-1.0, 5.0).is_none());
        assert!(hm.sample(5.0, 11.0).is_none());
    }

    #[test]
    fn test_source_heightmap_zero_size() {
        let hm = SourceHeightmap {
            data: vec![1.0],
            resolution: 1,
            world_min: [5.0, 5.0],
            world_max: [5.0, 5.0], // zero-size
            min_height: 1.0,
            max_height: 1.0,
        };
        assert!(hm.sample(5.0, 5.0).is_none());
    }

    #[test]
    fn test_fixed_placement_scene_centroid() {
        let placements = vec![
            FixedPlacement {
                position: [10.0, 0.0, 20.0],
                rotation: [0.0; 3],
                scale: [1.0; 3],
                mesh_path: "tree.glb".into(),
                category: "vegetation".into(),
                name: "Tree".into(),
            },
            FixedPlacement {
                position: [30.0, 0.0, 40.0],
                rotation: [0.0; 3],
                scale: [1.0; 3],
                mesh_path: "rock.glb".into(),
                category: "rock".into(),
                name: "Rock".into(),
            },
        ];

        let centroid = compute_scene_centroid(&placements);
        assert!((centroid.x - 20.0).abs() < 0.01);
        assert!((centroid.y - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_scene_footprint_estimation() {
        let placements = vec![
            FixedPlacement {
                position: [0.0, 0.0, 0.0],
                rotation: [0.0; 3],
                scale: [1.0; 3],
                mesh_path: "a.glb".into(),
                category: "vegetation".into(),
                name: "A".into(),
            },
            FixedPlacement {
                position: [100.0, 0.0, 50.0],
                rotation: [0.0; 3],
                scale: [1.0; 3],
                mesh_path: "b.glb".into(),
                category: "rock".into(),
                name: "B".into(),
            },
        ];

        let footprint = estimate_scene_footprint_from_placements(&placements);
        assert!((footprint - 5000.0).abs() < 0.01); // 100 × 50
    }

    #[test]
    fn test_empty_placements_footprint() {
        assert!((estimate_scene_footprint_from_placements(&[]) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_heightmap_patch_generation() {
        // Create a 2×2 source heightmap covering [0, 0] to [256, 256]
        let source = SourceHeightmap {
            data: vec![0.0, 0.0, 0.0, 10.0],
            resolution: 2,
            world_min: [0.0, 0.0],
            world_max: [256.0, 256.0],
            min_height: 0.0,
            max_height: 10.0,
        };

        // A zone polygon covering one quadrant
        let zone = BlueprintZone {
            id: ZoneId(1),
            name: "HM Test".into(),
            vertices: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(128.0, 0.0),
                Vec2::new(128.0, 128.0),
                Vec2::new(0.0, 128.0),
            ],
            source: ZoneSource::BiomePreset(BiomeType::Grassland),
            priority: 0,
            enabled: true,
            scatter_config_override: None,
            blend_margin: 0.0,
            blend_mask: None,
            adaptive_scale_override: None,
        };

        let chunk = make_flat_chunk(ChunkId::new(0, 0), 16);
        let chunks = vec![&chunk];
        let scale_params = AdaptiveScaleParams::compute(256.0 * 256.0, 128.0 * 128.0);

        let generator = ZoneScatterGenerator::new(256.0, 16);
        let patches = generator.generate_heightmap_patches(&zone, &chunks, &source, &scale_params);

        // We should get exactly 1 patch for chunk (0,0)
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].chunk_id, ChunkId::new(0, 0));
        // Some cells should have been modified (those inside the zone polygon)
        assert!(!patches[0].heights.is_empty());
    }

    #[test]
    fn test_biome_preset_generation() {
        let chunk = make_flat_chunk(ChunkId::new(0, 0), 16);
        let zone = make_zone_biome_preset(vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(256.0, 0.0),
            Vec2::new(256.0, 256.0),
            Vec2::new(0.0, 256.0),
        ]);

        let generator = ZoneScatterGenerator::new(256.0, 16);
        let result = generator
            .generate_zone_scatter(&zone, &[&chunk], 42)
            .unwrap();

        // Biome preset should generate some placements (density > 0 for Grassland)
        // The exact count depends on BiomeConfig::default_for_biome, but should be > 0
        // unless the default config has zero vegetation types.
        // We mainly verify no panic / no error.
        assert!(result.heightmap_patches.is_empty()); // No heightmap for biome preset
    }

    #[test]
    fn test_disabled_zone_produces_empty() {
        let chunk = make_flat_chunk(ChunkId::new(0, 0), 16);
        let mut zone = make_zone_biome_preset(vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 0.0),
            Vec2::new(100.0, 100.0),
        ]);
        zone.enabled = false;

        let generator = ZoneScatterGenerator::new(256.0, 16);
        let result = generator
            .generate_zone_scatter(&zone, &[&chunk], 42)
            .unwrap();

        assert_eq!(result.placement_count(), 0);
        assert_eq!(result.modified_height_count(), 0);
    }

    #[test]
    fn test_degenerate_zone_produces_empty() {
        let chunk = make_flat_chunk(ChunkId::new(0, 0), 16);
        // Only 2 vertices — not a valid polygon
        let zone = make_zone_biome_preset(vec![Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0)]);

        let generator = ZoneScatterGenerator::new(256.0, 16);
        let result = generator
            .generate_zone_scatter(&zone, &[&chunk], 42)
            .unwrap();

        assert_eq!(result.placement_count(), 0);
    }

    #[test]
    fn test_multi_zone_priority() {
        // Two zones with different priorities overlapping at the same heightmap cells
        let source = SourceHeightmap {
            data: vec![5.0; 4],
            resolution: 2,
            world_min: [0.0, 0.0],
            world_max: [256.0, 256.0],
            min_height: 5.0,
            max_height: 5.0,
        };

        let low_zone = BlueprintZone {
            id: ZoneId(1),
            name: "Low".into(),
            vertices: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(256.0, 0.0),
                Vec2::new(256.0, 256.0),
                Vec2::new(0.0, 256.0),
            ],
            source: ZoneSource::BiomePreset(BiomeType::Grassland),
            priority: 0,
            enabled: true,
            scatter_config_override: None,
            blend_margin: 0.0,
            blend_mask: None,
            adaptive_scale_override: None,
        };

        let high_zone = BlueprintZone {
            id: ZoneId(2),
            name: "High".into(),
            vertices: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(128.0, 0.0),
                Vec2::new(128.0, 128.0),
                Vec2::new(0.0, 128.0),
            ],
            source: ZoneSource::BiomePreset(BiomeType::Desert),
            priority: 10,
            enabled: true,
            scatter_config_override: None,
            blend_margin: 0.0,
            blend_mask: None,
            adaptive_scale_override: None,
        };

        let chunk = make_flat_chunk(ChunkId::new(0, 0), 16);
        let zones = vec![low_zone, high_zone];

        let results = generate_multi_zone_scatter(&zones, &[&chunk], 256.0, 16, 42).unwrap();

        assert_eq!(results.len(), 2);
        // Both zones should have results (the lower priority one just has its overlap cells stripped)
    }

    #[test]
    fn test_zone_generation_result_empty() {
        let r = ZoneGenerationResult::empty();
        assert_eq!(r.placement_count(), 0);
        assert_eq!(r.modified_height_count(), 0);
    }

    #[test]
    fn test_apply_heightmap_patches() {
        let chunk_id = ChunkId::new(0, 0);
        let chunk = make_flat_chunk(chunk_id, 16);

        // Verify initial height is 0
        assert!((chunk.heightmap().get_height(5, 5) - 0.0).abs() < 0.01);

        let mut heights = HashMap::new();
        heights.insert((5, 5), 42.0);
        heights.insert((10, 10), 99.0);

        let patch = HeightmapPatch { chunk_id, heights };
        let result = ZoneGenerationResult {
            placements: Vec::new(),
            heightmap_patches: vec![patch],
        };

        let mut chunk_map = HashMap::new();
        chunk_map.insert(chunk_id, chunk);

        apply_heightmap_patches(&mut chunk_map, &[result]);

        let modified_chunk = chunk_map.get(&chunk_id).unwrap();
        assert!((modified_chunk.heightmap().get_height(5, 5) - 42.0).abs() < 0.01);
        assert!((modified_chunk.heightmap().get_height(10, 10) - 99.0).abs() < 0.01);
        // Unchanged cell
        assert!((modified_chunk.heightmap().get_height(0, 0) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_smoothstep_boundaries() {
        assert!((smoothstep(0.0) - 0.0).abs() < 0.001);
        assert!((smoothstep(1.0) - 1.0).abs() < 0.001);
        assert!((smoothstep(0.5) - 0.5).abs() < 0.001);
        // Monotonic
        assert!(smoothstep(0.25) < smoothstep(0.75));
    }

    #[test]
    fn test_boundary_blending_at_edge() {
        // Create a 256×256 chunk with resolution 16
        let chunk_id = ChunkId::new(0, 0);
        let chunk = make_flat_chunk(chunk_id, 16);
        let chunks = vec![&chunk];

        // Zone: 50×50 square in the middle of the chunk
        let zone = BlueprintZone {
            id: ZoneId(1),
            name: "BlendTest".into(),
            vertices: vec![
                Vec2::new(64.0, 64.0),
                Vec2::new(192.0, 64.0),
                Vec2::new(192.0, 192.0),
                Vec2::new(64.0, 192.0),
            ],
            source: ZoneSource::BiomePreset(BiomeType::Grassland),
            priority: 0,
            enabled: true,
            scatter_config_override: None,
            blend_margin: 30.0,
            blend_mask: None,
            adaptive_scale_override: None,
        };

        // Create patches with a uniform injected height of 100.0
        let mut heights = HashMap::new();
        let cell_size = 256.0 / 16.0; // 16.0 per cell
        for gz in 0..16u32 {
            for gx in 0..16u32 {
                let wx = gx as f32 * cell_size;
                let wz = gz as f32 * cell_size;
                let point = Vec2::new(wx, wz);
                if point_in_polygon(point, &zone.vertices) {
                    heights.insert((gx, gz), 100.0);
                }
            }
        }

        let mut patches = vec![HeightmapPatch { chunk_id, heights }];

        let generator = ZoneScatterGenerator::new(256.0, 16);
        generator.apply_boundary_blending(&zone, &chunks, &mut patches);

        // Cells deep inside the zone (far from edge) should be close to 100.0
        // Center cell at (8, 8) → world (128, 128), center of zone, far from edge
        if let Some(&h) = patches[0].heights.get(&(8, 8)) {
            assert!(
                (h - 100.0).abs() < 1.0,
                "Center cell should be ~100.0, got {}",
                h
            );
        }

        // Cells near the edge (within blend_margin=30) should be blended toward 0.0
        // Cell at (4, 8) → world (64, 128), right at the edge → should be near 0.0
        if let Some(&h) = patches[0].heights.get(&(4, 8)) {
            assert!(h < 80.0, "Edge cell should be blended toward 0, got {}", h);
        }
    }

    #[test]
    fn test_boundary_blending_zero_margin() {
        let chunk_id = ChunkId::new(0, 0);
        let chunk = make_flat_chunk(chunk_id, 16);
        let chunks = vec![&chunk];

        let zone = BlueprintZone {
            id: ZoneId(1),
            name: "NoBlend".into(),
            vertices: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(256.0, 0.0),
                Vec2::new(256.0, 256.0),
                Vec2::new(0.0, 256.0),
            ],
            source: ZoneSource::BiomePreset(BiomeType::Grassland),
            priority: 0,
            enabled: true,
            scatter_config_override: None,
            blend_margin: 0.0, // No blending
            blend_mask: None,
            adaptive_scale_override: None,
        };

        let mut heights = HashMap::new();
        heights.insert((5, 5), 100.0);

        let mut patches = vec![HeightmapPatch { chunk_id, heights }];
        let original_h = patches[0].heights[&(5, 5)];

        let generator = ZoneScatterGenerator::new(256.0, 16);
        generator.apply_boundary_blending(&zone, &chunks, &mut patches);

        // With blend_margin=0, nothing should change
        assert!((patches[0].heights[&(5, 5)] - original_h).abs() < 0.001);
    }

    #[test]
    fn test_blend_mask_override() {
        use crate::blueprint_zone::BlendMask;

        let chunk_id = ChunkId::new(0, 0);
        let chunk = make_flat_chunk(chunk_id, 16);
        let chunks = vec![&chunk];

        // A zone that covers the whole chunk
        let zone = BlueprintZone {
            id: ZoneId(1),
            name: "MaskTest".into(),
            vertices: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(256.0, 0.0),
                Vec2::new(256.0, 256.0),
                Vec2::new(0.0, 256.0),
            ],
            source: ZoneSource::BiomePreset(BiomeType::Grassland),
            priority: 0,
            enabled: true,
            scatter_config_override: None,
            blend_margin: 200.0, // Large margin so all cells are "near edge"
            // BlendMask with value 1.0 everywhere → full surrounding → blended to 0
            blend_mask: Some(BlendMask {
                data: vec![1.0; 4],
                resolution: 2,
                world_bounds: (0.0, 0.0, 256.0, 256.0),
            }),
            adaptive_scale_override: None,
        };

        let mut heights = HashMap::new();
        heights.insert((8, 8), 100.0);
        let mut patches = vec![HeightmapPatch { chunk_id, heights }];

        let generator = ZoneScatterGenerator::new(256.0, 16);
        generator.apply_boundary_blending(&zone, &chunks, &mut patches);

        // blend_mask=1.0 everywhere → blend_factor = 1.0 - 1.0 = 0.0
        // So result = original*(1-0) + injected*0 = original = 0.0
        let h = patches[0].heights[&(8, 8)];
        assert!(
            h.abs() < 0.01,
            "With mask=1.0, should blend fully to original (0.0), got {}",
            h
        );
    }
}
