//! Editor-Engine Render Parity Harness (P.1 baseline, failure-first).
//!
//! Single-test integration harness for the Editor-Engine Render Parity campaign
//! launched by P.0. Renders the agreed grassland fixture through the engine
//! production path (`Renderer::new_from_device` + `draw_into` — the same pattern
//! `EngineRenderAdapter::new` uses) and through the editor viewport path
//! (`ViewportRenderer::render` with grid / physics-debug / gizmo disabled),
//! then computes SHA-256 of each path's readback bytes and asserts equality.
//!
//! **Expected to FAIL at P.1.** The hash mismatch is the campaign's regression
//! target. Each subsequent sub-phase (P.2 loader, P.3 tonemap, P.4 quality
//! preset, P.5 target format, P.6 composition layer) closes one of the named
//! seams from the P.0 audit and reduces the per-axis SAD. P.7 removes the
//! `#[ignore]` attribute when hash equality is achieved.
//!
//! Per-machine parity contract: this harness verifies editor and engine produce
//! identical bytes on whatever GPU runs it. Cross-machine reproducibility is
//! explicitly out of scope. `wgpu::AdapterInfo` is logged on every run so a
//! future failure can be distinguished as either a real parity regression or
//! a GPU/driver environment change.
//!
//! Run: `cargo test -p aw_editor --test render_parity_harness -- --include-ignored --nocapture`

use anyhow::{Context, Result};
use astraweave_core::World;
use astraweave_render::Renderer;
use aw_editor_lib::viewport::{OrbitCamera, ViewportRenderer};
use glam::Vec3;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Arc;

const WIDTH: u32 = 512;
const HEIGHT: u32 = 512;
const FIXED_TIME_OF_DAY: f32 = 12.0;

/// Locked fixture parameters. See P.0 Phase 5 / P.1 Phase 1 audit summary.
struct ParityFixture {
    width: u32,
    height: u32,
    time_of_day: f32,
    biome_path: PathBuf,
}

impl ParityFixture {
    fn default_grassland() -> Self {
        Self {
            width: WIDTH,
            height: HEIGHT,
            time_of_day: FIXED_TIME_OF_DAY,
            biome_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../assets/materials/grassland"),
        }
    }

    fn camera(&self) -> OrbitCamera {
        // 25m distance, 45° yaw, 30° pitch — matches OrbitCamera::default()'s
        // intent (diagonal view) with the fixture aspect locked to 1:1.
        let mut cam = OrbitCamera::new(
            Vec3::ZERO,
            25.0,
            std::f32::consts::FRAC_PI_4,
            std::f32::consts::FRAC_PI_6,
        );
        cam.set_aspect(self.width as f32, self.height as f32);
        cam
    }
}

/// Engine production path readback (Rgba16Float HDR passthrough; 8 B / px).
struct EngineFrame {
    bytes: Vec<u8>,
    width: u32,
    height: u32,
}

/// Editor viewport path readback (Rgba8UnormSrgb LDR tonemapped; 4 B / px).
struct EditorFrame {
    bytes: Vec<u8>,
    width: u32,
    height: u32,
}

async fn acquire_device() -> Result<(Arc<wgpu::Device>, Arc<wgpu::Queue>, wgpu::AdapterInfo)> {
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
        .context("no suitable wgpu adapter for parity harness")?;
    let info = adapter.get_info();
    // Renderer::new requires max_bind_groups: 8 — see renderer.rs:1084.
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("parity-harness device"),
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
    Ok((Arc::new(device), Arc::new(queue), info))
}

async fn render_engine_path(
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    fixture: &ParityFixture,
) -> Result<EngineFrame> {
    // Mirror EngineRenderAdapter::new (tools/aw_editor/src/viewport/engine_adapter.rs:626-647):
    // Bgra8UnormSrgb config + surface=None. This selects the engine's editor-mode
    // hdr_blit_pipeline branch inside draw_into (renderer.rs:5910-5921) — the same
    // branch the editor's adapter consumes. The engine production windowed path
    // (post_pipeline with ACES + scene-env tint) is not invokable headlessly with
    // the current API (it requires surface=Some), and anti-drift constraint 10
    // forbids expanding the engine API for that. The Axis 11 (tonemap) divergence
    // therefore registers fully at P.1's baseline — that is the campaign's point.
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: fixture.width,
        height: fixture.height,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    let device_owned = (*device).clone();
    let queue_owned = (*queue).clone();
    let mut renderer = Renderer::new_from_device(device_owned, queue_owned, None, config)
        .await
        .context("Renderer::new_from_device failed")?;

    renderer.time_of_day_mut().current_time = fixture.time_of_day;

    let camera = fixture.camera();
    renderer.update_camera_matrices(
        camera.view_matrix(),
        camera.projection_matrix(),
        camera.position(),
        0.5,
        5000.0,
        60f32.to_radians(),
        fixture.width as f32 / fixture.height as f32,
    );

    // External Rgba16Float HDR target — matches hdr_blit_pipeline's hardcoded
    // output format at renderer.rs:2456.
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("parity-engine-hdr-target"),
        size: wgpu::Extent3d {
            width: fixture.width,
            height: fixture.height,
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
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());

    // Warm-up frame so clustered-lights cache + IBL bake settle deterministically.
    let mut enc1 = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("parity-engine warm-up encoder"),
    });
    renderer
        .draw_into(&view, None, &mut enc1)
        .context("engine warm-up draw_into failed")?;
    queue.submit(std::iter::once(enc1.finish()));

    // Measurement frame.
    let mut enc2 = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("parity-engine measurement encoder"),
    });
    renderer
        .draw_into(&view, None, &mut enc2)
        .context("engine measurement draw_into failed")?;

    let bytes = readback_texture(&device, &queue, &target, fixture.width, fixture.height, 8, enc2)?;
    Ok(EngineFrame {
        bytes,
        width: fixture.width,
        height: fixture.height,
    })
}

async fn render_editor_path(
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    fixture: &ParityFixture,
) -> Result<EditorFrame> {
    let mut viewport = ViewportRenderer::new(device.clone(), queue.clone())
        .context("ViewportRenderer::new failed")?;
    viewport
        .init_engine_adapter()
        .await
        .context("ViewportRenderer::init_engine_adapter failed")?;
    if let Some(adapter) = viewport.engine_adapter_mut() {
        adapter.set_time_of_day(fixture.time_of_day);
    }

    // LDR target — matches viewport's LDR_COLOR_FORMAT
    // (tools/aw_editor/src/viewport/renderer.rs:34). The `view_formats` slice
    // mirrors the editor's own create_render_texture (renderer.rs:807) so the
    // tonemap blit's binding is layout-compatible across both formats.
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("parity-editor-ldr-target"),
        size: wgpu::Extent3d {
            width: fixture.width,
            height: fixture.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
    });

    let camera = fixture.camera();
    let world = World::new();

    // Two-frame settle: first call also lazily allocates HDR/depth targets,
    // tonemap pipeline, and engine adapter state. The measurement is frame 2.
    viewport
        .render(&target, &camera, &world, None, None, None, false, false, 0)
        .context("editor warm-up render failed")?;
    viewport
        .render(&target, &camera, &world, None, None, None, false, false, 0)
        .context("editor measurement render failed")?;

    let enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("parity-editor readback encoder"),
    });
    let bytes = readback_texture(&device, &queue, &target, fixture.width, fixture.height, 4, enc)?;
    Ok(EditorFrame {
        bytes,
        width: fixture.width,
        height: fixture.height,
    })
}

/// Generic texture readback. Encoder must be open (no submit yet); this fn
/// records `copy_texture_to_buffer`, submits, maps synchronously, de-pads rows.
fn readback_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    tex: &wgpu::Texture,
    width: u32,
    height: u32,
    bytes_per_pixel: u32,
    mut encoder: wgpu::CommandEncoder,
) -> Result<Vec<u8>> {
    let unpadded = width * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded = unpadded.div_ceil(align) * align;
    let size = (padded * height) as u64;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("parity-readback-staging"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));

    let (tx, rx) = std::sync::mpsc::channel();
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
    loop {
        if device.poll(wgpu::MaintainBase::Wait).is_ok() {
            if let Ok(r) = rx.try_recv() {
                r.context("map_async failed")?;
                break;
            }
        }
    }
    let mapped = staging.slice(..).get_mapped_range();
    let mut out = Vec::with_capacity((unpadded * height) as usize);
    let row_stride = padded as usize;
    let row_bytes = unpadded as usize;
    for y in 0..height as usize {
        let src = y * row_stride;
        out.extend_from_slice(&mapped[src..src + row_bytes]);
    }
    drop(mapped);
    staging.unmap();
    Ok(out)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

// ─── Linear-space normalization (cross-format SAD) ──────────────────────────

#[inline]
fn rgba16f_to_linear(pixel: &[u8]) -> [f32; 3] {
    let h = |a: u8, b: u8| half::f16::from_le_bytes([a, b]).to_f32();
    [
        h(pixel[0], pixel[1]),
        h(pixel[2], pixel[3]),
        h(pixel[4], pixel[5]),
    ]
}

#[inline]
fn rgba8srgb_to_linear(pixel: &[u8]) -> [f32; 3] {
    let s = |v: u8| -> f32 {
        let f = v as f32 / 255.0;
        if f <= 0.04045 {
            f / 12.92
        } else {
            ((f + 0.055) / 1.055).powf(2.4)
        }
    };
    [s(pixel[0]), s(pixel[1]), s(pixel[2])]
}

// ─── Per-axis SAD attribution (heuristic) ───────────────────────────────────
//
// Per the P.1 prompt: attribution does not need to sum to 100% and is not
// expected to perfectly isolate any axis. The point is directional usefulness
// for sub-phase progress tracking — each subsequent sub-phase reduces the SAD
// attributed to the named axis it closes.
//
// Heuristics used here:
// - **Tonemap axis (Axis 11)**: synthesize what engine HDR WOULD look like
//   after the canonical engine ACES pass (POST_SHADER, exposure 1.35, no
//   tint — scene-env tint is identity in default state). The SAD reduction
//   from pre-tonemap to post-tonemap on the engine side, per pixel, is
//   tonemap-attributable.
// - **Target format axis (Axis 6)**: synthesize a Rgba8UnormSrgb round-trip
//   of engine HDR (clamp [0,1] → sRGB-encode → 8-bit truncate → sRGB-decode).
//   The SAD between original and round-tripped engine values is the bytes
//   that would be lost purely to format quantization.
// - **Loader axis (Axes 1, 10)** and **quality preset axis (Axis 8)**: probed
//   at 16 fixed sample positions each. Without a real terrain chunk uploaded
//   in P.1's fixture (sky + engine-default ground plane only), the loader
//   probe registers near-zero. Reported faithfully; P.2 expands the fixture.
// - **Overlay composition (residual)**: `total - (tonemap + format)`. The
//   loader/quality probe SADs are reported separately and not double-counted.

struct AxisAttribution {
    total_sad: f64,
    tonemap_axis_sad: f64,
    target_format_axis_sad: f64,
    loader_axis_probe_sad: f64,
    loader_axis_probe_pixels: u64,
    quality_preset_axis_probe_sad: f64,
    quality_preset_axis_probe_pixels: u64,
    overlay_composition_residual: f64,
    pixel_count: u64,
}

impl AxisAttribution {
    fn format_report(&self) -> String {
        let pct = |v: f64| -> f64 {
            if self.total_sad > 0.0 {
                100.0 * v / self.total_sad
            } else {
                0.0
            }
        };
        format!(
            "Per-axis SAD attribution (heuristic, linear-RGB space):\n  \
             Pixels compared: {}\n  \
             Total SAD: {:.4} (mean {:.6} / pixel)\n  \
             \n  \
             Full-frame attributions:\n    \
             - Tonemap divergence (Axis 11):       {:>12.4} ({:.1}%)\n    \
             - Target format divergence (Axis 6):  {:>12.4} ({:.1}%)\n    \
             - Overlay composition (residual):     {:>12.4} ({:.1}%)\n  \
             \n  \
             Sampled-probe attributions (16 fixed positions each):\n    \
             - Loader divergence (Axes 1, 10):     {:>12.4} over {} px\n    \
             - Quality preset (Axis 8):            {:>12.4} over {} px",
            self.pixel_count,
            self.total_sad,
            self.total_sad / self.pixel_count.max(1) as f64,
            self.tonemap_axis_sad,
            pct(self.tonemap_axis_sad),
            self.target_format_axis_sad,
            pct(self.target_format_axis_sad),
            self.overlay_composition_residual,
            pct(self.overlay_composition_residual),
            self.loader_axis_probe_sad,
            self.loader_axis_probe_pixels,
            self.quality_preset_axis_probe_sad,
            self.quality_preset_axis_probe_pixels,
        )
    }
}

/// Engine canonical ACES (POST_SHADER, astraweave-render/src/renderer.rs:366-383):
/// ACES Narkowicz fit with exposure pre-multiply 1.35. Scene-env tint at default
/// is identity (`tint_alpha = 0.0`), so the `mix(color, tint_color, tint_alpha)`
/// at POST_SHADER:381 is a no-op for the default fixture.
#[inline]
fn aces_canonical(rgb: [f32; 3]) -> [f32; 3] {
    const EXPOSURE: f32 = 1.35;
    let aces = |x: f32| -> f32 {
        let xe = x * EXPOSURE;
        let a = 2.51;
        let b = 0.03;
        let c = 2.43;
        let d = 0.59;
        let e = 0.14;
        ((xe * (a * xe + b)) / (xe * (c * xe + d) + e)).clamp(0.0, 1.0)
    };
    [aces(rgb[0]), aces(rgb[1]), aces(rgb[2])]
}

/// Round-trip a linear value through Rgba8UnormSrgb (clamp to [0,1], sRGB-encode,
/// truncate to u8, sRGB-decode back to linear). The discrepancy between input
/// and output is what would be lost to format-axis quantization.
#[inline]
fn srgb_u8_roundtrip(v: f32) -> f32 {
    let clamped = v.clamp(0.0, 1.0);
    let srgb = if clamped <= 0.0031308 {
        clamped * 12.92
    } else {
        1.055 * clamped.powf(1.0 / 2.4) - 0.055
    };
    let u = (srgb * 255.0).round().clamp(0.0, 255.0) as u8;
    let f = u as f32 / 255.0;
    if f <= 0.04045 {
        f / 12.92
    } else {
        ((f + 0.055) / 1.055).powf(2.4)
    }
}

fn compute_attribution(engine: &EngineFrame, editor: &EditorFrame) -> AxisAttribution {
    assert_eq!(engine.width, editor.width);
    assert_eq!(engine.height, editor.height);
    let w = engine.width as usize;
    let h = engine.height as usize;

    // Fixed probe positions. Real attribution requires per-pixel knowledge of
    // which axis dominates — impossible without running ablated paths (P.2-P.6
    // scope). These probes register samples in regions where the named axis
    // would heuristically dominate; P.1's fixture has no terrain chunk so the
    // loader probe is near-zero by design (documented in the report comment).
    let loader_xy: Vec<(usize, usize)> = (0..16)
        .map(|i| {
            let col = i % 4;
            let row = i / 4;
            let x = w / 8 + (w * col * 3) / 16;
            let y = h * 5 / 8 + (h * row) / 32;
            (x.min(w - 1), y.min(h - 1))
        })
        .collect();
    let quality_xy: Vec<(usize, usize)> = (0..16)
        .map(|i| {
            let col = i % 4;
            let row = i / 4;
            let x = w / 4 + (w * col) / 8;
            let y = h / 8 + (h * row) / 32; // upper-frame: sky/horizon region
            (x.min(w - 1), y.min(h - 1))
        })
        .collect();
    let loader_set: std::collections::HashSet<(usize, usize)> =
        loader_xy.iter().copied().collect();
    let quality_set: std::collections::HashSet<(usize, usize)> =
        quality_xy.iter().copied().collect();

    let mut total_sad = 0.0f64;
    let mut tonemap_sad = 0.0f64;
    let mut format_sad = 0.0f64;
    let mut loader_probe_sad = 0.0f64;
    let mut quality_probe_sad = 0.0f64;

    for y in 0..h {
        for x in 0..w {
            let eng_idx = (y * w + x) * 8;
            let edt_idx = (y * w + x) * 4;
            let eng_lin = rgba16f_to_linear(&engine.bytes[eng_idx..eng_idx + 8]);
            let edt_lin = rgba8srgb_to_linear(&editor.bytes[edt_idx..edt_idx + 4]);

            let dr = (eng_lin[0] - edt_lin[0]).abs() as f64;
            let dg = (eng_lin[1] - edt_lin[1]).abs() as f64;
            let db = (eng_lin[2] - edt_lin[2]).abs() as f64;
            let pixel_sad = dr + dg + db;
            total_sad += pixel_sad;

            // Tonemap: how much SAD is removed when we tonemap engine HDR.
            let eng_tm = aces_canonical(eng_lin);
            let post_tm = ((eng_tm[0] - edt_lin[0]).abs()
                + (eng_tm[1] - edt_lin[1]).abs()
                + (eng_tm[2] - edt_lin[2]).abs()) as f64;
            tonemap_sad += (pixel_sad - post_tm).max(0.0);

            // Format: sRGB-u8 round-trip discrepancy on the engine value.
            // Compare against the tonemapped value rather than raw HDR so the
            // format-axis contribution reflects what would remain AFTER the
            // tonemap axis is closed. (Otherwise format and tonemap double-count
            // the clamp-to-[0,1] effect.)
            let fr = (eng_tm[0] - srgb_u8_roundtrip(eng_tm[0])).abs() as f64;
            let fg = (eng_tm[1] - srgb_u8_roundtrip(eng_tm[1])).abs() as f64;
            let fb = (eng_tm[2] - srgb_u8_roundtrip(eng_tm[2])).abs() as f64;
            format_sad += fr + fg + fb;

            if loader_set.contains(&(x, y)) {
                loader_probe_sad += pixel_sad;
            }
            if quality_set.contains(&(x, y)) {
                quality_probe_sad += pixel_sad;
            }
        }
    }

    let attributed_full_frame = tonemap_sad + format_sad;
    let overlay_composition_residual = (total_sad - attributed_full_frame).max(0.0);

    AxisAttribution {
        total_sad,
        tonemap_axis_sad: tonemap_sad,
        target_format_axis_sad: format_sad,
        loader_axis_probe_sad: loader_probe_sad,
        loader_axis_probe_pixels: 16,
        quality_preset_axis_probe_sad: quality_probe_sad,
        quality_preset_axis_probe_pixels: 16,
        overlay_composition_residual,
        pixel_count: (w * h) as u64,
    }
}

// ─── Test ────────────────────────────────────────────────────────────────────

#[test]
#[ignore = "P.1 baseline — expected to FAIL until P.7 closes Editor-Engine Render Parity"]
fn editor_engine_render_parity() {
    let fixture = ParityFixture::default_grassland();
    let (device, queue, adapter_info) =
        pollster::block_on(acquire_device()).expect("acquire wgpu device");

    eprintln!("============================================================");
    eprintln!("Editor-Engine Render Parity Harness (P.1 baseline)");
    eprintln!("============================================================");
    eprintln!(
        "Adapter: {} | device_type={:?} | backend={:?}",
        adapter_info.name, adapter_info.device_type, adapter_info.backend
    );
    eprintln!(
        "Driver info: \"{}\" | vendor=0x{:x} | device_id=0x{:x}",
        adapter_info.driver_info, adapter_info.vendor, adapter_info.device
    );
    eprintln!(
        "Fixture: {}x{} | ToD={} | biome={}",
        fixture.width,
        fixture.height,
        fixture.time_of_day,
        fixture.biome_path.display()
    );
    eprintln!("Per-machine parity contract — hash comparison valid only on this adapter.");
    eprintln!();

    let engine = pollster::block_on(render_engine_path(device.clone(), queue.clone(), &fixture))
        .expect("engine path render failed");
    let editor = pollster::block_on(render_editor_path(device.clone(), queue.clone(), &fixture))
        .expect("editor path render failed");

    let engine_hash = sha256_hex(&engine.bytes);
    let editor_hash = sha256_hex(&editor.bytes);

    eprintln!("Engine path: Rgba16Float HDR passthrough (draw_into, surface=None branch)");
    eprintln!(
        "  Bytes:   {} ({} px × 8 B/px)",
        engine.bytes.len(),
        engine.width * engine.height
    );
    eprintln!("  SHA-256: {}", engine_hash);
    eprintln!();
    eprintln!("Editor path: Rgba8UnormSrgb LDR (ViewportRenderer::render — engine + editor tonemap)");
    eprintln!(
        "  Bytes:   {} ({} px × 4 B/px)",
        editor.bytes.len(),
        editor.width * editor.height
    );
    eprintln!("  SHA-256: {}", editor_hash);
    eprintln!();

    let attribution = compute_attribution(&engine, &editor);
    eprintln!("{}", attribution.format_report());
    eprintln!();
    eprintln!("Heuristic limitations (per P.1 prompt):");
    eprintln!("  - Engine and editor produce different byte formats (8 B/px Rgba16Float vs");
    eprintln!("    4 B/px Rgba8UnormSrgb). Total SAD is computed in linear-RGB space after");
    eprintln!("    format-aware decoding. Hash comparison is on raw bytes — guaranteed to");
    eprintln!("    mismatch at P.1 until P.5 (target format unification) and P.3 (tonemap");
    eprintln!("    unification) collapse the two outputs to the same format and same pipeline.");
    eprintln!("  - Loader/quality attributions are sampled at 16 fixed positions only. P.1's");
    eprintln!("    fixture has no terrain chunk uploaded (sky + engine-default ground plane),");
    eprintln!("    so the loader probe registers small. P.2 expands the fixture to include the");
    eprintln!("    canonical 5-layer grassland splat (per the P.0 audit's fixture spec).");
    eprintln!("  - Cross-axis interactions are real. Attribution does not sum to 100%.");

    assert_eq!(
        engine_hash, editor_hash,
        "Parity hash mismatch (expected at P.1 baseline — campaign tracks reduction across P.2..P.6)"
    );
}
