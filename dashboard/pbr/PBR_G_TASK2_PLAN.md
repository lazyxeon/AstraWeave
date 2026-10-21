# Phase PBR-G Task 2: Material Inspector - IMPLEMENTATION PLAN

**Status**: 🚧 **IN PROGRESS**  
**Date**: 2025-01-XX  
**Phase**: PBR-G (Tooling, Validation, and Debug)  
**Task**: 2/6 - Material Inspector (aw_editor)

---

## Executive Summary

Extend `aw_editor` with a comprehensive Material Inspector panel that provides:

1. **Texture Map Viewer**: Display albedo, normal, ORM maps with zoom/pan
2. **Channel Isolation**: View R/G/B/A channels individually
3. **Color Space Toggle**: Switch between linear and sRGB display
4. **BRDF Response Sampling**: Visualize material appearance under different lighting
5. **Validation Integration**: Show validation results from Task 1 validators
6. **Asset Database Integration**: Browse materials from asset database

---

## Architecture Design

### Module Structure

```
tools/aw_editor/src/
├── main.rs                          (existing, integrate new panel)
├── material_inspector.rs            (NEW - inspector logic)
└── brdf_preview.rs                  (NEW - BRDF visualization)
```

### Data Structures

```rust
// material_inspector.rs

use anyhow::Result;
use egui::{Color32, ColorImage, TextureHandle, Ui};
use image::DynamicImage;
use std::path::{Path, PathBuf};

/// Material Inspector state
pub struct MaterialInspector {
    /// Currently loaded material TOML path
    pub material_path: Option<PathBuf>,
    
    /// Parsed material data (from TOML)
    pub material_data: Option<MaterialData>,
    
    /// Loaded texture images
    pub textures: MaterialTextures,
    
    /// Display settings
    pub display_mode: DisplayMode,
    pub channel_filter: ChannelFilter,
    pub color_space: ColorSpace,
    pub zoom_level: f32,
    
    /// Validation results (from Task 1 validators)
    pub validation_results: Vec<ValidationResult>,
    
    /// BRDF preview
    pub brdf_preview: BrdfPreview,
    
    /// egui texture handles (cached)
    texture_handles: TextureHandles,
}

/// Material data parsed from TOML
#[derive(Debug, Clone)]
pub struct MaterialData {
    pub name: String,
    pub albedo_path: PathBuf,
    pub normal_path: PathBuf,
    pub orm_path: PathBuf,  // ORM or MRA
    pub metallic: f32,
    pub roughness: f32,
    pub base_color: [f32; 4],
}

/// Loaded texture images
#[derive(Default)]
pub struct MaterialTextures {
    pub albedo: Option<DynamicImage>,
    pub normal: Option<DynamicImage>,
    pub orm: Option<DynamicImage>,
}

/// Display modes for texture viewer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Albedo,
    Normal,
    Orm,
    Split,  // Side-by-side comparison
}

/// Channel isolation filter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelFilter {
    All,    // RGB
    Red,
    Green,
    Blue,
    Alpha,
}

/// Color space for display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    Linear,
    Srgb,
}

/// Cached egui texture handles
#[derive(Default)]
struct TextureHandles {
    albedo: Option<TextureHandle>,
    normal: Option<TextureHandle>,
    orm: Option<TextureHandle>,
    brdf_preview: Option<TextureHandle>,
}

impl MaterialInspector {
    pub fn new() -> Self {
        Self {
            material_path: None,
            material_data: None,
            textures: MaterialTextures::default(),
            display_mode: DisplayMode::Albedo,
            channel_filter: ChannelFilter::All,
            color_space: ColorSpace::Srgb,
            zoom_level: 1.0,
            validation_results: Vec::new(),
            brdf_preview: BrdfPreview::new(),
            texture_handles: TextureHandles::default(),
        }
    }
    
    /// Load a material from TOML file
    pub fn load_material(&mut self, path: &Path) -> Result<()> {
        // TODO: Parse TOML, load textures, run validators
        Ok(())
    }
    
    /// Render the inspector UI
    pub fn show(&mut self, ui: &mut Ui) {
        // TODO: Implement UI layout
    }
}
```

### BRDF Preview Module

```rust
// brdf_preview.rs

use glam::{Vec3, Vec4};

/// BRDF preview sphere renderer
pub struct BrdfPreview {
    /// Sphere resolution (tessellation)
    pub resolution: usize,
    
    /// Light direction (normalized)
    pub light_dir: Vec3,
    
    /// View direction (normalized)
    pub view_dir: Vec3,
    
    /// Material parameters
    pub base_color: Vec4,
    pub metallic: f32,
    pub roughness: f32,
    
    /// Rendered preview image
    preview_image: Option<Vec<u8>>,
}

impl BrdfPreview {
    pub fn new() -> Self {
        Self {
            resolution: 256,
            light_dir: Vec3::new(0.0, 1.0, 0.5).normalize(),
            view_dir: Vec3::new(0.0, 0.0, 1.0),
            base_color: Vec4::new(0.8, 0.8, 0.8, 1.0),
            metallic: 0.0,
            roughness: 0.5,
            preview_image: None,
        }
    }
    
    /// Render the BRDF preview sphere
    pub fn render(&mut self) -> &[u8] {
        // TODO: Software rasterize sphere with PBR shading
        // Use Cook-Torrance BRDF from Phase PBR-D
        &[]
    }
    
    /// Update material parameters
    pub fn set_material(&mut self, base_color: Vec4, metallic: f32, roughness: f32) {
        self.base_color = base_color;
        self.metallic = metallic;
        self.roughness = roughness;
        self.preview_image = None;  // Invalidate cache
    }
}
```

---

## Implementation Tasks

### Task 2.1: Material Inspector Module (2-3 hours)

**File**: `tools/aw_editor/src/material_inspector.rs`

**Subtasks**:
1. Create `MaterialInspector` struct with state management
2. Implement `load_material()` to parse TOML and load textures
3. Integrate Task 1 validators (call `validate_material_toml()`, `validate_texture()`)
4. Create texture caching system (egui TextureHandle management)
5. Implement channel isolation (R/G/B/A filtering)
6. Add color space conversion (linear ↔ sRGB toggle)
7. Implement zoom/pan controls for texture viewer

**Dependencies**:
- `image` crate (texture loading)
- `toml` crate (TOML parsing)
- `egui` crate (UI rendering)
- `aw_asset_cli::validators` (validation integration)

**Success Criteria**:
- ✅ Can load material TOML and display textures
- ✅ Channel isolation works (red/green/blue/alpha views)
- ✅ Color space toggle functional (linear vs sRGB)
- ✅ Validation results displayed in UI

### Task 2.2: BRDF Preview Renderer (3-4 hours)

**File**: `tools/aw_editor/src/brdf_preview.rs`

**Subtasks**:
1. Implement sphere tessellation (UV sphere with configurable resolution)
2. Software rasterizer for sphere rendering
3. Integrate Cook-Torrance BRDF from Phase PBR-D
4. Add light direction controls (azimuth, elevation sliders)
5. Add view direction controls
6. Implement Lambertian fallback (for comparison)
7. Add IBL sampling (optional, if IBL assets available)

**Physics**:
- **Cook-Torrance BRDF**: `f_r = kD * f_lambert + kS * f_cook_torrance`
- **GGX Normal Distribution**: `D(h, α) = α² / (π * ((n·h)² * (α² - 1) + 1)²)`
- **Smith Geometry**: `G(v, l, α) = G1(v, α) * G1(l, α)`
- **Fresnel-Schlick**: `F(v, h) = F0 + (1 - F0) * (1 - v·h)⁵`

**Success Criteria**:
- ✅ Sphere renders with correct PBR shading
- ✅ Light direction controls work
- ✅ Material parameters (roughness, metallic) affect appearance
- ✅ Performance acceptable (< 100ms render time for 256×256)

### Task 2.3: UI Layout & Integration (1-2 hours)

**File**: `tools/aw_editor/src/main.rs`

**Subtasks**:
1. Add `MaterialInspector` field to `EditorApp`
2. Create collapsing panel in main UI
3. Add file picker for loading materials
4. Layout texture viewer (main area with controls sidebar)
5. Layout validation results panel
6. Layout BRDF preview panel
7. Add keyboard shortcuts (space = toggle channel, +/- = zoom)

**UI Layout** (egui):
```
┌─────────────────────────────────────────────────┐
│ Material Inspector                              │
├─────────────────┬───────────────────────────────┤
│ Controls        │ Texture Viewer                │
│                 │                               │
│ [Load Material] │  [Texture Display Area]       │
│                 │  - Zoom: 1.0x                 │
│ Display Mode:   │  - Pan: (0, 0)                │
│  ○ Albedo       │                               │
│  ○ Normal       │                               │
│  ○ ORM          │                               │
│  ○ Split        │                               │
│                 │                               │
│ Channel:        │                               │
│  ● All (RGB)    │                               │
│  ○ Red          │                               │
│  ○ Green        │                               │
│  ○ Blue         │                               │
│  ○ Alpha        │                               │
│                 │                               │
│ Color Space:    │                               │
│  ● sRGB         │                               │
│  ○ Linear       │                               │
├─────────────────┴───────────────────────────────┤
│ Validation Results                              │
│ ✅ Albedo: 1024×1024 sRGB, 10 mipmaps          │
│ ✅ Normal: 1024×1024 BC5, 10 mipmaps           │
│ ✅ ORM: 1024×1024 linear, 10 mipmaps           │
├─────────────────────────────────────────────────┤
│ BRDF Preview                                    │
│ [Preview Sphere Image]                          │
│ Light Dir: Az 45°, El 60°                       │
│ Material: Metallic 0.0, Roughness 0.5           │
└─────────────────────────────────────────────────┘
```

**Success Criteria**:
- ✅ UI layout is functional and intuitive
- ✅ All controls respond correctly
- ✅ Panels collapsible/expandable
- ✅ Integration with existing editor seamless

### Task 2.4: Testing & Validation (1 hour)

**Test Materials**:
- Phase PBR-F demo materials (grassland, mountain, desert)
- Custom test materials with various channel configurations
- Invalid materials (missing textures, wrong formats)

**Test Cases**:
1. Load grassland_demo.toml → verify textures display
2. Toggle channel isolation → verify R/G/B/A views correct
3. Toggle color space → verify sRGB vs linear difference visible
4. Adjust BRDF parameters → verify preview updates
5. Load invalid material → verify validation errors shown
6. Zoom/pan texture → verify smooth interaction

**Success Criteria**:
- ✅ All test materials load correctly
- ✅ Channel isolation produces expected views
- ✅ Color space toggle shows visible difference
- ✅ BRDF preview updates in real-time
- ✅ Validation errors displayed clearly

---

## Integration with Task 1 (Validators)

The Material Inspector will call validators from Task 1 to provide feedback:

```rust
use aw_asset_cli::validators::{
    validate_material_toml,
    validate_texture,
    TextureValidationConfig,
};

impl MaterialInspector {
    pub fn load_material(&mut self, path: &Path) -> Result<()> {
        // 1. Validate TOML structure
        let toml_result = validate_material_toml(path)?;
        self.validation_results.push(toml_result);
        
        // 2. Parse TOML
        let content = std::fs::read_to_string(path)?;
        let material: MaterialData = toml::from_str(&content)?;
        
        // 3. Load and validate textures
        let config = TextureValidationConfig::default();
        
        if let Ok(albedo) = image::open(&material.albedo_path) {
            self.textures.albedo = Some(albedo.clone());
            let validation = validate_texture(&material.albedo_path, &config)?;
            self.validation_results.push(validation);
        }
        
        if let Ok(normal) = image::open(&material.normal_path) {
            self.textures.normal = Some(normal.clone());
            let validation = validate_texture(&material.normal_path, &config)?;
            self.validation_results.push(validation);
        }
        
        if let Ok(orm) = image::open(&material.orm_path) {
            self.textures.orm = Some(orm.clone());
            let validation = validate_texture(&material.orm_path, &config)?;
            self.validation_results.push(validation);
        }
        
        self.material_data = Some(material);
        Ok(())
    }
}
```

---

## Dependencies & Cargo.toml Changes

```toml
# tools/aw_editor/Cargo.toml

[dependencies]
# Existing dependencies...
aw_asset_cli = { path = "../aw_asset_cli" }  # NEW - for validators

# Image processing
image = "0.24"
glam = "0.24"  # For BRDF math

# UI
egui = "0.28"
eframe = "0.28"
```

---

## Performance Considerations

### Texture Loading
- **Lazy loading**: Load textures only when panel opened
- **Caching**: Keep loaded textures in memory (invalidate on file change)
- **Thumbnail generation**: Create downscaled versions for fast preview

### BRDF Preview
- **Resolution**: Default 256×256 (adjustable for quality/speed trade-off)
- **Caching**: Render once, cache until parameters change
- **Progressive rendering**: Option to render lower-res first, then refine

**Estimated Performance**:
- Texture loading: 50-200ms per texture (depending on size)
- BRDF preview rendering: 50-100ms (256×256, software rasterizer)
- UI update: <16ms (60 FPS)

---

## Future Enhancements (Post-Task 2)

1. **GPU-accelerated BRDF preview**: Use wgpu to render sphere in real-time
2. **HDR environment map support**: Load HDR images for IBL preview
3. **Material comparison**: Side-by-side comparison of multiple materials
4. **Histogram view**: Luminance/color distribution for textures
5. **Mipmap viewer**: Display mipmap chain (level 0, 1, 2, ...)
6. **Normal map visualization**: Colored arrows for normal directions
7. **ORM channel blending**: Visualize occlusion/roughness/metallic overlay

---

## Acceptance Criteria

| Criterion | Status | Notes |
|-----------|--------|-------|
| ⏳ Material TOML loading | **TODO** | Parse TOML, load textures |
| ⏳ Texture display (albedo/normal/ORM) | **TODO** | egui texture rendering |
| ⏳ Channel isolation (R/G/B/A) | **TODO** | Pixel filtering |
| ⏳ Color space toggle (linear/sRGB) | **TODO** | Gamma correction |
| ⏳ Validation integration | **TODO** | Call Task 1 validators |
| ⏳ BRDF preview sphere | **TODO** | Software rasterizer |
| ⏳ Light direction controls | **TODO** | Azimuth/elevation sliders |
| ⏳ Material parameter controls | **TODO** | Roughness/metallic sliders |
| ⏳ Zoom/pan controls | **TODO** | Texture viewer interaction |
| ⏳ UI integration in aw_editor | **TODO** | Main editor panel |
| ⏳ Testing with demo materials | **TODO** | 3 demo materials |
| ⏳ Documentation | **TODO** | Usage guide |

**Overall Status**: 0/12 criteria met (0%)

---

## Timeline Estimate

| Task | Estimated Time | Status |
|------|---------------|--------|
| 2.1: Material Inspector Module | 2-3 hours | ⏳ TODO |
| 2.2: BRDF Preview Renderer | 3-4 hours | ⏳ TODO |
| 2.3: UI Layout & Integration | 1-2 hours | ⏳ TODO |
| 2.4: Testing & Validation | 1 hour | ⏳ TODO |
| **Total** | **7-10 hours** | **0%** |

---

## Next Actions

**Immediate (Start Task 2.1)**:
1. Create `material_inspector.rs` module
2. Implement `MaterialInspector` struct
3. Add texture loading logic
4. Integrate Task 1 validators
5. Create basic UI layout

**Then (Task 2.2)**:
6. Create `brdf_preview.rs` module
7. Implement sphere tessellation
8. Add Cook-Torrance BRDF
9. Render preview sphere

**Finally (Tasks 2.3-2.4)**:
10. Integrate into aw_editor main.rs
11. Test with demo materials
12. Document usage

---

**Status**: 📋 **PLAN COMPLETE** - Ready to begin implementation

**Next Command**: Start Task 2.1 (Material Inspector Module)
