//! ED-3 — live proof that the formerly inert controls now work.
//!
//! Two `#[ignore]`d GPU tests, both driving the editor's LIVE path
//! (`ViewportRenderer::render` + ED-2's `capture_frame_png` — the same
//! sequence the widget performs; ED2_OUTCOME.md §3.1):
//!
//! * `ed3_shading_modes_render_differently` — renders one pinned station in
//!   all five shading modes and asserts every non-Lit mode differs from Lit.
//!   **On pre-ED-3 code this test fails by construction**: the
//!   `shading_mode` argument was `_shading_mode` (unused), so all five
//!   frames rendered byte-identical — exactly the defect.
//!
//! * `ed3_amplitude_scale_changes_terrain` — generates the same world at
//!   amplitude scales 0.5 / 1.0 / 2.0 and asserts the rendered frames
//!   differ pairwise. This is the render-level twin of
//!   `astraweave-terrain::ed3_base_amplitude_scale_is_live_and_identity_at_default`,
//!   and the exact check that proved the old slider dead (T.2d Experiment F:
//!   three amplitudes, byte-identical frames).
//!
//! ```text
//! cargo test -p aw_editor --profile release-fast --test ed3_proof -- --ignored --nocapture
//! ```

use anyhow::{Context, Result};
use astraweave_core::World;
use astraweave_terrain::world_archetypes::WorldArchetypeId;
use aw_editor_lib::terrain_integration::TerrainState;
use aw_editor_lib::viewport::{OrbitCamera, ViewportRenderer};
use glam::Vec3;
use std::path::PathBuf;
use std::sync::Arc;

const TIME_OF_DAY: f32 = 12.0;
const SEED: u64 = 12345;
const PRIMARY_BIOME: &str = "grassland";

/// T.2a's pinned desert focal (ground 36.3 m) — the same anchor the T.2d/T.2d.F
/// stations use.
const DESERT_FOCAL: [f32; 3] = [43.1, 36.3, -1961.8];

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 768;

fn out_root() -> PathBuf {
    PathBuf::from(std::env::var("ED3_OUT").unwrap_or_else(|_| "d:/tmp/ed3_staging".to_string()))
}

fn find_assets_dir() -> Option<PathBuf> {
    for candidate in ["assets", "../../assets"] {
        let p = PathBuf::from(candidate);
        if p.join("materials")
            .join("biomes")
            .join("materials.toml")
            .is_file()
        {
            return Some(p);
        }
    }
    None
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
        .context("no suitable wgpu adapter for the ED-3 proof")?;
    let info = adapter.get_info();
    // Mirror the editor's device setup: request POLYGON_MODE_LINE when the
    // adapter has it, so the Wireframe mode is exercised the way the live
    // editor exercises it (main.rs device descriptor).
    let mut features = wgpu::Features::empty();
    if adapter
        .features()
        .contains(wgpu::Features::POLYGON_MODE_LINE)
    {
        features |= wgpu::Features::POLYGON_MODE_LINE;
    }
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("ed3-proof device"),
            required_features: features,
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

async fn build_viewport(
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
) -> Result<ViewportRenderer> {
    let assets = find_assets_dir().context("assets dir not found")?;
    let biome_dir = assets.join("materials").join("biomes");
    let mut viewport =
        ViewportRenderer::new(device, queue).context("ViewportRenderer::new failed")?;
    viewport
        .init_engine_adapter()
        .await
        .context("init_engine_adapter failed")?;
    {
        let adapter = viewport
            .engine_adapter_mut()
            .context("engine adapter missing")?;
        adapter.set_time_of_day(TIME_OF_DAY);
        adapter.set_biome_pack(Some(biome_dir));
    }
    Ok(viewport)
}

fn generate_desert(radius: i32, amplitude_scale: f32) -> Result<TerrainState> {
    let mut state = TerrainState::new();
    state.configure(SEED, PRIMARY_BIOME);
    state.set_noise_params(6, 2.0, 0.5, 50.0);
    state.set_amplitude_scale(amplitude_scale);
    state.set_world_archetype(WorldArchetypeId::Desert.default_archetype());
    let n = state
        .generate_terrain(radius)
        .context("generate_terrain failed")?;
    println!("[ed3] desert radius {radius} scale {amplitude_scale}: {n} chunks");
    Ok(state)
}

/// The station: oblique over the desert anchor, relief in frame.
fn station_camera() -> OrbitCamera {
    let mut cam = OrbitCamera::new(
        Vec3::from_array(DESERT_FOCAL),
        46.7,
        45.0_f32.to_radians(),
        40.0_f32.to_radians(),
    );
    cam.set_aspect(WIDTH as f32, HEIGHT as f32);
    cam
}

fn luma_stats(path: &std::path::Path) -> Result<(f64, usize)> {
    let img = astraweave_asset::open_limited_image(path)?.to_rgba8();
    let mut sum = 0.0f64;
    for p in img.pixels() {
        sum += 0.2126 * p.0[0] as f64 + 0.7152 * p.0[1] as f64 + 0.0722 * p.0[2] as f64;
    }
    Ok((
        sum / (img.width() * img.height()) as f64,
        (img.width() * img.height()) as usize,
    ))
}

fn diff_pixels(a: &std::path::Path, b: &std::path::Path) -> Result<usize> {
    let ia = astraweave_asset::open_limited_image(a)?.to_rgba8();
    let ib = astraweave_asset::open_limited_image(b)?.to_rgba8();
    anyhow::ensure!(ia.dimensions() == ib.dimensions(), "dimension mismatch");
    Ok(ia
        .pixels()
        .zip(ib.pixels())
        .filter(|(x, y)| x.0 != y.0)
        .count())
}

#[test]
#[ignore = "GPU + terrain generation; run explicitly (see module docs)"]
fn ed3_shading_modes_render_differently() -> Result<()> {
    pollster::block_on(async {
        let out = out_root().join("modes");
        std::fs::create_dir_all(&out)?;
        let (device, queue, info) = acquire_device().await?;
        println!(
            "[ed3] adapter: {} · {:?} · driver {}",
            info.name, info.backend, info.driver_info
        );
        let mut viewport = build_viewport(device.clone(), queue.clone()).await?;
        let state = generate_desert(6, 1.0)?;
        viewport.upload_terrain_chunks_raw(&state.get_gpu_chunks());
        let texture = viewport.create_render_texture(WIDTH, HEIGHT)?;
        let world = World::new();
        let cam = station_camera();

        let modes = [
            (0u32, "0_lit"),
            (1, "1_unlit"),
            (2, "2_wireframe"),
            (3, "3_normals"),
            (4, "4_uvs"),
        ];
        let mut paths = Vec::new();
        for (mode, name) in modes {
            for _ in 0..2 {
                viewport
                    .render(&texture, &cam, &world, None, None, None, false, false, mode)
                    .with_context(|| format!("render failed in mode {name}"))?;
            }
            let png = out.join(format!("{name}.png"));
            viewport.capture_frame_png(&texture, &png)?;
            let (mean, _) = luma_stats(&png)?;
            println!("[ed3] {name}: mean luma {mean:.2} -> {}", png.display());
            paths.push((name, png));
        }

        println!(
            "[ed3] wireframe_supported: {}",
            viewport.wireframe_supported()
        );

        // Every non-Lit mode must differ from Lit. On pre-ED-3 code the
        // shading_mode argument was unused and all five frames were
        // byte-identical — this loop is the fails-on-old-code proof.
        let lit = &paths[0].1;
        let total = (WIDTH * HEIGHT) as usize;
        for (name, png) in &paths[1..] {
            let d = diff_pixels(lit, png)?;
            println!(
                "[ed3] {name} vs lit: {d}/{total} differing pixels ({:.2}%)",
                100.0 * d as f64 / total as f64
            );
            assert!(
                d > total / 100,
                "mode {name} must differ from Lit by more than 1% of pixels — \
                 an (almost) identical frame means the mode is inert again"
            );
        }
        Ok(())
    })
}

#[test]
#[ignore = "GPU + 3x terrain generation; run explicitly (see module docs)"]
fn ed3_amplitude_scale_changes_terrain() -> Result<()> {
    pollster::block_on(async {
        let out = out_root().join("amplitude");
        std::fs::create_dir_all(&out)?;
        let (device, queue, _info) = acquire_device().await?;
        let mut viewport = build_viewport(device.clone(), queue.clone()).await?;
        let texture = viewport.create_render_texture(WIDTH, HEIGHT)?;
        let world = World::new();
        let cam = station_camera();

        let mut paths = Vec::new();
        for scale in [0.5f32, 1.0, 2.0] {
            let state = generate_desert(6, scale)?;
            viewport.upload_terrain_chunks_raw(&state.get_gpu_chunks());
            for _ in 0..2 {
                viewport
                    .render(&texture, &cam, &world, None, None, None, false, false, 0)
                    .context("render failed")?;
            }
            let png = out.join(format!("scale_{scale:.2}.png"));
            viewport.capture_frame_png(&texture, &png)?;
            let (mean, _) = luma_stats(&png)?;
            println!(
                "[ed3] scale {scale}: mean luma {mean:.2} -> {}",
                png.display()
            );
            paths.push(png);
        }

        // Pairwise different — the exact check that proved the old slider
        // dead (T.2d Experiment F: three amplitudes, byte-identical frames,
        // DEGENERATE flag). If any pair collapses to near-identity, the
        // lever has gone inert again.
        let total = (WIDTH * HEIGHT) as usize;
        for i in 0..paths.len() {
            for j in (i + 1)..paths.len() {
                let d = diff_pixels(&paths[i], &paths[j])?;
                println!(
                    "[ed3] {} vs {}: {d}/{total} differing pixels ({:.2}%)",
                    paths[i].file_name().unwrap().to_string_lossy(),
                    paths[j].file_name().unwrap().to_string_lossy(),
                    100.0 * d as f64 / total as f64
                );
                assert!(
                    d > total / 20,
                    "amplitude scales must produce visibly different terrain \
                     (>5% of pixels); byte-similar frames = the T.2d Experiment F defect"
                );
            }
        }
        Ok(())
    })
}
