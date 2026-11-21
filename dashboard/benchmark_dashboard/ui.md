---
layout: default
title: UI Subsystem
---

# User Interface (astraweave-ui)

AstraWeave's UI system is built on **egui 0.32** with wgpu rendering integration, providing in-game menus, HUD elements, dialogue trees, and accessibility features. The crate has **331 tests**.

## Modules

| Module | Purpose |
|--------|---------|
| `menu` / `menus` | Main menu, pause menu, settings panels |
| `hud` | Health bars, damage numbers, quest tracker, minimap, dialogue |
| `panels` | Extended panel system |
| `gamepad` | Controller navigation via gilrs |
| `accessibility` | Colorblind modes, high contrast |
| `persistence` | TOML-based settings save/load |
| `state` | UI state machine management |
| `layer` | UI layering and draw order |

## Menus

### Main Menu
- New Game, Continue, Settings, Quit
- Responsive layout with egui

### Pause Menu
- Resume, Settings, Quit to Main Menu
- Non-blocking overlay on game view

### Settings Panels

| Panel | Contents |
|-------|----------|
| **Graphics** | Quality preset (`QualityPreset`), resolution, fullscreen |
| **Audio** | Per-bus volume sliders (Master, Music, SFX, Ambient, Voice) |
| **Controls** | Key binding display, click-to-rebind, mouse sensitivity |

Settings persist to TOML via `save_settings()` / `load_settings()`.

Types: `AudioSettings`, `ControlsSettings`, `GraphicsSettings`, `QualityPreset`, `SettingsState`.

## HUD Elements

### Health Bars
- Smooth value transitions with easing
- Flash effect on damage/heal events
- Glow overlay for critical states
- Spring-physics smoothing (critically damped)

### Damage Numbers
- Arc motion (parabolic trajectories)
- Combo counter for consecutive hits
- Screen shake on heavy impacts
- Color-coded by `DamageType`

### Quest Tracker
- Active quest list with `Quest` and `Objective` types
- Checkmark animations on objective completion
- Banner notifications for quest milestones

### Minimap
- World-space POI markers with `PoiMarker` and `PoiType`
- Player position indicator
- Enemy faction indicators via `EnemyFaction`

### FPS Counter / Debug Overlay
- F3 toggle for debug information
- Frame time, entity count, system timing

## Dialogue System

Branching NPC dialogue built on tree structures:

```rust
pub struct DialogueNode {
    // Text content, speaker, choices
}

pub struct DialogueChoice {
    // Player response option
}
```

Features:
- Multi-node dialogue trees with branching paths
- Player response selection
- Tooltip system with hover detection (`TooltipData`)
- Integration with audio engine for voice playback

## Accessibility

```rust
pub struct AccessibilitySettings {
    pub colorblind_mode: ColorblindMode,
    pub high_contrast: bool,
    // ...
}

pub enum ColorblindMode {
    None,
    Protanopia,
    Deuteranopia,
    Tritanopia,
}
```

Functions: `transform_color()`, `get_health_colors()`, `to_egui_color()`.

All UI elements respect the active colorblind mode through the `colors` namespace.

## Gamepad Support

```rust
pub struct GamepadManager {
    // gilrs-based controller state
}

pub enum GamepadAction {
    // Mapped controller inputs
}

pub struct GamepadBindings {
    // Button → action mapping
}
```

Full controller navigation: menu traversal, HUD interaction, and in-game controls via **gilrs 0.11** (UI override) / **gilrs 0.10** (workspace default).

## Architecture

```
egui::Context
    │
    ├── MenuManager ─── MainMenu / PauseMenu / SettingsMenu
    │                   └── save_settings / load_settings (TOML)
    │
    ├── HudManager ──── HealthBar / DamageNumbers / QuestTracker / Minimap
    │                   └── PlayerStats / EnemyData / Quest / Objective
    │
    ├── DialogueManager ── DialogueNode / DialogueChoice / TooltipData
    │
    ├── AccessibilitySettings ── ColorblindMode / HighContrast
    │
    └── GamepadManager ── GamepadBindings / GamepadAction
```

Entry point: `draw_ui(ctx, ui_data, ui_flags)` — renders all active UI layers.

## Animation Framework

Health bars, damage numbers, and notifications use a shared animation system:

| Effect | Implementation |
|--------|----------------|
| Easing functions | Linear, EaseInOut, Bounce, Elastic |
| Spring physics | Critically-damped spring for smooth transitions |
| Flash/glow | Color overlay on damage/heal events |
| Arc motion | Parabolic trajectories for floating damage numbers |

## Test Coverage

| Category | Tests |
|----------|-------|
| Core HUD logic | 25+ |
| State management | 20+ |
| Menu systems | 15+ |
| Accessibility | 10+ |
| Edge cases | 9+ |
| Mutation tests | 50+ |
| **Total** | **331** |

[← Back to Home](index.html) · [Architecture](architecture.html) · [Audio](audio.html)
