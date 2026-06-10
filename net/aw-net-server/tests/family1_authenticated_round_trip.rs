//! W.3 Family-1: authenticated round-trip tests.
//!
//! Every server here runs `SignatureFailurePolicy::Kick` (the default).
//! Under Kick, ANY InputFrame signature verification failure disconnects the
//! client (WebSocket Close 1008), so a connection that survives N signed
//! frames and then completes an in-order Ping→Pong round trip has PROVEN
//! zero verification failures across those N frames. That survival proof is
//! the load-bearing assertion in each test.

mod common;

use aw_net_proto::{ClientToServer, ServerToClient, SigningKey};
use aw_net_server::SignatureFailurePolicy;
use common::{spawn_test_server, TestClient};

/// A 64-hex-char (32-byte) custom key — deliberately NOT the dev default.
const CUSTOM_KEY_HEX: &str = "8f1d3b5a7c9e0f2468ace13579bdf02468ace13579bdf02468ace13579bdf024";

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

/// The frame battery: varying seq (including 0, non-monotonic, u32::MAX),
/// varying tick_ms (including 0 and u64::MAX), and blobs from empty through
/// 1 KiB with all byte values represented. 20 frames total — under the
/// 30-token rate-limit bucket, so RateLimited cannot fire.
fn frame_battery() -> Vec<(u32, u64, Vec<u8>)> {
    let mut frames: Vec<(u32, u64, Vec<u8>)> = vec![
        (1, 0, Vec::new()),                     // empty blob, zero tick_ms
        (2, 33, b"forward".to_vec()),           // small ascii blob
        (3, 66, vec![0u8; 256]),                // zero-filled blob
        (4, 99, (0u8..=255).collect()),         // every byte value once
        (5, u64::MAX, vec![0xAA; 1024]),        // extreme tick_ms, 1 KiB blob
        (0, 5, vec![1, 2, 3]),                  // seq 0, out-of-order
        (u32::MAX, 12_345, vec![0xFF; 64]),     // extreme seq
        (7, 231, vec![0x00, 0xFF, 0x00, 0xFF]), // alternating bytes
    ];
    for i in 8u32..20 {
        let blob: Vec<u8> = (0..i * 7).map(|b| (b % 251) as u8).collect();
        frames.push((i, u64::from(i) * 33, blob));
    }
    assert_eq!(frames.len(), 20);
    frames
}

#[tokio::test(flavor = "multi_thread")]
async fn dev_key_round_trip_survives_20_signed_frames() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    let (mut client, info) = TestClient::connect_and_join(server.ws_addr(), "f1-dev", "coop").await;
    assert!(!info.room_id.is_empty(), "room_id must be non-empty");
    assert!(!info.player_id.is_empty(), "player_id must be non-empty");
    assert_eq!(info.tick_hz, 30, "server rooms tick at 30 Hz");

    let frames = frame_battery();
    let mid = frames.len() / 2;
    let mut tick_mid = 0u64;
    for (i, (seq, tick_ms, blob)) in frames.iter().enumerate() {
        client.send_input_frame(*seq, *tick_ms, blob, &key).await;
        if i == mid {
            // Snapshots must keep flowing mid-battery, and the payload must
            // honor the real contract (lz4 + postcard, tick >= 1).
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
            assert!(tick_mid >= 1, "snapshot tick must have advanced");
        }
    }

    // In-order Pong after all 20 frames == zero verification failures.
    client.assert_ping_pong(0xF1_DE_77).await;

    // Snapshots still flowing after the battery, with a strictly monotonic
    // room tick (single-client room: every snapshot increments the tick).
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
        "snapshot tick must keep advancing ({tick_after} after {tick_mid})"
    );

    client.close().await;
    server.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn custom_key_round_trip_survives_20_signed_frames() {
    // Both ends use the SAME custom key. If the server ignored its configured
    // key (e.g. always verified with the dev default), every frame would fail
    // verification and the Kick policy would close the connection — so the
    // survival proof below also falsifies key plumbing on both sides.
    let key = SigningKey::from_hex(CUSTOM_KEY_HEX).expect("valid 64-hex custom key");
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    let (mut client, _info) =
        TestClient::connect_and_join(server.ws_addr(), "f1-custom", "coop").await;

    for (seq, tick_ms, blob) in frame_battery() {
        client.send_input_frame(seq, tick_ms, &blob, &key).await;
    }
    client.assert_ping_pong(0xC0_57_0E).await;

    client.close().await;
    server.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn multiple_concurrent_clients_all_stay_authenticated() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    // Sequential joins with identical region/game_mode: matchmaking must
    // coalesce all three into ONE room (capacity 4).
    let (mut c1, i1) = TestClient::connect_and_join(server.ws_addr(), "f1-multi", "coop").await;
    let (mut c2, i2) = TestClient::connect_and_join(server.ws_addr(), "f1-multi", "coop").await;
    let (mut c3, i3) = TestClient::connect_and_join(server.ws_addr(), "f1-multi", "coop").await;

    assert_eq!(i1.room_id, i2.room_id, "matchmaking must reuse the room");
    assert_eq!(i1.room_id, i3.room_id, "matchmaking must reuse the room");
    assert_ne!(i1.player_id, i2.player_id, "player ids must be unique");
    assert_ne!(i1.player_id, i3.player_id, "player ids must be unique");
    assert_ne!(i2.player_id, i3.player_id, "player ids must be unique");

    // Interleave validly signed frames across all three connections.
    for round in 1u32..=10 {
        let tick_ms = u64::from(round) * 33;
        c1.send_input_frame(round, tick_ms, &[1, round as u8], &key)
            .await;
        c2.send_input_frame(round, tick_ms, &[2, round as u8], &key)
            .await;
        c3.send_input_frame(round, tick_ms, &[3, round as u8], &key)
            .await;
    }

    // Every connection must still be served (in-order Pong == no kicks).
    c1.assert_ping_pong(111).await;
    c2.assert_ping_pong(222).await;
    c3.assert_ping_pong(333).await;

    c1.close().await;
    c2.close().await;
    c3.close().await;
    server.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn clean_disconnect_leaves_server_healthy_for_next_client() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    // Client A: full session, then a proper WebSocket close handshake.
    let (mut a, _) = TestClient::connect_and_join(server.ws_addr(), "f1-disc-a", "coop").await;
    for seq in 1u32..=3 {
        a.send_input_frame(seq, u64::from(seq) * 33, &[seq as u8], &key)
            .await;
    }
    a.assert_ping_pong(0xA1).await;
    a.close().await;

    // Client B: a FRESH connection against the same server must complete the
    // full handshake and an authenticated round trip after A's departure.
    // (Different region on purpose: B must not race A's emptying room.)
    let (mut b, info_b) = TestClient::connect_and_join(server.ws_addr(), "f1-disc-b", "coop").await;
    assert!(!info_b.player_id.is_empty());
    for seq in 1u32..=5 {
        b.send_input_frame(seq, u64::from(seq) * 33, &[0xB0, seq as u8], &key)
            .await;
    }
    b.assert_ping_pong(0xB1).await;

    b.close().await;
    server.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
async fn ping_pong_and_ack_paths_with_valid_signatures() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;

    let (mut client, _) = TestClient::connect_and_join(server.ws_addr(), "f1-ack", "coop").await;

    // Signed frame, then Ack a REAL snapshot id taken off the live stream.
    client.send_input_frame(1, 33, b"pre-ack", &key).await;
    let snap_id = client
        .recv_until(|m| match m {
            ServerToClient::Snapshot { id, .. } => Some(id),
            _ => None,
        })
        .await;
    client
        .send_msg(&ClientToServer::Ack {
            last_snapshot_id: snap_id,
        })
        .await;

    // Ping → exact-nano Pong (in-order: proves frame + Ack were accepted).
    client.assert_ping_pong(987_654_321).await;

    // Second cycle: another signed frame, an Ack for a later snapshot, and a
    // distinct Pong — the connection keeps serving after the Ack path.
    client.send_input_frame(2, 66, &[0xEE; 16], &key).await;
    let snap_id2 = client
        .recv_until(|m| match m {
            ServerToClient::Snapshot { id, .. } => Some(id),
            _ => None,
        })
        .await;
    assert!(
        snap_id2 > snap_id,
        "snapshot ids must advance ({snap_id2} after {snap_id})"
    );
    client
        .send_msg(&ClientToServer::Ack {
            last_snapshot_id: snap_id2,
        })
        .await;
    client.assert_ping_pong(123_456_789).await;

    client.close().await;
    server.shutdown();
}
