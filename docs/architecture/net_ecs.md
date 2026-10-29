---
schema_version: 1
trace_id: net_ecs
title: "Net-ECS (ECS Plugin + Standalone Matchmaking Server)"
description: "Net-ECS + standalone matchmaking"
primary_crate: astraweave-net-ecs
domain: networking
lifecycle_status: unknown
integration_status: unknown
summary: "Working tests but ECS Plugin layer is dormant — no production consumer; stress-test declares dep but doesn't import. Wire format mismatch with standalone server. net_ecs.md §1"
owns: [astraweave-net-ecs, aw-net-client, aw-net-proto, aw-net-server]
doc_version: "1.16"
last_verified_commit: 0a2d93ea
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
| **Document version** | 1.16 |
| **Last verified against commit** | `0a2d93ea` |
| **Last verified date** | 2026-10-29 |
| **Status** | Active (mixed: production-grade standalone binary; dormant ECS Plugin layer). **Standalone-trio input signing is real and enforced:** client and server both compute HMAC-SHA256 over the canonical `aw-net-proto` signing surface; the server verifies FIRST (before any per-player state mutation) and kicks by default on failure. The historical `sign16`/XOR-vs-HMAC mismatch is RESOLVED (Net-Trio-Remediation, see revision 1.3). |
| **Revision history** | 1.16 (2026-10-29): Verified at `0a2d93ea`. Cite correction on the 64/s server. `handle_socket` is `:472-744` and `handle_socket_tls` is `:461-469`. `sled::open` is `:313-314`, default `db_path` is `:166`, and `AppState.db` is `#[allow(dead_code)]` at `:254-255`. `load_certs` is `:263`, `load_private_key` is `:270`, and the generation-script warning is `:360`. `accept_loop_tls` is `:390` (`accept_hdr_async` `:409`) and `accept_loop_plain` is `:423` (`:428`). `HelloAck` is `:483`. The player insert is `:588-601`, `MatchResult` is `:631-638`, and `JoinAccepted` is `:639-647`. `verify` is `:948`. Warn charges and does not apply (`:956-958`). Text and Pong are charged (`:729`). `Player` is `:215-231`, `Room` is `:234-248`, and `AppState` is `:251-260`. `expired_drops_leave_the_window_and_one_more_stays_open` returns Kick if the drop-window `pop_front` is removed. 1.15 text is unchanged. 1.15 (2026-10-29): Verified at `4e2e7352`. The bucket refills at 64/s with capacity 64, and a new player starts full (`lib.rs:808-809`, insert `:597`). InputFrame, protocol Ping, Ack, Hello, FindOrCreate, JoinRoom, undecodable binary, WebSocket Ping, Text, and Pong are charged. WebSocket Close is not. HMAC verify runs before any charge (`:944-966`). Kick returns before a charge. Warn charges a bad tag and does not apply it (`:956-958`). Thirty-two unpaid messages inside one second close 1008 (`:813-814`, `:853`). `RateLimited` is not sent. A paid WebSocket Ping is one Pong; a dropped Ping still emits the Pong tungstenite queued while parsing. `send_policy_close` (`:881`) drains before and after the Close. `prequeued_frames_advance_exactly_one_tick_per_period` keeps the socket readable and requires `server_tick` to step by 1. Putting `ws.next()` first under `biased` (`:665`) delivered 1 snapshot in 500 ms. `room.tick += 2` and `room.tick *= 1` fail that step. Removing `biased` skipped a tick on 6 of 8 two-second runs and on 5 of 8 half-second runs; one window does not promise the skip. 1.14 (2026-10-29): Verified at `2b3347ae`. Server bytes are still `f931c184`. Invariant 9's ready-socket case sends one protocol Ping every 20 ms from its own task. Ping is uncharged here (`lib.rs:821-823` sends `Pong` and leaves the bucket alone). The bucket refills at 8/s and charges only `InputFrame` (`:795-797`). 50/s was picked to fit #526's 64/s refill and to stay shorter than `tick_dt`, so a fresh per-connection `sleep` is cancelled. Measured against this server's 8/s refill, 50/s would be over the limit if Ping were charged. Fewer than half the due snapshots in 500 ms fails `paid_frame_inside_limit_still_receives_snapshots`. Restoring the biased `select!` with a fresh `sleep(tick_dt)` delivered 1 snapshot in 500 ms, three runs out of three. One ping per tick on the same task still receives snapshots. 1.13 (2026-10-29): Verified at `74a9ef0b`. Server bytes are still `f931c184`. Invariant 9's ready-socket case sends WebSocket text, which this server does not charge, so a frame is ready on every poll. Fewer than half the due snapshots in 500 ms fails `text_flood_still_receives_snapshots`. One protocol Ping per tick also receives snapshots. 1.12 (2026-10-29): Verified at `f931c184`. One `spawn_room_tick` (`lib.rs:693-712`) publishes snapshots for a room. `FindOrCreate` starts it once (`:519`). `build_snapshot` (`:714-746`) still increments `room.tick` once per call (`:720`), and only that owner calls it. The connection selects `snaps.changed()` first under `biased` (`:638`). `Room` carries `snap_tx` (`:223-225`, struct `:212-226`). Four connections advance `server_tick` once per period. Regression: `net/aw-net-server/tests/room_tick_owner.rs`. Line cites in 1.11 describe the per-connection `sleep` at `de6f2725`. <br><br>1.11 (2026-10-28): Verified at `de6f2725`. Plain and TLS sessions share `handle_socket`. `handle_socket_tls` delegates to it. `PlayerMembership` removes the player, and an empty room, on every exit from that handler, including a failed `MatchResult` or `JoinAccepted` send. The trailing cleanup block, `send_tls`, `recv_tls`, and `on_client_msg_tls` are gone. Line cites in 1.10 describe the duplicated handlers at `28ef548c`. <br><br>1.10 (2026-10-28): Line cites were read against main 28ef548c. HelloAck and JoinAccepted are not the same lines. The player insert happens before either send. Warn writes last_input_seq and last_seen only while the token bucket stays non-negative. The client TLS feature is rustls-tls-native-roots. spawn_server refuses the published key outside debug and test builds. --sig-failure-policy parses on every build. <br><br>1.9 (2026-10-28): A release server does not see the client's key. A published-key client joins; the first InputFrame is Close 1008 under Kick, and under --sig-failure-policy warn the connection can stay up after a bad tag (see 1.10 for bucket-gated last_input_seq/last_seen). A debug server keeps dev_default() when --shared-key-hex is absent. The client TLS feature is rustls-tls-native-roots. Checked against main b6f6c654. <br><br>1.8 (2026-10-27): `decode_msg` for `Codec::Bincode` uses `standard().with_limit::<MAX_LZ4_DECLARED_OUTPUT>()` (4 MiB). The parent `99382af7` uses `standard()` with no limit, so a length prefix allocates before the payload bytes exist. The LZ4 declared-size cap is unchanged. 1.7 (2026-10-16): `astraweave-net-ecs` opts into `tokio` feature `net`. The workspace dependency does not enable it.<br><br>1.6 (2026-10-15): `aw-net-server` uses rustls 0.23 with the ring provider. Certificate PEM is parsed with `rustls-pki-types`.<br><br>1.5 (2026-10-15): `/healthz` and `/regions` require `ServerConfig.admin_token` as a bearer token. `None` rejects every admin request. The listener still binds so `RunningServer.http_addr` stays valid. CLI flag `--admin-token`. 1.4 (2026-10-15): `ServerConfig::default` binds `127.0.0.1`. `spawn_server` refuses the published dev signing key outside debug and test builds. `decode_msg` rejects an LZ4 declared output above 4 MiB. 1.3 (2026-06-10): **Net-Trio-Remediation closeout.** The standalone trio's signature defect — client signed with the XOR `sign16` (16-byte tag) while the server verified HMAC-SHA256 (32-byte tag), so every verification failed and the server only `warn!`ed — is FIXED and ENFORCED. `aw-net-proto` now exposes a canonical signing surface (`SigningKey` with a private 32-byte field + redacted `Debug`; `sign`/`verify` (constant-time via `Mac::verify_slice`); `hmac_sha256`; `input_frame_sig_payload(seq, tick_ms, blob)`; `SIG_LEN = 32`). The `sign16` XOR stub, the `SessionKey` type, and the `session_key_hint` fields on `MatchResult`/`JoinAccepted` are DELETED; `InputFrame.sig` is now `[u8; 32]`. Both client (sign) and server (verify) build the MAC'd bytes via the single shared `input_frame_sig_payload`. The server is now lib+bin (`aw-net-server/src/lib.rs` ~1080 lines holds all logic; `src/main.rs` is a thin arg-parse → `run_server`). Verification runs FIRST in both plain and TLS handlers and applies `SignatureFailurePolicy { Kick (DEFAULT), Warn }`: Kick sends a WebSocket Close (code 1008) and routes the disconnect through the existing cleanup path; Warn retains legacy log-and-continue. Regression net: 104 tests (proto 59, server 41 across families 1-5 + w2b_fix1, client 4); a §5 adversarial refute pass ran and CONVERGED. Commit ledger: `561b20957` (W.1 canonical HMAC; stub deleted), `79424389e` (W.2.a client adopts), `066cd6cfd` (W.2.b server verifies + policy + lib/bin split), `9a3fc94e3` (W.2.b-fix1 graceful unknown-room + cleanup-preserving snapshot path), `7029d7d7f`/`a2b494942`/`0e702738e`/`68a9a1936` (W.3.1-3.4 test families 1-4), `420a6f61b` (W.5.1 SigningKey encapsulation + RFC 4231), `2955cd14c` (W.5.2 TLS test family), `eb9977b88` (W.5.3 S2C trust note). Known deliberate boundaries (NOT open defects): replay protection / nonces / sequence-number freshness is not implemented (HMAC proves authenticity, not freshness — a captured valid frame can be replayed); server→client messages are not signature-verified (authoritative-server asymmetric trust — a shared symmetric key cannot meaningfully authenticate S2C); the shipped client binary over TLS is untested (rustls cert validation rejects the self-signed dev cert — a cert-validation concern), though the server's TLS verify/kick path IS now tested. Full audit: `docs/audits/net_trio_signature_remediation_findings_2026-06.md`.<br><br>1.2 (2026-05-12): Deep investigation pass. Enriched §11 Open Questions 1, 4, 8, 9 with comprehensive factual context. Recovered creation commit for `astraweave-net-ecs` dep in `astraweave-stress-test` (commit `08befc6ec` — same as net-ecs's own birth commit). Verified Q8 dual-TLS-version situation factually via `Cargo.lock` (both `tokio-rustls 0.25.0` and `tokio-rustls 0.26.4` are present) and via `cargo tree` (0.26 comes from reqwest→hyper-rustls used by many crates including `astraweave-llm`, `astraweave-assets`; 0.25 is exclusive to `aw-net-server`). Q4 factual finding: empty `entity_states: HashMap::new()` was present from the crate's birth commit `08befc6ec` (2025-10-01) — stub from day one. Resolved the last `[INFERRED]` marker (§5 TLS_IMPLEMENTATION_SUMMARY.txt content). Added new §6 row: a third copy of `lib.rs`-like content exists at `archive/temp_files/temp/temp_lib.rs` (a `use aw_net_proto` archive).<br><br>1.1 (2026-05-12): Verification pass. Corrected §2 Stage 6 HMAC mechanism — `hmac::verify_slice` (via `digest-0.10.7/src/mac.rs:168-179`) strict-rejects any tag length ≠ `OutputSize`; the previous `[INFERRED]` "truncated comparison" claim was wrong. The real mechanism: `Sha256::OutputSize == 32`, sig is 16 bytes, so `verify_slice` returns `MacError` immediately on length mismatch — **all client signatures fail length validation before any byte comparison happens**. Resolved §7 Decision 5 Date marker — HMAC verification landed in commit `88434f3a2` (2025-11-18, "security: fix critical vulnerabilities in network server (Priority 1)"). Resolved §7 Decision 7 Date marker — TLS-by-default in release added in commit `4889a9a33` (2025-11-13, "feat: Integrate astraweave-security…"). Noted that `parking_lot::Mutex` (not `std::sync::Mutex`) is used in §3 and §9. |
| **Owner notes** | As of 1.16, current line cites match `0a2d93ea`. Drops older than one second leave the kick window before the next unpaid message. Warn charges a bad tag and does not apply it. Text and Pong are charged. As of 1.15, the bucket is 64/s with capacity 64. A paid message does not clear the drop window. Thirty-two unpaid messages in one second close 1008. Protocol Ping is charged, and one ping every 20 ms stays under the refill. The 33 ms demo client pays every frame. `prequeued_frames_advance_exactly_one_tick_per_period` keeps a readable socket from skipping the room tick. As of 1.14, the ready-socket check is one protocol Ping every 20 ms, read on another task. Ping is uncharged here (`lib.rs:821-823`). The 50/s rate was picked to fit #526's 64/s refill. As of 1.12, each room has one tick owner. Connections subscribe to `snap_tx` and do not call `build_snapshot`. As of 1.11, `aw-net-server` has one connection handler. `handle_socket_tls` calls `handle_socket`. `PlayerMembership` removes the player when that task leaves, so a failed join send cannot leave a ghost. Two distinct integration paths share `aw-net-proto`: (1) the standalone `aw-net-server` / `aw-net-client` binaries communicate end-to-end over WSS with HMAC-SHA256 input signatures, token-bucket rate limiting, sled persistence, and matchmaking. As of revision 1.3 the signing is real and enforced: client and server agree on bytes via the canonical proto surface (`SigningKey` + `sign`/`verify` + `input_frame_sig_payload`), the server verifies FIRST, and the default `SignatureFailurePolicy::Kick` closes the connection (Close 1008) on a bad tag. (2) `astraweave-net-ecs` provides ECS Plugin scaffolding (`NetworkClientPlugin`, `NetworkServerPlugin`, components, simulation-stub systems) — but **workspace grep finds zero `use astraweave_net_ecs` outside the crate's own tests and benches**, including in `astraweave-stress-test` which declares the dep in `Cargo.toml`. The ECS layer is currently dormant code. None of these crates appear in any `.github/workflows/*.yml` as of `a2474c5b7`. |

---

## 1. Executive Summary

**What this system does:**
Provides two coexisting integration paths over a shared binary wire protocol (`aw-net-proto`):

1. **Standalone binary trio** (`net/aw-net-{proto,client,server}`): A production-style multiplayer server with axum HTTP admin endpoints, region-aware matchmaking, room-based session management, sled-backed persistence, token-bucket rate limiting, **enforced HMAC-SHA256 input signing**, and TLS-by-default WebSocket. The client signs each `InputFrame` with HMAC-SHA256 over the canonical `input_frame_sig_payload(seq, tick_ms, blob)`, and the server verifies it FIRST (before any per-player state mutation) via the same shared `aw-net-proto` surface — kicking by default (`SignatureFailurePolicy::Kick` → Close 1008) on a bad tag. Demo client connects with `wss://`, joins a room via `FindOrCreate`, then streams `InputFrame` messages at 30 ms intervals. (Before the Net-Trio-Remediation, the client signed with the XOR `sign16` while the server verified HMAC-SHA256, so every verification failed and the server only `warn!`ed — see §6/§7, now RESOLVED.)

2. **ECS Plugin layer** (`astraweave-net-ecs`): A library that registers `NetworkClientPlugin` / `NetworkServerPlugin` with the `astraweave-ecs::App` and adds four systems (`client_input_system`, `client_reconciliation_system`, `server_snapshot_system`, `server_input_processing_system`) plus three components (`CNetworkClient`, `CClientPrediction`, `CNetworkAuthority`) and async helpers (`connect_to_server`, `start_network_server`). The systems contain simulation stubs (e.g., literal `prediction.predicted_position.x += 0.1` placeholder for prediction).

**Why it exists:**
Per `net/README.md:5-9`, the three crates provide multiplayer capabilities, layered on the older `astraweave-net` snapshot system. The standalone trio is the user-facing artifact; `astraweave-net-ecs` is the ECS adaptation layer intended to plug the protocol into a game's ECS world.

**Where it primarily lives:**
- `astraweave-net-ecs/src/lib.rs` — 437 lines. ECS Plugin + components + four systems + two async helpers + 4 inline tests.
- `astraweave-net-ecs/src/lib_temp.rs` — 436 lines. Near-duplicate of `lib.rs` (see §6).
- `net/aw-net-proto/src/lib.rs` — 321 lines. `ClientToServer` / `ServerToClient` enums, `Codec` (PostcardLz4 / Bincode), `encode_msg` / `decode_msg`, the **canonical signing surface** (`SigningKey`, `sign`, `verify`, `hmac_sha256`, `input_frame_sig_payload`, `SIG_LEN = 32`), `new_room_id`, `PROTOCOL_VERSION = 1`, `WireError`. **The `sign16` XOR stub, the `SessionKey` type, and the `session_key_hint` fields are DELETED** (Net-Trio-Remediation `561b20957`).
- `net/aw-net-server/src/lib.rs` — ~1357 lines. All server logic: `ServerConfig` / `run_server` / `spawn_server`, one generic connection handler (`handle_socket`; `handle_socket_tls` delegates), `PlayerMembership` cleanup, matchmaking, rooms, sled DB, axum HTTP admin, **HMAC-SHA256 verification via the canonical proto surface (verify-FIRST)**, `SignatureFailurePolicy { Kick, Warn }`, token-bucket rate limiting.
- `net/aw-net-server/src/main.rs` — 129 lines. Thin CLI wrapper: parse args (`--shared-key-hex`, `--sig-failure-policy`, `--disable-tls`, `--tls-cert`/`--tls-key`, `--ws-listen`/`--http-listen`/`--db-path`) into a `ServerConfig`, then `run_server`.
- `net/aw-net-client/src/main.rs` — 193 lines. Standalone CLI client demo. Signs each `InputFrame` with `aw_net_proto::sign` over `input_frame_sig_payload`; key from `AW_SHARED_KEY` (64 hex), or `SigningKey::dev_default()` when that variable is unset, on every build (`:19-25`).
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
    │ tokio_tungstenite::connect_async("wss://127.0.0.1:8788")            (client/main.rs:34)
    │ Default: wss:// (TLS via rustls-tls-native-roots feature; client/Cargo.toml:9)
    │
    ▼
[WebSocket handshake]
    │
    │ ClientToServer::Hello { protocol: PROTOCOL_VERSION (= 1) }          (client/main.rs:39-45)
    │   ↓ encode_msg(Codec::PostcardLz4, &msg)                            (proto/lib.rs:280; client send helper :190)
    │   ↓ Message::Binary(bytes)
    │
    ▼
[net/aw-net-server/src/lib.rs — handle_socket]                            (lib.rs:472)
    │
    │ handle_socket_tls delegates to handle_socket                         (lib.rs:461-469)
    │ recv (message type, then socket)
    │ Branches on hello.protocol:
    │   == 1   → ServerToClient::HelloAck { protocol }                    (lib.rs:483)
    │   != 1   → ServerToClient::ProtocolError { msg }; return            (lib.rs:485-494)
    │
    ▼
[Matchmaking phase]
    │
    │ ClientToServer::FindOrCreate { region, game_mode, party_size: 1 }   (client/main.rs:48-55)
    │
    ▼
[Server room logic]                                                       (lib.rs:509-647)
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
    │    shared SigningKey. The client uses AW_SHARED_KEY or dev_default() on every build. A debug server keeps dev_default() when --shared-key-hex is absent (net/aw-net-server/src/main.rs:111-117); only the release CLI requires that flag (:103-109). Release spawn_server refuses the published key at boot (lib.rs:307-312; `published_dev_key_allowed` at aw-net-proto/lib.rs:114-116). `--sig-failure-policy` parses on every build (main.rs:72-77).)
    │
    │ Unsigned player insert, then PlayerMembership                        (lib.rs:588-611)
    ├── ServerToClient::MatchResult { room_id }                             (lib.rs:631-638)
    └── ServerToClient::JoinAccepted { room_id, player_id, tick_hz }        (lib.rs:639-647)
    │   A failed send returns with `?`. PlayerMembership still runs.

[Per-connection game loop]                                                (lib.rs:663-741)
    │
    │ tokio::select! biased:
    │
    ├── ws.next() → ClientToServer::InputFrame { seq, tick_ms, input_blob, sig: [u8;32] }
    │     ↓ on_client_msg                                                  (lib.rs:926)
    │     ↓ VERIFY FIRST (before any state mutation):                     (verify lib.rs:948)
    │       · payload = aw_net_proto::input_frame_sig_payload(seq, tick_ms, &input_blob)
    │       · aw_net_proto::verify(&signing_key, &payload, &sig)  (constant-time)
    │       · fail + policy Kick (DEFAULT) → MsgOutcome::Kick → Close 1008 + cleanup
    │       · fail + policy Warn          → charge, do not apply (:956-958)
    │     ↓ Token bucket: 64/s refill, capacity 64, cost 1 (lib.rs:808-810)
    │       · Paid InputFrame → note_accepted_input (:862-872)
    │       · 32 unpaid messages inside 1s → Close 1008 (:853, :881)
    │       · no ServerToClient::RateLimited
    │
    ├── ws.next() → ClientToServer::Ping { nano }    (charged, lib.rs:971)
    │     ↓ ServerToClient::Pong { nano } when the charge is paid
    │
    ├── ws.next() → ClientToServer::Ack { .. }       (charged, lib.rs:978)
    │
    └── snaps.changed() first under biased ← spawn_room_tick               (lib.rs:665, owner :750-768)
          ↓ build_snapshot once per period (lib.rs:771-802); room.tick += 1 at :777
          ↓ Encode DemoState { tick: server_tick } as postcard
          ↓ Wrap with lz4_flex::compress_prepend_size
          ↓ ServerToClient::Snapshot { id, server_tick, base_id: None, compressed: true, payload }

[On every exit from handle_socket, including `?` and panic]
    │
    │ PlayerMembership::drop removes the player
    │ If room.players.is_empty() → remove room from AppState.rooms        (lib.rs:443-458)
```

### Path B: ECS Plugin systems (dormant)

```text
[Caller: NetworkClientPlugin::new(addr).build(&mut App)]                  (net-ecs/lib.rs:55-67)
    │
    ├── app.add_system_known_stage("simulation", client_input_system)                 (net-ecs/lib.rs:64)
    └── app.add_system_known_stage("presentation", client_reconciliation_system)      (net-ecs/lib.rs:65)

[Caller: NetworkServerPlugin::new(addr).build(&mut App)]                  (net-ecs/lib.rs:75-87)
    │
    ├── app.add_system_known_stage("simulation", server_snapshot_system)              (net-ecs/lib.rs:84)
    └── app.add_system_known_stage("simulation", server_input_processing_system)      (net-ecs/lib.rs:85)

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
    │ Query2<CNetworkClient, CNetworkAuthority>                            (net-ecs/lib.rs:201)
    │ TODO comment: "Apply input validation, anti-cheat checks, etc."     (net-ecs/lib.rs:209)
    └── world.insert(entity, client)    ← no-op (client cloned, written back unchanged)

──────────────────────────────────────────────────────────────────────
[Async helpers — never called by the ECS systems]
    │
    ├── connect_to_server(addr) → mpsc::UnboundedReceiver<ServerToClient> (net-ecs/lib.rs:217-252)
    │     · connect_async(ws://) (NOT wss://)
    │     · Spawn task reading Message::Binary frames
    │     · decode_msg::<ServerToClient>(Codec::Bincode, ...)             ← note Bincode, not PostcardLz4
    │     · Forward via mpsc::UnboundedSender
    │
    └── start_network_server(bind_addr) → mpsc::UnboundedReceiver<ClientToServer>  (net-ecs/lib.rs:255-297)
          · TcpListener::bind + accept_async (plain TCP, no TLS)      (net-ecs/lib.rs:262, :264)
          · Same Bincode codec
          · No matchmaking, no room logic, no auth
```

### Stage-by-stage detail

#### Stage 1: Protocol layer (`aw-net-proto/src/lib.rs`)
**Role:** Define the wire enums (`ClientToServer`, `ServerToClient`) and the two encoder/decoder paths (`Codec::PostcardLz4` and `Codec::Bincode`).
**Inputs:** Serializable structs implementing `serde::Serialize` / `Deserialize`.
**Outputs:** `Vec<u8>` (encode) or `Result<T, WireError>` (decode).
**Notes:**
- `PROTOCOL_VERSION` is `u16 = 1` (proto/lib.rs:7).
- `Codec` is `#[non_exhaustive]` (proto/lib.rs:261). PostcardLz4 is the recommended path (smaller payloads); Bincode is "Fallback / compatibility" per the doc comment at proto/lib.rs:264.
- **Canonical signing surface** (Net-Trio-Remediation `561b20957`): `SigningKey` (private 32-byte field; `from_hex` / `from_bytes` / `as_bytes` / `dev_default`; redacted `Debug`) at proto/lib.rs:31-96; `sign(key, payload) -> [u8;32]` (proto/lib.rs:134), `verify(key, payload, tag) -> bool` (constant-time via `Mac::verify_slice`, proto/lib.rs:143-149), `hmac_sha256(key, payload)` (proto/lib.rs:123), `input_frame_sig_payload(seq, tick_ms, blob)` — the canonical MAC'd byte range (`seq.to_le_bytes() ++ tick_ms.to_le_bytes() ++ input_blob`, proto/lib.rs:161-167), and `SIG_LEN = 32` (proto/lib.rs:13). `InputFrame.sig` is `[u8; 32]` (proto/lib.rs:195). **The former `sign16` XOR stub and `SessionKey` type are DELETED.**
- `new_room_id()` returns 8 alphanumeric characters (proto/lib.rs:318).

> **Layout note (revision 1.12):** the standalone server was split into lib+bin (`066cd6cfd`). All server logic now lives in `net/aw-net-server/src/lib.rs` (~1393 lines: `ServerConfig`, `run_server`, `spawn_server`, `accept_loop_{tls,plain}`, one `handle_socket` (`handle_socket_tls` delegates), `on_client_msg`, `PlayerMembership`, `spawn_room_tick`, `build_snapshot`, the `send`/`recv` helpers); `src/main.rs` (129 lines) is a thin CLI wrapper that parses args into a `ServerConfig` and calls `run_server`. The pre-remediation single-file `server/main.rs` layout is gone. Stages 3–7 below were re-read at `0a2d93ea`. Stage 2 line cites were read at `0a2d93ea`. The 1.11 note's cites describe `de6f2725`.

#### Stage 2: Standalone server entry (`aw-net-server/src/lib.rs` — `spawn_server`)
**Role:** Parse CLI args (`--disable-tls`, `--tls-cert`, `--tls-key`, `--shared-key-hex`, `--sig-failure-policy`, `--ws-listen`, `--http-listen`, `--db-path`) in `main.rs::parse_args`, open sled DB, spawn the axum HTTP admin server on port 8789, then loop-accept WebSocket connections on port 8788.
**Inputs:** CLI args; PEM cert/key files (default `net/certs/dev/dev-cert.pem` and `dev-key.pem`).
**Outputs:** Long-running tokio tasks; tracing logs.
**Notes:**
- `spawn_server` refuses the published dev signing key outside debug and test builds (`lib.rs:307-312`; `published_dev_key_allowed` at `aw-net-proto/lib.rs:114-116`). `--sig-failure-policy` is parsed on every build (`main.rs:72-77`).
- `--disable-tls` is `cfg(not(debug_assertions))`-rejected (`net/aw-net-server/src/main.rs:36-42`) — release builds cannot disable TLS.
- HTTP admin defaults to `127.0.0.1:8789`. `/healthz` and `/regions` require a bearer token equal to `ServerConfig.admin_token` (`admin_router`). Missing token, empty token, or a wrong token returns 401. With the token, `/healthz` returns `"ok"` and `/regions` returns hardcoded `["us-east","us-west","eu-central"]`. No real region-resolution logic; the string list is decorative. `ServerConfig::default` assigns `SigningKey::dev_default()` on every build (`net/aw-net-server/src/lib.rs:167`). The release CLI refuses to start without `--shared-key-hex` (`main.rs:103-109`). Release `spawn_server` refuses the published key (`lib.rs:307-312`).
- sled opens `config.db_path` (`lib.rs:313-314`). `ServerConfig::default` sets that path to `aw_net_server_db` (`:166`). The handle is **never read or written** in the visible server surface. The `db` field on `AppState` is `#[allow(dead_code)]` (`lib.rs:254-255`).

#### Stage 3: One connection handler (`lib.rs:461-744`)
**Role:** Per-connection handshake, matchmaking, snapshot forwarding, player cleanup.
**Inputs:** A `WebSocketStream<S>` where `S: AsyncRead + AsyncWrite + Unpin` (TLS-wrapped or plain).
**Outputs:** Mutations to `AppState.rooms`; outbound `ServerToClient` messages.
**Notes:** `handle_socket` (`lib.rs:472-744`) is the only session body. `handle_socket_tls` (`lib.rs:461-469`) calls it. `on_client_msg` is `:926`, `send` is `:999`, and `recv` is `:1011`. `PlayerMembership` (`lib.rs:443-458`, `is_empty` at `:454`) removes the player, and the room when that player was the last one, when the handler returns, including a `?` from `MatchResult` or `JoinAccepted`.

#### Stage 4: Matchmaking + room allocation (`lib.rs:509-647`)
**Role:** On first non-Hello message, either find an existing room or create a new one; allocate the player into it; send `MatchResult` and `JoinAccepted`.
**Inputs:** `ClientToServer::FindOrCreate { region, game_mode, party_size }` or `ClientToServer::JoinRoom { room_id, display_name }`.
**Outputs:** Updated `AppState.rooms`; `MatchResult` + `JoinAccepted` over WS.
**Notes:**
- Room-find filter: `r.region == region && r.game_mode == game_mode && r.players.len() < 4` (`lib.rs:522`). Cap of 4 players per room is hardcoded on that `FindOrCreate` predicate. A new room also builds `snap_tx` and `spawn_room_tick` runs once (`lib.rs:544-546`).
- `party_size` field on `FindOrCreate` is destructured but ignored (`lib.rs:516-517`: `region, game_mode, ..`).
- `display_name` on `JoinRoom` is destructured but ignored (`lib.rs:548-550`: `display_name: _`).
- **No `session_key`/`session_key_hint` anymore** (deleted in `561b20957`). `MatchResult` carries only `{ room_id }` and `JoinAccepted` carries `{ room_id, player_id, tick_hz }` (`lib.rs:631-647`). Input-frame signing keys off the process-wide shared `SigningKey` (`AppState.signing_key`), not a per-room key.
- `tick_hz` starts at `30u32` (`lib.rs:512`) before room creation; the player insert copies `room.tick_hz` (`lib.rs:602`).
- New `Room.tick` starts at 0 and `snap_id` starts at 0 (`lib.rs:535-536`). `snap_tx` is created on that same struct (`:537`).
- The insert and the `PlayerMembership` binding happen under one lock release (`lib.rs:584-611`), before `MatchResult` (`:631-638`). A failed send still drops the membership.

#### Stage 5: Per-tick snapshot construction (`lib.rs:771-802`)
**Role:** Build a `ServerToClient::Snapshot` for one room.
**Inputs:** `&AppState`, `rid: &str` (room id).
**Outputs:** `(ServerToClient::Snapshot, snap_id)`.
**Notes:**
- The payload is a `DemoState { tick }` struct serialized with postcard (`lib.rs:785-790`). **This is a placeholder** — `net/README.md` documents how a real consumer would substitute its own snapshot type, but the current code only ships the tick.
- The snapshot is `compressed: true` and always wrapped with `lz4_flex::compress_prepend_size` (`lib.rs:795-800`).
- `room.snap_id` increments with `wrapping_add(1)` (`lib.rs:778`). At `u32::MAX` the snap ID wraps to 0.
- `spawn_room_tick` (`lib.rs:750-768`) is the only caller of `build_snapshot`. It waits on a persistent `Interval` (`MissedTickBehavior::Skip`, `:754`) and consumes the immediate first tick (`:755`). Each connection selects `snaps.changed()` first (`:665`). N sockets in one room advance `room.tick` once per period — see invariant 9.

#### Stage 6: HMAC-SHA256 input verification (`lib.rs:937-966`)
**Role:** Authenticate every `InputFrame` against the shared `SigningKey` BEFORE the frame is allowed to influence any server state, and apply the configured `SignatureFailurePolicy` on failure.
**Inputs:** `InputFrame.{seq, tick_ms, input_blob, sig: [u8; 32]}`, `AppState.signing_key: SigningKey`, `AppState.sig_failure_policy`.
**Outputs:** `MsgOutcome::Kick("input frame signature verification failed")` under policy `Kick` (default), or a `warn!` + continue under policy `Warn`.
**Notes (revision 1.3 — Net-Trio-Remediation, line cites refreshed at 1.11):**
- **Both ends build the MAC'd bytes via the single shared `input_frame_sig_payload(seq, tick_ms, &input_blob)`** (proto/lib.rs:161-167). The client signs (`aw_net_proto::sign`, client/main.rs:100-101) and the server verifies (`aw_net_proto::verify`, `lib.rs:948`) over EXACTLY those bytes. There is no longer any byte-range divergence between client and server.
- **`verify` is constant-time** — it calls `Mac::verify_slice` internally (proto/lib.rs:143-149); MAC tags are never compared with `==`.
- **Verification runs FIRST** — before any charge (`lib.rs:944-966`). `last_input_seq` / `last_seen` are updated only from `note_accepted_input` (`:862-872`) after a paid, verified `InputFrame`. Kick returns before a charge (`:950-954`). Plain and TLS call this one function.
- **The verification result DOES gate processing under the default `Kick` policy.** On a bad tag with `SignatureFailurePolicy::Kick`, the handler returns `MsgOutcome::Kick`, and `send_policy_close` writes Close 1008 (`lib.rs:881`). `PlayerMembership` then removes the player and an empty room. Under `SignatureFailurePolicy::Warn` the handler logs (`:957`), charges the bucket, and does not apply the frame (`:956-958`). A drained bucket does not send `RateLimited`. Thirty-two unpaid messages inside one second return `MsgOutcome::Kick("rate limited")`.
- The former pre-remediation defect — client signing with the XOR `sign16` (16-byte tag) while the server verified HMAC-SHA256 (32-byte tag), so every `verify_slice` failed on length mismatch and the server only `warn!`ed — is **RESOLVED** (`561b20957`+`79424389e`+`066cd6cfd`). The `sign16` function and the `[u8; 16]` sig field no longer exist.

#### Stage 7: Token-bucket rate limiting (`lib.rs:808-859`)
**Role:** Limit per-player message rate and close a socket that stays over the limit.
**Inputs:** `Player.tokens`, `Player.last_refill`, `Player.drop_times`, message arrival.
**Outputs:** `Toll::Paid`, `Toll::Drop`, or `Toll::Kick`. Kick becomes Close 1008.
**Notes:**
- Constants: `REFILL_RATE = 64.0` tokens/sec, `BUCKET_SIZE = 64.0`, `COST_PER_MESSAGE = 1.0`, `DROP_WINDOW = 1s`, `DROPS_IN_WINDOW = 32` (`lib.rs:808-814`).
- New players start full (`tokens: BUCKET_SIZE`, `lib.rs:597`). The refill keeps the fractional remainder and clamps to `[0, 64]`. An unpaid message does not put the bucket in debt and does not clear `drop_times`.
- A 30 Hz input pays every frame. About 96 messages/s (64 paid plus 32 unpaid inside one second) closes the socket. 60 Hz input plus one ack per input crosses that window. `thirty_hertz_input_stays_open_for_five_seconds` covers the demo client.
- Charged: `InputFrame`, protocol `Ping`, `Ack`, `Hello`, `FindOrCreate`, `JoinRoom`, undecodable binary, WebSocket Ping, Text, and Pong. WebSocket Close is not charged. The wildcard arm does not charge, so deleting the `Ack` arm leaves `Ack` uncharged (`lib.rs:976-986`).
- A paid WebSocket Ping replaces the Pong tungstenite queued while parsing, so it is one Pong. A dropped Ping still emits that library Pong. Thirty-one unpaid WebSocket Pings inside the window stay open, so that library Pong continues for the life of the socket. The 32nd closes.
- Drops older than one second leave `drop_times` before the length check (`lib.rs:844-850`). `expired_drops_leave_the_window_and_one_more_stays_open` fails if that `pop_front` is removed. The paid arm does not read the deque.
- `send_policy_close` (`lib.rs:881`) drains incoming frames, writes Close 1008, then drains until the peer is quiet, so a Windows stack does not RST a socket that still has unread data.

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
| **PROTOCOL_VERSION** | `u16 = 1` — wire-format version check on `Hello`. | `aw-net-proto/lib.rs:7` |
| **ClientToServer** | `#[non_exhaustive]` enum of 6 wire messages: `Hello`, `FindOrCreate`, `JoinRoom`, `InputFrame`, `Ping`, `Ack`. `InputFrame.sig` is `[u8; 32]`. | `aw-net-proto/lib.rs:172-205` |
| **ServerToClient** | `#[non_exhaustive]` enum of 8 wire messages: `HelloAck`, `MatchResult { room_id }`, `JoinAccepted { room_id, player_id, tick_hz }`, `Snapshot`, `Reconcile`, `Pong`, `RateLimited`, `ProtocolError`. (No `session_key_hint` on `MatchResult`/`JoinAccepted` — deleted in `561b20957`.) | `aw-net-proto/lib.rs:209-242` |
| **Codec** | `#[non_exhaustive]` enum: `PostcardLz4` (default), `Bincode` (compat). | `aw-net-proto/lib.rs:258-263` |
| **SigningKey** | Shared symmetric 32-byte HMAC key. Private `[u8; 32]` field (no tuple `.0` access); constructors `from_hex` (64 hex chars) / `from_bytes` / `dev_default`; accessor `as_bytes`; `Debug` is redacted to `SigningKey(<redacted>)`. Never transmitted on the wire — both ends hold it out-of-band (client: `AW_SHARED_KEY`, or `dev_default()` when unset, on every build; server: a debug build keeps `dev_default()` when `--shared-key-hex` is absent (`net/aw-net-server/src/main.rs:111-117`); only the release CLI requires that flag (`:103-109`); release `spawn_server` refuses the published key at boot (`lib.rs:307-312`). The server does not see the client's key; a published-key client joins and the first `InputFrame` is Close 1008 under Kick, or the connection stays up under `--sig-failure-policy warn`). | `aw-net-proto/lib.rs:31-96` |
| **sign / verify** | `sign(key: &SigningKey, payload: &[u8]) -> [u8; 32]` (HMAC-SHA256); `verify(key, payload, tag: &[u8; 32]) -> bool` (constant-time via `Mac::verify_slice`). Backed by `hmac_sha256(key, payload)`. | `aw-net-proto/lib.rs:123-149` |
| **input_frame_sig_payload** | THE canonical MAC'd byte range for `InputFrame`: `seq.to_le_bytes() ++ tick_ms.to_le_bytes() ++ input_blob`. Both client (signing) and server (verifying) MUST build the payload via this function — the single shared definition is what prevents signed-byte-range divergence. | `aw-net-proto/lib.rs:161-167` |
| **SIG_LEN** | `usize = 32` — length of an HMAC-SHA256 signature tag. | `aw-net-proto/lib.rs:13` |
| **SignatureFailurePolicy** | `#[derive(Default)]` enum `{ Kick (DEFAULT), Warn }`. On a failed `InputFrame` signature: `Kick` rejects the packet (no state updates) and disconnects via WebSocket Close (1008); `Warn` logs, charges the bucket, and does not apply the frame. `FromStr` parses `--sig-failure-policy kick\|warn`. | `aw-net-server/lib.rs:102-110`; `FromStr` at `:112-124`; CLI flag parsed on every build (`main.rs:72-77`) |
| **Room** | Server-side state: `{ id, region, game_mode, tick_hz, players: HashMap<PlayerId, Player>, tick, snap_id, snap_tx }`. `snap_tx` is the room's snapshot watch. (No `session_key` field — removed.) | `aw-net-server/lib.rs:234-248` |
| **Player** | Server-side per-player state: `{ id, display, last_input_seq, last_seen, accepted_inputs, dropped_inputs, tokens, last_refill, drop_times }`. | `aw-net-server/lib.rs:215-231` |
| **AppState** | Server-wide state: `{ rooms: Arc<parking_lot::Mutex<HashMap<RoomId, Room>>>, db: sled::Db, codec: Codec, signing_key: SigningKey, sig_failure_policy: SignatureFailurePolicy, rate_limit_enabled }`. Uses `parking_lot::Mutex` (`lib.rs:25`), not `std::sync::Mutex`. | `aw-net-server/lib.rs:251-260` |
| **ServerConfig / run_server / spawn_server** | `ServerConfig` (CLI-derived: listen addrs, TLS paths, db path, `signing_key`, `sig_failure_policy`, `rate_limit_enabled`) → `run_server(config)` (binary entry after arg-parse) → `spawn_server(config)` (binds listeners FIRST — the `127.0.0.1:0` ephemeral-port test seam — then spawns accept loops). | `aw-net-server/lib.rs:132-172, 289-299, 306-387` |
| **MsgOutcome** | Internal result of handling one in-session message: `Continue` (keep open) or `Kick(&'static str)` (close with 1008, then `PlayerMembership` removes the player). | `aw-net-server/lib.rs:205-212` |
| **WireError** | `#[non_exhaustive]` + `#[must_use]` decode error enum: `ProtocolMismatch`, `Decode`, `InvalidSigningKey`. | `aw-net-proto/lib.rs:247-254` |
| **CNetworkClient** | ECS component: `{ player_id, last_acknowledged_input, pending_inputs }`. | `astraweave-net-ecs/lib.rs:13-19` |
| **CClientPrediction** | ECS component: `{ predicted_position: Vec3, prediction_error: Vec3 }`. | `astraweave-net-ecs/lib.rs:21-26` |
| **CNetworkAuthority** | ECS component: `{ authoritative_tick, connected_clients: HashMap<String, mpsc::UnboundedSender<ServerToClient>> }`. | `astraweave-net-ecs/lib.rs:28-33` |
| **NetworkSnapshot** | Distinct from `aw-net-proto`'s `ServerToClient::Snapshot`. ECS-layer snapshot type with `{ server_tick, entity_states: HashMap<u64, EntityState> }`. | `astraweave-net-ecs/lib.rs:35-40` |

### Terms to NOT confuse

- **`Snapshot` (aw-net-proto wire variant) vs. `NetworkSnapshot` (astraweave-net-ecs)**: The wire-format snapshot is `ServerToClient::Snapshot { id, server_tick, base_id, compressed, payload }` carrying an opaque byte payload. The ECS-layer `NetworkSnapshot` is what's *inside* the payload — it deserializes per-entity state. Distinct types in distinct crates.
- **`SigningKey` (32 bytes, both ends) vs. `[u8; 32]` sig vs. `input_frame_sig_payload` (the signed bytes)**: One shared symmetric `SigningKey` keys the HMAC at both client and server (held out-of-band, never on the wire). The `sig` field is the 32-byte HMAC-SHA256 tag the client puts on each `InputFrame`. `input_frame_sig_payload(seq, tick_ms, &input_blob)` is the byte range that is MAC'd — NOT the raw `input_blob` alone; the `seq` and `tick_ms` fields are inside the signed bytes. (Historical note: there was formerly a per-room 32-byte `SessionKey`, an 8-byte `session_key_hint` sent to the client, and a 16-byte XOR `sign16` tag — all three deleted in `561b20957`.)
- **HMAC sign vs. verify (one algorithm, both ends agree)**: The client computes `aw_net_proto::sign(&signing_key, &payload)` (`client/main.rs:100-101`) and the server computes `aw_net_proto::verify(&app.signing_key, &payload, &sig)` (`lib.rs:948`), where `payload = input_frame_sig_payload(seq, tick_ms, &input_blob)` on both sides. **These produce/check the same bytes.** A valid frame verifies; a forged or wrong-key frame is rejected and (default policy) kicked. (Historical note: pre-remediation the client signed with XOR `sign16` while the server verified HMAC-SHA256, so every verification failed — RESOLVED.)
- **`Codec::Bincode` (proto) vs. `Codec::PostcardLz4` (proto)**: Both are members of the same enum. `PostcardLz4` is used by both standalone binaries (client/main.rs:190, lib.rs:318). `Bincode` is used by `astraweave-net-ecs` (lib.rs:179, 237, 279). The two integration paths thus serialize incompatibly even though they share the protocol crate.
- **`astraweave-net::Snapshot` (older subsystem) vs. anything in this subsystem**: Separate crate, different data model, JSON-text wire format. Documented in `docs/architecture/net.md`.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-ecs` (Plugin layer dep) | `App`, `Plugin`, `Query`, `World` (`net-ecs/lib.rs:7`); `Query2` is `astraweave_ecs::Query2` at `:201` | Component query/insert/get; `app.add_system_known_stage`; `app.add_plugin(Plugin)` | The four ECS systems take `&mut World`. Plugin builders register them in `"simulation"` or `"presentation"` stages. |
| `astraweave-core` (workspace dep) | (declared in `net-ecs/Cargo.toml:17`) | none directly imported in `net-ecs/src/lib.rs` | The dep is declared but workspace grep shows no `use astraweave_core` in `astraweave-net-ecs/src/`. The dependency may exist for future use or has been left from a prior version. |
| `glam` (ECS data) | `Vec3` (in `CClientPrediction.predicted_position`, `EntityState.position`) | 3D position component | Used by the ECS Plugin layer only; the standalone binaries operate on opaque `Vec<u8>` blobs. |
| Filesystem (sled) | `sled::open(&config.db_path)` (`lib.rs:313-314`; default `aw_net_server_db` at `:166`) | KV store | `AppState.db` field is `#[allow(dead_code)]` (`:254-255`) — opened but unused. The directory exists at workspace root with a `conf` file (62 B) and `db` file (96 B) — sled's tiny housekeeping artifacts. |
| Filesystem (TLS certs) | `load_certs` / `load_private_key` (`lib.rs:263`, `:270`) | PEM-encoded cert chain + key | Defaults to `net/certs/dev/dev-cert.pem` and `dev-key.pem`. Generation script: `net/certs/dev/generate_dev_cert.sh` (referenced in warning at `lib.rs:360`). |
| Environment | `AW_WS_URL`, `AW_REGION` (`client/main.rs:14-15`) | Server URL and region string | Client defaults: `wss://127.0.0.1:8788`, region `us-east`. |
| WebSocket client connections | `accept_hdr_async` (TLS `lib.rs:409`, plain `:428`) | Postcard-LZ4 binary frames | `accept_loop_tls` starts at `:390` and `accept_loop_plain` at `:423`. Text and Pong are charged (`:729`). WebSocket Close is not. |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-stress-test` (Cargo.toml:20: `astraweave-net-ecs = { workspace = true }`) | none in source | none | **The declared dependency is not used.** Workspace grep `use astraweave_net_ecs` against `astraweave-stress-test/` returns no source-file matches. See §6. |
| Standalone client binary | (none — terminal endpoint) | tracing logs | The client binary prints snapshot info via `tracing::info!`. |
| sled KV store | Unused write surface | none | `AppState.db` is opened but never queried or mutated in the visible source. |
| Future game integration | `NetworkSnapshot.entity_states` | Per-entity state | The data shape exists but no consumer reads it as of `a2474c5b7`. |

### Bidirectional / Coupled

- **Shared `SigningKey` (client ⟷ server):** A single 32-byte symmetric key is held by both ends out-of-band (never transmitted): the client loads it from `AW_SHARED_KEY` (64 hex) or `SigningKey::dev_default()` when that variable is unset, on every build (`net/aw-net-client/src/main.rs:19-25`); the server loads it from `--shared-key-hex`. A debug server keeps `dev_default()` when `--shared-key-hex` is absent (`net/aw-net-server/src/main.rs:111-117`); only the release CLI requires that flag (`:103-109`); release `spawn_server` refuses the published key at boot (`lib.rs:307-312`). The server never sees the client's key. `Hello`, `FindOrCreate`, and `JoinRoom` are unsigned. `HelloAck` is `lib.rs:483`. The unsigned player insert is `lib.rs:588-599`, before `MatchResult` (`:631-638`) and `JoinAccepted` (`:639-647`). Only `InputFrame` is verified (`lib.rs:948`). A client using the published key against a release server is a wrong-key client: it joins, is kicked with Close 1008 on its first input under the default Kick policy, and under `--sig-failure-policy warn` (parsed on every build at `main.rs:72-77`) the bad tag is charged and not applied. `last_input_seq` and `last_seen` move only in `note_accepted_input` after a paid, verified frame (`lib.rs:862-872`). The client signs each `InputFrame` (`aw_net_proto::sign`) and the server verifies (`aw_net_proto::verify`, constant-time) over the same `input_frame_sig_payload(seq, tick_ms, &input_blob)` bytes. A bad tag is kicked by default (Close 1008). (No per-room `session_key`/`session_key_hint` remains — deleted in `561b20957`; see §6 / Stage 6.)

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
| `net/aw-net-proto/src/lib.rs` | Wire-protocol types + `encode_msg`/`decode_msg` + **canonical signing surface** (`SigningKey`, `sign`, `verify`, `hmac_sha256`, `input_frame_sig_payload`, `SIG_LEN`) + `new_room_id` + `WireError` | Active | 321 lines. `#![forbid(unsafe_code)]`. **`sign16`/`SessionKey` deleted** (`561b20957`); `InputFrame.sig` is now `[u8; 32]`. |
| `net/aw-net-proto/tests/mutation_resistant_comprehensive_tests.rs` | Mutation-resistance harness + RFC 4231 HMAC known-answer vectors | Active (tests) | 59 tests. |
| `net/aw-net-proto/benches/proto_bench.rs` | Criterion benches for codec encode/decode | Active | 13 `bench_function` call sites. |
| `net/aw-net-client/src/main.rs` | Standalone CLI client binary | Active | 193 lines. Demo input loop at 33 ms tick (~30 Hz). Signs each `InputFrame` via `aw_net_proto::sign` over `input_frame_sig_payload`; key from `AW_SHARED_KEY`, or `dev_default()` when unset, on every build. Uses `tokio-tungstenite` with `rustls-tls-native-roots` feature for wss. Carries the S2C-unsigned design note (`561b20957`/`eb9977b88`). |
| `net/aw-net-client/Cargo.toml` | Binary crate metadata | Active | `default-features = false`; `tokio-tungstenite = { version = "0.28", features = ["connect", "rustls-tls-native-roots"] }` (line 9). |
| `net/aw-net-server/src/lib.rs` | **Server library**: `ServerConfig`/`run_server`/`spawn_server`, one generic connection handler, matchmaking, sled, rate limiting, **HMAC verify-FIRST via the canonical proto surface**, `SignatureFailurePolicy { Kick, Warn }` | Active | ~1393 lines. `handle_socket_tls` delegates to `handle_socket`. `PlayerMembership` removes the player on every exit. One `spawn_room_tick` per room publishes snapshots. The verify/kick logic lives in `on_client_msg`. |
| `net/aw-net-server/src/main.rs` | Thin CLI wrapper: parse args (`--shared-key-hex`, `--sig-failure-policy`, `--disable-tls`, `--tls-cert`/`--tls-key`, `--ws-listen`/`--http-listen`/`--db-path`) → `ServerConfig` → `run_server` | Active | 129 lines. No logic beyond arg-parse (`066cd6cfd` lib/bin split). |
| `net/aw-net-server/tests/common/mod.rs` | Shared in-process test harness (spawns a live server on an ephemeral port via `spawn_server`, drives a real WS client) | Active (tests) | W.3.1 (`7029d7d7f`). |
| `net/aw-net-server/tests/family1_authenticated_round_trip.rs` | Family 1: authenticated round-trip (valid signing) | Active (tests) | 5 tests. |
| `net/aw-net-server/tests/family2_malformed_packets.rs` | Family 2: malformed/garbage packets | Active (tests) | 12 tests. |
| `net/aw-net-server/tests/family3_wrong_key_policy.rs` | Family 3: wrong-key inputs under Kick vs Warn policy | Active (tests) | 9 tests. |
| `net/aw-net-server/tests/family4_disconnect_paths.rs` | Family 4: disconnect / cleanup paths | Active (tests) | 9 tests. |
| `net/aw-net-server/tests/family5_tls_signature_path.rs` | Family 5: TLS verify/kick path (`2955cd14c`) | Active (tests) | 4 tests. |
| `net/aw-net-server/tests/w2b_fix1_regressions.rs` | W.2.b-fix1 regressions: graceful unknown-room + cleanup-preserving snapshot path (`9a3fc94e3`) | Active (tests) | 2 tests. |
| `net/aw-net-client/tests/family1_client_binary.rs`, `net/aw-net-client/tests/family3_client_binary_wrong_key.rs` | Client-binary families: valid-key round-trip and wrong-key behavior | Active (tests) | 2 + 2 = 4 tests. |
| `net/aw-net-server/Cargo.toml` | lib+bin crate metadata | Active | Includes `tokio-rustls = "0.26"`, `rustls = "0.23"` (ring), `sled = "0.34"`, `axum = "0.8"`. PEM is parsed with `rustls-pki-types`. HMAC now lives in `aw-net-proto` (`hmac`/`sha2`), not the server. |
| `net/aw-net-proto/Cargo.toml` | Library crate metadata | Active | Uses `bincode = { version = "2.0", features = ["serde"] }` and `postcard = { version = "1", features = ["alloc"] }`. |
| `net/README.md` | Integration guide | Active | Usage instructions. Updated in the Net-Trio-Remediation (W.4): HMAC-SHA256 signing is documented as implemented and enforced (client signs, server verifies constant-time, kicks by default), with the known limitations (no replay protection; S2C unsigned; generate real dev certs) noted. See §6. |
| `net/TLS_IMPLEMENTATION_SUMMARY.txt`, `net/TLS_TESTING_GUIDE.txt` | TLS notes | Active | Standalone text files at `net/` root. `TLS_IMPLEMENTATION_SUMMARY.txt` is a structured implementation changelog documenting the cert-loading functions, CLI flags (`--tls-cert`, `--tls-key`, `--disable-tls`), and dual-handler design (`handle_socket_tls`/`handle_socket`). Verified by direct read 2026-05-12. The line numbers and decisions in this doc match the current code. |
| `net/certs/dev/` | Dev TLS certs + generation script | Active | Referenced at `lib.rs:360` (warning points users to `generate_dev_cert.sh` when cert load fails). |
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
| `Codec::PostcardLz4` (standalone binaries) vs. `Codec::Bincode` (ECS Plugin layer) | `client/main.rs:190`, `lib.rs:295` vs. `astraweave-net-ecs/lib.rs:179, 237, 279` | Two parallel wire formats sharing one enum | The standalone trio uses PostcardLz4 end-to-end. The ECS Plugin layer's `connect_to_server` / `start_network_server` / `server_snapshot_system` all hardcode `Codec::Bincode`. If a future game tries to wire the Plugin layer to talk to the standalone server, codec mismatch will cause every decode to fail. |
| `tokio-rustls 0.25` + `rustls 0.22` (historical server pin) vs. `tokio-rustls 0.26` + `rustls 0.23` | `net/aw-net-server/Cargo.toml` and `astraweave-net/Cargo.toml` | **RESOLVED** | Both crates now depend on tokio-rustls 0.26 and rustls 0.23 with the ring provider. `Cargo.lock` contains one `rustls` (0.23) and one `tokio-rustls` (0.26). Certificate PEM is parsed with `rustls-pki-types`, not `rustls-pemfile`. |
| `astraweave-net-ecs::EntityState` vs. `astraweave-net::EntityState` (in the other networking subsystem) | `net-ecs/lib.rs:43-47` vs. `astraweave-net/src/lib.rs:38-44` | Same name, disjoint shapes | `net-ecs::EntityState { position: Vec3, health: i32 }` vs. `net::EntityState { id: u32, pos: IVec2, hp: i32, team: u8, ammo: i32 }`. Cross-imports require qualification. (Already noted in `docs/architecture/net.md`.) |
| `connect_to_server` / `start_network_server` (ECS layer) vs. `aw-net-server`'s `handle_socket` (standalone) | `astraweave-net-ecs/lib.rs:217-294` vs. `aw-net-server/lib.rs:472-744` | Two independent WebSocket handlers | The ECS layer's `start_network_server` is a minimal `accept_async` loop with no matchmaking, no auth, no rate limiting — it just decodes `ClientToServer` frames into an mpsc channel. The standalone `aw-net-server` is the production-grade path. They share no code beyond the `aw-net-proto` types. |
| `handle_socket_tls` vs. `handle_socket` in the server | `aw-net-server/lib.rs:461-469` delegates to `:472` | **RESOLVED** (`de6f2725`) | One generic `handle_socket`. `handle_socket_tls` is a delegate. `send_tls`, `recv_tls`, and `on_client_msg_tls` are gone. `PlayerMembership` is the cleanup path for both accept loops. |
| `sled::Db` field on `AppState` is opened but unused | `lib.rs:254-255` (#[allow(dead_code)]), `:313-314` (open) | Active (declared, no read/write call sites) | `grep` over `lib.rs` for `app.db` / `state.db` read/write returns no read or write call sites after the `open` call. The sled directory at `aw_net_server_db/` exists but contains only sled's own initialization artifacts. |
| `NetworkClientPlugin::server_addr` and `NetworkServerPlugin::bind_addr` | `net-ecs/lib.rs:50-52, 70-72 (#[allow(dead_code)])` | Stored, never read | The plugin builders use neither field. The async helpers take the address as a separate parameter. The fields exist only as placeholders for a future wiring. |

### Naming collisions

- **`Snapshot`** appears in: `astraweave-net::Snapshot` (struct), `aw-net-proto::ServerToClient::Snapshot` (enum variant carrying opaque payload), `astraweave-net-ecs::NetworkSnapshot` (struct), `aw-net-server/lib.rs::build_snapshot::DemoState` (local placeholder struct).
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
  - **What's actually true:** `AppState.db` has no read/write call sites in `aw-net-server/src/lib.rs`. Restarting the server loses all in-memory `rooms`. The sled directory exists with tiny `conf` (62 B) and `db` (96 B) housekeeping files only.

- **Trap:** A client that sends input and an ack on every tick is over the kick ceiling.
  - **Why it's confusing:** 64 tokens/s pays a 30 Hz input. It does not pay 60 Hz input plus one ack per input.
  - **What's actually true:** `REFILL_RATE = 64.0`, `BUCKET_SIZE = 64.0`, `COST_PER_MESSAGE = 1.0` (`lib.rs:808-810`). Thirty-two unpaid messages inside one second close 1008. The ceiling is about 96 messages/s. `RateLimited` is not sent. The demo client sends one input every 33 ms and no ack, and stays connected.

- **Trap:** `lib_temp.rs` exists in `astraweave-net-ecs/src/`.
  - **Why it's confusing:** Looks like a partial refactor or backup file. The commit title that introduced it (`54d15c9f2` "added forest biome textures and assets, worked on renderer") gives no clue about why a temp file was added to the netcode crate.
  - **What's actually true:** It's not declared as `mod lib_temp` anywhere; it is not the crate's library entry. Cargo only compiles `src/lib.rs`, so `lib_temp.rs` doesn't enter the build. Its tests don't run. It's effectively orphan source.

- **Trap:** `connect_to_server` (`net-ecs/lib.rs:217`) uses `ws://`, not `wss://`.
  - **Why it's confusing:** The standalone client (`client/main.rs:14`) defaults to `wss://127.0.0.1:8788`. The ECS Plugin helper builds `format!("ws://{}", server_addr)`.
  - **What's actually true:** The two paths cannot interoperate without modification. The Plugin layer expects a plain-TCP server (which the standalone server only serves under `--disable-tls` in debug builds).

---

## 7. Decision Log

### Decision: Two integration paths (standalone trio + ECS Plugin layer) sharing one protocol crate
- **Date:** Standalone trio: 2025-09-09 commit `cc9a7e3e3` ("Implement production-ready enhanced networking layer with server authority, client prediction, and matchmaking (#58)"). ECS Plugin layer: 2025-10-01 commit `08befc6ec` ("phase 6 implementation").
- **Status:** Accepted (live in three workspace crates).
- **Context:** Per `net/README.md:181`, "This enhanced networking layer runs alongside the existing `astraweave-net` and coop examples without conflicts. The original examples continue to work for simple scenarios, while this provides capabilities for serious multiplayer games."
- **Decision:** Build a new networking layer instead of refactoring `astraweave-net`. Share types only through the new `aw-net-proto` crate.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Workspace now ships **two** networking subsystems with disjoint data models, disjoint wire formats, and overlapping concerns.
  - Cross-consumer of the new system: zero in production source code; standalone binaries are end-points.
  - The ECS Plugin layer was added a month later than the standalone binaries — it appears to be a separately-developed adaptation that has not been wired to a game.

### Decision: PostcardLz4 as default codec; Bincode for fallback
- **Date:** 2025-09-09 commit `cc9a7e3e3`.
- **Status:** Accepted (`aw-net-proto/lib.rs:258-263`).
- **Context:** The README mentions "60-80% size reduction on typical game state" from compression (`net/README.md:170`).
- **Decision:** PostcardLz4 (postcard serialize + lz4 compress) as the wire codec; Bincode as "Fallback / compatibility" (per doc comment at `aw-net-proto/lib.rs:261`).
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Standalone server and client both use PostcardLz4.
  - ECS Plugin layer hardcodes Bincode (`net-ecs/lib.rs:179, 237, 279`).
  - The two paths are wire-incompatible. Splitting the codec choice between the two consumers means the ECS Plugin layer cannot talk to the standalone server without modification.

### Decision: Matchmaking room cap of 4 players, hardcoded
- **Date:** 2025-09-09 commit `cc9a7e3e3`.
- **Status:** Accepted (`lib.rs:522`).
- **Context:** Room-find predicate: `r.region == region && r.game_mode == game_mode && r.players.len() < 4`.
- **Decision:** Hard-code the 4-player room cap in the `FindOrCreate` predicate. The predicate is one site (`lib.rs:522`).
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Changing the `FindOrCreate` cap is one edit in `handle_socket`.
  - The `ClientToServer::FindOrCreate.party_size` field is destructured but ignored, so the client cannot request a larger room.

### Decision: 30 Hz server tick, hardcoded
- **Date:** 2025-09-09 commit `cc9a7e3e3`.
- **Status:** Accepted (`lib.rs:512`; room `tick_hz` copied at `:602`).
- **Context:** `tick_hz = 30u32` is the default in `handle_socket`, baked into the new `Room.tick_hz` field.
- **Decision:** 30 Hz authoritative tick rate.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - The room tick owner waits `Duration::from_millis(1000 / tick_hz)` (`lib.rs:752`), about 33 ms at 30 Hz.
  - The token bucket refills at 64/s, which pays a 30 Hz input. Input plus ack at 60 Hz crosses the kick window — see §6 trap.

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

### Decision: Token bucket at 64/s, kick after 32 unpaid messages in one second
- **Date:** Supersedes the 8/s bucket from 2025-09-09 commit `cc9a7e3e3`.
- **Status:** Accepted (`lib.rs:808-814`).
- **Context:** `REFILL_RATE = 64.0`, `BUCKET_SIZE = 64.0`, `COST_PER_MESSAGE = 1.0`, `DROPS_IN_WINDOW = 32` inside `DROP_WINDOW` of one second. A new player starts full.
- **Decision:** Charge the messages a peer can send today. Verify an `InputFrame` before charging it. A paid message does not clear the drop window. The 32nd unpaid message inside the window closes the socket with 1008. Do not send `RateLimited`.
- **Alternatives considered:** Leaving the 8/s bucket and `RateLimited` (the 33 ms demo client was kicked, and a paced overrun could reset a consecutive counter). An uncharged Ping (a flood never hit the bucket).
- **Consequences:**
  - A 30 Hz input pays every frame. About 96 messages/s closes the socket. 60 Hz input plus one ack per input crosses that.
  - Warn charges a bad tag and does not apply it. Kick returns before a charge.

### Decision: TLS-by-default in release; opt-out only in debug builds
- **Date:** 2025-11-13, commit `4889a9a33` ("feat: Integrate astraweave-security for path validation and migration scripts"). Verified via `git log -S "tls_enabled = true" -- net/aw-net-server/src/main.rs`. This was the security-hardening sweep that also introduced HMAC verification five days later.
- **Status:** Accepted (`net/aw-net-server/src/main.rs:36-42` for `--disable-tls`; TLS defaults in `lib.rs:157-159`).
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
| 1 | `PROTOCOL_VERSION == 1` (u16) | Yes (compile-time) | `aw-net-proto/lib.rs:7`. Mismatch is rejected with `ServerToClient::ProtocolError` (`lib.rs:459-468`). |
| 2 | `ClientToServer.Hello` is the first message every client must send | Yes | `handle_socket` expects `recv` of `ClientToServer` first; any non-Hello produces `ProtocolError { "expected Hello" }` (`lib.rs:470-479`). |
| 3 | `InputFrame.sig` is a 32-byte HMAC-SHA256 tag (`SIG_LEN == 32`) | Yes (compile-time) | `aw-net-proto/lib.rs:13`, `:195`. (Replaces the former `session_key_hint = first 8 bytes` invariant — `session_key_hint`/`SessionKey` deleted in `561b20957`.) |
| 4 | `new_room_id()` returns 8 alphanumeric characters | Yes | `aw-net-proto/lib.rs:315`. |
| 5 | A `SigningKey` is exactly 32 bytes | Yes (compile-time) | Field type is a private `[u8; 32]` (`aw-net-proto/lib.rs:31`); `from_hex` rejects anything ≠ 64 hex chars (`:72-77`). |
| 6 | Room cap is 4 players (per region+game_mode) | Yes | `FindOrCreate` predicate at `lib.rs:522`. |
| 7 | Empty room is removed on player disconnect | Yes | `PlayerMembership::drop` (`lib.rs:423-430`): `if room.players.is_empty() { rooms.remove(&self.rid); }`. The guard runs on every exit from `handle_socket`, including a failed `MatchResult` or `JoinAccepted` send. |
| 8 | `aw_net_proto::encode_msg(Codec::PostcardLz4, ...)` is followed by `decode_msg(Codec::PostcardLz4, ...)` | Test-enforced | Server (`lib.rs:295`) and client (`client/main.rs:190`) both pin PostcardLz4. ECS Plugin layer (`net-ecs/lib.rs:179, 237, 279`) pins Bincode independently. Codec mismatch produces `WireError::Decode`. |
| 9 | One room tick owner advances `room.tick` once per period, independent of how many sockets are in the room | Yes | `spawn_room_tick` (`lib.rs:750-768`) is the only caller of `build_snapshot`. `lib.rs:777` increments `room.tick += 1` per call. The connection selects `snaps.changed()` first under `biased` (`:665`). `tests/room_tick_owner.rs` fails when four clients advance the tick about four times per period, when a client that sends one protocol Ping every 20 ms (50/s, charged, under the 64/s refill) receives fewer than half the due snapshots in 500 ms, or when pre-queued signed input keeps the socket readable and `server_tick` does not step by exactly 1. Putting `ws.next()` first under `biased` delivered 1 snapshot in 500 ms. |
| 10 | HMAC-SHA256 verification runs FIRST and gates processing per `SignatureFailurePolicy`: under `Kick` (DEFAULT) a failed tag returns before any charge and disconnects (Close 1008); under `Warn` it charges the tag and does not apply the frame | Yes | `lib.rs:944-966`: verify precedes `charge_message`. `note_accepted_input` (`:862`) runs only for a paid, verified `InputFrame`. |
| 11 | Token bucket refill rate is 64 tokens/sec; capacity is 64; cost per message is 1; 32 unpaid messages inside 1s close the socket | Yes (compile-time) | `lib.rs:808-814` constants. New players start full (`:597`). |
| 12 | `tick_hz` defaults to 30 | Yes | `lib.rs:512` (initial binding); the new room stores that `tick_hz` (`:533`). |
| 13 | `--disable-tls` is forbidden in release builds | Yes (compile-time) | `main.rs:36-42` `#[cfg(not(debug_assertions))]`. |
| 14 | `astraweave-net-ecs::server_snapshot_system` always sends snapshots with `entity_states: HashMap::new()` | Yes | `net-ecs/lib.rs:174`. Stub system. |
| 15 | Compression of snapshots in `aw-net-server::build_snapshot` is always on (`compressed: true`) | Yes | `lib.rs:742`. |
| 16 | Both ends build the MAC'd byte range via `input_frame_sig_payload(seq, tick_ms, &input_blob)` — the canonical signed bytes are `seq.to_le_bytes() ++ tick_ms.to_le_bytes() ++ input_blob`, never hand-rolled | Test-enforced (families 1/3/5) | Definition `aw-net-proto/lib.rs:161-167`; client signs over it (`client/main.rs:100-101`), server verifies over it (`lib.rs:948`). A single shared definition is what prevents signed-byte-range divergence (the original defect). |

---

## 9. Performance & Resource Profile

### Hot paths

- **One `build_snapshot` per room per tick** (`lib.rs:771-802`) — `spawn_room_tick` (`:750`) calls it `tick_dt` apart (default ~33 ms). Cost: mutex acquire on `app.rooms`, increment two `u32`/`u64` fields, postcard serialize a tiny `DemoState { tick: u64 }`, lz4 compress, `watch::send`. Sub-millisecond by inspection. Connection count does not multiply this call.
- **Per-message `decode_msg`** in the per-connection select loop (`lib.rs:656`) — lz4 decompress + postcard deserialize.
- **HMAC-SHA256 verify** per `InputFrame` (`aw_net_proto::verify`, called at `lib.rs:948`) — runs FIRST, before rate-limit/state updates; fast for small payloads but allocates a fresh `Hmac<Sha256>` and the `input_frame_sig_payload` `Vec` per message.
- **ECS systems run once per stage tick.** None hit the network synchronously; all encode/decode work in the ECS path is in-process (`encode_msg(Codec::Bincode, &snapshot)` at `net-ecs/lib.rs:179`, with no I/O).

### Cold paths

- **TLS handshake** (TLS handshake in `accept_loop_tls` at `lib.rs:375-381`) — milliseconds per connection.
- **Matchmaking room scan** (`lib.rs:495-497`) — linear scan of `rooms` HashMap; rooms cap not enforced; current scale assumed small. The `players.len() < 4` check is the player cap, not a room-count cap.
- **Signing-key load** — the 32-byte `SigningKey` is parsed once at process startup. The client uses `AW_SHARED_KEY` or `dev_default()` on every build. A debug server keeps `dev_default()` when `--shared-key-hex` is absent (`net/aw-net-server/src/main.rs:111-117`); only the release CLI requires that flag (`:103-109`); release `spawn_server` refuses the published key at boot (`lib.rs:307-312`). There is no per-room key generation anymore (the former per-room `SessionKey::random()` was removed in `561b20957`).
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
- **Standalone binaries are now integration-tested**, not just manually smoke-tested. The shipped client binary over TLS remains untested (rustls cert validation rejects the self-signed dev cert — see §11); the server's TLS verify/kick path IS covered (family 5).
- **Benchmarks:**
  - `astraweave-net-ecs/benches/net_ecs_benchmarks.rs`: 9 `bench_function` call sites.
  - `astraweave-net-ecs/benches/net_ecs_adversarial.rs`: 11 `bench_function` call sites (worst-case shapes).
  - `net/aw-net-proto/benches/proto_bench.rs`: 13 `bench_function` call sites.
- **Total bench cases:** 33.
- **CI presence:** **None as of `a2474c5b7`.** Workspace grep `grep -l "aw-net\|astraweave-net-ecs" .github/workflows/*.yml` returns no matches. No dedicated workflow runs `cargo test` for any of these four crates. Compare with the older `astraweave-net` crate which has `.github/workflows/net-tests.yml`.
- **Mutation testing:** Per-crate inline mutation-resistance test suites (27 in `astraweave-net-ecs`, 53 in `aw-net-proto`); not run from a centralized workflow.
- **Miri / Kani validation:** Not in `.github/workflows/miri.yml` or `kani.yml`. The library crates carry `#![forbid(unsafe_code)]`; the binaries do not but contain no observable unsafe blocks.
- **Manual validation:** `net/README.md:13-30` is the smoke-test workflow: `cargo run -p aw-net-server` (`:17`) then `cargo run -p aw-net-client` (`:29`). The client logs `joined; tick_hz=` at `net/aw-net-client/src/main.rs:67`.

---

## 11. Open Questions / Parked Decisions

- **Why is `astraweave-net-ecs` declared by `astraweave-stress-test` but not imported?** Workspace grep `use astraweave_net_ecs` returns no source-file matches outside `astraweave-net-ecs`'s own tests/benches. `astraweave-stress-test/Cargo.toml:20` declares `astraweave-net-ecs = { workspace = true }` but no source file in that crate references the types. **Investigation (2026-05-12):** The dep was added in commit `08befc6ec` (2025-10-01, "phase 6 implementation") — the **same** commit that created `astraweave-net-ecs` itself. `astraweave-stress-test/src/lib.rs:38-42` defines its own `CNetworkStress { player_id, input_buffer, last_sync }` component, parallel to but disjoint from `astraweave-net-ecs::CNetworkClient`. So the dep was added at crate creation and never wired through; the stress test invented its own simulated network state types. Is this stale residue, future planning, or a deliberate ABI-only inclusion? Andrew's call.

- **Why did the client sign with `sign16` (XOR) while the server verified with HMAC-SHA256? — RESOLVED (Net-Trio-Remediation, `561b20957`+`79424389e`+`066cd6cfd`).** It was an in-progress upgrade: the server was hardened to HMAC-SHA256 first (commit `88434f3a2`) and the client was never updated, so every verification failed on length+algorithm and the server only `warn!`ed. The trio is now unified on a single canonical `aw-net-proto` HMAC surface, signed end-to-end and enforced (kick-by-default). `sign16`/`SessionKey`/`session_key_hint` deleted.

- **Known deliberate boundaries of the remediation (NOT open defects).** These were fenced out of scope intentionally and are recorded for future session-security work; full rationale in `docs/audits/net_trio_signature_remediation_findings_2026-06.md`:
  1. **No replay protection / nonces / sequence-number freshness.** HMAC proves a frame is authentic (signed by a holder of the shared key), not fresh — a captured valid frame can be replayed. Mitigating this needs nonces or a monotonic-sequence auth window.
  2. **Server→client messages are not signature-verified.** This is an authoritative-server asymmetric-trust model: the client has no independent ground truth to validate server state against, and a *shared symmetric* key cannot meaningfully authenticate S2C anyway (every client in a room holds the same key). Meaningful S2C auth requires asymmetric server keys or per-session key exchange (a handshake — out of scope for this trio). See the design note in `client/main.rs:114-127`.
  3. **The shipped client binary over TLS is untested** — rustls (`rustls-tls-native-roots` on the client; source comment at `client/main.rs:32`) rejects the self-signed dev cert (a cert-validation concern, not a signing concern). The *server's* TLS verify/kick path IS tested (family 5).

- **What is `lib_temp.rs` for?** A 436-line near-duplicate of `lib.rs` exists in `astraweave-net-ecs/src/` but is not declared as a module. Created 2025-11-19 in a commit titled "added forest biome textures and assets, worked on renderer" — the title is unrelated. Is this orphan source to be deleted, a backup awaiting a refactor, or part of an in-progress experiment? Andrew's call.

- **Is the empty `entity_states: HashMap::new()` in `server_snapshot_system` a stub awaiting wiring, or by design?** The system runs successfully and sends out snapshots, but they carry no actual entity data. The ECS Plugin layer's snapshot pipeline is effectively a no-op. **Investigation (2026-05-12):** `git log -p -S "entity_states"` shows the empty `HashMap::new()` was present from the crate's birth commit `08befc6ec` (2025-10-01) — the comment in the original commit reads `// Would collect actual entity states` at `astraweave-net-ecs/src/lib.rs:174`. Day-one stub, never replaced. The decisional part remains: should this be wired up, removed, or kept as a marker for future implementation?

- **What is the intended fix for the token-bucket / client-tick mismatch? — RESOLVED.** The bucket refills at 64/s with capacity 64. A 30 Hz input pays every frame. `RateLimited` is not sent. Thirty-two unpaid messages inside one second close 1008. About 96 messages/s is the ceiling; 60 Hz input plus one ack per input crosses it.

- **Why is `sled::Db` opened but never used?** `AppState.db` is opened on server startup but has no read/write call site. The README's claim that the server persists room/player state survives restarts is not backed by code. Is this a parked persistence feature or dead allocation?

- **Why is `Codec::Bincode` (in `astraweave-net-ecs`) hardcoded instead of using the same `Codec::PostcardLz4` as the standalone trio?** Mixing the two requires every ECS-layer consumer to handle a different wire format than the standalone server uses. Is this intentional (separate use-cases) or oversight?

- **Why are the TLS stack versions one major behind the other workspace crate? — RESOLVED.** `aw-net-server` and `astraweave-net` both use `tokio-rustls` 0.26 and `rustls` 0.23 with the ring provider. `rustls` 0.22 and `tokio-rustls` 0.25 are not in `Cargo.lock`. Certificate PEM is parsed with `rustls-pki-types` (`CertificateDer::pem_file_iter`, `PrivateKeyDer::from_pem_file`).

- **Why is there no CI workflow for any of the four crates in this subsystem?** None of `astraweave-net-ecs`, `aw-net-proto`, `aw-net-client`, `aw-net-server` appear in any `.github/workflows/*.yml`. **Investigation (2026-05-12):** Confirmed via `grep -rn "aw-net-server\|aw-net-client\|aw_net_proto\|astraweave_net_ecs\|astraweave-net-ecs" .github/` — zero matches across the entire `.github/` directory tree (27 workflow files exist in `.github/workflows/`). The older `astraweave-net` has a dedicated `net-tests.yml`. Is the absence intentional (binaries are validated by manual smoke tests) or a CI-pipeline gap?

- **Should `handle_socket_tls` / `handle_socket` be unified? — RESOLVED** at `de6f2725`. `handle_socket` is generic over `AsyncRead + AsyncWrite + Unpin`. `handle_socket_tls` delegates. `PlayerMembership` is the shared cleanup.

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
- ~~TLS stack versions are bumped to align with workspace (§6 TLS-version row).~~ **DONE.** `aw-net-server` matches `astraweave-net` on tokio-rustls 0.26 / rustls 0.23.

**Verification process:**
- `rg 'pub fn|pub struct|pub enum|pub trait' astraweave-net-ecs/src/lib.rs net/aw-net-proto/src/lib.rs` should match §3 vocabulary surface.
- `cargo tree -p astraweave-net-ecs --depth 1` should list `aw-net-proto`, `astraweave-ecs`, `astraweave-core`, `postcard`, `lz4_flex`, `crc32fast`, `glam`, `bincode`, `tokio`, `futures-util`, `tokio-tungstenite`, `anyhow`, `serde`, `serde_json`.
- `cargo tree -p aw-net-server --depth 1` should list `aw-net-proto`, `tokio`, `axum`, `hyper`, `tokio-tungstenite`, `tungstenite`, `futures`, `serde`, `serde_json`, `anyhow`, `thiserror`, `tracing`, `tracing-subscriber`, `parking_lot`, `uuid`, `time`, `sled`, `postcard`, `lz4_flex`, `tokio-rustls`, `rustls`. (`hmac`/`sha2` now live in `aw-net-proto`, not the server.)
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
5. **Token bucket is 64/s, capacity 64.** A 30 Hz input pays every frame. About 96 messages/s closes 1008. `RateLimited` is not sent.
6. **`lib_temp.rs` is orphan source** — not declared, not compiled.
7. **None of these four crates are in CI** as of `a2474c5b7`.

**Files you'll most likely touch:**
- `net/aw-net-proto/src/lib.rs` — wire-protocol changes (always coordinated with both client and server).
- `net/aw-net-server/src/lib.rs` — matchmaking, rate-limit, HMAC verification, snapshot construction. Session behavior lives in `handle_socket` and `on_client_msg`. `handle_socket_tls` only delegates.
- `net/aw-net-client/src/main.rs` — client input/reconciliation flow.
- `astraweave-net-ecs/src/lib.rs` — ECS Plugin layer (stub systems and async helpers).

**Files you should NOT touch without strong reason:**
- `astraweave-net-ecs/src/lib_temp.rs` — orphan duplicate; modifications don't compile into the crate.
- `net/aw-net-proto/tests/mutation_resistant_comprehensive_tests.rs` and `astraweave-net-ecs/tests/mutation_resistant_comprehensive_tests.rs` — mutation-resistance assertions; changes here can mask real bugs.
- `aw_net_server_db/conf`, `aw_net_server_db/db` — sled runtime artifacts. Delete only if explicitly resetting the dev server's state.

**Common mistakes when changing this system:**
- **Adding a `ClientToServer` variant without updating `handle_socket` and `on_client_msg`.** Plain and TLS share those functions. `PlayerMembership` must stay bound until `handle_socket` returns, or a failed join send leaves the player in the room.
- **Adding a `ServerToClient` variant without updating the client's match arms in `net/aw-net-client/src/main.rs:63-75` and the per-tick read loop.**
- **Changing `Codec` choice in one path without changing the other.** The Plugin layer's Bincode and the standalone binaries' PostcardLz4 must agree if you want them to talk.
- **Assuming the HMAC verification is observed-only.** It isn't anymore — under the default `Kick` policy a bad signature rejects the packet and closes the connection (1008). It only logs-and-continues under the explicit `Warn` policy. Verification also runs FIRST, before any state mutation, so don't reorder it after the rate-limit/`last_input_seq` updates.
- **Trusting `app.db` to persist anything.** It doesn't — no reads or writes go through it.
- **Adding `unsafe` to `astraweave-net-ecs/src/lib.rs` or `aw-net-proto/src/lib.rs`.** Both crate roots carry `#![forbid(unsafe_code)]`.

---

## Appendix B: Historical context

The standalone trio (`net/aw-net-{proto,client,server}`) was introduced as a single squashed PR on **2025-09-09** in commit `cc9a7e3e3` ("Implement production-ready enhanced networking layer with server authority, client prediction, and matchmaking (#58)"). This predates the ECS Plugin layer.

The ECS Plugin layer (`astraweave-net-ecs`) was added on **2025-10-01** in commit `08befc6ec` ("phase 6 implementation") — roughly three weeks after the standalone trio. By that point, the standalone server had already adopted HMAC verification while the client still computed the XOR `sign16` — the mismatch that caused every verification to fail. This was finally reconciled in the **Net-Trio-Remediation (2026-06)**: canonical HMAC-SHA256 signing in `aw-net-proto`, adopted by both client and server, enforced kick-by-default; `sign16`/`SessionKey`/`session_key_hint` deleted (`561b20957`+`79424389e`+`066cd6cfd`).

The `lib_temp.rs` orphan was added on **2025-11-19** in commit `54d15c9f2` ("added forest biome textures and assets, worked on renderer"). The commit title gives no clue about why a temp file was added to the netcode crate — likely an unintentional checkin during a renderer-focused commit.

The older `astraweave-net` snapshot subsystem (documented separately in `docs/architecture/net.md`) was created 2025-09-05 and remains in place. Per `net/README.md:181`, the two networking systems are intended to coexist long-term.

The `aw_net_server_db/` directory is created on first server run by sled's `Db::open` call. Its presence in git (`conf` 62 B, `db` 96 B) indicates someone ran the server locally and committed the artifacts; the directory is not in any `.gitignore`.
