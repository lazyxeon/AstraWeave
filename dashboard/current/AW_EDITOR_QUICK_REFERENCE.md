# aw_editor Quick Reference Guide

**Last Updated**: January 17, 2026  
**Version**: 0.2.0 (Clipboard & Undo System Complete)

---

## 🎮 Keyboard Shortcuts

### Transform Modes
| Key | Mode | Description |
|-----|------|-------------|
| **G** | Translate | Move entity in screen plane |
| **R** | Rotate | Rotate entity around focal point |
| **S** | Scale | Scale entity (uniform by default) |

### Axis Constraints
| Key | Constraint | Example |
|-----|-----------|----------|
| **X** | X-axis only | G → X → drag = move along X |
| **Y** | Y-axis only | R → Y → drag = rotate around Y |
| **Z** | Z-axis only | S → Z → drag = scale along Z |

### Operation Controls
| Key | Action | When |
|-----|--------|------|
| **Enter** | Confirm | During G/R/S operation |
| **Escape** | Cancel (revert) | During G/R/S operation |
| **Shift** | Snap mode | R: 15° increments<br>S: Force uniform |

### Camera Controls
| Input | Action | Description |
|-------|--------|-------------|
| **Left Mouse Drag** | Orbit | Rotate camera around focal point |
| **Middle Mouse Drag** | Pan | Move focal point in screen space |
| **Scroll Wheel** | Zoom | Change distance from focal point |
| **F** | Frame Selected | Center camera on selected entity |

### Editing Commands
| Key | Action | Description |
|-----|--------|-------------|
| **Ctrl+C** | Copy | Copy selected entities to clipboard |
| **Ctrl+V** | Paste | Paste entities from clipboard |
| **Ctrl+D** | Duplicate | Duplicate selected entities in-place |
| **Delete** | Delete | Remove selected entities |
| **Ctrl+Z** | Undo | Undo last command |
| **Ctrl+Shift+Z** | Redo | Redo undone command |

---

## 🎯 Workflow Examples

### Example 1: Move Entity Along X-Axis
1. **Click** entity to select
2. Press **G** (start translate mode)
3. Press **X** (constrain to X-axis)
4. **Drag mouse** (entity moves along X only)
5. Press **Enter** (confirm) OR **Escape** (cancel)

### Example 2: Rotate Entity 90° Around Y-Axis
1. **Click** entity to select
2. Press **R** (start rotate mode)
3. Press **Y** (constrain to Y-axis)
4. Hold **Shift** (enable snap)
5. **Drag mouse** until ~90° (will snap to 90°)
6. Press **Enter** (confirm)

### Example 3: Scale Entity to 2× Size
1. **Click** entity to select
2. Press **S** (start scale mode)
3. **Type** "2.0" on keyboard (numeric input)
4. Press **Enter** (apply 2× scale)

### Example 4: Frame Entity in Camera
1. **Click** entity to select
2. Press **F** (frame selected)
3. Camera automatically centers on entity with nice framing distance

---

## 🧩 UI Panels

### Transform Panel
**Location**: Right sidebar → "🔧 Transform" collapsible

**Fields**:
- **Position**: X/Y/Z world coordinates (meters)
- **Rotation**: Euler angles in degrees (XYZ order)
- **Scale**: X/Y/Z scale factors (1.0 = original size)

**Usage**:
- **Drag sliders**: Fine-tune values
- **Type numbers**: Precise input (e.g., "5.0" for X position)
- **Sync**: Changes reflect in viewport immediately

---

## 🐛 Troubleshooting

### Issue: Can't Select Entity
**Symptom**: Clicking entities does nothing

**Causes**:
1. Entity is in World, not EntityManager (only World entities render currently)
2. Camera is too far away (entity too small on screen)

**Fix**:
1. Use World entities for testing (they render with team colors)
2. Press **F** to frame selected entity if already selected

---

### Issue: Gizmo Not Moving Entity
**Symptom**: Pressed G/R/S but dragging does nothing

**Causes**:
1. No entity selected
2. Not dragging with left mouse button
3. Viewport doesn't have focus

**Fix**:
1. Click entity first (selection should log to console)
2. Ensure using left mouse button (not middle/right)
3. Click viewport once to give focus

---

### Issue: Transform Panel Shows Wrong Values
**Symptom**: Panel displays old position/rotation/scale

**Causes**:
1. Different entity selected
2. Panel not updating (rare egui bug)

**Fix**:
1. Re-select entity (click it again)
2. Collapse and re-open Transform panel

---

### Issue: Camera Zoomed Too Far
**Symptom**: Can't see entities, viewport looks empty

**Fix**:
1. Select an entity (if you know its ID)
2. Press **F** to frame it
3. OR: Scroll wheel to zoom in slowly

---

## 📊 Performance Tips

### Large Scenes (>1000 entities)
- Entity picking may slow down (O(n) ray tests)
- Future optimization: Octree spatial partitioning
- Workaround: Use search/filter to reduce visible entities

### Low FPS (<30)
- Editor overhead: ~2ms/frame (should be fine)
- Check rendering: Press F3 to toggle debug stats (if available)
- Disable shadows/post-processing in settings

---

## 🔧 Developer Notes

### Adding Custom Entities
```rust
// In main.rs or entity_manager.rs
let mut manager = EntityManager::new();

manager.create(
    "MyEntity",                          // Name
    Vec3::new(0.0, 0.0, 0.0),           // Position
    Quat::IDENTITY,                      // Rotation
    Vec3::ONE,                           // Scale
    Some("my_mesh.glb".to_string()),    // Mesh
    HashMap::new(),                      // Components
);
```

### Testing Transform Math
```rust
// Test translation calculation
let translation = TranslateGizmo::calculate_translation(
    Vec2::new(100.0, 0.0),  // Mouse delta (100px right)
    AxisConstraint::X,       // X-axis only
    10.0,                    // Camera distance
    Quat::IDENTITY,          // Object rotation
    false,                   // World space
);

assert_eq!(translation.y, 0.0);  // No Y movement
assert_eq!(translation.z, 0.0);  // No Z movement
```

---

## 📝 Known Limitations

1. **EntityManager entities not rendered**: Only World entities visible (fix planned for next phase)
2. **No visual gizmo handles**: Keyboard-only workflow (3D arrows/rings coming later)
3. **Instant camera transitions**: Frame selected (F) jumps instantly (smooth lerp optional)

---

## ✅ Completed Features

- [x] Undo/redo system (Ctrl+Z / Ctrl+Shift+Z)
- [x] Multi-selection (Shift+Click, Ctrl+Click)
- [x] Duplicate (Ctrl+D)
- [x] Delete (Delete key, menu, multi-entity support)
- [x] Copy/Paste (Ctrl+C / Ctrl+V) with BehaviorGraph preservation
- [x] Scene save/load (RON format)
- [x] Asset browser with drag-and-drop prefabs
- [x] Prefab system with Apply/Revert
- [x] Play mode (F5/F6/F7/F8 controls)
- [x] Behavior graph editor

## 🚀 Future Features (Roadmap)

### Phase 2: Visual Improvements
- [ ] Render EntityManager entities in viewport
- [ ] Visual gizmo handles (arrow/ring/cube)
- [ ] Smooth camera transitions

### Phase 3: Advanced Editing
- [ ] Box selection (drag rectangle)
- [ ] Snapping grid alignment
- [ ] Local vs World space toggle

---

## 📚 Related Documentation

- **Full Implementation Report**: `docs/journey/daily/AW_EDITOR_GIZMO_INTEGRATION_COMPLETE.md`
- **Delete Command Report**: `docs/journey/daily/EDITOR_DELETE_COMMAND_COMPLETE.md`
- **Gizmo State API**: `tools/aw_editor/src/gizmo/state.rs`
- **Clipboard API**: `tools/aw_editor/src/clipboard.rs`
- **Command System**: `tools/aw_editor/src/command.rs`
- **EntityManager API**: `tools/aw_editor/src/entity_manager.rs`
- **Camera Controls**: `tools/aw_editor/src/viewport/camera.rs`

---

**Questions?** Open an issue on GitHub or check the implementation reports in `docs/journey/daily/`

**Contributing?** See `CONTRIBUTING.md` for development setup and coding standards
