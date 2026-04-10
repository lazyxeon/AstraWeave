# Scatter Rendering & Visual Quality Fix Plan

**Date**: 2026-04-08  
**Status**: ✅ IMPLEMENTED — All phases complete  
**Scope**: Scatter texturing, HDRI/skybox, fog, terrain distance rendering

---

## Executive Summary

Visual audit of the editor viewport revealed **4 critical issues** and **3 moderate issues** affecting terrain scatter rendering, skybox integration, and distance rendering quality. Root cause analysis traced all issues to specific code paths with high confidence.

---

## Issue Analysis

### CRITICAL-1: Scatter Objects Render Without Textures (Dark Blocks)

**Symptoms**: All scatter objects (trees, rocks, bushes) appear as solid dark shapes with no surface texture detail. They look like untextured geometry blocks.

**Root Cause**: The Blender decomposer exports `.glb` files with `export_materials: 'NONE'` — textures are stored as **separate PNG files** in the `textures/` directory, NOT embedded in the GLB. When `mesh_gltf::load_gltf()` calls `prim.material().pbr_metallic_roughness().base_color_texture()`, it returns `None` because no material data exists in the GLB.

**Evidence**:
- `crates/astraweave-blend/src/export_script.rs` line ~950: `export_materials: 'NONE'`
- `astraweave-render/src/mesh_gltf.rs` lines 75-88: `base_color_texture()` returns None → `cpu_meshes[0].albedo_image = None`
- `tools/aw_editor/src/viewport/engine_adapter.rs` lines 1073-1081: `has_texture` is false → `add_model()` called instead of `add_model_with_texture()` → model gets 1×1 white fallback texture
- The tint colors (e.g., `[0.35, 0.55, 0.25]` for trees) multiply with `uMaterial.base_color` (`[0.85, 0.78, 0.72]`) giving very dark final colors: `[0.30, 0.43, 0.18]`

**Fix**: Load textures from the BiomePack JSON `textures` array. For each scatter mesh, look up its BiomePackAsset, find the `diffuse` channel texture file in `root_dir/textures/`, load it as RGBA8, and pass it to `add_model_with_texture()`.

**Files to modify**:
- `tools/aw_editor/src/viewport/engine_adapter.rs` — `upload_scatter_placements()`: accept `&BiomePack` parameter, look up textures per mesh_key
- `tools/aw_editor/src/terrain_integration.rs` — pass BiomePack reference through to scatter upload
- `astraweave-terrain/src/biome_pack.rs` — may need helper to look up texture path by asset name

**Complexity**: Medium  
**Risk**: Low (additive change, fallback to existing behavior if texture not found)

---

### CRITICAL-2: Scatter Tint Colors Are Too Dark

**Symptoms**: Even if textures loaded correctly, the tint color multipliers in the Instance color are far too dark. Vegetation tint `[0.35, 0.55, 0.25]` and rock tint `[0.60, 0.58, 0.55]` are intended as color-shift hints but act as aggressive darkening multipliers.

**Root Cause**: The PBR shader computes `base_color = uMaterial.base_color.rgb × input.color.rgb × tex.rgb`. The Instance color (tint) is multiplied directly, meaning values below 1.0 darken the result. With material base_color at `[0.85, 0.78, 0.72]`, a vegetation tint of `[0.35, 0.55, 0.25]` yields a pre-texture color of only `[0.30, 0.43, 0.18]`.

**Fix**: Change tint values to be closer to `[1.0, 1.0, 1.0]` with subtle color shifts (±15% max). The tint should ADD color variation, not dominate. When real textures are loaded, the texture provides the color — the tint should only provide subtle per-instance variation.

**Proposed tint values**:
- Vegetation: `[0.90, 1.00, 0.85, 1.0]` (slight green bias)
- Rock: `[0.95, 0.93, 0.90, 1.0]` (slight warm bias)
- Structure: `[1.00, 0.98, 0.95, 1.0]` (near neutral)
- Default: `[0.95, 0.95, 0.95, 1.0]` (near white)

**Files to modify**:
- `tools/aw_editor/src/terrain_integration.rs` — `from_vegetation_instance()` and `from_zone_placement()` tint tables

**Complexity**: Low  
**Risk**: Low

---

### CRITICAL-3: Fog Washes to Solid White at Distance

**Symptoms**: Screenshots 4-5 show the scene becoming entirely white/washed out when the camera is far from terrain. The fog color `[0.70, 0.78, 0.90]` (pale sky blue) appears as solid white on screen.

**Root Cause**: `fog_end = extent × 1.2` where extent is the terrain radius. Beyond `fog_end`, everything is 100% fog color. The fog color `[0.70, 0.78, 0.90]` is very bright in linear space — after gamma correction it appears nearly white. The fog should match the SKY color, not be a fixed pale blue.

**Fix**:
1. **Match fog color to sky horizon color**: Pull fog color from `SkyConfig.day_color_horizon` instead of hardcoded blue
2. **Increase fog distances**: Use `fog_start = extent × 0.7` and `fog_end = extent × 2.0` so fog starts later and extends further
3. **Reduce fog density**: Lower `fog_density` for softer blending
4. **Consider height fog**: Ground-level fog (denser below, clearer above) instead of pure distance fog

**Files to modify**:
- `tools/aw_editor/src/viewport/engine_adapter.rs` — fog configuration in `upload_terrain_chunks()`

**Complexity**: Low  
**Risk**: Low

---

### CRITICAL-4: Terrain Rendering Artifacts at Distance (Screenshot 4)

**Symptoms**: Screenshot 4 shows terrain with black/dark patches and white/bright speckled patterns when viewed from far away (camera at X:-1500). The pattern shows aliasing and z-fighting-like artifacts.

**Root Cause**: Multiple contributing factors:
1. **Sub-pixel triangle aliasing**: At extreme distance, terrain triangles become smaller than a pixel, causing temporal noise/speckle
2. **No terrain LOD**: All 121 chunks rendered at full resolution regardless of distance, creating sub-pixel geometry
3. **Fog vs terrain edges**: Fog color doesn't match sky, creating harsh visual discontinuity at terrain boundary
4. **Possible z-fighting**: 4×4 grid cluster boundaries may have overlapping triangles from adjacent chunks

**Fix**: This is primarily a fog/color matching issue (CRITICAL-3 fix will help significantly). For the sub-pixel aliasing, a future LOD system would help, but the immediate fix is better fog blending. The alpha cutoff we added (`if (tex.a < 0.5) { discard; }`) is NOT the cause — terrain uses the 1×1 white texture with alpha=255.

**Complexity**: Low (fog fix) / High (LOD system — future work)  
**Risk**: Low

---

### MODERATE-1: HDRI/Skybox Not Loading from BiomePack

**Symptoms**: Screenshots 4-5 show HDRI names like `qwantani_moonrise_pur...` and `kloppenheim_02_puresk...` in the world panel, but the sky appears as a plain gradient (not an actual HDRI environment map). The sky color transitions are harsh with visible horizon lines.

**Root Cause**: The biome packs have `hdris: []` (empty arrays) in their JSON. The HDRI files shown in the editor are manually loaded by the user. The HDRI loading works (`load_hdri()` calls `bake_environment()`), but the procedural sky gradient is too simplistic for realistic visuals. The equirectangular sky renderer includes a harsh `horizon fade to black` that creates the visible horizon line.

**Fix**:
1. **Verify HDRI loading pipeline**: When HDRIs ARE loaded, ensure the sky renders properly without harsh horizon artifacts
2. **Improve procedural sky fallback**: Soften the gradient transitions, add atmospheric haze near horizon
3. **Auto-select HDRIs from catalog**: Use `HdriCatalog` to auto-select an HDRI based on biome type and time-of-day when terrain is generated

**Files to modify**:
- `astraweave-render/src/environment.rs` — horizon fade in equirectangular shader
- `tools/aw_editor/src/viewport/engine_adapter.rs` — auto-HDRI selection on terrain generation

**Complexity**: Medium  
**Risk**: Low

---

### MODERATE-2: Skybox Preset Dropdown Doesn't Actually Change Sky

**Symptoms**: The "Skybox Preset" dropdown (Clear Sky, Overcast, Sunset, Night, Space, Gradient) in the world panel only changes the preview color swatch — it doesn't modify the actual sky renderer configuration.

**Root Cause**: The dropdown modifies `self.world_skybox_preset` (an index) but no code reads this value to update `SkyConfig` or switch sky modes. It's purely a UI preview.

**Fix**: Wire the preset dropdown to actually update the procedural sky colors via `set_sky_config()` or switch to appropriate HDRIs from the catalog.

**Files to modify**:
- `tools/aw_editor/src/tab_viewer/mod.rs` — wire preset selection to sky config changes
- `tools/aw_editor/src/tab_viewer/sky_colors.rs` — may already have preset color definitions

**Complexity**: Medium  
**Risk**: Low

---

### MODERATE-3: No Mipmap Generation for Scatter Textures

**Symptoms**: Once textures ARE loaded (after CRITICAL-1 fix), scatter objects at distance will appear noisy/shimmering because the GPU texture has `mip_level_count: 1` — no mipmaps.

**Root Cause**: `add_model_with_texture()` creates the texture with `mip_level_count: 1`. Without mipmaps, the GPU's texture sampler cannot properly filter at distance, causing Moiré patterns and aliasing.

**Fix**: Generate mipmaps for scatter textures. Either:
- CPU-side mipmap generation (box filter downsample) before upload
- GPU-side mipmap generation via compute shader or blit chain
- At minimum, set `mip_level_count: N` and generate/upload mip levels

**Files to modify**:
- `astraweave-render/src/renderer.rs` — `add_model_with_texture()` mipmap generation

**Complexity**: Medium  
**Risk**: Low

---

## Implementation Order

| Phase | Fix | Priority | Est. Impact |
|-------|-----|----------|-------------|
| **1** | CRITICAL-1: Load scatter textures from BiomePack | P0 | Scatter becomes visually correct |
| **1** | CRITICAL-2: Fix tint multiplier values | P0 | Scatter brightness corrected |
| **2** | CRITICAL-3: Fix fog color and distances | P0 | Distance rendering fixed |
| **2** | CRITICAL-4: Distance aliasing (via fog fix) | P0 | Terrain edges blend naturally |
| **3** | MODERATE-1: HDRI horizon fix + auto-select | P1 | Sky becomes realistic |
| **3** | MODERATE-2: Wire skybox presets | P1 | UI becomes functional |
| **4** | MODERATE-3: Mipmap generation | P2 | Distant scatter quality |

---

## Files Modified (Full List)

| File | Changes |
|------|---------|
| `tools/aw_editor/src/viewport/engine_adapter.rs` | Scatter: load textures from BiomePack; Fog: fix colors/distances |
| `tools/aw_editor/src/terrain_integration.rs` | Fix tint multipliers; pass BiomePack to scatter upload |
| `astraweave-terrain/src/biome_pack.rs` | Helper to look up texture path by asset name + channel |
| `astraweave-render/src/renderer.rs` | Mipmap generation in `add_model_with_texture()` |
| `astraweave-render/src/environment.rs` | Soften horizon fade in equirect shader |
| `tools/aw_editor/src/tab_viewer/mod.rs` | Wire skybox presets to actual sky config |

---

## Verification Plan

1. Generate terrain with verdant_trail biome → scatter should show textured rocks/trees
2. Generate terrain with Namaqualand biome → scatter should show textured desert vegetation
3. Fly camera far from terrain → fog should blend smoothly to sky color, not white
4. Toggle skybox presets → sky should change visually
5. Load HDRI → sky should show environment map without harsh horizon lines
6. Night preset → stars/moon visible, terrain lit by ambient + moonlight
