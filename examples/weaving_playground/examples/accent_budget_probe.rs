//! F.4.3 — Combined-frame water budget probe (surface + weave-impact accents).
//!
//! The headline F.4.3 deliverable: a REAL min-spec measurement of the water
//! surface pass AND the accent composite rendering together in one frame, the
//! validation the whole F.4 arc estimated toward (surface ~0.2 ms + accents
//! ~0.1–0.2 ms ≈ ~0.3–0.4 ms vs the ≤0.5 ms accent target and 2.0 ms total
//! water budget).
//!
//! It lives in `weaving_playground` (which legitimately depends on BOTH
//! `astraweave-render` and `astraweave-fluids`) so the measurement does NOT add a
//! render↔fluids Cargo edge — invariant #18 stays literally true.
//!
//! Frame, in production order, one encoder:
//!   clear hdr+depth (sky) → copy hdr→scene_color → water surface pass
//!   (GpuProfiler "water" span) → render_accents (manual write_timestamp span).
//! The ground geometry of the W.2a probe is omitted: it shaped refraction
//! *appearance*, not water/accent *cost* — the GPU work of both passes is
//! identical against a cleared scene-color/depth.
//!
//! Run: `cargo run -p weaving_playground --example accent_budget_probe --release`

use astraweave_fluids::{FluidRenderer, FluidSystem, SecondaryParticle};
use astraweave_render::{GpuProfiler, WaterRenderer, WeaveInstance, WeaveKind};
use glam::{Mat4, Vec2, Vec3};

const W: u32 = 1920;
const H: u32 = 1080;
const COLOR_FMT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const DEPTH_FMT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const WARMUP: usize = 60;
const FRAMES: usize = 240;
/// Representative weave-impact accent density (several active weaves throwing
/// spray). The demo peaks ~1–2k; 512 is a mid-range representative count.
const ACCENT_COUNT: usize = 512;

struct CamCfg {
    name: &'static str,
    eye: Vec3,
    target: Vec3,
}

fn mats(eye: Vec3, target: Vec3) -> (Mat4, Mat4, Mat4) {
    let aspect = W as f32 / H as f32;
    let proj = Mat4::perspective_rh(60.0f32.to_radians(), aspect, 0.1, 4000.0);
    let view = Mat4::look_at_rh(eye, target, Vec3::Y);
    (view, proj, proj * view)
}

fn stats(mut s: Vec<f32>) -> (f32, f32, f32, f32) {
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = s.len().max(1);
    (s[0], s[n / 2], s.iter().sum::<f32>() / n as f32, s[n - 1])
}

fn print_stats(label: &str, s: Vec<f32>) {
    if s.is_empty() {
        println!("  {label:<34} (no samples)");
        return;
    }
    let (min, med, mean, max) = stats(s);
    println!("  {label:<34} min={min:.4}ms  median={med:.4}ms  mean={mean:.4}ms  max={max:.4}ms");
}

/// Synthesize a representative accent set: `ACCENT_COUNT` billboards spread over
/// the near weave sites at the water surface, kinds 0/1/2 mixed.
fn synth_accents() -> Vec<SecondaryParticle> {
    let sites = [
        Vec2::new(0.0, 0.0),
        Vec2::new(35.0, 0.0),
        Vec2::new(-35.0, 0.0),
        Vec2::new(0.0, 35.0),
        Vec2::new(0.0, -35.0),
    ];
    (0..ACCENT_COUNT)
        .map(|i| {
            let s = sites[i % sites.len()];
            // Cheap deterministic spread.
            let a = (i as f32) * 0.618_034;
            let jx = (a.sin()) * 8.0;
            let jz = (a.cos()) * 8.0;
            let jy = ((i * 7 % 13) as f32) * 0.25;
            SecondaryParticle {
                position: [s.x + jx, 2.0 + jy, s.y + jz, 1.0],
                velocity: [jx * 0.2, 1.0, jz * 0.2, 0.0],
                info: [0.3, (i % 3) as f32, 0.8, 0.5], // age, kind, alpha, scale
            }
        })
        .collect()
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
        println!(" F.4.3 COMBINED WATER BUDGET PROBE (surface + accents)");
        println!("===========================================================");
        println!("Adapter : {}", info.name);
        println!("Backend : {:?}", info.backend);
        println!("Type    : {:?}", info.device_type);
        println!("Driver  : {} {}", info.driver, info.driver_info);
        println!("Target  : {}x{}  accents={}", W, H, ACCENT_COUNT);
        let has_ts = adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY);
        let has_inside = adapter
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS);
        println!("TIMESTAMP_QUERY={has_ts}  INSIDE_ENCODERS={has_inside}");
        if matches!(info.device_type, wgpu::DeviceType::Cpu) {
            println!("!! WARNING: software rasterizer — NOT a min-spec GPU measurement.");
        }

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("accent_probe_device"),
                required_features: {
                    let mut f = wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
                    if has_ts {
                        f |= wgpu::Features::TIMESTAMP_QUERY;
                    }
                    if has_inside {
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

        // Targets.
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
        let hdr = mk(
            "probe_hdr",
            COLOR_FMT,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
        );
        let hdr_view = hdr.create_view(&Default::default());
        let depth = mk(
            "probe_depth",
            DEPTH_FMT,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let depth_view = depth.create_view(&Default::default());
        let scene_color = mk(
            "probe_scene_color",
            COLOR_FMT,
            wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let scene_color_view = scene_color.create_view(&Default::default());

        // Water surface with 8 representative weave instances.
        let mut water = WaterRenderer::new(&device, COLOR_FMT, DEPTH_FMT);
        let mkw = |kind, x: f32, z: f32| WeaveInstance {
            kind,
            position: Vec2::new(x, z),
            radius: 25.0,
            orientation: 0.0,
            intensity: 0.8,
            phase: 0.0,
        };
        let weaves = [
            mkw(WeaveKind::Part, 0.0, 0.0),
            mkw(WeaveKind::Raise, 35.0, 0.0),
            mkw(WeaveKind::Freeze, -35.0, 0.0),
            mkw(WeaveKind::Part, 0.0, 35.0),
            mkw(WeaveKind::Raise, 0.0, -35.0),
            mkw(WeaveKind::Part, 35.0, 35.0),
            mkw(WeaveKind::Raise, -35.0, 35.0),
            mkw(WeaveKind::Freeze, 35.0, -35.0),
        ];

        // Accent renderer + buffer (HDR format), one representative live set.
        let mut fluid_system = FluidSystem::new(&device, 2048);
        let fluid_renderer = FluidRenderer::new(&device, W, H, COLOR_FMT);
        fluid_system.set_secondary_particles(&queue, &synth_accents());
        let accent_count = fluid_system.secondary_particle_count();
        println!("Live accents uploaded: {accent_count}\n");

        let mut profiler = if has_ts { Some(GpuProfiler::new(&device, &queue)) } else { None };
        // Manual 2-query set for the accent pass (write_timestamp needs INSIDE_ENCODERS).
        let acc_qset = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("accent_qs"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });
        let acc_resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("acc_resolve"),
            size: 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let acc_read = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("acc_read"),
            size: 16,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ts_period = queue.get_timestamp_period();

        let cams = [
            CamCfg { name: "near", eye: Vec3::new(0.0, 14.0, 70.0), target: Vec3::new(0.0, 2.0, 0.0) },
            CamCfg { name: "horizon", eye: Vec3::new(0.0, 4.0, 245.0), target: Vec3::new(0.0, 2.0, -120.0) },
        ];

        for cam in &cams {
            let (view, _proj, vp) = mats(cam.eye, cam.target);
            let camu = astraweave_fluids::renderer::CameraUniform {
                view_proj: vp.to_cols_array_2d(),
                inv_view_proj: vp.inverse().to_cols_array_2d(),
                view_inv: view.inverse().to_cols_array_2d(),
                cam_pos: [cam.eye.x, cam.eye.y, cam.eye.z, 1.0],
                light_dir: [0.3, 0.9, 0.2, 0.0],
                time: 0.0,
                padding: [0.0; 19],
            };
            water.set_weave_instances(&weaves);

            let mut water_ms: Vec<f32> = Vec::with_capacity(FRAMES);
            let mut accent_ms: Vec<f32> = Vec::with_capacity(FRAMES);

            for frame in 0..(WARMUP + FRAMES) {
                let time = frame as f32 * (1.0 / 60.0);
                water.update(&queue, vp, cam.eye, time);
                water.prepare_scene(&device, &queue, &scene_color_view, &depth_view, [W as f32, H as f32], 0);
                if let Some(ref mut p) = profiler {
                    p.begin_frame();
                }
                let mut enc = device.create_command_encoder(&Default::default());

                // Clear hdr (sky) + depth.
                {
                    let _c = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("clear"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &hdr_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.5, g: 0.7, b: 0.95, a: 1.0 }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &depth_view,
                            depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                }
                // Snapshot hdr → scene_color (the water pass samples it).
                enc.copy_texture_to_texture(
                    wgpu::TexelCopyTextureInfo { texture: &hdr, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                    wgpu::TexelCopyTextureInfo { texture: &scene_color, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                    wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
                );
                // Water surface pass (GpuProfiler "water" span).
                {
                    let ts = profiler.as_mut().and_then(|p| p.render_pass_timestamps("water"));
                    let mut wp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("water_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &hdr_view,
                            resolve_target: None,
                            ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &depth_view,
                            depth_ops: None,
                            stencil_ops: None,
                        }),
                        timestamp_writes: ts,
                        occlusion_query_set: None,
                    });
                    water.render(&mut wp);
                }
                // Accent composite (manual timestamp span around render_accents).
                if has_inside {
                    enc.write_timestamp(&acc_qset, 0);
                }
                fluid_renderer.render_accents(
                    &queue,
                    &mut enc,
                    &hdr_view,
                    &depth_view,
                    fluid_system.secondary_particle_buffer(),
                    accent_count,
                    camu,
                );
                if has_inside {
                    enc.write_timestamp(&acc_qset, 1);
                    enc.resolve_query_set(&acc_qset, 0..2, &acc_resolve, 0);
                    enc.copy_buffer_to_buffer(&acc_resolve, 0, &acc_read, 0, 16);
                }
                if let Some(ref p) = profiler {
                    p.end_frame(&mut enc);
                }
                queue.submit(Some(enc.finish()));
                let _ = device.poll(wgpu::PollType::Wait);

                if has_inside {
                    let slice = acc_read.slice(..);
                    slice.map_async(wgpu::MapMode::Read, |_| {});
                    let _ = device.poll(wgpu::PollType::Wait);
                    let data = slice.get_mapped_range();
                    let t0 = u64::from_le_bytes(data[0..8].try_into().expect("ts0"));
                    let t1 = u64::from_le_bytes(data[8..16].try_into().expect("ts1"));
                    let dt = (t1.wrapping_sub(t0)) as f64 * ts_period as f64 / 1_000_000.0;
                    drop(data);
                    acc_read.unmap();
                    if frame >= WARMUP {
                        accent_ms.push(dt as f32);
                    }
                }
                if let Some(ref mut p) = profiler {
                    p.request_readback();
                    let _ = device.poll(wgpu::PollType::Wait);
                    p.poll_readback(&device);
                    if frame >= WARMUP {
                        if let Some(ms) = p.results_map().get("water") {
                            water_ms.push(*ms);
                        }
                    }
                }
            }

            let wmed = if water_ms.is_empty() { 0.0 } else { stats(water_ms.clone()).1 };
            let amed = if accent_ms.is_empty() { 0.0 } else { stats(accent_ms.clone()).1 };
            println!("[{}] @ {ACCENT_COUNT} accents:", cam.name);
            print_stats("water surface pass", water_ms);
            print_stats("accent composite", accent_ms);
            println!(
                "  COMBINED median = {:.4} ms   (accent vs 0.5ms target: {}; total vs 2.0ms budget: {})",
                wmed + amed,
                if amed <= 0.5 { "PASS" } else { "OVER" },
                if wmed + amed <= 2.0 { "PASS" } else { "OVER" }
            );
            println!();
        }
        println!("=== done ===");
    });
}
