//! Cross-process replay hashes for the determinism gate.
//!
//! Writes `target/replay-hash.json` (honoring `CARGO_TARGET_DIR`). The CI
//! compare job diffs that file across Linux, Windows, and macOS.

#[path = "replay_support/mod.rs"]
mod replay_support;

use astraweave_destruction::demolition_bytes;
use astraweave_math::fixed64_contract_bytes;
use replay_support::{
    gameplay_rng_bytes, perception_world_bytes, planner_pure_bytes, replay_hash_json_path,
    sha256_hex,
};

#[test]
fn replay_hash_cases_are_stable_and_written() {
    let planner_a = planner_pure_bytes();
    let planner_b = planner_pure_bytes();
    assert_eq!(
        planner_a, planner_b,
        "planner_pure diverged inside one process"
    );

    let perception = perception_world_bytes();
    let gameplay_a = gameplay_rng_bytes();
    let gameplay_b = gameplay_rng_bytes();
    assert_eq!(
        gameplay_a, gameplay_b,
        "gameplay_rng diverged inside one process"
    );
    let fixed_a = fixed64_contract_bytes();
    let fixed_b = fixed64_contract_bytes();
    assert_eq!(fixed_a, fixed_b, "fixed64 diverged inside one process");
    let destruction_a = demolition_bytes(0xA2A0_0001);
    let destruction_b = demolition_bytes(0xA2A0_0001);
    assert_eq!(
        destruction_a, destruction_b,
        "destruction diverged inside one process"
    );
    let cases = [
        ("destruction".to_string(), sha256_hex(&destruction_a)),
        ("fixed64".to_string(), sha256_hex(&fixed_a)),
        ("gameplay_rng".to_string(), sha256_hex(&gameplay_a)),
        ("perception_world".to_string(), sha256_hex(&perception)),
        ("planner_pure".to_string(), sha256_hex(&planner_a)),
    ];

    for (case, digest) in &cases {
        println!("replay-hash {case} {digest}");
        assert_eq!(digest.len(), 64, "{case} digest must be SHA-256 hex");
        assert!(
            digest.chars().all(|c| c.is_ascii_hexdigit()),
            "{case} digest must be hex"
        );
    }

    let path = replay_hash_json_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create replay-hash directory");
    }
    let rows: Vec<serde_json::Value> = cases
        .iter()
        .map(|(case, sha256)| serde_json::json!({ "case": case, "sha256": sha256 }))
        .collect();
    let body = serde_json::to_vec_pretty(&rows).expect("encode replay-hash json");
    std::fs::write(&path, body).expect("write replay-hash json");
    println!("wrote {}", path.display());
}
