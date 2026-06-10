//! W.3 Family-3: wrong-key and [`SignatureFailurePolicy`] behavior.
//!
//! Battery: the Kick default (asserted directly AND behaviorally), the kick
//! Close-frame contract (code 1008 + documented reason + stream end), full
//! disconnect-path cleanup after a kick (room reaping via the
//! room-rotation/no-ghost observation pattern from `w2b_fix1_regressions.rs`),
//! the no-partial-processing contract for rejected frames, Warn-policy legacy
//! processing (with an in-test Kick twin proving the signatures genuinely
//! fail), the Warn happy path, kick isolation in shared rooms, and `FromStr`
//! policy parsing.
//!
//! Proof conventions: every "stays open" claim is proven by subsequent
//! successful traffic (in-order Ping→Pong through the full protocol stack —
//! the server processes a connection's messages strictly in order); every
//! "kicked" claim is proven by an explicit Close-frame observation (code +
//! reason) plus stream end, bounded by [`common::IO_TIMEOUT`]. Servers run
//! on ephemeral ports with unique sled temp dirs (parallel-safe).

mod common;

use std::time::Duration;

use aw_net_proto::{decode_msg, ClientToServer, ServerToClient, SigningKey};
use aw_net_server::{ServerConfig, SignatureFailurePolicy};
use common::{spawn_test_server, JoinInfo, TestClient, TestServer, IO_TIMEOUT, TEST_CODEC};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;

/// Key A — the key configured on every server in this file (64 hex = 32 bytes).
const KEY_A_HEX: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

/// Key B — the WRONG key clients sign with. Differs from key A in every byte.
const KEY_B_HEX: &str = "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100";

/// The documented kick reason — the `MsgOutcome::Kick` payload in
/// `aw-net-server/src/lib.rs` carried into the Close frame.
const KICK_REASON: &str = "input frame signature verification failed";

fn key_a() -> SigningKey {
    SigningKey::from_hex(KEY_A_HEX).expect("KEY_A_HEX is a valid 64-hex key")
}

fn key_b() -> SigningKey {
    SigningKey::from_hex(KEY_B_HEX).expect("KEY_B_HEX is a valid 64-hex key")
}

/// Assert the full kick Close-frame contract: wire code 1008, which must map
/// to `CloseCode::Policy`, AND the documented reason string (exact match).
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

/// Drain to the kick: an inspectable Close frame must arrive (code 1008 +
/// documented reason) and the stream must END right after it (no zombie
/// session). The server always attaches code + reason when kicking, so a
/// bare stream end is a failure here.
async fn expect_kick(client: &mut TestClient) {
    let frame = client.recv_close().await.expect(
        "kick must send an inspectable Close frame (code + reason); the stream ended bare instead",
    );
    assert_kick_frame(&frame);
    assert!(
        client.recv_raw().await.is_none(),
        "stream must end after the kick Close frame"
    );
}

/// Strict variant of [`expect_kick`]: every message between our wrong-key
/// frame and the Close frame must be a `Snapshot` (the periodic stream that
/// is always flowing for a joined session). Any other reply — `RateLimited`
/// is the only direct reply the InputFrame processing path can produce —
/// would be observable evidence that the rejected frame WAS processed
/// before the kick.
async fn expect_kick_with_only_snapshots_before(client: &mut TestClient) {
    let deadline = tokio::time::Instant::now() + IO_TIMEOUT;
    let frame = loop {
        assert!(
            tokio::time::Instant::now() < deadline,
            "no Close frame within {IO_TIMEOUT:?}"
        );
        match client.recv_raw().await {
            Some(Message::Close(frame)) => {
                break frame.expect("kick must send an inspectable Close frame (code + reason)")
            }
            Some(Message::Binary(b)) => {
                let msg = decode_msg::<ServerToClient>(TEST_CODEC, &b)
                    .expect("failed to decode ServerToClient frame");
                assert!(
                    matches!(msg, ServerToClient::Snapshot { .. }),
                    "only periodic Snapshots may precede the kick Close — \
                     a rejected frame must produce no reply, got {msg:?}"
                );
            }
            Some(Message::Ping(_)) | Some(Message::Pong(_)) => {}
            Some(other) => panic!("unexpected ws frame before the kick Close: {other:?}"),
            None => panic!("stream ended without a Close frame — kick must send Close 1008"),
        }
    };
    assert_kick_frame(&frame);
    assert!(
        client.recv_raw().await.is_none(),
        "stream must end after the kick Close frame"
    );
}

/// Next snapshot's `server_tick`, skipping everything else on the stream.
/// Every `build_snapshot` call increments the room tick before sending, so
/// two successive reads on one connection must be strictly increasing.
async fn recv_snapshot_server_tick(client: &mut TestClient) -> u64 {
    client
        .recv_until(|m| match m {
            ServerToClient::Snapshot { server_tick, .. } => Some(server_tick),
            _ => None,
        })
        .await
}

/// Like `TestClient::assert_ping_pong`, but also skips queued `RateLimited`
/// replies. Local helper: the harness version panics on any non-Snapshot
/// message while awaiting the Pong, which makes it unusable right after a
/// deliberate rate-limit burst (harness gap, reported).
async fn assert_ping_pong_tolerating_rate_limited(client: &mut TestClient, nano: u128) {
    client.send_msg(&ClientToServer::Ping { nano }).await;
    let got = client
        .recv_until(|m| match m {
            ServerToClient::Pong { nano: n } => Some(n),
            ServerToClient::Snapshot { .. } | ServerToClient::RateLimited => None,
            other => panic!("unexpected message while awaiting Pong: {other:?}"),
        })
        .await;
    assert_eq!(got, nano, "Pong nano does not match the Ping we sent");
}

/// Bounded poll: keep joining `region` (with clean-close probes) until
/// matchmaking hands out a room id different from `old_room`. Returns the
/// successfully rotated client + join info.
///
/// Local copy of the room-rotation/no-ghost observation pattern established
/// in `tests/w2b_fix1_regressions.rs` — integration-test binaries cannot
/// import from each other and `common/mod.rs` does not export it (harness
/// gap, reported). Proof sketch: rooms are removed ONLY in the connection
/// handlers' shared cleanup block, and only when their player map is empty;
/// matchmaking reuses any existing room in the region with < 4 players.
/// Therefore a NEW room id proves `old_room` was dropped — i.e. the kicked
/// peer's player entry was removed (no ghost) and the empty room was reaped.
/// If the kick bypassed cleanup, the ghost entry would keep `old_room`
/// matchmaking-visible forever and this loop would panic at the deadline.
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
        // Cleanup of the kicked peer has not run yet — this probe landed in
        // the old room alongside the ghost entry. Leave cleanly and retry.
        probe.close().await;
        assert!(
            tokio::time::Instant::now() < deadline,
            "room {old_room} still matchmaking-visible after {IO_TIMEOUT:?}: \
             kicked player's room entry was not reaped (kick bypassed the shared cleanup block)"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Battery item 1a (direct): the policy default and the config default are
/// BOTH `Kick` — pinned with `matches!` so a `#[default]` move or a config
/// rewiring that changes the default fails here, not in production.
#[test]
fn default_policy_is_kick_direct() {
    assert!(
        matches!(
            SignatureFailurePolicy::default(),
            SignatureFailurePolicy::Kick
        ),
        "SignatureFailurePolicy::default() must be Kick"
    );
    assert!(
        matches!(
            ServerConfig::default().sig_failure_policy,
            SignatureFailurePolicy::Kick
        ),
        "ServerConfig::default() must carry the Kick policy"
    );
}

/// Battery item 1b (behavioral): a server spawned with the DEFAULT policy —
/// `SignatureFailurePolicy::default()` passed through the harness, exactly
/// what `ServerConfig::default()` wires — kicks a wrong-key client.
#[tokio::test(flavor = "multi_thread")]
async fn default_policy_kicks_wrong_key_client_behaviorally() {
    let server = spawn_test_server(key_a(), SignatureFailurePolicy::default()).await;

    let (mut wrong, _info) =
        TestClient::connect_and_join(server.ws_addr(), "f3-default", "coop").await;
    wrong
        .send_input_frame(1, 33, b"default-policy-probe", &key_b())
        .await;
    expect_kick(&mut wrong).await;

    server.shutdown();
}

/// Battery item 2: wrong key under Kick → the FIRST wrong-key frame draws a
/// Close frame with code 1008 AND the documented reason, then the stream
/// ends. Anti-vacuity: a key-A client on the SAME server first survives
/// signed frames + an in-order Pong, so the kick is attributable to the
/// key, not the environment.
#[tokio::test(flavor = "multi_thread")]
async fn wrong_key_under_kick_closes_1008_with_documented_reason() {
    let key = key_a();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    // Control: a key-A client survives on this exact server.
    let (mut control, _info) =
        TestClient::connect_and_join(server.ws_addr(), "f3-kick", "coop").await;
    for seq in 1u32..=3 {
        control
            .send_input_frame(seq, u64::from(seq) * 33, &[seq as u8], &key)
            .await;
    }
    control.assert_ping_pong(0xF32A).await;
    control.close().await;

    // Wrong-key client: the (unsigned) handshake succeeds, then the first
    // key-B-signed frame draws the kick.
    let (mut wrong, _info) =
        TestClient::connect_and_join(server.ws_addr(), "f3-kick", "coop").await;
    wrong
        .send_input_frame(1, 33, b"wrong-key-frame", &key_b())
        .await;
    expect_kick(&mut wrong).await;

    server.shutdown();
}

/// Battery item 3: the kick goes through the REAL disconnect path. After the
/// wrong-key kick, the room-rotation/no-ghost pattern proves the kicked
/// player's room entry was reaped (cleanup ran), and the rotated fresh
/// key-A client completes a full authenticated round trip on the same
/// server (connect/join/signed frame/in-order Pong).
#[tokio::test(flavor = "multi_thread")]
async fn kick_runs_real_disconnect_cleanup_room_reaped_and_server_serves() {
    const REGION: &str = "f3-reap";
    let key = key_a();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    // The wrong-key client is the sole occupant of a fresh room, then kicked.
    let (mut wrong, info_wrong) =
        TestClient::connect_and_join(server.ws_addr(), REGION, "coop").await;
    wrong.send_input_frame(1, 33, b"kick-me", &key_b()).await;
    expect_kick(&mut wrong).await;

    // No-ghost proof: matchmaking hands out a NEW room id for the same
    // region only after the kicked player's entry was removed and the empty
    // room dropped — i.e. the kick reached the shared cleanup block.
    let (mut fresh, info_fresh) = await_room_rotation(&server, REGION, &info_wrong.room_id).await;
    assert_ne!(
        info_fresh.room_id, info_wrong.room_id,
        "rotated client must land in a fresh room"
    );

    // Same fresh key-A client: full authenticated round trip post-kick.
    fresh.send_input_frame(1, 33, b"post-kick", &key).await;
    fresh.assert_ping_pong(0xF303).await;

    fresh.close().await;
    server.shutdown();
}

/// Battery item 4: under Kick, a wrong-key frame must not be processed at
/// all before the kick (the verify-first contract).
///
/// What IS provable black-box:
/// - The kick fires on the very FIRST in-session message (a wrong-key frame
///   carrying seq=999), and nothing but periodic Snapshots reaches the
///   client before the Close — in particular no `RateLimited`, the only
///   direct reply the InputFrame processing path can produce. The rejected
///   frame demonstrably did not run the reply-producing branch of the
///   legacy processing path.
/// - The kicked player leaves no residue that affects the next session: a
///   reconnecting key-A client gets a fresh working session — snapshots
///   ticking normally (strictly advancing server_tick) and a signed frame
///   with seq=1 (i.e. NOT shadowed by the rejected seq=999) accepted,
///   proven by an in-order Pong.
///
/// What is NOT provable black-box: the per-player fields the verify-first
/// contract protects (`last_input_seq`, rate-limit `tokens`, `last_seen`)
/// are private, are never echoed on the wire (the server never sends
/// `Reconcile`), and the player entry is destroyed by the kick cleanup
/// itself — so "the rejected frame mutated state that was then thrown away"
/// cannot be distinguished from "no mutation" by any wire observation. That
/// ordering (verify BEFORE seq/last_seen updates and token deduction) is
/// pinned at the code level by the early `MsgOutcome::Kick` return in
/// `on_client_msg`/`on_client_msg_tls`; this test pins every observable
/// consequence around it.
#[tokio::test(flavor = "multi_thread")]
async fn wrong_key_first_message_kicks_with_no_partial_processing() {
    const REGION: &str = "f3-no-partial";
    let key = key_a();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    let (mut victim, _info) = TestClient::connect_and_join(server.ws_addr(), REGION, "coop").await;
    victim
        .send_input_frame(999, 12_345, b"first-and-only", &key_b())
        .await;
    expect_kick_with_only_snapshots_before(&mut victim).await;

    // Reconnect: the fresh session must behave like nothing happened.
    let (mut fresh, _info) = TestClient::connect_and_join(server.ws_addr(), REGION, "coop").await;
    let t1 = recv_snapshot_server_tick(&mut fresh).await;
    let t2 = recv_snapshot_server_tick(&mut fresh).await;
    assert!(
        t2 > t1,
        "snapshots must tick normally in the fresh session ({t2} after {t1})"
    );
    fresh.send_input_frame(1, 33, b"fresh-session", &key).await;
    fresh.assert_ping_pong(0xF304).await;

    fresh.close().await;
    server.shutdown();
}

/// Battery item 5: wrong key under Warn → the connection STAYS OPEN and the
/// frames are fully processed (legacy behavior).
///
/// Stays-open proofs: in-order Pongs after each wrong-key batch plus the
/// snapshot stream still advancing. Full-processing proof: a burst past the
/// token-bucket capacity draws `RateLimited` — that reply is only reachable
/// THROUGH the InputFrame processing path (token deduction), so the
/// unauthenticated frames demonstrably went through legacy processing
/// rather than being silently dropped. The connection stays open even
/// after rate limiting (RateLimited is reply-only).
///
/// Anti-vacuity: the SAME wrong-key traffic shape kicks on a Kick-policy
/// twin server (same signing key A) within this test, proving the key-B
/// signatures genuinely fail verification against key A — i.e. the Warn
/// server really was surviving verification FAILURES, not passing them.
#[tokio::test(flavor = "multi_thread")]
async fn wrong_key_under_warn_stays_open_and_processes_frames() {
    let server_key = key_a();
    let wrong_key = key_b();
    let warn_server = spawn_test_server(server_key.clone(), SignatureFailurePolicy::Warn).await;

    let (mut client, _info) =
        TestClient::connect_and_join(warn_server.ws_addr(), "f3-warn", "coop").await;

    // Batch 1: five wrong-key frames; the in-order Pong proves all five
    // were accepted without disconnecting.
    for seq in 1u32..=5 {
        client
            .send_input_frame(seq, u64::from(seq) * 33, b"warn-batch-1", &wrong_key)
            .await;
    }
    client.assert_ping_pong(0x57A1).await;

    // Snapshot stream still alive and advancing.
    let t1 = recv_snapshot_server_tick(&mut client).await;
    let t2 = recv_snapshot_server_tick(&mut client).await;
    assert!(
        t2 > t1,
        "snapshots must keep arriving under Warn ({t2} after {t1})"
    );

    // Batch 2: more wrong-key frames are still accepted.
    for seq in 6u32..=10 {
        client
            .send_input_frame(seq, u64::from(seq) * 33, b"warn-batch-2", &wrong_key)
            .await;
    }
    client.assert_ping_pong(0x57A2).await;

    // Full-processing proof: 100 further wrong-key frames overrun the token
    // bucket (30 initial tokens, 60-token cap, 8/s refill, 1 token/frame —
    // even from a hypothetically full bucket, 100 frames need >=40 refilled
    // tokens = >=5 s of refill, i.e. >=50 ms per awaited loopback send,
    // orders of magnitude above reality), so the server MUST reply
    // RateLimited, which only the InputFrame processing path can emit.
    for seq in 11u32..=110 {
        client
            .send_input_frame(seq, u64::from(seq) * 33, b"warn-burst", &wrong_key)
            .await;
    }
    client
        .recv_until(|m| match m {
            ServerToClient::RateLimited => Some(()),
            ServerToClient::Snapshot { .. } => None,
            other => panic!("unexpected message while awaiting RateLimited: {other:?}"),
        })
        .await;

    // Still open after the burst and the rate limiting.
    assert_ping_pong_tolerating_rate_limited(&mut client, 0x57A3).await;
    client.close().await;

    // Anti-vacuity twin: the identical first wrong-key frame kicks under
    // Kick with the full Close contract.
    let kick_twin = spawn_test_server(server_key, SignatureFailurePolicy::Kick).await;
    let (mut twin_client, _info) =
        TestClient::connect_and_join(kick_twin.ws_addr(), "f3-warn-twin", "coop").await;
    twin_client
        .send_input_frame(1, 33, b"warn-batch-1", &wrong_key)
        .await;
    expect_kick(&mut twin_client).await;

    warn_server.shutdown();
    kick_twin.shutdown();
}

/// Battery item 6: sanity — a valid-key client on a Warn server works
/// normally (Warn must not break the happy path).
#[tokio::test(flavor = "multi_thread")]
async fn valid_key_under_warn_works_normally() {
    let key = key_a();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Warn).await;

    let (mut client, info) =
        TestClient::connect_and_join(server.ws_addr(), "f3-warn-valid", "coop").await;
    assert!(!info.room_id.is_empty(), "room_id must be non-empty");
    assert!(!info.player_id.is_empty(), "player_id must be non-empty");

    for seq in 1u32..=10 {
        client
            .send_input_frame(seq, u64::from(seq) * 33, &[0x6A, seq as u8], &key)
            .await;
    }
    client.assert_ping_pong(0xF306).await;

    let t1 = recv_snapshot_server_tick(&mut client).await;
    let t2 = recv_snapshot_server_tick(&mut client).await;
    assert!(
        t2 > t1,
        "snapshots must keep flowing for a valid client under Warn ({t2} after {t1})"
    );
    client.assert_ping_pong(0xF366).await;

    client.close().await;
    server.shutdown();
}

/// Battery item 7: kick isolation. Two clients share ONE room on a Kick
/// server; the wrong-key client is kicked with the full Close contract
/// while the valid-key client keeps working uninterrupted (signed frame +
/// in-order Pong + snapshots still advancing after the peer's kick).
#[tokio::test(flavor = "multi_thread")]
async fn mixed_clients_under_kick_wrong_key_kicked_valid_unaffected() {
    const REGION: &str = "f3-mixed";
    let key = key_a();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    let (mut valid, info_valid) =
        TestClient::connect_and_join(server.ws_addr(), REGION, "coop").await;
    valid.send_input_frame(1, 33, b"valid-pre-kick", &key).await;
    valid.assert_ping_pong(0xF371).await;

    // Matchmaking must coalesce same region+mode below capacity — the
    // isolation claim needs the two clients sharing one room's state.
    let (mut wrong, info_wrong) =
        TestClient::connect_and_join(server.ws_addr(), REGION, "coop").await;
    assert_eq!(
        info_valid.room_id, info_wrong.room_id,
        "both clients must share one room for the isolation proof"
    );
    assert_ne!(
        info_valid.player_id, info_wrong.player_id,
        "player ids must be unique"
    );

    wrong
        .send_input_frame(1, 33, b"wrong-in-shared-room", &key_b())
        .await;
    expect_kick(&mut wrong).await;

    // The valid client keeps working uninterrupted in the same room.
    valid
        .send_input_frame(2, 66, b"valid-post-kick", &key)
        .await;
    valid.assert_ping_pong(0xF372).await;
    let t1 = recv_snapshot_server_tick(&mut valid).await;
    let t2 = recv_snapshot_server_tick(&mut valid).await;
    assert!(
        t2 > t1,
        "snapshots must keep flowing to the valid client after the peer kick ({t2} after {t1})"
    );

    valid.close().await;
    server.shutdown();
}

/// Battery item 8: `FromStr` parsing is case-insensitive for the two valid
/// policies and rejects everything else with an error that names the
/// rejected input and states the valid choices.
#[test]
fn signature_failure_policy_from_str_parsing() {
    for s in ["kick", "Kick", "KICK"] {
        let parsed: SignatureFailurePolicy = s
            .parse()
            .unwrap_or_else(|e| panic!("'{s}' must parse: {e}"));
        assert!(
            matches!(parsed, SignatureFailurePolicy::Kick),
            "'{s}' must parse to Kick, got {parsed:?}"
        );
    }
    for s in ["warn", "Warn", "WARN"] {
        let parsed: SignatureFailurePolicy = s
            .parse()
            .unwrap_or_else(|e| panic!("'{s}' must parse: {e}"));
        assert!(
            matches!(parsed, SignatureFailurePolicy::Warn),
            "'{s}' must parse to Warn, got {parsed:?}"
        );
    }
    for s in ["ban", ""] {
        let Err(err) = s.parse::<SignatureFailurePolicy>() else {
            panic!("'{s}' must be rejected by FromStr");
        };
        let msg = err.to_string();
        assert!(
            msg.contains("expected 'kick' or 'warn'"),
            "rejection must state the valid choices, got: {msg}"
        );
        assert!(
            msg.contains(&format!("'{s}'")),
            "rejection must name the rejected input, got: {msg}"
        );
    }
}
