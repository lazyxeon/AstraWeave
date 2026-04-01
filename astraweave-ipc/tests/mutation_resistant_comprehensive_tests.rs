//! Mutation-resistant comprehensive tests for astraweave-ipc.
//!
//! Covers:
//! - WebSocket server/client roundtrip (`run_ws_server` + `ws_client_roundtrip`)
//! - WorldSnapshot ↔ JSON ↔ PlanIntent pipeline fidelity
//! - RuleOrchestrator integration through the IPC layer
//! - Boundary conditions, edge cases, exact field assertions
//!
//! These tests ensure that mutations to serialization, routing, or
//! orchestrator integration are immediately detected.

use astraweave_ai::{Orchestrator, RuleOrchestrator};
use astraweave_core::schema::{CompanionState, CoverType, EnemyState, IVec2, PlayerState, Poi, Stance};
use astraweave_core::{ActionStep, PlanIntent, WorldSnapshot};
use std::collections::BTreeMap;

// ════════════════════════════════════════════════════════════════════
//  Helper: construct WorldSnapshots with controlled fields
// ════════════════════════════════════════════════════════════════════

fn base_snapshot() -> WorldSnapshot {
    WorldSnapshot {
        t: 0.0,
        player: PlayerState {
            pos: IVec2::new(5, 5),
            hp: 100,
            stance: Stance::Stand,
            orders: vec![],
        },
        me: CompanionState {
            ammo: 30,
            cooldowns: BTreeMap::new(),
            morale: 1.0,
            pos: IVec2::new(3, 3),
        },
        enemies: vec![EnemyState {
            id: 0,
            pos: IVec2::new(10, 10),
            hp: 50,
            cover: CoverType::None,
            last_seen: 0.0,
        }],
        pois: vec![Poi {
            pos: IVec2::new(15, 15),
            k: "objective".to_string(),
        }],
        obstacles: vec![IVec2::new(7, 7)],
        objective: Some("Reach the extraction point".to_string()),
    }
}

fn empty_snapshot() -> WorldSnapshot {
    WorldSnapshot {
        t: 0.0,
        player: PlayerState {
            pos: IVec2::new(0, 0),
            hp: 100,
            stance: Stance::Stand,
            orders: vec![],
        },
        me: CompanionState {
            ammo: 0,
            cooldowns: BTreeMap::new(),
            morale: 0.5,
            pos: IVec2::new(0, 0),
        },
        enemies: vec![],
        pois: vec![],
        obstacles: vec![],
        objective: None,
    }
}

/// Find a free TCP port by binding to 0
fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

// ════════════════════════════════════════════════════════════════════
//  SECTION 1 — WorldSnapshot JSON round-trip fidelity
// ════════════════════════════════════════════════════════════════════

#[test]
fn snapshot_json_roundtrip_preserves_t() {
    let snap = base_snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.t, 0.0);
}

#[test]
fn snapshot_json_roundtrip_preserves_player_hp() {
    let snap = base_snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.player.hp, 100);
}

#[test]
fn snapshot_json_roundtrip_preserves_player_pos() {
    let snap = base_snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.player.pos.x, 5);
    assert_eq!(rt.player.pos.y, 5);
}

#[test]
fn snapshot_json_roundtrip_preserves_player_stance() {
    let snap = base_snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.player.stance, Stance::Stand);
}

#[test]
fn snapshot_json_roundtrip_preserves_companion_ammo() {
    let snap = base_snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.me.ammo, 30);
}

#[test]
fn snapshot_json_roundtrip_preserves_companion_morale() {
    let snap = base_snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.me.morale, 1.0);
}

#[test]
fn snapshot_json_roundtrip_preserves_companion_pos() {
    let snap = base_snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.me.pos.x, 3);
    assert_eq!(rt.me.pos.y, 3);
}

#[test]
fn snapshot_json_roundtrip_preserves_enemy_fields() {
    let snap = base_snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.enemies.len(), 1);
    let e = &rt.enemies[0];
    assert_eq!(e.id, 0);
    assert_eq!(e.pos.x, 10);
    assert_eq!(e.pos.y, 10);
    assert_eq!(e.hp, 50);
    assert_eq!(e.cover, CoverType::None);
    assert_eq!(e.last_seen, 0.0);
}

#[test]
fn snapshot_json_roundtrip_preserves_pois() {
    let snap = base_snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.pois.len(), 1);
    assert_eq!(rt.pois[0].k, "objective");
    assert_eq!(rt.pois[0].pos.x, 15);
    assert_eq!(rt.pois[0].pos.y, 15);
}

#[test]
fn snapshot_json_roundtrip_preserves_obstacles() {
    let snap = base_snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.obstacles.len(), 1);
    assert_eq!(rt.obstacles[0].x, 7);
    assert_eq!(rt.obstacles[0].y, 7);
}

#[test]
fn snapshot_json_roundtrip_preserves_objective() {
    let snap = base_snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.objective.as_deref(), Some("Reach the extraction point"));
}

#[test]
fn snapshot_json_empty_roundtrip() {
    let snap = empty_snapshot();
    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert!(rt.enemies.is_empty());
    assert!(rt.pois.is_empty());
    assert!(rt.obstacles.is_empty());
    assert!(rt.objective.is_none());
    assert_eq!(rt.me.ammo, 0);
}

// ════════════════════════════════════════════════════════════════════
//  SECTION 2 — PlanIntent JSON round-trip fidelity
// ════════════════════════════════════════════════════════════════════

#[test]
fn plan_intent_json_roundtrip_plan_id() {
    let plan = PlanIntent {
        plan_id: "test-plan-001".to_string(),
        steps: vec![ActionStep::MoveTo {
            x: 10,
            y: 10,
            speed: None,
        }],
    };
    let json = serde_json::to_string(&plan).unwrap();
    let rt: PlanIntent = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.plan_id, "test-plan-001");
}

#[test]
fn plan_intent_json_roundtrip_steps_count() {
    let plan = PlanIntent {
        plan_id: "p".to_string(),
        steps: vec![
            ActionStep::MoveTo {
                x: 1,
                y: 2,
                speed: None,
            },
            ActionStep::Attack { target_id: 99 },
        ],
    };
    let json = serde_json::to_string(&plan).unwrap();
    let rt: PlanIntent = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.steps.len(), 2);
}

#[test]
fn plan_intent_json_roundtrip_empty_steps() {
    let plan = PlanIntent {
        plan_id: "empty".to_string(),
        steps: vec![],
    };
    let json = serde_json::to_string(&plan).unwrap();
    let rt: PlanIntent = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.plan_id, "empty");
    assert!(rt.steps.is_empty());
}

// ════════════════════════════════════════════════════════════════════
//  SECTION 3 — RuleOrchestrator plan logic (verified through IPC path)
// ════════════════════════════════════════════════════════════════════

#[test]
fn rule_orch_plan_id_derived_from_time() {
    let mut snap = base_snapshot();
    snap.t = 1.5;
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);
    // plan_id = format!("plan-{}", (snap.t * 1000.0) as i64)
    assert_eq!(plan.plan_id, "plan-1500");
}

#[test]
fn rule_orch_plan_id_zero_time() {
    let snap = base_snapshot();
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);
    assert_eq!(plan.plan_id, "plan-0");
}

#[test]
fn rule_orch_plan_id_fractional_time() {
    let mut snap = base_snapshot();
    snap.t = 0.001;
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);
    assert_eq!(plan.plan_id, "plan-1");
}

#[test]
fn rule_orch_with_enemy_and_no_cooldown_produces_3_steps() {
    // Enemy present, no cooldowns → Throw + MoveTo + CoverFire
    let snap = base_snapshot();
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);
    assert_eq!(plan.steps.len(), 3);
}

#[test]
fn rule_orch_with_enemy_no_cooldown_first_step_is_throw_smoke() {
    let snap = base_snapshot();
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);
    match &plan.steps[0] {
        ActionStep::Throw { item, x, y } => {
            assert_eq!(item, "smoke");
            // midpoint of companion (3,3) and enemy (10,10) = (6, 6)
            assert_eq!(*x, 6);
            assert_eq!(*y, 6);
        }
        other => panic!("Expected Throw, got {:?}", other),
    }
}

#[test]
fn rule_orch_with_enemy_no_cooldown_second_step_is_move_to() {
    let snap = base_snapshot();
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);
    match &plan.steps[1] {
        ActionStep::MoveTo { x, y, speed } => {
            // companion (3,3), enemy (10,10) → signum = (1,1), *2 = (2,2), + pos = (5,5)
            assert_eq!(*x, 5);
            assert_eq!(*y, 5);
            assert!(speed.is_none());
        }
        other => panic!("Expected MoveTo, got {:?}", other),
    }
}

#[test]
fn rule_orch_with_enemy_no_cooldown_third_step_is_cover_fire() {
    let snap = base_snapshot();
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);
    match &plan.steps[2] {
        ActionStep::CoverFire {
            target_id,
            duration,
        } => {
            assert_eq!(*target_id, 0);
            assert_eq!(*duration, 2.5);
        }
        other => panic!("Expected CoverFire, got {:?}", other),
    }
}

#[test]
fn rule_orch_with_enemy_on_cooldown_produces_2_steps() {
    let mut snap = base_snapshot();
    snap.me.cooldowns.insert("throw:smoke".to_string(), 5.0);
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);
    assert_eq!(plan.steps.len(), 2);
}

#[test]
fn rule_orch_with_enemy_on_cooldown_first_step_is_move_to() {
    let mut snap = base_snapshot();
    snap.me.cooldowns.insert("throw:smoke".to_string(), 5.0);
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);
    match &plan.steps[0] {
        ActionStep::MoveTo { x, y, .. } => {
            // companion (3,3), enemy (10,10) → signum = (1,1), + pos = (4,4)
            assert_eq!(*x, 4);
            assert_eq!(*y, 4);
        }
        other => panic!("Expected MoveTo, got {:?}", other),
    }
}

#[test]
fn rule_orch_with_enemy_on_cooldown_second_step_is_cover_fire() {
    let mut snap = base_snapshot();
    snap.me.cooldowns.insert("throw:smoke".to_string(), 5.0);
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);
    match &plan.steps[1] {
        ActionStep::CoverFire {
            target_id,
            duration,
        } => {
            assert_eq!(*target_id, 0);
            assert_eq!(*duration, 1.5); // shorter when on cooldown
        }
        other => panic!("Expected CoverFire, got {:?}", other),
    }
}

#[test]
fn rule_orch_no_enemies_returns_empty_steps() {
    let snap = empty_snapshot();
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);
    assert!(plan.steps.is_empty());
}

#[test]
fn rule_orch_no_enemies_still_has_plan_id() {
    let mut snap = empty_snapshot();
    snap.t = 2.5;
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&snap);
    assert_eq!(plan.plan_id, "plan-2500");
    assert!(plan.steps.is_empty());
}

// ════════════════════════════════════════════════════════════════════
//  SECTION 4 — Async WebSocket integration tests
// ════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn ws_roundtrip_basic_snapshot() {
    let port = free_port();
    let addr = format!("127.0.0.1:{}", port);

    // Start server
    let server_addr = addr.clone();
    let _server = tokio::spawn(async move {
        let _ = astraweave_ipc::run_ws_server(&server_addr).await;
    });

    // Give server a moment to bind
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let snap = base_snapshot();
    let url = format!("ws://{}", addr);
    let plan = astraweave_ipc::ws_client_roundtrip(&url, &snap)
        .await
        .expect("roundtrip should succeed");

    // The server uses RuleOrchestrator internally
    assert_eq!(plan.plan_id, "plan-0");
    assert_eq!(plan.steps.len(), 3); // enemy present, no cooldown
}

#[tokio::test]
async fn ws_roundtrip_verifies_throw_smoke_step() {
    let port = free_port();
    let addr = format!("127.0.0.1:{}", port);
    let server_addr = addr.clone();
    let _server = tokio::spawn(async move {
        let _ = astraweave_ipc::run_ws_server(&server_addr).await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let snap = base_snapshot();
    let url = format!("ws://{}", addr);
    let plan = astraweave_ipc::ws_client_roundtrip(&url, &snap)
        .await
        .unwrap();

    match &plan.steps[0] {
        ActionStep::Throw { item, x, y } => {
            assert_eq!(item, "smoke");
            assert_eq!(*x, 6);
            assert_eq!(*y, 6);
        }
        other => panic!("Expected Throw step, got {:?}", other),
    }
}

#[tokio::test]
async fn ws_roundtrip_empty_snapshot_returns_empty_plan() {
    let port = free_port();
    let addr = format!("127.0.0.1:{}", port);
    let server_addr = addr.clone();
    let _server = tokio::spawn(async move {
        let _ = astraweave_ipc::run_ws_server(&server_addr).await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let snap = empty_snapshot();
    let url = format!("ws://{}", addr);
    let plan = astraweave_ipc::ws_client_roundtrip(&url, &snap)
        .await
        .unwrap();

    assert!(plan.steps.is_empty());
    assert_eq!(plan.plan_id, "plan-0");
}

#[tokio::test]
async fn ws_roundtrip_with_cooldown_returns_2_steps() {
    let port = free_port();
    let addr = format!("127.0.0.1:{}", port);
    let server_addr = addr.clone();
    let _server = tokio::spawn(async move {
        let _ = astraweave_ipc::run_ws_server(&server_addr).await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let mut snap = base_snapshot();
    snap.me.cooldowns.insert("throw:smoke".to_string(), 3.0);
    let url = format!("ws://{}", addr);
    let plan = astraweave_ipc::ws_client_roundtrip(&url, &snap)
        .await
        .unwrap();

    assert_eq!(plan.steps.len(), 2);
    match &plan.steps[1] {
        ActionStep::CoverFire { duration, .. } => {
            assert_eq!(*duration, 1.5);
        }
        other => panic!("Expected CoverFire, got {:?}", other),
    }
}

#[tokio::test]
async fn ws_roundtrip_preserves_time_in_plan_id() {
    let port = free_port();
    let addr = format!("127.0.0.1:{}", port);
    let server_addr = addr.clone();
    let _server = tokio::spawn(async move {
        let _ = astraweave_ipc::run_ws_server(&server_addr).await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let mut snap = base_snapshot();
    snap.t = 7.777;
    let url = format!("ws://{}", addr);
    let plan = astraweave_ipc::ws_client_roundtrip(&url, &snap)
        .await
        .unwrap();

    assert_eq!(plan.plan_id, "plan-7777");
}

#[tokio::test]
async fn ws_roundtrip_multiple_enemies_still_uses_first() {
    let port = free_port();
    let addr = format!("127.0.0.1:{}", port);
    let server_addr = addr.clone();
    let _server = tokio::spawn(async move {
        let _ = astraweave_ipc::run_ws_server(&server_addr).await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let mut snap = base_snapshot();
    snap.enemies.push(EnemyState {
        id: 1,
        pos: IVec2::new(20, 20),
        hp: 75,
        cover: CoverType::Wall,
        last_seen: 1.0,
    });
    let url = format!("ws://{}", addr);
    let plan = astraweave_ipc::ws_client_roundtrip(&url, &snap)
        .await
        .unwrap();

    // Orchestrator only uses first enemy
    assert_eq!(plan.steps.len(), 3);
    match &plan.steps[2] {
        ActionStep::CoverFire { target_id, .. } => {
            assert_eq!(*target_id, 0); // first enemy's id
        }
        other => panic!("Expected CoverFire, got {:?}", other),
    }
}

#[tokio::test]
async fn ws_client_roundtrip_bad_address_returns_error() {
    // Connecting to a port with no server should fail
    let result = astraweave_ipc::ws_client_roundtrip("ws://127.0.0.1:1", &empty_snapshot()).await;
    assert!(result.is_err());
}

// ════════════════════════════════════════════════════════════════════
//  SECTION 5 — Snapshot edge cases (mutation killers)
// ════════════════════════════════════════════════════════════════════

#[test]
fn snapshot_with_cooldowns_roundtrip() {
    let mut snap = base_snapshot();
    snap.me.cooldowns.insert("attack".to_string(), 2.5);
    snap.me.cooldowns.insert("heal".to_string(), 0.0);

    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.me.cooldowns.len(), 2);
    assert_eq!(rt.me.cooldowns["attack"], 2.5);
    assert_eq!(rt.me.cooldowns["heal"], 0.0);
}

#[test]
fn snapshot_with_orders_roundtrip() {
    let mut snap = base_snapshot();
    snap.player.orders = vec!["advance".to_string(), "hold".to_string()];

    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.player.orders.len(), 2);
    assert_eq!(rt.player.orders[0], "advance");
    assert_eq!(rt.player.orders[1], "hold");
}

#[test]
fn snapshot_large_t_roundtrip() {
    let mut snap = base_snapshot();
    snap.t = 99999.999;

    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert!((rt.t - 99999.999).abs() < 0.01);
}

#[test]
fn snapshot_negative_coords_roundtrip() {
    let mut snap = base_snapshot();
    snap.player.pos = IVec2::new(-10, -20);
    snap.me.pos = IVec2::new(-5, -5);

    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.player.pos.x, -10);
    assert_eq!(rt.player.pos.y, -20);
    assert_eq!(rt.me.pos.x, -5);
    assert_eq!(rt.me.pos.y, -5);
}

#[test]
fn snapshot_zero_hp_roundtrip() {
    let mut snap = base_snapshot();
    snap.player.hp = 0;
    snap.enemies[0].hp = 0;

    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.player.hp, 0);
    assert_eq!(rt.enemies[0].hp, 0);
}

#[test]
fn snapshot_max_morale_roundtrip() {
    let mut snap = base_snapshot();
    snap.me.morale = f32::MAX;

    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.me.morale, f32::MAX);
}

#[test]
fn snapshot_zero_morale_roundtrip() {
    let mut snap = base_snapshot();
    snap.me.morale = 0.0;

    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.me.morale, 0.0);
}

#[test]
fn snapshot_many_obstacles_roundtrip() {
    let mut snap = base_snapshot();
    snap.obstacles = (0..100).map(|i| IVec2::new(i, i * 2)).collect();

    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.obstacles.len(), 100);
    assert_eq!(rt.obstacles[50].x, 50);
    assert_eq!(rt.obstacles[50].y, 100);
}

#[test]
fn snapshot_many_enemies_roundtrip() {
    let mut snap = base_snapshot();
    snap.enemies = (0..50)
        .map(|i| EnemyState {
            id: i as u32,
            pos: IVec2::new(i * 3, i * 3),
            hp: 100 - i,
            cover: if i % 2 == 0 {
                CoverType::None
            } else {
                CoverType::Wall
            },
            last_seen: i as f32,
        })
        .collect();

    let json = serde_json::to_string(&snap).unwrap();
    let rt: WorldSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.enemies.len(), 50);
    assert_eq!(rt.enemies[49].id, 49);
    assert_eq!(rt.enemies[49].hp, 51);
    assert_eq!(rt.enemies[49].cover, CoverType::Wall);
}

// ════════════════════════════════════════════════════════════════════
//  SECTION 6 — Plan through full JSON pipeline (simulate IPC path)
// ════════════════════════════════════════════════════════════════════

/// Simulates the exact path the server takes:
/// JSON → WorldSnapshot → RuleOrchestrator → PlanIntent → JSON → PlanIntent
#[test]
fn full_pipeline_snapshot_to_plan_json_roundtrip() {
    let snap = base_snapshot();
    let snap_json = serde_json::to_string(&snap).unwrap();

    // Server side: deserialize, plan, serialize
    let server_snap: WorldSnapshot = serde_json::from_str(&snap_json).unwrap();
    let orch = RuleOrchestrator;
    let plan = orch.propose_plan(&server_snap);
    let plan_json = serde_json::to_string(&plan).unwrap();

    // Client side: deserialize
    let client_plan: PlanIntent = serde_json::from_str(&plan_json).unwrap();
    assert_eq!(client_plan.plan_id, "plan-0");
    assert_eq!(client_plan.steps.len(), 3);
}

#[test]
fn full_pipeline_cooldown_snapshot() {
    let mut snap = base_snapshot();
    snap.me.cooldowns.insert("throw:smoke".to_string(), 1.0);
    snap.t = 3.0;

    let snap_json = serde_json::to_string(&snap).unwrap();
    let server_snap: WorldSnapshot = serde_json::from_str(&snap_json).unwrap();
    let plan = RuleOrchestrator.propose_plan(&server_snap);
    let plan_json = serde_json::to_string(&plan).unwrap();
    let client_plan: PlanIntent = serde_json::from_str(&plan_json).unwrap();

    assert_eq!(client_plan.plan_id, "plan-3000");
    assert_eq!(client_plan.steps.len(), 2);
}

#[test]
fn full_pipeline_empty_snapshot() {
    let snap = empty_snapshot();
    let snap_json = serde_json::to_string(&snap).unwrap();
    let server_snap: WorldSnapshot = serde_json::from_str(&snap_json).unwrap();
    let plan = RuleOrchestrator.propose_plan(&server_snap);
    let plan_json = serde_json::to_string(&plan).unwrap();
    let client_plan: PlanIntent = serde_json::from_str(&plan_json).unwrap();

    assert!(client_plan.steps.is_empty());
}

// ════════════════════════════════════════════════════════════════════
//  SECTION 7 — Orchestrator boundary/mutation-killer assertions
// ════════════════════════════════════════════════════════════════════

#[test]
fn rule_orch_smoke_midpoint_calculation_x() {
    // companion at (0,0), enemy at (10,0) → midpoint x = 5
    let mut snap = empty_snapshot();
    snap.me.pos = IVec2::new(0, 0);
    snap.enemies.push(EnemyState {
        id: 0,
        pos: IVec2::new(10, 0),
        hp: 50,
        cover: CoverType::None,
        last_seen: 0.0,
    });
    let plan = RuleOrchestrator.propose_plan(&snap);
    match &plan.steps[0] {
        ActionStep::Throw { x, .. } => assert_eq!(*x, 5),
        other => panic!("Expected Throw, got {:?}", other),
    }
}

#[test]
fn rule_orch_smoke_midpoint_calculation_y() {
    // companion at (0,0), enemy at (0,8) → midpoint y = 4
    let mut snap = empty_snapshot();
    snap.me.pos = IVec2::new(0, 0);
    snap.enemies.push(EnemyState {
        id: 0,
        pos: IVec2::new(0, 8),
        hp: 50,
        cover: CoverType::None,
        last_seen: 0.0,
    });
    let plan = RuleOrchestrator.propose_plan(&snap);
    match &plan.steps[0] {
        ActionStep::Throw { y, .. } => assert_eq!(*y, 4),
        other => panic!("Expected Throw, got {:?}", other),
    }
}

#[test]
fn rule_orch_move_direction_positive() {
    // companion at (0,0), enemy at (10,10) → move (0+1*2, 0+1*2) = (2,2)
    let mut snap = empty_snapshot();
    snap.me.pos = IVec2::new(0, 0);
    snap.enemies.push(EnemyState {
        id: 0,
        pos: IVec2::new(10, 10),
        hp: 50,
        cover: CoverType::None,
        last_seen: 0.0,
    });
    let plan = RuleOrchestrator.propose_plan(&snap);
    match &plan.steps[1] {
        ActionStep::MoveTo { x, y, .. } => {
            assert_eq!(*x, 2);
            assert_eq!(*y, 2);
        }
        other => panic!("Expected MoveTo, got {:?}", other),
    }
}

#[test]
fn rule_orch_move_direction_negative() {
    // companion at (10,10), enemy at (0,0) → direction (-1,-1), *2=(-2,-2), move to (8,8)
    let mut snap = empty_snapshot();
    snap.me.pos = IVec2::new(10, 10);
    snap.enemies.push(EnemyState {
        id: 0,
        pos: IVec2::new(0, 0),
        hp: 50,
        cover: CoverType::None,
        last_seen: 0.0,
    });
    let plan = RuleOrchestrator.propose_plan(&snap);
    match &plan.steps[1] {
        ActionStep::MoveTo { x, y, .. } => {
            assert_eq!(*x, 8);
            assert_eq!(*y, 8);
        }
        other => panic!("Expected MoveTo, got {:?}", other),
    }
}

#[test]
fn rule_orch_cooldown_zero_treated_as_ready() {
    let mut snap = base_snapshot();
    snap.me.cooldowns.insert("throw:smoke".to_string(), 0.0);
    let plan = RuleOrchestrator.propose_plan(&snap);
    // 0.0 <= 0.0 is true → smoke available → 3 steps
    assert_eq!(plan.steps.len(), 3);
}

#[test]
fn rule_orch_cooldown_tiny_positive_treated_as_not_ready() {
    let mut snap = base_snapshot();
    snap.me.cooldowns.insert("throw:smoke".to_string(), 0.001);
    let plan = RuleOrchestrator.propose_plan(&snap);
    // 0.001 > 0.0 → smoke not available → 2 steps
    assert_eq!(plan.steps.len(), 2);
}

#[test]
fn rule_orch_cover_fire_duration_no_cooldown_is_2_5() {
    let snap = base_snapshot();
    let plan = RuleOrchestrator.propose_plan(&snap);
    match &plan.steps[2] {
        ActionStep::CoverFire { duration, .. } => {
            assert_eq!(*duration, 2.5);
        }
        other => panic!("Expected CoverFire, got {:?}", other),
    }
}

#[test]
fn rule_orch_cover_fire_duration_with_cooldown_is_1_5() {
    let mut snap = base_snapshot();
    snap.me.cooldowns.insert("throw:smoke".to_string(), 10.0);
    let plan = RuleOrchestrator.propose_plan(&snap);
    match &plan.steps[1] {
        ActionStep::CoverFire { duration, .. } => {
            assert_eq!(*duration, 1.5);
        }
        other => panic!("Expected CoverFire, got {:?}", other),
    }
}

#[test]
fn rule_orch_deterministic_same_input_same_output() {
    let snap = base_snapshot();
    let orch = RuleOrchestrator;
    let plan1 = orch.propose_plan(&snap);
    let plan2 = orch.propose_plan(&snap);
    assert_eq!(plan1.plan_id, plan2.plan_id);
    assert_eq!(plan1.steps.len(), plan2.steps.len());
}

#[test]
fn rule_orch_same_position_enemy_midpoint_is_self() {
    // companion at (5,5), enemy at (5,5) → midpoint (5,5), direction (0,0)
    let mut snap = empty_snapshot();
    snap.me.pos = IVec2::new(5, 5);
    snap.enemies.push(EnemyState {
        id: 0,
        pos: IVec2::new(5, 5),
        hp: 50,
        cover: CoverType::None,
        last_seen: 0.0,
    });
    let plan = RuleOrchestrator.propose_plan(&snap);
    match &plan.steps[0] {
        ActionStep::Throw { x, y, .. } => {
            assert_eq!(*x, 5);
            assert_eq!(*y, 5);
        }
        other => panic!("Expected Throw, got {:?}", other),
    }
    match &plan.steps[1] {
        ActionStep::MoveTo { x, y, .. } => {
            // signum(0) = 0, *2 = 0, + 5 = 5 → stays in place
            assert_eq!(*x, 5);
            assert_eq!(*y, 5);
        }
        other => panic!("Expected MoveTo, got {:?}", other),
    }
}
