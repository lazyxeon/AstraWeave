# AstraWeave Level & Encounter Editor

A non-programmer friendly GUI tool for creating and editing levels, encounters, and boss fights in AstraWeave.

## Features

- Visual editing of game levels with biomes, obstacles, and NPCs
- Fate thread system for creating trigger-based events
- Boss encounter configuration with Rhai scripting
- Hot-reload support for instant feedback

## Linux file dialogs

Open and save dialogs use `rfd` 0.17. On Linux and the BSDs that backend talks to the XDG desktop portal through `libdbus`. If `libdbus` is not installed, `rfd` falls back to `zenity`. A desktop that can show a portal dialog (GTK, GNOME, or KDE portal backend) or the `zenity` package is required at runtime. The editor does not link GTK3, so `libgtk-3-dev` is not a build dependency.

AccessKit is off. Screen readers do not receive the egui tree. `accesskit_unix` still depends on zbus 5, which is RUSTSEC-2026-0370, 0371, and 0372.

## Usage

```bash
cargo run -p aw_editor
```

## File Format

The editor saves levels in a human-readable TOML format:

```toml
title = "Forest Breach"
biome = "temperate_forest"
seed = 123456

[sky]
time_of_day = "dawn"
weather = "fog_light"

# Biome paints
[[biome_paints]]
kind = "grass_dense"
area = { cx = 0, cz = 0, radius = 64 }

# Obstacles
[[obstacles]]
id = "rock_big_01"
pos = [12.0, 0.0, -8.0]
yaw = 1.57
tags = ["cover", "climbable"]

# NPCs
[[npcs]]
archetype = "wolf_pack"
count = 3
spawn = { pos = [-15.0, 0.0, 12.0], radius = 3.0 }
behavior = "patrol"

# Fate threads (triggers → director ops)
[[fate_threads]]
name = "opening_ambush"
triggers = [{ kind = "enter_area", center = [0.0, 0.0, 0.0], radius = 6.0 }]
ops = [
  { op = "Fortify", area = { cx = 8, cz = -6, r = 6 } },
  { op = "SpawnWave", archetype = "wolf_pack", count = 2, scatter = 2.5 }
]

# Boss encounter
[boss]
director_budget_script = "content/encounters/forest_breach.budget.rhai"
phase_script = "content/encounters/forest_breach.phases.rhai"
```

## Integration

The editor creates a `reload.signal` file when saving, which the engine can watch to automatically reload content.

Rhai scripts for boss encounters are stored in the `content/encounters/` directory.

## Debug Menu

The editor includes a Debug menu (`🐛 Debug`) for testing engine features:

### Viewport Tests
- **Load Test Model (barrels.glb)**: Loads a sample 3D model to verify the PBR rendering pipeline
- **Toggle Engine Rendering**: Switch between the full engine renderer and fallback cube renderer

### Diagnostics
- **Show Engine Info**: Display current engine status (rendering mode, adapter initialization)
- **Clear Console**: Clear all console log messages

## 3D Viewport

The viewport supports full PBR (Physically-Based Rendering) through integration with `astraweave-render`:

### Features
- Orbit camera (right-click drag to rotate, scroll to zoom)
- Grid rendering with snap support
- Gizmo transform tools (translate, rotate, scale)
- glTF model loading with automatic material import
- Entity selection and multi-selection

### Keyboard Shortcuts
- **W**: Switch to Translate gizmo
- **E**: Switch to Rotate gizmo  
- **R**: Switch to Scale gizmo
- **F**: Frame selected entity
- **G**: Toggle grid visibility
- **F1-F12**: Camera bookmark recall (Ctrl+F1-F12 to save)