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
        renderer.submit_timestamped(encoder.finish());
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

/// The isolated write/resolve/copy/wait/read path must return a non-zero tick.
/// Lavapipe is the CI stand-in. A zero pair is the Vulkan same-encoder bug.
#[test]
fn timestamp_readback_self_test_is_nonzero() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::from_env_or_default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("adapter");
        let info = adapter.get_info();
        let adapter_features = adapter.features();
        let needed = wgpu::Features::TIMESTAMP_QUERY
            | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
            | wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES;
        if !adapter_features.contains(needed) {
            let name = info.name.to_lowercase();
            if name.contains("llvmpipe") || name.contains("lavapipe") {
                panic!(
                    "lavapipe is missing timestamp features ({:?}); adapter={} features={:?}",
                    needed, info.name, adapter_features
                );
            }
            eprintln!(
                "skip timestamp_readback_self_test_is_nonzero: adapter={} features={:?}",
                info.name, adapter_features
            );
            return;
        }
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("timestamp-self-test"),
                required_features: needed,
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: Default::default(),
            })
            .await
            .expect("device");
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let report = astraweave_render::GpuProfiler::prove_readback(&device, &queue)
            .unwrap_or_else(|err| panic!("readback path returned zeros: {err}"));
        assert!(
            !report.contains("begin=0 end=0"),
            "readback path returned zeros:\n{report}"
        );
        println!("{report}");
        let err = device.pop_error_scope().await;
        assert!(
            err.is_none(),
            "timestamp self-test validation: {err:?}\n{report}"
        );
    });
}
