# Building Your First Game

This guide walks you through building a complete game with AstraWeave, from project setup to a playable demo featuring AI companions, physics, and navigation.

```admonish info title="Prerequisites"
Before starting, ensure you have:
- Rust 1.75+ installed
- AstraWeave cloned and building successfully
- Ollama running with a compatible model (see [Installation Guide](../getting-started/installation.md))
```

## Project Overview

We'll build **"Companion Quest"** - a small adventure where an AI companion helps the player navigate a dungeon, solve puzzles, and defeat enemies. This showcases:

- AI companion with LLM-powered dialogue
- Physics-based puzzles
- Navigation and pathfinding
- Combat with adaptive enemies
- Save/load functionality

## Step 1: Project Setup

### Create a New Example

Create a new directory in `examples/`:

```bash
mkdir -p examples/companion_quest/src
```

Create `examples/companion_quest/Cargo.toml`:

```toml
[package]
name = "companion_quest"
version = "0.1.0"
edition = "2021"

[dependencies]
astraweave-core = { path = "../../astraweave-core" }
astraweave-ecs = { path = "../../astraweave-ecs" }
astraweave-ai = { path = "../../astraweave-ai" }
astraweave-physics = { path = "../../astraweave-physics" }
astraweave-nav = { path = "../../astraweave-nav" }
astraweave-render = { path = "../../astraweave-render" }
astraweave-audio = { path = "../../astraweave-audio" }
astraweave-input = { path = "../../astraweave-input" }
astraweave-llm = { path = "../../astraweave-llm" }
astraweave-gameplay = { path = "../../astraweave-gameplay" }
tokio = { version = "1.0", features = ["rt-multi-thread", "macros"] }
```

### Main Entry Point

Create `examples/companion_quest/src/main.rs`:

```rust
use astraweave_core::prelude::*;
use astraweave_ecs::prelude::*;
use astraweave_ai::prelude::*;
use astraweave_physics::prelude::*;
use astraweave_nav::prelude::*;

fn main() {
    let mut world = World::new();
    
    setup_world(&mut world);
    setup_player(&mut world);
    setup_companion(&mut world);
    setup_dungeon(&mut world);
    
    run_game_loop(&mut world);
}

fn setup_world(world: &mut World) {
    world.insert_resource(PhysicsConfig::default());
    world.insert_resource(NavigationConfig::default());
    world.insert_resource(AiConfig {
        tick_budget_ms: 8,
        max_concurrent_plans: 4,
        ..Default::default()
    });
}
```

## Step 2: Creating the Player

The player entity needs transform, physics, and input components:

```rust
fn setup_player(world: &mut World) {
    let player = world.spawn((
        Transform::from_xyz(0.0, 1.0, 0.0),
        RigidBody::Dynamic,
        Collider::capsule(0.5, 1.8),
        Player {
            health: 100.0,
            max_health: 100.0,
            stamina: 100.0,
        },
        InputReceiver::default(),
        CharacterController {
            speed: 5.0,
            jump_force: 8.0,
            ..Default::default()
        },
    ));
    
    world.insert_resource(PlayerEntity(player));
}

#[derive(Component)]
struct Player {
    health: f32,
    max_health: f32,
    stamina: f32,
}
```

## Step 3: Creating the AI Companion

The companion uses AstraWeave's AI-native architecture:

```rust
fn setup_companion(world: &mut World) {
    let companion = world.spawn((
        Transform::from_xyz(2.0, 1.0, 0.0),
        RigidBody::Dynamic,
        Collider::capsule(0.4, 1.6),
        
        AiAgent {
            personality: "helpful and curious".into(),
            knowledge_base: vec![
                "I am Spark, a magical companion".into(),
                "I can help solve puzzles and fight enemies".into(),
            ],
        },
        
        PerceptionRadius(15.0),
        
        NavAgent {
            speed: 4.0,
            acceleration: 10.0,
            avoidance_radius: 0.5,
        },
        
        Companion {
            following: true,
            follow_distance: 3.0,
        },
        
        DialogueCapable::default(),
    ));
    
    world.insert_resource(CompanionEntity(companion));
}

#[derive(Component)]
struct Companion {
    following: bool,
    follow_distance: f32,
}
```

## Step 4: Building the Dungeon

Create a simple dungeon with rooms and corridors:

```rust
fn setup_dungeon(world: &mut World) {
    let floor = world.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        RigidBody::Static,
        Collider::cuboid(50.0, 0.5, 50.0),
        Floor,
    ));
    
    spawn_walls(world);
    spawn_doors(world);
    spawn_puzzles(world);
    spawn_enemies(world);
    
    generate_navmesh(world);
}

fn generate_navmesh(world: &mut World) {
    let navmesh = NavMeshBuilder::new()
        .cell_size(0.3)
        .cell_height(0.2)
        .agent_radius(0.5)
        .agent_height(1.8)
        .max_slope(45.0)
        .build_from_world(world);
    
    world.insert_resource(navmesh);
}
```

## Step 5: The Game Loop

AstraWeave uses a fixed-tick simulation for determinism:

```rust
fn run_game_loop(world: &mut World) {
    let mut scheduler = Scheduler::new();
    
    scheduler.add_system(input_system);
    scheduler.add_system(player_movement_system);
    scheduler.add_system(ai_perception_system);
    scheduler.add_system(ai_planning_system);
    scheduler.add_system(companion_follow_system);
    scheduler.add_system(navigation_system);
    scheduler.add_system(physics_system);
    scheduler.add_system(combat_system);
    scheduler.add_system(dialogue_system);
    scheduler.add_system(render_system);
    
    let tick_rate = Duration::from_secs_f64(1.0 / 60.0);
    let mut accumulator = Duration::ZERO;
    let mut last_time = Instant::now();
    
    loop {
        let now = Instant::now();
        let delta = now - last_time;
        last_time = now;
        accumulator += delta;
        
        while accumulator >= tick_rate {
            scheduler.run(world);
            accumulator -= tick_rate;
        }
        
        if should_quit(world) {
            break;
        }
    }
}
```

## Step 6: AI Companion Behavior

The companion uses perception and planning:

```rust
fn companion_follow_system(
    player_query: Query<&Transform, With<Player>>,
    mut companion_query: Query<(&mut NavAgent, &Companion, &Transform)>,
    navmesh: Res<NavMesh>,
) {
    let player_pos = player_query.single().translation;
    
    for (mut nav_agent, companion, transform) in companion_query.iter_mut() {
        if !companion.following {
            continue;
        }
        
        let distance = transform.translation.distance(player_pos);
        
        if distance > companion.follow_distance {
            let target = player_pos - (player_pos - transform.translation)
                .normalize() * companion.follow_distance;
            
            if let Some(path) = navmesh.find_path(transform.translation, target) {
                nav_agent.set_path(path);
            }
        }
    }
}
```

## Step 7: Dialogue Integration

Enable LLM-powered dialogue with the companion:

```rust
fn dialogue_system(
    mut dialogue_events: EventReader<DialogueEvent>,
    mut llm_client: ResMut<LlmClient>,
    companion_query: Query<&AiAgent, With<Companion>>,
    mut dialogue_responses: EventWriter<DialogueResponse>,
) {
    for event in dialogue_events.read() {
        let agent = companion_query.single();
        
        let prompt = format!(
            "You are {}. The player says: '{}'. Respond in character.",
            agent.personality,
            event.message
        );
        
        match llm_client.generate_blocking(&prompt) {
            Ok(response) => {
                dialogue_responses.send(DialogueResponse {
                    speaker: "Spark".into(),
                    text: response,
                });
            }
            Err(e) => {
                dialogue_responses.send(DialogueResponse {
                    speaker: "Spark".into(),
                    text: "Hmm, I'm not sure what to say...".into(),
                });
            }
        }
    }
}
```

## Step 8: Combat System

Add combat with tool validation:

```rust
fn combat_system(
    mut combat_events: EventReader<CombatEvent>,
    mut health_query: Query<&mut Health>,
    tool_validator: Res<ToolValidator>,
) {
    for event in combat_events.read() {
        let validation = tool_validator.validate(&ToolCall {
            tool: "attack".into(),
            params: json!({
                "attacker": event.attacker,
                "target": event.target,
                "damage": event.damage,
            }),
        });
        
        match validation {
            ToolResult::Success => {
                if let Ok(mut health) = health_query.get_mut(event.target) {
                    health.current -= event.damage;
                }
            }
            ToolResult::Blocked(reason) => {
                println!("Attack blocked: {}", reason);
            }
        }
    }
}
```

## Step 9: Running Your Game

Build and run:

```bash
cargo run -p companion_quest --release
```

### Expected Output

```
[INFO] AstraWeave v0.1.0 starting...
[INFO] Physics initialized: Rapier 0.17
[INFO] Navigation: NavMesh built (2,450 polygons)
[INFO] AI: Connected to Ollama (hermes2-pro)
[INFO] Companion "Spark" spawned at (2.0, 1.0, 0.0)
[INFO] Game loop started at 60 Hz
```

## Complete Example

See the full working example at `examples/companion_quest/` or explore these related examples:

| Example | Description |
|---------|-------------|
| `hello_companion` | Minimal AI companion demo |
| `adaptive_boss` | Multi-phase adaptive boss fight |
| `quest_dialogue_demo` | Dialogue and quest system |
| `combat_physics_demo` | Physics-based combat |
| `unified_showcase` | Full engine demonstration |

## Next Steps

- [AI Companions](./companions.md) - Deep dive into companion AI
- [Adaptive Bosses](./bosses.md) - Creating intelligent enemies
- [Dialogue Systems](./dialogue.md) - LLM-powered conversations
- [Physics](../core-systems/physics.md) - Physics system details

```admonish tip title="Performance Tip"
Enable release mode (`--release`) for LLM inference. Debug builds can be 10-50x slower for AI operations.
```
