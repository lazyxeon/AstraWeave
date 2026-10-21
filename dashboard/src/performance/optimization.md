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

### Entity Spawning

```rust
// ❌ Slow: One-by-one spawning
for _ in 0..1000 {
    world.spawn((Position::default(), Velocity::default()));
}

// ✅ Fast: Batch spawning
world.spawn_batch((0..1000).map(|_| (Position::default(), Velocity::default())));
```

---

## AI Optimization

### GOAP Caching

GOAP cache hit: 9.8 ns vs cache miss: 286 ns (29× faster)

```rust
// Enable GOAP caching
let planner = GoapPlanner::new()
    .with_cache_size(1000)  // Cache 1000 plans
    .with_cache_ttl(Duration::from_secs(5));
```

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
// Reduce LLM request frequency for better performance
let arbiter = AIArbiter::new(executor, goap, bt)
    .with_llm_cooldown(15.0);  // 15 seconds between requests
```

---

## Physics Optimization

### Spatial Hashing

AstraWeave's spatial hash reduces collision checks by 99.96%:

```rust
// Automatic with default physics setup
let physics = PhysicsWorld::new()
    .with_spatial_hash(cell_size: 2.0);  // Tune cell size to object density
```

**Tuning**: Cell size should be ~2× average object radius.

### Rigid Body Batching

```rust
// Batch physics updates (47µs for 100 bodies)
physics.step_batch(&mut bodies, dt);
```

### Sleep Optimization

```rust
// Enable sleeping for static objects
body.enable_sleeping(linear_threshold: 0.01, angular_threshold: 0.01);
```

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
