# Hello Companion Walkthrough

The `hello_companion` example is the perfect introduction to AstraWeave's AI-native architecture. This walkthrough explains every step of what happens when you run this example and why it's designed this way.

## Running the Example

```bash
cargo run -p hello_companion --release
```

## Expected Output

```
[INFO] Initializing AstraWeave Engine...
[INFO] Creating world with ECS...
[INFO] Spawning AI companion entity
[INFO] Starting simulation loop at 60Hz
[INFO] Tick 1: Capturing perception snapshot
[INFO] AI perception: 1 entities visible, 0 audio events
[INFO] Sending perception to AI planning layer
[INFO] AI generated plan: MoveTo { target: Vec3(10.0, 0.0, 5.0), urgency: 0.7 }
[INFO] Validating movement tool usage...
[ERROR] Tool validation failed: LosBlocked - No clear line of sight to target
thread 'main' panicked at examples/hello_companion/src/main.rs:42:5:
called `Result::unwrap()` on an `Err` value: ToolValidationError(LosBlocked)
```

**This panic is intentional!** It demonstrates AstraWeave's core principle: AI agents cannot perform actions that violate the game world's constraints.

## Code Walkthrough

Let's examine the source code to understand each step:

### 1. Engine Initialization

```rust
// examples/hello_companion/src/main.rs
use astraweave_core::*;
use astraweave_ai::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    // Create the ECS world
    let mut world = World::new();
    
    // Configure the engine systems
    world.add_plugin(CorePlugin)
         .add_plugin(AIPlugin)
         .add_plugin(PhysicsPlugin);
```

**What's happening:**
- Sets up the Entity-Component-System (ECS) world
- Registers core systems for AI, physics, and simulation
- Configures 60Hz fixed-tick simulation

### 2. Spawning the AI Companion

```rust
    // Spawn an AI companion entity
    let companion = world.spawn()
        .insert(Position(Vec3::new(0.0, 0.0, 0.0)))
        .insert(AIAgent {
            perception_range: 10.0,
            planning_interval: Duration::from_millis(500),
            ai_model: AIModel::Local("companion-7b".to_string()),
        })
        .insert(MovementCapability {
            max_speed: 5.0,
            acceleration: 2.0,
        })
        .id();
    
    info!("Spawned AI companion with ID: {:?}", companion);
```

**What's happening:**
- Creates a new entity in the ECS world
- Adds position component (where the companion is)
- Adds AI agent component (makes it intelligent)
- Adds movement capability (what it can do)

### 3. The Simulation Loop

```rust
    // Run simulation for a few ticks
    for tick in 0..5 {
        info!("Starting tick {}", tick);
        
        // This is where the AI magic happens
        world.step(Duration::from_nanos(16_666_667)); // 1/60 second
        
        // Small delay so we can see the output
        std::thread::sleep(Duration::from_millis(100));
    }
```

**What's happening:**
- Runs exactly 5 simulation ticks
- Each tick advances the world by exactly 1/60th of a second
- The deterministic timing ensures consistent behavior

### 4. The AI Pipeline (Inside world.step())

During each `world.step()` call, several systems run in sequence:

#### A. Perception System

```rust
fn ai_perception_system(
    mut query: Query<(&Position, &AIAgent, &mut PerceptionState)>,
    world_query: Query<&Position>,
) {
    for (pos, agent, mut perception) in query.iter_mut() {
        // Gather what the AI can see
        let mut visible_entities = Vec::new();
        
        for other_pos in world_query.iter() {
            let distance = pos.0.distance(other_pos.0);
            if distance <= agent.perception_range {
                visible_entities.push(EntityData {
                    position: other_pos.0,
                    distance,
                    entity_type: "unknown".to_string(),
                });
            }
        }
        
        // Create perception snapshot
        perception.last_snapshot = Some(PerceptionSnapshot {
            timestamp: world.current_tick(),
            agent_id: entity,
            visible_entities,
            audio_events: vec![], // None in this simple example
            world_state: WorldState::default(),
        });
    }
}
```

#### B. AI Planning System

```rust
fn ai_planning_system(
    mut query: Query<(&AIAgent, &PerceptionState, &mut PlanningState)>,
    ai_service: Res<AIService>,
) {
    for (agent, perception, mut planning) in query.iter_mut() {
        if let Some(snapshot) = &perception.last_snapshot {
            // Send to AI model for planning
            let plan_request = PlanningRequest {
                perception: snapshot.clone(),
                agent_profile: agent.clone(),
                available_tools: vec!["MovementTool", "InteractionTool"],
            };
            
            // This is where the LLM generates a plan
            let plan = ai_service.generate_plan(plan_request)?;
            
            info!("AI generated plan: {:?}", plan.intent);
            planning.current_plan = Some(plan);
        }
    }
}
```

#### C. Tool Validation System

```rust
fn tool_validation_system(
    mut query: Query<(&PlanningState, &mut ActionState)>,
    tool_registry: Res<ToolRegistry>,
    world: &World,
) {
    for (planning, mut action) in query.iter_mut() {
        if let Some(plan) = &planning.current_plan {
            for tool_usage in &plan.tools {
                // This is where the validation happens
                let validation_result = tool_registry
                    .get_tool(&tool_usage.tool_name)
                    .unwrap()
                    .validate(world, tool_usage);
                
                match validation_result {
                    ValidationResult::Valid => {
                        info!("Tool validation passed: {}", tool_usage.tool_name);
                        action.pending_actions.push(tool_usage.clone());
                    }
                    ValidationResult::Blocked(reason) => {
                        error!("Tool validation failed: {:?}", reason);
                        // This causes the panic in hello_companion
                        return Err(ToolValidationError::from(reason));
                    }
                }
            }
        }
    }
}
```

## Why Does It Panic?

The panic occurs because the AI tries to move to a position but there's no clear line of sight. Here's what happens:

### 1. AI Perception
- The companion perceives its current position (0, 0, 0)
- It detects no obstacles in its perception range

### 2. AI Planning
- The AI decides it wants to move to position (10, 0, 5)
- This seems reasonable based on its limited perception

### 3. Tool Validation
- The MovementTool.validate() method checks line of sight
- There's an invisible obstacle blocking the path
- Validation fails with `LosBlocked` error

### 4. Engine Authority
- The engine refuses to execute the invalid action
- This maintains world integrity and prevents AI cheating

## Key Learning Points

### 1. AI Cannot Cheat
The AI doesn't have perfect information about the world. It can only act based on what it perceives, and all actions must be validated by the engine.

### 2. Deterministic Behavior
Run the example multiple times - you'll get the same result every time. This determinism is crucial for:
- Reliable testing
- Networking (same simulation on all clients)
- Debugging AI behavior

### 3. Tool-Based Architecture
The AI doesn't directly move entities or change the world. It can only request actions through validated tools:
- MovementTool (for movement)
- InteractionTool (for object interaction)
- CombatTool (for attacks)
- CommunicationTool (for dialogue)

### 4. Perception vs Reality
The AI's perception is limited and may not match reality. This creates interesting emergent behavior as AI agents must:
- Explore to gather information
- Make decisions with incomplete data
- Adapt when actions fail

## Modifying the Example

### Make It Not Panic

To see successful AI behavior, modify the world setup:

```rust
// Remove the obstacle that blocks line of sight
world.remove_obstacle(Vec3::new(5.0, 0.0, 2.5));

// Or give the AI perfect perception
ai_agent.perception_range = f32::INFINITY;
```

### Add More Interesting Behavior

```rust
// Add a target for the AI to find
world.spawn()
    .insert(Position(Vec3::new(15.0, 0.0, 0.0)))
    .insert(InteractableItem {
        item_type: "treasure_chest".to_string(),
        value: 100,
    });

// The AI will now try to navigate to and interact with the chest
```

### Enable Logging for More Detail

```bash
RUST_LOG=debug cargo run -p hello_companion --release
```

This shows detailed information about:
- ECS system execution order
- AI model input/output
- Tool validation steps
- World state changes

## Architectural Insights

### Fixed-Tick Simulation
The 60Hz fixed timestep ensures:
- Physics determinism
- Consistent AI decision making
- Reliable networking
- Predictable performance

### ECS Benefits
The Entity-Component-System architecture provides:
- Cache-friendly performance
- Clear separation of concerns
- Easy parallel system execution
- Modular, testable code

### AI Validation Pipeline
The perception → planning → validation → execution pipeline ensures:
- No AI cheating
- Consistent game rules
- Emergent behavior from constraints
- Easy debugging and testing

## Next Steps

After understanding hello_companion:

1. **Explore More Examples**: Try [Adaptive Boss](./adaptive-boss.md) for complex AI
2. **Learn Architecture**: Read [AI-Native Design](../architecture/ai-native.md)
3. **Build Your Own**: Follow [Building Your First Game](../game-dev/first-game.md)
4. **Dive Deeper**: Study [Core AI Systems](../core-systems/ai/index.md)

## Common Questions

### Q: Why does it panic instead of just logging the error?

**A**: The panic demonstrates that validation failures are serious. In a real game, you'd handle this gracefully, but the example uses panic to make the validation concept crystal clear.

### Q: Can I make the AI smarter to avoid this error?

**A**: Yes! You can:
- Improve the AI's perception system
- Give it better pathfinding tools
- Add obstacle detection to its planning
- Implement learning from failed actions

### Q: Is this really how a game AI should work?

**A**: For AI-native games, yes! This approach:
- Prevents AI cheating
- Creates emergent behavior
- Works in multiplayer
- Enables complex AI interactions

The "failed action = learning opportunity" approach leads to much more interesting AI behavior than scripted sequences.

---

*The hello_companion example may be simple, but it demonstrates the fundamental principles that enable AstraWeave's AI-native gameplay. Every complex AI behavior in the engine builds on these same validation patterns.*