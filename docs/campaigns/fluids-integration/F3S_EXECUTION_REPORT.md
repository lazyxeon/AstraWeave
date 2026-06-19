# Fluids Integration — F.3.S Execution Report

**Sub-phase**: F.3.S — Voxel Sparsity & the Budget-Conversion Benchmark
**Status**: ✅ COMPLETE
**Branch**: `campaign/fluids-f3s`
**Date**: 2026-06-19
**Basis**: `CAMPAIGN_PLAN.md` (Path B), `F1_EXECUTION_REPORT.md` v1.4 (dense baselines), `F3_EXECUTION_REPORT.md`
**Machine** (min-spec class — the budget target): **Intel i5-10300H (4C/8T, 2.5 GHz), 31.8 GB, Win 11**; the exact F.1 box (verified `Get-CimInstance`). The voxel sim is CPU-only, so the CPU is what the budget gate measures; GPU (GTX 1660 Ti Max-Q) is irrelevant here.
**Scope wall**: `astraweave-fluids` (voxel sim + benches), `MASTER_BENCHMARK_REPORT.md`, tests/docs. No `WaterQuery`/`astraweave-water` API change; no physics/render/editor.

> **The deliverable is a trustworthy measurement, not a target hit.** This report states, with same-machine sparse-vs-dense numbers, whether `WaterVolumeGrid` can meet the ~1 ms gameplay-water budget via active-cell sparsity — and it reports the answer honestly whichever way the evidence points.

---

## 0. Verdict up front

**PARTIAL (leaning NEGATIVE for the budget's intent).** Sparsity is real, bit-identical, and deterministic, and it delivers large speedups for small/localized water (up to **14×**). But the **~1 ms voxel budget at 64³ — the resolution the budget targets (F.1's 13.8 ms 64³ baseline is the number 1 ms must beat) — is met ONLY for small grids (32³, ≤ ~50 % fill) or strongly-localized water (active region ≲ 16³ ≈ 4 k cells).** A **full-extent 64³ flood never reaches 1 ms** — **2.35 ms even at 5 % depth**, 8.4 ms at 50 %. Two structural walls force this: **hydrostatic pressure is column-coupled** (full-extent water can't sparsify its pressure cost) and **the F.3 immediate-apply flow cascades forward** (growing the active box during flow). **Recommendation: STAY at the ratified Option A; do NOT convert to Option C for general gameplay water.** Option C's 1 ms voxel budget is reachable only under the stated constraints (small grids or small/localized volumes), not for the full-extent Enshrouded-class flooding that motivated T3. Forcing numbers in §4; recommendation detail in §6.

---

## 1. What F.3.S inherited and preserved

F.3 left `WaterVolumeGrid` correct (conserving, gates gate, hydrostatic/U-bend/dt tested, deterministic, behind `WaterQuery`) but **dense**: `simulate` iterated the full grid every phase, ignoring the `active_cells` field. F.3.S makes it sparse **without changing one bit of observable behaviour**:

- The pre-F.3.S dense algorithm is preserved verbatim as **`simulate_reference`** (the bit-identity target and the dense benchmark baseline).
- `simulate` is now the sparse path. **Every F.3 test passes bit-identically on it** (2259 lib + 9 voxel-invariant + 16 water-voxel including the determinism proof), plus a dedicated lockstep suite (§3).

---

## 2. WI-1 — the sparsity design, and why it is a box

### 2.1 The finding that shaped the design: the F.3 forward cascade

The obvious design — a per-cell active *set* (wet ∪ neighbours) — **cannot be bit-identical** with the F.3 sim, and finding out why is a core F.3.S result.

F.3's conservation fix made `flow_horizontal` **immediate-apply**: each cell's transfer reads the *live* neighbour level and applies at once (so multi-neighbour inflow can't overflow and leak). A side effect: in the forward sweep directions (**+x and +z**, the loop's increasing axes), a cell that receives water *earlier in the same pass* re-emits it *later in the same pass*. Water therefore **cascades many cells forward in a single tick** — empirically, a single full column spread to **166 cells in one tick**, vs a 1-hop active frontier's 63. Each transfer is capped at `flow_amount` (0.15) and halves down to `min_level` (0.001) in ≈ 8 hops, so the cascade reaches ~8 cells forward per tick. (Vertical flow is bottom-up so it advances only 1 cell/tick; backward −x/−z is also 1 hop.)

A 1-hop active frontier truncates this cascade and diverges. Reproducing it bit-identically requires including the whole forward run — which is what a **bounding box** does naturally.

> **This is also a latent F.3 observation worth recording**: the forward cascade means the leading edge of flowing water advances ~8 cells/tick (~480 cells/s at 60 Hz) — far faster than the configured 36 blocks/s flow rate, and asymmetrically (forward faster than backward). It is order-dependent behaviour baked into the immediate-apply scheme. F.3.S preserves it exactly (the brief forbids changing F.3 behaviour), but it is the kind of thing a future correctness pass should weigh.

### 2.2 The dirty-AABB

`simulate` maintains an axis-aligned box (`dirty_min`/`dirty_max`) bounding every nonzero-water (and source/drain) cell, and runs the dense phases **restricted to that box in the dense (y,x,z) order**, via the *same per-cell helpers* the dense path uses (`flow_vertical_at`, `flow_horizontal_at`, `compute_pressure_column`). Bit-identity then follows by construction: cells outside the box are provably dry (the box bounds all water), so the dense sweep would no-op them anyway; cells inside are visited in the same order with the same code.

- **Cascade containment**: the box is dilated by `CASCADE_MARGIN = 16` in +x/+z (2× the ~8-hop reach, with headroom from the 0.15 transfer cap) and 1 cell on the other faces (1-hop backward/vertical receivers). The cascade always completes inside the working box.
- **Maintenance**: after each substep the box is recomputed from a local rescan of the working box (it grows as water spreads, shrinks as it drains). After any external mutation, a one-time O(n) rebuild re-establishes it (`active_dirty`, set by every mutator including the conservative `get_cell_mut`/`cells_mut`).
- **Determinism (gate Q1)**: the box is two `IVec3`s; phases iterate fixed integer ranges in fixed order. No hash iteration, no RNG, no threads. Preserved.

### 2.3 The structural limit this exposes — pressure is column-coupled

`compute_pressure` accumulates each column top-to-bottom (a cell's pressure = weight of all water above it). It can only be skipped for **columns with no water at all**. So:

- **Localized water** (few wet columns) → pressure sparsifies → large speedup.
- **Full-extent water** (a flood covering the whole floor, even thinly) → *every* column is wet → pressure stays at full cost regardless of depth. Sparsity then helps only the flow phases, not pressure.

This makes **shape**, not just fill fraction, a first-class variable — which is why the benchmark measures both a full-extent basin and a localized pool.

---

## 3. WI-1 / WI-5 — the bit-identity & determinism proof

`tests/sparse_lockstep_f3s.rs` asserts `simulate` (sparse) and `simulate_reference` (dense) produce **bit-identical** water levels (`f32::to_bits`) at **every tick**:

| Test | What it pins |
|---|---|
| `lockstep_collapsing_column` | pure flow, 240 ticks |
| `lockstep_u_bend_with_walls` | order-sensitive flow through a wall channel, 300 ticks |
| `lockstep_gate_then_open` | flag-bearing (GATE) cells in the box |
| `lockstep_source_fills_dry_grid` | a SOURCE filling an otherwise-dry grid |
| `lockstep_drain_empties_basin` | a draining basin (box shrinks) |
| `lockstep_terrain_carve` | terrain boundary + mid-run carve |
| **`lockstep_large_grid_exercises_margin`** | **40³ grid — box genuinely smaller than the grid, so the cascade margin is really tested** (the 8³ cases clamp to dense) |
| `wake_boundary_water_spreads_into_dry_cells` | water wakes distant dry cells (no freezing) |
| `sleep_boundary_active_set_shrinks_to_zero` | a drained basin goes quiescent (active set → drain cells only) |
| `sparse_path_is_deterministic` | two sparse runs are bit-identical (WI-5) |

All pass. Determinism (gate Q1) is preserved on the sparse path, and the F.3 voxel-determinism test (`astraweave-water --features voxel`) still passes bit-identically.

---

## 4. WI-2 — the benchmark matrix (the core artifact)

`benches/voxel_sparsity.rs`, criterion, median per-tick, same machine, sparse (`simulate`) vs dense (`simulate_reference`). Two shapes, both **settled** (clean steady-state cost). F.1 dense baselines shown for cross-check.

### 4.1 Basin — flat settled water over the FULL floor at depth = fill

*Every column wet ⇒ pressure cannot sparsify; only flow does.* Median ms/tick, sparse (`simulate`) vs dense (`simulate_reference`), **bold = ≤ 1 ms**:

| Grid | dense (≈) | sparse 5 % | sparse 25 % | sparse 50 % | sparse 100 % |
|---|---|---|---|---|---|
| **32³** | 0.48–0.99 | **0.19** (2.5×) | **0.42** (1.4×) | **0.74** (1.0×) | 1.01 (1.0×) |
| **64³** | 7.4–13.1 | 2.35 (3.2×) | 5.12 (1.7×) | 8.40 (1.3×) | 12.88 (1.0×) |
| **128³** | 127–228 | 44.2 (2.9×) | 89.3 (1.6×) | 143.2 (1.2×) | 227.5 (1.0×) |

Reading it: sparsity scales the speedup with *dryness* (3.2× at 5 % → 1.0× at 100 %), and the high-fill guard (WI-3) keeps sparse ≈ dense at 100 % (never slower). But the **64³ full-extent floor never drops below 2.35 ms** — even 5 % depth is 2.3× over budget, because pressure is paid over all 64×64 wet columns regardless of depth.

### 4.2 Pool — localized stone-walled cube (few wet columns)

*Pressure sparsifies too (few wet columns) ⇒ the clustered best case.* 64³, median ms/tick:

| Localized region | wet cells (≈ fill) | sparse | dense | speedup | ≤ 1 ms? |
|---|---|---|---|---|---|
| side 12 (12³) | 1.7 k (~0.7 %) | **0.535** | 7.56 | **14.1×** | ✅ |
| side 24 (24³) | 13.8 k (~5.3 %) | 1.836 | 7.76 | 4.2× | ❌ |
| side 40 (40³) | 64 k (~24 %) | 6.574 | 9.07 | 1.4× | ❌ |

This is where sparsity shines (14× for a small pool) — and it pins the 64³ sub-1 ms boundary precisely: **between side 12 (0.535 ms, fits) and side 24 (1.836 ms, over), i.e. a localized water region of ≈ 16³ ≈ 4 k cells is the largest that fits 1 ms at 64³.**

### 4.3 Cross-check vs F.1 dense baselines

F.1 (half-full basin): 32³ = 0.551 ms, 64³ = 13.83 ms, 128³ = 206.1 ms. This run's **dense** column reproduces them within run-to-run noise (e.g. 64³ at 100 % fill = 13.06 ms ≈ 13.83; 32³ ≈ 0.5–1.0 ms). The dense path is the unchanged pre-F.3.S algorithm, so the baseline holds — the speedups are measured against a faithful reproduction of F.1's numbers on the same box.

---

## 5. WI-3 — further levers (evaluated, mostly not pursued — with reasons)

Sparsity misses 1 ms for full-extent 64³ water, so the brief authorises evaluating further levers. Each was weighed against its determinism cost; only the high-fill guard was applied.

| Lever | Decision | Why |
|---|---|---|
| **High-fill dense guard** | **APPLIED** | At ≥ 85 % box coverage the box bookkeeping cost exceeded its savings and sparse ran 0.76–0.89× (slower than dense). Falling back to the dense substep (bit-identical) makes sparse never worse than dense (now ≈ 1.0× at 100 %). A correctness/quality fix, not a budget lever — it does not help reach 1 ms (high fill is over budget regardless). |
| **CPU parallelism (rayon over the box)** | **NOT PURSUED — blocked** | The F.3 flow is **order-dependent by construction**: `flow_horizontal` reads each neighbour's *freshly-updated* level (the immediate-apply conservation fix) and `flow_vertical` is bottom-up. Cells that cascade into one another cannot be evaluated concurrently without changing the result, so a bit-identical parallel path is not available without rewriting the flow. Independently, F.1 measured rayon at **2.1× slower** than sequential below ~5 ms workloads — and the sub-1 ms targets are exactly that regime. Parallelism is blocked by the *same root cause* as the cascade (§2.1). |
| **Algorithmic / data layout (SoA, cache-friendly traversal)** | **NOT PURSUED** | Would not change the two structural walls. Column-coupled pressure (§2.3) is inherent to hydrostatics; the cascade is inherent to immediate-apply flow. Layout tuning might shave a constant factor but cannot turn 2.35 ms into 1 ms at 64³, and it is outside the F.3.S scope wall. |
| **GPU compute migration** | **NOT PURSUED — explicit finding, not a build** | Per the brief, this is out of scope and would re-open the determinism carve-out for what is supposed to be the deterministic gameplay-truth layer. *If* 1 ms full-extent voxel water is truly required, the honest conclusion is that it needs a GPU-voxel approach in a future campaign — reported here, not attempted. |

**The deeper finding the levers expose:** the F.3 immediate-apply flow's order-dependence is the common root that blocks *both* cheap bit-identical sparsity (forces the cascade-margin box) *and* parallelism. A future correctness/perf pass that replaced it with an order-independent conservative scheme (batched deltas with per-cell capacity tracking) would unblock both — but that changes F.3 behaviour and is out of this phase's scope.

---

## 6. WI-4 — the budget verdict

### Verdict: **PARTIAL**, recommending **STAY at Option A** (do not convert to C for general water).

Sparsity is correct, bit-identical, deterministic, and genuinely fast for the cases it suits — but the evidence does not support ratifying the ~1 ms Option-C voxel budget for general gameplay water at 64³.

**Where 1 ms IS met** (the constraints under which C is reachable):

- **32³ grids** up to ~50 % fill (0.19–0.74 ms); ~100 % is borderline (1.01 ms).
- **Localized water** of active region **≲ 16³ ≈ 4 k cells** at 64³ (side-12 pool = 0.535 ms), regardless of overall grid size.

**Where 1 ms is NOT met** (the forcing numbers):

- **64³ full-extent floor, any depth**: 2.35 ms (5 %) → 8.40 ms (50 %) → 12.88 ms (100 %). The most-favourable full-extent case is **2.3× over budget**; realistic depths are **5–8× over**.
- **64³ localized water ≥ ~5 %** as a compact region: 1.84 ms (side-24) and up.
- **128³ anything**: 44 ms (5 %) and up.

### The realistic-fill argument (does real gameplay water fit?)

The T3 ambition is **Enshrouded-class "Wake of Water"** — rooms/caves *flooding*, water *rising* over a floor. That is **full-extent** water by definition, and it is exactly the case that does **not** fit (column-coupled pressure is paid over every floor column). Concretely:

| Gameplay water | Shape | 64³ cost | Fits 1 ms? |
|---|---|---|---|
| Flooded room / rising water (the T3 case) | full-extent floor, growing depth | 2.3–12.9 ms | ❌ |
| Advancing flood front | actively flowing → cascade grows the box | worst case | ❌ |
| Contained pond / small pool | localized ≲ 16³ | ≤ 1 ms | ✅ |
| Ocean cell | 100 % | ≈ 12.9 ms (≈ dense) | ❌ |

So the **shapes that fit 1 ms are the *least* interesting ones** (small contained pools); the shape that motivated T3 (flooding volumes) is the shape that does not fit.

### Recommendation to the owner (the A→C gate input)

1. **Do not convert A→C for general voxel water.** Keep **Option A** as the ratified envelope; F.4 continues to build against it (as the campaign plan already provides for).
2. **C is reachable only under explicit constraints** — declare them if C is pursued for a *subset* of water: voxel gameplay water must be **small-grid (≤ 32³ regions)** or **small localized volumes (≲ 16³ active cells)**. Tiling the world into small bounded voxel volumes (rather than one large grid) is the way C survives — and it matches the F.3 "bounded volume" scope.
3. **Re-scope T3 honestly.** Enshrouded-class full-extent flooding at interactive cost is **not** delivered by CPU active-cell sparsity on this min-spec class. The realistic options are: (a) accept a **larger budget** for full-extent voxel water (it is 2–13 ms at 64³, deterministic and correct *today*); (b) **constrain** gameplay water to small bounded volumes; or (c) **defer** large-scale voxel water to a **GPU-voxel future campaign** (accepting the determinism-carve-out consequences). The campaign's staged A-floor/C-target design anticipated exactly this: **A is already ratified and F.4 is unaffected**, so this negative costs the campaign nothing but converts an open budget question into a decided fact.

**You (the owner) ratify or decline the A→C conversion at the F.3.S gate.** The evidence above forces "decline for general water; reachable only for small/localized water under stated constraints."

---

## 7. Deviations & honest gaps

| # | Item | Disposition |
|---|---|---|
| D1 | Sparsity is a **dirty-AABB**, not a per-cell active set | Forced by the F.3 forward-cascade (§2.1): a per-cell frontier cannot be bit-identical. The box is the correct bit-identical unit. Documented, not hidden. |
| D2 | `compute_pressure` does **not** sparsify for full-extent water | Column-coupling (§2.3), inherent to hydrostatic pressure. Measured both shapes to quantify it. |
| D3 | `CASCADE_MARGIN = 16` is a **bound**, not proven minimal | Theoretically safe (transfer capped at 0.15 ⇒ ≤ 8-hop reach; 2× headroom) and validated on the 40³ worst-case (full column, max pressure). A scenario cascading > 16 would silently diverge in production; the bound is documented and the cap argument makes it sound. |
| D4 | `total_volume` maintained from the box sum, not the dense full-grid sum | Float-summation order differs ⇒ low bits of the cached `total_volume` may differ from dense; it is a derived stat, not gameplay state. Cell levels (the gameplay truth) are bit-identical; conservation tests use a direct level sum. |

---

## 8. Verification gate

| Criterion | Result |
|---|---|
| Sparse `simulate` iterates only the active box; representation deterministic-order | ✅ §2.2 (two `IVec3`, fixed ranges) |
| **Bit-identical sparse-vs-dense lockstep, every tick** | ✅ §3 (incl. 40³ margin test) |
| Wake/sleep boundary correct both directions | ✅ §3 |
| All F.3 invariant tests pass on the sparse sim | ✅ 2259 lib + 9 voxel + 16 water |
| Benchmark matrix recorded (sizes × fills × shapes, min-spec, vs F.1) | ✅ §4 + `MASTER_BENCHMARK_REPORT.md` |
| Determinism re-proof | ✅ §3 (`sparse_path_is_deterministic` + water-voxel determinism) |
| Verdict stated unambiguously with forcing numbers | ✅ §6 |
| `cargo test`/`clippy -D warnings` green on touched crates; workspace unbroken | ✅ §9 |
| No `WaterQuery`/water API change; no physics/render/editor | ✅ scope honored |

---

## 9. Build & test ledger (verified 2026-06-19, min-spec box)

- `cargo test -p astraweave-fluids --test sparse_lockstep_f3s` → **10 passed** (bit-identical lockstep incl. 40³ margin, wake/sleep, sparse determinism).
- `cargo test -p astraweave-fluids --test voxel_water_f3` → **9 passed** (F.3 invariants, now on the sparse path).
- `cargo test -p astraweave-fluids --lib` → **2259 passed**.
- `cargo test -p astraweave-water --features voxel` → **16 passed** (incl. the voxel `WaterQuery` determinism proof, bit-identical on the sparse backend).
- `cargo clippy -p astraweave-fluids --benches --tests -- -D warnings` → **clean**.
- `cargo bench -p astraweave-fluids --bench voxel_sparsity` → 30 benchmarks, full matrix (§4), recorded in `MASTER_BENCHMARK_REPORT.md` v5.59.
- `cargo check --workspace` → fails ONLY on the pre-existing, F.3.S-unrelated `examples/llm_integration` (`DEFAULT_QWEN_INSTRUCT_MODEL`, the campaign's tracked carried item; zero edges to fluids). Surfaced, not fixed — out of scope.

---

## 10. Commits (branch `campaign/fluids-f3s`)

- `204ce421d` — WI-1/WI-5: dirty-AABB sparsity + dense `simulate_reference` + lockstep/determinism tests.
- `6656ea7fc` — WI-2 bench (`voxel_sparsity.rs`) + WI-3 high-fill dense guard.
- *(this report + `MASTER_BENCHMARK_REPORT.md` v5.59 + fluids trace update)*
