#![forbid(unsafe_code)]
use anyhow::Result;
use astraweave_ai::{Orchestrator, RuleOrchestrator};
use astraweave_core::{PlanIntent, WorldSnapshot};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

pub async fn run_ws_server(addr: &str) -> Result<()> {
    use tokio::net::TcpListener;
    let listener = TcpListener::bind(addr).await?;
    println!("Companion WS server listening on {}", addr);
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_conn(stream));
    }
    Ok(())
}

async fn handle_conn(stream: tokio::net::TcpStream) -> Result<()> {
    let ws = tokio_tungstenite::accept_async(stream).await?;
    let (mut tx, mut rx) = ws.split();
    let orch = RuleOrchestrator;

    while let Some(msg) = rx.next().await {
        let msg = msg?;
        if msg.is_text() {
            let txt = msg.into_text()?;
            let snap: WorldSnapshot = serde_json::from_str(&txt)?;
            let plan: PlanIntent = orch.propose_plan(&snap);
            let out = serde_json::to_string(&plan)?;
            tx.send(Message::Text(out.into())).await?;
        } else if msg.is_close() {
            break;
        }
    }
    Ok(())
}

pub async fn ws_client_roundtrip(addr: &str, snapshot: &WorldSnapshot) -> Result<PlanIntent> {
    let (ws, _) = tokio_tungstenite::connect_async(addr).await?;
    let (mut tx, mut rx) = ws.split();
    let js = serde_json::to_string(snapshot)?;
    tx.send(Message::Text(js.into())).await?;
    if let Some(msg) = rx.next().await {
        let msg = msg?;
        let txt = msg.into_text()?;
        let plan: PlanIntent = serde_json::from_str(&txt)?;
        return Ok(plan);
    }
    anyhow::bail!("no response")
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::schema::{CompanionState, CoverType, EnemyState, IVec2, PlayerState, Poi, Stance};
    use std::collections::BTreeMap;

    fn create_test_snapshot() -> WorldSnapshot {
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

    #[test]
    fn test_snapshot_serialization() {
        let snap = create_test_snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("extraction point"));

        let deserialized: WorldSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.t, snap.t);
        assert_eq!(deserialized.player.hp, 100);
        assert_eq!(deserialized.me.ammo, 30);
    }

    #[test]
    fn test_plan_intent_serialization() {
        use astraweave_core::ActionStep;

        let plan = PlanIntent {
            plan_id: "test-plan-001".to_string(),
            steps: vec![
                ActionStep::MoveTo {
                    x: 10,
                    y: 10,
                    speed: None,
                },
                ActionStep::Attack { target_id: 0 },
            ],
        };

        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("test-plan-001"));

        let deserialized: PlanIntent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.plan_id, plan.plan_id);
        assert_eq!(deserialized.steps.len(), 2);
    }

    #[test]
    fn test_rule_orchestrator_basic() {
        let snap = create_test_snapshot();
        let orch = RuleOrchestrator;
        let plan = orch.propose_plan(&snap);

        // RuleOrchestrator should produce a valid plan
        assert!(!plan.plan_id.is_empty());
    }

    #[test]
    fn test_empty_snapshot() {
        let snap = WorldSnapshot {
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
        };

        let json = serde_json::to_string(&snap).unwrap();
        let deserialized: WorldSnapshot = serde_json::from_str(&json).unwrap();
        assert!(deserialized.enemies.is_empty());
        assert!(deserialized.objective.is_none());
    }

    #[test]
    fn test_multiple_enemies() {
        let mut snap = create_test_snapshot();
        snap.enemies = (0..10)
            .map(|i| EnemyState {
                id: i as u32,
                pos: IVec2::new(i * 2, i * 2),
                hp: 50,
                cover: if i % 2 == 0 {
                    CoverType::None
                } else {
                    CoverType::Wall
                },
                last_seen: i as f32 * 0.5,
            })
            .collect();

        let json = serde_json::to_string(&snap).unwrap();
        let deserialized: WorldSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.enemies.len(), 10);
        assert_eq!(deserialized.enemies[0].cover, CoverType::None);
        assert_eq!(deserialized.enemies[1].cover, CoverType::Wall);
    }

    #[test]
    fn test_cooldowns_serialization() {
        let mut snap = create_test_snapshot();
        snap.me.cooldowns.insert("attack".to_string(), 2.5);
        snap.me.cooldowns.insert("heal".to_string(), 5.0);
        snap.me.cooldowns.insert("special".to_string(), 0.0);

        let json = serde_json::to_string(&snap).unwrap();
        let deserialized: WorldSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.me.cooldowns.len(), 3);
        assert_eq!(deserialized.me.cooldowns.get("attack"), Some(&2.5));
        assert_eq!(deserialized.me.cooldowns.get("heal"), Some(&5.0));
    }

    #[test]
    fn test_poi_serialization() {
        let snap = WorldSnapshot {
            t: 1.5,
            player: PlayerState {
                pos: IVec2::new(0, 0),
                hp: 100,
                stance: Stance::Crouch,
                orders: vec!["advance".to_string()],
            },
            me: CompanionState {
                ammo: 50,
                cooldowns: BTreeMap::new(),
                morale: 0.8,
                pos: IVec2::new(2, 2),
            },
            enemies: vec![],
            pois: vec![
                Poi {
                    k: "objective".to_string(),
                    pos: IVec2::new(10, 10),
                },
                Poi {
                    k: "ammo".to_string(),
                    pos: IVec2::new(5, 5),
                },
                Poi {
                    k: "cover".to_string(),
                    pos: IVec2::new(7, 3),
                },
            ],
            obstacles: vec![],
            objective: Some("Collect supplies".to_string()),
        };

        let json = serde_json::to_string(&snap).unwrap();
        let deserialized: WorldSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.pois.len(), 3);
        assert_eq!(deserialized.pois[1].k, "ammo");
    }

    #[test]
    fn test_obstacles_serialization() {
        let snap = WorldSnapshot {
            t: 0.0,
            player: PlayerState {
                pos: IVec2::new(0, 0),
                hp: 100,
                stance: Stance::Stand,
                orders: vec![],
            },
            me: CompanionState {
                ammo: 30,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2::new(1, 1),
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![IVec2::new(3, 3), IVec2::new(4, 4), IVec2::new(5, 5)],
            objective: None,
        };

        let json = serde_json::to_string(&snap).unwrap();
        let deserialized: WorldSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.obstacles.len(), 3);
        assert_eq!(deserialized.obstacles[0].x, 3);
        assert_eq!(deserialized.obstacles[0].y, 3);
    }
}
