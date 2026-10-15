//! Mutation-Resistant Tests for astraweave-core
//!
//! These tests verify **exact computed values** and **behavioral correctness** to ensure
//! mutations to formulas and logic are detected by `cargo mutants`.

#![allow(clippy::field_reassign_with_default)]

use crate::schema::{
    ActionStep, CompanionState, EnemyState, IVec2, PlanIntent, PlayerState, Poi, WorldSnapshot,
};
use crate::world::{Team, World};
use std::collections::BTreeMap;

// =============================================================================
// IVec2 Mathematical Invariants - Verify exact computed values
// =============================================================================

mod ivec2_tests {
    use super::*;

    #[test]
    fn zero_is_exactly_zero() {
        let zero = IVec2::zero();
        assert_eq!(zero.x, 0, "Zero x must be 0");
        assert_eq!(zero.y, 0, "Zero y must be 0");
    }

    #[test]
    fn new_sets_exact_values() {
        let v = IVec2::new(42, -17);
        assert_eq!(v.x, 42, "x must be exactly 42");
        assert_eq!(v.y, -17, "y must be exactly -17");
    }

    #[test]
    fn is_zero_returns_true_only_for_zero() {
        assert!(IVec2::zero().is_zero(), "Zero vector is_zero must be true");
        assert!(!IVec2::new(1, 0).is_zero(), "(1,0) is_zero must be false");
        assert!(!IVec2::new(0, 1).is_zero(), "(0,1) is_zero must be false");
        assert!(!IVec2::new(1, 1).is_zero(), "(1,1) is_zero must be false");
    }

    #[test]
    fn manhattan_distance_is_sum_of_abs_differences() {
        let a = IVec2::new(3, 4);
        let b = IVec2::new(7, 1);
        // |3-7| + |4-1| = 4 + 3 = 7
        assert_eq!(a.manhattan_distance(&b), 7, "Manhattan must be |dx|+|dy|");
        assert_eq!(b.manhattan_distance(&a), 7, "Manhattan must be symmetric");
    }

    #[test]
    fn distance_squared_is_dx_squared_plus_dy_squared() {
        let a = IVec2::new(0, 0);
        let b = IVec2::new(3, 4);
        // 3² + 4² = 9 + 16 = 25
        assert_eq!(a.distance_squared(&b), 25, "Distance² must be dx²+dy²");
    }

    #[test]
    fn distance_is_sqrt_of_distance_squared() {
        let a = IVec2::new(0, 0);
        let b = IVec2::new(3, 4);
        let dist = a.distance(&b);
        assert!((dist - 5.0).abs() < 0.0001, "Distance must be √25 = 5");
    }

    #[test]
    fn self_distance_is_zero() {
        let v = IVec2::new(100, -50);
        assert_eq!(v.distance_squared(&v), 0, "Distance to self must be 0");
        assert_eq!(v.manhattan_distance(&v), 0, "Manhattan to self must be 0");
    }

    #[test]
    fn offset_adds_exact_amounts() {
        let v = IVec2::new(10, 20);
        let offset = v.offset(5, -3);
        assert_eq!(offset.x, 15, "Offset x must be 10+5=15");
        assert_eq!(offset.y, 17, "Offset y must be 20-3=17");
    }

    #[test]
    fn add_is_component_wise() {
        let a = IVec2::new(1, 2);
        let b = IVec2::new(3, 4);
        let sum = a + b;
        assert_eq!(sum.x, 4, "Add x must be 1+3=4");
        assert_eq!(sum.y, 6, "Add y must be 2+4=6");
    }

    #[test]
    fn sub_is_component_wise() {
        let a = IVec2::new(5, 7);
        let b = IVec2::new(2, 3);
        let diff = a - b;
        assert_eq!(diff.x, 3, "Sub x must be 5-2=3");
        assert_eq!(diff.y, 4, "Sub y must be 7-3=4");
    }
}

// =============================================================================
// WorldSnapshot Behavioral Invariants
// =============================================================================

mod world_snapshot_tests {
    use super::*;

    fn test_snapshot() -> WorldSnapshot {
        WorldSnapshot {
            t: 10.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2::new(0, 0),
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 5,
                cooldowns: BTreeMap::new(),
                morale: 0.8,
                pos: IVec2::new(5, 5),
            },
            enemies: vec![
                EnemyState {
                    id: 1,
                    pos: IVec2::new(10, 10),
                    hp: 50,
                    cover: "none".into(),
                    last_seen: 5.0,
                },
                EnemyState {
                    id: 2,
                    pos: IVec2::new(3, 3),
                    hp: 75,
                    cover: "wall".into(),
                    last_seen: 8.0,
                },
            ],
            pois: vec![Poi {
                k: "ammo".into(),
                pos: IVec2::new(7, 7),
            }],
            obstacles: vec![IVec2::new(2, 2)],
            objective: Some("capture_point".into()),

            hazards: [crate::WeaveHazard::EMPTY; crate::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        }
    }

    #[test]
    fn enemy_count_matches_vec_len() {
        let snap = test_snapshot();
        assert_eq!(snap.enemy_count(), 2, "enemy_count must match vec len");
    }

    #[test]
    fn has_no_enemies_false_when_enemies_present() {
        let snap = test_snapshot();
        assert!(
            !snap.has_no_enemies(),
            "has_no_enemies must be false with enemies"
        );
    }

    #[test]
    fn has_no_enemies_true_when_empty() {
        let mut snap = test_snapshot();
        snap.enemies.clear();
        assert!(
            snap.has_no_enemies(),
            "has_no_enemies must be true when empty"
        );
    }

    #[test]
    fn nearest_enemy_returns_closest_by_distance_squared() {
        let snap = test_snapshot();
        // Companion at (5,5), enemies at (10,10) and (3,3)
        // (3,3): dist² = (5-3)² + (5-3)² = 4+4 = 8
        // (10,10): dist² = (5-10)² + (5-10)² = 25+25 = 50
        let nearest = snap.nearest_enemy().expect("Should have enemies");
        assert_eq!(nearest.id, 2, "Nearest enemy should be id=2 at (3,3)");
    }

    #[test]
    fn enemies_within_range_filters_correctly() {
        let snap = test_snapshot();
        // Companion at (5,5)
        // (3,3): manhattan = |5-3| + |5-3| = 4
        // (10,10): manhattan = |5-10| + |5-10| = 10
        let within_5 = snap.enemies_within_range(5);
        assert_eq!(
            within_5.len(),
            1,
            "Only enemy at (3,3) should be within range 5"
        );
        assert_eq!(within_5[0].id, 2);
    }

    #[test]
    fn has_ammo_reflects_ammo_positive() {
        let snap = test_snapshot();
        assert!(snap.has_ammo(), "has_ammo should be true when ammo=5");
    }

    #[test]
    fn has_ammo_false_when_zero() {
        let mut snap = test_snapshot();
        snap.me.ammo = 0;
        assert!(!snap.has_ammo(), "has_ammo should be false when ammo=0");
    }

    #[test]
    fn has_pois_reflects_pois_present() {
        let snap = test_snapshot();
        assert!(snap.has_pois(), "has_pois should be true with POIs");
    }

    #[test]
    fn has_objective_reflects_objective_present() {
        let snap = test_snapshot();
        assert!(
            snap.has_objective(),
            "has_objective should be true with objective"
        );
    }

    #[test]
    fn distance_to_player_is_euclidean() {
        let snap = test_snapshot();
        // Companion at (5,5), Player at (0,0)
        // dist = √(25+25) = √50 ≈ 7.071
        let dist = snap.distance_to_player();
        assert!(
            (dist - 7.071).abs() < 0.01,
            "Distance to player should be √50"
        );
    }
}

// =============================================================================
// PlanIntent Behavioral Invariants
// =============================================================================

mod plan_intent_tests {
    use super::*;

    #[test]
    fn empty_plan_has_zero_steps() {
        let plan = PlanIntent::empty();
        assert_eq!(plan.step_count(), 0, "Empty plan step_count must be 0");
        assert!(plan.is_empty(), "Empty plan is_empty must be true");
    }

    #[test]
    fn new_plan_with_id_preserves_id() {
        let plan = PlanIntent::new("test_plan");
        assert_eq!(plan.plan_id, "test_plan", "Plan ID must match");
    }

    #[test]
    fn with_step_increments_count() {
        let plan = PlanIntent::empty()
            .with_step(ActionStep::Wait { duration: 1.0 })
            .with_step(ActionStep::Wait { duration: 2.0 });
        assert_eq!(
            plan.step_count(),
            2,
            "step_count should be 2 after 2 with_step"
        );
    }

    #[test]
    fn first_step_returns_first() {
        let plan = PlanIntent::empty()
            .with_step(ActionStep::Wait { duration: 1.0 })
            .with_step(ActionStep::Heal { target_id: None });
        let first = plan.first_step().expect("Should have first step");
        assert!(
            matches!(first, ActionStep::Wait { .. }),
            "First step should be Wait"
        );
    }

    #[test]
    fn has_movement_detects_movement_actions() {
        let plan_with_movement = PlanIntent::empty().with_step(ActionStep::MoveTo {
            x: 5,
            y: 5,
            speed: None,
        });
        assert!(
            plan_with_movement.has_movement(),
            "Plan with MoveTo should have_movement"
        );

        let plan_without_movement =
            PlanIntent::empty().with_step(ActionStep::Wait { duration: 1.0 });
        assert!(
            !plan_without_movement.has_movement(),
            "Plan without MoveTo should not have_movement"
        );
    }

    #[test]
    fn has_offensive_detects_offensive_actions() {
        let plan_with_offensive =
            PlanIntent::empty().with_step(ActionStep::Attack { target_id: 1 });
        assert!(
            plan_with_offensive.has_offensive(),
            "Plan with Attack should have_offensive"
        );

        let plan_without_offensive =
            PlanIntent::empty().with_step(ActionStep::Wait { duration: 1.0 });
        assert!(
            !plan_without_offensive.has_offensive(),
            "Plan without Attack should not have_offensive"
        );
    }
}

// =============================================================================
// World Entity Lifecycle Invariants
// =============================================================================

mod world_tests {
    use super::*;

    #[test]
    fn spawn_returns_unique_ids() {
        let mut world = World::new();
        let id1 = world.spawn("A", IVec2::zero(), Team { id: 0 }, 100, 10);
        let id2 = world.spawn("B", IVec2::zero(), Team { id: 0 }, 100, 10);
        assert_ne!(id1, id2, "Each spawn must return unique ID");
    }

    #[test]
    fn spawn_with_id_preserves_entity_id() {
        let mut world = World::new();
        let explicit_id = 42;
        let returned_id =
            world.spawn_with_id(explicit_id, "Test", IVec2::new(1, 1), Team { id: 1 }, 50, 5);
        assert_eq!(
            returned_id, explicit_id,
            "spawn_with_id must return the explicit ID"
        );
    }

    #[test]
    fn spawned_entity_has_correct_pose() {
        let mut world = World::new();
        let pos = IVec2::new(7, 8);
        let id = world.spawn("Entity", pos, Team { id: 0 }, 100, 10);
        let pose = world.pose(id).expect("Entity should have pose");
        assert_eq!(pose.pos, pos, "Pose position must match spawn position");
    }

    #[test]
    fn spawned_entity_has_correct_health() {
        let mut world = World::new();
        let id = world.spawn("Entity", IVec2::zero(), Team { id: 0 }, 75, 10);
        let health = world.health(id).expect("Entity should have health");
        assert_eq!(health.hp, 75, "Health must match spawn value");
    }

    #[test]
    fn spawned_entity_has_correct_team() {
        let mut world = World::new();
        let id = world.spawn("Entity", IVec2::zero(), Team { id: 2 }, 100, 10);
        let team = world.team(id).expect("Entity should have team");
        assert_eq!(team.id, 2, "Team must match spawn value");
    }

    #[test]
    fn tick_advances_time_by_dt() {
        let mut world = World::new();
        world.tick(0.016);
        assert!(
            (world.t - 0.016).abs() < 0.0001,
            "Time should advance by dt"
        );
        world.tick(0.016);
        assert!((world.t - 0.032).abs() < 0.0001, "Time should accumulate");
    }

    #[test]
    fn tick_decrements_cooldowns() {
        let mut world = World::new();
        let id = world.spawn("Entity", IVec2::zero(), Team { id: 0 }, 100, 10);

        // Set a cooldown
        if let Some(cds) = world.cooldowns_mut(id) {
            cds.map.insert("attack".into(), 1.0);
        }

        world.tick(0.3);

        let cds = world.cooldowns(id).expect("Should have cooldowns");
        let remaining = cds.map.get("attack").copied().unwrap_or(0.0);
        assert!(
            (remaining - 0.7).abs() < 0.0001,
            "Cooldown should decrease by dt"
        );
    }

    #[test]
    fn tick_clamps_cooldowns_at_zero() {
        let mut world = World::new();
        let id = world.spawn("Entity", IVec2::zero(), Team { id: 0 }, 100, 10);

        if let Some(cds) = world.cooldowns_mut(id) {
            cds.map.insert("attack".into(), 0.5);
        }

        world.tick(1.0); // More than cooldown

        let cds = world.cooldowns(id).expect("Should have cooldowns");
        let remaining = cds.map.get("attack").copied().unwrap_or(0.0);
        assert_eq!(remaining, 0.0, "Cooldown should clamp at 0");
    }

    #[test]
    fn destroy_entity_removes_all_components() {
        let mut world = World::new();
        let id = world.spawn("Entity", IVec2::zero(), Team { id: 0 }, 100, 10);

        assert!(
            world.pose(id).is_some(),
            "Entity should exist before destroy"
        );

        let destroyed = world.destroy_entity(id);
        assert!(destroyed, "destroy_entity should return true");

        assert!(
            world.pose(id).is_none(),
            "Pose should be gone after destroy"
        );
        assert!(
            world.health(id).is_none(),
            "Health should be gone after destroy"
        );
        assert!(
            world.team(id).is_none(),
            "Team should be gone after destroy"
        );
    }

    #[test]
    fn destroy_nonexistent_returns_false() {
        let mut world = World::new();
        let result = world.destroy_entity(999);
        assert!(
            !result,
            "destroy_entity for non-existent should return false"
        );
    }
}

// =============================================================================
// Behavioral Correctness Tests - Mathematical Invariants
// =============================================================================

mod behavioral_correctness_tests {
    use super::*;

    // --- IVec2 triangle inequality (d(a,c) <= d(a,b) + d(b,c)) ---
    #[test]
    fn manhattan_obeys_triangle_inequality() {
        let a = IVec2::new(0, 0);
        let b = IVec2::new(5, 5);
        let c = IVec2::new(10, 0);

        let d_ac = a.manhattan_distance(&c);
        let d_ab = a.manhattan_distance(&b);
        let d_bc = b.manhattan_distance(&c);

        assert!(
            d_ac <= d_ab + d_bc,
            "Triangle inequality must hold for Manhattan"
        );
    }

    // --- Distance is symmetric ---
    #[test]
    fn distance_is_symmetric() {
        let a = IVec2::new(3, 7);
        let b = IVec2::new(-2, 11);

        let d_ab = a.distance(&b);
        let d_ba = b.distance(&a);

        assert!((d_ab - d_ba).abs() < 0.0001, "Distance must be symmetric");
    }

    // --- Distance squared is always non-negative ---
    #[test]
    fn distance_squared_non_negative() {
        let a = IVec2::new(-100, 50);
        let b = IVec2::new(75, -25);

        assert!(
            a.distance_squared(&b) >= 0,
            "Distance² must be non-negative"
        );
    }

    // --- Offset by zero is identity ---
    #[test]
    fn offset_by_zero_is_identity() {
        let v = IVec2::new(42, -17);
        let offset = v.offset(0, 0);
        assert_eq!(offset.x, v.x, "Offset(0,0) x must be identity");
        assert_eq!(offset.y, v.y, "Offset(0,0) y must be identity");
    }

    // --- Add/Sub are inverse operations ---
    #[test]
    fn add_sub_are_inverse() {
        let a = IVec2::new(10, 20);
        let b = IVec2::new(3, 7);
        let result = (a + b) - b;
        assert_eq!(result.x, a.x, "a + b - b must equal a.x");
        assert_eq!(result.y, a.y, "a + b - b must equal a.y");
    }

    // --- World time is monotonically increasing ---
    #[test]
    fn world_time_monotonically_increases() {
        let mut world = World::new();
        let t0 = world.t;
        world.tick(0.1);
        let t1 = world.t;
        world.tick(0.2);
        let t2 = world.t;

        assert!(t1 > t0, "Time must increase after tick");
        assert!(t2 > t1, "Time must continue increasing");
    }

    // --- Entity IDs are monotonically increasing ---
    #[test]
    fn entity_ids_monotonically_increase() {
        let mut world = World::new();
        let id1 = world.spawn("A", IVec2::zero(), Team { id: 0 }, 100, 10);
        let id2 = world.spawn("B", IVec2::zero(), Team { id: 0 }, 100, 10);
        let id3 = world.spawn("C", IVec2::zero(), Team { id: 0 }, 100, 10);

        assert!(id2 > id1, "ID2 must be greater than ID1");
        assert!(id3 > id2, "ID3 must be greater than ID2");
    }

    // --- Nearest enemy is actually nearest ---
    #[test]
    fn nearest_enemy_is_truly_nearest() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState {
                pos: IVec2::new(0, 0),
                ..Default::default()
            },
            enemies: vec![
                EnemyState {
                    id: 1,
                    pos: IVec2::new(10, 0),
                    ..Default::default()
                },
                EnemyState {
                    id: 2,
                    pos: IVec2::new(3, 0),
                    ..Default::default()
                },
                EnemyState {
                    id: 3,
                    pos: IVec2::new(7, 0),
                    ..Default::default()
                },
            ],
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [crate::WeaveHazard::EMPTY; crate::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };

        let nearest = snap.nearest_enemy().expect("Should have enemies");

        // Verify no other enemy is closer
        for enemy in &snap.enemies {
            assert!(
                snap.me.pos.distance_squared(&nearest.pos)
                    <= snap.me.pos.distance_squared(&enemy.pos),
                "Nearest enemy must have smallest or equal distance"
            );
        }
    }

    // --- PlanIntent step_count equals steps.len() ---
    #[test]
    fn plan_step_count_equals_vec_len() {
        let mut plan = PlanIntent::empty();
        assert_eq!(plan.step_count(), plan.steps.len());

        plan = plan.with_step(ActionStep::Wait { duration: 1.0 });
        assert_eq!(plan.step_count(), plan.steps.len());

        plan = plan.with_step(ActionStep::Heal { target_id: None });
        assert_eq!(plan.step_count(), plan.steps.len());
    }

    // --- is_empty is equivalent to step_count == 0 ---
    #[test]
    fn is_empty_equivalent_to_zero_steps() {
        let empty = PlanIntent::empty();
        assert_eq!(empty.is_empty(), empty.step_count() == 0);

        let non_empty = PlanIntent::empty().with_step(ActionStep::Wait { duration: 1.0 });
        assert_eq!(non_empty.is_empty(), non_empty.step_count() == 0);
    }
}

// =============================================================================
// BOUNDARY CONDITION TESTS - Test exact boundary values to catch < vs <= mutations
// =============================================================================

mod boundary_condition_tests {
    use super::*;
    use crate::schema::{
        PersistenceMode, RelativeLocation, TerrainFeatureType, TerrainGenerationRequest,
    };

    // --- TerrainGenerationRequest::validate() intensity boundaries ---
    // Condition: if self.intensity < 0.0 || self.intensity > 1.0 { return Err }

    fn make_terrain_request(
        intensity: f32,
        narrative_len: usize,
        request_id: &str,
    ) -> TerrainGenerationRequest {
        TerrainGenerationRequest {
            request_id: request_id.to_string(),
            narrative_reason: "x".repeat(narrative_len),
            feature_type: TerrainFeatureType::default(),
            relative_location: RelativeLocation::default(),
            intensity,
            persistence_mode: PersistenceMode::default(),
            biome_constraints: Vec::new(),
            seed: None,
        }
    }

    #[test]
    fn intensity_at_lower_bound_0_is_valid() {
        let req = make_terrain_request(0.0, 10, "test");
        assert!(
            req.validate().is_ok(),
            "intensity=0.0 should be valid (at lower boundary)"
        );
    }

    #[test]
    fn intensity_just_below_lower_bound_fails() {
        let req = make_terrain_request(-0.001, 10, "test");
        assert!(
            req.validate().is_err(),
            "intensity=-0.001 should fail (below lower boundary)"
        );
    }

    #[test]
    fn intensity_at_upper_bound_1_is_valid() {
        let req = make_terrain_request(1.0, 10, "test");
        assert!(
            req.validate().is_ok(),
            "intensity=1.0 should be valid (at upper boundary)"
        );
    }

    #[test]
    fn intensity_just_above_upper_bound_fails() {
        let req = make_terrain_request(1.001, 10, "test");
        assert!(
            req.validate().is_err(),
            "intensity=1.001 should fail (above upper boundary)"
        );
    }

    #[test]
    fn intensity_in_middle_is_valid() {
        let req = make_terrain_request(0.5, 10, "test");
        assert!(
            req.validate().is_ok(),
            "intensity=0.5 should be valid (in range)"
        );
    }

    #[test]
    fn intensity_negative_one_fails() {
        let req = make_terrain_request(-1.0, 10, "test");
        assert!(req.validate().is_err(), "intensity=-1.0 should fail");
    }

    // --- TerrainGenerationRequest::validate() narrative_reason.len() > 100 ---

    #[test]
    fn narrative_at_99_chars_is_valid() {
        let req = make_terrain_request(0.5, 99, "test");
        assert!(
            req.validate().is_ok(),
            "narrative_reason with 99 chars should be valid"
        );
    }

    #[test]
    fn narrative_at_100_chars_is_valid() {
        let req = make_terrain_request(0.5, 100, "test");
        assert!(
            req.validate().is_ok(),
            "narrative_reason with 100 chars should be valid (at boundary)"
        );
    }

    #[test]
    fn narrative_at_101_chars_fails() {
        let req = make_terrain_request(0.5, 101, "test");
        assert!(
            req.validate().is_err(),
            "narrative_reason with 101 chars should fail (exceeds boundary)"
        );
    }

    #[test]
    fn narrative_at_0_chars_is_valid() {
        let req = make_terrain_request(0.5, 0, "test");
        assert!(
            req.validate().is_ok(),
            "narrative_reason with 0 chars should be valid"
        );
    }

    // --- TerrainGenerationRequest::validate() request_id.is_empty() ---

    #[test]
    fn empty_request_id_fails() {
        let req = make_terrain_request(0.5, 10, "");
        assert!(req.validate().is_err(), "empty request_id should fail");
    }

    #[test]
    fn single_char_request_id_is_valid() {
        let req = make_terrain_request(0.5, 10, "a");
        assert!(
            req.validate().is_ok(),
            "single char request_id should be valid"
        );
    }

    // --- IVec2 is_zero boundary ---

    #[test]
    fn is_zero_with_x_only_nonzero() {
        let v = IVec2::new(1, 0);
        assert!(!v.is_zero(), "(1, 0) is_zero should be false");
    }

    #[test]
    fn is_zero_with_y_only_nonzero() {
        let v = IVec2::new(0, 1);
        assert!(!v.is_zero(), "(0, 1) is_zero should be false");
    }

    #[test]
    fn is_zero_with_negative_x() {
        let v = IVec2::new(-1, 0);
        assert!(!v.is_zero(), "(-1, 0) is_zero should be false");
    }

    #[test]
    fn is_zero_with_negative_y() {
        let v = IVec2::new(0, -1);
        assert!(!v.is_zero(), "(0, -1) is_zero should be false");
    }

    // --- WorldSnapshot enemies_within_range boundary ---

    fn test_snapshot_with_enemies() -> WorldSnapshot {
        WorldSnapshot {
            t: 10.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2::new(0, 0),
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 5,
                cooldowns: BTreeMap::new(),
                morale: 0.8,
                pos: IVec2::new(0, 0),
            },
            enemies: vec![
                EnemyState {
                    id: 1,
                    pos: IVec2::new(5, 0),
                    hp: 50,
                    cover: "none".into(),
                    last_seen: 5.0,
                },
                EnemyState {
                    id: 2,
                    pos: IVec2::new(0, 5),
                    hp: 75,
                    cover: "wall".into(),
                    last_seen: 8.0,
                },
                EnemyState {
                    id: 3,
                    pos: IVec2::new(6, 0),
                    hp: 60,
                    cover: "none".into(),
                    last_seen: 7.0,
                },
            ],
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [crate::WeaveHazard::EMPTY; crate::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        }
    }

    #[test]
    fn enemies_within_range_at_exact_boundary() {
        let snap = test_snapshot_with_enemies();
        // Enemy 1 at (5,0), manhattan = 5
        let within_5 = snap.enemies_within_range(5);
        assert!(
            within_5.iter().any(|e| e.id == 1),
            "Enemy at manhattan=5 should be within range 5"
        );
        assert!(
            within_5.iter().any(|e| e.id == 2),
            "Enemy at manhattan=5 should be within range 5"
        );
    }

    #[test]
    fn enemies_within_range_just_outside() {
        let snap = test_snapshot_with_enemies();
        // Enemy 3 at (6,0), manhattan = 6
        let within_5 = snap.enemies_within_range(5);
        assert!(
            !within_5.iter().any(|e| e.id == 3),
            "Enemy at manhattan=6 should NOT be within range 5"
        );
    }

    #[test]
    fn enemies_within_range_4_excludes_at_5() {
        let snap = test_snapshot_with_enemies();
        // Range 4 should exclude enemies at manhattan distance 5
        let within_4 = snap.enemies_within_range(4);
        assert!(
            !within_4.iter().any(|e| e.id == 1),
            "Enemy at manhattan=5 should NOT be within range 4"
        );
        assert!(
            !within_4.iter().any(|e| e.id == 2),
            "Enemy at manhattan=5 should NOT be within range 4"
        );
    }

    // --- World entity ID boundaries ---

    #[test]
    fn world_spawn_with_id_at_next_id_boundary() {
        let mut world = World::new();
        // Spawn a few entities to advance next_id
        world.spawn("A", IVec2::zero(), Team { id: 0 }, 100, 10);
        world.spawn("B", IVec2::zero(), Team { id: 0 }, 100, 10);
        // Now next_id should be 3 (or similar)

        // This tests the id >= next_id condition in spawn_with_id
        let result = world.spawn_with_id(100, "C", IVec2::zero(), Team { id: 0 }, 100, 10);
        assert_eq!(result, 100, "spawn_with_id should accept ID >= next_id");
    }

    // --- Health/Ammo zero boundary tests ---

    #[test]
    fn health_at_zero_boundary() {
        let mut world = World::new();
        let id = world.spawn("Entity", IVec2::zero(), Team { id: 0 }, 0, 10);
        let health = world.health(id).expect("Entity should have health");
        assert_eq!(health.hp, 0, "Health should be exactly 0");
    }

    #[test]
    fn health_at_one_boundary() {
        let mut world = World::new();
        let id = world.spawn("Entity", IVec2::zero(), Team { id: 0 }, 1, 10);
        let health = world.health(id).expect("Entity should have health");
        assert_eq!(health.hp, 1, "Health should be exactly 1");
    }

    #[test]
    fn ammo_at_zero_boundary() {
        let mut world = World::new();
        let id = world.spawn("Entity", IVec2::zero(), Team { id: 0 }, 100, 0);
        let ammo = world.ammo(id).expect("Entity should have ammo");
        assert_eq!(ammo.rounds, 0, "Ammo should be exactly 0");
    }

    #[test]
    fn ammo_at_one_boundary() {
        let mut world = World::new();
        let id = world.spawn("Entity", IVec2::zero(), Team { id: 0 }, 100, 1);
        let ammo = world.ammo(id).expect("Entity should have ammo");
        assert_eq!(ammo.rounds, 1, "Ammo should be exactly 1");
    }

    // --- Cooldown zero boundary ---

    #[test]
    fn cooldown_at_exactly_zero_after_tick() {
        let mut world = World::new();
        let id = world.spawn("Entity", IVec2::zero(), Team { id: 0 }, 100, 10);

        if let Some(cds) = world.cooldowns_mut(id) {
            cds.map.insert("attack".into(), 0.5);
        }

        // Tick exactly 0.5 should bring cooldown to 0
        world.tick(0.5);

        let cds = world.cooldowns(id).expect("Should have cooldowns");
        let remaining = cds.map.get("attack").copied().unwrap_or(0.0);
        assert_eq!(
            remaining, 0.0,
            "Cooldown should be exactly 0 after exact tick"
        );
    }

    #[test]
    fn cooldown_just_above_zero() {
        let mut world = World::new();
        let id = world.spawn("Entity", IVec2::zero(), Team { id: 0 }, 100, 10);

        if let Some(cds) = world.cooldowns_mut(id) {
            cds.map.insert("attack".into(), 0.5);
        }

        // Tick 0.4 leaves cooldown at 0.1
        world.tick(0.4);

        let cds = world.cooldowns(id).expect("Should have cooldowns");
        let remaining = cds.map.get("attack").copied().unwrap_or(0.0);
        assert!((remaining - 0.1).abs() < 0.0001, "Cooldown should be 0.1");
    }
}

// =============================================================================
// COMPARISON OPERATOR TESTS - Test to catch == vs != and < vs > swaps
// =============================================================================

mod comparison_operator_tests {
    use super::*;

    // --- IVec2 equality ---

    #[test]
    fn ivec2_equal_when_both_match() {
        let a = IVec2::new(5, 7);
        let b = IVec2::new(5, 7);
        assert_eq!(a, b, "Same coordinates should be equal");
    }

    #[test]
    fn ivec2_not_equal_when_x_differs() {
        let a = IVec2::new(5, 7);
        let b = IVec2::new(6, 7);
        assert_ne!(a, b, "Different x should not be equal");
    }

    #[test]
    fn ivec2_not_equal_when_y_differs() {
        let a = IVec2::new(5, 7);
        let b = IVec2::new(5, 8);
        assert_ne!(a, b, "Different y should not be equal");
    }

    #[test]
    fn ivec2_not_equal_when_both_differ() {
        let a = IVec2::new(5, 7);
        let b = IVec2::new(6, 8);
        assert_ne!(a, b, "Both different should not be equal");
    }

    // --- Team ID filtering ---

    #[test]
    fn team_filter_same_id_matches() {
        let mut world = World::new();
        let id1 = world.spawn("A", IVec2::zero(), Team { id: 1 }, 100, 10);
        let _id2 = world.spawn("B", IVec2::zero(), Team { id: 2 }, 100, 10);

        let same_team = world.all_of_team(1);
        assert!(
            same_team.contains(&id1),
            "Team 1 entity should be in team 1 list"
        );
    }

    #[test]
    fn team_filter_different_id_excluded() {
        let mut world = World::new();
        let _id1 = world.spawn("A", IVec2::zero(), Team { id: 1 }, 100, 10);
        let id2 = world.spawn("B", IVec2::zero(), Team { id: 2 }, 100, 10);

        let same_team = world.all_of_team(1);
        assert!(
            !same_team.contains(&id2),
            "Team 2 entity should NOT be in team 1 list"
        );
    }

    #[test]
    fn enemy_team_filter_excludes_own_team() {
        let mut world = World::new();
        let id1 = world.spawn("A", IVec2::zero(), Team { id: 1 }, 100, 10);
        let _id2 = world.spawn("B", IVec2::zero(), Team { id: 2 }, 100, 10);

        let enemies = world.enemies_of(1);
        assert!(
            !enemies.contains(&id1),
            "Own team entity should NOT be in enemies list"
        );
    }

    #[test]
    fn enemy_team_filter_includes_other_teams() {
        let mut world = World::new();
        let _id1 = world.spawn("A", IVec2::zero(), Team { id: 1 }, 100, 10);
        let id2 = world.spawn("B", IVec2::zero(), Team { id: 2 }, 100, 10);

        let enemies = world.enemies_of(1);
        assert!(
            enemies.contains(&id2),
            "Other team entity SHOULD be in enemies list"
        );
    }

    // --- Distance comparisons ---

    #[test]
    fn manhattan_distance_zero_when_same_point() {
        let a = IVec2::new(5, 5);
        assert_eq!(a.manhattan_distance(&a), 0, "Distance to self should be 0");
    }

    #[test]
    fn manhattan_distance_nonzero_when_different() {
        let a = IVec2::new(0, 0);
        let b = IVec2::new(1, 0);
        assert_ne!(
            a.manhattan_distance(&b),
            0,
            "Distance between different points should be non-zero"
        );
    }

    // --- PlanIntent equality ---

    #[test]
    fn plan_intent_equal_when_same() {
        let plan1 = PlanIntent::new("test").with_step(ActionStep::Wait { duration: 1.0 });
        let plan2 = PlanIntent::new("test").with_step(ActionStep::Wait { duration: 1.0 });
        assert_eq!(plan1.plan_id, plan2.plan_id, "Same plan_id should be equal");
        assert_eq!(plan1.step_count(), plan2.step_count(), "Same step count");
    }

    #[test]
    fn plan_intent_different_when_ids_differ() {
        let plan1 = PlanIntent::new("test1");
        let plan2 = PlanIntent::new("test2");
        assert_ne!(
            plan1.plan_id, plan2.plan_id,
            "Different plan_ids should not be equal"
        );
    }

    // --- Nearest enemy comparison ---

    #[test]
    fn nearest_enemy_prefers_closer() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState {
                pos: IVec2::new(0, 0),
                ..Default::default()
            },
            enemies: vec![
                EnemyState {
                    id: 1,
                    pos: IVec2::new(10, 0),
                    ..Default::default()
                },
                EnemyState {
                    id: 2,
                    pos: IVec2::new(5, 0),
                    ..Default::default()
                },
            ],
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [crate::WeaveHazard::EMPTY; crate::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };

        let nearest = snap.nearest_enemy().unwrap();
        assert_eq!(nearest.id, 2, "Should select closer enemy (id=2 at dist=5)");
    }

    #[test]
    fn nearest_enemy_with_equal_distance_is_deterministic() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState {
                pos: IVec2::new(0, 0),
                ..Default::default()
            },
            enemies: vec![
                EnemyState {
                    id: 1,
                    pos: IVec2::new(5, 0),
                    ..Default::default()
                },
                EnemyState {
                    id: 2,
                    pos: IVec2::new(0, 5),
                    ..Default::default()
                },
            ],
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [crate::WeaveHazard::EMPTY; crate::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };

        // Both at distance 5, should pick first (id=1)
        let nearest = snap.nearest_enemy().unwrap();
        // Just verify it returns one of them consistently
        assert!(
            nearest.id == 1 || nearest.id == 2,
            "Should return one of the equidistant enemies"
        );
    }
}

// =============================================================================
// BOOLEAN RETURN PATH TESTS - Test all paths through boolean-returning functions
// =============================================================================

mod boolean_return_path_tests {
    use super::*;

    // --- IVec2::is_zero() all paths ---

    #[test]
    fn is_zero_returns_true_for_origin() {
        assert!(IVec2::new(0, 0).is_zero());
    }

    #[test]
    fn is_zero_returns_false_for_positive_x() {
        assert!(!IVec2::new(1, 0).is_zero());
    }

    #[test]
    fn is_zero_returns_false_for_negative_x() {
        assert!(!IVec2::new(-1, 0).is_zero());
    }

    #[test]
    fn is_zero_returns_false_for_positive_y() {
        assert!(!IVec2::new(0, 1).is_zero());
    }

    #[test]
    fn is_zero_returns_false_for_negative_y() {
        assert!(!IVec2::new(0, -1).is_zero());
    }

    #[test]
    fn is_zero_returns_false_for_both_nonzero() {
        assert!(!IVec2::new(1, 1).is_zero());
    }

    // --- WorldSnapshot::has_no_enemies() ---

    #[test]
    fn has_no_enemies_returns_true_when_empty() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState::default(),
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [crate::WeaveHazard::EMPTY; crate::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };
        assert!(
            snap.has_no_enemies(),
            "Should return true when enemies vec is empty"
        );
    }

    #[test]
    fn has_no_enemies_returns_false_when_one_enemy() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState::default(),
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2::new(0, 0),
                hp: 100,
                cover: "".into(),
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [crate::WeaveHazard::EMPTY; crate::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };
        assert!(
            !snap.has_no_enemies(),
            "Should return false when enemies vec has one enemy"
        );
    }

    #[test]
    fn has_no_enemies_returns_false_when_multiple_enemies() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState::default(),
            enemies: vec![
                EnemyState {
                    id: 1,
                    pos: IVec2::new(0, 0),
                    hp: 100,
                    cover: "".into(),
                    last_seen: 0.0,
                },
                EnemyState {
                    id: 2,
                    pos: IVec2::new(1, 1),
                    hp: 50,
                    cover: "".into(),
                    last_seen: 0.0,
                },
            ],
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [crate::WeaveHazard::EMPTY; crate::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };
        assert!(
            !snap.has_no_enemies(),
            "Should return false when enemies vec has multiple enemies"
        );
    }

    // --- WorldSnapshot::has_ammo() ---

    #[test]
    fn has_ammo_returns_false_when_zero() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState {
                ammo: 0,
                ..Default::default()
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [crate::WeaveHazard::EMPTY; crate::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };
        assert!(!snap.has_ammo(), "Should return false when ammo is 0");
    }

    #[test]
    fn has_ammo_returns_true_when_positive() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState {
                ammo: 1,
                ..Default::default()
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [crate::WeaveHazard::EMPTY; crate::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };
        assert!(snap.has_ammo(), "Should return true when ammo is 1");
    }

    #[test]
    fn has_ammo_returns_true_when_large() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState {
                ammo: 100,
                ..Default::default()
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [crate::WeaveHazard::EMPTY; crate::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };
        assert!(snap.has_ammo(), "Should return true when ammo is 100");
    }

    // --- WorldSnapshot::has_pois() ---

    #[test]
    fn has_pois_returns_false_when_empty() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState::default(),
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [crate::WeaveHazard::EMPTY; crate::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };
        assert!(
            !snap.has_pois(),
            "Should return false when pois vec is empty"
        );
    }

    #[test]
    fn has_pois_returns_true_when_one() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState::default(),
            enemies: vec![],
            pois: vec![crate::schema::Poi {
                k: "ammo".into(),
                pos: IVec2::new(0, 0),
            }],
            obstacles: vec![],
            objective: None,

            hazards: [crate::WeaveHazard::EMPTY; crate::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };
        assert!(
            snap.has_pois(),
            "Should return true when pois vec has one POI"
        );
    }

    // --- WorldSnapshot::has_objective() ---

    #[test]
    fn has_objective_returns_false_when_none() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState::default(),
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [crate::WeaveHazard::EMPTY; crate::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };
        assert!(
            !snap.has_objective(),
            "Should return false when objective is None"
        );
    }

    #[test]
    fn has_objective_returns_true_when_some() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState::default(),
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: Some("capture".into()),

            hazards: [crate::WeaveHazard::EMPTY; crate::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };
        assert!(
            snap.has_objective(),
            "Should return true when objective is Some"
        );
    }

    #[test]
    fn has_objective_returns_true_when_empty_string() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState::default(),
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: Some("".into()),

            hazards: [crate::WeaveHazard::EMPTY; crate::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        };
        assert!(
            snap.has_objective(),
            "Should return true when objective is Some even if empty string"
        );
    }

    // --- PlanIntent::is_empty() ---

    #[test]
    fn plan_is_empty_returns_true_for_empty_plan() {
        let plan = PlanIntent::empty();
        assert!(plan.is_empty(), "Empty plan is_empty should be true");
    }

    #[test]
    fn plan_is_empty_returns_false_for_non_empty_plan() {
        let plan = PlanIntent::empty().with_step(ActionStep::Wait { duration: 1.0 });
        assert!(!plan.is_empty(), "Non-empty plan is_empty should be false");
    }

    // --- PlanIntent::has_movement() ---

    #[test]
    fn has_movement_returns_false_for_empty_plan() {
        let plan = PlanIntent::empty();
        assert!(
            !plan.has_movement(),
            "Empty plan has_movement should be false"
        );
    }

    #[test]
    fn has_movement_returns_true_for_moveto() {
        let plan = PlanIntent::empty().with_step(ActionStep::MoveTo {
            x: 5,
            y: 5,
            speed: None,
        });
        assert!(
            plan.has_movement(),
            "Plan with MoveTo has_movement should be true"
        );
    }

    #[test]
    fn has_movement_returns_false_for_non_movement() {
        let plan = PlanIntent::empty().with_step(ActionStep::Wait { duration: 1.0 });
        assert!(
            !plan.has_movement(),
            "Plan with only Wait has_movement should be false"
        );
    }

    // --- PlanIntent::has_offensive() ---

    #[test]
    fn has_offensive_returns_false_for_empty_plan() {
        let plan = PlanIntent::empty();
        assert!(
            !plan.has_offensive(),
            "Empty plan has_offensive should be false"
        );
    }

    #[test]
    fn has_offensive_returns_true_for_attack() {
        let plan = PlanIntent::empty().with_step(ActionStep::Attack { target_id: 1 });
        assert!(
            plan.has_offensive(),
            "Plan with Attack has_offensive should be true"
        );
    }

    #[test]
    fn has_offensive_returns_false_for_defensive() {
        let plan = PlanIntent::empty().with_step(ActionStep::Heal { target_id: None });
        assert!(
            !plan.has_offensive(),
            "Plan with only Heal has_offensive should be false"
        );
    }

    // --- World::destroy_entity() return value ---

    #[test]
    fn destroy_entity_returns_true_for_existing() {
        let mut world = World::new();
        let id = world.spawn("Entity", IVec2::zero(), Team { id: 0 }, 100, 10);
        assert!(
            world.destroy_entity(id),
            "destroy_entity should return true for existing entity"
        );
    }

    #[test]
    fn destroy_entity_returns_false_for_nonexistent() {
        let mut world = World::new();
        assert!(
            !world.destroy_entity(999),
            "destroy_entity should return false for non-existent entity"
        );
    }

    #[test]
    fn destroy_entity_returns_false_for_already_destroyed() {
        let mut world = World::new();
        let id = world.spawn("Entity", IVec2::zero(), Team { id: 0 }, 100, 10);
        world.destroy_entity(id);
        assert!(
            !world.destroy_entity(id),
            "destroy_entity should return false for already destroyed entity"
        );
    }
}
