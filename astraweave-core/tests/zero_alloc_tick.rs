//! One simulation tick must allocate nothing.
//!
//! Ignored on the default test path (PR nextest does not run it). Allocation
//! Measurement runs it with `--features alloc-counter -- --ignored`.
//! The assertion stays at 0. A non-zero count is a deferred failure, not a
//! reason to raise this number.

#![cfg(feature = "alloc-counter")]

use astraweave_core::{step, IVec2, SimConfig, Team, World};
use astraweave_ecs::counting_alloc::{allocs, reset_allocs};

#[global_allocator]
static ALLOC: astraweave_ecs::counting_alloc::CountingAlloc =
    astraweave_ecs::counting_alloc::CountingAlloc;

#[test]
#[ignore = "Allocation Measurement only. Assertion stays at 0."]
fn one_sim_tick_allocates_nothing() {
    let mut world = World::new();
    let entity = world.spawn("player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 30);
    world
        .cooldowns_mut(entity)
        .unwrap()
        .map
        .insert("attack".into(), 5.0);
    let cfg = SimConfig { dt: 1.0 / 60.0 };
    // One warmup tick so any lazy init sits outside the measured call.
    step(&mut world, &cfg);

    reset_allocs();
    step(&mut world, &cfg);
    let n = allocs();
    eprintln!("[zero-alloc] sim step allocs={n}");
    assert_eq!(
        n, 0,
        "one sim tick allocated {n} times; the assertion stays at 0"
    );
}
