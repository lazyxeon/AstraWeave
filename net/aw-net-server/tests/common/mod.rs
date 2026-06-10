//! Shared test harness for the W.3 net-trio test families (Families 1-4).
//!
//! Owned by the Family-1 unit; sibling families consume it READ-ONLY.
//! Design contract:
//! - Every awaited network operation is bounded by [`IO_TIMEOUT`] — no
//!   unbounded receives, no sleep-as-synchronization.
//! - All servers are spawned on `127.0.0.1:0` (real ephemeral ports) with a
//!   unique sled temp dir, so tests are parallel-safe.
//! - The codec is [`Codec::PostcardLz4`], matching the server's `AppState`.
//! - Raw access (`send_raw`, `send_ws`, `recv_raw`, `recv_close`) is exposed
//!   so later families can send garbage bytes / non-binary frames and inspect
//!   Close frames (code + reason) without modifying this module.

// This module is shared by several integration-test binaries; not every
// binary uses every helper, so the per-binary dead_code lint must be off.
#![allow(dead_code)]

use std::net::SocketAddr;
use std::time::Duration;

use aw_net_proto::{
    decode_msg, encode_msg, input_frame_sig_payload, sign, ClientToServer, Codec, ServerToClient,
    SigningKey, PROTOCOL_VERSION,
};
use aw_net_server::{spawn_server, RunningServer, ServerConfig, SignatureFailurePolicy};
use futures::{SinkExt, StreamExt};
use tempfile::TempDir;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::error::ProtocolError;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

/// Upper bound on every awaited network operation in this harness.
pub const IO_TIMEOUT: Duration = Duration::from_secs(10);

/// Wire codec — MUST match the server's (`AppState.codec = Codec::PostcardLz4`).
pub const TEST_CODEC: Codec = Codec::PostcardLz4;

/// A live in-process server plus the temp dir backing its sled database.
///
/// sled holds an exclusive lock on its directory, so `db_dir` must stay alive
/// for the server's whole lifetime (it does: it is owned here). `TempDir`'s
/// drop is best-effort, so a still-locked dir on teardown cannot fail a test.
pub struct TestServer {
    pub server: RunningServer,
    pub db_dir: TempDir,
}

impl TestServer {
    pub fn ws_addr(&self) -> SocketAddr {
        self.server.ws_addr
    }

    pub fn http_addr(&self) -> SocketAddr {
        self.server.http_addr
    }

    pub fn ws_url(&self) -> String {
        format!("ws://{}", self.server.ws_addr)
    }

    /// Abort the server's accept loops. Established connections end when
    /// their sockets close.
    pub fn shutdown(self) {
        self.server.shutdown();
    }
}

/// Spawn a real in-process server: TLS disabled, both listeners on
/// `127.0.0.1:0` (unique ephemeral ports), unique sled temp dir.
pub async fn spawn_test_server(key: SigningKey, policy: SignatureFailurePolicy) -> TestServer {
    let db_dir = TempDir::new().expect("create unique sled temp dir");
    let config = ServerConfig {
        ws_listen: "127.0.0.1:0".parse().expect("valid loopback addr"),
        http_listen: "127.0.0.1:0".parse().expect("valid loopback addr"),
        tls_enabled: false,
        db_path: db_dir.path().join("sled-db"),
        signing_key: key,
        sig_failure_policy: policy,
        ..ServerConfig::default()
    };
    let server = timeout(IO_TIMEOUT, spawn_server(config))
        .await
        .expect("spawn_server timed out")
        .expect("spawn_server failed");
    TestServer { server, db_dir }
}

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Result of a completed protocol handshake (`Hello → HelloAck →
/// FindOrCreate → MatchResult → JoinAccepted`).
#[derive(Debug, Clone)]
pub struct JoinInfo {
    pub room_id: String,
    pub player_id: String,
    pub tick_hz: u32,
}

/// Plain-`ws://` test client over tokio-tungstenite.
pub struct TestClient {
    ws: WsStream,
}

impl TestClient {
    /// Open a plain `ws://` connection. NO protocol handshake is performed —
    /// use [`TestClient::join`] (or [`TestClient::connect_and_join`]) for a
    /// joined session, or drive raw frames directly for adversarial tests.
    pub async fn connect(ws_addr: SocketAddr) -> Self {
        let url = format!("ws://{ws_addr}");
        let (ws, _resp) = timeout(IO_TIMEOUT, connect_async(&url))
            .await
            .expect("ws connect timed out")
            .expect("ws connect failed");
        Self { ws }
    }

    /// Convenience: [`TestClient::connect`] + [`TestClient::join`].
    pub async fn connect_and_join(
        ws_addr: SocketAddr,
        region: &str,
        game_mode: &str,
    ) -> (Self, JoinInfo) {
        let mut client = Self::connect(ws_addr).await;
        let info = client.join(region, game_mode).await;
        (client, info)
    }

    /// Drive the REAL protocol handshake to a joined session:
    /// send `Hello` → expect `HelloAck` → send `FindOrCreate` → expect
    /// `MatchResult` → expect `JoinAccepted`.
    ///
    /// The server sends these strictly in order BEFORE its 30 Hz snapshot
    /// loop starts, so the exact-order matching here is itself a protocol
    /// assertion (any reordering or interleaving panics).
    pub async fn join(&mut self, region: &str, game_mode: &str) -> JoinInfo {
        self.send_msg(&ClientToServer::Hello {
            protocol: PROTOCOL_VERSION,
        })
        .await;
        match self.recv_msg().await {
            ServerToClient::HelloAck { protocol } => {
                assert_eq!(
                    protocol, PROTOCOL_VERSION,
                    "HelloAck echoed wrong protocol version"
                );
            }
            other => panic!("expected HelloAck, got {other:?}"),
        }

        self.send_msg(&ClientToServer::FindOrCreate {
            region: region.to_string(),
            game_mode: game_mode.to_string(),
            party_size: 1,
        })
        .await;

        let matched_room = match self.recv_msg().await {
            ServerToClient::MatchResult { room_id } => room_id,
            other => panic!("expected MatchResult, got {other:?}"),
        };
        match self.recv_msg().await {
            ServerToClient::JoinAccepted {
                room_id,
                player_id,
                tick_hz,
            } => {
                assert_eq!(
                    room_id, matched_room,
                    "JoinAccepted room differs from MatchResult room"
                );
                JoinInfo {
                    room_id,
                    player_id,
                    tick_hz,
                }
            }
            other => panic!("expected JoinAccepted, got {other:?}"),
        }
    }

    /// Encode `msg` with [`TEST_CODEC`] and send it as a binary WS frame.
    pub async fn send_msg(&mut self, msg: &ClientToServer) {
        let bytes = encode_msg(TEST_CODEC, msg);
        self.send_raw(bytes).await;
    }

    /// Send arbitrary bytes as a binary WS frame (for malformed-payload
    /// tests — bypasses the codec entirely).
    pub async fn send_raw(&mut self, bytes: Vec<u8>) {
        timeout(IO_TIMEOUT, self.ws.send(Message::Binary(bytes.into())))
            .await
            .expect("ws send timed out")
            .expect("ws send failed");
    }

    /// Send a raw tungstenite message (Text / Ping / Close / ...).
    pub async fn send_ws(&mut self, msg: Message) {
        timeout(IO_TIMEOUT, self.ws.send(msg))
            .await
            .expect("ws send timed out")
            .expect("ws send failed");
    }

    /// Sign via the canonical surface (`input_frame_sig_payload` + `sign`)
    /// and send an `InputFrame`.
    pub async fn send_input_frame(
        &mut self,
        seq: u32,
        tick_ms: u64,
        input_blob: &[u8],
        key: &SigningKey,
    ) {
        let payload = input_frame_sig_payload(seq, tick_ms, input_blob);
        let sig = sign(key, &payload);
        self.send_msg(&ClientToServer::InputFrame {
            seq,
            tick_ms,
            input_blob: input_blob.to_vec(),
            sig,
        })
        .await;
    }

    /// Next raw WS message, Close frames included (`Some(Message::Close(_))`
    /// carries code + reason for inspection).
    ///
    /// Returns `None` when the connection is gone: stream end, completed
    /// close handshake, or a transport-level "peer went away" error
    /// (`ConnectionClosed` / `AlreadyClosed` / reset-without-close /
    /// connection reset/aborted IO errors — the exact shape differs between
    /// platforms, so they are normalized here for determinism). Panics on
    /// timeout and on any other transport error.
    pub async fn recv_raw(&mut self) -> Option<Message> {
        match timeout(IO_TIMEOUT, self.ws.next())
            .await
            .expect("ws recv timed out")
        {
            None => None,
            Some(Ok(msg)) => Some(msg),
            Some(Err(WsError::ConnectionClosed)) | Some(Err(WsError::AlreadyClosed)) => None,
            Some(Err(WsError::Protocol(ProtocolError::ResetWithoutClosingHandshake))) => None,
            Some(Err(WsError::Io(e)))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
                ) =>
            {
                None
            }
            Some(Err(other)) => panic!("unexpected ws transport error: {other}"),
        }
    }

    /// Next decoded `ServerToClient` message. Skips WS-level Ping/Pong
    /// control frames; panics on Close, stream end, timeout, or an
    /// undecodable binary payload.
    pub async fn recv_msg(&mut self) -> ServerToClient {
        loop {
            match self.recv_raw().await {
                Some(Message::Binary(b)) => {
                    return decode_msg::<ServerToClient>(TEST_CODEC, &b)
                        .expect("failed to decode ServerToClient frame");
                }
                Some(Message::Ping(_)) | Some(Message::Pong(_)) => continue,
                Some(other) => panic!("expected binary ServerToClient frame, got {other:?}"),
                None => panic!("ws stream ended while expecting a ServerToClient message"),
            }
        }
    }

    /// Receive decoded messages until `f` returns `Some`, skipping everything
    /// `f` maps to `None` (typically the 30 Hz snapshot stream). Bounded by
    /// [`IO_TIMEOUT`] overall in addition to the per-message bound.
    pub async fn recv_until<T>(&mut self, mut f: impl FnMut(ServerToClient) -> Option<T>) -> T {
        let deadline = tokio::time::Instant::now() + IO_TIMEOUT;
        loop {
            assert!(
                tokio::time::Instant::now() < deadline,
                "recv_until: no matching message within {IO_TIMEOUT:?}"
            );
            if let Some(v) = f(self.recv_msg().await) {
                return v;
            }
        }
    }

    /// Drain raw messages until a Close frame or stream end. Returns the
    /// Close frame's payload (`code` + `reason`) when one was observed;
    /// `None` if the stream ended without an inspectable Close frame (which
    /// also covers an empty `Close(None)` frame — the server always attaches
    /// a code + reason when kicking, so asserting `Some` is correct there).
    pub async fn recv_close(&mut self) -> Option<CloseFrame> {
        let deadline = tokio::time::Instant::now() + IO_TIMEOUT;
        loop {
            assert!(
                tokio::time::Instant::now() < deadline,
                "recv_close: no Close frame within {IO_TIMEOUT:?}"
            );
            match self.recv_raw().await {
                Some(Message::Close(frame)) => return frame,
                Some(_) => continue,
                None => return None,
            }
        }
    }

    /// Liveness probe through the full protocol stack: send `Ping { nano }`
    /// and require the matching `Pong`, skipping interleaved snapshots.
    ///
    /// Because the server processes a connection's messages strictly in
    /// order, receiving the Pong for THIS nano proves every previously sent
    /// message on this connection was accepted without disconnecting us —
    /// under `SignatureFailurePolicy::Kick` that is the
    /// zero-verification-failures proof.
    pub async fn assert_ping_pong(&mut self, nano: u128) {
        self.send_msg(&ClientToServer::Ping { nano }).await;
        let got = self
            .recv_until(|m| match m {
                ServerToClient::Pong { nano: n } => Some(n),
                ServerToClient::Snapshot { .. } => None,
                other => panic!("unexpected message while awaiting Pong: {other:?}"),
            })
            .await;
        assert_eq!(got, nano, "Pong nano does not match the Ping we sent");
    }

    /// Initiate a clean WebSocket close handshake and drain until the stream
    /// ends, so the server has observed the Close frame before this returns.
    pub async fn close(mut self) {
        timeout(IO_TIMEOUT, self.ws.close(None))
            .await
            .expect("ws close timed out")
            .expect("ws close failed");
        let deadline = tokio::time::Instant::now() + IO_TIMEOUT;
        loop {
            assert!(
                tokio::time::Instant::now() < deadline,
                "close: stream did not end within {IO_TIMEOUT:?}"
            );
            if self.recv_raw().await.is_none() {
                return;
            }
        }
    }
}
