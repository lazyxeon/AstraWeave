//! W.3 Family-3 (client binary): wrong-key and malformed-key behavior of the
//! SHIPPED `aw-net-client` binary against a real in-process `aw-net-server`.
//!
//! Two contracts of the shipped binary are pinned here:
//!
//! 1. **Wrong key + Kick server**: the protocol handshake is UNSIGNED
//!    (verified against `aw-net-client/src/main.rs`: `Hello` and
//!    `FindOrCreate` carry no signature; only `InputFrame` does), so a binary
//!    holding key B joins a key-A server normally — then its FIRST InputFrame
//!    fails verification, the server closes with 1008, the binary's next send
//!    errors, `?` propagates out of `main`, and the PROCESS EXITS with a
//!    failure status (contrast Family-1, which proves the matched-key binary
//!    is STILL RUNNING after a soak).
//! 2. **Malformed `AW_SHARED_KEY`**: a hard non-zero exit BEFORE any
//!    connection is attempted — never a silent dev-key fallback — and the
//!    error output names the variable without echoing the key material.
//!
//! This file is intentionally self-contained (no shared `common` module —
//! that harness lives in aw-net-server's tests), following the established
//! `family1_client_binary.rs` pattern: in-process server via the
//! aw-net-server dev-dependency, the shipped binary via
//! `env!("CARGO_BIN_EXE_aw-net-client")`, `ChildGuard` kill-on-drop, and
//! dedicated output-drain threads. Output routing (family1-validated): the
//! binary's tracing goes to STDOUT (`tracing_subscriber::fmt::init()` default
//! writer — the `joined; tick_hz=` marker lives there); the `anyhow` error
//! returned from `main` prints to STDERR.

use std::io::Read;
use std::net::SocketAddr;
use std::process::{Child, Command, ExitStatus, Stdio};
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

/// Bounded window for the child process to exit on its own. The wrong-key
/// kick lands on the very first InputFrame (~33 ms cadence) and the
/// malformed-key error fires before any I/O, so both exits are expected well
/// under a second; the window is CI slack, not an expectation.
const EXIT_WINDOW: Duration = Duration::from_secs(15);

/// Poll interval for [`await_exit_within`] (bounded poll, not a sync sleep).
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Key A — the server's key (64 hex = 32 bytes), deliberately NOT the dev
/// default (so an accidental dev-key fallback in the binary could never
/// match it).
const KEY_A_HEX: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

/// Key B — the WRONG key the binary is given. Differs from key A in every
/// byte, and is also not the dev default.
const KEY_B_HEX: &str = "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100";

/// A malformed `AW_SHARED_KEY` value (not 64 hex chars). Doubles as the
/// key-material-hygiene probe: this exact string must never appear in the
/// binary's output.
const MALFORMED_KEY: &str = "zzzz";

/// Kills the child on drop so assertion failures never leak a process.
/// Dropping after a natural (already-reaped) exit is harmless: the redundant
/// kill/wait errors are ignored.
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

    /// Kill a still-running process and wait so the pipes close and the
    /// drain threads finish (failure-path teardown only — the success paths
    /// in this file end with a NATURAL exit observed via `try_wait`).
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

/// Bounded poll until the child exits on its own. `Some(status)` means the
/// process exited (and `try_wait` reaped it — its pipes are closed, so the
/// drain threads finish without intervention); `None` means it was still
/// running at the deadline (caller tears down via `kill_and_wait` and fails).
async fn await_exit_within(guard: &mut ChildGuard, window: Duration) -> Option<ExitStatus> {
    let deadline = tokio::time::Instant::now() + window;
    loop {
        if let Some(status) = guard
            .child_mut()
            .try_wait()
            .expect("try_wait on client binary failed")
        {
            return Some(status);
        }
        if tokio::time::Instant::now() >= deadline {
            return None;
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

/// Spawn a real in-process server: Kick policy, TLS disabled, ephemeral
/// loopback ports, unique sled temp dir (returned TempDir must outlive the
/// server).
async fn spawn_kick_server(key: SigningKey) -> (RunningServer, TempDir) {
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

/// Launch the shipped binary against `ws_addr` with `AW_SHARED_KEY` set to
/// `shared_key`, pipes captured. Returns the guard plus the stdout/stderr
/// drain threads.
fn launch_client_binary(
    ws_url: &str,
    region: &str,
    shared_key: &str,
) -> (
    ChildGuard,
    std::thread::JoinHandle<String>,
    std::thread::JoinHandle<String>,
) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aw-net-client"));
    cmd.env("AW_WS_URL", ws_url)
        .env("AW_REGION", region)
        .env("RUST_LOG", "info")
        .env("AW_SHARED_KEY", shared_key)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("failed to spawn aw-net-client binary");
    let stdout_thread = drain_on_thread(child.stdout.take().expect("child stdout piped"));
    let stderr_thread = drain_on_thread(child.stderr.take().expect("child stderr piped"));
    (ChildGuard::new(child), stdout_thread, stderr_thread)
}

/// Minimal post-kick health check: a fresh raw ws:// connection must still
/// complete Hello → HelloAck against the same server.
async fn assert_server_still_serves(ws_addr: SocketAddr) {
    let url = format!("ws://{ws_addr}");
    let (mut ws, _resp) = timeout(IO_TIMEOUT, tokio_tungstenite::connect_async(&url))
        .await
        .expect("post-kick ws connect timed out")
        .expect("post-kick ws connect failed");
    let hello = encode_msg(
        Codec::PostcardLz4,
        &ClientToServer::Hello {
            protocol: PROTOCOL_VERSION,
        },
    );
    timeout(IO_TIMEOUT, ws.send(Message::Binary(hello.into())))
        .await
        .expect("post-kick ws send timed out")
        .expect("post-kick ws send failed");
    let msg = timeout(IO_TIMEOUT, ws.next())
        .await
        .expect("post-kick ws recv timed out")
        .expect("post-kick ws stream ended")
        .expect("post-kick ws recv failed");
    match msg {
        Message::Binary(b) => {
            match decode_msg::<ServerToClient>(Codec::PostcardLz4, &b)
                .expect("post-kick HelloAck must decode")
            {
                ServerToClient::HelloAck { protocol } => {
                    assert_eq!(protocol, PROTOCOL_VERSION, "HelloAck protocol mismatch");
                }
                other => panic!("expected HelloAck after the kick, got {other:?}"),
            }
        }
        other => panic!("expected binary HelloAck frame, got {other:?}"),
    }
}

/// Battery item 9: shipped binary with a mismatched `AW_SHARED_KEY` against a
/// Kick-policy server. The binary joins (unsigned handshake), its first
/// key-B InputFrame draws the 1008 kick, and the PROCESS EXITS with a
/// failure status within the bounded window — the exact inverse of
/// family1's still-running-after-soak assertion. Anti-vacuity: stdout must
/// carry the `joined; tick_hz=` marker, attributing the death to the signed
/// traffic rather than to connect/join. The server must keep serving a
/// minimal fresh check (Hello → HelloAck) afterwards.
#[tokio::test(flavor = "multi_thread")]
async fn wrong_key_binary_is_kicked_and_process_exits() {
    let key_a = SigningKey::from_hex(KEY_A_HEX).expect("KEY_A_HEX is a valid 64-hex key");
    let (server, _db_dir) = spawn_kick_server(key_a).await;
    let ws_addr = server.ws_addr;

    let (mut guard, stdout_thread, stderr_thread) =
        launch_client_binary(&format!("ws://{ws_addr}"), "f3-binary-kick", KEY_B_HEX);

    // Kick → next send errors → `?` → process exit, all within the window.
    let status = match await_exit_within(&mut guard, EXIT_WINDOW).await {
        Some(status) => status,
        None => {
            guard.kill_and_wait();
            let stdout = stdout_thread.join().expect("stdout drain thread panicked");
            let stderr = stderr_thread.join().expect("stderr drain thread panicked");
            panic!(
                "client binary still running {EXIT_WINDOW:?} after launch with a wrong key \
                 against a Kick-policy server — the kick did not terminate it\n\
                 --- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
            );
        }
    };
    let stdout = stdout_thread.join().expect("stdout drain thread panicked");
    let stderr = stderr_thread.join().expect("stderr drain thread panicked");

    // The binary's only exit paths are `?` errors (its main loop is
    // infinite), so a kick-induced exit must be a failure status.
    assert!(
        !status.success(),
        "wrong-key exit must be a failure status (the binary exits via an error), got {status}\n\
         --- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );

    // Anti-vacuity: the binary got PAST the unsigned handshake — it died on
    // the signed traffic, not on connect/join. The join marker is on stdout
    // (tracing fmt default writer; family1-validated).
    assert!(
        stdout.contains("joined; tick_hz="),
        "client binary never completed the join handshake — the exit is not attributable \
         to the wrong-key kick\n\
         --- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );

    // The kick must not poison the server.
    assert_server_still_serves(ws_addr).await;

    server.shutdown();
}

/// Battery item 10: shipped binary with a malformed `AW_SHARED_KEY` must
/// fail fast — non-zero exit WITHOUT attempting any connection (never a
/// silent dev-key fallback) — and the error output must name the variable
/// without echoing the key material (hygiene pin).
///
/// No-connection proof: `AW_WS_URL` points at a real bound listener that is
/// never `accept`ed; the OS completes any TCP handshake into the listen
/// backlog regardless, so a later non-blocking `accept()` returning
/// `WouldBlock` proves zero connection attempts reached the socket.
#[tokio::test(flavor = "multi_thread")]
async fn malformed_shared_key_fails_fast_without_connecting() {
    let sentinel = std::net::TcpListener::bind("127.0.0.1:0").expect("bind sentinel TCP listener");
    sentinel
        .set_nonblocking(true)
        .expect("set sentinel listener non-blocking");
    let sentinel_addr = sentinel
        .local_addr()
        .expect("read sentinel listener local addr");

    let (mut guard, stdout_thread, stderr_thread) = launch_client_binary(
        &format!("ws://{sentinel_addr}"),
        "f3-binary-malformed",
        MALFORMED_KEY,
    );

    let status = match await_exit_within(&mut guard, EXIT_WINDOW).await {
        Some(status) => status,
        None => {
            guard.kill_and_wait();
            let stdout = stdout_thread.join().expect("stdout drain thread panicked");
            let stderr = stderr_thread.join().expect("stderr drain thread panicked");
            panic!(
                "client binary still running {EXIT_WINDOW:?} after launch with a malformed \
                 AW_SHARED_KEY — the fail-fast contract is broken\n\
                 --- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
            );
        }
    };
    let stdout = stdout_thread.join().expect("stdout drain thread panicked");
    let stderr = stderr_thread.join().expect("stderr drain thread panicked");

    assert!(
        !status.success(),
        "malformed AW_SHARED_KEY must exit non-zero, got {status}\n\
         --- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );

    // Fail-fast: the key is rejected BEFORE any connection is attempted.
    match sentinel.accept() {
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
        Ok((_stream, peer)) => panic!(
            "binary attempted a connection (from {peer}) despite a malformed AW_SHARED_KEY — \
             the key must be a hard error before any I/O\n\
             --- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
        ),
        Err(e) => panic!("sentinel listener accept failed unexpectedly: {e}"),
    }
    assert!(
        !stdout.contains("joined; tick_hz="),
        "binary must never join with a malformed key\n\
         --- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );

    // The error must name the variable (actionable)...
    assert!(
        stderr.contains("AW_SHARED_KEY"),
        "error output must name AW_SHARED_KEY\n\
         --- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
    // ...but never echo the key material, on either stream (hygiene pin —
    // a real deployment's typo'd key is still secret-adjacent material).
    assert!(
        !stdout.contains(MALFORMED_KEY) && !stderr.contains(MALFORMED_KEY),
        "output must not echo the AW_SHARED_KEY value\n\
         --- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
}
