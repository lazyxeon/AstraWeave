# AstraWeave World Scale Conventions

## World unit convention

**Decision (2026-04-24, provisional):** **1 world unit (WU) = 1 meter**, provisional until F.4.B.2's implementation pass formalizes via applied code + documented assumptions. Phase 1.6-F.4.B.1 investigation established this as the implicit authored intent; F.4.B.2 will unify the scatter system's hidden 14× tree multiplier (which currently obscures the convention) against the rest of the engine.

**Evidence for 1 WU = 1 meter:**

- Camera code (`tools/aw_editor/src/viewport/camera.rs:109-110`) documents `near: 0.5` and `far: 5000.0` with "meters" doc comments — implying the author-intended convention.
- Tree assets (`astraweave-terrain/src/biome.rs` `tree_small_02_a.glb`) have raw glTF bounding-box height of 3.689 units, consistent with author-intended 3.7m saplings / small trees in native Blender convention (1 Blender unit = 1m is standard).
- Fog parameters (`astraweave-render/src/volumetric_fog.rs`) assume scene extents on order of 1000 WU — consistent with kilometer-scale worlds at 1 WU = 1m.

**Counter-evidence (to be reconciled in F.4.B.2):**

- Scatter system (`tools/aw_editor/src/terrain_integration.rs:2532`) applies a hidden **14× render multiplier** on tree assets, rendering them at 37-79 WU effective height. If 1 WU = 1m, rendered trees are 37-79m — taller than most real trees (typical mature temperate forest tree: 20-30m). This suggests the scatter author saw asset sizing as "too small against terrain" and hacked a compensating scalar.
- F.4.B.2 resolution: reduce tree render multiplier to ~3-5×, increase mountain amplitude 5-8× (Target A from diagnostic) to restore natural peak-to-tree ratio. Unifies the convention at 1 WU = 1m.

**Implications at 1 WU = 1 meter (current state, pre-F.4.B.2):**

- Chunk extent (256 WU) = **256 m** real-world.
- Vertex spacing (4 WU) = **4 m** real-world.
- Post-erosion Y span (82-101 WU depending on climate) = **82-101 m** real-world relief.
- Radius-5 total world (2816 × 2816 WU) = **2.8 × 2.8 km = 7.93 km²** real-world.
- Tree rendered (post-14×-hack): **37-79 m** (unrealistic; will be 8-20m post-F.4.B.2).

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

**Target: PENDING ANDREW'S DECISION.**

`docs/audits/terrain_scale_diagnostic_2026-04-24.md` Task 3 presents three targets with concrete numeric recommendations:

- **Target A — Appalachian / Rolling (default):** 10-30 km², 300-600m Y span, ratio 0.02-0.05. Radius 7 + Y amp ×5-8 + tree mult 14×→3-5×. Achievable without rayon/streaming. Geometrically matches NC lore reference.

- **Target B — Stylized Open World:** 24-64 km², 800-1500m Y span, ratio 0.05-0.10. Radius 8-10 + Y amp ×10-20 + vertex density 64→96. Rayon parallelization required. Matches Skyrim/Enshrouded aesthetic.

- **Target C — Alpine / Crimson Desert:** 50-150 km², 2000-4000m Y span, ratio 0.10-0.20. Requires streaming / progressive generation. Separate campaign territory.

Andrew chooses in F.4.B.2 prompt; this document is updated once decision lands.

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

- **2026-04-24:** Initial conventions doc created during F.4.B.1 scale diagnostic. Provisional 1 WU = 1 meter convention established based on camera comments + tree asset author intent. 14× scatter multiplier flagged as inconsistent.
- **[future: F.4.B.2]:** apply scale change per Andrew's target selection. Update "Implications" section with new chunk/radius/Y-span/tree-multiplier values.
- **[future: F.5]:** if integration tuning reveals further scale refinements, update.

## References

- `docs/audits/terrain_scale_diagnostic_2026-04-24.md` — full diagnostic this convention is derived from.
- `docs/current/TERRAIN_GENERATION_QUALITY_CAMPAIGN.md` — campaign plan; §2.6 continental modulation, §1.5 elevation bands.
- `astraweave-terrain/src/noise_gen.rs` — amplitude configuration.
- `astraweave-terrain/src/elevation_biome.rs` — elevation band thresholds.
- `astraweave-terrain/src/biome.rs` — scatter config.
- `tools/aw_editor/src/terrain_integration.rs` — render-time multipliers, terrain generation orchestration.
- `tools/aw_editor/src/viewport/camera.rs` — camera defaults.
- `astraweave-render/src/volumetric_fog.rs` — fog configuration.
