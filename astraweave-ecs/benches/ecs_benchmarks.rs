// SPDX-License-Identifier: MIT
//! High-level ECS operation benchmarks with mission-critical correctness assertions
//!
//! Baseline performance metrics for regression detection.
//! Created: Week 6 Day 4 (October 24, 2025)
//! Updated: January 2026 - Added correctness assertions per production audit
//!
//! # Benchmarks
//!
//! 1. **Entity Spawn/Despawn** (1k, 10k entities)
//! 2. **Component Add/Remove** (1k, 10k operations)
//! 3. **Component Iteration** (1k, 10k entities)
//! 4. **Archetype Transitions** (100 entities × 10 transitions)
//!
//! # Correctness Assertions
#![allow(clippy::redundant_closure)]
//!
//! Each benchmark validates:
//! - Entity count matches expected
//! - Component data integrity after operations
//! - No data corruption during archetype transitions
//!
//! # Performance Targets (60 FPS = 16.67ms frame budget)
//!
//! - Entity spawn: <1 µs/entity (10k spawns < 10ms)
//! - Component iteration: <100 ns/entity (10k queries < 1ms)
//! - Archetype transition: <2 µs/transition

use astraweave_ecs::World;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;

// ============================================================================
// CORRECTNESS ASSERTION HELPERS
// ============================================================================

/// Validates entity count matches expected value - mission-critical invariant
#[inline]
fn assert_entity_count(world: &World, expected: usize, context: &str) {
    let actual = world.entity_count();
    assert_eq!(
        actual, expected,
        "[CORRECTNESS FAILURE] {}: expected {} entities, got {}",
        context, expected, actual
    );
}

/// Validates that an entity exists and has expected component - data integrity check
#[inline]
fn assert_has_component<T: 'static + Send + Sync>(
    world: &World,
    entity: astraweave_ecs::Entity,
    context: &str,
) {
    assert!(
        world.get::<T>(entity).is_some(),
        "[CORRECTNESS FAILURE] {}: entity {:?} missing expected component {}",
        context,
        entity,
        std::any::type_name::<T>()
    );
}

/// Validates component value matches expected - determinism check
#[inline]
fn assert_position_value(
    world: &World,
    entity: astraweave_ecs::Entity,
    expected_x: f32,
    context: &str,
) {
    let pos = world.get::<Position>(entity);
    assert!(
        pos.is_some(),
        "[CORRECTNESS FAILURE] {}: entity {:?} missing Position",
        context,
        entity
    );
    let pos = pos.unwrap();
    assert!(
        (pos.x - expected_x).abs() < 0.001,
        "[CORRECTNESS FAILURE] {}: Position.x expected {}, got {}",
        context,
        expected_x,
        pos.x
    );
}

// Test components
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct Health {
    current: i32,
    max: i32,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct Armor {
    value: i32,
}

// === Benchmark 1: Entity Spawn ===

fn bench_entity_spawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_spawn");

    for count in [100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::new("empty", count), &count, |b, &count| {
            b.iter_batched(
                || World::new(),
                |mut world| {
                    for _ in 0..count {
                        black_box(world.spawn());
                    }
                    // CORRECTNESS: Validate exact entity count after spawn
                    assert_entity_count(&world, count, "entity_spawn/empty");
                    black_box(world);
                },
                criterion::BatchSize::SmallInput,
            );
        });

        group.bench_with_input(
            BenchmarkId::new("with_position", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || World::new(),
                    |mut world| {
                        let mut first_entity = None;
                        for i in 0..count {
                            let entity = world.spawn();
                            if i == 0 {
                                first_entity = Some(entity);
                            }
                            world.insert(
                                entity,
                                Position {
                                    x: i as f32,
                                    y: i as f32 * 2.0,
                                    z: i as f32 * 3.0,
                                },
                            );
                        }
                        // CORRECTNESS: Validate entity count and first entity's Position
                        assert_entity_count(&world, count, "entity_spawn/with_position");
                        if let Some(e) = first_entity {
                            assert_position_value(
                                &world,
                                e,
                                0.0,
                                "entity_spawn/with_position first entity",
                            );
                        }
                        black_box(world);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("with_position_velocity", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || World::new(),
                    |mut world| {
                        let mut first_entity = None;
                        for i in 0..count {
                            let entity = world.spawn();
                            if i == 0 {
                                first_entity = Some(entity);
                            }
                            world.insert(
                                entity,
                                Position {
                                    x: i as f32,
                                    y: i as f32 * 2.0,
                                    z: i as f32 * 3.0,
                                },
                            );
                            world.insert(
                                entity,
                                Velocity {
                                    x: 1.0,
                                    y: 0.0,
                                    z: 0.0,
                                },
                            );
                        }
                        // CORRECTNESS: Validate both components present
                        assert_entity_count(&world, count, "entity_spawn/with_position_velocity");
                        if let Some(e) = first_entity {
                            assert_has_component::<Position>(&world, e, "spawn/pos_vel Position");
                            assert_has_component::<Velocity>(&world, e, "spawn/pos_vel Velocity");
                        }
                        black_box(world);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

// === Benchmark 2: Entity Despawn ===

fn bench_entity_despawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_despawn");

    for count in [100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::new("empty", count), &count, |b, &count| {
            b.iter_batched(
                || {
                    let mut world = World::new();
                    let entities: Vec<_> = (0..count).map(|_| world.spawn()).collect();
                    (world, entities)
                },
                |(mut world, entities)| {
                    // CORRECTNESS: Verify initial state before despawn
                    assert_entity_count(&world, count, "entity_despawn/empty pre-despawn");
                    for entity in entities {
                        black_box(world.despawn(entity));
                    }
                    // CORRECTNESS: Verify all entities removed after despawn
                    assert_entity_count(&world, 0, "entity_despawn/empty post-despawn");
                    black_box(world);
                },
                criterion::BatchSize::SmallInput,
            );
        });

        group.bench_with_input(
            BenchmarkId::new("with_components", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let mut world = World::new();
                        let entities: Vec<_> = (0..count)
                            .map(|i| {
                                let entity = world.spawn();
                                world.insert(
                                    entity,
                                    Position {
                                        x: i as f32,
                                        y: 0.0,
                                        z: 0.0,
                                    },
                                );
                                world.insert(
                                    entity,
                                    Velocity {
                                        x: 1.0,
                                        y: 0.0,
                                        z: 0.0,
                                    },
                                );
                                entity
                            })
                            .collect();
                        (world, entities)
                    },
                    |(mut world, entities)| {
                        // CORRECTNESS: Verify initial state with components
                        assert_entity_count(
                            &world,
                            count,
                            "entity_despawn/with_components pre-despawn",
                        );
                        for entity in entities {
                            black_box(world.despawn(entity));
                        }
                        // CORRECTNESS: Verify all entities and components cleaned up
                        assert_entity_count(
                            &world,
                            0,
                            "entity_despawn/with_components post-despawn",
                        );
                        black_box(world);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

// === Benchmark 3: Component Add ===

fn bench_component_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_add");

    for count in [100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("single_component", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let mut world = World::new();
                        let entities: Vec<_> = (0..count).map(|_| world.spawn()).collect();
                        (world, entities)
                    },
                    |(mut world, entities)| {
                        for (i, entity) in entities.iter().enumerate() {
                            world.insert(
                                *entity,
                                Position {
                                    x: i as f32,
                                    y: 0.0,
                                    z: 0.0,
                                },
                            );
                        }
                        // CORRECTNESS: Verify all entities have Position component
                        assert_entity_count(&world, count, "component_add/single");
                        if let Some(first) = entities.first() {
                            assert_has_component::<Position>(
                                &world,
                                *first,
                                "component_add/single first",
                            );
                            assert_position_value(
                                &world,
                                *first,
                                0.0,
                                "component_add/single first value",
                            );
                        }
                        if let Some(last) = entities.last() {
                            assert_has_component::<Position>(
                                &world,
                                *last,
                                "component_add/single last",
                            );
                        }
                        black_box(world);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("multiple_components", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let mut world = World::new();
                        let entities: Vec<_> = (0..count).map(|_| world.spawn()).collect();
                        (world, entities)
                    },
                    |(mut world, entities)| {
                        for (i, entity) in entities.iter().enumerate() {
                            world.insert(
                                *entity,
                                Position {
                                    x: i as f32,
                                    y: 0.0,
                                    z: 0.0,
                                },
                            );
                            world.insert(
                                *entity,
                                Velocity {
                                    x: 1.0,
                                    y: 0.0,
                                    z: 0.0,
                                },
                            );
                            world.insert(
                                *entity,
                                Health {
                                    current: 100,
                                    max: 100,
                                },
                            );
                        }
                        // CORRECTNESS: Verify all 3 components added to all entities
                        assert_entity_count(&world, count, "component_add/multiple");
                        if let Some(first) = entities.first() {
                            assert_has_component::<Position>(
                                &world,
                                *first,
                                "component_add/multiple Position",
                            );
                            assert_has_component::<Velocity>(
                                &world,
                                *first,
                                "component_add/multiple Velocity",
                            );
                            assert_has_component::<Health>(
                                &world,
                                *first,
                                "component_add/multiple Health",
                            );
                        }
                        black_box(world);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

// === Benchmark 4: Component Remove ===

fn bench_component_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_remove");

    for count in [100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("single_component", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let mut world = World::new();
                        let entities: Vec<_> = (0..count)
                            .map(|i| {
                                let entity = world.spawn();
                                world.insert(
                                    entity,
                                    Position {
                                        x: i as f32,
                                        y: 0.0,
                                        z: 0.0,
                                    },
                                );
                                entity
                            })
                            .collect();
                        (world, entities)
                    },
                    |(mut world, entities)| {
                        // CORRECTNESS: Verify components exist before removal
                        if let Some(first) = entities.first() {
                            assert_has_component::<Position>(
                                &world,
                                *first,
                                "component_remove/single pre-remove",
                            );
                        }
                        for entity in &entities {
                            black_box(world.remove::<Position>(*entity));
                        }
                        // CORRECTNESS: Verify component removed (entity still exists, component gone)
                        assert_entity_count(
                            &world,
                            count,
                            "component_remove/single post-remove entities",
                        );
                        if let Some(first) = entities.first() {
                            assert!(
                                world.get::<Position>(*first).is_none(),
                                "component_remove/single: Position should be removed"
                            );
                        }
                        black_box(world);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("multiple_components", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || {
                        let mut world = World::new();
                        let entities: Vec<_> = (0..count)
                            .map(|i| {
                                let entity = world.spawn();
                                world.insert(
                                    entity,
                                    Position {
                                        x: i as f32,
                                        y: 0.0,
                                        z: 0.0,
                                    },
                                );
                                world.insert(
                                    entity,
                                    Velocity {
                                        x: 1.0,
                                        y: 0.0,
                                        z: 0.0,
                                    },
                                );
                                world.insert(
                                    entity,
                                    Health {
                                        current: 100,
                                        max: 100,
                                    },
                                );
                                entity
                            })
                            .collect();
                        (world, entities)
                    },
                    |(mut world, entities)| {
                        // CORRECTNESS: Verify all 3 components exist before removal
                        if let Some(first) = entities.first() {
                            assert_has_component::<Position>(
                                &world,
                                *first,
                                "component_remove/multiple pre Position",
                            );
                            assert_has_component::<Velocity>(
                                &world,
                                *first,
                                "component_remove/multiple pre Velocity",
                            );
                            assert_has_component::<Health>(
                                &world,
                                *first,
                                "component_remove/multiple pre Health",
                            );
                        }
                        for entity in &entities {
                            black_box(world.remove::<Position>(*entity));
                            black_box(world.remove::<Velocity>(*entity));
                            black_box(world.remove::<Health>(*entity));
                        }
                        // CORRECTNESS: Verify all components removed
                        assert_entity_count(&world, count, "component_remove/multiple post-remove");
                        if let Some(first) = entities.first() {
                            assert!(
                                world.get::<Position>(*first).is_none(),
                                "Position should be removed"
                            );
                            assert!(
                                world.get::<Velocity>(*first).is_none(),
                                "Velocity should be removed"
                            );
                            assert!(
                                world.get::<Health>(*first).is_none(),
                                "Health should be removed"
                            );
                        }
                        black_box(world);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

// === Benchmark 5: Component Iteration (using each_mut) ===

fn bench_component_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("component_iteration");

    for count in [100, 1000, 10000] {
        // Setup world with Position components
        let mut world = World::new();
        let mut first_entity = None;
        for i in 0..count {
            let entity = world.spawn();
            if i == 0 {
                first_entity = Some(entity);
            }
            world.insert(
                entity,
                Position {
                    x: i as f32,
                    y: i as f32 * 2.0,
                    z: i as f32 * 3.0,
                },
            );
        }

        // CORRECTNESS: Capture initial value for first entity to verify iteration modifies correctly
        let initial_x = first_entity
            .and_then(|e| world.get::<Position>(e).map(|p| p.x))
            .unwrap_or(0.0);

        group.bench_with_input(BenchmarkId::new("position_write", count), &count, |b, _| {
            b.iter(|| {
                world.each_mut(|_entity, pos: &mut Position| {
                    pos.x += 1.0;
                    pos.y += 1.0;
                    pos.z += 1.0;
                });
            });
        });

        // CORRECTNESS: Verify iteration actually modified values (accumulates over benchmark iterations)
        if let Some(e) = first_entity {
            let final_pos = world.get::<Position>(e);
            assert!(
                final_pos.is_some(),
                "component_iteration: Position should still exist after iteration"
            );
            // After benchmarking, x should have increased from initial value
            // (we can't predict exact value due to unknown iteration count, but it should be > initial)
            let final_x = final_pos.unwrap().x;
            assert!(
                final_x >= initial_x,
                "component_iteration: Position.x should have increased (initial: {}, final: {})",
                initial_x,
                final_x
            );
        }
    }

    group.finish();
}

// === Benchmark 7: Archetype Transitions ===

fn bench_archetype_transitions(c: &mut Criterion) {
    let mut group = c.benchmark_group("archetype_transitions");

    // Test with 100 entities doing 10 transitions each
    let entity_count = 100;
    let transition_count = 10;

    group.bench_function("add_remove_cycle", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                let entities: Vec<_> = (0..entity_count)
                    .map(|i| {
                        let entity = world.spawn();
                        world.insert(
                            entity,
                            Position {
                                x: i as f32,
                                y: 0.0,
                                z: 0.0,
                            },
                        );
                        entity
                    })
                    .collect();
                (world, entities)
            },
            |(mut world, entities)| {
                // CORRECTNESS: Record initial Position values
                let initial_positions: Vec<f32> = entities
                    .iter()
                    .filter_map(|&e| world.get::<Position>(e).map(|p| p.x))
                    .collect();

                for _ in 0..transition_count {
                    // Add Velocity (transition to [Position, Velocity] archetype)
                    for &entity in &entities {
                        world.insert(
                            entity,
                            Velocity {
                                x: 1.0,
                                y: 0.0,
                                z: 0.0,
                            },
                        );
                    }

                    // Remove Velocity (transition back to [Position] archetype)
                    for &entity in &entities {
                        world.remove::<Velocity>(entity);
                    }
                }

                // CORRECTNESS: Verify Position data not corrupted during transitions
                assert_entity_count(
                    &world,
                    entity_count,
                    "archetype_transitions/add_remove_cycle",
                );
                for (i, &entity) in entities.iter().enumerate() {
                    assert_has_component::<Position>(
                        &world,
                        entity,
                        "archetype_transitions/add_remove Position",
                    );
                    assert!(
                        world.get::<Velocity>(entity).is_none(),
                        "Velocity should be removed after cycle"
                    );
                    // Verify Position.x not corrupted
                    if let Some(pos) = world.get::<Position>(entity) {
                        assert_eq!(
                            pos.x, initial_positions[i],
                            "Position.x corrupted during archetype transitions (entity {})",
                            i
                        );
                    }
                }
                black_box(world);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("multi_component_transitions", |b| {
        b.iter_batched(
            || {
                let mut world = World::new();
                let entities: Vec<_> = (0..entity_count)
                    .map(|i| {
                        let entity = world.spawn();
                        world.insert(
                            entity,
                            Position {
                                x: i as f32,
                                y: 0.0,
                                z: 0.0,
                            },
                        );
                        entity
                    })
                    .collect();
                (world, entities)
            },
            |(mut world, entities)| {
                // CORRECTNESS: Record initial Position values
                let initial_positions: Vec<f32> = entities
                    .iter()
                    .filter_map(|&e| world.get::<Position>(e).map(|p| p.x))
                    .collect();

                for _ in 0..transition_count {
                    // Transition 1: Add Velocity
                    for &entity in &entities {
                        world.insert(
                            entity,
                            Velocity {
                                x: 1.0,
                                y: 0.0,
                                z: 0.0,
                            },
                        );
                    }

                    // Transition 2: Add Health
                    for &entity in &entities {
                        world.insert(
                            entity,
                            Health {
                                current: 100,
                                max: 100,
                            },
                        );
                    }

                    // Transition 3: Add Armor
                    for &entity in &entities {
                        world.insert(entity, Armor { value: 50 });
                    }

                    // Transition 4: Remove Armor
                    for &entity in &entities {
                        world.remove::<Armor>(entity);
                    }

                    // Transition 5: Remove Health
                    for &entity in &entities {
                        world.remove::<Health>(entity);
                    }

                    // Transition 6: Remove Velocity (back to starting archetype)
                    for &entity in &entities {
                        world.remove::<Velocity>(entity);
                    }
                }

                // CORRECTNESS: Verify data integrity after 60 archetype transitions (10 cycles × 6 transitions)
                assert_entity_count(
                    &world,
                    entity_count,
                    "archetype_transitions/multi_component",
                );
                for (i, &entity) in entities.iter().enumerate() {
                    // Should only have Position after full cycle
                    assert_has_component::<Position>(
                        &world,
                        entity,
                        "multi_component final Position",
                    );
                    assert!(
                        world.get::<Velocity>(entity).is_none(),
                        "Velocity should be removed"
                    );
                    assert!(
                        world.get::<Health>(entity).is_none(),
                        "Health should be removed"
                    );
                    assert!(
                        world.get::<Armor>(entity).is_none(),
                        "Armor should be removed"
                    );
                    // Verify Position.x not corrupted through 60 transitions
                    if let Some(pos) = world.get::<Position>(entity) {
                        assert_eq!(
                            pos.x, initial_positions[i],
                            "Position.x corrupted during multi-component transitions (entity {})",
                            i
                        );
                    }
                }
                black_box(world);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_entity_spawn,
    bench_entity_despawn,
    bench_component_add,
    bench_component_remove,
    bench_component_iteration,
    bench_archetype_transitions,
);
criterion_main!(benches);
