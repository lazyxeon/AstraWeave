# Fluids Integration — F.3 Execution Report

**Sub-phase**: F.3 — Voxel Backend Behind `WaterQuery` (Integration Scope)
**Status**: ✅ COMPLETE
**Branch**: `campaign/fluids-f3`
**Date**: 2026-06-19
**Basis**: `CAMPAIGN_PLAN.md` (Path B), `F2_EXECUTION_REPORT.md` v1.0, F.3 brief
**Scope wall**: `astraweave-water`, `astraweave-fluids` (voxel-grid + gate-flag only — no solver/renderer/editor), terrain glue (cycle-safe), tests/docs. **No `astraweave-render`, no `tools/aw_editor`, no `astraweave-physics`.**

> **One-line thesis.** The existing deterministic `WaterVolumeGrid` voxel sim now satisfies the F.2-proven `WaterQuery` contract, its gate flags are finally *read* (Must-Fix #6), and it carries the four physical-invariant tests F.0 found missing — all without closing the forbidden dependency cycle and without touching sparsity or the 1 ms budget (carved out to F.3.S).

---

## 0. What F.3 is (and is not)

F.3 is the **integration half** of the voxel work. It puts an *existing* sim behind a *proven* trait and makes its behavior trustworthy. Explicitly **out of scope** (carried to F.3.S per the campaign plan):

- ❌ Active-cell sparsity. The grid remains dense.
- ❌ The ~1 ms voxel budget. F.1 measured 64³ dense at **13.8 ms/tick**; that is F.3.S's starting point, not a target F.3 tries to hit.
- ❌ Budget re-ratification (Option A→C). No performance claim is made here; the conversion gate stays closed until F.3.S produces sparse min-spec benchmarks.
- ❌ World-scale chunk stitching / carve-driven re-sim at scale. F.3's terrain glue and carve reactivity are for **bounded** volumes.

---

## 1. Dependency graph — the cycle stays open (Red Line 2)

The forbidden cycle is `physics → fluids → terrain → gameplay → physics`. F.3 adds exactly one new edge: **`astraweave-water → astraweave-fluids`** (feature-gated `voxel`).

**Verified acyclic** (`cargo tree`, 2026-06-19):

```
$ cargo tree -p astraweave-water --features voxel -e normal | grep astraweave
astraweave-water v0.1.0
└── astraweave-fluids v0.1.0     # ← the only astraweave edge; NO terrain/physics/gameplay

$ cargo tree -p astraweave-fluids -e normal --depth 1 | grep astraweave
astraweave-fluids v0.1.0          # ← zero astraweave-* deps: a Cargo leaf
```

`astraweave-fluids` is a leaf (no upward workspace edges), so `water → fluids → (leaf)` cannot close any cycle. The new edge is **feature-gated** so a consumer wanting only `AnalyticWater` never pulls the heavyweight `astraweave-fluids` (wgpu) graph.

The full water chain remains: `physics → water → {glam, fluids(opt)}`, with `terrain → gameplay → physics` above it. `water → terrain` would close the cycle and is **forbidden** — which is exactly why the terrain glue (§4) takes a plain `&[f32]`, not an `astraweave-terrain` type.

---

## 2. WI-1 — `impl WaterQuery for WaterVolumeGrid`

The trait is unchanged from F.2 (`fn sample(&self, point: Vec3) -> Option<WaterSample { surface_height, density }>`). The voxel backend implements it in `astraweave-water/src/voxel.rs` (local trait, foreign type — orphan-rule-clean because `WaterQuery` is owned by `astraweave-water`).

### Sample mapping (precise boundary behavior)

| Input condition | Result |
|---|---|
| `point.xz` outside the grid's XZ footprint | `None` (bounded volume, like an `AnalyticWater` AABB) |
| `point.xz` inside, column has a wet cell | `Some` — surface = topmost wet cell's water top |
| `point.xz` inside, column dry | `None` |
| top wet cell partially full (level `L`) | surface includes the fraction: `origin.y + (cell_y + L) * cell_size` |
| `point.y` (query height) | **ignored** — the caller compares `point.y` to `surface_height` itself, matching the analytic backend's contract |

- **Lateral gate**: `gx = floor((point.x - origin.x)/cell_size)`, same for `gz`; out-of-`[0,dim)` → `None`. This matches the grid's own `world_to_grid`.
- **Surface**: scan the column top-down; the highest cell with `level > SURFACE_MIN_LEVEL` (0.01) is the surface. Cells below that threshold hold only numerical residue and read as dry.
- **Density**: constant `VOXEL_WATER_DENSITY = 1000.0`. The voxel sim models **one** fluid (water) with a per-cell *level*, not a per-cell density — so a constant fresh-water density is the honest report. (A multi-fluid grid would carry density per material; `WaterVolumeGrid` does not.)

### Caller:method table (Red Line 5)

| Element | Real exerciser |
|---|---|
| `WaterVolumeGrid::sample` (the `WaterQuery` impl) | 6 mapping tests + the determinism test, **all through the `WaterQuery` trait** (`voxel.rs::tests`) |
| `apply_terrain_boundary` (WI-4) | `terrain_boundary_blocks_water`, `carve_reactivity_reopens_flow` |
| gate-flag reads in `simulate` (WI-2) | `gate_blocks_horizontal_flow`, `frozen_cell_holds_water`, `persistent_cell_exempt_from_absorption` |
| `VOXEL_WATER_DENSITY` (re-export) | asserted in `sample_returns_column_surface` |

**Honest position on the production consumer.** The *only* production (non-test) caller of `WaterQuery::sample` workspace-wide is `astraweave-physics/src/lib.rs:1464` (buoyancy), and physics holds a concrete `AnalyticWater` (F.2). Running a `WaterVolumeGrid` through physics buoyancy would require generalizing `PhysicsWorld`'s water field to a polymorphic backend — **a physics change, outside F.3's scope wall, and with no real driver today** (Red Line 5 forbids building that polymorphism speculatively). So the voxel backend is **contract-proven through the trait** (mapping + determinism tests consume it as a `WaterQuery`), and its installation into a production consumer is the named next step (physics voxel-backend install, or AI/gameplay), deferred for scope + driver reasons. This is consistent with the WI-6 decision below: no second *consumer* exists, so nothing is promoted and nothing is force-wired.

---

## 3. WI-2 — the gate flags are real (Must-Fix #6)

**The defect.** `WaterVolumeGrid::simulate` wrote `CellFlags` (`building.rs`/editor code set `GATE` on a closed gate) but **never read them**. `flow_vertical`/`flow_horizontal` checked only `material.blocks_flow()`. A "closed" gate let water through — the gate did not gate.

**The fix.** A single `cell_flow_blocked(cell)` predicate — `material.blocks_flow() || flags ∩ {GATE, FROZEN, EDITING}` — is now consulted on **both** the source and target of every vertical and horizontal transfer. Plus:

| Flag | Semantics now enforced | Where |
|---|---|---|
| `GATE` (closed) | blocks flow through the cell | `flow_vertical`, `flow_horizontal` |
| `FROZEN` | iced — no flow in or out | `flow_vertical`, `flow_horizontal` |
| `EDITING` | frozen out of simulation entirely | flow + `process_sources_and_drains` |
| `PERSISTENT` | exempt from natural draining (material absorption) | `apply_absorption` |

**"Does the gate gate?"** `gate_blocks_horizontal_flow` runs the *same* one-channel scenario twice: water at the left, gate cell in the middle. Open → water reaches the far end (`> 0.1`); `GATE` set → far end stays dry (`< 1e-3`). The flag changes the outcome — the proof.

**Honest deferral.** The gate is **binary** (block / pass), not a proportional throttle. `building.rs::WaterGate::flow_multiplier()` computes a `0..1` throttle, but `WaterCell` has **no per-cell multiplier field** to carry it, and adding one is a sim-data-model change beyond "gate-flag fix." F.3 makes the binary gate *real*; proportional throttling is left to a future phase and noted here rather than silently dropped.

---

## 4. WI-3 — physical invariants (the gap F.0 found: zero such tests)

### 4.1 Conservation — a real leak fixed

**The leak.** `flow_horizontal` batched per-cell deltas, then applied them with `clamp(0.0, 1.0)`. A cell receiving from multiple neighbors in one tick could be pushed past 1.0; the excess was **silently discarded** — water destroyed every tick that two inflows met.

**The fix.** Transfers now apply **immediately against live levels**: each transfer is bounded by the recipient's *real* remaining free space (`1.0 - neighbor.level` read live), so it can never overflow and never clamps away water. Iteration order is fixed (`y,x,z` then the fixed direction array) → deterministic. `conservation_closed_basin` (a tall column collapsing in a closed Air basin, no sources/drains/absorption) holds total volume to **< 1% drift over 180 ticks** — and fails hard on the old clamp-leak code.

### 4.2 The four invariant tests

| Invariant | Test | Assertion |
|---|---|---|
| **Conservation** | `conservation_closed_basin` | total volume drift < 1% over 180 ticks |
| **Hydrostatic** | `hydrostatic_column_settles` | a tall column drains downward (top empties, floor layer holds > 60% of the water) and spreads laterally |
| **U-bend** | `u_bend_connected_basins_equalize` | two basins split by a wall, joined only by a floor channel — water poured in one **rises in the other** (the headline capability the docstring advertised but never tested) |
| **dt-stability** | `dt_stability_large_dt_is_substepped` | a single `simulate(3.0)` call stays finite, conserved (< 1%), and in-range `[0,1]` |

### 4.3 dt-stability bound

`simulate(dt)` now substeps any `dt` into chunks `≤ MAX_STABLE_DT = 1/60 s` before stepping. Rationale: flow-per-tick is `flow_rate · 36 · dt`; beyond ~1/36 s at default `flow_rate` the explicit scheme over-transfers then back-transfers (oscillates). Substepping means a large/spiky frame `dt` cannot corrupt state. Tolerances justified by construction: conservation is exact up to f32 transfer error (chosen 1%); range `[0,1]` is enforced by the live free-space bound.

---

## 5. WI-4 — terrain heightmap glue (cycle-safe)

`WaterVolumeGrid::apply_terrain_boundary(&mut self, heights: &[f32], hres_x, hres_z)` takes a **plain `&[f32]`** world-space heightfield — *deliberately not* an `astraweave-terrain` type. This is what keeps the graph acyclic: `astraweave-fluids`/`-water` must never depend on `astraweave-terrain` (that closes `water → terrain → gameplay → physics → water`).

- Cells whose top lies at/below the sampled terrain height → `Stone` (solid, water removed); cells above that were terrain-`Stone` → cleared to `Air`.
- **Consumer wiring is a one-liner**: the code that has both terrain and water passes `heightmap.data()` (which *is* `&[f32]`, F.0 Seam 3) straight in. No adapter, no terrain dep near water.
- Deterministic (fixed iteration order, no RNG/hash). Scope: **bounded** volume over a terrain patch.
- Test `terrain_boundary_blocks_water`: a ridge heightfield makes a solid wall; water poured on the low side pools and never appears inside the ridge.

---

## 6. WI-5 — carve reactivity (bounded)

Re-calling `apply_terrain_boundary` with a **modified** heightfield is the carve path: lower the terrain and previously-solid cells reopen to `Air`. `carve_reactivity_reopens_flow` proves it: water blocked by a ridge, then the ridge is carved flat and re-applied → the formerly-solid cells become `Air` and water reaches the far side on subsequent ticks. (World-scale carve-driven re-sim is F.3.S; this is the bounded-volume reactivity the brief asked for.)

---

## 7. WI-6 — resource-promotion decision (decided on real evidence)

**Trigger (campaign plan):** "the water resource becomes a standalone ECS resource in F.3 *when a second consumer appears*."

**Evidence (grep of every `WaterQuery::sample` caller and `impl WaterQuery`, workspace-wide, 2026-06-19):**

- Production consumers of `WaterQuery::sample`: **exactly one** — `astraweave-physics/src/lib.rs:1464` (buoyancy).
- Backends: **two** — `AnalyticWater` (F.2) and now `WaterVolumeGrid` (F.3).
- AI/gameplay consumers: **none**.

**Decision: KEEP co-location in `PhysicsWorld`.** F.3 added a second *backend*, not a second *consumer*. The promotion trigger is a second consumer, which did not appear. Promoting the water to a standalone ECS resource now would be speculative (the very anti-pattern Red Line 5 guards against). The decision is made on the real call graph, not anticipation — and is re-evaluated the moment AI/gameplay (e.g. the deferred swim branch) calls `sample`.

---

## 8. WI-7 — voxel determinism proof (gate Q1)

`determinism_identical_grids_and_samples`: two independently-built 8³ grids run the identical source-fill + 120-tick sequence (with a `Stone` wall to force varied flow). Asserted **bit-identical** (`f32::to_bits`):

- every cell `level` across the whole grid, and
- `sample` results (surface_height + density) at four probe points, through the `WaterQuery` trait.

This extends the F.2 analytic determinism proof to the voxel backend: the gameplay-water-truth determinism contract (CPU-resident, no GPU/RNG/hash-iteration) holds for both backends.

---

## 9. Deviations & honest gaps

| # | Item | Disposition |
|---|---|---|
| D1 | Gate is **binary**, not proportional | `WaterCell` has no per-cell multiplier; adding one exceeds "gate-flag fix." Documented (§3), deferred. |
| D2 | Voxel backend not installed in a **production** consumer | physics holds concrete `AnalyticWater`; installing a voxel backend needs polymorphic physics water (out of F.3 scope, no driver). Contract-proven via trait tests; named next step (§2). |
| D3 | 4 trivial pre-existing `#[cfg(test)]` clippy fixes in `astraweave-fluids` | caustics excessive-precision, emitter mul-by-(-1), 2× gpu_volume PI-approx — never linted by F.1's `--all-features` gate. Fixed so `--tests -D warnings` is green (F.2 vehicle.rs precedent). Outside voxel-grid/gate sub-area but trivial + bounded. |
| D4 | `apply_terrain_boundary` **owns** column solidity | a bounded-volume assumption (terrain owns the floor); user-placed `Stone` above terrain is reset on re-apply. Documented in the method; fine for F.3's bounded scope. |

---

## 10. F.3.S handoff (sparsity + budget)

F.3.S inherits, with F.3's correctness foundation in place:

1. **Active-cell sparsity** — the grid is dense; `active_cells` exists but the sim iterates the full grid. Sparsity is the budget-C conversion gate.
2. **Min-spec benchmarks** — starting point: F.1's **64³ dense = 13.8 ms/tick**. The ~1 ms target is *unproven* until sparse and measured on min-spec class (GTX 1660 Ti Max-Q class).
3. **Budget re-ratification** — Option A→C conversion happens *only* when F.3.S demonstrates the ~1 ms sparse voxel budget on real hardware. **No budget is ratified here.**
4. **Conversion-cost benchmarks** — carved out per the brief.

F.3 deliberately makes **no** performance claim. It delivers a *correct, deterministic, gate-honoring, conserving* voxel backend behind the proven trait — the thing sparsity will optimize.

---

## 11. Verification gate

| Criterion | Result |
|---|---|
| `cargo tree` acyclic (water→fluids, fluids a leaf) | ✅ §1 |
| Sample mapping + caller:method table | ✅ §2 |
| Must-Fix #6 gate closure ("does the gate gate?") | ✅ §3 |
| 4 invariant tests (conservation/hydrostatic/U-bend/dt) | ✅ §4 |
| Terrain glue cycle-safe (plain `&[f32]`) | ✅ §5 |
| Carve reactivity | ✅ §6 |
| Resource-promotion decision on real evidence | ✅ §7 (keep co-location) |
| Voxel determinism test | ✅ §8 |
| `cargo test` green (fluids voxel tests + water voxel feature) | ✅ §12 |
| `clippy -D warnings` clean (F.3 changes) | ✅ §12 |
| `cargo check --workspace` unbroken by F.3 | ✅ §12 — F.3 crates clean; only the pre-existing `llm_integration` carried item fails (surfaced, not fixed) |
| NO sparsity / NO budget-C / NO render / NO editor / NO physics | ✅ scope honored |

---

## 12. Build & test ledger (verified 2026-06-19)

- `cargo test -p astraweave-fluids --test voxel_water_f3` → **9 passed** (WI-2/3/4/5).
- `cargo test -p astraweave-fluids --lib` → **2259 passed** (unchanged by the flow/flag edits).
- `cargo test -p astraweave-water --features voxel` → **16 passed** (F.2's 9 analytic + WI-1 mapping ×6 + WI-7 determinism ×1).
- `cargo clippy -p astraweave-fluids --tests -- -D warnings` → **clean**.
- `cargo clippy -p astraweave-water --all-targets --features voxel -- -D warnings` → **clean**.
- `cargo tree -p astraweave-water` (default) → **0** `astraweave-fluids` edges (feature gating verified); `--features voxel` → fluids only, fluids a leaf.
- `cargo check --workspace` → fails on **exactly one pre-existing, F.3-unrelated** member: `examples/llm_integration` (`error[E0425]: cannot find value DEFAULT_QWEN_INSTRUCT_MODEL`, `main.rs:94,99`). This is the campaign plan's tracked **carried open item** ("`llm_integration` stale import … pre-existing, trivial, fix when next in that crate"). It depends on **neither** `astraweave-water` nor `astraweave-fluids` (verified: zero edges), so F.3 cannot have caused it. **Surfaced, not fixed** (out of scope per the brief). Every F.3-touched crate (`astraweave-fluids`, `astraweave-water`) compiles clean; voxel module is gated off in the default workspace build, so the F.3 fluids edits are validated by the 2259 lib tests.

---

## 13. Commits

- `3022187a5` — WI-2/3/4/5: voxel gate flags real, conservation leak fix, terrain boundary, carve reactivity (`astraweave-fluids`).
- **this commit** — WI-1/6/7: voxel `WaterQuery` impl (feature `voxel`), sample-mapping ×6 + determinism tests (`astraweave-water`), and this report.

---

**Carried open items** (unchanged from campaign plan): character-controller ground-tolerance defect; `llm_integration` stale import; swim branch (gated on the controller defect). None touched by F.3.
