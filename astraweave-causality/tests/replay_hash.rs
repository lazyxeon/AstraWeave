//! `causal_bridge` digest for the three-OS determinism job.
//!
//! Writes `target/replay-hash-causal.json`. The compare job merges it with
//! the core and combat hashes.

use std::path::PathBuf;

use astraweave_causality::run_causal_bridge;

fn replay_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir).join("replay-hash-causal.json");
    }
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push("target");
    path.push("replay-hash-causal.json");
    path
}

#[test]
fn causal_bridge_replay_is_stable_and_written() {
    let world = run_causal_bridge().expect("bridge");
    assert_eq!(world.route_class(), 2);
    assert_eq!(world.stock(), 0);
    assert_eq!(world.stance(), 2);
    assert_eq!(world.quest_template(), 1);
    assert_eq!(world.quest_complete(), 1);
    let first = world.replay_hash();
    let second = run_causal_bridge().expect("bridge").replay_hash();
    assert_eq!(first, second, "causal_bridge diverged inside one process");
    assert_eq!(
        first, "813492cb11ffb9834dcfbcdd13fc0aaffd7b07c8ac54e1c921bbdb773dc83446",
        "causal_bridge digest drifted"
    );
    let path = replay_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("target dir");
    }
    let body = format!("[\n  {{ \"case\": \"causal_bridge\", \"sha256\": \"{first}\" }}\n]\n");
    std::fs::write(&path, &body).expect("write replay hash");
    println!("causal_bridge {first}");
    println!("wrote {}", path.display());
}
