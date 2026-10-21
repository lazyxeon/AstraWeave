# Design Patterns

This guide covers common design patterns used in AstraWeave game development, with practical examples and when to apply each pattern.

## Entity Patterns

### Marker Components

Use empty components to categorize entities:

```rust
#[derive(Component)]
struct Player;

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct NPC;

#[derive(Component)]
struct Interactable;

// Query with markers
fn player_system(query: Query<&Transform, With<Player>>) {
    for transform in query.iter() {
        // Only players
    }
}

fn enemy_system(query: Query<&Transform, (With<Enemy>, Without<Dead>)>) {
    for transform in query.iter() {
        // Only living enemies
    }
}
```

### Component Bundles

Group related components for entity archetypes:

```rust
#[derive(Bundle)]
struct CharacterBundle {
    transform: Transform,
    health: Health,
    velocity: Velocity,
    collider: Collider,
}

#[derive(Bundle)]
struct PlayerBundle {
    character: CharacterBundle,
    player: Player,
    inventory: Inventory,
    controller: PlayerController,
}

#[derive(Bundle)]
struct EnemyBundle {
    character: CharacterBundle,
    enemy: Enemy,
    ai: AiAgent,
    loot_table: LootTable,
}

// Spawn with bundles
fn spawn_player(mut commands: Commands) {
    commands.spawn(PlayerBundle {
        character: CharacterBundle {
            transform: Transform::default(),
            health: Health(100.0),
            velocity: Velocity::default(),
            collider: Collider::capsule(0.5, 1.8),
        },
        player: Player,
        inventory: Inventory::new(20),
        controller: PlayerController::default(),
    });
}
```

### Entity References

Store references between entities:

```rust
#[derive(Component)]
struct Parent(Entity);

#[derive(Component)]
struct Children(Vec<Entity>);

#[derive(Component)]
struct Target(Option<Entity>);

#[derive(Component)]
struct Owner(Entity);

fn targeting_system(
    mut agents: Query<(&Transform, &mut Target), With<Enemy>>,
    targets: Query<(Entity, &Transform), With<Player>>,
) {
    for (agent_transform, mut target) in agents.iter_mut() {
        let nearest = targets
            .iter()
            .min_by_key(|(_, t)| {
                OrderedFloat(t.translation.distance(agent_transform.translation))
            })
            .map(|(e, _)| e);
        
        target.0 = nearest;
    }
}
```

## State Patterns

### State Machine

Implement entity state machines:

```rust
#[derive(Component, Default)]
enum CharacterState {
    #[default]
    Idle,
    Walking,
    Running,
    Jumping,
    Attacking,
    Stunned { duration: f32 },
    Dead,
}

fn state_machine_system(
    time: Res<Time>,
    mut query: Query<(&mut CharacterState, &Velocity, &Health)>,
) {
    let dt = time.delta_seconds();
    
    for (mut state, velocity, health) in query.iter_mut() {
        // Check for death
        if health.0 <= 0.0 && !matches!(*state, CharacterState::Dead) {
            *state = CharacterState::Dead;
            continue;
        }
        
        // State transitions
        *state = match &*state {
            CharacterState::Stunned { duration } => {
                let remaining = duration - dt;
                if remaining <= 0.0 {
                    CharacterState::Idle
                } else {
                    CharacterState::Stunned { duration: remaining }
                }
            }
            CharacterState::Dead => CharacterState::Dead,
            _ => {
                if velocity.0.length() > 5.0 {
                    CharacterState::Running
                } else if velocity.0.length() > 0.1 {
                    CharacterState::Walking
                } else {
                    CharacterState::Idle
                }
            }
        };
    }
}
```

### Global Game State

Use app states for high-level game flow:

```rust
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum GameState {
    #[default]
    Loading,
    MainMenu,
    Playing,
    Paused,
    GameOver,
}

fn setup_states(app: &mut App) {
    app
        .add_state::<GameState>()
        .add_systems(OnEnter(GameState::Loading), start_loading)
        .add_systems(OnEnter(GameState::MainMenu), show_menu)
        .add_systems(OnExit(GameState::MainMenu), hide_menu)
        .add_systems(Update, game_logic.run_if(in_state(GameState::Playing)))
        .add_systems(OnEnter(GameState::Paused), show_pause_menu)
        .add_systems(OnEnter(GameState::GameOver), show_game_over);
}
```

## Resource Patterns

### Configuration Resources

Store runtime configuration:

```rust
#[derive(Resource, Default)]
struct GameSettings {
    master_volume: f32,
    music_volume: f32,
    sfx_volume: f32,
    mouse_sensitivity: f32,
    difficulty: Difficulty,
}

#[derive(Resource)]
struct LevelConfig {
    spawn_rate: f32,
    max_enemies: usize,
    boss_health_multiplier: f32,
}

fn load_settings(mut settings: ResMut<GameSettings>) {
    if let Ok(data) = std::fs::read_to_string("settings.toml") {
        if let Ok(loaded) = toml::from_str(&data) {
            *settings = loaded;
        }
    }
}
```

### Service Resources

Encapsulate complex functionality:

```rust
#[derive(Resource)]
struct AudioManager {
    music_channel: AudioChannel,
    sfx_channels: Vec<AudioChannel>,
    ambient_channel: AudioChannel,
}

impl AudioManager {
    fn play_music(&mut self, track: &str, fade_in: f32) {
        self.music_channel.fade_in(track, fade_in);
    }
    
    fn play_sfx(&mut self, sound: &str, position: Vec3) {
        if let Some(channel) = self.sfx_channels.iter_mut().find(|c| c.is_available()) {
            channel.play_spatial(sound, position);
        }
    }
}

#[derive(Resource)]
struct DialogueManager {
    active_dialogue: Option<DialogueTree>,
    history: Vec<DialogueEntry>,
}
```

## Event Patterns

### Game Events

Use events for decoupled communication:

```rust
#[derive(Event)]
struct DamageEvent {
    target: Entity,
    amount: f32,
    damage_type: DamageType,
    source: Option<Entity>,
}

#[derive(Event)]
struct DeathEvent {
    entity: Entity,
    killer: Option<Entity>,
    position: Vec3,
}

#[derive(Event)]
struct ItemPickupEvent {
    player: Entity,
    item: ItemId,
    quantity: u32,
}

fn damage_system(
    mut damage_events: EventReader<DamageEvent>,
    mut death_events: EventWriter<DeathEvent>,
    mut query: Query<(&mut Health, &Transform)>,
) {
    for event in damage_events.iter() {
        if let Ok((mut health, transform)) = query.get_mut(event.target) {
            health.0 -= event.amount;
            
            if health.0 <= 0.0 {
                death_events.send(DeathEvent {
                    entity: event.target,
                    killer: event.source,
                    position: transform.translation,
                });
            }
        }
    }
}

fn death_system(
    mut commands: Commands,
    mut death_events: EventReader<DeathEvent>,
    loot_query: Query<&LootTable>,
) {
    for event in death_events.iter() {
        // Spawn loot
        if let Ok(loot) = loot_query.get(event.entity) {
            spawn_loot(&mut commands, loot, event.position);
        }
        
        // Play death effects
        commands.spawn(ParticleEffect::death(event.position));
        
        // Mark entity for removal
        commands.entity(event.entity).insert(Dead);
    }
}
```

### Event Chains

Chain events for complex interactions:

```rust
// Event flow: Attack -> Hit -> Damage -> Death -> Loot -> XP

#[derive(Event)]
struct AttackEvent { attacker: Entity, weapon: Entity }

#[derive(Event)]
struct HitEvent { attacker: Entity, target: Entity, damage: f32 }

#[derive(Event)]
struct XpGainEvent { player: Entity, amount: u32 }

fn attack_resolution(
    mut attacks: EventReader<AttackEvent>,
    mut hits: EventWriter<HitEvent>,
    weapons: Query<&Weapon>,
    transforms: Query<&Transform>,
) {
    for attack in attacks.iter() {
        if let Ok(weapon) = weapons.get(attack.weapon) {
            // Perform hit detection
            for (entity, hit_point) in detect_hits(attack.attacker, weapon, &transforms) {
                hits.send(HitEvent {
                    attacker: attack.attacker,
                    target: entity,
                    damage: weapon.damage,
                });
            }
        }
    }
}
```

## AI Patterns

### Behavior Tree

Structure AI decisions hierarchically:

```rust
pub enum BtNode {
    Selector(Vec<BtNode>),
    Sequence(Vec<BtNode>),
    Condition(Box<dyn Fn(&AiContext) -> bool>),
    Action(Box<dyn Fn(&mut AiContext) -> BtStatus>),
}

#[derive(Clone, Copy, PartialEq)]
pub enum BtStatus {
    Success,
    Failure,
    Running,
}

fn create_enemy_bt() -> BtNode {
    BtNode::Selector(vec![
        // Priority 1: Flee if low health
        BtNode::Sequence(vec![
            BtNode::Condition(Box::new(|ctx| ctx.health_percent() < 0.2)),
            BtNode::Action(Box::new(|ctx| ctx.flee())),
        ]),
        // Priority 2: Attack if in range
        BtNode::Sequence(vec![
            BtNode::Condition(Box::new(|ctx| ctx.has_target())),
            BtNode::Condition(Box::new(|ctx| ctx.in_attack_range())),
            BtNode::Action(Box::new(|ctx| ctx.attack())),
        ]),
        // Priority 3: Chase target
        BtNode::Sequence(vec![
            BtNode::Condition(Box::new(|ctx| ctx.has_target())),
            BtNode::Action(Box::new(|ctx| ctx.chase_target())),
        ]),
        // Priority 4: Patrol
        BtNode::Action(Box::new(|ctx| ctx.patrol())),
    ])
}
```

### Blackboard

Share data between AI systems:

```rust
#[derive(Component, Default)]
pub struct Blackboard {
    values: HashMap<String, BlackboardValue>,
}

#[derive(Clone)]
pub enum BlackboardValue {
    Bool(bool),
    Float(f32),
    Int(i32),
    Entity(Entity),
    Vec3(Vec3),
    String(String),
}

impl Blackboard {
    pub fn set<K: Into<String>>(&mut self, key: K, value: BlackboardValue) {
        self.values.insert(key.into(), value);
    }
    
    pub fn get(&self, key: &str) -> Option<&BlackboardValue> {
        self.values.get(key)
    }
    
    pub fn get_entity(&self, key: &str) -> Option<Entity> {
        match self.get(key) {
            Some(BlackboardValue::Entity(e)) => Some(*e),
            _ => None,
        }
    }
}

fn perception_system(
    mut agents: Query<(&Transform, &Perception, &mut Blackboard)>,
    targets: Query<(Entity, &Transform), With<Player>>,
) {
    for (transform, perception, mut blackboard) in agents.iter_mut() {
        let nearest = find_nearest_in_range(transform, &targets, perception.radius);
        
        if let Some((entity, distance)) = nearest {
            blackboard.set("target", BlackboardValue::Entity(entity));
            blackboard.set("target_distance", BlackboardValue::Float(distance));
            blackboard.set("has_target", BlackboardValue::Bool(true));
        } else {
            blackboard.set("has_target", BlackboardValue::Bool(false));
        }
    }
}
```

## System Patterns

### System Sets

Organize systems with explicit ordering:

```rust
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum GameSystems {
    Input,
    Ai,
    Movement,
    Combat,
    Effects,
    Cleanup,
}

fn configure_systems(app: &mut App) {
    app.configure_sets(Update, (
        GameSystems::Input,
        GameSystems::Ai,
        GameSystems::Movement,
        GameSystems::Combat,
        GameSystems::Effects,
        GameSystems::Cleanup,
    ).chain());
    
    app
        .add_systems(Update, read_input.in_set(GameSystems::Input))
        .add_systems(Update, (
            ai_perception,
            ai_planning,
            ai_execution,
        ).chain().in_set(GameSystems::Ai))
        .add_systems(Update, (
            apply_velocity,
            resolve_collisions,
        ).chain().in_set(GameSystems::Movement));
}
```

### Run Conditions

Control when systems execute:

```rust
fn run_if_playing(state: Res<State<GameState>>) -> bool {
    *state.get() == GameState::Playing
}

fn run_if_has_enemies(enemies: Query<(), With<Enemy>>) -> bool {
    !enemies.is_empty()
}

fn run_every_n_frames(mut counter: Local<u32>) -> bool {
    *counter = (*counter + 1) % 5;
    *counter == 0
}

app
    .add_systems(Update, game_logic.run_if(run_if_playing))
    .add_systems(Update, enemy_ai.run_if(run_if_has_enemies))
    .add_systems(Update, expensive_system.run_if(run_every_n_frames));
```

## Data Patterns

### Flyweight

Share immutable data across entities:

```rust
#[derive(Resource)]
struct WeaponDatabase {
    weapons: HashMap<WeaponId, WeaponData>,
}

#[derive(Clone)]
struct WeaponData {
    name: String,
    base_damage: f32,
    attack_speed: f32,
    mesh: Handle<Mesh>,
    icon: Handle<Texture>,
}

#[derive(Component)]
struct Weapon {
    id: WeaponId,
    level: u32,
    durability: f32,
}

impl Weapon {
    fn get_data<'a>(&self, db: &'a WeaponDatabase) -> &'a WeaponData {
        db.weapons.get(&self.id).expect("Invalid weapon ID")
    }
    
    fn calculate_damage(&self, db: &WeaponDatabase) -> f32 {
        let data = self.get_data(db);
        data.base_damage * (1.0 + self.level as f32 * 0.1)
    }
}
```

### Observer

React to component changes:

```rust
fn on_health_changed(
    query: Query<(Entity, &Health, &MaxHealth), Changed<Health>>,
    mut damage_flash: Query<&mut DamageFlash>,
) {
    for (entity, health, max_health) in query.iter() {
        let health_percent = health.0 / max_health.0;
        
        // Trigger damage flash
        if let Ok(mut flash) = damage_flash.get_mut(entity) {
            flash.trigger();
        }
        
        // Update health bar
        if health_percent < 0.25 {
            // Critical health effects
        }
    }
}

fn on_entity_added(
    query: Query<Entity, Added<Enemy>>,
    mut enemy_count: ResMut<EnemyCount>,
) {
    for _entity in query.iter() {
        enemy_count.0 += 1;
    }
}
```

## Command Pattern

### Deferred Commands

Queue operations for later execution:

```rust
#[derive(Resource, Default)]
struct CommandQueue {
    commands: Vec<GameCommand>,
}

enum GameCommand {
    SpawnEnemy { position: Vec3, enemy_type: EnemyType },
    DealDamage { target: Entity, amount: f32 },
    GiveItem { player: Entity, item: ItemId },
    TriggerEvent { event_name: String },
}

fn queue_command(queue: &mut CommandQueue, command: GameCommand) {
    queue.commands.push(command);
}

fn execute_commands(
    mut commands: Commands,
    mut queue: ResMut<CommandQueue>,
    mut damage_events: EventWriter<DamageEvent>,
) {
    for command in queue.commands.drain(..) {
        match command {
            GameCommand::SpawnEnemy { position, enemy_type } => {
                commands.spawn(EnemyBundle::new(enemy_type, position));
            }
            GameCommand::DealDamage { target, amount } => {
                damage_events.send(DamageEvent { target, amount, .. });
            }
            // Handle other commands
            _ => {}
        }
    }
}
```

## Summary

| Pattern | Use Case |
|---------|----------|
| Marker Components | Entity categorization |
| Bundles | Entity archetypes |
| State Machine | Entity behavior |
| App States | Game flow |
| Events | Decoupled communication |
| Behavior Tree | AI decisions |
| Blackboard | AI data sharing |
| System Sets | Execution ordering |
| Run Conditions | Conditional execution |
| Flyweight | Shared data |
| Observer | Change reactions |
| Command Queue | Deferred execution |

## Related Documentation

- [Best Practices](best-practices.md) - General guidelines
- [ECS Architecture](../architecture/ecs.md) - ECS fundamentals
- [AI System](../core-systems/ai/index.md) - AI patterns in depth
- [Performance](../dev/performance.md) - Pattern performance
