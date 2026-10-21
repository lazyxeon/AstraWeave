# Camera System Architecture Audit (Sub-phase C.0)

| Field | Value |
|-------|-------|
| Date | 2026-05-18 |
| Scope | Read-only static source analysis. No source files modified. |
| Sub-phase | C.0 (Camera System Architecture Audit) |
| Forward chain (Andrew gate) | **Unified Camera crate (largest)** — see §6 |
| Doc artifact authorization | Andrew approved during Phase 6 gate |
| Reconciles against | `docs/current/CAMERA_SYSTEMS_SOTA_AUDIT_AND_RECOMMENDATIONS.md` (2026-04-28, SOTA audit) |

## 0. Why this audit exists

The Terrain Asset Quality campaign is paused at A.4 closeout (deferred A.5 doc reconciliation + A.6+ per-biome wire-ups). The Editor-Engine Render Parity campaign just closed at P.7 (commit `6dc95ae9b`) with all five parity seams structurally protected.

Before resuming Terrain Asset Quality A.5, Andrew surfaced a Pillar 7-class concern (architectural-priority validation): **the camera system has known multi-implementation accretion across the codebase, and it is unclear whether what is rendered in the editor viewport is a faithful representation of the intended scene or a downstream artifact of camera correctness issues.**

The Editor-Engine Render Parity campaign demonstrated Pillar 7 reactively (A.4 discovered parity divergence, campaign fixed it). C.0 applies Pillar 7 proactively (audit *before* resuming the feature campaign for architectural-priority concerns that would invalidate or undermine the feature work). C.0 documents this two-directional shape for future E-closeout codification, but does not codify the pillar itself.

C.0 distinguishes three potential problem classes:
1. **Cross-camera divergence** — different camera implementations disagree on conventions.
2. **Per-camera correctness issues** — internal bugs even if all cameras agreed with each other.
3. **Specialization vs accretion** — multi-implementation by design vs accidental.

This document is the read-only deliverable. Source remediation (if any) happens in subsequent sub-phases per Andrew's forward-chain choice.

## 1. Phase 1 — Camera Implementation Inventory

### 1.A Canonical engine 3D cameras (Rust types)

| # | Implementation | File:line | Shape | Role |
|---|---|---|---|---|
| 1 | `astraweave_render::Camera` | `astraweave-render/src/camera.rs:3-49` | yaw/pitch/fovy/aspect/znear/zfar | Production engine camera. `view_matrix()` = `Mat4::look_to_rh(pos, dir, +Y)`. `proj_matrix()` = `Mat4::perspective_rh(fovy, aspect, znear, zfar)`. Has `view_matrix_camera_relative()` for f32-jitter mitigation. |
| 2 | `astraweave_render::CameraController` | `astraweave-render/src/camera.rs:58-298` | dual-mode (FreeFly + Orbit) | Free-fly mode is the production runtime controller; Orbit mode is a *secondary* mode in the same controller (separate from editor `OrbitCamera`). |
| 3 | `aw_editor_lib::viewport::OrbitCamera` | `tools/aw_editor/src/viewport/camera.rs:50-639` | focal_point/distance/yaw/pitch/fov(deg)/aspect/near/far | Editor production camera. `view_matrix()` = `Mat4::look_at_rh(position(), focal_point, +Y)`. `projection_matrix()` = `Mat4::perspective_rh(fov.to_radians(), aspect, near, far)`. Has `view_matrix_relative()` and `view_projection_matrix_relative()`. Has `to_engine_camera()` conversion to type #1. |
| 4 | `aw_editor::gizmo::scene_viewport::CameraController` | `tools/aw_editor/src/gizmo/scene_viewport.rs:19-173` | position/target/up/fov(rad)/aspect/near/far + orbit/pan/zoom methods | **Distinct from #2** despite identical name. Gizmo-subsystem camera with its own orbit math (quat-based, axis-angle composition). |

### 1.B Renderer camera input paths (production callers feed these)

| # | Function | File:line | Inputs | Effect |
|---|---|---|---|---|
| 5 | `Renderer::update_camera(&Camera)` | `astraweave-render/src/renderer.rs:4013-4040` | type #1 | Computes view/proj internally. Used by every example that uses engine Camera (`hello_companion`, `veilweaver_demo`, `cutscene_render_demo`, `weaving_playground`, `physics_demo3d`, `npc_town_demo`, `navmesh_demo`, `visual_3d`, `debug_toolkit_demo`, `ui_controls_demo`, `audio_spatial_demo`, `biome_showcase`, `renderer_integration_test`). |
| 6 | `Renderer::update_camera_matrices(view, proj, position, znear, zfar, fovy, aspect)` | `astraweave-render/src/renderer.rs:3959-4011` | precomputed matrices | Parallel API to #5. Used by editor and parity harness. Internally reconstructs a temporary `Camera` (line 4001) by extracting yaw/pitch from view matrix to feed `update_cascade_splits`. |
| 7 | `EngineRenderAdapter::update_camera(&OrbitCamera)` | `tools/aw_editor/src/viewport/engine_adapter.rs:766-835` | type #3 | Editor's bridge. Calls `Renderer::update_camera_matrices` directly with OrbitCamera matrices, deliberately **bypassing** `to_engine_camera()` conversion. Comment at line 768-769: "This avoids yaw/pitch conversion issues between the orbit camera and the engine camera's direction conventions." |
| 8 | `EngineRenderAdapter::update_water(&OrbitCamera, time)` | `tools/aw_editor/src/viewport/engine_adapter.rs:3759-3764` | type #3 | **Dual conversion path** — uses `camera.to_engine_camera().vp()` *and* `camera.position()`. Per `docs/audits/water_system_architecture_2026-04-20.md:65`, no caller exists in `tools/aw_editor`. Reachable code; latent. |
| 9 | `Renderer::apply_camera_key(cam, k)` | `astraweave-render/src/renderer.rs:3371-3381` | `CameraKey { pos, look_at, fov_deg }` | Converts cinematics keyframe → type #1 fields by deriving `yaw = dir.z.atan2(dir.x)`, `pitch = dir.y.asin()`, `fovy = fov_deg.to_radians()`. Called by `Renderer::tick_cinematics` (line 3407). |

### 1.C Cinematics camera data

| # | Implementation | File:line | Shape | Role |
|---|---|---|---|---|
| 10 | `astraweave_cinematics::CameraKey` | `astraweave-cinematics/src/lib.rs:236-307` | `t: Time, pos: (f32,f32,f32), look_at: (f32,f32,f32), fov_deg: f32` | Timeline keyframe data with linear `lerp()`. Tuple-based (no glam::Vec3 in the type). Only production consumer is #9. |

### 1.D GPU uniform receivers (not cameras — accept pre-computed view_proj)

| # | Function | File:line | Inputs |
|---|---|---|---|
| 11 | `nanite_render::update_camera(queue, view_proj, position)` | `astraweave-render/src/nanite_render.rs:184-195` | Mat4 + Vec3 |
| 12 | `impostor_pass::update_camera(queue, view_proj, camera_pos)` | `astraweave-render/src/impostor_pass.rs:205-211` | Mat4 + Vec3 |
| 13 | `particle_render::update_camera(...)` | `astraweave-render/src/particle_render.rs:298` | (similar) |
| 14 | `terrain_material_manager::update_camera(...)` | `astraweave-render/src/terrain_material_manager.rs:650` | (similar) |
| 15 | `terrain_splat::update_camera(queue, view_proj, view, camera_pos, camera_forward, camera_right)` | `tools/aw_editor/src/viewport/terrain_splat.rs:259-290` | **superset signature** with both view matrices AND derived basis vectors — different shape from #11–14 |
| 16 | `clipmap_terrain::update_camera_relative(...)` | `astraweave-render/src/clipmap_terrain.rs:502` | camera-relative position |
| 17 | `terrain_modifier::update_camera(camera_pos)` | `astraweave-terrain/src/terrain_modifier.rs:241` | Vec3 only |
| 18 | `streaming_diagnostics::update_camera(camera_pos)` | `astraweave-terrain/src/streaming_diagnostics.rs:244` | Vec3 only |
| 19 | `background_loader::update_camera(position, direction)` | `astraweave-terrain/src/background_loader.rs:403` | Vec3 + Vec3 (direction!) |
| 20 | `GpuCamera` uniform layout | `astraweave-render/src/nanite_gpu_culling.rs:33-48` | view_proj + inv_view_proj + position + view_dir + frustum_planes + ... | Constructed by `from_matrix(view_proj, position, w, h)` — not a camera type. |
| 21 | `astraweave_fluids::renderer::CameraUniform` | `astraweave-fluids/src/renderer.rs:8` | view_proj + inv_view_proj + view_inv + cam_pos + light_dir + time | Fluid-specific 320-byte uniform. Filled externally. |

### 1.E Parity harness camera

| # | Implementation | File:line | Notes |
|---|---|---|---|
| 22 | `ParityFixture::camera()` | `tools/aw_editor/tests/render_parity_harness.rs:348-359` | Returns `OrbitCamera::new(Vec3::ZERO, 25.0, π/4, π/6)` + `set_aspect(width, height)`. Fixed orbit at 30° pitch, 45° yaw, 25 m distance. Engine and editor paths both consume this fixture identically. |

### 1.F Per-example ad-hoc cameras

| # | Implementation | File:line | Shape |
|---|---|---|---|
| 23 | `examples/unified_showcase/src/main.rs::ShowcaseApp` bespoke camera | `examples/unified_showcase/src/main.rs:148-150, 2158-2173` | bare `camera_pos: Vec3`, `camera_yaw: f32`, `camera_pitch: f32` fields. View computed inline via `Mat4::look_at_rh(pos, pos + Quat::y(yaw)*Quat::x(pitch)*NEG_Z, +Y)`. **Uses +Z forward convention** (NEG_Z target offset) — different from engine Camera's +X forward at yaw=0. FOV hardcoded `45.0_f32.to_radians()`. Backup files (`main_temp.rs`, `main_backup*.rs`, `main_bevy*.rs`, `main_clean.rs`) are unused historical drift in the src/ tree but not in `[[bin]]` entry. |
| 24 | `examples/shadow_csm_demo/src/main.rs::Camera` | `examples/shadow_csm_demo/src/main.rs:74-115` | Own struct: `position, yaw, pitch, fov, near, far`. `view_matrix()` = `look_at_rh(pos, pos+forward, +Y)`. Aspect passed externally to `proj_matrix(aspect)`. **Uses +X forward** (yaw=0 ⇒ forward = (cos·cos, sin, sin·cos)) — agrees with engine convention. |
| 25 | `examples/fluids_demo/src/main.rs::Camera` | `examples/fluids_demo/src/main.rs:26-46` | Own struct: `eye, target, up, aspect, fovy (degrees, then `.to_radians()` inside build_view_projection_matrix), znear, zfar`. Look-at style. |
| 26 | `examples/nanite_demo/src/main.rs::DemoState` bare fields | `examples/nanite_demo/src/main.rs:23-52` | `camera_pos, camera_yaw, camera_pitch` bare. View = `look_at_rh(pos, pos+forward, +Y)`. Same +X-forward convention as #1 and #24. No struct. |

### 1.G Non-rendering / 2D / gameplay-only

| # | Implementation | File:line | Notes |
|---|---|---|---|
| 27 | `astraweave_weaving::level::Camera` | `astraweave-weaving/src/level.rs:163-192` | 3rd-person follow camera: position/target/offset/smoothing. **No matrix output methods** — it's a gameplay state container, not a rendering camera. No production consumer renders via it. |
| 28 | `aw_editor::panels::blueprint_panel::CanvasCamera` | `tools/aw_editor/src/panels/blueprint_panel.rs:242-270` | **2D** screen-to-world transform for the blueprint zone-editor (`center: GVec2, zoom: f32`). Not a 3D camera. Out of audit scope. |

### 1.H Test/benchmark mock cameras (production-adjacent)

| # | Implementation | File:line | Notes |
|---|---|---|---|
| 29 | Bench mock `Camera` | `astraweave-render/benches/camera_primitives_instancing.rs:22-67` | "Matching actual API without wgpu dependencies." **Up vector is `-Vec3::Y` (line 35)** — explicitly the bug the engine fix `df7649287` eliminated. Stale copy of an older `camera.rs` revision. Bench-only; does not ship. |
| 30 | Bench mock `Camera` | `astraweave-render/benches/nanite_shadow_csm.rs:22` | Separate `GpuCamera` for bench. Out of scope for runtime fidelity. |
| 31 | `wave2_camera_remediation.rs` test fixture | `astraweave-render/tests/wave2_camera_remediation.rs:14-23` | Validation tests for engine Camera::dir(). Confirms convention (yaw=0,pitch=0 → +X). |

### 1.I Excluded (clearly out-of-scope)

- WGSL `struct Camera { ... }` strings inside Rust source (`astraweave-render/src/renderer.rs:48, 477`, `impostor_lod3.rs:226`, `skinning_gpu.rs:346`) — these are shader interface declarations, not Rust types.
- Dead alternative implementations: `examples/unified_showcase/src/main_temp.rs`, `main_backup*.rs`, `main_bevy*.rs`, `main_clean.rs` — present in src/ but not in `[[bin]]` paths.
- Archived: `docs/journey/archive/astraweave-render-bevy/src/extensions/nanite.rs:242`.

### 1.J Production camera count

**8 active 3D camera codepaths:**
1. Engine `Camera` (production runtime).
2. Engine `CameraController` (runtime + secondary orbit mode).
3. Editor `OrbitCamera` (production editor).
4. Gizmo `CameraController` (distinct from #2; production editor subsystem).
5. `unified_showcase` bespoke (production flagship example).
6. `shadow_csm_demo` own (production graphics example).
7. `fluids_demo` own (production graphics example).
8. `nanite_demo` bare-fields (production graphics example, feature-gated).

Plus `CameraKey` cinematics keyframes consumed by #1 via `apply_camera_key`. Plus 11 GPU receivers (not independent cameras; downstream consumers).

## 2. Phase 2 — Convention Divergence Across 9 Mandatory Axes

Implementations compared: **I**=engine `Camera` (#1), **E**=editor `OrbitCamera` (#3), **G**=gizmo `CameraController` (#4), **U**=unified_showcase (#23), **S**=shadow_csm_demo (#24), **F**=fluids_demo (#25), **N**=nanite_demo (#26), **C**=CameraKey (#10).

### Axis 1: FOV semantics

| Impl | Convention | Source |
|---|---|---|
| I | **vertical FOV in radians** (field name `fovy`) | `camera.rs:7`, used via `Mat4::perspective_rh(fovy, ...)` line 24 |
| E | **vertical FOV in degrees** (field `fov`), converted to radians at projection time via `fov.to_radians()` | `camera.rs:64, 458` |
| G | **vertical FOV in radians** (field `fov`) | `scene_viewport.rs:27, 58` |
| U | Hardcoded **`45.0_f32.to_radians()`** at projection site | `main.rs:2168-2169` |
| S | Stored as radians (`60.0_f32.to_radians()` at construction) | `main.rs:89` |
| F | **Field `fovy` in DEGREES**, converted at projection time via `to_radians()` | `main.rs:39` |
| N | No FOV state — uses unknown projection caller | `main.rs:23-30` |
| C | **`fov_deg`** stored as degrees, converted to radians at `apply_camera_key` line 3380 via `to_radians()` | `lib.rs:241` |

**Divergence**: **YES** — both unit (radians vs degrees) AND naming (`fovy` vs `fov` vs `fov_deg`) vary across implementations. Conversions exist but are at the boundary — anyone reading a `fov` field has to look up the type to know whether it's already radians or needs `.to_radians()`. Field-name discipline is inconsistent.

### Axis 2: Near/far plane handling

| Impl | Near default | Far default | Reversed-Z? |
|---|---|---|---|
| I | caller-supplied (typical `0.1`) | caller (typical `100`–`500`) | **Standard wgpu [0,1] forward-Z** (not reversed). Confirmed by `shadow_csm.rs` cascade extraction |
| E | `0.5` | `5000.0` | wgpu [0,1] (confirmed by `extract_near_plane` at `camera.rs:718-729` — near plane = row2, not row3+row2) |
| G | `0.1` | `1000.0` | wgpu [0,1] (uses standard `perspective_rh`) |
| U | `0.1` | `2000.0` | wgpu [0,1] |
| S | `0.1` | `100.0` | wgpu [0,1] |
| F | caller (`0.1` typical) | caller (`100` typical) | wgpu [0,1] |
| N | not visible in inspected lines | — | — |

**Divergence**: **NO** on handedness/depth convention (all wgpu [0,1]). **YES on near plane value**: editor `OrbitCamera` defaults to `near = 0.5` while engine examples typically use `near = 0.1` or `0.01`. The parity harness explicitly overrides engine path's near to `0.5` (render_parity_harness.rs:497) to match editor — but examples consuming engine Camera directly use `0.1` or `0.01`. Far plane defaults span 100 → 5000 across implementations. The `OrbitCamera` allows zoom range up to **20000 m** (`camera.rs:112`) — far=5000 may clip the orbit geometry at extreme zoom-out.

### Axis 3: Aspect ratio handling

| Impl | Storage | Update behavior | NaN guard |
|---|---|---|---|
| I | field `aspect` | caller responsible to update on resize; usage at line 24 calls `self.aspect.max(0.01)` | `.max(0.01)` floor |
| E | field `aspect`, default `16.0/9.0` | `set_aspect(width, height)` method `camera.rs:363-367`; only assigns if `height > 0.0` | `sanitize()` clamps NaN at line 611-613 |
| G | field `aspect`, default `16.0/9.0` | `set_aspect_ratio(width, height)` in `scene_viewport.rs:245` | none |
| U | caller computes `width/height.max(1)` at projection site | inline at line 2170 | `.max(1)` integer |
| S | computed at projection site `self.size.width as f32 / self.size.height as f32` | line 598 | **none** (divide-by-zero risk if height=0) |
| F | field `aspect`, updated on resize without `.max(1)` | line 460 | **none** |
| N | unclear — not in inspected lines | — | — |

**Divergence**: **YES** — engine has `.max(0.01)` guard, editor has `sanitize()` and `height > 0.0` check, examples are mixed (some `.max(1)`, some none). Several examples (`fluids_demo:460`, `shadow_csm_demo:598`, `hello_companion:870`, `renderer_integration_test:54`) lack divide-by-zero protection.

### Axis 4: Coordinate handedness

| Impl | Convention |
|---|---|
| I | **Right-handed**, +Y up (`look_to_rh(pos, dir, +Y)`) |
| E | Right-handed, +Y up (`look_at_rh(pos, focal, +Y)`) — explicit in docstring `camera.rs:447-449` |
| G | Right-handed, configurable up (defaults +Y, but `set_view_top` switches to `-Z` up at `scene_viewport.rs:163`) |
| U | Right-handed, +Y up |
| S, F, N | Right-handed, +Y up |
| Bench mock (#29) | **Right-handed but `-Y` (NEGATIVE) up** at `camera_primitives_instancing.rs:35` |

**Divergence**: All production cameras agree (RH, +Y up). The benchmark file #29 disagrees — flagged in §3 as a stale copy of pre-fix engine code.

### Axis 5: View matrix construction

| Impl | Style |
|---|---|
| I | `look_to_rh(eye, dir, up)` — direction-based |
| E | `look_at_rh(eye, target, up)` — target-based |
| G | `look_at_rh(position, target, up)` — target-based |
| U | `look_at_rh(eye, eye + rotation*NEG_Z, +Y)` — quat-rotated NEG_Z target |
| S, N | `look_at_rh(pos, pos + forward, +Y)` — pos+forward target |
| F | `look_at_rh(eye, target, up)` — explicit target |

**Divergence**: Two distinct styles (direction-based vs target-based). The editor `OrbitCamera::view_matrix_relative()` and engine `Camera::view_matrix_camera_relative()` both produce origin-eye rotation-only views, but via different paths — editor builds `Mat4::look_at_rh(ZERO, -eye_offset, +Y)` (`camera.rs:480`), engine builds `Mat4::look_to_rh(ZERO, dir, +Y)` (`camera.rs:47`). Result is equivalent when conventions agree.

### Axis 6: Projection matrix construction

All Rust implementations use **`Mat4::perspective_rh`** (glam's wgpu-compatible variant producing [0,1] depth output). No mixing of `perspective_rh` and `perspective_rh_gl` ([-1,1] depth) found in production. Editor's `Frustum::extract_near_plane` (`camera.rs:718-729`) explicitly extracts row 2 directly (not row3-row2) which is correct for wgpu [0,1] near. **Divergence: NO** for production-path projection construction.

### Axis 7: Jitter (TAA) handling

Searched workspace for jitter-applied projection in cameras: **no production camera applies jitter**. TAA is not active in production paths reviewed. **Divergence: N/A** — no implementation has this axis. If TAA is added later, each implementation will need updating; currently a uniform absence.

### Axis 8: Camera parameter semantics — yaw=0 forward direction

| Impl | Convention at yaw=0, pitch=0 |
|---|---|
| I | `dir(0,0) = (1, 0, 0)` = **+X forward** (`camera.rs:31-38`) |
| S | `forward = (1, 0, 0)` = **+X forward** (matches engine) |
| N | `forward = (1, 0, 0)` = **+X forward** (matches engine) |
| U | `rotation * NEG_Z = (0, 0, -1)` = **-Z forward** when yaw=0,pitch=0 (`main.rs:2162-2164`) |
| E | yaw/pitch parameterize orbit *offset*, not look direction. At yaw=0,pitch=0, `eye_offset = (distance, 0, 0)`, so camera looks **from +X toward origin** i.e. forward direction is **-X** (`camera.rs:376-383, 451`). |
| G | `look_at_rh(position, target, up)` — yaw/pitch not parameters; orbit is quat-composed. No direct "yaw=0" semantics. |
| F | `look_at_rh(eye, target, up)` — yaw/pitch not parameters of Camera struct (they're separate State fields). |
| C | "look_at" target stored explicitly; no yaw/pitch. |

**Divergence**: **YES — major.** Three distinct yaw conventions across cameras that use yaw/pitch parameters:
- **+X forward** at yaw=0: engine, shadow_csm_demo, nanite_demo
- **-Z forward** at yaw=0: unified_showcase
- **-X forward at yaw=0** (orbit toward origin from +X): editor OrbitCamera

This is the **root reason** `engine_adapter.rs:768-769` comments "This avoids yaw/pitch conversion issues" — the editor's `update_camera` deliberately bypasses `to_engine_camera()` precisely because the two conventions are 180° rotated. `OrbitCamera::to_engine_camera()` patches the mismatch via `yaw + PI, -pitch` (`camera.rs:631-632`). Whenever a system goes through `to_engine_camera` and a system goes through direct matrices, they will agree only if `to_engine_camera`'s rotation is correct.

### Axis 9: Interface contracts (what each camera promises the renderer)

| Impl | Exposes |
|---|---|
| I (engine Camera) | `view_matrix()`, `proj_matrix()`, `vp()`, position field, view_matrix_camera_relative(). `Renderer::update_camera(&Camera)` consumes this. |
| E (OrbitCamera) | `view_matrix()`, `projection_matrix()`, `view_projection_matrix()`, `view_matrix_relative()`, `view_projection_matrix_relative()`, `position()`, `target()`, `forward()`, `right()`, `up()`, `inverse_view_projection_matrix()`, `ray_from_screen()`, `extract_frustum()`, `unproject_depth_to_world()`. Plus `to_engine_camera()`. Plus all setters. Plus `sanitize()`. **Largest surface.** |
| G (gizmo CameraController) | `view_matrix()`, `projection_matrix()`, `view_projection_matrix()`, `inverse_view_projection_matrix()`, `distance()`, set_view_*(). Smaller surface than E. |
| U | No type — fields directly on `ShowcaseApp` struct. `update()` builds matrices inline. |
| S, F, N | Per-example types, varying surface. |
| C (CameraKey) | `lerp(other, t)`, `position()`, `fov_rad()`, `distance_to_target()`, `is_typical_fov()`. **No matrix output method** — must go through `apply_camera_key` → engine Camera → matrices. |

**Divergence**: **YES** — surfaces vary wildly. Editor has by far the most consumer methods; cinematics has the fewest (no direct matrices); examples are ad-hoc. There's no shared trait or interface that all cameras implement.

### 2.10 Phase 2 summary

| Axis | Divergence severity |
|---|---|
| 1 FOV unit/name | **Medium** (radians/degrees mix; field-name discipline absent) |
| 2 Near/far | **Medium** (editor near=0.5 vs examples 0.1; far range 100–5000) |
| 3 Aspect | **Low–Medium** (engine `.max(0.01)`, editor sanitize, examples mixed; some lack div-zero protection) |
| 4 Handedness | **None** (production); **Critical in benchmark** (#29 stale `-Y` up) |
| 5 View construction style | **Low** (`look_to` vs `look_at`, math-equivalent) |
| 6 Projection | **None** |
| 7 Jitter/TAA | **N/A** (uniform absence) |
| 8 Yaw=0 forward direction | **High** (+X vs -Z vs -X — three conventions) |
| 9 Interface surface | **High** (no shared trait; surface varies 1× to 25× method count) |

## 3. Phase 3 — Per-Implementation Correctness Audit

### 3.1 Engine `Camera` (#1)

- **Projection**: correct, `Mat4::perspective_rh(fovy, aspect.max(0.01), znear, zfar)` matches wgpu [0,1] depth convention.
- **View**: `look_to_rh(pos, dir(yaw, pitch), +Y)`. Comment at lines 17-20 documents a **prior bug**: previously used `-Vec3::Y` up, caused "clip-space w to go negative for all visible geometry … producing chunk-aligned rectangular voids in terrain." Fixed in commit `df7649287`. The comment is preserved as a tombstone.
- **`dir()` math**: `Vec3::new(cy*cp, sp, sy*cp).normalize()` — standard spherical→cartesian; pole singularities (`pitch = ±π/2`) handled by clamping. `wave2_camera_remediation.rs` proves all four quadrants.
- **Parameter clamping**: `fovy` not clamped at projection (zero or negative would degenerate). `aspect` clamped to `.max(0.01)`. `znear`/`zfar` not validated against `znear >= zfar`.
- **Latent footgun** (acknowledged in `docs/audits/tonemap_double_application_investigation_2026-04-19.md:221`): "`Camera::view_matrix` is gimbal-unstable at `pitch = ±π/2`. The initial harness attempted to render a straight-down camera and produced a degenerate view matrix." Production avoids this because `CameraController::process_mouse_*` clamps to ±1.54 rad (~88.2°).
- **High-confidence bugs**: 0. **Medium-confidence**: 1 (missing fovy/znear-vs-zfar validation). **Low-confidence**: 1 (gimbal at pitch=±π/2 — latent, controller clamps prevent in production).

### 3.2 Editor `OrbitCamera` (#3)

- **Projection**: `Mat4::perspective_rh(fov.to_radians(), aspect, near, far)`. wgpu-correct.
- **View**: `look_at_rh(position(), focal_point, +Y)`. Position derived from spherical `camera.rs:376-383` — same formula as engine `dir()` but interpreted as orbit offset. At yaw=0, pitch=0, eye sits at `focal_point + (distance, 0, 0)` and looks toward focal_point — i.e., camera looks down -X. This is the convention divergence in axis 8.
- **Orbit math**: spherical→cartesian as standard. `min_pitch = -π/2 + 0.01`, `max_pitch = π/2 - 0.01` (line 113-114) — guard against gimbal singularity. Tests `test_orbit_pitch_clamp` confirm clamping fires.
- **`to_engine_camera()` math** (line 624-638): adds `π` to yaw and negates pitch. Verification by hand:
  - OrbitCamera direction from camera toward focal_point: `-(cos(yaw)*cos(pitch), sin(pitch), sin(yaw)*cos(pitch))` = `(-cos(yaw)*cos(pitch), -sin(pitch), -sin(yaw)*cos(pitch))`.
  - Engine `dir(yaw+π, -pitch)` = `(cos(yaw+π)*cos(-pitch), sin(-pitch), sin(yaw+π)*cos(-pitch))` = `(-cos(yaw)*cos(pitch), -sin(pitch), -sin(yaw)*cos(pitch))`. **Equal.** The conversion is mathematically correct.
- **`view_matrix_relative()` math** (line 473-481): `Mat4::look_at_rh(ZERO, -eye_offset, +Y)`. `eye_offset` is the orbit offset (focal_point to camera). Looking from origin toward `-eye_offset` = looking from origin toward where focal_point would be if camera was at origin. Correct.
- **`unproject_depth_to_world` math** (line 541-559): uses pixel-center NDC (`(px + 0.5) / vp_width`), Y-flipped, inverts camera-relative VP, adds camera position. Correct given depth buffer was produced by `view_projection_matrix_relative()` (which it is — every depth render in editor uses the relative path per §1.E).
- **`ray_from_screen` math** (line 511-526): uses **non-relative** `view_projection_matrix().inverse()`. If picking happens in editor where renderer used relative path, **this picks against ABSOLUTE world coordinates while everything was rendered at camera-relative**. At large camera positions this would diverge from depth-buffer-based unprojection. **Medium-confidence suspected bug**: ray_from_screen vs unproject_depth_to_world use different VP matrices.
- **Parameter clamping**: `sanitize()` (line 563-622) covers FOV (10°–170°), aspect (>0), near (>0), far (>near), distance, pitch (within min/max). Robust against deserialization. NaN handling explicit.
- **High-confidence bugs**: 0. **Medium-confidence**: 1 (`ray_from_screen` uses absolute VP while depth buffer was produced from relative VP — picking divergence at large camera world positions). Editor camera position is typically modest in scale (orbit around focal_point which is usually within terrain bounds), so this rarely triggers. But the path exists.

### 3.3 Gizmo `CameraController` (#4)

- **Projection/View**: standard `perspective_rh` + `look_at_rh`.
- **Quaternion orbit math** (line 76-92): yaw around `Vec3::Y`, pitch around `right = offset.cross(self.up).normalize()`. Subject to gimbal lock when `offset` is parallel to `up`. No clamping. **Medium-confidence latent bug**: at top/bottom view extremes, `offset.cross(up)` approaches zero, normalize → NaN propagation.
- **`set_view_top`** (line 159-164): switches `up` to `Vec3::NEG_Z` when looking straight down. This is correct (you need a different up vector at top view) but means the `up` field can change semantics during use. Subsequent orbit operations with the changed `up` produce different rotations than the default. **Low-confidence semantic foot-gun**: callers that read `up` post-set_view_top may be surprised.
- **No `sanitize()`**, no NaN guards.
- **Production caller status**: This is a *gizmo-subsystem* camera — needs runtime confirmation whether the editor actually uses `SceneViewport`, or whether the gizmo subsystem is dormant (per `docs/architecture/aw_editor.md`). Likely Astract subsystem, may be partially-wired.
- **High-confidence bugs**: 0. **Medium-confidence**: 1 (gimbal-lock NaN at top/bottom). **Low-confidence**: 1 (`up` field semantics changing during set_view_top).

### 3.4 unified_showcase bespoke camera (#23)

- **Projection**: `Mat4::perspective_rh(45.0_f32.to_radians(), aspect, 0.1, 2000.0)`. Z-far 2000m.
- **View construction**: `Mat4::look_at_rh(pos, pos + Quat::y(yaw)*Quat::x(pitch)*NEG_Z, +Y)`. Math is correct **given the convention** — yaw=0,pitch=0 ⇒ forward=NEG_Z. But this is the +Z-back convention used by Bevy/glTF/Three.js, NOT the +X-forward convention used by engine `Camera::dir`. **Convention divergence (not a bug per se), but the example is inconsistent with the engine's authoritative convention.**
- **Pitch clamping**: not visible in inspected lines — would need to inspect input handlers to confirm gimbal protection.
- **High-confidence bugs**: 0. **Medium-confidence**: 1 (likely missing pitch clamp at ±π/2 — needs verification). **Low-confidence**: 1 (convention mismatch with engine doesn't actually cause rendering bugs because the example doesn't share matrices with engine code; flagged as drift).

### 3.5 shadow_csm_demo Camera (#24)

- **Projection/view**: standard.
- **Convention**: yaw=0 ⇒ +X forward (matches engine).
- **No pitch clamp in inspected lines** (only sets `-20°` initial pitch at line 88). Likely unconstrained — would need to inspect input.
- **High-confidence bugs**: 0. **Medium-confidence**: 1 (likely missing pitch clamp).

### 3.6 fluids_demo Camera (#25)

- **Projection**: `Mat4::perspective_rh(fovy.to_radians(), aspect, znear, zfar)` — note `fovy` field is in **degrees** (`main.rs:31`), converted at projection construction. Different unit convention from engine.
- **View**: `look_at_rh(eye, target, up)` standard.
- **Orbit controls separated from Camera struct**: yaw/pitch/distance fields on `State`, computed eye = target + spherical(yaw, pitch, distance). Standard.
- **Aspect resize** at line 460: `self.camera.aspect = new_size.width as f32 / new_size.height as f32` — no `.max(1)` guard. Divide-by-zero risk if window minimizes to zero height.
- **High-confidence bugs**: 0. **Medium-confidence**: 1 (no aspect-zero guard at resize).

### 3.7 nanite_demo bare fields (#26)

- **View**: `look_at_rh(pos, pos + forward, +Y)`. Convention matches engine (+X forward at yaw=0).
- **Projection**: not visible in inspected lines.
- **No struct, no encapsulation.**
- **Pitch clamping**: not visible. `camera_pitch: -0.3` initial; would need to inspect input to confirm.
- **High-confidence bugs**: 0. **Medium-confidence**: 1 (likely missing pitch clamp).

### 3.8 Bench mock `Camera` (#29)

- **Convention**: **`-Vec3::Y` up at `camera_primitives_instancing.rs:35`**. This is the EXACT bug that the engine's `camera.rs:18-20` tombstone comment describes: "Previous `-Vec3::Y` caused clip-space w to go negative for all visible geometry … producing chunk-aligned rectangular voids in terrain."
- **High-confidence bug**: 1 (`-Vec3::Y` up vector — stale copy of pre-fix code).
- **Production impact**: zero. The bench is `benches/`, runs under `cargo bench`, not in production code path. It produces (likely) **flipped/inverted matrices** in benchmark workloads — bench results may report timings for a degenerate rendering case rather than the production case. Cosmetic from a runtime fidelity standpoint; semantically wrong from a benchmark validity standpoint.

### 3.9 `CameraKey` + `apply_camera_key` (#10)

- **Lerp** (`lib.rs:291-306`): linear interpolation of pos/look_at/fov_deg. Math correct.
- **Conversion to engine Camera** (`renderer.rs:3371-3381`): `dir = (look - pos).normalize_or_zero(); yaw = dir.z.atan2(dir.x); pitch = dir.y.clamp(-1.0, 1.0).asin();`. Correct inversion of `Camera::dir(yaw, pitch)`. Edge case: `normalize_or_zero` returns zero vector when `look == pos`, then `atan2(0,0) = 0` and `asin(0) = 0` — degenerate but non-NaN. Resulting camera looks down +X by default.
- **`is_typical_fov()`** validation method exists but is documentation-only — `apply_camera_key` does not call it. Out-of-range FOVs from authored cinematics pass through silently.
- **High-confidence bugs**: 0. **Medium-confidence**: 1 (FOV not validated on apply — `fov_deg = 0` or `fov_deg = 180` would produce degenerate projection silently). **Low-confidence**: 1 (`normalize_or_zero` fallback at zero distance silently produces unconstrained camera state).

### 3.10 Cross-implementation suspected bug count

- **High-confidence bugs**: **1** (§3.8 bench `-Vec3::Y` — non-production but invalidates benchmark results)
- **Medium-confidence bugs**: **6**
  1. Engine `Camera`: missing `fovy`/`znear<zfar` validation
  2. Editor `OrbitCamera`: `ray_from_screen` uses absolute VP vs depth uses relative VP
  3. Gizmo `CameraController`: gimbal-lock NaN at orbit top/bottom
  4. `unified_showcase`: likely missing pitch clamp
  5. `shadow_csm_demo`: likely missing pitch clamp
  6. `nanite_demo`: likely missing pitch clamp; `fluids_demo`: no aspect-zero guard
- **Low-confidence / unusual but possibly intentional**: **4**
  1. Engine Camera gimbal-instability at pitch=±π/2 (mitigated by controller clamp)
  2. Gizmo `up` field semantics changing during set_view_top
  3. `unified_showcase` -Z-forward convention mismatch with engine +X-forward (not a bug, drift)
  4. `apply_camera_key` `normalize_or_zero` fallback silent acceptance of degenerate keys

## 4. Phase 4 — Architectural Intent

### 4.A Pair A: Engine `Camera` (#1) vs Editor `OrbitCamera` (#3)

**Intent**: **Deliberate specialization.** The editor needs orbit-interaction (focal-point + spherical offset + zoom-to-cursor + pan + frame-entity + view bookmarks + screen-space picking + frustum extraction + serialization). The runtime needs free-fly with yaw/pitch state for camera-relative rendering. Each has UI/interaction requirements the other doesn't. Evidence:
- `tools/aw_editor/src/viewport/camera.rs:1-31` explicit docstring: "Professional camera controller using spherical coordinates for smooth orbit, pan, and zoom operations. Designed for 3D editing workflows."
- SOTA audit line 174: "Renderer `Camera`… editor `OrbitCamera`… coexist" — flagged as architectural fragmentation needing canonical contract, but the underlying specialization is acknowledged.
- `to_engine_camera()` exists as the conversion bridge (line 624-638) — implies both implementations were intended to coexist.

However, the **dual matrix path** (editor's `update_camera` bypasses `to_engine_camera`, water's `update_water` uses it) is **accidental accretion**, not specialization. The bypass was added as a fix in `df7649287` when the conversion path had a bug, and the conversion path was preserved for water without being similarly migrated. This is the SOTA audit's P0 finding (line 22) — still current.

### 4.B Pair B: Engine `CameraController` (#2) vs Gizmo `CameraController` (#4)

**Intent**: **Mixed.** The gizmo `CameraController` was added in commit `7a5fcab74 "Astract/Gizmos editor implementation"` — i.e., as part of the Astract gizmo subsystem, which is a separate editor add-on. The gizmo team built its own CameraController rather than depend on the engine's. The shape (position/target/up vs position/yaw/pitch) is different enough that simple reuse wasn't possible. But the engine's CameraController already has Orbit mode that supports target-based interaction, so the divergence is partly avoidable.

**Verdict**: half-specialized (different orbit math + view bookmarks) and half-accretion (could have wrapped engine `CameraController`'s Orbit mode rather than reimplementing).

### 4.C Pair C: Editor `OrbitCamera` (#3) vs Gizmo `CameraController` (#4)

**Intent**: **Accidental accretion.** Both are spherical orbit cameras used inside the editor. The gizmo subsystem has its own viewport-camera implementation that overlaps significantly with `OrbitCamera` in role. There's no clear specialization justification — gizmo doesn't need anything `OrbitCamera` lacks; `OrbitCamera` already exposes view_projection, inverse_view_projection, distance, view_* methods that the gizmo CameraController also provides. Production wiring unclear — gizmo subsystem may be partially dormant (per `docs/architecture/aw_editor.md` god-struct EditorApp); needs verification whether `SceneViewport` actually flows into the live editor.

### 4.D Pair D: Engine `Camera` (#1) vs Examples (#23–26)

**Intent**: **Mostly accidental accretion.** Most examples could use engine `Camera` + `CameraController`. The four that don't:
- `unified_showcase` — has bespoke camera bound to its bespoke renderer (uses neither `astraweave_render::Renderer` nor engine `Camera`). The entire example operates outside the engine's rendering crate. Justification: it's a "flagship showcase" that doubles as an experimental sandbox. The fact that backup variants (`main_temp`, `main_backup*`, `main_bevy*`, `main_clean`) accumulated in src/ suggests it's been a refactor target multiple times.
- `shadow_csm_demo` — own Camera with debug_mode field for visualizing shadow cascades; some legitimate specialization but the bulk could use engine Camera.
- `fluids_demo` — uses fluid-specific CameraUniform with view_inv, inv_view_proj fields; not in engine Camera. Specialization for fluid SPH rendering.
- `nanite_demo` — bare fields without struct, simplest possible. No specialization justification — could use engine Camera trivially.

Most examples already DO use engine `Camera + CameraController` (13 examples confirmed: `cutscene_render_demo`, `weaving_playground`, `physics_demo3d`, `npc_town_demo`, `navmesh_demo`, `visual_3d`, `debug_toolkit_demo`, `ui_controls_demo`, `audio_spatial_demo`, `biome_showcase`, `renderer_integration_test`, `hello_companion`, `veilweaver_demo`).

### 4.E Pair E: `CameraKey` (cinematics, #10) vs Engine `Camera` (#1)

**Intent**: **Deliberate specialization** for the data layer (CameraKey is interpolation-friendly tuple-based timeline data), but the conversion via `apply_camera_key` is **the only production consumer**. SOTA audit's P1 finding (line 23) says CameraKey should evolve into a continuous evaluator producing the canonical view contract; current state is acknowledged as transitional.

### 4.F Pair F: Bench mock #29 vs Engine `Camera` (#1)

**Intent**: **Accidental drift.** The bench's comment explicitly says "matching actual API without wgpu dependencies." It is a deliberate mock — but it was created when engine Camera used `-Vec3::Y`, and was not updated when engine fixed to `+Vec3::Y`. Pure stale-copy drift.

### 4.G Architectural intent summary

| Pair | Intent | Consolidation appropriate? |
|---|---|---|
| A: Engine Camera ↔ Editor OrbitCamera | Specialization for data + UI, accretion in dual update paths (water) | **Partial** — keep both, unify the upload path |
| B: Engine CameraController ↔ Gizmo CameraController | Half-specialized, half-accretion | **Yes, mostly** — Gizmo could wrap engine Orbit mode |
| C: Editor OrbitCamera ↔ Gizmo CameraController | Accretion | **Yes** — Gizmo could use OrbitCamera if wired |
| D: Engine Camera ↔ Examples | Mostly accretion (3 of 4 unjustified) | **Yes for most** — examples could use engine; flagship `unified_showcase` is a separate question |
| E: CameraKey ↔ Engine Camera | Specialization for data; conversion is correct | **No** — but SOTA recommends evaluator layer |
| F: Bench mock ↔ Engine Camera | Drift | **Yes** — bench should be updated to match production convention |

## 5. Phase 5 — Andrew's Specific Concern

### 5.1 Same-scene camera output comparison

**Hypothesis**: Editor (`OrbitCamera` via `engine_adapter.update_camera`) and runtime (`Camera` via `Renderer.update_camera`) produce equivalent rendered output for the same scene at the same world-space position/orientation.

**Evidence**:
- Editor path uploads `OrbitCamera.view_matrix() = look_at_rh(position(), focal_point, +Y)` and `OrbitCamera.projection_matrix() = perspective_rh(fov.to_radians(), aspect, near, far)` via `update_camera_matrices`.
- Runtime path uploads `engine_Camera.view_matrix() = look_to_rh(position, dir(yaw, pitch), +Y)` and `engine_Camera.proj_matrix() = perspective_rh(fovy, aspect.max(0.01), znear, zfar)` via `update_camera`.
- If `OrbitCamera.position() == engine_Camera.position` and the direction from `OrbitCamera.position()` toward `focal_point` equals `engine_Camera::dir(engine_yaw, engine_pitch)`, the view matrices are mathematically equivalent. The `to_engine_camera()` conversion (§3.2 calculation) is correct at that math.
- However, the editor never calls `to_engine_camera()` in its main render path — it passes OrbitCamera matrices directly. So the editor's view matrix is unambiguously what OrbitCamera produces; no conversion can drift here. The matrices ARE what OrbitCamera outputs.
- The runtime camera's matrices are unambiguously what engine Camera produces.
- For a given physical camera placement (same world position, same look direction, same FOV/aspect/near/far), both paths produce the same view matrix (up to floating-point precision). **Verified.**
- For projection matrices, both use `Mat4::perspective_rh` with `vertical FOV in radians` semantics. The editor converts `fov` (deg) → radians via `.to_radians()` at projection time; the runtime stores `fovy` in radians directly. Same downstream matrix when the radian values match. **Verified.**

**Conclusion**: **Verified.** The editor's rendering of the same scene at the same camera state is faithful to what the runtime would produce. The parity contract (P.7) protects this for the harness fixture, and the math holds in general for OrbitCamera ⇌ engine Camera placement.

### 5.2 Known visual artifacts traceable to camera

| Artifact | Trace | Status |
|---|---|---|
| Chunk-aligned rectangular voids in terrain (historical) | `astraweave-render/src/camera.rs:18-20` tombstone — caused by `-Vec3::Y` up vector | **Fixed** in commit `df7649287`; bench mock #29 retains the bug |
| Camera pivot vibration in unified_showcase (historical) | Commit `0fa98c711 "Fix camera pivot vibration issue in unified_showcase"` | **Fixed** in that commit (not inspected for current state) |
| Editor↔runtime convention drift (yaw/pitch mismatch via to_engine_camera) | Commit `df7649287 "fix(editor): modify camera update to use direct view/proj matrices from OrbitCamera"` | **Fixed** by deliberately bypassing to_engine_camera in editor's update_camera. **Latent** in update_water path (still goes through to_engine_camera). |
| Gimbal instability at pitch ≈ ±π/2 in engine Camera | `docs/audits/tonemap_double_application_investigation_2026-04-19.md:221` | Latent footgun in API; mitigated in production by controller clamps |
| OrbitCamera `ray_from_screen` (absolute VP) vs `unproject_depth_to_world` (relative VP) divergence | §3.2 medium-confidence | Latent at large camera world positions. Editor focal-point typically modest, so rarely triggers in practice. |

**No known visual artifacts currently observable to the user from the editor side beyond the ones above.** The Editor-Engine Render Parity campaign (P.0–P.7) protected the parity contract for the fixture used at ToD 12.0 with the orbit camera at yaw=π/4, pitch=π/6, distance=25, aspect=1:1.

### 5.3 Parity harness camera coverage

**Harness fixture** (`render_parity_harness.rs:348-359`):

```
OrbitCamera::new(
    Vec3::ZERO,                    // focal point at origin
    25.0,                          // distance
    std::f32::consts::FRAC_PI_4,   // yaw = π/4 = 45°
    std::f32::consts::FRAC_PI_6,   // pitch = π/6 = 30°
)
set_aspect(width, height)          // width = height (1:1 aspect)
near = 0.5, far = 5000.0           // engine path explicit override at lines 497-498
fovy = 60° (radians)               // engine path explicit override at line 499
```

**What the fixture exercises**:
- Mid-range pitch (no pole singularity).
- Standard 45° diagonal yaw.
- Square aspect.
- Single physical camera placement; one of infinitely many.

**What the fixture does NOT exercise**:
1. **Extreme pitch** (near ±π/2 — gimbal region).
2. **Non-square aspect** (resize behavior, divide-by-near-zero risks).
3. **Large camera world positions** (camera-relative rendering correctness — the focal_point is at ZERO, so eye is only 25 m from origin).
4. **Camera *motion*** — the fixture is stationary. Two-frame settle then measure; no inter-frame jitter or matrix transitions.
5. **The `update_water` dual-conversion path** — water is not exercised by the parity fixture (no water plane in the fixture scene per the harness setup).
6. **`to_engine_camera()` path** — only the direct-matrix path is exercised; the conversion path is structurally bypassed by the editor's main `update_camera` but reachable in `update_water`.
7. **FOV/near/far at extremes** (very narrow FOV, FOV near 170°, near=0.01, far=50000 — none of these tested).
8. **Authored cinematic camera keys** flowing through `apply_camera_key` to runtime.

**Verdict**: **The parity harness adequately covers the common case but has structural blind spots.** The fixture's stationary square camera at moderate pitch is not adversarial enough to catch:
- pitch-clamping bugs in any implementation,
- aspect-zero divisions on resize,
- the dual-water-conversion latent code path,
- camera-relative drift at large world positions,
- cinematics→runtime camera evaluation.

The parity contract "editor pixels match runtime pixels for the same scene" is **verified for the fixture** and **not falsified for other camera states by the harness**.

### 5.4 Specific answer to Andrew's concern

**Is the editor a faithful preview of what runtime would render for the same scene?**

**For the camera component alone, given a *static* camera placement at modest world coordinates with non-extreme pitch and a square-or-typical aspect ratio**: **YES**. The math is correct, the matrices match the runtime's, the parity contract is enforced for the fixture, and the editor deliberately bypasses the historically-bug-prone `to_engine_camera()` conversion path in its main update.

**For camera placements outside the validated range**: **NOT GUARANTEED**. Specifically:
- At pitch near ±π/2 the engine Camera produces a degenerate view matrix (gimbal). Editor's `OrbitCamera` clamps to ±88.6°, partly protecting the editor but not the runtime if a cinematic or external producer feeds extreme pitch to engine Camera directly.
- At non-square aspect, examples (`fluids_demo`, `shadow_csm_demo`, `hello_companion`, `renderer_integration_test`) lack divide-by-zero protection. Editor's `set_aspect` does have it.
- At large camera world positions (>10 km off origin), the `ray_from_screen` vs `unproject_depth_to_world` divergence in OrbitCamera could produce inconsistent picking. The parity harness doesn't test this.
- For cinematic-driven cameras, `apply_camera_key` silently accepts FOV=0 and degenerate look_at=pos keys.

**The concrete potential corruption pattern for A.6 wire-ups**: If A.6 ships "grassland biome renders correctly in editor" as closure criterion, **the editor's rendering at the fixture camera placement is reliable**. If A.6's biome appears wrong in editor and the user verifies with the parity harness (matching pixels at ToD 12, orbit at π/4, π/6, 25 m) and pixels do match, then it's the biome that's wrong, not the camera. If the user wants to verify at non-fixture camera placements (interactive orbit, extreme pitches, non-square aspects), the camera fidelity is **not guaranteed** by the existing parity contract — though no specific bug is known to manifest there.

## 6. Phase 6 — Forward Chain (Andrew Gate Outcome)

### 6.1 Net findings vs SOTA audit (April 28)

The April 28 SOTA audit's findings are **all still current** as of May 18:
- P0 dual-conversion path (`to_engine_camera` in water) — still reachable code.
- P0 no canonical camera/view contract — unchanged.
- P1 cinematics emit events not continuous state — unchanged.
- P1 renderer UBO is minimal — unchanged.
- P1 camera-relative is feature-gated — unchanged.
- P2 editor camera combines navigation + persistence + picking + adaptation — unchanged.

C.0 **adds 4 findings the SOTA audit did not surface**:
1. **Bench mock #29 retains `-Vec3::Y` up bug** — single high-confidence bug (non-production, benchmark only). Bench results may be measuring degenerate rendering.
2. **Gizmo subsystem has its own `CameraController`** (§1.A #4, `scene_viewport.rs`) — third orbit camera in the workspace, undocumented in SOTA audit. Production-wiring status uncertain; may be Astract-subsystem dormant.
3. **`OrbitCamera::ray_from_screen` uses absolute VP, `unproject_depth_to_world` uses relative VP** — medium-confidence latent inconsistency that only triggers at large camera world positions.
4. **`unified_showcase`, `shadow_csm_demo`, `nanite_demo` likely lack pitch clamping** (medium-confidence; needs runtime inspection to confirm). Engine Camera's gimbal foot-gun is reachable through unconstrained example controllers.

### 6.2 Andrew's gate choice

Andrew chose **Unified Camera crate (largest)** during the Phase 6 gate. This is full SOTA-audit-recommended consolidation: new shared crate with canonical `CameraState`/`Projection`/`RenderView` types, camera producers (FreeFly, Orbit, Follow, Cinematic, Debug), camera manager/blender, renderer consumes `RenderView` only.

Andrew also approved this audit doc artifact.

### 6.3 Implications for Terrain Asset Quality

The Unified Camera crate is multi-week scope. Terrain Asset Quality (A.5+) remains **paused** until the Unified Camera campaign closes. The audit found no specific A.6+ corruption pattern that the parity contract doesn't already protect, but Andrew's prioritization treats the architectural-priority validation as worth the pause.

### 6.4 Forward chain shape (informational; planning happens in a separate session)

Per C.0 anti-drift constraint 4, **this audit does not draft C.1+ prompts**. The Unified Camera campaign's sub-phase structure will be planned in a separate session, informed by:
- This audit's Phase 1–5 findings.
- The April 28 SOTA audit's staged migration plan (§5 Phases 0–5).
- The 4 new C.0 findings beyond the SOTA audit.

Anticipated rough campaign shape (subject to planning-session refinement):
- **C.1 Phase 0**: Document and lock conventions formally; add contract tests.
- **C.2 Phase 1**: Introduce canonical types (`CameraState`, `Projection`, `RenderView`) without rewiring; adapters from existing implementations.
- **C.3 Phase 2**: Consolidate renderer camera upload; unify `update_camera` + `update_camera_matrices`; eliminate `to_engine_camera` dual path; migrate `update_water`.
- **C.4 Phase 3**: Camera manager/blender with producers.
- **C.5 Phase 4**: Cinematics evaluator upgrade.
- **C.6 Phase 5**: Multi-view preparation.
- Possibly **C.7**: Camera Parity harness expansion (extreme pitch, non-square aspect, large world positions, cinematic-driven coverage).

This is not a commitment — the planning session will determine the actual sub-phase structure.

## 7. §7.11 implications (informational, not codification)

C.0's existence (running this audit *before* resuming Terrain Asset Quality, prospectively) is the second concrete reinforcement of Pillar 7 (architectural-priority validation). The parity campaign demonstrated Pillar 7 reactively (post-divergence-discovery). C.0 demonstrates Pillar 7 proactively (pre-feature-resumption audit). Both are the same pillar; both directions worth codifying at the eventual methodology elevation moment. C.0 notes this for future capture, does not codify.

## 8. Anti-drift compliance

Per C.0 anti-drift constraints 1–17, this audit:
- ✅ Modified no source files.
- ✅ Modified no campaign or architecture docs.
- ✅ Did not "fix" any camera bug.
- ✅ Did not draft C.1+ prompts.
- ✅ Did not resume Terrain Asset Quality.
- ✅ Did not engage prior-campaign follow-ups or post-P.7 cleanup candidates.
- ✅ Did not touch render/terrain/editor source.
- ✅ Ran no GPU validation; static source analysis only.
- ✅ Modified no material/manifest/asset files.
- ✅ Authored no cargo example.
- ✅ Did not codify §7.11 candidate pillars (noted for future, not codified).
- ✅ Did not propose remediation outside camera correctness and consolidation.
- ✅ Did not modify parity harness or campaign-outcome doc.
- ✅ Did not engage broader paused/in-progress campaigns beyond noting cross-campaign implications.
- ✅ Stopped at Phase 6 gate; did not autonomously proceed.

This commit modifies only `docs/audits/camera_system_architecture_audit_2026-05.md` (this file). No other files are touched.

## Verification note

This was a read-only static source analysis. No code was changed, no tests were written, and no cargo checks were run. All citations are file:line references into the workspace as of 2026-05-18 (commit `6dc95ae9b` `Editor-Engine Render Parity P.7: parity validation closure`).
