//! Determinism validation tests for AI-native gameplay.
//!
//! # Overview
//!
//! AstraWeave's AI agents depend on **deterministic execution** for:
//! - Reproducible AI behavior (same inputs → same actions)
//! - Network synchronization (lockstep multiplayer)
//! - Replay systems (record/playback for debugging)
//! - Regression testing (validate AI changes don't break behavior)
//!
//! This module validates that the ECS provides deterministic guarantees.
//!
//! # What is Determinism?
//!
//! **Determinism** means: given the same initial state and same sequence of operations,
//! the system produces identical results every time.
//!
//! For AstraWeave's AI systems, this means:
//! - Entities iterated in consistent order **per archetype**
//! - Archetype iteration order deterministic (by ArchetypeId)
//! - Same operations produce same entity→archetype assignments
//! - Events delivered in FIFO order
//! - Archetype IDs assigned consistently
//!
//! # Ordering Guarantees
//!
//! **CRITICAL LIMITATION**: Entity spawn order is **not preserved across archetypes**.
//!
//! ## Current Behavior
//!
//! Entities are stored per-archetype, and iteration visits archetypes by ID:
//!
//! ```rust,ignore
//! let e1 = world.spawn();  // Empty archetype
//! let e2 = world.spawn();  // Empty archetype
//! world.insert(e1, Position { x: 1.0, y: 1.0 });  // Moves e1 to Position archetype
//!
//! // Iteration order: [e2, e1] (e2 still in empty archetype, e1 in Position archetype)
//! // NOT spawn order [e1, e2]!
//! ```
//!
//! **Why?**: When an entity changes archetypes (insert/remove component), it's **appended**
//! to the new archetype's entity list. This breaks spawn order but is **deterministic**
//! (same operations → same order every time).
//!
//! ## What IS Guaranteed
//!
//! 1. ✅ **Archetype iteration order**: Archetypes visited in ID order (BTreeMap)
//! 2. ✅ **Within-archetype order**: Entities in same archetype maintain relative order
//! 3. ✅ **Repeated iterations**: Same iteration order every time
//! 4. ✅ **Cross-world consistency**: Same operations → same archetype IDs
//!
//! ## What is NOT Guaranteed
//!
//! 1. ❌ **Spawn order across archetypes**: Entity order changes when components added/removed
//! 2. ❌ **Spawn order after archetype changes**: Moving archetypes breaks spawn order
//!
//! # Implication for AI Systems
//!
//! **This is acceptable for most AI use cases**:
//! - AI systems typically query entities **by component type** (e.g., "all enemies")
//! - Query results are deterministic within that component's archetypes
//! - If spawn order is critical, track explicitly via `SpawnOrder` component
//!
//! **Example workaround** (if spawn order needed):
//! ```rust,ignore
//! #[derive(Clone, Copy)]
//! struct SpawnOrder(u64);
//!
//! let e = world.spawn();
//! world.insert(e, SpawnOrder(world.entity_count() as u64));
//!
//! // Later: Query and sort by SpawnOrder
//! let mut entities = world.entities_with::<SpawnOrder>();
//! entities.sort_by_key(|&e| world.get::<SpawnOrder>(e).unwrap().0);
//! ```
//!
//! # Why is this Critical?
//!
//! **Example: Combat AI without determinism**:
//! ```rust,ignore
//! // Run 1: Entities iterated in random order
//! for entity in world.entities() {
//!     if can_attack(entity) {
//!         attack_target(entity);  // First entity found attacks
//!         break;
//!     }
//! }
//! // Result: Entity 42 attacks (random HashMap iteration order)
//!
//! // Run 2: Same world state, different iteration order
//! // Result: Entity 17 attacks (different outcome! 💥)
//! ```
//!
//! **With determinism**:
//! - Same entity order every run (by archetype ID)
//! - AI makes same decisions
//! - Multiplayer stays in sync
//! - Replays work correctly
//!
//! # Test Categories
//!
//! 1. **Entity Ordering**: Consistent iteration order (deterministic, not spawn order)
//! 2. **Archetype Stability**: Same components → same archetype ID
//! 3. **Component Modification**: Deterministic archetype transitions
//! 4. **Despawn/Respawn Cycles**: Generation increments work correctly
//! 5. **Query Iteration**: Queries return entities in consistent order

use crate::{Entity, World};
use std::collections::HashSet;

// === Test Components ===

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
struct Velocity {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Health {
    current: i32,
    max: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
struct Tag;

// === Helper Functions ===

/// Collect all entities from a world into a Vec.
///
/// Note: Currently uses archetype iteration, which may not preserve spawn order.
/// This is the behavior we're testing!
fn collect_entities(world: &World) -> Vec<Entity> {
    let mut entities = Vec::new();
    for archetype in world.archetypes().iter() {
        for &entity in archetype.entities_vec() {
            if world.is_alive(entity) {
                entities.push(entity);
            }
        }
    }
    entities
}

// === Entity Spawn Ordering Tests ===

#[test]
fn test_spawn_order_preserved() {
    let mut world = World::new();

    // Spawn 100 entities
    let spawned: Vec<Entity> = (0..100).map(|_| world.spawn()).collect();

    // Collect entities (should match spawn order)
    let collected = collect_entities(&world);

    assert_eq!(collected.len(), 100, "All 100 entities should be present");

    // Check spawn order preserved
    for (i, &entity) in collected.iter().enumerate() {
        assert_eq!(
            entity, spawned[i],
            "Entity at index {} should match spawn order (expected {:?}, got {:?})",
            i, spawned[i], entity
        );
    }
}

#[test]
fn test_spawn_order_with_components() {
    let mut world = World::new();

    // Spawn entities with components
    let e1 = world.spawn();
    world.insert(e1, Position { x: 1.0, y: 1.0 });

    let e2 = world.spawn();
    world.insert(e2, Velocity { x: 2.0, y: 2.0 });

    let e3 = world.spawn();
    world.insert(e3, Position { x: 3.0, y: 3.0 });
    world.insert(e3, Velocity { x: 3.0, y: 3.0 });

    let expected_order = [e1, e2, e3];

    // Collect entities
    let collected = collect_entities(&world);

    assert_eq!(collected.len(), 3, "Should have 3 entities");

    // Check spawn order preserved even with different component combinations
    for (entity, expected) in collected.iter().zip(expected_order.iter()) {
        assert_eq!(entity, expected, "Entity should match spawn order");
    }
}

#[test]
fn test_spawn_order_after_component_modifications() {
    let mut world = World::new();

    // Spawn entities
    let entities: Vec<Entity> = (0..10).map(|_| world.spawn()).collect();

    // Add components to some entities (changes archetypes)
    for (i, &entity) in entities.iter().enumerate() {
        if i % 2 == 0 {
            world.insert(
                entity,
                Position {
                    x: i as f32,
                    y: i as f32,
                },
            );
        }
        if i % 3 == 0 {
            world.insert(
                entity,
                Velocity {
                    x: i as f32,
                    y: i as f32,
                },
            );
        }
    }

    // Collect entities
    let collected = collect_entities(&world);

    assert_eq!(collected.len(), 10, "Should have 10 entities");

    // Check all entities present (order may differ from spawn order due to archetype changes)
    let collected_set: HashSet<Entity> = collected.iter().copied().collect();
    for &entity in entities.iter() {
        assert!(
            collected_set.contains(&entity),
            "Entity {:?} should be present after component modifications",
            entity
        );
    }

    // Check order is deterministic: same world state → same iteration order
    let collected2 = collect_entities(&world);
    assert_eq!(
        collected, collected2,
        "Repeated iteration should produce identical order"
    );
}

// === Despawn/Respawn Ordering Tests ===

#[test]
fn test_despawn_respawn_ordering() {
    let mut world = World::new();

    // Spawn initial entities
    let e1 = world.spawn();
    let e2 = world.spawn();
    let e3 = world.spawn();

    // Despawn middle entity
    world.despawn(e2);

    // Respawn (ID recycled, generation incremented)
    let e4 = world.spawn();

    // Collect alive entities
    let collected = collect_entities(&world);

    assert_eq!(collected.len(), 3, "Should have 3 alive entities");

    // Check e2 is not in collected (stale generation)
    assert!(
        !collected.contains(&e2),
        "Despawned entity should not appear in iteration"
    );

    // Check e1, e3, e4 are present
    assert!(collected.contains(&e1), "Entity e1 should be alive");
    assert!(collected.contains(&e3), "Entity e3 should be alive");
    assert!(
        collected.contains(&e4),
        "Respawned entity e4 should be alive"
    );

    // Check determinism: same operations → same order
    let collected2 = collect_entities(&world);
    assert_eq!(
        collected, collected2,
        "Repeated iteration should produce identical order"
    );
}

#[test]
fn test_multiple_despawn_respawn_cycles() {
    let mut world = World::new();

    // Spawn 5 entities
    let entities: Vec<Entity> = (0..5).map(|_| world.spawn()).collect();

    // Despawn entities 1 and 3
    world.despawn(entities[1]);
    world.despawn(entities[3]);

    // Respawn 2 entities (should recycle IDs)
    let e_new1 = world.spawn();
    let e_new2 = world.spawn();

    // Collect alive entities
    let collected = collect_entities(&world);

    assert_eq!(collected.len(), 5, "Should have 5 alive entities");

    // Check stale entities not present
    assert!(
        !collected.contains(&entities[1]),
        "Despawned entity[1] should not appear"
    );
    assert!(
        !collected.contains(&entities[3]),
        "Despawned entity[3] should not appear"
    );

    // Check alive entities present
    assert!(
        collected.contains(&entities[0]),
        "Entity[0] should be alive"
    );
    assert!(
        collected.contains(&entities[2]),
        "Entity[2] should be alive"
    );
    assert!(
        collected.contains(&entities[4]),
        "Entity[4] should be alive"
    );
    assert!(collected.contains(&e_new1), "New entity 1 should be alive");
    assert!(collected.contains(&e_new2), "New entity 2 should be alive");

    // Check uniqueness (no duplicates)
    let unique: HashSet<Entity> = collected.iter().copied().collect();
    assert_eq!(unique.len(), 5, "All entities should be unique");
}

// === Archetype Stability Tests ===

#[test]
fn test_archetype_deterministic_assignment() {
    let mut world1 = World::new();
    let mut world2 = World::new();

    // Spawn entities with identical component combinations in both worlds
    let e1_w1 = world1.spawn();
    world1.insert(e1_w1, Position { x: 1.0, y: 1.0 });
    world1.insert(e1_w1, Velocity { x: 1.0, y: 1.0 });

    let e1_w2 = world2.spawn();
    world2.insert(e1_w2, Position { x: 1.0, y: 1.0 });
    world2.insert(e1_w2, Velocity { x: 1.0, y: 1.0 });

    // Get archetype IDs
    let arch_id1 = world1
        .archetypes()
        .get_entity_archetype(e1_w1)
        .expect("Entity should have archetype");
    let arch_id2 = world2
        .archetypes()
        .get_entity_archetype(e1_w2)
        .expect("Entity should have archetype");

    assert_eq!(
        arch_id1, arch_id2,
        "Same component combination should produce same archetype ID in different worlds"
    );
}

#[test]
fn test_archetype_stable_across_operations() {
    let mut world = World::new();

    // Spawn entity with components
    let e1 = world.spawn();
    world.insert(e1, Position { x: 1.0, y: 1.0 });
    world.insert(e1, Velocity { x: 1.0, y: 1.0 });

    let arch_id1 = world
        .archetypes()
        .get_entity_archetype(e1)
        .expect("Entity should have archetype");

    // Spawn another entity with same components
    let e2 = world.spawn();
    world.insert(e2, Position { x: 2.0, y: 2.0 });
    world.insert(e2, Velocity { x: 2.0, y: 2.0 });

    let arch_id2 = world
        .archetypes()
        .get_entity_archetype(e2)
        .expect("Entity should have archetype");

    assert_eq!(
        arch_id1, arch_id2,
        "Same component combination should reuse archetype"
    );

    // Remove component from e1, then add back
    world.remove::<Velocity>(e1);
    world.insert(e1, Velocity { x: 3.0, y: 3.0 });

    let arch_id1_restored = world
        .archetypes()
        .get_entity_archetype(e1)
        .expect("Entity should have archetype");

    assert_eq!(
        arch_id1, arch_id1_restored,
        "Restoring same component combination should use original archetype"
    );
}

// === Component Modification Ordering Tests ===

#[test]
fn test_component_add_preserves_spawn_order() {
    let mut world = World::new();

    // Spawn 20 entities
    let entities: Vec<Entity> = (0..20).map(|_| world.spawn()).collect();

    // Add Position component to all entities (changes archetype for all)
    for (i, &entity) in entities.iter().enumerate() {
        world.insert(
            entity,
            Position {
                x: i as f32,
                y: i as f32,
            },
        );
    }

    // Collect entities
    let collected = collect_entities(&world);

    assert_eq!(collected.len(), 20, "Should have 20 entities");

    // Check spawn order preserved
    for (i, &entity) in collected.iter().enumerate() {
        assert_eq!(
            entity, entities[i],
            "Entity at index {} should match spawn order after component add",
            i
        );
    }
}

#[test]
fn test_component_remove_preserves_spawn_order() {
    let mut world = World::new();

    // Spawn 20 entities with components
    let entities: Vec<Entity> = (0..20)
        .map(|i| {
            let e = world.spawn();
            world.insert(
                e,
                Position {
                    x: i as f32,
                    y: i as f32,
                },
            );
            world.insert(
                e,
                Velocity {
                    x: i as f32,
                    y: i as f32,
                },
            );
            e
        })
        .collect();

    // Remove Velocity component from all entities (changes archetype)
    for &entity in entities.iter() {
        world.remove::<Velocity>(entity);
    }

    // Collect entities
    let collected = collect_entities(&world);

    assert_eq!(collected.len(), 20, "Should have 20 entities");

    // Check spawn order preserved
    for (i, &entity) in collected.iter().enumerate() {
        assert_eq!(
            entity, entities[i],
            "Entity at index {} should match spawn order after component remove",
            i
        );
    }
}

#[test]
fn test_mixed_component_operations_preserve_order() {
    let mut world = World::new();

    // Spawn 10 entities
    let entities: Vec<Entity> = (0..10).map(|_| world.spawn()).collect();

    // Perform mixed operations
    for (i, &entity) in entities.iter().enumerate() {
        match i % 4 {
            0 => {
                // Add Position
                world.insert(
                    entity,
                    Position {
                        x: i as f32,
                        y: i as f32,
                    },
                );
            }
            1 => {
                // Add Position + Velocity
                world.insert(
                    entity,
                    Position {
                        x: i as f32,
                        y: i as f32,
                    },
                );
                world.insert(
                    entity,
                    Velocity {
                        x: i as f32,
                        y: i as f32,
                    },
                );
            }
            2 => {
                // Add Position + Velocity, then remove Velocity
                world.insert(
                    entity,
                    Position {
                        x: i as f32,
                        y: i as f32,
                    },
                );
                world.insert(
                    entity,
                    Velocity {
                        x: i as f32,
                        y: i as f32,
                    },
                );
                world.remove::<Velocity>(entity);
            }
            3 => {
                // Add Position + Health
                world.insert(
                    entity,
                    Position {
                        x: i as f32,
                        y: i as f32,
                    },
                );
                world.insert(
                    entity,
                    Health {
                        current: 100,
                        max: 100,
                    },
                );
            }
            _ => unreachable!(),
        }
    }

    // Collect entities
    let collected = collect_entities(&world);

    assert_eq!(collected.len(), 10, "Should have 10 entities");

    // Check all entities present (order may differ from spawn order)
    let collected_set: HashSet<Entity> = collected.iter().copied().collect();
    for &entity in entities.iter() {
        assert!(
            collected_set.contains(&entity),
            "Entity {:?} should be present after mixed operations",
            entity
        );
    }

    // Check order is deterministic
    let collected2 = collect_entities(&world);
    assert_eq!(
        collected, collected2,
        "Repeated iteration should produce identical order"
    );
}

// === Repeated Iteration Tests ===

#[test]
fn test_repeated_iteration_produces_same_order() {
    let mut world = World::new();

    // Spawn entities with mixed components
    for i in 0..50 {
        let e = world.spawn();
        if i % 2 == 0 {
            world.insert(
                e,
                Position {
                    x: i as f32,
                    y: i as f32,
                },
            );
        }
        if i % 3 == 0 {
            world.insert(
                e,
                Velocity {
                    x: i as f32,
                    y: i as f32,
                },
            );
        }
    }

    // Collect entities multiple times
    let collected1 = collect_entities(&world);
    let collected2 = collect_entities(&world);
    let collected3 = collect_entities(&world);

    assert_eq!(
        collected1, collected2,
        "First and second iteration should produce identical order"
    );
    assert_eq!(
        collected2, collected3,
        "Second and third iteration should produce identical order"
    );
}

// === Query Ordering Tests (Basic) ===

#[test]
fn test_query_iteration_deterministic() {
    let mut world = World::new();

    // Spawn entities with Position component
    let _entities: Vec<Entity> = (0..30)
        .map(|i| {
            let e = world.spawn();
            world.insert(
                e,
                Position {
                    x: i as f32,
                    y: i as f32,
                },
            );
            e
        })
        .collect();

    // Collect entities via direct iteration
    let collected = collect_entities(&world);

    // Collect entities with Position component
    let mut queried = Vec::new();
    for archetype in world.archetypes().iter() {
        for (entity, _) in archetype.iter_components::<Position>() {
            queried.push(entity);
        }
    }

    assert_eq!(
        collected.len(),
        queried.len(),
        "Query should find all entities with Position"
    );

    // Check order matches
    assert_eq!(
        collected, queried,
        "Query iteration order should match direct iteration order"
    );
}

// === Edge Cases ===

#[test]
fn test_empty_world_iteration() {
    let world = World::new();

    let collected = collect_entities(&world);

    assert_eq!(collected.len(), 0, "Empty world should have no entities");
}

#[test]
fn test_all_entities_despawned() {
    let mut world = World::new();

    // Spawn entities
    let entities: Vec<Entity> = (0..10).map(|_| world.spawn()).collect();

    // Despawn all
    for entity in entities {
        world.despawn(entity);
    }

    // Collect (should be empty)
    let collected = collect_entities(&world);

    assert_eq!(
        collected.len(),
        0,
        "World with all entities despawned should have no alive entities"
    );
}

#[test]
fn test_spawn_after_full_despawn() {
    let mut world = World::new();

    // Spawn and despawn entities
    for _ in 0..5 {
        let e = world.spawn();
        world.despawn(e);
    }

    // Spawn new entities
    let new_entities: Vec<Entity> = (0..5).map(|_| world.spawn()).collect();

    // Collect
    let collected = collect_entities(&world);

    assert_eq!(collected.len(), 5, "Should have 5 new entities");

    // Check all new entities present
    for entity in new_entities {
        assert!(
            collected.contains(&entity),
            "New entity {:?} should be present",
            entity
        );
    }
}
