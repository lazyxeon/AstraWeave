#![cfg(not(feature = "visual"))]

use std::path::PathBuf;
use std::process::Command;

/// Console `hello_companion` as nextest extracted it, or the compile-time path.
///
/// `env!("CARGO_BIN_EXE_hello_companion")` is an absolute path on the machine
/// that compiled the test. Pull-request partitions run from the nextest
/// archive, which does not have that directory. Nextest puts the console
/// binary in the archive and sets `CARGO_BIN_EXE_hello_companion` and
/// `NEXTEST_BIN_EXE_hello_companion` to the extracted file. `cargo test` on
/// this toolchain does not set those, so the compile-time path is the fallback
/// when it still names a file.
fn console_binary() -> PathBuf {
    for key in [
        "CARGO_BIN_EXE_hello_companion",
        "NEXTEST_BIN_EXE_hello_companion",
    ] {
        let Some(raw) = std::env::var_os(key) else {
            continue;
        };
        if raw.is_empty() {
            continue;
        }
        if let Some(path) = existing_exe(PathBuf::from(raw)) {
            return path;
        }
    }
    PathBuf::from(env!("CARGO_BIN_EXE_hello_companion"))
}

/// `CARGO_BIN_EXE_*` already ends in `.exe` on Windows. Try that suffix when a
/// toolchain omits it.
fn existing_exe(path: PathBuf) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path);
    }
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
    {
        return None;
    }
    let mut with_suffix = path.into_os_string();
    with_suffix.push(".exe");
    let exe = PathBuf::from(with_suffix);
    if exe.is_file() {
        Some(exe)
    } else {
        None
    }
}

#[test]
fn console_demo_prints_the_banner() {
    let exe = console_binary();
    let output = Command::new(&exe).output().unwrap_or_else(|err| {
        panic!("hello_companion starts at {}: {err}", exe.display());
    });
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "status {}\n{stdout}\n{stderr}",
        output.status
    );
    assert!(stdout.contains("AstraWeave AI Companion Demo"), "{stdout}");
}
