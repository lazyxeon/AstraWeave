# T.W.1.A — World-Scale Water Presentation (outcome)

> **Beat:** T.W.1.A (terrain series, closes the T.W.1 render-gate rejection) · **Date:** 2026-07-22/23
> **Commits:** code `49f1e3396` (5 files, +321/−20) · docs (this note + `water.md` v1.3) — hash in the session report.
> Evidence tiers: **built** / **run** / **verified**. Session screenshots in `d:/tmp/tw1a_staging/` (local, not committed; frame numbers `NN_*.png` cited below). Baseline/after probe logs: `d:/tmp/tw1a_probe_baseline2.txt`, `d:/tmp/tw1a_probe_after.txt`.

## 0. Summary

The director's render-gate finding — *"a square plane that follows the camera everywhere … clearly just following the camera instead of being part of the terrain"* with *"pretty strong whitecaps"* — is closed by three small, structural changes: a **horizon shell** (the water now extends past every camera far plane, so its visible boundary is the far-plane horizon arc or fog, never a mesh edge), a **distance amplitude fade** (the Gerstner surface is exactly flat before the shell's seam, which also kills far-field alias shimmer and distant whitecaps), and **per-`WaterStyle` wave parameters** (Ocean calmed to world scale; Lake/Swamp near-still with crest foam off). Verified by flying the camera ~25 km across open water with no visible boundary at any position, in BOTH view regimes (judging aids far=40,000; production fog 800/1800 — checked locally, values not committed). GPU cost is unchanged at the ~0.26 ms class (2.0 ms provisional budget). The baseline probe run also caught and fixed a T.W.1 residue: the budget probe's own Probe B tripped T.W.1's install-format assert.

## 1. The defect, quantified

The chunked LOD grid spans `(2·GRID_RADIUS+1)·CHUNK_SIZE` = 17×64 = **1,088 units** centered on the camera's chunk (edge at ±544). The editor judging regime is camera far = **40,000** ([camera.rs:262-264](../../tools/aw_editor/src/viewport/camera.rs)) with fog at **60k/120k** ([engine_adapter.rs:1751-1755](../../tools/aw_editor/src/viewport/engine_adapter.rs)) — fog starts *beyond* the far plane, so it conceals nothing: the grid's square edge sat in plain view mid-water and re-centered with the camera every 64 units. The production regime the T.3 beat restores (far 5,000, fog full at 1,800) needs ≥1,800 of coverage. Required visual horizon = `min(camera_far, full-fog distance)`: **40,000** judging / **1,800** production → design for 40,000+.

## 2. Work items — what shipped, with evidence

### 2.1 Horizon coverage — the horizon shell (+ the fade that makes its seam invisible)

**Options weighed (against measured need, not preference):** scaling `GRID_RADIUS` to the far plane needs radius ≈ 625 → **~1.56 M chunks iterated per frame on the CPU** — ruled out by arithmetic. A far-LOD mega-tile ladder reaches 40 k in ~3-4 rings but adds per-ring CPU and a **new LOD-crack surface at every ring boundary**. The shell adds one draw call of 16 triangles, an 8-byte instance write per frame, and **no new crack class**.

**Shipped** (`astraweave-render/src/water.rs`, `shaders/water.wgsl`):
- `generate_horizon_shell()`: a flat 8-quad square annulus (4 corner blocks + 4 edge strips sharing complete edges — no internal T-junctions), inner half-extent **480** (one full chunk *inside* the grid's ±544 boundary → the shell and grid **overlap** in a 64-unit band; same-height same-shader overlap is visually idempotent — the water fragment writes alpha = 1 — and immune to the T-junction pinholes abutment risks), outer half-extent **100,000** (2.5× the judging far plane). Anchored per frame to the grid's own snapped chunk center, so the overlap always aligns. Surface-only (no skirt: inner edge hides under the grid, outer edge is beyond every far plane). Drawn under the same dormant condition as the grid — `has_visible_chunks() == false` still means the water draws nothing.
- **Distance amplitude fade** (`WAVE_FADE_START/END` = 260/420, WGSL consts mirrored in `water.rs`): Gerstner amplitude AND steepness scale to exactly **zero** by 420 units from the camera — before the shell's inner edge under worst-case camera offset (480 − 32·√2 ≈ 434.7) — so the grid→shell transition is **flat-against-flat by construction**. The fade starts beyond the LOD0/LOD1 bands (≤ 220), leaving the W-series near-field look untouched. Side effects, both desirable: far chunks stop alias-shimmering (LOD3's 16-unit vertices undersample the 42-unit swell wavelength), and distant whitecap churn is gone.
- The far-plane cut at 40,000 reads as **the sea horizon** — a level, camera-independent line — which is what an ocean horizon is.

### 2.2 Per-`WaterStyle` wave/foam scale

`WaterRenderer::set_wave_params(amplitude_scale, foam_threshold)` — one scale multiplying each wave's amplitude AND steepness (their ratio, the Q shape factor, is preserved: the surface calms without changing character), clamped to `[0,1]` so styles only calm DOWN from the W-series baseline (skirt-crack margin intact). The uniform rides the former `_pad2` at offset 148 — **layout unchanged**, `WaterUniforms` still 512 B (offset-pinned in `test_uniforms_size`). The Q denominator gained an epsilon (`max(freq·amp·4, 1e-6)`) so a zero scaled amplitude yields Q = 0, not 0/0 = NaN. Editor map (`engine_adapter.rs::set_water_enabled`): **Ocean 0.55 / 0.75** (visible swell, whitecaps only on the tallest crests), **River 0.30 / off**, **Lake 0.12 / off**, **Swamp 0.08 / off** ("off" = threshold 10.0, far above the ≈1.65·scale max crest). Defaults are this beat's observation-pass tuning; final judgment at the re-gate.

### 2.3 Slider range + probe repair

- Water level slider max 15 → **30 m** (`terrain_panel.rs:933`; clamp + "Sea level" reset kept). 30 floods deep valleys (world amplitude 50) while staying sane.
- **T.W.1 residue found by this beat's baseline run:** `water_budget_probe` Probe B constructed its full-frame `WaterRenderer` with the *surface* format; T.W.1's install assert (correctly) panicked it — the assert had landed without a probe re-run. Fixed to `hdr_format()` (the assert's own prescription) before the baseline was captured, so before/after numbers compare like-for-like.

## 3. Budget proof (work item 4)

Min-spec GPU (**NVIDIA GTX 1660 Ti Max-Q, Vulkan, TIMESTAMP_QUERY**), 1920×1080, medians over 300 frames. Baseline at HEAD `1ef81c239` (+ probe-B fix only); after = with all T.W.1.A changes. Both runs `EXIT=0` (run + verified).

| Probe (median) | Baseline | After | Δ |
|---|---|---|---|
| A isolated water pass — near | 0.2478 ms | 0.2521 ms | **+0.0043 ms** |
| A isolated water pass — horizon | 0.1585 ms (0.1782 in run 1 — run-to-run variance) | 0.1770 ms | within variance |
| C refraction water pass — near | 0.0891 ms | 0.0982 ms | +0.0091 ms |
| C refraction water pass — horizon | 0.1204 ms | 0.1198 ms | −0.0006 ms |
| C scene-color copy — near | 0.0886 ms | 0.0889 ms | ≈0 |
| D weave 8-inst — near | 0.1577 ms | 0.1598 ms | +0.0021 ms |
| A render-check lit pixels | 62.8% | **64.5%** | shell rasterizes (independent draw evidence) |

Worst-case remains in the **~0.26 ms class vs the 2.0 ms provisional budget** — far under the beat's 0.5 ms STOP threshold.

## 4. Observation pass (run + verified; frames in `d:/tmp/tw1a_staging/`)

Mediterranean seed 12345 r6 (169 chunks, census auto-ON):

- **Money frame redone** at level 12.5 m, same high-camera family as T.W.1's `08_med_level15.png`: `13_money_frame_redo.png` — flooded basins with shoreline foam, no churn, **sea to the skyline beyond the terrain** (in T.W.1 the world edge faded into void).
- **No edge from a moving camera** — the decisive sequence: the camera was flown ~25 km across open water in three ~10-km steps — (−3,499, 1,855, 8,043) `19_yaw1.png` → (−9,270, 1,855, 13,814) `20_yaw2.png` → (−15,037, 1,855, 19,581) `21_yaw3.png`. Every frame: open sea to a dead-level horizon line, **no square, no boundary, nothing tracking the camera**. Plus overview `18_zoomout.png` and the pulled-back archipelago `28_seam_discriminate.png` (sea running to the horizon between islands).
- **Seam-artifact discrimination:** a faint straight-lined pattern seen in one flooded valley (`27_valley_flood2.png`) was tested by moving the camera ~465 units/axis (snapped chunk 2→10): the pattern **stayed glued to the same world-space basin** (`28`) → it is the submerged terrain's splat pattern refracting through clear shallow water (pre-existing terrain visual; refraction working as designed), NOT a water-mesh seam.
- **Ocean vs Lake reading distinctly:** Med frames (`13`, `23`, `28`) show the ocean-blue palette with foam-lined shores; Boreal with the checkbox override ON at 12.5 m (`32_boreal_lake.png`) shows the Lake style — near-black still water in the valley lake and a dark navy horizon band, no foam, no swell.
- **Census behavior re-verified across archetypes:** Boreal generated → checkbox arrived **UNCHECKED**, bone-dry (`31_boreal.png` — also closes T.W.1's pending Boreal one-click confirmation); Desert generated → checkbox **UNCHECKED** (the Boreal-era manual override was reset by generation, per contract), bone-dry with the level slider still at 12.5 (`35_desert.png`) — the enable gate governs.
- **Production-regime check (local, uncommitted):** fog 800/1800 + far 5,000 set temporarily, editor rebuilt, Med regenerated: `39_prod_med_near.png` — near pools crisp with foam, mid-field progressively hazed, everything beyond ~1,800 dissolved in fog; a 12-km view (`38_prod_med.png`) is pure fog. No edge can be visible in this regime. **Both values reverted before commit** (verified absent from the code-commit diff); T.3 owns the real revert of the judging aids.

## 5. Verification ladder

1. `cargo test -p astraweave-render --lib water` → **22 passed; 0 failed** (1,263 filtered out) — T.W.1's 18 + 4 new: `test_horizon_shell_geometry` (counts, flatness, hole, per-triangle upward winding, overlap = grid boundary − one chunk), `test_horizon_covers_both_view_regimes` (required-horizon math for both regimes, shell ≥ 1.25× the larger), `test_wave_fade_flat_before_shell_seam` (fade completes before the worst-case seam distance; fade starts beyond LOD1), `test_wgsl_mirrors_wave_fade_constants` (WGSL const lines pinned verbatim). The GPU draw-through test (`test_renderer_water_initialization`) compiles the modified shader and draws grid + shell under a wgpu validation error scope. `test_uniforms_size` pins `wave_amp_scale` at offset 148, size still 512; `set_wave_params` default/apply/clamp asserted in the GPU test.
2. `cargo test -p aw_editor --lib tw1` → **2 passed** (4,022 filtered); `terrain_panel` suite **58 passed** (3,966 filtered). `cargo check --workspace` **exit 0**. Clippy on both touched crates: zero warnings in this beat's files (pre-existing residue unchanged: 7 terrain-lib warnings, `terrain_panel.rs:741` complex-type, `engine_adapter.rs:1472` field-assignment).
3. Budget probe before AND after (§3), same machine, same GPU, clean exits.
4. Observation matrix (§4), including the moving-camera no-edge sequence and both view regimes.

## 6. Residue / open items

- The **submerged-terrain splat pattern** (checkered diamonds) visible through clear shallow water (§4) is a pre-existing terrain visual surfaced by working refraction — a terrain-material concern if the director wants it addressed; not water-owned.
- Style defaults (Ocean 0.55/0.75 etc.) are first-pass tuning; the re-gate may adjust.
- The archetype ComboBox synthetic-input quirk recurred once (one dead click, retry worked) — same T.W.1 note, editor-UI health pass candidate.
- Probe A's `horizon` camera shows run-to-run variance (0.159–0.178 ms baseline) — cite ranges, not single runs, in future comparisons.
- T.3 still owns reverting the judging aids (fog 60k/120k, far 40k) to production values; `test_horizon_covers_both_view_regimes` hard-codes both regimes and stays valid either way.

## 7. Director re-gate repro

`cargo editor` → Terrain panel → Mediterranean, seed 12345, radius 6 → water arrives checked:
1. At sea level and at a raised level (slider now reaches 30 m): fly/rotate the camera anywhere, including far off-world — the water must read as part of the world with **no square, no visible edge, no camera-tracking boundary**; the sea meets the sky at a level horizon.
2. Ocean character: swell without whitecap churn at world scale (crest foam only on the tallest waves, near field).
3. Boreal + checkbox override ON: near-still dark Lake water, no foam. Desert: dry.
4. Near-field close-up (< ~250 units): the W-series Gerstner/refraction/foam look, unchanged.
That verdict closes the row-7 water gate and unblocks T.W.2.
