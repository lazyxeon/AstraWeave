# Optimization Guide

This guide covers performance optimization techniques for AstraWeave games.

## General Principles

### Measure First

> **Never optimize without profiling.**

Before optimizing:
1. Run benchmarks to establish baseline
2. Profile with Tracy or `cargo flamegraph`
3. Identify actual bottlenecks
4. Optimize the measured slow path

### Amdahl's Law

Only parallelize work that benefits from it:
- Sequential overhead limits speedup
- 0.15-22.4% of work is parallelizable in typical games
- Maximum theoretical speedup: ~1.24× in many cases

---

## ECS Optimization

### Batching Over Scattering

```rust
// ❌ Slow: Scattered access
for entity in entities {
    let pos = world.get_mut::<Position>(entity);
    pos.x += 1.0;
}

// ✅ Fast: Batch access (3-5× faster)
let mut positions: Vec<_> = world.query::<&mut Position>().iter_mut().collect();
for pos in &mut positions {
    pos.x += 1.0;
}
```

**Why**: Archetype lookup is O(log n) per access. Batching amortizes this cost.

### Component Layout

- Keep frequently-accessed components small
- Group components that are accessed together
- Use `u32` over `u64` when possible (cache efficiency)

### Entity spawning

`astraweave-ecs` has no `spawn_batch`, and `World::spawn` does not take a tuple of components. The legacy `astraweave_core::World::spawn` used by `hello_companion` takes a name, an `IVec2`, a `Team`, hp, and ammo. Do not copy a Bevy spawn loop into this engine.

---

## AI Optimization

### GOAP

`astraweave-ai` has no `GoapPlanner` and no `with_cache_size` builder. `astraweave-behavior` defines `GoapPlanner` in `astraweave-behavior/src/goap.rs`. Advanced GOAP in `astraweave-ai` is `astraweave_ai::goap::AdvancedGOAP` behind `planner_advanced`. The always-compiled `GoapOrchestrator` calls `htn::legacy_goap_plan`. Cache-hit nanoseconds are not on the engine status page.

### Agent Update Staggering

```rust
// Update 1/10th of agents per frame (10-frame rotation)
for (i, agent) in agents.iter_mut().enumerate() {
    if i % 10 == frame_count % 10 {
        agent.update(&snapshot);
    }
}
```

### AIArbiter Cooldown

```rust
#[cfg(feature = "llm_orchestrator")]
let arbiter = AIArbiter::with_single_executor(executor, goap, bt)
    .with_llm_cooldown(15.0);
```

`AIArbiter::new` takes an `Option<LlmExecutor>` before the orchestrators. The type is not on the sim tick. See [AI Arbiter](../core-systems/ai/arbiter.md).

---

## Physics Optimization

### Broadphase

`PhysicsWorld` stores Rapier's `DefaultBroadPhase`. There is no `PhysicsWorld::with_spatial_hash`. `SpatialHash` is a separate type and is not the broadphase the world steps.

`PhysicsWorld::step_batch` and `enable_sleeping` with the call shape previously shown here are not in `astraweave-physics/src/lib.rs`. Step the world through `PhysicsWorld`. Picosecond tables that used to follow this section are not on the engine status page.

---

## Rendering Optimization

### Frustum Culling

Frustum check: 889-915 ps (essentially free)

```rust
// Automatic in renderer
renderer.enable_frustum_culling(true);
```

### LOD Selection

```rust
// Configure LOD distances
mesh.set_lod_distances(&[10.0, 50.0, 100.0, 500.0]);
```

### Instanced Rendering

Instance overhead: 1.43-1.52 ns per calculation

```rust
// Batch identical meshes
let instances = entities
    .filter(|e| e.mesh_id == mesh_id)
    .map(|e| e.transform)
    .collect();
renderer.draw_instanced(mesh, &instances);
```

---

## Memory Optimization

### SparseSet vs BTreeMap

| Operation | SparseSet | BTreeMap | Winner |
|-----------|-----------|----------|--------|
| Lookup (1000) | 1.56 ns | 59 µs | 37× SparseSet |
| Insert (1000) | 9.9 ns | 129 ns | 13× SparseSet |

AstraWeave uses SparseSet for entity storage—architecture decision validated.

### Component Pooling

```rust
// Reuse allocations
let mut pool = ComponentPool::<Particle>::new(10000);
for _ in 0..10000 {
    let particle = pool.allocate();
    // Use particle...
    pool.deallocate(particle);
}
```

---

## Profiling Tools

### Tracy Integration

```rust
// Add Tracy spans to hot paths
#[cfg(feature = "profiling")]
astraweave_profiling::span!("AI::Update");
```

### Criterion Benchmarks

```bash
# Profile specific benchmark
cargo bench -p astraweave-ecs -- entity_spawn/empty/10000 --profile-time 10

# Generate flamegraph
cargo flamegraph -- --bench entity_bench
```

### Frame Timing

```rust
// Log frame timing
let start = Instant::now();
// ... game loop ...
let frame_time = start.elapsed();
if frame_time > Duration::from_millis(16) {
    warn!("Frame budget exceeded: {:?}", frame_time);
}
```

---

## Common Pitfalls

### 1. Over-Parallelization

**Problem**: Adding Rayon to small workloads

**Why**: Thread pool overhead (~50-100 µs) exceeds benefit for <5 ms work

**Solution**: Only parallelize workloads >5 ms

### 2. Allocations in Hot Paths

**Problem**: `Vec::new()` in per-frame code

**Solution**: Pre-allocate and reuse

```rust
// ❌ Bad: Allocates every frame
fn update(&mut self) {
    let temp = Vec::new();  // Allocation!
    // ...
}

// ✅ Good: Reuse allocation
fn update(&mut self) {
    self.temp_buffer.clear();  // No allocation
    // ...
}
```

### 3. Debug vs Release

**Problem**: Testing performance in debug mode

**Why**: Debug can be 10-100× slower

**Solution**: Always benchmark with `--release`

```bash
cargo bench -p astraweave-ecs  # Automatically uses release
```

---

## See Also

- [Benchmarks](./benchmarks.md) - Current performance data
- [Methodology](./methodology.md) - How we measure
- [Performance Budgets](./budgets.md) - Frame allocation
