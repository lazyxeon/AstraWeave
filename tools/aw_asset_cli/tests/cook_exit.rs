//! `aw_asset_cli cook` exit status for jailed glTF URIs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const HOSTILE: &[&str] = &[
    "file:///etc/passwd",
    "file:/%2e%2e/x",
    "%2e%2e/x",
    "../x",
    "/abs",
    r"C:\x",
];

fn pipeline(dir: &Path) -> PathBuf {
    let src_dir = dir.join("src");
    let out_dir = dir.join("out");
    fs::create_dir_all(&src_dir).expect("src");
    let cfg = dir.join("pipeline.toml");
    let src_toml = src_dir.display().to_string().replace('\\', "/");
    let out_toml = out_dir.display().to_string().replace('\\', "/");
    fs::write(
        &cfg,
        format!(
            "source = \"{src_toml}\"\noutput = \"{out_toml}\"\ncompress = false\nvalidate = false\n\n[[rules]]\nkind = \"model\"\nglob = \"*.gltf\"\n"
        ),
    )
    .expect("toml");
    cfg
}

fn asset_cli() -> PathBuf {
    if let Some(path) = std::env::var_os("NEXTEST_BIN_EXE_aw_asset_cli") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_BIN_EXE_aw_asset_cli"))
}

fn run_cook(cfg: &Path) -> Output {
    Command::new(asset_cli())
        .arg("cook")
        .arg(cfg)
        .output()
        .expect("spawn cook")
}

fn json_string(uri: &str) -> String {
    let mut out = String::from("\"");
    for ch in uri.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[test]
fn cook_exits_nonzero_for_hostile_uris() {
    let root = std::env::temp_dir().join(format!("aw-cook-exit-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    for (index, uri) in HOSTILE.iter().enumerate() {
        let dir = root.join(index.to_string());
        let _ = fs::remove_dir_all(&dir);
        let cfg = pipeline(&dir);
        let body = format!(
            r#"{{"asset":{{"version":"2.0"}},"buffers":[{{"uri":{},"byteLength":4}}]}}"#,
            json_string(uri)
        );
        fs::write(dir.join("src").join("tri.gltf"), body).expect("gltf");
        let output = run_cook(&cfg);
        assert!(
            !output.status.success(),
            "{uri} exited {:?}",
            output.status.code()
        );
        assert_ne!(output.status.code(), Some(0), "{uri}");
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        )
        .to_lowercase();
        assert!(
            combined.contains("traversal") || combined.contains("rejected"),
            "{uri}: {combined}"
        );
        assert!(
            !dir.join("out").join("tri.meshbin").exists(),
            "{uri} wrote tri.meshbin"
        );
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn cook_exits_zero_for_a_relative_uri() {
    let dir = std::env::temp_dir().join(format!("aw-cook-ok-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let cfg = pipeline(&dir);
    fs::write(dir.join("src").join("box.bin"), b"box!").expect("bin");
    fs::write(
        dir.join("src").join("tri.gltf"),
        r#"{"asset":{"version":"2.0"},"buffers":[{"uri":"box.bin","byteLength":4}]}"#,
    )
    .expect("gltf");
    let output = run_cook(&cfg);
    assert!(
        output.status.success(),
        "relative uri exited {:?}\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let meshbin = dir.join("out").join("tri.meshbin");
    assert!(meshbin.is_file(), "missing {}", meshbin.display());
    let _ = fs::remove_dir_all(&dir);
}
