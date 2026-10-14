//! Cross-OS replay hash for the mod script system.
//!
//! Appends the `scripted_system` case to `target/replay-hash.json`, the file
//! `determinism-replay` already compares across Linux, Windows, and macOS.
//! The digest is SHA-256 of little-endian integers and length-prefixed strings.

use astraweave_ecs::App;
use astraweave_scripting::sim_mod::{
    mod_script_system, mod_state_digest, CModAgent, ModClock, ModPrograms, ModScriptPlugin,
    ModScriptSource,
};
use std::path::{Path, PathBuf};

const SCRIPT: &str = "let roll = rng.sim_roll(4);\nmoves.move_to(pos_x + roll, pos_y + 1);\n";
const TICKS: u32 = 32;
const WORLD_SEED: u64 = 0xA11C_E5E0;

fn run_case() -> [u8; 32] {
    let mut app = App::new().add_plugin(ModScriptPlugin);
    app.world.insert_resource(ModClock::new(WORLD_SEED));
    app.world.insert_resource(ModPrograms {
        scripts: vec![ModScriptSource::new(
            "demo.mod",
            "scripts/tick.rhai",
            SCRIPT,
        )],
    });
    for (x, y) in [(0, 0), (3, 1), (8, 2)] {
        let entity = app.world.spawn();
        app.world.insert(entity, CModAgent { x, y });
    }
    for _ in 0..TICKS {
        mod_script_system(&mut app.world);
    }
    mod_state_digest(&app.world)
}

fn replay_hash_json_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir).join("replay-hash.json");
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/replay-hash.json")
}

fn json_field(object: &str, key: &str) -> Option<String> {
    let marker = format!("\"{key}\"");
    let rest = object.split_once(&marker)?.1;
    let start = rest.find('"')? + 1;
    let end = start + rest[start..].find('"')?;
    Some(rest[start..end].to_string())
}

fn upsert_case(path: &Path, case: &str, sha256: &str) {
    let mut rows = Vec::new();
    if let Ok(text) = std::fs::read_to_string(path) {
        for object in text.split('{').skip(1) {
            let Some(existing_case) = json_field(object, "case") else {
                continue;
            };
            let Some(existing_sha) = json_field(object, "sha256") else {
                continue;
            };
            if existing_case != case {
                rows.push((existing_case, existing_sha));
            }
        }
    }
    rows.push((case.to_string(), sha256.to_string()));
    rows.sort();
    let mut body = String::from("[\n");
    for (index, (name, digest)) in rows.iter().enumerate() {
        body.push_str(&format!(
            "  {{\"case\": \"{name}\", \"sha256\": \"{digest}\"}}"
        ));
        if index + 1 != rows.len() {
            body.push(',');
        }
        body.push('\n');
    }
    body.push_str("]\n");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create replay-hash directory");
    }
    std::fs::write(path, body).expect("write replay-hash json");
}

#[test]
fn scripted_system_replay_hash() {
    let first = run_case();
    let second = run_case();
    assert_eq!(
        first, second,
        "same seed and script bytes diverged in-process"
    );

    let hex = hex::encode(first);
    assert_eq!(
        hex,
        "362e4c6d3085d2a95d1219b988326beb4cbe898408de4a91e5b2abe3c3294d78"
    );
    let path = replay_hash_json_path();
    upsert_case(&path, "scripted_system", &hex);
    println!("scripted_system {hex}");
    println!("wrote {}", path.display());
}
