//! Room tick must not depend on how many sockets are in the room.
//!
//! One `spawn_room_tick` publishes snapshots. Each connection selects
//! `snaps.changed()` first under `biased`, then `ws.next()`. A socket that
//! already has a frame queued cannot skip the tick.
//!
//! Protocol Ping is charged. One ping every 20 ms is 50/s, under the 64/s
//! refill, and shorter than `tick_dt` (~33 ms). The sender and the reader are
//! separate tasks. One ping per tick on the same task also receives snapshots.
//! `prequeued_frames_advance_exactly_one_tick_per_period` keeps the socket
//! readable for the whole window and requires `server_tick` to step by 1.
//! The frames are signed input: small on the wire, slow to verify, so the
//! handler does not drain the buffer and fall through to the snapshot arm.

mod common;

use std::time::Duration;

use aw_net_proto::{
    encode_msg, input_frame_sig_payload, sign, ClientToServer, ServerToClient, SigningKey,
};
use aw_net_server::SignatureFailurePolicy;
use common::{spawn_test_server, spawn_test_server_without_rate_limit, TestClient, TEST_CODEC};
use futures::{SinkExt, StreamExt};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

async fn snapshot_ticks(mut client: TestClient, window: Duration) -> Vec<u64> {
    let mut ticks = Vec::new();
    let start = tokio::time::Instant::now();
    while start.elapsed() < window {
        match timeout(Duration::from_millis(100), client.recv_msg()).await {
            Ok(ServerToClient::Snapshot { server_tick, .. }) => ticks.push(server_tick),
            Ok(_) => {}
            Err(_) => break,
        }
    }
    ticks
}

#[tokio::test]
async fn four_clients_advance_one_tick_per_period() {
    let server = spawn_test_server(SigningKey::dev_default(), SignatureFailurePolicy::Kick).await;
    let mut clients = Vec::new();
    let mut room_id = None;
    let mut tick_hz = 0u32;
    for _ in 0..4 {
        let (client, info) =
            TestClient::connect_and_join(server.ws_addr(), "us-east", "tick-owner").await;
        match &room_id {
            None => room_id = Some(info.room_id),
            Some(id) => assert_eq!(id, &info.room_id, "clients must share one room"),
        }
        tick_hz = info.tick_hz;
        clients.push(client);
    }

    let window = Duration::from_millis(500);
    let mut tasks = Vec::new();
    for client in clients {
        tasks.push(tokio::spawn(snapshot_ticks(client, window)));
    }
    let mut series = Vec::new();
    for task in tasks {
        series.push(task.await.expect("collector"));
    }

    let expected = window.as_secs_f64() * f64::from(tick_hz.max(1));
    for ticks in &series {
        assert!(
            ticks.len() >= 4,
            "need several snapshots to compare the tick, got {ticks:?}"
        );
        let delta = ticks.last().expect("last") - ticks.first().expect("first");
        assert!(
            (delta as f64) < expected * 2.0,
            "server_tick advanced {delta} in {window:?} with 4 clients; one room period is about {expected:.0}"
        );
    }
    server.shutdown();
}

/// One protocol Ping every 20 ms, on its own task. That is 50 messages/s,
/// under the 64/s refill, and shorter than `tick_dt` (~33 ms). Ping is
/// charged. The reader does not share that task, so the sender does not wait
/// for a receive before the next ping. The room owner must still deliver
/// snapshots. One ping per tick is
/// `ping_at_or_under_limit_still_receives_snapshots`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn paid_frame_inside_limit_still_receives_snapshots() {
    let server = spawn_test_server(SigningKey::dev_default(), SignatureFailurePolicy::Kick).await;
    let (client, info) =
        TestClient::connect_and_join(server.ws_addr(), "us-east", "tick-paid").await;
    let (mut sink, mut stream) = client.into_split();
    let window = Duration::from_millis(500);
    // Shorter than tick_dt. 50/s stays under the 64/s refill, so the charged
    // ping is paid and the socket is not kicked.
    let period = Duration::from_millis(20);
    let reader = tokio::spawn(async move {
        let mut snapshots = 0u32;
        let deadline = tokio::time::Instant::now() + window;
        while tokio::time::Instant::now() < deadline {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left.is_zero() {
                break;
            }
            match timeout(left, stream.next()).await {
                Ok(Some(Ok(Message::Binary(bytes)))) => {
                    if let Ok(ServerToClient::Snapshot { .. }) =
                        aw_net_proto::decode_msg::<ServerToClient>(TEST_CODEC, &bytes)
                    {
                        snapshots += 1;
                    }
                }
                Ok(Some(Ok(Message::Close(frame)))) => {
                    panic!("tick-rate ping closed the socket: {frame:?}");
                }
                Ok(Some(Ok(_))) => {}
                Ok(Some(Err(e))) => panic!("tick-rate ping transport error: {e}"),
                Ok(None) => panic!("tick-rate ping ended the stream"),
                Err(_) => break,
            }
        }
        snapshots
    });
    let start = tokio::time::Instant::now();
    let mut nano = 0u128;
    while start.elapsed() < window {
        let bytes = encode_msg(TEST_CODEC, &ClientToServer::Ping { nano });
        sink.send(Message::Binary(bytes.into()))
            .await
            .unwrap_or_else(|e| panic!("tick-rate ping send failed: {e}"));
        nano = nano.wrapping_add(1);
        tokio::time::sleep(period).await;
    }
    let snapshots = reader.await.expect("reader");
    let expected = window.as_secs_f64() * f64::from(info.tick_hz.max(1));
    assert!(
        f64::from(snapshots) * 2.0 >= expected,
        "a ping every 20 ms delivered {snapshots} snapshots in {window:?}; about {expected:.0} are due at {} Hz",
        info.tick_hz
    );
    server.shutdown();
}

/// One protocol Ping per tick. The socket is idle for the rest of the
/// period, and snapshots still arrive.
#[tokio::test]
async fn ping_at_or_under_limit_still_receives_snapshots() {
    let server = spawn_test_server(SigningKey::dev_default(), SignatureFailurePolicy::Kick).await;
    let (mut client, info) =
        TestClient::connect_and_join(server.ws_addr(), "us-east", "tick-paced").await;
    let window = Duration::from_millis(500);
    let period = Duration::from_millis((1000 / info.tick_hz.max(1)) as u64);
    let start = tokio::time::Instant::now();
    let mut snapshots = 0u32;
    let mut nano = 0u128;
    while start.elapsed() < window {
        client.send_msg(&ClientToServer::Ping { nano }).await;
        nano = nano.wrapping_add(1);
        let slice = tokio::time::Instant::now();
        loop {
            match timeout(Duration::from_millis(1), client.recv_msg()).await {
                Ok(ServerToClient::Snapshot { .. }) => snapshots += 1,
                Ok(_) => {}
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

/// Pre-queue signed input, then keep sending it for the whole window.
///
/// Zeros compress, so several frames fit in the capped send window, and
/// verify still walks the full blob. The handler stays behind the sender, so
/// `ws.next()` stays ready. Rate limiting is off: a charged flood would
/// close the socket.
/// `server_tick` must step by exactly 1.
///
/// With `biased`, putting `ws.next()` before `snaps.changed()` never polls
/// the watch while the socket is readable, so this receives no snapshots.
/// `room.tick += 1` changed to `+= 2` or `*= 1` delivers snapshots whose
/// ticks are not +1. Removing `biased` still polls the watch on the runs
/// that start there, so a skip is not guaranteed in one window.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn prequeued_frames_advance_exactly_one_tick_per_period() {
    let server = spawn_test_server_without_rate_limit(
        SigningKey::dev_default(),
        SignatureFailurePolicy::Kick,
    )
    .await;
    let (client, info) =
        TestClient::connect_and_join(server.ws_addr(), "us-east", "tick-prequeue").await;
    client.set_send_buffer_bytes(16 * 1024);
    let (mut sink, mut stream) = client.into_split();
    let window = Duration::from_millis(500);

    // Zeros compress, so several frames fit in the capped send window, and
    // verify still walks the whole blob. Under the 4 MiB lz4 cap.
    let blob = vec![0u8; 256 * 1024];
    let key = SigningKey::dev_default();
    let sig = sign(&key, &input_frame_sig_payload(1, 1, &blob));
    let frame = Message::Binary(
        encode_msg(
            TEST_CODEC,
            &ClientToServer::InputFrame {
                seq: 1,
                tick_ms: 1,
                input_blob: blob,
                sig,
            },
        )
        .into(),
    );

    for _ in 0..8 {
        sink.send(frame.clone()).await.expect("pre-queue");
    }

    let reader = tokio::spawn(async move {
        let mut ticks = Vec::new();
        let deadline = tokio::time::Instant::now() + window;
        while tokio::time::Instant::now() < deadline {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left.is_zero() {
                break;
            }
            match timeout(left, stream.next()).await {
                Ok(Some(Ok(Message::Binary(bytes)))) => {
                    if let Ok(ServerToClient::Snapshot { server_tick, .. }) =
                        aw_net_proto::decode_msg::<ServerToClient>(TEST_CODEC, &bytes)
                    {
                        ticks.push(server_tick);
                    }
                }
                Ok(Some(Ok(Message::Close(frame)))) => {
                    panic!("pre-queued frames closed the socket: {frame:?}");
                }
                Ok(Some(Ok(_))) => {}
                Ok(Some(Err(e))) => panic!("pre-queue transport error: {e}"),
                Ok(None) => panic!("pre-queue ended the stream"),
                Err(_) => break,
            }
        }
        ticks
    });

    let start = tokio::time::Instant::now();
    while start.elapsed() < window {
        if sink.send(frame.clone()).await.is_err() {
            break;
        }
    }

    let ticks = reader.await.expect("reader");
    drop(sink);
    let expected = window.as_secs_f64() * f64::from(info.tick_hz.max(1));
    assert!(
        ticks.len() as f64 >= expected * 0.5,
        "pre-queued frames delivered {} snapshots in {window:?}; about {expected:.0} are due at {} Hz",
        ticks.len(),
        info.tick_hz
    );
    for pair in ticks.windows(2) {
        assert_eq!(
            pair[1],
            pair[0] + 1,
            "room tick must advance by exactly 1 per snapshot, got {ticks:?}"
        );
    }
    let span = ticks.last().copied().unwrap_or(0) - ticks.first().copied().unwrap_or(0);
    assert_eq!(
        span,
        (ticks.len() - 1) as u64,
        "tick span {span} over {} snapshots is not one step each: {ticks:?}",
        ticks.len()
    );
    server.shutdown();
}
