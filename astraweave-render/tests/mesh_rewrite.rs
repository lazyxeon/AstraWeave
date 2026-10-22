//! A deformed mesh rewrites GPU buffers in place when the counts still fit.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::mesh::{CpuMesh, MeshVertex};
use astraweave_render::{Instance, Renderer};

fn quad(lift: f32) -> CpuMesh {
    let mut mesh = CpuMesh::default();
    mesh.vertices = vec![
        MeshVertex::from_arrays(
            [0.0, lift, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 0.0],
        ),
        MeshVertex::from_arrays(
            [1.0, lift, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0],
        ),
        MeshVertex::from_arrays(
            [1.0, lift, 1.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 1.0],
        ),
        MeshVertex::from_arrays(
            [0.0, lift, 1.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0],
        ),
    ];
    mesh.indices = vec![0, 2, 1, 0, 3, 2];
    mesh
}

#[test]
fn write_mesh_keeps_buffers_when_the_deformation_fits() {
    pollster::block_on(async {
        let Some(mut renderer) = Renderer::headless_for_tests(64, 64).await else {
            return;
        };
        let mesh = renderer.create_mesh_from_cpu_mesh(&quad(0.0));
        let instance = Instance::from_pos_scale_color(glam::Vec3::ZERO, glam::Vec3::ONE, [1.0; 4]);
        renderer.add_model_with_bounds(
            "plate",
            mesh,
            &[instance],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
        );
        let generation = renderer.mesh_storage_generation("plate").expect("model");

        renderer
            .write_mesh("plate", &quad(0.25))
            .expect("first deformation");
        assert_eq!(renderer.mesh_storage_generation("plate"), Some(generation));

        let device = renderer.device().clone();
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mesh rewrite color"),
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
            label: Some("mesh rewrite"),
        });
        renderer.draw_into(&view, None, &mut encoder).expect("draw");

        renderer
            .write_mesh("plate", &quad(0.5))
            .expect("second deformation");
        assert_eq!(
            renderer.mesh_storage_generation("plate"),
            Some(generation),
            "in-capacity deformation must not recreate the GPU buffers"
        );

        let mut grown = quad(0.5);
        grown.vertices.push(MeshVertex::from_arrays(
            [0.5, 0.5, 0.5],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.5, 0.5],
        ));
        grown.indices.extend_from_slice(&[0, 1, 4]);
        renderer.write_mesh("plate", &grown).expect("growth");
        assert_eq!(
            renderer.mesh_storage_generation("plate"),
            Some(generation + 1),
            "growth replaces the GPU buffers"
        );
    });
}
