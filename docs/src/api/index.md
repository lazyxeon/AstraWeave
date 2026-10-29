# API Reference

Coverage percentages and "Production" grades that used to lead this page are removed. Whole-workspace coverage is a dated figure on the [engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md) page. `use` lines below were checked only where this edit says so. Generate rustdoc with `cargo doc -p <crate> --no-deps` before copying a signature.

Browse the Rust API documentation for all AstraWeave crates.

## Quick Links

| Crate | Description | Book page |
|-------|-------------|-----------|
| `astraweave-ecs` | Schedule and archetype storage | [ECS](./ecs.md) |
| `astraweave-core` | Legacy world, snapshots, validation | no separate page; see [architecture](../architecture/ecs.md) |
| `astraweave-ai` | Orchestrators. `AIArbiter` is feature-gated | [AI](./ai.md) |
| `astraweave-render` | wgpu path shared with the editor | [Render](./render.md) |
| `astraweave-physics` | Rapier 0.22 wrapper. Broadphase is `DefaultBroadPhase` | [Physics](./physics.md) |
| `astraweave-fluids` | Accent remnant, not the game-loop solver | [Fluids](./fluids.md) |
| `astraweave-nav` | `NavMesh` and `NavTiles` | [Navigation](./nav.md) |
| `astraweave-gameplay` | Gameplay systems, including `perform_attack_sweep` | no separate API page |
| `astraweave-terrain` | Terrain crate | [Terrain chapter](../core-systems/terrain.md) |
| `astraweave-llm` | Local model client. Orchestrator default model is `phi3:medium` | no separate API page |
| `astraweave-behavior` | Behavior graphs | no separate API page |

## Core Engine Crates

### astraweave-ecs

High-performance Entity Component System:

| Export | Description |
|--------|-------------|
| `World` | Container. `spawn` takes no bundle. `insert` adds a component. `insert_resource` / `get_resource` store singletons |
| `Entity` | `id`, `generation`, `to_raw`. `from_raw` is `unsafe` and takes one `u64` |
| `App` | Owns a `World` and a `Schedule`. `add_system` takes a stage name and `fn(&mut World)` |
| `Schedule` | Stage list. `run` walks stages on one thread |
| `Component` | Blanket trait: `impl<T: 'static + Send + Sync> Component for T` |
| `Resource` | Blanket trait with the same bounds. `World::insert_resource` does not require the trait |
| `Query` | `Query::<T>::new(&world)` iterates entities that have `T` |
| `CommandBuffer` | `spawn`, `insert`, `despawn`, then `flush(&mut world)` |
| `Events` | `send` / `read`. `Event` is a marker with no blanket impl |

```rust
use astraweave_ecs::{App, CommandBuffer, Entity, Events, Query, World};
```

[View Full Documentation →](./ecs.md)

---

### astraweave-core

Core engine systems and AI infrastructure:

| Module | Description |
|--------|-------------|
| `capture_replay` | `capture_state` and `replay_state`. There is no `CaptureReplay` type |
| `perception` | `PerceptionConfig` and `build_snapshot`. There is no `PerceptionFilter` type |
| `schema` | `WorldSnapshot`, `PlanIntent`, `ActionStep` |
| `sim` | Sim config and step helpers |
| `tool_sandbox` | `ToolBlock` and `map_engine_error` |
| `tool_vocabulary` | `get_all_tools`. The module comment says 37 action tools |
| `validation` | `validate_and_execute` |
| `world` | Legacy `World::spawn(name, pos, team, hp, ammo)` |

```rust
use astraweave_core::{
    perception::PerceptionConfig,
    schema::{ActionStep, PlanIntent, WorldSnapshot},
};
```

*Generate rustdoc:* `cargo doc -p astraweave-core --no-deps --open`

---

## AI & Behavior Crates

### astraweave-ai

AI orchestration layer with GOAP+LLM hybrid support:

| Module | Feature Gate | Description |
|--------|--------------|-------------|
| `core_loop` | — | Perception-Reasoning-Planning-Action loop |
| `orchestrator` | — | AI coordination trait |
| `tool_sandbox` | — | Tool validation and sandboxing |
| `async_task` | `llm_orchestrator` | Async LLM task wrapper |
| `llm_executor` | `llm_orchestrator` | LLM plan generation |
| `goap` | `planner_advanced` | Advanced GOAP module. `GoapOrchestrator` in `orchestrator.rs` is a different type and is always compiled |
| `AIArbiter` | `llm_orchestrator` | Not on `sys_ai_planning`. See the arbiter chapter |

```rust
use astraweave_ai::{core_loop, orchestrator::Orchestrator, GoapOrchestrator};

#[cfg(feature = "llm_orchestrator")]
use astraweave_ai::AIArbiter;
```

[View Full Documentation →](./ai.md) | [Arbiter Guide →](../core-systems/ai/arbiter.md)

---

### astraweave-behavior

Behavior systems:

- **BehaviorGraph** - Hierarchical behavior nodes (BehaviorNode enum)
- **BehaviorContext** - Shared context for tick execution
- **BehaviorStatus** - Success/Failure/Running states
- **UtilityAI** - Score-based decision making

```rust
use astraweave_behavior::{BehaviorGraph, BehaviorNode, BehaviorContext, BehaviorStatus};
```

*Generate rustdoc:* `cargo doc -p astraweave-behavior --no-deps --open`

---

### astraweave-llm

LLM integration via Ollama (runtime default model phi3:medium; set OLLAMA_MODEL to override):

| Export | Where |
|--------|-------|
| `LlmOrchestrator` | `astraweave-ai`, feature `llm_orchestrator`. Not an export of this crate |
| `FallbackOrchestrator` | `astraweave_llm::fallback_system`. There is no `FallbackChain` type |
| `OllamaClient` | `astraweave_llm::OllamaClient` |

```rust
// LlmOrchestrator is astraweave_ai, behind llm_orchestrator.
// The Qwen constant lives in astraweave_llm:
use astraweave_llm::qwen3_ollama::{Qwen3Ollama, DEFAULT_QWEN_INSTRUCT_MODEL};
```

*Generate rustdoc:* `cargo doc -p astraweave-llm --no-deps --open`

## Rendering & Graphics Crates

### astraweave-render

wgpu rendering. There is no `camera`, `forward`, `light`, `shadow`, `ibl_manager`, `post_fx_shader`, `animation_blending`, `skeleton`, `texture_array`, or `particle_system` module. Public modules in `astraweave-render/src/lib.rs` include `renderer`, `clustered`, `clustered_forward`, `deferred`, `ibl`, `shadow_csm`, `animation`, `skinning_gpu`, `mesh`, `culling`, `lod_generator`, `instancing`, `material`, `vertex_compression`, `water`, `volumetric_fog`, and `gpu_particles`.

`Camera` is not in this crate. `Material` and `Mesh` are re-exported from `types`. `Renderer` is re-exported from `renderer`.

```rust
use astraweave_render::{Material, Mesh, Renderer};
```

[View Full Documentation →](./render.md)

---

## Physics & Navigation Crates

### astraweave-physics

Rapier3D wrapper. There is no `rigid_body`, `collider`, or `character_controller` module. `PhysicsWorld`, `CharacterController`, `RigidBodyBuilder`, and `ColliderBuilder` are in the crate root (`RigidBodyBuilder` and `ColliderBuilder` are Rapier re-exports). `PhysicsWorld::broad_phase` is `DefaultBroadPhase`.

Public modules: `spatial_hash`, `async_scheduler`, `ecs`, `projectile`, `ragdoll`, `vehicle`, `environment`, `cloth`, `deformed`, `traversal`.

`SpatialHash::new` and `insert` exist. The world step uses Rapier's broadphase, not that helper.

```rust
use astraweave_physics::{CharacterController, PhysicsWorld};
use glam::vec3;

let mut physics = PhysicsWorld::new(vec3(0.0, -9.81, 0.0));
let _body = physics.create_ground_plane(vec3(50.0, 0.0, 50.0), 1.0);
physics.step();
let controller = CharacterController::new(0.3, 1.8);
let _grounded = controller.is_grounded();
```

[View Full Documentation →](./physics.md)

---

### astraweave-fluids

Not a game-loop solver. `PCISPHSystem` is not in the crate. See [Fluids API](./fluids.md).

There is no `solver`, `surface_tension`, `boundary`, or `spatial_hash` module. Public modules include `anisotropic`, `caustics`, `emitter`, `foam`, `renderer`, `sdf`, `underwater`, `water_effects`, and `waterfall`. `FluidSystem` and `Particle` are in `lib.rs`.

```rust
use astraweave_fluids::{FluidSystem, Particle};
```

[View Full Documentation →](./fluids.md) | [Fluids Guide →](../core-systems/fluids.md)

---

### astraweave-nav

Navigation and pathfinding:

The crate has a private `tiles` module. It re-exports `NavTiles` and `NavMesh` from the crate root. There is no `navmesh`, `pathfinding`, `portal_graph`, or `agent` module. `NavMesh::find_path` returns `Vec<Vec3>`. `NavTiles::query_path` is the hashed route.

```rust
use astraweave_nav::{NavMesh, NavTiles};
```

[View Full Documentation →](./nav.md)

---

## Gameplay Crates

### astraweave-gameplay

Combat and game mechanics:

Public modules include `combat`, `combat_physics`, `crafting`, `dialogue`, `items`, `quests`, and `stats`. There is no `damage_system` or `ability_system` module.

```rust
use astraweave_gameplay::combat_physics::perform_attack_sweep;
```

*Generate rustdoc:* `cargo doc -p astraweave-gameplay --no-deps --open`

---

### astraweave-terrain

Procedural terrain generation:

There is no `voxel_mesh` module. The crate does not record a `15.06ms/chunk` figure. Public modules include `biome`, `chunk`, `meshing`, `marching_cubes_tables`, and `voxel_data`.

`Biome` is `astraweave_terrain::biome::Biome`. `TerrainChunk` is `astraweave_terrain::chunk::TerrainChunk`. Neither name is re-exported from `lib.rs`. There is no `VoxelMesh` type in `lib.rs`.

*Generate rustdoc:* `cargo doc -p astraweave-terrain --no-deps --open` | [Terrain Guide →](../core-systems/terrain.md)

---

## Infrastructure Crates

### astraweave-audio

Public modules are `engine`, `voice`, and `dialogue_runtime`. There is no `audio_engine`, `spatial`, or `crossfade` module. `AudioEngine`, `ListenerPose`, `MusicTrack`, `PanMode`, and `EmitterId` are re-exported from `engine`. Volumes are `set_master_volume`, `set_music_volume`, `set_sfx_volume`, and `set_voice_volume`. `play_music` takes a `MusicTrack` and a `crossfade_sec: f32`.

```rust
use astraweave_audio::{AudioEngine, ListenerPose};
use glam::vec3;

fn open() -> anyhow::Result<()> {
    let mut audio = AudioEngine::null()?;
    audio.set_master_volume(1.0);
    audio.update_listener(ListenerPose {
        position: vec3(0.0, 0.0, 0.0),
        forward: vec3(1.0, 0.0, 0.0),
        up: vec3(0.0, 1.0, 0.0),
    });
    audio.tick(1.0 / 60.0);
    Ok(())
}
```

[View Full Documentation →](./audio.md)

---

### astraweave-scene

World streaming and partitioning:

| Module | Description |
|--------|-------------|
| `streaming` | Async cell loading |
| `partition` | World partitioning |

`WorldCell` and `CellLoader` were not found as public types in `astraweave-scene/src/lib.rs` during this pass. Use `cargo doc -p astraweave-scene --no-deps`.

*Generate rustdoc:* `cargo doc -p astraweave-scene --no-deps --open`

---

## Building API Docs Locally

```bash
# Generate all API documentation
cargo doc --workspace --no-deps --open

# Generate for specific crate with all features
cargo doc --package astraweave-ai --all-features --no-deps --open

# Generate with private items (for internal development)
cargo doc --package astraweave-core --document-private-items --no-deps --open
```

Per-crate coverage tiers are not repeated here. The dated whole-workspace figure is on the engine status page.

## See Also

- [Crate Overview](../reference/crates.md) - High-level crate descriptions
- [Architecture](../architecture/overview.md) - System design
- [Examples](../examples/index.md) - Working code examples
- [Benchmarks](../performance/benchmarks.md) - Performance data
