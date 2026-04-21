# Phase 1.5 Tuning Investigation — Heightmap Y-range Diagnostic

**Date:** 2026-04-20
**Context:** Phase 1.5-T of the Terrain Material System Campaign tuning pass. After Phase 1.5 landed (commit `77bd4adf6`), visual inspection suggested Beach/Grassland dominance with Forest and Mountain underrepresented. Before retuning, the actual heightmap Y-range was measured via an integration test so the elevation band constants could be retuned against data rather than guessed.
**Method:** `TerrainState::configure(12345, "grassland") + generate_terrain(5)` driven from `tools/aw_editor/tests/phase_1_5_heightmap_diagnostic.rs` (temporary infrastructure, removed at 1.5-T.E closeout). Aggregated per-vertex Y statistics across all 121 generated chunks.

---

## Measured Y-range (seed 12345, grassland, chunk_radius 5)

| Metric | Value |
|---|---|
| chunks | 121 (11×11) |
| vertices | 526,592 |
| Y min | −3.84 |
| Y max | +121.38 |
| Y span | 125.22 |
| Y mean | +30.99 |

**Distribution shape:** broadly normal with a long right tail. ~21% of vertices sit below Y=8.68, ~35% between Y=8.68–33.73, ~35% between Y=33.73–71.29, ~9% above Y=71.29, and <1% above Y=90.

## Chunk-radius decision

**Kept at 5** (editor default). The measured 125-unit Y span comfortably supports a four-biome elevation distribution without growing the chunk grid. Phase 1.5-T.B was skipped.

## Band retune — before vs. after

Dominant-biome-per-vertex (Temperate climate, same heightmap):

|  | Beach | Grassland | Forest | Mountain |
|---|---:|---:|---:|---:|
| Before (commit `77bd4adf6`) | 16.77% | 12.47% | 26.75% | 44.01% |
| After (commit `990dbac63`)  | 18.26% | 12.05% | 38.91% | 30.79% |

**Key changes (all peaks relative to SEA_LEVEL = 2.0):**

- **Beach** `width 3.0 → 4.0` — more robust at rel < 1.5 so it continues to dominate at sea-level-plus across all coastal climates.
- **Grassland** `peak 8 → 10`, `width 6 → 8` — re-centered and widened.
- **Forest** `peak 22 → 24`, `width 10 → 20` — the headline change; Forest now covers a full mid-elevation slice (~Y 14–44) rather than a narrow 10-unit stripe.
- **Mountain** `HighPass start 30 → 38`, `ramp 30 → 22` — plateau starts higher; less dominance of the transitional zone.

Cold, Arid, Tropical, Wetland, Highland retuned proportionally; Cold/Arid mid-biomes narrowed from width 30 → 25 so their tails no longer out-dominate Beach near sea level.

## Verification

- All 7 `elevation_biome` unit tests pass unchanged with the new constants.
- All three `cargo check` invocations pass (all-features, default, postfx+textures fallback).
- `cargo test -p astraweave-terrain --lib`: 657 passed.
- `cargo test -p aw_editor --lib`: 3945 passed.
- `cargo build -p aw_editor --release`: clean build.

Visual verification is Andrew's interactive gate per §0 discipline.

---

## Correction (2026-04-21, added during Phase 1.6-F.1 execution)

The measured Y-range reported above (125-unit span, min `−3.84`, max
`+121.38`) was taken by a diagnostic test that called
`TerrainState::configure(12345, "grassland") + state.generate_terrain(5)`
**without** applying `state.apply_biome_noise_preset(&preset)` first —
so it measured the effect of `NoiseConfig::default()` (amplitudes
50 / 80 / 5) rather than the runtime grassland `BiomeNoisePreset`
that the editor's `regenerate_terrain` path uses (pre-F.1 amplitudes
35 / 15 / 5). The editor's actual runtime Y span for seed `12345`
grassland-primary was ~40 units, substantially less than this document
records.

Downstream implication at the time: Phase 1.5's `elevation_biome.rs`
band constants (retuned in commit `990dbac63` per the "Band retune"
table above) were tuned for a 125-span terrain but the editor produced
a 40-span terrain, so Forest's band peak at `rel = 24` sat above
nearly every vertex and Mountain's HighPass plateau was unreachable.
This was the root cause of the "invisible Forest/Mountain" Issue 2
from the parent campaign's Phase 1 re-cleanup.

**Fix landed in Phase 1.6-F.1** (commits `fff581aa4` and `a05b856d8`,
per `docs/current/TERRAIN_GENERATION_QUALITY_CAMPAIGN.md` §3): the
eight `BiomeNoisePreset` entries in `tools/aw_editor/src/panels/
terrain_panel.rs` were retuned so each produces a runtime Y span
appropriate for its climate character. The grassland-primary runtime
span is now 116 units; the `elevation_biome.rs` band constants were
left unchanged — Phase 1.6-F's decision was to tune the presets to
fit the bands rather than the bands to fit the presets.

Per-preset runtime Y spans at seed 12345, radius 5, measured by the
F.1 temporary diagnostic test (removed at F.1.C closeout): mountain
252.2, tundra 192.8, grassland 116.0, forest 75.9, desert 75.7,
swamp 72.0, river 68.5, beach 65.3.

See `docs/audits/heightmap_generator_audit_2026-04-21.md` §5.2 and
§6 for the original finding, and
`docs/current/TERRAIN_GENERATION_QUALITY_CAMPAIGN.md` §3 for F.1's
scope.
