# W.1 — Water Successor: Ratified Deprecation — Execution Report

**Campaign:** W-series (Water Successor) · **Phase:** W.1 · **Mode:** mutation (scoped removal + git tag)
**Branch:** `campaign/water-successor` (branched from `campaign/fluids-f3s` working state)
**Pre-removal anchor:** annotated tag `w0-pre-deprecation` @ `3a8296038`
**Date:** 2026-06-20 · **Status:** complete, build green, stopped at the W.1 gate

This report records the execution of the director's W.1 ratification (removal set
+ three carried ratifications ①②③ + four gate decisions a–d). It is the forensic
companion to [`W0_PRE_DEPRECATION_AUDIT.md`](./W0_PRE_DEPRECATION_AUDIT.md).

---

## 1. Director ratifications executed

| # | Ratification | Action taken |
|---|---|---|
| ① | F.4 accent substrate = Reading A — `FluidSystem` path KEEP | `FluidSystem`, `FluidRenderer`, `optimization.rs`, `sdf.rs`, `lod.rs`, `profiling.rs`, `serialization.rs`, `emitter.rs` + shaders `fluid.wgsl`/`ssfr_*.wgsl`/`secondary.wgsl`/`sdf_gen.wgsl` retained untouched (bodies). |
| ② | Visual-effects layer = DEFERRED (W.3+) | `caustics.rs`, `foam.rs`, `god_rays.rs`, `water_reflections.rs`, `underwater.rs`, `underwater_particles.rs`, `waterfall.rs`, `water_effects.rs`, `anisotropic.rs`, `debug_viz.rs` left in tree, untouched. |
| ③ | `editor.rs` = DEFERRED (editor phase) | `editor.rs` (5,823 LoC) left in tree, untouched. |

## 2. Removal set (firsthand `wc -l`, pre-removal)

**20 files fully deleted = 58,130 LoC** (counted before deletion, confirmed against `git diff`):

- **Voxel sim:** `volume_grid.rs` (1,382), `gpu_volume.rs` (1,676), `building.rs` (1,116), `terrain_integration.rs` (860); tests `voxel_water_f3.rs` (280), `sparse_lockstep_f3s.rs` (283); bench `voxel_sparsity.rs` (136).
- **Research/experimental SPH:** `research.rs` (1,189), `pcisph_system.rs` (1,630), `multi_phase.rs` (1,583), `turbulence.rs` (1,593), `warm_start.rs` (743), `particle_shifting.rs` (738), `viscosity_gpu.rs` (547), `viscosity.rs` (1,337), `boundary.rs` (1,411), `validation.rs` (1,113); shader `shaders/research/pcisph.wgsl` (749).
- **SPH math substrate:** `simd_ops.rs` (39,554 — single largest reclamation).
- **Facade collateral:** `astraweave-water/src/voxel.rs` (210).

**Reconciliation prunes (kept files, surgical) = 666 deletions / 22 insertions:**

- `astraweave-fluids/src/lib.rs`: removed the 9 removed-modules' `pub mod` decls, the `experimental` block, the 5 removed re-exports (`building`/`gpu_volume`/`simd_ops`/`terrain_integration`/`volume_grid`), and 2 stale doc bullets. **`FluidSystem`/`FluidOptimizationController` bodies untouched.**
- `mutation_resistant_comprehensive_tests.rs` (785→456): removed MaterialType/WaterCell/WaterSimConfig/WaterVolumeGrid/CellFlags sections; kept FluidTimingStats/FluidProfiler/FoamConfig/OptimizationStats/Snapshot coverage.
- `fluids_adversarial.rs` (1893→1678): removed `bench_parallel_operations` + `bench_optimized_library_functions` (only production-`simd_ops` consumers) + their registration; kept all bench-local benches.
- `fluid_baselines.rs` (184→149): excised the `WaterVolumeGrid` half; kept the `FluidSystem` half.
- `astraweave-fluids/Cargo.toml`: removed `experimental` feature; flagged orphaned `parallel`/`rayon`; removed `voxel_sparsity` bench entry.
- `astraweave-water` (`Cargo.toml` + `lib.rs`): removed `voxel` feature + optional `astraweave-fluids` dep + `voxel` module wiring; `WaterQuery`/`WaterSample`/`AnalyticWater` API unchanged.

**`git diff --shortstat` (fluids + water): 27 files changed, 22 insertions, 58,796 deletions.**

## 3. Decision (d) — orphaned `parallel`/`rayon`: DEFERRED, not removed

`simd_ops` was their sole consumer; both are now orphaned. Per ratification ④ they are **retained** (the F.4 accent path may want parallelism when the particle path scales) and flagged in `astraweave-fluids/Cargo.toml` + the fluids trace §11 as a known janitorial item for a later W phase.

## 4. Verification (build stays green — verified, not assumed)

| Target | Result |
|---|---|
| `cargo check -p astraweave-water --all-targets` | ✅ |
| `cargo check -p astraweave-fluids --all-targets` / `--all-features` | ✅ |
| `cargo check -p astraweave-physics -p fluids_demo --all-targets` | ✅ |
| `cargo check --workspace --exclude llm_integration` | ✅ (pre-existing warnings only) |
| `cargo test -p astraweave-water` | ✅ 9 passed |
| `cargo test -p astraweave-fluids` | ✅ lib **677**, gpu_execution **7**, mutation **53**, 0 failed |

- **Physics→water seam intact:** `apply_buoyancy_forces` (`astraweave-physics/src/lib.rs:1429`) compiles; its two facade calls — `AnalyticWater::set_plane` (`:1435`) and `WaterQuery::sample` (`:1464`) — resolve.
- **`astraweave-render::WaterRenderer` untouched:** zero render files in the changeset; render compiled clean as a transitive dependency.
- **GPU execution tests pass:** confirms the F.4 accent substrate (`FluidSystem`) executes post-removal.

## 5. Pre-existing, unrelated workspace error (NOT a W.1 regression)

`cargo check --workspace` fails only in `examples/llm_integration/src/main.rs:99` (`DEFAULT_QWEN_INSTRUCT_MODEL` not imported). Proven pre-existing: identical unqualified reference at tag `w0-pre-deprecation`; the crate has no fluids/water dependency; `git diff -- examples/llm_integration astraweave-llm` is empty. Per gate decision (c) this is fixed as a **separate, standalone commit off `campaign/fluids-f3s`**, not on the W branch.

## 6. Gate

Stopped after removal built green and this report + the trace revision landed. **No F.4 or W.2+ work begun** — per the director, the camera campaign (C.1+) takes the queue before F.4 resumes.
