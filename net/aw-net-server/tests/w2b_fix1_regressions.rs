//! W.2.b-fix1 regression tests for the two production defects found by the
//! W.3 Family-1 unit:
//!
//! - **Defect 1** — remotely-triggerable connection-task panic: the
//!   post-matchmaking player-allocation block did
//!   `rooms.get_mut(&rid).expect("room exists")`, so a `JoinRoom` carrying a
//!   client-supplied room id that does not exist (or a FindOrCreate race
//!   where the matched room emptied and was dropped before re-lock)
//!   panicked the connection task. Fixed: graceful `ProtocolError` + clean
//!   connection end, no player state inserted.
//! - **Defect 2** — cleanup-skip on snapshot-arm errors: the snapshot arm of
//!   both connection loops used `?`, early-returning out of the socket
//!   handler BEFORE the shared cleanup block, permanently leaking the player
//!   entry (ghost player; the room never empties). Fixed: `warn!` + `break`
//!   so cleanup always runs.
//!
//! All assertions are bounded by the harness [`common::IO_TIMEOUT`]; servers
//! run on ephemeral ports with unique sled temp dirs (parallel-safe).

mod common;

use std::time::Duration;

use aw_net_proto::{ClientToServer, ServerToClient, SigningKey, PROTOCOL_VERSION};
use aw_net_server::SignatureFailurePolicy;
use common::{spawn_test_server, JoinInfo, TestClient, TestServer, IO_TIMEOUT};

/// A room id that no server in these tests has ever created (room ids are
/// server-generated UUIDs, so this cannot collide).
const BOGUS_ROOM: &str = "w2b-fix1-room-that-never-existed";

/// Defect-1 regression: `JoinRoom` with a nonexistent room id must yield an
/// explicit `ProtocolError` and a clean connection end — and the server must
/// keep serving fresh clients afterwards.
///
/// Pre-fix falsification: the connection task panicked on
/// `.expect("room exists")` and the socket died with NO `ProtocolError`, so
/// the `recv_msg` below would panic on stream end ("ws stream ended while
/// expecting a ServerToClient message").
#[tokio::test(flavor = "multi_thread")]
async fn join_nonexistent_room_yields_protocol_error_and_clean_end() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    // Real handshake, then ask for a room that has never existed.
    let mut client = TestClient::connect(server.ws_addr()).await;
    client
        .send_msg(&ClientToServer::Hello {
            protocol: PROTOCOL_VERSION,
        })
        .await;
    match client.recv_msg().await {
        ServerToClient::HelloAck { protocol } => assert_eq!(protocol, PROTOCOL_VERSION),
        other => panic!("expected HelloAck, got {other:?}"),
    }
    client
        .send_msg(&ClientToServer::JoinRoom {
            room_id: BOGUS_ROOM.to_string(),
            display_name: "w2b-fix1".to_string(),
        })
        .await;

    // Load-bearing assertion: the refusal must be an explicit ProtocolError
    // naming the missing room — not a hang and not an abrupt drop.
    match client.recv_msg().await {
        ServerToClient::ProtocolError { msg } => assert!(
            msg.contains(BOGUS_ROOM),
            "ProtocolError must name the missing room, got: {msg}"
        ),
        other => panic!("expected ProtocolError for nonexistent room, got {other:?}"),
    }

    // The connection must then END within the bounded window (recv_close
    // panics on timeout). The server drops the socket after the
    // ProtocolError; whether the client observes a Close frame or a bare
    // stream end is a transport detail, so both are accepted — the
    // assertion here is bounded termination (no zombie session).
    let _close = client.recv_close().await;

    // The refused join must not poison the server: a FRESH client completes
    // the full handshake, has a signed frame accepted (Kick policy: any
    // verification failure would disconnect), and gets an in-order Pong.
    let (mut fresh, info) =
        TestClient::connect_and_join(server.ws_addr(), "w2b-join-nx", "coop").await;
    assert!(!info.room_id.is_empty(), "room_id must be non-empty");
    assert!(!info.player_id.is_empty(), "player_id must be non-empty");
    fresh.send_input_frame(1, 33, b"post-refusal", &key).await;
    fresh.assert_ping_pong(0xF1A5).await;

    fresh.close().await;
    server.shutdown();
}

/// Bounded poll: keep joining `region` (with clean-close probes) until
/// matchmaking hands out a room id different from `old_room`. Returns the
/// successfully rotated client + join info.
///
/// Rooms are only removed in the connection handlers' shared cleanup block,
/// and only when their player map is empty. Matchmaking reuses any existing
/// room in the region with < 4 players. Therefore a NEW room id proves
/// `old_room` was dropped — i.e. the abruptly-dropped peer's player entry
/// was removed (no ghost) and the empty room was reaped.
///
/// If cleanup never ran (the pre-fix leak), the ghost entry keeps `old_room`
/// alive and matchmaking-visible forever, every probe lands in `old_room`,
/// and this loop panics at the deadline — the falsification path.
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
        // Cleanup of the dropped peer has not run yet — this probe landed in
        // the old room alongside the ghost entry. Leave cleanly and retry.
        probe.close().await;
        assert!(
            tokio::time::Instant::now() < deadline,
            "room {old_room} still matchmaking-visible after {IO_TIMEOUT:?}: \
             ghost player leak (connection cleanup never ran)"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Defect-2 regression (indirect, black-box): after an ABRUPT client drop
/// (socket dropped with no WebSocket close handshake) the server must clean
/// up the player entry and reap the now-empty room within a bounded window,
/// and must keep serving; a SECOND abrupt-drop/reconnect cycle must also
/// work, since a leak would compound.
///
/// What the rotation assertion PROVES: room removal happens only in the
/// shared cleanup block and only when the player map is empty, so observing
/// a fresh room id for the same region proves the dropped client's entry was
/// removed and the room was dropped — no permanent ghost player.
///
/// What it CANNOT prove, precisely:
/// - It cannot isolate WHICH select arm detected the dead socket. The
///   recv-arm (`ws.next()` error) and the snapshot-arm (the `?`-bypass fixed
///   here) both lead to the same cleanup block post-fix, and black-box the
///   recv arm usually observes the FIN first. So this test pins the
///   user-visible invariant (room empties and is dropped after an abrupt
///   drop; no compounding leak across cycles) rather than the exact
///   formerly-`?`'d line.
/// - The rotation proof relies on probes' clean-close cleanup working (so
///   the only possible persistent occupant of the old room is the ghost).
///   In the theoretical worst case where 3 probe entries simultaneously
///   lingered alongside a ghost, the 4-player room cap could force a new
///   room despite a leak; serial probes with 50 ms spacing and the
///   family1-validated clean-disconnect path make that practically
///   unreachable, but it is not logically excluded.
#[tokio::test(flavor = "multi_thread")]
async fn abrupt_drop_reaps_room_and_server_keeps_serving_across_two_cycles() {
    const REGION: &str = "w2b-drop";
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    // Cycle 1: client A joins, proves it is live (signed frame + in-order
    // Pong), then vanishes abruptly — dropping TestClient closes the TCP
    // socket with NO WebSocket close handshake, so the server sees a
    // mid-protocol stream death.
    let (mut a, info_a) = TestClient::connect_and_join(server.ws_addr(), REGION, "coop").await;
    a.send_input_frame(1, 33, b"pre-drop-1", &key).await;
    a.assert_ping_pong(0xA0).await;
    drop(a);

    // Bounded proof that A's room was reaped, and that the server still
    // serves authenticated traffic in the same region.
    let (mut b, info_b) = await_room_rotation(&server, REGION, &info_a.room_id).await;
    b.send_input_frame(1, 33, b"cycle-2", &key).await;
    b.assert_ping_pong(0xB0).await;

    // Cycle 2: a second abrupt drop in the SAME region must also be cleaned
    // up — a leak would compound (the first leaked room would already have
    // pinned matchmaking, and a second ghost would pin the next room too).
    drop(b);
    let (mut c, info_c) = await_room_rotation(&server, REGION, &info_b.room_id).await;
    assert_ne!(
        info_c.room_id, info_a.room_id,
        "rotated room must not be the first dropped room (UUIDs are never reused)"
    );
    c.send_input_frame(1, 33, b"cycle-3", &key).await;
    c.assert_ping_pong(0xC0).await;

    c.close().await;
    server.shutdown();
}
