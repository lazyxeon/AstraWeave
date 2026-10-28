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
| `World` | Main container for all ECS data |
| `Entity` | Lightweight entity handles with generational indices |
| `App` | Application builder and runner |
| `Schedule` | System scheduling and ordering |
| `Component` | Trait for component data |
| `Resource` | Singleton data storage |
| `Query` | Efficient component access patterns |
| `CommandBuffer` | Deferred entity operations |
| `Events` | Event queues and readers |

```rust
use astraweave_ecs::{World, Entity, App, Schedule, Component, Resource, Query, CommandBuffer};
```

[View Full Documentation →](./ecs.md)

---

### astraweave-core

Core engine systems and AI infrastructure:

| Module | Description |
|--------|-------------|
| `capture_replay` | Deterministic frame recording/playback |
| `perception` | AI world observation and filtering |
| `schema` | WorldSnapshot, PlanIntent, ActionStep |
| `sim` | Game state management |
| `tool_sandbox` | Secure AI action execution |
| `tool_vocabulary` | 37-tool AI action vocabulary |
| `validation` | Input/output validation |
| `world` | World state management |

```rust
use astraweave_core::{
    schema::{WorldSnapshot, PlanIntent, ActionStep},
    capture_replay::CaptureReplay,
    perception::PerceptionFilter,
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

| Export | Description |
|--------|-------------|
| `LlmOrchestrator` | Provider-agnostic LLM coordination |
| `FallbackChain` | 4-tier fallback (Full LLM → Simplified → Heuristic → Emergency) |
| `OllamaClient` | Ollama API client |

```rust
// LlmOrchestrator is astraweave_ai, behind llm_orchestrator.
// The Qwen constant lives in astraweave_llm:
use astraweave_llm::qwen3_ollama::{Qwen3Ollama, DEFAULT_QWEN_INSTRUCT_MODEL};
```

*Generate rustdoc:* `cargo doc -p astraweave-llm --no-deps --open`

## Rendering & Graphics Crates

### astraweave-render

wgpu 25.0.2 based rendering (40+ modules):

| Category | Key Modules |
|----------|-------------|
| **Pipeline** | `camera`, `clustered`, `deferred`, `forward`, `renderer` |
| **Animation** | `animation`, `animation_blending`, `skeleton`, `skinning_gpu` |
| **Lighting** | `light`, `shadow`, `ibl_manager`, `post_fx_shader` |
| **Geometry** | `mesh`, `culling`, `lod_generator`, `instancing` |
| **Materials** | `material`, `texture_array`, `vertex_compression` |
| **Effects** | `water`, `volumetric`, `particle_system` |

```rust
use astraweave_render::{
    camera::Camera,
    material::Material,
    mesh::Mesh,
    renderer::Renderer,
};
```

[View Full Documentation →](./render.md)

---

## Physics & Navigation Crates

### astraweave-physics

Rapier3D 0.22 integration:

| Module | Performance | Description |
|--------|-------------|-------------|
| `rigid_body` | — | Physics bodies and dynamics |
| `collider` | — | Collision shapes and detection |
| `character_controller` | not restated here | Player movement. The 114 ns figure is not on the engine status page |
| `spatial_hash` | dormant (test-only) | Dormant; actual broadphase is Rapier's `DefaultBroadPhase` |
| `async_scheduler` | — | Parallel physics stepping |

```rust
use astraweave_physics::{
    rigid_body::RigidBody,
    collider::Collider,
    character_controller::CharacterController,
};
```

[View Full Documentation →](./physics.md)

---

### astraweave-fluids

Not a game-loop solver. `PCISPHSystem` is not in the crate. See [Fluids API](./fluids.md).

| Module | Description |
|--------|-------------|
| `solver` | SPH pressure/viscosity solver |
| `surface_tension` | Surface tension forces |
| `boundary` | Domain boundary handling |
| `spatial_hash` | Neighbor lookup acceleration |

```rust
use astraweave_fluids::{FluidSystem, Particle};
```

[View Full Documentation →](./fluids.md) | [Fluids Guide →](../core-systems/fluids.md)

---

### astraweave-nav

Navigation and pathfinding:

| Module | Description |
|--------|-------------|
| `navmesh` | Navigation mesh generation |
| `pathfinding` | A* and hierarchical planning |
| `portal_graph` | Room-to-room navigation |
| `agent` | Navigation agent component |

```rust
use astraweave_nav::{NavMesh, NavTiles};
```

[View Full Documentation →](./nav.md)

---

## Gameplay Crates

### astraweave-gameplay

Combat and game mechanics:

| Module | Description |
|--------|-------------|
| `combat_physics` | Raycast attacks, parry, iframes |
| `damage_system` | Damage calculation |
| `ability_system` | Ability cooldowns and effects |

```rust
use astraweave_gameplay::combat_physics::perform_attack_sweep;
```

*Generate rustdoc:* `cargo doc -p astraweave-gameplay --no-deps --open`

---

### astraweave-terrain

Procedural terrain generation:

| Module | Description |
|--------|-------------|
| `voxel_mesh` | Marching cubes (256 configurations) |
| `biome` | Biome distribution and blending |
| `chunk` | Terrain chunking (15.06ms/chunk) |

`VoxelMesh`, `Biome`, and `TerrainChunk` were not found as public types in `astraweave-terrain/src/lib.rs` during this pass. Use `cargo doc -p astraweave-terrain --no-deps`.

*Generate rustdoc:* `cargo doc -p astraweave-terrain --no-deps --open` | [Terrain Guide →](../core-systems/terrain.md)

---

## Infrastructure Crates

### astraweave-audio

Spatial audio with rodio:

| Module | Description |
|--------|-------------|
| `audio_engine` | 4-bus mixer system (master, music, SFX, voice) |
| `spatial` | 3D audio positioning |
| `crossfade` | Music transitions |

```rust
use astraweave_audio::AudioEngine;
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
