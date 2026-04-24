# AstraWeave World Scale Conventions

## World unit convention

**Decision (2026-04-24, CONFIRMED via F.4.B.2):** **1 world unit (WU) = 1 meter**. Phase 1.6-F.4.B.2 applied the convention: tree render multiplier reduced 14× → 4× (`tools/aw_editor/src/terrain_integration.rs:2532`) so trees render at realistic 12-21 m mature-forest scale; terrain amplitudes and world extent scaled to match (Target B: 115 km² world, ~500 m Y span).

**Evidence for 1 WU = 1 meter:**

- Camera code (`tools/aw_editor/src/viewport/camera.rs:109-110`) documents `near: 0.5` and `far: 5000.0` with "meters" doc comments — implying the author-intended convention.
- Tree assets (`astraweave-terrain/src/biome.rs` `tree_small_02_a.glb`) have raw glTF bounding-box height of 3.689 units, consistent with author-intended 3.7m saplings / small trees in native Blender convention (1 Blender unit = 1m is standard).
- Fog parameters (`astraweave-render/src/volumetric_fog.rs`) assume scene extents on order of 1000 WU — consistent with kilometer-scale worlds at 1 WU = 1m.

**Counter-evidence (to be reconciled in F.4.B.2):**

- Scatter system (`tools/aw_editor/src/terrain_integration.rs:2532`) applies a hidden **14× render multiplier** on tree assets, rendering them at 37-79 WU effective height. If 1 WU = 1m, rendered trees are 37-79m — taller than most real trees (typical mature temperate forest tree: 20-30m). This suggests the scatter author saw asset sizing as "too small against terrain" and hacked a compensating scalar.
- F.4.B.2 resolution: reduce tree render multiplier to ~3-5×, increase mountain amplitude 5-8× (Target A from diagnostic) to restore natural peak-to-tree ratio. Unifies the convention at 1 WU = 1m.

**Implications at 1 WU = 1 meter (post-F.4.B.2 Target B):**

- Chunk extent (512 WU) = **512 m** real-world (was 256).
- Vertex spacing (5.39 WU) = **5.39 m** real-world (was 4).
- Post-erosion Y span (~510 WU at default slider) = **~510 m** real-world relief (was 82-101).
- Radius-10 total world (10752 × 10752 WU) = **10.75 × 10.75 km = 115.58 km²** (was 7.93).
- Tree rendered: **11.8-20.6 m** at `tree_small_02_a.glb` (was 37-79 m). Peak-to-tree ratio ~30× at Target B default, matching Enshrouded baseline.
- Mountain Drama slider (0.4-2.0, default 1.0) scales mountains amplitude only — at 0.4 = gentle hills (Target A territory); at 2.0 = alpine peaks (Target C-ish without streaming).

## Reference-title comparison table

Source: `docs/audits/terrain_scale_diagnostic_2026-04-24.md` Task 2.D. Full provenance and URLs in the diagnostic document.

| Dimension          | AstraWeave (current)| AstraWeave (Target A) | Skyrim    | Witcher 3  | Enshrouded EA/1.0 | RDR2        | NC Blue Ridge 100 km transect |
|--------------------|--------------------:|----------------------:|----------:|-----------:|------------------:|------------:|------------------------------:|
| Chunk / cell side  | 256 WU (256 m)      | 256 WU (256 m)        | 57.6 m    | ~186 m     | not doc.          | not doc.    | N/A                           |
| Vertex spacing     | 4 WU (4 m)          | 4 WU (4 m)            | ~1.8 m    | ~0.37 m    | not doc.          | not doc.    | 30 m (SRTM) / 1 m (LiDAR)     |
| Total extent (km²) | 7.93                | 14.75 (radius 7)      | 37        | ~74        | 24 / 64           | ~75         | 100 km transect               |
| Max vertical relief| 92 m                | ~400 m                | ~766 m    | not doc.   | not doc.          | not doc.    | 1820 m                        |
| Vert/horiz ratio   | 0.033               | ~0.10                 | 0.018     | not doc.   | not doc.          | not doc.    | 0.018                         |

**Verdict from diagnostic:** AstraWeave's geometric character (ratio) is fine. Absolute scale is ~20-25% of Skyrim and ~10% of larger AAA open worlds. Target A restores absolute scale to Appalachian / low-mountain range without requiring architectural changes (rayon or streaming).

## Aesthetic target for Veilweaver

**Target: B (Stylized Open World / Enshrouded-class) — IMPLEMENTED 2026-04-24 via F.4.B.2.**

- 115.58 km² world extent (radius 10 × 512 WU chunks).
- ~500 m post-erosion Y span (matches Enshrouded-class peaks).
- Ratio 0.047 — in plan §2.3's Target B 0.05-0.10 bracket.
- Peak-to-tree ratio ~30× (Enshrouded baseline).
- Mountain Drama slider (0.4-2.0, default 1.0) provides user-tunable dramatic-ness within Target B envelope without requiring re-scale.

**Target A (Appalachian) recoverable via slider:** setting Mountain Drama to 0.4 brings `mountains_amplitude` back to Target A range (~200 m Y span) while keeping horizontal extent at 115 km². Users wanting full Target A should ALSO reduce radius 10 → 6 for 4-5 km² extent.

**Target C (Crimson-Desert class) explicitly deferred to Phase 1.7 Streaming Terrain campaign:** requires sparse virtual textures, progressive generation, scatter impostors. Not within F.4/F.5 scope.

## Knobs and their relationships

- **Chunk extent** (WU): how much world one chunk covers. Currently **256**. Increasing expands horizontal extent linearly but increases halo erosion work quadratically (halo is 3×3 chunks — larger chunk → 9× more vertices per halo).
- **Vertex density** (vertices per chunk side): currently **64**. Vertex spacing = `chunk_extent / (vertex_count - 1)`. Increasing adds memory + GPU cost quadratically; increases per-chunk erosion cost quadratically; does NOT expand world extent.
- **Radius**: number of chunk rings. Total chunks = `(2×radius + 1)²`. Currently **5** (121 chunks). Increasing expands world extent quadratically with linear memory + erosion-time scaling.
- **Y amplitude** (preset `base_amplitude`, `mountains_amplitude`, `detail_amplitude`): drives peak height. Currently per-preset in `astraweave-terrain/src/noise_gen.rs` + `tools/aw_editor/src/panels/terrain_panel.rs:1816-1991`. Scaling is cheap (no extra compute), but F.2-T-4's regression test `phase_1_6_f2_t_highland_regions_reach_f1_target` encodes absolute values — needs recalibration if amplitudes shift.
- **Continental scale** (`continental_scale` in `NoiseConfig`, currently 0.0012 = ~830 WU wavelength): drives regional highland/lowland clustering per §2.6. Must scale with world extent; if radius grows, continental wavelength should grow too or the continental feature collapses to single-cycle modulation.
- **Elevation bands** (`astraweave-terrain/src/elevation_biome.rs` `TEMPERATE_BANDS`, `COLD_BANDS`, etc.): thresholds for biome-weight slots along Y. Tuned for current ~100 WU Y span; must be recalibrated if amplitude changes.
- **Tree render multiplier** (`tools/aw_editor/src/terrain_integration.rs:2532` currently 14.0 for trees): per-type scalar applied at rendering. Not a terrain parameter per se, but directly affects perceived peak-to-tree ratio. Part of F.4.B.2's scope if scale changes materially.

Changing ANY knob in isolation has known effects (§2.1, §2.6, F.3 halo sizing, elevation bands). Changing MULTIPLE knobs requires a calibration pass — particularly for:

- **Continental scale** when world extent changes.
- **Elevation bands** when Y amplitude changes.
- **Tree render multiplier** when either world extent or Y amplitude changes substantially.
- **F.2 regression tests** (highland Y target, spike curvature) when any of the above changes, because they encode absolute values.

## Phase 1.6 history of scale-relevant decisions

- **F.1 (2026-04-21):** amplitude tuning for eight biome presets. Grassland Y span target ≥ 100 units (Phase 1.5's intended elevation-band range).
- **F.2 (2026-04-21):** DomainWarped noise + continental-scale modulation (§2.6). Continental wavelength 830 WU matches radius-5 extent.
- **F.2-T / F.2-T-2 / F.2-T-3 / F.2-T-4 (2026-04-21/22):** surface-spike remediation (derivative-weighted fBm landed in F.2-T-4). Curvature 2.016 → 0.576 cumulative. Highland Y max stabilized at 96.04 WU on 121-chunk grid. Encoded as `phase_1_6_f2_t_highland_regions_reach_f1_target` and `phase_1_6_f2_t2_surface_spikiness_under_threshold` permanent regression tests.
- **F.3-phase-0 / F.3-phase-1 / F.3-phase-2 / F.3-phase-3 / F.3-phase-4 (2026-04-23/24):** AdvancedErosionSimulator wiring, halo scaffolding, world-coord droplet seeding, shared-vertex averaging, droplet-count reduction. Net effect on scale: post-erosion Y max 82-101 WU depending on climate (down from pre-erosion 121.9 WU).
- **F.4.B.1 (2026-04-24, this document):** scale diagnostic. Established this conventions document + Target A as default recommendation.

## Changelog

- **2026-04-24 (F.4.B.1):** Initial conventions doc created. Provisional 1 WU = 1 meter convention established based on camera comments + tree asset author intent. 14× scatter multiplier flagged as inconsistent.
- **2026-04-24 (F.4.B.2):** Convention CONFIRMED via implementation. Target B applied: chunk 512 WU, vertex density 96, radius 10, amplitudes ×3-8 (per-preset), continental scale 0.0012 → 0.0003, tree multiplier 14 → 4, elevation bands ×5, F.2 regression tests recalibrated. Rayon parallelization lands (phase-2.E's deferred work). Mountain Drama slider (0.4-2.0) added for dramatic-ness tuning without re-scaling. World extent grows 7.93 km² → 115 km²; Y span 92 m → ~510 m; trees 37-79 m → 12-21 m.
- **[future: F.4.A]:** climate-as-spatial-field. Bands and continental scale are now Target B-calibrated; F.4.A extends the biome-weight API to per-vertex ClimateSample without re-tuning scale.
- **[future: F.5]:** integration tuning Andrew-gate evaluates all eight climates at Target B. Per-climate tuning adjustments (e.g. mountain preset's ×8 might be too dramatic, Mountain Drama default ≠ 1.0) landed as 1-parameter tweaks.
- **[future: Phase 1.7 Streaming]:** Target C (Crimson-Desert class) if authored intent demands it.

## References

- `docs/audits/terrain_scale_diagnostic_2026-04-24.md` — full diagnostic this convention is derived from.
- `docs/current/TERRAIN_GENERATION_QUALITY_CAMPAIGN.md` — campaign plan; §2.6 continental modulation, §1.5 elevation bands.
- `astraweave-terrain/src/noise_gen.rs` — amplitude configuration.
- `astraweave-terrain/src/elevation_biome.rs` — elevation band thresholds.
- `astraweave-terrain/src/biome.rs` — scatter config.
- `tools/aw_editor/src/terrain_integration.rs` — render-time multipliers, terrain generation orchestration.
- `tools/aw_editor/src/viewport/camera.rs` — camera defaults.
- `astraweave-render/src/volumetric_fog.rs` — fog configuration.
