# AstraWeave Editor: Production Readiness Implementation Plan

**Document Version**: 2.1  
**Created**: January 14, 2026  
**Last Updated**: January 2026  
**Status**: ACTIVE DEVELOPMENT  
**Objective**: Transform aw_editor into a fully production-ready game editor

---

## Progress Log

### Session Update (Latest - Week 4 COMPLETE)

**Completed Work:**
- ✅ **Phase 2 Week 1 Day 1-4**: Engine adapter integration (infrastructure)
- ✅ **Phase 2 Week 1 Day 3-4**: Asset Browser → Viewport Integration
  - Added LoadToViewport action
  - Double-click model files to load them
  - Context menu with "Load to Viewport" button
- ✅ **Phase 2 Week 2 Day 1-2**: Material Inspector → Viewport Sync
  - Added set_material_params API to Renderer/EngineAdapter/Widget
  - Added model_count() and model_names() methods
  - Debug menu with material testing (Red/Green/Blue/White presets)
- ✅ **Phase 2 Week 2 Day 5**: Material Editing - CONNECTED TO VIEWPORT
  - Material Editor sliders now sync to 3D viewport in real-time
  - Added color preview swatch with PBR property display
  - Added manual "Apply to Viewport" button
  - Added viewport sync status indicator
- ✅ **Phase 2 Week 3 Day 1-2**: Directional Light (Time of Day)
  - Added get/set_time_of_day API through full stack
  - Added time presets (Dawn/Noon/Sunset/Midnight) to Debug menu
  - Display current time and period (Day/Twilight/Night)
- ✅ **Phase 2 Week 3 Day 3-4**: Shadow Preview
  - Added shadows_enabled flag and toggle methods
  - Shadow ON/OFF button in Debug menu
- ✅ **Phase 2 Week 3 Day 5**: Viewport Settings UI (ALREADY COMPLETE)
  - Toolbar has Shading Mode (Lit/Unlit/Wireframe)
  - Toolbar has Grid toggle with type (Infinite/Crosshair)
  - Toolbar has Grid Snap with size controls
  - Toolbar has Angle Snap controls
- ✅ **Phase 3 Week 4 Day 1-2**: Drag-Drop Import
  - glTF/GLB drag-drop to viewport (auto-loads model)
  - .ron scene drag-drop (loads scene file)
  - Visual drop overlay showing file types being dragged
  - Toast notifications for import status
  - Console logging for all import operations
- ✅ **Phase 3 Week 4 Day 3-4**: Texture Compression (BC7)
  - BC7 compression via astraweave-asset-pipeline/intel_tex
  - Validates dimensions (must be divisible by 4)
  - Reports compression ratio and time in console
  - Saves compressed textures to project/textures/*.bc7.bin
  - KTX2 files recognized as pre-compressed
- ✅ **Phase 3 Week 4 Day 5**: Asset Validation
  - GLB header validation (magic number check)
  - File size warnings (>50MB LOD suggestion, >100MB slow warning)
  - Format detection (GLB binary vs glTF JSON)
  - External .bin file detection for glTF files

**Commits This Session:**
1. `f9c1b998` - Asset browser → viewport integration
2. `62ef4014` - PBR material parameter API and Debug menu
3. `b5401a38` - Time-of-day lighting controls
4. `8af5df96` - Shadow toggle to Debug menu
5. `a494a853` - Progress log documentation update
6. `ff8e8879` - Material Editor sliders connected to viewport
7. `6851435b` - Drag-drop file import with visual overlay
8. `77220fe0` - Asset validation on import
9. `4538ef80` - BC7 texture compression on import

**Current Status**: ✅ Week 4 COMPLETE (Full Import Workflow)
- All 274 tests passing
- Zero-warning policy maintained

**Next Steps**:
- Week 5 Day 1-2: Multi-Select Operations
- Week 5 Day 3-4: Prefab Workflow Enhancements
- Week 5 Day 5: Scene Statistics
- Week 5: Scene Workflow Polish (Multi-select, Prefabs, Statistics)

---

## Executive Summary

The AstraWeave Editor is at **82% production readiness** with a solid architectural foundation. To reach a status capable of designing, building, and shipping AAA-quality games, we need to address **5 critical gaps**:

1. **Viewport Mesh Rendering** (95% gap vs engine) - Currently renders cubes only
2. **PBR Material Preview** - No real-time material visualization in 3D
3. **Lighting System** - No shadows, no directional light preview
4. **Asset Pipeline Integration** - Import buttons don't fully integrate with viewport
5. **Polish & Stability** - Error handling, progress feedback, save confirmation

**Estimated Timeline**: 6-8 weeks with focused development.

---

## Current State Analysis

### ✅ What's Working Well (274 tests passing)

| System | Status | Tests | Notes |
|--------|--------|-------|-------|
| **Undo/Redo** | ✅ Complete | 48+ | 100-command history, auto-merge |
| **Scene Serialization** | ✅ Complete | 20+ | RON format, full fidelity |
| **Prefab System** | ✅ Complete | 15+ | Nested prefabs, overrides |
| **Gizmo System** | ✅ Complete | 50+ | Translate/Rotate/Scale |
| **Play-in-Editor** | ✅ Complete | 15+ | Snapshot/restore, frame step |
| **Plugin System** | ✅ Complete | 10+ | Event hooks, lifecycle |
| **Hot Reload** | ✅ Complete | 10+ | File watching, auto-reload |
| **Asset Browser** | ✅ Functional | 5+ | Thumbnails, drag-drop |
| **Build Manager** | ✅ Functional | 8+ | Platform targets |
| **Themes/Layouts** | ✅ Complete | 5+ | 5 themes, 5 presets |

### ❌ Critical Gaps

| Gap | Impact | Effort | Priority |
|-----|--------|--------|----------|
| **Viewport renders cubes only** | Can't preview assets | 40-60h | P0-CRITICAL |
| **No PBR materials in viewport** | Can't tune materials | 24-32h | P0-CRITICAL |
| **No lighting preview** | Dark scenes | 16-24h | P1-HIGH |
| **Asset actions incomplete** | Import doesn't show in 3D | 16-24h | P1-HIGH |
| **No progress for long ops** | Editor appears frozen | 8-12h | P1-HIGH |
| **No save confirmation** | Data loss risk | 4-8h | P2-MEDIUM |

---

## Phase 2: Viewport MVP (Weeks 1-3)

**Objective**: Render real glTF meshes with PBR materials

### Week 1: Mesh Loading & Display ✅ INFRASTRUCTURE COMPLETE

#### Day 1-2: Engine Adapter Integration ✅ DONE
- Engine rendering path verified in `viewport/renderer.rs` (lines 280-315)
- `EngineRenderAdapter` properly wired up with lazy initialization
- Camera conversion to engine format working (`to_engine_camera()`)
- Model loading via `load_gltf_model()` functional

**Completed Infrastructure**:
- `viewport/renderer.rs` - Engine render path integrated
- `viewport/engine_adapter.rs` - glTF loading with diagnostics
- `viewport/widget.rs` - FPS counter + rendering mode indicator added

**New Debug Menu Features**:
- 📦 Load Test Model (barrels.glb)
- 🛏️ Load Test Model (bed.glb)
- 🌲 Load Pine Tree
- 🔄 Toggle Engine Rendering
- 📋 Show Engine Info
- 📁 Scan For Models

**Viewport HUD**:
- FPS counter (top-left, color-coded: green/yellow/red)
- Rendering mode indicator (bottom-left: PBR Engine / Cube)

#### Day 3-4: Asset Browser → Viewport Integration ✅ DONE
Connect "Import Model" action to engine renderer:

**Completed Features**:
- `AssetAction::LoadToViewport` - New action variant for direct viewport preview
- 👁️ "Load to Viewport" button in asset browser context menu (no entity created)
- ➕ "Import to Scene" button now properly loads mesh AND creates entity
- **Double-click behavior**: Double-clicking any .glb/.gltf model in asset browser loads it to viewport
- Handler in `main.rs` with proper error handling and console feedback

**Files modified**:
- `panels/asset_browser.rs` - Added LoadToViewport action, context button, double-click behavior
- `main.rs` - Added LoadToViewport handler with logging

#### Day 5: Validation & Testing
- Test loading pine_tree_01_1k.glb from pine_forest assets
- Verify mesh displays correctly
- Add integration test

### Week 2: PBR Material Integration

#### Day 1-2: Material Inspector → Viewport Sync ✅ DONE
Material parameter API added:

**Completed Features**:
- `Renderer::set_material_params(base_color, metallic, roughness)` - Core renderer method
- `EngineRenderAdapter::set_material_params()` - Editor adapter wrapper with logging
- `ViewportWidget::set_material_params()` - Public API for editor integration
- `Renderer::model_count()` and `Renderer::model_names()` - Model discovery methods
- Debug menu "Material Testing" section with preset materials (Red, Green Metallic, Blue Rough, White)

**Files modified**:
- `astraweave-render/src/renderer.rs` - Added model_count() and model_names() methods
- `viewport/engine_adapter.rs` - Added set_material_params() wrapper
- `viewport/widget.rs` - Added public set_material_params() method
- `main.rs` - Added Material Testing section to Debug menu

#### Day 3-4: Texture Loading Pipeline
Enable texture preview from asset browser:

```rust
// In viewport/engine_adapter.rs
pub fn set_material_texture(&mut self, 
    model_name: &str,
    slot: MaterialSlot, 
    texture_path: &Path
) -> Result<()> {
    // Load texture via astraweave-render
    // Apply to model's material
}
```

#### Day 5: Material Editing
- Add sliders for roughness/metallic in inspector
- Real-time preview updates

### Week 3: Lighting & Polish

#### Day 1-2: Directional Light
Add basic sun light for scene preview:

```rust
// In viewport/engine_adapter.rs
pub fn set_sun_light(&mut self, direction: Vec3, intensity: f32, color: [f32; 3]) {
    self.renderer.set_directional_light(direction, intensity, color);
}
```

#### Day 3-4: Shadow Preview
Enable CSM shadows (engine already supports):

```rust
pub fn enable_shadows(&mut self, enabled: bool) {
    self.renderer.set_shadows_enabled(enabled);
}
```

#### Day 5: Viewport Settings UI
Add toolbar options:
- Toggle shadows
- Toggle grid
- Wireframe mode
- Normals visualization

---

## Phase 3: Asset Pipeline Completion (Weeks 4-5)

### Week 4: Full Import Workflow

#### Asset Types to Support
| Type | Extension | Import Action | Priority |
|------|-----------|---------------|----------|
| glTF/GLB | .gltf, .glb | Load to scene | P0 |
| Textures | .png, .jpg, .ktx2 | Apply to material | P0 |
| Materials | .toml | Load material def | P1 |
| Scenes | .ron | Load scene | P1 |
| Prefabs | .prefab | Instantiate | P1 |
| Audio | .ogg, .wav | Add to entity | P2 |
| Scripts | .rhai | Attach to entity | P2 |

#### Day 1-2: Drag-Drop Import
```rust
// In asset_browser.rs - Handle file drop
fn handle_dropped_files(&mut self, files: Vec<PathBuf>) {
    for file in files {
        match file.extension().and_then(|e| e.to_str()) {
            Some("glb" | "gltf") => self.pending_actions.push(AssetAction::ImportModel { path: file }),
            Some("png" | "jpg") => self.pending_actions.push(AssetAction::ApplyTexture { 
                path: file, 
                texture_type: TextureType::from_filename(&file.to_string_lossy())
            }),
            _ => {}
        }
    }
}
```

#### Day 3-4: Texture Compression
Add KTX2/BC7 compression on import:
- Use `astraweave-asset-pipeline` for compression
- Show progress bar for large textures

#### Day 5: Asset Validation
- Verify imported assets are valid
- Show warnings for unsupported features
- Log import statistics

### Week 5: Scene Workflow Polish

#### Day 1-2: Multi-Select Operations
- Apply material to multiple entities
- Group/ungroup selection
- Align/distribute tools

#### Day 3-4: Prefab Workflow
- Right-click → Create Prefab from selection
- Prefab override visualization (bolded properties)
- Break prefab connection option

#### Day 5: Scene Statistics
- Entity count
- Mesh statistics (triangles, vertices)
- Texture memory usage
- Performance estimates

---

## Phase 4: UX Polish (Weeks 6-7)

### Week 6: Feedback Systems

#### Day 1-2: Progress Bar System
```rust
// New: src/ui/progress.rs
pub struct ProgressManager {
    tasks: HashMap<TaskId, ProgressTask>,
}

pub struct ProgressTask {
    pub label: String,
    pub progress: f32,  // 0.0 - 1.0
    pub status: String,
    pub cancellable: bool,
}
```

**Integrate with**:
- Scene loading
- Asset import
- Build process
- Enter play mode

#### Day 3-4: Toast Notification Polish
Already implemented but needs refinement:
- Stack multiple toasts
- Click to dismiss
- Action buttons (Undo, View Details)

#### Day 5: Status Bar Enhancement
- Show background task progress
- Memory usage indicator
- GPU utilization

### Week 7: Safety & Recovery

#### Day 1-2: Unsaved Changes Handling
```rust
// In main.rs
fn on_close_request(&mut self) -> bool {
    if self.is_dirty {
        self.show_quit_dialog = true;
        false  // Cancel close
    } else {
        true   // Allow close
    }
}
```

**Dialog options**:
- Save & Quit
- Quit without saving
- Cancel

#### Day 3-4: Auto-Save & Recovery
```rust
// Auto-save every 5 minutes to .autosave/
fn auto_save(&mut self) {
    if self.auto_save_enabled && self.is_dirty {
        let backup_path = self.content_root.join(".autosave")
            .join(format!("scene_{}.ron", chrono::Utc::now().format("%Y%m%d_%H%M%S")));
        // Keep last 3 auto-saves
    }
}
```

#### Day 5: Crash Recovery
- Detect previous crash (lock file)
- Offer to restore from auto-save
- Log crash context for debugging

---

## Phase 5: Production Hardening (Week 8)

### Error Handling Audit
- [ ] Replace remaining mutex `.expect()` calls
- [ ] Add GPU device lost recovery
- [ ] Plugin error isolation
- [ ] Input validation for all text fields

### Performance Optimization
- [ ] Texture cache LRU eviction (max 100 items)
- [ ] Viewport frustum culling (already done)
- [ ] Async thumbnail loading
- [ ] Entity batching for 10k+ scenes

### Testing & CI
- [ ] 500+ tests target (currently 274)
- [ ] Viewport rendering tests
- [ ] Asset pipeline integration tests
- [ ] Memory leak stress tests (30 min)

---

## Success Criteria: Editor

### Minimum Viable (Week 3)
- [ ] Load and display glTF meshes
- [ ] PBR material preview with textures
- [ ] Basic directional lighting

### Production Ready (Week 6)
- [ ] Full asset import pipeline
- [ ] Progress bars for long operations
- [ ] Save confirmation dialog
- [ ] Auto-save with recovery

### World-Class (Week 8)
- [ ] 60 FPS with 10k entities
- [ ] <100ms scene load for typical scenes
- [ ] Zero crash path in normal usage
- [ ] Complete keyboard shortcut coverage
- [ ] Drag-drop everything

---

## Immediate Next Steps

### Today: Viewport MVP Setup
1. **Enable engine rendering** in ViewportRenderer
2. **Wire asset import** to engine adapter
3. **Test with pine_tree_01_1k.glb**

### This Week: Core Mesh Display
1. Complete engine adapter integration
2. Add material slot assignment
3. Implement basic lighting

### Next Week: Polish & Integration
1. Asset browser improvements
2. Material inspector sync
3. Progress system

---

## Resource Estimates

| Phase | Duration | Effort | Risk |
|-------|----------|--------|------|
| Phase 2: Viewport MVP | 3 weeks | 120h | Medium |
| Phase 3: Asset Pipeline | 2 weeks | 80h | Low |
| Phase 4: UX Polish | 2 weeks | 80h | Low |
| Phase 5: Hardening | 1 week | 40h | Low |
| **Total** | **8 weeks** | **320h** | |

**With parallel development**: 6 weeks possible

---

## Appendix: File Reference

### Critical Files for Viewport Work
- `tools/aw_editor/src/viewport/renderer.rs` - Main render coordinator
- `tools/aw_editor/src/viewport/engine_adapter.rs` - Engine integration
- `tools/aw_editor/src/viewport/widget.rs` - egui integration
- `tools/aw_editor/src/panels/asset_browser.rs` - Import actions
- `tools/aw_editor/src/main.rs` - Action handlers

### Key Engine Integration Points
- `astraweave-render/src/lib.rs` - Renderer API
- `astraweave-render/src/mesh_gltf.rs` - glTF loading
- `astraweave-render/src/material.rs` - Material system

---

**Document End**

*This plan represents the critical path to a production-ready game editor capable of shipping AAA-quality games.*
