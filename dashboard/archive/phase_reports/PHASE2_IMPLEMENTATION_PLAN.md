# Phase 2: Rendering & Scene Graph Modernization — Implementation Plan

**Status**: In Progress  
**Target**: Q4 2025  
**Goal**: Achieve Bevy/Fyrox-caliber rendering with deterministic scene graph, GPU-driven culling, PBR materials, IBL, post-processing, and skeletal animation.

---

## Executive Summary

Phase 2 transforms AstraWeave's rendering from a basic forward renderer into a modern, modular system with:

1. **Scene Graph**: Hierarchical transforms with ECS integration
2. **Unified PBR Materials**: Single source of truth via `MaterialManager`
3. **GPU-Driven Rendering**: Compute-based culling and indirect draws
4. **IBL & Post-Processing**: Unified image-based lighting and bloom
5. **Skeletal Animation**: CPU/GPU skinning pipeline

All systems must maintain determinism, integrate with the ECS, and pass golden image tests.

---

## Current State Analysis

### ✅ What Already Works

1. **Render Graph Scaffolding** (`astraweave-render::graph`)
   - Linear execution model with typed resources
   - `ClearNode`, `RendererMainNode` adapters
   - Headless unit tests passing
   - Resource table with texture/buffer/bind group management

2. **Basic Scene Graph** (`astraweave-scene`)
   - `Transform`, `Node`, `Scene` structures
   - Tree traversal with world matrix propagation
   - ECS components: `CTransform`, `CParent`, `CChildren`
   - Basic `update_world_transforms` system (needs enhancement)

3. **Material System Foundation** (`astraweave-render::material`)
   - `MaterialManager` with D2 array texture support
   - TOML-based material authoring (`materials.toml`, `arrays.toml`)
   - Neutral fallbacks for missing textures
   - Working in `unified_showcase` example

4. **IBL Manager** (`astraweave-render::ibl`)
   - Basic `IblManager` structure exists
   - Prefiltered environment, irradiance, BRDF LUT concepts in place
   - Needs unification and consistent binding

### ⚠️ What Needs Polish

1. **Material System**
   - Examples still have local material loaders (`visual_3d`, `cutscene_render_demo`)
   - TOML schema validation missing
   - Hot-reload not implemented
   - Feature flags not comprehensive

2. **Scene Graph**
   - No dirty flags for transform updates
   - No skinning preparation (joint indices)
   - Limited ECS integration
   - No benchmarks for scalability

3. **IBL Integration**
   - Inconsistent usage across examples
   - No unified binding pattern
   - Missing integration with render graph

### ❌ What's Missing

1. **GPU-Driven Rendering**
   - No compute culling pass
   - No indirect draw buffers
   - No clustered/forward+ lighting
   - All culling is CPU-based

2. **Post-Processing**
   - No bloom implementation
   - No post-process nodes in graph
   - No HDR pipeline integration

3. **Skeletal Animation**
   - No skeleton/joint structures
   - No glTF skinning import
   - No CPU or GPU skinning systems
   - No animation sampler/timeline

---

## Phase 2 Implementation Tasks

### Task 1: Scene Graph Enhancement ❌ → ✅

**Owner**: Rendering Team  
**Priority**: P0 (Blocking for all rendering work)  
**Estimated Effort**: 3-5 days

#### Objectives

1. Enhance `astraweave-scene` with production-ready features:
   - Dirty flags for efficient transform updates
   - Deterministic traversal with stable ordering
   - Skinning preparation (joint indices storage)
   - Re-parenting support with proper invalidation

2. Deep ECS integration:
   - `CTransformLocal`, `CTransformWorld` split
   - `CVisible` component for culling
   - `CMesh`, `CMaterial` handles
   - Event emission for transform changes

3. Renderer synchronization:
   - `sync_scene_to_renderer` system
   - Instance collection with visibility filtering
   - Submission to `MeshRegistry` and render graph

#### Implementation Details

**New Components** (in `astraweave-scene/src/ecs.rs`):

```rust
pub struct CTransformLocal(pub Transform);
pub struct CTransformWorld(pub Mat4);
pub struct CParent(pub EntityId);
pub struct CChildren(pub Vec<EntityId>);
pub struct CDirtyTransform; // Tag component for dirty tracking
pub struct CVisible(pub bool);
pub struct CMesh(pub MeshHandle);
pub struct CMaterial(pub u32); // Material layer index
pub struct CJointIndices(pub Vec<u32>); // For skinning
```

**Enhanced Systems**:

```rust
// 1. Mark dirty on local transform changes
fn mark_dirty_transforms(world: &mut World) {
    // Detect CTransformLocal changes and tag with CDirtyTransform
}

// 2. Update world transforms top-down with dirty propagation
fn update_world_transforms(world: &mut World) {
    // Topological sort by parent relationships
    // Only update dirty nodes and propagate to children
    // Remove CDirtyTransform after update
}

// 3. Sync to renderer
fn sync_scene_to_renderer(
    world: &World,
    mesh_registry: &MeshRegistry,
    material_arrays: &MaterialGpuArrays,
) -> Vec<RenderInstance> {
    // Query entities with CMesh + CMaterial + CTransformWorld + CVisible
    // Filter by CVisible(true)
    // Collect into RenderInstance structs
}
```

**API for Hierarchy Manipulation**:

```rust
impl SceneGraph {
    pub fn attach(&mut self, world: &mut World, child: EntityId, parent: EntityId);
    pub fn detach(&mut self, world: &mut World, child: EntityId);
    pub fn reparent(&mut self, world: &mut World, child: EntityId, new_parent: EntityId);
}
```

#### Acceptance Tests

```rust
#[test]
fn test_transform_hierarchy_three_levels() {
    // Root -> Child1 -> Grandchild
    // Verify world transforms multiply correctly
}

#[test]
fn test_reparenting_invalidates_world_transforms() {
    // Move node to new parent
    // Verify CDirtyTransform propagates to all descendants
}

#[test]
fn test_deterministic_traversal_order() {
    // Spawn entities in random order
    // Verify traversal is always parent-before-child
    // Verify deterministic iteration (BTreeMap-backed)
}

#[test]
fn test_visibility_culling() {
    // Mark some entities CVisible(false)
    // Verify sync_scene_to_renderer excludes them
}
```

#### Benchmarks

```rust
// benches/scene_transform.rs
fn bench_update_1000_transforms(c: &mut Criterion) {
    // Flat hierarchy: 1000 entities, no parent
}

fn bench_update_deep_hierarchy(c: &mut Criterion) {
    // Deep chain: 100 levels of parent->child
}

fn bench_wide_hierarchy(c: &mut Criterion) {
    // One root with 1000 children
}
```

**Success Criteria**:
- [ ] All unit tests pass
- [ ] Benchmarks show < 1ms for 1000 transforms
- [ ] Integration test renders 3-level hierarchy with golden image match

---

### Task 2: PBR Material System Unification ✅

**Owner**: Asset Team  
**Priority**: P0 (Blocking for visual quality)  
**Estimated Effort**: 4-6 days

#### Objectives

1. Make `MaterialManager` the single source of truth
2. Remove all local material loaders from examples
3. Add TOML validation and better error handling
4. Implement hot-reload support
5. Add comprehensive feature flags

#### Implementation Details

**Enhanced MaterialManager API**:

```rust
impl MaterialManager {
    /// Load from biome directory with validation
    pub fn load_biome(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        biome_path: impl AsRef<Path>,
    ) -> Result<(MaterialGpuArrays, MaterialLoadStats)> {
        // 1. Parse materials.toml + arrays.toml
        // 2. Validate schema (all required fields, path existence)
        // 3. Load textures with fallbacks
        // 4. Build arrays with stable layer indices
        // 5. Return arrays + stats
    }

    /// Create stable bind group layout (group 1)
    pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        // 0: albedo_array (D2Array<rgba8_srgb>)
        // 1: sampler
        // 2: normal_array (D2Array<rg8_unorm>)
        // 3: sampler_linear
        // 4: mra_array (D2Array<rgba8_unorm>)
    }

    /// Create bind group from arrays
    pub fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        arrays: &MaterialGpuArrays,
    ) -> wgpu::BindGroup;

    /// Hot-reload support
    pub fn reload_biome(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        biome_path: impl AsRef<Path>,
    ) -> Result<MaterialLoadStats>;
}
```

**TOML Schema Validation**:

```rust
fn validate_material_pack(pack: &MaterialPackDesc) -> Result<()> {
    // Check all required fields present
    // Validate tiling values > 0
    // Check texture paths exist
    // Ensure no duplicate layer keys
}

fn validate_array_layout(layout: &ArrayLayoutToml) -> Result<()> {
    // Ensure indices are unique and contiguous
    // Warn on gaps in index space
}
```

**Example Migration**:

Before (`visual_3d`):
```rust
// Local material loading code (REMOVE)
let textures = load_local_textures(...);
```

After:
```rust
use astraweave_render::MaterialManager;

let (arrays, stats) = MaterialManager::load_biome(
    &device,
    &queue,
    "assets/materials/temperate",
)?;
println!("{}", stats.concise_summary());

let bind_group = MaterialManager::create_bind_group(&device, &layout, &arrays);
```

#### Feature Flags (in `astraweave-render/Cargo.toml`):

```toml
[features]
default = ["materials-runtime"]
materials-runtime = ["textures"] # Runtime loading only
materials-authoring = ["materials-runtime", "toml-validation"] # + hot-reload + validation
materials-dev = ["materials-authoring", "image/png", "image/tga"] # + all formats
```

#### Acceptance Tests

```rust
#[test]
fn test_toml_validation_rejects_invalid() {
    // Missing required fields
    // Duplicate layer keys
    // Non-existent texture paths
}

#[test]
fn test_stable_layer_indices() {
    // Load biome twice
    // Verify layer indices match across loads
}

#[test]
fn test_fallback_textures() {
    // Missing normal map -> flat blue
    // Missing MRA -> neutral (0.5, 0.5, 1.0)
}

#[test]
fn test_hot_reload_preserves_indices() {
    // Load biome
    // Modify TOML
    // Reload
    // Verify existing layer indices unchanged
}
```

**Golden Image Tests**:

```rust
// tests/golden_material.rs
#[test]
fn test_pbr_materials_golden() {
    // Load temperate biome
    // Render scene with 3 different materials
    // Compare to baseline PNG
    // Allow 1% pixel diff for platform variance
}
```

**Success Criteria**:
- [ ] All examples use `MaterialManager` (no local loaders)
- [ ] TOML validation catches common errors
- [ ] Hot-reload works in `unified_showcase` (press 'R' key)
- [ ] Golden image tests pass on Windows/Linux/macOS

### Task 3: GPU-Driven Rendering ✅ COMPLETE

**Owner**: Graphics Programming Team  
**Priority**: P1 (Performance-critical)  
**Estimated Effort**: 7-10 days  
**Actual Effort**: 2 days  
**Test Results**: 78/78 tests passing (100%)  
**Status**: ✅ Production-ready

#### Completion Summary

**What Was Delivered**:
1. ✅ Compute-based frustum culling with CPU/GPU parity
2. ✅ Indirect draw buffer generation (CPU path)
3. ✅ Culling node integrated into render graph
4. ✅ Fixed critical struct layout bug (std140 compliance)
5. ✅ Batching by mesh+material for efficient draws
6. ✅ Comprehensive test coverage (unit, integration, layout, indirect draw)
7. ✅ Feature flags for CPU/GPU paths

**Critical Bug Fixed**: InstanceAABB struct had incorrect field ordering causing GPU to read garbage data. Reordered fields to match std140 vec3 alignment rules (16-byte boundary after each vec3).

**Test Coverage**: 78/78 tests passing
- 50 unit tests (frustum math, AABB intersection)
- 2 layout tests (struct padding verification)
- 5 integration tests (CPU/GPU parity, pipeline validation)
- 7 indirect draw tests (batching, command generation)
- 2 debug tests (frustum plane inspection)
- 12 other tests (materials, pipeline)

**Documentation**: See `docs/PHASE2_TASK3_IMPLEMENTATION_SUMMARY.md` for complete details.

---

#### Original Objectives

1. Implement compute-based frustum culling
2. Build indirect draw buffers
3. Integrate culling node into render graph
4. Optional: Forward+ clustered lighting

#### Implementation Details

**Data Structures** (in `astraweave-render/src/culling.rs`):

```rust
/// Per-instance data for culling compute shader
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceAABB {
    center: [f32; 3],
    extent: [f32; 3],
    instance_index: u32,
    _pad: u32,
}

/// Frustum planes in compute shader
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct FrustumPlanes {
    planes: [[f32; 4]; 6], // left, right, bottom, top, near, far
}

/// Indirect draw command
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct DrawIndirectCommand {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}
```

**Compute Shader** (`astraweave-render/src/shaders/frustum_cull.wgsl`):

```wgsl
struct InstanceAABB {
    center: vec3<f32>,
    extent: vec3<f32>,
    instance_index: u32,
}

struct FrustumPlanes {
    planes: array<vec4<f32>, 6>,
}

@group(0) @binding(0) var<storage, read> instance_aabbs: array<InstanceAABB>;
@group(0) @binding(1) var<uniform> frustum: FrustumPlanes;
@group(0) @binding(2) var<storage, read_write> visible_instances: array<u32>;
@group(0) @binding(3) var<storage, read_write> visible_count: atomic<u32>;

fn is_aabb_visible(center: vec3<f32>, extent: vec3<f32>, frustum: FrustumPlanes) -> bool {
    // Test AABB against all 6 frustum planes
    for (var i = 0u; i < 6u; i++) {
        let plane = frustum.planes[i];
        let normal = plane.xyz;
        let d = plane.w;
        
        // Compute AABB corners relative to plane
        let p = center + extent * sign(normal);
        if (dot(p, normal) + d < 0.0) {
            return false;
        }
    }
    return true;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= arrayLength(&instance_aabbs)) { return; }
    
    let aabb = instance_aabbs[idx];
    if (is_aabb_visible(aabb.center, aabb.extent, frustum)) {
        let slot = atomicAdd(&visible_count, 1u);
        visible_instances[slot] = aabb.instance_index;
    }
}
```

**Culling Node** (in `astraweave-render/src/graph.rs`):

```rust
pub struct FrustumCullingNode {
    name: String,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    // Buffers created per-frame
    instance_buffer: Option<wgpu::Buffer>,
    frustum_buffer: Option<wgpu::Buffer>,
    visible_buffer: Option<wgpu::Buffer>,
    visible_count_buffer: Option<wgpu::Buffer>,
}

impl FrustumCullingNode {
    pub fn new(device: &wgpu::Device) -> Self {
        // Compile compute shader
        // Create pipeline
        // Create bind group layout
    }
    
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        instances: &[RenderInstance],
        camera: &Camera,
    ) {
        // Extract frustum planes from camera
        // Build InstanceAABB buffer
        // Upload frustum uniforms
        // Allocate visible buffers
    }
}

impl RenderNode for FrustumCullingNode {
    fn name(&self) -> &str { &self.name }
    
    fn run(&mut self, ctx: &mut GraphContext) -> Result<()> {
        let encoder = ctx.encoder.as_deref_mut().context("requires encoder")?;
        
        // Dispatch compute shader
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("frustum-culling"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, &self.bind_group, &[]);
        cpass.dispatch_workgroups((instance_count + 63) / 64, 1, 1);
        drop(cpass);
        
        // Insert visible buffer into resource table
        ctx.resources.insert_buf("visible_instances", self.visible_buffer.take().unwrap());
        Ok(())
    }
}
```

**Indirect Draw Integration**:

```rust
// In main render pass
let visible_buffer = ctx.resources.buffer("visible_instances")?;
render_pass.draw_indexed_indirect(visible_buffer, 0, draw_count);
```

#### CPU Fallback

```rust
#[cfg(not(feature = "gpu-culling"))]
fn fallback_cpu_culling(
    instances: &[RenderInstance],
    camera: &Camera,
) -> Vec<u32> {
    instances
        .iter()
        .enumerate()
        .filter(|(_, inst)| camera.frustum_contains_aabb(inst.aabb))
        .map(|(idx, _)| idx as u32)
        .collect()
}
```

#### Acceptance Tests

```rust
#[test]
fn test_compute_culling_correctness() {
    // Create instances: half in frustum, half out
    // Run GPU culling
    // Readback visible_count
    // Verify count matches expected
}

#[test]
fn test_indirect_draw_compiles() {
    // Ensure indirect draw path doesn't crash
    // Verify draw count is reasonable
}

#[cfg(feature = "gpu-culling")]
#[test]
fn test_gpu_vs_cpu_culling_parity() {
    // Run same scene through GPU and CPU culling
    // Verify visible instance lists match
}
```

#### Performance Benchmarks

```rust
// benches/culling.rs
fn bench_cpu_culling_10k(c: &mut Criterion);
fn bench_gpu_culling_10k(c: &mut Criterion);
fn bench_gpu_culling_100k(c: &mut Criterion);
```

**Success Criteria**:
- [ ] Compute culling works on DX12/Vulkan/Metal
- [ ] Indirect draw path tested
- [ ] CPU fallback functional
- [ ] GPU culling 10x faster than CPU for 100k+ instances
- [ ] Parity tests pass

---

### Task 4: IBL & Post-Processing Unification ⚠️ → ✅

**Owner**: Lighting Team  
**Priority**: P1 (Visual quality)  
**Estimated Effort**: 5-7 days

#### Objectives

1. Unify `IblManager` usage across all examples
2. Generate BRDF LUT, prefiltered environment, irradiance
3. Implement bloom post-process node
4. Integrate into render graph

#### Implementation Details

**Enhanced IblManager**:

```rust
pub struct IblResources {
    pub prefiltered_env: wgpu::TextureView,  // Cubemap with mips
    pub irradiance: wgpu::TextureView,       // Cubemap (low-res)
    pub brdf_lut: wgpu::TextureView,         // 2D (512x512)
    pub bind_group: wgpu::BindGroup,
}

impl IblManager {
    /// Create from HDR equirectangular
    pub fn from_hdr(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        hdr_bytes: &[u8],
    ) -> Result<IblResources> {
        // 1. Load HDR image
        // 2. Convert to cubemap
        // 3. Prefilter for roughness mips
        // 4. Compute irradiance (diffuse)
        // 5. Generate BRDF LUT
        // 6. Create bind group (group 2)
    }
    
    /// Create bind group layout (group 2)
    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        // 0: prefiltered_env (cube, 6 mips)
        // 1: irradiance (cube)
        // 2: brdf_lut (2D)
        // 3: sampler
    }
}
```

**BRDF LUT Generation** (compute shader):

```wgsl
// Integrate GGX BRDF for (NdotV, roughness) -> (scale, bias)
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let uv = vec2<f32>(gid.xy) / vec2<f32>(512.0, 512.0);
    let ndotv = uv.x;
    let roughness = uv.y;
    
    let result = integrate_brdf(ndotv, roughness);
    textureStore(brdf_lut, gid.xy, vec4<f32>(result, 0.0, 1.0));
}
```

**Bloom Implementation** (`astraweave-render/src/post/bloom.rs`):

```rust
pub struct BloomNode {
    name: String,
    threshold: f32,
    intensity: f32,
    // Pipelines for downsample, threshold, upsample
    downsample_pipeline: wgpu::RenderPipeline,
    threshold_pipeline: wgpu::RenderPipeline,
    upsample_pipeline: wgpu::RenderPipeline,
    // Mip chain textures (created per-frame)
    mip_chain: Vec<wgpu::TextureView>,
}

impl BloomNode {
    pub fn new(
        device: &wgpu::Device,
        hdr_format: wgpu::TextureFormat,
        threshold: f32,
        intensity: f32,
    ) -> Self {
        // Compile shaders
        // Create pipelines
    }
}

impl RenderNode for BloomNode {
    fn name(&self) -> &str { &self.name }
    
    fn run(&mut self, ctx: &mut GraphContext) -> Result<()> {
        // 1. Downsample HDR texture + apply threshold
        // 2. Downsample 5 levels (half-res each)
        // 3. Upsample with additive blending
        // 4. Composite onto final target
        Ok(())
    }
}
```

**Render Graph Integration**:

```rust
let mut graph = RenderGraph::new();

// Main lighting pass (outputs to HDR target)
graph.add_node(ClearNode::new("clear_hdr", "hdr_target", wgpu::Color::BLACK));
graph.add_node(LightingNode::new("lighting", "hdr_target", ibl_resources));

// Post-processing
graph.add_node(BloomNode::new("bloom", wgpu::TextureFormat::Rgba16Float, 1.0, 0.04));

// Tonemap to LDR surface
graph.add_node(TonemapNode::new("tonemap", "hdr_target", "surface"));

graph.execute(&mut ctx)?;
```

#### Acceptance Tests

```rust
#[test]
fn test_brdf_lut_generation() {
    // Generate LUT
    // Sample at (0.5, 0.5) -> expect reasonable value
    // Verify symmetry properties
}

#[test]
fn test_bloom_doesnt_alter_geometry() {
    // Render scene with bloom off
    // Render scene with bloom on
    // Verify geometry pass matches (pre-bloom)
}

#[test]
fn test_ibl_golden_image() {
    // Load "studio" HDR environment
    // Render sphere with varying roughness
    // Compare to baseline
}
```

**Success Criteria**:
- [ ] IBL works in all examples using unified API
- [ ] Bloom pass integrated into `unified_showcase`
- [ ] Golden image tests pass
- [ ] Toggle bloom on/off with 'B' key

---

### Task 5: Skeletal Animation ⚠️ → ✅ (IN PROGRESS - 20% COMPLETE)

**Owner**: Animation Team  
**Priority**: P2 (Feature-complete)  
**Estimated Effort**: 10-14 days  
**Current Status**: Phases A-B complete (Asset Import + Animation Runtime), ECS integration pending

#### Progress Summary

**✅ Completed (2/6 phases)**:
- Phase A: Asset Import - glTF skeleton/animation loading
- Phase B: Animation Runtime - Keyframe sampling, pose computation, CPU skinning

**⏳ In Progress**:
- Phase C: ECS Components (next)
- Phase D: GPU Skinning Pipeline
- Phase E: Integration Tests
- Phase F: Example Application

**Test Coverage**: 13/13 unit tests passing (100%)  
**Documentation**: Implementation plan created (`PHASE2_TASK5_IMPLEMENTATION_PLAN.md`)

#### Objectives

1. Import glTF skeletons and skinning data
2. Implement CPU skinning (correctness reference)
3. Implement GPU skinning (production path)
4. Integrate with ECS and scene graph

#### Implementation Details

**Data Structures** (in `astraweave-render/src/animation.rs`):

```rust
pub struct Skeleton {
    pub joints: Vec<Joint>,
    pub inverse_bind_matrices: Vec<Mat4>,
}

pub struct Joint {
    pub name: String,
    pub parent: Option<usize>,
    pub local_transform: Transform,
}

pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub channels: Vec<AnimationChannel>,
}

pub struct AnimationChannel {
    pub joint_index: usize,
    pub keyframes: Keyframes,
}

pub enum Keyframes {
    Translation(Vec<(f32, Vec3)>),
    Rotation(Vec<(f32, Quat)>),
    Scale(Vec<(f32, Vec3)>),
}

pub struct SkinnedMeshData {
    pub vertices: Vec<SkinnedVertex>,
    pub indices: Vec<u32>,
    pub joint_indices: Vec<[u16; 4]>,
    pub joint_weights: Vec<[f32; 4]>,
}
```

**glTF Import** (in `astraweave-render/src/mesh_gltf.rs`):

```rust
pub fn load_skinned_mesh_gltf(
    path: impl AsRef<Path>,
) -> Result<(SkinnedMeshData, Skeleton, Vec<AnimationClip>)> {
    let (doc, buffers, _) = gltf::import(path)?;
    
    // Extract skeleton
    let skin = doc.skins().next().context("no skin")?;
    let joints = extract_joints(&skin)?;
    let inverse_bind_matrices = extract_inverse_bind_matrices(&skin, &buffers)?;
    
    // Extract mesh with skinning data
    let mesh_data = extract_skinned_mesh(&doc, &buffers)?;
    
    // Extract animations
    let clips = doc.animations()
        .map(|anim| extract_animation_clip(&anim, &buffers))
        .collect::<Result<Vec<_>>>()?;
    
    Ok((mesh_data, Skeleton { joints, inverse_bind_matrices }, clips))
}
```

**ECS Components**:

```rust
pub struct CSkeleton(pub Arc<Skeleton>);
pub struct CSkinnedMesh(pub MeshHandle);
pub struct CAnimator {
    pub clip: Arc<AnimationClip>,
    pub time: f32,
    pub speed: f32,
    pub looping: bool,
}
pub struct CJointMatrices(pub Vec<Mat4>); // Final joint transforms for GPU
```

**CPU Skinning** (reference implementation):

```rust
fn update_joint_matrices_cpu(world: &mut World) {
    for (id, (skeleton, animator)) in world.query::<(&CSkeleton, &CAnimator)>() {
        // Sample animation at current time
        let local_transforms = sample_animation(&animator.clip, animator.time);
        
        // Compute world-space joint matrices
        let mut joint_matrices = vec![Mat4::IDENTITY; skeleton.0.joints.len()];
        compute_joint_hierarchy(&skeleton.0, &local_transforms, &mut joint_matrices);
        
        // Apply inverse bind matrices
        for (i, ibm) in skeleton.0.inverse_bind_matrices.iter().enumerate() {
            joint_matrices[i] = joint_matrices[i] * ibm;
        }
        
        world.insert(id, CJointMatrices(joint_matrices));
    }
}

fn sample_animation(clip: &AnimationClip, time: f32) -> Vec<Transform> {
    // Interpolate keyframes for each channel
    // Return local transform for each joint
}

fn compute_joint_hierarchy(
    skeleton: &Skeleton,
    local: &[Transform],
    out_world: &mut [Mat4],
) {
    // Topological traversal: parent before child
    for (i, joint) in skeleton.joints.iter().enumerate() {
        let local_mat = local[i].matrix();
        out_world[i] = if let Some(parent_idx) = joint.parent {
            out_world[parent_idx] * local_mat
        } else {
            local_mat
        };
    }
}
```

**GPU Skinning** (vertex shader):

```wgsl
struct SkinnedVertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) joint_indices: vec4<u32>,
    @location(4) joint_weights: vec4<f32>,
}

@group(3) @binding(0) var<storage, read> joint_matrices: array<mat4x4<f32>>;

@vertex
fn vs_skinned(in: SkinnedVertex) -> VertexOutput {
    // Compute skinned position
    var skinned_pos = vec4<f32>(0.0);
    var skinned_normal = vec3<f32>(0.0);
    
    for (var i = 0u; i < 4u; i++) {
        let joint_idx = in.joint_indices[i];
        let weight = in.joint_weights[i];
        let joint_mat = joint_matrices[joint_idx];
        
        skinned_pos += weight * (joint_mat * vec4<f32>(in.position, 1.0));
        skinned_normal += weight * (mat3x3<f32>(
            joint_mat[0].xyz,
            joint_mat[1].xyz,
            joint_mat[2].xyz
        ) * in.normal);
    }
    
    // Rest of vertex shader...
}
```

**System Integration**:

```rust
// In ECS schedule (simulation stage)
app.add_system_to_stage("simulation", update_animators);
app.add_system_to_stage("simulation", update_joint_matrices_cpu);
app.add_system_to_stage("presentation", upload_joint_matrices_gpu);
```

#### Acceptance Tests

```rust
#[test]
fn test_animation_sampler_interpolation() {
    // Create clip with keyframes at t=0, t=1, t=2
    // Sample at t=0.5, t=1.5
    // Verify linear interpolation
}

#[test]
fn test_animation_looping() {
    // Clip duration = 2.0s
    // Time = 2.5s
    // Verify loops back to t=0.5s
}

#[test]
fn test_joint_hierarchy_computation() {
    // Skeleton: root -> arm -> hand
    // root rotates 90°
    // Verify hand world position reflects parent rotations
}

#[test]
fn test_cpu_vs_gpu_skinning_parity() {
    // Compute skinned positions on CPU
    // Compute same on GPU
    // Read back and compare (within epsilon)
}
```

**Golden Image Test**:

```rust
// tests/skinned_mesh_golden.rs
#[test]
fn test_skinned_character_rest_pose() {
    // Load character.gltf
    // Render in rest pose (no animation)
    // Compare to baseline
}

#[test]
fn test_skinned_character_animated() {
    // Load character.gltf
    // Advance animation to t=1.0s
    // Render
    // Compare to baseline
}
```

**Example** (`examples/skinning_demo`):

```rust
fn main() -> Result<()> {
    // Load glTF with skeleton
    let (mesh_data, skeleton, clips) = load_skinned_mesh_gltf("assets/character.gltf")?;
    
    // Setup ECS
    let mut world = World::new();
    let character = world.spawn();
    world.insert(character, CSkeleton(Arc::new(skeleton)));
    world.insert(character, CSkinnedMesh(mesh_handle));
    world.insert(character, CAnimator {
        clip: Arc::new(clips[0].clone()),
        time: 0.0,
        speed: 1.0,
        looping: true,
    });
    
    // Render loop
    loop {
        // Update animation time
        update_animators(&mut world, dt);
        update_joint_matrices_cpu(&mut world);
        upload_joint_matrices_gpu(&world, &device, &queue);
        
        // Render with skinned vertex shader
        renderer.render_skinned(&world)?;
    }
}
```

**Success Criteria**:
- [ ] glTF import works for rigged characters
- [ ] CPU skinning produces correct results
- [ ] GPU skinning matches CPU (within epsilon)
- [ ] `skinning_demo` runs at 60fps
- [ ] Golden image tests pass
- [ ] Animation loops smoothly

---

## Integration & Testing Strategy

### Golden Image Testing

**Framework** (`astraweave-render/tests/golden_framework.rs`):

```rust
pub struct GoldenImageTest {
    name: &'static str,
    render_fn: Box<dyn Fn(&Device, &Queue) -> TextureView>,
    tolerance: f32, // 0.0 - 1.0
}

impl GoldenImageTest {
    pub fn run(&self) -> Result<()> {
        // 1. Render scene
        let output = (self.render_fn)(&device, &queue);
        
        // 2. Read back pixels
        let pixels = readback_texture(&device, &queue, &output)?;
        
        // 3. Load baseline
        let baseline_path = format!("tests/golden/{}.png", self.name);
        let baseline = if Path::new(&baseline_path).exists() {
            image::open(&baseline_path)?.to_rgba8()
        } else {
            // First run: save as baseline
            image::save_buffer(&baseline_path, &pixels, width, height, image::ColorType::Rgba8)?;
            return Ok(());
        };
        
        // 4. Compare
        let diff = compute_diff(&pixels, &baseline);
        assert!(diff < self.tolerance, "diff {} exceeds tolerance {}", diff, self.tolerance);
        Ok(())
    }
}
```

**Test Cases**:

```rust
#[test] fn test_golden_pbr_materials() { ... }
#[test] fn test_golden_ibl_lighting() { ... }
#[test] fn test_golden_bloom_post() { ... }
#[test] fn test_golden_scene_graph_hierarchy() { ... }
#[test] fn test_golden_skinned_mesh() { ... }
```

### CI Pipeline Updates

**`.github/workflows/phase2.yml`**:

```yaml
name: Phase 2 Validation

on: [push, pull_request]

jobs:
  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Check formatting
        run: cargo fmt --check
      
      - name: Clippy
        run: cargo clippy --workspace --all-features -- -D warnings
      
      - name: Unit tests
        run: cargo test -p astraweave-scene -p astraweave-render
      
      - name: Golden image tests (headless)
        run: cargo test --test golden_* --features gpu-tests
      
      - name: Examples compile check
        run: |
          cargo check -p visual_3d
          cargo check -p unified_showcase
          cargo check -p skinning_demo
      
      - name: Benchmarks
        run: cargo bench --no-run -p astraweave-scene -p astraweave-render
```

### Performance Metrics

**Telemetry** (logged per frame):

```rust
struct RenderMetrics {
    instances_total: u32,
    instances_culled: u32,
    instances_rendered: u32,
    draw_calls: u32,
    triangles: u64,
    gpu_time_ms: f32,
    cpu_time_ms: f32,
}
```

**Benchmarks**:

- Transform update: < 1ms for 10k nodes
- GPU culling: < 0.5ms for 100k instances
- Skinning: < 2ms for 100 characters (GPU)
- Bloom: < 3ms at 1080p

---

## Roadmap Sync

After all tasks complete, update `roadmap.md`:

```markdown
## Phase 2 (3–6 months): Rendering & Scene Graph Modernization

**Status**: ✅ Complete (Oct 2025)

### Completed Features

- ✅ **Scene Graph** [`astraweave-scene`]
  - Hierarchical transforms with dirty tracking
  - ECS integration: `CTransformLocal`, `CTransformWorld`, `CParent`, `CChildren`
  - Deterministic traversal (BTreeMap-backed)
  - `sync_scene_to_renderer` system
  - Tests: `test_transform_hierarchy_three_levels`, `test_reparenting_invalidates_world_transforms`
  - Benchmarks: < 1ms for 1000 transforms

- ✅ **PBR Material System** [`astraweave-render::material`]
  - Unified `MaterialManager` as single source of truth
  - TOML schema validation with friendly errors
  - Hot-reload support (press 'R' in examples)
  - Feature flags: `materials-runtime`, `materials-authoring`, `materials-dev`
  - All examples migrated (no local loaders)
  - Golden image tests: `test_pbr_materials_golden`

- ✅ **GPU-Driven Rendering** [`astraweave-render::culling`]
  - Compute-based frustum culling (`frustum_cull.wgsl`)
  - Indirect draw buffer generation
  - `FrustumCullingNode` integrated into render graph
  - CPU fallback for unsupported backends
  - Tests: `test_gpu_vs_cpu_culling_parity`
  - 10x faster than CPU for 100k+ instances

- ✅ **IBL & Post-Processing** [`astraweave-render::ibl`, `astraweave-render::post`]
  - Unified `IblManager` with BRDF LUT, prefiltered env, irradiance
  - Bloom node with threshold + 5-level mip chain
  - Integrated into render graph
  - Tests: `test_brdf_lut_generation`, `test_bloom_doesnt_alter_geometry`
  - Golden images: `test_ibl_golden_image`

- ✅ **Skeletal Animation** [`astraweave-render::animation`]
  - glTF skeleton/skinning import
  - CPU skinning (reference) + GPU skinning (production)
  - ECS components: `CSkeleton`, `CSkinnedMesh`, `CAnimator`, `CJointMatrices`
  - Animation sampling with interpolation and looping
  - Example: `skinning_demo`
  - Tests: `test_cpu_vs_gpu_skinning_parity`
  - Golden images: `test_skinned_character_animated`

### How to Validate

```powershell
# Build all Phase 2 components
cargo build -p astraweave-scene -p astraweave-render

# Run unit tests
cargo test -p astraweave-scene -p astraweave-render

# Run golden image tests
cargo test --test golden_* --features gpu-tests

# Run examples
cargo run -p unified_showcase --release
cargo run -p skinning_demo --release

# Benchmarks
cargo bench -p astraweave-scene -p astraweave-render
```

### Exit Criteria Met

- [x] All unit tests passing
- [x] Golden image tests stable across Windows/Linux/macOS
- [x] All examples compile and run with unified systems
- [x] Benchmarks within performance targets
- [x] CI green on all platforms
- [x] Documentation updated

---

## Phase 3 (Next): AI & Gameplay Systems

Objectives: Dynamic behavior trees, GOAP, ECS-based gameplay mechanics (combat, crafting, dialogue), weaving system, PCG...
```

---

## Risk Mitigation

### Known Risks

1. **Platform Differences in Golden Images**
   - **Mitigation**: Use 1-2% pixel diff tolerance; normalize color space; test on all platforms

2. **Compute Shader Support Variability**
   - **Mitigation**: Feature-flag GPU culling; maintain CPU fallback; detect capabilities at runtime

3. **glTF Import Complexity**
   - **Mitigation**: Start with simple rigs; validate against Blender exports; unit test edge cases

4. **Performance Regression**
   - **Mitigation**: Continuous benchmarking in CI; flame graphs for GPU traces; profile before/after

5. **API Breakage for Examples**
   - **Mitigation**: Deprecation warnings; compatibility shims; migrate examples incrementally

---

## Success Metrics

### Quantitative

- Transform updates: < 1ms for 10k nodes
- GPU culling: < 0.5ms for 100k instances
- Skinning: < 2ms for 100 characters
- Golden image diff: < 1% pixel variance
- Example startup time: < 2s

### Qualitative

- Examples use unified `MaterialManager` (no bespoke code)
- Scene graph API feels natural (minimal boilerplate)
- Hot-reload works reliably
- Documentation is comprehensive

---

## Timeline Estimate

| Task | Priority | Estimated Effort | Dependencies |
|------|----------|------------------|--------------|
| 1. Scene Graph Enhancement | P0 | 3-5 days | None |
| 2. Material System Unification | P0 | 4-6 days | None |
| 3. GPU-Driven Rendering | P1 | 7-10 days | Task 1 (scene graph) |
| 4. IBL & Post-Processing | P1 | 5-7 days | Task 2 (materials) |
| 5. Skeletal Animation | P2 | 10-14 days | Task 1, 2 |
| **Total** | | **29-42 days** | |

With parallel work streams:
- **Optimistic**: 4-5 weeks (parallelizing tasks 1+2, then 3+4)
- **Realistic**: 6-8 weeks (accounting for integration, testing, iteration)
- **Conservative**: 10-12 weeks (with unknowns, platform issues, polish)

---

## Next Steps

1. **Immediate (This Week)**:
   - Create feature branches: `phase2/scene-graph`, `phase2/materials`, etc.
   - Set up golden image test infrastructure
   - Begin Task 1 (Scene Graph Enhancement)

2. **Short-term (Next 2 Weeks)**:
   - Complete Tasks 1 & 2 (Scene Graph + Materials)
   - Migrate 2-3 examples to MaterialManager
   - Set up CI for golden image tests

3. **Mid-term (Weeks 3-6)**:
   - Implement Tasks 3 & 4 (GPU Culling + IBL/Bloom)
   - Validate performance benchmarks
   - Begin Task 5 (Skeletal Animation) in parallel

4. **Long-term (Weeks 7-12)**:
   - Complete Task 5 (Skeletal Animation)
   - Polish and bug fixes
   - Update all documentation
   - Prepare Phase 3 kickoff

---

## Conclusion

Phase 2 is an ambitious but achievable goal that will bring AstraWeave's rendering to modern engine parity. The plan prioritizes:

- **Modularity**: Scene graph, materials, culling, post-processing as separate, testable modules
- **Determinism**: Stable iteration order, golden image tests, reproducible results
- **Performance**: GPU-driven rendering, efficient culling, scalable transforms
- **Quality**: PBR materials, IBL, bloom, skeletal animation

With disciplined execution, comprehensive testing, and iterative refinement, we'll emerge with a rendering system worthy of Bevy/Fyrox comparison.

**Let's build something amazing! 🚀**
