use std::path::PathBuf;
use std::process::Command;

// `env!("CARGO_BIN_EXE_aw_trace_sync")` is an absolute path on the machine that
// compiled the test. Partition jobs run an extracted nextest archive, so that
// path is not there (`NotFound` on Linux and Windows). Nextest sets
// `NEXTEST_BIN_EXE_aw_trace_sync` to the binary inside the archive. `cargo test`
// does not set it, and the compile-time path is the local one.
fn trace_sync_exe() -> PathBuf {
    std::env::var_os("NEXTEST_BIN_EXE_aw_trace_sync")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_aw_trace_sync")))
}

#[test]
fn check_prints_the_in_sync_counts() {
    let root =
        std::fs::canonicalize(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .expect("repo root");
    let output = Command::new(trace_sync_exe())
        .arg("--check")
        .arg("--root")
        .arg(&root)
        .output()
        .expect("spawn aw_trace_sync");
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "status {:?}\n{err}",
        output.status.code()
    );
    assert!(err.contains("0 node diffs, 0 edge diffs"), "{err}");
}

fn write_drifted_repo(dir: &std::path::Path) {
    let arch = dir.join("docs/architecture");
    std::fs::create_dir_all(&arch).unwrap();
    std::fs::create_dir_all(dir.join("crates/demo/src")).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/demo\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("crates/demo/Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("crates/demo/src/lib.rs"), "pub fn demo() {}\n").unwrap();
    std::fs::write(
        dir.join("CLAUDE.md"),
        "<!-- TRACE-TABLE:START -->\n<!-- TRACE-TABLE:END -->\n",
    )
    .unwrap();
    std::fs::write(
        arch.join("demo.md"),
        "---\nschema_version: 1\ntrace_id: demo\ntitle: Demo\ndescription: Demo trace\n\
primary_crate: demo\ndomain: core\nlifecycle_status: active\nintegration_status: wired\n\
owns: [demo]\ndoc_version: \"1.0\"\nlast_verified_commit: abc1234\n---\n\
<a id=\"5-active-file-map\"></a>\n<a id=\"6-conflict-map\"></a>\n\
<a id=\"7-decision-log\"></a>\n<a id=\"8-invariants\"></a>\n\
<a id=\"11-open-questions\"></a>\n",
    )
    .unwrap();
    std::fs::write(
        arch.join("workspace_map.html"),
        "<span><!-- MAP-TRACE-COUNT:START -->x<!-- MAP-TRACE-COUNT:END --></span>\n\
<span><!-- MAP-COUNTER:START -->x<!-- MAP-COUNTER:END --></span>\n\
<!-- MAP-MEMBERS:START -->x<!-- MAP-MEMBERS:END -->\n\
<!-- TRACE-FOOTER:START -->x<!-- TRACE-FOOTER:END -->\n\
'examples_collapsed': 'stale',\n\
<a href=\"#5-active-file-map\"></a><a href=\"#6-conflict-map\"></a>\
<a href=\"#7-decision-log\"></a><a href=\"#8-invariants\"></a>\
<a href=\"#11-open-questions\"></a>\n\
<script id=\"workspace-data\">{\"nodes\":[],\"edges\":[],\"domains\":[{\"id\":\"core\"}]}</script>\n",
    )
    .unwrap();
}

#[test]
fn check_reports_a_known_missing_node() {
    let dir = tempfile::tempdir().unwrap();
    write_drifted_repo(dir.path());
    let output = Command::new(trace_sync_exe())
        .arg("--check")
        .arg("--root")
        .arg(dir.path())
        .env("CARGO_NET_OFFLINE", "true")
        .output()
        .expect("spawn aw_trace_sync");
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "status {:?}\n{err}",
        output.status.code()
    );
    assert!(err.contains("missing node demo"), "{err}");
    // The line diff is printed only for CLAUDE.md. The map drifts in the same
    // run, so a flipped comparison still prints some first-change block.
    assert!(err.contains("DRIFT: CLAUDE.md would change."), "{err}");
    assert!(err.contains("| Trace | Subsystem |"), "{err}");
}

#[test]
fn list_untraced_skips_examples_and_owned_crates() {
    let root =
        std::fs::canonicalize(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .expect("repo root");
    let output = Command::new(trace_sync_exe())
        .arg("--list-untraced")
        .arg("--root")
        .arg(&root)
        .env("CARGO_NET_OFFLINE", "true")
        .output()
        .expect("spawn aw_trace_sync");
    let out = String::from_utf8_lossy(&output.stdout);
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "status {:?}\n{err}\n{out}",
        output.status.code()
    );
    assert!(out.contains("examples excluded"), "{out}");
    assert!(
        !out.lines().any(|line| line.trim() == "hello_companion"),
        "{out}"
    );
    assert!(
        !out.lines().any(|line| line.trim() == "astraweave-core"),
        "{out}"
    );
}
