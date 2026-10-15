# Weaving archaeology

**Status:** Phase 1 map. No runtime code in this change.
**Tree:** `main` at `85d09a2c` (2026-10-15).
**Open PRs read, not merged:** [#243](https://github.com/lazyxeon/AstraWeave/pull/243) destruction, [#212](https://github.com/lazyxeon/AstraWeave/pull/212) combat, [#244](https://github.com/lazyxeon/AstraWeave/pull/244) traversal, [#214](https://github.com/lazyxeon/AstraWeave/pull/214) mods. The Aravath greybox is `cursor/aravath-greybox-b3de` (PR #205), also not on `main`.

This document is the inventory the design in [`WEAVING_FRAMEWORK_DESIGN.md`](WEAVING_FRAMEWORK_DESIGN.md) is allowed to build on. Paths and type names are the ones on disk. New player-facing text uses Aravath. This pass did not find a red test; the rows below are residue (stub, dormant, unwired), and they are recorded here so Phase 2 does not treat them as a working chemistry engine.

## How to read a status

| Status | Meaning |
|---|---|
| REAL | A non-test caller on `main` runs the code. Example binaries count as REAL and are named. Editor and AI callers are called out when they exist. |
| STUBBED | The function or variant exists and its body skips the work the name describes. |
| DORMANT | It compiles and usually has tests. No production caller outside the defining crate. |
| UNTESTED | A production or example path with no dedicated test. |
| ABSENT | Searched, not in the tree on `main`. |

## 1. Three weave surfaces, none of them a chemistry engine

`docs/architecture/weaving.md` (rev 1.2, verified `7c29b8182`) already separates three pipelines that do not call each other. Rechecked on `85d09a2c`:

| Surface | Path | What it actually does | Status |
|---|---|---|---|
| Emergent intents | `astraweave-weaving/src/{patterns,intents,adjudicator}.rs` | Pattern detectors over a caller-supplied `WorldMetrics`, stringly `WeaveIntent`, budget and cooldown sort | DORMANT. Zero library consumers. Tests and benches only. |
| Anchor / ability slice | `astraweave-weaving/src/` (see §2) | Stability, currency, dash/shield, quests | REAL only inside two example binaries. Not an ECS system. |
| Director weave ops | `astraweave-gameplay/src/weaving.rs` `apply_weave_op` | Six hardcoded ops against the legacy grid `World` and `PhysicsWorld` | REAL in `examples/weaving_playground` only. |
| Water deformation | `astraweave-render/src/water.rs` `WeaveInstance` | GPU mesh deformation for part / raise / freeze | REAL as presentation, fed by the playground. |
| Fluid accents | `examples/weaving_playground/src/weave_accent_producer.rs` | Maps an op to GPU `SecondaryParticle` accents | REAL in the playground. Presentation. |

`rg 'astraweave-weaving' --glob '**/Cargo.toml'` on this tree finds the crate itself plus `examples/advanced_content_demo` and `examples/veilweaver_quest_demo`. `tools/aw_editor` and `astraweave-ai` do not depend on it. `apply_weave_op` has the same single example caller.

`examples/weaving_pcg_demo` defines its own signal and intent enums and does not import `astraweave-weaving`.

## 2. `astraweave-weaving`

Crate deps (`Cargo.toml`): `anyhow`, `serde`, `toml`, `rand`, `glam`, `astraweave-pcg`. No `astraweave-ecs`, `astraweave-core`, `astraweave-physics`, `astraweave-gameplay`, or `astraweave-render`. `astraweave-pcg` has zero `use` sites under `src/`. `#![forbid(unsafe_code)]` in `src/lib.rs`.

The functions in `src/systems/` take slices and `Vec`s. `SystemFn` is `fn(&mut World)` (`astraweave-ecs/src/lib.rs`). Those functions cannot be passed to `App::add_system`. The `add_system` lines in their docs are ignored examples.

| Item | Path | Behavior | Status | Determinism |
|---|---|---|---|---|
| `PatternDetector` and four detectors | `src/patterns.rs` | Pure `f32` scores over `WorldMetrics` | DORMANT | Deterministic given metrics |
| `WeaveIntent`, four proposers | `src/intents.rs` | String `kind`. Scavenger uses `seed % 2` | DORMANT | Seed argument, not a tick stream |
| `WeaveAdjudicator` | `src/adjudicator.rs` | Filter `min_priority`, sort `(priority desc, cost asc, kind asc)`, per-tick budget, per-key cooldowns in ticks | DORMANT | Deterministic if the caller ticks it |
| `CWeaveAgent`, `CWeaveSignal`, `WeaveIntentEvent` | `src/lib.rs` | Scan interval in `f32` seconds. String maps. No ECS impl | DORMANT | Wall-style `current_time` |
| `SimRng` duplicate | `src/sim_rng.rs` | Copy of core's mixer so this crate can avoid a core dep. Same `0xAE11_7*` ids | REAL inside spawner, patrol, particles | ChaCha12 via `StdRng`. `unit_f32` is dyadic |
| `Anchor` | `src/anchor.rs` | Stability in `[0, 1]`, decay, repair `+0.3` | REAL via the level type in `src/level.rs` | Decay takes a caller `dt` |
| `anchor::AbilityType` | `src/anchor.rs` | `EchoDash`, `BarricadeDeploy`. This is the re-export | Barricade is STUBBED (no deploy body) | — |
| `abilities::AbilityType` | `src/abilities.rs` | `EchoDash`, `EchoShield`. Second enum, not re-exported | REAL in the examples' dash/shield | Cooldowns take `dt` |
| `EchoCurrency` | `src/echo_currency.rs` | `u32` plus a trimmed log | REAL on the system path | Integer |
| `Player.echo_currency` | `src/level.rs` | A second `i32` balance. Dash and shield subtract literals `10` and `15` | REAL. This is the balance the quest demo spends | Integer, not checked against `AbilityState.echo_cost` |
| `anchor_repair_system` | `src/systems/anchor_repair_system.rs` | Spends `EchoCurrency`, unlocks below stability `0.7` | DORMANT (tests) | — |
| Level repair | `src/level.rs` | Parallel repair, quest threshold `0.8`, the `i32` balance | REAL in the quest demo | — |
| `anchor_decay_system` | `src/systems/anchor_decay_system.rs` | Passive decay. `apply_combat_stress_to_nearby_anchors` stresses every anchor in the slice | STUBBED distance check | Caller `dt` |
| Proximity, interaction, echo pickup, HUD | `src/systems/` | Plain functions, logic present | DORMANT | — |
| `CombatSystem` | `src/combat.rs` | Separate combat queue. The level's kill path does not use it | DORMANT at runtime | `f32` |
| `Enemy`, `EnemySpawner` | `src/enemy.rs`, `src/spawner.rs` | Patrol and waves | DORMANT at runtime. The level update does not run the spawner | `SimRng` when called |
| Quests | `src/quest.rs`, `quest_types.rs`, `starter_quests.rs` | Objective counters | REAL in `advanced_content_demo` and the quest demo | Counters |
| Particles, audio, UI | `src/particles/`, `src/audio/`, `src/ui/` | Data and command lists. egui `render()` is `#[cfg(any())]` | DORMANT. Particles use `SimRng` | Presentation data |
| `tests/*.rs`, in-module `#[cfg(test)]`, two criterion benches | crate tests | Logic, mutation, determinism of the emergent sort and of particle/spawn streams | Tests exist. They do not prove a game loop | — |

`src/combat.rs` `CombatEvent` is an enum (`PlayerDamaged`, `EnemyDamaged`, `EnemyKilled`, `PlayerKilled`). `src/systems/anchor_decay_system.rs` `CombatEvent` is a struct. Both names are in play. A third unrelated combat event type lives in `veilweaver_slice_runtime/src/combat.rs`, and that runtime does not depend on this crate.

## 3. Gameplay weave ops and combat modifiers

`astraweave-gameplay/src/types.rs`:

```rust
pub enum WeaveOpKind {
    ReinforcePath,
    CollapseBridge,
    RedirectWind,
    LowerWater,
    RaisePlatform,
    FreezeWater,
}
```

`apply_weave_op` (`src/weaving.rs`) matches that enum and does the following:

| Kind | Effect on `main` | Status |
|---|---|---|
| `ReinforcePath`, `RaisePlatform` | Pushes `DirectorOp::Fortify` on a grid rect, then `apply_director_plan` | REAL in the playground |
| `CollapseBridge` | `DirectorOp::Collapse` between two grid points | REAL in the playground. This is a 2D grid edit, not `DestructionManager` |
| `RedirectWind` | `PhysicsWorld::set_wind` with a normalized `f32` direction and strength `10.0` | REAL in the playground. One global vector |
| `LowerWater` | `PhysicsWorld::clear_water()` | REAL in the playground |
| `FreezeWater` | Spends `weather_ops` and logs. Comment at the match arm: no physics, collision, or traversal change | STUBBED for truth. Presentation uses `op.a` |

After the match, the function always calls `NavMesh::bake(nav_src, 0.5, 55.0)` and discards the mesh (`let _nav`). It allocates a `DirectorPlan { ops: vec![] }` per call. `WeaveConsequence.weather_shift` is an `Option<String>` (`"windy"`, `"dry"`, `"frozen"`).

| Modifier | Path | What a caller observes | Status |
|---|---|---|---|
| `DamageType` {Physical, Echo, Fire, Frost, Shock, Poison} | `types.rs` | Serde labels | The label is REAL. `Stats::apply_damage` ignores `_dtype` and subtracts `amount - defense * 0.5` |
| `StatusEffect` {Stagger, Bleed, Chill} | `stats.rs` | `tick(dt)` applies bleed damage and ages timers. `Chill.slow` is never read | Stagger is REAL in `examples/crafting_combat_demo` via `AttackState`. Chill slow is STUBBED. Bleed is tested, not pushed by the combo path |
| `EchoMod.special` | `items.rs` | Freeform string. Tests parse `"burn"` and `"stun"` | UNTESTED at runtime. No gameplay reader outside tests |
| `EchoMod.dtype_override` | `items.rs` | Overrides the combo's damage label | REAL on the combo path. The label still dies in `apply_damage` |
| `perform_attack_sweep` | `combat_physics.rs` | Rapier ray sweep, parry, iframes | REAL in `examples/combat_physics_demo`. No material or destructible coupling |
| ECS combat components | `ecs.rs` | Combo state on ECS entities | DORMANT. No engine `add_system` loop |

`examples/adaptive_boss` and `veilweaver_slice_runtime` drive `astraweave-director` telegraphs (`AntiRangedField`, `CounterShockAura` behind the slice feature). Those are director ops, not `WeaveOp`s.

## 4. Material and physics data a weave would touch

### Materials

`astraweave-materials` is a PBR graph baker (`Node`, `Graph`, `MaterialPackage`). It has metallic, roughness, normal, clearcoat, and transmission. It has no density, ignition, conductivity, or elemental tag. Terrain render identity is `astraweave-render/src/material_library.rs` (`MaterialLibrary`, 32 layer slots): texture ids, not sim properties.

There is no sim-material table on `main`.

### Physics

| Surface | Path | Status |
|---|---|---|
| `PhysicsWorld` step, kinematic `control_character` | `astraweave-physics/src/lib.rs` | REAL. Global gravity is a Rapier `f32` vector. Broadphase is Rapier's `DefaultBroadPhase` |
| Climb flag | same, `control_character(..., _climb)` | STUBBED. Zeroes gravity and adds `2.0 * dt` on Y. Covered by tests. The bench failure mode is tracked by issue #221 (traversal doc §6), not re-filed here |
| `set_wind` | `lib.rs` | REAL as a single vector. `EnvironmentManager` wind zones in `environment.rs` are DORMANT |
| `GravityManager`, `GravityZone` | `gravity.rs` | Implemented and tested (box, sphere, point). Stored in `HashMap`. Not called from `PhysicsWorld::step`. DORMANT. The module docs mention a cylinder the enum does not have |
| `DestructionManager` | `destruction.rs` | Health, fracture patterns, debris. Tested. No caller outside the physics crate. DORMANT |
| `process_destructible_hits` | `lib.rs` (empty body, `dead_code`) | STUBBED |
| `SpatialHash` | `spatial_hash.rs` | DORMANT as a broadphase. The crate docs still describe it as the broadphase |
| Buoyancy | `lib.rs` plus `astraweave-water` | REAL. `WaterQuery` / `AnalyticWater` / `WaterSample.density`. CPU `f32`, documented bit-identical for the same volumes |
| `water_movement.rs` | `astraweave-gameplay` | Swim, oxygen, wetness. Tested. Zero callers outside the crate. DORMANT |

`StructuralIntegrity` does not exist. Destructible "integrity" is `health`.

### Terrain density

`astraweave-terrain/src/voxel_data.rs` `Voxel { density, material }` is a real isosurface sample. `terrain_modifier.rs` `TerrainModifier` queues `AddDensity`, `SubtractDensity`, `SetMaterial`, and `Blend`. `TerrainModifier::` has no caller outside `astraweave-terrain`. `SIM_SYSTEM_TERRAIN_SOLVER` (`0x7E22_7001` in `astraweave-core/src/sim.rs`) has no caller.

### Fluids, gas, thermo

`docs/architecture/fluids.md` §0: GPU particle state is non-deterministic and presentation-only. A PR that hashes or replays it is rejected. The SPH solver and the voxel water grid were removed in W.1. What remains:

| Surface | Status |
|---|---|
| `astraweave-fluids` `FluidSystem` | GPU PBD. `Particle` carries `density`, `temperature`, and a phase byte. Consumers are `examples/fluids_demo` and `examples/weaving_playground`. DORMANT relative to the game loop |
| `editor.rs` in that crate | Forward-design. `tools/aw_editor` does not depend on the crate |
| Gas simulation | ABSENT |
| Gameplay thermo | ABSENT. Terrain climate `temperature_c` feeds biome generation, not a runtime field |
| `FreezeWater` truth | STUBBED (§3). Render freeze is REAL |

### Traversal

On `main`, the only wired 3D body move is `control_character`. Vault, mantle, shimmy, wall-run, glide, and prone are ABSENT. PR #244 is the review-gated plan and adds no feature code. Wetness exists only on the dormant water-movement module.

## 5. AI perception

`WorldSnapshot` (`astraweave-core/src/schema.rs`) is `t`, `player`, `me`, `enemies`, `pois`, `obstacles`, `objective`. No material, temperature, gravity, phase, or weave field. `build_snapshot` (`astraweave-core/src/perception.rs`) fills the grid tactical view. `astraweave-ai` has no weave or hazard hook.

AI cannot perceive a weave on `main`, and it cannot respond to one.

## 6. Mods (PR #214, not on `main`)

On `main`, authoring is `astraweave-author` (`run_author_script` → `DirectorBudget`) and `astraweave-scripting` (`register_api` with `spawn` and `spawn_particle`). Item echo rows load from TOML in `load_echo_defs` with no non-test caller. Editor scenes are RON. There is no weave schema.

PR #214 (`cursor/engine-ws-mod`) adds a sandboxed Rhai tick. The allowlist in `astraweave-security/src/sandbox.rs` on that branch is:

```text
apply_damage, configure, despawn, move_to, play_sound,
set_position, sim_roll, spawn_particle, spawn_prefab
```

`MOD_COMMAND_API_VERSION` is 1. `MOD_SYSTEM_ID` is `0x4D4F_4453`. Manifests load in Kahn order with ties broken by mod id. Director data is capped (traps 64, terrain edits 256, spawns 128). Rolls use `astraweave_core::sim::SimRng`. No weave command, no graph file, no reaction table.

## 7. Local simulation agent

Searched `main` for a bounded fluid/gas/thermo agent, an active-region map, and a local sim type. ABSENT.

The nearest pieces are the full-scene GPU fluid grid (presentation, example-only), the render water chunk window (presentation), and `TerrainModifier`'s edit queue (dormant, not thermo).

## 8. Determinism already on `main`

`astraweave-core/src/sim.rs`:

| Id | Constant | Used |
|---|---|---|
| `0xC2AF_7001` | `SIM_SYSTEM_CRAFT` | gameplay crafting |
| `0xC2AF_7002` | `SIM_SYSTEM_HARVEST` | gameplay harvest |
| `0xAE11_7001` | `SIM_SYSTEM_ANCHOR_PARTICLES` | weaving particles, duplicated in `astraweave-weaving/src/sim_rng.rs` |
| `0xAE11_7002` | `SIM_SYSTEM_ENEMY_PATROL` | weaving patrol, same duplicate |
| `0xAE11_7003` | `SIM_SYSTEM_SPAWNER` | weaving spawner, same duplicate |
| `0x7E22_7001` | `SIM_SYSTEM_TERRAIN_SOLVER` | unused |

`SimRng::new(world_seed, tick, system_id)` reseeds from `mix_sim_seed`. `unit_f32` is a multiple of `2^-24`. `Fixed64` has zero matches on `main`. PR #212 places a Q32.32 `Fixed64` in `astraweave-combat/src/fixed.rs` and `SIM_SYSTEM_COMBAT = 0xC0BA_7001`. PR #243 reserves `SIM_SYSTEM_DESTRUCTION = 0xD357_7001` and does not add a second fixed-point type.

ECS stage order (`App::new` in `astraweave-ecs/src/lib.rs`): `pre_simulation`, `perception`, `simulation`, `sync`, `ai_planning`, `physics`, `post_simulation`, `presentation`. Systems in a stage run in registration order, single-threaded.

Replay workflow on `main` hashes `perception_world` and `planner_pure`. PR #212 adds `combat_resolve`. PR #214 adds `scripted_system`. Neither hashes a weave field.

## 9. Aravath demo

Not on this branch. `docs/current/ENGINE_PRODUCTION_ROADMAP.md` records the greybox on `cursor/aravath-greybox-b3de` and tells engine workstreams to stay out of `aravath_runtime/**` and `assets/aravath/**`. PR #243's showcase is a headless scene plus a call site once that runtime is on the branch being built. This map uses the same boundary.

## 10. Consequences

1. A weave today is either a string intent nobody executes, a six-arm match that allocates, or a GPU deformation. None of those is a modifier of density, gravity, decay, or temperature.
2. The data those modifiers need (sim material tags, a deterministic gravity table, a CPU thermo field, an AI hazard record) is ABSENT or DORMANT. The dormant physics modules (`GravityManager`, `DestructionManager`) use `f32` and `HashMap` and are the wrong place to hang a replay hash.
3. Phase 2 has to add a truth layer and then sink into the open PRs. It does not get to pretend `apply_weave_op` or the anchor systems are that layer.
4. GPU fluids stay presentation. The local sim, if approved, is a new CPU field with a fixed tick.
