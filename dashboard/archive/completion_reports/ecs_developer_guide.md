# AstraWeave ECS Developer Guide

This guide covers the ECS (Entity Component System) architecture in AstraWeave, providing patterns for plugin development, scheduling, and testing.

## Core Concepts

### ECS Architecture
AstraWeave uses a custom ECS implementation in `astraweave-ecs` with:
- **Deterministic storage**: BTreeMap-backed component storage for stable iteration order
- **Fixed stages**: perception → simulation → ai_planning → physics → presentation
- **Plugin system**: Modular system registration via `Plugin` trait
- **Event system**: Deterministic event queuing with reader/writer pattern

### Key Types
- `App`: Main application builder with plugin registration
- `World`: Component storage and system execution
- `Entity`: Unique entity identifier
- `Events<T>`: Deterministic event resource with VecDeque backing

## Plugin Development

### Basic Plugin Structure
```rust
use astraweave_ecs as ecs;

pub struct MyPlugin;

impl ecs::Plugin for MyPlugin {
    fn build(&self, app: &mut ecs::App) {
        // Register components
        app.register_component::<MyComponent>();

        // Register systems in appropriate stages
        app.add_system("simulation", my_system as ecs::SystemFn);

        // Register events
        app.register_event::<MyEvent>();
    }
}
```

### System Stages
- **perception**: Gather world state, build snapshots
- **simulation**: Core game logic, movement, cooldowns
- **ai_planning**: AI decision making and plan generation
- **physics**: Collision detection, physics simulation
- **presentation**: Rendering, audio, UI updates

### Component Registration
Components must be registered before use:
```rust
app.register_component::<CPos>();
app.register_component::<CHealth>();
```

## Query Patterns

### Filtered Queries
Use `FilteredQuery` for efficient component iteration:
```rust
use astraweave_ecs::FilteredQuery;

// Query entities with position and health
let query = FilteredQuery::<(CPos, CHealth)>::new(&world);
for (entity, (pos, health)) in query {
    // Process entities
}
```

### Query Macros
Convenience macros for common patterns:
```rust
// Iterate all entities with specific components
query!((pos: CPos, health: CHealth) in &world => {
    // pos and health are borrowed references
});

// Find single entity
let entity = query_first!((pos: CPos) where pos.pos == target in &world);
```

### World Helpers
```rust
// Check if entity exists
if world.has(entity) { ... }

// Count entities with component
let count = world.count::<CPos>();

// Remove entity and all its components
world.remove(entity);
```

## Event System

### Event Emission
```rust
// Get event writer
let mut events = world.resource_mut::<Events<MovedEvent>>().unwrap();
events.writer().push(MovedEvent {
    entity,
    from: old_pos,
    to: new_pos,
});
```

### Event Consumption
```rust
// Get event reader
let events = world.resource::<Events<MovedEvent>>().unwrap();
let mut reader = events.reader();
for event in reader.drain() {
    // Process event
}
```

## AI Integration

### AI Plugin Pattern
```rust
use astraweave_ai::ecs_ai_plugin::AiPlanningPlugin;

let app = ecs_adapter::build_app(world, dt)
    .add_plugin(AiPlanningPlugin);
```

### Perception Building
Use `core::perception::build_snapshot` for rich AI inputs:
```rust
let snapshot = astraweave_core::perception::build_snapshot(&world, agent_entity);
```

## Testing Patterns

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movement_system() {
        let mut world = World::new();
        let entity = world.spawn();
        world.insert(entity, CPos { pos: IVec2::new(0, 0) });
        world.insert(entity, CDesiredPos { pos: IVec2::new(1, 0) });

        // Run movement system
        movement_system(&mut world);

        let pos = world.get::<CPos>(entity).unwrap();
        assert_eq!(pos.pos, IVec2::new(1, 0));
    }
}
```

### Integration Tests
```rust
#[test]
fn test_ecs_vs_legacy_parity() {
    // Compare ECS and legacy world state after N ticks
    let mut legacy = World::new();
    let mut ecs_app = build_app(legacy.clone(), 0.016);

    for _ in 0..10 {
        legacy.tick(0.016);
    }
    ecs_app = ecs_app.run_fixed(10);

    // Assert state equivalence
}
```

## Performance Considerations

### Deterministic Iteration
- BTreeMap ensures stable entity/component order
- Use `run_fixed()` for consistent timestep simulation
- Avoid HashMap for component storage to prevent non-deterministic behavior

### Memory Management
- Components are stored efficiently in archetype-like structures
- Events use VecDeque for FIFO ordering
- Resource borrowing follows Rust ownership rules

## Migration from Legacy World

### Entity Bridging
```rust
// Bridge legacy entities to ECS
let bridge = EntityBridge::new();
bridge.insert_pair(legacy_id, ecs_entity);
```

### Component Migration
```rust
// Migrate legacy data to ECS components
for (id, pos) in legacy.positions() {
    let ecs_id = bridge.get(&id).unwrap();
    world.insert(ecs_id, CPos { pos });
}
```

## Best Practices

1. **Register components**: Always register components in plugin build
2. **Stage ordering**: Place systems in appropriate stages for correct execution order
3. **Event hygiene**: Drain event readers to prevent memory leaks
4. **Determinism**: Use deterministic data structures and avoid random number generation in core logic
5. **Testing**: Write both unit tests for individual systems and integration tests for full cycles
6. **Performance**: Profile system execution time and optimize hot paths

## Common Patterns

### Movement System
```rust
fn movement_system(world: &mut World) {
    let query = FilteredQuery::<(CPos, CDesiredPos)>::new(world);
    for (entity, (mut pos, desired)) in query {
        let delta = desired.pos - pos.pos;
        if delta != IVec2::ZERO {
            let dir = delta.signum();
            pos.pos += dir;
            // Emit movement event
        }
    }
}
```

### AI Planning Integration
```rust
fn ai_planning_system(world: &mut World) {
    let query = FilteredQuery::<(CPos, CTeam)>::new(world);
    for (entity, (pos, team)) in query {
        let snapshot = build_snapshot(world, entity);
        if let Some(plan) = orchestrator.plan(&snapshot) {
            // Apply plan to set desired positions, etc.
        }
    }
}
```</content>
<parameter name="filePath">c:\Users\pv2br\source\repos\AstraWeave-AI-Native-Gaming-Engine\docs\ecs_developer_guide.md