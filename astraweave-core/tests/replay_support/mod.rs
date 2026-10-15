#![allow(dead_code)]
//! Canonical replay encoding for the determinism gate.
//!
//! Bytes are little-endian integers, length-prefixed UTF-8 strings, and
//! `f32::to_bits`. This module does not sort. Production perception must
//! already emit entity ids and obstacles in a stable order; sorting here
//! would hide an unsorted `HashMap` walk.

use astraweave_ai::{GoapOrchestrator, Orchestrator, RuleOrchestrator};
use astraweave_core::{
    build_snapshot, ActionStep, CompanionState, EnemyState, IVec2, MovementSpeed, PerceptionConfig,
    PlanIntent, PlayerState, Team, World, WorldSnapshot,
};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

pub fn replay_hash_json_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir).join("replay-hash.json");
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/replay-hash.json")
}

pub fn planner_pure_bytes() -> Vec<u8> {
    let mut out = Vec::new();
    for (label, snap) in planner_pure_snapshots() {
        push_str(&mut out, label);
        let rule = RuleOrchestrator.propose_plan(&snap);
        let goap = GoapOrchestrator.propose_plan(&snap);
        push_str(&mut out, "rule");
        encode_plan(&mut out, &rule);
        push_str(&mut out, "goap");
        encode_plan(&mut out, &goap);
    }
    out
}

pub fn nav_reroute_bytes() -> Vec<u8> {
    use astraweave_ai::htn::{plan_route, AgentPlan, SpatialHooks};
    use astraweave_nav::{bridge_and_ford, nav_change_from_kind, NavChangeKind, NavScratch};

    let mut tiles = bridge_and_ford();
    let mut scratch = NavScratch::new();
    let hooks = SpatialHooks::none();
    let mut out = Vec::new();
    for tick in 0..60u64 {
        if tick == 30 {
            tiles.mark(nav_change_from_kind(
                NavChangeKind::Detach,
                tick,
                0,
                [-1_000, -1_000, 17_000],
                [5_000, 1_000, 31_999],
            ));
        }
        let committed = tiles.commit_tick() as u32;
        let mut budget = 64u32;
        let mut plan = AgentPlan::empty();
        plan_route(
            &tiles,
            &mut scratch,
            [1_000, 0, 1_000],
            [1_000, 0, 46_000],
            &hooks,
            tick,
            &mut plan,
            &mut budget,
        );
        push_u32(&mut out, tick as u32);
        push_u32(&mut out, committed);
        push_u8(&mut out, plan.len);
        for slot in plan.slots.iter().take(plan.len as usize) {
            match slot {
                astraweave_ai::htn::ActionSlot::Empty => push_u8(&mut out, 0),
                astraweave_ai::htn::ActionSlot::MoveTo { x, y } => {
                    push_u8(&mut out, 1);
                    push_i32(&mut out, *x);
                    push_i32(&mut out, *y);
                }
                astraweave_ai::htn::ActionSlot::UseBreach { portal_id } => {
                    push_u8(&mut out, 2);
                    push_u32(&mut out, *portal_id);
                }
                astraweave_ai::htn::ActionSlot::TakeCover { cover_id } => {
                    push_u8(&mut out, 3);
                    push_u32(&mut out, *cover_id);
                }
                astraweave_ai::htn::ActionSlot::Wait { duration_milli } => {
                    push_u8(&mut out, 4);
                    push_u32(&mut out, *duration_milli);
                }
            }
        }
    }
    out
}

/// Seeded harvest respawn and craft item ids. Same `(world_seed, tick)` pair
/// the gameplay crate locks in `gameplay_rng_*`. No `HashMap` walk.
pub fn gameplay_rng_bytes() -> Vec<u8> {
    use astraweave_gameplay::{
        CraftCost, CraftRecipe, Inventory, ItemKind, RecipeBook, ResourceKind, ResourceNode,
    };

    let mut out = Vec::new();
    push_u64(&mut out, 0xA11C);
    for tick in 0..8u64 {
        let mut node = ResourceNode {
            kind: ResourceKind::Wood,
            pos: glam::Vec3::new(10.0, 0.0, 5.0),
            amount: 0,
            respawn_time: 30.0,
            timer: 1.0,
        };
        node.tick_with_seed(1.5, 0xA11C, tick);
        push_u32(&mut out, node.amount);
        push_f32(&mut out, node.timer);
        push_f32(&mut out, node.pos.x);
        push_f32(&mut out, node.pos.y);
        push_f32(&mut out, node.pos.z);
    }

    let book = RecipeBook {
        recipes: vec![CraftRecipe {
            name: "Blade".into(),
            output_item: ItemKind::Consumable { heal: 1 },
            costs: vec![CraftCost {
                kind: ResourceKind::Ore,
                count: 1,
            }],
        }],
    };
    for tick in 0..8u64 {
        let mut inv = Inventory::default();
        inv.add_resource(ResourceKind::Ore, 5);
        match book.craft_with_seed("Blade", &mut inv, 0xA11C, tick) {
            Some(item) => {
                push_u8(&mut out, 1);
                push_u32(&mut out, item.id);
                push_str(&mut out, &item.name);
            }
            None => push_u8(&mut out, 0),
        }
        push_u32(&mut out, inv.resources.len() as u32);
        for (kind, count) in &inv.resources {
            push_u8(&mut out, resource_tag(*kind));
            push_u32(&mut out, *count);
        }
    }
    out
}

fn resource_tag(kind: astraweave_gameplay::ResourceKind) -> u8 {
    match kind {
        astraweave_gameplay::ResourceKind::Wood => 1,
        astraweave_gameplay::ResourceKind::Crystal => 2,
        astraweave_gameplay::ResourceKind::Ore => 3,
        astraweave_gameplay::ResourceKind::Fiber => 4,
        astraweave_gameplay::ResourceKind::Essence => 5,
        _ => 255,
    }
}

pub fn perception_world_bytes() -> Vec<u8> {
    let mut world = World::new();
    let player = world.spawn("player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);
    let companion = world.spawn("companion", IVec2 { x: 2, y: 1 }, Team { id: 1 }, 80, 12);
    let _far = world.spawn("raider", IVec2 { x: 9, y: 4 }, Team { id: 2 }, 40, 0);
    let _near = world.spawn("skirmisher", IVec2 { x: 4, y: 2 }, Team { id: 2 }, 55, 0);
    world.obstacles.insert((9, 1));
    world.obstacles.insert((1, 4));
    world.obstacles.insert((3, 3));
    world.obstacles.insert((1, 2));
    if let Some(cds) = world.cooldowns_mut(companion) {
        cds.map.insert("throw:smoke".to_string(), 1.0);
    }

    let rule = RuleOrchestrator;
    let goap = GoapOrchestrator;
    let cfg = PerceptionConfig { los_max: 6 };
    let mut out = Vec::new();
    for _tick in 0..100u32 {
        world.tick(1.0 / 60.0);
        let enemies = world.enemies_of(1);
        push_u32(&mut out, enemies.len() as u32);
        for id in &enemies {
            push_u32(&mut out, *id);
        }
        let snap = build_snapshot(
            &world,
            player,
            companion,
            &enemies,
            Some("hold".to_string()),
            &cfg,
        );
        encode_snapshot(&mut out, &snap);
        let rule_plan = rule.propose_plan(&snap);
        let goap_plan = goap.propose_plan(&snap);
        encode_plan(&mut out, &rule_plan);
        encode_plan(&mut out, &goap_plan);
        if let Some(IVec2 { x, y }) = first_move(&rule_plan) {
            if let Some(pose) = world.pose_mut(companion) {
                let dx = (x - pose.pos.x).signum();
                let dy = (y - pose.pos.y).signum();
                if dx != 0 {
                    pose.pos.x += dx;
                } else if dy != 0 {
                    pose.pos.y += dy;
                }
            }
        }
    }
    out
}

fn planner_pure_snapshots() -> Vec<(&'static str, WorldSnapshot)> {
    vec![
        ("smoke-ready", hand_snapshot(0.0, 9, true)),
        ("smoke-cooling", hand_snapshot(3.0, 3, true)),
        ("no-enemy", hand_snapshot(0.0, 9, false)),
    ]
}

fn hand_snapshot(smoke_cd: f32, first_enemy_x: i32, with_enemies: bool) -> WorldSnapshot {
    let mut cooldowns = std::collections::BTreeMap::new();
    cooldowns.insert("dash".to_string(), 1.5);
    cooldowns.insert("throw:smoke".to_string(), smoke_cd);
    let enemies = if with_enemies {
        vec![
            EnemyState {
                id: 11,
                pos: IVec2 {
                    x: first_enemy_x,
                    y: 4,
                },
                hp: 50,
                cover: "low".into(),
                last_seen: 1.0,
            },
            EnemyState {
                id: 12,
                pos: IVec2 { x: 9, y: 1 },
                hp: 30,
                cover: "high".into(),
                last_seen: 1.0,
            },
        ]
    } else {
        Vec::new()
    };
    WorldSnapshot {
        t: 4.25,
        player: PlayerState {
            hp: 73,
            pos: IVec2 { x: 0, y: 1 },
            stance: "crouch".into(),
            orders: vec!["hold_east".to_string()],
        },
        me: CompanionState {
            ammo: 7,
            cooldowns,
            morale: 0.6,
            pos: IVec2 { x: 2, y: 3 },
        },
        enemies,
        // Authored fixture. Perception no longer inserts a breach at a fixed cell.
        pois: vec![],
        obstacles: vec![IVec2 { x: 1, y: 2 }, IVec2 { x: 3, y: 3 }],
        objective: Some("extract".to_string()),
    }
}

fn first_move(plan: &PlanIntent) -> Option<IVec2> {
    plan.steps.iter().find_map(|step| match step {
        ActionStep::MoveTo { x, y, .. } => Some(IVec2 { x: *x, y: *y }),
        _ => None,
    })
}

fn encode_plan(out: &mut Vec<u8>, plan: &PlanIntent) {
    push_str(out, &plan.plan_id);
    push_u32(out, plan.steps.len() as u32);
    for step in &plan.steps {
        encode_step(out, step);
    }
}

fn encode_snapshot(out: &mut Vec<u8>, snap: &WorldSnapshot) {
    push_f32(out, snap.t);
    push_i32(out, snap.player.hp);
    push_ivec2(out, snap.player.pos);
    push_str(out, &snap.player.stance.to_string());
    push_u32(out, snap.player.orders.len() as u32);
    for order in &snap.player.orders {
        push_str(out, order);
    }
    push_i32(out, snap.me.ammo);
    push_u32(out, snap.me.cooldowns.len() as u32);
    for (key, value) in &snap.me.cooldowns {
        push_str(out, key);
        push_f32(out, *value);
    }
    push_f32(out, snap.me.morale);
    push_ivec2(out, snap.me.pos);
    push_u32(out, snap.enemies.len() as u32);
    for enemy in &snap.enemies {
        push_u32(out, enemy.id);
        push_ivec2(out, enemy.pos);
        push_i32(out, enemy.hp);
        push_str(out, &enemy.cover.to_string());
        push_f32(out, enemy.last_seen);
    }
    push_u32(out, snap.pois.len() as u32);
    for poi in &snap.pois {
        push_str(out, &poi.k);
        push_ivec2(out, poi.pos);
    }
    push_u32(out, snap.obstacles.len() as u32);
    for obstacle in &snap.obstacles {
        push_ivec2(out, *obstacle);
    }
    match &snap.objective {
        Some(text) => {
            push_u8(out, 1);
            push_str(out, text);
        }
        None => push_u8(out, 0),
    }
}

fn encode_step(out: &mut Vec<u8>, step: &ActionStep) {
    match step {
        ActionStep::MoveTo { x, y, speed } => {
            push_u8(out, 1);
            push_i32(out, *x);
            push_i32(out, *y);
            encode_speed(out, *speed);
        }
        ActionStep::Throw { item, x, y } => {
            push_u8(out, 2);
            push_str(out, item);
            push_i32(out, *x);
            push_i32(out, *y);
        }
        ActionStep::CoverFire {
            target_id,
            duration,
        } => {
            push_u8(out, 3);
            push_u32(out, *target_id);
            push_f32(out, *duration);
        }
        ActionStep::Wait { duration } => {
            push_u8(out, 4);
            push_f32(out, *duration);
        }
        other => {
            push_u8(out, 255);
            push_str(out, other.action_name());
        }
    }
}

fn encode_speed(out: &mut Vec<u8>, speed: Option<MovementSpeed>) {
    match speed {
        None => push_u8(out, 0),
        Some(MovementSpeed::Walk) => push_u8(out, 1),
        Some(MovementSpeed::Run) => push_u8(out, 2),
        Some(MovementSpeed::Sprint) => push_u8(out, 3),
        Some(_) => push_u8(out, 255),
    }
}

fn push_ivec2(out: &mut Vec<u8>, pos: IVec2) {
    push_i32(out, pos.x);
    push_i32(out, pos.y);
}

fn push_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_i32(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_f32(out: &mut Vec<u8>, value: f32) {
    push_u32(out, value.to_bits());
}

fn push_str(out: &mut Vec<u8>, value: &str) {
    push_u32(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}
