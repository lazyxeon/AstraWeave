//! The pipeline helper creates a pipeline when no pipeline cache exists.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

#[path = "support/gpu_capability.rs"]
mod gpu_capability;

use astraweave_render::pipeline_cache;

#[test]
fn fallback_without_pipeline_cache_still_creates_pipeline() {
    pollster::block_on(async {
        let Some(gpu) = gpu_capability::request_headless(
            "fallback_without_pipeline_cache_still_creates_pipeline",
            wgpu::Features::empty(),
            wgpu::Limits::downlevel_defaults(),
            wgpu::PowerPreference::HighPerformance,
            false,
        )
        .await
        else {
            return;
        };
        let device = gpu.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fallback shader"),
            source: wgpu::ShaderSource::Wgsl(
                "@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }
                 @fragment fn fs() -> @location(0) vec4<f32> { return vec4<f32>(1.0, 0.0, 0.0, 1.0); }"
                    .into(),
            ),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fallback layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let pipeline = pipeline_cache::create_render_pipeline(
            &device,
            None,
            wgpu::RenderPipelineDescriptor {
                label: Some("fallback pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            },
        );
        drop(pipeline);
    });
}
