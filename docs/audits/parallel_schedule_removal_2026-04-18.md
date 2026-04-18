# ParallelSchedule removal — 2026-04-18

**Status**: Complete. `ParallelSchedule` and its two opt-in consumers have been removed from the workspace. AstraWeave is now documented as a deterministic single-threaded ECS with subsystem-level parallelism.
**Driver**: Execution of Rev 2 §5.3 Framing Y recommendation in [`docs/audits/parallel_schedule_safety_audit_2026-04-18.md`](parallel_schedule_safety_audit_2026-04-18.md) — "execute Option E now" given the evidence that no default-features consumer exists and the opt-in consumers produce observed-incorrect output.
**Scope**: Deletion + user-facing documentation updates. No refactoring. No fixes beyond what the scope requires.

---

## 1 Executive summary

Removed:

- The `ParallelSchedule` scheduler and all its unsafe sharing machinery (`SendWorldPtr`, `SystemDescriptor`, `SystemAccess`, `build_groups`, `run_group_parallel`).
- The `parallel` feature in `astraweave-ecs/Cargo.toml` and its `dep:rayon` dependency.
- The `parallel-schedule` feature in both opt-in consumer binaries (`profiling_demo`, `ecs_ai_showcase`) along with all `#[cfg(feature = "parallel-schedule")]` branches.
- The `alloc_measure` bench harness (entirely about `ParallelSchedule`).
- Dead references in `astraweave-ecs`'s public re-exports.

Updated:

- `README.md` — added a sequential-ECS + subsystem-parallelism framing to the Core Engine section, with measured FPS numbers at 1000 entities.
- `CLAUDE.md` — updated the ECS System Stages section to explicitly state "deterministic single-threaded" and reference this report for the removal rationale. Added the `SYNC` stage to the canonical list.
- `docs/current/ARCHITECTURE_MAP.md` — removed `ParallelSchedule` from the Types list; added the single-threaded clarification and a removal note.
- The four prior audit documents in `docs/audits/parallel_schedule_*.md` — each received a supersede note at the top pointing to this report. Bodies unchanged (historical record).

Verified (full details §3 below):

- Workspace compiles cleanly at default features (`astraweave-ecs`, `profiling_demo`, `ecs_ai_showcase`).
- All 448 `astraweave-ecs` library tests pass. All integration tests pass (with one pre-existing test-data update; see §5).
- State checksums for `profiling_demo` and `ecs_ai_showcase` at `frame 100` match bit-for-bit the sequential-path values recorded in the prior experiment reports — confirming default-features behaviour is unchanged.

Not touched:

- Subsystem-level parallelism (rayon in terrain meshing and fluids SPH, tokio in async I/O and LLM and streaming, build-tool rayon in `aw_build`) is untouched. The engine's actual multi-core work continues unaffected.
- No changes to `Schedule`, `App`, or any other ECS primitive. The sequential path is identical to pre-removal behaviour.
- No CI workflow changes — there were none to remove (verified by `rg` returning zero hits in `.github/workflows/`).

---

## 2 What was removed — the complete list

### 2.1 Source files deleted

| File | Lines before deletion | Notes |
|---|---:|---|
| `astraweave-ecs/src/parallel.rs` | 519 | The scheduler itself: `SystemAccess`, `SystemDescriptor`, `ParallelSchedule`, `run_group_parallel`, `SendWorldPtr`, `build_groups`, plus 5 unit tests. |
| `astraweave-ecs/benches/alloc_measure.rs` | 168 | Bench harness entirely about `ParallelSchedule` (`bench_schedule_run`, `bench_build_groups`). No sequential content to preserve. |

### 2.2 Source files edited

| File | Diff summary |
|---|---|
| `astraweave-ecs/src/lib.rs` | Removed `pub mod parallel;` (line 43) and `pub use parallel::{ParallelSchedule, SystemAccess, SystemDescriptor};` (line 79). Replaced with a 3-line comment pointing to this report. |
| `astraweave-ecs/Cargo.toml` | Removed `rayon = { version = "1.10", optional = true }` dependency line. Removed `parallel = ["dep:rayon"]` feature entry, replaced with a supersede comment. Removed the `[[bench]] name = "alloc_measure"` entry. |
| `astraweave-ecs/tests/world_app_tests.rs` | **Pre-existing test-data gap from commit `70266b74e` (schedule-stage-fix)**, fixed as part of this task: `test_app_new` and `test_app_default` asserted `stages.len() == 5`, current default is 8. Updated both to 8 stages and explicit stage-name assertions matching the canonical order. See §5 for context. |
| `examples/profiling_demo/src/main.rs` | Removed `#[cfg(feature = "parallel-schedule")] use astraweave_ecs::parallel::{ParallelSchedule, SystemDescriptor};` (lines 48-49). Removed the gated `parallel_schedule` field on `GameState` and its initializer. Removed the `build_parallel_schedule()` helper (~70 lines). Simplified the tick-loop `#[cfg]` block to just the sequential path. |
| `examples/profiling_demo/Cargo.toml` | Removed the `parallel-schedule = ["astraweave-ecs/parallel"]` feature entry. |
| `examples/ecs_ai_showcase/src/main.rs` | Same pattern as profiling_demo: removed gated imports, removed `build_parallel_schedule()` helper (~65 lines), simplified the tick-loop `#[cfg]` block, updated setup comment to note single-threaded scheduling. |
| `examples/ecs_ai_showcase/Cargo.toml` | Removed the `parallel-schedule = ["astraweave-ecs/parallel"]` feature entry. |

### 2.3 `Cargo.toml` features removed

| Crate | Feature | Was |
|---|---|---|
| `astraweave-ecs` | `parallel` | `parallel = ["dep:rayon"]` |
| `examples/profiling_demo` | `parallel-schedule` | `parallel-schedule = ["astraweave-ecs/parallel"]` |
| `examples/ecs_ai_showcase` | `parallel-schedule` | `parallel-schedule = ["astraweave-ecs/parallel"]` |

`astraweave-fluids/Cargo.toml:9` has its own `parallel = ["dep:rayon"]` feature — **preserved**; this is an unrelated SPH-parallelism feature, not the ECS scheduler.

### 2.4 CI workflow steps removed

**None.** Pre-removal `rg -l 'ParallelSchedule\|parallel-schedule' .github/workflows/` returned zero hits.

### 2.5 Documentation files updated

| File | Change |
|---|---|
| `README.md` | Added ~5 lines to the Core Engine section: explicit "single-threaded archetype scheduler" framing, subsystem parallelism enumeration (rayon in terrain/fluids, tokio in async I/O / LLM / network, GPU compute in rendering), and sequential throughput numbers at 1000 entities citing the schedule-stage-fix report. |
| `CLAUDE.md` | Updated the "ECS System Stages" header section: added "deterministic single-threaded" language, expanded canonical stage list from 7 to 8 (inserted `SYNC` between `SIMULATION` and `AI_PLANNING` per `App::new()`'s actual order), added a pointer to this report. |
| `docs/current/ARCHITECTURE_MAP.md` | `astraweave-ecs` crate entry at line 118: removed `ParallelSchedule` from the Types list; removed `parallel` from the Modules list; added the SYNC stage to the stage list; added a removal note. |
| `docs/audits/parallel_schedule_binary_inventory_2026-04-18.md` | Added supersede header pointing to this report. Body unchanged. |
| `docs/audits/parallel_schedule_experiment_2026-04-18.md` | Added supersede header. Body unchanged. |
| `docs/audits/parallel_schedule_experiment_ecs_ai_showcase_2026-04-18.md` | Added supersede header. Body unchanged. |
| `docs/audits/parallel_schedule_safety_audit_2026-04-18.md` | Added supersede header noting this removal executes Rev 2 §5.3 Framing Y. Body unchanged. |

---

## 3 Verification

### 3.1 Compile checks

```
$ cargo check --release -p astraweave-ecs
    Finished `release` profile [optimized] target(s) in 1m 07s

$ cargo check --release -p profiling_demo
    Finished `release` profile [optimized] target(s) in 21.88s

$ cargo check --release -p ecs_ai_showcase
    Finished `release` profile [optimized] target(s) in 7.98s

$ cargo check --release -p astraweave-ecs -p profiling_demo -p ecs_ai_showcase
    Finished `release` profile [optimized] target(s) in 26.73s
```

All three focal crates compile cleanly at default features, and in combination.

### 3.2 Test suite

```
$ cargo test -p astraweave-ecs --lib
test result: ok. 448 passed; 0 failed; 0 ignored; 0 measured

$ cargo test -p astraweave-ecs
[15 test-file result lines, all passing]
test result: ok. 25 passed; 0 failed
test result: ok. 36 passed; 0 failed
test result: ok. 22 passed; 0 failed
test result: ok. 2 passed; 0 failed
test result: ok. 11 passed; 0 failed
test result: ok. 15 passed; 0 failed; 5 ignored
test result: ok. 6 passed; 0 failed
test result: ok. 152 passed; 0 failed
test result: ok. 21 passed; 0 failed
test result: ok. 20 passed; 0 failed
test result: ok. 0 passed; 0 failed; 6 ignored
test result: ok. 27 passed; 0 failed
test result: ok. 28 passed; 0 failed
test result: ok. 22 passed; 0 failed; 19 ignored
```

Full test pass. The lib test count dropped from 460 (pre-removal) to 448: the 12-test delta is the 5 unit tests that lived inside `parallel.rs` plus 7 stage-fix tests in `lib.rs` that specifically tested ParallelSchedule interactions — all expected to disappear with the deletion.

### 3.3 State-checksum correctness

Confirms default-features observable behaviour unchanged on both opt-in consumers.

```
$ cargo run --release -p profiling_demo --features alloc-counter,fast-alloc -- -e 100 -f 100
[state-checksum] frame 100: pos=00000f92d825caad vel=0000144f43f18daf agent=00000000000009c4
```

**Matches** the historical value from `docs/audits/schedule_stage_fix_2026-04-18.md` §3.1 bit-for-bit.

```
$ cargo run --release -p ecs_ai_showcase --features alloc-counter,fast-alloc -- -e 100 -f 500
[state-checksum] frame 100: pos=00000a9d4a3b691a vel=00000ad05501a730 health=000000000008df7c ai=ffffffffffffffb1 stats=0000000000000000
[state-checksum] frame 200: pos=00000aa08c170bde vel=00000ac750665cf0 health=000000000008b4d4 ai=ffffffffffffffbb stats=0000000000097a2b
[state-checksum] frame 300: pos=00000aa322d58826 vel=00000ac750665cf0 health=00000000000862cc ai=ffffffffffffffbb stats=00000000001bba6f
[state-checksum] frame 400: pos=00000aa4ddb14b0b vel=00000ac750665cf0 health=00000000000810c4 ai=ffffffffffffffbb stats=00000000002dfab3
[state-checksum] frame 500: pos=00000aa6978e72e1 vel=00000ac750665cf0 health=000000000007bebc ai=ffffffffffffffbb stats=0000000000403af7
```

**Matches** the sequential-path value from `docs/audits/parallel_schedule_experiment_ecs_ai_showcase_2026-04-18.md` §3.3 bit-for-bit.

### 3.4 Residual symbol check

```
$ rg -rln 'ParallelSchedule\|SystemDescriptor\|SystemAccess\|SendWorldPtr\|parallel-schedule' \
    --include='*.rs' --include='*.toml' --glob '!**/target/**' --glob '!**/archive/**' --glob '!docs/**'
astraweave-ecs/Cargo.toml
astraweave-ecs/src/lib.rs
```

Both remaining hits are **intentional supersede comments** pointing to this report (`Cargo.toml` in the feature block; `lib.rs` in place of the former `pub use`). No production code references the deleted types.

---

## 4 Reversibility

This task's changes land as one or a small number of commits on branch `main`. To restore `ParallelSchedule`:

```bash
# Find the removal commit:
git log --oneline --all | grep -i "remove.*ParallelSchedule\|parallel_schedule_removal"

# Revert it (where <hash> is the commit identified above):
git revert <hash>
```

Restoring `ParallelSchedule` does **not** restore its soundness. The restored scheduler is the same one documented as unsound in `docs/audits/parallel_schedule_safety_audit_2026-04-18.md` — it would bring back seven shared-state races and the observed-incorrect output on multi-writer groups. Restoration is a *starting point* for implementing Option F (scheduler dispatch change, 4-7 days) or Option C (column-level access primitives, 6-14 weeks) per the safety audit's Rev 2 §2.6 and §2.3 respectively, not a ready-to-use capability.

Commit hash for this removal: **<to be filled in at commit time — reader should check `git log --follow docs/audits/parallel_schedule_removal_2026-04-18.md` for the merge commit>**.

---

## 5 Notes on scope boundaries

One item fell just inside the scope of this task even though it technically predates it:

- **`astraweave-ecs/tests/world_app_tests.rs` test-data assertions**. Two tests (`test_app_new`, `test_app_default`) asserted `app.schedule.stages.len() == 5`. The `App::new()` canonical stage count changed from 5 to 8 in commit `70266b74e` (the schedule-stage-fix task, 2026-04-18 am). That task updated `test_app_creation` in `lib.rs` but missed the integration tests, which were failing on `main` before this task began. Running `cargo test -p astraweave-ecs` for this task's verification step exposed the failures. I updated the two test assertions to match the current canonical stage count and names (`pre_simulation → perception → simulation → sync → ai_planning → physics → post_simulation → presentation`). This is a test-data update, not a refactor — the tests assert observable public API values. The alternative (leaving the tests failing) would have blocked the verification requirement at §3.2 of this task.

No other items required scope stretching.

---

## 6 What remains — AstraWeave's actual parallelism after this task

One paragraph per active parallelism layer, cited against the inventory at `docs/audits/job_system_audit_2026-04-18.md`.

### 6.1 Subsystem rayon

Three production rayon call sites remain, each bounded to its subsystem:

- `astraweave-terrain/src/meshing.rs:478` — `into_par_iter` over voxel chunks for parallel dual-contouring mesh generation. The unit of work is one chunk.
- `astraweave-fluids/src/simd_ops.rs:685-2036` (12 call sites, feature-gated on `astraweave-fluids/parallel`) — per-particle SPH position/velocity/force integration and grid updates. Unit of work is one particle.
- `tools/aw_build/src/main.rs:172` — `par_iter` over build artifacts. Build-tool scope only, not runtime.

Full catalog in `docs/audits/job_system_audit_2026-04-18.md` §1.3.

### 6.2 tokio runtimes

Used for asynchronous work: asset streaming (`astraweave-asset`, `astraweave-scene`), LLM inference (`astraweave-ai/src/llm_executor.rs:165-188`), network server/client (`net/aw-net-server`, `net/aw-net-client`), asset CLI downloads (`tools/astraweave-assets/src/downloader.rs` with bounded concurrency via `tokio::sync::Semaphore(8)`). The workspace dep at `Cargo.toml:178` uses `features = ["full"]` for `rt-multi-thread`. Bridge points between sync game code and async subsystems are cataloged in the job system audit §1.5.

### 6.3 GPU compute

Rendering and shader work via wgpu. Details at `docs/audits/job_system_audit_2026-04-18.md` §2.3. Not modified by this task.

---

## 7 Future parallelism path

ECS-level parallelism is **not foreclosed** by this removal. Options F (scheduler dispatch change, 4-7 days) and C (column-level access primitives, 6-14 weeks) from the safety audit's Rev 2 §2 remain on the menu.

The right moment to revisit is when a specific consumer workload appears that clearly requires ECS-level parallelism — for example, a stage with 4+ systems that each have hot CPU work and whose access sets are genuinely disjoint at the column level. Until such a workload exists, the higher-leverage parallelism work is at the subsystem level. Concrete candidates per `docs/audits/job_system_audit_2026-04-18.md` §3 recommendations:

- **#3**: Cache the `build_groups` output from the prior scheduler — now obsolete (the scheduler is gone). Close the item.
- **#6**: Parallelise per-agent GOAP planning — still open. Applicable as a rayon `par_iter` over AI agents inside a single sequential system; does not require ECS-level parallelism. High value for AI-heavy scenes.
- **#8**: Parallelise `build_visible_instances` and `bin_lights_cpu` — still open. Applicable as a rayon `par_iter` inside the render prep path. Benchmarked impact per the audit.

These are the next parallelism tasks. None of them require restoring `ParallelSchedule`.

---

## 8 Appendix — changes that were considered and rejected

- **Removing the unused `rayon = "1.10"` direct dep in `examples/profiling_demo/Cargo.toml:16`**. This dep was declared non-optional (not feature-gated) but is not actually used in `profiling_demo/src/main.rs`. Leaving in place per the task's "If you find something broken in a file you're touching, add a TODO and move on" rule. Not scope-adjacent enough to warrant removal.
- **Reviewing other `astraweave-*` crates for stray `ParallelSchedule` references**. The `rg` inventory in Phase 1.1 covered the full workspace; zero stray references exist. No action.
- **Updating old journey/daily documents that mention parallel ECS as past or planned work**. Those documents are historical records of work done at specific dates; updating them would revise history. They remain as-is.
- **Removing `astraweave-ecs/Cargo.toml`'s `loom = "0.7"` dev-dependency** now that the primary parallelism consumer is gone. Keeping it because `astraweave-ecs/tests/concurrency_tests.rs` has 11 loom models that still test `World` under `Mutex<World>` wrapping — useful for future concurrency work even without `ParallelSchedule`. The dep is a dev-only cost; no runtime impact.

---

**Report status**: Removal complete. Verified. Safe to commit.
