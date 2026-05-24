# Camera Conventions

| Field | Value |
|-------|-------|
| Status | **CANONICAL** — authoritative reference for camera conventions across the workspace |
| Sub-phase | C.1 (Unified Camera campaign) |
| Established | 2026-05-18 |
| Contract tests | `astraweave-render/tests/camera_conventions.rs` |
| Audit precedent | `docs/audits/camera_system_architecture_audit_2026-05.md` (C.0, pre-campaign) and `docs/audits/camera_system_phase_2_audit_2026-05.md` (C.5, mid-migration) |
| Prior research | `docs/current/CAMERA_SYSTEMS_SOTA_AUDIT_AND_RECOMMENDATIONS.md` (2026-04-28) |

## §1 — Purpose

This document is the canonical reference for camera conventions across `astraweave-render`, `astraweave-camera` (when created in C.2), `tools/aw_editor/`, and all example crates. Any production camera implementation must comply with these conventions. **Discrepancies between code and this doc are bugs in the code, not the doc.**

The conventions here are the result of C.0's Phase 1–5 audit (which inventoried 8 active 3D camera codepaths plus cinematics keyframes and identified major divergence on yaw=0 forward direction across three competing conventions) and the C.0 Phase 6 gate decision to consolidate via a unified camera campaign. C.1 establishes conventions; subsequent sub-phases C.2–C.9 progressively migrate non-canonical implementations to comply (see §3, §5).

The contract tests in `astraweave-render/tests/camera_conventions.rs` (§4) assert that engine `Camera` (the production runtime camera, in `astraweave-render/src/camera.rs`) complies with every non-deferred convention in §2. Future changes to engine `Camera` that violate a documented convention will fail these tests — that is the convention's structural protection.

## §2 — Conventions

The 9 axes follow the C.0 Phase 2 numbering. Each subsection states the chosen convention, the reasoning, current-code citations, and how boundary conversions are handled when a non-canonical input must be accepted (e.g., user-facing degrees converted at the UI boundary).

### §2.1 FOV semantics

**Convention**: vertical FOV in **radians**, field name `fovy`.

Boundary conversions: degrees-based inputs (UI sliders, serialized scene files, user-facing config) convert to radians at the boundary via `.to_radians()` *before* assigning to `fovy`. The canonical type never stores degrees internally. Field names other than `fovy` (e.g., `fov`, `fov_deg`, `fov_rad`) are non-canonical and must migrate.

**Reasoning**: matches glam's `Mat4::perspective_rh` input (radians). Eliminates the radians-vs-degrees-at-call-site question that produces silent bugs (a literal `45.0` passed where radians is expected yields ~2580° FOV, which `perspective_rh` accepts without complaint and produces a degenerate matrix). C.0 Phase 3 §3.9 documented `apply_camera_key` silently converting `fov_deg = 0` and `fov_deg = 180` to degenerate projections — a latent bug that the canonical-radians convention does not eliminate but the field-name discipline (`fovy`, not `fov_deg`) signals more clearly at the consumer site.

**Citation**: `astraweave-render/src/camera.rs:7` defines `fovy: f32` storing radians, consumed at line 24 via `Mat4::perspective_rh(self.fovy, ...)`.

### §2.2 Near/far plane handling

**Convention**: standard wgpu **[0, 1] depth range, forward-Z** (not reversed-Z). Construction via `Mat4::perspective_rh`. Near plane must be `> 0`. Far plane must be `> near`.

Boundary conversions: none — wgpu's pipelines default to [0, 1] depth; all production cameras use this. Reversed-Z (better float distribution at far ranges) is a separate architectural decision out of this campaign's scope.

**Reasoning**: matches wgpu's native depth convention. `Mat4::perspective_rh` from glam produces matrices that map view-space `z = -near` to NDC `z = 0` and view-space `z = -far` to NDC `z = 1`. The shadow CSM extraction (`astraweave-render/src/shadow_csm.rs`) and the editor's `Frustum::extract_near_plane` (`tools/aw_editor/src/viewport/camera.rs:718-729`) both encode the [0, 1] assumption directly — switching to reversed-Z would require coordinated changes to these consumers and is therefore out of C.1 scope. C.8 may revisit if performance evidence warrants.

**Citation**: `Mat4::perspective_rh` used at `astraweave-render/src/camera.rs:24` and `tools/aw_editor/src/viewport/camera.rs:458`. wgpu [0, 1] depth confirmed by `extract_near_plane` extracting row 2 (not row3+row2) at `tools/aw_editor/src/viewport/camera.rs:718-729`.

### §2.3 Aspect ratio handling

**Convention**: `width / height` ratio, stored as `f32`, **guarded by `.max(0.01)` floor at projection construction time**. Resize handlers must clamp `height >= 1` (or check `height > 0.0`) before computing `aspect = width / height`. NaN values are clamped via sanitize() in any type that supports deserialization (e.g., editor `OrbitCamera`'s sanitize() at `tools/aw_editor/src/viewport/camera.rs:563-622`).

Boundary conversions: viewport resize is the dominant boundary. Resize handlers must not let `aspect` reach 0 or NaN. The `.max(0.01)` floor at projection time is the defense-in-depth: even if a caller forgets the resize guard, the projection matrix won't NaN-out.

**Reasoning**: divide-by-zero from a zero-height window (minimize, hidden tab) is the most common aspect-related failure mode. Engine `Camera::proj_matrix` uses `.max(0.01)`; editor `OrbitCamera::set_aspect` checks `height > 0.0`. C.0 §3.6 (fluids_demo aspect_zero_guard absent) and C.0 §3.5 (shadow_csm_demo no guard) listed examples that lack this protection — they migrate at C.6.

**Citation**: `astraweave-render/src/camera.rs:24` shows `self.aspect.max(0.01)`. `tools/aw_editor/src/viewport/camera.rs:363-367` shows `set_aspect` with `if height > 0.0` guard.

### §2.4 Coordinate handedness

**Convention**: **right-handed coordinate system, +Y up**. All view matrices must construct via `Mat4::look_at_rh` or `Mat4::look_to_rh` with `Vec3::Y` (positive) as the up vector. **Never `-Vec3::Y`.**

Boundary conversions: imported assets (glTF defaults to RH +Y up; .blend may use Z up) must convert at the import boundary. No runtime camera ever sees a -Y up input.

**Reasoning**: matches glam's right-handed convention, wgpu's clip-space convention, and the majority of game-engine practice. The `-Vec3::Y` bug ("chunk-aligned rectangular voids in terrain") was fixed in commit `df7649287` for engine `Camera`; the tombstone comment at `astraweave-render/src/camera.rs:17-20` documents the failure mode. The bench mock at `astraweave-render/benches/camera_primitives_instancing.rs:35` retained the bug until C.1 Deliverable C; the contract test in `tests/camera_conventions.rs` (§4) prevents regression.

**Citation**: `astraweave-render/src/camera.rs:20`: `Mat4::look_to_rh(self.position, dir, Vec3::Y)`. `tools/aw_editor/src/viewport/camera.rs:451`: `Mat4::look_at_rh(self.position(), self.focal_point, Vec3::Y)`.

### §2.5 View matrix construction style

**Convention**: **two acceptable styles for canonical types** — `look_to_rh(eye, dir, up)` (direction-based, for FreeFly producers) and `look_at_rh(eye, target, up)` (target-based, for Orbit/Follow/Cinematic producers). Both produce equivalent matrices for equivalent inputs; the choice is a producer-side convenience.

Boundary conversions: when a producer needs to switch styles (rare), the conversion is `look_to_rh(eye, (target - eye).normalize(), up) == look_at_rh(eye, target, up)`.

**Reasoning**: forcing one style would inconvenience the dominant use cases. FreeFly cameras naturally yield a direction vector (from yaw/pitch composition); orbit cameras naturally yield a target (the focal point). Both glam functions are equally well-tested and produce identical matrices for equivalent inputs.

**Citation**: `astraweave-render/src/camera.rs:20` uses `look_to_rh`; `tools/aw_editor/src/viewport/camera.rs:451` uses `look_at_rh`. Both consume Vec3::Y up; the result feeds the same `Renderer::update_camera_matrices` upload path.

### §2.6 Projection matrix construction

**Convention**: **`Mat4::perspective_rh` only** (glam's wgpu-compatible variant producing [0, 1] depth). **Never `Mat4::perspective_rh_gl`** (which produces [-1, 1] depth for OpenGL compatibility — incompatible with wgpu shaders).

Boundary conversions: none. The two function names differ by one suffix and the wrong one produces a silently-incorrect depth buffer that misbehaves with shadow mapping, depth-based picking, and CSM cascade extraction. C.0 audit confirmed no `perspective_rh_gl` usage in production. The contract test in §4 includes a depth-range assertion that would fail under `perspective_rh_gl`.

**Reasoning**: wgpu's clip-space depth is [0, 1] by convention; `perspective_rh_gl` exists for OpenGL ports and is wrong for wgpu. Documenting this convention prevents accidental use during refactors or copy-paste from OpenGL tutorials.

**Citation**: All Rust-side projection constructions found in C.0 audit use `Mat4::perspective_rh`. Zero usages of `perspective_rh_gl` confirmed via workspace grep.

### §2.7 Jitter (TAA) handling — *DEFERRED*

**Convention**: **no canonical convention yet — TAA is not active in production.**

When TAA lands, jitter is applied as a post-multiply onto the canonical projection matrix:

```
jittered_proj = jitter_matrix * canonical_proj
```

where `jitter_matrix` is a tiny sub-pixel translation in clip space. Previous-frame matrices (for motion vectors and disocclusion) become part of the canonical `RenderView` (see §2.9) when TAA enters the codebase.

**Reasoning**: C.0 §2.7 found no production TAA path. Documenting the deferral here prevents premature optimization or speculative API design. The convention is established when TAA enters the codebase — at that point, this section is revised in-place and the contract tests gain a jitter axis. TAA-related code paths in `astraweave-render` (e.g., `advanced-post` feature flag) are gated off by default and do not currently apply jitter.

**Citation**: `astraweave-render/Cargo.toml:22` defines `advanced-post = []` feature; not active in default builds. No production camera currently calls a jitter-application function.

### §2.8 Yaw=0 forward direction

**Convention**: **+X**. The canonical direction at `yaw = 0, pitch = 0` is `Vec3::X = (1, 0, 0)`. The direction formula is:

```rust
dir(yaw, pitch) = Vec3::new(
    yaw.cos() * pitch.cos(),
    pitch.sin(),
    yaw.sin() * pitch.cos(),
).normalize()
```

Boundary conversions: implementations using -Z forward (Bevy/glTF/Three.js convention) or -X forward (orbit-offset semantic where camera looks from +X toward origin) must convert at their boundary. Specifically:
- glTF camera nodes (which use -Z forward) convert by rotating the import transform by π around Y.
- Editor `OrbitCamera`'s `to_engine_camera()` already implements the -X→+X conversion via `yaw + π, -pitch` at `tools/aw_editor/src/viewport/camera.rs:631-632`.
- unified_showcase's bespoke camera (using -Z forward at `examples/unified_showcase/src/main.rs:2162-2164`) does NOT currently convert; it operates outside the engine's authoritative pipeline. Migration at C.6 either aligns the example to canonical or wraps the example's camera in a boundary converter.

**Reasoning**: +X forward at yaw=0 is the convention engine `Camera::dir` has used since the `df7649287` fix. It is documented in `astraweave-render/tests/wave2_camera_remediation.rs` tests (`dir_yaw0_pitch0_is_positive_x`, `dir_yaw_half_pi_is_positive_z`, etc.) and is the convention the production runtime expects. Three conventions exist in the codebase today (+X, -Z, -X per C.0 §2 axis 8); picking +X canonical lets the dominant production-runtime convention define the workspace standard rather than the example or editor variants.

**Citation**: `astraweave-render/src/camera.rs:31-38`: `dir(0.0, 0.0) = (1, 0, 0) = +X`. Confirmed by `astraweave-render/tests/wave2_camera_remediation.rs:30-39` `dir_yaw0_pitch0_is_positive_x`.

### §2.9 Interface contracts — `RenderView`

**Convention**: canonical types (in `astraweave-camera`, created in C.2) expose **`RenderView { view: Mat4, projection: Mat4, position: Vec3, view_dir: Vec3 }`** as the minimum upload contract. Producers may expose richer surfaces (orbit-specific picking methods, follow-camera target accessors, cinematic blend states) but must always be able to produce a `RenderView`. The renderer consumes `RenderView` exclusively — no per-producer-type renderer APIs.

Boundary conversions: this is the consolidation contract; it does not exist yet in C.1. C.2 creates the type; C.3 migrates the renderer's `update_camera`/`update_camera_matrices` duplicate APIs into a single `update_render_view(RenderView)` entry point. The contract tests for §2.9 are written in C.2 against the new types — C.1's contract tests cover §2.1–§2.6, §2.8 only.

**Reasoning**: per C.0 SOTA audit §4 ("Renderer Consumes RenderView Only") and the C.0 audit doc's §4.A (the dual `update_camera`/`update_camera_matrices` paths are accidental accretion). Single upload contract eliminates the dual-path side-effect-alignment risk (every change to upload behavior must currently update both functions). Producers retain their distinct ergonomic surfaces (`OrbitCamera::ray_from_screen`, `FreeFlyController::process_mouse_delta`, `FollowRig::set_target`) but converge at `to_render_view()`.

**Citation**: target shape, not yet implemented. C.2 will land this. Per anti-drift constraint 1, C.1 does not create the crate or the type.

## §3 — Non-canonical conventions in the codebase

**Status log:**

- **C.2 closed (2026-05-18):** canonical types `Projection`, `RenderView`, and trait `CameraProducer` exist in `astraweave-camera`. The §2.9 convention has a Rust-side referent — previously target-shape only. C.2 prepared the destination; no production consumer.

- **C.3.A closed (2026-05-18):** engine `Camera` → `astraweave_camera::FreeFly` (re-exported via `astraweave-render/src/camera.rs` shim as `Camera` for backward-compat). `Renderer::update_view(&RenderView)` canonical upload path exists alongside `#[deprecated]` wrappers `update_camera` / `update_camera_matrices`. `update_cascade_splits` + `frustum_corners_ws` consume `&RenderView` directly (lossy yaw/pitch reconstruction at renderer.rs:4001 eliminated). Parity harness verified byte-equivalence between old wrapper path and new canonical path via dual-test pattern.

- **C.3.B.1 closed (2026-05-18):** all editor-side callers migrated to `Renderer::update_view` directly. Parity harness's canonical-default `editor_engine_render_parity` test now exercises `update_view`; sibling test renamed to `update_camera_matrices_wrapper_preserves_behavior` and reframed to verify the deprecated wrapper preserves behavior during the C.3.B.1 → C.3.B.2 → C.3.C transition window. `EngineRenderAdapter::update_camera` constructs `RenderView` directly via `Projection::perspective` + `RenderView::new` (no `to_engine_camera` intermediate). `EngineRenderAdapter::update_water` signature migrated to accept `&RenderView`; function body simplified. Reachability verified zero callers in inspected tree per C.0 §1.B #8 — deletion queued as standalone follow-up post-campaign. Deprecation warning count dropped from 11 → 10 (editor-side adapter site closed; 10 engine-side example sites remain for C.3.B.2). §3 migration table rows for "EngineRenderAdapter::update_water dual-conversion" fully closed. "Renderer dual upload paths" row partially closed (editor-side wrappers no longer have callers; engine-side wrappers still do).

- **C.3.B.2 closed (2026-05-18):** all remaining callers migrated to `Renderer::update_view` directly. 13 production example call sites + 7 engine-internal test/example sites migrated via `renderer.update_camera(&cam)` → `renderer.update_view(&cam.to_render_view())` (20 sites total — broader than C.3.B.1's expected 10–13 inventory because prior audits didn't grep test files; Andrew approved the expanded scope mid-execution). The `CameraProducer` trait re-exported at `astraweave_render::CameraProducer` (lib.rs additive, shim unchanged per anti-drift constraint 3) so callers don't need `astraweave-camera` as a direct dependency. Cinematics investigation (Phase 2) found `tick_cinematics` and `apply_camera_key` operate on a caller-provided `&mut Camera` with no internal renderer→`update_camera` chain — the cinematics renderer-side migration was the SUBSEQUENT `renderer.update_camera(&cam)` issued by `tick_cinematics` callers, all captured in the 20-site inventory. **A C.3.A regression fix landed alongside**: `test_frustum_corners_ws` at `astraweave-render/src/renderer.rs:7584` was broken by C.3.A's `frustum_corners_ws(&Camera)` → `frustum_corners_ws(&RenderView)` signature migration (the test caller wasn't updated then); the fix is mechanical (build a `RenderView` via `to_render_view()`). Deprecation warning count dropped from 10 → 0. Direct grep confirms: zero remaining `renderer.update_camera(` or `renderer.update_camera_matrices(` calls outside the intentional `update_camera_matrices_wrapper_preserves_behavior` test site at `render_parity_harness.rs:535`. §3 migration table row "Renderer dual upload paths" fully closed (no callers remain on the deprecated wrappers; wrappers themselves still exist as backward-compatibility code until C.3.C deletes them).

- **C.3.C closed (2026-05-18):** Renderer-side consolidation chapter complete. Pure subtraction sub-phase. Deleted symbols: `Renderer::update_camera`, `Renderer::update_camera_matrices`, `OrbitCamera::to_engine_camera` (prompt's Deliverable B specified `FreeFly::to_engine_camera` but the actual method was on `OrbitCamera` in `tools/aw_editor/src/viewport/camera.rs:624` — pure inversion of the prompt's text; deletion intent unchanged), `astraweave-render/src/camera.rs` shim (file removed entirely), `pub mod camera;` and `pub use astraweave_camera::CameraProducer;` re-exports at `astraweave-render/src/lib.rs`, `update_camera_matrices_wrapper_preserves_behavior` parity test, `CameraUploadPath` enum. Caller migration shape: every file with `use astraweave_render::camera::Camera` or `use astraweave_render::Camera` (alone or in multi-line) migrated to `use astraweave_camera::FreeFly as Camera;` (alias preserved at per-file scope so body type references stay `Camera` — anti-drift constraint 1 forbids global aliases like `pub type Camera = FreeFly;` or re-exports, NOT per-file local import aliases which are pure migration ergonomic). `astraweave-camera` added to 11 example Cargo.toml files as a direct dep (previously transitive through `astraweave-render`). Renderer crate's public surface is now: `Renderer::update_view(&RenderView)` as the canonical (and only) camera-upload entry point. `astraweave-camera` is the canonical home for all camera producer types and traits; `astraweave-render` consumes them as a regular dependency without re-exporting. §3 migration table row "Renderer dual upload paths" fully **CLOSED** (wrappers deleted; no caller path exists). Remaining §3 rows scoped for C.4 (OrbitCamera), C.6 (gizmo + examples), C.7 (cinematics CameraKey).

- **C.4 closed (2026-05-24):** First editor-side sub-phase. Editor `OrbitCamera` adopts `astraweave-camera`. `OrbitCamera` implements `CameraProducer` with two methods (`to_render_view` world-relative trait method; `to_render_view_camera_relative` concrete-only) mirroring `FreeFly`'s pattern from C.3.A; both reuse the existing `position()` method per Decision 2 (no inline spherical-to-cartesian math). `EngineRenderAdapter::update_camera` simplified from C.3.B.1's inline `RenderView` construction to single-line delegation through the canonical contract (`self.renderer.update_view(&camera.to_render_view())`); variant: world-relative trait method because the editor's main render path runs in world space (the `camera-relative` feature on `astraweave-render` is not in the editor's default feature set; `Renderer::update_view` stores the supplied `view_proj` directly in the camera UBO without per-pipeline transformation). Latent picking-vs-depth VP mismatch from C.0 §3.2 fixed: `ray_from_screen` migrated from absolute VP inversion to the precision-stable camera-relative VP + position-translation pattern, matching the discipline already used by `unproject_depth_to_world`. Phase 1 inspection clarified that the pre-fix divergence was a **float-precision** issue (not a depth-buffer coord-space mismatch as C.0's framing suggested) — clip-space output is invariant across world-space vs camera-relative VP pipelines, but the absolute VP's translation column grows with `|position()|`, losing precision at large camera world positions; the precision-stable path translates after a near-zero-translation inversion. Three closure proofs: (1) parity harness `editor_engine_render_parity` 1/1 passing (migration byte-equivalence — twelfth Pillar 5-refinement reinforcement); (2) new `picking_consistency.rs` 2/2 passing (`ray_from_screen` and `unproject_depth_to_world` agree at both modest and large camera world positions — new closure-proof shape); (3) new `orbit_camera_producer.rs` 5/5 passing (trait implementation; `RenderView::position` matches `position()` method; world-relative trait method uses `view_matrix()`; camera-relative concrete method uses `view_matrix_relative()`; `fov: degrees` correctly converts to `RenderView::fovy: radians` at the producer boundary — new closure-proof shape). Two-camera architecture (FreeFly engine in `astraweave-camera` + OrbitCamera editor in `tools/aw_editor/`) documented in both Jekyll (`gh-pages/rendering.md`) and mdBook (`docs/src/reference/crates.md`, `docs/src/core-systems/rendering.md`) surfaces. **Field-rename deferred**: `OrbitCamera.fov: degrees` → `fovy: radians` is deferred to sub-phase C.4.B per Decision 3 (structural rename touches OrbitCamera struct + constructors + UI bindings + serialization; scope warrants its own commit).

- **C.5 closed (2026-05-24):** Phase 2 audit document committed at `docs/audits/camera_system_phase_2_audit_2026-05.md`. Pure-audit sub-phase (no source changes). Inventoried 5 in-scope migration targets (gizmo `CameraController` + 4 per-example bespoke cameras), 19 latent issues, and 3 parallel cinematics camera systems (canonical `astraweave_cinematics::CameraKey`, editor `CameraKeyframe`, and `astraweave_gameplay::cutscenes::Cue::CameraTo` — new finding vs C.0). Three of C.0's medium-confidence "likely missing pitch clamp" findings (shadow_csm_demo, fluids_demo, nanite_demo) **falsified** by empirical re-inspection — clamps present in all three. The unified_showcase pitch-clamp finding **confirmed** missing. Major new finding: gizmo `CameraController` is structurally instantiated in production via `TransformPanel.camera` but the field is never read; the type is functionally dormant. Audit produces draft C.6 migration queue (5 per-target clusters) and 10 open questions for the planning round; decision-locking deferred per C.5 Decision 3's α. Cinematics inventory restated as forward-looking informational content for C.7's eventual planning round (Decision 1's Medium scope). Second audit-shaped sub-phase in the campaign (first was C.0); same closure proof family — audit-document-completeness — applied at a different phase boundary.

- **C.4.B closed (2026-05-24):** Pure mechanical field rename closing C.4's deferred concern. `OrbitCamera.fov: f32` (degrees) renamed to `OrbitCamera.fovy: f32` (radians); field convention now matches CAMERA_CONVENTIONS.md §2.1 directly. Setter API `set_fov(degrees: f32)` preserved per Decision 1 (Option α) — boundary stays in degrees (UI slider widget convention), internal storage in radians. Added getters: `fov_degrees() -> f32` (returns `self.fovy.to_degrees()`, for UI read paths) and `fovy() -> f32` (returns `self.fovy`, for radian-aware callers). Internal references updated in `projection_matrix`, `sanitize` (clamp range converted to `[10°.to_radians(), 170°.to_radians()]`), and both `to_render_view` impls (`.to_radians()` boundary conversions removed — the field is already canonical units). Editor UI write path at `tools/aw_editor/src/main.rs:4822` migrated from direct field assignment `cam.fov = fov` to `cam.set_fov(fov)` (degrees boundary preserved at the call site; conversion to radians happens inside the setter). **Serialization backward-compat per Decision 2 (Option α)**: `OrbitCamera` deserializes via a shadow type `OrbitCameraSerde` with `#[serde(from = "OrbitCameraSerde")]`; the shadow holds both `fov: Option<f32>` (legacy degrees, pre-C.4.B) and `fovy: Option<f32>` (canonical radians, post-C.4.B) as deserialization fields; `From<OrbitCameraSerde> for OrbitCamera` resolves by preferring `fovy` if present, falling back to `fov.to_radians()` if only the legacy field is present, defaulting to `60_f32.to_radians()` otherwise. Pre-C.4.B saved `.editor_preferences.json` files load correctly; first save after upgrade migrates the file forward to the canonical `fovy` field name. Tests added (5 new): `fovy_field_stores_radians_post_c_4_b` (anchors radian storage); `set_fov_takes_degrees_per_boundary_convention` (anchors API boundary); `fov_degrees_getter_returns_degrees` (anchors UI read path); `deserializes_legacy_fov_field_as_degrees` (backward-compat); `deserializes_canonical_fovy_field_as_radians` (canonical path); Test 5 of C.4 reframed from "FOV boundary conversion" to "RenderView::fovy emits correct value" (no functional change). orbit_camera_producer.rs 10/10 passing; picking_consistency.rs 2/2 passing (no fixture changes — uses `::new()` constructor); render_parity_harness.rs 1/1 passing (rendered output byte-identical to pre-rename — the rename doesn't change rendered pixels). **Second concrete reinforcement of structural-rename closure shape** (first was C.3.C's workspace-wide `Camera` → `FreeFly` rename); same proof shape applied to a single-field rename rather than a type rename. After C.4.B, all OrbitCamera-related §3 migration rows are closed (§2.8 -X forward in C.4; picking VP mismatch in C.4; §2.1 FOV degrees in C.4.B). Editor-side first-pass migration structurally complete.

The migration table rows below remain scheduled for their target sub-phases. Individual row status updates happen as each row's target sub-phase fully closes; partial-closure status is captured in this log.

This table enumerates every implementation found in C.0 Phase 1 that uses a non-canonical convention, with the specific axis violated and the target migration sub-phase. **This is the migration tracking list for C.3–C.7.** After C.9 closes, any implementation listed here that still exists in the codebase becomes a violation (the contract tests would then assert these specific files are convention-compliant or removed).

The C.0 audit doc inventory numbers (#1–#31) are preserved for cross-reference.

| # | Implementation | File:line | Axis | Non-canonical convention | Target migration sub-phase |
|---|---|---|---|---|---|
| #3 | ~~Editor `OrbitCamera`~~ | ~~`tools/aw_editor/src/viewport/camera.rs:50-639`~~ | ~~§2.1~~ | ~~FOV in degrees, field name `fov` (stores degrees, converts to radians at producer boundary in `to_render_view`)~~ | **CLOSED (C.4.B) — field renamed to `fovy: f32` storing radians per §2.1. Setter `set_fov(degrees)` preserves the UI boundary in degrees per C.4.B Decision 1; getters `fov_degrees()` and `fovy()` added. Serde shadow type accepts both legacy `fov` (degrees) and canonical `fovy` (radians) for backward compat with pre-C.4.B saved files.** |
| #3 | ~~Editor `OrbitCamera`~~ | ~~`tools/aw_editor/src/viewport/camera.rs:376-383`~~ | ~~§2.8~~ | ~~-X forward at yaw=0 (orbit-offset semantic: camera sits at `focal_point + spherical(yaw,pitch,distance)` and looks toward focal_point — opposite of engine `dir(yaw,pitch)`)~~ | **CLOSED (C.4) — `to_engine_camera` bridge deleted in C.3.C; OrbitCamera now produces `RenderView` directly via `CameraProducer::to_render_view`; the `view_dir` in `RenderView` is derived from `focal_point - position()` so the producer reports the actual look direction and no axis-flip conversion is needed** |
| #3 | ~~Editor `OrbitCamera::ray_from_screen` vs `unproject_depth_to_world`~~ | ~~`tools/aw_editor/src/viewport/camera.rs:511-526, 541-559`~~ | ~~(correctness, not §2)~~ | ~~`ray_from_screen` uses absolute VP, depth-based unproject uses relative VP — divergent results at large camera world positions~~ | **CLOSED (C.4) — `ray_from_screen` migrated to the precision-stable camera-relative VP + position-translation pattern. Phase 1 inspection clarified the pre-fix issue was a float-precision divergence (clip-space output is invariant across VP pipelines), not a coord-space mismatch. New `picking_consistency.rs` test verifies agreement at both modest and large camera world positions.** |
| #4 | Gizmo `CameraController` | `tools/aw_editor/src/gizmo/scene_viewport.rs:19-173` | §2.4 (partial) | `set_view_top` switches `up` field to `Vec3::NEG_Z` (line 163) — field semantics change during use | C.6 |
| #4 | Gizmo `CameraController` | `tools/aw_editor/src/gizmo/scene_viewport.rs:76-92` | (correctness, not §2) | Quaternion orbit math has unguarded gimbal-lock NaN at top/bottom (`offset.cross(up)` → zero) | C.6 |
| #4 | Gizmo `CameraController` | `tools/aw_editor/src/gizmo/scene_viewport.rs:19-173` | (duplication, not §2) | Reimplements OrbitCamera-class functionality in editor subsystem | C.6 |
| #5/#6 | ~~Renderer dual upload paths~~ | ~~`astraweave-render/src/renderer.rs:4013-4040 (update_camera) and 3959-4011 (update_camera_matrices)`~~ | ~~§2.9~~ | ~~Two parallel APIs that must keep side effects aligned (UBO, cached matrices, CSM cascades, water, sky, impostors)~~ | **CLOSED (C.3.C)** |
| #8 | `EngineRenderAdapter::update_water` | `tools/aw_editor/src/viewport/engine_adapter.rs:3759-3764` | §2.9 + §2.8 | Uses `to_engine_camera()` dual-conversion path (the bypass that editor's `update_camera` deliberately avoids) | **C.3** (eliminate dual path) |
| #10 | `CameraKey` | `astraweave-cinematics/src/lib.rs:236-307` | §2.1 | Field `fov_deg` stores degrees (not radians, not `fovy`) | C.7 (cinematic evaluator upgrade) |
| #10 | `apply_camera_key` | `astraweave-render/src/renderer.rs:3371-3381` | §2.9 | Converts CameraKey to engine `Camera` per-event (not continuous `RenderView` evaluator) | C.7 |
| #10 | `apply_camera_key` | `astraweave-render/src/renderer.rs:3371-3381` | (correctness, not §2) | `normalize_or_zero` silently accepts degenerate look_at==pos; `is_typical_fov()` validation never called on apply | C.7 |
| #23 | `unified_showcase` bespoke camera | `examples/unified_showcase/src/main.rs:148-150` | §2.1 | No `fovy` field; hardcoded `45.0_f32.to_radians()` at projection site (line 2168) | C.6 |
| #23 | `unified_showcase` bespoke camera | `examples/unified_showcase/src/main.rs:2162-2164` | §2.8 | -Z forward at yaw=0 (uses `Vec3::NEG_Z` rotated by yaw/pitch quats — Bevy/glTF convention) | C.6 |
| #23 | `unified_showcase` bespoke camera | (general) | §2.9 | Operates entirely outside `astraweave-render::Renderer` — bespoke wgpu pipeline with own `CameraUniforms` struct | C.6 (decision: migrate to engine OR formalize as separate sandbox) |
| #24 | `shadow_csm_demo::Camera` | `examples/shadow_csm_demo/src/main.rs:74-115` | §2.1 | Field `fov` (not `fovy`); stores radians but uses non-canonical name | C.6 |
| #24 | `shadow_csm_demo::Camera` | (input layer) | (correctness, not §2) | Likely missing pitch clamping (only inspected fields; input handler not audited in C.0) | C.6 |
| #25 | `fluids_demo::Camera` | `examples/fluids_demo/src/main.rs:26-46` | §2.1 | Field `fovy` stores **degrees**, converted to radians at projection time inside `build_view_projection_matrix` (line 39) — name matches §2.1 but unit doesn't | C.6 |
| #25 | `fluids_demo::Camera` | `examples/fluids_demo/src/main.rs:460` | §2.3 | Aspect resize without `.max(1)` or `height > 0` guard | C.6 |
| #26 | `nanite_demo` bare fields | `examples/nanite_demo/src/main.rs:23-30` | §2.9 | No Camera type; bare `camera_pos`, `camera_yaw`, `camera_pitch` fields on `DemoState` | C.6 |
| #26 | `nanite_demo` bare fields | (input layer) | (correctness, not §2) | Likely missing pitch clamping | C.6 |
| **#29** | **Bench mock `Camera`** | **`astraweave-render/benches/camera_primitives_instancing.rs:35`** | **§2.4** | **`-Vec3::Y` up vector — stale copy of pre-`df7649287` engine code (BUG)** | **C.1 Deliverable C (THIS SUB-PHASE)** |

**Convention violations vs migration debt**: rows targeting C.3–C.7 are documented divergences scheduled for migration, not bugs *yet*. After their target sub-phase closes, they become violations. The row targeting **C.1 (this sub-phase)** is the bench mock — fixed in Deliverable C.

The C.0 audit also identified 7 GPU receivers (`nanite_render::update_camera`, `impostor_pass::update_camera`, etc., audit inventory #11–#19) and the GPU uniform layouts (#20–#21) that take pre-computed matrices. These are not independent camera implementations; they consume `view_proj: Mat4, camera_pos: Vec3` produced by an upstream camera. Their migration is downstream of C.3's renderer upload consolidation — they continue to accept matrices, but the matrices come from `RenderView` instead of dual `update_camera`/`update_camera_matrices` paths.

## §4 — Contract tests reference

The contract tests live at `astraweave-render/tests/camera_conventions.rs`. They assert that engine `Camera` (the production runtime camera in `astraweave-render/src/camera.rs`) complies with every non-deferred convention in §2. **Failure of any test in `camera_conventions.rs` means a convention violation** — either the code drifted, or the convention needs revision (a Andrew-design call, not an autonomous test relaxation).

Test coverage in C.1:

| Test | Asserts | Convention |
|---|---|---|
| `fovy_stores_radians` | `Camera::fovy = 60_deg.to_radians()` produces same projection as `Mat4::perspective_rh(60_deg.to_radians(), ...)` | §2.1 |
| `near_far_use_wgpu_zero_to_one_depth` | view-space `z = -near` maps to NDC `z = 0`; `z = -far` maps to NDC `z = 1` | §2.2, §2.6 |
| `aspect_floored_at_projection` | `aspect = 0.0` does not produce NaN projection matrix (clamped via `.max(0.01)`) | §2.3 |
| `up_vector_is_positive_y` | view matrix matches manual `Mat4::look_to_rh(pos, dir, Vec3::Y)` construction | §2.4 |
| `look_to_and_look_at_styles_equivalent` | `look_to_rh(eye, dir, up) == look_at_rh(eye, eye + dir, up)` for matching inputs | §2.5 |
| `yaw_zero_pitch_zero_forward_is_positive_x` | `Camera::dir(0.0, 0.0) == Vec3::X` | §2.8 |
| **`negative_y_up_produces_different_view`** | **discriminator: `-Vec3::Y` up vector produces a structurally different view matrix than `Vec3::Y` up** — proves the convention test discriminates correctly | §2.4 (negative test) |
| **`bench_mock_camera_uses_canonical_up_vector`** | **`include_str!` of `benches/camera_primitives_instancing.rs` confirms the `view_matrix` function uses `Vec3::Y` (not `-Vec3::Y`)** — references and verifies C.1 Deliverable C fix | §2.4 (bench-mock fix verification) |

Test commands:

```sh
cargo test --tests -p astraweave-render camera_conventions
```

Run after any modification to `astraweave-render/src/camera.rs`, `astraweave-render/benches/camera_primitives_instancing.rs`, or any new addition to the renderer's projection/view paths.

## §5 — Forward chain

The Unified Camera campaign (C.1 through C.9) progressively migrates non-canonical implementations to comply with this doc. The campaign roadmap (subject to refinement at each sub-phase's planning round):

- **C.1 (this sub-phase)** — Conventions lockdown + contract tests + bench mock fix. **Closure proof: contract-test closure** (`cargo test camera_conventions` passes; CAMERA_CONVENTIONS.md exists with §1–§5; bench mock up vector is `Vec3::Y`).
- **C.2** — `astraweave-camera` crate exists with canonical `CameraState`, `Projection`, `RenderView` types; no consumers yet; convention tests apply to new types.
- **C.3** — Engine `Camera` + `CameraController` migrate to `astraweave-camera` types internally; renderer upload path unified to `RenderView` only (eliminates §3 row "Renderer dual upload paths"); `update_camera`/`update_camera_matrices` duplication deleted; `to_engine_camera` dual path eliminated including the `update_water` case (eliminates §3 row "`EngineRenderAdapter::update_water`").
- **C.4** — Editor `OrbitCamera` adopts `astraweave-camera` types (eliminates §3 rows "Editor `OrbitCamera` §2.1 FOV degrees" and "ray_from_screen vs unproject_depth_to_world VP mismatch"); OrbitCamera UI/picking surface stays in editor but produces canonical `RenderView`.
- **C.5** — Audit Pass over gizmo `CameraController` + per-example cameras (`unified_showcase`, `shadow_csm_demo`, `fluids_demo`, `nanite_demo`); produces a queue of migration targets with per-target decisions.
- **C.6** — Execute C.5's migration queue (may split into C.6.A–C.6.X if large). Eliminates §3 rows for #4, #23, #24, #25, #26.
- **C.7** — Cinematics `CameraKey` evolved into evaluator producing canonical `RenderView`; eliminates `apply_camera_key` round-trip. Eliminates §3 rows for #10.
- **C.8** — Camera Parity harness expansion (extreme pitch fixtures, non-square aspect, large world positions, cinematic-driven fixtures) — exercises blind spots C.0 §5.3 identified.
- **C.9** — Campaign closeout: outcome doc analog to Editor-Engine Render Parity's; methodology pillar reinforcement notes; post-campaign cleanup queue.

After C.9 closes, Terrain Asset Quality campaign resumes at A.5 (doc reconciliation, now incorporating both Editor-Engine Render Parity outcome and Unified Camera outcome as architectural inputs), then A.6+ per-biome wire-ups proceed against the doubly-verified foundation.

**§7.11 candidate Pillars** (informational; codification deferred to C.9 closeout or E-closeout): Pillar 6 frame-currency, Pillar 7 architectural-priority validation (reinforced reactively by Editor-Engine Render Parity launch and proactively by C.0 audit), Pillar 5-refinement (now reinforced six times: P.2 byte / P.3 pipeline / P.4 parameter / P.5 format / P.6 isolation / **C.1 contract-test**). This doc and the contract tests are the C.1 contract-test closure proof — the sixth concrete instance of measurement-instrument-matched-to-seam-type.

---

**Revision history**:

- v1.0 (2026-05-18) — initial publication. C.1 closure. Establishes conventions; migration tracking table enumerates 21 non-canonical rows across 11 implementations.
- v1.1 (2026-05-24) — C.4 status log entry appended. Migration table rows updated: "Editor `OrbitCamera` -X forward (bridged)" marked CLOSED (C.4); "Editor `OrbitCamera::ray_from_screen` vs `unproject_depth_to_world` VP mismatch" marked CLOSED (C.4); "Editor `OrbitCamera` §2.1 FOV degrees" reframed as "boundary conversion applied; structural field rename deferred to C.4.B".
- v1.2 (2026-05-24) — C.4.B status log entry appended. Migration table row "Editor `OrbitCamera` §2.1 FOV degrees" marked **CLOSED (C.4.B)** — structural field rename `fov: degrees` → `fovy: radians` landed; serde backward-compat preserves pre-C.4.B saved files. After C.4.B, all OrbitCamera-related §3 migration rows are closed; the editor-side first-pass migration is structurally complete.
- v1.3 (2026-05-24) — C.5 status log entry appended. §0 metadata table updated with parallel reference to `docs/audits/camera_system_phase_2_audit_2026-05.md` (Phase 2 audit) alongside the C.0 audit. C.5 is the second audit-shaped sub-phase in the campaign; produces draft C.6 migration queue and cinematics inventory for C.7 planning. Three of C.0's medium-confidence pitch-clamp findings falsified by empirical re-inspection.
