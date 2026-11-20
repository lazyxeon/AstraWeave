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
