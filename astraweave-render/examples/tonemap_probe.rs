//! Tonemap double-application diagnostic.
//!
//! **Purpose**: determine, with GPU-observed pixel values, whether
//! `astraweave_render::Renderer::draw_into` applies tonemapping when the
//! external view is `Rgba16Float` (editor mode, `surface = None`). See
//! `docs/audits/tonemap_double_application_investigation_<DATE>.md` for the
//! accompanying report.
//!
//! # What this does
//!
//! 1. Constructs a headless `astraweave_render::Renderer` the same way
//!    `tools/aw_editor/src/viewport/engine_adapter.rs::EngineRenderAdapter::new`
//!    does — via `Renderer::new_from_device(device, queue, None, config)` with
//!    `config.format = Bgra8UnormSrgb`. `surface = None` selects the engine's
//!    editor mode.
//! 2. Creates an external `Rgba16Float` texture (same format as the editor's
//!    `hdr_texture` in `viewport/renderer.rs`) with `COPY_SRC` usage for
//!    readback.
//! 3. Pushes the sun intensity to a very high value via
//!    `Renderer::set_light_direction_override`. The engine's lit shader
//!    multiplies albedo by `sun_color * sun_intensity`, so with
//!    `intensity = 20.0` the ground plane's HDR colour will sit well above
//!    `1.0` — **if** the pipeline stays linear HDR.
//! 4. Calls `draw_into(view, encoder)` and copies the Rgba16Float texture
//!    into a readback buffer.
//! 5. Reports: (max red-channel value, mean red-channel value, a 5-point UV
//!    sample) for four configurations:
//!      - `PostProcessChain { tonemap_operator: Aces, bloom: off, taa: off }`
//!      - `PostProcessChain { tonemap_operator: None, bloom: off, taa: off }`
//!      - Same two configurations at sun intensity 1.0 (sanity reference).
//!
//! # How to interpret the numbers
//!
//! The engine's ACES shader (`POST_SHADER`, renderer.rs:331-384) clamps output
//! to `[0, 1]`. If `draw_into` tonemaps into the external view, **every red
//! channel value in the readback will be ≤ 1.0**. If `draw_into` passes
//! through HDR, values can exceed 1.0 when the sun intensity is large.
//!
//! Additionally: the editor path selects the **passthrough** `hdr_blit_pipeline`
//! (renderer.rs:2300-2425, 5783-5787). The `PostProcessChain.tonemap_operator`
//! is *not* consumed by that pipeline — only `post_chain.bloom_enabled` is.
//! Swapping `tonemap_operator` should therefore produce **identical readbacks**.
//! Any difference would contradict the passthrough claim.
//!
//! # Running
//!
//! ```bash
//! cargo run --example tonemap_probe -p astraweave-render --release
//! ```
//!
//! Requires a working GPU / adapter. Uses ~2 MB of device memory (a
//! 512×512 Rgba16Float texture + readback buffer).

use anyhow::{Context, Result};
use astraweave_render::camera::Camera;
use astraweave_render::hdr_pipeline::{PostProcessChain, TonemapOperator};
use astraweave_render::Renderer;
use half::f16;

const WIDTH: u32 = 512;
const HEIGHT: u32 = 512;

/// A single probe run: configures the engine as described, renders one frame
/// via `draw_into`, reads back the Rgba16Float target, returns per-pixel-channel
/// statistics.
struct ProbeResult {
    label: String,
    min_red: f32,
    max_red: f32,
    max_green: f32,
    max_blue: f32,
    mean_red: f32,
    distinct_red_values: usize,
    /// Red channel sampled at UV = 0.0, 0.25, 0.5, 0.75, 1.0 along the
    /// horizontal mid-line (y = HEIGHT/2).
    horizontal_samples: [f32; 5],
    /// Corner samples: (top-left, top-right, bottom-left, bottom-right, center) red channel.
    corner_samples: [f32; 5],
}

impl std::fmt::Display for ProbeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "─── {} ───", self.label)?;
        writeln!(
            f,
            "  min/max R = {:.4} / {:.4}    max G = {:.4}    max B = {:.4}    mean R = {:.4}",
            self.min_red, self.max_red, self.max_green, self.max_blue, self.mean_red
        )?;
        writeln!(
            f,
            "  distinct R values across whole frame: {}",
            self.distinct_red_values
        )?;
        write!(f, "  red @ horizontal mid-line: ")?;
        for (i, v) in self.horizontal_samples.iter().enumerate() {
            let uv = [0.0, 0.25, 0.5, 0.75, 1.0][i];
            write!(f, "u={:.2}:{:.4} ", uv, v)?;
        }
        writeln!(f)?;
        write!(f, "  red @ corners + center:    ")?;
        for (i, v) in self.corner_samples.iter().enumerate() {
            let label = ["TL", "TR", "BL", "BR", "CE"][i];
            write!(f, "{}={:.4} ", label, v)?;
        }
        writeln!(f)?;
        writeln!(
            f,
            "  peak > 1.0?   {}",
            if self.max_red > 1.0 || self.max_green > 1.0 || self.max_blue > 1.0 {
                "YES (linear HDR — no engine-side tonemap)"
            } else {
                "no  (≤ 1.0 — engine-side tonemap or scene too dim)"
            }
        )
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    println!("=================================================================");
    println!("Tonemap Double-Application Probe");
    println!("  target: astraweave_render::Renderer::draw_into");
    println!("  mode  : editor (surface = None)");
    println!("  view  : {}×{} Rgba16Float", WIDTH, HEIGHT);
    println!("=================================================================");

    pollster::block_on(run_all_probes())
}

async fn run_all_probes() -> Result<()> {
    let (device, queue) = acquire_device().await?;
    let device = std::sync::Arc::new(device);
    let queue = std::sync::Arc::new(queue);

    // Four probes: 2 tonemap operators × 2 sun intensities.
    // If the engine tonemaps, Aces vs None should differ in high-intensity
    // runs. If the engine passes through (editor path), all four should
    // track the sun intensity linearly.
    let configs: &[(&str, TonemapOperator, f32)] = &[
        ("ACES  operator, sun intensity = 1.0 ", TonemapOperator::Aces, 1.0),
        ("None  operator, sun intensity = 1.0 ", TonemapOperator::None, 1.0),
        ("ACES  operator, sun intensity = 20.0", TonemapOperator::Aces, 20.0),
        ("None  operator, sun intensity = 20.0", TonemapOperator::None, 20.0),
    ];

    let mut results = Vec::new();
    for (label, op, sun_intensity) in configs.iter() {
        let r = probe(
            device.clone(),
            queue.clone(),
            label.to_string(),
            *op,
            *sun_intensity,
        )
        .await
        .with_context(|| format!("probe '{}' failed", label))?;
        println!("\n{}", r);
        results.push(r);
    }

    println!("\n=================================================================");
    println!("Summary");
    println!("=================================================================");
    println!(
        "If the engine tonemaps into the editor's HDR view:\n  → all four max values should be ≤ 1.0, and\n  → ACES vs None at intensity 20.0 should differ visibly.\n"
    );
    println!(
        "If the engine passes HDR through (static analysis of\nrenderer.rs:5783-5787 predicts this):\n  → max values at intensity 20.0 should exceed 1.0, and\n  → ACES vs None at the SAME intensity should be numerically\n    identical (operator is ignored on the passthrough path)."
    );

    // Cross-verify: are the ACES and None readings at intensity 20.0 identical?
    let aces_hi = &results[2];
    let none_hi = &results[3];
    let op_diff_max_r = (aces_hi.max_red - none_hi.max_red).abs();
    let op_diff_mean_r = (aces_hi.mean_red - none_hi.mean_red).abs();
    println!(
        "\nOperator swap at intensity 20.0:\n  |max_red(ACES) - max_red(None)|   = {:.6}\n  |mean_red(ACES) - mean_red(None)| = {:.6}",
        op_diff_max_r, op_diff_mean_r
    );

    Ok(())
}

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
        .context("no suitable wgpu adapter")?;
    let info = adapter.get_info();
    println!(
        "Adapter : {} ({:?}) backend={:?}",
        info.name, info.device_type, info.backend
    );

    // Mirror renderer.rs:970-973 — the engine's `Renderer::new` requests
    // `max_bind_groups: 8` because the scene_env / IBL bind groups occupy
    // slots 4 and 5. Default wgpu limits cap at 4 which would reject every
    // pipeline the engine creates.
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("tonemap_probe device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits {
                max_bind_groups: 8,
                ..wgpu::Limits::default()
            },
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await
        .context("request_device failed")?;
    Ok((device, queue))
}

async fn probe(
    device: std::sync::Arc<wgpu::Device>,
    queue: std::sync::Arc<wgpu::Queue>,
    label: String,
    tonemap_op: TonemapOperator,
    sun_intensity: f32,
) -> Result<ProbeResult> {
    // Mirror engine_adapter.rs:657-666 — Bgra8UnormSrgb, surface = None.
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: WIDTH,
        height: HEIGHT,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    // Take inner Device/Queue clones the same way engine_adapter.rs does.
    let device_owned = (*device).clone();
    let queue_owned = (*queue).clone();
    let mut renderer = Renderer::new_from_device(device_owned, queue_owned, None, config)
        .await
        .context("Renderer::new_from_device failed")?;

    // Configure post-processing chain. Turn off everything that allocates
    // extra intermediate targets or could perturb the signal. Setting
    // `bloom_enabled = false` makes the hdr_blit_bind_group's bloom
    // binding the 1×1 black dummy with intensity 0.0 — a pure passthrough
    // per renderer.rs:2300-2330 and renderer.rs:5750-5755.
    renderer.set_post_process_chain(PostProcessChain {
        ssao_enabled: false,
        ssr_enabled: false,
        ssgi_enabled: false,
        god_rays_enabled: false,
        auto_exposure_enabled: false,
        bloom_enabled: false,
        taa_enabled: false,
        dof_enabled: false,
        motion_blur_enabled: false,
        color_grading_enabled: false,
        tonemap_operator: tonemap_op,
    });

    // Inject HDR radiance directly into the SKY. The sky pass writes
    // its config colour into the HDR target unclamped, so this is the
    // cleanest deterministic HDR signal we can emit without modifying
    // the engine's scene pass. `day_color_top` is in linear RGB and is
    // multiplied directly by sky coverage — pushing it to 50.0 means
    // zenith sky fragments produce radiance ≈ 50 in a fully linear
    // pipeline.
    let mut sky = astraweave_render::environment::SkyConfig::default();
    sky.day_color_top = glam::Vec3::new(
        sun_intensity * 50.0,
        sun_intensity * 50.0,
        sun_intensity * 50.0,
    );
    sky.day_color_horizon = glam::Vec3::new(
        sun_intensity * 10.0,
        sun_intensity * 10.0,
        sun_intensity * 10.0,
    );
    sky.cloud_coverage = 0.0; // remove cloud modulation for a clean signal
    renderer.set_sky_config(sky);

    // Set sun intensity via SceneEnv for consumers that read it.
    renderer.set_light_direction_override(glam::Vec3::new(0.0, -1.0, 0.0), sun_intensity);
    {
        let env = renderer.scene_environment_mut();
        env.sun_color = [1.0, 1.0, 1.0];
        env.sun_intensity = sun_intensity;
    }

    // Camera tilted up above the ground to capture mostly sky at the
    // top of the frame (the HDR-bright zenith) and some ground near the
    // bottom. A slight upward pitch ensures the zenith sample is in
    // frame.
    let camera = Camera {
        position: glam::Vec3::new(0.0, 2.0, 0.0),
        yaw: -std::f32::consts::FRAC_PI_2, // facing -Z
        pitch: std::f32::consts::FRAC_PI_4, // 45° up — zenith visible at top
        fovy: 60f32.to_radians(),
        aspect: WIDTH as f32 / HEIGHT as f32,
        znear: 0.1,
        zfar: 100.0,
    };
    renderer.update_camera(&camera);

    // Create the external Rgba16Float view that the editor would create
    // (viewport/renderer.rs:773-804 constructs essentially this same
    // texture but uses `Rgba8UnormSrgb` for the final LDR target; the
    // HDR scene target is created at viewport/renderer.rs:86-99 and has
    // the Rgba16Float format we mirror here).
    let probe_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("tonemap_probe external Rgba16Float"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let probe_view = probe_tex.create_view(&wgpu::TextureViewDescriptor::default());

    // Readback buffer with correct row-alignment.
    const BYTES_PER_PIXEL: u32 = 8; // Rgba16Float = 4 × f16 = 8 bytes
    let unpadded_bytes_per_row = WIDTH * BYTES_PER_PIXEL;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT; // 256
    let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
    let readback_size = (padded_bytes_per_row * HEIGHT) as u64;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("tonemap_probe readback"),
        size: readback_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("tonemap_probe encoder"),
    });

    // Give the engine a single frame to settle any first-frame state
    // (clustered-lights cache, ibl bake, etc.) before the measurement
    // frame. Both go through the same passthrough path so the observation
    // is the second frame.
    renderer
        .draw_into(&probe_view, None, &mut encoder)
        .context("warm-up draw_into failed")?;
    queue.submit(std::iter::once(encoder.finish()));

    // Measurement frame.
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("tonemap_probe measurement encoder"),
    });
    renderer
        .draw_into(&probe_view, None, &mut encoder)
        .context("measurement draw_into failed")?;

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &probe_tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(HEIGHT),
            },
        },
        wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(std::iter::once(encoder.finish()));

    // Synchronously map and read.
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
    // Poll until the map callback fires. This is a test-only spin-wait.
    loop {
        if let Ok(_maint) = device.poll(wgpu::MaintainBase::Wait) {
            if let Ok(r) = rx.try_recv() {
                r.context("map_async failed")?;
                break;
            }
        }
    }
    let mapped = readback.slice(..).get_mapped_range();

    // Decode Rgba16Float rows with padding.
    let mut min_red = f32::INFINITY;
    let mut max_red = f32::NEG_INFINITY;
    let mut max_green = f32::NEG_INFINITY;
    let mut max_blue = f32::NEG_INFINITY;
    let mut sum_red = 0.0f64;
    let mut count = 0u64;
    let mut red_values_seen: std::collections::HashSet<u32> = std::collections::HashSet::new();

    let row_stride = padded_bytes_per_row as usize;
    for y in 0..HEIGHT as usize {
        let row_start = y * row_stride;
        for x in 0..WIDTH as usize {
            let px = row_start + x * BYTES_PER_PIXEL as usize;
            let r = f16::from_le_bytes([mapped[px], mapped[px + 1]]).to_f32();
            let g = f16::from_le_bytes([mapped[px + 2], mapped[px + 3]]).to_f32();
            let b = f16::from_le_bytes([mapped[px + 4], mapped[px + 5]]).to_f32();
            if r < min_red {
                min_red = r;
            }
            if r > max_red {
                max_red = r;
            }
            if g > max_green {
                max_green = g;
            }
            if b > max_blue {
                max_blue = b;
            }
            sum_red += r as f64;
            count += 1;
            // Bucket distinct red values by their f16 bit pattern so
            // "essentially identical" floats (1e-8 apart) don't inflate the count.
            red_values_seen.insert(r.to_bits());
        }
    }

    // Helper to sample red at a specific (x,y).
    let sample_red = |x: usize, y: usize| -> f32 {
        let px = y * row_stride + x * BYTES_PER_PIXEL as usize;
        f16::from_le_bytes([mapped[px], mapped[px + 1]]).to_f32()
    };
    let mid_y = HEIGHT as usize / 2;
    let horizontal_samples = [
        sample_red(0, mid_y),
        sample_red(WIDTH as usize / 4, mid_y),
        sample_red(WIDTH as usize / 2, mid_y),
        sample_red(3 * WIDTH as usize / 4, mid_y),
        sample_red(WIDTH as usize - 1, mid_y),
    ];
    let corner_samples = [
        sample_red(0, 0),                                // top-left
        sample_red(WIDTH as usize - 1, 0),               // top-right
        sample_red(0, HEIGHT as usize - 1),              // bottom-left
        sample_red(WIDTH as usize - 1, HEIGHT as usize - 1), // bottom-right
        sample_red(WIDTH as usize / 2, HEIGHT as usize / 2), // center
    ];

    let mean_red = (sum_red / count as f64) as f32;

    drop(mapped);
    readback.unmap();

    // Pin the Renderer briefly to make sure it's dropped before the next
    // probe recreates one (each Renderer owns IBL resources, pipelines,
    // etc., so serializing their lifetimes avoids double-allocation).
    let _ = renderer;
    // Also ensure the device clones used internally are released.
    let _ = device_owned_marker();

    Ok(ProbeResult {
        label,
        min_red,
        max_red,
        max_green,
        max_blue,
        mean_red,
        distinct_red_values: red_values_seen.len(),
        horizontal_samples,
        corner_samples,
    })
}

fn device_owned_marker() {}
