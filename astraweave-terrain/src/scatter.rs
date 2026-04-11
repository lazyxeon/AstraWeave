//! Vegetation and object scatter system

use crate::{Biome, BiomeConfig, ChunkId, TerrainChunk};
use astraweave_gameplay::{spawn_resources, ResourceNode};
use glam::Vec3;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

/// Generate a per-instance tint color centered around 1.0 with subtle variation.
/// Uses position-based hashing for deterministic results.
/// `variation` controls the maximum deviation from 1.0 (e.g. 0.1 → range [0.9, 1.1]).
fn generate_instance_tint(position: Vec3, variation: f32) -> Vec3 {
    // Simple bit-mixing hash from position for deterministic per-instance variation
    let hash_x = (position.x * 73856093.0) as i32;
    let hash_y = (position.y * 19349663.0) as i32;
    let hash_z = (position.z * 83492791.0) as i32;
    let h = (hash_x ^ hash_y ^ hash_z) as u32;

    // Extract 3 independent channels from the hash bits
    let r = ((h & 0xFF) as f32) / 255.0; // 0..1
    let g = (((h >> 8) & 0xFF) as f32) / 255.0;
    let b = (((h >> 16) & 0xFF) as f32) / 255.0;

    // Map to [1.0 - variation, 1.0 + variation]
    Vec3::new(
        1.0 + (r * 2.0 - 1.0) * variation,
        1.0 + (g * 2.0 - 1.0) * variation,
        1.0 + (b * 2.0 - 1.0) * variation,
    )
}

/// LOD-based density falloff configuration for vegetation.
///
/// Mirrors the editor `FoliageType` LOD settings. At each LOD band boundary
/// the instance density is halved, and beyond `cull_distance` no instances
/// are generated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VegetationLodConfig {
    /// Distance thresholds for each LOD level (up to 4 bands).
    /// LOD0: [0, lod_distances[0]) — full density
    /// LOD1: [lod_distances[0], lod_distances[1]) — 50% density
    /// LOD2: [lod_distances[1], lod_distances[2]) — 25% density
    /// LOD3: [lod_distances[2], lod_distances[3]) — 12.5% density
    pub lod_distances: [f32; 4],
    /// Beyond this distance, all instances are culled (density = 0).
    pub cull_distance: f32,
}

impl Default for VegetationLodConfig {
    fn default() -> Self {
        Self {
            lod_distances: [50.0, 150.0, 500.0, 1000.0],
            cull_distance: 1500.0,
        }
    }
}

/// Compute the density multiplier for a vegetation instance at the given
/// distance from the camera.
///
/// Returns a value in `[0.0, 1.0]` that should be used to probabilistically
/// thin instances in the scatter pass.
///
/// - Within LOD0 range: 1.0 (full density)
/// - Each successive LOD band halves the density
/// - Beyond `cull_distance`: 0.0
pub fn density_at_distance(dist: f32, lod: &VegetationLodConfig) -> f32 {
    if dist > lod.cull_distance {
        return 0.0;
    }
    let mut density = 1.0f32;
    for &lod_dist in &lod.lod_distances {
        if dist < lod_dist {
            return density;
        }
        density *= 0.5;
    }
    density
}

/// A placed vegetation instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VegetationInstance {
    /// World position
    pub position: Vec3,
    /// Rotation in radians around Y axis
    pub rotation: f32,
    /// Scale multiplier
    pub scale: f32,
    /// Vegetation type name
    pub vegetation_type: String,
    /// Model path for rendering
    pub model_path: String,
    /// Terrain surface normal at placement point
    pub terrain_normal: Vec3,
    /// Per-instance color tint for visual variation (RGB, centered around 1.0)
    #[serde(default = "default_tint")]
    pub tint: Vec3,
}

fn default_tint() -> Vec3 {
    Vec3::ONE
}

/// A scatter pattern configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScatterConfig {
    /// Use Poisson disk sampling for natural distribution
    pub use_poisson_disk: bool,
    /// Minimum distance between objects (for Poisson disk)
    pub min_distance: f32,
    /// Maximum slope allowed (degrees)
    pub max_slope: f32,
    /// Maximum surface curvature (convexity) allowed.
    /// Positive values reject ridge tips and spire peaks.
    /// Default 0.15 rejects only sharp knife-edge features.
    pub max_curvature: f32,
    /// Height range filter (min, max)
    pub height_filter: Option<(f32, f32)>,
    /// Random seed offset for this scatter type
    pub seed_offset: u64,
}

impl Default for ScatterConfig {
    fn default() -> Self {
        Self {
            use_poisson_disk: true,
            min_distance: 2.0,
            max_slope: 35.0,
            max_curvature: 0.15,
            height_filter: None,
            seed_offset: 0,
        }
    }
}

impl ScatterConfig {
    /// Create a ScatterConfig from a [`BiomePack`](crate::biome_pack::BiomePack).
    pub fn from_biome_pack(pack: &crate::biome_pack::BiomePack) -> Self {
        pack.to_scatter_config()
    }
}

/// Vegetation scatter system that places objects based on biome rules
pub struct VegetationScatter {
    config: ScatterConfig,
}

impl VegetationScatter {
    /// Create a new vegetation scatter system
    pub fn new(config: ScatterConfig) -> Self {
        Self { config }
    }

    /// Generate vegetation instances for a terrain chunk
    pub fn scatter_vegetation(
        &self,
        chunk: &TerrainChunk,
        chunk_size: f32,
        biome_config: &BiomeConfig,
        seed: u64,
    ) -> anyhow::Result<Vec<VegetationInstance>> {
        let mut instances = Vec::new();
        let _chunk_origin = chunk.id().to_world_pos(chunk_size);

        if biome_config.vegetation.vegetation_types.is_empty() {
            return Ok(instances);
        }

        // Calculate approximate number of vegetation instances
        let chunk_area = chunk_size * chunk_size;
        let target_count = (chunk_area * biome_config.vegetation.density) as usize;

        if target_count == 0 {
            return Ok(instances);
        }

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed + self.config.seed_offset);

        if self.config.use_poisson_disk {
            instances = self.generate_poisson_disk_scatter(
                chunk,
                chunk_size,
                biome_config,
                &mut rng,
                target_count,
            )?;
        } else {
            instances = self.generate_random_scatter(
                chunk,
                chunk_size,
                biome_config,
                &mut rng,
                target_count,
            )?;
        }

        Ok(instances)
    }

    /// Hierarchical multi-pass vegetation scatter.
    ///
    /// Sorts vegetation types by `placement_priority` (trees first, then shrubs,
    /// then grass) and performs separate Poisson-disk passes for each priority
    /// tier.  Higher-priority (earlier) placements create exclusion zones that
    /// later passes must respect.
    ///
    /// Falls back to `scatter_vegetation` when no types define priorities.
    pub fn scatter_vegetation_hierarchical(
        &self,
        chunk: &TerrainChunk,
        chunk_size: f32,
        biome_config: &BiomeConfig,
        seed: u64,
    ) -> anyhow::Result<Vec<VegetationInstance>> {
        let veg_types = &biome_config.vegetation.vegetation_types;
        if veg_types.is_empty() {
            return Ok(Vec::new());
        }

        // Check if any type defines non-default priority
        let has_priorities = veg_types.iter().any(|v| v.placement_priority != 0);
        if !has_priorities {
            // No hierarchical data — use the standard path
            return self.scatter_vegetation(chunk, chunk_size, biome_config, seed);
        }

        // Group types by priority tier
        let mut tiers: std::collections::BTreeMap<u8, Vec<usize>> =
            std::collections::BTreeMap::new();
        for (i, vt) in veg_types.iter().enumerate() {
            tiers.entry(vt.placement_priority).or_default().push(i);
        }

        let chunk_origin = chunk.id().to_world_pos(chunk_size);

        // Exclusion zones from earlier tiers: (position_xz, exclusion_radius²)
        let mut exclusion_zones: Vec<(Vec3, f32)> = Vec::new();

        // Spatial grid for fast exclusion lookups
        let excl_cell_size = 4.0f32; // coarse grid for exclusion checks
        let excl_grid_dim = ((chunk_size / excl_cell_size).ceil() as usize).max(1);
        let mut excl_grid: Vec<Vec<usize>> = vec![Vec::new(); excl_grid_dim * excl_grid_dim];

        let mut all_instances: Vec<VegetationInstance> = Vec::new();
        let chunk_area = chunk_size * chunk_size;
        let base_density = biome_config.vegetation.density;

        let hmin = chunk.heightmap().min_height();
        let hmax = chunk.heightmap().max_height();
        let altitude_ceiling = hmin + (hmax - hmin) * 0.90;

        for (_priority, type_indices) in &tiers {
            // Build a sub-BiomeConfig with only this tier's types
            let tier_weight_sum: f32 = type_indices
                .iter()
                .map(|&i| veg_types[i].weight)
                .sum();

            if tier_weight_sum <= 0.0 {
                continue;
            }

            // Per-tier density scaled by weight fraction
            let tier_density = base_density * tier_weight_sum;
            let tier_target = (chunk_area * tier_density) as usize;
            if tier_target == 0 {
                continue;
            }

            // Per-species min_distance: use the smallest non-zero value in the tier
            let tier_min_dist = type_indices
                .iter()
                .map(|&i| veg_types[i].min_distance)
                .filter(|&d| d > 0.0)
                .fold(self.config.min_distance, f32::min);

            let min_dist_sq = tier_min_dist * tier_min_dist;

            // Build placement grid for this tier
            let cell_size = tier_min_dist;
            let grid_dim = ((chunk_size / cell_size).ceil() as usize).max(1);
            let mut grid: Vec<Vec<usize>> = vec![Vec::new(); grid_dim * grid_dim];

            let mut rng = rand::rngs::StdRng::seed_from_u64(
                seed.wrapping_add(self.config.seed_offset)
                    .wrapping_add(*_priority as u64 * 0x9E37_79B9),
            );

            let effective_target = tier_target.min(16_384);
            let max_attempts = effective_target * 15;
            let mut attempts = 0;
            let tier_start = all_instances.len();

            while (all_instances.len() - tier_start) < effective_target && attempts < max_attempts {
                attempts += 1;

                let local_x = rng.random::<f32>() * chunk_size;
                let local_z = rng.random::<f32>() * chunk_size;
                let mut world_pos =
                    Vec3::new(chunk_origin.x + local_x, 0.0, chunk_origin.z + local_z);

                let height = match chunk.get_height_at_world_pos(world_pos, chunk_size) {
                    Some(h) => h,
                    None => continue,
                };
                world_pos.y = height;

                // Altitude ceiling
                if height > altitude_ceiling {
                    continue;
                }

                // Height filter
                if let Some((min_h, max_h)) = self.config.height_filter {
                    if height < min_h || height > max_h {
                        continue;
                    }
                }

                // Slope + curvature
                let (slope, terrain_normal) =
                    self.estimate_slope_and_normal(chunk, world_pos, chunk_size);
                if slope > self.config.max_slope {
                    continue;
                }
                let curvature = self.estimate_curvature(chunk, world_pos, chunk_size);
                if curvature > self.config.max_curvature {
                    continue;
                }

                // Intra-tier minimum distance check
                let gx = ((local_x / cell_size) as usize).min(grid_dim - 1);
                let gz = ((local_z / cell_size) as usize).min(grid_dim - 1);
                let x_min = gx.saturating_sub(1);
                let x_max = (gx + 1).min(grid_dim - 1);
                let z_min = gz.saturating_sub(1);
                let z_max = (gz + 1).min(grid_dim - 1);

                let mut too_close = false;
                'intra: for nz in z_min..=z_max {
                    for nx in x_min..=x_max {
                        for &idx in &grid[nz * grid_dim + nx] {
                            let diff = all_instances[idx].position - world_pos;
                            if diff.x * diff.x + diff.z * diff.z < min_dist_sq {
                                too_close = true;
                                break 'intra;
                            }
                        }
                    }
                }
                if too_close {
                    continue;
                }

                // Exclusion zone check from earlier tiers
                let egx = ((local_x / excl_cell_size) as usize).min(excl_grid_dim - 1);
                let egz = ((local_z / excl_cell_size) as usize).min(excl_grid_dim - 1);
                // Check 3×3 neighborhood for exclusion zones
                let ex_min = egx.saturating_sub(1);
                let ex_max = (egx + 1).min(excl_grid_dim - 1);
                let ez_min = egz.saturating_sub(1);
                let ez_max = (egz + 1).min(excl_grid_dim - 1);

                let mut excluded = false;
                'excl: for ez in ez_min..=ez_max {
                    for ex in ex_min..=ex_max {
                        for &ei in &excl_grid[ez * excl_grid_dim + ex] {
                            let (epos, erad_sq) = exclusion_zones[ei];
                            let diff = epos - world_pos;
                            if diff.x * diff.x + diff.z * diff.z < erad_sq {
                                excluded = true;
                                break 'excl;
                            }
                        }
                    }
                }
                if excluded {
                    continue;
                }

                // Select type from this tier (weighted)
                let type_roll = rng.random::<f32>() * tier_weight_sum;
                let mut accum = 0.0f32;
                let mut selected_idx = type_indices[0];
                for &ti in type_indices {
                    accum += veg_types[ti].weight;
                    if type_roll <= accum {
                        selected_idx = ti;
                        break;
                    }
                }

                let selected = &veg_types[selected_idx];

                // Per-type slope tolerance check
                if slope > selected.slope_tolerance {
                    continue;
                }

                // Per-species altitude band check
                if let Some((alt_min, alt_max)) = selected.altitude_range {
                    if height < alt_min || height > alt_max {
                        continue;
                    }
                }

                let scale = rng.random_range(selected.scale_range.0..=selected.scale_range.1);
                let rotation = if biome_config.vegetation.random_rotation {
                    rng.random::<f32>() * std::f32::consts::TAU
                } else {
                    0.0
                };

                let idx = all_instances.len();
                all_instances.push(VegetationInstance {
                    position: world_pos,
                    rotation,
                    scale,
                    vegetation_type: selected.name.clone(),
                    model_path: selected.model_path.clone(),
                    terrain_normal,
                    tint: generate_instance_tint(world_pos, 0.1),
                });

                // Register in intra-tier grid
                grid[gz * grid_dim + gx].push(idx);

                // Register exclusion zone for this instance
                let excl_r = selected.exclusion_radius;
                if excl_r > 0.0 {
                    let excl_idx = exclusion_zones.len();
                    exclusion_zones.push((world_pos, excl_r * excl_r));
                    excl_grid[egz * excl_grid_dim + egx].push(excl_idx);
                }
            }
        }

        Ok(all_instances)
    }

    /// Generate scatter using Poisson disk sampling for natural distribution.
    ///
    /// Uses Bridson's algorithm (2007) with an active-list approach for O(n)
    /// guaranteed placement, replacing the previous rejection-based dart-throwing
    /// which degraded to >90% rejection rate near saturation.
    ///
    /// Grid cell size = min_distance / √2 so each cell holds at most one sample
    /// and only a 5×5 neighborhood check is needed.
    fn generate_poisson_disk_scatter(
        &self,
        chunk: &TerrainChunk,
        chunk_size: f32,
        biome_config: &BiomeConfig,
        rng: &mut rand::rngs::StdRng,
        target_count: usize,
    ) -> anyhow::Result<Vec<VegetationInstance>> {
        let mut instances: Vec<VegetationInstance> = Vec::new();
        let chunk_origin = chunk.id().to_world_pos(chunk_size);
        let min_dist = self.config.min_distance;
        let min_dist_sq = min_dist * min_dist;

        // Altitude ceiling: reject placements above 90% of the chunk's height
        // range to prevent trees on improbable mountain peaks.
        let hmin = chunk.heightmap().min_height();
        let hmax = chunk.heightmap().max_height();
        let altitude_ceiling = hmin + (hmax - hmin) * 0.90;

        // Bridson's grid: cell_size = r/√2 ensures at most one sample per cell.
        let cell_size = min_dist / std::f32::consts::SQRT_2;
        let grid_dim = ((chunk_size / cell_size).ceil() as usize).max(1);
        let mut grid: Vec<Option<usize>> = vec![None; grid_dim * grid_dim];

        let effective_target = target_count.min(16_384);
        let k = 30u32; // candidates per active point (Bridson's standard)

        // Active list: indices into `instances` of points that can still spawn neighbors.
        let mut active_list: Vec<usize> = Vec::new();

        // --- Seed: find a valid initial point via random sampling ---
        for _ in 0..1000 {
            let local_x = rng.random::<f32>() * chunk_size;
            let local_z = rng.random::<f32>() * chunk_size;
            let mut world_pos =
                Vec3::new(chunk_origin.x + local_x, 0.0, chunk_origin.z + local_z);

            let height = match chunk.get_height_at_world_pos(world_pos, chunk_size) {
                Some(h) => h,
                None => continue,
            };
            world_pos.y = height;

            if !self.passes_terrain_filters(
                world_pos,
                height,
                altitude_ceiling,
                chunk,
                chunk_size,
            ) {
                continue;
            }

            if let Some(instance) = self.create_vegetation_instance(
                world_pos,
                biome_config,
                rng,
                self.estimate_slope_and_normal(chunk, world_pos, chunk_size).0,
                self.estimate_slope_and_normal(chunk, world_pos, chunk_size).1,
            )? {
                let gx = ((local_x / cell_size) as usize).min(grid_dim - 1);
                let gz = ((local_z / cell_size) as usize).min(grid_dim - 1);
                let idx = instances.len();
                instances.push(instance);
                grid[gz * grid_dim + gx] = Some(idx);
                active_list.push(idx);
                break;
            }
        }

        // --- Main Bridson loop ---
        while !active_list.is_empty() && instances.len() < effective_target {
            // Pick a random parent from the active list.
            let ai = rng.random_range(0..active_list.len());
            let parent_idx = active_list[ai];
            let parent_pos = instances[parent_idx].position;
            let parent_lx = parent_pos.x - chunk_origin.x;
            let parent_lz = parent_pos.z - chunk_origin.z;

            let mut accepted_any = false;
            for _ in 0..k {
                // Generate candidate in annulus [r, 2r] around parent.
                let angle = rng.random::<f32>() * std::f32::consts::TAU;
                let radius = min_dist + rng.random::<f32>() * min_dist;
                let cx = parent_lx + radius * angle.cos();
                let cz = parent_lz + radius * angle.sin();

                // Bounds check
                if cx < 0.0 || cx >= chunk_size || cz < 0.0 || cz >= chunk_size {
                    continue;
                }

                // Grid-accelerated distance check (5×5 neighborhood)
                let gx = ((cx / cell_size) as usize).min(grid_dim - 1);
                let gz = ((cz / cell_size) as usize).min(grid_dim - 1);

                let mut too_close = false;
                let x_min = gx.saturating_sub(2);
                let x_max = (gx + 2).min(grid_dim - 1);
                let z_min = gz.saturating_sub(2);
                let z_max = (gz + 2).min(grid_dim - 1);

                'neighbor: for nz in z_min..=z_max {
                    for nx in x_min..=x_max {
                        if let Some(ni) = grid[nz * grid_dim + nx] {
                            let diff = instances[ni].position
                                - Vec3::new(chunk_origin.x + cx, 0.0, chunk_origin.z + cz);
                            if diff.x * diff.x + diff.z * diff.z < min_dist_sq {
                                too_close = true;
                                break 'neighbor;
                            }
                        }
                    }
                }

                if too_close {
                    continue;
                }

                // Terrain validation
                let mut world_pos =
                    Vec3::new(chunk_origin.x + cx, 0.0, chunk_origin.z + cz);
                let height = match chunk.get_height_at_world_pos(world_pos, chunk_size) {
                    Some(h) => h,
                    None => continue,
                };
                world_pos.y = height;

                if !self.passes_terrain_filters(
                    world_pos,
                    height,
                    altitude_ceiling,
                    chunk,
                    chunk_size,
                ) {
                    continue;
                }

                let (slope, terrain_normal) =
                    self.estimate_slope_and_normal(chunk, world_pos, chunk_size);

                if let Some(instance) = self.create_vegetation_instance(
                    world_pos,
                    biome_config,
                    rng,
                    slope,
                    terrain_normal,
                )? {
                    let idx = instances.len();
                    instances.push(instance);
                    grid[gz * grid_dim + gx] = Some(idx);
                    active_list.push(idx);
                    accepted_any = true;
                }
            }

            if !accepted_any {
                active_list.swap_remove(ai);
            }
        }

        Ok(instances)
    }

    /// Check height filter, altitude ceiling, slope, and curvature constraints.
    fn passes_terrain_filters(
        &self,
        world_pos: Vec3,
        height: f32,
        altitude_ceiling: f32,
        chunk: &TerrainChunk,
        chunk_size: f32,
    ) -> bool {
        if let Some((min_height, max_height)) = self.config.height_filter {
            if height < min_height || height > max_height {
                return false;
            }
        }
        if height > altitude_ceiling {
            return false;
        }
        let (slope, _) = self.estimate_slope_and_normal(chunk, world_pos, chunk_size);
        if slope > self.config.max_slope {
            return false;
        }
        let curvature = self.estimate_curvature(chunk, world_pos, chunk_size);
        if curvature > self.config.max_curvature {
            return false;
        }
        true
    }

    /// Generate scatter using simple random placement
    fn generate_random_scatter(
        &self,
        chunk: &TerrainChunk,
        chunk_size: f32,
        biome_config: &BiomeConfig,
        rng: &mut rand::rngs::StdRng,
        target_count: usize,
    ) -> anyhow::Result<Vec<VegetationInstance>> {
        let mut instances = Vec::new();
        let chunk_origin = chunk.id().to_world_pos(chunk_size);

        // Altitude ceiling: reject placements above 90% of the chunk's height range
        let hmin = chunk.heightmap().min_height();
        let hmax = chunk.heightmap().max_height();
        let altitude_ceiling = hmin + (hmax - hmin) * 0.90;

        for _ in 0..target_count {
            // Generate random position
            let local_x = rng.random::<f32>() * chunk_size;
            let local_z = rng.random::<f32>() * chunk_size;
            let mut world_pos = Vec3::new(chunk_origin.x + local_x, 0.0, chunk_origin.z + local_z);

            // Get height and biome at this position
            if let Some(height) = chunk.get_height_at_world_pos(world_pos, chunk_size) {
                world_pos.y = height;

                // Check height filter
                if let Some((min_height, max_height)) = self.config.height_filter {
                    if height < min_height || height > max_height {
                        continue;
                    }
                }

                // Altitude ceiling: no vegetation on the very top of peaks
                if height > altitude_ceiling {
                    continue;
                }

                // Check slope
                let (slope, terrain_normal) =
                    self.estimate_slope_and_normal(chunk, world_pos, chunk_size);
                if slope > self.config.max_slope {
                    continue;
                }

                // Curvature filter: reject ridge tips and spire peaks
                let curvature = self.estimate_curvature(chunk, world_pos, chunk_size);
                if curvature > self.config.max_curvature {
                    continue;
                }

                // Create vegetation instance
                if let Some(vegetation_instance) = self.create_vegetation_instance(
                    world_pos,
                    biome_config,
                    rng,
                    slope,
                    terrain_normal,
                )? {
                    instances.push(vegetation_instance);
                }
            }
        }

        Ok(instances)
    }

    /// Estimate slope at a position using multi-scale height sampling.
    ///
    /// Samples at two offsets (0.5m and 2.0m) and takes the **maximum** slope
    /// reading.  The fine scale catches micro-features (spikes, ridges) while
    /// the coarse scale catches broad cliff faces where a single 1m sample
    /// could land on an adjacent flat face, producing a false-low reading.
    ///
    /// Returns `(slope_degrees, terrain_normal)`.
    fn estimate_slope_and_normal(
        &self,
        chunk: &TerrainChunk,
        world_pos: Vec3,
        chunk_size: f32,
    ) -> (f32, Vec3) {
        let height_center = world_pos.y;

        // --- Fine-scale sample (0.5 m) — catches micro-features -----------
        let fine = self.slope_at_offset(chunk, world_pos, chunk_size, 0.5, height_center);
        // --- Coarse-scale sample (2.0 m) — catches broad cliff faces ------
        let coarse = self.slope_at_offset(chunk, world_pos, chunk_size, 2.0, height_center);

        // Take the steepest reading from either scale for filtering, but use
        // the fine-scale normal for surface alignment (more localised).
        if coarse.0 > fine.0 {
            (coarse.0, fine.1)
        } else {
            fine
        }
    }

    /// Compute slope and surface normal at a single sample offset.
    /// Returns `(slope_degrees, terrain_normal)`.
    fn slope_at_offset(
        &self,
        chunk: &TerrainChunk,
        world_pos: Vec3,
        chunk_size: f32,
        offset: f32,
        height_center: f32,
    ) -> (f32, Vec3) {
        let height_px = chunk
            .get_height_at_world_pos(world_pos + Vec3::new(offset, 0.0, 0.0), chunk_size)
            .unwrap_or(height_center);
        let height_nx = chunk
            .get_height_at_world_pos(world_pos + Vec3::new(-offset, 0.0, 0.0), chunk_size)
            .unwrap_or(height_center);
        let height_pz = chunk
            .get_height_at_world_pos(world_pos + Vec3::new(0.0, 0.0, offset), chunk_size)
            .unwrap_or(height_center);
        let height_nz = chunk
            .get_height_at_world_pos(world_pos + Vec3::new(0.0, 0.0, -offset), chunk_size)
            .unwrap_or(height_center);

        // Central-difference gradient (more accurate than one-sided)
        let dx = (height_px - height_nx) / (2.0 * offset);
        let dz = (height_pz - height_nz) / (2.0 * offset);
        let slope_radians = (dx * dx + dz * dz).sqrt().atan();

        // Normal from gradient: n = normalize(-dh/dx, 1, -dh/dz)
        let normal = Vec3::new(-dx, 1.0, -dz).normalize_or(Vec3::Y);

        (slope_radians.to_degrees(), normal)
    }

    /// Estimate local surface curvature (Laplacian of the height field).
    ///
    /// Returns a positive value proportional to convexity (ridges, peaks).
    /// Values > `threshold` indicate placement points that sit on knife-edge
    /// ridges or spire tips where vegetation cannot credibly grow.
    fn estimate_curvature(&self, chunk: &TerrainChunk, world_pos: Vec3, chunk_size: f32) -> f32 {
        let h = 2.0; // Sample spacing for second derivative
        let hc = world_pos.y;

        let hpx = chunk
            .get_height_at_world_pos(world_pos + Vec3::new(h, 0.0, 0.0), chunk_size)
            .unwrap_or(hc);
        let hnx = chunk
            .get_height_at_world_pos(world_pos + Vec3::new(-h, 0.0, 0.0), chunk_size)
            .unwrap_or(hc);
        let hpz = chunk
            .get_height_at_world_pos(world_pos + Vec3::new(0.0, 0.0, h), chunk_size)
            .unwrap_or(hc);
        let hnz = chunk
            .get_height_at_world_pos(world_pos + Vec3::new(0.0, 0.0, -h), chunk_size)
            .unwrap_or(hc);

        // Discrete Laplacian: d²h/dx² + d²h/dz²
        let d2x = (hpx - 2.0 * hc + hnx) / (h * h);
        let d2z = (hpz - 2.0 * hc + hnz) / (h * h);

        // Negative Laplacian = convexity (positive on ridges/peaks)
        -(d2x + d2z)
    }

    /// Create a vegetation instance with appropriate type and scaling
    fn create_vegetation_instance(
        &self,
        position: Vec3,
        biome_config: &BiomeConfig,
        rng: &mut rand::rngs::StdRng,
        slope: f32,
        terrain_normal: Vec3,
    ) -> anyhow::Result<Option<VegetationInstance>> {
        // Filter vegetation types by slope tolerance and altitude range
        let height = position.y;
        let suitable_types: Vec<_> = biome_config
            .vegetation
            .vegetation_types
            .iter()
            .filter(|veg_type| slope <= veg_type.slope_tolerance)
            .filter(|veg_type| match veg_type.altitude_range {
                Some((alt_min, alt_max)) => height >= alt_min && height <= alt_max,
                None => true,
            })
            .collect();

        if suitable_types.is_empty() {
            return Ok(None);
        }

        // Weighted random selection
        let total_weight: f32 = suitable_types.iter().map(|vt| vt.weight).sum();
        if total_weight <= 0.0 {
            return Ok(None);
        }

        let random_value = rng.random::<f32>() * total_weight;
        let mut accumulated_weight = 0.0;
        let mut selected_type = suitable_types[0];

        for veg_type in &suitable_types {
            accumulated_weight += veg_type.weight;
            if random_value <= accumulated_weight {
                selected_type = veg_type;
                break;
            }
        }

        // Generate scale
        let scale = if biome_config.vegetation.random_rotation {
            rng.random_range(selected_type.scale_range.0..=selected_type.scale_range.1)
        } else {
            (selected_type.scale_range.0 + selected_type.scale_range.1) * 0.5
        };

        // Generate rotation
        let rotation = if biome_config.vegetation.random_rotation {
            rng.random::<f32>() * std::f32::consts::TAU
        } else {
            0.0
        };

        Ok(Some(VegetationInstance {
            position,
            rotation,
            scale,
            vegetation_type: selected_type.name.clone(),
            model_path: selected_type.model_path.clone(),
            terrain_normal,
            tint: generate_instance_tint(position, 0.1),
        }))
    }

    /// Generate resource nodes using existing spawn_resources function
    pub fn scatter_resources(
        &self,
        chunk: &TerrainChunk,
        chunk_size: f32,
        biome_config: &BiomeConfig,
        seed: u64,
    ) -> anyhow::Result<Vec<ResourceNode>> {
        let chunk_origin = chunk.id().to_world_pos(chunk_size);
        let area_min = chunk_origin;
        let area_max = chunk_origin + Vec3::new(chunk_size, 0.0, chunk_size);

        // Calculate resource count based on biome density
        let chunk_area = chunk_size * chunk_size;
        let base_count = (chunk_area * biome_config.vegetation.density * 0.1) as usize; // 10% of vegetation density
        let count = base_count.clamp(1, 20); // Reasonable limits

        // Convert to BiomeRule for compatibility
        let biome = Biome::new(biome_config.biome_type, biome_config.clone());
        let biome_rule = biome.to_biome_rule();

        let resources = spawn_resources(
            seed + 1000, // Different seed offset for resources
            area_min,
            area_max,
            count,
            &biome_rule,
            None, // No weave consequence for now
        );

        Ok(resources)
    }
}

/// Combined scatter result containing vegetation, resources, and structures
#[derive(Debug, Clone)]
pub struct ScatterResult {
    pub vegetation: Vec<VegetationInstance>,
    pub resources: Vec<ResourceNode>,
    pub structures: Vec<crate::structures::StructureInstance>,
    pub chunk_id: ChunkId,
}

impl ScatterResult {
    /// Create a new scatter result
    pub fn new(chunk_id: ChunkId) -> Self {
        Self {
            vegetation: Vec::new(),
            resources: Vec::new(),
            structures: Vec::new(),
            chunk_id,
        }
    }

    /// Get total number of scattered objects
    pub fn total_count(&self) -> usize {
        self.vegetation.len() + self.resources.len()
    }

    /// Check if the scatter result is empty
    pub fn is_empty(&self) -> bool {
        self.vegetation.is_empty() && self.resources.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BiomeConfig, BiomeType, Heightmap, HeightmapConfig};

    #[test]
    fn test_vegetation_scatter_creation() {
        let config = ScatterConfig::default();
        let scatter = VegetationScatter::new(config);
        assert_eq!(scatter.config.min_distance, 2.0);
    }

    #[test]
    #[ignore = "slow test - skip for mutation testing"]
    fn test_scatter_generation() -> anyhow::Result<()> {
        let scatter = VegetationScatter::new(ScatterConfig::default());

        // Create a simple test chunk
        let chunk_id = ChunkId::new(0, 0);
        let heightmap_config = HeightmapConfig {
            resolution: 32,
            ..Default::default()
        };
        let heightmap = Heightmap::new(heightmap_config)?;
        let biome_map = vec![BiomeType::Grassland; 32 * 32];
        let chunk = TerrainChunk::new(chunk_id, heightmap, biome_map);

        let biome_config = BiomeConfig::grassland();
        let vegetation = scatter.scatter_vegetation(&chunk, 256.0, &biome_config, 12345)?;

        // Should generate some vegetation for grassland
        assert!(!vegetation.is_empty());

        Ok(())
    }

    #[test]
    fn test_resource_scattering() -> anyhow::Result<()> {
        let scatter = VegetationScatter::new(ScatterConfig::default());

        // Create a test chunk
        let chunk_id = ChunkId::new(0, 0);
        let heightmap_config = HeightmapConfig {
            resolution: 32,
            ..Default::default()
        };
        let heightmap = Heightmap::new(heightmap_config)?;
        let biome_map = vec![BiomeType::Forest; 32 * 32];
        let chunk = TerrainChunk::new(chunk_id, heightmap, biome_map);

        let biome_config = BiomeConfig::forest();
        let resources = scatter.scatter_resources(&chunk, 256.0, &biome_config, 12345)?;

        // Forest should have resources
        assert!(!resources.is_empty());

        Ok(())
    }

    #[test]
    fn test_slope_filtering() {
        let scatter = VegetationScatter::new(ScatterConfig {
            max_slope: 30.0,
            ..Default::default()
        });

        // Create chunk with varying heights
        let chunk_id = ChunkId::new(0, 0);
        let heightmap_config = HeightmapConfig {
            resolution: 16,
            ..Default::default()
        };
        let mut heightmap = Heightmap::new(heightmap_config).unwrap();

        // Create a steep slope
        for x in 0..16 {
            for z in 0..16 {
                heightmap.set_height(x, z, x as f32 * 10.0); // Very steep
            }
        }

        let biome_map = vec![BiomeType::Mountain; 16 * 16];
        let chunk = TerrainChunk::new(chunk_id, heightmap, biome_map);

        let test_pos = Vec3::new(64.0, 50.0, 64.0);
        let (slope, _normal) = scatter.estimate_slope_and_normal(&chunk, test_pos, 256.0);

        // Should detect steep slope
        assert!(slope > 30.0);
    }

    #[test]
    fn test_scatter_result() {
        let mut result = ScatterResult::new(ChunkId::new(0, 0));
        assert!(result.is_empty());
        assert_eq!(result.total_count(), 0);

        result.vegetation.push(VegetationInstance {
            position: Vec3::ZERO,
            rotation: 0.0,
            scale: 1.0,
            vegetation_type: "test".to_string(),
            model_path: "test.glb".to_string(),
            terrain_normal: Vec3::Y,
            tint: Vec3::ONE,
        });

        assert!(!result.is_empty());
        assert_eq!(result.total_count(), 1);
    }

    #[test]
    fn test_density_at_distance_lod0() {
        let lod = VegetationLodConfig::default();
        // Within LOD0 range (0..50) → full density
        assert_eq!(density_at_distance(0.0, &lod), 1.0);
        assert_eq!(density_at_distance(25.0, &lod), 1.0);
        assert_eq!(density_at_distance(49.9, &lod), 1.0);
    }

    #[test]
    fn test_density_at_distance_lod1() {
        let lod = VegetationLodConfig::default();
        // Within LOD1 range (50..150) → 50% density
        assert_eq!(density_at_distance(50.0, &lod), 0.5);
        assert_eq!(density_at_distance(100.0, &lod), 0.5);
        assert_eq!(density_at_distance(149.9, &lod), 0.5);
    }

    #[test]
    fn test_density_at_distance_lod2() {
        let lod = VegetationLodConfig::default();
        // Within LOD2 range (150..500) → 25% density
        assert_eq!(density_at_distance(150.0, &lod), 0.25);
        assert_eq!(density_at_distance(300.0, &lod), 0.25);
    }

    #[test]
    fn test_density_at_distance_lod3() {
        let lod = VegetationLodConfig::default();
        // Within LOD3 range (500..1000) → 12.5% density
        assert_eq!(density_at_distance(500.0, &lod), 0.125);
        assert_eq!(density_at_distance(750.0, &lod), 0.125);
    }

    #[test]
    fn test_density_at_distance_beyond_lod3() {
        let lod = VegetationLodConfig::default();
        // Beyond LOD3 but under cull_distance (1000..1500) → 6.25%
        assert_eq!(density_at_distance(1000.0, &lod), 0.0625);
        assert_eq!(density_at_distance(1200.0, &lod), 0.0625);
    }

    #[test]
    fn test_density_at_distance_culled() {
        let lod = VegetationLodConfig::default();
        // Beyond cull_distance → 0
        assert_eq!(density_at_distance(1500.1, &lod), 0.0);
        assert_eq!(density_at_distance(5000.0, &lod), 0.0);
    }

    #[test]
    fn test_density_at_distance_custom_config() {
        let lod = VegetationLodConfig {
            lod_distances: [10.0, 20.0, 30.0, 40.0],
            cull_distance: 50.0,
        };
        assert_eq!(density_at_distance(5.0, &lod), 1.0);
        assert_eq!(density_at_distance(15.0, &lod), 0.5);
        assert_eq!(density_at_distance(25.0, &lod), 0.25);
        assert_eq!(density_at_distance(35.0, &lod), 0.125);
        assert_eq!(density_at_distance(45.0, &lod), 0.0625);
        assert_eq!(density_at_distance(55.0, &lod), 0.0);
    }
}
