# Phase PBR-F: Terrain Layering System - Quick Reference

**Status**: Implementation Complete (Core System) ✅  
**Date**: January 2025  
**Version**: 1.0

---

## 5-Minute Quick Start

### 1. Create Terrain Material (TOML)

```toml
name = "my_terrain"
biome = "grassland"
splat_map = "../splat_map.png"
splat_uv_scale = 0.5
triplanar_enabled = true
triplanar_slope_threshold = 35.0
normal_blend_method = "rnm"
height_blend_enabled = true

[[layers]]
name = "grass"
albedo = "../grass.png"
normal = "../grass_n.png"
orm = "../grass_mra.png"
height = "../grass.png"
uv_scale = [8.0, 8.0]
height_range = [0.0, 50.0]
blend_sharpness = 0.6
triplanar_power = 3.0
metallic = 0.0
roughness = 0.9

# ... up to 3 more layers
```

### 2. Load in Rust

```rust
use astraweave_render::terrain_material::{TerrainMaterialDesc, TerrainMaterialGpu};

// Parse from TOML
let toml_str = std::fs::read_to_string("assets/materials/terrain/my_terrain.toml")?;
let material_desc: TerrainMaterialDesc = toml::from_str(&toml_str)?;

// Convert to GPU format (texture resolver resolves paths to indices)
let texture_resolver = |path: &PathBuf| -> u32 {
    // Your texture loading logic here
    texture_registry.get_or_load(path)
};
let material_gpu = material_desc.to_gpu(&texture_resolver);

// Upload to GPU
let material_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("Terrain Material Buffer"),
    contents: bytemuck::bytes_of(&material_gpu),
    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
});
```

### 3. Use in WGSL Shader

```wgsl
@group(7) @binding(0) var<storage, read> terrain_material: TerrainMaterialGpu;
@group(7) @binding(1) var splat_texture: texture_2d<f32>;
@group(7) @binding(2) var splat_sampler: sampler;
// ... texture arrays for albedo, normal, orm, height

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Evaluate terrain material
    let terrain_sample = evaluate_terrain_material(
        terrain_material,
        in.world_pos,
        in.world_normal,
        in.uv,
        albedo_array,
        normal_array,
        orm_array,
        height_array,
        splat_texture,
        albedo_sampler,
        linear_sampler
    );
    
    // Use in PBR lighting
    let final_color = cook_torrance_brdf(
        terrain_sample.albedo,
        terrain_sample.normal,
        terrain_sample.metallic,
        terrain_sample.roughness,
        view_dir,
        light_dir
    );
    
    return vec4<f32>(final_color, 1.0);
}
```

---

## Struct Reference

### TerrainLayerGpu (64 bytes)

| Field              | Type      | Description                              |
|--------------------|-----------|------------------------------------------|
| texture_indices    | [u32; 4]  | [albedo, normal, orm, height] indices    |
| uv_scale           | [f32; 2]  | Per-layer tiling multiplier              |
| height_range       | [f32; 2]  | [min, max] height for this layer         |
| blend_sharpness    | f32       | Transition sharpness (0.0-1.0)           |
| triplanar_power    | f32       | Triplanar blend power (2.0-8.0)          |
| material_factors   | [f32; 2]  | [metallic, roughness]                    |

### TerrainMaterialGpu (320 bytes)

| Field                      | Type              | Description                              |
|----------------------------|-------------------|------------------------------------------|
| layers                     | [Layer; 4]        | 4 terrain layers                         |
| splat_uv_scale             | f32               | Global splat map tiling                  |
| triplanar_enabled          | u32               | 0=off, 1=on                              |
| triplanar_slope_threshold  | f32               | Angle in degrees (30-45 typical)         |
| normal_blend_method        | u32               | 0=Linear, 1=RNM, 2=UDN                   |
| height_blend_enabled       | u32               | 0=off, 1=on                              |

---

## WGSL Functions Quick Reference

```wgsl
// Main evaluation function
fn evaluate_terrain_material(
    material: TerrainMaterialGpu,
    world_pos: vec3<f32>,
    world_normal: vec3<f32>,
    uv: vec2<f32>,
    // ... texture arrays and samplers
) -> TerrainSample

// Triplanar projection
fn sample_triplanar(
    texture_array: texture_2d_array<f32>,
    sampler_handle: sampler,
    layer_index: u32,
    world_pos: vec3<f32>,
    world_normal: vec3<f32>,
    uv_scale: f32,
    blend_power: f32
) -> vec4<f32>

// Normal blending (RNM - best quality)
fn blend_normals_rnm(base: vec3<f32>, detail: vec3<f32>) -> vec3<f32>

// Calculate triplanar blend factor
fn calculate_triplanar_blend(world_normal: vec3<f32>, slope_threshold: f32) -> f32

// Height-based weight adjustment
fn calculate_height_weights(
    base_weights: vec4<f32>,
    heights: vec4<f32>,
    blend_sharpness: f32
) -> vec4<f32>
```

---

## Factory Methods (Rust)

```rust
// Pre-built terrain configurations
let grassland = TerrainMaterialDesc::grassland();  // Grass + dirt + rock + moss
let forest = TerrainMaterialDesc::forest();        // Forest floor + leaves + moss + bark
let desert = TerrainMaterialDesc::desert();        // Sand + sandstone + rock + pebbles
```

---

## Common Patterns

### Grassland Terrain (Gentle Slopes)

```toml
triplanar_slope_threshold = 35.0  # Late triplanar activation
normal_blend_method = "rnm"       # Best quality
[[layers]]
uv_scale = [8.0, 8.0]             # High detail grass
triplanar_power = 3.0             # Gentle triplanar
```

### Mountain Terrain (Steep Cliffs)

```toml
triplanar_slope_threshold = 30.0  # Early triplanar activation
normal_blend_method = "rnm"       # Best quality for cliffs
[[layers]]
uv_scale = [3.0, 3.0]             # Large rock formations
triplanar_power = 6.0             # Strong triplanar
```

### Desert Terrain (Expansive Flat)

```toml
triplanar_slope_threshold = 40.0  # Rare triplanar (mostly flat)
normal_blend_method = "udn"       # Faster for large areas
[[layers]]
uv_scale = [10.0, 10.0]           # Fine sand detail
triplanar_power = 2.0             # Minimal triplanar
```

---

## Parameter Tuning Guide

### UV Scale (Per-Layer Tiling)

- **Small features** (grass, pebbles): `[8.0-15.0, 8.0-15.0]`
- **Medium features** (dirt, sand): `[4.0-8.0, 4.0-8.0]`
- **Large features** (rock, cliffs): `[2.0-4.0, 2.0-4.0]`

### Blend Sharpness (Layer Transitions)

- **Very soft** (sand, snow): `0.2-0.4`
- **Moderate** (grass, dirt): `0.5-0.6`
- **Sharp** (rock edges): `0.7-0.9`

### Triplanar Power (Slope Blending)

- **Minimal** (gentle hills): `2.0-3.0`
- **Moderate** (rolling terrain): `4.0-5.0`
- **Strong** (cliffs, vertical): `6.0-8.0`

### Normal Blend Method

- **Linear**: Fast, lowest quality (mobile/low-end)
- **UDN**: Fast, good quality (console/desktop)
- **RNM**: Slow, best quality (high-end/hero assets)

---

## Validation Rules

### Layer Count

- **Minimum**: 1 layer (fallback)
- **Maximum**: 4 layers (GPU limitation)

### UV Scale

- **Range**: `> 0.0` (no zero or negative)
- **Typical**: `1.0-20.0`

### Height Range

- **Constraint**: `height_range[0] < height_range[1]`
- **Typical**: `[0.0, 50.0]` to `[100.0, 250.0]`

### Blend Sharpness

- **Range**: `0.0-1.0` (clamped)
- **Typical**: `0.3-0.8`

### Triplanar Power

- **Range**: `> 0.0` (no zero or negative)
- **Typical**: `2.0-8.0`

---

## Performance Quick Tips

### Texture Fetch Count

- **Standard UV**: 16 fetches (4 layers × 4 textures)
- **Full Triplanar**: 48 fetches (4 layers × 4 textures × 3 planes)
- **Hybrid**: ~24 fetches (slope-adaptive)

### ALU Operations

- **Standard UV + Linear**: ~130 ALU ops/pixel
- **Triplanar + RNM**: ~240 ALU ops/pixel

### Optimization Strategies

1. **Use hybrid triplanar**: Only activate on steep slopes (slope_threshold)
2. **Choose UDN over RNM**: For large terrain patches (5-10 ALU ops savings)
3. **Reduce layer count**: 2-3 layers instead of 4 (25-50% faster)
4. **Lower UV scales**: For distant terrain LODs
5. **Disable height blending**: For flat terrains (saves ~20 ALU ops)

---

## Troubleshooting

### Issue: Visible seams at chunk boundaries

**Solution**: Ensure splat maps tile seamlessly or use world-space UVs instead of chunk-local UVs.

### Issue: Over-bright or washed-out materials

**Solution**: Check that `blend_sharpness` isn't too low (<0.2) and weights normalize to 1.0.

### Issue: Stretching on steep slopes

**Solution**: Enable triplanar and lower `triplanar_slope_threshold` (try 30° instead of 45°).

### Issue: Loss of normal map detail

**Solution**: Use `normal_blend_method = "rnm"` instead of "linear" for better quality.

### Issue: Performance too slow

**Solution**: Reduce `triplanar_power`, use UDN instead of RNM, or limit to 3 layers.

---

## Testing Checklist

- [ ] TOML parses without errors
- [ ] All texture paths resolve correctly
- [ ] Splat map loads and samples properly
- [ ] Triplanar activates on slopes (45°+ angles)
- [ ] Normal blending preserves detail
- [ ] Height-based blending looks natural
- [ ] Performance >30 FPS on target hardware
- [ ] No visible seams at chunk boundaries
- [ ] Materials look correct under all lighting conditions

---

## Example Workflow

1. **Create TOML** → Author terrain material definition
2. **Generate Splat Map** → Paint layer weights in image editor or procedurally
3. **Load in Rust** → Parse TOML, convert to GPU format
4. **Upload to GPU** → Create storage buffer with TerrainMaterialGpu
5. **Bind in Shader** → Add bind group for material + splat texture
6. **Evaluate** → Call `evaluate_terrain_material()` in fragment shader
7. **Test** → Visual validation, performance profiling
8. **Iterate** → Adjust parameters, re-export, hot-reload

---

## File Locations

- **Rust Structs**: `astraweave-render/src/terrain_material.rs`
- **WGSL Shaders**: `astraweave-render/shaders/pbr_terrain.wgsl`
- **Unit Tests**: `astraweave-render/tests/test_terrain_material.rs`
- **Demo Materials**: `assets/materials/terrain/*.toml`
- **Design Doc**: `PBR_F_DESIGN.md`
- **This File**: `PBR_F_QUICK_REFERENCE.md`

---

**Version**: 1.0  
**Last Updated**: January 2025  
**Status**: Core system implemented (terrain material example-scoped)  
**Next Steps**: Integration examples, visual validation, performance profiling
