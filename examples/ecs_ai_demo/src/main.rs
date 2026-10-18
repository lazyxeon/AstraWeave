//! ECS AI Demo: Demonstrates AI planning in an ECS context
use astraweave_ai::AiPlanningPlugin;
use astraweave_core::{CAmmo, CCooldowns, CDesiredPos, CHealth, CPos, CTeam, IVec2};
use astraweave_ecs as ecs;

fn main() -> anyhow::Result<()> {
    println!("=== ECS AI Demo ===");

    // Create ECS app with AI plugin and movement systems
    let mut app = ecs::App::new().add_plugin(AiPlanningPlugin);
    app.world
        .insert_resource(astraweave_ai::SpatialTick::demo_agent());

    // Add movement system (from ecs_adapter)
    app.add_system_known_stage("simulation", move_system as ecs::SystemFn);

    // Spawn entities
    let player = app.world.spawn();
    app.world.insert(
        player,
        CPos {
            pos: IVec2 { x: 0, y: 0 },
        },
    );
    app.world.insert(player, CHealth { hp: 100 });
    app.world.insert(player, CTeam { id: 0 });

    let companion = app.world.spawn();
    app.world.insert(
        companion,
        CPos {
            pos: IVec2 { x: 2, y: 2 },
        },
    );
    app.world.insert(companion, CHealth { hp: 80 });
    app.world.insert(companion, CTeam { id: 1 });
    app.world.insert(companion, CAmmo { rounds: 10 });
    app.world.insert(
        companion,
        CCooldowns {
            map: std::collections::BTreeMap::new(),
        },
    );

    let enemy = app.world.spawn();
    app.world.insert(
        enemy,
        CPos {
            pos: IVec2 { x: 5, y: 0 },
        },
    );
    app.world.insert(enemy, CHealth { hp: 60 });
    app.world.insert(enemy, CTeam { id: 2 });

    println!("Initial state:");
    print_entities(&app.world);

    // Run AI planning for a few ticks
    for tick in 1..=10 {
        println!("\n--- Tick {} ---", tick);
        app = app.run_fixed(1);

        print_entities(&app.world);
    }

    println!("\n=== Demo Complete ===");
    Ok(())
}

fn move_system(world: &mut ecs::World) {
    // Move entities one step toward desired pos (cardinal-only 4-neighborhood)
    // Deterministic order by BTreeMap underlying storage
    use std::collections::BTreeMap;
    let goals: BTreeMap<ecs::Entity, CDesiredPos> = {
        let mut m = BTreeMap::new();
        let q = ecs::Query::<CDesiredPos>::new(&*world);
        for (e, g) in q {
            m.insert(e, *g);
        }
        m
    };
    world.each_mut::<CPos>(|e, p| {
        if let Some(goal) = goals.get(&e) {
            let dx = (goal.pos.x - p.pos.x).signum();
            let mut dy = (goal.pos.y - p.pos.y).signum();
            // Cardinal-only behavior: prefer moving along X this tick; if we move in X,
            // do not also move in Y (prevents diagonal movement).
            if dx != 0 {
                dy = 0;
            }
            if dx != 0 || dy != 0 {
                if dx != 0 {
                    p.pos.x += dx;
                } else if dy != 0 {
                    p.pos.y += dy;
                }
            }
        }
    });
}

fn print_entities(world: &ecs::World) {
    println!("Entities:");
    for (entity, pos) in ecs::Query::<CPos>::new(world) {
        let team = world.get::<CTeam>(entity).map(|t| t.id).unwrap_or(255);
        let hp = world.get::<CHealth>(entity).map(|h| h.hp).unwrap_or(0);
        println!(
            "  Entity {:?}: Team {}, Pos {:?}, HP {}",
            entity.id(),
            team,
            pos.pos,
            hp
        );
    }
}
