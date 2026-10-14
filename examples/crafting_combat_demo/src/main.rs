use astraweave_gameplay::*;
use glam::vec3;

fn main() -> anyhow::Result<()> {
    // Inventory + harvesting
    let mut inv = Inventory::default();
    let mut node = harvesting::ResourceNode {
        kind: ResourceKind::Crystal,
        pos: vec3(0.0, 0.0, 0.0),
        amount: 3,
        respawn_time: 30.0,
        timer: 0.0,
    };
    node.harvest(&mut inv, 3);
    inv.add_resource(ResourceKind::Ore, 3);
    inv.add_resource(ResourceKind::Essence, 2);

    println!("Inventory resources: {:?}", inv.resources);

    // Crafting recipes (inline sample). You can load TOML (assets/recipes.toml) similarly.
    let book = RecipeBook {
        recipes: vec![
            CraftRecipe {
                name: "Echo Blade".into(),
                output_item: ItemKind::Weapon {
                    base_damage: 14,
                    dtype: DamageType::Physical,
                },
                costs: vec![
                    CraftCost {
                        kind: ResourceKind::Ore,
                        count: 3,
                    },
                    CraftCost {
                        kind: ResourceKind::Crystal,
                        count: 2,
                    },
                ],
            },
            CraftRecipe {
                name: "Health Tonic".into(),
                output_item: ItemKind::Consumable { heal: 30 },
                costs: vec![
                    CraftCost {
                        kind: ResourceKind::Essence,
                        count: 1,
                    },
                    CraftCost {
                        kind: ResourceKind::Crystal,
                        count: 1,
                    },
                ],
            },
        ],
    };

    // Craft blade from an explicit sim stream (world seed 1, tick 0).
    let mut blade = book
        .craft_with_seed("Echo Blade", &mut inv, 1, 0)
        .expect("craft echo blade");
    println!("Crafted: {}", blade.name);

    // Infuse echo mod
    let echo = EchoMod {
        name: "Wind‑sung".into(),
        power_mult: 1.35,
        dtype_override: Some(DamageType::Echo),
        special: None,
    };
    items::infuse(&mut blade, echo);
    println!("Infused: {:?}", blade.echo.as_ref().unwrap().name);

    // Combat combo vs. dummy target
    let chain = ComboChain {
        name: "L‑L‑H".into(),
        steps: vec![
            ComboStep {
                kind: AttackKind::Light,
                window: (0.0, 0.5),
                damage: 6,
                reach: 1.8,
                stagger: 0.3,
            },
            ComboStep {
                kind: AttackKind::Light,
                window: (0.1, 0.6),
                damage: 7,
                reach: 1.9,
                stagger: 0.3,
            },
            ComboStep {
                kind: AttackKind::Heavy,
                window: (0.2, 0.8),
                damage: 12,
                reach: 2.0,
                stagger: 0.5,
            },
        ],
    };
    let mut atk = AttackState::new(chain);
    let mut player = Stats::new(120);
    player.power = 8;
    let mut enemy = Stats::new(100);
    enemy.defense = 4;

    let player_pos = vec3(0.0, 0.0, 0.0);
    let enemy_pos = vec3(1.2, 0.0, 0.0);

    atk.start();
    let dt = 0.16;

    // step 1: press Light
    let (hit1, dmg1) = atk.tick(
        dt,
        true,
        false,
        player_pos,
        enemy_pos,
        &player,
        Some(&blade),
        &mut enemy,
    );
    // step 2: press Light
    let (hit2, dmg2) = atk.tick(
        0.2,
        true,
        false,
        player_pos,
        enemy_pos,
        &player,
        Some(&blade),
        &mut enemy,
    );
    // step 3: press Heavy
    let (hit3, dmg3) = atk.tick(
        0.25,
        false,
        true,
        player_pos,
        enemy_pos,
        &player,
        Some(&blade),
        &mut enemy,
    );

    println!(
        "Combo hits: {:?}, damage total: {}",
        [hit1, hit2, hit3],
        dmg1 + dmg2 + dmg3
    );
    println!("Enemy HP after: {}", enemy.hp);

    Ok(())
}
