//! Room tick must not depend on how many sockets are in the room.
//!
//! A client that sends one ping per tick is at or under the rate-limit
//! refill (64/s on the rate-limit server) and must still receive snapshots.
//! A tight flood above that refill may be closed. That case is
//! `rate_limit_kick::ping_flood_is_capped_and_closes`.
//!
//! The unfixed loop is `biased` with `ws.next()` first and a fresh
//! `sleep(tick_dt)` on the other arm. A ready frame wins every poll, so the
//! sleep arm is never polled. `build_snapshot` also adds one to `room.tick`
//! per connection, so four sockets advance the room about four times per period.

mod common;

use std::time::Duration;

use aw_net_proto::{ClientToServer, ServerToClient, SigningKey};
use aw_net_server::SignatureFailurePolicy;
use common::{spawn_test_server, TestClient};
use tokio::time::timeout;

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

/// One ping per tick stays under the rate-limit refill and must still
/// receive snapshots. A tight flood is allowed to be kicked.
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
