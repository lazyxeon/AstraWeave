# Architecture Trace: ECS Substrate + Math / Core / SDK Primitives

## Metadata

| Field | Value |
|---|---|
| **System name** | ECS substrate + Math / Core / SDK primitives (foundation layer) |
| **Primary crates** | `astraweave-ecs`, `astraweave-math`, `astraweave-core`, `astraweave-sdk` |
| **Document version** | 1.2 |
| **Last verified against commit** | `67c9de7e1` |
| **Last verified date** | 2026-05-10 |
| **Status** | Active foundation. `astraweave-ecs` and `astraweave-math` are canonical; `astraweave-core` contains a canonical schema **plus** a transitional legacy `World` that bridges to ECS; `astraweave-sdk` is a stable C ABI surface with a single in-tree consumer (`examples/sdk_c_harness`) and an otherwise out-of-tree audience. |
| **Owner notes** | This trace covers four related-but-distinct foundation crates as one document, per Andrew's request. Note: the doc is a foundation **layer**, not a single tightly-coupled system. Per-crate roles are kept distinct throughout. Deep investigation pass 2026-05-10 resolved the `lib_new.rs` status, corrected the SDK-consumer claim (one in-tree example exists), added the loom-tests + `simd`-feature-no-op findings, and refined Invariant 8 against the actual JSON-serializer code paths. |

---

## 1. Executive Summary

**What this system does:**
Provides the four foundation crates everything else builds on: an archetype-based deterministic single-threaded ECS substrate (`astraweave-ecs`), SIMD math primitives (`astraweave-math`), shared schema and a legacy `World` with ECS↔legacy bridging (`astraweave-core`), and a C ABI embedding surface (`astraweave-sdk`).

**Why it exists:**
Every other crate (AI, render, physics, gameplay, scene, terrain, persistence, networking, editor) depends on at least one of these for entity lifecycle, component storage, math operations, the canonical `WorldSnapshot`/`PlanIntent` schema used by AI prompts, or FFI embedding. These are the load-bearing primitives.

**Where it primarily lives:**
- `astraweave-ecs/src/` — archetype storage, entity allocator with generational indices, command buffer, events, schedule/app driver, system params, sparse set, blob vec, RNG, type registry
- `astraweave-math/src/` — SIMD vec/mat/quat/movement primitives over `glam`
- `astraweave-core/src/` — `WorldSnapshot` and related schema, legacy `World` struct, ECS adapter and bridge, ECS components mirroring legacy state, perception helpers, tool sandbox, validation, capture/replay
- `astraweave-sdk/src/` — C ABI: `aw_world_create/destroy/tick/snapshot_json/submit_intent_json`, version helpers, snapshot/delta callback registration

**Status note:**
Two structural facts every consumer must know up front:
1. **Two `World` types coexist.** `astraweave_ecs::World` is the archetype-based ECS substrate. `astraweave_core::World` is the legacy data-oriented struct (HashMap-per-component). They are bridged inside an `astraweave_ecs::App` by `astraweave_core::ecs_adapter::build_app` (`astraweave-core/src/ecs_adapter.rs:169-227`), which inserts the legacy World as a resource and runs `sys_sync_to_legacy` in the `sync` stage. The dual-World setup is transitional.
2. **Two `Entity` types coexist.** `astraweave_core::Entity = u32` (`astraweave-core/src/schema.rs:62`) and `astraweave_ecs::Entity { id: u32, generation: u32 }` (`astraweave-ecs/src/entity_allocator.rs:53-57`) are different types. `astraweave_core::ecs_bridge::EntityBridge` maps between them in both directions.

A prior `ParallelSchedule` parallel scheduler was removed from `astraweave-ecs` on 2026-04-18 (commit `617c14de8`). The engine is now deterministic single-threaded at the ECS layer; parallelism lives at the subsystem level (rayon in terrain/fluids, tokio for I/O, GPU compute in rendering). See `docs/audits/parallel_schedule_removal_2026-04-18.md`.

---

## 2. Authoritative Pipeline

There are several distinct flows through this layer. The first is the canonical tick driver; the second is the FFI surface; the third is the math primitives (stateless).

```text
[Application / example / editor / SDK]
    │
    │ App::new()  ┐
    ▼             │  registers eight stages
[astraweave_ecs::App]
    file: astraweave-ecs/src/lib.rs:758-812
    role: top-level driver: owns one ECS World + one Schedule
    │
    │ build_app(legacy_world, dt) inserts:
    │   - Dt resource, Events<MovedEvent>, EntityBridge
    │   - one ECS entity per legacy entity (mirroring CPos/CHealth/CTeam/CAmmo/CCooldowns)
    │   - the legacy World itself as a resource
    │   - sys_sim, sys_move (simulation stage), sys_bridge_sync, sys_sync_to_legacy (sync stage),
    │     sys_refresh_los (perception stage)
    ▼
[Stage execution: deterministic single-threaded, registration order]
    file: astraweave-ecs/src/lib.rs:700-735 (Schedule::run)
    stages (canonical, from App::new at astraweave-ecs/src/lib.rs:783-792):
        pre_simulation → perception → simulation → sync →
        ai_planning → physics → post_simulation → presentation
    │
    │ for each stage, for each system: fn(&mut World)
    ▼
[Component access via Query/Query2/Query2Mut OR World::each_mut/get/insert]
    file: astraweave-ecs/src/system_param.rs (Query types)
    file: astraweave-ecs/src/lib.rs:259-312 (insert), 404-447 (get/get_mut),
           509-532 (each_mut), 573-586 (each_changed)
    role: archetype-keyed component access; change-tick stamped on every mutation
    │
    │ structural changes via CommandBuffer (queued, flushed at safe point)
    │   file: astraweave-ecs/src/command_buffer.rs
    ▼
[Tick advance: schedule.run executes all stages once]
    role: deterministic; same inputs → same outputs across runs
```

```text
[C / C++ / C# host application]
    │
    │ aw_world_create() → AWWorld handle
    ▼
[astraweave_sdk::AwWorldWrap]
    file: astraweave-sdk/src/lib.rs:130-161
    role: owns an astraweave_core::World, snapshot/delta callbacks, reusable JSON buffers
    │
    │ aw_world_tick(handle, dt) advances world.tick(dt),
    │   computes SimpleSnapshot + SimpleDelta, invokes registered callbacks
    │   file: astraweave-sdk/src/lib.rs:175-241
    ▼
[Host receives NUL-terminated UTF-8 JSON via callback]
    or polls aw_world_snapshot_json(handle, buf, len)
    │
    │ host submits a PlanIntent JSON via aw_world_submit_intent_json
    │   file: astraweave-sdk/src/lib.rs:382-… (validate_and_execute)
    ▼
[astraweave_core::validation::validate_and_execute applies the intent to the legacy World]
    file: astraweave-core/src/validation.rs:10-…
```

```text
[Math primitives (stateless, called from any subsystem)]
    astraweave_math::simd_vec::{dot_simd, cross_simd, normalize_simd}    (Vec3/Vec4)
    astraweave_math::simd_mat::{mul_simd, transpose_simd, …}             (Mat4)
    astraweave_math::simd_quat::{mul_quat_simd, slerp_simd, …}           (Quat)
    astraweave_math::simd_movement::update_positions_simd                (batched p += v*dt)
    astraweave_math::enable_flush_to_zero()                              (one-shot at engine init)
```

### Stage-by-stage detail

#### Stage 1: ECS App / Schedule construction
**File:** `astraweave-ecs/src/lib.rs:758-812`
**Role:** Construct an `App` with the canonical eight-stage schedule.
**Inputs:** Caller calls `App::new()`.
**Outputs:** `App { world: World, schedule: Schedule }` with stages registered in canonical order.
**Notes:** Stage order is fixed by registration: `pre_simulation → perception → simulation → sync → ai_planning → physics → post_simulation → presentation` (`astraweave-ecs/src/lib.rs:783-792`). `Schedule::add_system` for an unregistered stage prints a debug-build warning and silently drops the system in release (`astraweave-ecs/src/lib.rs:708-725`) — this was added after the binary-inventory audit (see `docs/audits/parallel_schedule_binary_inventory_2026-04-18.md`).

#### Stage 2: World storage and entity lifecycle
**File:** `astraweave-ecs/src/lib.rs:106-685` (World), `astraweave-ecs/src/archetype.rs`, `astraweave-ecs/src/entity_allocator.rs`, `astraweave-ecs/src/blob_vec.rs`, `astraweave-ecs/src/sparse_set.rs`
**Role:** Archetype-based storage with generational entity indices.
**Inputs:** `World::spawn()`, `World::insert::<T>(e, c)`, `World::remove::<T>(e)`, `World::despawn(e)`.
**Outputs:** Components are placed in the archetype matching the entity's current signature; structural changes move the entity between archetypes (`astraweave-ecs/src/lib.rs:259-312` insert, `astraweave-ecs/src/lib.rs:315-387` `move_entity_to_new_archetype`).
**Notes:**
- Two storage modes coexist in each archetype: legacy `Vec<Box<dyn Any>>` (always available) and `BlobVec` (zero-indirection, requires `world.register_component::<T>()` and `T: Clone`). Mode is selected per archetype via the `uses_blob` flag (`astraweave-ecs/src/archetype.rs:66-99`).
- Stale entity handles are detected via the generation field; `insert`, `get`, `get_mut`, `remove`, `despawn` all check `entity_allocator.is_alive(entity)` and return / no-op on stale handles (`astraweave-ecs/src/lib.rs:261, 409, 437, 603, 641`).
- A monotonic `change_tick: u32` is stamped on every mutation and queried by `each_changed::<T>(since_tick, …)` (`astraweave-ecs/src/lib.rs:131-172, 573-586`).
- `CommandBuffer` allows deferred structural changes during iteration (`astraweave-ecs/src/command_buffer.rs`).

#### Stage 3: Component queries
**File:** `astraweave-ecs/src/system_param.rs`
**Role:** Read-only and mutable iteration over entities matching a component signature.
**Types exported:** `Query<T>`, `Query2<T1,T2>`, `Query2Mut<T1,T2>`, `SystemParam` (marker).
**Notes:** Each query call does `archetype.get::<T>(entity)` per entity (O(1) sparse-set + O(1) hashmap + O(1) Vec index + O(1) downcast). Module header documents the Week 10 SparseSet integration that brought entity lookup from O(log n) to O(1), with 9.4× speedup on the movement system (`astraweave-ecs/src/system_param.rs:1-83`).

#### Stage 4: Resources (singletons)
**File:** `astraweave-ecs/src/lib.rs:450-488`
**Role:** Single-instance values stored by `TypeId` on `World` (separate from per-entity components).
**API:** `world.insert_resource(r)`, `world.get_resource::<T>()`, `world.get_resource_mut::<T>()`.
**Notes:** This is how `astraweave_core::ecs_adapter::build_app` makes the legacy `World` available to systems running in the ECS schedule (`astraweave-core/src/ecs_adapter.rs:217`), and how the LLM/director crates pass per-tick state.

#### Stage 5: Events
**File:** `astraweave-ecs/src/events.rs`
**Role:** Typed event bus with frame-stamped entries, retained for a window for late readers.
**Types:** `Event` (marker trait), `Events<E>`, `EventReader<E>`, `EventEntry<E>`.
**Notes:** Used cross-subsystem — physics emits collision events, AI uses them in perception, gameplay subscribes for combat reactions.

#### Stage 6: Schema (shared types)
**File:** `astraweave-core/src/schema.rs`
**Role:** Canonical typed data shipped between subsystems and serialized to/from LLM prompts.
**Key types:** `IVec2` (line 173), `Stance`, `CoverType`, `WorldSnapshot` (line 270), `PlayerState`, `CompanionState`, `EnemyState`, `Poi`, `PlanIntent`, `ActionStep` (latter four defined later in the file and re-exported from `crate::schema::*` via `astraweave-core/src/lib.rs:52`).
**Notes:** Per `docs/current/ARCHITECTURE_MAP.md:233`, `WorldSnapshot` is **CRITICAL** because field names are hard-coded in LLM prompts. Renaming `me.pos` or `enemies` would simultaneously break Rust compilation and AI behavior — even after compilation is fixed, the LLM has been trained against the existing field names.

#### Stage 7: Legacy `World` and ECS bridge
**Files:** `astraweave-core/src/world.rs`, `astraweave-core/src/ecs_adapter.rs`, `astraweave-core/src/ecs_bridge.rs`, `astraweave-core/src/ecs_components.rs`
**Role:** Older HashMap-per-component-type world with explicit `spawn`, `pose`, `health`, `team`, `ammo`, `cooldowns`, `behavior_graphs`, `parents`/`children_map` (`astraweave-core/src/world.rs:49-69`). Bridged into an `astraweave_ecs::App` by `build_app` (`astraweave-core/src/ecs_adapter.rs:169-227`).
**Notes:**
- `build_app` inserts the legacy `World` as a resource so ECS systems can call legacy methods (`astraweave-core/src/ecs_adapter.rs:217`).
- `EntityBridge` (`astraweave-core/src/ecs_bridge.rs:6-83`) maintains bidirectional `legacy::Entity (u32) ↔ ecs::Entity (id+generation)` maps with reverse-mapping cleanup.
- `ecs_components.rs` defines `CPos`, `CHealth`, `CTeam`, `CAmmo`, `CCooldowns`, `CDesiredPos`, `CAiAgent`, `CLegacyId` (`astraweave-core/src/ecs_components.rs:5-83`). The module header explicitly labels itself "Phase 1 incremental migration" (line 1).

#### Stage 8: SDK FFI surface
**File:** `astraweave-sdk/src/lib.rs`
**Role:** C ABI for embedding the engine in non-Rust hosts.
**Inputs/outputs:** Opaque `AWWorld` handle wrapping `astraweave_core::World`; UTF-8 NUL-terminated JSON via output buffers or callback pointers.
**Notes:** All FFI entry points are `unsafe extern "C"` (or `extern "C"` for pointer-free entry). Null checks at every boundary (`astraweave-sdk/src/lib.rs:177, 245, 265, 389`). Reusable per-tick buffers (`json_buf`, `changed_buf`, `removed_buf`) avoid per-tick allocation (`astraweave-sdk/src/lib.rs:138-141`). Snapshot/delta callbacks are invoked synchronously inside `aw_world_tick` (`astraweave-sdk/src/lib.rs:186-241`).

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **ECS World** | The archetype-based runtime substrate. Owns entity allocator, archetype storage, resources, change tick. | `astraweave-ecs/src/lib.rs` (`pub struct World`) |
| **Legacy World** | The older HashMap-per-component-type struct. Owns `t`, `next_id`, `obstacles`, and HashMaps for poses/health/team/ammo/cooldowns/names/behavior_graphs/parents. | `astraweave-core/src/world.rs:49-69` (`pub struct World`) |
| **ECS Entity** | `{ id: u32, generation: u32 }` — generational handle that detects use-after-free across despawn/respawn. | `astraweave-ecs/src/entity_allocator.rs:53-57` |
| **Legacy Entity** | `pub type Entity = u32;` — bare numeric ID, recycled by `world.next_id`. | `astraweave-core/src/schema.rs:62` |
| **Archetype** | A set of entities that share the same exact component signature. Components are stored as contiguous columns per archetype. | `astraweave-ecs/src/archetype.rs` |
| **ArchetypeSignature** | A sorted, deduplicated `Vec<TypeId>` describing one archetype's component types. `ArchetypeSignature::new` sorts and dedupes (`astraweave-ecs/src/archetype.rs:25-30`). | `astraweave-ecs/src/archetype.rs` |
| **BlobVec** | Type-erased contiguous storage for components of one type, with manual layout/drop. Faster than `Vec<Box<dyn Any>>` for archetype iteration. | `astraweave-ecs/src/blob_vec.rs` |
| **SparseSet** | Sparse-array → dense-array mapping for O(1) `Entity → row` lookup with packed iteration. | `astraweave-ecs/src/sparse_set.rs` |
| **Component** | `pub trait Component: 'static + Send + Sync {}` — blanket-implemented for any `'static + Send + Sync` type. | `astraweave-ecs/src/lib.rs:85-86` |
| **Resource** | `pub trait Resource: 'static + Send + Sync {}` — blanket-implemented; stored as singletons on `World`. | `astraweave-ecs/src/lib.rs:89-90` |
| **System** | `pub type SystemFn = fn(&mut World);` — bare function pointer (no closure state). | `astraweave-ecs/src/lib.rs:688` |
| **Stage** | A named bucket of systems within a `Schedule`. Systems within a stage run in registration order; stages run in registration order. | `astraweave-ecs/src/lib.rs:691-697` |
| **Change tick** | Monotonically increasing `u32` stamped on every component mutation. Wraps via `wrapping_add` (`astraweave-ecs/src/lib.rs:170`). | `astraweave-ecs/src/lib.rs:131-172, 573-586` |
| **WorldSnapshot** | The canonical AI perception snapshot: `{ t, player, me, enemies, pois, obstacles, objective }`. Field names are hard-coded in LLM prompts. | `astraweave-core/src/schema.rs:270` |
| **PlanIntent** | The canonical AI output: `{ plan_id, steps: Vec<ActionStep> }`. Validated by `validate_and_execute`. | `astraweave-core/src/schema.rs` and `astraweave-core/src/validation.rs:10-…` |
| **EntityBridge** | Bidirectional map between legacy `Entity` (u32) and ECS `Entity`. Lives in a `Resource` slot when `build_app` is used. | `astraweave-core/src/ecs_bridge.rs:6-83` |
| **AWWorld** | Opaque C ABI handle wrapping `astraweave_core::World` plus per-tick callback/buffer state. | `astraweave-sdk/src/lib.rs:128-141` |
| **enable_flush_to_zero()** | Sets MXCSR FTZ+DAZ on x86_64 to avoid 10-100× denormal-float penalties. Call once at engine init. | `astraweave-math/src/lib.rs:73-…` |

### Terms to NOT confuse

- **ECS World vs Legacy World**: These are different types in different crates. `astraweave_ecs::World` is the archetype substrate. `astraweave_core::World` is HashMap-based. They are linked only by the `build_app` bridge, which makes the legacy World available as an `astraweave_ecs::Resource` inside the ECS schedule. Code in the AI/director/example layers commonly takes a `&mut astraweave_ecs::World` and pulls the legacy `World` via `world.get_resource_mut::<astraweave_core::World>()`.
- **ECS Entity vs Legacy Entity**: These are different types. ECS `Entity` has a generation; legacy `Entity` is a bare `u32`. Code that holds both must be specific about which it has. `EntityBridge::get` / `EntityBridge::get_legacy` are the only safe conversion points.
- **`Component` vs `Resource`**: Both are blanket-implemented on `'static + Send + Sync` types — the same Rust type can be both a component (per-entity) and a resource (singleton) depending on how it is inserted into the World. The distinction is by API call, not by trait declaration.
- **`Schedule::add_system` (debug-warns) vs `App::add_system` (delegates)**: Calling `add_system` with an unregistered stage name silently drops the registration in release builds (`astraweave-ecs/src/lib.rs:708-725`). The `pre_simulation`, `sync`, and `post_simulation` stages were added on 2026-04-18 to close out three known silent-drop sites — see `docs/audits/schedule_stage_fix_2026-04-18.md` referenced in `astraweave-ecs/src/lib.rs:771-782`.
- **`fn(&mut World)` vs Bevy-style `IntoSystem`**: AstraWeave uses bare function pointers, not a system-builder DSL. Closures with captured state cannot be registered; state must live on `World` or `Resource`s. This is intentional (per the audits cited in §7) and constrains how Bevy-flavored patterns can be ported.
- **`ParallelSchedule` (removed)**: Any reference to `ParallelSchedule`, `SystemAccess`, or `SystemDescriptor` in older docs is outdated as of 2026-04-18. The current scheduler is `Schedule` only (`astraweave-ecs/src/lib.rs:690-735`).

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this layer)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| `glam` crate | `Vec3`, `Vec4`, `Mat4`, `Quat`, `IVec2`-free (legacy `IVec2` is in core) | Math primitive types | `astraweave-math` is a thin SIMD layer over `glam` types; falls back to `glam`'s own scalar paths on non-x86_64 (`astraweave-math/src/simd_vec.rs:71-75`) |
| `serde` / `serde_json` | `Serialize`, `Deserialize` impls | `WorldSnapshot`, `PlanIntent`, `IVec2`, all C `ecs_components` | Drives JSON round-trips for SDK and capture/replay |
| `rand_chacha` (via `Rng`) | Seeded RNG for determinism | Combat rolls, AI tiebreaks | `astraweave-ecs/src/rng.rs` |

### Downstream (what consumes this layer's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| `astraweave-ai` | `App`, `World`, `EntityBridge`, `WorldSnapshot`, `PlanIntent`, `ToolRegistry` | AI orchestrators read snapshots, emit plan intents | `astraweave-ai/src/ai_arbiter.rs` (consumer); `build_app_with_ai` is the canonical entry point per `ARCHITECTURE_MAP.md:159` |
| `astraweave-behavior` | `World`, `Entity` from ECS | Behavior tree execution context | `astraweave-behavior/src/ecs.rs` |
| `astraweave-gameplay` | `World`, `Entity`, `EntityBridge`, combat math from `astraweave-math` | Combat physics, weaving, items | `astraweave-gameplay/src/ecs.rs`, `astraweave-gameplay/src/veilweaver_tutorial.rs` |
| `astraweave-scene` | `World`, `Entity`, `Transform` (in scene), components | Scene streaming, partitioning | `astraweave-scene/src/lib.rs` |
| `astraweave-persistence-ecs` | `World`, `Entity` from ECS, all `ecs_components` | Save/load entire ECS state | `astraweave-persistence-ecs/src/lib.rs` |
| `astraweave-net-ecs` | `World`, `Entity` from ECS, change ticks | Snapshot/delta network replication | `astraweave-net-ecs/src/lib.rs` |
| `astraweave-physics` | `astraweave-math` SIMD ops (vec/mat); `Entity` from ECS via `astraweave-gameplay::ecs` | Rapier3D bridge, character controllers | Physics does not depend on core's legacy World directly |
| `astraweave-render` | `astraweave-math` SIMD ops; `Entity` from ECS via scene/gameplay | Math for transforms, camera, lighting | Render is independent of `astraweave-core` schema |
| `astraweave-llm` (feature-gated) | `WorldSnapshot`, `PlanIntent`, `CompanionState` | LLM prompt construction and tool-call output | `WorldSnapshot` field names hard-coded in prompts |
| `astraweave-director` | `WorldSnapshot`, `PlanIntent` | Director-mode planning | `astraweave-director/src/lib.rs` |
| `astraweave-ipc` | `WorldSnapshot`, `PlanIntent` | IPC bridge for external orchestrators | `astraweave-ipc/src/lib.rs` |
| `astraweave-scripting` | `World`, `Entity` from ECS | Rhai-backed scripting | `astraweave-scripting/src/lib.rs` |
| `astraweave-observability` | `World`, `Entity` from ECS | Telemetry hooks | `astraweave-observability/src/lib.rs` |
| `astraweave-security` | `World`, `Entity` from ECS | Capability checks | `astraweave-security/src/lib.rs` |
| `astraweave-stress-test` | `World`, `Entity` from ECS | Stress harness | `astraweave-stress-test/src/lib.rs` |
| `tools/aw_editor` | `astraweave-core::World`, `WorldSnapshot`, ECS `Entity`, `astraweave-math` SIMD | Editor runtime, scene state, command system | `tools/aw_editor/src/runtime.rs`, `tools/aw_editor/src/scene_state.rs`, `tools/aw_editor/src/command.rs` |
| Examples (`hello_companion`, `ecs_ai_showcase`, `profiling_demo`, `weaving_playground`, etc.) | `App`, `build_app`, `WorldSnapshot`, math primitives | Flagship demos | Example crates under `examples/` |
| External hosts (C, C++, C#, …) | `aw_world_create/destroy/tick/snapshot_json/submit_intent_json` C ABI | Embedded engine instance | `astraweave-sdk` is the engine-exposure path for non-Rust callers. Workspace-wide audit (2026-05-10): exactly one in-tree consumer — `examples/sdk_c_harness/Cargo.toml:9` depends on `astraweave-sdk` and `examples/sdk_c_harness/src/main.rs:4-8` links the FFI surface via `#[link(name = "astraweave_sdk", kind = "static")]` and `extern "C"` declarations for `aw_version` and `aw_version_string`. No other Rust crate `use`s the SDK |

### Bidirectional / Coupled

- **`astraweave-core` ↔ `astraweave-ecs`**: `astraweave-core` depends on `astraweave-ecs` (per `docs/current/ARCHITECTURE_MAP.md:35`: `astraweave-core` → `ecs, behavior`). The coupling is via `astraweave-core::ecs_adapter::build_app`, which mints an `astraweave_ecs::App`, populates it from a legacy `World`, and writes the legacy `World` back as a resource. `astraweave-core::ecs_bridge::EntityBridge` keeps both entity-ID spaces in sync.
- **`astraweave-sdk` ↔ `astraweave-core`**: `astraweave-sdk` wraps an `astraweave_core::World` (not the ECS World) and exposes it through the C ABI. It is a one-way dependency; `astraweave-core` does not know about `astraweave-sdk`.
- **`astraweave-math` ↔ everything else**: Standalone, downstream-only. No reverse coupling.

---

## 5. Active File Map

### `astraweave-ecs`

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-ecs/src/lib.rs` | Top-level: `World`, `Schedule`, `App`, `SystemStage` constants, `Plugin` trait, `Component`/`Resource` blanket impls, change-tick API | Active | `pub mod` declarations at lines 37-46; `pub use` re-exports at lines 73-83. The `ParallelSchedule` removal note lives at lines 78-80. |
| `astraweave-ecs/src/archetype.rs` | `Archetype`, `ArchetypeId`, `ArchetypeSignature`, `ArchetypeStorage`, change-tick columns | Active | Dual storage modes (Box vs BlobVec) selected per archetype |
| `astraweave-ecs/src/entity_allocator.rs` | `Entity { id, generation }`, `EntityAllocator` with O(1) free list | Active | 8-byte handle; generational counter on slot reuse |
| `astraweave-ecs/src/blob_vec.rs` | Type-erased contiguous component column with manual `Layout`/`drop_fn` | Active | Uses `unsafe` (alloc/dealloc/realloc, pointer arithmetic, drop) — Kani-verified in `blob_vec_kani.rs` |
| `astraweave-ecs/src/sparse_set.rs` | Sparse-set entity → dense-row mapping | Active | O(1) entity lookups; replaced earlier BTreeMap (see `system_param.rs:1-15` perf notes) |
| `astraweave-ecs/src/system_param.rs` | `SystemParam` marker, `Query<T>`, `Query2<T1,T2>`, `Query2Mut<T1,T2>` | Active | Lengthy perf-history docstring at top |
| `astraweave-ecs/src/command_buffer.rs` | `CommandBuffer` for deferred spawn/insert/remove/despawn | Active | `!Send + !Sync` to match World access model |
| `astraweave-ecs/src/component_meta.rs` | `ComponentMeta`, `ComponentMetaRegistry` — drop/clone function pointers for BlobVec storage | Active | Populated by `World::register_component::<T: Component + Clone>` |
| `astraweave-ecs/src/type_registry.rs` | `TypeRegistry` with type-erased insert/remove handlers for `CommandBuffer` | Active | Required for `CommandBuffer` to operate on registered types |
| `astraweave-ecs/src/events.rs` | `Event` marker, `Events<E>`, `EventReader<E>`, `EventEntry` | Active | Frame-stamped retention; cleanup via `cleanup(current_frame, keep_frames)` |
| `astraweave-ecs/src/rng.rs` | `Rng` resource wrapping `ChaCha8Rng` | Active | Deterministic-AI requirement |
| `astraweave-ecs/src/counting_alloc.rs` | Allocation-counting allocator for tests | Active (feature `alloc-counter`) | Behind `#[cfg(feature = "alloc-counter")]` (`lib.rs:48-49`) |
| `astraweave-ecs/src/lib_new.rs` | 17-line stub containing only blanket-impl `Component`/`Resource` traits (identical to `lib.rs:85-90`) and module-level `use` imports | Dead (orphan scratch file) | Verified — not declared in `pub mod` list at `lib.rs:37-46`; workspace-wide grep finds zero non-self references to `lib_new`. Git log shows only two commits: original add in `400903a18` ("bug fixes, update ecs, nanite implementation, voxelization_pipeline") and a CI fmt sweep in `2702232fb`. Module-doc comment says "AstraWeave ECS / Provides Bevy-like API with Query tuples, Res/ResMut" — describes an API that the file does not actually implement |
| `astraweave-ecs/src/blob_vec_kani.rs` | Kani proofs for `BlobVec` invariants | Active (kani gate) | `#[cfg(kani)]` (`lib.rs:64-65`) |
| `astraweave-ecs/src/entity_allocator_kani.rs` | Kani proofs for generational-index correctness | Active (kani gate) | `#[cfg(kani)]` (`lib.rs:67-68`) |
| `astraweave-ecs/src/determinism_tests.rs` | Inline determinism tests | Active (test gate) | `#[cfg(test)] mod determinism_tests;` (`lib.rs:51-52`) |
| `astraweave-ecs/src/property_tests.rs` | Inline property-based tests | Active (test gate) | `#[cfg(test)]` (`lib.rs:54-55`) |
| `astraweave-ecs/src/mutation_tests.rs` | Inline mutation-resistance tests | Active (test gate) | `#[cfg(test)]` (`lib.rs:57-58`) |
| `astraweave-ecs/src/mutation_resistance_tests.rs` | Additional mutation-resistance tests | Active (test gate) | `#[cfg(test)]` (`lib.rs:60-61`) |
| `astraweave-ecs/tests/*.rs` | 14 integration test files (world_app, behavioral_correctness, mutation_resistant_comprehensive, panic_safety, full_pipeline_integration, sparse_set_additional, zero_alloc, blob_vec_entity_allocator, system_param, ecs_core, archetype_command_rng, stress, concurrency, coverage_booster) | Active | See §10 for counts. `concurrency_tests.rs:10-15` is `#[cfg(feature = "loom")]`-gated and uses `loom::sync::Arc` / `loom::thread` to model-check entity-spawn interleavings under the optional `loom` feature declared at `astraweave-ecs/Cargo.toml:33` |

### `astraweave-math`

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-math/src/lib.rs` | Top-level: re-export modules, `enable_flush_to_zero()` | Active | Module declarations at lines 68-71. Note: `astraweave-math/Cargo.toml:23-24` defines a `simd` feature with empty dependency list — workspace-wide grep finds zero `cfg(feature = "simd")` usages. The feature is a no-op stub; actual SIMD activation is via `target_feature = "sse2"` cfg-gates throughout the source (37 occurrences across 4 files) |
| `astraweave-math/src/simd_vec.rs` | SIMD Vec3/Vec4 dot, cross, normalize, length | Active | SSE2 fast-path with `glam` fallback (`simd_vec.rs:64-75`) |
| `astraweave-math/src/simd_mat.rs` | SIMD Mat4 multiply, transpose, transform | Active | Uses `unsafe` for intrinsics; SAFETY comments document target-feature gates |
| `astraweave-math/src/simd_quat.rs` | SIMD Quat multiply, normalize, slerp | Active | `mul_quat_simd`, `slerp_simd` |
| `astraweave-math/src/simd_movement.rs` | Batched SIMD `position += velocity * dt` for 1k+ entities | Active | `update_positions_simd` |
| `astraweave-math/src/simd_vec_kani.rs` | Kani proofs for SIMD vec operations | Active (kani gate) | Verified per `docs/current/ARCHITECTURE_MAP.md:549` |
| `astraweave-math/src/mutation_tests.rs` | Inline mutation tests | Active (test gate) | |
| `astraweave-math/tests/mutation_resistant_comprehensive_tests.rs` | Integration test file | Active | |
| `astraweave-math/benches/*` | 4 criterion bench files (simd_benchmarks, simd_mat_benchmarks, simd_quat_benchmarks, simd_movement) | Active | |

### `astraweave-core`

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-core/src/lib.rs` | Top-level: module declarations, re-exports, `default_tool_registry()` | Active | `pub mod` at lines 28-43; `pub use` re-exports at lines 51-62 |
| `astraweave-core/src/schema.rs` | `IVec2`, `WorldSnapshot`, `PlayerState`, `CompanionState`, `EnemyState`, `Poi`, `PlanIntent`, `ActionStep`, `Stance`, `CoverType`, plus `pub type Entity = u32` | Active | Canonical AI-perception schema. **CRITICAL** — names hardcoded in LLM prompts |
| `astraweave-core/src/world.rs` | Legacy `World` struct with HashMap-per-component-type (poses, health, team, ammo, cds, names, behavior_graphs, parents/children_map) | Transitional | Bridged into ECS via `ecs_adapter::build_app`. Long-term role is the open question in §11 |
| `astraweave-core/src/ecs_adapter.rs` | `build_app(legacy_world, dt) -> ecs::App`, plus `sys_sim`, `sys_move`, `sys_bridge_sync`, `sys_sync_to_legacy`, `sys_refresh_los` | Transitional | Bridge layer between legacy World and ECS substrate |
| `astraweave-core/src/ecs_bridge.rs` | `EntityBridge` bidirectional `legacy::Entity ↔ ecs::Entity` map | Active | Reverse index maintained on every insert/remove |
| `astraweave-core/src/ecs_components.rs` | `CPos`, `CHealth`, `CTeam`, `CAmmo`, `CCooldowns`, `CDesiredPos`, `CAiAgent`, `CLegacyId`, plus placeholders `CompanionProfile`, `Fact` | Transitional | Module header: "Phase 1 incremental migration" (line 1) |
| `astraweave-core/src/ecs_events.rs` | `Events<E>` bridge type, `MovedEvent` | Active | Used by `ecs_adapter` for `sys_move` |
| `astraweave-core/src/perception.rs` | `astar_path`, `find_cover_positions`, `los_clear` AI perception helpers | Active | Re-exported via `pub use perception::*` (`lib.rs:51`) |
| `astraweave-core/src/tools.rs` | `astar_path`, `find_cover_positions`, `los_clear`, `path_exists`, `glam_to_schema`, `schema_to_glam` | Active | Some helpers also re-exported via `pub use tools::{…}` (`lib.rs:58-60`); see also §6 naming-collision note on `Poi` |
| `astraweave-core/src/tool_sandbox.rs` | `ToolBlock`, `ToolBlockReason`, `map_engine_error` | Active | Maps engine errors to a stable tool taxonomy |
| `astraweave-core/src/tool_vocabulary.rs` | Vocabulary used by AI tool sandbox | Active | Re-exported via `pub use tool_vocabulary::*` (`lib.rs:57`) |
| `astraweave-core/src/validation.rs` | `ValidateCfg`, `validate_and_execute(world, actor, intent, cfg, log)` | Active | Used by SDK (`astraweave-sdk/src/lib.rs:40-43`) |
| `astraweave-core/src/sim.rs` | `SimConfig { dt }`, `step(world, cfg)` — thin wrapper around `World::tick` | Active | Re-exported via `pub use sim::*` (`lib.rs:53`) |
| `astraweave-core/src/capture_replay.rs` | JSON Snapshot/replay infrastructure for legacy World | Active | Phase 0 capture state (`capture_replay.rs:1-2` header) |
| `astraweave-core/src/metrics.rs` | Telemetry and performance counters | Active | |
| `astraweave-core/src/util.rs` | Misc utilities | Active | |
| `astraweave-core/src/schema_kani.rs` | Kani proofs for `WorldSnapshot` schema | Active (kani gate) | `#[cfg(kani)]` (`lib.rs:45-46`) |
| `astraweave-core/src/mutation_tests.rs` | Inline mutation tests | Active (test gate) | `#[cfg(test)]` (`lib.rs:48-49`) |
| `astraweave-core/tests/*.rs` | 10 integration test files | Active | |

### `astraweave-sdk`

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-sdk/src/lib.rs` | C ABI: `aw_version`, `aw_version_string`, `aw_world_create/destroy/tick`, `aw_world_set_snapshot_callback`, `aw_world_set_delta_callback`, `aw_world_snapshot_json`, `aw_world_submit_intent_json`, `aw_last_error_string`. Plus `Version`, `AWVersion`, `SdkError`, `GameAdapter` Rust-side types | Active | All entry points either `extern "C"` or `unsafe extern "C"` with documented SAFETY contracts (`lib.rs:27-32, 106-109, 169-172, 191-198, 229-232, 376-381`) |
| `astraweave-sdk/src/lib_kani.rs` | Kani proofs for FFI safety (null pointer, buffer overflow) | Active (kani gate) | `#[cfg(kani)]` (`lib.rs:46-47`) |
| `astraweave-sdk/tests/mutation_resistant_comprehensive_tests.rs` | Integration test file | Active | Only test file for this crate |
| `astraweave-sdk/benches/{sdk_benchmarks,sdk_adversarial}.rs` | Bench harnesses | Active | |

**Status definitions used here:**
- **Active**: Canonical, load-bearing, edit with care
- **Transitional**: Live in active code paths but tagged for migration. Specifically `astraweave-core::World`, `ecs_adapter`, and `ecs_components` carry "Phase 1 incremental migration" framing (`ecs_components.rs:1`); the long-term plan is to retire the legacy World in favour of pure ECS
- **Active (test gate)** / **Active (kani gate)**: Inline modules behind `#[cfg(test)]` or `#[cfg(kani)]`

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Notes |
|---|---|---|---|
| ECS `World` (archetype-based, generational entities) | `astraweave-ecs/src/lib.rs:106-685`, `astraweave-ecs/src/archetype.rs` | Active (canonical) | Substrate for all current ECS work |
| Legacy `World` (HashMap-per-component, bare `u32` entities) | `astraweave-core/src/world.rs:49-69` | Transitional | Bridged via `build_app`; module header on `ecs_components.rs:1` describes the migration framing |
| Two `Entity` types | `astraweave-ecs/src/entity_allocator.rs:53-57` (generational struct) vs `astraweave-core/src/schema.rs:62` (`pub type Entity = u32`) | Coexisting — mapped via `EntityBridge` | The two are not interchangeable. Any code holding both must be explicit about which is which |
| Two `Events` containers | `astraweave-ecs/src/events.rs` (typed event bus, frame-stamped) vs `astraweave-core/src/ecs_events.rs` (legacy bridge events including `MovedEvent`) | Coexisting | `astraweave-core::ecs_events::Events<E>` is a different type from `astraweave_ecs::Events<E>` — both are referenced by name. See §11 |
| Two storage modes in `Archetype` | `astraweave-ecs/src/archetype.rs:66-99` — `components: HashMap<TypeId, Vec<Box<dyn Any>>>` (Box mode) vs `blob_components: Option<HashMap<TypeId, BlobVec>>` (BlobVec mode) | Coexisting — selected per-archetype | Box mode is the default; BlobVec mode requires `world.register_component::<T>()` |
| `astraweave-core::Poi` (in `schema.rs`) vs `astraweave-core::tools::Poi` | `astraweave-core/src/lib.rs:54` comment: "tools::Poi and schema::Poi are different types - using qualified imports where needed" | Coexisting | Documented at the module-doc level; expect qualified imports |

### Naming collisions

- **`World`**: `astraweave_ecs::World` is the ECS substrate; `astraweave_core::World` is the legacy struct. Always namespace fully when both might be in scope.
- **`Entity`**: `astraweave_ecs::Entity` is `{id, generation}`; `astraweave_core::Entity` is `u32`. Different identity semantics, different memory layouts (8 vs 4 bytes). The `EntityBridge` is the only safe conversion.
- **`Events`**: `astraweave_ecs::Events<E>` (event bus, used cross-subsystem) vs `astraweave_core::ecs_events::Events<E>` (legacy bridge events). The latter is what `ecs_adapter::build_app` inserts as a resource.
- **`Poi`**: `astraweave_core::schema::Poi` vs `astraweave_core::tools::Poi` — same crate, different submodules.
- **`Component`/`Resource`**: Both are blanket-implemented marker traits in `astraweave-ecs/src/lib.rs:85-90`. A single Rust type is both — the distinction is made at the call site (`World::insert` vs `World::insert_resource`), not at the type level.

### Known cognitive traps

- **Trap**: Editing `astraweave-core/src/world.rs` (the legacy World) expecting it to be the ECS World.
  **What's actually true**: `astraweave-core::World` is the legacy struct retained for transitional bridging. The canonical ECS World is `astraweave_ecs::World` in a different crate. Both are still in active use because the bridge runs the legacy `World::tick` (`astraweave-core/src/ecs_adapter.rs:11-13`) from inside the ECS `simulation` stage.
- **Trap**: Reading `astraweave-ecs/src/lib_new.rs` and treating it as authoritative.
  **What's actually true**: `lib_new.rs` is a 17-line orphan stub never wired into `lib.rs`. It contains only blanket-impl `Component`/`Resource` trait declarations identical to those in `lib.rs:85-90`. Workspace grep finds zero non-self references; git history shows only the original add (`400903a18`) and a fmt sweep (`2702232fb`). The module-doc comment promises "Bevy-like API with Query tuples, Res/ResMut" that the file does not actually implement. Treat as dead code.
- **Trap**: Adding parallel scheduling to ECS systems.
  **What's actually true**: `ParallelSchedule` was removed on 2026-04-18. The current scheduler is deterministic single-threaded by design. Parallelism is delivered at the subsystem level (rayon, tokio, GPU compute). See `docs/audits/parallel_schedule_removal_2026-04-18.md` and the framing note in `astraweave-ecs/src/lib.rs:78-80`.
- **Trap**: Registering a system on a stage name that isn't in the canonical eight-stage list.
  **What's actually true**: `Schedule::add_system` for an unknown stage name prints a `[astraweave-ecs] Schedule::add_system: stage '…' is not registered` warning in debug builds and silently drops the registration in release builds (`astraweave-ecs/src/lib.rs:708-725`). Always go through `App::new()` for the canonical stages, or call `Schedule::with_stage` first for custom names.
- **Trap**: Calling `World::register_component::<T>()` and expecting all archetypes to switch to BlobVec mode.
  **What's actually true**: Box mode and BlobVec mode coexist per-archetype. Registration enables the BlobVec path but does not migrate existing archetypes. New archetypes created after registration that contain the registered component will use the BlobVec column for that type; this is documented at `astraweave-ecs/src/lib.rs:108-119` and `astraweave-ecs/src/archetype.rs:45-93`.
- **Trap**: Treating `WorldSnapshot` field names as flexible.
  **What's actually true**: The names `me`, `enemies`, `pois`, `obstacles`, `objective`, `player`, `t`, plus inner fields like `me.pos`, `me.ammo`, `me.cooldowns`, are hard-coded in LLM prompts. Renaming any of them requires a coordinated LLM-prompt update. See `docs/current/ARCHITECTURE_MAP.md:233-235`.

---

## 7. Decision Log

### Decision: Archetype-based storage with optional BlobVec mode
- **Date:** Date of original decision not recovered. BlobVec capacity-check refinement landed in commit `d8e2df210` ("fix: optimize archetype component lookup and adjust BlobVec capacity check"). Property/fuzz hardening landed in `18dc462d3`.
- **Status:** Accepted (in active code)
- **Context:** Stated goals are documented in two places: (1) `astraweave-ecs/src/lib.rs:1-32` module header cites "Archetype-based storage for cache-friendly iteration (like Bevy/Flecs)" and "Deterministic execution via fixed schedules and ordered iteration"; (2) `docs/src/architecture/ecs.md:1-3, 87-131` documents archetype semantics and per-archetype dual storage. BlobVec is explicitly framed there as "Components stored in contiguous byte arrays. Requires component registration but provides 2-10× faster iteration" (`docs/src/architecture/ecs.md:117`).
- **Decision:** Components live in archetype columns keyed by sorted `Vec<TypeId>`. Each archetype provides two storage modes: `Vec<Box<dyn Any>>` (always available) and contiguous `BlobVec` (after `register_component::<T: Clone>` is called).
- **Alternatives considered:** Bevy and Flecs are cited as inspirations in `astraweave-ecs/src/lib.rs:1-4`; no detailed alternative-architecture record (e.g. sparse-set-only à la EnTT) is present in available sources.
- **Consequences:** Cache-friendly iteration; structural changes (insert/remove) require moving the entity to a different archetype, which is implemented in `move_entity_to_new_archetype` (`astraweave-ecs/src/lib.rs:315-387`). BlobVec adds substantial `unsafe` (manual `Layout`, drop function pointers, raw pointer arithmetic) — mitigated by Kani proofs in `astraweave-ecs/src/blob_vec_kani.rs`. Lazy-init `Option<HashMap<TypeId, BlobVec>>` was retained after a measured +388% spawn/despawn regression (`astraweave-ecs/src/archetype.rs:60-66`).

### Decision: Generational entity indices
- **Date:** Date of original introduction not recovered. Subsequent hardening: `eef644b2f` added capacity accessors and reserve/with_capacity mutation tests.
- **Status:** Accepted (in active code)
- **Context:** Documented in two places: (1) `astraweave-ecs/src/entity_allocator.rs:1-30` module header describes the prior use-after-free hazard from naïve `u32` reuse and the generational-index fix with explicit before/after code examples; (2) `docs/src/architecture/ecs.md:7-28` reiterates the rationale at the architecture-doc layer.
- **Decision:** `Entity` is `{ id: u32, generation: u32 }` (`astraweave-ecs/src/entity_allocator.rs:53-57`). The allocator stamps a fresh generation on every reuse; every `World` accessor checks `entity_allocator.is_alive(entity)` before proceeding.
- **Alternatives considered:** The `entity_allocator.rs:38-43` ASCII memory-layout diagram explicitly contrasts the 64-bit `{id, generation}` packing against a bare `u64` (both 8 bytes). No record found of evaluating sparse-array vs free-list allocator strategies, but the file documents the chosen "O(1) amortized free-list" approach at lines 28-30.
- **Consequences:** 8-byte entity handle (no size growth from a bare `u64`). Stale handles are rejected silently (return None / no-op) — there is no panic on use-after-despawn. Kani-verified by `entity_allocator_kani.rs`.

### Decision: Deterministic single-threaded ECS; subsystem-level parallelism
- **Date:** 2026-04-18 (removal of `ParallelSchedule`)
- **Status:** Accepted; supersedes a prior opt-in `ParallelSchedule` scheduler
- **Context:** A prior parallel scheduler (`ParallelSchedule`, `SystemAccess`, `SystemDescriptor`, plus `SendWorldPtr` for cross-thread world sharing) existed behind a `parallel` feature flag. The safety audit at `docs/audits/parallel_schedule_safety_audit_2026-04-18.md` and binary inventory at `docs/audits/parallel_schedule_binary_inventory_2026-04-18.md` found: zero default-features consumers, and the two opt-in consumers (`profiling_demo`, `ecs_ai_showcase`) produced observed-incorrect output under parallel mode.
- **Decision:** Remove `ParallelSchedule` entirely. The deterministic single-threaded `Schedule` is the sole scheduler. Subsystem-level parallelism (rayon in `astraweave-terrain` meshing and `astraweave-fluids` SPH; tokio for async I/O, LLM, streaming; GPU compute in `astraweave-render`) is preserved.
- **Alternatives considered:** Documented in `docs/audits/parallel_schedule_safety_audit_2026-04-18.md` §5.3 (Framing X, Y, Z). The decision was Framing Y / Option E: full removal.
- **Consequences:** ECS tick is single-threaded by contract. Determinism guarantees hold across runs. The bit-for-bit state checksums at frame 100 for both opt-in consumers matched the sequential path (per `docs/audits/parallel_schedule_removal_2026-04-18.md` §1).

### Decision: Add `pre_simulation`, `sync`, and `post_simulation` stages
- **Date:** 2026-04-18 (per the comment block at `astraweave-ecs/src/lib.rs:771-782`)
- **Status:** Accepted (in active code)
- **Context:** A binary-inventory audit (`docs/audits/parallel_schedule_binary_inventory_2026-04-18.md` §4, referenced in `astraweave-ecs/src/lib.rs:712-717`) surfaced that `Schedule::add_system` silently dropped systems registered to unknown stage names — including `pre_simulation`, `sync`, and `post_simulation`, which were defined as constants in `SystemStage` but not registered by `App::new()`.
- **Decision:** Extend `App::new()` to register all eight canonical stages. The `sync` stage is placed between `simulation` and `ai_planning` so that ECS→legacy-World propagation (`sys_sync_to_legacy`) runs after component mutations and before AI planners read the legacy World (`astraweave-ecs/src/lib.rs:778-781`).
- **Alternatives considered:** [Reasoning not recovered beyond the inline comment]
- **Consequences:** Eight stages total; `Schedule::add_system` now logs a debug-build warning for unknown stage names while keeping release-build behaviour for backward compatibility with optional custom stages.

### Decision: Bridge legacy `World` into ECS via `ecs_adapter::build_app`
- **Date:** Date of original introduction not recovered. Subsequent fixes: `d00d398b8` corrected cooldown decay direction (increment vs decrement); `07648ab76` added mutation-killing tests for cooldown decay and sync.
- **Status:** Accepted (transitional)
- **Context:** The pre-ECS engine used `astraweave_core::World` (HashMap-per-component). The ECS substrate was introduced afterward. The header at `astraweave-core/src/ecs_components.rs:1` explicitly labels the components in that file as "ECS component types mirroring legacy World data (Phase 1 incremental migration)". Deeper rationale for choosing bridge-over-rewrite (e.g. timeline, blocker list, alternatives evaluated) is not recorded in available sources.
- **Decision:** `astraweave-core::ecs_adapter::build_app(legacy_world, dt)` mints an `astraweave_ecs::App`, mirrors each legacy entity into ECS (with `CPos`, `CHealth`, `CTeam`, `CAmmo`, `CCooldowns`), inserts the legacy `World` as a resource, and registers `sys_sim`, `sys_move`, `sys_bridge_sync`, `sys_sync_to_legacy`, `sys_refresh_los`. `EntityBridge` maintains bidirectional ID maps.
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** Dual-World runtime. AI orchestrators, validation, capture/replay, and the SDK continue to operate on legacy `World`. ECS-native systems operate on ECS components. The `sync` stage exists specifically to keep the two consistent. Long-term migration target is unresolved (see §11).

### Decision: SIMD math primitives as thin layer over `glam`
- **Date:** Week 5 Action 21 (Vec3/Vec4) and Week 6 Action 26 (Mat4 / Quat), per the module header at `astraweave-math/src/lib.rs:14-18`. SIMD movement landed in Week 8 Day 3 (`docs/journey/daily/WEEK_8_DAY_3_SIMD_MOVEMENT_PLAN.md` dated October 12, 2025; implementation completion documented in `WEEK_8_DAY_3_COMPLETE.md` and `WEEK_8_DAY_3_IMPLEMENTATION_COMPLETE.md`).
- **Status:** Accepted (in active code)
- **Context:** Profile data identified the movement system as a hot path (861 µs/frame for 1,000 entities, 30% of 2.87 ms total — see `WEEK_8_DAY_3_SIMD_MOVEMENT_PLAN.md:14-22`). The `lib.rs` module header documents 1.7-2.5× speedups (lines 47-58) via SSE2 with `glam` scalar fallback.
- **Decision:** `astraweave-math` exposes `simd_vec`, `simd_mat`, `simd_quat`, `simd_movement` modules. Each function `#[cfg]`-gates on `target_feature = "sse2"` (x86_64) and falls back to `glam`'s own scalar path otherwise.
- **Alternatives considered:** AVX2 (Intel Haswell+ 2013 / AMD Excavator+ 2015) was evaluated for theoretical 8× speedup but rejected in favour of broader baseline compatibility — see `WEEK_8_DAY_3_SIMD_MOVEMENT_PLAN.md:32-41` for the SIMD-architecture trade-off discussion. CLAUDE.md item 5 in "Key Lessons" captures the post-hoc principle: "Trust glam auto-vectorization (80-85% of hand-written AVX2)". Manual AVX2 intrinsics were not adopted; SSE2 plus glam-fallback was retained.
- **Consequences:** Portable across x86_64/ARM/other with predictable performance. `debug_assert!(a.is_finite(), …)` checks NaN/Inf inputs in debug builds (e.g. `simd_vec.rs:62-63`). All SIMD entry points are `unsafe` from the intrinsics perspective; SAFETY comments at each call site cite the cfg-gate target-feature guarantee (e.g. `simd_vec.rs:64-69`).

### Decision: C ABI as the embedding interface
- **Date:** Date of original introduction not recovered. The SDK's MVP completion is documented in `docs/journey/weeks/PHASE_0_WEEK_2_COMPLETE.md:201-214` (Phase 0 Week 2: "Crate 6: astraweave-sdk (C ABI Exports)"). Subsequent hardening: `a8ff30337`, `080121891`, `92899b0c4` added mutation-killing tests for delta change/removal detection and destroy.
- **Status:** Accepted (in active code)
- **Context:** `astraweave-sdk` exposes the engine to non-Rust hosts (C, C++, C#, Python, …) through a `cbindgen`-compatible interface. The audience and supported FFI languages are stated in the module header at `astraweave-sdk/src/lib.rs:1-32`. Deeper rationale for choosing C ABI over gRPC, WebAssembly, or raw shared-library Rust exports is not recorded in available sources.
- **Decision:** A small set of opaque-handle functions (`aw_world_create/destroy/tick/snapshot_json/submit_intent_json` plus version/callback helpers) carrying state as UTF-8 NUL-terminated JSON.
- **Alternatives considered:** The `GameAdapter` trait at `astraweave-sdk/src/lib.rs:65-68` carries a comment "future: hooks for feeding snapshots, receiving intents via IPC (gRPC/WebSocket)" — indicating gRPC and WebSocket are tracked as future expansions, not abandoned alternatives. No explicit pros/cons record was found.
- **Consequences:** Stable cross-language surface, JSON serialisation overhead per tick, every entry point is `unsafe` from the Rust side. Reusable per-tick `json_buf`/`changed_buf`/`removed_buf` (`astraweave-sdk/src/lib.rs:138-141`) avoid per-tick allocation.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | An `Entity` handle returned by `World::spawn` is unique for the lifetime of the allocator: even after `despawn` and slot reuse, the `(id, generation)` pair never repeats | Yes | `entity_allocator.rs`: generation is incremented on every reuse (`despawn` increments, `spawn` returns the new generation). Kani proof in `entity_allocator_kani.rs` |
| 2 | Stale entity handles never read or write live data | Yes | Every accessor on `World` checks `entity_allocator.is_alive(entity)` first (`astraweave-ecs/src/lib.rs:261, 409, 437, 603, 641`). Stale handles return `None` / no-op / `false` |
| 3 | `Schedule::run` executes stages in registration order; systems within a stage execute in registration order | Yes | `astraweave-ecs/src/lib.rs:726-735`. Tests in `astraweave-ecs/tests/world_app_tests.rs` (`test_app_new`, `test_app_default`) assert canonical eight-stage order (updated 2026-04-18) |
| 4 | The ECS tick is deterministic single-threaded — same inputs produce same outputs across runs and platforms | Yes | Single-threaded scheduler (`Schedule::run`). RNG is seeded (`astraweave-ecs/src/rng.rs`). Determinism tests in `astraweave-ecs/src/determinism_tests.rs` and `astraweave-core/tests/full_system_determinism.rs` |
| 5 | `ArchetypeSignature` is sorted and deduplicated | Yes | `ArchetypeSignature::new` calls `sort_unstable` and `dedup` (`astraweave-ecs/src/archetype.rs:26-29`). Two signatures with the same component set hash and compare equal |
| 6 | Mutation of a component stamps the current change tick on that entity's row | Yes | `insert` and `each_mut` route through `add_entity_with_tick` / `stamp_change_tick::<T>` (`astraweave-ecs/src/lib.rs:308-310, 526-527`). `get_mut` also stamps (conservative Bevy-style change detection, `astraweave-ecs/src/lib.rs:446-447`) |
| 7 | `EntityBridge::insert_pair` preserves bidirectional consistency: every entry in the forward map has a matching entry in the reverse map (and vice-versa) | Yes | `ecs_bridge.rs:17-34` removes any conflicting old forward/reverse mappings before insertion. Tested in `astraweave-core/tests/ecs_integration_tests.rs` |
| 8 | LLM-facing JSON keys produced from `WorldSnapshot` remain stable. Top-level keys (`t`, `player`, `me`, `enemies`, `pois`, `obstacles`, `objective`) are emitted 1:1 with Rust field names; sub-level keys are **translated** by some serializers (e.g. `prompts.rs:212` emits `"position"` for `snapshot.player.pos`; `prompts.rs:229` emits `"points_of_interest"` for `snapshot.pois`; `prompts.rs:213` emits `"health"` for `snapshot.player.hp`) and preserved by others (`compression.rs:152` emits `"pois"`; `prompt_template.rs:227-251` uses `pos`/`hp` directly in few-shot example strings) | Partially | Rust-side compilation enforces struct shape and catches access-site renames. Cross-LLM-prompt alignment between Rust fields, the three serializers (`astraweave-llm/src/prompts.rs:209-238`, `astraweave-llm/src/compression.rs:139-300`, `astraweave-llm/src/prompt_template.rs:227-251`), and the LLM training corpus is doc-only — no build script, lint, or test compares serializer output keys to schema field names. Kani proof in `astraweave-core/src/schema_kani.rs` covers `IVec2` numerical properties and `WorldSnapshot` helper correctness (`schema_kani.rs:1-60+`), not field-name stability |
| 9 | All `unsafe` in this layer has a corresponding Kani proof or Miri test | Partially | Miri CI weekly (`miri.yml`, `docs/current/ARCHITECTURE_MAP.md:577`): `ecs` (120m), `core` (90m). Kani CI weekly (`kani.yml`): `ecs` (120m), `math` (60m), `sdk` (60m), `core` (90m). 977 tests, 0 UB per CLAUDE.md and `MIRI_VALIDATION_REPORT.md` |
| 10 | C ABI entry points reject NULL handles and NULL string pointers without UB | Yes | `astraweave-sdk/src/lib.rs:177, 245, 265, 389-393` explicit null checks; `astraweave-sdk/src/lib_kani.rs` proves FFI safety properties |
| 11 | `Schedule::add_system` for an unregistered stage name does not panic — it either logs (debug) or silently drops (release) | Yes (behavioural) | `astraweave-ecs/src/lib.rs:708-724`. This is the documented behavioural contract — silent-drop is intentional for optional custom stages, while debug-mode logging surfaces typos |
| 12 | `Component` and `Resource` traits are blanket-implemented for `'static + Send + Sync` types — any such type can be both | Yes (compile-time) | `astraweave-ecs/src/lib.rs:85-90` |
| 13 | SIMD math functions handle NaN/Inf input — `debug_assert!(a.is_finite())` in debug; production paths fall through to scalar `glam` for non-x86_64 | Yes (debug) | E.g. `astraweave-math/src/simd_vec.rs:62-63`, `simd_quat.rs:33-34`. Release builds do not check finiteness |

---

## 9. Performance & Resource Profile

### Hot paths
- **`World::get` / `World::get_mut` / `World::each_mut`**: O(1) per call after the SparseSet integration (Week 10). Module header at `astraweave-ecs/src/system_param.rs:1-83` documents the journey from O(log n) BTreeMap to O(1) SparseSet, with 12-57× lookup speedup, frame time from 2.70 ms → 1.144 ms, and movement system from 1,000 µs → 106 µs.
- **`Query`/`Query2`/`Query2Mut`**: Per-entity overhead is ~4 O(1) operations. The `system_param.rs:36-83` discussion documents why true batch iteration (`&mut T` over a whole archetype column) is blocked by the borrow checker.
- **SIMD math primitives**: Per `astraweave-math/src/lib.rs:47-58`, typical speedups vs scalar `glam` are 1.75-2.5× (Vec3 dot 2.1×, Vec3 cross 2.3×, Vec3 normalize 2.4×, Mat4 multiply 2.5×, Quat slerp 1.75×).
- **`update_positions_simd`**: Batched `p += v*dt` at ~100-200 ns per 1,000 entities (2-4× over scalar) per `astraweave-math/src/simd_movement.rs:10-13`.

### Cold paths
- **`World::spawn` / `World::despawn`**: Amortized O(1) via free list; involves archetype lookup and SparseSet update. The cached `empty_archetype_id` at `astraweave-ecs/src/lib.rs:129-130, 204-212` avoids per-spawn signature creation and HashMap lookup.
- **Structural changes (insert/remove)**: O(component_count) for the new signature plus the cost of moving the entity to a new archetype. CommandBuffer batches these.
- **`Schedule::run`**: One traversal of stages × systems per tick. Cost is dominated by the systems themselves.
- **SDK `aw_world_tick`**: Includes legacy `World::tick` plus optional JSON snapshot serialization and delta computation. Reusable buffers (`astraweave-sdk/src/lib.rs:138-141`) avoid per-tick allocation; JSON serialization to `json_buf` via `serde_json::to_writer` (`lib.rs:190, 228`).

### Resource ownership
- **`astraweave_ecs::World`**: One per `App` (`astraweave-ecs/src/lib.rs:759-761`). Lifetime = App lifetime.
- **`astraweave_ecs::EntityAllocator`**: Owned by World. Lifetime = World lifetime.
- **`astraweave_ecs::ArchetypeStorage`**: Owned by World. Lifetime = World lifetime.
- **Resources (singletons)**: Owned by World's `HashMap<TypeId, Box<dyn Any>>` (`astraweave-ecs/src/lib.rs:124`). Type-erased; cannot be queried in `Query`.
- **`astraweave_core::World`**: Used as a Resource inside `astraweave_ecs::World` when `ecs_adapter::build_app` is used. Otherwise constructed standalone (e.g. by SDK or examples).
- **`EntityBridge`**: Resource inside ECS World; populated by `build_app`. Lifetime = App lifetime.
- **`AWWorld`**: Heap-allocated `Box<AwWorldWrap>` owned by the C host. Lifetime is host-managed; `aw_world_destroy` is the single deallocation point (`astraweave-sdk/src/lib.rs:163-173`).
- **`LAST_ERROR`**: Global `OnceLock<Mutex<String>>` for SDK error messages (`astraweave-sdk/src/lib.rs:357`).

---

## 10. Testing & Validation

- **Unit tests (inline `#[cfg(test)]` modules):**
  - `astraweave-ecs/src/lib.rs` — extensive inline tests (lines 906+: `test_spawn_and_insert`, `test_despawn`, `test_remove_component`, `test_query_single_component`, `test_query_two_components`, `test_resource_management`, `test_get_mut`, `test_count_*`, `test_entities_with_*`, …)
  - `astraweave-ecs/src/determinism_tests.rs`, `property_tests.rs`, `mutation_tests.rs`, `mutation_resistance_tests.rs`
  - `astraweave-core/src/sim.rs` — `test_sim_config_creation`, `test_step_*`
  - `astraweave-core/src/mutation_tests.rs`
  - `astraweave-math/src/mutation_tests.rs`
- **Integration tests (`tests/`):**
  - `astraweave-ecs/tests/`: 14 files — `world_app_tests`, `behavioral_correctness_tests`, `mutation_resistant_comprehensive_tests`, `panic_safety_tests`, `full_pipeline_integration`, `sparse_set_additional_tests`, `zero_alloc_tests`, `blob_vec_entity_allocator_tests`, `system_param_tests`, `ecs_core_tests`, `archetype_command_rng_tests`, `stress_tests`, `concurrency_tests`, `coverage_booster_ecs`
  - `astraweave-core/tests/`: 10 files — `ecs_integration_tests`, `full_system_determinism`, `tools_tests`, `cross_crate_stability_tests`, `performance_integration`, `simulation`, `mutation_resistant_comprehensive_tests`, `schema_tests`, `perception_tests`, `behavioral_correctness_tests`
  - `astraweave-math/tests/mutation_resistant_comprehensive_tests.rs`
  - `astraweave-sdk/tests/mutation_resistant_comprehensive_tests.rs`
- **Benchmarks (`benches/`):**
  - `astraweave-ecs/benches/`: `ecs_benchmarks.rs`, `storage_benchmarks.rs`
  - `astraweave-core/benches/`: `core_benchmarks.rs`, `full_game_loop.rs`
  - `astraweave-math/benches/`: `simd_benchmarks.rs`, `simd_mat_benchmarks.rs`, `simd_quat_benchmarks.rs`, `simd_movement.rs`
  - `astraweave-sdk/benches/`: `sdk_benchmarks.rs`, `sdk_adversarial.rs`
- **Miri validation (UB detection):**
  - CI workflow `.github/workflows/miri.yml`, weekly, nightly toolchain. Per `docs/current/ARCHITECTURE_MAP.md:577`: `ecs` (120m), `core` (90m). Per CLAUDE.md and `docs/current/MIRI_VALIDATION_REPORT.md`: 977 tests across `ecs`, `math`, `core`, `sdk` with ZERO undefined behaviour.
- **Kani formal verification:**
  - CI workflow `.github/workflows/kani.yml`, weekly. Per `docs/current/ARCHITECTURE_MAP.md:578`: `ecs` (120m), `math` (60m), `sdk` (60m), `core` (90m).
  - Proof files (per `docs/current/ARCHITECTURE_MAP.md:546-550`):
    - `astraweave-ecs/src/blob_vec_kani.rs` — BlobVec invariants
    - `astraweave-ecs/src/entity_allocator_kani.rs` — Generational index correctness
    - `astraweave-math/src/simd_vec_kani.rs` — SIMD operation correctness
    - `astraweave-core/src/schema_kani.rs` — Schema verification
    - `astraweave-sdk/src/lib_kani.rs` — FFI safety (buffer overflow, null pointer)
- **Mutation testing:** Wave 2 mutation testing campaigns covered ecs, math, core, sdk with mutation-resistant test suites under each crate's `tests/mutation_resistant_comprehensive_tests.rs`. Specific kill rates not enumerated in this trace.
- **Manual / example validation:** `examples/hello_companion`, `examples/ecs_ai_showcase`, `examples/profiling_demo`, `examples/weaving_playground` and the editor in `tools/aw_editor` exercise the foundation layer end-to-end every time they are run.

---

## 11. Open Questions / Parked Decisions

- **What is the long-term plan for the legacy `astraweave-core::World`?** [Decisional — requires Andrew's call on direction.] Factual state (verified 2026-05-10): `astraweave-core/src/ecs_components.rs:1` describes the components as "Phase 1 incremental migration". The bridge in `ecs_adapter::build_app` is in active use. Workspace-wide audit of `use astraweave_core::World` finds the legacy `World` consumed by: AI orchestrators, the SDK (`AwWorldWrap` wraps it), `validate_and_execute`, `capture_replay`, and the editor (`tools/aw_editor/`). Retiring the legacy World would require coordinated changes across all of those plus replacement of the `EntityBridge` machinery. No retirement campaign was identified in `docs/current/` or audit docs.
- **Is `astraweave-ecs/src/lib_new.rs` a draft or a live module?** [Decisional remainder only — factual portion resolved.] Factual state (verified 2026-05-10): dead/orphan. Not in the `pub mod` list at `astraweave-ecs/src/lib.rs:37-46`; zero workspace references via grep across both `use` statements and `mod` declarations; only two commits in history (original add `400903a18` + CI fmt sweep `2702232fb`). The remaining question is what to do with it: delete, keep as a scratch staging area for a future Bevy-style API refactor (per its module-doc comment), or preserve as historical residue.
- **Are there machine-checkable enforcements for Invariant 8 (LLM-prompt-name alignment)?** [Decisional remainder only — factual portion resolved.] Factual state (verified 2026-05-10): NO build-time enforcement exists. Workspace-wide `build.rs` audit returns only `examples/fluids_demo/build.rs` and `examples/sdk_c_harness/build.rs`, neither of which validates LLM prompt content against schema field names. No tests under `astraweave-llm/tests/`, `astraweave-core/tests/`, or workspace-level integration tests programmatically compare JSON-output key names to `WorldSnapshot`/`CompanionState`/`PlayerState`/etc. field names. The "Enforced by" column on Invariant 8 already records this as doc-only. The decisional remainder: whether to add a build-time check (e.g. parse `astraweave-llm/src/{prompts,compression,prompt_template}.rs` for the JSON output keys and assert that the canonical set matches a schema-derived list).
- **`Component` vs `Resource` as blanket impls — is the lack of a distinguishing trait method intentional, or a deferred tightening?** [Decisional — design intent inquiry.] Factual state (verified 2026-05-10): both traits are blanket-implemented at `astraweave-ecs/src/lib.rs:85-90`, with identical bounds (`'static + Send + Sync`). The same definition appears in the orphan `astraweave-ecs/src/lib_new.rs:12-16`. No record found in `docs/` or git log of either (a) a stricter trait definition being deliberately rejected, or (b) a planned tightening. The distinction is enforced only by the API (call `insert` vs `insert_resource`), which Rust compiles successfully either way.
- **Should the BlobVec and Box storage modes be unified?** [Decisional — "should" question.] Factual state (verified 2026-05-10): both still coexist per-archetype (`astraweave-ecs/src/archetype.rs:66-99`). Migration to BlobVec-only would require every component type used in the engine to implement `Clone` (BlobVec requirement at `astraweave-ecs/src/lib.rs:827`) and to be registered via `World::register_component::<T>()` before use. Workspace grep for `register_component::<` reveals which types are already on the BlobVec path. No active campaign was identified in `docs/current/` that targets unification.
- **What is the SDK's intended audience, and what is its versioning/stability commitment?** [Decisional remainder only — factual portion resolved.] Factual state (verified 2026-05-10): the SDK has exactly one in-tree consumer (`examples/sdk_c_harness`, which exercises `aw_version` / `aw_version_string` via `#[link]` + `extern "C"` per `examples/sdk_c_harness/src/main.rs:4-8`). The crate type at `astraweave-sdk/Cargo.toml:19` is `rlib, cdylib, staticlib` — supporting both Rust-link and C/C++/etc. static or dynamic linking. cbindgen 0.29 is a dev-dependency with explicit `package.metadata.cbindgen` config at `astraweave-sdk/Cargo.toml:33-36` for C header generation. SDK version is `0.4.0` (`astraweave-sdk/Cargo.toml:3`) — distinct from ecs/math/core at `0.1.0` and from the engine root version. The decisional remainder: whether to commit to a stability tier for the FFI surface, and what versioning policy applies.
- **`Schedule::add_system` silent-drop behaviour in release builds — is this the final disposition?** [Decisional — "final disposition" inquiry.] Factual state (verified 2026-05-10): the inline comment at `astraweave-ecs/src/lib.rs:712-724` describes this as a backward-compatibility concession for call sites that register to optional custom stages. Debug builds emit a single `eprintln!` warning per unknown-stage registration (`lib.rs:718-723`). No alternative disposition (e.g. hard panic, soft `Result` return, or registry of declared-optional stages) was found documented in the codebase, audit docs (including `docs/audits/parallel_schedule_binary_inventory_2026-04-18.md` §4 that originally surfaced the issue), or commit messages.
- **Multiple `WorldSnapshot` JSON serializers with different key conventions — should the LLM-facing JSON format be normalized to a single canonical encoder?** [Decisional — surfaced by 2026-05-10 deep investigation pass.] Factual state: three serializers emit JSON for LLM consumption: `astraweave-llm/src/prompts.rs:209-238` (translates Rust names: `pos`→`position`, `hp`→`health`, `pois`→`points_of_interest`); `astraweave-llm/src/compression.rs:139-300` (mostly preserves Rust names: emits `"pois"`, `"morale"`, `"cooldowns"`, `"ammo"`); `astraweave-llm/src/prompt_template.rs:227-251` (uses Rust names like `"pos"`, `"hp"`, `"k"` in static few-shot example strings the LLM is trained against). The three encoders are not cross-checked against each other or against the schema. Any LLM that has been trained against one encoder's outputs is implicitly bound to that encoder's keys. The decisional remainder: whether to consolidate to a single canonical encoder, accept the multiple-encoder state, or split the question into a Schema Stability vs Prompt Format ADR.

---

## 12. Maintenance Notes

**Update this doc when:**
- A new module is added to any of the four crates, or an existing module's role changes
- The canonical eight-stage list in `App::new()` (`astraweave-ecs/src/lib.rs:783-792`) changes
- The `WorldSnapshot`, `CompanionState`, `PlayerState`, `EnemyState`, `Poi`, `PlanIntent`, or `ActionStep` schema changes — especially field names (LLM-prompt impact)
- The dual `World`/`Entity` situation changes (e.g. legacy World retirement, EntityBridge replacement)
- The `unsafe` surface in `astraweave-ecs/src/blob_vec.rs`, `archetype.rs`, `entity_allocator.rs`, `sparse_set.rs`, `system_param.rs`, `command_buffer.rs`, `component_meta.rs` changes
- New Kani proofs or Miri test gates are added or removed
- The SDK C ABI gains or loses entry points
- A decision in Section 7 is superseded

**Verification process:**
- Spot-check the pipeline diagrams in Section 2 against `astraweave-ecs/src/lib.rs`, `astraweave-core/src/ecs_adapter.rs`, and `astraweave-sdk/src/lib.rs`
- Verify the file map in Section 5 still reflects actual file roles and the `pub mod`/`pub use` declarations in each crate's `lib.rs`
- Verify the invariants in Section 8 against the Miri/Kani CI status pages
- Update the metadata commit hash and date after verification

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**

1. **Two `World`s, two `Entity`s.** `astraweave_ecs::World` is the ECS substrate with generational entities. `astraweave_core::World` is the legacy HashMap-per-component struct with bare-`u32` entities. They are bridged inside `App` via `ecs_adapter::build_app`. Always namespace explicitly when both are in scope.
2. **ECS is single-threaded by design.** Do not add a parallel scheduler; the removal of `ParallelSchedule` is documented at `docs/audits/parallel_schedule_removal_2026-04-18.md`. Parallelism lives in subsystems (rayon, tokio, GPU compute), not in the ECS schedule.
3. **`WorldSnapshot` field names are load-bearing in LLM prompts.** Renaming `me`, `enemies`, `pois`, `obstacles`, `objective`, `me.pos`, `me.ammo`, `me.cooldowns`, etc. requires a coordinated LLM-prompt update — even after Rust compiles, the LLM has been trained against existing names.
4. **All `unsafe` in this layer must round-trip through Miri and Kani.** Don't ship new `unsafe` to `astraweave-ecs`, `astraweave-math`, `astraweave-core`, or `astraweave-sdk` without a Kani proof or a Miri-validated test and a `// SAFETY:` comment documenting the invariant.
5. **Stage names matter.** Registering a system on a stage name not in `App::new()`'s canonical eight will silently drop the registration in release builds.
6. **Stale entity handles are silent.** They do not panic — they return `None` or no-op. Code reasoning about entity liveness should call `World::is_alive` first if a stale handle could be meaningful.
7. **Math primitives are stateless and `glam`-compatible.** Take `glam::Vec3`/`Mat4`/`Quat` in, give them back. The SSE2 path uses `#[target_feature]` gating; non-x86_64 falls through to `glam`'s scalar path.
8. **The SDK has one in-tree consumer and an otherwise out-of-tree audience.** `examples/sdk_c_harness` exercises the FFI surface via `#[link]` + `extern "C"` (`examples/sdk_c_harness/src/main.rs:4-8`) and statically links the SDK. Beyond that example, no engine crate `use`s `astraweave-sdk` — so most C ABI changes affect out-of-tree embedders plus the one harness example.

**Files you'll most likely touch:**
- `astraweave-ecs/src/lib.rs` — World, App, Schedule
- `astraweave-ecs/src/archetype.rs` — archetype storage
- `astraweave-ecs/src/system_param.rs` — query types
- `astraweave-core/src/schema.rs` — shared types (heavy LLM impact)
- `astraweave-core/src/ecs_adapter.rs` — the bridge
- `astraweave-math/src/simd_*.rs` — SIMD primitives

**Files you should NOT touch without strong reason:**
- `astraweave-ecs/src/blob_vec.rs`, `archetype.rs`, `entity_allocator.rs`, `sparse_set.rs`, `command_buffer.rs`, `system_param.rs`, `component_meta.rs` — `unsafe` surface, Kani- and Miri-verified. Any change must round-trip through both before merge.
- `astraweave-math/src/simd_vec.rs`, `simd_mat.rs`, `simd_quat.rs` — `unsafe` SIMD intrinsics with `#[target_feature]` gating. Touching these requires re-running Miri and Kani.
- `astraweave-sdk/src/lib.rs` — FFI boundary. Any change risks breaking out-of-tree embedders.
- `astraweave-core/src/world.rs`, `ecs_adapter.rs`, `ecs_components.rs` — transitional bridge; changes here ripple through every consumer of the legacy World.
- `astraweave-ecs/src/lib_new.rs` — Dead orphan stub (not in `pub mod` list; zero workspace references). Treat as residue; do not extend or build on without first deciding its fate (see §11).

**Common mistakes when changing this system:**
- **Mistake**: Passing an `astraweave_core::Entity` (u32) where an `astraweave_ecs::Entity` is required (or vice versa).
  **Why wrong**: They are distinct types with different memory layouts. The compiler will catch the mismatch; the fix is `EntityBridge::get` / `EntityBridge::get_legacy`, not casting.
- **Mistake**: Adding a system on a stage name that isn't in `App::new()`'s canonical list, expecting it to run.
  **Why wrong**: Release builds silently drop unknown-stage registrations. Either use a canonical stage or extend `App::new()`.
- **Mistake**: Renaming a `WorldSnapshot` field as a refactor.
  **Why wrong**: LLM prompts reference the field name by string; renaming breaks Rust **and** AI behaviour in two separate places.
- **Mistake**: Adding `unsafe` to ECS storage code without updating the corresponding Kani proof.
  **Why wrong**: The Kani gate is the primary safety net for the type-erased storage layer.
- **Mistake**: Treating the legacy `astraweave-core::World` as deprecated and removing references.
  **Why wrong**: It is transitional, not deprecated. The SDK wraps it; `validate_and_execute` operates on it; `capture_replay` serializes it. Removal requires a coordinated migration plan that has not yet been recorded.

---

## Appendix B: Historical context

The foundation layer grew in two epochs.

The earlier epoch produced `astraweave-core::World` (the legacy HashMap-per-component struct in `astraweave-core/src/world.rs:49-69`) along with the canonical AI schema (`WorldSnapshot`, `PlanIntent`, `ActionStep`) in `astraweave-core/src/schema.rs`. The legacy World was the engine's authoritative state.

The later epoch introduced `astraweave-ecs` as an archetype-based substrate with generational entity indices, designed for cache-friendly iteration and deterministic execution. Rather than rewriting every consumer of the legacy World, `astraweave-core::ecs_adapter::build_app` was introduced as a bridge: it mints an ECS `App`, mirrors legacy entities into ECS components (`CPos`, `CHealth`, `CTeam`, `CAmmo`, `CCooldowns`), inserts the legacy `World` as an ECS resource, and registers `sys_sim` / `sys_move` / `sys_bridge_sync` / `sys_sync_to_legacy` to keep the two consistent. `EntityBridge` keeps the two ID spaces synchronized. This bridge is still in active use today; the long-term migration target is one of the open questions in §11.

`astraweave-sdk` was added as the embedding interface and wraps the legacy World rather than the ECS World. The SDK has one in-tree consumer — the `examples/sdk_c_harness` Rust binary that statically links and exercises the FFI surface for testing. Its primary audience is otherwise out-of-tree.

`astraweave-math` arrived during the Week 5-6 performance push (per the module headers at `astraweave-math/src/simd_vec.rs:1-21` and `astraweave-math/src/lib.rs:14-18`), bringing SIMD acceleration over `glam`. It carries no migration baggage and remains the cleanest of the four crates.

A `ParallelSchedule` parallel scheduler existed in `astraweave-ecs` behind a `parallel` feature for some time. After the 2026-04-18 audit chain (`docs/audits/parallel_schedule_experiment_2026-04-18.md`, `parallel_schedule_experiment_ecs_ai_showcase_2026-04-18.md`, `parallel_schedule_binary_inventory_2026-04-18.md`, `parallel_schedule_safety_audit_2026-04-18.md`) found zero default-features consumers and observed-incorrect output in the opt-in consumers, it was removed in `parallel_schedule_removal_2026-04-18.md`. The deterministic single-threaded `Schedule` is now canonical, and the comment block at `astraweave-ecs/src/lib.rs:78-80` is the in-source reminder.

The `pre_simulation`, `sync`, and `post_simulation` stages were added to `App::new()` on the same date after the binary-inventory audit surfaced that `Schedule::add_system` silently dropped systems registered to those names. The `sync` stage is specifically positioned between `simulation` and `ai_planning` so the legacy-World propagation runs after mutations and before AI planners read.
