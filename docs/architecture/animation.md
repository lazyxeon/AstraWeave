# Architecture Trace: Animation System

## Metadata

| Field | Value |
|---|---|
| **System name** | Skeletal Animation System |
| **Primary crates** | `astraweave-render` (canonical runtime), `astraweave-asset` (glTF import), `astraweave-scene` (ECS integration), `tools/aw_editor` (editor UI + bridge) |
| **Document version** | 1.0 |
| **Last verified against commit** | `32afac52f` |
| **Last verified date** | 2026-05-10 |
| **Status** | Active. Phase 2 Task 5 delivered the core pipeline (asset import → animation runtime → CPU/GPU skinning → ECS integration → tests + demo) on 2025-10-01. Continues to evolve via the editor's `EditorAnimationBridge` and Wave 2 mutation-remediation tests. |
| **Owner notes** | The system spans **four crates** with **parallel type families** for the same logical concepts (`Skeleton`, `Joint`, `Transform`, `AnimationClip`, `AnimationChannel`, `Interpolation`, `ChannelData`). Each family is internally consistent but distinct from the others. The crossings are handled at integration points (glTF loader → runtime, runtime → ECS, editor → bridge). See §6 for the forensic conflict map. |

---

## 1. Executive Summary

**What this system does:**
Loads glTF skeletons and animation clips, plays them back at runtime with keyframe interpolation (Step / Linear / Slerp), composes hierarchical joint matrices, and applies vertex skinning either on the CPU (deterministic, default) or on the GPU (via the `skinning-gpu` feature, using a pooled storage-buffer of joint palettes with dynamic offsets). ECS components (`CSkeleton`, `CSkinnedMesh`, `CAnimator`, `CJointMatrices`, `CParentBone`) drive per-entity animation state through scene-graph systems. The editor surfaces a clip library and per-entity playback through `EditorAnimationBridge`.

**Why it exists:**
Provides the canonical skeletal animation runtime for the engine — used by `examples/skinning_demo`, integration tests, the editor viewport, and any subsystem that needs to render bone-animated meshes. The CPU path is the deterministic CI-safe default; the GPU path is opt-in for performance.

**Where it primarily lives:**
- `astraweave-render/src/animation.rs` (710 LoC) — canonical runtime types and CPU pipeline (`Skeleton`, `Joint`, `Transform`, `AnimationClip`, `AnimationChannel`, `ChannelData`, `Interpolation`, `AnimationState`, `JointMatrixGPU`, `JointPalette`, `MAX_JOINTS = 256`, `compute_joint_matrices`, `skin_vertex_cpu`)
- `astraweave-render/src/skinning_gpu.rs` (814 LoC, feature `skinning-gpu`) — pooled `JointPaletteManager` SSBO with dynamic offsets, plus the embedded WGSL `SKINNING_GPU_SHADER`
- `astraweave-render/src/types.rs:51-106` — `SkinnedVertex` GPU vertex layout (60 bytes per vertex: position + normal + tangent + uv + joints[u16;4] + weights[f32;4])
- `astraweave-asset/src/lib.rs:854-…` — glTF import (`load_skeleton`, `load_animations`, plus parallel asset-side `Skeleton`/`Joint`/`Transform`/`AnimationClip`/`AnimationChannel`/`ChannelData`/`Interpolation` types using plain float arrays for Pod-friendly serialization)
- `astraweave-scene/src/lib.rs:415-` — ECS animation components (`CSkeleton`, `CSkinnedMesh`, `CAnimator`, `CJointMatrices`, `CParentBone`, `PlaybackState`) + systems (`update_animations`, `compute_poses_stub`, `update_bone_attachments`)
- `tools/aw_editor/src/viewport/types.rs:240-298` — editor-side `Gltf*` animation types (`GltfSkeleton`, `GltfJoint`, `GltfAnimationClip`, `GltfAnimChannel`, `GltfChannelProperty`, `GltfInterpolation`)
- `tools/aw_editor/src/animation_bridge.rs` (451 LoC) — `EditorAnimationBridge` with per-entity playback state (`EntityAnimationState`, `ClipEntry`, `AnimationOutput`, `JointTransform`)
- `tools/aw_editor/src/panels/animation_panel.rs` (3,680 LoC) — editor UI panel
- `examples/skinning_demo/src/main.rs` — interactive CPU/GPU skinning demonstration

**Status note:**
The core pipeline is Phase-2-Task-5 complete (per `docs/archive/phase_reports/PHASE2_TASK5_IMPLEMENTATION_SUMMARY.md`, 2025-10-01). The current visible surface area is dominated by **four parallel type families** for the same logical animation concepts — this is the dominant cognitive trap in the system. See §6.

---

## 2. Authoritative Pipeline

```text
[A: glTF asset file (.gltf / .glb)]
    │
    │ astraweave_asset::load_skeleton(bytes) / load_animations(bytes, skeleton)
    │   file: astraweave-asset/src/lib.rs:942-… (load_skeleton),
    │         astraweave-asset/src/lib.rs:1091-… (load_animations)
    │   role: parse glTF skin + animation, emit asset-side
    │         Skeleton / AnimationClip with [f32;3]/[f32;4]/[[f32;4];4] field storage
    ▼
[B: Conversion to runtime types]
    file: (caller-site, no canonical conversion function found in available sources)
    role: Translate asset-side {Skeleton, Joint, Transform, AnimationClip, AnimationChannel,
          ChannelData, Interpolation} into runtime astraweave_render::animation types using
          glam::{Vec3, Quat, Mat4}.
    note: [NEEDS VERIFICATION — exact conversion site not traced; loaders historically
          consumed by skinning_demo + scene loaders]
    │
    ▼
[C: ECS component spawn]
    files: astraweave-scene/src/lib.rs:421 (CSkeleton),
           astraweave-scene/src/lib.rs:493 (CAnimator),
           astraweave-scene/src/lib.rs:609 (CJointMatrices),
           astraweave-scene/src/lib.rs:629 (CParentBone)
    role: Attach skeleton + animator state to entities. CSkeleton uses parallel-array storage
          (joint_count + parent_indices + inverse_bind_matrices + local_transforms) rather than
          Vec<Joint>; CAnimator carries clip_index + time + speed + PlaybackState + looping
    │
    │ per-tick: update_animations(world, dt, clip_durations)  (astraweave-scene/src/lib.rs:896)
    ▼
[D: AnimationClip::sample(time, skeleton)]
    file: astraweave-render/src/animation.rs:101-166
    role: Per active channel: find_keyframes() binary-search → interpolate per ChannelData variant
          (Translation: Vec3::lerp; Rotation: Quat::slerp; Scale: Vec3::lerp;
           CubicSpline falls back to linear/slerp at lines 127, 140, 153).
          Returns Vec<Transform> indexed by joint.
    │
    ▼
[E: compute_joint_matrices(skeleton, local_transforms)]
    file: astraweave-render/src/animation.rs:274-336
    role: Hierarchical traversal from root_indices via recursive walk; cycle detection via
          visited HashSet (anyhow::Error on cycle, line 290).
          Result: Vec<Mat4> = world_matrix * inverse_bind_matrix per joint (line 332).
    │
    ├─→ CPU path (default)
    │   file: astraweave-render/src/animation.rs:401-435 (skin_vertex_cpu)
    │   role: For each vertex { position, normal, joints[u16;4], weights[f32;4] }:
    │         accumulate weight * (matrix.transform_point3(position)) and weight * (mat3 * normal);
    │         normalize_or_zero the normal at the end.
    │   →  Skinned (Vec3, Vec3) returned for software vertex transform / CPU shading
    │
    └─→ GPU path (feature "skinning-gpu")
        files: astraweave-render/src/skinning_gpu.rs (JointPaletteManager),
               astraweave-render/src/skinning_gpu.rs:254-… (SKINNING_GPU_SHADER)
        role: JointPaletteManager::upload_matrices(handle, &matrices) writes via
              queue.write_buffer(pool_buffer, offset, ...). Bind group at group(4) binding(0)
              with has_dynamic_offset=true (skinning_gpu.rs:59-71). Vertex shader fetches
              joint_palette.joints[j.x..w] and blends position/normal/tangent (lines 277-333).
              Pool slot stride aligned to device limits().min_storage_buffer_offset_alignment
              (skinning_gpu.rs:55-57).
    │
    ▼
[F: Renderer dispatches the skinned draw]
    files: astraweave-render/src/renderer.rs:6744-6760 (set_skinned_mesh — v0 API),
           astraweave-render/src/renderer.rs:6762-6771 (update_skin_palette),
           astraweave-render/src/types.rs:51-106 (SkinnedVertex layout: 60 bytes /
                                                  shader_locations 0/1/10/11/12/13)
    role: Uploads skinned vbuf+ibuf into Renderer.skinned_mesh slot;
          uploads contiguous Mat4 array into Renderer.skin_palette_buf via queue.write_buffer.
```

The two-path structure (CPU default, GPU optional) plus the four-crate type federation are the load-bearing structural facts. The GPU shader at `skinning_gpu.rs:254-335` is embedded as a Rust string constant and is the only WGSL in the animation system.

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **Skeleton** | A hierarchy of joints with inverse-bind matrices. Three structurally distinct definitions exist: `astraweave_render::Skeleton { joints: Vec<Joint>, root_indices }`, `astraweave_asset::Skeleton` (same shape but with `[f32;…]` field storage on `Joint`/`Transform`), and `astraweave_scene::CSkeleton { joint_count, root_indices, parent_indices: Vec<Option<usize>>, inverse_bind_matrices: Vec<Mat4>, local_transforms: Vec<Transform> }` (parallel-array layout, no `Vec<Joint>` wrapper). The editor uses a fourth `GltfSkeleton` with `Vec<GltfJoint>`. | `astraweave-render/src/animation.rs:58-61`, `astraweave-asset/src/lib.rs:902-905`, `astraweave-scene/src/lib.rs:421-427`, `tools/aw_editor/src/viewport/types.rs:250-255` |
| **Joint** | A single bone with name, parent index, inverse-bind matrix, and bind-pose local transform. Render-side stores `Mat4` + render-side `Transform`; asset-side stores `[[f32;4];4]` + asset-side `Transform`; editor-side `GltfJoint` stores `glam::Mat4` for **both** inverse-bind and local-transform (no TRS decomposition). | `astraweave-render/src/animation.rs:16-21`, `astraweave-asset/src/lib.rs:874-880`, `tools/aw_editor/src/viewport/types.rs:238-246` |
| **Transform (TRS)** | Translation + rotation (quaternion) + scale. The render-side and asset-side `Transform` types share the same conceptual shape but differ in field representation (`Vec3`/`Quat` vs `[f32;3]`/`[f32;4]`). The scene crate also defines its own `Transform` at `astraweave-scene/src/lib.rs:37-42` used inside `CSkeleton::local_transforms`. | `astraweave-render/src/animation.rs:25-29`, `astraweave-asset/src/lib.rs:884-888`, `astraweave-scene/src/lib.rs:37-42` |
| **AnimationClip** | A named, fixed-duration set of channels animating joint properties. Render-side, asset-side, and editor-side (`GltfAnimationClip`) all use `name` + `duration` + `channels` shape but with distinct channel type definitions. | `astraweave-render/src/animation.rs:92-96`, `astraweave-asset/src/lib.rs:935-939`, `tools/aw_editor/src/viewport/types.rs:290-297` |
| **AnimationChannel** | One animated property (translation/rotation/scale) on one joint, with keyframe times + values + interpolation mode. Render-side and asset-side embed the property in a `ChannelData` discriminated union; editor-side `GltfAnimChannel` splits the property name (`GltfChannelProperty`) from the values (`Vec<Vec<f32>>`). | `astraweave-render/src/animation.rs:83-88`, `astraweave-asset/src/lib.rs:918-923`, `tools/aw_editor/src/viewport/types.rs:275-286` |
| **ChannelData** | The discriminated union variant carrying typed keyframe values: `Translation(Vec<Vec3>)`, `Rotation(Vec<Quat>)`, `Scale(Vec<Vec3>)` on render-side; same variants with `Vec<[f32;3]>`/`Vec<[f32;4]>` on asset-side. Marked `#[non_exhaustive]`. | `astraweave-render/src/animation.rs:74-79`, `astraweave-asset/src/lib.rs:910-914` |
| **Interpolation** | `Step / Linear / CubicSpline` keyframe interpolation modes. `#[non_exhaustive]`. CubicSpline falls back to Linear/Slerp in `AnimationClip::sample` (`astraweave-render/src/animation.rs:127, 140, 153`); the asset side comments "Not fully implemented yet" at `astraweave-asset/src/lib.rs:930`. | `astraweave-render/src/animation.rs:64-70`, `astraweave-asset/src/lib.rs:925-931`, `tools/aw_editor/src/viewport/types.rs:258-263` |
| **AnimationState** | Render-side per-clip playback state: `clip_index`, `time`, `speed`, `looping`, `playing`. Looping wraps via `%` (`animation.rs:236-240`); non-looping clamps and sets `playing = false` at duration (`animation.rs:242-247`). | `astraweave-render/src/animation.rs:204-267` |
| **CAnimator** | ECS-side per-entity playback state. Same conceptual role as `AnimationState` but with a `PlaybackState` enum (`Playing` / `Paused` / `Stopped`) instead of a plain `playing: bool`. Distinct type from `AnimationState`. | `astraweave-scene/src/lib.rs:493-499` |
| **PlaybackState** | ECS-side enum (`Playing`, `Paused`, `Stopped`) with helper predicates `is_playing` / `is_paused` / `is_stopped` / `is_active`. `#[non_exhaustive]`. | `astraweave-scene/src/lib.rs:437-489` |
| **EntityAnimationState** | Editor-bridge per-entity state. Same conceptual role as `AnimationState` / `CAnimator` but with `clip_index: Option<usize>` (Optional, while render-side defaults to `0`). | `tools/aw_editor/src/animation_bridge.rs:14-26` |
| **JointMatrixGPU** | `#[repr(C)]` Pod-Zeroable wrapper around `[[f32; 4]; 4]` for storage-buffer upload. 64 bytes, 16-byte aligned. | `astraweave-render/src/animation.rs:345-355` |
| **JointPalette** | `#[repr(C)]` Pod-Zeroable: `[JointMatrixGPU; MAX_JOINTS=256]` + `joint_count: u32` + 12 bytes padding. Total ~16KB per palette. | `astraweave-render/src/animation.rs:361-394`, with `MAX_JOINTS = 256` at line 358 |
| **JointPaletteHandle** | Opaque `u32`-wrapped handle returned by `JointPaletteManager::allocate()`. Maps to a pool slot via `handle_to_slot: HashMap<JointPaletteHandle, u32>`. | `astraweave-render/src/skinning_gpu.rs:13` |
| **JointPaletteManager** | Pooled SSBO manager: single large storage buffer with dynamic offsets, one slot per skeleton. Grows by doubling on demand (`grow` at `skinning_gpu.rs:198-246`). Initial capacity 64 slots (line 50). | `astraweave-render/src/skinning_gpu.rs:20-247` |
| **SkinnedVertex** | GPU vertex layout: 60 bytes = position(12) + normal(12) + tangent(16) + uv(8) + joints `u16x4`(8) + weights `f32x4`(16). Locations 0/1/10/11/12/13. | `astraweave-render/src/types.rs:49-106` |
| **SKINNING_GPU_SHADER** | Embedded WGSL string defining `JointPalette` SSBO (binding `@group(4) @binding(0)`), `SkinnedVertexInput`, and `apply_skinning` / `apply_skinning_normal` / `apply_skinning_tangent` helpers. | `astraweave-render/src/skinning_gpu.rs:254-335` |
| **CSkeleton / CSkinnedMesh / CJointMatrices / CParentBone** | ECS components. `CSkeleton` uses parallel-array layout (no `Vec<Joint>`). `CJointMatrices` carries `matrices: Vec<Mat4>` + `dirty: bool`. `CParentBone` is the bone-attachment component (child entity follows a parent joint). | `astraweave-scene/src/lib.rs:421-665` |
| **EditorAnimationBridge** | Editor-side owner of clip library, per-entity playback state, and a "preview" state used by the animation panel. Stores its own `GltfSkeleton` + `Vec<GltfAnimationClip>` per entity (`tools/aw_editor/src/animation_bridge.rs:88-91`). | `tools/aw_editor/src/animation_bridge.rs:78-…` |

### Terms to NOT confuse

- **`Skeleton`** has FOUR definitions in this codebase. `astraweave_render::Skeleton` (canonical runtime), `astraweave_asset::Skeleton` (asset-side, float-array), `astraweave_scene::CSkeleton` (ECS, parallel-array), `aw_editor::viewport::types::GltfSkeleton` (editor-side, glam types). All four are structurally distinct. Always namespace explicitly.
- **`Joint`** similarly has FOUR definitions. The editor's `GltfJoint` uniquely stores `local_transform: glam::Mat4` instead of TRS components (`tools/aw_editor/src/viewport/types.rs:245`).
- **`Transform`** has THREE definitions. `astraweave_render::animation::Transform` (`Vec3`/`Quat`/`Vec3`), `astraweave_asset::Transform` (`[f32;3]`/`[f32;4]`/`[f32;3]`), and `astraweave_scene::Transform` (`Vec3`/`Quat`/`Vec3` — same shape as render-side but a different Rust type at `astraweave-scene/src/lib.rs:37-42`). They are not interchangeable across crate boundaries without explicit conversion.
- **`AnimationClip`** has THREE definitions: render-side, asset-side, and `GltfAnimationClip`. Same conceptual shape but distinct types.
- **`AnimationChannel`** has THREE definitions, with the editor's `GltfAnimChannel` being structurally different: it separates the property *kind* (`GltfChannelProperty::Translation/Rotation/Scale`) from the values (`Vec<Vec<f32>>`, weakly typed). The render-side and asset-side encode the property *in* `ChannelData` as a discriminated union.
- **`Interpolation`** has THREE definitions; the render-side and asset-side both note "CubicSpline not fully implemented" / falls back to Linear (`animation.rs:127, 140, 153`; `astraweave-asset/src/lib.rs:930`).
- **`AnimationState` (render) vs `CAnimator` (scene) vs `EntityAnimationState` (editor bridge)**: three different per-entity playback-state types. Render-side uses `playing: bool`; scene-side uses `PlaybackState` enum; editor-bridge uses `playing: bool` and an `Option<usize>` clip index. Each system maintains its own.
- **Render `MAX_JOINTS = 256`** is a hard limit on per-skeleton joints in the GPU palette (`astraweave-render/src/animation.rs:358`). Skeletons with more joints than this would have surplus joints silently dropped at `JointPalette::from_matrices` (line 388 `take(MAX_JOINTS)`).

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| `gltf` crate (v?) | `Gltf::from_slice`, `gltf::binary::Glb::from_slice`, `gltf::animation::{Property, Interpolation}` | Skin / animation data parsed from glTF | Consumed by `astraweave_asset::load_skeleton` and `load_animations`. Property translation at `astraweave-asset/src/lib.rs:1171-1180` (`gltf::animation::Property::Translation/Rotation/Scale` → asset-side `ChannelData`) |
| `glam` 0.x | `Vec3`, `Quat`, `Mat4`, `Mat3` | Math primitives | Used throughout render-side animation and skinning paths |
| `bytemuck` | `Pod`, `Zeroable` derives | Trait impls for GPU-uploadable structs | `JointMatrixGPU`, `JointPalette`, `SkinnedVertex` |
| `wgpu` 25 | `Device`, `Queue`, `BindGroupLayout`, `Buffer`, `BindGroup` | GPU resources | Owned by `JointPaletteManager`; pool buffer at `skinning_gpu.rs:74-81` |
| Editor UI events | `AnimationAction` enum (referenced from `crate::panels::animation_panel`) | User playback control | Consumed by `EditorAnimationBridge::process_action` [NEEDS VERIFICATION on exact method name; only the type import is visible at `animation_bridge.rs:6`] |

### Downstream (what consumes this system's output)

| Consumer | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-render::Renderer` | `set_skinned_mesh(&[SkinnedVertex], &[u32])` at `renderer.rs:6744`; `update_skin_palette(&[Mat4])` at `renderer.rs:6762` | Skinned vbuf+ibuf and a contiguous Mat4 palette | The Renderer's skinned-mesh API is v0 (annotated `// --- Skinning API (v0) ---` at `renderer.rs:6743`). One skinned mesh slot per Renderer (`self.skinned_mesh: Option<(vbuf, ibuf, count)>`) |
| `examples/skinning_demo` | `use astraweave_render::animation::*;` at `skinning_demo/src/main.rs:6` | Skeleton + AnimationClip directly | Interactive demo with CPU/GPU toggle, play/pause, speed control. Per `examples/skinning_demo/src/main.rs:1-23` |
| Integration tests | `skinning_rest_pose_golden.rs`, `skinning_pose_frame_golden.rs`, `skinning_integration.rs`, `skinning_parity_cpu_vs_gpu.rs`, `skinning_stress_many_entities.rs` | Test fixtures using runtime types | 305+330+262+642+472 LoC total |
| `astraweave-scene` ECS systems | `update_animations(world, dt, clip_durations)` at `astraweave-scene/src/lib.rs:896`; `compute_poses_stub(world)` at line 941 | Per-tick advance of `CAnimator::time` | `compute_poses_stub` is a stub (per its name); it checks for presence of `CSkeleton` + `CJointMatrices` but full pose-write is gated. [NEEDS VERIFICATION on whether stub is wired into a complete pose-compute path elsewhere] |
| `tools/aw_editor::EditorAnimationBridge` | `EditorAnimationBridge::tick(dt)`, `process_action(AnimationAction)`, etc. | Per-entity animation state + computed joint matrices | The bridge owns the clip library and per-entity state independently of the runtime |
| `tools/aw_editor::AnimationPanel` | `EditorAnimationBridge` methods | UI events | 3,680 LoC panel at `tools/aw_editor/src/panels/animation_panel.rs` |
| `astraweave-render::nanite_*` (feature `nanite`) | [NEEDS VERIFICATION — `wave2_nanite_anim_prim_remediation.rs` test file suggests integration but the exact interface was not traced in this pass] | Possibly skinned LOD vertices | |

### Bidirectional / Coupled

- **`JointPaletteManager` ↔ `Renderer`**: When the `skinning-gpu` feature is active, the manager owns the pooled SSBO and the shared `bind_group`. Renderer-side pipeline consumes it at group(4) with `has_dynamic_offset=true`. Pool growth (`grow`, `skinning_gpu.rs:198-246`) copies existing data to a doubled buffer and rebuilds the bind group; any cached external references to the old bind group are invalidated.
- **`astraweave-asset` ↔ `astraweave-render::animation`**: Asset-side types are independent of render-side types. Conversion happens at the call site (typically in scene loaders or examples). No canonical `From`/`Into` impl was found between the two type families in this pass.
- **`astraweave-scene::CSkeleton` ↔ `astraweave-render::Skeleton`**: Same conceptual data, different physical layout. `CSkeleton` uses parallel arrays; `Skeleton` uses `Vec<Joint>`. Conversion happens at the ECS-spawn site.
- **`EditorAnimationBridge` ↔ runtime animation**: The bridge maintains its own clip library, per-entity skeletons (`GltfSkeleton`), per-entity clips (`Vec<GltfAnimationClip>`), and computed `joint_matrices: Vec<Mat4>`. The matrices it computes flow back into the editor's renderer adapter; the bridge does not feed `Renderer::update_skin_palette` directly [NEEDS VERIFICATION on exact wiring].

---

## 5. Active File Map

### `astraweave-render` — canonical runtime

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-render/src/animation.rs` | Canonical runtime types + CPU pipeline (`Skeleton`, `Joint`, `Transform`, `AnimationClip`, `AnimationChannel`, `ChannelData`, `Interpolation`, `AnimationState`, `JointMatrixGPU`, `JointPalette`, `MAX_JOINTS`, `compute_joint_matrices`, `skin_vertex_cpu`) plus 13 inline unit tests | Active | 710 LoC. Re-exported at crate root via `lib.rs:272-276` |
| `astraweave-render/src/skinning_gpu.rs` | `JointPaletteManager` pooled SSBO + embedded `SKINNING_GPU_SHADER` WGSL + 8 inline tests | Active (feature `skinning-gpu`) | 814 LoC. Re-exported at `lib.rs:278-279` (feature-gated) |
| `astraweave-render/src/animation_extra_tests.rs` | 29 additional unit tests (mutation-killing coverage) | Active (test gate) | 613 LoC; declared `#[cfg(test)]` at `lib.rs:126-127` |
| `astraweave-render/src/types.rs:49-106` | `SkinnedVertex` GPU vertex struct + `layout()` | Active | 60-byte vertex; shader locations 0/1/10/11/12/13 |
| `astraweave-render/src/renderer.rs:6744-6760` | `Renderer::set_skinned_mesh(&[SkinnedVertex], &[u32])` | Active (v0 API per inline comment at line 6743) | Stores one skinned mesh in `Renderer.skinned_mesh` |
| `astraweave-render/src/renderer.rs:6762-6771` | `Renderer::update_skin_palette(&[Mat4])` | Active | Uploads contiguous Mat4 array via `queue.write_buffer` |
| `astraweave-render/tests/skinning_rest_pose_golden.rs` | Rest-pose correctness golden tests | Active | 262 LoC |
| `astraweave-render/tests/skinning_pose_frame_golden.rs` | Animated-frame correctness golden tests | Active | 330 LoC |
| `astraweave-render/tests/skinning_integration.rs` | End-to-end integration tests | Active | 305 LoC |
| `astraweave-render/tests/skinning_parity_cpu_vs_gpu.rs` | CPU vs GPU equivalence tests; `#[ignore]` for CI (require GPU) | Active (feature `skinning-gpu`, --ignored gate) | 642 LoC; module comment at lines 6-7 documents the gate |
| `astraweave-render/tests/skinning_stress_many_entities.rs` | Stress tests (100-2000 entities) | Active | 472 LoC |
| `astraweave-render/tests/wave2_animation_remediation.rs` | Wave 2 mutation-remediation tests | Active | 848 LoC |
| `astraweave-render/tests/wave2_culling_anim_misc_tests.rs` | Wave 2 misc tests touching animation | Active | |
| `astraweave-render/tests/wave2_nanite_anim_prim_remediation.rs` | Wave 2 Nanite + animation + primitive tests | Active (likely feature `nanite`-gated) | [NEEDS VERIFICATION on feature gating] |
| `astraweave-render/tests/wave2_extended_coverage_tests.rs` | Mixed Wave 2 coverage including animation | Active | |
| `astraweave-render/tests/wave2_asset_index_biome_skinning_remediation.rs` | Wave 2 tests crossing asset / biome / skinning | Active | |
| `astraweave-render/benches/animation_skinning.rs` | Criterion benchmarks for transform / clip-sample / hierarchy / palette / blending | Active | 965 LoC; correctness assertions at file header (lines 13-23) |
| `astraweave-render/benches/gpu_memory_terrain_skinning_depth_overlay.rs` | Cross-cutting GPU-memory benchmarks including skinning | Active | 1,570 LoC |

### `astraweave-asset` — glTF import

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-asset/src/lib.rs:854-…` | Asset-side `Skeleton`, `Joint`, `Transform`, `AnimationClip`, `AnimationChannel`, `ChannelData`, `Interpolation`, `SkinnedMeshData`, `SkinnedVertexLite` plus `load_skeleton` and `load_animations` functions | Active | Phase 2 Task 5 Phase A. `load_animations` translates `gltf::animation::Property` and `Interpolation` enums at `lib.rs:1171-1180` |

### `astraweave-scene` — ECS integration

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-scene/src/lib.rs:37-42` | `Transform` (scene-crate definition; same TRS shape as render-side but distinct type) | Active | |
| `astraweave-scene/src/lib.rs:415-665` | ECS animation components: `CSkeleton` (parallel-array storage), `CSkinnedMesh`, `PlaybackState` (Playing/Paused/Stopped), `CAnimator`, `CJointMatrices`, `CParentBone` | Active | Phase 2 Task 5 Phase C |
| `astraweave-scene/src/lib.rs:896` | `update_animations(world, dt, clip_durations)` system | Active | Advances `CAnimator::time`; ECS-side counterpart to `AnimationState::update` |
| `astraweave-scene/src/lib.rs:941` | `compute_poses_stub(world)` | Active | Module name says "stub"; checks for presence of `CSkeleton` + `CJointMatrices` and is wired for future pose-compute completion. [NEEDS VERIFICATION on whether a complete pose-compute system exists elsewhere or this is the only path] |
| `astraweave-scene/tests/bone_attachment_integration.rs` | Bone-attachment integration tests | Active | Per grep |

### `tools/aw_editor` — editor UI + bridge

| File | Role | Status | Notes |
|---|---|---|---|
| `tools/aw_editor/src/viewport/types.rs:238-298` | Editor-side `GltfSkeleton`, `GltfJoint`, `GltfInterpolation`, `GltfChannelProperty`, `GltfAnimChannel`, `GltfAnimationClip` | Active | `GltfJoint` uniquely stores `local_transform: glam::Mat4` (line 245) without TRS decomposition |
| `tools/aw_editor/src/animation_bridge.rs` | `EditorAnimationBridge`, `EntityAnimationState`, `ClipEntry`, `AnimationOutput`, `JointTransform` | Active | 451 LoC. Bridges UI events to per-entity playback + per-entity GltfSkeleton/GltfAnimationClip libraries |
| `tools/aw_editor/src/panels/animation_panel.rs` | Animation UI panel | Active | 3,680 LoC — second-largest editor panel |
| `tools/aw_editor/src/panels/animation.rs` | Smaller animation panel module | Active | |
| `tools/aw_editor/src/subsystems/audio_animation.rs` | Audio-coupled animation subsystem | Active | |
| `tools/aw_editor/src/console_bridge.rs` | Console commands touching animation | Active | Per grep |
| `tools/aw_editor/tests/wave2_animation_easing_tests.rs` | Editor-side animation easing tests | Active | |

### Examples

| File | Role | Status | Notes |
|---|---|---|---|
| `examples/skinning_demo/src/main.rs` | Interactive CPU/GPU skinning demo | Active | Per `main.rs:1-23`: keyboard controls Space/[ /] /R /G /ESC; uses `astraweave_render::animation::*` directly |
| `examples/astract_gallery/src/animation_tab.rs` | Animation-tab demo in astract gallery | Active | |

**Status definitions used here:**
- **Active**: Canonical, load-bearing, edit with care
- **Active (feature `X`)**: Compiles only when the named Cargo feature is enabled
- **Active (test gate)**: Inside `#[cfg(test)]`
- **Stub**: Function/system exists with a placeholder implementation acknowledged in its name or comment

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Notes |
|---|---|---|---|
| Four `Skeleton` definitions | `astraweave_render::Skeleton`, `astraweave_asset::Skeleton`, `astraweave_scene::CSkeleton`, `aw_editor::viewport::types::GltfSkeleton` | Coexisting | Each crate has its own representation optimized for its purpose: glam types for runtime math, float arrays for asset serialization, parallel arrays for ECS, glam types for editor UI |
| Four `Joint` definitions | Same four crates | Coexisting | Editor's `GltfJoint` uniquely stores `local_transform: glam::Mat4` instead of TRS components (`viewport/types.rs:245`) |
| Three `Transform` definitions | `astraweave_render::animation::Transform`, `astraweave_asset::Transform`, `astraweave_scene::Transform` | Coexisting | Render and scene share TRS shape with `Vec3`/`Quat`/`Vec3` fields but are distinct Rust types |
| Three `AnimationClip` definitions | render / asset / editor (`GltfAnimationClip`) | Coexisting | Same `name`/`duration`/`channels` shape, distinct channel types |
| Three `AnimationChannel` definitions | render / asset / editor (`GltfAnimChannel`) | Coexisting | Editor's `GltfAnimChannel` separates property kind from values; render+asset embed property in `ChannelData` discriminated union |
| Three `Interpolation` definitions | render / asset / editor (`GltfInterpolation`) | Coexisting | All three carry the same three variants (`Step`/`Linear`/`CubicSpline`). CubicSpline falls back to Linear/Slerp in render-side `AnimationClip::sample` (lines 127, 140, 153); asset-side comments "Not fully implemented yet" (`lib.rs:930`) |
| Three per-entity playback states | `AnimationState` (render), `CAnimator` + `PlaybackState` (scene), `EntityAnimationState` (editor bridge) | Coexisting | Same conceptual data; different field choices (`playing: bool` vs `PlaybackState` enum; `clip_index: usize` vs `Option<usize>`) |
| Asset-side `SkinnedVertexLite` vs render-side `SkinnedVertex` | `astraweave-asset/src/lib.rs:857-864` vs `astraweave-render/src/types.rs:51-58` | Coexisting | Asset-side uses `[f32; …]` plus `joints: [u16; 4]` and `weights: [f32; 4]` for serialization; render-side has the same fields but as a `#[repr(C)] Pod Zeroable` struct with an explicit GPU `layout()` |
| Render-side `Renderer::set_skinned_mesh` "v0 API" vs newer pooled `JointPaletteManager` | `renderer.rs:6743-6771` vs `skinning_gpu.rs:20-…` | Coexisting | Inline comment at `renderer.rs:6743` labels the simple API as "v0"; the pooled SSBO with dynamic offsets is the modern path. Both are active. [NEEDS VERIFICATION on whether v0 has documented planned deprecation] |
| `compute_poses_stub` (scene ECS, named "stub") vs `compute_joint_matrices` (render canonical) | `astraweave-scene/src/lib.rs:941` vs `astraweave-render/src/animation.rs:274-336` | Coexisting | The render-side function is the canonical hierarchical pose-compute. The scene-side `compute_poses_stub` checks ECS component presence but does not (per its name) replicate the full pose computation. Whether it's intentional that pose compute lives entirely on the render-side is unrecorded |
| CPU skinning (default) vs GPU skinning (`skinning-gpu` feature) | `astraweave-render/src/animation.rs:401-435` vs `astraweave-render/src/skinning_gpu.rs:…` | Coexisting (intentional, by feature flag) | Documented in `lib.rs:117-118` (`#[cfg(feature = "skinning-gpu")]` module gate). CPU is the deterministic CI-safe default per `animation.rs:1-6` module header |

### Naming collisions

- **`Skeleton`** — collides across `astraweave_render::Skeleton`, `astraweave_asset::Skeleton`, `astraweave_scene::CSkeleton` (different name, same concept), `aw_editor::viewport::types::GltfSkeleton`. Always namespace explicitly.
- **`Joint`** — collides across `astraweave_render::Joint`, `astraweave_asset::Joint`, `aw_editor::viewport::types::GltfJoint`.
- **`Transform`** — collides across `astraweave_render::animation::Transform`, `astraweave_asset::Transform`, `astraweave_scene::Transform`. The scene-crate `Transform` is also used by non-animation scene code (it's the crate's root scene-graph TRS).
- **`AnimationClip`** — render / asset / editor (`GltfAnimationClip`).
- **`AnimationChannel`** — render / asset / editor (`GltfAnimChannel`).
- **`Interpolation`** — render / asset / editor (`GltfInterpolation`).
- **`ChannelData`** — render / asset.
- **`AnimationState` vs `CAnimator` vs `EntityAnimationState`** — three names for the same conceptual playback record in three crates.
- **"Skinning"** — refers to CPU skinning in `animation.rs::skin_vertex_cpu`, GPU skinning in `skinning_gpu.rs::SKINNING_GPU_SHADER`, and the v0 Renderer API at `renderer.rs:6743-6771`. Always specify which path.

### Known cognitive traps

- **Trap**: Treating any one of the four `Skeleton` types as the canonical type and passing it across crate boundaries without conversion.
  **What's actually true**: Each crate uses its own definition. The asset-side loader returns `astraweave_asset::Skeleton`; the runtime expects `astraweave_render::Skeleton`; the ECS expects `astraweave_scene::CSkeleton`; the editor expects `aw_editor::viewport::types::GltfSkeleton`. Each boundary needs explicit conversion — none of the four types implement `From`/`Into` for any of the others (verified by grep across the four crates for `impl From<` involving these names).
- **Trap**: Assuming `AnimationClip::sample` interpolates `CubicSpline` keyframes correctly.
  **What's actually true**: `astraweave-render/src/animation.rs:127, 140, 153` explicitly fall back to Linear (for translations and scales) or Slerp (for rotations) when the interpolation is `CubicSpline`. The asset-side comment at `astraweave-asset/src/lib.rs:930` confirms "Not fully implemented yet". A glTF clip authored with cubic-spline keyframes will play back as linear in this engine until that gap is closed.
- **Trap**: Skeletons with more than 256 joints.
  **What's actually true**: `JointPalette::from_matrices` silently drops surplus joints via `take(MAX_JOINTS)` at `animation.rs:388`. No error is returned. `MAX_JOINTS = 256` is hard-coded at `animation.rs:358` and is also the size baked into the GPU SSBO struct at `skinning_gpu.rs:261`.
- **Trap**: Calling `compute_joint_matrices` on a skeleton whose `root_indices` is empty or whose joint parent chain has cycles.
  **What's actually true**: Cycle detection exists (`animation.rs:289-294` returns `anyhow::Error` with the offending joint index). Empty `root_indices` returns an all-identity `Vec<Mat4>` (because the inner `compute_recursive` is never called). The "invalid parent index" test at `animation.rs:680-709` documents that out-of-bounds parents are treated as orphans (the child becomes detached rather than panicking).
- **Trap**: Using `compute_poses_stub` (`astraweave-scene/src/lib.rs:941`) and expecting it to populate `CJointMatrices::matrices`.
  **What's actually true**: The function name says "stub". It checks for presence of `CSkeleton` + `CJointMatrices` but does not (per its name) write the full pose. [NEEDS VERIFICATION on whether a full pose-compute system exists elsewhere in `astraweave-scene` or whether ECS-side pose computation is incomplete.]
- **Trap**: Treating `Renderer::set_skinned_mesh` and `JointPaletteManager` as the same API.
  **What's actually true**: They are two different ways to feed skinned data to the renderer. `set_skinned_mesh` (v0 per its inline comment at `renderer.rs:6743`) takes a `&[SkinnedVertex]` + `&[u32]` and uploads a contiguous Mat4 palette via `update_skin_palette`. `JointPaletteManager` (modern, pooled) takes `JointPaletteHandle`s and exposes a shared bind group with dynamic offsets. The two paths likely coexist for backward compatibility.

---

## 7. Decision Log

### Decision: CPU skinning is the deterministic default; GPU skinning is opt-in via feature flag
- **Date:** Phase 2 Task 5 Phase D, completed 2025-10-01 per `docs/archive/phase_reports/PHASE2_TASK5_IMPLEMENTATION_SUMMARY.md`
- **Status:** Accepted (in active code)
- **Context:** The module header at `astraweave-render/src/animation.rs:1-6` documents the rationale: "CPU skinning: Default, CI-safe, computes skinned vertices on CPU; GPU skinning: Optional feature flag, computes skinning in vertex shader." The Phase 2 Task 5 implementation summary (line 195-200) frames it as "Deterministic, CI-safe" vs "Hardware-accelerated, optional". CI-safety is the explicit driver — tests that depend on exact vertex output run deterministically on the CPU path; the GPU parity tests at `astraweave-render/tests/skinning_parity_cpu_vs_gpu.rs` are `#[ignore]`d for CI per the module header at lines 6-7.
- **Decision:** CPU path lives in `animation.rs:401-435` (`skin_vertex_cpu`); GPU path lives in `skinning_gpu.rs` behind the `skinning-gpu` Cargo feature.
- **Alternatives considered:** [Reasoning not recovered from available sources — but the implementation summary at PHASE_D mentions only "Pipeline implemented, compute dispatch integration pending" suggesting GPU was always seen as an addition, not a replacement.]
- **Consequences:** Two parallel skinning implementations. CPU is required for golden tests and deterministic playback; GPU is required for high-entity-count scenes. Parity testing (`skinning_parity_cpu_vs_gpu.rs`) bridges them but is `--ignored` for CI.

### Decision: Pooled SSBO with dynamic offsets for GPU joint palettes
- **Date:** Not directly recovered; module-level docstring at `astraweave-render/src/skinning_gpu.rs:14-19` documents the rationale.
- **Status:** Accepted (in active code, feature `skinning-gpu`)
- **Context:** The docstring at `skinning_gpu.rs:16-19` says: "Uses a single large SSBO with dynamic offsets instead of per-skeleton individual buffers. This eliminates per-skeleton buffer/bind-group allocation overhead and enables efficient draw-call batching." Pool growth doubles capacity (`grow` at lines 198-246) and copies the existing buffer to a new larger one, rebuilds the bind group (lines 233-245).
- **Decision:** Single `pool_buffer` of `slot_stride * pool_capacity` bytes; `slot_stride` rounded up to `device.limits().min_storage_buffer_offset_alignment` (`skinning_gpu.rs:55-57`); per-handle slot allocation from a LIFO `free_slots: Vec<u32>`.
- **Alternatives considered:** Per-skeleton individual buffers — explicitly framed as the rejected alternative in the docstring at lines 16-19.
- **Consequences:** One bind group rebinding per draw with a `dynamic_offset` rather than per-skeleton bind-group switching. Initial pool capacity is 64 slots (`INITIAL_POOL_CAPACITY` at line 50); growth doubles. Each slot is `slot_stride` bytes (≥16 KB for `JointPalette` size 16,396 bytes rounded up to alignment).

### Decision: `MAX_JOINTS = 256` as the per-skeleton GPU palette limit
- **Date:** Phase 2 Task 5 Phase D
- **Status:** Accepted (in active code)
- **Context:** Defined at `astraweave-render/src/animation.rs:358` and consumed by `JointPalette` (line 364) and `JointPalette::from_matrices` (line 388). The WGSL shader hard-codes the same constant: `joints: array<JointMatrix, 256>` at `skinning_gpu.rs:261`. Phase 2 Task 5 implementation summary line 180 confirms the constant.
- **Decision:** Skeletons may have at most 256 joints; surplus joints are silently dropped via `.take(MAX_JOINTS)` in `JointPalette::from_matrices` (`animation.rs:388`).
- **Alternatives considered:** [Reasoning not recovered.]
- **Consequences:** A `JointPalette` is 16,396 bytes (256 × 64 + 12 padding). Skeletons over 256 joints lose data silently. The constant must be kept in sync between Rust (`animation.rs:358`) and WGSL (`skinning_gpu.rs:261`) by manual edit — there is no compile-time mirror.

### Decision: Four parallel type families across crate boundaries (rather than a shared crate)
- **Date:** Phase 2 Task 5 (asset/render/scene types landed together)
- **Status:** Accepted (in active code; transitional disposition undocumented)
- **Context:** Asset-side types use `[f32; …]` for Pod-friendly serialization; runtime types use `glam` for math; ECS types use parallel arrays for efficient component access; editor types use `glam::Mat4` for both transforms and inverse-bind matrices (no TRS decomposition). Each crate's choice is internally consistent.
- **Decision:** Each crate defines its own `Skeleton` / `Joint` / `Transform` / `AnimationClip` / `AnimationChannel` / `ChannelData` / `Interpolation` types; conversions happen at integration points.
- **Alternatives considered:** [Reasoning not recovered from available sources — no audit doc or campaign doc was found discussing the alternative of a shared `astraweave-animation` crate.]
- **Consequences:** Adding or modifying an animation field requires touching up to four crates plus any conversion sites. The risk profile maps directly onto the §7.7 wrapped-component resource identity trap surfaced in the broader render-pipeline audits (see `docs/architecture/render_pipeline_material_system_shader_infrastructure.md` §6 traps and `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Status header).

### Decision: Bone attachment via `CParentBone` ECS component
- **Date:** Phase 2 Task 5 Phase C, completed 2025-10-01
- **Status:** Accepted (in active code)
- **Context:** Phase 2 Task 5 implementation summary describes "Bone attachment (child entities follow joints)". `CParentBone` is defined at `astraweave-scene/src/lib.rs:629`; integration test at `astraweave-scene/tests/bone_attachment_integration.rs`.
- **Decision:** Bone attachment is a first-class ECS concept; child entities carry a `CParentBone { parent_entity, joint_index }` component and are updated by `update_bone_attachments`.
- **Alternatives considered:** [Reasoning not recovered.]
- **Consequences:** Bone attachment lives at the ECS layer rather than inside the animation runtime. A bone attachment without a parent that has a `CJointMatrices` will silently fail to update [NEEDS VERIFICATION on exact error/silent behavior].

### Decision: `Renderer::set_skinned_mesh` is "v0" — coexists with pooled `JointPaletteManager`
- **Date:** Not recovered; inline comment at `astraweave-render/src/renderer.rs:6743` labels it `// --- Skinning API (v0) ---`
- **Status:** Accepted (in active code, transitional)
- **Context:** Renderer has a single `skinned_mesh: Option<(vbuf, ibuf, count)>` slot plus a `skin_palette_buf`. The v0 API uploads a contiguous `Mat4` array (`update_skin_palette`, line 6762). The pooled SSBO path in `skinning_gpu.rs` is the multi-skeleton modern alternative.
- **Decision:** Keep the v0 simple API for single-skeleton scenarios; use the pooled manager when many skeletons need to be in flight simultaneously.
- **Alternatives considered:** [Reasoning not recovered.]
- **Consequences:** Two ways to feed GPU skinning data exist concurrently; callers must choose. No deprecation path was found documented in this pass.

### Decision: Editor maintains its own `EditorAnimationBridge` rather than reusing scene-crate ECS
- **Date:** Not recovered
- **Status:** Accepted (in active code)
- **Context:** The bridge at `tools/aw_editor/src/animation_bridge.rs:78-…` owns its own clip library, per-entity `EntityAnimationState`, per-entity `GltfSkeleton`, per-entity `Vec<GltfAnimationClip>`, and computed `joint_matrices` per frame. It uses the editor's `GltfAnimationClip` / `GltfSkeleton` types from `viewport/types.rs`, not the render-side or scene-side types.
- **Decision:** Editor has its own animation state model and its own type representation, parallel to the runtime/scene path.
- **Alternatives considered:** [Reasoning not recovered.]
- **Consequences:** Editor animation behavior may diverge from runtime if the bridge's playback math is not kept in sync with `AnimationState::update` / `AnimationClip::sample`. No automated parity test between editor bridge and runtime was found in this pass.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `MAX_JOINTS = 256` is the per-skeleton GPU palette limit; the WGSL shader and the Rust constant must match | Manual | `animation.rs:358` (Rust) ↔ `skinning_gpu.rs:261` (WGSL string) — kept in sync by manual edit; no compile-time check |
| 2 | `JointPalette::from_matrices` silently drops joints beyond `MAX_JOINTS` (no error returned) | Yes | `animation.rs:388` (`.take(MAX_JOINTS)`); test coverage [NEEDS VERIFICATION] |
| 3 | `compute_joint_matrices` detects cycles in the joint hierarchy and returns `anyhow::Error` | Yes | `animation.rs:289-294` (`if !visited.insert(joint_idx) { return Err(...) }`); test in `animation_extra_tests.rs` [NEEDS VERIFICATION on exact test name] |
| 4 | `AnimationState::update` wraps time modulo clip duration when `looping`, and clamps + stops at duration when not | Yes | `animation.rs:233-247`; tests `test_animation_state_update_looping` (line 464) and `test_animation_state_update_clamping` (line 479) |
| 5 | CubicSpline interpolation is NOT fully implemented; falls back to Linear/Slerp | Yes (compile-time fall-through) | `animation.rs:127, 140, 153`; asset-side comment at `astraweave-asset/src/lib.rs:930` ("Not fully implemented yet") |
| 6 | CPU and GPU skinning produce equivalent results within floating-point tolerance | Yes (test) | `skinning_parity_cpu_vs_gpu.rs` (642 LoC, `--ignored` for CI per file header) |
| 7 | `JointPaletteManager` grows by doubling and preserves existing data; bind group is rebuilt after growth | Yes | `skinning_gpu.rs:198-246`; growth-test [NEEDS VERIFICATION on exact test name] |
| 8 | The pooled SSBO slot stride equals `JointPalette` size rounded up to `device.limits().min_storage_buffer_offset_alignment` | Yes | `skinning_gpu.rs:55-57` (`slot_stride = raw_size.div_ceil(alignment) * alignment`) |
| 9 | `skin_vertex_cpu` normalizes the skinned normal (or returns zero) | Yes | `animation.rs:434` (`skinned_normal.normalize_or_zero()`) |
| 10 | `AnimationClip::sample` returns one `Transform` per skeleton joint, initialized to the joint's bind-pose `local_transform` and then overwritten by active channels | Yes | `animation.rs:104-107, 162-165` |
| 11 | Vertices with `weights[i] == 0.0` skip the corresponding joint matrix multiply (early-continue) | Yes | `animation.rs:413-415` (`if weight == 0.0 { continue; }`) |
| 12 | Out-of-range joint indices in CPU skinning skip the contribution (no panic) | Yes | `animation.rs:417-420` (`if joint_idx >= joint_matrices.len() { continue; }`) |
| 13 | Animation crate has no `unsafe` code on the render side (crate-level `#![deny(unsafe_code)]`) and the scene crate is `#![forbid(unsafe_code)]` | Yes | `astraweave-render/src/lib.rs:1` (`#![deny(unsafe_code)]`); `astraweave-scene/src/lib.rs:1` (`#![forbid(unsafe_code)]`) |

---

## 9. Performance & Resource Profile

### Hot paths
- **`AnimationClip::sample`**: per active animator per tick. Cost = O(channels × log keyframes) via the binary search at `animation.rs:187-192`. For a 30-bone skeleton with 90 channels (TRS per bone) and 30 keyframes per channel, ~270 binary searches per sample.
- **`compute_joint_matrices`**: per skeleton per tick. Cost = O(joints) for the recursive walk + O(joints) for inverse-bind multiply at the end (`animation.rs:330-333`).
- **CPU `skin_vertex_cpu`**: per vertex per tick when CPU skinning is active. Cost = O(4 × matrix-vector mults) per vertex. For a 30k-vertex mesh: ~120k matrix-vector mults per frame.
- **GPU skinning vertex shader**: per vertex per draw, but executed on the GPU in parallel. The shader at `skinning_gpu.rs:277-296` does 4 mat4×vec4 mults per vertex.
- **`JointPaletteManager::upload_matrices`**: per skeleton per frame when joint matrices change. One `queue.write_buffer` call writes the entire palette (~16 KB).

### Cold paths
- **glTF skeleton + animation load**: at asset-load time only. `astraweave-asset::load_skeleton` and `load_animations` parse the entire glTF document; not per-frame.
- **`JointPaletteManager::grow`**: amortized O(1) on top of doubling; the actual `copy_buffer_to_buffer` + bind-group rebuild happens infrequently.
- **`Renderer::set_skinned_mesh`**: only when the skinned mesh changes — creates new `vbuf` + `ibuf` GPU buffers (`renderer.rs:6745-6759`).

### Resource ownership
- **`Skeleton` / `AnimationClip` (runtime)**: typically owned by the caller (example main, ECS resource, or editor bridge). The render crate provides no central registry.
- **`JointPaletteManager.pool_buffer`**: owned by the manager. Lifetime = manager lifetime. Grows on demand.
- **`JointPaletteHandle → slot` map** + **free slot stack**: owned by the manager.
- **`Renderer.skinned_mesh`** and **`Renderer.skin_palette_buf`**: owned by the Renderer for the v0 single-skeleton path.
- **ECS components** (`CSkeleton`, `CAnimator`, `CJointMatrices`, `CParentBone`, `CSkinnedMesh`): owned by the ECS world per entity.
- **Editor `EditorAnimationBridge`**: owned by the editor's `EditorApp`. Lifetime = editor session.

### Bench coverage
- `astraweave-render/benches/animation_skinning.rs` (965 LoC): transform / clip-sample / hierarchy / palette / blending benchmarks. Includes correctness assertions at file head (lines 13-23): quaternion-normalized check, transform-matrix-finite check, interpolation-bounds check, joint-hierarchy validity, blend-weights-sum-to-1.0 check.
- `astraweave-render/benches/gpu_memory_terrain_skinning_depth_overlay.rs` (1,570 LoC): cross-cutting GPU-memory benchmarks including skinning.

---

## 10. Testing & Validation

- **Inline `#[cfg(test)]` modules (counted via grep for `#[test]`):**
  - `astraweave-render/src/animation.rs` — 13 tests covering transforms, animation state, keyframe search, joint matrices, CPU skinning, joint palette, edge cases (invalid parent, cycle detection)
  - `astraweave-render/src/skinning_gpu.rs` — 8 tests (presence verified via grep)
  - `astraweave-render/src/animation_extra_tests.rs` — 29 additional tests (mutation-killing coverage)
- **Dedicated integration tests under `astraweave-render/tests/`:**
  - `skinning_rest_pose_golden.rs` (262 LoC) — rest-pose correctness with TRS combinations and parent-child propagation
  - `skinning_pose_frame_golden.rs` (330 LoC) — animated-frame correctness
  - `skinning_integration.rs` (305 LoC) — end-to-end integration
  - `skinning_parity_cpu_vs_gpu.rs` (642 LoC) — CPU vs GPU equivalence; `#[ignore]`d for CI per file header (lines 6-7); run locally with `cargo test -p astraweave-render --tests --features skinning-gpu -- --ignored`
  - `skinning_stress_many_entities.rs` (472 LoC) — 100-2000 entity load tests
- **Wave 2 mutation-remediation tests:**
  - `astraweave-render/tests/wave2_animation_remediation.rs` (848 LoC)
  - `astraweave-render/tests/wave2_culling_anim_misc_tests.rs`
  - `astraweave-render/tests/wave2_nanite_anim_prim_remediation.rs`
  - `astraweave-render/tests/wave2_extended_coverage_tests.rs`
  - `astraweave-render/tests/wave2_asset_index_biome_skinning_remediation.rs`
- **Scene ECS integration tests:** `astraweave-scene/tests/bone_attachment_integration.rs`
- **Editor tests:** `tools/aw_editor/tests/wave2_animation_easing_tests.rs`
- **Benchmarks:** `astraweave-render/benches/animation_skinning.rs` (965 LoC), `gpu_memory_terrain_skinning_depth_overlay.rs` (1,570 LoC) — both include explicit correctness assertions on quaternion normalization, finite matrices, and blend-weight sums
- **Interactive validation:** `examples/skinning_demo/src/main.rs` with keyboard controls (Space play/pause, `[`/`]` speed, R reset, G CPU/GPU toggle, ESC exit)
- **Formal verification:** No Kani proof files were found specifically for animation. The animation system relies on test coverage and the Rust type system (notably `#![deny(unsafe_code)]` on the render crate and `#![forbid(unsafe_code)]` on the scene crate).

---

## 11. Open Questions / Parked Decisions

- **Should the four parallel `Skeleton`/`Joint`/`Transform`/`AnimationClip`/`AnimationChannel`/`ChannelData`/`Interpolation` type families be unified into a single shared crate (e.g., `astraweave-animation`)?** [Decisional.] Factual state: each of the four crates defines its own representation, optimized for its purpose (Pod serialization in asset, glam math in render, parallel arrays in scene ECS, glam matrices in editor). The §7.7 wrapped-component resource identity pattern documented in `docs/architecture/render_pipeline_material_system_shader_infrastructure.md` §6 and `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` Status header is the architectural precedent. No retirement campaign was identified.
- **`compute_poses_stub` — when does it stop being a stub?** [Decisional / factual mixed.] Factual: the function name says "stub" and its body (per `astraweave-scene/src/lib.rs:941+`) checks for `CSkeleton` + `CJointMatrices` presence. [NEEDS VERIFICATION on whether a complete `compute_poses` system exists elsewhere in `astraweave-scene` or whether ECS-side pose-compute is a known gap.] Decisional: if it's a gap, when does it close?
- **CubicSpline interpolation — implement or remove the variant?** [Decisional.] Factual: render-side and asset-side both have the variant but fall back to Linear/Slerp. glTF files authored with cubic-spline keyframes lose fidelity in this engine. Whether to implement it (matrix coefficient + tangent computation per glTF spec) or to remove the variant from the enum is undecided.
- **`Renderer::set_skinned_mesh` "v0 API" — eventual deprecation?** [Decisional.] Factual: the inline comment at `renderer.rs:6743` labels it v0; the pooled `JointPaletteManager` is the modern path. No deprecation timeline was found.
- **`EditorAnimationBridge` parity with runtime playback** — should a parity test ensure editor preview matches `AnimationState::update` + `AnimationClip::sample` outputs? [Decisional, with factual context.] Factual: no automated parity test between editor bridge and runtime animation was found in this pass.
- **`MAX_JOINTS = 256` — is this a permanent limit, or should it scale with hardware?** [Decisional.] Factual: hard-coded at `animation.rs:358` and `skinning_gpu.rs:261`, kept in sync by manual edit. Surplus joints silently drop. Industry conventions vary (Unity 32-64, Unreal 256-1024, custom engines often 256-512).
- **Are skeletons >256 joints a real authoring concern in current AstraWeave assets?** [Factual — investigable.] [NEEDS VERIFICATION via asset audit — but if yes, the silent drop at `animation.rs:388` is a hidden data-loss bug.]
- **Cross-crate conversion functions — should `From`/`Into` impls be added between the four `Skeleton`/`Joint`/etc. type families?** [Decisional, with factual context.] Factual: no `impl From<` between any pair of the four `Skeleton` types was found in this pass. Conversions happen ad-hoc at call sites.

---

## 12. Maintenance Notes

**Update this doc when:**
- A new `Skeleton` / `Joint` / `Transform` / `AnimationClip` definition is added or one of the existing four is removed (touch §3, §5, §6)
- `MAX_JOINTS` changes (must update Rust constant at `animation.rs:358` AND WGSL constant at `skinning_gpu.rs:261` AND this doc)
- CubicSpline interpolation is implemented or removed (touch §6, §8, §11)
- A new ECS animation component is added to `astraweave-scene` (touch §5, §3)
- The Renderer's "v0 skinning API" gains a planned deprecation (touch §6 conflict map, §11)
- `EditorAnimationBridge` gains or loses its parallel type system (touch §5, §6, §11)
- A new bone-attachment or pose-blending feature lands (touch §1, §2, §5)
- A decision in §7 is superseded by new code or audit

**Verification process:**
- Spot-check the pipeline in §2 against `animation.rs`, `skinning_gpu.rs`, `astraweave-asset/src/lib.rs:854-…`, and `astraweave-scene/src/lib.rs:415-…`
- Verify the four type families in §3 and §6 against the actual struct definitions in each crate
- Verify invariants in §8 against the cited line numbers
- Run `cargo test -p astraweave-render --tests` and the `#[ignore]`d parity test locally if GPU is available
- Update the metadata commit hash and date

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**

1. **There are FOUR parallel type families for animation data.** `astraweave_render::animation` (runtime, glam), `astraweave_asset::*` (loader, float arrays), `astraweave_scene::C*` (ECS, parallel arrays), `aw_editor::viewport::types::Gltf*` (editor, glam). They are not interchangeable. Each boundary needs explicit conversion. Editor also adds `EditorAnimationBridge`-specific types (`JointTransform`, `AnimationOutput`, `EntityAnimationState`, `ClipEntry`).
2. **CPU is the deterministic default; GPU is opt-in.** Per `animation.rs:1-6` module header. The CI-safe path is CPU. GPU parity test is `--ignored` for CI per `skinning_parity_cpu_vs_gpu.rs:6-7`.
3. **`MAX_JOINTS = 256`.** Hard-coded in Rust (`animation.rs:358`) and WGSL (`skinning_gpu.rs:261`). Surplus joints silently dropped at `animation.rs:388`. Keep both constants in sync.
4. **CubicSpline interpolation falls back to Linear/Slerp.** `animation.rs:127, 140, 153`. glTF cubic-spline clips lose fidelity.
5. **`compute_poses_stub` is named stub for a reason.** ECS-side full pose compute may be a gap; the canonical compute is render-side `compute_joint_matrices`.
6. **CPU skinning early-continues on weight==0 and out-of-range joint indices.** Silent skips, not panics (`animation.rs:413-420`).
7. **`JointPaletteManager` uses a pooled SSBO with dynamic offsets.** One bind group, many skeletons. Pool doubles on grow.
8. **Renderer has two skinning paths.** v0 simple API (`renderer.rs:6743-6771`) and pooled manager (`skinning_gpu.rs`). Pick deliberately.

**Files you'll most likely touch:**
- `astraweave-render/src/animation.rs` — canonical runtime
- `astraweave-render/src/skinning_gpu.rs` — GPU path (feature `skinning-gpu`)
- `astraweave-asset/src/lib.rs:854-…` — glTF loader
- `astraweave-scene/src/lib.rs:415-…` — ECS components and systems
- `tools/aw_editor/src/animation_bridge.rs` — editor bridge
- `tools/aw_editor/src/viewport/types.rs:238-298` — editor type definitions

**Files you should NOT touch without strong reason:**
- `astraweave-render/src/animation.rs:358` `MAX_JOINTS` — must stay in sync with `skinning_gpu.rs:261`
- `astraweave-render/src/skinning_gpu.rs:254-335` `SKINNING_GPU_SHADER` — embedded WGSL; CLAUDE.md mandates `shader-wgsl-reviewer` agent on shader changes
- `astraweave-render/src/types.rs:49-106` `SkinnedVertex` — GPU vertex layout; changing offsets/types breaks the WGSL shader at `skinning_gpu.rs` and all `SkinnedVertex` consumers
- `astraweave-render/src/animation.rs:274-336` `compute_joint_matrices` — cycle detection logic; the canonical hierarchical-pose computation

**Common mistakes when changing this system:**
- **Mistake**: Passing an `astraweave_asset::Skeleton` where an `astraweave_render::Skeleton` is required (or vice versa).
  **Why wrong**: Distinct types. Compiler catches the type mismatch. Add explicit conversion at the call site.
- **Mistake**: Updating `MAX_JOINTS` in Rust without updating the WGSL string constant.
  **Why wrong**: The WGSL at `skinning_gpu.rs:261` hard-codes `256`. Out-of-sync constants cause GPU buffer-binding mismatches.
- **Mistake**: Authoring a glTF clip with cubic-spline interpolation and expecting it to play back correctly.
  **Why wrong**: Both render-side and asset-side fall back to Linear/Slerp.
- **Mistake**: Authoring a skeleton with >256 joints.
  **Why wrong**: Surplus joints silently drop at `animation.rs:388`. There is no warning.
- **Mistake**: Treating `compute_poses_stub` as the canonical pose-compute path.
  **Why wrong**: Its name says stub; the canonical compute is `astraweave_render::animation::compute_joint_matrices`.
- **Mistake**: Adding a fifth `Skeleton` type in a new crate.
  **Why wrong**: There are already four. Per CLAUDE.md Scope Discipline: "Never build a second implementation of a logical system that already exists." Either reuse one of the four, or surface the decision explicitly.

---

## Appendix B: Historical context

The animation system was delivered as **Phase 2 Task 5** of the engine roadmap. Per `docs/archive/phase_reports/PHASE2_TASK5_IMPLEMENTATION_SUMMARY.md` (2025-10-01), the implementation followed a six-phase plan:

- **Phase A — Asset Import**: `astraweave-asset` extended with `Skeleton`, `Joint`, `Transform`, `AnimationClip` types and `load_skeleton` / `load_animations` glTF functions (5 tests).
- **Phase B — Animation Runtime**: `astraweave-render/src/animation.rs` (~600 lines original; 710 today) with `AnimationClip::sample`, `AnimationState`, `compute_joint_matrices`, `skin_vertex_cpu` (10 tests).
- **Phase C — ECS Integration**: `astraweave-scene` extended with `CSkeleton`, `CSkinnedMesh`, `CAnimator`, `CJointMatrices`, `CParentBone` and the `update_animations` / `compute_poses` / `update_bone_attachments` systems (14 tests).
- **Phase D — GPU Skinning Pipeline**: `astraweave-render/src/skinning_gpu.rs` (~400 lines original; 814 today after pool refactor) with `JointPalette`, `JointPaletteManager`, embedded WGSL shader (9 tests). The summary at line 210 notes "Pipeline implemented, compute dispatch integration pending" — the pooled SSBO with dynamic offsets refactor came later.
- **Phase E — Golden & Stress Tests**: 5 test files / 1,921 lines / 38 tests covering rest pose, animated pose, integration, CPU/GPU parity, stress.
- **Phase F — Interactive Demo**: `examples/skinning_demo` with keyboard controls and HUD.

The implementation summary frames the result as "~8,500+ lines, 70+ tests (66 passing + 4 ignored)" as of October 2025. Subsequent work added Wave 2 mutation-remediation tests (`wave2_animation_remediation.rs` 848 LoC), additional unit tests in `animation_extra_tests.rs` (29 tests / 613 LoC), and the editor-side `EditorAnimationBridge` (`tools/aw_editor/src/animation_bridge.rs` 451 LoC, with `animation_panel.rs` 3,680 LoC).

The four parallel type families originated during the phased delivery: each phase landed types in its target crate (asset / render / scene / editor) tailored to that crate's needs. There is no recovered record of a planned consolidation campaign.
