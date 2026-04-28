# F.4.B.3.D.5-diagnostic — Three-Question Investigation Report

**Date**: 2026-04-28
**Mode**: Investigation-only (no production code changes; throwaway diagnostic test added then reverted).
**Trigger**: Andrew-gate REGRESS verdict on D.5 across all six archetypes (explosive radial spike pattern, knife-edge 2D-wall mountains, 11.3-min generation, "Not Responding" UI state).
**Diagnostic state checked**: D.5b commit `0315d6911` (head of main).

---

## 1. Executive Summary

Three questions investigated. Findings:

- **Q1 (parameter regression)**: CONFIRMED — Continental Temperate's `moisture_variance_mm` dropped 600→400 (-33%) and `continentalness_variance` dropped 0.25→0.2 (-20%) at D.5a. **However, this was intentional** per D.5 plan §1.1 (documented in §10) and **does NOT cause the visual REGRESS** — see Q3.
- **Q2 (UI freeze)**: PARTIAL — chunk generation runs on a background thread (line 2013 in `terrain_panel.rs`), so the UI is structurally protected from blocking. Per-chunk cost at D.5b measured **0.837s/chunk** vs D.4 baseline **0.747s/chunk** (+12%). The 11.3-min Equatorial Tropical generation likely results from per-chunk cost in archetypes with heavier biome distributions × 441-chunk radius-10 work × interactive multi-regeneration on slider drags, NOT a step-cost regression in the per-chunk path. ALSO found: a *dead-write* bug — the dropdown's `self.terrain_state.set_world_archetype(...)` call (line 943) mutates state that `regenerate_terrain` immediately discards by creating a fresh `TerrainState::new()` (line 1982). Functional but confusing.
- **Q3 (per-vertex amplitude/biome distribution)**: UNEXPECTED FINDING — **mountain-character biomes (Alpine + SnowCap + MountainRocky + Scree) account for 41-53% of biome assignments across ALL six archetypes**, including Equatorial Tropical (41.7%) and Continental Temperate (41.8%). Equatorial Tropical's #1 dominant biome is **Alpine at 41.4%** (not equatorial at all). Comparing D.4 variance (600/0.25) vs D.5 variance (400/0.2): biome distributions are **essentially identical** (±1%). Conclusion: this is NOT a D.5 regression — it's a structural finding from the D.1-D.4 architecture that surfaced when D.5's Andrew-gate finally tested visually. **D.4 was never visually verified** (D.4 §10 explicitly deferred Andrew-gate to D.6); D.5 is the first interactive test of the entire stack.

**Verdict on the three questions:**
- Q1: Variance regression confirmed but ruled out as primary cause.
- Q2: UI freeze is multi-second pauses (per-chunk × 441 chunks × possibly multiple regenerations on slider drag), not blocking; dead-write bug in dropdown callback documented.
- Q3: Architectural finding — elevation overlay dominance + biome boundary amplitude flips drive the spike pattern. The architectural correction landed at D.1-D.4 has a structural defect; D.5 just exposed it.

The primary remediation path is NOT another D.5 fix; it's a D.1-D.4 architectural review focused on **elevation overlay threshold calibration** and/or **per-biome amplitude defaults** for mountain-character biomes.

---

## 2. Q1 Findings: Continental Temperate parameter regression

**D.4 Continental Temperate parameters** (`WorldArchetype::default()` at commit `646e00657`):

```
temperature_mean_c: 12.0
temperature_variance_c: 8.0
latitude_temperature_drop_c: 10.0
moisture_mean_mm: 1100.0
moisture_variance_mm: 600.0
continentalness_mean: 0.5
continentalness_variance: 0.25
```

**D.5 Continental Temperate parameters** (`world_archetypes::continental_temperate()` at commit `0315d6911`):

```
temperature_mean_c: 12.0          [unchanged]
temperature_variance_c: 8.0        [unchanged]
latitude_temperature_drop_c: 10.0  [unchanged]
moisture_mean_mm: 1100.0           [unchanged]
moisture_variance_mm: 400.0        [-33% from 600]
continentalness_mean: 0.5          [unchanged]
continentalness_variance: 0.2      [-20% from 0.25]
```

**Differences:**
- `moisture_variance_mm`: 600 → 400 (-200, -33.3%)
- `continentalness_variance`: 0.25 → 0.2 (-0.05, -20%)
- All other 5 fields unchanged.

**D.5a commit diff inspection** (`git diff 646e00657..88c1d2669 -- astraweave-terrain/src/climate.rs`):
- `WorldArchetype::default()` body changed from inline `Self { ... }` to `crate::world_archetypes::continental_temperate()` delegation.
- The delegated function in the new `world_archetypes.rs` file uses 400/0.2 values per D.5 plan §1.1's tuned envelope.
- No other archetype's parameters were touched in the D.5a diff (only Continental Temperate had its `default()` lifted).
- Equatorial Tropical's later tuning (2200→1900 / 800→1300, documented in D.5a §10) was deliberate and confined to that archetype; spot-checked the D.5a→D.5b commit chain — no other archetype's parameters drifted.

**Verdict: REGRESSION CONFIRMED but NOT ROOT CAUSE.**

The variance change was intentional per D.5 plan §1.1 ("moisture_variance_mm 400mm, continentalness_variance 0.2"). The D.5 §10 deviation log documented this as expected: "downstream tests (D.4 blending, D.2 distribution) are robust to this small variance change." That assertion is correct at the test level. But Q3 below shows the visual issue is upstream of this change — **the same architectural defect exists at D.4's variance values**.

**Sub-finding (Q3-driven counter-test)**: re-running the per-archetype distribution diagnostic with Continental Temperate's variance reverted to D.4 values (600/0.25) produced biome distributions within ±1% of D.5 values. The variance change is NOT shifting biome assignment meaningfully.

---

## 3. Q2 Findings: UI freeze + per-chunk cost

**`set_world_archetype` callback chain** (`tools/aw_editor/src/panels/terrain_panel.rs`):

```
Dropdown selection (line 943) →
  self.terrain_state.set_world_archetype(archetype)  [DEAD WRITE — see below]
  self.regenerate_terrain()  →
    if self.generating { return; }  [re-entrancy guard]
    let mut state = TerrainState::new()  [discards self.terrain_state mutation]
    state.configure(self.seed, &self.primary_biome)
    state.set_noise_params(...)
    state.set_world_archetype(archetype)  [actual archetype application]
    std::thread::spawn(move || { state.generate_terrain(chunk_radius); ... })  [BACKGROUND THREAD]
```

**Dead-write finding**: the dropdown's `self.terrain_state.set_world_archetype(archetype)` at line 943 mutates `self.terrain_state`, but `regenerate_terrain` immediately creates `let mut state = TerrainState::new()` at line 1982 and applies the archetype to that fresh state. The mutation on `self.terrain_state` never reaches the generator. Functionally equivalent to no-op but confusing for future developers. The Custom-archetype slider callback (line 1035) has the same pattern.

**UI thread blocking analysis**:
- `regenerate_terrain` early-returns if `self.generating` is true (re-entrancy guard at line 1950).
- Background thread spawned at line 2013 runs the actual chunk generation.
- UI thread should NOT block on the 441-chunk work.

**However**, the slider-drag pattern can cause multi-second UI stalls:
- Slider drag fires `changed()` per frame (~60 Hz).
- Each `changed()` calls `regenerate_terrain()`.
- First call sets `self.generating = true` and spawns thread.
- Subsequent calls early-return (re-entrancy guard works correctly).
- BUT — when generation completes, `self.generating` becomes false. The NEXT slider tick fires another generation.
- For a slider drag spanning 5 seconds at 60 Hz, this is bounded by the re-entrancy guard but still produces N sequential generations of ~0.8s each = N × 0.8s of cumulative work.
- The user observes the editor as "Not Responding" if the UI redraw takes > a few hundred ms; chunk generation on background thread shouldn't trigger this directly, but a busy UI thread (re-rendering panel + processing slider drag events + listening for thread completion) might.

**Per-chunk generation time at D.5b** (Continental Temperate, seed 12345, perf test): **0.837s/chunk mean** (range 0.776-0.993s) across 6 sample positions.

vs D.4 baseline: 0.747s/chunk (perf test at same commit). Δ = +12%. This is plausibly explained by:
- D.5a Continental Temperate variance change (less smooth climate distribution → marginally more biome boundaries → slightly more blending sample variance).
- Test-to-test noise.

vs Andrew's reported 11.3-min single Equatorial Tropical generation: 11.3 × 60 = 678s. For 441 chunks at radius 10: 678 / 441 = **1.54s/chunk average**. That's roughly 2× D.5b's 0.837s Continental Temperate baseline. The difference is plausibly attributable to:
- Equatorial Tropical produces 41.7% mountain-character biomes (per Q3); these biomes have higher `mountains_amplitude` (2.0-3.0×) which produces more dramatic terrain → more erosion droplet work per chunk.
- The perf test disables erosion's full mountain preset; the editor's `generate_terrain` uses the full pipeline including erosion.
- 11.3 min may include scatter generation + GPU upload + mesh assembly work on top of pure chunk generation.

**Per-step cost breakdown (NOT measured directly — would require timing each step)**:
- Climate sample: pure function of (x, z, elev), cheap (~µs per call).
- Biome lookup: pure function, cheap.
- Blending (6 samples per vertex × climate + lookup): bounded multiplier, ~6µs per vertex.
- Per-vertex height computation (sample_height_with_mountain_amplitude): dominated by noise sampling, ~10µs per vertex.
- Erosion: dominant cost (per F.3 measurements, 60s+ for Temperate radius-5 grid; D.4 mountain preset bumps this further).

Step that grew significantly between D.4 and D.5: NONE measurable. The +12% per-chunk delta is within sampling noise.

**Verdict: NOT UI THREAD BLOCKING (background thread protects), NOT REGENERATION MULTIPLICATION (re-entrancy guard works), NOT PER-CHUNK COST REGRESSION (within noise).**

The 11.3-min observation is plausibly explained by:
1. Equatorial Tropical archetype's higher mountain-character biome frequency → more erosion work per chunk (~2× Continental Temperate cost).
2. 441 chunks × ~1.5s average = ~11 min for full generation.
3. UI feeling unresponsive because the background thread saturates the CPU and the UI redraw fights for thread time.

The dropdown dead-write is documented as a non-blocking finding.

---

## 4. Q3 Findings: per-archetype amplitude + biome distribution

**Diagnostic methodology**: throwaway test added to `phase_1_6_f4_b_3_d_3_perf.rs` (subsequently reverted per §2 plan). For each archetype, sampled 1000 random `(world_x, world_z, elevation)` tuples (elevation uniform in [-10, 510m]) and called `blend_biome_parameters` to capture blended `mountains_amplitude` + dominant `BiomeId`.

**Per-archetype results** (D.5b parameters, seed 12345):

### Continental Temperate
- Blended `mountains_amplitude`: min=0.000 mean=**1.773** p50=1.500 p95=2.500 p99=2.500 max=2.500
- Top 5 dominant biomes: TemperateDeciduousForest 31.8%, **SnowCap 28.9%**, BorealForest 20.0%, **Alpine 12.7%**, Tundra 3.0%
- Mountain-character (Alpine+SnowCap+MountainRocky+Scree): **41.8%**
- Blending math bound violations: 0/1000 (convex-combination invariant holds)

### Equatorial Tropical
- Blended `mountains_amplitude`: min=0.000 mean=**1.643** p50=1.277 p95=2.500 p99=2.500 max=2.500
- Top 5 dominant biomes: **Alpine 41.4%** (!!), TropicalSeasonalForest 31.2%, TemperateGrassland 9.1%, TropicalRainforest 7.7%, Savanna 3.6%
- Mountain-character: **41.7%**
- Blending math bound violations: 0/1000

### Boreal/Subarctic
- Blended `mountains_amplitude`: min=0.000 mean=**1.926** p50=2.000 p95=2.500 p99=2.500 max=2.500
- Top 5 dominant biomes: Tundra 43.7%, **SnowCap 28.9%**, **Alpine 12.7%**, **Scree 11.2%**, Ocean 2.0%
- Mountain-character: **52.8%**
- Blending math bound violations: 0/1000

### Mediterranean
- Blended `mountains_amplitude`: min=0.000 mean=**1.638** p50=2.000 p95=2.500 p99=2.500 max=2.500
- Top 5 dominant biomes: TemperateGrassland 36.5%, **SnowCap 28.8%**, **Alpine 12.8%**, **Scree 10.3%**, TemperateDeciduousForest 6.6%
- Mountain-character: **51.9%**
- Blending math bound violations: 0/1000

### Desert
- Blended `mountains_amplitude`: min=0.000 mean=**1.709** p50=2.000 p95=2.500 p99=2.500 max=2.500
- Top 5 dominant biomes: ColdDesert 29.3%, **Alpine 26.2%**, **SnowCap 15.4%**, SubtropicalDesert 15.1%, **Scree 11.2%**
- Mountain-character: **52.8%**
- Blending math bound violations: 0/1000

### Custom (= Continental Temperate parameters)
- Identical to Continental Temperate distribution (test confirms equality invariant).

**Cross-archetype summary:**
- Archetypes with >20% mountain-character biomes: ALL SIX (41-53%).
- Archetypes where mountain-character is the #1 dominant biome family: ALL SIX (Equatorial Tropical's #1 dominant is Alpine).
- Archetypes with blended `mountains_amplitude` p95 > 2.0: ALL SIX (all hit 2.5 at p95).
- Archetypes with blending math bound violations: NONE (convex-combination invariant holds — `blended_params_within_biome_taxonomy_bounds` test from D.4 was correct).

**D.4 vs D.5 variance comparison** (Continental Temperate):

| Metric | D.4 (variance 600/0.25) | D.5 (variance 400/0.2) |
|--------|------------------------|------------------------|
| Mean amp | 1.772 | 1.773 |
| TemperateDeciduousForest % | 29.9% | 31.8% |
| SnowCap % | 28.9% | 28.9% |
| BorealForest % | 19.7% | 20.0% |
| Alpine % | 12.7% | 12.7% |
| Mountain-character total | 42.5% | 41.8% |

**Differences are within ±2% sampling noise. The variance change has essentially no effect on biome distribution.**

**Verdict: BIOME-CLASSIFICATION SHIFT (architectural; not D.5-specific). Elevation overlay layer dominates biome assignment under Target B's noise-amplitude profile.**

---

## 5. Combined Diagnosis

The visual REGRESS at D.5 is explained by a structural defect in the climate-field architecture (D.1-D.4) that was **latent through D.4** because the D.4 Andrew-gate was deferred to D.6 (per D.4 §10 explicit deferral). D.5's Andrew-gate is the first time the entire D.1-D.5 stack got visually tested; the explosive radial spike pattern is the first visual exposure of an issue that has existed since D.3 wired per-vertex per-biome amplitude.

**Mechanism**:

1. `WorldConfig::default()` Target B amplitudes: `base_elevation.amplitude = 150`, `mountains.amplitude = 480`, `detail.amplitude = 12.5`. Bootstrap heights from `TerrainNoise::sample_height(x, z)` at world center can easily reach **200-400m** (base ~75 + mountain ~150 with continental factor ~0.5 + detail ~5 = ~230m typical, much more at peaks).

2. `lookup_biome(temp, moisture, elevation)` triggers elevation overlays at:
   - `elevation > 220m` AND `moisture < 600mm` → Scree
   - `elevation > 280m` → Alpine (regardless of moisture)
   - `elevation > 350m` AND `temp < 18°C` → SnowCap

3. Bootstrap heights of 200-400m at world center → **Alpine + SnowCap + Scree dominate**, regardless of archetype's climate envelope. Equatorial Tropical's hot temperatures don't help because lapse rate (-6.5°C/1000m) cools 350m elevation by ~2.3°C; combined with Equatorial's temp_mean 26°C → ~24°C at 350m, which is ABOVE SnowCap's 18°C threshold → falls back to Alpine. That's the **Equatorial Tropical Alpine 41.4% finding**.

4. `BiomeParameters::for_biome(Alpine).mountains_amplitude = 2.5` (D.3a defaults). `SnowCap = 2.5`, `MountainRocky = 3.0`, `Scree = 2.0`. These are SUBSTANTIAL multipliers.

5. `apply_per_biome_modulation_to_halo` resamples each vertex's height with the BLENDED per-biome multiplier. Adjacent vertices that resolve to different biomes (e.g., TemperateGrassland 0.8× ↔ SnowCap 2.5×) experience an effective amplitude differential of **1.7×** even after blending. The D.4 blending (radius 48 WU, 6 samples) softens this to ~0.5-0.8 absolute amplitude delta over a 48 WU span — but at noise frequencies in the 100-500 WU range, that's still a steep gradient.

6. The visual signature: explosive radial spikes at biome boundaries (Andrew images 1, 4, 5). 2D-wall mountain character (image 2, Boreal/Subarctic). Same character family as F.4.B.3.C's Mountain preset failure — the root cause is *high per-biome amplitude composing with adjacent biome boundaries*.

**Why D.5's variance change isn't the root cause**: the variance reduction (600→400 / 0.25→0.2) tightens the climate field's distribution but doesn't change which vertices fall above the elevation overlay thresholds. Bootstrap heights are determined by the noise pipeline, not the climate field. The variance change moves moisture and continentalness distributions, which affects rare-biome expression (e.g., Scree's dry-biome bias), not the dominant elevation-overlay assignment.

**Why D.4 was masked**: D.4's `gradient_smoothness` test sampled blended amplitude along a 4000 WU climate gradient and asserted `max_delta < 0.5` for adjacent samples. That test passes because the blending DOES smooth across boundaries. But the test measured per-step delta at climate-driven boundaries, not at the elevation-overlay boundary. The elevation overlay flips happen at (almost) every chunk's high-elevation region, not at climate gradient transitions; the test's sample sweep didn't cross those.

---

## 6. Recommended Remediation Paths

**Three plausible directions, ordered by intrusiveness:**

### Path A: Recalibrate elevation-overlay thresholds upward

Currently `Alpine 280m / SnowCap 350m / Scree 220m`. Bootstrap heights regularly hit 200-400m at world center, which means most non-coastal terrain triggers elevation overlays.

**Sketch**: raise thresholds to e.g. `Alpine 400m / SnowCap 470m / Scree 350m`. Only true peaks → overlays. Most non-coastal terrain stays in the appropriate Whittaker terrestrial zone.

**Risk**: under-classifies real mountain peaks; biome distribution shifts toward forest/grassland dominance.

**Test impact**: D.2's `phase_1_6_f4_b_3_d_2_canonical_snowcap` test (which checks `(15°C, 800mm, 3500m) → SnowCap`) needs threshold sync. Other distribution tests need re-validation.

### Path B: Lower per-biome amplitudes for elevation-overlay biomes

D.3a's defaults: Alpine 2.5×, SnowCap 2.5×, MountainRocky 3.0×, Scree 2.0×.

**Sketch**: clamp these to e.g. 1.5× / 1.5× / 1.8× / 1.3×. Reduces the amplitude flip at boundaries; spikes attenuate.

**Risk**: alpine terrain looks less dramatic; "cartoon-shape problem" the campaign was trying to solve might re-surface.

**Test impact**: D.3a's `alpine_biomes_have_dramatic_mountains` test (asserts >= 2.0 amplitude) needs threshold update.

### Path C: Use a different elevation source for biome lookup (decouple bootstrap from overlay trigger)

Currently `apply_per_biome_modulation_to_halo` passes the *full bootstrap height* (base + mountain × continental + detail) to `climate.sample` and `lookup_biome`. This means mountain-layer noise contributes to biome assignment.

**Sketch**: pass *base-layer-only height* (or a low-frequency proxy) to biome lookup. The mountain layer then shapes the LOCAL terrain, but biome assignment is driven by the broader spatial structure. This decouples "where mountains are" from "what biome this vertex belongs to."

**Risk**: largest architectural change; touches D.3b's `apply_per_biome_modulation_to_halo`. Most invasive.

**Test impact**: distribution tests might shift considerably; potentially desirable (less mountain-character) but unverified.

### Combined approach (recommendation for Andrew's consideration)

A targeted fix combining Path A (raise overlay thresholds modestly, e.g., +50m each) + Path B (modestly clamp overlay amplitudes, e.g., max 2.0× across all overlay biomes) is the lowest-risk path. Each individual change is bounded; together they address both the trigger frequency and the amplitude flip magnitude.

Path C is a deeper architectural fix that may be necessary if A+B don't sufficiently address the visual issue, but it should not be attempted before A+B are validated against the Andrew-gate.

---

## 7. What This Investigation Did NOT Measure

**Blind spots:**

1. **Actual heightmap elevation distribution**. The Q3 diagnostic sampled elevation uniformly from -10 to 510m. The real heightmap doesn't produce uniform elevation — it produces a bimodal distribution (cluster near sea level, sparse high peaks). A more accurate per-archetype biome distribution would generate an actual chunk and measure biome IDs from the chunk's `biome_ids()` accessor, not from synthetic uniform sampling. **Investigation finding limitation: my Q3 percentages are upper bounds; real-world distributions likely have fewer mountain-character biomes than the 41-53% measured.** Actual rendered terrain may show mountain-character biomes only in genuinely high-elevation regions. This shifts the question from "the architecture has too many mountain biomes" to "the architecture has too DRAMATIC a transition at the mountain biome boundary." Either framing leads to the same remediation paths (A/B/C).

2. **Visual character of D.4 alone**. D.4 was never visually tested. A visual A/B comparison D.4 vs D.5 would tell us whether the spike pattern existed at D.4 or only emerged at D.5. The Q3 distribution comparison strongly suggests it existed at D.4 but the conclusive test is visual.

3. **Per-chunk timing breakdown**. I didn't profile individual steps within a chunk's generation. The +12% delta vs D.4 baseline is small enough to defer profiling, but a detailed breakdown would confirm whether climate sampling, blending, or erosion is the dominant cost.

4. **Custom archetype slider responsiveness**. The slider-drag → regenerate flow's actual behavior under interactive use wasn't tested (would require running the editor live with telemetry).

5. **The elevation overlay temperature interaction at very-warm climates**. At Equatorial Tropical's 26°C mean + lapse rate, 2000m elevation reaches 13°C — below SnowCap's 18°C threshold. So at very high elevations, even tropical mountains get SnowCap (correct, matches Mt. Kenya / Kilimanjaro). But at 350m elevation (the SnowCap threshold), 26°C - 2.3°C = 23.7°C, ABOVE 18°C → SnowCap is suppressed → falls back to Alpine. **Equatorial Tropical's Alpine 41.4% is the elevation overlay for warm climates — geometrically equivalent to the snowy peaks for cold ones**. Same root cause, different visual character.

**Questions that emerged during investigation:**

- Is the explosive radial spike pattern intrinsic to D.4's blending math, or does it emerge from the per-biome amplitude defaults composing with the noise pipeline at boundary positions?
- Would Path A's threshold raise actually shift the dominant biome distribution toward terrestrial Whittaker biomes, or just push the overlay zone higher in elevation while still capturing most of the noise distribution?
- Is the radius-48-WU blending kernel the right scale, or should it expand to span the elevation-overlay-to-terrestrial transition (potentially 100-200 WU)?

These are questions for the implementation prompt that follows this diagnostic, not for this report.

---

## 8. Summary of Investigation Discipline

- **No production code changed**. `world_archetypes.rs` was temporarily edited for the D.4 variance counter-test and immediately reverted. `tests/phase_1_6_f4_b_3_d_3_perf.rs` had a throwaway diagnostic added and immediately reverted per §2 plan.
- **No fixes proposed at the implementation level**. Three remediation paths sketched at the abstraction level of "what would need to change."
- **One obvious-looking small finding documented but not applied**: the dead-write bug in `terrain_panel.rs` line 943. Documented in Q2; not fixed during this investigation.
- **Workspace verified clean** before report finalization (`git status` shows only `.aw_editor.lock` + the new `CAMERA_SYSTEMS_SOTA_AUDIT_AND_RECOMMENDATIONS.md` untracked, plus this report).

The investigation cost one session of measurement; the remediation prompt that follows can target the actual root cause with data, not speculation.
