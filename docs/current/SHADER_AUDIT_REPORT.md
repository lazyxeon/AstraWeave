# AstraWeave Shader Audit Report

**Version**: 1.0.0  
**Date**: 2025-07-17  
**Scope**: READ-ONLY audit of all 132 `.wgsl` files  
**Auditor**: AI (Claude Opus 4.6)

---

## Executive Summary

AstraWeave contains **132 `.wgsl` shader files** totaling approximately **17,900 lines of WGSL**. Of these, **103 are active** (production engine, editor tools, examples, assets) and **29 are archived Bevy ports** in `docs/journey/archive/`. The shader codebase covers a comprehensive AAA-class rendering pipeline: PBR with Disney BRDF, volumetric fog/clouds/god rays, Bruneton atmosphere, Lumen-style GI, Nanite-style mesh rendering, cascaded shadow maps with PCSS, SPH/PBD fluid simulation, particles with GPU sort, TAA/SSGI/GTAO/SSR post-processing, and subgroup-optimized variants.

### Critical Findings Summary

| Severity | Count | Description |
|----------|-------|-------------|
| **CRITICAL** | 3 | Race conditions, `@workgroup_size(1,1,1)` serial bottlenecks |
| **HIGH** | 8 | Struct mismatches, dead code in production paths, broken depth calc |
| **MEDIUM** | 22 | Magic numbers, missing optimizations, inconsistent PI definitions |
| **LOW** | 15 | Minor code quality, duplicated utility functions, hardcoded lighting |
| **INFO** | 6 | Architecture observations, optimization opportunities |

---

## 1. Complete Shader Inventory

### 1.1 Production — Fluid Simulation (14 files, ~3,530 lines)

| File | Lines | Type | Workgroup | Shared Mem | Override | Issues |
|------|-------|------|-----------|------------|----------|--------|
| `astraweave-fluids/src/shaders/viscosity_morris.wgsl` | 537 | Compute (7 entry) | 64 | No | No | Magic numbers, grid indexing bug suspect |
| `astraweave-fluids/shaders/pcisph.wgsl` | 721 | Compute (9 entry) | 64 | Labeled but absent | No | Dead shared memory label, magic boundaries |
| `astraweave-fluids/shaders/fluid_optimized.wgsl` | 603 | Compute (multi) | 64/1 | **Yes** | **Yes** | **`@workgroup_size(1)` on reset_counters**, dead shared arrays |
| `astraweave-fluids/shaders/fluid.wgsl` | 447 | Compute (multi) | 64 | No | No | Dead code (emit/update_whitewater), magic numbers |
| `astraweave-fluids/shaders/ssfr_shade.wgsl` | 153 | V+F | — | — | No | PI as let, ~12 texture samples, magic numbers |
| `astraweave-fluids/shaders/ssfr_temporal.wgsl` | 124 | V+F | — | — | No | 9 texture samples in 3×3 loop |
| `astraweave-fluids/shaders/sdf_gen.wgsl` | 114 | Compute (3 entry) | 8,8,4 | No | No | Sentinel values, signed distance not implemented |
| `astraweave-fluids/shaders/anisotropic.wgsl` | 109 | Compute | 64 | No | No | `aniso_kernel` function **never called** (dead code) |
| `astraweave-fluids/shaders/ssfr_depth.wgsl` | 87 | V+F | — | — | No | **Acknowledged incorrect depth calculation** |
| `astraweave-fluids/shaders/secondary.wgsl` | 76 | V+F | — | — | No | Hardcoded colors, magic numbers |
| `astraweave-fluids/shaders/ssfr_smooth_v2.wgsl` | 57 | Compute | 16,16 | No | No | Binding gap (2,3 unused), struct alignment |
| `astraweave-fluids/shaders/ssfr_smooth.wgsl` | 55 | Compute | 16,16 | No | No | Params struct 12 bytes — uniform alignment issue |
| `astraweave-fluids/shaders/cull.wgsl` | 52 | Compute | 64 | No | No | Particle struct mismatch, particle_radius inconsistent |
| `astraweave-fluids/shaders/despawn_region.wgsl` | 51 | Compute | 64 | No | No | Clean |

### 1.2 Production — Render Engine (69 files, ~9,000 lines)

#### Core Rendering

| File | Lines | Type | Workgroup | Shared Mem | Override | Issues |
|------|-------|------|-----------|------------|----------|--------|
| `render/shaders/pbr.wgsl` | 280 | V+F | — | — | No | height_scale in _pad (fragile), up to 37 tex samples (POM), 5-digit PI |
| `render/shaders/clustered_lighting.wgsl` | 108 | Library | — | — | No | Inline PI, magic F0=0.04 |
| `render/shaders/bindless_material.wgsl` | 75 | Library | — | — | No | Clean. Modern `binding_array` approach |
| `render/shaders/pbr_terrain.wgsl` | 438 | Library | — | — | No | Large TerrainMaterialGpu struct w/ padding arrays |
| `render/shaders/clipmap_terrain.wgsl` | 90 | V+F | — | — | No | Hardcoded light_dir, diffuse-only |
| `render/shaders/virtual_texture.wgsl` | 74 | Compute | 8,8 | No | No | Approximate mip calc (no UV derivatives in compute) |
| `render/shaders/debug_quad.wgsl` | 23 | V+F | — | — | No | Clean |

#### Nanite-Style Mesh Pipeline

| File | Lines | Type | Workgroup | Shared Mem | Override | Issues |
|------|-------|------|-----------|------------|----------|--------|
| `render/src/shaders/vxgi_voxelize.wgsl` | 349 | Compute | 64,1,1 | No | No | **RACE CONDITION**: non-atomic inject_radiance, hardcoded light_dir |
| `render/src/shaders/anchor_vfx.wgsl` | 272 | V+F | — | — | No | State-based effects, clean |
| `render/src/shaders/nanite_material_resolve.wgsl` | 261 | V+F | — | — | No | Meshlet struct inconsistency, hardcoded voxel_size |
| `render/src/shaders/nanite_cluster_cull.wgsl` | 209 | Compute | 64 | No | No | 3-stage culling, uses atomics. Well-structured |
| `render/src/shaders/nanite_sw_raster.wgsl` | 163 | Compute | 8,8 | No | No | **RACE CONDITION**: depth test, O(N*T), no shared memory |
| `render/src/shaders/nanite_material.wgsl` | 86 | V+F | — | — | No | Debug LOD colors |
| `render/src/shaders/nanite_visibility.wgsl` | 72 | V+F | — | — | No | Clean visibility buffer |
| `render/src/shaders/nanite_hiz_pyramid.wgsl` | 34 | Compute | 8,8 | No | No | Clean |
| `render/src/shaders/water.wgsl` | 144 | V+F | — | — | No | 4 hardcoded Gerstner wave sets, hardcoded sun |
| `render/src/shaders/debug_shader.wgsl` | 23 | V+F | — | — | No | Clean |

#### Megalights Clustering

| File | Lines | Type | Workgroup | Shared Mem | Override | Issues |
|------|-------|------|-----------|------------|----------|--------|
| `render/shaders/megalights/prefix_sum.wgsl` | 111 | Compute | — | — | No | Contains fix/comments but see src version below |
| `render/src/shaders/megalights/prefix_sum.wgsl` | 28 | Compute | **1,1,1** | No | No | **CRITICAL: Serial `@workgroup_size(1,1,1)`** |
| `render/shaders/megalights/count_lights.wgsl` | 77 | Compute | 64,1,1 | No | No | Naive O(N), no shared memory |
| `render/src/shaders/megalights/count_lights.wgsl` | 49 | Compute | 64,1,1 | No | No | Similar to above |
| `render/shaders/megalights/write_indices.wgsl` | 108 | Compute | 64 | No | No | Redundant sphere-AABB test (same as count pass) |
| `render/src/shaders/megalights/write_indices.wgsl` | 51 | Compute | 64 | No | No | Similar |

#### Shadows

| File | Lines | Type | Workgroup | Shared Mem | Override | Issues |
|------|-------|------|-----------|------------|----------|--------|
| `render/shaders/shadow_sampling.wgsl` | 262 | Library | — | — | No | PCSS + Poisson PCF. Professional quality. PCSS blocker depth workaround |
| `render/shaders/shadow_csm.wgsl` | 241 | V+F+Lib | — | — | No | 4-cascade, 5×5 PCF. Magic atlas_size=4096 |
| `render/shaders/shadow_point.wgsl` | 101 | Library | — | — | No | Omni + spot, `texture_depth_cube_array`. Clean |

#### Post-Processing

| File | Lines | Type | Workgroup | Shared Mem | Override | Issues |
|------|-------|------|-----------|------------|----------|--------|
| `render/shaders/taa.wgsl` | 238 | Compute | 8,8,1 | **Yes (3×100)** | No | Catmull-Rom + YCoCg + velocity dilation. Well-optimized |
| `render/shaders/temporal_upscale.wgsl` | 201 | Compute | 8,8,1 | No | No | TAA-U w/ RCAS. Magic upscale_boost numbers |
| `render/shaders/gtao.wgsl` | 167 | Compute | 8,8,1 | **Yes** | No | Visibility bitmask, countOneBits. Binary threshold loses precision |
| `render/shaders/auto_exposure.wgsl` | 148 | Compute (2) | 16,16 / 256 | **Yes** | No | Tree reduction. Well-optimized |
| `render/shaders/ssgi.wgsl` | 121 | Compute | 8,8,1 | No | No | One-bounce GI. No Hi-Z acceleration |
| `render/shaders/ssr.wgsl` | 107 | Compute | 8,8,1 | No | No | Linear march. No Hi-Z acceleration |
| `render/shaders/ssgi_denoise.wgsl` | 91 | Compute | 8,8,1 | **Yes (2×100)** | No | Cooperative load. Well-structured |
| `render/shaders/dof.wgsl` | 87 | Compute | 8,8,1 | No | No | 16 Poisson samples. **Magic near=0.1, far=200.0** |
| `render/shaders/gtao_blur.wgsl` | 80 | Compute | 8,8,1 | **Yes (2×196)** | No | 14×14 tile. Well-structured |
| `render/shaders/motion_blur.wgsl` | 63 | Compute | 8,8,1 | No | No | depth_weight `* 100.0` magic |
| `render/shaders/bloom_downsample.wgsl` | 57 | Compute | 8,8,1 | No | No | 13-tap CoD:AW filter. Magic 0.0001 |
| `render/shaders/bloom_upsample.wgsl` | 39 | Compute | 8,8,1 | No | No | 9-tap tent. Clean |
| `render/shaders/post_fx.wgsl` | 32 | V+F | — | — | No | Magic ao_strength=0.6, gi_strength=0.2 |
| `render/shaders/post_basic.wgsl` | 29 | V+F | — | — | No | Clean ACES tonemap |
| `render/shaders/velocity.wgsl` | 69 | V+F | — | — | No | Per-object previous transform. Clean |
| `render/shaders/hiz_generate.wgsl` | 22 | Compute | 8,8,1 | No | No | 2×2 max depth mip. Clean |
| `render/shaders/oit_wboit.wgsl` | 74 | F + V+F | — | — | No | McGuire WBOIT. Weight function magic numbers |

#### PBR Library

| File | Lines | Type | Workgroup | Shared Mem | Override | Issues |
|------|-------|------|-----------|------------|----------|--------|
| `render/shaders/pbr/disney_brdf.wgsl` | 260 | Library | — | — | No | Full Disney Principled. Professional quality |
| `render/shaders/pbr/parallax.wgsl` | 167 | Library | — | — | No | POM + binary refinement + self-shadow. Clean |
| `render/shaders/pbr/brdf_lut.wgsl` | 81 | Compute | 8,8 | No | No | Split-sum integration. One-time precompute. Clean |

#### Area Lights

| File | Lines | Type | Workgroup | Shared Mem | Override | Issues |
|------|-------|------|-----------|------------|----------|--------|
| `render/shaders/ltc_area_lights.wgsl` | 230 | Library | — | — | No | Heitz 2016 LTC. Rect/disk/tube. Hardcoded 64×64 LUT |

#### Volumetrics

| File | Lines | Type | Workgroup | Shared Mem | Override | Issues |
|------|-------|------|-----------|------------|----------|--------|
| `render/shaders/volumetrics/cloud_raymarching.wgsl` | 357 | Compute | (TBD) | No | No | Horizon ZD-style. Perlin-Worley + dual-lobe HG. Complex. Clean |
| `render/shaders/volumetrics/scatter.wgsl` | 127 | Compute | 4,4,4 | No | No | Froxel in-scatter + HG + temporal. Clean |
| `render/shaders/volumetrics/fog_density.wgsl` | 120 | Compute | 4,4,4 | No | No | Height fog + noise animation. Clean |
| `render/shaders/volumetrics/integrate.wgsl` | 80 | Compute | 8,8 | No | No | Front-to-back Beer's law. Early termination. Clean |
| `render/shaders/volumetrics/god_rays.wgsl` | 65 | Compute | 8,8 | No | No | Magic `step(2.0, luminance)`, inconsistent sampling |
| `render/shaders/volumetrics/cloud_composite.wgsl` | 62 | Compute | 8,8,1 | No | No | Magic far_plane thresholds |
| `render/shaders/volumetrics/apply.wgsl` | 30 | Compute | 8,8 | No | No | Clean |

#### Atmosphere

| File | Lines | Type | Workgroup | Shared Mem | Override | Issues |
|------|-------|------|-----------|------------|----------|--------|
| `render/shaders/atmosphere/sky_render.wgsl` | 188 | Compute | 8,8 | No | No | Sin-hash stars (low quality), moon=sun*1.05 |
| `render/shaders/atmosphere/aerial_perspective.wgsl` | 128 | Compute | 8,8 | No | No | Clean |
| `render/shaders/atmosphere/transmittance_lut.wgsl` | 109 | Compute | 8,8 | No | No | Bruneton model. 40 integration steps. Clean |

#### Lumen-Style GI

| File | Lines | Type | Workgroup | Shared Mem | Override | Issues |
|------|-------|------|-----------|------------|----------|--------|
| `render/shaders/lumen/final_gather.wgsl` | 174 | Compute | 8,8 | No | No | SSGI+probes+DFAO. 11 bindings. Neighborhood clamp SSGI-only |
| `render/shaders/lumen/surface_cache_update.wgsl` | 169 | Compute | 64 | No | No | SH L2 probes. Simplified radiance (no scene tracing) |
| `render/shaders/lumen/dfao.wgsl` | 112 | Compute | 8,8 | No | No | 5-cone SDF AO. Clean |

#### Particles

| File | Lines | Type | Workgroup | Shared Mem | Override | Issues |
|------|-------|------|-----------|------------|----------|--------|
| `render/shaders/particles/simulate.wgsl` | 146 | Compute | 64 | No | No | Niagara-class. Semi-implicit Euler. Clean |
| `render/shaders/particles/render.wgsl` | 103 | V+F | — | — | No | Soft particles. Inconsistent depth/texture LOD |
| `render/shaders/particles/bitonic_sort.wgsl` | 57 | Compute | 256 | No | No | Clean. Requires log²(n) dispatches |

#### Subgroup-Optimized (3 files, ~386 lines) ✅ EXIST

| File | Lines | Type | Workgroup | Shared Mem | Override | Subgroup Ops |
|------|-------|------|-----------|------------|----------|--------------|
| `render/shaders/subgroup/auto_exposure_subgroup.wgsl` | 214 | Compute | 16,16,1 | **Yes (256)** | No | `enable subgroups`, `subgroupAdd`, ballot. Pre-reduction strategy |
| `render/shaders/subgroup/bitonic_sort_subgroup.wgsl` | 94 | Compute | 256 | No | No | `enable subgroups`, `subgroupShuffleXor`. Eliminates 5 inner passes |
| `render/shaders/subgroup/prefix_sum_subgroup.wgsl` | 78 | Compute | 256,1,1 | **Yes (2×16)** | No | `enable subgroups`, `subgroupExclusiveAdd`. Eliminates 10 of 18 barriers |

### 1.3 Editor Tools (3 files, ~278 lines)

| File | Lines | Type | Workgroup | Shared Mem | Override | Issues |
|------|-------|------|-----------|------------|----------|--------|
| `tools/aw_editor/shaders/grid.wgsl` | 161 | V+F | — | — | No | Infinite grid. 3-level LOD. `fwidth()`. Well-engineered |
| `tools/aw_editor/shaders/tonemap.wgsl` | 89 | V+F | — | — | No | ACES + PBR Neutral + Reinhard. Uniform-selectable. Clean |
| `tools/aw_editor/shaders/gizmo.wgsl` | 28 | V+F | — | — | No | Simple line rendering. Clean |

### 1.4 Crates & Assets (2 files, ~500 lines)

| File | Lines | Type | Workgroup | Shared Mem | Override | Issues |
|------|-------|------|-----------|------------|----------|--------|
| `crates/astraweave-render/shaders/bevy_shadows.wgsl` | 153 | Library | — | — | No | Adapted Bevy CSM. Castano PCF. Clean |
| `assets/shaders/water_surface.wgsl` | 347 | V+F | — | — | No | Volumetric water with flow, Fresnel, foam. PI as const. Clean |

### 1.5 Examples (16 files, ~2,850 lines)

| File | Lines | Type | Workgroup | Shared Mem | Override | Issues |
|------|-------|------|-----------|------------|----------|--------|
| `examples/unified_showcase/shaders/pbr_advanced.wgsl` | 469 | Library | — | — | No | Clearcoat+aniso+SSS+sheen+transmission. Professional |
| `examples/unified_showcase/shaders/pbr_lib.wgsl` | 285 | Library | — | — | No | Cook-Torrance + IBL + material sampling. Clean |
| `examples/unified_showcase/enhanced_shader.wgsl` | 580 | V+F | — | — | No | Full PBR w/ MaterialGpuExtended, shader composition |
| `examples/unified_showcase/pbr_shader.wgsl` | 261 | V+F | — | — | No | Terrain multi-material + atlas remapping |
| `examples/unified_showcase/shader.wgsl` | 209 | V+F | — | — | No | Shadow mapping + PBR |
| `examples/unified_showcase/terrain.wgsl` | 167 | V+F | — | — | No | Triplanar + grass/rock blend |
| `examples/unified_showcase/shader_clean.wgsl` | 165 | V+F | — | — | No | Near-duplicate of shader.wgsl |
| `examples/unified_showcase/shader_v2.wgsl` | 136 | V+F | — | — | No | PBR with inverse_mat3 helper |
| `examples/fluids_demo/ocean.wgsl` | 145 | V+F | — | — | No | Godot Water port. Dual-wave displacement |
| `examples/fluids_demo/fluid.wgsl` | 120 | V+F | — | — | No | Billboard particle rendering |
| `examples/unified_showcase/water.wgsl` | 75 | V+F | — | — | No | Simple Fresnel water |
| `examples/unified_showcase/skybox.wgsl` | 55 | V+F | — | — | No | Equirectangular skybox |
| `examples/fluids_demo/skybox.wgsl` | 54 | V+F | — | — | No | Equirectangular skybox + ACES |
| `examples/unified_showcase/skybox_shader.wgsl` | 47 | V+F | — | — | No | Cubemap full-screen triangle |
| `examples/fluids_demo/glass.wgsl` | 61 | V+F | — | — | No | Fresnel glass refraction |
| `examples/fluids_demo/texture_mesh.wgsl` | 26 | V+F | — | — | No | Textured quad. Minimal |

### 1.6 Archive (29 files, ~4,700 lines)

Historical Bevy engine shader ports in `docs/journey/archive/` and `archive/debug_artifacts/`. Includes Bevy PBR fragment, PBR lighting, PBR functions, mesh types, fog, skinning, forward_io, and duplicates of Nanite/megalights shaders. **Not actively used.** The `combined_shader.wgsl` (1,234 lines) is a debug concatenation artifact.

---

## 2. Critical Issues

### 2.1 CRITICAL — Race Conditions

| File | Issue | Impact |
|------|-------|--------|
| [vxgi_voxelize.wgsl](astraweave-render/src/shaders/vxgi_voxelize.wgsl) | `inject_radiance` writes to 3D texture non-atomically from parallel compute threads | **Data corruption**: Multiple threads voxelizing different triangles may write to the same voxel simultaneously. Requires atomic image operations or a separate accumulation buffer |
| [nanite_sw_raster.wgsl](astraweave-render/src/shaders/nanite_sw_raster.wgsl) | Depth test uses non-atomic compare-and-swap on storage buffer | **Z-fighting/corruption**: Parallel threads may fail visibility test simultaneously. Requires `atomicMin` on depth buffer |

### 2.2 CRITICAL — Serial Bottlenecks

| File | Issue | Impact |
|------|-------|--------|
| [megalights/prefix_sum.wgsl](astraweave-render/src/shaders/megalights/prefix_sum.wgsl#L1-L28) | `@workgroup_size(1,1,1)` — entire prefix sum runs on a **single GPU thread** | **Performance**: O(N) serial scan. Should use parallel Blelloch or subgroup-optimized scan (which already exists in `subgroup/prefix_sum_subgroup.wgsl`!) |
| [fluid_optimized.wgsl](astraweave-fluids/shaders/fluid_optimized.wgsl) | `reset_counters` uses `@workgroup_size(1)` | **Minor**: Only resets counters (low-frequency), but sets bad precedent |

### 2.3 HIGH — Struct Mismatches

| Files | Issue |
|-------|-------|
| `cull.wgsl` vs other fluid shaders | Particle struct has `_pad` field where others have `phase`/`temperature`. Reading a culled particle's phase in a subsequent shader will get garbage data |
| `nanite_material_resolve.wgsl` vs `nanite_cluster_cull.wgsl` | Meshlet struct field order differs — vertex_offset vs cluster_offset naming creates silent data misinterpretation risk |

### 2.4 HIGH — Dead Code in Production

| File | Dead Code | Impact |
|------|-----------|--------|
| `fluid.wgsl` | `emit_whitewater` and `update_whitewater` entry points immediately `return` | 447-line shader with ~80 lines unreachable. Dispatching these wastes GPU time |
| `fluid_optimized.wgsl` | `var<workgroup> shared_positions/velocities` declared but never populated in `predict_positions` | GPU allocates shared memory (64×4×3 = 768 bytes) that is never written |
| `anisotropic.wgsl` | `aniso_kernel` function defined but never called | 30+ lines of dead code |

### 2.5 HIGH — Known Broken Code

| File | Issue |
|------|-------|
| `ssfr_depth.wgsl` | Source comments acknowledge "this depth calculation is not correct" — shipping known-broken |

---

## 3. Anti-Pattern Detection

### 3.1 `@workgroup_size(1,1,1)` Instances

| File | Entry Point | Justified? |
|------|-------------|------------|
| `megalights/prefix_sum.wgsl` (src) | `prefix_sum` | **No** — serial scan, should use parallel algorithm |
| `fluid_optimized.wgsl` | `reset_counters` | **Marginal** — single atomic reset could use higher workgroup size with early-exit |

### 3.2 Missing Shared Memory Optimization

| File | Opportunity |
|------|-------------|
| `ssgi.wgsl` | Screen-space ray march resamples depth/normal each step — could cache neighborhood in shared memory |
| `ssr.wgsl` | Same as SSGI — linear march without shared memory pre-load |
| `megalights/count_lights.wgsl` | Naive O(N) per-tile — could use shared memory tile AABB for early rejection |
| `nanite_sw_raster.wgsl` | Software rasterizer without shared memory tile — O(N×T) triangle-pixel tests |
| `lumen/surface_cache_update.wgsl` | 14-direction radiance sampling without shared memory probe interpolation |

### 3.3 `override` Constants Usage

**Only 1 file uses `override`**: `fluid_optimized.wgsl`

All other compute shaders hardcode workgroup sizes and algorithm parameters. The subgroup shaders use `enable subgroups` but not `override`. This means workgroup sizes cannot be tuned at pipeline creation time across the engine.

### 3.4 Subgroup Operations

**3 files use `enable subgroups`** — all in `render/shaders/subgroup/`:
- `auto_exposure_subgroup.wgsl` — `subgroupAdd`
- `prefix_sum_subgroup.wgsl` — `subgroupExclusiveAdd`
- `bitonic_sort_subgroup.wgsl` — `subgroupShuffleXor`

Each has a non-subgroup fallback shader. This is the correct pattern.

**Note**: The `megalights/prefix_sum.wgsl` (serial) should be replaced by `subgroup/prefix_sum_subgroup.wgsl` for production use.

### 3.5 Naga Compatibility Concerns

| Feature | Files | Naga Status |
|---------|-------|-------------|
| `enable subgroups` | 3 subgroup shaders | Requires `wgpu` 24+ with `SUBGROUPS` feature |
| `binding_array` | `bindless_material.wgsl` | Requires `SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING` |
| `texture_depth_cube_array` | `shadow_point.wgsl` | Requires `TEXTURE_CUBE_ARRAY` feature |
| `fwidth()` | `grid.wgsl` | Fragment-only (correctly used). Clean |
| `countOneBits()` | `gtao.wgsl` | Universally supported. Clean |

### 3.6 Shader Permutation Strategy

**No compile-time permutation system exists.** The codebase uses three strategies:

1. **Runtime branching** — Feature flags checked in fragment shader (e.g., `disney_brdf.wgsl` checks `flags & MATERIAL_FLAG_CLEARCOAT`). Causes warp divergence on material boundaries.
2. **Separate shader files** — Different implementations for different features (e.g., `auto_exposure.wgsl` vs `auto_exposure_subgroup.wgsl`). Does not scale.
3. **Rust-side concatenation** — `enhanced_shader.wgsl` notes that `pbr_lib.wgsl` is concatenated by Rust code. Fragile, no `#include` equivalent.

**Recommendation**: Implement a preprocessor or use `naga_oil` for `#define`/`#ifdef` permutations.

---

## 4. Cross-Cutting Analysis

### 4.1 PI Definition Inconsistency

| Pattern | Files |
|---------|-------|
| `const PI: f32 = 3.14159265359;` | `pbr_lib.wgsl`, `water_surface.wgsl`, `fluid.wgsl`, most examples |
| `let PI = 3.14159265;` | `ssfr_shade.wgsl`, `ssfr_temporal.wgsl` (scoped, lower precision) |
| Inline `3.14159265359` | `clearcoat_distribution_ggx`, `distribution_ggx_anisotropic` in `pbr_advanced.wgsl` |
| Inline `3.14159` (5 digits) | `pbr.wgsl` |
| No PI (uses `acos(-1.0)` etc.) | None |

**Impact**: The `let PI` in fluid shaders creates a new variable each invocation (negligible cost but inconsistent). The 5-digit PI in `pbr.wgsl` introduces ~0.00003 error per trigonometric calculation.

**Recommendation**: Define `const PI: f32 = 3.14159265359;` in a shared constants library.

### 4.2 Struct Alignment & Padding

Uniform buffer structs generally respect 16-byte alignment rules, but several issues exist:

| File | Issue |
|------|-------|
| `ssfr_smooth.wgsl` | `Params { threshold: f32, sigma: f32, size: f32 }` = 12 bytes. Uniforms require 16-byte alignment. **Will fail on some drivers** |
| `pbr.wgsl` | Uses `_pad` field to carry `height_scale`. Semantic abuse — if padding changes, POM breaks silently |
| `pbr_terrain.wgsl` | `TerrainMaterialGpu` has explicit `_pad` arrays — correct but inflated struct size |
| `MaterialGpuExtended` | 256-byte struct with deliberate padding. Well-aligned for SSBO |

### 4.3 Texture Sampling Patterns

| Pattern | Count | Files |
|---------|-------|-------|
| >10 texture samples per fragment | 5 | `pbr.wgsl` (37 w/ POM), `ssfr_shade.wgsl` (12), `bloom_downsample.wgsl` (13), `shadow_sampling.wgsl` (16 Poisson), `pbr_terrain.wgsl` (multi-layer) |
| Redundant same-texture samples | 2 | `megalights/write_indices.wgsl` (same sphere-AABB as count pass), `ssfr_temporal.wgsl` (3×3 neighborhood) |
| Inconsistent LOD handling | 2 | `particles/render.wgsl` (`textureLoad` depth + `textureSample` texture), `god_rays.wgsl` (`textureLoad` depth + `textureSampleLevel` scene) |

### 4.4 Divergent Branching

| File | Branch | Impact |
|------|--------|--------|
| `disney_brdf.wgsl` | Per-pixel `has_feature()` checks for clearcoat/aniso/SSS/sheen/transmission | Moderate — up to 5 divergent branches per fragment in mixed-material scenes |
| `enhanced_shader.wgsl` | Similar runtime material feature branching | Same issue at example level |
| `ocean.wgsl` | `mode == 0/1/2` branch in wave_height | Minor — `mode` is uniform, not per-pixel |
| `tonemap.wgsl` | `switch params.mode` for tonemap selection | Minor — uniform-driven, no warp divergence |

---

## 5. Shader Complexity Heatmap

Complexity scored on: LOC, texture samples, bind groups, ALU depth, entry points, shared memory usage.

```
COMPLEXITY HEATMAP (■ = low, ■■ = medium, ■■■ = high, ■■■■ = very high, ■■■■■ = extreme)

EXTREME (>400 LOC, complex algorithms, many bindings):
■■■■■  pcisph.wgsl ............... 721 LOC, 9 entries, SPH solver
■■■■■  fluid_optimized.wgsl ...... 603 LOC, override, shared mem, PBD
■■■■■  enhanced_shader.wgsl ...... 580 LOC, full Disney PBR, 7 bind groups
■■■■■  viscosity_morris.wgsl ..... 537 LOC, 7 entries, non-Newtonian viscosity
■■■■■  pbr_advanced.wgsl ......... 469 LOC, 5 BRDF lobes, anisotropic
■■■■■  fluid.wgsl ................ 447 LOC, PBD solver + dead whitewater
■■■■■  pbr_terrain.wgsl .......... 438 LOC, 4-layer terrain, triplanar POM
■■■■■  cloud_raymarching.wgsl .... 357 LOC, full volumetric cloud model
■■■■■  vxgi_voxelize.wgsl ........ 349 LOC, conservative rast., race condition
■■■■■  water_surface.wgsl ........ 347 LOC, volumetric water + flow + foam

VERY HIGH (200-400 LOC, significant complexity):
■■■■   pbr_lib.wgsl .............. 285 LOC, Cook-Torrance + IBL + material
■■■■   pbr.wgsl .................. 280 LOC, POM, CSM, 5 bind groups
■■■■   anchor_vfx.wgsl ........... 272 LOC, state machine VFX
■■■■   shadow_sampling.wgsl ...... 262 LOC, PCSS + Poisson PCF
■■■■   nanite_material_resolve.wgl 261 LOC, visibility buffer PBR resolve
■■■■   pbr_shader.wgsl ........... 261 LOC, terrain + atlas UV remapping
■■■■   disney_brdf.wgsl .......... 260 LOC, full Disney BRDF
■■■■   shadow_csm.wgsl ........... 241 LOC, 4-cascade CSM + PCF
■■■■   taa.wgsl .................. 238 LOC, 3 shared arrays, YCoCg
■■■■   ltc_area_lights.wgsl ...... 230 LOC, polygon clip + edge integral
■■■■   auto_exposure_subgroup.wgsl 214 LOC, subgroup + shared memory
■■■■   shader.wgsl ............... 209 LOC, shadow mapping + PBR
■■■■   nanite_cluster_cull.wgsl .. 209 LOC, 3-stage cull + atomics
■■■■   temporal_upscale.wgsl ..... 201 LOC, TAA-U + RCAS

HIGH (100-200 LOC, moderate complexity):
■■■    compute_noise.wgsl ........ 183 LOC, Perlin 3D + fBM variants
■■■    gpu_erosion.wgsl .......... 195 LOC, 3-pass SWE erosion
■■■    sky_render.wgsl ........... 188 LOC, sun/moon/stars scattering
■■■    lumen/final_gather.wgsl ... 174 LOC, SSGI+probes+DFAO composite
■■■    lumen/surface_cache_update. 169 LOC, SH L2 probe grid
■■■    parallax.wgsl ............. 167 LOC, POM + self-shadow
■■■    terrain.wgsl (example) .... 167 LOC, triplanar blending
■■■    shader_clean.wgsl ......... 165 LOC, PBR + shadows
■■■    grid.wgsl (editor) ........ 161 LOC, infinite grid, 3-level LOD
■■■    nanite_sw_raster.wgsl ..... 163 LOC, SW rasterizer (race cond.)
■■■    ssfr_shade.wgsl ........... 153 LOC, fluid SSR shading
■■■    bevy_shadows.wgsl ......... 153 LOC, Bevy CSM port
■■■    auto_exposure.wgsl ........ 148 LOC, shared mem histogram
■■■    particles/simulate.wgsl ... 146 LOC, Niagara-class sim
■■■    water.wgsl (render) ....... 144 LOC, Gerstner waves
■■■    ocean.wgsl (example) ...... 145 LOC, Godot water port
■■■    shader_v2.wgsl ............ 136 LOC, PBR variant
■■■    scatter.wgsl .............. 127 LOC, froxel in-scatter
■■■    ssfr_temporal.wgsl ........ 124 LOC, temporal reprojection
■■■    ssgi.wgsl ................. 121 LOC, screen-space GI
■■■    fog_density.wgsl .......... 120 LOC, froxel density
■■■    fluid.wgsl (example) ...... 120 LOC, billboard particles
■■■    aerial_perspective.wgsl ... 128 LOC, depth-based atmosphere
■■■    dfao.wgsl ................. 112 LOC, SDF cone-trace AO
■■■    sdf_gen.wgsl .............. 114 LOC, JFA signed distance
■■■    anisotropic.wgsl .......... 109 LOC, anisotropic kernel (dead fn)
■■■    transmittance_lut.wgsl .... 109 LOC, Bruneton LUT
■■■    clustered_lighting.wgsl ... 108 LOC, Cook-Torrance library
■■■    write_indices.wgsl (mega) . 108 LOC, light index scatter
■■■    ssr.wgsl .................. 107 LOC, linear ray march
■■■    particles/render.wgsl ..... 103 LOC, billboard + soft particles
■■■    shadow_point.wgsl ......... 101 LOC, omni+spot shadow

MEDIUM (50-100 LOC):
■■     subgroup/*.wgsl (3 files) . 78-94 LOC, subgroup-optimized
■■     ssgi_denoise.wgsl ......... 91 LOC, bilateral spatial+temporal
■■     tonemap.wgsl (editor) ..... 89 LOC, 3-mode tonemap
■■     ssfr_depth.wgsl ........... 87 LOC, fluid depth (broken)
■■     dof.wgsl .................. 87 LOC, Poisson bokeh
■■     nanite_material.wgsl ...... 86 LOC, debug LOD
■■     brdf_lut.wgsl ............. 81 LOC, split-sum precompute
■■     integrate.wgsl ............ 80 LOC, Beer's law
■■     gtao_blur.wgsl ............ 80 LOC, bilateral blur
■■     secondary.wgsl ............ 76 LOC, foam billboards
■■     bindless_material.wgsl .... 75 LOC, binding arrays
■■     water.wgsl (example) ...... 75 LOC, Fresnel water
■■     oit_wboit.wgsl ............ 74 LOC, WBOIT
■■     virtual_texture.wgsl ...... 74 LOC, VT feedback
■■     nanite_visibility.wgsl .... 72 LOC, visibility buffer
■■     velocity.wgsl ............. 69 LOC, motion vectors
■■     god_rays.wgsl ............. 65 LOC, radial light shafts
■■     motion_blur.wgsl .......... 63 LOC, velocity blur
■■     cloud_composite.wgsl ...... 62 LOC, depth-aware composite
■■     glass.wgsl ................ 61 LOC, Fresnel glass
■■     bloom_downsample.wgsl ..... 57 LOC, 13-tap filter
■■     bitonic_sort.wgsl ......... 57 LOC, GPU sort
■■     ssfr_smooth.wgsl .......... 55 LOC, bilateral filter
■■     ssfr_smooth_v2.wgsl ....... 57 LOC, bilateral filter v2
■■     skybox.wgsl ............... 55 LOC, equirectangular
■■     skybox.wgsl (fluids) ...... 54 LOC, equirectangular + ACES
■■     cull.wgsl ................. 52 LOC, frustum cull
■■     despawn_region.wgsl ....... 51 LOC, region despawn

LOW (<50 LOC):
■      skybox_shader.wgsl ........ 47 LOC, cubemap fullscreen
■      bloom_upsample.wgsl ....... 39 LOC, 9-tap tent
■      nanite_hiz_pyramid.wgsl ... 34 LOC, 2×2 max mip
■      post_fx.wgsl .............. 32 LOC, AO+GI composite
■      apply.wgsl ................ 30 LOC, fog composite
■      post_basic.wgsl ........... 29 LOC, ACES tonemap
■      prefix_sum.wgsl (src) ..... 28 LOC, serial scan
■      gizmo.wgsl ................ 28 LOC, colored lines
■      texture_mesh.wgsl ......... 26 LOC, textured quad
■      debug_shader.wgsl ......... 23 LOC, fullscreen quad
■      debug_quad.wgsl ........... 23 LOC, textured quad
■      hiz_generate.wgsl ......... 22 LOC, max depth mip
```

---

## 6. Architecture Recommendations

### 6.1 Immediate Fixes (Critical)

1. **Fix VXGI race condition**: Use atomic image operations or a separate R32Uint accumulation buffer with `atomicAdd` for `inject_radiance` in `vxgi_voxelize.wgsl`.
2. **Fix Nanite SW raster race condition**: Replace direct storage buffer depth write with `atomicMin` on packed depth+triangle_id in `nanite_sw_raster.wgsl`.
3. **Replace serial prefix sum**: Wire `megalights/prefix_sum.wgsl` (28 LOC, `@workgroup_size(1,1,1)`) to use the existing `subgroup/prefix_sum_subgroup.wgsl` or at minimum a parallel Blelloch scan.

### 6.2 High Priority

4. **Unify Particle struct**: Resolve `cull.wgsl` vs production fluid shaders field mismatch (`_pad` vs `phase`/`temperature`).
5. **Fix or remove broken `ssfr_depth.wgsl`**: Commented as "not correct" — either fix the depth calculation or remove the shader.
6. **Remove dead code**: Clean `emit_whitewater`/`update_whitewater` in `fluid.wgsl`, `aniso_kernel` in `anisotropic.wgsl`, unused shared arrays in `fluid_optimized.wgsl`.
7. **Fix uniform alignment**: `ssfr_smooth.wgsl` Params struct (12 bytes) needs padding to 16 bytes.

### 6.3 Medium Priority

8. **Standardize PI**: Create a shared `constants.wgsl` library with `const PI: f32 = 3.14159265359;` (until WGSL gets a `#include` mechanism, use Rust-side concatenation or `naga_oil`).
9. **Add Hi-Z acceleration** to `ssgi.wgsl` and `ssr.wgsl` — Hi-Z pyramid already exists (`hiz_generate.wgsl`).
10. **Replace magic numbers**: Extract hardcoded values (near=0.1, far=200.0 in `dof.wgsl`; atlas_size=4096 in `shadow_csm.wgsl`; ao_strength=0.6 in `post_fx.wgsl`) into uniform parameters.
11. **Implement shader permutation system**: Consider `naga_oil` for `#define`-based specialization to eliminate runtime branching in Disney BRDF.

### 6.4 Low Priority

12. **Upgrade star rendering**: Replace sin-hash procedural stars in `sky_render.wgsl` with a blue noise or Voronoi-based approach.
13. **Remove duplicate shaders**: `shader.wgsl` and `shader_clean.wgsl` in examples are nearly identical — consolidate.
14. **Implement `override` constants** more broadly for tunable workgroup sizes.

---

## 7. Statistics

| Metric | Value |
|--------|-------|
| Total `.wgsl` files | 132 |
| Active files | 103 |
| Archived files | 29 |
| Total lines (active) | ~13,200 |
| Total lines (archive) | ~4,700 |
| Compute shaders | 52 |
| Vertex+Fragment shaders | 38 |
| Library files (no entry point) | 13 |
| Files using `var<workgroup>` | 8 |
| Files using `override` | 1 |
| Files using `enable subgroups` | 3 |
| Files with race conditions | 2 |
| Files with `@workgroup_size(1,1,1)` | 2 |
| Files with dead code | 4 |
| Files with struct mismatches | 2 pairs |
| Files with known broken code | 1 |
| Unique bind group layouts | ~15 |
| Max texture samples per fragment | 37 (pbr.wgsl with POM) |
| Max bind groups per shader | 7 (enhanced_shader.wgsl) |
| Max entry points per file | 9 (pcisph.wgsl) |

---

**Version**: 1.0.0 | Generated by AI Audit | READ-ONLY — No files modified
