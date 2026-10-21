# Phase 8.1 Quick Start Guide

**Date**: October 14, 2025  
**Purpose**: Get started with Phase 8.1 (In-Game UI Framework) implementation  
**Duration**: Week 1 (5 days)

---

## Prerequisites

**Before starting**:
1. ✅ Read `PHASE_8_PRIORITY_1_UI_PLAN.md` (comprehensive 5-week plan)
2. ✅ Read `PHASE_8_MASTER_INTEGRATION_PLAN.md` (understand dependencies)
3. ✅ Ensure workspace builds: `cargo check-all` (or task: Phase1-check)

---

## Week 1 Overview

**Goal**: Core UI infrastructure with main menu and pause menu

**Deliverables**:
- ✅ New crate: `astraweave-ui` with egui-wgpu integration
- ✅ Main menu (New Game, Load Game, Settings, Quit)
- ✅ Pause menu (Resume, Save, Settings, Quit)
- ✅ Menu navigation (keyboard, mouse, controller)
- ✅ Menu rendering over 3D scene

**Timeline**: 5 days (October 15-19, 2025)

---

## Step-by-Step Instructions

### Day 1: Create Crate & Setup egui-wgpu

**1. Create new crate**:
```bash
# In workspace root
cargo new --lib crates/astraweave-ui
```

**2. Add to workspace** (`Cargo.toml` root):
```toml
[workspace]
members = [
    # ... existing members
    "crates/astraweave-ui",
]
```

**3. Add dependencies** (`crates/astraweave-ui/Cargo.toml`):
```toml
[package]
name = "astraweave-ui"
version = "0.1.0"
edition = "2021"

[dependencies]
# UI framework
egui = "0.29"
egui-wgpu = "0.29"

# Rendering
wgpu = { workspace = true }

# Utilities
anyhow = { workspace = true }
serde = { workspace = true, features = ["derive"] }

# ECS integration (for game state)
astraweave-ecs = { path = "../astraweave-ecs" }

[features]
default = []
```

**4. Create module structure**:
```bash
# In crates/astraweave-ui/src/
touch lib.rs          # Main module
touch menu.rs         # Menu system
touch main_menu.rs    # Main menu UI
touch pause_menu.rs   # Pause menu UI
touch ui_context.rs   # UI rendering context
```

**5. Verify build**:
```bash
cargo check -p astraweave-ui
```

**Success**: Crate builds without errors

---

### Day 2: egui-wgpu Integration

**1. Create UI rendering context** (`ui_context.rs`):
```rust
use egui_wgpu::Renderer;
use wgpu::{Device, Queue, TextureFormat};
use anyhow::Result;

pub struct UiContext {
    pub egui_ctx: egui::Context,
    pub egui_renderer: Renderer,
    pub screen_descriptor: egui_wgpu::ScreenDescriptor,
}

impl UiContext {
    pub fn new(device: &Device, surface_format: TextureFormat, width: u32, height: u32) -> Result<Self> {
        let egui_ctx = egui::Context::default();
        let egui_renderer = Renderer::new(device, surface_format, None, 1);
        
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: 1.0,
        };
        
        Ok(Self {
            egui_ctx,
            egui_renderer,
            screen_descriptor,
        })
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        self.screen_descriptor.size_in_pixels = [width, height];
    }
    
    pub fn render(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        run_ui: impl FnOnce(&egui::Context),
    ) -> Result<()> {
        // Begin frame
        let raw_input = egui::RawInput::default();
        self.egui_ctx.begin_frame(raw_input);
        
        // Run user UI code
        run_ui(&self.egui_ctx);
        
        // End frame and render
        let output = self.egui_ctx.end_frame();
        let paint_jobs = self.egui_ctx.tessellate(output.shapes, output.pixels_per_point);
        
        // Upload textures
        for (id, image_delta) in &output.textures_delta.set {
            self.egui_renderer.update_texture(device, queue, *id, image_delta);
        }
        
        // Render
        self.egui_renderer.update_buffers(device, queue, encoder, &paint_jobs, &self.screen_descriptor);
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Don't clear (render over 3D scene)
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            self.egui_renderer.render(&mut rpass, &paint_jobs, &self.screen_descriptor);
        }
        
        // Free textures
        for id in &output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }
        
        Ok(())
    }
}
```

**2. Export from lib.rs**:
```rust
// crates/astraweave-ui/src/lib.rs
pub mod ui_context;
pub mod menu;
pub mod main_menu;
pub mod pause_menu;

pub use ui_context::UiContext;
```

**3. Verify build**:
```bash
cargo check -p astraweave-ui
```

**Success**: UI context compiles, ready for menu implementation

---

### Day 3: Main Menu Implementation

**1. Create menu state enum** (`menu.rs`):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuState {
    MainMenu,
    PauseMenu,
    SettingsMenu,
    None, // In-game, no menu
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    NewGame,
    LoadGame,
    SaveGame,
    Resume,
    Settings,
    Quit,
    None,
}
```

**2. Implement main menu** (`main_menu.rs`):
```rust
use egui::{Align2, Color32, FontId, RichText};
use super::MenuAction;

pub fn show_main_menu(ctx: &egui::Context) -> MenuAction {
    let mut action = MenuAction::None;
    
    // Centered window
    egui::Window::new("AstraWeave")
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .title_bar(false)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                // Title
                ui.label(
                    RichText::new("ASTRAWEAVE")
                        .font(FontId::proportional(48.0))
                        .color(Color32::WHITE)
                );
                
                ui.add_space(20.0);
                
                // Buttons
                if ui.button(RichText::new("New Game").font(FontId::proportional(24.0))).clicked() {
                    action = MenuAction::NewGame;
                }
                
                if ui.button(RichText::new("Load Game").font(FontId::proportional(24.0))).clicked() {
                    action = MenuAction::LoadGame;
                }
                
                if ui.button(RichText::new("Settings").font(FontId::proportional(24.0))).clicked() {
                    action = MenuAction::Settings;
                }
                
                if ui.button(RichText::new("Quit").font(FontId::proportional(24.0))).clicked() {
                    action = MenuAction::Quit;
                }
            });
        });
    
    action
}
```

**3. Verify build**:
```bash
cargo check -p astraweave-ui
```

**Success**: Main menu renders (will integrate with example next)

---

### Day 4: Pause Menu & Navigation

**1. Implement pause menu** (`pause_menu.rs`):
```rust
use egui::{Align2, Color32, FontId, RichText};
use super::MenuAction;

pub fn show_pause_menu(ctx: &egui::Context) -> MenuAction {
    let mut action = MenuAction::None;
    
    // Semi-transparent background
    egui::Area::new("pause_overlay")
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.visuals_mut().window_fill = Color32::from_rgba_premultiplied(0, 0, 0, 200);
            
            egui::Frame::window(ui.style())
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        // Title
                        ui.label(
                            RichText::new("PAUSED")
                                .font(FontId::proportional(36.0))
                                .color(Color32::WHITE)
                        );
                        
                        ui.add_space(20.0);
                        
                        // Buttons
                        if ui.button(RichText::new("Resume").font(FontId::proportional(24.0))).clicked() {
                            action = MenuAction::Resume;
                        }
                        
                        if ui.button(RichText::new("Save Game").font(FontId::proportional(24.0))).clicked() {
                            action = MenuAction::SaveGame;
                        }
                        
                        if ui.button(RichText::new("Settings").font(FontId::proportional(24.0))).clicked() {
                            action = MenuAction::Settings;
                        }
                        
                        if ui.button(RichText::new("Quit to Main Menu").font(FontId::proportional(24.0))).clicked() {
                            action = MenuAction::Quit;
                        }
                    });
                });
        });
    
    action
}
```

**2. Create menu manager** (`menu.rs`):
```rust
pub struct MenuManager {
    pub state: MenuState,
}

impl MenuManager {
    pub fn new() -> Self {
        Self {
            state: MenuState::MainMenu,
        }
    }
    
    pub fn show(&self, ctx: &egui::Context) -> MenuAction {
        match self.state {
            MenuState::MainMenu => crate::main_menu::show_main_menu(ctx),
            MenuState::PauseMenu => crate::pause_menu::show_pause_menu(ctx),
            MenuState::SettingsMenu => MenuAction::None, // TODO: Week 2
            MenuState::None => MenuAction::None,
        }
    }
    
    pub fn handle_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::Resume => self.state = MenuState::None,
            MenuAction::Settings => self.state = MenuState::SettingsMenu,
            MenuAction::Quit => self.state = MenuState::MainMenu,
            _ => {} // Handle in game logic
        }
    }
}
```

**3. Verify build**:
```bash
cargo check -p astraweave-ui
```

**Success**: All UI code compiles

---

### Day 5: Integration Example

**1. Create example** (`examples/ui_demo/main.rs`):
```rust
use astraweave_ui::{UiContext, MenuManager, MenuAction};
use winit::event_loop::EventLoop;
use anyhow::Result;

fn main() -> Result<()> {
    // Create window + wgpu setup (simplified, use existing pattern from unified_showcase)
    let event_loop = EventLoop::new()?;
    // ... wgpu device, queue, surface setup
    
    let mut ui_context = UiContext::new(&device, surface_format, 1920, 1080)?;
    let mut menu_manager = MenuManager::new();
    
    event_loop.run(move |event, elwt| {
        match event {
            winit::event::Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::RedrawRequested => {
                    // Render 3D scene (if any)
                    // ...
                    
                    // Render UI on top
                    let mut encoder = device.create_command_encoder(&Default::default());
                    ui_context.render(&device, &queue, &mut encoder, &view, |ctx| {
                        let action = menu_manager.show(ctx);
                        menu_manager.handle_action(action);
                        
                        match action {
                            MenuAction::NewGame => println!("Start new game!"),
                            MenuAction::Quit => elwt.exit(),
                            _ => {}
                        }
                    })?;
                    queue.submit([encoder.finish()]);
                    surface.present();
                }
                winit::event::WindowEvent::CloseRequested => elwt.exit(),
                _ => {}
            },
            _ => {}
        }
    })?;
    
    Ok(())
}
```

**2. Test example**:
```bash
cargo run --example ui_demo --release
```

**Success**: 
- ✅ Main menu displays on startup
- ✅ Buttons are clickable
- ✅ "Quit" exits application
- ✅ UI renders over black background (no 3D scene yet)

---

## Week 1 Completion Checklist

- [ ] `astraweave-ui` crate created and added to workspace
- [ ] egui-wgpu integration complete (`UiContext`)
- [ ] Main menu implemented (New Game, Load Game, Settings, Quit)
- [ ] Pause menu implemented (Resume, Save, Settings, Quit)
- [ ] Menu manager handles state transitions
- [ ] Example demonstrates UI working
- [ ] All code compiles: `cargo check -p astraweave-ui`
- [ ] Example runs: `cargo run --example ui_demo --release`

---

## Common Issues & Solutions

**Issue 1: egui version mismatch**
- **Error**: `egui` and `egui-wgpu` version conflict
- **Solution**: Use same version for both (0.29.x)

**Issue 2: wgpu surface format**
- **Error**: Surface format not compatible with egui
- **Solution**: Use `TextureFormat::Bgra8UnormSrgb` or `Rgba8UnormSrgb`

**Issue 3: UI not rendering**
- **Check 1**: Is `render()` called after 3D scene rendering?
- **Check 2**: Is `LoadOp::Load` used (not `Clear`)?
- **Check 3**: Are textures uploaded before rendering?

**Issue 4: Keyboard/mouse input not working**
- **Solution**: Pass winit events to egui (will implement in Week 2)

---

## Next Steps (Week 2)

**After Week 1 complete**:
1. Read Week 2 section of `PHASE_8_PRIORITY_1_UI_PLAN.md`
2. Implement settings menu (audio, graphics, controls)
3. Integrate input handling (keyboard, mouse, gamepad)
4. Polish menu visuals (backgrounds, animations)

---

## Resources

**Documentation**:
- egui documentation: https://docs.rs/egui/
- egui-wgpu documentation: https://docs.rs/egui-wgpu/
- PHASE_8_PRIORITY_1_UI_PLAN.md (this workspace)

**Reference Examples**:
- `examples/unified_showcase/` - wgpu setup pattern
- `tools/aw_editor/` - egui usage in editor (14 panels)

**Questions?**
- Consult `PHASE_8_MASTER_INTEGRATION_PLAN.md` for timeline
- Check `.github/copilot-instructions.md` for general guidance

---

**Status**: 🚀 READY TO START - Begin Day 1 immediately

**Last Updated**: October 14, 2025  
**Timeline**: Week 1 of 5 (Phase 8.1)
