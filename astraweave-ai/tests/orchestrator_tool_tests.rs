//! Day 5: Tests for orchestrator.rs and tool_sandbox.rs (astraweave-ai)
//!
//! Target coverage:
//! - orchestrator.rs: 0% → 80%+ (27 uncovered lines)
//! - tool_sandbox.rs: 0% → 80%+ (8 uncovered lines)
//!
//! Test categories:
//! - RuleOrchestrator: smoke logic, fallback, no enemies
//! - UtilityOrchestrator: candidate scoring, sorting, fallback
//! - GoapOrchestrator: next_action fast path, distance logic
//! - ToolSandbox: validation (cooldown, ammo, LOS, physics, nav)
//! - ValidationContext: builder pattern, physics integration
//! - ToolError: Display impl, all variants

use astraweave_ai::{
    validate_tool_action, GoapOrchestrator, Orchestrator, RuleOrchestrator, ToolError, ToolVerb,
    UtilityOrchestrator, ValidationContext,
};
use astraweave_core::{ActionStep, CompanionState, EnemyState, IVec2, PlayerState, WorldSnapshot};
use astraweave_nav::NavMesh;
use rapier3d::prelude::*;
use std::collections::BTreeMap;

// ============================================================================
// Helper Functions
// ============================================================================

fn make_snap_with_enemy(
    enemy_pos: IVec2,
    my_ammo: i32,
    cooldowns: BTreeMap<String, f32>,
) -> WorldSnapshot {
    WorldSnapshot {
        t: 1.0,
        player: PlayerState {
            hp: 100,

            pos: IVec2 { x: 0, y: 0 },
            stance: "standing".into(),
            orders: vec![],
        },
        me: CompanionState {
            ammo: my_ammo,
            cooldowns,
            morale: 1.0,
            pos: IVec2 { x: 0, y: 0 },
        },
        enemies: vec![EnemyState {
            id: 42,
            pos: enemy_pos,
            hp: 50,
            cover: "none".into(),
            last_seen: 0.0,
        }],
        pois: vec![],
        objective: None,
        obstacles: vec![],

        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}

fn make_snap_no_enemies() -> WorldSnapshot {
    WorldSnapshot {
        t: 1.0,
        player: PlayerState {
            hp: 100,

            pos: IVec2 { x: 0, y: 0 },
            stance: "standing".into(),
            orders: vec![],
        },
        me: CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,
            pos: IVec2 { x: 0, y: 0 },
        },
        enemies: vec![],
        pois: vec![],
        objective: None,
        obstacles: vec![],

        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}

// ============================================================================
// RuleOrchestrator Tests (11 tests)
// ============================================================================

#[test]
fn test_rule_orchestrator_smoke_logic() {
    // Enemy at (6, 6), no cooldown - should throw smoke at midpoint (3, 3)
    let snap = make_snap_with_enemy(IVec2 { x: 6, y: 6 }, 10, BTreeMap::new());
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert!(!plan.steps.is_empty());
    assert_eq!(plan.steps.len(), 3);

    // First step: throw smoke at midpoint
    match &plan.steps[0] {
        ActionStep::Throw { item, x, y } => {
            assert_eq!(item, "smoke");
            assert_eq!(*x, 3); // midpoint x
            assert_eq!(*y, 3); // midpoint y
        }
        _ => panic!("Expected Throw action"),
    }

    // Second step: move closer
    match &plan.steps[1] {
        ActionStep::MoveTo { x, y, .. } => {
            assert_eq!(*x, 2); // 0 + signum(6-0) * 2
            assert_eq!(*y, 2); // 0 + signum(6-0) * 2
        }
        _ => panic!("Expected MoveTo action"),
    }

    // Third step: cover fire
    match &plan.steps[2] {
        ActionStep::CoverFire {
            target_id,
            duration,
        } => {
            assert_eq!(*target_id, 42);
            assert_eq!(*duration, 2.5);
        }
        _ => panic!("Expected CoverFire action"),
    }
}

#[test]
fn test_rule_orchestrator_smoke_on_cooldown() {
    // Enemy at (4, 4), smoke on cooldown (5.0) - should advance cautiously
    let mut cooldowns = BTreeMap::new();
    cooldowns.insert("throw:smoke".into(), 5.0);
    let snap = make_snap_with_enemy(IVec2 { x: 4, y: 4 }, 10, cooldowns);
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert_eq!(plan.steps.len(), 2);

    // First step: advance one step closer
    match &plan.steps[0] {
        ActionStep::MoveTo { x, y, .. } => {
            assert_eq!(*x, 1); // 0 + signum(4-0)
            assert_eq!(*y, 1); // 0 + signum(4-0)
        }
        _ => panic!("Expected MoveTo action"),
    }

    // Second step: cover fire
    match &plan.steps[1] {
        ActionStep::CoverFire {
            target_id,
            duration,
        } => {
            assert_eq!(*target_id, 42);
            assert_eq!(*duration, 1.5);
        }
        _ => panic!("Expected CoverFire action"),
    }
}

#[test]
fn test_rule_orchestrator_no_enemies() {
    // No enemies - should return empty plan
    let snap = make_snap_no_enemies();
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert!(plan.steps.is_empty());
    assert!(plan.plan_id.starts_with("plan-"));
}

#[test]
fn test_rule_orchestrator_plan_id_generation() {
    // Plan ID should be based on timestamp
    let snap = make_snap_with_enemy(IVec2 { x: 5, y: 5 }, 10, BTreeMap::new());
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert!(plan.plan_id.starts_with("plan-"));
    assert_eq!(plan.plan_id, "plan-1000"); // t=1.0 * 1000 = 1000
}

#[test]
fn test_rule_orchestrator_smoke_cooldown_zero() {
    // Cooldown exactly 0.0 should trigger smoke logic
    let mut cooldowns = BTreeMap::new();
    cooldowns.insert("throw:smoke".into(), 0.0);
    let snap = make_snap_with_enemy(IVec2 { x: 8, y: 8 }, 10, cooldowns);
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert_eq!(plan.steps.len(), 3);

    // First step should be Throw (cooldown 0.0 is ready)
    match &plan.steps[0] {
        ActionStep::Throw { item, .. } => {
            assert_eq!(item, "smoke");
        }
        _ => panic!("Expected Throw action with 0.0 cooldown"),
    }
}

#[test]
fn test_rule_orchestrator_negative_enemy_pos() {
    // Enemy at negative position (-5, -5) - should handle signum correctly
    let snap = make_snap_with_enemy(IVec2 { x: -5, y: -5 }, 10, BTreeMap::new());
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert_eq!(plan.steps.len(), 3);

    // Midpoint should be (-2, -2) - integer division rounds toward zero
    match &plan.steps[0] {
        ActionStep::Throw { x, y, .. } => {
            assert_eq!(*x, -2); // (0 + -5) / 2 = -2 (rounds toward zero in integer division)
            assert_eq!(*y, -2);
        }
        _ => panic!("Expected Throw action"),
    }

    // MoveTo should be (-2, -2)
    match &plan.steps[1] {
        ActionStep::MoveTo { x, y, .. } => {
            assert_eq!(*x, -2); // 0 + signum(-5-0) * 2
            assert_eq!(*y, -2);
        }
        _ => panic!("Expected MoveTo action"),
    }
}

// ============================================================================
// UtilityOrchestrator Tests (6 tests)
// ============================================================================

#[test]
fn test_utility_orchestrator_smoke_candidate() {
    // Enemy at (6, 6), no cooldown - should score smoke candidate
    let snap = make_snap_with_enemy(IVec2 { x: 6, y: 6 }, 10, BTreeMap::new());
    let orch = UtilityOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert!(!plan.steps.is_empty());
    assert!(plan.plan_id.starts_with("util-"));

    // Should have at least 2 steps (smoke + move)
    assert!(plan.steps.len() >= 2);

    // First step should be Throw (smoke candidate has higher score)
    match &plan.steps[0] {
        ActionStep::Throw { item, x, y } => {
            assert_eq!(item, "smoke");
            assert_eq!(*x, 3); // midpoint
            assert_eq!(*y, 3);
        }
        _ => panic!("Expected Throw action as highest utility"),
    }
}

#[test]
fn test_utility_orchestrator_advance_candidate() {
    // Enemy at (2, 2), smoke on cooldown - should choose advance candidate
    let mut cooldowns = BTreeMap::new();
    cooldowns.insert("throw:smoke".into(), 5.0);
    let snap = make_snap_with_enemy(IVec2 { x: 2, y: 2 }, 10, cooldowns);
    let orch = UtilityOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert!(!plan.steps.is_empty());

    // First step should be MoveTo (no smoke candidate due to cooldown)
    match &plan.steps[0] {
        ActionStep::MoveTo { x, y, .. } => {
            assert_eq!(*x, 1); // 0 + signum(2-0)
            assert_eq!(*y, 1);
        }
        _ => panic!("Expected MoveTo action"),
    }
}

#[test]
fn test_utility_orchestrator_cover_fire_when_close() {
    // Enemy at (2, 1), distance = 3 (2+1), should add cover fire to advance candidate
    // Block smoke candidate with cooldown to force advance candidate selection
    let mut cooldowns = BTreeMap::new();
    cooldowns.insert("throw:smoke".into(), 5.0);
    let snap = make_snap_with_enemy(IVec2 { x: 2, y: 1 }, 10, cooldowns);
    let orch = UtilityOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert!(plan.steps.len() >= 2);

    // Should have MoveTo + CoverFire (advance candidate with dist <= 3)
    let has_cover_fire = plan
        .steps
        .iter()
        .any(|step| matches!(step, ActionStep::CoverFire { .. }));
    assert!(has_cover_fire, "Expected CoverFire when distance <= 3");
}

#[test]
fn test_utility_orchestrator_no_cover_fire_when_far() {
    // Enemy at (10, 10), distance = 20, should not add cover fire
    let mut cooldowns = BTreeMap::new();
    cooldowns.insert("throw:smoke".into(), 5.0); // Block smoke candidate
    let snap = make_snap_with_enemy(IVec2 { x: 10, y: 10 }, 10, cooldowns);
    let orch = UtilityOrchestrator;
    let plan = orch.propose_plan(&snap);

    // Should have only MoveTo (distance > 3)
    assert_eq!(plan.steps.len(), 1);
    match &plan.steps[0] {
        ActionStep::MoveTo { .. } => {}
        _ => panic!("Expected only MoveTo when far"),
    }
}

#[test]
fn test_utility_orchestrator_no_enemies() {
    // No enemies - should return empty plan
    let snap = make_snap_no_enemies();
    let orch = UtilityOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert!(plan.steps.is_empty());
    assert!(plan.plan_id.starts_with("util-"));
}

#[test]
fn test_utility_orchestrator_candidate_sorting() {
    // Enemy at (1, 1), distance = 2 (very close)
    // Smoke candidate score ~= 1.0 + player_hp*0.0 + enemy_hp*0.01 = 1.0 + 50*0.01 = 1.5
    // Advance candidate score ~= 0.8 + (3.0-2.0)*0.05 = 0.85
    // Smoke should win (higher score)
    let snap = make_snap_with_enemy(IVec2 { x: 1, y: 1 }, 10, BTreeMap::new());
    let orch = UtilityOrchestrator;
    let plan = orch.propose_plan(&snap);

    // Should prioritize smoke (higher score)
    match &plan.steps[0] {
        ActionStep::Throw { item, .. } => {
            assert_eq!(item, "smoke");
        }
        _ => panic!("Expected Throw action to have highest utility score"),
    }
}

// ============================================================================
// GoapOrchestrator Tests (7 tests)
// ============================================================================

#[test]
fn test_goap_orchestrator_next_action_move() {
    // Enemy at (5, 5), distance = 10, should move toward enemy
    let snap = make_snap_with_enemy(IVec2 { x: 5, y: 5 }, 10, BTreeMap::new());
    let orch = GoapOrchestrator;
    let action = orch.next_action(&snap);

    match action {
        ActionStep::MoveTo { x, y, .. } => {
            assert_eq!(x, 1); // 0 + signum(5-0)
            assert_eq!(y, 1);
        }
        _ => panic!("Expected MoveTo action"),
    }
}

#[test]
fn test_goap_orchestrator_next_action_cover_fire() {
    // Enemy at (1, 1), distance = 2, should cover fire
    let snap = make_snap_with_enemy(IVec2 { x: 1, y: 1 }, 10, BTreeMap::new());
    let orch = GoapOrchestrator;
    let action = orch.next_action(&snap);

    match action {
        ActionStep::CoverFire {
            target_id,
            duration,
        } => {
            assert_eq!(target_id, 42);
            assert_eq!(duration, 1.5);
        }
        _ => panic!("Expected CoverFire action when dist <= 2"),
    }
}

#[test]
fn test_goap_orchestrator_next_action_wait() {
    // No enemies - should wait
    let snap = make_snap_no_enemies();
    let orch = GoapOrchestrator;
    let action = orch.next_action(&snap);

    match action {
        ActionStep::Wait { duration } => {
            assert_eq!(duration, 1.0);
        }
        _ => panic!("Expected Wait action when no enemies"),
    }
}

#[test]
fn test_goap_orchestrator_propose_plan_move() {
    // Enemy at (5, 5), distance = 10, should return MoveTo plan
    let snap = make_snap_with_enemy(IVec2 { x: 5, y: 5 }, 10, BTreeMap::new());
    let orch = GoapOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert_eq!(plan.steps.len(), 1);
    assert!(plan.plan_id.starts_with("goap-"));

    match &plan.steps[0] {
        ActionStep::MoveTo { x, y, .. } => {
            assert_eq!(*x, 1);
            assert_eq!(*y, 1);
        }
        _ => panic!("Expected MoveTo action"),
    }
}

#[test]
fn test_goap_orchestrator_propose_plan_cover_fire() {
    // Enemy at (2, 0), distance = 2, should return CoverFire plan
    let snap = make_snap_with_enemy(IVec2 { x: 2, y: 0 }, 10, BTreeMap::new());
    let orch = GoapOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert_eq!(plan.steps.len(), 1);

    match &plan.steps[0] {
        ActionStep::CoverFire {
            target_id,
            duration,
        } => {
            assert_eq!(*target_id, 42);
            assert_eq!(*duration, 1.5);
        }
        _ => panic!("Expected CoverFire action"),
    }
}

#[test]
fn test_goap_orchestrator_propose_plan_no_enemies() {
    // No enemies - should return empty plan
    let snap = make_snap_no_enemies();
    let orch = GoapOrchestrator;
    let plan = orch.propose_plan(&snap);

    assert!(plan.steps.is_empty());
    assert!(plan.plan_id.starts_with("goap-"));
}

#[test]
fn test_goap_orchestrator_boundary_distance() {
    // Enemy at (1, 1), distance = 2 (exactly at boundary)
    let snap = make_snap_with_enemy(IVec2 { x: 1, y: 1 }, 10, BTreeMap::new());
    let orch = GoapOrchestrator;
    let plan = orch.propose_plan(&snap);

    // Distance = 2, should cover fire (dist <= 2)
    assert_eq!(plan.steps.len(), 1);
    match &plan.steps[0] {
        ActionStep::CoverFire { .. } => {}
        _ => panic!("Expected CoverFire at distance boundary"),
    }
}

// ============================================================================
// ToolSandbox ValidationContext Tests (5 tests)
// ============================================================================

#[test]
fn test_validation_context_default() {
    let ctx = ValidationContext::new();

    assert!(ctx.nav_mesh.is_none());
    assert!(ctx.physics_pipeline.is_none());
    assert!(ctx.rigid_body_set.is_none());
    assert!(ctx.collider_set.is_none());
}

#[test]
fn test_validation_context_with_nav() {
    let nav = NavMesh::bake(&[], 0.4, 60.0);
    let ctx = ValidationContext::new().with_nav(&nav);

    assert!(ctx.nav_mesh.is_some());
    assert!(ctx.physics_pipeline.is_none());
}

#[test]
fn test_validation_context_with_physics() {
    let pipeline = PhysicsPipeline::new();
    let bodies = RigidBodySet::new();
    let colliders = ColliderSet::new();

    let ctx = ValidationContext::new().with_physics(&pipeline, &bodies, &colliders);

    assert!(ctx.physics_pipeline.is_some());
    assert!(ctx.rigid_body_set.is_some());
    assert!(ctx.collider_set.is_some());
    assert!(ctx.nav_mesh.is_none());
}

#[test]
fn test_validation_context_chained_builders() {
    let nav = NavMesh::bake(&[], 0.4, 60.0);
    let pipeline = PhysicsPipeline::new();
    let bodies = RigidBodySet::new();
    let colliders = ColliderSet::new();

    let ctx = ValidationContext::new()
        .with_nav(&nav)
        .with_physics(&pipeline, &bodies, &colliders);

    assert!(ctx.nav_mesh.is_some());
    assert!(ctx.physics_pipeline.is_some());
    assert!(ctx.rigid_body_set.is_some());
    assert!(ctx.collider_set.is_some());
}

#[test]
fn test_validation_context_multiple_with_nav_calls() {
    let nav1 = NavMesh::bake(&[], 0.4, 60.0);
    let nav2 = NavMesh::bake(&[], 0.5, 45.0);

    let ctx = ValidationContext::new().with_nav(&nav1).with_nav(&nav2);

    assert!(ctx.nav_mesh.is_some());
    // Should have nav2 (last call wins)
    assert_eq!(ctx.nav_mesh.unwrap().max_step, 0.5);
}

// ============================================================================
// ToolSandbox Validation Tests (12 tests)
// ============================================================================

#[test]
fn test_validate_move_to_no_nav_no_physics() {
    // MoveTo without nav or physics should succeed (no validation)
    let snap = make_snap_with_enemy(IVec2 { x: 5, y: 5 }, 10, BTreeMap::new());
    let ctx = ValidationContext::new();

    let result = validate_tool_action(0, ToolVerb::MoveTo, &snap, &ctx, Some(IVec2 { x: 5, y: 5 }));
    assert!(result.is_ok());
}

#[test]
fn test_validate_move_to_cooldown() {
    // MoveTo with active cooldown should fail
    let mut cooldowns = BTreeMap::new();
    cooldowns.insert("moveto".into(), 2.5);
    let snap = make_snap_with_enemy(IVec2 { x: 5, y: 5 }, 10, cooldowns);
    let ctx = ValidationContext::new();

    let result = validate_tool_action(0, ToolVerb::MoveTo, &snap, &ctx, Some(IVec2 { x: 5, y: 5 }));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cooldown"));
}

#[test]
fn test_validate_throw_insufficient_ammo() {
    // Throw with 0 ammo should fail
    let snap = make_snap_with_enemy(IVec2 { x: 5, y: 5 }, 0, BTreeMap::new());
    let ctx = ValidationContext::new();

    let result = validate_tool_action(0, ToolVerb::Throw, &snap, &ctx, Some(IVec2 { x: 5, y: 5 }));
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("insufficient ammo"));
}

#[test]
fn test_validate_throw_no_line_of_sight() {
    // Throw with obstacle blocking LOS should fail
    let mut snap = make_snap_with_enemy(IVec2 { x: 5, y: 0 }, 10, BTreeMap::new());
    snap.obstacles = vec![IVec2 { x: 2, y: 0 }]; // Obstacle between (0,0) and (5,0)
    let ctx = ValidationContext::new();

    let result = validate_tool_action(0, ToolVerb::Throw, &snap, &ctx, Some(IVec2 { x: 5, y: 0 }));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no line of sight"));
}

#[test]
fn test_validate_throw_success() {
    // Throw with ammo and clear LOS should succeed
    let snap = make_snap_with_enemy(IVec2 { x: 5, y: 5 }, 10, BTreeMap::new());
    let ctx = ValidationContext::new();

    let result = validate_tool_action(0, ToolVerb::Throw, &snap, &ctx, Some(IVec2 { x: 5, y: 5 }));
    assert!(result.is_ok());
}

#[test]
fn test_validate_cover_fire_insufficient_ammo() {
    // CoverFire with 0 ammo should fail
    let snap = make_snap_with_enemy(IVec2 { x: 2, y: 0 }, 0, BTreeMap::new());
    let ctx = ValidationContext::new();

    let result = validate_tool_action(
        0,
        ToolVerb::CoverFire,
        &snap,
        &ctx,
        Some(IVec2 { x: 2, y: 0 }),
    );
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("insufficient ammo"));
}

#[test]
fn test_validate_cover_fire_no_line_of_sight() {
    // CoverFire with obstacle blocking LOS should fail
    let mut snap = make_snap_with_enemy(IVec2 { x: 3, y: 0 }, 10, BTreeMap::new());
    snap.obstacles = vec![IVec2 { x: 1, y: 0 }];
    let ctx = ValidationContext::new();

    let result = validate_tool_action(
        0,
        ToolVerb::CoverFire,
        &snap,
        &ctx,
        Some(IVec2 { x: 3, y: 0 }),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no line of sight"));
}

#[test]
fn test_validate_revive_low_morale() {
    // Revive with morale < 0.5 should fail
    let mut snap = make_snap_with_enemy(IVec2 { x: 1, y: 0 }, 10, BTreeMap::new());
    snap.me.morale = 0.3;
    let ctx = ValidationContext::new();

    let result = validate_tool_action(0, ToolVerb::Revive, &snap, &ctx, Some(IVec2 { x: 1, y: 0 }));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("low morale"));
}

#[test]
fn test_validate_revive_target_too_far() {
    // Revive with target >2.0 distance should fail
    let snap = make_snap_with_enemy(IVec2 { x: 3, y: 3 }, 10, BTreeMap::new());
    let ctx = ValidationContext::new();

    let result = validate_tool_action(0, ToolVerb::Revive, &snap, &ctx, Some(IVec2 { x: 3, y: 3 }));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("too far"));
}

#[test]
fn test_validate_revive_success() {
    // Revive with morale >= 0.5 and close target should succeed
    let snap = make_snap_with_enemy(IVec2 { x: 1, y: 0 }, 10, BTreeMap::new());
    let ctx = ValidationContext::new();

    let result = validate_tool_action(0, ToolVerb::Revive, &snap, &ctx, Some(IVec2 { x: 1, y: 0 }));
    assert!(result.is_ok());
}

#[test]
fn test_validate_stay_no_checks() {
    // Stay should always succeed (no validation checks)
    let snap = make_snap_with_enemy(IVec2 { x: 5, y: 5 }, 0, BTreeMap::new());
    let ctx = ValidationContext::new();

    let result = validate_tool_action(0, ToolVerb::Stay, &snap, &ctx, None);
    assert!(result.is_ok());
}

#[test]
fn test_validate_wander_no_checks() {
    // Wander should always succeed (no validation checks)
    let snap = make_snap_no_enemies();
    let ctx = ValidationContext::new();

    let result = validate_tool_action(0, ToolVerb::Wander, &snap, &ctx, None);
    assert!(result.is_ok());
}

// ============================================================================
// ToolError Display Tests (8 tests)
// ============================================================================

#[test]
fn test_tool_error_out_of_bounds_display() {
    assert_eq!(ToolError::OutOfBounds.to_string(), "OutOfBounds");
}

#[test]
fn test_tool_error_cooldown_display() {
    assert_eq!(ToolError::Cooldown.to_string(), "Cooldown");
}

#[test]
fn test_tool_error_no_line_of_sight_display() {
    assert_eq!(ToolError::NoLineOfSight.to_string(), "NoLineOfSight");
}

#[test]
fn test_tool_error_insufficient_resource_display() {
    assert_eq!(
        ToolError::InsufficientResource.to_string(),
        "InsufficientResource"
    );
}

#[test]
fn test_tool_error_invalid_target_display() {
    assert_eq!(ToolError::InvalidTarget.to_string(), "InvalidTarget");
}

#[test]
fn test_tool_error_physics_blocked_display() {
    assert_eq!(ToolError::PhysicsBlocked.to_string(), "PhysicsBlocked");
}

#[test]
fn test_tool_error_no_path_display() {
    assert_eq!(ToolError::NoPath.to_string(), "NoPath");
}

#[test]
fn test_tool_error_unknown_display() {
    assert_eq!(ToolError::Unknown.to_string(), "Unknown");
}

// ============================================================================
// Additional Edge Case Tests (4 tests)
// ============================================================================

#[test]
fn test_rule_orchestrator_enemy_at_origin() {
    // Enemy at (0, 0), same position as companion
    let snap = make_snap_with_enemy(IVec2 { x: 0, y: 0 }, 10, BTreeMap::new());
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);

    // Should still generate plan (signum of 0 is 0)
    assert_eq!(plan.steps.len(), 3);
}

#[test]
fn test_utility_orchestrator_equal_scores() {
    // Test candidate sorting with potentially equal scores
    let snap = make_snap_with_enemy(IVec2 { x: 1, y: 0 }, 10, BTreeMap::new());
    let orch = UtilityOrchestrator;
    let plan = orch.propose_plan(&snap);

    // Should produce valid plan regardless of sorting order
    assert!(!plan.steps.is_empty());
}

#[test]
fn test_goap_orchestrator_manhattan_distance() {
    // Enemy at (3, 4), Manhattan distance = 7, should move
    let snap = make_snap_with_enemy(IVec2 { x: 3, y: 4 }, 10, BTreeMap::new());
    let orch = GoapOrchestrator;
    let action = orch.next_action(&snap);

    match action {
        ActionStep::MoveTo { x, y, .. } => {
            assert_eq!(x, 1); // 0 + signum(3)
            assert_eq!(y, 1); // 0 + signum(4)
        }
        _ => panic!("Expected MoveTo for distance > 2"),
    }
}

#[test]
fn test_validate_tool_action_no_target_pos() {
    // Some actions don't require target_pos (None)
    let snap = make_snap_no_enemies();
    let ctx = ValidationContext::new();

    let result = validate_tool_action(0, ToolVerb::Stay, &snap, &ctx, None);
    assert!(result.is_ok());
}

// ============================================================================
// OrchestratorAsync Trait Tests (6 tests)
// ============================================================================

use astraweave_ai::OrchestratorAsync;

#[tokio::test]
async fn test_rule_orchestrator_async_plan() {
    let snap = make_snap_with_enemy(IVec2 { x: 5, y: 5 }, 10, BTreeMap::new());
    let orch = RuleOrchestrator;

    let plan = orch.plan(snap, 100).await.expect("plan should succeed");

    assert!(!plan.steps.is_empty());
    assert!(plan.plan_id.starts_with("plan-"));
}

#[tokio::test]
async fn test_rule_orchestrator_async_name() {
    let orch = RuleOrchestrator;
    let name = orch.name();

    assert!(name.contains("RuleOrchestrator"));
}

#[tokio::test]
async fn test_utility_orchestrator_async_plan() {
    let snap = make_snap_with_enemy(IVec2 { x: 3, y: 3 }, 10, BTreeMap::new());
    let orch = UtilityOrchestrator;

    let plan = orch.plan(snap, 100).await.expect("plan should succeed");

    assert!(!plan.steps.is_empty());
    assert!(plan.plan_id.starts_with("util-"));
}

#[tokio::test]
async fn test_utility_orchestrator_async_name() {
    let orch = UtilityOrchestrator;
    let name = orch.name();

    assert!(name.contains("UtilityOrchestrator"));
}

#[tokio::test]
async fn test_goap_orchestrator_async_plan() {
    let snap = make_snap_with_enemy(IVec2 { x: 4, y: 4 }, 10, BTreeMap::new());
    let orch = GoapOrchestrator;

    let plan = orch.plan(snap, 100).await.expect("plan should succeed");

    assert_eq!(plan.steps.len(), 1);
    assert!(plan.plan_id.starts_with("goap-"));
}

#[tokio::test]
async fn test_goap_orchestrator_async_name() {
    let orch = GoapOrchestrator;
    let name = orch.name();

    assert!(name.contains("GoapOrchestrator"));
}
