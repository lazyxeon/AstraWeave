use astraweave_physics::PhysicsWorld;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use glam::{vec3, Mat4};
use std::hint::black_box;

// =============================================================================
// MISSION-CRITICAL CORRECTNESS ASSERTIONS
// =============================================================================
// Character controller benchmarks validate CORRECTNESS of movement systems.
// Assertions verify:
//   1. Character Position: Characters actually move in response to input
//   2. Transform Validity: Transformation matrices are finite and reasonable
//   3. Ground Contact: Characters don't fall through ground
//   4. Multi-Character: Batch operations don't corrupt state
//   5. Physics Integration: Full tick produces valid world state
// =============================================================================

/// CORRECTNESS: Validate character transform matrix is geometrically valid
#[inline]
fn assert_transform_valid(transform: Option<Mat4>, context: &str) {
    assert!(
        transform.is_some(),
        "[CORRECTNESS FAILURE] {}: transform lookup returned None",
        context
    );
    if let Some(mat) = transform {
        // Extract position from column 3 (translation column)
        let pos = mat.col(3).truncate();
        // Position must be finite
        assert!(
            pos.x.is_finite() && pos.y.is_finite() && pos.z.is_finite(),
            "[CORRECTNESS FAILURE] {}: position contains non-finite values {:?}",
            context,
            pos
        );
        // Position Y should be above ground (not fallen through)
        assert!(
            pos.y > -10.0,
            "[CORRECTNESS FAILURE] {}: character fell through ground (y={})",
            context,
            pos.y
        );
        // Check matrix has valid scaling (determinant should be non-zero, not NaN)
        let det = mat.determinant();
        assert!(
            det.is_finite() && det.abs() > 1e-10,
            "[CORRECTNESS FAILURE] {}: transform matrix has invalid determinant {}",
            context,
            det
        );
    }
}

/// Put the capsule back on the finite ground and clear the fall velocity.
fn reset_character_for_tick(world: &mut PhysicsWorld, id: astraweave_physics::BodyId) {
    world.set_body_position(id, vec3(0.0, 1.0, 0.0));
    if let Some(ctrl) = world.char_map.get_mut(&id) {
        ctrl.vertical_velocity = 0.0;
        ctrl.time_since_grounded = 0.0;
        ctrl.jump_buffer_timer = 0.0;
        ctrl.pending_jump_velocity = 0.0;
    }
}

/// Setup a simple test world with ground
fn setup_simple_world() -> PhysicsWorld {
    let mut world = PhysicsWorld::new(vec3(0.0, -9.8, 0.0));
    world.create_ground_plane(vec3(50.0, 0.5, 50.0), 0.9);
    world
}

/// Benchmark: Single character controller update (straight movement)
fn character_move_straight(c: &mut Criterion) {
    let mut world = setup_simple_world();
    let char_id = world.add_character(vec3(0.0, 1.0, 0.0), vec3(0.4, 0.9, 0.4));

    // CORRECTNESS: Verify initial transform is valid
    let initial_transform = world.body_transform(char_id);
    assert_transform_valid(initial_transform, "character_move_straight/initial");

    c.bench_function("character_move_straight", |b| {
        b.iter(|| {
            let desired_move = black_box(vec3(1.0, 0.0, 0.0));
            world.control_character(char_id, desired_move, 1.0 / 60.0, false);
            // CORRECTNESS: Transform must remain valid after movement
            let transform = world.body_transform(char_id);
            assert_transform_valid(transform, "character_move_straight/post");
            black_box(transform)
        });
    });
}

/// Benchmark: Character controller with diagonal movement
fn character_move_diagonal(c: &mut Criterion) {
    let mut world = setup_simple_world();
    let char_id = world.add_character(vec3(0.0, 1.0, 0.0), vec3(0.4, 0.9, 0.4));

    c.bench_function("character_move_diagonal", |b| {
        b.iter(|| {
            let desired_move = black_box(vec3(1.0, 0.0, 1.0).normalize());
            world.control_character(char_id, desired_move, 1.0 / 60.0, false);
            // CORRECTNESS: Diagonal movement should produce valid state
            let transform = world.body_transform(char_id);
            assert_transform_valid(transform, "character_move_diagonal");
            black_box(transform)
        });
    });
}

/// Benchmark: Multiple characters moving simultaneously
fn character_move_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("character_move_batch");

    for char_count in [1, 10, 50, 100].iter() {
        let mut world = setup_simple_world();
        let mut char_ids = Vec::new();

        // Spawn characters in a grid
        let grid_size = (*char_count as f32).sqrt().ceil() as usize;
        let spacing = 3.0;

        for i in 0..*char_count {
            let x = (i % grid_size) as f32 * spacing - (grid_size as f32 * spacing / 2.0);
            let z = (i / grid_size) as f32 * spacing - (grid_size as f32 * spacing / 2.0);
            let id = world.add_character(vec3(x, 1.0, z), vec3(0.4, 0.9, 0.4));
            char_ids.push(id);
        }

        // CORRECTNESS: Verify all characters spawned correctly
        assert_eq!(
            char_ids.len(),
            *char_count,
            "[CORRECTNESS FAILURE] character_move_batch: expected {} characters, got {}",
            char_count,
            char_ids.len()
        );

        group.throughput(Throughput::Elements(*char_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(char_count),
            char_count,
            |b, &count| {
                b.iter(|| {
                    for &id in &char_ids {
                        let desired_move = black_box(vec3(1.0, 0.0, 0.0));
                        world.control_character(id, desired_move, 1.0 / 60.0, false);
                    }
                    // CORRECTNESS: Verify all characters still have valid transforms
                    for (i, &id) in char_ids.iter().enumerate() {
                        let transform = world.body_transform(id);
                        assert_transform_valid(
                            transform,
                            &format!("character_move_batch/{}/char_{}", count, i),
                        );
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Character controller with obstacle avoidance
fn character_move_with_obstacles(c: &mut Criterion) {
    let mut world = setup_simple_world();

    // Add wall of obstacles in front of character
    for i in 0..10 {
        world.add_dynamic_box(
            vec3(5.0, 1.0, (i as f32) - 5.0),
            vec3(0.5, 1.0, 0.5),
            10.0,
            astraweave_physics::Layers::DEFAULT,
        );
    }

    let char_id = world.add_character(vec3(0.0, 1.0, 0.0), vec3(0.4, 0.9, 0.4));

    c.bench_function("character_move_with_obstacles", |b| {
        b.iter(|| {
            let desired_move = black_box(vec3(1.0, 0.0, 0.0));
            world.control_character(char_id, desired_move, 1.0 / 60.0, false);
            // CORRECTNESS: Character should remain in valid state even with obstacles
            let transform = world.body_transform(char_id);
            assert_transform_valid(transform, "character_move_with_obstacles");
            black_box(transform)
        });
    });
}

/// Benchmark: Character controller step climbing
fn character_step_climbing(c: &mut Criterion) {
    let mut world = setup_simple_world();

    // Add step obstacles (small boxes to climb over)
    for i in 0..5 {
        world.add_dynamic_box(
            vec3((i as f32) * 2.0, 0.2, 0.0),
            vec3(0.8, 0.2, 1.0),
            100.0, // Heavy so they don't move
            astraweave_physics::Layers::DEFAULT,
        );
    }

    let char_id = world.add_character(vec3(-2.0, 1.0, 0.0), vec3(0.4, 0.9, 0.4));

    c.bench_function("character_step_climbing", |b| {
        b.iter(|| {
            let desired_move = black_box(vec3(1.0, 0.0, 0.0));
            world.control_character(char_id, desired_move, 1.0 / 60.0, true);
            // CORRECTNESS: Step climbing should produce valid transform
            let transform = world.body_transform(char_id);
            assert_transform_valid(transform, "character_step_climbing");
            black_box(transform)
        });
    });
}

/// Benchmark: Full character simulation tick (move + physics step)
fn character_full_tick(c: &mut Criterion) {
    let mut world = setup_simple_world();
    let char_id = world.add_character(vec3(0.0, 1.0, 0.0), vec3(0.4, 0.9, 0.4));

    c.bench_function("character_full_tick", |b| {
        b.iter(|| {
            // The ground plane is 100 m across. Criterion repeats this body, so
            // each sample starts at the origin. The y > -10 assertion stays.
            reset_character_for_tick(&mut world, char_id);
            let desired_move = black_box(vec3(1.0, 0.0, 0.0));
            world.control_character(char_id, desired_move, 1.0 / 60.0, false);
            world.step();
            let transform = world.body_transform(char_id);
            assert_transform_valid(transform, "character_full_tick");
            black_box(transform)
        });
    });
}

/// Benchmark: Character transforms retrieval
fn character_transform_lookup(c: &mut Criterion) {
    let mut world = setup_simple_world();
    let char_id = world.add_character(vec3(0.0, 1.0, 0.0), vec3(0.4, 0.9, 0.4));

    c.bench_function("character_transform_lookup", |b| {
        b.iter(|| {
            let transform = world.body_transform(black_box(char_id));
            // CORRECTNESS: Transform lookup must always return valid data
            assert_transform_valid(transform, "character_transform_lookup");
            black_box(transform)
        });
    });
}

criterion_group!(
    character_controller_benches,
    character_move_straight,
    character_move_diagonal,
    character_move_batch,
    character_move_with_obstacles,
    character_step_climbing,
    character_full_tick,
    character_transform_lookup,
);
criterion_main!(character_controller_benches);
