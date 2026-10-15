use astraweave_core::*;
use astraweave_llm::{plan_from_llm, MockLlm, PlanSource};

fn tool_spec(name: &str, args: &[(&str, &str)]) -> ToolSpec {
    ToolSpec {
        name: name.into(),
        args: args
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Tiny snapshot
    let snap = WorldSnapshot {
        t: 1.0,
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 2, y: 2 },
            stance: "stand".into(),
            orders: vec![],
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
    let reg = ToolRegistry {
        tools: vec![
            tool_spec("move_to", &[("x", "i32"), ("y", "i32")]),
            tool_spec(
                "throw",
                &[("item", "enum[smoke,grenade]"), ("x", "i32"), ("y", "i32")],
            ),
            tool_spec("cover_fire", &[("target_id", "u32"), ("duration", "f32")]),
        ],
        constraints: Constraints {
            enforce_cooldowns: true,
            enforce_los: true,
            enforce_stamina: true,
        },
    };
    let client = MockLlm;
    let plan_source = plan_from_llm(&client, &snap, &reg).await;
    let plan = match plan_source {
        PlanSource::Llm(p) => p,
        PlanSource::Fallback { plan: p, reason } => {
            eprintln!("Fell back to heuristic: {}", reason);
            p
        }
        _ => {
            eprintln!("Unknown plan source");
            return Ok(());
        }
    };
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}
