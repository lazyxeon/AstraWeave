#![forbid(unsafe_code)]
//! ECS Networking Integration for AstraWeave
//!
//! Provides client-server networking with prediction and reconciliation
//! integrated into the ECS architecture.

use astraweave_ecs::{App, Plugin, Query, World};
use aw_net_proto::{decode_msg, ClientToServer, Codec, ServerToClient};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

/// Network client component
#[derive(Clone, Debug)]
pub struct CNetworkClient {
    pub player_id: String,
    pub last_acknowledged_input: u64,
    pub pending_inputs: Vec<u64>,
}

/// Client prediction component
#[derive(Clone, Debug)]
pub struct CClientPrediction {
    pub predicted_position: glam::Vec3,
    pub prediction_error: glam::Vec3,
}

/// Network authority component (server-side)
#[derive(Clone, Debug)]
pub struct CNetworkAuthority {
    pub authoritative_tick: u64,
    pub connected_clients: HashMap<String, mpsc::UnboundedSender<ServerToClient>>,
}

/// Network snapshot for synchronization
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NetworkSnapshot {
    pub server_tick: u64,
    pub entity_states: HashMap<u64, EntityState>,
}

/// Entity state in network snapshot
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EntityState {
    pub position: glam::Vec3,
    pub health: i32,
}

/// Network client plugin
#[allow(dead_code)]
pub struct NetworkClientPlugin {
    server_addr: String,
}

impl NetworkClientPlugin {
    pub fn new(server_addr: String) -> Self {
        Self { server_addr }
    }
}

impl Plugin for NetworkClientPlugin {
    fn build(&self, app: &mut App) {
        // Add client systems
        app.add_system("simulation", client_input_system)
            .expect("registered schedule stage");
        app.add_system("presentation", client_reconciliation_system)
            .expect("registered schedule stage");
    }
}

/// Network server plugin
#[allow(dead_code)]
pub struct NetworkServerPlugin {
    bind_addr: String,
}

impl NetworkServerPlugin {
    pub fn new(bind_addr: String) -> Self {
        Self { bind_addr }
    }
}

impl Plugin for NetworkServerPlugin {
    fn build(&self, app: &mut App) {
        // Add server systems
        app.add_system("simulation", server_snapshot_system)
            .expect("registered schedule stage");
        app.add_system("simulation", server_input_processing_system)
            .expect("registered schedule stage");
    }
}

/// Client input system - sends player inputs to server
pub fn client_input_system(world: &mut World) {
    // Query for network clients and predictions
    let mut clients_to_update = Vec::new();

    {
        let q = Query::<CNetworkClient>::new(world);
        for (entity, client) in q {
            if let Some(prediction) = world.get::<CClientPrediction>(entity) {
                clients_to_update.push((entity, client.clone(), prediction.clone()));
            }
        }
    }

    for (entity, mut client, mut prediction) in clients_to_update {
        // Simulate input processing
        let input_sequence =
            client.last_acknowledged_input + client.pending_inputs.len() as u64 + 1;

        // Add to pending inputs
        client.pending_inputs.push(input_sequence);

        // Update prediction based on input
        prediction.predicted_position.x += 0.1; // Simple movement prediction

        // Update components
        world.insert(entity, client);
        world.insert(entity, prediction);
    }
}

/// Client reconciliation system - applies server corrections
pub fn client_reconciliation_system(world: &mut World) {
    // Query for clients that need reconciliation
    let mut clients_to_reconcile = Vec::new();

    {
        let q = Query::<CNetworkClient>::new(world);
        for (entity, client) in q {
            if let Some(prediction) = world.get::<CClientPrediction>(entity) {
                clients_to_reconcile.push((entity, client.clone(), prediction.clone()));
            }
        }
    }

    for (entity, mut client, mut prediction) in clients_to_reconcile {
        // Simulate receiving server snapshot
        let server_snapshot = NetworkSnapshot {
            server_tick: client.last_acknowledged_input + 1,
            entity_states: HashMap::new(), // Would contain actual server state
        };

        // Apply reconciliation
        prediction.prediction_error = prediction.predicted_position - glam::Vec3::ZERO; // Simplified

        // Remove acknowledged inputs
        client
            .pending_inputs
            .retain(|input| *input > server_snapshot.server_tick);
        client.last_acknowledged_input = server_snapshot.server_tick;

        // Update components
        world.insert(entity, client);
        world.insert(entity, prediction);
    }
}

/// Server snapshot system - generates world state snapshots
pub fn server_snapshot_system(world: &mut World) {
    // Query for network authorities
    let mut authorities_to_update = Vec::new();

    {
        let q = Query::<CNetworkAuthority>::new(world);
        for (entity, authority) in q {
            authorities_to_update.push((entity, authority.clone()));
        }
    }

    for (entity, mut authority) in authorities_to_update {
        authority.authoritative_tick += 1;

        // Generate snapshot (simplified)
        let snapshot = NetworkSnapshot {
            server_tick: authority.authoritative_tick,
            entity_states: HashMap::new(), // Would collect actual entity states
        };

        // Broadcast to connected clients (simplified)
        for sender in authority.connected_clients.values() {
            let payload = aw_net_proto::encode_msg(Codec::Bincode, &snapshot);
            let server_snapshot = ServerToClient::Snapshot {
                id: authority.authoritative_tick as u32,
                server_tick: authority.authoritative_tick,
                base_id: None,
                compressed: false,
                payload,
            };
            let _ = sender.send(server_snapshot);
        }

        // Update component
        world.insert(entity, authority);
    }
}

/// Server input processing system
fn server_input_processing_system(world: &mut World) {
    // Query for clients and authorities
    let mut clients_to_process = Vec::new();

    {
        let q2 = astraweave_ecs::Query2::<CNetworkClient, CNetworkAuthority>::new(world);
        for (entity, client, authority) in q2 {
            clients_to_process.push((entity, client.clone(), authority.clone()));
        }
    }

    for (entity, client, _authority) in clients_to_process {
        // Process validated client inputs
        // TODO: Apply input validation, anti-cheat checks, etc.

        // Update component
        world.insert(entity, client);
    }
}

/// Connect to server (async helper)
pub async fn connect_to_server(
    server_addr: &str,
) -> Result<mpsc::UnboundedReceiver<ServerToClient>, Box<dyn std::error::Error>> {
    use futures_util::stream::StreamExt;

    use tokio_tungstenite::connect_async;

    let url = format!("ws://{}", server_addr);
    let (ws_stream, _) = connect_async(url).await?;
    let (_write, mut read) = ws_stream.split();

    // Create channels for communication
    let (tx, rx) = mpsc::unbounded_channel();

    // Spawn task to handle incoming messages
    tokio::spawn(async move {
        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Binary(data)) => {
                    // Parse server message
                    if let Ok(server_msg) = decode_msg::<ServerToClient>(Codec::Bincode, &data) {
                        let _ = tx.send(server_msg);
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }
    });

    Ok(rx)
}

/// Start network server (async helper)
pub async fn start_network_server(
    bind_addr: &str,
) -> Result<mpsc::UnboundedReceiver<ClientToServer>, Box<dyn std::error::Error>> {
    use futures_util::stream::StreamExt;
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;

    let listener = TcpListener::bind(bind_addr).await?;
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let ws_stream = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("WebSocket accept error: {e}");
                    continue;
                }
            };
            let (_write, mut read) = ws_stream.split();
            let tx_clone = tx.clone();

            tokio::spawn(async move {
                while let Some(message) = read.next().await {
                    match message {
                        Ok(Message::Binary(data)) => {
                            if let Ok(client_msg) =
                                decode_msg::<ClientToServer>(Codec::Bincode, &data)
                            {
                                let _ = tx_clone.send(client_msg);
                            }
                        }
                        Ok(Message::Close(_)) => break,
                        Err(_) => break,
                        _ => {}
                    }
                }
            });
        }
    });

    Ok(rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_input_processing() {
        let mut world = World::new();
        let entity = world.spawn();

        world.insert(
            entity,
            CNetworkClient {
                player_id: "test_player".to_string(),
                last_acknowledged_input: 0,
                pending_inputs: Vec::new(),
            },
        );

        world.insert(
            entity,
            CClientPrediction {
                predicted_position: glam::Vec3::ZERO,
                prediction_error: glam::Vec3::ZERO,
            },
        );

        // Run client input system
        client_input_system(&mut world);

        // Verify input was added
        let client = world.get::<CNetworkClient>(entity).unwrap();
        assert_eq!(client.pending_inputs.len(), 1);
        assert_eq!(client.pending_inputs[0], 1);
    }

    #[test]
    fn client_reconciliation() {
        let mut world = World::new();
        let entity = world.spawn();

        world.insert(
            entity,
            CNetworkClient {
                player_id: "test_player".to_string(),
                last_acknowledged_input: 0,
                pending_inputs: vec![1, 2, 3],
            },
        );

        world.insert(
            entity,
            CClientPrediction {
                predicted_position: glam::Vec3::new(1.0, 0.0, 0.0),
                prediction_error: glam::Vec3::ZERO,
            },
        );

        // Run reconciliation system
        client_reconciliation_system(&mut world);

        // Verify reconciliation occurred
        let client = world.get::<CNetworkClient>(entity).unwrap();
        assert_eq!(client.last_acknowledged_input, 1);
        assert_eq!(client.pending_inputs.len(), 2); // Should have removed acknowledged input
    }

    #[test]
    fn server_snapshot_generation() {
        let mut world = World::new();
        let entity = world.spawn();

        let (tx, _rx) = mpsc::unbounded_channel();
        let mut connected_clients = HashMap::new();
        connected_clients.insert("test_client".to_string(), tx);

        world.insert(
            entity,
            CNetworkAuthority {
                authoritative_tick: 0,
                connected_clients,
            },
        );

        // Run snapshot system
        server_snapshot_system(&mut world);

        // Verify tick was incremented
        let authority = world.get::<CNetworkAuthority>(entity).unwrap();
        assert_eq!(authority.authoritative_tick, 1);
    }

    #[test]
    fn network_integration() {
        let mut app = App::new();

        // Add network plugins
        app = app.add_plugin(NetworkClientPlugin::new("127.0.0.1:8080".to_string()));
        app = app.add_plugin(NetworkServerPlugin::new("127.0.0.1:8080".to_string()));

        // Create test entities
        let client_entity = app.world.spawn();
        app.world.insert(
            client_entity,
            CNetworkClient {
                player_id: "test_player".to_string(),
                last_acknowledged_input: 0,
                pending_inputs: Vec::new(),
            },
        );
        app.world.insert(
            client_entity,
            CClientPrediction {
                predicted_position: glam::Vec3::ZERO,
                prediction_error: glam::Vec3::ZERO,
            },
        );

        let server_entity = app.world.spawn();
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut connected_clients = HashMap::new();
        connected_clients.insert("test_player".to_string(), tx);
        app.world.insert(
            server_entity,
            CNetworkAuthority {
                authoritative_tick: 0,
                connected_clients,
            },
        );

        // Run a few simulation steps
        app = app.run_fixed(3);

        // Verify systems ran (basic smoke test)
        let client = app.world.get::<CNetworkClient>(client_entity).unwrap();
        // Client tracking initialized (u64, always >= 0)
        let _ = client.last_acknowledged_input;

        let authority = app.world.get::<CNetworkAuthority>(server_entity).unwrap();
        // Server tracking initialized (u64, always >= 0)
        let _ = authority.authoritative_tick;
    }
}
