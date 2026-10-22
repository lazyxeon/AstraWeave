#![cfg(feature = "http")]

//! A 200 with `Content-Length: 64` and no body must not hang `embed_batch`.

use astraweave_embeddings::{EmbeddingClient, RemoteEmbeddingClient};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

fn serve_declared_body() -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    std::thread::spawn(move || {
        let Ok((mut sock, _)) = listener.accept() else {
            return;
        };
        let _ = sock.set_read_timeout(Some(Duration::from_secs(2)));
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
        let _ =
            sock.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 64\r\nConnection: close\r\n\r\n");
        let _ = sock.flush();
        std::thread::sleep(Duration::from_secs(30));
    });
    addr
}

fn run_declared_body_child() {
    let exe = std::env::current_exe().expect("current test executable");
    let mut child = std::process::Command::new(&exe)
        .args([
            "--exact",
            "embed_batch_fails_when_the_declared_body_never_arrives",
            "--test-threads=1",
        ])
        .env("AW_EMBED_DECLARED_BODY", "1")
        .env("OLLAMA_TIMEOUT_SECS", "1")
        .env_remove("NEXTEST")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn {}: {err}", exe.display()));
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > Duration::from_secs(4) {
            let _ = child.kill();
            panic!(
                "embed_batch still reading a missing body after {:?}",
                started.elapsed()
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        status.success(),
        "declared-body child failed: {stdout} {stderr}"
    );
}

#[tokio::test]
async fn embed_batch_fails_when_the_declared_body_never_arrives() {
    // The client copies `OLLAMA_TIMEOUT_SECS` at build. The parent sets it on
    // the child with `Command::env` and does not call `set_var`.
    if std::env::var_os("AW_EMBED_DECLARED_BODY").is_none() {
        run_declared_body_child();
        return;
    }
    let addr = serve_declared_body();
    let client = RemoteEmbeddingClient::new(format!("http://{addr}"), "m".into(), 8)
        .expect("loopback client");
    let started = Instant::now();
    let err = client
        .embed_batch(&["hello".into()])
        .await
        .expect_err("missing body");
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(3),
        "embed_batch took {elapsed:?}: {err}"
    );
    let msg = format!("{err:#}").to_lowercase();
    assert!(
        msg.contains("timeout") || msg.contains("timed out"),
        "{err:#}"
    );
}
