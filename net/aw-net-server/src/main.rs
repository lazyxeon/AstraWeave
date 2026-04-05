use std::{
    collections::HashMap, fs::File, io::BufReader, net::SocketAddr, path::Path, sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, Result};
use axum::{routing::get, Router};
use futures::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use parking_lot::Mutex;
use rustls_pemfile::{certs, private_key};
use sha2::Sha256;
use tokio::{net::TcpListener, time::Instant};
use tokio_rustls::{rustls::ServerConfig, TlsAcceptor};
use tokio_tungstenite::{
    accept_hdr_async, tungstenite::handshake::server::Request, tungstenite::protocol::Message,
};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use aw_net_proto::{
    new_room_id, ClientToServer, Codec, ServerToClient, SessionKey, PROTOCOL_VERSION,
};

type HmacSha256 = Hmac<Sha256>;

type PlayerId = String;
type RoomId = String;

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
    session_key: SessionKey,
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

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| anyhow!("Invalid TLS configuration: {}", e))?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(
                "info"
                    .parse()
                    .map_err(|e| anyhow!("Invalid log directive: {}", e))?,
            ),
        )
        .init();

    // Parse command-line arguments for TLS configuration
    let args: Vec<String> = std::env::args().collect();
    let mut tls_enabled = true;
    let mut cert_path = "net/certs/dev/dev-cert.pem".to_string();
    let mut key_path = "net/certs/dev/dev-key.pem".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--disable-tls" => {
                #[cfg(not(debug_assertions))]
                {
                    return Err(anyhow!(
                        "SECURITY: TLS cannot be disabled in release builds"
                    ));
                }
                #[cfg(debug_assertions)]
                {
                    tls_enabled = false;
                    info!("TLS disabled via command line (debug build only)");
                }
            }
            "--tls-cert" => {
                if i + 1 < args.len() {
                    cert_path = args[i + 1].clone();
                    i += 1;
                }
            }
            "--tls-key" => {
                if i + 1 < args.len() {
                    key_path = args[i + 1].clone();
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let db = sled::open("aw_net_server_db")?;
    let state = AppState {
        rooms: Arc::new(Mutex::new(HashMap::new())),
        db,
        codec: Codec::PostcardLz4,
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

    // Spawn HTTP server
    tokio::spawn(async move {
        let addr_result: Result<SocketAddr> = "0.0.0.0:8789"
            .parse()
            .map_err(|e| anyhow!("Invalid HTTP admin address: {}", e));

        match addr_result {
            Ok(addr) => {
                info!("HTTP admin on http://{}", addr);
                match TcpListener::bind(addr).await {
                    Ok(listener) => {
                        if let Err(e) = axum::serve(listener, http_app).await {
                            warn!("HTTP server error: {}", e);
                        }
                    }
                    Err(e) => warn!("Failed to bind HTTP server: {}", e),
                }
            }
            Err(e) => warn!("HTTP server initialization failed: {}", e),
        }
    });

    // WS server
    let ws_addr: SocketAddr = "0.0.0.0:8788"
        .parse()
        .map_err(|e| anyhow!("Invalid WebSocket address: {}", e))?;

    let listener = TcpListener::bind(ws_addr).await?;

    if tls_enabled {
        // TLS mode
        let tls_acceptor = match create_tls_acceptor(Path::new(&cert_path), Path::new(&key_path)) {
            Ok(acceptor) => acceptor,
            Err(e) => {
                warn!("Failed to load TLS certificates: {}", e);
                warn!("Run certificate generation script: net/certs/dev/generate_dev_cert.sh");
                warn!("Or use --disable-tls flag to run without TLS");
                return Err(e);
            }
        };
        info!("WSS (TLS) listening on wss://{}", ws_addr);
        info!("Using cert: {}, key: {}", cert_path, key_path);

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

                let peer = match accept_hdr_async(tls_stream, |_req: &Request, resp| Ok(resp)).await
                {
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
    } else {
        // Plain TCP mode (no TLS)
        info!("WS listening on ws:// {} (TLS DISABLED)", ws_addr);

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
    let mut session_hint = [0u8; 8];
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
                    let key = SessionKey::random();
                    let r = Room {
                        id: rid.clone(),
                        region,
                        game_mode,
                        session_key: key.clone(),
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

    // allocate player into room
    {
        let mut rooms = app.rooms.lock();
        let room = rooms.get_mut(&rid).expect("room exists");
        session_hint.copy_from_slice(&room.session_key.0[0..8]);
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
    }

    send_tls(
        &app,
        &mut ws,
        &ServerToClient::MatchResult {
            room_id: rid.clone(),
            session_key_hint: session_hint,
        },
    )
    .await?;
    send_tls(
        &app,
        &mut ws,
        &ServerToClient::JoinAccepted {
            room_id: rid.clone(),
            player_id: player_id.clone(),
            session_key_hint: session_hint,
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
                            if let Err(e) = on_client_msg_tls(&app, &rid, &player_id, &mut ws, m).await {
                                warn!("client msg error: {e:?}");
                            }
                        }
                    }
                    Some(Ok(Message::Ping(p))) => { let _ = ws.send(Message::Pong(p)).await; }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => { warn!("ws recv: {e}"); break; }
                    None => break,
                }
            }

            // Send authoritative snapshot periodically
            _ = tokio::time::sleep(tick_dt) => {
                let (snap, sid) = build_snapshot(&app, &rid)?;
                _last_snap = sid;
                send_tls(&app, &mut ws, &snap).await?;
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
    let mut session_hint = [0u8; 8];
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
                    let key = SessionKey::random();
                    let r = Room {
                        id: rid.clone(),
                        region,
                        game_mode,
                        session_key: key.clone(),
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

    // allocate player into room
    {
        let mut rooms = app.rooms.lock();
        let room = rooms.get_mut(&rid).expect("room exists");
        session_hint.copy_from_slice(&room.session_key.0[0..8]);
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
    }

    send(
        &app,
        &mut ws,
        &ServerToClient::MatchResult {
            room_id: rid.clone(),
            session_key_hint: session_hint,
        },
    )
    .await?;
    send(
        &app,
        &mut ws,
        &ServerToClient::JoinAccepted {
            room_id: rid.clone(),
            player_id: player_id.clone(),
            session_key_hint: session_hint,
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
                            if let Err(e) = on_client_msg(&app, &rid, &player_id, &mut ws, m).await {
                                warn!("client msg error: {e:?}");
                            }
                        }
                    }
                    Some(Ok(Message::Ping(p))) => { let _ = ws.send(Message::Pong(p)).await; }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => { warn!("ws recv: {e}"); break; }
                    None => break,
                }
            }

            // Send authoritative snapshot periodically
            _ = tokio::time::sleep(tick_dt) => {
                let (snap, sid) = build_snapshot(&app, &rid)?;
                _last_snap = sid;
                send(&app, &mut ws, &snap).await?;
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
) -> Result<()> {
    match msg {
        ClientToServer::InputFrame {
            seq,
            input_blob,
            sig,
            ..
        } => {
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
                        // Cryptographically secure tamper check: HMAC-SHA256
                        let mut mac = HmacSha256::new_from_slice(&room.session_key.0)
                            .expect("HMAC can take key of any size");
                        mac.update(&input_blob);

                        if mac.verify_slice(&sig).is_err() {
                            warn!("HMAC signature verification failed for pid={pid}");
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
    Ok(())
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
) -> Result<()> {
    match msg {
        ClientToServer::InputFrame {
            seq,
            input_blob,
            sig,
            ..
        } => {
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
                        // Cryptographically secure tamper check: HMAC-SHA256
                        let mut mac = HmacSha256::new_from_slice(&room.session_key.0)
                            .expect("HMAC can take key of any size");
                        mac.update(&input_blob);

                        if mac.verify_slice(&sig).is_err() {
                            warn!("HMAC signature verification failed for pid={pid}");
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
    Ok(())
}
