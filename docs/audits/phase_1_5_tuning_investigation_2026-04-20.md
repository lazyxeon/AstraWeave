# Phase 1.5 Tuning Investigation — Heightmap Y-range Diagnostic

**Date:** 2026-04-20
**Scope:** Phase 1.5-T.A of the Terrain Material System Campaign tuning pass. Measure the actual Y-range produced by the terrain generator for the canonical test case (seed `12345`, `terrain_primary_biome = "grassland"`, chunk radius `5`) so the `elevation_biome` band constants can be retuned against data rather than guessed.
**Method:** Integration test `tools/aw_editor/tests/phase_1_5_heightmap_diagnostic.rs` drives `TerrainState::configure` + `generate_terrain(5)` through the same code path the editor uses, then aggregates per-vertex Y statistics across all generated chunks.
**Verification:** `cargo test -p aw_editor --test phase_1_5_heightmap_diagnostic -- --nocapture`.

---

## Findings summary

**Chunk radius decision: keep at 5.** The heightmap produces a healthy 125-unit Y span at the default radius; no increase is required to support a four-band elevation distribution.

**Root cause of visible Beach/Grassland dominance: band placement, not heightmap deficit.** The heightmap covers `[-3.84, +121.38]` (span 125 units) with mean `+30.99`, but the initial Phase 1.5 bands cluster Forest in a narrow window around rel=22 and push Mountain's plateau to rel=60. The effect is that `44.01%` of vertices fall into Mountain-dominant territory (much of it the wide transitional zone rel=30–60 where smoothstep interpolation is gradual), while Forest only dominates `26.75%` of vertices in a tight band. Visually this reads as mid-elevation slopes sliding gradually through a washed-out Forest/Mountain gradient rather than crisp geological bands.

**Action taken:** 1.5-T.B is skipped (chunk radius unchanged). Retuning in 1.5-T.C reshapes the Temperate bands to produce clearer, more evenly-proportioned visual bands across the measured range.

---

## Raw measurement (seed 12345, primary_biome "grassland", radius 5)

- **Chunks generated:** 121 (11×11)
- **Total vertices:** 526,592 (4,352 per chunk on average)

### Global Y statistics

| Metric | Value |
|---|---|
| `min`  | −3.840 |
| `max`  | +121.380 |
| `span` | 125.220 |
| `mean` | +30.989 |

### 20-bucket Y histogram across `[-3.84, 121.38]` (~6.26 units per bucket)

| Bucket | Y range | Count | % |
|---:|---|---:|---:|
|  0 | −3.84 … 2.42 | 73,336 | 13.93% |
|  1 |  2.42 … 8.68 | 36,563 |  6.94% |
|  2 |  8.68 … 14.94 | 43,313 |  8.23% |
|  3 | 14.94 … 21.20 | 48,681 |  9.24% |
|  4 | 21.20 … 27.46 | 48,818 |  9.27% |
|  5 | 27.46 … 33.73 | 47,534 |  9.03% |
|  6 | 33.73 … 39.99 | 46,120 |  8.76% |
|  7 | 39.99 … 46.25 | 41,277 |  7.84% |
|  8 | 46.25 … 52.51 | 36,861 |  7.00% |
|  9 | 52.51 … 58.77 | 33,352 |  6.33% |
| 10 | 58.77 … 65.03 | 26,769 |  5.08% |
| 11 | 65.03 … 71.29 | 19,053 |  3.62% |
| 12 | 71.29 … 77.55 | 12,176 |  2.31% |
| 13 | 77.55 … 83.81 |  6,758 |  1.28% |
| 14 | 83.81 … 90.07 |  4,049 |  0.77% |
| 15 | 90.07 … 96.34 |  1,336 |  0.25% |
| 16 | 96.34 … 102.60 |  430 |  0.08% |
| 17 | 102.60 … 108.86 |  102 |  0.02% |
| 18 | 108.86 … 115.12 |   44 |  0.01% |
| 19 | 115.12 … 121.38 |   20 |  0.00% |

**Shape:** broadly normal with a long right tail. ~21% of vertices sit below Y=8.68 (buckets 0–1), ~35% between Y=8.68–33.73 (buckets 2–5), ~35% between Y=33.73–71.29 (buckets 6–11), and ~9% above Y=71.29 (buckets 12+). Less than 1% above Y=90.

### Per-chunk sample (first 5 chunks)

| Chunk | min | max | mean | n |
|---|---:|---:|---:|---:|
| (2, 0) | −3.84 | 121.38 | +42.25 | 4,352 |
| (−1, 4) | −3.84 | +87.07 | +35.36 | 4,352 |
| (3, 0) | −3.84 | +54.60 | +30.34 | 4,352 |
| (3, 5) | −3.84 | +73.33 | +27.19 | 4,352 |
| (5, 1) | −3.84 | +88.25 | +33.41 | 4,352 |

Each chunk reaches the global minimum (−3.84 — the seafloor floor). Per-chunk maxima vary widely (54–121) which confirms real terrain variation rather than a flat plain; a chunk with a nearby peak has a max of 121, a chunk next to a lowland has a max of 54.

### Dominant-biome-per-vertex counts (current Phase 1.5 Temperate bands)

| Biome | Count | % |
|---|---:|---:|
| Beach (slot 6)     |  88,285 | 16.77% |
| Grassland (slot 0) |  65,685 | 12.47% |
| Forest (slot 2)    | 140,861 | 26.75% |
| Mountain (slot 3)  | 231,761 | 44.01% |
| other              |       0 |  0.00% |

All four biomes are mathematically represented; 0% "other" confirms no slot-misassignment. The visible "Beach+Grassland dominate" symptom therefore comes from band *placement and smoothstep falloff shape*, not from a missing biome — Forest is mathematically the dominant biome in only 26.75% of vertices despite covering an elevation zone (rel=12–32) that spans 20% of the usable range, because its `width=10` is too narrow relative to the surrounding bands, and Mountain's `start=30` + `ramp=30` pulls mountain dominance early and holds it across a large middle zone where visually the colour is transitional rather than crisp.

---

## Interpretation → band-tuning strategy

Given a healthy measured Y range (useful terrain covering roughly `[0, 85]` with 99% of vertices below Y=90) and `SEA_LEVEL = 2.0`:

**Temperate band targets (post-retune):**

| Biome | New `peak` (rel) | New `width` / ramp | Dominant zone (abs Y) |
|---|---:|---:|---|
| Beach     | 2.0  | 3.0  | ~0 … 5 |
| Grassland | 10.0 | 8.0  | ~4 … 16 |
| Forest    | 24.0 | 20.0 | ~14 … 44 |
| Mountain  | 38.0 start, 22.0 ramp | — | ~40+ |

Rationale: widen Forest from `width=10` to `width=20` so Forest covers a fuller mid-elevation slice; push Mountain's `start` from 30 to 38 so the plateau does not eat the upper Forest zone; keep the same Beach + Grassland lowland shape since those are already well-distributed. The resulting distribution should be roughly Beach 10–15% / Grassland 15–20% / Forest 30–35% / Mountain 30–35% — all four biomes clearly visible, with smooth blended transitions in the ~5-unit overlap windows.

Non-Temperate climates get proportional adjustments preserving each climate's character (Cold keeps Tundra in place of Grassland+Forest, Highland keeps no Beach, etc.) using the same Y range.

Constraints preserved:
- `SEA_LEVEL = 2.0` matches the water system's hardcoded plane per the water audit.
- `ClimateBias::from_primary_biome_str` string mapping unchanged.
- `BandShape::Pulse` / `BandShape::HighPass` shapes unchanged — only numeric constants move.
- Fallback slots (Temperate→Beach, Cold→Tundra, Highland→Mountain) unchanged.
- Sum-to-one normalization unchanged.

The existing unit tests (Beach dominant at sea_level+0.5, Mountain dominant at sea_level+100, mid-elevation climate-distinct biomes) all continue to pass with the new constants; specific test elevations were chosen for compatibility with the retuning.
