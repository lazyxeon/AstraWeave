//! aw-net-server library: snapshot-broadcast WebSocket game server with
//! room-based matchmaking, token-bucket rate limiting, and HMAC-SHA256
//! input-frame verification keyed by the shared [`SigningKey`].
//!
//! The binary target (`main.rs`) is a thin CLI wrapper: it parses arguments
//! into a [`ServerConfig`] and calls [`run_server`]. Integration tests use
//! [`spawn_server`] with `127.0.0.1:0` listen addresses to bind real
//! ephemeral ports and drive a live server in-process.

use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    net::SocketAddr,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use axum::{routing::get, Router};
use futures::{SinkExt, StreamExt};
use parking_lot::Mutex;
use rustls_pemfile::{certs, private_key};
use tokio::{net::TcpListener, task::JoinHandle, time::Instant};
use tokio_rustls::{rustls::ServerConfig as RustlsServerConfig, TlsAcceptor};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::handshake::server::Request,
    tungstenite::protocol::{frame::coding::CloseCode, CloseFrame, Message},
};
use tracing::{info, warn};

use aw_net_proto::{
    new_room_id, ClientToServer, Codec, ServerToClient, SigningKey, PROTOCOL_VERSION,
};

type PlayerId = String;
type RoomId = String;

/// What the server does when an [`ClientToServer::InputFrame`] signature
/// fails verification against the shared [`SigningKey`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SignatureFailurePolicy {
    /// Reject the packet (no state updates) and disconnect the client via a
    /// best-effort WebSocket close frame (policy violation, 1008).
    #[default]
    Kick,
    /// Log the failure and process the packet anyway (legacy behavior).
    Warn,
}

impl FromStr for SignatureFailurePolicy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "kick" => Ok(Self::Kick),
            "warn" => Ok(Self::Warn),
            other => Err(anyhow!(
                "invalid signature failure policy '{other}': expected 'kick' or 'warn'"
            )),
        }
    }
}

/// Full server configuration. `Default` preserves the historical hardcoded
/// behavior: TLS on with the dev cert paths, listen on `0.0.0.0:8788` (WS)
/// and `0.0.0.0:8789` (HTTP admin), sled db at `aw_net_server_db`, the
/// development signing key, and the `Kick` signature-failure policy.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    /// WebSocket listen address.
    pub ws_listen: SocketAddr,
    /// HTTP admin (healthz/regions) listen address.
    pub http_listen: SocketAddr,
    /// Whether the WebSocket listener wraps connections in TLS.
    pub tls_enabled: bool,
    /// TLS certificate path (PEM).
    pub tls_cert_path: PathBuf,
    /// TLS private key path (PEM).
    pub tls_key_path: PathBuf,
    /// sled database directory (sled holds an exclusive lock on it).
    pub db_path: PathBuf,
    /// Shared symmetric key verifying `InputFrame` signatures.
    pub signing_key: SigningKey,
    /// Policy applied when an `InputFrame` signature fails verification.
    pub sig_failure_policy: SignatureFailurePolicy,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ws_listen: SocketAddr::from(([0, 0, 0, 0], 8788)),
            http_listen: SocketAddr::from(([0, 0, 0, 0], 8789)),
            tls_enabled: true,
            tls_cert_path: PathBuf::from("net/certs/dev/dev-cert.pem"),
            tls_key_path: PathBuf::from("net/certs/dev/dev-key.pem"),
            db_path: PathBuf::from("aw_net_server_db"),
            signing_key: SigningKey::dev_default(),
            sig_failure_policy: SignatureFailurePolicy::default(),
        }
    }
}

/// Handle to a server spawned via [`spawn_server`]. Listeners are already
/// bound, so `ws_addr`/`http_addr` carry the real (possibly ephemeral) ports.
pub struct RunningServer {
    /// Bound WebSocket listener address.
    pub ws_addr: SocketAddr,
    /// Bound HTTP admin listener address.
    pub http_addr: SocketAddr,
    ws_task: JoinHandle<Result<()>>,
    http_task: JoinHandle<()>,
}

impl RunningServer {
    /// Stop the server by aborting its accept loops. Already-established
    /// connection tasks are detached and end when their sockets close.
    pub fn shutdown(self) {
        self.ws_task.abort();
        self.http_task.abort();
    }
}

/// Outcome of handling one in-session client message.
enum MsgOutcome {
    /// Keep the connection open.
    Continue,
    /// Disconnect the client (policy violation). The connection loop sends a
    /// best-effort close frame (1008) and breaks into the shared cleanup
    /// block, which removes the player from the room (dropping empty rooms).
    Kick(&'static str),
}

#[derive(Clone)]
struct Player {
    #[allow(dead_code)]
    id: PlayerId,
    #[allow(dead_code)]
    display: String,
    last_input_seq: u32,
    last_seen: Instant,
    // token bucket rate limit
    tokens: f32,
    last_refill: Instant,
}

#[derive(Clone)]
struct Room {
    #[allow(dead_code)]
    id: RoomId,
    region: String,
    game_mode: String,
    tick_hz: u32,
    players: HashMap<PlayerId, Player>,
    // minimal world tick counter
    tick: u64,
    // last snapshot id
    snap_id: u32,
}

#[derive(Clone)]
struct AppState {
    rooms: Arc<Mutex<HashMap<RoomId, Room>>>,
    // sled persistence as key-value: "room:<id>" => JSON, "player:<id>" => JSON
    #[allow(dead_code)]
    db: sled::Db,
    codec: Codec,
    signing_key: SigningKey,
    sig_failure_policy: SignatureFailurePolicy,
}

// TLS certificate loading functions
fn load_certs(path: &Path) -> Result<Vec<rustls::pki_types::CertificateDer<'static>>> {
    let file = File::open(path)
        .map_err(|e| anyhow!("Failed to open certificate file {}: {}", path.display(), e))?;
    let mut reader = BufReader::new(file);
    certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow!("Failed to parse certificates: {}", e))
}

fn load_private_key(path: &Path) -> Result<rustls::pki_types::PrivateKeyDer<'static>> {
    let file = File::open(path)
        .map_err(|e| anyhow!("Failed to open private key file {}: {}", path.display(), e))?;
    let mut reader = BufReader::new(file);
    private_key(&mut reader)
        .map_err(|e| anyhow!("Failed to parse private key: {}", e))?
        .ok_or_else(|| anyhow!("No private key found in {}", path.display()))
}

fn create_tls_acceptor(cert_path: &Path, key_path: &Path) -> Result<TlsAcceptor> {
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;

    let config = RustlsServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| anyhow!("Invalid TLS configuration: {}", e))?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// Run the server until its WebSocket accept loop ends (normally never).
/// This is the binary's entry point after CLI parsing.
pub async fn run_server(config: ServerConfig) -> Result<()> {
    let server = spawn_server(config).await?;
    let RunningServer {
        ws_task, http_task, ..
    } = server;
    let result = match ws_task.await {
        Ok(r) => r,
        Err(e) => Err(anyhow!("WebSocket accept loop task failed: {e}")),
    };
    http_task.abort();
    result
}

/// Bind the HTTP and WebSocket listeners, then spawn the accept loops as
/// background tasks. Binding happens FIRST so that configuring
/// `127.0.0.1:0` yields real ephemeral ports in the returned
/// [`RunningServer`] — this is the test seam.
pub async fn spawn_server(config: ServerConfig) -> Result<RunningServer> {
    let db = sled::open(&config.db_path)
        .with_context(|| format!("Failed to open sled db at {}", config.db_path.display()))?;
    let state = AppState {
        rooms: Arc::new(Mutex::new(HashMap::new())),
        db,
        codec: Codec::PostcardLz4,
        signing_key: config.signing_key.clone(),
        sig_failure_policy: config.sig_failure_policy,
    };

    // Health check and region info (HTTP)
    let http_app = {
        let app_state = state.clone();
        Router::new()
            .route("/healthz", get(|| async { "ok" }))
            .route(
                "/regions",
                get(|| async { r#"["us-east","us-west","eu-central"]"# }),
            )
            .with_state(app_state)
    };

    let http_listener = TcpListener::bind(config.http_listen)
        .await
        .with_context(|| {
            format!(
                "Failed to bind HTTP admin listener on {}",
                config.http_listen
            )
        })?;
    let http_addr = http_listener
        .local_addr()
        .context("Failed to read HTTP admin listener local address")?;
    info!("HTTP admin on http://{}", http_addr);
    let http_task = tokio::spawn(async move {
        if let Err(e) = axum::serve(http_listener, http_app).await {
            warn!("HTTP server error: {}", e);
        }
    });

    // WS server
    let ws_listener = TcpListener::bind(config.ws_listen)
        .await
        .with_context(|| format!("Failed to bind WebSocket listener on {}", config.ws_listen))?;
    let ws_addr = ws_listener
        .local_addr()
        .context("Failed to read WebSocket listener local address")?;

    let ws_task = if config.tls_enabled {
        // TLS mode
        let tls_acceptor = match create_tls_acceptor(&config.tls_cert_path, &config.tls_key_path) {
            Ok(acceptor) => acceptor,
            Err(e) => {
                warn!("Failed to load TLS certificates: {}", e);
                warn!("Run certificate generation script: net/certs/dev/generate_dev_cert.sh");
                warn!("Or use --disable-tls flag to run without TLS");
                http_task.abort();
                return Err(e);
            }
        };
        info!("WSS (TLS) listening on wss://{}", ws_addr);
        info!(
            "Using cert: {}, key: {}",
            config.tls_cert_path.display(),
            config.tls_key_path.display()
        );

        tokio::spawn(accept_loop_tls(state, ws_listener, tls_acceptor))
    } else {
        // Plain TCP mode (no TLS)
        info!("WS listening on ws:// {} (TLS DISABLED)", ws_addr);

        tokio::spawn(accept_loop_plain(state, ws_listener))
    };

    Ok(RunningServer {
        ws_addr,
        http_addr,
        ws_task,
        http_task,
    })
}

async fn accept_loop_tls(
    state: AppState,
    listener: TcpListener,
    tls_acceptor: TlsAcceptor,
) -> Result<()> {
    loop {
        let (stream, _addr) = listener.accept().await?;
        let app = state.clone();
        let acceptor = tls_acceptor.clone();

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(stream).await {
                Ok(s) => s,
                Err(e) => {
                    warn!("TLS handshake failed: {}", e);
                    return;
                }
            };

            let peer = match accept_hdr_async(tls_stream, |_req: &Request, resp| Ok(resp)).await {
                Ok(ws) => ws,
                Err(e) => {
                    warn!("ws handshake failed: {e}");
                    return;
                }
            };
            if let Err(e) = handle_socket_tls(app, peer).await {
                warn!("ws session error: {e:?}");
            }
        });
    }
}

async fn accept_loop_plain(state: AppState, listener: TcpListener) -> Result<()> {
    loop {
        let (stream, _addr) = listener.accept().await?;
        let app = state.clone();
        tokio::spawn(async move {
            let peer = match accept_hdr_async(stream, |_req: &Request, resp| Ok(resp)).await {
                Ok(ws) => ws,
                Err(e) => {
                    warn!("ws handshake failed: {e}");
                    return;
                }
            };
            if let Err(e) = handle_socket(app, peer).await {
                warn!("ws session error: {e:?}");
            }
        });
    }
}

async fn handle_socket_tls(
    app: AppState,
    mut ws: tokio_tungstenite::WebSocketStream<
        tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    >,
) -> Result<()> {
    // Handshake
    let hello = recv_tls::<ClientToServer>(&app, &mut ws).await?;
    match hello {
        ClientToServer::Hello { protocol } if protocol == PROTOCOL_VERSION => {
            send_tls(&app, &mut ws, &ServerToClient::HelloAck { protocol }).await?;
        }
        ClientToServer::Hello { protocol } => {
            send_tls(
                &app,
                &mut ws,
                &ServerToClient::ProtocolError {
                    msg: format!("protocol mismatch: client={protocol}, server={PROTOCOL_VERSION}"),
                },
            )
            .await?;
            return Ok(());
        }
        _ => {
            send_tls(
                &app,
                &mut ws,
                &ServerToClient::ProtocolError {
                    msg: "expected Hello".into(),
                },
            )
            .await?;
            return Ok(());
        }
    }

    // Matchmaking or direct join
    let mut room_id: Option<RoomId> = None;
    let player_id = uuid::Uuid::new_v4().to_string();
    let mut tick_hz = 30u32;

    if let Ok(msg) = recv_tls::<ClientToServer>(&app, &mut ws).await {
        match msg {
            ClientToServer::FindOrCreate {
                region, game_mode, ..
            } => {
                let mut rooms = app.rooms.lock();
                if let Some((rid, _)) = rooms.iter().find(|(_, r)| {
                    r.region == region && r.game_mode == game_mode && r.players.len() < 4
                }) {
                    room_id = Some(rid.clone());
                } else {
                    let rid = new_room_id();
                    let r = Room {
                        id: rid.clone(),
                        region,
                        game_mode,
                        tick_hz,
                        players: HashMap::new(),
                        tick: 0,
                        snap_id: 0,
                    };
                    rooms.insert(rid.clone(), r);
                    room_id = Some(rid.clone());
                }
            }
            ClientToServer::JoinRoom {
                room_id: rid,
                display_name: _,
            } => {
                room_id = Some(rid);
            }
            _other => {
                warn!("unexpected message before join: {_other:?}");
            }
        }
    }

    let rid = match room_id {
        Some(x) => x,
        None => {
            send_tls(
                &app,
                &mut ws,
                &ServerToClient::ProtocolError {
                    msg: "no room selected".into(),
                },
            )
            .await?;
            return Ok(());
        }
    };

    // allocate player into room. The room can be missing here for two
    // reasons: (a) a hostile/buggy `JoinRoom` carried a room id that never
    // existed, or (b) a FindOrCreate race — the matched room's last player
    // disconnected and cleanup dropped the room between releasing and
    // re-taking the lock. Neither may panic the connection task: report a
    // best-effort ProtocolError and end the connection cleanly. No state is
    // leaked — the player is only inserted when the room exists.
    // MUST stay semantically identical to the non-TLS path in handle_socket.
    let room_exists = {
        let mut rooms = app.rooms.lock();
        match rooms.get_mut(&rid) {
            Some(room) => {
                room.players.insert(
                    player_id.clone(),
                    Player {
                        id: player_id.clone(),
                        display: "player".into(),
                        last_input_seq: 0,
                        last_seen: tokio::time::Instant::now(),
                        tokens: 30.0,
                        last_refill: tokio::time::Instant::now(),
                    },
                );
                tick_hz = room.tick_hz;
                true
            }
            None => false,
        }
    };
    if !room_exists {
        warn!("join refused: room {rid} does not exist");
        if let Err(e) = send_tls(
            &app,
            &mut ws,
            &ServerToClient::ProtocolError {
                msg: format!("room {rid} does not exist"),
            },
        )
        .await
        {
            warn!("failed to send ProtocolError for missing room {rid}: {e}");
        }
        return Ok(());
    }

    send_tls(
        &app,
        &mut ws,
        &ServerToClient::MatchResult {
            room_id: rid.clone(),
        },
    )
    .await?;
    send_tls(
        &app,
        &mut ws,
        &ServerToClient::JoinAccepted {
            room_id: rid.clone(),
            player_id: player_id.clone(),
            tick_hz,
        },
    )
    .await?;

    // Per-connection game loop
    let tick_dt = Duration::from_millis((1000 / tick_hz.max(1)) as u64);
    let mut _last_snap = 0u32;

    loop {
        tokio::select! {
            biased;

            // Receive client messages
            msg = ws.next() => {
                match msg {
                    Some(Ok(Message::Binary(bytes))) => {
                        if let Ok(m) = aw_net_proto::decode_msg::<ClientToServer>(app.codec, &bytes) {
                            match on_client_msg_tls(&app, &rid, &player_id, &mut ws, m).await {
                                Ok(MsgOutcome::Continue) => {}
                                Ok(MsgOutcome::Kick(reason)) => {
                                    // Best-effort policy-violation close (1008), then break so
                                    // the shared cleanup block below runs (player removal,
                                    // empty-room drop).
                                    let close = Message::Close(Some(CloseFrame {
                                        code: CloseCode::Policy,
                                        reason: reason.into(),
                                    }));
                                    if let Err(e) = ws.send(close).await {
                                        warn!("failed to send close frame to pid={player_id}: {e}");
                                    }
                                    break;
                                }
                                Err(e) => {
                                    warn!("client msg error: {e:?}");
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Ping(p))) => { let _ = ws.send(Message::Pong(p)).await; }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => { warn!("ws recv: {e}"); break; }
                    None => break,
                }
            }

            // Send authoritative snapshot periodically. Failures (snapshot
            // build or send to a vanished client) must NOT `?`-return out of
            // the handler — that would skip the shared cleanup block below
            // and permanently leak the player entry (ghost player; the room
            // would never empty). warn + break so cleanup always runs.
            _ = tokio::time::sleep(tick_dt) => {
                match build_snapshot(&app, &rid) {
                    Ok((snap, sid)) => {
                        _last_snap = sid;
                        if let Err(e) = send_tls(&app, &mut ws, &snap).await {
                            warn!("snapshot send failed for pid={player_id}: {e}; closing");
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("snapshot build failed for room {rid}: {e}; closing");
                        break;
                    }
                }
            }
        }
    }

    // Cleanup
    {
        let mut rooms = app.rooms.lock();
        if let Some(room) = rooms.get_mut(&rid) {
            room.players.remove(&player_id);
            if room.players.is_empty() {
                rooms.remove(&rid);
            }
        }
    }
    Ok(())
}

async fn handle_socket(
    app: AppState,
    mut ws: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
) -> Result<()> {
    // Handshake
    let hello = recv::<ClientToServer>(&app, &mut ws).await?;
    match hello {
        ClientToServer::Hello { protocol } if protocol == PROTOCOL_VERSION => {
            send(&app, &mut ws, &ServerToClient::HelloAck { protocol }).await?;
        }
        ClientToServer::Hello { protocol } => {
            send(
                &app,
                &mut ws,
                &ServerToClient::ProtocolError {
                    msg: format!("protocol mismatch: client={protocol}, server={PROTOCOL_VERSION}"),
                },
            )
            .await?;
            return Ok(());
        }
        _ => {
            send(
                &app,
                &mut ws,
                &ServerToClient::ProtocolError {
                    msg: "expected Hello".into(),
                },
            )
            .await?;
            return Ok(());
        }
    }

    // Matchmaking or direct join
    let mut room_id: Option<RoomId> = None;
    let player_id = uuid::Uuid::new_v4().to_string();
    let mut tick_hz = 30u32;

    if let Ok(msg) = recv::<ClientToServer>(&app, &mut ws).await {
        match msg {
            ClientToServer::FindOrCreate {
                region, game_mode, ..
            } => {
                let mut rooms = app.rooms.lock();
                if let Some((rid, _)) = rooms.iter().find(|(_, r)| {
                    r.region == region && r.game_mode == game_mode && r.players.len() < 4
                }) {
                    room_id = Some(rid.clone());
                } else {
                    let rid = new_room_id();
                    let r = Room {
                        id: rid.clone(),
                        region,
                        game_mode,
                        tick_hz,
                        players: HashMap::new(),
                        tick: 0,
                        snap_id: 0,
                    };
                    rooms.insert(rid.clone(), r);
                    room_id = Some(rid.clone());
                }
            }
            ClientToServer::JoinRoom {
                room_id: rid,
                display_name: _,
            } => {
                room_id = Some(rid);
            }
            _other => {
                warn!("unexpected message before join: {_other:?}");
            }
        }
    }

    let rid = match room_id {
        Some(x) => x,
        None => {
            send(
                &app,
                &mut ws,
                &ServerToClient::ProtocolError {
                    msg: "no room selected".into(),
                },
            )
            .await?;
            return Ok(());
        }
    };

    // allocate player into room. The room can be missing here for two
    // reasons: (a) a hostile/buggy `JoinRoom` carried a room id that never
    // existed, or (b) a FindOrCreate race — the matched room's last player
    // disconnected and cleanup dropped the room between releasing and
    // re-taking the lock. Neither may panic the connection task: report a
    // best-effort ProtocolError and end the connection cleanly. No state is
    // leaked — the player is only inserted when the room exists.
    // MUST stay semantically identical to the TLS path in handle_socket_tls.
    let room_exists = {
        let mut rooms = app.rooms.lock();
        match rooms.get_mut(&rid) {
            Some(room) => {
                room.players.insert(
                    player_id.clone(),
                    Player {
                        id: player_id.clone(),
                        display: "player".into(),
                        last_input_seq: 0,
                        last_seen: tokio::time::Instant::now(),
                        tokens: 30.0,
                        last_refill: tokio::time::Instant::now(),
                    },
                );
                tick_hz = room.tick_hz;
                true
            }
            None => false,
        }
    };
    if !room_exists {
        warn!("join refused: room {rid} does not exist");
        if let Err(e) = send(
            &app,
            &mut ws,
            &ServerToClient::ProtocolError {
                msg: format!("room {rid} does not exist"),
            },
        )
        .await
        {
            warn!("failed to send ProtocolError for missing room {rid}: {e}");
        }
        return Ok(());
    }

    send(
        &app,
        &mut ws,
        &ServerToClient::MatchResult {
            room_id: rid.clone(),
        },
    )
    .await?;
    send(
        &app,
        &mut ws,
        &ServerToClient::JoinAccepted {
            room_id: rid.clone(),
            player_id: player_id.clone(),
            tick_hz,
        },
    )
    .await?;

    // Per-connection game loop
    let tick_dt = Duration::from_millis((1000 / tick_hz.max(1)) as u64);
    let mut _last_snap = 0u32;

    loop {
        tokio::select! {
            biased;

            // Receive client messages
            msg = ws.next() => {
                match msg {
                    Some(Ok(Message::Binary(bytes))) => {
                        if let Ok(m) = aw_net_proto::decode_msg::<ClientToServer>(app.codec, &bytes) {
                            match on_client_msg(&app, &rid, &player_id, &mut ws, m).await {
                                Ok(MsgOutcome::Continue) => {}
                                Ok(MsgOutcome::Kick(reason)) => {
                                    // Best-effort policy-violation close (1008), then break so
                                    // the shared cleanup block below runs (player removal,
                                    // empty-room drop).
                                    let close = Message::Close(Some(CloseFrame {
                                        code: CloseCode::Policy,
                                        reason: reason.into(),
                                    }));
                                    if let Err(e) = ws.send(close).await {
                                        warn!("failed to send close frame to pid={player_id}: {e}");
                                    }
                                    break;
                                }
                                Err(e) => {
                                    warn!("client msg error: {e:?}");
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Ping(p))) => { let _ = ws.send(Message::Pong(p)).await; }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => { warn!("ws recv: {e}"); break; }
                    None => break,
                }
            }

            // Send authoritative snapshot periodically. Failures (snapshot
            // build or send to a vanished client) must NOT `?`-return out of
            // the handler — that would skip the shared cleanup block below
            // and permanently leak the player entry (ghost player; the room
            // would never empty). warn + break so cleanup always runs.
            _ = tokio::time::sleep(tick_dt) => {
                match build_snapshot(&app, &rid) {
                    Ok((snap, sid)) => {
                        _last_snap = sid;
                        if let Err(e) = send(&app, &mut ws, &snap).await {
                            warn!("snapshot send failed for pid={player_id}: {e}; closing");
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("snapshot build failed for room {rid}: {e}; closing");
                        break;
                    }
                }
            }
        }
    }

    // Cleanup
    {
        let mut rooms = app.rooms.lock();
        if let Some(room) = rooms.get_mut(&rid) {
            room.players.remove(&player_id);
            if room.players.is_empty() {
                rooms.remove(&rid);
            }
        }
    }
    Ok(())
}

fn build_snapshot(app: &AppState, rid: &str) -> Result<(ServerToClient, u32)> {
    let (server_tick, sid, payload) = {
        let mut rooms = app.rooms.lock();
        let room = rooms
            .get_mut(rid)
            .ok_or_else(|| anyhow!("Room {} not found", rid))?;
        room.tick += 1;
        room.snap_id = room.snap_id.wrapping_add(1);

        let server_tick = room.tick;
        let sid = room.snap_id;

        // Payload is engine-owned; here we emit minimal demo payload (tick only)
        #[derive(serde::Serialize)]
        struct DemoState {
            tick: u64,
        }
        let demo = DemoState { tick: server_tick };
        let raw = postcard::to_allocvec(&demo)
            .map_err(|e| anyhow!("Failed to serialize snapshot: {}", e))?;

        (server_tick, sid, raw)
    };

    let msg = ServerToClient::Snapshot {
        id: sid,
        server_tick,
        base_id: None,
        compressed: true,
        payload: lz4_flex::compress_prepend_size(&payload),
    };
    Ok((msg, sid))
}

async fn on_client_msg(
    app: &AppState,
    rid: &str,
    pid: &str,
    ws: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    msg: ClientToServer,
) -> Result<MsgOutcome> {
    match msg {
        ClientToServer::InputFrame {
            seq,
            tick_ms,
            input_blob,
            sig,
            ..
        } => {
            // Verify the signature FIRST — before last_input_seq/last_seen
            // updates and before rate-limit token deduction. An
            // unauthenticated packet must not influence server state.
            let payload = aw_net_proto::input_frame_sig_payload(seq, tick_ms, &input_blob);
            if !aw_net_proto::verify(&app.signing_key, &payload, &sig) {
                match app.sig_failure_policy {
                    SignatureFailurePolicy::Kick => {
                        warn!("HMAC signature verification failed for pid={pid}; kicking (policy: kick)");
                        return Ok(MsgOutcome::Kick(
                            "input frame signature verification failed",
                        ));
                    }
                    SignatureFailurePolicy::Warn => {
                        // Legacy behavior: log and process the packet anyway.
                        warn!("HMAC signature verification failed for pid={pid} (policy: warn)");
                    }
                }
            }

            // token bucket rate limit
            let mut kick = false;
            {
                let mut rooms = app.rooms.lock();
                if let Some(room) = rooms.get_mut(rid) {
                    if let Some(p) = room.players.get_mut(pid) {
                        let now = tokio::time::Instant::now();
                        let elapsed = now.duration_since(p.last_refill).as_secs_f32();

                        // Refill tokens based on elapsed time (8 tokens/sec)
                        const REFILL_RATE: f32 = 8.0;
                        const BUCKET_SIZE: f32 = 60.0;
                        const COST_PER_MESSAGE: f32 = 1.0;

                        p.tokens += REFILL_RATE * elapsed;
                        if p.tokens > BUCKET_SIZE {
                            p.tokens = BUCKET_SIZE;
                        }
                        p.last_refill = now;

                        // Deduct cost
                        p.tokens -= COST_PER_MESSAGE;

                        if p.tokens < 0.0 {
                            kick = true;
                        } else {
                            p.last_input_seq = seq;
                            p.last_seen = now;
                        }
                    }
                }
            }
            if kick {
                send(app, ws, &ServerToClient::RateLimited).await?;
            }
        }
        ClientToServer::Ping { nano } => {
            send(app, ws, &ServerToClient::Pong { nano }).await?;
        }
        ClientToServer::Ack { .. } => { /* optional: track delivery / deltas */ }
        _ => { /* ignore other in-session messages */ }
    }
    Ok(MsgOutcome::Continue)
}

// Helpers
async fn send(
    app: &AppState,
    ws: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    msg: &ServerToClient,
) -> Result<()> {
    let bytes = aw_net_proto::encode_msg(app.codec, msg);
    ws.send(Message::Binary(bytes.into())).await?;
    Ok(())
}
async fn recv<T: for<'de> serde::Deserialize<'de>>(
    app: &AppState,
    ws: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
) -> Result<T> {
    let msg = ws
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("ws closed"))??;
    match msg {
        Message::Binary(b) => {
            let t = aw_net_proto::decode_msg::<T>(app.codec, &b)?;
            Ok(t)
        }
        _ => Err(anyhow::anyhow!("unexpected ws message")),
    }
}

// TLS versions of helper functions
async fn send_tls(
    app: &AppState,
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    >,
    msg: &ServerToClient,
) -> Result<()> {
    let bytes = aw_net_proto::encode_msg(app.codec, msg);
    ws.send(Message::Binary(bytes.into())).await?;
    Ok(())
}

async fn recv_tls<T: for<'de> serde::Deserialize<'de>>(
    app: &AppState,
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    >,
) -> Result<T> {
    let msg = ws
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("ws closed"))??;
    match msg {
        Message::Binary(b) => {
            let t = aw_net_proto::decode_msg::<T>(app.codec, &b)?;
            Ok(t)
        }
        _ => Err(anyhow::anyhow!("unexpected ws message")),
    }
}

async fn on_client_msg_tls(
    app: &AppState,
    rid: &str,
    pid: &str,
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    >,
    msg: ClientToServer,
) -> Result<MsgOutcome> {
    match msg {
        ClientToServer::InputFrame {
            seq,
            tick_ms,
            input_blob,
            sig,
            ..
        } => {
            // Verify the signature FIRST — before last_input_seq/last_seen
            // updates and before rate-limit token deduction. An
            // unauthenticated packet must not influence server state.
            // MUST stay semantically identical to the non-TLS handler in
            // on_client_msg — a skippable path on either is a security bug.
            let payload = aw_net_proto::input_frame_sig_payload(seq, tick_ms, &input_blob);
            if !aw_net_proto::verify(&app.signing_key, &payload, &sig) {
                match app.sig_failure_policy {
                    SignatureFailurePolicy::Kick => {
                        warn!("HMAC signature verification failed for pid={pid}; kicking (policy: kick)");
                        return Ok(MsgOutcome::Kick(
                            "input frame signature verification failed",
                        ));
                    }
                    SignatureFailurePolicy::Warn => {
                        // Legacy behavior: log and process the packet anyway.
                        warn!("HMAC signature verification failed for pid={pid} (policy: warn)");
                    }
                }
            }

            // token bucket rate limit
            let mut kick = false;
            {
                let mut rooms = app.rooms.lock();
                if let Some(room) = rooms.get_mut(rid) {
                    if let Some(p) = room.players.get_mut(pid) {
                        let now = tokio::time::Instant::now();
                        let elapsed = now.duration_since(p.last_refill).as_secs_f32();

                        // Refill tokens based on elapsed time (8 tokens/sec)
                        const REFILL_RATE: f32 = 8.0;
                        const BUCKET_SIZE: f32 = 60.0;
                        const COST_PER_MESSAGE: f32 = 1.0;

                        p.tokens += REFILL_RATE * elapsed;
                        if p.tokens > BUCKET_SIZE {
                            p.tokens = BUCKET_SIZE;
                        }
                        p.last_refill = now;

                        // Deduct cost
                        p.tokens -= COST_PER_MESSAGE;

                        if p.tokens < 0.0 {
                            kick = true;
                        } else {
                            p.last_input_seq = seq;
                            p.last_seen = now;
                        }
                    }
                }
            }
            if kick {
                send_tls(app, ws, &ServerToClient::RateLimited).await?;
            }
        }
        ClientToServer::Ping { nano } => {
            send_tls(app, ws, &ServerToClient::Pong { nano }).await?;
        }
        ClientToServer::Ack { .. } => { /* optional: track delivery / deltas */ }
        _ => { /* ignore other in-session messages */ }
    }
    Ok(MsgOutcome::Continue)
}
