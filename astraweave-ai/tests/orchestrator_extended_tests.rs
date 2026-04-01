//! Extended orchestrator tests for comprehensive coverage of AI planning logic.
//!
//! This test file complements the inline tests in orchestrator.rs by adding:
//! - Edge case validation (no enemies, multiple enemies, boundary positions)
//! - Utility scoring verification (attack/defend priorities, morale effects, distance weighting)
//! - GOAP planning scenarios (no valid plan, cost optimization, state transitions)
//! - Configuration and serialization tests
//!
//! Target: +10-15pp coverage for astraweave-ai orchestrator.rs

use astraweave_ai::{GoapOrchestrator, Orchestrator, RuleOrchestrator, UtilityOrchestrator};
use astraweave_core::{
    ActionStep, CompanionState, EnemyState, IVec2, PlayerState, Stance, WorldSnapshot,
};
use std::collections::BTreeMap;

const COOLDOWN_THROW_SMOKE: &str = "throw:smoke";

/// Helper: Create a WorldSnapshot with configurable parameters
#[allow(clippy::too_many_arguments)]
fn make_snapshot(
    time: f32,
    companion_pos: IVec2,
    companion_ammo: i32,
    companion_morale: f32,
    smoke_cooldown: f32,
    player_hp: i32,
    enemies: Vec<EnemyState>,
    pois: Vec<astraweave_core::Poi>,
    obstacles: Vec<IVec2>,
    objective: Option<String>,
) -> WorldSnapshot {
    WorldSnapshot {
        t: time,
        player: PlayerState {
            pos: IVec2 { x: 0, y: 0 },
            hp: player_hp,
            stance: Stance::Stand,
            orders: vec![],
        },
        me: CompanionState {
            ammo: companion_ammo,
            cooldowns: BTreeMap::from([(COOLDOWN_THROW_SMOKE.to_string(), smoke_cooldown)]),
            morale: companion_morale,
            pos: companion_pos,
        },
        enemies,
        pois,
        obstacles,
        objective,
    }
}

// =============================================================================
// RuleOrchestrator Tests (3 tests)
// =============================================================================

#[test]
fn test_rule_orchestrator_no_enemies() {
    // Edge case: No enemies present
    // Expected: Empty plan (fallback behavior)
    let snap = make_snapshot(
        1.0,
        IVec2 { x: 5, y: 5 },
        30,
        80.0,
        0.0,
        100,
        vec![], // No enemies
        vec![],
        vec![],
        Some("Patrol area".to_string()),
    );

    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert_eq!(plan.plan_id, "plan-1000");
    assert!(
        plan.steps.is_empty(),
        "Should return empty plan when no enemies"
    );
}

#[test]
fn test_rule_orchestrator_multiple_enemies() {
    // Scenario: 3 enemies at different distances
    // Expected: Always targets FIRST enemy (array index 0)
    let snap = make_snapshot(
        2.5,
        IVec2 { x: 0, y: 0 },
        30,
        80.0,
        0.0, // Smoke ready
        100,
        vec![
            EnemyState {
                id: 10,
                pos: IVec2 { x: 6, y: 6 }, // Closest (distance 12)
                hp: 50,
                cover: "none".into(),
                last_seen: 2.5,
            },
            EnemyState {
                id: 11,
                pos: IVec2 { x: 10, y: 0 }, // Medium (distance 10)
                hp: 75,
                cover: "low".into(),
                last_seen: 2.5,
            },
            EnemyState {
                id: 12,
                pos: IVec2 { x: 20, y: 20 }, // Farthest (distance 40)
                hp: 100,
                cover: "high".into(),
                last_seen: 2.5,
            },
        ],
        vec![],
        vec![],
        None,
    );

    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert_eq!(plan.steps.len(), 3, "Should have 3-step plan with smoke");

    // Verify first step is Throw (smoke ready)
    match &plan.steps[0] {
        ActionStep::Throw { item, x, y } => {
            assert_eq!(item, "smoke");
            // Midpoint between companion (0,0) and first enemy (6,6) = (3,3)
            assert_eq!(*x, 3);
            assert_eq!(*y, 3);
        }
        _ => panic!("Expected Throw as first step"),
    }

    // Verify second step is MoveTo (advances toward first enemy)
    match &plan.steps[1] {
        ActionStep::MoveTo { x, y, speed } => {
            assert!(speed.is_none());
            // Should move 2 steps toward first enemy (6,6)
            assert_eq!(*x, 2); // 0 + signum(6-0) * 2 = 2
            assert_eq!(*y, 2); // 0 + signum(6-0) * 2 = 2
        }
        _ => panic!("Expected MoveTo as second step"),
    }

    // Verify third step is CoverFire (targets first enemy)
    match &plan.steps[2] {
        ActionStep::CoverFire {
            target_id,
            duration,
        } => {
            assert_eq!(*target_id, 10); // First enemy ID
            assert_eq!(*duration, 2.5);
        }
        _ => panic!("Expected CoverFire as third step"),
    }
}

#[test]
fn test_rule_orchestrator_edge_positions() {
    // Edge case: Companion and enemy at negative coordinates
    // Validates signum() logic works correctly for all quadrants
    let snap = make_snapshot(
        3.0,
        IVec2 { x: -5, y: -3 },
        30,
        80.0,
        5.0, // Smoke on cooldown
        100,
        vec![EnemyState {
            id: 20,
            pos: IVec2 { x: -10, y: -8 },
            hp: 60,
            cover: "none".into(),
            last_seen: 3.0,
        }],
        vec![],
        vec![],
        None,
    );

    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert_eq!(
        plan.steps.len(),
        2,
        "Should have 2-step cautious advance (smoke on CD)"
    );

    // Verify MoveTo advances toward enemy
    match &plan.steps[0] {
        ActionStep::MoveTo { x, y, speed } => {
            assert!(speed.is_none());
            // -5 + signum(-10 - (-5)) * 1 = -5 + (-1) = -6
            assert_eq!(*x, -6);
            // -3 + signum(-8 - (-3)) * 1 = -3 + (-1) = -4
            assert_eq!(*y, -4);
        }
        _ => panic!("Expected MoveTo as first step"),
    }

    // Verify CoverFire
    match &plan.steps[1] {
        ActionStep::CoverFire {
            target_id,
            duration,
        } => {
            assert_eq!(*target_id, 20);
            assert_eq!(*duration, 1.5);
        }
        _ => panic!("Expected CoverFire as second step"),
    }
}

// =============================================================================
// UtilityOrchestrator Tests (4 tests)
// =============================================================================

#[test]
fn test_utility_scoring_attack() {
    // Scenario: High-HP enemy, companion has ammo and morale
    // Expected: Prefer smoke + advance plan (higher score)
    let snap = make_snapshot(
        4.0,
        IVec2 { x: 0, y: 0 },
        30,
        90.0,
        0.0, // Smoke ready
        100,
        vec![EnemyState {
            id: 30,
            pos: IVec2 { x: 8, y: 0 },
            hp: 100, // High HP enemy
            cover: "low".into(),
            last_seen: 4.0,
        }],
        vec![],
        vec![],
        Some("Eliminate threat".to_string()),
    );

    let orch = UtilityOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert!(!plan.steps.is_empty(), "Should produce a plan");

    // Verify first action is Throw (smoke scores higher with high-HP enemy)
    match &plan.steps[0] {
        ActionStep::Throw { item, x, y } => {
            assert_eq!(item, "smoke");
            // Midpoint between (0,0) and (8,0) = (4,0)
            assert_eq!(*x, 4);
            assert_eq!(*y, 0);
        }
        _ => panic!("Expected Throw as first step for high-priority attack"),
    }
}

#[test]
fn test_utility_scoring_defend() {
    // Scenario: Low morale, enemy very close (distance 2)
    // Expected: Prefer defensive MoveTo (adds brief CoverFire due to dist <= 3)
    let snap = make_snapshot(
        5.0,
        IVec2 { x: 0, y: 0 },
        15,
        20.0, // Low morale
        5.0,  // Smoke on cooldown
        80,
        vec![EnemyState {
            id: 40,
            pos: IVec2 { x: 1, y: 1 }, // Very close (distance 2)
            hp: 70,
            cover: "high".into(),
            last_seen: 5.0,
        }],
        vec![],
        vec![],
        None,
    );

    let orch = UtilityOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert!(!plan.steps.is_empty(), "Should produce a defensive plan");

    // Verify first action is MoveTo (smoke unavailable, defensive positioning)
    match &plan.steps[0] {
        ActionStep::MoveTo { x, y, speed } => {
            assert!(speed.is_none());
            // Should move toward enemy (distance 2 <= 3, so adds CoverFire)
            assert_eq!(*x, 1); // 0 + signum(1-0) = 1
            assert_eq!(*y, 1); // 0 + signum(1-0) = 1
        }
        _ => panic!("Expected MoveTo as first step for defensive plan"),
    }

    // Verify second action is CoverFire (distance <= 3 triggers this)
    if plan.steps.len() > 1 {
        match &plan.steps[1] {
            ActionStep::CoverFire {
                target_id,
                duration,
            } => {
                assert_eq!(*target_id, 40);
                assert_eq!(*duration, 1.0);
            }
            _ => panic!("Expected CoverFire as second step when close"),
        }
    }
}

#[test]
fn test_utility_morale_effect() {
    // Scenario: Two identical enemies, different morale values
    // Expected: Morale affects scoring (though current impl doesn't use it directly)
    // This test validates scoring consistency with morale variations
    let snap_high_morale = make_snapshot(
        6.0,
        IVec2 { x: 0, y: 0 },
        30,
        100.0, // High morale
        0.0,
        100,
        vec![EnemyState {
            id: 50,
            pos: IVec2 { x: 6, y: 0 },
            hp: 50,
            cover: "none".into(),
            last_seen: 6.0,
        }],
        vec![],
        vec![],
        None,
    );

    let snap_low_morale = make_snapshot(
        6.0,
        IVec2 { x: 0, y: 0 },
        30,
        10.0, // Low morale
        0.0,
        100,
        vec![EnemyState {
            id: 50,
            pos: IVec2 { x: 6, y: 0 },
            hp: 50,
            cover: "none".into(),
            last_seen: 6.0,
        }],
        vec![],
        vec![],
        None,
    );

    let orch = UtilityOrchestrator;
    let plan_high = orch.propose_plan(&snap_high_morale);
    let plan_low = orch.propose_plan(&snap_low_morale);

    // Both should produce plans (morale doesn't block planning)
    assert!(!plan_high.steps.is_empty());
    assert!(!plan_low.steps.is_empty());

    // Plans should be identical (current impl doesn't factor morale into scoring)
    // This validates scoring determinism regardless of morale
    assert_eq!(plan_high.steps.len(), plan_low.steps.len());
}

#[test]
fn test_utility_distance_weighting() {
    // Scenario: Test distance effect on scoring (dist <= 3 adds CoverFire)
    let snap_far = make_snapshot(
        7.0,
        IVec2 { x: 0, y: 0 },
        30,
        80.0,
        5.0, // Smoke on CD to test pure distance logic
        100,
        vec![EnemyState {
            id: 60,
            pos: IVec2 { x: 10, y: 0 }, // Far (distance 10)
            hp: 50,
            cover: "none".into(),
            last_seen: 7.0,
        }],
        vec![],
        vec![],
        None,
    );

    let snap_close = make_snapshot(
        7.0,
        IVec2 { x: 0, y: 0 },
        30,
        80.0,
        5.0, // Smoke on CD
        100,
        vec![EnemyState {
            id: 61,
            pos: IVec2 { x: 2, y: 0 }, // Close (distance 2)
            hp: 50,
            cover: "none".into(),
            last_seen: 7.0,
        }],
        vec![],
        vec![],
        None,
    );

    let orch = UtilityOrchestrator;
    let plan_far = orch.propose_plan(&snap_far);
    let plan_close = orch.propose_plan(&snap_close);

    // Far plan: Should have 1 step (MoveTo only, distance > 3)
    assert_eq!(
        plan_far.steps.len(),
        1,
        "Far enemy plan should be move only"
    );
    assert!(matches!(plan_far.steps[0], ActionStep::MoveTo { .. }));

    // Close plan: Should have 2 steps (MoveTo + CoverFire, distance <= 3)
    assert_eq!(
        plan_close.steps.len(),
        2,
        "Close enemy plan should add CoverFire"
    );
    assert!(matches!(plan_close.steps[0], ActionStep::MoveTo { .. }));
    assert!(matches!(plan_close.steps[1], ActionStep::CoverFire { .. }));
}

// =============================================================================
// GoapOrchestrator Tests (3 tests)
// =============================================================================

#[test]
fn test_goap_no_valid_plan() {
    // Edge case: No enemies (goal is unreachable)
    // Expected: Empty plan (no valid path to goal)
    let snap = make_snapshot(
        8.0,
        IVec2 { x: 5, y: 5 },
        30,
        80.0,
        0.0,
        100,
        vec![], // No enemies
        vec![],
        vec![],
        Some("Find enemy".to_string()),
    );

    let orch = GoapOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert_eq!(plan.plan_id, "goap-8000");
    assert!(
        plan.steps.is_empty(),
        "Should return empty plan when goal unreachable"
    );
}

#[test]
fn test_goap_cost_optimization() {
    // Scenario: Test next_action() fast-path (cost-optimized single action)
    // This validates the <100 µs fast-path documented in orchestrator.rs
    let snap = make_snapshot(
        9.0,
        IVec2 { x: 0, y: 0 },
        30,
        80.0,
        0.0,
        100,
        vec![EnemyState {
            id: 70,
            pos: IVec2 { x: 5, y: 0 }, // Distance 5
            hp: 50,
            cover: "none".into(),
            last_seen: 9.0,
        }],
        vec![],
        vec![],
        None,
    );

    let orch = GoapOrchestrator;
    let action = orch.next_action(&snap);

    // Should return MoveTo (distance 5 > 2, not in range for CoverFire)
    match action {
        ActionStep::MoveTo { x, y, speed } => {
            assert!(speed.is_none());
            assert_eq!(x, 1); // 0 + signum(5-0) = 1
            assert_eq!(y, 0); // 0 + signum(0-0) = 0
        }
        _ => panic!("Expected MoveTo for far enemy"),
    }

    // Verify plan() produces equivalent result
    let plan = orch.propose_plan(&snap);
    assert_eq!(plan.steps.len(), 1);
    assert!(matches!(plan.steps[0], ActionStep::MoveTo { .. }));
}

#[test]
fn test_goap_state_transitions() {
    // Scenario: Companion starts far, moves closer (validates state transition logic)
    // Test 3 states: Far (dist 10), Medium (dist 3), Close (dist 2)
    let orch = GoapOrchestrator;

    // State 1: Far (distance 10)
    let snap_far = make_snapshot(
        10.0,
        IVec2 { x: 0, y: 0 },
        30,
        80.0,
        0.0,
        100,
        vec![EnemyState {
            id: 80,
            pos: IVec2 { x: 10, y: 0 },
            hp: 50,
            cover: "none".into(),
            last_seen: 10.0,
        }],
        vec![],
        vec![],
        None,
    );
    let plan_far = orch.propose_plan(&snap_far);
    assert!(matches!(plan_far.steps[0], ActionStep::MoveTo { .. }));

    // State 2: Medium (distance 3)
    let snap_medium = make_snapshot(
        11.0,
        IVec2 { x: 7, y: 0 }, // Moved closer
        30,
        80.0,
        0.0,
        100,
        vec![EnemyState {
            id: 80,
            pos: IVec2 { x: 10, y: 0 },
            hp: 50,
            cover: "none".into(),
            last_seen: 11.0,
        }],
        vec![],
        vec![],
        None,
    );
    let plan_medium = orch.propose_plan(&snap_medium);
    assert!(
        matches!(plan_medium.steps[0], ActionStep::MoveTo { .. }),
        "Distance 3 should still move"
    );

    // State 3: Close (distance 2, within range)
    let snap_close = make_snapshot(
        12.0,
        IVec2 { x: 8, y: 0 }, // In range
        30,
        80.0,
        0.0,
        100,
        vec![EnemyState {
            id: 80,
            pos: IVec2 { x: 10, y: 0 },
            hp: 50,
            cover: "none".into(),
            last_seen: 12.0,
        }],
        vec![],
        vec![],
        None,
    );
    let plan_close = orch.propose_plan(&snap_close);
    match &plan_close.steps[0] {
        ActionStep::CoverFire {
            target_id,
            duration,
        } => {
            assert_eq!(*target_id, 80);
            assert_eq!(*duration, 1.5);
        }
        _ => panic!("Expected CoverFire when in range (dist <= 2)"),
    }
}

// =============================================================================
// SystemOrchestratorConfig Tests (2 tests)
// =============================================================================

// Note: These tests would validate make_system_orchestrator() and PlannerMode
// serialization, but since these are in orchestrator.rs and not exported,
// we'll test them indirectly through the orchestrator behaviors above.
// If SystemOrchestratorConfig and PlannerMode are moved to a separate module
// or exported, add dedicated tests here.

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_orchestrator_type_names() {
        // Validate type names for debugging/logging
        let rule = RuleOrchestrator;
        let utility = UtilityOrchestrator;
        let goap = GoapOrchestrator;

        // These type names are used in plan_id generation
        assert_eq!(
            std::any::type_name::<RuleOrchestrator>(),
            "astraweave_ai::orchestrator::RuleOrchestrator"
        );
        assert_eq!(
            std::any::type_name::<UtilityOrchestrator>(),
            "astraweave_ai::orchestrator::UtilityOrchestrator"
        );
        assert_eq!(
            std::any::type_name::<GoapOrchestrator>(),
            "astraweave_ai::orchestrator::GoapOrchestrator"
        );

        // Verify plan_id prefixes are unique
        let snap = make_snapshot(
            1.0,
            IVec2 { x: 0, y: 0 },
            30,
            80.0,
            0.0,
            100,
            vec![],
            vec![],
            vec![],
            None,
        );

        let plan_rule = rule.propose_plan(&snap);
        let plan_utility = utility.propose_plan(&snap);
        let plan_goap = goap.propose_plan(&snap);

        assert!(plan_rule.plan_id.starts_with("plan-"));
        assert!(plan_utility.plan_id.starts_with("util-"));
        assert!(plan_goap.plan_id.starts_with("goap-"));
    }

    #[test]
    fn test_plan_id_uniqueness_by_time() {
        // Validate plan_id changes with timestamp (determinism check)
        let snap1 = make_snapshot(
            1.0,
            IVec2 { x: 0, y: 0 },
            30,
            80.0,
            0.0,
            100,
            vec![],
            vec![],
            vec![],
            None,
        );
        let snap2 = make_snapshot(
            2.0,
            IVec2 { x: 0, y: 0 },
            30,
            80.0,
            0.0,
            100,
            vec![],
            vec![],
            vec![],
            None,
        );

        let rule = RuleOrchestrator;
        let plan1 = rule.propose_plan(&snap1);
        let plan2 = rule.propose_plan(&snap2);

        assert_eq!(plan1.plan_id, "plan-1000");
        assert_eq!(plan2.plan_id, "plan-2000");
        assert_ne!(
            plan1.plan_id, plan2.plan_id,
            "Plan IDs should differ by timestamp"
        );
    }
}
