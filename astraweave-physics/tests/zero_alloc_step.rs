//! One `PhysicsWorld::step` must allocate nothing.
//!
//! Ignored unless built with `--features alloc-counter` and `--ignored`.
//! PR nextest does not enable that feature, so it does not run this test.
//! Allocation Measurement does. The assertion stays at 0.

#![cfg(feature = "alloc-counter")]

use std::alloc::{GlobalAlloc, Layout, System};

use astraweave_physics::{Layers, PhysicsWorld};
use astraweave_profiling::FrameAllocStats;
use glam::Vec3;

struct CountingAlloc;

// SAFETY: every method forwards the pointer and layout to `System` after
// recording the size. `System` upholds the `GlobalAlloc` contract.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        astraweave_profiling::counters::record_alloc(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        astraweave_profiling::counters::record_dealloc(layout.size());
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        astraweave_profiling::counters::record_alloc(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        astraweave_profiling::counters::record_realloc(layout.size(), new_size);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

fn world_with_stacked_boxes() -> PhysicsWorld {
    let mut world = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));
    world.create_ground_plane(Vec3::new(100.0, 0.5, 100.0), 0.5);
    for i in 0..64 {
        let pos = Vec3::new(0.0, 2.0 + i as f32 * 1.1, 0.0);
        world.add_dynamic_box(pos, Vec3::splat(0.5), 1.0, Layers::DEFAULT);
    }
    world
}

#[test]
#[ignore = "Allocation Measurement only. Assertion stays at 0."]
fn one_physics_step_allocates_nothing() {
    let mut world = world_with_stacked_boxes();
    for _ in 0..30 {
        world.step();
    }

    let stats = FrameAllocStats::begin_frame();
    world.step();
    let delta = stats.end_frame();
    eprintln!(
        "[zero-alloc] physics.step allocs={} bytes={}",
        delta.allocs, delta.bytes_allocated
    );
    assert_eq!(
        delta.allocs, 0,
        "one physics.step allocated {} times after 30 warmup steps; the assertion stays at 0",
        delta.allocs
    );
}
