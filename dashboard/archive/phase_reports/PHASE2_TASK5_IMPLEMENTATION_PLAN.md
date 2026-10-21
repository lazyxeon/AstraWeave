# Phase 2 Task 5: Skeletal Animation - Implementation Plan

**Status**: ⚠️ **IN PROGRESS**  
**Started**: 2025-01-01  
**Target Completion**: TBD

---

## Overview

This document tracks the implementation of Phase 2 Task 5: Skeletal Animation for AstraWeave. The goal is to add complete skeletal animation support with both CPU and GPU skinning paths, integrated with the ECS and rendering pipelines.

### Design Principles

1. **Determinism First**: CPU skinning is the default and CI-safe path
2. **GPU Skinning Optional**: Feature-gated GPU skinning for performance
3. **ECS Integration**: Animation state stored as ECS components
4. **Modular WGSL**: Separate shaders for CPU and GPU paths
5. **Asset Pipeline**: glTF skeleton/animation import with reproducible hashes

---

## Implementation Phases

### ✅ Phase A: Asset Import (COMPLETE)

**Goal**: Extend `astraweave-asset` to load skeletons and animations from glTF/GLB files.

#### Completed Work

1. **Data Structures** (`astraweave-asset/src/lib.rs`):
   - ✅ `Joint`: Skeleton joint with parent index, inverse bind matrix, local transform
   - ✅ `Transform`: TRS representation with default identity values
   - ✅ `Skeleton`: Joint hierarchy with root indices
   - ✅ `AnimationChannel`: Keyframe data for translation/rotation/scale
   - ✅ `ChannelData`: Enum for different property types
   - ✅ `AnimationClip`: Named clip with duration and multiple channels
   - ✅ `Interpolation`: Step/Linear/CubicSpline modes

2. **Import Functions**:
   - ✅ `load_skeleton()`: Extract skeleton from glTF with hierarchy
   - ✅ `load_animations()`: Load all animation clips targeting skeleton
   - ✅ `load_skinned_mesh_complete()`: One-stop function for mesh + skeleton + animations
   - ✅ Legacy compatibility: Updated `load_first_skinned_mesh_and_idle()` to use new types

3. **Tests** (`astraweave-asset/src/lib.rs`):
   - ✅ `test_skeleton_structure`: Verify Joint and Transform types
   - ✅ `test_animation_channel_types`: Verify animation data structures
   - ✅ `test_skeleton_root_detection`: Verify hierarchy construction

#### Validation

```bash
# Test asset loader with gltf feature
cargo test -p astraweave-asset --features gltf --lib

# Expected: All tests passing including new skeleton tests
```

---

### ✅ Phase B: Animation Runtime (COMPLETE)

**Goal**: Implement animation playback and pose computation in `astraweave-render`.

#### Completed Work

1. **Core Module** (`astraweave-render/src/animation.rs`):
   - ✅ `Transform`: Runtime TRS with `to_matrix()` and `lerp()`
   - ✅ `AnimationClip::sample()`: Keyframe sampling with interpolation
   - ✅ `AnimationClip::find_keyframes()`: Binary search for keyframe indices
   - ✅ `AnimationState`: Playback state (time, speed, looping, playing)
   - ✅ `AnimationState::update()`: Advance time with looping/clamping
   - ✅ `compute_joint_matrices()`: Hierarchical world-space matrix computation
   - ✅ `skin_vertex_cpu()`: CPU skinning for single vertex
   - ✅ `JointPalette`: GPU upload structure with 256 joints max
   - ✅ `JointMatrixGPU`: Aligned GPU matrix representation

2. **Feature Flags** (`astraweave-render/Cargo.toml`):
   - ✅ `skinning-cpu`: Default feature, enables CPU skinning
   - ✅ `skinning-gpu`: Optional feature, enables GPU skinning

3. **Library Exports** (`astraweave-render/src/lib.rs`):
   - ✅ Exported all animation types for external use

4. **Tests** (`astraweave-render/src/animation.rs`):
   - ✅ `test_transform_default`: Verify identity transform
   - ✅ `test_transform_to_matrix`: TRS to Mat4 conversion
   - ✅ `test_animation_state_update_looping`: Looping playback
   - ✅ `test_animation_state_update_clamping`: Clamped playback
   - ✅ `test_find_keyframes`: Keyframe search edge cases
   - ✅ `test_joint_matrices_single_joint`: Single joint pose
   - ✅ `test_joint_matrices_hierarchy`: Parent-child transforms
   - ✅ `test_cpu_skinning_single_joint`: CPU skinning single influence
   - ✅ `test_cpu_skinning_blend`: CPU skinning blended weights
   - ✅ `test_joint_palette_creation`: GPU palette construction

#### Validation

```bash
# Test animation module
cargo test -p astraweave-render --lib animation --features skinning-cpu

# Expected: All 13 animation tests passing
```

---

### ⏳ Phase C: ECS Components (TODO)

**Goal**: Add ECS components for skeletal animation in `astraweave-scene`.

#### Planned Components

1. **`CSkeleton`**: Skeleton data (joints, hierarchy)
2. **`CSkinnedMesh`**: Skinned mesh reference with joint weights
3. **`CAnimator`**: Animation playback state (clip index, time)
4. **`CJointMatrices`**: Computed joint matrices (cached)
5. **`CDirtyAnimation`**: Tag for entities needing pose update

#### Planned Systems

1. **`update_animations`**: Advance animation time, loop/clamp logic
2. **`compute_poses`**: Sample animations → compute joint matrices
3. **`apply_cpu_skinning`**: Transform vertices with joint matrices
4. **`sync_to_renderer`**: Upload joint palettes for GPU skinning

#### Integration

- Integrate with existing `astraweave-scene` transform hierarchy
- Attach child entities to joints (weapons, accessories)
- Support multiple animation layers (idle + overlay)

---

### ⏳ Phase D: GPU Skinning Pipeline (TODO)

**Goal**: Implement GPU skinning pipeline with WGSL shaders.

#### Planned WGSL Shaders

1. **`skinning_gpu.wgsl`**:
   - Vertex shader with joint palette binding
   - Apply skinning matrix per vertex
   - Support up to 4 joint influences per vertex

2. **Bind Group Layout** (Group 4: Skinning):
   - Binding 0: Joint palette (storage buffer, read-only)
   - Binding 1: Skinning uniforms (joint count, etc.)

#### Render Pipeline Updates

- Add skinned mesh pipeline variant
- Bind joint palette buffer per entity
- Support both static and skinned meshes in same frame

---

### ⏳ Phase E: Integration Tests (TODO)

**Goal**: Comprehensive integration tests for CPU and GPU paths.

#### Planned Tests

1. **CPU vs GPU Parity**:
   - Load simple skinned mesh (cube with 2 joints)
   - Compute skinned pose on CPU and GPU
   - Compare vertex positions within tolerance

2. **Animation Playback**:
   - Load animation clip
   - Advance time deterministically
   - Verify pose repeatability across runs

3. **Scene Graph Integration**:
   - Attach object to joint
   - Verify object moves with skeleton

4. **Performance**:
   - CPU skinning scales linearly with vertex count
   - GPU skinning handles 10k instances without regression

---

### ⏳ Phase F: Example Application (TODO)

**Goal**: Create `examples/skinning_demo` to demonstrate features.

#### Planned Features

- Load character model with skeleton
- Play idle animation with looping
- Toggle CPU vs GPU skinning
- Display FPS and performance stats
- Attach sword to hand joint

---

## Current Test Coverage

### Unit Tests (Passing: 13/13)

- **Asset Loader** (3 tests):
  - `test_skeleton_structure`
  - `test_animation_channel_types`
  - `test_skeleton_root_detection`

- **Animation Runtime** (10 tests):
  - `test_transform_default`
  - `test_transform_to_matrix`
  - `test_animation_state_update_looping`
  - `test_animation_state_update_clamping`
  - `test_find_keyframes`
  - `test_joint_matrices_single_joint`
  - `test_joint_matrices_hierarchy`
  - `test_cpu_skinning_single_joint`
  - `test_cpu_skinning_blend`
  - `test_joint_palette_creation`

### Integration Tests (Planned: 0/4)

- ⏳ CPU vs GPU parity
- ⏳ Animation playback determinism
- ⏳ Scene graph integration
- ⏳ Performance validation

---

## API Surface

### Import (astraweave-asset)

```rust
use astraweave_asset::gltf_loader::{
    load_skeleton, load_animations, load_skinned_mesh_complete,
    Skeleton, AnimationClip, SkinnedMeshData
};

// Load complete skinned model
let bytes = std::fs::read("model.glb")?;
let (mesh, skeleton, animations, material) = load_skinned_mesh_complete(&bytes)?;
```

### Runtime (astraweave-render)

```rust
use astraweave_render::{
    Skeleton, AnimationClip, AnimationState,
    compute_joint_matrices, skin_vertex_cpu, JointPalette
};

// Setup animation state
let mut state = AnimationState {
    clip_index: 0,
    time: 0.0,
    speed: 1.0,
    looping: true,
    playing: true,
};

// Update each frame
state.update(dt, clip.duration);

// Sample animation
let local_transforms = clip.sample(state.time, &skeleton);

// Compute joint matrices
let joint_matrices = compute_joint_matrices(&skeleton, &local_transforms);

// CPU skinning
let (skinned_pos, skinned_normal) = skin_vertex_cpu(
    position, normal, joints, weights, &joint_matrices
);

// Or GPU skinning
let palette = JointPalette::from_matrices(&joint_matrices);
// Upload palette to GPU buffer
```

---

## Performance Characteristics

### CPU Skinning (Default)

- **Complexity**: O(vertices × influences × joints)
- **Typical**: ~1-5ms for 10k vertices, 4 influences, 64 joints
- **Deterministic**: Yes, bit-exact across platforms
- **CI-Safe**: Yes, no GPU required

### GPU Skinning (Optional)

- **Complexity**: O(1) on GPU (parallel per-vertex)
- **Typical**: ~0.1-0.5ms for 10k vertices (GPU-dependent)
- **Deterministic**: No (GPU floating-point variance)
- **CI-Safe**: No (requires GPU for tests)

---

## Known Limitations

1. **Max Joints**: 256 joints per skeleton (GPU buffer size)
2. **Max Influences**: 4 joints per vertex (glTF standard)
3. **Interpolation**: Cubic spline not fully implemented (fallback to linear)
4. **Animation Blending**: Not implemented (single clip only)
5. **IK/Physics**: Not implemented (future Phase 3)

---

## Acceptance Criteria

### Must Complete Before ✅

- [ ] All unit tests passing (currently 13/13 ✅)
- [ ] Integration tests for CPU skinning passing
- [ ] Integration tests for GPU skinning passing (feature-gated)
- [ ] CPU vs GPU parity test within 0.01% tolerance
- [ ] Scene graph integration working
- [ ] Example application running
- [ ] `cargo fmt` clean
- [ ] `cargo clippy -D warnings` clean
- [ ] CI green with CPU skinning only
- [ ] Documentation complete

### Current Status: 2/10 Phases Complete

- ✅ Phase A: Asset Import
- ✅ Phase B: Animation Runtime
- ⏳ Phase C: ECS Components
- ⏳ Phase D: GPU Skinning Pipeline
- ⏳ Phase E: Integration Tests
- ⏳ Phase F: Example Application

---

## Commands

```bash
# Development
cargo fmt -p astraweave-asset -p astraweave-render
cargo clippy -p astraweave-asset -p astraweave-render --features skinning-cpu -- -D warnings

# Testing
cargo test -p astraweave-asset --features gltf --lib
cargo test -p astraweave-render --lib animation --features skinning-cpu

# GPU skinning tests (requires GPU)
cargo test -p astraweave-render --features skinning-gpu

# Run example (when implemented)
cargo run -p skinning_demo --release
```

---

## Next Steps

1. **Phase C**: Implement ECS components in `astraweave-scene`
   - Add `CSkeleton`, `CSkinnedMesh`, `CAnimator` components
   - Add `update_animations` and `compute_poses` systems
   - Write ECS integration tests

2. **Phase D**: Implement GPU skinning pipeline
   - Write `skinning_gpu.wgsl` shader
   - Update renderer to bind joint palette
   - Create GPU vs CPU parity test

3. **Phase E**: Integration tests
   - CPU vs GPU comparison with tolerance
   - Animation playback repeatability
   - Scene graph attachment

4. **Phase F**: Example application
   - Load skinned model
   - Play animations
   - Toggle CPU/GPU skinning
   - Display performance stats

---

**Last Updated**: 2025-01-01
**Document Version**: 0.1.0
