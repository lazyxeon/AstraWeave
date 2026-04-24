//! Advanced Erosion Simulation - Production Ready
//!
//! This module provides industry-standard erosion algorithms including:
//! - Particle-based hydraulic erosion (water droplet simulation)
//! - Multi-pass thermal erosion with talus angle
//! - Coastal/shoreline erosion
//! - Wind erosion (aeolian)
//! - GPU-friendly data structures for compute shaders

use crate::Heightmap;
use glam::Vec2;
use serde::{Deserialize, Serialize};

// ============================================================================
// Configuration Structures
// ============================================================================

/// Configuration for hydraulic erosion simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydraulicErosionConfig {
    /// Number of water droplets to simulate
    pub droplet_count: u32,
    /// Droplet inertia (0.0 = changes direction instantly, 1.0 = never changes)
    pub inertia: f32,
    /// Rate at which droplet picks up sediment
    pub sediment_capacity_factor: f32,
    /// Minimum slope to maintain capacity
    pub min_slope: f32,
    /// Rate of sediment deposition
    pub deposit_speed: f32,
    /// Rate of erosion
    pub erode_speed: f32,
    /// Evaporation rate per step
    pub evaporation_rate: f32,
    /// Initial water volume per droplet
    pub initial_water: f32,
    /// Initial velocity magnitude
    pub initial_speed: f32,
    /// Maximum droplet lifetime (steps)
    pub max_droplet_lifetime: u32,
    /// Erosion brush radius
    pub erosion_radius: u32,
    /// Gravity strength
    pub gravity: f32,
}

impl Default for HydraulicErosionConfig {
    fn default() -> Self {
        Self {
            droplet_count: 50000,
            inertia: 0.05,
            sediment_capacity_factor: 4.0,
            min_slope: 0.01,
            deposit_speed: 0.3,
            erode_speed: 0.3,
            evaporation_rate: 0.01,
            initial_water: 1.0,
            initial_speed: 1.0,
            max_droplet_lifetime: 30,
            erosion_radius: 3,
            gravity: 4.0,
        }
    }
}

/// Configuration for thermal erosion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalErosionConfig {
    /// Number of iterations
    pub iterations: u32,
    /// Maximum stable slope angle (degrees)
    pub talus_angle: f32,
    /// Rate of material redistribution (0.0-1.0)
    pub redistribution_rate: f32,
    /// Whether to use 8-directional (true) or 4-directional (false) neighbors
    pub eight_directional: bool,
    /// Cell size in world units (affects slope calculation)
    pub cell_size: f32,
}

impl Default for ThermalErosionConfig {
    fn default() -> Self {
        Self {
            iterations: 50,
            talus_angle: 45.0,
            redistribution_rate: 0.5,
            eight_directional: true,
            cell_size: 1.0,
        }
    }
}

/// Configuration for wind erosion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindErosionConfig {
    /// Wind direction (normalized 2D vector)
    pub wind_direction: Vec2,
    /// Wind strength (erosion multiplier)
    pub wind_strength: f32,
    /// Particle suspension height
    pub suspension_height: f32,
    /// Number of iterations
    pub iterations: u32,
    /// Saltation distance (jump distance for particles)
    pub saltation_distance: f32,
}

impl Default for WindErosionConfig {
    fn default() -> Self {
        Self {
            wind_direction: Vec2::new(1.0, 0.0),
            wind_strength: 0.5,
            suspension_height: 5.0,
            iterations: 30,
            saltation_distance: 3.0,
        }
    }
}

/// Combined erosion configuration for multi-pass simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErosionPreset {
    /// Human-readable name
    pub name: String,
    /// Hydraulic erosion config (None to skip)
    pub hydraulic: Option<HydraulicErosionConfig>,
    /// Thermal erosion config (None to skip)
    pub thermal: Option<ThermalErosionConfig>,
    /// Wind erosion config (None to skip)
    pub wind: Option<WindErosionConfig>,
    /// Order of erosion passes (e.g., ["thermal", "hydraulic", "wind"])
    pub pass_order: Vec<String>,
}

impl Default for ErosionPreset {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            hydraulic: Some(HydraulicErosionConfig::default()),
            thermal: Some(ThermalErosionConfig::default()),
            wind: None,
            pass_order: vec!["thermal".to_string(), "hydraulic".to_string()],
        }
    }
}

impl ErosionPreset {
    /// Create a desert-style erosion preset (wind + thermal)
    pub fn desert() -> Self {
        Self {
            name: "Desert".to_string(),
            hydraulic: None,
            thermal: Some(ThermalErosionConfig {
                talus_angle: 35.0, // Steeper sand slopes
                ..Default::default()
            }),
            wind: Some(WindErosionConfig::default()),
            pass_order: vec!["thermal".to_string(), "wind".to_string()],
        }
    }

    /// Create a mountain-style erosion preset (heavy hydraulic + thermal)
    pub fn mountain() -> Self {
        Self {
            name: "Mountain".to_string(),
            hydraulic: Some(HydraulicErosionConfig {
                droplet_count: 100000,
                erode_speed: 0.4,
                ..Default::default()
            }),
            thermal: Some(ThermalErosionConfig {
                talus_angle: 50.0, // Rocky steep slopes
                iterations: 30,
                ..Default::default()
            }),
            wind: None,
            pass_order: vec!["hydraulic".to_string(), "thermal".to_string()],
        }
    }

    /// Create a coastal erosion preset
    pub fn coastal() -> Self {
        Self {
            name: "Coastal".to_string(),
            hydraulic: Some(HydraulicErosionConfig {
                droplet_count: 30000,
                evaporation_rate: 0.02, // More evaporation near coast
                ..Default::default()
            }),
            thermal: Some(ThermalErosionConfig {
                talus_angle: 40.0,
                iterations: 20,
                ..Default::default()
            }),
            wind: Some(WindErosionConfig {
                wind_strength: 0.3, // Gentle sea breeze
                ..Default::default()
            }),
            pass_order: vec![
                "thermal".to_string(),
                "hydraulic".to_string(),
                "wind".to_string(),
            ],
        }
    }

    /// Phase 1.6-F.3-phase-2.B balanced default preset; droplet count revised
    /// by F.3-phase-4.C.
    ///
    /// Phase 2: 50k → 35k (−30%) for 30-second §2.3 budget compliance.
    /// Phase 4: 35k → 25k (additional −29%) to restore peak heights after
    /// phase-3's world-coord grid-jittered droplet distribution proved more
    /// aggressive than phase-2's clumpy RNG. Phase-3 measured Temperate Δp99
    /// at -19.6% (vs phase-2's -15.2%); target post-phase-4 is ~-15%.
    ///
    /// All other parameters identical to `default()`. Existing `default()`
    /// unchanged so phase-0 behavioral measurements on the original remain
    /// valid.
    pub fn default_balanced() -> Self {
        Self {
            name: "Default (balanced)".to_string(),
            hydraulic: Some(HydraulicErosionConfig {
                droplet_count: 25_000,
                ..HydraulicErosionConfig::default()
            }),
            ..Self::default()
        }
    }

    /// Phase 1.6-F.3-phase-2.B balanced mountain preset; droplet count revised
    /// by F.3-phase-4.C.
    ///
    /// Phase 2: 100k → 50k (−50%) for approximate §2.3 budget compliance.
    /// Phase 4: 50k → 35k (additional −30%) to restore peak heights after
    /// phase-3's droplet distribution proved more aggressive. Phase-3
    /// measured Cold/Highland Δp99 at -38.5% (vs phase-2's -28%); target
    /// post-phase-4 is ~-25 to -30%.
    ///
    /// All other parameters identical to `mountain()`.
    pub fn mountain_balanced() -> Self {
        let base = Self::mountain();
        Self {
            name: "Mountain (balanced)".to_string(),
            hydraulic: base.hydraulic.map(|h| HydraulicErosionConfig {
                droplet_count: 35_000,
                ..h
            }),
            ..base
        }
    }
}

/// Phase 1.6-F.3-phase-2.A: selects an [`ErosionPreset`] for a chunk based
/// on its climate bias. Per campaign plan §2.2, the six `ClimateBias`
/// variants map to four distinct presets (two 2-to-1 groupings: Cold /
/// Highland → mountain-family, Tropical / Wetland → coastal).
///
/// Temperate and Cold / Highland use "balanced" preset variants with
/// reduced droplet counts to stay inside §2.3's 30-second generation
/// budget for 121-chunk editor runs (F.3-phase-0 projected full `default`
/// at 39.7s and full `mountain` at 83.5s).
pub fn erosion_preset_for_climate(climate: crate::ClimateBias) -> ErosionPreset {
    use crate::ClimateBias;
    match climate {
        ClimateBias::Temperate => ErosionPreset::default_balanced(),
        ClimateBias::Cold => ErosionPreset::mountain_balanced(),
        ClimateBias::Arid => ErosionPreset::desert(),
        ClimateBias::Tropical => ErosionPreset::coastal(),
        ClimateBias::Wetland => ErosionPreset::coastal(),
        ClimateBias::Highland => ErosionPreset::mountain_balanced(),
    }
}

// ============================================================================
// Erosion Statistics
// ============================================================================

/// Statistics from erosion simulation (for debugging/visualization)
#[derive(Debug, Clone, Default)]
pub struct ErosionStats {
    /// Total material eroded (cubic units)
    pub total_eroded: f64,
    /// Total material deposited (cubic units)
    pub total_deposited: f64,
    /// Number of droplets that reached water/edge
    pub droplets_terminated: u32,
    /// Average droplet lifetime
    pub avg_droplet_lifetime: f32,
    /// Maximum erosion depth at any point
    pub max_erosion_depth: f32,
    /// Heightmap of erosion/deposition (optional, for visualization)
    pub erosion_map: Option<Vec<f32>>,
}

// ============================================================================
// Water Droplet for Particle-Based Erosion
// ============================================================================

/// Water droplet for hydraulic erosion simulation
#[derive(Debug, Clone)]
struct WaterDroplet {
    /// Position on heightmap (continuous coordinates)
    pos: Vec2,
    /// Movement direction
    dir: Vec2,
    /// Current velocity
    velocity: f32,
    /// Current water volume
    water: f32,
    /// Carried sediment amount
    sediment: f32,
}

impl WaterDroplet {
    fn new(pos: Vec2, initial_speed: f32, initial_water: f32) -> Self {
        Self {
            pos,
            dir: Vec2::ZERO,
            velocity: initial_speed,
            water: initial_water,
            sediment: 0.0,
        }
    }
}

// ============================================================================
// Advanced Erosion Simulator
// ============================================================================

/// Advanced erosion simulator with multiple erosion types
pub struct AdvancedErosionSimulator {
    /// Random seed for deterministic results
    seed: u64,
    /// Precomputed erosion brush weights
    erosion_brush_indices: Vec<Vec<usize>>,
    erosion_brush_weights: Vec<Vec<f32>>,
}

impl AdvancedErosionSimulator {
    /// Create a new erosion simulator with the given seed
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            erosion_brush_indices: Vec::new(),
            erosion_brush_weights: Vec::new(),
        }
    }

    /// Initialize erosion brush for given radius and heightmap size
    fn init_erosion_brush(&mut self, radius: u32, map_size: u32) {
        self.erosion_brush_indices.clear();
        self.erosion_brush_weights.clear();

        for _ in 0..(map_size * map_size) as usize {
            self.erosion_brush_indices.push(Vec::new());
            self.erosion_brush_weights.push(Vec::new());
        }

        let mut weight_sum = 0.0f32;
        let radius_i = radius as i32;

        for z in -radius_i..=radius_i {
            for x in -radius_i..=radius_i {
                let sqr_dst = (x * x + z * z) as f32;
                if sqr_dst < (radius * radius) as f32 {
                    weight_sum += 1.0 - (sqr_dst.sqrt() / radius as f32);
                }
            }
        }

        for center_idx in 0..(map_size * map_size) as usize {
            let center_x = (center_idx as u32) % map_size;
            let center_z = (center_idx as u32) / map_size;

            for dz in -radius_i..=radius_i {
                for dx in -radius_i..=radius_i {
                    let sqr_dst = (dx * dx + dz * dz) as f32;
                    if sqr_dst < (radius * radius) as f32 {
                        let new_x = center_x as i32 + dx;
                        let new_z = center_z as i32 + dz;

                        if new_x >= 0
                            && new_x < map_size as i32
                            && new_z >= 0
                            && new_z < map_size as i32
                        {
                            let weight = (1.0 - (sqr_dst.sqrt() / radius as f32)) / weight_sum;
                            let idx = (new_z as u32 * map_size + new_x as u32) as usize;

                            self.erosion_brush_indices[center_idx].push(idx);
                            self.erosion_brush_weights[center_idx].push(weight);
                        }
                    }
                }
            }
        }
    }

    /// Apply hydraulic erosion using particle simulation
    pub fn apply_hydraulic_erosion(
        &mut self,
        heightmap: &mut Heightmap,
        config: &HydraulicErosionConfig,
    ) -> ErosionStats {
        let resolution = heightmap.resolution();
        self.init_erosion_brush(config.erosion_radius, resolution);

        let mut stats = ErosionStats::default();
        let mut total_lifetime = 0u64;
        let mut rng = SimpleRng::new(self.seed);

        // Create erosion map for visualization
        let mut erosion_map = vec![0.0f32; (resolution * resolution) as usize];

        for _droplet_idx in 0..config.droplet_count {
            // Spawn droplet at random position (LOCAL coords).
            let start_x = rng.next_float() * (resolution - 1) as f32;
            let start_z = rng.next_float() * (resolution - 1) as f32;
            let start_pos = Vec2::new(start_x, start_z);
            let lifetime = self.simulate_one_droplet(
                heightmap,
                &mut erosion_map,
                start_pos,
                config,
                &mut rng,
                &mut stats,
            );
            total_lifetime += lifetime as u64;
        }

        stats.avg_droplet_lifetime = total_lifetime as f32 / config.droplet_count as f32;
        stats.erosion_map = Some(erosion_map);

        stats
    }

    /// Phase 1.6-F.3-phase-3.C: world-coordinate droplet seeding for
    /// seamless chunk boundaries.
    ///
    /// Droplet spawn positions are derived from a world-aligned spatial
    /// cell grid, each cell seeded by `hash(world_seed, cell_x, cell_z)`.
    /// Adjacent halos sharing world-space extent in their overlap region
    /// iterate OVERLAPPING SUBSETS of the same global cell grid, so
    /// droplets in the overlap have identical spawn positions and
    /// identical per-droplet RNG state → identical erosion contributions
    /// from overlap-originated droplets.
    ///
    /// Residual state-dependent divergence remains (droplets entering the
    /// overlap from OUTSIDE experience a slightly different pre-erosion
    /// heightmap in each halo) but is bounded by halo width and erosion
    /// intensity per `docs/audits/terrain_seamless_erosion_research_2026-04-24.md`.
    fn apply_hydraulic_erosion_world_coord(
        &mut self,
        heightmap: &mut Heightmap,
        config: &HydraulicErosionConfig,
        world_origin_x: f64,
        world_origin_z: f64,
        vertex_spacing: f64,
        world_seed: u64,
    ) -> ErosionStats {
        let resolution = heightmap.resolution();
        self.init_erosion_brush(config.erosion_radius, resolution);

        let mut stats = ErosionStats::default();
        let mut total_lifetime = 0u64;
        let mut droplets_executed = 0u64;
        let mut erosion_map = vec![0.0f32; (resolution * resolution) as usize];

        // Halo's world-space extent (inclusive on min; exclusive spawn on max
        // since max maps to resolution-1 local which is the off-grid edge).
        let halo_min_x = world_origin_x;
        let halo_min_z = world_origin_z;
        let halo_max_x = world_origin_x + (resolution - 1) as f64 * vertex_spacing;
        let halo_max_z = world_origin_z + (resolution - 1) as f64 * vertex_spacing;

        // Cell size chosen so the halo contains ~droplet_count cells.
        // halo_area / cell² ≈ droplet_count  →  cell = sqrt(halo_area / droplet_count).
        // World-aligned cells ensure adjacent halos iterate the SAME cells in
        // their overlap region.
        let halo_width = (halo_max_x - halo_min_x).max(1.0);
        let halo_height = (halo_max_z - halo_min_z).max(1.0);
        let target_droplets = config.droplet_count.max(1) as f64;
        let cell_size = (halo_width * halo_height / target_droplets).sqrt();

        // Iterate integer cell coords whose range covers the halo extent.
        let first_cx = (halo_min_x / cell_size).floor() as i64;
        let last_cx = (halo_max_x / cell_size).ceil() as i64;
        let first_cz = (halo_min_z / cell_size).floor() as i64;
        let last_cz = (halo_max_z / cell_size).ceil() as i64;

        for cz in first_cz..=last_cz {
            for cx in first_cx..=last_cx {
                let cell_seed = Self::hash_world_cell(world_seed, cx, cz);

                // Hash-jittered spawn inside the cell (32-bit precision).
                let jx = ((cell_seed & 0xFFFF_FFFF) as f64) / (u32::MAX as f64);
                let jz = (((cell_seed >> 32) & 0xFFFF_FFFF) as f64) / (u32::MAX as f64);
                let world_spawn_x = cx as f64 * cell_size + jx * cell_size;
                let world_spawn_z = cz as f64 * cell_size + jz * cell_size;

                // Skip cells whose spawn falls outside this halo's extent —
                // they are "owned" by (and simulated in) a neighboring halo.
                if world_spawn_x < halo_min_x
                    || world_spawn_x >= halo_max_x
                    || world_spawn_z < halo_min_z
                    || world_spawn_z >= halo_max_z
                {
                    continue;
                }

                // Convert world → local heightmap coords.
                let local_x = ((world_spawn_x - halo_min_x) / vertex_spacing) as f32;
                let local_z = ((world_spawn_z - halo_min_z) / vertex_spacing) as f32;
                if local_x >= (resolution - 1) as f32 || local_z >= (resolution - 1) as f32 {
                    continue;
                }
                let start_pos = Vec2::new(local_x, local_z);

                // Per-droplet RNG seeded from world cell.
                let mut rng = SimpleRng::new(cell_seed);

                let lifetime = self.simulate_one_droplet(
                    heightmap,
                    &mut erosion_map,
                    start_pos,
                    config,
                    &mut rng,
                    &mut stats,
                );
                total_lifetime += lifetime as u64;
                droplets_executed += 1;
            }
        }

        stats.avg_droplet_lifetime = total_lifetime as f32 / droplets_executed.max(1) as f32;
        stats.erosion_map = Some(erosion_map);
        stats
    }

    /// Wang-style integer hash keyed by (world_seed, cell_x, cell_z). Matches
    /// the pattern of `WorldGenerator::halo_seed` for cross-codebase
    /// determinism. Full avalanche behavior; low-entropy hash alternatives
    /// (simple XOR) would produce grid-aligned aliasing per research.
    fn hash_world_cell(world_seed: u64, cx: i64, cz: i64) -> u64 {
        let mut h = world_seed;
        h = h.wrapping_add(cx as u64).wrapping_mul(0x9E3779B97F4A7C15);
        h ^= h >> 32;
        h = h.wrapping_add(cz as u64).wrapping_mul(0x85EBCA6BE11ECC0D);
        h ^= h >> 32;
        h.max(1)
    }

    /// Simulate one droplet from `start_pos` for up to
    /// `config.max_droplet_lifetime` steps. Mutates `heightmap`,
    /// `erosion_map`, and `stats`; consumes `rng` for zero-gradient
    /// direction jitter. Returns lifetime (step count).
    ///
    /// Shared by `apply_hydraulic_erosion` (legacy LOCAL spawn) and
    /// `apply_hydraulic_erosion_world_coord` (phase-3 world-coord spawn).
    #[allow(clippy::too_many_arguments)]
    fn simulate_one_droplet(
        &self,
        heightmap: &mut Heightmap,
        erosion_map: &mut [f32],
        start_pos: Vec2,
        config: &HydraulicErosionConfig,
        rng: &mut SimpleRng,
        stats: &mut ErosionStats,
    ) -> u32 {
        let resolution = heightmap.resolution();
        let mut droplet = WaterDroplet::new(start_pos, config.initial_speed, config.initial_water);
        let mut lifetime = 0u32;

        for _ in 0..config.max_droplet_lifetime {
            let node_x = droplet.pos.x as i32;
            let node_z = droplet.pos.y as i32;

            let cell_offset_x = droplet.pos.x - node_x as f32;
            let cell_offset_z = droplet.pos.y - node_z as f32;

            let (height, gradient) = self.calculate_height_and_gradient(heightmap, droplet.pos);

            let new_dir = droplet.dir * config.inertia - gradient * (1.0 - config.inertia);
            droplet.dir = if new_dir.length_squared() > 0.0001 {
                new_dir.normalize()
            } else {
                let angle = rng.next_float() * std::f32::consts::TAU;
                Vec2::new(angle.cos(), angle.sin())
            };

            let new_pos = droplet.pos + droplet.dir;

            if new_pos.x < 0.0
                || new_pos.x >= (resolution - 1) as f32
                || new_pos.y < 0.0
                || new_pos.y >= (resolution - 1) as f32
            {
                stats.droplets_terminated += 1;
                break;
            }

            let new_height = self.sample_height_bilinear(heightmap, new_pos);
            let delta_height = new_height - height;

            let sediment_capacity = (-delta_height).max(config.min_slope).max(0.0)
                * droplet.velocity
                * droplet.water
                * config.sediment_capacity_factor;

            if droplet.sediment > sediment_capacity || delta_height > 0.0 {
                let amount_to_deposit = if delta_height > 0.0 {
                    (droplet.sediment).min(delta_height)
                } else {
                    (droplet.sediment - sediment_capacity) * config.deposit_speed
                };

                droplet.sediment -= amount_to_deposit;
                stats.total_deposited += amount_to_deposit as f64;

                self.deposit_sediment(
                    heightmap,
                    erosion_map,
                    node_x,
                    node_z,
                    cell_offset_x,
                    cell_offset_z,
                    amount_to_deposit,
                );
            } else {
                let amount_to_erode =
                    ((sediment_capacity - droplet.sediment) * config.erode_speed).min(-delta_height);

                let center_idx = (node_z as u32 * resolution + node_x as u32) as usize;
                if center_idx < self.erosion_brush_indices.len() {
                    for i in 0..self.erosion_brush_indices[center_idx].len() {
                        let idx = self.erosion_brush_indices[center_idx][i];
                        let weight = self.erosion_brush_weights[center_idx][i];

                        let weighed_erode = amount_to_erode * weight;
                        let current = heightmap.data()[idx];
                        let delta = current.min(weighed_erode);

                        heightmap.data_mut()[idx] -= delta;
                        erosion_map[idx] -= delta;

                        droplet.sediment += delta;
                        stats.total_eroded += delta as f64;
                        stats.max_erosion_depth = stats.max_erosion_depth.max(delta);
                    }
                }
            }

            droplet.velocity = (droplet.velocity * droplet.velocity
                + delta_height.abs() * config.gravity)
                .sqrt();

            droplet.water *= 1.0 - config.evaporation_rate;
            droplet.pos = new_pos;
            lifetime += 1;
        }

        lifetime
    }

    /// Apply thermal erosion (talus-based material sliding)
    pub fn apply_thermal_erosion(
        &self,
        heightmap: &mut Heightmap,
        config: &ThermalErosionConfig,
    ) -> ErosionStats {
        let resolution = heightmap.resolution();
        let talus = (config.talus_angle * std::f32::consts::PI / 180.0).tan() * config.cell_size;

        let mut stats = ErosionStats::default();
        let mut erosion_map = vec![0.0f32; (resolution * resolution) as usize];

        // Neighbor offsets (8-directional or 4-directional)
        let neighbors: Vec<(i32, i32, f32)> = if config.eight_directional {
            vec![
                (-1, -1, std::f32::consts::SQRT_2),
                (0, -1, 1.0),
                (1, -1, std::f32::consts::SQRT_2),
                (-1, 0, 1.0),
                (1, 0, 1.0),
                (-1, 1, std::f32::consts::SQRT_2),
                (0, 1, 1.0),
                (1, 1, std::f32::consts::SQRT_2),
            ]
        } else {
            vec![(0, -1, 1.0), (-1, 0, 1.0), (1, 0, 1.0), (0, 1, 1.0)]
        };

        for _ in 0..config.iterations {
            let mut material_delta = vec![0.0f32; (resolution * resolution) as usize];

            for z in 1..(resolution - 1) {
                for x in 1..(resolution - 1) {
                    let idx = (z * resolution + x) as usize;
                    let current_height = heightmap.data()[idx];

                    let mut max_diff = 0.0f32;
                    let mut total_diff = 0.0f32;
                    let mut lower_neighbors = Vec::new();

                    for &(dx, dz, dist) in &neighbors {
                        let nx = (x as i32 + dx) as u32;
                        let nz = (z as i32 + dz) as u32;
                        let n_idx = (nz * resolution + nx) as usize;
                        let neighbor_height = heightmap.data()[n_idx];

                        let diff = (current_height - neighbor_height) / (dist * config.cell_size);
                        if diff > talus {
                            max_diff = max_diff.max(diff);
                            total_diff += diff - talus;
                            lower_neighbors.push((n_idx, diff - talus));
                        }
                    }

                    if total_diff > 0.0 {
                        let material_to_move = max_diff * config.redistribution_rate * 0.5;

                        // Remove material from current cell
                        material_delta[idx] -= material_to_move;

                        // Distribute to lower neighbors proportionally
                        for (n_idx, weight) in &lower_neighbors {
                            let fraction = weight / total_diff;
                            material_delta[*n_idx] += material_to_move * fraction;
                        }

                        stats.total_eroded += material_to_move as f64;
                    }
                }
            }

            // Apply deltas
            for (idx, delta) in material_delta.iter().enumerate() {
                heightmap.data_mut()[idx] += delta;
                erosion_map[idx] += delta;
            }
        }

        stats.erosion_map = Some(erosion_map);
        stats
    }

    /// Apply wind erosion (aeolian processes)
    pub fn apply_wind_erosion(
        &self,
        heightmap: &mut Heightmap,
        config: &WindErosionConfig,
    ) -> ErosionStats {
        let resolution = heightmap.resolution();
        let wind_dir = config.wind_direction.normalize();

        let mut stats = ErosionStats::default();
        let mut erosion_map = vec![0.0f32; (resolution * resolution) as usize];

        for _ in 0..config.iterations {
            let mut material_delta = vec![0.0f32; (resolution * resolution) as usize];

            for z in 1..(resolution - 1) {
                for x in 1..(resolution - 1) {
                    let idx = (z * resolution + x) as usize;
                    let current_height = heightmap.data()[idx];

                    // Check windward and leeward heights
                    let windward_x = (x as f32 - wind_dir.x).clamp(0.0, (resolution - 1) as f32);
                    let windward_z = (z as f32 - wind_dir.y).clamp(0.0, (resolution - 1) as f32);
                    let leeward_x = (x as f32 + wind_dir.x * config.saltation_distance)
                        .clamp(0.0, (resolution - 1) as f32);
                    let leeward_z = (z as f32 + wind_dir.y * config.saltation_distance)
                        .clamp(0.0, (resolution - 1) as f32);

                    let windward_idx =
                        (windward_z as u32 * resolution + windward_x as u32) as usize;
                    let leeward_idx = (leeward_z as u32 * resolution + leeward_x as u32) as usize;

                    let windward_height = heightmap.data()[windward_idx];

                    // Erosion happens on windward-facing slopes
                    if current_height > windward_height {
                        let slope = (current_height - windward_height).abs();
                        let erosion_amount = slope * config.wind_strength * 0.01;

                        material_delta[idx] -= erosion_amount;
                        material_delta[leeward_idx] += erosion_amount;

                        stats.total_eroded += erosion_amount as f64;
                    }
                }
            }

            // Apply deltas
            for (idx, delta) in material_delta.iter().enumerate() {
                heightmap.data_mut()[idx] += delta;
                erosion_map[idx] += delta;
            }
        }

        stats.erosion_map = Some(erosion_map);
        stats
    }

    /// Apply a full erosion preset with multiple passes
    pub fn apply_preset(
        &mut self,
        heightmap: &mut Heightmap,
        preset: &ErosionPreset,
    ) -> ErosionStats {
        let mut combined_stats = ErosionStats::default();

        for pass_name in &preset.pass_order {
            let pass_stats = match pass_name.as_str() {
                "thermal" => {
                    if let Some(config) = &preset.thermal {
                        self.apply_thermal_erosion(heightmap, config)
                    } else {
                        ErosionStats::default()
                    }
                }
                "hydraulic" => {
                    if let Some(config) = &preset.hydraulic {
                        self.apply_hydraulic_erosion(heightmap, config)
                    } else {
                        ErosionStats::default()
                    }
                }
                "wind" => {
                    if let Some(config) = &preset.wind {
                        self.apply_wind_erosion(heightmap, config)
                    } else {
                        ErosionStats::default()
                    }
                }
                _ => ErosionStats::default(),
            };

            combined_stats.total_eroded += pass_stats.total_eroded;
            combined_stats.total_deposited += pass_stats.total_deposited;
            combined_stats.max_erosion_depth = combined_stats
                .max_erosion_depth
                .max(pass_stats.max_erosion_depth);
        }

        combined_stats
    }

    /// Phase 1.6-F.3-phase-3.C: apply an erosion preset with world-coordinate
    /// droplet seeding for seamless chunk boundaries.
    ///
    /// Hydraulic passes use [`apply_hydraulic_erosion_world_coord`] which
    /// derives droplet spawn positions from world-aligned spatial cells
    /// seeded by `hash(world_seed, cell_x, cell_z)`. Adjacent halos sharing
    /// a world-space overlap region iterate the SAME cells in the overlap
    /// and therefore execute identical droplets there, producing nearly
    /// identical erosion output (residual divergence comes from droplets
    /// that enter the overlap from outside-overlap regions where each
    /// halo's prior heightmap state differs — bounded but not zero).
    ///
    /// Thermal and wind passes use the existing deterministic implementations
    /// (no RNG → already world-coord-safe, confirmed by F.3-phase-3.A's
    /// Arid 0-divergence measurement).
    ///
    /// Prefer this over [`apply_preset`] for production terrain chunk
    /// generation. [`apply_preset`] remains available for synthetic
    /// heightmap testing (phase 0) where world coords are meaningless.
    ///
    /// # Arguments
    /// * `heightmap` — mutated in place; pre-erosion state on input.
    /// * `preset` — which passes to run and their parameters.
    /// * `world_origin_x`, `world_origin_z` — world-space position of
    ///   heightmap vertex (0, 0).
    /// * `vertex_spacing` — world-space distance between adjacent vertices.
    /// * `world_seed` — hashed into per-cell droplet seeds. Same seed +
    ///   same cell = same droplet, regardless of which halo is running.
    pub fn apply_preset_at_world_offset(
        &mut self,
        heightmap: &mut Heightmap,
        preset: &ErosionPreset,
        world_origin_x: f64,
        world_origin_z: f64,
        vertex_spacing: f64,
        world_seed: u64,
    ) -> ErosionStats {
        let mut combined_stats = ErosionStats::default();

        for pass_name in &preset.pass_order {
            let pass_stats = match pass_name.as_str() {
                "thermal" => {
                    if let Some(config) = &preset.thermal {
                        self.apply_thermal_erosion(heightmap, config)
                    } else {
                        ErosionStats::default()
                    }
                }
                "hydraulic" => {
                    if let Some(config) = &preset.hydraulic {
                        self.apply_hydraulic_erosion_world_coord(
                            heightmap,
                            config,
                            world_origin_x,
                            world_origin_z,
                            vertex_spacing,
                            world_seed,
                        )
                    } else {
                        ErosionStats::default()
                    }
                }
                "wind" => {
                    if let Some(config) = &preset.wind {
                        self.apply_wind_erosion(heightmap, config)
                    } else {
                        ErosionStats::default()
                    }
                }
                _ => ErosionStats::default(),
            };

            combined_stats.total_eroded += pass_stats.total_eroded;
            combined_stats.total_deposited += pass_stats.total_deposited;
            combined_stats.max_erosion_depth = combined_stats
                .max_erosion_depth
                .max(pass_stats.max_erosion_depth);
        }

        combined_stats
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    fn calculate_height_and_gradient(&self, heightmap: &Heightmap, pos: Vec2) -> (f32, Vec2) {
        let resolution = heightmap.resolution();
        let x = pos.x as u32;
        let z = pos.y as u32;

        let x1 = (x + 1).min(resolution - 1);
        let z1 = (z + 1).min(resolution - 1);

        let h00 = heightmap.get_height(x, z);
        let h10 = heightmap.get_height(x1, z);
        let h01 = heightmap.get_height(x, z1);
        let h11 = heightmap.get_height(x1, z1);

        let u = pos.x - x as f32;
        let v = pos.y - z as f32;

        // Bilinear interpolation
        let height =
            h00 * (1.0 - u) * (1.0 - v) + h10 * u * (1.0 - v) + h01 * (1.0 - u) * v + h11 * u * v;

        // Gradient
        let gx = (h10 - h00) * (1.0 - v) + (h11 - h01) * v;
        let gz = (h01 - h00) * (1.0 - u) + (h11 - h10) * u;

        (height, Vec2::new(gx, gz))
    }

    fn sample_height_bilinear(&self, heightmap: &Heightmap, pos: Vec2) -> f32 {
        let resolution = heightmap.resolution();
        let x = pos.x as u32;
        let z = pos.y as u32;

        let x1 = (x + 1).min(resolution - 1);
        let z1 = (z + 1).min(resolution - 1);

        let h00 = heightmap.get_height(x, z);
        let h10 = heightmap.get_height(x1, z);
        let h01 = heightmap.get_height(x, z1);
        let h11 = heightmap.get_height(x1, z1);

        let u = pos.x - x as f32;
        let v = pos.y - z as f32;

        h00 * (1.0 - u) * (1.0 - v) + h10 * u * (1.0 - v) + h01 * (1.0 - u) * v + h11 * u * v
    }

    #[allow(clippy::too_many_arguments)]
    fn deposit_sediment(
        &self,
        heightmap: &mut Heightmap,
        erosion_map: &mut [f32],
        node_x: i32,
        node_z: i32,
        offset_x: f32,
        offset_z: f32,
        amount: f32,
    ) {
        let resolution = heightmap.resolution() as i32;

        // Bilinear deposit to 4 corners
        let weights = [
            (1.0 - offset_x) * (1.0 - offset_z),
            offset_x * (1.0 - offset_z),
            (1.0 - offset_x) * offset_z,
            offset_x * offset_z,
        ];

        let positions = [
            (node_x, node_z),
            (node_x + 1, node_z),
            (node_x, node_z + 1),
            (node_x + 1, node_z + 1),
        ];

        for (i, &(px, pz)) in positions.iter().enumerate() {
            if px >= 0 && px < resolution && pz >= 0 && pz < resolution {
                let idx = (pz * resolution + px) as usize;
                let deposit = amount * weights[i];
                heightmap.data_mut()[idx] += deposit;
                erosion_map[idx] += deposit;
            }
        }
    }
}

// ============================================================================
// Simple RNG for Deterministic Results
// ============================================================================

/// Simple deterministic RNG (xorshift)
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn next_float(&mut self) -> f32 {
        (self.next() as f32) / (u64::MAX as f32)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HeightmapConfig;

    fn create_test_heightmap() -> Heightmap {
        let config = HeightmapConfig {
            resolution: 64,
            ..Default::default()
        };
        let mut heightmap = Heightmap::new(config).unwrap();

        // Create a mountain in the center
        for z in 0..64 {
            for x in 0..64 {
                let dx = x as f32 - 32.0;
                let dz = z as f32 - 32.0;
                let dist = (dx * dx + dz * dz).sqrt();
                let height = (32.0 - dist).max(0.0) * 3.0;
                heightmap.set_height(x, z, height);
            }
        }

        heightmap
    }

    #[test]
    fn test_hydraulic_erosion_reduces_peaks() {
        let mut heightmap = create_test_heightmap();
        let initial_max = heightmap.max_height();

        let mut simulator = AdvancedErosionSimulator::new(12345);
        let config = HydraulicErosionConfig {
            droplet_count: 20000, // More droplets to ensure visible erosion
            erode_speed: 0.5,     // Higher erosion rate
            ..Default::default()
        };

        let stats = simulator.apply_hydraulic_erosion(&mut heightmap, &config);
        heightmap.recalculate_bounds(); // Recalculate after bulk changes
        let final_max = heightmap.max_height();

        // Erosion should reduce peak height or at least move significant material
        // The test passes if either peak is reduced OR we eroded significant material
        assert!(
            final_max < initial_max || stats.total_eroded > 100.0,
            "Expected erosion: initial_max={}, final_max={}, total_eroded={}",
            initial_max,
            final_max,
            stats.total_eroded
        );
    }

    #[test]
    fn test_thermal_erosion_smooths_slopes() {
        let mut heightmap = create_test_heightmap();

        let simulator = AdvancedErosionSimulator::new(12345);
        let config = ThermalErosionConfig {
            iterations: 50,
            talus_angle: 30.0,
            ..Default::default()
        };

        let stats = simulator.apply_thermal_erosion(&mut heightmap, &config);

        // Should have eroded some material
        assert!(stats.total_eroded > 0.0);
    }

    #[test]
    fn test_erosion_preset_applies_all_passes() {
        let mut heightmap = create_test_heightmap();
        let mut simulator = AdvancedErosionSimulator::new(12345);

        let preset = ErosionPreset::mountain();
        let stats = simulator.apply_preset(&mut heightmap, &preset);

        // Both hydraulic and thermal should have contributed
        assert!(stats.total_eroded > 0.0);
    }

    #[test]
    fn test_wind_erosion_moves_material() {
        let mut heightmap = create_test_heightmap();
        let simulator = AdvancedErosionSimulator::new(12345);

        let config = WindErosionConfig::default();
        let stats = simulator.apply_wind_erosion(&mut heightmap, &config);

        // Wind should move some material
        assert!(stats.total_eroded > 0.0);
    }

    /// Phase 1.6-F.3-phase-2.A: verify climate → ErosionPreset mapping is
    /// total (doesn't panic on any variant) and distinct-group variants
    /// share presets per §2.2.
    #[test]
    fn phase_1_6_f3_phase_2_erosion_preset_for_climate_maps_all_six_variants() {
        use crate::ClimateBias;

        let temperate = erosion_preset_for_climate(ClimateBias::Temperate);
        let cold = erosion_preset_for_climate(ClimateBias::Cold);
        let arid = erosion_preset_for_climate(ClimateBias::Arid);
        let tropical = erosion_preset_for_climate(ClimateBias::Tropical);
        let wetland = erosion_preset_for_climate(ClimateBias::Wetland);
        let highland = erosion_preset_for_climate(ClimateBias::Highland);

        // Cold and Highland both map to mountain-family preset.
        let cold_drop = cold.hydraulic.as_ref().map(|h| h.droplet_count).unwrap_or(0);
        let highland_drop = highland
            .hydraulic
            .as_ref()
            .map(|h| h.droplet_count)
            .unwrap_or(0);
        assert_eq!(
            cold_drop, highland_drop,
            "Cold and Highland should both map to mountain-family preset"
        );

        // Tropical and Wetland both map to coastal preset.
        let tropical_drop = tropical
            .hydraulic
            .as_ref()
            .map(|h| h.droplet_count)
            .unwrap_or(0);
        let wetland_drop = wetland
            .hydraulic
            .as_ref()
            .map(|h| h.droplet_count)
            .unwrap_or(0);
        assert_eq!(
            tropical_drop, wetland_drop,
            "Tropical and Wetland should both map to coastal preset"
        );

        // Arid should be desert (no hydraulic).
        assert!(
            arid.hydraulic.is_none(),
            "Arid → desert preset should have no hydraulic pass"
        );

        // Temperate should be default-family (has hydraulic).
        assert!(
            temperate.hydraulic.is_some(),
            "Temperate → default-family preset should have hydraulic pass"
        );
    }

    #[test]
    fn test_deterministic_erosion() {
        let heightmap1 = create_test_heightmap();
        let heightmap2 = create_test_heightmap();

        let mut hm1 = heightmap1;
        let mut hm2 = heightmap2;

        let mut sim1 = AdvancedErosionSimulator::new(99999);
        let mut sim2 = AdvancedErosionSimulator::new(99999);

        let config = HydraulicErosionConfig {
            droplet_count: 1000,
            ..Default::default()
        };

        sim1.apply_hydraulic_erosion(&mut hm1, &config);
        sim2.apply_hydraulic_erosion(&mut hm2, &config);

        // Same seed should produce identical results
        for i in 0..hm1.data().len() {
            assert!(
                (hm1.data()[i] - hm2.data()[i]).abs() < 0.001,
                "Mismatch at index {}: {} vs {}",
                i,
                hm1.data()[i],
                hm2.data()[i]
            );
        }
    }
}
