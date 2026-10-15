# Destruction Archaeology

**Status:** Phase 1 survey. No runtime code in this change.
**Base:** `main` `85d09a2c` (merge of PR #211), read 2026-10-15.
**Open PRs read, not merged:** [#205](https://github.com/lazyxeon/AstraWeave/pull/205) Aravath demo, [#212](https://github.com/lazyxeon/AstraWeave/pull/212) combat, [#215](https://github.com/lazyxeon/AstraWeave/pull/215) terrain, [#216](https://github.com/lazyxeon/AstraWeave/pull/216) render, [#223](https://github.com/lazyxeon/AstraWeave/pull/223) CI tiers.
**Design:** [`DESTRUCTION_DESIGN.md`](DESTRUCTION_DESIGN.md).

Status words:

| Word | Meaning |
|---|---|
| REAL | Implemented and reached from a non-test caller |
| STUBBED | Signature exists; body ignores its inputs or does not do the named job |
| DORMANT | Implemented, usually tested, and not called from production |
| UNTESTED | No test covers the behavior |

A row uses one primary word. Tests are mentioned in the "what it does" cell when the primary word would hide them.

Determinism is judged for a three-OS bit-identical replay. Same-process repeatability is not enough.

---

## 1. Physics

Rapier 0.22 is the solver (`Cargo.toml` workspace dep, features `debug-render` only). `enhanced-determinism` is not enabled on `main`. `PhysicsWorld` stores `HashMap<RigidBodyHandle, BodyId>`. The step uses `f32` (`PhysicsConfig` default `dt = 1/60`, gravity `(0, -9.81, 0)`). The crate forbids `unsafe`. None of this crate is an input to `.github/workflows/determinism-replay.yml`.

`tests/determinism.rs` is `#[cfg(feature = "async-physics")]`, seeds `rand::StdRng`, and compares two runs in one process. It is not a cross-OS hash.

| Path | What it does | Status | Determinism-safe? | Relevance |
|---|---|---|---|---|
| `astraweave-physics/src/lib.rs` `PhysicsWorld::step` / `step_internal` | Rapier pipeline, then query-pipeline update. Buoyancy impulse runs before the step. | REAL | N. `f32` Rapier, `HashMap` body maps, no enhanced-determinism, not in the replay workflow. | Debris must not use this step as the hashed sim. |
| `astraweave-physics/src/lib.rs` `JointType` (`Fixed`, `Revolute`, `Prismatic`, `Spherical`), `add_joint` | Rapier joints. `JointId(0)` means a missing body. | REAL | N. Same `f32` solver. | Bonds in the support graph are not these joints. A joint is a one-way mirror after a bond breaks. |
| `astraweave-physics/src/lib.rs` `CharacterController` | Kinematic capsule, coyote time, jump buffer, `max_climb_angle_deg` (slope limit, default 45). | REAL | N. `f32` timers and `dt`. | Slope limit is not ledge climb. No climbable flag on a body. |
| `astraweave-physics/src/lib.rs` `add_destructible_box` | `_health` and `_break_impulse` are ignored. Delegates to `add_dynamic_box`. | STUBBED | N | The name is the trap. Do not grow this function into the fracture system. |
| `astraweave-physics/src/lib.rs` `break_destructible` | Removes the Rapier body and the local maps. | REAL as deletion | N | Not a fracture. `examples/physics_demo3d` calls it. |
| `astraweave-physics/src/lib.rs` `process_destructible_hits` | Empty body, `#[allow(dead_code)]`. | STUBBED | n/a | Documented invariant 19 in `docs/architecture/physics.md`. |
| `astraweave-physics/src/lib.rs` `apply_radial_impulse` | `f32` falloff over dynamic bodies. | REAL | N | Explosion gameplay belongs in the command log. This helper is a float mirror. |
| `astraweave-physics/src/lib.rs` `add_water_aabb` / `clear_water` | Water facade. Real since F.2 (the physics trace's "empty stub" note is stale). | REAL | N. `f32` | Out of scope. Do not retouch water. |
| `astraweave-physics/src/destruction.rs` (2,788 lines) | `DestructionManager`, `FracturePattern::{uniform,radial,layered}`, health, force, debris lifetime. `HashMap` storage. Angular velocity is `sin` of the id. `spawn_debris` walks the map. | DORMANT | N. `f32`, `HashMap` order, libm `sin`/`acos`/`cbrt`. | Tested (`destruction.rs` unit tests, `tests/coverage_boost_tests.rs`, `tests/mutation_resistant_comprehensive_tests.rs`, `benches/destruction.rs`). No production caller. The editor panel does not call it. |
| `astraweave-physics/src/spatial_hash.rs` | Grid broadphase. Doc-comment claims it is the world broadphase. | DORMANT | N. `FxHashMap`. | `PhysicsWorld` uses Rapier `DefaultBroadPhase`. Leave it. |
| `astraweave-physics/src/async_scheduler.rs` | Parallel step profile. Feature `async-physics`. | DORMANT | N. Thread count can change Rapier. | Do not enable for destruction. |
| `astraweave-physics/src/ecs.rs` | `PhysicsPlugin`. Feature `ecs`. | DORMANT | N | No consumer enables the feature. |
| `astraweave-physics/src/projectile.rs` | Hitscan and kinematic projectiles, `ExplosionConfig`. | DORMANT | N. `f32`. | Tests only. Combat sweep does not call it. |
| `astraweave-physics/src/gravity.rs` | Zonal gravity. Caller must push forces back. | DORMANT | N | Not the support-graph load. |
| `astraweave-physics/src/ragdoll.rs` | Jointed bone ragdoll built into `PhysicsWorld`. | DORMANT | N | Tests only. |
| `astraweave-physics/src/vehicle.rs` | Raycast suspension. | DORMANT | N | Tests only. |
| `astraweave-physics/src/cloth.rs` | Verlet cloth. | DORMANT | N. Same-process test exists; still `f32`. | Not a fracture solver. |
| `astraweave-physics/src/environment.rs` | Wind zones and gusts. | DORMANT | N | Separate from `PhysicsWorld.wind`. |
| `astraweave-physics/benches/destruction.rs` | Criterion: create, damage, update the float manager. | REAL bench, DORMANT system | N | Not a budget for the new graph. PR checks do not run it. |
| `examples/physics_demo3d/src/main.rs` | Spawns `add_destructible_box` and calls `break_destructible`. | REAL demo of the stub | N | Do not treat this demo as the showcase. |
| `tools/aw_editor/src/panels/physics_panel.rs` | Sliders for debris lifetime, max count, impulse, chain reaction. Buttons log and do not spawn. `destructible_count` stays 0. | STUBBED | n/a | UI knobs with no reader. Do not wire them to the new crate in passing. |

---

## 2. Terrain

`docs/architecture/terrain.md` (verified 2026-07-26) already splits a wired heightmap path from a voxel path that is tested and not the editor's ground. PR #215 deletes several of the dormant modules and adds integer columns. Those deletions are not on `main`.

| Path | What it does | Status | Determinism-safe? | Relevance |
|---|---|---|---|---|
| `astraweave-terrain/src/heightmap.rs` | `f32` height grid. `set_height`. | REAL | N. `f32` samples. | Editor ground. Craters on this grid would not be cross-OS identical. |
| `astraweave-terrain/src/chunk.rs` `TerrainChunk` / `ChunkManager` | Heightmap chunks. | REAL | N | Editor streaming unit. |
| `astraweave-terrain/src/noise_gen.rs`, `noise_simd.rs`, `perlin_gradient.rs` | Fractal noise. SIMD path uses floats. | REAL | N | #215 owns generation. Do not retune noise. |
| `astraweave-terrain/src/climate.rs`, `biome*.rs`, `world_archetypes.rs`, `spline_types.rs`, `regional_archetype_mask.rs` | Climate, biomes, splines, regional blend. | REAL in the editor | N. `f32`. Solver rolls use `SIM_SYSTEM_TERRAIN_SOLVER` (`0x7E22_7001`) through `SimRng`. | Leave to #215. |
| `astraweave-terrain/src/erosion.rs` `apply_thermal_erosion` / `apply_hydraulic_erosion` | In-place `f32` heightmap filters. | REAL | N | Not the integer droplets on #215. |
| `astraweave-terrain/src/advanced_erosion.rs` `AdvancedErosionSimulator` | Hydraulic, thermal, wind presets. | DORMANT | N. `f32`, seed is a constructor `u64` but the math is float. | Tests only (`tests/phase_1_6_f3_*`). #215 does not delete this file. |
| `astraweave-terrain/src/runevision_erosion.rs` | Filter on `f32` heights. | DORMANT | N | Not a dig API. |
| `astraweave-terrain/src/voxel_data.rs` | SVO, `CHUNK_SIZE = 32`, `Density = f32`, `MaterialId = u16`, `Voxel::is_solid` at `> 0.5`. `HashMap` chunk storage. | DORMANT relative to the editor ground. REAL as a library used by `examples/hybrid_voxel_demo` and the editor's unused brush. | N. `f32` density, `HashMap`. | The comment says "dynamic terrain deformation". The samples are not quantized. |
| `astraweave-terrain/src/meshing.rs` `DualContouring` | Hermite dual contouring to a float `ChunkMesh`. Also `AsyncMeshGenerator`, `LodMeshGenerator`. | DORMANT in the editor. REAL in `examples/hybrid_voxel_demo`. | N. The mesh is float. | Display meshing may stay float. Collision must not be derived from it. No surface-nets implementation. |
| `astraweave-terrain/src/marching_cubes_tables.rs` | Standard edge and triangle tables. | DORMANT | Y for the tables (integer). | No marching-cubes mesher calls them outside tests. |
| `astraweave-terrain/src/collision.rs` | `collision_mesh_from_chunk` / `collision_mesh_from_heightmap` build a float trimesh. | DORMANT | N. Vertices are `Vec3`. | Call sites are the module tests. Collision today follows the float mesh. |
| `astraweave-terrain/src/terrain_modifier.rs` (762 lines) | Queued `VoxelOp` (`Set`, `AddDensity`, `SubtractDensity`, `SetMaterial`, `Blend`). Budget uses `std::time::Instant`. | DORMANT | N. Wall clock plus `f32` density. | Tests only. #215 deletes this file. Do not revive it. |
| `astraweave-terrain/src/terrain_persistence.rs` | Chunk save header and region IO. | DORMANT | N | #215 deletes it. Edits should be a command log, not a full voxel dump. |
| `astraweave-terrain/src/compressed_voxels.rs` | Palette and RLE. | DORMANT | Unknown; not the edit format. | #215 deletes it. |
| `astraweave-terrain/src/background_loader.rs` | Async chunk streaming. | DORMANT | N | #215 deletes it. |
| `astraweave-terrain/src/lod_manager.rs`, `lod_blending.rs` | View-dependent LOD and morph. | DORMANT | N. `f32` pixel error. | #215 replaces this with integer `lod_step` / `morphed_height`. |
| `astraweave-terrain/src/streaming_diagnostics.rs` | Hitch and memory stats. | DORMANT | N. Uses frame time. | #215 deletes it. |
| `astraweave-terrain/src/gpu_bridge.rs` | GPU heightmap, noise, erosion requests. | DORMANT | N | #215 deletes it. Render GPU erosion is also deleted there. |
| `astraweave-terrain/src/partition_integration.rs` | World-partition voxel windows. | DORMANT | N | Not the #215 column stream. |
| `astraweave-terrain/src/scatter.rs`, `zone_scatter.rs`, `structures.rs` | Vegetation and structure placement. | REAL in the editor / zone path | N. `f32` and, in places, unseeded variation. | #215 owns forest scatter (`SIM_SYSTEM_FOREST_SCATTER`). Props are not destructible chunks. |
| `astraweave-terrain/src/solver.rs` `TerrainSolver` | Validates authored locations against the seed. | REAL | Y for the rolls that go through `SIM_SYSTEM_TERRAIN_SOLVER`. The rest of placement is still float. | Do not fold destruction seeds into this id. |
| `tools/aw_editor/src/terrain_integration.rs` `apply_brush` / `apply_brush_paint_material` | Editor height and material paint. | REAL | N. `f32` height. | This is the live sculpt path. It is not a sim command log. |
| `tools/aw_editor/src/voxel_tools.rs` `VoxelEditor` | Sphere, cube, cylinder; add, remove, paint. `apply_brush` writes a `VoxelGrid`. | DORMANT | N. `f32`. | `show_voxel_editor` edits brush fields. Undo logs "integration pending" and does not call `apply_brush` on the world. |
| `examples/hybrid_voxel_demo/src/main.rs` | Builds a grid and meshes it with `DualContouring`. | REAL example | N | Not the Aravath scene. |

### What PR #215 adds (not on `main`)

Branch `cursor/ws-terrain-scatter`. These are the APIs destruction should call, not reimplement.

| Path on #215 | What it does | Determinism-safe? |
|---|---|---|
| `astraweave-terrain/src/column_gen.rs` `surface_height`, `column_voxels`, `write_column` | Integer column ground from `FOREST_WORLD_SEED`. | Y. `i32` heights. |
| `column_gen.rs` `erode_chunk_heights` | Integer droplets, neighbor order north, east, south, west. `SimRng::new(FOREST_WORLD_SEED, droplets, SIM_SYSTEM_COLUMN_EROSION)`. `SIM_SYSTEM_COLUMN_EROSION = 0x7E22_7102`. | Y |
| `column_gen.rs` `morphed_height` | LOD blend in milli-units, 0..1000. | Y. Display only if the sim keeps the unmorphed column. |
| `column_gen.rs` `build_chunks_sorted` | Rayon generate, commit in `(x, z)` order. | Y for the committed order. |
| `astraweave-terrain/src/column_stream.rs` `ColumnStream`, `StreamDelta`, `lod_step` | `BTreeSet` load set. Hysteresis LOD. | Y |
| Replay case `worldgen_columns` | Added to that branch's required set. | Y, once merged |

#215's PR text says chunks with edits stay on the full mesh, and physics keeps the full column while distant render meshes use the morphed height. There is no per-chunk player-edit log on that branch. `Density` remains `f32` in the surviving `voxel_data.rs`.

---

## 3. Render mesh

PR #216 owns IBL, PBR maps, and `TimeOfDay`. It does not add a destruction pass.

| Path | What it does | Status | Determinism-safe? | Relevance |
|---|---|---|---|---|
| `astraweave-render/src/renderer.rs` `add_model`, `add_model_with_bounds`, `add_model_with_pbr_textures`, `add_model_sharing_texture` | Upload a mesh and instances once via `queue.write_buffer`. | REAL | n/a (GPU). | No `replace_mesh` / `update_vertices` API. A changed chunk is remove plus add, or a new hook. That hook is an overlap to flag, not a second pipeline. |
| `astraweave-render/src/instancing.rs` | Per-mesh instance batches. | REAL | n/a | Debris LOD can reuse instancing. |
| `astraweave-render/src/particle_render.rs`, `effects.rs` | Billboard particles. | REAL | n/a. Render-only if the sim never reads them. | Cosmetic micro-debris belongs here. |
| `astraweave-render/src/decals.rs` `Decal`, `DecalSystem`, `GpuDecal` | Deferred decal struct and a GPU system. | DORMANT | n/a | `renderer.rs` has no `decals::` call. Tests construct `DecalSystem`. Damage masks are not on screen. |
| `astraweave-render/src/mesh.rs` and glTF/OBJ loaders | Static mesh import. | REAL | n/a | Prefractured meshes can enter through the existing loader. |
| #216 `ModelSurfaceMaps`, `add_composite_model`, `bake_showcase_environment`, `TimeOfDay::update(dt)` | PBR maps and a clock that takes `dt` instead of `Instant`. | On the PR, not on `main` | `TimeOfDay` is replay-stable given `dt`. GPU work is not hashed. | Destruction does not add a pass, a sky, or a material. |

---

## 4. Damage and combat

| Path | What it does | Status | Determinism-safe? | Relevance |
|---|---|---|---|---|
| `astraweave-gameplay/src/stats.rs` `Stats::apply_damage` | `i32` HP minus `defense as f32 * 0.5`. | REAL | N. Float mitigation. | Legacy. PR #212 does not delete it. |
| `astraweave-gameplay/src/combat_physics.rs` `perform_attack_sweep` | Rapier overlap, then `Stats::apply_damage`. | REAL | N | The production melee path on `main`. It does not emit a destruction event. |
| `astraweave-core/src/sim.rs` `SimRng` | ChaCha12 via `mix_sim_seed(world_seed, tick, system_id)`. `unit_f32` is a dyadic rational. | REAL | Y for integer draws and `unit_f32`. | This is the canonical RNG on `main`. |
| `astraweave-weaving/src/sim_rng.rs` | Duplicate of three system ids. Weaving does not depend on core. | REAL | Y if the constants stay equal. | Do not add a third copy of `SimRng`. |
| PR #212 `astraweave-combat` | `resolve_hit` draws four `u32`s. `hit_rng` folds ordinal and attack id into `SIM_SYSTEM_COMBAT` (`0xC0BA_7001`) and calls `SimRng::new` once. | On the PR | Y. Required replay case `combat_resolve`. | Destruction consumes a resolved impulse. It does not re-roll armor or schools. |
| PR #212 `astraweave-combat/src/fixed.rs` `Fixed64` | Q32.32. `SCALE = 1 << 32`. Mul through `i128`. Div truncates toward zero. `den == 0` yields zero. Overflow saturates. | On the PR, not on `main` | Y. No float conversion in the module. | The only `Fixed64` implementation. `rg Fixed64` on `main` is empty. |
| PR #205 `aravath_runtime/src/combat.rs` | Calls `astraweave_veil::Fixed64`. | On the PR | Same arithmetic, second crate. | #212's own comment says to delete the local module once one `Fixed64` is on `main`. |

There is no `voronoi`, support graph, stress solve, or CSG brush in sim code on `main`. `FracturePattern::radial` is a golden-angle sphere of debris prefabs, not a mesh cut.

---

## 5. Infrastructure

| Path | What it does | Status | Determinism-safe? | Relevance |
|---|---|---|---|---|
| `.github/workflows/determinism-replay.yml` | Three OS jobs run `cargo test -p astraweave-core --test replay_hash`. Compare requires `planner_pure` and `perception_world`. | REAL | Y for those two cases. | A new case must be added to this required set. Other PRs add different names (see below). |
| `astraweave-core/tests/replay_hash.rs` | Writes `target/replay-hash.json` as `{case, sha256}` rows. | REAL | Y | The harness. |
| `astraweave-core/tests/replay_support/mod.rs` | `planner_pure` and `perception_world` (100 ticks). SHA-256 of explicit little-endian bytes. Does not sort. | REAL | Y | Perception must already be ordered. Destruction encoding should follow this style. |
| Cases named in the brief but absent on `main` | `ai_depth_tick`, `scripted_system`, `worldgen_columns`, `combat_resolve`, `gameplay_rng`. | Not on `main` | — | `worldgen_columns` is #215. `combat_resolve` is #212. `gameplay_rng` is #223. `ai_depth_tick` and `scripted_system` are on none of those heads. |
| `astraweave-net/src/lib.rs` | Snapshot plus delta. `EntityState` is `id, pos: IVec2, hp, team, ammo`. `world_hash: u64` via `DefaultHasher`. | REAL | N for the hash. `DefaultHasher` is not stable across rustc. | No destruction payload. Late join is not a command log. |
| `astraweave-persistence-ecs` `save_game` / `load_game` | `SaveBundleV2`. | REAL | Separate from destruction. | #212 adds combat tables on `meta` and keeps schema version 2. A destruction blob must not collide with that field. |
| `docs/current/CI_ARCHITECTURE.md` | Not on `main`. On #223: benches run on `main`, nightly, dispatch, or the `run-benchmarks` label. They are not a pull-request check. | On the PR | — | New criterion benches stay out of the required PR set. |
| Traversal | No `feat/traversal` branch, no open PR, no navmesh climb link (`astraweave-nav` has no climb or off-mesh match). | Absent | — | Authoring docs mention a string tag `"climbable"` (`docs/reference/authoring_schemas.md`). That tag is not read by the character controller. |

### `SimRng` system ids already taken

| Constant | Value | Where |
|---|---|---|
| `SIM_SYSTEM_CRAFT` | `0xC2AF_7001` | `main` |
| `SIM_SYSTEM_HARVEST` | `0xC2AF_7002` | `main` |
| `SIM_SYSTEM_ANCHOR_PARTICLES` | `0xAE11_7001` | `main` |
| `SIM_SYSTEM_ENEMY_PATROL` | `0xAE11_7002` | `main` |
| `SIM_SYSTEM_SPAWNER` | `0xAE11_7003` | `main` |
| `SIM_SYSTEM_TERRAIN_SOLVER` | `0x7E22_7001` | `main` |
| `SIM_SYSTEM_COMBAT` | `0xC0BA_7001` | #212 |
| `SIM_SYSTEM_FOREST_SCATTER` | `0x7E22_7101` | #215 |
| `SIM_SYSTEM_COLUMN_EROSION` | `0x7E22_7102` | #215 |

---

## 6. Aravath demo (PR #205, not on `main`)

`aravath_runtime` is absent on `main`. On `cursor/aravath-greybox-b3de`:

| Path | What it does | Status | Determinism-safe? | Relevance |
|---|---|---|---|---|
| `aravath_runtime/src/sim.rs` | Fixed 60 Hz slices. Dig is held across a short frame. | REAL on the PR | The demo hashes its own replay (`aravath-determinism.yml`, Linux and Windows). Not the three-OS `determinism-replay.yml` case. | Showcase call site. |
| `aravath_runtime/src/ground.rs` `dig_sphere` / `VoxelEdit { x, y, z, density: f32, material }` | Writes `Voxel::new(0.0, 0)` or `1.0`. `replay` restores the edit vec. | REAL on the PR | N. `f32` density. `HashMap<ChunkCoord, u64>` for bodies. `HashSet` for generated chunks. | A second dig path would fork the demo. Brush commands replace this log. |
| `aravath_runtime/src/ground.rs` | Meshes with `DualContouring` and builds Rapier trimeshes from that mesh. | REAL on the PR | N. Collision follows the float mesh. | The integer field has to become the collider source. |
| `aravath_runtime/src/locomotion.rs`, `motor.rs` | Movement. No climb, mantle, or vault symbol. | REAL on the PR | Rapier `f32`, with `enhanced-determinism` and libm on that branch. | Still not a Fixed64 body. |
| `assets/aravath/` | KayKit forest props, Kenney nature, Poly Haven textures. `CREDITS.md` on the PR. | REAL on the PR | n/a | CC0. Reuse. Do not vendor a second copy of the same files. |

---

## 7. Consequences for the design

1. `DestructionManager` is a tested float toy with no caller. Extending it cannot reach cross-OS identity without throwing away its math, its maps, and its coupling to Rapier.
2. `Fixed64` is not on `main`. Copying it would be a third implementation next to #212 and the type PR #205 already imports.
3. `SimRng` is on `main`. The seed shape is three `u64`s. Extra ids are folded into `system_id`, which is what `hit_rng` does.
4. Terrain generation, erosion, and streaming belong to #215. The live edit path on `main` is an `f32` height brush plus a wall-clock voxel queue that #215 deletes.
5. Render upload is one-shot. Decals are unwired. #216 is the PBR and lighting owner.
6. The replay harness hashes explicit bytes from `astraweave-core`'s test. Rapier state is not in that hash, and it must stay out.
7. Nothing in the tree publishes a climbable or anti-climb flag on a fractured piece.
