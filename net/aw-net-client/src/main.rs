use std::time::Duration;

use anyhow::Context;
use aw_net_proto::{ClientToServer, Codec, ServerToClient, SigningKey, PROTOCOL_VERSION};
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Default to wss:// for secure connection, fallback to ws:// if specified
    let url = std::env::var("AW_WS_URL").unwrap_or_else(|_| "wss://127.0.0.1:8788".into());
    let region = std::env::var("AW_REGION").unwrap_or_else(|_| "us-east".into());

    // Shared HMAC-SHA256 signing key for InputFrame signatures. A malformed
    // value is a hard error — NEVER silently fall back to the dev key.
    let signing_key = match std::env::var("AW_SHARED_KEY") {
        Ok(hex_key) => SigningKey::from_hex(&hex_key).context(
            "AW_SHARED_KEY is set but invalid; expected exactly 64 hex characters (32 bytes)",
        )?,
        Err(std::env::VarError::NotPresent) => {
            warn!("AW_SHARED_KEY not set; using built-in development signing key — set AW_SHARED_KEY (64 hex chars) for real deployments");
            SigningKey::dev_default()
        }
        Err(std::env::VarError::NotUnicode(_)) => {
            anyhow::bail!("AW_SHARED_KEY is set but is not valid UTF-8; expected exactly 64 hex characters (32 bytes)");
        }
    };

    // Connect with native-tls (supports both ws:// and wss://)
    // For development with self-signed certs, you may need to disable certificate validation
    let (mut ws, _resp) = tokio_tungstenite::connect_async(&url).await.map_err(|e| {
        anyhow::anyhow!("Connection failed: {}. If using self-signed certs, this is expected. Use ws:// or set AW_WS_URL=ws://127.0.0.1:8788", e)
    })?;
    info!("connected to {url}");

    send(
        &mut ws,
        &ClientToServer::Hello {
            protocol: PROTOCOL_VERSION,
        },
    )
    .await?;

    // Request a room (or create)
    send(
        &mut ws,
        &ClientToServer::FindOrCreate {
            region,
            game_mode: "coop".into(),
            party_size: 1,
        },
    )
    .await?;

    // Wait match + accept
    loop {
        let msg = ws.next().await.ok_or_else(|| anyhow::anyhow!("closed"))??;
        if let Message::Binary(b) = msg {
            let m = aw_net_proto::decode_msg::<ServerToClient>(Codec::PostcardLz4, &b)?;
            match m {
                ServerToClient::HelloAck { .. } => {}
                ServerToClient::MatchResult { .. } => {}
                ServerToClient::JoinAccepted { tick_hz, .. } => {
                    info!("joined; tick_hz={tick_hz}");
                    break;
                }
                ServerToClient::ProtocolError { msg } => {
                    anyhow::bail!("server error: {msg}");
                }
                _ => {}
            }
        }
    }

    // Client prediction loop (demo input)
    let mut seq = 1u32;
    let codec = aw_net_proto::Codec::PostcardLz4;

    let input_tick = Duration::from_millis(33);
    loop {
        // build tiny demo input blob (e.g. movement intent)
        #[derive(serde::Serialize)]
        struct DemoInput {
            forward: f32,
            strafe: f32,
            jump: bool,
        }
        let cmd = DemoInput {
            forward: 1.0,
            strafe: 0.0,
            jump: false,
        };
        let blob = postcard::to_allocvec(&cmd).unwrap();
        // The MAC'd byte range MUST come from the canonical payload builder,
        // over EXACTLY the same seq/tick_ms placed in the InputFrame fields.
        let tick_ms: u64 = 33;
        let payload = aw_net_proto::input_frame_sig_payload(seq, tick_ms, &blob);
        let sig = aw_net_proto::sign(&signing_key, &payload);
        send(
            &mut ws,
            &ClientToServer::InputFrame {
                seq,
                tick_ms,
                input_blob: blob,
                sig,
            },
        )
        .await?;

        // read any server messages
        while let Ok(Some(msg)) = tokio::time::timeout(Duration::from_millis(1), ws.next()).await {
            match msg {
                Ok(Message::Binary(b)) => {
                    let m = aw_net_proto::decode_msg::<ServerToClient>(codec, &b)?;
                    match m {
                        ServerToClient::Snapshot {
                            id,
                            server_tick,
                            compressed,
                            payload,
                            ..
                        } => {
                            // demo: decompress and read tick
                            let bytes = if compressed {
                                lz4_flex::decompress_size_prepended(&payload).unwrap()
                            } else {
                                payload
                            };
                            #[derive(serde::Deserialize)]
                            struct DemoState {
                                tick: u64,
                            }
                            let st: DemoState = postcard::from_bytes(&bytes).unwrap();
                            tracing::info!(
                                "snapshot id={id} tick={server_tick} state.tick={}",
                                st.tick
                            );
                            // reconciliation placeholder: would apply correction vs predicted state
                        }
                        ServerToClient::Reconcile {
                            input_seq_ack,
                            corrected_state_hash,
                        } => {
                            tracing::info!(
                                "reconcile ack={} hash={}",
                                input_seq_ack,
                                corrected_state_hash
                            );
                        }
                        ServerToClient::RateLimited => {
                            warn!("rate limited by server");
                        }
                        _ => {}
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    warn!("ws recv: {e}");
                    break;
                }
            }
        }

        seq = seq.wrapping_add(1);
        tokio::time::sleep(input_tick).await;
    }
}

async fn send(
    ws: &mut WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    msg: &ClientToServer,
) -> anyhow::Result<()> {
    let bytes = aw_net_proto::encode_msg(aw_net_proto::Codec::PostcardLz4, msg);
    ws.send(Message::Binary(bytes.into())).await?;
    Ok(())
}
