//! Cross-process replay hash for the AI depth tick.
//!
//! Two worlds built with the same seed produce the same SHA-256 of the
//! canonical tick trace. The digest is upserted into `target/replay-hash.json`
//! so `determinism-replay` can compare it with `planner_pure` and
//! `perception_world` across Linux, Windows, and macOS.

use std::fs;
use std::path::PathBuf;

use astraweave_dialogue::depth_tick::{install, AiDepthState};
use astraweave_ecs::App;

fn run(seed: u64, recorded: Option<(u64, &str)>, steps: u32) -> (String, String) {
    let mut app = App::new();
    install(&mut app);
    {
        let state = app
            .world
            .get_resource_mut::<AiDepthState>()
            .expect("depth state installed");
        // install() seeds state with 1. Replace it before any step.
        *state = AiDepthState::new(seed);
        if let Some((tick, line)) = recorded {
            state.record_line(tick, line);
        }
    }
    let app = app.run_fixed(steps);
    let state = app
        .world
        .get_resource::<AiDepthState>()
        .expect("depth state");
    (state.replay_hash(), state.trace_joined())
}

fn replay_json_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir).join("replay-hash.json");
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/replay-hash.json")
}

#[test]
fn ai_depth_tick_replay_hash_is_stable() {
    let (left_hash, left_trace) = run(7, None, 8);
    let (right_hash, right_trace) = run(7, None, 8);
    assert_eq!(left_hash, right_hash);
    assert_eq!(left_trace, right_trace);
    assert!(left_trace.contains("merged_"), "{left_trace}");
    assert!(left_trace.contains("heard:tick-"), "{left_trace}");
    assert!(left_trace.contains("say:"), "{left_trace}");
    assert!(left_trace.contains("scripted:tick "), "{left_trace}");
    assert_eq!(left_hash.len(), 64);

    let (other_hash, _) = run(8, None, 8);
    assert_ne!(left_hash, other_hash);

    let (recorded_hash, recorded_trace) = run(7, Some((3, "MODEL-LINE")), 8);
    let (recorded_again, _) = run(7, Some((3, "MODEL-LINE")), 8);
    assert_eq!(recorded_hash, recorded_again);
    assert_ne!(recorded_hash, left_hash);
    assert!(recorded_trace.contains("MODEL-LINE"), "{recorded_trace}");
    assert!(
        !recorded_trace.contains("scripted:tick 3 "),
        "{recorded_trace}"
    );

    upsert_replay_case(&replay_json_path(), "ai_depth_tick", &left_hash);
}

fn upsert_replay_case(path: &std::path::Path, case: &str, sha256: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("replay hash directory");
    }
    let mut rows: Vec<serde_json::Value> = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    rows.retain(|row| row.get("case").and_then(|value| value.as_str()) != Some(case));
    rows.push(serde_json::json!({ "case": case, "sha256": sha256 }));
    let body = serde_json::to_vec_pretty(&rows).expect("encode replay-hash json");
    fs::write(path, body).expect("write replay hash");
}
