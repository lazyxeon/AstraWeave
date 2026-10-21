# Y/Z Axis Swap Fix - Test Plan

## Problem Fixed
**Issue**: When pressing Y while Z is active during drag, the rotation would continue to apply to the Z axis instead of switching to Y axis.

**Root Cause**: The constraint was being **captured from the pattern match** when entering the drag handling code, so mid-drag constraint changes via X/Y/Z keys were not reflected in the rotation application.

**Solution**: Changed pattern matching from `GizmoMode::Rotate { constraint }` to `GizmoMode::Rotate { constraint: _ }` and added code to read the CURRENT constraint fresh from `self.gizmo_state.mode` inside the Rotate arm.

## Changes Made

### File: `tools/aw_editor/src/viewport/widget.rs`

1. **Line ~509** (Rotate mode):
   ```rust
   // OLD (WRONG - captures constraint at match time):
   GizmoMode::Rotate { constraint } => {
       let (rotation_angle, target_axis) = match constraint { ... }
   
   // NEW (CORRECT - reads current constraint):
   GizmoMode::Rotate { constraint: _ } => {
       let constraint = match self.gizmo_state.mode {
           GizmoMode::Rotate { constraint: c } => c,
           _ => AxisConstraint::None,
       };
       let (rotation_angle, target_axis) = match constraint { ... }
   ```

2. **Line ~398** (Translate mode - same fix for consistency):
   ```rust
   GizmoMode::Translate { constraint: _ } => {
       let constraint = match self.gizmo_state.mode {
           GizmoMode::Translate { constraint: c } => c,
           _ => AxisConstraint::None,
       };
   ```

## Test Plan

### Test 1: Y → Z Mid-Drag Switch (The Original Bug!)
1. ✅ Launch editor: `cargo run -p aw_editor --release`
2. ✅ Select cube entity (click on it)
3. ✅ Press `R` to enter Rotate mode (no axis selected, default Y rotation)
4. ✅ Press `Z` to select Z-axis (blue circle should highlight)
5. ✅ Start dragging the mouse (entity should rotate around Z-axis/roll)
6. ✅ **While still dragging**, press `Y` key
7. ✅ **EXPECTED**: Entity should immediately start rotating around Y-axis (yaw) instead
8. ✅ **VERIFY**: Green circle highlights, rotation changes from roll to yaw

### Test 2: X → Y Mid-Drag Switch
1. ✅ Press `R` then `X` (red circle highlights)
2. ✅ Start dragging (entity rotates around X-axis/pitch)
3. ✅ **While dragging**, press `Y`
4. ✅ **EXPECTED**: Switch to Y-axis rotation (yaw), green circle highlights

### Test 3: Y → X Mid-Drag Switch
1. ✅ Press `R` then `Y` (green circle highlights)
2. ✅ Start dragging (entity rotates around Y-axis/yaw)
3. ✅ **While dragging**, press `X`
4. ✅ **EXPECTED**: Switch to X-axis rotation (pitch), red circle highlights

### Test 4: Z → X Mid-Drag Switch
1. ✅ Press `R` then `Z` (blue circle highlights)
2. ✅ Start dragging (entity rotates around Z-axis/roll)
3. ✅ **While dragging**, press `X`
4. ✅ **EXPECTED**: Switch to X-axis rotation (pitch), red circle highlights

### Test 5: Rapid Axis Switching
1. ✅ Press `R` then start dragging
2. ✅ **While dragging**, rapidly press `X`, `Y`, `Z`, `Y`, `X` keys
3. ✅ **EXPECTED**: Rotation axis changes immediately with each key press
4. ✅ **VERIFY**: Circle highlights follow the key presses

### Test 6: Translate Mode (Consistency Check)
1. ✅ Press `G` to enter Translate mode
2. ✅ Press `X` (red arrow highlights)
3. ✅ Start dragging
4. ✅ **While dragging**, press `Z`
5. ✅ **EXPECTED**: Switch to Z-axis constraint, movement changes accordingly

## Console Output to Watch For

During testing, you should see:
```
🔧 Axis constraint: Y
🔧 Rotate: entity=0, axis=Y, start=0.0°, mouse_delta=(12.5, 0.0), angle=0.1°, new=0.1°
🔧 Axis constraint: Z
🔧 Rotate: entity=0, axis=Z, start=0.0°, mouse_delta=(0.0, 15.2), angle=0.1°, new=0.1°
```

The `axis=` value should change IMMEDIATELY when you press a different axis key during drag!

## Expected Behavior Summary

✅ **BEFORE FIX**: Pressing Y while dragging with Z active would NOT switch rotation axis (bug!)
✅ **AFTER FIX**: Pressing Y while dragging with Z active IMMEDIATELY switches to Y-axis rotation

## Visual Indicators

- **Red circle** = X-axis (pitch) - Vertical mouse movement
- **Green circle** = Y-axis (yaw) - Horizontal mouse movement  
- **Blue circle** = Z-axis (roll) - Vertical mouse movement
- **Yellow highlight** = Currently selected constraint

## Success Criteria

✅ All 6 tests pass
✅ Circle highlights change instantly when axis keys pressed
✅ Rotation axis changes instantly (no lag or old constraint behavior)
✅ Console logs show correct axis switching
✅ No crashes or visual glitches

---

**Build Command**: `cargo build -p aw_editor --release`
**Run Command**: `cargo run -p aw_editor --release`
**Test Duration**: ~5 minutes for all tests
