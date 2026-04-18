# ParallelSchedule Adoption Experiment — profiling_demo — 2026-04-18

**Status**: Complete. Recommendation at §3.8.
**Scope**: Wire `astraweave_ecs::parallel::ParallelSchedule` into `examples/profiling_demo` behind a `parallel-schedule` feature flag, measure FPS and per-frame allocation delta against the sequential baseline at 1000 entities and across a scaling sweep, report. No other binary touched. No scheduler code modified.
**Context**: Executes job-system audit recommendation #1 (`docs/audits/job_system_audit_2026-04-18.md`). Prior context: `docs/audits/allocation_audit_2026-04-17.md`, `docs/audits/allocation_measurement_plan_2026-04-17.md`, `docs/audits/mimalloc_experiment_2026-04-17.md`.

---

## 3.1 What was wired

### Files changed

- `examples/profiling_demo/Cargo.toml` — added one feature:
  ```
  parallel-schedule = ["astraweave-ecs/parallel"]
  ```
  Independent of `alloc-counter`, `profiling`, and `fast-alloc`. All combinations compose (§3.2).
- `examples/profiling_demo/src/main.rs` — three additive edits, all behind `#[cfg(feature = "parallel-schedule")]`:
  1. Conditional `use astraweave_ecs::parallel::{ParallelSchedule, SystemDescriptor};`.
  2. New `GameState.parallel_schedule: ParallelSchedule` field (cfg-gated) plus a `build_parallel_schedule()` constructor function (cfg-gated) that mirrors the `App::new()+add_system` wiring but with access annotations.
  3. In `tick()`, the schedule-run call is gated: `self.parallel_schedule.run(&mut self.app.world)` when the feature is on, `self.app.schedule.run(&mut self.app.world)` otherwise.

Additionally, a non-gated change: a `[state-checksum] frame N:` line is printed every 100 frames. This is a deterministic summary of all entity positions, velocities, and AI states — used in §3.3 to prove the two schedulers produce bit-identical world state. It emits unconditionally so the same build can be diffed across feature combinations; its cost is one Query2 pass over `(Position, Velocity)` and one over `(Position, AIAgent)` every 100 frames, which is dominated by the existing timing prints.

`App::new()` is not modified. The sequential `Schedule` is untouched and remains the default when the feature is off.

### System inventory

Every system `profiling_demo` registers, its stage, the components it touches, and the `.reads::<T>()` / `.writes::<T>()` annotation chosen. All annotations derive from direct inspection of each system body at the cited line ranges.

| System | Stage | Observed access | Annotation |
|---|---|---|---|
| `ai_perception_system` ([main.rs:410-430](../../examples/profiling_demo/src/main.rs#L410-L430)) | `PRE_SIMULATION` | `Query2::<Position, AIAgent>` iteration, read-only; no `get_mut` or `insert` | `.reads::<Position>().reads::<AIAgent>()` |
| `ai_planning_system` ([main.rs:432-483](../../examples/profiling_demo/src/main.rs#L432-L483)) | `AI_PLANNING` | Collects entities via `Query2::<Position, AIAgent>` then calls `world.get_mut::<AIAgent>(entity)` to rotate `AgentState`; Position only read during the collect | `.reads::<Position>().writes::<AIAgent>()` |
| `movement_system` ([main.rs:485-528](../../examples/profiling_demo/src/main.rs#L485-L528)) | `SIMULATION` | `Query2Mut::<Position, Velocity>` — iterator item is `(Entity, &mut A, &B)` per [system_param.rs:258-259](../../astraweave-ecs/src/system_param.rs#L258-L259); body writes `pos.0.*`, reads `vel.0.*` | `.writes::<Position>().reads::<Velocity>()` |
| `physics_system` ([main.rs:530-598](../../examples/profiling_demo/src/main.rs#L530-L598)) | `PHYSICS` | `Query2::<Position, RigidBody>` read-only collect into local `Vec<(Entity, Vec3)>`; builds spatial-hash grid; no world mutation | `.reads::<Position>().reads::<RigidBody>()` |
| `cleanup_system` ([main.rs:600-608](../../examples/profiling_demo/src/main.rs#L600-L608)) | `POST_SIMULATION` | No world access — the body is only a timing record | `.exclusive()` (safe default; has no effect here because nothing else runs in POST_SIMULATION) |
| `rendering_system` ([main.rs:610-636](../../examples/profiling_demo/src/main.rs#L610-L636)) | `PRESENTATION` | `Query2::<Renderable, Position>` read-only iteration | `.reads::<Renderable>().reads::<Position>()` |

### Consequence of the inventory

**`profiling_demo` registers exactly one system per stage, over six stages.** `ParallelSchedule::build_groups` ([parallel.rs:230-257](../../astraweave-ecs/src/parallel.rs#L230-L257)) produces one group with one member per stage. `ParallelSchedule::run` ([parallel.rs:273-283](../../astraweave-ecs/src/parallel.rs#L273-L283)) short-circuits the `group.len() == 1` case and calls the system directly — **`rayon::scope` is never entered during a run** on this binary. The experiment therefore measures pure scheduler overhead on top of the same system call sequence; it does not measure intra-stage parallelism, because there is none to measure.

This is an important framing for §3.7-§3.8: the binary exercises the code path that builds groups and indexes into a group vector, but not the code path that fans out to rayon workers. A positive result would still be noteworthy (free win on the overhead code path); a zero or negative result does not invalidate `ParallelSchedule` for workloads that would actually benefit from intra-stage parallelism.

---

## 3.2 Compile-check matrix

All eight feature combinations compile on Windows 11, cargo 1.89.0, release profile.

| `--features` | Result |
|---|---|
| *(default = `fast-alloc`)* | `Finished release profile [optimized] target(s) in 1m 06s` |
| `parallel-schedule` | `Finished release profile [optimized] target(s) in 2.93s` |
| `alloc-counter` | `Finished release profile [optimized] target(s) in 2.22s` |
| `parallel-schedule,alloc-counter` | `Finished release profile [optimized] target(s) in 1.70s` |
| `fast-alloc` | `Finished release profile [optimized] target(s) in 0.79s` |
| `parallel-schedule,fast-alloc` | `Finished release profile [optimized] target(s) in 0.77s` |
| `parallel-schedule,alloc-counter,fast-alloc` | `Finished release profile [optimized] target(s) in 0.74s` |
| `profiling,alloc-counter,fast-alloc,parallel-schedule` | `Finished release profile [optimized] target(s) in 8.19s` |

The first `--features` row defaults to `fast-alloc` per `examples/profiling_demo/Cargo.toml:25`, so this is the mimalloc-on build. The total-from-scratch build time is a cold-cache artifact; all subsequent builds are incremental.

---

## 3.3 Correctness check

**Procedure** (per brief §1.6): run the demo for 100 frames at 100 entities once with `--features alloc-counter,fast-alloc` (sequential scheduler) and once with `--features parallel-schedule,alloc-counter,fast-alloc` (parallel scheduler). Capture the `[state-checksum]` and `[alloc-measure]` lines. Diff.

**Result**:

```
sequential: [state-checksum] frame 100: pos=00000f92d825caad vel=0000144f43f18daf agent=00000000000009c4
parallel:   [state-checksum] frame 100: pos=00000f92d825caad vel=0000144f43f18daf agent=00000000000009c4

sequential: [alloc-measure]  frame 100: allocs=356 bytes=54076 reallocs=51 net=0
parallel:   [alloc-measure]  frame 100: allocs=369 bytes=54732 reallocs=51 net=0
```

- Position checksum, velocity checksum, agent-state checksum: **bit-identical**.
- `reallocs` and `net` allocations: **identical**.
- `allocs` and `bytes` differ by **+13 allocs / +656 bytes per 100 frames**, which is **+0.13 allocs / +6.56 bytes per frame** above sequential. The additional allocations come from `build_groups` constructing a `Vec<Vec<usize>>` (one outer + one inner vec per group per stage) every `run()` call ([parallel.rs:236-253](../../astraweave-ecs/src/parallel.rs#L236-L253)). Six stages × 2 allocs/stage ≈ 12 allocs matches.

World state is identical at matching frame numbers. Proceed to Phase 2.

A secondary observation that hardens the correctness claim: because every stage has exactly one system, `ParallelSchedule::run_group_parallel` is never entered (§3.1). This experiment could not have produced a race condition even if the annotations in §3.1 were wrong, because no system ever ran concurrently with another. A proper race-detection test would require a binary with at least two systems in the same stage, which `profiling_demo` does not have.

---

## 3.4 Primary results — 1000 entities, 1000 frames

Three independent runs per cell. Release profile, Windows 11, cargo 1.89.0, same workstation / same build / same day. Each run from scratch (fresh process).

| Configuration | Runs (FPS) | Median | Min | Max | Allocs/frame @ f1000 | Bytes/frame @ f1000 |
|---|---|---:|---:|---:|---:|---:|
| Sequential + mimalloc | 2040.55 / 1356.14 / 2036.09 | **2036.09** | 1356.14 | 2040.55 | 2931 | 453 452 |
| ParallelSchedule + mimalloc | 1279.65 / 1882.06 / 1711.56 | **1711.56** | 1279.65 | 1882.06 | 2944 | 454 108 |
| Sequential + system alloc | 1303.82 / 1062.44 / 1446.48 | **1303.82** | 1062.44 | 1446.48 | 2931 | 453 452 |
| ParallelSchedule + system alloc | 1251.76 / 1057.37 / 1395.91 | **1251.76** | 1057.37 | 1395.91 | 2944 | 454 108 |

**Allocation delta is +13 allocs / +656 bytes per frame** regardless of allocator, schedule, or entity count — a constant overhead from `build_groups` per §3.3. Sequential and parallel allocs are exactly equal within each column ignoring those 13 — confirming neither path changes the workload's allocation profile, only the scheduler's own.

**Median delta, scheduler→parallel** (same allocator):
- mimalloc: (1711.56 − 2036.09) / 2036.09 = **−15.9 %**.
- system allocator: (1251.76 − 1303.82) / 1303.82 = **−4.0 %**.

**Caveat on §2.3/2.4 command syntax**. The brief's verbatim commands for the system-allocator rows (`cargo run ... --features alloc-counter`) do not actually disable mimalloc, because `examples/profiling_demo/Cargo.toml:25` has `default = ["fast-alloc"]`. To measure the system allocator you must pass `--no-default-features`. I used `--no-default-features --features alloc-counter` and `--no-default-features --features parallel-schedule,alloc-counter` for rows 3 and 4 respectively. This is a cherry-pick correction, not a scope creep — without it the comparison would be mimalloc vs mimalloc and row 2.3/2.4 would be noise.

---

## 3.5 Entity-scaling results

Three independent runs per cell. 500 frames (per brief §2.5). Mimalloc on. Medians shown.

| Entities | Sequential median FPS | ParallelSchedule median FPS | Δ (%) | Sequential allocs @ f500 | Parallel allocs @ f500 |
|---:|---:|---:|---:|---:|---:|
| 200 | 10 149.19 | 9 796.56 | **−3.5 %** | 628 | 641 |
| 1000 (from §3.4, 1000f) | 2 036.09 | 1 711.56 | **−15.9 %** | 2 931 *(at f1000)* | 2 944 *(at f1000)* |
| 2000 | 939.54 | 904.67 | **−3.7 %** | 6 011 | 6 024 |
| 4000 | 449.09 | 427.18 | **−4.9 %** | 12 036 | 12 049 |

Per-entity allocation delta between schedulers is always exactly +13 — matching §3.3 and §3.4. `build_groups`-driven overhead does not scale with entity count (as expected — it scales with system count, which is constant).

The Δ does not grow with entity count. Per the brief's instrumentation ("a Δ that stays flat or shrinks with entity count indicates the scheduler isn't actually parallelizing meaningful work — either because systems are too small, or because too many are marked `.exclusive()`"), the flat-to-slightly-negative Δ here is the signature of the third condition the brief did not explicitly name: **one system per stage, so `run_group_parallel` is never entered** (§3.1 last paragraph). ParallelSchedule is running the same sequential code path plus `build_groups` every tick.

The 1000e row's apparent outlier at −15.9 % vs the flat −3 to −5 % at other entity counts is almost certainly noise — see §3.6.

---

## 3.6 Noise characterization

For each row in §3.4 and §3.5, three-run min and max and the spread as a percentage of the median.

### §3.4 primary (1000 entities, 1000 frames)

| Row | Min (FPS) | Max (FPS) | Spread = (max − min)/median | Flag |
|---|---:|---:|---:|:---:|
| Sequential + mimalloc | 1 356.14 | 2 040.55 | 33.6 % | **noisy** |
| ParallelSchedule + mimalloc | 1 279.65 | 1 882.06 | 35.2 % | **noisy** |
| Sequential + system alloc | 1 062.44 | 1 446.48 | 29.5 % | **noisy** |
| ParallelSchedule + system alloc | 1 057.37 | 1 395.91 | 27.0 % | **noisy** |

**Range overlap check** (primary comparison, sequential + mimalloc vs ParallelSchedule + mimalloc):
- Sequential range: [1 356.14, 2 040.55].
- Parallel range: [1 279.65, 1 882.06].
- **Intersection: [1 356.14, 1 882.06]** — non-empty; the two ranges overlap over ~53 % of the parallel range.

When the ranges overlap this heavily, the median delta (−15.9 %) is not statistically meaningful regardless of its magnitude. Per the brief: "If yes, the delta is not significant regardless of what the medians say."

Same check for system allocator: seq [1 062, 1 446] vs par [1 057, 1 396] — overlap [1 062, 1 396], roughly the full parallel range. Not significant.

### §3.5 scaling (500 frames)

| Row | Min (FPS) | Max (FPS) | Spread | Flag |
|---|---:|---:|---:|:---:|
| Sequential 200e | 9 868.47 | 10 183.22 | 3.1 % | — |
| Parallel 200e | 9 368.93 | 9 870.95 | 5.1 % | — |
| Sequential 2000e | 643.85 | 958.48 | 33.5 % | **noisy** |
| Parallel 2000e | 588.75 | 930.30 | 37.8 % | **noisy** |
| Sequential 4000e | 344.39 | 449.77 | 23.5 % | **noisy** |
| Parallel 4000e | 312.63 | 436.03 | 28.9 % | **noisy** |

The 200-entity rows are stable (sub-5 % spread); the 2000- and 4000-entity rows are as noisy as the 1000-entity rows in §3.4. The 200e rows show a real, small regression (seq 10 149 vs par 9 796 = −3.5 %, non-overlapping ranges: seq [9 868, 10 183] vs par [9 369, 9 871] just touch at 9 870). At 2000e and 4000e the ranges overlap heavily and no conclusion can be drawn.

Interpretation of the noise: the workload takes 500 ms to 2 s per run (1000 frames / 2000 FPS ≈ 0.5 s at the fastest, 500 frames / 450 FPS ≈ 1.1 s at 4000e). Cold-cache startup + Windows scheduler jitter easily dominate at these short runtimes. A single 10-second run per cell would reduce relative noise; the brief explicitly required three runs per cell (`Single runs are not acceptable`), which surfaced the noise rather than averaging it out.

---

## 3.7 Interpretation

**Does ParallelSchedule help at 1000 entities?** No. Median drops 15.9 %; range overlap (§3.6) makes the delta statistically indistinguishable from noise in both directions. At best, `ParallelSchedule` is neutral on `profiling_demo`; at worst, it pays 0.44 % extra allocations per frame (13 allocs / 656 bytes on top of 2931 / 453 452) for zero parallelism gain. The mechanism is transparent: every `profiling_demo` stage holds one system, so `ParallelSchedule::run` takes the single-system fast path ([parallel.rs:276-278](../../astraweave-ecs/src/parallel.rs#L276-L278)) and rayon is never invoked. We are measuring scheduler-overhead-on-top-of-sequential, not scheduler-with-parallelism.

**Does the help scale with entity count?** No. The 200e / 2000e / 4000e deltas are all in the −3 % to −5 % band (within noise), and the outlier at 1000e (−16 %) is itself within noise per the range overlap check. The pattern "flat Δ across entity scaling" is the code-structure signature of the overhead-only path — if intra-stage parallelism were firing, we would expect Δ to grow with entity count as each system's work multiplied. The absence of that growth is consistent with §3.1's observation that `run_group_parallel` is never called.

**Is the allocation-per-tick cost of `build_groups` meaningful?** No in absolute terms, yes in relative terms as a gratuitous cost. 13 allocs / 656 bytes per tick is 0.44 % / 0.14 % of sequential's 2931 / 453 452. That is not meaningful throughput-wise. It is meaningful philosophically — ParallelSchedule is paying this cost on every tick and the binary it is paying for returns nothing. Job-system audit recommendation #3 ("cache `build_groups` output") would eliminate this cost and restore neutrality on overhead-only workloads. Whether that is worth the effort is a separate decision — the cost is already small.

---

## 3.8 Recommendation

**Do not merge; the sequential scheduler is sufficient for `profiling_demo`'s current workload. Document findings and keep ParallelSchedule available for future workloads that exceed sequential capacity.**

Defense, in order of weight:

1. Median delta on the primary comparison (1000e, mimalloc) is **−15.9 %**, i.e. ParallelSchedule is slower. Even granting the noise band (§3.6 range overlap proves no statistical significance), the best-case outcome is "no improvement" — the best the data supports is neutrality, not a win.
2. The Δ does not scale with entity count (§3.5, §3.7 paragraph 2). In a parallel-scheduler win scenario, larger workloads amortise scheduling overhead and expose per-core throughput; here the Δ stays flat because there is no per-core throughput being exposed — every stage has one system and rayon is never entered.
3. `build_groups` adds a constant 13-allocs / 656-bytes tax per tick (§3.4, §3.5). Small in absolute terms (0.44 % of sequential) but gratuitous for a workload that receives no parallelism benefit from paying it.
4. The experiment did not invalidate `ParallelSchedule` in general — it invalidated it **for this specific binary's topology** (one system per stage). A binary with multiple systems sharing a stage could produce a different verdict. The audit's recommendation #1 is satisfied by the measurement, not by a positive adoption signal, and the outcome should be treated as data, not a failure of the scheduler itself.

Do not revert the `parallel-schedule` feature. Leave the wiring in place so the measurement can be reproduced and so the path is available when a multi-system-per-stage binary wants to opt in. But do not change the default, do not wire it into any other binary in this task, and do not proceed with audit recommendations #3 (cache `build_groups`) or #5 (per-system Tracy spans) on the strength of this result alone.

**Next steps for whoever picks this up next**:

- If ParallelSchedule adoption is still a goal: the next experiment needs a binary with at least two disjoint-access systems in the same stage. `examples/ecs_ai_showcase` (manual 300-tick loop, audit §1.7) is a candidate. Inventory its systems first; only proceed if 2+ systems share a stage and have disjoint access.
- If the 1000e noise is unacceptable for future measurements: run for more frames per cell (10 000+) to drown out startup jitter, or run under a single persistent process that records multiple trials internally. The brief's "three independent runs" fits the mimalloc-experiment shape (where the effect was so large the noise did not matter); for overhead-only comparisons the run length needs to grow.

---

## 3.9 Open questions

Questions this experiment could not answer and what it would take to answer them.

- **What does a multi-agent AI workload look like under each scheduler?** Not testable here. The `ai_planning_system` runs GOAP cache-miss math on every tenth agent but still as one sequential pass; no per-agent parallelism. Would require either (a) a rewritten `ai_planning_system` that `par_iter`s over agents — separate optimisation, different feature — or (b) a binary with multiple AI-scheduling systems each handling a subset.
- **Does the Δ change with more systems per stage?** Directly gated on finding or constructing such a binary. `profiling_demo`'s 1-per-stage structure is the limiting factor; duplicating systems to force stage-sharing would change the workload too much to be a meaningful A/B.
- **What's the per-system wall-time breakdown?** Requires audit recommendation #5 (per-system Tracy spans). The existing `SYSTEM_TIMINGS` struct at [main.rs:163-174](../../examples/profiling_demo/src/main.rs#L163-L174) gives microsecond totals per system but is manually-measured and not a hook for Tracy flame-graph view. Tracy spans in `Schedule::run` (sequential) and `ParallelSchedule::run_group_parallel` (parallel) would expose where a run actually spends its time.
- **Is the 1000e outlier at −15.9 % Δ a real effect or noise?** Range overlap proves it is not significant at three runs. Could be answered with either a longer run per cell (10 000+ frames) or a larger N (50+ independent runs). Not worth it on the strength of the current result.
- **Does mimalloc's thread-local free list matter when rayon never spawns?** §3.4 row 1 vs row 2 shows mimalloc gives the sequential scheduler more headroom (+56 % median FPS over system allocator at 1000e: 2036 vs 1304) than it gives the parallel scheduler (+37 % over system: 1712 vs 1252). Could be allocator-internal state interaction with the extra `build_groups` allocations, could be noise given the spread. Would need per-allocation-site instrumentation to tell apart.

---

## 3.10 Appendix — issues found in `ParallelSchedule`

Nothing that looks like a real bug. One rough edge worth flagging for a future task:

- **`SystemDescriptor::new` defaults to `exclusive: true`** ([parallel.rs:129-138](../../astraweave-ecs/src/parallel.rs#L129-L138)). The safe default, but it also means a migration from `Schedule` → `ParallelSchedule` that forgets to annotate every system silently produces a correct but fully-serialised schedule. A migration diagnostic (e.g. a `log::debug!` or feature-gated `println!` when `build_groups` returns `groups.len() == systems.len()`, i.e. "every system is exclusive") would catch this class of mistake. Not a bug, a missing safety rail. Do not fix as part of this task.
- **`build_groups` is `O(systems² × avg_group_size)` per call** and is called every `run()` ([parallel.rs:231, 273](../../astraweave-ecs/src/parallel.rs#L231)). The audit already flagged this (rec #3). On `profiling_demo`'s 6-system / 6-stage topology the cost is trivial (+13 allocs / +656 bytes per tick — §3.3); on a hypothetical 50-system binary it would bite. Caching is the right fix; do not fix here.
- **`SendWorldPtr` only usable inside `run_group_parallel`** ([parallel.rs:52-70](../../astraweave-ecs/src/parallel.rs#L52-L70)). Not a bug, but the `#[cfg(feature = "parallel")]` gate on the `send_ptr` module means `ParallelSchedule` itself compiles without the `parallel` feature and falls back to a serial in-order loop at [parallel.rs:318-324](../../astraweave-ecs/src/parallel.rs#L318-L324). That is a useful property — it lets a binary opt into the annotation API even on platforms where rayon is undesired — but it is not documented anywhere a consumer would see. No code change required; a line in `parallel.rs`'s module doc comment would clarify.

No issue I would characterise as "the scheduler is broken". The three items above are design trade-offs or documentation gaps, not defects.

---

## Reproducibility

Exact commands, in order. From repo root.

```bash
# Phase 1.5 — compile matrix (all eight must print `Finished release profile [optimized]`)
cargo check --release -p profiling_demo
cargo check --release -p profiling_demo --features parallel-schedule
cargo check --release -p profiling_demo --features alloc-counter
cargo check --release -p profiling_demo --features parallel-schedule,alloc-counter
cargo check --release -p profiling_demo --features fast-alloc
cargo check --release -p profiling_demo --features parallel-schedule,fast-alloc
cargo check --release -p profiling_demo --features parallel-schedule,alloc-counter,fast-alloc
cargo check --release -p profiling_demo --features profiling,alloc-counter,fast-alloc,parallel-schedule

# Phase 1.6 — correctness check (state checksums at f100 must match bit-for-bit)
cargo run --release -p profiling_demo --features alloc-counter,fast-alloc -- -e 100 -f 100
cargo run --release -p profiling_demo --features parallel-schedule,alloc-counter,fast-alloc -- -e 100 -f 100

# Phase 2.1-2.4 — primary (three runs each; capture `Average FPS:` and the frame 1000 alloc line)
for i in 1 2 3; do cargo run --release -p profiling_demo --features alloc-counter,fast-alloc -- -e 1000 -f 1000; done
for i in 1 2 3; do cargo run --release -p profiling_demo --features parallel-schedule,alloc-counter,fast-alloc -- -e 1000 -f 1000; done
for i in 1 2 3; do cargo run --release -p profiling_demo --no-default-features --features alloc-counter -- -e 1000 -f 1000; done
for i in 1 2 3; do cargo run --release -p profiling_demo --no-default-features --features parallel-schedule,alloc-counter -- -e 1000 -f 1000; done

# Phase 2.5 — scaling sweep (three runs per entity count per scheduler; mimalloc on)
for e in 200 2000 4000; do for i in 1 2 3; do cargo run --release -p profiling_demo --features alloc-counter,fast-alloc -- -e $e -f 500; done; done
for e in 200 2000 4000; do for i in 1 2 3; do cargo run --release -p profiling_demo --features parallel-schedule,alloc-counter,fast-alloc -- -e $e -f 500; done; done
```

**Note on §2.3/2.4 commands**: the brief used `--features alloc-counter` for the "system allocator" rows, but `examples/profiling_demo/Cargo.toml:25` sets `default = ["fast-alloc"]`. The commands above use `--no-default-features --features alloc-counter` to actually disable mimalloc; the brief's originals would have measured mimalloc-vs-mimalloc. This correction is discussed at the end of §3.4.

---

**Report status**: Discovery, wiring, and measurement complete. No decisions about wider ParallelSchedule adoption made. The `parallel-schedule` feature is committable but not enabled by default; whether to merge and how to proceed is a decision left for the reader per §3.8.
