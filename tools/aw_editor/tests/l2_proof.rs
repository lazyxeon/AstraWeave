//! L.2 — IBL for editor terrain: A/B capture harness.
//!
//! One `#[ignore]`d GPU test driving the editor's LIVE path
//! (`ViewportRenderer::render` + ED-2's `capture_frame_png`), cloned from
//! `l1_proof.rs` (same four stations, same worlds, same two-frame settle).
//! Each station is captured twice per run:
//!
//! * `<name>.png` — lit frame (shading_mode 0);
//! * `<name>_normals.png` — ED-3 world-space-normals debug (viewport
//!   shading_mode 3 → scene-env debug_mode 2), which bypasses lighting for
//!   terrain pixels. Terrain pixels must be bit-identical before/after L.2;
//!   any differing pixel is sky — which is exactly the sky/ground mask the
//!   A/B metrics use. (Viewport mode 2 is WIREFRAME, not normals — and it
//!   silently falls back to lit fill on a device without POLYGON_MODE_LINE.)
//!
//! ```text
//! L2_LABEL=<label> cargo test -p aw_editor --profile release-fast \
//!     --test l2_proof -- --ignored --nocapture l2_ab_stations
//! ```
//!
//! Output: `$L2_OUT/<label>/` (default `d:/tmp/l2_staging/<label>/`).

use anyhow::{Context, Result};
use astraweave_core::World;
use astraweave_terrain::world_archetypes::WorldArchetypeId;
use aw_editor_lib::terrain_integration::TerrainState;
use aw_editor_lib::viewport::{OrbitCamera, ViewportRenderer};
use glam::Vec3;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

const TIME_OF_DAY: f32 = 12.0;
const SEED: u64 = 12345;
const PRIMARY_BIOME: &str = "grassland";

/// Same anchors as `l1_proof.rs` / `t2df_stations.rs`.
const DESERT_FOCAL: [f32; 3] = [43.1, 36.3, -1961.8];
const REC_FOCAL: [f32; 3] = [-1029.9, 36.3, 254.7];

struct Station {
    name: &'static str,
    focal: [f32; 3],
    distance: f32,
    yaw_deg: f32,
    pitch_deg: f32,
    width: u32,
    height: u32,
}

fn out_root() -> PathBuf {
    PathBuf::from(std::env::var("L2_OUT").unwrap_or_else(|_| "d:/tmp/l2_staging".to_string()))
}

fn label() -> String {
    std::env::var("L2_LABEL").unwrap_or_else(|_| "unlabelled".to_string())
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
        .context("no suitable wgpu adapter for the L.2 proof")?;
    let info = adapter.get_info();
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("l2-proof device"),
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

fn generate_world(archetype: WorldArchetypeId, radius: i32) -> Result<TerrainState> {
    let mut state = TerrainState::new();
    state.configure(SEED, PRIMARY_BIOME);
    state.set_noise_params(6, 2.0, 0.5, 50.0);
    state.set_world_archetype(archetype.default_archetype());
    let n = state
        .generate_terrain(radius)
        .context("generate_terrain failed")?;
    println!("[l2] {archetype:?} radius {radius}: {n} chunks");
    Ok(state)
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
        // L.2: the init bake runs on a worker thread — block here so every
        // capture sees the deterministic post-bake state.
        let baked = adapter.wait_env_bake(std::time::Duration::from_secs(60));
        let res = adapter.renderer().ibl_resources.as_ref();
        println!(
            "[l2] ibl baked: {baked} (avg_luminance {:?})",
            res.and_then(|r| r.avg_luminance)
        );
    }
    Ok(viewport)
}

fn station_camera(s: &Station) -> OrbitCamera {
    let mut cam = OrbitCamera::new(
        Vec3::from_array(s.focal),
        s.distance,
        s.yaw_deg.to_radians(),
        s.pitch_deg.to_radians(),
    );
    cam.set_aspect(s.width as f32, s.height as f32);
    cam
}

fn shoot(
    viewport: &mut ViewportRenderer,
    textures: &mut HashMap<(u32, u32), wgpu::Texture>,
    s: &Station,
    out: &std::path::Path,
    shading_mode: u32,
    suffix: &str,
) -> Result<PathBuf> {
    let texture = match textures.entry((s.width, s.height)) {
        std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
        std::collections::hash_map::Entry::Vacant(v) => v.insert(
            viewport
                .create_render_texture(s.width, s.height)
                .context("create_render_texture failed")?,
        ),
    };
    let cam = station_camera(s);
    let world = World::new();
    for _ in 0..2 {
        viewport
            .render(
                texture,
                &cam,
                &world,
                None,
                None,
                None,
                false,
                false,
                shading_mode,
            )
            .with_context(|| format!("render failed at {}{suffix}", s.name))?;
    }
    let png = out.join(format!("{}{suffix}.png", s.name));
    viewport.capture_frame_png(texture, &png)?;
    let (mean, sd) = luma_stats(&png)?;
    println!(
        "[l2] {}{suffix}: mean luma {mean:.2} sd {sd:.2} -> {}",
        s.name,
        png.display()
    );
    Ok(png)
}

fn luma_stats(path: &std::path::Path) -> Result<(f64, f64)> {
    let img = astraweave_asset::open_limited_image(path)?.to_rgba8();
    let n = (img.width() * img.height()) as f64;
    let mut sum = 0.0f64;
    let mut sum_sq = 0.0f64;
    for p in img.pixels() {
        let l = 0.2126 * p.0[0] as f64 + 0.7152 * p.0[1] as f64 + 0.0722 * p.0[2] as f64;
        sum += l;
        sum_sq += l * l;
    }
    let mean = sum / n;
    Ok((mean, (sum_sq / n - mean * mean).max(0.0).sqrt()))
}

/// The four L.1 stations, captured lit + normals-debug.
#[test]
#[ignore = "GPU + terrain generation; run explicitly (see module docs)"]
fn l2_ab_stations() -> Result<()> {
    pollster::block_on(async {
        let out = out_root().join(label());
        std::fs::create_dir_all(&out)?;
        let (device, queue, info) = acquire_device().await?;
        println!(
            "[l2] adapter: {} · {:?} · driver {}",
            info.name, info.backend, info.driver_info
        );

        // Desert (radius 10 — the world the director's frames were made against).
        let mut viewport = build_viewport(device.clone(), queue.clone()).await?;
        let state = generate_world(WorldArchetypeId::Desert, 10)?;
        viewport.upload_terrain_chunks_raw(&state.get_gpu_chunks());
        let mut textures = HashMap::new();
        for s in [
            Station {
                name: "desert_boundary_y414",
                focal: REC_FOCAL,
                distance: 529.6,
                yaw_deg: 45.5,
                pitch_deg: 45.6,
                width: 962,
                height: 501,
            },
            Station {
                name: "desert_close_20m",
                focal: DESERT_FOCAL,
                distance: 20.0,
                yaw_deg: 45.0,
                pitch_deg: 55.0,
                width: 1024,
                height: 768,
            },
        ] {
            shoot(&mut viewport, &mut textures, &s, &out, 0, "")?;
            shoot(&mut viewport, &mut textures, &s, &out, 3, "_normals")?;
        }
        drop(viewport);

        // Cont-Temp grassland (radius 6 — the T.2f stations).
        let mut viewport = build_viewport(device.clone(), queue.clone()).await?;
        let state = generate_world(WorldArchetypeId::ContinentalTemperate, 6)?;
        let ground = state
            .get_height_at(0.0, 0.0)
            .context("no grassland height at origin")?;
        viewport.upload_terrain_chunks_raw(&state.get_gpu_chunks());
        let mut textures = HashMap::new();
        for (name, dist, pitch) in [
            ("grass_close_20m", 20.0, 55.0),
            ("grass_mid_47m", 46.7, 40.0),
        ] {
            let s = Station {
                name,
                focal: [0.0, ground, 0.0],
                distance: dist,
                yaw_deg: 45.0,
                pitch_deg: pitch,
                width: 1024,
                height: 768,
            };
            shoot(&mut viewport, &mut textures, &s, &out, 0, "")?;
            shoot(&mut viewport, &mut textures, &s, &out, 3, "_normals")?;
        }
        println!("[l2] label '{}' -> {}", label(), out.display());
        Ok(())
    })
}
