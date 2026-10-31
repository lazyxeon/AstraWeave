---
layout: default
title: ECS Subsystem
---

# Entity Component System (astraweave-ecs)

AstraWeave's ECS is an archetype-based system with deterministic execution ordering, 7 fixed system stages, typed event channels, and comprehensive memory-safety validation.

## Key Features

- **Archetype storage**: Components grouped by type-set for cache-friendly iteration
- **Deterministic ordering**: Fixed 60 Hz tick with ordered entity iteration
- **System stages**: 8 stages executed in strict sequence every frame
- **Events**: Typed event channels for decoupled inter-system communication
- **Generation-based entity IDs**: Prevents use-after-free with generational indices
- **Miri-validated**: 330 tests with zero undefined behavior
- **Kani-verified**: Formal proofs for entity allocation soundness

## Core Types

### World

The central data store for all entities and components:

```rust
let mut world = World::new();              // 25.8 ns
let entity = world.spawn();                // 420 ns
world.insert(entity, Position { x: 0.0, y: 0.0 });
let pos = world.get::<Position>(entity);   // archetype-indexed lookup
```

### Components

Any `'static + Send + Sync` type is automatically a component — no derive macros needed:

```rust
pub struct Position { pub x: f32, pub y: f32 }
pub struct Velocity { pub dx: f32, pub dy: f32 }
pub struct Health(pub f32);
pub struct AiTag;  // marker components are zero-sized
```

### System Stages

Systems execute in this deterministic order every frame at 60 Hz:

| # | Stage | Purpose | Example Systems |
|---|-------|---------|-----------------|
| 1 | `PRE_SIMULATION` | Setup, initialization | Resource loading, state init |
| 2 | `PERCEPTION` | Build WorldSnapshots | AI sensor updates, visibility checks |
| 3 | `SIMULATION` | Game logic, cooldowns | Combat resolution, state machines |
| 4 | `AI_PLANNING` | Generate PlanIntents | Orchestrator dispatch, arbiter ticks |
| 5 | `PHYSICS` | Forces, collisions | Rapier3D step, spatial hash queries |
| 6 | `POST_SIMULATION` | Cleanup, constraints | Constraint resolution, entity GC |
| 7 | `PRESENTATION` | Rendering, audio, UI | Draw calls, audio playback, UI layout |

### Registration

```rust
app.add_system(SystemStage::PERCEPTION, build_ai_snapshots);
app.add_system(SystemStage::AI_PLANNING, orchestrator_tick);
app.add_system(SystemStage::PHYSICS, physics_step);
app.add_system(SystemStage::PRESENTATION, render_frame);
```

### Events

Typed event channels enable decoupled communication between systems:

```rust
// Emit an event
world.send_event(DamageEvent { target: entity, amount: 25.0 });

// Read events in a system
for event in world.read_events::<DamageEvent>() {
    // handle damage
}
```

## Performance

| Operation | Latency | Throughput |
|-----------|---------|------------|
| World creation | 25.8 ns | 38.8M/sec |
| Entity spawn (empty) | 50 ns | 20M/sec |
| Entity spawn (with components) | 420 ns | 2.38M/sec |
| Per-entity tick | <1 ns | >1B/sec |
| Component add | ~100 ns | 10M/sec |

### Best Practice: Batching for Performance

The collect → process → writeback pattern yields 3–5× speedups over scattered access:

```rust
// FAST: Collect into contiguous buffer, then batch process
let batch: Vec<_> = query.iter().collect();
update_positions_simd(&mut batch, &velocities, dt);
// Write back to ECS

// SLOW: Per-entity archetype lookup (O(log n) each time)
for entity in query.iter() {
    let pos = world.get_mut::<Position>(entity);
    pos.x += vel.dx * dt;
}
```

## Memory Safety

All unsafe code in the ECS has been validated with both Miri and Kani:

### Miri Validation (330 tests, 0 UB)

| Pattern | Description |
|---------|-------------|
| **BlobVec** | Untyped heap storage for archetype columns — raw pointer arithmetic, alignment |
| **SparseSet** | Entity-indexed sparse storage — bounds checking, pointer validity |
| **EntityAllocator** | Generation-based entity recycling — prevents use-after-free |
| **SystemParam** | Type-erased system parameter access — lifetime correctness |

### Kani Proofs

Formal verification proofs confirm:
- Entity allocation never produces duplicate IDs
- Generational indices correctly invalidate stale references
- Component storage operations maintain memory safety invariants

Miri flags: `-Zmiri-symbolic-alignment-check -Zmiri-strict-provenance`

## Integration Points

- **AI**: Systems at `PERCEPTION` and `AI_PLANNING` stages build snapshots and dispatch orchestrators
- **Physics**: `PHYSICS` stage integrates Rapier3D step results back into ECS components
- **Rendering**: `PRESENTATION` stage extracts transform + mesh data for GPU submission
- **Scene**: Scene graph nodes map to entities with `Transform` components

[← Back to Home](index.html) · [Architecture](architecture.html) · [Crate Index](crates.html)
