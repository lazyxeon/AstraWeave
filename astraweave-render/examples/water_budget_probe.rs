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

use astraweave_render::{GpuProfiler, Renderer, WaterRenderer};
use glam::{Mat4, Vec3};

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
                required_features: if has_timestamps {
                    wgpu::Features::TIMESTAMP_QUERY
                } else {
                    wgpu::Features::empty()
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
        probe_full_frame(has_timestamps).await;

        println!("\n=== done ===");
    });
}
