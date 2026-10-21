# API Reference

> **Documentation Accuracy**: All exports listed below are verified against actual `lib.rs` files as of v0.4.0. Coverage percentages sourced from [Master Coverage Report](../performance/methodology.md).

Browse the Rust API documentation for all AstraWeave crates.

## Quick Links

| Crate | Description | Coverage | Status |
|-------|-------------|----------|--------|
| [astraweave-ecs](./ecs.md) | Entity Component System implementation | 83.2% | Production |
| [astraweave-core](./core.md) | Core ECS, simulation, and world systems | 79.1% | Production |
| [astraweave-ai](./ai.md) | AI orchestration and planning | 71.3% | Production |
| [astraweave-render](./render.md) | wgpu 25-based renderer | 67.4% | Production |
| [astraweave-physics](./physics.md) | Rapier3D 0.22 integration | 76.8% | Production |
| [astraweave-fluids](./fluids.md) | SPH fluid simulation | 94.2% | Production |
| [astraweave-nav](./nav.md) | Navigation and pathfinding | 72.1% | Production |
| [astraweave-gameplay](./gameplay.md) | Combat physics, damage | 68.9% | Production |
| [astraweave-terrain](./terrain.md) | Voxel terrain generation | 71.5% | Production |
| [astraweave-llm](./llm.md) | LLM integration (Ollama; runtime default phi3:medium) | 58.3% | Beta |
| [astraweave-behavior](./behavior.md) | Behavior trees, utility AI | 74.2% | Production |

## Core Engine Crates

### astraweave-ecs (83.2% coverage)

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

### astraweave-core (79.1% coverage)

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

### astraweave-ai (71.3% coverage)

AI orchestration layer with GOAP+LLM hybrid support:

| Module | Feature Gate | Description |
|--------|--------------|-------------|
| `core_loop` | — | Perception-Reasoning-Planning-Action loop |
| `orchestrator` | — | AI coordination trait |
| `tool_sandbox` | — | Tool validation and sandboxing |
| `async_task` | `llm_orchestrator` | Async LLM task wrapper |
| `llm_executor` | `llm_orchestrator` | LLM plan generation |
| `goap` | `llm_orchestrator` | Goal-oriented action planning |
| `AIArbiter` | `llm_orchestrator` | GOAP+LLM hybrid (101.7ns control) |

```rust
use astraweave_ai::{core_loop, orchestrator::Orchestrator};

// With llm_orchestrator feature:
#[cfg(feature = "llm_orchestrator")]
use astraweave_ai::{AIArbiter, goap::GoapOrchestrator};
```

[View Full Documentation →](./ai.md) | [Arbiter Guide →](../core-systems/ai/arbiter.md)

---

### astraweave-behavior (74.2% coverage)

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

### astraweave-llm (58.3% coverage)

LLM integration via Ollama (runtime default model phi3:medium; set OLLAMA_MODEL to override):

| Export | Description |
|--------|-------------|
| `LlmOrchestrator` | Provider-agnostic LLM coordination |
| `FallbackChain` | 4-tier fallback (Full LLM → Simplified → Heuristic → Emergency) |
| `OllamaClient` | Ollama API client |

```rust
use astraweave_llm::{LlmOrchestrator, FallbackChain};
```

*Generate rustdoc:* `cargo doc -p astraweave-llm --no-deps --open`

## Rendering & Graphics Crates

### astraweave-render (67.4% coverage)

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

### astraweave-physics (76.8% coverage)

Rapier3D 0.22 integration:

| Module | Performance | Description |
|--------|-------------|-------------|
| `rigid_body` | — | Physics bodies and dynamics |
| `collider` | — | Collision shapes and detection |
| `character_controller` | 114ns/move | Player movement |
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

### astraweave-fluids (94.2% coverage, 2,404 tests)

SPH fluid simulation (A+ grade):

| Module | Description |
|--------|-------------|
| `solver` | SPH pressure/viscosity solver |
| `surface_tension` | Surface tension forces |
| `boundary` | Domain boundary handling |
| `spatial_hash` | Neighbor lookup acceleration |

```rust
use astraweave_fluids::{FluidWorld, Particle, FluidConfig};
```

[View Full Documentation →](./fluids.md) | [Fluids Guide →](../core-systems/fluids.md)

---

### astraweave-nav (72.1% coverage)

Navigation and pathfinding:

| Module | Description |
|--------|-------------|
| `navmesh` | Navigation mesh generation |
| `pathfinding` | A* and hierarchical planning |
| `portal_graph` | Room-to-room navigation |
| `agent` | Navigation agent component |

```rust
use astraweave_nav::{Navmesh, PathQuery, Agent};
```

[View Full Documentation →](./nav.md)

---

## Gameplay Crates

### astraweave-gameplay (68.9% coverage)

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

### astraweave-terrain (71.5% coverage)

Procedural terrain generation:

| Module | Description |
|--------|-------------|
| `voxel_mesh` | Marching cubes (256 configurations) |
| `biome` | Biome distribution and blending |
| `chunk` | Terrain chunking (15.06ms/chunk) |

```rust
use astraweave_terrain::{VoxelMesh, Biome, TerrainChunk};
```

*Generate rustdoc:* `cargo doc -p astraweave-terrain --no-deps --open` | [Terrain Guide →](../core-systems/terrain.md)

---

## Infrastructure Crates

### astraweave-audio (69.2% coverage)

Spatial audio with rodio:

| Module | Description |
|--------|-------------|
| `audio_engine` | 4-bus mixer system (master, music, SFX, voice) |
| `spatial` | 3D audio positioning |
| `crossfade` | Music transitions |

```rust
use astraweave_audio::{AudioEngine, SpatialAudio};
```

[View Full Documentation →](./audio.md)

---

### astraweave-scene (74.6% coverage)

World streaming and partitioning:

| Module | Description |
|--------|-------------|
| `streaming` | Async cell loading |
| `partition` | World partitioning |

```rust
use astraweave_scene::{WorldCell, CellLoader};
```

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

## Coverage by Tier

| Tier | Crates | Avg Coverage | Status |
|------|--------|--------------|--------|
| **Tier 1** (Critical) | ecs, core, ai, render | 75.3% | ✅ Production |
| **Tier 2** (Important) | physics, nav, gameplay | 72.6% | ✅ Production |
| **Tier 3** (Supporting) | audio, scene, terrain | 71.8% | ✅ Production |
| **Tier 4** (Specialized) | fluids, llm, prompts | 71.5% | ✅ Production |

See [Master Coverage Report](../performance/methodology.md#coverage) for detailed breakdown.

## See Also

- [Crate Overview](../reference/crates.md) - High-level crate descriptions
- [Architecture](../architecture/overview.md) - System design
- [Examples](../examples/index.md) - Working code examples
- [Benchmarks](../performance/benchmarks.md) - Performance data
