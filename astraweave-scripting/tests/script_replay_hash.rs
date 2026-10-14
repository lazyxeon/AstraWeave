//! Cross-OS replay hash for the mod script system.
//!
//! Two runs in this process must match. CI runs this binary on Linux, Windows,
//! and macOS and diffs `target/script-replay-hash.json`. The digest is SHA-256
//! of little-endian integers and length-prefixed strings. No `DefaultHasher`.

use astraweave_ecs::App;
use astraweave_scripting::sim_mod::{
    mod_script_system, mod_state_digest, CModAgent, ModClock, ModPrograms, ModScriptPlugin,
    ModScriptSource,
};
use std::path::PathBuf;

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

#[test]
fn scripted_system_replay_hash() {
    let first = run_case();
    let second = run_case();
    assert_eq!(
        first, second,
        "same seed and script bytes diverged in-process"
    );
    assert_eq!(
        hex::encode(first),
        "5a54c6f435f4d43e15bcd97548f33e15fd2f5fc4e4d2b7cc8fb3af209079dfd2"
    );

    let hex = hex::encode(first);
    let json = format!("[{{\"case\":\"scripted_system\",\"sha256\":\"{hex}\"}}]\n");
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    let dir = path.join("target");
    std::fs::create_dir_all(&dir).expect("create target/");
    let file = dir.join("script-replay-hash.json");
    std::fs::write(&file, &json).expect("write replay hash");
    println!("scripted_system {hex}");
    println!("wrote {}", file.display());
}
