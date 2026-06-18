# AstraWeave ECS - Production-Grade, AI-Native Entity Component System

## Overview

AstraWeave ECS is a **production-ready, AI-native** Entity Component System designed for game engines. Unlike traditional ECS implementations where AI is an afterthought, AstraWeave ECS embeds the **Perception → Reasoning → Planning → Action** loop directly into its architecture.

## Key Features

### 🚀 **Performance**
- **Archetype-based storage**: Cache-friendly component iteration (similar to Bevy, Flecs, EnTT)
- **Deterministic execution**: Fixed schedules with ordered iteration (critical for multiplayer/replays)
- **Zero-cost abstractions**: System parameters compile to direct World access
- **BTreeMap-based ordering**: Predictable entity iteration for reproducible AI

### 🤖 **AI-Native Design**
- **Event-driven perception**: AI agents react to game events (damage, state changes, spawns)
- **Structured system stages**: Explicit Perception → Simulation → AI Planning → Physics → Presentation pipeline
- **WorldSnapshot integration**: Direct support for AI orchestrator data contracts
- **Plan validation**: Built-in support for PlanIntent validation and execution

### 🛠️ **Developer Experience**
- **Ergonomic queries**: `Query<T>`, `QueryMut<T>`, `QueryTuple<A, B>` with iterator support
- **Resource management**: Type-safe global state via `Res<T>` and `ResMut<T>`
- **Event system**: Deterministic event ordering with frame-based cleanup
- **Plugin architecture**: Modular game systems with clear boundaries

### 🔒 **Reliability**
- **Type-safe**: Compile-time checks for component/resource access
- **Send + Sync**: Parallel-ready (future: multi-threaded system execution)
- **Panic-safe**: Graceful handling of missing components/resources
- **Test coverage**: Comprehensive unit tests for core functionality

## Architecture

### System Stages

```rust
pub struct SystemStage;

impl SystemStage {
    pub const PRE_SIMULATION: &'static str = "pre_simulation";
    pub const PERCEPTION: &'static str = "perception";        // Build WorldSnapshots
    pub const SIMULATION: &'static str = "simulation";        // Game logic
    pub const AI_PLANNING: &'static str = "ai_planning";      // Generate PlanIntents
    pub const PHYSICS: &'static str = "physics";              // Physics simulation
    pub const POST_SIMULATION: &'static str = "post_simulation";
    pub const PRESENTATION: &'static str = "presentation";    // Rendering, audio, UI
}
```

### Data Flow

```
┌─────────────┐
│ PERCEPTION  │ ← Build AI WorldSnapshots, update sensors
└──────┬──────┘
       │
┌──────▼──────┐
│ SIMULATION  │ ← Game logic, movement, cooldowns
└──────┬──────┘
       │
┌──────▼──────┐
│ AI PLANNING │ ← AI generates PlanIntents
└──────┬──────┘
       │
┌──────▼──────┐
│  PHYSICS    │ ← Physics simulation, collision resolution
└──────┬──────┘
       │
┌──────▼──────────┐
│ POST_SIMULATION │ ← Cleanup, stats, logging
└──────┬──────────┘
       │
┌──────▼────────┐
│ PRESENTATION  │ ← Rendering, audio, UI updates
└───────────────┘
```

## Core API

### World - The ECS Container

```rust
use astraweave_ecs::*;

let mut world = World::new();

// Spawn entities
let entity = world.spawn();

// Add components
world.insert(entity, Position { x: 0.0, y: 0.0 });
world.insert(entity, Velocity { x: 1.0, y: 0.0 });

// Query components
if let Some(pos) = world.get::<Position>(entity) {
    println!("Position: ({}, {})", pos.x, pos.y);
}

// Mutate components
if let Some(vel) = world.get_mut::<Velocity>(entity) {
    vel.x *= 0.9; // Apply friction
}

// Resources (singletons)
world.insert_resource(GameTime { elapsed: 0.0 });
let time = world.get_resource::<GameTime>().unwrap();
```

### Queries - Efficient Component Iteration

```rust
// Single component query
fn movement_system(world: &mut World) {
    let entities: Vec<Entity> = world.entities_with::<Position>();
    for entity in entities {
        if let (Some(pos), Some(vel)) = (
            world.get_mut::<Position>(entity),
            world.get::<Velocity>(entity),
        ) {
            pos.x += vel.x;
            pos.y += vel.y;
        }
    }
}

// Query parameters (ergonomic)
use astraweave_ecs::system_param::{Query, QueryMut};

fn system_with_query(world: &World) {
    let query = Query::<Health>::new(world);
    for (entity, health) in query.iter() {
        println!("Entity {:?} has {} HP", entity, health.current);
    }
}

fn system_with_mut_query(world: &mut World) {
    let mut query = QueryMut::<Health>::new(world);
    for (entity, health) in query.iter_mut() {
        health.current += 1; // Regenerate HP
    }
}
```

### Events - AI Perception & Reactive Behaviors

```rust
use astraweave_ecs::{Events, Event};

#[derive(Clone, Debug)]
struct DamageEvent {
    attacker: Entity,
    target: Entity,
    damage: i32,
}
impl Event for DamageEvent {}

// Sending events
fn combat_system(world: &mut World) {
    let events = world.get_resource_mut::<Events>().unwrap();
    events.send(DamageEvent {
        attacker: Entity(0),
        target: Entity(1),
        damage: 10,
    });
}

// Reading events
fn ai_perception_system(world: &mut World) {
    let events = world.get_resource::<Events>().unwrap();
    for event in events.read::<DamageEvent>() {
        // AI reacts to damage
        println!("Entity {:?} took {} damage!", event.target, event.damage);
    }
}

// Drain events (consume)
fn event_processor(world: &mut World) {
    let events = world.get_resource_mut::<Events>().unwrap();
    for event in events.drain::<DamageEvent>() {
        // Process and remove event
    }
}
```

### App - The Game Loop

```rust
use astraweave_ecs::*;

fn main() {
    let mut app = App::new();

    // Add resources
    app.world.insert_resource(GameTime { tick: 0 });
    app.world.insert_resource(Events::new());

    // Register systems
    app.add_system(SystemStage::PERCEPTION, ai_perception_system);
    app.add_system(SystemStage::SIMULATION, movement_system);
    app.add_system(SystemStage::AI_PLANNING, ai_planning_system);

    // Run fixed timestep
    for _ in 0..1000 {
        app.schedule.run(&mut app.world);
        
        // Update game time
        if let Some(time) = app.world.get_resource_mut::<GameTime>() {
            time.tick += 1;
        }
    }
}
```

## AI-Native Patterns

### Pattern 1: AI Perception System

```rust
fn ai_perception_system(world: &mut World) {
    let ai_entities: Vec<Entity> = world.entities_with::<AIAgent>();

    for agent in ai_entities {
        let my_pos = world.get::<Position>(agent).unwrap().clone();
        
        // Find nearby enemies
        let enemies: Vec<Entity> = world.entities_with::<Enemy>();
        let mut visible_enemies = Vec::new();

        for enemy in enemies {
            if let Some(enemy_pos) = world.get::<Position>(enemy) {
                let distance = (enemy_pos.pos - my_pos.pos).length();
                if distance < PERCEPTION_RADIUS {
                    visible_enemies.push(enemy);
                }
            }
        }

        // Update AI perception
        if let Some(ai) = world.get_mut::<AIAgent>(agent) {
            ai.visible_enemies = visible_enemies;
        }
    }
}
```

### Pattern 2: AI Planning with Events

```rust
fn ai_planning_system(world: &mut World) {
    let ai_entities: Vec<Entity> = world.entities_with::<AIAgent>();

    for agent in ai_entities {
        let ai = world.get::<AIAgent>(agent).unwrap();
        
        // Decision making based on perception
        let plan = if ai.visible_enemies.is_empty() {
            PlanIntent::patrol()
        } else if ai.health < 30 {
            PlanIntent::flee()
        } else {
            PlanIntent::attack(ai.visible_enemies[0])
        };

        // Store plan
        if let Some(ai) = world.get_mut::<AIAgent>(agent) {
            ai.current_plan = Some(plan);
        }

        // Emit planning event
        if let Some(events) = world.get_resource_mut::<Events>() {
            events.send(AIPlanGeneratedEvent { agent, plan });
        }
    }
}
```

### Pattern 3: Event-Driven State Machines

```rust
fn ai_state_machine(world: &mut World) {
    // Read damage events
    let damage_events: Vec<DamageEvent> = {
        let events = world.get_resource::<Events>().unwrap();
        events.read::<DamageEvent>().cloned().collect()
    };

    // Update AI states based on events
    for event in damage_events {
        if let Some(ai) = world.get_mut::<AIAgent>(event.target) {
            match ai.state {
                AIState::Idle | AIState::Patrol => {
                    ai.state = AIState::Combat;
                    ai.target = Some(event.attacker);
                }
                AIState::Combat if ai.health < 30 => {
                    ai.state = AIState::Flee;
                }
                _ => {}
            }
        }
    }
}
```

## Comparison with Other Engines

| Feature | AstraWeave ECS | Bevy | Unity ECS | Unreal |
|---------|---------------|------|-----------|--------|
| **Archetype Storage** | ✅ | ✅ | ✅ | ❌ |
| **Deterministic** | ✅ | ⚠️ (HashMap) | ❌ | ❌ |
| **AI-Native Events** | ✅ | ⚠️ (Generic) | ❌ | ⚠️ |
| **System Stages** | ✅ (AI-focused) | ✅ | ✅ | ✅ |
| **Query Ergonomics** | ✅ | ✅ | ⚠️ | ⚠️ |
| **Zero-Cost Abstraction** | ✅ | ✅ | ⚠️ | ⚠️ |

## Performance Benchmarks

*(Coming soon: Comprehensive benchmarks vs Bevy, specs, hecs)*

Expected performance targets:
- **Entity spawn**: < 10ns per entity
- **Component insertion**: < 50ns per component
- **Query iteration**: ~1M entities/ms (cache-friendly)
- **Event dispatch**: < 100ns per event

## Roadmap

### Phase 1: Foundation ✅
- [x] Basic World/Entity/Component
- [x] Archetype-based storage
- [x] Event system
- [x] System parameters (Query, Res, ResMut)
- [x] AI-native system stages

### Phase 2: Performance (Q1 2026)
- [ ] Parallel system execution
- [ ] Change detection
- [ ] Archetype graph optimization
- [ ] Query caching

### Phase 3: Advanced Features (Q2 2026)
- [ ] Hierarchical entities (parent/child)
- [ ] Entity relationships (graph queries)
- [ ] Command buffers (deferred mutations)
- [ ] System dependencies

### Phase 4: Tooling (Q3 2026)
- [ ] ECS debugger/inspector
- [ ] Performance profiler
- [ ] Visual system graph
- [ ] Hot-reloading

## Examples

### Basic Example
```bash
cargo run -p ecs_ai_showcase
```

### Advanced Examples
- `ecs_ai_demo`: Simple AI planning with movement
- `hello_companion`: AI companion with personality
- `core_loop_bt_demo`: Behavior tree integration
- `core_loop_goap_demo`: GOAP (Goal-Oriented Action Planning)

## Contributing

We welcome contributions! See [CONTRIBUTING.md](../docs/supplemental-docs/CONTRIBUTING.md) for guidelines.

## License

See [LICENSE](../../LICENSE) for details.

## Credits

Inspired by:
- **Bevy**: Rust ECS with excellent ergonomics
- **Flecs**: C ECS with entity relationships
- **EnTT**: Fast C++ ECS library
- **Unity DOTS**: Data-Oriented Technology Stack

Built for **AstraWeave**, the AI-native game engine.
