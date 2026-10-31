---
layout: default
title: Gameplay Subsystem
---

# Gameplay (astraweave-gameplay)

AstraWeave's gameplay crate provides combat physics, crafting, quests, items, inventory, biome definitions, and the Veilweaver fate-weaving mechanic. The crate has **471 tests**.

## Key Systems

### Combat Physics

Raycast-based attack sweep with cone filtering and full damage pipeline:

```rust
use astraweave_gameplay::combat_physics::perform_attack_sweep;

let hits = perform_attack_sweep(
    &physics_world,
    attacker_id,
    &attacker_position,
    &target_list,
    attack_range,
    &mut stats_map,
    &mut parry_map,
    &mut iframe_map,
);
```

| Feature | Description |
|---------|-------------|
| Cone-angle filtering | Configurable melee/ranged sweep angle |
| Parry detection | Timing window-based parry with directional checks |
| Invincibility frames | Per-entity i-frame tracking prevents double-hits |
| Damage calculation | Armor/resistance reduction, critical hits |
| Hit registration | Raycast sweep returns all valid targets in cone |

### Crafting System

Recipe-based crafting with structured progression:

| Feature | Description |
|---------|-------------|
| Recipe definitions | Material requirements, output items |
| Unlockable tiers | Progressive access to advanced recipes |
| Failure chances | Skill-based success probability |
| Material consumption | Inventory integration for ingredient removal |

### Quest System

Full quest lifecycle management integrated with the HUD:

```
Quest Definition → Objective Tracking → Progress Events → Completion → Rewards
```

| Feature | Description |
|---------|-------------|
| Quest definitions | Structured quest data with objectives |
| Per-player progress | Individual objective tracking |
| Reward distribution | Items, XP, or gameplay unlocks on completion |
| HUD integration | Quest tracker widget shows active objectives |
| AI awareness | NPCs can reference active quests in dialogue |

### Items & Inventory

| Feature | Description |
|---------|-------------|
| Typed item database | Items categorized by type (weapon, armor, consumable, material) |
| Stack management | Stackable items with quantity tracking |
| Equipment slots | Head, chest, legs, weapon, shield, accessory |
| Consumable effects | Healing, buffs, debuffs with duration |

### Biome Definitions

Material and terrain configuration per biome, integrated with the terrain and PCG systems:

| Biome | Visual | Terrain |
|-------|--------|---------|
| Forest | Dense vegetation, temperate colors | Rolling hills, rivers |
| Desert | Arid, sand tones | Dunes, mesas |
| Tundra | Snow, ice | Frozen lakes, mountains |
| Grassland | Low vegetation, open fields | Plains, gentle slopes |
| Swamp | Dark, murky water | Wetlands, bog |

Biome data feeds into:
- `astraweave-render` for `BiomeVisuals` (fog, sky, water, clouds)
- `astraweave-terrain` for noise parameters and vegetation scatter
- `astraweave-materials` for per-biome texture arrays

### Veilweaver Mechanic

The fate-weaving system — AstraWeave's signature game mechanic:

| Metric | Value |
|--------|-------|
| Test coverage | 94.26% (line coverage) |
| Tests | 407 (394 lib + 13 integration) |
| Grade | A+ |

**Core Concept**: Players manipulate "threads of fate" that affect NPC destinies, unlocking branching narrative paths and emergent gameplay outcomes.

| Feature | Description |
|---------|-------------|
| Thread manipulation | Weave, cut, and redirect fate threads |
| NPC destiny effects | NPCs react to fate changes through AI perception |
| Deterministic outcomes | All fate resolutions are reproducible |
| AI integration | WorldSnapshot includes weaving state for AI reasoning |

The Veilweaver foundation has been audited as a solid base, with an estimated 6–8 weeks to a 30-minute playable vertical slice.

## Integration Points

| System | Integration |
|--------|-------------|
| **ECS** | All gameplay entities are ECS entities with components |
| **AI** | Combat stats, quest state feed into WorldSnapshot for AI planning |
| **Physics** | Combat uses physics raycasting for hit detection |
| **UI** | Quest tracker, health bars, damage numbers display gameplay data |
| **Terrain** | Biome definitions drive terrain generation and material assignment |
| **Rendering** | Biome visuals configure fog, sky, water per-area |

[← Back to Home](index.html) · [Architecture](architecture.html) · [Physics](physics.html)
