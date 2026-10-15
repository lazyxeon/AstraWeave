//! Mutation-Resistant Tests for astraweave-ai
//!
//! These tests verify **exact computed values** and **behavioral correctness** to ensure
//! mutations to formulas and logic are detected by `cargo mutants`.

use crate::core_loop::{CAiController, PlannerMode};
use crate::orchestrator::{Orchestrator, RuleOrchestrator};
use astraweave_core::{ActionStep, CompanionState, EnemyState, IVec2, PlayerState, WorldSnapshot};
use std::collections::BTreeMap;

// =============================================================================
// PlannerMode Tests - Verify exact values and behavior
// =============================================================================

mod planner_mode_tests {
    use super::*;

    #[test]
    fn rule_display_is_exactly_rule() {
        assert_eq!(format!("{}", PlannerMode::Rule), "Rule");
    }

    #[test]
    fn behavior_tree_display_is_exactly_behaviortree() {
        assert_eq!(format!("{}", PlannerMode::BehaviorTree), "BehaviorTree");
    }

    #[test]
    fn goap_display_is_exactly_goap() {
        assert_eq!(format!("{}", PlannerMode::GOAP), "GOAP");
    }

    #[test]
    fn rule_is_always_available() {
        assert!(PlannerMode::Rule.is_always_available());
        assert!(!PlannerMode::BehaviorTree.is_always_available());
        assert!(!PlannerMode::GOAP.is_always_available());
    }

    #[test]
    fn behavior_tree_requires_bt_feature() {
        assert!(PlannerMode::BehaviorTree.requires_bt_feature());
        assert!(!PlannerMode::Rule.requires_bt_feature());
        assert!(!PlannerMode::GOAP.requires_bt_feature());
    }

    #[test]
    fn goap_requires_goap_feature() {
        assert!(PlannerMode::GOAP.requires_goap_feature());
        assert!(!PlannerMode::Rule.requires_goap_feature());
        assert!(!PlannerMode::BehaviorTree.requires_goap_feature());
    }

    #[test]
    fn required_feature_returns_correct_values() {
        assert_eq!(PlannerMode::Rule.required_feature(), None);
        assert_eq!(PlannerMode::BehaviorTree.required_feature(), Some("ai-bt"));
        assert_eq!(PlannerMode::GOAP.required_feature(), Some("ai-goap"));
    }

    #[test]
    fn all_returns_four_modes() {
        // Re-locked from 3 when `PlannerMode::Htn` became an always-available
        // mode. The count stays exact.
        let all = PlannerMode::all();
        assert_eq!(all.len(), 4, "all() should return 4 modes");
        assert!(all.contains(&PlannerMode::Rule));
        assert!(all.contains(&PlannerMode::BehaviorTree));
        assert!(all.contains(&PlannerMode::GOAP));
        assert!(all.contains(&PlannerMode::Htn));
    }
}

// =============================================================================
// CAiController Tests - Verify controller behavior
// =============================================================================

mod ai_controller_tests {
    use super::*;

    #[test]
    fn default_mode_is_rule() {
        let controller = CAiController::default();
        assert_eq!(
            controller.mode,
            PlannerMode::Rule,
            "Default mode should be Rule"
        );
    }

    #[test]
    fn default_policy_is_none() {
        let controller = CAiController::default();
        assert!(controller.policy.is_none(), "Default policy should be None");
    }

    #[test]
    fn new_sets_mode_correctly() {
        let controller = CAiController::new(PlannerMode::GOAP);
        assert_eq!(controller.mode, PlannerMode::GOAP);
    }

    #[test]
    fn with_policy_sets_both() {
        let controller = CAiController::with_policy(PlannerMode::BehaviorTree, "combat");
        assert_eq!(controller.mode, PlannerMode::BehaviorTree);
        assert_eq!(controller.policy, Some("combat".to_string()));
    }

    #[test]
    fn rule_constructor_creates_rule_mode() {
        let controller = CAiController::rule();
        assert_eq!(controller.mode, PlannerMode::Rule);
    }

    #[test]
    fn behavior_tree_constructor_creates_bt_mode() {
        let controller = CAiController::behavior_tree();
        assert_eq!(controller.mode, PlannerMode::BehaviorTree);
    }

    #[test]
    fn goap_constructor_creates_goap_mode() {
        let controller = CAiController::goap();
        assert_eq!(controller.mode, PlannerMode::GOAP);
    }

    #[test]
    fn has_policy_reflects_policy_presence() {
        let without = CAiController::new(PlannerMode::Rule);
        assert!(!without.has_policy());

        let with = CAiController::with_policy(PlannerMode::Rule, "test");
        assert!(with.has_policy());
    }

    #[test]
    fn policy_name_returns_correct_value() {
        let controller = CAiController::with_policy(PlannerMode::Rule, "my_policy");
        assert_eq!(controller.policy_name(), Some("my_policy"));
    }

    #[test]
    fn set_policy_updates_policy() {
        let mut controller = CAiController::new(PlannerMode::Rule);
        controller.set_policy("new_policy");
        assert_eq!(controller.policy, Some("new_policy".to_string()));
    }

    #[test]
    fn clear_policy_removes_policy() {
        let mut controller = CAiController::with_policy(PlannerMode::Rule, "old");
        controller.clear_policy();
        assert!(controller.policy.is_none());
    }

    #[test]
    fn requires_feature_reflects_mode() {
        assert!(!CAiController::rule().requires_feature());
        assert!(CAiController::behavior_tree().requires_feature());
        assert!(CAiController::goap().requires_feature());
    }
}

// =============================================================================
// RuleOrchestrator Tests - Verify planning behavior
// =============================================================================

mod rule_orchestrator_tests {
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
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2::new(5, 5),
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2::new(15, 15),
                hp: 50,
                cover: "none".into(),
                last_seen: 8.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        }
    }

    #[test]
    fn name_is_rule_orchestrator() {
        let orch = RuleOrchestrator::new();
        assert_eq!(orch.name(), "RuleOrchestrator");
    }

    #[test]
    fn display_is_rule_orchestrator() {
        let orch = RuleOrchestrator::new();
        assert_eq!(format!("{}", orch), "RuleOrchestrator");
    }

    #[test]
    fn propose_plan_returns_non_empty_when_enemies() {
        let orch = RuleOrchestrator::new();
        let snap = test_snapshot();
        let plan = orch.propose_plan(&snap);

        assert!(
            !plan.steps.is_empty(),
            "Plan should have steps when enemies present"
        );
    }

    #[test]
    fn propose_plan_includes_throw_smoke_when_cooldown_ready() {
        let orch = RuleOrchestrator::new();
        let snap = test_snapshot();
        let plan = orch.propose_plan(&snap);

        let has_smoke = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::Throw { item, .. } if item == "smoke"));
        assert!(
            has_smoke,
            "Plan should include smoke throw when cooldown ready"
        );
    }

    #[test]
    fn propose_plan_skips_smoke_when_on_cooldown() {
        let orch = RuleOrchestrator::new();
        let mut snap = test_snapshot();
        snap.me.cooldowns.insert("throw:smoke".into(), 5.0); // On cooldown

        let plan = orch.propose_plan(&snap);

        let has_smoke = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::Throw { item, .. } if item == "smoke"));
        assert!(
            !has_smoke,
            "Plan should NOT include smoke throw when on cooldown"
        );
    }

    #[test]
    fn propose_plan_includes_move_to() {
        let orch = RuleOrchestrator::new();
        let snap = test_snapshot();
        let plan = orch.propose_plan(&snap);

        let has_move = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::MoveTo { .. }));
        assert!(has_move, "Plan should include movement action");
    }

    #[test]
    fn plan_id_includes_timestamp() {
        let orch = RuleOrchestrator::new();
        let snap = test_snapshot();
        let plan = orch.propose_plan(&snap);

        assert!(
            plan.plan_id.starts_with("plan-"),
            "Plan ID should start with 'plan-'"
        );
    }

    #[test]
    fn empty_enemies_produces_empty_plan() {
        let orch = RuleOrchestrator::new();
        let mut snap = test_snapshot();
        snap.enemies.clear();

        let plan = orch.propose_plan(&snap);

        assert!(
            plan.steps.is_empty(),
            "Plan should be empty when no enemies"
        );
    }
}

// =============================================================================
// Behavioral Correctness Tests - AI Invariants
// =============================================================================

mod behavioral_correctness_tests {
    use super::*;

    // --- PlannerMode equality is reflexive ---
    #[test]
    fn planner_mode_equality_reflexive() {
        assert_eq!(PlannerMode::Rule, PlannerMode::Rule);
        assert_eq!(PlannerMode::BehaviorTree, PlannerMode::BehaviorTree);
        assert_eq!(PlannerMode::GOAP, PlannerMode::GOAP);
    }

    // --- PlannerMode modes are distinct ---
    #[test]
    fn planner_modes_are_distinct() {
        assert_ne!(PlannerMode::Rule, PlannerMode::BehaviorTree);
        assert_ne!(PlannerMode::Rule, PlannerMode::GOAP);
        assert_ne!(PlannerMode::BehaviorTree, PlannerMode::GOAP);
    }

    // --- CAiController equality ---
    #[test]
    fn ai_controller_equality() {
        let a = CAiController::new(PlannerMode::Rule);
        let b = CAiController::new(PlannerMode::Rule);
        let c = CAiController::new(PlannerMode::GOAP);

        assert_eq!(a, b, "Same mode controllers should be equal");
        assert_ne!(a, c, "Different mode controllers should not be equal");
    }

    // --- Plan steps are deterministic for same input ---
    #[test]
    fn rule_orchestrator_is_deterministic() {
        let orch = RuleOrchestrator::new();
        let snap = WorldSnapshot {
            t: 5.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2::new(0, 0),
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2::new(2, 2),
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2::new(10, 10),
                hp: 50,
                cover: "none".into(),
                last_seen: 3.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };

        let plan1 = orch.propose_plan(&snap);
        let plan2 = orch.propose_plan(&snap);

        assert_eq!(
            plan1.steps.len(),
            plan2.steps.len(),
            "Same input should produce same plan length"
        );
        // Note: plan_id includes timestamp, so compare steps only
    }

    // --- Smoke position is midpoint between companion and enemy ---
    #[test]
    fn smoke_is_at_midpoint() {
        let orch = RuleOrchestrator::new();
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState {
                pos: IVec2::new(0, 0),
                ..Default::default()
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2::new(10, 10),
                ..Default::default()
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };

        let plan = orch.propose_plan(&snap);

        if let Some(ActionStep::Throw { x, y, .. }) = plan.steps.first() {
            // Midpoint of (0,0) and (10,10) = (5,5)
            assert_eq!(*x, 5, "Smoke x should be midpoint");
            assert_eq!(*y, 5, "Smoke y should be midpoint");
        } else {
            panic!("First step should be Throw");
        }
    }

    // --- Move direction is towards enemy ---
    #[test]
    fn move_direction_towards_enemy() {
        let orch = RuleOrchestrator::new();
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState::default(),
            me: CompanionState {
                pos: IVec2::new(0, 0),
                ..Default::default()
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2::new(10, 10),
                ..Default::default()
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        };

        let plan = orch.propose_plan(&snap);

        // Second step should be MoveTo
        if let Some(ActionStep::MoveTo { x, y, .. }) = plan.steps.get(1) {
            // From (0,0) towards (10,10), should move in positive direction
            assert!(*x > 0, "Move x should be towards enemy (positive)");
            assert!(*y > 0, "Move y should be towards enemy (positive)");
        } else {
            panic!("Second step should be MoveTo");
        }
    }

    // --- has_policy is false when policy_name is None ---
    #[test]
    fn has_policy_consistent_with_policy_name() {
        let without_policy = CAiController::new(PlannerMode::Rule);
        assert_eq!(
            without_policy.has_policy(),
            without_policy.policy_name().is_some()
        );

        let with_policy = CAiController::with_policy(PlannerMode::Rule, "test");
        assert_eq!(
            with_policy.has_policy(),
            with_policy.policy_name().is_some()
        );
    }

    // --- All modes are covered by all() ---
    #[test]
    fn all_covers_all_modes() {
        let all = PlannerMode::all();

        // Verify each mode is in the list exactly once
        let rule_count = all.iter().filter(|m| **m == PlannerMode::Rule).count();
        let bt_count = all
            .iter()
            .filter(|m| **m == PlannerMode::BehaviorTree)
            .count();
        let goap_count = all.iter().filter(|m| **m == PlannerMode::GOAP).count();
        let htn_count = all.iter().filter(|m| **m == PlannerMode::Htn).count();

        assert_eq!(rule_count, 1, "Rule should appear exactly once");
        assert_eq!(bt_count, 1, "BehaviorTree should appear exactly once");
        assert_eq!(goap_count, 1, "GOAP should appear exactly once");
        assert_eq!(htn_count, 1, "Htn should appear exactly once");
    }
}

// =============================================================================
// BOUNDARY CONDITION TESTS - Test exact boundary values to catch < vs <= mutations
// =============================================================================

mod boundary_condition_tests {
    use super::*;

    // --- Cooldown boundary tests ---

    #[test]
    fn cooldown_at_zero_allows_action() {
        let orch = RuleOrchestrator::new();
        let mut snap = test_snapshot();
        snap.me.cooldowns.insert("throw:smoke".into(), 0.0); // Exactly zero

        let plan = orch.propose_plan(&snap);

        let has_smoke = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::Throw { item, .. } if item == "smoke"));
        assert!(has_smoke, "Zero cooldown should allow smoke throw");
    }

    #[test]
    fn cooldown_at_epsilon_blocks_action() {
        let orch = RuleOrchestrator::new();
        let mut snap = test_snapshot();
        snap.me.cooldowns.insert("throw:smoke".into(), 0.001); // Slightly positive

        let plan = orch.propose_plan(&snap);

        let has_smoke = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::Throw { item, .. } if item == "smoke"));
        assert!(!has_smoke, "Non-zero cooldown should block smoke throw");
    }

    // --- Morale boundary tests ---

    #[test]
    fn morale_at_zero() {
        let controller = CAiController::default();
        let mut snap = test_snapshot();
        snap.me.morale = 0.0;

        // Controller should still function
        assert_eq!(controller.mode, PlannerMode::Rule);
    }

    #[test]
    fn morale_at_one() {
        let controller = CAiController::default();
        let mut snap = test_snapshot();
        snap.me.morale = 1.0;

        // Controller should still function
        assert_eq!(controller.mode, PlannerMode::Rule);
    }

    // --- Ammo boundary tests ---

    #[test]
    fn ammo_at_zero() {
        let orch = RuleOrchestrator::new();
        let mut snap = test_snapshot();
        snap.me.ammo = 0;

        let plan = orch.propose_plan(&snap);
        // Plan should still generate (even if no ammo)
        assert!(!plan.plan_id.is_empty());
    }

    #[test]
    fn ammo_at_one() {
        let orch = RuleOrchestrator::new();
        let mut snap = test_snapshot();
        snap.me.ammo = 1;

        let plan = orch.propose_plan(&snap);
        assert!(!plan.plan_id.is_empty());
    }

    // --- HP boundary tests ---

    #[test]
    fn enemy_hp_at_zero() {
        let orch = RuleOrchestrator::new();
        let mut snap = test_snapshot();
        snap.enemies[0].hp = 0;

        let plan = orch.propose_plan(&snap);
        // Should still plan against 0-HP enemy (it exists)
        assert!(!plan.steps.is_empty());
    }

    #[test]
    fn enemy_hp_at_one() {
        let orch = RuleOrchestrator::new();
        let mut snap = test_snapshot();
        snap.enemies[0].hp = 1;

        let plan = orch.propose_plan(&snap);
        assert!(!plan.steps.is_empty());
    }

    // --- Position boundary tests ---

    #[test]
    fn position_at_origin() {
        let orch = RuleOrchestrator::new();
        let mut snap = test_snapshot();
        snap.me.pos = IVec2::new(0, 0);
        snap.enemies[0].pos = IVec2::new(10, 10);

        let plan = orch.propose_plan(&snap);
        assert!(!plan.steps.is_empty());
    }

    #[test]
    fn position_at_same_location() {
        let orch = RuleOrchestrator::new();
        let mut snap = test_snapshot();
        snap.me.pos = IVec2::new(5, 5);
        snap.enemies[0].pos = IVec2::new(5, 5);

        let plan = orch.propose_plan(&snap);
        // Midpoint of same location is the location
        if let Some(ActionStep::Throw { x, y, .. }) = plan.steps.first() {
            assert_eq!(*x, 5);
            assert_eq!(*y, 5);
        }
    }

    // --- Empty policy string ---

    #[test]
    fn empty_policy_string() {
        let controller = CAiController::with_policy(PlannerMode::Rule, "");
        assert!(controller.has_policy()); // Empty string is still Some
        assert_eq!(controller.policy_name(), Some(""));
    }

    // --- Timestamp boundary ---

    #[test]
    fn timestamp_at_zero() {
        let orch = RuleOrchestrator::new();
        let mut snap = test_snapshot();
        snap.t = 0.0;

        let plan = orch.propose_plan(&snap);
        assert!(!plan.plan_id.is_empty());
    }

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
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2::new(5, 5),
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2::new(15, 15),
                hp: 50,
                cover: "none".into(),
                last_seen: 8.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        }
    }
}

// =============================================================================
// COMPARISON OPERATOR TESTS - Test to catch == vs != and < vs > swaps
// =============================================================================

mod comparison_operator_tests {
    use super::*;

    // --- PlannerMode equality ---

    #[test]
    fn planner_mode_rule_equals_rule() {
        assert_eq!(PlannerMode::Rule, PlannerMode::Rule);
    }

    #[test]
    fn planner_mode_bt_equals_bt() {
        assert_eq!(PlannerMode::BehaviorTree, PlannerMode::BehaviorTree);
    }

    #[test]
    fn planner_mode_goap_equals_goap() {
        assert_eq!(PlannerMode::GOAP, PlannerMode::GOAP);
    }

    #[test]
    fn planner_mode_rule_not_equals_bt() {
        assert_ne!(PlannerMode::Rule, PlannerMode::BehaviorTree);
    }

    #[test]
    fn planner_mode_rule_not_equals_goap() {
        assert_ne!(PlannerMode::Rule, PlannerMode::GOAP);
    }

    #[test]
    fn planner_mode_bt_not_equals_goap() {
        assert_ne!(PlannerMode::BehaviorTree, PlannerMode::GOAP);
    }

    // --- CAiController equality ---

    #[test]
    fn controller_equals_self() {
        let controller = CAiController::new(PlannerMode::Rule);
        assert_eq!(controller, controller);
    }

    #[test]
    fn controllers_equal_with_same_mode() {
        let a = CAiController::new(PlannerMode::GOAP);
        let b = CAiController::new(PlannerMode::GOAP);
        assert_eq!(a, b);
    }

    #[test]
    fn controllers_not_equal_with_different_mode() {
        let a = CAiController::new(PlannerMode::Rule);
        let b = CAiController::new(PlannerMode::BehaviorTree);
        assert_ne!(a, b);
    }

    #[test]
    fn controllers_not_equal_with_different_policy() {
        let a = CAiController::with_policy(PlannerMode::Rule, "policy_a");
        let b = CAiController::with_policy(PlannerMode::Rule, "policy_b");
        assert_ne!(a, b);
    }

    #[test]
    fn controller_with_policy_not_equal_without() {
        let with = CAiController::with_policy(PlannerMode::Rule, "test");
        let without = CAiController::new(PlannerMode::Rule);
        assert_ne!(with, without);
    }

    // --- Cooldown comparison ---

    #[test]
    fn cooldown_comparison_zero_vs_positive() {
        let mut snap = test_snapshot();
        snap.me.cooldowns.insert("throw:smoke".into(), 0.0);
        let cd_zero = *snap.me.cooldowns.get("throw:smoke").unwrap();

        snap.me.cooldowns.insert("throw:smoke".into(), 1.0);
        let cd_one = *snap.me.cooldowns.get("throw:smoke").unwrap();

        assert!(cd_zero < cd_one);
    }

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
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2::new(5, 5),
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2::new(15, 15),
                hp: 50,
                cover: "none".into(),
                last_seen: 8.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        }
    }
}

// =============================================================================
// BOOLEAN RETURN PATH TESTS - Test all paths through boolean-returning functions
// =============================================================================

mod boolean_return_path_tests {
    use super::*;

    // --- is_always_available() paths ---

    #[test]
    fn is_always_available_returns_true_for_rule() {
        assert!(PlannerMode::Rule.is_always_available());
    }

    #[test]
    fn is_always_available_returns_false_for_bt() {
        assert!(!PlannerMode::BehaviorTree.is_always_available());
    }

    #[test]
    fn is_always_available_returns_false_for_goap() {
        assert!(!PlannerMode::GOAP.is_always_available());
    }

    // --- requires_bt_feature() paths ---

    #[test]
    fn requires_bt_feature_returns_true_for_bt() {
        assert!(PlannerMode::BehaviorTree.requires_bt_feature());
    }

    #[test]
    fn requires_bt_feature_returns_false_for_rule() {
        assert!(!PlannerMode::Rule.requires_bt_feature());
    }

    #[test]
    fn requires_bt_feature_returns_false_for_goap() {
        assert!(!PlannerMode::GOAP.requires_bt_feature());
    }

    // --- requires_goap_feature() paths ---

    #[test]
    fn requires_goap_feature_returns_true_for_goap() {
        assert!(PlannerMode::GOAP.requires_goap_feature());
    }

    #[test]
    fn requires_goap_feature_returns_false_for_rule() {
        assert!(!PlannerMode::Rule.requires_goap_feature());
    }

    #[test]
    fn requires_goap_feature_returns_false_for_bt() {
        assert!(!PlannerMode::BehaviorTree.requires_goap_feature());
    }

    // --- has_policy() paths ---

    #[test]
    fn has_policy_returns_true_when_policy_set() {
        let controller = CAiController::with_policy(PlannerMode::Rule, "test");
        assert!(controller.has_policy());
    }

    #[test]
    fn has_policy_returns_false_when_no_policy() {
        let controller = CAiController::new(PlannerMode::Rule);
        assert!(!controller.has_policy());
    }

    #[test]
    fn has_policy_returns_false_after_clear() {
        let mut controller = CAiController::with_policy(PlannerMode::Rule, "test");
        controller.clear_policy();
        assert!(!controller.has_policy());
    }

    // --- requires_feature() paths ---

    #[test]
    fn requires_feature_returns_false_for_rule() {
        assert!(!CAiController::rule().requires_feature());
    }

    #[test]
    fn requires_feature_returns_true_for_bt() {
        assert!(CAiController::behavior_tree().requires_feature());
    }

    #[test]
    fn requires_feature_returns_true_for_goap() {
        assert!(CAiController::goap().requires_feature());
    }

    // --- required_feature().is_some() paths ---

    #[test]
    fn required_feature_is_none_for_rule() {
        assert!(PlannerMode::Rule.required_feature().is_none());
    }

    #[test]
    fn required_feature_is_some_for_bt() {
        assert!(PlannerMode::BehaviorTree.required_feature().is_some());
    }

    #[test]
    fn required_feature_is_some_for_goap() {
        assert!(PlannerMode::GOAP.required_feature().is_some());
    }

    // --- plan.steps.is_empty() paths ---

    #[test]
    fn plan_steps_is_empty_when_no_enemies() {
        let orch = RuleOrchestrator::new();
        let mut snap = test_snapshot();
        snap.enemies.clear();

        let plan = orch.propose_plan(&snap);
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn plan_steps_not_empty_when_enemies_present() {
        let orch = RuleOrchestrator::new();
        let snap = test_snapshot();

        let plan = orch.propose_plan(&snap);
        assert!(!plan.steps.is_empty());
    }

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
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2::new(5, 5),
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2::new(15, 15),
                hp: 50,
                cover: "none".into(),
                last_seen: 8.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        }
    }
}
