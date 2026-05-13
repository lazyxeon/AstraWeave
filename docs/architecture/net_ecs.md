# Architecture Trace: Net-ECS (ECS Plugin + Standalone Matchmaking Server)

> **Scope note:** This doc traces the **second** of three networking subsystems in AstraWeave: the ECS-Plugin layer (`astraweave-net-ecs`) plus the production-style standalone-binary trio (`net/aw-net-proto`, `net/aw-net-client`, `net/aw-net-server`) and their runtime artifact directory (`aw_net_server_db/`). Other networking subsystems:
> - `docs/architecture/net.md` — `astraweave-net` (snapshot-based game server)
> - `docs/architecture/persistence_ecs.md` — `astraweave-persistence-ecs` (save/load)

## Metadata

| Field | Value |
|---|---|
| **System name** | Net-ECS + standalone matchmaking server |
| **Primary crates** | `astraweave-net-ecs` (ECS Plugin layer); `net/aw-net-proto` (wire protocol); `net/aw-net-client` (standalone CLI client binary); `net/aw-net-server` (standalone CLI server binary with matchmaking, persistence, anti-cheat); `aw_net_server_db/` (sled runtime artifacts — not code) |
| **Document version** | 1.2 |
| **Last verified against commit** | `a2474c5b7` |
| **Last verified date** | 2026-05-12 |
| **Status** | Active (mixed: production-grade standalone binary; dormant ECS Plugin layer) |
| **Revision history** | 1.2 (2026-05-12): Deep investigation pass. Enriched §11 Open Questions 1, 4, 8, 9 with comprehensive factual context. Recovered creation commit for `astraweave-net-ecs` dep in `astraweave-stress-test` (commit `08befc6ec` — same as net-ecs's own birth commit). Verified Q8 dual-TLS-version situation factually via `Cargo.lock` (both `tokio-rustls 0.25.0` and `tokio-rustls 0.26.4` are present) and via `cargo tree` (0.26 comes from reqwest→hyper-rustls used by many crates including `astraweave-llm`, `astraweave-assets`; 0.25 is exclusive to `aw-net-server`). Q4 factual finding: empty `entity_states: HashMap::new()` was present from the crate's birth commit `08befc6ec` (2025-10-01) — stub from day one. Resolved the last `[INFERRED]` marker (§5 TLS_IMPLEMENTATION_SUMMARY.txt content). Added new §6 row: a third copy of `lib.rs`-like content exists at `archive/temp_files/temp/temp_lib.rs` (a `use aw_net_proto` archive).<br><br>1.1 (2026-05-12): Verification pass. Corrected §2 Stage 6 HMAC mechanism — `hmac::verify_slice` (via `digest-0.10.7/src/mac.rs:168-179`) strict-rejects any tag length ≠ `OutputSize`; the previous `[INFERRED]` "truncated comparison" claim was wrong. The real mechanism: `Sha256::OutputSize == 32`, sig is 16 bytes, so `verify_slice` returns `MacError` immediately on length mismatch — **all client signatures fail length validation before any byte comparison happens**. Resolved §7 Decision 5 Date marker — HMAC verification landed in commit `88434f3a2` (2025-11-18, "security: fix critical vulnerabilities in network server (Priority 1)"). Resolved §7 Decision 7 Date marker — TLS-by-default in release added in commit `4889a9a33` (2025-11-13, "feat: Integrate astraweave-security…"). Noted that `parking_lot::Mutex` (not `std::sync::Mutex`) is used in §3 and §9. |
| **Owner notes** | Two distinct integration paths share `aw-net-proto`: (1) the standalone `aw-net-server` / `aw-net-client` binaries communicate end-to-end over WSS with HMAC-SHA256 input signatures, token-bucket rate limiting, sled persistence, and matchmaking. (2) `astraweave-net-ecs` provides ECS Plugin scaffolding (`NetworkClientPlugin`, `NetworkServerPlugin`, components, simulation-stub systems) — but **workspace grep finds zero `use astraweave_net_ecs` outside the crate's own tests and benches**, including in `astraweave-stress-test` which declares the dep in `Cargo.toml`. The ECS layer is currently dormant code. None of these crates appear in any `.github/workflows/*.yml` as of `a2474c5b7`. |

---

## 1. Executive Summary

**What this system does:**
Provides two coexisting integration paths over a shared binary wire protocol (`aw-net-proto`):

1. **Standalone binary trio** (`net/aw-net-{proto,client,server}`): A production-style multiplayer server with axum HTTP admin endpoints, region-aware matchmaking, room-based session management, sled-backed persistence, token-bucket rate limiting, HMAC-SHA256 signature verification, and TLS-by-default WebSocket. Demo client connects with `wss://`, joins a room via `FindOrCreate`, then streams `InputFrame` messages at 30 ms intervals.

2. **ECS Plugin layer** (`astraweave-net-ecs`): A library that registers `NetworkClientPlugin` / `NetworkServerPlugin` with the `astraweave-ecs::App` and adds four systems (`client_input_system`, `client_reconciliation_system`, `server_snapshot_system`, `server_input_processing_system`) plus three components (`CNetworkClient`, `CClientPrediction`, `CNetworkAuthority`) and async helpers (`connect_to_server`, `start_network_server`). The systems contain simulation stubs (e.g., literal `prediction.predicted_position.x += 0.1` placeholder for prediction).

**Why it exists:**
Per `net/README.md:1-12`, the goal was a "production-ready multiplayer capabilities" path layered on top of (not replacing) the older `astraweave-net` snapshot system. The standalone trio is the user-facing artifact; `astraweave-net-ecs` is the ECS adaptation layer intended to plug the protocol into a game's ECS world.

**Where it primarily lives:**
- `astraweave-net-ecs/src/lib.rs` — 437 lines. ECS Plugin + components + four systems + two async helpers + 4 inline tests.
- `astraweave-net-ecs/src/lib_temp.rs` — 436 lines. Near-duplicate of `lib.rs` (see §6).
- `net/aw-net-proto/src/lib.rs` — 174 lines. `ClientToServer` / `ServerToClient` enums, `Codec` (PostcardLz4 / Bincode), `encode_msg` / `decode_msg`, `sign16` legacy XOR signature, `new_room_id`, `SessionKey`, `PROTOCOL_VERSION = 1`, `WireError`.
- `net/aw-net-server/src/main.rs` — 852 lines. Standalone binary: TLS+plain dual paths, matchmaking, rooms, sled DB, axum HTTP admin, HMAC-SHA256 verification, token-bucket rate limiting.
- `net/aw-net-client/src/main.rs` — 165 lines. Standalone CLI client demo.
- `aw_net_server_db/` — Runtime artifact directory containing sled DB files (`conf` 62 B + `db` 96 B + empty `blobs/`). Not code; not git-ignored.

**Status note:**
The standalone server/client binaries are functional and tested by hand (`net/README.md`). The ECS Plugin layer (`astraweave-net-ecs`) has working tests but **no production consumer**: the only Cargo-declared dependency (`astraweave-stress-test/Cargo.toml:20`) does not actually import the crate in any source file. Treat the ECS Plugin layer as scaffolding awaiting wiring.

---

## 2. Authoritative Pipeline

### Path A: Standalone client → server (the production-facing path)

```text
[net/aw-net-client/src/main.rs — standalone binary]
    │
    │ tokio_tungstenite::connect_async("wss://127.0.0.1:8788")            (client/main.rs:18)
    │ Default: wss:// (TLS via native-tls feature; client/Cargo.toml:9)
    │
    ▼
[WebSocket handshake]
    │
    │ ClientToServer::Hello { protocol: PROTOCOL_VERSION (= 1) }          (client/main.rs:23-29)
    │   ↓ encode_msg(Codec::PostcardLz4, &msg)                            (proto/lib.rs:116-128)
    │   ↓ Message::Binary(bytes)
    │
    ▼
[net/aw-net-server/src/main.rs — handle_socket_tls or handle_socket]    (server/main.rs:261, 444)
    │
    │ recv_tls::<ClientToServer> or recv::<ClientToServer>
    │ Branches on hello.protocol:
    │   == 1   → ServerToClient::HelloAck { protocol }                    (server/main.rs:271, 452)
    │   != 1   → ServerToClient::ProtocolError { msg }; return            (server/main.rs:273-282, 454-463)
    │
    ▼
[Matchmaking phase]
    │
    │ ClientToServer::FindOrCreate { region, game_mode, party_size: 1 }   (client/main.rs:32-40)
    │
    ▼
[Server room logic]                                                       (server/main.rs:305-339, 485-520)
    │
    │ AppState.rooms (Arc<Mutex<HashMap<RoomId, Room>>>)
    │ Find existing room where region+game_mode match AND players.len() < 4
    │   ↓ found  → reuse room_id
    │   ↓ none   → create new Room { id: new_room_id() (8-char alphanumeric),
    │                                  region, game_mode,
    │                                  session_key: SessionKey::random() (32 bytes),
    │                                  tick_hz: 30,
    │                                  players: HashMap::new(),
    │                                  tick: 0, snap_id: 0 }
    │
    ├── ServerToClient::MatchResult { room_id, session_key_hint: first 8 bytes of key }
    └── ServerToClient::JoinAccepted { room_id, player_id: Uuid::v4(), session_key_hint, tick_hz }

[Per-connection game loop]                                                (server/main.rs:401-429, 582-610)
    │
    │ tokio::select! biased:
    │
    ├── ws.next() → ClientToServer::InputFrame { seq, tick_ms, input_blob, sig }
    │     ↓ on_client_msg_tls / on_client_msg                             (server/main.rs:659-724, 785-)
    │     ↓ Token-bucket rate limit: 8 tokens/sec refill, 60-bucket, 1 cost/msg
    │       · if tokens < 0.0 → kick = true → ServerToClient::RateLimited
    │     ↓ HMAC-SHA256(session_key.0, input_blob) compared to sig         (server/main.rs:703-709, 831-837)
    │       · verify failure → warn!() — connection NOT terminated
    │
    ├── ws.next() → ClientToServer::Ping { nano }
    │     ↓ ServerToClient::Pong { nano }
    │
    ├── ws.next() → ClientToServer::Ack { .. }       (ignored / commented)
    │
    └── sleep(tick_dt = 1000/tick_hz ms) → build_snapshot(&app, &rid)      (server/main.rs:625-657)
          ↓ room.tick += 1; room.snap_id += 1
          ↓ Encode DemoState { tick: server_tick } as postcard
          ↓ Wrap with lz4_flex::compress_prepend_size
          ↓ ServerToClient::Snapshot { id, server_tick, base_id: None, compressed: true, payload }

[On disconnect / loop exit]
    │
    │ Remove player from room
    │ If room.players.is_empty() → remove room from AppState.rooms        (server/main.rs:431-440, 613-621)
```

### Path B: ECS Plugin systems (dormant)

```text
[Caller: NetworkClientPlugin::new(addr).build(&mut App)]                  (net-ecs/lib.rs:55-67)
    │
    ├── app.add_system("simulation", client_input_system)                 (net-ecs/lib.rs:64)
    └── app.add_system("presentation", client_reconciliation_system)      (net-ecs/lib.rs:65)

[Caller: NetworkServerPlugin::new(addr).build(&mut App)]                  (net-ecs/lib.rs:75-87)
    │
    ├── app.add_system("simulation", server_snapshot_system)              (net-ecs/lib.rs:84)
    └── app.add_system("simulation", server_input_processing_system)      (net-ecs/lib.rs:85)

──────────────────────────────────────────────────────────────────────
[client_input_system(world)]                                              (net-ecs/lib.rs:90-118)
    │
    │ Query<CNetworkClient> + world.get::<CClientPrediction>(entity)
    │
    ▼
[Per-(client, prediction) tuple]
    │
    ├── input_sequence = last_acknowledged_input + pending_inputs.len() + 1
    ├── pending_inputs.push(input_sequence)
    ├── prediction.predicted_position.x += 0.1     ← literal stub (net-ecs/lib.rs:112)
    └── world.insert(entity, updated client + prediction)

──────────────────────────────────────────────────────────────────────
[client_reconciliation_system(world)]                                     (net-ecs/lib.rs:121-154)
    │
    ▼
[Per-(client, prediction) tuple]
    │
    │ Simulated server snapshot { server_tick: last_acknowledged_input + 1, entity_states: empty }
    │
    ├── prediction.prediction_error = predicted_position - Vec3::ZERO   ← literal stub
    ├── client.pending_inputs.retain(|i| *i > server_snapshot.server_tick)
    └── client.last_acknowledged_input = server_snapshot.server_tick

──────────────────────────────────────────────────────────────────────
[server_snapshot_system(world)]                                           (net-ecs/lib.rs:157-193)
    │
    │ Query<CNetworkAuthority>
    │
    ▼
[Per-authority]
    │
    ├── authority.authoritative_tick += 1
    ├── snapshot = NetworkSnapshot { server_tick: authority.authoritative_tick, entity_states: empty }
    ├── For each (player_id, mpsc::UnboundedSender<ServerToClient>) in connected_clients:
    │     ↓ payload = aw_net_proto::encode_msg(Codec::Bincode, &snapshot)
    │     ↓ sender.send(ServerToClient::Snapshot { id, server_tick, base_id: None, compressed: false, payload })
    │     ↓ Result is discarded with `let _ =`                            (net-ecs/lib.rs:187)
    └── world.insert(entity, updated authority)

──────────────────────────────────────────────────────────────────────
[server_input_processing_system(world)]                                   (net-ecs/lib.rs:196-214)
    │
    │ Query2<CNetworkClient, CNetworkAuthority>
    │ TODO comment: "Apply input validation, anti-cheat checks, etc."     (net-ecs/lib.rs:209)
    └── world.insert(entity, client)    ← no-op (client cloned, written back unchanged)

──────────────────────────────────────────────────────────────────────
[Async helpers — never called by the ECS systems]
    │
    ├── connect_to_server(addr) → mpsc::UnboundedReceiver<ServerToClient> (net-ecs/lib.rs:217-249)
    │     · connect_async(ws://) (NOT wss://)
    │     · Spawn task reading Message::Binary frames
    │     · decode_msg::<ServerToClient>(Codec::Bincode, ...)             ← note Bincode, not PostcardLz4
    │     · Forward via mpsc::UnboundedSender
    │
    └── start_network_server(bind_addr) → mpsc::UnboundedReceiver<ClientToServer>  (net-ecs/lib.rs:252-294)
          · TcpListener::bind + accept_async (plain TCP, no TLS)
          · Same Bincode codec
          · No matchmaking, no room logic, no auth
```

### Stage-by-stage detail

#### Stage 1: Protocol layer (`aw-net-proto/src/lib.rs`)
**Role:** Define the wire enums (`ClientToServer`, `ServerToClient`) and the two encoder/decoder paths (`Codec::PostcardLz4` and `Codec::Bincode`).
**Inputs:** Serializable structs implementing `serde::Serialize` / `Deserialize`.
**Outputs:** `Vec<u8>` (encode) or `Result<T, WireError>` (decode).
**Notes:**
- `PROTOCOL_VERSION` is `u16 = 1` (proto/lib.rs:6).
- `Codec` is `#[non_exhaustive]` (proto/lib.rs:108). PostcardLz4 is the recommended path (smaller payloads); Bincode is "Fallback / compatibility" per the doc comment at proto/lib.rs:113.
- `sign16(input, session_key_hint: [u8; 8]) -> [u8; 16]` is described as "Minimal tamper-evident signature (MVP): xor 16 bytes of input hash with key hint" (proto/lib.rs:151). **Note:** the standalone server uses HMAC-SHA256 (not `sign16`) — see §6 conflict map.
- `new_room_id()` returns 8 alphanumeric characters (proto/lib.rs:168-174).
- `SessionKey(pub [u8; 32])` (proto/lib.rs:11). Generated via `SessionKey::random()` using `rand::rng().fill(&mut bytes)` (proto/lib.rs:14-18).

#### Stage 2: Standalone server entry (`aw-net-server/src/main.rs:99-258`)
**Role:** Parse CLI args (`--disable-tls`, `--tls-cert`, `--tls-key`), open sled DB, spawn the axum HTTP admin server on port 8789, then loop-accept WebSocket connections on port 8788.
**Inputs:** CLI args; PEM cert/key files (default `net/certs/dev/dev-cert.pem` and `dev-key.pem`).
**Outputs:** Long-running tokio tasks; tracing logs.
**Notes:**
- `--disable-tls` is `cfg(not(debug_assertions))`-rejected (server/main.rs:120-123) — release builds cannot disable TLS.
- HTTP admin on 0.0.0.0:8789: `/healthz` returns `"ok"`, `/regions` returns hardcoded `["us-east","us-west","eu-central"]` (server/main.rs:160-164). No real region-resolution logic; the string list is decorative.
- sled DB opened at `aw_net_server_db/` (server/main.rs:149) but **never read or written** in this file's surface (`grep -n "app.db" server/main.rs` returns nothing). The `db` field on `AppState` is `#[allow(dead_code)]` (server/main.rs:62-63).

#### Stage 3: TLS vs plain handler split (`server/main.rs:261-624`)
**Role:** Per-connection handshake, matchmaking, per-tick snapshot loop.
**Inputs:** A `WebSocketStream<...>` (TLS-wrapped or plain).
**Outputs:** Mutations to `AppState.rooms`; outbound `ServerToClient` messages.
**Notes:** `handle_socket_tls` (server/main.rs:261-442, 182 lines) and `handle_socket` (server/main.rs:444-623, 180 lines) are **near-identical** — same matchmaking logic, same per-tick loop, same cleanup, differing only in the `WebSocketStream` type parameter (TLS vs plain). The helper functions `send`/`send_tls`, `recv`/`recv_tls`, `on_client_msg`/`on_client_msg_tls` form three more duplicated pairs (server/main.rs:727-852). Total duplication: roughly 400 lines.

#### Stage 4: Matchmaking + room allocation (`server/main.rs:297-396, 478-577`)
**Role:** On first non-Hello message, either find an existing room or create a new one; allocate the player into it; send `MatchResult` and `JoinAccepted`.
**Inputs:** `ClientToServer::FindOrCreate { region, game_mode, party_size }` or `ClientToServer::JoinRoom { room_id, display_name }`.
**Outputs:** Updated `AppState.rooms`; `MatchResult` + `JoinAccepted` over WS.
**Notes:**
- Room-find filter: `r.region == region && r.game_mode == game_mode && r.players.len() < 4` (server/main.rs:309-311, 490-492). Cap of 4 players per room is hardcoded.
- `party_size` field on `FindOrCreate` is destructured but ignored (server/main.rs:306-308, 487-489: `region, game_mode, ..`).
- `display_name` on `JoinRoom` is destructured but ignored (server/main.rs:333, 514: `display_name: _`).
- `session_key_hint` is the **first 8 bytes** of the 32-byte `SessionKey` (server/main.rs:361, 542): `session_hint.copy_from_slice(&room.session_key.0[0..8])`. The full session key never leaves the server.
- `tick_hz` is currently hardcoded to 30 in `Room::new`-equivalent literal (server/main.rs:321, 502: `tick_hz` field, the binding starts at 30u32 on line 301/482 before being overwritten by `room.tick_hz` on line 373/554 — but the literal 30 is the seed).
- New `Room.tick` starts at 0 and `snap_id` starts at 0 (server/main.rs:323-324, 504-505).

#### Stage 5: Per-tick snapshot construction (`server/main.rs:625-657`)
**Role:** Build a `ServerToClient::Snapshot` for one room.
**Inputs:** `&AppState`, `rid: &str` (room id).
**Outputs:** `(ServerToClient::Snapshot, snap_id)`.
**Notes:**
- The payload is a `DemoState { tick }` struct serialized with postcard (server/main.rs:638-644). **This is a placeholder** — `net/README.md` documents how a real consumer would substitute its own snapshot type, but the current code only ships the tick.
- The snapshot is `compressed: true` and always wrapped with `lz4_flex::compress_prepend_size` (server/main.rs:653-654).
- `room.snap_id` increments with `wrapping_add(1)` (server/main.rs:632). At `u32::MAX` the snap ID wraps to 0.
- The snap is built once per `tick_dt` per connection inside the per-connection `tokio::select!` (server/main.rs:423-427, 604-608). With N connections sharing a room, each connection independently increments the room's `tick` — see invariant 9.

#### Stage 6: HMAC-SHA256 input verification (`server/main.rs:703-709, 831-837`)
**Role:** Verify the `InputFrame.sig` against an HMAC-SHA256(`session_key.0`, `input_blob`).
**Inputs:** `InputFrame.input_blob`, `InputFrame.sig: [u8; 16]`, `room.session_key.0`.
**Outputs:** A `warn!` log on verification failure.
**Notes:**
- Uses `hmac::Hmac<Sha256>` (server/Cargo.toml:29-30).
- HMAC-SHA256 produces 32 bytes, but `mac.verify_slice(&sig)` is called with a 16-byte sig. **Verified via vendored `digest-0.10.7/src/mac.rs:168-179`:** `verify_slice` strict-rejects any tag whose length is not equal to `Self::OutputSize::USIZE` — for HMAC-SHA256 that is 32 bytes. The 16-byte `sig` therefore fails the length check at line 170 and returns `Err(MacError)` immediately, **before any byte comparison happens**. The HMAC-vs-XOR algorithmic mismatch is moot: length mismatch alone guarantees failure. (The `digest` crate exposes `verify_truncated_left`/`verify_truncated_right` for truncated-tag comparison, but the server does not use them.)
- **The verification result does not gate processing.** A failed HMAC check produces a `warn!("HMAC signature verification failed for pid={pid}")` but the player's `last_input_seq` and `last_seen` are still updated above (server/main.rs:699-700, 827-828). This is the docs-vs-code drift noted in §6: `net/README.md` claims HMAC verification is "anti-cheat" but the verification result is observed-only.
- The client side uses `aw_net_proto::sign16(input_blob, session_key_hint)` (client/main.rs:86) — which is the **XOR-based 16-byte signature** function, not an HMAC. Even if the server switched to `verify_truncated_left` to accept 16-byte tags, the algorithmic mismatch would still cause the byte comparison to fail (XOR-hash bytes ≠ HMAC-SHA256[0..16]).

#### Stage 7: Token-bucket rate limiting (`server/main.rs:679-700, 807-828`)
**Role:** Limit per-player input message rate.
**Inputs:** `Player.tokens`, `Player.last_refill`, message arrival.
**Outputs:** Updated bucket state; `kick = true` if drained.
**Notes:**
- Constants: `REFILL_RATE = 8.0` tokens/sec, `BUCKET_SIZE = 60.0`, `COST_PER_MESSAGE = 1.0` (server/main.rs:683-685, 811-813).
- New players start with `tokens: 30.0` (server/main.rs:369, 550).
- Steady-state input rate at 30 ticks/sec ≈ 1 token/msg * 30/sec = drains the bucket. So the 8 tokens/sec refill is **insufficient** to sustain the documented 30 Hz client tick. [INFERRED — based on arithmetic; no test exercises sustained-rate behavior]. See §11 for the open question.
- When `tokens < 0.0`, `kick = true` → server sends `ServerToClient::RateLimited` (server/main.rs:713-715, 841-843). The connection is **not** closed; the client may choose to back off or disconnect itself.

#### Stage 8: ECS Plugin layer registration (`net-ecs/lib.rs:55-87`)
**Role:** Provide a Plugin-shaped API for the ECS app to opt into network systems.
**Inputs:** `App`.
**Outputs:** Two systems added per Plugin.
**Notes:**
- `NetworkClientPlugin::server_addr` and `NetworkServerPlugin::bind_addr` are both `#[allow(dead_code)]` (net-ecs/lib.rs:50, 70) — the addresses are stored on the plugin struct but **never read** by `build()` or the systems. The async helpers (`connect_to_server`, `start_network_server`) take their addresses as parameters and are not called from any system.
- The four registered systems (`client_input_system`, `client_reconciliation_system`, `server_snapshot_system`, `server_input_processing_system`) do not call `connect_to_server` or `start_network_server` — they operate on local ECS components only, with simulation-stub logic (literal `predicted_position.x += 0.1`, hardcoded `entity_states: HashMap::new()`).

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **PROTOCOL_VERSION** | `u16 = 1` — wire-format version check on `Hello`. | `aw-net-proto/lib.rs:6` |
| **ClientToServer** | `#[non_exhaustive]` enum of 6 wire messages: `Hello`, `FindOrCreate`, `JoinRoom`, `InputFrame`, `Ping`, `Ack`. | `aw-net-proto/lib.rs:22-56` |
| **ServerToClient** | `#[non_exhaustive]` enum of 8 wire messages: `HelloAck`, `MatchResult`, `JoinAccepted`, `Snapshot`, `Reconcile`, `Pong`, `RateLimited`, `ProtocolError`. | `aw-net-proto/lib.rs:58-95` |
| **Codec** | `#[non_exhaustive]` enum: `PostcardLz4` (default), `Bincode` (compat). | `aw-net-proto/lib.rs:107-114` |
| **SessionKey** | `[u8; 32]` per-room secret. `SessionKey::random()` fills via `rand::rng().fill`. | `aw-net-proto/lib.rs:10-19` |
| **session_key_hint** | `[u8; 8]` — first 8 bytes of `SessionKey.0` sent to the client; the client uses this with `sign16` to build a 16-byte XOR signature on each `InputFrame`. | `aw-net-server/main.rs:361, 542` |
| **sign16** | Legacy XOR-based 16-byte signature: hashes input with `DefaultHasher`, splits the u64 hash into 8 + 8 bytes, then XORs both halves against `session_key_hint`. | `aw-net-proto/lib.rs:151-165`; called only by `aw-net-client/main.rs:86`. |
| **Room** | Server-side state: `{ id, region, game_mode, session_key, tick_hz, players: HashMap<PlayerId, Player>, tick, snap_id }`. | `aw-net-server/main.rs:43-57` |
| **Player** | Server-side per-player state: `{ id, display, last_input_seq, last_seen, tokens, last_refill }`. | `aw-net-server/main.rs:30-41` |
| **AppState** | Server-wide state: `{ rooms: Arc<parking_lot::Mutex<HashMap<RoomId, Room>>>, db: sled::Db, codec: Codec }`. Uses `parking_lot::Mutex` (`server/main.rs:10`), not `std::sync::Mutex`. | `aw-net-server/main.rs:58-66` |
| **WireError** | `#[non_exhaustive]` + `#[must_use]` decode error enum: `ProtocolMismatch`, `Decode`. | `aw-net-proto/lib.rs:97-105` |
| **CNetworkClient** | ECS component: `{ player_id, last_acknowledged_input, pending_inputs }`. | `astraweave-net-ecs/lib.rs:13-19` |
| **CClientPrediction** | ECS component: `{ predicted_position: Vec3, prediction_error: Vec3 }`. | `astraweave-net-ecs/lib.rs:21-26` |
| **CNetworkAuthority** | ECS component: `{ authoritative_tick, connected_clients: HashMap<String, mpsc::UnboundedSender<ServerToClient>> }`. | `astraweave-net-ecs/lib.rs:28-33` |
| **NetworkSnapshot** | Distinct from `aw-net-proto`'s `ServerToClient::Snapshot`. ECS-layer snapshot type with `{ server_tick, entity_states: HashMap<u64, EntityState> }`. | `astraweave-net-ecs/lib.rs:35-40` |

### Terms to NOT confuse

- **`Snapshot` (aw-net-proto wire variant) vs. `NetworkSnapshot` (astraweave-net-ecs)**: The wire-format snapshot is `ServerToClient::Snapshot { id, server_tick, base_id, compressed, payload }` carrying an opaque byte payload. The ECS-layer `NetworkSnapshot` is what's *inside* the payload — it deserializes per-entity state. Distinct types in distinct crates.
- **`session_key` (32 bytes, server only) vs. `session_key_hint` (8 bytes, sent to client) vs. `[u8; 16]` sig**: Three different sizes. The full key never leaves the server. The 8-byte hint is what the client uses to compute `sign16`'s output. The 16-byte sig is what the client sends on each `InputFrame`.
- **`sign16` (proto, XOR-based) vs. `HmacSha256` (server, real HMAC)**: The client computes `sign16(input_blob, &session_hint)` (`client/main.rs:86`). The server verifies with `HmacSha256::new_from_slice(&room.session_key.0)`, `mac.update(&input_blob)`, `mac.verify_slice(&sig)` (`server/main.rs:703-707, 831-835`). **These do not produce the same bytes.** Every signature verification is therefore expected to fail under the current code; the server only `warn!`s but does not act on the failure.
- **`Codec::Bincode` (proto) vs. `Codec::PostcardLz4` (proto)**: Both are members of the same enum. `PostcardLz4` is used by both standalone binaries (client/main.rs:69, server/main.rs:153). `Bincode` is used by `astraweave-net-ecs` (lib.rs:179, 237, 279). The two integration paths thus serialize incompatibly even though they share the protocol crate.
- **`astraweave-net::Snapshot` (older subsystem) vs. anything in this subsystem**: Separate crate, different data model, JSON-text wire format. Documented in `docs/architecture/net.md`.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-ecs` (Plugin layer dep) | `App`, `Plugin`, `Query`, `Query2`, `World` (`net-ecs/lib.rs:7`) | Component query/insert/get; `app.add_system(stage, fn)`; `app.add_plugin(Plugin)` | The four ECS systems take `&mut World`. Plugin builders register them in `"simulation"` or `"presentation"` stages. |
| `astraweave-core` (workspace dep) | (declared in `net-ecs/Cargo.toml:16`) | none directly imported in `net-ecs/src/lib.rs` | The dep is declared but workspace grep shows no `use astraweave_core` in `astraweave-net-ecs/src/`. The dependency may exist for future use or has been left from a prior version. |
| `glam` (ECS data) | `Vec3` (in `CClientPrediction.predicted_position`, `EntityState.position`) | 3D position component | Used by the ECS Plugin layer only; the standalone binaries operate on opaque `Vec<u8>` blobs. |
| Filesystem (sled) | `sled::open("aw_net_server_db")` (`server/main.rs:149`) | KV store | `AppState.db` field is `#[allow(dead_code)]` — opened but unused. The directory exists at workspace root with a `conf` file (62 B) and `db` file (96 B) — sled's tiny housekeeping artifacts. |
| Filesystem (TLS certs) | `load_certs` / `load_private_key` (`server/main.rs:68-95`) | PEM-encoded cert chain + key | Defaults to `net/certs/dev/dev-cert.pem` and `dev-key.pem`. Generation script: `net/certs/dev/generate_dev_cert.sh` (referenced in error message at `server/main.rs:203`). |
| Environment | `AW_WS_URL`, `AW_REGION` (`client/main.rs:13-14`) | Server URL and region string | Client defaults: `wss://127.0.0.1:8788`, region `us-east`. |
| WebSocket client connections | `accept_hdr_async`, `accept_async` (`server/main.rs:225, 246, 264`) | Postcard-LZ4 binary frames | Server expects `Message::Binary` and silently drops `Message::Text` and other variants. |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-stress-test` (Cargo.toml:20: `astraweave-net-ecs = { workspace = true }`) | none in source | none | **The declared dependency is not used.** Workspace grep `use astraweave_net_ecs` against `astraweave-stress-test/` returns no source-file matches. See §6. |
| Standalone client binary | (none — terminal endpoint) | tracing logs | The client binary prints snapshot info via `tracing::info!`. |
| sled KV store | Unused write surface | none | `AppState.db` is opened but never queried or mutated in the visible source. |
| Future game integration | `NetworkSnapshot.entity_states` | Per-entity state | The data shape exists but no consumer reads it as of `a2474c5b7`. |

### Bidirectional / Coupled

- **`Room.session_key.0` ↔ `session_key_hint`:** The full key lives only on the server. The first 8 bytes are sent to the client as a `session_key_hint`. The client uses this hint with `sign16` to build a `[u8; 16]` signature on every `InputFrame`. The server verifies (or attempts to — see §6 / Stage 6) with HMAC-SHA256 over the full key, which produces a different 16-byte prefix.

### Documentation references with no code backing

- **None observed.** The `net/README.md` accurately describes the implemented modules. Discrepancies between docs and code are around signature scheme (README claims XOR, code uses HMAC verification with XOR-signed inputs) and security posture (claimed anti-cheat is `warn!`-only) — surfaced in §6, not §4.

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-net-ecs/src/lib.rs` | ECS Plugin layer: 3 components + 2 Plugins + 4 systems + 2 async helpers + 4 inline tests | Active (Plugin layer dormant — no production consumer) | 437 lines. `#![forbid(unsafe_code)]`. |
| `astraweave-net-ecs/src/lib_temp.rs` | Near-duplicate of `lib.rs` minus `#![forbid(unsafe_code)]` | Orphan (not declared in any `mod` statement; not the crate's lib entry; effectively dead source) | 436 lines. Created 2025-11-19 in commit `54d15c9f2` ("added forest biome textures and assets, worked on renderer"). The commit title is misleading — the file is unrelated to forest biomes. See §6. |
| `astraweave-net-ecs/tests/mutation_resistant_comprehensive_tests.rs` | Mutation-resistance harness | Active (tests) | 341 lines, 27 tests. |
| `astraweave-net-ecs/benches/net_ecs_benchmarks.rs` | Criterion benches for serialization, snapshot construction, component-store ops | Active | 487 lines, 9 `bench_function` call sites. |
| `astraweave-net-ecs/benches/net_ecs_adversarial.rs` | Adversarial benches (worst-case shapes, edge inputs) | Active | 921 lines, 11 `bench_function` call sites. |
| `net/aw-net-proto/src/lib.rs` | Wire-protocol types + `encode_msg`/`decode_msg`/`sign16`/`new_room_id` + `WireError` | Active | 174 lines. `#![forbid(unsafe_code)]`. |
| `net/aw-net-proto/tests/mutation_resistant_comprehensive_tests.rs` | Mutation-resistance harness | Active (tests) | 53 tests. |
| `net/aw-net-proto/benches/proto_bench.rs` | Criterion benches for codec encode/decode | Active | 13 `bench_function` call sites. |
| `net/aw-net-client/src/main.rs` | Standalone CLI client binary | Active | 165 lines. Demo input loop at 33 ms tick (~30 Hz). Uses `tokio-tungstenite` with `native-tls` feature for wss. |
| `net/aw-net-client/Cargo.toml` | Binary crate metadata | Active | Pulls `tokio-tungstenite = { version = "0.28", features = ["native-tls"] }` (line 9). |
| `net/aw-net-server/src/main.rs` | Standalone CLI server binary with TLS, matchmaking, sled, rate limiting, HMAC verification | Active | 852 lines. Dual TLS/plain handlers (Stage 3 — ~400 lines of duplicated code). |
| `net/aw-net-server/Cargo.toml` | Binary crate metadata | Active | Includes `tokio-rustls = "0.25"`, `rustls = "0.22"`, `sled = "0.34"`, `hmac = "0.12"`, `sha2 = "0.10"`, `axum = "0.8"`. **Note:** TLS stack version is one major behind the workspace `astraweave-net` crate which uses `tokio-rustls = "0.26"` and `rustls = "0.23"` (workspace-pin). See §6. |
| `net/aw-net-proto/Cargo.toml` | Library crate metadata | Active | Uses `bincode = { version = "2.0", features = ["serde"] }` and `postcard = { version = "1", features = ["alloc"] }`. |
| `net/README.md` | Integration guide | Active (with drift) | Production-ready usage instructions. Documents XOR signature scheme that is no longer used; documents `anti-cheat: input signature validation` which is `warn!`-only. See §6. |
| `net/TLS_IMPLEMENTATION_SUMMARY.txt`, `net/TLS_TESTING_GUIDE.txt` | TLS notes | Active | Standalone text files at `net/` root. `TLS_IMPLEMENTATION_SUMMARY.txt` is a structured implementation changelog documenting the cert-loading functions, CLI flags (`--tls-cert`, `--tls-key`, `--disable-tls`), and dual-handler design (`handle_socket_tls`/`handle_socket`). Verified by direct read 2026-05-12. The line numbers and decisions in this doc match the current code. |
| `net/certs/dev/` | Dev TLS certs + generation script | Active | Referenced at `server/main.rs:203` (error message points users here). |
| `aw_net_server_db/` | sled runtime artifact dir | Active (runtime) | Two checked-in files: `conf` (62 B), `db` (96 B), plus empty `blobs/`. Created on first server run; minimal because `app.db` is never written to. |

**Status definitions:**
- **Active**: Canonical, load-bearing, edit freely with care.
- **Active (tests)**: Carries no runtime weight but exercises invariants.
- **Active (Plugin layer dormant)**: Code compiles and tests pass, but no production code path imports it.
- **Active (with drift)**: Doc accurately covers some details and disagrees with current code on others.
- **Active (runtime)**: Artifact directory generated at runtime.
- **Orphan**: Source file exists but is not declared as a module and is not the crate's lib entry; effectively dead source.

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| `astraweave-net-ecs/src/lib_temp.rs` vs. `astraweave-net-ecs/src/lib.rs` | both files in same crate `src/` | Orphan duplicate | `lib_temp.rs` (436 lines) is a near-identical copy of `lib.rs` (437 lines) minus the `#![forbid(unsafe_code)]` attribute. `diff astraweave-net-ecs/src/lib.rs astraweave-net-ecs/src/lib_temp.rs` shows the only structural difference is the missing crate attribute on line 1. Created in commit `54d15c9f2` (2025-11-19); not declared as `mod lib_temp` anywhere, not the crate's lib entry, so it doesn't compile into the crate. |
| Third copy of net-ecs-shaped lib.rs in archive | `archive/temp_files/temp/temp_lib.rs` | Archived | An additional near-copy of `lib.rs`'s import/struct surface (`use aw_net_proto::{decode_msg, ClientToServer, Codec, ServerToClient}`, `CNetworkClient`, `CClientPrediction`, `CNetworkAuthority`) lives under `archive/temp_files/temp/`. Not part of the active crate; presumably checked in during a refactor archive sweep. Worth noting alongside `lib_temp.rs` as a forensic indicator that this code has been duplicated several times during its history. |
| `sign16` (XOR-based) vs. HMAC-SHA256 (in server) | `aw-net-proto/lib.rs:151-165` vs. `aw-net-server/main.rs:703-709, 831-837` | Active drift — client signs with `sign16`, server verifies with HMAC | Client computes `aw_net_proto::sign16(&blob, &session_hint)` (`aw-net-client/main.rs:86`). Server runs `HmacSha256::new_from_slice(&room.session_key.0)` + `.update(&input_blob)` + `.verify_slice(&sig)`. The two algorithms produce different bytes for the same input, so the server's verification always fails — but the failure is `warn!`-only (server doesn't kick or reject). The `net/README.md` describes the XOR scheme as "MVP" and lists HMAC as a "Production Hardening" upgrade — the server appears to have been upgraded while the client was not. |
| `Codec::PostcardLz4` (standalone binaries) vs. `Codec::Bincode` (ECS Plugin layer) | `aw-net-client/main.rs:69`, `aw-net-server/main.rs:153` vs. `astraweave-net-ecs/lib.rs:179, 237, 279` | Two parallel wire formats sharing one enum | The standalone trio uses PostcardLz4 end-to-end. The ECS Plugin layer's `connect_to_server` / `start_network_server` / `server_snapshot_system` all hardcode `Codec::Bincode`. If a future game tries to wire the Plugin layer to talk to the standalone server, codec mismatch will cause every decode to fail. |
| `tokio-rustls = "0.25"` + `rustls = "0.22"` (standalone server) vs. `tokio-rustls = "0.26"` + `rustls = "0.23"` (workspace `astraweave-net`) | `net/aw-net-server/Cargo.toml:26-27` vs. workspace `astraweave-net/Cargo.toml:24-25` | Two major versions of the same TLS stack in one workspace | The standalone server is one major behind. Workspace builds may pull both versions into the dep graph. |
| `astraweave-net-ecs::EntityState` vs. `astraweave-net::EntityState` (in the other networking subsystem) | `net-ecs/lib.rs:43-47` vs. `astraweave-net/src/lib.rs:38-44` | Same name, disjoint shapes | `net-ecs::EntityState { position: Vec3, health: i32 }` vs. `net::EntityState { id: u32, pos: IVec2, hp: i32, team: u8, ammo: i32 }`. Cross-imports require qualification. (Already noted in `docs/architecture/net.md`.) |
| `connect_to_server` / `start_network_server` (ECS layer) vs. `aw-net-server`'s `handle_socket` (standalone) | `astraweave-net-ecs/lib.rs:217-294` vs. `aw-net-server/main.rs:444-623` | Two independent WebSocket handlers | The ECS layer's `start_network_server` is a minimal `accept_async` loop with no matchmaking, no auth, no rate limiting — it just decodes `ClientToServer` frames into an mpsc channel. The standalone `aw-net-server` is the production-grade path. They share no code beyond the `aw-net-proto` types. |
| `handle_socket_tls` vs. `handle_socket` in the server | `aw-net-server/main.rs:261-442` vs. `444-623` | Active code duplication | Two 180-line functions differ only in their `WebSocketStream` type parameter. Same matchmaking, same per-tick loop, same cleanup. Helper functions `send_tls`/`send`, `recv_tls`/`recv`, `on_client_msg_tls`/`on_client_msg` are also duplicated. ~400 lines total. |
| `sled::Db` field on `AppState` is opened but unused | `aw-net-server/main.rs:62-63 (#[allow(dead_code)])`, `:149` (open) | Active (declared, no read/write call sites) | `grep -n "app.db\|app\.db\|state\.db" aw-net-server/src/main.rs` returns no read or write call sites after the `open` call. The sled directory at `aw_net_server_db/` exists but contains only sled's own initialization artifacts. |
| `NetworkClientPlugin::server_addr` and `NetworkServerPlugin::bind_addr` | `net-ecs/lib.rs:50-52, 70-72 (#[allow(dead_code)])` | Stored, never read | The plugin builders use neither field. The async helpers take the address as a separate parameter. The fields exist only as placeholders for a future wiring. |

### Naming collisions

- **`Snapshot`** appears in: `astraweave-net::Snapshot` (struct), `aw-net-proto::ServerToClient::Snapshot` (enum variant carrying opaque payload), `astraweave-net-ecs::NetworkSnapshot` (struct), `aw-net-server/main.rs::build_snapshot::DemoState` (local placeholder struct).
- **`EntityState`** appears in: `astraweave-net::EntityState`, `astraweave-net-ecs::EntityState`.
- **`Codec`** is unique to `aw-net-proto` but contains two variants (`PostcardLz4`, `Bincode`) — each used by different consumers as noted above.

### Known cognitive traps

- **Trap:** `aw-net-client` signs with `sign16` (XOR); `aw-net-server` verifies with HMAC-SHA256, and additionally the sig length is wrong.
  - **Why it's confusing:** Both sides reference `session_key`. The server has `hmac` + `sha2` in its `Cargo.toml`; the client doesn't.
  - **What's actually true:** Verification ALWAYS fails for **two independent reasons**, either of which is sufficient:
    1. **Length mismatch (immediate).** `digest::Mac::verify_slice` (`~/.cargo/registry/.../digest-0.10.7/src/mac.rs:168-179`) returns `Err(MacError)` whenever `tag.len() != OutputSize::USIZE`. The server passes a 16-byte sig to a `Hmac<Sha256>` that expects 32 bytes. The check at line 170 fails before any cryptographic comparison.
    2. **Algorithm mismatch (would matter if length matched).** Even if the server used `verify_truncated_left` (which accepts shorter tags), the XOR `sign16` output bytes do not match HMAC-SHA256's first 16 bytes for the same input.
  - The failure is `warn!`-only — no kick, no drop, no reject. The `net/README.md` lists "HMAC Signatures" under "Production Hardening" as a planned upgrade, but the server already implements HMAC verification — the client was never updated to match.

- **Trap:** `server_snapshot_system` in the ECS layer sends snapshots with **empty `entity_states` HashMap**.
  - **Why it's confusing:** The system runs, the message goes out — but it carries no actual entity data.
  - **What's actually true:** `astraweave-net-ecs/lib.rs:174` literally constructs `entity_states: HashMap::new()`. The comment "Would collect actual entity states" (line 174) admits the placeholder.

- **Trap:** `client_input_system` and `client_reconciliation_system` contain hardcoded simulation stubs.
  - **Why it's confusing:** They look like real systems and are wired into the Plugin.
  - **What's actually true:** `client_input_system` advances `prediction.predicted_position.x += 0.1` per tick (literal constant, no input handling). `client_reconciliation_system` computes `prediction_error = predicted_position - Vec3::ZERO` (always returns the un-reconciled position as error). These are scaffold systems, not implementations.

- **Trap:** `AppState.db: sled::Db` is opened but never used.
  - **Why it's confusing:** The README's "Persistence" section claims room and player state survive restarts.
  - **What's actually true:** `app.db` has no read/write call sites in `aw-net-server/main.rs`. Restarting the server loses all in-memory `rooms`. The sled directory exists with tiny `conf` (62 B) and `db` (96 B) housekeeping files only.

- **Trap:** Token bucket rate (8 tokens/sec refill) is slower than the documented client tick rate (30 Hz).
  - **Why it's confusing:** A client following the README's "30 Hz tick" guidance would burst the 60-token bucket in ~7.5 seconds and then be permanently rate-limited.
  - **What's actually true:** `REFILL_RATE = 8.0`, `COST_PER_MESSAGE = 1.0`, `BUCKET_SIZE = 60.0` (`aw-net-server/main.rs:683-685`). A 30 Hz client drains 30 tokens/sec and refills 8 — net drain 22/sec. The 60-token bucket buys ~2.7 seconds before `RateLimited` starts firing.

- **Trap:** `lib_temp.rs` exists in `astraweave-net-ecs/src/`.
  - **Why it's confusing:** Looks like a partial refactor or backup file. The commit title that introduced it (`54d15c9f2` "added forest biome textures and assets, worked on renderer") gives no clue about why a temp file was added to the netcode crate.
  - **What's actually true:** It's not declared as `mod lib_temp` anywhere; it is not the crate's library entry. Cargo only compiles `src/lib.rs`, so `lib_temp.rs` doesn't enter the build. Its tests don't run. It's effectively orphan source.

- **Trap:** `connect_to_server` (`net-ecs/lib.rs:217`) uses `ws://`, not `wss://`.
  - **Why it's confusing:** The standalone client (`aw-net-client/main.rs:13`) defaults to `wss://127.0.0.1:8788`. The ECS Plugin helper builds `format!("ws://{}", server_addr)`.
  - **What's actually true:** The two paths cannot interoperate without modification. The Plugin layer expects a plain-TCP server (which the standalone server only serves under `--disable-tls` in debug builds).

---

## 7. Decision Log

### Decision: Two integration paths (standalone trio + ECS Plugin layer) sharing one protocol crate
- **Date:** Standalone trio: 2025-09-09 commit `cc9a7e3e3` ("Implement production-ready enhanced networking layer with server authority, client prediction, and matchmaking (#58)"). ECS Plugin layer: 2025-10-01 commit `08befc6ec` ("phase 6 implementation").
- **Status:** Accepted (live in three workspace crates).
- **Context:** Per `net/README.md:90-92`, "This enhanced networking layer runs alongside the existing `astraweave-net` and coop examples without conflicts. The original examples continue to work for simple scenarios, while this provides production-grade capabilities for serious multiplayer games."
- **Decision:** Build a new networking layer instead of refactoring `astraweave-net`. Share types only through the new `aw-net-proto` crate.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Workspace now ships **two** networking subsystems with disjoint data models, disjoint wire formats, and overlapping concerns.
  - Cross-consumer of the new system: zero in production source code; standalone binaries are end-points.
  - The ECS Plugin layer was added a month later than the standalone binaries — it appears to be a separately-developed adaptation that has not been wired to a game.

### Decision: PostcardLz4 as default codec; Bincode for fallback
- **Date:** 2025-09-09 commit `cc9a7e3e3`.
- **Status:** Accepted (`aw-net-proto/lib.rs:107-114`).
- **Context:** The README mentions "60-80% size reduction on typical game state" from compression (`net/README.md:81`).
- **Decision:** PostcardLz4 (postcard serialize + lz4 compress) as the wire codec; Bincode as "Fallback / compatibility" (per doc comment at `aw-net-proto/lib.rs:113`).
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Standalone server and client both use PostcardLz4.
  - ECS Plugin layer hardcodes Bincode (`net-ecs/lib.rs:179, 237, 279`).
  - The two paths are wire-incompatible. Splitting the codec choice between the two consumers means the ECS Plugin layer cannot talk to the standalone server without modification.

### Decision: Matchmaking room cap of 4 players, hardcoded
- **Date:** 2025-09-09 commit `cc9a7e3e3`.
- **Status:** Accepted (`aw-net-server/main.rs:310, 491`).
- **Context:** Room-find predicate: `r.region == region && r.game_mode == game_mode && r.players.len() < 4`.
- **Decision:** Hard-code the 4-player room cap in two places (TLS + plain handlers).
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Changing party size requires editing both `handle_socket` and `handle_socket_tls`.
  - The `ClientToServer::FindOrCreate.party_size` field is destructured but ignored, so the client cannot request a larger room.

### Decision: 30 Hz server tick, hardcoded
- **Date:** 2025-09-09 commit `cc9a7e3e3`.
- **Status:** Accepted (`aw-net-server/main.rs:301, 482, 321, 502`).
- **Context:** `tick_hz = 30u32` is the default at both connection-handler entrypoints, baked into the new `Room.tick_hz` field.
- **Decision:** 30 Hz authoritative tick rate.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Per-connection `tick_dt = Duration::from_millis(1000 / 30) ≈ 33ms`.
  - The token-bucket refill rate (8 tokens/sec) is significantly below the expected client tick rate — see §6 trap.

### Decision: HMAC-SHA256 verification on the server side
- **Date:** 2025-11-18, commit `88434f3a2` ("security: fix critical vulnerabilities in network server (Priority 1)"). Verified via `git log -S "HmacSha256" -- net/aw-net-server/src/main.rs`.
- **Status:** Accepted but mismatched with client (server uses HMAC; client uses XOR `sign16`)
- **Context:** `aw-net-server/Cargo.toml:29-30` adds `hmac = "0.12"` and `sha2 = "0.10"`. `server/main.rs:703-709, 831-837` implements verification.
- **Decision:** Upgrade server-side verification from `sign16` (XOR) to HMAC-SHA256 with `verify_slice(&sig)` on a 16-byte sig.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Verification effectively always fails because `verify_slice` requires `tag.len() == 32` for HMAC-SHA256 but the sig is 16 bytes (`digest-0.10.7/src/mac.rs:168-179`).
  - Failure is `warn!`-only; player input continues to be processed regardless.
  - `net/README.md`'s "Current Implementation (MVP) — Input Validation: Lightweight XOR-based signatures" description (line 70) is half-true: the client is XOR; the server is HMAC.

### Decision: Token-bucket rate limiting with 8 tokens/sec refill
- **Date:** 2025-09-09 commit `cc9a7e3e3`.
- **Status:** Accepted (`aw-net-server/main.rs:683-685, 811-813`).
- **Context:** Three constants `REFILL_RATE = 8.0`, `BUCKET_SIZE = 60.0`, `COST_PER_MESSAGE = 1.0`.
- **Decision:** Token bucket with 8 tokens/sec refill, 60-token capacity, 1 token per InputFrame.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - A 30 Hz client cannot sustain its input rate; the bucket drains in ~2.7 seconds, then `RateLimited` fires.
  - Either the client is expected to send inputs at < 8 Hz, or the rate constants are misconfigured.

### Decision: TLS-by-default in release; opt-out only in debug builds
- **Date:** 2025-11-13, commit `4889a9a33` ("feat: Integrate astraweave-security for path validation and migration scripts"). Verified via `git log -S "tls_enabled = true" -- net/aw-net-server/src/main.rs`. This was the security-hardening sweep that also introduced HMAC verification five days later.
- **Status:** Accepted (`aw-net-server/main.rs:120-130`).
- **Context:** `--disable-tls` flag is rejected in release builds via `#[cfg(not(debug_assertions))]`.
- **Decision:** Refuse to start a release-mode server without TLS.
- **Alternatives considered:** None reasonable for a production-style server.
- **Consequences:**
  - Release builds always require valid cert+key files.
  - Developer convenience preserved through debug-build opt-out.

### Decision: `#![forbid(unsafe_code)]` on `astraweave-net-ecs` and `aw-net-proto`; not on standalone binaries
- **Date:** Initial creation.
- **Status:** Accepted (`net-ecs/lib.rs:1`; `aw-net-proto/lib.rs:1`).
- **Context:** Library crates ban unsafe; binaries don't add the attribute.
- **Decision:** Apply the attribute only at the crate-root of the libraries.
- **Alternatives considered:** None reasonable for this layer.
- **Consequences:** Inherited unsafe in deps (tokio, rustls, etc.) is unconstrained. Binaries could in principle add unsafe later without violating any in-tree attribute.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `PROTOCOL_VERSION == 1` (u16) | Yes (compile-time) | `aw-net-proto/lib.rs:6`. Mismatch is rejected with `ServerToClient::ProtocolError` (server/main.rs:273-282, 454-463). |
| 2 | `ClientToServer.Hello` is the first message every client must send | Yes | Both server handlers expect `recv*::<ClientToServer>` first; any non-Hello produces `ProtocolError { "expected Hello" }` (server/main.rs:284-294, 465-475). |
| 3 | `session_key_hint` is exactly the first 8 bytes of `session_key.0` | Yes | `server/main.rs:361, 542`: `session_hint.copy_from_slice(&room.session_key.0[0..8])`. |
| 4 | `new_room_id()` returns 8 alphanumeric characters | Yes | `aw-net-proto/lib.rs:168-174`. |
| 5 | `SessionKey.0.len() == 32` | Yes (compile-time) | Field type is `[u8; 32]` (`aw-net-proto/lib.rs:11`). |
| 6 | Room cap is 4 players (per region+game_mode) | Yes | `server/main.rs:310, 491`. |
| 7 | Empty room is removed on player disconnect | Yes | `server/main.rs:436-438, 617-619`: `if room.players.is_empty() { rooms.remove(&rid); }`. |
| 8 | `aw_net_proto::encode_msg(Codec::PostcardLz4, ...)` is followed by `decode_msg(Codec::PostcardLz4, ...)` | Test-enforced | Server (`server/main.rs:153`) and client (`client/main.rs:69`) both pin PostcardLz4. ECS Plugin layer (`net-ecs/lib.rs:179, 237, 279`) pins Bincode independently. Codec mismatch produces `WireError::Decode`. |
| 9 | Per-connection ticking advances `room.tick` per connection (not per tick globally) | Yes (current behavior) | `server/main.rs:631` inside `build_snapshot` increments `room.tick += 1` per call. Each `handle_socket{,_tls}` runs its own `sleep(tick_dt)` loop and calls `build_snapshot` independently. With N connected players in one room, `room.tick` advances N times per real second per Hz unit. [INFERRED — this is what the code reads; whether the design intends per-room or per-connection tick semantics is decisional.] |
| 10 | HMAC-SHA256 verification result does NOT terminate the connection | Yes | `server/main.rs:707-709, 835-837`: `if mac.verify_slice(&sig).is_err() { warn!(...); }` — no return, no kick. |
| 11 | Token bucket refill rate is 8 tokens/sec; bucket capacity is 60; cost per message is 1 | Yes (compile-time) | `server/main.rs:683-685, 811-813` constants. |
| 12 | `tick_hz` defaults to 30 | Yes | `server/main.rs:301, 482` (initial), `:321, 502` (Room construction). |
| 13 | `--disable-tls` is forbidden in release builds | Yes (compile-time) | `server/main.rs:120-123` `#[cfg(not(debug_assertions))]`. |
| 14 | `astraweave-net-ecs::server_snapshot_system` always sends snapshots with `entity_states: HashMap::new()` | Yes | `net-ecs/lib.rs:174`. Stub system. |
| 15 | Compression of snapshots in `aw-net-server::build_snapshot` is always on (`compressed: true`) | Yes | `server/main.rs:653-654`. |

---

## 9. Performance & Resource Profile

### Hot paths

- **Per-connection per-tick `build_snapshot`** (`server/main.rs:625-657`) — runs `tick_dt` apart (default ~33 ms). Cost: mutex acquire on `app.rooms`, increment two `u32`/`u64` fields, postcard serialize a tiny `DemoState { tick: u64 }`, lz4 compress. Sub-millisecond by inspection.
- **Per-message `decode_msg`** in the per-connection select loop (`server/main.rs:409, 590`) — lz4 decompress + postcard deserialize.
- **HMAC-SHA256 update + verify** per `InputFrame` (`server/main.rs:703-707, 831-835`) — fast for small `input_blob`s but allocates a new `HmacSha256` per message.
- **ECS systems run once per stage tick.** None hit the network synchronously; all encode/decode work in the ECS path is in-process (`encode_msg(Codec::Bincode, &snapshot)` at `net-ecs/lib.rs:179`, with no I/O).

### Cold paths

- **TLS handshake** (`server/main.rs:217-223`) — milliseconds per connection.
- **Matchmaking room scan** (`server/main.rs:309-310, 490-491`) — linear scan of `rooms` HashMap; rooms cap not enforced; current scale assumed small.
- **`SessionKey::random()`** — one 32-byte `rand::rng().fill` per new room.
- **`net_ecs_adversarial.rs`** (921 lines, 11 bench cases) — explicitly worst-case payload shapes for serialization.

### Resource ownership

- **`AppState.rooms: Arc<parking_lot::Mutex<HashMap<RoomId, Room>>>`** — server-wide lock (`parking_lot` flavor, not `std::sync`). Acquired per-message and per-tick. Cloned (the `Arc`) into each connection task.
- **`sled::Db`** — opened at `aw_net_server_db/`; never read or written in the visible code. Stays open for the server's lifetime.
- **`SessionKey.0: [u8; 32]`** — owned by `Room`. Cloned when starting a new room session.
- **Per-connection `tokio::spawn`** — one spawn per WebSocket accept; each task owns its `WebSocketStream`, a clone of `AppState`, and a clone of the `TlsAcceptor` (in TLS mode).
- **`HmacSha256` instance** — new per `InputFrame`; lives only for the verification check.
- **ECS Plugin components** — owned by ECS entity storage. `CNetworkAuthority.connected_clients: HashMap<String, mpsc::UnboundedSender<ServerToClient>>` keeps a strong reference to every connected client's channel; if a client disconnects without cleanup, the sender is retained — potential leak source [INFERRED — no cleanup path observed in the ECS systems].

---

## 10. Testing & Validation

- **Unit tests:** Inline `#[cfg(test)] mod tests` in `astraweave-net-ecs/src/lib.rs`: 4 tests (`client_input_processing`, `client_reconciliation`, `server_snapshot_generation`, `network_integration`).
- **Integration tests:**
  - `astraweave-net-ecs/tests/mutation_resistant_comprehensive_tests.rs`: 27 tests.
  - `net/aw-net-proto/tests/mutation_resistant_comprehensive_tests.rs`: 53 tests.
- **Total tests in this subsystem:** **84 tests** (counting only files that actually compile; `lib_temp.rs`'s 4 tests are in orphan source and do not run).
- **No tests in `net/aw-net-server/` or `net/aw-net-client/`.** Both standalone binaries are validated only by manual end-to-end runs per `net/README.md:13-30`.
- **Benchmarks:**
  - `astraweave-net-ecs/benches/net_ecs_benchmarks.rs`: 9 `bench_function` call sites.
  - `astraweave-net-ecs/benches/net_ecs_adversarial.rs`: 11 `bench_function` call sites (worst-case shapes).
  - `net/aw-net-proto/benches/proto_bench.rs`: 13 `bench_function` call sites.
- **Total bench cases:** 33.
- **CI presence:** **None as of `a2474c5b7`.** Workspace grep `grep -l "aw-net\|astraweave-net-ecs" .github/workflows/*.yml` returns no matches. No dedicated workflow runs `cargo test` for any of these four crates. Compare with the older `astraweave-net` crate which has `.github/workflows/net-tests.yml`.
- **Mutation testing:** Per-crate inline mutation-resistance test suites (27 in `astraweave-net-ecs`, 53 in `aw-net-proto`); not run from a centralized workflow.
- **Miri / Kani validation:** Not in `.github/workflows/miri.yml` or `kani.yml`. The library crates carry `#![forbid(unsafe_code)]`; the binaries do not but contain no observable unsafe blocks.
- **Manual validation:** `net/README.md:13-30` documents the manual smoke-test workflow:
  1. `cargo run -p aw-net-server`
  2. `cargo run -p aw-net-client`
  3. Observe tracing output for `joined; tick_hz=30`.

---

## 11. Open Questions / Parked Decisions

- **Why is `astraweave-net-ecs` declared by `astraweave-stress-test` but not imported?** Workspace grep `use astraweave_net_ecs` returns no source-file matches outside `astraweave-net-ecs`'s own tests/benches. `astraweave-stress-test/Cargo.toml:20` declares `astraweave-net-ecs = { workspace = true }` but no source file in that crate references the types. **Investigation (2026-05-12):** The dep was added in commit `08befc6ec` (2025-10-01, "phase 6 implementation") — the **same** commit that created `astraweave-net-ecs` itself. `astraweave-stress-test/src/lib.rs:38-42` defines its own `CNetworkStress { player_id, input_buffer, last_sync }` component, parallel to but disjoint from `astraweave-net-ecs::CNetworkClient`. So the dep was added at crate creation and never wired through; the stress test invented its own simulated network state types. Is this stale residue, future planning, or a deliberate ABI-only inclusion? Andrew's call.

- **Why does the client sign with `sign16` (XOR) while the server verifies with HMAC-SHA256?** The two algorithms produce different bytes for the same input, so every signature verification fails. The server's `warn!` is observed-only; player input continues to be processed. Was this an in-progress upgrade where the server was updated first and the client was forgotten, or a deliberate "no enforcement yet" stance with a HMAC scaffold ready for future activation? Andrew's call.

- **What is `lib_temp.rs` for?** A 436-line near-duplicate of `lib.rs` exists in `astraweave-net-ecs/src/` but is not declared as a module. Created 2025-11-19 in a commit titled "added forest biome textures and assets, worked on renderer" — the title is unrelated. Is this orphan source to be deleted, a backup awaiting a refactor, or part of an in-progress experiment? Andrew's call.

- **Is the empty `entity_states: HashMap::new()` in `server_snapshot_system` a stub awaiting wiring, or by design?** The system runs successfully and sends out snapshots, but they carry no actual entity data. The ECS Plugin layer's snapshot pipeline is effectively a no-op. **Investigation (2026-05-12):** `git log -p -S "entity_states"` shows the empty `HashMap::new()` was present from the crate's birth commit `08befc6ec` (2025-10-01) — the comment in the original commit reads `// Would collect actual entity states` at `astraweave-net-ecs/src/lib.rs:174`. Day-one stub, never replaced. The decisional part remains: should this be wired up, removed, or kept as a marker for future implementation?

- **What is the intended fix for the token-bucket / client-tick mismatch?** At 30 Hz client tick (per the documented design), the 8-tokens-per-second refill rate drains the 60-token bucket in ~2.7 seconds and then `RateLimited` fires continuously. Is the client tick rate wrong, the refill rate wrong, or is the rate-limit threshold tuned for some other client behavior?

- **Why is `sled::Db` opened but never used?** `AppState.db` is opened on server startup but has no read/write call site. The README's claim that the server persists room/player state survives restarts is not backed by code. Is this a parked persistence feature or dead allocation?

- **Why is `Codec::Bincode` (in `astraweave-net-ecs`) hardcoded instead of using the same `Codec::PostcardLz4` as the standalone trio?** Mixing the two requires every ECS-layer consumer to handle a different wire format than the standalone server uses. Is this intentional (separate use-cases) or oversight?

- **Why are the TLS stack versions one major behind the other workspace crate?** `aw-net-server` pins `tokio-rustls = "0.25"` + `rustls = "0.22"` while `astraweave-net` uses `tokio-rustls = "0.26"` + `rustls = "0.23"` (workspace-pin). **Investigation (2026-05-12):** Confirmed via `Cargo.lock` that the workspace contains **both** TLS major versions simultaneously — `rustls 0.22.4` AND `rustls 0.23.32`, `tokio-rustls 0.25.0` AND `tokio-rustls 0.26.4`. `cargo tree -i tokio-rustls@0.26.4` shows it arrives via `reqwest 0.12.24 → hyper-rustls 0.27.7 → tokio-rustls 0.26.4`, consumed by `astraweave-llm`, `astraweave-assets`, `astraweave-ai`, `astraweave-stress-test`, and many examples. `tokio-rustls 0.25.0` is exclusive to `aw-net-server`. A single workspace build that includes both `aw-net-server` and a reqwest user (most builds) compiles both versions. Is this acceptable for current scale, or should `aw-net-server` be bumped to align?

- **Why is there no CI workflow for any of the four crates in this subsystem?** None of `astraweave-net-ecs`, `aw-net-proto`, `aw-net-client`, `aw-net-server` appear in any `.github/workflows/*.yml`. **Investigation (2026-05-12):** Confirmed via `grep -rn "aw-net-server\|aw-net-client\|aw_net_proto\|astraweave_net_ecs\|astraweave-net-ecs" .github/` — zero matches across the entire `.github/` directory tree (27 workflow files exist in `.github/workflows/`). The older `astraweave-net` has a dedicated `net-tests.yml`. Is the absence intentional (binaries are validated by manual smoke tests) or a CI-pipeline gap?

- **Should `handle_socket_tls` / `handle_socket` be unified?** The ~400 lines of code duplication between TLS and plain paths in `aw-net-server/main.rs` could plausibly be abstracted over the `WebSocketStream` type parameter, but doing so requires either a generic over `AsyncRead + AsyncWrite + Unpin` or a trait abstraction. Andrew's call on whether the duplication is worth the abstraction cost.

---

## 12. Maintenance Notes

**Update this doc when:**
- A new wire-protocol variant is added to `ClientToServer` or `ServerToClient` (§2, §3 vocabulary, §8 invariant 1).
- The codec choice between paths is reconciled (§6 codec row, §11 codec question).
- The signature scheme is unified between client and server (§6 sign16-vs-HMAC row, §11 second question).
- `lib_temp.rs` is either declared as a module or removed (§5 row, §6 row, §11 third question).
- `sled::Db` gains actual read/write call sites (§4 sled row, §6 sled row, §11 sled question).
- A workflow is added that runs `cargo test` for any of these four crates (§10 CI presence note).
- The standalone binaries gain test coverage (§10 "No tests" note).
- A real production consumer of `astraweave-net-ecs` lands (§4 downstream table, §11 first question).
- TLS stack versions are bumped to align with workspace (§6 TLS-version row).

**Verification process:**
- `rg 'pub fn|pub struct|pub enum|pub trait' astraweave-net-ecs/src/lib.rs net/aw-net-proto/src/lib.rs` should match §3 vocabulary surface.
- `cargo tree -p astraweave-net-ecs --depth 1` should list `aw-net-proto`, `astraweave-ecs`, `astraweave-core`, `postcard`, `lz4_flex`, `crc32fast`, `glam`, `bincode`, `tokio`, `futures-util`, `tokio-tungstenite`, `anyhow`, `serde`, `serde_json`.
- `cargo tree -p aw-net-server --depth 1` should list `aw-net-proto`, `tokio`, `axum`, `hyper`, `tokio-tungstenite`, `tungstenite`, `futures`, `serde`, `serde_json`, `anyhow`, `thiserror`, `tracing`, `tracing-subscriber`, `parking_lot`, `uuid`, `time`, `sled`, `postcard`, `lz4_flex`, `tokio-rustls`, `rustls`, `rustls-pemfile`, `hmac`, `sha2`.
- `rg 'use astraweave_net_ecs\|use aw_net_proto' --type rust -g '!*test*' -g '!benches/*'` should match §4 consumers; new consumers must be added.
- `grep -c '#\[test\]\|#\[tokio::test\]' astraweave-net-ecs/src/lib.rs astraweave-net-ecs/tests/*.rs net/aw-net-proto/tests/*.rs` should total ≥ 84 (test-count invariant grows, never shrinks).
- Stamp the new commit hash and date in the metadata table.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. There are TWO integration paths sharing `aw-net-proto`: the standalone `net/aw-net-{client,server}` binaries (PostcardLz4, TLS-by-default, matchmaking, HMAC verify), and `astraweave-net-ecs` (ECS Plugin layer with stub systems, Bincode codec, plain `ws://`). They do not currently interoperate.
2. **`astraweave-net-ecs` has no production consumers.** The declared dep in `astraweave-stress-test/Cargo.toml:20` is unused. The Plugin layer is dormant code.
3. **HMAC verification on the server is observed-only** — failed verification logs a `warn!` but does not affect player processing. The client signs with the XOR `sign16` so HMAC verification effectively always fails.
4. **`sled::Db` is opened but unused** — the README's persistence claim is unbacked.
5. **Token-bucket rate (8/sec refill) is below client tick rate (30 Hz)** — sustained input will trigger `RateLimited` after ~2.7s.
6. **`lib_temp.rs` is orphan source** — not declared, not compiled.
7. **None of these four crates are in CI** as of `a2474c5b7`.

**Files you'll most likely touch:**
- `net/aw-net-proto/src/lib.rs` — wire-protocol changes (always coordinated with both client and server).
- `net/aw-net-server/src/main.rs` — matchmaking, rate-limit, HMAC verification, snapshot construction. Both `handle_socket` and `handle_socket_tls` may need parallel edits (and the three other `_tls`/non-`_tls` helper pairs).
- `net/aw-net-client/src/main.rs` — client input/reconciliation flow.
- `astraweave-net-ecs/src/lib.rs` — ECS Plugin layer (stub systems and async helpers).

**Files you should NOT touch without strong reason:**
- `astraweave-net-ecs/src/lib_temp.rs` — orphan duplicate; modifications don't compile into the crate.
- `net/aw-net-proto/tests/mutation_resistant_comprehensive_tests.rs` and `astraweave-net-ecs/tests/mutation_resistant_comprehensive_tests.rs` — mutation-resistance assertions; changes here can mask real bugs.
- `aw_net_server_db/conf`, `aw_net_server_db/db` — sled runtime artifacts. Delete only if explicitly resetting the dev server's state.

**Common mistakes when changing this system:**
- **Adding a `ClientToServer` variant without updating both `handle_socket` and `handle_socket_tls` (and their `on_client_msg` / `on_client_msg_tls` helpers).** The duplication is wide.
- **Adding a `ServerToClient` variant without updating the client's match arms in `net/aw-net-client/src/main.rs:48-63` and the per-tick read loop.**
- **Changing `Codec` choice in one path without changing the other.** The Plugin layer's Bincode and the standalone binaries' PostcardLz4 must agree if you want them to talk.
- **Assuming the HMAC verification gates input.** It doesn't — it only logs a `warn!`.
- **Trusting `app.db` to persist anything.** It doesn't — no reads or writes go through it.
- **Adding `unsafe` to `astraweave-net-ecs/src/lib.rs` or `aw-net-proto/src/lib.rs`.** Both crate roots carry `#![forbid(unsafe_code)]`.

---

## Appendix B: Historical context

The standalone trio (`net/aw-net-{proto,client,server}`) was introduced as a single squashed PR on **2025-09-09** in commit `cc9a7e3e3` ("Implement production-ready enhanced networking layer with server authority, client prediction, and matchmaking (#58)"). This predates the ECS Plugin layer.

The ECS Plugin layer (`astraweave-net-ecs`) was added on **2025-10-01** in commit `08befc6ec` ("phase 6 implementation") — roughly three weeks after the standalone trio. By that point, the standalone server had already adopted HMAC verification while the client retained `sign16`.

The `lib_temp.rs` orphan was added on **2025-11-19** in commit `54d15c9f2` ("added forest biome textures and assets, worked on renderer"). The commit title gives no clue about why a temp file was added to the netcode crate — likely an unintentional checkin during a renderer-focused commit.

The older `astraweave-net` snapshot subsystem (documented separately in `docs/architecture/net.md`) was created 2025-09-05 and remains in place. Per `net/README.md:90-92`, the two networking systems are intended to coexist long-term.

The `aw_net_server_db/` directory is created on first server run by sled's `Db::open` call. Its presence in git (`conf` 62 B, `db` 96 B) indicates someone ran the server locally and committed the artifacts; the directory is not in any `.gitignore`.
