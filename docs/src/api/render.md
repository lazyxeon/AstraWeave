# Render API Reference

> **Crate**: `astraweave-render`  
> **Coverage**: ~68%  
> **Tests**: 300+

Modern wgpu-based rendering with PBR materials, shadows, post-processing, GPU skinning, and mesh optimization.

## Quick Links

- [rustdoc](https://docs.rs/astraweave-render) (when published)
- [Source](https://github.com/astraweave/astraweave/tree/main/astraweave-render)
- [Rendering Guide](../core-systems/rendering.md)

---

## Core Types

### Renderer

Central rendering coordinator.

```rust
use astraweave_render::{Renderer, RendererConfig};

let config = RendererConfig {
    width: 1920,
    height: 1080,
    vsync: true,
    msaa_samples: 4,
    ..Default::default()
};

let mut renderer = Renderer::new(&window, config).await?;

// Render frame
renderer.begin_frame();
renderer.draw_scene(&scene);
renderer.end_frame();
```

---

### MeshRegistry

GPU mesh resource management.

```rust
use astraweave_render::MeshRegistry;

let mut registry = MeshRegistry::new(&device);

// Register mesh
let mesh_id = registry.register(MeshData {
    vertices: vertex_data,
    indices: index_data,
    bounds: aabb,
});

// Draw mesh
renderer.draw_mesh(mesh_id, &transform, &material);
```

---

### MaterialManager

PBR material system with hot reloading.

```rust
use astraweave_render::{MaterialManager, PbrMaterial};

let mut materials = MaterialManager::new(&device);

// Create material
let material = PbrMaterial {
    albedo: Color::rgb(0.8, 0.2, 0.2),
    metallic: 0.0,
    roughness: 0.5,
    normal_map: Some(normal_texture),
    ..Default::default()
};

let material_id = materials.register(material);
```

---

### IblManager

Image-Based Lighting for environment reflections.

```rust
use astraweave_render::IblManager;

let mut ibl = IblManager::new(&device);

// Load environment map
ibl.load_hdr("environment.hdr").await?;

// Bind to shader
ibl.bind(&mut render_pass);
```

---

## GPU Skinning

### SkinnedVertex

Vertex format with bone weights.

```rust
use astraweave_render::skinning_gpu::{SkinnedVertex, SkinnedMesh};

let vertex = SkinnedVertex {
    position: [0.0, 1.0, 0.0],
    normal: [0.0, 1.0, 0.0],
    uv: [0.5, 0.5],
    bone_indices: [0, 1, 0, 0],  // Up to 4 bones
    bone_weights: [0.7, 0.3, 0.0, 0.0],
};

let skinned_mesh = SkinnedMesh::new(&device, vertices, indices);
```

---

### BoneMatrixBuffer

GPU bone matrix upload.

```rust
use astraweave_render::skinning_gpu::BoneMatrixBuffer;

let mut bone_buffer = BoneMatrixBuffer::new(&device, max_bones);

// Update bone matrices each frame
bone_buffer.update(&queue, &bone_matrices);

// Bind for skinning
bone_buffer.bind(&mut render_pass);
```

---

## Mesh Optimization

### Vertex Compression

37% memory reduction via octahedral normals and half-float UVs.

```rust
use astraweave_render::vertex_compression::{compress_vertices, CompressedVertex};

let compressed = compress_vertices(&vertices);

// 32 bytes → 20 bytes per vertex
assert!(std::mem::size_of::<CompressedVertex>() < std::mem::size_of::<Vertex>());
```

---

### LOD Generation

Automatic level-of-detail mesh generation.

```rust
use astraweave_render::lod_generator::{generate_lods, LodConfig};

let config = LodConfig {
    target_ratios: vec![1.0, 0.5, 0.25, 0.1],
    min_triangles: 100,
    ..Default::default()
};

let lods = generate_lods(&mesh, config);

// lods[0] = original
// lods[1] = 50% triangles
// lods[2] = 25% triangles
// lods[3] = 10% triangles
```

---

### Instancing

GPU instanced rendering for repeated objects.

```rust
use astraweave_render::instancing::{InstanceBuffer, InstanceData};

let mut instances = InstanceBuffer::new(&device, max_instances);

// Update instance data
let instance_data: Vec<InstanceData> = trees.iter()
    .map(|t| InstanceData {
        transform: t.transform.to_cols_array_2d(),
        color: t.color.into(),
    })
    .collect();

instances.update(&queue, &instance_data);

// Draw all instances in one call
renderer.draw_instanced(mesh_id, &instances);
```

---

## Shadows

### CascadedShadowMaps

Multi-cascade shadow mapping for large scenes.

```rust
use astraweave_render::shadows::{CascadedShadowMaps, CsmConfig};

let config = CsmConfig {
    cascade_count: 4,
    resolution: 2048,
    split_lambda: 0.5,
    ..Default::default()
};

let mut csm = CascadedShadowMaps::new(&device, config);

// Update cascades
csm.update(&camera, &sun_direction);

// Render shadow maps
for cascade in csm.cascades() {
    renderer.render_shadow_pass(cascade, &shadow_casters);
}
```

---

## Post-Processing

### PostFxPipeline

Post-processing effect chain.

```rust
use astraweave_render::post_fx::{PostFxPipeline, PostFxConfig};

let config = PostFxConfig {
    bloom_enabled: true,
    bloom_intensity: 0.5,
    tonemapping: Tonemapping::Aces,
    exposure: 1.0,
    ..Default::default()
};

let mut post_fx = PostFxPipeline::new(&device, config);

// Apply post-processing
post_fx.process(&mut render_context, &scene_texture);
```

**Available Effects**:
- Bloom
- Tonemapping (ACES, Reinhard, Filmic)
- Color grading
- Vignette
- Film grain
- SSAO (optional)

---

## Camera

Camera types live in the [`astraweave-camera`](./camera.md) crate as of the
Unified Camera campaign's C.3.A sub-phase. The renderer consumes the
canonical `RenderView` upload contract exclusively via
`Renderer::update_view`; the historical `Renderer::update_camera(&Camera)`
and `Renderer::update_camera_matrices(...)` APIs were removed in C.3.C.

### Canonical upload pattern

```rust
use astraweave_camera::{CameraProducer, FreeFly};

let camera = FreeFly {
    position: Vec3::new(0.0, 5.0, 10.0),
    yaw: 0.0,
    pitch: 0.0,
    fovy: 60_f32.to_radians(),
    aspect: 16.0 / 9.0,
    znear: 0.1,
    zfar: 1000.0,
};

// Upload to the renderer via the canonical RenderView contract.
renderer.update_view(&camera.to_render_view());
```

For camera-relative rendering (used by certain shadow/atmospheric paths to
mitigate large-world float precision), use the concrete-only sibling method:

```rust
renderer.update_view(&camera.to_render_view_camera_relative());
```

For new camera implementations (orbit, follow, cinematic), implement the
`CameraProducer` trait from `astraweave-camera`. See the crate's documentation
for the trait surface, and `docs/current/CAMERA_CONVENTIONS.md` for the
canonical convention reference.

---

## Debug Rendering

### DebugDraw

Immediate-mode debug visualization.

```rust
use astraweave_render::debug_draw::DebugDraw;

let mut debug = DebugDraw::new(&device);

// Draw primitives
debug.line(start, end, Color::RED);
debug.sphere(center, radius, Color::GREEN);
debug.box3d(&aabb, Color::BLUE);
debug.arrow(origin, direction, Color::YELLOW);

// Submit
debug.flush(&mut render_pass);
```

---

## Render Graph

### RenderGraph

Declarative render pipeline.

```rust
use astraweave_render::graph::{RenderGraph, RenderPass};

let mut graph = RenderGraph::new();

// Define passes
graph.add_pass(RenderPass::new("shadow")
    .with_output("shadow_map", Format::Depth32Float));

graph.add_pass(RenderPass::new("gbuffer")
    .with_output("albedo", Format::Rgba8Unorm)
    .with_output("normal", Format::Rgba16Float)
    .with_output("depth", Format::Depth32Float));

graph.add_pass(RenderPass::new("lighting")
    .with_input("albedo")
    .with_input("normal")
    .with_input("shadow_map")
    .with_output("hdr_color", Format::Rgba16Float));

// Execute
graph.execute(&mut renderer);
```

---

## Performance

| Operation | Latency | Notes |
|-----------|---------|-------|
| Mesh draw | 21 ns | Per instance |
| Vertex compression | 21 ns | Per vertex |
| Instancing overhead | 2 ns | Per instance |
| Shadow pass (2K) | ~1 ms | Per cascade |
| Post-FX (full) | ~2 ms | All effects |

---

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `pbr` | PBR materials | ✅ |
| `shadows` | Shadow mapping | ✅ |
| `post-fx` | Post-processing | ✅ |
| `debug-render` | Debug visualization | ❌ |
| `hot-reload` | Shader hot reload | ❌ |

---

## See Also

- [Rendering Guide](../core-systems/rendering.md)
- [Materials System](./materials.md)
- [GPU Skinning](../core-systems/animation.md#gpu-skinning)
- [Performance Optimization](../performance/optimization.md#rendering)
