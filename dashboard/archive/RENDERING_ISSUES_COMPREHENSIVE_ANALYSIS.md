# Rendering Issues: Comprehensive Root Cause Analysis & Implementation Plan

**Date**: November 11, 2025  
**Status**: Deep Analysis Phase - Pre-Implementation  
**Objective**: Identify ALL rendering issues and develop systematic fix plan

---

## 🔍 Executive Summary

**Primary Observations** (from screenshots):
1. ✅ **Lighting FIXED**: Scene is now properly lit (35% ambient vs 3% before)
2. ❌ **Black Silhouettes**: Trees, rocks, NPCs rendering as BLACK shadows with SOME faces showing texture
3. ❌ **Terrain**: Smooth color gradients only, NO texture detail visible
4. ⚠️ **Shadows MISSING**: No shadow rendering at all (expected based on current implementation)
5. ⚠️ **Partial Texture Visibility**: SOME cube faces show wood texture correctly, but MOST faces are black

**Critical Discovery**: The issue is **NOT** lighting (that's fixed). The BLACK silhouettes with SOME textured faces is a CLASSIC symptom of **face culling problems** combined with **possible depth test or winding order issues**.

---

## 🎯 Issue Classification & Priority

### Issue 1: Face Culling / Winding Order (P0 - CRITICAL)
**Symptom**: Objects render as black silhouettes, with ONLY certain faces showing texture  
**Evidence**: Screenshot 2 shows tree with wood plank texture visible on some faces, black on others  
**Impact**: 90% of scene geometry appears black/invisible  
**Root Cause Hypothesis**: 
- GLTF models may have inconsistent winding order
- OR: Render pipeline cull mode misconfigured
- OR: Normals inverted causing lighting to fail on back-facing geometry
- OR: Depth test rejecting fragments incorrectly

**Current Pipeline Settings**:
```rust
primitive: wgpu::PrimitiveState {
    topology: wgpu::PrimitiveTopology::TriangleList,
    front_face: wgpu::FrontFace::Ccw,  // Counter-clockwise = front face
    cull_mode: Some(wgpu::Face::Back), // Cull back faces
    polygon_mode: wgpu::PolygonMode::Fill,
    // ...
}
```

**Analysis**: Settings are CORRECT for standard GLTF. However:
- If GLTF exporter flipped winding order → faces appear reversed
- If normals point INWARD → lighting fails → appears black
- If depth test rejects valid fragments → holes appear

### Issue 2: Terrain Texture Missing (P0 - CRITICAL)
**Symptom**: Terrain shows smooth green→brown→gray gradient, no texture detail  
**Evidence**: All 3 screenshots show perfectly smooth terrain coloring  
**Impact**: Terrain looks like untextured placeholder  
**Root Cause Hypothesis**:
- Terrain texture array using FALLBACK COLORS (confirmed in console: "⚠️ No grass texture found")
- UV tiling factor (`* 10.0`) not working with fallback colors
- Blend weights correct, but sampling flat colors instead of real textures

**Current Shader Logic**:
```wgsl
if is_terrain {
    // Blend multiple materials from texture array
    let grass_color = textureSample(terrain_albedo_array, terrain_sampler, in.uv * 10.0, 0).rgb;
    let dirt_color = textureSample(terrain_albedo_array, terrain_sampler, in.uv * 10.0, 1).rgb;
    let stone_color = textureSample(terrain_albedo_array, terrain_sampler, in.uv * 10.0, 2).rgb;
    
    albedo = grass_color * in.material_blend.x 
           + dirt_color * in.material_blend.y 
           + stone_color * in.material_blend.z;
}
```

**Analysis**: Logic is CORRECT, but fallback textures are solid colors (no detail):
- Fallback grass: `[51, 153, 51, 255]` = flat green
- Fallback dirt: `[128, 77, 51, 255]` = flat brown  
- Fallback stone: `[153, 153, 153, 255]` = flat gray
- Result: Smooth gradient blending, but NO texture detail

### Issue 3: Shadow System Not Implemented (P1 - HIGH)
**Symptom**: No shadows visible despite directional light  
**Evidence**: Flat lighting, no depth perception from shadows  
**Impact**: Reduced visual quality and depth perception  
**Root Cause**: Shadow mapping not implemented yet (expected, not a bug)

### Issue 4: Normal Maps Not Working (P2 - MEDIUM)
**Symptom**: Surfaces appear flat, no bump detail  
**Evidence**: Lighting looks too uniform, no surface detail variation  
**Root Cause**: Using fallback normal map (flat `[128, 128, 255]`) for ALL materials

**Console Confirms**:
```
⚠️  Using fallback normal/roughness (TODO: atlas these like albedo)
```

### Issue 5: Specular Highlights Barely Visible (P2 - MEDIUM)
**Symptom**: No shiny highlights on surfaces  
**Evidence**: Everything looks matte despite PBR shader  
**Root Cause**: Using fallback roughness (1.0 = fully rough) for ALL materials

---

## 🔬 Deep Technical Investigation

### Investigation 1: GLTF Winding Order Analysis

**Files to Check**:
- `examples/unified_showcase/src/gltf_loader.rs` (lines 100-200)
- Specifically: Index buffer handling when merging primitives

**Current Implementation**:
```rust
// Offset indices by current vertex count (for merging primitives)
let index_offset = all_vertices.len() as u32;

// Append indices (with offset)
for idx in indices {
    all_indices.push(idx + index_offset);
}
```

**Analysis**: 
- ✅ Offset calculation is CORRECT
- ✅ No index reversal or winding flip
- ✅ GLTF indices preserved as-is

**Hypothesis**: GLTF files themselves may have mixed winding orders (Kenney assets are procedurally generated, may have inconsistencies)

**Test Strategy**:
1. **Disable culling temporarily**: `cull_mode: None` to see if ALL faces render
2. **Check if normals inverted**: Multiply normal by -1.0 in shader to test
3. **Visualize normals as colors**: Output `normal * 0.5 + 0.5` as RGB to check direction

### Investigation 2: Depth Test Interference

**Current Depth State**:
```rust
depth_stencil: Some(wgpu::DepthStencilState {
    format: wgpu::TextureFormat::Depth32Float,
    depth_write_enabled: true,
    depth_compare: wgpu::CompareFunction::Less,
    // ...
})
```

**Analysis**:
- ✅ Settings are STANDARD and correct
- ✅ Skybox renders BEFORE scene (depth writes may interfere?)
- ❌ Skybox should use `DepthLess` to ensure it's BEHIND scene

**Hypothesis**: Skybox depth writes may be occluding scene geometry if skybox depth is wrong

**Test Strategy**:
1. Disable skybox rendering to see if objects become visible
2. Check skybox depth writes (should write max depth, not near plane)

### Investigation 3: Atlas UV Remapping

**Current Shader Function**:
```wgsl
fn remap_atlas_uv(base_uv: vec2<f32>, material_id: u32) -> vec2<f32> {
    let region = atlas_regions[material_id];
    let clamped_uv = clamp(base_uv, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
    return (clamped_uv * region.uv_scale) + region.uv_offset;
}
```

**Analysis**:
- ✅ Math is CORRECT: `(UV * scale) + offset` is standard atlas remapping
- ✅ Clamping prevents atlas bleeding
- ✅ Atlas regions calculated correctly (confirmed in console)

**Atlas Regions** (from console):
```
Material 0 (Grass): UV (0.000, 0.000) scale (0.250, 0.250)  → [0, 0.25] × [0, 0.25]
Material 1 (Dirt):  UV (0.250, 0.000) scale (0.250, 0.250)  → [0.25, 0.5] × [0, 0.25]
Material 2 (Stone): UV (0.500, 0.000) scale (0.250, 0.250)  → [0.5, 0.75] × [0, 0.25]
Material 3 (Wood):  UV (0.750, 0.000) scale (0.250, 0.250)  → [0.75, 1.0] × [0, 0.25]
Material 4 (Leaves):UV (0.000, 0.250) scale (0.250, 0.250)  → [0, 0.25] × [0.25, 0.5]
Material 5 (Roof):  UV (0.250, 0.250) scale (0.250, 0.250)  → [0.25, 0.5] × [0.25, 0.5]
Material 6 (Build): UV (0.500, 0.250) scale (0.250, 0.250)  → [0.5, 0.75] × [0.25, 0.5]
```

**Verification**: Atlas packing is CORRECT (4×2 grid, each material gets 1024×1024 in 4096×4096 atlas)

### Investigation 4: Terrain Texture Array Fallback

**Current Implementation** (`main_bevy_v2.rs` lines 1470-1530):
```rust
let terrain_grass_texture = texture_loader::load_texture_with_usage(
    device, queue,
    "assets/textures/grass.ktx2",
    texture_loader::TextureUsage::Albedo
).or_else(|_| {
    println!("⚠️  grass.ktx2 not found, trying PNG...");
    texture_loader::load_texture_with_usage(
        device, queue,
        "assets/grass.ktx2",
        texture_loader::TextureUsage::Albedo
    )
}).unwrap_or_else(|_| {
    println!("⚠️  No grass texture found, using fallback");
    texture_loader::generate_fallback_texture(device, queue, [0.2, 0.6, 0.2, 1.0])
});
```

**Analysis**:
- ✅ Fallback logic is CORRECT
- ❌ Files `grass.ktx2`, `dirt.ktx2`, `stone.ktx2` do NOT exist in `assets/textures/`
- ❌ Second fallback tries `assets/grass.ktx2` (also doesn't exist)
- ✅ Falls back to solid color textures (1×1 pixel)

**Result**: Terrain samples from 1×1 pixel fallback textures → smooth color gradient, no detail

### Investigation 5: Shader Fragment Output

**Current Fragment Shader** (`pbr_shader.wgsl` lines 155-205):
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let blend_sum = in.material_blend.x + in.material_blend.y + in.material_blend.z;
    let is_terrain = blend_sum > 0.01;
    
    var albedo: vec3<f32>;
    
    if is_terrain {
        // Terrain path (working correctly)
        // ...
    } else {
        // Standard object: Use material atlas with remapped UVs
        let atlas_uv = remap_atlas_uv(in.uv, in.material_id);
        albedo = textureSample(albedo_texture, texture_sampler, atlas_uv).rgb;
    }
    
    // Sample normal and roughness
    let normal_sample = textureSample(normal_texture, texture_sampler, uv_for_sampling).rgb;
    let roughness = textureSample(roughness_texture, texture_sampler, uv_for_sampling).r;
    
    // Apply normal map
    let normal = apply_normal_map(in.world_normal, in.world_tangent, normal_sample);
    
    // Calculate lighting
    let view_dir = normalize(uniforms.camera_pos - in.world_position);
    let light_dir = normalize(vec3<f32>(0.3, 0.8, 0.4));
    
    let final_color = simple_pbr(albedo, normal, roughness, view_dir, light_dir);
    
    return vec4<f32>(final_color, 1.0);
}
```

**Analysis**:
- ✅ Logic flow is CORRECT
- ✅ Atlas UV remapping called properly
- ✅ PBR lighting calculation correct
- ❌ NO early returns or black color fallbacks

**Question**: Why do SOME faces show texture and others don't?

**Hypothesis 1**: Lighting calculation returns BLACK for back-facing geometry
- `simple_pbr()` uses `max(dot(normal, light_dir), 0.0)` → back faces get 0 diffuse
- With 35% ambient, back faces should still be visible (not black)
- UNLESS normals are INVERTED → ndotl becomes negative → clamped to 0 → only ambient remains
- If ambient texture sampling FAILS → returns black → result is black

**Hypothesis 2**: Texture atlas sampling fails for certain UVs
- If UVs are outside [0, 1] range and clamping fails → samples wrong region
- If material_id is WRONG → samples wrong atlas slot
- Result: Either black texture or wrong texture

---

## 🔬 Advanced Technical Analysis

### Vertex Data Flow Verification

**Vertex Structure** (main_bevy_v2.rs lines 169-180):
```rust
struct Vertex {
    position: [f32; 3],     // World position
    normal: [f32; 3],       // World normal (NOT normalized in buffer!)
    uv: [f32; 2],           // Texture coordinates [0, 1]
    material_blend: [f32; 4], // [grass, dirt, stone, unused]
    material_id: u32,       // Atlas slot index (0-6)
}

// Vertex attributes sent to GPU
const ATTRIBS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
    0 => Float32x3, // position - @location(0)
    1 => Float32x3, // normal - @location(1)
    2 => Float32x2, // uv - @location(2)
    3 => Float32x4, // material_blend - @location(3)
    4 => Uint32,    // material_id - @location(4)
];
```

**Shader Vertex Input** (pbr_shader.wgsl lines 44-52):
```wgsl
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) material_blend: vec4<f32>,
    @location(4) material_id: u32,  // Material index in atlas (0-6)
}
```

**Analysis**: Vertex layout is CORRECT and matches shader expectations. Data flows properly from CPU → GPU.

### Material ID Assignment Verification

**Tree Material Assignment** (main_bevy_v2.rs lines 1840-1860):
```rust
let trunk_threshold = min_y + (max_y - min_y) * 0.4; // Bottom 40% = trunk

vertices = loaded_mesh.vertices.iter().map(|v| {
    let is_trunk = v.position[1] < trunk_threshold;
    let material_id = if is_trunk { 
        trunk_verts += 1;
        3 // Wood/bark (index 3 in atlas)
    } else { 
        leaf_verts += 1;
        4 // Leaves (index 4 in atlas)
    };
    
    Vertex {
        position: v.position,
        normal: v.normal,
        uv: v.uv,
        material_blend: DEFAULT_MATERIAL_BLEND, // [0, 0, 0, 0]
        material_id,
    }
}).collect();
```

**Console Verification**:
```
🌲 Tree height range: Y=0.00 to 1.71 (height=1.71)
📏 Trunk threshold: Y < 0.68 = Wood, Y >= 0.68 = Leaves
✅ Assigned materials: 180 trunk vertices (Wood), 204 leaf vertices (Leaves)
```

**Analysis**: 
- ✅ 180 vertices get material_id = 3 (Wood)
- ✅ 204 vertices get material_id = 4 (Leaves)
- ✅ Material assignment is WORKING correctly
- ❓ Why do leaves (material 4) render as BLACK if material_id is correct?

**Hypothesis**: Material 4 (Leaves) atlas slot may contain BLACK texture OR UV remapping fails for material 4

### Atlas Material Slot Inspection

**Material Definitions** (main_bevy_v2.rs lines 838-883):
```rust
materials: vec![
    Material { name: "Grass".to_string(), albedo_path: "assets/textures/texture-d.png", ... },
    Material { name: "Dirt".to_string(), albedo_path: "assets/textures/texture-f.png", ... },
    Material { name: "Stone".to_string(), albedo_path: "assets/textures/cobblestone.png", ... },
    Material { name: "Wood".to_string(), albedo_path: "assets/textures/planks.png", ... },
    Material { name: "Leaves".to_string(), albedo_path: "assets/textures/texture-j.png", ... },
    Material { name: "Roof".to_string(), albedo_path: "assets/textures/roof.png", ... },
    Material { name: "Building".to_string(), albedo_path: "assets/textures/cobblestonePainted.png", ... },
]
```

**Atlas Packing** (confirmed in console):
```
📌 Material 0 (Grass):   Slot [0, 0]     → UV [0.000, 0.000] + scale [0.250, 0.250]
📌 Material 1 (Dirt):    Slot [1024, 0]  → UV [0.250, 0.000] + scale [0.250, 0.250]
📌 Material 2 (Stone):   Slot [2048, 0]  → UV [0.500, 0.000] + scale [0.250, 0.250]
📌 Material 3 (Wood):    Slot [3072, 0]  → UV [0.750, 0.000] + scale [0.250, 0.250]
📌 Material 4 (Leaves):  Slot [0, 1024]  → UV [0.000, 0.250] + scale [0.250, 0.250]
📌 Material 5 (Roof):    Slot [1024, 1024] → UV [0.250, 0.250] + scale [0.250, 0.250]
📌 Material 6 (Building):Slot [2048, 1024] → UV [0.500, 0.250] + scale [0.250, 0.250]
```

**Critical Question**: Does `texture-j.png` exist and contain valid texture data?

**Test Strategy**:
```powershell
# Check if file exists and get dimensions
Test-Path "assets/textures/texture-j.png"
Add-Type -AssemblyName System.Drawing
$img = [System.Drawing.Image]::FromFile("$(Get-Location)\assets\textures\texture-j.png")
Write-Host "texture-j.png: $($img.Width)×$($img.Height)"
$img.Dispose()
```

### Shader Atlas Sampling Deep Dive

**Atlas UV Remapping** (pbr_shader.wgsl lines 143-154):
```wgsl
fn remap_atlas_uv(base_uv: vec2<f32>, material_id: u32) -> vec2<f32> {
    let region = atlas_regions[material_id];  // GPU uniform array lookup
    
    // Clamp UVs to [0,1] to prevent atlas bleeding
    let clamped_uv = clamp(base_uv, vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 1.0));
    
    // Remap: atlas_uv = (clamped_uv * scale) + offset
    return (clamped_uv * region.uv_scale) + region.uv_offset;
}
```

**Example Calculation for Material 4 (Leaves)**:
- Input: `base_uv = [0.5, 0.5]` (center of model UVs)
- Region: `uv_offset = [0.0, 0.25]`, `uv_scale = [0.25, 0.25]`
- Calculation: `([0.5, 0.5] * [0.25, 0.25]) + [0.0, 0.25]`
- Result: `[0.125, 0.125] + [0.0, 0.25] = [0.125, 0.375]`
- Atlas Position: 512px, 1536px (center of material 4's slot)

**Analysis**: Math is CORRECT. Material 4 SHOULD sample from correct atlas region.

### Lighting Calculation Review

**Simple PBR Function** (pbr_shader.wgsl lines 96-122):
```wgsl
fn simple_pbr(
    base_color: vec3<f32>,
    normal: vec3<f32>,
    roughness: f32,
    view_dir: vec3<f32>,
    light_dir: vec3<f32>,
) -> vec3<f32> {
    let light_color = vec3<f32>(1.0, 0.95, 0.9) * 2.0; // Warm white, 2× intensity
    
    // Lambertian diffuse
    let ndotl = max(dot(normal, light_dir), 0.0); // Clamp negative to 0
    let diffuse = base_color * light_color * ndotl;
    
    // Blinn-Phong specular
    let half_dir = normalize(view_dir + light_dir);
    let ndoth = max(dot(normal, half_dir), 0.0);
    let shininess = mix(256.0, 16.0, roughness);
    let specular = pow(ndoth, shininess) * light_color * (1.0 - roughness) * 0.5;
    
    // Ambient (FIXED: was 0.03, now 0.35)
    let ambient = base_color * vec3<f32>(0.35, 0.38, 0.40);
    
    return ambient + diffuse + specular;
}
```

**Analysis for Back-Facing Surfaces**:
- If `dot(normal, light_dir) < 0.0` (back-facing) → `ndotl = 0.0`
- Diffuse term: `base_color * 2.0 * 0.0 = [0, 0, 0]`
- Specular term: Also 0 (no highlight on back faces)
- Ambient term: `base_color * 0.35 = 35% of albedo`

**Expected Result**: Back-facing surfaces should show 35% brightness (visible, not black)

**Observed Result**: Back-facing surfaces are COMPLETELY BLACK (0% brightness)

**Conclusion**: Either:
1. `base_color` is BLACK (texture sampling returns [0, 0, 0])
2. OR: Normals are inverted (front faces treated as back, vice versa)
3. OR: Fragment shader exits early with black color before lighting

### Fragment Shader Early Exit Analysis

**Fragment Shader Full Flow** (pbr_shader.wgsl lines 155-205):
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let blend_sum = in.material_blend.x + in.material_blend.y + in.material_blend.z;
    let is_terrain = blend_sum > 0.01;
    
    var albedo: vec3<f32>;
    var uv_for_sampling: vec2<f32>;
    
    if is_terrain {
        // Terrain branch (trees have blend = [0,0,0,0], so they SKIP this)
        // ...
    } else {
        // Standard object: Use material atlas with remapped UVs
        let atlas_uv = remap_atlas_uv(in.uv, in.material_id);
        albedo = textureSample(albedo_texture, texture_sampler, atlas_uv).rgb;
        uv_for_sampling = atlas_uv;
    }
    
    // NO early returns here! All fragments reach lighting calculation
    
    let normal_sample = textureSample(normal_texture, texture_sampler, uv_for_sampling).rgb;
    let roughness = textureSample(roughness_texture, texture_sampler, uv_for_sampling).r;
    let normal = apply_normal_map(in.world_normal, in.world_tangent, normal_sample);
    let view_dir = normalize(uniforms.camera_pos - in.world_position);
    let light_dir = normalize(vec3<f32>(0.3, 0.8, 0.4));
    let final_color = simple_pbr(albedo, normal, roughness, view_dir, light_dir);
    
    return vec4<f32>(final_color, 1.0); // Final output
}
```

**Analysis**: NO early returns, NO conditional black color fallbacks. Every fragment MUST go through full lighting.

**Critical Insight**: If leaves render BLACK, either:
- `albedo` is BLACK after `textureSample()`
- OR: `simple_pbr()` returns BLACK despite 35% ambient

**Most Likely**: `textureSample(albedo_texture, texture_sampler, atlas_uv).rgb` returns `[0, 0, 0]` for leaves

### Bind Group Verification

**Render Pass Bind Group Setup** (main_bevy_v2.rs lines 2443-2459):
```rust
render_pass.set_pipeline(pipeline);
render_pass.set_bind_group(0, uniform_bg, &[]);       // Camera/view matrices

// Set terrain bind group (group 2) once for all objects
if let Some(terrain_bg) = &self.terrain_bind_group {
    render_pass.set_bind_group(2, terrain_bg, &[]);   // Terrain texture array
}

// Set atlas regions bind group (group 3) once for all objects
if let Some(atlas_regions_bg) = &self.atlas_regions_bind_group {
    render_pass.set_bind_group(3, atlas_regions_bg, &[]); // Atlas UV regions
}

// Set single atlas bind group (group 1) once for all objects
if let Some(atlas_bg) = &self.atlas_bind_group {
    render_pass.set_bind_group(1, atlas_bg, &[]);     // Material atlas texture
    
    // Draw terrain, trees, buildings, NPCs, animals...
}
```

**Analysis**: Bind groups set ONCE before all draw calls (correct and efficient)

**Question**: Is `atlas_bg` correctly bound? Does it contain the atlas texture created in lines 1320-1340?

**Verification Needed**: Check if `self.atlas_bind_group` contains correct texture view

### GPU Texture Format Verification

**Atlas Creation** (atlas_packer.rs lines 165-180):
```rust
let texture = device.create_texture(&wgpu::TextureDescriptor {
    label: Some(label),
    size: wgpu::Extent3d {
        width: self.config.atlas_size,  // 4096
        height: self.config.atlas_size, // 4096
        depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::Rgba8UnormSrgb, // sRGB for albedo (correct!)
    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    view_formats: &[],
});
```

**Analysis**: Format is CORRECT (sRGB for color textures). 4096×4096 is standard and should work.

### Possible Root Causes Ranked by Probability

1. **P(95%): Texture File Issue**
   - `texture-j.png` doesn't exist → fallback generation fails → black texture
   - OR: `texture-j.png` is actually a BLACK image (all pixels [0,0,0])
   - OR: Loading fails silently, no fallback triggered

2. **P(85%): GLTF Winding Order**
   - Kenney GLTF models have MIXED winding orders (trunk CW, leaves CCW)
   - Front-face culling culls leaves, back-face culling culls trunk
   - Result: SOME faces visible (front-facing), others black (culled)

3. **P(75%): Inverted Normals**
   - GLTF loader or shader transforms normals incorrectly
   - Leaves have normals pointing INWARD
   - `ndotl` always negative → clamped to 0 → no diffuse → only ambient
   - If ambient sampling also fails → BLACK

4. **P(40%): Depth Test Issue**
   - Skybox writes depth values that occlude scene geometry
   - OR: Depth buffer not cleared properly
   - Result: Objects fail depth test, don't render

5. **P(20%): Bind Group Null**
   - `self.atlas_bind_group` is `None` or contains wrong texture
   - Shader samples invalid texture → undefined behavior → black
   - UNLIKELY: Console confirms atlas creation succeeded

---

## 🔧 Advanced Shader Debugging Techniques

### Technique 1: Color-Coded Visualization Shaders

**Purpose**: Isolate which pipeline stage is failing by visualizing intermediate data

**Debug Shader A: Material ID Visualization**
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Color-code by material ID (0=black, 7=white)
    let color_value = f32(in.material_id) / 7.0;
    return vec4<f32>(color_value, 0.0, 0.0, 1.0);
}
// Expected: Trees show 2 shades of red (trunk=43% red, leaves=57% red)
```

**Debug Shader B: UV Coordinate Visualization**
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Red=U, Green=V (UV gradient from black→yellow)
    return vec4<f32>(in.uv, 0.0, 1.0);
}
// Expected: Objects show red/green gradient, NOT solid colors
```

**Debug Shader C: Normal Direction Visualization**
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Convert normal from [-1,1] to [0,1] color space
    // Red=+X, Green=+Y, Blue=+Z
    let normal_color = in.world_normal * 0.5 + 0.5;
    return vec4<f32>(normal_color, 1.0);
}
// Expected: Trees show rainbow colors (smooth gradients), NOT solid black/white
```

**Debug Shader D: Raw Atlas Sampling** (NO UV remapping)
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample atlas directly with model UVs (should show TILED atlas)
    let albedo = textureSample(albedo_texture, texture_sampler, in.uv).rgb;
    return vec4<f32>(albedo, 1.0);
}
// Expected: Objects show REPEATED atlas (7 materials tiled), NOT black
```

**Debug Shader E: Atlas UV Visualization**
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let atlas_uv = remap_atlas_uv(in.uv, in.material_id);
    // Visualize remapped UVs (should be in [0,1] range)
    return vec4<f32>(atlas_uv, 0.0, 1.0);
}
// Expected: Objects show red/green gradient (UVs in atlas space)
```

**Debug Shader F: Forced White Lighting**
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Force white albedo to test lighting only
    let albedo = vec3<f32>(1.0, 1.0, 1.0);
    
    let normal_sample = textureSample(normal_texture, texture_sampler, in.uv).rgb;
    let roughness = textureSample(roughness_texture, texture_sampler, in.uv).r;
    let normal = apply_normal_map(in.world_normal, in.world_tangent, normal_sample);
    let view_dir = normalize(uniforms.camera_pos - in.world_position);
    let light_dir = normalize(vec3<f32>(0.3, 0.8, 0.4));
    let final_color = simple_pbr(albedo, normal, roughness, view_dir, light_dir);
    
    return vec4<f32>(final_color, 1.0);
}
// Expected: Objects render as WHITE lit surfaces with proper shading
```

### Technique 2: PowerShell Asset Verification

**Check Texture Files Exist**:
```powershell
# Check all material albedo textures
$materials = @(
    "texture-d.png",    # Grass
    "texture-f.png",    # Dirt
    "cobblestone.png",  # Stone
    "planks.png",       # Wood
    "texture-j.png",    # Leaves
    "roof.png",         # Roof
    "cobblestonePainted.png"  # Building
)

foreach ($file in $materials) {
    $path = "assets\textures\$file"
    if (Test-Path $path) {
        $img = [System.Drawing.Image]::FromFile("$(Get-Location)\$path")
        Write-Host "✅ $file`: $($img.Width)×$($img.Height)" -ForegroundColor Green
        $img.Dispose()
    } else {
        Write-Host "❌ $file`: NOT FOUND" -ForegroundColor Red
    }
}
```

**Sample Pixel Colors**:
```powershell
# Check if texture-j.png is actually BLACK
Add-Type -AssemblyName System.Drawing
$bitmap = [System.Drawing.Bitmap]::FromFile("$(Get-Location)\assets\textures\texture-j.png")

# Sample center pixel
$centerX = $bitmap.Width / 2
$centerY = $bitmap.Height / 2
$pixel = $bitmap.GetPixel($centerX, $centerY)

Write-Host "Center pixel color: R=$($pixel.R) G=$($pixel.G) B=$($pixel.B)"

# Check if predominantly black
$avgColor = ($pixel.R + $pixel.G + $pixel.B) / 3
if ($avgColor < 10) {
    Write-Host "⚠️  WARNING: Texture appears BLACK (avg=$avgColor)" -ForegroundColor Yellow
}

$bitmap.Dispose()
```

### Technique 3: Winding Order Test

**Temporary Pipeline Change**:
```rust
// main_bevy_v2.rs line 1588
// BEFORE:
cull_mode: Some(wgpu::Face::Back),

// TEST 1: Disable culling (render all faces)
cull_mode: None,

// TEST 2: Cull front faces instead (reverse culling)
cull_mode: Some(wgpu::Face::Front),

// TEST 3: Flip winding order
front_face: wgpu::FrontFace::Cw,  // Was: Ccw
cull_mode: Some(wgpu::Face::Back),
```

**Expected Results**:
- **No culling**: ALL faces render (both front and back) - objects should be fully visible
- **Cull front**: Opposite faces render - if this fixes it, winding order is inverted
- **Flip winding**: If this fixes it, GLTF models use CW winding (non-standard)

### Technique 4: Normal Inversion Test

**Vertex Shader Change**:
```wgsl
// pbr_shader.wgsl line 71
// BEFORE:
out.world_normal = normalize((uniforms.model * vec4<f32>(in.normal, 0.0)).xyz);

// TEST: Invert normals
out.world_normal = -normalize((uniforms.model * vec4<f32>(in.normal, 0.0)).xyz);
```

**Expected Result**: If normals were inverted, lighting should now work correctly

### Technique 5: Depth Buffer Inspection

**Disable Skybox Test**:
```rust
// main_bevy_v2.rs line 2430
// Comment out skybox rendering to test if depth interferes
/*
if let (Some(skybox_pipeline), Some(skybox_uniform_bg), Some(skybox_texture_bg)) = 
    (&self.skybox_pipeline, &self.skybox_uniform_bind_group, &self.skybox_bind_group) 
{
    render_pass.set_pipeline(skybox_pipeline);
    render_pass.set_bind_group(0, skybox_uniform_bg, &[]);
    render_pass.set_bind_group(1, skybox_texture_bg, &[]);
    render_pass.draw(0..3, 0..1);
}
*/
```

**Expected Result**: Objects should render identically (skybox should NOT affect scene)

### Technique 6: Console Logging Enhancement

**Add Fragment Shader Print** (wgpu doesn't support printf, but we can log on CPU):
```rust
// main_bevy_v2.rs - Add debug logging for first tree vertex
println!("🔍 DEBUG: First tree vertex data:");
println!("   Position: {:?}", all_tree_vertices[0].position);
println!("   Normal: {:?}", all_tree_vertices[0].normal);
println!("   UV: {:?}", all_tree_vertices[0].uv);
println!("   Material ID: {}", all_tree_vertices[0].material_id);
println!("   Material Blend: {:?}", all_tree_vertices[0].material_blend);
```

**Add Atlas Bind Group Verification**:
```rust
// After atlas bind group creation
println!("🔍 Atlas bind group created: {:?}", self.atlas_bind_group.is_some());
println!("🔍 Atlas texture created: {:?}", self.atlas_texture.is_some());
```

---

## 🎯 Diagnostic Decision Tree

```
START: Black silhouettes with some textured faces visible

│
├─ Test 1: Disable culling (cull_mode: None)
│  │
│  ├─ ✅ ALL faces now visible
│  │  └─ ROOT CAUSE: Winding order issue
│  │     └─ FIX: Use cull_mode: Some(wgpu::Face::Front)
│  │            OR: Flip winding in GLTF loader
│  │            OR: Change front_face to Cw
│  │
│  └─ ❌ Still black silhouettes
│     └─ CONTINUE TO TEST 2
│
├─ Test 2: Debug Shader F (Forced white albedo)
│  │
│  ├─ ✅ Objects render as white with proper lighting
│  │  └─ ROOT CAUSE: Texture sampling failure
│  │     └─ CONTINUE TO TEST 5
│  │
│  └─ ❌ Still black
│     └─ CONTINUE TO TEST 3
│
├─ Test 3: Debug Shader C (Normal visualization)
│  │
│  ├─ ✅ Rainbow colors visible
│  │  └─ Normals are CORRECT
│  │     └─ CONTINUE TO TEST 4
│  │
│  └─ ❌ Solid black/white only
│     └─ ROOT CAUSE: Normals inverted or broken
│        └─ FIX: Invert normals in vertex shader
│
├─ Test 4: Debug Shader B (UV visualization)
│  │
│  ├─ ✅ Red/green gradient visible
│  │  └─ UVs are CORRECT
│  │     └─ CONTINUE TO TEST 5
│  │
│  └─ ❌ Solid colors only
│     └─ ROOT CAUSE: UVs not being passed correctly
│        └─ FIX: Check vertex buffer layout
│
├─ Test 5: Debug Shader D (Raw atlas sampling)
│  │
│  ├─ ✅ Tiled atlas visible on objects
│  │  └─ Atlas texture is VALID
│  │     └─ ROOT CAUSE: UV remapping calculation wrong
│  │        └─ FIX: Debug remap_atlas_uv() function
│  │
│  └─ ❌ Still black
│     └─ ROOT CAUSE: Atlas texture is BLACK or not bound
│        └─ CONTINUE TO ASSET VERIFICATION
│
└─ Asset Verification: Check texture files
   │
   ├─ ✅ All files exist and non-black
   │  └─ ROOT CAUSE: Bind group not connected
   │     └─ FIX: Check atlas bind group creation
   │
   └─ ❌ texture-j.png missing or BLACK
      └─ ROOT CAUSE: Missing/corrupted texture file
         └─ FIX: Replace with known-good texture
```

---

## 📋 Pre-Implementation Checklist

Before starting Phase 1 tests, ensure:

- [ ] Latest build is successful (`cargo build -p unified_showcase --release`)
- [ ] Have screenshots of CURRENT state (baseline for comparison)
- [ ] Know how to quickly revert changes (git or backup files)
- [ ] Have PowerShell ready for asset verification
- [ ] Understand which files to edit for each test
- [ ] Have text editor open to both shader and main files

**Recommended Test Order** (most likely to least likely):
1. Test 1: Disable culling (5 min) - Most likely culprit based on symptoms
2. Test 5: Debug Shader D (10 min) - Check if atlas texture valid
3. Test 6: Asset Verification (5 min) - Check texture-j.png exists
4. Test 2: Debug Shader F (10 min) - Isolate texture vs lighting
5. Test 3: Debug Shader C (10 min) - Verify normal directions

**Total Time**: ~40 minutes for comprehensive diagnostics

---

## 🧪 Diagnostic Tests to Run (Next Prompt)

### Test 1: Disable Culling (5 min - CRITICAL)
**Purpose**: Determine if winding order is the issue  
**Change**: `cull_mode: None` in pipeline  
**Expected**: ALL faces render (both front and back)  
**If passes**: Issue is winding order, need to fix GLTF loader or flip faces  
**If fails**: Issue is NOT culling, investigate lighting/texture sampling

### Test 2: Visualize Normals as Colors (10 min - HIGH)
**Purpose**: Verify normals are correct direction  
**Change**: In fragment shader, output `vec4<f32>(in.world_normal * 0.5 + 0.5, 1.0)`  
**Expected**: Surfaces show rainbow colors (red=+X, green=+Y, blue=+Z)  
**If fails**: Normals inverted or incorrect

### Test 3: Visualize UVs as Colors (10 min - HIGH)
**Purpose**: Verify UV coordinates are correct  
**Change**: In fragment shader, output `vec4<f32>(in.uv, 0.0, 1.0)`  
**Expected**: Objects show red/green gradient (UV mapping visible)  
**If fails**: UVs are broken or not being passed correctly

### Test 4: Visualize Material ID (5 min - MEDIUM)
**Purpose**: Verify material_id is correct per-vertex  
**Change**: In fragment shader, output `vec4<f32>(f32(in.material_id) / 7.0, 0.0, 0.0, 1.0)`  
**Expected**: Objects show red gradient based on material (material 0=black, 7=red)  
**If fails**: Material IDs not being passed correctly

### Test 5: Sample Raw Atlas (10 min - HIGH)
**Purpose**: Verify atlas texture contains correct data  
**Change**: In fragment shader, output `textureSample(albedo_texture, texture_sampler, in.uv)`  
**Expected**: Objects show REPEATED atlas texture (7 materials tiled)  
**If fails**: Atlas creation failed or texture not bound correctly

### Test 6: Disable Skybox (5 min - MEDIUM)
**Purpose**: Check if skybox depth interferes with scene  
**Change**: Comment out skybox rendering  
**Expected**: Objects should still render identically  
**If passes**: Skybox not the issue

### Test 7: Use Terrain Textures for Objects (15 min - LOW)
**Purpose**: Verify texture sampling works with known-good textures  
**Change**: Create PNG versions of grass/dirt/stone and use in terrain array  
**Expected**: Terrain shows texture detail  
**If passes**: Issue is fallback texture generation or file loading

---

## 🛠️ Implementation Plan (Phased Approach)

### Phase 1: Critical Diagnostics (30 min)
**Objective**: Identify exact root cause of black silhouettes

1. **Test 1: Disable Culling**
   - File: `main_bevy_v2.rs` line 1588
   - Change: `cull_mode: None`
   - Rebuild, test, screenshot
   - Revert if not helpful

2. **Test 2: Visualize Normals**
   - File: `pbr_shader.wgsl` line 200
   - Add: `return vec4<f32>(in.world_normal * 0.5 + 0.5, 1.0);` at start of fragment
   - Rebuild, test, screenshot
   - Expected: Rainbow colored objects

3. **Test 3: Visualize UVs**
   - File: `pbr_shader.wgsl` line 200
   - Add: `return vec4<f32>(in.uv, 0.0, 1.0);` at start of fragment
   - Rebuild, test, screenshot
   - Expected: Red/green gradient

4. **Test 4: Sample Raw Atlas**
   - File: `pbr_shader.wgsl` line 183
   - Change: `albedo = textureSample(albedo_texture, texture_sampler, in.uv).rgb;`
   - Rebuild, test, screenshot
   - Expected: Repeated atlas texture on all objects

5. **Test 5: Force White Albedo**
   - File: `pbr_shader.wgsl` line 183
   - Change: `albedo = vec3<f32>(1.0, 1.0, 1.0);`
   - Rebuild, test, screenshot
   - Expected: All objects render as white lit surfaces

**Success Criteria**: Identify which component is causing black rendering

### Phase 2: Root Cause Fix (1-2 hours)
**Objective**: Fix identified issue based on Phase 1 results

**Scenario A: Culling Issue**
- Fix: Flip winding order in GLTF loader OR change front face to CW
- Implementation: Add index reversal in `gltf_loader.rs`
- Test: All faces render correctly

**Scenario B: Normals Inverted**
- Fix: Multiply normals by -1.0 in vertex shader OR GLTF loader
- Implementation: `out.world_normal = -normalize(...)` in shader
- Test: Lighting works on all faces

**Scenario C: UV/Material ID Issues**
- Fix: Debug vertex data generation in terrain/tree loading
- Implementation: Add extensive logging for UVs and material_ids
- Test: Verify correct data in shader

**Scenario D: Texture Sampling Failure**
- Fix: Add error handling for texture sampling
- Implementation: Check atlas creation, verify bind group
- Test: Sample succeeds with correct colors

### Phase 3: Terrain Texture Fix (30 min)
**Objective**: Replace fallback colors with real textures

**Option A: Use Existing Kenney Textures**
1. Copy `assets/textures/grass_*.png` (if exists) to terrain texture paths
2. Or use Kenney 64×64 textures directly (grass, dirt, stone)
3. Update terrain texture loading paths

**Option B: Generate Procedural Textures**
1. Create 256×256 noise-based grass texture (green with variation)
2. Create 256×256 dirt texture (brown with particles)
3. Create 256×256 stone texture (gray with cracks)
4. Use `image` crate to generate and save as PNG

**Option C: Use Atlas Materials for Terrain**
1. Modify shader to use material atlas for terrain too
2. Update terrain rendering to pass material_ids instead of blend weights
3. Simplifies system (single texture for all)

**Recommended**: Option C (unified material system, already working for objects)

### Phase 4: Normal/Roughness Atlas (1-2 hours)
**Objective**: Add proper normal and roughness maps to atlas

1. Extend `atlas_packer.rs` to create 3 separate atlases:
   - Albedo atlas (already done)
   - Normal atlas (new)
   - MRA atlas (Metallic/Roughness/AO, new)

2. Update shader bindings:
   ```wgsl
   @group(1) @binding(0) var albedo_atlas: texture_2d<f32>;
   @group(1) @binding(1) var texture_sampler: sampler;
   @group(1) @binding(2) var normal_atlas: texture_2d<f32>;
   @group(1) @binding(3) var roughness_atlas: texture_2d<f32>;
   ```

3. Load normal/roughness for each material (or generate fallbacks)

4. Update shader to sample all 3 atlases with same UV remapping

### Phase 5: Shadow Mapping (4-6 hours - OPTIONAL for now)
**Objective**: Add depth map shadows from directional light

**Deferred**: Shadows are visual polish, not critical for correct rendering

---

## 📊 Expected Results per Phase

### After Phase 1 (Diagnostics):
- ✅ Know exact root cause of black silhouettes
- ✅ Have working diagnostic shader for future debugging
- ✅ Can reproduce issue reliably

### After Phase 2 (Root Cause Fix):
- ✅ All object faces render with correct textures
- ✅ Trees show wood trunk + leaf canopy (two-tone)
- ✅ Rocks show stone texture
- ✅ NPCs show skin texture
- ✅ No more black silhouettes

### After Phase 3 (Terrain Textures):
- ✅ Terrain shows grass/dirt/stone TEXTURE detail
- ✅ Proper tiling (UV × 10.0 visible as repeated texture)
- ✅ Smooth transitions between materials
- ✅ No more flat color gradients

### After Phase 4 (Normal/Roughness):
- ✅ Surfaces show bump detail from normal maps
- ✅ Different materials have different roughness (shiny vs matte)
- ✅ Better depth perception from surface detail

---

## 🔍 Key Files Reference

### Rendering Pipeline
- `main_bevy_v2.rs` lines 1565-1610: Pipeline descriptor (culling, depth, blending)
- `main_bevy_v2.rs` lines 2430-2520: Draw calls and bind group setup

### Shaders
- `pbr_shader.wgsl` lines 62-90: Vertex shader (normal transformation)
- `pbr_shader.wgsl` lines 143-154: Atlas UV remapping
- `pbr_shader.wgsl` lines 155-205: Fragment shader (texture sampling, lighting)
- `pbr_shader.wgsl` lines 96-122: PBR lighting calculation

### Asset Loading
- `gltf_loader.rs` lines 108-200: GLTF mesh loading and merging
- `gltf_loader.rs` lines 54-102: Normal/UV generation
- `texture_loader.rs` lines 50-120: Texture loading with format handling
- `texture_loader.rs` lines 183-246: Fallback texture generation

### Material System
- `atlas_packer.rs` lines 10-80: Atlas region calculation
- `atlas_packer.rs` lines 95-180: Atlas building and GPU upload
- `main_bevy_v2.rs` lines 1270-1340: Material atlas creation
- `main_bevy_v2.rs` lines 1385-1420: Atlas bind group setup

### Terrain System
- `main_bevy_v2.rs` lines 1440-1540: Terrain texture array setup
- `main_bevy_v2.rs` lines 1650-1750: Terrain vertex generation

---

## 🎯 Success Metrics

**Phase 1 Success** (Diagnostics):
- [ ] Identified root cause with 95%+ confidence
- [ ] Have working test shaders for UV/normal/material visualization
- [ ] Know which component (culling/normals/UVs/sampling) is broken

**Phase 2 Success** (Root Cause Fix):
- [ ] 0% black silhouettes (all faces render)
- [ ] 100% texture visibility on objects
- [ ] Trees show 2-tone (trunk + leaves) correctly
- [ ] Proper lighting response (diffuse + specular)

**Phase 3 Success** (Terrain Textures):
- [ ] Terrain shows clear texture detail (not smooth colors)
- [ ] Visible texture tiling (UV × 10.0 working)
- [ ] Smooth material transitions (grass → dirt → stone)

**Phase 4 Success** (Normal/Roughness):
- [ ] Surface bump detail visible
- [ ] Different roughness per material (wood matte, stone rough)
- [ ] Specular highlights visible on shiny surfaces

---

## 🚨 Critical Path (Must-Do First)

1. **Test 1**: Disable culling → See if ALL faces render
2. **Test 2**: Visualize normals → Verify lighting direction
3. **Test 5**: Force white albedo → Isolate lighting from texture sampling
4. **Based on results**: Apply targeted fix in Phase 2
5. **Verify**: All objects render correctly with textures
6. **Then**: Move to terrain texture fix (Phase 3)

**DO NOT** skip Phase 1 diagnostics. Without knowing root cause, fixing randomly will waste time.

---

## 📝 Notes for Next Implementation

- Use `cargo build -p unified_showcase --release` for fast iteration
- Test each change individually with screenshots
- Keep notes on what works/doesn't work for debugging history
- Consider creating a "debug shader mode" toggle (F1 key) for live testing
- Add FPS counter to verify performance isn't degrading

---

**Next Prompt Instructions**:
1. Start with Phase 1 Test 1 (disable culling)
2. Rebuild and screenshot
3. If that doesn't reveal issue, proceed to Test 2, 3, 5 in order
4. Report findings before attempting fixes
5. Use this document as reference for implementation details

