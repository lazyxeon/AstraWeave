# Render API Reference

> **Crate**: `astraweave-render`  
> **Manifest version**: `0.1.0`

`Renderer` is constructed with `pub async fn new(window: Arc<winit::window::Window>) -> Result<Self>`. `new_headless(width, height)` is also async. There is no `RendererConfig`. There is no `begin_frame`, `end_frame`, or `draw_scene`.

`render(&mut self) -> Result<()>`. `update_view(&mut self, view: &RenderView)`. `set_material_params(&mut self, base_color: [f32; 4], metallic: f32, roughness: f32)`.

`Material` and `Mesh` are re-exported from `types`. `Material` is `color: [f32; 4]`. `Mesh` is `vertex_buf`, `index_buf`, and `index_count`. There is no `PbrMaterial` in that module.

`MeshRegistry::new()` takes no device. `fetch_or_upload(&mut self, device, queue, key: MeshKey, mesh: &CpuMesh) -> Result<MeshHandle>`.

`IblManager` is `astraweave_render::ibl::IblManager`. `Renderer::ibl_mut` returns `&mut IblManager`. `bake_environment(&mut self, quality: IblQuality) -> Result<()>`.

`SkinnedVertex` fields are `position: [f32; 3]`, `normal: [f32; 3]`, `tangent: [f32; 4]`, `uv: [f32; 2]`, `joints: [u16; 4]`, `weights: [f32; 4]`.

## Camera

Camera types live in `astraweave-camera`. The renderer takes a `RenderView` through `Renderer::update_view`.

```rust
use astraweave_camera::{CameraProducer, FreeFly};
use glam::Vec3;

let camera = FreeFly {
    position: Vec3::new(0.0, 5.0, 10.0),
    yaw: 0.0,
    pitch: 0.0,
    fovy: 60_f32.to_radians(),
    aspect: 16.0 / 9.0,
    znear: 0.1,
    zfar: 1000.0,
};
let view = camera.to_render_view();
let relative = camera.to_render_view_camera_relative();
let _ = (view, relative);
```

`to_render_view` is the `CameraProducer` method. `to_render_view_camera_relative` is inherent on `FreeFly`. Pass either view to `Renderer::update_view`.

## Features

`default = ["postfx", "textures", "terrain-splat-arrays"]`.

Other names in the manifest include `gpu-frame-budget`, `skinning-cpu`, `skinning-gpu`, `ibl`, `msaa`, `deferred`, `advanced-post`, `nanite`, and `gltf-assets`. There is no `pbr`, `shadows`, `post-fx`, `debug-render`, or `hot-reload` feature.

## See Also

- [Rendering Guide](../core-systems/rendering.md)
- [Rendering showcase](../examples/unified-showcase.md)
- [Performance Optimization](../performance/optimization.md#rendering-optimization)
