# WGPU Material Rendering Research Report
## Common Pitfalls and Debugging Guide for unified_showcase

### Executive Summary
Research into WGPU material binding patterns reveals that the unified_showcase implementation follows mostly correct patterns, but the mono-texture rendering bug likely stems from **material indexing or bind group state persistence issues**. All objects rendering with the same texture is a classic symptom of materials not being properly switched during the render loop.

---

## 1. Material Binding Patterns - Best Practices

### 1.1 Core Principle: Frequency-Based Grouping
**Source:** [WebGPU Bind Group Best Practices (toji.dev)](https://toji.dev/webgpu-best-practices/bind-groups.html)

Resources should be grouped by **change frequency**:
```rust
// CORRECT PATTERN
@group(0) @binding(0) var<uniform> camera;     // Changes per-frame
@group(1) @binding(0) var baseColor: texture;   // Changes per-material
@group(2) @binding(0) var<uniform> model;      // Changes per-object
```

**unified_showcase implementation:**
```rust
// Current shader_v2.wgsl
@group(0) @binding(0) var<uniform> camera;      // ✓ Per-frame
@group(1) @binding(0) var<uniform> light;       // ✓ Per-frame
@group(2) @binding(0) var t_diffuse: texture;   // ✓ Per-material
@group(3) @binding(0) var<uniform> model;       // ✓ Per-object
```

**Status:** ✅ **CORRECT** - Bind group layout follows industry best practices.

### 1.2 Bind Group Persistence vs Recreation
**Key Finding:** Bind groups should be created once at load time and reused.

**Current Implementation (main.rs:96-100):**
```rust
struct Material {
    #[allow(dead_code)]
    name: String,
    bind_group: wgpu::BindGroup,  // ✓ Stored, not recreated
}
```

**Status:** ✅ **CORRECT** - Materials store bind groups, avoiding recreation overhead.

---

## 2. Texture Sampling Issues - Common Causes

### 2.1 UV Coordinate Problems
**Sources:** 
- [WebGPU Textures Fundamentals](https://webgpufundamentals.org/webgpu/lessons/webgpu-textures.html)
- [Learn WGPU Textures Tutorial](https://sotrh.github.io/learn-wgpu/beginner/tutorial5-textures/)

**Common Mistakes:**
1. **Y-axis Flip:** WGPU texture coordinates have (0,0) at top-left, but 3D models may expect bottom-left
2. **UV Out of Range:** Coordinates outside [0,1] with incorrect address modes
3. **Incorrect Vertex Attribute Mapping:** UV data not properly bound

**unified_showcase Vertex Structure (main.rs:44-52):**
```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],        // ✓ UV present
    color: [f32; 4],
    tangent: [f32; 4],
}
```

**Shader UV Usage (shader_v2.wgsl:81):**
```wgsl
let object_color = textureSample(t_diffuse, s_diffuse, in.uv) * in.color;
```

**Potential Issue:** UV coordinates passed through but not validated. If GLTF loader doesn't populate UVs correctly, all objects sample the same texture coordinate.

### 2.2 Texture View Format Mismatches
**Common Issue:** Creating texture views with incompatible formats.

**Current Implementation:** Uses default view descriptors - likely correct but should verify GLTF texture formats match expected `Rgba8UnormSrgb`.

### 2.3 Sampler Configuration
**Best Practice:** Samplers should have appropriate address modes and filtering.

**Current Implementation (gltf_loader.rs - needs verification):**
```rust
// Material bind group includes sampler at binding 1
// Layout defined at main.rs:299-301
wgpu::BindGroupLayoutEntry {
    binding: 1,
    visibility: wgpu::ShaderStages::FRAGMENT,
    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
    count: None,
}
```

**Status:** ✅ Layout correct, but sampler creation in GLTF loader needs verification.

---

## 3. Material Index Systems - Architecture Analysis

### 3.1 Current Architecture
```rust
struct Mesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    material_index: usize,  // ← Links mesh to material
}

struct SceneObject {
    mesh_index: usize,  // ← Links object to mesh
    // ... transform data ...
}
```

### 3.2 Render Loop Material Binding (main.rs:1797-1805)
```rust
for obj in &self.objects {
    let mesh = &self.meshes[obj.mesh_index];
    let material = &self.materials[mesh.material_index];  // ← Material lookup
    render_pass.set_bind_group(2, &material.bind_group, &[]);
    render_pass.set_bind_group(3, &obj.model_bind_group, &[]);
    // ... draw calls ...
}
```

**CRITICAL FINDING:** The render loop **does** set the material bind group per-object. The bug must be in:
1. **Material creation** - all materials might be sharing the same texture
2. **Material index assignment** - all meshes might point to the same material
3. **GLTF loader** - not creating separate materials per-primitive

---

## 4. Common Bugs Causing Mono-Texture Rendering

### 🔴 BUG #1: All Materials Share Same Texture Reference
**Symptom:** Every object renders with the same texture despite having different materials.

**Root Cause:** Material bind groups created with references to the same texture view.

**Debugging Steps:**
```rust
// Add to material creation code:
println!("Creating material {} with texture at {:p}", name, texture);

// Add to render loop:
println!("Object {} using material {} (index {})", 
    obj_idx, self.materials[mesh.material_index].name, mesh.material_index);
```

**Fix Pattern:**
```rust
// WRONG - reuses same texture for all materials
let shared_texture = create_texture(...);
for primitive in primitives {
    create_material(shared_texture);  // ❌ All materials use same texture
}

// CORRECT - each material gets its own texture
for primitive in primitives {
    let texture = match primitive.material {
        Some(mat) => load_texture_for_material(mat),
        None => default_texture.clone(),
    };
    create_material(texture);  // ✓ Unique textures per material
}
```

### 🔴 BUG #2: Material Index Not Updated Per-Primitive
**Symptom:** All meshes have `material_index = 0`.

**Root Cause:** GLTF loader assigns default material index instead of per-primitive material.

**GLTF Structure:**
```json
{
  "meshes": [{
    "primitives": [
      { "material": 0 },  // ← Pine bark
      { "material": 1 }   // ← Pine leaves
    ]
  }]
}
```

**Current Code Pattern (needs verification in gltf_loader.rs):**
```rust
// WRONG - ignores primitive.material
for primitive in mesh.primitives {
    meshes.push(Mesh {
        material_index: 0,  // ❌ Always defaults to 0
        // ...
    });
}

// CORRECT - uses primitive-specific material
for primitive in mesh.primitives {
    meshes.push(Mesh {
        material_index: primitive.material.unwrap_or(0),  // ✓ Per-primitive
        // ...
    });
}
```

### 🔴 BUG #3: Bind Group State Not Cleared Between Pipelines
**Symptom:** Material bind group from previous pipeline "bleeds" into current render.

**Root Cause:** State persistence across pipeline changes.

**Current Code (main.rs:1793-1795):**
```rust
render_pass.set_pipeline(&self.render_pipeline);
render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
render_pass.set_bind_group(1, &self.light_bind_group, &[]);
// ⚠️ Group 2 (material) not explicitly set before loop
```

**Fix:** Always set ALL bind groups after `set_pipeline()`.

### 🔴 BUG #4: Fallback Texture Used for All Materials
**Symptom:** Every material shows the same default/white texture.

**Root Cause:** Texture loading fails silently, all materials fall back to same default.

**Detection:**
```rust
// Check for repeated texture addresses
let mut texture_set = HashSet::new();
for material in &self.materials {
    // Get texture pointer from bind group somehow
    if !texture_set.insert(texture_ptr) {
        println!("WARNING: Duplicate texture detected!");
    }
}
```

---

## 5. GLTF-Specific Material Patterns

### 5.1 Efficient GLTF Rendering Pattern
**Source:** [Efficiently Rendering glTF Models (toji.dev)](https://toji.dev/webgpu-gltf-case-study/)

**Best Practice Hierarchy:**
```
For each Pipeline:
  Set Pipeline
  For each Material using Pipeline:
    Set Material Bind Group (group 2)
    For each Primitive using Material:
      Set Model Bind Group (group 3)
      Draw Primitive
```

**Current unified_showcase Pattern:**
```rust
// Current: Sets pipeline once, then loops objects
set_pipeline(&self.render_pipeline);
for obj in &self.objects {
    set_bind_group(2, material);  // ← Could be optimized
    set_bind_group(3, model);
    draw();
}
```

**Optimization Opportunity:**
```rust
// Optimized: Group by material to reduce bind group changes
for (material_idx, material) in &self.materials {
    set_bind_group(2, &material.bind_group);
    for obj in objects_using_material(material_idx) {
        set_bind_group(3, &obj.model_bind_group);
        draw();
    }
}
```

### 5.2 Per-Primitive vs Per-Mesh Materials
**Critical Concept:** In GLTF, materials are assigned **per-primitive**, not per-mesh.

**Example:**
```
Pine Tree Mesh:
  - Primitive 0: Trunk geometry → bark.png (material 0)
  - Primitive 1: Foliage geometry → leaves.png (material 1)
```

**Implication:** A single GLTF "mesh" can become multiple render `Mesh` structs in the engine.

**Current Implementation:** Needs verification that GLTF loader creates separate `Mesh` instances per primitive.

---

## 6. Debugging Checklist for unified_showcase

### Phase 1: Validate Material Creation
```rust
// In gltf_loader.rs or wherever materials are created:

// ✅ Check: Each material has unique texture
for (i, material) in materials.iter().enumerate() {
    println!("Material {}: name={}, bind_group={:?}", 
        i, material.name, material.bind_group);
}

// ✅ Check: Texture loading succeeded
println!("Loaded {} textures", texture_count);
for (name, texture) in &textures {
    println!("  {}: {:?}", name, texture);
}

// ✅ Check: Material count matches GLTF
assert_eq!(materials.len(), gltf_materials.len());
```

### Phase 2: Validate Mesh-Material Links
```rust
// In mesh creation:
for (i, mesh) in meshes.iter().enumerate() {
    println!("Mesh {}: material_index={}, indices={}", 
        i, mesh.material_index, mesh.num_indices);
}

// ✅ Check: Material indices are diverse
let material_indices: HashSet<_> = meshes.iter()
    .map(|m| m.material_index)
    .collect();
println!("Unique materials in use: {}/{}", 
    material_indices.len(), materials.len());
```

### Phase 3: Validate Render Loop
```rust
// In render() function:
let mut last_material_idx = usize::MAX;
for (obj_idx, obj) in self.objects.iter().enumerate() {
    let mesh = &self.meshes[obj.mesh_index];
    let mat_idx = mesh.material_index;
    
    if mat_idx != last_material_idx {
        println!("Object {} switching to material {}", obj_idx, mat_idx);
        last_material_idx = mat_idx;
    }
}
```

### Phase 4: Verify Bind Group Contents
```rust
// Cannot directly inspect bind group contents, but can verify creation:
println!("Creating material bind group:");
println!("  Texture: {:?}", texture_view);
println!("  Sampler: {:?}", sampler);
```

---

## 7. Specific Recommendations for unified_showcase

### 7.1 IMMEDIATE CHECKS (Priority 1)
1. **Print material indices during object creation**
   - Verify meshes have different `material_index` values
   - Expected: pine_bark (0), pine_leaves (1), tower_wood (2), tower_stone (3)

2. **Validate material count**
   - Check `self.materials.len()` matches expected count
   - Should be at least 4 materials for the scene

3. **Inspect bind group set calls**
   - Add debug output in render loop showing which material is bound per object

### 7.2 CODE ADDITIONS (Priority 2)
```rust
// Add to ShowcaseApp::new()
impl ShowcaseApp {
    async fn new(window: Arc<Window>) -> Self {
        // ... existing code ...
        
        // DEBUG: Print material summary
        println!("\n=== Material System Diagnostic ===");
        println!("Total materials: {}", materials.len());
        for (i, mat) in materials.iter().enumerate() {
            println!("  [{}] {}", i, mat.name);
        }
        
        println!("\nTotal meshes: {}", meshes.len());
        for (i, mesh) in meshes.iter().enumerate() {
            println!("  Mesh {}: material_index={}", i, mesh.material_index);
        }
        
        println!("\nMaterial usage:");
        let mut mat_usage = std::collections::HashMap::new();
        for mesh in &meshes {
            *mat_usage.entry(mesh.material_index).or_insert(0) += 1;
        }
        for (mat_idx, count) in mat_usage {
            println!("  Material {}: used by {} meshes", mat_idx, count);
        }
        println!("=================================\n");
        
        // ... rest of code ...
    }
}
```

### 7.3 GLTF LOADER VERIFICATION (Priority 1)
The most likely bug location is in `gltf_loader.rs`. Verify:

```rust
// Check that materials are loaded per-primitive:
for mesh in gltf.meshes() {
    for primitive in mesh.primitives() {
        let material_index = primitive.material().index().unwrap_or(0);
        
        // ✓ Each primitive should create a separate Mesh with its own material_index
        let mesh = Mesh {
            material_index,  // ← This must vary per primitive
            // ...
        };
    }
}
```

**Common Error Pattern:**
```rust
// ❌ WRONG - assigns same material to all primitives
let material_index = mesh.primitives().next()
    .and_then(|p| p.material().index())
    .unwrap_or(0);

for primitive in mesh.primitives() {
    create_mesh(material_index);  // All primitives get same material!
}
```

### 7.4 FALLBACK TEXTURE CHECK (Priority 2)
Verify that texture loading doesn't silently fail:

```rust
// In texture loading code:
let texture = match load_texture_from_gltf(primitive.material) {
    Ok(tex) => {
        println!("✓ Loaded texture for material {}", mat_name);
        tex
    }
    Err(e) => {
        println!("❌ Failed to load texture: {}, using fallback", e);
        default_texture  // This might be the problem!
    }
};
```

---

## 8. Testing Strategy

### 8.1 Minimal Reproduction Test
Create a simple scene with 2 cubes using 2 different materials:
```rust
// Test: Two cubes, two materials
let cube1 = create_cube_mesh(material_index: 0);  // Red texture
let cube2 = create_cube_mesh(material_index: 1);  // Blue texture

// Render both
// Expected: Cube 1 is red, Cube 2 is blue
// If both are red → material binding bug
// If both are blue → last material "wins", state not cleared
// If both are white → texture loading bug
```

### 8.2 Material Isolation Test
Render objects one at a time:
```rust
// Render only pine tree (should show bark + leaves)
render_single_model("pine_tree");

// Render only wizard tower (should show wood + stone)
render_single_model("wizard_tower");
```

### 8.3 Bind Group Trace
Log every bind group set call:
```rust
let mut bind_log = Vec::new();
for obj in &self.objects {
    bind_log.push(format!("Object {}: material {}", 
        obj.mesh_index, mesh.material_index));
    render_pass.set_bind_group(2, &material.bind_group, &[]);
    // ...
}
println!("Bind sequence: {:#?}", bind_log);
```

---

## 9. Reference Implementation Comparison

### Bevy's GLTF Material Handling
```rust
// Bevy pattern for per-primitive materials:
for mesh in gltf.meshes() {
    for (primitive_index, primitive) in mesh.primitives().enumerate() {
        let material = match primitive.material() {
            gltf::Material::Some(mat) => {
                // Create unique material from GLTF material
                materials.add(load_gltf_material(mat))
            }
            gltf::Material::None => {
                // Use default material
                materials.add(StandardMaterial::default())
            }
        };
        
        // Each primitive becomes a separate entity with its own material
        spawn_mesh_primitive(primitive, material);
    }
}
```

### Learn WGPU Pattern
```rust
// Per-object material binding:
for obj in objects {
    pass.set_bind_group(0, &camera_bind_group, &[]);
    pass.set_bind_group(1, &obj.material.bind_group, &[]);  // ← Per-object material
    pass.set_bind_group(2, &obj.transform_bind_group, &[]);
    pass.draw_indexed(0..obj.mesh.indices, 0, 0..1);
}
```

---

## 10. Conclusion and Next Steps

### Root Cause Hypothesis (Ranked by Probability)
1. **90% Likely:** GLTF loader creates all meshes with `material_index = 0`
2. **5% Likely:** All materials share same texture due to loader bug
3. **3% Likely:** Bind group state persistence between objects
4. **2% Likely:** UV coordinate corruption

### Immediate Action Items
1. ✅ Add diagnostic logging to material creation
2. ✅ Verify material indices in mesh array
3. ✅ Check GLTF loader per-primitive material assignment
4. ✅ Validate texture diversity in material bind groups

### Success Criteria
After fixes:
- Pine trees show brown bark + green leaves
- Wizard towers show wood planks + stone brick
- At least 4 unique materials visible in scene
- Debug output confirms material switches during render

---

## References
1. [WebGPU Bind Group Best Practices - Toji](https://toji.dev/webgpu-best-practices/bind-groups.html)
2. [Efficiently Rendering glTF - Toji](https://toji.dev/webgpu-gltf-case-study/)
3. [WebGPU Textures Fundamentals](https://webgpufundamentals.org/webgpu/lessons/webgpu-textures.html)
4. [Learn WGPU - Textures and Bind Groups](https://sotrh.github.io/learn-wgpu/beginner/tutorial5-textures/)
5. [From 0 to glTF with WebGPU - Will Usher](https://www.willusher.io/graphics/2024/04/28/0-to-gltf-basic-materials/)

---

**Document Version:** 1.0  
**Last Updated:** 2025-11-21  
**Status:** Ready for Implementation
