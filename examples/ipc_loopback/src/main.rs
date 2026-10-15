use astraweave_core::{CompanionState, EnemyState, IVec2, PlayerState, WorldSnapshot};
use astraweave_ipc::{run_ws_server, ws_client_roundtrip};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tokio::spawn(async {
        run_ws_server("127.0.0.1:8088").await.unwrap();
    });
    // give server a tick
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let snap = WorldSnapshot {
        t: 1.0,
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 2, y: 2 },
            stance: "stand".into(),
            orders: vec!["hold_east".into()],
        },
        me: CompanionState {
            ammo: 30,
            cooldowns: Default::default(),
            morale: 0.9,
            pos: IVec2 { x: 3, y: 2 },
        },
        enemies: vec![EnemyState {
            id: 99,
            pos: IVec2 { x: 12, y: 2 },
            hp: 60,
            cover: "low".into(),
            last_seen: 1.0,
        }],
        pois: vec![],
        obstacles: vec![],
        objective: Some("extract".into()),

        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    };

    let plan = ws_client_roundtrip("ws://127.0.0.1:8088", &snap).await?;
    println!(
        "Got plan from WS orchestrator: {}",
        serde_json::to_string_pretty(&plan)?
    );
    Ok(())
}
