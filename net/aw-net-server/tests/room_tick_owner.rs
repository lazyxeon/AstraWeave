//! Room tick must not depend on how many sockets are in the room.
//!
//! A paid protocol Ping every 20 ms is inside a 64/s refill and faster than
//! `tick_dt` (~33 ms). The sender and the reader are separate tasks, so the
//! next ping arrives before a fresh `sleep(tick_dt)` can finish. Snapshots
//! must still arrive. One ping per tick on the same task also receives
//! snapshots, because that sender waits out the period and the sleep can fire.
//!
//! The unfixed loop is `biased` with `ws.next()` first and a fresh
//! `sleep(tick_dt)` on the other arm. A frame that arrives before that sleep
//! elapses wins the poll, so the sleep arm never fires. `build_snapshot` also
//! adds one to `room.tick` per connection, so four sockets advance the room
//! about four times per period.

mod common;

use std::time::Duration;

use aw_net_proto::{encode_msg, ClientToServer, ServerToClient, SigningKey};
use aw_net_server::SignatureFailurePolicy;
use common::{spawn_test_server, TestClient, TEST_CODEC};
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

/// One protocol Ping every 20 ms, on its own task. That is 50 messages/s:
/// inside the 64/s refill, and shorter than `tick_dt` (~33 ms). The reader
/// does not share that task, so the sender does not wait for a receive
/// before the next ping. A fresh `sleep(tick_dt)` in the connection loop is
/// cancelled before it fires. The room owner must still deliver snapshots.
/// One ping per tick does not: the socket is idle for the whole period, and
/// the fresh sleep fires. That case is
/// `ping_at_or_under_limit_still_receives_snapshots`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn paid_frame_inside_limit_still_receives_snapshots() {
    let server = spawn_test_server(SigningKey::dev_default(), SignatureFailurePolicy::Kick).await;
    let (client, info) =
        TestClient::connect_and_join(server.ws_addr(), "us-east", "tick-paid").await;
    let (mut sink, mut stream) = client.into_split();
    let window = Duration::from_millis(500);
    // Shorter than tick_dt so a fresh sleep cannot elapse. Longer than 1/64 s
    // so every frame still pays on the 64/s server.
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
