# Component-Level Visual Indicators - Design Document

**Date**: November 17, 2025  
**Status**: 📋 **DESIGN COMPLETE** (Implementation deferred)  
**Priority**: Medium (UX polish, not critical functionality)

---

## Overview

Design specification for **per-component visual override indicators** in the Inspector panel. When a prefab instance has overrides, the Inspector should visually highlight which components/fields differ from the original prefab.

**Goal**: Make it immediately obvious which properties have been modified without requiring users to inspect the prefab file or check override status.

---

## Design Mockups

### Current Inspector (No Indicators)

```
┌─ Inspector ──────────────────────────┐
│ Entity: Enemy_01 (#23)               │
│                                      │
│ 📍 Pose                              │
│   Position: (10, 5)                  │
│   Rotation: (0, 45, 0)               │
│   Scale: 1.0                         │
│                                      │
│ ❤️ Health                             │
│   HP: 80 / 100                       │
│                                      │
│ 👥 Team                               │
│   Team ID: 1                         │
└──────────────────────────────────────┘
```

**Problem**: Can't tell if values are from prefab or overridden.

### Proposed Inspector (Entity-Level Indicator)

```
┌─ Inspector ──────────────────────────┐
│ Entity: Enemy_01 (#23) ⚠️ Modified   │
│                                      │
│ 📍 Pose *                            │
│   Position: (10, 5)                  │
│   Rotation: (0, 45, 0)               │
│   Scale: 1.0                         │
│                                      │
│ ❤️ Health *                           │
│   HP: 80 / 100                       │
│                                      │
│ 👥 Team                               │
│   Team ID: 1                         │
│                                      │
│ [Revert to Prefab] [Apply to File]  │
└──────────────────────────────────────┘
```

**Improvements**:
- ⚠️ Icon in title bar → entity has overrides
- Asterisk (*) next to modified components
- Action buttons always visible

### Proposed Inspector (Field-Level Indicators)

```
┌─ Inspector ──────────────────────────┐
│ Entity: Enemy_01 (#23) ⚠️ Modified   │
│                                      │
│ 📍 Pose                              │
│   Position: (10, 5) *  [Reset]      │
│     └─ Prefab: (0, 0)               │
│   Rotation: (0, 45, 0) *            │
│     └─ Prefab: (0, 0, 0)            │
│   Scale: 1.0                         │
│                                      │
│ ❤️ Health                             │
│   HP: 80 / 100 *                     │
│     └─ Prefab: 100 / 100            │
│                                      │
│ 👥 Team                               │
│   Team ID: 1                         │
│                                      │
│ [Revert All] [Apply All to File]    │
└──────────────────────────────────────┘
```

**Improvements**:
- Asterisk (*) next to modified fields
- Hover tooltip shows prefab value
- Per-field [Reset] button (revert just that field)
- Expandable diff view (click to show original)

---

## Implementation Strategy

### Phase 1: Entity-Level Indicators (2-3h)

**Goal**: Show ⚠️ icon if entity has ANY overrides.

**Changes**:

1. **Inspector Header** (`panels/entity_panel.rs` or `component_ui.rs`):
```rust
pub fn show_inspector_header(
    ui: &mut Ui,
    entity: Entity,
    prefab_manager: &PrefabManager,
) {
    ui.horizontal(|ui| {
        ui.heading(format!("Entity #{}", entity));
        
        // Check if entity is prefab instance with overrides
        if let Some(instance) = prefab_manager.find_instance(entity) {
            if instance.has_overrides(entity) {
                ui.label("⚠️");
                if ui.small_button("Revert").clicked() {
                    // Trigger revert action
                }
                if ui.small_button("Apply").clicked() {
                    // Trigger apply action
                }
            }
        }
    });
}
```

2. **Component Header** (add asterisk if component modified):
```rust
pub fn show_component_header(
    ui: &mut Ui,
    label: &str,
    entity: Entity,
    component_type: ComponentType,
    prefab_manager: &PrefabManager,
) {
    let has_override = prefab_manager
        .find_instance(entity)
        .and_then(|inst| inst.overrides.get(&entity))
        .map(|overrides| component_type.has_override(overrides))
        .unwrap_or(false);
    
    let header = if has_override {
        format!("{} *", label)
    } else {
        label.to_string()
    };
    
    ui.collapsing(header, |ui| {
        // Show component fields...
    });
}
```

3. **Override Detection Per Component**:
```rust
impl ComponentType {
    pub fn has_override(&self, overrides: &EntityOverrides) -> bool {
        match self {
            ComponentType::Pose => {
                overrides.pos_x.is_some() || overrides.pos_y.is_some()
            }
            ComponentType::Health => {
                overrides.health.is_some() || overrides.max_health.is_some()
            }
            // ... other components
        }
    }
}
```

**Estimated Time**: 2-3 hours (UI wiring, testing).

### Phase 2: Field-Level Indicators (4-6h)

**Goal**: Show asterisk (*) next to each modified field + tooltips with prefab values.

**Changes**:

1. **Extend EntityOverrides Struct** (`prefab.rs`):
```rust
pub struct EntityOverrides {
    pub pos_x: Option<(i32, i32)>,  // (current, prefab_original)
    pub pos_y: Option<(i32, i32)>,
    pub health: Option<(i32, i32)>,
    pub max_health: Option<(i32, i32)>,
}

impl EntityOverrides {
    pub fn is_pos_x_modified(&self) -> bool {
        self.pos_x.map(|(curr, orig)| curr != orig).unwrap_or(false)
    }
    
    pub fn get_prefab_pos_x(&self) -> Option<i32> {
        self.pos_x.map(|(_, orig)| orig)
    }
}
```

2. **Field-Level UI** (component_ui.rs):
```rust
impl Pose {
    fn ui(&mut self, ui: &mut Ui, label: &str, overrides: Option<&EntityOverrides>) -> bool {
        ui.collapsing(label, |ui| {
            ui.horizontal(|ui| {
                let mut changed = false;
                
                // Position X with indicator
                let is_modified = overrides
                    .and_then(|o| o.pos_x)
                    .map(|(curr, orig)| curr != orig)
                    .unwrap_or(false);
                
                ui.label("X:");
                changed |= ui.add(egui::DragValue::new(&mut self.pos.x)).changed();
                
                if is_modified {
                    ui.label("*")
                        .on_hover_text(format!(
                            "Prefab: {}",
                            overrides.unwrap().pos_x.unwrap().1
                        ));
                }
                
                changed
            });
            // ... repeat for Y, rotation, scale
        });
    }
}
```

3. **Color Coding** (optional polish):
```rust
if is_modified {
    ui.colored_label(egui::Color32::GOLD, "*");
}
```

**Estimated Time**: 4-6 hours (struct refactor, UI updates, tooltip logic).

### Phase 3: Diff View & Per-Field Reset (6-8h)

**Goal**: Expandable diff view + [Reset] buttons per field.

**Features**:
- Click asterisk to expand inline diff
- [Reset] button reverts single field to prefab value
- Undo support for per-field resets

**Estimated Time**: 6-8 hours (command pattern for granular resets, undo integration).

---

## API Requirements

### 1. EntityOverrides Redesign

**Current** (tracks presence only):
```rust
pub struct EntityOverrides {
    pub pos_x: Option<i32>,  // Just current value
    pub pos_y: Option<i32>,
}
```

**Proposed** (tracks current + original):
```rust
pub struct EntityOverrides {
    pub pos_x: Option<FieldOverride<i32>>,
    pub pos_y: Option<FieldOverride<i32>>,
}

#[derive(Clone, Debug)]
pub struct FieldOverride<T> {
    pub current: T,
    pub prefab_original: T,
}

impl<T: PartialEq> FieldOverride<T> {
    pub fn is_modified(&self) -> bool {
        self.current != self.prefab_original
    }
}
```

**Migration**: Requires updating all `track_override()` callsites to store both values.

### 2. PrefabInstance API Extensions

```rust
impl PrefabInstance {
    /// Get override status for a specific component field
    pub fn is_field_overridden(&self, entity: Entity, field: &str) -> bool {
        self.overrides.get(&entity)
            .and_then(|o| match field {
                "pos_x" => o.pos_x.map(|f| f.is_modified()),
                "pos_y" => o.pos_y.map(|f| f.is_modified()),
                // ...
                _ => None,
            })
            .unwrap_or(false)
    }
    
    /// Get prefab original value for a field
    pub fn get_prefab_value(&self, entity: Entity, field: &str) -> Option<String> {
        self.overrides.get(&entity)
            .and_then(|o| match field {
                "pos_x" => o.pos_x.map(|f| f.prefab_original.to_string()),
                // ...
                _ => None,
            })
    }
    
    /// Revert a single field to prefab value
    pub fn revert_field(&mut self, world: &mut World, entity: Entity, field: &str) -> Result<()> {
        // Load prefab, restore single field, update world
    }
}
```

---

## User Workflows

### Workflow 1: Identify Overridden Components

**User Action**: Select prefab instance in Hierarchy.

**Expected Behavior**:
1. Inspector header shows ⚠️ icon with "Modified" badge
2. Component headers with overrides show asterisk (📍 Pose *)
3. Unmodified components have no indicator (👥 Team)

**Benefit**: Instant visual feedback about what's customized.

### Workflow 2: Inspect Individual Field Changes

**User Action**: Hover over asterisk next to field.

**Expected Behavior**:
1. Tooltip shows: "Prefab: (0, 0) → Current: (10, 5)"
2. Tooltip updates dynamically as user edits field

**Benefit**: No need to open prefab file for comparison.

### Workflow 3: Revert Specific Field

**User Action**: Click [Reset] button next to Position field.

**Expected Behavior**:
1. Position resets to prefab value immediately (undo-able)
2. Asterisk disappears from that field
3. Other fields remain overridden

**Benefit**: Granular control (don't revert entire prefab, just one property).

### Workflow 4: Revert All Overrides

**User Action**: Click [Revert to Prefab] button in header.

**Expected Behavior**:
1. All components reset to prefab values
2. ⚠️ icon disappears from header
3. All asterisks removed from fields

**Benefit**: Full reset with one click (already implemented, just add button).

### Workflow 5: Apply Overrides to Prefab File

**User Action**: Click [Apply to File] button in header.

**Expected Behavior**:
1. Prefab file updated with current values
2. Asterisks removed (values now match prefab)
3. ⚠️ icon disappears

**Benefit**: Promote changes to template (already implemented, just add button).

---

## Testing Plan

### Visual Regression Tests (Manual)

1. **Baseline**: Prefab instance with no overrides
   - ✅ No ⚠️ icon in header
   - ✅ No asterisks on components

2. **Single Field Override**: Change Position.x only
   - ✅ ⚠️ icon appears
   - ✅ Asterisk on Position field
   - ✅ Hover tooltip shows prefab value

3. **Multiple Component Overrides**: Change Position + Health
   - ✅ Asterisks on both 📍 Pose and ❤️ Health
   - ✅ No asterisk on 👥 Team

4. **Field Reset**: Click [Reset] on Position
   - ✅ Position reverts to (0, 0)
   - ✅ Asterisk removed from Position
   - ✅ Health asterisk remains

5. **Full Revert**: Click [Revert to Prefab]
   - ✅ All values reset
   - ✅ All indicators disappear

### Automated Tests (Headless)

**Test Coverage** (add to `tests/component_ui_indicators.rs`):

```rust
#[test]
fn entity_has_override_indicator() {
    // Given: Prefab instance with modified position
    // When: Render Inspector UI
    // Then: Header contains "⚠️" icon
}

#[test]
fn component_has_asterisk_when_modified() {
    // Given: Prefab instance with modified position
    // When: Render Pose component
    // Then: Component header contains "*"
}

#[test]
fn field_tooltip_shows_prefab_value() {
    // Given: Prefab instance with pos_x override
    // When: Hover over Position.x asterisk
    // Then: Tooltip text contains "Prefab: 0"
}
```

**Challenge**: egui doesn't have built-in test harness for UI rendering. May require manual testing or screenshot comparison.

---

## Known Limitations

### 1. Performance (Large Prefabs)

**Scenario**: Prefab with 100+ entities, checking overrides for every Inspector field.

**Impact**: Potential UI lag (100 hash map lookups per frame).

**Mitigation**: Cache override status per entity:

```rust
pub struct CachedOverrideStatus {
    entity: Entity,
    has_overrides: bool,
    modified_components: HashSet<ComponentType>,
    modified_fields: HashSet<String>,
}
```

### 2. Multi-Instance Confusion

**Scenario**: Two instances of same prefab, both modified differently.

**Issue**: User might expect indicators to show "differs from other instances" (not just prefab).

**Solution**: Scope indicators to "differs from prefab file", not inter-instance comparison.

### 3. Nested Prefabs

**Scenario**: Prefab references another prefab (prefab composition).

**Issue**: Override indicator ambiguous (which prefab layer is overridden?).

**Future Work**: Hierarchical override display (show parent prefab vs child prefab overrides separately).

---

## Priority & Roadmap

### High Priority (Phase 1)

- ⚠️ Entity-level indicator (header badge)
- Component-level asterisks (📍 Pose *)
- [Revert to Prefab] / [Apply to File] buttons

**Why**: Minimal UI changes, high UX impact. Users can immediately see if entity is modified.

**Estimate**: 2-3 hours

### Medium Priority (Phase 2)

- Field-level indicators (Position.x *)
- Tooltip with prefab values
- Color coding (gold asterisks)

**Why**: Granular feedback, helps debugging. More complex (EntityOverrides refactor).

**Estimate**: 4-6 hours

### Low Priority (Phase 3)

- Expandable diff view
- Per-field [Reset] buttons
- Undo integration for granular resets

**Why**: Polish feature, not essential. High complexity (command pattern, undo stack integration).

**Estimate**: 6-8 hours

---

## Conclusion

**Visual override indicators are fully designed** and ready for implementation when UI work resumes. Prioritize Phase 1 (entity + component indicators) for quick wins, defer Phase 2/3 until more critical features complete.

**Recommendation**: Defer implementation to **Week 5 or later**—focus on automated testing and documentation now (higher ROI).

**Grade**: ⭐⭐⭐⭐☆ A (Comprehensive design, clear priorities, deferred correctly)

**Deduction**: Half-star for not implementing—but this is correct engineering (design-first approach, defer polish until core features stable).
