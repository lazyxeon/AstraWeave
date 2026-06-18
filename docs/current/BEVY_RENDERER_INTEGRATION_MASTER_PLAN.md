# Bevy Renderer Integration - Master Plan (Option C)

**Date**: November 5, 2025  
**Strategy**: Hybrid approach - Bevy foundation + AstraWeave innovations  
**Timeline**: 3-5 days (Phase 1), 1-2 weeks (Phase 2 optional)  
**Status**: 🚀 **FULL DIVE - NO TIPTOEING**

---

## Executive Summary

After 13+ hours on custom renderer with diminishing returns, we're pivoting to **Option C: Bevy Foundation + Custom Extensions**.

**Why This Works**:
- ✅ **Bevy renderer** = 5+ years battle-tested, professional quality
- ✅ **AstraWeave innovations** = AI-native features (unique value)
- ✅ **Time efficiency** = 3-5 days vs 2-4 weeks of debugging
- ✅ **Professional grade** = User's "mission critical standards" directive

**What We're NOT Doing**:
- ❌ Abandoning custom work (MegaLights preserved for Phase 2)
- ❌ Full Bevy ECS coupling (adapter layer keeps independence)
- ❌ Compromising on quality (battle-tested > custom debugging)

---

## Phase 1: Foundation (3-5 Days) - CRITICAL PATH

### Day 1: Extract Bevy PBR Core (6-8 hours)

**Objective**: Fork `bevy_pbr` crate as `astraweave-render-bevy`

**Source Files** (from `bevy/crates/bevy_pbr/src/`):
```
bevy_pbr/
├── lib.rs                      # Main exports (400 lines)
├── render/
│   ├── mod.rs                  # Render graph setup (600 lines)
│   ├── light.rs                # Directional/point/spot lights (1,200 lines)
│   ├── mesh.rs                 # Mesh pipeline (2,000 lines)
│   ├── pbr_functions.wgsl      # WGSL shader functions (800 lines)
│   ├── pbr_fragment.wgsl       # Fragment shader (400 lines)
│   ├── shadows.wgsl            # Shadow sampling (300 lines) ✅ Already extracted
│   ├── shadow_sampling.wgsl    # PCF implementation (200 lines) ✅ Already extracted
│   └── mesh_view_bindings.wgsl # View/projection uniforms (150 lines)
├── material.rs                 # Material trait + standard material (1,500 lines)
├── bundle.rs                   # PbrBundle, MaterialBundle (200 lines)
└── prepass/                    # Depth/normal prepass (optional, 800 lines)

Total: ~8,000-10,000 lines core functionality
```

**Extraction Strategy**:
1. Create `crates/astraweave-render-bevy/` directory structure
2. Copy core files (lib.rs, render/, material.rs)
3. **Remove Bevy ECS dependencies**: Replace with trait abstractions
4. Keep WGSL shaders intact (proven quality)
5. Simplify: Remove features we don't need yet (prepass, wireframe, fog)

**Deliverables**:
- ✅ `astraweave-render-bevy` crate compiles standalone
- ✅ WGSL shaders validated with `naga`
- ✅ Dependency tree: Only wgpu, glam, encase (no bevy_ecs)

**Acceptance Criteria**:
```bash
cargo build -p astraweave-render-bevy
# Expected: Compiles successfully, ~8k lines, 0 errors
```

---

### Day 2: Build ECS Adapter Layer (6-8 hours)

**Objective**: Bridge AstraWeave ECS ↔ Bevy render data structures

**Adapter Architecture**:
```rust
// astraweave-render-bevy/src/adapter.rs (NEW - 500+ lines)

use astraweave_ecs::{World, Query, Component};
use crate::render::{MeshPipeline, MaterialPipeline, LightData};

/// Extracts render data from AstraWeave ECS into Bevy-compatible structures
pub struct RenderAdapter {
    // Mesh extraction
    mesh_registry: HashMap<EntityId, MeshHandle>,
    
    // Material extraction
    material_registry: HashMap<EntityId, MaterialHandle>,
    
    // Light extraction
    directional_lights: Vec<DirectionalLight>,
    point_lights: Vec<PointLight>,
    spot_lights: Vec<SpotLight>,
    
    // Shadow cascades
    csm_data: CascadeShadowMapData,
}

impl RenderAdapter {
    /// Extract meshes from AstraWeave World
    pub fn extract_meshes(&mut self, world: &World) {
        // Query AstraWeave ECS for (Transform, MeshComponent)
        for (entity, (transform, mesh)) in world.query::<(&Transform, &MeshComponent)>() {
            // Convert to Bevy's render format
            let mesh_handle = self.convert_mesh(mesh);
            self.mesh_registry.insert(entity, mesh_handle);
        }
    }
    
    /// Extract materials from AstraWeave World
    pub fn extract_materials(&mut self, world: &World) {
        // Query for (MaterialComponent)
        for (entity, material) in world.query::<&MaterialComponent>() {
            // Convert to Bevy StandardMaterial
            let bevy_material = StandardMaterial {
                base_color: material.albedo_color,
                metallic: material.metallic,
                roughness: material.roughness,
                // ... etc
            };
            self.material_registry.insert(entity, bevy_material);
        }
    }
    
    /// Extract lights (directional, point, spot)
    pub fn extract_lights(&mut self, world: &World) {
        // Directional lights
        for (_, light) in world.query::<&DirectionalLightComponent>() {
            self.directional_lights.push(DirectionalLight {
                direction: light.direction,
                color: light.color,
                illuminance: light.intensity,
                shadows_enabled: light.cast_shadows,
            });
        }
        
        // Point lights
        for (_, (transform, light)) in world.query::<(&Transform, &PointLightComponent)>() {
            self.point_lights.push(PointLight {
                position: transform.translation,
                color: light.color,
                intensity: light.intensity,
                range: light.range,
                radius: light.radius,
                shadows_enabled: light.cast_shadows,
            });
        }
    }
    
    /// Submit extracted data to Bevy render pipeline
    pub fn submit_render_data(&self, render_context: &mut RenderContext) {
        // Build render batches
        let mesh_batches = self.build_mesh_batches();
        
        // Submit to GPU
        render_context.submit_meshes(mesh_batches);
        render_context.submit_lights(&self.directional_lights, &self.point_lights);
        render_context.submit_shadows(&self.csm_data);
    }
}
```

**Component Mapping** (AstraWeave → Bevy):
```rust
// AstraWeave Components
pub struct Transform { translation: Vec3, rotation: Quat, scale: Vec3 }
pub struct MeshComponent { vertices: Vec<Vertex>, indices: Vec<u32> }
pub struct MaterialComponent { albedo: TextureHandle, normal: TextureHandle, mra: TextureHandle }
pub struct DirectionalLightComponent { direction: Vec3, color: Vec3, intensity: f32 }
pub struct PointLightComponent { color: Vec3, intensity: f32, range: f32 }

// Bevy Equivalents (from bevy_pbr)
pub struct GlobalTransform(Mat4);  // Computed from Transform hierarchy
pub struct Mesh { /* Vertex buffers */ }
pub struct Handle<StandardMaterial> { /* Material handle */ }
pub struct DirectionalLight { direction: Vec3, color: Color, illuminance: f32 }
pub struct PointLight { position: Vec3, color: Color, intensity: f32, range: f32 }
```

**Deliverables**:
- ✅ `RenderAdapter` extracts data from AstraWeave ECS
- ✅ Component conversion working (Transform, Mesh, Material, Light)
- ✅ No tight coupling (adapter is the ONLY bridge)

**Acceptance Criteria**:
```bash
cargo test -p astraweave-render-bevy --test adapter_tests
# Expected: 10+ tests passing (extraction, conversion, submission)
```

---

### Day 3: Integrate CSM + Materials (8-10 hours)

**Objective**: Wire Bevy's proven CSM pipeline and PBR materials

**CSM Integration** (Replace `shadow_csm.rs`):
```rust
// Use Bevy's CSM implementation (bevy_pbr/src/render/light.rs)
// Already proven to work - just adapt uniforms

use bevy_render_bevy::render::light::{
    setup_directional_light_cascades,
    DirectionalLightShadowMap,
    CascadesVisibleEntities,
};

// In astraweave-render/src/renderer.rs
impl Renderer {
    pub fn setup_shadows(&mut self, lights: &[DirectionalLight]) {
        // Use Bevy's cascade calculation (already debugged!)
        let cascade_config = CascadeShadowConfig {
            num_cascades: 4,
            minimum_distance: 0.1,
            maximum_distance: 100.0,
            overlap_proportion: 0.2,
            first_cascade_far_bound: 4.0,
        };
        
        // Bevy calculates splits using camera frustum (CORRECT approach)
        let cascades = setup_directional_light_cascades(
            &cascade_config,
            &self.camera_view,
            &self.camera_projection,
        );
        
        // Render shadow maps using Bevy's proven pipeline
        for (i, cascade) in cascades.iter().enumerate() {
            self.render_shadow_cascade(i, cascade, &lights[0]);
        }
    }
}
```

**Material System** (Use Bevy's StandardMaterial):
```rust
// Bevy's material.rs already handles texture arrays, PBR math, etc.
use bevy_render_bevy::material::StandardMaterial;

pub struct MaterialManager {
    materials: HashMap<String, Handle<StandardMaterial>>,
}

impl MaterialManager {
    pub fn load_from_toml(&mut self, path: &Path) -> Result<()> {
        // Read AstraWeave materials.toml
        let config: MaterialConfig = toml::from_str(&fs::read_to_string(path)?)?;
        
        // Convert to Bevy StandardMaterial
        for (name, mat) in config.materials {
            let bevy_mat = StandardMaterial {
                base_color_texture: Some(self.load_texture(&mat.albedo)?),
                normal_map_texture: Some(self.load_texture(&mat.normal)?),
                metallic_roughness_texture: Some(self.load_texture(&mat.mra)?),
                metallic: mat.metallic,
                perceptual_roughness: mat.roughness,
                ..Default::default()
            };
            
            self.materials.insert(name, self.add_material(bevy_mat));
        }
        Ok(())
    }
}
```

**Shader Integration** (Use existing Bevy shaders):
```wgsl
// bevy_pbr/src/render/pbr_fragment.wgsl (PROVEN)
// Already handles:
// - PBR lighting (Cook-Torrance BRDF)
// - Normal mapping
// - Metallic/roughness workflow
// - Shadow sampling (our extracted shadows.wgsl)
// - IBL (diffuse + specular)

// Just wire up bindings to match AstraWeave uniform layout
@group(0) @binding(0) var<uniform> view: View;
@group(1) @binding(0) var base_color_texture: texture_2d<f32>;
@group(1) @binding(1) var base_color_sampler: sampler;
@group(1) @binding(2) var normal_map_texture: texture_2d<f32>;
// ... etc (use Bevy's proven binding layout)
```

**Deliverables**:
- ✅ CSM shadows working (using Bevy's cascade calculation)
- ✅ PBR materials loaded from TOML (albedo, normal, MRA)
- ✅ First successful render with shadows + materials

**Acceptance Criteria**:
```bash
cargo run -p hello_companion --release --features bevy-renderer
# Expected: 
# - 3D scene renders with PBR materials
# - Shadows visible and correct
# - No debug mode failures (using proven Bevy shaders)
```

---

### Day 4: Lighting + Post-Processing (6-8 hours)

**Objective**: Complete PBR pipeline with all lighting types and post-FX

**Point/Spot Lights** (Bevy's implementation):
```rust
// bevy_pbr/src/render/light.rs already has:
// - Point light attenuation (physically-based inverse square)
// - Spot light cone calculation
// - Shadow mapping for omnidirectional (cubemap) and spot (2D)

use bevy_render_bevy::render::light::{PointLight, SpotLight, PointLightShadowMap};

impl Renderer {
    pub fn render_lights(&mut self, adapter: &RenderAdapter) {
        // Extract lights from AstraWeave ECS
        let point_lights = adapter.point_lights();
        let spot_lights = adapter.spot_lights();
        
        // Use Bevy's lighting shader (pbr_functions.wgsl)
        // Already handles:
        // - Multiple lights accumulation
        // - Shadow PCF sampling
        // - Light radius soft shadows
        
        self.submit_point_lights(&point_lights);
        self.submit_spot_lights(&spot_lights);
    }
}
```

**IBL (Image-Based Lighting)**:
```rust
// Bevy's IBL uses:
// - Diffuse irradiance map (prefiltered cubemap)
// - Specular prefiltered environment map (mipmapped cubemap)
// - BRDF LUT (2D texture for split-sum approximation)

// We already have IblManager in astraweave-render!
// Just wire it to Bevy's shader bindings

@group(2) @binding(0) var environment_map_diffuse: texture_cube<f32>;
@group(2) @binding(1) var environment_map_specular: texture_cube<f32>;
@group(2) @binding(2) var brdf_lut: texture_2d<f32>;

// Bevy's pbr_functions.wgsl::environment_light() handles the math
```

**Post-Processing** (Bloom + Tonemapping):
```rust
// Bevy's tonemapping.wgsl supports:
// - Reinhard
// - ACES (film-like)
// - AgX (neutral)
// - Tony McMapface (modern)

use bevy_render_bevy::render::tonemapping::{Tonemapping, TonemappingPipeline};

impl Renderer {
    pub fn apply_post_processing(&mut self, hdr_texture: &TextureView) {
        // 1. Bloom (Bevy's bloom.wgsl)
        let bloomed = self.bloom_pipeline.apply(hdr_texture);
        
        // 2. Tonemapping (Bevy's tonemapping.wgsl)
        let final_color = self.tonemapping_pipeline.apply(
            bloomed,
            Tonemapping::AcesFitted, // Default
        );
        
        // 3. Output to swapchain
        self.copy_to_swapchain(final_color);
    }
}
```

**Deliverables**:
- ✅ Point lights working (8+ simultaneous, correct attenuation)
- ✅ Spot lights working (cone angle, shadows)
- ✅ IBL integrated (diffuse + specular contribution)
- ✅ Post-processing (bloom, tonemapping, SSAO optional)

**Acceptance Criteria**:
```bash
cargo run -p unified_showcase --release --features bevy-renderer
# Expected:
# - Multiple point/spot lights visible
# - Bloom on bright surfaces
# - Tonemapping (no blown-out whites)
# - 60+ FPS at 1080p
```

---

### Day 5: Validation + Documentation (4-6 hours)

**Objective**: Prove production-readiness and document

**Validation Tests**:
1. **hello_companion** - Basic AI + rendering integration
2. **unified_showcase** - All features (materials, lights, shadows)
3. **Performance benchmark** - 60 FPS @ 1080p with 100+ entities
4. **Visual comparison** - Side-by-side with Bevy demo (should match quality)

**Performance Benchmarks**:
```bash
# Expected targets (based on Bevy's proven performance)
cargo bench -p astraweave-render-bevy --bench rendering_pipeline

# Targets:
# - Frame time: <16.67 ms (60 FPS) @ 1080p
# - Shadow rendering: <4 ms for 4 cascades
# - Material batching: <2 ms for 100+ unique materials
# - Point lights: <0.5 ms per 10 lights
# - Post-processing: <3 ms (bloom + tonemap)
```

**API Documentation**:
```bash
# Generate comprehensive docs
cargo doc -p astraweave-render-bevy --no-deps --open

# Write integration guide (300+ lines)
docs/current/BEVY_RENDERER_INTEGRATION_GUIDE.md
```

**Completion Report**:
```markdown
# BEVY_RENDERER_PHASE_1_COMPLETE.md (500+ lines)

## Summary
- ✅ 3-5 day timeline: ACHIEVED (Day 5 of 5)
- ✅ Professional quality: Bevy-proven rendering
- ✅ CSM shadows: Working (no debug mode issues!)
- ✅ PBR materials: Full workflow (albedo, normal, MRA)
- ✅ Lighting: Directional, point, spot + IBL
- ✅ Post-FX: Bloom, tonemapping, SSAO
- ✅ Performance: 60+ FPS validated

## Metrics
- Lines extracted from Bevy: ~10,000
- Adapter layer code: ~1,500 lines
- Tests written: 25+ (all passing)
- Examples updated: 3 (hello_companion, unified_showcase, shadow_csm_demo)
- Documentation: 1,200+ lines

## Next Steps
- Phase 2 (Optional): Port MegaLights as extension (1-2 weeks)
- Integration: Wire into main Renderer (1-2 days)
- Optimization: AI-specific batching for 12,700 agents (3-5 days)
```

**Deliverables**:
- ✅ All validation tests passing
- ✅ Performance targets met (60 FPS)
- ✅ API documentation complete
- ✅ Integration guide written
- ✅ Completion report published

**Acceptance Criteria**:
```bash
# All examples compile and run
cargo run -p hello_companion --release --features bevy-renderer
cargo run -p unified_showcase --release --features bevy-renderer

# Benchmarks pass thresholds
cargo bench -p astraweave-render-bevy

# Documentation builds
cargo doc -p astraweave-render-bevy --open

# Declare PHASE 1 COMPLETE ✅
```

---

## Phase 2: Optimization + Extensions (1-2 Weeks) - OPTIONAL

**DEFER THIS UNTIL PHASE 1 PROVEN**

### Week 1: Port MegaLights

**Objective**: Integrate MegaLights as extension to Bevy renderer

**Approach**:
1. Extract MegaLights from `astraweave-render/src/megalights.rs` (1,620 lines)
2. Create `bevy_render_bevy::extensions::megalights` module
3. Hook into Bevy's lighting pipeline (pre-lighting pass)
4. Benchmark: Compare MegaLights vs standard Bevy lights

**Success Criteria**:
- ✅ MegaLights 100k+ lights validated
- ✅ Performance: <5 ms for 100k lights (vs Bevy's ~50 ms for 1k)
- ✅ Visual quality: Matches Bevy's standard lighting

### Week 2: AI-Specific Optimizations

**Objective**: Optimize for 12,700 agent rendering

**Optimizations**:
1. **GPU Instancing**: Batch identical agents (already in Bevy)
2. **LOD System**: Distance-based mesh simplification
3. **Frustum Culling**: Spatial hash for visibility checks
4. **Async Mesh Loading**: Stream agent meshes from disk

**Success Criteria**:
- ✅ 12,700 agents @ 60 FPS (validated AI-native capacity)
- ✅ Memory: <4 GB VRAM (instancing reduces overhead)
- ✅ Load time: <2 seconds for 10k agents

---

## Technical Details

### Bevy Version Compatibility

**Target**: `bevy 0.14.0` (latest stable as of Nov 2025)

**Dependencies** (add to workspace Cargo.toml):
```toml
[workspace.dependencies]
bevy_pbr = { version = "0.14", default-features = false, features = ["wgsl"] }
bevy_render = { version = "0.14", default-features = false }
bevy_asset = { version = "0.14", default-features = false }
bevy_math = { version = "0.14" }

# Our dependencies remain
wgpu = "25.0"
glam = "0.29"
encase = "0.10"
```

### Crate Structure

```
crates/astraweave-render-bevy/
├── Cargo.toml              # Dependencies (bevy_pbr, wgpu, glam)
├── src/
│   ├── lib.rs              # Public API exports
│   ├── adapter.rs          # ECS bridge (AstraWeave ↔ Bevy)
│   ├── render/
│   │   ├── mod.rs          # Render graph setup
│   │   ├── mesh.rs         # Mesh pipeline (from Bevy)
│   │   ├── material.rs     # StandardMaterial (from Bevy)
│   │   ├── light.rs        # Lighting (directional, point, spot)
│   │   ├── shadow.rs       # CSM + shadow mapping
│   │   └── postfx.rs       # Bloom, tonemapping
│   ├── shaders/
│   │   ├── pbr_fragment.wgsl      # From Bevy
│   │   ├── pbr_functions.wgsl     # From Bevy
│   │   ├── shadows.wgsl           # Already extracted ✅
│   │   ├── shadow_sampling.wgsl   # Already extracted ✅
│   │   └── tonemapping.wgsl       # From Bevy
│   └── extensions/
│       └── megalights.rs   # Phase 2 (optional)
├── tests/
│   ├── adapter_tests.rs    # ECS extraction tests
│   ├── render_tests.rs     # Pipeline validation
│   └── integration_tests.rs # End-to-end
└── examples/
    └── bevy_renderer_demo.rs # Standalone demo
```

### Feature Flags

```toml
[features]
default = ["csm", "materials", "post-processing"]
csm = []              # Cascade shadow maps
materials = []        # PBR materials (albedo, normal, MRA)
post-processing = []  # Bloom, tonemapping, SSAO
megalights = []       # Phase 2: 100k+ lights
ibl = []              # Image-based lighting
point-shadows = []    # Omnidirectional shadow maps
```

### Migration Path (Examples)

**Before** (custom renderer):
```rust
use astraweave_render::{Renderer, MaterialManager};

let mut renderer = Renderer::new(&device, &queue, &config)?;
renderer.render(&world, &camera)?;
```

**After** (Bevy renderer):
```rust
use astraweave_render_bevy::{BevyRenderer, RenderAdapter};

let mut adapter = RenderAdapter::new();
let mut renderer = BevyRenderer::new(&device, &queue, &config)?;

// Extract data from AstraWeave ECS
adapter.extract_meshes(&world);
adapter.extract_materials(&world);
adapter.extract_lights(&world);

// Render using Bevy pipeline
renderer.render(&adapter, &camera)?;
```

**Compatibility**: Both renderers coexist with feature flags
```toml
# Cargo.toml
astraweave-render = { path = "../astraweave-render", optional = true }
astraweave-render-bevy = { path = "../astraweave-render-bevy", optional = true }

[features]
custom-renderer = ["astraweave-render"]
bevy-renderer = ["astraweave-render-bevy"]
```

---

## Risk Mitigation

### Risk 1: Bevy ECS Coupling
**Mitigation**: Adapter layer is the ONLY bridge. AstraWeave ECS remains independent.

### Risk 2: Bevy Version Churn
**Mitigation**: Fork specific version (0.14.0), control upgrades ourselves.

### Risk 3: MegaLights Integration Complexity
**Mitigation**: Phase 2 is OPTIONAL. Ship without it if needed.

### Risk 4: Performance Regression
**Mitigation**: Benchmark suite validates 60 FPS before declaring Phase 1 complete.

---

## Success Metrics

### Phase 1 (3-5 Days)
- ✅ CSM shadows working (no debug mode issues)
- ✅ PBR materials (albedo, normal, MRA texture arrays)
- ✅ Lighting (directional, point, spot, IBL)
- ✅ Post-processing (bloom, tonemapping)
- ✅ Performance: 60+ FPS @ 1080p, 100+ entities
- ✅ Examples: hello_companion, unified_showcase working

### Phase 2 (1-2 Weeks, Optional)
- ✅ MegaLights integrated (100k+ lights)
- ✅ AI optimizations (12,700 agents @ 60 FPS)
- ✅ LOD system (distance-based mesh simplification)
- ✅ Async loading (streaming mesh data)

---

## Timeline

**Day 1 (Today)**: Extract Bevy PBR core → `astraweave-render-bevy` compiles  
**Day 2**: Build ECS adapter → Component conversion working  
**Day 3**: CSM + Materials → First successful render with shadows  
**Day 4**: Lighting + Post-FX → Full PBR pipeline operational  
**Day 5**: Validation + Docs → PHASE 1 COMPLETE ✅  

**Week 2-3** (Optional): MegaLights + AI optimizations  

---

## Conclusion

**This is the RIGHT MOVE**:
- ✅ Proven quality (Bevy's 5+ years of work)
- ✅ Time efficient (3-5 days vs 2-4 weeks debugging)
- ✅ Professional grade (user's directive)
- ✅ Focus on AI features (AstraWeave's unique value)

**No tiptoeing. Full dive. Let's build this.** 🚀

---

**Next Action**: Execute Day 1 - Extract Bevy PBR Core

```bash
# Create crate structure
mkdir -p crates/astraweave-render-bevy/src/render
mkdir -p crates/astraweave-render-bevy/src/shaders

# Clone Bevy repository (reference only, we'll extract files)
cd /tmp
git clone --depth 1 --branch v0.14.0 https://github.com/bevyengine/bevy.git

# Extract files (next prompt)
```

**STATUS**: 🚀 **READY TO EXECUTE - AWAITING GO SIGNAL**
