use astraweave_ai::{
    GoapOrchestrator, LlmOrchestrator, OrchestratorAsync, RuleOrchestrator, UtilityOrchestrator,
};
use astraweave_core::{
    default_tool_registry, map_engine_error, validation::validate_and_execute,
    validation::ValidateCfg, Entity, IVec2, PlanIntent, World, WorldSnapshot,
};
use astraweave_llm::MockLlm;

fn mk_snap_from_world(w: &World, me: Entity, player: Entity, enemy: Entity) -> WorldSnapshot {
    use std::collections::BTreeMap;
    let me_pose = w.pos_of(me).unwrap();
    let player_pose = w.pos_of(player).unwrap();
    let enemy_pose = w.pos_of(enemy).unwrap();
    astraweave_core::WorldSnapshot {
        t: w.t,
        player: astraweave_core::PlayerState {
            hp: w.health(player).unwrap().hp,
            pos: player_pose,
            stance: "stand".into(),
            orders: vec![],
        },
        me: astraweave_core::CompanionState {
            ammo: w.ammo(me).unwrap().rounds,
            cooldowns: BTreeMap::new(),
            morale: 0.5,
            pos: me_pose,
        },
        enemies: vec![astraweave_core::EnemyState {
            id: enemy,
            pos: enemy_pose,
            hp: w.health(enemy).unwrap().hp,
            cover: "low".into(),
            last_seen: 0.0,
        }],
        pois: vec![],
        obstacles: vec![],
        objective: None,
    }
}

fn main() {
    pollster::block_on(async_main());
}

async fn async_main() {
    // Minimal tick loop exercising OrchestratorAsync and tool-block mapping
    let mut w = World::new();
    let player = w.spawn(
        "player",
        IVec2 { x: 0, y: 0 },
        astraweave_core::Team { id: 0 },
        100,
        0,
    );
    let me = w.spawn(
        "companion",
        IVec2 { x: 0, y: 1 },
        astraweave_core::Team { id: 1 },
        100,
        5,
    );
    let enemy = w.spawn(
        "enemy",
        IVec2 { x: 6, y: 1 },
        astraweave_core::Team { id: 2 },
        50,
        0,
    );

    let cfg = ValidateCfg {
        world_bounds: (-20, -20, 20, 20),
    };

    let rule = RuleOrchestrator;
    let util = UtilityOrchestrator;
    let goap = GoapOrchestrator;

    // Optional LLM orch (mock)
    let llm = LlmOrchestrator::new(MockLlm, Some(default_tool_registry()));

    for tick in 0..5 {
        w.tick(0.1);
        let snap = mk_snap_from_world(&w, me, player, enemy);
        // Choose orchestrator per tick
        let plan = match tick % 4 {
            0 => rule.plan(snap.clone(), 2).await.expect("rule.plan failed"),
            1 => util.plan(snap.clone(), 2).await.expect("util.plan failed"),
            2 => goap.plan(snap.clone(), 2).await.expect("goap.plan failed"),
            _ => llm.plan(snap.clone(), 10).await.expect("llm.plan failed"),
        };
        println!("tick {}: {} steps", tick, plan.steps.len());
        if let Err(e) = run_plan_with_logging(&mut w, me, &plan, &cfg) {
            let tb = map_engine_error(plan.steps.first().unwrap(), &e);
            println!("  blocked: {:?} tool={} msg={}", tb.reason, tb.tool, tb.msg);
        }
    }
}

fn run_plan_with_logging(
    w: &mut World,
    actor: Entity,
    intent: &PlanIntent,
    cfg: &ValidateCfg,
) -> Result<(), astraweave_core::EngineError> {
    validate_and_execute(w, actor, intent, cfg, &mut |s| println!("{}", s))
}
