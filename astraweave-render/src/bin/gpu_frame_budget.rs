//! Median `draw_into` time for the step-8 scene.
//!
//! This does not decide the 11 ms budget. Lavapipe time is not that budget.
//! On a GTX 1660 Ti-class GPU the printed median is the measurement.
//!
//! ```text
//! cargo run -p astraweave-render --release --features gpu-frame-budget --bin gpu_frame_budget -- --scene heavy --debug-timestamps --glb "assets/The Complete KayKit Collection v4/KayKit Adventurers 2.0/Characters/gltf/Rogue.glb"
//! ```

use anyhow::Context;
use std::path::PathBuf;
use std::time::Instant;

use astraweave_camera::{Projection, RenderView};
use astraweave_render::gpu_particles::EmitterParams;
use astraweave_render::mesh_gltf::{load_gltf, GltfOptions};
use astraweave_render::{Instance, Renderer};
use glam::{Mat4, Vec3};

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const WARMUP: usize = 30;
const SAMPLES: usize = 120;

fn main() -> anyhow::Result<()> {
    let glb = glb_path();
    let heavy = scene_is_heavy();
    let debug_timestamps = debug_timestamps();
    if !glb.is_file() {
        anyhow::bail!(
            "textured GLB not found at {}. Pass --glb <path>. No frame time is invented.",
            glb.display()
        );
    }

    let mut renderer = pollster::block_on(Renderer::new_headless(WIDTH, HEIGHT))?;
    println!("adapter={}", renderer.gpu_adapter_name());
    println!("backend={}", renderer.gpu_backend());
    println!("features={:?}", renderer.gpu_features());
    if let Some(profiler) = renderer.gpu_profiler() {
        println!("timestamp_period_ns={}", profiler.timestamp_period_ns());
    }
    let timestamps = renderer.timestamps_supported();
    if timestamps
        && renderer
            .gpu_features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
    {
        renderer
            .device()
            .push_error_scope(wgpu::ErrorFilter::Validation);
        match astraweave_render::GpuProfiler::prove_readback(renderer.device(), renderer.queue()) {
            Ok(report) => {
                if let Some(err) = pollster::block_on(renderer.device().pop_error_scope()) {
                    anyhow::bail!("timestamp self-test validation: {err}\n{report}");
                }
                println!("timestamp_self_test=ok");
                println!("{report}");
            }
            Err(err) => {
                let scope = pollster::block_on(renderer.device().pop_error_scope());
                anyhow::bail!(
                    "timestamp self-test failed before the benchmark.\n{err}\nvalidation={scope:?}"
                );
            }
        }
    } else if timestamps {
        println!("timestamp_self_test=skipped missing TIMESTAMP_QUERY_INSIDE_ENCODERS");
    }
    let timestamps = renderer.timestamps_supported();
    if !timestamps {
        eprintln!(
            "GPU timestamps are unsupported on this adapter (need TIMESTAMP_QUERY and a positive timestamp period)."
        );
    }
    let hdri = renderer.bake_showcase_environment(astraweave_render::ibl::IblQuality::Medium)?;
    let meshes = load_gltf(&glb, &GltfOptions::default())?;
    if meshes.is_empty() {
        anyhow::bail!("{} produced no meshes", glb.display());
    }
    let instances = if heavy {
        heavy_instances()
    } else {
        vec![Instance::from_pos_scale_color(
            Vec3::ZERO,
            Vec3::ONE,
            [1.0, 1.0, 1.0, 1.0],
        )]
    };
    if instances.len() <= 1 {
        renderer.add_composite_model("budget-model", meshes, &instances);
    } else {
        renderer.add_composite_model("budget-0", meshes, &instances[..1]);
        for (i, instance) in instances.iter().enumerate().skip(1) {
            if !renderer.duplicate_model("budget-0", format!("budget-{i}"), instance.clone()) {
                anyhow::bail!("failed to place heavy model {i}");
            }
        }
    }
    renderer.set_shadows_enabled(true);
    add_heavy_terrain(&mut renderer)?;
    renderer.set_particle_emitter(Some(emitter()))?;
    // Cloud shadows stay at the renderer default. Volumetric fog and god rays
    // are not called. GPU indirect culling is not the draw path yet: each
    // model still binds group 3, so the CPU frustum remains the filter.
    let gpu_cull = "cpu_fallback";

    let device = renderer.device().clone();
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frame budget color"),
        size: wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: renderer.surface_format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = color.create_view(&wgpu::TextureViewDescriptor::default());
    let camera = fixed_camera();

    let frame = |renderer: &mut Renderer| -> anyhow::Result<(f64, f64)> {
        renderer.update_view(&camera);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frame budget"),
        });
        let encode_started = Instant::now();
        renderer.draw_into(&view, None, &mut encoder)?;
        let encode_ms = encode_started.elapsed().as_secs_f64() * 1_000.0;
        let frame_started = Instant::now();
        renderer.submit_timestamped(encoder.finish())?;
        if timestamps {
            renderer.request_gpu_profiler_readback();
        }
        astraweave_render::gpu_profiler::wait_for_gpu(&device, "frame gpu")
            .map_err(anyhow::Error::msg)?;
        if timestamps {
            if let Some(profiler) = renderer.gpu_profiler_mut() {
                profiler
                    .block_on_readback(&device)
                    .map_err(anyhow::Error::msg)?;
            }
        }
        if debug_timestamps {
            if let Some(profiler) = renderer.gpu_profiler() {
                println!("{}", profiler.submission_report());
            }
        }
        let cpu_frame_ms = frame_started.elapsed().as_secs_f64() * 1_000.0 + encode_ms;
        Ok((encode_ms, cpu_frame_ms))
    };

    for i in 0..WARMUP {
        if debug_timestamps {
            println!("warmup {}/{WARMUP}", i + 1);
        }
        frame(&mut renderer)?;
    }
    let mut encode = Vec::with_capacity(SAMPLES);
    let mut cpu_frame = Vec::with_capacity(SAMPLES);
    let mut gpu = Vec::new();
    let mut gpu_frame = Vec::new();
    let mut empty_passes = String::new();
    let mut gpu_error = None;
    for sample_index in 0..SAMPLES {
        if debug_timestamps {
            println!("sample {}/{SAMPLES}", sample_index + 1);
        }
        let (encode_ms, frame_ms) = frame(&mut renderer)?;
        encode.push(encode_ms);
        cpu_frame.push(frame_ms);
        if !timestamps {
            continue;
        }
        let profiler = renderer
            .gpu_profiler()
            .context("GPU profiler disappeared after a supported device")?;
        match profiler.validated_stamp() {
            Ok(summary) => {
                if debug_timestamps {
                    println!("sample={sample_index}\n{}", summary.report);
                }
                gpu.push(f64::from(summary.pass_sum_ms));
                if let Some(ms) = summary.frame_ms {
                    gpu_frame.push(f64::from(ms));
                }
                if empty_passes.is_empty() && !summary.empty.is_empty() {
                    empty_passes = summary.empty.join(",");
                }
            }
            Err(report) => {
                let trace = renderer
                    .gpu_profiler()
                    .map(|profiler| profiler.submission_report())
                    .unwrap_or_default();
                println!("sample={sample_index}\n{report}\n{trace}");
                gpu_error = Some(report);
                break;
            }
        }
    }

    let scene = if heavy { "heavy" } else { "light" };
    println!("scene={scene} resolution=1920x1080 warmup={WARMUP} samples={SAMPLES}");
    println!("hdri={}", hdri.display());
    println!("glb={}", glb.display());
    println!(
        "instances={} rendered_models={} particles=on shadows=on volumetrics=off gpu_cull={gpu_cull}",
        instances.len(),
        renderer.rendered_model_count()
    );
    println!("median_draw_into_ms={:.3}", median(&mut encode));
    println!("median_cpu_frame_ms={:.3}", median(&mut cpu_frame));
    if !timestamps {
        anyhow::bail!(
            "GPU timestamps are unsupported. median_cpu_frame_ms is the CPU fallback and is not the 11 ms GPU budget."
        );
    }
    if let Some(report) = gpu_error {
        anyhow::bail!(
            "GPU timestamp read failed. median_cpu_frame_ms is the CPU fallback.\n{report}"
        );
    }
    let gpu_median = median(&mut gpu);
    println!(
        "median_gpu_pass_sum_ms={:.3} gpu_samples={} empty={empty_passes}",
        gpu_median,
        gpu.len()
    );
    if !gpu_frame.is_empty() {
        println!(
            "median_gpu_frame_ms={:.3} frame_samples={}",
            median(&mut gpu_frame),
            gpu_frame.len()
        );
    }
    let frame_positive = gpu_frame.iter().any(|ms| *ms > 0.0);
    if gpu_median.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) && !frame_positive {
        anyhow::bail!("no positive GPU duration. median_cpu_frame_ms is the CPU fallback.");
    }
    println!("budget_11ms=not_decided");
    Ok(())
}

fn debug_timestamps() -> bool {
    std::env::args().any(|arg| arg == "--debug-timestamps" || arg == "--debug-timestamps=true")
}

fn scene_is_heavy() -> bool {
    std::env::args().any(|arg| arg == "--scene=heavy" || arg == "heavy")
        || std::env::args()
            .collect::<Vec<_>>()
            .windows(2)
            .any(|pair| pair[0] == "--scene" && pair[1] == "heavy")
}

fn heavy_instances() -> Vec<Instance> {
    let mut instances = Vec::with_capacity(64);
    for z in 0..8 {
        for x in 0..8 {
            instances.push(Instance::from_pos_scale_color(
                Vec3::new((x as f32 - 3.5) * 4.0, 0.0, (z as f32 - 3.5) * 4.0),
                Vec3::ONE,
                [1.0, 1.0, 1.0, 1.0],
            ));
        }
    }
    instances
}

#[cfg(feature = "terrain-splat-arrays")]
fn add_heavy_terrain(renderer: &mut Renderer) -> anyhow::Result<()> {
    if !scene_is_heavy() {
        return Ok(());
    }
    renderer.init_terrain_forward()?;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let n = 8u32;
    for z in 0..=n {
        for x in 0..=n {
            vertices.push(
                astraweave_render::terrain_material_manager::TerrainSplatVertex {
                    position: [
                        (x as f32 / n as f32) * 64.0 - 32.0,
                        0.0,
                        (z as f32 / n as f32) * 64.0 - 32.0,
                    ],
                    normal: [0.0, 1.0, 0.0],
                    uv: [x as f32 / n as f32, z as f32 / n as f32],
                },
            );
        }
    }
    for z in 0..n {
        for x in 0..n {
            let i = z * (n + 1) + x;
            indices.extend([i, i + n + 1, i + 1, i + 1, i + n + 1, i + n + 2]);
        }
    }
    let texel = [255u8, 0, 0, 255];
    let map = texel.repeat(16);
    let maps = vec![map; 8];
    let refs: Vec<&[u8]> = maps.iter().map(|bytes| bytes.as_slice()).collect();
    renderer.upload_terrain_chunk(1, &vertices, &indices, &refs, (4, 4))?;
    Ok(())
}

#[cfg(not(feature = "terrain-splat-arrays"))]
fn add_heavy_terrain(_renderer: &mut Renderer) -> anyhow::Result<()> {
    Ok(())
}

fn glb_path() -> PathBuf {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--glb" {
            if let Some(path) = args.next() {
                return PathBuf::from(path);
            }
        } else if let Some(path) = arg.strip_prefix("--glb=") {
            return PathBuf::from(path);
        }
    }
    PathBuf::from(
        "assets/The Complete KayKit Collection v4/KayKit Adventurers 2.0/Characters/gltf/Rogue.glb",
    )
}

fn emitter() -> EmitterParams {
    EmitterParams {
        position: [0.0, 1.0, 0.0, 1.0],
        velocity: [0.0, 1.0, 0.0, 0.0],
        emission_rate: 8.0,
        lifetime: 2.0,
        velocity_randomness: 0.25,
        delta_time: 1.0 / 60.0,
        gravity: [0.0, -9.8, 0.0, 0.0],
        particle_count: 32,
        max_particles: 256,
        random_seed: 7,
        _padding: 0,
    }
}

fn fixed_camera() -> RenderView {
    let eye = Vec3::new(4.0, 2.0, 4.0);
    let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y);
    let projection = Projection::perspective(
        60_f32.to_radians(),
        WIDTH as f32 / HEIGHT as f32,
        0.1,
        200.0,
    );
    RenderView::new(
        view,
        &projection,
        eye,
        Vec3::new(-1.0, -0.5, -1.0).normalize(),
    )
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) * 0.5
    } else {
        values[mid]
    }
}
