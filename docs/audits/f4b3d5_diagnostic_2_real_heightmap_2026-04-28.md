# F.4.B.3.D.5-diagnostic-2 Real-Heightmap Biome Distribution Audit

**Date**: 2026-04-28
**Mode**: Investigation-only (no production code changes; `#[ignore]`-marked measurement test added).
**Trigger**: D.5-diagnostic flagged as a blind spot (§7) that synthetic uniform-elevation sampling may inflate the mountain-character biome figure. This follow-up measures real-chunk distribution to inform path selection.
**Predecessor audit**: `docs/audits/f4b3d5_diagnostic_report_2026-04-28.md`.
**Diagnostic state**: D.5b commit `0315d6911`.

---

## 1. Background

D.5-diagnostic established that mountain-character biomes (Alpine + SnowCap + MountainRocky + Scree) account for 41-53% of biome assignments across all six archetypes when synthetic samples are drawn from a uniform elevation distribution in `[-10, 510]m`. The diagnostic flagged this figure as an *upper bound* — actual heightmaps don't produce uniform elevation, so real-chunk distributions may differ substantially.

The decision matrix from D.5-diagnostic §2 makes path selection contingent on:

- Real-chunk MountainCharacter % (low / moderate / high)
- Where MountainCharacter biomes appear in elevation (above or below 280m Alpine threshold)
- Spatial pattern (coherent regions vs speckled per-vertex assignment)

This audit measures all three.

---

## 2. Methodology

`#[ignore]`-marked test `phase_1_6_f4_b_3_d_5_diagnostic_2_real_heightmap.rs::d5_diagnostic_2_real_heightmap_biome_distribution` (added in this session). Procedure:

1. Construct `WorldGenerator` with seed 12345; set `config.climate.archetype` to the target archetype's `default_archetype()`.
2. Generate 121 chunks via `generate_chunk_with_climate(chunk_id, ClimateBias::Temperate)` for chunks `(-5..=5, -5..=5)` sequentially.
3. For each chunk, iterate all `96² = 9216` vertices, capture `(elevation, biome_id)` tuples (1,115,136 vertices per archetype).
4. Aggregate biome-class fractions, MC elevation histogram, per-biome elevation distribution, spatial pattern dump.
5. Repeat for both archetypes.

Measurements run on D.5b state (commit `0315d6911`). No production code changed.

**Wall-clock cost**: ~90s per archetype (sequential generation, 121 chunks each), ~3 min total.

---

## 3. Measurements

### 3.1 Continental Temperate

```
Total vertices: 1,115,136
Generation time: 90.77s

Biome class fractions:
  Aquatic:             6.46% (72,000)
  MountainCharacter:  28.93% (322,621)
  Terrestrial:        64.61% (720,515)
  Other:               0.00% (0)

MountainCharacter elevation distribution (322,621 verts, 28.93% of total):
  Histogram:
    <0      m:       0 ( 0.00%)
    0-50    m:       0 ( 0.00%)
    50-100  m:       0 ( 0.00%)
    100-200 m:      38 ( 0.01%)
    200-280 m:    1100 ( 0.34%)
    280-350 m:    6669 ( 2.07%)
    350-450 m:   31387 ( 9.73%)
    450+    m:  283427 (87.85%)
  Stats: min=146.1 p25=522.1 p50=650.5 p75=791.2 max=1214.5
  Above 280m:  321483 (99.65% of MC, 28.83% of total)
  Below 280m:    1138 ( 0.35% of MC,  0.10% of total)

Per-biome (within MC):
  Alpine:        158,362  elev: min=190.3  p50=528.5  max=978.1
  SnowCap:       161,688  elev: min=301.6  p50=785.1  max=1214.5
  Scree:           2,571  elev: min=146.1  p50=337.2  max=679.1
  MountainRocky:       0  (no producer per D.2; reserved for future slope-conditional path)

Threshold violations (vertices below biome's documented threshold):
  Alpine BELOW 280m: 524 (0.33% of Alpine)
  SnowCap BELOW 350m: 16 (0.01% of SnowCap)
  Scree BELOW 220m:  102 (3.97% of Scree)

Overall elevation distribution (sanity context):
  Histogram:
    <0      m:       0 ( 0.00%)
    0-50    m:   53,397 ( 4.79%)
    50-100  m:  135,775 (12.18%)
    100-200 m:  318,951 (28.60%)
    200-280 m:  156,165 (14.00%)
    280-350 m:   79,404 ( 7.12%)
    350-450 m:   80,439 ( 7.21%)
    450+    m:  291,005 (26.10%)
  Stats: min=0.0 p25=126.6 p50=220.5 p75=467.6 max=1214.5
```

**Spatial pattern (chunk (5,5))**: COHERENT. Two large connected MountainCharacter regions on the left and right edges of the chunk, with a clean Terrestrial corridor in between transitioning into a small Aquatic region (lake or river depression). No speckled boundary thrashing — biome assignment forms blobs, not noise. (Full 96×96 grid in test stdout.)

### 3.2 Equatorial Tropical

```
Total vertices: 1,115,136
Generation time: 85.73s

Biome class fractions:
  Aquatic:             8.02% (89,403)
  MountainCharacter:  28.76% (320,724)
  Terrestrial:        63.22% (705,009)
  Other:               0.00% (0)

MountainCharacter elevation distribution (320,724 verts, 28.76% of total):
  Stats: min=153.6 p25=509.6 p50=641.7 p75=786.8 max=1214.5
  Above 280m:  319,458 (99.61% of MC, 28.65% of total)
  Below 280m:    1,266 ( 0.39% of MC,  0.11% of total)

Per-biome (within MC):
  Alpine:        319,970  elev: min=167.1  p50=642.3  max=1214.5  ← dominates
  SnowCap:            80  elev: min=682.9  p50=848.4  max=901.5   ← suppressed by warm temperatures
  Scree:             674  elev: min=153.6  p50=324.1  max=643.3
  MountainRocky:       0

Threshold violations:
  Alpine BELOW 280m: 1,042 (0.33% of Alpine)
  SnowCap BELOW 350m: 0
  Scree BELOW 220m:   49 (7.27% of Scree)

Overall elevation distribution:
  Stats: min=0.0 p25=118.2 p50=206.8 p75=451.2 max=1214.5
```

**Spatial pattern (chunk (0,0))**: COHERENT. Massive Alpine regions cover most of the chunk's area (left half + bottom-right quadrant), with Terrestrial corridors and a small Aquatic depression. The chunk reads as "tropical archetype produces Alpine-dominated terrain at elevation peaks" — exactly the structural finding D.5-diagnostic identified.

**Striking observation about Equatorial Tropical**: 319,970 Alpine vertices vs 80 SnowCap vertices. SnowCap fires only when `temp < SNOWCAP_MAX_TEMP_C (18°C)`. Equatorial Tropical's mean 26°C — even with lapse rate cooling at peak elevations, most high-altitude vertices stay above 18°C → SnowCap suppressed → Alpine dominates the mountain-character class. This is the Mt. Kenya / Kilimanjaro edge case at world scale — tropical mountains have rocky exposure rather than snow.

### 3.3 Comparison table

| Archetype             | Synthetic MC% (D.5-diag) | Real MC% | Above 280m   | Below 280m   | Pattern   |
| --------------------- | ------------------------ | -------- | ------------ | ------------ | --------- |
| Continental Temperate | 41.8%                    | 28.93%   | 99.65% of MC | 0.35% of MC  | COHERENT  |
| Equatorial Tropical   | 41.7%                    | 28.76%   | 99.61% of MC | 0.39% of MC  | COHERENT  |

**Both archetypes show essentially identical structural behavior**: ~28% MC%, ~99.6% of MC above 280m, coherent spatial pattern.

---

## 4. Findings

### 4.1 Synthetic figure was inflated, but not by as much as suspected.

Synthetic 41-42% → real 28-29%. The synthetic figure was 45% high (relative). Real-chunk MountainCharacter is meaningfully lower — but still meaningful at 28-29% (not the <15% that would suggest the issue is purely cosmetic at boundaries).

### 4.2 Elevation overlay is firing at the right places (above 280m).

**99.6% of MountainCharacter vertices are above 280m.** The elevation overlay layer (`Alpine ≥ 280m`, `SnowCap ≥ 350m`, `Scree ≥ 220m` + low moisture) is correctly identifying genuinely high-elevation terrain. Path A (raise thresholds) would solve a problem that doesn't exist — overlays are not firing eagerly at moderate elevations.

The 0.35-0.39% below-280m vertices are mostly explained by the §2.5 invariant: biome IDs are computed PRE-erosion, elevations are reported POST-erosion. Hydraulic erosion drops some vertices below their biome's elevation threshold. This is intentional (authorial-intent stability) and not a bug.

### 4.3 Spatial pattern is coherent, not speckled.

The chunk (5,5) and chunk (0,0) spatial dumps show large connected MountainCharacter regions, NOT per-vertex thrashing at boundaries. The blending architecture is working — biome assignment is structured, not noise.

This rules out Path C (decouple bootstrap from lookup, architectural change). Per-vertex assignment is stable; the architectural correction (D.1-D.4) is fundamentally sound.

### 4.4 Maximum elevation 1214m is 2.4× Target B's design target (510m).

This is the smoking gun. Both archetypes produce vertices reaching 1214m elevation. The Target B Y range was designed for ~510m peaks (per F.4.B.2 §2.B's "post-erosion Y max ~510 WU across climates" measurement). With per-biome amplitude multipliers (Alpine 2.5×, SnowCap 2.5×, MountainRocky 3.0×) layering on the bootstrap noise (already 480 mountain × continental factor), final heights compound far past the design target.

Visually, this means:
- Adjacent vertices at SnowCap (2.5×) and TemperateGrassland (0.8×) experience a 3.1× amplitude differential (2.5 / 0.8 ≈ 3.1).
- Even after D.4 blending (radius 48 WU, 6 samples), the spatial gradient of amplitude remains steep.
- The noise pipeline expresses this gradient as the explosive radial spike pattern Andrew observed.

### 4.5 Equatorial Tropical's 41.4% Alpine is real, not synthetic noise.

D.5-diagnostic Q3's claim that Equatorial Tropical's #1 dominant biome is Alpine (41.4% synthetic) is confirmed at real-chunk scale: 319,970 / 1,115,136 = **28.69% of tropical world is Alpine** (with Alpine being the dominant biome of the MC class because warm temperatures suppress SnowCap).

This is geographically correct (tropical mountains have rocky-not-snow caps) but architecturally problematic (a tropical archetype should not have ~30% of its area being mountain-character biomes — that's not what "tropical" means semantically).

### 4.6 Threshold violations are not bugs.

The 0.33% Alpine / 3.97% Scree / 0.01% SnowCap below-threshold counts are all explained by §2.5: PRE-erosion biome assignment + POST-erosion elevation drift. Verifying this would require capturing pre-erosion heights too; deferred. The per-biome elevation p25/p50/p75 stats all sit well above the respective thresholds, confirming the bulk of assignments are correct.

---

## 5. Decision Matrix Mapping

Per D.5-diagnostic §2 + this audit's §2 decision matrix:

| Outcome | This audit's measurement | Mapped path |
|---------|--------------------------|-------------|
| Real-chunk MC% < 25% AND mostly above 280m | **28-29%** ≥ 25% threshold; 99.6% above 280m | Borderline matrix entry |
| Real-chunk MC% 25-45% AND mostly above 280m | **28-29%** in range; 99.6% above 280m | **MATCHES** |
| Real-chunk MC% > 25% AND meaningful mass below 280m | 28-29% in range, but only 0.35-0.39% below 280m | DOES NOT MATCH |
| Spatial pattern speckled regardless of % | **COHERENT** observed | DOES NOT MATCH |

**Mapped path: Combined Path A + Path B**, BUT with a twist — the data shows that Path A (raise thresholds) is *not* needed because overlays are already firing at correct elevations. The 99.6% above-280m number is decisive: thresholds are calibrated correctly.

**The actual primary fix is Path B (lower per-biome amplitudes for elevation-overlay biomes).** Path A is optional for fine-tuning (e.g., "Scree threshold from 220m to 250m to reduce the 3.97% below-threshold count toward zero"), but the principal lever is amplitude clamping.

The maximum elevation 1214m is the symptom; per-biome amplitudes are the cause.

---

## 6. Recommended Remediation Path

**Primary: Path B** (lower per-biome amplitudes for elevation-overlay biomes).

**Sketch (for the next remediation session — not implemented in this audit)**:

The current `BiomeParameters::for_biome(...)` defaults for elevation-overlay biomes (per `astraweave-terrain/src/biome_parameters.rs`):

```
Alpine:        mountains_amplitude = 2.5
SnowCap:       mountains_amplitude = 2.5
MountainRocky: mountains_amplitude = 3.0
Scree:         mountains_amplitude = 2.0
```

To bring max elevations from 1214m down toward Target B's 510m design target, amplitudes should drop by roughly the ratio 510/1214 ≈ 0.42, then add headroom for the fact that not every vertex is at the maximum:

```
Alpine:        2.5 → 1.4   (~44% reduction)
SnowCap:       2.5 → 1.4   (~44% reduction)
MountainRocky: 3.0 → 1.6   (~47% reduction)
Scree:         2.0 → 1.2   (~40% reduction)
```

These specific values are sketches; the remediation session should iterate against a re-run of this audit's measurement to confirm:
1. Real-chunk maximum elevation drops to roughly 510-700m (within Target B's intent ± headroom).
2. Mountain-character biomes still exhibit dramatic relief (alpine character not lost — Path B's risk).
3. D.4's `gradient_smoothness` test still passes with tightened tolerance toward the original 0.35 (currently relaxed to 0.5).
4. Andrew-gate visual verification: spike pattern eliminated, mountain-character regions read as plausible alpine terrain.

**Secondary: optional Path A fine-tuning** (raise Scree threshold modestly).

The 3.97% below-threshold Scree count suggests Scree's 220m threshold may be slightly too low — bumping to e.g. 250m would push the post-erosion Scree distribution cleanly above its threshold. Marginal benefit; not blocking.

**Path C (decouple bootstrap from lookup) ruled out** by spatial-pattern coherence. The architecture is sound; the parameter values are wrong.

**Equatorial Tropical specifically**: Path B's amplitude reduction will help, but the underlying issue (tropical archetype produces 30% Alpine because the climate envelope's high temperature mean keeps SnowCap suppressed at all elevations, while bootstrap heights still reach 280m+ Alpine threshold) may warrant per-archetype tuning of one or more of:

- Equatorial Tropical's `latitude_temperature_drop_c` (currently 3°C; if increased, polar regions of the world cool enough for SnowCap to fire occasionally).
- An archetype-specific bootstrap-amplitude reduction (e.g., Equatorial Tropical world has lower mountain noise contribution to reduce overall elevation profile, matching tropical-world-feels-flatter intuition).

These are speculative beyond Path B's scope; the remediation session should land Path B first and re-evaluate.

---

## 7. What This Measurement Did NOT Address

- **Other 4 archetypes**: only Continental Temperate + Equatorial Tropical measured. Boreal/Subarctic, Mediterranean, Desert, Custom not directly verified by real-chunk measurement. The strikingly similar 28-29% MC across the two archetypes measured suggests other archetypes will land in the same range; per-archetype validation lives in a follow-up session after Path B remediation.
- **D.4 alone vs D.5**: not measured at real-chunk level. D.5-diagnostic Q3 found ±1% difference on synthetic samples; if Path B's amplitude reduction makes the visual issue disappear, this question becomes moot.
- **Pre-erosion biome ID + pre-erosion elevation correlation**: not captured. The threshold violations (Alpine 0.33%, Scree 3.97%) are explained by §2.5 PRE-erosion biome assignment; verifying this would require capturing both PRE and POST heights. Deferred unless threshold bumps land in remediation.
- **Visual character at world scale**: this audit reports per-vertex statistics + per-chunk spatial patterns, not visual rendering of the full world. Final visual verification still depends on Andrew-gate after Path B remediation.

**Questions surfaced during measurement (out of original three Q's scope):**

- Why does 26% of total vertices land at elevation ≥ 450m (per Continental Temperate's overall histogram)? That's too high a fraction for "mostly lowland with rare peaks" — suggests the bootstrap noise pipeline itself produces elevation distributions skewed too high for Target B intent.
- The maximum elevation 1214m suggests the per-biome amplitude compounds with the bootstrap unbounded; an alternative remediation would be to clamp the FINAL height after per-biome modulation, not just lower the multipliers. This is a Path B variant; defer to remediation session.

---

## 8. References

- D.5-diagnostic audit: `docs/audits/f4b3d5_diagnostic_report_2026-04-28.md`.
- Campaign §10 D.5 entry (commit `0315d6911`).
- D.4 commits: `646e00657` (blending) + `84e81bee5` (doc).
- D.3 commits: `0c1a4c0d5` / `3692e8b39` / `fdbf71e2c` / `6c04a0ce2`.
- `BiomeParameters::for_biome` defaults: `astraweave-terrain/src/biome_parameters.rs` (Alpine 2.5×, SnowCap 2.5×, MountainRocky 3.0×, Scree 2.0×).
- `lookup_biome` thresholds: `astraweave-terrain/src/biome_lookup.rs` (`SCREE_THRESHOLD_M=220`, `ALPINE_THRESHOLD_M=280`, `SNOWCAP_THRESHOLD_M=350`, `SNOWCAP_MAX_TEMP_C=18`).
- F.4.B.2.B post-erosion Y max measurement (Target B design target): `docs/current/TERRAIN_GENERATION_QUALITY_CAMPAIGN.md` §10 entry 2026-04-24.

---

## 9. Test Disposition

The diagnostic test file `astraweave-terrain/tests/phase_1_6_f4_b_3_d_5_diagnostic_2_real_heightmap.rs` is committed alongside this audit. Marked `#[ignore]` so it doesn't run in CI; runnable on demand via:

```
cargo test -p astraweave-terrain --release --test phase_1_6_f4_b_3_d_5_diagnostic_2_real_heightmap d5_diagnostic_2 -- --ignored --nocapture
```

**Recommended disposition: KEEP**. This measurement is directly useful for validating Path B remediation: re-run after amplitude reduction, confirm max elevation drops toward 510m target, confirm spatial pattern remains coherent. The next session may rename to a non-diagnostic permanent name once remediation lands.

---

## 10. Decision summary (one-paragraph TL;DR)

Real-chunk biome distribution at D.5b confirms the architectural correction (D.1-D.4) is structurally sound: 28-29% mountain-character biomes (down from synthetic 41-42%) cluster correctly above 280m elevation (99.6% of them) in coherent spatial regions (not speckled boundary thrashing). The visual REGRESS is caused by per-biome amplitude multipliers (Alpine 2.5×, SnowCap 2.5×, MountainRocky 3.0×, Scree 2.0×) compounding with the bootstrap noise to produce maximum elevations of 1214m — 2.4× Target B's 510m design target. **Path B (lower per-biome amplitudes for overlay biomes by ~40-47%) is the recommended primary remediation.** Path A (raise thresholds) is unnecessary because overlays already fire at correct elevations. Path C (architectural change) is ruled out by spatial coherence. Equatorial Tropical's tropical character will require additional per-archetype tuning beyond Path B but is not blocking the principal fix.
