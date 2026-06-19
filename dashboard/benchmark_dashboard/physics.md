---
layout: default
title: Physics Subsystem
---

# Physics (astraweave-physics)

AstraWeave wraps **Rapier3D 0.22** with extensive additional subsystems for game-ready physics simulation. The crate has **1,244 tests** and supports the `async-physics` feature for parallel execution.

## Subsystems

| Subsystem | Grade | Tests | Description |
|-----------|-------|-------|-------------|
| **Core / Character Controller** | A | 110+ | Movement, gravity, NaN safety, ground detection |
| **Spatial Hash** | A- | 41 | Grid structure with FxHashMap (dormant; active broadphase is Rapier's `DefaultBroadPhase`) |
| **Async Scheduler** | B+ | 13+ | 3-stage parallel pipeline (rayon) with timing stats |
| **Fluids (SPH)** | A+ | 2,404 | Reference SPH implementation (separate crate) |
| **Environment** | A- | 55+ | Wind forces, buoyancy, environmental effects |
| **Vehicle** | B+ | 50+ | Wheel/suspension model, drivetrain |
| **Gravity** | B+ | 30+ | Multi-body gravity, gravity zones |
| **Cloth** | B | 25+ | Verlet integration, constraint solver |
| **Ragdoll** | B | 33+ | Joint-based ragdoll, pose blending |
| **Destruction** | C+ | 17 | Fracture patterns, debris generation |
| **Projectile** | C+ | 21 | Ballistics, penetration, ricochet |

**Total**: 1,244+ tests passing (with `--features async-physics`)

## Spatial Hash

A grid-based spatial partitioning system for O(n log n) broadphase collision detection:

```rust
let mut grid = SpatialHash::new(2.0); // 2.0 unit cell size
grid.insert(entity_id, aabb);
let neighbors = grid.query(&query_aabb);
```

Key stats:
- **99.96% collision pair reduction** (499,500 → 180 checks for 1,000 entities)
- FxHashMap for fast integer hashing (no cryptographic overhead)
- Cache-locality cascading improves all downstream systems by 9–17%

## Async Scheduler

A 3-stage parallel pipeline using rayon for multi-core physics simulation:

```
Broad Phase → Barrier → Narrow Phase → Barrier → Integration
   (rayon)                  (rayon)                  (rayon)
```

```rust
let (broad, narrow, integrated) = scheduler.step_parallel_staged(
    bodies.par_iter(),
    |body| compute_aabb(body),       // Stage 1: Broad phase
    |aabb| detect_collision(aabb),   // Stage 2: Narrow phase
    |col| resolve_contact(col),      // Stage 3: Integration
);
```

With timing statistics for profiling:

```rust
let (result, stats) = scheduler.step_parallel_staged_with_stats(
    bodies.par_iter(), broad_fn, narrow_fn, integrate_fn,
);
println!("Broad: {:?}, Narrow: {:?}, Integrate: {:?}",
    stats.broad_phase, stats.narrow_phase, stats.integration);
```

**Note**: Only parallelize workloads >5 ms — rayon overhead is ~50–100 µs.

## Character Controller

Game-ready character movement with ground detection and gravity:

```rust
let move_result = controller.move_character(
    &physics_world,
    character_entity,
    desired_velocity * dt,
);
// 114 ns per move operation, NaN-safe
```

## Fluids (SPH)

The `astraweave-fluids` crate provides a reference Smoothed Particle Hydrodynamics implementation:

- **2,404 tests** — the most-tested subsystem in the entire engine
- Kernel functions (cubic spline, Wendland)
- Pressure solver
- Viscosity
- Surface tension
- Boundary conditions
- Particle neighbor search

## Environment Effects

| Effect | Description |
|--------|-------------|
| Wind | Directional wind forces applied to physics bodies |
| Buoyancy | Archimedes' principle for water/fluid interaction |
| Gravity zones | Localized gravity overrides (low-G, zero-G, reversed) |

## Vehicle Physics

Wheel-and-suspension model with:
- Per-wheel suspension spring/damper
- Drivetrain force distribution
- Tire friction model
- Steering geometry

## Cloth Simulation

Verlet integration cloth solver:
- Distance constraints between particles
- Self-collision prevention
- Pin constraints for attachment points
- Wind interaction

## Ragdoll System

Joint-based ragdoll physics:
- Hierarchical joint chain definition
- Blend between animation and physics
- Joint limits and motors
- Pose snapshotting for transitions

## Destruction

Fracture and destruction system:
- Voronoi-based fracture patterns
- Debris generation with mass distribution
- Force threshold triggering
- Structural integrity propagation

## Performance

| Operation | Latency |
|-----------|---------|
| Character move | 114 ns |
| Rigid body step | 2.97 µs |
| Full physics tick | 6.52 µs |
| Spatial hash broadphase | 3.77 ms |
| Collision pair reduction | 99.96% |

### 60 FPS Budget

Physics uses only **0.04%** of the 16.67 ms frame budget at standard workloads, leaving extensive headroom for complex scenes.

[← Back to Home](index.html) · [Architecture](architecture.html) · [Navigation](navigation.html)
