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
    let mut with_suffix = path.clone().into_os_string();
    with_suffix.push(".exe");
    let exe = std::path::PathBuf::from(with_suffix);
    if exe.is_file() {
        exe
    } else {
        path
    }
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

fn read_http_headers(sock: &mut std::net::TcpStream) {
    use std::io::Read;
    let _ = sock.set_read_timeout(Some(std::time::Duration::from_secs(2)));
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    while buf.len() < 8192 {
        match sock.read(&mut byte) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if buf.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
        }
    }
}

fn write_chunk(sock: &mut std::net::TcpStream, data: &str) {
    use std::io::Write;
    let _ = write!(sock, "{:X}\r\n{data}\r\n", data.len());
    let _ = sock.flush();
}

/// NDJSON chat chunks, one every 50 ms. `hang_after` stops writing and leaves
/// the socket open so the client must idle out.
fn serve_slow_chunks(count: usize, hang_after: Option<usize>) -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let Ok((mut sock, _)) = listener.accept() else {
            return;
        };
        read_http_headers(&mut sock);
        use std::io::Write;
        let _ = sock.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
        );
        let _ = sock.flush();
        let stop = hang_after.unwrap_or(count);
        for i in 0..count.min(stop) {
            write_chunk(
                &mut sock,
                &format!("{{\"message\":{{\"content\":\"n{i:02}\"}},\"done\":false}}\n"),
            );
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        if hang_after.is_none() {
            write_chunk(
                &mut sock,
                "{\"message\":{\"content\":\"\"},\"done\":true}\n",
            );
            let _ = sock.write_all(b"0\r\n\r\n");
            let _ = sock.flush();
        } else {
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
    });
    addr
}

fn run_against(
    mode: &str,
    addr: std::net::SocketAddr,
) -> (std::time::Duration, std::process::Output) {
    let started = std::time::Instant::now();
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let out = std::process::Command::new(&exe)
        .args([mode, &format!("http://{addr}")])
        .env("OLLAMA_LOW_LATENCY", "1")
        .env("OLLAMA_TIMEOUT_SECS", "2")
        .env("OLLAMA_EARLY_EXIT", "0")
        .output()
        .unwrap_or_else(|err| panic!("spawn {}: {err}", exe.display()));
    (started.elapsed(), out)
}

#[test]
fn slow_chunks_longer_than_the_timeout_are_kept() {
    let addr = serve_slow_chunks(60, None);
    let (elapsed, out) = run_against("complete", addr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "slow stream failed after {:?}: {stdout} {stderr}",
        elapsed
    );
    for i in 0..60 {
        assert!(
            stdout.contains(&format!("n{i:02}")),
            "dropped token n{i:02} after {:?}: {stdout}",
            elapsed
        );
    }
    assert!(
        elapsed < std::time::Duration::from_secs(6),
        "stream spun for {elapsed:?}"
    );

    let addr = serve_slow_chunks(60, None);
    let (elapsed, out) = run_against("qwen-stream", addr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "qwen stream failed after {:?}: {stdout} {stderr}",
        elapsed
    );
    for i in 0..60 {
        assert!(
            stdout.contains(&format!("n{i:02}")),
            "qwen dropped token n{i:02} after {:?}: {stdout}",
            elapsed
        );
    }
}

/// Accept the TCP connection, read the request, and never send headers.
/// `first_not_found` answers `/api/chat` with 404 so the chat client falls
/// through to `/api/generate`.
fn serve_no_headers(first_not_found: bool) -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let mut first = true;
        loop {
            let Ok((mut sock, _)) = listener.accept() else {
                return;
            };
            read_http_headers(&mut sock);
            if first && first_not_found {
                first = false;
                use std::io::Write;
                let _ = sock.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
                let _ = sock.flush();
                continue;
            }
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
    });
    addr
}

fn assert_headers_fail_near_timeout(mode: &str, first_not_found: bool) {
    let addr = serve_no_headers(first_not_found);
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = std::process::Command::new(&exe)
        .args([mode, &format!("http://{addr}")])
        .env("OLLAMA_LOW_LATENCY", "1")
        .env("OLLAMA_TIMEOUT_SECS", "1")
        .env("OLLAMA_EARLY_EXIT", "0")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn {}: {err}", exe.display()));
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(4) {
            let _ = child.kill();
            panic!(
                "{mode} still waiting for headers after {:?}",
                started.elapsed()
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let elapsed = started.elapsed();
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!status.success(), "{mode} succeeded: {stdout} {stderr}");
    assert!(
        elapsed < std::time::Duration::from_secs(3),
        "{mode} took {elapsed:?}: {stdout} {stderr}"
    );
    let msg = format!("{stdout} {stderr}").to_lowercase();
    assert!(
        msg.contains("headers")
            || msg.contains("timeout")
            || msg.contains("timed out")
            || msg.contains("error sending request"),
        "{mode}: {stdout} {stderr}"
    );
}

/// Headers arrive, `Content-Length: 64` is declared, and no body bytes follow.
fn serve_declared_body(status_line: &'static str) -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let Ok((mut sock, _)) = listener.accept() else {
            return;
        };
        read_http_headers(&mut sock);
        use std::io::Write;
        let _ = sock.write_all(
            format!("{status_line}\r\nContent-Length: 64\r\nConnection: close\r\n\r\n").as_bytes(),
        );
        let _ = sock.flush();
        std::thread::sleep(std::time::Duration::from_secs(30));
    });
    addr
}

fn assert_body_fails_near_timeout(
    mode: &str,
    status_line: &'static str,
    low_latency: &str,
    needles: &[&str],
    absent: &[&str],
) {
    let addr = serve_declared_body(status_line);
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = std::process::Command::new(&exe)
        .args([mode, &format!("http://{addr}")])
        .env("OLLAMA_LOW_LATENCY", low_latency)
        .env("OLLAMA_TIMEOUT_SECS", "1")
        .env("OLLAMA_EARLY_EXIT", "0")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn {}: {err}", exe.display()));
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(4) {
            let _ = child.kill();
            panic!(
                "{mode} still reading a missing body after {:?}",
                started.elapsed()
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let elapsed = started.elapsed();
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!status.success(), "{mode} succeeded: {stdout} {stderr}");
    assert!(
        elapsed < std::time::Duration::from_secs(3),
        "{mode} took {elapsed:?}: {stdout} {stderr}"
    );
    let msg = format!("{stdout} {stderr}").to_lowercase();
    for needle in needles {
        assert!(
            msg.contains(needle),
            "{mode} missing {needle}: {stdout} {stderr}"
        );
    }
    for needle in absent {
        assert!(
            !msg.contains(needle),
            "{mode} still contains {needle}: {stdout} {stderr}"
        );
    }
}

#[test]
fn declared_body_that_never_arrives_fails_near_the_read_timeout() {
    assert_body_fails_near_timeout(
        "complete",
        "HTTP/1.1 500 Internal Server Error",
        "1",
        &["body read failed", "operation timed out"],
        &[],
    );
    assert_body_fails_near_timeout(
        "complete",
        "HTTP/1.1 500 Internal Server Error",
        "0",
        &["body read failed", "operation timed out"],
        &["initiating streaming post"],
    );
    assert_body_fails_near_timeout(
        "complete",
        "HTTP/1.1 200 OK",
        "1",
        &["operation timed out"],
        &[],
    );
    assert_body_fails_near_timeout(
        "complete",
        "HTTP/1.1 200 OK",
        "0",
        &["body read failed", "operation timed out"],
        &["initiating streaming post"],
    );
    assert_body_fails_near_timeout(
        "warmup",
        "HTTP/1.1 200 OK",
        "1",
        &["body read failed", "operation timed out"],
        &[],
    );
    for mode in ["qwen-think", "qwen-health", "phi-health", "hermes-health"] {
        assert_body_fails_near_timeout(mode, "HTTP/1.1 200 OK", "1", &["operation timed out"], &[]);
    }
    assert_body_fails_near_timeout(
        "qwen-preload",
        "HTTP/1.1 200 OK",
        "1",
        &["body read failed", "operation timed out"],
        &[],
    );
}

/// A readable error body must be the `complete` result. `OLLAMA_LOW_LATENCY=0`
/// used to log it and then start the streaming POST.
fn serve_readable_error(body: &'static str) -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let Ok((mut sock, _)) = listener.accept() else {
            return;
        };
        read_http_headers(&mut sock);
        use std::io::Write;
        let _ = sock.write_all(
            format!(
                "HTTP/1.1 500 Internal Server Error\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .as_bytes(),
        );
        let _ = sock.flush();
    });
    addr
}

#[test]
fn readable_error_status_does_not_start_streaming() {
    let addr = serve_readable_error("nope");
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = std::process::Command::new(&exe)
        .args(["complete", &format!("http://{addr}")])
        .env("OLLAMA_LOW_LATENCY", "0")
        .env("OLLAMA_TIMEOUT_SECS", "1")
        .env("OLLAMA_EARLY_EXIT", "0")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn {}: {err}", exe.display()));
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(4) {
            let _ = child.kill();
            panic!(
                "complete still running after a readable 500: {:?}",
                started.elapsed()
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let msg = format!("{stdout} {stderr}").to_lowercase();
    assert!(!status.success(), "complete succeeded: {stdout} {stderr}");
    assert!(
        msg.contains("nope") && msg.contains("500"),
        "missing the error body: {stdout} {stderr}"
    );
    assert!(
        !msg.contains("initiating streaming post"),
        "readable 500 started streaming: {stdout} {stderr}"
    );
}

fn serve_oversized_ok() -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let Ok((mut sock, _)) = listener.accept() else {
            return;
        };
        read_http_headers(&mut sock);
        let body = vec![b'a'; astraweave_llm::MAX_MODEL_STREAM_BYTES + 8];
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        write_and_hold(&mut sock, header.as_bytes(), &body);
    });
    addr
}

/// Finish the response without dropping the socket. Windows turns a drop
/// with unread data into a connection reset before the cap can fire.
fn write_and_hold(sock: &mut std::net::TcpStream, header: &[u8], body: &[u8]) {
    use std::io::Write;
    let _ = sock.write_all(header);
    let _ = sock.write_all(body);
    let _ = sock.flush();
    let _ = sock.shutdown(std::net::Shutdown::Write);
    std::thread::sleep(std::time::Duration::from_secs(2));
}

#[test]
fn oversized_chat_body_stops_at_64kib() {
    let addr = serve_oversized_ok();
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = std::process::Command::new(&exe)
        .args(["complete", &format!("http://{addr}")])
        .env("OLLAMA_LOW_LATENCY", "1")
        .env("OLLAMA_TIMEOUT_SECS", "5")
        .env("OLLAMA_EARLY_EXIT", "0")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn {}: {err}", exe.display()));
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(4) {
            let _ = child.kill();
            panic!("complete kept reading past the stream budget");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !status.success(),
        "oversized body succeeded: {stdout} {stderr}"
    );
    assert!(
        format!("{stdout} {stderr}").contains("model stream exceeds"),
        "{stdout} {stderr}"
    );
}

fn serve_empty_lines() -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let Ok((mut sock, _)) = listener.accept() else {
            return;
        };
        read_http_headers(&mut sock);
        // Empty NDJSON lines do not grow the kept text. The byte counter must
        // still stop the read.
        let body = vec![b'\n'; astraweave_llm::MAX_MODEL_STREAM_BYTES + 8];
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        write_and_hold(&mut sock, header.as_bytes(), &body);
    });
    addr
}

#[test]
fn empty_ndjson_lines_stop_at_64kib() {
    let addr = serve_empty_lines();
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = std::process::Command::new(&exe)
        .args(["complete", &format!("http://{addr}")])
        .env("OLLAMA_LOW_LATENCY", "1")
        .env("OLLAMA_TIMEOUT_SECS", "5")
        .env("OLLAMA_EARLY_EXIT", "0")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn {}: {err}", exe.display()));
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(4) {
            let _ = child.kill();
            panic!("empty NDJSON lines were still streaming");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !status.success(),
        "empty lines succeeded: {stdout} {stderr}"
    );
    assert!(
        format!("{stdout} {stderr}").contains("model stream exceeds"),
        "{stdout} {stderr}"
    );
}

fn hermes_token_line(token: &str, done: bool) -> String {
    format!(
        r#"{{"model":"adrienbrault/nous-hermes2pro:Q4_K_M","created_at":"2026-10-23T00:00:00.000000000Z","message":{{"role":"assistant","content":"{token}"}},"done":{done}}}"#
    )
}

fn serve_512_token_reply() -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let Ok((mut sock, _)) = listener.accept() else {
            return;
        };
        read_http_headers(&mut sock);
        let mut body = String::new();
        for i in 0..512 {
            let done = i == 511;
            body.push_str(&hermes_token_line(&format!("t{i:03}"), done));
            body.push('\n');
        }
        let bytes = body.into_bytes();
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            bytes.len()
        );
        write_and_hold(&mut sock, header.as_bytes(), &bytes);
    });
    addr
}

#[test]
fn a_512_token_reply_is_kept() {
    let sample = hermes_token_line("t000", false);
    assert!(
        sample.len() >= 140 && sample.len() <= 180,
        "envelope drifted from a Hermes line: {}",
        sample.len()
    );
    assert!(
        512 * sample.len() > 64 * 1024,
        "512 tokens fit in the old 64 KiB raw cap"
    );
    assert!(512 * (sample.len() + 1) < astraweave_llm::MAX_MODEL_STREAM_BYTES);
    let addr = serve_512_token_reply();
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = std::process::Command::new(&exe)
        .args(["complete", &format!("http://{addr}")])
        .env("OLLAMA_LOW_LATENCY", "1")
        .env("OLLAMA_TIMEOUT_SECS", "5")
        .env("OLLAMA_EARLY_EXIT", "0")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn {}: {err}", exe.display()));
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(8) {
            let _ = child.kill();
            panic!("512-token reply was still open");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        status.success(),
        "512-token reply was capped: {stdout} {stderr}"
    );
    assert!(stdout.contains("t000"), "missing first token: {stdout}");
    assert!(stdout.contains("t511"), "missing last token: {stdout}");
}

fn serve_bad_utf8_line() -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let Ok((mut sock, _)) = listener.accept() else {
            return;
        };
        read_http_headers(&mut sock);
        let body = b"\xff\n";
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        write_and_hold(&mut sock, header.as_bytes(), body);
    });
    addr
}

#[test]
fn invalid_utf8_line_is_an_error() {
    let addr = serve_bad_utf8_line();
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = std::process::Command::new(&exe)
        .args(["complete", &format!("http://{addr}")])
        .env("OLLAMA_LOW_LATENCY", "1")
        .env("OLLAMA_TIMEOUT_SECS", "2")
        .env("OLLAMA_EARLY_EXIT", "0")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn {}: {err}", exe.display()));
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(4) {
            let _ = child.kill();
            panic!("invalid utf-8 line was still open");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !status.success(),
        "invalid utf-8 succeeded: {stdout} {stderr}"
    );
    assert!(
        format!("{stdout} {stderr}")
            .to_lowercase()
            .contains("utf-8"),
        "{stdout} {stderr}"
    );
}

fn serve_good_line_then_bad_utf8() -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let Ok((mut sock, _)) = listener.accept() else {
            return;
        };
        read_http_headers(&mut sock);
        let body = b"{\"message\":{\"content\":\"hello\"}}\n\xff\n";
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        write_and_hold(&mut sock, header.as_bytes(), body);
    });
    addr
}

#[test]
fn good_line_before_bad_utf8_is_kept() {
    let addr = serve_good_line_then_bad_utf8();
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = std::process::Command::new(&exe)
        .args(["complete", &format!("http://{addr}")])
        .env("OLLAMA_LOW_LATENCY", "1")
        .env("OLLAMA_TIMEOUT_SECS", "2")
        .env("OLLAMA_EARLY_EXIT", "0")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn {}: {err}", exe.display()));
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(4) {
            let _ = child.kill();
            panic!("good line then bad utf-8 was still open");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        status.success(),
        "good token was dropped: {stdout} {stderr}"
    );
    assert!(
        stdout.contains("hello"),
        "kept token missing: {stdout} {stderr}"
    );
}

#[test]
fn silent_accept_fails_near_the_header_timeout() {
    assert_headers_fail_near_timeout("complete", false);
    assert_headers_fail_near_timeout("complete", true);
    assert_headers_fail_near_timeout("qwen-stream", false);
    assert_headers_fail_near_timeout("hermes-stream", false);
}

#[test]
fn idle_timeout_keeps_partial_chunks() {
    let addr = serve_slow_chunks(5, Some(5));
    let (elapsed, out) = run_against("complete", addr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "idle wait spun for {elapsed:?}: {stdout} {stderr}"
    );
    assert!(
        out.status.success(),
        "partial stream failed: {stdout} {stderr}"
    );
    for i in 0..5 {
        assert!(
            stdout.contains(&format!("n{i:02}")),
            "dropped partial token n{i:02}: {stdout}"
        );
    }
}
