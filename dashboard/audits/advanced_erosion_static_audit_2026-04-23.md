# AdvancedErosionSimulator Static Audit — 2026-04-23

**Scope:** Phase 1.6-F.3-phase-0.A: static code review of `astraweave-terrain/src/advanced_erosion.rs` (902 lines) paired with literature grounding. Determines whether the simulator matches canonical particle-based hydraulic erosion implementations and identifies any divergences or apparent bugs worth behavioral testing in phase 0.B.

**Method:** full read of `advanced_erosion.rs`; cross-reference against Sebastian Lague's widely-copied C# implementation (pedagogical), the Hans Beyer 2015 article (original particle-based formulation), and dandrino/terrain-erosion-3-ways (community-canonical Rust-adjacent reference).

---

## Module structure

### Top-level structs

- `HydraulicErosionConfig` — droplet simulation parameters (count, inertia, capacity, rates, lifetime, gravity, brush radius).
- `ThermalErosionConfig` — talus-angle-based redistribution (iterations, angle, rate, 4/8-directional, cell size).
- `WindErosionConfig` — aeolian simulation (direction, strength, suspension, saltation).
- `ErosionPreset` — named container bundling optional hydraulic/thermal/wind configs plus an explicit `pass_order` vec of string names.
- `ErosionStats` — return value (total eroded, total deposited, termination count, avg lifetime, max depth, optional erosion map).
- `WaterDroplet` (private) — position, direction, velocity, water, sediment.
- `AdvancedErosionSimulator` — holds seed + precomputed erosion brush.
- `SimpleRng` (private) — xorshift PRNG.

### Public functions

- `AdvancedErosionSimulator::new(seed: u64) -> Self` — constructor.
- `AdvancedErosionSimulator::apply_hydraulic_erosion(&mut self, &mut Heightmap, &HydraulicErosionConfig) -> ErosionStats`
- `AdvancedErosionSimulator::apply_thermal_erosion(&self, &mut Heightmap, &ThermalErosionConfig) -> ErosionStats` (note: `&self`, not `&mut self`)
- `AdvancedErosionSimulator::apply_wind_erosion(&self, &mut Heightmap, &WindErosionConfig) -> ErosionStats`
- `AdvancedErosionSimulator::apply_preset(&mut self, &mut Heightmap, &ErosionPreset) -> ErosionStats` — the main entry F.3 will wire.
- `ErosionPreset::default()` / `desert()` / `mountain()` / `coastal()` — named preset constructors (four of them, not five — §2.2 maps six climates to these four).

### Private helpers

- `init_erosion_brush(radius, map_size)` — precomputes per-cell neighbour-index + weight vectors for radial erosion distribution.
- `calculate_height_and_gradient(heightmap, pos) -> (f32, Vec2)` — bilinear sample + bilinear gradient.
- `sample_height_bilinear(heightmap, pos) -> f32` — duplicate of the height half of the above; could be DRY'd.
- `deposit_sediment(heightmap, erosion_map, node_x, node_z, offset_x, offset_z, amount)` — bilinear deposit to 4 cell corners.

---

## Public API surface (relevant for F.3-phase-2)

```rust
impl AdvancedErosionSimulator {
    pub fn new(seed: u64) -> Self;
    pub fn apply_preset(&mut self, heightmap: &mut Heightmap, preset: &ErosionPreset) -> ErosionStats;
}

impl ErosionPreset {
    pub fn default() -> Self;
    pub fn desert() -> Self;
    pub fn mountain() -> Self;
    pub fn coastal() -> Self;
}
```

- Takes `&mut Heightmap` (the crate's Heightmap type), not a raw `&mut [f32]`. Phase 2's wiring will access the chunk's `heightmap` field directly.
- `apply_preset` takes preset by reference — consumers can reuse the preset across calls.
- Returns `ErosionStats` for introspection/logging; safely ignorable.

**API is stable.** Phase 2 can consume it as-is.

---

## Hydraulic erosion algorithm

### Droplet initialization (lines 349-358)

```rust
let start_x = rng.next_float() * (resolution - 1) as f32;
let start_z = rng.next_float() * (resolution - 1) as f32;
```

Droplets spawn at uniformly-random positions across the heightmap. Initial velocity = `config.initial_speed` (default 1.0), direction zero, water `config.initial_water` (default 1.0), sediment 0.

**Compare to Lague:** matches (uniform random spawn). Dandrino also uses uniform random. OK.

### Velocity / direction update (lines 370-394)

```rust
let (height, gradient) = calculate_height_and_gradient(heightmap, pos);
let new_dir = droplet.dir * inertia - gradient * (1.0 - inertia);
droplet.dir = new_dir.normalize();  // if non-zero; else random
let new_pos = droplet.pos + droplet.dir;
```

Direction update combines old direction (weighted by inertia) with the downhill gradient (weighted by 1-inertia). Inertia default 0.05 — low retention of previous direction, so droplets closely follow the gradient.

**Compare to Lague:** direction update formula is `dirX = dirX * inertia - gradientX * (1 - inertia)` — matches.

### Sediment capacity (lines 400-404)

```rust
let sediment_capacity = (-delta_height).max(config.min_slope).max(0.0)
    * droplet.velocity
    * droplet.water
    * config.sediment_capacity_factor;
```

Capacity = `max(-Δh, min_slope) × velocity × water × factor`.

- `-Δh` is positive when going downhill. Max with `min_slope=0.01` means even on flat terrain, droplets carry a tiny amount.
- `.max(0.0)` after the preceding `.max(min_slope)` is redundant (since `min_slope=0.01 > 0`).

**Compare to Lague:** `capacity = max(-deltaHeight, minSlope) * speed * water * capacityFactor` — matches exactly.

### Deposit / erode branch (lines 406-453)

```rust
if droplet.sediment > sediment_capacity || delta_height > 0.0 {
    // Deposit
    let amount_to_deposit = if delta_height > 0.0 {
        droplet.sediment.min(delta_height)  // fill the pit
    } else {
        (droplet.sediment - sediment_capacity) * deposit_speed  // overflow
    };
    // ...
} else {
    // Erode
    let amount_to_erode = ((sediment_capacity - droplet.sediment) * erode_speed)
        .min(-delta_height);  // don't erode past delta
    // apply via brush to neighbours
}
```

When `Δh > 0` (going uphill, impossible per gradient-following but may happen near ridges): deposit at least enough to fill the uphill step. When sediment exceeds capacity: shed the overflow. Otherwise: erode, capped at `-Δh` so we don't erode past the downhill step.

**Compare to Lague:** closely matches. Key deviation — Lague's erosion cap is `(sediment_capacity - sediment) * erodeSpeed` without the `.min(-delta_height)` clamp. AstraWeave's cap makes erosion more conservative near shallow slopes. Minor divergence; slightly less aggressive.

### Brush-based erosion (lines 434-452)

```rust
let center_idx = (node_z as u32 * resolution + node_x as u32) as usize;
if center_idx < self.erosion_brush_indices.len() {
    for i in 0..self.erosion_brush_indices[center_idx].len() {
        let idx = self.erosion_brush_indices[center_idx][i];
        let weight = self.erosion_brush_weights[center_idx][i];
        let weighed_erode = amount_to_erode * weight;
        let current = heightmap.data()[idx];
        let delta = current.min(weighed_erode);
        heightmap.data_mut()[idx] -= delta;
        // ...
        droplet.sediment += delta;
    }
}
```

Precomputed radial brush distributes erosion over a disc of radius `erosion_radius` (default 3). `delta = current.min(weighed_erode)` prevents height going negative. **Heights are CLAMPED TO ZERO, not CLAMPED TO ORIGINAL** — this means erosion over many passes can reduce all terrain to zero. For AstraWeave's use case (post-F.2 heights are always ≥ 0 due to `.max(0.0)` in `sample_height`), this is fine but worth noting.

**Compare to Lague:** same pattern (brush-based erosion with per-cell weighting). Matches.

### Velocity update (lines 455-458) — **SUSPECTED BUG**

```rust
droplet.velocity = (droplet.velocity * droplet.velocity
    + delta_height.abs() * config.gravity)
    .sqrt();
```

Uses `delta_height.abs()`. Standard kinetic-energy update: KE_new = KE_old + gravity × drop_height (where drop_height = |Δh| when going down, -|Δh| when going up). With `.abs()`, droplets going UPHILL also gain velocity, which is physically wrong.

**Compare to Lague:**
```csharp
speed = Math.Sqrt(Math.Max(0, speed*speed + deltaHeight * gravity));
```

Lague's `deltaHeight` is `newHeight - height` (same sign convention as AstraWeave). When `deltaHeight < 0` (downhill), `speed*speed + deltaHeight*gravity` decreases → but `Max(0, ...)` prevents negative. So in Lague's convention **going downhill REDUCES speed (?!)**. That's because Lague inverts the sign of gravity implicitly via his sign conventions — in his coordinate system, height decreasing is the "expected motion direction" and gravity×deltaHeight being negative represents kinetic energy "being used up" to descend.

Actually, looking at Lague's full loop: he's modeling velocity as a momentum variable updated by `deltaHeight * gravity` (not `-deltaHeight * gravity`). The expectation is that velocity DOES decrease going downhill in his formulation, because his downhill movement already "spent" the height delta. Physically weird, but consistent within his model.

**Verdict:** AstraWeave's `.abs()` differs from Lague's non-abs formula. Whether this is a bug depends on the intended physics. Will be tested in behavioral audit (§2) — if droplets travel unreasonable distances or gain velocity on flat terrain, this is the cause.

### Termination (lines 386-394, 461-464)

Droplet terminates when:
1. New position leaves `[0, resolution-1]` bounds.
2. `max_droplet_lifetime` steps (default 30) reached.

Water evaporates at `evaporation_rate` per step (default 0.01 → ~22% remaining after 30 steps). **Droplet doesn't explicitly terminate when water hits zero** — it continues until lifetime runs out. This is a minor divergence from Lague's "terminate when water < threshold."

---

## Thermal erosion algorithm (lines 476-555)

### Structure

Iteration-based (default 50 iterations). Per iteration:
1. Compute material_delta vector for all cells.
2. For each interior cell (x,z):
   - Find neighbours with `(current − neighbour) / (dist × cell_size) > talus`.
   - If any such "too-steep" neighbours, compute `material_to_move = max_diff × redistribution_rate × 0.5`.
   - Remove `material_to_move` from center; distribute to lower neighbours proportionally to `(diff − talus) / total_diff`.
3. Apply all deltas in one pass.

Talus threshold: `tan(talus_angle_degrees × π/180) × cell_size`.

### Canonical correctness

**Compare to classic thermal erosion:** standard Musgrave-Kolb pattern. Two-pass (compute-then-apply) avoids order-dependence artifacts. 8-directional with distance correction (`sqrt(2)` for diagonals) is the high-quality option.

**One suspicious detail:** `material_to_move = max_diff × rate × 0.5`. The `× 0.5` factor is a damping constant to prevent oscillation (common in iterative material redistribution). Combined with `redistribution_rate=0.5` default, effective redistribution per iteration is `max_diff × 0.25`. Over 50 iterations this converges quickly.

**Boundary handling:** loop excludes x,z = 0 and x,z = resolution-1. Boundary cells are never touched. For halo-expanded heightmaps (phase 1), this means the very edges of the halo are untouched — the interesting region (the target chunk inside the halo) is 1-cell inset from the boundary, which is fine.

### No apparent bugs

Algorithm looks sound. Behavioral testing will verify the ridge-flattening expectation.

---

## Wind erosion algorithm (lines 558-613)

### Structure

Iteration-based (default 30). Per iteration:
1. Compute material_delta.
2. For each interior cell (x,z):
   - Compute `windward_pos = (x, z) − wind_direction` (where wind comes from).
   - Compute `leeward_pos = (x, z) + wind_direction × saltation_distance`.
   - If `current_height > windward_height`: erode `slope × wind_strength × 0.01` from current cell, deposit at leeward cell.
3. Apply deltas.

### Analysis

- The `.abs()` on line 593 is redundant since `current > windward` is already checked.
- `windward_idx` and `leeward_idx` use `as u32` cast of clamped f32 values — no bounds issue.
- The `× 0.01` constant is a baked-in scale; not configurable.

**Literature status:** wind erosion is less standardized than hydraulic. This is a reasonable simplification — erode windward-facing slopes, deposit on leeward. Real aeolian processes (saltation, creep, suspension) are more complex. **Niche but not wrong.**

**No apparent bugs.** Behavioral test should verify directional transport (material moves in wind direction on slope-containing heightmap).

---

## Preset analysis (§2.2 dependency)

### The four named presets

| Preset | Droplet count | Erode speed | Thermal talus | Thermal iter | Wind |
|---|---:|---:|---:|---:|---:|
| `default()` | 50000 | 0.3 | 45° | 50 | None |
| `desert()` | (no hydraulic) | - | 35° | 50 | Default (strength 0.5) |
| `mountain()` | 100000 | 0.4 | 50° | 30 | None |
| `coastal()` | 30000 | 0.3 (evap 0.02) | 40° | 20 | Strength 0.3 |

**All four presets use different numeric parameters** — they are genuinely differentiable.

### §2.2 mapping uses FOUR presets for SIX climates

The campaign plan §2.2 maps:

| ClimateBias | Erosion Preset |
|---|---|
| Temperate | `default()` |
| Cold | `mountain()` |
| Arid | `desert()` |
| Tropical | `coastal()` |
| Wetland | `coastal()` |
| Highland | `mountain()` |

Four presets × six climates works. `coastal()` covers tropical + wetland (both sediment-heavy); `mountain()` covers cold + highland (both alpine-rocky). OK per §2.2.

---

## Apparent bugs or suspicious patterns

### BUG 1 (suspected, needs behavioral verification): velocity uses `.abs()` on line 457

See hydraulic-erosion section above. Standard formula is `speed² = speed² + gravity × deltaHeight` (no abs). AstraWeave's `.abs()` means droplets always gain kinetic energy from any height change, including uphill. Effect: droplets travel further than physics would allow.

**Impact:** droplet travel distance distribution shifts upward, affecting halo-size assumption (§2.3 assumes p95 < 256 units).

**Test in phase 0.B:** measure droplet trajectories empirically; compare observed distribution to physical expectation.

### Minor: `.max(0.0)` on line 401 is redundant

`.max(config.min_slope).max(0.0)` — the second `.max(0.0)` can never trigger since `min_slope=0.01 > 0`. Stylistic; no behavioral impact.

### Minor: `.abs()` on line 593 is redundant

After `if current_height > windward_height` check, the subsequent `.abs()` is a no-op. Stylistic.

### Minor: `sample_height_bilinear` duplicates the height half of `calculate_height_and_gradient`

DRY opportunity but no correctness concern.

### Minor: no explicit termination on low water

Droplets continue for full `max_droplet_lifetime` even after water has evaporated to near-zero. Sediment capacity becomes near-zero → droplets start depositing all sediment → effectively terminates but via deposit-all rather than loop-exit. Wasted computation, not incorrect.

### Not a bug: heights clamped to `[0, ∞)` by erosion `current.min(weighed_erode)`

Erosion can reduce heights to zero but not below. Acceptable for AstraWeave's always-positive heightmaps.

---

## Determinism analysis

- `apply_hydraulic_erosion` creates fresh `SimpleRng::new(self.seed)` on every call. Same seed → same RNG sequence → same droplet positions → same output.
- `apply_thermal_erosion` and `apply_wind_erosion` are purely deterministic (no RNG).
- `apply_preset` calls each component in `pass_order`. Hydraulic RNG is reset each time `apply_hydraulic_erosion` is called, so different `pass_order` orderings with hydraulic-called-twice would produce identical hydraulic output both times.
- Existing test `test_deterministic_erosion` verifies determinism for hydraulic alone.

**Determinism is cross-run**: same seed + same heightmap = same output. ✓

**Determinism is NOT across compiler versions** (float ordering of operations may differ). Not a concern for in-session use.

---

## Existing test coverage

| Test | What it verifies |
|---|---|
| `test_hydraulic_erosion_reduces_peaks` | Peak decreases OR ≥100 material eroded (disjunction — either satisfies) |
| `test_thermal_erosion_smooths_slopes` | `total_eroded > 0` |
| `test_erosion_preset_applies_all_passes` | `total_eroded > 0` |
| `test_wind_erosion_moves_material` | `total_eroded > 0` |
| `test_deterministic_erosion` | Two runs with same seed match within 0.001 for every vertex |

**Coverage gaps:**
- No flat-heightmap preservation test.
- No mass-conservation test.
- No preset-differentiation test.
- No droplet travel distance characterization.
- No test that ridges flatten (only that material moved).
- No test that sediment accumulates at low points.
- The existing tests all use a synthetic cone heightmap; no stress-test heightmaps.

---

## Recommendations for phase 0.B (behavioral testing)

Based on the static findings, the behavioral audit should verify:

1. **Velocity-`.abs()` impact** — does it produce unreasonable droplet travel? If yes, fix in phase 0.D.
2. **Flat heightmap preservation** — erosion on flat terrain should be a no-op.
3. **Mass conservation on slopes** — material moved but total preserved (modulo evaporation).
4. **Ridge flattening** — sharp ridges reduced by thermal.
5. **Spike flattening** — isolated spikes (the F.2-T-4 post-residual pattern) reduced by erosion.
6. **Multi-spike curvature reduction** — grid of spikes smoothed.
7. **Bowl sediment accumulation** — water collects at low points, deposits sediment.
8. **Preset differentiation** — default vs desert vs mountain produce visibly different output.
9. **Droplet travel distance** — empirical p95 validation for §2.3 halo assumption.

---

## Sources cited

- Sebastian Lague, "Coding Adventure: Hydraulic Erosion" (YouTube, 2019) — widely-copied C# reference implementation. No URL (video); corresponding GitHub repo at <https://github.com/SebLague/Hydraulic-Erosion>.
- Hans Beyer, "Implementation of a method for hydraulic erosion" (2015) — original particle-based formulation.
- dandrino/terrain-erosion-3-ways — <https://github.com/dandrino/terrain-erosion-3-ways>, community-canonical Rust/Python reference.
- F.2-T-3 research doc: `docs/audits/terrain_noise_research_2026-04-22.md`.
- Campaign plan: `docs/current/TERRAIN_GENERATION_QUALITY_CAMPAIGN.md` §2.2, §2.3, §5.

---

## Static-audit verdict

**Simulator soundness: MOSTLY SOUND with one suspected behavioral divergence and several stylistic redundancies.**

- Algorithm structure matches canonical implementations (Lague, dandrino).
- One suspected bug (velocity `.abs()`) needs behavioral verification.
- Minor redundancies don't affect output.
- API is stable and suitable for phase 2 wiring.
- Existing test coverage is weak (only checks "erosion ran"); phase 0.B adds behavioral assertions.

**Recommendation for phase 0.B:** proceed with synthetic heightmap tests. Focus on measuring droplet travel distance (validates §2.3 halo assumption) and the velocity-`.abs()` question. If both pass, phase 1 can proceed with confidence.
