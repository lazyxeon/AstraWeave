//! W.3 Family-1: end-to-end tests of the SHIPPED `aw-net-client` binary
//! against a real in-process `aw-net-server`.
//!
//! Server policy is `SignatureFailurePolicy::Kick`: any InputFrame signature
//! verification failure closes the connection (WS Close 1008), after which
//! the client binary's send path errors and the process EXITS with a failure.
//! Therefore "the process is still running after a ~3 s soak AND its stdout
//! shows a completed join plus a sustained snapshot stream" is the proof
//! that every frame the binary signed verified on the server.
//!
//! This file is intentionally self-contained (no shared `common` module —
//! that harness lives in aw-net-server's tests).

use std::io::Read;
use std::net::SocketAddr;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use aw_net_proto::{
    decode_msg, encode_msg, ClientToServer, Codec, ServerToClient, SigningKey, PROTOCOL_VERSION,
};
use aw_net_server::{spawn_server, RunningServer, ServerConfig, SignatureFailurePolicy};
use futures::{SinkExt, StreamExt};
use tempfile::TempDir;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

/// Upper bound on every awaited network operation in this file.
const IO_TIMEOUT: Duration = Duration::from_secs(10);

/// The one sanctioned duration-based soak: long enough for dozens of frames
/// at the client's 33 ms input cadence and ~90 snapshots at 30 Hz.
const SOAK: Duration = Duration::from_secs(3);

/// Minimum number of snapshot log lines required to call the stream
/// "sustained" (expected ~90 over the soak; 10 is robust under slow CI).
const MIN_SNAPSHOT_LINES: usize = 10;

/// A 64-hex-char (32-byte) custom key — deliberately NOT the dev default.
const CUSTOM_KEY_HEX: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// Kills the child on drop so assertion failures never leak a process.
struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    fn child_mut(&mut self) -> &mut Child {
        self.child.as_mut().expect("child already taken")
    }

    /// Kill the process (it runs an infinite loop; kill is the only way to
    /// stop it) and wait so the pipes close and reader threads finish.
    fn kill_and_wait(mut self) {
        let child = self.child.as_mut().expect("child already taken");
        child.kill().expect("failed to kill client binary");
        child.wait().expect("failed to wait for client binary");
        self.child = None;
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Drain a child pipe on a dedicated thread so the client can NEVER block on
/// a full pipe buffer (the binary logs every snapshot at INFO — 30 Hz).
fn drain_on_thread<R: Read + Send + 'static>(pipe: R) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut pipe = pipe;
        // Read until EOF (pipe closes when the process dies). An I/O error
        // mid-drain still yields whatever was captured.
        let _ = pipe.read_to_end(&mut bytes);
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

/// Spawn a real in-process server: TLS disabled, ephemeral loopback ports,
/// unique sled temp dir (returned TempDir must outlive the server).
async fn spawn_test_server(key: SigningKey) -> (RunningServer, TempDir) {
    let db_dir = TempDir::new().expect("create unique sled temp dir");
    let config = ServerConfig {
        ws_listen: "127.0.0.1:0".parse().expect("valid loopback addr"),
        http_listen: "127.0.0.1:0".parse().expect("valid loopback addr"),
        tls_enabled: false,
        db_path: db_dir.path().join("sled-db"),
        signing_key: key,
        sig_failure_policy: SignatureFailurePolicy::Kick,
        ..ServerConfig::default()
    };
    let server = timeout(IO_TIMEOUT, spawn_server(config))
        .await
        .expect("spawn_server timed out")
        .expect("spawn_server failed");
    (server, db_dir)
}

/// Minimal post-kill health check: a fresh raw ws:// connection must still
/// complete Hello → HelloAck against the same server.
async fn assert_server_still_serves(ws_addr: SocketAddr) {
    let url = format!("ws://{ws_addr}");
    let (mut ws, _resp) = timeout(IO_TIMEOUT, tokio_tungstenite::connect_async(&url))
        .await
        .expect("post-kill ws connect timed out")
        .expect("post-kill ws connect failed");
    let hello = encode_msg(
        Codec::PostcardLz4,
        &ClientToServer::Hello {
            protocol: PROTOCOL_VERSION,
        },
    );
    timeout(IO_TIMEOUT, ws.send(Message::Binary(hello.into())))
        .await
        .expect("post-kill ws send timed out")
        .expect("post-kill ws send failed");
    let msg = timeout(IO_TIMEOUT, ws.next())
        .await
        .expect("post-kill ws recv timed out")
        .expect("post-kill ws stream ended")
        .expect("post-kill ws recv failed");
    match msg {
        Message::Binary(b) => {
            match decode_msg::<ServerToClient>(Codec::PostcardLz4, &b)
                .expect("post-kill HelloAck must decode")
            {
                ServerToClient::HelloAck { protocol } => {
                    assert_eq!(protocol, PROTOCOL_VERSION, "HelloAck protocol mismatch");
                }
                other => panic!("expected HelloAck after client kill, got {other:?}"),
            }
        }
        other => panic!("expected binary HelloAck frame, got {other:?}"),
    }
}

/// Core scenario: launch the shipped binary against a Kick-policy server and
/// require survival + a completed join + a sustained snapshot stream.
///
/// `shared_key_hex = None` exercises the dev-default key path on both sides;
/// `Some(hex)` sets `AW_SHARED_KEY` on the child AND configures the server
/// with the same key (falsifying the binary's env-key plumbing: if the binary
/// ignored `AW_SHARED_KEY`, the server would kick it and it would exit).
async fn run_client_binary_soak(shared_key_hex: Option<&str>) {
    let server_key = match shared_key_hex {
        Some(hex) => SigningKey::from_hex(hex).expect("valid 64-hex custom key"),
        None => SigningKey::dev_default(),
    };
    let (server, _db_dir) = spawn_test_server(server_key).await;
    let ws_addr = server.ws_addr;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aw-net-client"));
    cmd.env("AW_WS_URL", format!("ws://{ws_addr}"))
        .env("AW_REGION", "f1-binary")
        .env("RUST_LOG", "info")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match shared_key_hex {
        Some(hex) => {
            cmd.env("AW_SHARED_KEY", hex);
        }
        None => {
            cmd.env_remove("AW_SHARED_KEY");
        }
    }
    let mut child = cmd.spawn().expect("failed to spawn aw-net-client binary");

    let stdout_thread = drain_on_thread(child.stdout.take().expect("child stdout piped"));
    let stderr_thread = drain_on_thread(child.stderr.take().expect("child stderr piped"));
    let mut guard = ChildGuard::new(child);

    // The sanctioned soak: dozens of signed frames at 33 ms cadence.
    tokio::time::sleep(SOAK).await;

    // Under Kick, ANY signature verification failure closes the connection
    // and the binary exits (send error -> `?` -> process exit). Still
    // running == zero verification failures across the whole soak.
    let exit = guard
        .child_mut()
        .try_wait()
        .expect("try_wait on client binary failed");
    if let Some(status) = exit {
        guard.kill_and_wait(); // no-op kill on dead process; closes pipes
        let stdout = stdout_thread.join().expect("stdout drain thread panicked");
        let stderr = stderr_thread.join().expect("stderr drain thread panicked");
        panic!(
            "client binary exited during the soak (status: {status})\n\
             --- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
        );
    }

    guard.kill_and_wait();
    let stdout = stdout_thread.join().expect("stdout drain thread panicked");
    let stderr = stderr_thread.join().expect("stderr drain thread panicked");

    // Survival alone could be vacuous (e.g. a client hung waiting for
    // JoinAccepted never sends a frame). Require the binary's own log
    // evidence: a completed join AND a sustained snapshot stream.
    assert!(
        stdout.contains("joined; tick_hz="),
        "client binary never completed the join handshake\n\
         --- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
    let snapshot_lines = stdout.matches("snapshot id=").count();
    assert!(
        snapshot_lines >= MIN_SNAPSHOT_LINES,
        "expected >= {MIN_SNAPSHOT_LINES} snapshot log lines over the soak, got {snapshot_lines}\n\
         --- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
    // NOTE: no assertion on `RateLimited` — the shipped client's ~30 Hz input
    // cadence can outrun the server's token bucket (refill 8/s, bucket 30)
    // near the end of the soak depending on OS timer granularity. Rate
    // limiting is reply-only (the server never disconnects for it), so it is
    // orthogonal to the signature-survival proof and timing-marginal to
    // assert either way.

    // The server must remain healthy after the client process is killed.
    assert_server_still_serves(ws_addr).await;

    server.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn client_binary_survives_soak_with_dev_default_key() {
    run_client_binary_soak(None).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn client_binary_survives_soak_with_custom_shared_key() {
    run_client_binary_soak(Some(CUSTOM_KEY_HEX)).await;
}
