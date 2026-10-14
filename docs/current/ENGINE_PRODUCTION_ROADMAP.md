# Engine Production Roadmap

**Status**: audit + phased workstreams. No engine code changes in the audit commit.
**HEAD audited**: `a3fe3ecd` (`main`, 2026-08-05). Workspace: 133 packages (`cargo metadata --no-deps`).
**Audience**: agents who will implement one workstream each, in parallel.
**Date**: 2026-10-14.

The Aravath greybox (`cursor/aravath-greybox-b3de`) is a proof that a scene can run. It is not the definition of a finished engine. This roadmap is the path to a production-grade, fully featured engine. [R.1](ROADMAP_R1_PATH_TO_V1.md) remains the ratified *v1.0 editor* sequence (networking deferred there). This document raises the bar: every shipped sim system is functionally complete **and** passes the determinism gate in §2.

---

## 0. Determinism gate (applies to every workstream)

100% determinism is a merge gate, not a follow-up. A workstream is unfinished until every simulation value it writes satisfies all eight rules.

The deterministic domain is the simulation tick: ECS component state, the legacy `World`, physics body state, navigation queries used by gameplay, AI plans applied on that tick, RNG draws, and anything persisted as sim state. Presentation (pixels, audio device clocks, editor chrome) may use wall time. It must not write that time, an unseeded draw, or a completion-order race back into the tick.

1. **Replay hash.** Two processes of the same binary, given the same seed and the same recorded inputs, produce identical SHA-256 digests of the canonical state encoding. Equality within a tolerance (`1e-4`, `1e-6`) does not pass.
2. **Cross-platform.** The same digest matches on Linux, Windows, and macOS under the pinned toolchain (Rust 1.89.0) with a pinned baseline CPU (`-C target-cpu=x86-64` on x86_64, the default aarch64 baseline on Apple Silicon). `target-cpu=native` is forbidden for this job.
3. **Clocks.** The tick does not read `SystemTime`, `Instant`, or UTC. A frame loop may use `Instant` only to choose how many fixed steps to run. Replay feeds that integer step count (or the recorded `dt` sequence). `App::run_fixed` is the reference driver (`astraweave-ecs/src/lib.rs`).
4. **Ordered collections.** Iteration that affects outputs uses `BTreeMap` / `BTreeSet` or a `Vec` sorted by a stable key (entity id, coordinate). `HashMap`, `HashSet`, and `DashMap` iteration is forbidden on that path. Rust randomizes `HashMap` seeds per map, so two worlds built the same way in one process can already diverge.
5. **RNG.** Sim draws come from an explicit `StdRng` / ChaCha seeded by `(world_seed, tick, system_id)`. `rand::random` and `thread_rng` are forbidden in sim crates. Authoring tools may roll a seed once; the seed that is stored is the sim input.
6. **LLM and async I/O.** Model calls, HTTP, and task-completion time sit outside the tick. A plan applied on tick N is bytes in the recorded input log. Replay reads the log and does not call the model. `AIArbiter` polling a background task is outside the deterministic profile until that log exists.
7. **Threads.** The ECS schedule stays single-threaded and registration-ordered. `rayon` / `tokio::spawn` may prepare data (mesh cook, asset I/O) only when the merge is a pure function of seed plus inputs and happens in stable key order. They do not mutate sim state inside the tick.
8. **Floats.** Values that enter the hash are `f32::to_bits` after a defined operation order. No fast-math. Reductions that depend on thread count or on FMA-vs-libm stay out of the tick. Integer grid state (`IVec2`) is the preferred gameplay representation and is already what `RuleOrchestrator` consumes.

Canonical encoding (the bytes that are hashed):

- little-endian integers
- entity records sorted by id
- map entries sorted by key
- obstacles sorted by `(x, y)`
- `f32` stored as `to_bits` (IEEE-754 bit pattern)
- strings as UTF-8 with an explicit length prefix
- digest: SHA-256 (already a workspace dependency)

`std::collections::hash_map::DefaultHasher` is the wrong digest. Its algorithm is unspecified across rustc versions. `veilweaver_slice_runtime/src/determinism.rs` already uses an FNV-style hasher for in-process compares; the engine gate uses SHA-256 so the artifact is stable across OS and toolchain.

### CI job to add

New workflow `.github/workflows/determinism-replay.yml`, on pull requests and on pushes to `main`. It becomes a required check.

**Job `hash`** (matrix: `ubuntu-latest`, `windows-latest`, `macos-latest`):

- install the pinned 1.89.0 toolchain
- set `RUSTFLAGS` to the baseline CPU for that runner
- run `cargo test -p astraweave-core --test replay_hash -- --nocapture`
- the test writes `target/replay-hash.json`: a list of `{ "case": string, "sha256": hex }`
- upload the JSON as artifact `replay-hash-${{ runner.os }}`

**Job `compare`** (needs all three `hash` legs):

- download the three artifacts
- fail if any case's hex differs across OS
- fail if a required case is missing

**Cases, and when each becomes required:**

| Case | What it runs | Required when |
|---|---|---|
| `planner_pure` | Hand-built `WorldSnapshot` → `RuleOrchestrator::propose_plan` and `GoapOrchestrator::propose_plan`, full `PlanIntent` bytes | WS-D0 merges (this case is already pure; see `astraweave-ai/tests/a2_wired_path_determinism.rs`) |
| `perception_world` | Legacy `World` built through `spawn`, then `enemies_of` + `build_snapshot` + `propose_plan`, 100 fixed ticks | WS-D1 merges. Until then the case is reported and the compare job records the mismatch. It is not a silent skip. |
| `physics_bodies` | `PhysicsWorld::step` for a seeded fixture, body translations as `to_bits` | WS-D3 merges |
| `gameplay_rng` | harvesting + crafting through the seeded APIs only | WS-D2 merges |

The current jobs do not implement this gate:

- `.github/workflows/sanitizers.yml` job `determinism` runs on `schedule` / `workflow_dispatch` only. Physics is `continue-on-error: true`. The HashMap scan prints imports and exits 0.
- `.github/workflows/integration-tests.yml` job `determinism-cross-platform` runs `cargo test -p astraweave-ecs --test full_pipeline_integration -- determinism` on three OS and uploads `target/test-results/` with `if-no-files-found: ignore`. That test (`test_determinism_3_runs` in `astraweave-ecs/tests/full_pipeline_integration.rs`) discards its seed (`let _ = seed`) and compares a toy sim inside one process at `1e-6`. Each OS succeeds on its own. Nothing diffs the three machines.
- `astraweave-core/tests/full_system_determinism.rs` sorts entities before hashing, then uses `DefaultHasher`. Sorting in the test hasher hides unsorted production iteration. The gate hashes the plan the production path actually emits.
- `astraweave-physics/tests/determinism.rs` is `#![cfg(all(test, feature = "async-physics"))]`. Default `cargo test` does not compile those tests. The assertions allow a `0.0001` length error. `test_async_vs_sync_equivalence` checks that a scheduler handle exists and does not compare states.

---

## 1. What is true today

### 1.1 Scheduling

`Schedule::run` walks stages in vector order and systems in registration order. It does not spawn threads (`astraweave-ecs/src/lib.rs`). `ParallelSchedule` was removed on 2026-04-18. `App::new` registers eight stages: `pre_simulation`, `perception`, `simulation`, `sync`, `ai_planning`, `physics`, `post_simulation`, `presentation`. `App::run_fixed(steps)` runs that schedule `steps` times with no clock.

Release builds still silent-drop a system whose stage name was never registered (`Schedule::add_system`). Debug builds print the miss. That is a correctness hazard for any workstream that adds a system: register the stage, and add a test that the system ran.

### 1.2 Perception and planning

`CompanionState.cooldowns` is a `BTreeMap` (`astraweave-core/src/schema.rs`, `world.rs` `Cooldowns`). `RuleOrchestrator::propose_plan` and the heuristic `GoapOrchestrator::propose_plan` are pure functions of the snapshot they are given. `astraweave-ai/tests/a2_wired_path_determinism.rs` locks that, including full `PlanIntent` bytes. The `llm_orchestrator` feature is off by default (`astraweave-ai/Cargo.toml`).

The wired ECS path does not use that hand-built snapshot. `sys_ai_planning` (`astraweave-ai/src/ecs_ai_plugin.rs`) does:

```text
player = w.all_of_team(0).first()
companion = w.all_of_team(1).first()
enemies = w.enemies_of(1)
snap = build_snapshot(...)
plan = orch.propose_plan(&snap)
```

`World::all_of_team` and `World::enemies_of` iterate `team: HashMap<Entity, Team>` (`astraweave-core/src/world.rs`). `World::entities` iterates `poses: HashMap`. `build_snapshot` copies `obstacles: HashSet` into the snapshot in iteration order (`perception.rs`). `RuleOrchestrator` acts on `enemies.first()`. The same spawns can yield a different first enemy, a different player, and a different obstacle list. The A2 test never calls `enemies_of`, so it stays green while the plugin path is unordered.

`astraweave-behavior::GoapPlanner` stores facts in a `BTreeMap`. A* orders nodes with `f32` `partial_cmp` (`astraweave-ai/src/goap/planner.rs` and the nav heap in `astraweave-nav`). Equal costs have no stable tie-break. `core_loop` behavior-tree dispatch is a TODO that bails (`astraweave-ai/src/core_loop.rs`). `BehaviorPlugin::behavior_tick_system` has no production registration.

`AIArbiter` / `LlmExecutor` spawn tokio tasks and apply a plan when the model returns (`astraweave-ai/src/ai_arbiter.rs`, `llm_executor.rs`). There is no recorded plan log. `astraweave-llm` cache writes use `Instant`. `astraweave-llm/src/llm_adapter.rs` (`safe_llm_invoke`) has no production caller.

`GoapOrchestrator` in `astraweave-ai` is a heuristic (MoveTo / CoverFire). The symbolic planner lives in `astraweave-behavior` and is used by `VeilweaverCompanionOrchestrator` behind the `veilweaver_slice` feature.

### 1.3 Physics, RNG, time, threads

`PhysicsWorld::step` uses `IntegrationParameters.dt` from `PhysicsConfig` (default `1/60`) and Rapier `0.22` with feature `debug-render` only (workspace `Cargo.toml`). No enhanced-determinism / libm flag is enabled. `async-physics` builds a Rayon pool and the in-crate comments state that Rapier may solve islands in parallel. The in-crate `SpatialHash` is not the broadphase `step` uses.

Unseeded sim draws:

| Site | What it does |
|---|---|
| `astraweave-gameplay/src/crafting.rs` `craft` | `rand::random::<u32>()` item id. `craft_seeded` exists beside it. |
| `astraweave-gameplay/src/harvesting.rs` `tick` | deprecated; `rand::random` respawn amount. `tick_seeded` exists beside it. |
| `astraweave-weaving/src/particles/anchor_particle.rs` | `rand::random::<f32>()` for spawn direction and speed |
| `astraweave-terrain/src/solver.rs` `Solver::new` | `world_time_seed` from `SystemTime::now()`. `with_seed` exists and is opt-in. |
| `astraweave-render/src/environment.rs` | `rand::random` for weather particles (presentation, unless a later change copies it into sim state) |

`tools/aw_editor/src/runtime.rs` `tick(dt)` accumulates the caller-supplied `dt` and runs an integer number of 60 Hz steps. Replay has to record that integer. `step_frame` already runs exactly one step. `Instant::now` in that file measures frame cost; it does not advance sim time by itself.

`astraweave-scene/src/streaming.rs` uses `tokio::spawn` for cell loads. Completion order can change which cell is resident. `astraweave-terrain/src/meshing.rs` uses `into_par_iter`. The editor's halo-seed comment says chunk seeds are a function of `(world_seed, chunk_id)` and the drain is sorted. That file is on the Aravath branch (§7); leave it to that branch unless a workstream is explicitly rebasing.

`replay_system` in `astraweave-persistence-ecs/src/lib.rs` increments `current_tick` and does not apply events. `auto_save_system` is an empty TODO. `astraweave-core/src/capture_replay.rs` records time, next id, and obstacles.

### 1.4 Floats and SIMD

Gameplay positions on the AI path are `IVec2` plus `f32` time, morale, and cooldowns. `astraweave-math` SIMD movement is tested and benchmarked inside that crate; production sim crates do not call it. Physics mixes Rapier `Real` and `glam` `f32`. Same-process Rapier runs can match within `1e-4` and still differ in `to_bits` across libm and FMA. WS-D3 owns that gap. Other workstreams keep new sim state on integers or on a single-threaded, ordered `f32` sequence.

---

## 2. Functional completeness

Verdicts:

- **Complete** — the advertised runtime path has a non-example caller and tests that exercise it. Determinism defects are called out in the evidence column; they do not by themselves downgrade a finished feature to a stub.
- **Partial** — real implementation, with a missing production caller, a feature gate, a stub beside the live path, or tests that do not cover the shipped entry point.
- **Stub** — the advertised core is unlinked, example-only, or a TODO body.

Annotation counts are `#[test]` / `#[tokio::test]` in `src/` plus `tests/`, from a static scan at this HEAD. They are not pass counts. Pass counts are in §6. There are **zero** `todo!()` / `unimplemented!()` macros under these crates; the stubs are comments, empty functions, and unlinked files.

### 2.1 Foundation

| Crate | Verdict | Evidence |
|---|---|---|
| `astraweave-ecs` | Complete | Single-threaded `Schedule::run`. ~848 test annotations, 2 benches (`ecs_benchmarks`, `storage_benchmarks`). Miri/Kani history in `docs/current/MIRI_VALIDATION_REPORT.md`. |
| `astraweave-core` | Complete | `World`, `build_snapshot`, `ecs_adapter::build_app` used by the editor runtime and `veilweaver_slice_runtime`. ~974 tests, 2 benches. Determinism hole: `HashMap` entity iteration (§1.2). |
| `astraweave-math` | Partial | SIMD modules are real (~176 tests, 4 benches) and linked transitively. Few direct sim callers; `update_positions_simd` is not on the tick. |
| `astraweave-sdk` | Partial | C ABI exists (~81 tests, 2 benches). Callers are tests and the SDK harness, not a shipped game. |
| `crates/astraweave-alloc` | Partial | `setup_global_allocator!` is used by `tools/aw_editor` and `hello_companion`. **0 tests.** |

### 2.2 AI (depth)

| Crate | Verdict | Evidence |
|---|---|---|
| `astraweave-ai` | Partial | Live path: `AiPlanningPlugin` → `RuleOrchestrator` (`ecs_ai_plugin.rs`). ~1367 tests, 9 benches (`ai_core_loop`, `goap_bench`, `arbiter_bench`, `alloc_measure`, …). `AIArbiter` has no production `::new` outside tests/benches. `llm_orchestrator` default off. BT dispatch TODO. Orphan dirs `src/rag/`, `src/persona/` are not modules. |
| `astraweave-behavior` | Partial | `BehaviorGraph` and `GoapPlanner` are real (~459 tests, benches `goap_planning`, `behavior_tree`). Editor serializes graphs (`tools/aw_editor`). `behavior_tick_system` is only registered from this crate's tests. |
| `astraweave-director` | Partial | `BossDirector` integer Manhattan scoring is live via `astraweave-gameplay/src/weaving.rs` and the veilweaver slice. `LlmDirector` / `DirectorLlmSystem` have no gameplay registration. ~187 tests, 1 bench. |
| `astraweave-dialogue` | Partial | `DialogueGraph` / `DialogueRunner` are live in the slice and the editor. ~300 tests, 1 bench. `llm_dialogue.rs` is not declared in `lib.rs` (file on disk, not in the crate). |
| `astraweave-llm` | Partial | Parse, cache, fallback, and `plan_from_llm` are implemented and tested (~1026 tests, 3 benches, fuzz targets). Engine tick does not call them. `llm_adapter.rs` is a stub module with no callers. Production caller outside examples: `tools/parse_probe`. |
| `astraweave-llm-eval` | Partial | CLI harness (~48 tests, 2 benches). No engine caller. |
| `astraweave-memory` | Partial | `MemoryManager` pipeline is tested (~1022 tests, 1 bench) and consumed as persona types by `astraweave-persona`. No tick records episodes. `persona.rs` sign/save placeholders. 7 `SystemTime::now` sites in `src/`. |
| `astraweave-embeddings` | Partial | `VectorStore::search` is a linear scan. `lib.rs` documents HNSW. `rebuild_index` is a stub (`store.rs`). `current_timestamp` uses `SystemTime`. Callers are the dormant RAG stack. ~220 tests. |
| `astraweave-context` | Partial | History and token counting are real (~299 tests). Callers are RAG / LLM quest / LLM director, which are not on the ECS tick. |
| `astraweave-prompts` | Partial | Template engine is tested (~1375 tests). `save_to_directory` is a stub. Callers are the unwired quest and director LLM stacks. |
| `astraweave-rag` | Partial | `RagPipeline` is implemented (~287 tests, 2 benches). `get_all_memories` returns an empty vec, so consolidation is a no-op. No `astraweave-ai` tick caller. |
| `astraweave-persona` | Partial | Zip load works (~308 tests). No caller outside `examples/persona_loader` and tests. `sign()` is a no-op placeholder. |
| `astraweave-coordination` | Stub | `AgentCoordinator` and related types are tested (~152 tests, 1 bench) and have **zero** callers outside the crate. `Transform` placeholder in `coordination.rs`. |
| `astraweave-npc` | Partial | `NpcRuntime` is a separate town AI (~113 tests). Only `examples/npc_town_demo` constructs it. |
| `astraweave-ipc` | Partial | WebSocket snapshot → `RuleOrchestrator` → plan (~64 tests). External exercise is `examples/ipc_loopback`. |
| `astraweave-optimization` | Partial | Batch inference and prompt cache (~67 tests). Used as a library by dormant LLM paths, not the tick. |
| `astraweave-observability` | Partial | Events are emitted from `astraweave-ai` / `astraweave-llm`. `ObservabilityPlugin` as an engine shell is test/example only. ~132 tests. |

### 2.3 Render, world, gameplay

| Crate | Verdict | Evidence |
|---|---|---|
| `astraweave-render` | Complete | `Renderer` is the editor viewport (`tools/aw_editor/src/viewport/renderer.rs`). ~4859 tests, 19 benches. Open comments: sky target, clustered CPU path. Weather particles use `rand::random` (8 sites in `src/`). |
| `astraweave-materials` | Partial | `MaterialPackage` is used by the renderer (~250 tests). The node graph is mostly tests and benches. |
| `astraweave-camera` | Complete | `CameraProducer` / `RenderView` upload through the editor viewport. ~25 tests. Module docs lag the wiring. |
| `astraweave-scene` | Partial | World partition is used by `veilweaver_slice_runtime`. The editor does not depend on the crate. `compute_poses_stub` is test-only. Streaming spawns tasks (§1.3). ~510 tests, 1 bench. |
| `astraweave-asset` | Complete | `AssetDatabase` in the editor. ~444 tests, 1 bench. GPU-compressed cook is a separate gap (`astraweave-asset-pipeline`). |
| `astraweave-asset-pipeline` | Stub | `lib.rs` states there is no live non-test caller. ~61 tests, 1 bench. Live bake, where it exists, is `tools/aw_asset_cli`. BC7/KTX2 cook remains broken (placeholder encoder); v1.0 authors on raw PNG/glTF per the E4 relabel. |
| `astraweave-water` | Complete | Narrow on purpose: `AnalyticWater` buoyancy inside `PhysicsWorld`. ~9 tests, 1 bench. Surface rendering is `astraweave-render`, not this crate. |
| `astraweave-physics` | Partial | Rapier `PhysicsWorld` is real (~1822 tests, 10 benches) and used by demos and combat. Editor uses it for debug lines, not as the play-mode integrator. `process_destructible_hits` is empty. `SpatialHash` is not wired into `step`. Determinism file is feature-gated (§0). |
| `astraweave-nav` | Partial | `NavMesh::bake` from the editor and from gameplay/scripting. ~497 tests, 1 bench. No mass runtime path in editor play. |
| `astraweave-terrain` | Complete | Editor voxel/chunk/material path. ~2513 tests, 1 bench (`terrain_generation`). `Solver::new` seeds from the wall clock (§1.3). `meshing.rs` is on the Aravath branch. |
| `astraweave-fluids` | Partial | Large tested surface (~738 tests, 2 benches). Callers are examples (`fluids_demo`, `weaving_playground`), not the editor loop. The old SPH core was removed; do not treat `PROJECT_STATUS.md`'s January 2026 fluids grade as current (the status doc says so). |
| `astraweave-pcg` | Stub | WFC / encounters are tests and benches only (~114 tests). No production layout caller. |
| `astraweave-gameplay` | Partial | Combat, crafting, and the in-crate weave bridge are used by the veilweaver slice and examples (~730 tests, bench `combat_pipeline`). Duplicate quest/dialogue surfaces exist beside the dedicated crates. Unseeded `craft` / `tick` sit next to seeded APIs. |
| `astraweave-quests` | Partial | Quest graph types are in the editor. `systems` / `terrain_quests` have no non-test callers. ~227 tests. |
| `astraweave-weaving` | Partial | The crate's level types are example-driven (~796 tests, 2 benches). The slice uses `astraweave-gameplay` weaving. Particle spawn is unseeded (§1.3). |
| `astraweave-cinematics` | Partial | `Timeline` / `Sequencer` used by the slice, the renderer, and the editor panel. ~341 tests. |
| `astraweave-ui` | Partial | HUD/menu implementation is real (~782 tests, 2 benches) and used by UI examples. The editor draws with egui and does not call `draw_ui`. Subtitle/toast/chat TODOs in `hud.rs`. |
| `astraweave-audio` | Partial | `AudioEngine` is constructed from `tools/aw_editor/src/audio_bridge.rs` (~531 tests, 1 bench). CI debug jobs fail device-backed tests (§6). One `rand::random` in `dialogue_runtime.rs` for a temp id. |
| `astraweave-input` | Stub | `InputManager` is implemented and tested (~305 tests, 1 bench). The only constructor outside tests is `examples/ui_controls_demo`. The editor bindings panel does not use this crate. |

### 2.4 Net, persistence, security, modding, tools

| Crate | Verdict | Evidence |
|---|---|---|
| `astraweave-net` | Partial | Snapshot `GameServer` is tested (~369 tests, 1 bench) and used by `examples/coop_*` and `net_headless_sim`. It is a second stack beside `net/aw-net-*`. |
| `astraweave-net-ecs` | Stub | Plugins are tests and benches (~35 tests). `src/lib_temp.rs` is not a module. |
| `net/aw-net-proto` | Complete | HMAC-SHA256 `SigningKey` used by the client and server. ~59 tests, 1 bench. Replay/nonce is intentionally absent (June 2026 net remediation). |
| `net/aw-net-server` | Complete | `run_server` verifies signatures before mutation. Integration tests live beside the binary. |
| `net/aw-net-client` | Partial | Binary client. Server→client frames are unsigned by design (`net/aw-net-client/src/main.rs`). |
| `persistence/aw-save` | Complete | `SaveManager` library (~44 tests, 1 bench), used by `aw_save_cli` and persistence-ecs. **PR #207 is already hardening this crate, the dep audit, and the Windows build. Do not redo that work.** |
| `astraweave-persistence-ecs` | Partial | Save helpers are tested (~135 tests, 3 benches). `auto_save_system` and `replay_system` do not persist or apply events (§1.3). Entity blobs built from a `HashSet` have unstable order. Not called from the editor (the editor has its own save path in `polish.rs`). |
| `crates/astraweave-persistence-player` | Stub | `PlayerProfile` is tests plus `examples/profile_demo`. ~91 tests. |
| `astraweave-security` | Partial | `path::safe_under` is used by the editor and tools. `ScriptSandbox`, anticheat, and `parse_*_limited` are tests only (`docs/architecture/security.md` marks the plugin test-only). ~491 tests. |
| `astraweave-secrets` | Stub | `SecretManager` is the `aw_secrets` binary and tests (~54 tests). No runtime caller. |
| `astraweave-scripting` | Partial | Rhai `ScriptingPlugin` with operation limits (~139 tests, 2 benches). Examples only. `CScript` path load is documented as not implemented. It does not call `astraweave-security::ScriptSandbox`. No `.rhai` hook in `tools/aw_editor/src/file_watcher.rs` (that watcher reloads materials, prefabs, models). |
| `astraweave-author` | Stub | `run_author_script` uses an unsandboxed `Engine::new()` (`astraweave-author/src/lib.rs`). Caller: `examples/rhai_authoring`. ~23 tests. |
| `tools/aw_plugin_template` | Stub | `{{ProjectName}}` scaffold. Not a loadable plugin. |
| `astraweave-steam` | Stub | `SteamIntegration` is tests and benches (~30 tests). No shipped caller. |
| `tools/aw_editor` | Complete | The authoring product: render, terrain, asset, nav, quests, camera, audio bridge, play mode. ~9421 test annotations, bench `editor_performance`. Play mode is fixed-step (§1.3). Several `#[ignore]` GPU proofs. |
| `veilweaver_slice_runtime` | Partial | A composed slice loop with its own FNV determinism module (`src/determinism.rs`, ~829 tests). It is a vertical slice, not the engine tick. |
| `astraweave-profiling` | Complete | Spans in the editor loop. ~41 tests. Zero-cost when the feature is off. |
| `astraweave-stress-test` | Partial | Generators and a stress `App` (~35 tests, 4 benches). `peak_memory_usage_mb` is a TODO constant 0. No product caller. |
| `crates/astract` (+ macro) | Complete | In-engine UI toolkit used by `astraweave-ui` and the gallery. ~170 + 7 tests. |
| `crates/astraweave-blend` | Partial | Blend import used by the editor pipeline. ~550 tests, 3 benches. |
| `tools/aw_asset_cli`, `tools/astraweave-assets`, `tools/asset_signing`, `tools/aw_doc_lint`, `tools/aw_trace_sync`, `tools/aw_headless`, `tools/aw_build` | Complete | Shipped tools with tests (see scan). |
| `tools/aw_demo_builder`, `tools/aw_release`, `tools/aw_save_cli`, `tools/aw_texture_gen`, `tools/ollama_probe` | Partial | Binaries with **0** `#[test]` annotations. |
| `tools/aw_debug` | Partial | Debug helper (~15 tests) with an unseeded event pick. |

Data-driven content that is already live, and that modding work must extend rather than replace: editor RON scenes (`scene_serialization.rs`), RON prefabs, TOML materials, `game.toml` in the build manager.

---

## 3. Performance

Benches exist (114 `benches/*.rs` files in the static scan). Hot ones agents should re-run when they touch the crate, and not replace:

| Area | Benches | Notes from code and prior measurement |
|---|---|---|
| ECS | `astraweave-ecs/benches/ecs_benchmarks.rs`, `storage_benchmarks.rs` | Schedule is single-threaded by design. Parallelism belongs outside the tick. |
| AI | `astraweave-ai/benches/*` (9), `astraweave-behavior/benches/goap_planning.rs` | `alloc_measure` records `ai.goap.plan`. The April 2026 mimalloc experiment measured `ai.goap.plan` about 52% faster and is already merged as an opt-in allocator (`docs/current/PROJECT_STATUS.md`). Re-measure; do not assume the number still matches this HEAD. |
| Physics | 10 benches including `alloc_measure.rs`, `physics_async.rs` | `async-physics` is a determinism risk (§1.3). Do not turn it on to win a bench. |
| Render | 19 benches under `astraweave-render/benches/` | `astraweave-render/src/water.rs` records why the ocean is a chunked LOD plus an 8-quad horizon shell: growing the old grid to the far plane was ~1.56M CPU chunk iterations per frame. `staging_ring.rs` exists to avoid per-frame `create_buffer_init`. |
| Terrain | `astraweave-terrain/benches/terrain_generation.rs` | Generation is seeded per chunk. Mesh `par_iter` is a prepare step; keep the merge ordered. |
| Net / save / scripting | `net_bench`, `proto_bench`, `save_benchmarks`, `script_performance` | Present. |

`astraweave-physics/src/spatial_hash.rs` documents an O(n²) fallback. It is not the live broadphase. Wiring it is a physics workstream decision, and the wired structure must iterate in stable order.

A3 from R.1 (tool-sandbox throughput versus the 60 Hz budget) is still an unmeasured number. WS-PERF measures it. It does not invent a budget in this doc.

---

## 4. Security (do not redo PR #207)

PR **#207** (`cursor/aravath-security`, open) already changes `persistence/aw-save` (manifest and `src/lib.rs`), `deny.toml`, `.cargo/audit.toml`, `.github/workflows/security-audit.yml`, the Aravath Windows manifest/resources, and a wide set of `Cargo.toml` dependency pins. Workstreams in this roadmap do not edit those files.

Remaining engine gaps, owned by the workstreams named:

| Gap | Evidence | Owner |
|---|---|---|
| Gameplay scripts are not on `ScriptSandbox` | `astraweave-scripting` applies Rhai fuel/limits only. `astraweave-author` builds a raw `Engine`. | WS-MOD |
| Limited deserializers have no production caller | `astraweave-security/src/deserialization.rs` `parse_*_limited`. Editor scene load uses `ron::from_str` after `safe_under`. | WS-SEC |
| Input replay window | `aw-net-server` stores `last_input_seq` and does not reject an older seq. Documented boundary from the June 2026 net remediation. | WS-NET |
| Server→client frames unsigned | `net/aw-net-client/src/main.rs` | WS-NET (design stays asymmetric until that workstream explicitly changes it; determinism does not require signatures, it requires an ordered input log) |
| Anticheat plugin | `validate_player_input` / `CAntiCheat` tested only | WS-SEC |
| Key material | `astraweave-security` key generation and `astraweave-secrets` are outside the sim hash | leave them outside the tick |

---

## 5. Moddability

What exists:

- Rhai runtime in `astraweave-scripting` with resource limits, shipped only through examples.
- A second, unsandboxed authoring engine in `astraweave-author`.
- A security sandbox that tests prove and gameplay does not call.
- Editor hot reload for materials, prefabs, and models (`file_watcher.rs`). No script reload.
- Stable-looking data files (RON/TOML) already on the editor path (§2.4).
- No versioned mod ABI. `astraweave-sdk` is a C ABI with test callers. `tools/aw_plugin_template` is a placeholder.

Production modding means one sandboxed script API, load-from-path, reload on file change, and a semver'd command set whose results are sim inputs (so they obey §0). WS-MOD is that work. It extends `astraweave-scripting` and calls `astraweave-security`. It does not create a third script engine.

---

## 6. Stability

### 6.1 CI on this HEAD

`main` at `a3fe3ecd` last ran **Comprehensive CI** on 2026-08-06: [run 31058778151](https://github.com/lazyxeon/AstraWeave/actions/runs/31058778151), conclusion **failure**.

Failed jobs:

- `Test (ubuntu-latest, debug)`
- `Test (windows-latest, debug)`
- `Test (macos-latest, debug)`
- `Code Quality & Security` (step `Run Clippy (warnings as errors)`)

Debug-test failures are device and golden-data, not the AI crates:

- Ubuntu and Windows: `astraweave-audio` `dialogue_runtime` tests (`speak_beep_fallback`, folder scan, TTS fallback, subtitle callback, and the rest of that set) plus a large `mutation_tests` audio block (master volume, listener, spatial sinks, beep durations). Ubuntu logs show ALSA `snd_func_card_inum` errors immediately before the dialogue failures.
- macOS: `caustics::tests::test_sample_multi_point_golden` (`676 passed; 1 failed` in that binary).

Release test jobs on the same run succeeded, including `Test (ubuntu-latest, release)` and the Windows/macOS release legs. Quick check, MSRV, benchmarks, coverage, and demo validation succeeded on that run.

Later scheduled workflows on this same unmoving `main` succeeded (Integration Tests, Sanitizers, Mutation Testing on 2026-10-11 through 2026-10-13). Those workflows are not the full debug matrix. The determinism job inside Sanitizers does not run on a plain push (§0).

### 6.2 Panics and unwraps

Workspace policy forbids `.unwrap()` on production fallible I/O and GPU calls (`CLAUDE.md`). `build_snapshot` uses `expect` on missing player/companion components (`astraweave-core/src/perception.rs`): a missing component aborts the tick. That is a stated precondition, and it is still a panic on the AI path. WS-AI-1 replaces those with a recorded planning-failure event (the plugin already has `AiPlanningFailedEvent`) so a bad entity cannot abort the process.

`Schedule::add_system` silent-drop in release is the other structural panic-adjacent bug: the system vanishes and the tick continues. WS-STAB makes a missing stage a `Result` in all builds.

### 6.3 Local test run (this audit)

Command (default features, `--test-threads=8`):

```text
cargo test -p astraweave-ecs -p astraweave-core -p astraweave-ai \
  -p astraweave-behavior -p astraweave-director -p astraweave-llm \
  -p astraweave-physics -p veilweaver_slice_runtime
```

Log: `/tmp/engine-audit/suite.log` on the audit machine. Results are filled in under the marker below when the process exits. The physics determinism integration test is cfg-gated off in this command; a `0 tests` result for `--test determinism` is the expected evidence, not a pass of the physics gate.

<!-- LOCAL_TEST_RESULTS -->

Run on this machine after `g++` provided `libstdc++` (the first link of `veilweaver_slice_runtime` tests failed with `cannot find -lstdc++`). Command, default features, `--no-fail-fast`, `--test-threads=8`:

```text
cargo test -p astraweave-ecs -p astraweave-core -p astraweave-ai \
  -p astraweave-behavior -p astraweave-director -p astraweave-llm \
  -p astraweave-physics -p veilweaver_slice_runtime --no-fail-fast
```

**Result: exit 101.** 121 test binaries. **6676 passed, 14 failed, 41 ignored.** About 23s of test runtime after compile. One failing target: `-p astraweave-ai --test ecs_integration_tests` (12 passed, 14 failed).

| Crate lib (unit tests) | Passed | Failed | Ignored |
|---|---:|---:|---:|
| `astraweave-physics` | 1193 | 0 | 0 |
| `astraweave-llm` | 618 | 0 | 0 |
| `astraweave-core` | 516 | 0 | 4 |
| `astraweave-ecs` | 448 | 0 | 0 |
| `veilweaver_slice_runtime` | 314 | 0 | 0 |
| `astraweave-ai` | 303 | 0 | 0 |
| `astraweave-behavior` | 234 | 0 | 0 |
| `astraweave-director` | 92 | 0 | 0 |

Determinism-named binaries in this run:

| Binary | Result |
|---|---|
| `astraweave-ai/tests/a2_wired_path_determinism.rs` | 1 passed |
| `astraweave-ai/tests/determinism_tests.rs` | 4 passed, 1 ignored |
| `astraweave-core/tests/full_system_determinism.rs` | 7 passed |
| `astraweave-ecs/tests/full_pipeline_integration.rs` | 6 passed (includes `test_determinism_3_runs`) |
| `astraweave-physics/tests/determinism.rs` | **0 tests** (file is `cfg(feature = "async-physics")`) |

`astraweave-ai` integration binaries that compiled empty under default features (the tests sit behind `llm_orchestrator` or another non-default feature): `ai_arbiter_implementation_tests`, `arbiter_comprehensive_tests`, `arbiter_tests`, `async_task_comprehensive_tests`, `llm_fallback`, `rag_integration_test`, `goap_hierarchical_planning`, `goap_learning_integration`, `goap_vs_rule_comparison`, `zero_alloc_tests`. A green default `cargo test` does not exercise the arbiter.

The 14 failures are all in `astraweave-ai/tests/ecs_integration_tests.rs`. They assert that every companion receives a `CDesiredPos` or that `AiPlannedEvent` count equals the companion count. Examples: `test_multi_agent_100_agents` got **72 of 100** plans; `test_multi_agent_determinism` panics on `CDesiredPos` being `None` before it compares the two runs; `test_event_AiPlannedEvent_published` expected 1 event and observed 0. `sys_ai_planning` writes `CDesiredPos` only when the plan contains `MoveTo`, and the legacy branch returns after a single companion (`astraweave-ai/src/ecs_ai_plugin.rs`, comment "single companion minimal"). WS-AI-1 owns making this binary green. The A2 purity test staying green at the same time is the gap §1.2 describes: planner purity on a hand-built snapshot, integration path red.

This run is the AI, foundation, physics, and veilweaver slice suites. It is not `cargo test --workspace`. Render, terrain, editor, and audio were not re-executed here; §6.1 is the CI evidence for those.
<!-- /LOCAL_TEST_RESULTS -->

---

## 7. Conflict map

Demo branch `cursor/aravath-greybox-b3de` (merge-base `a3fe3ecd`) touches engine files:

- `astraweave-physics/src/lib.rs`
- `astraweave-render/src/renderer.rs`, `god_rays.rs`, `volumetric_fog.rs`
- `astraweave-render/shaders/volumetrics/composite_shafts.wgsl`, `scatter.wgsl`
- `astraweave-camera/src/lib.rs`, `third_person.rs`
- `astraweave-terrain/src/meshing.rs`
- `Cargo.toml`, `Cargo.lock`
- `.github/workflows/aravath-windows.yml`
- plus `aravath_runtime/**` and `assets/aravath/**` (demo-owned; engine workstreams stay out)

PR #207 additionally pins many `Cargo.toml` files (including `astraweave-ecs`, `astraweave-physics`, `astraweave-persistence-ecs`, `astraweave-memory`, `astraweave-embeddings`, `net/aw-net-*`, `persistence/aw-save`) and edits `persistence/aw-save/src/lib.rs`, `deny.toml`, and security workflows.

A workstream that must touch a file on that list rebases onto the demo or onto #207 after those land. It does not edit the file "meanwhile" and hope the merge is textual.

---

## 8. Workstreams

Each stream is independently assignable. Shared-crate notes say who owns which file so two agents do not edit it.

Every stream's acceptance list includes the §0 gate for the sim state it writes. A stream that only adds a required hash case turns that case from "reported" to "required" in `determinism-replay.yml`.

### Phase A — make the tick a pure function

#### WS-D0 — Replay-hash CI

- **Scope.** Add `astraweave-core/tests/replay_hash.rs` and `.github/workflows/determinism-replay.yml` as specified in §0. Implement the canonical encoder once, in the test crate (promote to a library function only if a second crate needs it). First required case: `planner_pure`. Emit `perception_world` and print both digests; compare-job failure on `perception_world` stays non-blocking until WS-D1, and the job summary says so in one line.
- **Crates.** `astraweave-core` (new test only), `.github/workflows/determinism-replay.yml` (new file).
- **Acceptance.** Three OS artifacts exist. `planner_pure` matches across them. The workflow is required on pull requests. Encoder uses SHA-256 of the canonical layout in §0. No use of `DefaultHasher`.
- **Determinism gate.** This stream *is* the gate. It adds no sim behavior.
- **Conflict.** Low with the demo (new files). Low with #207 if this stream does not edit `security-audit.yml`, `sdk-ci.yml`, or `aravath-windows.yml`.
- **Owns.** `astraweave-core/tests/replay_hash.rs`, the new workflow. Does not edit `world.rs`.

#### WS-D1 — Ordered perception

- **Scope.** `all_of_team`, `enemies_of`, `entities`, and obstacle snapshots return stable order (sort by entity id or coordinate, or store those indexes in a `BTreeMap`). `build_snapshot` and `obstacles_as_ivec2` use that order. `sys_ai_planning` stays the caller; do not sort only inside a test. Extend coverage so a two-process run of `perception_world` matches. Switch `determinism-replay.yml` so `perception_world` is required.
- **Crates.** `astraweave-core` (`world.rs`, `perception.rs`), `astraweave-ai` (`ecs_ai_plugin.rs` only if the call site needs a comment or a sort assertion).
- **Acceptance.** Two worlds filled with the same `spawn` sequence produce the same enemy id list, the same `enemies.first()` plan, and the same SHA-256. Existing A2 test still passes. No new `HashMap` walk on the snapshot path.
- **Conflict.** Low with the demo (those files are not on the branch). Low with #207 if `astraweave-ecs/Cargo.toml` and `astraweave-ai` deps are untouched.
- **Owns.** `world.rs`, `perception.rs`. Coordinate with WS-D0 so only WS-D0 edits `replay_hash.rs` until D1 flips the case to required (D1 may add the expectation in that file in a follow-up commit on the same branch after D0's file exists; if both run at once, D1 puts the new assertion in `astraweave-core/tests/perception_order.rs` and D0's compare job learns the case name).

#### WS-D2 — Seeded RNG and clocks

- **Scope.** Gameplay sim entry points call `craft_seeded` / `tick_seeded` only. Remove the unseeded bodies or make them unreachable from the tick (a deprecated wrapper that still draws is a failing gate). Weaving anchor particles take a `SimRng`. `Solver::new` requires a `u64` seed; delete the `SystemTime` default. Editor play mode records the fixed step count into the play session so replay can feed it. Grep gate: `rand::random` under `astraweave-gameplay/src`, `astraweave-weaving/src`, `astraweave-terrain/src/solver.rs` is zero.
- **Crates.** `astraweave-gameplay`, `astraweave-weaving`, `astraweave-terrain/src/solver.rs`, `tools/aw_editor/src/runtime.rs`.
- **Acceptance.** `gameplay_rng` case matches across OS once added. Same seed and same step count reproduce harvest amounts, item ids, particle spawn state, and solver output. `meshing.rs` is not modified.
- **Conflict.** Low with the demo **only while `meshing.rs` is untouched**. `solver.rs` is not on the demo branch. Low with #207 (no `Cargo.toml` edits).
- **Owns.** The files in Scope. Terrain agents on the demo own `meshing.rs`.

#### WS-D3 — Physics bit-stability

- **Scope.** Keep the default `step` single-threaded. `async-physics` stays off the deterministic profile; document that in the crate root. Move determinism coverage out of the `async-physics` cfg so default `cargo test -p astraweave-physics --test determinism` runs the same-seed comparison. Assert `to_bits` equality, not a 0.0001 epsilon. Either implement a real sync-vs-async comparison or keep async from being compared (the stub test must not claim equivalence). Investigate Rapier 0.22's determinism/libm switch; enabling it is in scope if the flag exists on this pin. If it does not exist, the stream reports that and the `physics_bodies` case stays unrequired until a Rapier bump is its own reviewed change. Sort any projectile/destructible maps that affect spawn or hit order.
- **Crates.** `astraweave-physics`.
- **Acceptance.** Two default-feature worlds, same seed, same step count, identical translation bit patterns in-process. Cross-OS `physics_bodies` required only after the bit pattern matches on the three runners. `continue-on-error` for physics determinism is removed from `sanitizers.yml` in that same change.
- **Conflict.** **High** with the demo: `astraweave-physics/src/lib.rs` is already modified on `cursor/aravath-greybox-b3de`. Rebase onto that branch or wait. **Medium** with #207 (`astraweave-physics/Cargo.toml` pin). Do not edit the Cargo.toml pin in this stream.
- **Owns.** `astraweave-physics/src/**`, `astraweave-physics/tests/determinism.rs`.

#### WS-D4 — LLM outside the tick

- **Scope.** Define a `PlanLog` (tick → `PlanIntent` bytes) applied by the AI system when `llm_orchestrator` is on. `AIArbiter` does not read a finished plan until the log has the bytes. Tests of the deterministic profile use the log, not a live model. Cache timestamps stay out of plan bytes. `DirectorLlmSystem` follows the same rule. Leave `llm_orchestrator` default off.
- **Crates.** `astraweave-ai` (`ai_arbiter.rs`, `llm_executor.rs`), `astraweave-llm` (cache metadata only), `astraweave-director` (`llm_director.rs`, `systems.rs`).
- **Acceptance.** A replay test feeds a recorded plan and produces the same world hash with the network stack disabled. A test that the deterministic profile issues no HTTP. `RuleOrchestrator` behavior unchanged (WS-D1 owns perception order).
- **Conflict.** Low with the demo and with #207 (do not retune example `Cargo.toml` pins).
- **Owns.** The files in Scope. Does not own `ecs_ai_plugin.rs` (WS-D1) or `orchestrator.rs` heuristic rules (WS-AI-1). If the log must be consulted from `sys_ai_planning`, WS-D4 adds a single function call and WS-D1 reviews it.

### Phase B — finish sim features on the pure tick

#### WS-AI-1 — One deterministic planner on the tick

- **Scope.** The name `GoapOrchestrator` in `astraweave-ai` is a heuristic. Either route the wired path through `astraweave-behavior::GoapPlanner` or rename the heuristic so the two are not confused. A* tie-break is `(f_cost, action_id)` with integer costs, or a documented total order on the existing `f32` that does not use `partial_cmp`'s `Equal` for NaN. Register `behavior_tick_system` in the runtime that claims behavior trees, or delete the claim. `core_loop` BT dispatch either runs that system or returns a typed error (the TODO bail goes away). `build_snapshot`'s `expect` on missing components becomes `AiPlanningFailedEvent`.
- **Crates.** `astraweave-ai` (`orchestrator.rs`, `core_loop.rs`, `goap/`), `astraweave-behavior` (`goap.rs`, `ecs.rs`).
- **Acceptance.** Same snapshot in, same `PlanIntent` out, including tied costs. Behavior-tree tick runs inside `App::run_fixed` in at least one non-example runtime (editor play or the veilweaver slice) or the public registration API is removed. §0 hash covers the plan bytes. `cargo test -p astraweave-ai --test ecs_integration_tests` is green, including `test_multi_agent_100_agents` and `test_multi_agent_determinism`. The early return marked "single companion minimal" in `sys_ai_planning` is gone, and a plan that is not `MoveTo` still records a deterministic outcome (desired position or a failure event) for every companion.
- **Conflict.** Low with the demo. Shares `astraweave-ai` with WS-D4; file split in WS-D4 applies.
- **Owns.** `orchestrator.rs`, `core_loop.rs`, `astraweave-behavior/src/goap.rs`, `astraweave-behavior/src/ecs.rs`.

#### WS-AI-2 — Dormant AI: wire one caller or correct the claim

- **Scope.** For each of `astraweave-memory`, `astraweave-rag`, `astraweave-embeddings`, `astraweave-context`, `astraweave-prompts`, `astraweave-persona`, `astraweave-coordination`, `astraweave-npc`, and the unlinked `llm_dialogue.rs`: either add one non-example caller on a deterministic boundary (recorded output, no `SystemTime` in scores) **or** change the crate-level docs/trace so they describe an in-design library. Do not build a second memory or RAG pipeline. HNSW: implement it or stop claiming it in `astraweave-embeddings/src/lib.rs`. `get_all_memories` returning empty is a bug if consolidation stays advertised.
- **Crates.** The list in Scope. Traces: `docs/architecture/ai_pipeline.md` (and the child trace each crate owns).
- **Acceptance.** `rg` shows a production caller or the trace `integration_status` says the crate is not on the tick. Any new sim read is covered by §0. `SystemTime::now` is gone from scoring functions that feed a plan.
- **Conflict.** Low with the demo. Medium with #207 if `astraweave-memory/Cargo.toml` or `astraweave-embeddings/Cargo.toml` change. This stream does not edit those manifests.
- **Owns.** Those crates' `src/` and their trace sections. Does not own `astraweave-ai/src/ecs_ai_plugin.rs`.

#### WS-IN — One input path

- **Scope.** Editor bindings and gameplay read `astraweave-input`. Delete the parallel binding logic or make it a thin UI over that crate (`tools/aw_editor` input bindings panel). Input applied on a tick comes from a recorded frame log (device id, action, tick). Polling `Instant` to decide a sim action is out.
- **Crates.** `astraweave-input`, `astraweave-gameplay`, `tools/aw_editor` (bindings panel only).
- **Acceptance.** At least one non-example caller of `InputManager` / `BindingSet`. Replay of a recorded frame log is bit-stable under §0. The duplicate panel is gone or delegates.
- **Conflict.** Low. The editor is not on the demo branch. Avoid `astraweave-camera` and `renderer.rs`.

#### WS-SAVE — Replay that applies events

- **Scope.** `replay_system` applies recorded events in tick order. `auto_save_system` calls the existing `aw-save` API on an interval counted in ticks, not wall time. Entity serialization sorts ids before writing. Editor play can round-trip a save through this path or the stream documents which of the two save stacks is canonical and deletes the other from the play path.
- **Crates.** `astraweave-persistence-ecs`, `docs/architecture/persistence_ecs.md`.
- **Acceptance.** Record N ticks, reload, hash matches `perception_world` / the gameplay case. `replay_system` no longer advances a counter without applying events. **No edits** to `persistence/aw-save/src/lib.rs` or its manifest (PR #207).
- **Conflict.** Medium with #207 because that PR edits `docs/architecture/persistence_ecs.md` and `astraweave-persistence-ecs/Cargo.toml`. This stream edits `src/lib.rs` and rebases the trace. Low with the demo.

#### WS-MOD — Sandboxed mod API

- **Scope.** `astraweave-scripting` executes through `astraweave-security::ScriptSandbox` (or the sandbox moves behind a function the scripting crate already calls — one implementation). `astraweave-author` uses that sandbox. `CScript` loads from a path. Editor file watcher reloads `.rhai` the same way it reloads prefabs. Public commands live in one module with a version constant. Script results that move entities are sim inputs on a tick boundary.
- **Crates.** `astraweave-scripting`, `astraweave-author`, `astraweave-security` (call the existing sandbox; do not fork it), `tools/aw_editor/src/file_watcher.rs`.
- **Acceptance.** A script cannot touch the filesystem outside the mod root (existing sandbox tests stay green, plus one editor-path test). Reload picks up a changed file without restarting the process. Two runs with the same script bytes and seed match §0. `tools/aw_plugin_template` either generates a crate that typechecks against this API or is labeled a non-runtime scaffold in its README.
- **Conflict.** Low with the demo and with #207 (do not edit `deny.toml` or `security-audit.yml`).

#### WS-WORLD — Play mode runs the sim

- **Scope.** Editor play (already fixed-step) registers the same gameplay, nav, and AI systems the slice uses, in the canonical stage order. Physics `step` runs in the physics stage at `1/60` (depends on WS-D3 for the hash, and can land against the current single-threaded step before bit-stability). This is the R.1 "play-in-editor" seam, narrowed so it does not reopen camera or renderer files.
- **Crates.** `tools/aw_editor/src/runtime.rs` (coordinate with WS-D2, which owns the step-count recorder), `astraweave-gameplay`, `astraweave-nav`.
- **Acceptance.** Play mode for a fixture scene advances companion plans, nav, and physics for 100 steps with a recorded step count and a stable hash. Systems are not silent-dropped (assert they ran).
- **Conflict.** Low with the demo if `runtime.rs` is the only editor file and physics changes stay in WS-D3. WS-D2 and WS-WORLD both touch `runtime.rs`: WS-D2 owns the step recorder, WS-WORLD owns system registration. Split the file by function.

### Phase C — capability beside the tick

#### WS-NET — Ordered inputs, replay window

- **Scope.** Reject input frames with `seq` older than or equal to `last_input_seq` before any state write (the field is already stored). Client sim consumes a contiguous seq log. Arrival time does not change the resulting tick. Signature work from the June 2026 remediation stays. Server→client signing is an explicit product choice inside this stream; leaving it unsigned is allowed if the trace says so in one paragraph.
- **Crates.** `net/aw-net-server`, `net/aw-net-client`, `docs/architecture/net_ecs.md`.
- **Acceptance.** A replayed input log produces the same sim hash. A reordered UDP/WebSocket delivery test (deterministic fake clock) still matches. No edits to HMAC primitives in `aw-net-proto` unless a test proves a bug.
- **Conflict.** Medium with #207 (`net/aw-net-*/Cargo.toml`). Do not edit those manifests. Low with the demo.

#### WS-SEC — Bounds and anticheat on a real caller

- **Scope.** Editor (or the save path) uses `parse_*_limited` for RON/TOML scene loads, size-capped, after the existing `safe_under` check. One gameplay caller runs `validate_player_input` on the recorded input log from WS-IN. No new crypto stack.
- **Crates.** `astraweave-security`, the editor scene loader (`scene_serialization.rs` only).
- **Acceptance.** An oversized fixture is rejected with a `Result`. A forged input fails validation and does not move the player. §0 holds for the accepted log.
- **Conflict.** Low with the demo. Out of scope: `deny.toml`, `.cargo/audit.toml`, Windows manifest, `aw-save` (PR #207).

#### WS-ASSET — Cook path with a caller

- **Scope.** Give `astraweave-asset-pipeline` one non-test caller (`tools/aw_asset_cli` or the editor import path) for mesh optimization and a real texture encode, or keep the crate dormant and stop any doc that says the cook path is live. The broken BC7/KTX2 encoder is in scope. GPU upload of compressed textures is **out of scope** because it requires `astraweave-render/src/renderer.rs`.
- **Crates.** `astraweave-asset-pipeline`, `tools/aw_asset_cli`.
- **Acceptance.** A fixture file cooks to a deterministic byte blob (content hash) with no wall-clock in the bytes. A production caller invokes it. Renderer files untouched.
- **Conflict.** Low if those two crates are the whole diff. **High** if the stream starts editing `renderer.rs`, volumetric shaders, or `meshing.rs`.

#### WS-PRESENT — Presentation stays off the hash

- **Scope.** Confirm weather particles, water shading, and audio device clocks do not write sim components. Seed presentation particles from `(world_seed, tick)` if they are compared in golden images; leave them unhashed. Fix or cfg-gate the audio tests that require ALSA/WASAPI so the debug CI job can go green without a sound card (WS-STAB can take the test harness; this stream takes the engine boundary comment and any sim write it finds).
- **Crates.** `astraweave-render/src/environment.rs`, `astraweave-audio` (test cfg only if WS-STAB agrees), `astraweave-render/src/water.rs` read-only unless a sim write is found.
- **Acceptance.** A written list, in the render or audio trace, of clocks and RNGs that are presentation-only. Grep shows they are not read by `astraweave-gameplay` or `astraweave-ai`.
- **Conflict.** **High** if editing `renderer.rs`, `god_rays.rs`, `volumetric_fog.rs`, or the volumetric shaders (demo branch). `environment.rs` and `water.rs` are not on that list today; stay inside them.

### Phase D — after the hashes are stable

#### WS-PERF — Measure, then speed up the pure tick

- **Scope.** Re-run the existing benches for any crate Phase A–C changed. Measure tool-sandbox validation at 1, 100, and 1000 agents against a 16.67 ms tick (R.1 A3) and record the number in `docs/current/MASTER_BENCHMARK_REPORT.md`. Optimize only paths whose hash stayed identical. Forbidden: enabling `async-physics` or `par_iter` inside `Schedule::run` to improve a number.
- **Crates.** Whichever Phase A–C touched, plus `astraweave-ai` benches and `astraweave-render` water CPU path **only if** WS-PRESENT proved it is off the sim hash and the demo branch has merged (otherwise skip `water.rs` when the demo still owns neighboring renderer files).
- **Acceptance.** Bench numbers checked in with the command line. A replay hash from before the optimization matches after, for every required case.
- **Conflict.** Inherits the file conflicts of the crate it optimizes. Do not start this stream before WS-D0's `planner_pure` case is required and green.

#### WS-STAB — Debug CI and release-silent failures

- **Scope.** Make `Test (*-latest, debug)` green on the three OS: audio device tests skip or use a null backend when no device exists; macOS caustics golden is updated from a recorded baseline or the test stops depending on a platform-specific libm bit. `cargo clippy -p <crate> --all-features -- -D warnings` is green for crates this program touched. `Schedule::add_system` returns a `Result` in release as well as debug (call sites in the editor and the slice updated).
- **Crates.** `astraweave-audio` tests, the caustics test module (fluids or render — confirm with the failure binary name before editing), `astraweave-ecs` `Schedule::add_system`, call sites.
- **Acceptance.** A re-run of the failing CI jobs is green, or a link to the run is in the PR. New sim logic still passes §0. Clippy fixes do not drive-by `renderer.rs`, `physics/src/lib.rs`, `camera`, or `terrain/src/meshing.rs`.
- **Conflict.** Low if the audio tests and the caustics fixture are the diff. ECS schedule signature is a wide but mechanical call-site update; keep it separate from WS-D1's `world.rs` edit.

---

## 9. Parallel assignment

Safe to run at the same time, because they do not share source files:

| Agent | Stream | Stay out of |
|---|---|---|
| 1 | WS-D0 | `world.rs` |
| 2 | WS-D1 | `replay_hash.rs` until the handoff in WS-D1 |
| 3 | WS-D2 | `meshing.rs`, `runtime.rs` system-registration functions |
| 4 | WS-D3 | demo physics file until rebase; any `Cargo.toml` |
| 5 | WS-D4 | `ecs_ai_plugin.rs`, `orchestrator.rs` |
| 6 | WS-AI-1 | `ai_arbiter.rs`, `llm_executor.rs` |
| 7 | WS-AI-2 | `astraweave-ai` plugin/orchestrator; memory/embeddings manifests |
| 8 | WS-MOD | `deny.toml`, security workflow |
| 9 | WS-IN | camera, renderer |
| 10 | WS-SAVE | `aw-save` sources and manifest |
| 11 | WS-NET | `aw-net-*/Cargo.toml`, HMAC primitives |
| 12 | WS-SEC | scene loader only; not WS-MOD's sandbox body (call it) |
| 13 | WS-ASSET | `renderer.rs` and shaders |
| 14 | WS-STAB | renderer, physics lib, camera, terrain meshing |

Sequence constraints (still parallel until the join):

1. WS-D0 lands the workflow before any stream marks its case required.
2. WS-D1, WS-D2, WS-D3 flip their own cases to required as their last commit.
3. WS-WORLD waits for WS-D2's step recorder in `runtime.rs`, or edits only registration functions.
4. WS-PERF waits for the required cases it could disturb.
5. WS-AI-1 and WS-D4 can proceed together under the file split. WS-D1 can proceed with them if it stays in `world.rs` / `perception.rs`.

---

## 10. Out of scope for all of the above

- Re-implementing PR #207 (aw-save hardening, `deny.toml` / cargo-audit, Aravath Windows manifest and signing scripts).
- Editing `aravath_runtime/**` or `assets/aravath/**`.
- A second renderer, a second nav crate, a second script engine, or a second save format.
- Turning on `llm_orchestrator` by default.
- Declaring multiplayer gameplay done. WS-NET makes the input log ordered. A full game server is a later program that consumes this gate.
- Treating `docs/src/` wiki pages as evidence. Architecture traces under `docs/architecture/` are the docs to update when a stream changes a traced crate.

---

## Revision history

| Version | Date | Notes |
|---|---|---|
| 0.1 | 2026-10-14 | Whole-engine audit at `a3fe3ecd`. Determinism gate and replay-hash CI specified. Local suite: 6676 passed, 14 failed (`astraweave-ai` ecs integration), 41 ignored. Physics `tests/determinism.rs` ran 0 tests on default features. |
