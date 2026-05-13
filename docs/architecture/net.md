# Architecture Trace: Net (Snapshot-Based Game Server)

> **Scope note:** This doc traces the `astraweave-net` crate — the original snapshot-based networking implementation. AstraWeave's "networking" domain comprises three loosely-coupled subsystems; the other two have dedicated traces:
> - `docs/architecture/net_ecs.md` — `astraweave-net-ecs` + `net/aw-net-{proto,client,server}` (ECS-Plugin + matchmaking + standalone binaries)
> - `docs/architecture/persistence_ecs.md` — `astraweave-persistence-ecs` (save/load)

## Metadata

| Field | Value |
|---|---|
| **System name** | Net (snapshot-based game server) |
| **Primary crates** | `astraweave-net` (sole crate in this subsystem) |
| **Document version** | 1.2 |
| **Last verified against commit** | `a2474c5b7` |
| **Last verified date** | 2026-05-12 |
| **Status** | Active |
| **Revision history** | 1.2 (2026-05-12): Deep investigation pass. **Major new finding**: the aspirational `docs/src/core-systems/networking.md` doc claims AstraWeave uses **QUIC (via Quinn)** for transport — but the actual implementation uses **WebSocket (tokio-tungstenite over TCP)**. This is an architectural-class mismatch beyond the type-name mismatches already documented; added to §6. Enriched §11 Q2, Q6, Q8, Q9 with concrete factual context (test counts, call-site counts, exact line refs). Q6 (replay log) factually closed at the code level: 2 push sites (`lib.rs:837, 868`), 0 remove sites anywhere — confirmed truly unbounded. Q9 (webpki-roots) clarified as **client-side-only** — `TLS_SERVER_ROOTS` is imported only at `tls.rs:33, 119`, both inside `TlsClientConfig::default_connector`. Q8 (Box<dyn Interest>) quantified — 2 allocation sites per broadcast per connection. Added note that `tls.rs` exports **5 public methods** (not the 2 the §1 summary highlights): `TlsServerConfig::{from_pem_files, acceptor}`, `TlsClientConfig::{default_connector, with_custom_ca, insecure_connector}`.<br><br>1.1 (2026-05-12): Verification pass. **Corrected §4 / Appendix B**: the prior `[INFERRED]` claim of "no aspirational doc tree" was wrong — `docs/src/core-systems/networking.md` exists and references 7+ nonexistent submodules (`replication`, `state`, `delta`, `serialization`, `prediction`, plus types `Server`/`ServerConfig`/`Client`/`ClientConfig`/`ClientEvent`). Same `28bc94f21` "Create comprehensive bespoke wiki" sweep that created the aspirational audio/input docs. Added new §6 row. Resolved §10 sanitizer `[INFERRED]` — `astraweave-net` is explicitly listed in the `P1_CRATES` array at `sanitizers.yml:205`. Corrected Appendix B creation date: first commit was 2025-09-04 (`ba52548b3`), not 2025-09-05. |
| **Owner notes** | Self-contained snapshot-server library. Three direct production consumers (`examples/coop_server`, `examples/coop_client`, `examples/net_headless_sim`). Optional `tls` feature for secure WebSocket. Dedicated `.github/workflows/net-tests.yml` workflow. Not an ECS-Plugin — `astraweave-net` predates and operates parallel to the ECS-Plugin networking in `astraweave-net-ecs`. |

---

## 1. Executive Summary

**What this system does:**
Implements an authoritative-server multiplayer model for AstraWeave's grid-based `astraweave-core::World`. Builds canonical `Snapshot`s of `(entities, obstacles)`, diffs them into `Delta`s, filters per-viewer via pluggable `Interest` policies (radius / FoV / FoV+LoS), and serves them over WebSocket with optional TLS. Validates incoming client `PlanIntent`s and records them into a deterministic `ReplayEvent` log.

**Why it exists:**
Provides authoritative multiplayer for the legacy `World`-based runtime (grid coordinates, `i32` tile positions, simple team/HP/ammo state). The model predates the ECS architecture and serves as the proof-of-concept for AstraWeave's networking + replay determinism story.

**Where it primarily lives:**
- `astraweave-net/src/lib.rs` — 932 lines. Owns `Snapshot`, `Delta`, `EntityState`, `Interest` trait + 4 impls, `InterestPolicy` enum, `GameServer` struct, `Msg` enum (wire), `ServerEvent` enum (internal), `ReplayEvent`, `build_snapshot`, `filter_snapshot_for_viewer`, `diff_snapshots`, `apply_delta`, `replay_from`.
- `astraweave-net/src/tls.rs` — 258 lines. Feature-gated (`tls`). `TlsServerConfig::from_pem_files`, `TlsClientConfig::default_connector`. Uses `tokio-rustls` with `ring` crypto.
- `astraweave-net/src/error.rs` — 43 lines. `NetError` enum (7 non-`Other` variants), `NetResult<T>` alias.
- `astraweave-net/src/tests.rs` — 999 lines (38 tests).
- `astraweave-net/src/mutation_tests.rs` — 1087 lines (50 tests).
- `astraweave-net/tests/` — 11 integration test files plus `integration/` subdirectory.
- `astraweave-net/benches/net_bench.rs` — Criterion benchmarks.
- `astraweave-net/fuzz/fuzz_targets/` — 4 cargo-fuzz targets (delta compression, interest management, packet parsing, snapshot serialization).

**Status note:**
Active and feature-complete for its grid-`World` data model. Co-exists with `astraweave-net-ecs` (the ECS-Plugin networking layer) which uses a different data model (`Vec3` positions, binary postcard wire format, matchmaking-room protocol). Both systems live in the workspace; neither imports the other.

---

## 2. Authoritative Pipeline

```text
[astraweave-core::World — grid-based, tick-driven]
    │
    │ build_snapshot(world, tick, seq)        (lib.rs:293-305)
    ▼
[Snapshot { version, tick, t, seq, world_hash, entities: Vec<EntityState> }]
    │
    │ Server side per-tick loop in GameServer::run_ws_on_listener        (lib.rs:539-593)
    │  · 60 Hz fixed-tick (Duration::from_micros(16_666))
    │  · world.tick(dt) under Mutex
    │  · snapshot built each tick, obstacle cache updated
    │  · broadcast cadence:
    │      tick % 60 == 0      → ServerEvent::Snapshot (full)
    │      tick % 3 == 0 else   → ServerEvent::Snapshot (delta-eligible)
    │
    ▼
[broadcast::Sender<ServerEvent>]                (lib.rs:499, 547)
    │
    │ Per-connection writer task (subscribed to rx_bcast)    (lib.rs:612-771)
    │  · pulls viewer_id from Mutex (set on ClientHello)
    │  · pulls policy: InterestPolicy from Mutex
    │  · builds Box<dyn Interest> per snapshot
    │
    ▼
[filter_snapshot_for_viewer(head, &interest, viewer)]     (lib.rs:307-322)
    │   → Snapshot with entities filtered by interest.include(viewer, e)
    │   → world_hash recomputed via subset_hash on filtered entities
    │
    ▼
[Has last_sent snapshot?]
    │
    ├── No  → ServerSnapshot { snap: filtered }     (full)
    │        last_sent = Some(filtered)
    │
    └── Yes → diff_snapshots(base=last_sent, head=filtered, FullInterest, viewer)    (lib.rs:324-401)
              │   → Delta { base_tick, tick, changed: Vec<EntityDelta>, removed: Vec<u32>, head_hash }
              │
              ├── delta empty?      → skip send (continue)
              │
              └── send ServerDelta { delta }
                  last_sent = Some(filtered)

──────────────────────────────────────────────────────────────────────
Client → Server (JSON over WebSocket text frames; lib.rs:773-896)
    │
    ├── ClientHello { name, token, policy }
    │     · name → viewer_id (player/comp/enemy mapped at lib.rs:791-796)
    │     · token: must equal "dev" to be "authenticated" (lib.rs:785-789, logs only)
    │     · policy: "radius" | "fov" | "fovlos" → InterestPolicy (lib.rs:801-816)
    │     · Sends ServerEvent::ForceSnapshot via broadcast → writer task picks it up
    │
    ├── ClientProposePlan { actor_id, intent }
    │     · Locks world.Mutex; calls validate_and_execute (astraweave-core)
    │     · ValidateCfg { world_bounds: (0, 0, 19, 9) }       ← hardcoded
    │     · On success or failure: append ReplayEvent to replay log
    │     · Broadcasts ServerEvent::Snapshot + ServerEvent::ApplyResult
    │
    └── ClientInput { seq, tick, actor_id, intent }
          · Same flow as ClientProposePlan
          · Additionally broadcasts ServerEvent::Ack { seq, tick_applied }

──────────────────────────────────────────────────────────────────────
Client side (consumers — only example consumers exist)
    │
    │ examples/coop_client/src/main.rs:3   uses { apply_delta, Msg, Snapshot }
    │
    ▼
[Receive Msg::ServerSnapshot { snap }] → store as base snapshot
[Receive Msg::ServerDelta   { delta }] → apply_delta(&mut base, &delta)    (lib.rs:403-445)
    │
    └── If base.tick != delta.base_tick: silent no-op (no error)

──────────────────────────────────────────────────────────────────────
Deterministic replay path
    │
    │ replay_from(world, events)            (lib.rs:911-932)
    ▼
[Sort events by (tick, seq)]
    │
    │ For each event:
    │   · advance world.tick(dt) until current_tick == event.tick
    │   · validate_and_execute(event.intent) — Result is discarded
    │
    ▼
[build_snapshot at final tick → return snap.world_hash]
    │
    ├── examples/net_headless_sim verifies: baseline_hash vs final_hash for short replay log
    │   (`net_headless_sim/src/main.rs:32`)

──────────────────────────────────────────────────────────────────────
Optional TLS path (feature = "tls")
    │
    │ astraweave-net/src/tls.rs
    ▼
[TlsServerConfig::from_pem_files(cert.pem, key.pem)]    (tls.rs:56-)
    │   → Reads cert chain + private key with rustls_pemfile
    │   → Builds rustls::ServerConfig with ring crypto backend
    │   → Wraps in TlsAcceptor via .acceptor()
    │
[TlsClientConfig::default_connector]                    (tls.rs)
    │   → Builds ClientConfig with TLS_SERVER_ROOTS from webpki-roots
    │   → Wraps in TlsConnector
    │
   *Note:* The doc-comment examples (tls.rs:7-21) show how callers would wire this into
   `tokio-tungstenite::accept_async_with_tls_acceptor` / `connect_async_tls_with_config`, but
   `GameServer::run_ws_on_listener` uses plain `tokio_tungstenite::accept_async` (lib.rs:596) —
   no TLS wiring inside `GameServer`. TLS is exposed as a library primitive; callers must
   wire it themselves outside `astraweave-net`.
```

### Stage-by-stage detail

#### Stage 1: World → Snapshot construction (`lib.rs:263-305`)
**Role:** Convert a mutable `astraweave-core::World` into an immutable `Snapshot` value with a deterministic content hash.
**Inputs:** `&World`, `tick: u64`, `seq: u32`.
**Outputs:** `Snapshot { version: SNAPSHOT_VERSION (=1), tick, t: world.t, seq, world_hash, entities }`.
**Notes:** `world_to_entities` (lib.rs:263-287) sorts entity ids and collects `(pos, hp, team, ammo)` for each. `stable_hash_snapshot` (lib.rs:228-243) feeds the entity list AND the obstacle set into `DefaultHasher` to produce `world_hash` — both pieces must be stable-ordered. Used by both the broadcast loop and the replay determinism check.

#### Stage 2: Interest filtering (`lib.rs:85-208, 307-322`)
**Role:** Restrict the entity set seen by one viewer to those that pass an `Interest` policy.
**Inputs:** Head snapshot, an `Interest` trait object, the viewer's own `EntityState`.
**Outputs:** Filtered `Snapshot` with `world_hash` recomputed via `subset_hash` (lib.rs:245-256) over the *filtered* entity set (note: this is a *different* hash from `stable_hash_snapshot`, since `subset_hash` omits the obstacles).
**Notes:** Four `Interest` impls exist:
- `FullInterest` (lib.rs:89-94): always include.
- `RadiusTeamInterest { radius }` (lib.rs:96-108): same team OR within euclidean grid radius.
- `FovInterest { radius, half_angle_deg, facing }` (lib.rs:110-141): same team OR within radius AND within FoV cone.
- `FovLosInterest { radius, half_angle_deg, facing, obstacles }` (lib.rs:143-208): adds Bresenham line-of-sight check (lib.rs:150-180) against an obstacle `BTreeSet`.

The runtime selects via the `InterestPolicy` enum (lib.rs:212-226): `Radius | Fov | FovLos`. `FovLos` pulls obstacles from `GameServer.obstacles` (an `Arc<Mutex<BTreeSet<(i32, i32)>>>`) which is refreshed each tick from `world.obstacles`.

#### Stage 3: Delta diffing (`lib.rs:324-401`)
**Role:** Compute the minimal per-entity diff between two snapshots.
**Inputs:** `&base`, `&head`, `&impl Interest`, `&viewer`.
**Outputs:** `Delta { base_tick, tick, changed: Vec<EntityDelta>, removed: Vec<u32>, head_hash }`.
**Notes:** `EntityDeltaMask` bitfield (lib.rs:59-64) packs which fields changed: `POS | HP | TEAM | AMMO`. New entities (not in `base_map`) get all four bits set with all fields populated. Entities in `base` but not in `head` go into `removed`. Empty diffs (no `changed`, no `removed`) cause the writer task to skip the send (lib.rs:713-715).

#### Stage 4: Delta application (`lib.rs:403-445`)
**Role:** Apply a `Delta` to a base snapshot in place.
**Inputs:** `&mut Snapshot` (base), `&Delta`.
**Outputs:** mutates base.
**Notes:** **If `base.tick != delta.base_tick`, the function returns silently** (lib.rs:404-406), without error. A client that misses a snapshot will continue to apply mismatched deltas to nothing — the only signal is the next world_hash mismatch. Missing entities in `changed` get default-zeroed before mask application (lib.rs:410-416).

#### Stage 5: Per-tick server loop (`lib.rs:539-593`)
**Role:** Fixed-tick 60 Hz world update + snapshot broadcast.
**Inputs:** The bound `TcpListener`.
**Outputs:** `ServerEvent::Snapshot` over `broadcast::Sender<ServerEvent>` (1024-slot channel; lib.rs:520).
**Notes:** The loop uses `next += Duration::from_micros(16_666)` (lib.rs:558) to drift-correct. Full snapshots emit every 60 ticks (~once per second); delta-eligible snapshots emit every 3 ticks (~20 Hz). The per-connection writer decides between full vs. delta based on whether `last_sent.is_some()`.

#### Stage 6: Per-connection handling (`lib.rs:595-898`)
**Role:** Accept one WebSocket connection, send `ServerWelcome`, spawn a snapshot-writer task subscribed to the broadcast, and read incoming `Msg` JSON frames until close.
**Inputs:** A `tokio::net::TcpStream`.
**Outputs:** A long-running task pair (writer + reader) until close or error.
**Notes:**
- The writer task ends if the broadcast receiver returns `Err(Lagged(_))` it continues (lib.rs:618); other errors break the loop.
- `ServerEvent::ForceSnapshot` (lib.rs:621-670) is the path used after `ClientHello` to give a brand-new client an immediate full snapshot regardless of normal broadcast cadence.
- Five `Msg` variants are explicitly server-side-only (`ServerWelcome`, `ServerSnapshot`, `ServerDelta`, `ServerApplyResult`, `ServerAck`); the server ignores them if received from a client (lib.rs:885-891).
- Errors during writes use `let _ = tx.send(...)` (lib.rs:665-667, 743-745, 751-757, 761-767) — write failures are not surfaced beyond stdout logging.

#### Stage 7: Replay determinism (`lib.rs:901-932`)
**Role:** Given a `World` and a list of `ReplayEvent`s, replay them deterministically and return the final `world_hash`.
**Inputs:** Owned `World`, `&[ReplayEvent]`.
**Outputs:** `Result<u64>` (the final hash).
**Notes:** Events are sorted by `(tick, seq)` before replay. The world's `dt = 1/60` matches the live server's tick rate. `validate_and_execute`'s `Result` is *discarded* inside the replay loop (lib.rs:928: `let _ = ...`) — replay does not bail on a single failing input, matching the server's tolerant behavior.

#### Stage 8: TLS configuration (`tls.rs`, feature `tls`)
**Role:** Build rustls `ServerConfig` from PEM files (server) or a default `ClientConfig` trusting webpki roots (client).
**Inputs:** `cert.pem` + `key.pem` paths (server); none (client).
**Outputs:** `TlsAcceptor` / `TlsConnector`.
**Notes:** Uses `ring` crypto backend (`Cargo.toml:24` features `["ring"]`). No default-features on rustls (line 25 `default-features = false`) to ensure ring is the sole crypto provider for Windows compatibility. The module is consumed only via doc-comment examples (`tls.rs:7-21`); `GameServer::run_ws_on_listener` uses plain `accept_async` and does not invoke any TLS path. Callers wanting TLS must wire the acceptor/connector themselves.

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **Snapshot** | Immutable view of the world at one tick: `{ version, tick, t, seq, world_hash, entities }`. | `lib.rs:47-54` |
| **Delta** | Minimal diff between two snapshots: `{ base_tick, tick, changed, removed, head_hash }`. | `lib.rs:77-83` |
| **EntityState** | Per-entity payload: `{ id: u32, pos: IVec2, hp: i32, team: u8, ammo: i32 }`. Note: 2D grid, NOT 3D. | `lib.rs:38-44` |
| **EntityDelta** | One entity's diff: `{ id, mask: u8 (POS/HP/TEAM/AMMO bitfield), pos, hp, team, ammo }`. | `lib.rs:67-74` |
| **Interest** | Trait deciding "should this entity be visible to this viewer?". | `lib.rs:85-87` |
| **InterestPolicy** | Enum picking which Interest to use at runtime: `Radius | Fov | FovLos`. | `lib.rs:212-226` |
| **world_hash** | `DefaultHasher` digest of the entity list + obstacles, used for replay-determinism verification. Two flavors: `stable_hash_snapshot` (includes obstacles), `subset_hash` (entities only — used after filtering). | `lib.rs:228-256` |
| **Msg** | Wire-frame enum serialized as JSON; tagged via `#[serde(tag = "type")]`. 8 variants. | `lib.rs:447-483` |
| **ServerEvent** | Internal broadcast channel payload; 4 variants. Not on the wire — the writer task translates these into `Msg` frames. | `lib.rs:485-492` |
| **GameServer** | The server runtime: world Mutex + viewer ids + broadcast tx + tick counter + replay log + obstacles cache. | `lib.rs:494-503` |
| **ReplayEvent** | Recorded input for deterministic replay: `{ tick, seq, actor_id, intent: PlanIntent, world_hash }`. | `lib.rs:901-908` |
| **ValidateCfg** | World-bounds config passed into `validate_and_execute` — hardcoded as `(0, 0, 19, 9)` everywhere in `astraweave-net`. | `lib.rs:826-828, 857-859` |

### Terms to NOT confuse

- **`Msg::ServerSnapshot` (wire) vs. `ServerEvent::Snapshot` (internal):** The broadcast channel emits `ServerEvent`s. The per-connection writer task **translates** these into `Msg::ServerSnapshot` (for full sends) or `Msg::ServerDelta` (when a base exists). Five `Msg::Server*` variants exist on the wire; only four `ServerEvent` variants exist internally — `Msg::ServerSnapshot` and `Msg::ServerDelta` both map from `ServerEvent::Snapshot`.

- **`ServerEvent::Snapshot` vs. `ServerEvent::ForceSnapshot`:** Both deliver a `Snapshot` to the writer task. `Snapshot` participates in the diff-when-possible logic (writer compares against `last_sent`); `ForceSnapshot` always emits a full `Msg::ServerSnapshot` and skips the diff path entirely. Used by `ClientHello` to bootstrap a new client (lib.rs:821).

- **`stable_hash_snapshot` (entities + obstacles) vs. `subset_hash` (entities only):** `stable_hash_snapshot` is used at `build_snapshot` time and includes the world's obstacle layout. `subset_hash` is used at `filter_snapshot_for_viewer` time and in `diff_snapshots`'s `head_hash` — it omits obstacles, since the filtering result depends only on the visible entity set.

- **`Interest` (trait, runtime polymorphism) vs. `InterestPolicy` (enum, config):** Each per-connection task builds a fresh `Box<dyn Interest>` per snapshot from the policy held in `Arc<Mutex<InterestPolicy>>`. Policies are settable by the client via `ClientHello { policy }`.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-core` (direct dep, `Cargo.toml:20`) | `World`, `IVec2`, `Team`, `PlanIntent`, `ActionStep`, `ValidateCfg`, `validate_and_execute(&mut World, actor_id, &PlanIntent, &ValidateCfg, &mut log)` | Tick-driven grid world; movement / action validation | `GameServer::new` (lib.rs:512-531) constructs a hardcoded test world with three entities (P, C, E) and a wall of obstacles at x=6. `validate_and_execute` is called from both `ClientProposePlan` and `ClientInput` handlers (lib.rs:829, 860). |
| Filesystem (TLS certs) | `TlsServerConfig::from_pem_files("cert.pem", "key.pem")` (`tls.rs:56-`) | PEM-encoded X.509 cert chain + private key | Feature-gated (`tls`). Webpki trust roots for client side embedded via `webpki-roots` crate. |
| WebSocket client connections | `tokio_tungstenite::accept_async(TcpStream) -> WebSocketStream` (lib.rs:596) | JSON-encoded `Msg::Client*` frames as `Message::Text` | Server uses plain TCP; TLS-wrapping is library-exposed but not wired inside `GameServer`. |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| `examples/coop_server` (`Cargo.toml:10`) | `astraweave_net::GameServer::new()`, `.run_ws(addr)` (`coop_server/src/main.rs:1`) | Stand-up of a running server | The flagship server demo. |
| `examples/coop_client` (`Cargo.toml:15`) | `astraweave_net::{Msg, Snapshot, apply_delta}` (`coop_client/src/main.rs:3`) | Receives JSON `Msg` frames; applies `Delta` to a maintained `Snapshot` | The flagship client demo. |
| `examples/net_headless_sim` (`Cargo.toml:12`) | `build_snapshot`, `replay_from`, `ReplayEvent` (`net_headless_sim/src/main.rs:3`) | Determinism check: replay a fixed event log and compare `world_hash`. | Headless smoke test, not a true game client. |
| Fuzz harness (`astraweave-net/fuzz/`) | 4 fuzz targets: `fuzz_delta_compression`, `fuzz_interest_management`, `fuzz_packet_parsing`, `fuzz_snapshot_serialization` | Random byte streams interpreted as `Msg` / `Snapshot` / `Delta` | cargo-fuzz. Not run in standard CI. |

### Bidirectional / Coupled

- **`GameServer` ↔ `astraweave-core::World`:** The server owns the canonical `World` in `Mutex<World>` and ticks it at 60 Hz. Every client input feeds through `validate_and_execute` against that `World`. Server snapshots are pure functions of the `World` state (Stage 1).

### Documentation references with no code backing

- **`docs/src/core-systems/networking.md`** references many nonexistent submodules and types in `astraweave_net`: `Server`, `ServerConfig`, `Client`, `ClientConfig`, `ClientId`, `ClientEvent`, `astraweave_net::replication::{NetEntity, ReplicationMode}`, `astraweave_net::state::{WorldState, EntityState}` (note: `EntityState` exists but at crate root, not in a `state` submodule), `astraweave_net::delta::{DeltaEncoder, DeltaDecoder}`, `astraweave_net::serialization::{BitWriter, BitReader}`, `astraweave_net::prediction::{PredictionSystem, InputBuffer}`. None of these submodules exist in the actual crate (which has only `error`, `tls`, and inline modules). Origin: `git log --diff-filter=A` traces the file to commit `28bc94f21` (2025-09-08, "Create comprehensive bespoke wiki with 51-section documentation structure (#34)") — the same AI-generated documentation sweep that produced the aspirational audio/input/net-ecs docs covered in other traces. See §6 row.

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-net/src/lib.rs` | All public types (`Snapshot`, `Delta`, `EntityState`, `Interest`+4 impls, `InterestPolicy`, `GameServer`, `Msg`, `ServerEvent`, `ReplayEvent`) + the core pipeline functions (`build_snapshot`, `filter_snapshot_for_viewer`, `diff_snapshots`, `apply_delta`, `replay_from`) + the WebSocket server loop. | Active | 932 lines. `#![forbid(unsafe_code)]` (line 1). |
| `astraweave-net/src/error.rs` | `NetError` enum (7 variants) + `NetResult<T>` alias. | Active | 43 lines. `#[non_exhaustive]` + `#[must_use]`. |
| `astraweave-net/src/tls.rs` | `TlsServerConfig::{from_pem_files, acceptor}` (server-side) + `TlsClientConfig::{default_connector, with_custom_ca, insecure_connector}` (client-side). 5 public methods total. | Active (feature-gated) | 258 lines. Compiled only when `tls` feature enabled. Library primitives — not wired into `GameServer` directly. `webpki_roots::TLS_SERVER_ROOTS` is imported at line 33 and used only at line 119 inside `default_connector` (client-side trust store). |
| `astraweave-net/src/tests.rs` | Unit tests for snapshot/delta/replay/interest. | Active (tests) | 999 lines, 38 tests. Included via `#[cfg(test)] mod tests` at `lib.rs:260-261`. |
| `astraweave-net/src/mutation_tests.rs` | Mutation-resistance harness targeting cargo-mutants survivors. | Active (tests) | 1087 lines, 50 tests. Included via `#[cfg(test)] mod mutation_tests` at `lib.rs:258-259`. |
| `astraweave-net/tests/interest_coverage_tests.rs` | Coverage tests for the 4 Interest impls. | Active (tests) | 19 tests including FoV-LoS Bresenham edge cases. |
| `astraweave-net/tests/mutation_resistant_comprehensive_tests.rs` | Second mutation-resistance pass. | Active (tests) | 129 tests. |
| `astraweave-net/tests/property_tests.rs` | proptest-based property tests. | Active (tests) | 23 tests. Uses `proptest = "1.5"` (Cargo.toml:31). |
| `astraweave-net/tests/property_tests_extended.rs` | Extended proptest suite. | Active (tests) | 14 tests + checked-in `property_tests_extended.proptest-regressions` file. |
| `astraweave-net/tests/boundary_condition_tests.rs` | Edge-case unit tests. | Active (tests) | 6 tests. |
| `astraweave-net/tests/concurrent_stress_tests.rs` | Concurrency stress harness. | Active (tests) | 1 test (gated on tokio runtime). |
| `astraweave-net/tests/coverage_booster_tests.rs` | Coverage-targeted miscellanea. | Active (tests) | 4 tests. |
| `astraweave-net/tests/error_message_validation_tests.rs` | Asserts on `NetError` Display strings. | Active (tests) | 1 test. |
| `astraweave-net/tests/integration_tests.rs` | Integration-test main file. | Active (tests) | Imports submodules below; contains no own `#[test]`. |
| `astraweave-net/tests/integration/sync_tests.rs` | Snapshot ↔ client sync flow tests. | Active (tests) | 25 tests. |
| `astraweave-net/tests/integration/snapshot_sync_tests.rs` | Snapshot sequencing tests. | Active (tests) | 6 tests. |
| `astraweave-net/tests/integration/packet_loss_tests.rs` | Drop-and-recover behavior over `Delta` chain. | Active (tests) | 25 tests. |
| `astraweave-net/tests/integration/server_logic_tests.rs` | `Msg` handler logic tests. | Active (tests) | 3 tests. |
| `astraweave-net/tests/integration/auth_tests.rs` | Authentication tests (using the "dev" token convention). | Active (tests) | 30 tests. |
| `astraweave-net/tests/integration/mod.rs` | Integration submodule glue. | Active (tests) | 0 tests. |
| `astraweave-net/benches/net_bench.rs` | Criterion benches: snapshot construction, delta diff/apply, interest filtering. | Active | Run via `cargo bench -p astraweave-net`. |
| `astraweave-net/fuzz/Cargo.toml` + `fuzz/fuzz_targets/*.rs` | 4 cargo-fuzz targets: `fuzz_delta_compression`, `fuzz_interest_management`, `fuzz_packet_parsing`, `fuzz_snapshot_serialization`. | Active (off-CI) | Run on demand with `cargo +nightly fuzz run <target>`. Not in any GitHub Actions workflow as of `a2474c5b7` (verified via `grep -l fuzz .github/workflows/*.yml`). |

**Status definitions:**
- **Active**: Canonical, load-bearing, edit freely with care.
- **Active (tests)**: Carries no runtime weight but exercises invariants.
- **Active (feature-gated)**: Only compiled with a feature flag; not on the default build.
- **Active (off-CI)**: Exists and works but is not part of automated CI.

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| `astraweave-net::EntityState { pos: IVec2, hp, team, ammo }` vs. `astraweave-net-ecs::EntityState { position: Vec3, health }` | `astraweave-net/src/lib.rs:38-44` vs. `astraweave-net-ecs/src/lib.rs:43-47` | Parallel data models | The two networking subsystems use disjoint per-entity state types. `astraweave-net` models grid-aligned 2D positions (`IVec2`, tile coordinates) plus team + ammo; `astraweave-net-ecs` models continuous 3D positions and tracks only health. Neither crate imports the other. |
| `astraweave-net::Msg` (JSON) vs. `aw-net-proto::{ClientToServer, ServerToClient}` (postcard + lz4) | `astraweave-net/src/lib.rs:447-483` vs. `net/aw-net-proto/src/lib.rs:22-` | Parallel wire formats | `astraweave-net` uses `serde_json::to_string` + `Message::Text` (lib.rs:602, 666, 721, 734, 744, 754, 765). `aw-net-proto` defines a postcard-encoded enum with optional lz4 compression and crc32 framing. Distinct envelopes; no shared dispatcher. |
| `GameServer` (this crate) vs. `net/aw-net-server` standalone binary | `astraweave-net/src/lib.rs:494-899` vs. `net/aw-net-server/src/main.rs` (852 lines) | Two server implementations | This crate provides a library `GameServer` consumed by `examples/coop_server`. `net/aw-net-server` is a standalone binary with matchmaking + room management using `aw-net-proto`. They are completely independent processes. |
| `astraweave-net::tls` vs. `GameServer::run_ws_on_listener` | `astraweave-net/src/tls.rs` vs. `astraweave-net/src/lib.rs:596` | TLS available but not wired | `tls.rs` exposes `TlsAcceptor` / `TlsConnector` library primitives. `GameServer` uses plain `tokio_tungstenite::accept_async`. The doc comment at `tls.rs:7-21` shows how a caller would compose them, but no production code in `astraweave-net` itself does this wiring. The optional `webpki-roots` dep (`Cargo.toml:27`) is server-config-only and unused by client code in this crate (`coop_client`). |
| Two hashing strategies (`stable_hash_snapshot` vs. `subset_hash`) | `lib.rs:228-256` | Intentional split | `stable_hash_snapshot` includes obstacles; used in `build_snapshot` (lib.rs:296) for the canonical `world_hash`. `subset_hash` omits obstacles; used in `filter_snapshot_for_viewer` (lib.rs:320) and in `diff_snapshots`'s `head_hash` (lib.rs:393). Filtering produces a different hash than the source snapshot's hash, which means a delta's `head_hash` is not directly comparable to a snapshot's `world_hash` after filtering. |
| Aspirational `docs/src/core-systems/networking.md` API surface (`Server`, `ServerConfig`, `Client`, `ClientConfig`, `ClientId`, `ClientEvent`, `replication::{NetEntity, ReplicationMode}`, `state::{WorldState, EntityState}`, `delta::{DeltaEncoder, DeltaDecoder}`, `serialization::{BitWriter, BitReader}`, `prediction::{PredictionSystem, InputBuffer}`) | `docs/src/core-systems/networking.md` | Reference-only, code-absent | None of these submodules exist in `astraweave-net`. Origin: commit `28bc94f21` (2025-09-08, "Create comprehensive bespoke wiki with 51-section documentation structure (#34)") authored by `Copilot <198982749+Copilot@users.noreply.github.com>` — the same bulk AI-generated documentation sweep that introduced aspirational audio/input/net-ecs docs (per their respective traces). |
| Architectural mismatch in same aspirational doc: claims **QUIC (via Quinn)** transport | `docs/src/core-systems/networking.md:28-33` | Doc-vs-code architectural disagreement | The aspirational doc states: "Transport Layer: UDP-based reliable/unreliable messaging with Quinn (QUIC)" and "AstraWeave uses QUIC (via Quinn) for modern, secure, and multiplexed networking with built-in congestion control and 0-RTT connection establishment." The actual `astraweave-net` implementation uses **WebSocket over TCP** via `tokio-tungstenite` (`Cargo.toml:18-19`, `lib.rs:596`). No `quinn` dependency exists in `astraweave-net/Cargo.toml`. This is more than a type-name mismatch — it's an architectural-class disagreement (TCP+WS vs UDP+QUIC). |

### Naming collisions

- **`EntityState` (this crate, `lib.rs:38-44`) vs. `EntityState` (in `astraweave-net-ecs/src/lib.rs:43-47`):** Same name, completely disjoint shapes. Both are public types. A consumer that imports `astraweave_net::EntityState` and `astraweave_net_ecs::EntityState` simultaneously must qualify each.
- **`Snapshot` (this crate) vs. `NetworkSnapshot` (in `astraweave-net-ecs`):** The ECS-Plugin layer disambiguated by prefixing "Network", but this crate took the bare name.
- **`world_hash` (this crate, `Snapshot.world_hash`) vs. `head_hash` (`Delta.head_hash`):** Computed differently as noted above — confusion-prone.

### Known cognitive traps

- **Trap:** `apply_delta` silently no-ops when `base.tick != delta.base_tick`.
  - **Why it's confusing:** It returns nothing; callers cannot detect the mismatch directly. The next computed `world_hash` will diverge but there's no immediate signal.
  - **What's actually true:** `lib.rs:404-406`: `if base.tick != delta.base_tick { return; }`. The expected recovery path is the next `ServerSnapshot` (full) — which the server emits once per second (lib.rs:574-576).

- **Trap:** "Authentication" via `ClientHello.token` only logs a warning; it does not reject the connection.
  - **Why it's confusing:** The string `"unauthenticated or unknown token: {}"` (lib.rs:787) sounds like a security failure.
  - **What's actually true:** The token is only printed via `println!`. Bad tokens proceed normally. Tests at `tests/integration/auth_tests.rs` (30 tests) lock in the current observable behavior.

- **Trap:** Five `Msg::Server*` variants are accepted by the server but silently ignored.
  - **Why it's confusing:** `Msg` is a single enum; one might assume all variants are equally valid.
  - **What's actually true:** `lib.rs:885-891` catches `Msg::ServerWelcome | Msg::ServerSnapshot | Msg::ServerDelta | Msg::ServerApplyResult | Msg::ServerAck` and falls through — they are "server-only" and a client sending them is treated as a benign protocol error.

- **Trap:** `ValidateCfg::world_bounds` is hardcoded as `(0, 0, 19, 9)` at two call sites.
  - **Why it's confusing:** Looks like a configurable parameter.
  - **What's actually true:** `lib.rs:826-828` and `lib.rs:857-859` both inline the constant. The test world (`GameServer::new`, lib.rs:512-531) places obstacles up to `(6, 8)`, fitting this bound. Changing the world size in `GameServer::new` requires updating these constants too.

- **Trap:** `let _ = tx.send(...)` on the broadcast channel ignores send failures.
  - **Why it's confusing:** It looks like message delivery is being suppressed.
  - **What's actually true:** A `broadcast::Sender::send` returns `Err` only if there are no receivers. The 1024-slot buffer (lib.rs:520) is per-receiver; lagged receivers drop in their own task (lib.rs:618). So "no receivers" is a legitimate "everyone disconnected" state; `let _ =` is the right shape.

- **Trap:** `replay_from` discards `validate_and_execute`'s `Result`.
  - **Why it's confusing:** Replay seemingly silently swallows command-validation failures.
  - **What's actually true:** `lib.rs:928`: `let _ = validate_and_execute(...)`. Matches the server's tolerance: the live server also accepts invalid plans (validation result is reported as `Msg::ServerApplyResult { ok: false }` and replay-logged anyway — `lib.rs:836-844, 866-874`). Replay determinism is about reaching the same world state given the same inputs in order, not about consistent validation outcomes.

---

## 7. Decision Log

### Decision: JSON wire format (Msg as serde_json `#[serde(tag = "type")]`)
- **Date:** 2025-09-04, commit `ba52548b3` ("Implement GameServer with WebSocket handling") — the same commit that created the GameServer + Msg enum. Empty commit body — no design rationale captured.
- **Status:** Accepted (`lib.rs:447-449`)
- **Context:** Each `Msg` is serialized via `serde_json::to_string` and sent as a WebSocket `Message::Text` frame.
- **Decision:** Use human-readable, debuggable JSON for the wire format.
- **Alternatives considered:** [Reasoning not recovered from available sources]. The companion subsystem `astraweave-net-ecs` chose postcard + lz4 + crc32 for the same job — but there's no commit message explaining whether this crate is "legacy text format" or "intentionally human-readable for testing".
- **Consequences:**
  - Trivially debuggable with `wscat`/browser dev tools.
  - Significantly larger payload than binary formats.
  - JSON-parsing on the server side is the per-frame cost.

### Decision: 60 Hz fixed tick, ~20 Hz delta broadcast, 1 Hz full-snapshot floor
- **Date:** 2025-09-04, commit `ba52548b3` ("Implement GameServer with WebSocket handling") — present in the initial implementation. Empty commit body.
- **Status:** Accepted (`lib.rs:558, 574-580`)
- **Context:** Per the tick loop in `run_ws_on_listener`.
- **Decision:** Tick at 60 Hz (16,666 µs period); broadcast delta-eligible snapshots every 3 ticks; broadcast full snapshots every 60 ticks.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Clients with up-to-the-millisecond reconciliation needs are served at 20 Hz, not 60 Hz.
  - The 1 Hz full-snapshot floor caps how long a packet-loss recovery can take to under one second.

### Decision: Hardcoded test world in `GameServer::new`
- **Date:** 2025-09-04, commit `ba52548b3` (initial GameServer implementation). Empty commit body.
- **Status:** Accepted (`lib.rs:512-531`)
- **Context:** The constructor spawns three named entities (P, C, E) on a 20×10 grid with a wall of obstacles at x=6.
- **Decision:** Bake a test scene into the constructor rather than accepting a `World` parameter.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Production use requires editing the constructor or using a different code path (no parameter-driven `GameServer::with_world(...)` exists).
  - Tests and demos all get the same predictable initial state.

### Decision: `apply_delta` silent-no-op on tick mismatch
- **Date:** 2025-09-04, commit `ba52548b3` (initial implementation). Empty commit body.
- **Status:** Accepted (`lib.rs:404-406`)
- **Context:** Delta apply requires `base.tick == delta.base_tick`.
- **Decision:** On mismatch, return silently without applying or erroring.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Lost deltas do not propagate as errors; the next full snapshot (≤ 1 second away) recovers state.
  - Callers cannot detect "I missed a snapshot" without comparing `world_hash` against an expected value.

### Decision: `ValidateCfg::world_bounds = (0, 0, 19, 9)` hardcoded
- **Date:** 2025-09-04, commit `ba52548b3` (initial implementation; both call sites added together). Empty commit body.
- **Status:** Accepted (`lib.rs:826-828, 857-859`)
- **Context:** Two call sites in `Msg::ClientProposePlan` and `Msg::ClientInput` handlers.
- **Decision:** Hardcode the validation bounds inline at both call sites.
- **Alternatives considered:** [Reasoning not recovered from available sources]
- **Consequences:**
  - Tied to the test world's 20×10 dimensions.
  - Changing world dimensions requires editing both inline copies.

### Decision: `ring` crypto backend for rustls (no default-features)
- **Date:** 2025-12-05, commit `3e51f6521` ("feat: Introduce extensive documentation, new test suites, and core module files...") — the commit that introduced `tls.rs` and added the optional `tls` feature to Cargo.toml. The "ring for Windows compatibility" inline comment landed at the same time.
- **Status:** Accepted (`Cargo.toml:24-25`)
- **Context:** Cargo.toml comment at line 23: `# TLS support (optional) - using ring for Windows compatibility`.
- **Decision:** Disable rustls's default `aws-lc-rs` provider and use `ring` instead.
- **Alternatives considered:** Default `aws-lc-rs` (Cargo.toml comment suggests this was rejected for Windows compatibility).
- **Consequences:**
  - Wider Windows toolchain support.
  - Loses some performance advantages aws-lc-rs has on Linux.
  - Both rustls and tokio-rustls explicit `default-features = false, features = ["ring", "std"]` to ensure consistent provider selection.

### Decision: `#![forbid(unsafe_code)]` at crate root
- **Date:** 2026-02-09, commit `745c100a8` (sweeping commit titled "Mutation-resistant test suites across ~35+ crates …" that added `#![forbid(unsafe_code)]` to `lib.rs:1` as a small line within a much larger workspace-wide change; verified via `git log -L "1,5:astraweave-net/src/lib.rs"`). The attribute was **not** present in the original 2025-09-04 implementation; same pattern as `astraweave-input` per its trace.
- **Status:** Accepted (`lib.rs:1`)
- **Context:** Network code can have UB-sensitive surface (e.g. zero-copy decoders), but this crate stays purely in safe Rust.
- **Decision:** No `unsafe` anywhere in `astraweave-net`.
- **Alternatives considered:** None reasonable for this layer.
- **Consequences:** All zero-copy / FFI work must happen inside tokio, tokio-tungstenite, rustls, or webpki-roots.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `Snapshot.version == SNAPSHOT_VERSION (= 1)` | Yes | `lib.rs:35, 298` — every snapshot built via `build_snapshot` stamps this constant. |
| 2 | `Snapshot.entities` is sorted by `id` (stable order) | Yes | `lib.rs:271`: `ids.sort_unstable()` in `world_to_entities`. |
| 3 | `Snapshot.world_hash` is deterministic for the same `(entities, obstacles)` content | Yes | `lib.rs:228-243` uses `DefaultHasher` over canonically-ordered inputs. Property tests at `tests/property_tests.rs` exercise this. |
| 4 | `Delta.head_hash == subset_hash(head.entities)` after filtering | Yes | `lib.rs:393`. |
| 5 | `apply_delta` is a no-op when `base.tick != delta.base_tick` | Yes | `lib.rs:404-406`; tested via integration `packet_loss_tests.rs`. |
| 6 | Empty `Delta` (no `changed`, no `removed`) results in no send to the client | Yes | `lib.rs:713-715`. |
| 7 | `EntityDeltaMask` bits are `POS=1, HP=2, TEAM=4, AMMO=8` (low 4 bits) | Yes | `lib.rs:60-63`. Mutation tests target these constants. |
| 8 | Full snapshots are broadcast at minimum every 60 ticks (~1 Hz floor) | Yes | `lib.rs:574-576`: `if tick - last_full >= 60`. |
| 9 | Delta-eligible snapshots broadcast every 3 ticks when no full is due | Yes | `lib.rs:577-580`: `else if tick - last_broadcast >= 3`. |
| 10 | `Interest::include(viewer, viewer)` returns true for all four impls (a viewer always sees themselves) | Yes — implicitly | `FullInterest`: always true. `RadiusTeamInterest`: same team check (viewer.team == viewer.team). `FovInterest` / `FovLosInterest`: distance from self is 0 and same-team check applies. Tested at `tests/interest_coverage_tests.rs`. |
| 11 | `replay_from` returns the same `world_hash` for the same `(World, events)` sorted by `(tick, seq)` | Yes | `lib.rs:911-932`; verified by `examples/net_headless_sim` which prints both `baseline` and `final` hashes for a fixed event log. |
| 12 | `validate_and_execute`'s `Result` is reported back over the wire as `Msg::ServerApplyResult` for client input | Yes | `lib.rs:830-831, 847, 861-862, 878`. |
| 13 | `replay_from` does NOT bail on per-event validation failures | Yes (intentional contract) | `lib.rs:928`: `let _ = validate_and_execute(...)`. |
| 14 | `ServerWelcome` is sent immediately after WebSocket upgrade, before any other server message | Yes | `lib.rs:601-604` — synchronous send before the writer-task spawn at `lib.rs:612`. |
| 15 | `GameServer::tick` uses `AtomicU64::fetch_add(1, Ordering::Relaxed)` | Yes | `lib.rs:559`. The relaxed ordering is sufficient because all reads happen on the same task that incremented. |
| 16 | TLS feature is opt-in (default features = `[]`) | Yes (compile-time) | `Cargo.toml:8`: `default = []`; `tls = ["tokio-rustls", "rustls", "rustls-pemfile", "webpki-roots"]` at line 9. |

---

## 9. Performance & Resource Profile

### Hot paths

- **Per-tick world step + snapshot construction** (`lib.rs:559-573`) — runs 60 times/sec on the server. Cost = world.tick(dt) + `world_to_entities` (O(N) scan + sort) + `stable_hash_snapshot` (O(N + M) hashing where M = obstacle count). Benched via `benches/net_bench.rs`.
- **Per-snapshot per-client interest filter + diff** (`lib.rs:621-670` for `ForceSnapshot`, `lib.rs:671-748` for `Snapshot`) — runs once per broadcast event per connected client. Cost dominated by `filter_snapshot_for_viewer` (O(N) per client) plus `diff_snapshots` (O(N) for the BTreeMap construction). Allocates a fresh `Box<dyn Interest>` per snapshot per client (lib.rs:625-656, 676-707) — minor heap churn.
- **JSON encode** (`serde_json::to_string(&Msg)` at multiple sites: `lib.rs:602, 666, 721, 734, 744, 754, 765`) — runs once per outbound frame. Cost scales linearly with serialized payload size.

### Cold paths

- **Connection accept + WebSocket upgrade** (`tokio_tungstenite::accept_async`, `lib.rs:596`) — once per client connect.
- **TLS handshake** (when `tls` feature is wired by an external caller) — once per client connect; rustls handshake is well-known to take milliseconds.
- **`replay_from`** (`lib.rs:911-932`) — batch determinism check, not a per-frame path. Run once on demand.

### Resource ownership

- **`World`** — owned by `GameServer.world: Mutex<World>`. One per `GameServer`. Locked per tick and per client-input handler.
- **`broadcast::Sender<ServerEvent>`** — owned by `GameServer.tx`. 1024-slot buffer (`lib.rs:520`). One per `GameServer`. Cloned implicitly when each connection's writer subscribes.
- **`obstacles` cache** — `Arc<Mutex<BTreeSet<(i32, i32)>>>` (`lib.rs:502`). Refreshed each tick from `world.obstacles`. Used only by the `FovLos` interest path (`lib.rs:646, 697`).
- **`replay` log** — `Mutex<Vec<ReplayEvent>>` (`lib.rs:501`). Append-only on every validated client input. No bound, no rotation, no eviction.
- **Per-connection** — Each `handle_conn` task owns a `viewer_id: Arc<Mutex<u32>>` and a `policy: Arc<Mutex<InterestPolicy>>` that are updated from the reader side and read from the writer side. The writer task pulls `obstacles_ref` from `GameServer.obstacles` (an `Arc`).
- **TLS material** — `TlsServerConfig.server_config: Arc<ServerConfig>` (`tls.rs:44`). Constructed once per server startup; cloned via `Arc::clone` when distributed.

---

## 10. Testing & Validation

- **Unit tests:** 91 total inline (`#[cfg(test)] mod tests` and `mod mutation_tests` in `lib.rs`):
  - `src/tests.rs`: 38 tests.
  - `src/mutation_tests.rs`: 50 tests.
  - `src/tls.rs`: 3 tests (TLS configuration loading).
- **Integration tests:** 286 across `tests/` and `tests/integration/`:
  - `tests/boundary_condition_tests.rs`: 6 tests.
  - `tests/concurrent_stress_tests.rs`: 1 test.
  - `tests/coverage_booster_tests.rs`: 4 tests.
  - `tests/error_message_validation_tests.rs`: 1 test.
  - `tests/interest_coverage_tests.rs`: 19 tests.
  - `tests/mutation_resistant_comprehensive_tests.rs`: 129 tests.
  - `tests/property_tests.rs`: 23 tests (proptest 1.5).
  - `tests/property_tests_extended.rs`: 14 tests; corresponding `.proptest-regressions` file is checked in.
  - `tests/integration/sync_tests.rs`: 25 tests.
  - `tests/integration/snapshot_sync_tests.rs`: 6 tests.
  - `tests/integration/packet_loss_tests.rs`: 25 tests.
  - `tests/integration/server_logic_tests.rs`: 3 tests.
  - `tests/integration/auth_tests.rs`: 30 tests.
- **Total tests:** **377** in this crate.
- **Mutation testing:** Two dedicated suites — `src/mutation_tests.rs` (50 tests, 1087 lines) and `tests/mutation_resistant_comprehensive_tests.rs` (129 tests). The crate is **not** present in `.github/workflows/mutation-testing.yml` as of `a2474c5b7` (verified via workspace grep) — the tests are owned by the crate, not by a centralized mutation-testing run.
- **Fuzz targets:** 4 in `astraweave-net/fuzz/fuzz_targets/`: `fuzz_delta_compression`, `fuzz_interest_management`, `fuzz_packet_parsing`, `fuzz_snapshot_serialization`. Run on demand with `cargo +nightly fuzz run <target>`. **Not** present in any `.github/workflows/*.yml` as of `a2474c5b7`.
- **Property tests:** 37 (proptest 1.5). Checked-in regressions file at `tests/property_tests_extended.proptest-regressions` preserves shrunk failing inputs.
- **Benchmarks:** `benches/net_bench.rs` (criterion 0.5 with `html_reports`). Run via `cargo bench -p astraweave-net`.
- **Sanitizer CI:** `astraweave-net` is explicitly listed in the `P1_CRATES` array at `.github/workflows/sanitizers.yml:205` (alongside `astraweave-asset`, `astraweave-memory`, `astraweave-context`). Verified by direct read 2026-05-12.
- **Dedicated CI:** `.github/workflows/net-tests.yml` runs `cargo test -p astraweave-net --all-features --verbose` on every push to main and every PR (verified at workflow line 17).
- **Miri validation:** `#![forbid(unsafe_code)]` (`lib.rs:1`) leaves no in-crate UB surface. The crate is **not** present in `.github/workflows/miri.yml` as of `a2474c5b7`.
- **Kani validation:** Not present in `.github/workflows/kani.yml`. No `#[kani::proof]` harnesses in the crate.
- **Manual validation:** Three example demos (`coop_server`, `coop_client`, `net_headless_sim`) — only `net_headless_sim` runs headlessly without GUI; the others require interactive wiring.

---

## 11. Open Questions / Parked Decisions

- **Co-existence with `astraweave-net-ecs`.** Two networking subsystems live in the workspace with disjoint data models, wire formats, and integration patterns (see §6 row 1). `astraweave-net` is consumed by `coop_server` / `coop_client` / `net_headless_sim`; `astraweave-net-ecs` is consumed by `astraweave-stress-test` (per workspace Cargo.toml). Are these intended to coexist long-term as parallel options for different game architectures, or is one slated to supersede the other? Andrew's call.

- **`token` field in `Msg::ClientHello` only logs.** `lib.rs:785-789` prints a warning when the token != `"dev"` but does not reject the connection. **Investigation (2026-05-12):** Verified 30 tests in `tests/integration/auth_tests.rs` (via `grep -c '#\[test\]'`). The tests exercise the observable behavior: bad tokens are accepted, only logged. The lock-in is real — changing to reject-on-bad-token would break the test suite. Are these tests locking in intentional "log-only" auth, or is the rejection path a parked TODO? Andrew's call.

- **TLS exposed but not wired inside `GameServer`.** `tls.rs` provides library primitives; `GameServer::run_ws_on_listener` uses plain TCP. Should `GameServer` gain a `run_wss(...)` variant that wraps the listener with `TlsAcceptor`, or is the design intent "TLS is the caller's responsibility"? Andrew's call.

- **Hardcoded `ValidateCfg::world_bounds = (0, 0, 19, 9)` at two sites.** Tied to `GameServer::new`'s baked test world. Should this be plumbed through a configuration parameter, or kept inline pending a larger server-config refactor? Andrew's call.

- **`GameServer::new` bakes a 3-entity test world.** No `GameServer::with_world(world: World) -> Self` exists. Should the API expose a "bring your own world" constructor, or is the constraint that all production servers go through this proof-of-concept code path? Andrew's call.

- **Replay log has no bound or rotation.** `GameServer.replay: Mutex<Vec<ReplayEvent>>` grows monotonically. **Investigation (2026-05-12):** Comprehensive grep confirms exactly **2 push sites** (`lib.rs:837, 868`, in the `ClientProposePlan` and `ClientInput` handlers) and **0 remove/drain/clear/truncate sites** anywhere in `astraweave-net/src/`. The replay log is truly unbounded — once a server runs long enough, the `Vec` will exhaust memory. For long-running servers this is unbounded memory growth. Is rotation a parked feature, or is replay only intended for short-session tests? Andrew's call.

- **Fuzz targets exist but are not in CI.** Four cargo-fuzz targets at `astraweave-net/fuzz/fuzz_targets/` cover delta, interest, packet, and snapshot — but no workflow runs them. Are they intended to be opportunistic local-only checks, or should a nightly fuzz workflow be added?

- **`Box<dyn Interest>` allocated per snapshot per client.** Each broadcast triggers a fresh `Box::new(...)` for each connected client. **Investigation (2026-05-12):** Two allocation sites (the `ForceSnapshot` branch at `lib.rs:625-656` and the `Snapshot` branch at `lib.rs:676-707`). With M connected players and N broadcasts/sec (~20 Hz delta + 1 Hz full = ~21 broadcasts/sec), the workspace allocates `M × 21` boxes/sec at steady state. For M=2 (typical demo) that's ~42 allocations/sec — negligible. For M=64 (production hypothetical) that's ~1,344/sec — still modest but worth noting at higher scales. Is this hot-path allocation acceptable at current player counts, or worth caching the policy → interest object map?

- **`webpki-roots` dep is client-side-only but lives in the unified `tls` feature flag.** **Investigation (2026-05-12):** Confirmed `webpki_roots::TLS_SERVER_ROOTS` is imported at `tls.rs:33` and used at `tls.rs:119` — both inside `TlsClientConfig::default_connector` (a client-side method that builds a `RootCertStore` of trusted CAs). The `TlsServerConfig::from_pem_files` server-side path does NOT touch `webpki-roots`. So the dep is genuinely client-only. The `tls.rs` module exports 5 public methods total: server-side `TlsServerConfig::{from_pem_files, acceptor}` and client-side `TlsClientConfig::{default_connector, with_custom_ca, insecure_connector}`. A consumer that only wants server TLS (e.g., `coop_server` providing a service) still pulls `webpki-roots` because the feature flag is unified. Is this fine because both client and server are co-located in this crate's primitives, or worth splitting client-tls and server-tls feature flags?

---

## 12. Maintenance Notes

**Update this doc when:**
- A new `Msg` wire variant is added (§2 stage 6, §3 vocabulary, §7 first decision).
- A new `Interest` impl is added (§2 stage 2, §3 vocabulary, §8 invariant 10).
- The tick rate or broadcast cadence changes (§2 stage 5, §7 second decision, §8 invariants 8-9).
- The `Cargo.toml` deps change (workspace versions, `tls` feature contents).
- A consumer outside the existing three examples adopts the crate (§4 downstream table).
- The hardcoded test world or `ValidateCfg` bounds are parameterized (§6 trap 4, §7 fifth decision, §11 third-from-bottom question).
- TLS is wired into `GameServer` directly (§6 row 4, §11 third question).

**Verification process:**
- `rg 'pub fn|pub struct|pub enum|pub trait' astraweave-net/src/` should match §3 vocabulary surface.
- `cargo tree -p astraweave-net --depth 1` should list `anyhow`, `thiserror`, `serde`, `serde_json`, `tokio`, `futures-util`, `tungstenite`, `tokio-tungstenite`, `astraweave-core`, `rand`. Adding `tls` feature adds `tokio-rustls`, `rustls`, `rustls-pemfile`, `webpki-roots`.
- `rg 'use astraweave_net' --type rust -g '!*test*' -g '!benches/*'` should find the three example consumers in §4. New consumers must be added there.
- `grep -c '#\[test\]\|#\[tokio::test\]\|proptest!' astraweave-net/src/*.rs astraweave-net/tests/*.rs astraweave-net/tests/integration/*.rs` should total ≥ 377 (the test-count invariant should grow, never shrink).
- Stamp the new commit hash and date in the metadata table after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. The data model is **2D grid-based** (`IVec2`, tile coordinates, `i32`). Do not confuse it with `astraweave-net-ecs`'s `Vec3`-based ECS model.
2. The wire format is **JSON over WebSocket text frames** with `#[serde(tag = "type")]`. The companion subsystem uses postcard + lz4 — they don't interoperate.
3. **`apply_delta` is a silent no-op on tick mismatch.** Don't assume a delta will report dropped state — it won't.
4. **`ValidateCfg::world_bounds` is hardcoded `(0, 0, 19, 9)` at two call sites.** Changing the world dimensions in `GameServer::new` requires editing both.
5. **TLS is exposed as primitives only.** `GameServer::run_ws_on_listener` uses plain TCP. A caller wanting TLS must compose `tls.rs`'s acceptor/connector with `tokio_tungstenite` themselves.
6. **The replay log grows unboundedly.** Long-running servers will accumulate `ReplayEvent`s forever (no rotation).

**Files you'll most likely touch:**
- `astraweave-net/src/lib.rs` — wire-protocol changes, tick-rate changes, interest-policy additions, server-loop tweaks.
- `astraweave-net/src/tls.rs` — TLS configuration changes (rare).
- `astraweave-net/src/error.rs` — new error variants.

**Files you should NOT touch without strong reason:**
- `astraweave-net/src/mutation_tests.rs` — mutation-resistance assertions; changes here can mask real bugs.
- `astraweave-net/tests/mutation_resistant_comprehensive_tests.rs` — same.
- `astraweave-net/tests/property_tests_extended.proptest-regressions` — checked-in shrunk failing inputs; deleting hides a real bug.

**Common mistakes when changing this system:**
- **Adding a new `Msg` variant without updating the "server-only ignore" arm** at `lib.rs:885-891`. New `Server*` variants belong in that ignore list if clients shouldn't send them.
- **Changing `EntityState` fields without updating both `EntityDeltaMask` constants AND `EntityDelta` fields AND `diff_snapshots` AND `apply_delta`.** All four are coupled; the mask bits are tied to specific fields.
- **Adding a third `Interest` impl without updating the `InterestPolicy` enum, both writer-task match blocks, AND the `ClientHello.policy` string-to-enum mapping** at `lib.rs:801-816`.
- **Assuming `world_hash` and `Delta.head_hash` are comparable.** They aren't after filtering — `world_hash` includes obstacles (via `stable_hash_snapshot`), `head_hash` does not (via `subset_hash`).
- **Adding hashing logic with non-canonical iteration order.** `stable_hash_snapshot` carefully iterates pre-sorted entities + a `BTreeSet` of obstacles; any HashMap/HashSet iteration here will break determinism.

---

## Appendix B: Historical context

Original creation: commit `ba52548b3` (2025-09-04, "Implement GameServer with WebSocket handling"). Verified via `git log --diff-filter=A -- astraweave-net/src/lib.rs`. The crate predates `astraweave-net-ecs` (which arrived 2025-10-01) and is the historical "first networking implementation". Its data model (grid `IVec2`, team/HP/ammo) mirrors the early `astraweave-core::World` shape rather than the ECS-component model that emerged later.

The companion `astraweave-net-ecs` subsystem was layered on top of `astraweave-ecs` and uses a separate wire-protocol crate (`net/aw-net-proto`) — see `docs/architecture/net_ecs.md` (when produced).
