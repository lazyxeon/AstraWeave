# PolyHaven Quick Start Guide

**Goal**: Get your first PolyHaven material loaded in 5 minutes!

---

## Step 1: Download a Sample Material (2 min)

Let's start with **Metal Plate** (easiest to verify):

1. **Visit**: https://polyhaven.com/a/metal_plate

2. **Click**: "Download" button (top right)

3. **Select**:
   - Resolution: **2K**
   - Format: **PNG**
   - Maps: **All maps** (or individual: Diffuse, Normal, Roughness, Metallic, AO)

4. **Download** the ZIP file (~20-50 MB)

5. **Extract** the ZIP to a temporary folder

6. **Rename and copy** files to `assets/materials/polyhaven/metal_plate/`:

   ```
   metal_plate_diff_2k.png    →  albedo.png
   metal_plate_nor_gl_2k.png  →  normal.png
   metal_plate_rough_2k.png   →  roughness.png
   metal_plate_metal_2k.png   →  metallic.png
   metal_plate_ao_2k.png      →  ao.png (optional)
   ```

**Result**: You should have these files:
```
assets/materials/polyhaven/metal_plate/
├── albedo.png      ← Diffuse/color map
├── normal.png      ← Normal map (OpenGL format)
├── roughness.png   ← Roughness map
├── metallic.png    ← Metallic map
└── ao.png          ← Ambient occlusion (optional)
```

---

## Step 2: Update materials.toml (1 min)

Open `assets/materials/polyhaven/materials.toml` and **uncomment** the metallic/ao lines for metal_plate:

```toml
[[layer]]
key = "metal_plate"
albedo = "metal_plate/albedo.png"
normal = "metal_plate/normal.png"
roughness = "metal_plate/roughness.png"
metallic = "metal_plate/metallic.png"     # ← UNCOMMENT THIS
ao = "metal_plate/ao.png"                 # ← UNCOMMENT THIS (if downloaded)
tiling = [1.0, 1.0]
triplanar_scale = 16.0
```

**Save** the file.

---

## Step 3: Test the Material (2 min)

### Option A: Using unified_showcase

```powershell
cargo run -p unified_showcase --release
```

If the example doesn't load PolyHaven materials yet, you'll see fallback rendering.

### Option B: Quick validation script

Create `test_material_load.rs`:

```rust
use astraweave_render::MaterialManager;
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize wgpu (minimal setup)
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .ok_or_else(|| anyhow::anyhow!("No GPU adapter found"))?;
    
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await?;

    // Load PolyHaven materials
    let mut material_manager = MaterialManager::new();
    let stats = material_manager
        .load_biome(&device, &queue, Path::new("assets/materials/polyhaven"))
        .await?;

    println!("✅ Loaded {} materials", stats.layers_loaded);
    println!("   Albedo textures: {}", stats.albedo_count);
    println!("   Normal textures: {}", stats.normal_count);
    println!("   MRA textures: {}", stats.mra_count);

    Ok(())
}
```

Run:
```powershell
cargo run --bin test_material_load --features assets
```

---

## Expected Output

### If Successful ✅

```
✅ Loaded 5 materials
   Albedo textures: 5
   Normal textures: 5
   MRA textures: 5
```

### If Files Missing ⚠️

```
Error: albedo texture not found: assets/materials/polyhaven/metal_plate/albedo.png

Hint: Download from https://polyhaven.com/a/metal_plate
      Follow README.md in the folder
```

---

## Troubleshooting

### Problem: "File not found" error

**Cause**: Textures not downloaded or misnamed

**Solution**:
1. Check file exists: `Test-Path assets/materials/polyhaven/metal_plate/albedo.png`
2. Verify naming (must be exact: `albedo.png`, not `albedo_2k.png`)
3. Check README.md in the folder for correct naming

### Problem: "Invalid TOML syntax" error

**Cause**: Syntax error in materials.toml

**Solution**:
1. Validate: `toml lint assets/materials/polyhaven/materials.toml`
2. Check for missing quotes or brackets
3. Ensure paths use forward slashes: `metal_plate/albedo.png`

### Problem: GPU out of memory

**Cause**: Too many 2K textures loaded

**Solution**:
1. Start with 1-2 materials first
2. Use 1K textures instead of 2K
3. Enable KTX2 compression (Phase 2)

---

## Next Materials to Download

Once metal_plate works, download these (in order of usefulness):

1. **cobblestone** - Good for outdoor scenes
   - https://polyhaven.com/a/cobblestone_floor_01
   
2. **wood_floor** - Good for indoor scenes
   - https://polyhaven.com/a/wood_floor_deck
   
3. **plastered_wall** - Good for buildings
   - https://polyhaven.com/a/plastered_wall
   
4. **aerial_rocks** - Good for natural terrain
   - https://polyhaven.com/a/aerial_rocks_02

**Tip**: All use the same download process - just change the destination folder!

---

## Download a Sample HDRI (Bonus - 3 min)

For environment lighting:

1. **Visit**: https://polyhaven.com/a/kloppenheim_06_puresky

2. **Download**:
   - Resolution: **2K**
   - Format: **HDR**

3. **Rename and copy**:
   ```
   kloppenheim_06_puresky_2k.hdr  →  assets/hdri/polyhaven/kloppenheim/kloppenheim.hdr
   ```

4. **Use in renderer**:
   ```rust
   ibl_manager.load_hdri("assets/hdri/polyhaven/kloppenheim/kloppenheim.hdr")?;
   ```

---

## Summary Checklist

- [ ] Download metal_plate from PolyHaven (2K PNG)
- [ ] Extract and rename to: albedo.png, normal.png, roughness.png, metallic.png
- [ ] Place in: `assets/materials/polyhaven/metal_plate/`
- [ ] Uncomment metallic/ao lines in materials.toml
- [ ] Run renderer to validate
- [ ] See realistic PBR material in engine!

**Total time**: ~5 minutes for first material

**License**: CC0 (Public Domain) - Use anywhere, no attribution required!

---

Need help? Check:
- `assets/materials/polyhaven/metal_plate/README.md` (detailed instructions)
- `docs/root-archive/POLYHAVEN_ASSET_INTEGRATION_ROADMAP.md` (full roadmap)
- `docs/root-archive/POLYHAVEN_PHASE_1_COMPLETE.md` (completion report)
