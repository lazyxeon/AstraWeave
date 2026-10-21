# ECS Architecture

AstraWeave uses an archetype-based Entity Component System (ECS) designed for deterministic, AI-native game development. The architecture provides cache-friendly iteration, deterministic execution ordering, and efficient event propagation for AI perception.

## Core Concepts

### Entity

An **Entity** is a unique identifier representing a game object. Internally, entities use a 32-bit ID and 32-bit generation counter for safe recycling:

```rust
use astraweave_ecs::*;

let mut world = World::new();

// Spawn returns a unique Entity handle
let player = world.spawn();
let enemy = world.spawn();

// Check if entity is alive (generation-safe)
assert!(world.is_alive(player));

// Despawn removes entity and all components
world.despawn(enemy);
assert!(!world.is_alive(enemy));
```

The generation counter prevents "dangling entity" bugs—if you hold a stale entity handle after despawn, operations silently fail rather than affecting the recycled entity.

### Component

A **Component** is data attached to an entity. Any `'static + Send + Sync` type automatically implements `Component`:

```rust
// Components are just plain structs
#[derive(Clone, Copy)]
struct Position { x: f32, y: f32 }

#[derive(Clone, Copy)]
struct Velocity { x: f32, y: f32 }

struct Health(i32);

// Insert components
let mut world = World::new();
let e = world.spawn();
world.insert(e, Position { x: 0.0, y: 0.0 });
world.insert(e, Velocity { x: 1.0, y: 0.0 });
world.insert(e, Health(100));

// Query components
if let Some(pos) = world.get::<Position>(e) {
    println!("Entity at ({}, {})", pos.x, pos.y);
}

// Mutate components
if let Some(health) = world.get_mut::<Health>(e) {
    health.0 -= 10;
}
```

### Resource

A **Resource** is a singleton value accessible across systems—perfect for shared state like input, time, or game configuration:

```rust
struct DeltaTime(f32);
struct InputState {
    move_direction: (f32, f32),
    attack_pressed: bool,
}

let mut world = World::new();

// Insert resources
world.insert_resource(DeltaTime(1.0 / 60.0));
world.insert_resource(InputState {
    move_direction: (0.0, 0.0),
    attack_pressed: false,
});

// Query resources
let dt = world.get_resource::<DeltaTime>().unwrap();
let input = world.get_resource::<InputState>().unwrap();
```

## Archetype Storage

### What is an Archetype?

An **Archetype** groups all entities with the same set of component types. When you add or remove components, the entity moves to a different archetype.

```text
Archetype 0: [Position]
├── Entity(1): Position(0,0)
└── Entity(4): Position(5,3)

Archetype 1: [Position, Velocity]
├── Entity(2): Position(1,1), Velocity(1,0)
└── Entity(3): Position(2,2), Velocity(0,1)

Archetype 2: [Position, Velocity, Health]
└── Entity(5): Position(3,3), Velocity(1,1), Health(100)
```

This design provides:
- **Cache-friendly iteration**: Components of the same type are stored contiguously
- **Efficient queries**: Filter by archetype signature, not per-entity checks
- **Predictable memory layout**: Improves CPU prefetching

### Storage Modes

AstraWeave supports two storage backends:

**Box Mode (Legacy)**: Components stored as `Box<dyn Any>`. Works for any component type but has heap indirection overhead.

**BlobVec Mode (Optimized)**: Components stored in contiguous byte arrays. Requires component registration but provides 2-10× faster iteration:

```rust
let mut world = World::new();

// Register component for optimized storage
world.register_component::<Position>();
world.register_component::<Velocity>();

// Now Position and Velocity use BlobVec storage
```

### SparseSet Entity Lookup

Entity-to-archetype mapping uses a **SparseSet** for O(1) lookup:

```text
Memory Layout:
sparse: [None, Some(0), None, Some(1), None, Some(2), ...]
             ↓              ↓              ↓
dense:  [Entity(1), Entity(3), Entity(5), ...]
```

This replaced the previous `BTreeMap` approach, providing 12-57× faster entity lookups.

## System Architecture

### System Stages

AstraWeave uses fixed stages for deterministic execution—critical for AI agents that must produce identical behavior across game sessions:

```rust
use astraweave_ecs::*;

// System stages execute in order
pub struct SystemStage;

impl SystemStage {
    pub const PRE_SIMULATION: &'static str = "pre_simulation";
    pub const PERCEPTION: &'static str = "perception";      // Build AI snapshots
    pub const SIMULATION: &'static str = "simulation";      // Game logic
    pub const AI_PLANNING: &'static str = "ai_planning";    // Generate plans
    pub const PHYSICS: &'static str = "physics";            // Apply forces
    pub const POST_SIMULATION: &'static str = "post_simulation";
    pub const PRESENTATION: &'static str = "presentation";  // Render, audio
}
```

The AI-native game loop follows: **Perception → Reasoning → Planning → Action**

```text
┌───────────────────────────────────────────────────────────────────────┐
│                         Frame N                                       │
├─────────────┬─────────────┬──────────────┬─────────────┬─────────────┤
│ pre_sim     │ perception  │  simulation  │ ai_planning │  physics    │
│ (setup)     │ (sensors)   │ (game logic) │ (decide)    │ (movement)  │
├─────────────┴─────────────┴──────────────┴─────────────┴─────────────┤
│                       post_sim → presentation                         │
└───────────────────────────────────────────────────────────────────────┘
```

### Registering Systems

Systems are functions that operate on the World:

```rust
fn movement_system(world: &mut World) {
    world.each_mut::<Position>(|entity, pos| {
        if let Some(vel) = world.get::<Velocity>(entity) {
            pos.x += vel.x;
            pos.y += vel.y;
        }
    });
}

fn ai_perception_system(world: &mut World) {
    // Build WorldSnapshot for AI agents
    // (see AI documentation for details)
}

let mut app = App::new();
app.add_system("simulation", movement_system);
app.add_system("perception", ai_perception_system);
```

### Query Types

AstraWeave provides ergonomic query iterators:

```rust
// Single-component read-only query
let query = Query::<Position>::new(&world);
for (entity, pos) in query {
    println!("Entity {:?} at ({}, {})", entity, pos.x, pos.y);
}

// Two-component query
let query2 = Query2::<Position, Velocity>::new(&world);
for (entity, pos, vel) in query2 {
    println!("Entity {:?} moving at ({}, {})", entity, vel.x, vel.y);
}

// Mutable queries
let mut query = Query2Mut::<Position, Velocity>::new(&mut world);
for (entity, pos, vel) in query.iter_mut() {
    pos.x += vel.x;
    pos.y += vel.y;
}
```

## Event System

Events enable reactive AI behaviors and decoupled communication between systems.

### Sending Events

```rust
use astraweave_ecs::*;

// Define custom events
#[derive(Clone)]
struct DamageEvent {
    target: Entity,
    amount: i32,
    source: Option<Entity>,
}

impl Event for DamageEvent {}

// Send events via Events resource
let mut events = Events::new();
events.send(DamageEvent {
    target: player,
    amount: 25,
    source: Some(enemy),
});
```

### Reading Events

```rust
// Systems read events via EventReader
fn damage_system(world: &mut World) {
    let mut events = world.get_resource_mut::<Events>().unwrap();
    
    for event in events.drain::<DamageEvent>() {
        if let Some(health) = world.get_mut::<Health>(event.target) {
            health.0 -= event.amount;
        }
    }
}
```

### Built-in Events

AstraWeave provides common events for AI perception:

| Event | Purpose |
|-------|---------|
| `EntitySpawnedEvent` | Entity creation notification |
| `EntityDespawnedEvent` | Entity removal notification |
| `HealthChangedEvent` | Health changes (for AI threat assessment) |
| `AiPlanningFailedEvent` | AI plan generation failures |
| `ToolValidationFailedEvent` | AI action validation failures |

## App Builder Pattern

The `App` struct provides a Bevy-like builder pattern:

```rust
use astraweave_ecs::*;

#[derive(Clone, Copy)]
struct Position { x: f32, y: f32 }

#[derive(Clone, Copy)]
struct Velocity { x: f32, y: f32 }

fn movement_system(world: &mut World) {
    world.each_mut::<Position>(|entity, pos| {
        if let Some(vel) = world.get::<Velocity>(entity) {
            pos.x += vel.x;
            pos.y += vel.y;
        }
    });
}

fn main() {
    let mut app = App::new();
    app.add_system("simulation", movement_system);
    
    // Spawn entities
    let e = app.world.spawn();
    app.world.insert(e, Position { x: 0.0, y: 0.0 });
    app.world.insert(e, Velocity { x: 1.0, y: 0.0 });
    
    // Run 100 simulation ticks
    app = app.run_fixed(100);
    
    // Entity moved 100 units
    let pos = app.world.get::<Position>(e).unwrap();
    assert_eq!(pos.x, 100.0);
}
```

### Plugin Architecture

Plugins encapsulate related systems and resources:

```rust
pub trait Plugin {
    fn build(&self, app: &mut App);
}

struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_system("physics", physics_tick);
        app.world.insert_resource(PhysicsConfig::default());
    }
}

// Add plugins to app
let app = App::new().add_plugin(PhysicsPlugin);
```

## Command Buffer

For deferred operations (avoiding borrow conflicts), use `CommandBuffer`:

```rust
use astraweave_ecs::*;

// Register components first
let mut world = World::new();
world.register_component::<Position>();
world.register_component::<Velocity>();

// Queue commands
let mut cmd = CommandBuffer::new();
cmd.spawn()
    .insert(Position { x: 0.0, y: 0.0 })
    .insert(Velocity { x: 1.0, y: 0.0 });

cmd.entity(existing_entity)
    .remove::<Velocity>();

// Apply all commands at once
cmd.apply(&mut world);
```

## Determinism

AstraWeave ECS is designed for deterministic replay and multiplayer synchronization:

### Ordered Iteration

Entities within an archetype iterate in spawn order (using packed arrays), ensuring consistent system behavior.

### Seeded RNG

Use the built-in deterministic RNG:

```rust
use astraweave_ecs::Rng;

let mut rng = Rng::from_seed(42);
let damage = rng.next_f32() * 10.0;  // Same value every time with seed 42
```

### Event Ordering

Events are stored in order-of-send, with frame tracking for cleanup:

```rust
let events = Events::new()
    .with_keep_frames(2);  // Keep events for 2 frames
```

## Performance Characteristics

Benchmarked on the AstraWeave test suite (Week 10):

| Operation | Time | Notes |
|-----------|------|-------|
| World creation | 25.8 ns | Empty world |
| Entity spawn | 420 ns | Includes archetype assignment |
| Component insert | 1-2 µs | Archetype migration if needed |
| Entity lookup | O(1) | SparseSet, 12-57× faster than BTreeMap |
| Iteration (per entity) | <1 ns | Packed array iteration |
| Query creation | 50-100 ns | Archetype filtering |

### 60 FPS Budget

With 16.67 ms per frame, current performance provides:

- **1,000 entities**: 1.14 ms frame time (93% headroom)
- **10,000 entities**: ~11 ms frame time (34% headroom)
- **Movement system**: 106 µs for 1,000 entities (9.4× faster post-optimization)

## Advanced Topics

### Custom Component Storage

For specialized use cases, you can interact with archetype storage directly:

```rust
// Access archetypes for low-level iteration
for archetype in world.archetypes().iter() {
    println!("Archetype {} has {} entities", 
             archetype.id, archetype.len());
    
    for &entity in archetype.entities_vec() {
        // Direct entity access
    }
}
```

### Profiling Integration

Enable the `profiling` feature for Tracy integration:

```toml
[dependencies]
astraweave-ecs = { version = "0.4", features = ["profiling"] }
```

Key spans are automatically instrumented: `ECS::World::spawn`, `ECS::World::get`, `ECS::Schedule::run`.

## See Also

- [API Reference: ECS](../api/ecs.md) — Full API documentation
- [Core Systems: AI](../core-systems/ai/index.md) — AI integration with ECS
- [Patterns](../resources/patterns.md) — Common ECS design patterns
- [First Companion Tutorial](../getting-started/first-companion.md) — Hands-on ECS usage

