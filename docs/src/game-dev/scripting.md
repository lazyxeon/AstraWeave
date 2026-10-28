# Scripting with Rhai

> **Not the live gameplay API.** This page describes callbacks, a `ScriptEngine` config, and a `raycast` function that `script_system` does not provide. As of #318 (`3971cbe1`), gameplay scripts do not receive `physics` or `nav`. The live contract is [docs/architecture/scripting.md](../../architecture/scripting.md) and [docs/guides/ai-scripting.md](../../guides/ai-scripting.md).

> **Crates**: `astraweave-scripting`, `astraweave-author`  
> **Status**: Historical sketch. The sandboxed tick is real. The snippets below are not.

AstraWeave provides secure Rhai scripting for modding, behavior authoring, and runtime customization with full sandboxing.

## Quick Start

```rhai
// scripts/my_behavior.rhai

// Called when entity spawns
fn on_spawn(entity) {
    log("Entity spawned: " + entity.id);
    entity.set_health(100);
    entity.play_animation("idle");
}

// Called every frame
fn on_update(entity, dt) {
    let player = find_nearest("Player");
    
    if distance(entity.position, player.position) < 10.0 {
        entity.look_at(player);
        entity.move_towards(player, dt * 5.0);
    }
}

// Called when taking damage
fn on_damage(entity, amount, source) {
    if entity.health - amount <= 0 {
        entity.play_animation("death");
        spawn_particles("blood", entity.position, 20);
    }
}
```

---

## Script Engine Setup

```rust
use astraweave_scripting::{ScriptEngine, ScriptConfig};

let config = ScriptConfig {
    max_operations: 100_000,    // Operation limit per call
    timeout_ms: 10,             // Max execution time
    memory_limit_kb: 1024,      // Memory limit
    allow_network: false,       // Network access
    allow_filesystem: false,    // File access
};

let mut engine = ScriptEngine::new(config);

// Register scripts
engine.load_script("behaviors/enemy.rhai")?;
engine.load_script("behaviors/companion.rhai")?;

// Execute
engine.call("on_spawn", (entity,))?;
```

---

## Available APIs

### Entity API

```rhai
// Position & Movement
entity.position              // Get Vec3
entity.set_position(x, y, z) // Set position
entity.move_towards(target, speed)
entity.look_at(target)
entity.rotate(yaw, pitch, roll)

// Health & Combat
entity.health               // Current health
entity.max_health           // Maximum health
entity.set_health(amount)
entity.damage(amount)
entity.heal(amount)
entity.is_dead()

// Animation
entity.play_animation(name)
entity.stop_animation()
entity.is_animating()

// Components
entity.has_component("Physics")
entity.get_tag("faction")
entity.set_tag("faction", "enemy")
```

### World API

```rhai
// Entity queries
let player = find_nearest("Player");
let enemies = find_all("Enemy");
let nearby = find_in_radius(position, radius);

// Spawning
let entity = spawn("enemy_type", position);
destroy(entity);

// Raycasting
let hit = raycast(origin, direction, max_distance);
if hit.success {
    log("Hit: " + hit.entity.name + " at " + hit.point);
}

// Time
let time = game_time();
let dt = delta_time();
```

### Math API

```rhai
// Vectors
let v = vec3(1.0, 2.0, 3.0);
let dist = distance(a, b);
let dir = normalize(v);
let dot = dot_product(a, b);
let cross = cross_product(a, b);

// Interpolation
let result = lerp(a, b, t);
let smooth = smoothstep(a, b, t);

// Random
let r = random();           // 0.0 - 1.0
let ri = random_range(1, 10); // 1 - 10
let choice = random_choice(array);
```

### Audio API

```rhai
// Sound playback
play_sound("sfx/explosion.ogg");
play_sound_at("sfx/growl.ogg", position);
play_music("music/combat.ogg", fade_in_seconds);
stop_music(fade_out_seconds);
set_volume("sfx", 0.8);
```

### Particle API

```rhai
spawn_particles("fire", position, count);
spawn_particles_at_entity("smoke", entity, count);
stop_particles(particle_system);
```

---

## Behavior Components

### Attaching Scripts to Entities

```rust
use astraweave_scripting::ScriptComponent;

commands.spawn((
    Transform::default(),
    Health::new(100),
    ScriptComponent::new("behaviors/enemy.rhai"),
));
```

### Callbacks

| Callback | When Called | Parameters |
|----------|-------------|------------|
| `on_spawn` | Entity created | `(entity)` |
| `on_update` | Every frame | `(entity, dt)` |
| `on_destroy` | Entity removed | `(entity)` |
| `on_collision` | Physics collision | `(entity, other, contact)` |
| `on_trigger` | Trigger overlap | `(entity, other, entered)` |
| `on_damage` | Taking damage | `(entity, amount, source)` |
| `on_death` | Health reaches 0 | `(entity, killer)` |
| `on_interact` | Player interaction | `(entity, player)` |

---

## Security Sandboxing

### Operation Limits

```rhai
// This will be terminated after 100,000 operations
fn infinite_loop(entity) {
    while true {
        // Will hit operation limit
    }
}
```

### Memory Limits

```rhai
fn memory_hog(entity) {
    let huge_array = [];
    for i in 0..1000000 {
        huge_array.push(i);  // Will hit memory limit
    }
}
```

### Blocked Operations

Scripts cannot:
- Access filesystem
- Open network connections
- Call system commands
- Modify engine internals
- Access other scripts' data

---

## Hot Reloading

Scripts can be modified at runtime:

```rust
// Enable hot reload
engine.enable_hot_reload(true);

// In game loop
engine.check_for_changes()?; // Reloads modified scripts
```

**Development workflow**:
1. Run game
2. Edit `.rhai` files
3. Changes apply immediately
4. No restart needed

---

## Custom Functions

### Exposing Rust to Scripts

```rust
use astraweave_scripting::ScriptEngine;

engine.register_fn("custom_ability", |entity: Entity, power: f64| {
    // Rust implementation
    let damage = power * 2.0;
    apply_area_damage(entity.position(), damage, 5.0);
});
```

### Using in Scripts

```rhai
fn on_special_attack(entity) {
    custom_ability(entity, 50.0);
    play_sound("sfx/special.ogg");
    spawn_particles("explosion", entity.position, 100);
}
```

---

## State Machines

### Defining States

```rhai
// AI state machine
let state = "idle";

fn on_update(entity, dt) {
    switch state {
        "idle" => {
            if see_player() {
                state = "chase";
            }
        }
        "chase" => {
            let player = find_nearest("Player");
            entity.move_towards(player, dt * 8.0);
            
            if distance(entity.position, player.position) < 2.0 {
                state = "attack";
            }
        }
        "attack" => {
            entity.attack();
            state = "cooldown";
            set_timer("attack_cooldown", 1.0);
        }
        "cooldown" => {
            if timer_expired("attack_cooldown") {
                state = "chase";
            }
        }
    }
}
```

---

## Debugging

### Logging

```rhai
log("Debug message");
log_warn("Warning message");
log_error("Error message");
```

### Console Commands

```rust
// Register console command
engine.register_command("spawn_enemy", |args| {
    let count = args.get(0).unwrap_or(1);
    for _ in 0..count {
        spawn("enemy", random_position());
    }
});
```

### Script Inspector

The editor provides:
- Breakpoints
- Variable inspection
- Call stack
- Performance profiling

---

## Performance

| Operation | Latency | Notes |
|-----------|---------|-------|
| Script call | ~5 µs | Per function call |
| Entity API | ~100 ns | Property access |
| Script load | ~1 ms | Parse + compile |
| Hot reload | ~2 ms | Per file |

### Optimization Tips

1. **Cache lookups** - Don't find entities every frame
2. **Limit scope** - Smaller scripts are faster
3. **Use events** - Don't poll in on_update
4. **Batch operations** - Group similar calls

```rhai
// Bad: Finding player every frame
fn on_update(entity, dt) {
    let player = find_nearest("Player"); // Expensive!
}

// Good: Cache reference
let cached_player = null;

fn on_spawn(entity) {
    cached_player = find_nearest("Player");
}

fn on_update(entity, dt) {
    if cached_player != null {
        entity.look_at(cached_player);
    }
}
```

---

## See Also

- [Behavior Trees](../core-systems/ai/behavior-trees.md)
- [AI System](../core-systems/ai/index.md)
- [Modding Guide](../dev/contributing.md)
- [Rhai Language Reference](https://rhai.rs/book/)
