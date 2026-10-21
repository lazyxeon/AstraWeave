# Job System & Task Graph Audit — 2026-04-18

**Scope**: Forensic audit of how AstraWeave parallelizes CPU work and structures dependencies between tasks within a frame. Covers 55 workspace crates.
**Method**: Static analysis via `rg` and direct file reads. No runtime tracing.
**Status**: Discovery + analysis complete; Phase 3 recommendations are proposed, not applied.
**Related prior audits**: `docs/audits/allocation_audit_2026-04-17.md`, `docs/audits/allocation_measurement_plan_2026-04-17.md`, `docs/audits/mimalloc_experiment_2026-04-17.md`. The ECS `ParallelSchedule::build_groups` allocation finding in those audits (§2.3 #6) is **reclassified** here — see §2.4.

> **Partially superseded 2026-04-18**: §1.2 paragraph 3 claims "consumers must `add_stage` them manually if used" for PRE_SIMULATION / POST_SIMULATION. As of `docs/audits/schedule_stage_fix_2026-04-18.md`, `App::new()` now creates all 8 canonical stages (`pre_simulation` → `perception` → `simulation` → `sync` → `ai_planning` → `physics` → `post_simulation` → `presentation`) automatically. The rest of this audit is unaffected.

---

## Executive Summary

AstraWeave has **two** ECS schedulers that coexist — a sequential `Schedule` (`astraweave-ecs/src/lib.rs:690-722`) and a rayon-scope-based `ParallelSchedule` (`astraweave-ecs/src/parallel.rs:186-326`). The sequential one is installed by `App::new()` (`lib.rs:756-768`) and is what every example and tool in the workspace actually runs. The parallel one is **not instantiated by any production binary** — only its own unit tests (`parallel.rs:476`) and one bench (`astraweave-ecs/benches/alloc_measure.rs:75`) construct it. The engine therefore ships a deterministic parallel scheduler it does not use.

Outside the ECS, parallelism is scattered and feature-gated: rayon `par_iter` appears in fluids SPH (`astraweave-fluids/src/simd_ops.rs:694-2036`, gated `parallel`), terrain chunk meshing (`astraweave-terrain/src/meshing.rs:478`, unconditional), and physics helpers that are defined but never called (`astraweave-physics/src/async_scheduler.rs:315-332`, gated `async-physics`). Tokio is used for async asset I/O, LLM planning, and network server tasks; the workspace pulls tokio with `features = ["full"]` (`Cargo.toml:178`) and several call sites create a fresh `Runtime::new()` per bridge point instead of sharing one. Rendering is entirely CPU-serial — one command encoder, one `queue.submit` per frame (`astraweave-render/src/renderer.rs:4677-5173`).

A full DAG-based render graph exists (`astraweave-render/src/graph.rs:388-744`, `astraweave-render/src/frame_graph.rs:433-620`) with Kahn topological sort, transient resource aliasing, and a ten-pass default topology. **It is never executed in production** — `build_default_graph` has zero non-test callers, and `run_graph_on_renderer` (`astraweave-render/src/graph_adapter.rs:4-19`) is declared but uncalled. The main `Renderer::render()` function does not import the graph module.

Net effect: the engine advertises AI-native, deterministic, parallel scheduling but executes nearly every per-frame system single-threaded through the sequential scheduler, with the exception of the terrain chunk mesh path (which lives off the frame critical path) and GPU command submission (which wgpu parallelizes internally). The single largest opportunity is not "write a new scheduler" — it is "use the one already in the crate."

---

## Phase 1 — Discovery

### 1.1 Thread pools and worker threads

**Rayon thread pools**

- **Global pool** — used implicitly by every `par_iter`/`rayon::scope` call. Worker count defaults to `num_cpus::get()`. The only site that explicitly configures it is `astraweave-physics/src/lib.rs:1011-1018` (`enable_async_physics`), which calls `rayon::ThreadPoolBuilder::new().num_threads(n).build_global()` and discards the return value (`let _ = ...`). `build_global()` returns `Err` if called after the pool is already initialised; because the error is discarded, a second call silently fails. **This function is called from exactly one place: `astraweave-physics/tests/determinism.rs:190`.** No example, tool, or production binary calls it.
- **Named per-pool**: none. There is no `rayon::ThreadPool` field in any struct, no `ThreadPoolBuilder::build()` call (only `build_global()`). All rayon work runs in the implicit default pool.

**`std::thread::spawn` / `std::thread::Builder`**

- Editor background workers: `tools/aw_editor/src/main.rs:6622` spawns a decomposition worker thread; `tools/aw_editor/src/file_watcher.rs:200,253` uses a shutdown flag on a dedicated watch thread; `tools/aw_editor/src/panels/build_manager.rs:313,337` owns a cancellable build thread via `Arc<AtomicBool>`. Per-subsystem lifetime.
- GPU readback helpers in tests (`astraweave-render/tests/*`) — not production.
- No `std::thread::Builder::name(...)` is used — threads are unnamed.

**Tokio runtimes**

- Workspace dep: `tokio = { version = "1", features = ["full"] }` at `Cargo.toml:178`. `full` pulls `rt-multi-thread` (the default flavor).
- `#[tokio::main]` entry points: 13 discovered, including `net/aw-net-server/src/main.rs:98`, `net/aw-net-client/src/main.rs:8`, `examples/world_partition_demo/src/main.rs:138`, `examples/phi3_demo/src/main.rs:25`, `examples/scripting_advanced_demo/src/main.rs:9`, `examples/nanite_demo/src/main.rs:192`, and five more. Each instance gets its own multi-thread runtime.
- Ad-hoc `Runtime::new()` at sync→async bridges — multiple per process possible:
  - `astraweave-asset/src/lib.rs:3666-3669`
  - `tools/aw_editor/src/main.rs:6622-6661` (runtime created inside a `std::thread::spawn` worker, then `rt.block_on` + inner `tokio::spawn`)
  - `tools/aw_editor/src/viewport/renderer.rs:413,1026,1068,1371` — four separate `.block_on()` sites
  - `examples/hello_companion/src/main.rs:920,942` — two per-function runtimes
- Current-thread variant: `astraweave-ai/src/orchestrator.rs:540` uses `Builder::new_current_thread()` for LLM warmup.
- **Multiple runtimes active simultaneously is possible** inside `aw_editor` when a background thread's runtime runs alongside the editor's main runtime. Not verified at runtime but structurally plausible.

**`pollster::block_on`** (synchronous driver for wgpu async APIs)

- Used at every wgpu init site: `astraweave-fluids/src/gpu_volume.rs:545,551`, renderer construction in examples (e.g. `examples/unified_showcase/src/main.rs:2442`), editor viewport. One-shot, not frame-scoped.

**`futures::executor::block_on` / `futures_executor::block_on`**

- `astraweave-ai/src/orchestrator.rs:565,631,1086,1192,1266,1365-1367` — used to drive async planner calls from sync code.
- `astraweave-ai/src/veilweaver.rs:748` — 100 ms-budgeted plan call.
- `astraweave-ai/src/llm_executor.rs:221` — sync variant of `spawn_plan` (used in tests).

**Number of independently-sized pools that could be active simultaneously** in a running editor: rayon global (one, sized to `num_cpus`) + primary tokio runtime (one, sized to `num_cpus`) + ad-hoc `Runtime::new()` inside editor workers (one per bridge, each sized to `num_cpus`). The three-or-more-pool scenario is real in the editor.

**Cross-thread handoff (channels)**

- `std::sync::mpsc`: 15+ production sites. Notable production (non-test) uses:
  - `astraweave-asset/src/lib.rs:3561` — asset load sync channel
  - `astraweave-render/src/terrain_gpu_bridge.rs:49,170` — terrain→GPU
  - `astraweave-render/src/impostor_bake.rs:602`
  - `astraweave-render/src/renderer.rs:4485` — device callback
  - `astraweave-render/src/clustered.rs:540` — `sync_channel(1)` for CPU light bin readback
  - `tools/aw_editor/src/panels/world_panel.rs:860`, `tools/aw_editor/src/panels/terrain_panel.rs:2036-2037`, `tools/aw_editor/src/panels/build_manager.rs:313,455`, `tools/aw_editor/src/main.rs:6616,6659`, `tools/aw_editor/src/file_watcher.rs:47`
- `tokio::sync::mpsc`: 6 sites in `astraweave-llm/src/production_hardening.rs:1630,1751,1807,1866,2181,2226` (buffer sizes 1-9), plus `astraweave-render/src/texture_streaming.rs:11` for GPU texture streaming results.
- `tokio::sync::oneshot`: only in render tests.
- `tokio::sync::broadcast`: none found.
- `crossbeam::channel` / `flume`: none found in production code (crossbeam is pulled in transitively by rayon).

### 1.2 ECS parallel schedule

This is the most important finding in the audit; it is documented in full.

**Two schedulers coexist.**

**Sequential `Schedule`** (`astraweave-ecs/src/lib.rs:690-722`):
- Stage-based: `Vec<Stage>`, each stage is `name: &'static str` + `systems: Vec<SystemFn>`.
- `Schedule::run` (`lib.rs:712-721`) is two nested loops: `for s in &self.stages { for f in &s.systems { (f)(world); } }`. No parallelism.
- Stage order is fixed by `App::new` at `lib.rs:757-763`: `perception → simulation → ai_planning → physics → presentation`. Note this is five stages; the declared canonical order in `CLAUDE.md` lists seven (`PRE_SIMULATION`, `PERCEPTION`, `SIMULATION`, `AI_PLANNING`, `PHYSICS`, `POST_SIMULATION`, `PRESENTATION`) per the `SystemStage` constants at `lib.rs:94-102`. `App::new` does not create the `PRE_SIMULATION` or `POST_SIMULATION` stages — consumers must `add_stage` them manually if used.
- **Schedule does not record `SystemAccess`** — there is no per-system read/write declaration in the sequential path. Systems are plain `fn(&mut World)` (`lib.rs:690-696` implicit via `SystemFn`).

**Parallel `ParallelSchedule`** (`astraweave-ecs/src/parallel.rs:186-326`):
- Scheduling model: stage-based with per-stage parallel groups. Stages run strictly sequentially (`parallel.rs:268-284`). Within a stage, systems are greedy-coloured into groups; groups run sequentially; systems within a group run concurrently via `rayon::scope` (`parallel.rs:296-325`).
- Access declaration: trait-free, builder-based on `SystemDescriptor` at `parallel.rs:117-174`. `.reads::<T>()` (line 156-160) and `.writes::<T>()` (line 162-167) record `TypeId` into hash sets; `.exclusive()` (line 169-173) forces serial execution. `SystemDescriptor::new` defaults to `exclusive = true` (`parallel.rs:129-138`) — **undeclared systems run alone**. This is the safe default, but it means a drop-in migration from `Schedule` to `ParallelSchedule` that does not annotate every system provides zero parallelism.
- Conflict detection: `SystemAccess::conflicts_with` (`parallel.rs:97-113`) — write-write, write-read, read-write, or either-exclusive counts as conflict; read-read is non-conflicting.
- `build_groups` (`parallel.rs:230-257`): greedy graph-colouring. For each system, scan existing groups and drop it in the first whose members all have disjoint access; otherwise start a new group. O(systems² × avg_group_size) per call; "fine since stages typically have <20 systems" per the doc comment. **Called every `run()` call, every tick**, per `parallel.rs:273`.
- Dispatch: `run_group_parallel` (`parallel.rs:296-325`) uses `rayon::scope` + an unsafe `SendWorldPtr(*mut World)` wrapper (`parallel.rs:42-70`). Single systems run directly, multi-system groups fan out via `s.spawn`. The `SAFETY` argument at `parallel.rs:290-295, 302-304, 310-311` ties soundness to the scheduler's own conflict analysis.
- System kinds: three — read-only (`.reads`), mutating (`.writes`), exclusive (`.exclusive()`). There is no event-writer / event-reader / command-buffer split the way Bevy has.
- Cross-stage parallelism: **none**. Stages are hard barriers.
- Frame pipelining: **none**. `run()` returns when all stages finish; there is no "stage N of frame F+1 starts while stage N+1 of frame F still running" path.

**Who uses `ParallelSchedule`?**

Only tests and benches. `rg 'ParallelSchedule::new|ParallelSchedule::default'` returns three hits workspace-wide:
1. `astraweave-ecs/src/parallel.rs:26` (doc example, not real)
2. `astraweave-ecs/src/parallel.rs:476` (unit test)
3. `astraweave-ecs/benches/alloc_measure.rs:75` (bench)

`rg 'ParallelSchedule'` finds matches in only 6 files — the source, its doc, the bench, the allocation audit docs, and `ARCHITECTURE_MAP.md`. **No example, no `tools/aw_editor`, no `examples/profiling_demo`, no `examples/unified_showcase`, and no other runtime driver constructs a `ParallelSchedule`.** Every runtime driver uses the sequential `Schedule` via `App::new()`.

Consequence: the allocation audit's §2.3 #6 ("`ParallelSchedule::build_groups` rebuilt every tick") is technically correct but the observation's practical impact in real binaries is zero — the per-tick allocation only happens in benches. The mimalloc experiment's `+43%` FPS gain on `profiling_demo` therefore comes from the sequential scheduler's per-tick allocation (`lib.rs:712-721` — mainly the component-side cost recorded by `CountingAlloc`), plus GOAP planning and render bin-lights, not from parallel-schedule overhead.

### 1.3 Per-subsystem parallelism

**Rendering (`astraweave-render`)** — no CPU-level parallelism on the frame critical path.
- Culling (`culling.rs:128-145`): serial `for plane in &self.planes`.
- Command encoding: single `wgpu::CommandEncoder` per frame (`renderer.rs:4677-4681`); all passes (cluster binning, shadow, sky, main scene, post) record into it; one `queue.submit` per frame (`renderer.rs:5173`).
- Multiple submissions at init or offline: `clustered.rs:217` (CPU bin setup), `gpu_erosion.rs:145`, `ibl.rs`, `impostor_bake.rs` — all not-per-frame.
- Texture decode: `astraweave-render/src/texture.rs:339-344` uses `tokio::task::spawn_blocking` to move PNG/KTX2 decode off the tokio worker; not parallel with the render path.

**Physics (`astraweave-physics`)** — parallelism is feature-gated and unused in practice.
- `Cargo.toml:9,28` declares `async-physics = ["rayon"]`. Rayon is optional.
- `async_scheduler.rs:315-332` declares `par_process_bodies` and `par_process_collision_pairs` helpers. `rg` shows **zero non-test callers** in the workspace (the only hits are in archive mutation-test outputs and documentation).
- Rapier3D itself may use rayon internally; that is delegated and out of scope here.

**Terrain (`astraweave-terrain`)** — the one real production CPU parallel path.
- `meshing.rs:472-484`: `AsyncMeshGenerator::generate_meshes_parallel` runs `chunks.into_par_iter().map(|chunk| { let mut gen = DualContouring::new(); gen.generate_mesh(&chunk) }).collect()`. One dual-contouring mesher constructed per chunk (potential alloc source — not a topic for this audit).
- `background_loader.rs:536,710`: two `tokio::spawn` sites for per-chunk generation and per-task terrain modification. Uses `mpsc` to return completed chunks to the main thread via `collect_completed_chunks`.
- Meshing and background loading live off the per-frame critical path (chunk generation is amortised by streaming cadence).

**Fluids (`astraweave-fluids`)** — feature-gated per-particle parallelism.
- Gated: `#[cfg(feature = "parallel")]` module at `simd_ops.rs:685-827` and row `simd_ops.rs:2036`.
- 12 `par_iter`/`par_iter_mut`/`into_par_iter` call sites; unit of work is per-particle integration (position/velocity/force) and per-cell grid updates.
- GPU SPH path (`wgpufluid`) is separate and uses wgpu compute dispatch rather than rayon.

**AI (`astraweave-ai`)** — single-agent per-tick; planning can be offloaded.
- `AIArbiter::update` (`ai_arbiter.rs:201-350+`) is a state machine that returns immediately per tick.
- LLM planning: `llm_executor.rs:165-188` spawns an async task onto its internal tokio `Runtime` (constructed once per `LlmExecutor`). The game loop polls via `AsyncTask::try_recv` (`async_task.rs:1-80`) — non-blocking.
- GOAP `plan_direct` (`goap/planner.rs:225-304`) is called synchronously from whatever system invokes it; there is no per-agent parallel planning layer. If the ECS parallel scheduler were in use and multiple agents each ran GOAP in their own system, those systems could parallelise via `ParallelSchedule`, but today every GOAP call is serial.
- Rule/utility planners: `futures_executor::block_on` at `orchestrator.rs:565,631,1086,1192,1266,1365-1367` synchronously drives async planner calls.

**Asset loading (`astraweave-asset`, `astraweave-asset-pipeline`, `astraweave-scene`)**
- `astraweave-asset/src/lib.rs:3666` wraps `Runtime::new()` + `block_on` for sync asset APIs.
- `astraweave-scene/src/streaming.rs:178` + `partitioned_scene.rs:96`: `tokio::spawn` per cell streaming task.
- `astraweave-asset/src/nanite_preprocess.rs:691`: `tokio::task::spawn_blocking` for CPU-bound hierarchy build.
- `tools/astraweave-assets/src/downloader.rs:8` uses `tokio::sync::Semaphore` to bound concurrent downloads (default 8).
- Neither pipeline builds a dependency DAG of asset→asset prerequisites. `astraweave-asset-pipeline/src/lib.rs:1-41` is a flat utility crate (`compress_bc7`, `optimize_mesh`, `AssetValidator`). Build orchestration in `tools/aw_build/src/main.rs:172` uses a single `par_iter()` over artifacts.

**Editor (`tools/aw_editor`)**
- `main.rs:6616-6700`: Blender decomposition worker — `std::thread::spawn` → `Runtime::new()` → `rt.block_on(async { ... tokio::spawn(progress_monitor); importer.decompose(...).await })`.
- `file_watcher.rs`: dedicated watch thread with `Arc<AtomicBool>` shutdown.
- `panels/build_manager.rs`: cancellable build worker with `Arc<AtomicBool> cancel_requested`.
- `panels/terrain_panel.rs:2036-2037` + `panels/world_panel.rs:860`: sync `mpsc` channels for terrain generation results.
- Editor main UI loop is single-threaded egui + winit. Heavy ops fan out to worker threads.

**Behavior / dialogue / quests / director / coordination**: searched — no production rayon/tokio-spawn sites. Serial.

### 1.4 Task graph patterns

**Render graph exists, dormant in production.**

- `astraweave-render/src/graph.rs:388-744` defines `RenderGraph` with two modes:
  - Legacy: `add_node` → `execute` (insertion order).
  - DAG: `add_pass` → `compile()` → `execute_compiled()` (Kahn topological sort).
- `RenderGraph::compile` (`graph.rs:457-630`):
  - Builds a resource → producer map with single-writer enforcement (`graph.rs:459-472`).
  - Builds an adjacency list via read-of-produced (`graph.rs:474-494`).
  - Kahn's algorithm for topsort with cycle detection (`graph.rs:502-527`).
  - Computes per-resource lifetimes (`graph.rs:537-557`).
  - Identifies aliasing groups (non-overlapping transient-texture lifetimes with compatible descriptors; `graph.rs:572-626`).
- `RenderGraph::execute_compiled` (`graph.rs:663-744`): creates physical textures for alias groups once, runs nodes in topo order sharing one `CommandEncoder`, releases resources at computed release points.
- `frame_graph.rs:433-620 build_default_graph(config: &FrameGraphConfig)`: constructs a 10-pass DAG — cluster binning, shadow, sky, GTAO, SSGI, volumetric fog, main scene, SSR, bloom, tonemap.
- `graph_adapter.rs:4-19 run_graph_on_renderer`: a documented entry point that would drive the graph via `Renderer::render_with`.

**Production integration**: `rg 'build_default_graph\|run_graph_on_renderer\|execute_compiled'` against non-test files finds only the declarations. The main `Renderer::render()` function does not import `graph` or `frame_graph`. The graph compiles correctly in tests (13 test call sites in `frame_graph.rs:621-861`), but **no binary, example, or tool drives it**. The legacy fixed-sequence render path (cluster binning → shadow → sky → main → post) at `renderer.rs:4665-5173` is what actually runs every frame.

**GPU dependency management**
- One encoder, one queue.submit per frame (`renderer.rs:5173`). Implicit in-order execution via wgpu.
- No explicit `Fence`, `Semaphore`, or barrier primitives (wgpu's API does not expose them directly; implicit per-pass barriers only).
- Multiple encoders per frame: **not** in the main path. Impostor bake, IBL bake, CPU light bin setup each have their own `queue.submit`, but those run once at init or on-demand, not per frame.

**Non-render task graphs**
- AI: `AIArbiter` is a state machine (`ai_arbiter.rs:88-109` — `AIControlMode::{GOAP, ExecutingLLM { step_index }, BehaviorTree}`). Not a DAG.
- Asset pipeline: no DAG (§1.3 above).
- Cinematics (`astraweave-cinematics/src/lib.rs:96-160`): time-indexed `Track::{Camera, Animation, Audio, FX}` with linear playback by `start_time`; no dependencies between tracks.
- Nav (`astraweave-nav/src/lib.rs:146-150`): runtime query-driven A*; the navmesh is an internal graph, not a scheduling structure.

### 1.5 Async runtime

- Tokio is the sole async runtime. The workspace uses `features = ["full"]` globally (`Cargo.toml:178`), pulling `rt-multi-thread`. Individual crates sometimes pin narrower feature sets (e.g. `astraweave-asset/Cargo.toml:17` uses `["sync", "rt-multi-thread", "fs"]`), but at workspace level `full` wins.
- Entry points: 13 `#[tokio::main]` sites (§1.1). Each instance creates its own multi-thread runtime.
- Sync→async bridges: `pollster::block_on` for wgpu init, `futures::executor::block_on` for orchestrator, `tokio::runtime::Runtime::new().block_on` for asset sync API and editor workers.
- No shared runtime singleton — runtimes are created lazily at each bridge. A given process can have several live runtimes.
- No `LocalSet` use observed in production.

### 1.6 Parallelism primitives in use

**Atomics**
- `AtomicU64` × 18 in `astraweave-llm/src/telemetry.rs:10-27` (requests_total, cache_hits, retries, etc.) — all Relaxed at call sites (`telemetry.rs:50-168`).
- `AtomicU64` × 4 in `astraweave-llm/src/cache/mod.rs:43-60` (cache stats) — Relaxed.
- `AtomicU64` × 2 in `astraweave-render/src/gpu_memory.rs:101,104` (total_used, total_budget) — Relaxed in production, SeqCst in tests.
- `AtomicU64` in `tools/aw_editor/src/viewport/renderer.rs:485-486 LAST_WARN` and `engine_adapter.rs:843-844 LAST_GPU_LOG` for log throttling — Relaxed.
- `AtomicU32` × multiple in ECS tests (`parallel.rs:463-484`), `core` test counter (`capture_replay.rs:73`) — SeqCst.
- `AtomicUsize` × 5 in `astraweave-ecs/src/counting_alloc.rs:73-85` (ALLOCS, DEALLOCS, REALLOCS, BYTES_ALLOCATED, BYTES_DEALLOCATED) — Relaxed, test-only allocator.
- `AtomicBool` in `astraweave-render/src/renderer.rs:669,980,1052,1084 device_lost` — Relaxed.
- `AtomicBool FIRST_UPDATE/FIRST_CALL` in `astraweave-render/src/shadow_csm.rs:735-736,845` — Relaxed, one-shot init guards.
- `AtomicBool` in editor file_watcher/build_manager (shutdown flags) — Relaxed.
- No `AcqRel`/`Release`-ordered atomics in production paths — only Relaxed (telemetry) or SeqCst (tests). The engine does not implement producer-consumer synchronisation via atomics; it uses channels or locks for that.

**Locks**
- `std::sync::Mutex`: used sparingly, mostly inside `OnceLock` containers (`tools/aw_editor/src/telemetry.rs:134-135`, `tools/aw_editor/src/console_bridge.rs:30`, `astraweave-core/src/metrics.rs:91`).
- `std::sync::RwLock`: not used in production.
- `parking_lot::Mutex`: `astraweave-embeddings/src/store.rs:25,57`, `astraweave-embeddings/src/client.rs:312`. `parking_lot` is pulled in `crates/astraweave-blend/Cargo.toml:70`.
- `parking_lot::RwLock`: `astraweave-context/src/token_counter.rs:48-49` (cache + stats), `astraweave-context/src/summarizer.rs:153`, `astraweave-embeddings/src/client.rs:273,312`, `astraweave-embeddings/src/store.rs:11,22`.
- `tokio::sync::Mutex`: `astraweave-llm/src/phi3.rs:33`, `astraweave-llm/src/hermes2pro_ollama.rs:63` (model weights).
- `tokio::sync::RwLock`: LLM subsystem pervasively — `production_hardening.rs:37,327`, `fallback_system.rs:12`, `circuit_breaker.rs:6`, `ab_testing.rs:7`, plus director/dialogue/coordination state.

**Lock-free**
- `dashmap::DashMap`: 8 dependent crates (Cargo hits in `astraweave-llm`, `astraweave-context`, `astraweave-embeddings`, `astraweave-coordination`, `astraweave-memory`, `astraweave-optimization`, `astraweave-observability`, `astraweave-rag`). Call sites at `astraweave-llm/src/tool_guard.rs:86,90,104,106`, `astraweave-llm/src/scheduler.rs:29,88-89,104-105,230-231`, `astraweave-embeddings/src/store.rs:22,56`.
- `arc_swap::ArcSwap`: not found anywhere.
- `crossbeam::queue`: not found.

**OnceLock / lazy singletons**
- `astraweave-llm/src/qwen3_ollama.rs:367,425,638,751` — four `OnceLock<reqwest::Client>`.
- `astraweave-llm/src/phi3_ollama.rs:245`, `astraweave-llm/src/hermes2pro_ollama.rs:261,398,523`.
- `astraweave-core/src/metrics.rs:91` — `GLOBAL_REGISTRY: OnceLock<MetricsRegistry>`.
- `tools/aw_editor/src/telemetry.rs:134-135`.

**Thread-local**: not used in production (`rg 'thread_local!'` returns only examples of what NOT to use, and test harness code).

**Rayon-specific usage breakdown** (production non-test sites):
- `rayon::scope`: 1 site — `astraweave-ecs/src/parallel.rs:305` (only active inside `ParallelSchedule::run_group_parallel`, which is not called in production).
- `rayon::join`: 0 sites.
- `par_iter` / `par_iter_mut`: 13 sites total (§1.3) — 12 in `astraweave-fluids/src/simd_ops.rs`, 1 in `astraweave-physics/src/async_scheduler.rs` (unused), 1 in `tools/aw_build/src/main.rs` (build tool).
- `into_par_iter`: 3 sites — `astraweave-fluids/src/simd_ops.rs:740`, `astraweave-terrain/src/meshing.rs:478`, `astraweave-physics/src/async_scheduler.rs:320` (unused).
- `par_chunks` / `par_bridge`: 0 sites.
- `rayon::ThreadPool::install`: 0 sites.

### 1.7 Frame loop structure

Two patterns coexist.

**Push-driven (manual tick loops)**
- `examples/profiling_demo/src/main.rs:624-631`: `for frame in 0..max_frames { game.tick()?; }`. `tick()` calls `self.app.schedule.run(&mut self.app.world)` at `main.rs:271`, then `frame_mark!()` at `main.rs:352`. Flat-out (no sleep/yield). Fixed frame count determined by CLI args.
- `examples/ecs_ai_showcase/src/main.rs:557-570`: `for _ in 0..300 { app.schedule.run(...); }`. No frame mark, no sleep.
- `examples/hello_companion/src/main.rs:391-393`: `app.run_fixed(1)` for 20 ticks total. `App::run_fixed` at `astraweave-ecs/src/lib.rs:777-782` is `for _ in 0..steps { self.schedule.run(&mut self.world); }`. No sleep.

**Pull-driven (winit event loop)**
- `examples/unified_showcase/src/main.rs:2493-2498`: `event_loop.set_control_flow(ControlFlow::Poll); event_loop.run_app(&mut app)`. Winit drives; redraw via `window.request_redraw()` in `about_to_wait` (`main.rs:2486`). No accumulator, no fixed timestep — renders on event. The showcase has **no** `rayon`, `par_iter`, `tokio::spawn`, `std::thread::spawn`, or `frame_mark!` in its main.rs (`rg` returned zero hits). It uses the sequential `Schedule` via the standard `App`.
- Same pattern in `examples/debug_toolkit_demo/src/main.rs:427-431`, `examples/npc_town_demo/src/main.rs:292-295`, `examples/physics_demo3d/src/main.rs:299-302`, `examples/navmesh_demo/src/main.rs:186-189`.
- `tools/aw_editor/src/main.rs`: egui/winit driven; frame mark at `main.rs:9521`.

**Timestep**
- No fixed-timestep accumulator pattern (`while accumulator >= dt`) anywhere in the workspace. `rg 'accumulator|FIXED_DT|fixed_dt|Duration::from_secs_f32\(1\.0\s*/\s*60'` returns no match in any main/example file.
- Push loops are frame-count-bounded; pull loops are winit-rate-bounded.
- The declared "60 Hz deterministic tick" in `CLAUDE.md` is an aspiration, not a property enforced by any code in the workspace.

**CPU-GPU pipelining**
- No explicit evidence. The renderer's `StagingRing` holds `MAX_FRAMES_IN_FLIGHT = 3` (`astraweave-render/src/staging_ring.rs:27`) — this is GPU-side multi-buffering for staging buffers, not CPU-side frame N+1 prep overlap.
- The main render loop is a straight-line function; nothing preps frame N+1 while the GPU processes N.

**Frame boundary marker (`frame_mark!`)**
- Present: `examples/profiling_demo/src/main.rs:352`, `tools/aw_editor/src/main.rs:9521`.
- Absent: every winit-driven example (`unified_showcase`, `npc_town_demo`, `physics_demo3d`, `navmesh_demo`, `debug_toolkit_demo`), `ecs_ai_showcase`, `hello_companion`.

### 1.8 Oversubscription risks

Enumerated with a claim strength marker (**verified** / **plausible** / **safe by construction**).

1. **Nested rayon inside the ECS parallel scheduler**: **safe by construction today** because `ParallelSchedule` is never instantiated in production (§1.2). Would become **plausible** if a system that is placed inside a `ParallelSchedule` group calls `par_iter` internally — e.g. if a hypothetical "fluid simulation system" annotated with `.writes::<FluidState>()` called `par_iter_mut` from §1.3 while scheduled alongside other systems. Rayon handles nested scopes safely in principle but amplifies jitter.
2. **Rayon `par_iter` called from inside a tokio task**: **plausible** at `tools/aw_editor/src/main.rs:6622-6700` (editor decomposition worker starts `Runtime::new()` inside a `std::thread::spawn`, then `rt.block_on` drives `importer.decompose().await`). If `astraweave-blend` or downstream decoders use rayon internally, the tokio worker blocks on rayon work. No deadlock, but the effective worker count becomes `num_cpus × 2` (tokio pool + rayon pool). Not verified since `astraweave-blend` internals are not inspected.
3. **`tokio::spawn` from inside a rayon worker**: none found. No call site places `tokio::spawn` inside a `par_iter` closure.
4. **`std::thread::spawn` from inside a worker thread**: none found in hot code. Editor spawns threads from the UI thread only.
5. **Multiple thread pools summing to >`num_cpus`**: **verified** in the editor scenario. Simultaneous live pools: rayon global + primary tokio (from editor's `#[tokio::main]` if any, otherwise from `Runtime::new()` in bridges) + secondary `Runtime::new()` in decomposition worker. Each defaults to `num_cpus` workers. Total can exceed core count. Load is bursty (editor mostly idle), so the hazard is intermittent latency spikes rather than sustained contention.
6. **`block_on` inside a rayon worker**: **plausible** if any system scheduled into a `ParallelSchedule` group calls `futures_executor::block_on` (see `astraweave-ai/src/orchestrator.rs:565,631,1086,1192,1266,1365-1367`). Today this is zero because `ParallelSchedule` is unused, but would be a real hazard the moment the parallel scheduler is adopted — the blocked rayon worker cannot be rescheduled.
7. **Silent `build_global()` failure**: **verified at `astraweave-physics/src/lib.rs:1011-1018`**. Called only from `tests/determinism.rs:190` today. If production ever calls `enable_async_physics(n)` after rayon has already been initialised (almost certain, since any prior `par_iter` triggers lazy init of the default pool), the reconfigure silently fails and the pool runs with `num_cpus` workers instead of `n`.

None of the oversubscription risks above have been observed as actual bugs. The reason is mostly that the engine parallelises so little in practice that the nesting conditions rarely arise.

---

## Phase 2 — Analysis

### 2.1 Parallelism inventory

One row per distinct production usage.

| # | Subsystem | Pattern | Primitive | Evidence |
|---|---|---|---|---|
| 1 | ECS (unused in prod) | Fork-join within a stage | `rayon::scope` + `SendWorldPtr` | `astraweave-ecs/src/parallel.rs:296-325` |
| 2 | ECS (used in prod) | Sequential | `for f in &stage.systems { (f)(world); }` | `astraweave-ecs/src/lib.rs:712-721` |
| 3 | Terrain meshing | Data-parallel chunk mesher | `rayon::prelude::into_par_iter().map().collect()` | `astraweave-terrain/src/meshing.rs:472-484` |
| 4 | Terrain streaming | Independent task pool | `tokio::spawn` + `mpsc` | `astraweave-terrain/src/background_loader.rs:536,710` |
| 5 | Fluids SPH (gated) | Data-parallel particle pipeline | `par_iter_mut`, `par_iter`, `into_par_iter` | `astraweave-fluids/src/simd_ops.rs:694-824,2036` |
| 6 | Physics (gated, unused) | Data-parallel body/pair helper | `par_iter` | `astraweave-physics/src/async_scheduler.rs:315-332` |
| 7 | Rendering | Single-threaded with GPU dispatch | one encoder, one submit | `astraweave-render/src/renderer.rs:4677-5173` |
| 8 | AI LLM | Independent background tasks | tokio `Runtime::spawn` + `AsyncTask::try_recv` | `astraweave-ai/src/llm_executor.rs:165-188`, `async_task.rs:1-80` |
| 9 | AI orchestrator sync bridge | Block-on from sync caller | `futures_executor::block_on` | `astraweave-ai/src/orchestrator.rs:565,631,1086,1192,1266,1365-1367` |
| 10 | Asset decode | CPU-bound off async | `tokio::task::spawn_blocking` | `astraweave-render/src/texture.rs:339-344`, `astraweave-asset/src/nanite_preprocess.rs:691` |
| 11 | LLM request routing | Lock-free | `Arc<DashMap>` | `astraweave-llm/src/scheduler.rs:29,88-89,104-105` |
| 12 | LLM telemetry | Lock-free counter | `AtomicU64` Relaxed | `astraweave-llm/src/telemetry.rs:10-168` |
| 13 | Editor import | Dedicated worker + nested runtime | `std::thread::spawn` + `Runtime::new()` + `block_on` + inner `tokio::spawn` | `tools/aw_editor/src/main.rs:6622-6700` |
| 14 | Editor file watch | Dedicated worker thread | `std::thread::spawn` + `AtomicBool` shutdown | `tools/aw_editor/src/file_watcher.rs:200,253` |
| 15 | Editor build | Cancellable worker | `std::thread::spawn` + `Arc<AtomicBool>` | `tools/aw_editor/src/panels/build_manager.rs:313,337` |
| 16 | GPU texture streaming | Async loader with bounded queue | `tokio::sync::mpsc::Receiver` | `astraweave-render/src/texture_streaming.rs:11,76-96` |
| 17 | Network server | Per-connection task | `tokio::spawn` in `#[tokio::main]` | `net/aw-net-server/src/main.rs:169,216,245` |
| 18 | Asset downloads (CLI) | Bounded concurrent tasks | `tokio::sync::Semaphore(8)` | `tools/astraweave-assets/src/downloader.rs:8,137` |
| 19 | Build artifacts (CLI) | Data-parallel | `par_iter` | `tools/aw_build/src/main.rs:172` |

### 2.2 The frame graph (as implemented)

Based on what the code *does*, not what it *could* do.

Default stage order in `App::new()` (`lib.rs:757-763`):

```
perception → simulation → ai_planning → physics → presentation
```

Each stage is a hard barrier: the sequential `Schedule::run` (`lib.rs:712-721`) runs every system in a stage sequentially before moving to the next stage. There is no intra-stage parallelism on this path. Pictorially:

```
Frame N begin
  │
  ▼
  [perception]       ──── serial over N systems ────▶ [simulation]
                                                        │
                                                        ▼
  [presentation] ◀── [physics] ◀── [ai_planning] ◀────┘
  │
  ▼
  Frame N end        (wgpu submit somewhere inside presentation)
```

Render (inside whichever system invokes it) records one encoder with all passes in sequence:

```
cluster_bin (compute) → shadow → sky → main_scene → post → queue.submit (once)
```

CPU-GPU overlap: implicit only, via wgpu's command submission and the 3-slot `StagingRing`. CPU does not start frame N+1 until frame N returns.

Off-frame-critical-path tasks that run in parallel to the main loop (when the subsystem is active):
- Terrain meshing via `into_par_iter` on background chunks (not joined per frame).
- Terrain/cell streaming via `tokio::spawn` (not joined per frame; results collected best-effort).
- LLM planning via `LlmExecutor`'s internal `Runtime::spawn` (polled non-blocking per tick).
- Texture streaming via `tokio::sync::mpsc::Receiver`.
- Editor workers (decomposition, build, file watch) — not part of runtime frame.

**Critical path**: determined entirely by the sequential chain above. Cannot be reconstructed without tracing — I do not have runtime data on which system dominates per-frame wall time. The mimalloc experiment's `profiling_demo` FPS went 956 → 1369 with a faster allocator, which implies a substantial fraction of each tick is spent in allocator code rather than domain work (consistent with the allocation audit's findings on GOAP and render per-frame churn).

### 2.3 Coverage map

Three parallelism layers × subsystem-by-subsystem usage.

| Subsystem | Task-level (rayon/scope/thread) | Data-parallel (par_iter) | GPU (wgpu dispatch) |
|---|---|---|---|
| ECS (prod path) | **No** (sequential `Schedule`) | **No** | n/a |
| ECS (parallel path, unused) | Yes (`rayon::scope`) | **No** | n/a |
| Rendering | **No** (single encoder) | **No** | Yes (every pass) |
| Physics | **No** (unused helpers) | **No** in AstraWeave code | n/a (Rapier internal only) |
| Terrain meshing | n/a | Yes (`into_par_iter`) | n/a |
| Terrain streaming | Yes (tokio) | **No** | n/a |
| Fluids CPU | n/a | Yes (gated `parallel`) | n/a |
| Fluids GPU | n/a | n/a | Yes |
| AI GOAP | **No** | **No** | n/a |
| AI LLM | Yes (tokio) | **No** | n/a |
| Assets | Yes (tokio / spawn_blocking) | **No** | n/a |
| Editor | Yes (std::thread + tokio) | **No** | n/a |

Gap observations:
- **AI GOAP could benefit from data-parallel** (one agent per par_iter item), but would require the ECS parallel scheduler or an explicit `par_iter` over agents in the planning system.
- **Render CPU side** (culling, visible-instance build at `renderer.rs:6032-6069`, light binning at `clustered.rs`) could benefit from `par_iter` on lights/instances. Today all serial.
- **Shadow map rendering per cascade** is recorded serially in one encoder. Per-cascade command encoders + parallel record is the pattern Unreal HDRP uses; AstraWeave does not.
- **Physics has helpers but no callers.** If the parallel scheduler were adopted, this would be free perf.

### 2.4 Gaps

Ranked by inferred per-frame impact. Claim strength noted.

| # | Gap | Claim | Inferred impact | Rationale |
|---|---|---|---|---|
| 1 | `ParallelSchedule` exists but no production binary constructs it. Every `App::new()`-driven loop runs the sequential `Schedule`. | **Inferred from code structure** | High | Systems declaring disjoint reads/writes can run concurrently on `num_cpus` cores. Today they run one at a time. Many stages (perception, ai_planning) are allocation-heavy serial chains. Measurement would require a conversion of `Schedule` consumers (or a compat wrapper) + Tracy capture. |
| 2 | Render graph (`graph.rs`, `frame_graph.rs`) is fully implemented and tested but uncalled in production. The main `Renderer::render` issues passes in a hand-wired fixed sequence. | **Verified (code path)** | High | Aliasable transient textures today all own dedicated GPU memory; pass reordering opportunities are foreclosed; multi-encoder parallel record is impossible without the graph. The perf impact is GPU-side memory pressure and CPU-side encoder serialisation — both matter for complex scenes, less so for `profiling_demo`-scale workloads. |
| 3 | Visible-instance list and CPU light bins built serially per frame. | **Inferred** | Medium | `renderer.rs:6032-6069` iterates all instances in one thread; `clustered.rs` bin-lights is O(lights × cells). Documented at 0.5-2 ms for the CPU light-bin path (allocation audit §2.3 #3). Could `par_iter` trivially. |
| 4 | Per-agent AI planning is serial. GOAP `plan_direct` can take tens of thousands of iterations; if multiple agents plan per tick, each runs on the main thread. | **Inferred (§1.3 AI)** | Medium-to-High (scales with agent count) | The mimalloc experiment's −52% on `ai.goap.plan/actions_16` suggests GOAP dominates AI tick wall time. Running N agents' plans in `par_iter` (one plan per rayon task) would scale linearly with cores up to the number of planning agents per frame. |
| 5 | `build_groups` rebuilt every `ParallelSchedule::run()`. Previously flagged in allocation audit §2.3 #6. | **Verified at parallel.rs:273** | Zero in production (scheduler unused). Would be Low if adopted. | Cache at `add_system` time once scheduler is wired in. |
| 6 | No CPU-GPU frame pipelining. CPU has to finish frame N before issuing frame N+1. | **Inferred** | Medium at realistic render complexity | `MAX_FRAMES_IN_FLIGHT = 3` is only used for staging buffer multi-buffering, not CPU-side pipelining. Mainstream engines overlap CPU prep with GPU present by building frame N+1's command buffer while N's present is pending. |
| 7 | Multiple independent tokio runtimes (`Runtime::new()` at each bridge) rather than one shared runtime. | **Verified (§1.1)** | Low per-frame, Medium during cold starts | Each `Runtime::new()` constructs its own worker pool (~`num_cpus` threads). Creating three to five of these for editor init + asset load + LLM + bridges is expensive in thread count and memory; also increases oversubscription pressure. Share the runtime. |
| 8 | No fixed-timestep accumulator pattern — neither push nor pull loops enforce a 60 Hz tick. `CLAUDE.md`'s 60 Hz claim is aspirational. | **Verified** | Medium (determinism-adjacent) | Determinism audits exist separately; the impact on scheduling is that frame budgets are not defined, so scheduling decisions cannot be optimised against a fixed budget. |
| 9 | No per-system profiling spans around `(f)(world)` in sequential `Schedule::run`. Only one outer span (`ECS::Schedule::run`). The parallel path wraps `build_groups` (`parallel.rs:233-234`) but not individual systems. | **Verified** | Low (observability gap) | Makes it impossible to see which system dominates the frame in Tracy without instrumenting each system manually. |
| 10 | Silent `build_global()` failure when `enable_async_physics` runs after rayon init. | **Verified (§1.8 #7)** | Zero today (unused), High if relied on | Should either panic on reconfigure or not attempt reconfigure at all. |

Every row is "inferred from code structure" or "verified (code path)". No row is "measured" — runtime tracing is required to assign wall-time numbers.

### 2.5 Redundancy

- **Two ECS schedulers with partial functionality overlap.** `Schedule` (`lib.rs:690-722`) has no access declaration; `ParallelSchedule` (`parallel.rs:186-326`) has access declaration and parallel execution. There is no unified abstraction. `App::new` hard-codes `Schedule` at `lib.rs:757-763`; there is no `App::new_parallel` constructor. Tests for the two live separately (`parallel.rs:329-519` for the parallel one). If the parallel scheduler is adopted, the sequential one becomes redundant (and could be implemented as `ParallelSchedule` with all systems marked `.exclusive()`).
- **Render graph + direct render path.** `graph.rs` and `frame_graph.rs` duplicate concerns the direct render path already resolves. Keeping both indefinitely costs clarity.
- **Multiple tokio runtimes.** One per bridge site (§1.1). Each is independently sized.
- **Two streaming implementations** — noted in allocation audit §2.4 (`texture_streaming.rs` and `background_loader.rs`). Orthogonal to this audit but relevant: both spawn tokio tasks, both have priority queues, both recover via `mpsc`. They could share infrastructure; they do not.
- **Helper crate vacuum for rayon patterns.** `astraweave-physics/src/async_scheduler.rs:304-333`'s `par_process_bodies` / `par_process_collision_pairs` are the sort of helpers a shared `astraweave-tasks` crate could host — today they're orphaned in physics.

### 2.6 Consistency with engine philosophy

The project claims determinism, formal verification, AI-native design.

1. **Determinism under rayon work stealing**. The parallel ECS scheduler's groups are built from iteration-ordered `SystemDescriptor` vectors, and rayon `scope` preserves spawn order but not execution order. Systems in a group can touch different components only (by construction), so determinism of the *world state* is preserved. Determinism of *side effects* (logs, allocation counts, RNG draws) is not preserved unless systems avoid side effects or use deterministic per-system state. This is standard Bevy-style trade-off and not inherently broken — worth documenting if the parallel scheduler is adopted. Today, with the sequential scheduler, ordering is deterministic by construction.
2. **Unsafe outside the formally verified set**. `parallel.rs:52-70` defines `SendWorldPtr`, `unsafe impl Send`, `unsafe impl Sync`, and `unsafe fn as_mut` — these are inside `astraweave-ecs`, which is in the Miri/Kani set per `CLAUDE.md`. Four unsafe blocks total (per allocation audit §1.7 item 7). Good: the only unsafe parallelism primitive in the workspace lives in a validated crate.
3. **AI tick latency budget**. The 60 Hz aspiration (16.67 ms) is not enforced by any loop. GOAP planning on one agent alone benches at ~760 µs (baseline) / ~370 µs (mimalloc) per mimalloc experiment §3.3 — within budget for one agent. For ten agents planning in the same tick on the main thread, budget is exhausted. Per-agent parallelism (Gap #4) is a philosophy-consistency requirement, not an optimisation.
4. **LLM inference off the frame thread** — correctly handled via `LlmExecutor` + `AsyncTask::try_recv` (§1.3 AI). Good.
5. **Measurement gap for parallelism**. The engine has `tracy-client = 0.18` deps (allocation audit §1.8) but per-system spans are absent in the sequential `Schedule::run`. If a frame overruns budget, Tracy shows one big "ECS::Schedule::run" span with no breakdown. Same observation the allocation audit makes about `alloc!`/`free!` macros — instrumented infrastructure, not instrumented at call sites.
6. **Render graph claims vs execution**. A 10-pass DAG with topological compilation and aliasing sits unused. Spacecraft-grade standards require that deployed code be executed code.

### 2.7 Comparison to reference engines

| Pattern | Reference engine | AstraWeave state | Evidence |
|---|---|---|---|
| Fixed-stage ECS schedule with barriers | Bevy (pre-2024) | **matches** | `astraweave-ecs/src/lib.rs:757-763` defines five fixed stages; stages are hard barriers in both `Schedule::run` (`lib.rs:712-721`) and `ParallelSchedule::run` (`parallel.rs:268-284`). |
| Explicit dependency graph within stages | Bevy (post-2024), Unity DOTS | **simpler version** | `ParallelSchedule` uses greedy conflict colouring (`parallel.rs:230-257`) rather than a full DAG with explicit `.before()` / `.after()` / `.chain()` dependencies. Correct for disjoint access, but offers less control. |
| Cross-stage dependency resolution | Unreal TaskGraph | **absent** | Stages are always hard barriers; no system can start before its stage begins or overlap with a system in a different stage. |
| Fiber-based cooperative scheduling | Naughty Dog, Frostbite | **absent** | No fibers. |
| Render graph with pass dependencies | Frostbite, Unreal, Bevy | **more sophisticated (in code), absent (in execution)** | `astraweave-render/src/graph.rs:388-744` + `frame_graph.rs:433-620` — Kahn topsort, resource lifetimes, aliasing. Zero production callers. |
| CPU-GPU frame pipelining | Essentially all AAA | **absent** | No frame-N+1 CPU prep while frame-N GPU submit. `StagingRing`'s 3-slot multi-buffering is GPU-side only. |
| Work-stealing task pool | Rayon, TBB | **matches (one pool, unused in prod ECS)** | Default rayon pool via `par_iter`; used only in terrain meshing, fluids, and the unused parallel scheduler. |
| Async runtime for I/O | Any engine with streaming | **matches** | Tokio `full` workspace-wide; used for streaming, assets, network, LLM. |
| Parallel command encoding | Unreal, Unity HDRP | **absent** | Single encoder in `renderer.rs:4677-5173`; no per-pass encoder + parallel record. |
| Parallel chunk meshing | Minecraft-likes, voxel engines | **matches** | `astraweave-terrain/src/meshing.rs:478` uses `into_par_iter`. |

---

## Phase 3 — Recommendations

Ranked by value-per-risk. Capped at 8 per the audit brief.

### 1. Adopt `ParallelSchedule` in one production binary and measure

- **Current state**: `App::new()` (`astraweave-ecs/src/lib.rs:756-768`) hard-codes `Schedule`; `ParallelSchedule` (`parallel.rs:186-326`) is never instantiated in production (§1.2). `rg 'ParallelSchedule::new|ParallelSchedule::default'` returns only tests and benches.
- **Target state**: Pick one binary (`examples/profiling_demo` is the obvious candidate — it is already instrumented for Tracy and allocation measurement). Annotate each of its systems with `.reads::<T>() / .writes::<T>()` on a `SystemDescriptor`, construct a `ParallelSchedule` instead of the default `Schedule`, run under `--features parallel` and `--features profiling,alloc-counter`. Capture allocs/tick and wall-time for three runs each with the sequential and parallel schedulers. Report FPS delta alongside the mimalloc comparison.
- **Scope**: One binary + a thin helper to wrap `App` around `ParallelSchedule`. ~100 LoC.
- **Risk**: Low. The parallel scheduler has unit tests (`parallel.rs:329-519`); the change is opt-in per binary; sequential path stays in place.
- **Value**: High. Without measured data we cannot justify adopting the parallel path workspace-wide. Without adopting it, we carry dead infrastructure in a flagship crate.

### 2. Drive the render graph from `Renderer::render`

- **Current state**: `astraweave-render/src/graph.rs:388-744` + `frame_graph.rs:433-620` are fully implemented; `graph_adapter::run_graph_on_renderer` (`graph_adapter.rs:4-19`) exists; nothing in production calls any of them. `rg 'build_default_graph\|run_graph_on_renderer'` outside `frame_graph.rs`'s own test module returns zero production hits.
- **Target state**: Replace the hand-wired pass sequence in `Renderer::render` (`renderer.rs:4677-5173`) with `RenderGraph::execute_compiled` driven by `build_default_graph(&config)`. Preserve the direct-path as a fallback behind a `legacy_render` feature for one release cycle.
- **Scope**: `astraweave-render` only; medium diff because the pass nodes currently delegate back to `Renderer` methods (each needs a small adapter that records into the graph's encoder).
- **Risk**: Medium. The render crate has ~102k LoC and the main render path is the most-changed file; regressions are likely. Mitigated by keeping the legacy path behind a feature flag and by the graph's test coverage.
- **Value**: High. Unlocks transient-resource aliasing (GPU memory win), enables per-pass timing in Tracy, and puts the infrastructure that was written into use. Also enables later recommendations around parallel command encoding.

### 3. Cache `ParallelSchedule::build_groups` output

- **Current state**: `parallel.rs:273` recomputes groups on every `run()` call. `SystemDescriptor::access` only changes when `add_system` is called.
- **Target state**: Store the compiled groups on the `ParallelStage` struct. Invalidate on `add_system`. Compute on first `run()`.
- **Scope**: `astraweave-ecs/src/parallel.rs`, one file, ~30 LoC.
- **Risk**: Low. No behaviour change; the allocation audit (§2.3 #6) already identified this.
- **Value**: Low today (the scheduler is unused); becomes meaningful if recommendation #1 lands.

### 4. Share one tokio runtime per binary, not one per bridge

- **Current state**: `Runtime::new()` called at each sync→async bridge — `astraweave-asset/src/lib.rs:3666`, `tools/aw_editor/src/main.rs:6622`, `examples/hello_companion/src/main.rs:920,942`, `tools/aw_editor/src/viewport/renderer.rs:413,1026,1068,1371`. Each allocates its own worker pool (~`num_cpus` threads).
- **Target state**: Introduce a process-level shared runtime (e.g. `astraweave-core::runtime::shared()` returning `&'static tokio::runtime::Handle`). Rewrite every `Runtime::new().block_on(...)` as `shared().block_on(...)` and every standalone `block_on` as `Handle::block_on`.
- **Scope**: One new module in `astraweave-core` plus call-site edits; medium-sized PR.
- **Risk**: Medium. Global state requires careful init ordering (the runtime must be created before any `block_on` that uses it). Process teardown also needs attention — `OnceLock<Runtime>` works but the runtime must outlive all tokio work.
- **Value**: Medium. Reduces thread count in editor (3+ runtimes → 1), simplifies the oversubscription picture (§1.8 #5), and removes an anti-pattern that bleeds cold-start latency.

### 5. Per-system Tracy spans in `Schedule::run` / `ParallelSchedule::run_group_parallel`

- **Current state**: `Schedule::run` wraps the whole function in one span (`lib.rs:713-714`). `ParallelSchedule::run_group_parallel` has no span at all — only `build_groups` does (`parallel.rs:233-234`). A frame overrun shows up as one large "ECS::Schedule::run" in Tracy with no breakdown.
- **Target state**: Inside both executors, wrap each `(f)(world)` call in a `span!(system_name)`. For parallel execution, each rayon task opens a span on its worker thread. Requires storing the system name on `SystemDescriptor` (already present at `parallel.rs:120`) and on the sequential `Schedule::Stage::systems` (currently the `SystemFn` has no name — would need a `SystemFn + &'static str` pair or a `NamedSystemFn` wrapper).
- **Scope**: `astraweave-ecs` only; touches the `Schedule::Stage` API surface.
- **Risk**: Low-to-Medium. API-surface change to `Schedule::add_system` — backwards-compatible default via `#[track_caller]` or a default name possible.
- **Value**: High for observability. Unlocks measurement of every other recommendation on this list. Complements the allocation audit's recommendation #1.

### 6. Parallelise per-agent GOAP planning

- **Current state**: GOAP `plan_direct` (`astraweave-ai/src/goap/planner.rs:225-304`) runs on whatever thread calls it. If ten agents each need to plan in the same tick, ten plans run serially. Per mimalloc experiment §3.3, a 16-action plan is ~370 µs (mimalloc) — at ten agents this is 3.7 ms of the 16.67 ms budget, serial.
- **Target state**: In the system that triggers planning, run agent plans via `par_iter` on the agents slice, returning one plan per agent. Each plan's `PlannerScratch` (see allocation audit Phase 3 rec #2) is thread-local. This does **not** require the ECS parallel scheduler — just a single `par_iter` inside one system.
- **Scope**: `astraweave-ai` planning system only. Small diff (~50 LoC).
- **Risk**: Medium. GOAP's working state is not currently split per-thread; needs scratch ownership clarified.
- **Value**: Medium-to-High, depending on realistic agent count per tick. Scales linearly up to `min(agents, num_cpus)`.

### 7. Gate `astraweave-physics::enable_async_physics` on first-init success

- **Current state**: `astraweave-physics/src/lib.rs:1011-1018` silently discards the `build_global()` error. `let _ = rayon::ThreadPoolBuilder::new().num_threads(n).build_global();`.
- **Target state**: Distinguish "pool not yet initialised" from "already initialised". If already initialised with a different count, either `log::warn!` and continue, or return `Result<(), PoolAlreadyInitialised>` so the caller can decide. Document that rayon init must happen before the first `par_iter` workspace-wide.
- **Scope**: `astraweave-physics/src/lib.rs`, ~10 LoC, plus a one-line doc update.
- **Risk**: Low.
- **Value**: Low today (unused); a landmine-defuser for when recommendation #1 or #6 lands and rayon is initialised lazily before `enable_async_physics` can take effect.

### 8. Parallelise `build_visible_instances` and `bin_lights_cpu`

- **Current state**: `renderer.rs:6032-6069` (`build_visible_instances`) is a single-threaded loop over all instances. `clustered.rs:34-108` (`bin_lights_cpu`) is a single-threaded loop over lights × cells — documented at 0.5-2 ms per allocation audit §2.3 #3.
- **Target state**: Replace the instance visibility filter with `par_iter().filter().collect_into_vec(&mut reused_buffer)`. For CPU light binning, use a parallel reduction pattern or keep it serial but guarded by the GPU path (`clustered_megalights`) when available.
- **Scope**: Two functions in `astraweave-render`. ~40 LoC plus buffer reuse from allocation audit rec #3.
- **Risk**: Low (data-parallel over independent instances/lights). Medium if combined with buffer reuse — the `par_iter().collect_into_vec` pattern requires rayon 1.6+ (available in workspace).
- **Value**: Medium. Directly on the per-frame critical path.

### Appendix — future considerations (not in the top 8)

- **Parallel command encoding for shadow cascades.** Every cascade's draw call records into the same encoder. Unreal HDRP records each cascade into its own `SecondaryCommandEncoder` and joins at submit. Applies once recommendation #2 lands.
- **CPU-GPU frame pipelining.** Overlap frame-N+1 CPU prep with frame-N GPU submit. The `StagingRing` (`staging_ring.rs:27`, `MAX_FRAMES_IN_FLIGHT = 3`) already supports three frames' worth of staging; the render loop would need restructuring to match. Non-trivial change, not a quick win.
- **Unified streaming primitive.** Per-allocation-audit §2.4, terrain and texture streaming both have LRU + priority queue + mpsc. One abstraction could serve both.
- **Per-stage Tracy frame mark.** `frame_mark!()` is present in only two binaries (§1.7). Adding it at `Schedule::run`'s end (or at every `run_fixed` iteration) would enable Tracy's frame-bar view across all examples.
- **`ArcSwap` for hot read-heavy state.** The LLM subsystem uses `tokio::sync::RwLock` heavily (`production_hardening.rs:37,327`, `fallback_system.rs:12`, `circuit_breaker.rs:6`). Patterns dominated by reads could switch to `ArcSwap`. Low per-unit value, workshop-level improvement.
- **Benchmark `ParallelSchedule` with real-workload system counts.** The `astraweave-ecs/benches/alloc_measure.rs:75` bench uses `systems_16` — representative but not matched to any shipping binary. Once recommendation #1 has real consumer data, re-run the benchmark at the observed system counts.

---

## Evidence index

Deduplicated list of every file read or cited, grouped by crate. Line references in Phases 1-3 above are authoritative; this is just the inventory.

**astraweave-ecs**
- `src/lib.rs` — `Schedule` and `App` (stages, run, run_fixed).
- `src/parallel.rs` — `SystemAccess`, `SystemDescriptor`, `ParallelSchedule`, `run_group_parallel`, `build_groups`, `SendWorldPtr`.
- `benches/alloc_measure.rs` — only non-test `ParallelSchedule` consumer.

**astraweave-render**
- `src/renderer.rs` — `Renderer::render`, single command encoder, single queue.submit, `build_visible_instances`, `update_instances`, device_lost atomics.
- `src/graph.rs` — full render-graph DAG: `RenderGraph::{add_node, add_pass, compile, execute_compiled}`, `CompiledGraph`, resource lifetimes, aliasing.
- `src/frame_graph.rs` — `build_default_graph`, `FrameGraphConfig`, 13 test sites.
- `src/graph_adapter.rs` — `run_graph_on_renderer`.
- `src/clustered.rs` — CPU `bin_lights_cpu`, `sync_channel` readback.
- `src/staging_ring.rs` — `MAX_FRAMES_IN_FLIGHT = 3`.
- `src/texture.rs` — `tokio::task::spawn_blocking` for decode.
- `src/texture_streaming.rs` — `tokio::sync::mpsc::Receiver`.
- `src/terrain_gpu_bridge.rs`, `src/impostor_bake.rs` — `std::sync::mpsc`.
- `src/shadow_csm.rs` — `FIRST_UPDATE` / `FIRST_CALL` AtomicBool.
- `src/gpu_memory.rs` — `GpuMemoryBudget` atomics.
- `src/culling.rs` — serial frustum.

**astraweave-physics**
- `src/lib.rs` — `enable_async_physics`, `rayon::ThreadPoolBuilder::build_global()`.
- `src/async_scheduler.rs` — `AsyncPhysicsScheduler`, `par_process_bodies`, `par_process_collision_pairs`.
- `Cargo.toml` — `async-physics = ["rayon"]` gate.
- `tests/determinism.rs` — only `enable_async_physics` caller.

**astraweave-terrain**
- `src/meshing.rs` — `AsyncMeshGenerator::generate_meshes_parallel` `into_par_iter`.
- `src/background_loader.rs` — `tokio::spawn` per chunk/task.

**astraweave-fluids**
- `src/simd_ops.rs` — `#[cfg(feature = "parallel")]` SPH `par_iter_mut` chains (lines 694-824, 2036).
- `src/lib.rs` — GPU SPH path (wgpufluid).
- `src/gpu_volume.rs` — `pollster::block_on` at wgpu init.

**astraweave-ai**
- `src/ai_arbiter.rs` — `AIControlMode` state machine.
- `src/core_loop.rs` — `CAiController`.
- `src/llm_executor.rs` — tokio `Runtime`, `spawn`, `AsyncTask::try_recv`, `block_on` sync variant.
- `src/orchestrator.rs` — `Builder::new_current_thread`, `futures_executor::block_on` x multiple.
- `src/veilweaver.rs` — 100 ms `block_on`.
- `src/goap/planner.rs` — serial `plan_direct`.
- `src/async_task.rs` — `AsyncTask<T>` wrapper.

**astraweave-asset / astraweave-asset-pipeline / astraweave-scene**
- `astraweave-asset/src/lib.rs` — `Runtime::new` + `block_on`, `std::sync::mpsc`.
- `astraweave-asset/src/nanite_preprocess.rs` — `tokio::task::spawn_blocking`.
- `astraweave-asset-pipeline/src/lib.rs` — flat utilities, no DAG.
- `astraweave-scene/src/streaming.rs`, `src/partitioned_scene.rs` — `tokio::spawn`.

**astraweave-llm**
- `src/telemetry.rs` — 18x `AtomicU64` Relaxed.
- `src/cache/mod.rs` — 4x `AtomicU64` Relaxed.
- `src/scheduler.rs` — `DashMap` request routing.
- `src/tool_guard.rs` — `DashMap`.
- `src/production_hardening.rs` — `tokio::sync::mpsc` x 6, `tokio::sync::RwLock`.
- `src/qwen3_ollama.rs`, `src/phi3_ollama.rs`, `src/hermes2pro_ollama.rs` — `OnceLock<reqwest::Client>`.
- `src/phi3.rs` — `tokio::sync::Mutex`.
- `src/fallback_system.rs`, `src/circuit_breaker.rs`, `src/ab_testing.rs` — `tokio::sync::RwLock`.

**astraweave-embeddings / astraweave-context**
- `embeddings/src/store.rs` — `DashMap` + `parking_lot::Mutex`.
- `embeddings/src/client.rs` — `parking_lot::RwLock<BertModel>`.
- `context/src/token_counter.rs` — `parking_lot::RwLock`.
- `context/src/summarizer.rs` — `parking_lot::RwLock`.
- `context/src/history.rs` — `RwLock` import.

**astraweave-cinematics / astraweave-nav**
- `cinematics/src/lib.rs` — `Track` time-indexed enum.
- `nav/src/lib.rs` — `NavMesh::bake`, A* on demand.

**astraweave-core**
- `src/metrics.rs` — `GLOBAL_REGISTRY: OnceLock<MetricsRegistry>`.
- `src/capture_replay.rs` — test counter.

**Tools & examples**
- `tools/aw_editor/src/main.rs` — `std::thread::spawn` + `Runtime::new()` + `block_on` + `tokio::spawn` nested worker; frame_mark at line 9521.
- `tools/aw_editor/src/file_watcher.rs`, `src/panels/build_manager.rs`, `src/panels/terrain_panel.rs`, `src/panels/world_panel.rs`, `src/telemetry.rs`, `src/console_bridge.rs`, `src/viewport/{renderer.rs, widget.rs, engine_adapter.rs}`.
- `tools/astraweave-assets/src/downloader.rs` — `tokio::sync::Semaphore(8)`.
- `tools/aw_build/src/main.rs` — `par_iter`.
- `examples/profiling_demo/src/main.rs` — push loop, `schedule.run`, `frame_mark!`.
- `examples/hello_companion/src/main.rs` — `run_fixed(1)` x 20.
- `examples/ecs_ai_showcase/src/main.rs` — `for _ in 0..300` manual loop.
- `examples/unified_showcase/src/main.rs` — winit EventLoop, `pollster::block_on`, zero parallelism primitives.
- `examples/{debug_toolkit_demo, npc_town_demo, physics_demo3d, navmesh_demo}/src/main.rs` — winit.

**Net**
- `net/aw-net-server/src/main.rs` — `#[tokio::main]`, `tokio::spawn` x 3.
- `net/aw-net-client/src/main.rs` — `#[tokio::main]`.

**Workspace**
- `Cargo.toml` — `tokio = { version = "1", features = ["full"] }`.
- `CLAUDE.md` — 60 Hz and stage-order claims.
- `docs/audits/allocation_audit_2026-04-17.md`, `docs/audits/mimalloc_experiment_2026-04-17.md` — prior context.
- `docs/architecture/ARCHITECTURE_MAP.md` — only doc hit for `ParallelSchedule`.

---

## Open questions

Each cannot be answered from static analysis alone. Specific data that would resolve it is noted.

1. **Which stage dominates per-frame wall time in `profiling_demo` and `unified_showcase`?** Requires Tracy capture with per-system spans (recommendation #5 enables this).
2. **If `ParallelSchedule` is wired into `profiling_demo`, what FPS delta does it deliver?** Requires recommendation #1 implementation + criterion or release-build FPS capture, three runs each.
3. **Do rayon's implicit global pool and tokio's per-runtime pools actually contend in the editor?** Requires a Tracy capture of editor startup + one asset import; look for thread-oversubscription patterns (worker threads blocked on each other).
4. **Is the LLM `AsyncTask::try_recv` ever unable to make forward progress because the tokio runtime is saturated?** Requires end-to-end capture during a stress run with `N > num_cpus` simultaneous plans.
5. **Does `astraweave-blend::BlendImporter::decompose().await` internally use rayon?** Would confirm oversubscription risk #2. Requires reading `astraweave-blend` internals (out of scope here).
6. **What is the per-frame overhead of `build_groups` in `ParallelSchedule::run` at the system counts realistic for `profiling_demo`?** Requires benchmark extension to match production topology (currently only `systems_16` is benched).
7. **Does the render graph's compile-step meaningfully reduce GPU memory vs the hand-wired path?** Requires an `astraweave-render` measurement: total bytes allocated via `GpuMemoryBudget` before/after swapping recommendation #2 in.
8. **Does the `#[tokio::main]` on `world_partition_demo` create a second runtime concurrent with the one `astraweave-scene` uses for streaming, or do they share?** Requires runtime introspection — add a `runtime.metrics().num_workers()` probe to both.

---

## Verification hooks

Three one-line commands a reviewer can run to independently confirm the top-level findings.

```bash
# 1. Confirm ParallelSchedule is used only in tests and one bench — zero production callers.
rg -n 'ParallelSchedule::new|ParallelSchedule::default' --glob '!target'

# 2. Confirm the render graph infrastructure has no non-test drivers.
rg -n 'build_default_graph|run_graph_on_renderer|execute_compiled' --glob '!target' --glob '!**/tests/**' --glob '!**/_tests.rs'

# 3. Catalogue every rayon parallel-iterator call site in production (non-test) source,
#    so a reviewer can verify the inventory in §2.1 rows 3, 5, 6, 8.
rg -n 'par_iter|par_iter_mut|into_par_iter|par_chunks|par_bridge|rayon::scope|rayon::join' \
   --glob '**/src/**/*.rs' --glob '!target' --glob '!**/benches/**' --glob '!**/tests/**'
```

---

**Report status**: Discovery and analysis complete. No code changes made. All recommendations require explicit follow-up. Phase 3 items gate on runtime measurement that this audit does not have — every value claim is derived from code structure, not wall-time data.
