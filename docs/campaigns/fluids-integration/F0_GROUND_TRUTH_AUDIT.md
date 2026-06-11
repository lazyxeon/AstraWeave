# F.0 Ground-Truth Audit — Fluids Integration Campaign

**Document version**: 1.0
**Audit window**: 2026-06-10 → 2026-06-11
**Commit audited**: `8e1505dd863845098d2c7b84f39d829a327a7da5` (HEAD, branch `main`, clean working tree)
**Auditing agent**: Claude (Fable 5), read-only forensic pass; five parallel evidence agents + first-hand verification of all gate-level claims
**Mandate**: READ-ONLY. This file is the only write performed by this audit.

---

## Preamble

### Pre-flight outputs (verbatim)

**`cargo check -p astraweave-fluids`** (default features) — **PASS, zero warnings**:

```
    Checking astraweave-fluids v0.1.0 (D:\AstraWeave-AI-Native-Gaming-Engine\astraweave-fluids)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 32.18s
```

**`cargo check -p astraweave-fluids --features parallel`** — **PASS, zero warnings**:

```
    Checking astraweave-fluids v0.1.0 (D:\AstraWeave-AI-Native-Gaming-Engine\astraweave-fluids)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.78s
```

**`cargo test -p astraweave-fluids -- --list`** — **2,585 listed** (2,480 lib unit tests + 99 integration tests + 6 doc-tests, all doc-tests `ignore`d). Full suite executed during WS1.4: **2,579 passed, 0 failed** in 12.96 s wall (see §1.4).

**`git log --oneline -15 -- astraweave-fluids/`**:

```
4af95b47c Implement rain splash particle system, shader permutation system, snow footprint stamping, and vegetation interaction system
2702232fb style: fix formatting across all workspace crates for CI compliance
452c66339 astraweave-fluids: replace caustics golden test with multi-point + chromatic mutation killers
239b309c2 fix: remove cargo-mutants comment remnant from caustics.rs
df2bde50a astraweave-fluids: fix caustics golden test (pattern_scale=3.0 to catch *= vs += mutation)
2ce45f2be astraweave-fluids: add 7 mutation-killing tests for caustics.rs (presets, time, depth, golden noise)
90d277e1a astraweave-fluids: add 4 exact-value mutation-killing tests for simd_ops batch kernels
5d9df6662 astraweave-fluids: add 12 mutation-killing tests for foam.rs (config presets, RNG, source intensity)
0900a4a64 astraweave-fluids: add mutation-killing test for emitter jitter+velocity calculations
37b6b64b6 astraweave-fluids: add SKIP_GPU_TESTS env guard to gpu_volume test helper
44667f055 astraweave-fluids: add 10 mutation-killing tests for boundary.rs (config presets, kernel math, as_kinematic)
5af614b14 astraweave-fluids: add 4 more precise mutation-killing tests for gpu_volume sampling/positions
b7d40343b astraweave-fluids: add 5 more mutation-killing tests for gpu_volume fallbacks and sampling
e7a4a274c astraweave-fluids: add targeted mutation-killing tests for generate_surface_mesh
8f17c8779 astraweave-fluids: add 27 mutation-killing tests for gpu_volume.rs
```

**Trace currency check**: `git log --oneline 32afac52f..HEAD -- astraweave-fluids/` returned **empty** — zero fluids commits since `docs/architecture/fluids.md` v1.2's verification commit. The crate source audited here is byte-identical to what the trace describes; every drift item in §1.6 is therefore an error *in the trace*, not staleness.

**Machine context** (all timings/benches in this report): Intel Core i5-10300H @ 2.50 GHz (4C/8T), NVIDIA GeForce GTX 1660 Ti Max-Q + Intel UHD integrated, 31.8 GB RAM, Windows 11 Home 10.0.26200, Rust 1.89.0.

**Required-reading deviation**: the campaign brief lists `docs/architecture/terrain.md`; that file does not exist. The terrain trace is `docs/architecture/terrain_materials.md`, which was used instead. All other required reading (`CLAUDE.md`, `ARCHITECTURE_MAP.md`, `fluids.md`, `physics.md`, full fluids source/tests/benches) was performed.

### Headline (read this first)

1. The crate compiles clean both ways, runs 2,579 green tests in 13 s, and carries a 100%-adjusted mutation score — **and none of that exercises a GPU solver step**. `FluidSystem::new` is never called by any test; no test anywhere asserts a physical invariant across simulation ticks.
2. The one demo-validated solver (`FluidSystem`, PBF) has a **gate-level correctness defect**: its ping-pong machinery simulates two independently-evolving particle states on alternating frames, each at half rate, with the renderer always handed the stale one (§1.1-A, Must-Fix #1). Verified first-hand at shader and host level.
3. `UnifiedSolver::step` — the crate's flagship-named, root-re-exported solver — **is a no-op** (`unified_solver.rs:527-542`: a 10-step algorithm exists only as comments; the body is `self.frame_count += 1;`). `ResearchFluidSystem`, inventoried by the architecture trace as an active GPU pipeline, **does not exist as a type**. DFSPH/IISPH are enum variants with no solver loop anywhere in the crate.
4. The crate contains a second, underappreciated simulation paradigm: `WaterVolumeGrid` (`volume_grid.rs`), a **CPU cellular-automaton voxel water sim with hydrostatic-pressure-assisted flow, material absorption, and Enshrouded-inspired design** — fully deterministic by construction. It is the campaign's strongest asset for the T3 ambition, and it currently has zero callers, zero conservation tests, and a dead gate mechanism.
5. The GPU particle solvers are **non-deterministic by construction** (atomic linked-list neighbor ordering × float non-associativity), and `FluidSystem` additionally makes its iteration count a function of async GPU timing. Honest integration requires a publicly documented determinism carve-out for particle state — or building gameplay truth on the CPU voxel/analytic layer instead (§4, §6).

---

## WS1 — Fluids Crate Ground Truth

### 1.1 Simulation model inventory

Five solver/manager surfaces coexist. What each *actually* computes (every claim re-verified against source; doc-comments treated as hypotheses):

#### A. `FluidSystem` (lib.rs + `shaders/fluid.wgsl`) — real GPU Position-Based Fluids, with two gate-level defects

The original and only demo-consumed solver. A genuine wgpu compute pipeline: 7 pipelines compiled from `fluid.wgsl` (`lib.rs:364-367`, pipelines `lib.rs:751-779`).

- **Kernel**: cubic B-spline `kernel_w` + analytic gradient (`fluid.wgsl:95-117`). No poly6/spiky/Wendland in this shader.
- **Pressure solver**: textbook Macklin–Müller PBF. `compute_lambda` (`fluid.wgsl:167-225`): `C = ρ/ρ₀ − 1`, `λ = −C/(Σ|∇C|² + ε)`, hardcoded `ε = 100.0` (`:214-216`). `compute_delta_pos` (`:233-309`): `Δp += (λᵢ+λⱼ+s_corr)·∇W` with tensile-instability `s_corr` (`:288`). Density is `Σ W` with **no mass factor** (`:196-197`).
- **Viscosity**: XSPH with **hardcoded coefficient 0.01** (`fluid.wgsl:402-425`). The user-facing `params.viscosity` does NOT control XSPH — it scales **vorticity confinement** (`:396-400`). `params.pressure_multiplier` is declared (`:24`) and **never read by any kernel** — a dead uniform the demo UI nonetheless exposes as a slider.
- **Surface tension**: simplified cohesion-only term labeled "Akinci et al. 2013"; the curvature half is explicitly skipped (`fluid.wgsl:290-297`).
- **Boundary**: global SDF sampled in `integrate` with gradient pushout (`fluid.wgsl:319-341`); friction/damping **commented out** (`:343-346`). The SDF is regenerated **every frame** by a 3-pipeline Jump-Flood pass (`SdfSystem::generate`, called at `lib.rs:1041`; `sdf.rs:24-27, 228-246`). Analytic dynamic-object collision exists in the shader (`:243-264`) but `step()` hardcodes `object_count: 0` (`lib.rs:1033`) — only `update_objects` callers (the demo lab scenario) activate it. Fallback hard clamp to a **hardcoded world box** `[-29.5,29.5]×[0,59.5]` with `+30.0` domain offsets baked into the shader (`fluid.wgsl:71-73, 321, 349-358`) — the sim domain is not configurable without shader edits.
- **Neighbor search**: 128³ uniform grid (`lib.rs:417-420`), atomic linked lists — `atomicExchange` insertion (`fluid.wgsl:159-160`), list-walk traversal in every summation kernel.
- **Timestep**: externally supplied `dt`, clamped `dt.min(0.016)` (`lib.rs:1027`). **No substepping, no CFL bound.** The demo feeds wall-clock dt (`examples/fluids_demo/src/main.rs:480,490`). Iterations adaptive 2–8 (`lib.rs:809, 833`).
- **Entry point**: `pub fn step(&mut self, device, encoder, queue, dt)` (`lib.rs:1009-1220`) — records into a caller-provided encoder; the host submits.

**DEFECT 1 — ping-pong state divergence (verified first-hand).** Buffer 0 is created initialized, buffer 1 created **empty** (`lib.rs:395-412`). The two bind groups swap which buffer sits at binding 0 vs 1 (`lib.rs:700-729`), and `step` alternates them per frame (`current_src = frame_index % 2`, `lib.rs:1044-1048`). But every WGSL kernel reads **and writes binding 0 in place**; binding 1 (`particles_dst`) is literally annotated `// Reserved for full state copy if needed` and never written (`fluid.wgsl:56-57`). The only host-side buffer copy is the 4-byte density-error copy (`lib.rs:1183`). Net effect: **even frames simulate buffer 0, odd frames simulate buffer 1 — two independently-evolving particle states, each stepped every other frame (effective 30 Hz at 60 fps), synchronized only when `spawn_particles`/`reset_particles` write both buffers** (`lib.rs:847-851, 898-902`). `get_particle_buffer` (`lib.rs:1222-1231`) carries a comment describing a copy-to-dst design the shader does not implement, and returns the buffer *not* simulated in the frame just encoded. The demo "looks right" because both streams share spawn writes, identical params, and statistically similar evolution — this is exactly the class of silently-wrong-but-visually-plausible defect the engine's audit history warns about.

**DEFECT 2 — despawn is GPU-invisible (verified first-hand).** `particle_flags` (0=inactive/1=active) is created and host-written (`lib.rs:311, 659-663, 854, 906, 989`) but appears in **no bind group** and **no WGSL declaration** (grep `flags` over `fluid.wgsl`: zero matches). Despawned particles keep simulating; dispatches always cover `params.particle_count`, never `active_count`. The despawn logic itself runs against a CPU position cache documented as stale during GPU simulation (`lib.rs:316-320, 922-929`).

**DEFECT 3 — readback issued before submit.** `map_async` on the density-error staging buffer is issued inside `step` (`lib.rs:1215`) *before the caller has submitted the encoder containing the copy*, with only a non-blocking `device.poll` (`:1219`). What the mapping observes depends on submission interleaving. Its output feeds `self.iterations` (`:1207`) — see §4 (WS4) for the determinism consequence.

#### B. `PcisphSystem` (`pcisph_system.rs` + `shaders/research/pcisph.wgsl`) — real GPU PCISPH, fixed-iteration, dead convergence machinery

Actual type name is `PcisphSystem` (`pcisph_system.rs:245`), not "PCISPHSystem" as docs render it. A genuine self-submitting GPU pipeline (owns `Arc<Device>`/`Arc<Queue>`, `:302-308`; 11+ entry points compiled at `:549, :565`). `step(dt)` (`:908-1110`) runs predict → grid build → **fixed-count** pressure loop (`max_iterations.min(50)`, `:967-969`) of density → pressure-solve → apply-pressure (+ per-iteration grid rebuild) → Morris viscosity → optional δ-SPH shifting → integrate → optional vorticity. Physics kernels use **Wendland C2** (`pcisph.wgsl:168-188`); a cubic-spline definition exists but is unused (`:136-165`). Warm-starting from `previous_pressure` is real (`pcisph.wgsl:248-252`).

**The convergence apparatus is dead**: zero `map_async`/`poll` anywhere in the file; the `IterationState` buffer is created and bound but **no kernel writes it**; per-particle `density_errors` are written (`pcisph.wgsl:330-332`) but never reduced or read; `DEFAULT_DENSITY_THRESHOLD` is unreachable config. The header's "<0.1% density error after convergence" (`:14`) has no implementing code path. The PCISPH stiffness δ uses an admitted empirical fudge (`sum_term = -0.5`, "would be computed properly in production", `:899-904`). Boundary is a plain box clamp (`pcisph.wgsl:583-613`); `dynamic_objects` is bound but referenced by no kernel. **Zero callers outside its own tests.**

#### C. `ResearchFluidSystem` — does not exist

`research.rs` contains **zero wgpu code and no system struct**. It is a types/config module (`SolverType`, `ViscositySolver`, `KernelType`, `ResearchParticle`, `ResearchSimParams`, `ResearchFluidConfig`, `ValidationMetrics`, `FluidPhase`). The name `ResearchFluidSystem` occurs only in an `ignore`d doc-comment example (`research.rs:26, :35`) and in documentation — including the architecture trace, which inventories it as an active GPU pipeline (drift item §1.6-D1).

#### D. `UnifiedSolver` (`unified_solver.rs`) — config shell with a no-op step

No wgpu anywhere in the file. **`pub fn step(&mut self, _particles: &mut [ResearchParticle])` ignores its input, contains a 10-item "In a full implementation, this would:" comment, and executes `self.frame_count += 1;`** (`unified_solver.rs:527-542`, verified first-hand). Consequently every `UnifiedSolverConfig` field is execution-dead; the `TurbulenceSystem` it constructs is `#[allow(dead_code)] // Reserved for future use in step()` (`:464-465`); `MetricsHistory` is never written. **DFSPH/IISPH status**: enum variants exist and presets select them, but no DFSPH/IISPH solver loop exists anywhere in the crate. CPU building blocks exist in `simd_ops.rs` (an `IisphState` with O(n²) brute-force precompute, `simd_ops.rs:4205-4352`; DFSPH per-particle helpers `:19545, :19611, :19650, :19972`) — **all callers are in-file tests**. `PcisphSystem` actively *rejects* DFSPH/IISPH configs (`pcisph_system.rs:1171-1172`). Verdict: variant-level aspirational, helper-level tested-but-dormant.

#### E. `WaterVolumeGrid` (`volume_grid.rs`) — real, deterministic CPU cellular-automaton voxel water

The second paradigm, and the audit's most consequential finding for T3. `simulate(dt)` (`volume_grid.rs:397-419`) runs: per-column hydrostatic pressure accumulation (`:422-440`) → in-place sequential vertical flow at Enshrouded-style 36 blocks/s (`:443-491`, fixed bottom-to-top order) → gather-then-apply horizontal flow where flow potential = level diff + 0.01·pressure diff, enabling U-bend behavior (`:494-570`) → material absorption → sources/drains → cleanup. **Iteration order is fully deterministic**: fixed nested `y/x/z` loops, fixed direction array, linear `Vec<WaterCell>` storage, no HashMap, no RNG, no threads.

Honest limits: raw `dt` with no stability bound (saturates rather than explodes); **dense** iteration over all cells every tick — the `active_cells` set exists (`:163`) but `simulate` ignores it (the "sparse simulation" idea is aspirational); **no GPU compute path for the sim** — `gpu_volume.rs` is upload + CPU heightfield-column surface meshing only (zero `ComputePipeline` matches; `generate_surface_mesh` at `gpu_volume.rs:356` is real CPU meshing, not marching cubes); the `CellFlags::GATE/FROZEN/EDITING/PERSISTENT` flags are **never read by the sim** (Must-Fix #6); and no test asserts conservation (§1.4).

### 1.2 Public API surface

Full grouped inventory and per-export usage status: ~140 exported symbols across 13 groups (core solver, unified/research, voxel water, building, 7 visual-effects families, terrain integration, ~40 editor types, optimization/LOD/profiling, rendering, serialization, debug, SIMD kernels, emitters) — re-export block `lib.rs:88-203`, plus 11 `pub mod`-only modules (`lib.rs:53-86`).

**Usage classification** (workspace-verified):

- **Demo-exercised (the live set, ~7 of ~140 top-level symbols)**: `FluidSystem` (new/step/spawn_particles/reset_particles/update_objects/get_particle_buffer/secondary accessors/public fields), `FluidRenderer`, `renderer::CameraUniform`, `Particle`, `DynamicObject`, `FluidLodManager`+`FluidLodConfig`, `FluidOptimizationController` (12 methods). Canonical pattern: `examples/fluids_demo/src/main.rs` — `FluidSystem::new(&device, 20_000)` (`:286-287`), direct public-field configuration (`:290-298`), per-frame LOD gate → `step_with_budget` or `step` (`:530-563`), render via `FluidRenderer::render` with an explicitly-built `CameraUniform` (`:600-608`; `scenarios/laboratory.rs:135-148`).
- **Integration-test-only**: `FluidSnapshot`/`SnapshotParams`, `WaterVolumeGrid`/`WaterCell`/`WaterSimConfig`/`CellFlags`/`MaterialType`, `FluidProfiler`/`FluidTimingStats`, `FoamConfig`, `OptimizationStats`.
- **Bench-only**: 6 `simd_ops` functions (`fluids_adversarial.rs:1749, 1774, 1785, 1810-1812`).
- **Dormant (exported, exercised by nothing beyond inline unit tests)**: everything else — building, all 7 visual-effects families (incl. the three inline-WGSL constants `CAUSTICS_WGSL`/`GOD_RAYS_WGSL`/`SSR_WGSL`, compiled into **no pipeline anywhere**), terrain_integration, the entire editor surface, unified_solver, emitter, gpu_volume, serialization-beyond-tests, debug_viz, `FluidRenderContext`, and most of optimization/LOD.

**External-consumer grep (every hit)**: `astraweave-fluids` appears in exactly 3 Cargo.tomls — its own, the root member list (`Cargo.toml:139`), and `examples/fluids_demo/Cargo.toml:9`. **fluids_demo is confirmed the only consumer crate.** Incidental finding: `examples/fluids_demo/src/scenarios/{splash,waterfall}.rs` are **orphan files** — `scenarios/mod.rs:1-2` declares only `laboratory` and `ocean`. `docs/src/api/fluids.md` references ≥13 nonexistent types (`FluidWorld`, `PcisphConfig`, `CausticRenderer`, `WaterTerrainIntegration`, …) — consistent with the known 28bc94f21 aspirational-wiki hazard.

### 1.3 Lying-code check

Performed and ruthless. Clean axes first: **zero `todo!()`/`unimplemented!()`/`panic!("not implemented")` in src**; the two bare `Ok(())` returns found are legitimate.

**CRITICAL — public API that silently does nothing:**

| # | Finding | Evidence |
|---|---|---|
| C1 | `UnifiedSolver::step` is a no-op (frame counter only); entire `UnifiedSolverConfig` surface execution-dead | `unified_solver.rs:527-542` (verified first-hand) |
| C2 | `WaterGate` cannot block water: `apply_to_grid` sets `CellFlags::GATE` (`building.rs:338-347`), applied every tick (`:550-553`), but the voxel sim never reads GATE — flow checks only `material.blocks_flow()` and level (`volume_grid.rs:443-570`); `flow_multiplier()` has zero non-test callers. FROZEN/EDITING/PERSISTENT flags equally unread | `building.rs:329-347`; `volume_grid.rs:86-96, 443-596` |
| C3 | `ReferenceData::load_csv(_path)` ignores the path and returns `Ok(Self::default())` — fakes successful load | `validation.rs:367-370` |

**HIGH — config accepted and ignored:**

| # | Finding | Evidence |
|---|---|---|
| H1 | Temporal coherence does nothing: `set_temporal_coherence` sets a flag; `TemporalCoherence::should_simulate` has zero callers; `resting_particles` stat is always 0 | `lib.rs:1245-1272, 1336`; `optimization.rs:666` |
| H2 | GPU-vendor workgroup tuning never reaches the GPU: `step()` dispatches with hardcoded `div_ceil(64)`; `optimal_*_workgroups` helpers uncalled in the step path | `lib.rs:1043, 1101-1105, 1391-1398` |
| H3 | LOD/streaming computes but never applies: `StreamingOp`s produced (`lod.rs:943-956`) are consumed by nothing; `FluidRenderContext.lod_particle_factors` has zero consumers; LOD never reduces particle counts (only the demo's frame-skip LOD is real, `lod.rs:65-96`) | `lib.rs:2067-2210`; `lod.rs:943-1001` |
| H4 | `serde` feature gates nothing: `default=["serde"]` declared but zero `#[cfg(feature="serde")]` sites; 5 files `use serde::` unconditionally → **`--no-default-features` cannot compile** | `Cargo.toml:6-9`; `building.rs:7`, `editor.rs:43`, `serialization.rs:5`, `terrain_integration.rs:11`, `volume_grid.rs:8` |
| H5 | Divergence error hardcoded to 0.0 ("Placeholder") in `ValidationMetrics` — always reports perfect divergence | `validation.rs:81-84` |
| H6 | `SimParams.pressure_multiplier` is a dead uniform (declared `fluid.wgsl:24`, read by no kernel) yet exposed as a live demo slider; `params.viscosity` controls vorticity confinement, not viscosity (XSPH coefficient hardcoded 0.01) | `fluid.wgsl:24, 396-400, 402-425` |

**MED — dormant modules** (zero non-test production callers, all verified by caller grep): `simd_ops.rs` (39,554 LoC — **the crate's own GPU solvers never call it**; the only src reference is the `lib.rs:171` re-export), `multi_phase.rs` (1,583), `warm_start.rs` (740), `particle_shifting.rs` (738), `turbulence.rs` (1,593 — constructed by UnifiedSolver, never ticked), `viscosity_gpu.rs` (544 — header self-admits `// GPU VISCOSITY SYSTEM (Placeholder for wgpu integration)`, `:281`), `pcisph_system.rs` (real GPU code, zero callers), `anisotropic.rs` (real pipeline, zero callers), `editor.rs` (5,823 — real undo/validation/TOML I/O, zero consumers in `tools/`), the `water_effects.rs` tree (~6,000 LoC — `update` genuinely delegates per-subsystem, *not* a shell, but zero workspace callers), `terrain_integration.rs` (real D8 flow-accumulation analysis, zero callers), `gpu_volume.rs`, `volume_grid.rs`+`building.rs` (real sim; gate dead-ends per C2), `serialization.rs` (real roundtrip, **detached** — no capture-from/restore-to `FluidSystem` exists), `emitter.rs`.

**Orphan/phantom shader files**: `src/shaders/viscosity_morris.wgsl` (644 LoC) has **no `include_str!` consumer** (doc-comment references only); `viscosity.rs:503` cites `viscosity_implicit.wgsl`, **which does not exist anywhere**.

**LOW**: `simd_ops.rs:16969` (`_beach_slope` ignored), `:17767` (`_parent_diameter` ignored); `boundary.rs:860` `compute_density` returns 0 ("Would require SDF field"); `sdf.rs:398` mesh-voxelization caveat; demo dead UI (`target_particle_count` buttons never applied, `show_foam` never read, right-drag force advertised in help text but unimplemented — `examples/fluids_demo/src/main.rs:773-812, 854, 1164-1166`); `println!` debug spam in `FluidRenderer` production paths (`fluids/renderer.rs:54, 87, 100, 492`).

### 1.4 Test and bench reality

**Suite run (verbatim summaries)**, `cargo test -p astraweave-fluids`, exit 0, 12.96 s wall:

```
running 2480 tests ... test result: ok. 2480 passed; 0 failed; 0 ignored ... finished in 9.64s
running 99 tests  ... test result: ok. 99 passed; 0 failed; 0 ignored ... finished in 0.01s   (integration)
running 6 tests   ... test result: ok. 0 passed; 0 failed; 6 ignored                          (doc-tests)
```

A real GPU adapter was present; `SKIP_GPU_TESTS` was not needed. Note the guard's failure mode (`gpu_volume.rs:540-543`): on headless machines GPU-dependent tests early-return **and count as passed** — a green run does not prove GPU paths executed.

**Classification of the 2,579 tests** (sampled across all major files):

| Category | Share | Notes |
|---|---|---|
| Exact-value CPU kernel/vector math | ~53-55% | `simd_ops.rs` alone has 1,378 tests |
| Config/preset/default/serde/clamp smoke | ~30-35% | the 99 integration tests are 100% this class |
| Struct-size/bytemuck/GPU-layout | ~4-5% | |
| Construction/getter/Display | ~5% | |
| Runs any sim step with a physics assertion | **<1% (~10-15 tests)** | all directional-only |

**Headline answer: NO test verifies end-to-end simulation correctness against a physical invariant.** No mass-conservation-across-ticks, no hydrostatic-column, no energy-decay test exists. The closest: `volume_grid.rs`'s 4 directional flow tests (e.g. `test_vertical_flow` asserts top level fell and bottom rose, `:786-801`); `viscosity.rs`'s Morris test asserts velocity *direction* changes, never sums momentum (`:1192-1226`). The **U-bend physics advertised in `volume_grid.rs:4-5` has no test anywhere**. The conservation framework purpose-built for this (`validation.rs`: `ValidationMetrics::compute`, `is_research_grade()` thresholds, `BenchmarkConfig::hydrostatic()/dam_break()`) **has no runner and is fed only hand-constructed literals in its own tests** — `test_benchmark_config_hydrostatic` asserts `config.name.contains("Hydrostatic")`, a string check. Self-referential pattern noted in `pcisph_system.rs`: `test_pcisph_delta_computation` (`:1340`) re-derives the formula inside the test and asserts on its own arithmetic. **`FluidSystem::new` is never called by any test** (only call sites: demo + README). `PcisphSystem` and `UnifiedSolver` are never stepped in tests (UnifiedSolver's `reset` test sets `frame_count` manually).

**Mutation testing** (`docs/current/FLUIDS_MUTATION_TESTING_REPORT.md` v1.0.0, 2026-03-01): 411 viable mutants, 405 caught, 6 equivalent → adjusted 100.0%; **45 GPU-dependent mutants in lib.rs excluded** ("wgpu device initialization required") → grand score incl. GPU 88.8%. **Staleness**: the report references ≥11 module files that no longer exist (surface_reconstruction.rs, buoyancy.rs, solver.rs, …) — the crate was refactored after March; results no longer map 1:1. Coverage (`MASTER_COVERAGE_REPORT.md:187`): line 89.27% (2026-02-25, 2,509 tests then; live count now 2,579 — drift).

**Bench reality** (`benches/fluids_adversarial.rs`, 1,893 LoC, 12 groups / ~54 functions): the file's own header says it — `// LOCAL TYPES (Mirror astraweave-fluids API)` (`:10`). **~45 of ~54 bench functions measure bench-local re-implementations or mocks** (`MockOptimizationController` `:858`, `MockLodManager` `:1191`, a toy SPH loop `:575-698`, local kernels `:120-145`). Only ~9 functions (groups `parallel_operations`, `optimized_functions`) call production crate code, and those are CPU `simd_ops` micro-functions **that no production path calls** (§1.3-MED). **No bench creates a GPU device; no bench measures a production solver step.** `docs/current/MASTER_BENCHMARK_REPORT.md` contains **zero fluids entries** — no baseline has ever been recorded.

### 1.5 Parallel feature audit

`parallel = ["dep:rayon"]`, **not** in default features (`Cargo.toml:7-9`). Every `cfg(feature = "parallel")` site: `simd_ops.rs:685` (the `parallel` module, 7 functions), `:2022` (`par_compute_densities_spatial`), `:23292` (tests), plus 3 bench gates. **All 8 gated functions are element-wise `par_iter` maps or indexed collects; per-particle float sums are computed sequentially inside each element. No parallel float reduction (`.sum()`/`reduce`/`fold` over a par_iter) exists** — the classic rayon non-deterministic-summation hazard is structurally absent. The parallel path is **deterministic**.

However: **no solver path calls any `par_*` function.** `FluidSystem::step`, `WaterVolumeGrid::simulate`, and `WaterEffectsManager::update` contain zero `simd_ops` calls. Callers are benches and in-file tests only; `par_compute_densities_spatial` has zero callers anywhere including benches. **The `parallel` feature currently parallelizes nothing that runs.** Consequence for WS4: the feature is irrelevant to integration determinism today; it becomes relevant only if a CPU solver path (e.g. voxel grid at scale) adopts it — and the existing functions show the deterministic pattern to follow.

### 1.6 Trace drift list (`docs/architecture/fluids.md` v1.2)

Source is unchanged since the trace's verification commit, so each item is a trace error or omission:

| # | Claim | Verdict |
|---|---|---|
| D1 | `ResearchFluidSystem` inventoried as active research-grade GPU pipeline (fluids.md:261, §5 research.rs row, §6) | **WRONG — the type does not exist.** research.rs is wgpu-free config/types; the name occurs only in an `ignore`d doc example (`research.rs:26, :35`) |
| D2 | §2.1/§8 describe `FluidSystem` as a functioning ping-pong pipeline | **WRONG in effect** — ping-pong exists structurally but produces alternating-frame state divergence (Must-Fix #1); invariant 21's "2-entry ping-pong assumption throughout the code" conceals the defect |
| D3 | "8 WGSL shaders total" (fluids.md:8, 36) | **UNDERCOUNT** — a 9th file exists: `src/shaders/viscosity_morris.wgsl` (644 LoC), consumed by nothing; trace's §5 omits it. Bonus: `viscosity.rs:503` references the nonexistent `viscosity_implicit.wgsl` |
| D4 | `SimParams` at lib.rs:221-248 | Minor — actual `lib.rs:229-248`; `:221-227` is `DynamicObject`. Sizes (80 B / 64 B / 304 B CameraUniform) all CONFIRMED |
| D5 | "only 2 unsafe occurrences crate-wide" | Confirmed for src/; caveat: `benches/fluids_adversarial.rs:845-849` contains one `unsafe {}` block (`static mut SEED`) |
| D6 | PCISPH "<0.1% density error after convergence" perf note repeated at §9 | Misleading — no convergence check exists (fixed iterations, dead readback machinery, §1.1-B) |
| D7 | Type name "PCISPHSystem" | Actual: `PcisphSystem` (`pcisph_system.rs:245`) |
| D8 | §5 "Active (module-level)" status for unified_solver.rs | Understates: `step` is a no-op (C1); "Active" implies executable simulation |

Confirmed-accurate spot-checks (8 of 10): `FluidOptimizationController` at lib.rs:1433; no `#![forbid(unsafe_code)]`; SolverType collision + lib.rs:187 re-export; Cargo features; line counts (lib.rs 3,810 / simd_ops.rs 39,554 / editor.rs 5,823 exact); single 785-LoC integration test file; 8 named shaders' `include_str!` consumers.

**Drift found in OTHER documents** (incidental, report-worthy): CLAUDE.md "Where to Look" cites `astraweave-physics/src/character_controller.rs` — **file does not exist** (controller lives in `astraweave-physics/src/lib.rs:424-535`); `docs/architecture/render_pipeline_material_system_shader_infrastructure.md:233` upstream table claims "astraweave-fluids → SPH water particle data → water.rs" — **no such dep or import exists**; `docs/src/api/fluids.md` references ≥13 phantom types; `FLUIDS_MUTATION_TESTING_REPORT.md` references ≥11 deleted module files; `MASTER_COVERAGE_REPORT.md` fluids test count stale (2,509 → 2,579).

---

## WS2 — Integration Seam Map

All six seams' current state is "none" (zero engine wiring). Per-seam: what integration requires, risk, path-dependence. Full file:line detail retained from the seam agents' verification.

### Seam 1 — ECS (`astraweave-ecs`/`astraweave-core`) — Risk: MED

**Current state**: deterministic single-threaded 8-stage schedule (`Schedule::run` is a sequential nested loop, `astraweave-ecs/src/lib.rs:726-735`); systems are bare `fn(&mut World)` pointers (`:688`); **only 7 of the 8 stages have named constants** — `SystemStage` (`lib.rs:93-103`) lacks SYNC, which exists only as the raw literal `"sync"` (`:783-792`; consumed by literal at `astraweave-core/src/ecs_adapter.rs:221`). **Unknown stage names silently drop the system in release** (`lib.rs:718-724`). No system touches wgpu today; no GPU-owning type is an ECS resource anywhere; rendering is pull-based extraction outside the schedule (editor `EngineRenderAdapter` → `Renderer::draw_into`). No central tick driver exists — `App::run_fixed(steps)` just loops; 60 Hz is per-call-site convention (`PhysicsConfig::default().time_step = 1/60`, `astraweave-physics/src/lib.rs:618`; net server `astraweave-net/src/lib.rs:545-548`). No frame-accumulator/substep pattern exists in core/ecs.

**Integration requires**: `World::insert_resource` (`lib.rs:466`) of a fluids manager mirroring the `PhysicsPlugin` precedent (`astraweave-physics/src/ecs.rs:13-23`); a decision on GPU access — either (a) clone `wgpu::Device`/`Queue` into resources (wgpu 25 handles are `Send+Sync+Clone`; would be a **first-in-codebase precedent**) or (b) keep GPU stepping host-driven outside the schedule like rendering, with only CPU state (params, spawns, coupling, queries) in stage systems. Stage choice SIMULATION vs PHYSICS is secondary to that decision. dt source must be reconciled: fluids `step` takes variable dt, physics convention is fixed 1/60 — no accumulator exists to bridge them.

**Path-dependent**: Path C (voxel-first) needs no GPU-in-ECS decision at all (CPU sim is an ordinary resource); Paths A/B need the host-driven-GPU pattern for the SPH backend.

### Seam 2 — Physics (`astraweave-physics`, Rapier3D 0.22) — Risk: MED

**Current state**: **three parallel water abstractions already exist** — exactly the §7.7 wrapped-component trap the campaign must not worsen:
1. **Working flat-plane buoyancy**: `add_buoyancy(body, volume, drag)` (`lib.rs:1413-1416`), `apply_buoyancy_forces` (`:1418-1447`, Archimedes + linear drag vs scalar `water_level` field `:921-922`), called inside `step_internal` before Rapier integration (`:1084`).
2. **Unwired parallel system**: `environment.rs` `WaterVolume`/`EnvironmentManager` with `buoyancy_force_at`/`is_underwater`/`water_drag_at`/`water_current_at`/waves (`environment.rs:350-568`) — never wired into `step`.
3. **No-op stubs**: `pub fn add_water_aabb(&mut self, _min, _max, _density, _linear_damp) {}` (`lib.rs:1449`, quoted in full — empty body, all params ignored) and `clear_water` (`:1555`). Callers are a test and a demo (both no-op victims).

`CharState` has exactly **one variant: `Grounded`** (`lib.rs:391-395`, `#[non_exhaustive]`); zero `swim` hits crate-wide. Force APIs exist and are public: `apply_force` (`:1108-1114`), `apply_impulse` (`:1116-1122`). Rapier is **without** `enhanced-determinism` (root `Cargo.toml:194`) — same-machine determinism only (`tests/determinism.rs` covers that). ECS wiring (`physics_step_system`, `ecs.rs:26-30`) is feature-gated and dormant; production callers step `PhysicsWorld` directly.

**Integration requires**: reconcile the three abstractions into one fluid-query-backed surface (generalize `apply_buoyancy_forces` to sample per-position surface height/density instead of the scalar plane, or implement `add_water_aabb` as the facade shim); add a swim `CharState` variant + buoyancy/drag branch in `control_character` (`:1247-1342`; `#[non_exhaustive]` → audit all dispatch sites per Integration Completeness #2); resolve a flagged hazard — Rapier user forces added via `add_force` persist until `reset_forces`, and **`reset_forces` has zero call sites workspace-wide**; `apply_buoyancy_forces` re-adds every tick — verify semantics before any fluids coupling repeats the pattern (prefer `apply_impulse` or add explicit reset).

**Path-dependent**: with GPU SPH (Path A) the *only* feasible fluid→physics coupling is analytic (particle state is GPU-resident, unreadable — §4); with voxel/facade (B/C), per-position queries are real.

### Seam 3 — Terrain (`astraweave-terrain`) — Risk: HIGH

**Current state**: terrain exposes exactly what a fluid sim needs — `ChunkManager::get_height_at_world_pos` (`chunk.rs:427-431`), `Heightmap::get_height/calculate_normal/data()/resolution()` (`heightmap.rs:74-170`), `VoxelGrid::get_voxel(world_pos)` + `Voxel::is_solid()` (`voxel_data.rs:357-362, 96-99`) — and fluids' `analyze_terrain_for_water(&[f32], w, h, &config)` (`terrain_integration.rs:361-366`, real D8 flow-accumulation analysis) is **directly signature-compatible with `Heightmap::data()/resolution()` — no adapter needed**. Nothing connects them. Terrain mutation is `set_voxel` + **dirty-chunk polling** (`voxel_data.rs:346-375`); no event channel exists; `process_destructible_hits` (physics) is a dead no-op — carve-notification infrastructure is greenfield. Terrain's rayon meshing (`meshing.rs:473-484`) is the subsystem-parallelism precedent.

**Why HIGH**: the dependency topology is booby-trapped. `terrain → gameplay` (the known anomaly) and `gameplay → physics` mean: **`physics → fluids` AND `fluids → terrain` together create the cycle `physics → fluids → terrain → gameplay → physics`.** Safe shapes: fluids stays a leaf (slices in, its current design) and glue lives in a higher-level crate. Any path that adds `fluids → terrain` as a Cargo dep forecloses physics-side fluids deps permanently.

**Path-dependent**: Path A barely touches this seam (static volumes); Path B's voxel backend and all of Path C live here (chunk stitching across `ChunkManager`, dirty-chunk re-simulation, SDF baking of terrain height into `FluidSystem.sdf_system`).

### Seam 4 — Renderer (`astraweave-render`) — Risk: MED

**Current state**: `RenderView` (`astraweave-camera/src/render_view.rs:51-108`) is confirmed the sole view entry point (`Renderer::update_view`, `renderer.rs:4024-4064`; legacy setters deleted per C.3.C). Against fluids' 304-byte `CameraUniform`: **4 of 6 fields map directly** (view_proj, inv_view_proj, view_inv, cam_pos — with a camera-relative-feature caveat where shader-facing position is zeroed, `renderer.rs:4042-4046`); **`light_dir` and `time` are not in RenderView** — light lives on the Renderer (`light_override` / sky time-of-day, `:4030-4035`, no public getter for the resolved value) and time is caller-supplied by convention (`update_water(view_proj, cam_pos, time)` precedent, `:4546`). Pass order: clusters → CSM shadows → sky → cloud shadows → main forward+ pass (opaque → models → impostors → **water drawn inline last**, `:5035-5038`) → post/tonemap (`:5062`). Scene depth is readable (`Depth32Float`, `RENDER_ATTACHMENT|TEXTURE_BINDING`, `depth.rs:9,22`); HDR color (`Rgba16Float`) likewise (`:1291`). `transparency.rs` and `oit.rs` exist as modules with **zero references in renderer.rs** — not wired. **No pass-injection mechanism exists**: the frame graph validates topology only; the canonical optional-pass pattern is the `WaterRenderer` precedent — an `Option<…>` field + **two hardcoded call sites** (`render()` :5036 and `draw_into` :5668; dual-site mirroring is a known §7.7-class divergence source). Fluids' `FluidRenderer` is format-agnostic (`new(.., surface_format)`) and would accept `hdr_format()` directly.

**Integration requires**: modify `astraweave-render` (no injection alternative) — `Option<FluidsPass>` field + both call sites, slotted between main pass and post (SSFR needs scene depth + writes transparency in HDR pre-tonemap); a scene-color copy for the refraction sample-write hazard (shade pass samples scene color while writing the target — the existing `hdr aux tex`, `:1355-1369`, is a candidate); plumb resolved light_dir + caller time; build `CameraUniform` from `RenderView`; strip `println!`s; decide direct `render → fluids` Cargo dep vs a thin glue crate. **Naming/duplication landmine**: `WaterRenderer` (Gerstner ocean, `water.rs`) already exists — per the no-second-implementation rule, SSFR must be positioned explicitly as particle-fluid rendering distinct from the ocean plane, with the relationship documented.

**Path-dependent**: Path C's primary visual is the voxel heightfield surface mesh (`gpu_volume::generate_surface_mesh` → ordinary mesh draw — much cheaper seam, may reuse water shading); SSFR is Path A/B's SPH-backend seam.

### Seam 5 — Editor (`tools/aw_editor`) — Risk: HIGH

**Current state**: zero fluids presence. Panel registration is the documented **11-surface** procedure (`PanelType` enum at `panel_type.rs:107-233` — 41 variants, no Fluids — plus title/icon/category/description/`all()`/has_scroll arms, `EditorTabViewer` field + initializer + render match, `dock_panels.rs:92` dispatch; the F.5-paint diagnostic is the canonical partial-registration failure). Undo is per-panel `*Action` enums + `EditorCommand` trait (`command.rs:71`) + bounded `UndoStack` (`:236`), with the SP5 drain pattern (`main.rs:3996-4036`).

**The pre-built trap, confirmed**: `astraweave-fluids/src/editor.rs` ships `ConfigHistory` (`editor.rs:187-283`) — **a complete rival snapshot undo/redo system** (VecDeque past/future, max 50) over `FluidEditorConfig`, plus presets, validators, widget metadata. A naive FluidsPanel adopting it creates a second undo authority invisible to Ctrl+Z — a textbook §7.7 instance, pre-built and waiting. Resolution requirement: consume `fluids::editor` types/presets/validation (delegate, don't redeclare) but route **all mutations through `EditorCommand`**; demote `ConfigHistory` to at most a command payload.

**Viewport**: single unified path — the seam-4 `draw_into` hook automatically serves the editor viewport. Placement caveat: post-Parity-P.3 the editor target is LDR `Rgba8UnormSrgb` with tonemap-into-target (`viewport/renderer.rs:28-34`), so a fluids pass appended *after* `draw_into` would blend post-tonemap (wrong); correct placement is inside `draw_into` pre-tonemap.

**Scene format**: editor-local RON serializer (`scene_serialization.rs`), `EntityData` is a closed hand-rolled struct (`:12-32`) — a fluid volume needs a `#[serde(default)]` schema extension + `from_world`/`to_world` wiring + version handling; no generic component registry exists.

**Path-dependent**: marginally — all paths need volume authoring; Path C additionally needs voxel-grid bounds/material authoring.

### Seam 6 — Persistence + Networking — Risk: LOW

**Current state**: persistence-ecs snapshots a hardcoded closed list of 10 component types (`SerializedEntity`, `astraweave-persistence-ecs/src/lib.rs:186-198`) via a documented 5-site manual registration procedure (`:262-268`); `auto_save_system` confirmed still a comment-only TODO (`:72-75`); `replay_system` confirmed a dormant tick-counter stub (`:96`). aw-save: `ASVS` format, LZ4 postcard, u32 length → 4 GiB ceiling, one opaque-blob precedent (`WorldState.ecs_blob`). net-ecs replication is **itself a stub** (broadcasts empty `entity_states: HashMap::new()`, `astraweave-net-ecs/src/lib.rs:172-175`; `EntityState` hardcoded `{position, health}`) — **there is no per-component replication registry, so fluids "opting out" is the default and free**. Particle-replication math confirmed: 100k × 80 B = **8.0 MB/tick**, ~240 MB/s/client at 30 Hz — infeasible by ~4 orders of magnitude. `FluidSnapshot` (`serialization.rs:9-40`) serializes positions/velocities/colors + params via bincode — structurally pluggable as an opaque blob, but **no code constructs one from a `FluidSystem` or applies one back**, and GPU particle state has no readback path (§4.4), so today it could only snapshot the stale CPU cache.

**Integration requires** (authoring data only): a serializable `CFluidVolume` component + the 5-site persistence-ecs procedure; a documented policy line that particle state is excluded from `world_hash`/replay/replication. Nothing required on net.

**Standing red line**: any future attempt to replicate or hash live particle state is infeasible and determinism-breaking by construction; reject at review.

---

## WS3 — Capability Gap Analysis

For each target: what exists, what's missing, and whether the gap is **integration work** (I) or **simulation-model work** (S).

### T1 — Bounded gameplay water (volumes + buoyancy + drag + swimming)

**Exists**: working flat-plane Archimedes buoyancy + drag in physics (`apply_buoyancy_forces`); demo-proven physics→fluid one-way coupling (rigid bodies as `DynamicObject` colliders, `laboratory.rs:96-118`); `FluidAABB`/`FluidScenePlacement` authoring types (dormant); analytic volume queries in the unwired `EnvironmentManager::WaterVolume`.
**Missing**: per-volume (non-global-plane) buoyancy [I]; swim `CharState` + controller branch [I]; reconciliation of physics' three water abstractions [I]; volume authoring/persistence [I]; any fluid *surface* for the volume to render [I, seam 4].
**Honest note**: T1 does not require SPH at all. Analytic volumes + the existing buoyancy give the gameplay; SPH adds splash/visual dynamism. **Gap class: ~90% integration.**

### T2 — World water (lakes, rivers with flow, ocean at distance)

**Exists**: `analyze_terrain_for_water` (real D8 river/lake/waterfall detection, zero callers); the engine's separate `WaterRenderer` (Gerstner ocean plane) already wired in astraweave-render; voxel grid for bounded bodies.
**Missing**: the detection→placement→render wire [I]; flow fields for rivers (static flow-field bake from the D8 analysis is plausible [I]; dynamic flow is [S]); LOD story for distant water [I/S].
**Honest note**: SPH is **not** economical at world scale, confirmed by the crate's own roadmap (the "Hybrid: PBD+heightfield" tier). T2 is mostly wiring existing pieces (detector + Gerstner renderer + voxel/analytic bodies). **Gap class: ~70% integration, 30% sim-model (flow).**

### T3 — Dynamic voxel water (Enshrouded-class, terrain-coupled, player-redirectable)

**Exists — more than the campaign brief assumed**: `WaterVolumeGrid` is a real, deterministic, materially-aware CA with hydrostatic-pressure-assisted flow and U-bend capability, explicitly Enshrouded-inspired (`volume_grid.rs:5`); building-side dispensers/drains/wheels are real; GPU upload + CPU surface meshing exist (`gpu_volume.rs`). The brief's hypothesis "almost certainly a different sim model" is half-right: it IS a different model from SPH — **but that model already exists in this crate**.
**Missing**: scale — the sim iterates **all** cells densely; the active-cell sparsity is unimplemented [S]; terrain voxel coupling (solidity from `VoxelGrid`, material mapping) [I]; carve reactivity (dirty-chunk polling loop; the notification channel is greenfield) [I+S]; the gate mechanism (dead flag, Must-Fix #6) [S-small]; conservation tests [S-small]; chunked/streamed world residency [S]; performance ground truth (zero benches exist for `simulate`) [measurement].
**Verdict**: the existing crate contributes a genuine T3 *foundation*, not just splash effects — but T3 at world scale is the campaign's largest sim-model work regardless. **Gap class: ~40% integration, 60% sim-model.**

### T4 — Research-grade local simulation

**Exists**: working PBF (with Must-Fix #1-#3); real PCISPH pipeline with warm-start and δ-SPH shifting (unverified convergence, zero callers); dormant CPU helpers for DFSPH/IISPH; a purpose-built but runner-less validation framework; the 18-22-week roadmap doc (`FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`: current grade B, target A+).
**Missing**: everything that makes "research-grade" a checkable claim — a validation runner executing `BenchmarkConfig::hydrostatic()/dam_break()` against an actual solver [S]; real convergence readback in PCISPH [S]; honest δ computation (the `-0.5` fudge) [S]; DFSPH/IISPH solver loops (currently aspirational) [S-large]; any GPU-execution test at all [S].
**Gap class: ~95% simulation-model work.** T4 is orthogonal to engine integration and can proceed independently of (or after) any path.

---

## WS4 — Determinism Analysis

Direct answers, no hedging.

**Q1 — Is the current sim deterministic single-threaded? Under `parallel`?**

- **`WaterVolumeGrid` (CPU voxel): YES, deterministic by construction.** Fixed `y/x/z` iteration order, fixed direction array, linear Vec storage, no HashMap iteration, no RNG, no threads (`volume_grid.rs:443-570`). The in-place vertical pass is order-dependent but the order is fixed — bit-reproducible given identical inputs and dt sequence.
- **`FluidSystem` / `PcisphSystem` (GPU): NO, non-deterministic by construction, even "single-threaded" on the host.** `build_grid` inserts particles into per-cell linked lists via `atomicExchange` (`fluid.wgsl:159-160`; identically `pcisph.wgsl:274-275`); list order is the GPU thread-completion order of the atomic race, varying run-to-run and across hardware. Every summation kernel walks those lists; f32 addition is non-associative; PBF/PCISPH iteration amplifies ULP-scale differences into trajectory divergence.
- **`FluidSystem` is additionally host-timing-dependent**: density-error readback is non-blocking with `map_async` issued before submit (`lib.rs:1183-1219`); its result sets `self.iterations` (`:1207`) — **a physics-affecting parameter is a function of async GPU timing**. `step_with_budget` further blends wall-clock frame time into iteration count (`:1298-1327`).
- **Effects RNG**: all four hand-rolled LCGs are constant-seeded (foam 12345, waterfall 54321/98765, underwater 13579) — nominally deterministic, but state advances per call, is not externally seedable, and is not captured by any snapshot: replay-from-snapshot diverges in effect particles.
- **Under `parallel`: deterministic but moot** — all gated functions are element-wise with sequential inner sums (no parallel float reductions exist), and nothing calls them (§1.5).

**Q2 — What can honestly be guaranteed inside the 60 Hz tick?**

Three honest postures, evaluated:

1. **Deterministic-by-construction GPU SPH**: would require replacing atomic linked lists with sorted neighbor lists (Morton/bitonic sort — scaffolding exists, unintegrated) + fixed-order summation + fixed iteration counts + removing the timing-coupled adaptive loop. Substantial solver rework with a real performance cost (sorting per step); **not achievable by integration work alone**. Not recommended for V1.
2. **Deterministic-only-in-singlethread mode**: not meaningful here — the non-determinism is on the GPU, not in host threading. A "CPU reference solver" mode does not exist (simd_ops is primitives, not a solver).
3. **Non-deterministic-but-visually-stable particles, excluded from determinism guarantees** + **deterministic CPU gameplay-water layer** (analytic volumes and/or voxel grid): **the only honest V1 posture.** Gameplay-relevant water state (submersion, buoyancy forces, flow vectors, water levels) computed on the CPU layer is bit-reproducible and AI-queryable; particle state is declared presentation-only.

**This must be decided at the gate, publicly documented in `docs/architecture/fluids.md` + `net.md`, and enforced at review** (no particle state in WorldSnapshot, world_hash, replay events, or replication).

**Q3 — What do persistence/net snapshot architectures assume that fluids would violate?**

- `astraweave-net`'s `world_hash` (DefaultHasher over stable-ordered entities) and replay (`replay_from` re-executes the event log and compares hashes) assume **same-state-given-same-inputs re-execution**. GPU particle state folded into either would diverge on every replay and across every client. Violation is avoided only by exclusion.
- `persistence-ecs` assumes a closed component list with serde-able state; fine for `CFluidVolume` authoring data; the `replay_system` that would replay events is itself a dormant stub, so fluids does not *currently* break anything — the constraint is forward-looking.
- net-ecs replication: 8 MB/tick for 100k particles (math in §Seam 6) — physically infeasible; replicate authoring params + seeds only, accept cross-client visual divergence of particles.
- `FluidSnapshot` cannot capture GPU truth today (no particle readback path exists; only the two 4-byte error staging buffers are `MAP_READ`); it is dormant scaffolding either way (no capture/restore integration exists).

---

## WS5 — Performance Budget

**Assumed budget (stated assumption, owner to ratify)**: fluids gets **2 ms GPU + 1 ms CPU of the 16.6 ms frame** at 60 Hz. Justification: rendering owns the largest GPU share; physics+AI own the CPU SIMULATION/AI budget; 2 ms GPU matches what comparable engines allocate to hero fluids; 1 ms CPU bounds a voxel layer to ~6% of frame.

**Ground truth available: almost none.** No fluids numbers have ever been recorded in `MASTER_BENCHMARK_REPORT.md` (zero matches). The crate's bench file predominantly measures its own mock re-implementations (§1.4-D). **No production GPU solver step has ever been benchmarked.** The roadmap's tier table (50-100k @ 60 fps PBD … 500k-1M hybrid) is aspirational targets, not measurements.

**Fresh numbers from this audit** (the only bench groups exercising production code; `cargo bench --features parallel --bench fluids_adversarial -- "parallel_operations|optimized_functions" --quick`, criterion quick mode, machine per Preamble — CPU `simd_ops` micro-functions, **none called by any production path**, recorded as capability evidence only):

```
parallel_operations/sequential_position_update_100k   543.11 µs
parallel_operations/parallel_position_update_100k     397.19 µs   (1.37× on 4C/8T)
parallel_operations/sequential_kernel_eval_10k         19.26 µs
parallel_operations/parallel_kernel_eval_10k           41.18 µs   (parallel 2.1× SLOWER — rayon overhead below ~5 ms workloads, confirming CLAUDE.md lesson #2)
parallel_operations/sequential_morton_codes_100k        1.356 ms
parallel_operations/parallel_morton_codes_100k        490.29 µs   (2.77×)
optimized_functions/weighted_centroid_fast_1000         1.09 µs
optimized_functions/accumulate_density_simple_10k       9.65 µs
optimized_functions/accumulate_density_simple_100k    100.21 µs
optimized_functions/accumulate_density_4x_100k         97.46 µs
```

**Dominant cost centers (labeled inference, not profiling)** for the PBF GPU path, per frame: (1) **full SDF Jump-Flood regeneration every frame** (`lib.rs:1041` — 3 compute pipelines over the SDF volume regardless of whether boundaries changed; an obvious caching candidate); (2) the iteration loop — 2–8 × (lambda + delta_pos), each a full-particle dispatch with 27-cell neighbor walks; (3) grid clear+build over 128³ = 2.1M cells. PCISPH additionally rebuilds the grid *per pressure iteration* (`pcisph_system.rs:1009-1031`) — structurally more expensive per iteration.

**Particle-count ceiling: UNKNOWN.** Defensible working envelope only: the demo ships 20k particles by default (`main.rs:286-287`) and is interactively usable on this machine class (GTX 1660 Ti Max-Q); the roadmap claims 50-100k @ 60 fps for PBD-tier on unstated hardware. Within a 2 ms slice on mid-range hardware, treat **20k-50k as the plausible envelope until measured**. Voxel grid CPU cost: zero benches exist; a dense 64³ grid is 262,144 cells/tick of simple arithmetic — plausibly ~1 ms-class on this CPU, **UNKNOWN until measured**; world-scale grids require the unimplemented sparsity.

**Mandatory F.1 instrumentation** (any path): wgpu timestamp queries around each compute pass of `FluidSystem::step`; a criterion bench for `WaterVolumeGrid::simulate` at 32³/64³/128³; record both in `MASTER_BENCHMARK_REPORT.md` as the first-ever fluids baselines. GPU-compute migration of the voxel sim is a Path C cost consideration, not a recommendation.

---

## WS6 — Path Options

Common to all paths: Must-Fix items #4, #7, #9, #10 (hygiene) land in F.1 regardless; the determinism carve-out (Owner Question 1) and trace corrections are universal prerequisites.

### Path A — Couple what exists (SPH-visual + analytic gameplay)

**Scope**: repair `FluidSystem`, wire it as the engine's particle-water visual layer inside authored AABB volumes; gameplay forces come from the *analytic* volume (per-volume buoyancy/drag/swim), not from particles — because GPU particle state cannot be queried (§4).

**Sub-phases**:
- **F.1** FluidSystem correctness repair: ping-pong (#1), particle_flags (#2), readback ordering (#3), first-ever GPU-execution smoke + invariant tests, timestamp instrumentation. (M)
- **F.2** Volume gameplay coupling: `CFluidVolume` component; reconcile physics' three water abstractions into per-volume queries; `CharState::Swimming`; ECS resource + host-driven GPU step precedent. (M)
- **F.3** Render hook: SSFR pass in `render()`/`draw_into`, scene-color copy, light_dir/time plumbing, `CameraUniform`-from-`RenderView`. (M)
- **F.4** Editor: FluidsPanel (11 surfaces), `EditorCommand` volume edits, scene-schema extension. (L)
- **F.5** Persistence of authoring data + determinism policy docs + trace updates. (S)

**Unlocks**: T1 (gameplay via analytic volume, visuals via SPH), T4 seed (a *correct* PBF). **Defers**: T2, T3 entirely. **Determinism**: gameplay layer deterministic (analytic); particles carved out. **Relative effort: 1.0× (baseline).**

**Top risks**: (1) editor 11-surface registration class — mitigate with the E0004 compile-discovery method; (2) render dual call-site divergence — mitigate with a shared helper both sites call; (3) **strategic**: if T3 is later committed, the volume/coupling layer built here gets reworked into a facade anyway — A's savings are partially borrowed from B.

### Path B — Layered facade (deterministic water core, pluggable backends)

**Scope**: define the engine's single logical owner of "water state" now — a facade exposing (a) `FluidVolume` authoring and (b) a CPU-deterministic `WaterQuery` API (`is_submerged(pos)`, `flow_at(pos)`, `surface_height_at(pos)`, `density_at(pos)`) — with analytic volumes as backend #1, the voxel grid as backend #2, and SPH as a *visual* backend. Physics/AI/gameplay consume only the facade. Glue lives above physics+terrain+fluids (new thin crate or gameplay-level module) to respect the cycle constraint (Seam 3).

**Sub-phases**:
- **F.1** = Path A's F.1 (shared correctness repair + instrumentation). (M)
- **F.2** Facade + analytic backend + physics consumer **in the same sub-phase** (Integration Completeness: facade ships with ≥1 production caller); retire `add_water_aabb` stub, scalar `water_level`, and `EnvironmentManager::WaterVolume` into it; `CharState::Swimming`. (M/L)
- **F.3** Voxel backend: `WaterVolumeGrid` behind the facade; conservation + hydrostatic + U-bend tests; gate-flag fix (#6); terrain heightmap glue (`Heightmap::data()` → `analyze_terrain_for_water`); dirty-chunk carve reactivity (polling). (L)
- **F.4** Render: SSFR pass (as A-F.3) for particle volumes + heightfield surface mesh draw for voxel volumes. (L)
- **F.5** Editor (as A-F.4, plus voxel-volume authoring). (L)
- **F.6** Persistence + determinism policy + trace/docs closeout. (S)

**Unlocks**: T1 fully (deterministic, AI-queryable), T3 foundation (a *real* backend, not a hypothetical slot), T2 partial (detection wire + Gerstner + flow-field bake candidate). **Defers**: T4, world-scale T3 (sparsity/streaming). **Determinism**: gameplay-truth layer deterministic by construction; visual layers carved out. **Relative effort: ~1.6×.**

**Top risks**: (1) facade becomes another dormant abstraction if the campaign stalls — mitigated by F.2's ships-with-consumer rule; (2) voxel-grid performance at scale unproven (zero benches) — mitigated by F.1 instrumentation gating F.3 scope; (3) facade API designed before T3's real requirements are known — mitigated by keeping it minimal (the four queries) and versioned.

### Path C — Grid-first pivot (voxel water as THE integration)

**Scope**: treat T3 as the target; promote `WaterVolumeGrid` to the primary system — sparse/chunked, terrain-coupled, carve-reactive, gameplay-queryable; surface rendered from the heightfield mesh; SPH relegated to later detail effects (its repairs deferred or opportunistic).

**Sub-phases**:
- **F.1** Voxel sim hardening: conservation/hydrostatic/U-bend tests, gate fix (#6), dt stability bound, **active-cell sparsity implementation**, first benches. (L)
- **F.2** Terrain coupling: `VoxelGrid` solidity → cell materials, dirty-chunk carve re-simulation, chunked world residency. (L)
- **F.3** Physics/gameplay queries: buoyancy/swim from cells (facade-lite). (M)
- **F.4** Voxel surface rendering into the main pass (mesh draw; possibly reuse water shading). (M/L)
- **F.5** Editor. (L)
- **F.6** SPH detail layer (optional; requires Path A's F.1 repairs). (deferred)

**Unlocks**: T3 directly, T1 (via cells), T2 partial. **Defers**: T4 entirely, SSFR, most of the crate's 84K LoC particle inventory. **Determinism**: fully deterministic core — best alignment with engine identity. **Relative effort: ~2.0-2.2×, dominated by sim-model scaling work with unproven performance.**

**Top risks**: (1) dense-CA → world-scale is genuinely hard engineering (sparsity, chunk boundaries, streaming) with no existing perf data — the largest unknown in any path; (2) discards near-term value of the particle inventory and the demo-proven path; (3) carve-reactivity needs notification infrastructure that doesn't exist (greenfield).

### Comparison

| | **A — Couple what exists** | **B — Layered facade** | **C — Grid-first pivot** |
|---|---|---|---|
| T1 bounded gameplay water | ✅ (analytic forces + SPH visuals) | ✅✅ (deterministic, queryable) | ✅ (via cells) |
| T2 world water | ❌ | ◐ (detection wire + bake) | ◐ |
| T3 Enshrouded-class | ❌ | ◐ foundation (real backend) | ✅ direct target |
| T4 research-grade | ◐ seed (correct PBF) | ◐ seed | ❌ |
| Determinism posture | analytic-deterministic + carve-out | deterministic core + carve-out | fully deterministic core |
| Sim-model vs integration work | ~20/80 | ~35/65 | ~65/35 |
| Rework risk if T3 later committed | HIGH (volume layer → facade rebuild) | LOW (designed for it) | n/a (is it) |
| §7.7 hazard handling | partial (physics trio reconciled) | **resolves** (single logical owner) | partial |
| Hardest dependency | render core-mod | facade design quality | sparsity/scale engineering |
| Relative effort | 1.0× | ~1.6× | ~2.0-2.2× |

### Agent's recommendation (NOT a decision)

**Path B.** Four evidence-driven reasons:

1. **Engine identity**: AstraWeave's AI-first, deterministic-ECS architecture requires gameplay water to be CPU-queryable and reproducible (WorldSnapshot, GOAP preconditions, replay, net). GPU SPH state is structurally incapable of serving that role (§4) — in *any* path it can only ever be a visual layer. B is the only option that makes this architectural truth explicit in the API instead of leaving it implicit in Path A's volume hack.
2. **§7.7 resolution**: physics already carries three parallel water abstractions; the fluids crate carries five solver surfaces and a rival undo system. A facade is the one move that *reduces* the count of parallel implementations rather than adding a sixth — directly serving the CLAUDE.md no-second-implementation mandate.
3. **T3 is the stated ambition, and its foundation already exists in-tree**: `WaterVolumeGrid` is real, deterministic, and Enshrouded-shaped. B keeps it on the critical path at controlled cost; A strands it; C bets the campaign on scaling it before its performance is even measured.
4. **Cost honesty**: B's premium over A (~0.6×) buys exactly the layer A would have to retrofit the moment T2/T3 are committed. C's premium (~1.0-1.2× over B) buys world-scale T3 *now*, which the perf evidence (none exists) cannot yet justify.

If the owner declares T3 out of scope for the foreseeable future, **Path A** becomes the right answer; if the owner declares T3 the singular priority and accepts deferring all particle work, **Path C** — but only after F.1-style voxel benches exist.

---

## Must-Fix List (defects that gate any path)

| # | Defect | Evidence | Gates |
|---|---|---|---|
| 1 | **FluidSystem ping-pong divergence**: buf1 created empty; bind groups swap per frame; all kernels write binding 0 in place (`particles_dst` "Reserved"); two interleaved half-rate particle states; renderer handed the stale buffer; `get_particle_buffer` comment describes unimplemented behavior | `lib.rs:395-412, 700-729, 1044-1048, 1222-1231`; `fluid.wgsl:56-57` | any path shipping `FluidSystem` |
| 2 | **Despawn GPU-invisible**: `particle_flags` host-written, never bound, no WGSL declaration; dispatches cover `particle_count` not `active_count` | `lib.rs:311, 659-663, 854, 906, 989`; zero `flags` in `fluid.wgsl` | same |
| 3 | **`map_async` before submit + timing-coupled physics**: readback feeds `self.iterations`; physics-affecting state depends on async GPU timing | `lib.rs:1183-1219, 1207` | same; also WS4 posture |
| 4 | **`UnifiedSolver::step` no-op** sold as the flagship re-export: delete, gate behind an `experimental` feature, or implement — before any integration names a solver surface | `unified_solver.rs:527-542` | all paths (anti-hype mandate) |
| 5 | **PCISPH dead convergence machinery** + empirical δ fudge + unverifiable "<0.1%" claim | `pcisph_system.rs:14, 899-904, 967-969`; dead `IterationState` | T4 work; honesty of docs |
| 6 | **`WaterGate`/`CellFlags` dead**: GATE/FROZEN/EDITING/PERSISTENT never read by the sim; gates don't gate | `building.rs:338-347`; `volume_grid.rs:86-96, 443-596` | Paths B (F.3) and C |
| 7 | **`serde` feature decorative**: `--no-default-features` cannot compile; 5 unconditional `use serde::` | `Cargo.toml:6-9` | all paths (hygiene) |
| 8 | **Validation framework lies**: `load_csv` fakes success; divergence error hardcoded 0.0 | `validation.rs:367-370, 81-84` | T4; any "research-grade" claim |
| 9 | **Zero GPU-execution tests + zero physical-invariant tests + bench-of-mocks + no recorded baselines**: `FluidSystem::new` never called in tests; first F.1 task in every path | §1.4 | all paths |
| 10 | **Doc/trace corrections**: fluids.md D1-D8 (§1.6), orphan `viscosity_morris.wgsl`, phantom `viscosity_implicit.wgsl`, CLAUDE.md `character_controller.rs` path, render-trace phantom fluids row, stale mutation report, demo orphan scenario files + dead UI, renderer `println!`s | §1.3, §1.6 | all paths (doc contract) |

---

## Open Questions for Owner Gate

Decisions only the owner can make, asked precisely:

1. **Determinism carve-out**: do you accept a publicly documented policy that *particle* fluid state is non-deterministic, presentation-only, and permanently excluded from WorldSnapshot, world_hash, replay, and replication — with gameplay water truth living on a deterministic CPU layer? (Required for any path that renders SPH. If NO: only Path C's voxel-only core, or a major deterministic-GPU rework, is honest.)
2. **Is T3 (Enshrouded-class voxel water) a committed campaign target or an aspiration?** This is the A-vs-B fork: committed-or-likely → B; explicitly out of scope → A.
3. **Solver consolidation authority**: may the campaign delete or `experimental`-gate the dishonest/dormant surfaces (`UnifiedSolver` no-op step, `research.rs` phantom-system docs, dormant DFSPH/IISPH variants), per the no-second-implementation rule? Or must all five surfaces be preserved as-is?
4. **Glue placement**: new thin integration crate (e.g. `astraweave-water`) vs gameplay-level module? (Constraint: `physics → fluids` + `fluids → terrain` is a dependency cycle; fluids must stay a leaf — §Seam 3.)
5. **Render dependency shape**: direct `astraweave-render → astraweave-fluids` Cargo dep, or a thin pass-crate to keep the heavy fluids dep out of the renderer?
6. **Editor V1 scope**: volume placement + basic params only, or the full `FluidEditorConfig` surface (40+ types) in the first panel?
7. **Frame budget ratification**: is 2 ms GPU + 1 ms CPU at 60 Hz the fluids allocation (WS5 assumption)?
8. **Physics water reconciliation authority**: may the campaign retire `add_water_aabb`/`clear_water` stubs, the scalar `water_level` plane, and `EnvironmentManager::WaterVolume` into the chosen path's query surface (touching `astraweave-physics` public API)?

---

## Appendix — Raw command outputs

**A.1 Commit + history**: see Preamble (verbatim).

**A.2 `cargo check`** (both feature sets): see Preamble (verbatim; both PASS, zero warnings).

**A.3 Test suite** (`cargo test -p astraweave-fluids`, 2026-06-10):

```
running 2480 tests
test result: ok. 2480 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.64s
running 99 tests
test result: ok. 99 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
running 6 tests
test result: ok. 0 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo test -- --list` count: 2,585 listed test names (2,480 + 99 + 6).

**A.4 Bench run** (`cargo bench -p astraweave-fluids --features parallel --bench fluids_adversarial -- "parallel_operations|optimized_functions" --quick`, criterion `--quick`, bench profile compile 2m09s):

```
parallel_operations/sequential_position_update_100k   [538.02 µs 543.11 µs 563.47 µs]
parallel_operations/parallel_position_update_100k     [396.19 µs 397.19 µs 401.18 µs]
parallel_operations/sequential_kernel_eval_10k        [19.245 µs 19.264 µs 19.340 µs]
parallel_operations/parallel_kernel_eval_10k          [40.925 µs 41.184 µs 41.249 µs]
parallel_operations/sequential_morton_codes_100k      [1.3518 ms 1.3564 ms 1.3747 ms]
parallel_operations/parallel_morton_codes_100k        [486.75 µs 490.29 µs 491.17 µs]
optimized_functions/weighted_centroid_fast_1000       [1.0626 µs 1.0945 µs 1.1025 µs]
optimized_functions/accumulate_density_simple_10k     [9.5886 µs 9.6543 µs 9.9172 µs]
optimized_functions/accumulate_density_4x_10k         [9.8755 µs 9.9057 µs 10.026 µs]
optimized_functions/accumulate_density_simple_100k    [96.109 µs 100.21 µs 101.24 µs]
optimized_functions/accumulate_density_4x_100k        [95.876 µs 97.461 µs 97.857 µs]
```

Note: a first bench invocation without `--bench fluids_adversarial` failed with `error: Unrecognized option: 'quick'` because cargo routed criterion args to the lib's libtest harness — recorded for reproducibility.

**A.5 Workstream completeness**: WS1 (all six sub-sections) complete; WS2 (all six seams) complete; WS3 complete; WS4 complete (all three questions answered); WS5 complete with explicit UNKNOWNs (production GPU solver step and voxel-sim tick have never been measured — flagged as mandatory F.1 instrumentation); WS6 complete (3 paths + table + labeled recommendation). No source file outside this report was created, modified, or deleted.

---

**Revision history**

| Version | Date | Change |
|---|---|---|
| 1.0 | 2026-06-11 | Initial F.0 ground-truth audit at commit `8e1505dd8` |
