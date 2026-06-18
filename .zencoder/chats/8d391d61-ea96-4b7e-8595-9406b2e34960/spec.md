# Technical Specification: AstraWeave Visual Editor Enhancement

**Version**: 1.0  
**Date**: January 8, 2026  
**Status**: Ready for Implementation  
**Based on**: requirements.md (Product Requirements Document v1.0)

---

## Technical Context

### Language & Toolchain
- **Language**: Rust 2021 (toolchain 1.89.0)
- **Build System**: Cargo workspace (130 members) <!-- Source: CLAIMS_REGISTRY.md#workspace-members -->
- **Editor Package**: `tools/aw_editor` (workspace member)

### Primary Dependencies

**UI Framework**:
- `eframe` 0.32.0 - Application framework (wraps egui + wgpu + winit)
- `egui` 0.32.0 - Immediate mode GUI library
- `egui-wgpu` 0.32.0 - wgpu backend for egui rendering
- `egui_dock` (workspace dependency) - Panel docking system **(NOT YET INTEGRATED)**
- `egui_graphs` (workspace dependency) - Node graph editor for materials/behavior

**Graphics**:
- `wgpu` 25.0.2 - WebGPU implementation (Vulkan/DX12/Metal backends)
- `winit` 0.30 - Cross-platform windowing

**AstraWeave Engine Crates**:
- `astraweave-core` - ECS World, Entity, Pose, Team, Health components
- `astraweave-render` - Rendering pipeline (PBR, VXGI, MegaLights, GPU particles)
- `astraweave-asset` - AssetDatabase, file watching, hot-reload
- `astraweave-physics` - Rapier3D integration, character controllers
- `astraweave-behavior` - BehaviorGraph, GOAP, HTN planning
- `astraweave-materials` - Material system (PBR properties, textures)
- `astraweave-nav` - NavMesh, A* pathfinding
- `astraweave-scene` - Scene graph, serialization
- `astraweave-security` - Path validation, safe file operations

**Serialization**:
- `serde` 1.x + `serde_json` + `ron` 0.8 - Data serialization
- Scene format: `.scene.ron` (Rusty Object Notation)
- Material format: `.mat.ron`
- Prefab format: `.prefab.ron`

**File System**:
- `notify` 8.0 - File watching for hot-reload
- `walkdir` 2.0 - Directory traversal for asset scanning

**Profiling** (to be added):
- `tracy-client` - Integration with Tracy profiler (FR-7.2)

**File Dialogs** (to be added):
- `rfd` 0.14 - Native file dialogs (Open/Save/Browse)

**Version Control** (P2, optional):
- `git2` 0.18 - libgit2 bindings for Git integration (FR-9)

---

## Technical Implementation Brief

### Current State Analysis

**Existing Architecture** (`tools/aw_editor/src/main.rs:240-336`):
- **EditorApp**: Monolithic struct with 50+ fields
  - 16 panel instances (HierarchyPanel, EntityPanel, AssetBrowser, etc.)
  - Scene state management (EditorSceneState, World)
  - Undo/redo system (UndoStack with 100 command history)
  - Viewport integration (ViewportWidget with wgpu rendering)
  - Gizmo system (12 modules: translate, rotate, scale, picking, constraints, etc.)
  - Play-in-Editor (EditorMode: Edit/Play/Pause with F5-F8 hotkeys)
  - Prefab system (PrefabManager)
  - Recent files tracking (RecentFilesManager)
  - Auto-save system (5-minute interval)

**Critical Issues**:
1. **Compilation Failure**: Editor does not build (wgpu/egui/winit API migration issues)
2. **No Panel Docking**: UI uses fixed `egui::SidePanel`/`egui::CentralPanel` layout (inflexible)
3. **egui_dock Available but Unused**: Dependency exists but not integrated

**Key Strengths**:
- Comprehensive module structure (behavior_graph, gizmo, viewport, panels, etc.)
- Command pattern for undo/redo already implemented
- Scene serialization working (SceneData ↔ World conversion)
- ViewportWidget handles wgpu + egui integration correctly
- 12-module gizmo system with G/R/S keyboard shortcuts (Blender-style)

### Implementation Strategy

**Guiding Principles**:
1. **Iterative Migration**: Fix compilation → Restore functionality → Add docking → Enhance features
2. **Preserve Existing Code**: Leverage 67 source files (gizmo/, viewport/, panels/, scene_serialization.rs, etc.)
3. **Incremental Value**: Each delivery phase must be testable end-to-end
4. **Performance-First**: Maintain 60 FPS with 1000 entities (current: 370 FPS @ 1k, must not regress)

**Critical Technical Decisions**:

1. **Panel Docking System**:
   - **Choice**: Use `egui_dock::DockArea` (already in dependencies)
   - **Migration Path**: Replace root-level `egui::SidePanel` calls with `egui_dock::Tree`
   - **Layout Persistence**: Serialize `DockState` to `editor_layout.ron` in preferences directory

2. **Material Editor**:
   - **Choice**: Use `egui_graphs` for node graph UI (already in dependencies)
   - **Shader Generation**: Implement graph → WGSL transpiler (custom codegen module)
   - **Preview Integration**: Reuse existing `brdf_preview::BrdfPreviewWidget` (already exists)

3. **Asset Import Pipeline**:
   - **Choice**: Async tokio tasks with progress channels
   - **File Watching**: Extend existing `file_watcher` module (uses `notify` crate)
   - **Import Jobs**: Queue-based system with `AssetImportJob` trait

4. **Profiling Integration**:
   - **Choice**: Tracy client library with in-editor flamegraph viewer
   - **Data Collection**: Hook into existing profiling markers (astraweave-profiling crate has 94.12% coverage)

---

## Source Code Structure

### Modified Files (Existing Code)

```
tools/aw_editor/src/
├── main.rs                         [MODIFY] - Replace panel layout with egui_dock::DockArea
├── panels/
│   ├── mod.rs                       [MODIFY] - Add new panel types (Material, ProjectSettings, etc.)
│   ├── hierarchy_panel.rs           [ENHANCE] - Add drag-drop reordering
│   ├── entity_panel.rs              [ENHANCE] - Expand component inspector UI
│   ├── asset_browser.rs             [ENHANCE] - Add folder tree, drag-drop import
│   ├── console_panel.rs             [ENHANCE] - Virtual scrolling for 10k+ lines
│   ├── profiler_panel.rs            [ENHANCE] - Tracy flamegraph integration
│   └── build_manager.rs             [ENHANCE] - Multi-platform target selection
├── viewport/
│   ├── widget.rs                    [ENHANCE] - Add viewport toolbar (camera modes, debug draw)
│   ├── camera.rs                    [ENHANCE] - Camera bookmarks (F1-F12), ortho modes
│   └── toolbar.rs                   [EXISTS] - Extend with new debug draw modes
├── gizmo/
│   ├── rendering.rs                 [ENHANCE] - Add scene gizmos (lights, cameras, audio)
│   └── snapping.rs                  [ENHANCE] - UI for grid size configuration
├── material_inspector.rs            [ENHANCE] - Upgrade to full material graph editor
├── brdf_preview.rs                  [ENHANCE] - Add environment rotation controls
├── scene_serialization.rs           [MODIFY] - Add material/prefab serialization support
├── command.rs                       [ENHANCE] - Extend UndoStack to support all editor actions
├── editor_preferences.rs            [MODIFY] - Add layout persistence (DockState serialization)
└── runtime.rs                       [ENHANCE] - Communication channel for live debugging
```

### New Files (To Be Created)

```
tools/aw_editor/src/
├── panels/
│   ├── material_editor_panel.rs     [NEW] - Node-based material graph UI
│   ├── project_settings_panel.rs    [NEW] - Physics layers, input mappings, tags
│   ├── asset_import_panel.rs        [NEW] - Import job queue, progress visualization
│   └── timeline_panel.rs            [NEW] - Animation timeline (Phase 4, optional)
├── material/
│   ├── mod.rs                       [NEW] - Material graph system root
│   ├── graph.rs                     [NEW] - MaterialGraph data structure
│   ├── nodes.rs                     [NEW] - Node library (TextureSample, Math, PBR, etc.)
│   ├── compiler.rs                  [NEW] - Graph → WGSL transpiler
│   └── validation.rs                [NEW] - Type checking, cycle detection
├── asset_pipeline/
│   ├── mod.rs                       [NEW] - Asset import system root
│   ├── import_job.rs                [NEW] - AssetImportJob trait + queue
│   ├── fbx_importer.rs              [NEW] - FBX mesh import (via assimp or gltf converter)
│   ├── gltf_importer.rs             [NEW] - GLTF scene import (uses astraweave-render GLTF)
│   └── texture_importer.rs          [NEW] - PNG/JPG import with mipmap generation
├── profiling/
│   ├── mod.rs                       [NEW] - Tracy integration root
│   ├── client.rs                    [NEW] - Tracy client wrapper
│   └── flamegraph.rs                [NEW] - Flamegraph visualization widget
└── scene_gizmos/
    ├── mod.rs                       [NEW] - Scene gizmo rendering
    ├── light_gizmo.rs               [NEW] - Point/directional light visualization
    ├── camera_gizmo.rs              [NEW] - Camera frustum gizmo
    └── audio_gizmo.rs               [NEW] - Audio source sphere gizmo
```

### Dependency Additions

**Cargo.toml Changes**:
```toml
[dependencies]
# ... existing dependencies ...

# FR-8.2: Native file dialogs for asset import
rfd = "0.14"

# FR-7.2: Tracy profiler integration (optional feature)
tracy-client = { version = "0.17", optional = true }

# FR-9.1: Git integration (optional, P2 feature)
git2 = { version = "0.18", optional = true }

[features]
default = ["editor-core"]
editor-core = ["astraweave-render"]
profiling = ["tracy-client", "astraweave-profiling/profiling"]
git-integration = ["git2"]
```

---

## Contracts

### Data Models

#### DockState Serialization (FR-1.3)
```rust
// tools/aw_editor/src/editor_preferences.rs

#[derive(Serialize, Deserialize)]
pub struct EditorPreferences {
    pub layout: DockLayout,
    pub recent_files: Vec<PathBuf>,
    pub auto_save_enabled: bool,
    pub auto_save_interval_secs: f32,
    pub viewport_settings: ViewportSettings,
}

#[derive(Serialize, Deserialize)]
pub struct DockLayout {
    /// Serialized egui_dock::Tree structure
    pub tree: String, // JSON-encoded DockState
}

#[derive(Serialize, Deserialize)]
pub struct ViewportSettings {
    pub show_grid: bool,
    pub grid_size: f32,
    pub camera_speed: f32,
    pub camera_bookmarks: [Option<CameraBookmark>; 12], // F1-F12
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CameraBookmark {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
}
```

#### Material Graph Data Model (FR-3)
```rust
// tools/aw_editor/src/material/graph.rs

use egui_graphs::{Graph, Node};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct MaterialGraph {
    pub nodes: Vec<MaterialNode>,
    pub edges: Vec<MaterialEdge>,
    pub output_node: NodeId, // Must be MaterialOutputNode
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MaterialNode {
    pub id: NodeId,
    pub node_type: MaterialNodeType,
    pub position: (f32, f32), // Graph editor position
}

#[derive(Clone, Serialize, Deserialize)]
pub enum MaterialNodeType {
    // Inputs
    TextureSample { texture_path: String, uv_channel: u8 },
    ColorConstant { color: [f32; 4] },
    FloatConstant { value: f32 },
    Vec3Constant { value: [f32; 3] },
    
    // Math Operations
    Add,
    Multiply,
    Lerp,
    Clamp,
    
    // PBR Properties
    PbrOutput {
        base_color: InputSlot,
        metallic: InputSlot,
        roughness: InputSlot,
        normal: InputSlot,
        emissive: InputSlot,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MaterialEdge {
    pub from_node: NodeId,
    pub from_output: OutputSlot,
    pub to_node: NodeId,
    pub to_input: InputSlot,
}

pub type NodeId = usize;
pub type InputSlot = usize;
pub type OutputSlot = usize;
```

#### Asset Import Job Contract (FR-8.2)
```rust
// tools/aw_editor/src/asset_pipeline/import_job.rs

use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub struct ImportProgress {
    pub job_id: usize,
    pub status: ImportStatus,
    pub progress: f32, // 0.0 - 1.0
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum ImportStatus {
    Queued,
    Processing,
    Completed,
    Failed { error: String },
}

#[async_trait::async_trait]
pub trait AssetImporter: Send + Sync {
    /// File extensions this importer handles (e.g., ["fbx", "obj"])
    fn extensions(&self) -> &[&str];
    
    /// Import asset from source path to assets directory
    async fn import(
        &self,
        source_path: &Path,
        asset_db: &mut AssetDatabase,
        progress: mpsc::Sender<ImportProgress>,
    ) -> Result<PathBuf>;
}

pub struct AssetImportQueue {
    jobs: Vec<ImportJob>,
    importers: Vec<Box<dyn AssetImporter>>,
    progress_tx: mpsc::Sender<ImportProgress>,
}

impl AssetImportQueue {
    pub async fn enqueue(&mut self, source_path: PathBuf) -> Result<usize>;
    pub async fn process_jobs(&mut self, asset_db: &mut AssetDatabase) -> Result<()>;
}
```

#### Project Settings Schema (FR-8.1)
```rust
// tools/aw_editor/src/project_settings.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub physics: PhysicsSettings,
    pub input: InputSettings,
    pub tags: TagsSettings,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PhysicsSettings {
    /// 32 physics layers (bitfield)
    pub layers: [String; 32],
    /// Layer collision matrix (32x32)
    pub collision_matrix: [[bool; 32]; 32],
}

#[derive(Clone, Serialize, Deserialize)]
pub struct InputSettings {
    pub keyboard_mappings: HashMap<String, winit::keyboard::KeyCode>,
    pub gamepad_mappings: HashMap<String, GamepadButton>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TagsSettings {
    pub tags: Vec<String>, // User-defined tags for entities/assets
}

// Saved to: project_root/project_settings.ron
```

### API Changes

#### Viewport Toolbar Extension (FR-2.1, FR-2.2)
```rust
// tools/aw_editor/src/viewport/widget.rs

impl ViewportWidget {
    /// Render viewport toolbar (camera modes, debug draw toggles)
    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Camera mode dropdown
            egui::ComboBox::from_id_source("camera_mode")
                .selected_text(format!("{:?}", self.camera_mode))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.camera_mode, CameraMode::Perspective, "Perspective");
                    ui.selectable_value(&mut self.camera_mode, CameraMode::Top, "Top (Ortho)");
                    ui.selectable_value(&mut self.camera_mode, CameraMode::Front, "Front (Ortho)");
                    ui.selectable_value(&mut self.camera_mode, CameraMode::Side, "Side (Ortho)");
                });
            
            // Debug draw modes
            ui.toggle_value(&mut self.debug_options.wireframe, "🔲 Wireframe");
            ui.toggle_value(&mut self.debug_options.show_colliders, "🔳 Colliders");
            ui.toggle_value(&mut self.debug_options.show_navmesh, "🗺 NavMesh");
            
            // Grid settings
            if ui.button("⚙ Grid").clicked() {
                self.show_grid_settings = true;
            }
        });
    }
}

#[derive(Clone, Copy)]
pub enum CameraMode {
    Perspective,
    Top,    // Orthographic looking down -Y
    Front,  // Orthographic looking down -Z
    Side,   // Orthographic looking down -X
}
```

#### Material Graph Compiler Interface (FR-3.2)
```rust
// tools/aw_editor/src/material/compiler.rs

pub struct MaterialCompiler;

impl MaterialCompiler {
    /// Compile MaterialGraph to WGSL fragment shader code
    pub fn compile(graph: &MaterialGraph) -> Result<String, CompileError> {
        // 1. Validate graph (no cycles, type compatibility)
        Self::validate(graph)?;
        
        // 2. Topological sort (dependency order)
        let sorted_nodes = Self::topological_sort(graph)?;
        
        // 3. Generate WGSL code
        let mut wgsl = String::from("@fragment\nfn fragment_main() -> @location(0) vec4<f32> {\n");
        for node_id in sorted_nodes {
            let code = Self::generate_node_code(&graph.nodes[node_id])?;
            wgsl.push_str(&code);
        }
        wgsl.push_str("}\n");
        
        Ok(wgsl)
    }
    
    fn validate(graph: &MaterialGraph) -> Result<(), CompileError>;
    fn topological_sort(graph: &MaterialGraph) -> Result<Vec<NodeId>, CompileError>;
    fn generate_node_code(node: &MaterialNode) -> Result<String, CompileError>;
}

#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("Cycle detected in graph")]
    CycleDetected,
    #[error("Type mismatch: {0}")]
    TypeMismatch(String),
    #[error("Missing output node")]
    NoOutputNode,
    #[error("Invalid connection: {0}")]
    InvalidConnection(String),
}
```

---

## Delivery Phases

### Phase 1: Critical Path (Weeks 1-4) - Foundation

**Deliverable 1.1: Fix Compilation & Restore Functionality**  
**Estimated Effort**: 1 week  
**Priority**: P0 (CRITICAL)

**Tasks**:
1. Investigate wgpu 25.0.2, egui 0.32, winit 0.30 API migration issues
2. Fix compilation errors in:
   - `viewport/widget.rs` (wgpu surface API changes)
   - `gizmo/rendering.rs` (egui-wgpu integration)
   - `brdf_preview.rs` (shader compilation)
3. Run smoke test checklist (30 test cases):
   - Launch editor ✓
   - Load scene ✓
   - Select entity → inspector shows components ✓
   - Transform gizmo (G/R/S) ✓
   - Save scene ✓
   - Undo/redo ✓
   - Play-in-Editor (F5/F6/F7/F8) ✓
   - Asset browser loads ✓
   - Console logs visible ✓
   - Hierarchy panel shows entities ✓

**Acceptance Criteria**:
- `cargo build -p aw_editor --release` completes with 0 errors, 0 warnings
- Smoke test checklist passes (30/30 tests)

---

**Deliverable 1.2: Panel Docking System**  
**Estimated Effort**: 1 week  
**Priority**: P1-A

**Tasks**:
1. Refactor `main.rs::EditorApp::update()` to use `egui_dock::DockArea` instead of fixed `SidePanel`
2. Create initial layout:
   - Left dock: Hierarchy + Asset Browser (tabs)
   - Center: Viewport + Scene View (tabs)
   - Right dock: Inspector + Transform + Console (tabs)
   - Bottom dock: Console + Profiler + Build Manager (tabs)
3. Implement layout save/load:
   - Serialize `egui_dock::Tree` to `editor_layout.ron`
   - Load on startup, save on exit + Ctrl+S
4. Add "Window" menu:
   - Reset Layout to Default
   - Save Layout As...
   - Load Layout...

**Code Changes**:
```rust
// tools/aw_editor/src/main.rs

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Top menu bar
        self.render_menu_bar(ctx);
        
        // Main dockable area
        egui::CentralPanel::default().show(ctx, |ui| {
            egui_dock::DockArea::new(&mut self.dock_state)
                .show(ctx, &mut MyTabViewer {
                    panels: &mut self.panels,
                    viewport: &mut self.viewport,
                });
        });
    }
}

struct MyTabViewer<'a> {
    panels: &'a mut EditorPanels,
    viewport: &'a mut Option<ViewportWidget>,
}

impl<'a> egui_dock::TabViewer for MyTabViewer<'a> {
    type Tab = PanelType;
    
    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            PanelType::Hierarchy => self.panels.hierarchy.show(ui),
            PanelType::Inspector => self.panels.entity.show(ui),
            PanelType::Viewport => {
                if let Some(vp) = self.viewport.as_mut() {
                    vp.ui(ui, &self.panels.scene_state.world);
                }
            }
            // ... other panels
        }
    }
}
```

**Acceptance Criteria**:
- Drag panel tabs to reposition → layout updates
- Create new tab group by dragging to split zone → splits viewport
- Close panel → panel hidden (can reopen from Window menu)
- File → Save Layout → closes editor → reopens → layout restored

---

**Deliverable 1.3: Enhanced Viewport Controls**  
**Estimated Effort**: 1.5 weeks  
**Priority**: P1-A

**Tasks**:
1. **FR-2.1**: Multi-camera modes (Perspective, Top, Front, Side)
   - Modify `viewport/camera.rs` to support orthographic projection
   - Add dropdown in viewport toolbar
2. **FR-2.2**: Debug draw modes
   - Wireframe: Modify `viewport/renderer.rs` to use wireframe render pipeline
   - Colliders: Use `physics_renderer::PhysicsDebugRenderer` (already exists)
   - NavMesh: Render NavMesh triangles (use `astraweave-nav` API)
3. **FR-2.3**: Grid & snap settings
   - Add grid size dropdown (0.1, 0.25, 0.5, 1.0, 2.0, 5.0)
   - Modify `gizmo/snapping.rs` to use configurable grid size
4. **FR-2.4**: Camera bookmarks (F1-F12)
   - Detect F1-F12 keypress → save camera pose to `viewport_settings.camera_bookmarks[i]`
   - Detect Shift+F1-F12 → restore camera pose
   - Visual indicator: Show "📷 Saved F3" toast on bookmark save

**Acceptance Criteria**:
- Top view shows orthographic projection looking down -Y axis
- Wireframe mode renders mesh edges only (no fill)
- Collider mode shows green wireframe boxes/spheres for physics shapes
- Grid size 1.0 → gizmo snaps to integer coordinates
- Press F3 → camera saved → move camera → Shift+F3 → camera restored to saved position

---

**Deliverable 1.4: Project Settings Panel**  
**Estimated Effort**: 0.5 weeks  
**Priority**: P1-A

**Tasks**:
1. Create `panels/project_settings_panel.rs`
2. Implement UI tabs:
   - Physics: Layer names (32 text fields), collision matrix (32x32 checkboxes)
   - Input: Keyboard mappings table (action name, key binding)
   - Tags: Tag list (add/remove/rename)
3. Load/save to `project_settings.ron` in project root
4. Hot-reload: Apply changes without restart (send event to runtime)

**Acceptance Criteria**:
- Edit physics layer name "Player" → save → load scene → physics uses updated layer name
- Add input mapping "Jump" = Space → save → Play-in-Editor → Space key triggers jump
- Add tag "Enemy" → save → tag appears in entity inspector dropdown

---

### Phase 2: High-Value Features (Weeks 5-8) - Material Editor & Asset Pipeline

**Deliverable 2.1: Material Graph Editor**  
**Estimated Effort**: 3 weeks  
**Priority**: P1-B

**Tasks**:
1. **Week 5.1-5.2**: Node graph UI (FR-3.1)
   - Create `material/graph.rs`, `material/nodes.rs`
   - Integrate `egui_graphs::GraphEditor`
   - Implement node library:
     - Inputs: TextureSample, ColorConstant, FloatConstant, Vec3Constant
     - Math: Add, Multiply, Lerp, Clamp, Dot, Cross, Normalize
     - PBR: PbrOutputNode (base_color, metallic, roughness, normal, emissive)
   - Drag-and-drop node creation (right-click context menu)
   - Wire connections with type validation (float → vec3 fails)

2. **Week 5.3-5.4**: Shader compiler (FR-3.2)
   - Implement `material/compiler.rs`
   - Graph validation (cycle detection, type checking)
   - WGSL code generation:
     - Topological sort nodes
     - Generate variable declarations
     - Generate PBR output struct
   - Error reporting: Highlight invalid nodes in red, show error tooltip

3. **Week 6.1**: PBR preview (FR-3.3)
   - Enhance `brdf_preview.rs` to accept MaterialGraph
   - Compile graph → WGSL → bind to preview sphere
   - HDR environment rotation (drag mouse to rotate)

4. **Week 6.2**: Asset management (FR-3.4)
   - Save MaterialGraph to `.mat.ron`
   - Load in `AssetBrowser` with thumbnail preview
   - Drag material from asset browser → entity → applies to renderer component

**Acceptance Criteria**:
- Create material with TextureSample("albedo.png") → PbrOutput.base_color
- Compile → preview sphere shows textured surface
- Modify metallic slider 0.0 → 1.0 → preview updates in <500ms (hot-reload)
- Save as "metal.mat.ron" → drag to entity in viewport → entity becomes metallic

---

**Deliverable 2.2: Asset Import Pipeline**  
**Estimated Effort**: 2 weeks  
**Priority**: P1-A

**Tasks**:
1. **Week 7.1-7.2**: Import job queue (FR-8.2)
   - Create `asset_pipeline/import_job.rs`
   - Implement `AssetImportQueue` with tokio async tasks
   - Add importers:
     - `GltfImporter`: Uses `astraweave-render` GLTF loader
     - `TextureImporter`: PNG/JPG → DDS with mipmap generation
     - `FbxImporter`: Convert FBX to GLTF (external tool: FBX2glTF)
   - Progress UI: Show import queue in `AssetImportPanel` (bottom dock)

2. **Week 7.3**: Drag-drop import
   - Add file drop handler to `AssetBrowser`
   - Detect dropped files → enqueue import jobs
   - Show progress bar per job (0-100%)

3. **Week 7.4**: Asset organization (FR-8.3)
   - Tree view for folders in `AssetBrowser`
   - Drag assets between folders → move files on disk
   - Fuzzy search: Type "hero" → filters to matching assets
   - Tagging system: Right-click asset → Add Tag → tag stored in asset metadata

**Acceptance Criteria**:
- Drag `hero.fbx` to asset browser → converts to `hero.gltf` → imports mesh to AssetDatabase
- Drag `texture.png` → generates mipmaps → creates `texture.dds` in assets/textures/
- Create folder "Characters" → drag "hero.gltf" to folder → file moves to assets/Characters/hero.gltf
- Search "hero" → finds "hero.gltf", "hero_texture.png", "hero_anim.anim"

---

### Phase 3: Productivity Multipliers (Weeks 9-12) - Visual Scripting & Profiling

**Deliverable 3.1: Enhanced Behavior Graph Editor**  
**Estimated Effort**: 2 weeks  
**Priority**: P1-C

**Tasks**:
1. **Week 9.1-9.2**: Graph UX enhancements (FR-4.1)
   - Add search palette: Press Space → type node name → creates node
   - Minimap: Show graph overview in corner
   - Zoom/pan: Mouse wheel zoom, middle-mouse-drag pan
   - Node groups: Select nodes → Group → creates collapsible frame

2. **Week 9.3**: Live debugging (FR-4.2, partial)
   - Runtime → editor communication channel (tokio::sync::mpsc)
   - During Play-in-Editor: Highlight active nodes in green
   - Inspector shows live variable values (AI state, velocity, etc.)

3. **Week 9.4**: Graph validation (FR-4.3)
   - Detect missing connections (input not connected)
   - Type mismatches (bool → float)
   - Show error panel with clickable errors (click → jumps to node)

**Acceptance Criteria**:
- Press Space → type "Move" → creates MoveNode at mouse position
- Minimap shows full graph, viewport shows zoomed-in region
- Play-in-Editor → select AI entity → behavior graph shows active node highlighted
- Compile graph with unconnected input → error panel shows "MoveNode: speed input not connected"

---

**Deliverable 3.2: Build Pipeline & Profiling**  
**Estimated Effort**: 2 weeks  
**Priority**: P1-D

**Tasks**:
1. **Week 10.1**: Enhanced build manager (FR-7.1)
   - Add platform target dropdown (Windows, Linux, macOS, WASM)
   - Optimization level: Debug, Release, Release+LTO
   - Feature flags: Checkboxes for editor-core, profiling, git-integration
   - Build button → runs `cargo build` with selected config
   - Output logs to console panel in real-time (stream stderr)

2. **Week 10.2-10.3**: Tracy profiler integration (FR-7.2)
   - Add `tracy-client` dependency (behind "profiling" feature)
   - Implement `profiling/client.rs` wrapper
   - Collect frame data during Play-in-Editor
   - Render flamegraph in `ProfilerPanel` using `egui::plot::PlotUi`

3. **Week 10.4**: Performance budgets (FR-7.3)
   - Define budgets: ECS 2ms, AI 3ms, Physics 3ms, Rendering 8ms (total 16ms @ 60 FPS)
   - Show budget bars in profiler panel (green <80%, yellow 80-100%, red >100%)
   - Warning icon when budget exceeded

**Acceptance Criteria**:
- Select "Windows Release+LTO" → click Build → console shows `cargo build --release --target x86_64-pc-windows-msvc --features lto`
- Enable profiling → Play-in-Editor → profiler panel shows flamegraph with ECS/AI/Physics/Rendering sections
- Physics takes 5ms → budget bar shows red (exceeded 3ms budget)

---

**Deliverable 3.3: Scene Gizmos**  
**Estimated Effort**: 1 week  
**Priority**: P1-D

**Tasks**:
1. **Week 11**: Implement scene gizmos (FR-2.5)
   - `scene_gizmos/light_gizmo.rs`:
     - Point light: Yellow sphere (radius = light range)
     - Directional light: Yellow arrow
   - `scene_gizmos/camera_gizmo.rs`:
     - Camera frustum (wireframe pyramid)
   - `scene_gizmos/audio_gizmo.rs`:
     - Audio source: Blue sphere (radius = max distance)
   - Gizmos render in viewport, selectable (click → selects entity)
   - Scale with camera distance (always same screen size)

**Acceptance Criteria**:
- Add point light to scene → yellow sphere appears in viewport at light position
- Click sphere → selects light entity → inspector shows light properties
- Move camera away → gizmo sphere stays same screen size (not occluded by distance)

---

### Phase 4: Polish & Deferred Features (Weeks 13-16+) - OPTIONAL

**Deliverable 4.1: Animation Timeline** (Optional)  
**Estimated Effort**: 3 weeks  
**Priority**: P2 (defer if timeline at risk)

**Tasks**:
- Keyframe timeline UI (horizontal track editor)
- Curve editor (Bezier handles)
- Animation clip blending
- Export to `.anim.ron` format

**Acceptance Criteria**:
- Create position track → set keyframe at 0s, 1s, 2s → scrub timeline → viewport shows interpolated position

---

**Deliverable 4.2: Particle System Editor** (Optional)  
**Estimated Effort**: 2 weeks  
**Priority**: P2

**Tasks**:
- GPU particle preview integration
- Property inspector (rate, lifetime, forces, color gradient)
- Save to `.particle.ron` format

**Acceptance Criteria**:
- Modify emitter rate 100 → 1000 → viewport shows 10× more particles in real-time

---

**Deliverable 4.3: Version Control Integration** (Optional)  
**Estimated Effort**: 2 weeks  
**Priority**: P2

**Tasks**:
- Git status indicators in asset browser (M, A, D, ?, !)
- Diff viewer for `.scene.ron` files
- Commit dialog (select files, write message, commit)

**Acceptance Criteria**:
- Modify scene → asset browser shows yellow "M" icon next to scene file
- Right-click scene → "Show Diff" → side-by-side comparison with HEAD

---

## Verification Strategy

### Automated Testing

**Unit Tests**:
```rust
// tools/aw_editor/src/material/compiler.rs

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compile_simple_material() {
        let graph = MaterialGraph {
            nodes: vec![
                MaterialNode {
                    id: 0,
                    node_type: MaterialNodeType::ColorConstant { color: [1.0, 0.0, 0.0, 1.0] },
                    position: (0.0, 0.0),
                },
                MaterialNode {
                    id: 1,
                    node_type: MaterialNodeType::PbrOutput {
                        base_color: 0, // Connect to ColorConstant
                        metallic: None,
                        roughness: None,
                        normal: None,
                        emissive: None,
                    },
                    position: (200.0, 0.0),
                },
            ],
            edges: vec![
                MaterialEdge {
                    from_node: 0,
                    from_output: 0,
                    to_node: 1,
                    to_input: 0,
                },
            ],
            output_node: 1,
        };
        
        let wgsl = MaterialCompiler::compile(&graph).unwrap();
        assert!(wgsl.contains("vec4<f32>(1.0, 0.0, 0.0, 1.0)"));
    }
    
    #[test]
    fn test_detect_cycle() {
        // Graph with A → B → A cycle
        let graph = MaterialGraph { /* ... */ };
        let result = MaterialCompiler::compile(&graph);
        assert!(matches!(result, Err(CompileError::CycleDetected)));
    }
}
```

**Integration Tests**:
```rust
// tools/aw_editor/tests/integration_test.rs

#[test]
fn test_save_and_load_scene() {
    let mut app = EditorApp::default();
    
    // Create scene with 3 entities
    app.spawn_entity("Player", IVec2::new(0, 0));
    app.spawn_entity("Enemy", IVec2::new(10, 10));
    app.spawn_entity("Coin", IVec2::new(5, 5));
    
    // Save scene
    let path = PathBuf::from("test_scene.scene.ron");
    app.save_scene(&path).unwrap();
    
    // Clear scene
    app.new_scene();
    assert_eq!(app.scene_state.world.entity_count(), 0);
    
    // Load scene
    app.load_scene(&path).unwrap();
    assert_eq!(app.scene_state.world.entity_count(), 3);
    
    // Cleanup
    std::fs::remove_file(path).unwrap();
}
```

### Manual Testing Checklist

**Smoke Test Checklist** (30 test cases, run after each delivery):
```markdown
## Editor Launch
- [ ] 1. Launch editor → window opens (no crash)
- [ ] 2. Default layout loads (Hierarchy left, Viewport center, Inspector right)
- [ ] 3. Console shows "Editor started." log

## Scene Editing
- [ ] 4. Hierarchy panel shows default entities
- [ ] 5. Click entity → Inspector shows components
- [ ] 6. Press G → drag mouse → entity translates
- [ ] 7. Press R → drag mouse → entity rotates
- [ ] 8. Press S → drag mouse → entity scales
- [ ] 9. Press X/Y/Z → gizmo constrains to axis
- [ ] 10. Press Escape → cancels transform

## Undo/Redo
- [ ] 11. Modify entity → Ctrl+Z → change undone
- [ ] 12. Ctrl+Shift+Z → change redone
- [ ] 13. Undo/redo history visible in Edit menu

## Scene Save/Load
- [ ] 14. File → Save → select path → scene saves to .scene.ron
- [ ] 15. File → Load → select scene → scene loads
- [ ] 16. Unsaved changes → File → New Scene → shows "Save changes?" dialog

## Play-in-Editor
- [ ] 17. Press F5 → simulation starts (entities move)
- [ ] 18. Press F6 → simulation pauses
- [ ] 19. Press F7 → simulation resumes
- [ ] 20. Press F8 → simulation stops, returns to edit mode

## Asset Browser
- [ ] 21. Asset browser shows assets/ directory
- [ ] 22. Double-click .scene.ron → loads scene
- [ ] 23. Right-click → Create Folder → folder created
- [ ] 24. Drag asset → folder → asset moved

## Panel Docking
- [ ] 25. Drag panel tab → splits viewport → new dock region created
- [ ] 26. Close panel → panel hidden
- [ ] 27. Window → Reset Layout → layout restored to default

## Viewport
- [ ] 28. Middle-mouse drag → camera pans
- [ ] 29. Mouse wheel → camera zooms
- [ ] 30. Alt+LMB drag → camera orbits
```

### Performance Benchmarks

**Frame Time Budget Verification**:
```bash
# Run editor with profiling enabled
cargo run -p aw_editor --release --features profiling

# Load stress test scene (1000 entities)
# File → Load → tests/stress_test_1000.scene.ron

# Measure frame time (should be <16.67ms @ 60 FPS)
# Profiler panel → Show Flamegraph → verify total <16.67ms

# Expected breakdown:
# - ECS: <1ms
# - AI: <2ms
# - Physics: <3ms
# - Rendering: <8ms
# - Editor UI: <2ms
```

### Lint & Format Commands

**Pre-commit Checks**:
```bash
# Format code
cargo fmt --all

# Lint with clippy (zero warnings required)
cargo clippy -p aw_editor --all-features -- -D warnings

# Run tests
cargo test -p aw_editor --all-features

# Run integration tests
cargo test -p aw_editor --test integration_test
```

### Helper Scripts

**Verification Script** (`tools/aw_editor/verify.sh`):
```bash
#!/bin/bash
# Editor verification script - runs all checks

set -e  # Exit on error

echo "🔍 Running editor verification..."

echo "📦 Building editor..."
cargo build -p aw_editor --release --all-features

echo "🧪 Running unit tests..."
cargo test -p aw_editor --all-features

echo "📊 Checking code coverage..."
# Requires cargo-tarpaulin: cargo install cargo-tarpaulin
cargo tarpaulin -p aw_editor --all-features --out Stdout --target-dir target/tarpaulin | grep "^Coverage:"

echo "🔧 Linting with clippy..."
cargo clippy -p aw_editor --all-features -- -D warnings

echo "📝 Checking code format..."
cargo fmt -p aw_editor -- --check

echo "✅ All checks passed!"
```

**Material Compiler Test Assets** (`tools/aw_editor/tests/materials/`):
```
tests/materials/
├── simple_red.mat.ron        # Single ColorConstant → PbrOutput
├── textured.mat.ron           # TextureSample → PbrOutput
├── metal_rough.mat.ron        # Metallic/Roughness sliders
├── invalid_cycle.mat.ron      # Should fail validation (cycle)
└── type_mismatch.mat.ron      # Should fail validation (float → vec3)
```

These materials can be loaded in automated tests to verify compiler correctness.

### Required Test Artifacts

**Sample Assets** (to be created by implementation agent):
- `assets/textures/test_albedo.png` - 512×512 test texture
- `assets/meshes/test_cube.gltf` - Simple cube mesh
- `tests/stress_test_1000.scene.ron` - Scene with 1000 entities for performance testing
- `tests/materials/*.mat.ron` - Material test cases (see above)

**Discovery**: Agent can generate these programmatically:
```rust
// Example: Generate test texture
fn generate_test_texture() -> image::RgbaImage {
    let mut img = image::RgbaImage::new(512, 512);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let r = (x % 256) as u8;
        let g = (y % 256) as u8;
        *pixel = image::Rgba([r, g, 128, 255]);
    }
    img
}

// Save to assets/textures/test_albedo.png
img.save("assets/textures/test_albedo.png").unwrap();
```

### MCP Servers (None Required)

No external MCP servers needed. All verification can be done with:
- Built-in Rust tooling: `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt`
- Bash scripts for automation
- Programmatic test asset generation

---

## Risk Mitigation

### Risk 1: wgpu/egui API Migration Complexity
**Mitigation**:
- Allocate 1 week buffer for Deliverable 1.1
- Fallback: Pin to last known working versions (wgpu 24.x, egui 0.31, winit 0.29)
- Engage community: Post on wgpu/egui GitHub discussions if blocked >2 days

### Risk 2: egui_dock Integration Issues
**Mitigation**:
- Study `egui_dock` examples before implementation
- Prototype minimal example outside main codebase first
- Fallback: Implement custom docking system if egui_dock incompatible (adds 1 week)

### Risk 3: Performance Regression from Panel Updates
**Mitigation**:
- Profile after each delivery (use Tracy integration from Phase 3)
- Use `egui::Context::request_repaint()` sparingly (only on data changes)
- Virtual scrolling for large lists (hierarchy with 10k+ entities)

### Risk 4: Material Compiler Edge Cases
**Mitigation**:
- Extensive unit tests (20+ test cases for compiler)
- Manual testing with complex graphs (10+ nodes)
- Error messages must be actionable (show node ID, line number, suggested fix)

---

## Success Metrics

**Quantitative**:
- Build success: 0 errors, 0 warnings
- Test coverage: ≥80% for editor crates
- Performance: 60 FPS @ 1000 entities
- Feature parity: 95/100 vs Unity Editor (measured via feature matrix)

**Qualitative**:
- User satisfaction: 8/10 average rating from 5 developers
- Workflow efficiency: Level design task (10 prefabs, 5 materials) in <30 minutes
- Onboarding: "First Level" tutorial completes in <2 hours

---

**End of Technical Specification**
