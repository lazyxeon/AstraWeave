//! L.2 — irradiance-cube face-correctness regression (GPU).
//!
//! The L.2 Phase-0 STOP (docs/audits/L2_PHASE0_STOP.md) found the irradiance
//! convolution face-blind: `IRRADIANCE_WGSL` derived its normal with z pinned
//! to 1.0 and the bake loop bound no face index, so all six irradiance faces
//! received the same +Z-hemisphere convolution.
//!
//! This test bakes the git-tracked neutral-daylight HDRI
//! (`assets/hdri/polyhaven/kloppenheim_02_puresky_2k.hdr`, catalog entry
//! `kloppenheim_daytime`), probes the six face centres of the resulting
//! irradiance cube by sampling along the principal axes, and validates them
//! against a CPU reference that integrates the source equirect with the
//! SAME quadrature and tangent basis as `IRRADIANCE_WGSL`. It asserts:
//!
//! 1. the faces are NOT all identical (max relative spread > 5%);
//! 2. +Y (sky) irradiance strictly exceeds −Y (ground);
//! 3. each GPU face-centre luma is within [0.5, 2.0]× its CPU reference;
//! 4. wherever the CPU reference separates two faces by >25%, the GPU
//!    ordering agrees;
//! 5. the BRDF LUT's scale term (A) at low roughness exceeds A at high
//!    roughness for a fixed NdotV — the LUT is not roughness-flipped.
//!
//! The fixing campaign found TWO defects, and this test discriminates both:
//!
//! * **Face-blind irradiance** — `IRRADIANCE_WGSL` pinned N.z to 1.0 with no
//!   face index bound; all six faces held the same +Z convolution
//!   (spread 0.00% at `4c139460d`).
//! * **v-flipped bake writes** — every bake pass derived `uv` from clip space
//!   without flipping v (framebuffer row 0 is NDC y=+1, texture v=0 is
//!   row 0), so cube faces were written vertically flipped versus the
//!   hardware cube convention and the BRDF LUT was roughness-flipped.
//!   Horizontal-face irradiance integrals are invariant under the y-flip
//!   (reflection isometry — which is why ±X/±Z agreed with the CPU reference
//!   to 1–2% while ±Y read 0.70×/1.58×), and the specular prefilter only
//!   *looked* right through a double-flip cancellation (flipped env read ×
//!   flipped placement write) that the equirect fix alone would have broken.
//!
//! Run explicitly:
//!
//! ```text
//! cargo test -p astraweave-render --test ibl_irradiance_faces -- --ignored --nocapture
//! ```

use anyhow::{Context, Result};
use astraweave_render::{IblManager, IblQuality, SkyMode};

const HDRI_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../assets/hdri/polyhaven/kloppenheim_02_puresky_2k.hdr"
);

/// Probe shader: a 6×1 target where pixel x samples the cube along the
/// principal axis of face x (+X, −X, +Y, −Y, +Z, −Z) — i.e. each face's
/// centre texel.
const PROBE_WGSL: &str = r#"
struct VsOut { @builtin(position) pos: vec4<f32> };
@vertex fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    var p = array<vec2<f32>, 3>(vec2<f32>(-1.0,-1.0), vec2<f32>(3.0,-1.0), vec2<f32>(-1.0,3.0));
    var out: VsOut;
    out.pos = vec4<f32>(p[vi], 0.0, 1.0);
    return out;
}
@group(0) @binding(0) var cube: texture_cube<f32>;
@group(0) @binding(1) var samp: sampler;
@fragment fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let i = i32(in.pos.x); // 0..5
    var dir = vec3<f32>(1.0, 0.0, 0.0);
    if (i == 1) { dir = vec3<f32>(-1.0, 0.0, 0.0); }
    if (i == 2) { dir = vec3<f32>( 0.0, 1.0, 0.0); }
    if (i == 3) { dir = vec3<f32>( 0.0,-1.0, 0.0); }
    if (i == 4) { dir = vec3<f32>( 0.0, 0.0, 1.0); }
    if (i == 5) { dir = vec3<f32>( 0.0, 0.0,-1.0); }
    return textureSampleLevel(cube, samp, dir, 0.0);
}
"#;

async fn acquire_device() -> Result<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .context("no suitable wgpu adapter for the irradiance-face probe")?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("ibl-irr-faces device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await
        .context("request_device failed")?;
    Ok((device, queue))
}

/// Sample the six face centres of a cube view into a 6×1 Rgba32Float target
/// and read them back.
fn probe_face_centres(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cube: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> Result<[[f32; 4]; 6]> {
    probe_six(device, queue, cube, sampler, PROBE_WGSL)
}

/// Run any 6-direction probe shader against a cube view.
fn probe_six(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cube: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    wgsl: &str,
) -> Result<[[f32; 4]; 6]> {
    let sm = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ibl-irr-probe-sm"),
        source: wgpu::ShaderSource::Wgsl(wgsl.into()),
    });
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("ibl-irr-probe-bgl"),
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
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ibl-irr-probe-bg"),
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(cube),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("ibl-irr-probe-pl"),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("ibl-irr-probe-pipeline"),
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
                format: wgpu::TextureFormat::Rgba32Float,
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

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ibl-irr-probe-target"),
        size: wgpu::Extent3d {
            width: 6,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());

    // 6 px × 16 B = 96 B payload; rows must be 256-aligned for the copy.
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ibl-irr-probe-readback"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("ibl-irr-probe-enc"),
    });
    {
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ibl-irr-probe-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
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
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 6,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(enc.finish()));

    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device
        .poll(wgpu::PollType::Wait)
        .context("device poll failed")?;
    rx.recv()
        .context("map_async channel closed")?
        .context("map_async failed")?;
    let data = slice.get_mapped_range();
    let floats: &[f32] = bytemuck::cast_slice(&data[..96]);
    let mut out = [[0.0f32; 4]; 6];
    for (i, px) in floats.chunks_exact(4).enumerate() {
        out[i] = [px[0], px[1], px[2], px[3]];
    }
    drop(data);
    readback.unmap();
    Ok(out)
}

fn luma(px: [f32; 4]) -> f32 {
    0.2126 * px[0] + 0.7152 * px[1] + 0.0722 * px[2]
}

/// Firefly cluster probe: six directions within ~2° of a 10°-tilted centre
/// (off the pole to avoid basis symmetries) — about ±1 texel of the 64² +Y
/// face. TRUE irradiance varies ~1% at this separation (even under a
/// strongly anisotropic sky, whose real gradient is what a WIDE fan
/// measures); per-texel quadrature fireflies from a concentrated source
/// (the HDRI's sun core) produce far larger neighbour-to-neighbour spread.
const FAN_PROBE_WGSL: &str = r#"
struct VsOut { @builtin(position) pos: vec4<f32> };
@vertex fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    var p = array<vec2<f32>, 3>(vec2<f32>(-1.0,-1.0), vec2<f32>(3.0,-1.0), vec2<f32>(-1.0,3.0));
    var out: VsOut;
    out.pos = vec4<f32>(p[vi], 0.0, 1.0);
    return out;
}
@group(0) @binding(0) var cube: texture_cube<f32>;
@group(0) @binding(1) var samp: sampler;
@fragment fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let i = i32(in.pos.x); // 0..5
    var dir = vec3<f32>( 0.170, 1.0,  0.000);
    if (i == 1) { dir = vec3<f32>( 0.186, 1.0,  0.000); }
    if (i == 2) { dir = vec3<f32>( 0.154, 1.0,  0.000); }
    if (i == 3) { dir = vec3<f32>( 0.170, 1.0,  0.016); }
    if (i == 4) { dir = vec3<f32>( 0.170, 1.0, -0.016); }
    if (i == 5) { dir = vec3<f32>( 0.181, 1.0,  0.011); }
    return textureSampleLevel(cube, samp, normalize(dir), 0.0);
}
"#;

/// LUT probe: pixel 0 samples (NdotV 0.9, roughness 0.05), pixel 1 samples
/// (NdotV 0.9, roughness 0.95). The split-sum scale term A falls with
/// roughness at fixed NdotV, so a roughness-flipped LUT reverses the order.
const LUT_PROBE_WGSL: &str = r#"
struct VsOut { @builtin(position) pos: vec4<f32> };
@vertex fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    var p = array<vec2<f32>, 3>(vec2<f32>(-1.0,-1.0), vec2<f32>(3.0,-1.0), vec2<f32>(-1.0,3.0));
    var out: VsOut;
    out.pos = vec4<f32>(p[vi], 0.0, 1.0);
    return out;
}
@group(0) @binding(0) var lut: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@fragment fn fs(in: VsOut) -> @location(0) vec4<f32> {
    var uv = vec2<f32>(0.9, 0.05);
    if (i32(in.pos.x) == 1) { uv = vec2<f32>(0.9, 0.95); }
    return textureSampleLevel(lut, samp, uv, 0.0);
}
"#;

fn probe_lut(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    lut: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> Result<[[f32; 4]; 2]> {
    let sm = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ibl-lut-probe-sm"),
        source: wgpu::ShaderSource::Wgsl(LUT_PROBE_WGSL.into()),
    });
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("ibl-lut-probe-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
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
        ],
    });
    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ibl-lut-probe-bg"),
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(lut),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("ibl-lut-probe-pl"),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("ibl-lut-probe-pipeline"),
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
                format: wgpu::TextureFormat::Rgba32Float,
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

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ibl-lut-probe-target"),
        size: wgpu::Extent3d {
            width: 2,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ibl-lut-probe-readback"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("ibl-lut-probe-enc"),
    });
    {
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ibl-lut-probe-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
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
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 2,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(enc.finish()));

    let slice = readback.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device
        .poll(wgpu::PollType::Wait)
        .context("device poll failed")?;
    rx.recv()
        .context("map_async channel closed")?
        .context("map_async failed")?;
    let data = slice.get_mapped_range();
    let floats: &[f32] = bytemuck::cast_slice(&data[..32]);
    let out = [
        [floats[0], floats[1], floats[2], floats[3]],
        [floats[4], floats[5], floats[6], floats[7]],
    ];
    drop(data);
    readback.unmap();
    Ok(out)
}

/// Bilinear sample of the linear-light equirect (u wraps, v clamps —
/// matching the GPU sampler's Repeat/ClampToEdge addressing).
fn sample_equirect(img: &image::Rgb32FImage, u: f32, v: f32) -> [f32; 3] {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let x = u.rem_euclid(1.0) * w - 0.5;
    let y = (v.clamp(0.0, 1.0) * h - 0.5).clamp(0.0, h - 1.0);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let xi = |xx: f32| (xx.rem_euclid(w)) as u32;
    let yi = |yy: f32| (yy.clamp(0.0, h - 1.0)) as u32;
    let p = |xx: u32, yy: u32| img.get_pixel(xx, yy).0;
    let (p00, p10) = (p(xi(x0), yi(y0)), p(xi(x0 + 1.0), yi(y0)));
    let (p01, p11) = (p(xi(x0), yi(y0 + 1.0)), p(xi(x0 + 1.0), yi(y0 + 1.0)));
    let mut out = [0.0f32; 3];
    for c in 0..3 {
        let top = p00[c] * (1.0 - fx) + p10[c] * fx;
        let bot = p01[c] * (1.0 - fx) + p11[c] * fx;
        out[c] = top * (1.0 - fy) + bot * fy;
    }
    out
}

/// CPU replica of `IRRADIANCE_WGSL`'s cosine-weighted hemisphere quadrature
/// (same 60×30 grid, same tangent-basis selection), integrating the source
/// equirect directly. Small deviations from the GPU value come only from the
/// env-cube resampling step the GPU path goes through.
fn cpu_reference_irradiance(img: &image::Rgb32FImage, n: glam::Vec3) -> [f32; 3] {
    let up = if n.z.abs() < 0.999 {
        glam::Vec3::Y
    } else {
        glam::Vec3::X
    };
    let t = up.cross(n).normalize();
    let b = n.cross(t);
    const PHI_STEPS: u32 = 60;
    const THETA_STEPS: u32 = 30;
    let delta_phi = (2.0 * std::f32::consts::PI) / PHI_STEPS as f32;
    let delta_theta = (0.5 * std::f32::consts::PI) / THETA_STEPS as f32;
    let mut acc = [0.0f32; 3];
    let mut count = 0.0f32;
    for i_phi in 0..PHI_STEPS {
        for i_theta in 0..THETA_STEPS {
            let phi = i_phi as f32 * delta_phi;
            let theta = i_theta as f32 * delta_theta;
            let tangent = glam::Vec3::new(
                theta.sin() * phi.cos(),
                theta.sin() * phi.sin(),
                theta.cos(),
            );
            let dir = (t * tangent.x + b * tangent.y + n * tangent.z).normalize();
            // dir_to_equirect_uv from EQUIRECT_TO_CUBE_WGSL
            let u = dir.z.atan2(dir.x) / (2.0 * std::f32::consts::PI) + 0.5;
            let v = dir.y.clamp(-1.0, 1.0).acos() / std::f32::consts::PI;
            let mut l = sample_equirect(img, u, v);
            // Mirror of the bake-cube radiance clamp (L.2 calibration,
            // EQUIRECT_TO_CUBE_WGSL) so the reference matches the GPU input.
            for c in l.iter_mut() {
                *c = c.min(2.0);
            }
            let wgt = theta.cos() * theta.sin();
            for c in 0..3 {
                acc[c] += l[c] * wgt;
            }
            count += 1.0;
        }
    }
    for c in acc.iter_mut() {
        *c = *c * std::f32::consts::PI / count;
    }
    acc
}

#[test]
#[ignore = "GPU + HDRI bake; run explicitly (see module docs)"]
fn ibl_irradiance_faces_are_distinct_and_sky_up() -> Result<()> {
    pollster::block_on(async {
        anyhow::ensure!(
            std::path::Path::new(HDRI_PATH).is_file(),
            "tracked HDRI missing at {HDRI_PATH} — checkout is incomplete"
        );
        let (device, queue) = acquire_device().await?;
        let mut ibl = IblManager::new(&device, IblQuality::Medium)?;
        ibl.mode = SkyMode::HdrPath {
            biome: "l2-test".to_string(),
            path: HDRI_PATH.to_string(),
        };
        let bake_started = std::time::Instant::now();
        let _cold_res = ibl
            .bake_environment(&device, &queue, IblQuality::Medium)
            .context("bake_environment failed")?;
        println!(
            "[l2-bake] decode+bake (Medium, cold cache): {:.1} ms",
            bake_started.elapsed().as_secs_f64() * 1000.0
        );
        // Warm rebake (decode cached in `hdr_cache`): isolates the pure
        // upload/encode/submit cost from the image-decode cost — the number
        // that matters for the editor's deferred-decode init design.
        let warm_started = std::time::Instant::now();
        let res = ibl
            .bake_environment(&device, &queue, IblQuality::Medium)
            .context("warm bake_environment failed")?;
        println!(
            "[l2-bake] bake only (Medium, warm cache): {:.1} ms",
            warm_started.elapsed().as_secs_f64() * 1000.0
        );

        // Diagnostic for the intensity residue (L2_OUTCOME §5, item C
        // deferred): what the RETIRED image-average scheme would have derived
        // (the shipped default is the fixed 1.5 in rebuild_ibl_bind_group)
        // vs the true linear/log averages of the HDR source.
        let retired_scheme_intensity = res
            .avg_luminance
            .map(|a| (0.35 / a).clamp(0.3, 3.0))
            .unwrap_or(1.0);
        println!(
            "[l2-int] avg_luminance={:?} -> retired image-avg scheme would give {retired_scheme_intensity:.3} (shipped: fixed 1.5)",
            res.avg_luminance
        );
        {
            let img = astraweave_asset::open_limited_image(HDRI_PATH)
                .context("decode HDRI for luminance diagnostic")?
                .to_rgb32f();
            let mut lin = 0.0f64;
            let mut log = 0.0f64;
            let n = (img.width() * img.height()) as f64;
            for p in img.pixels() {
                let l = (0.2126 * p.0[0] + 0.7152 * p.0[1] + 0.0722 * p.0[2]) as f64;
                lin += l;
                log += (l.max(1e-6)).ln();
            }
            let lin_avg = lin / n;
            let log_avg = (log / n).exp();
            println!(
                "[l2-int] true HDR: linear avg {lin_avg:.4} (-> intensity {:.3}), \
                 log avg {log_avg:.4} (-> intensity {:.3})",
                (0.35 / lin_avg).clamp(0.3, 3.0),
                (0.35 / log_avg).clamp(0.3, 3.0)
            );
        }

        const FACE_NAMES: [&str; 6] = ["+X", "-X", "+Y", "-Y", "+Z", "-Z"];

        // Diagnostic: the env cube's own face centres (ground truth for the
        // convolution input — +Y must be sky, -Y must be ground).
        let env_probes = probe_face_centres(&device, &queue, &res.env_cube, ibl.sampler())?;
        for (name, px) in FACE_NAMES.iter().zip(env_probes.iter()) {
            println!(
                "[l2-env] {name}: rgb=({:.5}, {:.5}, {:.5}) luma={:.5}",
                px[0],
                px[1],
                px[2],
                luma(*px)
            );
        }

        let probes = probe_face_centres(&device, &queue, &res.irradiance_cube, ibl.sampler())?;
        for (name, px) in FACE_NAMES.iter().zip(probes.iter()) {
            println!(
                "[l2-irr] {name}: rgb=({:.5}, {:.5}, {:.5}) luma={:.5}",
                px[0],
                px[1],
                px[2],
                luma(*px)
            );
        }

        let lumas: Vec<f32> = probes.iter().map(|p| luma(*p)).collect();
        let max = lumas.iter().cloned().fold(f32::MIN, f32::max);
        let min = lumas.iter().cloned().fold(f32::MAX, f32::min);
        let rel_spread = (max - min) / max.max(1e-6);
        println!(
            "[l2-irr] face-centre luma spread: {:.2}% (max {max:.5} / min {min:.5})",
            100.0 * rel_spread
        );

        // 1. The six faces must not be (near-)identical. Pre-fix, the
        //    convolution was face-blind and every probe returned the same
        //    value (spread == 0).
        anyhow::ensure!(
            rel_spread > 0.05,
            "irradiance face centres are near-identical (spread {:.4}%) — \
             the face-blind convolution defect (L2_PHASE0_STOP.md §2)",
            100.0 * rel_spread
        );

        // 2. For a daylight sky HDRI, up-facing irradiance must exceed
        //    down-facing. CPU reference: +Y 0.167 vs -Y 0.081 — pre-fix the
        //    v-flipped env inverted this (GPU read 0.116 vs 0.128).
        anyhow::ensure!(
            lumas[2] > lumas[3],
            "+Y irradiance ({:.5}) must exceed -Y ({:.5}) for a sky HDRI",
            lumas[2],
            lumas[3]
        );

        // 3 + 4. CPU reference agreement: same quadrature, same basis,
        //    integrating the source equirect directly.
        let img = astraweave_asset::open_limited_image(HDRI_PATH)
            .context("decode HDRI for CPU reference")?
            .to_rgb32f();
        // For ±Y the exact pole is a tangent-basis singularity (up ∥ N) that
        // the shader never evaluates — a 64² face's nearest texel normal is
        // offset by half a texel (a = 1/64). Evaluate the CPU reference at
        // that representative texel normal instead of the exact pole.
        const POLE_EPS: f32 = 1.0 / 64.0;
        let axes: [glam::Vec3; 6] = [
            glam::Vec3::X,
            glam::Vec3::NEG_X,
            glam::Vec3::new(POLE_EPS, 1.0, POLE_EPS).normalize(),
            glam::Vec3::new(POLE_EPS, -1.0, POLE_EPS).normalize(),
            glam::Vec3::Z,
            glam::Vec3::NEG_Z,
        ];
        let cpu_lumas: Vec<f32> = axes
            .iter()
            .zip(FACE_NAMES.iter())
            .map(|(axis, name)| {
                let rgb = cpu_reference_irradiance(&img, *axis);
                let l = luma([rgb[0], rgb[1], rgb[2], 1.0]);
                println!(
                    "[l2-cpu] {name}: rgb=({:.5}, {:.5}, {:.5}) luma={:.5}",
                    rgb[0], rgb[1], rgb[2], l
                );
                l
            })
            .collect();

        for i in 0..6 {
            let ratio = lumas[i] / cpu_lumas[i].max(1e-6);
            println!(
                "[l2-ref] {}: gpu {:.5} / cpu {:.5} = {:.3}",
                FACE_NAMES[i], lumas[i], cpu_lumas[i], ratio
            );
            anyhow::ensure!(
                (0.5..=2.0).contains(&ratio),
                "GPU irradiance for face {} ({:.5}) diverges from the CPU \
                 reference ({:.5}) by {:.2}x — convolution defect",
                FACE_NAMES[i],
                lumas[i],
                cpu_lumas[i],
                ratio
            );
        }
        for i in 0..6 {
            for j in (i + 1)..6 {
                let (hi, lo) = if cpu_lumas[i] > cpu_lumas[j] {
                    (i, j)
                } else {
                    (j, i)
                };
                if cpu_lumas[hi] > cpu_lumas[lo] * 1.25 {
                    anyhow::ensure!(
                        lumas[hi] > lumas[lo],
                        "CPU reference orders {} ({:.5}) above {} ({:.5}) by >25%, \
                         but the GPU disagrees ({:.5} vs {:.5})",
                        FACE_NAMES[hi],
                        cpu_lumas[hi],
                        FACE_NAMES[lo],
                        cpu_lumas[lo],
                        lumas[hi],
                        lumas[lo]
                    );
                }
            }
        }

        // 6. Firefly guard (hard assertion since the L.2 calibration
        //    ratification): a ~2° neighbour cluster near +Y. True irradiance
        //    varies ~1% at this separation; per-texel sun-core quadrature
        //    aliasing (the L.2 terrain glitter — 35.1% over a wide fan
        //    pre-clamp) blows the neighbour spread up. The bake-cube radiance
        //    clamp (EQUIRECT_TO_CUBE_WGSL) bounds a single quadrature
        //    sample's contribution and collapses this.
        let fan = probe_six(
            &device,
            &queue,
            &res.irradiance_cube,
            ibl.sampler(),
            FAN_PROBE_WGSL,
        )?;
        let fan_lumas: Vec<f32> = fan.iter().map(|p| luma(*p)).collect();
        let fmax = fan_lumas.iter().cloned().fold(f32::MIN, f32::max);
        let fmin = fan_lumas.iter().cloned().fold(f32::MAX, f32::min);
        let fan_spread = (fmax - fmin) / fmax.max(1e-6);
        println!(
            "[l2-fan] +Y neighbour-cluster lumas: {:?} — spread {:.1}% of max",
            fan_lumas
                .iter()
                .map(|l| (l * 1e4).round() / 1e4)
                .collect::<Vec<_>>(),
            100.0 * fan_spread
        );
        anyhow::ensure!(
            fan_spread < 0.05,
            "irradiance neighbour-cluster spread {:.1}% exceeds 5% — sun-core \
             quadrature fireflies (the L.2 terrain glitter; check the \
             EQUIRECT_TO_CUBE_WGSL bake-cube radiance clamp)",
            100.0 * fan_spread
        );

        // 5. The BRDF LUT must not be roughness-flipped: A(NdotV .9, r .05)
        //    must exceed A(NdotV .9, r .95). Pre-fix the v-flipped write
        //    reversed this ordering.
        let lut = probe_lut(&device, &queue, &res.brdf_lut, ibl.sampler())?;
        println!(
            "[l2-lut] A(r=0.05)={:.4} B={:.4} | A(r=0.95)={:.4} B={:.4}",
            lut[0][0], lut[0][1], lut[1][0], lut[1][1]
        );
        anyhow::ensure!(
            lut[0][0] > lut[1][0],
            "BRDF LUT scale term A must fall with roughness at fixed NdotV \
             (got A(0.05)={:.4} <= A(0.95)={:.4}) — roughness-flipped LUT",
            lut[0][0],
            lut[1][0]
        );
        Ok(())
    })
}
