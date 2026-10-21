# Rendering Debug Quick Reference

**Last Updated**: November 11, 2025  
**Companion Doc**: See `RENDERING_ISSUES_COMPREHENSIVE_ANALYSIS.md` for full details

---

## 🚨 Current Issues Summary

1. **Black Silhouettes** (P0) - 90% of objects render black, SOME faces show texture
2. **Terrain No Texture** (P0) - Smooth color gradients, no detail
3. **No Shadows** (P1) - Expected (not implemented yet)
4. **Flat Normals** (P2) - Using fallback normal maps

**Most Likely Root Cause**: Face culling issue OR texture sampling failure

---

## ⚡ Quick Tests (Copy-Paste Ready)

### Test 1: Disable Culling (5 min)
**File**: `examples/unified_showcase/src/main_bevy_v2.rs` line 1588

```rust
// BEFORE:
cull_mode: Some(wgpu::Face::Back),

// CHANGE TO:
cull_mode: None,
```

**Rebuild**: `cargo build -p unified_showcase --release`  
**Expected**: ALL faces render (if this fixes it, winding order is wrong)

---

### Test 2: Visualize Normals (10 min)
**File**: `examples/unified_showcase/src/pbr_shader.wgsl` line 200

```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Add this line at START of function (before everything else)
    return vec4<f32>(in.world_normal * 0.5 + 0.5, 1.0);
    
    // ... rest of function (will be skipped) ...
}
```

**Expected**: Objects show rainbow colors (red/green/blue gradients)  
**If black/white only**: Normals are broken

---

### Test 3: Visualize UVs (10 min)
**File**: `examples/unified_showcase/src/pbr_shader.wgsl` line 200

```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.uv, 0.0, 1.0); // Red=U, Green=V
    // ... rest of function ...
}
```

**Expected**: Red/green gradient (UV coordinates visible)  
**If solid colors**: UVs not being passed correctly

---

### Test 4: Force White Albedo (10 min)
**File**: `examples/unified_showcase/src/pbr_shader.wgsl` line 183

```wgsl
// Find this line:
albedo = textureSample(albedo_texture, texture_sampler, atlas_uv).rgb;

// REPLACE WITH:
albedo = vec3<f32>(1.0, 1.0, 1.0); // Force white
```

**Expected**: Objects render as WHITE with proper lighting  
**If this works**: Issue is texture sampling, not lighting

---

### Test 5: Sample Raw Atlas (10 min)
**File**: `examples/unified_showcase/src/pbr_shader.wgsl` line 183

```wgsl
// BEFORE:
let atlas_uv = remap_atlas_uv(in.uv, in.material_id);
albedo = textureSample(albedo_texture, texture_sampler, atlas_uv).rgb;

// CHANGE TO:
albedo = textureSample(albedo_texture, texture_sampler, in.uv).rgb; // Skip remapping
```

**Expected**: Objects show TILED atlas texture (7 materials repeated)  
**If this works**: UV remapping calculation is wrong

---

## 🔍 Asset Verification (PowerShell)

### Check Texture Files Exist
```powershell
$textures = @("texture-d.png", "texture-f.png", "cobblestone.png", "planks.png", "texture-j.png", "roof.png", "cobblestonePainted.png")

Add-Type -AssemblyName System.Drawing

foreach ($file in $textures) {
    $path = "assets\textures\$file"
    if (Test-Path $path) {
        $img = [System.Drawing.Image]::FromFile("$(Get-Location)\$path")
        Write-Host "✅ $file`: $($img.Width)×$($img.Height)" -ForegroundColor Green
        
        # Check if predominantly black
        $bitmap = [System.Drawing.Bitmap]::FromFile("$(Get-Location)\$path")
        $pixel = $bitmap.GetPixel($bitmap.Width/2, $bitmap.Height/2)
        $avg = ($pixel.R + $pixel.G + $pixel.B) / 3
        if ($avg -lt 10) {
            Write-Host "   ⚠️  WARNING: Appears BLACK (avg=$avg)" -ForegroundColor Yellow
        }
        
        $img.Dispose()
        $bitmap.Dispose()
    } else {
        Write-Host "❌ $file`: NOT FOUND" -ForegroundColor Red
    }
}
```

---

## 🎯 Test Priority Order

1. **Test 1** (Disable culling) - 5 min - **START HERE** (most likely fix)
2. **Asset Verification** - 5 min - Check texture-j.png exists and not black
3. **Test 4** (Force white) - 10 min - Isolate texture vs lighting issue
4. **Test 2** (Visualize normals) - 10 min - Verify normal directions
5. **Test 5** (Raw atlas) - 10 min - Check if atlas texture valid

**Total Diagnostic Time**: ~40 minutes

---

## 📊 Expected Results

### Test 1 Success (Disable Culling)
- ✅ ALL faces visible (no black areas)
- **Action**: Keep `cull_mode: None` OR flip winding order OR change to `cull_mode: Some(wgpu::Face::Front)`

### Test 1 Failure (Still Black)
- ❌ Black silhouettes remain
- **Action**: Continue to Test 4 (lighting vs texture isolation)

### Test 4 Success (White Albedo)
- ✅ Objects render as white lit surfaces
- **Conclusion**: Texture sampling is failing
- **Action**: Run Asset Verification + Test 5

### Test 4 Failure (Still Black)
- ❌ Even white objects are black
- **Conclusion**: Lighting or normals broken
- **Action**: Run Test 2 (normal visualization)

---

## 🛠️ Common Fixes

### Fix A: Winding Order (if Test 1 succeeds)
```rust
// Option 1: Disable culling permanently (simple but not ideal)
cull_mode: None,

// Option 2: Cull front faces instead (if GLTF winding is inverted)
cull_mode: Some(wgpu::Face::Front),

// Option 3: Change winding definition
front_face: wgpu::FrontFace::Cw,  // Was: Ccw
```

### Fix B: Invert Normals (if Test 2 shows wrong directions)
```wgsl
// pbr_shader.wgsl line 71
out.world_normal = -normalize((uniforms.model * vec4<f32>(in.normal, 0.0)).xyz);
```

### Fix C: Replace Missing Textures (if Asset Verification fails)
```powershell
# Copy known-good texture to texture-j.png
Copy-Item "assets\textures\planks.png" "assets\textures\texture-j.png"
```

---

## 🔧 File Locations Reference

- **Pipeline Settings**: `main_bevy_v2.rs` lines 1565-1610
- **Shader**: `pbr_shader.wgsl` lines 155-205
- **GLTF Loader**: `gltf_loader.rs` lines 108-200
- **Material Definitions**: `main_bevy_v2.rs` lines 838-883
- **Atlas Creation**: `main_bevy_v2.rs` lines 1270-1340
- **Draw Calls**: `main_bevy_v2.rs` lines 2430-2520

---

## ⏱️ Iteration Workflow

1. Make ONE change (e.g., Test 1)
2. Rebuild: `cargo build -p unified_showcase --release` (~40s)
3. Run: `.\target\release\unified_showcase.exe`
4. Take screenshot
5. Compare with baseline
6. If not fixed, REVERT change and try next test
7. If fixed, keep change and move to next issue

**Critical**: Only change ONE thing at a time to isolate root cause!

---

## 📝 Notes Template

Use this for tracking test results:

```
Test 1 (Disable Culling):
- Changed: cull_mode: None
- Result: [BLACK STILL | FACES VISIBLE]
- Screenshot: [filename]
- Conclusion: [...]

Test 2 (Visualize Normals):
- Changed: [...]
- Result: [...]
- Conclusion: [...]
```

---

**Next Steps**: Start with Test 1 (disable culling), report results before proceeding.
