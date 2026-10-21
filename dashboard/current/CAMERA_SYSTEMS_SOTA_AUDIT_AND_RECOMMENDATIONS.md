# Camera Systems SOTA Audit and Recommendations

| Field | Value |
|-------|-------|
| Date | 2026-04-28 |
| Scope | Research and documentation only |
| Code changes | None |
| Tests written or run | None |
| Primary recommendation | Build a formal camera subsystem around the existing renderer/editor/cinematics pieces |

This report audits AstraWeave's camera-related implementation, compares it with current game-engine camera architecture practices, and recommends a staged path toward a canonical camera system. It intentionally changes no Rust code, no tests, and no runtime configuration.

## Executive Summary

AstraWeave has several strong camera foundations already in place: a small renderer camera primitive, camera-relative rendering support for large-world precision, editor orbit navigation, screen-to-world picking, frustum extraction, camera-aware CSM updates, and cinematic camera keyframes. The main weakness is not missing math; it is architectural fragmentation. Camera state, camera controllers, render-view data, editor navigation, cinematic keyframes, water/impostor update paths, and culling/shadow consumers all speak related but different camera dialects.

The correct direction is to move toward a formal camera subsystem, not to keep extending the scattered model. That subsystem should define one canonical evaluated camera/view contract and allow many producers: gameplay rigs, editor cameras, cinematic tracks, debug cameras, AI-directed cameras, and future split-screen/scene-capture views. Existing camera-relative rendering and wgpu `[0, 1]` depth assumptions should be preserved.

| Priority | Finding | Recommendation |
|----------|---------|----------------|
| P0 | No canonical camera/view contract across renderer, editor, cinematics, and tools. | Define `CameraState` / `RenderView` semantics before adding more camera features. |
| P0 | Editor orbit camera avoids engine yaw/pitch conversion in one path, but water still converts via `to_engine_camera()`. | Centralize conversion through the same evaluated view/projection contract. |
| P1 | Cinematics store `pos`, `look_at`, and `fov_deg`, but emit key events rather than an evaluated active camera. | Add a cinematic camera evaluator/blender that outputs the canonical camera contract. |
| P1 | Renderer UBO has only `view_proj`, light direction, and camera position; several systems need near/far, previous matrices, jitter, and inverse matrices over time. | Expand camera-derived view data behind a new render-view layer, not ad hoc call parameters. |
| P1 | Large-world camera-relative support exists but is feature-gated and requires caller discipline. | Make camera-relative origin an explicit field in the evaluated render view. |
| P2 | Editor camera combines navigation, persistence, picking, frustum extraction, and render adaptation in one type. | Keep editor UX behavior, but split camera data, controller, and viewport services over time. |

## AstraWeave Current-State Inventory

### Renderer Camera Primitive

`astraweave-render/src/camera.rs` defines a compact runtime camera:

- `Camera { position, yaw, pitch, fovy, aspect, znear, zfar }`.
- `view_matrix()` uses right-handed `Mat4::look_to_rh` with world `+Y` up.
- `proj_matrix()` uses `Mat4::perspective_rh`.
- `vp()` multiplies projection by view.
- `dir(yaw, pitch)` derives the forward vector from trigonometric yaw/pitch.
- `view_matrix_camera_relative()` strips translation by placing the camera at origin for large-world precision.

It also includes `CameraController` with free-fly and orbit modes, keyboard movement, right-mouse dragging, raw mouse delta support, FOV zoom, orbit-distance zoom, exponential smoothing, sprint/precision modifiers, and tests for basic camera math and camera-relative translation stripping.

Assessment: this is useful as a low-level renderer camera, but it is not a full engine camera system. It mixes a data carrier with a specific free-fly/orbit input controller and exposes yaw/pitch as the primary orientation contract. That is fragile for cinematic look-at cameras, roll, shoulder cameras, spline cameras, VR/multi-view, and externally authored transforms.

### Renderer Upload and Camera-Relative Paths

`astraweave-render/src/renderer.rs` carries camera-derived GPU state:

- `CameraUBO` stores `view_proj`, `light_dir_pad`, and `camera_pos_pad`.
- `cached_view` and `cached_proj` preserve the last uploaded view/projection matrices.
- The `camera-relative` feature exists in `astraweave-render/Cargo.toml`.
- `set_camera_world_position(DVec3)` stores a double-precision camera origin for camera-relative rendering.
- `update_camera_matrices(view, proj, position, znear, zfar, fovy, aspect)` accepts precomputed matrices, strips view translation when camera-relative is active, writes the camera UBO, and derives a temporary `Camera` for cascade updates.
- `update_camera(&Camera)` uses `view_matrix_camera_relative()` when the feature is active and passes a camera-relative camera to cascade updates.
- `update_instances()` offsets model translations by `camera_world_pos` under the camera-relative feature.
- `current_view_proj()` exposes `cached_proj * cached_view` for external fanout such as impostor passes.

Assessment: the renderer has a strong large-world rendering concept. The issue is that `update_camera_matrices` is effectively an alternate camera API beside `update_camera`, and both must keep side effects aligned: UBO upload, cached matrices, light direction, CSM cascade updates, external pass updates, instance offsets, water, sky, and impostors. That is a sign the renderer wants a single `RenderView` input object.

### Frustum Culling and Cascaded Shadows

`astraweave-render/src/culling.rs` defines `FrustumPlanes::from_view_proj(&Mat4)` and both CPU and GPU frustum-culling paths. The indirect culling shader consumes frustum planes and emits draw commands. The docs and tests also track a 96-byte `FrustumPlanes` layout.

`astraweave-render/src/shadow_csm.rs` updates four directional-light cascades from camera data:

- It computes logarithmic/uniform blended split distances.
- It unprojects cascade frustum corners from NDC using wgpu `[0, 1]` depth assumptions.
- It fits each cascade from the camera frustum slice, places the light by cascade bounds, snaps to texels to reduce shimmer, and stores dirty flags.

Assessment: CSM and culling are already camera consumers, and recent docs indicate older origin-locked shadow issues have been addressed in current code. The recommended change is not to rewrite those systems now; it is to ensure they consume canonical evaluated render-view data so shadow/culling correctness does not depend on which upstream camera path was used.

### Editor Orbit Camera

`tools/aw_editor/src/viewport/camera.rs` defines `OrbitCamera`, a professional editor navigation camera with:

- spherical focal-point coordinates: focal point, distance, yaw, pitch;
- FOV/aspect/near/far camera settings;
- min/max distance and pitch constraints;
- target values for smoothed orbit, pan, zoom, framing, and view changes;
- screen-space picking via `ray_from_screen`;
- camera-relative projection helpers;
- depth-buffer unprojection via `unproject_depth_to_world`;
- frustum extraction with wgpu `[0, 1]` near-plane handling;
- deserialization sanitation;
- `to_engine_camera()` for conversion to `astraweave_render::camera::Camera`.

`tools/aw_editor/src/viewport/engine_adapter.rs` calls `renderer.update_camera_matrices(camera.view_matrix(), camera.projection_matrix(), camera.position(), ...)` directly. Comments explicitly say this avoids yaw/pitch convention issues between the orbit camera and engine camera direction conventions.

Assessment: the editor camera is the most feature-complete camera implementation in the repo. It also proves why a canonical view contract is needed: direct matrix upload is correct for the editor, while `to_engine_camera()` remains available and is used by water update, preserving a second conversion path that can drift.

### Viewport, Terrain, Scatter, Gizmo, and Physics Integration

The editor viewport uses the camera for:

- renderer camera upload each frame;
- terrain framing after generated terrain;
- terrain brush raycasts and depth unprojection;
- grid/gizmo/physics debug rendering with camera-relative view-projection;
- scatter chunk streaming and LOD rebucketing from camera position;
- impostor pass camera UBO updates through `current_view_proj()`;
- water updates through an engine-camera conversion path.

Existing docs already connect camera quality to terrain/scatter performance and correctness. `docs/reference/RENDERING_SOTA_REFERENCE.md` calls camera-relative rendering a top rendering practice and flags f32 world-space beyond roughly 10 km as a jitter/z-fighting risk. `docs/current/RENDERING_UPGRADE_PLAN.md` documents the feature-gated camera-relative implementation and CSM use of camera-relative origin. `docs/current/TERRAIN_GENERATION_QUALITY_CAMPAIGN.md` notes that world-unit convention is still implicit, while camera docs use meters.

Assessment: camera is already a cross-cutting service for authoring, rendering, streaming, selection, LOD, and terrain UX. Treating it as a renderer helper will keep causing coordination bugs.

### Cinematics and Veilweaver Usage

`astraweave-cinematics/src/lib.rs` defines:

- `Track::Camera { keyframes: Vec<CameraKey> }`;
- `CameraKey { t, pos, look_at, fov_deg }`;
- `CameraKey::lerp()` with linear interpolation of position, look-at, and FOV;
- `SequencerEvent::CameraKey(CameraKey)`;
- `Sequencer::step()` that emits camera key events when their key time is crossed.

`veilweaver_slice_runtime/src/cinematic_player.rs` loads RON timelines and steps a sequencer; tests create camera tracks. The cinematic system is good for authored events, but it is not yet an active camera evaluator: it emits discrete key events, not a per-frame camera view blended into gameplay/editor/render outputs.

Assessment: cinematics should be treated as camera producers. They need an evaluator that outputs the same canonical camera state as gameplay and editor cameras.

## SOTA Camera Architecture Research

### Engine-Level Pattern: Cameras Are Producers, Views Are Render Inputs

Modern engines separate authored/control-side cameras from renderer-side views.

- Unreal documents cameras as first-class gameplay/editor objects: Camera Actors, Camera Components, PlayerCameraManager, camera modifiers, blends, shakes, and view targets. This points to an architecture where many gameplay sources can propose a view and a manager resolves the final player view. Source: Epic, "Cameras in Unreal Engine" and related Camera Actor/Component pages, accessed 2026-04-28.
- Unity Cinemachine is explicitly a procedural camera system that places and aims cameras according to tracked targets, blends between virtual cameras, and layers procedural motion/noise on top of camera intent. Source: Unity Cinemachine 3.1 manual and procedural motion concept page, accessed 2026-04-28.
- Godot exposes Camera3D and Camera2D as scene nodes with projection, frustum, culling-mask, environment, and current-camera behavior. Source: Godot Camera3D and Camera2D class documentation, accessed 2026-04-28.
- Bevy's camera architecture treats cameras as ECS components that drive render targets and view extraction. Its viewport-to-world examples emphasize using camera transforms plus projection state to derive rays and world positions. Source: Bevy camera crate docs and Bevy 3D viewport-to-world example, accessed 2026-04-28.

Conclusion: a SOTA engine should not have one hard-coded camera struct that every feature mutates directly. It should have camera producers/controllers and an evaluated view contract consumed by rendering, picking, audio, LOD, and streaming.

### Rig Pattern: Camera Data, Controller, Rig, and Effects Are Different Layers

The recurring SOTA split is:

- **Camera data:** transform, projection, clip/depth convention, viewport, render target, exposure/DOF metadata.
- **Controller/input:** free-fly, orbit, first-person, third-person, tactical, editor navigation.
- **Rig/procedural behavior:** follow target, shoulder offset, constraints, dead zones, collision, occlusion, damping, look-ahead.
- **Blender/manager:** priority, cuts, blend curves, gameplay/cinematic/debug handoff.
- **Effects:** shake, impulse, recoil, noise, FOV kick, post-process overrides.
- **Render view:** the immutable per-frame evaluated data used by GPU systems.

Unity Cinemachine's virtual-camera and procedural-motion model is the clearest shipped example of this separation. Unreal's PlayerCameraManager, modifiers, shakes, and view-target blending are another. AstraWeave currently has parts of these layers, but they are not named or routed through a single contract.

### Math and Rendering Contract Must Be Explicit

Camera systems must lock down coordinate and projection conventions because tiny mismatches produce severe rendering bugs:

- wgpu/WebGPU follows a modern GPU clip-depth convention with normalized depth `[0, 1]`; AstraWeave's CSM and editor frustum extraction already assume this.
- glam `Mat4` provides right-handed look/projection constructors such as `look_to_rh`, `look_at_rh`, and `perspective_rh`; AstraWeave uses these heavily.
- A renderer-facing view object should include more than `view_proj`: `view`, `proj`, inverses, near/far, FOV, aspect, viewport rect, camera origin, previous-frame matrices, and optional jitter should all be deliberate.

Conclusion: AstraWeave should document and encode its convention as part of a canonical `RenderView`, including right-handed world/view assumptions, `+Y` up, wgpu `[0, 1]` NDC depth, and camera-relative origin.

### Large Worlds Require Camera-Relative Rendering or Origin Management

Large-world engines avoid sending huge f32 world coordinates directly to the GPU. AstraWeave's existing camera-relative design matches the right pattern: store high-precision world camera position on CPU, subtract it from object/light positions before GPU upload, and use a rotation-only view matrix on the GPU. This matches the broader SOTA guidance in `docs/reference/RENDERING_SOTA_REFERENCE.md` and terrain-rendering literature where camera-centered techniques such as geometry clipmaps reduce precision and LOD problems.

Conclusion: preserve AstraWeave's camera-relative renderer path, but make it explicit in the evaluated render view rather than a feature-gated side channel that callers must remember to prepare.

### Cinematic Cameras Need Continuous Evaluation

SOTA cinematic cameras are not just keyframe events. They evaluate camera state continuously from authored data: cuts, blends, splines, look-at constraints, FOV/lens changes, camera shake, and post-process state. AstraWeave's `CameraKey::lerp()` is a useful base, but the active system should evaluate "camera at time t" and hand that result to the same camera manager/blender used by gameplay.

Conclusion: keep the timeline model, but add an evaluator that produces canonical camera states rather than only emitting key events.

## Comparison and Risk Analysis

| Area | Current State | SOTA Expectation | Risk |
|------|---------------|------------------|------|
| Canonical contract | Renderer `Camera`, editor `OrbitCamera`, matrices, and cinematic `CameraKey` coexist. | One evaluated camera/view model consumed by all systems. | High: convention drift and duplicate bug fixes. |
| Orientation | Runtime camera uses yaw/pitch; editor orbit uses focal-point look-at and direct matrices. | Transform/quaternion or matrix should be canonical; yaw/pitch can be controller state. | High: conversion errors, no roll, awkward cinematics. |
| Projection | FOV/aspect/near/far exist in several places. | Projection should be typed and validated: perspective/orthographic/custom/frustum. | Medium: editor/game/cinematic settings diverge. |
| Render-view data | UBO only exposes `view_proj`, light direction, camera position. | Render view should carry current/previous matrices, inverses, near/far, jitter, origin, viewport. | Medium: post effects, TAA, picking, water, shadows grow ad hoc parameters. |
| Camera-relative | Strong implementation exists, feature-gated. | Large-world origin should be part of view evaluation. | Medium: caller order bugs and inconsistent consumers. |
| Editor viewport | Feature-rich orbit camera and picking. | Editor camera should be a producer/controller over canonical view data. | Medium: editor remains correct but isolated. |
| Cinematics | Keyframes and events exist. | Continuous evaluator and blending into active camera manager. | High for future cutscenes/gameplay handoff. |
| Multi-camera | No clear camera registry or render-view list. | Multiple active cameras/views for split screen, scene capture, minimaps, mirrors, editor panels. | Medium: future features will duplicate renderer entry points. |
| Testing posture | Many unit tests exist around camera pieces. | Contract tests should lock conventions and producer equivalence. | Medium: current tests cover pieces, not whole-system invariants. |

## Recommended Target Architecture

### 1. Canonical Camera/View Contract

Define a renderer-independent camera contract that can be produced by gameplay, editor, cinematic, and debug systems:

- `CameraTransform`: position in world space, orientation as quaternion or basis, optional target/look-at metadata for tools.
- `Projection`: perspective, orthographic, or custom frustum with explicit vertical FOV, aspect, near/far, and validation.
- `CameraOutput`: physical/display metadata such as exposure hints, depth of field, post-process profile, viewport/render target, and camera category.
- `RenderView`: immutable per-frame evaluated data: `view`, `proj`, `view_proj`, inverses, previous matrices, near/far, FOV, aspect, viewport, camera world position as `DVec3`, camera-relative origin, jitter, and clip-space convention.

Do not make yaw/pitch canonical. Keep yaw/pitch as controller-specific state for free-fly and orbit navigation.

### 2. Camera Producers and Controllers

Separate producers from evaluated output:

- `FreeFlyCameraController`: current renderer camera controller behavior.
- `OrbitCameraController`: editor focal-point navigation behavior.
- `FollowRig` / `ThirdPersonRig`: gameplay target cameras.
- `CinematicCameraTrackEvaluator`: timeline keyframes/splines to camera output.
- `DebugCameraProducer`: inspection cameras.
- `AICameraDirector` later: high-level camera intent producer.

Each producer should output a candidate `CameraState` plus priority/blend metadata, not upload directly to the renderer.

### 3. Camera Manager and Blender

Add a manager that resolves active camera state:

- supports hard cuts and timed blends;
- blends transform, look-at target, FOV, and post-process metadata deliberately;
- allows gameplay/cinematic/debug/editor ownership transitions;
- emits exactly one active `RenderView` for normal game rendering, with room for multiple views later.

This mirrors the useful pieces of Unreal's PlayerCameraManager and Unity Cinemachine without copying their APIs wholesale.

### 4. Renderer Consumes RenderView Only

Long term, replace parallel `update_camera(&Camera)` and `update_camera_matrices(...)` entry points with one renderer-facing input:

```text
Renderer::update_view(RenderView)
```

That call should update:

- camera UBO and any expanded view UBO;
- cached current/previous matrices;
- camera-relative origin;
- CSM cascade inputs;
- water, impostor, sky, terrain, and culling view fanout;
- any future TAA/velocity/DOF/SSR data that requires current/previous camera state.

Existing public methods can remain during migration, but should become adapters to the same internal view path.

### 5. Cinematics Become Continuous Camera Sources

Keep `CameraKey` and `Timeline`, but add:

- "evaluate camera track at time t" behavior;
- interpolation modes beyond linear once needed;
- conversion from `(pos, look_at, fov_deg)` to canonical camera transform/projection;
- camera cuts/blends into the manager;
- explicit treatment of invalid keys such as zero-distance look-at or invalid FOV.

This keeps authored cinematic content compatible while making it useful to the runtime renderer.

## Staged Migration Plan

### Phase 0: Document and Lock Conventions

- Write a short camera convention note: right-handed view constructors, world `+Y` up, wgpu `[0, 1]` NDC depth, projection order, camera-relative origin semantics.
- Add contract tests later for runtime camera, editor orbit camera, frustum extraction, and cinematic look-at evaluation. No tests were written as part of this report.

### Phase 1: Introduce Canonical Types Without Rewiring Everything

- Add `CameraState`, `Projection`, and `RenderView` types in an appropriate shared crate or renderer-facing module.
- Implement adapters from existing `astraweave_render::camera::Camera`, editor `OrbitCamera`, and `CameraKey`.
- Keep existing renderer methods, but make their intended target mapping explicit.

### Phase 2: Consolidate Renderer Camera Upload

- Add an internal `update_view(RenderView)` path.
- Route `update_camera()` and `update_camera_matrices()` through it.
- Move camera-relative origin and current/previous matrix caching into the render-view update path.
- Ensure CSM, culling, sky, water, impostors, and terrain all consume the same view data.

### Phase 3: Add Camera Manager/Blender

- Introduce camera producers with priority and blend metadata.
- Support cuts and timed blends between gameplay, cinematic, debug, and editor sources.
- Keep editor single-camera behavior as a producer/controller so editor UX does not regress.

### Phase 4: Upgrade Cinematics

- Add continuous camera-track evaluation.
- Integrate cinematic camera source into the manager.
- Preserve existing timeline files by treating current `CameraKey` data as a v1 authoring format.

### Phase 5: Prepare Multi-View Features

- Expand `RenderView` lists for split screen, scene capture, minimaps, camera previews, mirrors, and multi-viewport editor layouts.
- Make viewport/render target part of the evaluated view.
- Add previous-view data for TAA, motion vectors, and temporal upscaling if not already present.

## Recommended Acceptance Criteria for Future Implementation

When this is implemented later, the camera subsystem should pass these scenarios:

- Runtime free-fly camera and editor orbit camera produce equivalent `RenderView` output when configured for the same position/orientation/projection.
- `OrbitCamera` no longer needs a special renderer matrix path to avoid yaw/pitch conversion errors; all paths use canonical transforms or matrices.
- Water, sky, CSM, impostors, terrain, picking, and culling read the same evaluated view data.
- Camera-relative rendering works at large coordinates without caller-specific setup mistakes.
- A cinematic camera track can evaluate continuously at arbitrary time `t` and blend to/from gameplay camera.
- wgpu `[0, 1]` depth and right-handed view conventions are covered by tests and documentation.

## Source Bibliography

Accessed 2026-04-28 unless otherwise noted.

- Epic Games, "Cameras in Unreal Engine" — https://dev.epicgames.com/documentation/en-us/unreal-engine/cameras-in-unreal-engine
- Epic Games, "Camera Actors" — https://dev.epicgames.com/documentation/en-us/unreal-engine/camera-actors-in-unreal-engine
- Epic Games, "Camera Components" — https://dev.epicgames.com/documentation/en-us/unreal-engine/camera-components-in-unreal-engine
- Unity Technologies, "Cinemachine 3.1 Manual" — https://docs.unity3d.com/Packages/com.unity.cinemachine@3.1/manual/
- Unity Technologies, "Cinemachine Procedural Motion" — https://docs.unity3d.com/Packages/com.unity.cinemachine@3.1/manual/concept-procedural-motion.html
- Godot Engine, "Camera3D" — https://docs.godotengine.org/en/stable/classes/class_camera3d.html
- Godot Engine, "Camera2D" — https://docs.godotengine.org/en/stable/classes/class_camera2d.html
- Bevy, "3D Viewport To World" — https://bevy.org/examples/3d-rendering/3d-viewport-to-world/
- Bevy Camera crate docs — https://docs.rs/bevy_camera/latest/bevy_camera/
- wgpu crate docs — https://docs.rs/wgpu/latest/wgpu/
- glam `Mat4` docs — https://docs.rs/glam/latest/glam/f32/struct.Mat4.html
- NVIDIA GPU Gems 2, Chapter 2, "Terrain Rendering Using GPU-Based Geometry Clipmaps" — https://developer.nvidia.com/gpugems/gpugems2/part-i-geometric-complexity/chapter-2-terrain-rendering-using-gpu-based-geometry
- AstraWeave local reference, `docs/reference/RENDERING_SOTA_REFERENCE.md`
- AstraWeave local plan, `docs/current/RENDERING_UPGRADE_PLAN.md`
- AstraWeave local campaign notes, `docs/current/TERRAIN_GENERATION_QUALITY_CAMPAIGN.md`

## Verification Note

This was a documentation-only task. No code was changed, no tests were written, and no cargo checks were run. External source pages were reachable during implementation, except one outdated Unity concept URL that was replaced with a reachable Unity Cinemachine procedural-motion page.
