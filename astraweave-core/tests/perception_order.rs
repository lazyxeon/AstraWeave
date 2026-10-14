//! Two worlds built with the same spawns must perceive and plan identically.

#[path = "replay_support/mod.rs"]
mod replay_support;

use astraweave_ai::{Orchestrator, RuleOrchestrator};
use astraweave_core::{build_snapshot, IVec2, PerceptionConfig, Team, World};
use replay_support::sha256_hex;

fn spawn_fixture() -> (World, u32, u32) {
    let mut world = World::new();
    let player = world.spawn_with_id(10, "player", IVec2 { x: 0, y: 0 }, Team { id: 0 }, 100, 0);
    let companion =
        world.spawn_with_id(3, "companion", IVec2 { x: 2, y: 1 }, Team { id: 1 }, 80, 12);
    world.spawn_with_id(8, "far", IVec2 { x: 9, y: 4 }, Team { id: 2 }, 40, 0);
    world.spawn_with_id(5, "near", IVec2 { x: 4, y: 2 }, Team { id: 2 }, 55, 0);
    world.obstacles.insert((9, 1));
    world.obstacles.insert((1, 4));
    world.obstacles.insert((1, 2));
    (world, player, companion)
}

#[test]
fn two_worlds_share_enemy_order_plan_and_hash() {
    let (a, player_a, comp_a) = spawn_fixture();
    let (b, player_b, comp_b) = spawn_fixture();
    assert_eq!(a.enemies_of(1), vec![5, 8, 10]);
    assert_eq!(a.enemies_of(1), b.enemies_of(1));
    assert_eq!(a.all_of_team(1), vec![3]);
    assert_eq!(a.entities(), vec![3, 5, 8, 10]);

    let cfg = PerceptionConfig { los_max: 6 };
    let enemies_a = a.enemies_of(1);
    let enemies_b = b.enemies_of(1);
    let snap_a = build_snapshot(&a, player_a, comp_a, &enemies_a, None, &cfg);
    let snap_b = build_snapshot(&b, player_b, comp_b, &enemies_b, None, &cfg);
    let obstacle_coords: Vec<(i32, i32)> = snap_a.obstacles.iter().map(|p| (p.x, p.y)).collect();
    assert_eq!(obstacle_coords, vec![(1, 2), (1, 4), (9, 1)]);
    assert_eq!(snap_a.enemies[0].id, 5);
    assert_eq!(snap_a.obstacles, snap_b.obstacles);
    assert_eq!(snap_a.enemies.len(), snap_b.enemies.len());
    for (left, right) in snap_a.enemies.iter().zip(snap_b.enemies.iter()) {
        assert_eq!(left.id, right.id);
        assert_eq!(left.pos, right.pos);
        assert_eq!(left.hp, right.hp);
        assert_eq!(left.cover, right.cover);
        assert_eq!(left.last_seen.to_bits(), right.last_seen.to_bits());
    }

    let rule = RuleOrchestrator;
    let plan_a = rule.propose_plan(&snap_a);
    let plan_b = rule.propose_plan(&snap_b);
    assert_eq!(plan_a, plan_b);

    let hash_a = sha256_hex(&replay_support::perception_world_bytes());
    let hash_b = sha256_hex(&replay_support::perception_world_bytes());
    assert_eq!(hash_a, hash_b);
}
