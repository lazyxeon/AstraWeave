# Local Simulation Archaeology

**Status:** Phase 1 map. No runtime code in this change.
**Base:** `main` `85d09a2c` (2026-10-15).
**Companion:** [`LOCAL_SIM_DESIGN.md`](LOCAL_SIM_DESIGN.md).

Labels:

| Label | Meaning |
|---|---|
| REAL | Implemented, and a non-test caller outside examples drives it. |
| DORMANT | Implemented, and the only callers are tests, benches, or examples. |
| STUBBED | The type or function exists and the body does not do the work the name claims. |
| UNTESTED | Implemented behavior with no test that exercises it. |
| ABSENT | No type and no function. |

A row can carry a second note when the primary label is not the whole story. GPU particle state is presentation. The fluids determinism carve-out in `astraweave-water/src/lib.rs` (lines 10–22) and `astraweave-fluids/src/lib.rs` (lines 1160–1167) is binding: particle positions are excluded from replay, `world_hash`, and net replication.

---

## 1. The split that already exists

Gameplay water truth and fluid rendering are different systems.

| Question | Owner | Where |
|---|---|---|
| Is there water here, how high, how dense? | `WaterQuery` / `AnalyticWater` | `astraweave-water/src/lib.rs` |
| Does a rigid body float? | `PhysicsWorld::apply_buoyancy_forces` | `astraweave-physics/src/lib.rs` 1438–1488 |
| What does the ocean look like? | `WaterRenderer` Gerstner heightfield | `astraweave-render/src/water.rs`, `shaders/water.wgsl` |
| What do splash particles look like? | `FluidSystem` GPU PBD + SSFR | `astraweave-fluids` |

`WaterQuery::sample` returns `surface_height` and `density` only (`astraweave-water/src/lib.rs` 48–71). Flow, drag, temperature, and a submersion fraction are absent from the trait. The crate comment says a future backend can implement the same trait. The voxel backend that once did that was removed in W.1 with `volume_grid.rs`.

---

## 2. `astraweave-fluids`

The crate is the F.4 GPU accent substrate plus a deferred effects and editor layer. `docs/architecture/fluids.md` §0.5 records the W.1 deletion of the SPH inventory, the voxel water sim, and `simd_ops.rs`. Section 1 of that trace still describes those deleted solvers. That section is stale. This map uses the tree.

No game-loop crate depends on `astraweave-fluids`. Callers are `examples/fluids_demo`, `examples/weaving_playground`, and `examples/weaving_playground/examples/accent_budget_probe.rs`. `tools/aw_editor` does not `use` the crate.

`Cargo.toml` feature `parallel` pulls optional `rayon` and gates no code. The comment in that file says W.1 orphaned it.

### 2.1 GPU PBD solver — DORMANT

| Piece | Path | Label | Note |
|---|---|---|---|
| `Particle`, `SimParams`, `FluidSystem` | `astraweave-fluids/src/lib.rs` 168–319 | DORMANT | GPU position-based fluids. `Particle.temperature` is an `f32` kelvin field (line 177). |
| `FluidSystem::step` | `lib.rs` 1168 | DORMANT | Records the compute chain into a `CommandEncoder`. The caller must submit before the next `step`. |
| Adaptive iteration count | `lib.rs` 1160–1167 | DORMANT | `iterations` for frame N comes from the density error harvested about two frames earlier. The comment states the particle state is non-deterministic, so the iteration count may differ between identical runs. |
| `shaders/fluid.wgsl` `predict` | lines 128–149 | DORMANT | Gravity plus a Boussinesq buoyancy term from `temperature` against a hardcoded 293 K ambient. |
| `shaders/fluid.wgsl` `mix_dye` | lines 454–500 | DORMANT | Neighbor temperature diffusion, clamped to 200–500 K. This is the only heat transport in the repo, and it runs only when `step` runs. |
| SDF coupling | `sdf.rs`, `shaders/sdf_gen.wgsl` | DORMANT | `step` builds an SDF of dynamic objects. Real kernel, example-driven. |
| `FluidOptimizationController`, `step_with_budget` | `lib.rs`, `optimization.rs` | DORMANT | Used by `fluids_demo`. Budget code can change the iteration count at runtime. |
| GPU tests | `tests/gpu_execution_tests.rs` | REAL tests, envelope asserts | The test header says the suite checks physical envelopes, not bit-identical replay. Construction of `FluidRenderer` fails on lavapipe. See §13. |

`TemporalCoherence` and Morton helpers in `optimization.rs` do not appear in `SimParams` or `fluid.wgsl`. They are stats and CPU helpers. Label: STUBBED relative to the solver, DORMANT as library code.

### 2.2 SSFR and accents

| Piece | Path | Label | Note |
|---|---|---|---|
| SSFR depth, smooth, shade | `renderer.rs`, `shaders/ssfr_depth.wgsl`, `ssfr_smooth_v2.wgsl`, `ssfr_shade.wgsl` | DORMANT | `FluidRenderer::render` is the `fluids_demo` path. |
| Smooth pass depth load | `shaders/ssfr_smooth_v2.wgsl` 4, 23, 39 | REAL bug, logged | `texture_depth_2d` plus `textureLoad`. Naga's GLSL backend rejects that. Issue #247. |
| `render_accents` | `renderer.rs` | DORMANT as a library, wired in one example | Additive billboards. The SSFR chain is not invoked. |
| `SecondaryParticle`, `set_secondary_particles` | `lib.rs` 275–283, upload method | DORMANT library, example producer | CPU upload. Lines 281–282: the GPU emission kernel was never written. |
| Accent producer | `examples/weaving_playground/src/weave_accent_producer.rs` | DORMANT | Ages particles on the CPU and uploads them. Presentation only. |

### 2.3 Effects, editor, and support modules

| Module | Path | Label | Note |
|---|---|---|---|
| `caustics`, `god_rays`, `foam`, `water_reflections`, `underwater`, `underwater_particles`, `waterfall` | matching files under `src/` | DORMANT | Aggregated by `WaterEffectsManager` in `water_effects.rs`. Callers are that module's tests. Shaders for caustics, god rays, and SSR are inline strings, not files under `shaders/`. |
| `anisotropic` | `anisotropic.rs`, `shaders/anisotropic.wgsl` | DORMANT | `include_str!` is real. No caller outside the crate. |
| `emitter` | `emitter.rs` (`FluidEmitter`, `FluidDrain`) | DORMANT | No caller outside the crate. |
| `lod` | `lod.rs` | DORMANT | `fluids_demo` updates a LOD manager. `ParticleStreamingManager` is tests. |
| `serialization` | `serialization.rs` `FluidSnapshot` | DORMANT | Snapshot of GPU-side parameters and counts. Not a replay of cell state, because there is no cell state. |
| `profiling`, `debug_viz` | matching files | DORMANT | Tests and editor metadata. |
| `editor.rs` | 5,823 lines | DORMANT | Authoring schema. `tools/aw_editor` does not import it. |
| `ThermalConfig` | `editor.rs` 2153–2188 | STUBBED | Ambient temperature, diffusivity, buoyancy coefficient, evaporation. Stored on `FluidEditorConfig`. `FluidSystem` never reads it. |
| `FlowConfig` | `editor.rs` 2668–2693 | STUBBED | Direction, speed, turbulence, eddies. No runtime consumer. |

---

## 3. Water truth, volumes, buoyancy

| Piece | Path | Label | Note |
|---|---|---|---|
| `WaterSample`, `WaterQuery`, `AnalyticWater` | `astraweave-water/src/lib.rs` | REAL | Plane plus AABB volumes. Topmost surface wins. Ties keep registration order. Tests lock bit-identical samples. |
| `Aabb.linear_drag` | `lib.rs` 90–95 | STUBBED as a force | Stored so the `add_water_aabb` argument is not dropped. `sample` does not read it. |
| `PhysicsWorld.water` | `astraweave-physics/src/lib.rs` 931 | REAL | The facade the buoyancy tick samples. |
| `water_level`, `fluid_density` | `lib.rs` 924–926, write-through at 1444 | REAL | Scalar plane copied into the facade every buoyancy tick. `NEG_INFINITY` clears the plane. |
| `apply_buoyancy_forces` | `lib.rs` 1438–1488 | REAL | Called from `step_internal`. Impulse is `force * dt` with `g = 9.81`. Drag is per body. |
| `add_buoyancy` | `lib.rs` 1433 | DORMANT | Inserts into `buoyancy_bodies`. Production gameplay never registers a body. `examples/fluids_demo` laboratory scenario and physics tests do. |
| `add_water_aabb` | `lib.rs` 1493–1494 | REAL | Calls `self.water.add_aabb`. The physics trace's older "empty stub" sentence is wrong; the function comment records the F.2 fix. |
| `clear_water` | `lib.rs` 1605–1608 | REAL | `WeaveOpKind::LowerWater` calls it (`astraweave-gameplay/src/weaving.rs` 70–77) and wipes every volume. |
| `examples/physics_demo3d` | example | DORMANT demo | Toggles AABB water. |
| Editor `WaterVolume` entity | `tools/aw_editor` viewport and inspectors | REAL presentation | Becomes `WaterVolumeDesc` on `WaterRenderer`. It does not register an `AnalyticWater` AABB. A painted volume can be visible and absent from buoyancy. |

`buoyancy_bodies` is a `HashMap`. Iteration order is not a replay input today, and it must not become one.

---

## 4. Rendered water — REAL presentation

`astraweave-render/src/water.rs` draws a chunked Gerstner surface, authored volume patches, and up to eight weave deformations (`Part`, `Raise`, `Freeze`). The pass runs from the runtime renderer and from the editor. `examples/hello_companion`, the slice runtime, and `examples/weaving_playground` all drive it.

`Freeze` damps waves and tints ice in the shader. It does not write a collider, a buoyancy block, or a `WaterQuery` sample. `astraweave-render` and `astraweave-fluids` do not depend on each other. The playground binary composites accents over the surface.

This layer stays presentation. The design does not hash it.

---

## 5. Gameplay swim — DORMANT, tested

`astraweave-gameplay/src/water_movement.rs`, re-exported from `astraweave-gameplay/src/lib.rs`.

| Type | Role |
|---|---|
| `WaterMovementMode` | `Dry`, `Wading`, `WaistDeep`, `Swimming`, `Diving`. Speed and stamina multipliers are constants. |
| `WetStatus` | `Dry`, `Damp`, `Wet`, `Soaking`. Dry times are 0 / 10 / 30 / 60 seconds. |
| `WaterPlayerState::update` | Line 206. Input is a caller-supplied `submersion: f32` and `dt`. It does not call `WaterQuery`. |
| `WaterMovementHelper::calculate_water_forces` | Buoyancy, drag, and a swim direction from the same submersion float. |
| `WaterSkills` | Splash-dash and oxygen bonuses. |

Inline tests and `astraweave-gameplay/tests/mutation_resistant_comprehensive_tests.rs` cover the thresholds. A workspace search shows no caller outside `astraweave-gameplay` tests. Nothing in the character step reads `Stats.stamina` together with these multipliers.

PR #244 (`feat/traversal-archaeology`) already maps this module and plans to wire it in that work's phase 4, with hysteresis around the existing thresholds. This design feeds `WaterQuery`. It does not retune those thresholds.

---

## 6. Gas, fire, heat

| Search | Result | Label |
|---|---|---|
| Gas, smoke, combustion, ignition field | No sim type in any engine crate. | ABSENT |
| `DamageType::Fire`, `Frost`, `Shock`, `Poison` | `astraweave-gameplay/src/types.rs` 6–13. Combat tags. No field they sample. | REAL as an enum, ABSENT as a sim |
| GPU particle temperature | `fluid.wgsl` `predict` and `mix_dye` | DORMANT. See §2.1. |
| `ThermalConfig` | `editor.rs` 2153 | STUBBED. See §2.3. |
| Terrain thermal erosion | `astraweave-terrain/src/erosion.rs` `apply_thermal_erosion`; `advanced_erosion.rs` | DORMANT heightfield cook. Talus slope, not temperature. Tests and benches. The editor panel stores `ThermalErosionParams` and does not cook the live world from them. |
| Terrain wind erosion | `advanced_erosion.rs` `apply_wind_erosion` | DORMANT. Same shape as thermal erosion. |
| Climate temperature | `astraweave-terrain` climate and biome fields | REAL authoring data for biome choice. Not a heat grid. |
| Light `temperature` | editor lights, `TimeOfDay::sun_temperature` | REAL color temperature in kelvin. Not heat. |

There is no fire-spread function, no smoke density field, and no gas buoyancy outside the dormant GPU particle shader.

---

## 7. Wind, three different values

| Value | Path | Label | Note |
|---|---|---|---|
| `PhysicsWorld.wind` | `lib.rs` 932, `set_wind` 1497–1498 | STUBBED | `RedirectWind` writes it (`weaving.rs` 61–67). `step_internal` never reads `self.wind`. Tests assert the stored vector. |
| `EnvironmentManager` | `environment.rs` 295–408 | DORMANT | Wind zones, gusts, `wind_force_at`. `HashMap` iteration. The crate-root comment at `lib.rs` 31 says this type owns water volumes and buoyancy. The struct has neither. No caller outside physics tests. |
| `ProjectileManager.wind` | `projectile.rs` | DORMANT, tested | Ballistics read this field. It is not `PhysicsWorld.wind`. |
| `Cloth` `config.wind` | `cloth.rs` | DORMANT, tested | Per-cloth vector inside the cloth solver. |
| Slice-runtime storm `wind_strength` | `veilweaver_slice_runtime/src/vfx_specs.rs` | REAL presentation | A storm spec for VFX. Not a velocity field. |
| Editor weather sliders | `tools/aw_editor/src/panels/world_panel.rs` | REAL UI | `wind_speed` and `wind_direction` on a settings struct. |

---

## 8. Weaving

`WeaveOpKind` (`astraweave-gameplay/src/types.rs` 42–53) is a fixed enum. `apply_weave_op` (`weaving.rs` 11–150) is a match. The only non-test caller is `examples/weaving_playground`.

| Op | What the match does | Label |
|---|---|---|
| `RedirectWind` | `phys.set_wind`. Nothing integrates that vector. | STUBBED |
| `LowerWater` | `phys.clear_water()`. Every volume disappears. | REAL, coarse |
| `FreezeWater` | Spends weather budget and logs. Comment at lines 94–105: no physics, collision, or traversal change. | STUBBED as truth, REAL as a presentation trigger |
| `ReinforcePath`, `CollapseBridge`, `RaisePlatform` | Director terrain ops on the legacy `World`. | REAL on that legacy path |

There is no modifier trait, no material-density field, and no localized-gravity field. `astraweave-weaving` has no `modifier` symbol. The core directive asks for programmable modifiers of world state. The code is a closed match.

Render translation of those ops into `WeaveInstance` and accent particles lives in the playground, not in a library.

---

## 9. AI perception — ABSENT for hazards

`WorldSnapshot` (`astraweave-core/src/schema.rs` 270–278) is `t`, `player`, `me`, `enemies`, `pois`, `obstacles`, `objective`. `Poi` is a string tag and an `IVec2` (lines 365–368). There is no water sample, heat, gas, or hazard list.

`astraweave-ai` has no hazard type. The word "fire" in that crate is combat cover-fire, not a burn cell.

`perception_world` is a required replay case (`.github/workflows/determinism-replay.yml` line 95, produced by `astraweave-core/tests/replay_hash.rs`). Putting hazards into `WorldSnapshot` changes that hash. The design keeps the first hazard API off the snapshot.

---

## 10. Activation, interest, sleep

| Mechanism | Path | Label | Note |
|---|---|---|---|
| Fluid LOD and `ParticleStreamingManager` | `astraweave-fluids/src/lod.rs` | DORMANT | Particle culling for the demo. Not a world activation map. |
| Net `Interest` | `astraweave-net/src/lib.rs` | REAL | Snapshot interest for replication. Not a sim sleep set. |
| Rapier islands | inside `PhysicsWorld` | REAL | Solver sleep for rigid bodies. Not exposed as a region map for fluids. |
| Active-region map around players and AI | — | ABSENT | |

---

## 11. Determinism substrate on `main`

| Piece | State |
|---|---|
| `SimRng` | REAL. `astraweave-core/src/sim.rs`. `mix_sim_seed(world_seed, tick, system_id)` then ChaCha12. `unit_f32` is a dyadic rational. |
| System ids on `main` | `SIM_SYSTEM_CRAFT` `0xC2AF_7001`, `HARVEST` `0xC2AF_7002`, `ANCHOR_PARTICLES` `0xAE11_7001`, `ENEMY_PATROL` `0xAE11_7002`, `SPAWNER` `0xAE11_7003`, `TERRAIN_SOLVER` `0x7E22_7001`. None for fluids. |
| `Fixed64` | ABSENT on `main`. PR #245 proposes the canonical Q32.32 type in `astraweave-math`. PR #212 has a copy on its branch. PR #243 plans to consume one of those, not a third copy. |
| Replay workflow | REAL. Required cases today: `planner_pure`, `perception_world`. Three OS compare. A case that is missing on one OS fails only if it is in the required set. |
| GPU fluids | Excluded by the carve-out in §1. |

---

## 12. Open work this design has to meet

| PR | What it owns that local sim must not fork |
|---|---|
| #243 `feat/destruction` | Support graph, fracture, command log, `i8` density overlay (0 empty, 127 solid), material id `u16`, `SIM_SYSTEM_DESTRUCTION = 0xD357_7001`. Heat that breaks matter becomes their command, not a second graph. |
| #244 `feat/traversal-archaeology` | Swim and dive call `WaterQuery` and the existing `WaterPlayerState`. Thresholds stay. Their numeric contract on that branch is fixed 60 Hz, `f32::to_bits`, and `libm` via `detmath`. |
| #245 `cursor/fixed64-math-8aec` | The single `Fixed64` if a later pressure solve needs it. This design's tick does not. |
| #205 `cursor/aravath-greybox-b3de` | The Aravath demo. Not on `main`. Showcase waits until that runtime is on the branch. CC0 packs already chosen there. |
| #212 combat | `DamageType` tags and hit resolution. Local sim does not reimplement armor. |
| #215 terrain | Integer columns and `column_material`. #243 reads them. Local sim reads solidity through #243's field, not a private copy of `column_gen.rs`. |
| #223 CI | Replay workflow stays on every PR. Benches stay off pull-request checks. |

---

## 13. Failures already in view

| Failure | Evidence | Disposition |
|---|---|---|
| SSFR smooth pass cannot `textureLoad` a depth texture on lavapipe | `ssfr_smooth_v2.wgsl` lines 4, 23, 39. Issue #247, open, with owner (fluids) and a plan (write depth to `R32Float` and load `texture_2d<f32>`). The issue has no `deferred-failure` label. | Logged. Phase 2 of the local sim does not compile this shader. The label is a review action; this agent cannot edit issue labels. |
| `PhysicsWorld.wind` is write-only | `set_wind` 1497–1498; no read in `step_internal`. | In scope for the design as a quantized wind modifier. Applying the `f32` vector inside the Rapier step would put a non-replay force on the physics hash, which destruction and traversal both refuse. The field stays a stored vector. |
| `fluids.md` §1 describes deleted SPH and voxel solvers | Trace header says post-W.1; §1 does not. | Doc drift. This PR does not rewrite the trace, because it does not change the crate. A later commit that touches `astraweave-fluids` updates §1, §5, and §8. |
| Crate-root line claims `EnvironmentManager` owns water volumes | `astraweave-physics/src/lib.rs` 31 versus `environment.rs` 295–303. | Comment drift. Same rule: fix it in the commit that edits that file. |

No test suite was run for this docs change. #247 is the fluids failure already on file. A red check on this PR that the docs did not cause gets the same treatment: fix, or a `deferred-failure` issue with an owner and a plan.

---

## 14. Absent, in one list

- An active-region activation map.
- A deterministic liquid, gas, or fire grid.
- A reaction table (water + heat, oil + heat, poison cloud).
- A programmable weave modifier that the sim reads.
- Truth for `FreezeWater` and for `RedirectWind`.
- A swim caller.
- A hazard sample the AI can read without inventing a `WorldSnapshot` field.
- A `local_sim` replay case.
