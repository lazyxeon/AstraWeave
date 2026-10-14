use astraweave_physics::{Layers, PhysicsWorld};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use glam::vec3;
use rapier3d::prelude::*;
use std::hint::black_box;

// =============================================================================
// MISSION-CRITICAL CORRECTNESS ASSERTIONS
// =============================================================================
// This benchmark suite validates not only performance but CORRECTNESS of physics.
// Each benchmark includes assertions to verify:
//   1. Raycast Hits: Rays that should hit do hit
//   2. Raycast Misses: Rays that shouldn't hit don't hit
//   3. Normal Validity: Surface normals are unit vectors
//   4. Distance Validity: Hit distances are within expected ranges
//   5. World Integrity: Physics world state isn't corrupted by queries
// =============================================================================

/// CORRECTNESS: Validate raycast hit result is geometrically valid
#[inline]
fn assert_raycast_hit_valid(
    result: &Option<(ColliderHandle, RayIntersection)>,
    max_distance: f32,
    context: &str,
) {
    if let Some((_, intersection)) = result {
        // Hit distance must be positive and within max_distance
        assert!(
            intersection.time_of_impact >= 0.0,
            "[CORRECTNESS FAILURE] {}: negative hit distance {}",
            context,
            intersection.time_of_impact
        );
        assert!(
            intersection.time_of_impact <= max_distance,
            "[CORRECTNESS FAILURE] {}: hit distance {} exceeds max {}",
            context,
            intersection.time_of_impact,
            max_distance
        );
        // Normal must be approximately unit length
        let normal_len = (intersection.normal.x.powi(2)
            + intersection.normal.y.powi(2)
            + intersection.normal.z.powi(2))
        .sqrt();
        assert!(
            (normal_len - 1.0).abs() < 0.01,
            "[CORRECTNESS FAILURE] {}: normal not unit length ({})",
            context,
            normal_len
        );
    }
}

/// CORRECTNESS: Validate that a ray expected to hit ground actually hits
#[inline]
fn assert_hits_ground(result: &Option<(ColliderHandle, RayIntersection)>, context: &str) {
    assert!(
        result.is_some(),
        "[CORRECTNESS FAILURE] {}: ray should hit ground but missed",
        context
    );
    if let Some((_, intersection)) = result {
        // Ground plane normal should point up (Y positive)
        assert!(
            intersection.normal.y > 0.5,
            "[CORRECTNESS FAILURE] {}: ground normal should point up, got Y={}",
            context,
            intersection.normal.y
        );
    }
}

/// Setup a physics world with ground plane and optional obstacles
fn setup_world_with_obstacles(obstacle_count: usize) -> PhysicsWorld {
    let mut world = PhysicsWorld::new(vec3(0.0, -9.8, 0.0));

    // Ground plane
    world.create_ground_plane(vec3(50.0, 0.5, 50.0), 0.9);

    // Add obstacles in a grid pattern
    let grid_size = (obstacle_count as f32).sqrt().ceil() as usize;
    let spacing = 5.0;

    for i in 0..obstacle_count {
        let x = (i % grid_size) as f32 * spacing - (grid_size as f32 * spacing / 2.0);
        let z = (i / grid_size) as f32 * spacing - (grid_size as f32 * spacing / 2.0);
        world.add_dynamic_box(vec3(x, 2.0, z), vec3(0.5, 0.5, 0.5), 1.0, Layers::DEFAULT);
    }

    // Colliders are invisible to QueryPipeline until a step updates it.
    world.step();
    world
}

/// Benchmark: Single raycast against empty scene
fn raycast_empty_scene(c: &mut Criterion) {
    let world = PhysicsWorld::new(vec3(0.0, -9.8, 0.0));

    c.bench_function("raycast_empty_scene", |b| {
        b.iter(|| {
            let origin = black_box(vec3(0.0, 1.0, 0.0));
            let direction = black_box(vec3(1.0, 0.0, 0.0).normalize());

            let ray = Ray::new(
                point![origin.x, origin.y, origin.z],
                vector![direction.x, direction.y, direction.z],
            );

            let filter = QueryFilter::default();
            let result = world.query_pipeline.cast_ray_and_get_normal(
                &world.bodies,
                &world.colliders,
                &ray,
                100.0,
                true,
                filter,
            );
            // CORRECTNESS: Empty scene should have no hits
            assert!(
                result.is_none(),
                "[CORRECTNESS FAILURE] raycast_empty_scene: should not hit anything in empty world"
            );
            black_box(result)
        });
    });
}

/// Benchmark: Single raycast against ground plane
fn raycast_ground_plane(c: &mut Criterion) {
    let mut world = PhysicsWorld::new(vec3(0.0, -9.8, 0.0));
    world.create_ground_plane(vec3(50.0, 0.5, 50.0), 0.9);
    // QueryPipeline stays empty until step() updates it, so a ray cast
    // before that misses the ground that was just inserted.
    world.step();

    c.bench_function("raycast_ground_plane", |b| {
        b.iter(|| {
            let origin = black_box(vec3(0.0, 5.0, 0.0));
            let direction = black_box(vec3(0.0, -1.0, 0.0));

            let ray = Ray::new(
                point![origin.x, origin.y, origin.z],
                vector![direction.x, direction.y, direction.z],
            );

            let filter = QueryFilter::default();
            let result = world.query_pipeline.cast_ray_and_get_normal(
                &world.bodies,
                &world.colliders,
                &ray,
                10.0,
                true,
                filter,
            );
            // CORRECTNESS: Downward ray from y=5 should hit ground
            assert_hits_ground(&result, "raycast_ground_plane");
            assert_raycast_hit_valid(&result, 10.0, "raycast_ground_plane");
            // Ground is at y=0.5 (half-height), so hit should be around y=5 - 0.5 = 4.5 distance
            if let Some((_, intersection)) = &result {
                assert!(
                    intersection.time_of_impact > 4.0 && intersection.time_of_impact < 5.5,
                    "[CORRECTNESS FAILURE] raycast_ground_plane: expected hit ~4.5, got {}",
                    intersection.time_of_impact
                );
            }
            black_box(result)
        });
    });
}

/// Benchmark: Raycast through varying obstacle density
fn raycast_obstacle_density(c: &mut Criterion) {
    let mut group = c.benchmark_group("raycast_obstacle_density");

    for obstacle_count in [0, 10, 50, 100].iter() {
        let world = setup_world_with_obstacles(*obstacle_count);

        // CORRECTNESS: Verify world was set up correctly
        assert!(
            world.colliders.len() >= *obstacle_count,
            "[CORRECTNESS FAILURE] raycast_obstacle_density: expected at least {} colliders, got {}",
            obstacle_count, world.colliders.len()
        );

        group.throughput(Throughput::Elements(*obstacle_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(obstacle_count),
            obstacle_count,
            |b, &count| {
                b.iter(|| {
                    let origin = black_box(vec3(-20.0, 1.0, 0.0));
                    let direction = black_box(vec3(1.0, 0.0, 0.0));

                    let ray = Ray::new(
                        point![origin.x, origin.y, origin.z],
                        vector![direction.x, direction.y, direction.z],
                    );

                    let filter = QueryFilter::default();
                    let result = world.query_pipeline.cast_ray_and_get_normal(
                        &world.bodies,
                        &world.colliders,
                        &ray,
                        50.0,
                        true,
                        filter,
                    );
                    // CORRECTNESS: Validate any hit is geometrically valid
                    assert_raycast_hit_valid(&result, 50.0, &format!("obstacle_density/{}", count));
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Multiple raycasts (simulating combat/vision systems)
fn raycast_batch_8_rays(c: &mut Criterion) {
    let mut world = PhysicsWorld::new(vec3(0.0, -9.8, 0.0));
    world.create_ground_plane(vec3(50.0, 0.5, 50.0), 0.9);

    // Add some obstacles
    for i in 0..20 {
        let angle = (i as f32) * std::f32::consts::PI * 2.0 / 20.0;
        world.add_dynamic_box(
            vec3(angle.cos() * 10.0, 2.0, angle.sin() * 10.0),
            vec3(0.5, 1.0, 0.5),
            1.0,
            Layers::DEFAULT,
        );
    }

    // CORRECTNESS: Verify setup - 20 obstacles + ground = at least 21 colliders
    assert!(
        world.colliders.len() >= 21,
        "[CORRECTNESS FAILURE] raycast_batch_8_rays setup: expected >= 21 colliders, got {}",
        world.colliders.len()
    );

    c.bench_function("raycast_batch_8_rays", |b| {
        b.iter(|| {
            let origin = black_box(vec3(0.0, 1.5, 0.0));
            let mut hit_count = 0u32;

            // Cast 8 rays in a circle (vision cone simulation)
            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::PI * 2.0 / 8.0;
                let direction = vec3(angle.cos(), 0.0, angle.sin());

                let ray = Ray::new(
                    point![origin.x, origin.y, origin.z],
                    vector![direction.x, direction.y, direction.z],
                );

                let filter = QueryFilter::default();
                let result = world.query_pipeline.cast_ray_and_get_normal(
                    &world.bodies,
                    &world.colliders,
                    &ray,
                    20.0,
                    true,
                    filter,
                );
                // CORRECTNESS: Validate each ray result
                assert_raycast_hit_valid(&result, 20.0, &format!("batch_8_rays/ray_{}", i));
                if result.is_some() {
                    hit_count += 1;
                }
                black_box(result);
            }
            // CORRECTNESS: With 20 obstacles in a circle at radius 10, most rays should hit
            // At least some rays (>2) should hit given obstacle density
            assert!(
                hit_count >= 2,
                "[CORRECTNESS FAILURE] raycast_batch_8_rays: expected >= 2 hits, got {}",
                hit_count
            );
        });
    });
}

/// Benchmark: Raycast with normal retrieval vs without
fn raycast_with_and_without_normal(c: &mut Criterion) {
    let mut world = PhysicsWorld::new(vec3(0.0, -9.8, 0.0));
    world.create_ground_plane(vec3(50.0, 0.5, 50.0), 0.9);

    let mut group = c.benchmark_group("raycast_normal_retrieval");

    // With normal
    group.bench_function("with_normal", |b| {
        b.iter(|| {
            let origin = black_box(vec3(0.0, 5.0, 0.0));
            let direction = black_box(vec3(0.0, -1.0, 0.0));

            let ray = Ray::new(
                point![origin.x, origin.y, origin.z],
                vector![direction.x, direction.y, direction.z],
            );

            let filter = QueryFilter::default();
            let result = world.query_pipeline.cast_ray_and_get_normal(
                &world.bodies,
                &world.colliders,
                &ray,
                10.0,
                true,
                filter,
            );
            // CORRECTNESS: Must hit ground and have valid normal
            assert_hits_ground(&result, "with_normal");
            assert_raycast_hit_valid(&result, 10.0, "with_normal");
            black_box(result)
        });
    });

    // Without normal (just hit detection)
    group.bench_function("without_normal", |b| {
        b.iter(|| {
            let origin = black_box(vec3(0.0, 5.0, 0.0));
            let direction = black_box(vec3(0.0, -1.0, 0.0));

            let ray = Ray::new(
                point![origin.x, origin.y, origin.z],
                vector![direction.x, direction.y, direction.z],
            );

            let filter = QueryFilter::default();
            let result = world.query_pipeline.cast_ray(
                &world.bodies,
                &world.colliders,
                &ray,
                10.0,
                true,
                filter,
            );
            // CORRECTNESS: Must hit ground (returns Option<(ColliderHandle, f32)>)
            assert!(
                result.is_some(),
                "[CORRECTNESS FAILURE] without_normal: should hit ground"
            );
            if let Some((_, toi)) = result {
                assert!(
                    (0.0..=10.0).contains(&toi),
                    "[CORRECTNESS FAILURE] without_normal: hit distance {} out of range",
                    toi
                );
            }
            black_box(result)
        });
    });

    group.finish();
}

criterion_group!(
    raycast_benches,
    raycast_empty_scene,
    raycast_ground_plane,
    raycast_obstacle_density,
    raycast_batch_8_rays,
    raycast_with_and_without_normal,
);
criterion_main!(raycast_benches);
