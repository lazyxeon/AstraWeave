# Phase PBR-E: Integration Testing & Validation Guide

**Status**: Ready for Visual Testing  
**Date**: October 2025  
**Build Status**: ✅ Clean compilation (0 errors, 1.06s)

---

## Quick Start

```powershell
# Build and run unified_showcase in release mode (recommended for performance)
cargo run -p unified_showcase --release

# Controls (press F5 to enable PBR-E demo mode):
# F5 - Toggle PBR-E demo ON/OFF
# F6 - Cycle through material types (clearcoat → anisotropy → subsurface → sheen → transmission)
# F7 - Decrease grid size (min 3x3 = 9 spheres)
# F8 - Increase grid size (max 10x10 = 100 spheres)
```

---

## Test Procedure

### Phase 1: Baseline Verification (5 minutes)

**Objective**: Verify normal rendering still works

1. **Launch Application**:
   ```powershell
   cargo run -p unified_showcase --release
   ```

2. **Expected Behavior**:
   - ✅ Window opens with 3D terrain/environment
   - ✅ No console errors or panics
   - ✅ Smooth camera movement (WASD + mouse)
   - ✅ Normal rendering visible (terrain, trees, buildings if present)

3. **Acceptance Criteria**:
   - Application launches without errors
   - Frame rate >30 FPS in normal mode
   - No visual artifacts or crashes

---

### Phase 2: PBR-E Demo Activation (10 minutes)

**Objective**: Enable PBR-E demo and verify sphere grid rendering

1. **Enable Demo Mode**:
   - Press `F5` to toggle PBR-E demo
   - Console should print:
     ```
     PBR-E Demo: ENABLED | Material: Clearcoat | Grid: 5x5
     ```

2. **Expected Behavior**:
   - ✅ Terrain/environment replaced with 5x5 grid of spheres (25 total)
   - ✅ Each sphere has unique material parameters (sweep from X=0 to X=1, Y=0 to Y=1)
   - ✅ Spheres evenly spaced in grid pattern
   - ✅ Lighting applies correctly to all spheres

3. **Visual Inspection**:
   - **Sphere Visibility**: All 25 spheres visible in viewport
   - **Grid Arrangement**: Spheres arranged in orderly 5x5 grid
   - **Parameter Sweep**: Visual variation across X and Y axes
   - **Lighting**: Spheres respond to directional light (sun)
   - **Camera Movement**: Can orbit around sphere grid

4. **Troubleshooting**:
   - **No Spheres Visible**: Check console for errors, verify mesh_type==6u in shader
   - **Black Spheres**: SSBO binding may be missing, check group 6 is set
   - **Crash on F5**: Check material_id range, verify SSBO buffer size

5. **Acceptance Criteria**:
   - Sphere grid renders without errors
   - Material variations visible across grid
   - Frame rate >30 FPS with 25 spheres

---

### Phase 3: Material Type Testing (30 minutes)

**Objective**: Verify all 5 advanced material types render correctly

#### Test 3.1: Clearcoat (Car Paint / Lacquer)

1. **Activate Material**:
   - Press `F6` until console shows: `PBR-E Material: Clearcoat`
   - Or start from default (clearcoat is first)

2. **Parameter Sweep**:
   - **X-axis**: Clearcoat Strength (0.0 → 1.0, left to right)
   - **Y-axis**: Clearcoat Roughness (0.0 → 1.0, bottom to top)

3. **Expected Visual Results**:
   - **Dual Specular Lobes**: Base layer + coating layer visible
   - **Left Edge** (strength=0): No clearcoat, standard PBR appearance
   - **Right Edge** (strength=1): Strong coating with sharp reflection
   - **Bottom Row** (roughness=0): Mirror-like coating reflection
   - **Top Row** (roughness=1): Diffuse, matte coating layer

4. **Physics Reference**:
   - Clearcoat models lacquer/varnish with IOR 1.5 (F0=0.04)
   - Energy splits: coating takes F_coat, base gets (1-F_coat)
   - Examples: Car paint, lacquered wood, glossy plastic

5. **Screenshot**: `unified_showcase_pbr_e_clearcoat.png`

---

#### Test 3.2: Anisotropy (Brushed Metal / Hair)

1. **Activate Material**:
   - Press `F6` to cycle to: `PBR-E Material: Anisotropy`

2. **Parameter Sweep**:
   - **X-axis**: Anisotropy Strength (-1.0 → 1.0, left to right)
   - **Y-axis**: Rotation (0 → 2π radians, bottom to top)

3. **Expected Visual Results**:
   - **Elliptical Highlights**: Directional specular reflection (not circular)
   - **Center** (strength=0): Isotropic, standard round highlights
   - **Left Edge** (strength=-1): Anisotropy in one direction
   - **Right Edge** (strength=1): Anisotropy in opposite direction
   - **Y-axis Variation**: Highlight direction rotates 360° from bottom to top

4. **Physics Reference**:
   - Anisotropic GGX uses separate α_t and α_b roughness
   - Tangent/bitangent define local coordinate system
   - Examples: Brushed aluminum, hair, satin fabric

5. **Screenshot**: `unified_showcase_pbr_e_anisotropy.png`

---

#### Test 3.3: Subsurface Scattering (Skin / Wax)

1. **Activate Material**:
   - Press `F6` to cycle to: `PBR-E Material: Subsurface`

2. **Parameter Sweep**:
   - **X-axis**: Subsurface Scale (0.0 → 1.0, left to right)
   - **Y-axis**: Radius (0 → 5mm, bottom to top, normalized 0→1)

3. **Expected Visual Results**:
   - **Soft, Translucent Appearance**: Light penetrates and scatters
   - **Wrapped Diffuse**: Forward + backscattering visible
   - **Left Edge** (scale=0): No SSS, standard Lambertian
   - **Right Edge** (scale=1): Strong subsurface scattering
   - **Bottom Row** (radius=0): Minimal scattering distance
   - **Top Row** (radius=5mm): Wide scattering, very soft diffuse

4. **Physics Reference**:
   - Burley diffusion profile (Disney BSDF 2015)
   - Wrapped diffuse: `NdotL_wrapped = (NdotL + scale) / (1 + scale)`
   - Examples: Skin, marble, wax, milk

5. **Screenshot**: `unified_showcase_pbr_e_subsurface.png`

---

#### Test 3.4: Sheen (Velvet / Fabric)

1. **Activate Material**:
   - Press `F6` to cycle to: `PBR-E Material: Sheen`

2. **Parameter Sweep**:
   - **X-axis**: Sheen Intensity (0.0 → 1.0, left to right)
   - **Y-axis**: Sheen Roughness (0.0 → 1.0, bottom to top)

3. **Expected Visual Results**:
   - **Retroreflection**: Bright halo at grazing angles (edges of sphere)
   - **Left Edge** (intensity=0): No sheen, standard appearance
   - **Right Edge** (intensity=1): Strong sheen glow at edges
   - **Bottom Row** (roughness=0): Sharp, concentrated sheen
   - **Top Row** (roughness=1): Soft, diffuse sheen glow

4. **Physics Reference**:
   - Charlie distribution (inverted Gaussian, Disney Sheen 2015)
   - Peaks at grazing angles: `sheen ∝ (1 - VdotN)^5`
   - Examples: Velvet, microfiber fabric, peach fuzz

5. **Screenshot**: `unified_showcase_pbr_e_sheen.png`

---

#### Test 3.5: Transmission (Glass / Water)

1. **Activate Material**:
   - Press `F6` to cycle to: `PBR-E Material: Transmission`

2. **Parameter Sweep**:
   - **X-axis**: Transmission Factor (0.0 → 1.0, left to right)
   - **Y-axis**: IOR (Index of Refraction, 1.0 → 2.5, bottom to top)

3. **Expected Visual Results**:
   - **Transparency and Refraction**: Light passes through material
   - **Left Edge** (transmission=0): Opaque, standard PBR
   - **Right Edge** (transmission=1): Fully transparent/refractive
   - **Bottom Row** (IOR=1.0): Air (no refraction)
   - **Top Row** (IOR=2.5): Diamond-like (strong refraction)
   - **Beer-Lambert Attenuation**: Color tint from absorption

4. **Physics Reference**:
   - Fresnel-dielectric with exact Snell's law refraction
   - Beer-Lambert absorption: `I = I₀ * exp(-distance / attenuation_distance)`
   - Total Internal Reflection (TIR) when angle exceeds critical
   - Examples: Glass, water, gemstones, ice

5. **Screenshot**: `unified_showcase_pbr_e_transmission.png`

---

### Phase 4: Grid Size Testing (15 minutes)

**Objective**: Verify performance with different instance counts

#### Test 4.1: 3x3 Grid (9 Spheres)

1. **Set Grid Size**:
   - Press `F7` repeatedly until console shows: `PBR-E Grid Size: 3x3 (9 spheres)`

2. **Expected Behavior**:
   - ✅ 9 spheres visible in 3x3 arrangement
   - ✅ Parameter sweeps still visible (coarser steps)
   - ✅ Very high frame rate (>100 FPS expected)

#### Test 4.2: 5x5 Grid (25 Spheres) - Default

1. **Set Grid Size**:
   - Default grid size, or press F7/F8 to reach 5x5

2. **Expected Behavior**:
   - ✅ 25 spheres visible in 5x5 arrangement
   - ✅ Good parameter resolution (5 steps per axis)
   - ✅ High frame rate (>60 FPS expected)

#### Test 4.3: 8x8 Grid (64 Spheres)

1. **Set Grid Size**:
   - Press `F8` repeatedly until console shows: `PBR-E Grid Size: 8x8 (64 spheres)`

2. **Expected Behavior**:
   - ✅ 64 spheres visible in 8x8 arrangement
   - ✅ Fine parameter resolution (8 steps per axis)
   - ✅ Good frame rate (>40 FPS expected)

#### Test 4.4: 10x10 Grid (100 Spheres) - Performance Stress Test

1. **Set Grid Size**:
   - Press `F8` repeatedly until max: `PBR-E Grid Size: 10x10 (100 spheres)`

2. **Expected Behavior**:
   - ✅ 100 spheres visible in 10x10 arrangement
   - ✅ Very fine parameter resolution (10 steps per axis)
   - ✅ **Target: >30 FPS** (acceptance criterion for performance)

3. **Performance Metrics** (record these):
   - Frame time (ms)
   - Frame rate (FPS)
   - GPU time (if profiler available)
   - Memory usage

4. **Acceptance Criteria**:
   - Frame rate ≥30 FPS with 100 spheres
   - No visual artifacts or stuttering
   - Smooth camera movement maintained

---

### Phase 5: Integration Testing (20 minutes)

**Objective**: Verify seamless transition between demo and normal modes

#### Test 5.1: Mode Switching

1. **Toggle Sequence**:
   - Start in normal mode (terrain/environment visible)
   - Press `F5` → PBR-E demo enabled (spheres appear)
   - Press `F5` again → Demo disabled (terrain returns)
   - Repeat 5 times

2. **Expected Behavior**:
   - ✅ Instant mode switching (no delay or freeze)
   - ✅ No memory leaks (RAM usage stable after 5 cycles)
   - ✅ No visual artifacts when switching
   - ✅ Camera position preserved across switches

#### Test 5.2: Material Cycling

1. **Cycle All Materials**:
   - Press `F5` to enable demo
   - Press `F6` repeatedly to cycle through all 5 materials
   - Verify each material renders correctly
   - Return to first material (clearcoat) after 5 presses

2. **Expected Behavior**:
   - ✅ Clean transition between materials (no flicker)
   - ✅ Material SSBO updated correctly
   - ✅ Console logging shows correct material name
   - ✅ Visual appearance matches material type

#### Test 5.3: Grid Size Extremes

1. **Min to Max Sequence**:
   - Press `F5` to enable demo
   - Press `F7` repeatedly to reach 3x3 (minimum)
   - Press `F8` repeatedly to reach 10x10 (maximum)
   - Press `F7` repeatedly to return to 3x3

2. **Expected Behavior**:
   - ✅ SSBO buffer resizes automatically
   - ✅ No crashes or errors when resizing
   - ✅ Instance count updates correctly
   - ✅ Performance degrades gracefully (no stuttering)

#### Test 5.4: Stress Test

1. **Rapid Input Sequence**:
   - Press `F5` (enable), `F6` (cycle), `F8` (increase), `F5` (disable) rapidly
   - Repeat 10 times in quick succession

2. **Expected Behavior**:
   - ✅ No crashes or panics
   - ✅ State remains consistent
   - ✅ No visual corruption
   - ✅ Frame rate recovers after input stops

---

## Acceptance Criteria Summary

### Critical (Must Pass)

| Criterion | Status | Notes |
|-----------|--------|-------|
| Clean compilation (0 errors) | ✅ Pass | 1.06s build time |
| Application launches | ⏳ Test | No errors expected |
| PBR-E demo toggle works | ⏳ Test | F5 keyboard shortcut |
| All 5 materials render | ⏳ Test | clearcoat, anisotropy, SSS, sheen, transmission |
| Grid size changes work | ⏳ Test | 3x3 to 10x10 range |
| Performance target (≥30 FPS @ 100 spheres) | ⏳ Test | Release mode required |

### Important (Should Pass)

| Criterion | Status | Notes |
|-----------|--------|-------|
| Material parameter sweeps visible | ⏳ Test | X/Y axis variations clear |
| No visual artifacts | ⏳ Test | No z-fighting, flicker, corruption |
| Smooth mode switching | ⏳ Test | Demo ↔ normal seamless |
| SSBO buffer resizes correctly | ⏳ Test | No crashes when changing grid |
| Console logging accurate | ⏳ Test | Correct material names, grid sizes |

### Nice-to-Have (Optional)

| Criterion | Status | Notes |
|-----------|--------|-------|
| >60 FPS @ 25 spheres | ⏳ Test | Expected with RTX 3060 Ti class |
| Real-time material switching | ⏳ Test | <100ms latency ideal |
| UI polish (tooltips, help text) | ⏳ Future | egui panel optional |

---

## Known Issues & Limitations

### Current Implementation

1. **Simplified Evaluation**: `evaluate_pbr_e_material()` is a simplified demo version
   - Full implementation in `pbr_advanced.wgsl` (~450 lines) not yet integrated
   - Current version demonstrates features but not production-quality BRDF
   - Future: Replace with full GGX, Smith geometry, energy conservation

2. **No Screen-Space SSS**: Subsurface scattering uses wrapped diffuse only
   - Screen-space SSS (separable blur) not implemented
   - Future: Add Jimenez 2015 separable SSS for higher quality

3. **No Refraction Environment**: Transmission doesn't sample refracted environment
   - Current: Simple transparency blend with attenuation color
   - Future: Ray-traced or screen-space refraction

4. **Fixed Sun Position**: Light direction static (time-based but not interactive)
   - Future: Add light position/color controls

### Performance Notes

- **GPU Budget**: ~370-510 ALU ops per pixel (all features enabled)
- **Bottlenecks**: SSBO access, complex BRDF evaluation, overdraw
- **Optimizations**: Material sorting not implemented (future enhancement)

### Visual Validation

- **Reference Images**: No ground truth screenshots yet (capture during testing)
- **Comparison**: Should match academic papers (Burley 2015, Karis 2013)
- **Quality**: Simplified evaluation may differ from full BRDF lobes

---

## Troubleshooting Guide

### Issue: No Spheres Visible

**Symptoms**: Press F5, console shows "ENABLED", but no spheres render

**Possible Causes**:
1. Mesh generation failed → Check `generate_pbr_e_demo_instances()` logs
2. SSBO buffer not created → Check `pbr_e_material_buffer.is_some()`
3. Bind group not set → Verify `rp.set_bind_group(6, ...)` called
4. Shader not using material_id → Check `mesh_type == 6u` branch in fragment shader

**Solution**:
- Check console for errors
- Add debug prints in `generate_pbr_e_demo_instances()`
- Verify `render.pbr_e_material_bind_group.is_some()` before rendering

---

### Issue: Black Spheres

**Symptoms**: Spheres render but all solid black

**Possible Causes**:
1. SSBO binding missing → Group 6 not set in render pass
2. Material data incorrect → SSBO contains zeros
3. Lighting calculation broken → `evaluate_pbr_e_material()` returns black

**Solution**:
- Verify bind group 6 set: `if ui.pbr_e_demo_enabled && render.pbr_e_material_bind_group.is_some()`
- Check SSBO buffer size: `materials.len() * 256 bytes`
- Add debug color output in shader: `return vec4<f32>(1.0, 0.0, 1.0, 1.0);` to test pipeline

---

### Issue: Crash on F5 Toggle

**Symptoms**: Application panics when enabling PBR-E demo

**Possible Causes**:
1. SSBO buffer size mismatch → `material_id` out of bounds
2. Device/queue invalid → Renderer not initialized
3. Memory allocation failed → Large grid size (10x10)

**Solution**:
- Reduce grid size to 3x3 and test
- Check `material_id < materials.len()` in instance generation
- Verify device/queue valid before buffer creation

---

### Issue: Low Frame Rate

**Symptoms**: FPS <30 with 100 spheres

**Possible Causes**:
1. Debug mode → Use `cargo run --release`
2. Overdraw → Spheres overlapping in viewport
3. GPU thermal throttling → Check GPU temps
4. Background processes → Close other GPU-intensive apps

**Solution**:
- **Always use release mode**: `--release` flag critical for performance
- Position camera to see all spheres without overlap
- Monitor GPU usage with Task Manager or GPU-Z
- Target: >30 FPS @ 100 spheres in release mode

---

## Performance Benchmarking

### Benchmark Procedure

1. **Setup**:
   - Build release: `cargo build -p unified_showcase --release`
   - Close all background apps
   - Enable PBR-E demo: Press F5
   - Set material type: Press F6 to select material

2. **Measure Frame Times**:
   - 3x3 grid (9 spheres): Record avg FPS for 10 seconds
   - 5x5 grid (25 spheres): Record avg FPS for 10 seconds
   - 8x8 grid (64 spheres): Record avg FPS for 10 seconds
   - 10x10 grid (100 spheres): Record avg FPS for 10 seconds

3. **Record Results**:
   ```
   Material: <Clearcoat|Anisotropy|Subsurface|Sheen|Transmission>
   Grid Size | Instance Count | Avg FPS | Min FPS | Frame Time (ms)
   -----------|----------------|---------|---------|----------------
   3x3        | 9              |         |         |
   5x5        | 25             |         |         |
   8x8        | 64             |         |         |
   10x10      | 100            |         |         |
   ```

4. **Repeat for All Materials** (5 materials × 4 grid sizes = 20 data points)

### Expected Performance (RTX 3060 Ti / Desktop)

| Grid Size | Instances | Target FPS | Expected FPS |
|-----------|-----------|------------|--------------|
| 3x3       | 9         | >100 FPS   | 200-400 FPS  |
| 5x5       | 25        | >60 FPS    | 100-200 FPS  |
| 8x8       | 64        | >40 FPS    | 60-120 FPS   |
| 10x10     | 100       | >30 FPS    | 40-80 FPS    |

---

## Screenshot Capture Checklist

### Required Screenshots (5 minimum)

- [ ] `unified_showcase_pbr_e_clearcoat.png` - 5x5 grid, clearcoat material
- [ ] `unified_showcase_pbr_e_anisotropy.png` - 5x5 grid, anisotropy material
- [ ] `unified_showcase_pbr_e_subsurface.png` - 5x5 grid, subsurface material
- [ ] `unified_showcase_pbr_e_sheen.png` - 5x5 grid, sheen material
- [ ] `unified_showcase_pbr_e_transmission.png` - 5x5 grid, transmission material

### Optional Screenshots

- [ ] `unified_showcase_pbr_e_clearcoat_10x10.png` - High resolution demo
- [ ] `unified_showcase_pbr_e_comparison.png` - All 5 materials in one image (montage)

### Camera Position Recommendations

- **Distance**: 12-15 units from grid center
- **Height**: 5-8 units above grid
- **Angle**: 30-45° looking down
- **Field of View**: Default (ensure all spheres visible)

---

## Success Metrics

### Phase PBR-E Integration: COMPLETE ✅

- ✅ **Code Complete** (6/6 tasks done):
  - Demo module (pbr_e_demo.rs)
  - Shader updates (material_id pipeline)
  - UI state extension (3 fields)
  - Renderer wiring (GPU infrastructure)
  - Shader integration (SSBO, eval function)
  - UI controls (keyboard shortcuts F5/F6/F7/F8)

- ✅ **Compilation** (2/2 checks):
  - Clean build (0 errors, 1.06s)
  - All warnings non-blocking (pre-existing)

- ⏳ **Visual Validation** (0/5 materials tested):
  - Clearcoat rendering
  - Anisotropy rendering
  - Subsurface rendering
  - Sheen rendering
  - Transmission rendering

- ⏳ **Performance** (0/1 benchmarks):
  - ≥30 FPS @ 100 spheres (target)

- ⏳ **Documentation** (1/2 docs):
  - Testing guide (this document) ✅
  - Final integration report (pending visual testing)

---

## Next Steps

### Immediate (Ready Now)

1. **Run Visual Tests**: `cargo run -p unified_showcase --release`
2. **Capture Screenshots**: All 5 material types at 5x5 grid
3. **Benchmark Performance**: Record FPS for each grid size
4. **Document Results**: Update acceptance criteria with test status

### Short-Term (After Visual Validation)

1. **Create Final Integration Report** (`PBR_E_INTEGRATION_FINAL_REPORT.md`)
2. **Update Roadmap**: Mark Phase PBR-E as 100% complete
3. **Commit Changes**: Git commit with comprehensive message
4. **Optional**: Add egui panel for more user-friendly controls

### Long-Term (Production Enhancements)

1. **Full BRDF Integration**: Replace simplified eval with pbr_advanced.wgsl
2. **Screen-Space SSS**: Implement separable blur for subsurface
3. **Material Sorting**: Optimize batch rendering by material_id
4. **Artist Tools**: TOML authoring for advanced materials
5. **Performance Profiling**: GPU time breakdown, optimization opportunities

---

**Document Version**: 1.0  
**Last Updated**: October 2025  
**Status**: Ready for Visual Testing ⏳  
**Estimated Test Time**: 90 minutes (comprehensive)
