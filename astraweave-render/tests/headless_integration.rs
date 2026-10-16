//! Headless smoke test: load one biome pack, create canonical BGLs, make a minimal pipeline, ensure no panics.

use std::path::PathBuf;

#[path = "support/gpu_capability.rs"]
mod gpu_capability;

#[test]
#[cfg(feature = "textures")]
fn headless_biome_pack_and_pipeline_compat() {
    // Need at least 5 bind groups (camera, materials, shadow, light, ibl)
    let mut limits = wgpu::Limits::downlevel_webgl2_defaults();
    limits.max_bind_groups = limits.max_bind_groups.max(5);
    let Some(gpu) = pollster::block_on(gpu_capability::request_headless(
        "headless-device",
        wgpu::Features::empty(),
        limits,
        wgpu::PowerPreference::LowPower,
        false,
    )) else {
        return;
    };
    let device = gpu.device;
    let queue = gpu.queue;

    // Build material BGL using integrator’s layout (same as examples)
    let material_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("pbr-material-layers"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
        ],
    });

    // Load one real biome pack if available; otherwise synthesize minimal content
    let assets_root = PathBuf::from("assets/materials/grassland");
    let (gpu, _stats) = if assets_root.exists() {
        let mut mm = astraweave_render::MaterialManager::new();
        let mats = assets_root.join("materials.toml");
        let arrays = assets_root.join("arrays.toml");
        pollster::block_on(mm.load_pack_from_toml(&device, &queue, &assets_root, &mats, &arrays))
            .expect("load pack")
    } else {
        // Fallback to tiny synthetic pack written to temp dir
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();
        std::fs::write(
            base.join("materials.toml"),
            "[biome]\nname=\"x\"\n[[layer]]\nkey=\"k\"\n",
        )
        .unwrap();
        std::fs::write(base.join("arrays.toml"), "[layers]\nk=0\n").unwrap();
        let mut mm = astraweave_render::MaterialManager::new();
        pollster::block_on(mm.load_pack_from_toml(
            &device,
            &queue,
            &base,
            &base.join("materials.toml"),
            &base.join("arrays.toml"),
        ))
        .expect("load pack synthetic")
    };

    // Create a dummy pipeline that references the material group layout alongside camera/light/ibl groups
    let camera_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("camera-bgl"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let shadow_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("shadow-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Depth,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            },
        ],
    });
    let light_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("light-bgl"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let ibl_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("ibl-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("pl-headless"),
        bind_group_layouts: &[
            &camera_bgl,
            &material_bgl,
            &shadow_bgl,
            &light_bgl,
            &ibl_bgl,
        ],
        push_constant_ranges: &[],
    });

    // Minimal shader that compiles with the layout (vertex-only)
    let sm = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("sm-headless"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
            "struct Camera{view_proj:mat4x4<f32>};@group(0)@binding(0)var<uniform>cam:Camera;@vertex fn vs(@builtin(vertex_index)vi:u32)->@builtin(position)vec4<f32>{var pos=array<vec2<f32>,3>(vec2<f32>(-1.0,-1.0),vec2<f32>(3.0,-1.0),vec2<f32>(-1.0,3.0));return cam.view_proj*vec4<f32>(pos[vi],0.0,1.0);} @fragment fn fs()->@location(0) vec4<f32>{return vec4<f32>(0.5,0.5,0.5,1.0);}"
        )),
    });
    let _pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("rp-headless"),
        layout: Some(&pl),
        vertex: wgpu::VertexState {
            module: &sm,
            entry_point: Some("vs"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &sm,
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
    });

    // Ensure we can create a bind group against the materials layout without mismatch
    let _bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("materials-bg-smoke"),
        layout: &material_bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&gpu.albedo),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&gpu.sampler_albedo),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&gpu.normal),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(&gpu.sampler_linear),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(&gpu.mra),
            },
        ],
    });
}
