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
