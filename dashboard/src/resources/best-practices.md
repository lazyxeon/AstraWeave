# Best Practices

This guide covers recommended practices for building production-quality games with AstraWeave, covering architecture, performance, AI integration, and maintainability.

## Project Architecture

### Modular Design

Organize game code into focused modules:

```
my_game/
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Game library root
│   ├── player/              # Player systems
│   │   ├── mod.rs
│   │   ├── controller.rs
│   │   └── inventory.rs
│   ├── enemy/               # Enemy systems
│   │   ├── mod.rs
│   │   ├── ai.rs
│   │   └── spawner.rs
│   ├── world/               # World management
│   │   ├── mod.rs
│   │   ├── terrain.rs
│   │   └── weather.rs
│   └── ui/                  # User interface
│       ├── mod.rs
│       ├── hud.rs
│       └── menus.rs
├── assets/
├── config/
└── Cargo.toml
```

### Plugin Pattern

Encapsulate features as plugins:

```rust
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<PlayerConfig>()
            .add_systems(Startup, spawn_player)
            .add_systems(Update, (
                player_movement,
                player_combat,
                player_inventory,
            ).chain());
    }
}

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (
                enemy_ai_tick,
                enemy_spawning,
                enemy_cleanup,
            ));
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PlayerPlugin)
        .add_plugins(EnemyPlugin)
        .run();
}
```

### Dependency Injection

Use resources for configuration and shared state:

```rust
#[derive(Resource)]
pub struct GameConfig {
    pub difficulty: Difficulty,
    pub render_distance: f32,
    pub ai_budget_ms: f32,
}

#[derive(Resource)]
pub struct GameState {
    pub current_level: String,
    pub player_score: u64,
    pub elapsed_time: f32,
}

fn game_system(
    config: Res<GameConfig>,
    mut state: ResMut<GameState>,
) {
    // Access shared configuration and state
}
```

## ECS Best Practices

### Component Design

**Do:**
```rust
// Small, focused components
#[derive(Component)]
struct Health(f32);

#[derive(Component)]
struct MaxHealth(f32);

#[derive(Component)]
struct Damage(f32);

// Use marker components
#[derive(Component)]
struct Player;

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct Hostile;
```

**Don't:**
```rust
// Avoid monolithic components
#[derive(Component)]
struct Entity {
    health: f32,
    max_health: f32,
    damage: f32,
    position: Vec3,
    rotation: Quat,
    velocity: Vec3,
    ai_state: AiState,
    inventory: Vec<Item>,
    // ... more fields
}
```

### System Organization

Group related systems with clear ordering:

```rust
app.add_systems(Update, (
    // Input phase
    read_player_input,
    
    // AI phase (can run in parallel)
    (enemy_perception, companion_perception).run_if(should_run_ai),
    enemy_decision,
    companion_decision,
    
    // Movement phase
    apply_movement,
    resolve_collisions,
    
    // Combat phase
    process_attacks,
    apply_damage,
    check_deaths,
    
    // Cleanup phase
    despawn_dead_entities,
).chain());
```

### Query Best Practices

```rust
// Good: Specific queries with filters
fn enemy_update(
    enemies: Query<(&mut Transform, &mut AiAgent), (With<Enemy>, Without<Dead>)>,
) {
    for (mut transform, mut ai) in enemies.iter_mut() {
        // Only active enemies
    }
}

// Good: Read-only where possible
fn render_system(
    renderables: Query<(&Transform, &Mesh, &Material)>,
) {
    for (transform, mesh, material) in renderables.iter() {
        // Read-only access allows parallelism
    }
}

// Good: Use change detection
fn on_health_changed(
    query: Query<(Entity, &Health), Changed<Health>>,
) {
    for (entity, health) in query.iter() {
        // Only runs when health actually changes
    }
}
```

## AI Integration

### LLM Usage Guidelines

**When to use LLM:**
- Dynamic dialogue and conversation
- Procedural narrative generation
- Player behavior analysis
- Creative content generation

**When NOT to use LLM:**
- Real-time combat decisions (too slow)
- Deterministic game logic
- Anything requiring < 100ms response

### AI Architecture

```rust
// Layer AI appropriately
pub struct AiStack {
    // Fast: Behavior trees for immediate reactions
    pub behavior_tree: BehaviorTree,
    
    // Medium: GOAP for tactical planning
    pub planner: GoalPlanner,
    
    // Slow: LLM for strategic/narrative decisions
    pub llm_client: Option<LlmClient>,
}

impl AiStack {
    pub fn update(&mut self, context: &AiContext) {
        // Always run fast layer
        self.behavior_tree.tick(context);
        
        // Run planner when needed
        if self.needs_replan(context) {
            self.planner.plan(context);
        }
        
        // Async LLM calls for non-critical decisions
        if self.should_consult_llm(context) {
            self.queue_llm_request(context);
        }
    }
}
```

### Tool Validation

Always validate LLM tool calls:

```rust
pub fn validate_tool_call(call: &ToolCall, context: &GameContext) -> ToolResult {
    match call.name.as_str() {
        "move_to" => {
            let target: Vec3 = call.parse_param("target")?;
            
            // Validate target is reachable
            if !context.navmesh.is_reachable(context.position, target) {
                return ToolResult::Error("Target unreachable".into());
            }
            
            // Validate target is within range
            let distance = context.position.distance(target);
            if distance > context.max_move_distance {
                return ToolResult::Error("Target too far".into());
            }
            
            ToolResult::Success(json!({ "path_found": true }))
        }
        "attack" => {
            let target_id: u64 = call.parse_param("target")?;
            
            // Validate target exists and is attackable
            if !context.can_attack(target_id) {
                return ToolResult::Error("Cannot attack target".into());
            }
            
            ToolResult::Success(json!({ "attack_initiated": true }))
        }
        _ => ToolResult::Error("Unknown tool".into()),
    }
}
```

## Performance

### Frame Budget Management

```rust
#[derive(Resource)]
pub struct FrameBudget {
    pub target_fps: f32,
    pub ai_budget_pct: f32,
    pub physics_budget_pct: f32,
    pub render_budget_pct: f32,
}

impl FrameBudget {
    pub fn frame_time_ms(&self) -> f32 {
        1000.0 / self.target_fps
    }
    
    pub fn ai_budget_ms(&self) -> f32 {
        self.frame_time_ms() * self.ai_budget_pct
    }
}
```

### Async Operations

```rust
// Load assets asynchronously
fn load_level_async(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut loading_state: ResMut<LoadingState>,
) {
    if loading_state.started {
        return;
    }
    
    loading_state.handles.push(asset_server.load_async::<Scene>("levels/level1.scn"));
    loading_state.started = true;
}

// Check loading progress
fn check_loading(
    asset_server: Res<AssetServer>,
    loading_state: Res<LoadingState>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if loading_state.handles.iter().all(|h| asset_server.is_loaded(h)) {
        next_state.set(GameState::Playing);
    }
}
```

### Memory Management

```rust
// Use object pools for frequent spawn/despawn
#[derive(Resource)]
pub struct ProjectilePool {
    available: Vec<Entity>,
    active: HashSet<Entity>,
}

impl ProjectilePool {
    pub fn get(&mut self, commands: &mut Commands) -> Entity {
        if let Some(entity) = self.available.pop() {
            commands.entity(entity).insert(Visible);
            self.active.insert(entity);
            entity
        } else {
            let entity = commands.spawn(ProjectileBundle::default()).id();
            self.active.insert(entity);
            entity
        }
    }
    
    pub fn return_entity(&mut self, entity: Entity, commands: &mut Commands) {
        commands.entity(entity).remove::<Visible>();
        self.active.remove(&entity);
        self.available.push(entity);
    }
}
```

## Error Handling

### Graceful Degradation

```rust
pub fn ai_system_with_fallback(
    mut agents: Query<&mut AiAgent>,
    llm: Option<Res<LlmClient>>,
) {
    for mut agent in agents.iter_mut() {
        let decision = if let Some(ref llm) = llm {
            // Try LLM-based decision
            match llm.try_decide(&agent.context) {
                Ok(decision) => decision,
                Err(e) => {
                    warn!("LLM failed: {}, using fallback", e);
                    agent.fallback_decision()
                }
            }
        } else {
            // No LLM available, use behavior tree
            agent.behavior_tree.tick()
        };
        
        agent.execute(decision);
    }
}
```

### Structured Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum GameError {
    #[error("Asset not found: {0}")]
    AssetNotFound(String),
    
    #[error("Invalid game state: {0}")]
    InvalidState(String),
    
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),
    
    #[error("AI error: {0}")]
    Ai(#[from] AiError),
}

pub type GameResult<T> = Result<T, GameError>;
```

## Testing

### Unit Testing Systems

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_damage_system() {
        let mut world = World::new();
        
        let entity = world.spawn((Health(100.0), DamageQueue::default())).id();
        
        world.get_mut::<DamageQueue>(entity).unwrap().push(Damage(30.0));
        
        let mut schedule = Schedule::default();
        schedule.add_systems(apply_damage_system);
        schedule.run(&mut world);
        
        assert_eq!(world.get::<Health>(entity).unwrap().0, 70.0);
    }
}
```

### Integration Testing

```rust
#[test]
fn test_combat_flow() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(CombatPlugin);
    
    app.world.spawn((
        Player,
        Health(100.0),
        Transform::default(),
    ));
    
    app.world.spawn((
        Enemy,
        Health(50.0),
        Transform::from_xyz(5.0, 0.0, 0.0),
    ));
    
    // Simulate multiple frames
    for _ in 0..100 {
        app.update();
    }
    
    // Verify expected state
}
```

## Debugging

### Logging

```rust
use tracing::{info, warn, error, debug, trace, instrument};

#[instrument(skip(query))]
fn my_system(query: Query<(&Transform, &Health)>) {
    for (transform, health) in query.iter() {
        trace!("Processing entity at {:?}", transform.translation);
        
        if health.0 < 10.0 {
            warn!("Entity at low health: {}", health.0);
        }
    }
}
```

### Debug Visualization

```rust
fn debug_draw_system(
    mut gizmos: Gizmos,
    agents: Query<(&Transform, &AiAgent)>,
) {
    #[cfg(debug_assertions)]
    for (transform, agent) in agents.iter() {
        // Draw perception radius
        gizmos.circle(transform.translation, Vec3::Y, agent.perception_radius, Color::YELLOW);
        
        // Draw path
        if let Some(path) = &agent.current_path {
            for window in path.windows(2) {
                gizmos.line(window[0], window[1], Color::GREEN);
            }
        }
    }
}
```

## Security

### Input Validation

```rust
pub fn validate_player_input(input: &PlayerInput) -> Result<(), InputError> {
    // Validate movement speed
    if input.movement.length() > MAX_MOVEMENT_SPEED {
        return Err(InputError::InvalidMovement);
    }
    
    // Validate action cooldowns
    if input.action.is_some() && !can_perform_action() {
        return Err(InputError::ActionOnCooldown);
    }
    
    Ok(())
}
```

### Save Data Integrity

```rust
use sha2::{Sha256, Digest};

pub fn save_game(state: &GameState) -> Result<(), SaveError> {
    let data = bincode::serialize(state)?;
    let hash = Sha256::digest(&data);
    
    let save = SaveFile {
        version: SAVE_VERSION,
        data,
        checksum: hash.to_vec(),
    };
    
    std::fs::write("save.dat", bincode::serialize(&save)?)?;
    Ok(())
}

pub fn load_game() -> Result<GameState, LoadError> {
    let bytes = std::fs::read("save.dat")?;
    let save: SaveFile = bincode::deserialize(&bytes)?;
    
    // Verify checksum
    let hash = Sha256::digest(&save.data);
    if hash.as_slice() != save.checksum {
        return Err(LoadError::CorruptedSave);
    }
    
    Ok(bincode::deserialize(&save.data)?)
}
```

## Summary Checklist

```admonish tip title="Before Shipping"
- [ ] All systems have error handling
- [ ] Performance profiled on target hardware
- [ ] AI fallbacks tested
- [ ] Save/load tested extensively
- [ ] Memory usage profiled
- [ ] Input validation complete
- [ ] Logging appropriate for production
- [ ] All assets properly loaded
- [ ] Multiplayer desync handled (if applicable)
```

## Related Documentation

- [Performance Optimization](../dev/performance.md) - Detailed performance guide
- [Configuration Reference](../reference/configuration.md) - Configuration options
- [Testing Guide](../dev/testing.md) - Testing strategies
- [Design Patterns](patterns.md) - Common patterns
