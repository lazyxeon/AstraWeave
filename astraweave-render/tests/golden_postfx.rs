#[path = "support/gpu_capability.rs"]
mod gpu_capability;

// no extra imports needed

fn aces_tonemap(x: [f32; 3]) -> [f32; 3] {
    let a = 2.51f32;
    let b = 0.03f32;
    let c = 2.43f32;
    let d = 0.59f32;
    let e = 0.14f32;
    let f = |v: f32| ((v * (a * v + b)) / (v * (c * v + d) + e)).clamp(0.0, 1.0);
    [f(x[0]), f(x[1]), f(x[2])]
}

fn srgb_encode_u8(x: f32) -> u8 {
    let x = x.clamp(0.0, 1.0);
    let srgb = if x <= 0.003_130_8 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    };
    (srgb * 255.0 + 0.5).floor().clamp(0.0, 255.0) as u8
}

fn image_delta(a: &[u8], b: &[u8]) -> (u8, f32) {
    assert_eq!(a.len(), b.len());
    let mut sum = 0u64;
    let mut maxd = 0u8;
    for i in 0..a.len() {
        let d = a[i].abs_diff(b[i]);
        sum += d as u64;
        if d > maxd {
            maxd = d;
        }
    }
    (maxd, (sum as f32) / (a.len() as f32))
}

const POST_SHADER_FX: &str = r#"
struct VSOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VSOut {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>( 3.0,  1.0),
        vec2<f32>(-1.0,  1.0)
    );
    var out: VSOut;
    out.pos = vec4<f32>(pos[vid], 0.0, 1.0);
    // Map NDC to UV with Y flipped so that v increases downward to match CPU reference
    out.uv = vec2<f32>((pos[vid].x + 1.0) * 0.5, (1.0 - pos[vid].y) * 0.5);
    return out;
}

@group(0) @binding(0) var hdr_tex: texture_2d<f32>;
@group(0) @binding(1) var ao_tex: texture_2d<f32>;
@group(0) @binding(2) var gi_tex: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;

fn aces_tonemap(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51; let b = 0.03; let c = 2.43; let d = 0.59; let e = 0.14;
    return clamp((x*(a*x+b))/(x*(c*x+d)+e), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let hdr = textureSampleLevel(hdr_tex, samp, in.uv, 0.0).rgb;
    let ao = textureSampleLevel(ao_tex, samp, in.uv, 0.0).r;
    let gi = textureSampleLevel(gi_tex, samp, in.uv, 0.0).rgb;
    let comp = hdr * (1.0 - ao * 0.6) + gi * 0.2;
    let mapped = aces_tonemap(comp);
    return vec4<f32>(mapped, 1.0);
}
"#;

#[test]
fn golden_postfx_compose_matches_cpu() {
    let w = 64u32;
    let h = 32u32;
    // Init wgpu (headless)
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    })) {
        Ok(adapter) => adapter,
        Err(err) => {
            let reason = format!("no wgpu adapter: {err}");
            if gpu_capability::skip_if_unavailable() {
                gpu_capability::note_skip(&reason);
                return;
            }
            panic!("Failed to find adapter: {reason}");
        }
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("golden-postfx device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
        memory_hints: wgpu::MemoryHints::default(),
        trace: wgpu::Trace::Off,
    }))
    .expect("device");

    // Source textures (sampled as float from unorm)
    let tex_usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
    let hdr_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hdr src"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: tex_usage,
        view_formats: &[],
    });
    let ao_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ao src"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: tex_usage,
        view_formats: &[],
    });
    let gi_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gi src"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: tex_usage,
        view_formats: &[],
    });
    let hdr_view = hdr_tex.create_view(&wgpu::TextureViewDescriptor::default());
    let ao_view = ao_tex.create_view(&wgpu::TextureViewDescriptor::default());
    let gi_view = gi_tex.create_view(&wgpu::TextureViewDescriptor::default());
    let samp = device.create_sampler(&wgpu::SamplerDescriptor::default());

    // Fill textures
    let mut hdr_bytes = vec![0u8; (w * h * 4) as usize];
    let mut ao_bytes = vec![0u8; (w * h * 4) as usize];
    let mut gi_bytes = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / (w as f32 - 1.0);
            let v = y as f32 / (h as f32 - 1.0);
            let idx = ((y * w + x) * 4) as usize;
            // hdr rgb in [0,1] encoded into unorm
            hdr_bytes[idx] = (u * 255.0).round().clamp(0.0, 255.0) as u8;
            hdr_bytes[idx + 1] = (v * 255.0).round().clamp(0.0, 255.0) as u8;
            hdr_bytes[idx + 2] = (0.25f32 * 255.0f32).round() as u8;
            hdr_bytes[idx + 3] = 255;
            // ao in red channel
            let ao_lin = ((u + v) * 0.5).clamp(0.0, 1.0);
            ao_bytes[idx] = (ao_lin * 255.0).round() as u8;
            ao_bytes[idx + 1] = 0;
            ao_bytes[idx + 2] = 0;
            ao_bytes[idx + 3] = 255;
            // gi constant color
            gi_bytes[idx] = (0.10f32 * 255.0f32).round() as u8;
            gi_bytes[idx + 1] = (0.05f32 * 255.0f32).round() as u8;
            gi_bytes[idx + 2] = 0;
            gi_bytes[idx + 3] = 255;
        }
    }
    let layout = wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(w * 4),
        rows_per_image: Some(h),
    };
    let extent = wgpu::Extent3d {
        width: w,
        height: h,
        depth_or_array_layers: 1,
    };
    queue.write_texture(hdr_tex.as_image_copy(), &hdr_bytes, layout, extent);
    queue.write_texture(ao_tex.as_image_copy(), &ao_bytes, layout, extent);
    queue.write_texture(gi_tex.as_image_copy(), &gi_bytes, layout, extent);
    // Ensure writes are flushed before using these textures in a render pass
    queue.submit(std::iter::empty());
    let _ = device.poll(wgpu::MaintainBase::Wait);

    // Target texture (sRGB) for readback
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("target"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());

    // Pipeline for POST_SHADER_FX
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("post-fx"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(POST_SHADER_FX)),
    });
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
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
    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bg"),
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&hdr_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&ao_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&gi_view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(&samp),
            },
        ],
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("pl"),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("post-fx pipeline"),
        layout: Some(&pl),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::REPLACE),
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

    // Render
    let mut enc =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("enc") });
    {
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("rp"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        rp.set_pipeline(&pipeline);
        rp.set_bind_group(0, &bg, &[]);
        rp.draw(0..3, 0..1);
    }
    // Readback
    let bytes_pp = 4u64;
    let row_bytes = w as u64 * bytes_pp;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u64;
    let padded = row_bytes.div_ceil(align) * align;
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("read"),
        size: padded * h as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    enc.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(enc.finish()));
    let _ = device.poll(wgpu::MaintainBase::Wait);
    let slice = buf.slice(..);
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    slice.map_async(wgpu::MapMode::Read, move |res| {
        let _ = tx.send(res);
    });
    let _ = device.poll(wgpu::MaintainBase::Wait);
    rx.recv().unwrap().unwrap();
    let data = slice.get_mapped_range();
    let mut img = vec![0u8; (w * h * 4) as usize];
    for y in 0..h as usize {
        let src = y as u64 * padded;
        let dst = y * w as usize * 4;
        img[dst..dst + (w as usize * 4)]
            .copy_from_slice(&data[src as usize..(src + row_bytes) as usize]);
    }
    drop(data);
    buf.unmap();

    // Build expected
    let mut exp = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / (w as f32 - 1.0);
            let v = y as f32 / (h as f32 - 1.0);
            let hdr = [u, v, 0.25f32];
            let ao = ((u + v) * 0.5).clamp(0.0, 1.0);
            let gi = [0.10f32, 0.05f32, 0.0f32];
            let comp = [
                hdr[0] * (1.0 - ao * 0.6) + gi[0] * 0.2,
                hdr[1] * (1.0 - ao * 0.6) + gi[1] * 0.2,
                hdr[2] * (1.0 - ao * 0.6) + gi[2] * 0.2,
            ];
            let mapped = aces_tonemap(comp);
            let idx = ((y * w + x) * 4) as usize;
            exp[idx] = srgb_encode_u8(mapped[0]);
            exp[idx + 1] = srgb_encode_u8(mapped[1]);
            exp[idx + 2] = srgb_encode_u8(mapped[2]);
            exp[idx + 3] = 255;
        }
    }

    let (maxd, avg) = image_delta(&img, &exp);
    assert!(maxd <= 3, "max delta {} too high", maxd);
    assert!(avg <= 0.6, "avg delta {} too high", avg);
}
