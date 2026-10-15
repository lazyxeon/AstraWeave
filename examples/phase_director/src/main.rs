use astraweave_core::*;
use astraweave_director::*;
fn main() -> anyhow::Result<()> {
    let mut w = World::new();
    let player = w.spawn("Player", IVec2 { x: 2, y: 2 }, Team { id: 0 }, 100, 0);
    let comp = w.spawn("Comp", IVec2 { x: 3, y: 2 }, Team { id: 1 }, 80, 30);
    let boss = w.spawn("Boss", IVec2 { x: 14, y: 2 }, Team { id: 2 }, 300, 0);

    let snap = WorldSnapshot {
        t: 0.0,
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
            hp: 300,
            cover: "high".into(),
            last_seen: w.t,
        }],
        pois: vec![],
        obstacles: vec![],
        objective: Some("defeat_boss".into()),
    
        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    };
    let budget = DirectorBudget {
        traps: 2,
        terrain_edits: 3,
        spawns: 2,
    };
    let mut pd = PhaseDirector::new(vec![
        PhaseSpec {
            name: "Dreadwatch".into(),
            hp_threshold: 250,
            terrain_bias: 0.6,
            aggression: 0.3,
        },
        PhaseSpec {
            name: "Lashing Gale".into(),
            hp_threshold: 150,
            terrain_bias: 0.3,
            aggression: 0.6,
        },
        PhaseSpec {
            name: "Terminal Spiral".into(),
            hp_threshold: 50,
            terrain_bias: 0.7,
            aggression: 0.9,
        },
    ]);
    let plan1 = pd.step(&snap, &budget);
    println!("Phase: {}", plan1.phase_name);
    println!("Telegraphs: {:?}", plan1.telegraphs);
    println!("{}", serde_json::to_string_pretty(&plan1.director)?);
    Ok(())
}
