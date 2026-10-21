# Coverage Push Strategy - Rendering System

**Goal**: Reach 75% test coverage across rendering modules  
**Current Status**: ~60% coverage (estimated)  
**Target Increase**: +15% coverage points  
**Timeline**: 2-3 days of focused test development

---

## Priority 1: IBL Module Tests (+5-10%)

**Module**: `crates/astraweave-render-bevy/src/ibl/`

### Current Coverage Gaps
- **Environment Map Loading**: No tests for HDRI → cubemap conversion
- **Irradiance Map Generation**: Untested convolution and filtering
- **Specular Prefiltering**: No validation of mipmap chain generation
- **BRDF LUT**: Missing tests for integration map creation

### Recommended Tests

#### 1.1 Environment Map Loading Tests
```rust
#[test]
fn test_load_hdri_basic() {
    // Load a simple 512×512 HDR image
    // Verify cubemap faces are created correctly
    // Check texture dimensions and format
}

#[test]
fn test_hdri_fallback_on_missing_file() {
    // Attempt to load non-existent file
    // Verify graceful fallback to default skybox
}

#[test]
fn test_hdri_format_validation() {
    // Test unsupported formats (JPEG, PNG without HDR)
    // Verify error handling
}
```

#### 1.2 Irradiance Convolution Tests
```rust
#[test]
fn test_irradiance_map_generation() {
    // Generate irradiance from test cubemap
    // Verify output dimensions (32×32 per face)
    // Check convolution produces lower frequency
}

#[test]
fn test_irradiance_symmetry() {
    // Use uniform white environment
    // Verify all faces have equal average intensity
}
```

#### 1.3 Specular Prefiltering Tests
```rust
#[test]
fn test_specular_mipmap_chain() {
    // Generate mipmap chain (512 → 256 → ... → 1)
    // Verify each level has correct roughness
    // Check smooth blurring progression
}

#[test]
fn test_specular_roughness_0_equals_environment() {
    // At roughness=0, specular should match environment map
    // Sample both and compare (within tolerance)
}
```

#### 1.4 BRDF LUT Tests
```rust
#[test]
fn test_brdf_lut_dimensions() {
    // Generate 512×512 BRDF integration map
    // Verify texture format (RG16F)
}

#[test]
fn test_brdf_lut_energy_conservation() {
    // Sample LUT at various roughness/NdotV
    // Verify total energy ≤ 1.0 (no free light)
}
```

**Estimated Impact**: +8% coverage (+40 tests @ ~20 LOC each = 800 LOC covered)

---

## Priority 2: Shadow CSM Module Tests (+5-8%)

**Module**: `crates/astraweave-render-bevy/src/shadows/csm/`

### Current Coverage Gaps
- **Cascade Calculation**: No tests for frustum split logic
- **Shadow Map Allocation**: Untested texture array creation
- **Cascade Transitions**: Missing blend zone validation
- **Depth Bias**: No tests for peter-panning/shadow acne fixes

### Recommended Tests

#### 2.1 Cascade Frustum Splitting Tests
```rust
#[test]
fn test_cascade_split_distances() {
    // Create 4 cascades for 0.1-1000m range
    // Verify splits follow logarithmic/PSSM distribution
    // Check near/far boundaries don't overlap
}

#[test]
fn test_cascade_count_validation() {
    // Test 1, 2, 4, 8 cascade configurations
    // Verify power-of-2 enforcement
}

#[test]
fn test_cascade_coverage_no_gaps() {
    // Verify every depth from near→far is in exactly one cascade
}
```

#### 2.2 Shadow Map Resolution Tests
```rust
#[test]
fn test_shadow_map_allocation() {
    // Create 2048×2048 atlas for 4 cascades
    // Verify each cascade gets 1024×1024
}

#[test]
fn test_shadow_texel_density_consistency() {
    // Compare texel/meter ratio across cascades
    // Verify close cascades have higher density
}
```

#### 2.3 Cascade Transition Tests
```rust
#[test]
fn test_cascade_blend_zone() {
    // Sample shadow at cascade boundary
    // Verify smooth transition (no hard edge)
}

#[test]
fn test_shadow_map_updates_on_camera_move() {
    // Move camera significantly
    // Verify cascade matrices update
}
```

#### 2.4 Depth Bias Tests
```rust
#[test]
fn test_depth_bias_prevents_shadow_acne() {
    // Render flat plane with directional light
    // Verify no self-shadowing artifacts
}

#[test]
fn test_depth_bias_slope_scale() {
    // Test on steep slopes (45°, 60°, 75°)
    // Verify dynamic bias scaling
}
```

**Estimated Impact**: +6% coverage (+30 tests @ ~25 LOC each = 750 LOC covered)

---

## Priority 3: Nanite Module Tests (+3-5%)

**Module**: `crates/astraweave-render-bevy/src/nanite/`

### Current Coverage Gaps
- **LOD Selection**: No tests for distance-based simplification
- **Cluster Culling**: Untested frustum and occlusion culling
- **Mesh Streaming**: Missing progressive load validation

### Recommended Tests

#### 3.1 LOD Selection Tests
```rust
#[test]
fn test_lod_distance_thresholds() {
    // Place mesh at 10m, 50m, 200m distances
    // Verify correct LOD level selection (0-7)
}

#[test]
fn test_lod_screen_space_error() {
    // Vary camera FOV and resolution
    // Verify LOD adjusts to maintain pixel error < threshold
}
```

#### 3.2 Cluster Culling Tests
```rust
#[test]
fn test_frustum_culling_basic() {
    // Place clusters inside/outside frustum
    // Verify only visible clusters rendered
}

#[test]
fn test_occlusion_culling_large_occluder() {
    // Place large wall between camera and object
    // Verify hidden clusters skipped
}
```

#### 3.3 Mesh Streaming Tests
```rust
#[test]
fn test_progressive_mesh_load() {
    // Start with LOD 7 (coarsest)
    // Simulate frame updates, verify LOD 0 loads
}

#[test]
fn test_memory_budget_enforcement() {
    // Set 512MB budget, load 1GB of meshes
    // Verify LRU eviction keeps memory under budget
}
```

**Estimated Impact**: +4% coverage (+20 tests @ ~30 LOC each = 600 LOC covered)

---

## Priority 4: Material System Tests (+2-3%)

**Module**: `crates/astraweave-render-bevy/src/materials/`

### Quick Wins

```rust
#[test]
fn test_pbr_material_defaults() {
    // Create material with no textures
    // Verify sane defaults (white albedo, 0.5 roughness, 0 metallic)
}

#[test]
fn test_material_atlas_uv_remapping() {
    // Create 4×4 atlas with 16 materials
    // Verify UV [0,1] maps to correct atlas slot
}

#[test]
fn test_material_texture_loading() {
    // Load albedo, normal, roughness for one material
    // Verify textures bind correctly
}
```

**Estimated Impact**: +2% coverage (+10 tests @ ~15 LOC each = 150 LOC covered)

---

## Implementation Plan

### Week 1: IBL + CSM Tests
**Days 1-2**: IBL module tests (environment, irradiance, specular, BRDF)  
**Days 3-4**: CSM module tests (cascades, shadow maps, transitions, bias)  
**Day 5**: Integration tests (IBL + CSM together)

### Week 2: Nanite + Materials
**Days 1-2**: Nanite tests (LOD, culling, streaming)  
**Day 3**: Material system tests  
**Day 4**: Coverage analysis and gap filling  
**Day 5**: Documentation and cleanup

---

## Success Metrics

| Module          | Current | Target | Gain   |
|-----------------|---------|--------|--------|
| IBL             | ~40%    | 85%    | +45%   |
| CSM Shadows     | ~50%    | 90%    | +40%   |
| Nanite          | ~55%    | 75%    | +20%   |
| Materials       | ~70%    | 85%    | +15%   |
| **Overall**     | **60%** | **75%**| **+15%**|

---

## Trade-Offs & Limitations

### What's NOT Being Tested (Intentionally)
1. **UI/Debug Overlays**: Low-value, high-maintenance tests
2. **Platform-Specific Code**: Covered by CI across Windows/Linux/macOS
3. **Performance Benchmarks**: Separate benchmark suite exists
4. **Visual Regression Tests**: Requires image comparison infrastructure

### Known Gaps (Future Work)
- **GPU Driver Validation**: Can't test vendor-specific bugs in CI
- **Multi-GPU Scenarios**: Requires specialized hardware
- **VR/AR Rendering Paths**: Not in current scope

---

## Execution Notes

- Run `cargo tarpaulin` after each test batch to track progress
- Use `#[cfg(test)]` modules to keep test code separate
- Mock GPU resources using `wgpu::Instance::new()` with dummy adapters
- Leverage existing test helpers in `crates/astraweave-render-bevy/tests/common/`

**Estimated Total Effort**: 8-10 days (including documentation)  
**Risk Level**: Low (all modules have integration tests, adding unit coverage)  
**Dependencies**: None (tests are independent)
