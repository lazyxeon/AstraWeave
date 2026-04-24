# Terrain Scale Diagnostic — 2026-04-24

## Scope

Phase 1.6-F.4.B.1: investigation-only diagnostic of AstraWeave's terrain scale relative to Andrew's perceived "small" problem and the campaign's §1.2 AAA-parity target. Produces measurement-based evidence to support F.4.B.2's implementation decision and anchors `docs/supplemental/WORLD_SCALE_CONVENTIONS.md`.

## Key findings (summary)

1. **AstraWeave has no explicit world-unit convention.** Camera code documents `near: 0.5, far: 5000.0` with "meters" comments, but the scatter system applies a hidden **14× multiplier** on tree assets whose raw Blender heights (~3.7 units for the primary `tree_small_02_a.glb`) were authored at real-world scale. The 14× compensates for a mismatch someone saw, not for a deliberate convention. **Working hypothesis: 1 WU = 1 meter was the intent, but the scatter hack obscured it.**

2. **Current terrain at radius 5 is horizontally and vertically under-scaled relative to AAA references.** Post-erosion Y span 82-101 WU across all climates; horizontal extent 2816 × 2816 WU = 7.93 km² (assuming 1 WU = 1m). Horizontal extent is **20% of Skyrim (37 km²)** and **11% of Enshrouded EA (24 km²)**. Vertical/horizontal ratio **0.033** is geometrically OK for Piedmont foothills but fails the "mountain" aesthetic expectation.

3. **The "short and thin" perception is a horizontal scale + vertical amplitude problem, compounded by the tree-size hack.** Mountains at ~92 WU with trees rendered at 50-60 WU produces only 2× peak-to-tree ratio. Real Blue Ridge mountains have ~90× peak-to-tree ratio. Even at 1 WU = 1m where mountains are 92m and trees are 50m, the composition reads as "hills with oversize trees" — exactly Andrew's visual finding.

4. **Recommended aesthetic target is Target A (Appalachian).** Matches Andrew's lived NC reference. Achievable within existing architecture: radius 7, ~5-8× mountain amplitude increase, tree multiplier reduced from 14× to ~3-5×. No rayon required for generation; no streaming required.

## Measured data

### Tree asset sizes (from `astraweave-terrain/src/biome.rs`)

| Asset | Raw height (units) | Scatter scale range | Render multiplier | Effective WU height |
|-------|-------------------:|:-------------------:|------------------:|-------------------:|
| `tree_small_02_a.glb` | 3.689 | 0.8–1.4 | 14.0× | **37.0–78.9** (typ. ~57) |
| `dead_tree_trunk.001.glb` | 0.290 | 0.6–1.1 | 14.0× | **2.2–5.0** |
| `dead_tree_trunk_02.glb` | 1.055 | 0.5–1.0 | 14.0× | **6.6–16.3** |

- Type multiplier source: `tools/aw_editor/src/terrain_integration.rs:2532` (per-type 14.0× for trees).
- Per-instance scale: `BiomeConfig::grassland()` etc. in `astraweave-terrain/src/biome.rs:320-510`.
- Tree spacing (min_distance): 8-10 WU.
- Scatter density: ~52 trees per 256×256 chunk (Grassland tier).
- LOD culling at 50/150/500/1500 WU; per-type tree cull at 1200 WU.

**Asset-author intent (inferred):** raw `tree_small_02_a.glb` at 3.689 Blender units likely represents a ~3-4m small tree / sapling (typical stylized asset authoring). The 14× multiplier scales this to 50+ WU, which is NOT a natural scale convention — it's a visual compensation.

### Camera / rendering parameters

From `tools/aw_editor/src/viewport/camera.rs` + `astraweave-render/src/volumetric_fog.rs`:

| Parameter | Value | Source |
|-----------|------:|--------|
| FOV | 60° | `camera.rs:107` |
| Near clip | 0.5 (m) | `camera.rs:109` |
| Far clip | 5000 (m) | `camera.rs:110` |
| Default orbit distance | 25 (m) | `camera.rs:104` |
| Default yaw / pitch | 45° / 30° (looking down) | `camera.rs:105-106` |
| Aspect ratio | 16:9 | `camera.rs:108` |
| Projection | `Mat4::perspective_rh` | `camera.rs:458` |
| Fog (volumetric) | density 0.01 base + 0.05 height @ 0.3 falloff | `volumetric_fog.rs:132-137` |
| Fog froxel grid | 160×90×64 (medium preset) | `volumetric_fog.rs:133` |
| Default chunk radius (UI) | 5 (range 1-6) | `terrain_panel.rs:408,646` |

**Camera convention consistency:** near/far comments use "meters" — reinforcing the 1 WU = 1 meter intent. Default orbit distance 25m + pitch 30° puts default camera at ~12.5 WU altitude looking toward horizon, which is below tree canopy (trees 50 WU). Camera evidently gets flown up manually before screenshots (Andrew's Y=177, Y=233).

### Screenshot cross-reference

Andrew's phase-3 and phase-4 screenshots show camera Y values around 177-233 WU. At 1 WU = 1m that's 177-233m altitude — between "tall church steeple" and "skyscraper penthouse". For a terrain with 92 WU (92m) peaks, the camera is flying above the tallest mountain by 85-140m. Visible horizon in the shots corresponds to the full 2816 WU world extent (reaching the culling edge at 1500 WU).

### AstraWeave terrain scale (current state post-phase-4)

**Horizontal:**
- Chunk extent: 256 × 256 WU.
- Heightmap resolution: 64 × 64 vertices per chunk → **4 WU vertex spacing**.
- Radius 5 → 11 × 11 = **121 chunks → 2816 × 2816 WU** total world extent.
- At 1 WU = 1m: **7.93 km²**. At 1 WU = 0.1m: 79,300 m² (0.08 km²).

**Vertical (full radius-5 grid, all 6 climates, seed 12345):**

| Climate    | pre.max | pre.p99 | pre.p50 | post.max | post.p99 | post.p50 | Y span |
|------------|--------:|--------:|--------:|---------:|---------:|---------:|-------:|
| Temperate  |  121.90 |   84.89 |   29.09 |    92.77 |    72.21 |    21.47 |  92.77 |
| Cold       |  121.90 |   84.89 |   29.09 |    82.03 |    62.69 |    19.22 |  82.01 |
| Arid       |  121.90 |   84.89 |   29.09 |    96.35 |    78.56 |    29.16 |  96.35 |
| Tropical   |  121.90 |   84.89 |   29.09 |   101.22 |    72.51 |    20.67 | 101.11 |
| Wetland    |  121.90 |   84.89 |   29.09 |   101.22 |    72.51 |    20.67 | 101.11 |
| Highland   |  121.90 |   84.89 |   29.09 |    82.03 |    62.69 |    19.22 |  82.01 |

(Pre-erosion values are identical across climates because `ClimateBias` only affects erosion preset selection, not the noise field. Source noise max is 121.9 WU; post-erosion varies 82-101 WU by climate.)

**Vertex/horizontal ratio:**
- Y span 92 WU / horizontal 2816 WU = **0.033**.
- Reference: NC Blue Ridge 100 km transect ≈ 0.018; Piedmont foothills ≈ 0.01-0.03; Blue Ridge peaks ≈ 0.08-0.15.
- AstraWeave **geometrically matches Piedmont / Appalachian foothills**, BUT with absolute scale of a small town rather than a region.

### Reference-title comparison

| Dimension          | AstraWeave | Skyrim    | Witcher 3  | Enshrouded EA | RDR2        | NC Blue Ridge transect |
|--------------------|-----------:|----------:|-----------:|--------------:|------------:|----------------------:|
| Chunk / cell side  | 256 WU     | 57.6 m    | ~186 m     | not doc.      | not doc.    | N/A                   |
| Vertex spacing     | 4 WU       | ~1.8 m    | ~0.37 m    | not doc.      | not doc.    | 30 m (SRTM)           |
| Total extent (km²) | 7.93       | 37        | 74         | 24 (64 @ 1.0) | ~75         | 100 km transect       |
| Max vertical relief| 92 WU      | ~766 m    | not doc.   | not doc.      | not doc.    | 1820 m                |
| Vert/horiz ratio   | 0.033      | 0.018     | not doc.   | not doc.      | not doc.    | 0.018                 |

(Sources for reference titles: Gollent GDC 2014 for Witcher 3; CreationKit wiki + UESP for Skyrim; Keen Games Steam page + community tweets for Enshrouded; community consensus for RDR2; USGS/Britannica for NC. Full citations at end of document.)

### Interpretation

- **Ratio is fine** (0.033 is similar to Skyrim's 0.018 and NC Blue Ridge's 0.018). Phase-3/phase-4 didn't create a shape problem.
- **Horizontal extent is small** (20% of Skyrim, 11% of Enshrouded EA). At 1 WU = 1m, a 2.8 km × 2.8 km world is roughly one NC county's width.
- **Vertical max of 92 WU** is consistent with "hills" at 1 WU = 1m (100m relief = nice rolling hills), NOT with "mountains" (Skyrim's Throat of the World: 766m).
- **Peak-to-tree ratio ~2×** is the visual tell. Real forests on real mountains have peaks 50-100× trees. Even Skyrim's 766m peaks next to its scaled stylized trees read as "mountains" because the ratio is dramatic.

## Aesthetic target analysis

### Target A — Appalachian / Rolling (default recommendation)

Matches Andrew's lived NC reference. Cal range: NC Piedmont → Blue Ridge foothills.

| Dimension | Current | Target A |
|-----------|--------:|---------:|
| Horizontal extent | 7.93 km² | **10-30 km²** (radius 7-10 OR chunk 384-512 WU) |
| Y span | 92 WU (m?) | **300-600m** (3-6× increase) |
| Vert/horiz ratio | 0.033 | **0.02-0.05** |
| Tree height | 37-79 WU (stylized-14× hack) | **8-20m** (match real trees) |
| Peak-to-tree ratio | ~2× | **15-60×** |

**Knob settings for Target A:**
- Radius: 5 → 7 (11² = 121 chunks → 15² = 225 chunks, +86%).
- OR chunk extent: 256 → 384 WU at radius 5 (1.5× horizontal, same chunk count).
- Y amplitude: grassland `mountains_amplitude` + `base_amplitude` × **5-8**.
- Continental scale/min: re-calibrate against new extent (§2.6's wavelength needs to match world extent).
- Tree render multiplier: 14× → **3-5×** (matches 1 WU = 1m convention).
- Elevation bands in `elevation_biome.rs`: re-calibrate thresholds to new Y range.

**Risks:**
- Elevation-band re-calibration is a §1.5 / F.4 concern (biome thresholds currently tuned for ~100 WU span).
- Continental modulation (§2.6) assumes ~2500 WU wavelength ≈ world extent; larger world needs wavelength recalibration.
- Tree multiplier change breaks existing scatter visuals; needs coordinated update.

**Performance (projected):**
- Current phase-4 generation time at radius 5: Temperate ~43s, Cold/Highland ~25s.
- Radius 7 → 225 chunks: Temperate ~80s (2.7× budget). Rayon with 4 threads: ~23s. OK.
- OR chunk extent 384 → 9/4 = 2.25× halo work per chunk, same chunk count: Temperate ~97s. Rayon: ~27s.
- Memory: radius 7 adds 86% of vertex data. Manageable.

### Target B — Stylized Open World / Enshrouded

More dramatic, matches current AAA open-world baselines.

| Dimension | Current | Target B |
|-----------|--------:|---------:|
| Horizontal extent | 7.93 km² | **24-64 km²** (Enshrouded bracket) |
| Y span | 92 WU | **800-1500m** (~10-15× increase) |
| Vert/horiz ratio | 0.033 | **0.05-0.10** |
| Peak-to-tree ratio (with 15m trees) | 2× | **50-100×** |

**Knob settings:**
- Chunk extent: 256 → 512 WU at radius 8-10 → 5-10 km per side.
- Y amplitude: ×10-20.
- Vertex density: potentially 64 → 96 for extra detail at larger chunk (keeps 5.3 WU vertex spacing).
- Continental + elevation bands re-calibration (large).

**Risks/Costs:**
- Generation time: ~20+ minutes unmitigated. Rayon required and may still be marginal.
- Editor usability suffers at this generation time. Implies progressive / async terrain loading — architectural work beyond what F.4/F.5 scope.
- Continental-scale physics: erosion droplet travel (~120 WU p95 from phase 0) becomes negligible against chunk size → halo=1 sufficient or potentially reducible.

### Target C — Alpine / Crimson Desert

Most dramatic end of current AAA.

| Dimension | Current | Target C |
|-----------|--------:|---------:|
| Horizontal extent | 7.93 km² | **50-150 km²** |
| Y span | 92 WU | **2000-4000m** |
| Vert/horiz ratio | 0.033 | **0.10-0.20** |

**Knob settings:**
- Chunk extent: 256 → 512 WU, vertex density 64 → 96, radius 10-15 → 10-15 km per side.
- Y amplitude: ×20-40.
- Full continental + elevation band rewrite.
- Almost certainly requires streaming (can't pre-generate all chunks at startup).

**Risks/Costs:**
- Multi-minute to multi-hour editor-time generation.
- Streaming architecture becomes prerequisite — separate F.6 territory.
- Phase-0's halo sizing assumption (p95 < 256 WU) still holds at chunk 512 but erosion effects on peak sculpting become more dramatic with larger vertical relief. Re-characterization needed.

### Default recommendation: Target A

**Rationale:**

1. **Aesthetic match:** Andrew's direct NC background suggests Appalachian scale as the authored target. Target A is honest to that reference.
2. **Lore consistency:** Veilweaver's Tebel/Chevel is described as continental (not alpine). Target A is geometrically appropriate.
3. **Engineering feasibility:** radius 7 fits existing architecture with rayon (phase-2 deferred, but F.5 / F.6 trivial to land). No streaming required. No new rendering systems needed.
4. **Reversibility:** if Andrew later wants more dramatic scale, amplitude can be pushed further or radius extended. Over-shooting now is harder to back out.
5. **14× tree hack resolvable:** with realistic mountain amplitude (300-600m) and realistic tree sizes (~15m), the ratio becomes naturally ~30× without any post-hoc multipliers.

**Targets B and C** are escape hatches. Flagged, not chosen. If Andrew's vision demands dramatic alpine scale, escalate to a multi-phase streaming architecture conversation (separate campaign).

## Implementation paths (for F.4.B.2 prompt)

### If Target A chosen

**Files / constants to touch:**
- `astraweave-terrain/src/noise_gen.rs` — `mountains_amplitude`, `base_amplitude`, `detail_amplitude` default values (and per-preset overrides in `terrain_panel.rs:1816-1991`).
- `astraweave-terrain/src/noise_gen.rs` — `continental_scale` recalibration for new world extent (currently 0.0012 for ~830 WU wavelength).
- `astraweave-terrain/src/elevation_biome.rs` — `HIGHLAND_BANDS`, `COLD_BANDS`, `TEMPERATE_BANDS` threshold values.
- `tools/aw_editor/src/panels/terrain_panel.rs` — UI chunk_radius default (5 → 7).
- `tools/aw_editor/src/terrain_integration.rs:2532` — tree render multiplier (14.0 → ~3.5).
- F.2 regression test (`phase_1_6_f2_t_highland_regions_reach_f1_target`) — threshold values need adjustment to match new scale.
- F.2 regression test (`phase_1_6_f2_t2_surface_spikiness_under_threshold`) — curvature threshold may need re-calibration because higher amplitudes change absolute curvature.

**Expected impact:**
- Y max: ~100 WU → ~400 WU.
- Horizontal extent: 2816 → 3840 WU.
- Tree height: 57 WU → 14 WU.
- Peak-to-tree ratio: 2× → 30×.
- Mountain visual presence: "hill" → "mountain ridge".

**Risks:**
- Calibration pass for continental + elevation bands. Needs measurement at each step.
- Tree multiplier change affects all scenes' visual character — may need audit of other scatter consumers (vegetation preview, biome_pack demos).
- F.2 regression tests that encode specific Y values need updating. New values become the post-F.4 baseline.

**Stepwise plan:** F.4.B.2 lands amplitude first (verify peaks visible), then radius (verify world larger), then tree multiplier (verify trees match mountains), then elevation-band re-calibration (verify biomes distribute correctly).

### If Target B chosen

Same categories as Target A, but:
- Radius 8-10 or chunk_extent 512.
- Mountain amplitude × 10-20.
- Vertex density possibly 64 → 96.
- Rayon parallelization REQUIRED (phase-2.E blocker becomes urgent).
- Elevation band rewrite more aggressive (bigger Y range → more bands possibly).

### If Target C chosen

Additional prerequisites beyond Target B:
- Streaming / async chunk loading (architectural — separate campaign).
- Possibly texture virtualization for splats at this scale.
- Rendering systems audit (fog/far-plane/LOD may need re-tuning at 10+ km extent).

## Out of scope (noted for future work)

- **Rayon parallelization** (still deferred — becomes urgent for Target B/C but optional for Target A).
- **Streaming / progressive generation** (Target C territory — separate campaign).
- **Vertex density increase** (Target C — affects memory/GPU).
- **Climate field re-calibration post-scale-change** (part of F.4 follow-up once scale lands).
- **14× tree multiplier rework** (part of F.4.B.2 Target-A implementation).
- **Sparse virtual textures / texture streaming** (Target C).
- **Water system scale** (has its own audit at `docs/audits/water_system_architecture_2026-04-20.md`).

## Sources cited

### AstraWeave measurements (this session)
- `astraweave-terrain/tests/phase_1_6_f3_phase_4_diagnostic.rs::phase_4_b_1_scale_radius5_per_climate` — Y statistics per climate at radius 5.
- `astraweave-terrain/src/biome.rs` lines 320-510 — scatter config per biome.
- `astraweave-terrain/src/scatter.rs` lines 37-56 — LOD config.
- `tools/aw_editor/src/terrain_integration.rs:2532` — tree render multiplier.
- `tools/aw_editor/src/viewport/camera.rs` lines 104-458 — camera defaults.
- `astraweave-render/src/volumetric_fog.rs` lines 132-155 — fog parameters.

### External references (research-scout 2026-04-24)
- [CreationKit Wiki — Unit (Skyrim)](https://ck.uesp.net/wiki/Unit) — 128 units = 1.83m; 70 units/m.
- [CreationKit Wiki — Exterior Cells (Skyrim)](https://ck.uesp.net/wiki/Exterior_Cells) — 4096 × 4096 units = 57.6m per cell.
- [Hoddminir — Heightmap to Worldspace (Skyrim modding)](http://hoddminir.blogspot.com/2012/02/from-heightmap-to-worldspace-in-skyrim.html) — 3808 × 3008 px = 119 × 94 cells.
- [UESP — Throat of the World peak elevation](https://www.facebook.com/UESP.net/photos/a.560106030690111/2849438801756811/?type=3) — 766.5m ASL.
- [GDC 2014 — Marcin Gollent "Landscape Creation and Rendering in REDengine 3" (Witcher 3)](https://media.gdcvault.com/GDC2014/Presentations/Gollent_Marcin_Landscape_Creation_and.pdf) — 46×46 tiles × 512² verts, 0.37m vertex spacing.
- [GameSkinny — Enshrouded map size](https://www.gameskinny.com/tips/how-big-is-the-enshrouded-map-size-world-size-detailed/) — 24 km² EA / 64 km² 1.0.
- [Newsweek — RDR2 map size vs GTA5](https://www.newsweek.com/red-dead-2-rdr2-map-size-vs-gta-5-v-1190240) — ~75 km² community consensus.
- [Screen Rant — Crimson Desert world size](https://screenrant.com/crimson-desert-world-size-confirmed-red-dead-skyrim/) — "twice Skyrim" ~74+ km²; community range 80-150 km².
- [Britannica — Mount Mitchell](https://www.britannica.com/place/Mount-Mitchell) — 2037m peak.
- [earthathome.org — Blue Ridge escarpment topography](https://earthathome.org/hoe/se/topography-brp/) — escarpment 400-760m over 5-10km.

### AstraWeave internal documents
- `docs/current/TERRAIN_GENERATION_QUALITY_CAMPAIGN.md` — §1.2 AAA-parity target, §2.3 halo strategy, §2.6 continental scale.
- `docs/audits/heightmap_generator_audit_2026-04-21.md` — the audit that surfaced Phase 1.6-F scope.
- `docs/audits/terrain_seamless_erosion_research_2026-04-24.md` — phase-3 research on seamless erosion.
- `docs/audits/terrain_erosion_seamless_diagnostic_2026-04-24.md` — phase-3 diagnostic measurements.

## Diagnostic conclusion

**The perceived scale problem is real and primarily horizontal + vertical under-scaling, compounded by the tree-multiplier hack.** The terrain's geometric character (ratio, erosion, biomes) is fine. The absolute scale is a tenth of what AAA references use. Target A (Appalachian) is recommended: achievable in F.4.B.2 with targeted amplitude/radius/tree-multiplier changes. Targets B and C are documented as escape hatches requiring separate architectural work.

**Andrew's decision point:** accept Target A as the F.4.B.2 scope, or specify one of the alternatives (B, C, or a custom target by knob values). F.4.B.2 implements whichever is chosen.
