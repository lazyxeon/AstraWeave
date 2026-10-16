//! Week 2 Day 2: Additional coverage tests for astraweave-ai orchestrator.rs
//!
//! Target: Fill coverage gaps from 44.83% (52/116) to 60%+ (~18 lines)
//! Focus areas:
//! - OrchestratorAsync trait default name() method
//! - RuleOrchestrator fallback paths (no enemies)
//! - UtilityOrchestrator edge cases (no candidates, tie-breaking)
//! - GoapOrchestrator next_action() fast path
//! - Async trait implementations (await paths)

use astraweave_ai::{
    GoapOrchestrator, Orchestrator, OrchestratorAsync, RuleOrchestrator, UtilityOrchestrator,
};
use astraweave_core::{ActionStep, CompanionState, EnemyState, IVec2, PlayerState, WorldSnapshot};
use std::collections::BTreeMap;

// ========== Helper Functions ==========

fn make_empty_snap() -> WorldSnapshot {
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

fn make_snap_with_enemy(enemy_pos: IVec2, cooldowns: BTreeMap<String, f32>) -> WorldSnapshot {
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

// ========== OrchestratorAsync Trait Tests ==========

#[tokio::test]
async fn test_orchestrator_async_default_name() {
    let rule = RuleOrchestrator;

    // Test default name() implementation (should return type name)
    let name = rule.name();
    assert!(name.contains("RuleOrchestrator"));
}

#[tokio::test]
async fn test_rule_orchestrator_async_plan() {
    let rule = RuleOrchestrator;
    let snap = make_snap_with_enemy(IVec2 { x: 10, y: 0 }, BTreeMap::new());

    // Test async plan() implementation
    let plan = rule.plan(snap, 1000).await.unwrap();

    // Should return 3-step plan (throw smoke, move, cover fire)
    assert_eq!(plan.steps.len(), 3);
    assert!(matches!(plan.steps[0], ActionStep::Throw { .. }));
}

#[tokio::test]
async fn test_utility_orchestrator_async_plan() {
    let utility = UtilityOrchestrator;
    let snap = make_snap_with_enemy(IVec2 { x: 5, y: 0 }, BTreeMap::new());

    // Test async plan() implementation
    let plan = utility.plan(snap, 1000).await.unwrap();

    // Should return at least one step
    assert!(!plan.steps.is_empty());
}

#[tokio::test]
async fn test_goap_orchestrator_async_plan() {
    let goap = GoapOrchestrator;
    let snap = make_snap_with_enemy(IVec2 { x: 10, y: 0 }, BTreeMap::new());

    // Test async plan() implementation
    let plan = goap.plan(snap, 1000).await.unwrap();

    // Should return 1-step plan (move toward enemy)
    assert_eq!(plan.steps.len(), 1);
    assert!(matches!(plan.steps[0], ActionStep::MoveTo { .. }));
}

// ========== RuleOrchestrator Edge Cases ==========

#[test]
fn test_rule_orchestrator_no_enemies() {
    let rule = RuleOrchestrator;
    let snap = make_empty_snap();

    // Test fallback path (no enemies)
    let plan = rule.propose_plan(&snap);

    // Should return empty plan
    assert_eq!(plan.steps.len(), 0);
    assert!(plan.plan_id.starts_with("plan-"));
}

#[test]
fn test_rule_orchestrator_smoke_on_cooldown() {
    let rule = RuleOrchestrator;
    let mut cooldowns = BTreeMap::new();
    cooldowns.insert("throw:smoke".into(), 5.0); // Smoke on cooldown
    let snap = make_snap_with_enemy(IVec2 { x: 10, y: 0 }, cooldowns);

    // Test cautious advance path (smoke unavailable)
    let plan = rule.propose_plan(&snap);

    // Should return 2-step plan (move, cover fire)
    assert_eq!(plan.steps.len(), 2);
    assert!(matches!(plan.steps[0], ActionStep::MoveTo { .. }));
    assert!(matches!(plan.steps[1], ActionStep::CoverFire { .. }));
}

#[test]
fn test_rule_orchestrator_negative_enemy_position() {
    let rule = RuleOrchestrator;
    let snap = make_snap_with_enemy(IVec2 { x: -5, y: -3 }, BTreeMap::new());

    // Test with negative coordinates
    let plan = rule.propose_plan(&snap);

    // Should still generate valid plan
    assert!(!plan.steps.is_empty());

    // Verify smoke throw position is midpoint
    if let ActionStep::Throw { x, y, .. } = &plan.steps[0] {
        assert_eq!(*x, -5 / 2); // -2 (rounds toward zero)
        assert_eq!(*y, -3 / 2); // -1
    } else {
        panic!("Expected Throw action");
    }
}

// ========== UtilityOrchestrator Edge Cases ==========

#[test]
fn test_utility_orchestrator_no_enemies() {
    let utility = UtilityOrchestrator;
    let snap = make_empty_snap();

    // Test with no enemies (no candidates)
    let plan = utility.propose_plan(&snap);

    // Should return empty plan (no candidates)
    assert_eq!(plan.steps.len(), 0);
    assert!(plan.plan_id.starts_with("util-"));
}

#[test]
fn test_utility_orchestrator_smoke_on_cooldown() {
    let utility = UtilityOrchestrator;
    let mut cooldowns = BTreeMap::new();
    cooldowns.insert("throw:smoke".into(), 3.0); // Smoke on cooldown
    let snap = make_snap_with_enemy(IVec2 { x: 5, y: 0 }, cooldowns);

    // Test with smoke unavailable (only advance candidate)
    let plan = utility.propose_plan(&snap);

    // Should return advance plan (only candidate)
    assert!(!plan.steps.is_empty());
    assert!(matches!(plan.steps[0], ActionStep::MoveTo { .. }));
}

#[test]
fn test_utility_orchestrator_close_enemy() {
    let utility = UtilityOrchestrator;
    let mut cooldowns = BTreeMap::new();
    cooldowns.insert("throw:smoke".into(), 1.0); // Smoke on cooldown
    let snap = make_snap_with_enemy(IVec2 { x: 2, y: 1 }, cooldowns); // Distance <= 3

    // Test with close enemy (should add cover fire)
    let plan = utility.propose_plan(&snap);

    // Should return move + cover fire
    assert_eq!(plan.steps.len(), 2);
    assert!(matches!(plan.steps[0], ActionStep::MoveTo { .. }));
    assert!(matches!(plan.steps[1], ActionStep::CoverFire { .. }));
}

#[test]
fn test_utility_orchestrator_far_enemy() {
    let utility = UtilityOrchestrator;
    let mut cooldowns = BTreeMap::new();
    cooldowns.insert("throw:smoke".into(), 1.0); // Smoke on cooldown
    let snap = make_snap_with_enemy(IVec2 { x: 10, y: 5 }, cooldowns); // Distance > 3

    // Test with far enemy (no cover fire)
    let plan = utility.propose_plan(&snap);

    // Should return move only (no cover fire for dist > 3)
    assert_eq!(plan.steps.len(), 1);
    assert!(matches!(plan.steps[0], ActionStep::MoveTo { .. }));
}

// ========== GoapOrchestrator next_action() Tests ==========

#[test]
fn test_goap_next_action_no_enemies() {
    let goap = GoapOrchestrator;
    let snap = make_empty_snap();

    // Test fast path with no enemies
    let action = goap.next_action(&snap);

    // Should return Wait action
    assert!(matches!(action, ActionStep::Wait { duration } if duration == 1.0));
}

#[test]
fn test_goap_next_action_in_range() {
    let goap = GoapOrchestrator;
    let snap = make_snap_with_enemy(IVec2 { x: 2, y: 0 }, BTreeMap::new()); // Distance = 2

    // Test fast path with enemy in range (dist <= 2)
    let action = goap.next_action(&snap);

    // Should return CoverFire action
    assert!(matches!(action, ActionStep::CoverFire { target_id: 42, duration } if duration == 1.5));
}

#[test]
fn test_goap_next_action_out_of_range() {
    let goap = GoapOrchestrator;
    let snap = make_snap_with_enemy(IVec2 { x: 10, y: 5 }, BTreeMap::new()); // Distance > 2

    // Test fast path with enemy out of range
    let action = goap.next_action(&snap);

    // Should return MoveTo action (one step closer)
    assert!(matches!(action, ActionStep::MoveTo { x: 1, y: 1, .. }));
}

#[test]
fn test_goap_next_action_negative_coords() {
    let goap = GoapOrchestrator;
    let snap = make_snap_with_enemy(IVec2 { x: -5, y: -3 }, BTreeMap::new());

    // Test fast path with negative coordinates
    let action = goap.next_action(&snap);

    // Should move one step closer (signum handles negative correctly)
    assert!(matches!(action, ActionStep::MoveTo { x: -1, y: -1, .. }));
}

// ========== GoapOrchestrator propose_plan() Edge Cases ==========

#[test]
fn test_goap_propose_plan_no_enemies() {
    let goap = GoapOrchestrator;
    let snap = make_empty_snap();

    // Test propose_plan with no enemies
    let plan = goap.propose_plan(&snap);

    // Should return empty plan
    assert_eq!(plan.steps.len(), 0);
    assert!(plan.plan_id.starts_with("goap-"));
}

#[test]
fn test_goap_propose_plan_in_range() {
    let goap = GoapOrchestrator;
    let snap = make_snap_with_enemy(IVec2 { x: 1, y: 1 }, BTreeMap::new()); // Distance = 2

    // Test propose_plan with enemy in range
    let plan = goap.propose_plan(&snap);

    // Should return 1-step plan (cover fire)
    assert_eq!(plan.steps.len(), 1);
    assert!(matches!(plan.steps[0], ActionStep::CoverFire { .. }));
}

#[test]
fn test_goap_propose_plan_out_of_range() {
    let goap = GoapOrchestrator;
    let snap = make_snap_with_enemy(IVec2 { x: 10, y: 0 }, BTreeMap::new()); // Distance > 2

    // Test propose_plan with enemy out of range
    let plan = goap.propose_plan(&snap);

    // Should return 1-step plan (move closer)
    assert_eq!(plan.steps.len(), 1);
    assert!(matches!(plan.steps[0], ActionStep::MoveTo { .. }));
}

// ========== Plan ID Generation Tests ==========

#[test]
fn test_plan_id_uniqueness() {
    let rule = RuleOrchestrator;

    // Create snapshots at different times
    let mut snap1 = make_snap_with_enemy(IVec2 { x: 5, y: 0 }, BTreeMap::new());
    snap1.t = 1.0;

    let mut snap2 = make_snap_with_enemy(IVec2 { x: 5, y: 0 }, BTreeMap::new());
    snap2.t = 2.5;

    let plan1 = rule.propose_plan(&snap1);
    let plan2 = rule.propose_plan(&snap2);

    // Plan IDs should be unique (based on timestamp)
    assert_ne!(plan1.plan_id, plan2.plan_id);
    assert_eq!(plan1.plan_id, "plan-1000"); // 1.0 * 1000
    assert_eq!(plan2.plan_id, "plan-2500"); // 2.5 * 1000
}

#[test]
fn test_utility_plan_id_format() {
    let utility = UtilityOrchestrator;
    let mut snap = make_snap_with_enemy(IVec2 { x: 5, y: 0 }, BTreeMap::new());
    snap.t = 3.25;

    let plan = utility.propose_plan(&snap);

    // Plan ID should follow util-{timestamp} format
    assert_eq!(plan.plan_id, "util-3250"); // 3.25 * 1000 = 3250
}

#[test]
fn test_goap_plan_id_format() {
    let goap = GoapOrchestrator;
    let mut snap = make_snap_with_enemy(IVec2 { x: 5, y: 0 }, BTreeMap::new());
    snap.t = 2.75;

    let plan = goap.propose_plan(&snap);

    // Plan ID should follow goap-{timestamp} format
    assert_eq!(plan.plan_id, "goap-2750"); // 2.75 * 1000 = 2750
}

// ========== Integration Tests ==========

#[test]
fn test_all_orchestrators_consistency() {
    // Test that all orchestrators handle same input consistently
    let snap = make_snap_with_enemy(IVec2 { x: 5, y: 0 }, BTreeMap::new());

    let rule = RuleOrchestrator;
    let utility = UtilityOrchestrator;
    let goap = GoapOrchestrator;

    let rule_plan = rule.propose_plan(&snap);
    let utility_plan = utility.propose_plan(&snap);
    let goap_plan = goap.propose_plan(&snap);

    // All should return non-empty plans (enemy exists)
    assert!(!rule_plan.steps.is_empty());
    assert!(!utility_plan.steps.is_empty());
    assert!(!goap_plan.steps.is_empty());

    // All should have valid plan IDs
    assert!(rule_plan.plan_id.starts_with("plan-"));
    assert!(utility_plan.plan_id.starts_with("util-"));
    assert!(goap_plan.plan_id.starts_with("goap-"));
}

#[tokio::test]
async fn test_all_orchestrators_async_consistency() {
    // Test that async implementations match sync implementations
    let snap = make_snap_with_enemy(IVec2 { x: 5, y: 0 }, BTreeMap::new());

    let rule = RuleOrchestrator;
    let utility = UtilityOrchestrator;
    let goap = GoapOrchestrator;

    // Sync versions
    let rule_sync = rule.propose_plan(&snap);
    let utility_sync = utility.propose_plan(&snap);
    let goap_sync = goap.propose_plan(&snap);

    // Async versions
    let rule_async = rule.plan(snap.clone(), 1000).await.unwrap();
    let utility_async = utility.plan(snap.clone(), 1000).await.unwrap();
    let goap_async = goap.plan(snap.clone(), 1000).await.unwrap();

    // Async should match sync
    assert_eq!(rule_sync.steps.len(), rule_async.steps.len());
    assert_eq!(utility_sync.steps.len(), utility_async.steps.len());
    assert_eq!(goap_sync.steps.len(), goap_async.steps.len());
}
