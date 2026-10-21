# AstraWeave Authoring Schemas

**Purpose**: Complete reference for all file formats used in the AstraWeave Editor  
**Version**: 0.1.0  
**Last Updated**: January 2025

This document provides schema definitions, examples, validation rules, and field descriptions for all authoring formats supported by `aw_editor`.

---

## Table of Contents

1. [Level Format](#1-level-format)
2. [Behavior Tree Format](#2-behavior-tree-format)
3. [Dialogue Format](#3-dialogue-format)
4. [Quest Format](#4-quest-format)
5. [Material Format](#5-material-format)
6. [Terrain Grid Format](#6-terrain-grid-format)
7. [Navmesh Metadata Format](#7-navmesh-metadata-format)
8. [Asset Database Format](#8-asset-database-format)
9. [Validation Rules](#9-validation-rules)
10. [Git Integration](#10-git-integration)

---

## 1. Level Format

### Overview
- **Purpose**: Define world geometry, NPCs, fate threads, and boss encounters
- **Formats**: TOML (primary), JSON (alternative)
- **Paths**:
  - TOML: `content/levels/{title}.level.toml`
  - JSON: `content/levels/{title}.level.json`
- **Usage**: Created/edited in `aw_editor`, loaded by engine at runtime

---

### Schema

```rust
struct LevelDoc {
    title: String,             // Unique level name
    biome: String,             // Biome ID (e.g., "temperate_forest")
    seed: u32,                 // World generation seed
    sky: SkyConditions,        // Lighting and weather
    obstacles: Vec<Obstacle>,  // Static geometry
    npcs: Vec<NPCSpawn>,       // NPC spawners
    biome_paints: Vec<BiomePaint>, // Terrain biome overrides
    fate_threads: Vec<FateThread>, // Trigger-based events
    boss: Option<BossEncounter>,   // Boss fight data
}

struct SkyConditions {
    time_of_day: String,    // "dawn", "noon", "dusk", "midnight"
    weather: String,        // "clear", "fog_light", "fog_heavy", "rain", "snow"
}

struct Obstacle {
    id: String,             // Prefab/archetype ID
    pos: [f32; 3],          // World position [x, y, z]
    yaw: f32,               // Rotation in radians
    tags: Vec<String>,      // Gameplay tags (e.g., "cover", "climbable")
}

struct NPCSpawn {
    archetype: String,      // NPC archetype ID
    count: u32,             // Number to spawn
    spawn: SpawnArea,       // Spawn location
    behavior: String,       // Initial behavior mode
}

struct SpawnArea {
    pos: [f32; 3],          // Center position
    radius: f32,            // Scatter radius
}

struct BiomePaint {
    kind: String,           // Biome variant (e.g., "grass_dense", "forest_pine")
    area: PaintArea,        // Affected region
}

struct PaintArea {
    cx: i32,                // Center X (grid units)
    cz: i32,                // Center Z (grid units)
    radius: i32,            // Radius (grid units)
}

struct FateThread {
    name: String,           // Unique thread ID
    triggers: Vec<Trigger>, // Activation conditions
    ops: Vec<DirectorOp>,   // Director operations to execute
}

struct Trigger {
    kind: String,           // "enter_area", "timer", "npc_death", etc.
    // Kind-specific fields:
    center: Option<[f32; 3]>,   // For enter_area
    radius: Option<f32>,        // For enter_area
    duration: Option<f32>,      // For timer
    npc_id: Option<String>,     // For npc_death
}

struct DirectorOp {
    op: String,             // Operation type
    // Op-specific fields (see examples)
}

struct BossEncounter {
    director_budget_script: String,  // Path to budget Rhai script
    phase_script: String,            // Path to phase Rhai script
}
```

---

### TOML Example

```toml
title = "Forest Breach"
biome = "temperate_forest"
seed = 123456

[sky]
time_of_day = "dawn"
weather = "fog_light"

# Biome overrides
[[biome_paints]]
kind = "grass_dense"

[biome_paints.area]
cx = 0
cz = 0
radius = 64

[[biome_paints]]
kind = "forest_pine"

[biome_paints.area]
cx = -20
cz = 15
radius = 30

# Static obstacles
[[obstacles]]
id = "rock_big_01"
pos = [12.0, 0.0, -8.0]
yaw = 1.57
tags = ["cover", "climbable"]

[[obstacles]]
id = "tree_oak_02"
pos = [-5.0, 0.0, 10.0]
yaw = 0.0
tags = ["tree", "cover"]

# NPC spawns
[[npcs]]
archetype = "wolf_pack"
count = 3
behavior = "patrol"

[npcs.spawn]
pos = [-15.0, 0.0, 12.0]
radius = 3.0

[[npcs]]
archetype = "bear"
count = 1
behavior = "idle"

[npcs.spawn]
pos = [8.0, 0.0, -12.0]
radius = 0.5

# Fate threads (dynamic events)
[[fate_threads]]
name = "opening_ambush"

[[fate_threads.triggers]]
kind = "enter_area"
center = [0.0, 0.0, 0.0]
radius = 6.0

[[fate_threads.ops]]
op = "Fortify"

[fate_threads.ops.area]
cx = 8
cz = -6
r = 6

[[fate_threads.ops]]
op = "SpawnWave"
archetype = "wolf_pack"
count = 2
scatter = 2.5

# Boss encounter (optional)
[boss]
director_budget_script = "content/encounters/forest_breach.budget.rhai"
phase_script = "content/encounters/forest_breach.phases.rhai"
```

---

### JSON Example

```json
{
  "title": "Forest Breach",
  "biome": "temperate_forest",
  "seed": 123456,
  "sky": {
    "time_of_day": "dawn",
    "weather": "fog_light"
  },
  "obstacles": [
    {
      "id": "rock_big_01",
      "pos": [12.0, 0.0, -8.0],
      "yaw": 1.57,
      "tags": ["cover", "climbable"]
    }
  ],
  "npcs": [
    {
      "archetype": "wolf_pack",
      "count": 3,
      "spawn": {
        "pos": [-15.0, 0.0, 12.0],
        "radius": 3.0
      },
      "behavior": "patrol"
    }
  ],
  "biome_paints": [
    {
      "kind": "grass_dense",
      "area": {
        "cx": 0,
        "cz": 0,
        "radius": 64
      }
    }
  ],
  "fate_threads": [
    {
      "name": "opening_ambush",
      "triggers": [
        {
          "kind": "enter_area",
          "center": [0.0, 0.0, 0.0],
          "radius": 6.0
        }
      ],
      "ops": [
        {
          "op": "Fortify",
          "area": { "cx": 8, "cz": -6, "r": 6 }
        },
        {
          "op": "SpawnWave",
          "archetype": "wolf_pack",
          "count": 2,
          "scatter": 2.5
        }
      ]
    }
  ],
  "boss": {
    "director_budget_script": "content/encounters/forest_breach.budget.rhai",
    "phase_script": "content/encounters/forest_breach.phases.rhai"
  }
}
```

---

### Field Descriptions

| Field | Type | Required | Description | Default |
|-------|------|----------|-------------|---------|
| `title` | String | Yes | Unique level identifier | - |
| `biome` | String | Yes | Base biome ID | - |
| `seed` | u32 | Yes | World generation seed | - |
| `sky.time_of_day` | String | Yes | Lighting preset | "noon" |
| `sky.weather` | String | Yes | Weather effect | "clear" |
| `obstacles[]` | Array | No | Static geometry | [] |
| `npcs[]` | Array | No | NPC spawners | [] |
| `biome_paints[]` | Array | No | Biome overrides | [] |
| `fate_threads[]` | Array | No | Event triggers | [] |
| `boss` | Object | No | Boss data | null |

---

### Validation Rules

1. **Title**: Non-empty, alphanumeric + underscores
2. **Biome**: Must exist in biome registry
3. **Seed**: Any u32 value
4. **Sky.time_of_day**: One of `["dawn", "noon", "dusk", "midnight"]`
5. **Sky.weather**: One of `["clear", "fog_light", "fog_heavy", "rain", "snow"]`
6. **Obstacles**:
   - `id` must exist in asset database
   - `pos` must be finite floats
   - `yaw` in radians (-π to π)
   - `tags` must be valid gameplay tags
7. **NPCs**:
   - `archetype` must exist in NPC registry
   - `count` must be > 0
   - `spawn.radius` must be >= 0
8. **BiomePaints**:
   - `kind` must be valid biome variant
   - `area.radius` must be > 0
9. **FateThreads**:
   - `name` must be unique within level
   - `triggers[]` must have at least 1 trigger
   - `ops[]` must have at least 1 operation
10. **Boss**:
    - Scripts must exist at specified paths
    - Must be valid Rhai syntax

---

## 2. Behavior Tree Format

### Overview
- **Purpose**: Define AI behavior hierarchies
- **Format**: TOML
- **Path**: `assets/behavior/{behavior_id}.bt.toml`
- **Usage**: Edited in Behavior Graph Editor, loaded by AI orchestrator

---

### Schema

```rust
enum BehaviorNode {
    Action(String),               // Leaf: Execute action
    Condition(String),            // Leaf: Check condition
    Sequence(Vec<BehaviorNode>),  // Composite: Execute children in order
    Selector(Vec<BehaviorNode>),  // Composite: First success wins
    Parallel(Vec<BehaviorNode>),  // Composite: Execute simultaneously
    Decorator {                   // Decorator: Modify child behavior
        kind: String,             // "invert", "repeat", "until_fail"
        child: Box<BehaviorNode>,
    },
}

struct BehaviorTree {
    id: String,                   // Unique tree ID
    root: BehaviorNode,           // Root node
}
```

---

### TOML Example

```toml
id = "guard_patrol"

[root]
type = "Sequence"

[[root.children]]
type = "Condition"
check = "has_patrol_route"

[[root.children]]
type = "Action"
action = "follow_patrol_route"

[[root.children]]
type = "Selector"

[[root.children.children]]
type = "Sequence"

[[root.children.children.children]]
type = "Condition"
check = "enemy_in_sight"

[[root.children.children.children]]
type = "Action"
action = "engage_enemy"

[[root.children.children]]
type = "Action"
action = "idle"
```

---

### Field Descriptions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | String | Yes | Unique behavior tree ID |
| `root` | Object | Yes | Root node definition |
| `root.type` | String | Yes | Node type: Action, Condition, Sequence, Selector, Parallel, Decorator |
| `root.children` | Array | If composite | Child nodes (for Sequence, Selector, Parallel) |
| `root.action` | String | If Action | Action ID to execute |
| `root.check` | String | If Condition | Condition ID to evaluate |
| `root.kind` | String | If Decorator | Decorator type: "invert", "repeat", "until_fail" |
| `root.child` | Object | If Decorator | Single child node |

---

### Validation Rules

1. **ID**: Non-empty, unique across all BTs
2. **Root**: Must be a valid node
3. **Action**: `action` field must exist in action registry
4. **Condition**: `check` field must exist in condition registry
5. **Sequence/Selector/Parallel**: Must have 1+ children
6. **Decorator**: Must have exactly 1 child
7. **No Cycles**: Tree must be acyclic (no recursive references)

---

## 3. Dialogue Format

### Overview
- **Purpose**: Define branching dialogue trees
- **Format**: JSON
- **Path**: `assets/dialogue/{dialogue_id}.dialogue.json`
- **Usage**: Edited in Dialogue Graph Editor, loaded by dialogue system

---

### Schema

```rust
struct DialogueTree {
    id: String,                   // Unique dialogue tree ID
    nodes: Vec<DialogueNode>,     // All dialogue nodes
}

struct DialogueNode {
    id: String,                   // Unique node ID within tree
    text: String,                 // NPC dialogue text
    responses: Vec<DialogueResponse>, // Player response options
}

struct DialogueResponse {
    text: String,                 // Player response text
    next_id: Option<String>,      // Next node ID (null = end)
    condition: Option<String>,    // Visibility condition (Rhai expression)
}
```

---

### JSON Example

```json
{
  "id": "merchant_greeting",
  "nodes": [
    {
      "id": "start",
      "text": "Welcome, traveler! What can I do for you?",
      "responses": [
        {
          "text": "Show me your wares.",
          "next_id": "shop_menu",
          "condition": null
        },
        {
          "text": "Tell me about this place.",
          "next_id": "lore_exposition",
          "condition": null
        },
        {
          "text": "Goodbye.",
          "next_id": null,
          "condition": null
        }
      ]
    },
    {
      "id": "shop_menu",
      "text": "Here's what I have in stock.",
      "responses": [
        {
          "text": "I'll buy the sword.",
          "next_id": "purchase_sword",
          "condition": "player.gold >= 100"
        },
        {
          "text": "Never mind.",
          "next_id": "start",
          "condition": null
        }
      ]
    },
    {
      "id": "purchase_sword",
      "text": "Excellent choice! That'll be 100 gold.",
      "responses": [
        {
          "text": "Thanks!",
          "next_id": null,
          "condition": null
        }
      ]
    },
    {
      "id": "lore_exposition",
      "text": "This village has stood for centuries...",
      "responses": [
        {
          "text": "Fascinating. What else?",
          "next_id": "start",
          "condition": null
        }
      ]
    }
  ]
}
```

---

### Field Descriptions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | String | Yes | Unique dialogue tree ID |
| `nodes[]` | Array | Yes | All nodes in the tree (1+ required) |
| `nodes[].id` | String | Yes | Unique node ID within tree |
| `nodes[].text` | String | Yes | NPC dialogue text |
| `nodes[].responses[]` | Array | Yes | Player response options (1+ required) |
| `responses[].text` | String | Yes | Player response text |
| `responses[].next_id` | String? | Yes | Next node ID (null = end conversation) |
| `responses[].condition` | String? | No | Rhai expression for visibility (null = always visible) |

---

### Validation Rules

1. **Tree ID**: Non-empty, unique across all dialogue trees
2. **Nodes**: Must have at least 1 node
3. **Node IDs**: Unique within tree, non-empty
4. **Text**: Non-empty
5. **Responses**: Each node must have 1+ responses
6. **next_id**: Must reference existing node or be null
7. **No Orphans**: All nodes must be reachable from "start" node
8. **No Cycles** (Optional): Detect infinite loops
9. **Condition Syntax**: Must be valid Rhai expression

---

## 4. Quest Format

### Overview
- **Purpose**: Define quest step sequences
- **Format**: TOML
- **Path**: `assets/quests/{quest_id}.quest.toml`
- **Usage**: Edited in Quest Graph Editor, loaded by quest system

---

### Schema

```rust
struct Quest {
    id: String,                   // Unique quest ID
    title: String,                // Display name
    description: String,          // Long description
    steps: Vec<QuestStep>,        // Sequential steps
}

struct QuestStep {
    id: String,                   // Unique step ID within quest
    description: String,          // Step objective text
    objectives: Vec<Objective>,   // Completion requirements
    next: Option<String>,         // Next step ID (null = quest complete)
    on_complete: Vec<String>,     // Rhai scripts to run on completion
}

struct Objective {
    kind: String,                 // Objective type
    target: String,               // Target ID
    count: u32,                   // Required count
    text: String,                 // Display text
}
```

---

### TOML Example

```toml
id = "wolves_at_the_gate"
title = "Wolves at the Gate"
description = "The village is under attack by wolf packs. Defend the villagers and discover the source of the threat."

[[steps]]
id = "defend_village"
description = "Kill 5 wolves threatening the village"
next = "investigate_den"

[[steps.objectives]]
kind = "kill"
target = "wolf"
count = 5
text = "Wolves slain: {current}/{total}"

[[steps.on_complete]]
script = "quests/wolves/reward_gold.rhai"

[[steps]]
id = "investigate_den"
description = "Search the wolf den for clues"
next = "confront_alpha"

[[steps.objectives]]
kind = "reach_area"
target = "wolf_den"
count = 1
text = "Investigate the wolf den"

[[steps]]
id = "confront_alpha"
description = "Defeat the alpha wolf"
next = null

[[steps.objectives]]
kind = "kill"
target = "wolf_alpha"
count = 1
text = "Defeat the alpha wolf"

[[steps.on_complete]]
script = "quests/wolves/unlock_region.rhai"
```

---

### Field Descriptions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | String | Yes | Unique quest ID |
| `title` | String | Yes | Display name (shown in quest log) |
| `description` | String | Yes | Long description |
| `steps[]` | Array | Yes | Sequential quest steps (1+ required) |
| `steps[].id` | String | Yes | Unique step ID within quest |
| `steps[].description` | String | Yes | Step objective text |
| `steps[].objectives[]` | Array | Yes | Completion requirements (1+ required) |
| `steps[].next` | String? | Yes | Next step ID (null = quest complete) |
| `steps[].on_complete[]` | Array | No | Rhai scripts to run on step completion |
| `objectives[].kind` | String | Yes | Objective type: "kill", "collect", "reach_area", "talk_to" |
| `objectives[].target` | String | Yes | Target ID (entity archetype, item ID, area ID, NPC ID) |
| `objectives[].count` | u32 | Yes | Required count to complete objective |
| `objectives[].text` | String | Yes | Display text (supports {current}/{total} placeholders) |

---

### Validation Rules

1. **Quest ID**: Non-empty, unique across all quests
2. **Title/Description**: Non-empty
3. **Steps**: Must have at least 1 step
4. **Step IDs**: Unique within quest, non-empty
5. **next**: Must reference existing step or be null
6. **No Cycles**: Step chain must be acyclic
7. **Reachability**: All steps must be reachable from first step
8. **Objectives**: Each step must have 1+ objectives
9. **Objective kind**: Must be one of valid types
10. **Objective target**: Must reference valid entity/item/area/NPC
11. **on_complete**: Scripts must exist at specified paths

---

## 5. Material Format

### Overview
- **Purpose**: Define PBR material properties
- **Format**: JSON
- **Path**: `assets/material_live.json` (live editing), `assets/materials/{material_id}.mat.json` (permanent)
- **Usage**: Edited in Material Editor, loaded by render system

---

### Schema

```rust
struct Material {
    base_color: [f32; 4],     // RGBA (0.0-1.0)
    metallic: f32,            // 0.0 = dielectric, 1.0 = metal
    roughness: f32,           // 0.04-1.0 (0.0 causes artifacts)
    texture_path: Option<String>, // Path to albedo texture
    normal_map: Option<String>,   // Path to normal map
    ao_map: Option<String>,       // Path to ambient occlusion map
}
```

---

### JSON Example

```json
{
  "base_color": [0.8, 0.7, 0.6, 1.0],
  "metallic": 0.1,
  "roughness": 0.7,
  "texture_path": "assets/textures/rock_diffuse.png",
  "normal_map": "assets/textures/rock_normal.png",
  "ao_map": "assets/textures/rock_ao.png"
}
```

---

### Field Descriptions

| Field | Type | Required | Description | Range |
|-------|------|----------|-------------|-------|
| `base_color` | [f32; 4] | Yes | RGBA color | [0.0-1.0] each |
| `metallic` | f32 | Yes | Metalness | 0.0-1.0 |
| `roughness` | f32 | Yes | Surface roughness | 0.04-1.0 |
| `texture_path` | String? | No | Albedo texture | Valid path |
| `normal_map` | String? | No | Normal map | Valid path |
| `ao_map` | String? | No | AO map | Valid path |

---

### Validation Rules

1. **base_color**: All components in [0.0, 1.0]
2. **metallic**: In [0.0, 1.0]
3. **roughness**: In [0.04, 1.0] (avoid 0.0 to prevent artifacts)
4. **Texture paths**: Must exist in asset database if specified
5. **Texture formats**: PNG, JPEG, TGA, DDS supported

---

## 6. Terrain Grid Format

### Overview
- **Purpose**: Define biome grid for terrain painting
- **Format**: JSON
- **Path**: `assets/terrain_grid.json`
- **Usage**: Edited in Terrain Painter, synced to level.biome_paints

---

### Schema

```rust
type TerrainGrid = Vec<Vec<String>>;  // 10×10 grid of biome IDs
```

---

### JSON Example

```json
[
  ["grass", "grass", "forest", "forest", "forest", "mountain", "mountain", "water", "water", "grass"],
  ["grass", "grass", "forest", "forest", "forest", "mountain", "mountain", "water", "water", "grass"],
  ["grass", "forest", "forest", "forest", "mountain", "mountain", "mountain", "water", "grass", "grass"],
  ["grass", "forest", "forest", "mountain", "mountain", "mountain", "water", "water", "grass", "grass"],
  ["grass", "grass", "forest", "mountain", "mountain", "water", "water", "grass", "grass", "grass"],
  ["grass", "grass", "forest", "mountain", "water", "water", "grass", "grass", "grass", "grass"],
  ["grass", "grass", "forest", "forest", "water", "water", "grass", "grass", "grass", "grass"],
  ["grass", "grass", "forest", "forest", "forest", "water", "grass", "grass", "grass", "grass"],
  ["grass", "grass", "grass", "forest", "forest", "forest", "grass", "grass", "grass", "grass"],
  ["grass", "grass", "grass", "grass", "forest", "grass", "grass", "grass", "grass", "grass"]
]
```

---

### Field Descriptions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| Grid | Array<Array<String>> | Yes | 10×10 grid of biome IDs |
| Cell | String | Yes | Biome ID: "grass", "forest", "mountain", "water" |

---

### Validation Rules

1. **Dimensions**: Must be exactly 10×10
2. **Biome IDs**: Each cell must be a valid biome ID
3. **Valid IDs**: `["grass", "forest", "mountain", "water"]` (extensible)
4. **Grid Mapping**: Cell (x, y) → World position (x*10, y*10)
5. **Sync to Level**: Converts grid to `BiomePaint` entries with `area.radius = 5`

---

## 7. Navmesh Metadata Format

### Overview
- **Purpose**: Store navmesh baking parameters and results
- **Format**: JSON
- **Path**: `assets/navmesh/{level_id}.nav.json`
- **Usage**: Generated by Navmesh Baking, loaded by navigation system

---

### Schema

```rust
struct NavmeshMetadata {
    level_id: String,         // Associated level ID
    bake_params: BakeParams,  // Baking parameters used
    triangle_count: usize,    // Number of triangles generated
    bake_time_ms: u64,        // Time taken to bake
    timestamp: String,        // ISO 8601 timestamp
}

struct BakeParams {
    max_step: f32,            // Agent max step height
    max_slope_deg: f32,       // Max walkable slope (degrees)
    agent_radius: f32,        // Agent radius for erosion
}
```

---

### JSON Example

```json
{
  "level_id": "forest_breach",
  "bake_params": {
    "max_step": 0.4,
    "max_slope_deg": 60.0,
    "agent_radius": 0.3
  },
  "triangle_count": 248,
  "bake_time_ms": 157,
  "timestamp": "2025-01-15T14:32:18Z"
}
```

---

### Field Descriptions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `level_id` | String | Yes | Associated level ID |
| `bake_params.max_step` | f32 | Yes | Agent max step height (meters) |
| `bake_params.max_slope_deg` | f32 | Yes | Max walkable slope (degrees) |
| `bake_params.agent_radius` | f32 | Yes | Agent radius (meters) |
| `triangle_count` | usize | Yes | Number of triangles generated |
| `bake_time_ms` | u64 | Yes | Bake time in milliseconds |
| `timestamp` | String | Yes | ISO 8601 timestamp |

---

### Validation Rules

1. **level_id**: Must match existing level
2. **max_step**: Must be > 0.0
3. **max_slope_deg**: Must be in [0.0, 90.0]
4. **agent_radius**: Must be > 0.0
5. **triangle_count**: Must be > 0 for valid navmesh
6. **timestamp**: Must be valid ISO 8601 format

---

## 8. Asset Database Format

### Overview
- **Purpose**: Catalog all game assets (textures, models, audio, etc.)
- **Format**: JSON
- **Path**: `assets/assets.json`
- **Usage**: Auto-generated by `aw_asset_cli`, browsed in Asset Inspector

---

### Schema

```rust
struct AssetDatabase {
    assets: Vec<AssetEntry>,
}

struct AssetEntry {
    guid: String,             // Unique asset ID
    kind: String,             // Asset type: "texture", "model", "audio", "script"
    path: String,             // Relative path from assets/
    size: u64,                // File size in bytes
    hash: String,             // SHA-256 hash
    modified: String,         // ISO 8601 timestamp
    dependencies: Vec<String>, // GUIDs of dependent assets
}
```

---

### JSON Example

```json
{
  "assets": [
    {
      "guid": "a3f8c2d1-4b5e-6789-abcd-ef0123456789",
      "kind": "texture",
      "path": "textures/rock_diffuse.png",
      "size": 2048576,
      "hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "modified": "2025-01-10T08:15:30Z",
      "dependencies": []
    },
    {
      "guid": "b4e9d3e2-5c6f-789a-bcde-f01234567890",
      "kind": "material",
      "path": "materials/rock.mat.json",
      "size": 512,
      "hash": "a8f5e6d72b4c3a1e9f8d7c6b5a4e3d2c1b0a9e8d7c6b5a4e3d2c1b0a9e8d7c6b5",
      "modified": "2025-01-12T10:45:00Z",
      "dependencies": ["a3f8c2d1-4b5e-6789-abcd-ef0123456789"]
    },
    {
      "guid": "c5f0e4f3-6d70-890b-cdef-012345678901",
      "kind": "model",
      "path": "models/rock_01.gltf",
      "size": 8192000,
      "hash": "d7c6b5a4e3d2c1b0a9e8d7c6b5a4e3d2c1b0a9e8d7c6b5a4e3d2c1b0a9e8d7c6",
      "modified": "2025-01-08T14:20:15Z",
      "dependencies": ["b4e9d3e2-5c6f-789a-bcde-f01234567890"]
    }
  ]
}
```

---

### Field Descriptions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `assets[]` | Array | Yes | All assets in database |
| `guid` | String | Yes | Unique asset ID (UUID v4) |
| `kind` | String | Yes | Asset type: "texture", "model", "audio", "script", "material", "level" |
| `path` | String | Yes | Relative path from `assets/` directory |
| `size` | u64 | Yes | File size in bytes |
| `hash` | String | Yes | SHA-256 hash (hex string) |
| `modified` | String | Yes | Last modified timestamp (ISO 8601) |
| `dependencies[]` | Array | No | GUIDs of dependent assets |

---

### Validation Rules

1. **GUIDs**: Unique across all assets, valid UUID v4 format
2. **kind**: Must be one of valid asset types
3. **path**: Must exist on disk relative to `assets/`
4. **size**: Must match actual file size
5. **hash**: Must match SHA-256 hash of file contents
6. **modified**: Must be valid ISO 8601 timestamp
7. **dependencies**: All referenced GUIDs must exist in database

---

## 9. Validation Rules

### Cross-Format Rules

1. **Determinism**: All formats must produce identical output for identical input
2. **Pretty-Printing**: JSON uses 2-space indent, TOML uses standard formatting
3. **Git Compatibility**: No binary data, no platform-specific line endings
4. **Version Control**: Include schema version in all formats (future)
5. **Hot Reload**: Save triggers `reload.signal` file creation

---

### Common Validation Errors

| Error | Cause | Solution |
|-------|-------|----------|
| "Invalid TOML syntax" | Missing quotes, brackets | Fix syntax, validate with TOML parser |
| "next_id references non-existent node" | Broken dialogue link | Fix next_id or add missing node |
| "Cyclic reference in quest steps" | Infinite loop in step chain | Fix step.next references |
| "Biome ID not found" | Invalid biome ID | Use valid biome from registry |
| "Obstacle ID not in asset database" | Missing asset | Add asset or fix ID |
| "Roughness out of range" | Value < 0.04 or > 1.0 | Clamp to [0.04, 1.0] |
| "Terrain grid not 10×10" | Wrong dimensions | Resize to exactly 10×10 |
| "Navmesh triangle count = 0" | No obstacles to process | Add obstacles to level |

---

## 10. Git Integration

### Recommended Workflow

1. **Edit in aw_editor**: Make changes to level/dialogue/quests
2. **Save**: Click Save or Save JSON
3. **Review Changes**:
   ```powershell
   git diff assets/
   git diff content/levels/
   ```
4. **Stage Changes**:
   ```powershell
   git add content/levels/*.json
   git add assets/terrain_grid.json
   git add assets/material_live.json
   ```
5. **Commit**:
   ```powershell
   git commit -m "feat: Add forest_breach level with wolf encounter"
   ```
6. **Push**:
   ```powershell
   git push origin feature/forest-breach
   ```

---

### Merge Conflict Resolution

**JSON Conflicts**:
- Use JSON merge tools (e.g., `jq`, `json-merge`)
- Prefer base + ours strategy for dialogue/quests
- Manual review for level conflicts

**TOML Conflicts**:
- Use TOML merge tools (e.g., `toml-merge`)
- Prefer manual review for complex conflicts

**Terrain Grid Conflicts**:
- Terrain grid is a 2D array - merge by regions
- Use diff tools to visualize changes
- Accept ours/theirs by region if possible

---

### Diff-Friendly Practices

1. **Consistent Formatting**: Always use pretty-print (2-space indent)
2. **Sorted Keys**: Sort object keys alphabetically (optional)
3. **One Element Per Line**: Array elements on separate lines
4. **No Trailing Whitespace**: Remove trailing spaces
5. **Unix Line Endings**: Use LF, not CRLF
6. **Atomic Commits**: One feature per commit
7. **Descriptive Messages**: Use conventional commits format

---

## Appendix A: File Extension Conventions

| Extension | Format | Purpose |
|-----------|--------|---------|
| `.level.toml` | TOML | Level data (primary) |
| `.level.json` | JSON | Level data (alternative) |
| `.bt.toml` | TOML | Behavior tree |
| `.dialogue.json` | JSON | Dialogue tree |
| `.quest.toml` | TOML | Quest |
| `.mat.json` | JSON | Material |
| `.nav.json` | JSON | Navmesh metadata |
| `.rhai` | Rhai | Script |

---

## Appendix B: Editor Integration

### Hot Reload Flow

1. **Edit** → Save in `aw_editor`
2. **Write** → File saved to disk + `reload.signal` created
3. **Watch** → File watcher detects change (notify crate)
4. **Reload** → Engine reloads asset from disk
5. **Validate** → Engine re-validates schema
6. **Apply** → Changes visible in game

**Status**: Partial (save + signal implemented, watcher pending)

---

### Save Format Selection

- **TOML**: Preferred for levels (human-readable, comments supported)
- **JSON**: Preferred for dialogue (standard, tooling support)
- **Both**: Editor supports both, engine loads both

---

## Appendix C: Future Extensions

1. **Schema Versioning**: Add `schema_version` field to all formats
2. **Migration Tools**: Automatic schema upgrades
3. **Validation CLI**: `aw_validate_schema <file>`
4. **JSON Schema**: Formal JSON Schema definitions
5. **LSP Support**: Language server for syntax/validation in external editors
6. **Diff Visualizer**: GUI tool for comparing level versions

---

**Document Version**: 1.0.0  
**Last Updated**: January 2025  
**Maintainer**: AstraWeave Core Team  
**Status**: ✅ Production Ready
