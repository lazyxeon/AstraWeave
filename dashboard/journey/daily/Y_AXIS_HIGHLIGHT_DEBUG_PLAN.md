# Y-Axis Highlight Persistence - Debug Plan

**Issue**: Y-axis circle stays highlighted when X or Z axis is selected in rotate mode

## Test Scenario

1. **Run Editor**:
   ```powershell
   .\target\release\aw_editor.exe
   ```

2. **Select Entity**:
   - Click on any cube in the viewport
   - Should see selection outline

3. **Start Rotate Mode**:
   - Press **R** key
   - **Expected Console Output**:
     ```
     🔄 Rotate mode started - constraint reset to None
     ```
   - **Expected Visual**: All 3 circles visible (red/green/blue), NONE highlighted

4. **Constrain to X-Axis**:
   - Press **X** key
   - **Expected Console Output**:
     ```
     🎯 Rotate constraint: None → X
     🎨 Gizmo Renderer: Rotate constraint = X
     🎨 Renderer: Rendering Rotate gizmo, constraint = X
     ```
   - **Expected Visual**: Only RED circle highlighted (X-axis)
   - **ACTUAL (Bug)**: RED and GREEN both highlighted

5. **Constrain to Z-Axis**:
   - Press **Z** key  
   - **Expected Console Output**:
     ```
     🎯 Rotate constraint: X → Z
     🎨 Gizmo Renderer: Rotate constraint = Z
     🎨 Renderer: Rendering Rotate gizmo, constraint = Z
     ```
   - **Expected Visual**: Only BLUE circle highlighted (Z-axis)
   - **ACTUAL (Bug)**: BLUE and GREEN both highlighted

6. **Constrain to Y-Axis**:
   - Press **Y** key
   - **Expected Console Output**:
     ```
     🎯 Rotate constraint: Z → Y
     🎨 Gizmo Renderer: Rotate constraint = Y
     🎨 Renderer: Rendering Rotate gizmo, constraint = Y
     ```
   - **Expected Visual**: Only GREEN circle highlighted (Y-axis)
   - **ACTUAL**: Should work correctly (only Y highlighted)

## Debug Questions

From the console output, we need to determine:

1. **Is render() being called multiple times per frame?**
   - Look for duplicate "🎨 Renderer: Rendering Rotate gizmo" messages
   - If YES: Something is calling render() twice with different gizmo states

2. **Is the constraint value correct in the renderer?**
   - Check if "🎨 Gizmo Renderer: Rotate constraint = X" matches what was set
   - If NO: GizmoState is not being updated properly

3. **Is the constraint cycling correctly?**
   - Check if "🎯 Rotate constraint: None → X" shows proper transitions
   - If NO: The cycle() function has a bug

## Hypothesis

Based on code review, the most likely causes are:

1. **Multiple render calls**: Renderer.render() called twice per frame with different states
2. **Stale GizmoState**: Using old/cached gizmo_state instead of current
3. **Color blending issue**: GPU rendering both highlighted and unhighlighted on top of each other

## Next Steps After Testing

Once you run the test and observe the console output, report back:
- Exact console messages when pressing R → X
- Whether there are duplicate render messages
- Screenshot of the visual bug (if possible)

Then I can pinpoint the exact issue and fix it.
