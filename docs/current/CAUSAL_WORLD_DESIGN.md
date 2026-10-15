# Causal world-state design

**Status:** approved and implemented. Restacked on `main` at `e894d019` after #251 and #263 merged. Do not merge.

**Decisions (review §10):**

1. The engine is a separate `astraweave-causality` crate. Games opt in by depending on it.
2. Collapse notice bounds must be the real span before any Aravath slice. That fix landed with #243 (`1d3e41be` on `main`). `causal_bridge` still feeds the authored span. `BridgeAgent` drains the notice.
3. One sack per 10 ticks stays the bridge pace. `bridge_rules(pace_ticks, sack_delta)` is the data. The replay case passes `(10, -1)`.
4. Quest completion is rule 7. An actor-cell input for the merchant at the ford cell emits `QUEST_COMPLETE` only when a `QUEST_OFFER` is already in the ring.

**Reads:** `main` at `e894d019`. Open draft #243 still owns the destruction copy. #279's bounds fix is carried here until that PR lands.

**Rule this design obeys.** An LLM may propose text or a rule id. The fixed tick validates that proposal against a baked table and executes only the recorded command. Model bytes never allocate a stock count, a route class, or a quest step.

## 1. Decision

Add one crate, `astraweave-causality`, when implementation is approved. It stores typed events, parent links, and a baked rule table. A host outside the crate drains destruction notices, nav route class, and weave sinks into a fixed input ring. Quests, AI, and persistence read a fixed output ring.

The crate depends on `astraweave-core` for `SimRng` and on `astraweave-math` for nothing in the first slice (the slice is integer milli-units and `u32` ids). It does not depend on destruction, nav, weaving, quests, dialogue, or the LLM stack. Those crates do not depend on it. The host is the only place that sees both sides. That is the same one-way shape `DestructionWorld::drain_geometry_notices` and the weaving design's sink rings already use.

### What this is not

| Option | Why it is not the plan |
|---|---|
| Extend `WorldEventGenerator` | Wall clock, `Uuid`, `HashMap`, tokio locks, `f32` probabilities, and a live `LlmClient`. Zero production callers. Putting it on the tick breaks the determinism gate. |
| Extend `NarrativeCoherenceEngine` | Same LLM stack and the same dormancy. Coherence stays an off-tick checker of recorded text. |
| A third `WorldEvent` struct | Two unrelated types already use that name. See §2. |
| Grow `WeaveConsequence` | `f32` drop multiplier and a weather string. `apply_weave_op` bakes a navmesh and discards it. Another match arm would be another script event. |
| Teach `replay_system` the chain first | It advances `current_tick` and does not apply `ReplayEvent`. WS-SAVE owns that function. The causal log is replayable inside the new crate before that system learns the bytes. |
| Hash the f32 waypoint list | `BridgeAgent` truncates `point.x * 1000.0` to `i32`. That assertion can stay in the demo. It is not the causal hash. |

## 2. What is real, what is stubbed, what is on a tick

Surveyed on `main` at `f1109340` unless a PR ref is named. "Wired" means a non-test caller on the fixed tick or in the Aravath runtime. "Dormant" means the type compiles and tests may call it, and no production tick reads it.

### 2.1 Coordination events (on `main`, dormant)

`astraweave-coordination` has no `use astraweave_coordination` outside itself. The roadmap marks the crate a stub. `docs/architecture/ai_pipeline.md` records the same fact.

| Piece | Real or stub | On a tick? |
|---|---|---|
| `WorldEvent` in `agent.rs` | Real POD-shaped struct: `String` id, `String` type, `serde_json::Value` payload, `DateTime<Utc>` timestamp, `EventSeverity`. | No. `Agent::handle_world_event` has no impl outside the crate's tests. |
| `WorldEventGenerator` | Real async generator. `generate_event` calls the LLM, scores coherence as `f32`, and stamps `Utc::now()`. Default interval is 30_000 ms. `EconomicState.trade_routes_status` is a `HashMap<String, String>` inside that generator's private world. It is not a route graph. Fields are `dead_code`. Constructors in this file are the unit tests. | No. |
| `NarrativeCoherenceEngine` | Real async validator. Same four `Arc` handles (`LlmClient`, `RagPipeline`, conversation history, prompt library). Plot points use `DateTime<Utc>`. | No. |
| `AgentCoordinator` | Real tokio router. `Transform` is a placeholder action. | No. |
| Editor `WorldEvent` in `tools/aw_editor/src/panels/world_panel.rs` | A different struct. `Instant` timestamp and a closed editor enum (terrain generated, weather, time, biome, settings). Presentation history for the world panel. | Editor UI only. Not sim. |

PR #213 adds `SimCoordinator` (`astraweave-coordination/src/sim_tick.rs`) and does **not** call `WorldEventGenerator` or `NarrativeCoherenceEngine`. `SimCoordinator::tick` sorts messages by `(from, to, body)` and applies `RoutingAction` in `(priority, rule id)` order. It still allocates `String` and `Vec` on every tick. `AiDepthState::record_line` is the pattern this design copies for text: a missing line becomes `scripted:` plus the baked prompt, and `SimRng` is seeded and not drawn. `llm_dialogue.rs` stays unloaded (deferred-failure #225 on that PR). The depth tick is not on `main`.

### 2.2 Destruction command log (sources on `main`, replay case still on #243)

`astraweave-destruction` is a workspace member on `main` because #212 squash-merged a copy of the #243 tree. The crate manifest on `main` says the copy is temporary and should be dropped when #243 lands. `git diff origin/main origin/pr-243 -- astraweave-destruction` is that comment plus nothing else in the Rust sources.

What the copy actually does:

| Piece | Real or stub | On a tick? |
|---|---|---|
| `DestructionCommand` | Real. `Copy` command: `tick`, `seq`, `entity_id`, and `CommandKind` (`Impact`, `Explosion`, `Brush`, `Detach`, `Anchor`, `SpawnNode`, `SpawnBond`, `Weave`). Ordered by `(tick, seq, entity_id)`. `validate` rejects a zero-extent brush, explosion, or weave. | `DestructionWorld::tick` applies due commands. No Aravath runtime caller on `main`. |
| Command codec | Real little-endian `push_command`. Log magic `AWDESTLOG`. `persist_bytes` / `restore` rebuild the world by replaying the log. | Save path inside the crate. Not called from `astraweave-persistence-ecs`. |
| `GeometryNotice` | Real. Integer AABB plus `reason`: 1 brush, 2 collapse, 3 weave. Cap 32. Overflow drops the oldest. | Produced inside `tick`. |
| `drain_geometry_notices` | Real. Copies the ring into a caller slice and clears it. | The only in-tree caller on `main` is the crate's own test. |
| Collapse bounds | Defect. After `graph.solve`, a new broken bond pushes notice `(0,0,0)-(1,1,1)` with reason 2. `Detach` itself does not push a notice. Brush and weave push a radius box in the command's integer space, which is not documented as milli-units. | Do not treat the unit box as a bridge span. |
| `enqueue_weave` | Real. Records density delta and bond decay in the log, then pushes reason 3. | API exists. No weaving caller on `main`. |
| Replay case `destruction` | Specified on #243 (`determinism-replay.yml` required set, local digest cited in that PR body). | Not in the required set on `main`. |
| `examples/aravath_destruction` | Named in the #243 body. | Not on `main`. |
| Old `DestructionManager` / `process_destructible_hits` | Empty or unwired. #243 defers them as #255. | Not this design's substrate. |

### 2.3 Navigation and the bridge (not on `main`)

PR #251 (`feat/ai-orchestration`, base `main`) contains the nav tile rewrite and an HTN tick. The PR title still says "Phase 1, review before code". The branch contains `NavTiles`, `nav_change_from_geometry_notice`, a zero-alloc test, and required case `nav_reroute`. Its workflow required set is `{planner_pure, perception_world, nav_reroute}` and is stale against `main`, which also requires `gameplay_rng`, `fixed64`, and `combat_resolve`.

`GeometryNotice` in that branch is a hand-copied twin of the destruction struct, because #251 does not depend on `astraweave-destruction`. Reason 1/2/3 map to `NavChangeKind::{Brush, Detach, Weave}`. Unknown reasons return `None`.

PR #263 (`cursor/aravath-bridge-reroute-568b`, base #205) has an older `NavChangeKind` with `Explosion`, `Brush`, and `Detach` only. It has no `GeometryNotice` and no weave kind. `aravath_runtime/src/bridge_agent.rs` is real and headless:

- Scene label `"Aravath"`.
- `bridge_and_ford()` tiles.
- Path from `(1, 0, 1)` to `(1, 0, 46)` meters.
- Tick 30 marks a `Detach` box `[-1_000, -1_000, 17_000]` to `[5_000, 1_000, 31_999]` milli-units.
- Before the mark, `route_x_milli < 8_000`. After tick 40, `route_x_milli > 16_000` (the ford).
- The window does not draw this body. The comment on the file says so.

#263 and #251 share no merge-base newer than `a3fe3ecd`. The tile files differ by about 113 lines. There is one nav rewrite, currently forked.

### 2.4 Quests and persistence (on `main`)

| Piece | Real or stub | On a tick? |
|---|---|---|
| `astraweave-quests::Quest` | Real `{ title, steps }`. The editor constructs it. | Authoring only. |
| `QuestLlmSystem` | Real async system. `Instant::now` around generation. Cooldown in milliseconds. Holds `LlmQuestGenerator`. | No non-test caller. Roadmap §2.3. |
| `terrain_quests` | Real tables of feature type to archetype name (`River` includes `"build_bridge"`). | No non-test caller. |
| `CReplayState` / `ReplayEvent` | Real. `ReplayEvent` is `{ tick, event_type: String, data: Vec<u8> }`. | `replay_system` increments `current_tick` and does not read `events`. `auto_save_system` is an empty TODO. |
| `CPersistenceManager::save_game` | Real `aw-save` bundle. Companions and inventory are hardcoded placeholders. Entity blobs from a `HashSet` (roadmap §1.3 and §2.4). | Not the editor play path. The editor saves through `polish.rs`. |
| `astraweave-core` capture replay | Records time, next id, and obstacles. | Not a causal log. |

WS-SAVE in the roadmap already owns "replay that applies events". This design does not take that file.

### 2.5 Weaving (split)

On `main`, `apply_weave_op` in `astraweave-gameplay` is a live function with six hardcoded kinds, including `CollapseBridge`. It spends a budget, pushes a director plan, rebakes a navmesh into a discarded local, and returns `WeaveConsequence { drop_multiplier: f32, faction_disposition: i32, weather_shift: Option<String> }`. `FreezeWater` is documented as presentation-only. This is not a causal graph.

PR #252 adds `WeaveCommand` (fixed array, cap 64), template ids, and `FieldWrite`. The design on that branch says decay past a strength threshold emits the destruction `Weave` command, and that the weaving crate does not fracture a support graph. Phase 2 of that PR is implemented on the branch and is still a draft. It does not edit `aravath_runtime`.

### 2.6 Roadmap fit

`ENGINE_PRODUCTION_ROADMAP.md` has no causal-world stream. The adjacent contracts are:

- WS-D0 / the replay workflow. Required cases on `main`: `planner_pure`, `perception_world`, `gameplay_rng`, `fixed64`, `combat_resolve`.
- WS-D4. A plan log, model output applied only when the bytes are already recorded. Default LLM path stays off.
- WS-AI-2. One deterministic caller for the dormant AI crates, or an honest trace. #213 is that stream.
- WS-SAVE. `replay_system` applies events in tick order. No edits to `persistence/aw-save`.

A future implementation adds a short WS-CAUSE paragraph to the roadmap in the same commit as the crate. This review PR does not edit the roadmap.

### 2.7 Defect carried by the destruction copy

`DestructionWorld` publishes a collapse notice whose box is `(0,0,0)-(1,1,1)`. A trade-route rule that trusted those bounds would cut the wrong place, on every OS, forever. The fix is to publish the broken structure's authored AABB in milli-units, in stable node-id order, with reason 2. That fix belongs on #243 (and the temporary copy on `main` has to pick it up when #243 reconciles).

This agent cannot open a GitHub issue (the forge CLI here is read-only). The implementation PR either includes that bounds fix or opens `deferred-failure` first, with owner "destruction workstream" and the plan in the previous paragraph. Until then the causal slice uses the span #263 already hardcodes, and it ignores notice bounds for reason 2.

## 3. Event schema

All sim fields are `Copy`, integer, and little-endian. Display strings live in a side table keyed by template id. They are not keys in the rule match.

```rust
pub const SIM_SYSTEM_CAUSAL: u64 = 0xCA05_7001;

pub const MAX_CAUSES: usize = 4;
pub const MAX_EVENTS: usize = 256;
pub const MAX_PENDING: usize = 64;
pub const MAX_INPUTS: usize = 32;
pub const MAX_RULES: usize = 32;
pub const MAX_FIRE_PER_TICK: usize = 32;
pub const MAX_CASCADE_DEPTH: usize = 4;

pub struct CausalInput {
    pub tick: u64,
    pub seq: u32,
    pub kind: u16,
    pub class: u16,
    pub subject: u64,
    pub region: [i32; 6], // x0,y0,z0,x1,y1,z1 in milli-units
    pub payload: [i32; 4],
}

pub struct CausalEvent {
    pub id: u32,       // 0 is unused; ids start at 1
    pub tick: u64,
    pub seq: u32,
    pub kind: u16,
    pub class: u16,
    pub subject: u64,
    pub cause_count: u8,
    pub cause_ids: [u32; MAX_CAUSES],
    pub region: [i32; 6],
    pub payload: [i32; 4],
    pub rule_id: u16,  // 0 when the event is an admitted input
}
```

`id` is the next integer in the world, not a UUID. `cause_ids` are parents. A route-blocked event names the geometry-cut event. A quest-offer event names the supply-short event. Four parents is the cap; a fifth parent is a rejected input, not a heap growth. Cycles are rejected when an event is admitted: a cause id must already exist and must be strictly older in `(tick, seq, id)`.

### 3.1 Closed kinds for the first slice

| `kind` | Name | Who emits it | Payload |
|---|---|---|---|
| 1 | `GEOMETRY_CUT` | Host, from a destruction notice the host has already paired with an authored span | `[structure_generation, 0, 0, 0]` |
| 2 | `ROUTE_BLOCKED` | Rule | `[route_id, 0, 0, 0]` |
| 3 | `ROUTE_CLASS` | Host, from the nav result **or** a baked alternate when the rule says the span has one | `[class, 0, 0, 0]` where 1 = bridge, 2 = ford |
| 4 | `SUPPLY_DELTA` | Rule, once per N ticks while class is ford | `[delta_sacks, stock_after, 0, 0]` |
| 5 | `SUPPLY_SHORT` | Rule, when stock hits 0 | `[0, 0, 0, 0]` |
| 6 | `NPC_STANCE` | Rule | `[stance, actor_id_lo, 0, 0]` where 1 = trading, 2 = asking |
| 7 | `QUEST_OFFER` | Rule | `[template_id, 0, 0, 0]` |
| 8 | `ACTOR_CELL` | Host | `[cell_id, 0, 0, 0]` where 2 is the ford cell |
| 9 | `QUEST_COMPLETE` | Rule 7 | `[template_id, 0, 0, 0]` |

Later kinds (heat, faction, weather) append to this table. They do not reuse `String` event types from `WorldEvent`.

`class` separates namespaces: 1 structure, 2 route, 3 actor, 4 quest template.

### 3.2 Rules are data

```rust
pub struct CausalRule {
    pub id: u16,
    pub when_kind: u16,
    pub when_class: u16,
    pub emit_kind: u16,
    pub emit_class: u16,
    pub delay_ticks: u32,
    pub stock_delta: i32,   // 0 unless emit is SUPPLY_DELTA
    pub every_ticks: u32,   // 0 = once
}
```

The slice table, in `(id)` order:

| id | When | Emit | Delay |
|---|---|---|---|
| 1 | `GEOMETRY_CUT` on the bridge structure | `ROUTE_BLOCKED` on route 1 | 0 |
| 2 | `ROUTE_BLOCKED` on route 1 | `ROUTE_CLASS` ford, if the span's baked alternate is the ford | 0 |
| 3 | `ROUTE_CLASS` ford while stock > 0 | `SUPPLY_DELTA` of `sack_delta` every `pace_ticks` | `pace_ticks` |
| 4 | `SUPPLY_DELTA` whose `stock_after` is 0 | `SUPPLY_SHORT` | 0 |
| 5 | `SUPPLY_SHORT` | `NPC_STANCE` asking for the merchant | 0 |
| 6 | `NPC_STANCE` asking | `QUEST_OFFER` template 1 | 0 |
| 7 | `ACTOR_CELL` for the merchant at the ford cell, and a `QUEST_OFFER` is already in the ring | `QUEST_COMPLETE` template 1 | 0 |

The bridge table is `bridge_rules(pace_ticks, sack_delta)`. The replay case uses `pace_ticks = 10` and `sack_delta = -1`. A different pair is a different table, not a different match arm.

Match is equality on kind, class, and subject. The scan is the rule array in id order, then the events admitted this tick in `(seq, id)` order. A rule fires at most once per cause event. Delayed emits sit in a fixed pending array and become due when `tick` matches. Pending overflow returns `CausalError::PendingFull` and does not drop silently.

No rule reads the clock, a hash map, or a float. The first slice draws zero times from `SimRng`. The canonical bytes still record `SIM_SYSTEM_CAUSAL`, the seed, and a draw count of 0, so a later rule that starts drawing cannot hide inside this case.

### 3.3 Text is a recorded attachment

Template 1 display line, baked, player-facing: `The ford is the only road. Carry the sacks across.` The scene name in that demo remains `Aravath`.

A model may submit, off the tick, a `(tick, template_id, line)` record. The tick accepts the line only when an event with that `template_id` already exists for that tick. A missing record resolves to `scripted:` plus the baked line. The line does not change stock, stance, or quest state. Rejected proposals (unknown template, line for a template the rules did not emit, line longer than 256 bytes) never enter the ring.

This is the same boundary as `AiDepthState::record_line`. It does not call `WorldEventGenerator`.

## 4. Determinism and the replay hash

State that enters the hash:

- world seed, tick, next event id, stock, stance, route class
- every event still in the ring, in `(tick, seq, id)` order
- pending emits, in `(due_tick, rule_id, cause_id)` order
- draw count for `SIM_SYSTEM_CAUSAL` (0 in the slice)
- the resolved template line for any `QUEST_OFFER` still in the ring, as UTF-8 length plus bytes

State that stays out:

- wall time, `Instant`, `Uuid`, `HashMap` iteration order
- GPU readback, task completion order, LLM latency
- nav waypoint floats and Rapier poses
- the destruction debris poses (already outside the causal crate; destruction hashes those itself)

Encoding matches the destruction `state_hash` and the depth-tick `canonical_bytes`: explicit little-endian pushes, then SHA-256. No `DefaultHasher`.

### 4.1 Case `causal_bridge`

New required case in `.github/workflows/determinism-replay.yml`. The compare job already runs on Ubuntu, Windows, and macOS and fails a required case on mismatch or absence. This case joins the eight already required on `main` at `e894d019`: `planner_pure`, `perception_world`, `gameplay_rng`, `fixed64`, `combat_resolve`, `destruction`, `gameplay_qa_aravath_slice`, and `nav_reroute`.

The case builds one world, seed `0xCA05_B81D`, and applies ticks 0 through 120 inclusive. At tick 30 the host admits one `GEOMETRY_CUT` for the authored bridge span (milli-units `[-1000, -1000, 17000]` to `[5000, 1000, 31999]`). At tick 120 it admits `ACTOR_CELL` for the merchant at the ford cell. No recorded model line. Expected integers after tick 120:

- route class = ford (2)
- stock = 0 (8 sacks, -1 per 10 ticks across ticks 40, 50, 60, 70, 80, 90, 100, 110)
- stance = asking (2)
- one `QUEST_OFFER` with template 1
- one `QUEST_COMPLETE` with template 1, caused by the cell event and the offer
- offer line = `scripted:The ford is the only road. Carry the sacks across.`
- completion line = `scripted:The sacks are across the ford.`

A second process in the same test hashes the same bytes. The workflow merges `replay-hash-causal.json` into `target/replay-hash.json` beside the combat and gameplay-qa files. Local digest, pinned in `astraweave-causality/tests/replay_hash.rs`: `813492cb11ffb9834dcfbcdd13fc0aaffd7b07c8ac54e1c921bbdb773dc83446`. The three-OS compare is the check that this pin is not Linux-only.

Same-input reorder check, in the crate tests: two `GEOMETRY_CUT` inputs that differ only by arrival order and share `(tick, seq)` are sorted by `(subject, kind)` before admission, and the hash matches. Inputs that differ by `seq` keep that order. That is the cross-OS case's defense against a host that drains notices in scheduler order.

GPU and async work do not write this state. A future integer compute result may be admitted only as a `CausalInput` whose bytes were already checked by their own cross-OS case. Otherwise it stays presentation.

### 4.2 System id

`0xCA05_7001` is free against the constants on `main` (`CRAFT`, `HARVEST`, `ANCHOR_PARTICLES`, `ENEMY_PATROL`, `SPAWNER`, `TERRAIN_SOLVER`, `COMBAT` `0xC0BA_7001`, `DESTRUCTION` `0xD357_7001`) and against the ids claimed on open branches (`HNSW` `0xA1DE_7001` and `LLM_DIALOGUE` `0xD1A1_7001` on #213, `NAV` `0x4E41_7001` and `HTN` `0x4854_7001` on #251). The weaving design proposes `0x57EA_7001` and that constant is not in `sim.rs` on #252 yet. Implementation adds `SIM_SYSTEM_CAUSAL` next to the other constants in `astraweave-core/src/sim.rs` and nowhere else.

## 5. Zero-alloc hot path

`CausalWorld::new` allocates the rings once:

- `[CausalEvent; 256]`
- `[CausalInput; 32]`
- `[PendingEmit; 64]`
- `[CausalRule; 32]`
- a hash scratch `Vec<u8>` with capacity reserved for a full ring (the hash runs in tests and in the replay case, not inside `tick`)

`tick` and `drain_events(&mut [CausalEvent]) -> usize` write slots that already exist. They do not call `format!`, `to_string`, `Vec::push`, or `Box`. A full ring returns `CausalError::RingFull`. A full pending array returns `PendingFull`. Both leave the previous tick's state unchanged.

The warmed-tick test copies `astraweave-destruction/tests/tick_alloc.rs`: a counting global allocator, 80 ticks of warmup, the counter reset, 20 more ticks, assert 0. The warmup includes the tick-30 cut and the supply cascade, so the firing path is what gets measured, not an empty loop.

`persist_bytes` may allocate. It is the save path, not the frame path. `restore` replays admitted inputs through `tick` and then checks the hash.

Nav commit and HTN keep their own zero-alloc tests on #251. This crate does not call them inside `tick`.

## 6. How the host integrates

One tick, in this order, in the host (the Aravath sim step, later the editor play step). The causal crate never calls into the other crates.

1. **Destruction.** `DestructionWorld::tick` applies the command log, including a `Weave` command if weaving enqueued one. The host then `drain_geometry_notices`.
2. **Span match.** The host pairs a notice with an authored structure only when the notice reason is 1 or 3 and the milli-unit box overlaps that structure, or when reason is 2 and the host substitutes the authored span (until the §2.7 fix). The paired span becomes one `CausalInput` of kind `GEOMETRY_CUT`.
3. **Nav.** Existing `nav_change_from_geometry_notice` / `nav_change_from_kind` and `NavTiles::commit_tick` stay the pathfinder. The host reads the resulting route class through the integer test #263 already uses (`route_x_milli < 8_000` is bridge, `> 16_000` is ford) and submits `ROUTE_CLASS` only if no causal rule already emitted one. The slice's rule 2 emits ford from the baked alternate, so the nav result is a check, not a second writer. If the two disagree, the host records a divergence counter in the demo assertion and does not patch the causal event.
4. **Causality.** `CausalWorld::tick` fires the table.
5. **AI.** A planner may read drained `NPC_STANCE` and `QUEST_OFFER` events. HTN on #251 keeps owning breach and cover from `NavChange`. It does not grow a trade simulation. `SimCoordinator` is not the stance store; stance is the `u8` on the event. A bark, if #213 has landed, is `record_line` for that tick and is not consulted when the line is absent.
6. **Quests.** Template 1 is a baked `Quest { title, steps }` with one step, completed when a later input says the merchant subject reached the ford cell. `QuestLlmSystem` and `terrain_quests` stay off this path. The editor quest type is the save shape for that one record, written by the host, not by the LLM generator.
7. **Persistence.** `persist_bytes` is the blob. `replay_system` learns to copy that blob into `ReplayEvent.data` when WS-SAVE is implemented. Until then `CausalWorld::restore` is the replay. No edit to `persistence/aw-save`.

### 6.1 Weaving

A cast stays a `WeaveCommand` on #252. Structural decay stays a `DestructionCommand::Weave` on the destruction log. The causal crate sees the geometry notice after destruction applies it. A weave that does not change a spanned structure emits no `GEOMETRY_CUT` and no supply chain.

`apply_weave_op(... CollapseBridge ...)` is not given a new side effect. The host that still calls it translates `CollapseBridge` into the destruction command the log already has, once, at the edge. One collapse, one log entry, one notice.

### 6.2 What the LLM is allowed to change

Allowed: a recorded line for a template the rules already emitted. Allowed later, still off-tick: a proposed rule row that the baker hashes into the rule table before the session starts. A rule that arrives mid-session is a new session seed, not a live patch.

Forbidden on the tick: `generate_event`, `generate_storyline`, `QuestLlmSystem::update`, `NarrativeCoherenceEngine`, HTTP, `Utc::now`, and any write into stock or stance from parsed model text.

## 7. First vertical slice in the Aravath demo

Headless, in `aravath_runtime::BridgeAgent`. No new mesh, texture, or audio.

The agent owns a `DestructionWorld` and a `CausalWorld`. Tick 30 detaches the deck. The collapse notice is the union of pieces that became dynamic, from the #243 bounds fix now on `main`. That box is converted to milli-units and is both the nav mark and the `GEOMETRY_CUT` region. Tick 120, once the route is on the ford, admits the merchant cell. Rule 7 completes the quest. Player-visible copy is the template line in §3.3 and the scene label `Aravath`.

## 8. Merge order

Do not merge this pull request. Implementation starts only after review, on a different branch.

`main` at `f1109340` already has the combat resolver and a temporary `astraweave-destruction` tree. It does not have nav tiles, `BridgeAgent`, the depth tick, or the weaving command queue.

| Order | Branch | What has to be true before the next row |
|---|---|---|
| 1 | #243 `feat/destruction` | Reconcile with `main`: the sources match today except the "temporary copy" comment. Land the `destruction` replay case and `docs/architecture/destruction.md`. Replace the reason-2 unit box with the structure AABB in milli-units, or open `deferred-failure` (§2.7) before any causal host trusts notice bounds. |
| 2 | #251 `feat/ai-orchestration` | Rebase onto `main`. Put `gameplay_rng`, `fixed64`, and `combat_resolve` back in the required set, and add `nav_reroute`. Keep the hand-copied `GeometryNotice` so nav does not depend on destruction. Add a one-line layout assert against `astraweave_destruction::GeometryNotice` in a test once both crates are on the branch. |
| 3 | #205 greybox, then #263 | Rebase the demo. Fold #251's `Weave` kind and `nav_change_from_geometry_notice` into the greybox nav so there is one `NavChangeKind`. `BridgeAgent` stays the demo owner. |
| 4 | #213 and #252, in either order | Rebase onto `main`. Neither blocks the doc. The bark attachment waits on #213. The weave-to-destruction command waits on #252's existing plan. The causal crate depends on neither. |
| 5 | Causal implementation | New branch stacked on the rebased #263. Adds `astraweave-causality`, `SIM_SYSTEM_CAUSAL`, the `causal_bridge` required case, the `BridgeAgent` stock/quest assertions, and a WS-CAUSE paragraph in the roadmap. Architecture trace for the crate lands in that same change, under `docs/architecture/`, because the trace describes code that does not exist today. |
| 6 | WS-SAVE | After the causal log bytes are frozen, `replay_system` applies them in tick order. Still no edit to `persistence/aw-save`. |

#215 (terrain command log), #253 (local sim), and #244 (traversal) can emit `CausalInput` later. They are not in the slice. #217 is CI burn-down and is untouched.

Shared-file note for the implementation PR: `determinism-replay.yml` and `astraweave-core/src/sim.rs` are contended. The causal PR edits both only to add one case and one constant, and it rebases onto whoever landed last among #243, #251, and #213.

## 9. Acceptance for the implementation PR (not this one)

- `cargo test -p astraweave-causality --lib` covers admission order, parent links, the six rules, ring full, pending full, and cycle rejection.
- The warmed-tick allocator test reports 0.
- `causal_bridge` is required in `determinism-replay.yml` and matches on Ubuntu, Windows, and macOS.
- `rg` shows `CausalWorld::tick` called from `aravath_runtime` and from no test-only path as the only production caller.
- `rg WorldEventGenerator` still shows no caller outside `astraweave-coordination`.
- No new dependency crate. `serde` only on the persist helpers if the destruction log's manual codec is not enough; prefer the manual little-endian codec and skip `serde` on the hot types.
- Clippy `-D warnings` on `astraweave-causality` and `aravath_runtime`.
- Player-facing strings say Aravath. The retired setting name is not introduced.

## 10. Decisions recorded

1. Separate crate `astraweave-causality`. Opt-in. Not a module of `astraweave-core`.
2. Bounds fix is a gate. #243 is on `main`, so `BridgeAgent` drains the landed notice. `causal_bridge` does not read it.
3. Pace is `bridge_rules(10, -1)` for `causal_bridge`. The numbers are arguments.
4. Rule 7 emits `QUEST_COMPLETE` from `ACTOR_CELL` at the ford while `QUEST_OFFER` is in the ring.

#251 and #263 are on `main` at `e894d019`. This branch rebases onto that commit. The required replay set is those eight cases plus `causal_bridge`.
