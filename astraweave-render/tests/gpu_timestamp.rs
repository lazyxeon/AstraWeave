//! A written timestamp pair must come back as a positive duration.
//! Zero is an unread query, not a fast pass.

use astraweave_render::Renderer;

#[test]
fn recorded_pass_timestamp_is_positive() {
    pollster::block_on(async {
        let mut renderer = Renderer::new_headless(64, 64).await.expect("headless");
        if !astraweave_render::gpu_fault::depth_texture_load_supported(renderer.device()) {
            eprintln!(
                "skip recorded_pass_timestamp_is_positive: GL cannot textureLoad depth, so this frame is not the lavapipe measurement"
            );
            return;
        }
        assert!(
            renderer.timestamps_supported(),
            "lavapipe should expose TIMESTAMP_QUERY with a positive period; adapter={} backend={} features={:?}",
            renderer.gpu_adapter_name(),
            renderer.gpu_backend(),
            renderer.gpu_features()
        );
        let device = renderer.device().clone();
        let queue = renderer.queue().clone();
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("timestamp color"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: renderer.surface_format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = color.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("timestamp frame"),
        });
        renderer
            .draw_into(&view, None, &mut encoder)
            .expect("draw_into");
        queue.submit(std::iter::once(encoder.finish()));
        renderer.request_gpu_profiler_readback();
        renderer
            .gpu_profiler_mut()
            .expect("profiler")
            .block_on_readback(&device)
            .expect("readback");
        let total = renderer
            .gpu_profiler()
            .expect("profiler")
            .validated_total_ms()
            .expect("positive GPU duration");
        assert!(total > 0.0, "GPU duration {total} ms");
    });
}
