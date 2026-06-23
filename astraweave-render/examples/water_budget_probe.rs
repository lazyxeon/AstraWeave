//! W-series W.2a — Water surface render-budget probe (measurement harness).
//!
//! Measures the *real* GPU cost of the water surface pass on the local adapter
//! using wgpu timestamp queries. Two probes:
//!
//!   A. Isolated water pass — renders ONLY the water into an offscreen
//!      Rgba16Float + Depth32Float target (matching the production HDR path),
//!      wrapped in a GPU timestamp query. Reports the pure water-pass cost at a
//!      "near" camera and a "worst-case horizon" camera.
//!
//!   B. Full-frame context — drives `Renderer::new_headless` (the real engine
//!      render path) with and without a WaterRenderer attached, reading the
//!      integrated `GpuProfiler` per-pass map. The `main_pass` delta is the
//!      marginal water cost inside a real frame; the total is the frame floor.
//!
//! This is a Step-0 budget instrument, not production code. It prints the
//! selected adapter (name/backend/device-type) so the numbers can be trusted as
//! a real measurement on this hardware — NOT a software-rasterizer proxy.
//!
//! Run: `cargo run -p astraweave-render --example water_budget_probe --release`

use astraweave_render::{GpuProfiler, Renderer, WaterRenderer, WeaveInstance, WeaveKind};
use glam::{Mat4, Vec2, Vec3};

const W: u32 = 1920;
const H: u32 = 1080;
const COLOR_FMT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const DEPTH_FMT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

const WARMUP: usize = 60;
const FRAMES: usize = 300;

/// One camera configuration for the isolated probe.
struct CamCfg {
    name: &'static str,
    eye: Vec3,
    target: Vec3,
}

fn view_proj(eye: Vec3, target: Vec3) -> (Mat4, Vec3) {
    let aspect = W as f32 / H as f32;
    let proj = Mat4::perspective_rh(60.0f32.to_radians(), aspect, 0.1, 4000.0);
    let view = Mat4::look_at_rh(eye, target, Vec3::Y);
    (proj * view, eye)
}

/// Summary statistics over a set of per-frame millisecond samples.
fn stats(mut samples: Vec<f32>) -> (f32, f32, f32, f32, f32) {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = samples.len().max(1);
    let min = samples[0];
    let max = samples[n - 1];
    let mean = samples.iter().sum::<f32>() / n as f32;
    let median = samples[n / 2];
    let p95 = samples[((n as f32 * 0.95) as usize).min(n - 1)];
    (min, median, mean, p95, max)
}

fn print_stats(label: &str, samples: Vec<f32>) {
    if samples.is_empty() {
        println!("  {label:<28} (no samples)");
        return;
    }
    let (min, median, mean, p95, max) = stats(samples);
    println!(
        "  {label:<28} min={min:.4}ms  median={median:.4}ms  mean={mean:.4}ms  p95={p95:.4}ms  max={max:.4}ms"
    );
}

/// Probe A: isolated water pass cost via a dedicated timestamp profiler.
fn probe_isolated(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    has_timestamps: bool,
) {
    println!("\n=== PROBE A: isolated water pass (chunked LOD surface) ===");

    let mut water = WaterRenderer::new(device, COLOR_FMT, DEPTH_FMT);

    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("probe_color"),
        size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: COLOR_FMT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("probe_depth"),
        size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FMT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

    let cams = [
        CamCfg { name: "near (gameplay cam ~12u above)", eye: Vec3::new(0.0, 14.0, 70.0), target: Vec3::new(0.0, 2.0, 0.0) },
        CamCfg { name: "horizon (eye at water level)", eye: Vec3::new(0.0, 4.0, 245.0), target: Vec3::new(0.0, 2.0, -120.0) },
    ];

    let mut profiler = if has_timestamps {
        Some(GpuProfiler::new(device, queue))
    } else {
        None
    };

    for cam in &cams {
        let (vp, eye) = view_proj(cam.eye, cam.target);
        let mut samples: Vec<f32> = Vec::with_capacity(FRAMES);

        for frame in 0..(WARMUP + FRAMES) {
            let time = frame as f32 * (1.0 / 60.0);
            water.update(queue, vp, eye, time);

            if let Some(ref mut p) = profiler {
                p.begin_frame();
            }
            let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("probe_enc"),
            });
            {
                let ts = profiler
                    .as_mut()
                    .and_then(|p| p.render_pass_timestamps("water"));
                let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("probe_water_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &color_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.05, g: 0.1, b: 0.2, a: 1.0 }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: ts,
                    occlusion_query_set: None,
                });
                water.render(&mut rp);
            }
            if let Some(ref p) = profiler {
                p.end_frame(&mut enc);
            }
            queue.submit(Some(enc.finish()));
            let _ = device.poll(wgpu::PollType::Wait);

            if let Some(ref mut p) = profiler {
                p.request_readback();
                let _ = device.poll(wgpu::PollType::Wait);
                p.poll_readback(device);
                if frame >= WARMUP {
                    if let Some(ms) = p.results_map().get("water") {
                        samples.push(*ms);
                    }
                }
            }
        }
        print_stats(cam.name, samples);
    }

    // ── Render-correctness check ─────────────────────────────────────────────
    // Timing alone can't tell a drawn surface from one silently back-face-culled
    // (vertex work clocks either way). Render the near view over a BLACK clear,
    // read it back, and count lit pixels: ~0 would mean the surface (or its
    // winding) is wrong; a substantial fraction proves it actually rasterizes.
    let (vp, eye) = view_proj(cams[0].eye, cams[0].target);
    water.update(queue, vp, eye, 1.0);
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("probe_verify_enc"),
    });
    {
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("probe_verify_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        water.render(&mut rp);
    }
    // Copy color → buffer (Rgba16Float = 8 bytes/pixel).
    let bytes_per_pixel = 8u32;
    let unpadded = W * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded = unpadded.div_ceil(align) * align;
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("probe_verify_readback"),
        size: (padded * H) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &color,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
    );
    queue.submit(Some(enc.finish()));
    let _ = device.poll(wgpu::PollType::Wait);

    let slice = buf.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    let _ = device.poll(wgpu::PollType::Wait);
    let data = slice.get_mapped_range();

    let mut lit = 0u64;
    for y in 0..H as usize {
        let row = &data[y * padded as usize..y * padded as usize + unpadded as usize];
        for px in row.chunks_exact(8) {
            let r = half::f16::from_le_bytes([px[0], px[1]]).to_f32();
            let g = half::f16::from_le_bytes([px[2], px[3]]).to_f32();
            let b = half::f16::from_le_bytes([px[4], px[5]]).to_f32();
            if r + g + b > 0.001 {
                lit += 1;
            }
        }
    }
    drop(data);
    buf.unmap();
    let total = (W as u64) * (H as u64);
    let frac = lit as f64 / total as f64 * 100.0;
    println!(
        "  render-check (near, black clear): {lit}/{total} lit pixels = {frac:.1}% — {}",
        if frac > 5.0 { "surface RASTERIZES (not culled away)" } else { "!! suspiciously empty — investigate winding/culling" }
    );
}

/// Probe B: full-frame context via the real headless renderer.
///
/// The integrated `GpuProfiler` readback is pipelined with non-blocking polls
/// internal to `render()`, which an external driver loop cannot reliably flush.
/// So we measure submit-to-GPU-completion **wall-clock** with a blocking poll —
/// a robust real measurement of the per-frame cost on this GPU. The scene is the
/// headless DEFAULT (near-empty), so this is a frame *floor*, not a representative
/// Veilweaver scene (see the printed caveat).
async fn probe_full_frame(_has_timestamps: bool) {
    println!("\n=== PROBE B: full headless frame (Renderer::new_headless, wall-clock) ===");

    // ---- without water ----
    let mut r = Renderer::new_headless(W, H)
        .await
        .expect("headless renderer");
    let no_water = drive_frames(&mut r, "no-water");

    // ---- with water ----
    let water = WaterRenderer::new(r.device(), r.config().format, DEPTH_FMT);
    r.set_water_renderer(water);
    let with_water = drive_frames(&mut r, "with-water");

    println!(
        "\n  CAVEAT: Renderer::new_headless renders the DEFAULT (near-empty) scene — clear + sky +\n  fixed passes + post only. This is a frame *floor*, NOT a representative Veilweaver scene\n  (island terrain + vegetation + clouds), which is a windowed winit app and cannot be loaded\n  or driven headless in this environment. Real-scene frame time is HIGHER => real headroom is\n  SMALLER than this floor implies. Wall-clock includes CPU encode + submit + GPU completion."
    );
    if let (Some(nw), Some(ww)) = (no_water, with_water) {
        println!("  marginal water cost (with-water minus no-water, wall-clock): {:.4}ms", (ww - nw).max(0.0));
    }
}

/// Render N frames with a blocking poll and return the median wall-clock ms.
fn drive_frames(r: &mut Renderer, tag: &str) -> Option<f32> {
    use std::time::Instant;
    let mut samples: Vec<f32> = Vec::with_capacity(WARMUP + FRAMES);
    for frame in 0..(WARMUP + FRAMES) {
        let t0 = Instant::now();
        if r.render().is_err() {
            println!("  [{tag}] render() failed");
            return None;
        }
        let _ = r.device().poll(wgpu::PollType::Wait);
        let dt = t0.elapsed().as_secs_f32() * 1000.0;
        if frame >= WARMUP {
            samples.push(dt);
        }
    }
    print_stats(&format!("[{tag}] frame wall-clock"), samples.clone());
    if samples.is_empty() {
        return None;
    }
    let (_, median, _, _, _) = stats(samples);
    Some(median)
}

// Opaque sloped-ground scene shader for the refraction probe: a big quad whose
// Y slopes with Z so it crosses the water level (creating a shoreline for the
// depth-delta foam) and recedes below it (deep water for refraction). Writes
// depth so the water pass can depth-test + sample it.
const GROUND_WGSL: &str = r#"
struct VP { mvp: mat4x4<f32> };
@group(0) @binding(0) var<uniform> u: VP;
struct VO { @builtin(position) pos: vec4<f32>, @location(0) world: vec3<f32> };
@vertex fn vs(@location(0) p: vec3<f32>) -> VO {
    var o: VO; o.pos = u.mvp * vec4<f32>(p, 1.0); o.world = p; return o;
}
@fragment fn fs(i: VO) -> @location(0) vec4<f32> {
    // Distinct red/green checker so refraction tint is detectable on readback.
    let c = step(0.5, fract(i.world.x * 0.05)) + step(0.5, fract(i.world.z * 0.05));
    let chk = abs(c - 1.0);
    return vec4<f32>(0.8 * chk + 0.1, 0.5 * (1.0 - chk) + 0.1, 0.08, 1.0);
}
"#;

/// Probe C: real refraction — render an opaque sloped ground, snapshot it, then
/// run the split water pass sampling that snapshot + depth. Reports the copy and
/// water-pass costs and confirms refraction actually tints the water.
fn probe_refraction(device: &wgpu::Device, queue: &wgpu::Queue, has_timestamps: bool) {
    use wgpu::util::DeviceExt;
    println!("\n=== PROBE C: refraction + depth-foam (opaque ground behind water) ===");

    let mut water = WaterRenderer::new(device, COLOR_FMT, DEPTH_FMT);

    let mk = |label, fmt, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: fmt,
            usage,
            view_formats: &[],
        })
    };
    let hdr = mk("probe_hdr", COLOR_FMT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST);
    let hdr_view = hdr.create_view(&Default::default());
    let depth = mk("probe_depth", DEPTH_FMT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING);
    let depth_view = depth.create_view(&Default::default());
    let scene_color = mk("probe_scene_color", COLOR_FMT,
        wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING);
    let scene_color_view = scene_color.create_view(&Default::default());

    // Sloped ground quad: y = -z * 0.04 → crosses water level (2.0) near z=-50.
    let g = |x: f32, z: f32| [x, -z * 0.04, z];
    let verts: [[f32; 3]; 4] = [g(-400.0, -400.0), g(400.0, -400.0), g(-400.0, 400.0), g(400.0, 400.0)];
    let idx: [u32; 6] = [0, 2, 1, 1, 2, 3];
    let gvb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ground_vb"), contents: bytemuck::cast_slice(&verts), usage: wgpu::BufferUsages::VERTEX });
    let gib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ground_ib"), contents: bytemuck::cast_slice(&idx), usage: wgpu::BufferUsages::INDEX });
    let gubo = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ground_ubo"), size: 64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let gshader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ground"), source: wgpu::ShaderSource::Wgsl(GROUND_WGSL.into()) });
    let gbgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("ground_bgl"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0, visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }] });
    let gbg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ground_bg"), layout: &gbgl,
        entries: &[wgpu::BindGroupEntry { binding: 0, resource: gubo.as_entire_binding() }] });
    let gpl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("ground_pl"), bind_group_layouts: &[&gbgl], push_constant_ranges: &[] });
    let gpipe = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("ground_pipe"), layout: Some(&gpl),
        vertex: wgpu::VertexState { module: &gshader, entry_point: Some("vs"),
            buffers: &[wgpu::VertexBufferLayout { array_stride: 12, step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 }] }],
            compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &gshader, entry_point: Some("fs"),
            targets: &[Some(wgpu::ColorTargetState { format: COLOR_FMT, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            compilation_options: Default::default() }),
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH_FMT, depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual, stencil: Default::default(), bias: Default::default() }),
        multisample: Default::default(), multiview: None, cache: None });

    let mut profiler = if has_timestamps { Some(GpuProfiler::new(device, queue)) } else { None };
    let copy_ts = device.features().contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS);
    // Dedicated 2-query set for the copy line item.
    let copy_qset = device.create_query_set(&wgpu::QuerySetDescriptor {
        label: Some("copy_qs"), ty: wgpu::QueryType::Timestamp, count: 2 });
    let copy_resolve = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("copy_resolve"), size: 16, usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
    let copy_read = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("copy_read"), size: 16, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let ts_period = queue.get_timestamp_period();

    let cams = [
        CamCfg { name: "near", eye: Vec3::new(0.0, 14.0, 70.0), target: Vec3::new(0.0, 2.0, 0.0) },
        CamCfg { name: "horizon", eye: Vec3::new(0.0, 4.0, 245.0), target: Vec3::new(0.0, 2.0, -120.0) },
    ];

    for cam in &cams {
        let (vp, eye) = view_proj(cam.eye, cam.target);
        queue.write_buffer(&gubo, 0, bytemuck::cast_slice(&vp.to_cols_array()));
        let mut water_ms: Vec<f32> = Vec::with_capacity(FRAMES);
        let mut copy_ms: Vec<f32> = Vec::with_capacity(FRAMES);

        for frame in 0..(WARMUP + FRAMES) {
            let time = frame as f32 * (1.0 / 60.0);
            water.update(queue, vp, eye, time);
            water.prepare_scene(device, queue, &scene_color_view, &depth_view, [W as f32, H as f32], 0);
            if let Some(ref mut p) = profiler { p.begin_frame(); }
            let mut enc = device.create_command_encoder(&Default::default());
            // 1. Opaque ground into hdr + depth.
            {
                let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("ground_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &hdr_view, resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.5, g: 0.7, b: 0.95, a: 1.0 }), store: wgpu::StoreOp::Store } })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: &depth_view,
                        depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }), stencil_ops: None }),
                    timestamp_writes: None, occlusion_query_set: None });
                rp.set_pipeline(&gpipe); rp.set_bind_group(0, &gbg, &[]);
                rp.set_vertex_buffer(0, gvb.slice(..)); rp.set_index_buffer(gib.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..6, 0, 0..1);
            }
            // 2. Snapshot hdr → scene_color (the copy line item).
            if copy_ts { enc.write_timestamp(&copy_qset, 0); }
            enc.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo { texture: &hdr, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                wgpu::TexelCopyTextureInfo { texture: &scene_color, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 });
            if copy_ts {
                enc.write_timestamp(&copy_qset, 1);
                enc.resolve_query_set(&copy_qset, 0..2, &copy_resolve, 0);
                enc.copy_buffer_to_buffer(&copy_resolve, 0, &copy_read, 0, 16);
            }
            // 3. Water pass (read-only depth, samples scene_color + depth).
            {
                let ts = profiler.as_mut().and_then(|p| p.render_pass_timestamps("water"));
                let mut wp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("water_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &hdr_view, resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: &depth_view,
                        depth_ops: None, stencil_ops: None }),
                    timestamp_writes: ts, occlusion_query_set: None });
                water.render(&mut wp);
            }
            if let Some(ref p) = profiler { p.end_frame(&mut enc); }
            queue.submit(Some(enc.finish()));
            let _ = device.poll(wgpu::PollType::Wait);
            // read copy timestamps
            if copy_ts {
                let slice = copy_read.slice(..);
                slice.map_async(wgpu::MapMode::Read, |_| {});
                let _ = device.poll(wgpu::PollType::Wait);
                let data = slice.get_mapped_range();
                let t: &[u64] = bytemuck::cast_slice(&data);
                let dt = (t[1].wrapping_sub(t[0])) as f64 * ts_period as f64 / 1_000_000.0;
                drop(data); copy_read.unmap();
                if frame >= WARMUP { copy_ms.push(dt as f32); }
            }
            if let Some(ref mut p) = profiler {
                p.request_readback(); let _ = device.poll(wgpu::PollType::Wait); p.poll_readback(device);
                if frame >= WARMUP { if let Some(ms) = p.results_map().get("water") { water_ms.push(*ms); } }
            }
        }
        print_stats(&format!("[{}] water pass (refraction)", cam.name), water_ms);
        print_stats(&format!("[{}] scene-color copy", cam.name), copy_ms);
    }

    // Render-correctness: confirm water rasterizes AND refraction tints it with the
    // ground's red/green checker (vs the flat blue water body) — proves the scene-
    // color tap is live, not a dummy/black sample.
    {
        let (vp, eye) = view_proj(cams[0].eye, cams[0].target);
        queue.write_buffer(&gubo, 0, bytemuck::cast_slice(&vp.to_cols_array()));
        water.update(queue, vp, eye, 1.0);
        water.prepare_scene(device, queue, &scene_color_view, &depth_view, [W as f32, H as f32], 0);
        let mut enc = device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("verify_ground"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &hdr_view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.5, g: 0.7, b: 0.95, a: 1.0 }), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: &depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }), stencil_ops: None }),
                timestamp_writes: None, occlusion_query_set: None });
            rp.set_pipeline(&gpipe); rp.set_bind_group(0, &gbg, &[]);
            rp.set_vertex_buffer(0, gvb.slice(..)); rp.set_index_buffer(gib.slice(..), wgpu::IndexFormat::Uint32);
            rp.draw_indexed(0..6, 0, 0..1);
        }
        enc.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo { texture: &hdr, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyTextureInfo { texture: &scene_color, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 });
        {
            let mut wp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("verify_water"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &hdr_view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: &depth_view, depth_ops: None, stencil_ops: None }),
                timestamp_writes: None, occlusion_query_set: None });
            water.render(&mut wp);
        }
        // readback hdr (bottom half = under-water region where ground is refracted)
        let bpp = 8u32; let unpadded = W * bpp; let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded = unpadded.div_ceil(align) * align;
        let buf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("verify_rb"),
            size: (padded * H) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &hdr, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(H) } },
            wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 });
        queue.submit(Some(enc.finish()));
        let _ = device.poll(wgpu::PollType::Wait);
        let slice = buf.slice(..); slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = device.poll(wgpu::PollType::Wait);
        let data = slice.get_mapped_range();
        // Diagnostic: average colour + warm fraction across vertical bands, so a
        // 0% can be told apart (blue=refraction off vs warm=working vs sky).
        let mut warm = 0u64; let mut sampled = 0u64; let mut foam = 0u64;
        for (lo, hi, label) in [(10u32, 40u32, "upper"), (40, 60, "mid"), (60, 90, "lower")] {
            let (mut sr, mut sg, mut sb, mut n) = (0f64, 0f64, 0f64, 0u64);
            let mut w = 0u64; let mut fm = 0u64;
            for y in (H * lo / 100)..(H * hi / 100) {
                let row = &data[y as usize * padded as usize..];
                for x in (W * 25 / 100)..(W * 75 / 100) {
                    let o = x as usize * 8;
                    let r = half::f16::from_le_bytes([row[o], row[o + 1]]).to_f32();
                    let g = half::f16::from_le_bytes([row[o + 2], row[o + 3]]).to_f32();
                    let b = half::f16::from_le_bytes([row[o + 4], row[o + 5]]).to_f32();
                    sr += r as f64; sg += g as f64; sb += b as f64; n += 1;
                    if r > b * 1.05 { w += 1; warm += 1; }
                    // Near-white = foam (wave-crest or depth-delta shoreline).
                    if r > 0.7 && g > 0.7 && b > 0.7 { fm += 1; foam += 1; }
                    sampled += 1;
                }
            }
            let nn = n.max(1) as f64;
            println!("    band {label:<6} avg rgb=({:.3},{:.3},{:.3})  warm={:.0}%  foam={:.1}%",
                sr / nn, sg / nn, sb / nn, w as f64 / nn * 100.0, fm as f64 / nn * 100.0);
        }
        let foam_pct = foam as f64 / sampled.max(1) as f64 * 100.0;
        println!("  foam-check: {:.2}% near-white foam pixels in water region — {}", foam_pct,
            if foam > 0 { "FOAM RENDERS (wave-crest + depth-delta shoreline)" } else { "no foam detected" });
        drop(data); buf.unmap();
        let pct = warm as f64 / sampled.max(1) as f64 * 100.0;
        println!(
            "  refraction-check: {:.1}% warm overall — {}",
            pct,
            if pct > 5.0 { "REFRACTION LIVE (scene-color sampled, not dummy/flat)" } else { "!! no refraction tint — investigate scene-color binding" }
        );
    }
}

/// Measure the median isolated-water-pass cost (ms) for the weave instances
/// currently set on `water`, at the given camera. Mirrors PROBE A's loop.
fn measure_water_pass(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    water: &mut WaterRenderer,
    profiler: &mut Option<GpuProfiler>,
    color_view: &wgpu::TextureView,
    depth_view: &wgpu::TextureView,
    vp: Mat4,
    eye: Vec3,
) -> Option<f32> {
    let mut samples: Vec<f32> = Vec::with_capacity(FRAMES);
    for frame in 0..(WARMUP + FRAMES) {
        let time = frame as f32 * (1.0 / 60.0);
        water.update(queue, vp, eye, time);
        if let Some(p) = profiler.as_mut() {
            p.begin_frame();
        }
        let mut enc = device.create_command_encoder(&Default::default());
        {
            let ts = profiler.as_mut().and_then(|p| p.render_pass_timestamps("water"));
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("weave_water_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.05, g: 0.1, b: 0.2, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: ts,
                occlusion_query_set: None,
            });
            water.render(&mut rp);
        }
        if let Some(p) = profiler.as_ref() {
            p.end_frame(&mut enc);
        }
        queue.submit(Some(enc.finish()));
        let _ = device.poll(wgpu::PollType::Wait);
        if let Some(p) = profiler.as_mut() {
            p.request_readback();
            let _ = device.poll(wgpu::PollType::Wait);
            p.poll_readback(device);
            if frame >= WARMUP {
                if let Some(ms) = p.results_map().get("water") {
                    samples.push(*ms);
                }
            }
        }
    }
    if samples.is_empty() {
        return None;
    }
    let (_, median, _, _, _) = stats(samples);
    Some(median)
}

/// Probe D: weave-deformation cost (W.2c). Measures the isolated water pass at
/// 0 / 1 / 8 active instances → per-instance cost + 8-instance total delta vs the
/// W.2b.2 baseline (~0.18–0.20 ms) and the 2.0 ms ceiling. Then confirms the
/// deformation actually changes rendered pixels (a raise vs no-weave diff).
fn probe_weave(device: &wgpu::Device, queue: &wgpu::Queue, has_timestamps: bool) {
    println!("\n=== PROBE D: weave-deformation cost (W.2c, ceiling 8) ===");
    if !has_timestamps {
        println!("  (no timestamp support — skipping deformation timing)");
        return;
    }

    let mut water = WaterRenderer::new(device, COLOR_FMT, DEPTH_FMT);
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("weave_color"),
        size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: COLOR_FMT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let color_view = color.create_view(&Default::default());
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("weave_depth"),
        size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FMT, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });
    let depth_view = depth.create_view(&Default::default());
    let mut profiler = Some(GpuProfiler::new(device, queue));

    // 8 test instances spread across the near chunk grid (mix of all three kinds).
    let mk = |kind, x: f32, z: f32| WeaveInstance {
        kind, position: Vec2::new(x, z), radius: 25.0, orientation: 0.0, intensity: 0.8, phase: 0.0,
    };
    let weaves = [
        mk(WeaveKind::Part, 0.0, 0.0),
        mk(WeaveKind::Raise, 35.0, 0.0),
        mk(WeaveKind::Freeze, -35.0, 0.0),
        mk(WeaveKind::Part, 0.0, 35.0),
        mk(WeaveKind::Raise, 0.0, -35.0),
        mk(WeaveKind::Part, 35.0, 35.0),
        mk(WeaveKind::Raise, -35.0, 35.0),
        mk(WeaveKind::Freeze, 35.0, -35.0),
    ];

    let cams = [
        CamCfg { name: "near", eye: Vec3::new(0.0, 14.0, 70.0), target: Vec3::new(0.0, 2.0, 0.0) },
        CamCfg { name: "horizon", eye: Vec3::new(0.0, 4.0, 245.0), target: Vec3::new(0.0, 2.0, -120.0) },
    ];

    for cam in &cams {
        let (vp, eye) = view_proj(cam.eye, cam.target);
        water.clear_weave_instances();
        let t0 = measure_water_pass(device, queue, &mut water, &mut profiler, &color_view, &depth_view, vp, eye);
        water.set_weave_instances(&weaves[..1]);
        let t1 = measure_water_pass(device, queue, &mut water, &mut profiler, &color_view, &depth_view, vp, eye);
        water.set_weave_instances(&weaves[..8]);
        let t8 = measure_water_pass(device, queue, &mut water, &mut profiler, &color_view, &depth_view, vp, eye);
        if let (Some(a), Some(b), Some(c)) = (t0, t1, t8) {
            let per = (c - a) / 8.0;
            println!(
                "  [{:<7}] water pass  0-inst={a:.4}ms  1-inst={b:.4}ms  8-inst={c:.4}ms  |  per-instance≈{per:.5}ms  8-inst Δ={:.4}ms",
                cam.name, c - a
            );
        }
    }

    // ── Deformation render-check ─────────────────────────────────────────────
    // Render the near view with NO weaves and with a strong raise at the look-at
    // point, over an identical black clear at the same time; count differing pixels.
    // A nonzero fraction proves the deformation actually moved the surface (vs a
    // shader that silently ignores the instances).
    let (vp, eye) = view_proj(cams[0].eye, cams[0].target);
    let read_surface = |water: &mut WaterRenderer| -> Vec<f32> {
        water.update(queue, vp, eye, 1.0);
        let mut enc = device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("weave_verify"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view, depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }), stencil_ops: None }),
                timestamp_writes: None, occlusion_query_set: None });
            water.render(&mut rp);
        }
        let bpp = 8u32;
        let unpadded = W * bpp;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded = unpadded.div_ceil(align) * align;
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("weave_rb"), size: (padded * H) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &color, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(H) } },
            wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 });
        queue.submit(Some(enc.finish()));
        let _ = device.poll(wgpu::PollType::Wait);
        let slice = buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = device.poll(wgpu::PollType::Wait);
        let data = slice.get_mapped_range();
        let mut out = Vec::with_capacity((W * H * 3) as usize);
        for y in 0..H as usize {
            let row = &data[y * padded as usize..];
            for x in 0..W as usize {
                let o = x * 8;
                out.push(half::f16::from_le_bytes([row[o], row[o + 1]]).to_f32());
                out.push(half::f16::from_le_bytes([row[o + 2], row[o + 3]]).to_f32());
                out.push(half::f16::from_le_bytes([row[o + 4], row[o + 5]]).to_f32());
            }
        }
        drop(data);
        buf.unmap();
        out
    };

    water.clear_weave_instances();
    let base = read_surface(&mut water);
    // A strong raise filling the near view so the displacement is unmissable.
    water.set_weave_instances(&[WeaveInstance {
        kind: WeaveKind::Raise, position: Vec2::new(0.0, 0.0), radius: 60.0, orientation: 0.0, intensity: 1.0, phase: 0.0,
    }]);
    let raised = read_surface(&mut water);
    let mut diff = 0u64;
    let n = base.len().min(raised.len()) / 3;
    for i in 0..n {
        let d = (base[i * 3] - raised[i * 3]).abs()
            + (base[i * 3 + 1] - raised[i * 3 + 1]).abs()
            + (base[i * 3 + 2] - raised[i * 3 + 2]).abs();
        if d > 0.02 {
            diff += 1;
        }
    }
    let pct = diff as f64 / n.max(1) as f64 * 100.0;
    println!(
        "  deformation render-check (raise vs no-weave): {diff}/{n} pixels changed = {pct:.1}% — {}",
        if pct > 1.0 { "DEFORMATION RENDERS (instances move the surface)" } else { "!! no change — instances ignored?" }
    );
    water.clear_weave_instances();
}

fn main() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("no adapter");

        let info = adapter.get_info();
        println!("===========================================================");
        println!(" W.2a WATER RENDER-BUDGET PROBE");
        println!("===========================================================");
        println!("Adapter : {}", info.name);
        println!("Backend : {:?}", info.backend);
        println!("Type    : {:?}", info.device_type);
        println!("Driver  : {} {}", info.driver, info.driver_info);
        println!("Target  : {}x{}  color={:?} depth={:?}", W, H, COLOR_FMT, DEPTH_FMT);
        let has_timestamps = adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY);
        println!("TIMESTAMP_QUERY supported: {has_timestamps}");
        if matches!(info.device_type, wgpu::DeviceType::Cpu) {
            println!("!! WARNING: adapter is a CPU/software rasterizer — numbers are NOT min-spec GPU.");
        }

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("probe_device"),
                required_features: {
                    let mut f = wgpu::Features::empty();
                    if has_timestamps {
                        f |= wgpu::Features::TIMESTAMP_QUERY;
                    }
                    // Needed for encoder.write_timestamp around the scene-color copy.
                    if adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS) {
                        f |= wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
                    }
                    f
                },
                required_limits: wgpu::Limits {
                    max_bind_groups: 8,
                    ..wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: Default::default(),
            })
            .await
            .expect("device");

        probe_isolated(&device, &queue, has_timestamps);
        probe_refraction(&device, &queue, has_timestamps);
        probe_weave(&device, &queue, has_timestamps);
        probe_full_frame(has_timestamps).await;

        println!("\n=== done ===");
    });
}
