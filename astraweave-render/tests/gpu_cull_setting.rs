//! GPU cull is off until a caller opts in. Opting in dispatches the indirect
//! frustum pass from `draw_into` without a CPU readback.

use astraweave_render::{Instance, Renderer};

fn color_target(device: &wgpu::Device) -> wgpu::TextureView {
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gpu cull color"),
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
    target.create_view(&wgpu::TextureViewDescriptor::default())
}

#[test]
fn gpu_cull_stays_off_until_enabled_and_then_dispatches() {
    pollster::block_on(async {
        let mut renderer = Renderer::new_headless(64, 64)
            .await
            .expect("headless renderer");
        assert_eq!(renderer.gpu_cull_mode(), "off");
        assert_eq!(renderer.gpu_cull_reason(), "quality_off");

        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = [[0.0, 0.0, 1.0]; 3];
        let indices = [0, 1, 2];
        let mesh = renderer.create_mesh_from_arrays(&positions, &normals, &indices);
        let instance = Instance::from_pos_scale_color(
            glam::Vec3::new(0.0, 0.0, 2.0),
            glam::Vec3::ONE,
            [1.0, 1.0, 1.0, 1.0],
        );
        renderer.add_model_with_bounds(
            "tri",
            mesh,
            &[instance],
            [-1.0, -1.0, 1.0],
            [2.0, 1.0, 3.0],
        );
        let device = renderer.device().clone();
        let view = color_target(&device);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gpu cull off"),
        });
        renderer
            .draw_into(&view, None, &mut encoder)
            .expect("draw off");
        drop(encoder);
        assert_eq!(renderer.gpu_cull_mode(), "off");

        renderer.set_gpu_cull_enabled(true);
        assert_eq!(renderer.gpu_cull_mode(), "gpu");
        assert_eq!(renderer.gpu_cull_reason(), "indirect");
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gpu cull on"),
        });
        renderer
            .draw_into(&view, None, &mut encoder)
            .expect("draw on");
        renderer.queue().submit(std::iter::once(encoder.finish()));
        let _ = renderer.device().poll(wgpu::MaintainBase::Wait);
        assert!(renderer.gpu_cull_ran());
        assert_eq!(renderer.gpu_cull_batch_count(), 1);
        assert_eq!(renderer.gpu_cull_mode(), "gpu");
    });
}
