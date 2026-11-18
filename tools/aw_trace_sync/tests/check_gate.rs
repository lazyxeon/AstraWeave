use std::process::Command;

#[test]
fn check_prints_the_in_sync_counts() {
    let root =
        std::fs::canonicalize(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .expect("repo root");
    let output = Command::new(env!("CARGO_BIN_EXE_aw_trace_sync"))
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
