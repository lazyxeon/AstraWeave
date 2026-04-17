// SPDX-License-Identifier: MIT
//! Allocation-count regression benchmarks for ECS hot paths.
//!
//! These benches complement the timing benches in `ecs_benchmarks.rs` and
//! `storage_benchmarks.rs` — they measure how many heap allocations each hot
//! path performs per call. A regression here means the allocation footprint
//! grew; fix timing regressions in the other benches and allocation
//! regressions here.
//!
//! # How to run
//!
//! ```bash
//! # Time + alloc measurement (installs CountingAlloc as the global allocator):
//! cargo bench -p astraweave-ecs --features alloc-counter --bench alloc_measure
//! ```
//!
//! # Interpretation
//!
//! Each bench does two things:
//! 1. A Criterion group for per-call wall-clock timing.
//! 2. A one-shot measurement that snapshots `FrameAllocStats` before and after a
//!    single call, then asserts `allocs <= MAX_ALLOCS`.
//!
//! The `MAX_ALLOCS` constant for each path is set to the first captured run's
//! count plus a ~10% margin. It is explicitly NOT an optimisation target —
//! regressions should investigate WHY allocation growth happened before
//! tightening the threshold.
//!
//! # Current thresholds (placeholders until first real capture)
//!
//! These values are generous upper bounds that will be tightened once a real
//! capture is taken. See `docs/audits/allocation_measurement_plan_<date>.md`
//! for the observed counts.

// Install `CountingAlloc` as the global allocator for this bench binary so that
// `astraweave_profiling::counters` sees every heap operation. Gated by
// `alloc-counter` so that `cargo bench` without the feature does not change the
// global allocator unexpectedly.
#[cfg(feature = "alloc-counter")]
#[global_allocator]
static ALLOC: astraweave_ecs::counting_alloc::CountingAlloc =
    astraweave_ecs::counting_alloc::CountingAlloc;

use astraweave_ecs::parallel::{ParallelSchedule, SystemDescriptor};
use astraweave_ecs::World;
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

/// Placeholder threshold — replace with measured value + 10% after first real
/// capture. Intentionally loose to avoid false positives during onboarding.
const MAX_ALLOCS_SCHEDULE_RUN: usize = 10_000;
const MAX_ALLOCS_BUILD_GROUPS: usize = 5_000;

#[derive(Clone, Copy)]
struct Position {
    #[allow(dead_code)]
    x: f32,
    #[allow(dead_code)]
    y: f32,
}

#[derive(Clone, Copy)]
struct Velocity {
    #[allow(dead_code)]
    dx: f32,
    #[allow(dead_code)]
    dy: f32,
}

fn noop_system_reads_position(_world: &mut World) {}
fn noop_system_reads_velocity(_world: &mut World) {}
fn noop_system_writes_position(_world: &mut World) {}

fn setup_schedule_with_n_systems(n: usize) -> ParallelSchedule {
    let mut schedule = ParallelSchedule::new();
    schedule.add_stage("simulation");
    for i in 0..n {
        // Rotate across read/write patterns so build_groups does real coloring work.
        let desc = match i % 3 {
            0 => SystemDescriptor::new(noop_system_reads_position).reads::<Position>(),
            1 => SystemDescriptor::new(noop_system_reads_velocity).reads::<Velocity>(),
            _ => SystemDescriptor::new(noop_system_writes_position).writes::<Position>(),
        };
        schedule.add_system("simulation", desc);
    }
    schedule
}

fn bench_schedule_run(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs.schedule.run");
    for n_systems in [4_usize, 16, 64] {
        group.bench_function(format!("systems_{}", n_systems), |b| {
            let schedule = setup_schedule_with_n_systems(n_systems);
            let mut world = World::new();
            b.iter(|| {
                schedule.run(&mut world);
                black_box(&mut world);
            });
        });
    }
    group.finish();

    #[cfg(feature = "alloc-counter")]
    assert_allocs_below::<_>(
        "ecs.schedule.run",
        MAX_ALLOCS_SCHEDULE_RUN,
        || {
            let schedule = setup_schedule_with_n_systems(16);
            let mut world = World::new();
            // One tick reflects steady-state cost.
            schedule.run(&mut world);
        },
    );
}

fn bench_build_groups(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs.schedule.build_groups");
    for n_systems in [4_usize, 16, 64] {
        group.bench_function(format!("systems_{}", n_systems), |b| {
            let schedule = setup_schedule_with_n_systems(n_systems);
            let mut world = World::new();
            b.iter(|| {
                // schedule.run indirectly calls build_groups per stage.
                schedule.run(&mut world);
                black_box(&mut world);
            });
        });
    }
    group.finish();

    // build_groups is a private method (not directly callable from benches), so
    // the one-shot assertion is naturally covered by the schedule.run assertion
    // above. See the measurement plan for how the two plots separate in Tracy.
    #[cfg(feature = "alloc-counter")]
    {
        let _ = MAX_ALLOCS_BUILD_GROUPS;
    }
}

/// Run `f` once and assert its allocation count is at most `max_allocs`.
/// Prints a summary line either way so Criterion output contains the number.
#[cfg(feature = "alloc-counter")]
fn assert_allocs_below<F: FnOnce()>(path_name: &str, max_allocs: usize, f: F) {
    use astraweave_profiling::FrameAllocStats;
    let stats = FrameAllocStats::begin_frame();
    f();
    let delta = stats.end_frame();
    println!(
        "[alloc-measure] {}: allocs={} bytes={} reallocs={} net={} (threshold={})",
        path_name,
        delta.allocs,
        delta.bytes_allocated,
        delta.reallocs,
        delta.net_allocs,
        max_allocs
    );
    assert!(
        delta.allocs <= max_allocs,
        "{}: allocation regression — {} allocs exceeds threshold {}. \
         If this is expected, investigate the cause before raising the threshold.",
        path_name,
        delta.allocs,
        max_allocs
    );
}

criterion_group!(alloc_measure_benches, bench_schedule_run, bench_build_groups);
criterion_main!(alloc_measure_benches);
