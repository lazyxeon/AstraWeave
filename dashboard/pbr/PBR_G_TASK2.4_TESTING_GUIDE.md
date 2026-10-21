# Phase PBR-G Task 2.4 Testing Guide
**Date**: 2025-10-07  
**Purpose**: Comprehensive testing and validation for Material Inspector

## Overview
This guide covers manual testing procedures for all Material Inspector features implemented in Tasks 2.1-2.3, including texture viewing, BRDF preview, asset browser, and material history.

---

## Prerequisites

### 1. Build the Editor
```powershell
cargo build -p aw_editor --release
```

**Expected Output**: Clean build with 3 warnings (unused future features)

### 2. Prepare Test Materials
Ensure demo materials exist:
```
assets/materials/terrain/
├── grassland_demo.toml
├── mountain_demo.toml
└── desert_demo.toml
```

Each material should have associated textures:
- `*_albedo.png` (or .ktx2)
- `*_normal.png` (or .ktx2)
- `*_orm.png` (or .ktx2)

### 3. Launch Editor
```powershell
cargo run -p aw_editor --release
```

---

## Test Suite 1: Basic Material Loading (Task 2.1)

### Test 1.1: Load Material via Browser
**Steps**:
1. Open Material Inspector panel
2. Expand "Material Browser" section
3. Click "▶ Show Browser"
4. Click on `terrain/grassland_demo.toml`

**Expected Results**:
- ✅ Material loads successfully
- ✅ Status shows "Loaded: grassland_demo.toml"
- ✅ Albedo texture displays in viewer
- ✅ Validation results appear (if any)

**Pass Criteria**: All checkboxes pass, no errors

---

### Test 1.2: Display Mode Switching
**Steps**:
1. Load grassland_demo.toml (Test 1.1)
2. Click "Albedo" radio button
3. Click "Normal" radio button
4. Click "ORM" radio button

**Expected Results**:
- ✅ Albedo: Shows color texture (green/brown tones)
- ✅ Normal: Shows purple/blue normal map
- ✅ ORM: Shows grayscale/color packed texture (Occlusion=R, Roughness=G, Metallic=B)

**Pass Criteria**: All 3 textures display correctly, no black/magenta placeholders

---

### Test 1.3: Channel Filtering
**Steps**:
1. Select "Albedo" display mode
2. Click "Red" channel filter
3. Click "Green" channel filter
4. Click "Blue" channel filter
5. Click "Alpha" channel filter
6. Click "All (RGB)" channel filter

**Expected Results**:
- ✅ Red: Shows red channel only (grayscale)
- ✅ Green: Shows green channel only (grayscale)
- ✅ Blue: Shows blue channel only (grayscale)
- ✅ Alpha: Shows alpha channel only (should be white if opaque)
- ✅ All: Shows full RGB color

**Pass Criteria**: Each channel isolates correctly, no color mixing

---

### Test 1.4: Color Space Toggle
**Steps**:
1. Select "Albedo" display mode, "All (RGB)" channel
2. Note current appearance
3. Click "Linear" color space
4. Click "sRGB" color space

**Expected Results**:
- ✅ sRGB: Normal appearance (gamma-corrected)
- ✅ Linear: Darker, less saturated (raw linear values)
- ✅ Toggling back to sRGB restores original appearance

**Pass Criteria**: Visible difference between color spaces, no errors

---

### Test 1.5: Zoom Controls
**Steps**:
1. Load material with albedo texture
2. Set zoom to 0.1x (minimum)
3. Set zoom to 1.0x (default)
4. Set zoom to 4.0x (maximum)

**Expected Results**:
- ✅ 0.1x: Texture very small, entire texture visible
- ✅ 1.0x: Normal size
- ✅ 4.0x: Texture magnified, pixelation visible

**Pass Criteria**: Zoom responds smoothly, no distortion

---

## Test Suite 2: BRDF Preview (Task 2.2)

### Test 2.1: BRDF Preview Display
**Steps**:
1. Load grassland_demo.toml
2. Expand "BRDF Preview" section
3. Observe sphere preview

**Expected Results**:
- ✅ Sphere renders with lighting (not black)
- ✅ Sphere has gradient shading (highlight + diffuse falloff)
- ✅ Preview updates automatically from material parameters
- ✅ Metallic=0.0, Roughness=0.5 (material defaults)

**Pass Criteria**: Sphere visible with realistic shading

---

### Test 2.2: Material Parameter Controls
**Steps**:
1. Expand "BRDF Preview" → Material Controls
2. Change Albedo to pure red (RGB: 1.0, 0.0, 0.0)
3. Set Metallic to 1.0
4. Set Roughness to 0.0 (smooth)
5. Set Roughness to 1.0 (rough)

**Expected Results**:
- ✅ Albedo change: Sphere turns red
- ✅ Metallic=1.0: Strong specular highlight, no diffuse
- ✅ Roughness=0.0: Sharp, mirror-like highlight
- ✅ Roughness=1.0: Diffuse, no highlight

**Pass Criteria**: Sphere appearance matches parameter changes

---

### Test 2.3: Lighting Controls
**Steps**:
1. Expand "BRDF Preview" → Lighting Controls
2. Change Light X to -1.0 (left side)
3. Change Light Y to 1.0 (top)
4. Set Light Intensity to 0.0
5. Set Light Intensity to 5.0
6. Change Light Color to blue (RGB: 0.0, 0.0, 1.0)

**Expected Results**:
- ✅ X=-1.0: Highlight moves to left side of sphere
- ✅ Y=1.0: Highlight moves to top of sphere
- ✅ Intensity=0.0: Sphere very dark (no light)
- ✅ Intensity=5.0: Sphere very bright (overexposed)
- ✅ Blue light: Sphere has blue tint

**Pass Criteria**: Lighting changes affect sphere appearance correctly

---

### Test 2.4: BRDF Preview Performance
**Steps**:
1. Expand "BRDF Preview"
2. Rapidly change Metallic slider (0.0 → 1.0 → 0.0)
3. Rapidly change Roughness slider (0.0 → 1.0 → 0.0)
4. Observe UI responsiveness

**Expected Results**:
- ✅ Preview updates within 10-20ms (imperceptible lag)
- ✅ UI remains responsive (no freezing)
- ✅ Dirty flag optimization prevents unnecessary renders

**Pass Criteria**: No noticeable lag, UI stays responsive

---

## Test Suite 3: Asset Browser (Task 2.3)

### Test 3.1: Material Discovery
**Steps**:
1. Launch editor (fresh start)
2. Expand "Material Browser"
3. Check available_materials count in info panel

**Expected Results**:
- ✅ Browser discovers all .toml files in assets/materials/
- ✅ At least 3 materials found (grassland, mountain, desert demos)
- ✅ Materials sorted alphabetically
- ✅ Relative paths displayed (e.g., "terrain/grassland_demo.toml")

**Pass Criteria**: All demo materials discovered, no duplicates

---

### Test 3.2: Browser Toggle & Refresh
**Steps**:
1. Click "▶ Show Browser"
2. Observe material list appears
3. Click "▼ Hide Browser"
4. Observe material list hides
5. Click "🔄 Refresh"
6. Observe material list updates

**Expected Results**:
- ✅ Toggle works (list shows/hides)
- ✅ Default state: hidden (collapsed)
- ✅ Refresh rescans directory (finds new materials)
- ✅ Refresh completes in <5ms

**Pass Criteria**: Toggle and refresh work smoothly

---

### Test 3.3: Material History Tracking
**Steps**:
1. Load grassland_demo.toml
2. Load mountain_demo.toml
3. Load desert_demo.toml
4. Check "Recent" dropdown

**Expected Results**:
- ✅ Dropdown shows 3 materials
- ✅ Order: desert (first), mountain, grassland (last)
- ✅ Clicking history item reloads material
- ✅ Material moves to top of history when reloaded

**Pass Criteria**: History tracks correctly (LRU order)

---

### Test 3.4: Manual Path Input
**Steps**:
1. Enter "assets/materials/terrain/grassland_demo.toml" in Path field
2. Click "Load" button
3. Check material loads
4. Check history updated

**Expected Results**:
- ✅ Material loads from typed path
- ✅ Path appears in history dropdown
- ✅ Invalid path shows error message
- ✅ Status updates correctly

**Pass Criteria**: Manual loading works, history updates

---

### Test 3.5: History LRU Eviction
**Steps**:
1. Load 11 different materials (requires creating test TOMLs)
2. Check "Recent" dropdown

**Expected Results**:
- ✅ Dropdown shows only 10 materials (max limit)
- ✅ Oldest material evicted (first loaded material absent)
- ✅ Most recent 10 retained

**Pass Criteria**: LRU cache works correctly (max 10 items)

---

## Test Suite 4: Edge Cases & Error Handling

### Test 4.1: Missing Directory
**Steps**:
1. Rename `assets/materials/` to `assets/materials_backup/`
2. Launch editor
3. Expand "Material Browser" → "▶ Show Browser"

**Expected Results**:
- ✅ No crash on startup
- ✅ Browser shows "No materials found in assets/materials/"
- ✅ Refresh button works (no error)

**Pass Criteria**: Graceful degradation, no panics

---

### Test 4.2: Missing Texture Files
**Steps**:
1. Create test material TOML with non-existent texture paths:
   ```toml
   [material]
   name = "test_missing"
   layers = [
       {albedo = "missing_albedo.png", normal = "missing_normal.png", orm = "missing_orm.png"}
   ]
   ```
2. Load material

**Expected Results**:
- ✅ Material loads (TOML parsing succeeds)
- ✅ Status shows error: "Failed to load texture: missing_albedo.png"
- ✅ Texture viewer shows empty/black placeholder
- ✅ No crash, inspector remains usable

**Pass Criteria**: Graceful error handling, no panics

---

### Test 4.3: Corrupt TOML File
**Steps**:
1. Create invalid TOML file:
   ```toml
   [material
   name = "corrupt"
   # Missing closing bracket
   ```
2. Attempt to load via browser

**Expected Results**:
- ✅ Status shows TOML parsing error
- ✅ Error message indicates line/column of error
- ✅ Inspector remains usable (no crash)

**Pass Criteria**: Error reported, no panic

---

### Test 4.4: Large Texture Files
**Steps**:
1. Create 8K texture (8192×8192) in PNG format
2. Reference in material TOML
3. Load material
4. Observe memory usage (Task Manager)

**Expected Results**:
- ✅ Texture loads (may take 1-5 seconds)
- ✅ Memory increase: ~256MB (8K RGBA8)
- ✅ Zoom/pan work with large texture
- ✅ No crash or out-of-memory error

**Pass Criteria**: Handles large textures, warns if >4K resolution

---

### Test 4.5: Invalid Path Input
**Steps**:
1. Enter invalid paths in manual input field:
   - Empty string
   - "../../../etc/passwd" (directory traversal)
   - "C:\Windows\System32\drivers\etc\hosts" (system file)
   - "nonexistent/path/material.toml"
2. Click "Load" for each

**Expected Results**:
- ✅ Empty string: No action (Load button disabled or shows error)
- ✅ Directory traversal: Error "File not found"
- ✅ System file: Error "Not a valid material TOML"
- ✅ Nonexistent path: Error "File not found"

**Pass Criteria**: All invalid paths handled gracefully, no crashes

---

## Test Suite 5: Integration Testing

### Test 5.1: Multi-Material Workflow
**Steps**:
1. Load grassland_demo.toml
2. Inspect albedo texture (zoom, channel filter)
3. Check BRDF preview (adjust metallic/roughness)
4. Load mountain_demo.toml (via history dropdown)
5. Compare BRDF preview (should auto-update)
6. Load desert_demo.toml (via browser list)

**Expected Results**:
- ✅ All materials load successfully
- ✅ History tracks all 3 materials
- ✅ BRDF preview updates for each material
- ✅ Texture viewer shows correct textures
- ✅ No memory leaks (check Task Manager)

**Pass Criteria**: Workflow completes smoothly, no errors

---

### Test 5.2: Browser + Validation Integration
**Steps**:
1. Run CLI validator on demo materials:
   ```powershell
   cargo run -p aw_asset_cli -- validate assets/materials/terrain/
   ```
2. Load same materials in inspector
3. Check validation results panel

**Expected Results**:
- ✅ CLI validation results match inspector validation
- ✅ Inspector shows same warnings/errors
- ✅ Both tools agree on material validity

**Pass Criteria**: Validation consistency between CLI and GUI

---

### Test 5.3: BRDF Preview + Material Sync
**Steps**:
1. Load material with specific parameters (e.g., metallic=0.8, roughness=0.3)
2. Check BRDF preview matches material
3. Manually adjust BRDF preview parameters
4. Load another material
5. Check BRDF preview resets to new material

**Expected Results**:
- ✅ BRDF preview auto-updates from material TOML
- ✅ Manual adjustments override material values
- ✅ Loading new material resets BRDF preview
- ✅ Material data precedence: loaded TOML > manual adjustments

**Pass Criteria**: BRDF preview syncs correctly with loaded materials

---

## Test Suite 6: Performance & Stress Testing

### Test 6.1: Large Material Database
**Steps**:
1. Create 100+ test material TOMLs in assets/materials/test/
2. Launch editor
3. Measure startup time
4. Click "🔄 Refresh"
5. Measure refresh time

**Expected Results**:
- ✅ Startup discovery: <100ms (for 100 materials)
- ✅ Refresh: <50ms
- ✅ Browser list scrollable (200px max height)
- ✅ No UI lag when scrolling list

**Pass Criteria**: Handles large material databases efficiently

---

### Test 6.2: Rapid Material Switching
**Steps**:
1. Load 5 different materials rapidly (1 per second)
2. Observe UI responsiveness
3. Check memory usage (Task Manager)

**Expected Results**:
- ✅ UI remains responsive
- ✅ Textures load without visible delay
- ✅ BRDF preview updates smoothly
- ✅ Memory usage stable (no leaks)

**Pass Criteria**: No lag, no memory leaks

---

### Test 6.3: BRDF Preview Stress Test
**Steps**:
1. Open BRDF preview
2. Rapidly adjust sliders (metallic, roughness, intensity) for 30 seconds
3. Monitor CPU usage (Task Manager)

**Expected Results**:
- ✅ CPU usage: <10% (single core, software rendering)
- ✅ Preview updates smoothly (10-20ms per frame)
- ✅ Dirty flag prevents unnecessary renders
- ✅ No stuttering or freezing

**Pass Criteria**: CPU usage reasonable, no performance degradation

---

## Troubleshooting Guide

### Issue: Material Browser Empty
**Symptoms**: "No materials found in assets/materials/"

**Solutions**:
1. Check `assets/materials/` directory exists
2. Verify `.toml` files present (not `.txt` or other extensions)
3. Click "🔄 Refresh" to rescan directory
4. Check file permissions (read access required)

---

### Issue: Textures Not Loading
**Symptoms**: Black or missing textures in viewer

**Solutions**:
1. Verify texture files exist (check TOML paths)
2. Check file extensions: `.png`, `.jpg`, `.ktx2`, `.dds` supported
3. Verify file permissions (read access)
4. Check TOML format: `albedo = "relative/path/to/texture.png"`
5. Check logs for image loading errors

---

### Issue: BRDF Preview Black
**Symptoms**: Sphere appears completely black

**Solutions**:
1. Check Light Intensity > 0.0
2. Verify material parameters loaded (albedo not [0,0,0])
3. Check light direction not pointing away from sphere
4. Restart editor (possible render state corruption)

---

### Issue: History Not Updating
**Symptoms**: Recent materials dropdown doesn't show loaded materials

**Solutions**:
1. Ensure material loads successfully (check status message)
2. Use browser or manual input (not hardcoded Load button)
3. Verify `load_material_with_history()` called (not `load_material()`)
4. Check history limit (max 10, oldest evicted)

---

### Issue: Compilation Warnings
**Symptoms**: 3 warnings about unused code

**Expected Behavior**: These warnings are normal:
- `set_lighting` (reserved for future interactive controls)
- `pan_offset` (reserved for texture panning)
- `Split` variant (reserved for comparison mode)

**Action**: No action needed, these are planned features.

---

## Test Completion Checklist

### Core Functionality (Task 2.1)
- [ ] Test 1.1: Load material via browser
- [ ] Test 1.2: Display mode switching
- [ ] Test 1.3: Channel filtering
- [ ] Test 1.4: Color space toggle
- [ ] Test 1.5: Zoom controls

### BRDF Preview (Task 2.2)
- [ ] Test 2.1: BRDF preview display
- [ ] Test 2.2: Material parameter controls
- [ ] Test 2.3: Lighting controls
- [ ] Test 2.4: BRDF preview performance

### Asset Browser (Task 2.3)
- [ ] Test 3.1: Material discovery
- [ ] Test 3.2: Browser toggle & refresh
- [ ] Test 3.3: Material history tracking
- [ ] Test 3.4: Manual path input
- [ ] Test 3.5: History LRU eviction

### Edge Cases
- [ ] Test 4.1: Missing directory
- [ ] Test 4.2: Missing texture files
- [ ] Test 4.3: Corrupt TOML file
- [ ] Test 4.4: Large texture files
- [ ] Test 4.5: Invalid path input

### Integration
- [ ] Test 5.1: Multi-material workflow
- [ ] Test 5.2: Browser + validation integration
- [ ] Test 5.3: BRDF preview + material sync

### Performance
- [ ] Test 6.1: Large material database (100+ materials)
- [ ] Test 6.2: Rapid material switching
- [ ] Test 6.3: BRDF preview stress test

---

## Success Criteria

**Task 2.4 Complete** when:
- ✅ All 18 test cases pass (suites 1-6)
- ✅ No crashes or panics in any test
- ✅ Edge cases handled gracefully
- ✅ Performance acceptable (no lag, <10% CPU for BRDF preview)
- ✅ Documentation complete (this guide + user guide)

---

## Next Steps After Testing

1. **Fix Bugs**: Address any failures found during testing
2. **UI Polish**: Improve spacing, add tooltips, better error messages
3. **Documentation**: Create user guide for material authors
4. **Roadmap Update**: Mark Task 2.4 complete, update progress to ~40%

---

**Version**: 1.0  
**Last Updated**: 2025-10-07  
**Estimated Test Time**: 2-3 hours (comprehensive)
