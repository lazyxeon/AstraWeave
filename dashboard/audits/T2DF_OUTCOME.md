# T.2d.F — Retiring the material-LOD tiers (outcome)

> **Beat:** T.2d.F (terrain series) · **Date:** 2026-07-25 · **Baseline commit:** `d4c4479d4`
> **Ratified decision (director, 2026-07-25):** DELETE the material-LOD tiers. The T.2d diagnosis
> (`T2D_CAMERA_LIGHT.md` §10) convicted `compute_material_lod`'s LOD1|2 threshold (pixel footprint
> 2.0) as the camera-anchored boundary, and LOD 2's per-pixel tier selection as 40–55% of far-field
> high-frequency energy. Pre-authorized fallback if perf regressed materially: a falloff
> **continuous** in footprint — a stepped tier may not return under any outcome.
> **Outcome: deleted; the boundary signature and the dithering are gone (measured); close range is
> untouched to 1 LSB; min-spec is ~1% FASTER; no fallback needed.**
> Anti-drift honoured: no Toksvig/normal-variance work, no hex-tile work, no material acquisition,
> no scatter/water work, ED-3 defects recorded but not fixed.

---

## 0. What shipped

| item | result |
|---|---|
| The tier machinery | **Deleted** from all four surfaces (§2) — no orphaned plumbing (grep-verified) |
| Regression coverage | `renderer.rs::material_lod_tiers_are_retired` — **proven to fail on pre-fix code** (run before the deletion: `FAILED. 0 passed; 1 failed`), passes after |
| Boundary stations | far-field grain **−48.6% / −51.7%** — inside the diagnosed 40–55% tier-flicker band; grain field now FLAT across the frame (§4.1) |
| Close range | **1 pixel of 786,432 differs, by 1 LSB** — close-up appearance untouched (§4.2) |
| Far field | +2.9 → +10.7 luma over 300–1500 m (up to **+9.5%**) — the accepted, pre-ratified brightening; T.2a/T.2c far-field frames rot (§4.3) |
| Far-field normal-variance gradient | **survives** (+13.5% over 300–1500 m after) — different mechanism, explicitly not this beat's (§4.3) |
| Min-spec perf | median **27.055 → 26.773 ms** (−1.0%) at 1920×1080 on the documented min-spec — no regression, fallback not triggered (§5) |
| Stations | 5 ED-2 `CameraStation`s pinned in `.editor_preferences.json` — the director's closing re-check is **Camera → Go** on each (§7) |

---

## 1. Phase 0 — the BEFORE, captured first

Five stations, all Desert at the live editor's world settings (chunk_radius 10 = 441 chunks, seed 12345, editor noise defaults, biomes pack):

| station | camera | why |
|---|---|---|
| `t2df_boundary_y414` | focal (−1029.9, 36.3, 254.7), dist 529.6, yaw 45.5°, pitch 45.6° → **eye (−770.1, 414.5, 518.9)**, 962×501 | director frame 1 — footprint-2.0 contour at norm row 0.527 |
| `t2df_boundary_y536` | same focal, dist 700.0 → **eye (−686.5, 536.2, 603.9)**, 962×501 | director frame 2 — contour at norm row 0.740 |
| `t2df_desert_close_20m` | focal (43.1, 36.3, −1961.8), dist 20, yaw 45°, pitch 55°, 1024×768 | close range — LOD 0 / multiscatter territory |
| `t2df_lod01_contour` | same focal, dist 46.7, pitch 40° (altitude 30 m), 1024×768 | the LOD0\|1 footprint-0.5 contour (~100 m) is IN frame |
| `t2df_profile_far` | same focal, dist 523.1, pitch 35° (altitude 300 m), 1024×768 | ground 140→3400 m for the far half of the distance profile |

The two boundary stations are the **recovered director camera** (T2D §10.2), asserted in the harness to reproduce the readout eyes to <0.5 m. Every capture goes through the ED-2 path — `ViewportRenderer::render` → `capture_frame_png`, the same function the live widget's Camera → Shot calls (the widget only adds egui presentation on top, ED2_OUTCOME §3.1) — and each restore goes through `capture_state()`/`apply_state`, so a valid frame is also proof the pinned `CameraState` alone reproduces the view. Sidecars (`.camera.json`) beside every PNG. Artifacts: `d:/tmp/t2df_staging/{before,after}/`; harness: `tools/aw_editor/tests/t2df_stations.rs`; analysis: `tools/material_cook/t2df_analysis.py`.

**Phase 0.3 — the boundary quantified (grain metric, per §2.2's lesson — luminance cannot see it):**

| station | far-field grain | near-field grain | footprint-2.0 contour above/below |
|---|---|---|---|
| y414 | **5.051** | 2.715 | 3.651 / 3.562 (×0.98) |
| y536 | **5.307** | 3.264 | 3.501 / 3.298 (×0.94) |

On this (rocky) harness world the tier defect manifests as the diagnosis predicted (§10.4): not a clean line but **an 86% far-field grain excess** — per-pixel tier dithering spread over the LOD2 region. The clean-line form needs the director's smooth dunes; the dithering form is what this world exhibits, and it is exactly what the deletion must remove.

### 1.1 A harness lesson, recorded

The first capture run produced 3 black frames of 5: a render into a **new** texture object of the **same size** comes back black, because the renderer rebinds its output only on a size change. The live widget never hits this (it always passes the same object). The harness now reuses one texture per size, with the reason in a comment.

---

## 2. Phase 1 — the deletion, site by site

Enumerated consumers of the tier machinery (workspace grep, all four surfaces):

| # | site | what changed |
|---|---|---|
| 1 | `brdf_common.wgsl:57-67` `compute_material_lod` + `:82-141` `evaluate_brdf_lod` | **Deleted.** `evaluate_brdf` is now the single BRDF, carrying the former LOD-0 body verbatim (GGX + Smith + Burley + Kulla-Conty multiscatter). Tombstone comment forbids reintroducing a stepped tier and points at the enforcing test. |
| 2 | `renderer.rs` SHADER_SRC (static PBR), formerly `:231/:234` + the **IBL gate `:296`** | BRDF call → `evaluate_brdf(...)`. The IBL gate (`mat_lod < 2u` → full 3-sample IBL, else 1-sample diffuse approx) **shared the tiers' footprint-2.0 threshold and died with them** — every fragment now takes the full IBL path. This is the fourth consumer the beat's "three call sites" undercounted; enumerated here so it is a decision, not a surprise. |
| 3 | `renderer.rs` SKINNED_SHADER_SRC, formerly `:571/:574` | BRDF call → `evaluate_brdf(...)`. (No IBL gate existed in this shader.) |
| 4 | `pbr_terrain_forward.wgsl`, formerly `:369-372` | BRDF call → `evaluate_brdf(...)`. Header comment (`:17`) updated. |

Also updated: `terrain_material_manager.rs` doc comment (`:168`) that described the old shader-concat contract.

**No orphaned plumbing:** `mat_lod` was shader-local (derived from `fwidth`, no uniform, no binding, no constant elsewhere); post-deletion grep for `compute_material_lod|evaluate_brdf_lod|mat_lod` finds only documentation and the absence-asserting test.

**Regression coverage that bites:** the two old tests pinning the tier functions' *presence* (`brdf_common_contains_material_lod_functions`, `shader_uses_material_lod`) were replaced by `material_lod_tiers_are_retired`, which asserts *absence* across all four shader surfaces plus survival of the unified `evaluate_brdf` and its multiscatter term. Landed **before** the shader edits and run: `FAILED. 0 passed; 1 failed` on pre-fix code; passes after. It fails again on any reintroduction.

---

## 3. Naga validation

`cargo test -p astraweave-render --test shader_validation`: the composed terrain shader validates (`test_pbr_terrain_forward_validates_with_prefix` **ok**), entry points and feature compatibility **ok**.

`test_all_shaders_compile` is **red for a pre-existing reason**: its non-vacuity floor (`validated_count >= 60`) fails at 58 with **zero actual shader failures** — and it fails identically on a clean HEAD tree (proven by stashing this beat's changes and re-running). This is a **structural liveness guard** tripping: the validation coverage genuinely shrank below the calibrated floor at some earlier point. Per the standing T.G rule, a liveness guard is never re-baked — the floor was left alone and the failure is flagged in the rot ledger (§6) for its own investigation.

---

## 4. Phase 2 — the AFTER, proven

### 4.1 The boundary signature is gone

| station | far-field grain before → after | near-field after | contour above/below after |
|---|---|---|---|
| y414 | 5.051 → **2.594 (−48.6%)** | 2.537 | 2.870 / 3.018 (×1.05) |
| y536 | 5.307 → **2.563 (−51.7%)** | 2.873 | 2.984 / 2.885 (×0.97) |

Both deltas sit **inside the 40–55% band the diagnosis measured as tier flicker** (§10.5's pinned-tier A/B) — the far-field high-frequency energy that vanished is the energy that was convicted. And the structure is gone, not just reduced: after the deletion the grain field is **flat across the frame** (far 2.59/2.56 vs near 2.54/2.87), where before the far field carried an 86% excess. There is no step at the footprint-2.0 contour (×1.05/×0.97 ≈ 1) and no tier boundary anywhere in the frame — by construction, since no threshold survives in any shader. Visually, the far field now reads as coherent dune relief instead of a noise carpet.

### 4.2 Close range untouched — measured, not asserted

`t2df_desert_close_20m` before vs after: **1 differing pixel of 786,432 (0.0001%), max channel delta 1.** The close field was entirely LOD 0 before, and the unified `evaluate_brdf` is the LOD-0 math verbatim — so every close-range judgment T.2a/T.2c made carries over unchanged. (The 1-LSB pixel is where the pipeline-hash change perturbs one rounding.) On the contour station the diffs are confined to the far rows (0–438 of 768; 2.37% of pixels, mean |Δ| 0.067), i.e. exactly the former LOD1/LOD2 region.

### 4.3 The distance profile — step gone, gradient survives

Luma binned by true per-pixel 3D distance (ray/ground-plane, the §9.2-corrected method), composited across the three profile stations:

| range | before | after | Δ |
|---|---|---|---|
| 18–45 m (close station) | 111.7 / 113.8 | 111.7 / 113.8 | **±0.00** |
| 70–100 m | 118.90 | 118.93 | +0.02 |
| 100–140 m | 124.85 | 125.14 | +0.28 |
| 140–200 m | 128.40 | 128.96 | +0.56 |
| 300–450 m | 105.10 | 108.02 | +2.92 |
| 650–900 m | 114.34 | 118.31 | +3.97 |
| 900–1200 m | 113.40 | 121.69 | +8.30 |
| 1200–1500 m | 111.90 | 122.55 | **+10.66 (+9.5%)** |

Reading it against the diagnosis's expectations:

- **The close field is numerically unchanged** (±0.00 to two decimals) — the multiscatter is not a change there because LOD 0 already had it. The former **LOD0|1 step** (multiscatter appearing/disappearing at footprint 0.5, ~100 m at this station) now shows as a smooth +0.3–0.6 luma fill-in with **no discontinuity** — note this is far smaller than §3.2's +9.5% Mediterranean close-field figure; the multiscatter magnitude depends on biome albedo, roughness and geometry, and §3.2's A/B removed it from an *entire* frame rather than across one contour.
- **The far field brightens up to +9.5%** — the §6.2-predicted "~+10%", the consequence the ruling pre-accepted. This is LOD 2's Lambertian + `F·0.25` approximation being replaced by the full BRDF. T.2a/T.2c **far-field** frames are rotted by exactly this (§6 ledger).
- **The far-field gradient survives, as it must:** after the fix the far station still climbs 108.0 → 122.6 over 300–1500 m (**+13.5%**, near-darker/far-brighter — the director's §9.1 sign). That is the unattributed normal-variance mechanism (Toksvig-class), untouched per the anti-drift line. The tiers were never its cause and this beat never claimed otherwise.

---

## 5. Phase 3 — min-spec performance

**Adapter (printed by the harness):** NVIDIA GeForce GTX 1660 Ti with Max-Q Design · Vulkan · DiscreteGpu · driver 592.82 — the documented min-spec (water.md §measured; driver was 592.27 then).

**Method:** the y414 boundary framing (the distance-heaviest configuration — 75.3% of terrain pixels were LOD 2 before, i.e. where the tiers saved the most) at **1920×1080**, 300 timed frames after 60 warm-up, wall time per frame with a forced GPU sync (`device.poll(Wait)`) — medians:

| leg | median | p10 | p90 |
|---|---|---|---|
| before | **27.055 ms** | 26.408 | 27.949 |
| after | **26.773 ms** | 26.103 | 27.674 |

**Δ = −0.28 ms (−1.0%) — the deletion is at worst free and measures slightly faster** (uniform control flow; the static shader also lost a divergent IBL branch). The pre-authorized continuous-falloff fallback is therefore **not implemented** — there is no regression to justify it.

Context for the numbers: the terrain-heavy full frame runs ~27 ms at 1080p full-window on min-spec (~37 FPS; the director's live editor at its 962×501 viewport reads 54–59 FPS in the T.2d frames — consistent, since the editor renders fewer pixels). For scale against the workspace's budget discipline: the entire water budget is 2.0 ms (water.md); this change moved the frame by ~0.3 ms in the *favourable* direction.

**A methodological note, recorded for the next perf beat:** requesting `TIMESTAMP_QUERY` on this harness device **hung the run** before terrain generation completed (CPU frozen at 479 s; killed after 20+ minutes) — twice-reproducible cost was not paid to bisect it. The production `GpuProfiler` per-pass numbers in water.md were measured through different binaries; this harness uses wall-clock with per-frame sync instead (`T2DF_TS=1` re-enables timestamps for whoever hunts the hang).

---

## 6. Phase 4 — consequences

### 6.1 Baseline / rot ledger (all counts measured, before-side at stashed HEAD)

| suite | before (HEAD) | after | delta |
|---|---|---|---|
| `astraweave-render --lib` | **1284 passed / 2 failed** | **1283 passed / 2 failed** | net −1 test by design (two presence tests → one absence test); failure set identical |
| ↳ those 2 failures | `test_water_renderer_wrong_format_rejected_at_install` + `test_water_renderer_new_and_update`, both `RequestDeviceError { Device(Lost) }` at device creation — **environmental**: they fail under full-suite device churn on this driver session *at clean HEAD too* (stash-proven), and **both pass when run in isolation** on the post-fix tree. Untouched by this beat. | same two | 0 |
| `astraweave-render --test "*" --no-fail-fast` | — | **89 targets: 3,217 passed / 1 failed** — incl. `test_pbr_brdf` (24), `test_terrain_material` (25), golden_postfx, visual_regression all green against the unified BRDF | the 1 failure is the row below |
| `shader_validation::test_all_shaders_compile` | **red at HEAD** (proven via stash: vacuity floor 58 < 60 with zero real shader failures) | red, identically | **pre-existing; liveness-guard class — flagged, NOT re-baked.** Needs its own investigation: which shaders left the standalone-validated set, and when. |
| `aw_editor --lib` | 4035 / 0 | **4035 / 0** | unchanged |
| `cargo check --workspace` | — | **Finished, exit 0** | clean |
| `astraweave-terrain` (T.G inheritance) | 2,440 / 64 / 15 (T.2a ledger) | not re-run — this beat changes shaders only; no CPU-side terrain code was touched | inherited unchanged |

**Golden-image rot:** the workspace has no golden-image suites; the rot from the far-field brightening lands on the *human-judged* artifacts instead — T.2a's and T.2c's far-field station frames are no longer representative beyond ~300 m (up to +9.5% luma at 1500 m). Their A/B **deltas** remain valid (identical cameras per leg — the offset cancels); their **absolute** far-field reads do not.

### 6.2 The re-judgment list (director decides; nothing was re-tuned)

1. **The T.2a "flat and shiny up close" complaint** — close range is pixel-identical through this fix, so the complaint now stands against a single, camera-independent BRDF. Re-judging it is now meaningful for the first time.
2. **T.2a station A/Bs** (NORMAL_XY_STRENGTH 1.8→1.4, hex pow 4→2) — comparisons remain valid; far-field absolute appearance shifted (+≤9.5% luma beyond ~300 m).
3. **T.2c close-up reads** (grassland shards, desert relief 4.3× Laplacian, tundra-too-dark, grassland-olive) — all judged at station distance 20, in territory this fix leaves pixel-identical; the verdicts carry over. They remain subject to the **§2.1 near-field gradient** (+5.9% over 12–110 m), which the tier deletion did NOT close — §3.2 proved the tiers were not its cause, and its residual (−3.7% after normal neutralization) is still open.
4. **Any distant-terrain judgment** (silhouette, biome banding, large-scale colour) — the LOD2 dithering is gone and the far field is brighter; prior far-field reads should be re-made on post-fix frames.
5. **The two director boundary frames themselves** — the closing check (§7).

### 6.3 What this beat did NOT fix, stated plainly

- The **§2.1 near-field gradient** (+5.9%, 12–110 m, camera-anchored) — its unattributed residual lives most plausibly in the specular `V`-dependence (§3.4). Open.
- The **far-field normal-variance gradient** (§9.1's sign; +13.5% over 300–1500 m post-fix) — Toksvig-class, cross-material, unratified. Open, by design.
- The **hex-tile lattice** — separate standing item. Untouched.

---

## 7. The closing check (director)

The five stations are pinned in `.editor_preferences.json` and load on editor start. The re-check that closes T.2d:

1. **Camera → t2df_boundary_y414 → Go**, look; then **t2df_boundary_y536 → Go**. The hard stationary boundary from the two 2026-07-25 frames should be absent (expected residual: smooth distance falloffs only).
2. **Camera → t2df_desert_close_20m → Go** — close-up check; this fix changed nothing here by measurement, so any remaining close-range complaint is the §2.1 gradient / material content, not the tiers.
3. Optionally **Camera → Capture ALL stations** for archived after-frames from the live editor itself.

---

## 8. Phase 5 — ED-3 handoff (recorded, not fixed — anti-drift)

Three settable-but-never-read surfaces, found during T.2d diagnosis, parked for the queued ED-3 beat:

1. **Viewport shading dropdown is inert.** `ShadingMode {Lit, Unlit, Wireframe}` (`toolbar.rs:455-464`) → `widget.rs:880` → `ViewportRenderer::render`'s **`_shading_mode: u32`** (`renderer.rs:570`), unused; `has_lighting()`/`is_wireframe()` have zero non-test callers.
2. **A second dropdown advertises Normals and UVs** (`tab_viewer/mod.rs:2082`), silently mapped to Lit (`main.rs:5208-5214`). These are the debug visualisations a renderer diagnosis reaches for first — never built. Highest-value ED-3 candidate.
3. **Terrain-panel Base Amplitude slider is dead** under the climate path: `terrain_panel.rs:1064` → `set_noise_params` → `config.noise.base_elevation.amplitude`, but `noise_gen.rs:575/:663` reads `params.base_elevation_amplitude`, blended from archetype splines (`regional_archetype_mask.rs:469`) — never from config. Proven: three amplitudes render byte-identical (t2d Experiment F). **T-series interaction: T.3's ratified gate asks the director to confirm amplitude finality — that gate is not actionable while the amplitude control does nothing.** ED-3 (or a T.3 precursor) must either re-plumb the slider into `BootstrapParams` or remove it and expose the archetype-spline authority.

---

## 9. Verification

| rung | result |
|---|---|
| `cargo fmt` (touched crates) | clean |
| `cargo check -p astraweave-render` / `-p aw_editor --tests` | exit 0 |
| `cargo check --workspace` | **Finished, exit 0** |
| `material_lod_tiers_are_retired` | **FAILED pre-fix / ok post-fix** (fails-on-old-code proven by execution) |
| `astraweave-render --lib` | 1283 / 2 — failure set identical to stashed HEAD (1284 / 2); both failures environmental device-loss, pass in isolation |
| `astraweave-render --test "*"` | 89 targets, **3,217 passed / 1 failed** (the pre-existing vacuity guard) |
| `aw_editor --lib` | **4035 passed / 0 failed** |
| shader validation | terrain-composition + entry-point + feature tests ok; the vacuity-guard red is pre-existing at HEAD (stash-proven) |
| residual tier symbols | grep: only docs + the absence test |
| capture/analysis artifacts | `d:/tmp/t2df_staging/{before,after}/` + `suites.log` + raw perf logs |
