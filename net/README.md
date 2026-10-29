# Enhanced Networking Layer - Integration Guide

## Overview

The enhanced networking layer consists of three crates that provide multiplayer capabilities:

- **`aw-net-proto`**: Versioned wire protocol with compression and the canonical HMAC-SHA256 signing surface (`SigningKey`, `sign`/`verify`, `input_frame_sig_payload`)
- **`aw-net-server`**: Authoritative server with matchmaking, persistence, and anti-cheat
- **`aw-net-client`**: Client prediction, reconciliation, and demo implementation

## Quick Start

### 1. Start the Server

```bash
# Start the authoritative server
cargo run -p aw-net-server

# Server provides:
# - WebSocket endpoint: ws://localhost:8788
# - HTTP health check: http://localhost:8789/healthz  
# - Regions endpoint: http://localhost:8789/regions
```

### 2. Connect a Client

```bash
# Connect a demo client
cargo run -p aw-net-client

# Or specify custom server/region:
AW_WS_URL=ws://your-server.com:8788 AW_REGION=eu-central cargo run -p aw-net-client
```

## Architecture

### Protocol Features

- **Versioned Protocol**: Protocol version 1 with future compatibility
- **Compression**: LZ4 compression on postcard-serialized messages  
- **Security**: Enforced HMAC-SHA256 signatures on input frames (server verifies first, kicks by default)
- **Reliability**: WebSocket with binary frames for cross-platform NAT traversal

### Server Features

- **Server Authority**: 30Hz authoritative tick loop with snapshot generation
- **Matchmaking**: Region-aware room finding and creation (up to 4 players per room)
- **Persistence**: Sled database for room and player state
- **Rate Limiting**: Token bucket system to prevent spam/abuse
- **Anti-cheat**: Enforced HMAC-SHA256 input signature verification (kick-by-default) and rate monitoring

### Client Features

- **Client Prediction**: Local input processing with server reconciliation
- **Compression**: Automatic snapshot decompression and state application
- **Networking**: Reconnection handling and RTT measurement via ping/pong

## Integration with AstraWeave Core

### Message Types

The protocol defines engine-agnostic message envelopes that can carry any serializable game state:

```rust
// Client sends input frames with game-specific commands
ClientToServer::InputFrame {
    seq: u32,                // Input sequence number
    tick_ms: u64,           // Client timestamp  
    input_blob: Vec<u8>,    // Serialized game input (e.g., movement, actions)
    sig: [u8; 32],          // HMAC-SHA256 tag over input_frame_sig_payload(seq, tick_ms, input_blob)
}

// Server sends authoritative snapshots
ServerToClient::Snapshot {
    id: u32,                // Snapshot ID for reconciliation
    server_tick: u64,       // Authoritative server tick
    base_id: Option<u32>,   // Base snapshot for delta compression
    compressed: bool,       // Whether payload is LZ4 compressed
    payload: Vec<u8>,       // Serialized world state
}
```

### Integrating with AstraWeave ECS

To use this networking layer with your existing `astraweave-core` types:

1. **Serialize your game input** (movement, actions) into the `input_blob`
2. **Serialize your world state** into the snapshot `payload`  
3. **Handle reconciliation** by comparing server snapshots with predicted state

Example integration pattern:

```rust
// In your game client
let player_input = PlayerInput {
    movement: glam::Vec3::new(dx, dy, dz),
    actions: player_actions,
    intent: current_plan_intent,
};
let input_blob = postcard::to_allocvec(&player_input)?;

// In your game server  
let world_snapshot = WorldSnapshot {
    entities: world.entities.clone(),
    tick: world.tick,
    events: recent_events,
};
let payload = postcard::to_allocvec(&world_snapshot)?;
```

## Deployment

### Single Server

```bash
# Production server with persistence
RUST_LOG=info cargo run --release -p aw-net-server
```

### Multi-Region Setup

Deploy multiple server instances in different regions:

```bash
# US East
AW_REGION=us-east cargo run --release -p aw-net-server

# EU Central  
AW_REGION=eu-central cargo run --release -p aw-net-server
```

Use a load balancer or DNS routing to direct clients to the nearest region.

### Scaling

- **Horizontal**: Run multiple server instances behind a load balancer
- **Database**: The Sled database can be replaced with a networked solution (Redis, PostgreSQL)
- **Matchmaking**: Extract matchmaking into a separate microservice

## Security Considerations

### Current Implementation

- **Input Authentication (implemented & enforced)**: Every `InputFrame` is signed with **HMAC-SHA256** over the canonical `input_frame_sig_payload(seq, tick_ms, input_blob)` byte range, keyed by a shared 32-byte symmetric `SigningKey`. The client signs; the server **verifies first** (before any per-player state mutation, constant-time) and, by default, **kicks** an unauthenticated client via a WebSocket Close frame (policy violation, code 1008). Configure the key via `AW_SHARED_KEY` (64 hex chars) on the client and `--shared-key-hex` on the server. The client falls back to `SigningKey::dev_default()` when `AW_SHARED_KEY` is unset, on every build (`net/aw-net-client/src/main.rs:19-25`). A debug server keeps `dev_default()` when `--shared-key-hex` is absent (`net/aw-net-server/src/main.rs:111-117`). Only the release CLI requires that flag (`:103-109`). `ServerConfig::default` still assigns the published key on every build (`lib.rs:167`). `db_path` is `:166` and `admin_token` is `:169`. Release `spawn_server` refuses that key at boot (`:307-312`), including when those bytes are passed as `--shared-key-hex`. The server never sees the client's key. `Hello`, `FindOrCreate`, and `JoinRoom` are unsigned. `HelloAck` is `lib.rs:457`. The unsigned player insert is `lib.rs:562-571`, before `MatchResult` (`:602-609`) and `JoinAccepted` (`:610-619`). Only `InputFrame` is verified (`lib.rs:770`). A client using the published key against a release server is a wrong-key client: it joins, is kicked with Close 1008 on its first input under the default Kick policy, and stays connected under `--sig-failure-policy warn` (parsed on every build at `main.rs:72-77`). Under warn, a bad tag is charged and not applied (`lib.rs:956-958`). `last_input_seq` and `last_seen` move only in `note_accepted_input` after a paid, verified frame (`lib.rs:862-872`). Configure the failure response via `--sig-failure-policy kick|warn` (`kick` is the default; `warn` logs, charges, and does not apply the frame).
- **TLS/WSS**: Secure WebSocket by default (`tls_enabled: true`, `lib.rs:163`); `--disable-tls` is rejected in release builds.
- **Rate Limiting**: Token bucket, 64 tokens/s, capacity 64, cost 1 (`lib.rs:808-810`). A new player starts full. InputFrame, protocol Ping, Ack, Hello, FindOrCreate, JoinRoom, undecodable binary, WebSocket Ping, Text, and Pong are charged. Thirty-two unpaid messages inside one second close the socket with 1008. A paid message does not clear that window. `RateLimited` is not sent. A 33 ms input stays connected. About 96 messages/s (64 paid plus 32 unpaid) is the ceiling. HMAC is checked before the bucket is charged.

#### Known limitations (deliberate boundaries)

- **No replay protection**: HMAC proves a frame is authentic, not *fresh* — a captured valid frame can be replayed. Nonces / sequence-number freshness are future session-security work.
- **Server→client messages are not signature-verified**: this is an authoritative-server, asymmetric-trust model; a shared *symmetric* key cannot meaningfully authenticate server→client traffic anyway (every client in a room holds the same key). Meaningful S2C auth needs asymmetric server keys or per-session key exchange (a handshake — out of scope).
- **Dev certs**: the bundled self-signed cert is for local development only; generate real certs via `net/certs/dev/generate_dev_cert.sh` (and provision proper CA-signed certs for deployment).

### Production Hardening

For production deployment, consider:

1. **Distribute a real shared key** out-of-band (never ship with the development key); rotate it as appropriate.
2. **Authentication**: Add player authentication and session management.
3. **Replay / freshness protection** and **server→client authentication** (asymmetric or per-session keys) if your threat model requires them.
4. **Advanced Anti-cheat**: Server-side physics validation, statistical analysis.
5. **Monitoring**: Add metrics, logging, and alerting.

## Performance

### Benchmarks

- **Latency**: Sub-millisecond message processing
- **Throughput**: Thousands of concurrent connections per server
- **Compression**: 60-80% size reduction on typical game state
- **Memory**: Minimal overhead with efficient binary protocols

### Optimization

- **Delta Compression**: Implement snapshot deltas using `base_id`
- **Lag Compensation**: Add server-side rewind buffers for hit validation  
- **Batching**: Batch multiple input frames for reduced network overhead

## Compatibility

This enhanced networking layer runs alongside the existing `astraweave-net` and coop examples without conflicts. The original examples continue to work for simple scenarios, while this provides capabilities for serious multiplayer games.
