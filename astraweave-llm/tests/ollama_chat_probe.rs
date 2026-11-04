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
///
/// `TCP_NODELAY` keeps macOS delayed ACK from holding a flushed chunk. On a
/// loaded runner each `sleep` still wakes late. The client keeps those
/// chunks when every gap stays inside the idle window. Silence still stops
/// after one idle period.
fn serve_slow_chunks(count: usize, hang_after: Option<usize>) -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let Ok((mut sock, _)) = listener.accept() else {
            return;
        };
        sock.set_nodelay(true).expect("nodelay");
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
        elapsed > std::time::Duration::from_secs(2),
        "stream finished inside the idle timeout ({elapsed:?})"
    );
    // macOS Test full delivered about thirty of these tokens in 4.01 s, so
    // sixty of them need more than the old 6 s ceiling. Fifteen seconds still
    // fails a client that waits out the 30 s harness hang or the default
    // 180 s timeout. A missing token fails above, on its own.
    assert!(
        elapsed < std::time::Duration::from_secs(15),
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
    assert!(
        elapsed > std::time::Duration::from_secs(2),
        "qwen stream finished inside the idle timeout ({elapsed:?})"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(15),
        "qwen stream spun for {elapsed:?}"
    );
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

fn serve_body(body: Vec<u8>) -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let Ok((mut sock, _)) = listener.accept() else {
            return;
        };
        read_http_headers(&mut sock);
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        write_and_hold(&mut sock, header.as_bytes(), &body);
    });
    addr
}

fn run_complete(addr: std::net::SocketAddr, early_exit: &str) -> std::process::Output {
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = std::process::Command::new(&exe)
        .args(["complete", &format!("http://{addr}")])
        .env("OLLAMA_LOW_LATENCY", "1")
        .env("OLLAMA_TIMEOUT_SECS", "5")
        .env("OLLAMA_EARLY_EXIT", early_exit)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn {}: {err}", exe.display()));
    // Drain while the child runs. A 64 KiB success fills the pipe and blocks
    // if the parent waits to read until after the child exits.
    let mut stdout = child.stdout.take().expect("stdout");
    let mut stderr = child.stderr.take().expect("stderr");
    let stdout_task = std::thread::spawn(move || {
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut stdout, &mut buf).expect("read stdout");
        buf
    });
    let stderr_task = std::thread::spawn(move || {
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut stderr, &mut buf).expect("read stderr");
        buf
    });
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(4) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_task.join();
            let _ = stderr_task.join();
            panic!("complete was still open after 4s");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    std::process::Output {
        status,
        stdout: stdout_task.join().expect("stdout thread"),
        stderr: stderr_task.join().expect("stderr thread"),
    }
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
fn oversized_chat_body_stops_at_the_stream_budget() {
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
fn empty_ndjson_lines_stop_at_the_stream_budget() {
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

#[test]
fn plain_text_over_64kib_is_rejected() {
    for early in ["0", "1"] {
        let addr = serve_body(vec![b'a'; 80 * 1024]);
        let output = run_complete(addr, early);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "80 KiB plain text succeeded with early_exit={early}: {stdout} {stderr}"
        );
        assert!(
            format!("{stdout} {stderr}").contains("64 KiB"),
            "early_exit={early}: {stdout} {stderr}"
        );
        assert!(
            stdout.len() < 4 * 1024,
            "80 KiB plain text was returned with early_exit={early}: {} bytes",
            stdout.len()
        );
    }
}

#[test]
fn plain_text_of_exactly_64kib_is_kept() {
    let addr = serve_body(vec![b'a'; astraweave_llm::MAX_MODEL_JSON_BYTES]);
    let output = run_complete(addr, "1");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "exactly 64 KiB was rejected: {stderr} stdout_len={}",
        stdout.len()
    );
    let body = stdout.rsplit_once('\n').expect("log line").1;
    assert_eq!(body.len(), astraweave_llm::MAX_MODEL_JSON_BYTES, "{stderr}");
    assert!(body.bytes().all(|byte| byte == b'a'), "{stderr}");
}

#[test]
fn message_content_over_64kib_without_a_newline_is_rejected() {
    for early in ["0", "1"] {
        let mut body = br#"{"message":{"content":""#.to_vec();
        body.extend(std::iter::repeat(b'q').take(70 * 1024));
        body.extend(br#""}}"#);
        assert!(!body.contains(&b'\n'));
        let addr = serve_body(body);
        let output = run_complete(addr, early);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "70 KiB content succeeded with early_exit={early}: {} bytes {stderr}",
            stdout.len()
        );
        assert!(
            format!("{stdout} {stderr}").contains("64 KiB"),
            "early_exit={early}: {stdout} {stderr}"
        );
        assert!(
            stdout.len() < 4 * 1024,
            "70 KiB content was returned with early_exit={early}: {} bytes",
            stdout.len()
        );
    }
}

#[test]
fn a_mebibyte_of_newlines_finishes() {
    let addr = serve_body(vec![b'\n'; astraweave_llm::MAX_MODEL_STREAM_BYTES]);
    let started = std::time::Instant::now();
    let output = run_complete(addr, "0");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(4),
        "newline drain took {:?}",
        started.elapsed()
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "newline body succeeded: {stdout} {stderr}"
    );
    assert!(
        !format!("{stdout} {stderr}").contains("model stream exceeds"),
        "the inclusive raw budget rejected an exact 1 MiB body: {stdout} {stderr}"
    );
    assert!(stdout.len() < 4 * 1024, "{}", stdout.len());
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

fn run_until(
    mode: &str,
    addr: std::net::SocketAddr,
    idle_secs: &str,
    lifetime_ms: &str,
    kill_after: std::time::Duration,
) -> (std::time::Duration, std::process::Output) {
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = std::process::Command::new(&exe)
        .args([mode, &format!("http://{addr}")])
        .env("OLLAMA_LOW_LATENCY", "1")
        .env("OLLAMA_TIMEOUT_SECS", idle_secs)
        .env("OLLAMA_STREAM_LIFETIME_MS", lifetime_ms)
        .env("OLLAMA_EARLY_EXIT", "0")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn {}: {err}", exe.display()));
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > kill_after {
            let _ = child.kill();
            panic!("{mode} still running after {kill_after:?}");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let elapsed = started.elapsed();
    let mut output = child.wait_with_output().expect("output");
    output.status = status;
    (elapsed, output)
}

/// Headers arrive and the declared body never does.
/// `first_not_found` answers `/api/chat` with an empty 404 so `complete`
/// reads the withheld body from `/api/generate`.
fn serve_withheld_body(status_line: &'static str, first_not_found: bool) -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let mut first = true;
        loop {
            let Ok((mut sock, _)) = listener.accept() else {
                return;
            };
            read_http_headers(&mut sock);
            use std::io::Write;
            if first && first_not_found {
                first = false;
                let _ = sock.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
                let _ = sock.flush();
                continue;
            }
            let _ = write!(
                sock,
                "{status_line}\r\nContent-Length: 64\r\nConnection: close\r\n\r\n"
            );
            let _ = sock.flush();
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
    });
    addr
}

#[test]
fn withheld_error_body_ends_at_the_lifetime() {
    // Thinking mode reads the JSON body only after a success status, so
    // that path withholds a 200. `complete` withholds the 500, including
    // the `/api/generate` fallback.
    for (mode, status_line, first_not_found) in [
        ("complete", "HTTP/1.1 500 Internal Server Error", false),
        ("complete", "HTTP/1.1 500 Internal Server Error", true),
        ("qwen-think", "HTTP/1.1 200 OK", false),
    ] {
        let addr = serve_withheld_body(status_line, first_not_found);
        let (elapsed, out) = run_until(mode, addr, "20", "400", std::time::Duration::from_secs(3));
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "{mode} first_not_found={first_not_found} succeeded: {stdout} {stderr}"
        );
        assert!(
            elapsed >= std::time::Duration::from_millis(250),
            "{mode} returned before the lifetime ({elapsed:?}): {stdout} {stderr}"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "{mode} waited out the idle timeout ({elapsed:?}): {stdout} {stderr}"
        );
        let msg = format!("{stdout} {stderr}");
        assert!(msg.contains("streaming lifetime deadline"), "{mode}: {msg}");
        assert!(
            !msg.contains("operation timed out"),
            "{mode} used the idle read timeout: {msg}"
        );
    }
}

#[test]
fn withheld_headers_end_at_the_lifetime() {
    for (mode, first_not_found) in [
        ("complete", false),
        ("complete", true),
        ("hermes-stream", false),
        ("qwen-stream", false),
    ] {
        let addr = serve_no_headers(first_not_found);
        let (elapsed, out) = run_until(mode, addr, "20", "400", std::time::Duration::from_secs(3));
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "{mode} first_not_found={first_not_found} succeeded: {stdout} {stderr}"
        );
        assert!(
            elapsed >= std::time::Duration::from_millis(250),
            "{mode} returned before the lifetime ({elapsed:?}): {stdout} {stderr}"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "{mode} waited out the idle timeout ({elapsed:?}): {stdout} {stderr}"
        );
        let msg = format!("{stdout} {stderr}");
        assert!(msg.contains("streaming lifetime deadline"), "{mode}: {msg}");
    }
}

/// Headers and a token arrive after the lifetime. The call fails on the
/// header wait and does not accept the body.
fn serve_late_headers() -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let Ok((mut sock, _)) = listener.accept() else {
            return;
        };
        read_http_headers(&mut sock);
        std::thread::sleep(std::time::Duration::from_secs(4));
        use std::io::Write;
        let body = "{\"message\":{\"content\":\"late-token\"},\"done\":true}\n";
        let _ = write!(
            sock,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = sock.flush();
    });
    addr
}

#[test]
fn late_headers_fail_cleanly() {
    for mode in ["complete", "qwen-stream"] {
        let addr = serve_late_headers();
        let (elapsed, out) = run_until(mode, addr, "10", "300", std::time::Duration::from_secs(3));
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "{mode} accepted late headers: {stdout} {stderr}"
        );
        assert!(
            !stdout.contains("late-token"),
            "{mode} kept the late body: {stdout} {stderr}"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "{mode} waited for the late headers ({elapsed:?}): {stdout} {stderr}"
        );
        let msg = format!("{stdout} {stderr}");
        assert!(msg.contains("streaming lifetime deadline"), "{mode}: {msg}");
    }
}

#[test]
fn lifetime_after_a_token_returns_the_token() {
    let addr = serve_slow_chunks(1, Some(1));
    let (elapsed, out) = run_until(
        "complete",
        addr,
        "10",
        "400",
        std::time::Duration::from_secs(3),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "token before the lifetime was dropped: {stdout} {stderr}"
    );
    assert!(
        stdout.contains("n00"),
        "kept token missing: {stdout} {stderr}"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "lifetime after a token waited out the idle timeout ({elapsed:?})"
    );
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
