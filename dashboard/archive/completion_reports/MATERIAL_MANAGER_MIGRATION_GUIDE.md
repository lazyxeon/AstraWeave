# MaterialManager Migration Guide

**Purpose**: Guide for migrating examples and applications from local material loading to the unified `MaterialManager` API.

**Status**: Active (Task 2 - Material System Unification)

---

## Why Migrate?

### Problems with Local Material Loading

**Before** (scattered patterns):
```rust
// visual_3d example
fn validate_textures() -> anyhow::Result<()> {
    let texture_files = [
        "assets/grass.png",
        "assets/dirt.png",
        "assets/stone.png",
        // ... manual validation
    ];
    astraweave_render::texture::validate_texture_assets(&texture_files)
}

// unified_showcase example
pub fn load_biome_toml(biome_name: &str) -> Option<(BiomeToml, PathBuf)> {
    let candidates = [
        base.join("textures").join(biome_name).join("materials.toml"),
        base.join(biome_name).join("materials.toml"),
    ];
    for path in candidates {
        if path.exists() {
            if let Ok(txt) = std::fs::read_to_string(&path) {
                match toml::from_str::<BiomeToml>(&txt) {
                    Ok(doc) => return Some((doc, dir)),
                    // ... manual error handling
                }
            }
        }
    }
    None
}
```

**Issues**:
- ❌ Duplicate TOML parsing logic across examples
- ❌ Inconsistent error handling
- ❌ No centralized validation
- ❌ Manual texture loading code
- ❌ No hot-reload support
- ❌ Bind group management scattered

### Benefits of MaterialManager

**After** (unified API):
```rust
use astraweave_render::MaterialManager;

let mut manager = MaterialManager::new();

// One-line biome loading with validation
let stats = manager.load_biome(
    &device,
    &queue,
    Path::new("assets/materials/temperate"),
).await?;

println!("{}", stats.concise_summary());
// Output: [materials] biome=temperate layers=5 | albedo L/S=4/1 | ...

// Cached bind group layout
let layout = manager.get_or_create_bind_group_layout(&device);

// Instant bind group creation
let bind_group = manager.create_bind_group(&device, layout)?;

// Hot-reload for live editing
let stats = manager.reload_biome(&device, &queue, biome_path).await?;
```

**Advantages**:
- ✅ Single source of truth for material data
- ✅ Automatic TOML validation (duplicates, tiling, triplanar)
- ✅ Unified error messages with context
- ✅ Built-in hot-reload support
- ✅ Cached bind group layouts (performance)
- ✅ Consistent material array indexing
- ✅ Statistics and debugging info

---

## Migration Steps

### Step 1: Add MaterialManager Dependency

**Cargo.toml** (if not already present):
```toml
[dependencies]
astraweave-render = { path = "../../astraweave-render", features = ["textures"] }
```

### Step 2: Replace Validation Functions

**Old Pattern**:
```rust
fn validate_textures() -> anyhow::Result<()> {
    #[cfg(feature = "textures")]
    {
        let texture_files = ["assets/grass.png", "assets/dirt.png"];
        astraweave_render::texture::validate_texture_assets(&texture_files)
    }
    #[cfg(not(feature = "textures"))]
    {
        println!("🎨 Texture validation skipped");
        Ok(())
    }
}
```

**New Pattern**:
```rust
use astraweave_render::MaterialManager;
use std::path::Path;

async fn load_materials(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<MaterialManager> {
    let mut manager = MaterialManager::new();
    
    #[cfg(feature = "textures")]
    {
        let stats = manager.load_biome(
            device,
            queue,
            Path::new("assets/materials/temperate"),
        ).await?;
        
        println!("🎨 Materials loaded: {}", stats.concise_summary());
    }
    #[cfg(not(feature = "textures"))]
    {
        println!("🎨 Material loading skipped (textures feature not enabled)");
    }
    
    Ok(manager)
}
```

### Step 3: Replace Direct Texture Loading

**Old Pattern** (manual texture management):
```rust
renderer.set_albedo_from_rgba8(img.width, img.height, &img.rgba8);
renderer.set_normal_from_rgba8(img_n.width, img_n.height, &img_n.rgba8);
renderer.set_metallic_roughness_from_rgba8(img_mr.width, img_mr.height, &img_mr.rgba8);
```

**New Pattern** (MaterialManager bind group):
```rust
// One-time setup
let bind_group_layout = material_manager.get_or_create_bind_group_layout(&device);
let bind_group = material_manager.create_bind_group(&device, &bind_group_layout)?;

// In render loop
render_pass.set_bind_group(1, &bind_group, &[]);

// Material selection via push constants or uniform
let material_index: u32 = 2; // e.g., "grass" layer from materials.toml
render_pass.set_push_constants(
    wgpu::ShaderStages::FRAGMENT,
    0,
    bytemuck::cast_slice(&[material_index]),
);
```

### Step 4: Update Shader Code

**Old Shader** (single textures):
```wgsl
@group(1) @binding(0) var t_albedo: texture_2d<f32>;
@group(1) @binding(1) var s_albedo: sampler;
@group(1) @binding(2) var t_normal: texture_2d<f32>;
@group(1) @binding(3) var s_normal: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = textureSample(t_albedo, s_albedo, in.uv);
    let normal = textureSample(t_normal, s_normal, in.uv);
    // ...
}
```

**New Shader** (material arrays):
```wgsl
@group(1) @binding(0) var albedo_array: texture_2d_array<f32>;
@group(1) @binding(1) var sampler_repeat: sampler;
@group(1) @binding(2) var normal_array: texture_2d_array<f32>;
@group(1) @binding(3) var sampler_linear: sampler;
@group(1) @binding(4) var mra_array: texture_2d_array<f32>;

struct PushConstants {
    material_index: u32,
}
var<push_constant> pc: PushConstants;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let layer = f32(pc.material_index);
    let albedo = textureSample(albedo_array, sampler_repeat, in.uv, i32(layer));
    let normal_rg = textureSample(normal_array, sampler_linear, in.uv, i32(layer));
    let mra = textureSample(mra_array, sampler_linear, in.uv, i32(layer));
    
    // Reconstruct normal Z from RG
    let normal_xy = normal_rg.rg * 2.0 - 1.0;
    let normal_z = sqrt(max(0.0, 1.0 - dot(normal_xy, normal_xy)));
    let normal = vec3<f32>(normal_xy, normal_z);
    
    // Extract PBR parameters
    let metallic = mra.r;
    let roughness = mra.g;
    let ao = mra.b;
    
    // ... lighting calculations
}
```

### Step 5: Add Hot-Reload Support

**Example** (keyboard shortcut for live editing):
```rust
use winit::keyboard::{KeyCode, PhysicalKey};

// In event loop
WindowEvent::KeyboardInput {
    event: KeyEvent {
        state: ElementState::Pressed,
        physical_key: PhysicalKey::Code(KeyCode::KeyR),
        ..
    },
    ..
} => {
    // Hot-reload materials
    match material_manager.reload_biome(
        &device,
        &queue,
        Path::new("assets/materials/temperate"),
    ).await {
        Ok(stats) => {
            println!("🔄 Materials reloaded: {}", stats.concise_summary());
            // Recreate bind group
            let layout = material_manager.get_or_create_bind_group_layout(&device);
            bind_group = material_manager.create_bind_group(&device, &layout)
                .expect("Failed to create bind group after reload");
        }
        Err(e) => eprintln!("❌ Reload failed: {}", e),
    }
}
```

---

## Example Migrations

### visual_3d Example

**Files to modify**:
- `examples/visual_3d/src/main.rs`
- `examples/visual_3d/Cargo.toml` (ensure `textures` feature)

**Changes**:
1. Remove `validate_textures()` function
2. Add `MaterialManager` initialization in `main()`
3. Replace individual texture loading with MaterialManager bind group
4. Add 'R' key hot-reload shortcut
5. Update shader if needed (or keep simple for demo)

**Estimated effort**: 30-45 minutes

### cutscene_render_demo Example

**Files to modify**:
- `examples/cutscene_render_demo/src/main.rs`
- `examples/cutscene_render_demo/shaders/*.wgsl` (if using materials)

**Changes**:
1. Replace any local material loading with MaterialManager
2. Add material array sampling in shaders
3. Test with different biomes (grassland, desert, forest)

**Estimated effort**: 20-30 minutes

### unified_showcase Example

**Files to modify**:
- `examples/unified_showcase/src/main.rs`

**Changes**:
1. Remove `load_biome_toml()` function (~50 lines)
2. Remove `BiomeToml` struct definitions
3. Replace with MaterialManager API calls
4. Keep asset path resolution logic
5. Update tests to use MaterialManager validation

**Estimated effort**: 45-60 minutes (due to test updates)

---

## Testing Checklist

After migration, verify:

- [ ] Example compiles without warnings
- [ ] Example runs and displays expected visuals
- [ ] Materials load correctly (check console output)
- [ ] Hot-reload works (press 'R' key)
- [ ] Statistics are accurate
- [ ] Validation errors are clear (try invalid materials.toml)
- [ ] Performance is unchanged or improved
- [ ] CI passes (cargo build, cargo test)

---

## Common Issues & Solutions

### Issue: "No materials loaded"

**Cause**: MaterialManager not initialized or load failed

**Solution**:
```rust
// Check for errors
match manager.load_biome(&device, &queue, path).await {
    Ok(stats) => println!("✅ Loaded: {}", stats.concise_summary()),
    Err(e) => {
        eprintln!("❌ Failed to load materials: {}", e);
        // Use fallback/default materials
    }
}
```

### Issue: "Bind group creation failed"

**Cause**: Trying to create bind group before loading materials

**Solution**:
```rust
// Always load materials first
let stats = manager.load_biome(&device, &queue, path).await?;

// Then create bind group
let bind_group = manager.create_bind_group(&device, &layout)?;
```

### Issue: "Invalid layer index in shader"

**Cause**: Shader trying to sample layer not in arrays.toml

**Solution**:
```rust
// Check stats for valid range
let stats = manager.current_stats().expect("No materials loaded");
let max_layer = stats.albedo_layers + stats.albedo_singles - 1;

// Clamp index in shader or CPU
let material_index = player_terrain_id.min(max_layer);
```

### Issue: "TOML parse error"

**Cause**: Invalid materials.toml or arrays.toml syntax

**Solution**:
```bash
# Validate TOML manually
cargo run -p astraweave-render --example validate_materials -- assets/materials/temperate

# Or use MaterialManager validation
let pack = MaterialPackDesc { /* ... */ };
if let Err(e) = validate_material_pack(&pack) {
    eprintln!("Validation failed: {}", e);
}
```

---

## Performance Considerations

### Bind Group Caching

MaterialManager caches the bind group layout:

```rust
// First call creates and caches
let layout1 = manager.get_or_create_bind_group_layout(&device);

// Subsequent calls return cached reference (fast)
let layout2 = manager.get_or_create_bind_group_layout(&device);

assert!(std::ptr::eq(layout1, layout2)); // Same object
```

### Hot-Reload Efficiency

Hot-reload recreates textures but reuses layout:

```rust
// Efficient: layout cached, only textures recreated
manager.reload_biome(&device, &queue, path).await?;

// Bind group needs recreation (cheap)
let bind_group = manager.create_bind_group(&device, &layout)?;
```

### Memory Usage

- **Array Textures**: More GPU memory than single textures, but batch-friendly
- **Trade-off**: Extra memory for reduced draw calls and bind group switches
- **Recommendation**: Use array textures for terrain/props (many instances), single textures for unique assets (characters)

---

## Next Steps

After migrating all examples:

1. ✅ Run tests: `cargo test -p astraweave-render`
2. ✅ Build examples: `cargo build --examples --features textures`
3. ✅ Manual smoke test: Run each example and verify visuals
4. ✅ Add golden image tests for visual regression
5. ✅ Update docs: Mark Task 2 complete in PHASE2_STATUS_REPORT.md
6. ✅ Proceed to Task 3: GPU-Driven Rendering (frustum culling)

---

**Document Version**: 1.0  
**Last Updated**: October 1, 2025  
**Author**: GitHub Copilot  
**Related**: PHASE2_IMPLEMENTATION_PLAN.md, PHASE2_TASK2_PROGRESS.md
