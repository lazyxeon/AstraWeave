//! First production-solver baselines for astraweave-fluids (WI-6, F.1).
//!
//! Unlike `fluids_adversarial` (which predominantly benchmarks bench-local
//! mock re-implementations — see the F.0 audit §1.4-D), every benchmark in
//! this file measures PRODUCTION crate code:
//!
//! - `voxel_grid_simulate`: `WaterVolumeGrid::simulate` at 32³/64³/128³ with
//!   a half-full basin — the first voxel-sim numbers ever recorded; these
//!   gate F.3 scope sizing.
//! - `fluid_system_step`: full `FluidSystem::step` GPU dispatch + submit +
//!   wait at 10k/20k/50k particles (demo-canonical parameters). Skipped
//!   (with a loud message) when no GPU adapter is available.
//!
//! Before the criterion groups run, `print_gpu_pass_breakdown` captures a
//! per-pass GPU timestamp table via `FluidSystem::read_gpu_timings` for the
//! same particle counts; record its output in MASTER_BENCHMARK_REPORT.md.

use criterion::{criterion_group, BenchmarkId, Criterion};
use glam::{IVec3, UVec3, Vec3};

use astraweave_fluids::{FluidSystem, WaterVolumeGrid};

const DT: f32 = 1.0 / 60.0;

// ---------------------------------------------------------------------------
// Voxel grid
// ---------------------------------------------------------------------------

/// Half-full basin: stone floor, water filling the lower half of the volume.
fn make_half_full_basin(n: u32) -> WaterVolumeGrid {
    let mut grid = WaterVolumeGrid::new(UVec3::new(n, n, n), 1.0, Vec3::ZERO);
    for x in 0..n as i32 {
        for z in 0..n as i32 {
            grid.set_material(IVec3::new(x, 0, z), astraweave_fluids::MaterialType::Stone);
            for y in 1..=(n as i32 / 2) {
                grid.set_level(IVec3::new(x, y, z), 1.0);
            }
        }
    }
    grid
}

fn bench_voxel_grid(c: &mut Criterion) {
    let mut group = c.benchmark_group("voxel_grid_simulate");
    for n in [32u32, 64, 128] {
        group.bench_with_input(BenchmarkId::new("half_full_basin", n), &n, |b, &n| {
            let mut grid = make_half_full_basin(n);
            b.iter(|| grid.simulate(std::hint::black_box(DT)));
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// GPU FluidSystem step
// ---------------------------------------------------------------------------

fn try_create_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    if std::env::var("SKIP_GPU_TESTS").is_ok() {
        return None;
    }
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .ok()?;
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("fluid baselines bench device"),
        required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            | wgpu::Features::TIMESTAMP_QUERY,
        required_limits: wgpu::Limits::default(),
        memory_hints: wgpu::MemoryHints::default(),
        trace: wgpu::Trace::Off,
    }))
    .ok()
}

/// Demo-canonical parameters (the only production-exercised configuration).
fn apply_demo_params(system: &mut FluidSystem) {
    system.smoothing_radius = 0.5;
    system.target_density = 1.0;
    system.viscosity = 0.01;
    system.surface_tension = 0.1;
    system.gravity = -9.81;
    system.cell_size = 1.2;
    system.grid_width = 64;
    system.grid_height = 64;
    system.grid_depth = 64;
}

fn run_step(system: &mut FluidSystem, device: &wgpu::Device, queue: &wgpu::Queue) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("bench"),
    });
    system.step(device, &mut encoder, queue, DT);
    queue.submit([encoder.finish()]);
}

fn bench_fluid_system_step(c: &mut Criterion) {
    let Some((device, queue)) = try_create_device() else {
        eprintln!("SKIPPED: fluid_system_step bench group (no GPU adapter / SKIP_GPU_TESTS)");
        return;
    };
    let mut group = c.benchmark_group("fluid_system_step");
    group.sample_size(30);
    for count in [10_000u32, 20_000, 50_000] {
        group.bench_with_input(
            BenchmarkId::new("step_submit_wait", count),
            &count,
            |b, &count| {
                let mut system = FluidSystem::new(&device, count);
                apply_demo_params(&mut system);
                // Warm up: let the fluid leave the degenerate initial lattice.
                for _ in 0..30 {
                    run_step(&mut system, &device, &queue);
                }
                let _ = device.poll(wgpu::MaintainBase::Wait);
                b.iter(|| {
                    run_step(&mut system, &device, &queue);
                    let _ = device.poll(wgpu::MaintainBase::Wait);
                });
            },
        );
    }
    group.finish();
}

/// One-shot per-pass GPU timestamp breakdown (printed, not criterion-timed).
fn print_gpu_pass_breakdown() {
    let Some((device, queue)) = try_create_device() else {
        eprintln!("SKIPPED: GPU pass breakdown (no GPU adapter / SKIP_GPU_TESTS)");
        return;
    };
    eprintln!("\n=== FluidSystem::step per-pass GPU timings (median of 60 frames, ms) ===");
    for count in [10_000u32, 20_000, 50_000] {
        let mut system = FluidSystem::new(&device, count);
        apply_demo_params(&mut system);
        if !system.enable_gpu_timing(&device, &queue) {
            eprintln!("SKIPPED: device lacks TIMESTAMP_QUERY");
            return;
        }
        // Warm up, then sample.
        for _ in 0..30 {
            run_step(&mut system, &device, &queue);
        }
        let mut samples: Vec<Vec<(&'static str, f32)>> = Vec::new();
        for _ in 0..60 {
            run_step(&mut system, &device, &queue);
            if let Some(t) = system.read_gpu_timings(&device) {
                samples.push(t);
            }
        }
        if samples.is_empty() {
            eprintln!("{count} particles: no timing samples captured");
            continue;
        }
        eprint!(
            "{count:>6} particles | iterations={} |",
            system.get_iterations()
        );
        let span_count = samples[0].len();
        let mut total = 0.0f32;
        for i in 0..span_count {
            let mut values: Vec<f32> = samples.iter().map(|s| s[i].1).collect();
            values.sort_by(f32::total_cmp);
            let median = values[values.len() / 2];
            total += median;
            eprint!(" {}={median:.3}", samples[0][i].0);
        }
        eprintln!(" | total={total:.3}");
    }
    eprintln!();
}

criterion_group!(baselines, bench_voxel_grid, bench_fluid_system_step);

fn main() {
    print_gpu_pass_breakdown();
    baselines();
    Criterion::default().configure_from_args().final_summary();
}
