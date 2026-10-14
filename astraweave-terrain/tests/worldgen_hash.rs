//! `worldgen_columns` case for `.github/workflows/determinism-replay.yml`.
//!
//! Appends one row to `target/replay-hash.json`. The core replay test writes
//! the other rows first; this test keeps those rows and replaces only its own.

use std::path::PathBuf;

use astraweave_terrain::column_gen::worldgen_record;
use sha2::{Digest, Sha256};

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
        return PathBuf::from(dir).join("replay-hash.json");
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/replay-hash.json")
}

#[test]
fn worldgen_columns_hash_is_stable_and_published() {
    let bytes_a = worldgen_record();
    let bytes_b = worldgen_record();
    assert_eq!(
        bytes_a, bytes_b,
        "worldgen record diverged inside one process"
    );
    let digest = sha256_hex(&bytes_a);
    assert_eq!(digest.len(), 64);

    let path = replay_hash_json_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create replay-hash directory");
    }
    let mut rows: Vec<serde_json::Value> = if path.exists() {
        let text = std::fs::read_to_string(&path).expect("read replay-hash json");
        serde_json::from_str(&text).unwrap_or_else(|_| Vec::new())
    } else {
        Vec::new()
    };
    rows.retain(|row| row.get("case").and_then(|value| value.as_str()) != Some("worldgen_columns"));
    rows.push(serde_json::json!({ "case": "worldgen_columns", "sha256": digest }));
    rows.sort_by(|left, right| {
        left.get("case")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .cmp(
                right
                    .get("case")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
            )
    });
    let body = serde_json::to_vec_pretty(&rows).expect("encode replay-hash json");
    std::fs::write(&path, body).expect("write replay-hash json");
    println!("replay-hash worldgen_columns {digest}");
    println!("wrote {}", path.display());
}

#[test]
fn pine_forest_manifest_matches_downloaded_bytes() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo.join("assets/terrain/polyhaven/pine_forest/manifest.json"))
            .expect("pine forest manifest"),
    )
    .expect("parse manifest");
    let blob = serde_json::to_string(&manifest).expect("manifest text");
    assert!(
        !blob.to_ascii_lowercase().contains("assetstore.unity"),
        "Unity Asset Store URLs are excluded"
    );
    let assets = manifest["assets"].as_array().expect("assets");
    let mut roles = Vec::new();
    for asset in assets {
        let role = asset["role"].as_str().expect("role");
        roles.push(role.to_string());
        assert_eq!(asset["license"].as_str(), Some("CC0"), "{role} license");
        let source = asset["source_url"].as_str().expect("source_url");
        assert!(source.starts_with("https://"), "{role} source");
        for file in asset["files"].as_array().expect("files") {
            let rel = file["path"].as_str().expect("path");
            let bytes = std::fs::read(repo.join(rel)).unwrap_or_else(|err| {
                panic!("missing {rel}: {err}");
            });
            let digest = sha256_hex(&bytes);
            assert_eq!(digest, file["sha256"].as_str().unwrap(), "{rel} hash drifted");
        }
        if let Some(remote) = asset.get("remote_files").and_then(|value| value.as_array()) {
            for file in remote {
                assert_eq!(file["vendored"], false, "{role} remote file");
                assert_eq!(file["sha256"].as_str().unwrap().len(), 64);
            }
        }
    }
    for required in [
        "forest_floor",
        "aerial_rocks_01",
        "coast_land_rocks_01",
        "leafy_grass",
        "forest_leaves_02",
        "broadleaf_tree",
        "island_tree",
        "dead_branches",
        "conifer",
        "opening_sky_ibl",
        "greybox_forest_props",
    ] {
        assert!(roles.iter().any(|role| role == required), "missing {required}");
    }
    assert_eq!(
        manifest["conifer"]["source_url"].as_str(),
        Some("https://polyhaven.com/a/pine_tree_01")
    );
    assert_eq!(
        manifest["conifer"]["license_url"].as_str(),
        Some("https://polyhaven.com/license")
    );
}
