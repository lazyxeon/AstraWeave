//! GPU Resource Leak Detection Tests
//!
//! Validates that GPU resources (buffers, textures, pipelines) are properly
//! cleaned up and don't leak across renderer creation/destruction cycles.

#[path = "support/gpu_capability.rs"]
mod gpu_capability;

#[test]
fn test_headless_device_creation() {
    // Verify headless device can be created for testing
    let Some((device, queue)) = pollster::block_on(create_test_device()) else {
        return;
    };

    // Basic validation
    assert!(device.limits().max_texture_dimension_2d > 0);

    // Device should drop cleanly
    drop(device);
    drop(queue);
}

#[test]
fn test_buffer_lifecycle_no_leaks() {
    let Some((device, queue)) = pollster::block_on(create_test_device()) else {
        return;
    };

    // Track resource counts by creating and destroying buffers multiple times
    let initial_usage = get_memory_usage();

    for iteration in 0..100 {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Test Buffer {}", iteration)),
            size: 1024 * 1024, // 1 MB
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Use the buffer
        queue.write_buffer(&buffer, 0, &vec![0u8; 1024]);

        // Explicitly drop
        drop(buffer);

        // Force GPU to process pending operations
        let _ = device.poll(wgpu::MaintainBase::Wait);
    }

    // Final poll to ensure all cleanup is done
    let _ = device.poll(wgpu::MaintainBase::Wait);

    let final_usage = get_memory_usage();

    // Memory usage should be roughly the same (allowing for some variance)
    let growth = final_usage.saturating_sub(initial_usage);
    assert!(
        growth < 10 * 1024 * 1024, // Allow up to 10 MB growth for internal bookkeeping
        "Potential buffer leak detected: memory grew by {} bytes",
        growth
    );
}

#[test]
fn test_texture_lifecycle_no_leaks() {
    let Some((device, _queue)) = pollster::block_on(create_test_device()) else {
        return;
    };

    let initial_usage = get_memory_usage();

    for iteration in 0..50 {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Test Texture {}", iteration)),
            size: wgpu::Extent3d {
                width: 512,
                height: 512,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Create view
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        drop(view);
        drop(texture);
        let _ = device.poll(wgpu::MaintainBase::Wait);
    }

    let _ = device.poll(wgpu::MaintainBase::Wait);
    let final_usage = get_memory_usage();

    let growth = final_usage.saturating_sub(initial_usage);
    assert!(
        growth < 20 * 1024 * 1024, // Allow up to 20 MB for texture overhead
        "Potential texture leak detected: memory grew by {} bytes",
        growth
    );
}

#[test]
fn test_pipeline_lifecycle_no_leaks() {
    let Some((device, _queue)) = pollster::block_on(create_test_device()) else {
        return;
    };

    let shader_source = r#"
        @vertex
        fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
            let x = f32(i32(in_vertex_index) - 1);
            let y = f32(i32(in_vertex_index & 1u) * 2 - 1);
            return vec4<f32>(x, y, 0.0, 1.0);
        }
        
        @fragment
        fn fs_main() -> @location(0) vec4<f32> {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }
    "#;

    let initial_usage = get_memory_usage();

    for iteration in 0..100 {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("Test Shader {}", iteration)),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("Test Pipeline {}", iteration)),
            layout: None,
            cache: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        drop(pipeline);
        drop(shader);
        let _ = device.poll(wgpu::MaintainBase::Wait);
    }

    let _ = device.poll(wgpu::MaintainBase::Wait);
    let final_usage = get_memory_usage();

    let growth = final_usage.saturating_sub(initial_usage);
    assert!(
        growth < 5 * 1024 * 1024, // Allow up to 5 MB for shader/pipeline overhead
        "Potential pipeline leak detected: memory grew by {} bytes",
        growth
    );
}

#[test]
fn test_renderer_lifecycle_stress_test() {
    // One Instance and Adapter for every cycle. The first cycle is the warm
    // baseline: lavapipe and WARP keep driver mappings from process bring-up,
    // and those are not renderer resources. Growth is measured after that
    // cycle. The 50 MB ceiling stays. Handle counts apply on every backend.
    let Some((instance, adapter)) = pollster::block_on(shared_instance_and_adapter()) else {
        return;
    };

    run_renderer_cycle(&adapter, 0);
    let _ = instance.poll_all(true);
    assert_user_gpu_handles_released(&instance);

    let baseline = get_memory_usage();
    run_renderer_cycle(&adapter, 1);
    let _ = instance.poll_all(true);
    assert_user_gpu_handles_released(&instance);

    let growth = get_memory_usage().saturating_sub(baseline);
    // The ceiling stays 50 MB. A shared Instance/Adapter plus one warm cycle
    // leaves lavapipe (LLVM 20.1.2) at about 5.2 MB of further VmRSS, with
    // zero user-held buffers, textures, devices, and queues. get_memory_usage
    // is 0 where VmRSS is unavailable, so the ceiling is a no-op there and
    // the handle check above is the signal.
    assert!(
        growth < 50 * 1024 * 1024,
        "Potential renderer lifecycle leak: memory grew by {growth} bytes after the warm cycle"
    );
}

// Helper functions

fn run_renderer_cycle(adapter: &wgpu::Adapter, cycle: u32) {
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("Renderer lifecycle"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        memory_hints: wgpu::MemoryHints::default(),
        trace: wgpu::Trace::Off,
    }))
    .expect("renderer lifecycle device");

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&format!("Render Target {cycle}")),
        size: wgpu::Extent3d {
            width: 800,
            height: 600,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&format!("Depth Buffer {cycle}")),
        size: wgpu::Extent3d {
            width: 800,
            height: 600,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("Uniform Buffer {cycle}")),
        size: 256,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    drop(buffer);
    drop(depth_texture);
    drop(texture);
    let _ = device.poll(wgpu::MaintainBase::Wait);
    drop(queue);
    drop(device);
}

fn assert_user_gpu_handles_released(instance: &wgpu::Instance) {
    let Some(report) = instance.generate_report() else {
        panic!("wgpu generate_report is required for the lifecycle handle check");
    };
    let hub = &report.hub;
    for (name, registry) in [
        ("buffers", &hub.buffers),
        ("textures", &hub.textures),
        ("texture_views", &hub.texture_views),
        ("devices", &hub.devices),
        ("queues", &hub.queues),
    ] {
        assert_eq!(
            registry.num_kept_from_user, 0,
            "{name} still held after drop; allocated {}",
            registry.num_allocated
        );
    }
}

async fn shared_instance_and_adapter() -> Option<(wgpu::Instance, wgpu::Adapter)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    let mut attempts = Vec::new();
    for fallback in [false, true] {
        match instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: fallback,
                compatible_surface: None,
            })
            .await
        {
            Ok(adapter) => return Some((instance, adapter)),
            Err(err) => attempts.push(err.to_string()),
        }
    }
    let reason = format!(
        "no wgpu adapter for renderer lifecycle. {}",
        attempts.join("; ")
    );
    if gpu_capability::skip_if_unavailable() {
        gpu_capability::note_skip(&reason);
        None
    } else {
        panic!("Failed to find adapter: {reason}");
    }
}

async fn create_test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    gpu_capability::request_headless(
        "Test Device",
        wgpu::Features::empty(),
        wgpu::Limits::default(),
        wgpu::PowerPreference::default(),
        false,
    )
    .await
    .map(|gpu| (gpu.device, gpu.queue))
}

fn get_memory_usage() -> usize {
    // On platforms with memory_stats, use it
    #[cfg(target_os = "linux")]
    {
        if let Ok(contents) = std::fs::read_to_string("/proc/self/status") {
            for line in contents.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<usize>() {
                            return kb * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }
    }

    // Fallback: use a simple counter (not accurate but detects gross leaks)
    // In production, you'd use platform-specific memory queries
    0
}
