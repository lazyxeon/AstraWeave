//! A flood must not earn a reply per frame, and it must end the connection.
//! A client that sends one signed input every 33 ms must stay connected.
//!
//! The unfixed bucket charges only `InputFrame`, sends `RateLimited` for
//! every over-limit frame, and answers every `Ping` with a `Pong`.

mod common;

use std::time::Duration;

use aw_net_proto::{ClientToServer, ServerToClient, SigningKey};
use aw_net_server::SignatureFailurePolicy;
use common::{spawn_test_server, TestClient, TEST_CODEC};
use tokio_tungstenite::tungstenite::Message;

const FLOOD: u32 = 200;
/// The bucket starts full (64). A tight flood can earn about that many
/// replies plus a little refill. It cannot earn one reply per frame.
const REPLY_CAP: u32 = 96;
const CLOSE_DEADLINE: Duration = Duration::from_secs(5);

async fn flood_until_close(client: &mut TestClient) -> (u16, String, u32, u32) {
    let deadline = tokio::time::Instant::now() + CLOSE_DEADLINE;
    let mut replies = 0u32;
    let mut rate_limited = 0u32;
    loop {
        if tokio::time::Instant::now() >= deadline {
            panic!("flood produced no Close within {CLOSE_DEADLINE:?}");
        }
        let left = deadline.saturating_duration_since(tokio::time::Instant::now());
        let next =
            tokio::time::timeout(left.min(Duration::from_millis(200)), client.recv_raw()).await;
        match next {
            Err(_) => continue,
            Ok(Some(Message::Binary(bytes))) => {
                if let Ok(msg) = aw_net_proto::decode_msg::<ServerToClient>(TEST_CODEC, &bytes) {
                    match msg {
                        ServerToClient::Pong { .. } | ServerToClient::RateLimited => {
                            if matches!(msg, ServerToClient::RateLimited) {
                                rate_limited += 1;
                            }
                            replies += 1;
                            assert!(
                                replies <= REPLY_CAP,
                                "flood produced {replies} replies; the bucket starts at 64"
                            );
                        }
                        ServerToClient::Snapshot { .. } | ServerToClient::ProtocolError { .. } => {}
                        other => panic!("unexpected server frame during flood: {other:?}"),
                    }
                }
            }
            Ok(Some(Message::Close(Some(frame)))) => {
                return (
                    u16::from(frame.code),
                    frame.reason.to_string(),
                    replies,
                    rate_limited,
                );
            }
            Ok(Some(Message::Close(None))) => panic!("close frame had no code"),
            Ok(Some(Message::Ping(_))) | Ok(Some(Message::Pong(_))) => {}
            Ok(Some(other)) => panic!("unexpected websocket frame: {other:?}"),
            Ok(None) => panic!("stream ended without a Close frame"),
        }
    }
}

fn assert_rate_close(code: u16, reason: &str, replies: u32, rate_limited: u32) {
    assert_eq!(
        code, 1008,
        "rate-limit kick must use Close 1008, reason {reason}"
    );
    assert!(
        reason.contains("rate limited"),
        "close reason must say rate limited, got {reason}"
    );
    assert_eq!(rate_limited, 0, "over-limit frames must not be answered");
    assert!(replies <= REPLY_CAP);
}

#[tokio::test]
async fn input_flood_closes_with_1008_and_does_not_reply_per_frame() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) =
        TestClient::connect_and_join(server.ws_addr(), "us-east", "rate-input").await;
    for seq in 1..=FLOOD {
        if client
            .try_send_input_frame(seq, u64::from(seq), b"flood", &key)
            .await
            .is_err()
        {
            break;
        }
    }
    let (code, reason, replies, rate_limited) = flood_until_close(&mut client).await;
    assert_rate_close(code, &reason, replies, rate_limited);
    server.shutdown();
}

#[tokio::test]
async fn ping_flood_is_capped_and_closes() {
    let server = spawn_test_server(SigningKey::dev_default(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) =
        TestClient::connect_and_join(server.ws_addr(), "us-east", "rate-ping").await;
    for nano in 0..u128::from(FLOOD) {
        if client
            .try_send_msg(&ClientToServer::Ping { nano })
            .await
            .is_err()
        {
            break;
        }
    }
    let (code, reason, replies, rate_limited) = flood_until_close(&mut client).await;
    assert_rate_close(code, &reason, replies, rate_limited);
    server.shutdown();
}

#[tokio::test]
async fn ack_flood_is_capped_and_closes() {
    let server = spawn_test_server(SigningKey::dev_default(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) =
        TestClient::connect_and_join(server.ws_addr(), "us-east", "rate-ack").await;
    for id in 0..FLOOD {
        if client
            .try_send_msg(&ClientToServer::Ack {
                last_snapshot_id: id,
            })
            .await
            .is_err()
        {
            break;
        }
    }
    let (code, reason, replies, rate_limited) = flood_until_close(&mut client).await;
    assert_rate_close(code, &reason, replies, rate_limited);
    server.shutdown();
}

#[tokio::test]
async fn ws_ping_flood_is_capped_and_closes() {
    let server = spawn_test_server(SigningKey::dev_default(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) =
        TestClient::connect_and_join(server.ws_addr(), "us-east", "rate-ws-ping").await;
    for _ in 0..FLOOD {
        if client
            .try_send_ws(Message::Ping(b"x".to_vec().into()))
            .await
            .is_err()
        {
            break;
        }
    }
    let (code, reason, replies, rate_limited) = flood_until_close(&mut client).await;
    assert_rate_close(code, &reason, replies, rate_limited);
    server.shutdown();
}

#[tokio::test]
async fn undecodable_flood_is_capped_and_closes() {
    let server = spawn_test_server(SigningKey::dev_default(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) =
        TestClient::connect_and_join(server.ws_addr(), "us-east", "rate-garbage").await;
    let garbage = vec![0u8];
    for _ in 0..FLOOD {
        if client.try_send_raw(garbage.clone()).await.is_err() {
            break;
        }
    }
    let (code, reason, replies, rate_limited) = flood_until_close(&mut client).await;
    assert_rate_close(code, &reason, replies, rate_limited);
    server.shutdown();
}

/// The in-repo client sends an input every 33 ms. That pace must stay open
/// for at least five seconds. A paced overrun is `steady_over_limit_closes_with_1008`.
#[tokio::test]
async fn thirty_hertz_input_stays_open_for_five_seconds() {
    let key = SigningKey::dev_default();
    let server = spawn_test_server(key.clone(), SignatureFailurePolicy::Kick).await;
    let (mut client, info) =
        TestClient::connect_and_join(server.ws_addr(), "us-east", "rate-30hz").await;
    let window = Duration::from_secs(5);
    let start = tokio::time::Instant::now();
    let mut seq = 1u32;
    while start.elapsed() < window {
        client
            .try_send_input_frame(seq, u64::from(seq) * 33, b"tick", &key)
            .await
            .unwrap_or_else(|e| panic!("33 ms client was dropped after {seq} frames: {e}"));
        seq += 1;
        loop {
            match tokio::time::timeout(Duration::from_millis(1), client.recv_raw()).await {
                Ok(Some(Message::Close(frame))) => {
                    panic!("33 ms client closed during the soak: {frame:?}");
                }
                Ok(Some(_)) => {}
                Ok(None) => panic!("33 ms client stream ended during the soak"),
                Err(_) => break,
            }
        }
        tokio::time::sleep(Duration::from_millis(33)).await;
    }
    let sent = seq - 1;
    tokio::time::sleep(Duration::from_millis(50)).await;
    let (accepted, dropped) = server
        .server
        .input_tolls(&info.room_id, &info.player_id)
        .expect("player still in the room");
    assert_eq!(
        dropped, 0,
        "30 Hz client dropped {dropped} of {sent} frames"
    );
    assert_eq!(
        accepted, sent,
        "30 Hz client paid {accepted} of {sent} frames"
    );
    client.assert_ping_pong(0x3333).await;
    server.shutdown();
}

/// One protocol ping per 33 ms tick is under the 64/s refill, so the server
/// must not kick it, and the snapshot arm must still run. A tight flood is
/// `ping_flood_is_capped_and_closes`.
#[tokio::test]
async fn ping_at_tick_rate_still_receives_snapshots() {
    let server = spawn_test_server(SigningKey::dev_default(), SignatureFailurePolicy::Kick).await;
    let (mut client, info) =
        TestClient::connect_and_join(server.ws_addr(), "us-east", "rate-ping-tick").await;
    let window = Duration::from_millis(500);
    let period = Duration::from_millis((1000 / info.tick_hz.max(1)) as u64);
    let start = tokio::time::Instant::now();
    let mut snapshots = 0u32;
    let mut nano = 0u128;
    while start.elapsed() < window {
        client
            .try_send_msg(&ClientToServer::Ping { nano })
            .await
            .unwrap_or_else(|e| panic!("tick-rate ping was dropped: {e}"));
        nano = nano.wrapping_add(1);
        let slice = tokio::time::Instant::now();
        loop {
            match tokio::time::timeout(Duration::from_millis(1), client.recv_raw()).await {
                Ok(Some(Message::Binary(bytes))) => {
                    if let Ok(ServerToClient::Snapshot { .. }) =
                        aw_net_proto::decode_msg::<ServerToClient>(TEST_CODEC, &bytes)
                    {
                        snapshots += 1;
                    }
                }
                Ok(Some(Message::Close(frame))) => {
                    panic!("tick-rate ping closed the socket: {frame:?}");
                }
                Ok(Some(_)) => {}
                Ok(None) => panic!("tick-rate ping ended the stream"),
                Err(_) => break,
            }
        }
        let spent = slice.elapsed();
        if let Some(rest) = period.checked_sub(spent) {
            tokio::time::sleep(rest).await;
        }
    }
    let expected = window.as_secs_f64() * f64::from(info.tick_hz.max(1));
    assert!(
        f64::from(snapshots) * 2.0 >= expected,
        "tick-rate pings delivered {snapshots} snapshots in {window:?}; about {expected:.0} are due at {} Hz",
        info.tick_hz
    );
    server.shutdown();
}

/// Two protocol pings, then 5 ms. That is well above the 64/s refill and
/// well under a back-to-back flood. Thirty-two unpaid frames inside one
/// second still close the socket, and a paid frame does not clear that window.
#[tokio::test]
async fn steady_over_limit_closes_with_1008() {
    let server = spawn_test_server(SigningKey::dev_default(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) =
        TestClient::connect_and_join(server.ws_addr(), "us-east", "rate-steady").await;
    let window = Duration::from_secs(2);
    let start = tokio::time::Instant::now();
    let mut nano = 0u128;
    while start.elapsed() < window {
        let mut stopped = false;
        for _ in 0..2 {
            if client
                .try_send_msg(&ClientToServer::Ping { nano })
                .await
                .is_err()
            {
                stopped = true;
                break;
            }
            nano = nano.wrapping_add(1);
        }
        if stopped {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let (code, reason, replies, rate_limited) = flood_until_close(&mut client).await;
    assert_rate_close(code, &reason, replies, rate_limited);
    server.shutdown();
}

/// Drain the bucket with fewer than 32 unpaid messages, wait out the window,
/// and require the next message to be paid. `RateLimited` is not a reply.
#[tokio::test]
async fn floor_recovers_after_one_second_and_the_next_ping_is_paid() {
    let server = spawn_test_server(SigningKey::dev_default(), SignatureFailurePolicy::Kick).await;
    let (mut client, info) =
        TestClient::connect_and_join(server.ws_addr(), "us-east", "rate-floor").await;
    let mut nano = 0u128;
    for _ in 0..90u32 {
        client
            .try_send_msg(&ClientToServer::Ping { nano })
            .await
            .expect("drain send");
        nano = nano.wrapping_add(1);
    }
    let mut dropped = 0u32;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
    while tokio::time::Instant::now() < deadline {
        if let Some((_accepted, drops)) = server.server.input_tolls(&info.room_id, &info.player_id)
        {
            dropped = drops;
            if drops > 0 {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        dropped > 0 && dropped < 32,
        "drain should leave an unpaid remainder under the kick, dropped {dropped}"
    );
    tokio::time::sleep(Duration::from_secs(1)).await;
    client
        .try_send_msg(&ClientToServer::Ping { nano })
        .await
        .expect("post-idle ping");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        if tokio::time::Instant::now() >= deadline {
            panic!("no Pong after the bucket refilled");
        }
        match tokio::time::timeout(Duration::from_millis(200), client.recv_raw()).await {
            Ok(Some(Message::Binary(bytes))) => {
                match aw_net_proto::decode_msg::<ServerToClient>(TEST_CODEC, &bytes) {
                    Ok(ServerToClient::Pong { .. }) => break,
                    Ok(ServerToClient::RateLimited) => {
                        panic!("a refilled bucket must not send RateLimited")
                    }
                    Ok(ServerToClient::Snapshot { .. }) => {}
                    Ok(other) => panic!("unexpected frame after refill: {other:?}"),
                    Err(_) => {}
                }
            }
            Ok(Some(Message::Close(frame))) => {
                panic!("refilled bucket closed the socket: {frame:?}")
            }
            Ok(Some(_)) => {}
            Ok(None) => panic!("stream ended while waiting for the paid Pong"),
            Err(_) => {}
        }
    }
    server.shutdown();
}

/// One WebSocket text frame per tick is inside the 64/s refill. Text is
/// charged and is not a protocol message. The sender keeps going for the
/// whole window, the socket stays open, and snapshots still arrive. A tight
/// text send is `text_flood_is_capped_and_closes`.
#[tokio::test]
async fn text_within_limit_still_receives_snapshots() {
    let server = spawn_test_server(SigningKey::dev_default(), SignatureFailurePolicy::Kick).await;
    let (mut client, info) =
        TestClient::connect_and_join(server.ws_addr(), "us-east", "rate-text-tick").await;
    let window = Duration::from_millis(500);
    let period = Duration::from_millis((1000 / info.tick_hz.max(1)) as u64);
    let start = tokio::time::Instant::now();
    let mut snapshots = 0u32;
    while start.elapsed() < window {
        client
            .try_send_ws(Message::Text("ready".into()))
            .await
            .unwrap_or_else(|e| panic!("tick-rate text was dropped: {e}"));
        let slice = tokio::time::Instant::now();
        loop {
            match tokio::time::timeout(Duration::from_millis(1), client.recv_raw()).await {
                Ok(Some(Message::Binary(bytes))) => {
                    if let Ok(ServerToClient::Snapshot { .. }) =
                        aw_net_proto::decode_msg::<ServerToClient>(TEST_CODEC, &bytes)
                    {
                        snapshots += 1;
                    }
                }
                Ok(Some(Message::Close(frame))) => {
                    panic!("tick-rate text closed the socket: {frame:?}");
                }
                Ok(Some(_)) => {}
                Ok(None) => panic!("tick-rate text ended the stream"),
                Err(_) => break,
            }
        }
        let spent = slice.elapsed();
        if let Some(rest) = period.checked_sub(spent) {
            tokio::time::sleep(rest).await;
        }
    }
    let expected = window.as_secs_f64() * f64::from(info.tick_hz.max(1));
    assert!(
        f64::from(snapshots) * 2.0 >= expected,
        "tick-rate text delivered {snapshots} snapshots in {window:?}; about {expected:.0} are due at {} Hz",
        info.tick_hz
    );
    server.shutdown();
}

#[tokio::test]
async fn text_flood_is_capped_and_closes() {
    let server = spawn_test_server(SigningKey::dev_default(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) =
        TestClient::connect_and_join(server.ws_addr(), "us-east", "rate-text").await;
    for _ in 0..FLOOD {
        if client
            .try_send_ws(Message::Text("flood".into()))
            .await
            .is_err()
        {
            break;
        }
    }
    let (code, reason, replies, rate_limited) = flood_until_close(&mut client).await;
    assert_rate_close(code, &reason, replies, rate_limited);
    server.shutdown();
}

#[tokio::test]
async fn pong_flood_is_capped_and_closes() {
    let server = spawn_test_server(SigningKey::dev_default(), SignatureFailurePolicy::Kick).await;
    let (mut client, _) =
        TestClient::connect_and_join(server.ws_addr(), "us-east", "rate-pong").await;
    for _ in 0..FLOOD {
        if client
            .try_send_ws(Message::Pong(b"x".to_vec().into()))
            .await
            .is_err()
        {
            break;
        }
    }
    let (code, reason, replies, rate_limited) = flood_until_close(&mut client).await;
    assert_rate_close(code, &reason, replies, rate_limited);
    server.shutdown();
}
