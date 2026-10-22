//! A written timestamp pair must come back as a positive duration.
//! Zero is an unread query, not a fast pass.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use std::future::Future;

#[path = "support/gpu_capability.rs"]
mod gpu_capability;

use astraweave_render::Renderer;
use wgpu::util::DeviceExt;

#[test]
fn recorded_pass_timestamp_is_positive() {
    pollster::block_on(async {
        let Some(mut renderer) = Renderer::headless_for_tests(64, 64).await else {
            return;
        };
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
        let Some(mut renderer) = Renderer::headless_for_tests(256, 256).await else {
            return;
        };
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
        // The budget tool turns this emitter on. Earlier lavapipe timestamp
        // passes did not, so they never bound scene depth inside the particle
        // pass that also attached it for writing.
        renderer
            .set_particle_emitter(Some(budget_emitter()))
            .expect("particles");
        let exec_src = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("heavy exec src"),
            contents: &astraweave_render::gpu_profiler::FRAME_EXEC_MAGIC.to_le_bytes(),
            usage: wgpu::BufferUsages::COPY_SRC,
        });
        let exec_dst = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("heavy exec dst"),
            size: 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
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
        let mut first_query_set = None;
        for frame_index in 0..2 {
            device.push_error_scope(wgpu::ErrorFilter::Validation);
            device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("heavy frame"),
            });
            renderer
                .draw_into(&view, None, &mut encoder)
                .expect("draw_into");
            encoder.copy_buffer_to_buffer(&exec_src, 0, &exec_dst, 0, 4);
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
            let oom = pop_scope(&device);
            let validation = pop_scope(&device);
            assert!(
                oom.is_none() && validation.is_none(),
                "frame {frame_index} frame_scope validation={} out_of_memory={}",
                validation.unwrap_or_else(|| "none".to_string()),
                oom.unwrap_or_else(|| "none".to_string())
            );
            let word = read_exec_word(&device, &exec_dst).expect("gpu_exec map");
            let magic = astraweave_render::gpu_profiler::FRAME_EXEC_MAGIC;
            assert_eq!(
                word, magic,
                "frame {frame_index} gpu_exec=0x{word:08X}, expected 0x{magic:08X}"
            );
            eprintln!("frame={frame_index} gpu_exec=0x{word:08X}");
        }
    });
}

/// The isolated write/resolve/copy/wait/read path must return a non-zero tick.
/// Lavapipe is the CI stand-in. A zero pair is the Vulkan same-encoder bug.
#[test]
fn timestamp_readback_self_test_is_nonzero() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::from_env_or_default());
        let adapter = match instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
        {
            Ok(adapter) => adapter,
            Err(err) => {
                let reason = format!("timestamp self-test adapter: {err}");
                if gpu_capability::skip_if_unavailable() {
                    gpu_capability::note_skip(&reason);
                    return;
                }
                panic!("Failed to find adapter: {reason}");
            }
        };
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

/// The Aravath window calls `render_with` → `draw_into` with fog and light
/// shafts on. Those passes sample depth from compute after the geometry pass
/// closes. This frame must not be rejected.
#[test]
fn aravath_draw_into_has_no_depth_conflict() {
    pollster::block_on(async {
        let Some(mut renderer) = Renderer::headless_for_tests(128, 128).await else {
            return;
        };
        if !astraweave_render::gpu_fault::depth_texture_load_supported(renderer.device()) {
            eprintln!("skip aravath_draw_into_has_no_depth_conflict: no depth textureLoad");
            return;
        }
        renderer.set_shadows_enabled(true);
        renderer.set_volumetric_fog_config(astraweave_render::VolumetricFogPass::gameplay_config());
        renderer.set_god_ray_config(astraweave_render::GodRayConfig {
            enabled: true,
            half_res: true,
            num_samples: 32,
            exposure: 0.22,
            ..astraweave_render::GodRayConfig::default()
        });
        assert!(
            renderer.volumetric_fog_enabled(),
            "Aravath fog did not stay enabled"
        );
        assert!(
            renderer.god_rays_enabled(),
            "Aravath light shafts did not stay enabled"
        );
        let device = renderer.device().clone();
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("aravath color"),
            size: wgpu::Extent3d {
                width: 128,
                height: 128,
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
        let exec_src = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("aravath exec src"),
            contents: &astraweave_render::gpu_profiler::FRAME_EXEC_MAGIC.to_le_bytes(),
            usage: wgpu::BufferUsages::COPY_SRC,
        });
        let exec_dst = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("aravath exec dst"),
            size: 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("aravath frame"),
        });
        renderer
            .draw_into(&view, None, &mut encoder)
            .expect("draw_into");
        encoder.copy_buffer_to_buffer(&exec_src, 0, &exec_dst, 0, 4);
        renderer
            .submit_timestamped(encoder.finish())
            .expect("submit");
        let oom = pop_scope(&device);
        let validation = pop_scope(&device);
        assert!(
            oom.is_none() && validation.is_none(),
            "aravath frame_scope validation={} out_of_memory={}",
            validation.unwrap_or_else(|| "none".to_string()),
            oom.unwrap_or_else(|| "none".to_string())
        );
        let word = read_exec_word(&device, &exec_dst).expect("gpu_exec map");
        let magic = astraweave_render::gpu_profiler::FRAME_EXEC_MAGIC;
        assert_eq!(word, magic, "aravath gpu_exec=0x{word:08X}");
        eprintln!("aravath gpu_exec=0x{word:08X}");
    });
}

fn budget_emitter() -> astraweave_render::gpu_particles::EmitterParams {
    astraweave_render::gpu_particles::EmitterParams {
        position: [0.0, 1.0, 0.0, 1.0],
        velocity: [0.0, 1.0, 0.0, 0.0],
        emission_rate: 8.0,
        lifetime: 2.0,
        velocity_randomness: 0.25,
        delta_time: 1.0 / 60.0,
        gravity: [0.0, -9.8, 0.0, 0.0],
        particle_count: 32,
        max_particles: 256,
        random_seed: 7,
        _padding: 0,
    }
}

struct ScopeWaker;

impl std::task::Wake for ScopeWaker {
    fn wake(self: std::sync::Arc<Self>) {}
}

fn pop_scope(device: &wgpu::Device) -> Option<String> {
    let waker = std::task::Waker::from(std::sync::Arc::new(ScopeWaker));
    let mut future = std::pin::pin!(device.pop_error_scope());
    for _ in 0..64 {
        let mut context = std::task::Context::from_waker(&waker);
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(Some(err)) => return Some(err.to_string()),
            std::task::Poll::Ready(None) => return None,
            std::task::Poll::Pending => {
                if device.poll(wgpu::PollType::Poll).is_err() {
                    return Some("device.poll failed while popping an error scope".to_string());
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }
    Some("error scope did not resolve".to_string())
}

fn read_exec_word(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Result<u32, String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result.map_err(|err| err.to_string()));
        });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        device
            .poll(wgpu::PollType::Poll)
            .map_err(|err| format!("device.poll failed while mapping gpu_exec: {err}"))?;
        match receiver.try_recv() {
            Ok(Ok(())) => break,
            Ok(Err(err)) => return Err(err),
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                if std::time::Instant::now() >= deadline {
                    return Err("gpu_exec map timed out".to_string());
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                return Err("gpu_exec map closed".to_string());
            }
        }
    }
    let data = buffer.slice(..).get_mapped_range();
    if data.len() < 4 {
        let len = data.len();
        drop(data);
        buffer.unmap();
        return Err(format!("gpu_exec map is {len} bytes"));
    }
    let word = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    drop(data);
    buffer.unmap();
    Ok(word)
}
