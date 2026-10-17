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
        renderer
            .submit_timestamped(encoder.finish())
            .expect("timestamp submit");
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

#[test]
fn draw_into_heavy_path_timestamps_and_models() {
    pollster::block_on(async {
        let mut renderer = Renderer::new_headless(256, 256).await.expect("headless");
        if !astraweave_render::gpu_fault::depth_texture_load_supported(renderer.device()) {
            eprintln!("skip draw_into_heavy_path: no depth textureLoad");
            return;
        }
        let device = renderer.device().clone();
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let normals = [[0.0, 0.0, 1.0]; 3];
        let indices = [0, 1, 2];
        let mesh = renderer.create_mesh_from_arrays(&positions, &normals, &indices);
        // Rogue.glb is not in the repo. The budget tool uploads one mesh, then
        // `duplicate_model` so each instance is its own model. This generated
        // triangle takes that same path. `rendered_models` counts models.
        let count = 64u32;
        let root = astraweave_render::Instance::from_pos_scale_color(
            glam::Vec3::new(-3.5 * 4.0, 0.0, -3.5 * 4.0),
            glam::Vec3::ONE,
            [1.0, 1.0, 1.0, 1.0],
        );
        renderer.add_model("budget-0", mesh, &[root]);
        for i in 1..count {
            let x = (i % 8) as f32;
            let z = (i / 8) as f32;
            let instance = astraweave_render::Instance::from_pos_scale_color(
                glam::Vec3::new((x - 3.5) * 4.0, 0.0, (z - 3.5) * 4.0),
                glam::Vec3::ONE,
                [1.0, 1.0, 1.0, 1.0],
            );
            assert!(
                renderer.duplicate_model("budget-0", format!("budget-{i}"), instance),
                "duplicate {i}"
            );
        }
        renderer.set_shadows_enabled(true);
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("heavy color"),
            size: wgpu::Extent3d {
                width: 256,
                height: 256,
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
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut first_query_set = None;
        for frame_index in 0..2 {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("heavy frame"),
            });
            renderer
                .draw_into(&view, None, &mut encoder)
                .expect("draw_into");
            renderer
                .submit_timestamped(encoder.finish())
                .expect("timestamp submit");
            renderer.request_gpu_profiler_readback();
            renderer
                .gpu_profiler_mut()
                .expect("profiler")
                .block_on_readback(&device)
                .expect("readback");
            let report = renderer
                .gpu_profiler()
                .expect("profiler")
                .timestamp_report();
            eprintln!(
                "frame={frame_index} rendered_models={}",
                renderer.rendered_model_count()
            );
            eprintln!("{report}");
            assert_eq!(
                renderer.rendered_model_count(),
                count,
                "frame {frame_index}\n{report}"
            );
            let profiler = renderer.gpu_profiler().expect("profiler");
            assert!(!profiler.results().is_empty(), "{report}");
            // `submit_timestamped` is the budget tool's submit/poll path,
            // including the 5s wait. Empty is a real zero-length pass.
            let mut measured = false;
            for timing in profiler.results() {
                assert!(
                    matches!(
                        timing.kind,
                        astraweave_render::gpu_profiler::TimestampKind::Measured
                            | astraweave_render::gpu_profiler::TimestampKind::Empty
                    ),
                    "frame {frame_index} pass {} is {:?}\n{report}",
                    timing.name,
                    timing.kind
                );
                if timing.kind == astraweave_render::gpu_profiler::TimestampKind::Measured {
                    measured = true;
                }
            }
            assert!(
                measured,
                "frame {frame_index} wrote no positive span\n{report}"
            );
            let trace_report = profiler.submission_report();
            eprintln!("{trace_report}");
            let trace = profiler.stamp_trace();
            let resolve = trace
                .iter()
                .find(|stamp| stamp.kind == astraweave_render::gpu_profiler::StampKind::Resolve)
                .unwrap_or_else(|| panic!("draw_into did not resolve\n{trace_report}"));
            let writes: Vec<_> = trace
                .iter()
                .filter(|stamp| stamp.kind == astraweave_render::gpu_profiler::StampKind::Write)
                .collect();
            assert!(
                !writes.is_empty(),
                "draw_into recorded no timestamp writes\n{trace_report}"
            );
            assert_ne!(resolve.encoder, 0, "{trace_report}");
            for write in writes {
                assert_eq!(
                    write.submit, resolve.submit,
                    "write {} index {} is submit {}, resolve is submit {}\n{trace_report}",
                    write.label, write.index, write.submit, resolve.submit
                );
                assert_eq!(
                    write.query_set, resolve.query_set,
                    "write {} query_set {} != resolve query_set {}\n{trace_report}",
                    write.label, write.query_set, resolve.query_set
                );
                assert_eq!(
                    write.encoder, resolve.encoder,
                    "write {} encoder {} != resolve encoder {}\n{trace_report}",
                    write.label, write.encoder, resolve.encoder
                );
                assert_ne!(write.submit, 0, "{trace_report}");
            }
            assert!(
                trace_report.contains("encoder_count=1\n"),
                "writes and resolve must be one encoder\n{trace_report}"
            );
            assert!(trace_report.contains("mapped_buffer="), "{trace_report}");
            assert!(trace_report.contains("raw_u64="), "{trace_report}");
            if frame_index == 0 {
                first_query_set = Some(profiler.stamp_trace()[0].query_set);
                renderer.recreate_timestamp_set().expect("fresh query set");
            } else if let Some(previous) = first_query_set {
                assert_ne!(
                    profiler.stamp_trace()[0].query_set,
                    previous,
                    "recreate must replace the query set\n{trace_report}"
                );
            }
        }
        let err = device.pop_error_scope().await;
        if let Some(err) = err {
            panic!("validation: {err}");
        }
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
        assert!(
            report.contains("encoder_count=1\n"),
            "self-test writes and resolve must share one encoder\n{report}"
        );
        println!("{report}");
        let err = device.pop_error_scope().await;
        assert!(
            err.is_none(),
            "timestamp self-test validation: {err:?}\n{report}"
        );
    });
}
