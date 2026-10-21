# AstraWeave Rendering System: Root Cause Analysis & Implementation Plan

**Document Version:** 1.0  
**Date:** November 11, 2025  
**Status:** Analysis Complete, Implementation In Progress  
**Target:** World-Class Production Rendering System

---

## Executive Summary

This document provides a comprehensive root cause analysis of rendering issues in `examples/unified_showcase` and defines a long-term, production-grade implementation plan. All identified issues stem from architectural limitations in the current single-material-per-mesh rendering pipeline and inadequate texture/asset handling.

**Key Findings:**
- Current architecture supports only one material per draw call, preventing multi-material terrain and biome blending
- Texture format handling is incorrect (sRGB vs Linear confusion for normal/MRA maps)
- GLTF loader lacks robust UV and tangent generation fallbacks
- Procedural geometry uses flat per-triangle normals instead of smooth per-vertex normals
- No asset preprocessing pipeline for texture format normalization

**Strategic Direction:**
Implement a **material-per-vertex system** with texture arrays and proper PBR material handling, enabling:
- Multi-material terrain with smooth biome blending
- Efficient GPU instancing with material variations
- Proper linear/sRGB texture handling
- Production-quality asset pipeline with validation

---

## Issue Classification & Priority

### P0 - Architectural (Blocking World-Class Quality)

**Issue 1: Single Material Per Draw Call Architecture**
- **Root Cause:** Shader accepts one material bind group `@group(1)` with fixed albedo/normal/roughness textures
- **Impact:** Entire terrain renders as single material (grass); no biome blending, height-based materials, or per-object material variation without multiple draw calls
- **Symptom:** Solid green terrain, inability to mix materials organically
- **Affects:** 
  - `examples/unified_showcase/src/pbr_shader.wgsl` (bind group structure)
  - `examples/unified_showcase/src/main_bevy_v2.rs` lines 1060-1300 (bind group layout)
  - Render pass lines 2148-2268 (draw calls hardcoded to single material)
- **Long-Term Solution:** Implement per-vertex material ID system with texture arrays (see Implementation Plan below)

**Issue 2: Incorrect Texture Format Handling (sRGB vs Linear)**
- **Root Cause:** All textures loaded as `Rgba8UnormSrgb` regardless of intended color space
- **Impact:** Normal maps and MRA (Metallic/Roughness/AO) textures receive incorrect gamma correction, causing lighting artifacts and incorrect material appearance
- **Symptom:** Flat lighting, incorrect specular response, wrong material properties
- **Affects:**
  - `examples/unified_showcase/src/main_bevy_v2.rs` lines 830-875 (`load_texture`)
  - Lines 876-940 (`load_or_generate_texture`)
  - Lines 1190-1310 (material texture loading)
- **Long-Term Solution:** Implement `TextureUsage` enum with proper format selection per texture type

**Issue 3: GLTF Loader Missing UV/Tangent Generation**
- **Root Cause:** Loader returns zero UVs when `read_tex_coords(0)` is None; no tangent generation for normal mapping
- **Impact:** Imported models render white or with incorrect textures; normal maps don't work correctly
- **Symptom:** White boxes for buildings, NPCs; "No building models found" fallback
- **Affects:**
  - `examples/unified_showcase/src/gltf_loader.rs` lines 60-75 (UV fallback)
  - Lines 80-100 (missing tangent generation)
- **Long-Term Solution:** Implement automatic UV generation (planar/spherical/triplanar) and mikktspace tangent calculation

### P1 - Quality (High Visual Impact)

**Issue 4: Flat Shading (Per-Triangle Normals)**
- **Root Cause:** `create_ground_plane` sets same normal for all 3 triangle vertices instead of averaging per-vertex
- **Impact:** Harsh lighting transitions, faceted appearance instead of smooth terrain
- **Symptom:** Flat-shaded triangles visible on terrain
- **Affects:**
  - `examples/unified_showcase/src/main_bevy_v2.rs` lines 620-650 (normal calculation in terrain generation)
- **Long-Term Solution:** Implement per-vertex normal averaging with optional smoothing groups

**Issue 5: No Mipmapping**
- **Root Cause:** Textures created with `mip_level_count: 1`
- **Impact:** Texture aliasing, shimmering at distance, poor performance due to cache misses
- **Symptom:** Moiré patterns, texture shimmer on terrain at oblique angles
- **Affects:**
  - `load_texture` lines 840-850
- **Long-Term Solution:** Generate full mip chains (CPU or GPU) with proper filtering

### P2 - Robustness (Developer Experience)

**Issue 6: Poor Error Diagnostics in GLTF Loading**
- **Root Cause:** Generic error messages without path, primitive, or accessor details
- **Impact:** Difficult to debug why specific models fail to load
- **Affects:**
  - `gltf_loader.rs` error handling
- **Long-Term Solution:** Structured error types with detailed context

**Issue 7: No Asset Preprocessing Pipeline**
- **Root Cause:** Manual texture conversion required (indexed PNG → RGBA)
- **Impact:** Developer friction, inconsistent asset quality
- **Long-Term Solution:** Automated asset validation and conversion on import

---

## Long-Term Architecture: Material-Per-Vertex System

### Design Goals
1. **Efficient multi-material rendering** without draw call overhead
2. **Smooth material blending** for terrain biomes and transitions
3. **Scalable to 100+ materials** without bind group limitations
4. **Compatible with GPU instancing** for performance
5. **PBR-correct** with proper linear/sRGB handling

### System Design

#### 1. Extended Vertex Format
```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    material_id: u32,        // NEW: Index into texture array
    blend_weights: [f32; 4], // NEW: For multi-material blending (optional)
}
```

#### 2. Texture Array Material System
```wgsl
// Bind Group 1: Material Texture Arrays
@group(1) @binding(0) var albedo_array: texture_2d_array<f32>;
@group(1) @binding(1) var texture_sampler: sampler;
@group(1) @binding(2) var normal_array: texture_2d_array<f32>;
@group(1) @binding(3) var mra_array: texture_2d_array<f32>;

// Fragment shader samples by material_id
let albedo = textureSample(albedo_array, texture_sampler, vec3(in.uv, f32(in.material_id)));
```

#### 3. Texture Format Management
```rust
enum TextureUsage {
    Albedo,    // Rgba8UnormSrgb (gamma-corrected)
    Normal,    // Rgba8Unorm (linear space)
    MRA,       // Rgba8Unorm (linear space)
    Emissive,  // Rgba8UnormSrgb (gamma-corrected)
}

fn load_texture_with_usage(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    path: &str,
    usage: TextureUsage,
    generate_mips: bool,
) -> Result<wgpu::Texture>
```

#### 4. Material Atlas Generation
- Pack textures into 2D arrays (max 256-2048 layers depending on hardware)
- Standardize texture sizes (1024×1024 or 2048×2048)
- Generate mipmaps for each layer
- Maintain material ID → array index mapping

#### 5. Terrain Material Blending
```rust
// Assign materials based on height, slope, and noise
fn assign_terrain_materials(height: f32, slope: f32, noise: f32) -> (u32, [f32; 4]) {
    let base_material = if height < 2.0 {
        MATERIAL_GRASS
    } else if height < 5.0 {
        MATERIAL_DIRT
    } else {
        MATERIAL_ROCK
    };
    
    // Calculate blend weights for smooth transitions
    let blend = calculate_blend_weights(height, slope, noise);
    (base_material, blend)
}
```

---

## Implementation Plan (Phased Approach)

### Phase 1: Foundation (Week 1, Days 1-3)
**Goal:** Establish proper texture handling and asset pipeline

#### Task 1.1: Texture Format System
- [ ] Create `TextureUsage` enum
- [ ] Refactor `load_texture` to accept usage parameter
- [ ] Update all texture loading call sites
- [ ] Add format validation tests

**Files:**
- `examples/unified_showcase/src/texture_manager.rs` (NEW)
- `examples/unified_showcase/src/main_bevy_v2.rs` (refactor)

**Acceptance:**
- Unit tests verify correct format per usage
- Visual test: normal maps produce correct lighting
- Zero format-related artifacts

#### Task 1.2: Mipmap Generation
- [ ] Implement CPU mip generation using `image` crate
- [ ] Add optional GPU mip generation path
- [ ] Update texture descriptor to use computed mip levels
- [ ] Add mipmap quality tests

**Acceptance:**
- Textures have full mip chains
- No aliasing or shimmer at distance
- Performance: <5ms overhead for all textures on load

#### Task 1.3: Asset Validation Pipeline
- [ ] Create `tools/asset_validator` CLI tool
- [ ] Implement texture format detection and conversion
- [ ] Add GLTF validation (check UVs, normals, tangents)
- [ ] Integrate into build process

**Acceptance:**
- Automated conversion of indexed PNGs to RGBA
- Warning on sRGB/Linear mismatches
- GLTF assets validated on import

### Phase 2: Vertex Material System (Week 1, Days 4-5)
**Goal:** Enable per-vertex material assignment

#### Task 2.1: Extended Vertex Format
- [ ] Add `material_id` and `blend_weights` to Vertex struct
- [ ] Update vertex buffer layout
- [ ] Update all procedural geometry generators
- [ ] Modify GLTF loader to populate material_id

**Files:**
- `examples/unified_showcase/src/main_bevy_v2.rs` lines 150-170

**Acceptance:**
- Vertex struct includes material fields
- All geometry generators compile and run
- Memory layout validated (alignment, padding)

#### Task 2.2: Terrain Material Assignment
- [ ] Implement height-based material selection
- [ ] Add slope and noise factors for variation
- [ ] Calculate smooth blend weights at boundaries
- [ ] Update `create_ground_plane` to set material_id per vertex

**Acceptance:**
- Terrain vertices have varied material IDs
- Smooth transitions at biome boundaries
- Performance: <10ms for 10k vertex terrain

### Phase 3: Texture Array System (Week 2, Days 1-3)
**Goal:** Implement material texture arrays and shader sampling

#### Task 3.1: Material Atlas Generator
- [ ] Create `MaterialAtlas` struct and builder
- [ ] Implement texture packing into 2D arrays
- [ ] Generate material ID → array index mapping
- [ ] Handle texture resizing and format normalization

**Files:**
- `examples/unified_showcase/src/material_atlas.rs` (NEW)

**Acceptance:**
- Pack 7+ materials into texture arrays
- All textures 1024×1024 with mipmaps
- <50MB total VRAM for material atlas

#### Task 3.2: Shader Update
- [ ] Replace single texture bindings with texture_2d_array
- [ ] Update fragment shader to sample by material_id
- [ ] Implement 4-way material blending (using blend_weights)
- [ ] Add blend quality controls (trilinear, anisotropic)

**Files:**
- `examples/unified_showcase/src/pbr_shader.wgsl`

**Acceptance:**
- Shader compiles without errors
- Correct texture sampling per material_id
- Visual: multiple materials visible on single mesh

#### Task 3.3: Bind Group Refactor
- [ ] Create material atlas bind group layout
- [ ] Update pipeline to use texture arrays
- [ ] Remove per-material bind group creation loop
- [ ] Simplify render pass to single bind group

**Files:**
- `examples/unified_showcase/src/main_bevy_v2.rs` lines 1100-1300

**Acceptance:**
- Single material bind group for all draws
- Reduced bind group switching overhead
- Performance: 2-3× faster material binding

### Phase 4: GLTF Robustness (Week 2, Days 4-5)
**Goal:** Production-quality asset loading

#### Task 4.1: UV Generation
- [ ] Implement planar UV projection
- [ ] Implement spherical UV projection
- [ ] Implement triplanar UV projection (for complex geometry)
- [ ] Add heuristic to choose projection method

**Files:**
- `examples/unified_showcase/src/gltf_loader.rs`
- `examples/unified_showcase/src/uv_generator.rs` (NEW)

**Acceptance:**
- Models without UVs receive generated coordinates
- Visual: previously white models now textured
- Unit tests for each projection method

#### Task 4.2: Tangent Generation
- [ ] Integrate mikktspace algorithm (via `mikktspace` crate)
- [ ] Generate tangents when normals exist but tangents missing
- [ ] Handle edge cases (degenerate triangles, zero-area UVs)

**Acceptance:**
- Normal maps work on all loaded models
- Tangent handedness correct (no inverted lighting)
- <20ms tangent generation for 10k vertex mesh

#### Task 4.3: Normal Smoothing
- [ ] Implement per-vertex normal averaging
- [ ] Add optional smoothing groups support
- [ ] Handle hard edges (smoothing angle threshold)
- [ ] Update terrain and procedural generators

**Acceptance:**
- Smooth shading on terrain
- Hard edges preserved where appropriate (cube corners)
- Visual: no faceted artifacts

### Phase 5: Integration & Polish (Week 3)
**Goal:** System-wide validation and optimization

#### Task 5.1: Performance Optimization
- [ ] Profile texture array sampling overhead
- [ ] Optimize material ID lookup and blending
- [ ] Implement GPU-side mip generation
- [ ] Add LOD material system (lower mip levels at distance)

**Acceptance:**
- <16ms frame time @ 1080p with 1000 draw calls
- 60 FPS stable on target hardware (GTX 1660 Ti)
- GPU utilization >80%

#### Task 5.2: Testing & Validation
- [ ] Create test suite: 50+ test meshes with edge cases
- [ ] Visual regression tests (screenshot comparison)
- [ ] Stress test: 10k instances with material variation
- [ ] Cross-platform validation (Vulkan, DX12, Metal)

**Acceptance:**
- 100% test pass rate
- Zero visual regressions from baseline
- Stable across platforms

#### Task 5.3: Documentation
- [ ] API documentation for TextureUsage, MaterialAtlas
- [ ] Tutorial: Adding new materials to engine
- [ ] Performance guide: Material limits and best practices
- [ ] Migration guide from old single-material system

---

## Technical Specifications

### Texture Array Constraints
- **Max Array Layers:** 256 (hardware minimum), 2048 (modern GPUs)
- **Texture Size:** 1024×1024 (mobile), 2048×2048 (desktop), 4096×4096 (high-end)
- **Format:** Rgba8UnormSrgb (albedo), Rgba8Unorm (normal/MRA)
- **Mip Levels:** Full chain (log2(size) + 1)
- **Compression:** BC7 (optional, for VRAM optimization)

### Material Blending
- **Blend Modes:** Height-based, slope-based, noise-based, manual (per-vertex weights)
- **Blend Quality:** 4-way interpolation (up to 4 materials per vertex)
- **Transition Distance:** Configurable (default 1.0m for smooth gradients)

### Performance Targets
- **Frame Time:** <16ms (60 FPS) @ 1080p
- **Terrain:** 50k triangles, multi-material, <5ms render time
- **Material Switches:** 1 bind group (no per-draw material overhead)
- **VRAM Budget:** <200MB for material system (albedo + normal + MRA for 256 materials)

### Asset Pipeline Requirements
- **Input Formats:** PNG (8/16/32-bit), JPEG, TGA, GLTF/GLB 2.0
- **Output:** Normalized RGBA textures, packed texture arrays, validated GLTF
- **Validation:** Format check, dimension check, UV/normal/tangent verification
- **Preprocessing:** Automatic mip generation, format conversion, atlas packing

---

## Testing Strategy

### Unit Tests
1. **Texture Format:** Verify correct format per TextureUsage
2. **Mipmap Generation:** Validate mip chain completeness and filtering
3. **Material ID Assignment:** Test height/slope/noise material selection
4. **UV Generation:** Test planar/spherical/triplanar projections
5. **Normal Smoothing:** Verify averaged normals for known meshes

### Integration Tests
1. **Material Atlas:** Pack 7 materials, verify indexing
2. **Shader Sampling:** Test texture_2d_array sampling per material_id
3. **Terrain Blending:** Validate smooth transitions at biome boundaries
4. **GLTF Loading:** Load 10+ models with missing UVs/tangents, verify fallbacks
5. **End-to-End:** Full showcase run with multi-material terrain + models

### Visual Regression Tests
1. **Baseline Screenshots:** Capture reference images for 10 camera angles
2. **Per-Change Comparison:** Automated diff against baseline (<1% pixel difference allowed)
3. **Material Accuracy:** Validate PBR appearance matches reference (albedo, roughness, metallic)

### Performance Benchmarks
1. **Frame Time:** Measure @ 720p, 1080p, 1440p, 4K
2. **Material Overhead:** Compare single-material vs multi-material terrain
3. **Texture Memory:** Track VRAM usage with 50, 100, 256 materials
4. **Load Time:** Asset loading and preprocessing time

---

## Risk Mitigation

### Risk 1: Texture Array Size Limits
- **Mitigation:** Detect hardware limits at runtime, fall back to smaller atlas
- **Fallback:** Use multiple texture arrays if >256 materials needed

### Risk 2: Performance Regression
- **Mitigation:** Profile before/after each phase, maintain performance budget
- **Fallback:** Keep single-material path as optional for low-end hardware

### Risk 3: Asset Pipeline Complexity
- **Mitigation:** Incremental rollout, validate each asset type separately
- **Fallback:** Manual preprocessing option for problematic assets

### Risk 4: Shader Compilation Errors
- **Mitigation:** Test on all target platforms early (Vulkan, DX12, Metal)
- **Fallback:** Simplified shader variant for compatibility

---

## Success Criteria

### Must Have (P0)
- ✅ Multi-material terrain with smooth biome blending
- ✅ Correct sRGB/Linear texture handling
- ✅ GLTF models load with auto-generated UVs/tangents
- ✅ Smooth per-vertex normals on terrain
- ✅ Full mipmap chains with no aliasing

### Should Have (P1)
- ✅ Texture array material system (256+ materials)
- ✅ <16ms frame time @ 1080p
- ✅ Automated asset validation pipeline
- ✅ Comprehensive test coverage (>80%)

### Nice to Have (P2)
- ⚪ BC7 texture compression
- ⚪ GPU mip generation
- ⚪ Material LOD system
- ⚪ Real-time material editing in editor

---

## Implementation Schedule

| Phase | Tasks | Duration | Dependencies |
|-------|-------|----------|--------------|
| Phase 1: Foundation | Texture format, mipmaps, validation | 3 days | None |
| Phase 2: Vertex Materials | Extended vertex, terrain assignment | 2 days | Phase 1 |
| Phase 3: Texture Arrays | Atlas generation, shader update | 3 days | Phase 2 |
| Phase 4: GLTF Robustness | UV generation, tangents, normals | 2 days | Phase 1 |
| Phase 5: Integration | Performance, testing, docs | 5 days | Phase 3, 4 |
| **Total** | | **15 days** | |

---

## Code Ownership & Files

### New Files
- `examples/unified_showcase/src/texture_manager.rs` (Phase 1)
- `examples/unified_showcase/src/material_atlas.rs` (Phase 3)
- `examples/unified_showcase/src/uv_generator.rs` (Phase 4)
- `tools/asset_validator/` (Phase 1)

### Modified Files
- `examples/unified_showcase/src/main_bevy_v2.rs` (All phases)
- `examples/unified_showcase/src/pbr_shader.wgsl` (Phase 3)
- `examples/unified_showcase/src/gltf_loader.rs` (Phase 4)

### Test Files
- `examples/unified_showcase/tests/texture_format_tests.rs`
- `examples/unified_showcase/tests/material_atlas_tests.rs`
- `examples/unified_showcase/tests/gltf_loader_tests.rs`
- `examples/unified_showcase/tests/visual_regression_tests.rs`

---

## References & Prior Art

### Industry Standards
- **Unreal Engine 5:** Virtual texture streaming, material layering
- **Unity HDRP:** Texture arrays for terrain, material blending
- **Godot 4:** MultiMesh with per-instance materials
- **wgpu Examples:** `texture-arrays`, `cube-array`, `mipmap`

### Academic References
- **Mikktspace:** Mikkelsen, M. "Simulation of Wrinkled Surfaces Revisited"
- **PBR:** Burley, B. "Physically Based Shading at Disney"
- **Texture Atlasing:** Forsyth, T. "Linear-Speed Vertex Cache Optimization"

### Crates & Dependencies
- `image` 0.24+ (texture loading, mip generation)
- `gltf` 1.4+ (GLTF/GLB parsing)
- `mikktspace` 0.3+ (tangent generation)
- `wgpu` 25.0+ (GPU API)
- `half` 2.4+ (f16 texture conversion)

---

## Appendix: Code Examples

### Example 1: TextureUsage-Based Loading
```rust
// NEW texture_manager.rs
pub enum TextureUsage {
    Albedo,
    Normal,
    MRA,
    Emissive,
}

impl TextureUsage {
    fn format(&self) -> wgpu::TextureFormat {
        match self {
            Self::Albedo | Self::Emissive => wgpu::TextureFormat::Rgba8UnormSrgb,
            Self::Normal | Self::MRA => wgpu::TextureFormat::Rgba8Unorm,
        }
    }
}

pub fn load_texture_with_usage(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    path: &str,
    usage: TextureUsage,
) -> Result<wgpu::Texture> {
    let img = image::open(path)?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    
    let mip_levels = (width.max(height) as f32).log2().floor() as u32 + 1;
    
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(path),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: mip_levels,
        format: usage.format(),
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        ..Default::default()
    });
    
    // Upload base level
    queue.write_texture(/* ... */);
    
    // Generate mipmaps (CPU-side for now)
    for level in 1..mip_levels {
        let mip_data = generate_mip_level(&rgba, level);
        queue.write_texture(/* mip_data to level */);
    }
    
    Ok(texture)
}
```

### Example 2: Material Atlas
```rust
// NEW material_atlas.rs
pub struct MaterialAtlas {
    albedo_array: wgpu::Texture,
    normal_array: wgpu::Texture,
    mra_array: wgpu::Texture,
    material_map: HashMap<String, u32>, // name -> array index
}

impl MaterialAtlas {
    pub fn builder(device: &wgpu::Device, capacity: u32) -> MaterialAtlasBuilder {
        MaterialAtlasBuilder {
            device,
            capacity,
            materials: Vec::new(),
        }
    }
    
    pub fn get_material_id(&self, name: &str) -> Option<u32> {
        self.material_map.get(name).copied()
    }
}

pub struct MaterialAtlasBuilder<'a> {
    device: &'a wgpu::Device,
    capacity: u32,
    materials: Vec<MaterialAsset>,
}

impl<'a> MaterialAtlasBuilder<'a> {
    pub fn add_material(&mut self, name: &str, albedo: &Path, normal: &Path, mra: &Path) -> Result<&mut Self> {
        self.materials.push(MaterialAsset { name: name.to_string(), albedo, normal, mra });
        Ok(self)
    }
    
    pub fn build(self, queue: &wgpu::Queue) -> Result<MaterialAtlas> {
        // Create texture arrays
        let albedo_array = self.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d { width: 1024, height: 1024, depth_or_array_layers: self.capacity },
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            dimension: wgpu::TextureDimension::D2,
            /* ... */
        });
        
        // Load and pack all materials
        for (idx, mat) in self.materials.iter().enumerate() {
            let albedo_img = image::open(&mat.albedo)?;
            // Resize to 1024x1024, generate mips, upload to layer `idx`
            queue.write_texture(/* ... */);
        }
        
        Ok(MaterialAtlas { albedo_array, normal_array, mra_array, material_map })
    }
}
```

### Example 3: Updated Shader
```wgsl
// pbr_shader.wgsl (Phase 3)
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) material_id: u32,
    @location(4) blend_weights: vec4<f32>,
}

@group(1) @binding(0) var albedo_array: texture_2d_array<f32>;
@group(1) @binding(1) var texture_sampler: sampler;
@group(1) @binding(2) var normal_array: texture_2d_array<f32>;
@group(1) @binding(3) var mra_array: texture_2d_array<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample primary material
    let albedo = textureSample(albedo_array, texture_sampler, in.uv, i32(in.material_id)).rgb;
    let normal_sample = textureSample(normal_array, texture_sampler, in.uv, i32(in.material_id)).rgb;
    let mra = textureSample(mra_array, texture_sampler, in.uv, i32(in.material_id)).rgb;
    
    // Optional: Blend with adjacent materials using blend_weights
    // (for smooth terrain transitions)
    
    let roughness = mra.g;
    let metallic = mra.b;
    
    // Rest of PBR calculation...
}
```

---

**Document End**
