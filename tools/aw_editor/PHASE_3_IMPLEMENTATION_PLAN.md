# AstraWeave Editor Phase 3: Implementation Plan
**Status**: Ready to Implement 🎯  
**Target**: Professional Editor  
**Timeline**: 4-6 weeks (Phase 3) + 6-8 weeks (Phase 4) = 10-14 weeks total

---

## 📊 Current State Assessment

### ✅ Completed (Phases 1-2)

**Phase 1: Gizmos & Viewport**
- ✅ 3D viewport with wgpu rendering (@60 FPS)
- ✅ Orbit camera (left drag), pan (middle drag), zoom (scroll)
- ✅ Transform gizmos (G/R/S for Translate/Rotate/Scale)
- ✅ Entity selection via raycast
- ✅ Axis constraints (X/Y/Z) with visual feedback
- ✅ Enter/Escape confirm/cancel workflow

**Phase 2: Foundation**
- ✅ Undo/redo system (`command.rs`) - Command pattern with 100-command history
- ✅ Scene serialization (`scene_serialization.rs`) - RON format save/load
- ✅ Component-based inspector (`component_ui.rs`) - InspectorUI trait for Pose/Health/Team/Ammo
- ✅ File watcher infrastructure (`file_watcher.rs`) - notify crate integration
- ✅ Clipboard infrastructure (`clipboard.rs`) - Ready for copy/paste

**Infrastructure**
- ✅ 14 panels: Hierarchy, Transform, Entity, Material, Asset Browser, Performance, Charts, Graphs, Animation, etc.
- ✅ ECS integration (astraweave-core World API)
- ✅ Material system with BRDF preview
- ✅ Entity manager with spawn/delete
- ✅ Grid/skybox/entity/gizmo renderers

---

## 🎯 Phase 3: Productivity Layer (Next 4-6 Weeks)

### Mission-Critical Priorities

#### PRIORITY 1: Multi-Selection & Context Menus (Week 1-2)
**Impact**: ⭐⭐⭐⭐⭐ (Highest productivity gain)  
**Complexity**: Medium  
**Blocks**: Bulk editing, copy/paste efficiency

**Implementation Plan**:

**Week 1: Multi-Selection Core**
1. **SelectionSet** data structure:
```rust
// In entity_manager.rs
#[derive(Debug, Clone, Default)]
pub struct SelectionSet {
    pub entities: HashSet<Entity>,
    pub primary: Option<Entity>, // Last selected
}

impl SelectionSet {
    pub fn add(&mut self, entity: Entity, is_primary: bool) {
        self.entities.insert(entity);
        if is_primary {
            self.primary = Some(entity);
        }
    }

    pub fn remove(&mut self, entity: Entity) {
        self.entities.remove(&entity);
        if self.primary == Some(entity) {
            self.primary = self.entities.iter().next().copied();
        }
    }

    pub fn toggle(&mut self, entity: Entity) {
        if self.entities.contains(&entity) {
            self.remove(entity);
        } else {
            self.add(entity, true);
        }
    }

    pub fn clear(&mut self) {
        self.entities.clear();
        self.primary = None;
    }

    pub fn is_selected(&self, entity: Entity) -> bool {
        self.entities.contains(&entity)
    }

    pub fn count(&self) -> usize {
        self.entities.len()
    }
}
```

2. **Input Handling** (in `main.rs` or `gizmo/input.rs`):
```rust
// Ctrl+Click: Toggle selection
if input.modifiers.ctrl && input.clicked_entity.is_some() {
    selection.toggle(input.clicked_entity.unwrap());
}
// Shift+Click: Range selection (select all between primary and new)
else if input.modifiers.shift && input.clicked_entity.is_some() {
    // Range select implementation (for hierarchy)
}
// Normal click: Single selection
else if input.clicked_entity.is_some() {
    selection.clear();
    selection.add(input.clicked_entity.unwrap(), true);
}
```

3. **Visual Feedback**:
   - Hierarchy panel: Highlight all selected entities (darker background)
   - Viewport: Draw gizmo on primary entity, outline on others
   - Status bar: Show "3 entities selected" count

**Week 2: Context Menus & Bulk Operations**
4. **Right-Click Context Menu**:
```rust
// In hierarchy_panel.rs
if ui.rect_contains_pointer(entity_rect) && ui.input(|i| i.pointer.button_released(egui::PointerButton::Secondary)) {
    ui.menu_button("Entity Options", |ui| {
        if ui.button("🔄 Duplicate").clicked() {
            // Duplicate selected entities
        }
        if ui.button("❌ Delete").clicked() {
            // Delete selected entities (with undo)
        }
        if ui.button("✏️ Rename").clicked() {
            // Enter rename mode
        }
        ui.separator();
        if ui.button("📋 Copy").clicked() {
            // Copy to clipboard
        }
        if ui.button("📄 Paste").clicked() {
            // Paste from clipboard
        }
    });
}
```

5. **Bulk Edit Support** (in `component_ui.rs`):
```rust
// When multiple entities selected, show "Multiple (X) selected" in inspector
// Only show components common to ALL selected entities
// Editing applies to all selected
pub fn show_multi_inspector(ui: &mut Ui, selection: &SelectionSet, world: &mut World) {
    ui.heading(format!("Multiple ({}) Selected", selection.count()));
    
    // Find common components
    let common_components = find_common_components(selection, world);
    
    for component_type in common_components {
        if let Some(edit) = show_component_multi(ui, component_type, selection, world) {
            // Apply edit to all selected entities
        }
    }
}
```

**Success Criteria**:
- ✅ Ctrl+Click adds/removes from selection
- ✅ Shift+Click range selects in hierarchy
- ✅ Right-click shows context menu
- ✅ Inspector shows "Multiple (X) selected" when 2+ entities selected
- ✅ Editing position applies to all selected entities
- ✅ Duplicate/delete works on selection
- ✅ Undo/redo works for bulk operations

---

#### PRIORITY 2: Snapping System (Week 2-3)
**Impact**: ⭐⭐⭐⭐ (Essential for level design)  
**Complexity**: Low-Medium  
**Blocks**: Precise placement, grid-aligned levels

**Implementation Plan**:

**Snapping Module** (new file: `gizmo/snapping.rs`):
```rust
use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct SnapSettings {
    pub grid_enabled: bool,
    pub grid_size: f32,
    pub angle_enabled: bool,
    pub angle_degrees: f32,
    pub vertex_enabled: bool,
}

impl Default for SnapSettings {
    fn default() -> Self {
        Self {
            grid_enabled: false,
            grid_size: 1.0,
            angle_enabled: false,
            angle_degrees: 15.0,
            vertex_enabled: false,
        }
    }
}

impl SnapSettings {
    pub fn snap_position(&self, pos: Vec3, is_ctrl_held: bool) -> Vec3 {
        if !self.grid_enabled && !is_ctrl_held {
            return pos; // No snapping
        }

        let grid = self.grid_size;
        Vec3::new(
            (pos.x / grid).round() * grid,
            (pos.y / grid).round() * grid,
            (pos.z / grid).round() * grid,
        )
    }

    pub fn snap_rotation(&self, rotation_deg: f32, is_ctrl_held: bool) -> f32 {
        if !self.angle_enabled && !is_ctrl_held {
            return rotation_deg; // No snapping
        }

        let angle = self.angle_degrees;
        (rotation_deg / angle).round() * angle
    }

    pub fn snap_to_vertex(&self, pos: Vec3, nearby_vertices: &[Vec3], threshold: f32) -> Vec3 {
        if !self.vertex_enabled {
            return pos;
        }

        // Find closest vertex within threshold
        nearby_vertices
            .iter()
            .filter(|v| v.distance(pos) < threshold)
            .min_by_key(|v| (v.distance(pos) * 1000.0) as i32)
            .copied()
            .unwrap_or(pos)
    }
}
```

**UI Integration** (in `viewport/toolbar.rs`):
```rust
// Add snapping toolbar
ui.horizontal(|ui| {
    ui.label("Snap:");
    
    // Grid toggle
    if ui.selectable_label(snap.grid_enabled, "🔲 Grid").clicked() {
        snap.grid_enabled = !snap.grid_enabled;
    }
    
    // Grid size
    if snap.grid_enabled {
        ui.label("Size:");
        ui.add(egui::DragValue::new(&mut snap.grid_size)
            .speed(0.1)
            .clamp_range(0.1..=10.0));
    }
    
    // Angle toggle
    if ui.selectable_label(snap.angle_enabled, "🔄 Angle").clicked() {
        snap.angle_enabled = !snap.angle_enabled;
    }
    
    // Angle increment
    if snap.angle_enabled {
        ui.add(egui::DragValue::new(&mut snap.angle_degrees)
            .speed(1.0)
            .clamp_range(1.0..=90.0)
            .suffix("°"));
    }
});

ui.label("💡 Hold Ctrl to temporarily toggle snap");
```

**Gizmo Integration** (in `gizmo/translate.rs`):
```rust
// Apply snapping before setting position
let mut new_pos = calculate_new_position(mouse_ray, constraint);
new_pos = snap_settings.snap_position(new_pos, input.modifiers.ctrl);
apply_position(entity, new_pos);
```

**Success Criteria**:
- ✅ Grid snapping works for translate gizmo
- ✅ Angle snapping works for rotate gizmo (15°, 30°, 45°, 90° increments)
- ✅ Ctrl held temporarily toggles snap state
- ✅ Toolbar shows current snap settings
- ✅ Grid size adjustable (0.1 to 10.0)
- ✅ Angle increment adjustable (1° to 90°)
- ✅ Snapping works with undo/redo

---

#### PRIORITY 3: Copy/Paste/Duplicate (Week 3-4)
**Impact**: ⭐⭐⭐⭐⭐ (Core workflow feature)  
**Complexity**: Medium  
**Blocks**: Nothing (but enhances everything)

**Implementation Plan**:

**Enhanced Clipboard** (update `clipboard.rs`):
```rust
use serde::{Deserialize, Serialize};
use astraweave_core::{Entity, IVec2, World};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardData {
    pub entities: Vec<EntityClipboard>,
    pub relative_positions: bool, // Paste at cursor vs absolute
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityClipboard {
    pub name: String,
    pub pos: IVec2,
    pub rotation: f32,
    pub rotation_x: f32,
    pub rotation_z: f32,
    pub scale: f32,
    pub hp: i32,
    pub team_id: u8,
    pub ammo: i32,
    pub cooldowns: HashMap<String, f32>,
}

impl ClipboardData {
    pub fn from_selection(world: &World, entities: &[Entity]) -> Self {
        let entities_data = entities
            .iter()
            .filter_map(|&e| EntityClipboard::from_world(world, e))
            .collect();

        Self {
            entities: entities_data,
            relative_positions: true,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }

    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse clipboard JSON")
    }

    pub fn paste_into_world(&self, world: &mut World, paste_offset: IVec2) -> Vec<Entity> {
        let mut new_entities = Vec::new();

        for entity_data in &self.entities {
            let mut pos = entity_data.pos;
            if self.relative_positions {
                pos = IVec2::new(pos.x + paste_offset.x, pos.y + paste_offset.y);
            }

            let id = world.spawn(
                &entity_data.name,
                pos,
                astraweave_core::Team { id: entity_data.team_id },
                entity_data.hp,
                entity_data.ammo,
            );

            // Apply rotation/scale
            if let Some(pose) = world.pose_mut(id) {
                pose.rotation = entity_data.rotation;
                pose.rotation_x = entity_data.rotation_x;
                pose.rotation_z = entity_data.rotation_z;
                pose.scale = entity_data.scale;
            }

            new_entities.push(id);
        }

        new_entities
    }
}
```

**Keyboard Shortcuts** (in `main.rs` or `input.rs`):
```rust
// Ctrl+C: Copy
if input.modifiers.command && input.key_pressed(egui::Key::C) && !selection.is_empty() {
    let entities: Vec<Entity> = selection.entities.iter().copied().collect();
    let clipboard_data = ClipboardData::from_selection(&world, &entities);
    
    // Copy to system clipboard (cross-platform)
    if let Ok(json) = serde_json::to_string(&clipboard_data) {
        clipboard.set_text(json);
    }
    
    println!("📋 Copied {} entities", entities.len());
}

// Ctrl+V: Paste
if input.modifiers.command && input.key_pressed(egui::Key::V) {
    if let Ok(json) = clipboard.get_text() {
        if let Ok(clipboard_data) = ClipboardData::from_json(&json) {
            // Paste at mouse cursor or offset from original
            let paste_offset = IVec2::new(2, 2); // Offset by 2 units
            let new_entities = clipboard_data.paste_into_world(&mut world, paste_offset);
            
            // Select pasted entities
            selection.clear();
            for entity in new_entities {
                selection.add(entity, false);
            }
            if let Some(&first) = new_entities.first() {
                selection.primary = Some(first);
            }
            
            // Add to undo stack
            let cmd = PasteEntitiesCommand::new(new_entities.clone());
            undo_stack.push_executed(Box::new(cmd));
            
            println!("📄 Pasted {} entities", new_entities.len());
        }
    }
}

// Ctrl+D: Duplicate
if input.modifiers.command && input.key_pressed(egui::Key::D) && !selection.is_empty() {
    let entities: Vec<Entity> = selection.entities.iter().copied().collect();
    let clipboard_data = ClipboardData::from_selection(&world, &entities);
    
    // Duplicate in place with small offset
    let paste_offset = IVec2::new(1, 1);
    let new_entities = clipboard_data.paste_into_world(&mut world, paste_offset);
    
    // Select duplicates
    selection.clear();
    for entity in new_entities.iter() {
        selection.add(*entity, false);
    }
    selection.primary = new_entities.first().copied();
    
    // Add to undo stack
    let cmd = DuplicateEntitiesCommand::new(new_entities.clone());
    undo_stack.push_executed(Box::new(cmd));
    
    println!("🔄 Duplicated {} entities", entities.len());
}
```

**Undo Commands** (in `command.rs`):
```rust
#[derive(Debug)]
pub struct PasteEntitiesCommand {
    pasted_entities: Vec<Entity>,
}

impl EditorCommand for PasteEntitiesCommand {
    fn execute(&mut self, _world: &mut World) -> Result<()> {
        // Already executed during paste
        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        // Delete pasted entities
        for &entity in &self.pasted_entities {
            world.despawn(entity);
        }
        Ok(())
    }

    fn describe(&self) -> String {
        format!("Paste {} Entities", self.pasted_entities.len())
    }
}
```

**Success Criteria**:
- ✅ Ctrl+C copies selected entities to clipboard (JSON format)
- ✅ Ctrl+V pastes entities at offset from original positions
- ✅ Ctrl+D duplicates entities in place (1 unit offset)
- ✅ Pasted/duplicated entities are auto-selected
- ✅ Copy/paste works across editor instances (system clipboard)
- ✅ Undo/redo works for paste/duplicate operations
- ✅ Multi-selection paste works correctly

---

#### PRIORITY 4: Asset Browser Enhancements (Week 4-5)
**Impact**: ⭐⭐⭐ (Workflow improvement)  
**Complexity**: Medium-High  
**Blocks**: Efficient asset management

**Current State**: Basic file tree with icons and file sizes

**Enhancements Needed**:

1. **Thumbnail Previews** (Week 4):
```rust
// In asset_browser.rs
use image::DynamicImage;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

pub struct ThumbnailCache {
    cache: Arc<Mutex<HashMap<PathBuf, egui::TextureHandle>>>,
    loading: Arc<Mutex<HashSet<PathBuf>>>,
}

impl ThumbnailCache {
    pub fn get_or_load(&mut self, ctx: &egui::Context, path: &Path) -> Option<&egui::TextureHandle> {
        // Check cache
        if let Some(texture) = self.cache.lock().unwrap().get(path) {
            return Some(texture);
        }

        // Check if already loading
        if self.loading.lock().unwrap().contains(path) {
            return None; // Show placeholder
        }

        // Start loading (async)
        self.loading.lock().unwrap().insert(path.to_path_buf());
        let path_clone = path.to_path_buf();
        let cache_clone = self.cache.clone();
        let ctx_clone = ctx.clone();

        std::thread::spawn(move || {
            if let Ok(img) = image::open(&path_clone) {
                let thumbnail = img.resize(64, 64, image::imageops::FilterType::Lanczos3);
                let rgba = thumbnail.to_rgba8();
                
                let texture = ctx_clone.load_texture(
                    path_clone.to_string_lossy(),
                    egui::ColorImage::from_rgba_unmultiplied(
                        [thumbnail.width() as usize, thumbnail.height() as usize],
                        rgba.as_raw(),
                    ),
                    Default::default(),
                );

                cache_clone.lock().unwrap().insert(path_clone, texture);
                ctx_clone.request_repaint();
            }
        });

        None
    }
}
```

2. **Drag-and-Drop into Viewport** (Week 4):
```rust
// In asset_browser.rs
if ui.button(format!("{} {}", asset.asset_type.icon(), asset.name)).dragged() {
    // Start drag operation
    ui.output_mut(|o| {
        o.cursor_icon = egui::CursorIcon::Grabbing;
    });
    
    // Store drag data
    drag_state.asset_path = Some(asset.path.clone());
}

// In viewport widget (viewport/mod.rs)
if let Some(asset_path) = drag_state.asset_path.take() {
    if response.hovered() {
        // Show preview at mouse position
        ui.painter().circle_filled(mouse_pos, 10.0, egui::Color32::GREEN);
    }
    
    if response.drag_released() {
        // Raycast to find 3D position
        if let Some(world_pos) = raycast_mouse_to_world(mouse_pos, camera) {
            // Spawn entity from asset
            spawn_entity_from_asset(&mut world, &asset_path, world_pos);
        }
    }
}
```

3. **Import Settings Dialog** (Week 5):
```rust
// In asset_browser.rs
if ui.button("Import...").clicked() {
    show_import_dialog = true;
}

if show_import_dialog {
    egui::Window::new("Import Settings")
        .show(ctx, |ui| {
            ui.heading("Model Import Options");
            
            ui.horizontal(|ui| {
                ui.label("Scale:");
                ui.add(egui::DragValue::new(&mut import_settings.scale)
                    .speed(0.1)
                    .clamp_range(0.01..=100.0));
            });
            
            ui.horizontal(|ui| {
                ui.label("Pivot:");
                egui::ComboBox::from_label("")
                    .selected_text(&import_settings.pivot)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut import_settings.pivot, "Bottom".into(), "Bottom");
                        ui.selectable_value(&mut import_settings.pivot, "Center".into(), "Center");
                        ui.selectable_value(&mut import_settings.pivot, "Top".into(), "Top");
                    });
            });
            
            ui.checkbox(&mut import_settings.generate_collider, "Generate Collider");
            
            if ui.button("Import").clicked() {
                // Import with settings
                import_asset(&asset_path, &import_settings);
                show_import_dialog = false;
            }
        });
}
```

**Success Criteria**:
- ✅ Thumbnail previews for textures (64x64, async loading)
- ✅ Placeholder icon while loading thumbnails
- ✅ Drag asset from browser to viewport spawns entity
- ✅ Preview circle shows drop location in 3D
- ✅ Import dialog shows for models (scale, pivot, collider)
- ✅ Asset import adds to undo stack
- ✅ Recent files list (last 10 imported)

---

## 🚀 Quick Wins (Can be done in parallel)

### Quick Win 1: Status Bar (2-3 hours)
Add bottom status bar showing:
- Current gizmo mode (G/R/S)
- Selected entity count
- Undo/redo state ("Undo: Move Entity")
- FPS counter
- Snap state (Grid: ON 1.0u)

```rust
// In main.rs
egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
    ui.horizontal(|ui| {
        // Gizmo mode
        ui.label(format!("Mode: {}", match gizmo.mode {
            GizmoMode::Translate => "Translate (G)",
            GizmoMode::Rotate => "Rotate (R)",
            GizmoMode::Scale => "Scale (S)",
        }));

        ui.separator();

        // Selection count
        ui.label(format!("{} selected", selection.count()));

        ui.separator();

        // Undo/redo
        if let Some(desc) = undo_stack.undo_description() {
            ui.label(format!("⏮️  Undo: {}", desc));
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // FPS
            ui.label(format!("FPS: {:.0}", ui.ctx().frame_duration().recip()));

            ui.separator();

            // Snap
            if snap.grid_enabled {
                ui.label(format!("🔲 Grid: {:.1}u", snap.grid_size));
            }
            if snap.angle_enabled {
                ui.label(format!("🔄 Angle: {:.0}°", snap.angle_degrees));
            }
        });
    });
});
```

### Quick Win 2: Keyboard Shortcuts Panel (1-2 hours)
Add Help → Keyboard Shortcuts window showing all hotkeys:
- G/R/S: Gizmo modes
- Ctrl+Z/Y: Undo/redo
- Ctrl+C/V/D: Copy/paste/duplicate
- Ctrl+S/O: Save/load scene
- Delete: Delete selected
- F: Frame selected entity in viewport

### Quick Win 3: Recent Files Menu (2-3 hours)
Add File → Recent menu with last 10 opened scenes:
```rust
#[derive(Serialize, Deserialize, Default)]
struct EditorSettings {
    recent_files: VecDeque<PathBuf>, // Max 10
}

impl EditorSettings {
    fn add_recent(&mut self, path: PathBuf) {
        // Remove if already exists
        self.recent_files.retain(|p| p != &path);
        
        // Add to front
        self.recent_files.push_front(path);
        
        // Limit to 10
        self.recent_files.truncate(10);
        
        // Save settings
        self.save();
    }
}
```

---

## 📈 Phase 3 Timeline Summary

| Week | Task | Deliverables |
|------|------|-------------|
| **Week 1** | Multi-selection core | SelectionSet, Ctrl+click, visual feedback |
| **Week 2** | Context menus & bulk edit | Right-click menu, multi-inspector, bulk operations |
| **Week 2-3** | Snapping system | Grid/angle snapping, toolbar UI, Ctrl toggle |
| **Week 3-4** | Copy/paste/duplicate | Ctrl+C/V/D, clipboard JSON, undo integration |
| **Week 4-5** | Asset browser enhancements | Thumbnails, drag-drop, import dialog |
| **Week 5-6** | Polish & testing | Status bar, shortcuts panel, recent files, bug fixes |

**Total**: 5-6 weeks for Phase 3 completion

---

## 🎯 Success Metrics

**Phase 3 Complete When**:
- ✅ Multi-selection works (Ctrl+click, Shift+click, range select)
- ✅ Right-click context menu on entities (duplicate, delete, rename, copy, paste)
- ✅ Bulk editing in inspector (change position for 10 entities at once)
- ✅ Grid snapping (Ctrl toggle, configurable size)
- ✅ Angle snapping (15°/30°/45°/90° increments)
- ✅ Copy/paste/duplicate (Ctrl+C/V/D with undo support)
- ✅ Asset browser thumbnails (64x64 previews for textures)
- ✅ Drag-drop assets into viewport (spawn entities)
- ✅ Import settings dialog (scale, pivot, collider generation)
- ✅ Status bar (mode, selection, undo/redo, FPS, snap state)
- ✅ Keyboard shortcuts panel (Help menu)
- ✅ Recent files menu (last 10 scenes)

**User Testing Goals**:
> "I can work as fast as in Unity for basic level design tasks"

---

## 🔧 Code Quality Requirements

**Mission-Critical Standards**:

1. **Error Handling**: NO `.unwrap()` in production code
   - Use `Result<T>` with proper error propagation
   - Log errors, don't crash editor
   - Graceful degradation (e.g., missing thumbnail → show icon)

2. **Performance**: Maintain 60 FPS with 1000+ entities
   - Async thumbnail loading (don't block main thread)
   - Cache expensive operations (raycasts, mesh bounding boxes)
   - Profile regularly with Tracy

3. **Testing**: All features have integration tests
   - Undo/redo: Test command merge, branching, memory limits
   - Serialization: Test round-trip (save → load → save)
   - Clipboard: Test JSON format stability
   - Multi-selection: Test edge cases (empty, single, 100+ entities)

4. **Documentation**: Every public API has doc comments
   - Module-level docs explaining architecture
   - Examples in doc comments
   - Update ROADMAP.md after each milestone

---

## 🚧 Phase 4 Preview (6-8 weeks)

After Phase 3, tackle advanced features:

1. **Prefab System** (Week 1-3): Create/instantiate/override tracking
2. **Play-in-Editor** (Week 3-5): Play/pause/stop, snapshot world state
3. **Hot Reload** (Week 5-6): Auto-reload textures/models on file change
4. **Visual Scripting** (Week 7-8, OPTIONAL): Node-based behavior editor

**Phase 4 Success**: "I can prototype gameplay without leaving editor"

---

**Last Updated**: November 9, 2025  
**Status**: Phase 3 Ready to Implement 🎯  
**Next Action**: Start Week 1 - Multi-Selection Core
