//! Median `draw_into` time for the step-8 scene.
//!
//! This does not decide the 11 ms budget. Lavapipe time is not that budget.
//! On a GTX 1660 Ti-class GPU the printed median is the measurement.
//!
//! ```text
//! cargo run -p astraweave-render --release --features gpu-frame-budget --bin gpu_frame_budget -- --glb "assets/The Complete KayKit Collection v4/KayKit Adventurers 2.0/Characters/gltf/Rogue.glb"
//! ```

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
    if !glb.is_file() {
        anyhow::bail!(
            "textured GLB not found at {}. Pass --glb <path>. No frame time is invented.",
            glb.display()
        );
    }

    let mut renderer = pollster::block_on(Renderer::new_headless(WIDTH, HEIGHT))?;
    let hdri = renderer.bake_showcase_environment(astraweave_render::ibl::IblQuality::Medium)?;
    let meshes = load_gltf(&glb, &GltfOptions::default())?;
    if meshes.is_empty() {
        anyhow::bail!("{} produced no meshes", glb.display());
    }
    let instance = Instance::from_pos_scale_color(Vec3::ZERO, Vec3::ONE, [1.0, 1.0, 1.0, 1.0]);
    renderer.add_composite_model("budget-model", meshes, &[instance]);
    renderer.set_particle_emitter(Some(emitter()))?;
    // Cloud shadows stay at the renderer default. Volumetric fog and god rays
    // are not called. GPU indirect culling is not the draw path yet: each
    // model still binds group 3, so the CPU frustum remains the filter.
    let gpu_cull = "cpu_fallback";

    let device = renderer.device().clone();
    let queue = renderer.queue().clone();
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

    let frame = |renderer: &mut Renderer| -> anyhow::Result<f64> {
        renderer.update_view(&camera);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frame budget"),
        });
        let started = Instant::now();
        renderer.draw_into(&view, None, &mut encoder)?;
        let cpu_ms = started.elapsed().as_secs_f64() * 1_000.0;
        queue.submit(std::iter::once(encoder.finish()));
        renderer.request_gpu_profiler_readback();
        let _ = device.poll(wgpu::MaintainBase::Wait);
        Ok(cpu_ms)
    };

    for _ in 0..WARMUP {
        frame(&mut renderer)?;
    }
    let mut cpu = Vec::with_capacity(SAMPLES);
    let mut gpu = Vec::new();
    for _ in 0..SAMPLES {
        cpu.push(frame(&mut renderer)?);
        if let Some(profiler) = renderer.gpu_profiler() {
            if !profiler.results().is_empty() {
                gpu.push(profiler.total_gpu_ms() as f64);
            }
        }
    }

    println!("scene=1920x1080 warmup={WARMUP} samples={SAMPLES}");
    println!("hdri={}", hdri.display());
    println!("glb={}", glb.display());
    println!("particles=on cloud_shadows=default volumetrics=off gpu_cull={gpu_cull}");
    println!("median_draw_into_ms={:.3}", median(&mut cpu));
    if gpu.is_empty() {
        println!("median_gpu_pass_sum_ms=unavailable");
    } else {
        println!(
            "median_gpu_pass_sum_ms={:.3} gpu_samples={}",
            median(&mut gpu),
            gpu.len()
        );
    }
    println!("budget_11ms=not_decided");
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
