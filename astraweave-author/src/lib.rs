#![forbid(unsafe_code)]
use anyhow::{bail, Context, Result};
use astraweave_core::DirectorBudget;
use astraweave_security::sandbox::{
    call_deterministic, read_rhai_under_root, validate_budget, BudgetLimits, EngineCeilings,
    SandboxEngine, SandboxProfile,
};
use rhai::{Dynamic, Map};
use std::path::Path;

#[derive(Clone)]
pub struct MapMeta {
    pub width: i32,
    pub height: i32,
    pub enemy_count: i32,
    pub difficulty: i32, // 1..5
}

/// Run `configure(meta)` from a `.rhai` file under the process working directory.
///
/// The path is resolved with the sandbox loader: `..`, absolute escapes, and
/// files outside the working directory are rejected. The script runs in the
/// author sandbox (operation and memory caps, no filesystem, network, or process).
pub fn run_author_script(
    path: &str,
    meta: &MapMeta,
) -> Result<(DirectorBudget, serde_json::Value)> {
    let root = std::env::current_dir().context("read working directory")?;
    run_author_script_in_root(&root, Path::new(path), meta)
}

/// Run `configure(meta)` from a script path that must stay inside `root`.
pub fn run_author_script_in_root(
    root: &Path,
    path: &Path,
    meta: &MapMeta,
) -> Result<(DirectorBudget, serde_json::Value)> {
    let ceilings = EngineCeilings::default();
    let (_canon, source) = read_rhai_under_root(root, path, ceilings.max_script_bytes)?;
    run_author_source(&source, meta)
}

/// Run `configure(meta)` from source bytes. Same sandbox as the path loader.
pub fn run_author_source(
    source: &str,
    meta: &MapMeta,
) -> Result<(DirectorBudget, serde_json::Value)> {
    let mut meta_map = Map::new();
    meta_map.insert("width".into(), Dynamic::from(i64::from(meta.width)));
    meta_map.insert("height".into(), Dynamic::from(i64::from(meta.height)));
    meta_map.insert(
        "enemy_count".into(),
        Dynamic::from(i64::from(meta.enemy_count)),
    );
    meta_map.insert(
        "difficulty".into(),
        Dynamic::from(i64::from(meta.difficulty)),
    );

    let sandbox = SandboxEngine::build(&SandboxProfile::author()).into_script_sandbox();
    let out = call_deterministic(&sandbox, source, "configure", (meta_map,))?;
    let map: Map = out
        .try_cast::<Map>()
        .context("configure must return a map")?;

    let traps = budget_field(&map, "traps", 1)?;
    let terrain = budget_field(&map, "terrain_edits", 2)?;
    let spawns = budget_field(&map, "spawns", 1)?;
    let ceilings = EngineCeilings::default();
    validate_budget(
        BudgetLimits {
            traps,
            terrain_edits: terrain,
            spawns,
        },
        &ceilings,
    )?;

    let hints_dyn = map
        .get("hints")
        .cloned()
        .unwrap_or_else(|| Dynamic::from(Map::new()));
    let hints_json = strict_dynamic_to_json(&hints_dyn, 0)?;

    Ok((
        DirectorBudget {
            traps,
            terrain_edits: terrain,
            spawns,
        },
        hints_json,
    ))
}

fn budget_field(map: &Map, name: &str, default: i32) -> Result<i32> {
    let Some(value) = map.get(name) else {
        return Ok(default);
    };
    let raw = value
        .clone()
        .try_cast::<i64>()
        .with_context(|| format!("{name} must be an integer"))?;
    i32::try_from(raw).with_context(|| format!("{name} does not fit in i32"))
}

/// JSON for mod hints. Floats are rejected so the value does not depend on libm.
fn strict_dynamic_to_json(value: &Dynamic, depth: usize) -> Result<serde_json::Value> {
    if depth > 8 {
        bail!("hints nesting exceeds 8");
    }
    if value.is::<Map>() {
        let map: Map = value.clone().cast();
        let mut out = serde_json::Map::new();
        for (key, child) in map {
            out.insert(key.to_string(), strict_dynamic_to_json(&child, depth + 1)?);
        }
        Ok(serde_json::Value::Object(out))
    } else if value.is::<rhai::Array>() {
        let array: rhai::Array = value.clone().cast();
        let mut out = Vec::with_capacity(array.len());
        for child in array {
            out.push(strict_dynamic_to_json(&child, depth + 1)?);
        }
        Ok(serde_json::Value::Array(out))
    } else if let Some(integer) = value.clone().try_cast::<i64>() {
        Ok(serde_json::Value::from(integer))
    } else if value.clone().try_cast::<f64>().is_some() || value.clone().try_cast::<f32>().is_some()
    {
        bail!("hints must not contain floats");
    } else if let Some(flag) = value.clone().try_cast::<bool>() {
        Ok(serde_json::Value::from(flag))
    } else if let Some(text) = value.clone().try_cast::<String>() {
        Ok(serde_json::Value::from(text))
    } else if value.is_unit() {
        Ok(serde_json::Value::Null)
    } else {
        bail!("hints contain an unsupported value");
    }
}

#[cfg(test)]
fn rhai_to_json(d: &rhai::Dynamic) -> Result<serde_json::Value> {
    if d.is::<rhai::Map>() {
        let m: rhai::Map = d.clone().cast();
        let mut out = serde_json::Map::new();
        for (k, v) in m {
            out.insert(k.into(), rhai_to_json(&v)?);
        }
        Ok(serde_json::Value::Object(out))
    } else if d.is::<rhai::Array>() {
        let arr: rhai::Array = d.clone().cast();
        let mut out = vec![];
        for v in arr {
            out.push(rhai_to_json(&v)?);
        }
        Ok(serde_json::Value::Array(out))
    } else if let Some(i) = d.clone().try_cast::<i64>() {
        Ok(serde_json::Value::from(i))
    } else if let Some(f) = d.clone().try_cast::<f64>() {
        Ok(serde_json::Value::from(f))
    } else if let Some(b) = d.clone().try_cast::<bool>() {
        Ok(serde_json::Value::from(b))
    } else if let Some(s) = d.clone().try_cast::<String>() {
        Ok(serde_json::Value::from(s))
    } else {
        Ok(serde_json::Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::Dynamic;

    #[test]
    fn test_map_meta_creation() {
        let meta = MapMeta {
            width: 100,
            height: 100,
            enemy_count: 10,
            difficulty: 3,
        };
        assert_eq!(meta.width, 100);
        assert_eq!(meta.height, 100);
        assert_eq!(meta.enemy_count, 10);
        assert_eq!(meta.difficulty, 3);
    }

    #[test]
    fn test_map_meta_clone() {
        let meta = MapMeta {
            width: 50,
            height: 75,
            enemy_count: 5,
            difficulty: 2,
        };
        let cloned = meta.clone();
        assert_eq!(cloned.width, meta.width);
        assert_eq!(cloned.height, meta.height);
    }

    #[test]
    fn test_rhai_to_json_i64() {
        let d = Dynamic::from(42_i64);
        let json = rhai_to_json(&d).unwrap();
        assert_eq!(json, serde_json::Value::from(42));
    }

    #[test]
    fn test_rhai_to_json_f64() {
        let d = Dynamic::from(3.25_f64);
        let json = rhai_to_json(&d).unwrap();
        assert_eq!(json, serde_json::Value::from(3.25));
    }

    #[test]
    fn test_rhai_to_json_bool() {
        let d_true = Dynamic::from(true);
        let d_false = Dynamic::from(false);
        assert_eq!(
            rhai_to_json(&d_true).unwrap(),
            serde_json::Value::from(true)
        );
        assert_eq!(
            rhai_to_json(&d_false).unwrap(),
            serde_json::Value::from(false)
        );
    }

    #[test]
    fn test_rhai_to_json_string() {
        let d = Dynamic::from("hello world".to_string());
        let json = rhai_to_json(&d).unwrap();
        assert_eq!(json, serde_json::Value::from("hello world"));
    }

    #[test]
    fn test_rhai_to_json_null() {
        let d = Dynamic::UNIT;
        let json = rhai_to_json(&d).unwrap();
        assert_eq!(json, serde_json::Value::Null);
    }

    #[test]
    fn test_rhai_to_json_array() {
        let arr: rhai::Array = vec![
            Dynamic::from(1_i64),
            Dynamic::from(2_i64),
            Dynamic::from(3_i64),
        ];
        let d = Dynamic::from(arr);

        let json = rhai_to_json(&d).unwrap();
        let expected = serde_json::json!([1, 2, 3]);
        assert_eq!(json, expected);
    }

    #[test]
    fn test_rhai_to_json_map() {
        let mut m = rhai::Map::new();
        m.insert("name".into(), Dynamic::from("test".to_string()));
        m.insert("value".into(), Dynamic::from(42_i64));
        let d = Dynamic::from(m);

        let json = rhai_to_json(&d).unwrap();
        let obj = json.as_object().unwrap();
        assert_eq!(obj.get("name").unwrap(), &serde_json::Value::from("test"));
        assert_eq!(obj.get("value").unwrap(), &serde_json::Value::from(42));
    }

    #[test]
    fn test_rhai_to_json_nested() {
        let mut inner = rhai::Map::new();
        inner.insert("x".into(), Dynamic::from(10_i64));
        inner.insert("y".into(), Dynamic::from(20_i64));

        let mut outer = rhai::Map::new();
        outer.insert("position".into(), Dynamic::from(inner));
        outer.insert("active".into(), Dynamic::from(true));
        let d = Dynamic::from(outer);

        let json = rhai_to_json(&d).unwrap();
        let obj = json.as_object().unwrap();
        assert!(obj.contains_key("position"));
        assert!(obj.contains_key("active"));

        let pos = obj.get("position").unwrap().as_object().unwrap();
        assert_eq!(pos.get("x").unwrap(), &serde_json::Value::from(10));
    }

    #[test]
    fn test_director_budget_from_defaults() {
        // Test that DirectorBudget has expected defaults
        let budget = astraweave_core::DirectorBudget {
            traps: 1,
            terrain_edits: 2,
            spawns: 1,
        };
        assert_eq!(budget.traps, 1);
        assert_eq!(budget.terrain_edits, 2);
        assert_eq!(budget.spawns, 1);
    }

    #[test]
    fn sandboxed_configure_is_deterministic_and_capped() {
        let source = r#"
            fn configure(meta) {
                #{
                    traps: meta.difficulty,
                    terrain_edits: 2,
                    spawns: 1,
                    hints: #{ width: meta.width, label: "keep" }
                }
            }
        "#;
        let meta = MapMeta {
            width: 40,
            height: 20,
            enemy_count: 4,
            difficulty: 3,
        };
        let (budget, hints) = run_author_source(source, &meta).unwrap();
        let (again, hints_again) = run_author_source(source, &meta).unwrap();
        assert_eq!(budget.traps, again.traps);
        assert_eq!(budget.terrain_edits, 2);
        assert_eq!(budget.spawns, 1);
        assert_eq!(hints, hints_again);
        assert_eq!(hints["width"], serde_json::json!(40));

        let over = r#"
            fn configure(meta) {
                #{ traps: 100000, terrain_edits: 1, spawns: 1, hints: #{} }
            }
        "#;
        assert!(run_author_source(over, &meta).is_err());

        let attack = r#"
            fn configure(meta) { import "std"; meta.width }
        "#;
        assert!(run_author_source(attack, &meta).is_err());
    }

    #[test]
    fn author_path_rejects_traversal() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("modroot");
        std::fs::create_dir(&root).unwrap();
        let outside = temp.path().join("secret.rhai");
        std::fs::write(
            &outside,
            "fn configure(meta) { #{ traps: 1, terrain_edits: 1, spawns: 1 } }",
        )
        .unwrap();
        let meta = MapMeta {
            width: 1,
            height: 1,
            enemy_count: 1,
            difficulty: 1,
        };
        assert!(run_author_script_in_root(&root, &outside, &meta).is_err());
        assert!(
            run_author_script_in_root(&root, std::path::Path::new("../secret.rhai"), &meta)
                .is_err()
        );
    }

    #[test]
    fn test_map_meta_different_difficulties() {
        for difficulty in 1..=5 {
            let meta = MapMeta {
                width: 100 + difficulty * 10,
                height: 100 + difficulty * 10,
                enemy_count: difficulty * 5,
                difficulty,
            };
            assert_eq!(meta.difficulty, difficulty);
            assert!(meta.width > 100);
            assert!(meta.enemy_count >= 5);
        }
    }
}
