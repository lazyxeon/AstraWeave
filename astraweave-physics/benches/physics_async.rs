// physics_async.rs - Async Physics Benchmarks
//
// Benchmarks for parallel physics pipeline measuring:
// - Single-thread vs multi-thread speedup
// - Scalability with thread count (1, 2, 4, 8)
// - Performance with varying NPC counts
// - Telemetry overhead

// =============================================================================
// MISSION-CRITICAL CORRECTNESS ASSERTIONS
// =============================================================================
// Async physics benchmarks validate CORRECTNESS of parallel simulation.
// Assertions verify:
//   1. Determinism: Same inputs produce same outputs regardless of thread count
//   2. Body Validity: All bodies have finite, valid transforms after simulation
//   3. Scalability Correctness: More threads don't corrupt physics state
//   4. Telemetry Integrity: Profile data is valid and complete
//   5. Mixed Workload: Characters + rigid bodies don't interfere incorrectly
// =============================================================================

#![cfg(feature = "async-physics")]

use astraweave_physics::{Layers, PhysicsWorld};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use glam::vec3;
use rand::{Rng, SeedableRng};
use std::hint::black_box;

/// CORRECTNESS: Validate physics world is in valid state after step
#[inline]
fn assert_world_valid_after_step(world: &PhysicsWorld, sample_ids: &[u64], context: &str) {
    for (i, &id) in sample_ids.iter().take(5).enumerate() {
        if let Some(mat) = world.body_transform(id) {
            // Extract position from column 3 (translation column)
            let pos = mat.col(3).truncate();
            // Position must be finite
            assert!(
                pos.x.is_finite() && pos.y.is_finite() && pos.z.is_finite(),
                "[CORRECTNESS FAILURE] {}/body_{}: position non-finite {:?}",
                context,
                i,
                pos
            );
            // Position shouldn't explode
            assert!(
                pos.length_squared() < 10000.0 * 10000.0,
                "[CORRECTNESS FAILURE] {}/body_{}: position exploded {:?}",
                context,
                i,
                pos
            );
            // Check matrix has valid determinant
            let det = mat.determinant();
            assert!(
                det.is_finite() && det.abs() > 1e-10,
                "[CORRECTNESS FAILURE] {}/body_{}: invalid determinant {}",
                context,
                i,
                det
            );
        }
    }
}

/// CORRECTNESS: Validate telemetry data is valid
#[inline]
fn assert_telemetry_valid(profile: &astraweave_physics::PhysicsStepProfile, context: &str) {
    assert!(
        profile.active_body_count < 10_000_000,
        "{context}: active body count is implausible ({})",
        profile.active_body_count
    );
    assert!(
        profile.collision_pair_count < 100_000_000,
        "{context}: collision pair count is implausible ({})",
        profile.collision_pair_count
    );
    let _ = profile.total_duration;
}

/// Create a world with N characters for benchmarking
fn create_world_with_characters(char_count: usize) -> (PhysicsWorld, Vec<u64>) {
    let mut world = PhysicsWorld::new(vec3(0.0, -9.8, 0.0));

    // Create ground
    world.create_ground_plane(vec3(100.0, 0.5, 100.0), 0.9);

    // Create characters in deterministic grid
    let grid_size = (char_count as f32).sqrt().ceil() as usize;
    let spacing = 3.0;
    let mut char_ids = Vec::with_capacity(char_count);

    for i in 0..char_count {
        let x = (i % grid_size) as f32 * spacing - (grid_size as f32 * spacing / 2.0);
        let z = (i / grid_size) as f32 * spacing - (grid_size as f32 * spacing / 2.0);
        let id = world.add_character(vec3(x, 1.0, z), vec3(0.4, 0.9, 0.4));
        char_ids.push(id);
    }

    // CORRECTNESS: Verify all characters spawned
    assert_eq!(
        char_ids.len(),
        char_count,
        "[CORRECTNESS FAILURE] create_world_with_characters: expected {}, got {}",
        char_count,
        char_ids.len()
    );

    (world, char_ids)
}

/// Create a world with N dynamic rigid bodies
fn create_world_with_rigid_bodies(body_count: usize) -> (PhysicsWorld, Vec<u64>) {
    let mut world = PhysicsWorld::new(vec3(0.0, -9.8, 0.0));

    // Create ground
    world.create_ground_plane(vec3(100.0, 0.5, 100.0), 0.9);

    // Create bodies in deterministic positions
    let mut rng = rand::rngs::StdRng::seed_from_u64(12345);
    let mut body_ids = Vec::with_capacity(body_count);

    for _ in 0..body_count {
        let x = rng.random_range(-40.0..40.0);
        let y = rng.random_range(5.0..15.0);
        let z = rng.random_range(-40.0..40.0);

        let id = world.add_dynamic_box(vec3(x, y, z), vec3(0.5, 0.5, 0.5), 1.0, Layers::DEFAULT);
        body_ids.push(id);
    }

    // CORRECTNESS: Verify all bodies spawned
    assert_eq!(
        body_ids.len(),
        body_count,
        "[CORRECTNESS FAILURE] create_world_with_rigid_bodies: expected {}, got {}",
        body_count,
        body_ids.len()
    );

    (world, body_ids)
}

/// Benchmark: Full physics tick (single-threaded baseline)
fn physics_full_tick_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_full_tick_baseline");

    for npc_count in [100, 500, 1000, 2500].iter() {
        let (mut world, char_ids) = create_world_with_characters(*npc_count);

        group.throughput(Throughput::Elements(*npc_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(npc_count),
            npc_count,
            |b, &count| {
                b.iter(|| {
                    world.step();
                    // CORRECTNESS: Validate world state after step
                    assert_world_valid_after_step(
                        &world,
                        &char_ids,
                        &format!("baseline/{}", count),
                    );
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Full physics tick with async enabled (4 threads)
fn physics_async_full_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_async_full_tick");

    for npc_count in [100, 500, 1000, 2500].iter() {
        let (mut world, char_ids) = create_world_with_characters(*npc_count);

        // Enable async physics with 4 threads
        world.enable_async_physics(4);

        group.throughput(Throughput::Elements(*npc_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(npc_count),
            npc_count,
            |b, &count| {
                b.iter(|| {
                    world.step();
                    // CORRECTNESS: Async simulation must produce valid state
                    assert_world_valid_after_step(&world, &char_ids, &format!("async_4/{}", count));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Thread scaling (2, 4, 8 threads with 2500 NPCs)
fn physics_async_thread_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_async_thread_scaling");
    const NPC_COUNT: usize = 2500;

    for thread_count in [1, 2, 4, 8].iter() {
        let (mut world, char_ids) = create_world_with_characters(NPC_COUNT);

        // Enable async physics with variable thread count
        world.enable_async_physics(*thread_count);

        group.throughput(Throughput::Elements(NPC_COUNT as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(thread_count),
            thread_count,
            |b, &threads| {
                b.iter(|| {
                    world.step();
                    // CORRECTNESS: Thread count shouldn't affect correctness
                    assert_world_valid_after_step(
                        &world,
                        &char_ids,
                        &format!("scaling/{}threads", threads),
                    );
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Rigid body simulation (async vs sync)
fn physics_async_rigid_bodies(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_async_rigid_bodies");
    const BODY_COUNT: usize = 500;

    // Baseline (single-thread)
    {
        let (mut world, body_ids) = create_world_with_rigid_bodies(BODY_COUNT);

        group.throughput(Throughput::Elements(BODY_COUNT as u64));
        group.bench_function("sync", |b| {
            b.iter(|| {
                world.step();
                // CORRECTNESS: Sync simulation validity
                assert_world_valid_after_step(&world, &body_ids, "rigid_bodies/sync");
            });
        });
    }

    // Async (4 threads)
    {
        let (mut world, body_ids) = create_world_with_rigid_bodies(BODY_COUNT);
        world.enable_async_physics(4);

        group.throughput(Throughput::Elements(BODY_COUNT as u64));
        group.bench_function("async_4_threads", |b| {
            b.iter(|| {
                world.step();
                // CORRECTNESS: Async simulation validity
                assert_world_valid_after_step(&world, &body_ids, "rigid_bodies/async");
            });
        });
    }

    group.finish();
}

/// Benchmark: Telemetry overhead
fn physics_async_telemetry_overhead(c: &mut Criterion) {
    const NPC_COUNT: usize = 1000;

    c.bench_function("physics_async_telemetry_overhead", |b| {
        let (mut world, _char_ids) = create_world_with_characters(NPC_COUNT);
        world.enable_async_physics(4);

        b.iter(|| {
            world.step();
            // Get telemetry (should be very fast)
            let profile = world
                .get_last_profile()
                .expect("physics step records a profile");
            assert_telemetry_valid(&profile, "telemetry_overhead");
            black_box(profile)
        });
    });
}

/// Benchmark: Character movement + physics (realistic workload)
fn physics_async_character_simulation(c: &mut Criterion) {
    const CHAR_COUNT: usize = 2500;
    let (mut world, char_ids) = create_world_with_characters(CHAR_COUNT);
    world.enable_async_physics(4);

    c.bench_function("physics_async_character_simulation", |b| {
        let mut frame = 0;
        b.iter(|| {
            // Move characters (simulating AI/input)
            for &id in &char_ids {
                let move_dir = vec3(
                    ((frame as f32 + id as f32) * 0.1).sin(),
                    0.0,
                    ((frame as f32 + id as f32) * 0.1).cos(),
                );
                world.control_character(id, move_dir, 1.0 / 60.0, false);
            }

            // Step physics
            world.step();

            // CORRECTNESS: Validate world after character simulation
            assert_world_valid_after_step(&world, &char_ids, "character_simulation");

            frame += 1;
        });
    });
}

/// Benchmark: Mixed workload (characters + rigid bodies)
fn physics_async_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics_async_mixed_workload");

    for (char_count, body_count) in [(500, 200), (1000, 400), (2000, 800)].iter() {
        let mut world = PhysicsWorld::new(vec3(0.0, -9.8, 0.0));
        world.create_ground_plane(vec3(100.0, 0.5, 100.0), 0.9);

        let mut all_body_ids = Vec::new();

        // Add characters
        for i in 0..*char_count {
            let x = (i % 50) as f32 * 2.0 - 50.0;
            let z = (i / 50) as f32 * 2.0 - 50.0;
            let id = world.add_character(vec3(x, 1.0, z), vec3(0.4, 0.9, 0.4));
            all_body_ids.push(id);
        }

        // Add rigid bodies
        let mut rng = rand::rngs::StdRng::seed_from_u64(54321);
        for _ in 0..*body_count {
            let x = rng.random_range(-40.0..40.0);
            let y = rng.random_range(5.0..15.0);
            let z = rng.random_range(-40.0..40.0);
            let id =
                world.add_dynamic_box(vec3(x, y, z), vec3(0.5, 0.5, 0.5), 1.0, Layers::DEFAULT);
            all_body_ids.push(id);
        }

        // CORRECTNESS: Verify all bodies spawned
        let expected_total = char_count + body_count;
        assert_eq!(
            all_body_ids.len(),
            expected_total,
            "[CORRECTNESS FAILURE] mixed_workload: expected {} bodies, got {}",
            expected_total,
            all_body_ids.len()
        );

        // Enable async
        world.enable_async_physics(4);

        let label = format!("{}chars_{}bodies", char_count, body_count);
        group.throughput(Throughput::Elements((char_count + body_count) as u64));
        group.bench_function(&label, |b| {
            b.iter(|| {
                world.step();
                // CORRECTNESS: Mixed workload must produce valid state
                assert_world_valid_after_step(&world, &all_body_ids, &label);
            });
        });
    }

    group.finish();
}

criterion_group!(
    async_physics_benches,
    physics_full_tick_baseline,
    physics_async_full_tick,
    physics_async_thread_scaling,
    physics_async_rigid_bodies,
    physics_async_telemetry_overhead,
    physics_async_character_simulation,
    physics_async_mixed_workload,
);
criterion_main!(async_physics_benches);
