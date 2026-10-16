use anyhow::Result;
use astraweave_ai::orchestrator::{
    GoapOrchestrator, Orchestrator, RuleOrchestrator, UtilityOrchestrator,
};
use astraweave_core::{CompanionState, EnemyState, IVec2, PlanIntent, PlayerState, WorldSnapshot};
use std::collections::BTreeMap;

fn fixture_world(smoke_cd: f32, enemy_pos: (i32, i32)) -> WorldSnapshot {
    WorldSnapshot {
        t: 1.0,
        player: PlayerState {
            hp: 90,

            pos: IVec2 { x: 0, y: 0 },
            stance: "stand".into(),
            orders: vec![],
        },
        me: CompanionState {
            ammo: 12,
            cooldowns: BTreeMap::from([("throw:smoke".to_string(), smoke_cd)]),
            morale: 0.8,
            pos: IVec2 { x: 0, y: 0 },
        },
        enemies: vec![EnemyState {
            id: 2,
            pos: IVec2 {
                x: enemy_pos.0,
                y: enemy_pos.1,
            },
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

fn to_json(plan: &PlanIntent) -> Result<String> {
    Ok(serde_json::to_string_pretty(plan)?)
}

#[test]
fn snapshot_rule_prefers_smoke_when_ready() -> Result<()> {
    let snap = fixture_world(0.0, (4, 0));
    let plan = RuleOrchestrator.propose_plan(&snap);
    let json = to_json(&plan)?;
    assert!(
        json.contains("\"Throw\""),
        "expected Throw in plan: {}",
        json
    );
    Ok(())
}

#[test]
fn snapshot_utility_advances_when_smoke_blocked() -> Result<()> {
    let snap = fixture_world(5.0, (3, 0));
    let plan = UtilityOrchestrator.propose_plan(&snap);
    let json = to_json(&plan)?;
    assert!(
        json.contains("\"MoveTo\""),
        "expected MoveTo in plan: {}",
        json
    );
    Ok(())
}

#[test]
fn snapshot_goap_independent_moves_and_covers() -> Result<()> {
    let far = fixture_world(10.0, (5, 0));
    let close = fixture_world(10.0, (1, 0));
    let p1 = GoapOrchestrator.propose_plan(&far);
    let p2 = GoapOrchestrator.propose_plan(&close);
    assert!(
        to_json(&p1)?.contains("\"MoveTo\""),
        "expected plan for far enemy to include MoveTo: {}",
        to_json(&p1)?
    );
    assert!(
        to_json(&p2)?.contains("\"CoverFire\""),
        "expected plan for close enemy to include CoverFire: {}",
        to_json(&p2)?
    );
    Ok(())
}
