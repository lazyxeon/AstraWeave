//! Zero-allocation hot path enforcement tests.
//!
//! These tests validate that the ECS hot paths (schedule.run, query iteration,
//! component access) do not perform heap allocations after warmup.
//!
//! # Usage
//!
//! **IMPORTANT**: Run with --test-threads=1 because the global allocator counter
//! is shared across all tests: tests running in parallel will interfere with each other.
//!
//! ```bash
//! cargo test -p astraweave-ecs --features alloc-counter zero_alloc -- --test-threads=1
//! ```

#![cfg(feature = "alloc-counter")]

use astraweave_ecs::counting_alloc::{allocs, reset_allocs};
use astraweave_ecs::{App, Query, World};

// Install the counting allocator as the global allocator for this test binary
#[global_allocator]
static ALLOC: astraweave_ecs::counting_alloc::CountingAlloc =
    astraweave_ecs::counting_alloc::CountingAlloc;

// ============================================================================
// Test Components
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity {
    dx: f32,
    dy: f32,
}

// ============================================================================
// Debugging: Find exactly where allocations occur
// ============================================================================

/// Debug test to find exactly where allocations occur.
#[test]
fn debug_find_allocation_source() {
    let mut world = World::new();

    // Setup entities
    let entities: Vec<_> = (0..100)
        .map(|i| {
            let e = world.spawn();
            world.insert(
                e,
                Position {
                    x: i as f32,
                    y: 0.0,
                },
            );
            e
        })
        .collect();

    // Extended warmup
    for _ in 0..50 {
        for &e in &entities {
            let _ = world.get::<Position>(e);
        }
    }

    // Test 1: is_alive only
    reset_allocs();
    let e = entities[0];
    let _ = world.is_alive(e);
    let is_alive_allocs = allocs();
    println!("is_alive(): {} allocations", is_alive_allocs);

    // Test 2: Single get() call
    reset_allocs();
    let _ = world.get::<Position>(e);
    let get_allocs = allocs();
    println!("get<Position>(): {} allocations", get_allocs);

    // Test 3: Second get() call (should be warmed up now)
    reset_allocs();
    let _ = world.get::<Position>(e);
    let get2_allocs = allocs();
    println!("get<Position>() second: {} allocations", get2_allocs);

    // Test 4: get() on different entity
    reset_allocs();
    let _ = world.get::<Position>(entities[50]);
    let get3_allocs = allocs();
    println!(
        "get<Position>() different entity: {} allocations",
        get3_allocs
    );

    // Test 5: Many get() calls
    reset_allocs();
    for &e in &entities {
        let _ = world.get::<Position>(e);
    }
    let many_get_allocs = allocs();
    println!("100x get<Position>(): {} allocations", many_get_allocs);

    // Assertions
    assert_eq!(is_alive_allocs, 0, "is_alive() should not allocate");
    assert_eq!(get2_allocs, 0, "Second get() should not allocate");
    assert_eq!(
        get3_allocs, 0,
        "get() on different entity should not allocate"
    );
    assert_eq!(many_get_allocs, 0, "100x get() should not allocate");
}

/// Test that World::get() does not allocate after warmup.
#[test]
fn test_component_get_zero_alloc() {
    let mut world = World::new();

    // Setup entities
    let entities: Vec<_> = (0..100)
        .map(|i| {
            let e = world.spawn();
            world.insert(
                e,
                Position {
                    x: i as f32,
                    y: 0.0,
                },
            );
            e
        })
        .collect();

    // Extended warmup: Multiple passes to ensure all internal structures are allocated
    // INCLUDING thread-local state that RandomState might initialize
    for _ in 0..50 {
        for &e in &entities {
            let _ = world.get::<Position>(e);
        }
    }

    // Measure: Single pass after warmup
    reset_allocs();

    for &e in &entities {
        let _ = world.get::<Position>(e);
    }

    let allocs_during = allocs();

    // Strict assertion: get() must be zero-alloc
    assert_eq!(
        allocs_during, 0,
        "World::get() should not allocate (actual: {} for 100 calls)",
        allocs_during
    );
}

/// Test that World::get_mut() does not allocate after warmup.
#[test]
fn test_component_get_mut_zero_alloc() {
    let mut world = World::new();

    // Setup entities
    let entities: Vec<_> = (0..100)
        .map(|i| {
            let e = world.spawn();
            world.insert(
                e,
                Position {
                    x: i as f32,
                    y: 0.0,
                },
            );
            e
        })
        .collect();

    // Extended warmup
    for _ in 0..50 {
        for &e in &entities {
            if let Some(pos) = world.get_mut::<Position>(e) {
                pos.x += 0.0;
            }
        }
    }

    // Measure
    reset_allocs();

    for &e in &entities {
        if let Some(pos) = world.get_mut::<Position>(e) {
            pos.x += 1.0;
        }
    }

    let allocs_during = allocs();

    assert_eq!(
        allocs_during, 0,
        "World::get_mut() should not allocate (actual: {} for 100 calls)",
        allocs_during
    );
}

/// Test that is_alive() does not allocate.
#[test]
fn test_is_alive_zero_alloc() {
    let mut world = World::new();

    let entities: Vec<_> = (0..100).map(|_| world.spawn()).collect();

    // Warmup
    for &e in &entities {
        let _ = world.is_alive(e);
    }

    // Measure
    reset_allocs();

    for _ in 0..100 {
        for &e in &entities {
            let _ = world.is_alive(e);
        }
    }

    let allocs_during = allocs();

    assert_eq!(
        allocs_during, 0,
        "is_alive() should not allocate (actual: {})",
        allocs_during
    );
}

/// Test that has() does not allocate.
#[test]
fn test_has_zero_alloc() {
    let mut world = World::new();

    let entities: Vec<_> = (0..100)
        .map(|i| {
            let e = world.spawn();
            world.insert(
                e,
                Position {
                    x: i as f32,
                    y: 0.0,
                },
            );
            e
        })
        .collect();

    // Extended warmup
    for _ in 0..50 {
        for &e in &entities {
            let _ = world.has::<Position>(e);
        }
    }

    // Measure
    reset_allocs();

    for &e in &entities {
        let _ = world.has::<Position>(e);
    }

    let allocs_during = allocs();

    assert_eq!(
        allocs_during, 0,
        "has() should not allocate (actual: {} for 100 calls)",
        allocs_during
    );
}

/// Test that Query iteration does not allocate (after construction).
#[test]
fn test_query_iteration_zero_alloc() {
    let mut world = World::new();

    // Setup entities with Position
    for i in 0..100 {
        let e = world.spawn();
        world.insert(
            e,
            Position {
                x: i as f32,
                y: 0.0,
            },
        );
    }

    // Setup entities with Position + Velocity
    for i in 0..100 {
        let e = world.spawn();
        world.insert(
            e,
            Position {
                x: i as f32,
                y: 0.0,
            },
        );
        world.insert(e, Velocity { dx: 1.0, dy: 1.0 });
    }

    // Warmup: Build and iterate query multiple times
    for _ in 0..10 {
        let query = Query::<Position>::new(&world);
        for (_, _pos) in query {
            // iterate
        }
    }

    // Measure: Pre-construct query, then measure iteration only
    let query = Query::<Position>::new(&world);

    reset_allocs();

    let mut sum = 0.0f32;
    for (_, pos) in query {
        sum += pos.x;
    }

    let iteration_allocs = allocs();

    // Prevent optimization
    assert!(sum > 0.0);

    // Iteration should be zero-alloc
    assert_eq!(
        iteration_allocs, 0,
        "Query iteration should not allocate (actual: {})",
        iteration_allocs
    );
}

/// Test that schedule.run() allocations are bounded after warmup.
#[test]
fn test_schedule_run_bounded_allocs() {
    let mut app = App::new();

    fn movement_system(world: &mut World) {
        let entities = world.entities_with::<Position>();
        for entity in entities {
            if let Some(pos) = world.get_mut::<Position>(entity) {
                pos.x += 1.0;
                pos.y += 1.0;
            }
        }
    }

    app.add_system_known_stage("simulation", movement_system);

    // Spawn test entities
    for i in 0..100 {
        let e = app.world.spawn();
        app.world.insert(
            e,
            Position {
                x: i as f32,
                y: 0.0,
            },
        );
        app.world.insert(e, Velocity { dx: 1.0, dy: 1.0 });
    }

    // Extended warmup
    for _ in 0..50 {
        app.schedule.run(&mut app.world);
    }

    // Measure: Reset counter and run one frame
    reset_allocs();

    app.schedule.run(&mut app.world);

    let allocs_during_run = allocs();

    // Currently entities_with() allocates a Vec - this is a known limitation
    // We accept <= 2 allocations until we provide iter_entities_with()
    assert!(
        allocs_during_run <= 2,
        "Schedule run should have minimal allocations (actual: {})",
        allocs_during_run
    );
}
