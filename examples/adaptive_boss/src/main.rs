use astraweave_core::*;
#[cfg(not(feature = "veilweaver_slice"))]
use astraweave_director::BossDirector;
#[cfg(feature = "veilweaver_slice")]
use astraweave_director::OathboundWardenDirector;

fn main() -> anyhow::Result<()> {
    let mut w = World::new();
    // simple arena
    let player = w.spawn("Player", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);
    let comp = w.spawn("Comp", IVec2 { x: 3, y: 2 }, Team { id: 1 }, 80, 30);
    let boss = w.spawn("Boss", IVec2 { x: 14, y: 2 }, Team { id: 2 }, 400, 0);
    // snapshot for director (pretend boss sees one enemy)
    let snap = WorldSnapshot {
        t: w.t,
        player: PlayerState {
            hp: 100,
            pos: w.pos_of(player).unwrap(),
            stance: "stand".into(),
            orders: vec![],
        },
        me: CompanionState {
            ammo: 30,
            cooldowns: Default::default(),
            morale: 0.8,
            pos: w.pos_of(comp).unwrap(),
        },
        enemies: vec![EnemyState {
            id: boss,
            pos: w.pos_of(boss).unwrap(),
            hp: 400,
            cover: "high".into(),
            last_seen: w.t,
        }],
        pois: vec![],
        obstacles: vec![],
        objective: Some("defeat_boss".into()),
    
        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    };

    let mut budget = DirectorBudget {
        traps: 2,
        terrain_edits: 2,
        spawns: 2,
    };

    #[cfg(feature = "veilweaver_slice")]
    {
        let mut director = OathboundWardenDirector::new();
        let directive = director.step(&snap, &budget);
        println!("Warden phase: {:?}", directive.phase);
        if !directive.telegraphs.is_empty() {
            println!("Telegraphs: {:?}", directive.telegraphs);
        }
        let mut log = |s: String| println!("{}", s);
        println!(
            "Director plan: {}",
            serde_json::to_string_pretty(&directive.plan)?
        );
        apply_director_plan(&mut w, &mut budget, &directive.plan, &mut log);
    }

    #[cfg(not(feature = "veilweaver_slice"))]
    {
        let director = BossDirector;
        let plan = director.plan(&snap, &budget);
        let mut log = |s: String| println!("{}", s);
        println!("Director plan: {}", serde_json::to_string_pretty(&plan)?);
        apply_director_plan(&mut w, &mut budget, &plan, &mut log);
    }

    println!(
        "Remaining budget: traps={}, terrain_edits={}, spawns={}",
        budget.traps, budget.terrain_edits, budget.spawns
    );
    Ok(())
}
