//! Cross-OS replay bytes for the `traversal` case.
//!
//! Writes `target/replay-hash-traversal.json`. The determinism workflow merges
//! that file with the other replay cases.
//! on Ubuntu, Windows, and macOS. The digest covers a real 180-tick run of
//! ground, jump, and roll. It is not a constant.

use astraweave_physics::traversal::{replay_bytes, ProbeFrame, TraversalBody, TICK_DT};
use astraweave_physics::PhysicsWorld;
use glam::vec3;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[test]
fn climb_flags_fail_closed_for_dynamic_bodies_and_puzzle_volumes() {
    use astraweave_physics::traversal::Climbability;
    use astraweave_physics::Layers;
    let mut world = PhysicsWorld::new(vec3(0.0, -9.8, 0.0));
    let ground = world.create_ground_plane(vec3(10.0, 0.5, 10.0), 0.9);
    assert_eq!(
        world.climbability_at(ground, vec3(0.0, 0.2, 0.0)),
        Climbability::Climbable
    );
    let built = world.add_dynamic_box(
        vec3(2.0, 1.0, 0.0),
        vec3(0.5, 0.5, 0.5),
        1.0,
        Layers::DEFAULT,
    );
    assert_eq!(
        world.climbability_at(built, vec3(2.0, 1.0, 0.0)),
        Climbability::NonClimbable
    );
    world.set_climbability(built, Climbability::Climbable);
    assert_eq!(
        world.climbability_at(built, vec3(2.0, 1.0, 0.0)),
        Climbability::Climbable
    );
    world.add_climb_blocker(vec3(-1.0, -1.0, -1.0), vec3(1.0, 3.0, 1.0));
    assert_eq!(
        world.climbability_at(ground, vec3(0.0, 0.2, 0.0)),
        Climbability::NonClimbable
    );
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

fn replay_hash_json_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir).join("replay-hash-traversal.json");
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/replay-hash-traversal.json")
}

#[test]
fn traversal_replay_is_stable_and_written() {
    let first = replay_bytes();
    let second = replay_bytes();
    assert_eq!(first, second, "traversal diverged inside one process");
    assert!(first.len() > 128, "replay bytes must carry per-tick state");
    let digest = sha256_hex(&first);
    assert_eq!(digest.len(), 64);

    let idle = {
        let mut body = TraversalBody::default();
        let mut out_changed = false;
        let probe = ProbeFrame::plane(0.0);
        let start = body.pos.x.to_bits();
        astraweave_physics::traversal::step(
            &mut body,
            astraweave_physics::traversal::TraversalInput {
                move_y: 1.0,
                ..Default::default()
            },
            &probe,
            0,
        );
        if body.pos.x.to_bits() != start {
            out_changed = true;
        }
        out_changed
    };
    assert!(idle, "ground motion must change position bits");

    let path = replay_hash_json_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create replay-hash directory");
    }
    let rows = vec![serde_json::json!({ "case": "traversal", "sha256": digest })];
    let body = serde_json::to_vec_pretty(&rows).expect("encode replay-hash json");
    std::fs::write(&path, body).expect("write replay-hash json");
    println!("replay-hash traversal {digest}");
    println!("wrote {}", path.display());
}

#[test]
fn character_full_tick_reset_stays_above_ground() {
    let mut world = PhysicsWorld::new(vec3(0.0, -9.8, 0.0));
    world.create_ground_plane(vec3(50.0, 0.5, 50.0), 0.9);
    let id = world.add_character(vec3(0.0, 1.0, 0.0), vec3(0.4, 0.9, 0.4));

    for _ in 0..4_000 {
        world.set_body_position(id, vec3(0.0, 1.0, 0.0));
        if let Some(ctrl) = world.char_map.get_mut(&id) {
            ctrl.vertical_velocity = 0.0;
            ctrl.time_since_grounded = 0.0;
            ctrl.jump_buffer_timer = 0.0;
            ctrl.pending_jump_velocity = 0.0;
        }
        world.control_character(id, vec3(1.0, 0.0, 0.0), TICK_DT, false);
        world.step();
        let pos = world
            .body_transform(id)
            .expect("character transform")
            .col(3);
        assert!(
            pos.y > -10.0,
            "reset tick fell through the ground (y={})",
            pos.y
        );
    }
}
