use std::process::Command;

#[test]
fn console_demo_prints_the_banner() {
    let exe = env!("CARGO_BIN_EXE_hello_companion");
    let output = Command::new(exe).output().expect("hello_companion starts");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "status {}\n{stdout}\n{stderr}",
        output.status
    );
    assert!(stdout.contains("AstraWeave AI Companion Demo"), "{stdout}");
}
