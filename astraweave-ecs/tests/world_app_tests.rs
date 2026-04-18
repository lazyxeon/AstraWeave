//! Week 2 Day 1: Coverage tests for astraweave-ecs lib.rs
//!
//! Target: Fill coverage gaps from 64.56% (102/158) to 85%+ (~15 lines)
//! Focus areas:
//! - Resource system (insert_resource, get_resource, get_resource_mut)
//! - Schedule and App builder methods
//! - Entity lifecycle edge cases
//! - Archetype API (archetypes() accessor)

use astraweave_ecs::*;

// ========== Test Components & Resources ==========

#[derive(Clone, Copy, Debug, PartialEq)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Velocity {
    vx: f32,
    vy: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Health(i32);

#[derive(Debug, PartialEq)]
struct GameConfig {
    tick_rate: u32,
}

#[derive(Debug, PartialEq)]
struct PlayerStats {
    score: i32,
    lives: i32,
}

// ========== Resource System Tests ==========

#[test]
fn test_insert_and_get_resource() {
    let mut world = World::new();

    // Insert resource
    world.insert_resource(GameConfig { tick_rate: 60 });

    // Retrieve resource
    let config = world.get_resource::<GameConfig>().unwrap();
    assert_eq!(config.tick_rate, 60);
}

#[test]
fn test_get_resource_nonexistent() {
    let world = World::new();

    // Try to get non-existent resource
    assert!(world.get_resource::<GameConfig>().is_none());
}

#[test]
fn test_get_resource_mut() {
    let mut world = World::new();
    world.insert_resource(PlayerStats { score: 0, lives: 3 });

    // Mutate resource
    {
        let stats = world.get_resource_mut::<PlayerStats>().unwrap();
        stats.score += 100;
        stats.lives -= 1;
    }

    // Verify mutation
    let stats = world.get_resource::<PlayerStats>().unwrap();
    assert_eq!(stats.score, 100);
    assert_eq!(stats.lives, 2);
}

#[test]
fn test_get_resource_mut_nonexistent() {
    let mut world = World::new();

    // Try to mutate non-existent resource
    assert!(world.get_resource_mut::<GameConfig>().is_none());
}

#[test]
fn test_resource_replacement() {
    let mut world = World::new();

    // Insert initial resource
    world.insert_resource(GameConfig { tick_rate: 30 });
    assert_eq!(world.get_resource::<GameConfig>().unwrap().tick_rate, 30);

    // Replace with new value
    world.insert_resource(GameConfig { tick_rate: 60 });
    assert_eq!(world.get_resource::<GameConfig>().unwrap().tick_rate, 60);
}

// ========== Schedule Tests ==========

#[test]
fn test_schedule_with_stage() {
    let schedule = Schedule::default()
        .with_stage("perception")
        .with_stage("simulation");

    assert_eq!(schedule.stages.len(), 2);
    assert_eq!(schedule.stages[0].name, "perception");
    assert_eq!(schedule.stages[1].name, "simulation");
}

#[test]
fn test_schedule_add_system() {
    let mut schedule = Schedule::default().with_stage("simulation");

    fn test_system(_world: &mut World) {}

    schedule.add_system("simulation", test_system);

    assert_eq!(schedule.stages[0].systems.len(), 1);
}

#[test]
fn test_schedule_add_system_nonexistent_stage() {
    let mut schedule = Schedule::default().with_stage("simulation");

    fn test_system(_world: &mut World) {}

    // Add system to non-existent stage (should be silently ignored)
    schedule.add_system("nonexistent", test_system);

    assert_eq!(schedule.stages[0].systems.len(), 0); // No system added
}

#[test]
fn test_schedule_run() {
    let mut world = World::new();
    world.insert_resource(PlayerStats { score: 0, lives: 3 });

    fn increment_score(world: &mut World) {
        if let Some(stats) = world.get_resource_mut::<PlayerStats>() {
            stats.score += 10;
        }
    }

    fn decrement_lives(world: &mut World) {
        if let Some(stats) = world.get_resource_mut::<PlayerStats>() {
            stats.lives -= 1;
        }
    }

    let mut schedule = Schedule::default().with_stage("simulation");
    schedule.add_system("simulation", increment_score);
    schedule.add_system("simulation", decrement_lives);

    // Run schedule
    schedule.run(&mut world);

    // Verify both systems executed
    let stats = world.get_resource::<PlayerStats>().unwrap();
    assert_eq!(stats.score, 10);
    assert_eq!(stats.lives, 2);
}

#[test]
fn test_schedule_run_empty() {
    let mut world = World::new();
    let schedule = Schedule::default();

    // Running empty schedule should not panic
    schedule.run(&mut world);
}

// ========== App Builder Tests ==========

#[test]
fn test_app_new() {
    let app = App::new();

    // Verify default stages — 8 canonical stages per the 2026-04-18 schedule-stage-fix
    // (commit 70266b74e). See `docs/audits/schedule_stage_fix_2026-04-18.md` §2.1.
    assert_eq!(app.schedule.stages.len(), 8);
    assert_eq!(app.schedule.stages[0].name, "pre_simulation");
    assert_eq!(app.schedule.stages[1].name, "perception");
    assert_eq!(app.schedule.stages[2].name, "simulation");
    assert_eq!(app.schedule.stages[3].name, "sync");
    assert_eq!(app.schedule.stages[4].name, "ai_planning");
    assert_eq!(app.schedule.stages[5].name, "physics");
    assert_eq!(app.schedule.stages[6].name, "post_simulation");
    assert_eq!(app.schedule.stages[7].name, "presentation");
}

#[test]
fn test_app_default() {
    let app = App::default();

    // Default should match new()
    assert_eq!(app.schedule.stages.len(), 8);
}

#[test]
fn test_app_add_system() {
    let mut app = App::new();

    fn test_system(_world: &mut World) {}

    app.add_system("simulation", test_system);

    // Find simulation stage and verify system added
    let sim_stage = app
        .schedule
        .stages
        .iter()
        .find(|s| s.name == "simulation")
        .unwrap();
    assert_eq!(sim_stage.systems.len(), 1);
}

#[test]
fn test_app_insert_resource() {
    let app = App::new().insert_resource(GameConfig { tick_rate: 60 });

    // Verify resource was inserted
    let config = app.world.get_resource::<GameConfig>().unwrap();
    assert_eq!(config.tick_rate, 60);
}

#[test]
fn test_app_run_fixed() {
    let mut app = App::new();
    app.world
        .insert_resource(PlayerStats { score: 0, lives: 3 });

    fn increment_score(world: &mut World) {
        if let Some(stats) = world.get_resource_mut::<PlayerStats>() {
            stats.score += 1;
        }
    }

    app.add_system("simulation", increment_score);

    // Run 10 ticks
    app = app.run_fixed(10);

    // Verify system ran 10 times
    let stats = app.world.get_resource::<PlayerStats>().unwrap();
    assert_eq!(stats.score, 10);
}

#[test]
fn test_app_run_fixed_zero_steps() {
    let mut app = App::new();
    app.world
        .insert_resource(PlayerStats { score: 0, lives: 3 });

    fn increment_score(world: &mut World) {
        if let Some(stats) = world.get_resource_mut::<PlayerStats>() {
            stats.score += 1;
        }
    }

    app.add_system("simulation", increment_score);

    // Run 0 ticks
    app = app.run_fixed(0);

    // Verify system never ran
    let stats = app.world.get_resource::<PlayerStats>().unwrap();
    assert_eq!(stats.score, 0);
}

#[test]
fn test_app_chained_builder() {
    let app = App::new()
        .insert_resource(GameConfig { tick_rate: 60 })
        .insert_resource(PlayerStats { score: 0, lives: 3 });

    // Verify both resources inserted
    assert!(app.world.get_resource::<GameConfig>().is_some());
    assert!(app.world.get_resource::<PlayerStats>().is_some());
}

// ========== Entity Lifecycle Edge Cases ==========

#[test]
fn test_get_on_dead_entity() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, Position { x: 1.0, y: 2.0 });

    // Despawn entity
    world.despawn(entity);

    // Try to get component from dead entity
    assert!(world.get::<Position>(entity).is_none());
}

#[test]
fn test_get_mut_on_dead_entity() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, Position { x: 1.0, y: 2.0 });

    // Despawn entity
    world.despawn(entity);

    // Try to get_mut component from dead entity
    assert!(world.get_mut::<Position>(entity).is_none());
}

#[test]
fn test_insert_on_dead_entity() {
    let mut world = World::new();
    let entity = world.spawn();

    // Despawn entity
    world.despawn(entity);

    // Try to insert component on dead entity (should be silently ignored)
    world.insert(entity, Position { x: 1.0, y: 2.0 });

    // Verify entity still dead and no component added
    assert!(!world.is_alive(entity));
    assert!(world.get::<Position>(entity).is_none());
}

#[test]
fn test_remove_on_dead_entity() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, Position { x: 1.0, y: 2.0 });

    // Despawn entity
    world.despawn(entity);

    // Try to remove component from dead entity
    let removed = world.remove::<Position>(entity);

    // Should return false (entity already dead)
    assert!(!removed);
}

#[test]
fn test_has_on_dead_entity() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, Position { x: 1.0, y: 2.0 });

    // Despawn entity
    world.despawn(entity);

    // Try to check component on dead entity
    assert!(!world.has::<Position>(entity));
}

#[test]
fn test_despawn_already_dead() {
    let mut world = World::new();
    let entity = world.spawn();

    // First despawn
    assert!(world.despawn(entity));

    // Second despawn (should return false)
    assert!(!world.despawn(entity));
}

// ========== Archetype API Tests ==========

#[test]
fn test_archetypes_accessor() {
    let mut world = World::new();

    // Spawn entities with different component combinations
    let e1 = world.spawn();
    world.insert(e1, Position { x: 1.0, y: 1.0 });

    let e2 = world.spawn();
    world.insert(e2, Position { x: 2.0, y: 2.0 });
    world.insert(e2, Velocity { vx: 1.0, vy: 0.0 });

    let e3 = world.spawn();
    world.insert(e3, Health(100));

    // Access archetype storage
    let archetypes = world.archetypes();

    // Verify archetype count (empty + 3 unique signatures)
    // Empty archetype + Position + (Position, Velocity) + Health = 4 archetypes
    let archetype_count = archetypes.iter().count();
    assert!(archetype_count >= 4); // At least 4 archetypes
}

#[test]
fn test_entity_count() {
    let mut world = World::new();

    assert_eq!(world.entity_count(), 0);

    let e1 = world.spawn();
    assert_eq!(world.entity_count(), 1);

    let e2 = world.spawn();
    assert_eq!(world.entity_count(), 2);

    world.despawn(e1);
    assert_eq!(world.entity_count(), 1);

    world.despawn(e2);
    assert_eq!(world.entity_count(), 0);
}

// ========== Integration Tests ==========

#[test]
fn test_full_app_lifecycle() {
    fn movement_system(world: &mut World) {
        let entities: Vec<Entity> = world.entities_with::<Position>();
        for entity in entities {
            // Get velocity first (immutable borrow)
            let vel = world.get::<Velocity>(entity).copied();
            // Then get position (mutable borrow)
            if let (Some(pos), Some(vel)) = (world.get_mut::<Position>(entity), vel) {
                pos.x += vel.vx;
                pos.y += vel.vy;
            }
        }
    }

    let mut app = App::new();
    app.add_system("simulation", movement_system);

    // Create entity
    let entity = app.world.spawn();
    app.world.insert(entity, Position { x: 0.0, y: 0.0 });
    app.world.insert(entity, Velocity { vx: 1.0, vy: 2.0 });

    // Run 5 ticks
    app = app.run_fixed(5);

    // Verify position updated
    let pos = app.world.get::<Position>(entity).unwrap();
    assert_eq!(pos.x, 5.0);
    assert_eq!(pos.y, 10.0);
}

#[test]
fn test_multiple_stages_execution_order() {
    #[derive(Debug, PartialEq)]
    struct ExecutionOrder(Vec<&'static str>);

    fn perception_system(world: &mut World) {
        if let Some(order) = world.get_resource_mut::<ExecutionOrder>() {
            order.0.push("perception");
        }
    }

    fn simulation_system(world: &mut World) {
        if let Some(order) = world.get_resource_mut::<ExecutionOrder>() {
            order.0.push("simulation");
        }
    }

    fn physics_system(world: &mut World) {
        if let Some(order) = world.get_resource_mut::<ExecutionOrder>() {
            order.0.push("physics");
        }
    }

    let mut app = App::new();
    app.world.insert_resource(ExecutionOrder(vec![]));
    app.add_system("perception", perception_system);
    app.add_system("simulation", simulation_system);
    app.add_system("physics", physics_system);

    // Run one tick
    app = app.run_fixed(1);

    // Verify execution order matches stage order
    let order = app.world.get_resource::<ExecutionOrder>().unwrap();
    assert_eq!(order.0, vec!["perception", "simulation", "physics"]);
}

#[test]
fn test_register_component() {
    let mut world = World::new();

    // Register component type (used by CommandBuffer)
    world.register_component::<Position>();
    world.register_component::<Velocity>();

    // Verify registration doesn't panic
    // (Type registry is internal, no public API to verify)
}
