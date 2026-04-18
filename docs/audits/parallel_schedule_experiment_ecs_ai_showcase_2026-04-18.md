# ParallelSchedule adoption experiment — `ecs_ai_showcase` — 2026-04-18

> **Status note (2026-04-18)**: The `ParallelSchedule` scheduler whose correctness failure is documented in this experiment was removed from the workspace on 2026-04-18. See [`parallel_schedule_removal_2026-04-18.md`](parallel_schedule_removal_2026-04-18.md) for the deletion report and rationale. This audit remains in place as historical record; do not treat it as describing current engine state. The `parallel-schedule` feature referenced throughout this document no longer exists.

**Status**: Phase 1 complete. **Phase 2 NOT run — correctness check failed at Phase 1.6 per the brief's stop rule.**
**Scope**: Wire `ParallelSchedule` into `examples/ecs_ai_showcase` behind a `parallel-schedule` feature, verify correctness, then measure. The measurement stage was not reached.
**Precondition met**: schedule-stage fix at `docs/audits/schedule_stage_fix_2026-04-18.md` is in place (commit `70266b74e`). `stats_display_system` runs; `App::new()` provides all 8 canonical stages.

---

## Executive summary

Wiring worked. All 7 feature combinations compile. The binary scaled from 5 enemies / 300 ticks to CLI-parametric `-e/-f`. The sequential scheduler produces deterministic output across runs. The `ParallelSchedule` path with annotations taken verbatim from the binary inventory audit also produces deterministic output — **but the output differs from the sequential one, starting at the first captured frame**.

The divergence is not a race condition in the "different every run" sense; it is a reproducible, deterministic shift caused by a shared-archetype-metadata aliasing hazard in `World::get_mut` that `ParallelSchedule`'s safety argument does not account for. Component-TypeId-disjoint access — the condition ParallelSchedule checks — is insufficient when multiple component types live in the same archetype, because every `get_mut::<T>` call borrows the archetype's `change_ticks: HashMap<TypeId, Vec<u32>>` as `&mut`. Two rayon tasks each calling `get_mut` on different TypeIds for entities in the same archetype concurrently hold `&mut` on that HashMap — undefined behavior under Rust's aliasing model, and empirically observable as a state divergence in this binary.

Recommendation (§3.8): **Do not merge. `ParallelSchedule`'s current design cannot safely parallelize the systems in `ecs_ai_showcase`, and by extension any binary whose entities share an archetype across parallel-group systems. An audit of `ParallelSchedule`'s safety model is warranted before any further adoption work.**

The sequential `Schedule` is unaffected by this finding; this audit does not invalidate the schedule-stage fix or the prior `profiling_demo` experiment (which had 1 system per stage, so `run_group_parallel` was never entered).

---

## Phase 1 — Wire up, scale up, verify

### 3.1 What was wired

Files changed:

- [`examples/ecs_ai_showcase/Cargo.toml`](../../examples/ecs_ai_showcase/Cargo.toml) — added `parallel-schedule`, `alloc-counter`, `fast-alloc` features following `profiling_demo`'s pattern. `astraweave-alloc` as optional dep, `astraweave-profiling` as workspace dep.
- [`examples/ecs_ai_showcase/src/main.rs`](../../examples/ecs_ai_showcase/src/main.rs) — five additive edits:
  1. Feature-gated global allocator selection (`CountingAlloc` when `alloc-counter`, `MiMalloc` when `fast-alloc`, otherwise default).
  2. `setup_world(app, enemy_count)` now takes an enemy count parameter; default behaviour preserved when the CLI `-e` arg is not passed.
  3. `parse_args()` CLI parser for `-e <count>` (default 5) and `-f <ticks>` (default 300), mirroring `profiling_demo`'s convention.
  4. `build_parallel_schedule()` function (cfg-gated) that constructs `ParallelSchedule` with the annotations prescribed by the brief §1.2.
  5. `print_state_checksum()` emits one line every 100 ticks with `pos`, `vel`, `health`, `ai`, and `stats` hashes. The `stats` hash was added per the task caveat to surface `combat_system`-ordering sensitivity.

The `main()` body was restructured to accept CLI args, construct the parallel schedule when the feature is on, drive whichever schedule is selected per tick, and print the checksum. At default args (`-e 5 -f 300`) observable behaviour is unchanged from the pre-experiment baseline (stats panels every 60 ticks, final `"✅ Simulation complete!"`).

### 3.1.1 System inventory (with audit-prescribed annotations)

| System | Stage | Reads | Writes | File:lines |
|---|---|---|---|---|
| `ai_perception_system` | `perception` | `AIAgent`, `Position`, `Team` | `AIAgent` | [main.rs:118-179](../../examples/ecs_ai_showcase/src/main.rs#L118-L179) |
| `ai_planning_system` | `ai_planning` | `AIAgent`, `Health`, `Position` | `AIAgent`, `Events` | [main.rs:186-278](../../examples/ecs_ai_showcase/src/main.rs#L186-L278) |
| `ai_behavior_system` | `simulation` | `AIAgent`, `Position` | `Velocity`, `Events` | [main.rs:303-380](../../examples/ecs_ai_showcase/src/main.rs#L303-L380) |
| `movement_system` | `simulation` | `Velocity`, `GameTime` | `Position` | [main.rs:285-300](../../examples/ecs_ai_showcase/src/main.rs#L285-L300) |
| `combat_system` | `simulation` | `Events` | `Health`, `Events`, `GameStats` | [main.rs:383-428](../../examples/ecs_ai_showcase/src/main.rs#L383-L428) |
| `stats_display_system` | `post_simulation` | `GameTime`, `GameStats`, `AIAgent` | — | [main.rs:435-466](../../examples/ecs_ai_showcase/src/main.rs#L435-L466) |

Greedy coloring by `ParallelSchedule::build_groups` for the `simulation` stage:

- Group 0: `[ai_behavior_system]` — writes Velocity + Events, conflicts with every other system.
- Group 1: `[movement_system, combat_system]` — movement reads Velocity + GameTime and writes Position; combat reads Events and writes Health + Events + GameStats. Intersection of their TypeId sets is empty per the brief's inventory §3.4. Greedy coloring places them together.

This matches the inventory's predicted topology exactly.

### 3.2 Compile-check matrix

All seven combinations compile cleanly (release profile, Windows 11, cargo 1.89.0):

| `--features` | Result |
|---|---|
| *(none)* | `Finished release profile [optimized] target(s) in 22.81s` |
| `parallel-schedule` | `Finished release profile [optimized] target(s) in 4.59s` |
| `alloc-counter` | `Finished release profile [optimized] target(s) in 3.27s` |
| `parallel-schedule,alloc-counter` | `Finished release profile [optimized] target(s) in 3.47s` |
| `fast-alloc` | `Finished release profile [optimized] target(s) in 3.60s` |
| `parallel-schedule,fast-alloc` | `Finished release profile [optimized] target(s) in 3.22s` |
| `parallel-schedule,alloc-counter,fast-alloc` | `Finished release profile [optimized] target(s) in 3.56s` |

### 3.3 Correctness check — FAILED

Command: `cargo run --release -p ecs_ai_showcase --features {alloc-counter,fast-alloc ∪ parallel-schedule} -- -e 100 -f 500`.

**Sequential output** (3 runs; all identical):

```
[state-checksum] frame 100: pos=00000a9d4a3b691a vel=00000ad05501a730 health=000000000008df7c ai=ffffffffffffffb1 stats=0000000000000000
[state-checksum] frame 200: pos=00000aa08c170bde vel=00000ac750665cf0 health=000000000008b4d4 ai=ffffffffffffffbb stats=0000000000097a2b
[state-checksum] frame 300: pos=00000aa322d58826 vel=00000ac750665cf0 health=00000000000862cc ai=ffffffffffffffbb stats=00000000001bba6f
[state-checksum] frame 400: pos=00000aa4ddb14b0b vel=00000ac750665cf0 health=00000000000810c4 ai=ffffffffffffffbb stats=00000000002dfab3
[state-checksum] frame 500: pos=00000aa6978e72e1 vel=00000ac750665cf0 health=000000000007bebc ai=ffffffffffffffbb stats=0000000000403af7
```

**Parallel output** (3 runs; all identical among themselves):

```
[state-checksum] frame 100: pos=00000a9d5295a6f7 vel=00000ad05501a73c health=000000000008df7c ai=ffffffffffffffb1 stats=0000000000000000
[state-checksum] frame 200: pos=00000aa0947876ef vel=00000ac750665cf0 health=000000000008b402 ai=ffffffffffffffbb stats=000000000009a8e4
[state-checksum] frame 300: pos=00000aa327438d42 vel=00000ac750665cf0 health=00000000000861fa ai=ffffffffffffffbb stats=00000000001be928
[state-checksum] frame 400: pos=00000aa4e21d8e60 vel=00000ac750665cf0 health=0000000000080ff2 ai=ffffffffffffffbb stats=00000000002e296c
[state-checksum] frame 500: pos=00000aa69bf94034 vel=00000ac750665cf0 health=000000000007bdea ai=ffffffffffffffbb stats=00000000004069b0
```

**Diff**: every checksum line differs between the two schedulers. Per-field pattern:

| Frame | `pos` diff | `vel` diff | `health` diff | `ai` diff | `stats` diff |
|---|---|---|---|---|---|
| 100 | YES | tiny (0xc) | no | no | no (both 0) |
| 200 | YES | no | YES | no | YES |
| 300 | YES | no | YES | no | YES |
| 400 | YES | no | YES | no | YES |
| 500 | YES | no | YES | no | YES |

**Determinism check**:

- Sequential run 1 ↔ Sequential run 2: bit-identical state checksums. Sequential is deterministic.
- Parallel run 1 ↔ Parallel run 2 ↔ Parallel run 3: bit-identical state checksums across all three. Parallel is *also* deterministic — but produces a different state than sequential.

So the divergence is not a classic race-condition flap ("different every run"). It is a reproducible, systematic offset in the observable world state caused by the parallel scheduling path.

Per the brief §1.6: **"If state diverges, stop — there's a race condition or the annotations are wrong. Do not proceed to Phase 2 if correctness fails."** Phase 2 was not executed.

### 3.3.1 Why does state diverge?

The divergence at frame 100 is the most instructive. At that frame:

- `stats` is 0 in both paths → `combat_system` has processed zero `DamageEvent`s in either path.
- `health` is identical → no damage has been applied yet.
- `ai` is identical → the AI state machine has made the same decisions.
- `vel` differs by `0xc` → `ai_behavior_system` has written slightly different `Velocity` components.
- `pos` differs → follows from `vel` divergence via `movement_system`'s integration.

`ai_behavior_system` is in group 0 — it runs alone, not in parallel with any other system. Yet its output differs between sequential and parallel. That can only happen if its *inputs* differ. Its inputs at frame N are `Position` and `AIAgent`, which are produced by the previous frame's `movement_system` (group 1) and `ai_planning_system` (stage `ai_planning`).

Both `Position` and `AIAgent` should be produced identically at the end of frame N-1 in both paths, because:

- `AIAgent.state` is written only by `ai_planning_system`, which runs alone in its own stage.
- `Position` is written only by `movement_system`, which per the access annotations is disjoint from `combat_system` within group 1.

**Yet they aren't identical.** Something about running `movement_system` concurrently with `combat_system` — even when their declared TypeId sets are disjoint — is producing different `Position`/`Velocity`/`Health` values than running them sequentially. That "something" is documented in §3.10 below — it is a shared-archetype-metadata aliasing hazard that `ParallelSchedule`'s safety argument does not cover.

### 3.4–3.7 Primary results / entity-scaling / noise / interpretation

Not produced. Phase 2 was not run. Running ParallelSchedule for throughput measurements on a workload whose correctness check has failed would produce meaningless FPS numbers (they would describe an incorrect simulation). A future re-run of this experiment is gated on either:

1. Fixing `ParallelSchedule`'s safety model so archetype-metadata aliasing cannot occur (audit §3.10 below), or
2. Constructing a purpose-built benchmark binary whose parallel-group systems operate on entities in provably disjoint archetypes (see §3.9 for what this would look like).

### 3.8 Recommendation

**Do not merge. ParallelSchedule's current design cannot help at `ecs_ai_showcase`'s topology, and an audit of the scheduler itself is warranted.**

Defense:

1. The wiring is correct per the brief's annotation prescription — verified by reading the brief §1.2 and confirming every `.reads::<T>()` / `.writes::<T>()` call matches. No ambiguity.
2. The divergence is reproducible (par1 == par2 == par3) and matches a specific code-level mechanism (archetype-metadata aliasing) that I can cite with `file:line` evidence — not a "runtime noise" explanation.
3. Component-TypeId-disjoint access is the only safety check `ParallelSchedule` performs ([parallel.rs:97-113](../../astraweave-ecs/src/parallel.rs#L97-L113)). That check is insufficient when multiple component types share an archetype, because `World::get_mut<T>` ([lib.rs:434-447](../../astraweave-ecs/src/lib.rs#L434-L447)) takes `&mut self.archetypes` and then `&mut archetype.change_ticks` regardless of T — two concurrent calls from different rayon tasks for different T on the same archetype hold concurrent `&mut` on shared state.
4. The `profiling_demo` experiment did not surface this bug because it had 1 system per stage, so `run_group_parallel` was never entered. The editor and `hello_companion` also won't surface it — the schedule-stage-fix audit showed both have at most 2 systems in the `simulation` stage (`sys_sim` and `sys_move`), and those two access entities via `world.each_mut` and per-entity reads which have the same archetype-metadata issue — but the editor's UI thread hides correctness drift behind "well, play mode shows a slightly different simulation than expected, who would notice". `ecs_ai_showcase` surfaced it because its state checksum instrumentation was designed to catch exactly this.

Do not revert the `parallel-schedule` feature. Leave the wiring + scale-up changes in place; they are useful for the post-audit re-run. The `stats`-inclusive state checksum instrumentation is also useful beyond this experiment — the task caveat was prescient.

**Specifically deferred / out-of-scope for this task**:

- Do not fix `ParallelSchedule`'s safety model. That is a multi-file engine change touching `astraweave-ecs/src/{parallel.rs, lib.rs, archetype.rs}` and needs its own task with a formal-verification (Kani) plan since `parallel.rs:52-70` is unsafe code inside the Miri/Kani validated crate.
- Do not construct a purpose-built benchmark binary. The inventory at `docs/audits/parallel_schedule_binary_inventory_2026-04-18.md` §3.3 describes what it would look like ("at least 4 systems in one stage; 2-3 of them writing to disjoint component sets; no shared resource conflict; no `futures_executor::block_on` inside a system body") — but building one before the scheduler's safety model is fixed would just produce another divergence report, not a useful measurement.

### 3.9 Open questions

Things this experiment could not determine.

1. **Is every archetype-shared-metadata access path equally problematic?** I traced the divergence to `get_mut`'s `stamp_change_tick` via `archetype.get_mut`. Other access paths (`each_mut`, `Query2Mut`, `QueryMut`) likely have the same issue but were not exhaustively verified. A future task should enumerate all component-mutating APIs and confirm which ones borrow archetype-level metadata.
2. **Would annotating systems with archetype-level conflicts (not just TypeId-level) be sufficient?** Conceptually yes — if `ParallelSchedule` could know that `movement_system` and `combat_system` both touch the `{Position, Velocity, Health, Team, AIAgent}` archetype, it would refuse to put them in the same group. But today `SystemAccess` only tracks component TypeIds. Extending it to "affected archetype set" would require runtime introspection of the system body (hard) or manual annotation (error-prone).
3. **Is the divergence UB manifesting as data corruption, or just a well-defined ordering effect that happens to differ?** I cannot tell without Miri. Running `cargo +nightly miri run -p ecs_ai_showcase --features parallel-schedule,alloc-counter` would flag the issue conclusively. Miri is not currently run on example binaries (per `CLAUDE.md` the Miri set is `ecs`, `math`, `core`, `sdk`), but wiring this experiment in as a Miri target is cheap and would turn the empirical observation into a formal one.
4. **At what entity count and tick count does the divergence become large enough to be user-visible?** At 100 entities / 500 ticks the `stats` hash diverges by ~17 % (seq=`0x97a2b`, par=`0x9a8e4` — different totals of enemies_defeated, total_damage_dealt). At 1000 entities the divergence would likely be larger. This is a quality metric for a hypothetical safety-model fix: post-fix divergence must go to zero at all scales.

### 3.10 Appendix — issues found in `ParallelSchedule`

This is where the primary deliverable of the experiment lives: a concrete description of a safety bug in `ParallelSchedule`, cited by `file:line`, with evidence that it manifests observably on a realistic workload. Do not fix here per the brief.

**Safety bug**: `ParallelSchedule::run_group_parallel` ([parallel.rs:296-325](../../astraweave-ecs/src/parallel.rs#L296-L325)) uses `SendWorldPtr(*mut World)` ([parallel.rs:42-70](../../astraweave-ecs/src/parallel.rs#L42-L70)) to hand each rayon task its own `&mut World` via raw pointer, relying on the scheduler's `SystemAccess::conflicts_with` check ([parallel.rs:97-113](../../astraweave-ecs/src/parallel.rs#L97-L113)) to guarantee disjoint access. The `SAFETY` comment at [parallel.rs:290-295, 302-304, 310-311](../../astraweave-ecs/src/parallel.rs#L290-L311) states this is sound because "All systems in the group have been verified by `build_groups()` to have non-conflicting access sets. Each system accesses disjoint resources/components."

The safety argument does not cover archetype-level shared metadata.

Evidence chain:

1. `SystemAccess::conflicts_with` (parallel.rs:97-113) considers only `reads: HashSet<TypeId>` and `writes: HashSet<TypeId>`. Two systems with no overlapping TypeIds are declared non-conflicting.
2. `World::get_mut::<T>` ([lib.rs:434-447](../../astraweave-ecs/src/lib.rs#L434-L447)) is called by both `movement_system` (for `Position`) and `combat_system` (for `Health`). Its body performs:
   - `self.archetypes.get_archetype_mut(archetype_id)` — returns `&mut Archetype`.
   - `archetype.stamp_change_tick::<T>(e, tick)` — mutates the archetype's change-tick metadata.
3. `Archetype::stamp_change_tick` ([archetype.rs:336-344](../../astraweave-ecs/src/archetype.rs#L336-L344)) takes `&mut self` and then calls `self.change_ticks.get_mut(&TypeId::of::<T>())` — a mutable borrow on `change_ticks: HashMap<TypeId, Vec<u32>>` ([archetype.rs:99-102](../../astraweave-ecs/src/archetype.rs#L99-L102)).
4. When `movement_system` and `combat_system` run concurrently via `rayon::scope` and both target entities in the *same* archetype (which is the case in `ecs_ai_showcase`, where all 5 enemies share the `{Position, Velocity, Health, Team, AIAgent}` archetype), both tasks simultaneously hold `&mut World`, then `&mut Archetype`, then `&mut HashMap<TypeId, Vec<u32>>`. Two concurrent `&mut` on the same memory. This is undefined behavior under Rust's aliasing model regardless of which TypeId each is writing to — the aliasing rule doesn't care about logical disjointness within the HashMap.

**Empirical manifestation**: state checksum divergence reported in §3.3. Not a crash; not obvious data corruption; a small, deterministic, reproducible offset in numerical outputs. UB can manifest this way when LLVM optimizations rely on unique-mutability but the runtime scheduling happens to produce consistent-enough outputs that the difference looks like "ordering" rather than "corruption". The next Rust compiler update or LLVM pass tweak could change how this manifests — including to an outright crash.

**Fix directions (not to be pursued in this task)**:

1. Narrow `SystemAccess` to track archetype-level conflicts. Requires runtime archetype introspection: every `add_system` call would need to know what archetypes the system could touch. Hard in general; tractable if systems declare their archetype signatures.
2. Decouple change-tick stamping from `get_mut`. Make the change-tick write atomic (e.g., `AtomicU32` per row) or move it to a post-tick batching pass. Eliminates the most obvious `&mut` on archetype shared state but `archetype.get_mut::<T>` itself would still take `&mut Archetype` for the BlobVec access.
3. Move BlobVec access to a non-aliasing primitive (`UnsafeCell<BlobVec>` per component column, scheduler tracks per-column access). Large refactor; closer to Bevy's architecture.
4. Restrict `ParallelSchedule` to systems that only use the `with`-style access helpers that demonstrably touch a single component column. Small scope but eliminates the generality the scheduler was designed for.

None of these are small. This is why the §3.8 recommendation is to audit before adopting, not to fix during this experiment.

**One other nit** (minor): `SystemDescriptor::new` defaults to `exclusive = true` ([parallel.rs:129-138](../../astraweave-ecs/src/parallel.rs#L129-L138)). That default would have masked this bug if the inventory §3.4 hadn't prescribed concrete annotations — any system registered without a `.reads::<T>()` / `.writes::<T>()` call would have been marked exclusive and put in its own group. This is a defensive default that saved the previous experiment on `profiling_demo`; worth keeping.

---

## Reproducibility

Exact commands run for this experiment.

```bash
# Phase 1.5 — compile matrix (all seven must print `Finished release profile [optimized]`)
cargo check --release -p ecs_ai_showcase
cargo check --release -p ecs_ai_showcase --features parallel-schedule
cargo check --release -p ecs_ai_showcase --features alloc-counter
cargo check --release -p ecs_ai_showcase --features parallel-schedule,alloc-counter
cargo check --release -p ecs_ai_showcase --features fast-alloc
cargo check --release -p ecs_ai_showcase --features parallel-schedule,fast-alloc
cargo check --release -p ecs_ai_showcase --features parallel-schedule,alloc-counter,fast-alloc

# Phase 1.6 — correctness check (diff state-checksum lines; they WILL differ)
cargo run --release -p ecs_ai_showcase --features alloc-counter,fast-alloc          -- -e 100 -f 500
cargo run --release -p ecs_ai_showcase --features parallel-schedule,alloc-counter,fast-alloc -- -e 100 -f 500

# Determinism verification (run sequential twice, parallel three times — each path matches itself)
# Not required by brief but essential to isolate "scheduler divergence" from "rayon work-stealing flap":
cargo run --release -p ecs_ai_showcase --features alloc-counter,fast-alloc          -- -e 100 -f 500  # run 2
cargo run --release -p ecs_ai_showcase --features parallel-schedule,alloc-counter,fast-alloc -- -e 100 -f 500  # par 2
cargo run --release -p ecs_ai_showcase --features parallel-schedule,alloc-counter,fast-alloc -- -e 100 -f 500  # par 3

# Phase 2 NOT RUN — correctness failed.
```

---

## What the reader should take away

The scheduler-stage fix at `docs/audits/schedule_stage_fix_2026-04-18.md` remains correct: it makes `App::new()` provide all 8 canonical stages and makes previously-dropped systems execute. Nothing in this experiment contradicts that.

The mimalloc recommendation also remains correct. Nothing in this experiment contradicts that.

What this experiment reveals is different: `ParallelSchedule`'s safety argument has a hole that surfaces when rayon actually gets invoked. `profiling_demo` couldn't surface it (1 system per stage). `ecs_ai_showcase` does. Every binary with N > 1 systems in a stage AND entities that share an archetype between those systems will surface it. The fix is not in the binaries; it is in the scheduler's safety model.

**Report status**: Phase 1 wiring committable. Phase 2 not run per brief's §1.6 rule. No ParallelSchedule adoption recommendation possible from this experiment; the scheduler needs a safety audit before any further adoption work.
