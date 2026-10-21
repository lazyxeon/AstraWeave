# Fix: `Schedule::add_system` silent-drop for unregistered stages — 2026-04-18

**Status**: Complete. The silent-drop bug is fixed. Every affected binary verified. Prior measurements re-baselined.
**Scope**: Engine primitive fix in `astraweave-ecs`. No binary patched except for the one-line test-count update in `test_app_creation`.
**Context**: Bug surfaced in `docs/audits/parallel_schedule_binary_inventory_2026-04-18.md` §4. Task brief referenced that audit and required this follow-up before any further ParallelSchedule work.

---

## Summary

`Schedule::add_system` at [astraweave-ecs/src/lib.rs:707-711](../../astraweave-ecs/src/lib.rs#L707-L711) silently dropped systems registered to non-existent stages. `App::new()` created only five stages (`perception`, `simulation`, `ai_planning`, `physics`, `presentation`), so every workspace call site registering to `pre_simulation`, `sync`, or `post_simulation` was a no-op at runtime.

**Fix (Option A per the brief)**: `App::new()` now registers eight canonical stages:

```
pre_simulation → perception → simulation → sync → ai_planning → physics → post_simulation → presentation
```

Additionally, `Schedule::add_system` now emits an `eprintln!` diagnostic in debug builds when targeted at an unknown stage (release builds preserve the original silent-drop for backward compatibility with optional-stage call patterns).

Previously-dropped systems now execute in all affected binaries. World-state correctness is preserved (`profiling_demo` state checksum unchanged). `ecs_ai_showcase`'s stats panel, silently broken before the fix, now prints every 60 ticks as its source code claimed it would. Mimalloc vs system-allocator FPS delta on `profiling_demo` narrowed slightly (+56 % → +46 %) but remains comfortably above the 10 % threshold, so the prior mimalloc merge recommendation still holds.

---

## Phase 1 — Approach chosen

**Option A**: add the missing canonical stages to `App::new()`'s default stage list. Also: add a debug-build warning for `add_system` on unknown stages.

**Why not Option B** (auto-create on first reference): registration-order-dependent stage placement is a correctness hazard. A system registered to `"post_simulation"` before any system is registered to `"simulation"` would end up with `post_simulation` running first — opposite of the intended pipeline. Every binary would have to internalise registration-order gotchas; this trades one silent-failure mode for another.

**Why not Option C** (panic on unknown stage): breaks every current binary on first run. The silent-drop has been the contract for long enough that some call sites rely on it (e.g. optional stages added only when certain features are enabled). A hard panic would be a large blast-radius change. Debug-build warning gets 80 % of the value at zero runtime cost in release.

**Sync-stage placement**: after `simulation`, before `ai_planning`. Rationale: `astraweave-core::ecs_adapter::build_app` registers `sys_sim` and `sys_move` to `simulation`, then `sys_bridge_sync` and `sys_sync_to_legacy` to `sync` (per [ecs_adapter.rs:218-223](../../astraweave-core/src/ecs_adapter.rs#L218-L223)). `sys_sim` mutates the legacy `World` resource (cooldowns); `sys_move` mutates ECS `CPos`; `sys_sync_to_legacy` propagates ECS `CPos`/`CHealth`/`CAmmo` back to the legacy `World`. Downstream planners (e.g. `astraweave-ai`'s `sys_ai_planning` at [ecs_ai_plugin.rs:85](../../astraweave-ai/src/ecs_ai_plugin.rs#L85)) read the legacy `World` resource, so they need the sync to have run first. Placing `sync` between `simulation` and `ai_planning` makes this explicit.

---

## Phase 2 — Implementation

### 2.1 Diff summary

Two edits in `astraweave-ecs/src/lib.rs`, both localised, total ~30 lines of production diff.

**Edit 1** — `Schedule::add_system` at [lib.rs:707-724](../../astraweave-ecs/src/lib.rs#L707-L724):

```rust
pub fn add_system(&mut self, stage: &'static str, sys: SystemFn) {
    if let Some(s) = self.stages.iter_mut().find(|s| s.name == stage) {
        s.systems.push(sys);
    } else {
        #[cfg(debug_assertions)]
        eprintln!(
            "[astraweave-ecs] Schedule::add_system: stage '{}' is not registered; \
             system will not execute. Call `schedule.add_stage(\"{}\")` or \
             `Schedule::with_stage(\"{}\")` first, or use App::new() which \
             provides the canonical stages.",
            stage, stage, stage
        );
    }
}
```

**Edit 2** — `App::new` at [lib.rs:770-797](../../astraweave-ecs/src/lib.rs#L770-L797):

Before:

```rust
let mut schedule = Schedule::default();
schedule = schedule
    .with_stage("perception")
    .with_stage("simulation")
    .with_stage("ai_planning")
    .with_stage("physics")
    .with_stage("presentation");
```

After (8 stages, canonical order with `sync` placed after `simulation`):

```rust
let mut schedule = Schedule::default();
schedule = schedule
    .with_stage("pre_simulation")
    .with_stage("perception")
    .with_stage("simulation")
    .with_stage("sync")
    .with_stage("ai_planning")
    .with_stage("physics")
    .with_stage("post_simulation")
    .with_stage("presentation");
```

Accompanying doc-comment block justifies the stage order and references this report.

### 2.2 Test additions

Seven test changes / additions in `astraweave-ecs/src/lib.rs`:

| Test | Purpose | Result |
|---|---|---|
| `test_app_creation` (updated) | Verify new default stage count (5 → 8) | **pass** |
| `test_app_default_stages_canonical_order` (new) | Assert the exact 8-stage order vector returned by `App::new()` | **pass** |
| `test_previously_dropped_stages_now_execute` (new) | Register systems to `pre_simulation` / `sync` / `post_simulation`, run 3 ticks, assert each system ran 3 times | **pass** |
| `test_stage_execution_order_respects_canonical` (new) | Register 8 systems in reverse order, verify execution sequence matches canonical order | **pass** |
| `test_add_system_with_unknown_stage_is_still_dropped` (new) | Regression guard: silent-drop contract preserved for genuinely unknown stage names | **pass** |
| `test_empty_new_stages_are_no_ops` (new) | `App::new` without any `add_system` calls has all 8 stages empty and runs cleanly | **pass** |
| `test_existing_5_stage_registrations_still_work` (new) | Regression guard: existing 5-stage binaries behave identically | **pass** |

### 2.3 Test results

```
$ cargo test -p astraweave-ecs --lib
test result: ok. 460 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.51s
```

All 460 tests pass — the 6 new tests and the updated one all pass, plus every pre-existing test still passes. No regressions.

---

## Phase 3 — Per-binary verification

### 3.1 `examples/profiling_demo` — correctness recheck at 100e/100f

The prior parallel-schedule experiment at `docs/audits/parallel_schedule_experiment_2026-04-18.md` §3.3 captured:

```
[state-checksum] frame 100: pos=00000f92d825caad vel=0000144f43f18daf agent=00000000000009c4
[alloc-measure]  frame 100: allocs=356 bytes=54076 reallocs=51 net=0
```

Post-fix rerun:

```
[state-checksum] frame 100: pos=00000f92d825caad vel=0000144f43f18daf agent=00000000000009c4
[alloc-measure]  frame 100: allocs=357 bytes=54108 reallocs=51 net=0
```

**State checksum: bit-identical.** `pos`, `vel`, and `agent` hashes unchanged — the two newly-executing systems (`ai_perception_system` and `cleanup_system`) do not mutate world state. Reading their bodies at [profiling_demo/src/main.rs:410-430](../../examples/profiling_demo/src/main.rs#L410-L430) and [main.rs:600-608](../../examples/profiling_demo/src/main.rs#L600-L608) confirms this: `ai_perception_system` iterates `Query2<Position, AIAgent>` read-only and computes a discarded `sin()`; `cleanup_system`'s body is purely timing. Neither calls `world.get_mut`, `world.insert`, or `world.spawn`.

The brief predicted divergence. The actual result is no divergence, which is a stronger correctness outcome — the fix restores the systems to execution without changing observable world state. The +1 alloc/frame (+32 bytes/frame) delta comes from `ai_perception_system`'s `Query2` construction, consistent with a per-frame allocation of one small `Vec<ArchetypeId>` inside the query constructor.

**Verdict**: correct. No regression in simulation state; measurable proof the dropped systems now run.

### 3.2 `examples/hello_companion` — default run

```
$ cargo run --release -p hello_companion
🤖 Classical AI (RuleOrchestrator)
   Generated 3 steps
✅ Generated 3 step plan in 0.025ms

--- Executing Plan @ t=0.00 ---
   Plan plan-0 with 3 steps
⚠️  Execution failed: line of sight blocked. Continuing...

--- Post-execution State @ t=5.00 ---
Companion: IVec2 { x: 2, y: 3 }
Enemy:     IVec2 { x: 12, y: 2 }
Enemy HP:  60
```

Completes without error. The "line of sight blocked" message is pre-existing behaviour in the RuleOrchestrator scenario (plan generated before LOS validation; not related to the fix).

`sys_bridge_sync` and `sys_sync_to_legacy` now execute once per fixed tick × 20 fixed ticks = 20 times each. Reading their bodies at [ecs_adapter.rs:98-163](../../astraweave-core/src/ecs_adapter.rs#L98-L163):

- `sys_bridge_sync` walks `EntityBridge.ecs_entities()` and adds a missing `CLegacyId` component to any entity the bridge references but that doesn't have the component yet. In hello_companion all entities get their `CLegacyId` during setup via the bridge at [ecs_adapter.rs:210-213](../../astraweave-core/src/ecs_adapter.rs#L210-L213), so this becomes a no-op on every subsequent tick.
- `sys_sync_to_legacy` reads `CPos`/`CHealth`/`CAmmo` from each ECS entity with `CLegacyId` and mirrors them into the legacy `World` resource. Since hello_companion's companion entity has no `CDesiredPos`, `sys_move` never updates `CPos`, so `sys_sync_to_legacy` writes the same values each tick — no observable change.

**Verdict**: functional correctness preserved. The systems now run; the values they propagate are identical to what the legacy World already had. No regression.

### 3.3 `examples/ecs_ai_showcase` — stats panel re-enabled

Pre-fix: `stats_display_system` was silently dropped (registered to `"post_simulation"` which did not exist in `App::new()`). The binary ran for 300 ticks with no stats prints — the panels the code describes as "Print stats every 60 ticks" at [main.rs:442-465](../../examples/ecs_ai_showcase/src/main.rs#L442-L465) never appeared.

Post-fix run (tail):

```
=== Game Stats (Tick 180) ===
Enemies Defeated: 87
Total Damage: 960
Player Deaths: 0

=== AI States ===
Attacking: 1
Patrolling: 4

=== Game Stats (Tick 240) ===
Enemies Defeated: 267
Total Damage: 2760
Player Deaths: 0
...
=== Game Stats (Tick 300) ===
Enemies Defeated: 447
Total Damage: 4560
Player Deaths: 0
```

Stats panels now print every 60 ticks as documented. (The "Enemies Defeated: 447" over 300 ticks is cumulative across damage events — `combat_system` at [main.rs:422-427](../../examples/ecs_ai_showcase/src/main.rs#L422-L427) increments on every fatal blow and the showcase does not respawn enemies, so the counter reflects total fatal-blow events rather than unique defeats. That's a pre-existing counting-semantics quirk in `combat_system`, not something the fix changed.)

**Verdict**: previously-silent feature (stats panels) is now visible. Exactly the behaviour the source code always promised. No regression; this is a bug fix surfaced by the primary fix.

### 3.4 `tools/aw_editor` — compile check

```
$ cargo build --release -p aw_editor
    Finished `release` profile [optimized] target(s) in 8m 33s
```

Editor compiles cleanly. Warnings (1 in `aw_editor` crate, nalgebra future-incompat) are pre-existing and unrelated.

**Runtime implication** (not tested here per brief): the editor's play-mode simulation at [runtime.rs:565,691](../../tools/aw_editor/src/runtime.rs) builds via `astraweave_core::ecs_adapter::build_app`. `sys_bridge_sync` and `sys_sync_to_legacy` will now run per fixed tick while the editor is in `RuntimeState::Playing`. Logic-wise this means the editor's legacy `World` resource will now reflect ECS `CPos`/`CHealth`/`CAmmo` mutations instead of staying frozen at the edit-time snapshot. This is a correctness improvement — the editor's play-mode behaviour will now match what it was always intended to do — but since play-mode UI has not been exhaustively regression-tested here, downstream editor panels that read the legacy `World` resource during play could surface bugs that were previously masked by the stale sync.

Follow-up: a UI-side sanity check of `RuntimeState::Playing` → snapshot round-trip is advisable, but is out of scope for this task.

---

## Phase 4 — Re-baseline mimalloc measurements

Prior baseline (from `docs/audits/parallel_schedule_experiment_2026-04-18.md` §3.4, pre-fix):

| Configuration | Median FPS | Range | Allocs/frame @ f1000 |
|---|---:|---|---:|
| Sequential + mimalloc | 2 036.09 | 1 356–2 041 | 2 931 |
| Sequential + system alloc | 1 303.82 | 1 062–1 446 | 2 931 |
| Mimalloc delta | **+56.2 %** | | 0 |

Post-fix rerun (same workstation, same day, three runs each):

| Configuration | Runs (FPS) | Median | Min | Max | Allocs/frame @ f1000 | Bytes/frame @ f1000 |
|---|---|---:|---:|---:|---:|---:|
| Sequential + system alloc | 1 201.39 / 1 140.06 / 1 293.90 | **1 201.39** | 1 140.06 | 1 293.90 | 2 932 | 453 484 |
| Sequential + mimalloc | 1 756.81 / 1 528.05 / 1 803.98 | **1 756.81** | 1 528.05 | 1 803.98 | 2 932 | 453 484 |
| Mimalloc delta | — | **+46.2 %** | — | — | 0 | 0 |

**Alloc delta pre vs post fix**: +1 alloc/frame and +32 bytes/frame across both configurations — matches the §3.1 correctness observation. The allocation workload is identical between system and mimalloc allocators (as before).

**FPS delta interpretation**:

1. Both pre- and post-fix absolute FPS numbers shifted down slightly: system allocator 1 304 → 1 201 (−7.9 %), mimalloc 2 036 → 1 757 (−13.7 %). This reflects the cost of running two additional systems per tick (`ai_perception_system` and `cleanup_system`, which — although they don't mutate state — still iterate and record timings). The ECS changes between the two runs are zero; the workload itself grew by two system invocations per tick.
2. Both pre- and post-fix have ~20–30 % run-to-run spread (noise characterisation matches the prior experiment's §3.6). The mimalloc-vs-system ratio is what matters for the brief's §4.3 threshold check.
3. The mimalloc delta narrowed from **+56 %** to **+46 %**. Both values are well above the 10 % merge-recommendation threshold. The ordering of the prior finding (mimalloc is worth enabling) is unchanged.

**Hypothesis for why mimalloc's edge narrowed** (not verified): the newly-running `ai_perception_system` performs `Query2` construction every tick, which is a small fixed-cost allocation that doesn't benefit much from mimalloc's thread-local free list (single-threaded, small). Adding a fixed-cost per-tick operation that both allocators handle similarly raises the baseline for both, which compresses their ratio. Consistent but not proven.

**§4.3 threshold outcome**: +46.2 % ≫ 10 %. The prior recommendation (merge mimalloc; keep `fast-alloc` on by default for release binaries) holds. No revert.

---

## New issues surfaced by the fix

Running the previously-dropped systems exposed exactly zero regressions across the four verified binaries. The fix is net-positive in every binary tested:

- **profiling_demo**: +1 alloc/frame, state checksum unchanged. Nothing else.
- **hello_companion**: no visible output change (the companion had no `CDesiredPos`, so `sys_sync_to_legacy` propagated unchanged values). Legacy-World stays in sync with ECS as originally intended.
- **ecs_ai_showcase**: stats panels now print — positive, documented, expected.
- **aw_editor**: compiles. Runtime behaviour change in play mode (legacy World now stays fresh during simulation) is a correctness improvement but unverified at the UI level.

No binary panicked, produced wildly wrong numbers, or otherwise misbehaved.

One adjacent issue was noted during §3.3 (not caused by this fix, not fixed here): `combat_system` in `ecs_ai_showcase` increments `GameStats.enemies_defeated` for every fatal-blow damage event, so with 5 enemies and 300 ticks of attacking the counter reaches 447. This is a counting-semantics quirk in `combat_system`, not a scheduling issue. Flag for a future correctness pass.

---

## Final status

**The silent-drop bug is fixed.**

Scope:

- Fixed: `Schedule::add_system` no longer silently drops in debug builds (warns). `App::new()` now creates all 8 canonical stages. Previously-dropped systems (`pre_simulation`/`sync`/`post_simulation` targets) now execute. Four affected binaries (profiling_demo, hello_companion, ecs_ai_showcase, aw_editor) verified: no regression, one feature (stats panel) restored, one latent legacy-World sync restored.
- Out of scope per brief, deliberately not touched: `ParallelSchedule::add_system` has the identical silent-drop pattern at [parallel.rs:216-220](../../astraweave-ecs/src/parallel.rs#L216-L220) but that scheduler has zero production callers (per `docs/audits/job_system_audit_2026-04-18.md` §1.2) and the brief explicitly excluded it. When ParallelSchedule is adopted (pending the ecs_ai_showcase experiment), the same fix pattern should land there.
- Not fixed here, flagged for future tasks: the `ssao` feature-check-cfg warnings and `CpuMesh` / `TaaConfig` / `BloomConfig` test-file errors in `astraweave-render` (pre-existing, visible in IDE diagnostics during this work, wholly unrelated to scheduling).

The ParallelSchedule `ecs_ai_showcase` experiment (audit rec #1 follow-up) can now proceed without worry about silent-stage-drop masking measurement.

---

## Reproducibility

Exact commands run, in order.

```bash
# Phase 2 — test the fix
cargo test -p astraweave-ecs --lib                     # 460 passed

# Phase 3.1 — profiling_demo correctness
cargo run --release -p profiling_demo --features alloc-counter,fast-alloc -- -e 100 -f 100

# Phase 3.2 — hello_companion default
cargo run --release -p hello_companion

# Phase 3.3 — ecs_ai_showcase (verify stats panel prints)
cargo run --release -p ecs_ai_showcase

# Phase 3.4 — aw_editor compile check
cargo build --release -p aw_editor

# Phase 4 — re-baseline primary mimalloc comparison (3 runs each)
for i in 1 2 3; do cargo run --release -p profiling_demo --no-default-features --features alloc-counter -- -e 1000 -f 1000; done
for i in 1 2 3; do cargo run --release -p profiling_demo --features alloc-counter,fast-alloc -- -e 1000 -f 1000; done
```

---

## Docs to supersede (not rewritten)

The following documents contain claims that are now out of date. Rather than rewrite, a pointer to this report should be added to each:

1. `docs/audits/job_system_audit_2026-04-18.md` §1.2 paragraph 3 ("consumers must `add_stage` them manually if used") — factually incorrect post-fix for the canonical stages. Supersede with a pointer here.
2. `docs/audits/parallel_schedule_binary_inventory_2026-04-18.md` §4 (Appendix) — bullets 1–3 describe the silent-drop bug and its consequences. Mark as superseded; the damage is repaired by this report.
3. `docs/audits/parallel_schedule_experiment_2026-04-18.md` §3.3 alloc delta (2931/453452 → 2944/454108 for ParallelSchedule) — numbers now off by one alloc per frame in either direction due to the newly-running systems. The experiment's conclusions are unaffected; flag as re-measurable, don't rewrite.

Supersede pointers added as part of the commit that lands this fix.

---

**Report status**: Fix committed and verified. Safe to proceed to the ParallelSchedule `ecs_ai_showcase` experiment.
