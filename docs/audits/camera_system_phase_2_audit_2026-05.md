# Camera System Phase 2 Audit (Sub-phase C.5)

| Field | Value |
|-------|-------|
| Date | 2026-05-24 |
| Scope | Read-only static source analysis. No source files modified. |
| Sub-phase | C.5 (Unified Camera campaign — Phase 2 audit) |
| Companion to | `camera_system_architecture_audit_2026-05.md` (C.0; pre-campaign state) |
| Audits | Mid-migration state post-renderer-consolidation (C.3.D) and post-editor-first-pass (C.4.B) |
| Forward chain | Planning round → C.6.A migration draft |
| Doc artifact authorization | C.5 launch prompt |

## 0. Why this audit exists

C.0 audited the pre-campaign state across 8 active 3D camera codepaths plus cinematics keyframes and identified the consolidation path Andrew gated at "Unified Camera crate (largest)." The campaign then executed:

- C.1 (conventions lockdown + contract tests + bench mock fix)
- C.2 (`astraweave-camera` crate with canonical types and `CameraProducer` trait)
- C.3.A–C.3.E (renderer-side migration: `Renderer::update_view` canonical upload; deprecated wrappers deleted; engine `Camera` → `FreeFly`; documentation closeout)
- C.4 + C.4.B (editor-side first-pass: `OrbitCamera` implements `CameraProducer`; picking-VP precision fix; `fov: degrees` → `fovy: radians` field rename)

The renderer-side and editor-side first-pass chapters are structurally closed. **C.5 audits what remains.** Per the campaign roadmap, the remaining migration surface covers (a) the gizmo `CameraController` (a third orbit-camera-class implementation that C.0 surfaced as new vs the April-28 SOTA audit), (b) the four per-example bespoke cameras (`unified_showcase`, `shadow_csm_demo`, `fluids_demo`, `nanite_demo`), and (c) cinematics camera state (inventory only, no migration proposal — that's C.7's planning round).

This document is the read-only deliverable for the C.6 planning round. **No decisions are locked here**; per Decision 3's α from the C.5 planning round, the planning round happens after C.5 closes and refines the migration queue with per-target decisions.

C.5 also serves as a mid-campaign **falsification opportunity**. C.0 identified 6 medium-confidence latent issues and 4 low-confidence concerns. Some of those concerns may have been resolved by subsequent sub-phases (e.g., C.4 reframed the picking-VP "bug" as a float-precision concern, then closed it). Others may still be open. Others may have been wrong from the start. C.5's Phase 2 inspection re-checks each — using the same empirical-verification discipline that anchors the audit (Decision 5's α).

## 1. Phase 2 — Camera surface inventory (post-campaign state)

### 1.A In-scope migration targets (per C.5 Decision 1's Medium scope)

| # | Implementation | File:line | Status |
|---|---|---|---|
| T1 | Gizmo `CameraController` | `tools/aw_editor/src/gizmo/scene_viewport.rs:19-173` | Pre-migration. See §2.A. |
| T2 | `unified_showcase` bespoke camera | `examples/unified_showcase/src/main.rs:148-150, 2158-2185` | Pre-migration. See §2.B.1. |
| T3 | `shadow_csm_demo::Camera` | `examples/shadow_csm_demo/src/main.rs:74-115` | Pre-migration. See §2.B.2. |
| T4 | `fluids_demo::Camera` | `examples/fluids_demo/src/main.rs:26-46` | Pre-migration. See §2.B.3. |
| T5 | `nanite_demo` bare fields | `examples/nanite_demo/src/main.rs:23-30` | Pre-migration. **No production renderer.** See §2.B.4. |

### 1.B Cinematics camera state (inventory only per Decision 1's Medium scope; feeds C.7 planning)

| # | Implementation | File:line | Status |
|---|---|---|---|
| C1 | `astraweave_cinematics::CameraKey` | `astraweave-cinematics/src/lib.rs:237-307` | Canonical cinematics keyframe data. Consumed by `apply_camera_key`. |
| C2 | `astraweave_render::Renderer::apply_camera_key` | `astraweave-render/src/renderer.rs:3371-3381` | Private function; converts C1 → `&mut FreeFly`. Only callable via `tick_cinematics`. |
| C3 | `astraweave_render::Renderer::tick_cinematics` | `astraweave-render/src/renderer.rs:3407` | Public; **zero production callers** (only `astraweave-render/tests/coverage_booster_render.rs:137, 6763`). |
| C4 | Editor `CameraKeyframe` | `tools/aw_editor/src/panels/cinematics_panel.rs:218-224` | **Parallel UI-only type** with different field names (`time`/`position`/`fov`/`roll`) from C1. Never converted to/from C1. |
| C5 | `astraweave_gameplay::cutscenes::Cue::CameraTo` | `astraweave-gameplay/src/cutscenes.rs:7-12` | **Parallel cinematics camera path** distinct from C1/C4. Operates on yaw/pitch directly (no look_at). Used by `cutscene_render_demo` (the example wires Cue → FreeFly directly, not through C1). |

### 1.C Reference inventory (migration-complete, out of scope)

| Type | Location | Status |
|---|---|---|
| `FreeFly` (canonical engine producer) | `astraweave-camera/src/freefly.rs:27-95` | Migrated C.3.A. Implements `CameraProducer`. |
| `OrbitCamera` (canonical editor producer) | `tools/aw_editor/src/viewport/camera.rs:50-99` | Migrated C.4 + C.4.B. Implements `CameraProducer`. `fovy: f32` (radians) post-C.4.B. |
| `astraweave_camera::CameraController` (engine controller) | `astraweave-camera/src/freefly.rs:120-156` | Migrated C.3.A. Used by every production example crate that uses FreeFly. |
| `Renderer::update_view(&RenderView)` | `astraweave-render/src/renderer.rs:3988-4028` | Canonical (and only) camera-upload entry point. Verified by §1.D's grep: zero callers of the deleted `update_camera`/`update_camera_matrices` wrappers. |
| Bench mock `Camera` | `astraweave-render/benches/camera_primitives_instancing.rs:32-53` | C.1 Deliverable C fix landed (line 35: `Vec3::Y`, not `-Vec3::Y`). Verified empirically. Contract test `bench_mock_camera_uses_canonical_up_vector` anchors this. |

### 1.D Upload-path verification (empirical)

Every `renderer.update_view(...)` call site in production code verified via grep:

| Caller | File:line | Producer pattern |
|---|---|---|
| `examples/cutscene_render_demo/src/main.rs:160` | engine FreeFly via `to_render_view()` | Canonical |
| `examples/weaving_playground/src/main.rs:408` | engine FreeFly via `to_render_view()` | Canonical |
| `examples/visual_3d/src/main.rs:170, 233` | engine FreeFly via `to_render_view()` | Canonical |
| `examples/veilweaver_demo/src/visual_renderer.rs:826, 859` | engine FreeFly via `to_render_view()` | Canonical |
| `examples/ui_controls_demo/src/main.rs:239` | engine FreeFly via `to_render_view()` | Canonical |
| `examples/renderer_integration_test/src/main.rs:96` | engine FreeFly via `to_render_view()` | Canonical |
| `examples/physics_demo3d/src/main.rs:290` | engine FreeFly via `to_render_view()` | Canonical |
| `examples/npc_town_demo/src/main.rs:230` | engine FreeFly via `to_render_view()` | Canonical |
| `examples/navmesh_demo/src/main.rs:117, 169` | engine FreeFly via `to_render_view()` | Canonical |
| `examples/audio_spatial_demo/src/main.rs:178` | engine FreeFly via `to_render_view()` | Canonical |
| `examples/hello_companion/src/visual_demo.rs:1225, 1382` | engine FreeFly via `to_render_view()` | Canonical |
| `tools/aw_editor/src/viewport/engine_adapter.rs:795` | OrbitCamera via `to_render_view()` (post-C.4) | Canonical |
| `tools/aw_editor/tests/render_parity_harness.rs:524` | OrbitCamera-derived RenderView (test fixture) | Canonical |
| `astraweave-render/tests/coverage_booster_render.rs:434, 2297, 5085, 6884, 7131` | FreeFly via `to_render_view()` (test) | Canonical |
| `astraweave-render/examples/tonemap_probe.rs:311` | FreeFly via `to_render_view()` | Canonical |
| `astraweave-render/src/renderer_tests.rs:2205` | FreeFly via `to_render_view()` (test) | Canonical |
| `astraweave-render/src/renderer.rs:7510` | FreeFly via `to_render_view()` (renderer-internal test caller for `test_frustum_corners_ws`) | Canonical |

**Net**: every `Renderer::update_view` caller in the workspace produces its RenderView via a `CameraProducer` impl (FreeFly or OrbitCamera). Zero remaining producer-bypass paths. The renderer-side upload contract is structurally closed.

### 1.E `CameraProducer` impl inventory

Verified empirically via `grep "impl CameraProducer for"`:

| Impl | File:line |
|---|---|
| `impl CameraProducer for FreeFly` | `astraweave-camera/src/freefly.rs:97` |
| `impl CameraProducer for OrbitCamera` | `tools/aw_editor/src/viewport/camera.rs:877` |

Two producers; both migration-complete. No other producer impls exist in the workspace.

### 1.F Excluded (out of scope per Decision 1 / anti-drift)

- `examples/unified_showcase/src/main_temp.rs`, `main_backup*.rs`, `main_bevy*.rs`, `main_clean.rs` — present in `src/` but not declared as `[[bin]]` paths (per C.0 §1.I). Dead alternative implementations.
- `docs/journey/archive/astraweave-render-bevy/src/extensions/nanite.rs:206` — archived.
- WGSL `struct Camera { view_proj: mat4x4<f32> }` strings inside Rust source — shader interface declarations, not Rust camera types.
- 11 GPU receivers (`nanite_render::update_camera`, `impostor_pass::update_camera`, etc., audit inventory #11–19 in C.0) — these consume pre-computed matrices; not independent cameras. Outside Decision 1's scope.

## 2. Per-target findings

### 2.A Gizmo `CameraController` (T1)

**Type purpose and current shape**:
A camera controller for the editor's gizmo subsystem viewport. Distinct from `astraweave_camera::CameraController` (the engine's free-fly + orbit controller used by FreeFly). The gizmo controller exposes orbit/pan/zoom + view-preset methods (front/right/top/perspective).

**Field-level analysis** (`scene_viewport.rs:19-34`):

| Field | Type | Unit | Convention |
|---|---|---|---|
| `position` | `Vec3` | world | — |
| `target` | `Vec3` | world | look-at point |
| `up` | `Vec3` | world | mutable; `set_view_top` changes to `Vec3::NEG_Z` (line 163) |
| `fov` | `f32` | **radians** (per docstring line 26 and default `FRAC_PI_4` line 42) | matches §2.1 in unit; field name `fov` (not `fovy`) doesn't match the post-C.4.B field-name discipline |
| `aspect` | `f32` | ratio | default `16.0/9.0` line 43; no `.max(0.01)` guard at projection time (line 58) |
| `near` | `f32` | meters | default `0.1` line 44 |
| `far` | `f32` | meters | default `1000.0` line 45 |

**API surface**: `view_matrix()`, `projection_matrix()`, `view_projection_matrix()`, `inverse_view_projection_matrix()`, `orbit(delta, sensitivity)`, `pan(delta, sensitivity)`, `zoom(delta, sensitivity)`, `distance()`, `focus_on(position)`, `set_view_front/right/top/perspective`. No `CameraProducer` impl, no `to_render_view()` method.

**Convention compliance** (vs `CAMERA_CONVENTIONS.md`):

| Axis | Compliance |
|---|---|
| §2.1 FOV unit | **Compliant in unit** (radians) but **field name `fov`** is non-canonical (should be `fovy` per post-C.4.B discipline) |
| §2.2 Depth range | Compliant (wgpu `[0,1]` via `perspective_rh`) |
| §2.3 Aspect guard | **Non-compliant** — no `.max(0.01)` at projection time (line 58); `set_aspect_ratio` (line 245) divides without `height > 0` guard |
| §2.4 Coordinate handedness | **Mostly compliant** (RH, default `Vec3::Y` up) but `set_view_top` switches up vector to `Vec3::NEG_Z` (line 163); field semantics change during use |
| §2.5 View matrix style | Compliant (`look_at_rh`) |
| §2.6 Projection | Compliant (`perspective_rh`) |
| §2.8 Yaw=0 forward direction | **N/A** — gizmo controller doesn't use yaw/pitch parameters; uses position+target quaternion orbit |
| §2.9 Upload contract | **Non-compliant** — no `CameraProducer` impl |

**`CameraProducer` trait status**: not implemented. The gizmo controller exposes `view_matrix()`, `projection_matrix()`, `position`, but no `to_render_view()`.

**Upload path**: matrices produced by `view_matrix()` and `projection_matrix()`. **No production code currently consumes them.** See production-wiring analysis below.

**Production wiring analysis** (load-bearing finding for the planning round):

Verified empirically via grep `SceneViewport::new()` and `SceneViewport::default()`:
- Zero production callers. The only matches are inside `tools/aw_editor/src/gizmo/scene_viewport.rs` itself (the type's tests and impl), `tools/aw_editor/benches/gizmo_benchmarks.rs:403-424` (benches), and the type's own `#[cfg(test)]` module (lines 553-598).

Verified empirically via grep `self.camera` in `tools/aw_editor/src/panels/transform_panel.rs`:
- `tools/aw_editor/src/panels/transform_panel.rs:51` declares `camera: CameraController` field. **`self.camera.*` is referenced zero times in `transform_panel.rs`.** The field is initialized at line 90 with `CameraController::default()` and never read by any method on `TransformPanel`.

**TransformPanel does have a production caller** (`tools/aw_editor/src/main.rs:508`), so the `CameraController` *instance* is created in production, but it is never *used* in production — its matrices are not produced, its orbit/pan/zoom methods are not invoked from any production code path.

**Verdict**: gizmo `CameraController` is **structurally instantiated, functionally dormant**. The gizmo subsystem's `SceneViewport` has no production caller; the `TransformPanel` holds a dormant field. This is consistent with the broader pattern documented in `CLAUDE.md` "wired beats tested" — a subsystem with tests and zero production usage is dormant code, not a feature.

**Latent issues** (C.0 mid-confidence findings re-verified):

- **Gimbal-lock NaN at top/bottom**: C.0 medium-confidence. **Reaffirmed.** Lines 76-92 perform quaternion orbit with `right = offset.cross(self.up).normalize()`. At `set_view_top` followed by subsequent orbit, `offset` becomes parallel to `up = Vec3::NEG_Z`, making `cross` produce a zero vector that `normalize` returns as NaN. No clamping. Production triggering blocked by dormancy.
- **`up` field semantics changing during `set_view_top`**: C.0 low-confidence. **Reaffirmed.** Line 163 mutates `self.up` to `Vec3::NEG_Z`. Subsequent orbit/pan operations behave differently than the default-up case. Latent foot-gun.
- **Aspect-zero guard absent at projection**: C.0 finding implicit (axis 3 — gizmo had no NaN guard). **Reaffirmed.** Line 58 calls `Mat4::perspective_rh(self.fov, self.aspect, ...)` with no `.max(0.01)` clamp. `set_aspect_ratio` at line 245 divides `width / height` without `height > 0` guard.

**Migration considerations for C.6** (suggested; planning round locks):

- Two structurally distinct paths the planning round could choose between:
  - **(α) Migrate**: implement `CameraProducer` on gizmo `CameraController`; align field name (`fov` → `fovy` for radians); add aspect-zero guard; gimbal-lock fix; remove `up` mutation in `set_view_top`.
  - **(β) Delete**: remove gizmo `CameraController` (and its `SceneViewport` container) entirely from production code, replacing `TransformPanel`'s dormant field with `OrbitCamera` if any future production use case wires a viewport into TransformPanel. The function would survive in tests/benches if needed; the production declaration would disappear.
- The dormancy finding is the strong indicator: a feature with zero production usage is more cheaply removed than maintained. But the planning round may have context the audit doesn't (e.g., near-term plans to wire `SceneViewport` into TransformPanel) — that's why decision-locking belongs to the planning round.
- If **(α)**, suggested closure proof: `CameraProducer` contract test analogous to `orbit_camera_producer.rs`; integration test verifying the gizmo's view matrix matches FreeFly's for an equivalent state.
- If **(β)**, suggested closure proof: structural-deletion verification (grep returns zero `gizmo::CameraController` references in production code).
- Estimated touch surface: (α) ~3-4 files (scene_viewport.rs + transform_panel.rs + tests + bench updates); (β) ~3-4 files (scene_viewport.rs removed or trimmed + transform_panel.rs field removed + tests/benches kept as standalone or deleted).

### 2.B Per-example cameras

#### 2.B.1 `unified_showcase` (T2)

**Type purpose and current shape**: bespoke flagship-example camera bound to a bespoke wgpu renderer pipeline. Operates entirely outside `astraweave-render::Renderer`. Has its own `CameraUniforms` struct (`examples/unified_showcase/src/main.rs:75-86`).

**Field-level analysis** (lines 148-150):

| Field | Type | Convention |
|---|---|---|
| `camera_pos` | `Vec3` | world |
| `camera_yaw` | `f32` | radians; **unconstrained** |
| `camera_pitch` | `f32` | radians; **unconstrained** |

No `Camera` struct — bare fields on the `ShowcaseApp` struct.

**View construction** (lines 2159-2166):
```rust
Mat4::look_at_rh(
    self.camera_pos,
    self.camera_pos
        + Quat::from_rotation_y(self.camera_yaw)
            * Quat::from_rotation_x(self.camera_pitch)
            * Vec3::NEG_Z,
    Vec3::Y,
)
```

At yaw=0, pitch=0: forward = `Quat::IDENTITY * Vec3::NEG_Z` = `Vec3::NEG_Z` = `(0, 0, -1)` — **-Z forward** (Bevy/glTF/Three.js convention; diverges from `CAMERA_CONVENTIONS.md` §2.8's canonical +X forward).

**Projection** (lines 2168-2173):
```rust
Mat4::perspective_rh(
    45.0_f32.to_radians(),
    self.config.width as f32 / self.config.height as f32,
    0.1, 2000.0,
)
```
- FOV: hardcoded `45.0_f32.to_radians()` at the projection site. No field, no UI control, no `Camera` struct.
- Aspect: no `.max(0.01)` guard. Resize handler at line 2389 does guard `width > 0 && height > 0`, so divide-by-zero is prevented at boundary.

**Pitch clamping status** (C.0 medium-confidence finding):

**CONFIRMED missing.** Lines 2380-2386:
```rust
fn handle_mouse_motion(&mut self, delta: (f64, f64)) {
    if self.mouse_pressed {
        let sensitivity = 0.005;
        self.camera_yaw += (delta.0 as f32) * sensitivity;
        self.camera_pitch += (delta.1 as f32) * sensitivity;
    }
}
```

No `.clamp(...)` on `camera_pitch`. Empirical re-verification with grep `camera_pitch.*clamp` returns zero matches in `unified_showcase/src/main.rs`. Pitch is unbounded; the quaternion-based rotation at line 2163 doesn't produce a degenerate matrix the way `Mat4::look_to_rh(pos, dir, +Y)` would (no gimbal-class matrix singularity), but the camera can flip over at extreme pitch values, producing a disorienting user experience.

**`CameraProducer` trait status**: not implemented (no Camera struct to implement on; bare fields).

**Upload path**: bespoke. The example doesn't use `astraweave-render::Renderer`; instead it writes directly to its own `camera_buffer` via `queue.write_buffer(...)` at lines 2175-2183 with a `CameraUniforms { view_proj, camera_pos, _padding }` struct.

**Migration considerations**:
- The example operates outside the engine's authoritative pipeline. Migration paths the planning round could choose between:
  - **(α) Migrate to engine**: replace bespoke camera + bespoke renderer with FreeFly + `astraweave-render::Renderer`. Large scope: the example is structurally bespoke-renderer-built (cube-grid scene, custom wgpu pipelines for grid/sky/etc.), not just bespoke-camera-built.
  - **(β) Formalize as separate sandbox**: keep bespoke; add a comment in the source acknowledging the divergence and the rationale (flagship experimental sandbox, not subject to canonical conventions); fix the pitch-clamp latent issue locally.
  - **(γ) Targeted fix only**: add the pitch clamp; defer broader migration.
- C.0's §4.D Pair-D analysis identified this as "mostly accretion" but acknowledged unified_showcase's "flagship experimental sandbox" role. Decision belongs to the planning round.

#### 2.B.2 `shadow_csm_demo::Camera` (T3)

**Type purpose**: shadow CSM example. Own `Camera` struct with debug-mode fields for visualizing shadow cascades.

**Field-level analysis** (lines 74-81):

| Field | Type | Unit | Notes |
|---|---|---|---|
| `position` | `Vec3` | world | — |
| `yaw` | `f32` | radians | default `-90.0_f32.to_radians()` line 87 |
| `pitch` | `f32` | radians | default `-20.0_f32.to_radians()` line 88 |
| `fov` | `f32` | radians | default `60.0_f32.to_radians()` line 89 |
| `near` | `f32` | meters | default `0.1` line 90 |
| `far` | `f32` | meters | default `100.0` line 91 |

**View construction** (lines 108-109):
```rust
Mat4::look_at_rh(self.position, self.position + self.forward(), Vec3::Y)
```

where `forward()` at lines 95-102 is the spherical formula:
```rust
Vec3::new(
    self.yaw.cos() * self.pitch.cos(),
    self.pitch.sin(),
    self.yaw.sin() * self.pitch.cos(),
).normalize()
```

At yaw=0, pitch=0: forward = `(1, 0, 0)` = **+X forward** (matches canonical §2.8).

**Pitch clamping status** (C.0 medium-confidence finding):

**FALSIFIED.** Lines 589-593:
```rust
self.camera.pitch -= self.mouse_delta.1 * sensitivity;
self.camera.pitch = self
    .camera
    .pitch
    .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
```

The clamp is present at the input handler. C.0's "likely missing pitch clamp" finding for shadow_csm_demo was incorrect; the clamp exists at lines 591-593.

**Aspect handling**: line 598 computes `width / height` without `.max(0.01)`. Resize handler at line 549 guards `width > 0 && height > 0` before resize. Divide-by-zero prevented at boundary, but if `proj_matrix(aspect)` is called with `aspect = 0.0` from another path, NaN would propagate. Latent.

**`CameraProducer` trait status**: not implemented.

**Upload path**: bespoke wgpu pipeline. Does not use `astraweave-render::Renderer`.

**Convention compliance**:

| Axis | Compliance |
|---|---|
| §2.1 FOV unit/name | **Mostly compliant** (radians) but field name `fov` (not `fovy`) — non-canonical |
| §2.2 Depth | Compliant |
| §2.3 Aspect guard | **Non-compliant** at projection time; partial at resize handler |
| §2.4 Handedness | Compliant (RH, +Y up) |
| §2.5 View style | `look_at_rh(pos, pos+forward, +Y)` — pos+forward target style; equivalent to `look_to_rh(pos, forward, +Y)` |
| §2.6 Projection | Compliant |
| §2.8 Yaw=0 forward | **Compliant** (+X forward) |
| §2.9 Upload contract | **Non-compliant** (no `CameraProducer` impl; bespoke pipeline) |

**Migration considerations**: similar to T2 — bespoke pipeline. Migration to `astraweave-render::Renderer` would be a significant restructuring of the example's rendering setup. Targeted fixes (rename `fov` → `fovy`; aspect guard) are cheaper.

#### 2.B.3 `fluids_demo::Camera` (T4)

**Type purpose**: fluid SPH demo example. Own `Camera` struct with fluid-specific `CameraUniform` (from `astraweave_fluids::renderer`).

**Field-level analysis** (lines 26-34):

| Field | Type | Unit | Notes |
|---|---|---|---|
| `eye` | `Vec3` | world | — |
| `target` | `Vec3` | world | look-at point |
| `up` | `Vec3` | world | — |
| `aspect` | `f32` | ratio | — |
| `fovy` | `f32` | **degrees** | field name matches §2.1 but unit doesn't (line 39 converts at projection: `self.fovy.to_radians()`) |
| `znear` | `f32` | meters | — |
| `zfar` | `f32` | meters | — |

Default at line 345: `fovy: 45.0` (degrees).

**View / projection construction** (lines 37-45):
```rust
fn build_view_projection_matrix(&self) -> Mat4 {
    let view = self.build_view_matrix();
    let proj = Mat4::perspective_rh(self.fovy.to_radians(), self.aspect, self.znear, self.zfar);
    proj * view
}

fn build_view_matrix(&self) -> Mat4 {
    Mat4::look_at_rh(self.eye, self.target, self.up)
}
```

Orbit math is on `State` separately (lines 80, 507-509 for spherical position computation):
```rust
let x = self.camera_distance * self.camera_yaw.cos() * self.camera_pitch.cos();
let y = self.camera_distance * self.camera_pitch.sin();
let z = self.camera_distance * self.camera_yaw.sin() * self.camera_pitch.cos();
```

**Pitch clamping status** (C.0 medium-confidence finding):

**FALSIFIED.** Line 502:
```rust
self.camera_pitch = self.camera_pitch.clamp(-1.4, 1.4);
```

The clamp is present (±1.4 rad ≈ ±80°). C.0's "likely missing pitch clamp" finding for fluids_demo was incorrect.

**Aspect handling**: lines 344 and 460 compute `width / height` with no `.max(0.01)` and no `height > 0` guard. C.0 §3.6 finding **reaffirmed**.

**`CameraProducer` trait status**: not implemented.

**Upload path**: bespoke fluid-specific pipeline via `astraweave_fluids::renderer::CameraUniform`. Does not use `astraweave-render::Renderer`.

**Convention compliance**:

| Axis | Compliance |
|---|---|
| §2.1 FOV unit | **Non-compliant** (field name `fovy` matches but unit is degrees; conversion at boundary) |
| §2.2 Depth | Compliant |
| §2.3 Aspect guard | **Non-compliant** (no guard at resize or projection) |
| §2.4 Handedness | Compliant |
| §2.5 View style | Compliant (look_at) |
| §2.6 Projection | Compliant |
| §2.8 Yaw=0 forward | **N/A** — uses look-at target, not yaw=0 semantic |
| §2.9 Upload contract | **Non-compliant** (no `CameraProducer` impl; fluid-specific uniform layout) |

**Migration considerations**: fluid-specific `CameraUniform` has fields (`view_inv`, `inv_view_proj`, additional fluid-specific shader inputs) that `RenderView` lacks. Migration would require either extending `RenderView` or maintaining the fluid uniform separately. C.0 §4.D classified this as "specialization for fluid SPH rendering" — partly justified. Targeted fixes (field unit naming, aspect guard) are cheaper than full migration.

#### 2.B.4 `nanite_demo` bare fields (T5)

**Type purpose**: high-polygon-scene rendering example for Nanite-style meshlet pipeline.

**Field-level analysis** (lines 23-30):

| Field | Type | Unit |
|---|---|---|
| `camera_pos` | `Vec3` | world |
| `camera_yaw` | `f32` | radians |
| `camera_pitch` | `f32` | radians |
| `camera_speed` | `f32` | scalar |
| `mouse_sensitivity` | `f32` | scalar |
| `last_mouse_pos` | `Option<(f32, f32)>` | screen |

No `Camera` struct, no FOV field, no near/far fields, no aspect field.

**View construction** (lines 44-52):
```rust
fn get_view_matrix(&self) -> Mat4 {
    let forward = Vec3::new(
        self.camera_yaw.cos() * self.camera_pitch.cos(),
        self.camera_pitch.sin(),
        self.camera_yaw.sin() * self.camera_pitch.cos(),
    );
    let target = self.camera_pos + forward;
    Mat4::look_at_rh(self.camera_pos, target, Vec3::Y)
}
```

At yaw=0, pitch=0: forward = `(1, 0, 0)` = **+X forward** (matches canonical §2.8).

**Pitch clamping status** (C.0 medium-confidence finding):

**FALSIFIED.** Lines 121-124:
```rust
self.camera_pitch = self.camera_pitch.clamp(
    -std::f32::consts::FRAC_PI_2 + 0.1,
    std::f32::consts::FRAC_PI_2 - 0.1,
);
```

The clamp is present (±π/2 ± 0.1). C.0's "likely missing pitch clamp" finding for nanite_demo was incorrect.

**NEW FINDING — production rendering status**:

Empirical re-verification via grep `view_proj|perspective|update_view|update_camera|view_projection|Renderer` against `examples/nanite_demo/src/main.rs` returns zero matches. The example **does not construct a projection matrix and does not call any renderer**.

Lines 272-275 (the redraw handler):
```rust
Event::RedrawRequested(_) => {
    // In a full implementation, this would render the scene
    // For now, we just demonstrate the meshlet generation
}
```

The example exists to demonstrate meshlet hierarchy generation (`generate_lod_hierarchy` at line 208), not to render. The camera state is structurally present but **never used** — its `get_view_matrix()` method has no caller in the example. Empirical grep `get_view_matrix` returns zero matches outside the type's own impl.

**Verdict**: nanite_demo's camera state is **dormant code at the example level**. There's no rendering pipeline to migrate; there's no upload path to fix; the pitch clamp protects against a degenerate matrix that's never computed.

**`CameraProducer` trait status**: not implemented (no Camera struct; bare fields).

**Upload path**: none in production.

**Migration considerations**: the planning round may choose:
- **(α) Migrate to engine**: replace bare fields with FreeFly via the alias pattern; wire `Renderer::update_view` and an actual render pass. This is example completion work, not just migration.
- **(β) Mark as demo-only**: add a comment noting that the example's rendering is a stub; defer rendering to a future "nanite_demo: complete the render path" sub-phase outside the Unified Camera campaign.
- **(γ) Delete the camera state**: if the example's purpose is purely meshlet generation, the camera fields are unused state that could be removed.

Planning round decides per Decision 1's per-target principle.

### 2.C Cinematics camera state (inventory only per Decision 1's Medium scope)

This subsection is **informational** for C.7's eventual planning round. **No migration proposal is made**; see §5 for structured C.7-input format.

#### 2.C.1 `astraweave_cinematics::CameraKey` (C1)

**Location**: `astraweave-cinematics/src/lib.rs:237-307`.

**Shape**:
```rust
pub struct CameraKey {
    pub t: Time,
    pub pos: (f32, f32, f32),
    pub look_at: (f32, f32, f32),
    pub fov_deg: f32,
}
```

Tuple-based (not `glam::Vec3`). Linear `lerp` interpolation method (lines 291-306). Convenience methods `position()`, `fov_rad()` (line 282: `self.fov_deg.to_radians()`), `distance_to_target()`, `is_typical_fov()` (validates `30°..=120°`).

**Convention compliance**:
- FOV unit: **degrees** stored, with `fov_rad()` accessor converting at the read boundary.
- Field name: `fov_deg` — explicit about its unit (better than ambiguous `fov`).
- Look-direction model: explicit `look_at` (not yaw/pitch).
- Tuple storage instead of `Vec3` — likely a serialization-format choice.

#### 2.C.2 `astraweave_render::Renderer::apply_camera_key` (C2)

**Location**: `astraweave-render/src/renderer.rs:3371-3381`. Private function (no `pub`):

```rust
fn apply_camera_key(cam: &mut FreeFly, k: &awc::CameraKey) {
    let pos = glam::Vec3::new(k.pos.0, k.pos.1, k.pos.2);
    let look = glam::Vec3::new(k.look_at.0, k.look_at.1, k.look_at.2);
    let dir = (look - pos).normalize_or_zero();
    let yaw = dir.z.atan2(dir.x);
    let pitch = dir.y.clamp(-1.0, 1.0).asin();
    cam.position = pos;
    cam.yaw = yaw;
    cam.pitch = pitch;
    cam.fovy = k.fov_deg.to_radians();
}
```

Converts `CameraKey` → `FreeFly` fields. Inverts `FreeFly::dir(yaw, pitch)`. Edge case: `normalize_or_zero` returns `Vec3::ZERO` when `look == pos`; subsequent `atan2(0, 0) = 0` and `asin(0) = 0` produce a degenerate-but-non-NaN camera state pointing +X. `is_typical_fov()` validation is documentation-only — `apply_camera_key` does not invoke it (C.0 §3.9 finding).

#### 2.C.3 `astraweave_render::Renderer::tick_cinematics` (C3)

**Location**: `astraweave-render/src/renderer.rs:3407`. Public:

```rust
pub fn tick_cinematics(&mut self, dt: f32, camera: &mut FreeFly) -> Vec<awc::SequencerEvent> {
    // steps sequencer, applies camera keys via apply_camera_key, returns events
}
```

**Production caller status**: zero. Empirical grep `tick_cinematics` returns only test sites:
- `astraweave-render/tests/coverage_booster_render.rs:137, 6763` (test code)
- The type's own declaration at `renderer.rs:3407`.

No production code in any example or tool currently calls `tick_cinematics`. The cinematics integration through `Renderer` is structurally available but unused.

#### 2.C.4 Editor `CameraKeyframe` (C4)

**Location**: `tools/aw_editor/src/panels/cinematics_panel.rs:218-224`. **Parallel UI-only type** distinct from C1:

```rust
pub struct CameraKeyframe {
    pub time: f32,           // vs C1::t: Time
    pub position: (f32, f32, f32),    // vs C1::pos
    pub look_at: (f32, f32, f32),
    pub fov: f32,            // degrees per UI slider line 1063 ("10.0..=120.0").suffix("°")
    pub roll: f32,           // extra field; C1 has no roll
}
```

**Conversion to/from C1**: empirical grep finds no conversion function. The editor's `CameraKeyframe` is never converted to/from `astraweave_cinematics::CameraKey`. They are entirely parallel types.

**Editor usage**: `CinematicsPanel.camera_keyframes: Vec<CameraKeyframe>` (line 489). UI panel for editing keyframes with sliders. No production flow from this UI to a renderer; the editor's cinematics panel UI is UI-state-only.

**Separately**: `astraweave-ui/src/panels.rs:332-346` does use the canonical `awc::CameraKey` (not the editor's `CameraKeyframe`). The editor's cinematics_panel and astraweave-ui's panels are different code paths.

#### 2.C.5 `astraweave_gameplay::cutscenes::Cue::CameraTo` (C5)

**Location**: `astraweave-gameplay/src/cutscenes.rs:7-12`. **Third parallel cinematics camera path**:

```rust
pub enum Cue {
    CameraTo {
        pos: Vec3,
        yaw: f32,
        pitch: f32,
        time: f32,
    },
    Title { text: String, time: f32 },
    Wait { time: f32 },
}
```

Operates on yaw/pitch directly (no look_at). Used by `cutscene_render_demo` (`examples/cutscene_render_demo/src/main.rs:47-50`):

```rust
Cue::CameraTo {
    pos: vec3(0.0, 6.0, 12.0),
    yaw: -1.57,
    pitch: -0.35,
    time: ...
}
```

The example's `CutsceneState::tick()` returns `(Option<(Vec3, f32, f32)>, ...)` — a `(pos, yaw, pitch)` tuple — which the application code applies directly to the FreeFly camera's fields. No conversion to/from `CameraKey`; entirely separate cinematics implementation.

#### 2.C.6 Cinematics inventory summary

Three parallel cinematics camera systems coexist:

| System | Type | Look-direction model | Used by |
|---|---|---|---|
| C1 (canonical) | `CameraKey` (tuple-based) | `look_at` target | `cinematics_timeline_demo` (timeline construction only; doesn't render); `astraweave-ui/src/panels.rs:332` (loads demo timeline) |
| C4 (editor UI) | `CameraKeyframe` (struct, extra roll field) | `look_at` target | Editor cinematics panel UI only; no renderer flow |
| C5 (gameplay) | `Cue::CameraTo` | yaw/pitch direct | `cutscene_render_demo` (renders via FreeFly direct update) |

The canonical engine integration (C2 + C3) exists but has zero production callers. The active production cinematics path is C5 (gameplay-cutscenes), which doesn't go through `astraweave-cinematics`.

**These observations are for C.7 planning input only.** No migration proposal is made; see §5.

## 3. Latent issues catalog

Issue IDs prefixed `L.5.<index>`. Status calls per Decision 4 (suggested only; planning round locks).

### L.5.1 — Gizmo `CameraController` is functionally dormant

- **Description**: gizmo `CameraController` is instantiated in production via `TransformPanel.camera` (`transform_panel.rs:51, 90`) but the field is never read (`self.camera.*` matches: zero). `SceneViewport` (the type's main container) has zero production callers.
- **Location**: `tools/aw_editor/src/gizmo/scene_viewport.rs:19-173`; `tools/aw_editor/src/panels/transform_panel.rs:51, 90`.
- **Confidence**: **high** (verified by grep).
- **Type**: dormant code / wired-vs-tested anomaly.
- **Suggested resolution**: planning round decides per §2.A's (α)/(β) split.

### L.5.2 — Gizmo `CameraController` gimbal-lock NaN at top/bottom

- **Description**: lines 76-92 perform quaternion orbit via `right = offset.cross(self.up).normalize()`. After `set_view_top` (which sets `up = NEG_Z`), subsequent orbit operations with `offset` parallel to `up` produce zero-vector cross product, then NaN through normalize.
- **Location**: `tools/aw_editor/src/gizmo/scene_viewport.rs:85-91`.
- **Confidence**: **medium** (math-derived; not empirically observed because of L.5.1's dormancy).
- **Type**: precision / numerical correctness.
- **Suggested resolution**: fix during C.6.A if (α) chosen; vanishes with deletion if (β) chosen.

### L.5.3 — Gizmo `CameraController` `up` field mutation in `set_view_top`

- **Description**: line 163 mutates `self.up = Vec3::NEG_Z`. Subsequent operations behave differently than default-up; the field's semantics change during use.
- **Location**: `tools/aw_editor/src/gizmo/scene_viewport.rs:163`.
- **Confidence**: **low** (semantic foot-gun; no current bug because of L.5.1's dormancy).
- **Type**: API semantics / hidden state mutation.
- **Suggested resolution**: fix during C.6.A if (α); vanishes with (β).

### L.5.4 — Gizmo `CameraController` aspect-zero guard absent

- **Description**: `Mat4::perspective_rh(self.fov, self.aspect, ...)` at line 58 has no `.max(0.01)` clamp. `set_aspect_ratio` at line 245 divides `width / height` with no `height > 0` guard.
- **Location**: `tools/aw_editor/src/gizmo/scene_viewport.rs:58, 245-247`.
- **Confidence**: **medium** (potential NaN propagation; not currently triggered because of L.5.1).
- **Type**: convention compliance (§2.3).
- **Suggested resolution**: fix during C.6.A if (α); vanishes with (β).

### L.5.5 — `unified_showcase` pitch clamp missing (C.0 #23 confirmed)

- **Description**: `camera_pitch += delta` at line 2384 without clamping. Quaternion-based rotation at line 2163 doesn't produce a degenerate matrix, but the user-experience effect (camera flips over) is disorienting at extreme pitches.
- **Location**: `examples/unified_showcase/src/main.rs:2384`.
- **Confidence**: **high** (verified by grep — pitch clamp absent).
- **Type**: UX / convention compliance (§2.8 forward-direction implicit assumption that pitch is bounded).
- **Suggested resolution**: fix during C.6 (either as part of full migration or as targeted patch).

### L.5.6 — `unified_showcase` uses `-Z forward` convention (C.0 #23 confirmed)

- **Description**: `Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch) * Vec3::NEG_Z` produces `-Z forward` at yaw=0,pitch=0. Diverges from canonical §2.8's `+X forward`.
- **Location**: `examples/unified_showcase/src/main.rs:2162-2164`.
- **Confidence**: **high** (verified by inspection).
- **Type**: convention deviation.
- **Suggested resolution**: aligns or formalizes-as-sandbox per §2.B.1's (α)/(β)/(γ) options.

### L.5.7 — `unified_showcase` hardcoded FOV at projection site

- **Description**: `Mat4::perspective_rh(45.0_f32.to_radians(), ...)` at line 2169. No FOV field, no UI control. C.0 #23 reaffirmed.
- **Location**: `examples/unified_showcase/src/main.rs:2169`.
- **Confidence**: **high**.
- **Type**: convention / discoverability (hard to find FOV when there's no field).
- **Suggested resolution**: fix or formalize per §2.B.1's options.

### L.5.8 — `shadow_csm_demo::Camera` field name `fov` (not `fovy`)

- **Description**: field name `fov: f32` stores radians but doesn't follow the post-C.4.B field-name discipline (`fovy` for radians).
- **Location**: `examples/shadow_csm_demo/src/main.rs:78, 89`.
- **Confidence**: **high** (verified by inspection).
- **Type**: convention deviation (cosmetic; unit is correct).
- **Suggested resolution**: rename during C.6 migration.

### L.5.9 — `shadow_csm_demo` aspect-zero guard absent at projection time

- **Description**: line 598 computes `width / height` with no `.max(0.01)`. Resize handler at line 549 guards but other code paths could pass `aspect = 0.0`.
- **Location**: `examples/shadow_csm_demo/src/main.rs:598`.
- **Confidence**: **medium** (latent; not currently triggered).
- **Type**: convention compliance (§2.3).
- **Suggested resolution**: targeted fix during C.6.

### L.5.10 — C.0 #24 finding "shadow_csm_demo likely missing pitch clamp"

- **Description**: C.0 medium-confidence finding. **FALSIFIED** by Phase 2 re-inspection: clamp present at lines 589-593 (`±89° in radians`).
- **Location**: `examples/shadow_csm_demo/src/main.rs:589-593`.
- **Confidence**: **high** that the finding was wrong.
- **Type**: false-positive in C.0.
- **Suggested resolution**: no action (closes the C.0 finding).

### L.5.11 — `fluids_demo::Camera` field `fovy` stores degrees (C.0 #25 confirmed)

- **Description**: field name `fovy: f32` matches §2.1 but stores degrees; `self.fovy.to_radians()` at projection site (line 39). Mismatch between name and unit.
- **Location**: `examples/fluids_demo/src/main.rs:31, 39`.
- **Confidence**: **high**.
- **Type**: convention deviation.
- **Suggested resolution**: rename to a degrees-explicit name OR convert to radians-storage during C.6 migration.

### L.5.12 — `fluids_demo` aspect-zero guard absent (C.0 finding confirmed)

- **Description**: lines 344 and 460 compute `width / height` with no `.max(0.01)` and no `height > 0` guard.
- **Location**: `examples/fluids_demo/src/main.rs:344, 460`.
- **Confidence**: **high** (verified by inspection).
- **Type**: convention compliance (§2.3).
- **Suggested resolution**: targeted fix during C.6.

### L.5.13 — C.0 #25 finding "fluids_demo likely missing pitch clamp"

- **Description**: C.0 medium-confidence finding. **FALSIFIED** by Phase 2 re-inspection: clamp present at line 502 (`.clamp(-1.4, 1.4)`).
- **Location**: `examples/fluids_demo/src/main.rs:502`.
- **Confidence**: **high** that the finding was wrong.
- **Type**: false-positive in C.0.
- **Suggested resolution**: no action.

### L.5.14 — `nanite_demo` doesn't render

- **Description**: redraw handler (lines 272-275) is a stub comment. Camera state (`camera_pos`, `camera_yaw`, `camera_pitch`) exists but is never used for rendering. `get_view_matrix()` has zero callers.
- **Location**: `examples/nanite_demo/src/main.rs:23-30, 272-275`.
- **Confidence**: **high** (verified by grep).
- **Type**: incomplete example / dormant code.
- **Suggested resolution**: planning round decides per §2.B.4's (α)/(β)/(γ) options.

### L.5.15 — C.0 #26 finding "nanite_demo likely missing pitch clamp"

- **Description**: C.0 medium-confidence finding. **FALSIFIED** by Phase 2 re-inspection: clamp present at lines 121-124 (`±π/2 ± 0.1`).
- **Location**: `examples/nanite_demo/src/main.rs:121-124`.
- **Confidence**: **high** that the finding was wrong.
- **Type**: false-positive in C.0.
- **Suggested resolution**: no action.

### L.5.16 — Engine `FreeFly` missing FOV/near-far validation

- **Description**: C.0 §3.1 medium-confidence finding: `fovy` not clamped at projection; `znear >= zfar` not validated. **Reaffirmed** by inspection of `astraweave-camera/src/freefly.rs:47-49`:
  ```rust
  Mat4::perspective_rh(self.fovy, self.aspect.max(0.01), self.znear, self.zfar)
  ```
  `aspect.max(0.01)` guards aspect but `fovy` and `znear/zfar` are unvalidated. Pathological inputs (`fovy = 0`, `zfar <= znear`) produce degenerate projections silently.
- **Location**: `astraweave-camera/src/freefly.rs:48`.
- **Confidence**: **medium**.
- **Type**: convention compliance (§2.6) / missing validation at API boundary.
- **Suggested resolution**: planning round decides whether to add validation during C.6 (perhaps a dedicated micro-sub-phase) or defer to standalone follow-up. The fix would touch `astraweave-camera` (which has been stable through C.4.B); the planning round should weigh stability-of-crate vs convention-completeness.

### L.5.17 — `apply_camera_key` silent acceptance of degenerate keys (C.0 #10 reaffirmed)

- **Description**: `normalize_or_zero` returns `Vec3::ZERO` when `look_at == pos`; subsequent `atan2(0, 0) = 0` produces degenerate yaw. `is_typical_fov()` validation method exists but is documentation-only — `apply_camera_key` doesn't invoke it.
- **Location**: `astraweave-render/src/renderer.rs:3371-3381`.
- **Confidence**: **medium**.
- **Type**: silent failure mode at API boundary.
- **Suggested resolution**: in scope for C.7 (cinematics evaluator upgrade). C.5 does not propose; C.7 planning round decides.

### L.5.18 — Three parallel cinematics camera systems coexist (new finding vs C.0)

- **Description**: `astraweave_cinematics::CameraKey` (canonical), editor `CameraKeyframe` (parallel UI-only with roll field), and `astraweave_gameplay::cutscenes::Cue::CameraTo` (parallel yaw/pitch system). No conversion functions between them. Only C5 (gameplay-cutscenes) has active production usage via `cutscene_render_demo`. C1's `Renderer::tick_cinematics` integration has zero production callers.
- **Locations**:
  - `astraweave-cinematics/src/lib.rs:237-307` (C1)
  - `tools/aw_editor/src/panels/cinematics_panel.rs:218-224` (C4)
  - `astraweave-gameplay/src/cutscenes.rs:7-12` (C5)
- **Confidence**: **high** (verified by grep — no conversion functions exist between the three).
- **Type**: parallel implementations / accretion.
- **Suggested resolution**: inventory-only per Decision 1's Medium scope. **C.7 planning round** decides the consolidation shape (potentially evolves C1 into a `RenderView`-producing evaluator that absorbs C5's usage and provides a canonical UI editing surface that subsumes C4).

### L.5.19 — `EngineRenderAdapter::update_water` has zero production callers (C.0 #8 reaffirmed)

- **Description**: per `astraweave-camera`'s C.3.B.1 migration note and the function's empirical caller search, `EngineRenderAdapter::update_water` (post-C.3.B.1 takes `&RenderView`) has no callers outside its own declaration.
- **Location**: `tools/aw_editor/src/viewport/engine_adapter.rs:3794`.
- **Confidence**: **high** (verified by grep in C.4.B's Phase 1 inventory).
- **Type**: dormant code.
- **Suggested resolution**: queued by CLAUDE.md as standalone follow-up ("update_water deletion verification"). Out of C.5 scope, but noted here for completeness.

### Summary table

| ID | Description | Confidence | Suggested resolution |
|---|---|---|---|
| L.5.1 | Gizmo CameraController dormant | high | C.6.A planning |
| L.5.2 | Gizmo gimbal-lock NaN | medium | C.6.A if (α), vanishes if (β) |
| L.5.3 | Gizmo `up` field mutation | low | C.6.A if (α), vanishes if (β) |
| L.5.4 | Gizmo aspect-zero guard absent | medium | C.6.A if (α), vanishes if (β) |
| L.5.5 | unified_showcase pitch clamp missing | high | C.6.B |
| L.5.6 | unified_showcase -Z forward | high | C.6.B planning |
| L.5.7 | unified_showcase hardcoded FOV | high | C.6.B |
| L.5.8 | shadow_csm_demo `fov` field name | high | C.6.C |
| L.5.9 | shadow_csm_demo aspect guard | medium | C.6.C |
| L.5.10 | shadow_csm_demo pitch clamp (C.0 falsified) | high | No action |
| L.5.11 | fluids_demo `fovy` stores degrees | high | C.6.D |
| L.5.12 | fluids_demo aspect guard | high | C.6.D |
| L.5.13 | fluids_demo pitch clamp (C.0 falsified) | high | No action |
| L.5.14 | nanite_demo doesn't render | high | C.6.E planning |
| L.5.15 | nanite_demo pitch clamp (C.0 falsified) | high | No action |
| L.5.16 | FreeFly missing FOV/near-far validation | medium | C.6 dedicated micro-phase or follow-up |
| L.5.17 | apply_camera_key degenerate keys | medium | C.7 |
| L.5.18 | Three parallel cinematics systems | high | C.7 (inventory-only here) |
| L.5.19 | update_water dormant | high | Standalone follow-up (already queued) |

## 4. Draft migration queue for C.6

Suggested cluster structure per Decision 6 (per-target). **The planning round locks order and scope decisions.**

### C.6.A — Gizmo `CameraController`

- **Migration goal options** (per §2.A): (α) implement `CameraProducer` + clean up latents; (β) delete production-side instance + retain in tests/benches if useful; (γ) middle path (retain type, drop dormant `TransformPanel.camera` field).
- **Estimated scope**:
  - (α): ~3-4 files (scene_viewport.rs internal + transform_panel.rs + tests for new producer impl + maybe astract integration test).
  - (β): ~3-4 files (scene_viewport.rs removed or trimmed + transform_panel.rs field removed + tests/benches updated/removed).
  - (γ): ~2 files (transform_panel.rs field removed + small doc update on scene_viewport.rs noting test/bench-only status).
- **Latent issues addressed**: L.5.1 (root); L.5.2, L.5.3, L.5.4 (fall out from (α) or vanish under (β)).
- **Suggested closure proof shape**:
  - (α): `CameraProducer` contract test analogous to `orbit_camera_producer.rs`; structural-presence assertion (impl exists for the type).
  - (β): structural-deletion verification (grep `CameraController` returns zero production matches).
  - (γ): structural-removal verification (grep `TransformPanel.*camera` returns zero matches).
- **Dependencies**: none; can execute immediately after C.5 closes.
- **Open question for planning round**: which of (α)/(β)/(γ)?

### C.6.B — `unified_showcase`

- **Migration goal options** (per §2.B.1): (α) migrate to engine; (β) formalize as sandbox + targeted fixes; (γ) targeted fix only (pitch clamp + comment noting convention deviation).
- **Estimated scope**:
  - (α): very large (the example is structurally bespoke-renderer; not just bespoke-camera). Likely deferred or out-of-campaign.
  - (β): small (a few comment additions + pitch clamp fix).
  - (γ): smallest (pitch clamp only).
- **Latent issues addressed**: L.5.5 (always); L.5.6, L.5.7 (only if (α) or comprehensive (β)).
- **Suggested closure proof shape**:
  - (α): full migration byte-equivalence via comparison fixture (large effort).
  - (β/γ): pitch-clamp structural test (assert that mouse delta with extreme value clamps pitch).
- **Dependencies**: none beyond C.6.A's potential gizmo cleanup.
- **Open question for planning round**: scope choice (α/β/γ)?

### C.6.C — `shadow_csm_demo`

- **Migration goal options**: (α) migrate to FreeFly via alias pattern; (β) targeted fix only (rename `fov` → `fovy`, aspect guard).
- **Estimated scope**:
  - (α): medium (the example has shadow-cascade-debug fields that may or may not translate cleanly to FreeFly).
  - (β): small (rename + 1-line aspect guard).
- **Latent issues addressed**: L.5.8 (rename), L.5.9 (aspect guard); L.5.10 already-closed.
- **Suggested closure proof shape**:
  - (α): the example compiles and renders the same shadow cascade visualization.
  - (β): grep verifies `fov:` is replaced by `fovy:`; aspect guard structural test.
- **Dependencies**: none.
- **Open question for planning round**: scope choice (α/β)?

### C.6.D — `fluids_demo`

- **Migration goal options**: (α) migrate to FreeFly + fluid-specific uniform retained separately; (β) targeted fix only (`fovy` degrees → radians; aspect guard).
- **Estimated scope**:
  - (α): medium-large (fluid `CameraUniform` has fields `RenderView` doesn't; need to either extend `RenderView` or maintain `fluid_uniform` as separate).
  - (β): small (~2-3 line changes).
- **Latent issues addressed**: L.5.11 (unit), L.5.12 (aspect); L.5.13 already-closed.
- **Suggested closure proof shape**: similar to C.6.C.
- **Dependencies**: none.
- **Open question for planning round**: scope choice (α/β)?

### C.6.E — `nanite_demo`

- **Migration goal options** (per §2.B.4): (α) migrate to engine + complete the render path; (β) mark as demo-only-stub + defer rendering; (γ) delete unused camera state.
- **Estimated scope**:
  - (α): large (complete the rendering implementation, not just migrate camera).
  - (β): trivial (add comment).
  - (γ): small (delete `camera_pos`, `camera_yaw`, `camera_pitch` fields and their handlers).
- **Latent issues addressed**: L.5.14 (root); L.5.15 already-closed.
- **Suggested closure proof shape**:
  - (α): integration test verifying the example renders the meshlet hierarchy.
  - (β): structural comment present.
  - (γ): grep verifies camera state fields are removed.
- **Dependencies**: none.
- **Open question for planning round**: scope choice (α/β/γ)?

### C.6.* — Optional dedicated micro-phase: `FreeFly` validation

If the planning round chooses to address L.5.16 (FreeFly missing FOV/near-far validation) during C.6, it warrants its own micro-phase due to its location in `astraweave-camera` (the canonical crate). The fix would be additive: a `sanitize()` or `validate()` method on FreeFly, or stricter `Mat4::perspective_rh` arguments via clamping. Empirical impact verification (contract tests for valid+invalid inputs) would be the closure proof.

**Suggested cluster order**:
1. C.6.A (gizmo) — smallest scope and clearest decision (dormancy is documented).
2. C.6.B, C.6.C, C.6.D, C.6.E — per-example migrations, parallel in ordering (planning round may interleave).
3. C.6.* (FreeFly validation, optional) — if Andrew approves the scope expansion.

Total estimated touch surface for full C.6: ~10-25 files depending on scope choices. Most clusters favor targeted fixes; full migrations are deferred or out-of-campaign.

## 5. Cinematics inventory for C.7 (informational)

This section restates §2.C's findings as structured input for the eventual C.7 planning round. **No migration proposal is made**; the planning round decides the consolidation shape.

### Type inventory

| ID | Type | Location | Field shape |
|---|---|---|---|
| C1 | `astraweave_cinematics::CameraKey` | `astraweave-cinematics/src/lib.rs:237-307` | `t: Time, pos: (f32,f32,f32), look_at: (f32,f32,f32), fov_deg: f32` |
| C2 | `Renderer::apply_camera_key` (private) | `astraweave-render/src/renderer.rs:3371-3381` | `(&mut FreeFly, &CameraKey)` |
| C3 | `Renderer::tick_cinematics` (public; **zero production callers**) | `astraweave-render/src/renderer.rs:3407` | `(&mut self, dt: f32, &mut FreeFly) -> Vec<SequencerEvent>` |
| C4 | Editor `CameraKeyframe` (parallel UI-only) | `tools/aw_editor/src/panels/cinematics_panel.rs:218-224` | `time: f32, position: (f32,f32,f32), look_at: (f32,f32,f32), fov: f32, roll: f32` |
| C5 | `Cue::CameraTo` (parallel gameplay-cutscenes) | `astraweave-gameplay/src/cutscenes.rs:7-12` | `pos: Vec3, yaw: f32, pitch: f32, time: f32` |

### Key questions for C.7 planning

1. **Consolidation target shape**: does C1 evolve into a `RenderView`-producing evaluator that subsumes C5's role and provides a canonical editing surface for C4? Or do C1/C5/C4 remain distinct with documented role boundaries?
2. **C5's yaw/pitch model**: gameplay cutscenes currently operate on yaw/pitch directly. Should C5 migrate to a look_at-based model (aligning with C1) or stay yaw/pitch (aligning with FreeFly's internal state)?
3. **Editor UI parity**: C4's `roll` field doesn't exist in C1 or C5. Is roll a real feature (cinematic camera roll for dutch tilts, etc.) that C1 should adopt, or is it editor-UI-only state that doesn't need canonical representation?
4. **Production wiring**: C3 has zero production callers today. Does C.7 wire C3 into a production cinematics pipeline (e.g., `cutscene_render_demo` migrates from C5 to C1/C2/C3), or does C.7 retire C3 in favor of C5's direct approach?
5. **`apply_camera_key` degenerate handling** (L.5.17): the silent acceptance of `look_at == pos` and out-of-range FOV. Does C.7 add validation, or document as caller-responsibility?
6. **Parity with rendered output**: the parity harness fixture (C.4 closure) doesn't exercise cinematics. Does C.7 add a cinematics-driven parity fixture (per the parity harness expansion noted in C.8's scope)?
7. **Camera-relative rendering**: when a cinematic camera flies through a large world (>10km from origin), does C.7's evaluator produce world-relative or camera-relative output? `CameraKey.pos` is `(f32, f32, f32)` — small magnitudes (mostly under 100m in the demo fixtures), but the API doesn't constrain it.

### Estimated complexity

- **Low** (preserving existing behavior with documentation): clarify role boundaries between C1/C4/C5; document that C3 is currently unused but reserved.
- **Medium** (consolidating C5 into C1): change `Cue::CameraTo` to wrap `CameraKey`; update `cutscene_render_demo` to use the unified type.
- **High** (full evaluator upgrade): C.7's SOTA target — `CameraKey` evolves into a continuous evaluator producing `RenderView` directly, replacing the event-based `apply_camera_key` round-trip; cinematics-driven parity fixtures; camera-relative handling for large worlds.

### Suggested sub-phase structure for C.7

If the planning round chooses the Low or Medium complexity:
- Single sub-phase (C.7).

If the High complexity:
- C.7.A: evaluator design + canonical `RenderView` production from `CameraKey`.
- C.7.B: integrate evaluator into existing `tick_cinematics`, deprecate `apply_camera_key`.
- C.7.C: migrate `cutscene_render_demo` and the editor cinematics panel to the unified path.
- C.7.D: cinematics-driven parity fixtures.

## 6. Open questions for the planning round

The planning round needs to lock decisions on the following before C.6.A drafts. Numbered for reference.

1. **C.6.A (gizmo CameraController) — scope choice**: which of (α) implement `CameraProducer` and fix latents / (β) delete production-side instance / (γ) drop the dormant `TransformPanel.camera` field but keep type for tests-and-benches?
2. **C.6.B (unified_showcase) — scope choice**: (α) migrate to engine / (β) formalize as sandbox + targeted fixes / (γ) targeted fix only?
3. **C.6.C (shadow_csm_demo) — scope choice**: (α) migrate to FreeFly / (β) targeted fix only?
4. **C.6.D (fluids_demo) — scope choice**: (α) migrate to FreeFly / (β) targeted fix only?
5. **C.6.E (nanite_demo) — scope choice**: (α) migrate + complete rendering / (β) mark as stub / (γ) delete unused camera state?
6. **L.5.16 (FreeFly FOV/near-far validation)**: address during C.6 (dedicated micro-phase) or bank as standalone follow-up?
7. **C.6 cluster ordering**: confirm or revise the suggested order C.6.A → C.6.B → C.6.C → C.6.D → C.6.E? (Per-target clusters are independent so ordering is flexible.)
8. **Closure proof shape per cluster**: confirm or revise the suggested closure proof shapes (producer-contract for (α), structural-deletion for (β/γ), targeted-test for (β/γ) of examples)?
9. **C.5 falsifications**: the planning round may want to note in `CAMERA_CONVENTIONS.md` §3 or elsewhere that C.0's three medium-confidence "likely missing pitch clamp" findings (shadow_csm_demo, fluids_demo, nanite_demo) were falsified by C.5's empirical re-inspection. Track as resolved.
10. **C.7 timing**: does C.7 planning happen before C.6 closes, or only after? (Decision 1's Medium scope deferred cinematics to C.7; C.5's §5 is informational for that planning round.)

## 7. Methodology observations

C.5's audit work surfaces a small number of methodology observations worth noting as §7.11 candidates for C.9 / E-closeout. Each is noted **as candidate**, not codified.

### 7.1 Audit-shaped-sub-phase pattern (second instance)

C.0 was the first audit-shaped sub-phase: a pure-audit deliverable produces an inventory document that feeds subsequent planning rounds. C.5 is the second instance of this shape. Distinct from execution sub-phases (which produce code changes with byte-equivalence / structural / contract closure proofs) and from documentation sub-phases (C.3.D — documentation-accuracy closure proofs).

The pattern has now been applied at two distinct campaign phase boundaries:
- C.0: pre-campaign (Phase 1 audit before any consolidation).
- C.5: mid-campaign (Phase 2 audit after first-pass consolidation, before second-pass execution).

If a future campaign also benefits from an audit-before-second-pass pattern, this would be a third concrete instance. Two instances is enough to **note as a candidate methodology pattern**, codification deferred to E-closeout.

### 7.2 Falsification opportunity discipline (new candidate)

C.5's Phase 2 inspection re-checked every C.0 medium-confidence latent issue. Three of four "likely missing pitch clamp" findings were **falsified** by empirical re-inspection (shadow_csm_demo, fluids_demo, nanite_demo). Only the unified_showcase finding was confirmed. This is a 75% false-positive rate on C.0's specific medium-confidence pitch-clamp findings.

**Observation**: medium-confidence findings drawn from inspection-without-runtime-grep are prone to false positives. The discipline that improves accuracy: when C.0 said "likely missing pitch clamp" based on "didn't find clamp in inspected lines," the right next step is to grep more broadly (input handlers, mouse delta processing) before recording the finding. C.5 did this for each example.

**Candidate methodology**: "audit findings of inferred-not-verified shape should be re-inspected during the immediate next audit phase." This is a load-bearing discipline for cross-phase audit fidelity. C.5 just demonstrated it; C.9 / E-closeout can codify whether this becomes a named pillar.

### 7.3 Empirical-verification α (Decision 5 reinforcement)

C.5's Decision 5 (empirical verification of every claim) was the load-bearing methodological choice. The dormant-gizmo-CameraController finding (L.5.1) is a direct product of this discipline: a casual reading of `transform_panel.rs:51` would suggest "`TransformPanel` has a `CameraController`," but grep verifies the field is never *read*. The "wired beats tested" pattern from CLAUDE.md is structurally indistinguishable from the dormancy this audit identifies — without empirical verification, the gizmo `CameraController` looks like a feature; with empirical verification, it's dormant code.

**Candidate pillar reinforcement**: "audit findings should distinguish between 'declared' and 'used' via grep." This is implicit in the existing Pillar 7 (architectural-priority validation) but C.5 demonstrates the specific empirical discipline that makes Pillar 7 land. Codification at C.9 / E-closeout.

### 7.4 C.4's reframing as audit-correction (observation, not candidate pillar)

C.0 identified the OrbitCamera `ray_from_screen` vs `unproject_depth_to_world` as a "VP mismatch divergence at large camera world positions" (C.0 §3.2 medium-confidence finding). C.4's Phase 1 inspection reframed this as a **float-precision issue, not a coord-space mismatch** (clip-space output is invariant across world-space vs camera-relative VP pipelines; the divergence at large positions is float precision in matrix inversion, not a structural disagreement). C.4 closed the issue with the precision-stable inversion pattern.

**Observation**: C.4's reframing is the right shape — the audit finding led to the inspection that produced the correct understanding, which led to the correct fix. The reframing is not a flaw in C.0's discipline; it's the discipline working as designed (audit surfaces hypothesis; execution phase verifies and acts).

This is noted to acknowledge that audit findings are hypotheses, not settled facts — which reinforces Decision 5's empirical-verification discipline. Not a new candidate pillar; reinforces the existing methodology.

## §8 — Cross-references

- **C.0 audit**: `docs/audits/camera_system_architecture_audit_2026-05.md`. The pre-campaign Phase 1 audit. C.5 is its mid-campaign Phase 2 companion.
- **Canonical conventions**: `docs/current/CAMERA_CONVENTIONS.md`. The authoritative reference for the conventions C.5 audits against.
- **C.4 + C.4.B planning context**: committed at `20666fb46` (C.4) and `550236208` (C.4.B). These sub-phases close the editor-side first-pass; C.5 audits what remains.
- **Parity outcome**: `docs/audits/editor_engine_render_parity_outcome_2026-05.md`. The Editor-Engine Render Parity campaign's outcome doc. C.5 does not modify this; cited for historical context on the campaign's broader scope.

---

**Revision history**:

- v1.0 (2026-05-24) — initial publication. C.5 closure. Phase 2 audit of the post-C.4.B mid-migration state. Five in-scope migration targets (gizmo + 4 examples) characterized; cinematics inventory captured for C.7 planning; 19 latent issues catalogued (3 of C.0's medium-confidence findings falsified by empirical re-inspection); draft C.6 migration queue with 5 cluster proposals; 10 open questions for the planning round.
