# Binary Inventory for ParallelSchedule Adoption — 2026-04-18

**Status**: Discovery complete. Read-only audit.
**Deliverable**: Shortlist of binaries whose registered-system topology could make the `ParallelSchedule` code path (specifically `run_group_parallel` → `rayon::scope`) execute with at least one disjoint-access pair. No code changes; recommendations at §3.
**Related**: `docs/audits/job_system_audit_2026-04-18.md`, `docs/audits/parallel_schedule_experiment_2026-04-18.md` (prior experiment on `profiling_demo` showed the binary has 1 system per stage → rayon never invoked).

---

## Executive summary

Of eight workspace binaries that touch `astraweave_ecs::App`, only **two** actually register more than one system in the same executing stage: `examples/ecs_ai_showcase` (3 systems in `"simulation"`) and the editor / `hello_companion` pair via `astraweave_core::ecs_adapter::build_app` (2 systems in `"simulation"`). Both have exactly one disjoint-access pair inside that stage; everything else is either 1 system per stage, not a schedule driver at all, or silently broken.

A latent engine bug surfaces during this inventory and deserves its own callout before the tables: `Schedule::add_system` at [astraweave-ecs/src/lib.rs:707-711](../../astraweave-ecs/src/lib.rs#L707-L711) **silently drops systems whose stage name does not already exist** in the schedule. `App::new()` at [astraweave-ecs/src/lib.rs:757-763](../../astraweave-ecs/src/lib.rs#L757-L763) creates only five stages — `perception`, `simulation`, `ai_planning`, `physics`, `presentation` — and does **not** create the `PRE_SIMULATION`, `POST_SIMULATION`, or `sync` stages named elsewhere in the codebase. Every `add_system("pre_simulation", ...)`, `add_system("post_simulation", ...)`, and `add_system("sync", ...)` call in the workspace is a no-op at runtime. This affects the registered-system counts in §2 and the prior experiment's system count on `profiling_demo`. The prior experiment's correctness result (bit-identical state checksums) still holds — the dropped systems in `profiling_demo` are pure timing instrumentation — but the 6-system count it reported in §3.1 should have been 4. Flagged in §4 as an open issue; not fixed here.

Shortlist, ranked (detail in §3.1):

1. **`examples/ecs_ai_showcase`** — 3 systems in `simulation`; one disjoint pair (`movement_system` ↔ `combat_system`); 300-tick push loop; no external async. Recommended as next experiment target.
2. **`examples/hello_companion`** — 2 systems in `simulation` (`sys_sim`, `sys_move`); one disjoint pair; only 20 ticks total, too short for meaningful timing measurement. Not recommended.
3. **`tools/aw_editor`** (via `runtime.rs`) — same 2-system `simulation` stage as `hello_companion`; can run 1000+ ticks in play mode; but exists inside a UI thread with 3 concurrent tokio runtimes already active. High oversubscription risk.

Everything else (`profiling_demo`, `ecs_ai_demo`, `scripting_playground`, `scripting_advanced_demo`) is 1-system-per-stage. `veilweaver_demo` never drives a schedule.

---

## Phase 1 — Binary enumeration

### 1.1 / 1.2 / 1.3 Combined table

`rg -l 'astraweave_ecs' --glob '**/main.rs'` workspace-wide (excluding `target/`, `archive/`, tests) returns **7 main.rs files**, plus `tools/aw_editor/src/runtime.rs` which hosts the editor's simulation `App`. The starting list from audit §1.7 has 15 entries; the rest either use a different App (winit/egui) or are network/CLI tools not driving an ECS tick loop. Verified: all 12 non-ECS binaries in the starting list have **zero** `astraweave_ecs` imports in their first 30 lines ([verification commands below](#verification-hooks)).

| Binary | Path | Schedule driver | Loop type | Tick count / condition | Notes |
|---|---|---|---|---|---|
| profiling_demo | `examples/profiling_demo/src/main.rs:262` | `self.app.schedule.run(&mut self.app.world)` inside `tick()` | Push | CLI arg `-f N` (default 1000) | Already instrumented. Prior experiment target. |
| ecs_ai_showcase | `examples/ecs_ai_showcase/src/main.rs:564` | `app.schedule.run(&mut app.world)` | Push | Hardcoded `for _ in 0..300` ([main.rs:557](../../examples/ecs_ai_showcase/src/main.rs#L557)) | 6-tick events update; AI state prints every 60 ticks. |
| hello_companion | `examples/hello_companion/src/main.rs:392,464` | `app = app.run_fixed(1)` inside `for _ in 0..20` | Push, fixed-step | 20 ticks total | Built via `build_app(w, 0.25)` from `astraweave-core::ecs_adapter`. |
| ecs_ai_demo | `examples/ecs_ai_demo/src/main.rs:59` | `app = app.run_fixed(1)` inside `for tick in 1..=10` | Push, fixed-step | 10 ticks | Uses `AiPlanningPlugin` + inline `move_system`. |
| scripting_playground | `examples/scripting_playground/src/main.rs:135` | `app.schedule.run(&mut app.world)` | Push | `for i in 0..100` ([main.rs:126](../../examples/scripting_playground/src/main.rs#L126)) | Only `ScriptingPlugin`. |
| scripting_advanced_demo | `examples/scripting_advanced_demo/src/main.rs:100` | `app.schedule.run(&mut app.world)` | Push | Hardcoded loop | Only `ScriptingPlugin`. `#[tokio::main]`. |
| veilweaver_demo | `examples/veilweaver_demo/src/main.rs:466` | (none) | — | — | Creates `App::new()` but never calls `schedule.run` / `run_fixed` ([rg: no match](#phase-1-enumeration) for these in that file). Uses `app.world` as a component store only. |
| aw_editor (runtime) | `tools/aw_editor/src/runtime.rs:691` | `moved_app.take().unwrap().run_fixed(1)` | Push, fixed-step, conditional | Per-UI-frame during `RuntimeState::Playing` (see §3.2) | Built via `build_app(sim_world, fixed_dt)` at [runtime.rs:565](../../tools/aw_editor/src/runtime.rs#L565). |

### 1.4 Classification

| Binary | Class | Reason |
|---|---|---|
| profiling_demo | Out of scope — already audited | Prior experiment showed 1 system per stage, no parallelism possible. Not re-evaluating. |
| ecs_ai_showcase | **In scope** | 3 systems in `simulation` (§2.2). Biggest target. |
| hello_companion | **In scope — short-lived** | 2 systems in `simulation` via `build_app`. 20 ticks total is below any meaningful measurement threshold. |
| ecs_ai_demo | Out of scope — trivial | 2 systems, 2 stages, 1 system each. No intra-stage parallelism possible. |
| scripting_playground | Out of scope — trivial | 1 system (`script_system` via `ScriptingPlugin` at [scripting/lib.rs:585](../../astraweave-scripting/src/lib.rs#L585)). |
| scripting_advanced_demo | Out of scope — trivial | Same 1 system as playground. |
| veilweaver_demo | Out of scope — no schedule | Never invokes `schedule.run` or `run_fixed`. `App` is a component container, nothing more. |
| aw_editor (runtime) | **In scope** | Same 2-system `simulation` as `hello_companion`, but can run for thousands of ticks. High external-concurrency risk — see §3.2. |

**Net**: three in-scope candidates (`ecs_ai_showcase`, `hello_companion`, `aw_editor`); two of them share the same system set (the editor and `hello_companion` both go through `build_app`).

---

## Phase 2 — System inventory per in-scope binary

### Editor first — `tools/aw_editor` (via `build_app` from `astraweave_core::ecs_adapter`)

Systems registered by [`build_app` at astraweave-core/src/ecs_adapter.rs:169-227](../../astraweave-core/src/ecs_adapter.rs#L169-L227):

```
app.add_system("simulation", sys_sim as ecs::SystemFn);       // line 218
app.add_system("simulation", sys_move as ecs::SystemFn);      // line 219
app.add_system("sync",       sys_bridge_sync as ecs::SystemFn);   // line 221
app.add_system("sync",       sys_sync_to_legacy as ecs::SystemFn); // line 223
app.add_system("perception", sys_refresh_los as ecs::SystemFn);   // line 225
```

`App::new()` at [lib.rs:757-763](../../astraweave-ecs/src/lib.rs#L757-L763) creates stages `perception`, `simulation`, `ai_planning`, `physics`, `presentation`. **The two `"sync"` systems are silently dropped** (see Executive summary and §4). The inventory tables below reflect the 3 systems that actually execute.

#### 2.1.1 Registered systems

| System | File:lines | Stage | Reads | Writes | Notes |
|---|---|---|---|---|---|
| `sys_sim` | [ecs_adapter.rs:15-26](../../astraweave-core/src/ecs_adapter.rs#L15-L26) | `simulation` | `Dt` resource, `CCooldowns` (mutating iter) | `World` resource (legacy), `CCooldowns` (mutates values in place) | Mutates the legacy `World` resource via `sim_cooldowns(w, dt)` at line 18 — so writes `Resource<World>`. |
| `sys_move` | [ecs_adapter.rs:28-87](../../astraweave-core/src/ecs_adapter.rs#L28-L87) | `simulation` | `CDesiredPos`, `CPos` (for from-pos capture before mutation) | `CPos`, `Events<MovedEvent>` resource | Emits `MovedEvent` via `ev.writer().send(...)` at line 80. |
| `sys_refresh_los` | [ecs_adapter.rs:89-96](../../astraweave-core/src/ecs_adapter.rs#L89-L96) | `perception` | `World` resource (legacy) | — | Read-only placeholder; touches legacy `W.obstacles` only. |
| `sys_bridge_sync` (dropped) | [ecs_adapter.rs:98-127](../../astraweave-core/src/ecs_adapter.rs#L98-L127) | `"sync"` (non-existent) | — | — | Never runs — see Executive summary. |
| `sys_sync_to_legacy` (dropped) | [ecs_adapter.rs:129-163](../../astraweave-core/src/ecs_adapter.rs#L129-L163) | `"sync"` (non-existent) | — | — | Never runs. |

#### 2.1.2 Resource and event access (executing systems only)

| System | Resource/Event | Read/Write | File:line |
|---|---|---|---|
| `sys_sim` | `Resource<Dt>` | Read | [ecs_adapter.rs:16](../../astraweave-core/src/ecs_adapter.rs#L16) |
| `sys_sim` | `Resource<World>` (legacy) | Write | [ecs_adapter.rs:17-19](../../astraweave-core/src/ecs_adapter.rs#L17-L19) |
| `sys_move` | `Resource<Events<MovedEvent>>` | Write | [ecs_adapter.rs:77-85](../../astraweave-core/src/ecs_adapter.rs#L77-L85) |
| `sys_refresh_los` | `Resource<World>` (legacy) | Read | [ecs_adapter.rs:93-95](../../astraweave-core/src/ecs_adapter.rs#L93-L95) |

The legacy `World` resource is the quiet conflict point here: `sys_sim` writes it (to tick cooldowns); `sys_refresh_los` reads it. They are in different stages (`simulation` vs `perception`), so they don't conflict — stages are hard barriers. Inside `simulation`, only `sys_sim` touches `World`; `sys_move` does not.

#### 2.1.3 Stage composition

| Stage | Systems | Same-stage access conflicts | Parallelism potential |
|---|---|---|---|
| `perception` | 1 (`sys_refresh_los`) | — | **none** (1 system) |
| `simulation` | 2 (`sys_sim`, `sys_move`) | `sys_sim` writes `CCooldowns` + `Resource<World>`; `sys_move` writes `CPos` + `Resource<Events<MovedEvent>>` and reads `CDesiredPos`. Access sets are disjoint — **no conflicts**. | **full** (1 disjoint pair) |
| `ai_planning` | 0 | — | none (no systems) |
| `physics` | 0 | — | none |
| `presentation` | 0 | — | none |

**One disjoint pair in one stage.** `sys_sim` and `sys_move` could run concurrently.

#### 2.1.4 External concurrency interactions

| System | Interaction | File:line | Risk if scheduled in parallel group |
|---|---|---|---|
| `sys_sim` | Calls `sim_cooldowns(w, dt)` → `world_compat.tick(dt)` at [ecs_adapter.rs:12](../../astraweave-core/src/ecs_adapter.rs#L12). No async, no thread spawn, no channel send. | — | Low. Self-contained; no cross-thread handoff. |
| `sys_move` | Writes into `Events<MovedEvent>` via `ev.writer()` at [ecs_adapter.rs:77](../../astraweave-core/src/ecs_adapter.rs#L77). Emission is synchronous. | — | Low. |
| `sys_refresh_los` | Read-only. | — | None. |

The editor's **surrounding** context has high external-concurrency risk — file watcher thread, build manager thread, blend decomposition worker with nested tokio runtime, plus the UI tokio runtime (audit §1.1 item 5). None of those threads interact with the sim_app's `World` directly; they interact with the editor's egui state. Ranking-wise, that's a global property of the editor binary, not a per-system property of the simulation schedule. It does, however, mean the editor already has multiple thread pools at steady state, and adding a rayon scope to the simulation tick increases worst-case oversubscription. Discussed in §3.2.

---

### 2.2 `examples/ecs_ai_showcase`

System registrations at [main.rs:545-550](../../examples/ecs_ai_showcase/src/main.rs#L545-L550):

```
app.add_system("perception",      ai_perception_system);
app.add_system("ai_planning",     ai_planning_system);
app.add_system("simulation",      ai_behavior_system);
app.add_system("simulation",      movement_system);
app.add_system("simulation",      combat_system);
app.add_system("post_simulation", stats_display_system);
```

`stats_display_system` targets the `"post_simulation"` stage, which **does not exist** in `App::new()` — silently dropped. The remaining 5 systems are the executing set.

#### 2.2.1 Registered systems

| System | File:lines | Stage | Reads | Writes | Notes |
|---|---|---|---|---|---|
| `ai_perception_system` | [main.rs:118-179](../../examples/ecs_ai_showcase/src/main.rs#L118-L179) | `perception` | `AIAgent`, `Position`, `Team` | `AIAgent` (`ai.target` field via `world.get_mut::<AIAgent>`) | Nested double loop over entities, O(n²). |
| `ai_planning_system` | [main.rs:186-278](../../examples/ecs_ai_showcase/src/main.rs#L186-L278) | `ai_planning` | `AIAgent`, `Health`, `Position` | `AIAgent` (`ai.state`), `Resource<Events>` (sends `AIStateChangedEvent`) | Event emission on state change. |
| `ai_behavior_system` | [main.rs:303-380](../../examples/ecs_ai_showcase/src/main.rs#L303-L380) | `simulation` | `AIAgent` (state + target fields), `Position` | `Velocity`, `Resource<Events>` (sends `DamageEvent`) | In `Attacking` state, emits damage event. |
| `movement_system` | [main.rs:285-300](../../examples/ecs_ai_showcase/src/main.rs#L285-L300) | `simulation` | `Velocity`, `Resource<GameTime>` | `Position` | `pos.pos += vel_val * delta_time` at line 297. |
| `combat_system` | [main.rs:383-428](../../examples/ecs_ai_showcase/src/main.rs#L383-L428) | `simulation` | `Resource<Events>` (reads `DamageEvent`) | `Health`, `Resource<Events>` (sends `HealthChangedEvent`), `Resource<GameStats>` | Event-driven damage application. |
| `stats_display_system` (dropped) | [main.rs:435-466](../../examples/ecs_ai_showcase/src/main.rs#L435-L466) | `"post_simulation"` (non-existent) | — | — | Never runs — see Executive summary. The console does not show the stats panel that the code claims to print every 60 ticks. |

#### 2.2.2 Resource and event access (executing systems only)

| System | Resource/Event | Read/Write | File:line |
|---|---|---|---|
| `ai_planning_system` | `Resource<Events>` | Write (sends `AIStateChangedEvent`) | [main.rs:269-275](../../examples/ecs_ai_showcase/src/main.rs#L269-L275) |
| `ai_behavior_system` | `Resource<Events>` | Write (sends `DamageEvent`) | [main.rs:357-364](../../examples/ecs_ai_showcase/src/main.rs#L357-L364) |
| `movement_system` | `Resource<GameTime>` | Read | [main.rs:286-289](../../examples/ecs_ai_showcase/src/main.rs#L286-L289) |
| `combat_system` | `Resource<Events>` | Read (reads `DamageEvent`) | [main.rs:386](../../examples/ecs_ai_showcase/src/main.rs#L386) |
| `combat_system` | `Resource<Events>` | Write (sends `HealthChangedEvent`) | [main.rs:417-421](../../examples/ecs_ai_showcase/src/main.rs#L417-L421) |
| `combat_system` | `Resource<GameStats>` | Write | [main.rs:424-427](../../examples/ecs_ai_showcase/src/main.rs#L424-L427) |

The showcase uses a single `Events` resource of mixed type (not typed per-event like `astraweave-core`'s `Events<MovedEvent>`). This means every system that touches events conflicts via the same `TypeId<Events>`.

#### 2.2.3 Stage composition

| Stage | Systems | Same-stage access conflicts | Parallelism potential |
|---|---|---|---|
| `perception` | 1 (`ai_perception_system`) | — | **none** (1 system) |
| `ai_planning` | 1 (`ai_planning_system`) | — | **none** (1 system) |
| `simulation` | 3 (`ai_behavior_system`, `movement_system`, `combat_system`) | Pairwise conflict analysis below | **partial** (1 disjoint pair) |
| `ai_planning` | (as above) | | |
| `physics` | 0 | — | — |
| `presentation` | 0 | — | — |

Pairwise conflict analysis for `simulation`:

- `ai_behavior_system` ↔ `movement_system`: ai_behavior writes `Velocity`, movement reads `Velocity` → **write-read conflict**. ai_behavior reads `Position`, movement writes `Position` → **write-read conflict**. **Conflict**.
- `ai_behavior_system` ↔ `combat_system`: both write `Resource<Events>` → **write-write conflict** on `Events` TypeId. **Conflict**.
- `movement_system` ↔ `combat_system`: movement reads `Velocity` + `Resource<GameTime>`, writes `Position`. Combat reads `Resource<Events>`, writes `Health` + `Resource<Events>` + `Resource<GameStats>`. No shared TypeId. **Disjoint** — 1 pair.

Greedy coloring per `build_groups` at [parallel.rs:230-257](../../astraweave-ecs/src/parallel.rs#L230-L257):

1. `ai_behavior_system` → group 0 = [ai_behavior].
2. `movement_system` — conflicts with ai_behavior → new group: group 1 = [movement].
3. `combat_system` — conflicts with ai_behavior (via Events) → can't join group 0. Check group 1 — disjoint with movement → **joins group 1** = [movement, combat].

Two groups, second one parallel. Running `ai_behavior_system` alone, then `movement_system` and `combat_system` concurrently. One rayon `scope` call per tick on the `simulation` stage.

#### 2.2.4 External concurrency interactions

| System | Interaction | File:line | Risk if scheduled in parallel group |
|---|---|---|---|
| All 5 executing systems | None — no tokio, no rayon, no thread spawn, no channel, no block_on. | — | None. Binary is pure CPU, single-threaded except when ParallelSchedule would dispatch. |

Clean — no external concurrency.

---

### 2.3 `examples/hello_companion` (via `build_app` + plan executor)

System set is **identical to the editor's** (`sys_sim`, `sys_move`, `sys_refresh_los`; `sync`-stage systems silently dropped) — both go through [`astraweave_core::ecs_adapter::build_app`](../../astraweave-core/src/ecs_adapter.rs#L169-L227) at [main.rs:431,529 etc.](../../examples/hello_companion/src/main.rs) — the binary also calls `build_app(w, 0.25)` in multiple setup paths.

Inventory tables are identical to §2.1's (see above). Only differences are driver + tick count, not system topology:

- Driver: `app = app.run_fixed(1)` at [main.rs:392,464](../../examples/hello_companion/src/main.rs#L392).
- Tick count: **20 ticks total** across each demo run.
- External concurrency: several `tokio::runtime::Runtime::new()` sites for LLM modes at [main.rs:920,942](../../examples/hello_companion/src/main.rs#L920) (audit §1.1 item 5) — these run *outside* the ECS schedule's tick loop (during plan generation), not concurrently with it.

#### Short-duration caveat

20 ticks × 2-system stage × disjoint-pair-size-2 is a total of ~40 system invocations and ~20 rayon `scope` dispatches. At the per-tick workload of `sys_sim` (cooldown decay) and `sys_move` (≤ small-N moves), each is sub-microsecond. Measurement noise from process startup + driver build alone will be orders of magnitude larger than any parallelism win. Including hello_companion in a measurement experiment is not useful.

---

## Phase 3 — Synthesis

### 3.1 Shortlist

Ranking by: (1) stages with multi-system potential, (2) total disjoint-pair count, (3) absence of external-concurrency risk, (4) sufficient tick count to measure above noise.

| Rank | Binary | Stages with multi-system potential | Total disjoint pairs | External risk | Recommended as next experiment? |
|---|---|---|---|---|---|
| 1 | `examples/ecs_ai_showcase` | 1 (`simulation`: 3 systems) | 1 (`movement_system` ↔ `combat_system`) | None | **Yes** |
| 2 | `tools/aw_editor` | 1 (`simulation`: 2 systems) | 1 (`sys_sim` ↔ `sys_move`) | High — 3 concurrent tokio runtimes, file watcher thread, build manager thread already active in steady state (audit §1.1) | No — see §3.2 |
| 3 | `examples/hello_companion` | 1 (`simulation`: 2 systems) | 1 (`sys_sim` ↔ `sys_move`) | Low for the sim itself; per-run LLM tokio runtimes are sequential to the sim | No — 20 ticks is too short to measure |
| 4 | `examples/profiling_demo` | 0 (1 system per executing stage, after accounting for dropped stages) | 0 | Low | Already audited; no parallelism to measure |
| 5 | `examples/ecs_ai_demo` | 0 | 0 | Low | Too trivial |
| 6 | `examples/scripting_playground` | 0 | 0 | Low | Too trivial |
| 7 | `examples/scripting_advanced_demo` | 0 | 0 | Low | Too trivial |
| — | `examples/veilweaver_demo` | n/a | n/a | n/a | No schedule to measure |

**Recommended**: `examples/ecs_ai_showcase` as the next experiment. **Not recommended**: everything else, each with a stated reason above.

### 3.2 Editor deep-dive

The editor's simulation schedule lives in [`tools/aw_editor/src/runtime.rs`](../../tools/aw_editor/src/runtime.rs) as a `SimulationApp` (an alias for `astraweave_ecs::App`) owned by the `EditorRuntime` struct at [runtime.rs:457](../../tools/aw_editor/src/runtime.rs#L457). It is constructed by `build_app(sim_world, fixed_dt)` at [runtime.rs:565](../../tools/aw_editor/src/runtime.rs#L565) and advanced via `app.run_fixed(1)` at [runtime.rs:691](../../tools/aw_editor/src/runtime.rs#L691).

**Where does the editor run a schedule, and how often?** Only when `RuntimeState::Playing` (see the enum at [runtime.rs:16-26](../../tools/aw_editor/src/runtime.rs#L16-L26)). The `tick` method at [runtime.rs:623-652](../../tools/aw_editor/src/runtime.rs#L623-L652) uses an accumulator pattern with a fixed `60 Hz` step (`self.fixed_dt`) — accumulate real-time `dt`, consume in 16.67 ms quanta, call `run_fixed_steps(steps)` which in turn calls `run_fixed(1)` per quantum. On a fast machine the editor's UI thread may run the schedule 1-5 times per redraw; on a slow one, 0 times per redraw plus backlog. Short bursts of up to 5× the fixed step are clamped at [runtime.rs:636](../../tools/aw_editor/src/runtime.rs#L636).

**Main UI loop or worker thread?** The UI main loop. The `sim_app` is owned by `EditorRuntime` which is owned by the editor's app state; `tick` is called from inside the egui/winit update. There is no background worker driving the schedule. This is important for the oversubscription analysis below: the rayon scope that `ParallelSchedule` would dispatch happens *on* the UI thread and blocks the UI thread for its duration.

**Interactions with the tokio runtimes identified in audit §1.1?** The editor hosts three concurrent tokio runtimes at steady state:

1. The egui/winit UI's embedded tokio runtime (if any — depends on egui version and features).
2. A `Runtime::new()` + `block_on` created inside a `std::thread::spawn` worker at [main.rs:6622-6661](../../tools/aw_editor/src/main.rs#L6622-L6661) for blend-asset decomposition.
3. Viewport renderer bridges at [viewport/renderer.rs:413,1026,1068,1371](../../tools/aw_editor/src/viewport/renderer.rs) — four separate `block_on` sites.

None of these runtimes share the sim_app's `World`. The decomposition worker operates on blend files; the viewport renderer operates on GPU resources; neither touches `CPos`, `CDesiredPos`, `CCooldowns`, or `Resource<World>` that `sys_sim` and `sys_move` need. This is good — it means a race condition between `ParallelSchedule`'s worker threads and the existing tokio runtimes is structurally unlikely.

What is **not** good is the thread-count arithmetic. Rayon default = `num_cpus` workers. Each tokio runtime = `num_cpus` workers. Three live tokio runtimes + rayon at peak = `4 × num_cpus` worker threads alive. Adding `ParallelSchedule` dispatch adds work to rayon's existing pool (it does not spawn a new pool — see [parallel.rs:305](../../astraweave-ecs/src/parallel.rs#L305)), so the count doesn't grow. But the dispatch happens from the UI thread, which is one of those `4 × num_cpus` workers effectively competing for cores. Short-duration blocks on the UI thread from a 2-system rayon::scope should stay sub-millisecond given the `sys_sim` / `sys_move` workload; longer blocks would show as UI hitches.

**Is there an obvious split between "editor UI systems" and "simulation preview systems"?** Yes. The editor does not register its own ECS systems on the simulation `App`. The UI operates on egui state; the simulation operates on `sim_app.world`. The sim schedule is entirely composed of `build_app`-provided systems, which are purely simulation. There is no editor-UI-system in the sim schedule that would need to be marked exclusive. If `ParallelSchedule` were adopted for the editor's sim, every system in the schedule could annotate its access cleanly; no split-schedule design is needed.

**Why it's not the top recommendation anyway**: the simulation stage has only 2 systems (1 disjoint pair) vs ecs_ai_showcase's 3 (also 1 disjoint pair, but one more system to benchmark scheduler overhead against). The per-tick work in `sys_sim` + `sys_move` is smaller than the per-tick work in ecs_ai_showcase's simulation stage (which includes AI behavior state machines and combat event processing). Smaller per-tick work amplifies `build_groups` allocation tax as a fraction of the tick. Also, the editor's UI thread is a poor measurement surface — any egui redraw hitch during the tick window pollutes the sample. ecs_ai_showcase is a clean push-driven loop with no UI.

### 3.3 What the inventory tells us about the engine

Of eight binaries that touch `astraweave_ecs::App`, only two have any system actually running in parallel with another. Both of those two get that property from a single 2- or 3-system `simulation` stage, and both have exactly one disjoint-access pair in that stage. The engine's `ParallelSchedule` is built for stage densities higher than this; nothing in the workspace exercises it at its designed capacity.

The shape of a binary that would be a *great* candidate for parallel scheduling looks like: at least 4 systems in one stage; 2-3 of them writing to disjoint component sets; no shared resource conflict (e.g. not all writing into the same `Events`); no `futures_executor::block_on` inside a system body. None of AstraWeave's current binaries fit that profile. `profiling_demo`'s design intent (separating AI perception, AI planning, movement, physics, rendering into their own stages) fits the *engine's* architectural model but explicitly puts one system per stage, which is the opposite of what `ParallelSchedule` parallelises — `ParallelSchedule` parallelises within a stage, not across.

The secondary finding — `Schedule::add_system` silently dropping systems for non-existent stages — means the engine has been operating with fewer systems than it appears to register. `profiling_demo`'s `ai_perception_system` (PRE_SIMULATION) and `cleanup_system` (POST_SIMULATION) never ran; `ecs_adapter::build_app`'s `sys_bridge_sync` and `sys_sync_to_legacy` (sync) never ran; `ecs_ai_showcase`'s `stats_display_system` (post_simulation) never ran. The prior experiment's result is still valid (the dropped systems are either no-ops or output-only), but this is a latent correctness hazard that the audit did not surface. Not fixing it here — out of scope — see §4.

### 3.4 Next experiment recommendation

**Target binary**: `examples/ecs_ai_showcase`.

**Measurement focus stages**: only `simulation`. Perception and ai_planning each have 1 system and will execute on the single-system fast path unchanged. The simulation stage has 3 systems grouped into 2 rayon rounds (group 0: `ai_behavior_system` alone; group 1: `movement_system` + `combat_system` in parallel) — this is the only place `rayon::scope` would actually dispatch.

**Expected Δ range**: Small. `movement_system` does `entities.len()` lookups + one `pos += vel * dt` per entity; at 6 entities (5 enemies + 1 player per [main.rs:506](../../examples/ecs_ai_showcase/src/main.rs#L506)) this is likely sub-microsecond per tick. `combat_system`'s per-tick cost is proportional to damage events emitted (usually ≤5 per tick). Fanning these two out across two cores gains tens to hundreds of nanoseconds per tick in the best case. With a 300-tick push loop that's ~30-100 µs over the full run, below the noise floor observed in the prior experiment (~30 % spread at sub-second run durations). To make the measurement legible:

- Raise the entity count. Add 100-1000 enemies instead of 5; parameterise via a CLI arg similar to `profiling_demo`.
- Raise the tick count to 5000-10000 for a multi-second run that drowns out startup jitter.

**Annotation risk / edge cases**:

- `Resource<Events>` is a single TypeId shared by all event types in ecs_ai_showcase. Every system that touches any event will conflict via `.writes::<Events>()`. This is already baked into the conflict analysis above (2 conflict, 1 disjoint pair). The correct annotations are: `ai_behavior_system` → `.reads::<AIAgent>().reads::<Position>().writes::<Velocity>().writes::<Events>()`; `movement_system` → `.reads::<Velocity>().reads::<GameTime>().writes::<Position>()`; `combat_system` → `.reads::<Events>().writes::<Health>().writes::<Events>().writes::<GameStats>()`. The doubled `.writes::<Events>()` on combat_system is not a problem — `writes` is a HashSet insert — but worth flagging that combat both reads and writes Events.
- `ai_behavior_system` reads `AIAgent`'s `target` field (an `Entity`) and then reads `Position` of that other entity. If two agents target each other and are in the same rayon task, the `world.get::<Position>(other)` call is safe (no mutation), but if `ai_behavior` were ever written to mutate `Position` of the target, it would be a cross-entity write. Currently it only mutates `Velocity` of the *calling* entity, so annotations hold.
- Silently-dropped `stats_display_system` should be addressed before the experiment, either by (a) adding a `post_simulation` stage in `App::new()` or (b) re-registering the system in an existing stage. If left as-is, the experiment measures a binary whose stats panel is dead. Flagging for the next task.

**What to skip**: `hello_companion` and `aw_editor` as measurement targets in the next experiment. `hello_companion` is too short-lived to produce signal above noise; `aw_editor` sits inside a UI thread already surrounded by multiple tokio runtimes, which contaminates measurement. The showcase is clean CPU, push-loop, no UI.

If after running the showcase experiment the parallelism Δ is still within noise — plausible, given only one disjoint pair and tiny per-system workload — the right next step is **not** to keep searching for binaries to adopt `ParallelSchedule` on. It is to construct a purpose-built benchmark binary with 6-10 systems per stage and a realistic entity count, matching the "great candidate profile" in §3.3. That is a task-spec request, out of scope here.

---

## 4 Appendix — issues found during inventory (not fixed; flagged)

> **Superseded 2026-04-18**: bullets 1–3 below describe the `Schedule::add_system` silent-drop bug and its consequences. The bug was fixed the same day in `docs/audits/schedule_stage_fix_2026-04-18.md`. `App::new()` now creates all 8 canonical stages; the dropped systems in `profiling_demo`, `ecs_adapter::build_app`, and `ecs_ai_showcase` now execute. The bullets are kept for historical context; do not act on them.

- **Schedule silently drops systems for unregistered stages.** [astraweave-ecs/src/lib.rs:707-711](../../astraweave-ecs/src/lib.rs#L707-L711). `App::new()` creates 5 stages; `SystemStage::PRE_SIMULATION` / `POST_SIMULATION` and the `sync` stage used by `astraweave-core::ecs_adapter::build_app` are not among them. Affects `profiling_demo` (2 systems dropped), `ecs_adapter::build_app` consumers including editor and hello_companion (2 systems dropped), and `ecs_ai_showcase` (1 system dropped — the stats panel). A fix would either add `PRE_SIMULATION` / `POST_SIMULATION` / `sync` to the default stage list, or make `add_system` create a stage on first reference, or log a warning. Not fixing here — out of scope per brief. This is the single most impactful issue surfaced by the inventory.
- **`build_app` system registrations target a non-existent `"sync"` stage.** [astraweave-core/src/ecs_adapter.rs:221,223](../../astraweave-core/src/ecs_adapter.rs#L221). Same issue as above; worth a direct callout because it means the "bridge sync" and "sync to legacy world" behaviours the code claims to provide are not running. Anything that reads legacy `World` pose/hp/ammo values expecting them to reflect recent ECS changes sees stale data. Diagnostic: grep for `World` resource reads after `run_fixed` / `schedule.run` — they will return data that is 1 tick older than expected. Not fixing here.
- **`ecs_ai_showcase`'s stats panel is dead.** [main.rs:550](../../examples/ecs_ai_showcase/src/main.rs#L550). `stats_display_system` registered to `"post_simulation"` never runs. User sees no stats prints despite the panel claiming to emit them every 60 ticks. Not fixing here.
- **The audit at `docs/audits/job_system_audit_2026-04-18.md` §1.2 lists `App::new`'s stages as five stages** — correctly — but does not flag the mismatch between that list and the `SystemStage` constants at [lib.rs:94-102](../../astraweave-ecs/src/lib.rs#L94-L102) which define seven. The prior audit identified the stage-count gap at §1.2 paragraph 3 as "consumers must `add_stage` them manually if used" but did not identify that *none* of the consumers do, and that `add_system` silently drops on miss. This inventory extends the audit's finding in that direction.

---

## Evidence index

Every file read, grouped by binary. (Test files excluded; one entry per file regardless of how many line ranges were consulted.)

**Engine core**

- `astraweave-ecs/src/lib.rs` (App::new stages, Schedule::add_system drop behaviour, SystemStage constants).
- `astraweave-ecs/src/parallel.rs` (build_groups, run_group_parallel, conflict analysis logic).
- `astraweave-core/src/ecs_adapter.rs` (build_app, sys_sim, sys_move, sys_bridge_sync, sys_sync_to_legacy, sys_refresh_los).
- `astraweave-ai/src/ecs_ai_plugin.rs` (AiPlanningPlugin, sys_ai_planning).
- `astraweave-scripting/src/lib.rs` (ScriptingPlugin, script_system).

**Binaries**

- `examples/profiling_demo/src/main.rs` (existing system registrations; confirmed via prior experiment).
- `examples/ecs_ai_showcase/src/main.rs` (main loop, system bodies for all 6 declared systems, setup_world).
- `examples/hello_companion/src/main.rs` (two build_app call sites, run_fixed loop).
- `examples/ecs_ai_demo/src/main.rs` (App + AiPlanningPlugin + inline move_system).
- `examples/scripting_playground/src/main.rs` (App + ScriptingPlugin, schedule.run loop).
- `examples/scripting_advanced_demo/src/main.rs` (App + ScriptingPlugin, #[tokio::main]).
- `examples/veilweaver_demo/src/main.rs` (App used as component store, no schedule driver).
- `tools/aw_editor/src/main.rs` (absence of astraweave_ecs imports; decomposition worker location).
- `tools/aw_editor/src/runtime.rs` (EditorRuntime, sim_app field, build_app call site, tick accumulator, run_fixed loop).

---

## Open questions

What this inventory could not determine from code alone.

1. **Does `ecs_ai_showcase`'s event emission actually produce output?** The `stats_display_system` being silently dropped means the user-facing confirmation that events fire (via the stats panel) is gone. Can only be confirmed by running the binary with eprintln! instrumented around `events.send()` call sites, or by adding a `"post_simulation"` stage to `App::new()` and re-running.
2. **Does the editor ever re-register systems for the sim_app after `build_app`?** This inventory reads `runtime.rs` carefully but does not exhaustively search every editor panel for `sim_app.add_system` calls. A workspace-wide `rg "sim_app\.add_system|sim_app.*add_plugin"` returned zero additional hits, but dynamic registration via a `Box<dyn Fn(&mut App)>` pattern wouldn't show up; the editor could theoretically be doing that somewhere. Unlikely but not ruled out without runtime inspection.
3. **Are `sys_bridge_sync` and `sys_sync_to_legacy` being silently dropped actually causing bugs users have reported?** Cannot tell from code. A downstream system that reads the legacy `World` resource expecting `CPos` / `CHealth` / `CAmmo` mirror data would see stale values — but whether any user-facing behaviour depends on that is a simulation-correctness question, not a scheduling one. Out of scope but worth flagging to whoever fixes §4 bullet 1.
4. **Does `scripting_playground`'s `script_system` have disjoint access that would make adding a second system to SIMULATION parallelizable?** Not inventoried in depth because the binary has only 1 system and is out of scope. If a future task adds a second script-adjacent system, the answer is in `astraweave-scripting/src/lib.rs` around line 585.

---

## Verification hooks

Three commands a reviewer can run to independently confirm the ranking.

```bash
# 1. Confirm only seven main.rs files import astraweave_ecs — and one more (runtime.rs) hosts the sim App.
# Expect: 7 hits in main.rs, 1 in tools/aw_editor/src/runtime.rs.
rg -l 'use astraweave_ecs|astraweave_ecs as ecs' --glob '**/main.rs' --glob '!**/archive/**' --glob '!**/target/**'
rg -l 'SimulationApp|astraweave_ecs::App' tools/aw_editor/src --glob '!**/target/**'

# 2. Confirm that `schedule.run` and `run_fixed` are the only tick drivers.
# Expect exactly one per push-driver binary (profiling_demo, ecs_ai_showcase, scripting_*) and two per fixed-step binary (hello_companion, ecs_ai_demo use run_fixed but call it from a loop).
rg -n 'schedule\.run\(|\.run_fixed\(' --glob '**/main.rs' --glob '!**/archive/**' --glob '!**/target/**'

# 3. Confirm that stages outside the App::new default set are in use — the silent-drop bug from §4.
# Expect: hits for "pre_simulation", "post_simulation", "sync" in build_app + ecs_ai_showcase + profiling_demo.
rg -n 'add_system\s*\(\s*"(pre_simulation|post_simulation|sync)"|SystemStage::(PRE_SIMULATION|POST_SIMULATION)' --glob '!**/target/**' --glob '!**/archive/**' --glob '!**/tests/**'
```

---

**Report status**: Inventory complete. No code changes. No experiments run. Recommendation at §3.4 is a recommendation only; the brief forbids implementation.
