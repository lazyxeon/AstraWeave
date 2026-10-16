//! `local_sim` digest. Not in the required set until it matches on three OSes.

use std::path::PathBuf;

use astraweave_localsim::local_sim_case_bytes;
use sha2::{Digest, Sha256};

fn replay_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir).join("replay-hash-localsim.json");
    }
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push("target");
    path.push("replay-hash-localsim.json");
    path
}

fn sha256_hex(bytes: &[u8]) -> String {
    let dig = Sha256::digest(bytes);
    dig.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn local_sim_replay_is_stable_and_written() {
    let first = local_sim_case_bytes();
    let second = local_sim_case_bytes();
    assert_eq!(first, second, "local_sim diverged inside one process");
    let hex = sha256_hex(&first);
    let path = replay_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("replay dir");
    }
    let body = format!("[\n  {{ \"case\": \"local_sim\", \"sha256\": \"{hex}\" }}\n]\n");
    std::fs::write(&path, body).expect("write local_sim replay");
    println!("local_sim {hex}");
}
