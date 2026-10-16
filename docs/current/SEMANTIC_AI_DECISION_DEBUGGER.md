# Semantic AI decision debugger

Approved 2026-10-15. Every question in §14 was accepted as proposed. The implementation is restacked on `main` at `e894d019` after PR #251 and PR #263 merged. The required replay set is eight cases: `planner_pure`, `perception_world`, `gameplay_rng`, `fixed64`, `combat_resolve`, `destruction`, `gameplay_qa_aravath_slice`, and `nav_reroute`. `perception_world` is `6e52e721ceb8a96413a7ae0ba1ad3f88adaeb37c56ba520c3a3cb9f6f0bf7ebb` because `4172ed01` stopped `build_snapshot` from inserting a breach POI. Tracing does not change that digest.

The survey below was taken at `main` `f1109340` and on the #251 draft before that rebase. It stays as the map. The code that landed from this approval is `astraweave-ai/src/decision_trace.rs` (feature `decision-trace`, default off), the record sites in the hierarchical tick and the grid planners, and the headless `tools/aw_decision` binary. No editor panel. No new asset.

## Approved answers

1. Feature-off is the zero-cost mode. `decision_record!` compiles to nothing unless `decision-trace` is on. The default replay build leaves the feature off.
2. The determinism proof is the in-process null-sink versus ring-sink plan digest (`decision_trace_hash_matches_when_the_ring_is_off`). Trace bytes are not a required replay case.
3. `tools/aw_decision` is the inspector. The editor panel stays later, after issues #265 and #273 close.
4. The #251 rebase rule stands. This branch is on `main` at `e894d019` and keeps all eight required cases.

`rerun` takes `&mut PlannerView` so the caller-owned nav scratch can be borrowed mutably. `DecisionTrace::new` allocates the agent rings once; `push` does not. The `aravath-bridge` script uses `bridge_and_ford` and a detach at tick 30. `BridgeAgent` is now on `main` in `aravath_runtime`; the inspector still does not call it.

## 1. What a user can select

Pick one agent and one tick. The inspector shows five layers, in this order:

1. **Perceived.** The snapshot fields the planner actually read: own cell, ammo, morale, smoke cooldown, nearest enemy cell and hp, obstacle count, breach cells, lost-cover cells.
2. **Believed.** A fixed belief slot. Empty until the depth tick on PR #213 publishes a `Copy` summary. The slot can also hold the `StatKnowledge` mask from `astraweave-combat` once a caller passes it in. Hidden stats stay masked.
3. **Goal.** A goal id. On the grid planners the id comes from `WorldSnapshot::objective` through a static table. On the spatial planner the id is the compound task (`cross`).
4. **Planner and choice.** Which planner ran, the action it wrote, every alternative it scored, the integer score, and the reject code. Planners in scope are the hierarchical planner and the utility method score from PR #251, plus the grid rule and utility orchestrators already on `main`.
5. **Execution.** Success, or a failure code from `validate_and_execute` (`NoPath`, `LosBlocked`, `Cooldown`, `InvalidAction`, `Resource`).

A counterfactual reruns that recorded tick with one input changed and prints the decision diff. The rerun is a pure function over a copy of the recorded inputs. It does not advance the live world, and it does not draw `SimRng`.

## 2. Archaeological map

Status words match `docs/current/AI_ORCHESTRATION_ARCHAEOLOGY.md` on PR #251.

| Status | Meaning |
|---|---|
| REAL | A non-test caller outside the owning crate exercises it. |
| DORMANT | It compiles, often with tests, and no shipping caller runs it. |
| ABSENT | No type and no algorithm in `*.rs` on the tree named in the row. |

### 2.1 Planners on `main`

The loop that ships is a 2D grid companion planner.

| Path | What it calls | Where |
|---|---|---|
| ECS planning, default features | `RuleOrchestrator` | `astraweave-ai/src/ecs_ai_plugin.rs` `sys_ai_planning` |
| ECS fallback arm | Same orchestrator, with `t: 0.0`, `obstacles: vec![]`, `pois: vec![]`, `objective: None` | same file, the companion loop that builds `WorldSnapshot` inline |
| `examples/hello_companion` default | `RuleOrchestrator`, then `validate_and_execute` | example `main.rs` |
| `hello_companion` GOAP flag | `GoapOrchestrator`, a one-step Manhattan heuristic | `astraweave-ai/src/orchestrator.rs` |
| `hello_companion` utility flag | Local score functions in the example | example `main.rs` |
| `UtilityOrchestrator` | Two candidates (smoke, advance) into a fresh `Vec<(f32, Vec<ActionStep>)>` | `orchestrator.rs`. DORMANT on `main` |
| `dispatch_planner` / `PlannerMode` | Rule, behavior tree, GOAP. No shipping caller | `astraweave-ai/src/core_loop.rs`. DORMANT |
| Behavior graphs | `BehaviorGraph` tick status. Sequence, selector, action, condition, decorator, parallel | `astraweave-behavior/src/lib.rs`. REAL in the example and the editor. This is a behavior tree |
| Symbolic GOAP | `GoapPlanner` A* over interned facts | `astraweave-behavior/src/goap.rs`. REAL only inside the feature-gated slice companion |
| Advanced GOAP tree | 22 files, feature `planner_advanced` | `astraweave-ai/src/goap/`. DORMANT. Issue #258 |
| `AIArbiter` / live model | Feature `llm_orchestrator`, default off. Runtime model string is `phi3:medium` | DORMANT. Issue #257. A live model is outside the replay contract |

`WorldSnapshot` (`astraweave-core/src/schema.rs`) is `t`, `player`, `me`, `enemies`, `pois`, `obstacles: Vec<IVec2>`, `objective`. `PlanIntent` is a `String` plan id plus `Vec<ActionStep>`. `propose_plan` allocates that `String` and that `Vec` on every call. `sys_ai_planning` also allocates a `Box<dyn Orchestrator>`, job `Vec`s, a `BTreeSet`, and `reason.to_string()` into `AiPlanningFailedEvent`.

`validate_and_execute` (`astraweave-core/src/validation.rs`) is the engine gate. It returns `Result<(), EngineError>`. `EngineError` is `InvalidAction(String)`, `Cooldown(String)`, `LosBlocked`, `NoPath`, `Resource(String)`. The function also calls a `log` closure with `format!` strings. Those strings are the existing execution story. They allocate, and they are the wrong place to hang a hot trace.

### 2.2 Hierarchical planner and utility on PR #251

PR #251 (`feat/ai-orchestration`, draft, tip `492e20ca`) is the planner this debugger explains. The survey above is what that branch started from. Phase 2 on that branch added:

| Piece | Where | What the tick stores |
|---|---|---|
| `ActionSlot` | `astraweave-ai/src/htn.rs` | `Empty`, `MoveTo { x, y }`, `UseBreach { portal_id }`, `TakeCover { cover_id }`, `Wait { duration_milli }` |
| `AgentPlan` | same | `[ActionSlot; 8]`, `len`, `band`, `tick`. The id is `(tick, entity_id)` |
| `SpatialTick` | same | 32 `AgentState`s, fixed weave and hazard hooks, `NavTiles` |
| `plan_route` | same | One expansion for the route, then a breach score, a move, and an optional cover score. Budget `HTN_EXPANSIONS_PER_TICK = 64` |
| `best_breach` | same | Score is `-site.x_milli`. A hazard or weave with non-zero `kind` on that cell drops the portal. Tie-break is the lower portal id |
| `best_cover` | same | Manhattan distance. Tie-break is the lower cover id |
| `legacy_utility_plan` | same | Integer smoke score `1000 + hp * 10` versus advance `800 + close bonus`. Equal scores keep smoke. `UtilityOrchestrator::propose_plan` calls this |
| `PlannerMode::Htn` | `core_loop.rs` | Always available. `dispatch_planner` calls `HtnOrchestrator` |
| `sys_spatial_tick` | `ecs_ai_plugin.rs` | Runs only when a `SpatialTick` resource exists. Constructs a fresh `NavScratch` on every call |
| `ObservationBuf` | `astraweave-core/src/perception.rs` on that branch | Fixed arrays: 16 breaches, 16 lost-cover cells, 8 weave hooks, 8 hazard hooks |
| Zero-alloc proof | `astraweave-ai/tests/zero_alloc_tick.rs` | After warmup, `commit_tick` and `tick_agents` record `allocs() == 0` under `CountingAlloc` |

`best_breach` and `best_cover` already walk every candidate and keep a winner. They do not keep the losers. The trace adds stores for the losers into a caller-provided slot. The winner formula stays the one on that branch.

`plan_from_snapshot` and `legacy_utility_plan` still `format!` a plan id and allocate a `Vec`. Those functions stay on the cold grid path. The hot spatial tick writes `AgentPlan` and is the path the ring records.

### 2.3 Perception

On `main`, `build_snapshot` (`astraweave-core/src/perception.rs`) sets cover from a Manhattan check against `los_max` and always pushes one POI `breach_door` at cell `(15, 8)`.

On #251, that constant POI is gone. `try_build_snapshot_observed` copies `ObservationBuf` breaches into POIs tagged `breach` and lost cover into POIs tagged `cover_lost`. Weave and hazard hooks stay on the buffer so `WorldSnapshot`'s field names stay stable. `fill_observation` in `htn.rs` copies nav breaches and lost cover into that buffer with no allocation.

The trace copies the buffer the planner received. On `main` that includes the constant breach cell. After #251 it includes the nav publish. The debugger does not invent a third perception struct.

### 2.4 Memory, PR #213

On `main`, `astraweave-memory` is a full pipeline (records, episodes, consolidation, forgetting, retrieval, persona) and it is DORMANT for the planning tick. Nothing in `astraweave-ai` calls it.

`MemoryDebugComponent` (`astraweave-memory/src/components.rs`) looks like a debugger and is the wrong one to extend. `recent_operations` is a `Vec<MemoryOperation>`. Each operation stores a `String` id, a `chrono` UTC timestamp, and a `String` detail. That is a wall clock and a heap growth on every operation.

PR #213 (`cursor/engine-ws-ai2`, draft, tip `61ebe2fa`) owns the wiring. `astraweave-dialogue/src/depth_tick.rs` on that branch defines `AiDepthState` and `sys_ai_depth`. One step calls `MemoryManager`, the embedding store, RAG consolidation, prompts, context, persona, the NPC `RulePlanner`, and coordination. A missing log line becomes `scripted:` plus the rendered prompt. `SimRng` is seeded with `(world_seed, tick, SIM_SYSTEM_LLM_DIALOGUE + entity)` and is not drawn. The branch's own `TickTrace` is a growing `Vec` of `String`s (`memory_ids`, prompt, dialogue, note). That trace allocates, and it is the depth tick's replay payload, so this debugger does not write into it and does not read it from inside `plan_route`.

Belief in this design is a `Copy` slot the depth tick may fill later:

```rust
pub struct BeliefView {
    pub fact_hash: u64,
    pub strength_milli: i32,
    pub source: u8,
    pub stat_mask: u16,
}
```

`fact_hash == 0` and `source == 0` mean the depth tick has not published a summary. `stat_mask` is the `StatKnowledge` bits (`astraweave-combat/src/resolve.rs` on `main`): hp, armor, resists, guard, bulwark, offense, weapon, attacker mods. The default mask is `StatKnowledge::none()`. The tracer never calls `resolve_hit`.

NPC names inside #213's depth tick are Mara (merchant) and Bren (guard). They are depth-tick fixtures. They are not the Aravath demo subject in §9.

`llm_dialogue.rs` stays unloaded. Issue #225 (`deferred-failure`) already owns the compile break.

### 2.5 Profiling and observability

`astraweave-profiling` is the zero-cost pattern to copy. `span!` (`astraweave-profiling/src/lib.rs`) expands to a Tracy span only under `feature = "profiling"`. With the feature off, the macro emits nothing. `alloc_plot!` and `measured_span!` follow the same gate. Release builds without the feature have no Tracy types and no extra branches.

`message!` is the pattern to leave alone. With the feature on it `format!`s a `String` and sends it to Tracy. A decision reason cannot go through it.

`CountingAlloc` (`astraweave-ecs/src/counting_alloc.rs`) exposes `allocs()`, `bytes_allocated()`, and `reset_allocs()`. The HTN proof on #251 and the allocation-measurement benches already use it. The trace proof uses the same counter.

`astraweave-observability` is a process-wide tracing and metrics plugin (`ObservabilityState`, `LlmTelemetry`, companion action logs). `LlmTrace` and `TelemetryEvent` in `astraweave-ai/src/goap/telemetry.rs` store `String`s, `Vec`s, and `Instant`. That telemetry is feature-gated with the dormant advanced GOAP tree (issue #258). The decision ring does not share those types, and it does not call `SystemTime` or `Instant`.

`capture_state` / `replay_state` (`astraweave-core/src/capture_replay.rs`) write a JSON file of obstacles, `t`, and `next_id`, then call `sim::step`. That is a smoke snapshot. It is not a per-agent decision log, and the counterfactual API does not go through it.

### 2.6 Replay hashes

`.github/workflows/determinism-replay.yml` on `main` (`f1109340`) requires five cases, and the compare job fails if any one is missing or differs across Ubuntu, Windows, and macOS:

| Case | Producer |
|---|---|
| `planner_pure` | `astraweave-core/tests/replay_support/mod.rs` `planner_pure_bytes`. Rule and heuristic GOAP plans |
| `perception_world` | `perception_world_bytes`. `build_snapshot` order |
| `gameplay_rng` | `gameplay_rng_bytes`. Seeded craft and harvest |
| `fixed64` | `astraweave_math::fixed64_contract_bytes` |
| `combat_resolve` | `astraweave-combat/tests/replay_hash.rs`, run in the same workflow |

Bytes are little-endian integers, length-prefixed UTF-8, and `f32::to_bits`. The encoder does not sort. `SimRng` on `main` is `(world_seed, tick, system_id)` into ChaCha12 via `mix_sim_seed`. Ids already assigned:

| Constant | Value |
|---|---|
| `SIM_SYSTEM_CRAFT` | `0xC2AF_7001` |
| `SIM_SYSTEM_HARVEST` | `0xC2AF_7002` |
| `SIM_SYSTEM_ANCHOR_PARTICLES` | `0xAE11_7001` |
| `SIM_SYSTEM_ENEMY_PATROL` | `0xAE11_7002` |
| `SIM_SYSTEM_SPAWNER` | `0xAE11_7003` |
| `SIM_SYSTEM_TERRAIN_SOLVER` | `0x7E22_7001` |
| `SIM_SYSTEM_COMBAT` | `0xC0BA_7001` |
| `SIM_SYSTEM_DESTRUCTION` | `0xD357_7001` |

#251 reserves `SIM_SYSTEM_NAV = 0x4E41_7001` and `SIM_SYSTEM_HTN = 0x4854_7001` and draws neither. #213 reserves `SIM_SYSTEM_HNSW` and `SIM_SYSTEM_LLM_DIALOGUE`. This debugger reserves nothing and draws nothing. A trace that needed a random tie-break would already have violated the planner's lower-id rule.

#251's copy of `determinism-replay.yml` replaces the required set with `planner_pure`, `perception_world`, and `nav_reroute`. That drops `gameplay_rng` and `fixed64`, and it predates `combat_resolve`. Stacking the debugger on that workflow file would ship the regression. §10 says the implementation refuses that base.

The hash this design adds is a local equality test (trace on versus trace off). It is not a sixth required case. Trace bytes are absent from `replay-hash.json`.

### 2.7 Editor

`tools/aw_editor` has no AI decision panel. Debug panels today are `Console`, `Profiler`, `Performance`, and `SceneStats` (`tools/aw_editor/src/panel_type.rs`). `PanelType` is `#[non_exhaustive]`. Adding a variant touches every match in that file: `title`, `icon`, `is_closable`, `has_scroll`, `category`, `description`, `shortcut_hint`, `is_debug_panel`, and `all`. The Debug menu already iterates `PanelType::all()` (`tools/aw_editor/src/ui/menu_bar.rs`). `default_left_panels`, `default_right_panels`, `default_bottom_panels`, and `default_center_panel` stay as they are so dock tests keep their current tabs. The new panel opens from the menu and stays out of the default layout.

Editor CI on `main` is partly red, and both failures already have owners:

| Issue | Failure | Owner and plan |
|---|---|---|
| #273 `deferred-failure` | `editor_engine_render_parity` hashes diverge on llvmpipe. Engine and editor LDR SHA-256 differ; overlay isolation holds. Mean SAD 1.538 over 262144 pixels | Editor render parity. Find the setter that still differs for the 512×512 grassland fixture. Do not widen the assert |
| #265 `deferred-failure` | Ubuntu and macOS editor jobs fail in `libudev-sys` because `libudev.pc` is missing | CI cleanup, PR #217. Install `pkg-config`, `libudev-dev`, and `libasound2-dev` |

`editor-ci.yml` and `aw-editor-tests.yml` on `main` still do not install `libudev-dev`. Both workflows set `continue-on-error: true` and do not run on pull requests. A decision panel in `aw_editor` would build in those jobs. This design leaves `tools/aw_editor` untouched until #265 and #273 are closed.

### 2.8 Aravath agents that already exist

| Agent | Tree | What it decides |
|---|---|---|
| `run_aravath_slice` | `main`, `astraweave-weaving/src/aravath.rs` | One combat tick: sword, a weave school, a cover bond. No planner, no perception buffer |
| Greybox `Enemy` | PR #205 `aravath_runtime/src/combat.rs` | Melee figurine. Aggro, windup, cooldown. Health is a flat `f32`. No `PlanIntent` |
| `BridgeAgent` | PR #263 `aravath_runtime/src/bridge_agent.rs`, stacked on #205 | Headless companion. `SCENE_LABEL` is `"Aravath"`. At tick 30 it marks a collapse and the route leaves the bridge for the ford. It stores `route_x_milli` only. It does not yet call `plan_route` |

The demo subject is `BridgeAgent`, entity id 1, after #251's `plan_route` is on the same branch. The greybox melee figurine and `run_aravath_slice` stay on their own owners. No new asset. The bridge and ford mesh is the procedural `bridge_and_ford()` already on #251. CC0 packs already on #205 stay there.

`GeometryNotice` is on `main` (`astraweave-destruction/src/lib.rs`): integer AABB plus `reason` (1 brush, 2 collapse, 3 weave), drained from a fixed array of 32. #251 maps that notice into a nav change and does not call `drain_geometry_notices` yet, because that branch does not contain the destruction crate. `main` now has the crate. The debugger still does not drain notices. A counterfactual seal is a patch on a copy of the recorded cell.

## 3. Decision

Add one module, `astraweave-ai/src/decision_trace.rs`, behind `feature = "decision-trace"`, default off. The feature is enabled by the headless inspector and by the proof tests. No other crate enables it. `astraweave-core`'s replay test keeps default features, so the five required hashes stay on the untraced build.

The ring is a separate ECS resource, `DecisionTrace`. It is not a field of `World`, `WorldSnapshot`, `AgentPlan`, `SpatialTick`, or `AiDepthState`. The planning system records only when the caller has inserted the resource. Shipping builds omit the feature, so the record macro expands to zero tokens.

Rejected:

| Option | Why it loses |
|---|---|
| Extend `MemoryDebugComponent` | `Vec`, `String`, and `chrono::Utc`. Wall clock in a sim debug path |
| Extend `goap::TelemetryCollector` | `String`, `VecDeque`, `Instant`, and it lives in the dormant advanced GOAP tree |
| Send reasons through `message!` or `tracing` | Allocates when the sink is on, and a global subscriber can change timing |
| Put the ring inside `SpatialTick` or `World` | A new field changes the bytes of a sim struct the moment the feature is on |
| Hash the ring in `determinism-replay.yml` | The trace would become part of the sim contract. Turning it on would change the required digest |
| Rerun by mutating the live world and calling `step` | That is a second simulation. The counterfactual is one pure planner call |
| Build the editor panel in the first implementation PR | #265 and #273 are open, and the panel has no data until the CLI dump exists |

## 4. Trace schema

All record types are `Copy`, `#[repr(C)]`, and integer-only. Scores are `i32`. The grid utility path on `main` stores `(score * 1000.0) as i32` inside the recorder, after `propose_plan` has returned. The planner's `PlanIntent` is unchanged. The spatial path stores the integer score it already computed (`-x_milli`, Manhattan, or the smoke/advance pair).

```rust
pub const ALT_CAP: usize = 4;

#[repr(u8)]
pub enum PlannerId {
    None = 0,
    Rule = 1,
    BehaviorTree = 2,
    GoapHeuristic = 3,
    Utility = 4,
    Htn = 5,
}

#[repr(u16)]
pub enum GoalId {
    None = 0,
    Cross = 1,
    Engage = 2,
    Hold = 3,
    Extract = 4,
}

#[repr(u16)]
pub enum ActionCode {
    Empty = 0,
    MoveTo = 1,
    UseBreach = 2,
    TakeCover = 3,
    Wait = 4,
    ThrowSmoke = 5,
    CoverFire = 6,
}

#[repr(u16)]
pub enum RejectCode {
    Selected = 0,
    LowerScore = 1,
    Cooldown = 2,
    SealedCell = 3,
    OutOfBand = 4,
    BudgetExhausted = 5,
    NoPath = 6,
    TieLostToLowerId = 7,
}

#[repr(u16)]
pub enum FailCode {
    Ok = 0,
    InvalidAction = 1,
    Cooldown = 2,
    LosBlocked = 3,
    NoPath = 4,
    Resource = 5,
    Pending = 6,
}

#[repr(C)]
pub struct AltScore {
    pub action: u16,
    pub reject: u16,
    pub score: i32,
    pub a: i32,
    pub b: i32,
}

#[repr(C)]
pub struct PerceivedView {
    pub self_x: i32,
    pub self_y: i32,
    pub ammo: i32,
    pub morale_milli: i32,
    pub smoke_cd_milli: i32,
    pub enemy_count: u8,
    pub nearest_x: i32,
    pub nearest_y: i32,
    pub nearest_hp: i32,
    pub obstacle_count: u16,
    pub breach_len: u8,
    pub breach_portal: u32,
    pub breach_x: i32,
    pub breach_y: i32,
    pub lost_cover_len: u8,
    pub lost_cover_id: u32,
}

#[repr(C)]
pub struct BeliefView {
    pub fact_hash: u64,
    pub strength_milli: i32,
    pub source: u8,
    pub stat_mask: u16,
}

#[repr(C)]
pub struct DecisionRecord {
    pub tick: u64,
    pub entity_id: u64,
    pub planner: u8,
    pub band: u8,
    pub goal: u16,
    pub selected: u16,
    pub selected_score: i32,
    pub param0: i32,
    pub param1: i32,
    pub outcome: u8,
    pub fail: u16,
    pub alt_len: u8,
    pub alts: [AltScore; ALT_CAP],
    pub perceived: PerceivedView,
    pub belief: BeliefView,
}
```

`param0` and `param1` are the action payload: move cell, portal id, cover id, or wait duration. Alternatives that do not fit in four slots keep the four the scorer already compared (the running winner and the last three it rejected). `alt_len` is the count stored. A fifth candidate increments nothing in the sim; the scorer's winner is unchanged.

String tables live in the inspector, as `&'static str`, and nowhere in the ring. Player-facing lines use "Aravath". The tables have no product name from the old setting.

`FailCode::Pending` is what planning writes. The caller that already has the `Result<(), EngineError>` stores `Ok` or the matching variant into the same slot. The `String` payload on `InvalidAction`, `Cooldown`, and `Resource` is dropped. The inspector prints the code. This write is a store into the ring. `validate_and_execute` keeps its signature.

Goal strings map through a static table at session start (`"extract"` → `GoalId::Extract`). An unknown objective stores `GoalId::None`. The hot tick does not intern.

## 5. Ring buffer

```rust
pub const RECORD_CAP: usize = 128;
pub const AGENT_CAP: usize = 32;

pub struct AgentRing {
    pub entity_id: u64,
    pub records: [DecisionRecord; RECORD_CAP],
    pub head: u16,
    pub count: u16,
}

pub struct DecisionTrace {
    pub agents: [AgentRing; AGENT_CAP],
    pub len: u16,
}
```

`RECORD_CAP` is a power of two. The write index is `head & 127`. When `count` saturates at 128, the oldest record is overwritten and `head` advances. `entity_id == 0` means the slot is free. Registration is a linear scan of 32 slots, done when the session binds an agent, not inside the per-candidate loop.

`DecisionRecord` is about 160 bytes. Thirty-two agents times 128 records is about 640 KiB, paid only when the feature is compiled in and the caller builds a `DecisionTrace`. The struct is plain data in the resource. `DecisionTrace::new` fills it once, before the tick loop. `record` writes one slot and returns. It does not call `Vec`, `String`, `Box`, `format`, `SystemTime`, or `Instant`.

Lookup for "agent at tick" walks that agent's ring from the newest slot backward at most 128 steps. The inspector does this. The tick does not search.

`sys_spatial_tick` on #251 builds a `NavScratch` every call. The trace does not add a second scratch, and it does not construct the ring there. The demo and the tests build one `DecisionTrace` and one `NavScratch` and reuse them.

When the resource is absent, the record macro takes the `None` branch and returns. That branch exists only in a feature-on build. A shipping build does not contain the branch. §6 is the distinction.

## 6. Zero cost, and the hash proof

Off means the crate is built without `decision-trace`. The call site is a macro in the style of `span!`:

```rust
macro_rules! decision_record {
    ($($tokens:tt)*) => {
        #[cfg(feature = "decision-trace")]
        { $($tokens)* }
    };
}
```

With the feature off, the tokens inside the macro are not type-checked and not executed. `World`, `AgentPlan`, and `PlanIntent` keep the layout they have on `main`. The five required replay cases compile against that layout.

With the feature on, a paused session (no `DecisionTrace` inserted) pays one resource lookup in the system that opts in. That lookup is the paused mode. It is not the zero-cost mode. The zero-cost mode is the feature flag off, which is the default and the configuration `determinism-replay.yml` already builds.

The proof test lives in `astraweave-ai` and runs in one process so both sinks are monomorphized:

```rust
fn script_bytes<S: DecisionSink>(sink: &mut S) -> Vec<u8>;
```

`NullSink::record` is an empty `#[inline(always)]` function. `RingSink::record` writes one `DecisionRecord`. Both return the same encoding the replay harness uses for a plan: little-endian action codes, cells, portal ids, and cover ids. The ring bytes are not appended.

`decision_trace_hash_matches_when_the_ring_is_off` asserts:

- `sha256(script_bytes(&mut NullSink)) == sha256(script_bytes(&mut RingSink))`
- `RingSink` count is greater than zero, so the traced run really wrote
- `NullSink` count is zero

A second test, `decision_trace_rerun_leaves_the_script_hash`, runs `rerun` (§7) after the script and asserts the script hash is unchanged. `rerun` takes the sample by value and a shared borrow of planner inputs. It has no `&mut World` and no `&mut AgentPlan`.

A third test, under `--features alloc-counter`, follows `zero_alloc_tick.rs`: warmup, `reset_allocs()`, one traced `plan_route` or one traced utility scoring pass, `assert_eq!(allocs(), 0)`.

The implementation also runs `cargo test -p astraweave-core --test replay_hash` and `cargo test -p astraweave-combat --test replay_hash` with default features. Those commands are the feature-off side of the five required cases. Their digests are not rewritten. `decision-trace` is not added to the workflow's `required` set.

## 7. Counterfactual replay API

```rust
#[derive(Clone, Copy)]
pub struct TickSample {
    pub record: DecisionRecord,
}

#[derive(Clone, Copy)]
pub enum Patch {
    NearestEnemy { x: i32, y: i32 },
    SmokeCooldownMilli(i32),
    Ammo(i32),
    SealCell { x: i32, y: i32 },
    GoalMilli { x: i32, y: i32, z: i32 },
}

#[derive(Clone, Copy)]
pub struct DecisionDiff {
    pub tick: u64,
    pub entity_id: u64,
    pub before: u16,
    pub after: u16,
    pub before_score: i32,
    pub after_score: i32,
    pub alt_len: u8,
    pub alts: [AltScore; ALT_CAP],
}

pub fn rerun(sample: TickSample, patch: Patch, view: &PlannerView<'_>) -> DecisionDiff;
```

`PlannerView` is a shared borrow of the nav publish and a scratch buffer the caller owns. The caller copies scratch before the call when it still needs the live waypoints. `rerun`:

1. Copies `sample` (it is `Copy`).
2. Applies exactly one `Patch` to that copy.
3. Calls the same pure function the tick used. `PlannerId::Htn` calls `plan_route`. `PlannerId::Utility` calls the integer smoke-versus-advance score. `PlannerId::Rule` calls `RuleOrchestrator::propose_plan` on a stack snapshot built from `PerceivedView`.
4. Writes the new winner into a stack `DecisionRecord` and returns the diff.
5. Returns. The live ring, the live plan, and the live nav tiles stay as they were.

`SealCell` sets a hazard hook of `kind = 1` on that cell in the copied hook list. That is the same predicate `best_breach` already uses. It does not call `DestructionWorld::drain_geometry_notices` and it does not `mark` a tile.

The grid path's `propose_plan` allocates a `PlanIntent`. That allocation is inside the planner, which already allocates on `main`, and it happens in the inspector call, outside the tick. The hot tick's `record` still allocates zero times. A later change can teach `RuleOrchestrator` to fill an `AgentPlan` directly; that change belongs to the planner owner, and this API does not require it for the spatial demo.

`rerun` draws no `SimRng` and reads no clock.

## 8. CLI first, editor panel later

### 8.1 Headless inspector

New binary crate `tools/aw_decision`. It depends on `astraweave-ai` with `decision-trace` and on `astraweave-nav`. It does not depend on `aw_editor`, `wgpu`, or `egui`.

```text
aw_decision show --script aravath-bridge --tick 31 --entity 1
aw_decision diff --script aravath-bridge --tick 31 --entity 1 --seal 1,17
```

`show` runs the in-process script, finds the record, and prints the five layers with the static string table. `diff` runs `rerun` with one patch and prints `before`, `after`, both scores, and the rejected rows. `--dump path` writes the raw `[DecisionRecord]` bytes (little-endian, length-prefixed count) for the panel to read later. The dump is an inspector write after the tick loop. The tick itself does not open a file.

Exit status is 0 when the record exists. A missing tick or entity exits 1 and prints the agent id and the tick range the ring holds.

### 8.2 Editor panel, after the CLI and after the editor CI failures close

A later PR, and only after issues #265 and #273 are closed, adds `PanelType::DecisionTrace` in the Debug category. Before editing, that PR lists every `PanelType` match in `panel_type.rs` and updates each one. `all()` includes the variant so the Debug menu shows it. The four `default_*` dock lists omit it. The panel reads a `DecisionTrace` resource when the editor binary was built with `decision-trace`, and otherwise reads the dump file from §8.1. It does not tick the simulation and it does not insert the resource into a play world that is hashing a replay.

That PR is also the one that updates `docs/architecture/ai_pipeline.md` (§5 file map, §8 invariants, §11, revision history) if the module landed without the trace update. This design PR does not edit the architecture trace, because no subsystem code changes here.

## 9. Aravath demo

The demo is headless. It lands only on a branch that already contains both `BridgeAgent` (PR #263, on top of #205) and `plan_route` (PR #251).

Script `aravath-bridge`:

- Scene line printed by the inspector: `Aravath`.
- Agent: entity id 1, the bridge companion. Start milli-position `(1000, 0, 1000)`, goal `(1000, 0, 46000)`, the same span #251's `nav_reroute` and #263's `BridgeAgent` already use.
- Ticks `0..40` at the fixed step. Tick 30 applies the collapse notice (reason 2) through the nav mapping #251 already has.
- The ring records each `plan_route`. At tick 31 the inspector shows the ford move, the breach or cover it scored, and the bridge portal it rejected once that portal is sealed or gone.
- `aw_decision diff --seal` on the ford cell shows the other action and the score delta.
- No window, no new file under `assets/`, no new pack. Player-facing strings say Aravath.

The greybox melee `Enemy` is a different body and stays unwired. `run_aravath_slice` on `main` stays the combat caller's demo.

## 10. Merge order

This PR is docs on `main`. It does not merge the drafts below. Implementation order after review:

| Step | Branch | What has to be true first |
|---|---|---|
| 0 | this design | Review answers §14. No code |
| 1 | #251 `feat/ai-orchestration` | Rebase onto `main` at or after `f1109340`. The required replay set keeps `planner_pure`, `perception_world`, `gameplay_rng`, `fixed64`, and `combat_resolve`, and adds `nav_reroute`. The current tip's workflow drops `gameplay_rng` and `fixed64`. The debugger implementation does not branch from that tip |
| 2 | #213 `cursor/engine-ws-ai2` | Rebase onto that same `main` without deleting `fixed64` or `combat_resolve`. Add `ai_depth_tick` as a further required case. Publish a `Copy` `BeliefView` from the depth tick when the belief line of this design is approved. Until that publish, `BeliefView` stays zero. This debugger does not edit `sys_ai_depth` |
| 3 | debugger implementation, new branch off the post-#251 main | `decision_trace.rs`, the three proof tests, `tools/aw_decision`. Feature off by default. No `aw_editor` edit. No `aravath_runtime` edit. No new required hash case |
| 4 | demo, branch that contains #263 and step 3 | `aravath-bridge` script and the two CLI commands. Still headless |
| 5 | editor panel | After #265 and #273 are closed. Separate PR. Reads the dump from step 3 |

Other work stays outside this tracer: #252, #214, and #244 are on main, #253 local sim, #243 destruction (the crate is already on `main`; that draft must not be re-merged over it), #215 terrain, #216 render, #217 CI burndown (the owner of #265), #275 combat wire, #264 local-sim host. The tracer reads `GeometryNotice` only as a documented input shape. It does not subscribe to destruction, fluids, or the character motor.

Issues #257 and #258 stay deferred. The trace has no `PlannerId` for a live model and no arm that calls the advanced GOAP tree.

## 11. Implementation sequence, after approval

Each step is its own commit on the implementation branch. `cargo check -p` the crate just touched, then that crate's tests.

1. Feature `decision-trace`, the types in §4, `DecisionTrace`, `record`, and the macro. No call site yet. Unit tests fill a ring, wrap at 128, and reject a 33rd agent.
2. `NullSink` / `RingSink` and `decision_trace_hash_matches_when_the_ring_is_off`.
3. Wire `plan_route`, `best_breach`, and `best_cover` to write losers into the slot. Winner tests on #251 stay green.
4. Wire the integer utility score and, on the grid path, a post-`propose_plan` copy into the ring. `planner_pure` bytes stay identical with the feature off.
5. Outcome store from the existing `Result<(), EngineError>` at the `hello_companion` and ECS call sites, behind the macro. `validate_and_execute` is not edited.
6. `rerun` and `decision_trace_rerun_leaves_the_script_hash`.
7. `decision_trace_alloc` under `alloc-counter`.
8. `tools/aw_decision` `show` and `diff` against a fixture script that does not need `aravath_runtime`. The Aravath script is step 4 of §10, on the branch that has `BridgeAgent`.
9. `cargo test -p astraweave-core --test replay_hash` and `cargo test -p astraweave-combat --test replay_hash` with default features. `cargo clippy -p astraweave-ai --all-features -- -D warnings` and the same for `aw_decision`.

The architecture trace update rides with step 3, in that commit: `docs/architecture/ai_pipeline.md` §5, §8, §11, and the revision history. Front-matter changes run `cargo run -p aw_trace_sync -- --write`.

## 12. Tests that define done

| Test | Asserts |
|---|---|
| Ring wrap | Record 129 writes on one agent; the oldest tick is gone; `count == 128` |
| Agent cap | A 33rd entity id is refused and the first 32 slots are unchanged |
| Winner stable | Sealed breach and equal-score cover tests from `htn.rs` return the same ids with the ring writing losers |
| Hash | Null sink and ring sink produce one SHA-256 over the plan bytes |
| Rerun isolation | After `rerun`, the script hash matches the hash from before the call |
| Zero alloc | One traced spatial tick reports `allocs() == 0` after warmup |
| Feature off | Default-feature `replay_hash` still emits the five required case names and does not emit a trace case |
| CLI | `show --tick 31 --entity 1` prints the planner id, the selected action, at least one rejected row, and the outcome code |

`cargo clippy -p astraweave-ai --all-features -- -D warnings` on the commits that touch it. The new module is `forbid(unsafe_code)` like the rest of the AI crate's safe surface. No new third-party crate.

## 13. Pre-existing failures

| Item | Disposition |
|---|---|
| #273 editor LDR hash mismatch on llvmpipe | Already `deferred-failure`. Owner: editor render parity. This work does not edit the parity harness and does not land the panel while it is open |
| #265 `libudev.pc` missing on editor Ubuntu/macOS jobs | Already `deferred-failure`. Owner: PR #217. This work does not edit `editor-ci.yml` or `aw-editor-tests.yml` |
| #225 `llm_dialogue.rs` does not compile | Already `deferred-failure`. Owner: the #213 line. This work does not declare the module |
| #257 `AIArbiter` | Deferred by #251. A live model stays off the tick. No trace arm |
| #258 advanced GOAP and `GoapPlanner` as the open-world planner | Deferred by #251. The ring does not call them |
| #251 required-set drop (`gameplay_rng`, `fixed64`, and the missing `combat_resolve`) | A defect on that draft, not on `main`. Owner: the #251 workstream, to fix on rebase before the debugger branches from it. This PR does not patch that workflow |
| Constant `breach_door` at `(15, 8)` on `main` | #251 already removes it. The trace shows whatever perception emitted. This PR does not change `build_snapshot` |
| `sys_ai_planning` allocates every tick | Existing cost. The macro adds no `to_string` and no `format` on top of it |

No new `deferred-failure` issue is opened by this PR.

## 14. Review questions

1. Confirm the recorder is `decision-trace`, default off, in `astraweave-ai`, with the ring in a separate `DecisionTrace` resource.
2. Confirm "off" means the feature is absent from the build, matching `span!`, and that a feature-on build with no resource inserted is only the paused mode.
3. Confirm the hash proof is the in-process null-sink versus ring-sink test, and that trace bytes stay out of `determinism-replay.yml`.
4. Confirm the counterfactual is `rerun(sample, patch, view)` over a `Copy` sample, one patch, no live world.
5. Confirm the first tool is `tools/aw_decision`, and the editor panel waits on #265 and #273.
6. Confirm the demo agent is `BridgeAgent` entity 1 on the #263/#251 stack, scene text "Aravath", and that `run_aravath_slice` and the greybox melee figurine stay unwired.
7. Confirm belief stays a zero `BeliefView` until #213 publishes a `Copy` summary, and that `StatKnowledge` is an optional mask the caller passes in.
8. Confirm §10's rebase rule for #251: the five required cases on `main` stay required, and `nav_reroute` is added beside them.
