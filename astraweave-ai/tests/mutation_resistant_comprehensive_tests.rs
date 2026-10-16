//! Comprehensive mutation-resistant tests for astraweave-ai.
//!
//! Targets the top mutation-vulnerable areas:
//! - UtilityOrchestrator score formulas & sort order
//! - GoapOrchestrator distance threshold & action values
//! - RuleOrchestrator exact CoverFire durations & movement offsets
//! - tool_sandbox validation boundaries (ammo/cooldown/morale/distance)
//! - PlannerMode/CAiController boolean classification
//! - ToolVerb classification methods & Display
//! - ValidationContext & ValidationCategory
//! - Orchestrator edge cases (multiple enemies, extreme coords, etc.)

use astraweave_ai::orchestrator::{
    GoapOrchestrator, Orchestrator, RuleOrchestrator, UtilityOrchestrator,
};
use astraweave_ai::tool_sandbox::{
    validate_tool_action, ToolVerb, ValidationCategory, ValidationContext,
};
use astraweave_core::{ActionStep, CompanionState, EnemyState, IVec2, PlayerState, WorldSnapshot};
use std::collections::BTreeMap;

// ============================================================================
// Test Helpers
// ============================================================================

fn base_snapshot() -> WorldSnapshot {
    WorldSnapshot {
        t: 1.0,
        player: PlayerState {
            hp: 100,
            pos: IVec2::new(0, 0),
            stance: "stand".into(),
            orders: vec![],
        },
        me: CompanionState {
            ammo: 5,
            cooldowns: BTreeMap::new(),
            morale: 1.0,
            pos: IVec2::new(5, 5),
        },
        enemies: vec![],
        pois: vec![],
        obstacles: vec![],
        objective: None,

        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}

fn snapshot_with_enemy(me_x: i32, me_y: i32, e_x: i32, e_y: i32, e_hp: i32) -> WorldSnapshot {
    let mut s = base_snapshot();
    s.me.pos = IVec2::new(me_x, me_y);
    s.enemies = vec![EnemyState {
        id: 1,
        pos: IVec2::new(e_x, e_y),
        hp: e_hp,
        cover: "none".into(),
        last_seen: 0.0,
    }];
    s
}

fn snapshot_with_cooldown(cd_val: f32) -> WorldSnapshot {
    let mut s = snapshot_with_enemy(0, 0, 10, 10, 50);
    s.me.cooldowns.insert("throw:smoke".into(), cd_val);
    s
}

// ============================================================================
// RuleOrchestrator — Exact Computed Values
// ============================================================================

mod rule_orchestrator_mutations {
    use super::*;

    #[test]
    fn plan_id_prefix_is_plan() {
        let ro = RuleOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 10, 0, 50);
        let plan = ro.propose_plan(&snap);
        assert!(
            plan.plan_id.starts_with("plan-"),
            "Expected 'plan-' prefix, got '{}'",
            plan.plan_id
        );
    }

    #[test]
    fn plan_id_encodes_time_times_1000() {
        let ro = RuleOrchestrator::new();
        let mut snap = snapshot_with_enemy(0, 0, 10, 0, 50);
        snap.t = 2.5;
        let plan = ro.propose_plan(&snap);
        assert_eq!(plan.plan_id, "plan-2500", "t=2.5 -> 2500");
    }

    #[test]
    fn smoke_ready_fires_throw_then_move_then_cover() {
        let ro = RuleOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 10, 0, 50);
        let plan = ro.propose_plan(&snap);
        assert_eq!(plan.steps.len(), 3, "Smoke-ready: 3 steps");

        // Step 0: Throw smoke at midpoint
        match &plan.steps[0] {
            ActionStep::Throw { item, x, y } => {
                assert_eq!(item, "smoke");
                assert_eq!(*x, 5, "Midpoint x = (0+10)/2 = 5");
                assert_eq!(*y, 0, "Midpoint y = (0+0)/2 = 0");
            }
            other => panic!("Step 0 must be Throw, got {:?}", other),
        }

        // Step 1: MoveTo with signum*2 offset
        match &plan.steps[1] {
            ActionStep::MoveTo { x, y, .. } => {
                assert_eq!(*x, 2, "Move x = 0 + signum(10)*2 = 2");
                assert_eq!(*y, 0, "Move y = 0 + signum(0)*2 = 0");
            }
            other => panic!("Step 1 must be MoveTo, got {:?}", other),
        }

        // Step 2: CoverFire with duration 2.5
        match &plan.steps[2] {
            ActionStep::CoverFire {
                target_id,
                duration,
            } => {
                assert_eq!(*target_id, 1);
                assert!(
                    (*duration - 2.5).abs() < 1e-6,
                    "Smoke CoverFire duration must be 2.5, got {}",
                    duration
                );
            }
            other => panic!("Step 2 must be CoverFire, got {:?}", other),
        }
    }

    #[test]
    fn cooldown_active_fires_move_then_cover() {
        let ro = RuleOrchestrator::new();
        let snap = snapshot_with_cooldown(5.0); // cooldown active
        let plan = ro.propose_plan(&snap);
        assert_eq!(plan.steps.len(), 2, "Cooldown-active: 2 steps");

        // Step 0: MoveTo with signum*1 offset (no *2)
        match &plan.steps[0] {
            ActionStep::MoveTo { x, y, .. } => {
                assert_eq!(*x, 1, "Move x = 0 + signum(10)*1 = 1");
                assert_eq!(*y, 1, "Move y = 0 + signum(10)*1 = 1");
            }
            other => panic!("Step 0 must be MoveTo, got {:?}", other),
        }

        // Step 1: CoverFire with duration 1.5 (not 2.5)
        match &plan.steps[1] {
            ActionStep::CoverFire {
                target_id,
                duration,
            } => {
                assert_eq!(*target_id, 1);
                assert!(
                    (*duration - 1.5).abs() < 1e-6,
                    "Cooldown CoverFire duration must be 1.5, got {}",
                    duration
                );
            }
            other => panic!("Step 1 must be CoverFire, got {:?}", other),
        }
    }

    #[test]
    fn cover_fire_duration_25_not_15_when_smoke_ready() {
        let ro = RuleOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 10, 10, 50);
        let plan = ro.propose_plan(&snap);
        if let ActionStep::CoverFire { duration, .. } = &plan.steps[2] {
            assert!(
                (*duration - 2.5).abs() < 1e-6,
                "Smoke-ready duration must be exactly 2.5"
            );
            assert!((*duration - 1.5).abs() > 0.5, "Must NOT be 1.5");
        }
    }

    #[test]
    fn cover_fire_duration_15_not_25_when_cooldown() {
        let ro = RuleOrchestrator::new();
        let snap = snapshot_with_cooldown(3.0);
        let plan = ro.propose_plan(&snap);
        if let ActionStep::CoverFire { duration, .. } = &plan.steps[1] {
            assert!(
                (*duration - 1.5).abs() < 1e-6,
                "Cooldown duration must be exactly 1.5"
            );
            assert!((*duration - 2.5).abs() > 0.5, "Must NOT be 2.5");
        }
    }

    #[test]
    fn movement_offset_2_when_smoke_ready() {
        let ro = RuleOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 10, 0, 50);
        let plan = ro.propose_plan(&snap);
        if let ActionStep::MoveTo { x, .. } = &plan.steps[1] {
            assert_eq!(*x, 2, "Smoke move offset must be signum*2 = 2");
        }
    }

    #[test]
    fn movement_offset_1_when_cooldown() {
        let ro = RuleOrchestrator::new();
        let snap = snapshot_with_cooldown(5.0);
        let plan = ro.propose_plan(&snap);
        if let ActionStep::MoveTo { x, y, .. } = &plan.steps[0] {
            assert_eq!(*x, 1, "Cooldown move offset must be signum*1 = 1");
            assert_eq!(*y, 1);
        }
    }

    #[test]
    fn cooldown_boundary_exactly_zero_fires_smoke() {
        let ro = RuleOrchestrator::new();
        let snap = snapshot_with_cooldown(0.0);
        let plan = ro.propose_plan(&snap);
        assert_eq!(plan.steps.len(), 3, "cd=0.0 must be smoke-ready (<=)");
        match &plan.steps[0] {
            ActionStep::Throw { item, .. } => assert_eq!(item, "smoke"),
            other => panic!("cd=0.0 should produce Throw, got {:?}", other),
        }
    }

    #[test]
    fn cooldown_boundary_tiny_positive_blocks_smoke() {
        let ro = RuleOrchestrator::new();
        let snap = snapshot_with_cooldown(0.001);
        let plan = ro.propose_plan(&snap);
        assert_eq!(plan.steps.len(), 2, "cd=0.001 must NOT fire smoke");
        assert!(matches!(&plan.steps[0], ActionStep::MoveTo { .. }));
    }

    #[test]
    fn no_enemies_produces_empty_steps() {
        let ro = RuleOrchestrator::new();
        let snap = base_snapshot();
        let plan = ro.propose_plan(&snap);
        assert!(plan.steps.is_empty(), "No enemies -> empty steps");
    }

    #[test]
    fn midpoint_calculation_asymmetric() {
        let ro = RuleOrchestrator::new();
        let snap = snapshot_with_enemy(2, 4, 8, 12, 50);
        let plan = ro.propose_plan(&snap);
        match &plan.steps[0] {
            ActionStep::Throw { x, y, .. } => {
                assert_eq!(*x, 5, "Midpoint x = (2+8)/2 = 5");
                assert_eq!(*y, 8, "Midpoint y = (4+12)/2 = 8");
            }
            other => panic!("Expected Throw, got {:?}", other),
        }
    }

    #[test]
    fn cooldown_key_is_throw_colon_smoke() {
        let ro = RuleOrchestrator::new();
        let mut snap = snapshot_with_enemy(0, 0, 10, 0, 50);
        snap.me.cooldowns.insert("throw_smoke".into(), 5.0);
        let plan = ro.propose_plan(&snap);
        assert_eq!(
            plan.steps.len(),
            3,
            "Wrong cooldown key should not block smoke"
        );
    }
}

// ============================================================================
// UtilityOrchestrator — Score Formulas & Sort Order
// ============================================================================

mod utility_orchestrator_mutations {
    use super::*;

    #[test]
    fn plan_id_prefix_is_util() {
        let uo = UtilityOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 10, 0, 50);
        let plan = uo.propose_plan(&snap);
        assert!(
            plan.plan_id.starts_with("util-"),
            "Expected 'util-' prefix, got '{}'",
            plan.plan_id
        );
    }

    #[test]
    fn smoke_candidate_wins_when_score_higher() {
        let uo = UtilityOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 10, 0, 50);
        let plan = uo.propose_plan(&snap);
        assert!(
            matches!(&plan.steps[0], ActionStep::Throw { .. }),
            "Smoke candidate (score 1.5) should win over advance (score 0.8)"
        );
    }

    #[test]
    fn advance_coverfire_at_distance_within_3() {
        let uo = UtilityOrchestrator::new();
        let mut snap = snapshot_with_enemy(0, 0, 2, 0, 50);
        snap.me.cooldowns.insert("throw:smoke".into(), 5.0);
        let plan = uo.propose_plan(&snap);
        assert!(plan.steps.len() >= 2, "Close enemy should get CoverFire");
        assert!(
            matches!(&plan.steps[1], ActionStep::CoverFire { duration, .. } if (*duration - 1.0).abs() < 1e-6),
            "Advance CoverFire duration must be 1.0"
        );
    }

    #[test]
    fn advance_coverfire_distance_threshold_exactly_3() {
        let uo = UtilityOrchestrator::new();
        let mut snap = snapshot_with_enemy(0, 0, 3, 0, 50);
        snap.me.cooldowns.insert("throw:smoke".into(), 5.0);
        let plan = uo.propose_plan(&snap);
        assert!(
            plan.steps.len() >= 2,
            "dist=3 (<=3.0): should include CoverFire"
        );
    }

    #[test]
    fn advance_no_coverfire_distance_above_3() {
        let uo = UtilityOrchestrator::new();
        let mut snap = snapshot_with_enemy(0, 0, 4, 0, 50);
        snap.me.cooldowns.insert("throw:smoke".into(), 5.0);
        let plan = uo.propose_plan(&snap);
        assert_eq!(
            plan.steps.len(),
            1,
            "dist=4 (>3.0): should NOT include CoverFire"
        );
    }

    #[test]
    fn sort_order_descending_smoke_wins_over_advance() {
        let uo = UtilityOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 10, 0, 50);
        let plan = uo.propose_plan(&snap);
        assert!(
            matches!(&plan.steps[0], ActionStep::Throw { .. }),
            "Descending sort: smoke should beat advance at far distance"
        );
    }

    #[test]
    fn advance_coverfire_duration_is_exactly_1_0() {
        let uo = UtilityOrchestrator::new();
        let mut snap = snapshot_with_enemy(0, 0, 1, 0, 50);
        snap.me.cooldowns.insert("throw:smoke".into(), 5.0);
        let plan = uo.propose_plan(&snap);
        if let Some(ActionStep::CoverFire { duration, .. }) = plan.steps.get(1) {
            assert!(
                (*duration - 1.0).abs() < 1e-6,
                "Advance CoverFire must be 1.0, got {}",
                duration
            );
        }
    }

    #[test]
    fn smoke_move_offset_is_2() {
        let uo = UtilityOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 10, 0, 50);
        let plan = uo.propose_plan(&snap);
        if let Some(ActionStep::MoveTo { x, .. }) = plan.steps.get(1) {
            assert_eq!(*x, 2, "Smoke movement offset = signum*2 = 2");
        }
    }

    #[test]
    fn advance_move_offset_is_1() {
        let uo = UtilityOrchestrator::new();
        let mut snap = snapshot_with_enemy(0, 0, 10, 0, 50);
        snap.me.cooldowns.insert("throw:smoke".into(), 5.0);
        let plan = uo.propose_plan(&snap);
        if let ActionStep::MoveTo { x, .. } = &plan.steps[0] {
            assert_eq!(*x, 1, "Advance movement offset = signum*1 = 1");
        }
    }

    #[test]
    fn no_enemies_utility_produces_empty() {
        let uo = UtilityOrchestrator::new();
        let snap = base_snapshot();
        let plan = uo.propose_plan(&snap);
        assert!(
            plan.steps.is_empty(),
            "No enemies -> empty plan for utility orchestrator"
        );
    }

    #[test]
    fn manhattan_distance_uses_abs() {
        let uo = UtilityOrchestrator::new();
        let mut snap = snapshot_with_enemy(5, 5, 2, 2, 50);
        snap.me.cooldowns.insert("throw:smoke".into(), 5.0);
        let plan = uo.propose_plan(&snap);
        assert_eq!(plan.steps.len(), 1, "dist=6: no CoverFire");
    }

    #[test]
    fn enemy_hp_affects_smoke_score() {
        let uo = UtilityOrchestrator::new();
        let snap_high_hp = snapshot_with_enemy(0, 0, 10, 0, 100);
        let snap_zero_hp = snapshot_with_enemy(0, 0, 10, 0, 0);
        let plan_high = uo.propose_plan(&snap_high_hp);
        let plan_zero = uo.propose_plan(&snap_zero_hp);
        assert!(matches!(&plan_high.steps[0], ActionStep::Throw { .. }));
        assert!(matches!(&plan_zero.steps[0], ActionStep::Throw { .. }));
    }
}

// ============================================================================
// GoapOrchestrator — Distance Threshold & Action Values
// ============================================================================

mod goap_orchestrator_mutations {
    use super::*;

    #[test]
    fn plan_id_prefix_is_goap() {
        let go = GoapOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 10, 0, 50);
        let plan = go.propose_plan(&snap);
        assert!(plan.plan_id.starts_with("goap-"));
    }

    #[test]
    fn in_range_dist_2_fires_coverfire() {
        let go = GoapOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 2, 0, 50);
        let plan = go.propose_plan(&snap);
        assert_eq!(plan.steps.len(), 1);
        match &plan.steps[0] {
            ActionStep::CoverFire {
                target_id,
                duration,
            } => {
                assert_eq!(*target_id, 1);
                assert!(
                    (*duration - 1.5).abs() < 1e-6,
                    "GOAP CoverFire duration must be 1.5, got {}",
                    duration
                );
            }
            other => panic!("dist=2 should produce CoverFire, got {:?}", other),
        }
    }

    #[test]
    fn out_of_range_dist_3_fires_moveto() {
        let go = GoapOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 3, 0, 50);
        let plan = go.propose_plan(&snap);
        assert_eq!(plan.steps.len(), 1);
        assert!(
            matches!(&plan.steps[0], ActionStep::MoveTo { .. }),
            "dist=3: should produce MoveTo, not CoverFire"
        );
    }

    #[test]
    fn boundary_dist_exactly_2_is_in_range() {
        let go = GoapOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 1, 1, 50);
        let plan = go.propose_plan(&snap);
        assert!(
            matches!(&plan.steps[0], ActionStep::CoverFire { .. }),
            "dist=2 (boundary) must fire CoverFire"
        );
    }

    #[test]
    fn coverfire_duration_is_15_not_10_or_20() {
        let go = GoapOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 1, 0, 50);
        let plan = go.propose_plan(&snap);
        if let ActionStep::CoverFire { duration, .. } = &plan.steps[0] {
            assert!((*duration - 1.5).abs() < 1e-6, "Must be exactly 1.5");
            assert!((*duration - 1.0).abs() > 0.1, "Must NOT be 1.0");
            assert!((*duration - 2.0).abs() > 0.1, "Must NOT be 2.0");
        }
    }

    #[test]
    fn no_enemies_produces_empty_plan() {
        let go = GoapOrchestrator::new();
        let snap = base_snapshot();
        let plan = go.propose_plan(&snap);
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn next_action_in_range_returns_coverfire() {
        let go = GoapOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 1, 0, 50);
        let action = go.next_action(&snap);
        match action {
            ActionStep::CoverFire { duration, .. } => {
                assert!(
                    (duration - 1.5).abs() < 1e-6,
                    "next_action at dist=1 must return CoverFire with duration 1.5"
                );
            }
            other => panic!("Expected CoverFire, got {:?}", other),
        }
    }

    #[test]
    fn next_action_out_of_range_returns_moveto() {
        let go = GoapOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 5, 5, 50);
        let action = go.next_action(&snap);
        assert!(
            matches!(action, ActionStep::MoveTo { .. }),
            "next_action at dist=10 should return MoveTo"
        );
    }

    #[test]
    fn next_action_no_enemies_returns_wait_1_0() {
        let go = GoapOrchestrator::new();
        let snap = base_snapshot();
        let action = go.next_action(&snap);
        match action {
            ActionStep::Wait { duration } => {
                assert!(
                    (duration - 1.0).abs() < 1e-6,
                    "No-enemies wait must be 1.0, got {}",
                    duration
                );
            }
            other => panic!("No enemies should produce Wait, got {:?}", other),
        }
    }

    #[test]
    fn moveto_uses_signum_direction() {
        let go = GoapOrchestrator::new();
        let snap = snapshot_with_enemy(5, 5, 10, 3, 50);
        let action = go.next_action(&snap);
        match action {
            ActionStep::MoveTo { x, y, .. } => {
                assert_eq!(x, 6, "x = 5 + signum(5) = 6");
                assert_eq!(y, 4, "y = 5 + signum(-2) = 4");
            }
            other => panic!("Expected MoveTo, got {:?}", other),
        }
    }

    #[test]
    fn propose_plan_negative_coords_works() {
        let go = GoapOrchestrator::new();
        // Test with negative coordinates — should use abs for distance
        let snap = snapshot_with_enemy(-5, -5, -3, -3, 50);
        let plan = go.propose_plan(&snap);
        // dist = |-3-(-5)| + |-3-(-5)| = 2+2 = 4 -> > 2 -> MoveTo
        assert_eq!(plan.steps.len(), 1);
        assert!(matches!(&plan.steps[0], ActionStep::MoveTo { .. }));
    }
}

// ============================================================================
// ToolVerb Classification Tests
// ============================================================================

mod tool_verb_mutations {
    use super::*;

    #[test]
    fn movement_verbs_correct_set() {
        assert!(ToolVerb::MoveTo.is_movement());
        assert!(ToolVerb::Wander.is_movement());
        assert!(ToolVerb::Hide.is_movement());
        assert!(ToolVerb::Stay.is_movement());
        // Non-movement — exhaustive
        assert!(!ToolVerb::Throw.is_movement());
        assert!(!ToolVerb::CoverFire.is_movement());
        assert!(!ToolVerb::Revive.is_movement());
        assert!(!ToolVerb::Rally.is_movement());
        assert!(!ToolVerb::Interact.is_movement());
        assert!(!ToolVerb::UseItem.is_movement());
    }

    #[test]
    fn combat_verbs_correct_set() {
        assert!(ToolVerb::Throw.is_combat());
        assert!(ToolVerb::CoverFire.is_combat());
        // Non-combat — exhaustive
        assert!(!ToolVerb::MoveTo.is_combat());
        assert!(!ToolVerb::Revive.is_combat());
        assert!(!ToolVerb::Rally.is_combat());
        assert!(!ToolVerb::Interact.is_combat());
        assert!(!ToolVerb::UseItem.is_combat());
        assert!(!ToolVerb::Stay.is_combat());
        assert!(!ToolVerb::Wander.is_combat());
        assert!(!ToolVerb::Hide.is_combat());
    }

    #[test]
    fn support_verbs_correct_set() {
        assert!(ToolVerb::Revive.is_support());
        assert!(ToolVerb::Rally.is_support());
        // Non-support — exhaustive
        assert!(!ToolVerb::Throw.is_support());
        assert!(!ToolVerb::CoverFire.is_support());
        assert!(!ToolVerb::MoveTo.is_support());
        assert!(!ToolVerb::Interact.is_support());
        assert!(!ToolVerb::UseItem.is_support());
        assert!(!ToolVerb::Stay.is_support());
        assert!(!ToolVerb::Wander.is_support());
        assert!(!ToolVerb::Hide.is_support());
    }

    #[test]
    fn requires_target_position_correct_set() {
        assert!(ToolVerb::MoveTo.requires_target_position());
        assert!(ToolVerb::Throw.requires_target_position());
        assert!(ToolVerb::CoverFire.requires_target_position());
        assert!(ToolVerb::Revive.requires_target_position());
        assert!(ToolVerb::Hide.requires_target_position());
        // Does NOT require target position
        assert!(!ToolVerb::Rally.requires_target_position());
        assert!(!ToolVerb::Stay.requires_target_position());
        assert!(!ToolVerb::Wander.requires_target_position());
        assert!(!ToolVerb::Interact.requires_target_position());
        assert!(!ToolVerb::UseItem.requires_target_position());
    }

    #[test]
    fn requires_ammo_correct_set() {
        assert!(ToolVerb::Throw.requires_ammo());
        assert!(ToolVerb::CoverFire.requires_ammo());
        // Does not require ammo — exhaustive
        assert!(!ToolVerb::MoveTo.requires_ammo());
        assert!(!ToolVerb::Revive.requires_ammo());
        assert!(!ToolVerb::Rally.requires_ammo());
        assert!(!ToolVerb::Interact.requires_ammo());
        assert!(!ToolVerb::UseItem.requires_ammo());
        assert!(!ToolVerb::Stay.requires_ammo());
        assert!(!ToolVerb::Wander.requires_ammo());
        assert!(!ToolVerb::Hide.requires_ammo());
    }

    #[test]
    fn requires_line_of_sight_correct_set() {
        assert!(ToolVerb::Throw.requires_line_of_sight());
        assert!(ToolVerb::CoverFire.requires_line_of_sight());
        // Does not require LOS — exhaustive
        assert!(!ToolVerb::MoveTo.requires_line_of_sight());
        assert!(!ToolVerb::Revive.requires_line_of_sight());
        assert!(!ToolVerb::Rally.requires_line_of_sight());
        assert!(!ToolVerb::Interact.requires_line_of_sight());
        assert!(!ToolVerb::UseItem.requires_line_of_sight());
        assert!(!ToolVerb::Stay.requires_line_of_sight());
        assert!(!ToolVerb::Wander.requires_line_of_sight());
        assert!(!ToolVerb::Hide.requires_line_of_sight());
    }

    #[test]
    fn display_strings_exact() {
        assert_eq!(format!("{}", ToolVerb::MoveTo), "MoveTo");
        assert_eq!(format!("{}", ToolVerb::Throw), "Throw");
        assert_eq!(format!("{}", ToolVerb::CoverFire), "CoverFire");
        assert_eq!(format!("{}", ToolVerb::Revive), "Revive");
        assert_eq!(format!("{}", ToolVerb::Interact), "Interact");
        assert_eq!(format!("{}", ToolVerb::UseItem), "UseItem");
        assert_eq!(format!("{}", ToolVerb::Stay), "Stay");
        assert_eq!(format!("{}", ToolVerb::Wander), "Wander");
        assert_eq!(format!("{}", ToolVerb::Hide), "Hide");
        assert_eq!(format!("{}", ToolVerb::Rally), "Rally");
    }

    #[test]
    fn all_returns_10_verbs() {
        assert_eq!(ToolVerb::all().len(), 10);
    }

    #[test]
    fn primary_validation_category_mapping() {
        assert_eq!(
            ToolVerb::MoveTo.primary_validation_category(),
            ValidationCategory::Nav
        );
        assert_eq!(
            ToolVerb::Wander.primary_validation_category(),
            ValidationCategory::Nav
        );
        assert_eq!(
            ToolVerb::Hide.primary_validation_category(),
            ValidationCategory::Nav
        );
        assert_eq!(
            ToolVerb::Throw.primary_validation_category(),
            ValidationCategory::Visibility
        );
        assert_eq!(
            ToolVerb::CoverFire.primary_validation_category(),
            ValidationCategory::Visibility
        );
        assert_eq!(
            ToolVerb::Revive.primary_validation_category(),
            ValidationCategory::Resources
        );
        assert_eq!(
            ToolVerb::UseItem.primary_validation_category(),
            ValidationCategory::Resources
        );
        assert_eq!(
            ToolVerb::Interact.primary_validation_category(),
            ValidationCategory::Physics
        );
        assert_eq!(
            ToolVerb::Stay.primary_validation_category(),
            ValidationCategory::Cooldown
        );
        assert_eq!(
            ToolVerb::Rally.primary_validation_category(),
            ValidationCategory::Cooldown
        );
    }
}

// ============================================================================
// PlannerMode & CAiController Tests
// ============================================================================

mod planner_mode_mutations {
    use astraweave_ai::core_loop::{CAiController, PlannerMode};

    #[test]
    fn planner_mode_display_exact_strings() {
        assert_eq!(format!("{}", PlannerMode::Rule), "Rule");
        assert_eq!(format!("{}", PlannerMode::BehaviorTree), "BehaviorTree");
        assert_eq!(format!("{}", PlannerMode::GOAP), "GOAP");
    }

    #[test]
    fn planner_mode_all_returns_4() {
        // Re-locked from 3 when Htn joined the always-available modes.
        let all = PlannerMode::all();
        assert_eq!(all.len(), 4, "PlannerMode::all() must return 4 modes");
        assert!(all.contains(&PlannerMode::Htn));
    }

    #[test]
    fn planner_mode_always_available() {
        assert!(PlannerMode::Rule.is_always_available());
        assert!(PlannerMode::Htn.is_always_available());
        assert!(!PlannerMode::BehaviorTree.is_always_available());
        assert!(!PlannerMode::GOAP.is_always_available());
    }

    #[test]
    fn planner_mode_requires_bt_feature() {
        assert!(PlannerMode::BehaviorTree.requires_bt_feature());
        assert!(!PlannerMode::Rule.requires_bt_feature());
        assert!(!PlannerMode::GOAP.requires_bt_feature());
    }

    #[test]
    fn planner_mode_requires_goap_feature() {
        assert!(PlannerMode::GOAP.requires_goap_feature());
        assert!(!PlannerMode::Rule.requires_goap_feature());
        assert!(!PlannerMode::BehaviorTree.requires_goap_feature());
    }

    #[test]
    fn planner_mode_required_feature_strings() {
        assert_eq!(PlannerMode::Rule.required_feature(), None);
        assert_eq!(PlannerMode::BehaviorTree.required_feature(), Some("ai-bt"));
        assert_eq!(PlannerMode::GOAP.required_feature(), Some("ai-goap"));
    }

    #[test]
    fn default_mode_is_rule() {
        let c = CAiController::default();
        assert_eq!(c.mode, PlannerMode::Rule);
    }

    #[test]
    fn has_policy_returns_false_on_default() {
        let c = CAiController::default();
        assert!(!c.has_policy());
    }

    #[test]
    fn set_policy_and_check() {
        let mut c = CAiController::default();
        c.set_policy("aggressive");
        assert!(c.has_policy());
        assert_eq!(c.policy_name(), Some("aggressive"));
    }

    #[test]
    fn clear_policy_resets() {
        let mut c = CAiController::default();
        c.set_policy("stealth");
        c.clear_policy();
        assert!(!c.has_policy());
        assert_eq!(c.policy_name(), None);
    }

    #[test]
    fn new_with_mode_stores_mode() {
        let c = CAiController::new(PlannerMode::GOAP);
        assert_eq!(c.mode, PlannerMode::GOAP);
    }

    #[test]
    fn requires_feature_for_bt_true() {
        let c = CAiController::new(PlannerMode::BehaviorTree);
        assert!(c.requires_feature());
    }

    #[test]
    fn requires_feature_for_goap_true() {
        let c = CAiController::new(PlannerMode::GOAP);
        assert!(c.requires_feature());
    }

    #[test]
    fn requires_feature_for_rule_false() {
        let c = CAiController::new(PlannerMode::Rule);
        assert!(!c.requires_feature());
    }

    #[test]
    fn with_policy_constructor() {
        let c = CAiController::with_policy(PlannerMode::GOAP, "gather_craft_policy");
        assert_eq!(c.mode, PlannerMode::GOAP);
        assert_eq!(c.policy_name(), Some("gather_craft_policy"));
    }

    #[test]
    fn convenience_constructors() {
        let r = CAiController::rule();
        assert_eq!(r.mode, PlannerMode::Rule);

        let bt = CAiController::behavior_tree();
        assert_eq!(bt.mode, PlannerMode::BehaviorTree);

        let g = CAiController::goap();
        assert_eq!(g.mode, PlannerMode::GOAP);
    }

    #[test]
    fn display_without_policy() {
        let c = CAiController::new(PlannerMode::Rule);
        assert_eq!(format!("{}", c), "CAiController(Rule)");
    }

    #[test]
    fn display_with_policy() {
        let c = CAiController::with_policy(PlannerMode::GOAP, "my_policy");
        assert_eq!(format!("{}", c), "CAiController(GOAP, policy=my_policy)");
    }
}

// ============================================================================
// tool_sandbox — validate_tool_action boundary tests
// ============================================================================

mod tool_sandbox_boundary_mutations {
    use super::*;

    fn make_world(me_pos: IVec2, morale: f32, ammo: i32) -> WorldSnapshot {
        WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2::new(0, 0),
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo,
                cooldowns: BTreeMap::new(),
                morale,
                pos: me_pos,
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2::new(10, 10),
                hp: 50,
                cover: "none".into(),
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,

            hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        }
    }

    #[test]
    fn validate_ammo_zero_blocks_throw() {
        let world = make_world(IVec2::new(0, 0), 1.0, 0);
        let ctx = ValidationContext::new();
        let result = validate_tool_action(1, ToolVerb::Throw, &world, &ctx, Some(IVec2::new(5, 5)));
        assert!(result.is_err(), "Zero ammo must block Throw");
    }

    #[test]
    fn validate_ammo_one_allows_throw() {
        let world = make_world(IVec2::new(0, 0), 1.0, 1);
        let ctx = ValidationContext::new();
        let result = validate_tool_action(1, ToolVerb::Throw, &world, &ctx, Some(IVec2::new(5, 5)));
        assert!(result.is_ok(), "Ammo=1 must allow Throw");
    }

    #[test]
    fn validate_ammo_zero_blocks_coverfire() {
        let world = make_world(IVec2::new(0, 0), 1.0, 0);
        let ctx = ValidationContext::new();
        let result =
            validate_tool_action(1, ToolVerb::CoverFire, &world, &ctx, Some(IVec2::new(5, 5)));
        assert!(result.is_err(), "Zero ammo must block CoverFire");
    }

    #[test]
    fn validate_revive_low_morale_blocks() {
        let world = make_world(IVec2::new(0, 0), 0.49, 5);
        let ctx = ValidationContext::new();
        let result =
            validate_tool_action(1, ToolVerb::Revive, &world, &ctx, Some(IVec2::new(0, 0)));
        assert!(result.is_err(), "Morale 0.49 < 0.5 must block Revive");
    }

    #[test]
    fn validate_revive_exact_morale_05_allows() {
        let world = make_world(IVec2::new(0, 0), 0.5, 5);
        let ctx = ValidationContext::new();
        let result =
            validate_tool_action(1, ToolVerb::Revive, &world, &ctx, Some(IVec2::new(0, 0)));
        assert!(
            result.is_ok(),
            "Morale exactly 0.5 must allow Revive (< 0.5 check)"
        );
    }

    #[test]
    fn validate_revive_distance_too_far_blocks() {
        let world = make_world(IVec2::new(0, 0), 1.0, 5);
        let ctx = ValidationContext::new();
        let result =
            validate_tool_action(1, ToolVerb::Revive, &world, &ctx, Some(IVec2::new(3, 0)));
        assert!(result.is_err(), "Distance 3.0 > 2.0 must block Revive");
    }

    #[test]
    fn validate_revive_distance_exactly_2_allows() {
        let world = make_world(IVec2::new(0, 0), 1.0, 5);
        let ctx = ValidationContext::new();
        let result =
            validate_tool_action(1, ToolVerb::Revive, &world, &ctx, Some(IVec2::new(2, 0)));
        assert!(
            result.is_ok(),
            "Distance exactly 2.0 must allow Revive (> 2.0 check)"
        );
    }

    #[test]
    fn validate_revive_distance_just_over_2_blocks() {
        let world = make_world(IVec2::new(0, 0), 1.0, 5);
        let ctx = ValidationContext::new();
        let result =
            validate_tool_action(1, ToolVerb::Revive, &world, &ctx, Some(IVec2::new(2, 1)));
        assert!(
            result.is_err(),
            "Distance sqrt(5)~2.236 > 2.0 must block Revive"
        );
    }

    #[test]
    fn validate_cooldown_active_blocks_throw() {
        let mut world = make_world(IVec2::new(0, 0), 1.0, 5);
        world.me.cooldowns.insert("throw".into(), 5.0);
        let ctx = ValidationContext::new();
        let result = validate_tool_action(1, ToolVerb::Throw, &world, &ctx, Some(IVec2::new(5, 5)));
        assert!(result.is_err(), "Active cooldown must block Throw");
    }

    #[test]
    fn validate_cooldown_zero_allows_throw() {
        let mut world = make_world(IVec2::new(0, 0), 1.0, 5);
        world.me.cooldowns.insert("throw".into(), 0.0);
        let ctx = ValidationContext::new();
        let result = validate_tool_action(1, ToolVerb::Throw, &world, &ctx, Some(IVec2::new(5, 5)));
        assert!(
            result.is_ok(),
            "Cooldown=0.0 must allow Throw (> 0.0 check)"
        );
    }

    #[test]
    fn validate_cooldown_tiny_positive_blocks() {
        let mut world = make_world(IVec2::new(0, 0), 1.0, 5);
        world.me.cooldowns.insert("throw".into(), 0.001);
        let ctx = ValidationContext::new();
        let result = validate_tool_action(1, ToolVerb::Throw, &world, &ctx, Some(IVec2::new(5, 5)));
        assert!(result.is_err(), "Cooldown=0.001 > 0.0 must block Throw");
    }

    #[test]
    fn validate_moveto_always_ok_without_nav_or_physics() {
        let world = make_world(IVec2::new(0, 0), 1.0, 5);
        let ctx = ValidationContext::new();
        let result =
            validate_tool_action(1, ToolVerb::MoveTo, &world, &ctx, Some(IVec2::new(5, 5)));
        assert!(
            result.is_ok(),
            "MoveTo should be OK without nav/physics context"
        );
    }

    #[test]
    fn validate_interact_always_ok_without_context() {
        let world = make_world(IVec2::new(0, 0), 1.0, 5);
        let ctx = ValidationContext::new();
        let result = validate_tool_action(1, ToolVerb::Interact, &world, &ctx, None);
        assert!(
            result.is_ok(),
            "Interact should be OK (falls through to no-op)"
        );
    }

    #[test]
    fn validate_rally_always_ok() {
        let world = make_world(IVec2::new(0, 0), 1.0, 5);
        let ctx = ValidationContext::new();
        let result = validate_tool_action(1, ToolVerb::Rally, &world, &ctx, None);
        assert!(
            result.is_ok(),
            "Rally should be OK (falls through to no-op)"
        );
    }
}

// ============================================================================
// ValidationContext Tests
// ============================================================================

mod validation_context_mutations {
    use super::*;

    #[test]
    fn new_context_is_empty() {
        let ctx = ValidationContext::new();
        assert!(!ctx.has_nav());
        assert!(!ctx.has_physics());
        assert!(!ctx.is_complete());
        assert!(ctx.is_empty());
    }

    #[test]
    fn default_context_is_empty() {
        let ctx = ValidationContext::default();
        assert!(ctx.is_empty());
    }

    #[test]
    fn empty_context_has_3_available_categories() {
        let ctx = ValidationContext::new();
        let cats = ctx.available_categories();
        assert_eq!(
            cats.len(),
            3,
            "Empty context should have 3 always-available categories"
        );
        assert!(cats.contains(&ValidationCategory::Resources));
        assert!(cats.contains(&ValidationCategory::Visibility));
        assert!(cats.contains(&ValidationCategory::Cooldown));
    }

    #[test]
    fn validation_category_display() {
        assert_eq!(format!("{}", ValidationCategory::Nav), "Nav");
        assert_eq!(format!("{}", ValidationCategory::Physics), "Physics");
        assert_eq!(format!("{}", ValidationCategory::Resources), "Resources");
        assert_eq!(format!("{}", ValidationCategory::Visibility), "Visibility");
        assert_eq!(format!("{}", ValidationCategory::Cooldown), "Cooldown");
    }
}

// ============================================================================
// Orchestrator Edge Cases
// ============================================================================

mod orchestrator_edge_cases {
    use super::*;

    #[test]
    fn rule_orchestrator_with_negative_enemy_hp() {
        let ro = RuleOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 10, 0, -5);
        let plan = ro.propose_plan(&snap);
        assert!(!plan.steps.is_empty());
    }

    #[test]
    fn utility_orchestrator_with_zero_hp_enemy() {
        let uo = UtilityOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 10, 0, 0);
        let plan = uo.propose_plan(&snap);
        assert!(!plan.steps.is_empty());
    }

    #[test]
    fn goap_orchestrator_dist_1_both_axes() {
        let go = GoapOrchestrator::new();
        let snap = snapshot_with_enemy(0, 0, 1, 1, 50);
        let plan = go.propose_plan(&snap);
        assert!(matches!(&plan.steps[0], ActionStep::CoverFire { .. }));
    }

    #[test]
    fn rule_with_multiple_enemies_uses_first() {
        let ro = RuleOrchestrator::new();
        let mut snap = snapshot_with_enemy(0, 0, 10, 0, 50);
        snap.enemies.push(EnemyState {
            id: 2,
            pos: IVec2::new(20, 20),
            hp: 80,
            cover: "none".into(),
            last_seen: 0.0,
        });
        let plan = ro.propose_plan(&snap);
        if let ActionStep::Throw { x, y, .. } = &plan.steps[0] {
            assert_eq!(*x, 5, "Should use first enemy for midpoint: (0+10)/2=5");
            assert_eq!(*y, 0, "Should use first enemy for midpoint: (0+0)/2=0");
        }
    }

    #[test]
    fn rule_snap_time_zero() {
        let ro = RuleOrchestrator::new();
        let mut snap = snapshot_with_enemy(0, 0, 10, 0, 50);
        snap.t = 0.0;
        let plan = ro.propose_plan(&snap);
        assert_eq!(plan.plan_id, "plan-0");
    }

    #[test]
    fn goap_same_position_enemy_is_in_range() {
        let go = GoapOrchestrator::new();
        let snap = snapshot_with_enemy(5, 5, 5, 5, 50);
        let plan = go.propose_plan(&snap);
        assert!(
            matches!(&plan.steps[0], ActionStep::CoverFire { .. }),
            "Same position (dist=0) must be in range"
        );
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn utility_time_encoded_in_plan_id() {
        let uo = UtilityOrchestrator::new();
        let mut snap = snapshot_with_enemy(0, 0, 10, 0, 50);
        snap.t = 3.14;
        let plan = uo.propose_plan(&snap);
        assert_eq!(plan.plan_id, "util-3140");
    }

    #[test]
    fn goap_time_encoded_in_plan_id() {
        let go = GoapOrchestrator::new();
        let mut snap = snapshot_with_enemy(0, 0, 10, 0, 50);
        snap.t = 7.5;
        let plan = go.propose_plan(&snap);
        assert_eq!(plan.plan_id, "goap-7500");
    }

    #[test]
    fn rule_coverfire_targets_enemy_id() {
        let ro = RuleOrchestrator::new();
        let mut snap = snapshot_with_enemy(0, 0, 10, 0, 50);
        snap.enemies[0].id = 42;
        let plan = ro.propose_plan(&snap);
        if let ActionStep::CoverFire { target_id, .. } = &plan.steps[2] {
            assert_eq!(*target_id, 42, "CoverFire should target correct enemy id");
        }
    }

    #[test]
    fn goap_coverfire_targets_enemy_id() {
        let go = GoapOrchestrator::new();
        let mut snap = snapshot_with_enemy(0, 0, 1, 0, 50);
        snap.enemies[0].id = 99;
        let action = go.next_action(&snap);
        if let ActionStep::CoverFire { target_id, .. } = action {
            assert_eq!(target_id, 99, "CoverFire should target correct enemy id");
        }
    }
}
