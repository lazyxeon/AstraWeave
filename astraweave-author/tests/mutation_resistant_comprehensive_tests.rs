//! Mutation-resistant comprehensive tests for astraweave-author.
//!
//! Targets: MapMeta (construction, clone, field access),
//! run_author_script (with a real Rhai script file).

use astraweave_author::MapMeta;

// =========================================================================
// MapMeta — field access, clone, boundary values
// =========================================================================

#[test]
fn map_meta_fields() {
    let m = MapMeta {
        width: 100,
        height: 200,
        enemy_count: 10,
        difficulty: 3,
    };
    assert_eq!(m.width, 100);
    assert_eq!(m.height, 200);
    assert_eq!(m.enemy_count, 10);
    assert_eq!(m.difficulty, 3);
}

#[test]
fn map_meta_clone_preserves_all_fields() {
    let m = MapMeta {
        width: 50,
        height: 75,
        enemy_count: 5,
        difficulty: 1,
    };
    let c = m.clone();
    assert_eq!(c.width, 50);
    assert_eq!(c.height, 75);
    assert_eq!(c.enemy_count, 5);
    assert_eq!(c.difficulty, 1);
}

#[test]
fn map_meta_clone_independent() {
    let m = MapMeta {
        width: 10,
        height: 20,
        enemy_count: 3,
        difficulty: 2,
    };
    let mut c = m.clone();
    c.width = 999;
    // Original unchanged
    assert_eq!(m.width, 10);
    assert_eq!(c.width, 999);
}

#[test]
fn map_meta_zero_values() {
    let m = MapMeta {
        width: 0,
        height: 0,
        enemy_count: 0,
        difficulty: 0,
    };
    assert_eq!(m.width, 0);
    assert_eq!(m.height, 0);
    assert_eq!(m.enemy_count, 0);
    assert_eq!(m.difficulty, 0);
}

#[test]
fn map_meta_negative_values() {
    let m = MapMeta {
        width: -1,
        height: -1,
        enemy_count: -5,
        difficulty: -1,
    };
    assert_eq!(m.width, -1);
    assert_eq!(m.enemy_count, -5);
}

#[test]
fn map_meta_max_difficulty() {
    let m = MapMeta {
        width: 100,
        height: 100,
        enemy_count: 50,
        difficulty: 5,
    };
    assert_eq!(m.difficulty, 5);
}

#[test]
fn map_meta_min_difficulty() {
    let m = MapMeta {
        width: 100,
        height: 100,
        enemy_count: 1,
        difficulty: 1,
    };
    assert_eq!(m.difficulty, 1);
}

// =========================================================================
// run_author_script — with a real Rhai file
// =========================================================================

/// The loader rejects paths outside the working directory. Keep the file there.
fn script_in_workdir(name: &str, source: &str) -> (tempfile::TempDir, String) {
    let root = std::env::current_dir().expect("workdir");
    let dir = tempfile::tempdir_in(&root).expect("script dir under workdir");
    let file = dir.path().join(name);
    std::fs::write(&file, source).expect("write script");
    let rel = file
        .strip_prefix(&root)
        .expect("script stays under workdir");
    let text = rel
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");
    (dir, text)
}

#[test]
fn run_author_script_simple() {
    let (_dir, script_path) = script_in_workdir(
        "test.rhai",
        r#"
fn configure(meta) {
    let traps = meta.difficulty * 2;
    let terrain_edits = meta.width / 10;
    let spawns = meta.enemy_count;
    let hints = #{"tip": "use cover"};
    #{
        traps: traps,
        terrain_edits: terrain_edits,
        spawns: spawns,
        hints: hints
    }
}
"#,
    );

    let meta = MapMeta {
        width: 100,
        height: 50,
        enemy_count: 8,
        difficulty: 3,
    };
    let (budget, hints) =
        astraweave_author::run_author_script(&script_path, &meta).expect("script under workdir");

    assert_eq!(budget.traps, 6); // difficulty(3) * 2
    assert_eq!(budget.terrain_edits, 10); // width(100) / 10
    assert_eq!(budget.spawns, 8); // enemy_count
    assert_eq!(hints["tip"], "use cover");
}

#[test]
fn run_author_script_defaults_on_missing_keys() {
    // Empty map — defaults: traps=1, terrain_edits=2, spawns=1
    let (_dir, script_path) = script_in_workdir(
        "minimal.rhai",
        r#"
fn configure(meta) {
    #{}
}
"#,
    );

    let meta = MapMeta {
        width: 10,
        height: 10,
        enemy_count: 0,
        difficulty: 1,
    };
    let (budget, hints) =
        astraweave_author::run_author_script(&script_path, &meta).expect("script under workdir");

    assert_eq!(budget.traps, 1); // default
    assert_eq!(budget.terrain_edits, 2); // default
    assert_eq!(budget.spawns, 1); // default
    assert!(hints.is_object()); // empty hints map → empty JSON object
}

#[test]
fn run_author_script_nonexistent_file_errors() {
    let meta = MapMeta {
        width: 10,
        height: 10,
        enemy_count: 0,
        difficulty: 1,
    };
    let result = astraweave_author::run_author_script("/nonexistent/path.rhai", &meta);
    assert!(result.is_err());
}

#[test]
fn run_author_script_meta_fields_passed_correctly() {
    let (_dir, script_path) = script_in_workdir(
        "echo.rhai",
        r#"
fn configure(meta) {
    #{
        traps: 1,
        terrain_edits: 2,
        spawns: 1,
        hints: #{
            w: meta.width,
            h: meta.height,
            e: meta.enemy_count,
            d: meta.difficulty
        }
    }
}
"#,
    );

    let meta = MapMeta {
        width: 42,
        height: 99,
        enemy_count: 7,
        difficulty: 4,
    };
    let (_, hints) =
        astraweave_author::run_author_script(&script_path, &meta).expect("script under workdir");

    assert_eq!(hints["w"], 42);
    assert_eq!(hints["h"], 99);
    assert_eq!(hints["e"], 7);
    assert_eq!(hints["d"], 4);
}
