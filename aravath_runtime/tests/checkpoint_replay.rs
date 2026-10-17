//! `greybox_checkpoint` digest. Save identity and the restored fight come from the sim.

use std::path::PathBuf;

use aravath_runtime::motor::MotorMode;
use aravath_runtime::save::{gameplay_hash, save_checkpoint, ARAVATH_SAVE_SEED};
use aravath_runtime::Greybox;
use aw_save::sim_save_id;
use glam::Vec3;
use sha2::{Digest, Sha256};

fn replay_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir).join("replay-hash-greybox.json");
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/replay-hash-greybox.json")
}

fn scenario_bytes() -> Vec<u8> {
    let mut sim = Greybox::island();
    sim.tick = 30;
    sim.motor.mode = MotorMode::Roll;
    sim.combat.attack.time = 0.12;
    sim.combat.attack.swinging = true;
    sim.combat.cover_broken = true;
    sim.spawn_enemy(Vec3::new(4.0, 2.0, 6.0));
    sim.combat.enemies[0].health = 17.0;
    sim.combat.lock = Some(0);
    let checkpoint = sim.checkpoint();
    let dir = tempfile::tempdir().expect("tempdir");
    save_checkpoint(dir.path(), &checkpoint).expect("save");
    let loaded = aravath_runtime::save::load_checkpoint(dir.path()).expect("load");
    let (bundle, _) = aw_save::SaveManager::new(dir.path())
        .load_latest_slot("aravath", 0)
        .expect("bundle");
    assert_eq!(bundle.save_id, sim_save_id(ARAVATH_SAVE_SEED, 30, 0));
    assert_eq!(bundle.created_at.unix_timestamp(), 30);
    assert_eq!(bundle.world.tick, 30);
    assert_eq!(bundle.world.state_hash, gameplay_hash(&checkpoint));
    assert_eq!(loaded.motor_mode, MotorMode::Roll);
    assert!(loaded.cover_broken);
    assert_eq!(loaded.lock, Some(0));
    assert_eq!(loaded.enemies.len(), 1);
    assert_eq!(loaded.enemies[0].health.to_bits(), 17.0f32.to_bits());
    assert!(loaded.attack_swinging);

    let mut bytes = Vec::new();
    bytes.extend_from_slice(bundle.save_id.as_bytes());
    bytes.extend_from_slice(&bundle.world.tick.to_le_bytes());
    bytes.extend_from_slice(&bundle.world.state_hash.to_le_bytes());
    bytes.extend_from_slice(&gameplay_hash(&loaded).to_le_bytes());
    bytes.extend_from_slice(&loaded.enemies[0].health.to_bits().to_le_bytes());
    bytes.push(u8::from(loaded.cover_broken));
    bytes.push(u8::from(loaded.attack_swinging));
    bytes
}

#[test]
fn greybox_checkpoint_replay_is_stable_and_written() {
    let first = scenario_bytes();
    let second = scenario_bytes();
    assert_eq!(first, second);
    let digest = hex_encode(&Sha256::digest(&first));
    let path = replay_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("target dir");
    }
    let body =
        format!("[\n  {{ \"case\": \"greybox_checkpoint\", \"sha256\": \"{digest}\" }}\n]\n");
    std::fs::write(&path, body).expect("write replay hash");
    println!("greybox_checkpoint {digest}");
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
