---
schema_version: 1
trace_id: causality
title: "Causal world state"
description: "Typed causal events, parent links, and baked consequence rules"
primary_crate: astraweave-causality
domain: gameplay
lifecycle_status: active
integration_status: partial
summary: "Opt-in ring. BridgeAgent drains a collapse notice into the cut. Editor play does not."
owns: [astraweave-causality]
doc_version: "1.0"
last_verified_commit: ea8ec329
---

# Architecture Trace: Causal world state

## Metadata

| Field | Value |
|---|---|
| **System name** | Causal world state |
| **Primary crates** | `astraweave-causality` |
| **Document version** | 1.0 |
| **Last verified against commit** | `ea8ec329` (base) plus the crate landing on this branch |
| **Last verified date** | 2026-10-15 |
| **Status** | Active library. Headless Aravath caller. Editor play does not call it. |
| **Owner notes** | Approved from `docs/current/CAUSAL_WORLD_DESIGN.md`. The Aravath slice waits on deferred-failure #279. |

---

## 1. Executive Summary

**What this system does:**
`CausalWorld::tick` admits recorded inputs, fires a baked rule table, and keeps the resulting events in a fixed ring.

**Why it exists:**
Quests, AI, and persistence need consequences of a world change (a cut span, a blocked route, a shortage) without a live model and without a second copy of `WorldEvent`.

**Where it primarily lives:**
`astraweave-causality/src/rules.rs` (kinds and `bridge_rules`) and `astraweave-causality/src/world.rs` (`tick`, hash, rollback).

**Status note:**
Games opt in by depending on the crate. `aravath_runtime::BridgeAgent` is the production caller. It drains the collapse notice from the temporary destruction copy. The editor does not.

---

## 2. Authoritative Pipeline

```text
Host (not this crate)
    │ push_input / record_line
    ▼
CausalWorld fixed rings
    │ tick
    │   1. admit inputs whose tick matches, sorted by (tick, seq, subject, kind)
    │   2. apply due pending emits
    │   3. cascade new events through the rule table, depth cap 4
    ▼
events()  — the ring is kept, not cleared, so a later rule can see QUEST_OFFER
    │
    ├─ canonical_bytes / replay_hash   (allocates; not the frame path)
    └─ host reads stock, stance, route_class, quest_template, quest_complete
```

`SimRng::new(seed, tick, SIM_SYSTEM_CAUSAL)` runs at the start of every tick and is not drawn. The bridge table's draw count stays 0.

A recorded line is hashed onto `QUEST_OFFER` and `QUEST_COMPLETE`. A missing line is `scripted:` plus the baked sentence. The line does not change stock or stance.

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **Causal input** | A `Copy` record the host admits for one tick | `rules.rs` `CausalInput` |
| **Causal event** | An admitted input or a rule emit, with up to four parent ids | `world.rs` `CausalEvent` |
| **Rule** | A data row: match kind/class/subject/payload, optional required kind, emit, delay, pace, sack delta | `CausalRule` |
| **Pace** | `delay_ticks` and `every_ticks` on the supply rule. The bridge case passes 10 | `bridge_rules` |
| **Bridge span** | Authored milli-unit box. Not the placeholder collapse notice | `BRIDGE_SPAN_MIN` / `BRIDGE_SPAN_MAX` |

Do not confuse this ring with `astraweave-coordination::WorldEvent` (strings, wall clock, dormant) or the editor world-panel `WorldEvent` (presentation).

---

## 4. Cross-System Touchpoints

### Upstream

| Source | Interface | Data |
|---|---|---|
| Host | `push_input` | `GEOMETRY_CUT`, `ACTOR_CELL` |
| Host, off tick | `record_line` | Optional display bytes for a template |

This crate does not call destruction, nav, weaving, quests, or an LLM client.

### Downstream

| Consumer | Interface | Data |
|---|---|---|
| Replay job | `replay_hash` written to `replay-hash-causal.json` | Case `causal_bridge` |
| `aravath_runtime::BridgeAgent` | `push_input`, then `stock`, `stance`, `quest_template`, `quest_complete` | Collapse notice converted to milli-units |

---

## 5. Active File Map

| File | Role | Status |
|---|---|---|
| `src/lib.rs` | Crate root. `forbid(unsafe_code)` | Active |
| `src/rules.rs` | Kinds, classes, `bridge_rules(pace, delta)` | Active |
| `src/world.rs` | Rings, `tick`, rollback, canonical bytes | Active |
| `tests/tick_alloc.rs` | Warmed ticks allocate 0 times, including the shortage and the completion | Active |
| `tests/replay_hash.rs` | Pins `causal_bridge` and writes the JSON the workflow merges | Active |

---

## 6. Conflict Map

| Conflict | Resolution |
|---|---|
| `WorldEventGenerator` already generates events | Leave it off the tick. It uses the wall clock and a live client. |
| `WeaveConsequence` already names an after-effect | Leave the `f32` struct. This ring does not grow another `CollapseBridge` arm. |
| `replay_system` already has `ReplayEvent` | It does not apply events. This crate hashes its own bytes. WS-SAVE may copy them later. |
| Destruction `GeometryNotice` reason 2 was a unit box | #279 fixed it on #243 at `7a571045`. This branch carries that fix in the temporary destruction copy. `causal_bridge` still uses the authored span. `BridgeAgent` uses the drained notice. |
| Two `NavChangeKind` copies (#251 and #263) | This crate does not pathfind. The host, later, submits `ACTOR_CELL` and `GEOMETRY_CUT`. |

---

## 7. Decision Log

| Decision | Reason |
|---|---|
| Separate crate, opt-in | Review answer 1. Core stays the sim-id owner (`SIM_SYSTEM_CAUSAL`) and does not own the ring. |
| No slice until #279 | Review answer 2. Placeholder bounds must not become a trade-route cut. |
| Pace is rule data | Review answer 3. `bridge_rules(10, -1)` is the case. A test uses `(5, -2)`. |
| Completion is rule 7 | Review answer 4. `ACTOR_CELL` at the ford emits `QUEST_COMPLETE` only when `QUEST_OFFER` is already stored. |
| Branch base is #251, not #263 | #263 has no `SimRng` and no six-case replay job. The slice that needs #263 waits on #279. |

---

## 8. Invariants

1. `tick` after `new` does not allocate. Proven by `tests/tick_alloc.rs` through the shortage tick and the completion tick.
2. A failed tick restores stock, the event length, pending occupancy, and the tick index.
3. Inputs that share `(tick, seq)` admit in `(subject, kind)` order.
4. A new event's cause id names an event that is already stored and strictly older in `(tick, seq, id)`.
5. Cascade depth is 4. A self-matching immediate rule returns `CascadeOverflow` and rolls back.
6. The bridge case draws `SIM_SYSTEM_CAUSAL` zero times.
7. Display text is not a stock input. `record_line` changes the hashed line only.

---

## 11. Open Questions

1. Drop the temporary `astraweave-destruction` copy, including the `7a571045` bounds fix, when #243 lands. Owner: the destruction merge.
2. WS-SAVE still does not apply `ReplayEvent`. Copying `canonical_bytes` into that log is a later change and does not edit `aw-save`.
