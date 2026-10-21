# Terrain Erosion Seamless Diagnostic — 2026-04-24

## Scope

Phase 1.6-F.3-phase-3.A: empirical measurements that quantify the two Andrew-gate visual failures from phase 2:

1. **Stitching artifacts** visible at chunk boundaries (phase 2 documented 15-40 world-unit divergence; this file records per-climate and explains the root cause mechanism).
2. **Mountain scale compression** — "short and thin" peaks (this file measures the pre-erosion vs post-erosion scale change per climate and classifies whether the issue is erosion-driven or source-noise-driven).

## Methodology

Test file: `astraweave-terrain/tests/phase_1_6_f3_phase_3_diagnostic.rs`.

Run command:

```
cargo test -p astraweave-terrain --release --test phase_1_6_f3_phase_3_diagnostic -- --nocapture
```

Configuration: seed 12345, `WorldConfig::default()` (F.2-T-4 noise + derivative-weighted fBm + continental modulation). Grid size 2×2 for stitching (two shared edges per axis = four internal edges; 64-vertex each = 256 samples per distribution). Single chunk (0,0) for scale; its noise field is representative of mid-amplitude terrain.

Pre-erosion measurements use `config.noise.erosion_enabled = false`, which bypasses `AdvancedErosionSimulator::apply_preset` entirely. Post-erosion uses the default pipeline.

## Findings 1 — Stitching divergence is hydraulic-only

### Data (2×2 grid, seed 12345, post-erosion)

| Climate   | mean | p50  | p95  | p99   | max   |
|-----------|-----:|-----:|-----:|------:|------:|
| Temperate | 1.66 | 0.74 | 7.10 | 12.62 | 14.82 |
| Cold      | 0.55 | 0.35 | 1.72 | 2.50  | 2.81  |
| Arid      | 0.00 | 0.00 | 0.00 | 0.00  | 0.00  |
| Tropical  | 1.46 | 0.69 | 7.01 | 12.27 | 13.22 |
| Wetland   | 1.46 | 0.69 | 7.01 | 12.27 | 13.22 |
| Highland  | 0.55 | 0.35 | 1.72 | 2.50  | 2.81  |

All units are world-space (vertical Y).

Pre-erosion: all columns are 0.000 (noise field is deterministic at world coords).

### Interpretation

- **Arid produces zero stitching** across all percentiles. Desert preset has no hydraulic pass (only thermal + wind). Thermal erosion iterates deterministically over the heightmap (no RNG); wind uses a fixed wind direction (no RNG). Therefore both are already world-coord-safe. The per-halo-seed divergence is purely a hydraulic problem.

- **Cold/Highland have less stitching than Temperate** (mean 0.55 vs 1.66). Counter-intuitive since mountain_balanced (50k droplets) is more aggressive than default_balanced (35k droplets). Likely explanation: mountain preset runs hydraulic FIRST then thermal. The subsequent thermal pass re-smooths and averages out the per-halo hydraulic divergence. Default_balanced runs thermal then hydraulic — no post-hydraulic smoothing, so per-halo divergence survives as the final output.

- **Tropical/Wetland are equivalent** because both map to `coastal`. Coastal runs thermal → hydraulic → wind; the post-hydraulic wind pass doesn't undo hydraulic divergence (wind erosion moves material in its own directional pattern).

- **Max divergence ≈ 15 world units for Temperate** matches Andrew-gate's visual observation. At 4-world-unit vertex spacing, this is ~3.7 units per neighboring vertex — slope change, visible as a seam ridge.

### Mechanism (confirmed from `advanced_erosion.rs:391-472`)

```rust
let mut rng = SimpleRng::new(self.seed);    // halo_seed(world_seed, target_chunk_id, halo_chunks)
for _droplet_idx in 0..config.droplet_count {
    let start_x = rng.next_float() * (resolution - 1) as f32;   // LOCAL halo coords
    let start_z = rng.next_float() * (resolution - 1) as f32;
    // ... simulate droplet ...
}
```

- `SimpleRng::new(self.seed)` is per-simulator. The simulator is constructed per `generate_chunk_with_climate` call with `halo_seed(world_seed, target_chunk_id, halo_chunks)`.
- Adjacent chunks have different `target_chunk_id` → different halo seeds → different RNG streams.
- `start_x`, `start_z` are LOCAL halo coordinates, not world coordinates. Halo A's local (0, 0) maps to world `(target_A - halo_size, target_A - halo_size)`; halo B's local (0, 0) maps to a DIFFERENT world position.
- Even if two halos happened to draw the same local random numbers, those numbers would correspond to different world positions.
- Therefore: adjacent halos have no correlation in droplet spawn positions or trajectories in the overlap region.

### Overlap divergence direct measurement

For the shared edge of chunks (0,0) and (1,0), the same world-coordinate column has:

- Erosion ENABLED: mean 1.03, max 5.84 world units divergence.
- Erosion DISABLED: mean 0.0, max 0.0 (floating-point precision).

This confirms erosion — specifically hydraulic erosion — is introducing all the divergence.

## Findings 2 — Scale compression is climate-dependent

### Data (single chunk (0,0), seed 12345)

| Climate   | pre.max | pre.p99 | pre.p95 | post.max | post.p99 | post.p95 | Δp99  |
|-----------|--------:|--------:|--------:|---------:|---------:|---------:|------:|
| Temperate |   87.93 |   80.85 |   69.55 |    76.04 |    68.53 |    54.24 | -15.2% |
| Cold      |   87.93 |   80.85 |   69.55 |    64.01 |    57.98 |    45.68 | -28.3% |
| Arid      |   87.93 |   80.85 |   69.55 |    78.96 |    75.20 |    64.81 |  -7.0% |
| Tropical  |   87.93 |   80.85 |   69.55 |    79.90 |    71.69 |    56.78 | -11.3% |
| Wetland   |   87.93 |   80.85 |   69.55 |    79.90 |    71.69 |    56.78 | -11.3% |
| Highland  |   87.93 |   80.85 |   69.55 |    64.01 |    57.98 |    45.68 | -28.3% |

### Interpretation

- **Pre-erosion max is 87.93 units** at chunk (0,0). Across the full 121-chunk grid, F.2-T-4 measured max 96.04 — peaks live in different chunks.

- **Cold/Highland show the most compression** (Δp99 -28.3%). This maps to `mountain_balanced` (50k droplets, erode_speed 0.4, talus 50°, 30 thermal iterations). Most aggressive erosion profile → biggest peak reduction.

- **Arid shows the least compression** (Δp99 -7.0%). Desert preset has no hydraulic, only thermal + wind. Limited peak reduction capability.

- **Temperate (-15.2%)** is moderate. `default_balanced` has 35k droplets + default thermal.

### Is Andrew-gate's "short and thin" erosion-driven?

**Yes, primarily for Cold/Highland (-28.3%).** That's a substantial peak reduction — a pre-erosion 81-unit p99 mountain becomes a 58-unit p99 after erosion. Against a baseline grass surface at Y ≈ 5-10, that's still a ~50-unit mountain, but ~28% shorter than F.2-T-4 produced.

For other climates (Temperate -15%, Tropical/Wetland -11%), the compression is less dramatic but still present.

**Task 4 decision logic:**

- After phase-3.C fixes stitching, re-measure scale with Andrew-gate visual check.
- If mountains still look short/thin on Cold/Highland: `mountain_balanced` is over-aggressive. Candidates: reduce droplet_count further (50k → 35k), raise talus_angle (50° → 55°), reduce erode_speed (0.4 → 0.3). One parameter change per preset max per `§10` discipline.
- If mountains look acceptable post-fix: the stitching was confounding visual perception. Decision documented, no further action.
- If mountains look bad even post-fix AND tuning doesn't help: document as aesthetic limitation (Appalachian-realism vs alpine-expectation). Consider for F.5 integration tuning.

## Recommendations for Task 3 (F.3-phase-3.C)

### What must change

- **Hydraulic erosion spawn loop** at `advanced_erosion.rs:406-411`: replace LOCAL-coordinate spawning with WORLD-coordinate spawning. Droplet positions derived deterministically from `(world_seed, global_droplet_index)`.

- **Per-droplet RNG**: currently the `SimpleRng` is single-stream through the entire hydraulic pass (each droplet's spawn consumes 2 RNG draws from the same stream). Phase-3 needs each droplet to have its OWN deterministic RNG state, keyed by world position or global index, so droplets are independent across halos.

### What does NOT need to change

- **Thermal erosion** (`apply_thermal_erosion`, `advanced_erosion.rs:482-560`): no RNG. Iterates deterministically over the heightmap. Already world-coord-safe (confirmed by Arid's 0 divergence).

- **Wind erosion** (`apply_wind_erosion`, `advanced_erosion.rs:564-618`): no RNG. Uses fixed wind direction. Already world-coord-safe.

- **Erosion brush precomputation** (`init_erosion_brush`): operates on LOCAL heightmap indices, but the brush weights are symmetric and the indices are computed from LOCAL positions — when a droplet erodes at local position (x, z), it affects local neighbors regardless of world coords. Same droplet at the same world position (executed by two adjacent halos) will do the same erosion pattern (same local offset from droplet's local position). No change needed.

### Proposed minimum-change API

Add to `AdvancedErosionSimulator`:

```rust
pub fn apply_preset_at_world_offset(
    &mut self,
    heightmap: &mut Heightmap,
    preset: &ErosionPreset,
    world_origin_x: f64,
    world_origin_z: f64,
    vertex_spacing: f64,
    world_seed: u64,
) -> ErosionStats;
```

`apply_preset` (the existing API) stays unchanged — it's used by phase-0 synthetic tests and should continue to work as-is (per-simulator-seed RNG, LOCAL spawn, no world coord awareness).

### Implementation sketch

```rust
pub fn apply_preset_at_world_offset(
    &mut self,
    heightmap: &mut Heightmap,
    preset: &ErosionPreset,
    world_origin_x: f64,
    world_origin_z: f64,
    vertex_spacing: f64,
    world_seed: u64,
) -> ErosionStats {
    // For hydraulic passes, use world-coord spawning.
    // For thermal/wind passes, reuse existing implementations (world-coord-safe).
    let mut combined = ErosionStats::default();
    for pass_name in &preset.pass_order {
        let pass_stats = match pass_name.as_str() {
            "hydraulic" => {
                if let Some(config) = &preset.hydraulic {
                    self.apply_hydraulic_erosion_world_coord(
                        heightmap, config,
                        world_origin_x, world_origin_z, vertex_spacing, world_seed,
                    )
                } else {
                    ErosionStats::default()
                }
            }
            "thermal" => {
                if let Some(config) = &preset.thermal {
                    self.apply_thermal_erosion(heightmap, config)
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
        // accumulate stats
    }
    combined
}

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

    // Halo's world-space extent.
    let halo_min_x = world_origin_x;
    let halo_min_z = world_origin_z;
    let halo_max_x = world_origin_x + (resolution - 1) as f64 * vertex_spacing;
    let halo_max_z = world_origin_z + (resolution - 1) as f64 * vertex_spacing;
    let halo_area = (halo_max_x - halo_min_x) * (halo_max_z - halo_min_z);

    // Target droplet count is interpreted as a world-space density:
    // droplets / (chunk_area). For halo=1 covering 9 chunks, that's 9×
    // the count. Each halo executes only droplets whose world positions
    // fall inside its extent — overlapping halos deterministically share
    // droplets in the overlap region.
    //
    // To deterministically generate droplets whose positions cover the
    // world, we iterate a spatial grid: each grid cell has one droplet
    // positioned by hash(world_seed, cell_x, cell_z). The grid cell size
    // is chosen so a one-chunk area (chunk_size × chunk_size) contains
    // exactly config.droplet_count droplets.
    //
    // Cell side = chunk_side / sqrt(droplet_count). For 256-world-unit
    // chunks and 35k droplets: cell_side ≈ 1.37 world units.
    let chunk_side = vertex_spacing * (resolution as f64 - 1.0) / 3.0; // halo=1 → 3 chunks per side
    let cell_side = chunk_side / (config.droplet_count as f64).sqrt();

    // Grid cells overlapping the halo extent.
    let first_cell_x = (halo_min_x / cell_side).floor() as i64;
    let last_cell_x = (halo_max_x / cell_side).ceil() as i64;
    let first_cell_z = (halo_min_z / cell_side).floor() as i64;
    let last_cell_z = (halo_max_z / cell_side).ceil() as i64;

    let mut stats = ErosionStats::default();
    let mut erosion_map = vec![0.0f32; (resolution * resolution) as usize];
    let mut total_lifetime = 0u64;
    let mut droplets_executed = 0u64;

    for cz in first_cell_z..=last_cell_z {
        for cx in first_cell_x..=last_cell_x {
            // Per-cell deterministic droplet: position = cell origin +
            // hash-jittered fraction, RNG seed = hash(world_seed, cx, cz).
            let cell_seed = hash_world_coords(world_seed, cx, cz);
            let jx = ((cell_seed.wrapping_mul(0x9E3779B97F4A7C15) >> 32) as f32 / u32::MAX as f32) as f64;
            let jz = ((cell_seed.wrapping_mul(0x85EBCA6BE11ECC0D) >> 32) as f32 / u32::MAX as f32) as f64;
            let world_spawn_x = cx as f64 * cell_side + jx * cell_side;
            let world_spawn_z = cz as f64 * cell_side + jz * cell_side;

            // Skip droplets outside halo extent.
            if world_spawn_x < halo_min_x || world_spawn_x >= halo_max_x
                || world_spawn_z < halo_min_z || world_spawn_z >= halo_max_z {
                continue;
            }

            // Convert world → local heightmap coords.
            let local_x = (world_spawn_x - halo_min_x) / vertex_spacing;
            let local_z = (world_spawn_z - halo_min_z) / vertex_spacing;

            // Simulate droplet with per-droplet RNG seeded from cell.
            let mut droplet_rng = SimpleRng::new(cell_seed);
            simulate_droplet(heightmap, &mut erosion_map, local_x as f32, local_z as f32,
                             config, &mut droplet_rng,
                             &self.erosion_brush_indices, &self.erosion_brush_weights,
                             &mut stats, &mut total_lifetime);
            droplets_executed += 1;
        }
    }

    stats.avg_droplet_lifetime = total_lifetime as f32 / droplets_executed.max(1) as f32;
    stats.erosion_map = Some(erosion_map);
    stats
}
```

The `simulate_droplet` inner function is extracted from the existing `apply_hydraulic_erosion` body (the for-lifetime loop) and parameterized by RNG + initial position.

### Expected behavior after phase-3.C

- Adjacent halos overlap in world space. The SAME set of cells in the overlap region appears in BOTH halos (different skip decisions outside their local extents, but the overlap cells are executed in BOTH).
- For each cell in overlap, both halos compute the SAME world spawn position (same cell_seed → same jitter) and the SAME initial RNG state.
- Heightmap state at droplet-run time DIFFERS between halos (because halo A's prior droplets have eroded differently than halo B's prior droplets would have in the same world position). This means trajectories WILL diverge somewhat in overlap — but the divergence should be small compared to phase-2's 15-unit problem.

### Potential issue: heightmap state dependency

Even with world-coord spawn positions, two adjacent halos have different heightmap states when a given overlap-region droplet runs. Halo A ran droplets 0..N_overlap_A before this one; halo B ran a different set 0..N_overlap_B. So erosion from prior droplets differs slightly, which affects the CURRENT droplet's trajectory.

Expected residual divergence: ≤ 1-2 world units (orders of magnitude better than 15). Task 3's test tolerance should be calibrated against empirical measurement; the ideal target is ≤ 1 world unit.

If measurements show ≥ 5 world units residual, the world-coord fix is incomplete — possibly requires two-pass execution (first pass: compute total droplet set from all adjacent halos; second pass: execute in deterministic order). That's significantly more complex and would need a §10 escalation.

## Summary

- **Stitching root cause confirmed**: hydraulic erosion's per-halo RNG produces different spawn positions in world space, thus different erosion in overlap regions. Thermal and wind erosion are already world-coord-safe.
- **Scale compression characterized per climate**: Cold/Highland -28%, Temperate -15%, Arid -7%. Most compression from `mountain_balanced`.
- **Minimum-change fix path**: add `apply_preset_at_world_offset` that replaces the hydraulic spawn loop with world-coord-deterministic spawning via spatial cell hashing. Thermal and wind passes reuse existing implementations unchanged.
- **Expected residual divergence after fix**: ≤ 1-2 world units (small; visually imperceptible). If measurement shows > 5, escalate.
- **Task 4 scale decision deferred to post-fix measurement** — cannot separate erosion-compression from stitching-confusion until stitching is fixed.

## Files produced

- `astraweave-terrain/tests/phase_1_6_f3_phase_3_diagnostic.rs` — 3 tests.
- This document.
