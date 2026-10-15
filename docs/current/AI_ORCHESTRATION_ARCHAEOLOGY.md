# AI orchestration archaeology and upgrade plan

Survey of `main` at `85d09a2c` (2026-10-15). Phase 1 only. No runtime code. Implementation waits on the review questions in §10.

Status words used below:

| Status | Meaning |
|---|---|
| REAL | A non-test caller outside the owning crate exercises it. Demos and the editor count, and are marked as such. |
| STUBBED | The function exists and its comment promises incremental or spatial behavior the body does not perform. |
| DORMANT | It compiles, and often has tests, and no shipping caller runs it. Feature-off with no workspace crate enabling the feature counts. |
| UNTESTED | A shipping path has no test that would fail if the behavior changed. |
| ABSENT | No type and no algorithm in `*.rs`. |

## 1. What is wired today

The loop that actually runs is a 2D grid companion planner. It does not see destroyed cover, breaches, or a navmesh.

| Path | What it calls | Where |
|---|---|---|
| ECS planning, default features | `RuleOrchestrator` | `astraweave-ai/src/ecs_ai_plugin.rs` (`sys_ai_planning`, the `not(feature)` arm) |
| `examples/hello_companion` default | `RuleOrchestrator`, then `validate_and_execute` on the grid | `examples/hello_companion/src/main.rs` |
| `hello_companion` GOAP flag | `GoapOrchestrator` in `astraweave-ai`. That type is a one-step Manhattan heuristic. It is not `GoapPlanner`. | `astraweave-ai/src/orchestrator.rs` |
| `hello_companion --utility` | Local `calculate_*_score` functions in the example. Not `UtilityOrchestrator`. | `examples/hello_companion/src/main.rs` |
| IPC | `RuleOrchestrator` | `astraweave-ipc/src/lib.rs` |
| Slice runtime, feature-gated | Companion orchestrator calls `astraweave-behavior`'s `GoapPlanner` (real A*). | `astraweave-ai/src/veilweaver.rs`, feature `veilweaver_slice` in `astraweave-ai/Cargo.toml`; `GoapPlanner` in `astraweave-behavior/src/goap.rs` |
| Movement validation on that grid | `path_exists` over `IVec2` obstacles | `astraweave-core/src/validation.rs`, `astraweave-core/src/tools.rs` |

`hello_companion` builds its app with `build_app` and does not install `AiPlanningPlugin`. The plugin's only non-test example caller is `examples/ecs_ai_demo`.

`WorldSnapshot` is the perception product (`astraweave-core/src/schema.rs`). Fields are `t`, `player`, `me`, `enemies`, `pois`, `obstacles: Vec<IVec2>`, `objective`. There is no navmesh, no height, no destruction record. `build_snapshot` (`astraweave-core/src/perception.rs`) sets every visible enemy's cover from a Manhattan check against `los_max` (far → unknown, near → low) and always emits one POI named `breach_door` at `(15, 8)`. The ECS-only arm of `sys_ai_planning` builds a snapshot with `obstacles: vec![]` and `t: 0.0`.

Nothing in this loop subscribes to physics destruction or terrain edits.

## 2. Inventory

### 2.1 Planners in `astraweave-ai`

`astraweave-ai/src/lib.rs` declares `core_loop`, `ecs_ai_plugin`, `orchestrator`, `tool_sandbox`. The rest is feature-gated or not a module.

| Surface | Status | Evidence |
|---|---|---|
| `Orchestrator` / `RuleOrchestrator` | REAL | `astraweave-ai/src/orchestrator.rs`. Callers in §1. Allocates a `String` plan id and a `Vec` of steps on every call. |
| `GoapOrchestrator` (heuristic) | REAL in the demo only | `orchestrator.rs`. The doc comment on the struct says it is not the symbolic planner. |
| `UtilityOrchestrator` | DORMANT | `orchestrator.rs`. Zero non-test callers. Scores two smoke/advance candidates into a fresh `Vec`. |
| `PlannerMode`, `CAiController`, `dispatch_planner` | DORMANT | `astraweave-ai/src/core_loop.rs`. No shipping caller. `ai-goap` and `ai-bt` are off, and no workspace `Cargo.toml` enables them. |
| `LlmOrchestrator`, `AIArbiter`, `LlmExecutor`, `AsyncTask` | DORMANT | Gated by `llm_orchestrator` (default off). `AIArbiter::` has no non-test constructor. Runtime model string when that path is built is `phi3:medium` (`orchestrator.rs`). A live model is not deterministic. |
| `make_system_orchestrator` | DORMANT | Same file. Default branch constructs `UtilityOrchestrator`. |
| `tool_sandbox::validate_tool_action` and `ValidationContext::with_nav` | DORMANT | `astraweave-ai/src/tool_sandbox.rs` (`with_nav`, `MoveTo` arm calls `nav.find_path`). Tests and benches only. The engine gate remains `validate_and_execute`. |
| `astraweave-ai/src/goap/` (22 files, `plan_decomposed`) | DORMANT | Feature `planner_advanced` (default off). `plan_decomposed` (`goap/planner.rs`) splits a goal into sub-goals and runs A*. It returns `Vec<String>` action names. `GOAPOrchestrator` in that tree does not implement `Orchestrator`. Bins under `src/bin.disabled/` are not crate targets. |
| `astraweave-ai/src/persona/`, `astraweave-ai/src/rag/` | DORMANT, not compiled | No `mod` line in `lib.rs`. Feature `rag = []` wires nothing. |
| AI LOD, tick budget, interest management for agents | ABSENT | No `AiLod` / `tick_budget` in `*.rs`. `budget_ms` on `OrchestratorAsync` is an LLM timeout. |

`sys_ai_planning` allocates on every tick: `Box<dyn Orchestrator>`, job `Vec`s, `BTreeSet`, and `reason.to_string()` (`ecs_ai_plugin.rs`).

### 2.2 Behavior trees and the real GOAP

| Surface | Status | Evidence |
|---|---|---|
| `BehaviorGraph` / `BehaviorNode` | REAL | `astraweave-behavior/src/lib.rs`. Sequence, selector, action, condition, decorator, parallel. Used by `hello_companion` and the editor. This is not an HTN: there are no methods, branches, or backtracking preconditions. |
| `GoapPlanner` (A* over interned facts) | REAL inside the slice runtime only | `astraweave-behavior/src/goap.rs`. Non-test caller is `astraweave-ai/src/veilweaver.rs`. |
| `CachedGoapPlanner` | DORMANT | `astraweave-behavior/src/goap_cache.rs`. Tests and benches. |
| `behavior_tick_system` | REAL in the slice runtime | `astraweave-behavior/src/ecs.rs`. Collects `to_update = vec![]` every tick. |

### 2.3 HTN and utility

| Idea | Status |
|---|---|
| HTN planner (compound tasks, ordered methods, backtracking) | ABSENT. The behavior crate's module docs mention HTN. The node enum is a behavior tree. `plan_decomposed` is goal splitting on top of GOAP, feature-off, and unwired. |
| Utility as a consideration stack (curves, weights, position scores) | ABSENT. `UtilityOrchestrator` and the `hello_companion` scores are a fixed handful of combat steps. |

### 2.4 Memory, RAG, persona, coordination (PR #213's crates)

On `main` these crates are implemented and tested, and they are not on the planning tick.

| Crate | Status on `main` | Note |
|---|---|---|
| `astraweave-memory` | DORMANT for the planning tick | No `use` from `astraweave-ai`. `astraweave-persistence-ecs` declares the dep and does not use it. |
| `astraweave-embeddings` | DORMANT; search is a linear scan | `VectorStore::search` calls `brute_force_search` (`astraweave-embeddings/src/store.rs`). `hnsw_rs` is a dependency and is not referenced from `src/`. |
| `astraweave-rag`, `astraweave-context` | DORMANT for the planning tick | Held by LLM-side modules and examples. |
| `astraweave-persona` | Zip loader is demo-real (`examples/persona_loader`). LLM manager is DORMANT. | |
| `astraweave-coordination` | DORMANT | No non-test caller. |
| `astraweave-npc` | Demo-real only | `examples/npc_town_demo`. Not in the ECS planning stage. |
| `astraweave-dialogue` graph / TOML | REAL | Editor and the slice runtime. |
| `astraweave-dialogue/src/llm_dialogue.rs` | DORMANT, not a module | Issue #225. See §9. |
| `astraweave-director` heuristic warden | REAL in the slice runtime | LLM director is DORMANT. |

PR #213 (`cursor/engine-ws-ai2`, draft) is the owner of wiring those crates. Its `sys_ai_depth` (`astraweave-dialogue/src/depth_tick.rs` on that branch) calls memory, embeddings, RAG consolidation, prompts, context, persona, NPC `RulePlanner`, and coordination once per fixed step. It does not call a live model. A missing log line becomes the rendered prompt with a `scripted:` prefix. `SimRng` is seeded and not drawn. That branch adds `SIM_SYSTEM_HNSW = 0xA1DE_7001` and `SIM_SYSTEM_LLM_DIALOGUE = 0xD1A1_7001`. It also adds a real HNSW index whose level draw uses `SimRng`. This plan does not edit that tick and does not compile `llm_dialogue.rs`.

### 2.5 Navigation

The production implementation is one file: `astraweave-nav/src/lib.rs`. `crates/astraweave-nav/` is an orphan benchmark with no `Cargo.toml`.

`NavMesh::bake` filters triangles (degenerate, downward, slope via `acos`), then builds adjacency with an O(n²) `share_edge` loop. `find_path` snaps to the nearest triangle center (linear scan), runs A* on centers, and smooths with two 0.25/0.5/0.25 passes. Endpoints are not constrained to the mesh. `max_step` is stored and never read.

| Surface | Status | Evidence |
|---|---|---|
| `bake` / `find_path` | REAL implementation. Runtime callers are the editor bake button and the scripting proxy. | `tools/aw_editor/src/main.rs` (bake). `astraweave-scripting/src/api.rs` (`find_path`) if some caller inserted a `NavMesh` resource. The non-test insert is `examples/scripting_advanced_demo`. |
| `invalidate_region`, `rebake_dirty_regions` | DORMANT | Tests only. `rebake_dirty_regions` always calls full `bake`. |
| `partial_rebake` | STUBBED | `astraweave-nav/src/lib.rs`. It counts triangles whose AABB hits a dirty region, then calls `rebake_dirty_regions`. The comment says a production system would rebuild the affected portion. |
| Recast, Detour, `dtNavMesh`, tile cache | ABSENT | Docs under `docs/src/` mention them. No Rust nav code does. |
| Editor navigation panel bake | STUBBED | `tools/aw_editor/src/panels/navigation_panel.rs` logs and does not call `bake`. The working button is in `main.rs`. |
| `astraweave-gameplay` weave path | STUBBED | `astraweave-gameplay/src/weaving.rs` binds `NavMesh::bake` to `_nav` and drops it. |
| Portal graph `string_pull` | DORMANT | `astraweave-gameplay/src/weave_portals.rs`. Tests only. |
| Flow fields | ABSENT | |
| `find_cover_positions` | DORMANT | `astraweave-core/src/tools.rs`. Tests only. |
| Terrain `NavMeshRegion` | DORMANT bridge | `astraweave-terrain` records dirty regions. Nothing calls `NavMesh::invalidate_region`. |
| `DestructionEvent` | No nav subscriber | `astraweave-physics/src/destruction.rs`. |

There is no tile id, no border portal, and no generation counter.

### 2.6 Destruction events this plan has to consume

PR #243 (`feat/destruction`, draft, docs only) has no crate yet. The design's log is the event source:

```rust
DestructionCommand { tick, seq, entity_id, kind }
CommandKind::{ Impact, Explosion, Brush, Detach, Anchor }
```

Applied in `(tick, seq, entity_id)` order inside the 60 Hz tick. Coordinates in the log are `Fixed64` or integers. `SIM_SYSTEM_DESTRUCTION = 0xD357_7001` in that design. The design does not mention navmesh. This plan does not add fracture, debris, or terrain deltas. It consumes AABBs derived from commands that change walkable geometry (`Explosion`, `Brush`, `Detach`).

On `main`, `DestructionManager` does not drive nav. Do not extend it. #243 already rejected that module as the destruction owner.

### 2.7 Traversal and combat, so this plan does not absorb them

PR #244 (`feat/traversal-archaeology`, draft) owns displacement: climb, swim, vault, the character motor. The wired move on `main` is `PhysicsWorld::control_character` (`astraweave-physics/src/lib.rs`), a kinematic capsule. This plan emits a path and a plan. It does not move the body and does not add ledge probes.

PR #212 (`cursor/combat-damage-design`, draft) owns `resolve_hit` / `resolve_heal` and `StatKnowledge` (`astraweave-combat/src/resolve.rs` on that branch). `SIM_SYSTEM_COMBAT = 0xC0BA_7001`. AI estimates read the mask. This plan does not resolve hits. A later utility term may read `StatKnowledge` the way that PR documents; it does not read hidden stats.

### 2.8 Determinism already on `main`

`SimRng` is `astraweave-core/src/sim.rs`: `(world_seed, tick, system_id)` into ChaCha12 via `mix_sim_seed`. Ids on `main`:

| Id | Constant |
|---|---|
| `0xC2AF_7001` | `SIM_SYSTEM_CRAFT` |
| `0xC2AF_7002` | `SIM_SYSTEM_HARVEST` |
| `0xAE11_7001` | `SIM_SYSTEM_ANCHOR_PARTICLES` |
| `0xAE11_7002` | `SIM_SYSTEM_ENEMY_PATROL` |
| `0xAE11_7003` | `SIM_SYSTEM_SPAWNER` |
| `0x7E22_7001` | `SIM_SYSTEM_TERRAIN_SOLVER` |

`.github/workflows/determinism-replay.yml` requires `planner_pure` and `perception_world` from `astraweave-core/tests/replay_hash.rs`. Both hash the grid snapshot and `RuleOrchestrator` / heuristic GOAP. Neither loads a navmesh, removes a bridge, nor checks a reroute.

Nav bake calls `f32::acos`. That is a libm input. The replay workflow already pins `target-cpu=x86-64` on x86_64 and forbids `target-cpu=native`.

### 2.9 Aravath demo

`aravath_runtime/` is not on `main`. `docs/current/ENGINE_PRODUCTION_ROADMAP.md` places it on `cursor/aravath-greybox-b3de`. This PR does not edit that tree and does not add assets.

## 3. What shipped games published

These are the sources for the decision in §4. They are not claims about this repository.

**F.E.A.R. GOAP.** Jeff Orkin, "Three States and a Plan: The A.I. of F.E.A.R." (GDC 2006). Each action has preconditions, effects, and a cost. Regressive A* searches the action space from the goal. Squad mates share the action set and differ by cost. The action set is small, and the search replans when the world invalidates the plan. That is the shape of `GoapPlanner` in `astraweave-behavior`: a flat A* over symbolic facts. It does not decompose a domain, and the cost grows with the action set. An open-world activity list (travel, use a breach, take cover, flee, regroup) makes that search the wrong inner loop for every agent every tick.

**Killzone position evaluation, then Killzone 2 HTN.** Guerrilla's GDCE 2005 Killzone slides ("Dynamic procedural tactics") score candidate waypoints with a weighted sum: proximity, line of fire to the primary threat, cover from secondary threats, preferred range. The highest score wins. Tactical path costs add exposure and friendly lines of fire onto A* links. Tim Verweij's Killzone 2 multiplayer bots (Paris Game AI Conference 2009) put HTN on the individual, with strategy and squad layers above it, and replan on a fixed cadence. The position score is how a method chooses a place. It is not the planner.

**Horizon / Decima HTN, plus utility.** Guerrilla, "The AI of Horizon Zero Dawn" (17 October 2017), states the combination directly: HTN planning and utility-based decision making, plus navigation for agents of different sizes in a dynamic environment. Their planner writeup decomposes an abstract task through ordered branches. Each branch has declarative preconditions and replacement tasks. Failure backtracks. Tim Verweij, "HTN Planning in Decima" (2024), is the same planner: backtracking over preconditions, compiled to generated code. Individual machines request a plan; group agents assign roles. Julian Berteling's GDC talk "Beyond Killzone: Creating New AI Systems for Horizon Zero Dawn" is the navigation and animation half of that jump from corridors to an open world.

**The Division.** The public Massive talk is Jonas Gillberg, "AI Behavior Editing and Debugging in Tom Clancy's The Division" (GDC). Snowdrop authors behavior trees, with shared tools so designers can edit and debug a large NPC set. It is not a published utility architecture. This repo already has that layer (`BehaviorGraph`). Copying it again does not add spatial replanning.

**Recast / Detour tiles.** `dtTileCache::update` rebuilds tiles touched by obstacle requests, one tile per call, from a compressed heightfield plus cylinder or box obstacles (`DetourTileCache.h`). That is the right schedule (bounded work, tile ids, publish into `dtNavMesh`). It is the wrong edit model for a collapsed bridge or a reshaped crater: those change source triangles, and the tile cache only stamps boxes and cylinders. Structural edits use `removeTile` / `addTile` (or `buildNavMeshTilesAt`) from the new geometry. Recast's raster and contour steps are `f32` C++. Putting that library on the sim hash fights the cross-OS replay rule. The idea to keep is the tile, the border link, and the bounded commit. The code to keep is `astraweave-nav`.

## 4. Decision

**Upgrade `astraweave-nav` in place, and add an HTN whose method choice is a utility score. Do not add a second planner crate, a second nav crate, or a Recast dependency. Do not revive the advanced GOAP tree or `AIArbiter` as this system.**

Rejected:

| Option | Why not |
|---|---|
| Make `GoapPlanner` the open-world planner | Flat action search. The only shipping caller is the slice companion. Scaling it to every agent reallocates `WorldState` and strings, which §5 forbids. |
| Finish `astraweave-ai/src/goap/` | Second GOAP, feature-off, does not implement `Orchestrator`, returns action-name strings. Wiring it duplicates `astraweave-behavior`. |
| Grow `UtilityOrchestrator` | No task stack, no backtrack, no spatial query. Two hardcoded candidates. |
| Grow `BehaviorGraph` into an HTN | Different data model (tick status vs. a decomposed plan). The Division talk is why the tree stays as an authored executor, not why it becomes the massive-agent planner. |
| Vendor Recast | `f32` C++ on the sim hash, and `dtTileCache` does not rebuild source geometry. |
| New nav crate beside `astraweave-nav` | `crates/astraweave-nav` is already the orphan that mistake produces. |
| Live LLM / `AIArbiter` on the tick | Not deterministic. #213 already proved the depth tick can run with a recorded line and no model. |

What stays:

- `RuleOrchestrator` remains the far-band fallback and the existing replay cases `planner_pure` / `perception_world`.
- `WorldSnapshot` field names stay. LLM prompts and the grid planners read them. The new spatial view is a separate struct.
- `BehaviorGraph` stays for authored scenes and the slice. The hot tick does not call `behavior_tick_system`.
- `GoapPlanner` stays for the slice companion. This work does not retarget it.
- Editor `NavMesh::bake` and scripting `find_path` keep their signatures. Tile rebuild is a new type next to `NavMesh`, and `partial_rebake` delegates to it instead of pretending.

## 5. Phase 2 design (not in this PR)

### 5.1 Nav tiles

New type `NavTiles` in `astraweave-nav`. Fixed world tile size (proposed 32 m; §10). Each tile stores its own triangles and neighbor indices. Border portals are edges that match a portal on `tile_id ± 1` within the existing `1e-3` epsilon, stored in sorted `(tile, edge)` order.

`NavChange` is the only input from the world:

```rust
pub struct NavChange {
    pub tick: u64,
    pub seq: u32,
    pub min_milli: [i32; 3],
    pub max_milli: [i32; 3],
}
```

A change marks every tile the milli-unit AABB overlaps. Dirty tile ids sit in a sorted unique list.

**Commit.** At most `K` tiles per tick (proposed `K = 4`, matching #243's remesh cap). The sim thread rebuilds the lowest `K` dirty ids, in order, by calling one pure function `build_tile(tris, params) -> Tile`. Publish replaces those slots in the immutable table the AI stage borrows. Agents do not take a lock. A worker may run `build_tile` into a side buffer. The sim accepts that buffer only when its input hash equals the hash of the triangles it would have built itself. Otherwise the sim builds the tile. Thread completion order never changes which ids commit. That is the reading of the determinism rule that allows async work: the async result is bit-identical to the CPU function, and the commit set is order-independent.

`partial_rebake` stops calling full `bake`. It pushes one `NavChange` per dirty AABB and commits. `rebake_count` remains the number of commits. Tests that assumed a full rebuild of unaffected triangles get updated in that commit, because those tests currently lock in the stub.

Path query reads the published table only. A* walks tile-local neighbors and border portals. The open set is a fixed array in `NavScratch` (cleared, not freed). Waypoints are written into a caller-provided buffer. No `Vec` growth on the query.

Slope filtering stays a dot test against a precomputed cosine threshold so the hot rebuild does not call `acos`. `max_step` becomes a real reject: a shared edge whose height delta exceeds `max_step` is not a portal. Today the field is inert; changing that is behavior, called out in §10.

Cover and breaches fall out of the same publish. A triangle that lost all walkable neighbors on one side is cover. A border portal that exists this generation and did not exist last generation is a breach. Both lists are sorted by id. The hardcoded `breach_door` POI is not an input.

### 5.2 Mapping #243's log

When `DestructionCommand` exists, a single adapter appends `NavChange` values in the log's existing order:

| Command | AABB |
|---|---|
| `Explosion` | center ± radius, quantized once at the edge |
| `Brush` | cell origin and radius or half extents |
| `Detach` | the detached node's chunk AABB |
| `Impact` | omitted until debris occupancy is queryable |
| `Anchor` | no nav change |

Until that crate merges, the replay and the unit tests construct `NavChange` directly. This branch does not copy `DestructionCommand` and does not depend on a crate that is not on `main`.

### 5.3 HTN plus utility

New module `astraweave-ai/src/htn.rs`, always compiled, implements `Orchestrator` for the grid snapshot so the old tests still type-check, and a second entry `plan_spatial` for the nav-aware tick.

A domain is static data: compound tasks, methods with an ordered branch list, preconditions as predicates over a borrowed `SpatialView`. Decomposition follows Horizon's order: try branches top to bottom, bind the first whose preconditions hold, backtrack when a child fails. No search over an action set.

The method that needs a place (take cover, use breach, cross) does not hardcode a cell. It scores candidates from the nav publish with integer weights, in the Killzone form:

- distance in milli-units from the agent
- the candidate still exists after the last commit (destroyed cover scores zero and drops out)
- a new breach portal that shortens the remaining path
- preferred range band

Tie-break is lower candidate id. No `SimRng` draw. Equal costs cannot depend on `HashMap` iteration.

The plan is written into `[ActionSlot; 8]` on the agent. `ActionSlot` is a copy of the `ActionStep` variants the tick needs (`MoveTo` cell or waypoint index, `CoverFire`, `Wait`). No `String` plan id. The id is `(tick, entity_id)`.

`plan_spatial` does not call `MemoryManager`, `VectorStore::search`, `RagPipeline`, or `sys_ai_depth`. Those stay on #213's tick. A later step may copy a fixed `Consideration` struct that #213 has already produced. It is not in the first implementation steps, and it is not read from inside HTN if the copy is missing.

### 5.4 LOD and the tick budget

Agents are sorted by `(band, entity_id)` once per tick. Bands use distance to the player in milli-units:

| Band | Work |
|---|---|
| 0 | Full HTN. Replan when a committed tile intersects the current path, and at least every 30 ticks. |
| 1 | Utility choice among the methods of the current compound task. No backtrack. |
| 2 | Keep the last plan. If none, one `RuleOrchestrator` step. |
| 3 | Not visited. |

The tick spends at most `H` HTN expansions (proposed `H = 64`) and the nav stage spends at most `K` tile builds. Agents past the budget keep their last plan and sort first next tick only within their band, still by entity id, so the set is a pure function of the world. There is no mutex and no channel wait on this path.

The legacy `sys_ai_planning` body is not the hot path. The new system is a separate schedule stage that runs after the nav commit and does not allocate the `Box<dyn Orchestrator>` per tick. `RuleOrchestrator` stays for the grid replay cases.

### 5.5 Reserved `SimRng` ids

Planning and tile builds draw no random numbers. Reserve the ids so a later tie-break cannot collide:

| Constant | Value | Collides with |
|---|---|---|
| `SIM_SYSTEM_NAV` | `0x4E41_7001` | none of §2.8, #212, #213, or #243 |
| `SIM_SYSTEM_HTN` | `0x4854_7001` | none of those |

### 5.6 Replay case `nav_reroute`

Added to the `required` set in `.github/workflows/determinism-replay.yml`. The case lives next to the existing core replay test (a dev-dependency on `astraweave-nav` and `astraweave-ai` from that test, or a thin test in `astraweave-ai` that writes the same `target/replay-hash.json` shape #213 uses). The workflow compare already fails a missing required name.

Script, fixed 60 Hz, seed recorded in the test:

1. Tiles for a span: a bridge over a gap, and a longer ford beside it.
2. One agent, goal on the far side. The first path's portal ids cross the bridge.
3. At tick 30, one `NavChange` covers the bridge AABB (the same shape an `Explosion` or `Detach` will produce).
4. The following ticks commit that tile with the bridge triangles removed.
5. The agent replans. The new path uses the ford. The hash covers the committed tile ids, the portal id sequence, and the `ActionSlot` buffer.

The hash is SHA-256 of little-endian integers and `u32` bit patterns. It does not hash a `Debug` string or a `HashMap`. Ubuntu, Windows, and macOS must print the same digest. The case does not start a worker thread. Workers are tested separately by checking `build_tile` equality against the sim path.

`planner_pure` and `perception_world` stay required. If #212, #213, or #243 have already added `combat_resolve`, `ai_depth_tick`, or `destruction` on `main` by the time this lands, the required set keeps them.

### 5.7 Aravath showcase

After the replay case is green, and only when `aravath_runtime` is on the branch being integrated: one agent walks a CC0 bridge (KayKit or Kenney, already licensed on that branch; no new pack), the destruction log drops the span, the agent takes the ford. HUD strings say Aravath. This PR does not add the scene.

## 6. Coordination

| PR | Boundary |
|---|---|
| #213 | Do not edit `sys_ai_depth`, embeddings, or `llm_dialogue.rs`. Do not call RAG or the linear scan from the HTN tick. `SIM_SYSTEM_HNSW` and `SIM_SYSTEM_LLM_DIALOGUE` stay theirs. |
| #243 | Nav is a consumer of `Explosion`, `Brush`, and `Detach` AABBs. No support graph, fracture, debris, or terrain delta in this work. `SIM_SYSTEM_DESTRUCTION` stays theirs. |
| #244 | Paths stop at waypoints. Climb, mantle, swim, and the motor stay there. No character controller edits. |
| #212 | No `resolve_hit`. A utility term may later take `StatKnowledge` and must ignore masked fields. `SIM_SYSTEM_COMBAT` stays theirs. |

Also out of scope: `astraweave-director`, `astraweave-npc` physics stepping, the editor navigation panel beyond the existing bake button, and any asset under `assets/`.

## 7. Phase 2 order, after approval

Each step is its own commit. `cargo check -p` for the crate just touched, then that crate's tests. Stop if a pre-existing failure shows up (§9).

1. `NavTiles`, `NavChange`, sorted dirty ids, `build_tile` without `acos`. `partial_rebake` delegates. Existing `find_path` tests stay on `NavMesh` until a later commit opts callers in.
2. Border portals, scratch-buffer A*, destroyed-cover and new-breach lists.
3. Replay case `nav_reroute` and the workflow required set. Sync commit only.
4. Side-buffer acceptance test: a mismatched input hash is ignored and the sim builds the tile.
5. HTN domain, utility method scores, `[ActionSlot; 8]`, LOD bands, expansion budget. No call into #213's crates.
6. Adapter comment and a pure function from `(kind tag, milli AABB)` so #243 can append `NavChange` without this crate importing destruction.
7. Trace updates: `docs/architecture/nav.md` and `docs/architecture/ai_pipeline.md` (§5 file map, §8 invariants, §11, revision history). Then `cargo run -p aw_trace_sync -- --write` if front-matter changed.
8. Aravath call site when that runtime is on the branch. CC0 bridge. Player text says Aravath.

## 8. Tests that define "done"

| Test | Asserts |
|---|---|
| Tile isolation | Editing one tile does not change another tile's bytes |
| Stub gone | `partial_rebake` leaves triangles outside the AABB identical |
| Bridge | Path portal ids cross the bridge, then the ford, after one `NavChange` |
| Order | Two changes in either insertion order commit in `(tick, seq)` order |
| Budget | Five dirty tiles and `K = 4` leave the highest id dirty |
| No heap on the query | The path query and `plan_spatial` record zero allocations under the existing `alloc-counter` feature (the `alloc_measure` bench pattern) |
| No lock | The AI stage holds no `Mutex` guard; the published table is a shared borrow |
| Determinism | `nav_reroute` identical on three OS |
| Ties | Two equal utility scores pick the lower id |
| Dropped cover | A cover candidate inside the committed tile disappears from the score list |

`cargo clippy -p astraweave-nav -p astraweave-ai --all-features -- -D warnings` on the commits that touch them. Unsafe stays forbidden in `astraweave-nav`.

## 9. Pre-existing failures

| Item | Disposition |
|---|---|
| `partial_rebake` full-bake stub | This plan's step 1. Not a separate defect to file. The tests pass because they assert the stub. |
| `llm_dialogue.rs` does not compile (`rand::thread_rng`, API drift, 69 errors) | Already open as issue #225, label `deferred-failure`. Owner is the #213 line. This work does not declare the module. |
| `DestructionManager` has no nav subscriber | #243 owns destruction and already rejected extending that manager. This plan will not file a second issue. |
| Inert `max_step` | Fixed only if §10 accepts the behavior change. Until then it stays a documented inert field, not a silent change. |
| CI on #213 (editor, deny, toolchain, benchmark shard) | Their branch, not `main`. Not reproduced here. Not absorbed. |

No new `deferred-failure` issue is opened by this PR. The failures above already have an owner or are the work itself.

## 10. Review questions

1. Confirm the hybrid: HTN decomposition with Killzone-style utility scores for places, `RuleOrchestrator` only as the far band, no GOAP revival, no Recast.
2. Confirm async means a bit-identical side buffer, and the sim still commits the lowest `K` tile ids itself when the buffer does not match. Replay does not depend on worker timing.
3. Confirm `NavChange` milli-unit AABBs are the #243 seam (`Explosion`, `Brush`, `Detach` only), and this crate does not take a dependency on destruction until that crate is on `main`.
4. Confirm tile size 32 m, `K = 4` tile commits per tick, `H = 64` HTN expansions per tick, and bands at fixed milli-unit radii.
5. Confirm `max_step` may start rejecting portals. That changes bake results relative to today's inert field.
6. Confirm the hot tick writes `ActionSlot` and does not tick `BehaviorGraph` or `sys_ai_depth`.
7. Confirm the Aravath scene waits until `aravath_runtime` is on the integration branch, and this PR stays docs-only.

## 11. Phase 2 status

Approved 2026-10-15. Decisions 1–5 are implemented on `feat/ai-orchestration`. This branch still does not merge.

| Decision | Where |
|---|---|
| HTN with utility scores | `astraweave-ai/src/htn.rs` |
| Side buffer accepted only on a matching input hash | `NavTiles::accept_side_buffer` |
| `Explosion`, `Brush`, `Detach` | `NavChangeKind` and `nav_change_from_kind` |
| 32 m, 4 commits, 64 expansions | `TILE_SIZE_M`, `TILE_COMMITS_PER_TICK`, `HTN_EXPANSIONS_PER_TICK`. Unit tests assert each budget is under 11 ms. Benches: `tile_commits_per_tick`, `htn_expansions_per_tick` |
| `max_step` live | `share_edge`. Golden `max_step_links_a_low_ledge_and_rejects_a_tall_one` locks a 0.3 m link and a 1.0 m reject. Coplanar neighbor counts stayed 1 |

Dormant systems:

| Surface | Decision |
|---|---|
| `UtilityOrchestrator` | Wired. `propose_plan` calls `htn::legacy_utility_plan` |
| `dispatch_planner` | Wired. `PlannerMode::Htn` is always available and the ECS demo runs `SpatialTick` |
| Slice companion planner | Wired. Method select by priority. It no longer calls `GoapPlanner` or reads the wall clock |
| `AIArbiter` | Deferred. A live model cannot write the sim tick. Issue #257 |
| `astraweave-ai/src/goap/` (22 files) and `GoapPlanner` | Deferred. The slice path no longer calls them. Deleting them would drop the symbolic A* suite without a zero-alloc replacement. Issue #258 |

`nav_reroute` is a required case in `.github/workflows/determinism-replay.yml`.

The Aravath runtime is not on this branch, so the showcase scene is not added.
