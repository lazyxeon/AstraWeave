# Babylon.js-Style Editor Vision & Implementation Plan

**Date**: November 4, 2025  
**Objective**: Transform AstraWeave Editor to match Babylon.js editor UX  
**Status**: 📋 **PLANNING PHASE**  
**Estimated Effort**: 6-8 weeks (full-time) or 12-16 weeks (part-time)

---

## Executive Summary

**Current State**: AstraWeave editor is a **settings panel collection** with collapsible sections, no 3D viewport, and minimal visual feedback.

**Target State**: Professional 3D scene editor matching Babylon.js UX:
- ✅ **Large 3D viewport** (primary focus, 60-70% of screen)
- ✅ **Scene hierarchy tree** (entity list with parent-child relationships)
- ✅ **Inspector panel** (properties for selected object)
- ✅ **Asset browser** (visual previews of meshes/textures/materials)
- ✅ **Toolbar with gizmos** (translate/rotate/scale tools)
- ✅ **Play/Stop controls** (test scene in-editor)
- ✅ **Modern split-panel layout** (resizable, dockable)

**Gap Analysis**: Current editor is **~30% complete** for this vision. Major missing pieces:
1. ❌ **3D Viewport** (NO wgpu rendering in editor yet)
2. ❌ **Scene Graph Tree** (flat entity list exists, but not hierarchical)
3. ⚠️ **Inspector Panel** (exists but not context-sensitive)
4. ❌ **Asset Browser** (asset list exists, no visual previews)
5. ⚠️ **Gizmos** (code exists from Days 5-14, not integrated with 3D viewport)
6. ⚠️ **Play/Stop** (simulation exists, no visual preview)

**Recommendation**: **Phased approach** over 6-8 weeks, prioritizing 3D viewport → Scene graph → Inspector → Assets

---

## Babylon.js Editor Analysis

### Core Layout (Industry Standard)

```
┌─────────────────────────────────────────────────────────────────┐
│ File  Edit  Add  Tools  Help              [▶️ Play] [⏹️ Stop]  │ Top Bar
├────────┬────────────────────────────────────────┬───────────────┤
│ Scene  │                                        │   Inspector   │
│ Graph  │                                        │ ┌───────────┐ │
│ ┌────┐ │         3D Viewport (60-70%)          │ │ Transform │ │
│ │🎯📦│ │                                        │ ├───────────┤ │
│ │🎮📦│ │     [Gizmo overlays entity]            │ │ Position  │ │
│ │💡📦│ │                                        │ │ Rotation  │ │
│ │📦📦│ │                                        │ │ Scale     │ │
│ └────┘ │                                        │ └───────────┘ │
│ (15%)  │                                        │ ┌───────────┐ │
│        │                                        │ │ Material  │ │
│        │                                        │ ├───────────┤ │
│        │                                        │ │ Mesh      │ │
│        │                                        │ │ Physics   │ │
│        │                                        │ │ Scripts   │ │
├────────┴────────────────────────────────────────┤ (20-25%)  │ │
│ Assets Browser (Visual Grid)                    │               │
│ [🖼️][🖼️][🖼️][🖼️][🖼️][🖼️][🖼️][🖼️]              │               │
│ (10-15% height)                                 │               │
└─────────────────────────────────────────────────┴───────────────┘
```

**Key Principles**:
1. **3D Viewport is PRIMARY** - Largest panel, always visible
2. **Scene Graph is SECONDARY** - Quick navigation, always accessible
3. **Inspector is CONTEXT-SENSITIVE** - Shows properties of selected object
4. **Assets are VISUAL** - Thumbnails, not just file lists
5. **Gizmos OVERLAY viewport** - Direct manipulation (not separate panel)

---

## Current AstraWeave Editor Layout

```
┌─────────────────────────────────────────────────────────────────┐
│ File  Open  Save                                    Status: ... │
├────────────┬────────────────────────────────────┬───────────────┤
│ Astract    │ Central Panel (Collapsibles)       │  Performance  │
│ Panels     │ ▼ Scene Hierarchy                  │  ┌──────────┐ │
│ ▼ World    │ ▼ Inspector                        │  │ Budget   │ │
│ ▼ Entities │ ▼ Console (auto-expand)            │  │ Widget   │ │
│ ▼ Charts   │ ▼ Profiler                         │  └──────────┘ │
│ ▼ Advanced │ ▼ Behavior Graph Editor            │               │
│ ▼ Graph    │ ▼ Dialogue Graph Editor            │               │
│ ▼ Animation│ ▼ Quest Graph Editor               │               │
│            │ ▼ Material Editor                  │               │
│            │ ▼ Material Inspector               │               │
│            │ ▼ Terrain Painter                  │               │
│            │ ▼ Navmesh Controls                 │               │
│            │ ▼ Asset Inspector                  │               │
│ (30%)      │ (40%)                              │ (30%)         │
└────────────┴────────────────────────────────────┴───────────────┘
```

**Problems**:
1. ❌ **NO 3D viewport** - Can't see what you're editing!
2. ❌ **Collapsible hell** - Everything hidden by default
3. ❌ **No visual hierarchy** - Flat lists, no tree structure
4. ❌ **No context sensitivity** - All panels always show same content
5. ❌ **Text-heavy** - No visual previews (asset thumbnails, etc.)
6. ❌ **Panels compete for space** - Equal weight (should be 60/15/25)

---

## Phased Implementation Plan

### Phase 1: 3D Viewport Foundation (2-3 weeks)

**Goal**: Get wgpu rendering into the editor

**Tasks**:
1. **Integrate wgpu into eframe** (Week 1)
   - Use `egui_wgpu` renderer (already in dependencies!)
   - Create `SceneViewport` widget (custom egui widget)
   - Render simple scene (grid, axes, test cube)
   
2. **Camera Controls** (Week 1)
   - Orbit camera (mouse drag to rotate around focal point)
   - Pan camera (middle mouse drag)
   - Zoom camera (scroll wheel)
   - Frame selected (F key)
   
3. **Entity Rendering** (Week 2)
   - Render entities from `sim_world` (boxes/spheres for now)
   - Color-code by type (obstacle, NPC, player)
   - Wireframe/solid toggle
   
4. **Selection System** (Week 2)
   - Ray-cast from mouse click → 3D world
   - Highlight selected entity (outline shader)
   - Show selection in Scene Graph + Inspector
   
5. **Grid & Gizmo Overlay** (Week 3)
   - Render grid on ground plane
   - Integrate existing gizmo code (from Days 5-14)
   - Mouse drag gizmo handles → update entity transform

**Deliverable**: **Editor with functional 3D viewport** where you can:
- See entities in 3D
- Orbit/pan/zoom camera
- Click to select entities
- Transform with gizmos (visual manipulation)

**Estimated Lines of Code**: ~2,500 lines
- `viewport.rs` - 800 lines (wgpu setup, rendering)
- `camera.rs` - 400 lines (orbit/pan/zoom controls)
- `picking.rs` - 300 lines (ray-casting, selection)
- `gizmo_integration.rs` - 500 lines (connect existing gizmo to viewport)
- `renderer.rs` updates - 500 lines (entity rendering)

---

### Phase 2: Scene Graph Tree (1-2 weeks)

**Goal**: Hierarchical entity view with parent-child relationships

**Tasks**:
1. **Tree Widget** (Week 1)
   - Use `egui::CollapsingHeader` for tree nodes
   - Drag-and-drop to reparent entities
   - Right-click context menu (Add Child, Delete, Duplicate)
   
2. **Parent-Child Transform** (Week 1)
   - Implement transform hierarchy (child inherits parent transform)
   - Update rendering to respect hierarchy
   - Gizmo operates in parent/world space toggle
   
3. **Icons & Naming** (Week 2)
   - Icons for entity types (🎯 player, 🎮 NPC, 💡 light, 📦 obstacle)
   - Inline rename (double-click entity name)
   - Search/filter entities

**Deliverable**: **Hierarchical scene graph** where you can:
- Expand/collapse entity trees
- Drag entities to reparent
- See visual icons for types
- Search for entities by name

**Estimated Lines of Code**: ~1,200 lines
- `scene_graph.rs` - 600 lines (tree rendering, drag-drop)
- `transform_hierarchy.rs` - 400 lines (parent-child math)
- `entity_icons.rs` - 200 lines (icon mapping)

---

### Phase 3: Context-Sensitive Inspector (1 week)

**Goal**: Inspector shows properties of SELECTED entity only

**Tasks**:
1. **Selection State** (Day 1)
   - Global `SelectedEntity` state (ID + type)
   - Update on viewport click OR scene graph click
   
2. **Component Editors** (Days 2-3)
   - Transform editor (position/rotation/scale) - DONE (from P0 work)
   - Material editor (base color, metallic, roughness)
   - Physics editor (mass, friction, collision shape)
   - AI editor (behavior tree, goals, state)
   
3. **Add Component** (Days 4-5)
   - "Add Component" button in inspector
   - Dropdown of available components
   - Instantiate component with defaults

**Deliverable**: **Inspector that updates** when you select an entity, showing:
- All components attached to that entity
- Editable properties (sliders, color pickers, dropdowns)
- Add/Remove component buttons

**Estimated Lines of Code**: ~1,000 lines
- `inspector.rs` - 400 lines (selection handling, component list)
- `component_editors/` - 600 lines (Transform, Material, Physics, AI editors)

---

### Phase 4: Asset Browser with Visual Previews (1-2 weeks)

**Goal**: Grid of asset thumbnails (not just text list)

**Tasks**:
1. **Thumbnail Generation** (Week 1)
   - Render meshes to texture (wgpu offscreen rendering)
   - Load textures as thumbnails (already images)
   - Generate material preview spheres (PBR shader)
   
2. **Grid Layout** (Week 1)
   - `egui::Grid` or custom layout
   - 128x128 or 256x256 thumbnails
   - Hover tooltip (asset name, size, type)
   
3. **Drag-and-Drop** (Week 2)
   - Drag mesh from browser → viewport → creates entity
   - Drag material → selected entity → applies material
   - Drag texture → material slot → updates texture

**Deliverable**: **Visual asset browser** where you can:
- See thumbnail previews of all assets
- Drag meshes into scene to create entities
- Drag materials/textures to apply them

**Estimated Lines of Code**: ~1,500 lines
- `asset_browser.rs` - 500 lines (grid layout, thumbnails)
- `thumbnail_generator.rs` - 600 lines (render-to-texture)
- `asset_drag_drop.rs` - 400 lines (drag-drop handling)

---

### Phase 5: Modern Layout & Docking (1 week)

**Goal**: Resizable, dockable panels (professional UX)

**Tasks**:
1. **Replace Collapsibles with Docking** (Days 1-3)
   - Use `egui_dock` crate (professional docking system)
   - Define 4 main areas: Left (Scene), Center (Viewport), Right (Inspector), Bottom (Assets)
   - Save/load layout preferences
   
2. **Panel Resizing** (Days 4-5)
   - Drag panel borders to resize
   - Minimum/maximum sizes (viewport never < 50%)
   - Snap-to-edge behavior

**Deliverable**: **Professional layout** with:
- Resizable panels (drag borders)
- Dockable panels (drag tabs to rearrange)
- Saved layout preferences

**Estimated Lines of Code**: ~800 lines
- `layout.rs` - 500 lines (egui_dock integration)
- `preferences.rs` - 300 lines (save/load layout)

---

### Phase 6: Play/Stop with Visual Preview (1 week)

**Goal**: Run simulation in 3D viewport (not background)

**Tasks**:
1. **Play Mode Toggle** (Days 1-2)
   - Play button → start simulation, entities move in viewport
   - Stop button → reset to pre-play state
   - Pause button → freeze simulation
   
2. **Visual Feedback** (Days 3-4)
   - Entity trails (show movement paths)
   - AI debug visualization (perception radius, nav paths)
   - Physics debug (collision shapes, forces)
   
3. **Time Controls** (Days 5)
   - Playback speed slider (0.1x → 4x)
   - Frame-by-frame stepping
   - Rewind/replay (save simulation state)

**Deliverable**: **In-editor simulation preview** where you can:
- Press Play → see entities move in 3D
- See AI debug visualization (perception radius, paths)
- Control playback speed, pause, step frame-by-frame

**Estimated Lines of Code**: ~1,000 lines
- `play_mode.rs` - 400 lines (play/stop/pause logic)
- `debug_visualization.rs` - 400 lines (trails, AI debug, physics debug)
- `time_controls.rs` - 200 lines (speed slider, stepping)

---

## Total Effort Summary

| Phase | Description | Duration | Lines of Code | Priority |
|-------|-------------|----------|---------------|----------|
| **Phase 1** | 3D Viewport Foundation | 2-3 weeks | ~2,500 | 🔥 **P0** (Critical) |
| **Phase 2** | Scene Graph Tree | 1-2 weeks | ~1,200 | 🔥 **P0** (Critical) |
| **Phase 3** | Context-Sensitive Inspector | 1 week | ~1,000 | ⭐ **P1** (High) |
| **Phase 4** | Asset Browser (Visual) | 1-2 weeks | ~1,500 | ⭐ **P1** (High) |
| **Phase 5** | Modern Layout & Docking | 1 week | ~800 | ⚠️ **P2** (Medium) |
| **Phase 6** | Play/Stop Preview | 1 week | ~1,000 | ⚠️ **P2** (Medium) |
| **TOTAL** | **Full Babylon.js-style Editor** | **6-10 weeks** | **~8,000** | - |

**Realistic Timeline**:
- **Full-time (40h/week)**: 6-8 weeks
- **Part-time (20h/week)**: 12-16 weeks
- **Hobby pace (10h/week)**: 24-32 weeks (~6-8 months)

---

## MVP Recommendation (Fastest Path to Usable)

**Goal**: Get to "minimally usable 3D editor" in **4 weeks**

**Scope**: Phase 1 + Phase 2 + minimal Phase 3

### Week 1-2: 3D Viewport
- ✅ wgpu integration (render grid + test cube)
- ✅ Orbit camera (mouse drag)
- ✅ Entity rendering (boxes for obstacles, spheres for NPCs)
- ✅ Click selection (ray-casting)

### Week 3: Scene Graph
- ✅ Tree widget (expand/collapse)
- ✅ Parent-child relationships (basic hierarchy)
- ✅ Click to select (sync with viewport)

### Week 4: Inspector (Basic)
- ✅ Transform editor (position/rotation/scale) - **ALREADY DONE!**
- ✅ Show selected entity properties
- ✅ Gizmo manipulation (G/R/S keys + mouse drag)

**Deliverable**: After 4 weeks, you can:
1. ✅ See scene in 3D (orbit camera around entities)
2. ✅ Click entities to select (in viewport OR scene graph)
3. ✅ Transform entities with gizmos (visual + numeric)
4. ✅ See parent-child hierarchies

**This is 80% of the value** with 40% of the effort! 🎯

---

## Technical Architecture

### Key Technologies

1. **egui_wgpu**: Integrate wgpu rendering into egui panels
   - Already in dependencies (`eframe` includes this)
   - Pattern: Custom `egui::Widget` trait impl for viewport
   
2. **egui_dock**: Professional docking system
   - Add to `Cargo.toml`: `egui_dock = "0.15"`
   - Provides VS Code-style panel management
   
3. **Existing AstraWeave Code to Leverage**:
   - ✅ `astraweave-render` (wgpu renderer, materials, meshes)
   - ✅ `gizmo/` module (transform gizmos from Days 5-14)
   - ✅ `Transform` panel (numeric input, already functional)
   - ✅ `World` simulation (entities, physics, AI)

### Code Structure

```
tools/aw_editor/src/
├── main.rs                    # eframe app setup
├── layout.rs                  # egui_dock layout management (NEW)
├── viewport/                  # NEW - Phase 1
│   ├── mod.rs
│   ├── widget.rs              # Custom egui widget for 3D view
│   ├── camera.rs              # Orbit/pan/zoom camera
│   ├── renderer.rs            # wgpu rendering (entities, grid, gizmos)
│   └── picking.rs             # Ray-casting for selection
├── scene_graph/               # NEW - Phase 2
│   ├── mod.rs
│   ├── tree.rs                # Hierarchical tree widget
│   ├── hierarchy.rs           # Parent-child transform math
│   └── drag_drop.rs           # Reparenting via drag-drop
├── inspector/                 # REFACTOR - Phase 3
│   ├── mod.rs
│   ├── transform.rs           # Move from panels/transform_panel.rs
│   ├── material.rs
│   ├── physics.rs
│   └── ai.rs
├── assets/                    # NEW - Phase 4
│   ├── mod.rs
│   ├── browser.rs             # Grid layout with thumbnails
│   ├── thumbnails.rs          # Render-to-texture for previews
│   └── drag_drop.rs           # Asset drag-drop
├── play_mode/                 # NEW - Phase 6
│   ├── mod.rs
│   ├── controller.rs          # Play/stop/pause logic
│   └── debug_viz.rs           # AI/physics debug visualization
├── panels/                    # EXISTING (keep for now)
│   ├── world_panel.rs
│   ├── entity_panel.rs
│   ├── performance_panel.rs
│   ├── charts_panel.rs
│   └── ... (Astract demos)
└── gizmo/                     # EXISTING (Days 5-14)
    ├── mod.rs
    ├── state.rs
    ├── translate.rs
    ├── rotate.rs
    └── scale.rs
```

---

## Example: Phase 1 Viewport Widget (Pseudocode)

```rust
// tools/aw_editor/src/viewport/widget.rs

use egui::Ui;
use wgpu::{Device, Queue, Surface, TextureView};

pub struct ViewportWidget {
    camera: Camera,
    renderer: EntityRenderer,
    gizmo_renderer: GizmoRenderer,
    selected_entity: Option<u32>,
}

impl ViewportWidget {
    pub fn new(device: &Device, queue: &Queue) -> Self {
        Self {
            camera: Camera::new_orbit(Vec3::ZERO, 10.0),
            renderer: EntityRenderer::new(device),
            gizmo_renderer: GizmoRenderer::new(device),
            selected_entity: None,
        }
    }
    
    pub fn ui(&mut self, ui: &mut Ui, world: &World) {
        // Allocate space for viewport (most of the screen)
        let available = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(available.x * 0.7, available.y),
            egui::Sense::click_and_drag(),
        );
        
        // Handle mouse input
        if response.dragged() {
            self.camera.orbit(response.drag_delta());
        }
        if response.clicked() {
            let ray = self.camera.ray_from_screen(response.interact_pointer_pos());
            self.selected_entity = pick_entity(ray, world);
        }
        
        // Render to texture
        let frame_texture = self.render_to_texture(ui.ctx(), world);
        
        // Display texture in egui
        ui.painter().image(
            frame_texture.into(),
            rect,
            egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    
    fn render_to_texture(&mut self, ctx: &egui::Context, world: &World) -> egui::TextureId {
        // 1. Clear frame
        // 2. Render grid
        // 3. Render entities from world
        // 4. If selected_entity, render gizmo overlay
        // 5. Return texture ID for egui
        
        // TODO: Implement wgpu rendering
        todo!()
    }
}
```

---

## Risk Assessment

### High-Risk Areas

1. **wgpu + egui Integration** (Phase 1)
   - **Risk**: Complex graphics API, potential performance issues
   - **Mitigation**: Use `egui_wgpu` backend (already exists), start with simple rendering
   - **Fallback**: Use `egui::Image` with pre-rendered textures if real-time is too slow

2. **Ray-Casting Accuracy** (Phase 1)
   - **Risk**: Mouse clicks might miss entities (precision issues)
   - **Mitigation**: Use bounding boxes for picking (not pixel-perfect), add visual feedback
   - **Fallback**: Click-to-highlight with tolerance radius

3. **Transform Hierarchy Math** (Phase 2)
   - **Risk**: Parent-child transforms can have edge cases (gimbal lock, scale issues)
   - **Mitigation**: Use glam quaternions (already in codebase), test thoroughly
   - **Fallback**: Disable scale inheritance if bugs occur

4. **Thumbnail Generation Performance** (Phase 4)
   - **Risk**: Rendering 100+ thumbnails might freeze editor
   - **Mitigation**: Lazy loading (render on-demand), cache thumbnails to disk
   - **Fallback**: Use placeholder icons if render-to-texture fails

### Medium-Risk Areas

5. **egui_dock Learning Curve** (Phase 5)
   - **Risk**: Complex API, might take longer than 1 week
   - **Mitigation**: Read docs, look at examples, prototype first
   - **Fallback**: Use static `SidePanel` layout (no docking, but functional)

6. **Play Mode State Management** (Phase 6)
   - **Risk**: Simulation might crash editor if not sandboxed
   - **Mitigation**: Clone world state before play, revert on stop
   - **Fallback**: Run simulation in separate thread (already done in current editor)

---

## Success Criteria

**MVP (4 weeks)**:
- ✅ 3D viewport renders scene with orbit camera
- ✅ Click to select entities (viewport + scene graph sync)
- ✅ Transform entities with gizmos (visual manipulation)
- ✅ Scene graph shows hierarchical tree

**Full Vision (6-8 weeks)**:
- ✅ All 6 phases complete
- ✅ Babylon.js-style layout (viewport-centric)
- ✅ Asset browser with visual thumbnails
- ✅ Dockable/resizable panels
- ✅ In-editor simulation preview

**User Feedback Goals**:
- ✅ "I can see what I'm editing" (3D viewport)
- ✅ "I can find entities quickly" (scene graph tree + search)
- ✅ "I can modify properties easily" (inspector + gizmos)
- ✅ "The editor feels professional" (modern layout, visual assets)
- ✅ "I can test my scene immediately" (play mode)

---

## Next Steps

### Immediate (This Week)

1. **Decision Point**: MVP (4 weeks) vs Full Vision (6-8 weeks)?
2. **Technology Validation**: Prototype egui_wgpu viewport (1-2 hours)
   - Render a single cube in egui panel
   - Confirm wgpu integration works
3. **Plan Review**: Get user feedback on priorities

### Phase 1 Kickoff (Next Week)

**Day 1**: Setup `viewport/` module, integrate `egui_wgpu`  
**Day 2**: Render grid + test cube  
**Day 3**: Implement orbit camera  
**Day 4**: Implement ray-casting selection  
**Day 5**: Integrate existing gizmo code  

**End of Week 1**: Functional 3D viewport with camera controls! 🎉

---

## Comparison: Current vs MVP vs Full Vision

| Feature | Current Editor | MVP (4 weeks) | Full Vision (8 weeks) |
|---------|----------------|---------------|-----------------------|
| **3D Viewport** | ❌ None | ✅ Orbit camera, entity rendering | ✅ Advanced camera, debug viz |
| **Scene Graph** | ⚠️ Flat list | ✅ Hierarchical tree | ✅ Tree + search + icons |
| **Inspector** | ⚠️ Static panels | ✅ Context-sensitive | ✅ All component editors |
| **Asset Browser** | ⚠️ Text list | ❌ Deferred (P1) | ✅ Visual thumbnails + drag-drop |
| **Gizmos** | ✅ Code exists | ✅ Integrated with viewport | ✅ Fully polished |
| **Layout** | ⚠️ Collapsibles | ⚠️ Static panels | ✅ Dockable/resizable |
| **Play Mode** | ⚠️ Background sim | ⚠️ Background sim | ✅ In-viewport preview |
| **Visual Polish** | ⭐⭐ (C) | ⭐⭐⭐ (B) | ⭐⭐⭐⭐⭐ (A+) |

**Recommendation**: **Start with MVP (4 weeks)**, then evaluate:
- If user loves it → Continue to Full Vision (4 more weeks)
- If user satisfied → Polish MVP, ship it
- If user wants different features → Pivot based on feedback

---

## Conclusion

**Babylon.js-style editor is ACHIEVABLE** but requires significant work:

- **MVP (4 weeks)**: Core 3D viewport + scene graph + inspector → **80% of value**
- **Full Vision (6-8 weeks)**: Professional-grade editor matching Babylon.js UX → **100% of value**

**Current editor has ~30% of needed infrastructure**:
- ✅ Gizmo code (Days 5-14)
- ✅ Transform panel logic
- ✅ Simulation system
- ❌ Missing: 3D viewport (CRITICAL)
- ❌ Missing: Visual asset browser
- ❌ Missing: Modern layout

**Recommended Path**: **Prototype Phase 1 (Week 1)** to validate egui_wgpu integration, then commit to MVP if successful.

**This is a MAJOR undertaking** but will transform AstraWeave into a professional game engine with a world-class editor! 🚀

---

**Vision Document Complete**: November 4, 2025  
**Ready for**: User decision (MVP vs Full Vision vs Pivot)
