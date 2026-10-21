---
schema_version: 1
trace_id: destruction
title: "Deterministic Destruction"
description: "Command log, support graph, fracture, terrain deltas, and debris. Sim state is Fixed64."
primary_crate: astraweave-destruction
domain: physics-world
lifecycle_status: active
integration_status: wired
summary: "Greybox digs with a sphere Subtract brush on ColumnFns. FlatColumnBase keeps the destruction replay hash."
owns: [astraweave-destruction]
doc_version: "1.7"
last_verified_commit: c34acf1e
runtime_edges:
  - to: astraweave-core
    note: "The core replay test calls demolition_bytes."
  - to: astraweave-math
    note: "Sim scalars are Fixed64."
---

# Architecture Trace: Deterministic Destruction

## Metadata

| Field | Value |
|---|---|
| **System name** | Deterministic destruction |
| **Primary crates** | `astraweave-destruction` |
| **Document version** | 1.7 |
| **Last verified against commit** | `c34acf1e` |
| **Last verified date** | 2026-10-21 |
| **Status** | Active. Headless showcase and the required replay case. The Aravath greybox binds integer columns through `ColumnFns` and digs with a sphere `Subtract` brush. `Greybox::step` calls `BridgeAgent::step` after the motor. `apply_brush_now` loads every chunk in `(origin ± radius) / CHUNK_EXTENT` before the tick. A checkpoint stores `persist_bytes` and `restore_dug` replays that log before the next dig. `FlatColumnBase` remains the terrain when no functions are bound, so the `destruction` replay hash is unchanged. The greybox golden is `b6a9d5a2c5da28a6` (#330). The earlier #329 value was `0a576578512f25d2`. |
| **Owner notes** | Design: `docs/current/DESTRUCTION_DESIGN.md`. Survey: `docs/current/DESTRUCTION_ARCHAEOLOGY.md`. `Fixed64` lives in `astraweave-math` (PR #245). |

---

## 1. Executive Summary

**What this system does:**
Applies an ordered command log inside a 60 Hz tick. The log breaks a support graph, edits integer terrain, and steps a fixed debris pool. The per-tick SHA-256 is the replay hash.

**Why it exists:**
The physics crate's float `DestructionManager` is deleted. Rapier is not in the cross-OS hash. `PhysicsWorld` does not write `DestructionCommand`.

**Where it primarily lives:**
`astraweave-destruction`. `astraweave-core` holds `SIM_SYSTEM_DESTRUCTION` and the replay test. `examples/aravath_destruction` prints the showcase lines.

---

## 2. Authoritative Pipeline

Each `DestructionWorld::tick`:

1. Take pending commands whose `tick` equals the world tick.
2. Sort by `(tick, seq, entity_id)` and apply them. Rejected commands never enter the log.
3. If the graph is dirty, snapshot which nodes are already `DYNAMIC`, run Tier B (`STRESS_ITERS = 8`) then Tier A, and publish one reason-2 `GeometryNotice` whose box is the union of the pieces that became `DYNAMIC` in that solve. Bounds are `floor(pos - half)` and `ceil(pos + half)` in whole units. Fracture and debris spawn run after the notice, so the box is the collapsed piece, not its children.
4. Step the debris pool against the integer terrain.
5. On the showcase, tick 60 unloads and reloads chunk `(0, 0)` before the caller hashes.

Floats do not enter this pipeline. Rapier is not called.

---

## 3. Semantic Vocabulary

| Term | Meaning |
|---|---|
| `DestructionCommand` | One impact, explosion, brush, detach, or anchor |
| Tier A | Flood-fill from anchors. Islands with no anchor and without `ANTI_DESTROY` become `DYNAMIC` |
| Tier B | Fixed 8-iteration load push. A bond breaks when pushed load exceeds strength minus damage |
| `FlatColumnBase` | Every column has the same integer top. Used when `ColumnFns` is unset |
| `ColumnFns` | Caller-supplied `surface` top and material. This crate does not depend on `astraweave-terrain` |
| Cosmetic debris | Not represented. Slots are gameplay debris and are hashed |
| `GeometryNotice` | Integer box navigation can consume. Reason 2 is a structural collapse |

---

## 4. Cross-System Touchpoints

### Upstream

- `astraweave_math::Fixed64`
- `astraweave_core::SimRng` via `destruction_rng`
- `SIM_SYSTEM_DESTRUCTION = 0xD357_7001`

### Downstream

- `astraweave-core/tests/replay_hash.rs` hashes `demolition_bytes`
- `examples/aravath_destruction` prints `aravath_lines`
- `examples/physics_demo3d` key M enqueues an `Impact`. Health is the bond strength. The break impulse is that impact. The window draws the planar children as boxes (#411)

### Not touched

- `astraweave-physics` `break_destructible` (Rapier body removal only), `add_destructible_box`. The float `DestructionManager` is removed.
- `astraweave-terrain` column generator (the greybox passes function pointers; this crate does not import that crate)
- `astraweave-render` passes (PR #216)
- `astraweave-combat` (PR #212)

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-destruction/src/lib.rs` | `DestructionWorld`, showcase, hash, `demolition_bytes` | Active | `#![forbid(unsafe_code)]` |
| `astraweave-destruction/src/budget.rs` | Named caps, including `STRESS_ITERS` | Active | |
| `astraweave-destruction/src/command.rs` | Command types, validation, canonical bytes | Active | Serde on the command types |
| `astraweave-destruction/src/graph.rs` | Nodes, bonds, Tier A, Tier B, climb flags | Active | `Vec` sorted by id. No `HashMap` |
| `astraweave-destruction/src/fracture.rs` | Planar cut, seeded Voronoi, `bake_gate` | Active | Bake bytes are the manifest input |
| `astraweave-destruction/src/column_base.rs` | `ColumnFns` function pointers | Active | `top` and `material`. No terrain-crate dependency |
| `astraweave-destruction/src/terrain.rs` | Fixed-capacity brush slab, unload/reload | Active | `i32` height, `i8` density. No map alloc on apply. `surface_y` adds the brush delta to `ColumnFns::top` when bound, otherwise to `FlatColumnBase` |
| `astraweave-destruction/src/octree.rs` | Sparse voxel DAG for an edited chunk | Active | Shared leaves and branches. Base column fills unedited cells |
| `astraweave-destruction/src/jobs.rs` | Two query workers, ordered slot merge | Active | Started in `QueryPool::new`. GPU is not a sim writer |
| `astraweave-destruction/src/debris.rs` | 256-slot pool, sleep, eviction | Active | Array, no steady-state alloc |
| `astraweave-destruction/src/rng.rs` | `destruction_rng` | Active | One `SimRng::new` per call |
| `astraweave-destruction/benches/destruction.rs` | graph, fracture, brush, debris | Active | Not a pull-request check |
| `examples/aravath_destruction` | Headless showcase binary | Active | Player lines say Aravath |
| `examples/physics_demo3d` | Rapier window plus one `DestructionWorld` | Active | Key M applies the stored impulse. Pieces are dynamic boxes |
| `assets/aravath_destruction/CREDITS.md` | License record | Active | Procedural blocks, no third-party bytes |

---

## 6. Conflict Map / Residue

| Abstraction | Status | Disposition |
|---|---|---|
| `astraweave-physics` `DestructionManager` | **Removed** (historical) | Deleted with `destruction.rs`, `benches/destruction.rs`, and `process_destructible_hits`. `break_destructible` only removes the Rapier body. `PhysicsWorld` does not write `DestructionCommand` |
| `PhysicsWorld::add_destructible_box` | Stub | Not extended |
| Greybox `dig_sphere` | Calls `apply_brush_now` | Sphere `Subtract` brush. Every chunk the radius touches is loaded before `tick`. The island mesh is carved by `seed - surface_y`. A checkpoint replays `persist_bytes` on load. A full command log returns before that carve |
| #215 `surface_height` | Bound by the greybox | `column_gen` stays in `astraweave-terrain`. `ColumnFns` is the hook. `FlatColumnBase` stays for the replay case |

---

## 7. Decision Log

### Decision: New crate, one-way hooks
- **Date:** 2026-10-15
- **Status:** Accepted at design review
- **Decision:** `astraweave-destruction` owns the hashed state. Physics does not receive a mirror in this slice because the showcase queries integer AABBs and column heights.
- **Consequences:** Rapier transforms are not in the hash.

### Decision: Fixed64 stays in astraweave-math
- **Date:** 2026-10-15
- **Status:** Accepted
- **Decision:** Destruction depends on `astraweave-math` and not on `astraweave-combat`. The type move is PR #245.
- **Consequences:** `Fixed64` is on `main` via #245. Destruction does not depend on `astraweave-combat`.

### Decision: Eight load-push iterations
- **Date:** 2026-10-15
- **Status:** Accepted
- **Decision:** `STRESS_ITERS` is 8. The Aravath gate matches a 64-iteration solve. A 16-node chain does not, and that partial push is the cap, not a defect.
- **Consequences:** Deeper authored buildings need a later tuning pass. No deferred-failure was opened because the showcase converges.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | Commands apply in `(tick, seq, entity_id)` order | Yes | `command_ord` and `DestructionWorld::tick` |
| 2 | `STRESS_ITERS` is 8 | Yes | `budget.rs` |
| 3 | The Aravath gate's broken-bond set at 8 iterations equals the set at 64 | Yes | `graph::tests::damaged_lintel_breaks_the_same_bonds_at_8_and_64` |
| 4 | Unload then load of an edited chunk restores the occupancy bytes | Yes | `terrain::tests::unload_then_load_matches_the_edited_occupancy` |
| 5 | Debris eviction drops the oldest tick, then the lowest id | Yes | `debris::tests::eviction_drops_the_oldest_lowest_id` |
| 6 | `ANTI_CLIMB` wins over `CLIMBABLE` | Yes | `allows_climb` |
| 7 | Sim sources contain no `f32`, `f64`, `HashMap`, `Instant`, or `thread_rng` | Yes | `tests/source_scan.rs` |
| 8 | `demolition_bytes` is stable and is a required replay case | Yes | `determinism-replay.yml` |
| 9 | Restoring `persist_bytes` matches the live hash | Yes | `command_log_restores_the_gate_and_the_crater` |
| 10 | Parallel occupancy queries match a direct DAG walk | Yes | `parallel_queries_match_the_dag_walk` |
| 11 | A collapse notice covers every piece that became `DYNAMIC` this solve, and no piece that was already dynamic | Yes | `collapse_notice_covers_the_dropped_bridge_span` |
| 12 | With `ColumnFns` set, unload then load restores the brush occupancy, and an undug column reports the seed top | Yes | `terrain::tests::seeded_columns_replay_the_same_occupancy` |

---

## 9. Performance & Resource Profile

Caps, not wall-clock gates: 256 debris, 1024 nodes per tick, 8 stress iterations, 4 remeshes, 4096 brush cells, 30 sleep ticks, 65536 commands. The pool is a 256-element array. Graph solves allocate on dirty ticks only.

---

## 10. Testing & Validation

`cargo test -p astraweave-destruction` and `cargo test -p astraweave-core --test replay_hash`. The workflow runs the replay test on Ubuntu, Windows, and macOS. Benches are criterion and stay off pull-request checks.

---

## 11. Open Questions / Parked Decisions

- **#249 column adapter.** Closed on `5c664f23` for the hook. `ColumnFns::top` is what `surface_y` reads. `ColumnFns::material` is the paired pointer. `rebuild_dag` still fills density overrides only, the same as `TerrainBase::column_material`, which no rebuild calls. The generator, erosion, and scatter in `astraweave-terrain` are not edited here.
- **Greybox brush.** Closed on `5c664f23`. `Ground::dig_sphere` enqueues a sphere `Subtract` brush through `DestructionWorld::apply_brush_now`. The headless `examples/aravath_destruction` binary remains the crate showcase. The visible island hole is the occupancy drop applied to `island::surface_height`, because the streamed voxels are still the island fill.
- **Direct stress solve.** A sparse direct solve can replace the 8-iteration push only if it stays integer and ordered. That would change the hash and needs its own review.
- **Physics mirror.** Not added. Add `mirror_debris_pose` only when a caller needs a Rapier body it cannot get from the integer query.

---

## 12. Maintenance Notes

Update this trace when the command set, the stress iteration count, the pool size, or the replay case changes. Bump `doc_version` and `last_verified_commit`.

## Revision history

| Version | Date | Notes |
|---|---|---|
| 1.0 | 2026-10-15 | Initial trace for the crate, the replay case, and the headless Aravath showcase |
| 1.1 | 2026-10-15 | Sparse voxel DAG, ordered query jobs, command-log persistence, weave and navigation notices |
| 1.2 | 2026-10-15 | Collapse notices publish the union of newly dynamic piece bounds. Issue #279 |
| 1.4 | 2026-10-16 | Merge `84d3888a`. `Greybox::step` calls `BridgeAgent`. Greybox golden is `0a576578512f25d2`. #249, #250, and #261 are still open on the tracker. |
| 1.3 | 2026-10-16 | `ColumnFns` and `apply_brush_now`. Greybox digs with a brush. `FlatColumnBase` stays the unbound terrain. Issues #249 and #250 |
| 1.5 | 2026-10-16 | `apply_brush_now` loads the brush span, not only the centre chunk. Greybox save stores the command log and replays it on restore. The `destruction` replay case and the greybox golden are unchanged. |
| 1.6 | 2026-10-21 | §6: the float `DestructionManager` is removed, not left in place. `break_destructible` only removes the Rapier body. |
| 1.7 | 2026-10-21 | `physics_demo3d` key M enqueues an impact. Health is the bond strength. The pieces are Rapier boxes. #411 |
