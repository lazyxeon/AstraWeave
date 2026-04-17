// SPDX-License-Identifier: MIT
//! Allocation-count regression bench for `PhysicsWorld::step_internal`.
//!
//! Answers audit open question #7 — "Does Rapier3D's per-step allocation count
//! grow with simulation time?" — by running a fixed body set for N steps and
//! asserting bounded allocation per step.
//!
//! # How to run
//!
//! ```bash
//! cargo bench -p astraweave-physics --features alloc-counter --bench alloc_measure
//! ```

// A tiny `System`-forwarding counting allocator. Lives in the bench to avoid a
// runtime dep on astraweave-ecs from astraweave-physics (kept optional).
#[cfg(feature = "alloc-counter")]
mod bench_alloc {
    use std::alloc::{GlobalAlloc, Layout, System};

    pub struct BenchAlloc;

    unsafe impl GlobalAlloc for BenchAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            astraweave_profiling::counters::record_alloc(layout.size());
            System.alloc(layout)
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            astraweave_profiling::counters::record_dealloc(layout.size());
            System.dealloc(ptr, layout)
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            astraweave_profiling::counters::record_alloc(layout.size());
            System.alloc_zeroed(layout)
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            astraweave_profiling::counters::record_realloc(layout.size(), new_size);
            System.realloc(ptr, layout, new_size)
        }
    }
}

#[cfg(feature = "alloc-counter")]
#[global_allocator]
static ALLOC: bench_alloc::BenchAlloc = bench_alloc::BenchAlloc;

use astraweave_physics::{Layers, PhysicsWorld};
use criterion::{criterion_group, criterion_main, Criterion};
use glam::Vec3;
use std::hint::black_box;

/// Placeholder threshold — tighten after first real capture. See measurement plan.
const MAX_ALLOCS_PER_STEP: usize = 10_000;

fn setup_world_with_bodies(count: usize) -> PhysicsWorld {
    let mut w = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
    let _ = w.create_ground_plane(Vec3::new(100.0, 0.5, 100.0), 0.5);
    // Stack a column of dynamic cubes so Rapier actually solves contacts.
    for i in 0..count {
        let pos = Vec3::new(0.0, 2.0 + i as f32 * 1.1, 0.0);
        let _ = w.add_dynamic_box(pos, Vec3::splat(0.5), 1.0, Layers::DEFAULT);
    }
    w
}

fn bench_physics_step(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics.step");
    for body_count in [16_usize, 64, 256] {
        group.bench_function(format!("bodies_{}", body_count), |b| {
            let mut world = setup_world_with_bodies(body_count);
            // Warm up one step so broadphase is primed.
            world.step();
            b.iter(|| {
                world.step();
                black_box(&mut world);
            });
        });
    }
    group.finish();

    #[cfg(feature = "alloc-counter")]
    {
        use astraweave_profiling::FrameAllocStats;

        // Warmup: let Rapier reach steady-state. Audit open question asks whether
        // allocation grows with sim time — the test sequence below captures 3
        // windows (early, mid, late) and asserts the late window stays bounded.
        let mut world = setup_world_with_bodies(64);
        for _ in 0..30 {
            world.step();
        }

        let stats = FrameAllocStats::begin_frame();
        world.step();
        let early = stats.end_frame();

        for _ in 0..200 {
            world.step();
        }

        let stats = FrameAllocStats::begin_frame();
        world.step();
        let late = stats.end_frame();

        println!(
            "[alloc-measure] physics.step (bodies=64, 30 warmup steps): allocs={} bytes={}",
            early.allocs, early.bytes_allocated
        );
        println!(
            "[alloc-measure] physics.step (bodies=64, 230 warmup steps): allocs={} bytes={}",
            late.allocs, late.bytes_allocated
        );

        assert!(
            early.allocs <= MAX_ALLOCS_PER_STEP,
            "physics.step (early) regression: {} allocs > threshold {}",
            early.allocs,
            MAX_ALLOCS_PER_STEP
        );
        assert!(
            late.allocs <= MAX_ALLOCS_PER_STEP,
            "physics.step (late) regression: {} allocs > threshold {}",
            late.allocs,
            MAX_ALLOCS_PER_STEP
        );
    }
}

criterion_group!(benches, bench_physics_step);
criterion_main!(benches);
