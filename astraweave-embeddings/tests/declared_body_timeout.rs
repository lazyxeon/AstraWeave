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

#[tokio::test]
async fn embed_batch_fails_when_the_declared_body_never_arrives() {
    // This test process is the only reader. The client copies the value at build.
    std::env::set_var("OLLAMA_TIMEOUT_SECS", "1");
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
