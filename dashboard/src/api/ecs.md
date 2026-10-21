# ECS API Reference

> **Crate**: `astraweave-ecs`  
> **Coverage**: 94.26%  
> **Tests**: 1,200+

The Entity Component System is the foundation of AstraWeave, providing deterministic, cache-friendly entity management with archetype-based storage.

## Quick Links

- [rustdoc](https://docs.rs/astraweave-ecs) (when published)
- [Source](https://github.com/astraweave/astraweave/tree/main/astraweave-ecs)
- [ECS Core Concepts](../core-systems/ecs.md)

---

## Core Types

### World

The central container for all entities, components, and resources.

```rust
use astraweave_ecs::World;

let mut world = World::new();

// Spawn entities
let entity = world.spawn((Position::default(), Velocity::default()));

// Access resources
world.insert_resource(GameTime::default());
let time = world.resource::<GameTime>();
```

**Key Methods**:
- `new()` → `World` - Create empty world
- `spawn(bundle)` → `Entity` - Create entity with components
- `despawn(entity)` - Remove entity
- `insert_resource<R>(resource)` - Add singleton resource
- `resource<R>()` → `&R` - Get resource reference
- `query<Q>()` → `Query<Q>` - Create component query

---

### Entity

Lightweight 64-bit identifier (32-bit index + 32-bit generation).

```rust
use astraweave_ecs::Entity;

// Entities are created via World::spawn()
let entity = world.spawn(MyBundle::default());

// Check if entity exists
if world.contains(entity) {
    // Entity is alive
}

// Create from raw (unsafe, for FFI)
let raw = Entity::from_raw(42, 1);
```

**Properties**:
- 8 bytes memory footprint
- Copy, Clone, Hash, Eq
- Safe to store in collections
- Generation prevents use-after-free

---

### App

Application builder with plugin support.

```rust
use astraweave_ecs::App;

App::new()
    .add_plugin(DefaultPlugins)
    .add_startup_system(setup)
    .add_system(update)
    .run();
```

**Key Methods**:
- `add_plugin<P>(plugin)` - Add functionality bundle
- `add_startup_system(system)` - Run once at start
- `add_system(system)` - Add to main loop
- `add_system_to_stage(stage, system)` - Add to specific stage
- `run()` - Start main loop

---

### Schedule

System execution scheduler with parallel support.

```rust
use astraweave_ecs::{Schedule, SystemStage};

let mut schedule = Schedule::new();
schedule.add_system_to_stage(SystemStage::Update, my_system);
schedule.run(&mut world);
```

**System Stages** (in execution order):
1. `First` - Pre-frame setup
2. `PreUpdate` - Input processing
3. `Update` - Main game logic
4. `PostUpdate` - Physics, AI
5. `Last` - Rendering prep

---

### Component

Trait for data attached to entities (auto-implemented for `'static + Send + Sync`).

```rust
use astraweave_ecs::Component;

#[derive(Component, Default)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Component)]
struct Velocity(Vec3);

#[derive(Component)]
struct Name(String);
```

**Requirements**:
- `'static` lifetime
- `Send + Sync` (for parallel systems)
- Prefer small, focused components

---

### Resource

Singleton data shared across systems.

```rust
use astraweave_ecs::Resource;

#[derive(Resource, Default)]
struct GameTime {
    elapsed: f32,
    delta: f32,
}

#[derive(Resource)]
struct Settings {
    volume: f32,
    difficulty: Difficulty,
}

// Access in systems
fn update_time(mut time: ResMut<GameTime>) {
    time.elapsed += time.delta;
}
```

---

### Query

Efficient iteration over component combinations.

```rust
use astraweave_ecs::Query;

// Read-only query
fn read_system(query: Query<&Position>) {
    for pos in query.iter() {
        println!("Position: {:?}", pos);
    }
}

// Mutable query
fn write_system(mut query: Query<&mut Position>) {
    for mut pos in query.iter_mut() {
        pos.x += 1.0;
    }
}

// Multiple components
fn complex_query(query: Query<(&Position, &Velocity, Option<&Name>)>) {
    for (pos, vel, name) in query.iter() {
        // name is Option<&Name>
    }
}

// Filters
fn filtered_query(query: Query<&Position, With<Player>>) {
    // Only entities with Player component
}
```

**Filter Types**:
- `With<T>` - Entity must have T
- `Without<T>` - Entity must not have T
- `Or<(A, B)>` - Entity has A or B
- `Added<T>` - T was just added
- `Changed<T>` - T was modified

---

### CommandBuffer

Deferred entity/component modifications (thread-safe).

```rust
use astraweave_ecs::CommandBuffer;

fn spawn_system(mut commands: Commands) {
    // Spawn entity with components
    commands.spawn((Position::default(), Velocity::default()));
    
    // Modify existing entity
    commands.entity(some_entity)
        .insert(Health(100))
        .remove::<Poisoned>();
    
    // Despawn
    commands.entity(dead_entity).despawn();
}
```

**Key Methods**:
- `spawn(bundle)` → `EntityCommands` - Queue entity spawn
- `entity(entity)` → `EntityCommands` - Get entity commands
- `insert_resource<R>(resource)` - Queue resource insert
- `remove_resource<R>()` - Queue resource removal

---

### Events

Type-safe event channel for system communication.

```rust
use astraweave_ecs::{Events, EventReader, EventWriter};

struct DamageEvent {
    target: Entity,
    amount: f32,
}

fn damage_sender(mut events: EventWriter<DamageEvent>) {
    events.send(DamageEvent { target: enemy, amount: 50.0 });
}

fn damage_receiver(mut events: EventReader<DamageEvent>) {
    for event in events.iter() {
        println!("Entity {:?} took {} damage", event.target, event.amount);
    }
}
```

**Event Lifecycle**:
1. Events sent via `EventWriter`
2. Events read via `EventReader`
3. Events cleared at end of frame
4. Double-buffered for reliable delivery

---

## Archetype Storage

AstraWeave uses archetype-based storage for optimal cache performance:

```
Archetype A: [Position, Velocity]
┌──────────┬──────────┐
│ Position │ Velocity │
├──────────┼──────────┤
│ (1,2,3)  │ (0,1,0)  │  Entity 1
│ (4,5,6)  │ (1,0,0)  │  Entity 2
│ (7,8,9)  │ (0,0,1)  │  Entity 3
└──────────┴──────────┘

Archetype B: [Position, Velocity, Health]
┌──────────┬──────────┬────────┐
│ Position │ Velocity │ Health │
├──────────┼──────────┼────────┤
│ (0,0,0)  │ (0,0,0)  │  100   │  Entity 4
└──────────┴──────────┴────────┘
```

**Benefits**:
- Cache-friendly iteration
- No pointer chasing
- Parallel-safe by design

---

## Performance

| Operation | Latency | Notes |
|-----------|---------|-------|
| World creation | 25.8 ns | Empty world |
| Entity spawn | 420 ns | With components |
| Component access | <1 ns | Direct archetype access |
| Query iteration | ~2 ns/entity | Cache-optimal |

---

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `parallel` | Parallel system execution | ✅ |
| `tracing` | Tracy profiling integration | ❌ |
| `serde` | Serialization support | ❌ |

```toml
[dependencies]
astraweave-ecs = { version = "0.4", features = ["serde"] }
```

---

## See Also

- [ECS Architecture](../core-systems/ecs.md)
- [System Ordering](../core-systems/ecs.md#system-ordering)
- [Parallel Systems](../performance/optimization.md#ecs-optimization)
- [Core API](./core.md)
