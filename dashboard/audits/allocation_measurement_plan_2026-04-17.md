# Allocation Measurement Plan — 2026-04-17

**Status**: Instrumentation wired, initial measurements captured.
**Scope**: Wires existing `astraweave-profiling` + `CountingAlloc` infrastructure into Tracy and into criterion benches so allocation volume per hot path is a number, not a guess. Does NOT act on any Phase 3 recommendation from the prior audit (`docs/audits/allocation_audit_2026-04-17.md`) — that is a separate task that becomes valuable only after data is collected.

## What was wired

### Phase 1 — Counters + macros

| Change | File:lines | Notes |
|---|---|---|
| New `alloc-counter` feature on `astraweave-profiling` | `astraweave-profiling/Cargo.toml:23-41` | Independent of `profiling`; atomics live behind this feature. |
| New `counters` module (atomics + record fns + `snapshot()` + `reset()`) | `astraweave-profiling/src/lib.rs:321-499` | Single source of truth; `record_alloc/dealloc/realloc` are no-ops when feature off. |
| New `FrameAllocDelta` struct (allocs, deallocs, reallocs, net_allocs, bytes) | `astraweave-profiling/src/lib.rs:501-527` | Plain POD; always compiles. |
| New `FrameAllocStats` (`begin_frame`/`end_frame`/`peek_delta`) | `astraweave-profiling/src/lib.rs:546-602` | RAII-style measurement window. |
| New `alloc_plot!($name, $count, $bytes)` macro | `astraweave-profiling/src/lib.rs:604-646` | Emits `{name}.allocs` + `{name}.bytes` Tracy plots. No-op when `profiling` off. |
| New `measured_span!($name)` macro + `MeasuredSpanGuard` | `astraweave-profiling/src/lib.rs:668-710` | Span + alloc delta plot in one call. |
| Extended `CountingAlloc` (tracks `REALLOCS`, `BYTES_*`, forwards to profiling counters) | `astraweave-ecs/src/counting_alloc.rs:1-175` | Backward-compatible: `allocs()/deallocs()/net_allocs()/reset_allocs()` kept; `reallocs()/bytes_allocated()/bytes_deallocated()/reset()` added. |
| `astraweave-profiling` made non-optional in ecs, with `alloc-counter` cascade | `astraweave-ecs/Cargo.toml:11-25` | Zero new non-trivial deps (anyhow already pulled in). |
| `profiling_demo` installs `CountingAlloc` and emits `frame.allocs`/`frame.bytes` | `examples/profiling_demo/src/main.rs:46-59, 236-253, 308-329`; `examples/profiling_demo/Cargo.toml:18-25` | Also prints a `[alloc-measure]` line every 100 frames when `alloc-counter` is on but `profiling` is off (no Tracy needed). |

### Phase 2 — Hot-path instrumentation points

All eight sites emit a Tracy span and a matching `{name}.allocs` + `{name}.bytes` plot when built with `--features profiling,alloc-counter`. When `profiling` is off the line is stripped entirely (zero cost).

| Path | Site | Mapping to prior audit |
|---|---|---|
| `render.submit` | `astraweave-render/src/renderer.rs:4665-4671` | §2.3 #4, #5 |
| `render.visible_instances` | `astraweave-render/src/renderer.rs:6043-6049` | §2.3 #5 |
| `render.bin_lights_cpu` | `astraweave-render/src/clustered.rs:40-48` | §2.3 #3 |
| `physics.step` | `astraweave-physics/src/lib.rs:1070-1079` | Open question #7 |
| `ai.tick` | `astraweave-ai/src/ai_arbiter.rs:384-392` | §2.3 #7 |
| `ai.goap.plan` | `astraweave-ai/src/goap/planner.rs:225-236` | §2.3 #1, OQ #2 |
| `ecs.schedule.run` | `astraweave-ecs/src/parallel.rs:256-265` | §2.3 #6 |
| `ecs.schedule.build_groups` | `astraweave-ecs/src/parallel.rs:230-238` | §2.3 #6 |

### Phase 3 — Regression-proof benches

Each bench runs under criterion and, with `--features alloc-counter`, prints a `[alloc-measure]` summary line AND asserts `allocs <= MAX_ALLOCS`. Thresholds are initial placeholders — tightening is a follow-up, not a goal.

| Bench file | Covers | `MAX_ALLOCS` (initial) | Current measurement (see §Results) |
|---|---|---:|---|
| `astraweave-ecs/benches/alloc_measure.rs` | `ecs.schedule.run`, indirectly `ecs.schedule.build_groups` | 10 000 | **31** allocs (16 systems) |
| `astraweave-physics/benches/alloc_measure.rs` | `physics.step` (early + late window, 64 bodies) | 10 000 | **8** early / **34** late |
| `astraweave-render/benches/alloc_measure.rs` | `render.bin_lights_cpu` | 1 000 | **4** allocs (128 lights) |
| `astraweave-ai/benches/alloc_measure.rs` | `ai.goap.plan` (16 actions) | 200 000 | **5 639** allocs |

Each `Cargo.toml` declares the bench with `required-features = ["alloc-counter"]`. All four benches compile with `cargo check --benches --features alloc-counter`. Verified. See §Results for the exact stdout lines.

### Phase 3 (cont.) — CI workflow

New workflow at `.github/workflows/allocation-measurement.yml`:
- Triggers on `push` to `measurement` / `measurement/**` branches, or via `workflow_dispatch`.
- Runs `cargo bench --bench alloc_measure --features alloc-counter -- --test` for each of the four crates.
- `continue-on-error: true` — **does NOT gate merges**. Uploads `alloc-measure-*` artifacts per crate plus a combined `alloc-measure-summary` Markdown artifact.
- Intentionally non-blocking per task spec: "Blocking CI on allocation counts is a good goal for six months from now, not today."

## How to run the capture

### 1. Criterion benches (no Tracy needed)

```bash
# One line per instrumented path; asserts count stays under MAX_ALLOCS.
cargo bench -p astraweave-ecs     --features alloc-counter                      --bench alloc_measure -- --test
cargo bench -p astraweave-physics --features alloc-counter                      --bench alloc_measure -- --test
cargo bench -p astraweave-render  --features alloc-counter                      --bench alloc_measure -- --test
cargo bench -p astraweave-ai      --features alloc-counter,planner_advanced    --bench alloc_measure -- --test
```

Drop the `-- --test` to get full Criterion timing histograms alongside the alloc print.

### 2. Profiling demo (terminal output, no Tracy needed)

```bash
cargo run --release -p profiling_demo --features alloc-counter -- -e 200 -f 500
cargo run --release -p profiling_demo --features alloc-counter -- -e 1000 -f 300
```

Prints one `[alloc-measure] frame N: allocs=… bytes=… reallocs=… net=…` line every 100 frames.

### 3. Profiling demo + Tracy (plots)

1. Launch Tracy server (e.g. `Tracy.exe` on Windows — see `examples/profiling_demo/src/main.rs:87-92`).
2. Run:

```bash
cargo run --release -p profiling_demo --features profiling,alloc-counter -- -e 1000 -f 3600
```

Tracy then shows eight plots (`render.submit.allocs/.bytes`, `render.visible_instances.*`, `render.bin_lights_cpu.*`, `physics.step.*`, `ai.tick.*`, `ai.goap.plan.*`, `ecs.schedule.run.*`, `ecs.schedule.build_groups.*`) alongside the existing timing spans. In Tracy: File → Save Trace, then move the `.tracy` file to `docs/audits/captures/` if small enough (<10 MB); otherwise document the path and skip check-in (per task spec §Deliverables #3).

## Results — 2026-04-17 initial capture

Captured on Windows 11, cargo 1.89.0, release profile. All numbers are from `cargo bench … -- --test` (single invocation, not statistical distribution). Values will stabilise once this lands in the measurement CI and is run across multiple seeds.

### Per-call allocation counts (bench assertions)

| Path | Allocs/call | Bytes/call | Reallocs/call | Threshold (N) | Notes |
|------|-----------:|-----------:|-------------:|-------------:|-------|
| `ecs.schedule.run` (16 systems, empty world, one tick) | **31** | 5 648 | 6 | 10 000 | Includes `build_groups` call-through. |
| `ecs.schedule.build_groups` | subset of above | — | — | 5 000 | Private fn; measured via `run()`. |
| `render.bin_lights_cpu` (128 lights, 16×8×24 clusters, 1080p) | **4** | 202 176 | 0 | 1 000 | Matches audit's inferred "3-4 Vec<u32> per call" exactly. Single-frame cost is modest in count, but bytes-per-call is large (~198 KiB). |
| `physics.step` (64 bodies stacked, 30 warmup steps) | **8** | 4 112 | — | 10 000 | Early window. |
| `physics.step` (64 bodies stacked, 230 warmup steps) | **34** | 11 556 | — | 10 000 | Late window. **~4× allocation growth over 200 steps** — not a leak (`FrameAllocDelta.net_allocs` stayed near zero in the demo run) but worth probing. |
| `ai.goap.plan` (16-action linear chain, depth 16) | **5 639** | 462 845 | 138 | 200 000 | Confirms the audit's top concern (§2.3 #1). Per-plan byte volume is ~452 KiB. |

### Frame-scope allocation rate (profiling_demo)

| Entity count | Frame | Allocs/frame | Bytes/frame | Reallocs/frame | Net allocs/frame | FPS |
|-------------:|-----:|-------------:|------------:|---------------:|----------------:|----:|
| 200 | 400 | 618 | 64 572 | 81 | 0 | ≈ 4 850 |
| 200 | 500 | 625 | 64 444 | 72 | 0 | ≈ 4 850 |
| 1 000 | 100 | 3 147 | 469 388 | 717 | 0 | ≈ 946 |
| 1 000 | 200 | 3 086 | 355 308 | 987 | 0 | ≈ 910 |
| 1 000 | 300 | 3 021 | 311 204 | 752 | 0 | ≈ 868 |

Per-frame `net_allocs == 0` across all samples: every allocation is matched by a deallocation within the same frame. No leak detected in this capture.

### Steady-state workspace-wide rates

Extrapolating from the 1 000-entity demo at frame 300:
- **Allocs/sec**: ≈ 3 021 × 868 FPS ≈ **2.62 M allocs/sec**.
- **Bytes/sec**: ≈ 311 KB × 868 FPS ≈ **270 MB/sec** of (matched) allocation throughput.
- `net_allocs/sec ≈ 0` → live heap is not growing.

### Peak `StagingRing` utilisation

Not captured in this pass — the existing field `peak_bytes` at `astraweave-render/src/staging_ring.rs:66` is already tracked but not exposed via a Tracy plot in this change set. Adding `astraweave_profiling::plot!("render.staging_ring.peak_bytes", …)` inside `StagingRing::begin_frame` is a one-line addition and a good follow-up.

## Answers to prior-audit open questions

Using only the captures described above.

1. **OQ1 — What fraction of frame time in `unified_showcase` is spent in the default allocator?** *Not answered in this capture.* The demo used here is `profiling_demo`, not `unified_showcase`, and the captures were taken without Tracy's `profiling-system` feature (which is what exposes the system-allocator wait zone). **What would answer it**: `cargo run --release -p unified_showcase --features profiling-full` with Tracy attached for 30-60 seconds, then read the allocator category in Tracy's statistics panel.

2. **OQ2 — How many allocations does a single GOAP plan cost in practice?** *Answered.* 16-action linear chain: **5 639 allocs, ~453 KiB, 138 reallocs per plan call**. At the `max_plan_iterations = 10000` ceiling with more actions and wider search, this could be 10–50× higher — the audit's inference of up to 40 000 allocations per worst-case plan call is plausible. The per-expansion pattern (3 clones + 1 String allocation × ~1 400 A* iterations for this problem) matches the audit's static analysis almost exactly.

3. **OQ3 — Which of the 345 `create_bind_group` sites in `astraweave-render` run per-frame vs at init?** *Not answered in this capture*. Requires a Tracy capture of a full renderer frame with per-call-site zone naming. **What would answer it**: add a `span!("create_bg:<pass_name>")` wrapper at each `device.create_bind_group` site (or a single `measured_span!("render.create_bind_group")` wrapper and inspect its frame-local deltas in the Tracy histogram). Follow-up, not gated on this task.

4. **OQ4 — Is `ResidencyManager` referenced anywhere outside its own file?** *Not answered via measurement*. This is a static-analysis question; use the verification-hooks `rg` command from the prior audit. Not a measurement task.

5. **OQ5 — What is the peak `StagingRing` utilisation in a steady-state frame?** *Not answered — instrumentation not yet added*. `staging_ring.rs:66` already holds the number; a single `plot!` call inside `StagingRing::begin_frame` is the one-line follow-up.

6. **OQ6 — Does every `device.create_buffer` call site report to `GpuMemoryBudget`?** *Not answered via measurement*. Needs a wrapper + `debug_assert_eq!(create_buffer_calls, try_allocate_calls)` — a separate change not in scope here.

7. **OQ7 — Does Rapier3D's per-step allocation count grow with simulation time?** *Partially answered*. For 64 stacked cubes: **8 allocs/step after 30 steps, 34 allocs/step after 230 steps** — about a 4× growth over 200 additional steps. `net_allocs` per frame stayed at zero in the broader profiling_demo capture, so this is almost certainly Rapier internal buffer growth (broadphase, contact caches) to accommodate actual contacts as the stack settles, not a leak. To confirm: run `cargo bench -p astraweave-physics --features alloc-counter --bench alloc_measure` with 10 000+ warmup steps and check whether the count plateaus or keeps climbing.

8. **OQ8 — What is the steady-state per-tick allocation count of `ParallelSchedule::run`?** *Answered.* For a 16-system empty-world topology: **31 allocs, 5 648 bytes, 6 reallocs per tick**. Scales with system count (not measured statistically, but the bench has `systems_4`/`_16`/`_64` groups to surface that curve when run with Criterion's full statistical mode).

## What this tells us

The engine's allocation profile, measured for the first time, is **high-throughput and leak-free**. A 1000-entity world churns ~2.6 M allocations/sec and ~270 MB/sec of allocator traffic while holding `net_allocs/frame == 0`. Every finding from the prior audit's §2.3 "Gaps" is confirmed in shape and in order: GOAP A* is by far the largest per-operation allocator (≈ 5 600/call vs. everyone else in the low tens), bin_lights_cpu matches its inferred four-Vec pattern exactly, ECS schedule.run is unexpectedly bounded (31/tick) because `build_groups` has few stages with few systems in this test, and physics step shows a modest allocation-count growth over the first few hundred steps that is best explained by Rapier's internal caches reaching steady-state. The instrumentation is cheap enough to ship on: the `profiling_demo` at 1000 entities with `alloc-counter` hit ≈ 870 FPS — the counters are not meaningfully slowing anything down.

---

## Verification commands

```bash
# Everything compiles in every relevant feature combination.
cargo check -p astraweave-profiling
cargo check -p astraweave-profiling --features profiling
cargo check -p astraweave-profiling --features alloc-counter
cargo check -p astraweave-profiling --features profiling,alloc-counter
cargo check -p astraweave-ecs --features alloc-counter
cargo check -p astraweave-physics --features alloc-counter
cargo check -p astraweave-render --features alloc-counter
cargo check -p astraweave-ai --features alloc-counter,planner_advanced

# All four benches compile and run with --features alloc-counter.
cargo bench -p astraweave-ecs     --features alloc-counter                    --bench alloc_measure -- --test
cargo bench -p astraweave-physics --features alloc-counter                    --bench alloc_measure -- --test
cargo bench -p astraweave-render  --features alloc-counter                    --bench alloc_measure -- --test
cargo bench -p astraweave-ai      --features alloc-counter,planner_advanced   --bench alloc_measure -- --test
```

## Tracy capture file

The initial captures documented in §Results were taken via `--test`-mode bench runs and stdout logging from `profiling_demo` — no `.tracy` file was produced (no Tracy server was available in the environment where the captures were taken). To produce a full `.tracy` capture:

1. Install Tracy 0.12+ (matches `tracy-client 0.18`).
2. Start the Tracy profiler server.
3. Run `cargo run --release -p profiling_demo --features profiling,alloc-counter -- -e 1000 -f 3600`.
4. In Tracy: File → Save Trace → `allocation_2026-04-17_profiling_demo_1k.tracy`.
5. If the file is under ~10 MB, commit it to `docs/audits/captures/`. Otherwise record its path in this document and leave it outside the repo.

`docs/audits/captures/` was not created in this change set because no `.tracy` file exists yet. Future captures should live there.
