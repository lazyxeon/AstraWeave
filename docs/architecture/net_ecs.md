---
schema_version: 1
trace_id: net_ecs
title: "Net-ECS (ECS Plugin + Standalone Matchmaking Server)"
description: "Net-ECS + standalone matchmaking"
primary_crate: astraweave-net-ecs
domain: networking
lifecycle_status: unknown
integration_status: unknown
owns: [astraweave-net-ecs, aw-net-client, aw-net-proto, aw-net-server]
doc_version: "1.3"
last_verified_commit: eb9977b88
---

# Architecture Trace: Net-ECS (ECS Plugin + Standalone Matchmaking Server)

> **Scope note:** This doc traces the **second** of three networking subsystems in AstraWeave: the ECS-Plugin layer (`astraweave-net-ecs`) plus the production-style standalone-binary trio (`net/aw-net-proto`, `net/aw-net-client`, `net/aw-net-server`) and their runtime artifact directory (`aw_net_server_db/`). Other networking subsystems:
> - `docs/architecture/net.md` — `astraweave-net` (snapshot-based game server)
> - `docs/architecture/persistence_ecs.md` — `astraweave-persistence-ecs` (save/load)

## Metadata

| Field | Value |
|---|---|
| **System name** | Net-ECS + standalone matchmaking server |
| **Primary crates** | `astraweave-net-ecs` (ECS Plugin layer); `net/aw-net-proto` (wire protocol); `net/aw-net-client` (standalone CLI client binary); `net/aw-net-server` (standalone CLI server binary with matchmaking, persistence, anti-cheat); `aw_net_server_db/` (sled runtime artifacts — not code) |
| **Document version** | 1.3 |
| **Last verified against commit** | `eb9977b88` |
| **Last verified date** | 2026-06-10 |
| **Status** | Active (mixed: production-grade standalone binary; dormant ECS Plugin layer). **Standalone-trio input signing is real and enforced:** client and server both compute HMAC-SHA256 over the canonical `aw-net-proto` signing surface; the server verifies FIRST (before any per-player state mutation) and kicks by default on failure. The historical `sign16`/XOR-vs-HMAC mismatch is RESOLVED (Net-Trio-Remediation, see revision 1.3). |
| **Revision history** | 1.3 (2026-06-10): **Net-Trio-Remediation closeout.** The standalone trio's signature defect — client signed with the XOR `sign16` (16-byte tag) while the server verified HMAC-SHA256 (32-byte tag), so every verification failed and the server only `warn!`ed — is FIXED and ENFORCED. `aw-net-proto` now exposes a canonical signing surface (`SigningKey` with a private 32-byte field + redacted `Debug`; `sign`/`verify` (constant-time via `Mac::verify_slice`); `hmac_sha256`; `input_frame_sig_payload(seq, tick_ms, blob)`; `SIG_LEN = 32`). The `sign16` XOR stub, the `SessionKey` type, and the `session_key_hint` fields on `MatchResult`/`JoinAccepted` are DELETED; `InputFrame.sig` is now `[u8; 32]`. Both client (sign) and server (verify) build the MAC'd bytes via the single shared `input_frame_sig_payload`. The server is now lib+bin (`aw-net-server/src/lib.rs` ~1080 lines holds all logic; `src/main.rs` is a thin arg-parse → `run_server`). Verification runs FIRST in both plain and TLS handlers and applies `SignatureFailurePolicy { Kick (DEFAULT), Warn }`: Kick sends a WebSocket Close (code 1008) and routes the disconnect through the existing cleanup path; Warn retains legacy log-and-continue. Regression net: 104 tests (proto 59, server 41 across families 1-5 + w2b_fix1, client 4); a §5 adversarial refute pass ran and CONVERGED. Commit ledger: `561b20957` (W.1 canonical HMAC; stub deleted), `79424389e` (W.2.a client adopts), `066cd6cfd` (W.2.b server verifies + policy + lib/bin split), `9a3fc94e3` (W.2.b-fix1 graceful unknown-room + cleanup-preserving snapshot path), `7029d7d7f`/`a2b494942`/`0e702738e`/`68a9a1936` (W.3.1-3.4 test families 1-4), `420a6f61b` (W.5.1 SigningKey encapsulation + RFC 4231), `2955cd14c` (W.5.2 TLS test family), `eb9977b88` (W.5.3 S2C trust note). Known deliberate boundaries (NOT open defects): replay protection / nonces / sequence-number freshness is not implemented (HMAC proves authenticity, not freshness — a captured valid frame can be replayed); server→client messages are not signature-verified (authoritative-server asymmetric trust — a shared symmetric key cannot meaningfully authenticate S2C); the shipped client binary over TLS is untested (native-tls rejects the self-signed dev cert — a cert-validation concern), though the server's TLS verify/kick path IS now tested. Full audit: `docs/audits/net_trio_signature_remediation_findings_2026-06.md`.<br><br>1.2 (2026-05-12): Deep investigation pass. Enriched §11 Open Questions 1, 4, 8, 9 with comprehensive factual context. Recovered creation commit for `astraweave-net-ecs` dep in `astraweave-stress-test` (commit `08befc6ec` — same as net-ecs's own birth commit). Verified Q8 dual-TLS-version situation factually via `Cargo.lock` (both `tokio-rustls 0.25.0` and `tokio-rustls 0.26.4` are present) and via `cargo tree` (0.26 comes from reqwest→hyper-rustls used by many crates including `astraweave-llm`, `astraweave-assets`; 0.25 is exclusive to `aw-net-server`). Q4 factual finding: empty `entity_states: HashMap::new()` was present from the crate's birth commit `08befc6ec` (2025-10-01) — stub from day one. Resolved the last `[INFERRED]` marker (§5 TLS_IMPLEMENTATION_SUMMARY.txt content). Added new §6 row: a third copy of `lib.rs`-like content exists at `archive/temp_files/temp/temp_lib.rs` (a `use aw_net_proto` archive).<br><br>1.1 (2026-05-12): Verification pass. Corrected §2 Stage 6 HMAC mechanism — `hmac::verify_slice` (via `digest-0.10.7/src/mac.rs:168-179`) strict-rejects any tag length ≠ `OutputSize`; the previous `[INFERRED]` "truncated comparison" claim was wrong. The real mechanism: `Sha256::OutputSize == 32`, sig is 16 bytes, so `verify_slice` returns `MacError` immediately on length mismatch — **all client signatures fail length validation before any byte comparison happens**. Resolved §7 Decision 5 Date marker — HMAC verification landed in commit `88434f3a2` (2025-11-18, "security: fix critical vulnerabilities in network server (Priority 1)"). Resolved §7 Decision 7 Date marker — TLS-by-default in release added in commit `4889a9a33` (2025-11-13, "feat: Integrate astraweave-security…"). Noted that `parking_lot::Mutex` (not `std::sync::Mutex`) is used in §3 and §9. |
| **Owner notes** | Two distinct integration paths share `aw-net-proto`: (1) the standalone `aw-net-server` / `aw-net-client` binaries communicate end-to-end over WSS with HMAC-SHA256 input signatures, token-bucket rate limiting, sled persistence, and matchmaking. As of revision 1.3 the signing is real and enforced: client and server agree on bytes via the canonical proto surface (`SigningKey` + `sign`/`verify` + `input_frame_sig_payload`), the server verifies FIRST, and the default `SignatureFailurePolicy::Kick` closes the connection (Close 1008) on a bad tag. (2) `astraweave-net-ecs` provides ECS Plugin scaffolding (`NetworkClientPlugin`, `NetworkServerPlugin`, components, simulation-stub systems) — but **workspace grep finds zero `use astraweave_net_ecs` outside the crate's own tests and benches**, including in `astraweave-stress-test` which declares the dep in `Cargo.toml`. The ECS layer is currently dormant code. None of these crates appear in any `.github/workflows/*.yml` as of `a2474c5b7`. |

---

## 1. Executive Summary

**What this system does:**
Provides two coexisting integration paths over a shared binary wire protocol (`aw-net-proto`):

1. **Standalone binary trio** (`net/aw-net-{proto,client,server}`): A production-style multiplayer server with axum HTTP admin endpoints, region-aware matchmaking, room-based session management, sled-backed persistence, token-bucket rate limiting, **enforced HMAC-SHA256 input signing**, and TLS-by-default WebSocket. The client signs each `InputFrame` with HMAC-SHA256 over the canonical `input_frame_sig_payload(seq, tick_ms, blob)`, and the server verifies it FIRST (before any per-player state mutation) via the same shared `aw-net-proto` surface — kicking by default (`SignatureFailurePolicy::Kick` → Close 1008) on a bad tag. Demo client connects with `wss://`, joins a room via `FindOrCreate`, then streams `InputFrame` messages at 30 ms intervals. (Before the Net-Trio-Remediation, the client signed with the XOR `sign16` while the server verified HMAC-SHA256, so every verification failed and the server only `warn!`ed — see §6/§7, now RESOLVED.)

2. **ECS Plugin layer** (`astraweave-net-ecs`): A library that registers `NetworkClientPlugin` / `NetworkServerPlugin` with the `astraweave-ecs::App` and adds four systems (`client_input_system`, `client_reconciliation_system`, `server_snapshot_system`, `server_input_processing_system`) plus three components (`CNetworkClient`, `CClientPrediction`, `CNetworkAuthority`) and async helpers (`connect_to_server`, `start_network_server`). The systems contain simulation stubs (e.g., literal `prediction.predicted_position.x += 0.1` placeholder for prediction).

**Why it exists:**
Per `net/README.md:1-12`, the goal was a "production-ready multiplayer capabilities" path layered on top of (not replacing) the older `astraweave-net` snapshot system. The standalone trio is the user-facing artifact; `astraweave-net-ecs` is the ECS adaptation layer intended to plug the protocol into a game's ECS world.

**Where it primarily lives:**
- `astraweave-net-ecs/src/lib.rs` — 437 lines. ECS Plugin + components + four systems + two async helpers + 4 inline tests.
- `astraweave-net-ecs/src/lib_temp.rs` — 436 lines. Near-duplicate of `lib.rs` (see §6).
- `net/aw-net-proto/src/lib.rs` — 285 lines. `ClientToServer` / `ServerToClient` enums, `Codec` (PostcardLz4 / Bincode), `encode_msg` / `decode_msg`, the **canonical signing surface** (`SigningKey`, `sign`, `verify`, `hmac_sha256`, `input_frame_sig_payload`, `SIG_LEN = 32`), `new_room_id`, `PROTOCOL_VERSION = 1`, `WireError`. **The `sign16` XOR stub, the `SessionKey` type, and the `session_key_hint` fields are DELETED** (Net-Trio-Remediation `561b20957`).
- `net/aw-net-server/src/lib.rs` — ~1080 lines. All server logic: `ServerConfig` / `run_server` / `spawn_server`, TLS+plain dual paths, matchmaking, rooms, sled DB, axum HTTP admin, **HMAC-SHA256 verification via the canonical proto surface (verify-FIRST)**, `SignatureFailurePolicy { Kick, Warn }`, token-bucket rate limiting.
- `net/aw-net-server/src/main.rs` — 116 lines. Thin CLI wrapper: parse args (`--shared-key-hex`, `--sig-failure-policy`, `--disable-tls`, `--tls-cert`/`--tls-key`, `--ws-listen`/`--http-listen`/`--db-path`) into a `ServerConfig`, then `run_server`.
- `net/aw-net-client/src/main.rs` — 193 lines. Standalone CLI client demo. Signs each `InputFrame` with `aw_net_proto::sign` over `input_frame_sig_payload`; key from `AW_SHARED_KEY` (64 hex) or `SigningKey::dev_default()`.
- `net/aw-net-server/tests/`, `net/aw-net-client/tests/` — regression families (see §5, §10).
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
    │                                  tick_hz: 30,
    │                                  players: HashMap::new(),
    │                                  tick: 0, snap_id: 0 }
    │   (no per-room session_key — input signing keys off the process-wide
    │    shared SigningKey from --shared-key-hex / AW_SHARED_KEY / dev_default)
    │
    ├── ServerToClient::MatchResult { room_id }
    └── ServerToClient::JoinAccepted { room_id, player_id: Uuid::v4(), tick_hz }

[Per-connection game loop]                                                (server/main.rs:401-429, 582-610)
    │
    │ tokio::select! biased:
    │
    ├── ws.next() → ClientToServer::InputFrame { seq, tick_ms, input_blob, sig: [u8;32] }
    │     ↓ on_client_msg_tls / on_client_msg                             (server/lib.rs:861-938, 999-1080)
    │     ↓ VERIFY FIRST (before any state mutation):                     (server/lib.rs:879-893, 1021-1035)
    │       · payload = aw_net_proto::input_frame_sig_payload(seq, tick_ms, &input_blob)
    │       · aw_net_proto::verify(&signing_key, &payload, &sig)  (constant-time)
    │       · fail + policy Kick (DEFAULT) → MsgOutcome::Kick → Close 1008 + cleanup
    │       · fail + policy Warn          → warn!() and process anyway (legacy)
    │     ↓ Token-bucket rate limit: 8 tokens/sec refill, 60-bucket, 1 cost/msg
    │       · if tokens < 0.0 → kick = true → ServerToClient::RateLimited
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
- `Codec` is `#[non_exhaustive]` (proto/lib.rs:235). PostcardLz4 is the recommended path (smaller payloads); Bincode is "Fallback / compatibility" per the doc comment at proto/lib.rs:239.
- **Canonical signing surface** (Net-Trio-Remediation `561b20957`): `SigningKey` (private 32-byte field; `from_hex` / `from_bytes` / `as_bytes` / `dev_default`; redacted `Debug`) at proto/lib.rs:28-94; `sign(key, payload) -> [u8;32]` (proto/lib.rs:112), `verify(key, payload, tag) -> bool` (constant-time via `Mac::verify_slice`, proto/lib.rs:121-127), `hmac_sha256(key, payload)` (proto/lib.rs:101-109), `input_frame_sig_payload(seq, tick_ms, blob)` — the canonical MAC'd byte range (`seq.to_le_bytes() ++ tick_ms.to_le_bytes() ++ input_blob`, proto/lib.rs:139-145), and `SIG_LEN = 32` (proto/lib.rs:10). `InputFrame.sig` is `[u8; 32]` (proto/lib.rs:173). **The former `sign16` XOR stub and `SessionKey` type are DELETED.**
- `new_room_id()` returns 8 alphanumeric characters (proto/lib.rs:279-285).

> **Layout note (revision 1.3):** the standalone server was split into lib+bin (`066cd6cfd`). All server logic now lives in `net/aw-net-server/src/lib.rs` (~1080 lines: `ServerConfig`, `run_server`, `spawn_server`, `accept_loop_{tls,plain}`, `handle_socket{,_tls}`, `on_client_msg{,_tls}`, `build_snapshot`, the `send`/`recv` helpers); `src/main.rs` (116 lines) is a thin CLI wrapper that parses args into a `ServerConfig` and calls `run_server`. The `server/main.rs:*` line citations in Stages 2-5 and 7 below reflect the **pre-remediation single-file layout** and are retained for forensic continuity; for current line numbers read `lib.rs` (Stage 6, fully reconciled below, is authoritative for the verification path).

#### Stage 2: Standalone server entry (`aw-net-server/src/lib.rs` — `spawn_server`)
**Role:** Parse CLI args (`--disable-tls`, `--tls-cert`, `--tls-key`, `--shared-key-hex`, `--sig-failure-policy`, `--ws-listen`, `--http-listen`, `--db-path`) in `main.rs::parse_args`, open sled DB, spawn the axum HTTP admin server on port 8789, then loop-accept WebSocket connections on port 8788.
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
- `display_name` on `JoinRoom` is destructured but ignored (server/lib.rs:429, 661: `display_name: _`).
- **No `session_key`/`session_key_hint` anymore** (deleted in `561b20957`). `MatchResult` carries only `{ room_id }` and `JoinAccepted` carries `{ room_id, player_id, tick_hz }` (server/lib.rs:499-516, 731-748). Input-frame signing keys off the process-wide shared `SigningKey` (`AppState.signing_key`), not a per-room key.
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

#### Stage 6: HMAC-SHA256 input verification (`server/lib.rs:861-938 (plain), 999-1080 (TLS)`)
**Role:** Authenticate every `InputFrame` against the shared `SigningKey` BEFORE the frame is allowed to influence any server state, and apply the configured `SignatureFailurePolicy` on failure.
**Inputs:** `InputFrame.{seq, tick_ms, input_blob, sig: [u8; 32]}`, `AppState.signing_key: SigningKey`, `AppState.sig_failure_policy`.
**Outputs:** `MsgOutcome::Kick("input frame signature verification failed")` under policy `Kick` (default), or a `warn!` + continue under policy `Warn`.
**Notes (revision 1.3 — Net-Trio-Remediation, fully reconciled against the live code):**
- **Both ends build the MAC'd bytes via the single shared `input_frame_sig_payload(seq, tick_ms, &input_blob)`** (proto/lib.rs:139-145). The client signs (`aw_net_proto::sign`, client/main.rs:101) and the server verifies (`aw_net_proto::verify`, server/lib.rs:880, 1022) over EXACTLY those bytes. There is no longer any byte-range divergence between client and server.
- **`verify` is constant-time** — it calls `Mac::verify_slice` internally (proto/lib.rs:121-127); MAC tags are never compared with `==`.
- **Verification runs FIRST** — before the rate-limit token deduction and before `last_input_seq` / `last_seen` are touched (server/lib.rs:876-893, 1016-1035). An unauthenticated packet cannot influence server state. The plain and TLS handlers are kept semantically identical by contract (a skippable path on either is a security bug — see the in-code comment at lib.rs:1019-1020).
- **The verification result DOES gate processing under the default `Kick` policy.** On a bad tag with `SignatureFailurePolicy::Kick`, the handler returns `MsgOutcome::Kick`, the connection loop sends a WebSocket Close frame (`CloseCode::Policy`, code 1008) and breaks into the shared cleanup block (player removed, empty room dropped) — see server/lib.rs:533-545, 765-777. Under `SignatureFailurePolicy::Warn` the legacy behavior is retained (log a `warn!` and process the packet anyway), provided for debugging.
- The former pre-remediation defect — client signing with the XOR `sign16` (16-byte tag) while the server verified HMAC-SHA256 (32-byte tag), so every `verify_slice` failed on length mismatch and the server only `warn!`ed — is **RESOLVED** (`561b20957`+`79424389e`+`066cd6cfd`). The `sign16` function and the `[u8; 16]` sig field no longer exist.

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
| **ClientToServer** | `#[non_exhaustive]` enum of 6 wire messages: `Hello`, `FindOrCreate`, `JoinRoom`, `InputFrame`, `Ping`, `Ack`. `InputFrame.sig` is `[u8; 32]`. | `aw-net-proto/lib.rs:150-183` |
| **ServerToClient** | `#[non_exhaustive]` enum of 8 wire messages: `HelloAck`, `MatchResult { room_id }`, `JoinAccepted { room_id, player_id, tick_hz }`, `Snapshot`, `Reconcile`, `Pong`, `RateLimited`, `ProtocolError`. (No `session_key_hint` on `MatchResult`/`JoinAccepted` — deleted in `561b20957`.) | `aw-net-proto/lib.rs:185-220` |
| **Codec** | `#[non_exhaustive]` enum: `PostcardLz4` (default), `Bincode` (compat). | `aw-net-proto/lib.rs:234-241` |
| **SigningKey** | Shared symmetric 32-byte HMAC key. Private `[u8; 32]` field (no tuple `.0` access); constructors `from_hex` (64 hex chars) / `from_bytes` / `dev_default`; accessor `as_bytes`; `Debug` is redacted to `SigningKey(<redacted>)`. Never transmitted on the wire — both ends hold it out-of-band (client: `AW_SHARED_KEY` or `dev_default`; server: `--shared-key-hex` or `dev_default`). | `aw-net-proto/lib.rs:28-94` |
| **sign / verify** | `sign(key: &SigningKey, payload: &[u8]) -> [u8; 32]` (HMAC-SHA256); `verify(key, payload, tag: &[u8; 32]) -> bool` (constant-time via `Mac::verify_slice`). Backed by `hmac_sha256(key, payload)`. | `aw-net-proto/lib.rs:101-127` |
| **input_frame_sig_payload** | THE canonical MAC'd byte range for `InputFrame`: `seq.to_le_bytes() ++ tick_ms.to_le_bytes() ++ input_blob`. Both client (signing) and server (verifying) MUST build the payload via this function — the single shared definition is what prevents signed-byte-range divergence. | `aw-net-proto/lib.rs:139-145` |
| **SIG_LEN** | `usize = 32` — length of an HMAC-SHA256 signature tag. | `aw-net-proto/lib.rs:10` |
| **SignatureFailurePolicy** | `#[derive(Default)]` enum `{ Kick (DEFAULT), Warn }`. On a failed `InputFrame` signature: `Kick` rejects the packet (no state updates) and disconnects via WebSocket Close (1008); `Warn` logs and processes anyway (legacy). `FromStr` parses `--sig-failure-policy kick\|warn`. | `aw-net-server/lib.rs:42-66` |
| **Room** | Server-side state: `{ id, region, game_mode, tick_hz, players: HashMap<PlayerId, Player>, tick, snap_id }`. (No `session_key` field — removed.) | `aw-net-server/lib.rs:150-162` |
| **Player** | Server-side per-player state: `{ id, display, last_input_seq, last_seen, tokens, last_refill }`. | `aw-net-server/lib.rs:137-148` |
| **AppState** | Server-wide state: `{ rooms: Arc<parking_lot::Mutex<HashMap<RoomId, Room>>>, db: sled::Db, codec: Codec, signing_key: SigningKey, sig_failure_policy: SignatureFailurePolicy }`. Uses `parking_lot::Mutex` (`server/lib.rs:24`), not `std::sync::Mutex`. | `aw-net-server/lib.rs:164-173` |
| **ServerConfig / run_server / spawn_server** | `ServerConfig` (CLI-derived: listen addrs, TLS paths, db path, `signing_key`, `sig_failure_policy`) → `run_server(config)` (binary entry after arg-parse) → `spawn_server(config)` (binds listeners FIRST — the `127.0.0.1:0` ephemeral-port test seam — then spawns accept loops). | `aw-net-server/lib.rs:72-105, 206-307` |
| **MsgOutcome** | Internal result of handling one in-session message: `Continue` (keep open) or `Kick(&'static str)` (close with 1008 + run shared cleanup). | `aw-net-server/lib.rs:127-135` |
| **WireError** | `#[non_exhaustive]` + `#[must_use]` decode error enum: `ProtocolMismatch`, `Decode`, `InvalidSigningKey`. | `aw-net-proto/lib.rs:222-232` |
| **CNetworkClient** | ECS component: `{ player_id, last_acknowledged_input, pending_inputs }`. | `astraweave-net-ecs/lib.rs:13-19` |
| **CClientPrediction** | ECS component: `{ predicted_position: Vec3, prediction_error: Vec3 }`. | `astraweave-net-ecs/lib.rs:21-26` |
| **CNetworkAuthority** | ECS component: `{ authoritative_tick, connected_clients: HashMap<String, mpsc::UnboundedSender<ServerToClient>> }`. | `astraweave-net-ecs/lib.rs:28-33` |
| **NetworkSnapshot** | Distinct from `aw-net-proto`'s `ServerToClient::Snapshot`. ECS-layer snapshot type with `{ server_tick, entity_states: HashMap<u64, EntityState> }`. | `astraweave-net-ecs/lib.rs:35-40` |

### Terms to NOT confuse

- **`Snapshot` (aw-net-proto wire variant) vs. `NetworkSnapshot` (astraweave-net-ecs)**: The wire-format snapshot is `ServerToClient::Snapshot { id, server_tick, base_id, compressed, payload }` carrying an opaque byte payload. The ECS-layer `NetworkSnapshot` is what's *inside* the payload — it deserializes per-entity state. Distinct types in distinct crates.
- **`SigningKey` (32 bytes, both ends) vs. `[u8; 32]` sig vs. `input_frame_sig_payload` (the signed bytes)**: One shared symmetric `SigningKey` keys the HMAC at both client and server (held out-of-band, never on the wire). The `sig` field is the 32-byte HMAC-SHA256 tag the client puts on each `InputFrame`. `input_frame_sig_payload(seq, tick_ms, &input_blob)` is the byte range that is MAC'd — NOT the raw `input_blob` alone; the `seq` and `tick_ms` fields are inside the signed bytes. (Historical note: there was formerly a per-room 32-byte `SessionKey`, an 8-byte `session_key_hint` sent to the client, and a 16-byte XOR `sign16` tag — all three deleted in `561b20957`.)
- **HMAC sign vs. verify (one algorithm, both ends agree)**: The client computes `aw_net_proto::sign(&signing_key, &payload)` (`client/main.rs:101`) and the server computes `aw_net_proto::verify(&app.signing_key, &payload, &sig)` (`server/lib.rs:880, 1022`), where `payload = input_frame_sig_payload(seq, tick_ms, &input_blob)` on both sides. **These produce/check the same bytes.** A valid frame verifies; a forged or wrong-key frame is rejected and (default policy) kicked. (Historical note: pre-remediation the client signed with XOR `sign16` while the server verified HMAC-SHA256, so every verification failed — RESOLVED.)
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

- **Shared `SigningKey` (client ⟷ server):** A single 32-byte symmetric key is held by both ends out-of-band (never transmitted): the client loads it from `AW_SHARED_KEY` (64 hex) or `SigningKey::dev_default()`; the server from `--shared-key-hex` or `dev_default()`. The client signs each `InputFrame` (`aw_net_proto::sign`) and the server verifies (`aw_net_proto::verify`, constant-time) over the same `input_frame_sig_payload(seq, tick_ms, &input_blob)` bytes. A bad tag is kicked by default (Close 1008). (No per-room `session_key`/`session_key_hint` remains — deleted in `561b20957`; see §6 / Stage 6.)

### Documentation references with no code backing

- **None observed.** The `net/README.md` accurately describes the implemented modules. The former signature-scheme drift (README described XOR `sign16` as the MVP; client signed XOR while server verified HMAC; "anti-cheat" was `warn!`-only) is RESOLVED: the trio now signs and enforces HMAC-SHA256 end-to-end (kick-by-default), and `net/README.md` was updated to match (Net-Trio-Remediation W.4). See §6.

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-net-ecs/src/lib.rs` | ECS Plugin layer: 3 components + 2 Plugins + 4 systems + 2 async helpers + 4 inline tests | Active (Plugin layer dormant — no production consumer) | 437 lines. `#![forbid(unsafe_code)]`. |
| `astraweave-net-ecs/src/lib_temp.rs` | Near-duplicate of `lib.rs` minus `#![forbid(unsafe_code)]` | Orphan (not declared in any `mod` statement; not the crate's lib entry; effectively dead source) | 436 lines. Created 2025-11-19 in commit `54d15c9f2` ("added forest biome textures and assets, worked on renderer"). The commit title is misleading — the file is unrelated to forest biomes. See §6. |
| `astraweave-net-ecs/tests/mutation_resistant_comprehensive_tests.rs` | Mutation-resistance harness | Active (tests) | 341 lines, 27 tests. |
| `astraweave-net-ecs/benches/net_ecs_benchmarks.rs` | Criterion benches for serialization, snapshot construction, component-store ops | Active | 487 lines, 9 `bench_function` call sites. |
| `astraweave-net-ecs/benches/net_ecs_adversarial.rs` | Adversarial benches (worst-case shapes, edge inputs) | Active | 921 lines, 11 `bench_function` call sites. |
| `net/aw-net-proto/src/lib.rs` | Wire-protocol types + `encode_msg`/`decode_msg` + **canonical signing surface** (`SigningKey`, `sign`, `verify`, `hmac_sha256`, `input_frame_sig_payload`, `SIG_LEN`) + `new_room_id` + `WireError` | Active | 285 lines. `#![forbid(unsafe_code)]`. **`sign16`/`SessionKey` deleted** (`561b20957`); `InputFrame.sig` is now `[u8; 32]`. |
| `net/aw-net-proto/tests/mutation_resistant_comprehensive_tests.rs` | Mutation-resistance harness + RFC 4231 HMAC known-answer vectors | Active (tests) | 59 tests. |
| `net/aw-net-proto/benches/proto_bench.rs` | Criterion benches for codec encode/decode | Active | 13 `bench_function` call sites. |
| `net/aw-net-client/src/main.rs` | Standalone CLI client binary | Active | 193 lines. Demo input loop at 33 ms tick (~30 Hz). Signs each `InputFrame` via `aw_net_proto::sign` over `input_frame_sig_payload`; key from `AW_SHARED_KEY`/`dev_default`. Uses `tokio-tungstenite` with `native-tls` feature for wss. Carries the S2C-unsigned design note (`561b20957`/`eb9977b88`). |
| `net/aw-net-client/Cargo.toml` | Binary crate metadata | Active | Pulls `tokio-tungstenite = { version = "0.28", features = ["native-tls"] }` (line 9). |
| `net/aw-net-server/src/lib.rs` | **Server library**: `ServerConfig`/`run_server`/`spawn_server`, TLS+plain dual paths, matchmaking, sled, rate limiting, **HMAC verify-FIRST via the canonical proto surface**, `SignatureFailurePolicy { Kick, Warn }` | Active | ~1080 lines. Dual TLS/plain handlers (Stage 3 — ~400 lines of duplicated code). The verify/kick logic lives in `on_client_msg{,_tls}`. |
| `net/aw-net-server/src/main.rs` | Thin CLI wrapper: parse args (`--shared-key-hex`, `--sig-failure-policy`, `--disable-tls`, `--tls-cert`/`--tls-key`, `--ws-listen`/`--http-listen`/`--db-path`) → `ServerConfig` → `run_server` | Active | 116 lines. No logic beyond arg-parse (`066cd6cfd` lib/bin split). |
| `net/aw-net-server/tests/common/mod.rs` | Shared in-process test harness (spawns a live server on an ephemeral port via `spawn_server`, drives a real WS client) | Active (tests) | W.3.1 (`7029d7d7f`). |
| `net/aw-net-server/tests/family1_authenticated_round_trip.rs` | Family 1: authenticated round-trip (valid signing) | Active (tests) | 5 tests. |
| `net/aw-net-server/tests/family2_malformed_packets.rs` | Family 2: malformed/garbage packets | Active (tests) | 12 tests. |
| `net/aw-net-server/tests/family3_wrong_key_policy.rs` | Family 3: wrong-key inputs under Kick vs Warn policy | Active (tests) | 9 tests. |
| `net/aw-net-server/tests/family4_disconnect_paths.rs` | Family 4: disconnect / cleanup paths | Active (tests) | 9 tests. |
| `net/aw-net-server/tests/family5_tls_signature_path.rs` | Family 5: TLS verify/kick path (`2955cd14c`) | Active (tests) | 4 tests. |
| `net/aw-net-server/tests/w2b_fix1_regressions.rs` | W.2.b-fix1 regressions: graceful unknown-room + cleanup-preserving snapshot path (`9a3fc94e3`) | Active (tests) | 2 tests. |
| `net/aw-net-client/tests/family1_client_binary.rs`, `net/aw-net-client/tests/family3_client_binary_wrong_key.rs` | Client-binary families: valid-key round-trip and wrong-key behavior | Active (tests) | 2 + 2 = 4 tests. |
| `net/aw-net-server/Cargo.toml` | lib+bin crate metadata | Active | Includes `tokio-rustls = "0.25"`, `rustls = "0.22"`, `sled = "0.34"`, `axum = "0.8"`. HMAC now lives in `aw-net-proto` (`hmac`/`sha2`), not the server. **Note:** TLS stack version is one major behind the workspace `astraweave-net` crate which uses `tokio-rustls = "0.26"` and `rustls = "0.23"` (workspace-pin). See §6. |
| `net/aw-net-proto/Cargo.toml` | Library crate metadata | Active | Uses `bincode = { version = "2.0", features = ["serde"] }` and `postcard = { version = "1", features = ["alloc"] }`. |
| `net/README.md` | Integration guide | Active | Usage instructions. Updated in the Net-Trio-Remediation (W.4): HMAC-SHA256 signing is documented as implemented and enforced (client signs, server verifies constant-time, kicks by default), with the known limitations (no replay protection; S2C unsigned; generate real dev certs) noted. See §6. |
| `net/TLS_IMPLEMENTATION_SUMMARY.txt`, `net/TLS_TESTING_GUIDE.txt` | TLS notes | Active | Standalone text files at `net/` root. `TLS_IMPLEMENTATION_SUMMARY.txt` is a structured implementation changelog documenting the cert-loading functions, CLI flags (`--tls-cert`, `--tls-key`, `--disable-tls`), and dual-handler design (`handle_socket_tls`/`handle_socket`). Verified by direct read 2026-05-12. The line numbers and decisions in this doc match the current code. |
| `net/certs/dev/` | Dev TLS certs + generation script | Active | Referenced at `server/lib.rs:280` (warning points users to `generate_dev_cert.sh` when cert load fails). |
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
| `sign16` (XOR) vs. HMAC-SHA256 — client/server signature mismatch | (historical) `aw-net-proto` `sign16` vs. inline server HMAC | **RESOLVED** (`561b20957`+`79424389e`+`066cd6cfd`; refute `eb9977b88`) | *Historical:* the client computed the XOR `sign16` (16-byte tag) while the server verified HMAC-SHA256 (32-byte tag), so every verification failed (length + algorithm) and the server only `warn!`ed. *Now:* both ends sign/verify HMAC-SHA256 via the single canonical `aw-net-proto` surface (`SigningKey` + `sign`/`verify` over `input_frame_sig_payload`); the server verifies FIRST and kicks by default (`SignatureFailurePolicy::Kick` → Close 1008). `sign16`, `SessionKey`, and `session_key_hint` are deleted. See §7 Decision Log and Stage 6. |
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

- **Trap (RESOLVED — historical):** `aw-net-client` signed with `sign16` (XOR) while `aw-net-server` verified HMAC-SHA256, so every verification failed and the server only `warn!`ed.
  - **Why it was confusing:** Both sides referenced a `session_key`; the server had `hmac` + `sha2` in its `Cargo.toml`, the client didn't. Verification failed for two independent reasons (length: 16-byte XOR tag vs. 32-byte HMAC `OutputSize`; and algorithm), and the failure was log-only.
  - **What's true now (Net-Trio-Remediation `561b20957`+`79424389e`+`066cd6cfd`; refute `eb9977b88`):** there is ONE algorithm and ONE shared key. The client signs HMAC-SHA256 (`aw_net_proto::sign`) and the server verifies (`aw_net_proto::verify`, constant-time) over the same `input_frame_sig_payload(seq, tick_ms, &input_blob)`. `InputFrame.sig` is `[u8; 32]`. The server verifies FIRST and, under the default `SignatureFailurePolicy::Kick`, rejects the packet and closes the connection (1008) — no longer log-only. `sign16`, `SessionKey`, and `session_key_hint` are deleted.

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

### Decision: Canonical HMAC-SHA256 input signing, enforced end-to-end
- **Date:** Original server-side HMAC landed 2025-11-18 (commit `88434f3a2`, "security: fix critical vulnerabilities in network server (Priority 1)"), but was mismatched with the client. **Reconciled** in the Net-Trio-Remediation (2026-06): `561b20957` (W.1), `79424389e` (W.2.a), `066cd6cfd` (W.2.b); adversarial refute `eb9977b88` (W.5.3).
- **Status:** **Resolved.** Client and server both compute HMAC-SHA256 via the single canonical `aw-net-proto` surface; verification is enforced (kick-by-default).
- **Context:** Previously the server verified HMAC-SHA256 (32-byte tag) while the client signed with the XOR `sign16` (16-byte tag), so every `verify_slice` failed on length mismatch and the failure was `warn!`-only. The defect was a half-finished upgrade: the server was hardened first and the client was never updated to match.
- **Decision:** Move signing into `aw-net-proto` as the canonical surface (`SigningKey`, `sign`, `verify`, `hmac_sha256`, `input_frame_sig_payload`, `SIG_LEN = 32`). The client signs (`sign`) and the server verifies (`verify`, constant-time) over the SAME `input_frame_sig_payload(seq, tick_ms, &input_blob)` bytes. `InputFrame.sig` becomes `[u8; 32]`. The server verifies FIRST (before any per-player state mutation) and applies `SignatureFailurePolicy { Kick (DEFAULT), Warn }`. Delete `sign16`, `SessionKey`, and the `session_key_hint` fields. Split the server into lib+bin so the logic is integration-testable.
- **Alternatives considered:** Server-side `verify_truncated_left` to accept a 16-byte tag (rejected — would have kept two algorithms and a truncated, weaker MAC); a per-room key handshake (deferred — out of scope; the shared symmetric key is sufficient for C2S authenticity).
- **Consequences:**
  - A valid frame verifies; a forged or wrong-key frame is rejected and, by default, the client is kicked via a WebSocket Close (1008) routed through the existing cleanup path (player removed, empty room dropped). `Warn` retains the legacy log-and-continue for debugging.
  - The two ends can never drift on the signed byte range as long as both build it via `input_frame_sig_payload` (see Invariant 16).
  - Deliberate boundaries (NOT defects): no replay protection / nonce / sequence-freshness (HMAC proves authenticity, not freshness); server→client messages are unsigned (authoritative-server asymmetric trust — a shared symmetric key cannot authenticate S2C). See §11 and the findings doc.

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
| 3 | `InputFrame.sig` is a 32-byte HMAC-SHA256 tag (`SIG_LEN == 32`) | Yes (compile-time) | `aw-net-proto/lib.rs:10, 173`. (Replaces the former `session_key_hint = first 8 bytes` invariant — `session_key_hint`/`SessionKey` deleted in `561b20957`.) |
| 4 | `new_room_id()` returns 8 alphanumeric characters | Yes | `aw-net-proto/lib.rs:279-285`. |
| 5 | A `SigningKey` is exactly 32 bytes | Yes (compile-time) | Field type is a private `[u8; 32]` (`aw-net-proto/lib.rs:28`); `from_hex` rejects anything ≠ 64 hex chars (`:68-81`). |
| 6 | Room cap is 4 players (per region+game_mode) | Yes | `server/main.rs:310, 491`. |
| 7 | Empty room is removed on player disconnect | Yes | `server/main.rs:436-438, 617-619`: `if room.players.is_empty() { rooms.remove(&rid); }`. |
| 8 | `aw_net_proto::encode_msg(Codec::PostcardLz4, ...)` is followed by `decode_msg(Codec::PostcardLz4, ...)` | Test-enforced | Server (`server/main.rs:153`) and client (`client/main.rs:69`) both pin PostcardLz4. ECS Plugin layer (`net-ecs/lib.rs:179, 237, 279`) pins Bincode independently. Codec mismatch produces `WireError::Decode`. |
| 9 | Per-connection ticking advances `room.tick` per connection (not per tick globally) | Yes (current behavior) | `server/main.rs:631` inside `build_snapshot` increments `room.tick += 1` per call. Each `handle_socket{,_tls}` runs its own `sleep(tick_dt)` loop and calls `build_snapshot` independently. With N connected players in one room, `room.tick` advances N times per real second per Hz unit. [INFERRED — this is what the code reads; whether the design intends per-room or per-connection tick semantics is decisional.] |
| 10 | HMAC-SHA256 verification runs FIRST and gates processing per `SignatureFailurePolicy`: under `Kick` (DEFAULT) a failed tag rejects the packet and disconnects (Close 1008); under `Warn` it logs and processes anyway | Yes | `server/lib.rs:879-893 (plain), 1021-1035 (TLS)`: `if !aw_net_proto::verify(...) { match policy { Kick => return MsgOutcome::Kick(..), Warn => warn!(..) } }` — verify precedes the rate-limit/`last_input_seq` updates. (Was previously `warn!`-only and did NOT terminate; changed in `066cd6cfd`.) |
| 11 | Token bucket refill rate is 8 tokens/sec; bucket capacity is 60; cost per message is 1 | Yes (compile-time) | `server/main.rs:683-685, 811-813` constants. |
| 12 | `tick_hz` defaults to 30 | Yes | `server/main.rs:301, 482` (initial), `:321, 502` (Room construction). |
| 13 | `--disable-tls` is forbidden in release builds | Yes (compile-time) | `server/main.rs:120-123` `#[cfg(not(debug_assertions))]`. |
| 14 | `astraweave-net-ecs::server_snapshot_system` always sends snapshots with `entity_states: HashMap::new()` | Yes | `net-ecs/lib.rs:174`. Stub system. |
| 15 | Compression of snapshots in `aw-net-server::build_snapshot` is always on (`compressed: true`) | Yes | `server/lib.rs:855`. |
| 16 | Both ends build the MAC'd byte range via `input_frame_sig_payload(seq, tick_ms, &input_blob)` — the canonical signed bytes are `seq.to_le_bytes() ++ tick_ms.to_le_bytes() ++ input_blob`, never hand-rolled | Test-enforced (families 1/3/5) | Definition `aw-net-proto/lib.rs:139-145`; client signs over it (`client/main.rs:100-101`), server verifies over it (`server/lib.rs:879, 1021`). A single shared definition is what prevents signed-byte-range divergence (the original defect). |

---

## 9. Performance & Resource Profile

### Hot paths

- **Per-connection per-tick `build_snapshot`** (`server/main.rs:625-657`) — runs `tick_dt` apart (default ~33 ms). Cost: mutex acquire on `app.rooms`, increment two `u32`/`u64` fields, postcard serialize a tiny `DemoState { tick: u64 }`, lz4 compress. Sub-millisecond by inspection.
- **Per-message `decode_msg`** in the per-connection select loop (`server/main.rs:409, 590`) — lz4 decompress + postcard deserialize.
- **HMAC-SHA256 verify** per `InputFrame` (`aw_net_proto::verify`, called at `server/lib.rs:880, 1022`) — runs FIRST, before rate-limit/state updates; fast for small payloads but allocates a fresh `Hmac<Sha256>` and the `input_frame_sig_payload` `Vec` per message.
- **ECS systems run once per stage tick.** None hit the network synchronously; all encode/decode work in the ECS path is in-process (`encode_msg(Codec::Bincode, &snapshot)` at `net-ecs/lib.rs:179`, with no I/O).

### Cold paths

- **TLS handshake** (`server/main.rs:217-223`) — milliseconds per connection.
- **Matchmaking room scan** (`server/main.rs:309-310, 490-491`) — linear scan of `rooms` HashMap; rooms cap not enforced; current scale assumed small.
- **Signing-key load** — the 32-byte `SigningKey` is parsed once at process startup (`from_hex` of `AW_SHARED_KEY` / `--shared-key-hex`, or `dev_default()`); there is no per-room key generation anymore (the former per-room `SessionKey::random()` was removed in `561b20957`).
- **`net_ecs_adversarial.rs`** (921 lines, 11 bench cases) — explicitly worst-case payload shapes for serialization.

### Resource ownership

- **`AppState.rooms: Arc<parking_lot::Mutex<HashMap<RoomId, Room>>>`** — server-wide lock (`parking_lot` flavor, not `std::sync`). Acquired per-message and per-tick. Cloned (the `Arc`) into each connection task.
- **`sled::Db`** — opened at `aw_net_server_db/`; never read or written in the visible code. Stays open for the server's lifetime.
- **`SigningKey` (private `[u8; 32]`)** — one process-wide instance held on `AppState.signing_key`, cloned (cheaply) into each connection task's `AppState`. Not per-room.
- **Per-connection `tokio::spawn`** — one spawn per WebSocket accept; each task owns its `WebSocketStream`, a clone of `AppState`, and a clone of the `TlsAcceptor` (in TLS mode).
- **HMAC instance** — `aw_net_proto::verify` constructs a fresh `Hmac<Sha256>` per `InputFrame`; it lives only for the verification check.
- **ECS Plugin components** — owned by ECS entity storage. `CNetworkAuthority.connected_clients: HashMap<String, mpsc::UnboundedSender<ServerToClient>>` keeps a strong reference to every connected client's channel; if a client disconnects without cleanup, the sender is retained — potential leak source [INFERRED — no cleanup path observed in the ECS systems].

---

## 10. Testing & Validation

- **Unit tests:** Inline `#[cfg(test)] mod tests` in `astraweave-net-ecs/src/lib.rs`: 4 tests (`client_input_processing`, `client_reconciliation`, `server_snapshot_generation`, `network_integration`).
- **Integration tests:**
  - `astraweave-net-ecs/tests/mutation_resistant_comprehensive_tests.rs`: 27 tests.
  - `net/aw-net-proto/tests/mutation_resistant_comprehensive_tests.rs`: 59 tests (includes RFC 4231 HMAC known-answer vectors, `420a6f61b`).
  - **Net-Trio-Remediation regression net (`7029d7d7f`/`a2b494942`/`0e702738e`/`68a9a1936`/`2955cd14c`):** server families 1-5 + w2b_fix1 = **41 tests** (f1 round-trip 5, f2 malformed 12, f3 wrong-key policy 9, f4 disconnect 9, f5 TLS path 4, w2b_fix1 2), driven through a shared in-process harness (`tests/common/mod.rs`) that spawns a live server on an ephemeral port via `spawn_server`; client families 1+3 = **4 tests**.
- **Total tests in this subsystem:** **104 tests in the standalone trio** (proto 59 + server 41 + client 4), plus the ECS Plugin layer's 27 (`net-ecs` integration) + 4 inline = 31. (Counting only files that actually compile; `lib_temp.rs`'s 4 tests are in orphan source and do not run.)
- **Adversarial refute pass:** a §5 (HARDENED-skeptic) refute pass over the remediation ran and **CONVERGED** (`eb9977b88`).
- **Standalone binaries are now integration-tested**, not just manually smoke-tested. The shipped client binary over TLS remains untested (native-tls rejects the self-signed dev cert — see §11); the server's TLS verify/kick path IS covered (family 5).
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

- **Why did the client sign with `sign16` (XOR) while the server verified with HMAC-SHA256? — RESOLVED (Net-Trio-Remediation, `561b20957`+`79424389e`+`066cd6cfd`).** It was an in-progress upgrade: the server was hardened to HMAC-SHA256 first (commit `88434f3a2`) and the client was never updated, so every verification failed on length+algorithm and the server only `warn!`ed. The trio is now unified on a single canonical `aw-net-proto` HMAC surface, signed end-to-end and enforced (kick-by-default). `sign16`/`SessionKey`/`session_key_hint` deleted.

- **Known deliberate boundaries of the remediation (NOT open defects).** These were fenced out of scope intentionally and are recorded for future session-security work; full rationale in `docs/audits/net_trio_signature_remediation_findings_2026-06.md`:
  1. **No replay protection / nonces / sequence-number freshness.** HMAC proves a frame is authentic (signed by a holder of the shared key), not fresh — a captured valid frame can be replayed. Mitigating this needs nonces or a monotonic-sequence auth window.
  2. **Server→client messages are not signature-verified.** This is an authoritative-server asymmetric-trust model: the client has no independent ground truth to validate server state against, and a *shared symmetric* key cannot meaningfully authenticate S2C anyway (every client in a room holds the same key). Meaningful S2C auth requires asymmetric server keys or per-session key exchange (a handshake — out of scope for this trio). See the design note in `client/main.rs:114-127`.
  3. **The shipped client binary over TLS is untested** — `native-tls` rejects the self-signed dev cert (a cert-validation concern, not a signing concern). The *server's* TLS verify/kick path IS tested (family 5).

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
- ~~The signature scheme is unified between client and server~~ **DONE (Net-Trio-Remediation, `561b20957`+`79424389e`+`066cd6cfd`)** — client+server now agree on canonical HMAC-SHA256, enforced kick-by-default (§6 row RESOLVED, §7 Decision Resolved, §11 question RESOLVED). Update again only if the signing surface or `SignatureFailurePolicy` changes.
- `lib_temp.rs` is either declared as a module or removed (§5 row, §6 row, §11 third question).
- `sled::Db` gains actual read/write call sites (§4 sled row, §6 sled row, §11 sled question).
- A workflow is added that runs `cargo test` for any of these four crates (§10 CI presence note).
- The standalone binaries gain further test coverage beyond the Net-Trio-Remediation families (§10 regression-net note; e.g. closing the untested client-binary-over-TLS gap).
- A real production consumer of `astraweave-net-ecs` lands (§4 downstream table, §11 first question).
- TLS stack versions are bumped to align with workspace (§6 TLS-version row).

**Verification process:**
- `rg 'pub fn|pub struct|pub enum|pub trait' astraweave-net-ecs/src/lib.rs net/aw-net-proto/src/lib.rs` should match §3 vocabulary surface.
- `cargo tree -p astraweave-net-ecs --depth 1` should list `aw-net-proto`, `astraweave-ecs`, `astraweave-core`, `postcard`, `lz4_flex`, `crc32fast`, `glam`, `bincode`, `tokio`, `futures-util`, `tokio-tungstenite`, `anyhow`, `serde`, `serde_json`.
- `cargo tree -p aw-net-server --depth 1` should list `aw-net-proto`, `tokio`, `axum`, `hyper`, `tokio-tungstenite`, `tungstenite`, `futures`, `serde`, `serde_json`, `anyhow`, `thiserror`, `tracing`, `tracing-subscriber`, `parking_lot`, `uuid`, `time`, `sled`, `postcard`, `lz4_flex`, `tokio-rustls`, `rustls`, `rustls-pemfile`. (`hmac`/`sha2` now live in `aw-net-proto`, not the server.)
- `rg 'use astraweave_net_ecs\|use aw_net_proto' --type rust -g '!*test*' -g '!benches/*'` should match §4 consumers; new consumers must be added.
- `grep -c '#\[test\]\|#\[tokio::test\]'` across the four crates' test files should total ≥ 104 (proto 59 + server 41 across families 1-5 + w2b_fix1 + client 4; the test-count invariant grows, never shrinks). The standalone server/client now carry their own `tests/` families (W.3.1-3.4, W.5.2).
- Stamp the new commit hash and date in the metadata table.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. There are TWO integration paths sharing `aw-net-proto`: the standalone `net/aw-net-{client,server}` binaries (PostcardLz4, TLS-by-default, matchmaking, enforced HMAC-SHA256 signing), and `astraweave-net-ecs` (ECS Plugin layer with stub systems, Bincode codec, plain `ws://`). They do not currently interoperate.
2. **`astraweave-net-ecs` has no production consumers.** The declared dep in `astraweave-stress-test/Cargo.toml:20` is unused. The Plugin layer is dormant code.
3. **HMAC verification on the server is ENFORCED** — the server verifies each `InputFrame` FIRST (constant-time, via `aw_net_proto::verify` over `input_frame_sig_payload`) and, under the default `SignatureFailurePolicy::Kick`, rejects the packet and closes the connection (1008). The client signs HMAC-SHA256 with the same shared `SigningKey`. `Warn` policy restores the legacy log-and-continue for debugging. (Pre-remediation the client signed XOR `sign16` and the server only `warn!`ed — fixed.)
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
- **Assuming the HMAC verification is observed-only.** It isn't anymore — under the default `Kick` policy a bad signature rejects the packet and closes the connection (1008). It only logs-and-continues under the explicit `Warn` policy. Verification also runs FIRST, before any state mutation, so don't reorder it after the rate-limit/`last_input_seq` updates.
- **Trusting `app.db` to persist anything.** It doesn't — no reads or writes go through it.
- **Adding `unsafe` to `astraweave-net-ecs/src/lib.rs` or `aw-net-proto/src/lib.rs`.** Both crate roots carry `#![forbid(unsafe_code)]`.

---

## Appendix B: Historical context

The standalone trio (`net/aw-net-{proto,client,server}`) was introduced as a single squashed PR on **2025-09-09** in commit `cc9a7e3e3` ("Implement production-ready enhanced networking layer with server authority, client prediction, and matchmaking (#58)"). This predates the ECS Plugin layer.

The ECS Plugin layer (`astraweave-net-ecs`) was added on **2025-10-01** in commit `08befc6ec` ("phase 6 implementation") — roughly three weeks after the standalone trio. By that point, the standalone server had already adopted HMAC verification while the client still computed the XOR `sign16` — the mismatch that caused every verification to fail. This was finally reconciled in the **Net-Trio-Remediation (2026-06)**: canonical HMAC-SHA256 signing in `aw-net-proto`, adopted by both client and server, enforced kick-by-default; `sign16`/`SessionKey`/`session_key_hint` deleted (`561b20957`+`79424389e`+`066cd6cfd`).

The `lib_temp.rs` orphan was added on **2025-11-19** in commit `54d15c9f2` ("added forest biome textures and assets, worked on renderer"). The commit title gives no clue about why a temp file was added to the netcode crate — likely an unintentional checkin during a renderer-focused commit.

The older `astraweave-net` snapshot subsystem (documented separately in `docs/architecture/net.md`) was created 2025-09-05 and remains in place. Per `net/README.md:90-92`, the two networking systems are intended to coexist long-term.

The `aw_net_server_db/` directory is created on first server run by sled's `Db::open` call. Its presence in git (`conf` 62 B, `db` 96 B) indicates someone ran the server locally and committed the artifacts; the directory is not in any `.gitignore`.
