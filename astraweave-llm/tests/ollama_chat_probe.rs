#![cfg(feature = "ollama")]

//! Closed-port checks for `OllamaChatClient`.
//!
//! `CARGO_BIN_EXE_ollama_chat_probe` is an absolute path on the machine that
//! compiled the test. Windows partition jobs only have the nextest archive, so
//! profile `ci-pr-archive` skips this target and the archive build job runs it.

/// `CARGO_BIN_EXE_*` is absolute and already ends in `.exe` on Windows.
/// If a toolchain omits that suffix, try it before giving up.
fn probe_exe(compiled: &str) -> std::path::PathBuf {
    let path = std::path::PathBuf::from(compiled);
    if path.is_file() {
        return path;
    }
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
    {
        return path;
    }
    let mut exe = path.into_os_string();
    exe.push(".exe");
    std::path::PathBuf::from(exe)
}

fn ollama_chat_probe(mode: &str) -> std::process::Output {
    // The default client is low-latency, so `complete` skips the non-stream
    // attempts and reads `OLLAMA_TIMEOUT_SECS`. `Command::env` does not call `set_var`.
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    std::process::Command::new(&exe)
        .arg(mode)
        .env("OLLAMA_LOW_LATENCY", "1")
        .env("OLLAMA_TIMEOUT_SECS", "1")
        .output()
        .unwrap_or_else(|err| panic!("spawn {}: {err}", exe.display()))
}

#[test]
fn test_ollama_client_connection_error() {
    let out = ollama_chat_probe("complete");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "closed port should fail: {stdout} {stderr}"
    );
    let err = stdout.to_lowercase();
    assert!(
        err.contains("error") || err.contains("fail") || err.contains("timeout"),
        "{stdout} {stderr}"
    );
}

#[test]
fn test_ollama_batch_execution() {
    let out = ollama_chat_probe("batch");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "closed port should fail: {stdout} {stderr}"
    );
}
