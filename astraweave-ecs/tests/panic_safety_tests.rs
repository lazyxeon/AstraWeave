//! Panic safety tests for ECS subsystem.
//!
//! P0-Critical: Ensures ECS handles edge cases gracefully without panicking.
//! Tests cover double despawn, dead entity access, and invalid operations.

use astraweave_ecs::*;
use std::panic;

/// Helper to verify a closure doesn't panic
fn should_not_panic<F: FnOnce() + panic::UnwindSafe>(name: &str, f: F) {
    let result = panic::catch_unwind(f);
    assert!(result.is_ok(), "{} should not panic on invalid input", name);
}

// Define test components (no need to implement Component - blanket impl exists)
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
struct Velocity {
    vx: f32,
    vy: f32,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
struct Health(i32);

/// Helper to spawn entity with a component using spawn + insert
fn spawn_with_position(world: &mut World, x: f32, y: f32) -> Entity {
    let e = world.spawn();
    world.insert(e, Position { x, y });
    e
}

fn spawn_with_health(world: &mut World, h: i32) -> Entity {
    let e = world.spawn();
    world.insert(e, Health(h));
    e
}

fn spawn_with_velocity(world: &mut World, vx: f32, vy: f32) -> Entity {
    let e = world.spawn();
    world.insert(e, Velocity { vx, vy });
    e
}

// ============================================================================
// Double despawn tests
// ============================================================================

#[test]
fn test_double_despawn_no_panic() {
    should_not_panic("double despawn", || {
        let mut world = World::new();
        let entity = world.spawn();

        // First despawn should succeed
        let first_result = world.despawn(entity);
        assert!(first_result, "First despawn should succeed");

        // Second despawn should fail gracefully, not panic
        let second_result = world.despawn(entity);
        assert!(!second_result, "Second despawn should return false");
    });
}

#[test]
fn test_triple_despawn_no_panic() {
    should_not_panic("triple despawn", || {
        let mut world = World::new();
        let entity = world.spawn();

        world.despawn(entity);
        world.despawn(entity);
        world.despawn(entity);
    });
}

#[test]
fn test_despawn_after_component_removal() {
    should_not_panic("despawn after component removal", || {
        let mut world = World::new();
        let entity = spawn_with_position(&mut world, 1.0, 2.0);

        // Remove component then despawn
        world.remove::<Position>(entity);
        let result = world.despawn(entity);
        assert!(result, "Should despawn entity without components");
    });
}

#[test]
fn test_despawn_with_multiple_components() {
    should_not_panic("despawn with multiple components", || {
        let mut world = World::new();
        let entity = spawn_with_position(&mut world, 1.0, 2.0);
        world.insert(entity, Velocity { vx: 10.0, vy: -5.0 });
        world.insert(entity, Health(100));

        // Despawn entity with multiple components
        let result = world.despawn(entity);
        assert!(result, "Should despawn entity with multiple components");
    });
}

// ============================================================================
// Dead entity access tests
// ============================================================================

#[test]
fn test_get_component_dead_entity_no_panic() {
    should_not_panic("get component from dead entity", || {
        let mut world = World::new();
        let entity = spawn_with_position(&mut world, 1.0, 2.0);

        world.despawn(entity);

        // Accessing component on dead entity should return None, not panic
        let pos = world.get::<Position>(entity);
        assert!(pos.is_none(), "Should return None for dead entity");
    });
}

#[test]
fn test_get_mut_component_dead_entity_no_panic() {
    should_not_panic("get_mut component from dead entity", || {
        let mut world = World::new();
        let entity = spawn_with_position(&mut world, 1.0, 2.0);

        world.despawn(entity);

        // Mutable access on dead entity should return None, not panic
        let pos = world.get_mut::<Position>(entity);
        assert!(pos.is_none(), "Should return None for dead entity");
    });
}

#[test]
fn test_has_component_dead_entity_no_panic() {
    should_not_panic("has component on dead entity", || {
        let mut world = World::new();
        let entity = spawn_with_position(&mut world, 1.0, 2.0);

        world.despawn(entity);

        // has() on dead entity should return false, not panic
        let has_pos = world.has::<Position>(entity);
        assert!(!has_pos, "Should return false for dead entity");
    });
}

#[test]
fn test_remove_component_dead_entity_no_panic() {
    should_not_panic("remove component from dead entity", || {
        let mut world = World::new();
        let entity = spawn_with_position(&mut world, 1.0, 2.0);

        world.despawn(entity);

        // remove() on dead entity should return false, not panic
        let removed = world.remove::<Position>(entity);
        assert!(!removed, "Should return false for dead entity");
    });
}

#[test]
fn test_insert_component_dead_entity_no_panic() {
    should_not_panic("insert component into dead entity", || {
        let mut world = World::new();
        let entity = spawn_with_position(&mut world, 1.0, 2.0);

        world.despawn(entity);

        // insert() on dead entity should be handled gracefully
        // (may fail or succeed based on implementation, but should not panic)
        world.insert(entity, Velocity { vx: 10.0, vy: -5.0 });
    });
}

#[test]
fn test_is_alive_dead_entity_no_panic() {
    should_not_panic("is_alive on dead entity", || {
        let mut world = World::new();
        let entity = world.spawn();

        world.despawn(entity);

        // is_alive() on dead entity should return false, not panic
        let alive = world.is_alive(entity);
        assert!(!alive, "Should return false for dead entity");
    });
}

// ============================================================================
// Stale generation tests
// ============================================================================

#[test]
fn test_operations_on_stale_generation_no_panic() {
    should_not_panic("operations on stale generation", || {
        let mut world = World::new();

        // Spawn and despawn to increment generation
        let entity = spawn_with_position(&mut world, 1.0, 2.0);
        world.despawn(entity);

        // entity now has stale generation
        // All operations should handle this gracefully
        let _ = world.get::<Position>(entity);
        let _ = world.get_mut::<Position>(entity);
        let _ = world.has::<Position>(entity);
        let _ = world.remove::<Position>(entity);
        let _ = world.despawn(entity);
    });
}

// ============================================================================
// Entity reuse tests
// ============================================================================

#[test]
fn test_entity_reuse_no_panic() {
    should_not_panic("entity reuse with stale handle", || {
        let mut world = World::new();

        // Spawn entity, save handle
        let old_entity = spawn_with_position(&mut world, 1.0, 2.0);

        // Despawn it
        world.despawn(old_entity);

        // Spawn a new entity (may reuse the ID)
        let new_entity = spawn_with_health(&mut world, 100);

        // Old handle should not access new entity's data
        let pos = world.get::<Position>(old_entity);
        assert!(pos.is_none(), "Stale handle should not access reused slot");

        // New entity should work correctly
        let health = world.get::<Health>(new_entity);
        assert!(health.is_some(), "New entity should have component");
    });
}

#[test]
fn test_mass_spawn_despawn_cycle_no_panic() {
    should_not_panic("mass spawn/despawn cycle", || {
        let mut world = World::new();
        let mut entities = Vec::new();

        // Spawn many entities
        for i in 0..1000 {
            let e = spawn_with_position(&mut world, i as f32, i as f32);
            entities.push(e);
        }

        // Despawn all
        for e in &entities {
            world.despawn(*e);
        }

        // Try to access despawned entities
        for e in &entities {
            let _ = world.get::<Position>(*e);
            let _ = world.has::<Position>(*e);
        }

        // Spawn again, reusing slots
        for _ in 0..1000 {
            let _ = spawn_with_health(&mut world, 100);
        }
    });
}

// ============================================================================
// Interleaved operations
// ============================================================================

#[test]
fn test_interleaved_spawn_despawn_no_panic() {
    should_not_panic("interleaved spawn/despawn", || {
        let mut world = World::new();

        for _ in 0..100 {
            let e1 = spawn_with_position(&mut world, 1.0, 2.0);
            let e2 = spawn_with_health(&mut world, 100);
            world.despawn(e1);
            let e3 = spawn_with_velocity(&mut world, 1.0, 2.0);
            world.despawn(e2);
            world.despawn(e3);
        }
    });
}

#[test]
fn test_component_modification_after_despawn_no_panic() {
    should_not_panic("component modification after despawn", || {
        let mut world = World::new();
        let entity = spawn_with_position(&mut world, 1.0, 2.0);

        world.despawn(entity);

        // Try to modify component after despawn
        if let Some(pos) = world.get_mut::<Position>(entity) {
            pos.x = 999.0; // This should never execute
        }

        // Try to insert component after despawn
        world.insert(entity, Health(100));
    });
}

// ============================================================================
// Edge cases with empty world
// ============================================================================

#[test]
fn test_query_on_empty_world_no_panic() {
    should_not_panic("query on empty world", || {
        let world = World::new();
        let entities = world.entities_with::<Position>();
        assert!(entities.is_empty(), "Empty world should have no entities");
    });
}

#[test]
fn test_entity_count_on_empty_world_no_panic() {
    should_not_panic("entity_count on empty world", || {
        let world = World::new();
        let count = world.entity_count();
        assert_eq!(count, 0, "Empty world should have 0 entities");
    });
}

// ============================================================================
// Command buffer tests
// ============================================================================

#[test]
fn test_command_buffer_despawn_dead_entity_no_panic() {
    should_not_panic("command buffer despawn dead entity", || {
        let mut world = World::new();
        let entity = spawn_with_position(&mut world, 1.0, 2.0);

        // Despawn entity directly
        world.despawn(entity);

        // Try to despawn again via command buffer
        let mut commands = CommandBuffer::new();
        commands.despawn(entity);

        // Flush should handle dead entity gracefully
        commands.flush(&mut world);
    });
}

#[test]
fn test_command_buffer_double_despawn_no_panic() {
    should_not_panic("command buffer double despawn", || {
        let mut world = World::new();
        let entity = spawn_with_position(&mut world, 1.0, 2.0);

        // Queue despawn twice
        let mut commands = CommandBuffer::new();
        commands.despawn(entity);
        commands.despawn(entity);

        // Flush should handle both gracefully
        commands.flush(&mut world);
    });
}

// ============================================================================
// Stress tests for panic safety
// ============================================================================

#[test]
fn test_rapid_entity_lifecycle_no_panic() {
    should_not_panic("rapid entity lifecycle", || {
        let mut world = World::new();

        for _ in 0..10000 {
            let e = world.spawn();
            world.despawn(e);
        }
    });
}

#[test]
fn test_all_operations_on_same_entity_no_panic() {
    should_not_panic("all operations on same entity", || {
        let mut world = World::new();
        let entity = spawn_with_position(&mut world, 1.0, 2.0);

        // Perform all operations
        let _ = world.is_alive(entity);
        let _ = world.has::<Position>(entity);
        let _ = world.get::<Position>(entity);
        let _ = world.get_mut::<Position>(entity);
        world.insert(entity, Health(100));
        let _ = world.has::<Health>(entity);
        let _ = world.remove::<Health>(entity);
        let _ = world.despawn(entity);

        // Try all operations again on dead entity
        let _ = world.is_alive(entity);
        let _ = world.has::<Position>(entity);
        let _ = world.get::<Position>(entity);
        let _ = world.get_mut::<Position>(entity);
        world.insert(entity, Velocity { vx: 1.0, vy: 2.0 });
        let _ = world.remove::<Position>(entity);
        let _ = world.despawn(entity);
    });
}
