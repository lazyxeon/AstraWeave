//! Mutation-Resistant Behavioral Correctness Tests for ECS Systems
//!
//! These tests verify that ECS subsystems produce CORRECT behavior, not just
//! that they run without crashing. Each test is designed to catch common mutations
//! (e.g., + to -, * to /, sign flips, wrong comparisons, off-by-one errors).
//!
//! Tests verify:
//! - Entity allocation/deallocation correctness
//! - Component storage integrity
//! - Archetype transitions preserve data
//! - Generation validation prevents stale access
//! - Query iteration correctness
//! - Event system ordering
//! - Determinism properties
//!
//! Phase 8.8: Production-Ready ECS Validation

#![allow(clippy::unnecessary_cast, clippy::approx_constant)]

use astraweave_ecs::{Event, Events, World};
use std::collections::HashSet;

// ============================================================================
// TEST COMPONENT TYPES
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct Health(i32);

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct Marker;

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
struct Name(String);

// ============================================================================
// ENTITY ALLOCATION CORRECTNESS
// ============================================================================

/// Verify entity IDs are unique on spawn
#[test]
fn test_entity_ids_unique() {
    let mut world = World::new();

    let mut entities = Vec::new();
    for _ in 0..100 {
        entities.push(world.spawn());
    }

    // All entity IDs should be unique
    let unique: HashSet<_> = entities.iter().map(|e| e.id()).collect();
    assert_eq!(unique.len(), 100, "All 100 entities should have unique IDs");
}

/// Verify entity count tracks correctly (catches off-by-one)
#[test]
fn test_entity_count_accuracy() {
    let mut world = World::new();

    assert_eq!(world.entity_count(), 0, "Initial count should be 0");

    let e1 = world.spawn();
    assert_eq!(world.entity_count(), 1, "Count should be 1 after spawn");

    let e2 = world.spawn();
    assert_eq!(
        world.entity_count(),
        2,
        "Count should be 2 after second spawn"
    );

    world.despawn(e1);
    assert_eq!(world.entity_count(), 1, "Count should be 1 after despawn");

    world.despawn(e2);
    assert_eq!(
        world.entity_count(),
        0,
        "Count should be 0 after all despawned"
    );
}

/// Verify is_alive returns correct values
#[test]
fn test_entity_is_alive_correctness() {
    let mut world = World::new();

    let e = world.spawn();

    assert!(world.is_alive(e), "Entity should be alive after spawn");

    world.despawn(e);

    assert!(!world.is_alive(e), "Entity should be dead after despawn");
}

/// Verify double despawn returns false (not a bug)
#[test]
fn test_double_despawn_safe() {
    let mut world = World::new();
    let e = world.spawn();

    let first = world.despawn(e);
    let second = world.despawn(e);

    assert!(first, "First despawn should succeed");
    assert!(!second, "Second despawn should fail (already dead)");
}

/// Verify generation prevents stale entity access
#[test]
fn test_generation_prevents_stale_access() {
    let mut world = World::new();

    // Spawn, add component, despawn
    let old_entity = world.spawn();
    world.insert(old_entity, Health(100));
    world.despawn(old_entity);

    // Spawn new entity (may reuse slot)
    let new_entity = world.spawn();
    world.insert(new_entity, Health(50));

    // Old handle should NOT access new entity's data
    assert!(
        !world.is_alive(old_entity),
        "Old entity handle should be dead"
    );
    assert_eq!(
        world.get::<Health>(old_entity),
        None,
        "Old entity should not access any data"
    );

    // New entity should be accessible
    assert!(world.is_alive(new_entity), "New entity should be alive");
    assert_eq!(
        world.get::<Health>(new_entity),
        Some(&Health(50)),
        "New entity should have Health(50)"
    );
}

// ============================================================================
// COMPONENT STORAGE CORRECTNESS
// ============================================================================

/// Verify component insert and get roundtrip
#[test]
fn test_component_roundtrip() {
    let mut world = World::new();
    let e = world.spawn();

    let pos = Position { x: 10.5, y: -20.3 };
    world.insert(e, pos);

    let retrieved = world.get::<Position>(e);
    assert_eq!(
        retrieved,
        Some(&pos),
        "Retrieved component should match inserted"
    );
}

/// Verify component overwrite replaces previous value
#[test]
fn test_component_overwrite() {
    let mut world = World::new();
    let e = world.spawn();

    world.insert(e, Health(100));
    world.insert(e, Health(50)); // Overwrite

    assert_eq!(
        world.get::<Health>(e),
        Some(&Health(50)),
        "Component should be overwritten to 50"
    );
}

/// Verify get_mut allows modification
#[test]
fn test_component_get_mut() {
    let mut world = World::new();
    let e = world.spawn();

    world.insert(e, Health(100));

    if let Some(health) = world.get_mut::<Health>(e) {
        health.0 -= 25;
    }

    assert_eq!(
        world.get::<Health>(e),
        Some(&Health(75)),
        "Health should be modified to 75"
    );
}

/// Verify component removal works correctly
#[test]
fn test_component_remove() {
    let mut world = World::new();
    let e = world.spawn();

    world.insert(e, Health(100));
    world.insert(e, Position { x: 0.0, y: 0.0 });

    assert!(world.has::<Health>(e), "Should have Health before remove");

    let removed = world.remove::<Health>(e);

    assert!(removed, "Remove should return true");
    assert!(
        !world.has::<Health>(e),
        "Should NOT have Health after remove"
    );
    assert!(world.has::<Position>(e), "Should still have Position");
}

/// Verify remove returns false for non-existent component
#[test]
fn test_component_remove_nonexistent() {
    let mut world = World::new();
    let e = world.spawn();

    let removed = world.remove::<Health>(e);

    assert!(
        !removed,
        "Remove non-existent component should return false"
    );
}

/// Verify multiple components per entity
#[test]
fn test_multiple_components() {
    let mut world = World::new();
    let e = world.spawn();

    world.insert(e, Position { x: 1.0, y: 2.0 });
    world.insert(e, Velocity { dx: 0.5, dy: -0.5 });
    world.insert(e, Health(100));

    assert_eq!(world.get::<Position>(e), Some(&Position { x: 1.0, y: 2.0 }));
    assert_eq!(
        world.get::<Velocity>(e),
        Some(&Velocity { dx: 0.5, dy: -0.5 })
    );
    assert_eq!(world.get::<Health>(e), Some(&Health(100)));
}

/// Verify component isolation between entities
#[test]
fn test_component_isolation() {
    let mut world = World::new();

    let e1 = world.spawn();
    let e2 = world.spawn();

    world.insert(e1, Health(100));
    world.insert(e2, Health(50));

    // Modify e1's health
    if let Some(h) = world.get_mut::<Health>(e1) {
        h.0 = 25;
    }

    // e2's health should be unchanged
    assert_eq!(world.get::<Health>(e1), Some(&Health(25)));
    assert_eq!(world.get::<Health>(e2), Some(&Health(50)));
}

// ============================================================================
// ARCHETYPE TRANSITION CORRECTNESS
// ============================================================================

/// Verify archetype transition preserves existing components
#[test]
fn test_archetype_transition_preserves_data() {
    let mut world = World::new();
    let e = world.spawn();

    world.insert(e, Position { x: 10.0, y: 20.0 });
    world.insert(e, Health(100));

    // Adding new component triggers archetype transition
    world.insert(e, Velocity { dx: 1.0, dy: 1.0 });

    // All components should be preserved
    assert_eq!(
        world.get::<Position>(e),
        Some(&Position { x: 10.0, y: 20.0 }),
        "Position should be preserved after archetype transition"
    );
    assert_eq!(
        world.get::<Health>(e),
        Some(&Health(100)),
        "Health should be preserved after archetype transition"
    );
    assert_eq!(
        world.get::<Velocity>(e),
        Some(&Velocity { dx: 1.0, dy: 1.0 }),
        "Velocity should be added"
    );
}

/// Verify component removal triggers correct archetype transition
#[test]
fn test_archetype_transition_on_remove() {
    let mut world = World::new();
    let e = world.spawn();

    world.insert(e, Position { x: 5.0, y: 5.0 });
    world.insert(e, Velocity { dx: 1.0, dy: 0.0 });
    world.insert(e, Health(100));

    // Remove component triggers archetype transition
    world.remove::<Velocity>(e);

    // Remaining components should be preserved
    assert_eq!(
        world.get::<Position>(e),
        Some(&Position { x: 5.0, y: 5.0 }),
        "Position should be preserved after remove"
    );
    assert_eq!(
        world.get::<Health>(e),
        Some(&Health(100)),
        "Health should be preserved after remove"
    );
    assert!(!world.has::<Velocity>(e), "Velocity should be gone");
}

// ============================================================================
// QUERY ITERATION CORRECTNESS
// ============================================================================

/// Verify count<T> accuracy
#[test]
fn test_count_accuracy() {
    let mut world = World::new();

    assert_eq!(world.count::<Health>(), 0, "Initial count should be 0");

    for i in 0..10 {
        let e = world.spawn();
        world.insert(e, Health(i));
    }

    assert_eq!(world.count::<Health>(), 10, "Count should be 10");
}

/// Verify entities_with<T> returns all entities
#[test]
fn test_entities_with_completeness() {
    let mut world = World::new();

    let mut expected = HashSet::new();
    for i in 0..5 {
        let e = world.spawn();
        world.insert(e, Health(i));
        expected.insert(e.id());
    }

    // Add some entities without Health
    for _ in 0..3 {
        let e = world.spawn();
        world.insert(e, Position { x: 0.0, y: 0.0 });
    }

    let found: HashSet<_> = world
        .entities_with::<Health>()
        .iter()
        .map(|e| e.id())
        .collect();

    assert_eq!(
        found, expected,
        "entities_with should return exactly the entities with Health"
    );
}

/// Verify each_mut visits all entities exactly once
#[test]
fn test_each_mut_visits_all() {
    let mut world = World::new();

    for i in 0..10 {
        let e = world.spawn();
        world.insert(e, Health(i as i32));
    }

    let mut sum = 0;
    let mut count = 0;
    world.each_mut(|_entity, health: &mut Health| {
        sum += health.0;
        count += 1;
    });

    assert_eq!(count, 10, "Should visit all 10 entities");
    assert_eq!(sum, 45, "Sum should be 0+1+2+...+9 = 45");
}

/// Verify each_mut can modify components
#[test]
fn test_each_mut_modifies() {
    let mut world = World::new();

    for _ in 0..5 {
        let e = world.spawn();
        world.insert(e, Health(100));
    }

    // Damage all entities
    world.each_mut(|_entity, health: &mut Health| {
        health.0 -= 25;
    });

    // Verify all are modified
    for entity in world.entities_with::<Health>() {
        assert_eq!(
            world.get::<Health>(entity),
            Some(&Health(75)),
            "All entities should have Health(75)"
        );
    }
}

// ============================================================================
// RESOURCE SYSTEM CORRECTNESS
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
struct GameTime(f32);

#[derive(Clone, Debug, PartialEq)]
struct Score(i32);

/// Verify resource insert and get
#[test]
fn test_resource_roundtrip() {
    let mut world = World::new();

    world.insert_resource(GameTime(0.0));

    assert_eq!(
        world.get_resource::<GameTime>(),
        Some(&GameTime(0.0)),
        "Resource should be retrievable"
    );
}

/// Verify resource overwrite
#[test]
fn test_resource_overwrite() {
    let mut world = World::new();

    world.insert_resource(Score(100));
    world.insert_resource(Score(200)); // Overwrite

    assert_eq!(
        world.get_resource::<Score>(),
        Some(&Score(200)),
        "Resource should be overwritten"
    );
}

/// Verify resource mutation
#[test]
fn test_resource_mutation() {
    let mut world = World::new();

    world.insert_resource(Score(0));

    if let Some(score) = world.get_resource_mut::<Score>() {
        score.0 += 100;
    }

    assert_eq!(
        world.get_resource::<Score>(),
        Some(&Score(100)),
        "Resource should be mutated"
    );
}

/// Verify missing resource returns None
#[test]
fn test_resource_missing() {
    let world = World::new();

    assert_eq!(
        world.get_resource::<GameTime>(),
        None,
        "Missing resource should return None"
    );
}

/// Verify resource isolation
#[test]
fn test_resource_isolation() {
    let mut world = World::new();

    world.insert_resource(GameTime(1.0));
    world.insert_resource(Score(50));

    // Modifying one shouldn't affect the other
    if let Some(score) = world.get_resource_mut::<Score>() {
        score.0 = 100;
    }

    assert_eq!(world.get_resource::<GameTime>(), Some(&GameTime(1.0)));
    assert_eq!(world.get_resource::<Score>(), Some(&Score(100)));
}

// ============================================================================
// EVENT SYSTEM CORRECTNESS
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
struct DamageEvent {
    target: u32,
    amount: i32,
}

impl Event for DamageEvent {}

/// Verify event send and receive
#[test]
fn test_event_roundtrip() {
    let mut events = Events::new();

    events.send(DamageEvent {
        target: 1,
        amount: 10,
    });
    events.send(DamageEvent {
        target: 2,
        amount: 20,
    });

    let collected: Vec<_> = events.read::<DamageEvent>().collect();

    assert_eq!(collected.len(), 2, "Should receive 2 events");
    assert_eq!(collected[0].target, 1);
    assert_eq!(collected[0].amount, 10);
    assert_eq!(collected[1].target, 2);
    assert_eq!(collected[1].amount, 20);
}

/// Verify event ordering is preserved (FIFO)
#[test]
fn test_event_ordering() {
    let mut events = Events::new();

    for i in 0..10 {
        events.send(DamageEvent {
            target: i,
            amount: i as i32 * 5,
        });
    }

    let targets: Vec<_> = events.read::<DamageEvent>().map(|e| e.target).collect();

    let expected: Vec<u32> = (0..10).collect();
    assert_eq!(targets, expected, "Events should be in FIFO order");
}

/// Verify event count accuracy
#[test]
fn test_event_count() {
    let mut events = Events::new();

    assert_eq!(events.len::<DamageEvent>(), 0, "Initial count should be 0");

    events.send(DamageEvent {
        target: 1,
        amount: 10,
    });
    assert_eq!(events.len::<DamageEvent>(), 1, "Count should be 1");

    events.send(DamageEvent {
        target: 2,
        amount: 20,
    });
    assert_eq!(events.len::<DamageEvent>(), 2, "Count should be 2");
}

/// Verify event drain consumes events
#[test]
fn test_event_drain() {
    let mut events = Events::new();

    events.send(DamageEvent {
        target: 1,
        amount: 10,
    });
    events.send(DamageEvent {
        target: 2,
        amount: 20,
    });

    // Drain all events
    let drained: Vec<_> = events.drain::<DamageEvent>().collect();

    assert_eq!(drained.len(), 2, "Should drain 2 events");
    assert_eq!(
        events.len::<DamageEvent>(),
        0,
        "Should be empty after drain"
    );
}

/// Verify event clear
#[test]
fn test_event_clear() {
    let mut events = Events::new();

    events.send(DamageEvent {
        target: 1,
        amount: 10,
    });
    events.send(DamageEvent {
        target: 2,
        amount: 20,
    });

    events.clear::<DamageEvent>();

    assert_eq!(
        events.len::<DamageEvent>(),
        0,
        "Should be empty after clear"
    );
}

// ============================================================================
// DETERMINISM CORRECTNESS
// ============================================================================

/// Verify entity iteration order is deterministic
#[test]
fn test_iteration_determinism() {
    // Create two worlds with same sequence
    let mut world1 = World::new();
    let mut world2 = World::new();

    for i in 0..20 {
        let e1 = world1.spawn();
        let e2 = world2.spawn();
        world1.insert(e1, Health(i));
        world2.insert(e2, Health(i));
    }

    // Collect iteration order from both
    let order1: Vec<_> = world1
        .entities_with::<Health>()
        .iter()
        .map(|e| e.id())
        .collect();
    let order2: Vec<_> = world2
        .entities_with::<Health>()
        .iter()
        .map(|e| e.id())
        .collect();

    assert_eq!(
        order1, order2,
        "Entity iteration order should be deterministic"
    );
}

/// Verify component values are preserved exactly
#[test]
fn test_component_value_preservation() {
    let mut world = World::new();
    let e = world.spawn();

    // Use specific float values that could be affected by floating point errors
    let pos = Position {
        x: 0.1 + 0.2,
        y: -3.14159,
    };
    world.insert(e, pos);

    let retrieved = world.get::<Position>(e).unwrap();

    // Bit-exact comparison (not fuzzy)
    assert_eq!(
        retrieved.x.to_bits(),
        pos.x.to_bits(),
        "X should be bit-exact"
    );
    assert_eq!(
        retrieved.y.to_bits(),
        pos.y.to_bits(),
        "Y should be bit-exact"
    );
}

// ============================================================================
// MUTATION-CATCHING EDGE CASES
// ============================================================================

/// Catch off-by-one in entity ID generation
#[test]
fn test_mutation_entity_id_sequence() {
    let mut world = World::new();

    let e1 = world.spawn();
    let e2 = world.spawn();
    let e3 = world.spawn();

    // Entity IDs should be sequential (0, 1, 2 or 1, 2, 3 depending on implementation)
    assert_eq!(e2.id() - e1.id(), 1, "Entity IDs should differ by 1");
    assert_eq!(e3.id() - e2.id(), 1, "Entity IDs should differ by 1");
}

/// Catch comparison operator mutations in has<T>
#[test]
fn test_mutation_has_component() {
    let mut world = World::new();
    let e = world.spawn();

    assert!(
        !world.has::<Health>(e),
        "Should NOT have Health before insert"
    );

    world.insert(e, Health(100));

    assert!(world.has::<Health>(e), "Should have Health after insert");
    assert!(!world.has::<Velocity>(e), "Should NOT have Velocity");
}

/// Catch sign errors in component modification
#[test]
fn test_mutation_sign_errors() {
    let mut world = World::new();
    let e = world.spawn();

    world.insert(e, Health(100));

    // Subtract health
    if let Some(h) = world.get_mut::<Health>(e) {
        h.0 -= 30;
    }

    // Result should be 70, not 130
    assert_eq!(
        world.get::<Health>(e),
        Some(&Health(70)),
        "Health should be 70 (100 - 30), not 130"
    );
}

/// Catch wrong return on empty world
#[test]
fn test_mutation_empty_world() {
    let world = World::new();

    assert_eq!(
        world.entity_count(),
        0,
        "Empty world should have 0 entities"
    );
    assert_eq!(
        world.count::<Health>(),
        0,
        "Empty world should have 0 Health components"
    );
    assert!(
        world.entities_with::<Health>().is_empty(),
        "Empty world should return empty vec"
    );
}

/// Catch boolean inversion in is_alive
#[test]
fn test_mutation_boolean_inversion() {
    let mut world = World::new();
    let e = world.spawn();

    // These must be different - catches ! mutation
    let alive_before = world.is_alive(e);
    world.despawn(e);
    let alive_after = world.is_alive(e);

    assert_ne!(
        alive_before, alive_after,
        "is_alive should return different values before and after despawn"
    );
}

/// Verify stale entity operations are safe
#[test]
fn test_stale_entity_safety() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Health(100));
    world.despawn(e);

    // All operations on stale entity should be safe
    assert!(!world.is_alive(e), "Stale entity should not be alive");
    assert!(world.get::<Health>(e).is_none(), "get should return None");
    assert!(
        world.get_mut::<Health>(e).is_none(),
        "get_mut should return None"
    );
    assert!(!world.has::<Health>(e), "has should return false");
    assert!(!world.remove::<Health>(e), "remove should return false");
    assert!(!world.despawn(e), "despawn should return false");
}
