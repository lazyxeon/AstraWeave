# Phase 8 Priority 1: In-Game UI Framework

**Document Version**: 1.0  
**Date**: October 14, 2025  
**Status**: Ready to Implement  
**Timeline**: 4-5 weeks  
**Priority**: 🥇 CRITICAL - Blocks Veilweaver Playability

---

## Executive Summary

**Mission**: Build in-game UI system to enable playable Veilweaver demo

**Why This First**:
- ✅ **Veilweaver needs menus RIGHT NOW** - Can't test gameplay without UI
- ✅ **Blocks everything else** - Can't show health bars, objectives, settings without UI framework
- ✅ **High visibility** - Players interact with UI before anything else
- ✅ **Foundation for Phase 8** - HUD, save/load dialogs, audio settings all need UI

**Current State**:
- ✅ Editor UI (egui 0.28) - Works great for authoring tools
- ❌ No in-game UI system
- ❌ No main menu or pause menu
- ❌ No HUD rendering
- ❌ No settings UI
- ❌ No dialog boxes

**Gap**: Can compile a game but can't make it playable for end users

---

## Goals & Success Criteria

### Primary Goal

**Enable Veilweaver to be played from main menu through 5-minute demo level**

### Success Criteria

**Week 1-2 (Core UI Framework)**:
- ✅ Main menu with "New Game", "Load", "Settings", "Quit" buttons
- ✅ Pause menu (ESC key toggles, "Resume", "Settings", "Quit to Menu")
- ✅ Settings menu (volume sliders, resolution, keybind display)
- ✅ UI renders at correct resolution (1920x1080, 1280x720, 2560x1440)
- ✅ Mouse input works for all buttons
- ✅ Visual feedback (hover, click, disabled states)

**Week 3-4 (HUD System)**:
- ✅ Health bar (player HP 0-100%)
- ✅ Stamina bar (0-100%)
- ✅ Quest objectives overlay (top-right, 3-5 lines of text)
- ✅ Minimap (128x128 pixel top-down view, 50m radius)
- ✅ Dialogue subtitles (bottom-center, 2-3 lines, 3-second fade)
- ✅ HUD visible during gameplay, hidden in menus

**Week 5 (Polish)**:
- ✅ UI animations (fade in/out, slide transitions, button press)
- ✅ Controller support (D-pad navigation, A/B confirm/cancel)
- ✅ Accessibility (text scaling 80-150%, colorblind modes for health bars)
- ✅ Performance (UI rendering <0.5ms @ 60 FPS)

### Acceptance Test

**"Veilweaver Playability Test"**:
1. Launch game → Main menu appears
2. Click "New Game" → Gameplay starts with HUD visible
3. Press ESC → Pause menu appears, gameplay frozen
4. Click "Settings" → Volume sliders work, resolution changes
5. Click "Resume" → Gameplay continues
6. Walk 100m → Minimap updates, quest objective appears
7. Take damage → Health bar animates smoothly
8. Trigger dialogue → Subtitles appear and fade
9. Press ESC → Click "Quit to Menu" → Returns to main menu

**All steps must work smoothly without crashes or visual glitches**

---

## Architecture Overview

### Technology Choice: egui for In-Game UI

**Decision**: Use egui (same as editor) for in-game UI

**Rationale**:
- ✅ Already integrated (egui 0.28 in editor)
- ✅ Immediate-mode paradigm (simple state management)
- ✅ Fast prototyping (build menus in minutes)
- ✅ Cross-platform (wgpu backend works everywhere)
- ✅ Good performance (<0.5ms for typical UI)
- ✅ Mature ecosystem (egui_extras for plots, colors, etc.)

**Tradeoffs**:
- ⚠️ Less control than custom UI (but good enough for menus/HUD)
- ⚠️ Not as "game-like" as Unity UI or Unreal UMG (but acceptable for demo)
- ✅ Can replace later if needed (UI is isolated system)

**Alternatives Considered**:
- **Custom immediate-mode UI**: More work (2-3 weeks extra), full control
- **kayak_ui**: ECS-based UI, less mature than egui
- **iced**: Retained-mode, heavier, overkill for game UI

**Verdict**: egui is the pragmatic choice for Phase 8

---

## Crate Structure

### New Crate: `astraweave-ui`

**Purpose**: In-game UI rendering, menus, HUD, dialogs

**Dependencies**:
```toml
[dependencies]
astraweave-ecs = { path = "../astraweave-ecs" }
astraweave-render = { path = "../astraweave-render" }
egui = "0.28"
egui-wgpu = "0.28"
egui-winit = "0.28"
glam = "0.29"
anyhow = "1.0"
```

**Module Structure**:
```
astraweave-ui/
├── src/
│   ├── lib.rs                  # Public API, UiPlugin
│   ├── context.rs              # UiContext (egui + wgpu state)
│   ├── menus/
│   │   ├── mod.rs
│   │   ├── main_menu.rs        # Main menu screen
│   │   ├── pause_menu.rs       # Pause overlay
│   │   └── settings_menu.rs    # Settings screen
│   ├── hud/
│   │   ├── mod.rs
│   │   ├── health_bar.rs       # HP/stamina bars
│   │   ├── objectives.rs       # Quest text overlay
│   │   ├── minimap.rs          # Top-down view
│   │   └── subtitles.rs        # Dialogue text
│   ├── widgets/
│   │   ├── mod.rs
│   │   ├── button.rs           # Styled button
│   │   ├── slider.rs           # Volume slider
│   │   └── keybind_display.rs  # Key binding widget
│   ├── animations.rs           # Fade, slide, pulse effects
│   ├── accessibility.rs        # Text scaling, colorblind
│   └── input.rs                # Mouse + controller input
├── examples/
│   ├── main_menu_demo.rs       # Standalone menu test
│   └── hud_demo.rs             # HUD overlay test
└── Cargo.toml
```

---

## Week-by-Week Implementation Plan

### Week 1: Core UI Infrastructure (Oct 14-18)

**Goal**: Get egui rendering in-game with basic main menu

**Day 1-2: Setup & Integration**
- Create `astraweave-ui` crate
- Add egui-wgpu-winit integration
- Create `UiContext` struct (egui::Context + wgpu state)
- Integrate with `astraweave-render` pipeline
- Test: Render "Hello, UI!" text in game window

**Day 3-4: Main Menu Screen**
- Create `MainMenuScreen` struct
- Implement `render()` method with egui layout:
  ```rust
  egui::CentralPanel::default().show(ctx, |ui| {
      ui.vertical_centered(|ui| {
          ui.heading("AstraWeave");
          if ui.button("New Game").clicked() {
              // Transition to gameplay
          }
          if ui.button("Load Game").clicked() {
              // Open load menu
          }
          if ui.button("Settings").clicked() {
              // Open settings
          }
          if ui.button("Quit").clicked() {
              // Exit game
          }
      });
  });
  ```
- Add button styling (hover, click states)
- Wire up "New Game" → gameplay transition
- Test: Main menu appears on launch, "New Game" works

**Day 5: Pause Menu**
- Create `PauseMenuScreen` struct
- Add ESC key binding to toggle pause
- Implement pause overlay (semi-transparent background)
- Add "Resume", "Settings", "Quit to Menu" buttons
- Freeze gameplay when paused (skip ECS systems except UI)
- Test: ESC toggles pause, "Resume" works

**Deliverables**:
- ✅ Main menu with 4 functional buttons
- ✅ Pause menu with ESC toggle
- ✅ Basic egui styling
- ✅ Clean transition between screens

**Estimated Effort**: 40 hours (5 days @ 8hr/day)

---

### Week 2: Settings & Resolution (Oct 21-25)

**Goal**: Complete settings menu with volume, resolution, keybinds

**Day 1-2: Settings Menu Layout**
- Create `SettingsMenuScreen` struct
- Add tabs: "Audio", "Graphics", "Controls"
- Implement tab switching UI
- Add "Back" button to return to main menu

**Day 3: Audio Settings**
- Add volume sliders:
  - Master volume (0-100%)
  - Music volume (0-100%)
  - SFX volume (0-100%)
  - Voice volume (0-100%)
- Wire sliders to `astraweave-audio` mixer (requires audio mixer implementation)
- Placeholder: Store values in ECS resource for now

**Day 4: Graphics Settings**
- Add resolution dropdown:
  - 1280x720
  - 1920x1080
  - 2560x1440
  - 3840x2160
- Add fullscreen toggle
- Wire to window resize (winit API)
- Test: Resolution changes work, UI scales correctly

**Day 5: Controls Display**
- Create keybind display widget (non-editable for now):
  - Movement: WASD
  - Jump: Space
  - Pause: ESC
  - Interact: E
- Show current bindings from input system
- Note: Rebinding deferred to Phase 8.5 (not critical for demo)

**Deliverables**:
- ✅ Settings menu with 3 tabs
- ✅ Volume sliders (4 channels)
- ✅ Resolution dropdown + fullscreen toggle
- ✅ Keybind display (read-only)

**Estimated Effort**: 40 hours (5 days @ 8hr/day)

---

### Week 3: HUD System Foundation (Oct 28 - Nov 1)

**Goal**: Render HUD with health bar, stamina bar, objectives

**Day 1-2: HUD Context & Layout**
- Create `HudOverlay` struct
- Define HUD regions:
  - Top-left: Health/stamina bars (200x60 pixels)
  - Top-right: Quest objectives (300x150 pixels)
  - Top-right corner: Minimap (128x128 pixels)
  - Bottom-center: Subtitles (600x80 pixels)
- Implement `render()` with egui `Area` widgets (absolute positioning)
- Test: HUD appears during gameplay, hidden in menus

**Day 3: Health & Stamina Bars**
- Create `HealthBar` widget:
  - Render bar with fill percentage (0-100%)
  - Color gradient: Green (100%) → Yellow (50%) → Red (0%)
  - Optional: Damage flash effect (red blink on hit)
- Create `StaminaBar` widget (blue bar, similar to health)
- Wire to ECS components:
  ```rust
  struct Health { current: f32, max: f32 }
  struct Stamina { current: f32, max: f32 }
  ```
- Test: Bars update smoothly when values change

**Day 4: Quest Objectives Overlay**
- Create `ObjectivesOverlay` widget
- Render 3-5 lines of text:
  ```
  Quest: Find the Lost Artifact
  - Talk to Elder (0/1)
  - Search the ruins (0/3)
  - Return to village (0/1)
  ```
- Wire to ECS resource:
  ```rust
  struct ActiveQuest {
      title: String,
      objectives: Vec<(String, u32, u32)>, // (text, progress, total)
  }
  ```
- Test: Objectives appear, checkmarks update

**Day 5: HUD Integration Testing**
- Test HUD visibility during gameplay
- Verify HUD hidden in menus
- Check HUD at different resolutions (1280x720, 1920x1080, 2560x1440)
- Fix layout bugs (text overflow, bar scaling)

**Deliverables**:
- ✅ HUD overlay with 4 regions defined
- ✅ Health and stamina bars with color gradients
- ✅ Quest objectives overlay
- ✅ HUD toggles correctly (gameplay vs menus)

**Estimated Effort**: 40 hours (5 days @ 8hr/day)

---

### Week 4: Minimap & Subtitles (Nov 4-8)

**Goal**: Complete HUD with minimap and dialogue subtitles

**Day 1-2: Minimap Rendering**
- Create `Minimap` widget (128x128 pixel canvas)
- Render top-down view:
  - Player position (center, blue dot)
  - Nearby entities (white dots for NPCs, red for enemies)
  - Terrain outline (gray lines for walls/obstacles)
  - Quest markers (yellow stars)
- Use egui custom painting (egui::Painter + shapes)
- Wire to ECS queries:
  ```rust
  query: Query<(&Position, &EntityType)>
  // EntityType: Player, NPC, Enemy, Objective
  ```
- Test: Minimap updates in real-time, entities move

**Day 3: Minimap Zoom & Rotation**
- Add zoom levels (25m, 50m, 100m radius)
- Optional: Rotate minimap based on player facing (deferred if complex)
- Add border + background (semi-transparent black)
- Test: Zoom works, minimap readable

**Day 4-5: Dialogue Subtitles**
- Create `Subtitles` widget (bottom-center, 600x80 pixels)
- Render 2-3 lines of text:
  ```
  [Elder]: "The artifact lies deep within the ruins..."
  ```
- Add fade-in/fade-out animation (0.5s fade, 3s visible, 0.5s fade)
- Wire to ECS events:
  ```rust
  struct DialogueEvent {
      speaker: String,
      text: String,
      duration: f32, // seconds
  }
  ```
- Test: Subtitles appear on dialogue trigger, fade correctly

**Deliverables**:
- ✅ Minimap with player, entities, terrain
- ✅ Minimap zoom levels
- ✅ Dialogue subtitles with fade animation
- ✅ All HUD elements working together

**Estimated Effort**: 40 hours (5 days @ 8hr/day)

---

### Week 5: Polish & Accessibility (Nov 11-15)

**Goal**: Add animations, controller support, accessibility features

**Day 1-2: UI Animations**
- Implement fade-in/fade-out for menus:
  - Main menu fades in on launch (0.5s)
  - Pause menu fades in on ESC (0.3s)
  - Settings slide in from right (0.4s)
- Add button animations:
  - Hover: Scale 1.0 → 1.05 (0.1s)
  - Click: Scale 1.05 → 0.95 → 1.0 (0.2s)
  - Disabled: Grayscale + 50% opacity
- Use egui animation utilities (egui::Animate)
- Test: Animations smooth, no frame drops

**Day 3: Controller Support**
- Add gamepad input (gilrs crate):
  - D-pad: Navigate menu buttons
  - A button: Confirm
  - B button: Cancel/Back
  - Start button: Pause
- Highlight selected button (blue outline)
- Test with Xbox/PS5 controller
- Fallback: Keyboard navigation (Arrow keys, Enter, ESC)

**Day 4: Accessibility Features**
- Text scaling:
  - Add "UI Scale" slider in settings (80%, 100%, 125%, 150%)
  - Scale all text, buttons, HUD elements
- Colorblind modes:
  - Deuteranopia (red-green): Health bar green → blue
  - Protanopia (red-green): Health bar green → cyan
  - Tritanopia (blue-yellow): Health bar blue → magenta
- Add toggle in settings: "Colorblind Mode: None / Deuteranopia / Protanopia / Tritanopia"
- Test: Text readable at 150%, colorblind modes work

**Day 5: Performance Optimization & Bug Fixes**
- Profile UI rendering with Tracy:
  - Target: <0.5ms per frame
  - Optimize egui vertex buffers (batch draws)
  - Cache static UI elements
- Fix bugs from previous weeks:
  - Resolution scaling issues
  - Controller input lag
  - Animation glitches
- Final integration test: Run full "Veilweaver Playability Test"

**Deliverables**:
- ✅ UI animations (fade, slide, button effects)
- ✅ Full controller support (Xbox, PS5, keyboard)
- ✅ Accessibility (text scaling 80-150%, colorblind modes)
- ✅ Performance <0.5ms UI rendering
- ✅ All bugs fixed, ready for Veilweaver integration

**Estimated Effort**: 40 hours (5 days @ 8hr/day)

---

## Total Effort Estimate

| Week | Focus | Hours |
|------|-------|-------|
| Week 1 | Core UI Infrastructure (main menu, pause menu) | 40 |
| Week 2 | Settings Menu (audio, graphics, controls) | 40 |
| Week 3 | HUD Foundation (health, stamina, objectives) | 40 |
| Week 4 | Minimap & Subtitles | 40 |
| Week 5 | Polish (animations, controller, accessibility) | 40 |
| **TOTAL** | **5 weeks** | **200 hours** |

**Calendar**: Oct 14 - Nov 15 (5 weeks)

---

## Technical Specifications

### UiContext Structure

```rust
pub struct UiContext {
    /// egui context for rendering
    pub egui_ctx: egui::Context,
    
    /// egui-wgpu renderer
    pub egui_renderer: egui_wgpu::Renderer,
    
    /// egui-winit integration
    pub egui_winit: egui_winit::State,
    
    /// Current screen state
    pub current_screen: UiScreen,
    
    /// Pending screen transition (for fade effects)
    pub transition: Option<UiTransition>,
    
    /// UI settings (scale, colorblind mode)
    pub settings: UiSettings,
}

pub enum UiScreen {
    MainMenu,
    PauseMenu,
    SettingsMenu { tab: SettingsTab },
    InGame, // HUD visible
}

pub struct UiTransition {
    from: UiScreen,
    to: UiScreen,
    progress: f32, // 0.0 - 1.0
    duration: f32, // seconds
}

pub struct UiSettings {
    pub scale: f32, // 0.8 - 1.5
    pub colorblind_mode: ColorblindMode,
}

pub enum ColorblindMode {
    None,
    Deuteranopia,
    Protanopia,
    Tritanopia,
}
```

### HUD Component Specs

**Health Bar**:
- Position: Top-left, 20px margin
- Size: 200x20 pixels
- Fill: Left-to-right, health percentage
- Color: Green (>50%) → Yellow (25-50%) → Red (<25%)
- Border: 2px black outline
- Text: "HP: 75/100" centered

**Stamina Bar**:
- Position: Below health bar, 5px gap
- Size: 200x15 pixels
- Fill: Blue (100%) → Dark blue (0%)
- Border: 2px black outline
- Text: None (stamina is visual only)

**Quest Objectives**:
- Position: Top-right, 20px margin
- Size: 300x150 pixels max
- Font: 14px, white text with black shadow
- Lines: 3-5 objectives
- Format:
  ```
  Quest: [Title]
  ✓ Completed objective
  - Active objective (2/5)
  - Incomplete objective
  ```

**Minimap**:
- Position: Top-right corner, below objectives
- Size: 128x128 pixels
- Background: Semi-transparent black (80% opacity)
- Border: 2px white outline
- Player: Blue dot (center)
- NPCs: White dots
- Enemies: Red dots
- Objectives: Yellow stars
- Terrain: Gray lines

**Subtitles**:
- Position: Bottom-center, 100px from bottom
- Size: 600x80 pixels
- Font: 16px, white text with black background (90% opacity)
- Format:
  ```
  [Speaker Name]: "Dialogue text here..."
  ```
- Fade: 0.5s in, 3s visible, 0.5s out

---

## Integration with Existing Systems

### ECS Integration

**UI Components** (add to entities):
```rust
struct Health { current: f32, max: f32 }
struct Stamina { current: f32, max: f32 }
struct Position { x: f32, y: f32, z: f32 }
struct EntityType { kind: EntityKind } // Player, NPC, Enemy, Objective
```

**UI Resources** (global state):
```rust
struct ActiveQuest {
    title: String,
    objectives: Vec<Objective>,
}

struct Objective {
    text: String,
    progress: u32,
    total: u32,
    completed: bool,
}

struct DialogueQueue {
    events: VecDeque<DialogueEvent>,
}

struct DialogueEvent {
    speaker: String,
    text: String,
    duration: f32,
}

struct UiSettings {
    master_volume: f32,
    music_volume: f32,
    sfx_volume: f32,
    voice_volume: f32,
    resolution: (u32, u32),
    fullscreen: bool,
    ui_scale: f32,
    colorblind_mode: ColorblindMode,
}
```

### Rendering Pipeline Integration

**Current**: wgpu rendering (PBR, IBL, GPU skinning)  
**Add**: egui pass after 3D rendering

```rust
// In astraweave-render/src/renderer.rs
pub fn render_frame(&mut self) {
    // 1. Clear screen
    // 2. Render 3D scene (meshes, skinned models, terrain)
    // 3. Render UI (NEW - egui pass)
    self.ui_context.render(&mut self.encoder, &self.surface_view);
    // 4. Present frame
}
```

**egui-wgpu Integration**:
- Use `egui_wgpu::Renderer` for GPU rendering
- Render to same `SurfaceTexture` as 3D scene
- UI renders **after** 3D (on top)
- Depth test: Off (UI is 2D overlay)

### Audio Integration

**Current**: `astraweave-audio` has spatial audio, dialogue playback  
**Add**: Audio mixer system (Phase 8.4)

**For Week 2 Settings**:
- Store volume values in `UiSettings` resource
- Wire sliders to mixer (placeholder for now):
  ```rust
  // In settings_menu.rs
  if ui.add(egui::Slider::new(&mut settings.master_volume, 0.0..=1.0)).changed() {
      // TODO: Apply to audio mixer when implemented
      println!("Master volume: {}", settings.master_volume);
  }
  ```

**Phase 8.4 will implement actual audio mixer**

### Input Integration

**Current**: Input system handles WASD, mouse, keybinds  
**Add**: UI input layer (mouse clicks, gamepad)

**Priority**:
1. Mouse input (Week 1) - egui handles this automatically
2. Keyboard navigation (Week 5) - Arrow keys, Enter, ESC
3. Gamepad input (Week 5) - gilrs crate for Xbox/PS5 controllers

**Input Routing**:
```rust
// In UI system
if ui_context.current_screen != UiScreen::InGame {
    // UI is active - consume input
    egui_winit.handle_event(&event);
    return; // Don't pass to gameplay systems
}

// Gameplay is active - pass input to game systems
input_system.handle_event(&event);
```

---

## Dependencies & Prerequisites

### Crate Dependencies

**Required**:
- `egui = "0.28"` - Already used in editor (version match!)
- `egui-wgpu = "0.28"` - wgpu backend for egui
- `egui-winit = "0.28"` - winit integration for egui
- `glam = "0.29"` - Vector math (already in project)

**Optional (Week 5)**:
- `gilrs = "0.11"` - Gamepad input (Xbox, PS5)
- `egui_extras = "0.28"` - Extra widgets (color picker, plots)

### System Prerequisites

**None** - All dependencies already in project or simple to add

**Verification**:
```powershell
# Check egui version in editor
cargo tree -p aw_editor | Select-String "egui"
# Should show egui 0.28.x

# Verify wgpu compatibility
cargo tree -p astraweave-render | Select-String "wgpu"
# Should show wgpu 25.0.2 (compatible with egui-wgpu 0.28)
```

---

## Risk Assessment & Mitigation

### High Risk

**1. egui Performance at High Resolution**
- **Risk**: UI rendering >0.5ms at 4K resolution
- **Mitigation**:
  - Cache static UI elements (main menu buttons don't change)
  - Batch egui draw calls (egui does this automatically)
  - Profile with Tracy, optimize hot paths
  - Fallback: Reduce UI detail at 4K (smaller fonts, fewer effects)
- **Likelihood**: Low (egui is fast, <0.5ms typical)

**2. Controller Input Complexity**
- **Risk**: Gamepad navigation feels clunky
- **Mitigation**:
  - Start with keyboard navigation (Arrow keys, Enter, ESC)
  - Test with actual controllers early (borrow Xbox/PS5)
  - Follow platform conventions (A=confirm on Xbox, X=confirm on PS5)
  - Fallback: Ship with keyboard/mouse only, add gamepad in Phase 8.5
- **Likelihood**: Medium (gamepad UI is tricky)

### Medium Risk

**3. Accessibility Implementation**
- **Risk**: Colorblind modes don't work for all users
- **Mitigation**:
  - Use proven color palettes (Deuteranopia, Protanopia, Tritanopia)
  - Test with colorblind simulation tools (online color blindness simulators)
  - Add text labels to health bars ("HP: 75/100") for redundancy
  - Fallback: Offer high-contrast mode (black/white only)
- **Likelihood**: Low (color palettes are well-established)

**4. Resolution Scaling Bugs**
- **Risk**: UI breaks at 1280x720 or 3840x2160
- **Mitigation**:
  - Test all resolutions early (Day 4 Week 2)
  - Use relative positioning (% of screen) instead of absolute pixels
  - egui handles DPI scaling automatically
  - Fallback: Lock to 1920x1080 for demo, fix later
- **Likelihood**: Low (egui handles scaling well)

### Low Risk

**5. Animation Glitches**
- **Risk**: Fade/slide effects stutter or flicker
- **Mitigation**:
  - Use egui::Animate for smooth interpolation
  - Profile animations with Tracy
  - Clamp animation delta time (avoid huge jumps)
  - Fallback: Disable animations, use instant transitions
- **Likelihood**: Very Low (egui animations are simple)

---

## Testing Strategy

### Unit Tests

**Target**: 70% code coverage for UI modules

**Week 1-2 (Menus)**:
```rust
#[test]
fn test_main_menu_buttons() {
    let mut ctx = UiContext::new();
    let mut screen = MainMenuScreen::default();
    
    // Simulate "New Game" click
    screen.handle_input(UiInput::Click { x: 100, y: 200 });
    assert_eq!(screen.transition, Some(UiScreen::InGame));
}

#[test]
fn test_pause_menu_toggle() {
    let mut ctx = UiContext::new();
    ctx.current_screen = UiScreen::InGame;
    
    // Press ESC
    ctx.handle_input(UiInput::Key { key: Key::Escape });
    assert_eq!(ctx.current_screen, UiScreen::PauseMenu);
    
    // Press ESC again
    ctx.handle_input(UiInput::Key { key: Key::Escape });
    assert_eq!(ctx.current_screen, UiScreen::InGame);
}
```

**Week 3-4 (HUD)**:
```rust
#[test]
fn test_health_bar_color() {
    let health = Health { current: 25.0, max: 100.0 };
    let color = HealthBar::get_color(&health);
    assert_eq!(color, Color::RED); // <25% = red
    
    let health = Health { current: 75.0, max: 100.0 };
    let color = HealthBar::get_color(&health);
    assert_eq!(color, Color::GREEN); // >50% = green
}

#[test]
fn test_minimap_entity_visibility() {
    let player_pos = Position { x: 0.0, y: 0.0, z: 0.0 };
    let entity_pos = Position { x: 60.0, y: 0.0, z: 0.0 }; // 60m away
    
    let minimap = Minimap { zoom: 50.0 }; // 50m radius
    assert!(!minimap.is_visible(&player_pos, &entity_pos)); // Out of range
    
    let entity_pos = Position { x: 40.0, y: 0.0, z: 0.0 }; // 40m away
    assert!(minimap.is_visible(&player_pos, &entity_pos)); // In range
}
```

### Integration Tests

**Week 5 (End-to-End)**:
```rust
#[test]
fn test_veilweaver_playability() {
    // Full acceptance test
    let mut app = App::new();
    app.add_plugin(UiPlugin);
    app.add_plugin(GameplayPlugin);
    
    // 1. Main menu appears
    app.update();
    assert_eq!(app.ui().current_screen, UiScreen::MainMenu);
    
    // 2. Click "New Game"
    app.ui().click_button("New Game");
    app.update();
    assert_eq!(app.ui().current_screen, UiScreen::InGame);
    
    // 3. Press ESC
    app.input().press_key(Key::Escape);
    app.update();
    assert_eq!(app.ui().current_screen, UiScreen::PauseMenu);
    
    // 4. Click "Resume"
    app.ui().click_button("Resume");
    app.update();
    assert_eq!(app.ui().current_screen, UiScreen::InGame);
    
    // 5. Verify HUD visible
    assert!(app.ui().hud().health_bar_visible());
    assert!(app.ui().hud().minimap_visible());
}
```

### Manual Testing

**Daily Smoke Tests** (5 min each day):
- Launch game → Main menu appears
- Click all buttons → Correct transitions
- Press ESC → Pause menu works
- Change resolution → UI scales correctly

**Weekly Acceptance Tests** (30 min end of each week):
- Run full "Veilweaver Playability Test" (see Success Criteria)
- Test on 3 resolutions (1280x720, 1920x1080, 2560x1440)
- Test with mouse + keyboard + controller (Week 5 only)
- Log any bugs, fix before next week

---

## Deliverables & Documentation

### Code Deliverables

**Week 1**:
- `astraweave-ui` crate scaffolding
- `MainMenuScreen` with 4 buttons
- `PauseMenuScreen` with ESC toggle
- Integration with `astraweave-render`

**Week 2**:
- `SettingsMenuScreen` with 3 tabs
- Audio settings (4 volume sliders)
- Graphics settings (resolution, fullscreen)
- Controls display (keybind list)

**Week 3**:
- `HudOverlay` with 4 regions
- `HealthBar` and `StaminaBar` widgets
- `ObjectivesOverlay` widget
- ECS component integration

**Week 4**:
- `Minimap` widget with top-down view
- `Subtitles` widget with fade animation
- Full HUD integration

**Week 5**:
- UI animations (fade, slide, button effects)
- Controller support (gilrs integration)
- Accessibility features (text scaling, colorblind modes)
- Performance optimizations

### Documentation Deliverables

**Week 1**:
- `astraweave-ui/README.md` - Crate overview, usage examples
- `docs/ui/MAIN_MENU_GUIDE.md` - How to customize main menu

**Week 2**:
- `docs/ui/SETTINGS_GUIDE.md` - How to add new settings
- `docs/ui/RESOLUTION_SCALING.md` - How resolution changes work

**Week 3**:
- `docs/ui/HUD_GUIDE.md` - How to add custom HUD elements
- `docs/ui/HUD_LAYOUT.md` - HUD positioning reference

**Week 4**:
- `docs/ui/MINIMAP_GUIDE.md` - How to customize minimap
- `docs/ui/SUBTITLES_GUIDE.md` - How to trigger dialogue

**Week 5**:
- `docs/ui/ANIMATIONS_GUIDE.md` - How to add UI animations
- `docs/ui/ACCESSIBILITY_GUIDE.md` - Accessibility best practices
- `PHASE_8_PRIORITY_1_COMPLETE.md` - Completion summary

### Example Code

**Week 1** (`examples/main_menu_demo.rs`):
```rust
// Standalone main menu test
use astraweave_ui::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugin(UiPlugin);
    app.run();
    // Shows main menu on launch
}
```

**Week 3** (`examples/hud_demo.rs`):
```rust
// HUD overlay test
use astraweave_ui::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugin(UiPlugin);
    app.add_plugin(GameplayPlugin); // Spawns player entity
    
    // Start in-game with HUD visible
    app.ui().current_screen = UiScreen::InGame;
    app.run();
}
```

---

## Success Metrics

### Performance Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| UI render time | <0.5ms @ 60 FPS | Tracy profiler |
| Frame time impact | <2% increase | Before/after comparison |
| Memory usage | <50MB for UI | OS task manager |
| Input latency | <16ms (1 frame) | Manual testing |

### Quality Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Code coverage | 70%+ | `cargo tarpaulin` |
| Compilation warnings | 0 | `cargo clippy` |
| UI bugs found | <5 per week | Manual testing log |
| Accessibility compliance | WCAG 2.1 Level AA | Checklist |

### User Experience Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Menu navigation | <3 clicks to any screen | Click counting |
| Settings apply time | <1 second | Stopwatch |
| HUD readability | Clear at 1280x720 | Visual inspection |
| Controller responsiveness | <100ms lag | Manual testing |

---

## Phase 8 Priority 1 Completion Checklist

**Core UI Framework** (Week 1-2):
- [ ] Main menu with "New Game", "Load", "Settings", "Quit"
- [ ] Pause menu with ESC toggle
- [ ] Settings menu with audio, graphics, controls tabs
- [ ] Volume sliders (4 channels)
- [ ] Resolution dropdown + fullscreen toggle
- [ ] Keybind display (read-only)

**HUD System** (Week 3-4):
- [ ] Health bar with color gradient
- [ ] Stamina bar
- [ ] Quest objectives overlay (3-5 lines)
- [ ] Minimap (128x128, top-down view)
- [ ] Dialogue subtitles with fade
- [ ] HUD visibility toggle (gameplay vs menus)

**Polish** (Week 5):
- [ ] UI animations (fade, slide, button effects)
- [ ] Controller support (D-pad, A/B buttons)
- [ ] Accessibility (text scaling 80-150%)
- [ ] Colorblind modes (3 variants)
- [ ] Performance <0.5ms UI rendering
- [ ] 70%+ code coverage

**Documentation**:
- [ ] 6 UI guides written
- [ ] 2 example demos created
- [ ] Completion summary published

**Acceptance Test**:
- [ ] "Veilweaver Playability Test" passes 100%
- [ ] Tested on 3 resolutions
- [ ] Tested with mouse, keyboard, controller
- [ ] 0 critical bugs

---

## Next Steps After Completion

**Phase 8 Priority 2: Complete Rendering Pipeline** (4-6 weeks)
- Shadow mapping (CSM + omnidirectional)
- Skybox/atmosphere rendering
- Post-processing stack (bloom, tonemapping, SSAO)
- Dynamic lighting (point/spot/directional)
- Particle system (GPU-accelerated)

**Phase 8 Priority 3: Save/Load System** (2-3 weeks)
- Serialize ECS world state
- Player profile management
- Save slot system with versioning

**Phase 8 Priority 4: Production Audio** (3-4 weeks)
- Audio mixer (master, music, SFX, voice buses)
- Dynamic music system
- Audio occlusion and reverb zones

**Total Phase 8**: 13-18 weeks (3-4.5 months)

---

## Conclusion

Phase 8 Priority 1 delivers the **critical missing piece** for Veilweaver playability: a complete in-game UI system. After 5 weeks of focused development, AstraWeave will have menus, HUD, and settings that enable end-to-end gameplay testing.

**Key Wins**:
- ✅ Veilweaver becomes playable (main menu → gameplay → pause → settings)
- ✅ Foundation for all future UI (save/load dialogs, audio settings, keybind editor)
- ✅ Polished player experience (animations, controller support, accessibility)
- ✅ Fast implementation (egui = rapid prototyping)

**Recommended Path**: Approve this plan and begin **Week 1: Core UI Infrastructure** immediately (Oct 14, 2025).

---

**Document Status**: Ready for implementation  
**Last Updated**: October 14, 2025  
**Next Review**: End of Week 1 (Oct 18, 2025)  
**Maintainer**: AI Development Team
