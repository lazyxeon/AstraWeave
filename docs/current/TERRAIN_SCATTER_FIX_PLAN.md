# Terrain & Scatter Rendering Fix Plan

**Version**: 1.0.0  
**Created**: 2026-04-08  
**Context**: AI analysis of editor viewport video recording cross-referenced against codebase  
**Priority**: Production-critical — fixes are ordered by visual impact and implementation cost

---

## Executive Summary

A video analysis of the editor viewport revealed 5 categories of issues: floating vegetation, slope filter failures, visual fidelity gaps, chunk seaming artifacts, and performance regressions. After deep cross-referencing against the actual codebase, findings fall into three categories:

| Category | Count | Root Cause |
|----------|-------|------------|
| **Real bugs** — code exists but doesn't work correctly | 3 | Missing integration, dead code paths |
| **Missing features** — capability exists in engine but isn't wired to editor | 5 | EditorDefault quality preset too conservative |
| **Genuine gaps** — no implementation exists | 4 | Never built: normal alignment, pivot adjust, scatter LOD |

---

## Issue-by-Issue Analysis

### Issue 1: Trees Floating Above Ground (CRITICAL)

**Video observation**: Canopies float 1–3m above rock; some trees are just green discs with no trunk; on slopes one side clips while the other floats.

**Codebase reality**: **CONFIRMED — real bug with three compounding causes.**

#### Cause A: No model pivot adjustment
- **File**: `tools/aw_editor/src/viewport/engine_adapter.rs` → `upload_scatter_placements()`
- **Code**: Transform is `Mat4::from_scale_rotation_translation(Vec3::splat(p.scale), Quat::from_rotation_y(p.rotation), p.position)`
- **Problem**: The glTF models from KIT/Nature Pack have their origin at the **center of the bounding box** (or center of the canopy), NOT at the base of the trunk. When positioned at terrain height, the model center sits at ground level, causing the canopy to float and the trunk to appear partially buried or invisible.
- **Evidence**: The `sink_factor` hack (`pos.y -= sink_factor * world_scale`) in `from_vegetation_instance()` partially compensates but is insufficient and produces inconsistent results across model geometries.

#### Cause B: Heightmap-vs-rendered-mesh height mismatch
- **File**: `astraweave-terrain/src/scatter.rs` → line 167–169
- **Code**: `world_pos.y = chunk.get_height_at_world_pos(...)` uses bilinear interpolation on the heightmap grid
- **Problem**: The rendered terrain mesh uses triangle rasterization, NOT bilinear interpolation. At terrain cell boundaries and on steep slopes, bilinear and triangle interpolation diverge by up to 0.5–2.0m, causing scatter objects to float above or sink through the rendered surface.

#### Cause C: No surface normal alignment
- **File**: `astraweave-terrain/src/scatter.rs` → `VegetationInstance.terrain_normal`
- **Code**: Normal is calculated and stored but **never applied** to object rotation
- **File**: `tools/aw_editor/src/viewport/engine_adapter.rs` → `upload_scatter_placements()`
- **Code**: `Quat::from_rotation_y(p.rotation)` — Y-axis rotation ONLY
- **Problem**: On slopes, trees remain perfectly vertical (aligned to world Y) while the ground tilts away underneath them, causing one side to float and the other to clip.

#### Fix Plan

| Step | File | Change | Effort |
|------|------|--------|--------|
| 1a | `engine_adapter.rs` | **Compute model AABB on glTF load**, calculate Y-offset as `aabb.min.y * scale` to place base at ground level instead of center. Cache per mesh_key. | Medium |
| 1b | `engine_adapter.rs` | **Apply normal-alignment rotation**: `Quat::from_rotation_arc(Vec3::Y, terrain_normal) * Quat::from_rotation_y(rotation)` | Small |
| 1c | `scatter.rs` | **Replace bilinear height lookup with barycentric interpolation** matching the rendered mesh triangulation (use `(cell_x + cell_z) % 2` to determine triangle split direction, then compute barycentric coords within the correct tri). Alternatively, add a small downward offset bias calibrated to match mesh triangulation. | Medium |
| 1d | `terrain_integration.rs` | **Remove the `sink_factor` hack** — it will be superseded by proper pivot adjustment (1a) and accurate height lookup (1c). | Small |

**Confidence**: 95% — all three causes identified in code with clear fix paths.

---

### Issue 2: Trees on Steep Cliffs (HIGH)

**Video observation**: Trees on 70–80° cliff faces and spire tips.

**Codebase reality**: **PARTIALLY CONFIRMED — code exists but may be miscalibrated.**

- **Slope filter EXISTS**: `scatter.rs` line 173–175 checks `slope > self.config.max_slope` (default 35°)
- **Per-type slope tolerance EXISTS**: `biome.rs` line 305–309 filters by `veg_type.slope_tolerance`
- **BUT**: The slope calculation uses a 1.0m sample distance for gradient estimation. On sharp terrain features (spires, ridges), a 1m offset may land on an adjacent face with a completely different slope, producing inaccurate low slope readings.
- **AND**: The analysis mentions trees at 70–80° — this could indicate the slope filter is being bypassed or miscalculated, OR the max_slope/slope_tolerance values are too permissive for specific vegetation types.

#### Fix Plan

| Step | File | Change | Effort |
|------|------|--------|--------|
| 2a | `scatter.rs` | **Multi-scale slope estimation**: Sample at 0.5m AND 2.0m offsets, take the **maximum** slope reading. This catches both micro-features (0.5m catches local spikes) and macro-features (2.0m catches broad cliff faces). | Small |
| 2b | `scatter.rs` | **Add curvature filter**: Compute Laplacian curvature (sum of second-order height differences). Reject placements where curvature exceeds threshold — prevents placement on ridge tips and spire peaks. | Medium |
| 2c | `biome.rs` | **Audit slope_tolerance values**: Verify tree types have slope_tolerance ≤ 30°, reduce from current 25° if needed. Add per-biome overrides for mountain biome (more restrictive). | Small |
| 2d | `scatter.rs` | **Add altitude ceiling filter**: Reject placement above `max_height * 0.9` relative to chunk/world height range. Prevents trees on improbable mountain peaks. | Small |

**Confidence**: 85% — slope filter exists and should work, but calibration and multi-scale estimation will fix edge cases.

---

### Issue 3: Placeholder Visual Quality (HIGH)

**Video observation**: Flat white sky, no shadows, no AO, emissive green canopies, no normal maps, single stretched rock texture, no scale variation control.

**Codebase reality**: **CONFIRMED — features exist in engine but are disabled/unconfigured in editor.**

The rendering engine has production-ready systems that are simply turned off in `EditorDefault` quality preset:

| Feature | Engine Status | Editor Status | Gap |
|---------|--------------|---------------|-----|
| Cascaded shadows (4-cascade CSM) | ✅ Implemented, 2048px | ❌ OFF in EditorDefault | Configuration |
| IBL/Environment lighting | ✅ Bruneton atmosphere + cubemap IBL | ❌ Not wired to editor viewport | Integration |
| Skybox | ✅ HDR equirect + procedural sky | ❌ Not activated (white background) | Configuration |
| Terrain triplanar projection | ✅ Code in `texture_splatting.rs` | ⚠️ TOML config may not enable it | Configuration |
| Terrain multi-layer splatting | ✅ 8-layer system with slope rules | ⚠️ May be using single-layer fallback | Configuration |
| Normal maps | ✅ Material system supports them | ⚠️ May not be loading terrain normal maps | Asset pipeline |
| PBR materials for vegetation | ✅ Full material pipeline | ❌ Vegetation uses flat tint colors | Integration |

#### Fix Plan

| Step | File | Change | Effort |
|------|------|--------|--------|
| 3a | `engine_adapter.rs` | **Add "EditorTerrain" quality preset**: Shadows ON (2 cascades for perf), SSAO ON, Bloom ON. Apply when terrain is loaded. | Small |
| 3b | `engine_adapter.rs` | **Activate skybox/atmosphere**: Call `renderer.set_environment_map()` or enable procedural sky when terrain loaded. Eliminates white background. | Small |
| 3c | `engine_adapter.rs` | **Enable fog for terrain edge**: Already partially done with ground-plane fog config, but verify it renders. Fade terrain edges instead of hard cutoff. | Small |
| 3d | `terrain_integration.rs` | **Enable triplanar projection**: Ensure terrain material config passes `triplanar_slope_threshold` and `triplanar_blend_power` correctly to GPU material. | Medium |
| 3e | `terrain_integration.rs` | **Multi-layer terrain materials**: Configure at least 3 splat layers (grass/dirt on gentle slopes, rock on steep, snow at altitude). Generate proper splat map weights from slope+altitude. | Medium |
| 3f | `engine_adapter.rs` | **Scatter material improvement**: When loading glTF for scatter, extract embedded PBR textures and use them. Fall back to tint only when no textures exist. Many KIT models include albedo textures. | Medium |
| 3g | `scatter.rs`/`biome.rs` | **Tighten scale variation**: Audit `scale_range` values. Ensure trees use (0.85, 1.15) not a wider range. Add 2–3 canopy hue variants by randomizing tint HSV ±10°. | Small |

**Confidence**: 90% — all systems exist, primarily a wiring/configuration issue.

---

### Issue 4: Chunk Seaming & Edge Artifacts (MEDIUM)

**Video observation**: Faceted edges on peaks, visible seams between chunks with different shading, terrain ends in sharp drop to white.

**Codebase reality**: **PARTIALLY CONFIRMED — architecture relies on procedural seamlessness without geometric guarantees.**

- **No shared edge vertices**: Chunks generate independently. Seamlessness relies on identical noise functions producing matching edge heights.
- **Normals computed per-chunk**: Central-difference normal computation at chunk edges uses clamped values instead of neighboring chunk data, causing visible lighting seams.
- **LOD system exists** (`lod_manager.rs`, `lod_blending.rs`) but is NOT used in the editor — all 121 chunks rendered at full resolution.
- **Terrain boundary**: Ground plane placed 5 units below with fog, but fog may not be rendering in EditorDefault preset.

#### Fix Plan

| Step | File | Change | Effort |
|------|------|--------|--------|
| 4a | `terrain_integration.rs` | **Cross-chunk normal stitching**: When generating chunk meshes, fetch edge heights from adjacent chunks and use them for normal computation on boundary vertices. This eliminates the lighting seam. | Medium |
| 4b | `terrain_integration.rs` | **Force identical edge heights**: After chunk generation, enforce that `chunk[x][edge] == neighbor[x][0]` by averaging. Prevents micro-gaps from floating point noise divergence. | Small |
| 4c | `engine_adapter.rs` | **Add terrain skirts**: Extend each chunk border downward by 2–5 units. Hides any sub-pixel gaps between chunks. Low cost since it's a single strip of triangles per chunk edge. | Medium |
| 4d | `engine_adapter.rs` | **Verify fog renders in editor**: Ensure distance fog is active and properly configured to fade terrain edges. The white background cutoff is likely because fog isn't rendering. Tied to Fix 3b (skybox). | Small |

**Confidence**: 80% — procedural seamlessness mostly works but edge normals are a known gap.

---

### Issue 5: Performance Regression (MEDIUM-HIGH)

**Video observation**: FPS drops 33→18 when looking at dense scatter patches. Frame time spikes to 55ms. Entities: 0. Memory locked at 786MB.

**Codebase reality**: **CONFIRMED — multiple performance issues identified.**

#### Cause A: No frustum culling on scatter objects
- `upload_scatter_placements()` uploads ALL scatter instances to GPU — no per-frame visibility testing
- Camera facing away from a grove still renders all trees in that grove
- **Evidence**: FPS drops correlate with looking AT dense areas (more visible overdraw)

#### Cause B: No LOD for scatter
- Every tree renders at full mesh detail regardless of distance
- Distant trees (>100m) still use the same vertex count as near trees
- No impostor/billboard fallback exists

#### Cause C: All 121 chunks merged into single mesh
- Single draw call is good for CPU, but no frustum culling is possible on individual chunks
- Camera facing away from 90% of the terrain still renders the full merged mesh
- No terrain LOD in editor despite `lod_manager.rs` being fully implemented

#### Cause D: "Entities: 0" explanation
- Scatter objects are uploaded directly to the renderer via `add_model()`, bypassing the ECS entity system
- The entity counter reads `World::entities().len()` which only counts ECS-managed entities
- Scatter exists purely as GPU draw data — no scene graph node, no transform component, no culling

#### Fix Plan

| Step | File | Change | Effort |
|------|------|--------|--------|
| 5a | `engine_adapter.rs` | **Per-group AABB culling**: Compute bounding box for each scatter mesh group. Before each frame, test group AABB against camera frustum. Skip draw call for culled groups. Quick win. | Medium |
| 5b | `engine_adapter.rs` | **Distance-based scatter LOD**: Sort scatter instances by distance from camera. For distance >100m, skip rendering (cull). For distance 50–100m, apply reduced instance count (thin every 2nd instance). Full detail <50m. | Medium |
| 5c | `engine_adapter.rs` | **Split terrain into chunk groups**: Instead of merging all 121 chunks into one mesh, merge into ~16 spatial clusters (4×4 grid). Each cluster gets its own draw call + AABB for frustum culling. Massive win when camera doesn't see 75% of terrain. | Large |
| 5d | `engine_adapter.rs` | **Report scatter stats correctly**: Add `scatter_instance_count` to performance overlay. Show GPU draw calls from renderer stats. Fix "Entities: 0" to show meaningful data (either total instances or total draw calls). | Small |
| 5e | `engine_adapter.rs` | **Disable overdraw-heavy transparency**: If tree canopy models use alpha blending, switch to alpha-test (discard fragments below 0.5 alpha). Eliminates costly sorted transparency and overdraw. | Small |

**Confidence**: 90% — performance issues are well-understood and have clear solutions in the existing rendering infrastructure.

---

## Implementation Phases

### Phase 1: Ground Truth (1–2 days) ← DO THIS FIRST
**Goal**: Trees touch the ground, don't appear on impossible slopes.

| Task | Issue | Steps |
|------|-------|-------|
| Pivot adjustment | #1 | 1a |
| Normal alignment | #1 | 1b |
| Height accuracy | #1 | 1c, 1d |
| Multi-scale slope | #2 | 2a, 2c |
| Curvature filter | #2 | 2b, 2d |

**Validation**: Generate terrain → verify trees are grounded on flat areas, properly tilted on mild slopes, absent from steep cliffs and peaks. Take screenshots for before/after comparison.

### Phase 2: Visual Fidelity (1–2 days)
**Goal**: Terrain looks like a real landscape, not a placeholder.

| Task | Issue | Steps |
|------|-------|-------|
| Quality preset | #3 | 3a |
| Sky + atmosphere | #3 | 3b |
| Fog at edges | #3, #4 | 3c, 4d |
| Terrain materials | #3 | 3d, 3e |
| Scatter materials | #3 | 3f, 3g |
| Chunk normals | #4 | 4a, 4b |

**Validation**: Terrain should have visible shadows, atmospheric depth, multi-material slopes, and natural-looking vegetation with color variation. No white background.

### Phase 3: Performance (1–2 days)
**Goal**: Consistent 60+ FPS in editor viewport.

| Task | Issue | Steps |
|------|-------|-------|
| Group culling | #5 | 5a |
| Scatter LOD | #5 | 5b |
| Chunk splitting | #5 | 5c |
| Stats overlay | #5 | 5d |
| Alpha optimization | #5 | 5e |
| Terrain skirts | #4 | 4c |

**Validation**: FPS should stay above 60 when panning across terrain. GPU profiler should show reduced draw calls and triangle counts when camera faces away from terrain/scatter.

---

## Files Modified (Anticipated)

| File | Phases | Changes |
|------|--------|---------|
| `astraweave-terrain/src/scatter.rs` | 1, 2 | Multi-scale slope, curvature filter, height accuracy |
| `astraweave-terrain/src/biome.rs` | 1, 2 | Slope tolerance audit, altitude ceiling |
| `tools/aw_editor/src/viewport/engine_adapter.rs` | 1, 2, 3 | Pivot adjust, normal align, quality preset, materials, culling, LOD |
| `tools/aw_editor/src/terrain_integration.rs` | 1, 2, 3 | Remove sink hack, cross-chunk normals, edge height enforcement, terrain materials |
| `tools/aw_editor/src/viewport/toolbar.rs` | 3 | Stats overlay improvements |

---

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| AABB pivot computation wrong for some models | Medium | Test with multiple KIT assets; log AABB min/max for verification |
| Normal alignment makes trees look "drunk" on gentle slopes | Medium | Only apply when slope > 5° threshold; interpolate alignment factor |
| Multi-scale slope rejects too many placements | Low | Test density before/after; adjust thresholds by biome |
| Chunk splitting breaks terrain painting/brushes | Medium | Ensure brush system uses chunk indices not merged mesh indices |
| Shadow performance in editor | Medium | Use only 2 cascades at 1024px in editor; full quality is GameQuality preset |

---

## Metrics for Success

| Metric | Current | Target |
|--------|---------|--------|
| Trees touching ground | ~40% | 99%+ |
| Trees on slopes >45° | Many | Zero |
| FPS (dense scatter view) | 18–33 | 60+ |
| Visible chunk seams | Yes | No |
| Sky/atmosphere | White void | Procedural sky with fog |
| Shadow coverage | None | Directional + CSM |
| Terrain texture stretching | Visible on cliffs | Eliminated (triplanar) |

---

**Revision History**

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-04-08 | Initial plan from video analysis cross-reference |
