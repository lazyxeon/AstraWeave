# Save & Load Systems

> **Crate**: `astraweave-persistence-ecs`

`SaveSystem`, `SaveConfig`, `SlotManager`, `AutoSave`, `PlayerProfile`, `ProfileManager`, `SaveVersion`, `Migration`, `BackupConfig`, and `verify_save` are not types or functions in `astraweave-persistence-ecs/src/lib.rs`. `astraweave_core::replay` is not a module.

## What the crate exports

| Name | Role |
|------|------|
| `CPersistenceManager` | Holds `aw_save::SaveManager` and `current_player: String` |
| `save_game` | `(&self, slot: u8, world_tick: u64, world_hash: u64, ecs_blob: Vec<u8>) -> Result<PathBuf>` |
| `save_game_with_replay` | Same arguments plus `replay_events: &[ReplayEvent]` |
| `load_game` | `(&self, slot: u8) -> Result<(SaveBundleV2, PathBuf)>` |
| `start_replay` | `(&self, slot: u8) -> Result<CReplayState>` |
| `list_saves` | `(&self) -> Result<Vec<aw_save::SaveMeta>>` |
| `migrate_save` | `(&self, path, resave: bool) -> Result<SaveBundleV2>` |
| `serialize_ecs_world` | `(&World) -> Result<Vec<u8>>` |
| `deserialize_ecs_world` | `(&[u8], &mut World) -> Result<()>` |
| `calculate_world_hash` | `(&World) -> u64` |
| `PersistencePlugin` | `new(save_directory: PathBuf)`. `build` registers `auto_save_system` and `replay_system` |
| `CSimClock` | `world_seed`, `tick`, `save_every`, `last_saved_tick`, `slot`. `save_every` is a sim-tick interval |
| `CReplayState` | `is_replaying`, `current_tick`, `total_ticks`, `events` |
| `ReplayEvent` | `tick`, `event_type: String`, `data: Vec<u8>` |
| `AppliedReplay` | Events `replay_system` copied for the current tick |

`serialize_ecs_world` writes entities that have any of `CPos`, `CHealth`, `CTeam`, `CAmmo`, `CCooldowns`, `CDesiredPos`, `CAiAgent`, `CLegacyId`, `CPersona`, or `CMemory`.

There is no `register_serializable` on `App`, and no `#[component(skip_save)]` attribute in this crate.

## Call the functions that exist

```rust
use astraweave_ecs::World;
use astraweave_persistence_ecs::{calculate_world_hash, serialize_ecs_world};
use aw_save::SaveManager;
use astraweave_persistence_ecs::CPersistenceManager;

fn save_empty(dir: &std::path::Path) -> anyhow::Result<()> {
    let world = World::new();
    let blob = serialize_ecs_world(&world)?;
    let hash = calculate_world_hash(&world);
    let manager = CPersistenceManager {
        save_manager: SaveManager::new(dir),
        current_player: "player".into(),
    };
    let _path = manager.save_game(0, 0, hash, blob)?;
    Ok(())
}
```

`SaveManager::new` takes a directory and returns `Self`. `save_game` forwards to `save_game_with_replay` with an empty event slice.

`auto_save_system` runs when a `CSimClock` resource is present, `save_every` is not zero, `tick % save_every == 0`, and `last_saved_tick` is not already that tick. It serializes the world, hashes the blob with FNV-64, and calls `save_game_with_replay`.

## See Also

- [ECS Architecture](../architecture/ecs.md)
- [ECS API](../api/ecs.md)
