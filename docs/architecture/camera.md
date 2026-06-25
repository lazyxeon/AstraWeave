---
schema_version: 1
trace_id: camera
title: "Camera System"
description: "Camera — freefly, projection, render-view, parity producer"
primary_crate: astraweave-camera
domain: rendering
lifecycle_status: active
integration_status: wired
owns: [astraweave-camera]
doc_version: "1.1"
last_verified_commit: 7c29b8182
---

# Architecture Trace: Camera System

## Metadata

| Field | Value |
|---|---|
| **System name** | Camera System |
| **Primary crates** | [`astraweave-camera`](../../astraweave-camera) (canonical types + FreeFly producer); consumed by [`astraweave-render`](../../astraweave-render) and [`tools/aw_editor`](../../tools/aw_editor) (OrbitCamera producer) |
| **Document version** | 1.1 |
| **Last verified against commit** | `7c29b8182` |
| **Last verified date** | 2026-06-25 |
| **Status** | Active. Unified Camera campaign (C.0→C.9) closed 2026-06-01 — single upload contract, two producers, hardened cinematics path. |
| **Owner notes** | Product of the Unified Camera campaign. Canonical convention reference: [`docs/current/CAMERA_CONVENTIONS.md`](../current/CAMERA_CONVENTIONS.md). Campaign outcome: [`docs/audits/unified_camera_outcome_2026-06.md`](../audits/unified_camera_outcome_2026-06.md). |

---

## 1. Executive Summary

**What this system does:**
Provides the canonical camera math types and producer interface for the engine: every camera implementation converts its state into a single `RenderView` (matrices + position + view direction + projection params) that the renderer consumes exclusively.

**Why it exists:**
The C.0 audit ([`camera_system_architecture_audit_2026-05.md`](../audits/camera_system_architecture_audit_2026-05.md)) found 8 active 3D camera codepaths with three competing yaw=0 forward conventions (+X, −Z, −X), dual renderer upload paths, and three parallel cinematics camera systems; the Unified Camera campaign consolidated this into one upload contract (`RenderView`), one upload entry point (`Renderer::update_view`), and two producers (`FreeFly`, `OrbitCamera`).

**Where it primarily lives:**
- [`astraweave-camera/src/`](../../astraweave-camera/src) — `Projection` (`projection.rs`), `RenderView` (`render_view.rs`), `CameraProducer` trait (`producer.rs`), `FreeFly` + `CameraController` (`freefly.rs`).
- [`tools/aw_editor/src/viewport/camera.rs`](../../tools/aw_editor/src/viewport/camera.rs) — `OrbitCamera`, the second `CameraProducer` (editor-only).
- [`astraweave-render/src/renderer.rs`](../../astraweave-render/src/renderer.rs) — `Renderer::update_view(&RenderView)` (consumer) and `tick_cinematics` / `apply_camera_key` (cinematics→FreeFly bridge).

**Status note:**
The campaign is **closed and immutable** as recorded in the outcome doc; the conventions in [`CAMERA_CONVENTIONS.md`](../current/CAMERA_CONVENTIONS.md) §2 are authoritative — *discrepancies between code and that doc are bugs in the code, not the doc* (`CAMERA_CONVENTIONS.md` §1). `astraweave-camera` itself has **no `wgpu` coupling** — it produces pure math types ([`lib.rs:29-32`](../../astraweave-camera/src/lib.rs)).

---

## 2. Authoritative Pipeline

```text
[Camera state: FreeFly (yaw/pitch/pos/fovy/aspect/znear/zfar)  OR  OrbitCamera (focal_point/yaw/pitch/distance/fovy)]
    │
    │ CameraProducer::to_render_view(&self)   (world-relative, the trait method)
    │ — OR — concrete FreeFly::to_render_view_camera_relative(&self)  (large-position precision; NOT on trait)
    ▼
[Projection::perspective(fovy, aspect, znear, zfar)]
    file: astraweave-camera/src/projection.rs:67-87
    role: build view→clip matrix via Mat4::perspective_rh(fovy, aspect.max(0.01), znear, zfar)
    key data: Projection { matrix, fovy, aspect (pre-floor), znear, zfar }
    │
    │ view matrix built via Mat4::look_to_rh(position, dir(yaw,pitch), Vec3::Y)   (FreeFly)
    │                  or  Mat4::look_at_rh(position(), focal_point, Vec3::Y)      (OrbitCamera)
    ▼
[RenderView::new(view, &projection, position, view_dir)]
    file: astraweave-camera/src/render_view.rs:126-143
    role: compute derived matrices once on CPU (view_proj, inverse_view, inverse_view_proj)
    key data: RenderView { view, projection, view_proj, inverse_view, inverse_view_proj,
                           position, view_dir, fovy, aspect, znear, zfar }
    │
    │ Renderer::update_view(&RenderView)   — the SOLE upload entry point
    ▼
[Renderer GPU upload: camera UBO, CSM cascade splits, water/sky/impostor passes]
    file: astraweave-render/src/renderer.rs:4093 (update_view)
    role: store view_proj in camera UBO; derive shadow subviews from this primary RenderView
    key data: GPU camera uniforms consumed by shaders
```

### Stage-by-stage detail

#### Stage 1: Producer → RenderView
**File(s):** [`astraweave-camera/src/freefly.rs:137-151`](../../astraweave-camera/src/freefly.rs) (`FreeFly::to_render_view`); [`tools/aw_editor/src/viewport/camera.rs`](../../tools/aw_editor/src/viewport/camera.rs) (`OrbitCamera::to_render_view`).
**Role:** Each producer commits its current state to a `RenderView` via the `CameraProducer` trait.
**Inputs:** Producer struct fields (position/yaw/pitch/fovy/etc. for FreeFly; focal_point/spherical params for OrbitCamera).
**Outputs:** A `RenderView`.
**Notes:** `FreeFly::to_render_view` constructs `Projection::perspective(...)`, the view via `view_matrix()`, and `view_dir` via `Self::dir(yaw, pitch)` ([`freefly.rs:145-150`](../../astraweave-camera/src/freefly.rs)). `OrbitCamera` mirrors this pattern with the target-based `look_at_rh` style and derives `view_dir` from `focal_point − position()` (per `CAMERA_CONVENTIONS.md` §3 #3 closed-row text).

#### Stage 2: Projection construction
**File(s):** [`astraweave-camera/src/projection.rs:67-87`](../../astraweave-camera/src/projection.rs).
**Role:** Build the perspective projection matrix and carry the originating parameters.
**Inputs:** `fovy` (radians), `aspect`, `znear`, `zfar`.
**Outputs:** `Projection { matrix, fovy, aspect, znear, zfar }`.
**Notes:** `znear > 0` and `zfar > znear` are `debug_assert`ed (panic in debug, silent in release); the only *release-active* defense is the `.max(0.01)` aspect floor at matrix construction ([`projection.rs:68-79`](../../astraweave-camera/src/projection.rs)). The stored `aspect` field is the **pre-floor** value — callers wanting the floored value recompute `self.aspect.max(0.01)` ([`projection.rs:40-43`](../../astraweave-camera/src/projection.rs)). Matrix uses `Mat4::perspective_rh` (wgpu `[0,1]` depth), never `perspective_rh_gl`.

#### Stage 3: RenderView derived-matrix computation
**File(s):** [`astraweave-camera/src/render_view.rs:126-143`](../../astraweave-camera/src/render_view.rs).
**Role:** Precompute `view_proj = projection.matrix * view`, `inverse_view`, `inverse_view_proj` once on CPU.
**Inputs:** `view: Mat4`, `&Projection`, `position: Vec3`, `view_dir: Vec3`.
**Outputs:** Fully-populated `RenderView`.
**Notes:** Caller responsibilities are documented in the constructor doc-comment: `position` should equal `inverse_view.col(3).xyz` (or the camera-relative equivalent), and `view_dir` should equal `-inverse_view.col(2).xyz` ([`render_view.rs:118-125`](../../astraweave-camera/src/render_view.rs)). TAA jitter fields (`unjittered_*`, `previous_view_proj`) and explicit basis-vector fields (`right`, `up`) are **deferred** — shaders extract basis from `inverse_view` columns ([`render_view.rs:33-48`](../../astraweave-camera/src/render_view.rs)).

#### Stage 4: Renderer upload
**File(s):** [`astraweave-render/src/renderer.rs:4093`](../../astraweave-render/src/renderer.rs) (`update_view`).
**Role:** The sole camera-upload entry point; stores `view_proj` in the camera UBO and derives shadow CSM subviews from this single primary `RenderView`.
**Inputs:** `&RenderView`.
**Outputs:** GPU camera uniforms.
**Notes:** Per `CAMERA_CONVENTIONS.md` §2.9, there are no per-producer-type renderer APIs. The CSM cascade computation derives subview matrices inside the renderer from the one primary `RenderView` (`producer.rs:32-41` doc-comment; `CAMERA_CONVENTIONS.md` §2.9). `Renderer::update_view` stores the supplied `view_proj` directly without per-pipeline transformation (`engine_adapter.rs:776-779` doc-comment).

#### Stage 5 (specialized): Cinematics bridge
**File(s):** [`astraweave-render/src/renderer.rs:3420`](../../astraweave-render/src/renderer.rs) (`apply_camera_key`, private), [`renderer.rs:3492`](../../astraweave-render/src/renderer.rs) (`tick_cinematics`).
**Role:** Route `astraweave_cinematics::CameraKey` keyframes through the `FreeFly` producer.
**Inputs:** `dt`, `&mut FreeFly`, the active cinematics `Timeline`.
**Outputs:** `Vec<SequencerEvent>`; mutates the supplied `FreeFly`.
**Notes:** `tick_cinematics(dt, &mut FreeFly)` is the only public reach; it dispatches `CameraKey` events to private `apply_camera_key`, which clone-and-`sanitize()`s the key, converts `look_at`→yaw/pitch, and sets `cam.fovy = fov_deg.to_radians()` at the producer boundary (the degrees→radians conversion). `FreeFly` then produces the `RenderView`. This is the "route, don't replace" resolution of the C.0-planned evaluator-elimination (`unified_camera_outcome_2026-06.md` §2 cinematics path; `CAMERA_CONVENTIONS.md` §3 #10 rows).

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **Producer** | A camera type that can commit its state to a `RenderView` via `CameraProducer::to_render_view`. Two exist: `FreeFly`, `OrbitCamera`. | `producer.rs`, `freefly.rs`, `camera.rs` |
| **`RenderView`** | The minimum upload contract: view/projection/view_proj/inverse matrices + position + view_dir + projection params. The renderer consumes this exclusively. | `render_view.rs` |
| **`Projection`** | Perspective projection carrying both the derived matrix and the originating `fovy/aspect/znear/zfar` (no orthographic variant yet — deferred). | `projection.rs` |
| **`FreeFly`** | The engine free-fly producer (yaw/pitch/position + projection params). Formerly `astraweave_render::camera::Camera`. | `freefly.rs` |
| **`CameraController`** | The input/movement controller that mutates a `FreeFly` (WASD, mouse-look smoothing, orbit/freefly mode, FOV-zoom). **Not** itself a producer. | `freefly.rs:160-400` |
| **`OrbitCamera`** | The editor producer (focal_point + spherical yaw/pitch/distance). Lives in `tools/aw_editor`, not in `astraweave-camera`. | `tools/aw_editor/src/viewport/camera.rs` |
| **`CameraKey` / `CameraKeyframe`** | Cinematics keyframe (`pos`, `look_at`, `fov_deg`). `CameraKey` (in `astraweave-cinematics`) is canonical; the editor's `CameraKeyframe` was retired into it in C.7.C. | `astraweave-cinematics`, `renderer.rs` |
| **Camera-relative rendering** | A producer builds `view` with camera position pre-subtracted (eye at origin in view-construction space) and reports `position` separately, to avoid f32 jitter at large world coordinates. The *producer's* responsibility, not encoded in `RenderView` layout. | `freefly.rs:64-94`, `render_view.rs:19-31` |
| **`view_dir`** | World-space camera forward (unit). At yaw=0,pitch=0 equals `Vec3::X`. Equals `-inverse_view.col(2).xyz`. | `render_view.rs:89-95` |

### Terms to NOT confuse

- **`FreeFly` vs `CameraController`:** `FreeFly` is the camera *state* and producer (it owns the matrices). `CameraController` is the *input handler* that mutates a `FreeFly`'s yaw/pitch/position/fovy each frame; it does not implement `CameraProducer`. A `CameraMode` enum on the controller (`FreeFly` / `Orbit`) is **a movement mode of the controller**, not the editor's `OrbitCamera` type ([`freefly.rs:153-158`](../../astraweave-camera/src/freefly.rs)).
- **`CameraController::CameraMode::Orbit` vs editor `OrbitCamera`:** the controller's `Orbit` mode is a free-fly-camera movement style (WASD moves an orbit target, mouse rotates around it) implemented entirely on `FreeFly`+`CameraController` in `astraweave-camera`. The editor's `OrbitCamera` (`tools/aw_editor`) is a *separate producer type* with its own spherical state and picking surface. They are unrelated implementations that share a word.
- **`fovy` (radians) vs `fov`/`fov_deg` (degrees):** the canonical field is `fovy` storing **radians** (§2.1). `OrbitCamera` keeps a degrees *boundary* at its UI (`set_fov(degrees)`, `fov_degrees()`) but stores `fovy` radians internally (C.4.B). `CameraKey.fov_deg` stays degrees as the data-layer keyframe format and converts at the apply boundary.
- **`to_render_view` (trait, world-relative) vs `to_render_view_camera_relative` (concrete, off-trait):** the trait method is always world-relative. The camera-relative variant is an opt-in concrete capability, **not** on the `CameraProducer` trait ([`freefly.rs:74-94`](../../astraweave-camera/src/freefly.rs); `CAMERA_CONVENTIONS.md` §2.9). It is **not** FreeFly-exclusive: `OrbitCamera` also exposes its own `to_render_view_camera_relative` ([`tools/aw_editor/src/viewport/camera.rs:863`](../../tools/aw_editor/src/viewport/camera.rs)) — and the editor's main render path uses it. Both producers carry the method as a concrete-type capability; neither puts it on the trait.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| Input (`winit`) | `CameraController::process_keyboard / process_mouse_button / process_mouse_move / process_mouse_delta / process_scroll` ([`freefly.rs:226-314`](../../astraweave-camera/src/freefly.rs)) | `winit::keyboard::KeyCode`, `winit::event::MouseButton`, `Vec2` deltas | `astraweave-camera` depends on `winit` directly (Cargo.toml). Controller accumulates input, then `update_camera(&mut FreeFly, dt)` applies it. |
| Cinematics (`astraweave-cinematics`) | `Renderer::tick_cinematics(dt, &mut FreeFly)` → private `apply_camera_key` ([`renderer.rs:3420,3492`](../../astraweave-render/src/renderer.rs)) | `CameraKey { pos, look_at, fov_deg }` | Mutates a caller-supplied `&mut FreeFly`; `aspect/znear/zfar` persist through apply. First production caller: `examples/cutscene_render_demo` (C.7.B). |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| Render (`astraweave-render`) | `Renderer::update_view(&RenderView)` ([`renderer.rs:4093`](../../astraweave-render/src/renderer.rs)) | `&RenderView` | The SOLE upload entry point. The renderer's update path is **not** generic over `CameraProducer` — it takes `&RenderView` directly (`producer.rs:19-31`). |
| Render shadow CSM | derives subview matrices from the primary `RenderView` inside the renderer | `view_proj`, `position`, `fovy/aspect/znear/zfar` | Single-view producer; multi-view (`to_render_views`) is an additive future option, not in scope (`producer.rs:33-41`). |
| Render water pass | `EngineRenderAdapter::update_water(&RenderView, time)` ([`engine_adapter.rs:3836`](../../tools/aw_editor/src/viewport/engine_adapter.rs)) | `&RenderView` | Editor viewport calls `adapter.update_water(&camera.to_render_view(), water_time)` ([`renderer.rs:696-697`](../../tools/aw_editor/src/viewport/renderer.rs)). |
| Editor viewport | `EngineRenderAdapter::update_camera` → `self.renderer.update_view(&camera.to_render_view())` ([`engine_adapter.rs:794-795`](../../tools/aw_editor/src/viewport/engine_adapter.rs)) | `&RenderView` from `OrbitCamera` | Single-line delegation through the canonical contract (C.4). |
| Editor picking | `OrbitCamera::ray_from_screen` / `unproject_depth_to_world` (`tools/aw_editor/src/viewport/camera.rs`) | screen coords → world ray | Producer-specific ergonomic surface (lives on the concrete type, not the trait). Migrated to precision-stable camera-relative VP in C.4. |

### Bidirectional / Coupled

- **`CameraController` ↔ `FreeFly`:** the controller holds no camera; it borrows `&mut FreeFly` on each input/update call and writes back yaw/pitch/position/fovy. Tightly coupled by method signatures ([`freefly.rs:257-399`](../../astraweave-camera/src/freefly.rs)).
- **Cinematics ↔ `FreeFly`:** `tick_cinematics` borrows `&mut FreeFly`, overwriting its yaw/pitch/fovy from keyframes while preserving aspect/znear/zfar (`unified_camera_outcome_2026-06.md` §2).

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| [`astraweave-camera/src/lib.rs`](../../astraweave-camera/src/lib.rs) | Crate root; re-exports `FreeFly`, `CameraController`, `CameraMode`, `CameraProducer`, `Projection`, `RenderView` | Active | `glam`-only (+ optional `serde`); no `wgpu`. |
| [`astraweave-camera/src/projection.rs`](../../astraweave-camera/src/projection.rs) | `Projection::perspective` (matrix + params) | Active | `.max(0.01)` aspect floor; `debug_assert` near/far. |
| [`astraweave-camera/src/render_view.rs`](../../astraweave-camera/src/render_view.rs) | `RenderView` upload contract + `RenderView::new` | Active | Computes inverse matrices once. TAA/basis fields deferred. |
| [`astraweave-camera/src/producer.rs`](../../astraweave-camera/src/producer.rs) | `CameraProducer` trait (one method `to_render_view`) | Active | Minimal trait; renderer update path is NOT generic over it. |
| [`astraweave-camera/src/freefly.rs`](../../astraweave-camera/src/freefly.rs) | `FreeFly` producer + `CameraController` + `CameraMode` + `sanitize()` | Active | The bulk of the crate (771 LoC; ~370 are tests). |
| [`tools/aw_editor/src/viewport/camera.rs`](../../tools/aw_editor/src/viewport/camera.rs) | `OrbitCamera` — the second `CameraProducer` (editor) | Active | Lives in the editor, not `astraweave-camera`. Owns picking surface, serde shadow type. |
| [`astraweave-render/src/renderer.rs`](../../astraweave-render/src/renderer.rs) | `update_view` (consumer), `tick_cinematics`/`apply_camera_key` (cinematics bridge) | Active | `apply_camera_key` is private. |
| [`astraweave-render/tests/camera_conventions.rs`](../../astraweave-render/tests/camera_conventions.rs) | Contract tests asserting the production camera obeys §2 conventions | Active | The convention's structural protection (§2.4 negative test, bench-mock check). |
| [`tools/aw_editor/tests/render_parity_harness.rs`](../../tools/aw_editor/tests/render_parity_harness.rs) | Standing regression guard: GPU SHA parity + 16 C.8 matrix fixtures | Active | Four fixture families; baselines independently re-derived (not GPU SHA). |
| [`tools/aw_editor/tests/orbit_camera_producer.rs`](../../tools/aw_editor/tests/orbit_camera_producer.rs) | `OrbitCamera` `CameraProducer` conformance tests | Active | 10/10 after C.4.B. |
| [`tools/aw_editor/tests/picking_consistency.rs`](../../tools/aw_editor/tests/picking_consistency.rs) | `ray_from_screen` vs `unproject_depth_to_world` agreement | Active | Closure proof for the C.4 picking-precision fix. |

**Deleted (no longer present, recorded for orientation):** `astraweave-render/src/camera.rs` shim (removed C.3.C); gizmo `CameraController`/`SceneViewport` (removed C.6.A); editor `CameraKeyframe` (retired into `CameraKey` C.7.C); `Renderer::update_camera`/`update_camera_matrices` + `CameraUploadPath` enum (deleted C.3.C). See `CAMERA_CONVENTIONS.md` §3 and `unified_camera_outcome_2026-06.md` §1 for the deletion ledger.

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| `FreeFly` producer (engine) | `astraweave-camera/src/freefly.rs` | Active | Canonical engine producer. |
| `OrbitCamera` producer (editor) | `tools/aw_editor/src/viewport/camera.rs` | Active | Canonical editor producer. Two producers by design (`unified_camera_outcome_2026-06.md` §2). |
| `CameraController`'s internal `CameraMode::Orbit` movement mode | `astraweave-camera/src/freefly.rs:153-158,308-397` | Active | A free-fly *movement style* on `FreeFly`; distinct from the editor `OrbitCamera` type (see Naming collisions). |
| Bespoke per-example cameras (`unified_showcase`, `shadow_csm_demo`, `fluids_demo`) | `examples/*/src/main.rs` | Active (intentional sandboxes) | C.6 formalized these as deviations (e.g. `unified_showcase` uses −Z forward and a bespoke wgpu pipeline outside `Renderer`); migrating them was ruled out of campaign scope. They are NOT part of the canonical pipeline. See `CAMERA_CONVENTIONS.md` §3 #23–#26. |

### Naming collisions

- **"Orbit":** In `astraweave-camera`, `CameraMode::Orbit` is a free-fly *movement mode* of `CameraController` (WASD moves an orbit target, mouse rotates around it). In `tools/aw_editor`, `OrbitCamera` is a separate *producer type* with its own spherical state and picking. Same word, unrelated implementations.
- **"Camera":** Historically `astraweave_render::camera::Camera` was the engine free-fly camera; it was renamed `FreeFly` workspace-wide in C.3.C. Per-file local import aliases `use astraweave_camera::FreeFly as Camera;` were preserved at migration sites (anti-drift constraint forbids global aliases / re-exports, NOT per-file local aliases — `CAMERA_CONVENTIONS.md` §3 C.3.C log). So `Camera` may still appear as a local alias in some files; the canonical type name is `FreeFly`.
- **"fov" / "fovy" / "fov_deg":** `fovy` = canonical radians (§2.1). `fov_deg` = cinematics data-layer degrees (`CameraKey`). `OrbitCamera` exposes `set_fov(degrees)`/`fov_degrees()` UI-boundary methods over an internal `fovy` radians field.

### Known cognitive traps

- **Trap:** Editing `Projection`/`FreeFly` and assuming near/far validation runs in release.
- **Why it's confusing:** `Projection::perspective` `debug_assert`s `znear > 0` and `zfar > znear` — these panic in debug but are **silent in release** ([`projection.rs:68-78`](../../astraweave-camera/src/projection.rs)). The only release-active guard is `.max(0.01)` on aspect.
- **What's actually true:** Pathological FOV/near/far inputs are caught only if the caller explicitly invokes `FreeFly::sanitize()` ([`freefly.rs:127-134`](../../astraweave-camera/src/freefly.rs)); `sanitize()` does **not** run automatically at projection time (the projection path is hot and pathological inputs are rare — `freefly.rs:96-103`).

- **Trap:** Assuming `position` in a camera-relative `RenderView` is zero.
- **Why it's confusing:** the *view matrix* has translation stripped (eye at origin), but `RenderView.position` still reports the **original world position** ([`freefly.rs:89-94`](../../astraweave-camera/src/freefly.rs); test `to_render_view_camera_relative_position_field_preserved` at `freefly.rs:651-672`).
- **What's actually true:** only the matrices are camera-relative; `position` is the world position for fog/reconstruction.

- **Trap:** Expecting the FreeFly view matrix to NaN at exactly pitch = ±π/2.
- **Why it's confusing:** a direction-based `look_to_rh` with forward exactly along ±Y would be degenerate, but `cos(FRAC_PI_2 as f32) ≈ −4.37e-8`, so `dir(yaw, ±π/2)` retains a tiny xz residue and `look_to_rh` stays finite (`render_parity_harness.rs:1596-1660`; `unified_camera_outcome_2026-06.md` §4, Pillar 13). The **target-based** `OrbitCamera` path *can* degenerate at the exact singularity — tested separately.

---

## 7. Decision Log

### Decision: `RenderView` is the sole upload contract; renderer consumes it exclusively
- **Date:** 2026-05-18 (C.1 convention; realized C.3.C)
- **Status:** Accepted
- **Context:** C.0 audit found dual `update_camera`/`update_camera_matrices` paths whose side effects (UBO, cached matrices, CSM cascades, water, sky, impostors) had to be kept aligned by hand (`CAMERA_CONVENTIONS.md` §2.9 reasoning).
- **Decision:** Every producer emits `RenderView`; `Renderer::update_view(&RenderView)` is the only upload entry point. The renderer's update path is **not** generic over `CameraProducer`.
- **Alternatives considered:** UE5-style carrying both `ViewMatrix` and `TranslatedViewMatrix` simultaneously per draw — rejected in favor of per-producer-call commitment (a producer needing both flavors emits two `RenderView`s) ([`render_view.rs:24-31`](../../astraweave-camera/src/render_view.rs)).
- **Consequences:** Single upload path; camera-relative vs world-relative becomes the producer's contract, invisible to consumers.

### Decision: +X forward at yaw=0, right-handed +Y up, wgpu [0,1] depth
- **Date:** 2026-05-18 (C.1)
- **Status:** Accepted
- **Context:** Three competing forward conventions existed (+X, −Z, −X). The `-Vec3::Y` up-vector bug had caused "chunk-aligned rectangular voids in terrain" (fixed in `df7649287`; tombstone at `freefly.rs:40-43`).
- **Decision:** Canonical `dir(yaw,pitch) = (cos·cos, sin, sin·cos)` so `dir(0,0) = Vec3::X`; view via `look_to_rh`/`look_at_rh` with `Vec3::Y` up; projection via `Mat4::perspective_rh` (never `perspective_rh_gl`).
- **Alternatives considered:** −Z forward (Bevy/glTF/Three.js) or −X (orbit-offset). Rejected: the dominant production-runtime convention (+X, from the `df7649287` fix) was chosen to define the workspace standard (`CAMERA_CONVENTIONS.md` §2.8 reasoning).
- **Consequences:** Non-canonical inputs (glTF −Z, orbit −X) must convert at their boundary; the bench mock and contract tests guard against regression.

### Decision: `sanitize()` is caller-invoked, not automatic at projection time
- **Date:** 2026-05-24 (C.6.F, per C.5 audit finding L.5.16)
- **Status:** Accepted
- **Context:** FreeFly lacked FOV/near-far validation; pathological inputs could produce degenerate matrices.
- **Decision:** Add `FreeFly::sanitize()` clamping `fovy ∈ [10°,170°]`, `znear > 0.0001`, `zfar > znear + 0.001`, `aspect ≥ 0.01`; callers invoke explicitly when input may be pathological (deserialization, user-modified state, fixtures). NOT run on the hot projection path ([`freefly.rs:96-134`](../../astraweave-camera/src/freefly.rs)).
- **Alternatives considered:** boundary clamping inside `proj_matrix()` (Shape B) — rejected because that path is hot and pathological inputs rare. Per-type invocation model: `CameraKey::sanitize()` (C.7.D) IS invoked defensively inside `apply_camera_key` because that conversion is a hot path every keyframe flows through (`unified_camera_outcome_2026-06.md` §6 Pillar 3).
- **Consequences:** Defensive validation is opt-in for FreeFly; the FOV clamp range `[10°,170°]` is harmonized with `CameraKey::sanitize` (C.7.D removed the unused tighter `is_typical_fov` 30°–120° query).

### Decision: TAA jitter and explicit basis-vector fields deferred from `RenderView`
- **Date:** 2026-05-18 (C.2)
- **Status:** Accepted (additive when needed)
- **Context:** No production TAA path exists; shaders can extract `right`/`up` from `inverse_view` columns.
- **Decision:** Omit `unjittered_*`/`previous_view_proj`/jitter fields (§2.7) and `right`/`up` fields (Decision 3); add additively if/when TAA lands or shader-side column extraction proves costly ([`render_view.rs:33-48`](../../astraweave-camera/src/render_view.rs)).
- **Alternatives considered:** carrying the full UE5-style view-uniform set up front — rejected as premature ([`CAMERA_CONVENTIONS.md` §2.7]).
- **Consequences:** `RenderView` stays minimal; adding TAA is a non-breaking additive change.

### Decision: Cinematics routed through `FreeFly` rather than an independent `RenderView` evaluator
- **Date:** 2026-06-01 (C.7 chapter)
- **Status:** Accepted (honest divergence from the C.0 plan)
- **Context:** C.0/§5 envisioned `CameraKey` evolving into a continuous `RenderView` evaluator, eliminating the `apply_camera_key` round-trip.
- **Decision:** Instead, `tick_cinematics(dt, &mut FreeFly)` dispatches keyframes to `apply_camera_key`, which sanitizes and builds a `FreeFly`; `FreeFly` produces `RenderView` via the canonical contract. `CameraKey.fov_deg` stays degrees in the serialized `Timeline`; degrees→radians happens at the apply boundary (`cam.fovy = k.fov_deg.to_radians()` at `renderer.rs:3444`).
- **Alternatives considered:** the evaluator-elimination path — not taken; the chapter *routed and hardened* the boundary instead (`CAMERA_CONVENTIONS.md` §3 #10 family; `unified_camera_outcome_2026-06.md` §2/§3.5).
- **Consequences:** No bespoke cinematics renderer API (§2.9 holds); producer-internal degrees convention is permitted because the `RenderView` boundary is canonical (§2.5/§2.6).

---

## 8. Known Invariants

These are the parity-critical coordinate conventions. Authoritative source: [`CAMERA_CONVENTIONS.md`](../current/CAMERA_CONVENTIONS.md) §2. **Discrepancies between code and that doc are bugs in the code.**

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `fovy` stores **radians** (§2.1); degrees convert at boundaries only (`OrbitCamera::set_fov`, `CameraKey.fov_deg`→`to_radians()`). | Yes | `camera_conventions.rs::fovy_stores_radians`; `freefly.rs` tests; doc-only for boundaries |
| 2 | **Right-handed, +Y up** (§2.4). View matrices use `Vec3::Y` (never `-Vec3::Y`). | Yes | `camera_conventions.rs::up_vector_is_positive_y` + negative test `negative_y_up_produces_different_view` |
| 3 | View matrix built via `Mat4::look_to_rh(pos, dir, up)` (FreeFly) or `look_at_rh(eye, target, up)` (OrbitCamera) — equivalent for matching inputs (§2.5). | Yes | `camera_conventions.rs::look_to_and_look_at_styles_equivalent` |
| 4 | Projection via `Mat4::perspective_rh` → **wgpu [0,1] depth, forward-Z** (§2.2/§2.6). Never `perspective_rh_gl` ([-1,1]). | Yes | `camera_conventions.rs::near_far_use_wgpu_zero_to_one_depth`; `projection.rs:79` |
| 5 | At yaw=0, pitch=0, forward = `Vec3::X` (+X) (§2.8). `dir(yaw,pitch) = (cos·cos, sin, sin·cos).normalize()`. | Yes | `camera_conventions.rs::yaw_zero_pitch_zero_forward_is_positive_x`; `freefly.rs::to_render_view_yaw_zero_pitch_zero_has_x_forward` |
| 6 | `aspect` is floored at `.max(0.01)` at matrix construction; the stored `Projection.aspect` field preserves the **pre-floor** value (§2.3). | Yes | `camera_conventions.rs::aspect_floored_at_projection`; `projection.rs:79` |
| 7 | `RenderView.view_dir == -inverse_view.col(2).xyz`; `RenderView.position == inverse_view.col(3).xyz` (or camera-relative world position). | Partial | `render_view.rs:118-125` doc-comment; `freefly.rs` producer tests (doc-enforced caller contract) |
| 8 | `znear > 0`, `zfar > znear` (§2.2). | Debug only | `debug_assert!` in `projection.rs:68-78` (silent in release; `FreeFly::sanitize()` is the opt-in release-time guard) |
| 9 | Camera-relative `RenderView` strips translation from `view` but `position` reports the world position. | Yes | `freefly.rs::view_matrix_camera_relative_strips_translation`, `::to_render_view_camera_relative_position_field_preserved` |
| 10 | The renderer consumes `RenderView` exclusively; no per-producer-type renderer API (§2.9). | No | doc-only / structural (parity harness GPU SHA test) |

---

## 9. Performance & Resource Profile

### Hot paths
- `FreeFly::to_render_view` / `view_matrix` / `proj_matrix`: runs ~once per frame per active camera. Cheap (a few `Mat4` ops + two matrix inverses in `RenderView::new`). The inverses are computed once on CPU so shaders read precomputed `inverse_view`/`inverse_view_proj` ([`render_view.rs:126-143`](../../astraweave-camera/src/render_view.rs)).
- `apply_camera_key` (cinematics): runs per keyframe-event during a cutscene; clone + `sanitize()` is intentionally on this path (degenerate inputs every keyframe must be hardened — `unified_camera_outcome_2026-06.md` §6 Pillar 3).

### Cold paths
- `FreeFly::sanitize()`: caller-invoked at deserialization / fixture-construction boundaries, not per-frame. Looser budget.
- `CameraController` input methods: per-input-event; trivial.

### Resource ownership
- `FreeFly` / `OrbitCamera`: owned by the application/editor; borrowed `&mut` by `CameraController` and `tick_cinematics`. No GPU resources — `astraweave-camera` has no `wgpu` dependency ([`lib.rs:29-32`](../../astraweave-camera/src/lib.rs)).
- Camera UBO / CSM resources: owned by `Renderer`, written by `update_view`.

---

## 10. Testing & Validation

- **Unit tests:** [`astraweave-camera/src/freefly.rs:402-771`](../../astraweave-camera/src/freefly.rs) — basic matrix/dir generation, controller movement/zoom/mode-toggle/orbit, camera-relative translation stripping, `CameraProducer` impl (yaw0/pitch0 = +X), and the six `sanitize()` contract tests (C.6.F).
- **Contract tests:** [`astraweave-render/tests/camera_conventions.rs`](../../astraweave-render/tests/camera_conventions.rs) — 8 tests asserting the production camera obeys §2 (radians, [0,1] depth, aspect floor, +Y up + negative discriminator, look_to/look_at equivalence, +X forward, bench-mock up-vector). Run via `cargo test --tests -p astraweave-render camera_conventions`.
- **Producer conformance:** [`tools/aw_editor/tests/orbit_camera_producer.rs`](../../tools/aw_editor/tests/orbit_camera_producer.rs) (10/10 after C.4.B) and [`picking_consistency.rs`](../../tools/aw_editor/tests/picking_consistency.rs) (2/2).
- **Parity / matrix fixtures:** [`tools/aw_editor/tests/render_parity_harness.rs`](../../tools/aw_editor/tests/render_parity_harness.rs) — the GPU SHA-256 `editor_engine_render_parity` test plus 16 C.8 RenderView/matrix fixtures across four families (extreme pitch, non-square aspect, large world positions, cinematics-driven). Baselines are **independently re-derived** from camera math (`manual_look_to_rh_oracle`, f64 `DMat4` references) — never SHA of GPU output (anti-fabrication discipline; `unified_camera_outcome_2026-06.md` §4).
- **Mutation testing:** Not specifically recorded for this crate. [NEEDS VERIFICATION] (Verified absent: no `.cargo-mutants.toml` exists for `astraweave-camera` or at the workspace root as of `7c29b8182`. The marker is preserved because absence of config does not rule out ad-hoc `cargo mutants` runs recorded elsewhere; a coverage-report or CI citation would resolve it.)
- **Miri validation:** Not applicable — no `unsafe` in `astraweave-camera` (pure glam math). (Verified — `rg unsafe astraweave-camera/src/` returns zero matches at `7c29b8182`; the crate is `glam`-only math with no `unsafe` blocks.)
- **Manual validation:** the editor viewport (`OrbitCamera`) and `hello_companion`/`unified_showcase` examples exercise the producers at runtime.

---

## 11. Open Questions / Parked Decisions

- **`CameraMode` is `#[non_exhaustive]` with two variants (`FreeFly`, `Orbit`) ([`freefly.rs:153-158`](../../astraweave-camera/src/freefly.rs)).** Is a third controller mode (e.g. follow/cinematic movement) planned, or is `#[non_exhaustive]` purely forward-compat hygiene? Not recovered from available sources.
- **Multi-view producers (`to_render_views`).** The `CameraProducer` trait doc notes shadow cascades / cubemap faces / split-screen could be served by an additive `to_render_views(&self) -> Vec<RenderView>` but that it is "not currently in scope" and CSM derives subviews inside the renderer ([`producer.rs:33-41`](../../astraweave-camera/src/producer.rs)). When does a true multi-view producer arrive?
- **Cinematics GPU-gated fixtures.** The four C.8 cinematics fixtures are GPU-gated because `apply_camera_key` is private and `tick_cinematics` needs a live `Renderer`; a GPU-free CPU anchor would require exposing a `CameraKey → FreeFly` conversion on `astraweave-render`'s public surface (out of C.8 scope — `unified_camera_outcome_2026-06.md` §5.4). Should that conversion be exposed to enable CPU-only cinematics-path testing?
- **Orthographic projection.** `Projection` is deliberately enum-less; orthographic is deferred until a use case lands (additive promotion to an enum — `projection.rs:23-27`). No current consumer needs it.
- **Editor `CinematicsPanel` / `astraweave-ui` Simple Cinematics panel.** Documented as deliberate deferrals (L.7.4 / L.7.5) — the editor panel is UI-state-only and not wired to a live renderer preview (`unified_camera_outcome_2026-06.md` §5.2/§5.3). Not a camera-crate concern but adjacent to the cinematics camera path.

---

## 12. Maintenance Notes

**Update this doc when:**
- Any Active file in §5 changes (especially `freefly.rs`, `render_view.rs`, `projection.rs`, `producer.rs`, editor `camera.rs`).
- A convention in [`CAMERA_CONVENTIONS.md`](../current/CAMERA_CONVENTIONS.md) §2 is revised (those conventions are the source of truth for §8 here — keep them in sync).
- A new `CameraProducer` implementation is added (it becomes a third producer; update §2/§5/§3).
- The cinematics bridge (`tick_cinematics`/`apply_camera_key`) or the `Renderer::update_view` upload contract changes.

**Verification process:**
- Run `cargo test --tests -p astraweave-render camera_conventions` and `cargo test -p astraweave-camera --lib`; spot-check the §2 pipeline against `freefly.rs` + `render_view.rs` + `projection.rs`.
- Confirm `CAMERA_CONVENTIONS.md` §3 migration table is still all-closed (it was after C.7).
- Stamp the new commit hash and date in the Metadata table after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. **Coordinate conventions are parity-critical** (§8): radians `fovy`, right-handed +Y up, `perspective_rh` ([0,1] depth), +X forward at yaw=0. The contract tests in `camera_conventions.rs` and the matrix fixtures in `render_parity_harness.rs` will catch drift — but read `CAMERA_CONVENTIONS.md` §2 *first*. Discrepancies are bugs in the code, not the doc.
2. **Two producers, one contract:** `FreeFly` (engine, `astraweave-camera`) and `OrbitCamera` (editor, `tools/aw_editor`) both emit `RenderView`; the renderer only ever takes `&RenderView` via `update_view`. Don't add a per-producer renderer API.
3. **`sanitize()` is opt-in** for `FreeFly`; near/far asserts are debug-only. If you ingest untrusted camera state, call `sanitize()`.

**Files you'll most likely touch:**
- [`astraweave-camera/src/freefly.rs`](../../astraweave-camera/src/freefly.rs)
- [`astraweave-camera/src/render_view.rs`](../../astraweave-camera/src/render_view.rs)
- [`astraweave-camera/src/projection.rs`](../../astraweave-camera/src/projection.rs)

**Files you should NOT touch without strong reason:**
- [`tools/aw_editor/tests/render_parity_harness.rs`](../../tools/aw_editor/tests/render_parity_harness.rs) — the standing regression guard; fixture baselines are independently derived and must stay anti-fabricated (no SHA-of-GPU-output baselines).
- [`astraweave-render/tests/camera_conventions.rs`](../../astraweave-render/tests/camera_conventions.rs) — a test failure here is a convention violation, not a test to relax (it's a design call, not an autonomous relaxation — `CAMERA_CONVENTIONS.md` §4).

**Common mistakes when changing this system:**
- **Using `Mat4::perspective_rh_gl`** (OpenGL [-1,1] depth): silently breaks shadow mapping, depth picking, CSM. Always `perspective_rh` (§2.6).
- **Passing degrees where `fovy` (radians) is expected:** a literal `45.0` becomes ~2580° FOV and `perspective_rh` accepts it silently, producing a degenerate matrix (§2.1 reasoning). Convert at boundaries only.
- **Assuming `RenderView.position` is zero in camera-relative mode:** only the matrices are camera-relative; `position` is the world position.
- **Confusing `CameraController`'s `CameraMode::Orbit` with the editor `OrbitCamera`:** unrelated implementations sharing a word (§3 Naming collisions).

---

## Appendix B: Historical context

This system is the product of the **Unified Camera campaign** (C.0→C.9, 2026-05-18 → 2026-06-01). The C.0 audit inventoried 8 active 3D camera codepaths with three competing yaw=0 forward conventions, dual renderer upload paths, an editor `OrbitCamera` storing FOV in degrees under a non-canonical name, and three parallel cinematics camera systems. C.1 locked the conventions and contract tests; C.2 created `astraweave-camera`; C.3 migrated the engine `Camera`→`FreeFly` and consolidated the renderer to `update_view(&RenderView)` (deleting the dual upload paths and the `camera.rs` shim in C.3.C); C.4/C.4.B migrated the editor `OrbitCamera` to the producer contract and renamed its `fov`(deg)→`fovy`(rad); C.5/C.6 audited and migrated the gizmo and per-example cameras (deleting the dormant gizmo `CameraController`); C.7 consolidated the three cinematics systems into the single `CameraKey` path routed through `FreeFly`; and C.8 expanded the parity harness with 16 matrix fixtures. The full immutable record is [`docs/audits/unified_camera_outcome_2026-06.md`](../audits/unified_camera_outcome_2026-06.md); the canonical convention reference is [`docs/current/CAMERA_CONVENTIONS.md`](../current/CAMERA_CONVENTIONS.md).

A note on the prompt's framing: `astraweave-camera/src/producer.rs` is the **`CameraProducer` trait definition**, not a "parity-harness producer." The parity harness is `tools/aw_editor/tests/render_parity_harness.rs`, which *uses* `FreeFly`/`OrbitCamera`/`RenderView` as test fixtures. And while only two non-example *crates* depend on `astraweave-camera` (`astraweave-render`, `tools/aw_editor`), the crate is also a direct dependency of 13 example crates (verified at `7c29b8182`: `weaving_playground`, `visual_3d`, `veilweaver_demo`, `ui_controls_demo`, `debug_toolkit_demo`, `biome_showcase`, `cutscene_render_demo`, `audio_spatial_demo`, `navmesh_demo`, `hello_companion` (optional/feature-gated), `npc_town_demo`, `physics_demo3d`, `renderer_integration_test`).
