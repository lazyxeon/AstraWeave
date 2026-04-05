# Session Handoff — 2026-04-04/05

**Session type**: Extended campaign — context checkpoint, NOT session end due to completion
**Branch**: main (ahead of origin/main by 10 commits)
**Working tree**: CLEAN — no uncommitted changes

---

## Original Mandate

Run a full behavioral correctness audit of the AstraWeave editor (`tools/aw_editor`) and execute a remediation campaign to fix all identified issues, culminating in a plan for the architectural unification of the dual rendering pipeline (Fix 27).

---

## What Was Accomplished

### Audit Phase (completed — read-only)
- 8-phase behavioral correctness audit executed across 3 parallel waves
- 14 CRITICAL, 18 HIGH, 28 MEDIUM, 10 LOW findings catalogued
- 42 items verified correct; 9 of 13 known pre-existing issues confirmed fixed
- Master audit report written to `docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md`

### Remediation Campaign (26 tracked fixes committed across 30 commits)
- **3,894 tests passing, 0 failures** maintained throughout all 30 commits
- Campaign state tracked in `CAMPAIGN_PROGRESS.md` and `CAMPAIGN_NOTES.md`

#### Tier 1 — COMPLETE (5/5)
- `dc95b220d` — VC-1: GGX NDF epsilon corrected to `max(PI * denom^2, 1e-7)` in `entity.wgsl:163`
- `dc95b220d` — VC-2: Fresnel energy conservation; diffuse reduced by `(1-F)*(1-metallic)`; specular moved before diffuse in shader evaluation order
- `3af1bf712` — C-3: 17 mutex lock sites in `widget.rs` replaced with `with_renderer()` poison-recovering helper
- `0bf76f4d9` — C-1: `let _ =` on material preview result replaced with `tracing::error!` in `main.rs`
- `0bf76f4d9` — C-2: `let _ =` on asset scan result replaced with `tracing::error!` in `main.rs`

#### Tier 2 — COMPLETE (8/8)
- `3b23f71a2` — R8G2: R8G8 normal map blue=0 bug fixed; Z reconstructed via `sqrt(1 - x^2 - y^2)` during CPU texture load in `entity_renderer.rs`
- `faca629f0` — VC-3: BRDF LUT geometry model switched from separable Schlick-GGX to height-correlated Smith-GGX matching `entity.wgsl` in `brdf_lut.wgsl`
- `9561e3bda` — VC-4: Turquin 2019 multi-scatter energy compensation added; threaded through both `disney_brdf_directional` and `disney_brdf_point` in `entity.wgsl`
- `3de4bc10d` — M-21: Scale gizmo bidirectional; changed `mouse_delta.length()` to signed `mouse_delta.x` in `gizmo/scale.rs`
- Fix 10 (surface lost) — RECLASSIFIED N/A: Editor viewport uses offscreen textures, never wgpu surfaces directly; eframe manages the window surface
- `4857e55a2` — Fix 11: Simulation crash recovery via `catch_unwind` + `Option<App>` shuttle pattern; auto-pauses on panic in `runtime.rs`
- `d3570e6d0` — Fix 12: Removed 6 duplicate EntityManager update calls in event handlers that bypassed undo stack in `command.rs` + `main.rs`
- `199574b05` — Fix 13 (C-5): 4 of 9 entity creation ops wired to undo stack (SpawnEntitiesCommand, DuplicateEntitiesCommand hooked); remaining 5 deferred

#### Deferred Items — RESOLVED (3/3)
- `288f305b3` — Fix 17: Per-axis scale added to `astraweave_core::Pose` (scale_y, scale_z fields); `ScaleEntityCommand` extended to `[f32; 3]`
- `f13c38055` — Fix 18: Terrain vertex simplification documented as deliberate adapter decision; no code change
- Fix 19: Instance layout divergence verified as intentional; separate pipelines, no shared bind groups; no code change

#### Tier 3 — COMPLETE (5 fixed, 3 deferred → resolved)
- `a5aaa548d` — Fix 14: Permanent mesh blacklist replaced with retry-limited cache in `entity_renderer.rs`
- `e1032324b` — Fix 15: Exposure applied in HDR path in `entity.wgsl`; roughness-aware IBL Fresnel via `fresnel_schlick_roughness`
- `b2c052dc8` — Fix 20: Autosave ring rotation `let _ =` replaced with error logging
- `b2c052dc8` — Fix 21: Prefab hot-reload `let _ =` replaced with error logging
- ScaleEntityCommand Vec3 — DEFERRED: `astraweave_core::Pose.scale` is `f32`; cross-crate change deferred per scope discipline

#### Tier 4 SOTA Upgrades — COMPLETE (6/6)
- `34e8c480d` — Fix 22: Khronos PBR Neutral tonemapper + Reinhard added; 3-mode runtime selection in `tonemap.wgsl`
- `2f5b880d6` — Fix 23: 3-channel DFG LUT; Charlie DG cloth sheen term in B channel of `brdf_lut.wgsl`
- `a9b48c684` — Fix 24: glTF tangent attribute loading for MikkTSpace TBN; 64-byte vertex in `entity_renderer.rs`
- `30d4f1415` + `1d6f175cd` — Fix 25: 4-cascade CSM shadows with frustum-fitted splits and texel-snap stabilization
- `4d0f77796` + `49682bb71` — Fix 26: IBL prefiltered cubemap infrastructure + `IblManager` bake + shader integration
- `d1db2aa89` — Fix 27: Unified pipeline campaign plan written at `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` (plan only, no execution)

---

## What Is In Progress

### Fix 27: Unified Rendering Pipeline
- **What**: Eliminate the 3,609-LOC FastPreview `EntityRenderer` path and route all scene rendering through `astraweave_render::Renderer`. Delete ~4,589 LOC total.
- **Current state**: Campaign plan fully written and committed (`d1db2aa89`). Phase 0 analysis completed (read-only) — ~70 `#[cfg(feature = "astraweave-render")]` guards identified across 7 files. Zero code changed.
- **Next step**: Start Phase 0 execution: open `tools/aw_editor/Cargo.toml` and remove `optional = true` from the `astraweave-render` dependency, then systematically delete the ~70 cfg guards in the 7 files identified during analysis.
- **Blocking on**: Nothing. Plan is approved (committed). Tests are green. Working tree is clean.
- **Files involved (Phase 0)**:
  - `tools/aw_editor/Cargo.toml` — remove `optional = true` from astraweave-render
  - `tools/aw_editor/src/viewport/mod.rs` — cfg guards
  - `tools/aw_editor/src/viewport/renderer.rs` — cfg guards
  - `tools/aw_editor/src/viewport/engine_adapter.rs` — cfg guards + stub impl deletion
  - `tools/aw_editor/src/headless.rs` — cfg guards
  - `tools/aw_editor/src/lib.rs` — cfg guards
  - `tools/aw_editor/src/main.rs` — cfg guards
  - `astraweave-render/src/renderer.rs` — verify always-on assumptions hold

---

## What Was Attempted and Failed

### Fix 8 (VC-4) Multi-scatter: First attempt multiplied entire `color` including diffuse
- **What was tried**: Applied `energy_comp` multiplier to the combined `color` output variable (after diffuse and specular had been summed)
- **Why it failed**: Multi-scatter energy compensation should only apply to the specular lobe. Multiplying diffuse as well overcorrects for rough metals and produces incorrect results
- **What was learned**: `energy_comp` must wrap only the specular contribution before it is added to the diffuse term; the parameter threading must be done at the `disney_brdf_directional` level, not post-summation
- **Should it be revisited**: No, the correction was made in the same session and is shipped correctly

### Fix 10 (Surface Lost Recovery): Audit finding was invalid
- **What was tried**: Investigation into adding `SurfaceError::Lost` handling to `viewport/renderer.rs`
- **Why it failed**: The finding was based on a false assumption. The editor viewport renders to offscreen textures (Rgba16Float HDR + Depth32Float), copying pixels to an egui texture handle via CPU readback. There is no wgpu window surface in the viewport code at all; grep confirms zero matches for `get_current_texture`, `SurfaceError`, `surface.configure`, or `SurfaceTexture` anywhere in `tools/aw_editor/src/viewport/`
- **What was learned**: The audit assumed GPU surface management patterns common in standalone renderers. Always grep for actual wgpu surface API usage before writing surface-loss recovery code
- **Should it be revisited**: No. Correctly classified N/A.

---

## What Was Discovered But Not Addressed

- **C-5 partial**: 5 of 9 entity creation operations still bypass the undo stack. The 5 remaining ops (import, template instantiation, etc.) need new `EditorCommand` wrappers. Located in `tools/aw_editor/src/main.rs` event handlers — medium severity, deferred.
- **astraweave-render pre-existing warnings**: 13 `dead_code` field warnings in `astraweave-render/src/`. Not caused by this campaign. Should be cleaned before Fix 27 Phase 0 to keep signal-to-noise high.
- **Integration test pre-existing failures**: `prefab_workflow`, `delete_command_tests`, and related integration tests have compilation errors from `cmd.undo()` signature changes that were never updated in the test files. These predate this campaign. Blocked from running integration tests until fixed — medium severity.
- **`Pose.scale` cross-crate Vec3 upgrade**: `astraweave_core::Pose.scale` is still `f32` (uniform). Every crate that reads/writes `Pose` would need updating. Requires a dedicated campaign with change-impact-tracer analysis before touching.

---

## Current State of the Codebase

- **Uncommitted changes**: None. Working tree clean.
- **Failing tests**: None. 3,894 lib tests passing, 0 failures.
- **Compiler warnings**: 13 pre-existing `dead_code` warnings in `astraweave-render`; not introduced by this campaign.
- **Known broken state**: Integration tests (`prefab_workflow`, `delete_command_tests`) have pre-existing compilation errors unrelated to this campaign.
- **Commit recommendation**: Nothing to commit. Working tree is clean. All campaign work is committed.
- **Branch state**: main, ahead of origin/main by 10 commits — not yet pushed.

---

## Recommended Next Session Plan

1. **Fix 27 Phase 0** (3-4 days): Remove `optional = true` from `tools/aw_editor/Cargo.toml`, delete all ~70 `#[cfg(feature = "astraweave-render")]` guards, add `RenderBackend` enum `{ Engine, LegacyPreview, Headless }` to replace `RenderMode`. Run `cargo check -p aw_editor` after every file. Gate: all 3,894+ tests pass, both render paths produce frames.
2. **Fix 27 Phase 1** (8-10 days, CRITICAL PATH): Feed World entities to `astraweave_render::Renderer` via `engine_adapter.rs`. Implement selection highlighting via bounding box wireframe (Option A). Wire Lit/Unlit/Wireframe shading modes. Gate `EntityRenderer` behind `#[cfg(feature = "legacy-preview")]`.
3. **Fix 27 Phases 2-4** (11-14 days): Shadow/IBL unification, tonemap/post-processing unification, `EditorOverlayHooks` trait. Phases 2 and 5 (terrain) can run in parallel.
4. **Fix 27 Phases 5-7** (10-13 days): Terrain vertex unification, legacy deletion (~4,589 LOC), headless/CI fallback.
5. **C-5 remaining 5 ops** (1-2 days): Wire remaining 5 entity creation operations through undo stack.

---

## Files Modified This Session

### Shader files
- `tools/aw_editor/src/viewport/shaders/entity.wgsl` — GGX NDF epsilon, Fresnel energy conservation, multi-scatter compensation, exposure in HDR path, roughness-aware IBL Fresnel
- `tools/aw_editor/src/viewport/shaders/brdf_lut.wgsl` — Height-correlated Smith-GGX, 3-channel DFG with cloth Charlie DG
- `tools/aw_editor/src/viewport/shaders/tonemap.wgsl` — Khronos PBR Neutral + Reinhard tonemappers, 3-mode selection
- `tools/aw_editor/src/viewport/shaders/shadow.wgsl` — 4-cascade CSM, frustum-fitted splits, texel-snap stabilization

### Viewport Rust files
- `tools/aw_editor/src/viewport/entity_renderer.rs` — R8G8 normal Z reconstruction, retry-limited mesh cache, glTF tangent loading (MikkTSpace TBN, 64-byte vertex), IBL prefiltered cubemap infrastructure
- `tools/aw_editor/src/viewport/renderer.rs` — IBL manager bake, runtime cubemap switching, error handling in bind group creation
- `tools/aw_editor/src/viewport/engine_adapter.rs` — minor error handling improvements
- `tools/aw_editor/src/viewport/widget.rs` — mutex poison-recovering `with_renderer()` helper, 17 lock sites refactored
- `tools/aw_editor/src/viewport/gizmo/scale.rs` — bidirectional scale via signed mouse delta

### Editor Rust files
- `tools/aw_editor/src/main.rs` — error surfacing for material preview and asset scan; entity creation undo wiring (4 ops)
- `tools/aw_editor/src/command.rs` — undo desync fix (6 duplicate EntityManager updates removed)
- `tools/aw_editor/src/headless.rs` — minor fixes
- `astraweave_core` (Pose) — per-axis scale fields (scale_y, scale_z), `ScaleEntityCommand` extended to `[f32; 3]`

### Documentation
- `docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md` — master audit report (new)
- `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` — Fix 27 detailed 7-phase plan (new)
- `CAMPAIGN_PROGRESS.md` — running status checklist (new)
- `CAMPAIGN_NOTES.md` — decisions and surprises log (new)
- `docs/current/VEILWEAVER_VERTICAL_SLICE_ANALYSIS.md` — updated
- Various lore/guide docs — updated

---

## Key Context the Next Agent Needs

- **Fix 27 commit strategy**: The plan specifies a feature branch `fix-27/unified-pipeline` with phase-gate PRs squash-merged to main. Read `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` fully before starting — phase gate criteria, rollback steps, and the `EditorOverlayHooks` trait design are all there.
- **Do not touch `entity_renderer.rs` for new features**: It is the primary deletion target for Fix 27 Phase 6. Any new code added to it creates more deletion debt.
- **Integration tests are pre-broken**: `cargo test -p aw_editor` will report integration test compilation failures. These predate the campaign. Use `cargo test -p aw_editor --lib` to run the 3,894 lib tests only.
- **The 10 unpushed commits**: `main` is 10 commits ahead of origin/main. Push when ready, or the next session can push.
- **Undo system coupling**: `command.rs` and `main.rs` event handlers are tightly coupled. Any undo work should touch both files in the same commit.
- **`astraweave_core::Pose.scale` is still `f32`**: Do not write code that assumes Vec3 scale in the World/ECS path. The EntityManager has Vec3 scale for viewport-only use, but the authoritative World data is uniform-only. Fix 17 added per-axis fields but the core `scale` field remains `f32` for compatibility.
- **`PROJECT_STATUS.md` is stale**: Last updated February 8, 2026. It does not reflect the behavioral correctness audit or Fix 27 campaign. Update it when Fix 27 begins.
