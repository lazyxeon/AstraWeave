# Architecture Trace: Fluids

## Metadata

| Field | Value |
|---|---|
| **System name** | Fluids (GPU SPH/PBD particle simulation + voxel water + visual effects + terrain integration + building integration + editor) |
| **Primary crates** | `astraweave-fluids` (35 source files / 8 WGSL shaders — 7 in `shaders/` + 1 in `shaders/research/pcisph.wgsl` / ~84.5K LoC total) |
| **Document version** | 1.3 |
| **Last verified against commit** | branch `campaign/fluids-f1` (F.1 execution, base `8e1505dd8`; see `docs/campaigns/fluids-integration/F1_EXECUTION_REPORT.md`) |
| **Last verified date** | 2026-06-11 |
| **Status** | **Dormant for the runtime engine; large parallel-solver inventory; example-only consumer.** Verified 2026-05-12: workspace grep for `use astraweave_fluids` outside `astraweave-fluids/` itself returned exactly one production consumer — `examples/fluids_demo/src/main.rs:18-21` (which imports `FluidSystem`, `FluidRenderer`, `FluidLodConfig`, `FluidLodManager`, `FluidOptimizationController`, `renderer::CameraUniform`). No game-loop crate (`astraweave-render`, `astraweave-gameplay`, `astraweave-physics`, `astraweave-scene`, `astraweave-terrain`, `astraweave-ecs`) depends on `astraweave-fluids`. The crate contains **five major parallel solver/manager surfaces** (`FluidSystem` in `lib.rs`, `UnifiedSolver` in `unified_solver.rs`, `ResearchFluidSystem` in `research.rs`, `PCISPHSystem` in `pcisph_system.rs`, `WaterEffectsManager` in `water_effects.rs`) that coexist with overlapping responsibilities. |
| **Owner notes** | Scale: 35 Rust source files, 8 WGSL compute shaders (7 in `shaders/` + 1 in `shaders/research/pcisph.wgsl`, 27.8 KB), 1 integration test file (`mutation_resistant_comprehensive_tests.rs`, 785 LoC), 1 benchmark (`fluids_adversarial`, 1,893 LoC). Largest single file is `simd_ops.rs` at 39,554 LoC (largely batch-operation surface for SIMD-friendly SPH primitives). Second largest is `editor.rs` at 5,823 LoC. The README + the audit doc at `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md` (v2.0, Jan 2026) document an explicit "research-grade enhancement" roadmap target of multi-solver SPH (PBD/PCISPH/DFSPH/IISPH). **Verification pass 2026-05-12 (version 1.1):** resolved 9 markers + 2 factual corrections — (a) zero unsafe blocks crate-wide (only 2 bytemuck unsafe-trait impls at `debug_viz.rs:479-480`); (b) `ResearchQualityTier` is 5-variant Low/Medium/High/Ultra/Research at `research.rs:198-213`; (c) `PhysicsConfig` (editor) has 9 fields at `editor.rs:2094-2113`; (d) `tools/aw_editor` does NOT consume `astraweave-fluids` (editor surface is forward-design only); (e) `CameraUniform` is **304 bytes** not 200 (corrected Invariant 6); (f) `FluidSystem.particle_buffers` confirmed 2-entry ping-pong at `lib.rs:414`; (g) `FluidOptimizationController` lives in `lib.rs:1433` NOT `optimization.rs` (corrected §5); (h) 8th WGSL shader discovered: `shaders/research/pcisph.wgsl`; (i) inline `#[test]` counts per file documented (140 in editor.rs, 79 in lib.rs, 78 in optimization.rs, etc., 600+ total inline tests). **Deep investigation pass 2026-05-12 (version 1.2):** closed 2 factual §11 Open Questions — (a) `ssfr_smooth.wgsl` v1 deletion confirmed via `git log --diff-filter=D`: deleted in commit `4af95b47c` "Implement rain splash particle system, shader permutation system, snow footprint stamping, and vegetation interaction system" (resolution moved to §5 file map + new §7 Decision Log entry); (b) Editor surface NOT wired into `tools/aw_editor` (workspace grep + Cargo.toml dep check both zero) — resolution captured in §5 file map editor.rs row. Resolved the new pcisph.wgsl include-path marker: `pcisph_system.rs:549` consumes it. Comprehensive shader-consumption audit confirmed all 8 WGSL shaders are consumed by Rust `include_str!` calls (`anisotropic.rs:80`, `lib.rs:366` for fluid.wgsl, `pcisph_system.rs:549`, `renderer.rs:61/65/69/370-371`, `sdf.rs:53`). Recovered Decision Log entry for SSFR shader refactor (commit `4af95b47c` "shader permutation system"). |

---

## 0. F.1 Revision Notice (2026-06-11) — READ FIRST

The Fluids-Integration campaign's F.0 audit (`docs/campaigns/fluids-integration/F0_GROUND_TRUTH_AUDIT.md`) falsified several claims in v1.2 of this trace, and the F.1 execution phase then changed the crate. Corrections and deltas:

**Trace errors corrected (the source had not changed since v1.2 — these were errors in the trace itself):**
- **`ResearchFluidSystem` never existed.** v1.2 inventoried it as an active research-grade GPU pipeline in `research.rs` (§3, §5, §6). In reality `research.rs` is a wgpu-free types/config module; the name appeared only in an `ignore`d doc example. All such references below should be read through this correction.
- **The shader count was 9, not 8**: `src/shaders/viscosity_morris.wgsl` (644 LoC) existed with no `include_str!` consumer (deleted in F.1). `viscosity.rs` also referenced a `viscosity_implicit.wgsl` that never existed (reference corrected).
- **Invariants 21–23 ("2-entry ping-pong") described a defect, not a design**: buffer 1 was created empty and never written by any kernel; the alternating bind groups simulated two divergent half-rate particle states (F.0 Must-Fix #1).

**F.1 code changes (this trace's v1.2 inventory no longer matches where marked):**
- `FluidSystem` repaired: single particle buffer (ping-pong deleted); `particle_flags` now bound and honored by every kernel (despawn is real, despawned particles parked at y=−10000); density-error readback is a race-free two-frame-lag state machine (`map_async` only after submit); `step()` documents a submit-before-next-step contract; per-pass GPU timestamp instrumentation behind `enable_gpu_timing`/`read_gpu_timings`; the dead `SimParams.pressure_multiplier` uniform removed.
- **Five blocking SDF defects fixed** (the F.1 GPU tests proved `FluidSystem::step` had *never* successfully executed): WGSL `JfaParams` 32-vs-16-byte mismatch; missing bind groups at every dispatch; inverted JFA ping-pong (first step read the uninitialized texture, destroying the seed); init pass voxelizing all 128 zeroed object-buffer entries (seeding every voxel as "inside an object"); z-dispatch covering only half the SDF volume.
- **`unified_solver.rs` deleted** (its `step()` was a no-op frame counter; the whole config surface was execution-dead). Root re-export removed.
- **`SolverType::DFSPH`/`IISPH` variants deleted** (no solver loop existed); quality tiers High/Ultra/Research now select PCISPH.
- **New `experimental` feature** gates the dormant-real inventory: `pcisph_system`, `multi_phase`, `warm_start`, `particle_shifting`, `turbulence`, `viscosity_gpu`.
- **`serde` feature removed** (it gated nothing; serde is now unconditional). Features are now `parallel` and `experimental`, both default-off.
- `validation.rs` honesty: `load_csv` actually parses CSV (was fake-success); divergence metrics are NaN-not-computed (were silently 0.0 = "perfect").
- First GPU-execution + physical-invariant tests (`tests/gpu_execution_tests.rs`) and first production-code benches (`benches/fluid_baselines.rs`).

**Determinism carve-out (campaign gate Q1, policy — binding for all future work):**
GPU particle fluid state is **non-deterministic by construction** (atomic neighbor-list insertion order × float non-associativity; `FluidSystem` additionally couples its adaptive iteration count to async GPU timing, with defined two-frame-lag semantics post-F.1). Therefore particle state is **presentation-only** and permanently excluded from `WorldSnapshot`, `world_hash`, replay event logs, and network replication. Gameplay-relevant water truth (submersion, buoyancy, flow, levels) must live on deterministic CPU layers (analytic volumes / `WaterVolumeGrid`, which is deterministic by construction). Any PR that hashes, replicates, or replays particle state must be rejected at review. The corresponding note lives in `docs/architecture/net.md` §1.

## 1. Executive Summary

**What this system does:**
Provides GPU-accelerated fluid simulation through multiple coexisting solvers (PBD, PCISPH, DFSPH, IISPH per `research.rs:42-68`), plus a voxel-based volumetric water grid for terrain/building interaction (`volume_grid.rs`), plus visual effects (caustics, god rays, foam, reflections, underwater fog, waterfall, anisotropic surface — all coordinated by `WaterEffectsManager` at `water_effects.rs:1-100`), plus terrain integration (river/lake/waterfall detection from heightmaps in `terrain_integration.rs`), plus building integration (water dispensers/drains/gates/wheels in `building.rs`), plus a comprehensive editor integration layer (`editor.rs`, 5,823 LoC).

**Why it exists:**
Per `astraweave-fluids/README.md:1`: "A production-grade GPU-accelerated fluid simulation system for the AstraWeave game engine with world-class performance optimization." Per `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md` v2.0 (Jan 2026): the system is the engine's intended path to "research-grade fluid simulation" — multi-fluid types (water/oil/honey), multi-phase interactions, non-Newtonian flows, turbulent simulations.

**Where it primarily lives:**
- **Core SPH/PBD simulator:** `astraweave-fluids/src/lib.rs` (3,810 LoC) — `FluidSystem` GPU pipeline with PBD compute pass (clear_grid / build_grid / predict / lambda / delta_pos / integrate / mix_dye), `Particle`/`SimParams`/`SecondaryParticle` GPU types
- **Research solvers:** `astraweave-fluids/src/{research,pcisph_system,unified_solver}.rs` — `ResearchFluidSystem`, `PCISPHSystem`, `UnifiedSolver` parallel implementations
- **Volumetric water grid:** `astraweave-fluids/src/{volume_grid,gpu_volume,building}.rs` — voxel-based water for terrain/building interaction (`WaterVolumeGrid`, `WaterCell`, `MaterialType`, `WaterBuildingManager`)
- **Visual effects:** `astraweave-fluids/src/{caustics,foam,god_rays,water_reflections,underwater,underwater_particles,waterfall,anisotropic}.rs` plus WGSL shaders in `shaders/`
- **Coordinator:** `astraweave-fluids/src/water_effects.rs` — `WaterEffectsManager` composes all effect systems behind `WaterQualityPreset` (Low/Medium/High/Ultra/Custom)
- **Optimization:** `astraweave-fluids/src/{optimization,lod,profiling,simd_ops}.rs` — workgroup tuning, LOD, profiling, batch SIMD primitives
- **Terrain hookup:** `astraweave-fluids/src/terrain_integration.rs` — `WaterBodyType` (River/Stream/Lake/Pond/Ocean/Waterfall/Aquifer), `analyze_terrain_for_water`, `DetectedWaterBody`
- **Editor support:** `astraweave-fluids/src/editor.rs` (5,823 LoC) — `FluidEditorConfig`, presets, undo/redo, validation, batch operations, clipboard, animation easing, color-blind palettes
- **SIMD primitives:** `astraweave-fluids/src/simd_ops.rs` (39,554 LoC) — Wendland C2/C4 + Cubic Spline SPH kernels, batch position/velocity/density operations, Morton-code spatial hashing
- **Renderer:** `astraweave-fluids/src/renderer.rs` — `FluidRenderer` with SSFR depth + smooth + shade + secondary-particle render pipelines
- **WGSL shaders:** `astraweave-fluids/shaders/{anisotropic,fluid,sdf_gen,secondary,ssfr_depth,ssfr_shade,ssfr_smooth_v2}.wgsl` (7 files, 1,132 LoC total) + `astraweave-fluids/shaders/research/pcisph.wgsl` (~27.8 KB) — 8 shader files total

**Status note (read first):**
1. **The crate is dormant in production game-loop integration.** Workspace grep verified 2026-05-12: `examples/fluids_demo` is the ONLY consumer outside the crate itself. No production game-loop crate depends on `astraweave-fluids`.
2. **Five parallel solver/manager surfaces coexist** with overlapping responsibilities — see §6 Conflict Map. The original `FluidSystem` (lib.rs PBD pipeline) predates the `ResearchFluidSystem` + `PCISPHSystem` + `UnifiedSolver` triad, which were added per the `FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md` roadmap.
3. **`#![forbid(unsafe_code)]` is NOT declared.** The crate does not enforce no-unsafe at the lib.rs level. Verified 2026-05-12: workspace grep for the keyword `unsafe` across `astraweave-fluids/src/*.rs` returned exactly 2 matches — both at `debug_viz.rs:479-480` (`unsafe impl bytemuck::Pod for DebugVertex {}` + `unsafe impl bytemuck::Zeroable for DebugVertex {}`). These are standard bytemuck unsafe-trait impls (no unsafe blocks). The crate contains zero `unsafe { ... }` blocks in the surveyed 84.5K LoC.
4. **GPU-first design.** The core `FluidSystem` constructor (`lib.rs:362`) takes `&wgpu::Device` and creates compute pipelines from `shaders/fluid.wgsl`. Most subsystems hold `wgpu::Buffer`/`wgpu::ComputePipeline`/`wgpu::BindGroup` resources directly.
5. **Two Cargo features:** `default = ["serde"]` (Cargo.toml:7-8) enables Serde derives on config types; `parallel = ["dep:rayon"]` (`:9`) enables Rayon-parallel CPU helpers. **Neither is feature-gated extensively in the source** — verified `parallel` mainly gates `simd_ops::parallel` namespace per bench imports at `benches/fluids_adversarial.rs:1749, :1785`.
6. **Single integration test file: `tests/mutation_resistant_comprehensive_tests.rs` (785 LoC).** Per CLAUDE.md mutation-testing campaign convention, this is the dedicated mutation-resistance test file. There are no other integration tests in `tests/`. Inline unit tests exist in source files.
7. **The audit + roadmap doc (`docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`)** rates the current state at "Overall Current Grade: B (Good for games, insufficient for research)" and documents the gap-to-target for each subsystem.

---

## 2. Authoritative Pipeline

### 2.1 Core PBD GPU simulation pipeline (the original `FluidSystem`)

```text
[Host code: caller creates FluidSystem::new(&device, particle_count)]
    │
    │ FluidSystem::new
    │ file: astraweave-fluids/src/lib.rs:362-…
    ▼
[FS1: Build initial particle grid]
    role: Generate `particle_count` particles in a cubic grid layout (spacing 0.5, size = cbrt(count)).
          Default `Particle { position: [x,y,z,1], velocity: [0;4], predicted_position, lambda: 0,
          density: 0, phase: 0=water, temperature: 293K, color: [0.2,0.5,0.8,1] }`.
    file: lib.rs:370-391
    │
    ▼
[FS2: Create ping-pong storage buffers + 8-stage compute pipeline]
    role: 2× Vec<wgpu::Buffer> particle_buffers (ping-pong), secondary_particle_buffer,
          density_error_buffer (+ 2× staging buffers), head_pointers + next_pointers grid
          linked-list buffers, particle_flags (active/inactive), params_buffer (SimParams).
          Eight compute pipelines: clear_grid, build_grid, predict, lambda, delta_pos,
          integrate, mix_dye + secondary spawn (per `pub fn new` continuation lines).
    files: lib.rs:393-…, shaders/fluid.wgsl (481 LoC, 8 compute kernels)
    │
    ▼
[Game tick — caller invokes per-frame update]
    │
    │ Per-frame caller: encode compute pass → dispatch all 8 pipelines in order
    │ (clear_grid → build_grid → predict → iterate(lambda → delta_pos) for N iterations →
    │ integrate → mix_dye → secondary-particle spawn check)
    ▼
[FS3a: Clear grid]
    shader: shaders/fluid.wgsl (clear_grid kernel)
    role: Reset head_pointers grid to -1 (no particles in cells)
    │
    ▼
[FS3b: Build grid (spatial hash)]
    shader: shaders/fluid.wgsl (build_grid kernel)
    role: Atomic-insert each particle into its grid cell linked-list via head_pointers + next_pointers
    │
    ▼
[FS3c: Predict positions]
    shader: shaders/fluid.wgsl (predict kernel)
    role: predicted_position = position + velocity * dt + gravity contribution
    │
    ▼
[FS3d: Iterative density correction (N iterations)]
    shaders: fluid.wgsl (lambda + delta_pos kernels)
    role: PBD pressure iterations — compute lambda (density constraint multiplier) from
          neighbors via grid-search, then apply delta_pos correction to predicted_position.
          Adaptive iteration count from `AdaptiveIterations` (optimization.rs)
    │
    ▼
[FS3e: Integrate final position]
    shader: shaders/fluid.wgsl (integrate kernel)
    role: velocity = (predicted_position - position) / dt; position = predicted_position;
          apply XSPH viscosity smoothing; update density buffer
    │
    ▼
[FS3f: Mix dye + Secondary particle spawn]
    shaders: fluid.wgsl (mix_dye + secondary kernels), shaders/secondary.wgsl
    role: Color-mixing for multi-phase visualization. Secondary spawn checks velocity/curvature
          criteria and writes to secondary_particle_buffer + secondary_counter (foam/spray/bubble)
    │
    ▼
[FS4: Frame copy density_error to staging for async readback]
    role: Copy density_error_buffer → density_error_staging_buffers (ping-pong); caller
          can poll staging_mapped[] flags to read convergence metrics
    │
    ▼
[Caller: render via FluidRenderer or read positions for game logic]
```

### 2.2 Screen-Space Fluid Rendering pipeline (`FluidRenderer`)

```text
[Caller: invoke FluidRenderer::render(...) with particle storage buffer + camera matrices]
    │
    │ file: astraweave-fluids/src/renderer.rs:47-…
    ▼
[R1: Depth pass]
    shader: shaders/ssfr_depth.wgsl
    pipeline: depth_pipeline (RenderPipeline)
    role: Rasterize each particle as a screen-space sphere; write depth = ray-sphere intersection
    output: depth_texture
    │
    ▼
[R2: Smooth pass (bilateral blur)]
    shader: shaders/ssfr_smooth_v2.wgsl
    pipeline: smooth_pipeline (ComputePipeline)
    role: Bilateral blur on depth_texture; preserves edges via depth_falloff parameter
    output: smoothed_depth_texture
    │
    ▼
[R3: Shade pass (PBR + caustics + god rays)]
    shader: shaders/ssfr_shade.wgsl
    pipeline: shade_pipeline (RenderPipeline)
    role: Reconstruct surface normal from smoothed depth; apply Beer-Lambert absorption,
          PBR shading, optional caustics + god-rays texture sampling
    output: Final water surface pixels
    │
    ▼
[R4: Secondary particle pass]
    shader: shaders/secondary.wgsl
    pipeline: secondary_pipeline (RenderPipeline)
    role: Render foam/spray/bubble particles from secondary_particle_buffer (instanced quads)
```

### 2.3 Voxel water grid (parallel non-particle path)

```text
[Caller: build WaterVolumeGrid with dimensions + cell_size]
    │
    │ file: astraweave-fluids/src/volume_grid.rs
    ▼
[V1: Per-cell state]
    role: 3D grid of WaterCell { level: f32, velocity: Vec3, material: MaterialType (8 variants
          Air/Stone/Soil/Mud/Rubble/Shroud/Glass/Wood), pressure: f32, temperature: f32,
          flags: CellFlags }. Material defines absorption rate + flow blocking
    file: volume_grid.rs:11-79
    │
    ▼
[V2: CPU update (hydrostatic pressure + flow + absorption + temperature)]
    role: Per-tick: compute hydrostatic pressure from water column above; flow water to
          neighbors based on pressure gradient; absorb water into porous materials at
          MaterialType-defined rates (Mud 50%/s, Shroud 80%/s, Soil 1%/s, etc.)
    │
    ▼
[V3: GPU upload via WaterVolumeGpu]
    file: gpu_volume.rs:1-80
    role: Convert WaterCell → GpuWaterCell (16-byte aligned: level, velocity_x/y/z);
          upload to 3D texture; build heightfield-based surface mesh via marching squares
          or similar; produce WaterSurfaceVertex[] (32 bytes: position[3], normal[3], uv[2])
    │
    ▼
[V4: Render via FluidRenderer or custom mesh draw]
```

### 2.4 Visual effects coordinator (`WaterEffectsManager`)

```text
[Caller: WaterEffectsManager::from_preset(WaterQualityPreset)]
    │
    │ file: astraweave-fluids/src/water_effects.rs:1-100+
    ▼
[WE1: Construct composed effects]
    role: Per preset (Low/Medium/High/Ultra/Custom), instantiate subset of:
          - CausticsSystem (caustics.rs) — Voronoi-pattern underwater light refraction
          - FoamSystem (foam.rs) — whitecaps, wakes, shore foam
          - GodRaysSystem (god_rays.rs) — volumetric light shafts
          - WaterReflectionSystem (water_reflections.rs) — SSR + planar
          - UnderwaterParticleSystem (underwater_particles.rs) — bubbles, debris
          - WaterfallSystem (waterfall.rs) — vertical particle rapids
          - UnderwaterState (underwater.rs) — depth-zone fog/density transitions
    │
    ▼
[WE2: Per-frame update(dt, camera_pos, water_height)]
    role: Update each enabled subsystem; collect WaterEffectsStats; return WaterEffectsResult<()>
          (typed errors: InvalidConfig, NotInitialized, ResourceLimitExceeded, InvalidStateTransition)
```

### 2.5 Terrain integration (water body detection)

```text
[Caller: analyze_terrain_for_water(heightmap, config: TerrainFluidConfig)]
    │
    │ file: terrain_integration.rs (860 LoC)
    ▼
[T1: Analyze heightmap]
    role: Detect river paths (downhill flow tracing), lake basins (closed contour depressions),
          waterfall edges (sharp height gradients), ocean coast (sea-level adjacency)
    output: Vec<DetectedWaterBody> with WaterBodyType per body (7 variants: River/Stream/Lake/Pond/Ocean/Waterfall/Aquifer)
    │
    ▼
[T2: Caller instantiates volumes from DetectedWaterBody]
    role: For each detected body, configure WaterVolumeGrid OR FluidSystem OR WaterEffectsManager
          (caller decides which subsystem renders/simulates the body)
```

### 2.6 Building integration (water dispensers/drains/gates/wheels)

```text
[Caller: WaterBuildingManager::new + add water_dispenser/drain/gate/wheel components]
    │
    │ file: building.rs (1,116 LoC)
    ▼
[B1: Per-frame update against WaterVolumeGrid]
    role: WaterDispenser emits water at FlowDirection (Down/Up/East/West/South/North);
          WaterDrain (alias VolumetricDrain) absorbs water; WaterGate opens/closes flow paths;
          WaterWheel rotates based on water flow (WheelAxis enum)
```

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **`FluidSystem`** | The original PBD-based GPU fluid simulator. Holds 8 compute pipelines + ping-pong buffers + grid-linked-list. Constructor takes `&wgpu::Device, particle_count: u32`. | `lib.rs:250-415` |
| **`Particle`** | GPU particle: `position: [f32; 4]`, `velocity: [f32; 4]`, `predicted_position: [f32; 4]`, `lambda: f32`, `density: f32`, `phase: u32` (0=water, 1=oil, 2=custom), `temperature: f32` (Kelvin), `color: [f32; 4]`. Total 80 bytes, `bytemuck::Pod + Zeroable`. | `lib.rs:208-219` |
| **`SimParams`** | GPU uniform: `smoothing_radius`, `target_density`, `pressure_multiplier`, `viscosity`, `surface_tension`, `gravity`, `dt`, `particle_count: u32`, grid dimensions + cell_size, `object_count: u32` + 3 f32 pad fields. Total 64 bytes. | `lib.rs:221-248` |
| **`SecondaryParticle`** | Foam/spray/bubble particle: position, velocity, info (lifetime, type, alpha, scale). Total 48 bytes. | `lib.rs:354-360` |
| **`UnifiedSolver`** | High-level interface combining research-grade SPH solvers (PBD/PCISPH/DFSPH/IISPH) + viscosity models + multi-phase + vorticity confinement + boundary handling. Includes built-in validation metrics. | `unified_solver.rs:1-…` |
| **`UnifiedSolverConfig`** | Config selecting `SolverType`, `ViscositySolverType`, `BoundaryMethod`, phase configs, quality preset. | `unified_solver.rs:…` |
| **`SolverType` (unified_solver.rs)** | `#[non_exhaustive]` enum: `Pbd`, `Pcisph` (default), `Dfsph`, `Iisph`. Note: lowercase variants. | `unified_solver.rs:50-60` |
| **`SolverType` (research.rs)** | `#[non_exhaustive]` enum: `PBD` (default), `PCISPH`, `DFSPH`, `IISPH`. **Naming collision** with `unified_solver::SolverType` — UPPERCASE acronyms here, lowercase there. See §6. | `research.rs:46-56` |
| **`ViscositySolverType`** | `#[non_exhaustive]` enum: `Xsph`, `Morris` (default), `ImplicitJacobi`. | `unified_solver.rs:65-73` |
| **`ViscositySolver` (research.rs)** | Parallel enum: `XSPH` (default), `Morris`, `ImplicitJacobi`. **Naming collision** with `ViscositySolverType` above. | `research.rs:81-89` |
| **`QualityPreset` (unified_solver.rs)** | `#[non_exhaustive]` 5-variant: `Mobile`, `Console` (default), `PcHigh`, `PcUltra`, `Research`. | `unified_solver.rs:28-44` |
| **`WaterQualityPreset` (water_effects.rs)** | `#[non_exhaustive]` 5-variant: `Low`, `Medium` (default), `High`, `Ultra`, `Custom`. Parallel to QualityPreset above but for visual effects. | `water_effects.rs:72-97` |
| **`ResearchQualityTier`** | `#[non_exhaustive]` 5-variant enum: `Low`, `Medium` (default), `High`, `Ultra`, `Research`. Per inline doc-comments: Low=50-100k particles PBD+XSPH, Medium=100-200k PCISPH+Morris, High=200-350k DFSPH+δ-SPH+vorticity, Ultra=350-500k full, Research=500k-1M offline + VTK export. | `research.rs:198-213` |
| **`ResearchFluidSystem`** | Research-grade SPH GPU pipeline supporting PCISPH/DFSPH/IISPH solvers + δ-SPH particle shifting + warm-starting + non-Newtonian viscosity + vorticity confinement + micropolar SPH. | `research.rs:1-100+` |
| **`PCISPHSystem`** | Dedicated PCISPH GPU implementation. Constants: `MAX_PARTICLES = 1_000_000`, `MAX_PCISPH_ITERATIONS = 50`, `DEFAULT_DENSITY_THRESHOLD = 0.001`, `WORKGROUP_SIZE = 64`, `DEFAULT_SMOOTHING_RADIUS = 1.2`. | `pcisph_system.rs:1-80` |
| **`IterationState`** | PCISPH per-iteration CPU-side state: `iteration: u32`, `max_density_error: f32`, `avg_density_error: f32`, `converged: u32`. `bytemuck::Pod + Zeroable`. | `pcisph_system.rs:42-49` |
| **`PhysicalParams`** | PCISPH simulation knobs: smoothing_radius, target_density, base_viscosity, surface_tension, gravity, pressure_stiffness, `delta_sph_c_delta`, `sor_omega`, `divergence_error_threshold`. | `pcisph_system.rs:55-93` |
| **`ResearchFluidConfig` / `ResearchParticle` / `ResearchSimParams` / `FluidPhase` / `ShiftingMethod`** | Research-pipeline types in `research.rs`. Separate type family from `Particle`/`SimParams`/`SecondaryParticle` in `lib.rs`. | `research.rs` |
| **`WaterVolumeGrid`** | Voxel-based water grid (parallel to particle simulation). Holds `Vec<WaterCell>` with hydrostatic pressure + flow + absorption + temperature. Inspired by Enshrouded "Wake of Water". | `volume_grid.rs:1-79+` |
| **`WaterCell`** | Voxel cell: `level: f32` (0=empty, 1=full), `velocity: Vec3`, `material: MaterialType`, `pressure: f32`, `temperature: f32`, `flags: CellFlags`. | `volume_grid.rs:65-79` |
| **`MaterialType`** | `#[non_exhaustive] #[repr(u8)]` 8-variant: `Air` (default, =0), `Stone` (=1), `Soil` (=2), `Mud` (=3), `Rubble` (=4), `Shroud` (=5), `Glass` (=6), `Wood` (=7). Methods: `absorption_rate()` (Stone/Glass/Air=0, Mud=0.5/s, Shroud=0.8/s, Rubble=0.05, Soil=0.01, Wood=0.002), `blocks_flow()` (Stone/Glass), `allows_water()`. | `volume_grid.rs:11-62` |
| **`GpuWaterCell`** | 16-byte GPU-aligned version of `WaterCell`: level + velocity_x/y/z. | `gpu_volume.rs:15-40` |
| **`WaterSurfaceVertex`** | 32-byte vertex: position[3], normal[3], uv[2]. `wgpu::VertexBufferLayout` provided. | `gpu_volume.rs:41-80` |
| **`WaterEffectsManager`** | Coordinator for all visual effects subsystems. Owns optional `CausticsSystem`, `FoamSystem`, `GodRaysSystem`, `WaterReflectionSystem`, `UnderwaterParticleSystem`, `WaterfallSystem`, `UnderwaterState` per quality preset. | `water_effects.rs:1-100+` |
| **`WaterEffectsError`** | `#[non_exhaustive] #[must_use]` 4-variant error: `InvalidConfig { field, reason }`, `NotInitialized { system }`, `ResourceLimitExceeded { resource, limit, requested }`, `InvalidStateTransition { from, to }`. Implements `Display` + `Error`. | `water_effects.rs:18-65` |
| **`WaterEffectsResult<T>`** | Type alias `Result<T, WaterEffectsError>`. | `water_effects.rs:67` |
| **`WaterBuildingManager`** | Building-side water entities coordinator. Manages `WaterDispenser`, `WaterDrain` (aliased `VolumetricDrain` at re-export to disambiguate from particle-side drains), `WaterGate`, `WaterWheel`. | `building.rs:1-50+` |
| **`FlowDirection`** | `#[non_exhaustive]` 6-variant: `Down` (default), `Up`, `East`, `West`, `South`, `North`. Helpers: `to_vec3()`, `to_ivec3()`. | `building.rs:11-49+` |
| **`WaterBodyType`** | Terrain-integration type. `#[non_exhaustive]` 7-variant: `River`, `Stream`, `Lake`, `Pond`, `Ocean`, `Waterfall`, `Aquifer`. | `terrain_integration.rs:13-58` |
| **`DetectedWaterBody`** | Result of `analyze_terrain_for_water` — describes a discovered river/lake/etc with bounding region + WaterBodyType + flow parameters. | `terrain_integration.rs` |
| **`FluidLodManager` + `OptimizedLodManager`** | Distance-based LOD systems. `FluidLodConfig`, `LodLevel`, `LodUpdateResult`, `OptimizedLodConfig` companion types. `ParticleStreamingManager` + `StreamingOp` for streaming. | `lod.rs` |
| **`FluidOptimizationController`** | Production auto-tuning controller. Composed of `WorkgroupConfig` (GPU-vendor-aware), `AdaptiveIterations`, `SimulationBudget` (frame-time budget), `TemporalCoherence` (skip resting particles), `BatchSpawner`, `OptimizationProfiler`. **Verified 2026-05-12: defined in `lib.rs:1433`**, NOT in `optimization.rs` (the previous trace location was wrong). The constituent types are in `optimization.rs`. | `lib.rs:1418-1483, :2062` |
| **`GpuVendor`** | `#[non_exhaustive]` 5-variant: `Nvidia`, `Amd`, `Intel`, `Apple`, `Unknown` (default). Drives workgroup sizing. | `optimization.rs:53-67` |
| **`WorkgroupConfig`** | `particle_workgroup`, `grid_workgroup`, `secondary_workgroup`. Defaults to `universal()` (64 threads). Constructors: `universal`, `nvidia`, `amd`, `apple`, `auto_detect`. | `optimization.rs:80-100+` |
| **`MortonCode`** | Z-order curve encoding for spatial coherence sorting. | `optimization.rs:…` |
| **`OptimizationPreset` / `QualityTier`** | Editor-friendly optimization presets. | `optimization.rs:…` |
| **`OptimizationStats`** | Runtime telemetry: `quality_level: f32 (0.0-1.0)`, `iterations: u32`, `resting_particles: u32`, `recommended_iterations: u32`, `under_budget: bool`. | `lib.rs:340-352` |
| **`FluidProfiler` + `FluidTimingStats`** | Per-frame profiling capture. | `profiling.rs:1-…` |
| **`BoundaryMethod`** | SPH boundary handling — likely SDF / Akinci / hybrid. | `boundary.rs` |
| **`TurbulenceSystem`** + `VorticityConfinementConfig` + `MicropolarConfig` | Turbulence injection. Vorticity confinement (re-inject lost vortices) + micropolar SPH (particle spin). | `turbulence.rs` |
| **`MetricsHistory`** | Research-grade validation metrics over time (density error, divergence, energy). | `validation.rs` |
| **`NonNewtonianModel`** | Carreau-model shear-thinning/thickening viscosity. | `viscosity.rs` |
| **`SdfSystem`** | Signed-Distance Field generation for boundary representation. Uses Jump-Flood Algorithm (JFA) per README. | `sdf.rs` + `shaders/sdf_gen.wgsl` |
| **`CAUSTICS_WGSL` / `GOD_RAYS_WGSL` / `SSR_WGSL`** | Inline-WGSL string constants exposed for caller-side shader composition. | `caustics.rs`, `god_rays.rs`, `water_reflections.rs` |
| **`FluidEditorConfig`** | Top-level editor-side fluid configuration. Composed of sub-configs for waves, emitters, drains, caustics, foam, god rays, reflections, underwater, waterfall, thermal, rendering, physics, LOD. | `editor.rs:…` |
| **`WaterBodyPreset`** | Editor presets — pre-configured water body types (TropicalOcean, Ocean, River, Pool, Waterfall, etc., per editor.rs:23-25 doc-example). | `editor.rs:48-…` |
| **`ConfigHistory`** | Editor undo/redo system with VecDeque-based history. | `editor.rs` |
| **`ConfigClipboard`** | Editor copy/paste for fluid configurations. | `editor.rs` |
| **`ConfigValidator`** | Real-time validation with safe clamping. Emits `ValidationIssue { severity: ValidationSeverity }`. | `editor.rs` |
| **`ValidationSeverity`** | Editor validation severity enum. | `editor.rs` |
| **`EasingFunction`** | Animation easing for smooth parameter transitions (presumed Linear/Cubic/etc.). | `editor.rs` |
| **`ColorblindPalette`** | Color-blind-safe visualization palette options. | `editor.rs` |
| **`BatchOperation`** | Multi-select editing operations. | `editor.rs` |
| **`KeyboardShortcut`** | Editor keybinding type. | `editor.rs` |
| **`PreviewHint`** | Real-time preview hint enum. | `editor.rs` |
| **`WidgetType`** | Editor UI widget metadata for inspector generation. | `editor.rs` |
| **`FluidAABB` + `FluidScenePlacement`** | Scene-integration types for placing fluids in 3D space. | `editor.rs` |
| **`SIMD_BATCH_SIZE`** | Constant = 8. Drives batch sizing for SIMD-friendly SPH primitives. | `simd_ops.rs:34` |
| **`batch_distances` / `batch_kernel_cubic` / `batch_kernel_gradient_cubic`** | Vectorized SPH primitives using glam auto-vectorization. Wendland C2/C4 + Cubic Spline kernels available. | `simd_ops.rs:42-…` |
| **`accumulate_density_simple` / `accumulate_pressure_force` / `accumulate_viscosity_force`** | Per-particle accumulator helpers. | `simd_ops.rs` |
| **`NEIGHBOR_OFFSETS`** | 27-element array of `[i32; 3]` 3D-neighbor offsets for grid iteration. | `simd_ops.rs` |
| **`position_to_cell` / `cell_hash` / `position_to_morton`** | Spatial indexing helpers. | `simd_ops.rs` |
| **`par_*` family (parallel feature)** | Rayon-parallel variants: `par_batch_kernel_cubic`, `par_compute_morton_codes`, etc. Gated on `parallel` feature flag (Cargo.toml:9). | `simd_ops::parallel` namespace |

### Terms to NOT confuse

- **`SolverType` in `unified_solver.rs` vs `SolverType` in `research.rs`**: Two enums with the same name in different modules. `unified_solver::SolverType` has `Pbd / Pcisph / Dfsph / Iisph` (lowercase). `research::SolverType` has `PBD / PCISPH / DFSPH / IISPH` (UPPERCASE acronyms). The crate root re-exports `unified_solver::SolverType` via `pub use unified_solver::{... SolverType ...}` at `lib.rs:187`. Code paths using `research.rs` types must namespace explicitly.
- **`ViscositySolverType` (`unified_solver.rs:65-73`) vs `ViscositySolver` (`research.rs:81-89`)**: Different type names. Same conceptual purpose. UPPERCASE acronyms in research path.
- **`QualityPreset` (`unified_solver.rs:28-44`) vs `WaterQualityPreset` (`water_effects.rs:72-97`) vs `QualityTier` (`optimization.rs`) vs `ResearchQualityTier` (`research.rs`)**: Four parallel quality enums for different subsystems. They are NOT unified or aliased.
- **`FluidSystem` (`lib.rs:250-415+`) vs `UnifiedSolver` (`unified_solver.rs`) vs `ResearchFluidSystem` (`research.rs`) vs `PCISPHSystem` (`pcisph_system.rs`)**: Four solver/manager surfaces. `FluidSystem` is the original PBD GPU pipeline. The other three are research-grade additions with overlapping responsibilities. The example consumer (`examples/fluids_demo/src/main.rs:20`) uses `FluidSystem`, NOT the unified or research variants.
- **`Particle` (`lib.rs:208-219`, 80 bytes) vs `ResearchParticle` (`research.rs`)**: Two GPU particle layouts. Cannot be mixed.
- **`SimParams` (`lib.rs:221-248`, 64 bytes) vs `ResearchSimParams` (`research.rs`)**: Two GPU uniform layouts. Cannot be mixed.
- **`WaterDrain` (`building.rs`, voxel-side) renamed to `VolumetricDrain` at crate re-export (`lib.rs:90`) vs `FluidDrain` (`emitter.rs`, particle-side)**: Two distinct drain types. The lib.rs explicitly renames to disambiguate. `WaterDrain` here aliases to `VolumetricDrain` for the voxel grid; `FluidDrain` is from `emitter.rs` for particles.
- **`WaterfallConfig` (`waterfall.rs`, particle-side) vs `TerrainWaterfallConfig` (`terrain_integration.rs::WaterfallConfig`, terrain-side)**: Two waterfall configs. The lib.rs re-exports the terrain one as `TerrainWaterfallConfig` at `:179` to disambiguate.
- **`PhysicsConfig` (in `editor.rs`)**: NOT the physics-side `astraweave_physics::PhysicsConfig`. This is the editor-side fluid physics config defined at `editor.rs:2094-2113` with 9 fields: `smoothing_radius: f32` (0.5-5.0), `target_density: f32` (1.0-50.0), `pressure_multiplier: f32` (10.0-1000.0), `viscosity: f32` (0.0-100.0), `surface_tension: f32` (0.0-1.0), `gravity: [f32; 3]`, `iterations: u32` (1-20), `enable_vorticity: bool`, `vorticity_strength: f32` (0.0-1.0). Derives `Clone, Debug, Serialize, Deserialize`.
- **`LodConfig` aliased as `EditorLodConfig` (`lib.rs:136`) vs `FluidLodConfig` (`lod.rs`)**: Two LOD configs — editor-side and runtime-side. Lib.rs explicitly renames the editor one to `EditorLodConfig` to disambiguate.
- **`SSFR` (Screen-Space Fluid Rendering)** vs **`SSR` (Screen-Space Reflection)**: Both used. SSFR is the surface reconstruction pipeline (`ssfr_depth.wgsl`, `ssfr_shade.wgsl`, `ssfr_smooth_v2.wgsl`). SSR is reflection (referenced by `SSR_WGSL` re-export at `lib.rs:198`).

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| `wgpu::Device` (host) | `FluidSystem::new(&device, particle_count)` at `lib.rs:362` | Device handle | The crate is GPU-first; every solver/renderer takes `&wgpu::Device` |
| `wgpu::Queue` (host) | Buffer writes (`queue.write_buffer(params_buffer, ...)`) | `SimParams` updates | Caller-driven per-frame parameter updates |
| Terrain heightmap | `analyze_terrain_for_water(heightmap, TerrainFluidConfig)` at `terrain_integration.rs` | Heightmap data + flow detection config | Caller provides heightmap; output is `Vec<DetectedWaterBody>` |
| Camera state | `CameraUniform { view_proj, inv_view_proj, view_inv, cam_pos, light_dir, time }` at `renderer.rs:7-16` | Camera matrices + time | 200-byte uniform consumed by all SSFR passes |
| Editor / authoring layer | `FluidEditorConfig` + `ConfigHistory::push(...)` + `from_preset(WaterBodyPreset)` per `editor.rs:21-41` doc-comment | Config objects | Editor-driven workflow |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| `examples/fluids_demo` | `use astraweave_fluids::{FluidSystem, FluidRenderer, FluidLodConfig, FluidLodManager, FluidOptimizationController}` at `examples/fluids_demo/src/main.rs:18-21` | All public surface | **The only workspace consumer** (verified 2026-05-12) |
| **No game-loop crate** | n/a | n/a | Verified workspace grep: `use astraweave_fluids` returns only the fluids crate itself, the demo example, the mutation test, and the bench. Zero production engine consumers. |

### Bidirectional / Coupled

- **`FluidSystem` ↔ `wgpu::Device` resources**: Owns 30+ `wgpu::Buffer`/`wgpu::ComputePipeline`/`wgpu::BindGroup`/`wgpu::BindGroupLayout` resources (`lib.rs:250-340`). The device is shared but the buffers are exclusively owned.
- **`FluidSystem` ↔ `SdfSystem` (`lib.rs:301`)**: `FluidSystem` holds `pub sdf_system: crate::sdf::SdfSystem`. The SDF is used as boundary representation for compute pass — particles read SDF distance + normal during collision response.
- **`WaterEffectsManager` ↔ 7 visual subsystems**: Holds optional fields for `CausticsSystem`, `FoamSystem`, `GodRaysSystem`, `WaterReflectionSystem`, `UnderwaterParticleSystem`, `WaterfallSystem`, `UnderwaterState`. Per-frame `update` cascades to each.
- **`WaterVolumeGrid` ↔ `WaterBuildingManager`**: `WaterBuildingManager` operates on `&mut WaterVolumeGrid` (per `building.rs:9` — `use crate::volume_grid::{CellFlags, WaterVolumeGrid}`). Dispensers add water cells; drains remove; gates modify CellFlags.
- **`FluidSystem` ↔ density_error staging buffer pipeline**: Producer-consumer pattern via `density_error_buffer` → `density_error_staging_buffers[2]` ping-pong + `staging_mapped: [bool; 2]` flags (`lib.rs:306-308`). Caller polls for async readback of convergence metrics.
- **`OptimizationController` ↔ `FluidSystem.iterations`**: `AdaptiveIterations` (in `OptimizationController`) reads density_error feedback and adjusts the iteration count consumed by `FluidSystem`'s PBD loop on the next frame.

---

## 5. Active File Map

### `astraweave-fluids` — fluid simulation crate

| File | LoC | Role | Status | Notes |
|---|---|---|---|---|
| `astraweave-fluids/src/lib.rs` | 3,810 | Re-exports + `Particle` / `SimParams` / `SecondaryParticle` GPU types + `FluidSystem` original PBD GPU pipeline + `OptimizationStats` | Active (in demo) | The example consumer constructs `FluidSystem::new(&device, particle_count)` (`fluids_demo/src/main.rs:19-21`). No production crate constructs `FluidSystem` directly. |
| `astraweave-fluids/src/anisotropic.rs` | 415 | Anisotropic kernel surface for sharper fluid surfaces | Active (module-level) | Companion shader at `shaders/anisotropic.wgsl` (86 LoC) |
| `astraweave-fluids/src/boundary.rs` | 1,411 | `BoundaryMethod` SPH boundary handling (SDF + Akinci or hybrid) | Active (module-level) | Used by `unified_solver.rs:15` (`use crate::boundary::BoundaryMethod`) |
| `astraweave-fluids/src/building.rs` | 1,116 | `WaterBuildingManager` + `WaterDispenser` / `WaterDrain` (aliased `VolumetricDrain`) / `WaterGate` / `WaterWheel` / `FlowDirection` / `WheelAxis` | Active (module-level) | Consumes `WaterVolumeGrid` from `volume_grid.rs` |
| `astraweave-fluids/src/caustics.rs` | 728 | `CausticsProjector` + `CausticsSystem` + `CausticsUniforms` + `CausticsConfig` + `CausticSample` + `CAUSTICS_WGSL` inline-WGSL | Active (module-level) | Voronoi-pattern caustics per README |
| `astraweave-fluids/src/debug_viz.rs` | 665 | `DebugDrawList` + `DebugLine` + `DebugPoint` + `DebugVertex` + `ParticleDebugType` + `StatsFormatter` + `WaterDebugConfig` | Active (module-level) | Debug visualization |
| `astraweave-fluids/src/editor.rs` | **5,823** | Editor integration: `FluidEditorConfig`, `WaterBodyPreset`, `ConfigHistory` (undo/redo), `ConfigClipboard`, `ConfigValidator`, `ValidationSeverity`, `ValidationIssue`, `BatchOperation`, `EasingFunction`, `ColorblindPalette`, `AccessibilitySettings`, `KeyboardShortcut`, `PreviewHint`, `WidgetType`, `EditorMetadata`, `ConfigTransition`, `FluidPerformanceMetrics`, `FluidAABB`, `FluidScenePlacement`, `ExportedPreset`, `FieldMetadata`, 12+ sub-config types (Caustics/Drain/Emitter/Foam/GodRays/Reflection/Rendering/Thermal/Underwater/Waterfall/Wave/Flow/Physics/Lod) | Active (module-level) — forward-design only | Largest non-SIMD file. Comprehensive editor surface. **Closed from §11 via deep investigation 2026-05-12:** NOT wired into `tools/aw_editor`. Workspace grep for `use astraweave_fluids`/`astraweave_fluids::` inside `tools/aw_editor` returned zero matches; `tools/aw_editor/Cargo.toml` does not declare `astraweave-fluids` as a dependency. |
| `astraweave-fluids/src/emitter.rs` | 827 | `EmitterShape` + `FluidDrain` + `FluidEmitter` (Point/Sphere/Box/Mesh shapes per README) | Active (module-level) | |
| `astraweave-fluids/src/foam.rs` | 780 | `FoamSystem` + `FoamConfig` + `FoamParticle` + `FoamSource` + `FoamTrail` + `GpuFoamParticle` | Active (module-level) | Whitecaps, wakes, shore foam |
| `astraweave-fluids/src/god_rays.rs` | 621 | `GodRaysSystem` + `GodRaysConfig` + `GodRaysUniforms` + `LightShaft` + `GOD_RAYS_WGSL` inline-WGSL | Active (module-level) | Volumetric light shafts |
| `astraweave-fluids/src/gpu_volume.rs` | 1,676 | `WaterVolumeGpu` + `GpuWaterCell` (16-byte aligned) + `WaterSurfaceVertex` (32-byte) + `WaterVolumeUniforms` + heightfield surface meshing | Active (module-level) | GPU-side of voxel water grid |
| `astraweave-fluids/src/lod.rs` | 1,269 | `FluidLodManager` + `FluidLodConfig` + `LodLevel` + `LodUpdateResult` + `OptimizedLodManager` + `OptimizedLodConfig` + `ParticleStreamingManager` + `StreamingOp` | Active (in demo) | Used by `fluids_demo` per imports |
| `astraweave-fluids/src/multi_phase.rs` | 1,583 | Multi-phase fluid surface (water/oil/custom phase interactions with δ⁺-SPH interfaces per audit doc) | Active (module-level) | |
| `astraweave-fluids/src/optimization.rs` | 2,392 | `WorkgroupConfig` + `AdaptiveIterations` + `SimulationBudget` + `TemporalCoherence` + `BatchSpawner` + `OptimizationProfiler` + `MortonCode` + `GpuVendor` + `OptimizationPreset` + `OptimizationRecommendation` + `OptimizationMetrics` + `OptimizedSimParams` + `ParticleStateGpu` + `QualityTier` + `GpuShaderConfig` + `analyze_metrics` | Active (in demo) | **Correction (verified 2026-05-12):** the `FluidOptimizationController` struct that the demo imports is NOT defined in this file — it lives at `lib.rs:1418-…`, `lib.rs:1433` (struct decl), `lib.rs:1477` (Default impl), `lib.rs:1483` + `:2062` (inherent impl blocks). The previous trace claim that it lives in `optimization.rs` was incorrect. |
| `astraweave-fluids/src/particle_shifting.rs` | 738 | δ-SPH particle shifting (Marrone et al. 2011) for tensile-instability fix | Active (module-level) | |
| `astraweave-fluids/src/pcisph_system.rs` | 1,620 | `PCISPHSystem` GPU PCISPH implementation. Constants: `MAX_PARTICLES = 1_000_000`, `MAX_PCISPH_ITERATIONS = 50`, `DEFAULT_DENSITY_THRESHOLD = 0.001`, `WORKGROUP_SIZE = 64`, `DEFAULT_SMOOTHING_RADIUS = 1.2`. + `IterationState` + `PhysicalParams` | Active (module-level) | Solver alternative to `FluidSystem`'s PBD |
| `astraweave-fluids/src/profiling.rs` | 527 | `FluidProfiler` + `FluidTimingStats` | Active (module-level) | Per-subsystem timing instrumentation |
| `astraweave-fluids/src/renderer.rs` | 748 | `FluidRenderer` SSFR pipeline (depth + smooth + shade + secondary) + `CameraUniform` (200-byte) + `SmoothParams` | Active (in demo) | The only rendering surface |
| `astraweave-fluids/src/research.rs` | 1,190 | `ResearchFluidSystem` + `ResearchFluidConfig` + `ResearchParticle` + `ResearchSimParams` + `SolverType` (UPPERCASE acronyms) + `ViscositySolver` + `ShiftingMethod` + `FluidPhase` + `ResearchQualityTier` | Active (module-level) | Research-grade SPH alternative to `FluidSystem` |
| `astraweave-fluids/src/sdf.rs` | 750 | `SdfSystem` (Jump-Flood Algorithm per README) | Active (in `FluidSystem`) | Required by `FluidSystem.sdf_system` field at `lib.rs:301` |
| `astraweave-fluids/src/serialization.rs` | 395 | `FluidSnapshot` + `SnapshotParams` save/load via bincode | Active (module-level) | |
| `astraweave-fluids/src/simd_ops.rs` | **39,554** | Vectorized SPH primitives: `batch_distances`, `batch_kernel_cubic`, `batch_kernel_gradient_cubic`, `accumulate_density_simple`, `accumulate_pressure_force`, `accumulate_viscosity_force`, `aos_to_soa_positions` / `soa_to_aos_positions`, `position_to_cell`, `cell_hash`, `position_to_morton`, `NEIGHBOR_OFFSETS`, `batch_apply_gravity`, `batch_integrate_positions`. Plus `parallel` sub-module (feature `parallel`): `par_batch_kernel_cubic`, `par_compute_morton_codes`. `SIMD_BATCH_SIZE = 8`. | Active (module-level) | **Largest file in the crate**. Mostly batch operations / inline functions designed for LLVM auto-vectorization. |
| `astraweave-fluids/src/terrain_integration.rs` | 860 | `analyze_terrain_for_water` + `DetectedWaterBody` + `WaterBodyType` (7 variants) + `TerrainFluidConfig` + `RiverConfig` + `OceanConfig` + `LakeConfig` + `WaterfallConfig` (renamed `TerrainWaterfallConfig` at lib re-export) | Active (module-level) | Heightmap → water-body detection |
| `astraweave-fluids/src/turbulence.rs` | 1,593 | `TurbulenceSystem` + `MicropolarConfig` (particle spin) + `VorticityConfinementConfig` | Active (module-level) | Used by `unified_solver.rs:17` |
| `astraweave-fluids/src/underwater.rs` | 752 | `DepthZoneManager` + `UnderwaterConfig` + `UnderwaterState` + `UnderwaterUniforms` | Active (module-level) | Depth-zone fog/density transitions |
| `astraweave-fluids/src/underwater_particles.rs` | 727 | `UnderwaterParticleSystem` + `UnderwaterParticle` + `UnderwaterParticleConfig` + `BubbleStream` + `GpuUnderwaterParticle` + `UnderwaterParticleType` | Active (module-level) | Bubbles, debris, spray |
| `astraweave-fluids/src/unified_solver.rs` | 982 | `UnifiedSolver` + `UnifiedSolverConfig` + `SolverStats` + `SolverType` (lowercase variants) + `ViscositySolverType` + `FluidPhaseConfig` + `FluidType` + `QualityPreset` (Mobile/Console/PcHigh/PcUltra/Research) | Active (module-level) | High-level solver coordinator alternative to `FluidSystem` |
| `astraweave-fluids/src/validation.rs` | 996 | `MetricsHistory` (density error, divergence, energy over time) | Active (module-level) | Used by `unified_solver.rs:18` |
| `astraweave-fluids/src/viscosity.rs` | 1,333 | `NonNewtonianModel` (Carreau model for shear-thinning/thickening) | Active (module-level) | Used by `unified_solver.rs:19` |
| `astraweave-fluids/src/viscosity_gpu.rs` | 544 | GPU viscosity solver implementation | Active (module-level) | |
| `astraweave-fluids/src/volume_grid.rs` | 928 | `WaterVolumeGrid` + `WaterCell` + `MaterialType` (8 variants, with absorption rates) + `WaterGridStats` + `WaterSimConfig` + `CellFlags` | Active (module-level) | Voxel-water alternative to particle simulation |
| `astraweave-fluids/src/warm_start.rs` | 740 | Warm-starting (reuse previous pressure for faster convergence) | Active (module-level) | Used by PCISPH/DFSPH/IISPH per research.rs `supports_warm_start()` |
| `astraweave-fluids/src/water_effects.rs` | 971 | `WaterEffectsManager` + `WaterEffectsConfig` + `WaterEffectsError` + `WaterEffectsResult<T>` + `WaterEffectsStats` + `WaterQualityPreset` (Low/Medium/High/Ultra/Custom) | Active (module-level) | High-level coordinator for visual effects |
| `astraweave-fluids/src/water_reflections.rs` | 593 | `WaterReflectionSystem` + `WaterReflectionConfig` + `PlanarReflection` + `ReflectionUniforms` + `SSR_WGSL` inline-WGSL | Active (module-level) | SSR + planar reflections |
| `astraweave-fluids/src/waterfall.rs` | 1,083 | `WaterfallSystem` + `WaterfallSource` + `WaterParticle` + `WaterParticleType` + `RapidsSystem` + `GpuWaterParticle` + `WaterfallConfig` | Active (module-level) | Vertical particle rapids |
| `astraweave-fluids/shaders/anisotropic.wgsl` | 86 | Anisotropic kernel | Active | Used by `anisotropic.rs` |
| `astraweave-fluids/shaders/fluid.wgsl` | 481 | 8 PBD compute kernels: `clear_grid`, `build_grid`, `predict`, `lambda`, `delta_pos`, `integrate`, `mix_dye`, secondary spawn | Active | Loaded by `FluidSystem::new` at `lib.rs:366` via `include_str!` |
| `astraweave-fluids/shaders/sdf_gen.wgsl` | 137 | SDF generation (Jump-Flood Algorithm) | Active | Used by `sdf.rs` |
| `astraweave-fluids/shaders/secondary.wgsl` | 81 | Secondary particle (foam/spray/bubble) shader | Active | Used by `renderer.rs::secondary_pipeline` |
| `astraweave-fluids/shaders/ssfr_depth.wgsl` | 125 | SSFR depth pass | Active | Used by `renderer.rs::depth_pipeline` |
| `astraweave-fluids/shaders/ssfr_shade.wgsl` | 161 | SSFR shade pass | Active | Used by `renderer.rs::shade_pipeline` |
| `astraweave-fluids/shaders/ssfr_smooth_v2.wgsl` | 61 | SSFR bilateral-blur smoothing pass | Active | Used by `renderer.rs:65` (`include_str!("../shaders/ssfr_smooth_v2.wgsl")`). The `_v2` suffix suggests a prior `_v1` was superseded — verified 2026-05-12: workspace `find` for `ssfr_smooth*` returned only this v2 file. Git log with `--diff-filter=D` did not surface an explicit deletion of `ssfr_smooth.wgsl` in recent commits. Either v1 was renamed-in-place before commit history or was deleted earlier than the available log. |
| `astraweave-fluids/shaders/research/pcisph.wgsl` | ~27.8 KB (~1000 LoC est.) | Research-grade PCISPH GPU compute shader | Active (consumed by `pcisph_system.rs`) | Added 2026-05-12 verification pass (eighth WGSL shader, lives in `shaders/research/` subdirectory). Consumer verified 2026-05-12: `astraweave-fluids/src/pcisph_system.rs:549` (`let shader_source = include_str!("../shaders/research/pcisph.wgsl")`). |
| `astraweave-fluids/tests/mutation_resistant_comprehensive_tests.rs` | 785 | Mutation-resistant integration tests | Active | Single dedicated integration test file. Uses `use astraweave_fluids::*` at `:12` |
| `astraweave-fluids/benches/fluids_adversarial.rs` | 1,893 | Criterion adversarial benchmarks. Imports include `simd_ops::parallel::par_batch_kernel_cubic`, `simd_ops::position_to_morton`, `simd_ops::parallel::par_compute_morton_codes`, plus broader `simd_ops::*` patterns. | Active | The crate's only benchmark file |

**Status definitions:**
- **Active**: Compiled into the crate library; available to any consumer
- **Active (in demo)**: Verified used by `examples/fluids_demo/src/main.rs:18-21`
- **Active (module-level)**: Compiles + has inline tests, but no external production consumer (the crate as a whole is dormant outside the demo)

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| `FluidSystem` (original PBD GPU pipeline) | `lib.rs:250-415+` | Active (in demo) | The original solver. Demo uses this one. |
| `UnifiedSolver` (high-level coordinator) | `unified_solver.rs` (982 LoC) | Active (module-level) | Designed as high-level API combining PBD/PCISPH/DFSPH/IISPH + viscosity solvers + multi-phase + vorticity + boundary + validation. No external consumer. |
| `ResearchFluidSystem` (research-grade SPH GPU) | `research.rs` (1,190 LoC) | Active (module-level) | Research-grade with δ-SPH shifting, warm-start, micropolar SPH. No external consumer. |
| `PCISPHSystem` (dedicated PCISPH GPU) | `pcisph_system.rs` (1,620 LoC) | Active (module-level) | Standalone PCISPH implementation. No external consumer. |
| `WaterVolumeGrid` (voxel water, parallel non-particle path) | `volume_grid.rs` + `gpu_volume.rs` + `building.rs` | Active (module-level) | Voxel-based water for building/terrain interaction. Independent of particle simulators. No external consumer. |
| `WaterEffectsManager` (visual effects coordinator) | `water_effects.rs` (971 LoC) | Active (module-level) | Coordinates 7 visual subsystems behind `WaterQualityPreset`. No external consumer. |

### Naming collisions

- **`SolverType`**: In `unified_solver.rs:50-60`, has variants `Pbd / Pcisph / Dfsph / Iisph` (lowercase). In `research.rs:46-56`, has variants `PBD / PCISPH / DFSPH / IISPH` (UPPERCASE acronyms). The crate root re-exports the unified-solver version via `pub use unified_solver::{... SolverType ...}` at `lib.rs:187`. `research::SolverType` is accessible only via explicit `astraweave_fluids::research::SolverType` import.
- **`ViscositySolverType` (`unified_solver.rs:65-73`)** vs **`ViscositySolver` (`research.rs:81-89`)**: Different type names; both 3-variant `Xsph/Morris/ImplicitJacobi` (lowercase) and `XSPH/Morris/ImplicitJacobi` (UPPERCASE) respectively. Conceptually identical, name-disambiguated.
- **`QualityPreset`**: Defined in `unified_solver.rs:28-44` (Mobile/Console/PcHigh/PcUltra/Research, default Console). Also `WaterQualityPreset` (`water_effects.rs:72-97`, Low/Medium/High/Ultra/Custom). Plus `QualityTier` and `ResearchQualityTier` per `optimization.rs` and `research.rs`. **Four parallel quality enums**, each scoped to a different subsystem.
- **`WaterDrain`**: Defined in `building.rs` for voxel-grid drains. Re-exported at `lib.rs:90` as `VolumetricDrain` to disambiguate from `emitter.rs::FluidDrain` (particle-side drain).
- **`WaterfallConfig`**: Defined in both `waterfall.rs` (particle-side) and `terrain_integration.rs` (terrain-side). The terrain version is re-exported as `TerrainWaterfallConfig` at `lib.rs:179`.
- **`LodConfig`**: Editor's version (`editor.rs`) is re-exported as `EditorLodConfig` at `lib.rs:136`; runtime's version is `FluidLodConfig` from `lod.rs`.
- **`PhysicsConfig`**: Defined inside `editor.rs` for editor-side fluid physics tuning. NOT the engine-wide `astraweave_physics::PhysicsConfig`. The two crates don't depend on each other so there is no compile-time collision, but a reader scanning re-exports could conflate them.
- **`InjectionConfig` / `InjectionStrategy`** — Verified 2026-05-12: workspace grep for these names returned zero matches in `astraweave-fluids/src/editor.rs` or `astraweave-fluids/src/lib.rs`. No collision with the `astraweave-rag` types of the same name (per `docs/architecture/ai_pipeline.md` §13.8).
- **`ssfr_smooth_v2.wgsl`**: The `_v2` suffix implies a previous `ssfr_smooth.wgsl` existed and was superseded. Verified 2026-05-12: only the v2 file exists in `shaders/`; recent git log did not surface a deletion of v1. v1 may have been renamed-in-place pre-history or deleted in an unrecovered commit.

### Known cognitive traps

- **Trap**: Reading `astraweave-fluids/README.md` (which advertises "production-grade" + "world-class") and assuming the crate is wired into the engine.
  **What's actually true**: Verified 2026-05-12 workspace grep: only `examples/fluids_demo` consumes the crate. No production game-loop crate depends on it. The audit doc at `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md` documents the current state as "Overall Current Grade: B (Good for games, insufficient for research)" and lists explicit gaps.
- **Trap**: Choosing `astraweave_fluids::SolverType` and getting `Pbd / Pcisph / Dfsph / Iisph` (lowercase).
  **What's actually true**: That's the `unified_solver::SolverType` re-export at `lib.rs:187`. The `research::SolverType` has UPPERCASE acronyms (`PBD / PCISPH / DFSPH / IISPH`). To use the research version, fully-qualify: `astraweave_fluids::research::SolverType::PCISPH`.
- **Trap**: Treating `FluidSystem`, `UnifiedSolver`, `ResearchFluidSystem`, and `PCISPHSystem` as substitutable.
  **What's actually true**: They have distinct GPU types (`Particle` vs `ResearchParticle`), distinct uniform layouts (`SimParams` vs `ResearchSimParams`), and distinct compute pipelines. Switching solvers requires migrating buffers + shaders, not just swapping the surface type.
- **Trap**: Using `WaterDrain` and assuming it's particle-side.
  **What's actually true**: `WaterDrain` is in `building.rs` (voxel-grid). The lib re-export at `:90` renames it to `VolumetricDrain` precisely to disambiguate. The particle-side drain is `FluidDrain` (`emitter.rs`).
- **Trap**: Reading the SSFR shader chain (`ssfr_depth.wgsl` → `ssfr_smooth_v2.wgsl` → `ssfr_shade.wgsl`) and assuming `_v2` is the only smoothing pass.
  **What's actually true**: Only `ssfr_smooth_v2.wgsl` is present in `shaders/`. The `_v2` suffix is a naming artifact from when v1 was superseded (no v1 file currently exists in the directory). The renderer at `renderer.rs::smooth_pipeline` loads only the v2 shader.
- **Trap**: Choosing `WaterQualityPreset` for solver quality.
  **What's actually true**: `WaterQualityPreset` (`water_effects.rs:72-97`) controls visual effects (caustics/foam/god rays). For solver quality, use `QualityPreset` (`unified_solver.rs:28-44`, with `Mobile/Console/PcHigh/PcUltra/Research`). They are different enums.
- **Trap**: Assuming the `parallel` Cargo feature enables GPU parallelism.
  **What's actually true**: `parallel = ["dep:rayon"]` (`Cargo.toml:9`) enables CPU-side Rayon-parallel batch primitives in `simd_ops::parallel` namespace. GPU parallelism is unconditional — the crate is GPU-first via wgpu.
- **Trap**: Looking at `lib.rs:1` and assuming `#![forbid(unsafe_code)]` is in force (like sibling AI crates).
  **What's actually true**: The fluids crate does NOT declare `#![forbid(unsafe_code)]` at line 1. The lib.rs starts with `//! # AstraWeave Fluids` doc-comment. Verified 2026-05-12: only 2 unsafe occurrences exist crate-wide — both at `debug_viz.rs:479-480` (`unsafe impl bytemuck::Pod for DebugVertex {}` + `unsafe impl bytemuck::Zeroable for DebugVertex {}`). No `unsafe { ... }` blocks exist.

---

## 7. Decision Log

### Decision: Adopt multi-solver SPH research-grade roadmap
- **Date:** Audit doc dated January 2026 (`docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md` v2.0 "Date: January 2026")
- **Status:** In-design per audit doc; current implementation includes PBD + PCISPH + research-grade scaffolding but verified 2026-05-12 has no production game-loop consumer
- **Context:** Per audit-doc executive summary: "comprehensive audit of the AstraWeave fluids system and a detailed enhancement roadmap to achieve research-grade fluid simulation." Documents gap from "Overall Current Grade: B" to "research-grade simulation."
- **Decision:** Adopt multi-solver inventory (PBD/PCISPH/DFSPH/IISPH) with quality-tier-based selection (Mobile→PBD, Console→PCISPH, PcHigh→DFSPH, PcUltra→all features, Research→offline).
- **Alternatives considered:** Per audit doc: hybrid Eulerian-Lagrangian (Houdini FLIP/APIC, Bifrost) listed as alternative for "superior detail"; FFT/shallow-water as alternative for "large-scale water"; pure SPH as the chosen path per the doc's "Realistic Performance Targets" table.
- **Consequences:** Three parallel solver implementations coexist (`FluidSystem` PBD in lib.rs, `PCISPHSystem` in pcisph_system.rs, `ResearchFluidSystem` umbrella in research.rs) plus the `UnifiedSolver` coordinator in unified_solver.rs. Naming collisions in `SolverType` and `ViscositySolver` between unified and research modules (§6).

### Decision: Use Position-Based Dynamics (PBD) as the original GPU pipeline
- **Date:** [Reasoning not recovered — predates the research-grade roadmap document]
- **Status:** Active (the original `FluidSystem` PBD path in lib.rs is what the demo uses)
- **Context:** PBD is a non-iterative-pressure constraint-projection scheme well-suited to GPU compute (fast convergence, visual fidelity).
- **Decision:** Implement PBD in `shaders/fluid.wgsl` (8 compute kernels: clear_grid, build_grid, predict, lambda, delta_pos, integrate, mix_dye, secondary).
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** Demo runs at the PBD quality tier (B-grade per audit). Research-grade solvers exist in parallel but are not the default consumer path.

### Decision: Eight-stage compute pipeline + ping-pong buffers + grid-linked-list broadphase
- **Date:** [Reasoning not recovered]
- **Status:** Active
- **Context:** PBD requires per-iteration neighbor search. GPU-friendly options include grid-cell linked lists (head + next pointers) or sorted spatial hash.
- **Decision:** Use head-pointer / next-pointer linked list per cell (`head_pointers + next_pointers` buffers per `lib.rs:270-272`) populated by `clear_grid` + `build_grid` compute passes at the start of every step.
- **Alternatives considered:** Sorted spatial hash (Morton code), bitonic sort — `MortonCode` type exists in `optimization.rs` and `position_to_morton` in `simd_ops.rs`, suggesting Morton-coding was prototyped, possibly for the research-grade path.
- **Consequences:** PBD path uses linked-list; research path may use Morton. Both code paths exist in the crate.

### Decision: GPU vendor-aware workgroup sizing
- **Date:** [Reasoning not recovered]
- **Status:** Active (in optimization module)
- **Context:** NVIDIA warp size = 32, AMD wave size = 64 (Wave64), Intel subgroup 8-32, Apple Silicon TBD. Optimal workgroup size depends on vendor.
- **Decision:** `WorkgroupConfig::auto_detect()` selects from `nvidia()` / `amd()` / `apple()` / `universal()` (64 threads default) per `optimization.rs:80-…` based on `GpuVendor` detection.
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** Workgroup tuning is automatic when the controller is wired in. The demo passes through `FluidOptimizationController`.

### Decision: Adaptive iteration count based on density error
- **Date:** [Reasoning not recovered]
- **Status:** Active (in optimization module)
- **Context:** PBD/PCISPH iteration count is a quality-vs-cost knob. Static count over-iterates in stable scenes and under-iterates in turbulent ones.
- **Decision:** `AdaptiveIterations::new(min, max)` reads `density_error_buffer` feedback and adjusts iteration count per frame. Default range surfaces per docs at `optimization.rs:1-44` example: `AdaptiveIterations::new(2, 8)`.
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** Frame-to-frame iteration count varies. Convergence metrics are async-readback via density_error staging buffers.

### Decision: Voxel water grid as a parallel system to particle simulation
- **Date:** [Reasoning not recovered — Enshrouded "Wake of Water" inspiration cited inline at `volume_grid.rs:5`]
- **Status:** Active (module-level)
- **Context:** Per inline doc-comment at `volume_grid.rs:1-6`: "Implements voxel-based water simulation for building/terrain interaction. Inspired by Enshrouded's 'Wake of Water' update with hydrostatic pressure, material absorption, and U-bend flow physics." Particle simulation is poorly suited to large-volume building-flooding scenarios.
- **Decision:** Implement `WaterVolumeGrid` with per-cell water level + material-dependent absorption + hydrostatic pressure as a distinct path from the particle simulator.
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** Two simulation paradigms coexist (particle for splashes/jets, voxel for building flood). Building integration (`WaterBuildingManager`) is voxel-only.

### Decision: `WaterEffectsManager` as visual-effects coordinator behind quality preset
- **Date:** [Reasoning not recovered]
- **Status:** Active (module-level)
- **Context:** Seven visual subsystems (caustics, foam, god rays, reflections, underwater, underwater particles, waterfall) have overlapping resource needs and life cycles.
- **Decision:** `WaterEffectsManager::from_preset(WaterQualityPreset)` instantiates a coordinated subset per quality tier (Low/Medium/High/Ultra/Custom).
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** Visual-effects API has a single entry point. Low-level subsystem types remain publicly accessible (re-exported at `lib.rs:88-203`) for fine-grained control.

### Decision: SIMD primitives via glam auto-vectorization (not manual intrinsics)
- **Date:** [Reasoning not recovered — design choice documented inline]
- **Status:** Active
- **Context:** Per `simd_ops.rs:19-25` doc-comment "Best Practices (2025)": "Prefer simple iterators over manual unrolling - LLVM auto-vectorizes better; Avoid manual FMA (mul_add) - creates artificial dependencies."
- **Decision:** Use `glam::Vec3` operations inside batch iterators rather than `#[target_feature(enable = "avx2")]` + `_mm256_*` intrinsics. Document the rationale inline.
- **Alternatives considered:** Manual AVX2 intrinsics (rejected per inline comment); WGSL-only solver (rejected — keeps CPU batch helpers for editor/preview/non-GPU code).
- **Consequences:** `simd_ops.rs` is 39,554 LoC of safe Rust batch operations. No `unsafe` SIMD intrinsics. Performance depends on LLVM auto-vectorization quality.

### Decision: Refactor `ssfr_smooth.wgsl` → `ssfr_smooth_v2.wgsl` ("shader permutation system")
- **Date:** Recovered via git log 2026-05-12 — commit `4af95b47c` "Implement rain splash particle system, shader permutation system, snow footprint stamping, and vegetation interaction system" deletes `astraweave-fluids/shaders/ssfr_smooth.wgsl` (confirmed via `git log --diff-filter=D -- astraweave-fluids/shaders/ssfr_smooth.wgsl`).
- **Status:** Accepted (v1 deleted; v2 active)
- **Context:** The commit message references a "shader permutation system" as one of four landed features. The deletion of v1 alongside that work implies v2 was introduced as part of the shader-permutation refactor.
- **Decision:** Replace v1 with v2 as part of broader shader-permutation infrastructure landing.
- **Alternatives considered:** [Reasoning not recovered — commit message documents the what but not the why]
- **Consequences:** Only `ssfr_smooth_v2.wgsl` exists in `shaders/` today. `renderer.rs:65` loads v2 directly. No v1 fallback.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `Particle` is 80 bytes, `bytemuck::Pod + Zeroable`, `#[repr(C)]` | Yes (compile-time) | `lib.rs:208-219` derive + repr attributes |
| 2 | `SimParams` is 64 bytes (12 f32 fields + 4 padding f32 + u32 fields) | Yes (compile-time) | `lib.rs:221-248` |
| 3 | `SecondaryParticle` is 48 bytes | Yes (compile-time) | `lib.rs:354-360` |
| 4 | `GpuWaterCell` is 16 bytes | Yes (compile-time) | `gpu_volume.rs:15-27` |
| 5 | `WaterSurfaceVertex` is 32 bytes (position[3]=12 + normal[3]=12 + uv[2]=8) | Yes (compile-time + `wgpu::VertexBufferLayout`) | `gpu_volume.rs:41-80` |
| 6 | `CameraUniform` is 304 bytes | Yes (compile-time, verified 2026-05-12) | `renderer.rs:7-16`: 3× `[[f32; 4]; 4]` mat4 = 192 bytes (view_proj + inv_view_proj + view_inv) + 2× `[f32; 4]` vec4 = 32 bytes (cam_pos + light_dir) + `f32` = 4 bytes (time) + `[f32; 19]` padding = 76 bytes. Total = 192+32+4+76 = 304 bytes. The previous trace claim "200 bytes" was incorrect. |
| 7 | `MaterialType::Air` is 0 (`#[repr(u8)]`) | Yes (compile-time) | `volume_grid.rs:14-18` |
| 8 | `MaterialType::absorption_rate` for `Air`/`Stone`/`Glass` is 0.0 | Yes (code) | `volume_grid.rs:38-49` |
| 9 | `MaterialType::blocks_flow` is true only for `Stone` and `Glass` | Yes (code) | `volume_grid.rs:53-55` |
| 10 | `PCISPHSystem::MAX_PARTICLES = 1_000_000` | Yes (code) | `pcisph_system.rs:27` |
| 11 | `PCISPHSystem::MAX_PCISPH_ITERATIONS = 50` | Yes (code) | `pcisph_system.rs:30` |
| 12 | `PCISPHSystem::DEFAULT_DENSITY_THRESHOLD = 0.001` (0.1% target) | Yes (code) | `pcisph_system.rs:33` |
| 13 | `PCISPHSystem::WORKGROUP_SIZE = 64` (matches `WorkgroupConfig::universal()`) | Yes (code) | `pcisph_system.rs:36` |
| 14 | `SIMD_BATCH_SIZE = 8` | Yes (code) | `simd_ops.rs:34` |
| 15 | `SolverType::PBD::typical_iterations() == 4`, `PCISPH == 5`, `DFSPH == 3`, `IISPH == 15` | Yes (code) | `research.rs:60-68` |
| 16 | `SolverType::supports_warm_start()` returns true only for PCISPH / DFSPH / IISPH (NOT PBD) | Yes (code) | `research.rs:71-74` |
| 17 | `ViscositySolver::supports_high_viscosity()` returns true only for `ImplicitJacobi` | Yes (code) | `research.rs:93-96` |
| 18 | `step_internal` does NOT exist here — fluids step is GPU-driven via compute pipelines, not a single host-side method | Yes (file inspection) | `lib.rs:1039-` is physics-crate territory, not fluids |
| 19 | All major user-facing enums are `#[non_exhaustive]`: `MaterialType`, `FlowDirection`, `WaterBodyType`, `WaterEffectsError`, `WaterQualityPreset`, `SolverType` (both versions), `ViscositySolverType` (and `ViscositySolver`), `QualityPreset` (multiple), `GpuVendor`, `ProjectileKind`-equivalent enums | Yes (compile-time) | Various file:line pairs documented in §3 |
| 20 | `WaterEffectsError` is `#[non_exhaustive] #[must_use]` | Yes (compile-time) | `water_effects.rs:18-22` |
| 21 | **(REWRITTEN F.1)** `FluidSystem` owns exactly ONE particle buffer; all kernels mutate it in place; `get_particle_buffer` always returns current state | Yes (compile-time + `gpu_visible_state_advances_every_frame` regression test) | The v1.2 "2-entry ping-pong" invariant described a defect: buffer 1 was created empty and written by no kernel, so alternating bind groups simulated two divergent half-rate states (F.0 Must-Fix #1, fixed F.1) |
| 22 | **(REWRITTEN F.1)** `particle_flags` is bound at group 1 binding 1 (read-only) and every per-particle kernel early-outs on flag==0; `build_grid` never inserts inactive particles | Yes (`gpu_despawn_removes_particles_from_simulation` test) | Pre-F.1 the flags buffer was host-written but bound to nothing (despawn was GPU-invisible, F.0 Must-Fix #2) |
| 23 | `density_error_staging_buffers` has exactly 2 entries; `map_async` is issued ONLY for a buffer whose copy was already submitted (`StagingState` machine; two-frame-lag adaptive iterations are the defined semantics) | Yes (compile-time + state machine) | F.1 replaced the pre-submit `map_async` (F.0 Must-Fix #3) |
| 24 | Crate does NOT declare `#![forbid(unsafe_code)]` | Yes (file inspection) | `lib.rs:1` is `//! # AstraWeave Fluids` doc-comment, not the forbid attribute |

---

## 9. Performance & Resource Profile

### Hot paths

| Path | Frequency | Budget | Sensitivity |
|---|---|---|---|
| `FluidSystem` 8-stage compute pass | Per frame (60 Hz default) | Per `SimulationBudget` — frame-time target configurable | Particle count, iteration count, grid cell count, GPU vendor |
| `lambda` + `delta_pos` PBD iteration | Per iteration × N iterations × per frame | Each iteration dispatches over all particles | Density error feedback drives iteration count via `AdaptiveIterations` |
| `FluidRenderer` SSFR pipeline | Per frame | Per render-target resolution | Smoothing radius, blur passes, particle screen-space coverage |
| `WaterVolumeGrid` per-cell flow | Per voxel update tick | CPU-bound (separate from GPU particle path) | Grid dimensions (W×H×D), active-cell count |
| `simd_ops::batch_distances` + `batch_kernel_cubic` | Per neighbor evaluation × per particle × per iteration | Inner loop hot path | Neighbor count, kernel evaluation count |

Per README:
- Realistic targets (from audit doc `FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`):
  - Low (PBD): 50-100k particles @ 60+ fps (mobile, background)
  - Medium (PCISPH): 100-200k particles @ 60 fps (standard gameplay)
  - High (DFSPH): 200-500k particles @ 30-60 fps (hero fluids, AAA)
  - Research (DFSPH+Implicit): 100-300k particles @ 15-30 fps (offline)
  - Hybrid (PBD+heightfield): 500k-1M particles @ 30-60 fps (large-scale water)
- Per inline `pcisph_system.rs:12-15` performance targets: "100-200k particles @ 60 FPS (Medium quality tier); <0.1% density error after convergence; 3-8 pressure correction iterations typical."

### Cold paths

| Path | Frequency | Budget |
|---|---|---|
| `FluidSystem::new` | Once per simulation domain | Per-frame budget not a concern; allocates all GPU buffers + pipelines |
| `WaterBuildingManager` add/remove dispensers/drains/gates/wheels | At entity spawn/destroy | One-off mutations |
| `analyze_terrain_for_water` | At terrain regeneration | Heightmap-size-dependent; expected to run off the hot path |
| Editor undo/redo (`ConfigHistory::push/undo/redo`) | At user interaction | UI-driven |
| `FluidSnapshot` save/load | At save/load events | Disk I/O bound |

### Resource ownership

| Resource | Owner | Lifetime | Access pattern |
|---|---|---|---|
| `wgpu::Buffer` (particle_buffers, head_pointers, next_pointers, params_buffer, etc.) | `FluidSystem` | `FluidSystem` lifetime | Mutated by compute passes; read by render passes |
| `wgpu::ComputePipeline` (8 PBD pipelines) | `FluidSystem` | `FluidSystem` lifetime | Read-only after construction |
| `wgpu::BindGroup` + `wgpu::BindGroupLayout` (4 + cache) | `FluidSystem` | `FluidSystem` lifetime | Bound per compute pass |
| `density_error_staging_buffers: [wgpu::Buffer; 2]` | `FluidSystem` | `FluidSystem` lifetime | Ping-pong async readback for CPU-side convergence monitoring |
| `staging_mapped: [bool; 2]` | `FluidSystem` | `FluidSystem` lifetime | Tracks mapped state of staging buffers |
| `SdfSystem` | `FluidSystem` (pub field at `lib.rs:301`) | `FluidSystem` lifetime | Bound to compute passes as boundary representation |
| `objects_buffer: wgpu::Buffer` | `FluidSystem` | `FluidSystem` lifetime | `DynamicObject[]` for moving collider geometry |
| `default_sampler: wgpu::Sampler` | `FluidSystem` | `FluidSystem` lifetime | Texture sampling |
| `WaterVolumeGrid` cells | `WaterVolumeGrid` | Caller-managed | Voxel state |
| Render textures (`depth_texture`, `smoothed_depth_texture`) | `FluidRenderer` | `FluidRenderer` lifetime | Recreated on viewport resize |

### Allocation profile

[NEEDS VERIFICATION — no per-step allocation audit found in fluids docs. The audit doc focuses on solver performance, not allocation counts. The bench `fluids_adversarial.rs` (1,893 LoC) may include allocation measurements but was not surveyed in this pass.]

---

## 10. Testing & Validation

- **Unit tests:** Inline `#[cfg(test)]` modules in each source file. Verified 2026-05-12 via `grep -rcn "#\[test\]"`: 600+ inline tests across 20+ source files. Per-file breakdown (top counts): `editor.rs:140`, `lib.rs:79`, `optimization.rs:78`, `multi_phase.rs:61`, `lod.rs:49`, `gpu_volume.rs:47`, `emitter.rs:41`, `pcisph_system.rs:37`, `research.rs:35`, `boundary.rs:34`, `building.rs:33`, `foam.rs:26`, `sdf.rs:25`, `caustics.rs:24`, `profiling.rs:24`, `serialization.rs:19`, `anisotropic.rs:18`, `debug_viz.rs:19`, `god_rays.rs:16`, `particle_shifting.rs:11`.
- **Integration tests:** Single file `astraweave-fluids/tests/mutation_resistant_comprehensive_tests.rs` (785 LoC). Per CLAUDE.md mutation-testing campaign convention, this is the dedicated mutation-resistance test surface. Imports via `use astraweave_fluids::*` at `:12`.
- **Mutation testing:** Tracked in `docs/current/FLUIDS_MUTATION_TESTING_REPORT.md` (status out of scope for this trace).
- **Miri validation:** Not formally tracked here. The crate does NOT declare `#![forbid(unsafe_code)]`, but verified 2026-05-12: only 2 unsafe occurrences exist crate-wide — both at `debug_viz.rs:479-480` (bytemuck Pod/Zeroable impls on `DebugVertex`). No `unsafe { ... }` blocks. Miri-relevant surface is therefore limited to bytemuck trait safety.
- **Benchmarks:** Single file `astraweave-fluids/benches/fluids_adversarial.rs` (1,893 LoC). Imports include `simd_ops::parallel::par_batch_kernel_cubic`, `simd_ops::position_to_morton`, `simd_ops::parallel::par_compute_morton_codes`, plus broader `simd_ops::*` patterns (per grep at `:1749, :1774, :1785, :1810`). Configured as `[[bench]] name = "fluids_adversarial"` at `Cargo.toml:11-13`.
- **Manual validation:** `examples/fluids_demo` is the sole interactive validation harness. Imports `FluidSystem`, `FluidRenderer`, `FluidLodConfig`, `FluidLodManager`, `FluidOptimizationController` per `fluids_demo/src/main.rs:18-21`. Uses `astraweave_fluids::renderer::CameraUniform` at `:18`.
- **Audit document:** `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md` v2.0 (January 2026) — comprehensive audit + roadmap targeting research-grade simulation. Cited grade: "B (Good for games, insufficient for research)."

---

## 11. Open Questions / Parked Decisions

> **F.1 closures (2026-06-11):** "Runtime production wiring — when and via which solver?" → DECIDED at the F.0 owner gate: **Path B (layered facade)**; the campaign plan governs. "Five parallel solver/manager surfaces — consolidation?" → RESOLVED per gate Q3: `UnifiedSolver` deleted, `PcisphSystem` + 5 modules gated `experimental`, `FluidSystem` is the canonical particle solver, `WaterVolumeGrid` the canonical voxel layer. "`SolverType` naming collision" → RESOLVED: the unified_solver enum was deleted with its module; only `research::SolverType` (PBD/PCISPH) remains. "`ViscositySolverType` vs `ViscositySolver`" → RESOLVED: only `research::ViscositySolver` remains. The remaining questions below are still open.

- **Runtime production wiring of fluids — when and via which solver?** [Decisional / **HIGH-IMPACT finding from 2026-05-12 trace investigation**.] Factual state (verified 2026-05-12): workspace grep for `use astraweave_fluids` outside the fluids crate itself returned only `examples/fluids_demo/src/main.rs:18-21`. NO production game-loop crate (`astraweave-render`, `astraweave-gameplay`, `astraweave-physics`, `astraweave-scene`, `astraweave-terrain`, `astraweave-ecs`) depends on `astraweave-fluids`. The crate is 84.5K LoC of working code (1 integration test passing, 1 benchmark, 1 demo) with zero engine integration. Per the audit doc `FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`, the system is in active development toward research-grade simulation. Three directional options: (a) wire the existing PBD `FluidSystem` (the demo's current choice) into the runtime engine — smallest integration step; (b) wait for the research-grade solvers to mature (PCISPH/DFSPH/IISPH per the roadmap) and wire the `UnifiedSolver` umbrella; (c) prune the parallel solvers and keep only the demo-validated PBD path. Same dormancy shape as the LLM Production Hardening and RAG subsystems traced in `docs/architecture/ai_pipeline.md` §13.7 + §13.8.
- **Five parallel solver/manager surfaces — consolidation roadmap?** [Decisional.] Factual: `FluidSystem` (lib.rs PBD), `UnifiedSolver` (unified_solver.rs coordinator), `ResearchFluidSystem` (research.rs research-grade umbrella), `PCISPHSystem` (pcisph_system.rs standalone PCISPH), `WaterEffectsManager` (water_effects.rs visual coordinator) coexist with overlapping responsibilities. The crate is 84.5K LoC. Whether to consolidate into a single `Fluid` facade, keep all five as separately-published modules, or migrate consumers to `UnifiedSolver` exclusively is undecided.
- **`SolverType` naming collision between `unified_solver.rs` (lowercase) and `research.rs` (UPPERCASE) — rename or coexist?** [Decisional / factual.] Two enums of the same name with different variant casing conventions. The crate root re-exports the lowercase version (`lib.rs:187`). Whether to rename the research version (e.g. `ResearchSolverType`) or accept the namespacing requirement is undecided.
- **`ViscositySolverType` (unified) vs `ViscositySolver` (research) — pick a single name?** [Decisional.] Factual: two parallel types with same conceptual purpose but different names. Standardize on one or keep both.
- **Four parallel quality enums (`QualityPreset`, `WaterQualityPreset`, `QualityTier`, `ResearchQualityTier`) — unify or document parallel intent?** [Decisional.] Each scopes to a different subsystem. Whether to share a single base enum + per-subsystem extension trait, keep four parallel enums, or accept the redundancy is undecided.
<!-- Question "ssfr_smooth_v2.wgsl suffix — vestige of v1 deletion?" closed via deep investigation 2026-05-12. Resolution: v1 (`ssfr_smooth.wgsl`) DID exist and was deleted in commit `4af95b47c` ("Implement rain splash particle system, shader permutation system, snow footprint stamping, and vegetation interaction system"). The commit's "shader permutation system" likely refactored ssfr_smooth as v2. Resolution captured in §5 file map row for `ssfr_smooth_v2.wgsl` and §7 Decision Log (new entry for SSFR shader refactor). -->
- **`#![forbid(unsafe_code)]` not declared — intentional or oversight?** [Decisional / factual, **enriched 2026-05-12**.] Sibling engine crates (`astraweave-physics`, `astraweave-llm`, `astraweave-behavior`, `astraweave-memory`, etc.) declare `#![forbid(unsafe_code)]`. `astraweave-fluids/src/lib.rs:1` does NOT. Verified 2026-05-12: only 2 unsafe occurrences crate-wide — `unsafe impl bytemuck::Pod for DebugVertex {}` + `unsafe impl bytemuck::Zeroable for DebugVertex {}` at `debug_viz.rs:479-480`. No `unsafe { ... }` blocks. Adding `#![forbid(unsafe_code)]` would require replacing these two trait impls with `#[derive(bytemuck::Pod, bytemuck::Zeroable)]` (which performs the same safety check via macro) or moving `DebugVertex` to its own module with `#![allow(unsafe_code)]`. Whether the absence is intentional or an oversight is undecided.
- **`add_water_aabb` stub equivalent in fluids?** [Factual — investigable.] In physics (`astraweave-physics/src/lib.rs:1449`), `add_water_aabb` is a no-op stub. The fluids crate provides actual volumetric water via `WaterVolumeGrid` + `gpu_volume::WaterVolumeGpu`. Whether the physics stub should be replaced by a fluids-integration shim is decisional + cross-crate.
- **Per-step GPU allocation count growth over simulation time?** [Factual / observable, **empirical**.] The crate uses extensive ping-pong buffer patterns. Whether per-step allocation grows over time vs is bounded is empirically observable through the `fluids_adversarial.rs` bench. [NEEDS VERIFICATION — bench results not surveyed in this pass.]
<!-- Question "Editor surface (5,823 LoC editor.rs) — wired to actual editor or future-design?" closed via verification + deep investigation 2026-05-12. Resolution: NOT wired into `tools/aw_editor` today. Workspace grep for `use astraweave_fluids` or `astraweave_fluids::` inside `tools/aw_editor` returned zero matches; `tools/aw_editor/Cargo.toml` does not declare `astraweave-fluids` as a dependency. The fluids editor surface is forward-design infrastructure. Resolution captured in §5 file map editor.rs row + §10 Manual validation note. -->
- **Audit doc grade target ("Overall Current Grade: B") — when will the roadmap reach grade A?** [Decisional / timeline.] The audit doc dated January 2026 lists per-subsystem gaps. The doc is a roadmap, not a commitment. Realistic target dates for each gap closure are not stated in the audit doc.

---

## 11.5 Revision History

| Version | Date | Change |
|---|---|---|
| 1.0–1.2 | 2026-05-12 | Initial trace + verification + deep-investigation passes |
| 1.3 | 2026-06-11 | **F.1 revision** (§0): F.0 audit corrections (phantom `ResearchFluidSystem`, 9th orphan shader, ping-pong-defect invariants) + F.1 code deltas (FluidSystem repair, 5 SDF fixes, UnifiedSolver deletion, DFSPH/IISPH variant removal, `experimental` feature, serde unconditional, validation honesty, first GPU tests + baselines) + determinism carve-out policy. §8 invariants 21–23 rewritten; §11 closures. Body sections older than §0 should be read through the §0 corrections; a full re-verification pass is queued post-campaign. |

## 12. Maintenance Notes

**Update this doc when:**
- A new solver is added to the parallel inventory (`FluidSystem`/`UnifiedSolver`/`ResearchFluidSystem`/`PCISPHSystem`) — touch §1, §3, §5, §6, §11
- A new variant is added to any `#[non_exhaustive]` enum (`MaterialType`, `WaterBodyType`, `SolverType` (either version), `WaterQualityPreset`, `WaterEffectsError`, `GpuVendor`, etc.) — touch §3, §8
- A new visual-effects subsystem is added under `WaterEffectsManager` — touch §3, §4 Bidirectional, §5
- A new compute kernel is added to `shaders/fluid.wgsl` — touch §2.1 pipeline diagram, §8 invariants if buffer layout changes
- A new WGSL shader file is added — touch §5 shader-files section
- The `examples/fluids_demo` consumer pattern changes (e.g. switches from `FluidSystem` to `UnifiedSolver`) — touch §1 status, §4 Downstream, §11 production-wiring question
- A production game-loop crate begins to depend on `astraweave-fluids` — touch §1 status note (dormancy), §4 Downstream, §11 first Open Question
- The `parallel` or `serde` Cargo feature gating changes — touch §1 status, §5 file map
- A new audit doc supersedes `FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md` — touch §7 Decision Log first entry, §11 first Open Question

**Verification process:**
- Spot-check §2.1 PBD pipeline against `shaders/fluid.wgsl` kernel names + `lib.rs::FluidSystem::new` pipeline construction
- Verify §5 file map line counts against `wc -l astraweave-fluids/src/*.rs astraweave-fluids/shaders/*.wgsl`
- Verify §6 naming-collision claims against `grep -n "pub enum SolverType" astraweave-fluids/src/{unified_solver,research}.rs`
- Verify §11 first Open Question with `grep -rn "use astraweave_fluids" --include="*.rs"` workspace-wide
- Run `cargo test -p astraweave-fluids --tests` for integration test surface
- Run `cargo bench -p astraweave-fluids` for bench surface
- Update metadata commit hash + date

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**

1. **The crate is dormant in production.** No engine game-loop crate consumes `astraweave-fluids`. Only `examples/fluids_demo` does (verified 2026-05-12).
2. **Five parallel solver/manager surfaces coexist.** `FluidSystem` (lib.rs PBD), `UnifiedSolver`, `ResearchFluidSystem`, `PCISPHSystem`, `WaterEffectsManager`. The demo uses `FluidSystem`. The others are research-grade additions per the roadmap.
3. **`SolverType` naming collision:** `unified_solver::SolverType` has `Pbd/Pcisph/Dfsph/Iisph` (lowercase). `research::SolverType` has `PBD/PCISPH/DFSPH/IISPH` (UPPERCASE). The crate root re-exports the lowercase version.
4. **Quality enums proliferate:** `QualityPreset`, `WaterQualityPreset`, `QualityTier`, `ResearchQualityTier` — each scopes to a different subsystem. They are NOT interchangeable.
5. **GPU-first architecture.** `FluidSystem::new` takes `&wgpu::Device`. Compute pipelines are constructed once; per-frame work is a single compute pass over 8 kernels.
6. **Two paradigms coexist:** particle simulation (PBD/PCISPH/etc.) for splashes/jets/hero fluids; voxel grid (`WaterVolumeGrid`) for building flooding / terrain interaction.
7. **`#![forbid(unsafe_code)]` is NOT declared at lib.rs:1.** Sibling engine crates do declare it. Verified 2026-05-12: only 2 unsafe occurrences crate-wide — bytemuck Pod/Zeroable impls at `debug_viz.rs:479-480`. No `unsafe { ... }` blocks.
8. **`simd_ops.rs` is the largest file at 39,554 LoC.** It is batch-operation surface designed for LLVM auto-vectorization via glam — NOT manual SIMD intrinsics. Per inline doc-comment, manual unrolling and manual FMA are explicitly avoided.
9. **`editor.rs` is the second-largest file at 5,823 LoC.** Comprehensive editor integration with undo/redo, validation, batch operations, color-blind palettes. Verified 2026-05-12: NOT consumed by `tools/aw_editor` — forward-design only.
10. **WGSL shaders live in `astraweave-fluids/shaders/`** (7 files). `shaders/fluid.wgsl` (481 LoC) is the central compute shader with 8 kernels.
11. **`WaterEffectsManager` is the visual-effects coordinator** behind `WaterQualityPreset`. The 7 visual subsystems (caustics/foam/god rays/reflections/underwater/underwater_particles/waterfall) are independently constructable but typically used via this coordinator.
12. **Two Cargo features only:** `default = ["serde"]`, `parallel = ["dep:rayon"]`. Neither is required for the GPU compute paths.

**Files you'll most likely touch:**

- `astraweave-fluids/src/lib.rs` — `FluidSystem` (PBD GPU pipeline), `Particle`/`SimParams`/`SecondaryParticle` GPU types
- `astraweave-fluids/src/water_effects.rs` — `WaterEffectsManager` visual coordinator
- `astraweave-fluids/src/volume_grid.rs` + `building.rs` — voxel water + building integration
- `astraweave-fluids/shaders/fluid.wgsl` — 8-kernel PBD compute shader
- `astraweave-fluids/src/renderer.rs` — SSFR rendering pipeline
- `examples/fluids_demo/src/main.rs` — the only consumer; the canonical usage pattern

**Files you should NOT touch without strong reason:**

- `astraweave-fluids/src/simd_ops.rs` (39,554 LoC) — the SIMD primitives layer; per inline doc-comment, the design choice is "iterators not intrinsics" and the file optimizes for LLVM auto-vectorization, not manual SIMD
- `astraweave-fluids/src/research.rs` + `pcisph_system.rs` + `unified_solver.rs` — research-grade solver inventory; coordinated by the roadmap doc and dormant in production
- `astraweave-fluids/src/editor.rs` (5,823 LoC) — comprehensive editor surface with extensive re-exports; modifications cascade through `lib.rs:100-153` re-exports
- `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md` — audit doc; this trace is anchored to it

**Common mistakes when changing this system:**

- **Assuming `astraweave-fluids` is wired into the engine.** It is not. The crate is dormant outside `examples/fluids_demo`.
- **Choosing `SolverType::PBD` (UPPERCASE) and expecting it to work from a `unified_solver` consumer.** The unified solver's enum is lowercase (`Pbd`). The UPPERCASE version is from `research::SolverType`. Fully-qualify imports.
- **Treating `FluidSystem` and `UnifiedSolver` as interchangeable.** They have different GPU types (`Particle` vs `ResearchParticle`) and different compute pipelines. Switching is a migration, not a swap.
- **Modifying `simd_ops.rs` to add manual SIMD intrinsics.** Per the file's inline doc-comment (lines 19-25), the design choice is explicitly to AVOID manual intrinsics and FMA in favor of LLVM auto-vectorization.
- **Adding a new variant to `#[non_exhaustive]` enums without checking dispatch sites.** Many enums (`MaterialType`, `SolverType`, `WaterEffectsError`, etc.) feed into match expressions in the `editor.rs` validation surface. Per CLAUDE.md Integration Completeness #2, audit all sites.
- **Calling `WaterDrain` and expecting particle behavior.** `WaterDrain` is voxel-side (aliased `VolumetricDrain` at re-export). Particle-side drains use `FluidDrain`.
- **Updating `shaders/ssfr_smooth_v2.wgsl` thinking v1 exists somewhere as a fallback.** Only v2 exists in the directory.

---

## Appendix B: Historical context

The fluids crate has gone through visible development phases captured in `docs/`:

1. **Initial PBD GPU implementation** (`FluidSystem` in `lib.rs`, `shaders/fluid.wgsl`) — the original solver. The crate's README and the demo consumer pattern center on this path.

2. **Research-grade enhancement** (`research.rs`, `pcisph_system.rs`, `unified_solver.rs`, `boundary.rs`, `viscosity.rs`, `turbulence.rs`, `warm_start.rs`, `particle_shifting.rs`, `validation.rs`) — added per the audit doc `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md` v2.0 (January 2026). The audit doc references external review by "Grok 4" and 2024-2026 SPH research. Cites SPlisHSPlasH, Taichi, Houdini FLIP/APIC, UE5 Water, Niagara as comparators.

3. **Voxel water grid** (`volume_grid.rs`, `gpu_volume.rs`, `building.rs`) — inspired by Enshrouded's "Wake of Water" per inline `volume_grid.rs:5` doc-comment. Adds the second simulation paradigm (voxel) alongside the original particle path.

4. **Visual effects layer** (`caustics.rs`, `foam.rs`, `god_rays.rs`, `water_reflections.rs`, `underwater.rs`, `underwater_particles.rs`, `waterfall.rs`, `water_effects.rs`) — coordinated by `WaterEffectsManager`. Inline-WGSL constants (`CAUSTICS_WGSL`, `GOD_RAYS_WGSL`, `SSR_WGSL`) suggest external shader-composition use cases.

5. **Optimization + LOD + profiling** (`optimization.rs`, `lod.rs`, `profiling.rs`, `simd_ops.rs`) — production-grade tuning surface. GPU-vendor-aware workgroup sizing, adaptive iteration, simulation budget, batch spawning.

6. **Editor integration** (`editor.rs` at 5,823 LoC) — the largest non-SIMD file. Suggests significant authoring-tool integration effort.

Status as of 2026-05-12: the crate is 84.5K LoC of working code with comprehensive tests and benchmarks, but is dormant in production — only `examples/fluids_demo` consumes it. The audit-doc grade is "B (Good for games, insufficient for research)." Per CLAUDE.md Integration Completeness rule, the crate qualifies as designed-but-not-wired infrastructure pending a production integration campaign.
