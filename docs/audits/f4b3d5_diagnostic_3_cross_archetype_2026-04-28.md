# F.4.B.3.D.5-diagnostic-3 Cross-Archetype Terrain Similarity Audit

**Date**: 2026-04-28
**Mode**: Investigation-only (no production code changes; `#[ignore]`-marked measurement test added).
**Trigger**: Andrew-gate post-D.5-fix revealed four archetypes (Continental Temperate, Equatorial Tropical, Desert, Mediterranean) producing visually identical worlds despite Path B amplitude reduction successfully bringing max elevations into Target B's design range. This diagnostic measures the contribution of archetype-aware vs archetype-agnostic code paths to actual terrain shape variance.
**Predecessor audits**: `docs/audits/f4b3d5_diagnostic_report_2026-04-28.md`, `docs/audits/f4b3d5_diagnostic_2_real_heightmap_2026-04-28.md`.
**Diagnostic state**: D.5-fix at commit `2d7459a75` + hash-fixup `35c6b4244`.

---

## 1. Background

D.5-fix's Path B successfully addressed the elevation problem (max 1214m → 698.5m). But Andrew-gate visual verification revealed a deeper issue: four archetypes produce identical-looking worlds. Different surface coloring, different scatter density, but the terrain **shape itself** — where peaks form, where valleys cut, the mountain spacing — is approximately archetype-invariant.

This diagnostic asks: how much of terrain shape comes from archetype-aware code paths (climate field → biome ID → per-biome amplitude) vs archetype-agnostic code paths (bootstrap noise: base + mountain layer + continental modulation)?

The decision matrix from §0:

- **Pearson > 0.95 + ratio < 0.10**: bootstrap dominates. Path 1 (new sub-campaign for per-archetype bootstrap shaping) justified.
- **Pearson 0.7-0.95 + ratio 0.10-0.30**: meaningful but bounded variance from archetype path. In-campaign fix possible if variance can be amplified.
- **Pearson < 0.7 + ratio > 0.30**: archetypes already differ; visual problem is elsewhere (lighting/scatter/color).

---

## 2. Methodology

`#[ignore]`-marked test `phase_1_6_f4_b_3_d_5_diagnostic_3_cross_archetype.rs::d5_diagnostic_3_cross_archetype_terrain_similarity`. Single test, ~36s wall-clock. Three measurements + one code inspection:

- **§1.1**: per-vertex post-erosion height comparison at 5 chunks per archetype pair (CT vs Desert; CT vs Equatorial Tropical). 5 × 9216 = 46,080 vertex-pair differences per pair.
- **§1.2**: per-source variance decomposition at chunk (0,0) for 3 archetypes (CT, Desert, ET). Bootstrap-only (mountains_amplitude=1.0 everywhere) vs current pipeline (per-biome blended modulation pre-erosion).
- **§1.3**: per-archetype blended amplitude distribution at 1000 random world positions for all 6 archetypes.
- **§1.4**: code-inspection audit of `WorldConfig` / `NoiseConfig` / `TerrainNoise::sample_height_with_mountain_amplitude` to identify archetype-aware vs archetype-agnostic parameters.

---

## 3. Measurements

### 3.1 §1.1 Cross-archetype heightmap similarity

**Continental Temperate vs Desert** (5 chunks aggregate):

| Chunk         | Mean \|Δ\| | p50 \|Δ\| | p95 \|Δ\| | Max \|Δ\| | Pearson |
| ------------- | --------: | --------: | --------: | --------: | ------: |
| (0, 0)        | 14.71m    | 13.34m    | 33.08m    | 79.15m    | 0.995   |
| (5, 5)        | 15.59m    | 13.86m    | 37.43m    | 99.14m    | 0.996   |
| (-3, 4)       | 37.90m    | 37.00m    | 74.22m    | 139.84m   | 0.988   |
| (0, -7)       | 28.22m    | 23.73m    | 73.13m    | 120.19m   | 0.986   |
| (8, 1)        | 14.01m    | 11.82m    | 33.87m    | 104.02m   | 0.994   |
| **AGGREGATE** | **22.08m** | 17.36m   | 58.89m    | 139.84m   | **0.989** |

**Continental Temperate vs Equatorial Tropical** (5 chunks aggregate):

| Chunk         | Mean \|Δ\| | p50 \|Δ\| | p95 \|Δ\| | Max \|Δ\| | Pearson |
| ------------- | --------: | --------: | --------: | --------: | ------: |
| (0, 0)        | 18.35m    | 8.57m     | 75.14m    | 154.98m   | 0.973   |
| (5, 5)        | 11.98m    | 8.00m     | 36.93m    | 103.23m   | 0.994   |
| (-3, 4)       | 43.34m    | 41.51m    | 86.97m    | 154.04m   | 0.981   |
| (0, -7)       | 27.25m    | 19.84m    | 80.70m    | 145.26m   | 0.979   |
| (8, 1)        | 10.57m    | 7.25m     | 32.02m    | 120.75m   | 0.993   |
| **AGGREGATE** | **22.30m** | 13.38m   | 73.32m    | 154.98m   | **0.978** |

Both archetype pairs land in the **Pearson > 0.95** range — nearly-identical heightmaps. Mean |Δ| of ~22m at typical chunk elevations of 100-500m = 4-20% relative variance. This is meaningful but bounded; per the §1 framework's "Mean |Δ| 20-50m: meaningful per-archetype shape variance, just not visually expressed."

### 3.2 §1.2 Per-source variance decomposition (chunk (0,0))

Bootstrap-only (climate-field path disabled, mountains_amplitude=1.0 everywhere) is **identical across all archetypes** (it ignores climate config; output depends only on `WorldConfig.noise` + seed):

```
Bootstrap-only:
  mean=178.38m  variance=12,827.52m²  p50=171.45m  p95=380.62m  max=535.64m
```

With per-biome modulation (current D.5-fix pipeline, pre-erosion):

| Archetype | mean | variance | p95 | max | var(delta) | Ratio | % contribution |
|-----------|-----:|---------:|----:|----:|-----------:|------:|---------------:|
| Continental Temperate | 228.85m | 25,460m² | 542.32m | 736.90m | 2,461m² | 0.192 | **19.2%** |
| Desert | 214.80m | 27,117m² | 542.32m | 736.90m | 3,221m² | 0.251 | **25.1%** |
| Equatorial Tropical | 212.52m | 25,529m² | 542.32m | 736.90m | 3,609m² | 0.281 | **28.1%** |

Per-biome modulation contributes **19-28% of variance**; bootstrap contributes 72-81%. Per the §1 framework's "Ratio 0.10-0.30: per-biome modulation is meaningful but small." Notably, the p95 elevation is identical (542.32m) across all three archetypes, and the max is identical (736.90m) — bootstrap completely determines the high-elevation regime, climate-field path only nudges intermediate elevations.

### 3.3 §1.3 Per-archetype amplitude distributions (1000 samples each)

| Archetype | mean | stddev | p25 | p50 | p75 | p95 | Dominant bin |
|-----------|-----:|-------:|----:|----:|----:|----:|-------------|
| Continental Temperate | 1.314 | 0.259 | 1.200 | 1.400 | 1.400 | 1.500 | [1.0, 1.5) at 79.6% |
| Equatorial Tropical | 1.184 | 0.327 | 1.200 | 1.272 | 1.400 | 1.400 | [1.0, 1.5) at 81.4% |
| Boreal/Subarctic | 1.379 | 0.259 | 1.400 | 1.400 | 1.500 | 1.500 | [1.0, 1.5) at 61.1% |
| Mediterranean | 1.097 | 0.327 | 0.800 | 1.200 | 1.400 | 1.400 | [1.0, 1.5) at 59.1% |
| Desert | 1.162 | 0.268 | 1.000 | 1.200 | 1.400 | 1.400 | [1.0, 1.5) at 85.0% |
| Custom (= CT) | 1.314 | 0.259 | 1.200 | 1.400 | 1.400 | 1.500 | [1.0, 1.5) at 79.6% |

Means range from 1.097 (Mediterranean) to 1.379 (Boreal/Subarctic) — a span of **0.282** (26% relative variance). All distributions concentrate in `[1.0, 1.5)` (59-85% of samples). The p95 values range only 1.4-1.5. The amplitude lookup is operating in a narrow band across all archetypes.

The Mediterranean and Desert archetypes show meaningful tail mass in `[0.5, 1.0)` (12-38%) due to grassland/desert biomes' lower amplitudes; Boreal/Subarctic shows mass in `[1.5, 2.0)` (35.8%) due to mountain-character biomes' higher post-Path-B amplitudes. But the dominant range is the same across all six.

### 3.4 §1.4 Bootstrap parameter archetype-awareness audit

Code inspection of `astraweave-terrain/src/noise_gen.rs::NoiseConfig::default()` and `TerrainNoise::sample_height_with_mountain_amplitude`:

| Parameter | Default value | Source | Archetype-aware? |
|-----------|--------------:|--------|:-:|
| `base_elevation.amplitude` | 150.0 | `NoiseConfig::default()` | ❌ |
| `base_elevation.scale` | 0.005 | `NoiseConfig::default()` | ❌ |
| `base_elevation.octaves` | 4 | `NoiseConfig::default()` | ❌ |
| `base_elevation.persistence` | 0.5 | `NoiseConfig::default()` | ❌ |
| `base_elevation.lacunarity` | 2.0 | `NoiseConfig::default()` | ❌ |
| `base_elevation.noise_type` | Perlin | `NoiseConfig::default()` | ❌ |
| `mountains.amplitude` | **480.0** | `NoiseConfig::default()` | ❌ |
| `mountains.scale` | 0.002 | `NoiseConfig::default()` | ❌ |
| `mountains.octaves` | 6 | `NoiseConfig::default()` | ❌ |
| `mountains.noise_type` | RidgedNoise | `NoiseConfig::default()` | ❌ |
| `detail.amplitude` | 12.5 | `NoiseConfig::default()` | ❌ |
| `continental_enabled` | false | `default_continental_enabled()` | ❌ |
| `continental_scale` | 0.0003 | `default_continental_scale()` | ❌ |
| `continental_min` | 0.50 | `default_continental_min()` | ❌ |
| `continental_seed_offset` | 7 | `default_continental_seed_offset()` | ❌ |
| `erosion_enabled` | true | `NoiseConfig::default()` | ❌ |
| `erosion_strength` | 0.3 | `NoiseConfig::default()` | ❌ |
| `mountain_amplitude_multiplier` (per-vertex) | varies | climate field → biome lookup → BiomeParameters | ✅ |

**Result: 1 of 18 bootstrap parameters is archetype-aware.** All shape-determining parameters (where peaks form, what frequency/octaves the noise has, what continental clustering looks like) are archetype-agnostic constants. The single archetype-aware lever is the per-vertex mountain layer amplitude multiplier — a SCALAR multiplied onto the mountain noise at each vertex.

A scalar multiplier cannot change shape; it can only scale heights. Different archetypes scale the same shape by different amounts. This is consistent with the §3.1-§3.2 measurements: Pearson > 0.95 (same shape), variance contribution 19-28% (height scaling is meaningful but bounded).

---

## 4. Findings

1. **Bootstrap dominates terrain shape (72-81% of variance), per-biome modulation contributes 19-28%.** Both pieces of data (cross-archetype Pearson 0.978-0.989 and variance ratio 0.19-0.28) are consistent: archetypes share a fundamental shape; per-biome modulation only adjusts heights within that shape.

2. **Bootstrap is 100% archetype-agnostic.** Of 18 noise/terrain parameters, only the per-vertex mountain amplitude multiplier varies by archetype. All shape-determining parameters (mountain amplitude bulk, scale, octaves, noise type, continental scale, continental min, base elevation amplitude, etc.) are constants from `NoiseConfig::default()` — no archetype touches them.

3. **Per-archetype amplitude distributions cluster narrowly.** Means span 1.097-1.379 (~26% relative variance); 60-85% of samples in `[1.0, 1.5)` across all 6 archetypes; p95 values 1.4-1.5 across the catalog. Even with the maximum amplitude differential (Boreal 1.379 vs Mediterranean 1.097 = 0.28 difference), the height delta at a typical peak (~150m mountain contribution) would be ~42m. That's the visual delta the cross-archetype comparison measures.

4. **Bootstrap dominates the high-elevation regime entirely.** §3.2 shows p95 and max post-modulation elevations are identical across all three archetypes (542.32m / 736.90m). The peaks are the same peaks; only intermediate-elevation regions vary by archetype.

5. **The visual identicality is correctly explained by the variance + parameter findings.** A 22m mean |Δ| on top of a 178m mean elevation = 12% relative variance. The architecture is producing the same world with ~12% perturbation per archetype. To the eye, that reads as "same world, slightly different lighting."

6. **None of these findings are bugs.** Each piece works as designed. The architecture chose to make biomes drive **per-biome shape character via amplitude** (D.3a's design decision), not **per-archetype shape character via bootstrap parameters**. The visual variety problem is in the design boundary, not in implementation correctness.

7. **D.5-fix's Path B did its job.** Max elevation 1214m → 698m as intended. The post-Path-B amplitude range (1.0-1.5 dominant band) IS the bounded variance the audit measures. Path B didn't shrink archetype variety — that was always small.

---

## 5. Campaign-Level Recommendation

The data is decisive: **bootstrap dominates (72-81%); the climate-field path cannot deliver per-archetype world variety on its own**. Path 1 vs Path 2 depends on whether per-archetype world variety is a near-term goal or a deferred goal.

### Path 1 (recommended for product impact): Branch new sub-campaign for per-archetype bootstrap shaping

**Scope sketch (research pass + 6-10 sub-phases)**:

1. **Research pass**: how do AAA games (Minecraft 1.18+ density functions, NMS per-planet shape parameters, Dwarf Fortress per-region erosion variance, Enshrouded biome-driven mountain shapes) deliver per-archetype world shape? What's the parameter space?
2. **F.0 design**: define per-archetype bootstrap parameter set. Candidates:
   - Per-archetype `NoiseConfig.mountains.amplitude` (e.g., Desert 200, Boreal 600).
   - Per-archetype `NoiseConfig.mountains.scale` (e.g., Mediterranean 0.001 = larger features, Equatorial Tropical 0.003 = denser smaller features).
   - Per-archetype `continental_scale` (different macro-clustering per archetype).
   - Per-archetype `noise_type` (Boreal RidgedNoise for sharp peaks, Equatorial DomainWarped for organic ridges).
   - Per-archetype `base_elevation.amplitude` (lowland-heavy archetypes get smaller base layer).
3. **F.1 plumbing**: extend `WorldArchetype` to carry these parameters or have `ArchetypeBootstrapParams` struct.
4. **F.2 wiring**: `WorldGenerator::new(config)` constructs `TerrainNoise` from archetype-driven `NoiseConfig` instead of `WorldConfig::default().noise`.
5. **F.3 archetype tuning**: define each of the 6 archetypes' specific bootstrap values (with re-running diagnostic-3 to verify cross-archetype Pearson drops from 0.98 toward 0.5-0.7 range).
6. **F.4 Andrew-gate + tightening**: visual verification across archetypes. Tune until each archetype reads as visibly distinct.
7. **F.5 closeout**.

**Estimated commitment**: 6-10 sessions, comparable to the F.4.B.3.D reframe campaign. Inherits everything D.1-D.5 built (climate field, biome lookup, per-biome parameters, blending) — just adds an upstream archetype-driven parameter layer.

**Risk**: per-archetype noise tuning is an iterative process. Some archetypes may not converge to "visually distinct" character without changing more than just bootstrap parameters (e.g., introducing per-archetype erosion styles, surface texture palettes, scatter character).

### Path 2 (recommended for campaign efficiency): Close current campaign as PARTIAL

Ship F.4.B.3.D at PARTIAL. The deliverables that DID land:
- Climate field architecture (D.1)
- Whittaker biome lookup over 19 fixed biomes (D.2)
- Per-biome parameter system (D.3, replacing legacy preset abstraction)
- Scattered-convolution biome blending (D.4)
- World archetype catalog + UI (D.5)
- Path B amplitude reduction for elevation-overlay biomes (D.5-fix)

What's deferred to a future campaign:
- Per-archetype world shape variety (this audit's finding).
- Climate Preview overlay (D.5c deferred per §1.5).
- Equatorial Tropical archetype-specific tuning.
- Bootstrap noise pipeline elevation skew (audit §7 of diagnostic-2).
- Dead-write bug at terrain_panel.rs line 943.
- F.4.B.3.G inheritances.

This path accepts archetype-as-coloring + biome-classification as the current layer of variety. The user gets:
- Same world shape across archetypes (NC/Appalachia analog).
- Biome distribution shifts per archetype (Continental Temperate has temperate forests, Desert has subtropical desert, etc.) — the §1.3 amplitude distributions confirm meaningful biome-level variation.
- Surface coloring + scatter species shift per dominant biome.

For Veilweaver development, this lets **playable content work proceed against the current terrain**. The per-archetype shape variety is a Phase 2/3 polish concern, not a Phase 1 blocker.

### Trade-off summary

| Dimension | Path 1 | Path 2 |
|-----------|--------|--------|
| Scope | 6-10 sessions, research + design + tuning | 1-2 sessions (closeout) |
| Risk | High (iterative tuning, may not converge) | Low (documented limitation) |
| Veilweaver impact | Per-archetype distinct worlds (high product polish) | Same world shape, archetype-specific biomes (acceptable for content work) |
| Sunk cost recovery | All D.1-D.5 work feeds Path 1 directly | All D.1-D.5 ships as-is |
| Campaign discipline | Yet another reframe; campaign continues open-ended | Honest closeout of stated scope |

**My recommendation: Path 1 if Andrew has appetite for 6-10 more sessions on terrain. Path 2 if the campaign needs to close and per-archetype variety is acceptable as deferred work.**

The data does NOT decide for Andrew. Both paths are defensible given the measurements. The decision is appetite + product priority.

---

## 6. What This Investigation Did NOT Address

- **Boreal/Subarctic + Mediterranean cross-archetype similarity**: only CT vs Desert and CT vs ET measured for §1.1. The other archetype pairs likely show similar Pearson > 0.95 patterns given §1.3's narrow amplitude distributions, but unverified at real-chunk level.
- **The 0.0-0.5 amplitude bin in §1.3**: every archetype shows ~3% of samples landing in `[0.0, 0.5)`. This is the aquatic/coastal biome zone. Verified consistent across archetypes; not a bug, not investigated further.
- **Visual character with high-amplitude biomes restored**: D.5-fix's Path B reduced these from 2.5-3.0× to 1.4-1.6×. If a future Path 1 campaign restores the higher amplitudes WHILE adding bootstrap-level archetype variation, the visual character will differ from this audit's data point. This audit is post-Path-B baseline.
- **Erosion's role in archetype variety**: post-erosion heights are what's visualized. If different archetypes used different `erosion_preset` (e.g., Mediterranean = arid erosion, Boreal = freeze-thaw erosion), per-archetype shape variety might emerge from erosion alone without changing bootstrap. Not measured; tracked as Path 1 candidate parameter.
- **Climate field's effect on continental modulation**: `continental_enabled` is `false` in `NoiseConfig::default()`. If continental modulation were enabled with archetype-driven `continental_scale`, that alone might shift cross-archetype Pearson. Not investigated; tracked as Path 1 candidate parameter.

---

## 7. References

- D.5-diagnostic audit: `docs/audits/f4b3d5_diagnostic_report_2026-04-28.md`.
- D.5-diagnostic-2 audit: `docs/audits/f4b3d5_diagnostic_2_real_heightmap_2026-04-28.md` (load-bearing for D.5-fix's Path B).
- D.5-diagnostic-2 §6 Path C sketch: "decouple bootstrap height from biome-lookup elevation" — distinct from this audit's Path 1 (which keeps biome-lookup's elevation source unchanged but adds per-archetype variation to bootstrap parameters themselves).
- D.5-fix commits: `2d7459a75` (Path B amplitude reduction) + `35c6b4244` (hash-fixup).
- `WorldConfig` definition: `astraweave-terrain/src/lib.rs`.
- `NoiseConfig::default()`: `astraweave-terrain/src/noise_gen.rs:170-235`.
- `TerrainNoise::sample_height_with_mountain_amplitude`: `astraweave-terrain/src/noise_gen.rs:545-660`.
- `WorldArchetype` definition: `astraweave-terrain/src/climate.rs`.
- `BiomeParameters::for_biome`: `astraweave-terrain/src/biome_parameters.rs`.
- D.5-diagnostic-2 measurement test: `astraweave-terrain/tests/phase_1_6_f4_b_3_d_5_diagnostic_2_real_heightmap.rs`.
- Diagnostic-3 measurement test: `astraweave-terrain/tests/phase_1_6_f4_b_3_d_5_diagnostic_3_cross_archetype.rs`.

---

## 8. Test Disposition

Diagnostic-3 measurement test marked `#[ignore]`. Runnable on demand:

```
cargo test -p astraweave-terrain --release --test phase_1_6_f4_b_3_d_5_diagnostic_3_cross_archetype d5_diagnostic_3 -- --ignored --nocapture
```

Wall-clock cost: ~36 seconds total (5 chunks × 2 archetype pairs at ~0.8s each + 3 archetype variance decompositions at <1s each + 6 archetype amplitude distributions at <0.5s each).

**Disposition recommendation**:
- **If Path 1 lands**: KEEP. The cross-archetype Pearson and variance ratio metrics are the principal "did per-archetype variety land?" signal. Re-run after each Path 1 sub-phase to verify Pearson drops from ~0.98 toward target ~0.5-0.7.
- **If Path 2 lands**: KEEP and rename to `phase_1_6_f_terrain_archetype_baseline.rs` or similar. Documents the as-shipped baseline for any future campaign that revisits per-archetype variety.

---

## 9. One-paragraph TL;DR

Cross-archetype heightmap correlation (Pearson) is 0.978-0.989 across the measured pairs (Continental Temperate vs Desert and CT vs Equatorial Tropical) — nearly identical worlds. Per-source variance decomposition shows bootstrap noise contributes 72-81% of terrain shape variance; per-biome modulation contributes 19-28%. Per-archetype amplitude distributions cluster narrowly (means 1.097-1.379, 60-85% of samples in `[1.0, 1.5)`). Code inspection confirms 1 of 18 bootstrap parameters is archetype-aware (the per-vertex mountain amplitude multiplier). **Path 1 (new sub-campaign for per-archetype bootstrap shaping)** is the data-driven recommendation if per-archetype world variety is a product goal; **Path 2 (close current campaign as PARTIAL)** is honest closeout if archetype-as-coloring + biome-classification is acceptable for current Veilweaver development. The data does not decide; appetite + product priority do.
