# Deterministic Simulation
AstraWeave provides bit-identical deterministic simulation across all core systems. This enables reproducible AI behavior, lockstep multiplayer, replay systems, and reliable regression testing.

## Why Determinism Matters

For AI-native gameplay, determinism is **critical**:

```rust
// Without determinism (BAD):
// Run 1: Entity 42 attacks first (random HashMap iteration)
// Run 2: Entity 17 attacks first (different outcome!)
// Multiplayer: Host and client see different results

// With determinism (GOOD):
// Run 1, 2, 3...: Same entity order, same decisions, same outcome
// Multiplayer: All clients see identical simulation
// Replays: Record inputs, reproduce exact gameplay
```

**Use Cases:**
- **Reproducible AI**: Same world state → same AI decisions
- **Lockstep Multiplayer**: Only sync inputs, not full game state
- **Replay Systems**: Record/playback for debugging and spectating
- **Regression Testing**: Validate AI changes don't break behavior
- **Competitive Gaming**: Provably fair gameplay

## Determinism Guarantees

### What IS Guaranteed

| System | Guarantee |
|--------|-----------|
| **ECS Iteration** | Archetypes visited in ID order (BTreeMap) |
| **Within-Archetype** | Entities maintain relative order |
| **Repeated Iterations** | Same order every time |
| **Cross-World** | Same operations → same archetype IDs |
| **Physics** | Deterministic parallel iteration (Rayon) |
| **Capture/Replay** | Bit-identical state restoration |
| **Events** | FIFO ordering |

### What is NOT Guaranteed

- **Spawn order across archetypes**: Entity order changes when components added/removed
- **Floating-point across platforms**: Different CPUs may produce slightly different results
- **Non-deterministic RNG**: Must use seeded RNG explicitly

## Fixed-Tick Simulation

AstraWeave runs physics and AI at exactly 60Hz:

```rust
const TICK_RATE: f64 = 60.0;
const TICK_DURATION: Duration = Duration::from_nanos(16_666_667);

// Every simulation step advances by exactly this amount
pub fn step(world: &mut World, cfg: &SimConfig) {
    world.tick(cfg.dt);  // cfg.dt = 1/60 second
}
```

**Benefits:**
- Consistent timing across different hardware
- Decoupled from render frame rate
- Predictable performance testing
- Reliable networking

## ECS Determinism

### BTreeMap Storage

The ECS uses `BTreeMap` instead of `HashMap` for archetype storage:

```rust
/// CRITICAL: BTreeMap for deterministic iteration by ID
pub struct ArchetypeSet {
    archetypes: BTreeMap<ArchetypeId, Archetype>,
    // NOT HashMap - iteration order would be non-deterministic!
}
```

**Why this matters:**
- HashMap iteration order depends on hash function and memory layout
- BTreeMap iteration order is sorted by key (deterministic)
- AI agents iterate entities in identical order every run

### Entity Iteration Order

```rust
// Entities are stored per-archetype
let e1 = world.spawn();  // Empty archetype (ID 0)
let e2 = world.spawn();  // Empty archetype (ID 0)
world.insert(e1, Position::new(1.0, 1.0));  // Moves e1 to Position archetype (ID 1)

// Iteration order: [e2, e1]
// - Archetype 0 (empty) visited first: contains e2
// - Archetype 1 (Position) visited second: contains e1
// NOT spawn order [e1, e2] - but IS deterministic!
```

### Preserving Spawn Order

If spawn order is critical, track it explicitly:

```rust
#[derive(Component, Clone, Copy)]
struct SpawnOrder(u64);

// When spawning:
let e = world.spawn();
world.insert(e, SpawnOrder(world.entity_count() as u64));

// When iterating in spawn order:
let mut entities = world.entities_with::<SpawnOrder>();
entities.sort_by_key(|&e| world.get::<SpawnOrder>(e).unwrap().0);
```

## Capture & Replay

AstraWeave provides state capture for debugging and determinism validation:

```rust
use astraweave_core::{capture_state, replay_state, SimConfig};

// Capture world state at tick 100
capture_state(100, "save.json", &world)?;

// Later: Replay from saved state
let cfg = SimConfig { dt: 1.0 / 60.0 };
let world = replay_state("save.json", 100, &cfg)?;  // Run 100 ticks
```

### Snapshot Format

```json
{
  "tick": 100,
  "world": {
    "t": 1.6666,
    "next_id": 42,
    "obstacles": [[5, 10], [0, 0], [15, 20]]
  }
}
```

**Stability guarantees:**
- Obstacles sorted for consistent serialization
- JSON format for human readability
- Minimal state for fast save/load

## Physics Determinism

### Parallel Processing

Physics uses Rayon with deterministic iteration:

```rust
/// Parallel iterator helpers for deterministic physics
impl PhysicsParallelOps {
    /// Process rigid bodies in parallel (deterministic order)
    pub fn par_process_bodies<F>(&self, bodies: &mut [RigidBody], f: F)
    where
        F: Fn(&mut RigidBody) + Sync,
    {
        // Rayon's par_iter preserves element order
        bodies.par_iter_mut().for_each(f);
    }
}
```

### Fixed Timestep

Physics simulation uses fixed timesteps:

```rust
// Physics world with deterministic gravity
let physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

// Fixed step (not variable dt!)
physics.step(1.0 / 60.0);  // Always 16.67ms
```

## Testing Determinism

### Validation Tests

The ECS includes comprehensive determinism tests:

```rust
#[test]
fn test_query_iteration_deterministic() {
    let mut world = World::new();
    
    // Spawn entities with components
    for i in 0..100 {
        let e = world.spawn();
        world.insert(e, Position { x: i as f32, y: 0.0 });
    }
    
    // First iteration
    let order1: Vec<Entity> = world.entities_with::<Position>().collect();
    
    // Second iteration - must be identical
    let order2: Vec<Entity> = world.entities_with::<Position>().collect();
    
    assert_eq!(order1, order2, "Iteration order must be deterministic");
}
```

### Multi-Run Verification

```rust
#[test]
fn test_determinism_across_runs() {
    let results: Vec<Vec<Entity>> = (0..5)
        .map(|_| {
            let mut world = World::new();
            // Same operations each run
            setup_world(&mut world);
            world.entities_with::<AIAgent>().collect()
        })
        .collect();
    
    // All runs must produce identical results
    for run in &results[1..] {
        assert_eq!(&results[0], run);
    }
}
```

## Best Practices

### DO

```rust
// ✅ Use fixed timestep
world.step(FIXED_DT);

// ✅ Use seeded RNG
let mut rng = StdRng::seed_from_u64(12345);

// ✅ Use BTreeMap for deterministic iteration
let entities: BTreeMap<EntityId, Entity> = ...;

// ✅ Sort before iteration if order matters
let mut entities = query.iter().collect::<Vec<_>>();
entities.sort_by_key(|e| e.id());
```

### DON'T

```rust
// ❌ Variable timestep
world.step(delta_time);

// ❌ Unseeded RNG
let mut rng = rand::thread_rng();

// ❌ HashMap for gameplay-critical iteration
let entities: HashMap<EntityId, Entity> = ...;

// ❌ Rely on spawn order across archetype changes
let first = world.entities().next();  // Order may change!
```

## Performance Impact

Determinism adds minimal overhead:

| Operation | HashMap | BTreeMap | Overhead |
|-----------|---------|----------|----------|
| Lookup | O(1) | O(log n) | ~10-20 ns |
| Iteration | O(n) | O(n) | None |
| Insert | O(1) | O(log n) | ~10-20 ns |

**In practice:** For typical games (<10,000 entities), the overhead is negligible (<1% of frame budget).

## Related Documentation

- [ECS Architecture](./ecs.md) - Entity-Component-System details
- [AI-Native Design](./ai-native.md) - How determinism enables AI
- [Performance Optimization](../performance/optimization.md) - Frame budget analysis

## See Also

- `astraweave-ecs/src/determinism_tests.rs` - 763 lines of determinism validation
- `astraweave-core/src/capture_replay.rs` - State capture/replay implementation
- `astraweave-physics/src/async_scheduler.rs` - Deterministic parallel physics