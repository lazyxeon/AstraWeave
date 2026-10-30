//! The fourth player is the last one a room accepts. `JoinRoom` must not
//! skip the cap, and the check has to sit in the same lock as the insert.

mod common;

use aw_net_proto::{ClientToServer, ServerToClient, SigningKey, PROTOCOL_VERSION};
use aw_net_server::SignatureFailurePolicy;
use common::{spawn_test_server, TestClient};

async fn hello(client: &mut TestClient) {
    client
        .send_msg(&ClientToServer::Hello {
            protocol: PROTOCOL_VERSION,
        })
        .await;
    match client.recv_msg().await {
        ServerToClient::HelloAck { protocol } => assert_eq!(protocol, PROTOCOL_VERSION),
        other => panic!("expected HelloAck, got {other:?}"),
    }
}

#[tokio::test]
async fn fifth_player_join_room_is_refused() {
    let server = spawn_test_server(SigningKey::dev_default(), SignatureFailurePolicy::Kick).await;
    let (host, info) = TestClient::connect_and_join(server.ws_addr(), "us-east", "room-cap").await;
    // Keep every accepted socket open. Dropping one removes that player, and
    // the next JoinRoom would see a free seat.
    let mut held = vec![host];
    let mut refused = false;
    for i in 0..4 {
        let mut client = TestClient::connect(server.ws_addr()).await;
        hello(&mut client).await;
        client
            .send_msg(&ClientToServer::JoinRoom {
                room_id: info.room_id.clone(),
                display_name: format!("p{i}"),
            })
            .await;
        match client.recv_msg().await {
            ServerToClient::ProtocolError { msg } => {
                assert!(
                    msg.contains("room full"),
                    "refusal must say the room is full, got {msg}"
                );
                assert_eq!(i, 3, "only the fifth player is refused");
                refused = true;
            }
            ServerToClient::MatchResult { room_id } => {
                assert!(i < 3, "fifth player was accepted into {room_id}");
                assert_eq!(room_id, info.room_id);
                match client.recv_msg().await {
                    ServerToClient::JoinAccepted { room_id, .. } => {
                        assert_eq!(room_id, info.room_id);
                    }
                    other => panic!("expected JoinAccepted, got {other:?}"),
                }
                held.push(client);
            }
            other => panic!("expected MatchResult or room full, got {other:?}"),
        }
    }
    assert!(refused, "fifth JoinRoom was not refused");
    drop(held);
    server.shutdown();
}

#[tokio::test]
async fn parallel_find_or_create_never_puts_five_in_one_room() {
    let server = spawn_test_server(SigningKey::dev_default(), SignatureFailurePolicy::Kick).await;
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let addr = server.ws_addr();
        tasks.push(tokio::spawn(async move {
            TestClient::connect_and_join(addr, "us-east", "room-cap-race").await
        }));
    }
    let mut held = Vec::new();
    for task in tasks {
        held.push(task.await.expect("join task"));
    }
    let mut counts = std::collections::HashMap::<String, usize>::new();
    for (_client, info) in &held {
        *counts.entry(info.room_id.clone()).or_insert(0) += 1;
    }
    for (room_id, count) in &counts {
        assert!(
            *count <= 4,
            "room {room_id} has {count} players; the cap is 4"
        );
    }
    assert!(
        counts.values().any(|count| *count == 4),
        "expected one full room among {counts:?}"
    );
    drop(held);
    server.shutdown();
}
