//! W.5.2 Family-5: the TLS (`wss://`) input-frame signature path.
//!
//! Every other test family (1-4, w2b_fix1) spawns a `tls_enabled: false`
//! plain-TCP server through `common::spawn_test_server` and drives the plain
//! `handle_socket` → `on_client_msg` path. But `ServerConfig::default()` sets
//! `tls_enabled: true`, so the DEFAULT production code path is the TLS one:
//! `accept_loop_tls` → `handle_socket_tls` → `on_client_msg_tls`
//! (`aw-net-server/src/lib.rs`). That TLS handler carries the load-bearing
//! comment "MUST stay semantically identical to the non-TLS handler in
//! on_client_msg — a skippable path on either is a security bug", yet no test
//! exercised it. This family closes that gap: it spins up a REAL TLS server
//! and a real `wss://` client, then proves the verify + `SignatureFailurePolicy`
//! behavior over TLS matches the plain path exactly.
//!
//! Proof conventions mirror Family-1/3:
//! - "stays open / accepted" is proven by subsequent successful authenticated
//!   traffic — an in-order `Ping → Pong` through the full protocol stack (the
//!   server processes a connection's messages strictly in order, so receiving
//!   THIS Pong proves every prior frame on the connection was accepted without
//!   a kick). Under `Kick`, survival of N signed frames == zero verification
//!   failures across them.
//! - "kicked" is proven by an explicit WebSocket Close frame (wire code 1008 /
//!   `CloseCode::Policy` + the documented reason) followed by stream end —
//!   never a bare stream-end or a timeout.
//! - Every awaited network op is bounded by [`IO_TIMEOUT`]; servers run on
//!   `127.0.0.1:0` ephemeral ports with unique sled temp dirs (parallel-safe).
//!
//! TLS client design: the server presents a self-signed dev cert. These tests
//! validate the HMAC SIGNATURE path over TLS, NOT TLS certificate validation,
//! so the client installs a permissive `ServerCertVerifier` that accepts the
//! server's cert (and still performs the real TLS key exchange + record
//! encryption through rustls). This isolates the signature assertions from
//! cert-SAN brittleness. The cert/key themselves are generated fresh per test
//! via `rcgen` into a `TempDir`, because the committed `net/certs/dev/*.pem`
//! are placeholders that the server's cert loader rejects (see the module
//! deviation note in the W.5.2 report).

mod common;

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use aw_net_proto::{
    decode_msg, encode_msg, input_frame_sig_payload, sign, ClientToServer, Codec, ServerToClient,
    SigningKey, PROTOCOL_VERSION,
};
use aw_net_server::{spawn_server, RunningServer, ServerConfig, SignatureFailurePolicy};
use common::{IO_TIMEOUT, TEST_CODEC};
use futures::{SinkExt, StreamExt};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use tempfile::TempDir;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
use tokio_tungstenite::tungstenite::error::ProtocolError;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
use tokio_tungstenite::{client_async, WebSocketStream};

/// Key A — the key configured on every server in this file (64 hex = 32 bytes).
const KEY_A_HEX: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

/// Key B — the WRONG key. Differs from key A in every byte.
const KEY_B_HEX: &str = "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100";

/// The documented kick reason — the `MsgOutcome::Kick` payload in
/// `aw-net-server/src/lib.rs`, carried into the Close frame. Must match
/// Family-3's assertion byte-for-byte.
const KICK_REASON: &str = "input frame signature verification failed";

fn key_a() -> SigningKey {
    SigningKey::from_hex(KEY_A_HEX).expect("KEY_A_HEX is a valid 64-hex key")
}

fn key_b() -> SigningKey {
    SigningKey::from_hex(KEY_B_HEX).expect("KEY_B_HEX is a valid 64-hex key")
}

/// Snapshot payload contract: lz4 size-prepended postcard of `{ tick: u64 }`
/// (mirrors the server's private `DemoState`).
#[derive(serde::Deserialize)]
struct DemoState {
    tick: u64,
}

fn decode_snapshot_tick(compressed: bool, payload: &[u8]) -> u64 {
    assert!(
        compressed,
        "server snapshots are always sent lz4-compressed"
    );
    let raw = lz4_flex::decompress_size_prepended(payload)
        .expect("snapshot payload must be valid lz4 (size-prepended)");
    let state: DemoState =
        postcard::from_bytes(&raw).expect("snapshot payload must decode as postcard DemoState");
    state.tick
}

// ---------------------------------------------------------------------------
// TLS server spawn (local; the shared harness only spawns plain TCP)
// ---------------------------------------------------------------------------

/// A live in-process TLS server plus the temp dirs backing its sled database
/// and its generated cert/key. Both temp dirs must outlive the server (sled
/// holds an exclusive lock on its dir; the cert/key files are read once at
/// spawn but kept for clarity and parity with the harness's ownership model).
struct TlsTestServer {
    server: RunningServer,
    _db_dir: TempDir,
    _cert_dir: TempDir,
}

impl TlsTestServer {
    fn ws_addr(&self) -> SocketAddr {
        self.server.ws_addr
    }

    fn shutdown(self) {
        self.server.shutdown();
    }
}

/// Write `contents` to `dir/name` and return the path. Used for the freshly
/// generated cert/key PEMs.
fn write_pem(dir: &Path, name: &str, contents: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, contents)
        .unwrap_or_else(|e| panic!("write {} failed: {e}", path.display()));
    path
}

/// Spawn a REAL `tls_enabled: true` server on `127.0.0.1:0` with a unique sled
/// temp dir and a freshly generated self-signed cert/key. This drives the
/// production `accept_loop_tls` → `handle_socket_tls` → `on_client_msg_tls`
/// path — the code under test.
///
/// The committed `net/certs/dev/*.pem` are placeholders the server's
/// `create_tls_acceptor` would reject, so we generate a valid cert at test
/// time (SANs `localhost` + `127.0.0.1`; the client uses a permissive verifier
/// so the SAN value does not gate acceptance). rcgen's `ring` backend matches
/// the server's rustls crypto provider.
async fn spawn_tls_test_server(key: SigningKey, policy: SignatureFailurePolicy) -> TlsTestServer {
    let db_dir = TempDir::new().expect("create unique sled temp dir");
    let cert_dir = TempDir::new().expect("create unique cert temp dir");

    let certified =
        rcgen::generate_simple_self_signed(vec!["localhost".to_string(), "127.0.0.1".to_string()])
            .expect("generate self-signed dev cert");
    let cert_path = write_pem(cert_dir.path(), "dev-cert.pem", &certified.cert.pem());
    let key_path = write_pem(
        cert_dir.path(),
        "dev-key.pem",
        &certified.signing_key.serialize_pem(),
    );

    let config = ServerConfig {
        ws_listen: "127.0.0.1:0".parse().expect("valid loopback addr"),
        http_listen: "127.0.0.1:0".parse().expect("valid loopback addr"),
        tls_enabled: true,
        tls_cert_path: cert_path,
        tls_key_path: key_path,
        db_path: db_dir.path().join("sled-db"),
        signing_key: key,
        sig_failure_policy: policy,
        // All eight ServerConfig fields are set explicitly above — no
        // `..ServerConfig::default()` (clippy::needless_update), and it keeps
        // the TLS config from silently inheriting the 0.0.0.0:8788 default
        // listen addrs.
    };
    let server = timeout(IO_TIMEOUT, spawn_server(config))
        .await
        .expect("spawn_server timed out")
        .expect("spawn_server (TLS) failed — TLS server must bind and load the generated cert");
    TlsTestServer {
        server,
        _db_dir: db_dir,
        _cert_dir: cert_dir,
    }
}

// ---------------------------------------------------------------------------
// Permissive TLS client cert verifier (TEST INFRASTRUCTURE ONLY)
// ---------------------------------------------------------------------------

/// A `ServerCertVerifier` that accepts ANY server certificate.
///
/// THIS IS TEST-ONLY INFRASTRUCTURE. These tests validate the HMAC input-frame
/// SIGNATURE path over a TLS transport — they are NOT testing TLS certificate
/// validation. Accepting the server's self-signed dev cert here is correct: it
/// isolates the signature assertions from cert-SAN/trust-anchor brittleness
/// while still performing the real TLS key exchange and record encryption
/// (the signature-scheme checks below delegate to rustls' ring provider, so
/// the handshake's own crypto is genuinely verified — only chain-of-trust is
/// bypassed). NEVER use a verifier like this in production code.
#[derive(Debug)]
struct AcceptAnyServerCert {
    provider: Arc<rustls::crypto::CryptoProvider>,
}

impl AcceptAnyServerCert {
    fn new() -> Self {
        Self {
            provider: Arc::new(rustls::crypto::ring::default_provider()),
        }
    }
}

impl ServerCertVerifier for AcceptAnyServerCert {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // Accept the chain unconditionally — see the type-level rationale.
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Build the test `ClientConfig`. `ClientConfig::builder()` uses rustls' ring
/// provider (the `ring` feature is on for both the server and these tests), so
/// no process-global `CryptoProvider` install is needed — avoiding a global
/// install also keeps parallel tests from racing on it.
fn test_client_config() -> ClientConfig {
    ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AcceptAnyServerCert::new()))
        .with_no_client_auth()
}

// ---------------------------------------------------------------------------
// wss:// test client (mirrors the plain harness TestClient methods we need)
// ---------------------------------------------------------------------------

type TlsWsStream = WebSocketStream<TlsStream<TcpStream>>;

/// Result of a completed protocol handshake over TLS.
#[derive(Debug, Clone)]
struct JoinInfo {
    room_id: String,
    player_id: String,
    tick_hz: u32,
}

/// A `wss://` test client: TCP → rustls TLS → WebSocket, then the same
/// `PostcardLz4`-encoded `ClientToServer`/`ServerToClient` protocol as the
/// plain `TestClient`.
struct TlsTestClient {
    ws: TlsWsStream,
}

impl TlsTestClient {
    /// Open a `wss://` connection: TCP connect → TLS handshake (permissive
    /// verifier) → WebSocket handshake. NO protocol handshake yet.
    async fn connect(ws_addr: SocketAddr) -> Self {
        let tcp = timeout(IO_TIMEOUT, TcpStream::connect(ws_addr))
            .await
            .expect("tcp connect timed out")
            .expect("tcp connect failed");

        let connector = TlsConnector::from(Arc::new(test_client_config()));
        // The permissive verifier ignores the server name, but rustls still
        // requires a syntactically valid one for SNI; "localhost" is fine.
        let domain = ServerName::try_from("localhost").expect("valid server name");
        let tls = timeout(IO_TIMEOUT, connector.connect(domain, tcp))
            .await
            .expect("tls handshake timed out")
            .expect("tls handshake failed — server must present a loadable cert over TLS");

        // Path is irrelevant: the server's accept closure ignores the request.
        let url = format!("wss://{ws_addr}/");
        let (ws, _resp) = timeout(IO_TIMEOUT, client_async(url, tls))
            .await
            .expect("ws-over-tls handshake timed out")
            .expect("ws-over-tls handshake failed");
        Self { ws }
    }

    /// `connect` + the full protocol handshake to a joined session.
    async fn connect_and_join(
        ws_addr: SocketAddr,
        region: &str,
        game_mode: &str,
    ) -> (Self, JoinInfo) {
        let mut client = Self::connect(ws_addr).await;
        let info = client.join(region, game_mode).await;
        (client, info)
    }

    /// Drive the REAL protocol handshake: `Hello → HelloAck → FindOrCreate →
    /// MatchResult → JoinAccepted`. Exact-order matching is itself a protocol
    /// assertion (any reordering panics).
    async fn join(&mut self, region: &str, game_mode: &str) -> JoinInfo {
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

    /// Encode `msg` with [`TEST_CODEC`] (= the server's `PostcardLz4`) and send
    /// it as a binary WS frame.
    async fn send_msg(&mut self, msg: &ClientToServer) {
        let bytes = encode_msg(TEST_CODEC, msg);
        timeout(IO_TIMEOUT, self.ws.send(Message::Binary(bytes.into())))
            .await
            .expect("ws send timed out")
            .expect("ws send failed");
    }

    /// Sign via the canonical surface (`input_frame_sig_payload` + `sign`) and
    /// send an `InputFrame` — identical signing to the plain `TestClient`.
    async fn send_input_frame(
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

    /// Next raw WS message; Close frames included. Returns `None` on the same
    /// "peer went away" shapes the plain harness normalizes (so a kick surfaces
    /// as the Close frame, and the post-Close stream end surfaces as `None`).
    ///
    /// TLS adds one normalization the plain harness never needs: rustls maps an
    /// abrupt peer close that omits a `close_notify` alert to an IO error of
    /// kind `UnexpectedEof` (the server drops the TLS stream right after the
    /// kick Close / clean-close drain without sending close_notify). That is a
    /// benign end-of-stream over TLS — the same "peer went away" signal plain
    /// TCP surfaces as `ConnectionReset` — so it normalizes to `None`, NOT a
    /// panic. The kick's Close-1008 frame is delivered and observed BEFORE this
    /// EOF, so this does not weaken any kick assertion.
    async fn recv_raw(&mut self) -> Option<Message> {
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
                    std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::UnexpectedEof
                ) =>
            {
                None
            }
            Some(Err(other)) => panic!("unexpected ws transport error: {other}"),
        }
    }

    /// Next decoded `ServerToClient`, skipping WS Ping/Pong control frames;
    /// panics on Close, stream end, timeout, or an undecodable payload.
    async fn recv_msg(&mut self) -> ServerToClient {
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

    /// Receive decoded messages until `f` returns `Some`, skipping the rest
    /// (typically the 30 Hz snapshot stream). Bounded by [`IO_TIMEOUT`].
    async fn recv_until<T>(&mut self, mut f: impl FnMut(ServerToClient) -> Option<T>) -> T {
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

    /// Next snapshot's `server_tick`, skipping everything else.
    async fn recv_snapshot_server_tick(&mut self) -> u64 {
        self.recv_until(|m| match m {
            ServerToClient::Snapshot { server_tick, .. } => Some(server_tick),
            _ => None,
        })
        .await
    }

    /// Liveness probe through the full protocol stack: `Ping { nano }` → the
    /// matching `Pong`, skipping interleaved snapshots. Because the server
    /// processes a connection's messages strictly in order, the matching Pong
    /// proves every previously sent frame was accepted without a kick — the
    /// zero-verification-failures proof under `Kick`.
    async fn assert_ping_pong(&mut self, nano: u128) {
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

    /// Like [`assert_ping_pong`] but also tolerates queued `RateLimited`
    /// replies (needed right after a deliberate rate-limit burst under Warn).
    async fn assert_ping_pong_tolerating_rate_limited(&mut self, nano: u128) {
        self.send_msg(&ClientToServer::Ping { nano }).await;
        let got = self
            .recv_until(|m| match m {
                ServerToClient::Pong { nano: n } => Some(n),
                ServerToClient::Snapshot { .. } | ServerToClient::RateLimited => None,
                other => panic!("unexpected message while awaiting Pong: {other:?}"),
            })
            .await;
        assert_eq!(got, nano, "Pong nano does not match the Ping we sent");
    }

    /// Drain raw messages until a Close frame or stream end. Returns the Close
    /// frame's `code` + `reason` when one was observed; `None` if the stream
    /// ended without an inspectable Close frame.
    async fn recv_close(&mut self) -> Option<CloseFrame> {
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

    /// Initiate a clean WebSocket close handshake and drain until stream end,
    /// so the server has observed the Close before this returns.
    async fn close(mut self) {
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

/// The kick Close-frame contract: wire code 1008 == `CloseCode::Policy`, AND
/// the documented reason (exact match). Mirrors Family-3's `assert_kick_frame`.
fn assert_kick_frame(frame: &CloseFrame) {
    assert_eq!(
        u16::from(frame.code),
        1008,
        "kick must close with wire code 1008, got {:?}",
        frame.code
    );
    assert_eq!(
        frame.code,
        CloseCode::Policy,
        "kick close code must be CloseCode::Policy"
    );
    assert_eq!(
        frame.reason.as_str(),
        KICK_REASON,
        "kick Close reason must be the documented string"
    );
}

/// Drain to the kick: an inspectable Close frame (code 1008 + documented
/// reason) must arrive AND the stream must END right after it. A bare stream
/// end is a failure — the server always attaches code + reason when kicking.
async fn expect_kick(client: &mut TlsTestClient) {
    let frame = client.recv_close().await.expect(
        "kick must send an inspectable Close frame (code + reason); the stream ended bare instead",
    );
    assert_kick_frame(&frame);
    assert!(
        client.recv_raw().await.is_none(),
        "stream must end after the kick Close frame"
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Pre-flight: confirm the wire codec these tests use matches the server's
/// (`AppState.codec = Codec::PostcardLz4`). A mismatch would make every decode
/// fail and silently defeat the signature assertions, so pin it directly.
#[test]
fn test_codec_matches_server() {
    // `Codec` implements neither `PartialEq` nor `Debug`, so pin via `matches!`.
    assert!(
        matches!(TEST_CODEC, Codec::PostcardLz4),
        "TLS client codec must match the server's PostcardLz4"
    );
}

/// Test 1: a validly signed `wss://` round trip SURVIVES under `Kick`.
///
/// Full handshake over TLS to `JoinAccepted`, then ~15 signed `InputFrame`s
/// (varying seq / tick_ms / blob), with snapshots draining mid-battery, then
/// an in-order `Ping → Pong`. Under `Kick`, any signature that failed
/// verification would have closed the connection — so completing the Pong
/// after every frame PROVES `on_client_msg_tls` ACCEPTS valid signatures over
/// TLS (the "stay semantically identical" comment, verified). Clean close.
#[tokio::test(flavor = "multi_thread")]
async fn tls_signed_round_trip_survives_under_kick() {
    let key = SigningKey::dev_default();
    let server = spawn_tls_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    let (mut client, info) =
        TlsTestClient::connect_and_join(server.ws_addr(), "f5-rt", "coop").await;
    assert!(!info.room_id.is_empty(), "room_id must be non-empty");
    assert!(!info.player_id.is_empty(), "player_id must be non-empty");
    assert_eq!(info.tick_hz, 30, "server rooms tick at 30 Hz");

    // 15 frames: seq 0 / non-monotonic / u32::MAX, tick_ms 0 / u64::MAX,
    // blobs empty → 1 KiB with all byte values. Under the 30-token bucket, so
    // RateLimited cannot fire and mask anything.
    let frames: Vec<(u32, u64, Vec<u8>)> = {
        let mut v: Vec<(u32, u64, Vec<u8>)> = vec![
            (1, 0, Vec::new()),
            (2, 33, b"forward".to_vec()),
            (3, 66, vec![0u8; 256]),
            (4, 99, (0u8..=255).collect()),
            (5, u64::MAX, vec![0xAA; 1024]),
            (0, 5, vec![1, 2, 3]),
            (u32::MAX, 12_345, vec![0xFF; 64]),
        ];
        for i in 7u32..15 {
            let blob: Vec<u8> = (0..i * 5).map(|b| (b % 251) as u8).collect();
            v.push((i, u64::from(i) * 33, blob));
        }
        assert_eq!(v.len(), 15);
        v
    };

    let mid = frames.len() / 2;
    let mut tick_mid = 0u64;
    for (i, (seq, tick_ms, blob)) in frames.iter().enumerate() {
        client.send_input_frame(*seq, *tick_ms, blob, &key).await;
        if i == mid {
            tick_mid = client
                .recv_until(|m| match m {
                    ServerToClient::Snapshot {
                        compressed,
                        payload,
                        ..
                    } => Some(decode_snapshot_tick(compressed, &payload)),
                    _ => None,
                })
                .await;
            assert!(
                tick_mid >= 1,
                "snapshot tick must have advanced mid-battery"
            );
        }
    }

    // In-order Pong after all 15 signed frames == zero verification failures
    // over TLS. This is the load-bearing assertion of the family.
    client.assert_ping_pong(0xF5_DE_77).await;

    // Snapshots still flowing afterward, with a strictly advancing room tick.
    let tick_after = client
        .recv_until(|m| match m {
            ServerToClient::Snapshot {
                compressed,
                payload,
                ..
            } => Some(decode_snapshot_tick(compressed, &payload)),
            _ => None,
        })
        .await;
    assert!(
        tick_after > tick_mid,
        "snapshot tick must keep advancing over TLS ({tick_after} after {tick_mid})"
    );

    client.close().await;
    server.shutdown();
}

/// Test 2: a wrong-key `wss://` client is KICKED via a REAL Close 1008.
///
/// Anti-vacuity FIRST: a key-A TLS client survives signed frames + an in-order
/// Pong on this exact server, so the kick below is attributable to the KEY,
/// not to TLS setup failing. Then a key-B client's first signed frame draws
/// the kick: Close code 1008 / `CloseCode::Policy` + the documented reason,
/// then stream end — proving `on_client_msg_tls` REJECTS bad signatures over
/// TLS and routes the kick through the real close + cleanup path.
#[tokio::test(flavor = "multi_thread")]
async fn tls_wrong_key_kicked_via_real_close() {
    let server = spawn_tls_test_server(key_a(), SignatureFailurePolicy::Kick).await;

    // Anti-vacuity control: key-A TLS client survives on this exact server.
    let (mut control, _info) =
        TlsTestClient::connect_and_join(server.ws_addr(), "f5-kick", "coop").await;
    for seq in 1u32..=3 {
        control
            .send_input_frame(seq, u64::from(seq) * 33, &[seq as u8], &key_a())
            .await;
    }
    control.assert_ping_pong(0xF52A).await;
    control.close().await;

    // Wrong-key client: the (unsigned) handshake succeeds over TLS, then the
    // first key-B-signed frame draws the kick.
    let (mut wrong, _info) =
        TlsTestClient::connect_and_join(server.ws_addr(), "f5-kick", "coop").await;
    wrong
        .send_input_frame(1, 33, b"wrong-key-frame-over-tls", &key_b())
        .await;
    expect_kick(&mut wrong).await;

    server.shutdown();
}

/// Test 3: wrong key under `Warn` STAYS OPEN over TLS.
///
/// On a Warn TLS server, a wrong-key client keeps receiving snapshots and
/// completes in-order Pongs after several wrong-key frames — the connection is
/// NOT closed (closing the fenced TLS-Warn coverage gap).
///
/// Anti-vacuity: a Kick-policy TLS TWIN server (same key A) KICKS the identical
/// wrong-key frame within this test, proving the key-B signatures genuinely
/// FAIL verification against key A — i.e. the Warn server was surviving
/// verification FAILURES, and only the policy differs between the two paths.
#[tokio::test(flavor = "multi_thread")]
async fn tls_wrong_key_under_warn_stays_open() {
    let server_key = key_a();
    let wrong_key = key_b();
    let warn_server = spawn_tls_test_server(server_key.clone(), SignatureFailurePolicy::Warn).await;

    let (mut client, _info) =
        TlsTestClient::connect_and_join(warn_server.ws_addr(), "f5-warn", "coop").await;

    // Batch 1: five wrong-key frames; in-order Pong proves none disconnected us.
    for seq in 1u32..=5 {
        client
            .send_input_frame(seq, u64::from(seq) * 33, b"warn-tls-batch-1", &wrong_key)
            .await;
    }
    client.assert_ping_pong(0x57B1).await;

    // Snapshot stream still alive and advancing under Warn over TLS.
    let t1 = client.recv_snapshot_server_tick().await;
    let t2 = client.recv_snapshot_server_tick().await;
    assert!(
        t2 > t1,
        "snapshots must keep arriving under Warn over TLS ({t2} after {t1})"
    );

    // Batch 2: more wrong-key frames still accepted; another in-order Pong.
    for seq in 6u32..=10 {
        client
            .send_input_frame(seq, u64::from(seq) * 33, b"warn-tls-batch-2", &wrong_key)
            .await;
    }
    client.assert_ping_pong(0x57B2).await;

    // Full-processing proof: a burst overruns the token bucket (30 initial,
    // 60 cap, 8/s refill, 1/frame), so the server MUST reply RateLimited —
    // only the InputFrame processing path can emit it. That the unauthenticated
    // frames reach the rate-limit path proves Warn processes them (parity with
    // the plain Family-3 Warn proof). The connection stays open after.
    for seq in 11u32..=110 {
        client
            .send_input_frame(seq, u64::from(seq) * 33, b"warn-tls-burst", &wrong_key)
            .await;
    }
    client
        .recv_until(|m| match m {
            ServerToClient::RateLimited => Some(()),
            ServerToClient::Snapshot { .. } => None,
            other => panic!("unexpected message while awaiting RateLimited: {other:?}"),
        })
        .await;
    client
        .assert_ping_pong_tolerating_rate_limited(0x57B3)
        .await;
    client.close().await;

    // Anti-vacuity twin: the identical first wrong-key frame KICKS under Kick
    // over TLS, with the full Close 1008 contract — proving the signatures
    // genuinely fail and ONLY the policy differs.
    let kick_twin = spawn_tls_test_server(server_key, SignatureFailurePolicy::Kick).await;
    let (mut twin, _info) =
        TlsTestClient::connect_and_join(kick_twin.ws_addr(), "f5-warn-twin", "coop").await;
    twin.send_input_frame(1, 33, b"warn-tls-batch-1", &wrong_key)
        .await;
    expect_kick(&mut twin).await;

    warn_server.shutdown();
    kick_twin.shutdown();
}
