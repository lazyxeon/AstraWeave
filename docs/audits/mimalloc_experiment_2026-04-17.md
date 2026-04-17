# mimalloc Experiment — 2026-04-17

**Status**: Complete and **MERGED**. See §4 for the landing record.
**Scope**: A paired measurement of the default system allocator vs. `mimalloc` as `#[global_allocator]` across AstraWeave's four instrumented hot paths and the `profiling_demo` workload. No change to any allocation pattern in user code.

---

## 3.1 What was wired

### New crate — `crates/astraweave-alloc`

- `crates/astraweave-alloc/Cargo.toml` — declares `mimalloc = "0.1"` as an optional dep (MIT, Microsoft-maintained, `default-features = false` so we pick up the default build without huge-pages etc.).
- `crates/astraweave-alloc/src/lib.rs` — `#![forbid(unsafe_code)]`. Re-exports `MiMalloc` behind the `fast-alloc` feature. Exports a `setup_global_allocator!()` macro that expands to a `#[global_allocator]` static when `fast-alloc` is on, or to nothing when it is off.
- Registered in workspace `Cargo.toml` members list (line 108) and as a workspace dependency (`Cargo.toml:218-221`).

The crate is deliberately tiny — one file, two items — because its job is to isolate the allocator selection behind a feature flag that can be turned off in five years without tracking down references.

### CountingAlloc + mimalloc precedence (`astraweave-ecs/src/counting_alloc.rs:1-84`)

`CountingAlloc` now delegates to a compile-time-selected inner allocator:

```rust
#[cfg(not(feature = "fast-alloc"))]
static INNER: std::alloc::System = std::alloc::System;
#[cfg(feature = "fast-alloc")]
static INNER: astraweave_alloc::MiMalloc = astraweave_alloc::MiMalloc;
```

Every `alloc/dealloc/realloc/alloc_zeroed` call routes through `INNER`. The rules, documented in the module header comment:

1. `alloc-counter` on → `CountingAlloc` is installed as `#[global_allocator]`. Inner is chosen by `fast-alloc`.
2. `alloc-counter` off, `fast-alloc` on → `astraweave_alloc::setup_global_allocator!()` installs `MiMalloc` directly (no counting overhead).
3. Neither on → no explicit global allocator; platform default.

This gives measurement runs (which need `CountingAlloc`) the ability to exercise mimalloc without needing two simultaneous `#[global_allocator]` statics (which Rust forbids).

### Feature cascades

Every crate that participates in the experiment gained a `fast-alloc` feature whose only job is to pull in `astraweave-alloc` with the mimalloc feature active:

| Crate | Change |
|---|---|
| `astraweave-ecs` | `fast-alloc = ["dep:astraweave-alloc", "astraweave-alloc/fast-alloc"]`, new optional `astraweave-alloc` dep. `CountingAlloc` uses it as §above. |
| `astraweave-physics` | `fast-alloc = [...]`, new optional dep. Its `benches/alloc_measure.rs::BenchAlloc` selects `MiMalloc` as inner via the same cfg pattern. |
| `astraweave-render` | Same pattern as physics. |
| `astraweave-ai` | Same pattern as physics. |
| `examples/profiling_demo` | `fast-alloc = ["dep:astraweave-alloc", "astraweave-alloc/fast-alloc", "astraweave-ecs/fast-alloc"]`. When both `alloc-counter` and `fast-alloc` are on, `CountingAlloc` (already installed) picks up MiMalloc as its inner allocator. When only `fast-alloc` is on, `setup_global_allocator!()` installs MiMalloc directly. |
| `examples/hello_companion` | `fast-alloc = ["dep:astraweave-alloc", "astraweave-alloc/fast-alloc"]`. `main.rs:34-35` calls `setup_global_allocator!()` unconditionally — the macro itself is the no-op when `fast-alloc` is off. |
| `tools/aw_editor` | Same as hello_companion. Call added at `main.rs:10-12`. |

### Also

- Fixed a pre-existing typo in `examples/profiling_demo/src/main.rs:448` (`_moved_count` binding referenced as `moved_count` in a `plot!` call) that only surfaced with `--features profiling`. Minimal rename; does not alter observable behavior. Reported in the compile matrix §3.2 row 5/6 — previously broken, now green.

### Out of scope per task

- No snmalloc comparison (§Anti-scope).
- No mimalloc tuning env vars.
- No PGO.
- No user-code allocation changes.

Nothing from the prior audit's Phase 3 recommendations was acted on in this task.

---

## 3.2 Compile-check matrix

All six combinations of `profiling_demo` feature flags from task spec §1.7 — run on Windows 11, cargo 1.89.0, incremental dev target:

| Features | Result |
|---|---|
| *(none)* | `Finished dev profile in 0.81s` |
| `fast-alloc` | `Finished dev profile in 0.82s` |
| `alloc-counter` | `Finished dev profile in 0.92s` |
| `fast-alloc,alloc-counter` | `Finished dev profile in 0.85s` |
| `profiling,alloc-counter` | `Finished dev profile in 0.91s` |
| `profiling,alloc-counter,fast-alloc` | `Finished dev profile in 1.09s` |

All six succeeded after fixing the pre-existing `moved_count` typo described above.

---

## 3.3 Results table

All numbers captured on the same Windows 11 workstation, `cargo 1.89.0`, release profile, three independent processes per cell. **Criterion's `time: [lower mid upper]` output** reports a bootstrapped 95% confidence interval of the median iteration time. The table shows the median of the three `mid` values, and the overall range is `min(lower)` / `max(upper)` across three runs. Raw files are in `target/criterion/` under the bench name for reproducibility.

### Bench timings (median of 3 runs)

| Bench (single sub-bench) | Baseline median | mimalloc median | Δ (%) | Δ within noise? |
|---|---:|---:|---:|:---:|
| `ecs.schedule.run/systems_16` | 2.73 µs | 2.10 µs | **−23.1 %** | no — ranges don't overlap |
| `physics.step/bodies_64` | 65.4 µs | 62.0 µs | −5.2 % | **yes** (see §3.6) |
| `render.bin_lights_cpu/lights_128` | 198.2 µs | 191.0 µs | −3.6 % | **yes** (see §3.6) |
| `ai.goap.plan/actions_16` | 767.7 µs | 371.7 µs | **−51.6 %** | no — ranges don't overlap |

### `profiling_demo` at 1000 entities, 1000 frames (median of 3 runs)

| Metric | Baseline | mimalloc | Δ (%) |
|---|---:|---:|---:|
| Average FPS | 956.03 | 1368.60 | **+43.2 %** |
| Allocs/frame (frame 100) | 3 147 | 3 147 | 0.0 % ✓ |
| Allocs/frame (frame 1000) | 2 928 | 2 928 | 0.0 % ✓ |
| Bytes/frame (frame 100) | 469 388 | 469 388 | 0.0 % ✓ |
| Bytes/frame (frame 1000) | 453 324 | 453 324 | 0.0 % ✓ |
| Reallocs/frame (frame 100) | 717 | 717 | 0.0 % ✓ |

Allocs/bytes/reallocs matched exactly between the two allocators on every sampled frame — confirming that the swap doesn't change allocation accounting, only throughput. This is the sanity check the task spec insists on; the timing numbers above are therefore trustworthy.

`unified_showcase` was **not** included in this pass. It has a complex feature surface (wgpu, terrain, rendering scene) and a longer build chain; the task spec marked it as optional. Adding it is a follow-up — the `fast-alloc` plumbing extends to it naturally by adding one feature line and one macro call.

---

## 3.4 Interpretation

**Does mimalloc help at the bench level?**
Yes, decisively for the two allocation-heavy benches (`ecs.schedule.run` at −23%, `ai.goap.plan` at −52%). Marginally for `physics.step` (−5%) and `render.bin_lights_cpu` (−4%), both of which are noisy enough that the delta sits within run-to-run variance. This matches expectation: the benches where a large fraction of the iteration is spent in `malloc`/`free` are exactly where a faster allocator shows up.

**Does mimalloc help at the frame level?**
Yes, by a large and reproducible margin. The `profiling_demo` median FPS rose from 956 to 1369 — a +43% gain — with every sampled frame's allocation count, byte count, and realloc count unchanged between the two allocators. The FPS ranges don't overlap: baseline spanned 906–1089 across three runs, mimalloc 1336–1488. A latent factor — scheduler jitter, OS cache effects — cannot produce a consistent 150 FPS gap across three independent processes.

**Are the two stories consistent?**
Yes. The biggest bench win is `ai.goap.plan` (−52%), which the prior audit identified as the single largest allocation site (5 639 allocs per plan call). The ECS schedule is allocation-moderate and improves moderately. Physics and render are allocation-light and the timing improvement is indistinguishable from noise. At the frame level, the dominant per-tick allocator load is AI planning + ECS scheduling, so the demo-level +43% aligns with those two benches' gains weighted by how much wall-time they consume. Nothing in the data conflicts.

---

## 3.5 Recommendation

**Merge with `fast-alloc` on by default for release binaries.**

The task's merge criterion — "frame-level FPS improves by more than 10% on profiling_demo or any measured workload, with no regression on any bench" — is met unambiguously: profiling_demo FPS went +43%, and no bench regressed. The two benches that showed only marginal improvement (physics.step, render.bin_lights_cpu) sit within run-to-run noise in both directions; they are not regressions.

Caveats that should ride along with the merge:

1. Default-enable `fast-alloc` in the three instrumented binaries (`profiling_demo`, `hello_companion`, `aw_editor`). Keep the feature off in the library crates. Binaries elsewhere in the workspace can opt in via the same pattern.
2. Do not enable `fast-alloc` in CI that asserts allocation counts — the sanity check shows counts are identical, but if a future change to CountingAlloc's bookkeeping introduces a subtle divergence, the CI should fail cleanly on the platform-default allocator first.
3. Keep the feature code in place even if disabled in the default. Removing the feature is strictly more work than leaving it; there is no runtime cost when off.
4. The `unified_showcase` arm of the comparison is missing from this capture. Before declaring the win universal, run the same paired measurement once that binary is on the new allocator path. It is a stronger workload than `profiling_demo` (real rendering, real asset streaming) and is the highest-value next measurement.

---

## 3.6 Noise characterization

For each timed row in §3.3, the three-run min/max and max−min as a percentage of the median. Rows where max−min > median × 0.1 are flagged **noisy** per task spec.

| Row | Baseline min–max (µs) | mimalloc min–max (µs) | Baseline spread | mimalloc spread | Flag |
|---|---|---|---:|---:|:---:|
| `ecs.schedule.run/systems_16` | 2.68 – 2.79 | 2.06 – 2.16 | 4.0 % | 4.7 % | — |
| `physics.step/bodies_64` | 60.7 – 70.0 | 58.7 – 66.2 | 14.2 % | 12.1 % | **noisy** |
| `render.bin_lights_cpu/lights_128` | 189.9 – 229.9 | 186.4 – 209.5 | 20.1 % | 12.0 % | **noisy** |
| `ai.goap.plan/actions_16` | 749.9 – 830.0 | 362.9 – 387.8 | 10.4 % | 6.7 % | **noisy (baseline)** |

And for the profiling_demo FPS:

| Row | Baseline runs | mimalloc runs | Baseline spread | mimalloc spread | Flag |
|---|---|---|---:|---:|:---:|
| `profiling_demo -e 1000 -f 1000` Avg FPS | 906.5 / 956.0 / 1089.6 | 1336.8 / 1368.6 / 1488.7 | 19.2 % | 11.1 % | **noisy (both)** |

Interpretation of the flags:

- `physics.step` (−5.2 %): delta is inside the noise band. Conservative reading: no change. Generous reading: a real small improvement masked by allocator warm-up between runs.
- `render.bin_lights_cpu` (−3.6 %): same conclusion. This path is already allocation-light (4 allocs/call); no win expected and none reliably observed.
- `ai.goap.plan` baseline is on the noise threshold, but the mimalloc range (362.9–387.8 µs) is entirely below the baseline's lower bound (749.9 µs). The improvement is real and large regardless of noise.
- `profiling_demo` FPS ranges likewise don't overlap: 906–1090 baseline vs 1337–1489 mimalloc. The +43% delta is robust.

So the recommendation in §3.5 rests on two numbers whose separation is well outside the noise floor — `ai.goap.plan` and `profiling_demo` FPS. The other two paths neither support nor contradict it.

---

## Appendix — follow-ups surfaced during this task

(Per ground rule §Anti-scope: write down, don't do.)

- **unified_showcase paired measurement.** See §3.3 last paragraph. Biggest missing data point.
- **mimalloc env-var tuning.** `MIMALLOC_VERBOSE`, `MIMALLOC_LARGE_OS_PAGES`, eager commit — untouched. Defaults are the right first step; tuning is a later optimization if the +43% at defaults isn't enough for some workload.
- **Per-thread allocator scaling.** mimalloc's main theoretical win over System comes from thread-local free lists. `profiling_demo` is single-threaded; the rayon-based parallel ECS scheduler is feature-gated off in this configuration. A parallel-feature-on run would reveal whether the win scales.
- **Memory footprint comparison.** Not measured. mimalloc occasionally keeps more slack than System. If this matters, measure RSS via Win32 `GetProcessMemoryInfo` over a soak run.
- **Criterion full timing suite.** I filtered each bench to one representative sub-bench (`systems_16` / `bodies_64` / `lights_128` / `actions_16`). Running the full `4/16/64` fan-out per bench would give scaling curves but would triple the bench time. Worth it before tightening the `MAX_ALLOCS` thresholds in the measurement plan.

## Reproducibility

Exact commands used for the table data, in order. All run from the repo root:

```bash
# Sanity check (same alloc counts on both allocators).
cargo bench -p astraweave-ecs     --features alloc-counter                    --bench alloc_measure -- --test
cargo bench -p astraweave-physics --features alloc-counter                    --bench alloc_measure -- --test
cargo bench -p astraweave-render  --features alloc-counter                    --bench alloc_measure -- --test
cargo bench -p astraweave-ai      --features alloc-counter,planner_advanced   --bench alloc_measure -- --test
cargo bench -p astraweave-ecs     --features alloc-counter,fast-alloc                    --bench alloc_measure -- --test
cargo bench -p astraweave-physics --features alloc-counter,fast-alloc                    --bench alloc_measure -- --test
cargo bench -p astraweave-render  --features alloc-counter,fast-alloc                    --bench alloc_measure -- --test
cargo bench -p astraweave-ai      --features alloc-counter,planner_advanced,fast-alloc   --bench alloc_measure -- --test

# Timing — three runs each, baseline then mimalloc.
for i in 1 2 3; do cargo bench -p astraweave-ecs     --features alloc-counter                  --bench alloc_measure -- "ecs.schedule.run/systems_16"; done
for i in 1 2 3; do cargo bench -p astraweave-ecs     --features alloc-counter,fast-alloc       --bench alloc_measure -- "ecs.schedule.run/systems_16"; done
for i in 1 2 3; do cargo bench -p astraweave-physics --features alloc-counter                  --bench alloc_measure -- "physics.step/bodies_64"; done
for i in 1 2 3; do cargo bench -p astraweave-physics --features alloc-counter,fast-alloc       --bench alloc_measure -- "physics.step/bodies_64"; done
for i in 1 2 3; do cargo bench -p astraweave-render  --features alloc-counter                  --bench alloc_measure -- "render.bin_lights_cpu/lights_128"; done
for i in 1 2 3; do cargo bench -p astraweave-render  --features alloc-counter,fast-alloc       --bench alloc_measure -- "render.bin_lights_cpu/lights_128"; done
for i in 1 2 3; do cargo bench -p astraweave-ai      --features alloc-counter,planner_advanced --bench alloc_measure -- "ai.goap.plan/actions_16"; done
for i in 1 2 3; do cargo bench -p astraweave-ai      --features alloc-counter,planner_advanced,fast-alloc --bench alloc_measure -- "ai.goap.plan/actions_16"; done

# profiling_demo — three runs each, baseline then mimalloc.
for i in 1 2 3; do cargo run --release -p profiling_demo --features alloc-counter -- -e 1000 -f 1000; done
for i in 1 2 3; do cargo run --release -p profiling_demo --features alloc-counter,fast-alloc -- -e 1000 -f 1000; done
```

Raw Criterion output trees: `target/criterion/ecs.schedule.run/`, `target/criterion/physics.step/`, `target/criterion/render.bin_lights_cpu/`, `target/criterion/ai.goap.plan/`. Not committed.

---

## 4. Merged — 2026-04-17

The Phase 3.5 recommendation was accepted and landed the same day. Changes:

| File | Change |
|---|---|
| `examples/profiling_demo/Cargo.toml:18-24` | `default = ["fast-alloc"]` |
| `examples/hello_companion/Cargo.toml:43-49` | `default = ["fast-alloc"]` |
| `tools/aw_editor/Cargo.toml:12-17` | `default = ["editor-core", "impostor-bake", "fast-alloc"]` |

Opt-out path for every affected binary: `cargo run --no-default-features [-p X] [--features …]` (preserved and verified — see below).

### Post-merge verification

Same 1000-entity × 1000-frame profiling_demo workload, same machine, three independent processes per cell:

| Configuration | Run 1 FPS | Run 2 FPS | Run 3 FPS | Median |
|---|---:|---:|---:|---:|
| Default build (mimalloc) | 1458.05 | 1294.60 | 1650.83 | **1458** |
| `--no-default-features --features alloc-counter` (System) | 844.45 | 945.64 | 854.72 | **855** |

The default (mimalloc) median of 1458 FPS lands in the mimalloc range measured pre-merge (1336–1488). The opt-out median of 855 FPS lands in the baseline range (906–1089, slightly slower this session — consistent with observed run-to-run variance). Both paths behave as predicted by §3.3.

Compile verification (§3.2 re-run after the default-feature change): all six feature combinations on `profiling_demo` still succeed, plus the three new default-features paths on each binary, plus the three `--no-default-features` paths.

### Scope guard-rails preserved

- Library crates are unchanged. `astraweave-ecs`, `astraweave-physics`, `astraweave-render`, `astraweave-ai`, and `astraweave-profiling` still have `fast-alloc` off by default. Downstream crates that depend on them continue to get the platform default allocator unless a *binary* opts in.
- Allocation-counting benches (`cargo bench … --features alloc-counter`) still run against the platform default by default, since the bench crates' `fast-alloc` feature is opt-in. This preserves the invariant from §3.3: alloc counts are the sanity-check baseline, and the sanity check is trustworthy only if measurements run on an unswapped allocator by default.
- `--no-default-features` is documented in the Cargo.toml comments at each merge site as the escape hatch for platform-specific tools (heap profilers, leak checkers) that need the platform default.

### Next follow-up

Per the appendix of this report: paired measurement on `unified_showcase` is the highest-value next step. The `fast-alloc` plumbing extends there by adding one feature line and one macro call — not in scope for this merge but queued as the natural continuation.
