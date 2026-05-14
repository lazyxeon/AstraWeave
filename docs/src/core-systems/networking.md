<!--
  Networking System page — replaced 2026-05-15 as part of the post-trace-campaign
  reconciliation.
  Source: ARCHITECTURE_MAP.md §7.1 (Documentation Hazards) and `net.md` / `net_ecs.md`
  trace files. The pre-trace version of this page (committed 2025-09-08 by GitHub
  Copilot bot as part of commit 28bc94f21) described a QUIC-based transport with
  fictional Server / ServerConfig / Client / ClientConfig types. None of those
  types exist in the actual codebase; actual networking is WebSocket over TCP
  via tokio-tungstenite. The pre-trace page has been archived in git history.
-->

# Networking Systems

```admonish warning title="Documentation under reconciliation"
This page was rewritten on 2026-05-15 to reflect the engineering reality surfaced by the
architecture trace campaign. A prior version (added in commit 28bc94f21,
2025-09-08, by an automated documentation pass) described a QUIC-based transport and
API surface that **does not exist in the codebase**. If you arrived here from an
external link expecting that API, see the architecture-trace links below.
```

AstraWeave's workspace contains **two networking subsystems** with disjoint data
models, wire formats, and integration patterns. Neither imports the other.

<!-- Source: net.md §1 + net_ecs.md §1 + ARCHITECTURE_MAP.md §8.8 -->

## `astraweave-net` — snapshot-based server

* **Data model:** 2D grid, `IVec2` positions. Maps to the original AI-companion / tactical
  game model.
* **Wire format:** JSON over WebSocket text frames via `tokio-tungstenite`. **Not QUIC.**
  There is no `quinn` dependency in the workspace.
* **Tick:** 60 Hz fixed tick on `GameServer`. Broadcast cadence: full snapshot every 60
  ticks, deltas every 3 ticks.
* **Interest filtering:** four `Interest` impls — `Full`, `RadiusTeam`, `Fov`, `FovLos`.
* **Trace:** [`net.md`](https://github.com/lazyxeon/AstraWeave-AI-Native-Gaming-Engine/blob/main/docs/architecture/net.md) — file map, conflict map, decision log, invariants, open questions.

## `astraweave-net-ecs` + standalone trio (`aw-net-{proto,client,server}`)

* **Data model:** ECS world, `Vec3` positions. Matchmaking-oriented multiplayer.
* **Wire format:** `Codec::PostcardLz4` (standalone server) or `Codec::Bincode` (ECS Plugin
  layer). Carried over WebSocket — `wss://` for the standalone server, `ws://` for the
  ECS Plugin variant.
* **Matchmaking:** room cap 4, `tick_hz = 30` (hardcoded), 32-byte session key,
  8-byte session hint.
* **Status (per trace §1):** the standalone server/client trio is production-style; the
  ECS Plugin layer is **dormant** — `astraweave-stress-test` declares the crate as a
  dependency but never imports it.
* **Trace:** [`net_ecs.md`](https://github.com/lazyxeon/AstraWeave-AI-Native-Gaming-Engine/blob/main/docs/architecture/net_ecs.md).

## Known integration hazards (surfaced by traces)

<!-- Source: ARCHITECTURE_MAP.md §4.3 (silent failures) and net.md §6 / net_ecs.md §6 -->

* **HMAC signature mismatch.** The standalone server runs HMAC-SHA256 input-frame
  verification; the standalone client still computes the legacy 16-byte XOR
  `sign16`. Every signature verification fails for two independent reasons (length
  mismatch + algorithm mismatch). The failure is `warn!` only — the server does not
  kick the client. Tracked as Q17 in `ARCHITECTURE_MAP.md` §14.
* **`apply_delta` silent no-op on tick mismatch** (`astraweave-net/src/lib.rs:404-406`).
* **`EntityState` type collision.** Both crates define a struct named `EntityState`
  with different field shapes (`{ pos: IVec2, hp, team, ammo }` vs.
  `{ position: Vec3, health }`). Qualify imports.
* **Long-term disposition** of the two subsystems is an open question (Q16 in §14
  of the architecture map): coexist long-term, retire one, or refactor toward a
  unified model?

## Where to actually look in the code

| Need | File |
|------|------|
| Snapshot pipeline | `astraweave-net/src/lib.rs` |
| Standalone server entry | `net/aw-net-server/src/main.rs` |
| Standalone client entry | `net/aw-net-client/src/main.rs` |
| ECS Plugin layer | `astraweave-net-ecs/src/lib.rs` |
| Wire-format types | `net/aw-net-proto/src/lib.rs` |

## Further reading

* **Interactive workspace map** — the
  [Networking Coexistence](https://lazyxeon.github.io/AstraWeave-AI-Native-Gaming-Engine/architecture/#story=networking_coexistence)
  story preset highlights both subsystems side by side.
* [`net.md`](https://github.com/lazyxeon/AstraWeave-AI-Native-Gaming-Engine/blob/main/docs/architecture/net.md) — snapshot-server trace.
* [`net_ecs.md`](https://github.com/lazyxeon/AstraWeave-AI-Native-Gaming-Engine/blob/main/docs/architecture/net_ecs.md) — matchmaking-trio trace.
* [`ARCHITECTURE_MAP.md`](https://github.com/lazyxeon/AstraWeave-AI-Native-Gaming-Engine/blob/main/docs/architecture/ARCHITECTURE_MAP.md) §8.8 — data-flow diagrams for both subsystems.
