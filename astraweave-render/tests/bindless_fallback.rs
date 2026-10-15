//! Bindless registration follows the device, and the bind-group path still draws.

use astraweave_render::mesh::{CpuImage, CpuMesh, MeshVertex};
use astraweave_render::{Instance, Renderer};

fn triangle() -> CpuMesh {
    let mut mesh = CpuMesh::default();
    mesh.vertices = vec![
        MeshVertex::from_arrays([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0, 1.0], [0.0, 0.0]),
        MeshVertex::from_arrays([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0, 1.0], [1.0, 0.0]),
        MeshVertex::from_arrays([0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0, 1.0], [0.0, 1.0]),
    ];
    mesh.indices = vec![0, 1, 2];
    mesh.albedo_image = Some(CpuImage {
        width: 1,
        height: 1,
        pixels: vec![200, 40, 40, 255],
    });
    // glTF metallic-roughness: G = roughness, B = metallic.
    mesh.metallic_roughness_image = Some(CpuImage {
        width: 1,
        height: 1,
        pixels: vec![0, 80, 200, 255],
    });
    mesh.normal_image = Some(CpuImage {
        width: 1,
        height: 1,
        pixels: vec![128, 128, 255, 255],
    });
    mesh
}

fn draw(renderer: &mut Renderer) {
    let device = renderer.device().clone();
    let queue = renderer.queue().clone();
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bindless color"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("bindless draw"),
    });
    renderer.draw_into(&view, None, &mut encoder).expect("draw");
    queue.submit(std::iter::once(encoder.finish()));
}

#[test]
fn bind_group_path_draws_when_bindless_is_forced_off() {
    pollster::block_on(async {
        let mut renderer = Renderer::new_headless(64, 64).await.expect("renderer");
        renderer.set_bindless_enabled(false);
        let instance = Instance::from_pos_scale_color(glam::Vec3::ZERO, glam::Vec3::ONE, [1.0; 4]);
        renderer.add_composite_model("forced-off", vec![triangle()], &[instance]);
        assert_eq!(renderer.bindless_material_index("forced-off"), None);
        draw(&mut renderer);
        assert!(!renderer.is_device_lost());
    });
}

#[test]
fn bindless_registration_records_an_ssbo_index_when_supported() {
    pollster::block_on(async {
        let mut renderer = Renderer::new_headless(64, 64).await.expect("renderer");
        let instance = Instance::from_pos_scale_color(glam::Vec3::ZERO, glam::Vec3::ONE, [1.0; 4]);
        renderer.add_composite_model("surfaced", vec![triangle()], &[instance]);
        if renderer.bindless_supported() {
            assert!(
                renderer.bindless_material_index("surfaced").is_some(),
                "a supported device records the material in the SSBO"
            );
        } else {
            assert_eq!(renderer.bindless_material_index("surfaced"), None);
        }
        draw(&mut renderer);
        assert!(!renderer.is_device_lost());
    });
}
