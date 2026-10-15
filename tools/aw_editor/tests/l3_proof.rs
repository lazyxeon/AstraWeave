//! L.3 — terrain CSM (cast + receive): A/B capture harness + min-spec perf.
//!
//! Cloned from `l2_proof.rs` (same four stations, same worlds, same two-frame
//! settle, same live path: `ViewportRenderer::render` + ED-2
//! `capture_frame_png`). L.3 additions:
//!
//! * every station is shot at TWO sun angles. In the editor terrain scene the
//!   sun is NOT TimeOfDay — the terrain-upload block pins the World panel's
//!   default override (`set_light_direction_override`, engine_adapter.rs;
//!   bit-identity with `DEFAULT_SUN_DIR` pinned by types.rs' L.1 tests), so
//!   `set_time_of_day` is inert here (proven: t=12.0 vs t=16.97 legs were
//!   byte-identical). The two legs therefore go through the honest L.1 panel
//!   path, `set_lighting_params`, varying ONLY `sun_dir`:
//!   - default leg: `DEFAULT_SUN_DIR` — elevation 43.2°, azimuth 51.3° (the
//!     angle every T-series/L-series frame was shot under);
//!   - rake leg: elevation 20.0°, same 51.3° azimuth — shadow legibility;
//! * a `sun.txt` sidecar records the delivered sun elevation/azimuth and
//!   light_dir for both legs (shadow A/Bs are meaningless without it);
//! * `l3_perf_stations` measures frame wall time (render + forced GPU sync —
//!   TIMESTAMP_QUERY hangs this driver, T2DF_OUTCOME §5) at the distance-heavy
//!   y414 boundary framing (1920×1080) AND the desert 20 m close-up (1024×768),
//!   60 warm-up + 300 timed frames, median/p10/p90 — caster cost differs by
//!   framing.
//!
//! ```text
//! L3_LABEL=<label> cargo test -p aw_editor --profile release-fast \
//!     --test l3_proof -- --ignored --nocapture l3_ab_stations
//! L3_LABEL=<label> cargo test -p aw_editor --profile release-fast \
//!     --test l3_proof -- --ignored --nocapture l3_perf_stations
//! ```
//!
//! Output: `$L3_OUT/<label>/` (default `d:/tmp/l3_staging/<label>/`).

use anyhow::{Context, Result};
use astraweave_core::World;
use astraweave_terrain::world_archetypes::WorldArchetypeId;
use aw_editor_lib::terrain_integration::TerrainState;
use aw_editor_lib::viewport::types::TerrainLightingParams;
use aw_editor_lib::viewport::{OrbitCamera, ViewportRenderer};
use glam::Vec3;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

const NOON: f32 = 12.0;
/// Raking sun for shadow legibility: elevation 20.0°, azimuth 51.3° — the
/// SAME azimuth as the pinned default sun (43.2° elevation), so the rake leg
/// changes only how low the light comes in, not which way it points.
/// `sun_dir` points TO the sun: (sin az·cos e, sin e, cos az·cos e).
const RAKE_SUN_DIR: [f32; 3] = [0.7335, 0.3420, 0.5870];
const SEED: u64 = 12345;
const PRIMARY_BIOME: &str = "grassland";

/// Same anchors as `l1_proof.rs` / `l2_proof.rs` / `t2df_stations.rs`.
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
    PathBuf::from(std::env::var("L3_OUT").unwrap_or_else(|_| "d:/tmp/l3_staging".to_string()))
}

fn label() -> String {
    std::env::var("L3_LABEL").unwrap_or_else(|_| "unlabelled".to_string())
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

/// The delivered light direction for a panel `sun_dir`, through the same math
/// as `set_lighting_params` (negate, then normalize) — not a reimplementation.
fn sun_line(leg: &str, sun_dir: [f32; 3]) -> String {
    let sun = Vec3::from_array(sun_dir).normalize();
    let light = (-sun).normalize();
    let elevation = sun.y.asin().to_degrees();
    let azimuth = sun.x.atan2(sun.z).to_degrees();
    format!(
        "{leg}: sun elevation {elevation:.1} deg, azimuth {azimuth:.1} deg, \
         light_dir ({:.4}, {:.4}, {:.4})",
        light.x, light.y, light.z
    )
}

/// The rake leg's panel params: ONLY `sun_dir` differs from the pinned
/// defaults (sun color/intensity, ambient, exposure all unchanged).
fn rake_params() -> TerrainLightingParams {
    TerrainLightingParams {
        sun_dir: RAKE_SUN_DIR,
        ..TerrainLightingParams::default()
    }
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
        .context("no suitable wgpu adapter for the L.3 proof")?;
    let info = adapter.get_info();
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("l3-proof device"),
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
    println!("[l3] {archetype:?} radius {radius}: {n} chunks");
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
        adapter.set_time_of_day(NOON);
        adapter.set_biome_pack(Some(biome_dir));
        // L.2: the init bake runs on a worker thread — block here so every
        // capture sees the deterministic post-bake state.
        let baked = adapter.wait_env_bake(std::time::Duration::from_secs(60));
        println!("[l3] ibl baked: {baked}");
    }
    Ok(viewport)
}

/// `L3_SHADOWS=off` disables shadows through the live sentinel path
/// (`set_shadows_enabled(false)` → `extras.x = -1.0`), producing the L.3
/// toggle-proof leg. Must run AFTER terrain upload — the upload auto-applies
/// the `EditorTerrain` preset, which owns the shadow enable.
fn apply_shadow_toggle(viewport: &mut ViewportRenderer) -> Result<()> {
    if std::env::var("L3_SHADOWS").as_deref() == Ok("off") {
        viewport
            .engine_adapter_mut()
            .context("engine adapter missing")?
            .renderer_mut()
            .set_shadows_enabled(false);
        println!("[l3] shadows DISABLED via sentinel (L3_SHADOWS=off)");
    }
    Ok(())
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
    // L.3: per-cascade caster culling counts for this frame ((0, 0) when the
    // shadow passes were skipped — e.g. the L3_SHADOWS=off toggle leg).
    let stats = viewport
        .engine_adapter_mut()
        .context("engine adapter missing")?
        .renderer()
        .terrain_shadow_stats();
    // Destructured, not indexed: the arity is compiler-enforced, so adding a
    // cascade cannot silently leave one out of the report.
    let [c0, c1, c2, c3] = stats;
    println!(
        "[l3] {}{suffix}: mean luma {mean:.2} sd {sd:.2} casters c0 {}/{} c1 {}/{} c2 {}/{} c3 {}/{} -> {}",
        s.name,
        c0.0, c0.1, c1.0, c1.1, c2.0, c2.1, c3.0, c3.1,
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

fn desert_stations() -> [Station; 2] {
    [
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
    ]
}

/// The four L.1/L.2 stations, each captured at noon (lit + ED-3 normals debug)
/// and under the raking sun (lit; the normals debug bypasses lighting, so one
/// frame per framing is the mask for both sun legs).
#[test]
#[ignore = "GPU + terrain generation; run explicitly (see module docs)"]
fn l3_ab_stations() -> Result<()> {
    pollster::block_on(async {
        let out = out_root().join(label());
        std::fs::create_dir_all(&out)?;
        let sun_default = sun_line(
            "default (panel-pinned)",
            TerrainLightingParams::default().sun_dir,
        );
        let sun_rake = sun_line("rake", RAKE_SUN_DIR);
        std::fs::write(out.join("sun.txt"), format!("{sun_default}\n{sun_rake}\n"))?;
        println!("[l3] {sun_default}");
        println!("[l3] {sun_rake}");
        let (device, queue, info) = acquire_device().await?;
        println!(
            "[l3] adapter: {} · {:?} · driver {}",
            info.name, info.backend, info.driver_info
        );

        // Desert (radius 10 — the world the director's frames were made against).
        let mut viewport = build_viewport(device.clone(), queue.clone()).await?;
        let state = generate_world(WorldArchetypeId::Desert, 10)?;
        viewport.upload_terrain_chunks_raw(&state.get_gpu_chunks());
        apply_shadow_toggle(&mut viewport)?;
        let mut textures = HashMap::new();
        for s in desert_stations() {
            shoot(&mut viewport, &mut textures, &s, &out, 0, "")?;
            shoot(&mut viewport, &mut textures, &s, &out, 3, "_normals")?;
        }
        viewport
            .engine_adapter_mut()
            .context("engine adapter missing")?
            .set_lighting_params(&rake_params());
        for s in desert_stations() {
            shoot(&mut viewport, &mut textures, &s, &out, 0, "_rake")?;
        }
        drop(viewport);

        // Cont-Temp grassland (radius 6 — the T.2f stations).
        let mut viewport = build_viewport(device.clone(), queue.clone()).await?;
        let state = generate_world(WorldArchetypeId::ContinentalTemperate, 6)?;
        let ground = state
            .get_height_at(0.0, 0.0)
            .context("no grassland height at origin")?;
        viewport.upload_terrain_chunks_raw(&state.get_gpu_chunks());
        apply_shadow_toggle(&mut viewport)?;
        let mut textures = HashMap::new();
        let grass = |name: &'static str, dist: f32, pitch: f32| Station {
            name,
            focal: [0.0, ground, 0.0],
            distance: dist,
            yaw_deg: 45.0,
            pitch_deg: pitch,
            width: 1024,
            height: 768,
        };
        for s in [
            grass("grass_close_20m", 20.0, 55.0),
            grass("grass_mid_47m", 46.7, 40.0),
        ] {
            shoot(&mut viewport, &mut textures, &s, &out, 0, "")?;
            shoot(&mut viewport, &mut textures, &s, &out, 3, "_normals")?;
        }
        viewport
            .engine_adapter_mut()
            .context("engine adapter missing")?
            .set_lighting_params(&rake_params());
        for s in [
            grass("grass_close_20m", 20.0, 55.0),
            grass("grass_mid_47m", 46.7, 40.0),
        ] {
            shoot(&mut viewport, &mut textures, &s, &out, 0, "_rake")?;
        }
        println!("[l3] label '{}' -> {}", label(), out.display());
        Ok(())
    })
}

/// Min-spec frame timing at the two shadow-relevant framings (T2DF_OUTCOME §5
/// methodology: wall time per frame with a forced GPU sync; TIMESTAMP_QUERY
/// hangs this driver). Noon sun, for continuity with the T.2d.F / L.2 numbers.
#[test]
#[ignore = "GPU + radius-10 terrain generation; run explicitly (see module docs)"]
fn l3_perf_stations() -> Result<()> {
    pollster::block_on(async {
        let leg = label();
        let (device, queue, info) = acquire_device().await?;
        println!(
            "[l3-perf] adapter: {} · {:?} · driver {}",
            info.name, info.backend, info.driver_info
        );
        let mut viewport = build_viewport(device.clone(), queue.clone()).await?;
        let state = generate_world(WorldArchetypeId::Desert, 10)?;
        viewport.upload_terrain_chunks_raw(&state.get_gpu_chunks());
        apply_shadow_toggle(&mut viewport)?;

        // Distance-heavy framing at 1080p (the T.2d.F/L.2 perf framing) and
        // the 20 m close-up at its capture resolution — caster cost differs.
        let perf_stations = [
            Station {
                name: "perf_boundary_y414",
                focal: REC_FOCAL,
                distance: 529.6,
                yaw_deg: 45.5,
                pitch_deg: 45.6,
                width: 1920,
                height: 1080,
            },
            Station {
                name: "perf_desert_close_20m",
                focal: DESERT_FOCAL,
                distance: 20.0,
                yaw_deg: 45.0,
                pitch_deg: 55.0,
                width: 1024,
                height: 768,
            },
        ];

        const WARMUP: usize = 60;
        const TIMED: usize = 300;
        fn stats(mut v: Vec<f64>) -> (f64, f64, f64, usize) {
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let n = v.len();
            (v[n / 2], v[n / 10], v[n - 1 - n / 10], n)
        }

        let world = World::new();
        for s in perf_stations {
            let texture = viewport.create_render_texture(s.width, s.height)?;
            let cam = station_camera(&s);
            let mut wall_ms: Vec<f64> = Vec::with_capacity(TIMED);
            for i in 0..(WARMUP + TIMED) {
                let t0 = std::time::Instant::now();
                viewport
                    .render(&texture, &cam, &world, None, None, None, false, false, 0)
                    .context("render failed")?;
                device
                    .poll(wgpu::MaintainBase::Wait)
                    .ok()
                    .context("device poll failed")?;
                let dt = t0.elapsed().as_secs_f64() * 1000.0;
                if i % 120 == 0 {
                    println!(
                        "[l3-perf] {} frame {i}/{} ({dt:.1} ms)",
                        s.name,
                        WARMUP + TIMED
                    );
                }
                if i >= WARMUP {
                    wall_ms.push(dt);
                }
            }
            let (med, p10, p90, n) = stats(wall_ms);
            println!(
                "[l3-perf] leg '{leg}' {} at {}x{}: median {med:.3} ms  p10 {p10:.3}  p90 {p90:.3}  (n={n})",
                s.name, s.width, s.height
            );
        }
        Ok(())
    })
}
