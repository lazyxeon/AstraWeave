//! NaN and Infinity validation tests for AI subsystem.
//!
//! P0-Critical: Ensures AI systems handle invalid float inputs gracefully
//! without panicking or corrupting state.

use astraweave_ai::core_loop::{dispatch_planner, CAiController, PlannerMode};
use astraweave_ai::orchestrator::{Orchestrator, RuleOrchestrator};
use astraweave_core::{CompanionState, IVec2, PlayerState, WorldSnapshot};
use std::collections::BTreeMap;
use std::panic;

/// Helper to verify a closure doesn't panic
fn should_not_panic<F: FnOnce() + panic::UnwindSafe>(name: &str, f: F) {
    let result = panic::catch_unwind(f);
    assert!(result.is_ok(), "{} should not panic on invalid input", name);
}

/// Create a valid baseline WorldSnapshot for testing
fn create_baseline_snapshot() -> WorldSnapshot {
    WorldSnapshot {
        t: 0.0,
        me: CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,
            pos: IVec2 { x: 5, y: 5 },
        },
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 5, y: 5 },
            stance: "stand".into(),
            orders: vec![],
        },
        enemies: vec![],
        pois: vec![],
        obstacles: vec![],
        objective: None,

        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}

// ============================================================================
// WorldSnapshot timestamp (t) NaN/Infinity tests
// ============================================================================

#[test]
fn test_dispatch_planner_with_nan_timestamp() {
    should_not_panic("dispatch_planner with NaN timestamp", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.t = f32::NAN;

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_with_infinity_timestamp() {
    should_not_panic("dispatch_planner with Infinity timestamp", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.t = f32::INFINITY;

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_with_neg_infinity_timestamp() {
    should_not_panic("dispatch_planner with -Infinity timestamp", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.t = f32::NEG_INFINITY;

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

// ============================================================================
// CompanionState morale NaN/Infinity tests
// ============================================================================

#[test]
fn test_dispatch_planner_with_nan_morale() {
    should_not_panic("dispatch_planner with NaN morale", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.morale = f32::NAN;

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_with_infinity_morale() {
    should_not_panic("dispatch_planner with Infinity morale", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.morale = f32::INFINITY;

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_with_neg_infinity_morale() {
    should_not_panic("dispatch_planner with -Infinity morale", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.morale = f32::NEG_INFINITY;

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

// ============================================================================
// Cooldowns with NaN/Infinity values
// ============================================================================

#[test]
fn test_dispatch_planner_with_nan_cooldown() {
    should_not_panic("dispatch_planner with NaN cooldown", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.cooldowns.insert("attack".to_string(), f32::NAN);

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_with_infinity_cooldown() {
    should_not_panic("dispatch_planner with Infinity cooldown", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot
            .me
            .cooldowns
            .insert("attack".to_string(), f32::INFINITY);

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_with_neg_infinity_cooldown() {
    should_not_panic("dispatch_planner with -Infinity cooldown", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot
            .me
            .cooldowns
            .insert("attack".to_string(), f32::NEG_INFINITY);

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_with_multiple_nan_cooldowns() {
    should_not_panic("dispatch_planner with multiple NaN cooldowns", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.cooldowns.insert("attack".to_string(), f32::NAN);
        snapshot.me.cooldowns.insert("heal".to_string(), f32::NAN);
        snapshot.me.cooldowns.insert("dodge".to_string(), f32::NAN);

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

// ============================================================================
// RuleOrchestrator direct tests
// ============================================================================

#[test]
fn test_rule_orchestrator_with_nan_morale() {
    should_not_panic("RuleOrchestrator with NaN morale", || {
        let orch = RuleOrchestrator;
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.morale = f32::NAN;

        let _ = orch.propose_plan(&snapshot);
    });
}

#[test]
fn test_rule_orchestrator_with_infinity_morale() {
    should_not_panic("RuleOrchestrator with Infinity morale", || {
        let orch = RuleOrchestrator;
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.morale = f32::INFINITY;

        let _ = orch.propose_plan(&snapshot);
    });
}

#[test]
fn test_rule_orchestrator_with_nan_timestamp() {
    should_not_panic("RuleOrchestrator with NaN timestamp", || {
        let orch = RuleOrchestrator;
        let mut snapshot = create_baseline_snapshot();
        snapshot.t = f32::NAN;

        let _ = orch.propose_plan(&snapshot);
    });
}

#[test]
fn test_rule_orchestrator_with_infinity_timestamp() {
    should_not_panic("RuleOrchestrator with Infinity timestamp", || {
        let orch = RuleOrchestrator;
        let mut snapshot = create_baseline_snapshot();
        snapshot.t = f32::INFINITY;

        let _ = orch.propose_plan(&snapshot);
    });
}

// ============================================================================
// Combined invalid inputs (stress tests)
// ============================================================================

#[test]
fn test_dispatch_planner_all_nan_floats() {
    should_not_panic("dispatch_planner with all NaN floats", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.t = f32::NAN;
        snapshot.me.morale = f32::NAN;
        snapshot.me.cooldowns.insert("attack".to_string(), f32::NAN);
        snapshot.me.cooldowns.insert("heal".to_string(), f32::NAN);

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_all_infinity_floats() {
    should_not_panic("dispatch_planner with all Infinity floats", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.t = f32::INFINITY;
        snapshot.me.morale = f32::INFINITY;
        snapshot
            .me
            .cooldowns
            .insert("attack".to_string(), f32::INFINITY);
        snapshot
            .me
            .cooldowns
            .insert("heal".to_string(), f32::INFINITY);

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_mixed_nan_infinity() {
    should_not_panic("dispatch_planner with mixed NaN and Infinity", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.t = f32::NAN;
        snapshot.me.morale = f32::INFINITY;
        snapshot
            .me
            .cooldowns
            .insert("attack".to_string(), f32::NEG_INFINITY);

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

// ============================================================================
// Edge case float values
// ============================================================================

#[test]
fn test_dispatch_planner_with_zero_morale() {
    should_not_panic("dispatch_planner with zero morale", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.morale = 0.0;

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_with_negative_morale() {
    should_not_panic("dispatch_planner with negative morale", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.morale = -1.0;

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_with_max_float_morale() {
    should_not_panic("dispatch_planner with MAX float morale", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.morale = f32::MAX;

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_with_min_float_morale() {
    should_not_panic("dispatch_planner with MIN float morale", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.morale = f32::MIN;

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_with_subnormal_morale() {
    should_not_panic("dispatch_planner with subnormal morale", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.morale = f32::MIN_POSITIVE / 2.0;

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_with_negative_zero_morale() {
    should_not_panic("dispatch_planner with -0.0 morale", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.morale = -0.0;

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

// ============================================================================
// Timestamp edge cases
// ============================================================================

#[test]
fn test_dispatch_planner_with_negative_timestamp() {
    should_not_panic("dispatch_planner with negative timestamp", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.t = -100.0;

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_with_max_float_timestamp() {
    should_not_panic("dispatch_planner with MAX float timestamp", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.t = f32::MAX;

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_with_min_float_timestamp() {
    should_not_panic("dispatch_planner with MIN float timestamp", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.t = f32::MIN;

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

// ============================================================================
// Cooldown edge cases
// ============================================================================

#[test]
fn test_dispatch_planner_with_zero_cooldown() {
    should_not_panic("dispatch_planner with zero cooldown", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.cooldowns.insert("attack".to_string(), 0.0);

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_with_negative_cooldown() {
    should_not_panic("dispatch_planner with negative cooldown", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.cooldowns.insert("attack".to_string(), -5.0);

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_dispatch_planner_with_max_float_cooldown() {
    should_not_panic("dispatch_planner with MAX float cooldown", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.cooldowns.insert("attack".to_string(), f32::MAX);

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

// ============================================================================
// CAiController stress tests
// ============================================================================

#[test]
fn test_all_planner_modes_with_nan_inputs() {
    should_not_panic("all planner modes with NaN inputs", || {
        let mut snapshot = create_baseline_snapshot();
        snapshot.t = f32::NAN;
        snapshot.me.morale = f32::NAN;

        // Test Rule mode
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let _ = dispatch_planner(&controller, &snapshot);
    });
}

#[test]
fn test_rule_planner_with_extreme_values() {
    should_not_panic("Rule planner with extreme values", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.t = f32::MAX;
        snapshot.me.morale = f32::MIN;
        snapshot.me.cooldowns.insert("attack".to_string(), f32::MAX);
        snapshot.me.cooldowns.insert("heal".to_string(), f32::MIN);

        let _ = dispatch_planner(&controller, &snapshot);
    });
}

// ============================================================================
// Repeated operations with invalid values
// ============================================================================

#[test]
fn test_repeated_dispatch_with_nan() {
    should_not_panic("repeated dispatch with NaN", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };
        let mut snapshot = create_baseline_snapshot();
        snapshot.me.morale = f32::NAN;

        for _ in 0..100 {
            let _ = dispatch_planner(&controller, &snapshot);
        }
    });
}

#[test]
fn test_alternating_valid_invalid_dispatch() {
    should_not_panic("alternating valid/invalid dispatch", || {
        let controller = CAiController {
            mode: PlannerMode::Rule,
            policy: None,
            gather: astraweave_ai::htn::GatherGoal::CookFood,
        };

        for i in 0..50 {
            let mut snapshot = create_baseline_snapshot();
            if i % 2 == 0 {
                snapshot.me.morale = f32::NAN;
                snapshot.t = f32::INFINITY;
            }
            let _ = dispatch_planner(&controller, &snapshot);
        }
    });
}
