//! L.1 — lighting calibration & honesty: proofs.
//!
//! Three `#[ignore]`d GPU tests driving the editor's LIVE path
//! (`ViewportRenderer::render` + ED-2's `capture_frame_png`):
//!
//! * `l1_neutrality_stations` — captures the pinned stations at DEFAULT
//!   lighting state (no panel push), labelled via `L1_LABEL`. Run once at the
//!   pre-L.1 commit (`L1_LABEL=before`) and once after (`L1_LABEL=after`);
//!   the beat's headline criterion is that the frames differ by ≤1 LSB.
//!
//! * `l1_exposure_is_live` — renders exposure 0.5 / 1.35 / 3.0 through the
//!   panel-push path and asserts the frames differ pairwise (>5% of pixels).
//!   **Fails on pre-L.1 code by construction**: `TerrainLightingParams::
//!   exposure` was dropped by `set_lighting_params` (T2F_LIGHTING_RECON.md
//!   §1.2 row 10), so all three frames rendered byte-identical.
//!   Also asserts the honesty invariant: pushing the PINNED DEFAULTS
//!   (types.rs L.1 constants, exposure 1.35) yields a frame byte-identical
//!   to pushing nothing — the panel no longer lies about the delivered state.
//!
//! * `l1_late_push_lands` — pushes extreme-dark lighting params BEFORE
//!   `init_engine_adapter` (the live editor's frame-1 ordering, T2F §3) and
//!   asserts the params land after init. **Fails on pre-L.1 code by
//!   construction**: the push was silently dropped when the adapter was
//!   `None`, so the frame rendered at default brightness.
//!
//! ```text
//! cargo test -p aw_editor --profile release-fast --test l1_proof -- --ignored --nocapture <name>
//! ```

use anyhow::{Context, Result};
use astraweave_core::World;
use astraweave_terrain::world_archetypes::WorldArchetypeId;
use aw_editor_lib::terrain_integration::TerrainState;
use aw_editor_lib::viewport::{OrbitCamera, TerrainLightingParams, ViewportRenderer};
use glam::Vec3;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

const TIME_OF_DAY: f32 = 12.0;
const SEED: u64 = 12345;
const PRIMARY_BIOME: &str = "grassland";

/// T.2a's pinned desert focal (ground 36.3 m) and the recovered director
/// camera (T2D_CAMERA_LIGHT.md §10.2) — same anchors as t2df_stations.
const DESERT_FOCAL: [f32; 3] = [43.1, 36.3, -1961.8];
const REC_FOCAL: [f32; 3] = [-1029.9, 36.3, 254.7];

/// The pinned delivered-state lighting (L.1): what the renderer actually
/// holds at defaults. sun_dir here NEGATED+normalized reproduces the
/// terrain-upload hardcode bit-for-bit (same `Vec3::normalize` on the same
/// component values).
fn pinned_defaults() -> TerrainLightingParams {
    TerrainLightingParams {
        sun_dir: [0.5, 0.6, 0.4],
        sun_color: [1.0, 0.98, 0.9],
        sun_intensity: 1.0,
        ambient_color: [0.45, 0.50, 0.55],
        ambient_intensity: 0.35,
        exposure: 1.35,
    }
}

struct Station {
    focal: [f32; 3],
    distance: f32,
    yaw_deg: f32,
    pitch_deg: f32,
    width: u32,
    height: u32,
}

fn out_root() -> PathBuf {
    PathBuf::from(std::env::var("L1_OUT").unwrap_or_else(|_| "d:/tmp/l1_staging".to_string()))
}

fn label() -> String {
    std::env::var("L1_LABEL").unwrap_or_else(|_| "unlabelled".to_string())
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
        .context("no suitable wgpu adapter for the L.1 proof")?;
    let info = adapter.get_info();
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("l1-proof device"),
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
    println!("[l1] {archetype:?} radius {radius}: {n} chunks");
    Ok(state)
}

/// Init the adapter + biome pack, in the standard (adapter-first) order.
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
        // L.2: the init IBL bake runs on a worker thread — block so captures
        // see the deterministic post-bake state.
        adapter.wait_env_bake(std::time::Duration::from_secs(60));
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
    name: &str,
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
            .render(texture, &cam, &world, None, None, None, false, false, 0)
            .with_context(|| format!("render failed at {name}"))?;
    }
    let png = out.join(format!("{name}.png"));
    viewport.capture_frame_png(texture, &png)?;
    let (mean, _) = luma_stats(&png)?;
    println!("[l1] {name}: mean luma {mean:.2} -> {}", png.display());
    Ok(png)
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

fn diff_pixels(a: &std::path::Path, b: &std::path::Path) -> Result<(usize, u8)> {
    let ia = astraweave_asset::open_limited_image(a)?.to_rgba8();
    let ib = astraweave_asset::open_limited_image(b)?.to_rgba8();
    anyhow::ensure!(ia.dimensions() == ib.dimensions(), "dimension mismatch");
    let mut differing = 0usize;
    let mut max_delta = 0u8;
    for (x, y) in ia.pixels().zip(ib.pixels()) {
        if x.0 != y.0 {
            differing += 1;
            for c in 0..4 {
                max_delta = max_delta.max(x.0[c].abs_diff(y.0[c]));
            }
        }
    }
    Ok((differing, max_delta))
}

/// The neutrality stations: the director's boundary station + the T.2d.F
/// close station (radius-10 desert, matching t2df_stations), and the two
/// T.2f grassland stations (radius-6 Cont-Temp).
#[test]
#[ignore = "GPU + terrain generation; run explicitly (see module docs)"]
fn l1_neutrality_stations() -> Result<()> {
    pollster::block_on(async {
        let out = out_root().join(label());
        std::fs::create_dir_all(&out)?;
        let (device, queue, info) = acquire_device().await?;
        println!(
            "[l1] adapter: {} · {:?} · driver {}",
            info.name, info.backend, info.driver_info
        );

        // Desert (radius 10 — the world the director's frames were made against).
        let mut viewport = build_viewport(device.clone(), queue.clone()).await?;
        let state = generate_world(WorldArchetypeId::Desert, 10)?;
        viewport.upload_terrain_chunks_raw(&state.get_gpu_chunks());
        let mut textures = HashMap::new();
        shoot(
            &mut viewport,
            &mut textures,
            &Station {
                focal: REC_FOCAL,
                distance: 529.6,
                yaw_deg: 45.5,
                pitch_deg: 45.6,
                width: 962,
                height: 501,
            },
            &out,
            "desert_boundary_y414",
        )?;
        shoot(
            &mut viewport,
            &mut textures,
            &Station {
                focal: DESERT_FOCAL,
                distance: 20.0,
                yaw_deg: 45.0,
                pitch_deg: 55.0,
                width: 1024,
                height: 768,
            },
            &out,
            "desert_close_20m",
        )?;
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
            shoot(
                &mut viewport,
                &mut textures,
                &Station {
                    focal: [0.0, ground, 0.0],
                    distance: dist,
                    yaw_deg: 45.0,
                    pitch_deg: pitch,
                    width: 1024,
                    height: 768,
                },
                &out,
                name,
            )?;
        }
        println!("[l1] label '{}' -> {}", label(), out.display());
        Ok(())
    })
}

#[test]
#[ignore = "GPU + terrain generation; run explicitly (see module docs)"]
fn l1_exposure_is_live() -> Result<()> {
    pollster::block_on(async {
        let out = out_root().join("exposure");
        std::fs::create_dir_all(&out)?;
        let (device, queue, _info) = acquire_device().await?;
        let mut viewport = build_viewport(device.clone(), queue.clone()).await?;
        let state = generate_world(WorldArchetypeId::Desert, 6)?;
        viewport.upload_terrain_chunks_raw(&state.get_gpu_chunks());
        let mut textures = HashMap::new();
        let station = Station {
            focal: DESERT_FOCAL,
            distance: 20.0,
            yaw_deg: 45.0,
            pitch_deg: 55.0,
            width: 1024,
            height: 768,
        };

        // Honesty invariant first: the frame with NO push at all…
        let no_push = shoot(&mut viewport, &mut textures, &station, &out, "no_push")?;
        // …must equal the frame after pushing the PINNED DEFAULTS (this is
        // the "panel displays the truth" contract; identical pre- and
        // post-L.1 only if the pinned constants really are the delivered
        // state, direction included).
        viewport.set_lighting_params(pinned_defaults());
        let defaults = shoot(
            &mut viewport,
            &mut textures,
            &station,
            &out,
            "push_defaults",
        )?;
        let (d, max_lsb) = diff_pixels(&no_push, &defaults)?;
        println!("[l1] push_defaults vs no_push: {d} differing pixels, max LSB delta {max_lsb}");
        assert!(
            max_lsb <= 1,
            "pushing the pinned defaults must be visually neutral (≤1 LSB); \
             got {d} pixels differing with max channel delta {max_lsb}"
        );

        // Control proof: three exposures must produce different frames.
        // Pre-L.1, params.exposure was dropped → all three byte-identical.
        let mut frames = Vec::new();
        for exposure in [0.5f32, 1.35, 3.0] {
            viewport.set_lighting_params(TerrainLightingParams {
                exposure,
                ..pinned_defaults()
            });
            let png = shoot(
                &mut viewport,
                &mut textures,
                &station,
                &out,
                &format!("exposure_{exposure:.2}"),
            )?;
            frames.push(png);
        }
        let total = (station.width * station.height) as usize;
        for i in 0..frames.len() {
            for j in (i + 1)..frames.len() {
                let (d, _) = diff_pixels(&frames[i], &frames[j])?;
                println!(
                    "[l1] {} vs {}: {d}/{total} differing ({:.2}%)",
                    frames[i].file_name().unwrap().to_string_lossy(),
                    frames[j].file_name().unwrap().to_string_lossy(),
                    100.0 * d as f64 / total as f64
                );
                assert!(
                    d > total / 20,
                    "exposure values must produce visibly different frames (>5%); \
                     near-identical frames = the inert-slider defect (T2F §1.2 row 10)"
                );
            }
        }
        Ok(())
    })
}

#[test]
#[ignore = "GPU + terrain generation; run explicitly (see module docs)"]
fn l1_late_push_lands() -> Result<()> {
    pollster::block_on(async {
        let out = out_root().join("late_push");
        std::fs::create_dir_all(&out)?;
        let (device, queue, _info) = acquire_device().await?;
        let assets = find_assets_dir().context("assets dir not found")?;
        let biome_dir = assets.join("materials").join("biomes");

        // The live editor's frame-1 ordering (T2F §3): the panel push happens
        // BEFORE the async engine adapter exists.
        let mut viewport = ViewportRenderer::new(device.clone(), queue.clone())
            .context("ViewportRenderer::new failed")?;
        // L.2 recalibration: terrain is now IBL-lit, and the lighting push
        // cannot scale the IBL term — a landed sun-0.15 push renders ~155
        // (was ~46 pre-L.2) vs ~178 dropped, too thin a margin. Push a dark
        // exposure too (exposure IS park-and-delivered — that's the point of
        // this test), which widens landed (~87) vs dropped (~178).
        let dark = TerrainLightingParams {
            sun_intensity: 0.15,
            ambient_intensity: 0.0,
            exposure: 0.5,
            ..pinned_defaults()
        };
        viewport.set_lighting_params(dark); // adapter is None here
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
            // L.2: deterministic post-bake state (worker-thread init bake).
            adapter.wait_env_bake(std::time::Duration::from_secs(60));
        }
        let state = generate_world(WorldArchetypeId::Desert, 6)?;
        viewport.upload_terrain_chunks_raw(&state.get_gpu_chunks());

        let mut textures = HashMap::new();
        let station = Station {
            focal: DESERT_FOCAL,
            distance: 20.0,
            yaw_deg: 45.0,
            pitch_deg: 55.0,
            width: 1024,
            height: 768,
        };
        let png = shoot(&mut viewport, &mut textures, &station, &out, "late_push")?;
        let (mean, _) = luma_stats(&png)?;

        // NOTE: the terrain upload overwrites ambient (engine_adapter.rs
        // terrain-upload block), but NOT sun colour/intensity/exposure — so a
        // landed dark push must render far darker than the delivered default
        // (~178 under the L.2 IBL fill; was ~112 pre-L.2). Pre-L.1 the push
        // was dropped → default brightness → this assertion fails.
        assert!(
            mean < 140.0,
            "late (pre-init) lighting push must land after adapter init; \
             mean luma {mean:.2} indicates the push was dropped (T2F §3 race)"
        );
        Ok(())
    })
}
