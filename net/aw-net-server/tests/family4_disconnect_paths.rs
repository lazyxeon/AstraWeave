//! W.3 Family-4: disconnect-path robustness.
//!
//! The kick machinery W.2.b wired (signature-failure Close 1008 + the shared
//! cleanup block all loop exits funnel into) must be robust under every
//! disconnect path it inherits: abrupt socket death (recv-arm `Some(Err)` /
//! `None`), abrupt death mid-snapshot-stream (snapshot-arm send failure —
//! the W.2.b-fix1 path), server-initiated kicks, close/drop races, churn,
//! pre-join aborts, and `RunningServer::shutdown()`.
//!
//! Battery map (test fn → spec item):
//! 1. [`abrupt_drop_mid_stream_cleans_up_and_keeps_serving`]
//! 2. [`abrupt_drop_during_active_input_no_ghost_server_survives`]
//! 3. [`signature_kick_full_lifecycle_with_client_double_close`]
//! 4. [`close_frame_then_immediate_drop_race_three_iterations`]
//! 5. [`reconnect_with_correct_key_after_kick_three_cycles`]
//! 6. [`kick_mid_snapshot_stream_close_arrives_cleanly`]
//! 7. [`mass_churn_ten_cycles_no_observable_accumulation`]
//! 8. [`shutdown_with_live_connections_pins_actual_semantics`]
//! 9. [`drop_after_hello_before_join_leaves_server_healthy`]
//!
//! ## Pinned `shutdown()` semantics (the workflow's record — see test 8)
//!
//! `RunningServer::shutdown` (net/aw-net-server/src/lib.rs:118-125) aborts
//! exactly two tasks: `ws_task` (the accept loop, spawned at lib.rs:298 for
//! the plain-TCP path) and `http_task`. Per-connection tasks are spawned via
//! `tokio::spawn` INSIDE the accept loop (lib.rs:346) — tokio tasks are
//! independent runtime citizens, not children of their spawner — so aborting
//! the accept loop does NOT touch them. Consequences, both asserted by
//! test 8:
//! - aborting `ws_task` drops the `TcpListener` it owns, so NEW connection
//!   attempts are refused (or, in a backlog race, never answered);
//! - EXISTING connections keep running their full select loop (snapshots
//!   keep streaming, Ping→Pong keeps working) until their sockets close —
//!   exactly as the `shutdown` doc-comment claims ("Already-established
//!   connection tasks are detached and end when their sockets close").
//!
//! ## Abrupt-drop fidelity
//!
//! `TestClient` declares no `Drop` impl (common/mod.rs), and neither
//! tungstenite's `WebSocketStream` nor tokio's `TcpStream` can perform a
//! close HANDSHAKE in a synchronous drop — dropping the client closes the
//! raw socket handle with NO WebSocket Close frame ever emitted (and, with
//! unread 30 Hz snapshot bytes pending in the receive buffer, typically an
//! RST rather than a graceful FIN). `drop(TestClient)` therefore IS the
//! abrupt drop these tests need; no separate raw connection is required.
//! (w2b_fix1_regressions.rs already relies on the same property.)
//!
//! ## Anti-vacuity discipline
//!
//! - every "server fine" claim = a subsequent authenticated round-trip
//!   (signed InputFrame under `SignatureFailurePolicy::Kick` + exact-nano
//!   Pong — any verification failure would have disconnected the probe);
//! - every "cleanup ran" claim = the room-rotation/no-ghost pattern from
//!   w2b_fix1 (rooms are removed ONLY in the shared cleanup block and ONLY
//!   when their player map empties, so a fresh room id in the same region
//!   proves the old entry was removed and the room reaped);
//! - every kick = an explicitly observed Close frame (code 1008 + reason);
//! - no raw sleeps as synchronization — only deadline-bounded polls (the
//!   50 ms pacing sleep inside those polls matches the blessed w2b pattern).

mod common;

use std::collections::HashSet;
use std::net::SocketAddr;
use std::time::Duration;

use aw_net_proto::{
    decode_msg, encode_msg, input_frame_sig_payload, sign, ClientToServer, ServerToClient,
    SigningKey, PROTOCOL_VERSION,
};
use aw_net_server::SignatureFailurePolicy;
use common::{spawn_test_server, JoinInfo, TestClient, TestServer, IO_TIMEOUT, TEST_CODEC};
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::error::ProtocolError;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

/// A 64-hex-char (32-byte) key that is NOT the server's dev-default key.
/// Frames signed with it are canonically well-formed but fail verification
/// on a dev-default server — the signature-kick trigger.
const WRONG_KEY_HEX: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

fn wrong_key() -> SigningKey {
    SigningKey::from_hex(WRONG_KEY_HEX).expect("valid 64-hex wrong key")
}

// ---------------------------------------------------------------------------
// Shared assertion helpers
// ---------------------------------------------------------------------------

/// The signature-kick Close-frame contract: code 1008 (Policy) and a reason
/// naming the signature failure (lib.rs Kick arm: "input frame signature
/// verification failed").
fn assert_policy_close(frame: &CloseFrame) {
    assert_eq!(
        u16::from(frame.code),
        1008,
        "signature kick must close with 1008 (Policy), got {:?}",
        frame.code
    );
    assert!(
        frame.reason.as_str().contains("signature"),
        "kick Close reason must name the signature failure, got: {:?}",
        frame.reason
    );
}

/// Trigger a signature kick (wrong-key frame) and require the explicit
/// Close 1008 observation — the load-bearing kick proof.
async fn expect_signature_kick(client: &mut TestClient, seq: u32) -> CloseFrame {
    client
        .send_input_frame(seq, 33, b"f4-wrong-key", &wrong_key())
        .await;
    let frame = client
        .recv_close()
        .await
        .expect("signature kick must carry an inspectable Close frame (code + reason)");
    assert_policy_close(&frame);
    frame
}

/// Authenticated round-trip: one validly signed frame, then an exact-nano
/// Pong. Under the Kick policy, the in-order Pong proves the frame was
/// accepted and the connection is fully served — the "server fine" proof.
async fn assert_authenticated_round_trip(client: &mut TestClient, key: &SigningKey, nano: u128) {
    client.send_input_frame(1, 33, b"f4-probe", key).await;
    client.assert_ping_pong(nano).await;
}

/// Next snapshot id off the live stream (skipping non-snapshot traffic).
async fn next_snapshot_id(client: &mut TestClient) -> u32 {
    client
        .recv_until(|m| match m {
            ServerToClient::Snapshot { id, .. } => Some(id),
            _ => None,
        })
        .await
}

/// Bounded poll: keep joining `region` (with clean-close probes) until
/// matchmaking hands out a room id different from `old_room`.
///
/// Pattern lifted from tests/w2b_fix1_regressions.rs (test binaries cannot
/// import each other and common/ is frozen, so the helper is replicated
/// here). Rooms are removed ONLY in the connection handlers' shared cleanup
/// block, and ONLY when their player map is empty; matchmaking reuses any
/// existing room in the region with < 4 players. A NEW room id therefore
/// proves `old_room` was dropped — i.e. the departed peer's player entry was
/// removed (no ghost) and the empty room was reaped. If cleanup never ran,
/// the ghost keeps `old_room` matchmaking-visible forever and this loop
/// panics at the deadline.
async fn await_room_rotation(
    server: &TestServer,
    region: &str,
    old_room: &str,
) -> (TestClient, JoinInfo) {
    let deadline = tokio::time::Instant::now() + IO_TIMEOUT;
    loop {
        let (probe, info) = TestClient::connect_and_join(server.ws_addr(), region, "coop").await;
        if info.room_id != old_room {
            return (probe, info);
        }
        // Cleanup of the departed peer has not run yet — this probe landed in
        // the old room alongside the lingering entry. Leave cleanly and retry.
        probe.close().await;
        assert!(
            tokio::time::Instant::now() < deadline,
            "room {old_room} still matchmaking-visible after {IO_TIMEOUT:?}: \
             ghost player leak (connection cleanup never ran)"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

// ---------------------------------------------------------------------------
// Local raw-WebSocket helpers (test 3 only)
//
// Harness gap: `TestClient` panics on ANY send error (`send_ws`, `close`),
// but a client-side Close sent AFTER the server's Close was already read is
// legitimately rejected by tungstenite 0.28 with
// `Protocol(SendAfterClosing)` (tungstenite protocol/mod.rs `write`: the
// `!is_active()` check precedes the Close special-case), and the flush can
// hit the server's already-torn-down socket. The double-close test therefore
// needs an error-TOLERANT client; common/ is frozen, so a minimal raw client
// lives here. Every await is bounded by `IO_TIMEOUT`.
// ---------------------------------------------------------------------------

type RawWs = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn raw_connect(ws_addr: SocketAddr) -> RawWs {
    let (ws, _resp) = timeout(IO_TIMEOUT, connect_async(format!("ws://{ws_addr}")))
        .await
        .expect("raw ws connect timed out")
        .expect("raw ws connect failed");
    ws
}

async fn raw_send_msg(ws: &mut RawWs, msg: &ClientToServer) {
    let bytes = encode_msg(TEST_CODEC, msg);
    timeout(IO_TIMEOUT, ws.send(Message::Binary(bytes.into())))
        .await
        .expect("raw ws send timed out")
        .expect("raw ws send failed");
}

async fn raw_recv_msg(ws: &mut RawWs) -> ServerToClient {
    loop {
        match timeout(IO_TIMEOUT, ws.next())
            .await
            .expect("raw ws recv timed out")
        {
            Some(Ok(Message::Binary(b))) => {
                return decode_msg::<ServerToClient>(TEST_CODEC, &b)
                    .expect("failed to decode ServerToClient frame")
            }
            Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => continue,
            other => panic!("expected binary ServerToClient frame, got {other:?}"),
        }
    }
}

/// Real protocol handshake to a joined session (mirrors `TestClient::join`).
async fn raw_join(ws: &mut RawWs, region: &str) -> String {
    raw_send_msg(
        ws,
        &ClientToServer::Hello {
            protocol: PROTOCOL_VERSION,
        },
    )
    .await;
    match raw_recv_msg(ws).await {
        ServerToClient::HelloAck { protocol } => assert_eq!(protocol, PROTOCOL_VERSION),
        other => panic!("expected HelloAck, got {other:?}"),
    }
    raw_send_msg(
        ws,
        &ClientToServer::FindOrCreate {
            region: region.to_string(),
            game_mode: "coop".to_string(),
            party_size: 1,
        },
    )
    .await;
    let matched = match raw_recv_msg(ws).await {
        ServerToClient::MatchResult { room_id } => room_id,
        other => panic!("expected MatchResult, got {other:?}"),
    };
    match raw_recv_msg(ws).await {
        ServerToClient::JoinAccepted { room_id, .. } => {
            assert_eq!(
                room_id, matched,
                "JoinAccepted room differs from MatchResult"
            );
            room_id
        }
        other => panic!("expected JoinAccepted, got {other:?}"),
    }
}

/// Drain raw frames until a Close frame (returned) or stream end / dead
/// transport (`None`). Deadline-bounded.
async fn raw_recv_close(ws: &mut RawWs) -> Option<CloseFrame> {
    let deadline = tokio::time::Instant::now() + IO_TIMEOUT;
    loop {
        assert!(
            tokio::time::Instant::now() < deadline,
            "raw_recv_close: no Close frame within {IO_TIMEOUT:?}"
        );
        match timeout(IO_TIMEOUT, ws.next())
            .await
            .expect("raw ws recv timed out")
        {
            Some(Ok(Message::Close(frame))) => return frame,
            Some(Ok(_)) => continue,
            Some(Err(_)) | None => return None,
        }
    }
}

/// The benign error classes a client-side close may hit AFTER the server has
/// already closed (frame-level close received and/or socket torn down).
/// Anything else is a real failure.
fn assert_benign_double_close_error(e: WsError) {
    match e {
        WsError::ConnectionClosed | WsError::AlreadyClosed => {}
        WsError::Protocol(ProtocolError::SendAfterClosing) => {}
        WsError::Protocol(ProtocolError::ResetWithoutClosingHandshake) => {}
        WsError::Io(_) => {} // server socket already gone (FIN/RST) — expected
        other => panic!("double-close must only fail with closed-class errors, got: {other}"),
    }
}

/// Client-side close attempt on a connection the SERVER already closed:
/// must complete within the bounded window (no hang) with either success or
/// a benign closed-class error. Two layers: an explicit `Message::Close`
/// send (exercises tungstenite's SendAfterClosing path) and a `SinkExt::
/// close` (drives tungstenite's close machinery, flushing the auto-queued
/// reply close if pending).
async fn raw_double_close(ws: &mut RawWs) {
    match timeout(IO_TIMEOUT, ws.send(Message::Close(None)))
        .await
        .expect("double-close send timed out (must never hang)")
    {
        Ok(()) => {}
        Err(e) => assert_benign_double_close_error(e),
    }
    match timeout(IO_TIMEOUT, SinkExt::close(ws))
        .await
        .expect("double-close drive timed out (must never hang)")
    {
        Ok(()) => {}
        Err(e) => assert_benign_double_close_error(e),
    }
}

/// Drain a raw connection to stream end (or dead transport). Bounded.
async fn raw_drain_to_end(ws: &mut RawWs) {
    let deadline = tokio::time::Instant::now() + IO_TIMEOUT;
    loop {
        assert!(
            tokio::time::Instant::now() < deadline,
            "raw_drain_to_end: stream did not end within {IO_TIMEOUT:?}"
        );
        match timeout(IO_TIMEOUT, ws.next())
            .await
            .expect("raw ws drain timed out")
        {
            None | Some(Err(_)) => return,
            Some(Ok(_)) => continue,
        }
    }
}

// ---------------------------------------------------------------------------
// 1. Abrupt drop mid-stream
// ---------------------------------------------------------------------------

/// A client that is demonstrably mid-snapshot-stream (two snapshots with
/// advancing ids observed) vanishes abruptly — no Close handshake. The
/// server must remove the player and reap the room within a bounded window
/// (room rotation) and keep serving authenticated traffic.
#[tokio::test(flavor = "multi_thread")]
async fn abrupt_drop_mid_stream_cleans_up_and_keeps_serving() {
    const REGION: &str = "f4-drop-stream";
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    let (mut a, info_a) = TestClient::connect_and_join(server.ws_addr(), REGION, "coop").await;

    // Mid-stream proof: two snapshots with advancing ids actually received.
    let snap_1 = next_snapshot_id(&mut a).await;
    let snap_2 = next_snapshot_id(&mut a).await;
    assert!(
        snap_2 > snap_1,
        "snapshot ids must advance mid-stream ({snap_2} after {snap_1})"
    );

    // Abrupt drop: no WS Close frame is emitted (see module doc) — the
    // server-side recv arm observes a dead transport, not a close handshake.
    drop(a);

    // Cleanup proof (bounded) + server-health proof (authenticated round
    // trip on the rotated client).
    let (mut b, _info_b) = await_room_rotation(&server, REGION, &info_a.room_id).await;
    assert_authenticated_round_trip(&mut b, &key, 0xF4_01).await;

    b.close().await;
    server.shutdown();
}

// ---------------------------------------------------------------------------
// 2. Abrupt drop DURING active input
// ---------------------------------------------------------------------------

/// The closest external trigger for the snapshot-send-failure path
/// (W.2.b-fix1): join, send one validly signed frame, then drop the socket
/// immediately WITHOUT reading anything further — unread 30 Hz snapshot
/// bytes are pending client-side, so the drop typically RSTs and the
/// server's next snapshot send (or its next recv poll) hits a dead socket.
/// WHICH select arm detects the death is documented as non-deterministic
/// (w2b_fix1); the invariant asserted here is arm-independent: no ghost
/// player (room rotation) and a healthy server (authenticated round trip).
#[tokio::test(flavor = "multi_thread")]
async fn abrupt_drop_during_active_input_no_ghost_server_survives() {
    const REGION: &str = "f4-drop-input";
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    let (mut a, info_a) = TestClient::connect_and_join(server.ws_addr(), REGION, "coop").await;
    a.send_input_frame(1, 33, b"f4-active-input", &key).await;
    drop(a); // no reads after JoinAccepted — send buffers fill naturally

    let (mut b, _info_b) = await_room_rotation(&server, REGION, &info_a.room_id).await;
    assert_authenticated_round_trip(&mut b, &key, 0xF4_02).await;

    b.close().await;
    server.shutdown();
}

// ---------------------------------------------------------------------------
// 3. Server-initiated kick: full lifecycle with client double-close
// ---------------------------------------------------------------------------

/// Wrong-key frame → explicit Close 1008 received → the client then ALSO
/// sends its own Close onto the already-server-closed connection. The
/// double-close must neither hang the client (all awaits bounded) nor harm
/// the server (kicked room reaped + fresh authenticated round trip).
///
/// Uses the local raw client: `TestClient` panics on any send error, but a
/// close-after-server-close legitimately fails client-side with
/// `SendAfterClosing`/closed-class errors (see helper docs) — those are part
/// of the lifecycle under test, not failures.
#[tokio::test(flavor = "multi_thread")]
async fn signature_kick_full_lifecycle_with_client_double_close() {
    const REGION: &str = "f4-kick-life";
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    let mut ws = raw_connect(server.ws_addr()).await;
    let kicked_room = raw_join(&mut ws, REGION).await;

    // Canonically signed with the WRONG key → verification fails → Kick.
    let blob: &[u8] = b"f4-double-close";
    let payload = input_frame_sig_payload(7, 231, blob);
    raw_send_msg(
        &mut ws,
        &ClientToServer::InputFrame {
            seq: 7,
            tick_ms: 231,
            input_blob: blob.to_vec(),
            sig: sign(&wrong_key(), &payload),
        },
    )
    .await;

    // Explicit kick observation: Close 1008 with the signature reason.
    let frame = raw_recv_close(&mut ws)
        .await
        .expect("signature kick must deliver an inspectable Close frame");
    assert_policy_close(&frame);

    // Double-close: our own Close onto the server-closed connection, then
    // drain to stream end. Bounded, tolerant of closed-class errors only.
    raw_double_close(&mut ws).await;
    raw_drain_to_end(&mut ws).await;
    drop(ws);

    // Server health: kicked player's room reaped + fresh round trip.
    let (mut fresh, _info) = await_room_rotation(&server, REGION, &kicked_room).await;
    assert_authenticated_round_trip(&mut fresh, &key, 0xF4_03).await;

    fresh.close().await;
    server.shutdown();
}

// ---------------------------------------------------------------------------
// 4. Double-disconnect race: Close frame + immediate drop, 3 iterations
// ---------------------------------------------------------------------------

/// The client sends a proper Close frame and IMMEDIATELY drops the socket
/// without waiting for the server's close reply, so the server's reply (and
/// any in-flight snapshot send) races a dead socket. Per iteration the
/// server must run cleanup (room rotation) and keep serving (authenticated
/// round trip on the rotated probe). Three iterations widen the
/// timing-dependent net cheaply; each probe leaves with a clean close so
/// every iteration starts from an empty region.
#[tokio::test(flavor = "multi_thread")]
async fn close_frame_then_immediate_drop_race_three_iterations() {
    const REGION: &str = "f4-close-race";
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    for iteration in 0u32..3 {
        let (mut c, info) = TestClient::connect_and_join(server.ws_addr(), REGION, "coop").await;
        c.send_input_frame(1, 33, b"f4-race", &key).await;

        // Proper Close frame (send_ws flushes it onto the wire), then the
        // socket dies immediately — no waiting for the server's close reply.
        c.send_ws(Message::Close(None)).await;
        drop(c);

        let (mut probe, _probe_info) = await_room_rotation(&server, REGION, &info.room_id).await;
        assert_authenticated_round_trip(&mut probe, &key, 0xF4_04_00 + u128::from(iteration)).await;
        probe.close().await;
    }

    server.shutdown();
}

// ---------------------------------------------------------------------------
// 5. Reconnect-after-kick, 3 cycles (compounding-leak check)
// ---------------------------------------------------------------------------

/// A kicked (wrong-key) client's "user" immediately reconnects on a fresh
/// connection with the CORRECT key: the full handshake and an authenticated
/// round trip must work. Three kick→reconnect cycles — a leak in the kick
/// path would compound (pattern from w2b_fix1 test 2). Each cycle requires:
/// the explicit Close 1008, the kicked room reaped (rotation — the rotated
/// client IS the reconnect), and the reconnect's round trip.
#[tokio::test(flavor = "multi_thread")]
async fn reconnect_with_correct_key_after_kick_three_cycles() {
    const REGION: &str = "f4-rekick";
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    for cycle in 0u32..3 {
        let (mut victim, info) =
            TestClient::connect_and_join(server.ws_addr(), REGION, "coop").await;
        let _frame = expect_signature_kick(&mut victim, cycle + 1).await;
        drop(victim);

        // Immediate reconnect with the correct key: full handshake happens
        // inside the rotation poll (connect_and_join), and the rotation
        // itself proves the kicked entry did not leak this cycle.
        let (mut reborn, _reborn_info) = await_room_rotation(&server, REGION, &info.room_id).await;
        assert_authenticated_round_trip(&mut reborn, &key, 0xF4_05_00 + u128::from(cycle)).await;
        reborn.close().await;
    }

    server.shutdown();
}

// ---------------------------------------------------------------------------
// 6. Kick during snapshot streaming
// ---------------------------------------------------------------------------

/// The kick must interleave cleanly with the 30 Hz snapshot arm: a client
/// that is demonstrably mid-stream (three snapshots with strictly advancing
/// ids) sends the wrong-key frame; the Close 1008 must still arrive within
/// the bounded window with snapshots in flight. `recv_close` drains
/// non-Close traffic while waiting (verified against common/mod.rs: it loops
/// over `recv_raw`, skipping `Some(_)` non-Close messages, under an
/// IO_TIMEOUT deadline whose expiry panics) — so this test failing on a
/// lost/corrupted close is loud, not silent.
#[tokio::test(flavor = "multi_thread")]
async fn kick_mid_snapshot_stream_close_arrives_cleanly() {
    const REGION: &str = "f4-kick-stream";
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    let (mut c, info) = TestClient::connect_and_join(server.ws_addr(), REGION, "coop").await;

    // Mid-stream proof: >= 3 snapshots, strictly advancing ids.
    let s1 = next_snapshot_id(&mut c).await;
    let s2 = next_snapshot_id(&mut c).await;
    let s3 = next_snapshot_id(&mut c).await;
    assert!(
        s1 < s2 && s2 < s3,
        "snapshot ids must advance strictly while streaming ({s1}, {s2}, {s3})"
    );

    // Kick with snapshots in flight: the Close 1008 must arrive within the
    // recv_close deadline despite interleaved snapshot frames.
    let _frame = expect_signature_kick(&mut c, 1).await;
    drop(c);

    // Server health after the interleaved kick.
    let (mut fresh, _info) = await_room_rotation(&server, REGION, &info.room_id).await;
    assert_authenticated_round_trip(&mut fresh, &key, 0xF4_06).await;

    fresh.close().await;
    server.shutdown();
}

// ---------------------------------------------------------------------------
// 7. Mass churn
// ---------------------------------------------------------------------------

/// 10 sequential connect→join→leave cycles on ONE server, alternating clean
/// close and abrupt drop. Resource-leak smoke, black-box: afterwards a final
/// client must (a) land in a room id never seen during the churn — since
/// matchmaking reuses ANY existing room in the region with < 4 players, a
/// fresh id proves every churn room was reaped (rooms rotated, no
/// accumulation observable) — and (b) complete an authenticated round trip.
#[tokio::test(flavor = "multi_thread")]
async fn mass_churn_ten_cycles_no_observable_accumulation() {
    const REGION: &str = "f4-churn";
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    let mut seen_rooms: HashSet<String> = HashSet::new();
    for cycle in 0u32..10 {
        let (mut c, info) = TestClient::connect_and_join(server.ws_addr(), REGION, "coop").await;
        seen_rooms.insert(info.room_id.clone());
        c.send_input_frame(1, 33, b"f4-churn", &key).await;
        if cycle % 2 == 0 {
            // Clean close: by the harness close() contract the server has
            // observed the Close AND finished cleanup before this returns.
            c.close().await;
        } else {
            // Abrupt drop: cleanup is asynchronous; the final bounded poll
            // below absorbs the lag.
            drop(c);
        }
    }
    assert!(
        !seen_rooms.is_empty(),
        "churn must have observed at least one room"
    );

    // Bounded poll for full convergence: a final client in a NEVER-seen room.
    let deadline = tokio::time::Instant::now() + IO_TIMEOUT;
    let (mut fin, fin_info) = loop {
        let (probe, info) = TestClient::connect_and_join(server.ws_addr(), REGION, "coop").await;
        if !seen_rooms.contains(&info.room_id) {
            break (probe, info);
        }
        probe.close().await;
        assert!(
            tokio::time::Instant::now() < deadline,
            "churn rooms still matchmaking-visible after {IO_TIMEOUT:?}: \
             player/room accumulation (cleanup did not run for some cycle)"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    assert!(
        !seen_rooms.contains(&fin_info.room_id),
        "final client must land in a fresh room"
    );
    assert_authenticated_round_trip(&mut fin, &key, 0xF4_07).await;

    fin.close().await;
    server.shutdown();
}

// ---------------------------------------------------------------------------
// 8. Shutdown with live connections — pins the ACTUAL semantics
// ---------------------------------------------------------------------------

/// PINNED SHUTDOWN SEMANTICS (the workflow's record; full derivation in the
/// module doc): `RunningServer::shutdown` (lib.rs:118-125) aborts ONLY the
/// accept-loop task (`ws_task`, spawned at lib.rs:298) and the HTTP admin
/// task. Per-connection tasks are `tokio::spawn`ed inside the accept loop
/// (lib.rs:346) and are therefore independent of it — they SURVIVE
/// `shutdown()` and keep their full select loop (snapshots + Ping→Pong)
/// running until their own sockets close. Asserted here:
/// 1. new connections are refused after shutdown (bounded poll: a connect
///    attempt errors or goes unanswered; a successful handshake means the
///    abort had not landed yet → retry until deadline);
/// 2. BOTH pre-shutdown clients still receive advancing snapshots AND
///    complete exact-nano Ping→Pongs AFTER new connections are provably
///    refused.
/// If a future change makes shutdown() also tear down connection tasks,
/// assertion 2 fails — deliberately: this test is the semantics record.
#[tokio::test(flavor = "multi_thread")]
async fn shutdown_with_live_connections_pins_actual_semantics() {
    const REGION: &str = "f4-shutdown";
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;
    let ws_addr = server.ws_addr();

    // Two live, joined, demonstrably streaming clients (same room: same
    // region + game_mode with < 4 occupants must coalesce).
    let (mut c1, i1) = TestClient::connect_and_join(ws_addr, REGION, "coop").await;
    let (mut c2, i2) = TestClient::connect_and_join(ws_addr, REGION, "coop").await;
    assert_eq!(i1.room_id, i2.room_id, "matchmaking must reuse the room");
    let c1_pre = next_snapshot_id(&mut c1).await;
    let c2_pre = next_snapshot_id(&mut c2).await;

    server.shutdown();

    // (1) New connections refused, bounded. After the abort lands, the
    // dropped TcpListener refuses TCP connects outright; in the tiny
    // backlog race a connect may succeed at TCP level but the WS handshake
    // is never answered (no accept loop), which the per-attempt timeout
    // classifies as refused.
    let url = format!("ws://{ws_addr}");
    let deadline = tokio::time::Instant::now() + IO_TIMEOUT;
    loop {
        match timeout(Duration::from_secs(2), connect_async(&url)).await {
            Err(_elapsed) => break,     // handshake never answered — accept loop gone
            Ok(Err(_refused)) => break, // TCP refused / transport error — listener closed
            Ok(Ok((raced_ws, _resp))) => {
                // The accept loop processed one more connection before the
                // abort landed — retry under the deadline.
                drop(raced_ws);
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "accept loop still accepting {IO_TIMEOUT:?} after shutdown()"
                );
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }

    // (2) Existing connections keep streaming and answering, AFTER the
    // refusal above proved the accept loop is dead.
    let c1_post = next_snapshot_id(&mut c1).await;
    let c2_post = next_snapshot_id(&mut c2).await;
    assert!(
        c1_post > c1_pre,
        "c1 snapshots must keep advancing after shutdown ({c1_post} after {c1_pre})"
    );
    assert!(
        c2_post > c2_pre,
        "c2 snapshots must keep advancing after shutdown ({c2_post} after {c2_pre})"
    );
    c1.assert_ping_pong(0xF4_08_01).await;
    c2.assert_ping_pong(0xF4_08_02).await;

    // Detached connection tasks end when their sockets close (here: client
    // drops at test end; the test runtime then tears down).
    drop(c1);
    drop(c2);
}

// ---------------------------------------------------------------------------
// 9. Disconnect before join (handshake abort)
// ---------------------------------------------------------------------------

/// Pre-room disconnect path: Hello → HelloAck, then the socket dies without
/// a join. The server is parked in the second `recv` (matchmaking phase);
/// the death routes through `room_id = None` → ProtocolError send onto the
/// dead socket → handler exit (lib.rs:671-684; `?`-return if the send fails,
/// `return Ok(())` if it lands in the doomed OS buffer — both pre-insertion).
/// No player was inserted and no room was created, so there is NO cleanup
/// entry to run — the assertion is pure server liveness: a fresh client
/// completes the full handshake and an authenticated round trip.
#[tokio::test(flavor = "multi_thread")]
async fn drop_after_hello_before_join_leaves_server_healthy() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    let mut aborter = TestClient::connect(server.ws_addr()).await;
    aborter
        .send_msg(&ClientToServer::Hello {
            protocol: PROTOCOL_VERSION,
        })
        .await;
    match aborter.recv_msg().await {
        ServerToClient::HelloAck { protocol } => assert_eq!(protocol, PROTOCOL_VERSION),
        other => panic!("expected HelloAck, got {other:?}"),
    }
    drop(aborter); // never joins — abrupt death between HelloAck and join

    // Server liveness: full handshake + authenticated round trip. (No room
    // rotation applies — the aborter never created or entered a room.)
    let (mut fresh, info) =
        TestClient::connect_and_join(server.ws_addr(), "f4-prejoin", "coop").await;
    assert!(!info.room_id.is_empty(), "room_id must be non-empty");
    assert!(!info.player_id.is_empty(), "player_id must be non-empty");
    assert_authenticated_round_trip(&mut fresh, &key, 0xF4_09).await;

    fresh.close().await;
    server.shutdown();
}
