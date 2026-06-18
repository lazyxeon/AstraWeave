# astraweave-gameplay

High-level gameplay systems and mechanics for AstraWeave.

## Overview

Game systems including combat, crafting, quests, dialogue, stats, inventory, biomes, and Veilweaver-specific mechanics.

## Modules

| Module | Description |
|--------|-------------|
| `combat` / `combat_physics` | Raycast attack sweep with cone filtering, parry, i-frames |
| `crafting` | Recipe system with material requirements |
| `quests` | Quest tracking with objectives and completion |
| `dialogue` | Branching NPC conversation trees |
| `stats` | Character statistics and damage calculation |
| `items` | Inventory management and item definitions |
| `biome` / `biome_spawn` | Biome definitions and spawn rules |
| `veilweaver_slice` | Veilweaver game-specific mechanics |
| `weaving` / `weave_portals` | Weaving system and portal mechanics |
| `ecs` | ECS system registration for all subsystems |

## Usage

```rust
use astraweave_gameplay::combat_physics::perform_attack_sweep;

let hits = perform_attack_sweep(
    &physics, attacker_id, &pos, &targets,
    range, &mut stats, &mut parry, &mut iframes,
);
```

## License

MIT
