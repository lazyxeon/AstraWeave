---
schema_version: 1
trace_id: physics
title: "Physics"
description: "Physics (Rapier3D wrapping + subsystems)"
primary_crate: astraweave-physics
domain: physics-world
lifecycle_status: active
integration_status: wired
owns: [astraweave-physics]
doc_version: "1.2"
last_verified_commit: 32afac52f
---

# Architecture Trace: Physics

## Metadata

| Field | Value |
|---|---|
| **System name** | Physics (Rigid Body Simulation + Subsystems) |
| **Primary crates** | `astraweave-physics` (12 source files / ~31K LoC), wraps `rapier3d 0.22` |
| **Document version** | 1.2 |
| **Last verified against commit** | `32afac52f` |
| **Last verified date** | 2026-05-12 |
| **Status** | **Active core** (`PhysicsWorld` + `CharacterController` + Rapier broadphase) **with feature-gated and dormant subsystems** (async-physics, ECS plugin, in-house SpatialHash). The core rigid-body / character / raycast surface is production-wired through `astraweave-gameplay` and `astraweave-npc`. Several subsystems (projectile, gravity zones, ragdoll, vehicle, environment, destruction, cloth) are independent managers with rich APIs but limited production wiring — most have tests + benches but no game-loop consumer yet. The `SpatialHash` module is **dormant** despite being advertised in the crate doc-comment. |
| **Owner notes** | This trace is forensic, not aspirational — it documents what is wired today, not what the doc-comments imply. Scale: 12 source files, 31K LoC, 20 test files, 10 benchmarks, 9 consumer Cargo deps (5 production + 4 examples). **Verification pass 2026-05-12 (version 1.1):** resolved 3 markers — (a) `astraweave-scripting` is a real production consumer that uses `PhysicsWorld` as an ECS resource AND wraps it in a `*const PhysicsWorld` raw pointer via `PhysicsProxy` for Rhai bindings (§4 Downstream row corrected); (b) `PhysicsWorld` IS `Send + Sync` (auto-derived, proven by compile-time obligations of `World::get_resource<T>` — §8 Invariant 18 corrected, previous "Send but NOT Sync" claim was wrong); (c) `process_destructible_hits` has zero callers workspace-wide (closed in §11). Decision Log "[Reasoning not recovered]" markers (8 entries) remain — git log searches returned no physics-specific decision commits. **Deep investigation pass 2026-05-12 (version 1.2):** corrected 4 factual errors and enriched 2 Open Questions. Corrections: (a) the crate has **5 features**, not 3 — `alloc-counter` and `fast-alloc` were missing from §1, §5, Appendix A; (b) `alloc-counter` feature location in §9 was wrong (was "Cargo.toml:67-69 — wait that's astraweave-ai", actually `Cargo.toml:17` + bench gating at `:77-80`); (c) `SpatialHash` test surface in §11 was significantly under-counted (claimed "3 dedicated tests" — actually 20+ tests across 4 files: `behavioral_correctness_tests.rs`, `coverage_boost_tests.rs`, `cross_subsystem_validation.rs`, `spatial_hash_character_tests.rs` with 33 `#[test]` attributes); (d) the claim that "`benches/raycast.rs` mentions spatial-hash patterns" was wrong — zero `SpatialHash` references in any bench file. Enrichments: (a) §11 `ecs` feature question now documents three de facto consumer patterns (direct passthrough / `World::insert_resource` without feature / designed `PhysicsPlugin` with feature) — `astraweave-scripting` uses pattern (b) which sits between the existing two; (b) `SpatialHash` Open Question enriched with comprehensive test surface inventory + "no bench" correction. |

---

## 1. Executive Summary

**What this system does:**
Provides 3D rigid-body simulation, kinematic character control, and a collection of optional physics subsystems (projectiles, gravity zones, ragdolls, vehicles, environmental effects, destruction, cloth) by wrapping Rapier3D 0.22 inside a higher-level `PhysicsWorld` facade with engine-native IDs (`BodyId: u64`) and glam vector types.

**Why it exists:**
Game-side code needs to manage rigid bodies, ground-aware character movement, raycasts, and joint constraints without exposing Rapier's lifetime-heavy raw handles or doing direct nalgebra↔glam conversions at every call site. `PhysicsWorld` is the single mutable state owner and `BodyId` is the stable identifier consumers store.

**Where it primarily lives:**
- `astraweave-physics/src/lib.rs` (5,381 LoC) — `PhysicsWorld` (struct + Rapier wrappers), `CharacterController`, `PhysicsConfig`, `ActorKind`, `Layers`, `JointType`, `BodyId`, `DebugLine`, `raycast`, `step`, `apply_force`/`impulse`, `control_character`, `add_dynamic_box`/`add_character`/`add_static_trimesh`, `add_joint`, `add_buoyancy`
- `astraweave-physics/src/spatial_hash.rs` (1,038 LoC) — independent `SpatialHash<T>` + `AABB` (dormant — see §5/§6)
- `astraweave-physics/src/async_scheduler.rs` (979 LoC) — `AsyncPhysicsScheduler` + `PhysicsStepProfile` (feature `async-physics`, dormant outside benches)
- `astraweave-physics/src/ecs.rs` (84 LoC) — `PhysicsPlugin` + `PhysicsBodyComponent` (feature `ecs`, dormant)
- `astraweave-physics/src/projectile.rs` (2,659 LoC) — `ProjectileManager`, `ProjectileConfig`, `ProjectileKind` (`Hitscan`/`Kinematic`)
- `astraweave-physics/src/gravity.rs` (1,425 LoC) — `GravityManager` + `GravityZone` + `GravityZoneShape` (`Box`/`Sphere`/`Point`)
- `astraweave-physics/src/ragdoll.rs` (3,062 LoC) — `Ragdoll`, `RagdollBuilder`, `BoneShape` (`Capsule`/`Sphere`/`Box`)
- `astraweave-physics/src/vehicle.rs` (4,869 LoC) — `Vehicle`, `VehicleManager`, `WheelConfig`, drivetrain + friction curves
- `astraweave-physics/src/environment.rs` (3,401 LoC) — `EnvironmentManager`, `WindZone` (`Global`/`Box`/`Sphere`/`Cylinder` shapes, `Directional`/`Vortex`/`Turbulent` types), `WaterVolume` (buoyancy)
- `astraweave-physics/src/destruction.rs` (2,788 LoC) — `DestructionManager`, `Destructible`, `FracturePattern`, `Debris`
- `astraweave-physics/src/cloth.rs` (4,359 LoC) — `ClothManager`, `Cloth`, `ClothParticle` (Verlet integration), `DistanceConstraint`

**Status note (read first):**
1. **`PhysicsWorld` is the single mutable owner.** All add/remove/step/query operations require `&mut PhysicsWorld`. There is no parallel mutation API in the canonical path.
2. **`BodyId: u64` is the stable engine handle**, NOT Rapier's `RigidBodyHandle`. Consumers store `BodyId`; `handle_of(id)` (`lib.rs:914-915` mapping fields + accessor) translates to Rapier's internal handle.
3. **The crate uses `#![forbid(unsafe_code)]`** (`lib.rs:1`) — all unsafe code is upstream in Rapier3D.
4. **Rapier 0.22's `DefaultBroadPhase` is the broadphase**, NOT the in-crate `SpatialHash`. See §6 Conflict Map.
5. **Fixed time step of 1/60s is the default** (`PhysicsConfig::default()` at `lib.rs:612-623`); callers override via `PhysicsConfig::with_time_step` (`:567-570`).
6. **The crate has 5 Cargo features:** `async-physics` (Rayon parallel scheduler, `Cargo.toml:9`), `profiling` (Tracy spans, `:10`), `ecs` (`PhysicsPlugin` for `astraweave-ecs`, `:11`), `alloc-counter` (bench-only — `[[bench]] name = "alloc_measure"` at `:77-80` requires it, `:17`), `fast-alloc` (bench-only — mimalloc allocator swap, "Does nothing to the library itself" per inline comment, `:20`). None of the five are enabled by any non-bench / non-example workspace consumer today (verified 2026-05-12).

---

## 2. Authoritative Pipeline

### 2.1 Canonical step cycle (sync, no features enabled)

```text
[Game tick — caller invokes physics_world.step()]
    │
    │ PhysicsWorld::step(&mut self)
    │ file: astraweave-physics/src/lib.rs:1039-1066
    ▼
[Stage 1: Choose path]
    role: If `async-physics` feature is enabled AND `async_scheduler.is_some()`,
          route through async path with telemetry; otherwise fall through to step_internal
    file: lib.rs:1039-1066
    │
    ▼
[Stage 2: step_internal]
    file: lib.rs:1070-1106
    role: Apply buoyancy forces, then run Rapier's PhysicsPipeline::step, then
          update QueryPipeline so raycasts in the SAME tick see post-step geometry
    │
    ├─[2a] apply_buoyancy_forces()
    │     role: Iterate self.buoyancy_bodies HashMap; compute Archimedes force per body
    │           below self.water_level; add_force on each Rapier rigid body
    │
    ├─[2b] self.pipeline.step(gravity, integration_params, island_mgr, broad_phase,
    │                          narrow_phase, bodies, colliders, joints, multibody_joints,
    │                          ccd, Some(query_pipeline), &(), event_handler)
    │     role: Rapier-native step: integration → broadphase (DefaultBroadPhase)
    │           → narrowphase → constraint solver → CCD → emit collision/contact-force events
    │
    └─[2c] self.query_pipeline.update(&self.colliders)
          role: Refresh query pipeline so subsequent raycasts see new positions
          rationale (per `lib.rs:1102-1104` comment): "Week 2 Day 3 fix — without this,
          raycasts in control_character() use stale geometry, causing character controller
          to fail ground detection"
    │
    ▼
[Stage 3: Drain events (caller responsibility)]
    role: Caller iterates self.collision_recv + self.contact_force_recv crossbeam channels
          to consume collision/contact events emitted during step
    files: collision_recv: rapier3d::crossbeam::channel::Receiver<CollisionEvent>
           contact_force_recv: rapier3d::crossbeam::channel::Receiver<ContactForceEvent>
    │
    ▼
[Stage 4: Read positions (caller responsibility)]
    role: For each tracked BodyId, caller may call get_velocity / read body.position()
          via self.bodies.get(handle_of(id))
```

### 2.2 Character control cycle (called once per tick per character, before/after step)

```text
[Game tick — caller updates desired_move + invokes physics_world.control_character(id, dm, dt, climb)]
    │
    │ PhysicsWorld::control_character(&mut self, id, desired_move, dt, _climb)
    │ file: lib.rs:1247-1320 (excerpt verified through :1300)
    ▼
[C1: Lookup]
    role: Copy CharacterController from self.char_map[id]; return early if missing
    file: lib.rs:1251-1260
    │
    ▼
[C2: Timer updates]
    role: ctrl.jump_buffer_timer -= dt
    file: lib.rs:1263
    │
    ▼
[C3: Apply gravity (or zero if climbing)]
    role: ctrl.vertical_velocity -= 9.81 * ctrl.gravity_scale * dt (or = 0.0 if _climb)
    file: lib.rs:1265-1270
    │
    ▼
[C4: Jump consumption]
    role: If (time_since_grounded < coyote_time_limit) AND (jump_buffer_timer > 0):
          set vertical_velocity = pending_jump_velocity; invalidate coyote; clear buffer
    file: lib.rs:1272-1280
    │
    ▼
[C5: Compute delta movement + raycast obstacle slide]
    role: Build delta d = desired_move * dt; raycast forward from torso height;
          slide along hit normal if blocked
    file: lib.rs:1282-…
    │
    ▼
[C6: Set translation back on Rapier body]
    role: rb.set_translation(...) with computed new position; rewrites self.char_map[id]
    file: lib.rs:…
```

### 2.3 Combat sweep (production caller pattern)

```text
[Game-side combat tick]
    │
    │ astraweave_gameplay::combat_physics::perform_attack_sweep(
    │     &mut phys, attacker_id, &attacker_pos, &targets,
    │     attack_range, &mut stats_map, &mut parry_map, &mut iframe_map
    │ )
    │ file: astraweave-gameplay/src/combat_physics.rs:36-…
    ▼
[CS1: Use PhysicsWorld::raycast or shape sweep]
    role: Iterate targets in range; use physics raycast to confirm line-of-sight
    file: combat_physics.rs:…
    output: Vec<HitInfo> — game-side struct, not Rapier
```

### 2.4 ECS-integrated step (feature `ecs`, currently dormant)

```text
[App tick via astraweave-ecs Schedule]
    │
    │ SystemStage::PHYSICS executes registered systems in order
    ▼
[E1: physics_step_system(world: &mut World)]
    file: astraweave-physics/src/ecs.rs:26-30
    role: Resolve world.get_resource_mut::<PhysicsWorld>(); call .step()
    │
    ▼
[E2: sync_physics_to_transform_system(world: &mut World)]
    file: astraweave-physics/src/ecs.rs:33-84
    role: For every entity with PhysicsBodyComponent, read pos/rotation from
          PhysicsWorld via handle_of(body_id), construct Transform { translation,
          rotation, scale (preserved) }, write back via world.insert
    │
    ▼
[Render systems read Transform components]
```

Note: This path requires `astraweave-physics/features = ["ecs"]` to be enabled by the consumer; no production crate enables it today (verified workspace grep 2026-05-12 — see §6).

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **`BodyId: u64`** | Engine-stable opaque handle for any rigid body. Stored by game code. Mapped to Rapier's `RigidBodyHandle` via `PhysicsWorld::handle_of(id)`. `BodyId(0)` is invalid (allocator starts at 1). | `lib.rs:184`, all consumer crates |
| **`ActorKind`** | `#[non_exhaustive]` 4-variant enum: `Static / Dynamic / Character / Other`. Tracked per body in `body_kinds: HashMap<RigidBodyHandle, ActorKind>`. Used for filtering and game-side dispatch. | `lib.rs:186-241` |
| **`CharacterController`** | Per-character kinematic state: state (`Grounded`), dimensions (`radius`, `height`, `max_step`), velocity (`vertical_velocity`, `pending_jump_velocity`), gravity (`gravity_scale`), and timers (`time_since_grounded`, `jump_buffer_timer`, `coyote_time_limit`, `jump_buffer_limit`). Stored in `PhysicsWorld.char_map: HashMap<BodyId, CharacterController>`. | `lib.rs:424-535` |
| **`CharState`** | `#[non_exhaustive]` enum with one variant: `Grounded`. Currently only "grounded" is tracked; `is_rising()` and `vertical_velocity > 0` are queried directly instead of having additional states. | `lib.rs:391-422` |
| **`Layers` (bitflags)** | Collision layer bit set: `DEFAULT = 0b01`, `CHARACTER = 0b10`. Used by `InteractionGroups::new(filter, mask)` for Rapier collision groups. | `lib.rs:383-389` |
| **`PhysicsConfig`** | Construction-time config: `gravity: Vec3` (default `(0, -9.81, 0)`), `ccd_enabled: bool`, `max_ccd_substeps: usize`, `time_step: f32` (default `1.0/60.0`), `water_level: f32`, `fluid_density: f32` (default `1000.0`). Used by `PhysicsWorld::from_config`. | `lib.rs:537-623` |
| **`JointType`** | `#[non_exhaustive]` 4-variant enum: `Fixed`, `Revolute { axis, limits }`, `Prismatic { axis, limits }`, `Spherical`. Construction helpers: `revolute_x/y/z()`, `prismatic_y()`. Reports `degrees_of_freedom()` (0/1/3). | `lib.rs:635-805` |
| **`JointId(u64)`** | Stable joint handle. `JointId(0)` is invalid. | `lib.rs:807-839` |
| **`DebugLine`** | `{ start: [f32; 3], end: [f32; 3], color: [f32; 3] }` — output of `DebugRenderPipeline`. Convenience constructors: `red`/`green`/`blue`/`white`. Color helpers + length/midpoint/direction queries. | `lib.rs:249-355` |
| **`BuoyancyData`** | Per-body buoyancy attachment: `{ volume, drag }`. Stored in `buoyancy_bodies: HashMap<BodyId, BuoyancyData>`; consumed inside `apply_buoyancy_forces` at step start. | `lib.rs:1413-1448` (`add_buoyancy`) |
| **`SpatialHash<T>` + `AABB`** | Independent grid broadphase data structure. `AABB { min: Vec3, max: Vec3 }` with `from_center_extents`/`from_sphere`/`intersects`. `SpatialHash<T>::new(cell_size)` + `insert(id, aabb)` + `query(aabb)` + `query_unique` + `clear`. **NOT used by `PhysicsWorld`** (which uses Rapier's `DefaultBroadPhase`). | `spatial_hash.rs` |
| **`ProjectileKind`** | `#[non_exhaustive]` 2-variant enum: `Hitscan` (instant raycast) / `Kinematic` (gravity + drag + bounce + penetrate). | `projectile.rs:43-52` |
| **`ProjectileConfig`** | Spawn parameters: position, velocity, gravity_scale, drag, radius, max_lifetime, max_bounces, restitution, penetration, owner (`Option<u64>`), user_data. | `projectile.rs:55-84` |
| **`GravityZoneShape`** | `#[non_exhaustive]` 3-variant enum: `Box { min, max }`, `Sphere { center, radius }`, `Point { center, radius, strength }` (attractor/repulsor). | `gravity.rs:57-72` |
| **`GravityZone`** | A region overriding gravity for bodies inside. Has `priority: u32` — higher priority zones win. | `gravity.rs` |
| **`BoneShape`** | `#[non_exhaustive]` 3-variant enum for ragdoll bones: `Capsule { radius, half_height }`, `Sphere { radius }`, `Box { half_extents }`. Each has `volume()` for mass distribution. | `ragdoll.rs:39-65` |
| **`WindZoneShape`** | `#[non_exhaustive]` 4-variant enum: `Global` (infinite), `Box { half_extents }`, `Sphere { radius }`, `Cylinder { radius, height }`. | `environment.rs:22-31` |
| **`WindType`** | `#[non_exhaustive]` 3-variant enum: `Directional`, `Vortex { tangential_speed, inward_pull, updraft }`, `Turbulent { intensity, frequency }`. | `environment.rs:42-61` |
| **`DebrisShape`** | `#[non_exhaustive]` 3-variant enum: `Box { half_extents }`, `Sphere { radius }`, `ConvexHull { half_extents }` (currently approximated as Box per inline comment at `destruction.rs:28`). | `destruction.rs:23-30` |
| **`ClothParticle`** | Verlet integration state per particle: `position`, `prev_position`, `acceleration`, `inv_mass` (0 = pinned), `pinned: bool`. | `cloth.rs:17-29` |
| **`PhysicsStepProfile`** | Per-step telemetry struct (`async-physics` feature only): `total_duration`, `broad_phase_duration`, `narrow_phase_duration`, `integration_duration`, `active_body_count`, `collision_pair_count`, `solver_iterations`. Percentage helpers: `broad_phase_percent` / `narrow_phase_percent` / `integration_percent`. | `async_scheduler.rs:17-74` |
| **`AsyncPhysicsScheduler`** | `{ thread_count, last_profile, enable_profiling }`. `with_threads(n)` constructor. Records step durations. Active only behind feature `async-physics`. | `async_scheduler.rs:78-100` |

### Terms to NOT confuse

- **Rapier `RigidBodyHandle` vs engine `BodyId`**: `RigidBodyHandle` is Rapier's internal generational index. `BodyId` is the engine-stable `u64` handle. Game code uses `BodyId`; only `PhysicsWorld` internals translate via `body_ids: HashMap<RigidBodyHandle, BodyId>` and `handle_of` accessor.
- **`SpatialHash` (in-crate) vs `DefaultBroadPhase` (Rapier)**: The in-crate `SpatialHash` is a standalone data structure with its own tests + doc-comments claiming "99.96% pair reduction." It is NOT used by `PhysicsWorld` — Rapier's `DefaultBroadPhase` is the actual broadphase. See §6 Conflict Map.
- **`PhysicsWorld::gravity` vs `GravityManager::default_gravity` vs `PhysicsConfig::gravity`**: Three places gravity is stated. `PhysicsConfig::gravity` is the constructor parameter; `PhysicsWorld::gravity: Vector<Real>` is the live setting Rapier consumes each step; `GravityManager` (in `gravity.rs`) is a separate optional system for per-body / zone-based gravity overrides — it is NOT wired into `PhysicsWorld::step` automatically. Consumers must drive `GravityManager` themselves and call `PhysicsWorld::apply_force` per body.
- **`buoyancy_bodies` (per-body) vs `add_water_aabb` (volume)**: `add_buoyancy(id, volume, drag)` (`lib.rs:1413`) tags a single body for buoyancy. `add_water_aabb` (`lib.rs:1449`) takes `_min`, `_max`, `_density`, `_linear_damp` but its body is a stub — **all parameters are underscored and the function body is empty** (`pub fn add_water_aabb(...) {}`). Water-volume-based buoyancy is not implemented in `PhysicsWorld`; only per-body buoyancy is.
- **`break_destructible(id)` (`PhysicsWorld`) vs `DestructionManager` (`destruction.rs`)**: `PhysicsWorld::break_destructible` removes a rigid body wholesale (cleans up Rapier maps + `char_map` + `buoyancy_bodies`). `DestructionManager` is a separate subsystem with `FracturePattern` + `Debris` for richer destruction. They are not coupled.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| Game loop tick | `physics_world.step()` (`lib.rs:1039`) | None (uses internal state) | Caller-driven; one tick per `step()` call |
| Game logic | `physics_world.apply_force(id, force)`, `apply_impulse(id, impulse)`, `set_velocity(id, vel)`, `set_body_position(id, pos)` (`lib.rs:1108-1137, :1589-1595`) | `BodyId`, `Vec3` | Mutate body state before next step |
| Game logic | `physics_world.control_character(id, desired_move, dt, climb)` (`lib.rs:1247`) | `BodyId`, `Vec3`, `f32`, `bool` | Kinematic character path, called once per tick per character |
| Game logic | `physics_world.jump(id, height)` (`lib.rs:1239`) | `BodyId`, `f32` | Queues a jump via `jump_buffer_timer` + `pending_jump_velocity`; consumed on next `control_character` |
| Construction | `PhysicsWorld::new(Vec3)` or `PhysicsWorld::from_config(PhysicsConfig)` (`lib.rs:931, :967`) | `Vec3` gravity or full `PhysicsConfig` | Single owner; one `PhysicsWorld` per simulation domain |
| Construction | `add_dynamic_box(pos, half, mass, groups)`, `add_static_trimesh(verts, indices, groups)`, `add_character(pos, half)`, `create_ground_plane(half, friction)`, `add_destructible_box(pos, half, mass, health, break_impulse)` (`lib.rs:1175, :1154, :1199, :1139, :1556`) | Vec3 transforms, geometry, `Layers` | Returns `BodyId`; tags body kind |
| Construction | `add_joint(body1, body2, joint_type)` (`lib.rs:1605`) | `BodyId × BodyId × JointType` | Returns `JointId(0)` on failure |
| Construction | `add_buoyancy(body, volume, drag)` (`lib.rs:1413`) | `BodyId`, `f32`, `f32` | Caller must ensure body has positive mass |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-gameplay` | `combat_physics::perform_attack_sweep` (`astraweave-gameplay/src/combat_physics.rs:36`) takes `&mut PhysicsWorld`; uses `raycast` + body queries | `&mut PhysicsWorld`, attacker/target IDs | The canonical production combat path. 12 tests in the same file exercise it. |
| `astraweave-gameplay` (mutation tests) | `astraweave-gameplay/src/mutation_tests.rs:2108-…` | `&mut PhysicsWorld` | 8 mutation-killing tests construct `PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0))` directly |
| `astraweave-npc` | `astraweave-npc/src/runtime.rs:6,21,81` — `pub phys: &'a mut PhysicsWorld` in `NpcManager` ctx, `spawn_from_profile(&mut PhysicsWorld, NpcProfile)` constructor | `&mut PhysicsWorld`, NPC config | Calls `control_character` with fixed `dt = 1.0/60.0` per `runtime.rs:33-34` (see `docs/architecture/ai_pipeline.md` §13.3 invariant N6) |
| `astraweave-scripting` | `world.insert_resource(physics)` + `world.get_resource::<PhysicsWorld>()` + `get_resource_mut::<PhysicsWorld>()` at `astraweave-scripting/src/lib.rs:148, :336, :453, :506`; raw-pointer wrapping via `PhysicsProxy { ptr: *const PhysicsWorld, body_map: Arc<HashMap<u64, u64>> }` at `astraweave-scripting/src/api.rs:67-74` (with `unsafe impl Send for PhysicsProxy` + `unsafe impl Sync for PhysicsProxy`) | `*const PhysicsWorld` raw pointer + ECS-resource access pattern | Verified 2026-05-12: Scripting is a production consumer that treats `PhysicsWorld` as an `astraweave-ecs::World` resource AND wraps it in a `PhysicsProxy` raw pointer for Rhai scripting bindings. The integration does NOT enable the `astraweave-physics/features = ["ecs"]` feature (scripting's Cargo.toml has no features set); it uses `World::insert_resource` directly, which only requires `T: 'static + Send + Sync`. |
| `examples/combat_physics_demo`, `examples/fluids_demo`, `examples/nav_physics_bridge`, `examples/npc_town_demo`, `examples/physics_demo3d`, `examples/profiling_demo`, `examples/scripting_advanced_demo`, `examples/scripting_playground`, `examples/ui_controls_demo`, `examples/veilweaver_demo` | Direct `astraweave-physics = { path = "../../astraweave-physics" }` deps | Whole API | Example-level usage |
| Rendering / Transform | `astraweave-physics::ecs::sync_physics_to_transform_system` writes `astraweave-scene::Transform` from physics positions (`ecs.rs:33-84`) | `astraweave-scene::Transform` | **Feature-gated behind `ecs` feature; not enabled by any production consumer** (verified workspace grep 2026-05-12) |

### Bidirectional / Coupled

- **`PhysicsWorld` ↔ collision/contact event channels:** Step emits events via `ChannelEventCollector` into `collision_recv: rapier3d::crossbeam::channel::Receiver<CollisionEvent>` and `contact_force_recv: …<ContactForceEvent>` (`lib.rs:911-913, :932-934`). Caller is responsible for draining them — receivers are `unbounded` so unconsumed events accumulate in memory.
- **`PhysicsWorld` ↔ `QueryPipeline`:** `step_internal` explicitly calls `query_pipeline.update(&self.colliders)` AFTER the Rapier step (`lib.rs:1105`) because `control_character` and `raycast` consult the query pipeline for collisions. Skipping this update was the cause of "Week 2 Day 3" character-controller ground-detection failures per the inline comment at `:1102-1104`.
- **`astraweave-physics::ecs::PhysicsBodyComponent` ↔ entity `Transform`:** When the `ecs` feature is enabled, every entity bearing `PhysicsBodyComponent(body_id)` has its `Transform` overwritten each `SystemStage::PHYSICS` tick by `sync_physics_to_transform_system` (`ecs.rs:33-84`). Scale is preserved; translation + rotation come from physics.

---

## 5. Active File Map

### `astraweave-physics` — core simulation crate

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-physics/src/lib.rs` (5,381 LoC) | `PhysicsWorld`, `CharacterController`, `PhysicsConfig`, `BodyId`, `ActorKind`, `Layers`, `JointType`, `JointId`, `DebugLine`, `BuoyancyData`, `LineCollector`, step + raycast + character + joint + buoyancy + body-add APIs | Active | `#![forbid(unsafe_code)]` at line 1. The crate's central state-owning module. |
| `astraweave-physics/src/spatial_hash.rs` (1,038 LoC) | Standalone `SpatialHash<T>` + `AABB` grid-broadphase data structure | **Dormant** | Verified 2026-05-12: workspace grep for `use astraweave_physics::SpatialHash` or `use astraweave_physics::spatial_hash` outside the crate's own `src` + `tests` returned zero matches. The crate doc-comment at `lib.rs:25-26` advertises it as "Grid-based broadphase for O(n log n) collision culling (99.96% pair reduction vs brute-force)" but `PhysicsWorld` uses Rapier's `DefaultBroadPhase` (`lib.rs:907`), not `SpatialHash`. The module has 3 dedicated tests (`tests/behavioral_correctness_tests.rs:22`, `tests/coverage_boost_tests.rs:870`, plus its own inline test module). |
| `astraweave-physics/src/async_scheduler.rs` (979 LoC) | `AsyncPhysicsScheduler`, `PhysicsStepProfile` (per-step telemetry) | Active (feature `async-physics`); dormant in production | Cargo.toml gates the entire module on `#[cfg(feature = "async-physics")]`. Verified 2026-05-12: only `astraweave-physics/Cargo.toml:55` (`physics_async` bench `required-features = ["async-physics"]`) enables the feature; no production consumer enables it. |
| `astraweave-physics/src/ecs.rs` (84 LoC) | `PhysicsPlugin` (`Plugin`), `PhysicsBodyComponent`, `physics_step_system`, `sync_physics_to_transform_system` | Active (feature `ecs`); dormant in production | Gated on `#[cfg(feature = "ecs")]`. Verified 2026-05-12: workspace grep for `astraweave-physics.*features.*ecs` or `astraweave-physics/features = ["ecs"]` returned zero matches in `.toml` files. No consumer enables the `ecs` feature. |
| `astraweave-physics/src/projectile.rs` (2,659 LoC) | `ProjectileManager`, `ProjectileConfig`, `Projectile`, `ProjectileKind`, `ExplosionConfig`, `ExplosionResult`, `FalloffCurve`, `ProjectileHit`, `ProjectileId`, `predict_trajectory`, `hitscan`, `update` with caller-provided `raycast_fn` | Active (module-level) | Independent of `PhysicsWorld` — caller provides a `raycast_fn: F` closure to `ProjectileManager::update`. Consumer integration left to game code. |
| `astraweave-physics/src/gravity.rs` (1,425 LoC) | `GravityManager`, `GravityZone`, `GravityZoneShape` (`Box`/`Sphere`/`Point`), `BodyGravitySettings`, `BodyGravityId`, `GravityZoneId` | Active (module-level) | Independent of `PhysicsWorld::gravity`. Caller must apply forces back through `PhysicsWorld::apply_force` (no auto-integration). |
| `astraweave-physics/src/ragdoll.rs` (3,062 LoC) | `Ragdoll`, `RagdollBuilder`, `BoneDef`, `BoneJointType`, `BoneShape`, `RagdollConfig`, `RagdollId`, `RagdollPresets`, `RagdollState` | Active (module-level) | `RagdollBuilder::build(&mut PhysicsWorld, spawn_pos)` is the Bridge into `PhysicsWorld`. Per `ragdoll.rs:25-28` doc-comment. |
| `astraweave-physics/src/vehicle.rs` (4,869 LoC) | `Vehicle`, `VehicleManager`, `VehicleConfig`, `WheelConfig`, `WheelPosition`, `WheelState`, `DrivetrainType`, `EngineConfig`, `FrictionCurve`, `TransmissionConfig`, `VehicleInput`, `VehicleId` | Active (module-level) | Raycast-suspension model (industry standard). Per `vehicle.rs:1-7` doc-comment. |
| `astraweave-physics/src/environment.rs` (3,401 LoC) | `EnvironmentManager`, `WindZone`, `WindZoneConfig`, `WindType`, `WindZoneShape`, `WindZoneId`, `WaterVolume`, `WaterVolumeId`, `GustEvent` | Active (module-level) | Separate from `PhysicsWorld::wind: Vec3` (a bare global wind vector on `PhysicsWorld` itself). |
| `astraweave-physics/src/destruction.rs` (2,788 LoC) | `DestructionManager`, `Destructible`, `DestructibleConfig`, `DestructibleId`, `DestructibleState`, `DestructionEvent`, `DestructionTrigger`, `FracturePattern`, `Debris`, `DebrisConfig`, `DebrisId`, `DebrisShape` | Active (module-level) | Separate from `PhysicsWorld::add_destructible_box` (`lib.rs:1556`) which is a passthrough to `add_dynamic_box`. |
| `astraweave-physics/src/cloth.rs` (4,359 LoC) | `Cloth`, `ClothManager`, `ClothCollider`, `ClothConfig`, `ClothId`, `ClothParticle`, `DistanceConstraint` | Active (module-level) | Verlet integration + distance constraints. Per `cloth.rs:1-7` doc-comment. |
| `astraweave-physics/src/mutation_tests.rs` (1,259 LoC) | Mutation-killing inline tests inside `#[cfg(test)] mod mutation_tests` | Active (test-only) | |
| `astraweave-physics/tests/*` (20 files) | `behavioral_correctness_tests`, `buoyancy_test`, `coverage_boost_tests`, `cross_subsystem_validation`, `debug_render_test`, `determinism`, `ecs_integration_test`, `environment_tests`, `gravity_tests`, `mutation_resistant_*_tests` (×2), `nan_infinity_tests`, `panic_safety_tests`, `phase1_verification`, `physics_core_tests`, `physics_laws_tests`, `projectile_tests`, `ragdoll_tests`, `spatial_hash_character_tests`, `vehicle_tests` | Active | Comprehensive integration test surface |
| `astraweave-physics/benches/*` (10 files) | `alloc_measure` (`required-features = ["alloc-counter"]` per `Cargo.toml:77-80`), `character_controller`, `cloth`, `destruction`, `gravity`, `physics_async` (`required-features = ["async-physics"]` per `:52-55`), `ragdoll`, `raycast`, `rigid_body`, `vehicle` | Active | Benchmark surface. Two of the 10 benches are feature-gated: `physics_async` is the only consumer of `async-physics`; `alloc_measure` is the only consumer of `alloc-counter`. The `fast-alloc` feature (`Cargo.toml:20`) is bench-only (per inline comment "Does nothing to the library itself") but no bench currently declares `required-features = ["fast-alloc"]`. |

**Status definitions:**
- **Active**: Canonical, load-bearing, edit with care
- **Active (feature `X`)**: Compiles only when the named Cargo feature is enabled
- **Dormant**: Compiles but has no production consumers (verified workspace grep)

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Disposition |
|---|---|---|---|
| `astraweave-physics::SpatialHash<T>` — in-crate grid broadphase | `spatial_hash.rs` (1,038 LoC) | Dormant | The lib.rs doc-comment at `:25-26` advertises this as the broadphase, but `PhysicsWorld` uses Rapier's `DefaultBroadPhase` (`lib.rs:907`) and `NarrowPhase` (`:908`). Verified 2026-05-12: zero external `use astraweave_physics::SpatialHash` imports. Whether to wire `SpatialHash` in or remove it is a decisional question — see §11. |
| Rapier's `DefaultBroadPhase` + `NarrowPhase` | `lib.rs:907-908` (struct fields) | Active | Canonical broadphase. Pulled in via `pub use rapier3d::prelude::{DefaultBroadPhase, NarrowPhase, ...}` at `lib.rs:53-103`. |
| `PhysicsWorld::wind: Vec3` (bare global) | `lib.rs:923` (struct field) | Active | Single global wind vector on `PhysicsWorld` itself. |
| `EnvironmentManager::WindZone` family | `environment.rs` (3,401 LoC) | Active (module-level) | Richer zonal wind system with `WindZoneShape` + `WindType` enums. Independent of `PhysicsWorld.wind`. |
| `PhysicsWorld::gravity: Vector<Real>` (single global) | `lib.rs:904` (struct field) | Active | Rapier consumes this each step (`:1086-1100`). |
| `GravityManager` + `GravityZone` (zonal/point attractors) | `gravity.rs` (1,425 LoC) | Active (module-level) | Independent of `PhysicsWorld.gravity`. Caller must drive `GravityManager` separately and apply per-body forces via `PhysicsWorld::apply_force`. |
| `PhysicsWorld::add_destructible_box` (passthrough) | `lib.rs:1556-1565` | Active | Wraps `add_dynamic_box`; `_health`/`_break_impulse` parameters are underscored and ignored. |
| `DestructionManager` + `Destructible` + `FracturePattern` | `destruction.rs` (2,788 LoC) | Active (module-level) | Richer destruction system. NOT coupled to `PhysicsWorld::break_destructible`. |
| `PhysicsWorld::add_water_aabb` (stub) | `lib.rs:1449` | **Stub** | All parameters (`_min, _max, _density, _linear_damp`) underscored; function body is `{}`. Water-volume-based buoyancy in `PhysicsWorld` is not implemented; per-body buoyancy via `add_buoyancy(id, volume, drag)` (`:1413`) IS implemented. |
| `EnvironmentManager::WaterVolume` + `WaterVolumeId` | `environment.rs` | Active (module-level) | Volume-based water system exists in environment module independent of the stub. |
| `astraweave-physics::ecs::PhysicsPlugin` (`ecs` feature) | `ecs.rs` (84 LoC) | Dormant | Feature-gated; no production consumer enables `ecs` feature. The plugin is the designed ECS integration path. |
| `astraweave-gameplay::combat_physics::perform_attack_sweep` (direct `&mut PhysicsWorld`) | `astraweave-gameplay/src/combat_physics.rs` | Active | Bypasses ECS integration; takes raw `&mut PhysicsWorld` argument. The canonical production combat path. |
| `astraweave-physics::AsyncPhysicsScheduler` (`async-physics` feature) | `async_scheduler.rs` (979 LoC) | Dormant | Feature-gated; only the bench `physics_async` enables it (`Cargo.toml:55`). |
| Sync `PhysicsWorld::step` via Rapier's serial pipeline | `lib.rs:1039-1066, :1070-1106` | Active | Default path. Rapier may internally parallelize island solving if Rayon's global thread pool is available, but `enable_async_physics` is required to configure thread count and record `PhysicsStepProfile`. |

### Naming collisions

- **`broad_phase`**: In `PhysicsWorld.broad_phase: DefaultBroadPhase` (`lib.rs:907`), means Rapier's broadphase. In `spatial_hash.rs` doc-comment, refers to the dormant in-crate `SpatialHash` advertised as "broad-phase collision optimization." Two different things in the same crate.
- **`step` / `step_internal`**: `step` is the public entry point (`lib.rs:1039`); `step_internal` is the private body (`:1070`). `step` chooses between async and sync paths.
- **Wind**: `PhysicsWorld.wind: Vec3` is a bare global. `EnvironmentManager::WindZone` is zonal. Both exist in the same crate.
- **`add_water_aabb` vs `add_buoyancy`**: `add_water_aabb` (`lib.rs:1449`) is a body-less stub. `add_buoyancy(body, volume, drag)` (`lib.rs:1413`) is the actual buoyancy attachment. Don't expect the former to do anything.
- **`break_destructible` (`PhysicsWorld`) vs `DestructionManager`**: Despite the name, `PhysicsWorld::break_destructible` is a wholesale-removal helper (deletes the body and cleans up internal maps). The richer fracture/debris system lives in `destruction.rs` and is independent.
- **`Layers` (bitflags, in-crate) vs Rapier's `Group`**: `Layers::DEFAULT | Layers::CHARACTER` is the engine-side bitset (`lib.rs:383-389`). Rapier's `InteractionGroups::new(Group::from_bits_truncate(layers.bits()), Group::ALL)` is the per-collider filter. Conversion happens at collider construction time inside `add_*` methods.
- **Two `RigidBodyHandle ↔ BodyId` maps**: `body_ids: HashMap<RigidBodyHandle, BodyId>` and `body_kinds: HashMap<RigidBodyHandle, ActorKind>` are parallel. The first translates handles to engine IDs; the second tracks actor kind per handle. Both have the same key set after `tag_body`.

### Known cognitive traps

- **Trap**: Reading the crate doc-comment at `lib.rs:25-26` ("`SpatialHash` — Grid-based broadphase for O(n log n) collision culling (99.96% pair reduction vs brute-force)") and assuming `PhysicsWorld` uses it.
  **What's actually true**: `PhysicsWorld` uses Rapier's `DefaultBroadPhase` (`lib.rs:907`). `SpatialHash` is a standalone data structure with its own tests but no production consumers. Verified 2026-05-12.

- **Trap**: Calling `add_water_aabb(min, max, density, linear_damp)` expecting it to enable water-volume buoyancy.
  **What's actually true**: The function body is `{}` (`lib.rs:1449`); all parameters are underscored. Use `add_buoyancy(body, volume, drag)` (`:1413`) for per-body buoyancy. Volume-based water requires either `EnvironmentManager::WaterVolume` or a custom integration.

- **Trap**: Reading `add_destructible_box(pos, half, mass, health, break_impulse)` and assuming `health` / `break_impulse` are tracked.
  **What's actually true**: `_health` and `_break_impulse` are underscored (`lib.rs:1556-1565`). The function delegates to `add_dynamic_box` and returns the same `BodyId`. The richer destruction system lives in `destruction.rs` independently.

- **Trap**: Calling `physics_world.raycast(...)` BEFORE `physics_world.step()` and expecting it to reflect post-step geometry.
  **What's actually true**: `step_internal` updates the query pipeline at `lib.rs:1105` (`self.query_pipeline.update(&self.colliders)`) AFTER Rapier's step. Raycasts before `step()` see geometry from the previous frame's post-step state. This was the bug fixed in "Week 2 Day 3" per the inline comment at `:1102-1104`.

- **Trap**: Treating `PhysicsWorld::break_destructible(id)` as just a "mark for destruction" hook.
  **What's actually true**: It immediately removes the body from Rapier's set + `body_ids` + `body_kinds` + `char_map` + `buoyancy_bodies` (`lib.rs:1566-1584`). Caller code that holds the `BodyId` after this call gets `None` from `handle_of(id)` on subsequent queries.

- **Trap**: Enabling the `ecs` feature on `astraweave-physics` and expecting it to wire automatically with `astraweave-ecs`.
  **What's actually true**: The `ecs` feature exists in `Cargo.toml:13` (`ecs = ["astraweave-ecs", "astraweave-scene"]`) and provides `PhysicsPlugin` (`ecs.rs:11-23`), but no production crate enables it. Production consumers (`astraweave-gameplay`, `astraweave-npc`) take `&mut PhysicsWorld` directly and call `step()` themselves.

- **Trap**: Calling `control_character(id, ..., dt, ...)` with a variable `dt`.
  **What's actually true**: The dt parameter IS used internally — `ctrl.jump_buffer_timer -= dt` (`lib.rs:1263`) and `ctrl.vertical_velocity -= 9.81 * ctrl.gravity_scale * dt` (`:1267`) — but `astraweave-npc/src/runtime.rs:33-34` hardcodes `dt = 1.0/60.0` regardless of variable game-loop dt. The function does not enforce or assume a fixed timestep at the call site.

- **Trap**: Reading `PhysicsWorld.wind: Vec3` (`lib.rs:923`) and assuming it integrates with `EnvironmentManager`.
  **What's actually true**: `PhysicsWorld.wind` is a bare global wind vector with no automatic integration into the step. `EnvironmentManager::WindZone` is a separate richer system. Neither feeds into the other automatically.

---

## 7. Decision Log

### Decision: Wrap Rapier3D 0.22 behind `PhysicsWorld` rather than expose it directly
- **Date:** [Reasoning not recovered from available sources]
- **Status:** Accepted
- **Context:** Rapier exposes lifetime-heavy handles (`RigidBodyHandle`) and nalgebra vector types directly. Game code prefers stable opaque IDs and glam vectors.
- **Decision:** Define `BodyId: u64` as the engine-stable handle (`lib.rs:184`); maintain `body_ids: HashMap<RigidBodyHandle, BodyId>` and `body_kinds: HashMap<RigidBodyHandle, ActorKind>` (`lib.rs:914-915`); convert nalgebra ↔ glam at API boundaries via the `vector!`/`point!` macros (`:108`).
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** Every `add_*` method translates parameters from glam to nalgebra; every accessor (`get_velocity`, `raycast`) translates back. Stable u64 handles survive across hot-reload + serialization paths.

### Decision: Use `#![forbid(unsafe_code)]` at crate level
- **Date:** [Reasoning not recovered]
- **Status:** Accepted
- **Context:** Engine-wide policy per CLAUDE.md is that all crates except the foundation (`ecs`, `math`, `core`, `sdk`) should forbid unsafe code; unsafe lives only in vetted, Miri-validated layers.
- **Decision:** Declare `#![forbid(unsafe_code)]` at `lib.rs:1`. All unsafe code is upstream in `rapier3d` and its dependencies.
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** No SIMD intrinsics or raw FFI in `astraweave-physics`. Performance optimization must use safe Rust patterns + glam auto-vectorization + Rayon parallelism.

### Decision: Update `query_pipeline` AFTER `pipeline.step` inside `step_internal`
- **Date:** "Week 2 Day 3" per inline comment at `lib.rs:1102-1104` [no absolute date recovered]
- **Status:** Accepted
- **Context:** Per inline comment: "Without this, raycasts in `control_character()` use stale geometry, causing character controller to fail ground detection."
- **Decision:** Call `self.query_pipeline.update(&self.colliders)` at `lib.rs:1105` AFTER `self.pipeline.step(...)`.
- **Alternatives considered:** Update query pipeline before step (rejected — would have stale-by-one-tick raycast). Skip update entirely (rejected — caused the bug this fix addresses).
- **Consequences:** Every `step()` call performs the query pipeline update, adding minor cost but ensuring raycasts within the same tick (e.g. in `control_character`'s raycast slide logic at `:1291-1298`) see post-step geometry.

### Decision: `step_internal` applies buoyancy forces BEFORE Rapier's step
- **Date:** [Reasoning not recovered]
- **Status:** Accepted
- **Context:** Buoyancy is a per-body force computed from `buoyancy_bodies: HashMap<BodyId, BuoyancyData>` and the water level. Rapier's integration step needs these forces as inputs.
- **Decision:** Call `self.apply_buoyancy_forces()` at `lib.rs:1084` before `self.pipeline.step(...)` at `:1086`.
- **Alternatives considered:** Apply after step (rejected — forces would be effectively delayed by one tick).
- **Consequences:** Buoyancy is integrated together with all other forces in the same Rapier solve.

### Decision: Provide `add_dynamic_box`, `add_static_trimesh`, `add_character`, `create_ground_plane`, `add_destructible_box` as primary body-add methods
- **Date:** [Reasoning not recovered]
- **Status:** Accepted
- **Context:** Game code rarely needs arbitrary shape combinations; common shapes are box, trimesh, character capsule, ground plane.
- **Decision:** Offer typed convenience constructors (`add_dynamic_box(pos, half, mass, groups) -> BodyId`, etc.) that wrap Rapier's `RigidBodyBuilder` + `ColliderBuilder` chains and call `self.tag_body(handle, ActorKind::X)`.
- **Alternatives considered:** [Reasoning not recovered — could have exposed Rapier's builders directly]
- **Consequences:** Common cases are one-line; uncommon shapes require dropping to Rapier's builders or extending the API. `add_destructible_box` reflects this evolved-over-time pattern by being a passthrough (`:1556-1565`).

### Decision: Three feature gates (`async-physics`, `profiling`, `ecs`) for optional capabilities
- **Date:** [Reasoning not recovered]
- **Status:** Accepted
- **Context:** Rayon (for async physics), Tracy (for profiling), and `astraweave-ecs` + `astraweave-scene` (for ECS integration) are heavy dependencies that not all consumers need.
- **Decision:** Gate behind Cargo features: `async-physics = ["rayon"]` (`Cargo.toml:9`), `profiling = [...]` (`:10`), `ecs = ["astraweave-ecs", "astraweave-scene"]` (`:13`).
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** Default build is lean (Rapier + glam + bitflags + rustc-hash). None of the three features are enabled by any production consumer today (verified 2026-05-12).

### Decision: Use `rustc-hash::FxHashMap` (via `rustc-hash = "2.0"` dep at `Cargo.toml:35`) inside `SpatialHash`
- **Date:** "Phase B optimization" per `Cargo.toml:35` comment
- **Status:** Accepted (in `SpatialHash`, which is currently dormant)
- **Context:** Per inline `Cargo.toml:35` comment: "Phase B optimization: FxHashMap provides better performance (3.77ms vs 5.61ms with Tracy)"
- **Decision:** Depend on `rustc-hash` for the in-crate `SpatialHash` data structure.
- **Alternatives considered:** Default `std::collections::HashMap` (slower per the comment).
- **Consequences:** ~30% broadphase performance improvement in benchmarks. Note: because `SpatialHash` is dormant in production (see §6), this optimization is realized only in tests + benches.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `BodyId(0)` is never returned from `add_*` methods (allocator starts at 1) | Yes (code) | `lib.rs:954, 996`: `next_body_id: 1` initial; `:1033-1037` allocator increments after issue |
| 2 | `JointId(0)` is returned on `add_joint` failure (missing body) | Yes (code) | `lib.rs:1606-1611`: `Some(handle) else { return JointId(0); }` for both bodies |
| 3 | `step_internal` always calls `query_pipeline.update` AFTER `pipeline.step` | Yes (code) | `lib.rs:1086-1105`: explicit ordering with inline comment |
| 4 | `apply_buoyancy_forces` runs BEFORE `pipeline.step` | Yes (code) | `lib.rs:1084` precedes `:1086` |
| 5 | `add_character` always creates a `RigidBodyType::KinematicPositionBased` body, NOT dynamic | Yes (code) | `lib.rs:1206`: `RigidBodyBuilder::kinematic_position_based()` |
| 6 | `add_character` always inserts a `CharacterController` entry into `char_map` keyed by the new `BodyId` | Yes (code) | `lib.rs:1218-1235`: `self.char_map.insert(id, CharacterController { ... })` |
| 7 | `break_destructible(id)` removes the body from ALL internal maps (`body_ids`, `body_kinds`, `char_map`, `buoyancy_bodies`) | Yes (code) | `lib.rs:1566-1584`: explicit 4-line cleanup after Rapier `bodies.remove` |
| 8 | `PhysicsConfig::default()` time_step = `1.0/60.0` (60 Hz target) | Yes (code) | `lib.rs:618` |
| 9 | `PhysicsConfig::default()` gravity = `(0.0, -9.81, 0.0)` (Earth-like) | Yes (code) | `lib.rs:615` |
| 10 | `PhysicsConfig::default()` water_level = `f32::NEG_INFINITY` (water disabled by default) | Yes (code) | `lib.rs:619` |
| 11 | `PhysicsConfig::default()` ccd_enabled = `false`, max_ccd_substeps = `1` | Yes (code) | `lib.rs:616-617` |
| 12 | `Layers::DEFAULT = 0b01`, `Layers::CHARACTER = 0b10` (bitflags, distinct bits) | Yes (code) | `lib.rs:385-388`: `bitflags!` macro defines disjoint bits |
| 13 | `ActorKind` is `#[non_exhaustive]` with exactly 4 variants: Static / Dynamic / Character / Other | Yes (code) | `lib.rs:186-193`: `#[non_exhaustive] pub enum ActorKind { ... }` |
| 14 | `CharState` is `#[non_exhaustive]` with exactly 1 variant: Grounded | Yes (code) | `lib.rs:391-395` |
| 15 | `JointType` is `#[non_exhaustive]` with exactly 4 variants: Fixed / Revolute / Prismatic / Spherical | Yes (code) | `lib.rs:635-648` |
| 16 | `ProjectileKind` is `#[non_exhaustive]` with exactly 2 variants: Hitscan / Kinematic; default = Kinematic | Yes (code) | `projectile.rs:43-52` |
| 17 | `add_water_aabb` is a no-op stub (function body is `{}`) | Yes (code) | `lib.rs:1449`: `pub fn add_water_aabb(&mut self, _min: Vec3, _max: Vec3, _density: f32, _linear_damp: f32) {}` |
| 18 | `PhysicsWorld` is `Send + Sync` (auto-derived) — must hold for use as `astraweave-ecs::World` resource | Yes (compile-time, transitively proven) | Verified 2026-05-12: `astraweave-ecs::World::get_resource<T>` / `insert_resource<T>` require `T: 'static + Send + Sync` (per `astraweave-ecs/src/lib.rs:466, 482, 486`). Multiple call sites compile: `astraweave-scripting/src/lib.rs:148, 336, 453, 506` (`world.get_resource::<PhysicsWorld>()`) and `astraweave-physics/tests/ecs_integration_test.rs:20` (`app.world.get_resource_mut::<PhysicsWorld>().unwrap()`). Therefore `PhysicsWorld` MUST be `Send + Sync`. The previous trace claim "Send but NOT Sync" was incorrect. |
| 19 | `process_destructible_hits` is a no-op stub with `#[allow(dead_code)]` | Yes (code) | `lib.rs:1586-1587`: `#[allow(dead_code)] fn process_destructible_hits(&mut self) {}` |
| 20 | Crate uses `#![forbid(unsafe_code)]`; no unsafe blocks in any of the 12 source files | Yes (compile-time + grep) | `lib.rs:1`; workspace grep within `astraweave-physics/src/*.rs` for `unsafe` returned no matches outside `forbid_unsafe_code` itself (verified 2026-05-12 indirectly via the absence of `unsafe` in surveyed files) |

---

## 9. Performance & Resource Profile

### Hot paths

| Path | Frequency | Budget | Sensitivity |
|---|---|---|---|
| `PhysicsWorld::step` | 60 Hz (default) or game-loop tick rate | ~6.52 µs per tick on baseline workload (per `lib.rs:46-48` doc-comment) | Body count (Rapier scales O(n log n) via broadphase); contact pair count drives narrowphase; CCD substeps multiply cost when enabled |
| `PhysicsWorld::control_character` | 60 Hz × number of characters | ~114 ns per call (per `lib.rs:45` doc-comment) | Raycast cost for obstacle slide; ground detection |
| `PhysicsWorld::raycast` | On-demand (combat sweeps, NPC perception) | Single query: sub-µs typical, bounded by collider count | Long rays + dense scenes increase cost linearly |
| `apply_buoyancy_forces` | Every `step` (60 Hz) | Linear in `buoyancy_bodies.len()` | Each body adds a `Vec3` force; HashMap iteration cost |
| `query_pipeline.update` | Every `step` after Rapier step | Linear in collider count | Cost amortized into the step budget per the "Week 2 Day 3 fix" |
| `RigidBodyBuilder::*` in `add_*` methods | At spawn time | Per-call, not in hot loop | Allocator-bound; per-body HashMap inserts |

Per the crate-level doc-comment at `lib.rs:43-48`:
- Character move: **114 ns**
- Full physics tick: **6.52 µs**
- Rigid body step: **2.97 µs**
- Spatial hash: **3.77 ms** (FxHashMap, vs 5.61 ms SipHash) — note this benchmark is on the dormant `SpatialHash` module, not the production broadphase

### Cold paths

| Path | Frequency | Budget |
|---|---|---|
| `add_dynamic_box` / `add_static_trimesh` / `add_character` | At spawn time | Looser budget; can afford HashMap inserts and Rapier builder chains |
| `break_destructible` | On destruction events | Looser budget; one body removal + 4 HashMap cleanups |
| `add_joint` | At spawn time | Single `ImpulseJointSet::insert` after a `match joint_type` chain |
| `enable_ccd(id)` | On-demand toggle | Single `rb.enable_ccd(true)` |
| `set_body_position(id, pos)` | Editor-driven or scripted | Single `rb.set_translation` |
| `add_buoyancy` | At spawn time | Single `buoyancy_bodies.insert` |

### Resource ownership

| Resource | Owner | Lifetime | Access pattern |
|---|---|---|---|
| `RigidBodySet`, `ColliderSet`, `ImpulseJointSet`, `MultibodyJointSet` | `PhysicsWorld` (public fields) | `PhysicsWorld` lifetime | Mutated only during `step`, `add_*`, `break_destructible`, `set_*` |
| `PhysicsPipeline`, `IslandManager`, `DefaultBroadPhase`, `NarrowPhase`, `QueryPipeline`, `CCDSolver` | `PhysicsWorld` (public fields) | `PhysicsWorld` lifetime | Mutated during `step` |
| `ChannelEventCollector` + crossbeam channels (`collision_recv`, `contact_force_recv`) | `PhysicsWorld` | `PhysicsWorld` lifetime | Events emitted during `step`; caller drains via `try_recv` / `recv` |
| `body_ids`, `body_kinds`, `char_map`, `buoyancy_bodies` HashMaps | `PhysicsWorld` (private + public) | `PhysicsWorld` lifetime | Inserted on `add_*`, removed on `break_destructible` |
| `DebugRenderPipeline` | `PhysicsWorld` (private field) | `PhysicsWorld` lifetime | Used by debug-render API; otherwise idle |

### Allocation profile

Per inline `lib.rs:1071-1075`: "Allocation-measurement instrumentation (audit 2026-04-17, open question #7). `physics.step.allocs` exposes whether Rapier3D's per-step allocation count grows with simulation time — the audit could not answer this statically." Verified 2026-05-12: the `alloc-counter` feature is declared at `Cargo.toml:17` (`alloc-counter = ["dep:astraweave-profiling", "astraweave-profiling/alloc-counter"]`) and the `alloc_measure` bench is gated on it at `Cargo.toml:77-80` (`[[bench]] name = "alloc_measure" / required-features = ["alloc-counter"]`). The crate also declares a `fast-alloc` feature at `Cargo.toml:20` (`fast-alloc = ["dep:astraweave-alloc", "astraweave-alloc/fast-alloc"]`) for bench-only mimalloc allocator swap (per inline `Cargo.toml:18-19` comment "Does nothing to the library itself").

---

## 10. Testing & Validation

- **Unit tests:** Inline `#[cfg(test)]` modules in every source file (e.g. `lib.rs:2980-…` `control_character_applies_gravity_when_not_climbing`, etc.). 1,259 LoC of dedicated `mutation_tests.rs` inside `src/`.
- **Integration tests:** 20 files in `astraweave-physics/tests/`:
  - `behavioral_correctness_tests`, `buoyancy_test`, `coverage_boost_tests`, `cross_subsystem_validation`, `debug_render_test`, `determinism`, `ecs_integration_test`, `environment_tests`, `gravity_tests`, `mutation_resistant_comprehensive_tests`, `mutation_resistant_tests`, `nan_infinity_tests`, `panic_safety_tests`, `phase1_verification`, `physics_core_tests`, `physics_laws_tests`, `projectile_tests`, `ragdoll_tests`, `spatial_hash_character_tests`, `vehicle_tests`
- **Mutation testing:** Two mutation-resistant test files (`mutation_resistant_tests.rs`, `mutation_resistant_comprehensive_tests.rs`) plus the inline `src/mutation_tests.rs` (1,259 LoC). Status: included in workspace-wide mutation-testing campaign per CLAUDE.md (specific coverage stats out of scope for this trace).
- **Miri validation:** Not directly applicable — the crate uses `#![forbid(unsafe_code)]` so no unsafe blocks exist locally. Miri validation of upstream Rapier3D is out of scope.
- **Benchmarks:** 10 files in `astraweave-physics/benches/`:
  - `alloc_measure` (allocation profiling)
  - `character_controller`, `cloth`, `destruction`, `gravity`, `ragdoll`, `raycast`, `rigid_body`, `vehicle` (per-subsystem)
  - `physics_async` (`required-features = ["async-physics"]`, the only consumer of the async feature)
- **NaN/Infinity safety tests:** `tests/nan_infinity_tests.rs` exercises pathological inputs to verify no panics or NaN-propagation through the API
- **Panic safety tests:** `tests/panic_safety_tests.rs` exercises invalid `BodyId` / `JointId` cases to verify graceful failure (no panics)
- **Determinism tests:** `tests/determinism.rs` exercises reproducibility (same seed + same inputs → same outputs)
- **Cross-subsystem validation:** `tests/cross_subsystem_validation.rs` exercises interactions between subsystems (e.g. projectile + character)
- **Manual validation:** Examples (`physics_demo3d`, `combat_physics_demo`, `npc_town_demo`, `veilweaver_demo`, etc.) provide visual playthrough validation

---

## 11. Open Questions / Parked Decisions

- **`SpatialHash` module — wire in or remove?** [Decisional / factual, **enriched 2026-05-12 deep investigation**.] Factual state (re-verified 2026-05-12): the `spatial_hash.rs` module (1,038 LoC) defines a complete grid-broadphase data structure with `AABB` + `SpatialHash<T>` and is advertised in the crate-level doc-comment at `lib.rs:25-26` as the broadphase. Workspace grep for `use astraweave_physics::SpatialHash` or `use astraweave_physics::spatial_hash` outside the crate's own `src` + `tests` returned zero matches. `PhysicsWorld` uses Rapier's `DefaultBroadPhase` (`lib.rs:907`). **Test surface is more extensive than previously documented:** 4 test files reference `SpatialHash` — `tests/behavioral_correctness_tests.rs` (3+ tests including `test_spatial_hash_no_false_negatives` at `:652`, `test_spatial_hash_no_false_positives` at `:684`, `test_mutation_spatial_hash_cell_boundary` at `:996`), `tests/coverage_boost_tests.rs:868-…` (`spatial_hash_coverage` mod with multiple tests), `tests/cross_subsystem_validation.rs:13, :280-…` (including `test_spatial_hash_with_multiple_physics_objects` and `test_spatial_hash_determinism`), and `tests/spatial_hash_character_tests.rs` (33 `#[test]` attributes total, mixed with character_controller tests — first ~20 are SpatialHash + AABB tests at lines `:18-:240`). **No dedicated benchmark file:** workspace grep for `SpatialHash\|spatial_hash` in `benches/*.rs` returned zero matches; the previous trace claim about `benches/raycast.rs` mentioning spatial-hash patterns was incorrect (`grep -c "SpatialHash\|spatial_hash" benches/raycast.rs` returned `0`). The "3.77 ms vs 5.61 ms" comparison cited at `lib.rs:48` and `Cargo.toml:32` was a Phase B measurement preserved in code comments, not an ongoing bench harness. Whether to wire `SpatialHash` into `PhysicsWorld` (replacing or supplementing `DefaultBroadPhase`), expose it as a public utility for external broadphase needs, or remove it is undecided. The doc-comment claim "99.96% pair reduction vs brute-force" is sourced from the Phase B comment, not from a currently-running bench.
- **`async-physics` feature — production-wire or experimental?** [Decisional.] Factual: the `async-physics` feature gates `AsyncPhysicsScheduler` + `PhysicsStepProfile` + `enable_async_physics` (`lib.rs:1010-1031`). Verified 2026-05-12: only `astraweave-physics/Cargo.toml:55` (the `physics_async` bench) enables it. No production consumer sets `features = ["async-physics"]`. The scheduler integrates by recording telemetry inside the existing `step()` path; Rapier itself uses Rayon's global thread pool when available (per inline comment at `lib.rs:1045-1047`). Whether to enable the feature by default, make it on-by-default opt-out, or keep as opt-in experimental is undecided.
- **`ecs` feature — production-wire `PhysicsPlugin` or rely on direct `&mut PhysicsWorld` passthrough?** [Decisional, **enriched 2026-05-12 deep investigation**.] Factual: `PhysicsPlugin` exists in `ecs.rs:11-23` and registers `physics_step_system` + `sync_physics_to_transform_system` on `SystemStage::PHYSICS`. Verified 2026-05-12: no production consumer enables `features = ["ecs"]` (workspace grep for `astraweave-physics.*features.*ecs` in `.toml` files returned zero matches outside the physics crate's own declaration at `Cargo.toml:11`). **Three de facto consumer patterns exist today:** (a) Direct `&mut PhysicsWorld` passthrough — `astraweave-gameplay::combat_physics::perform_attack_sweep` (`combat_physics.rs:36`), `astraweave-npc::NpcManager` (`runtime.rs:21`). (b) ECS-resource pattern WITHOUT the `ecs` feature — `astraweave-scripting/src/lib.rs:148, 336, 453, 506` uses `World::insert_resource(physics)` + `get_resource::<PhysicsWorld>()` which works because `World::*_resource<T>` only requires `T: 'static + Send + Sync`, NOT the physics-side `ecs` feature. (c) The designed `PhysicsPlugin` path — gated on the `ecs` feature, exercised only by `tests/ecs_integration_test.rs:1` (which declares `#[cfg(feature = "ecs")]`). The decisional question is whether to migrate (a) and (b) onto (c), keep all three patterns in parallel, or formalize (b) as a documented third pattern.
- **`add_water_aabb` stub — implement or remove?** [Decisional / factual.] Factual: `lib.rs:1449` defines `pub fn add_water_aabb(&mut self, _min: Vec3, _max: Vec3, _density: f32, _linear_damp: f32) {}` — all parameters underscored, function body is `{}`. The per-body `add_buoyancy` (`:1413`) IS implemented. `EnvironmentManager::WaterVolume` (`environment.rs`) exists as a parallel volume-based system. Whether to implement `add_water_aabb` (integrating with `EnvironmentManager::WaterVolume`), document the stub status, or remove the function entirely is undecided.
- **`add_destructible_box` — passthrough or distinct?** [Decisional / factual.] Factual: `lib.rs:1556-1565` defines `add_destructible_box(pos, half, mass, _health, _break_impulse) -> BodyId` that ignores `_health` + `_break_impulse` and delegates to `add_dynamic_box`. The richer `DestructionManager` system (`destruction.rs`, 2,788 LoC) is independent. Whether to make `add_destructible_box` actually integrate with `DestructionManager`, document the passthrough behavior, or remove the function is undecided.
- **`PhysicsWorld.wind: Vec3` vs `EnvironmentManager::WindZone` — unify or document parallel intent?** [Decisional / factual.] Factual: `PhysicsWorld.wind: Vec3` (`lib.rs:923`) is a bare global wind vector. `EnvironmentManager::WindZone` (`environment.rs`, 3,401 LoC) is a zonal wind system with shape + type variants. The two are not integrated. Whether to drive `PhysicsWorld.wind` from `EnvironmentManager`, keep them parallel, or remove the bare field is undecided.
- **`PhysicsWorld.gravity` (single `Vector<Real>`) vs `GravityManager` (zonal + point) — unify or document parallel intent?** [Decisional / factual.] Same shape as the wind question above. `PhysicsWorld.gravity` is what Rapier consumes each step. `GravityManager` (`gravity.rs`, 1,425 LoC) computes per-body gravity overrides but is not wired into `PhysicsWorld::step`. Caller must drive `GravityManager` separately and call `PhysicsWorld::apply_force` per body.
<!-- Question "process_destructible_hits no-op — vestigial?" closed via deep investigation 2026-05-12. Resolution: workspace grep across the entire repo returned exactly one match — the definition itself at `astraweave-physics/src/lib.rs:1587`. Zero callers anywhere (including no production crates, no tests, no benches). The `#[allow(dead_code)]` annotation is correct; the function is a complete vestige. Captured as §6 Conflict Map "Stub functions" note + remains in §8 Invariant 19 (stub function exists and is annotated). Whether to remove the function entirely is decisional. -->
<!-- Question "Send + Sync story for PhysicsWorld" closed via deep investigation 2026-05-12. Resolution: PhysicsWorld is `Send + Sync` (auto-derived) — compile-time-proven by multiple call sites including `astraweave-scripting/src/lib.rs:148, 336, 453, 506` and `astraweave-physics/tests/ecs_integration_test.rs:20`, all of which invoke `World::get_resource<PhysicsWorld>` / `get_resource_mut` (which require `T: 'static + Send + Sync` per `astraweave-ecs/src/lib.rs:466, 482, 486`). Resolution captured in §8 Invariant 18. -->

- **`astraweave-physics::ecs::PhysicsBodyComponent.0` is `BodyId` (`u64`) wrapped in tuple struct — match `astraweave-ecs` patterns?** [Factual / decisional.] `ecs.rs:8` defines `pub struct PhysicsBodyComponent(pub BodyId)`. The pub-tuple-struct pattern is consistent with simple ECS component conventions. Whether to add helper methods, `Display` impl, or migrate to a richer component shape is undecided.
- **Per-step allocation count growth over simulation time?** [Factual / observable, **already documented in inline comment**.] `lib.rs:1071-1075`: "Audit 2026-04-17, open question #7. `physics.step.allocs` exposes whether Rapier3D's per-step allocation count grows with simulation time — the audit could not answer this statically." The `alloc-counter` feature + `alloc_measure` bench (`Cargo.toml:22-24`) instrument this. Whether the per-step allocation count is bounded (zero growth) or grows over time is empirically observable but [NEEDS VERIFICATION — bench results not surveyed in this pass].

---

## 12. Maintenance Notes

**Update this doc when:**
- A new variant is added to any `#[non_exhaustive]` enum: `ActorKind`, `CharState`, `JointType`, `ProjectileKind`, `GravityZoneShape`, `BoneShape`, `WindZoneShape`, `WindType`, `DebrisShape` (touch §3 vocabulary + §8 invariants)
- A new `add_*` body-creation method is added to `PhysicsWorld` (touch §4 upstream interfaces + §5 file map + §8 invariants if a new actor-kind tagging behavior is introduced)
- The order of operations inside `step_internal` changes (touch §2.1 pipeline + §7 decision log if rationale changes)
- A new feature gate is added to `Cargo.toml` (touch §5 file map status + §11 if production wiring is in question)
- `SpatialHash` is wired into `PhysicsWorld` or removed (touch §5 status + §6 conflict map + §11 closure)
- `async-physics` or `ecs` features gain production consumers (touch §5 status + §11 closure)
- A new consumer crate adds an `astraweave-physics` dep (touch §4 downstream table + §5 file map)
- `process_destructible_hits`, `add_water_aabb`, or other stub methods are implemented or removed (touch §6 stub list + §8 invariants)
- Rapier3D is upgraded to a new major version (touch §1 status + crate-level doc-comment + every public re-export at `lib.rs:53-103`)

**Verification process:**
- Spot-check §2 pipelines against `step`, `step_internal`, `control_character`, `raycast`, `add_*` in `lib.rs`
- Verify the file map in §5 against `Cargo.toml` + the `pub mod` declarations in `lib.rs`
- Verify invariants in §8 against the cited line numbers
- Run `cargo test -p astraweave-physics --tests` (default features) and `cargo test -p astraweave-physics --tests --features async-physics,profiling,ecs` for the feature-gated suite
- Run `cargo bench -p astraweave-physics` for performance regression detection
- Update the metadata commit hash and date

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**

1. **`PhysicsWorld` is the single mutable owner.** All add/remove/step/query operations require `&mut PhysicsWorld`. Game code stores `BodyId: u64` (NOT Rapier `RigidBodyHandle`).
2. **`step()` always updates `query_pipeline` AFTER Rapier's step.** Raycasts within the same tick see post-step geometry (per `lib.rs:1102-1104` "Week 2 Day 3" fix).
3. **`SpatialHash` is dormant.** The crate doc-comment advertises it as the broadphase but `PhysicsWorld` uses Rapier's `DefaultBroadPhase`. Do not assume `SpatialHash` is in the live path.
4. **`add_water_aabb`, `process_destructible_hits`, and the `_health`/`_break_impulse` params of `add_destructible_box` are stubs** — calling them is harmless but does nothing.
5. **Per-body `add_buoyancy` works.** Volume-based water buoyancy in `PhysicsWorld` is not implemented; for richer water you need `EnvironmentManager::WaterVolume` from `environment.rs`.
6. **`PhysicsWorld.wind: Vec3` and `EnvironmentManager::WindZone` are parallel systems**, not unified. Same for `PhysicsWorld.gravity` vs `GravityManager`.
7. **All AI/game-loop callers pass fixed `dt = 1.0/60.0` to `control_character`.** The function uses `dt` internally for timer + gravity updates, but `astraweave-npc/src/runtime.rs:33-34` hardcodes the value. Variable-dt callers must call themselves at game-tick rate.
8. **All event channels (`collision_recv`, `contact_force_recv`) are unbounded.** Unconsumed events accumulate in memory; drain them every tick.
9. **`#![forbid(unsafe_code)]` is enforced at crate level.** No SIMD intrinsics or raw FFI here; rely on glam auto-vectorization + Rapier's internal optimizations.
10. **All `#[non_exhaustive]` enums are forward-compatible.** Adding a variant within the crate works; external matches must include a wildcard arm. Adding a variant to `ActorKind`, `CharState`, `JointType`, `ProjectileKind`, `BoneShape`, `WindZoneShape`, `WindType`, `GravityZoneShape`, or `DebrisShape` requires touching dispatch + dependents per CLAUDE.md Integration Completeness #2.
11. **`break_destructible(id)` is wholesale.** It deletes the body from Rapier + 4 internal HashMaps. Subsequent `handle_of(id)` returns `None`.
12. **Five feature gates exist but none are production-wired today**: `async-physics`, `profiling`, `ecs`, `alloc-counter`, `fast-alloc`. The first three are designed (`AsyncPhysicsScheduler`, Tracy spans, `PhysicsPlugin`); the last two are bench-only (`alloc_measure` bench `required-features`, `fast-alloc` for benchmark allocator swap). Production wiring of any of them is undecided.

**Files you'll most likely touch:**

- `astraweave-physics/src/lib.rs` — `PhysicsWorld` + `CharacterController` + core APIs
- `astraweave-physics/src/projectile.rs` / `gravity.rs` / `ragdoll.rs` / `vehicle.rs` / `environment.rs` / `destruction.rs` / `cloth.rs` — per-subsystem managers
- `astraweave-gameplay/src/combat_physics.rs` — canonical production combat sweep using `&mut PhysicsWorld`

**Files you should NOT touch without strong reason:**

- `astraweave-physics/src/spatial_hash.rs` — dormant in production; wiring it in is an architectural decision (see §11)
- `astraweave-physics/src/async_scheduler.rs` — feature-gated and dormant
- `astraweave-physics/src/ecs.rs` — feature-gated and dormant; production uses direct `&mut PhysicsWorld`
- Rapier3D's `pub use` block at `lib.rs:53-103` — explicit re-exports; an upgrade of Rapier requires coordinated changes across the entire workspace

**Common mistakes when changing this system:**

- **Assuming `SpatialHash` is wired in.** It isn't. The lib.rs doc-comment is aspirational. Don't extend `SpatialHash` thinking it affects production performance.
- **Calling `add_water_aabb` and expecting buoyancy.** Use `add_buoyancy(body, volume, drag)` instead.
- **Forgetting to drain `collision_recv` + `contact_force_recv`.** Unbounded channels grow forever.
- **Adding a variant to a `#[non_exhaustive]` enum without checking dispatch.** Match arms in subsystems (e.g. `add_joint` matching `JointType`) need updates.
- **Trying to share `PhysicsWorld` across threads.** Even if `Send + Sync` auto-derive permits it, the API requires exclusive `&mut` access for the entire `step` + `add_*` + `set_*` surface.
- **Mixing `BodyId(0)` with valid IDs.** `BodyId(0)` is the "invalid" sentinel for `JointId` failures; treat zero-ID returns as errors.

---

## Appendix B: Historical context

The crate wraps Rapier3D — the de-facto Rust physics engine — to provide engine-stable handles and glam-native types. Two recurring patterns shape the trace:

1. **Subsystem proliferation pre-dating integration.** The crate contains seven feature-rich subsystems (projectile, gravity zones, ragdoll, vehicle, environment, destruction, cloth) each with its own manager type and rich enum vocabulary. Each was developed in parallel as a self-contained module with tests + benches, but most have not yet been wired into `PhysicsWorld`'s step cycle or into a production consumer's game loop. Per the documented performance numbers (114 ns character move, 6.52 µs step, etc.) the core has been profile-optimized; the subsystems await integration campaigns.

2. **In-house broadphase vs Rapier broadphase.** The `SpatialHash` module (1,038 LoC, FxHashMap-optimized per the `Cargo.toml:35` "Phase B" comment) was authored as an alternative or supplement to Rapier's broadphase. The crate-level doc-comment at `lib.rs:25-26` advertises it as the broadphase, but the actual implementation in `PhysicsWorld` uses `DefaultBroadPhase` from Rapier directly. Whether `SpatialHash` is intended as a future replacement, a supplemental high-coherency tier, or vestigial is recorded as an Open Question in §11.

3. **"Week 2 Day 3 fix" inline comment.** The `query_pipeline.update` call after `pipeline.step` (`lib.rs:1102-1105`) carries an explicit comment about the bug it fixed: stale raycast geometry in `control_character`. This is one of the few decisions in the crate with recoverable rationale; most other design choices are unrecovered.
