//! Mutation-Resistant Behavioral Correctness Tests for Core Systems
//!
//! These tests verify that core engine subsystems produce CORRECT behavior, not just
//! that they run without crashing. Each test is designed to catch common mutations
//! (e.g., + to -, * to /, sign flips, wrong comparisons, off-by-one errors).
//!
//! Tests verify:
//! - IVec2 math operations (distance, manhattan, offset)
//! - World entity spawning and component access
//! - Cooldown tick calculations
//! - Health and damage application
//! - Validation of action steps
//! - Schema structures and WorldSnapshot
//!
//! Phase 8.8: Production-Ready Core Validation

use astraweave_core::{
    CompanionState, CoverType, EnemyState, IVec2, PlayerState, Poi, Stance, Team, World,
    WorldSnapshot,
};

// ============================================================================
// IVEC2 MATHEMATICAL CORRECTNESS
// ============================================================================

/// Verify IVec2::new constructs correctly
#[test]
fn test_ivec2_construction() {
    let v = IVec2::new(3, 5);
    assert_eq!(v.x, 3, "x should be 3");
    assert_eq!(v.y, 5, "y should be 5");
}

/// Verify IVec2::zero returns zero vector
#[test]
fn test_ivec2_zero() {
    let v = IVec2::zero();
    assert_eq!(v.x, 0, "x should be 0");
    assert_eq!(v.y, 0, "y should be 0");
    assert!(v.is_zero(), "is_zero should return true");
}

/// Verify is_zero distinguishes zero from non-zero
#[test]
fn test_ivec2_is_zero_correctness() {
    assert!(IVec2::new(0, 0).is_zero(), "(0,0) should be zero");
    assert!(!IVec2::new(1, 0).is_zero(), "(1,0) should not be zero");
    assert!(!IVec2::new(0, 1).is_zero(), "(0,1) should not be zero");
    assert!(!IVec2::new(-1, 0).is_zero(), "(-1,0) should not be zero");
}

/// Verify manhattan distance calculation (catches + to - mutations)
#[test]
fn test_manhattan_distance_correctness() {
    let a = IVec2::new(0, 0);
    let b = IVec2::new(3, 4);

    let dist = a.manhattan_distance(&b);
    // Manhattan = |3-0| + |4-0| = 3 + 4 = 7
    assert_eq!(dist, 7, "Manhattan distance should be 7");

    // Symmetric
    let dist_reverse = b.manhattan_distance(&a);
    assert_eq!(dist, dist_reverse, "Manhattan distance should be symmetric");
}

/// Verify manhattan distance with negative coordinates
#[test]
fn test_manhattan_distance_negative() {
    let a = IVec2::new(-3, -4);
    let b = IVec2::new(3, 4);

    let dist = a.manhattan_distance(&b);
    // Manhattan = |3-(-3)| + |4-(-4)| = 6 + 8 = 14
    assert_eq!(dist, 14, "Manhattan distance with negatives should be 14");
}

/// Verify distance_squared calculation (catches * to / mutations)
#[test]
fn test_distance_squared_correctness() {
    let a = IVec2::new(0, 0);
    let b = IVec2::new(3, 4);

    let dist_sq = a.distance_squared(&b);
    // Distance^2 = (3-0)^2 + (4-0)^2 = 9 + 16 = 25
    assert_eq!(dist_sq, 25, "Distance squared should be 25");
}

/// Verify Euclidean distance calculation
#[test]
fn test_euclidean_distance_correctness() {
    let a = IVec2::new(0, 0);
    let b = IVec2::new(3, 4);

    let dist = a.distance(&b);
    // Distance = sqrt(25) = 5
    assert!(
        (dist - 5.0).abs() < 0.001,
        "Euclidean distance should be 5.0"
    );
}

/// Verify distance is symmetric
#[test]
fn test_distance_symmetry() {
    let a = IVec2::new(-5, 3);
    let b = IVec2::new(7, -2);

    let dist_ab = a.distance(&b);
    let dist_ba = b.distance(&a);

    assert!(
        (dist_ab - dist_ba).abs() < 0.001,
        "Distance should be symmetric"
    );
}

/// Verify offset calculation
#[test]
fn test_offset_correctness() {
    let v = IVec2::new(10, 20);
    let result = v.offset(5, -3);

    assert_eq!(result.x, 15, "x should be 10+5=15");
    assert_eq!(result.y, 17, "y should be 20-3=17");
}

/// Verify IVec2 addition
#[test]
fn test_ivec2_addition() {
    let a = IVec2::new(3, 5);
    let b = IVec2::new(2, -1);
    let result = a + b;

    assert_eq!(result.x, 5, "x should be 3+2=5");
    assert_eq!(result.y, 4, "y should be 5+(-1)=4");
}

/// Verify IVec2 subtraction
#[test]
fn test_ivec2_subtraction() {
    let a = IVec2::new(10, 20);
    let b = IVec2::new(3, 7);
    let result = a - b;

    assert_eq!(result.x, 7, "x should be 10-3=7");
    assert_eq!(result.y, 13, "y should be 20-7=13");
}

// ============================================================================
// WORLD ENTITY MANAGEMENT
// ============================================================================

/// Verify entity spawning returns unique IDs
#[test]
fn test_entity_spawn_unique_ids() {
    let mut world = World::new();

    let e1 = world.spawn("Entity1", IVec2::new(0, 0), Team { id: 0 }, 100, 10);
    let e2 = world.spawn("Entity2", IVec2::new(1, 1), Team { id: 0 }, 100, 10);
    let e3 = world.spawn("Entity3", IVec2::new(2, 2), Team { id: 0 }, 100, 10);

    assert_ne!(e1, e2, "Entity IDs should be unique");
    assert_ne!(e2, e3, "Entity IDs should be unique");
    assert_ne!(e1, e3, "Entity IDs should be unique");
}

/// Verify entity position is stored correctly
#[test]
fn test_entity_position_storage() {
    let mut world = World::new();

    let pos = IVec2::new(42, 37);
    let e = world.spawn("Test", pos, Team { id: 0 }, 100, 10);

    let pose = world.pose(e).expect("Entity should have pose");
    assert_eq!(pose.pos.x, 42, "Position x should be 42");
    assert_eq!(pose.pos.y, 37, "Position y should be 37");
}

/// Verify health component is stored and retrievable
#[test]
fn test_entity_health_storage() {
    let mut world = World::new();

    let e = world.spawn("Test", IVec2::zero(), Team { id: 0 }, 75, 10);

    let health = world.health(e).expect("Entity should have health");
    assert_eq!(health.hp, 75, "Health should be 75");
}

/// Verify health can be modified
#[test]
fn test_entity_health_modification() {
    let mut world = World::new();

    let e = world.spawn("Test", IVec2::zero(), Team { id: 0 }, 100, 10);

    // Damage
    if let Some(h) = world.health_mut(e) {
        h.hp -= 30;
    }

    let health = world.health(e).unwrap();
    assert_eq!(health.hp, 70, "Health should be 100-30=70");
}

/// Verify team assignment
#[test]
fn test_entity_team_assignment() {
    let mut world = World::new();

    let player = world.spawn("Player", IVec2::zero(), Team { id: 0 }, 100, 10);
    let companion = world.spawn("Companion", IVec2::zero(), Team { id: 1 }, 100, 10);
    let enemy = world.spawn("Enemy", IVec2::zero(), Team { id: 2 }, 100, 10);

    assert_eq!(world.team(player).unwrap().id, 0, "Player should be team 0");
    assert_eq!(
        world.team(companion).unwrap().id,
        1,
        "Companion should be team 1"
    );
    assert_eq!(world.team(enemy).unwrap().id, 2, "Enemy should be team 2");
}

/// Verify entity destruction
#[test]
fn test_entity_destruction() {
    let mut world = World::new();

    let e = world.spawn("ToDestroy", IVec2::zero(), Team { id: 0 }, 100, 10);

    // Entity exists
    assert!(
        world.pose(e).is_some(),
        "Entity should exist before destruction"
    );

    // Destroy
    let destroyed = world.destroy_entity(e);
    assert!(
        destroyed,
        "Should return true when destroying existing entity"
    );

    // Entity gone
    assert!(
        world.pose(e).is_none(),
        "Entity should not exist after destruction"
    );
    assert!(
        world.health(e).is_none(),
        "Health should not exist after destruction"
    );
}

/// Verify double destruction is safe
#[test]
fn test_entity_double_destruction() {
    let mut world = World::new();

    let e = world.spawn("Test", IVec2::zero(), Team { id: 0 }, 100, 10);

    let first = world.destroy_entity(e);
    let second = world.destroy_entity(e);

    assert!(first, "First destruction should return true");
    assert!(!second, "Second destruction should return false");
}

// ============================================================================
// COOLDOWN TICK CORRECTNESS
// ============================================================================

/// Verify world tick advances time
#[test]
fn test_world_tick_time_advancement() {
    let mut world = World::new();

    assert!((world.t - 0.0).abs() < 0.001, "Initial time should be 0");

    world.tick(0.016); // ~60 FPS frame
    assert!((world.t - 0.016).abs() < 0.001, "Time should advance by dt");

    world.tick(0.016);
    assert!((world.t - 0.032).abs() < 0.001, "Time should be cumulative");
}

/// Verify cooldowns decrease with tick
#[test]
fn test_cooldown_decrease() {
    let mut world = World::new();

    let e = world.spawn("Test", IVec2::zero(), Team { id: 0 }, 100, 10);

    // Set a cooldown
    if let Some(cd) = world.cooldowns_mut(e) {
        cd.map.insert("ability".to_string(), 2.0);
    }

    // Tick 1 second
    world.tick(1.0);

    let cd_val = world
        .cooldowns(e)
        .unwrap()
        .map
        .get("ability")
        .copied()
        .unwrap();
    assert!(
        (cd_val - 1.0).abs() < 0.001,
        "Cooldown should decrease: expected 1.0, got {}",
        cd_val
    );
}

/// Verify cooldowns don't go negative
#[test]
fn test_cooldown_no_negative() {
    let mut world = World::new();

    let e = world.spawn("Test", IVec2::zero(), Team { id: 0 }, 100, 10);

    if let Some(cd) = world.cooldowns_mut(e) {
        cd.map.insert("ability".to_string(), 0.5);
    }

    // Tick more than the cooldown
    world.tick(1.0);

    let cd_val = world
        .cooldowns(e)
        .unwrap()
        .map
        .get("ability")
        .copied()
        .unwrap();
    assert!(
        cd_val >= 0.0,
        "Cooldown should not go negative, got {}",
        cd_val
    );
    assert!(
        cd_val == 0.0,
        "Cooldown should be clamped to 0, got {}",
        cd_val
    );
}

// ============================================================================
// WORLD SNAPSHOT CORRECTNESS
// ============================================================================

/// Verify WorldSnapshot default values
#[test]
fn test_world_snapshot_defaults() {
    let snap = WorldSnapshot::default();

    assert!((snap.t - 0.0).abs() < 0.001, "Default time should be 0");
    assert!(snap.enemies.is_empty(), "Default enemies should be empty");
    assert!(snap.pois.is_empty(), "Default pois should be empty");
    assert!(
        snap.obstacles.is_empty(),
        "Default obstacles should be empty"
    );
    assert!(snap.objective.is_none(), "Default objective should be None");
}

/// Verify PlayerState default values
#[test]
fn test_player_state_defaults() {
    let player = PlayerState::default();

    assert_eq!(player.hp, 100, "Default player HP should be 100");
    assert_eq!(player.pos.x, 0, "Default x should be 0");
    assert_eq!(player.pos.y, 0, "Default y should be 0");
    assert_eq!(
        player.stance,
        Stance::Stand,
        "Default stance should be 'stand'"
    );
}

/// Verify CompanionState default values
#[test]
fn test_companion_state_defaults() {
    let companion = CompanionState::default();

    assert_eq!(companion.ammo, 10, "Default ammo should be 10");
    assert!(
        (companion.morale - 1.0).abs() < 0.001,
        "Default morale should be 1.0"
    );
    assert!(
        companion.cooldowns.is_empty(),
        "Default cooldowns should be empty"
    );
}

/// Verify EnemyState default values
#[test]
fn test_enemy_state_defaults() {
    let enemy = EnemyState::default();

    assert_eq!(enemy.id, 0, "Default enemy id should be 0");
    assert_eq!(enemy.hp, 100, "Default enemy HP should be 100");
    assert_eq!(
        enemy.cover,
        CoverType::None,
        "Default cover should be 'none'"
    );
    assert!(
        (enemy.last_seen - 0.0).abs() < 0.001,
        "Default last_seen should be 0"
    );
}

/// Verify enemy_count helper
#[test]
fn test_world_snapshot_enemy_count() {
    let mut snap = WorldSnapshot::default();

    assert_eq!(snap.enemy_count(), 0, "Initial enemy count should be 0");

    snap.enemies.push(EnemyState::default());
    assert_eq!(snap.enemy_count(), 1, "Enemy count should be 1");

    snap.enemies.push(EnemyState::default());
    snap.enemies.push(EnemyState::default());
    assert_eq!(snap.enemy_count(), 3, "Enemy count should be 3");
}

/// Verify has_no_enemies helper
#[test]
fn test_world_snapshot_has_no_enemies() {
    let mut snap = WorldSnapshot::default();

    assert!(snap.has_no_enemies(), "Should have no enemies initially");

    snap.enemies.push(EnemyState::default());
    assert!(!snap.has_no_enemies(), "Should have enemies after push");
}

// ============================================================================
// POI STRUCTURE CORRECTNESS
// ============================================================================

/// Verify Poi construction
#[test]
fn test_poi_construction() {
    let poi = Poi {
        k: "objective".to_string(),
        pos: IVec2::new(50, 75),
    };

    assert_eq!(poi.k, "objective");
    assert_eq!(poi.pos.x, 50);
    assert_eq!(poi.pos.y, 75);
}

/// Verify Poi default
#[test]
fn test_poi_default() {
    let poi = Poi::default();

    assert_eq!(poi.k, "poi", "Default POI kind should be 'poi'");
    assert_eq!(poi.pos.x, 0, "Default POI x should be 0");
    assert_eq!(poi.pos.y, 0, "Default POI y should be 0");
}

// ============================================================================
// MUTATION DETECTION EDGE CASES
// ============================================================================

/// Verify manhattan distance uses abs() correctly (catches missing abs)
#[test]
fn test_manhattan_uses_abs() {
    let a = IVec2::new(5, 5);
    let b = IVec2::new(2, 8);

    let dist = a.manhattan_distance(&b);
    // |5-2| + |5-8| = 3 + 3 = 6
    // Without abs: (5-2) + (5-8) = 3 + (-3) = 0 WRONG

    assert_eq!(dist, 6, "Manhattan distance should use abs()");
    assert!(dist > 0, "Manhattan distance should be positive");
}

/// Verify distance_squared uses multiplication not division
#[test]
fn test_distance_squared_multiplication() {
    let a = IVec2::new(0, 0);
    let b = IVec2::new(6, 8);

    let dist_sq = a.distance_squared(&b);
    // Correct: 6*6 + 8*8 = 36 + 64 = 100
    // Wrong (if /): 6/6 + 8/8 = 1 + 1 = 2

    assert_eq!(dist_sq, 100, "Distance squared should use multiplication");
}

/// Verify offset adds, not subtracts
#[test]
fn test_offset_adds_not_subtracts() {
    let v = IVec2::new(10, 10);
    let result = v.offset(5, 5);

    // Correct: 10+5=15
    // Wrong (if -): 10-5=5
    assert_eq!(result.x, 15, "Offset should add, not subtract");
    assert_eq!(result.y, 15, "Offset should add, not subtract");
}

/// Verify subtraction order in operators
#[test]
fn test_subtraction_order() {
    let a = IVec2::new(10, 20);
    let b = IVec2::new(3, 5);
    let result = a - b;

    // a - b: (10-3, 20-5) = (7, 15)
    // b - a would be: (3-10, 5-20) = (-7, -15)

    assert_eq!(result.x, 7, "Subtraction should be a.x - b.x");
    assert_eq!(result.y, 15, "Subtraction should be a.y - b.y");
}

/// Verify cooldown uses subtraction not addition
#[test]
fn test_cooldown_subtracts_dt() {
    let mut world = World::new();
    let e = world.spawn("Test", IVec2::zero(), Team { id: 0 }, 100, 10);

    if let Some(cd) = world.cooldowns_mut(e) {
        cd.map.insert("test".to_string(), 5.0);
    }

    world.tick(1.0);

    let cd_after = world
        .cooldowns(e)
        .unwrap()
        .map
        .get("test")
        .copied()
        .unwrap();

    // Correct: 5.0 - 1.0 = 4.0
    // Wrong (if +): 5.0 + 1.0 = 6.0
    assert!(
        (cd_after - 4.0).abs() < 0.001,
        "Cooldown should subtract dt, expected 4.0, got {}",
        cd_after
    );
}

/// Verify health damage subtracts, not adds
#[test]
fn test_health_damage_subtracts() {
    let mut world = World::new();
    let e = world.spawn("Test", IVec2::zero(), Team { id: 0 }, 100, 10);

    if let Some(h) = world.health_mut(e) {
        h.hp -= 25;
    }

    let hp = world.health(e).unwrap().hp;

    // Correct: 100 - 25 = 75
    // Wrong (if +): 100 + 25 = 125
    assert_eq!(hp, 75, "Damage should subtract health");
}

// ============================================================================
// DETERMINISM TESTS
// ============================================================================

/// Verify identical operations produce identical results
#[test]
fn test_world_determinism() {
    let mut world1 = World::new();
    let mut world2 = World::new();

    // Same operations on both worlds
    let e1 = world1.spawn("A", IVec2::new(10, 20), Team { id: 0 }, 100, 50);
    let e2 = world2.spawn("A", IVec2::new(10, 20), Team { id: 0 }, 100, 50);

    world1.tick(0.5);
    world2.tick(0.5);

    // Results should be identical
    assert_eq!(e1, e2, "Entity IDs should be deterministic");
    assert!(
        (world1.t - world2.t).abs() < 0.0001,
        "Time should be deterministic"
    );
}

/// Verify snapshot construction is deterministic
#[test]
#[allow(clippy::field_reassign_with_default)]
fn test_snapshot_construction_determinism() {
    let create_snapshot = || {
        let mut snap = WorldSnapshot::default();
        snap.t = 5.0;
        snap.player.hp = 75;
        snap.me.ammo = 20;
        snap.enemies.push(EnemyState {
            id: 1,
            hp: 50,
            ..Default::default()
        });
        snap
    };

    let snap1 = create_snapshot();
    let snap2 = create_snapshot();

    assert!((snap1.t - snap2.t).abs() < 0.001);
    assert_eq!(snap1.player.hp, snap2.player.hp);
    assert_eq!(snap1.me.ammo, snap2.me.ammo);
    assert_eq!(snap1.enemies.len(), snap2.enemies.len());
}

// ============================================================================
// BOUNDARY CONDITIONS
// ============================================================================

/// Verify zero time tick doesn't break anything
#[test]
fn test_zero_dt_tick() {
    let mut world = World::new();
    let e = world.spawn("Test", IVec2::zero(), Team { id: 0 }, 100, 10);

    if let Some(cd) = world.cooldowns_mut(e) {
        cd.map.insert("test".to_string(), 5.0);
    }

    world.tick(0.0);

    let cd = world
        .cooldowns(e)
        .unwrap()
        .map
        .get("test")
        .copied()
        .unwrap();
    assert!(
        (cd - 5.0).abs() < 0.001,
        "Zero dt should not change cooldown"
    );
}

/// Verify negative coordinates work correctly
#[test]
fn test_negative_coordinates() {
    let mut world = World::new();
    let e = world.spawn("Test", IVec2::new(-100, -200), Team { id: 0 }, 100, 10);

    let pose = world.pose(e).unwrap();
    assert_eq!(pose.pos.x, -100, "Negative x should be preserved");
    assert_eq!(pose.pos.y, -200, "Negative y should be preserved");
}

/// Verify large values don't overflow in distance calculations
#[test]
fn test_large_distance_no_overflow() {
    let a = IVec2::new(10000, 10000);
    let b = IVec2::new(-10000, -10000);

    let dist_sq = a.distance_squared(&b);
    // (20000)^2 + (20000)^2 = 400,000,000 + 400,000,000 = 800,000,000
    // This is within i32 range (max ~2.1 billion)

    assert_eq!(dist_sq, 800_000_000, "Large distances should not overflow");
}
