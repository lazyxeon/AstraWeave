# Phase 4 Implementation Plan: Authoring Tools & Workflow Integration

**Date**: October 1, 2025  
**Phase**: Phase 4 — Authoring Tools & Workflow  
**Status**: 🚧 **IN PROGRESS** (0% → Target 100%)  
**Prerequisites**: ✅ Phase 1-3 Complete (ECS, Rendering, AI/Gameplay)

---

## Executive Summary

Phase 4 transforms the existing editor stub into a production-ready authoring environment with:
- Multi-dock UI with proper ECS integration
- Graph editors for BT/Dialogue/Quests with validation
- Live material/shader editing with hot reload
- Terrain/biome painting with deterministic output
- Navmesh baking integration
- Simulation playback with deterministic replay
- Collaborative-friendly saves with Git diffs

**Current State**: Editor exists with basic panels (single file, ~800 lines)  
**Target State**: Modular, feature-flagged, fully tested authoring suite

---

## Architecture Overview

### Module Structure
```
tools/aw_editor/
├── Cargo.toml              # Feature flags + dependencies
├── src/
│   ├── main.rs             # Entry point + app shell
│   ├── app.rs              # EditorApp with docking
│   ├── command_palette.rs  # Ctrl+P quick actions
│   ├── panels/
│   │   ├── mod.rs
│   │   ├── hierarchy.rs    # Scene/ECS hierarchy tree
│   │   ├── inspector.rs    # Component property editors
│   │   ├── console.rs      # Log viewer with filtering
│   │   └── profiler.rs     # Performance metrics display
│   ├── graphs/
│   │   ├── mod.rs
│   │   ├── bt/             # Behavior Tree editor
│   │   │   ├── mod.rs
│   │   │   ├── loader.rs   # TOML/JSON loading
│   │   │   ├── editor.rs   # Visual node editor
│   │   │   └── validate.rs # Cycle detection, orphans
│   │   ├── dialogue/       # Dialogue editor
│   │   │   ├── mod.rs
│   │   │   ├── loader.rs
│   │   │   ├── editor.rs
│   │   │   └── validate.rs
│   │   └── quests/         # Quest editor
│   │       ├── mod.rs
│   │       ├── loader.rs
│   │       ├── editor.rs
│   │       └── validate.rs
│   ├── materials/
│   │   ├── mod.rs
│   │   ├── editor.rs       # MaterialManager integration
│   │   └── hot_reload.rs   # File watcher + pipeline rebuild
│   ├── terrain/
│   │   ├── mod.rs
│   │   ├── painter.rs      # Biome brush UI
│   │   └── io.rs           # Deterministic JSON save/load
│   ├── nav/
│   │   ├── mod.rs
│   │   ├── bake.rs         # NavMesh generation
│   │   ├── overlay.rs      # Visual debug overlay
│   │   └── io.rs           # Navmesh metadata persistence
│   └── sim/
│       ├── mod.rs
│       ├── runner.rs       # ECS world tick loop
│       └── controls.rs     # Play/Pause/Step UI
│
tools/aw_editor_cli/        # CLI diff tool (optional)
└── src/
    └── main.rs             # Semantic diff for JSON/TOML
```

---

## Feature Flags

### Cargo.toml Features
```toml
[features]
default = ["editor-core"]
editor-core = []  # Base panels (Hierarchy, Inspector, Console, Profiler)
editor-graphs = ["editor-core"]  # BT/Dialogue/Quests editors
editor-materials = ["editor-core", "astraweave-render/materials"]  # Live materials
editor-terrain = ["editor-core"]  # Terrain/biome painter
editor-nav = ["editor-core", "astraweave-nav"]  # Navmesh baking
editor-sim = ["editor-core", "astraweave-ecs"]  # Simulation playback
editor-full = ["editor-graphs", "editor-materials", "editor-terrain", "editor-nav", "editor-sim"]
```

---

## Data Schemas

### 1. Behavior Tree (BT)
**File**: `content/behaviors/{name}.bt.toml`
```toml
[tree]
id = "patrol_guard"
root = "seq_main"

[[nodes]]
id = "seq_main"
type = "sequence"
children = ["check_alert", "action_patrol"]

[[nodes]]
id = "check_alert"
type = "condition"
script = "ai.threat_level > 0.5"

[[nodes]]
id = "action_patrol"
type = "action"
action = "move_to_waypoint"
params = { waypoint_set = "guard_route" }
```

**Validation**:
- No cycles in node references
- All child IDs exist
- Root node exists
- Script syntax valid (Rhai check)

---

### 2. Dialogue Graph
**File**: `content/dialogues/{name}.dialogue.json`
```json
{
  "id": "elder_intro",
  "nodes": [
    {
      "id": "start",
      "text": "Welcome, traveler. What brings you here?",
      "responses": [
        {
          "text": "I seek the ancient relic.",
          "next_id": "quest_offer",
          "conditions": ["has_item:map"]
        },
        {
          "text": "Just passing through.",
          "next_id": "farewell"
        }
      ]
    },
    {
      "id": "quest_offer",
      "text": "Ah, the Starfall Amulet! I can guide you.",
      "responses": [
        {
          "text": "Please, tell me more.",
          "next_id": "quest_details"
        }
      ]
    }
  ]
}
```

**Validation**:
- All `next_id` references exist
- No unreachable nodes (orphans)
- Conditions reference valid game state keys
- Deterministic ordering (sorted by id/index)

---

### 3. Quest Graph
**File**: `content/quests/{name}.quest.toml`
```toml
[quest]
id = "find_amulet"
title = "The Starfall Amulet"

[[steps]]
id = "step_1"
description = "Find the cave entrance"
completed = false
next = "step_2"

[[steps]]
id = "step_2"
description = "Retrieve the amulet"
completed = false
next = null  # Quest complete
```

**Validation**:
- All `next` references exist or are null
- No cyclic step chains
- Each step has unique ID

---

### 4. Material (Live Edit)
**File**: `assets/materials/{biome}/material_live.json`
```json
{
  "base_color": [0.8, 0.7, 0.6, 1.0],
  "metallic": 0.1,
  "roughness": 0.7,
  "albedo_index": 0,
  "normal_index": 1,
  "mra_index": 2,
  "tiling": [2.0, 2.0],
  "emissive": [0.0, 0.0, 0.0]
}
```

**Hot Reload Flow**:
1. User edits sliders in Material Editor Panel
2. Clicks "Save & Reload" (or Shift+R)
3. JSON written with sorted keys
4. File watcher detects change
5. MaterialManager reloads → GPU pipeline rebuilt
6. Console logs success/errors

---

### 5. Terrain Biome Paints
**File**: `content/levels/{level}_biome_paints.json`
```json
{
  "grid_size": [10, 10],
  "cell_size": 10.0,
  "paints": [
    { "x": 0, "z": 0, "biome": "grass" },
    { "x": 1, "z": 0, "biome": "forest" },
    { "x": 5, "z": 5, "biome": "mountain" }
  ]
}
```

**Determinism**:
- Sorted by (x, z) coordinates
- Compact float representation (1 decimal place for positions)
- No timestamps or UUIDs

---

### 6. Navmesh Metadata
**File**: `content/navmeshes/{level}.nav.json`
```json
{
  "agent_radius": 0.4,
  "max_step": 0.4,
  "max_slope_deg": 60.0,
  "voxel_size": 0.3,
  "triangle_count": 256,
  "bake_timestamp": null,
  "asset_hash": "sha256:abc123..."
}
```

**Note**: `bake_timestamp` omitted for determinism; use `asset_hash` for versioning

---

## Implementation Tasks

### Task 1: Editor Shell & Panels (Core)
**Files**: `app.rs`, `command_palette.rs`, `panels/{hierarchy,inspector,console,profiler}.rs`

**Hierarchy Panel**:
- Tree view of ECS entities (from `World::entities()`)
- Show entity ID + archetype name
- Click to select → populate Inspector
- Right-click context menu: Delete, Duplicate, Add Component

**Inspector Panel**:
- Load selected entity's components
- Typed editors:
  - `Position`: X/Y drag values
  - `Health`: HP/MaxHP sliders
  - `Team`: ID combo box
  - `MaterialRef`: Dropdown of available materials
  - `AIController`: Mode selector (Rule/BT/GOAP)
- Save changes → ECS component updates

**Console Panel**:
- Subscribe to `astraweave_observability` tracing
- Filter by level: Debug/Info/Warn/Error
- Search/filter by tag
- Clear button, auto-scroll toggle

**Profiler Panel**:
- Per-frame timings:
  - Renderer: ms/frame
  - AI planning: ticks/second
  - Physics: collision checks
- Counters:
  - Entities alive
  - Active AI agents
  - GPU memory usage
- Mini-graph (last 60 frames)

**Command Palette** (Ctrl/Cmd+P):
- Quick actions:
  - New Scene
  - Save/Load
  - Bake Navmesh
  - Paint Terrain
  - Toggle Simulation
  - Validate All Graphs

**Tests**:
```rust
#[test]
fn test_hierarchy_displays_entities() {
    let mut world = World::new();
    world.spawn("test", IVec2::ZERO, Team { id: 1 }, 100, 0);
    // Assert hierarchy shows 1 entity
}

#[test]
fn test_inspector_edits_position() {
    // Simulate UI input → assert component updated
}
```

---

### Task 2: Graph Editors (Feature: editor-graphs)
**Files**: `graphs/{bt,dialogue,quests}/{loader,editor,validate}.rs`

**BT Editor**:
- Load `.bt.toml` → parse into tree structure
- Visual editor: collapsible tree or node-link diagram
- Add node: Sequence, Selector, Condition, Action, Decorator
- Validation:
  - Detect cycles (DFS)
  - Check child ID existence
  - Rhai script syntax check (if present)
- Save: TOML with sorted keys

**Dialogue Editor**:
- Load `.dialogue.json` → node array
- Visual editor: list or flow diagram
- Add node, edit text/responses
- Validation:
  - All `next_id` exist
  - No orphans (unreachable nodes)
  - Condition keys valid
- Save: JSON with sorted node IDs

**Quest Editor**:
- Load `.quest.toml` → step array
- Visual editor: linear step list
- Edit description, mark completed
- Validation:
  - No cyclic `next` references
  - Unique step IDs
- Save: TOML with sorted steps

**Tests**:
```rust
#[test]
fn test_bt_detects_cycle() {
    let toml = r#"[[nodes]]
id = "a"
children = ["b"]
[[nodes]]
id = "b"
children = ["a"]"#;
    assert!(validate_bt(toml).is_err());
}

#[test]
fn test_dialogue_orphan_detection() {
    // Node with no incoming edges → warn
}
```

---

### Task 3: Live Materials/Shader Editing (Feature: editor-materials)
**Files**: `materials/{editor,hot_reload}.rs`

**Material Editor**:
- Bind to `MaterialManager` from `astraweave-render`
- UI: sliders for base_color, metallic, roughness, tiling
- Texture dropdowns: select from loaded array indices
- Save button → write JSON to `assets/materials/{biome}/material_live.json`
- Keyboard shortcut: Shift+R to reload

**Hot Reload**:
- `notify` crate file watcher on `assets/materials/**/*.json`
- On change:
  - Parse JSON
  - Call `MaterialManager::reload_material(path)`
  - If WGSL file changed: rebuild shader pipeline
  - Log to Console Panel (success/error)

**Shader Hot Reload**:
- Watch `assets/shaders/**/*.wgsl`
- On change:
  - Recompile shader
  - Rebuild pipeline
  - Log errors with line numbers to Console

**Tests**:
```rust
#[test]
fn test_material_round_trip() {
    let mat = MaterialLiveDoc { /* ... */ };
    let json = serde_json::to_string_pretty(&mat)?;
    let parsed: MaterialLiveDoc = serde_json::from_str(&json)?;
    assert_eq!(mat, parsed);
}

#[test]
fn test_hot_reload_triggers() {
    // Mock file watcher → assert reload called
}
```

---

### Task 4: Terrain/Biome Painting (Feature: editor-terrain)
**Files**: `terrain/{painter,io}.rs`

**Terrain Painter**:
- Grid UI: 10×10 cells (or configurable)
- Biome brush: select from [grass, forest, mountain, water, desert]
- Click cell → paint biome
- Radius slider: paint multiple cells
- Strength slider: blend biomes (optional)

**I/O**:
- Save: `{level}_biome_paints.json`
- Format: sorted by (x, z)
- Load: populate grid from JSON
- Sync with Level: convert grid → `level.biome_paints` array

**Determinism**:
- No timestamps
- Sorted coordinates
- Compact floats (1 decimal)

**Tests**:
```rust
#[test]
fn test_terrain_save_load_idempotent() {
    let grid = vec![vec!["grass".into(); 10]; 10];
    save_terrain(&grid, "test.json")?;
    let loaded = load_terrain("test.json")?;
    assert_eq!(grid, loaded);
}

#[test]
fn test_terrain_json_sorted() {
    // Assert keys sorted, no timestamps
}
```

---

### Task 5: Navmesh Baking (Feature: editor-nav)
**Files**: `nav/{bake,overlay,io}.rs`

**Bake Controls**:
- UI inputs: agent_radius, max_step, max_slope_deg, voxel_size
- "Bake" button:
  - Extract obstacles from level
  - Call `NavMesh::bake()` from `astraweave-nav`
  - Generate triangles
  - Save metadata to `{level}.nav.json`

**Overlay Visualization**:
- Render navmesh triangles in 3D viewport (optional)
- Toggle: show/hide navmesh
- Color code: walkable (green), blocked (red)

**Metadata Persistence**:
- Save: JSON with bake parameters + triangle count
- Hash: SHA-256 of triangle data for versioning
- Load: restore parameters, optionally rebuild

**Tests**:
```rust
#[test]
fn test_navmesh_bake_deterministic() {
    let obstacles = vec![/* ... */];
    let nav1 = bake_navmesh(&obstacles, 0.4, 60.0);
    let nav2 = bake_navmesh(&obstacles, 0.4, 60.0);
    assert_eq!(nav1.tris.len(), nav2.tris.len());
}

#[test]
fn test_navmesh_metadata_integrity() {
    // Save/load metadata → params unchanged
}
```

---

### Task 6: Simulation Playback (Feature: editor-sim)
**Files**: `sim/{runner,controls}.rs`

**Sim Runner**:
- Create ECS `World` from level assets
- Spawn entities (obstacles, NPCs)
- Fixed timestep loop (60Hz)
- Seed input: deterministic RNG
- Tick logic: AI planning, physics, events

**Controls Panel**:
- Play/Pause button
- Step (single tick) button
- Reset button (reload level)
- Seed input field
- Speed slider (1x, 2x, 4x)
- Tick counter display
- Entity count display

**Determinism Check**:
- After N ticks with seed S:
  - Hash world state (positions, health, cooldowns)
  - Compare against golden baseline
  - Log diff to Console if mismatch

**Tests**:
```rust
#[test]
fn test_sim_deterministic_replay() {
    let world1 = run_sim(seed=42, ticks=100);
    let world2 = run_sim(seed=42, ticks=100);
    assert_eq!(hash_world(&world1), hash_world(&world2));
}

#[test]
fn test_sim_spawns_from_level() {
    let level = load_level("test.level.toml")?;
    let world = create_world_from_level(&level);
    assert_eq!(world.entities().len(), level.npcs.iter().map(|n| n.count).sum());
}
```

---

### Task 7: Collaborative Saves & Diffs
**Files**: `tools/aw_editor_cli/src/main.rs` (optional), `io.rs` helpers

**Deterministic JSON/TOML**:
- Sort all object keys
- Compact float representation (avoid scientific notation)
- No timestamps (use hashes for versioning)
- No UUIDs unless necessary (prefer sequential IDs)

**CLI Diff Tool** (Optional):
```bash
aw_editor_cli diff file_a.json file_b.json
```
- Parse both files
- Normalize (sort keys, compact floats)
- Semantic diff (ignore whitespace, key order)
- Output: colorized terminal diff

**Git Integration**:
- Pre-commit hook example in docs
- Reformat assets on commit (normalize JSON/TOML)
- `.gitattributes` for LFS (binary assets)

**Tests**:
```rust
#[test]
fn test_json_deterministic_output() {
    let doc = LevelDoc { /* ... */ };
    let json1 = to_stable_json(&doc)?;
    let json2 = to_stable_json(&doc)?;
    assert_eq!(json1, json2);
}

#[test]
fn test_cli_diff_stable() {
    // Same semantic content, different key order → no diff
}
```

---

## Testing Strategy

### Unit Tests (Per Module)
- Graph loaders: invalid syntax, orphans, cycles
- Material I/O: round-trip stability
- Terrain I/O: determinism, sorted output
- Navmesh: metadata integrity
- Sim: entity spawning, tick logic

**Command**: `cargo test -p aw_editor --lib`

---

### Integration Tests (Headless)
**File**: `tests/editor_smoke.rs`
```rust
#[test]
fn test_editor_init_no_panic() {
    // Create EditorApp in headless mode
    // Open/close panels
    // No GPU operations (mock renderer)
    // Assert no panic
}

#[test]
fn test_hot_reload_integration() {
    // Write material JSON
    // Trigger file watcher
    // Assert MaterialManager reload called
}

#[test]
fn test_sim_determinism_integration() {
    // Run sim for 100 ticks, seed=42
    // Hash world state
    // Compare against golden hash
}
```

**Command**: `cargo test -p aw_editor --tests`

---

### Golden Tests (Optional)
**File**: `tests/golden/dialogue_example.json`
- Small JSON/TOML baselines after simple edits
- Text-based comparison (not images)
- Updated when schema changes (intentional)

---

## Acceptance Criteria

- [ ] **Editor shell** with multi-dock panels (Hierarchy, Inspector, Console, Profiler)
- [ ] **Graph editors** (BT, Dialogue, Quests) with load/save + validation + basic positioning
- [ ] **Live Material/Shader editing** with hot reload via MaterialManager + WGSL watcher
- [ ] **Terrain/biome painting** (save deterministic JSON) and Navmesh bake + overlay + metadata save
- [ ] **Simulation playback** panel (Play/Pause/Step/Reset, fixed timestep, seed) with deterministic state after N steps
- [ ] **Collaborative saves** & CLI diff tool (stable output)
- [ ] **Tests**: unit + headless smoke + determinism checks passing
- [ ] **CI green**: cargo fmt, clippy -D warnings, all tests pass
- [ ] **Docs updated**: implementation plan, status report, progress report, authoring schemas

---

## Documentation Deliverables

### 1. PHASE4_IMPLEMENTATION_PLAN.md (This Document)
- Architecture, schemas, tasks, tests

### 2. PHASE4_STATUS_REPORT.md
- Task checklist with % complete
- Test results
- Blockers/risks

### 3. PHASE4_PROGRESS_REPORT.md
- How to run editor
- Feature flags
- Controls/shortcuts

### 4. tools/aw_editor/README.md
- User guide: panels, controls, file outputs
- Hot reload keys (Shift+R)
- Troubleshooting

### 5. docs/authoring_schemas.md
- Full schema reference
- Examples for each file type
- Validation rules

---

## Timeline Estimate

| Task | Estimated Time | Priority |
|------|---------------|----------|
| Editor shell + panels | 2-3 days | HIGH |
| Graph editors | 3-4 days | HIGH |
| Live materials | 2 days | MEDIUM |
| Terrain painter | 1-2 days | MEDIUM |
| Navmesh baking | 1-2 days | MEDIUM |
| Simulation playback | 2-3 days | HIGH |
| Collaborative saves | 1 day | LOW |
| Tests + CI | 2-3 days | HIGH |
| Documentation | 1-2 days | HIGH |
| **Total** | **15-22 days** | |

---

## Commands Reference

```bash
# Development
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo check --all-targets

# Tests
cargo test -p aw_editor --lib       # Unit tests
cargo test -p aw_editor --tests     # Integration tests

# Run editor (with features)
cargo run -p aw_editor --features editor-full
cargo run -p aw_editor --features editor-graphs,editor-materials

# CLI diff tool (optional)
cargo run -p aw_editor_cli -- diff file_a.json file_b.json
```

---

**Plan Created**: October 1, 2025  
**Phase 4 Target**: 100% complete with all acceptance criteria met  
**Next Step**: Begin Task 1 (Editor Shell & Panels)
