# Save & Load Systems

> **Crates**: `astraweave-persistence-ecs`, `astraweave-core`  
> **Status**: Production Ready

AstraWeave provides deterministic save/load with full ECS state serialization, player profiles, and versioned migrations.

## Quick Start

```rust
use astraweave_persistence_ecs::{SaveSystem, LoadSystem, SaveConfig};
use astraweave_ecs::World;

let config = SaveConfig {
    path: "saves/",
    compression: true,
    encryption: false,
    ..Default::default()
};

let save_system = SaveSystem::new(config);

// Save world state
save_system.save(&world, "quicksave")?;

// Load world state
let loaded_world = save_system.load("quicksave")?;
```

---

## Save System Architecture

```
Save File Structure
├── Header (version, timestamp, checksum)
├── World State
│   ├── Entities (sparse set serialization)
│   ├── Components (per-archetype)
│   └── Resources (global state)
├── Player Data
│   ├── Inventory
│   ├── Progress
│   └── Settings
└── Metadata
    ├── Screenshot (optional)
    └── Play time
```

---

## ECS Serialization

### Component Registration

Components must be registered for serialization:

```rust
use astraweave_ecs::{Component, Serialize, Deserialize};

#[derive(Component, Serialize, Deserialize)]
struct Health {
    current: f32,
    max: f32,
}

#[derive(Component, Serialize, Deserialize)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}

// Register with world
app.register_serializable::<Health>();
app.register_serializable::<Position>();
```

### Selective Serialization

Mark components to skip during save:

```rust
#[derive(Component)]
#[component(skip_save)]
struct RenderCache {
    // Regenerated at load time
}

#[derive(Component, Serialize, Deserialize)]
#[component(save_priority = "high")]
struct PlayerData {
    // Saved first, loaded first
}
```

---

## Save Slots

### Slot Management

```rust
use astraweave_persistence_ecs::{SaveSlot, SlotManager};

let mut slots = SlotManager::new("saves/", 10); // 10 slots

// Get available slots
let available = slots.list()?;
for slot in &available {
    println!("Slot {}: {} ({})", slot.index, slot.name, slot.timestamp);
}

// Save to slot
slots.save_to_slot(3, &world, "My Save")?;

// Load from slot
let world = slots.load_from_slot(3)?;

// Delete slot
slots.delete_slot(3)?;
```

### Auto-Save

```rust
use astraweave_persistence_ecs::AutoSave;

let mut autosave = AutoSave::new(Duration::from_secs(300)); // 5 minutes

// In game loop
if autosave.should_save() {
    slots.save_to_slot(0, &world, "Autosave")?;
    autosave.mark_saved();
}
```

---

## Player Profiles

### Profile Management

```rust
use astraweave_persistence_ecs::{PlayerProfile, ProfileManager};

let mut profiles = ProfileManager::new("profiles/")?;

// Create profile
let profile = PlayerProfile::new("Player1");
profiles.create(profile)?;

// Load profile
let profile = profiles.load("Player1")?;

// Update settings
profile.settings.volume = 0.8;
profile.settings.difficulty = Difficulty::Hard;
profiles.save(&profile)?;
```

### Profile Contents

```rust
pub struct PlayerProfile {
    pub name: String,
    pub created: DateTime,
    pub play_time: Duration,
    pub settings: PlayerSettings,
    pub achievements: Vec<Achievement>,
    pub statistics: PlayerStats,
}

pub struct PlayerSettings {
    pub volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub voice_volume: f32,
    pub difficulty: Difficulty,
    pub controls: ControlBindings,
    pub graphics: GraphicsSettings,
}
```

---

## Versioning & Migration

### Version Handling

```rust
use astraweave_persistence_ecs::{SaveVersion, Migration};

// Current save version
const SAVE_VERSION: SaveVersion = SaveVersion::new(1, 2, 0);

// Load with migration
let world = save_system.load_with_migrations(
    "old_save",
    &[
        Migration::new(
            SaveVersion::new(1, 0, 0),
            SaveVersion::new(1, 1, 0),
            migrate_1_0_to_1_1,
        ),
        Migration::new(
            SaveVersion::new(1, 1, 0),
            SaveVersion::new(1, 2, 0),
            migrate_1_1_to_1_2,
        ),
    ],
)?;

fn migrate_1_0_to_1_1(data: &mut SaveData) -> Result<()> {
    // Add new health.shield field
    for entity in data.entities_with::<Health>() {
        let health: &mut Health = data.get_mut(entity)?;
        health.shield = 0.0; // New field
    }
    Ok(())
}
```

---

## Corruption Recovery

### Backup System

```rust
use astraweave_persistence_ecs::BackupConfig;

let config = SaveConfig {
    backup: BackupConfig {
        enabled: true,
        max_backups: 3,
        backup_on_save: true,
    },
    ..Default::default()
};

// Backups created automatically
// saves/quicksave.sav
// saves/quicksave.sav.bak1
// saves/quicksave.sav.bak2
// saves/quicksave.sav.bak3
```

### Integrity Verification

```rust
use astraweave_persistence_ecs::verify_save;

match verify_save("saves/quicksave.sav") {
    Ok(info) => {
        println!("Valid save: v{}", info.version);
    }
    Err(SaveError::Corrupted) => {
        // Try backup
        if let Ok(backup) = find_valid_backup("quicksave") {
            restore_from_backup(backup)?;
        }
    }
    Err(SaveError::VersionMismatch(v)) => {
        println!("Save from future version: {}", v);
    }
}
```

---

## Deterministic Replay

AstraWeave's deterministic ECS enables save/load that reproduces exact game states:

```rust
use astraweave_core::replay::{ReplayRecorder, ReplayPlayer};

// Record session
let mut recorder = ReplayRecorder::new();
recorder.start(&world);

// During gameplay
recorder.record_frame(&world, &inputs);

// Save replay
recorder.save("replay.bin")?;

// Play back (bit-identical to original)
let mut player = ReplayPlayer::load("replay.bin")?;
while let Some(frame) = player.next_frame() {
    world.apply_inputs(frame.inputs);
    world.step();
    
    assert_eq!(world.checksum(), frame.checksum);
}
```

---

## Performance

| Operation | Latency | Notes |
|-----------|---------|-------|
| Quick save (1K entities) | ~50 ms | With compression |
| Quick load (1K entities) | ~30 ms | With decompression |
| Profile save | ~5 ms | Settings only |
| Checksum verify | ~2 ms | Per save file |

### Optimization Tips

1. **Async saves** - Don't block gameplay
2. **Incremental saves** - Only save changed data
3. **Compression** - 60-80% size reduction
4. **Memory-mapped loading** - Faster large worlds

```rust
// Async save
let handle = save_system.save_async(&world, "quicksave");

// Continue gameplay...

// Wait for completion
handle.await?;
```

---

## See Also

- [ECS Architecture](../architecture/ecs.md)
- [Deterministic Simulation](../architecture/deterministic.md)
- [Player Profiles](./player-profiles.md)
