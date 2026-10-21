# LOD Viewport Audit Report

## Scope

This report audits the editor viewport rendering loop from terrain/scatter generation through instance upload, LOD selection, draw submission, and final viewport output.

Audited sources:

- `tools/aw_editor/src/terrain_integration.rs`
- `tools/aw_editor/src/viewport/engine_adapter.rs`
- `tools/aw_editor/src/viewport/renderer.rs`
- `tools/aw_editor/src/viewport/widget.rs`
- `tools/aw_editor/src/viewport/toolbar.rs`
- `tools/aw_editor/src/subsystems/scene_stats.rs`
- `tools/aw_editor/src/subsystems/docking_sync.rs`
- `astraweave-terrain/src/scatter.rs`
- `astraweave-render/src/vegetation_lod.rs`
- `astraweave-render/src/vegetation_gpu.rs`
- `astraweave-render/src/renderer.rs`
- `astraweave-render/src/mesh.rs`
- `astraweave-render/src/types.rs`
- `tools/aw_editor/Cargo.toml`

Method: static source audit only. No runtime capture, GPU timestamp trace, PIX/RenderDoc capture, or live editor execution was run in this session.

## Executive Summary

The user-visible symptom is real, and the strongest root cause is in the editor's custom scatter LOD path, not in the shared renderer.

The editor viewport does not use `astraweave-render`'s `VegetationGpuPipeline`. The shared renderer has a compute-driven vegetation path with per-instance LOD and indirect draw support in `astraweave-render/src/vegetation_gpu.rs:1-220` and `astraweave-render/src/vegetation_lod.rs:1-90`, but the viewport instead performs its own CPU-side filtering, LOD selection, density thinning, spatial binning, and named-model upload inside `tools/aw_editor/src/viewport/engine_adapter.rs:1313-1929`.

The primary failure mode is scale-insensitive LOD classification:

- terrain integration aggressively enlarges scatter instances with `PACK_SCALE_BOOST = 8.0` in `tools/aw_editor/src/terrain_integration.rs:2298-2300` and with heuristic multipliers such as `tree => 14.0` in `tools/aw_editor/src/terrain_integration.rs:2554-2562`
- the viewport decides whether a species is a "tree" from the raw mesh AABB height alone with `let is_tree = model_height > 3.0` in `tools/aw_editor/src/viewport/engine_adapter.rs:1470-1474`
- the final per-instance transform applies placement scale only after that decision in `tools/aw_editor/src/viewport/engine_adapter.rs:1530-1557`

That means a visually large world-space tree can still be treated as "small vegetation" and receive the much more aggressive thresholds `100 / 100 / 250` from `tools/aw_editor/src/viewport/engine_adapter.rs:1472-1474`, which pushes it into billboard rendering until the camera is much closer than its apparent size warrants.

Two additional design choices magnify the problem:

- LOD rebucketing only happens after camera movement exceeds `50.0` world units in XZ, not every frame, in `tools/aw_editor/src/viewport/engine_adapter.rs:242-253`
- the editor physically implements only two geometry outcomes for vegetation: full mesh and cross-billboard. `Simplified` is merged into the full-mesh bucket at `tools/aw_editor/src/viewport/engine_adapter.rs:1559-1560`, and `ImpostorCard` is merged into the billboard bucket at `tools/aw_editor/src/viewport/engine_adapter.rs:1563-1564`, which is then always materialized as `generate_cross_billboard(...)` at `tools/aw_editor/src/viewport/engine_adapter.rs:1776-1782`

So the viewport's real vegetation path is effectively:

`full mesh -> cross billboard -> cull`

not the richer shared path documented in `astraweave-render/src/vegetation_lod.rs:1-12`.

## Architecture Map

| Stage | Evidence | Status | Notes |
| --- | --- | --- | --- |
| Terrain chunk generation | `tools/aw_editor/src/terrain_integration.rs:2135-2304` | OK | Terrain integration generates chunk-local scatter placements and applies biome cull-distance overrides. |
| Placement scale injection | `tools/aw_editor/src/terrain_integration.rs:2298-2300`, `tools/aw_editor/src/terrain_integration.rs:2554-2562` | FAIL | World-space scale is inflated here, but later LOD classification does not use the scaled size. |
| Viewport camera update | `tools/aw_editor/src/viewport/engine_adapter.rs:219-255` | WARN | Camera update also owns scatter chunk streaming and LOD refresh policy. |
| Scatter chunk activation | `tools/aw_editor/src/viewport/engine_adapter.rs:1358-1364`, `tools/aw_editor/src/viewport/engine_adapter.rs:1968-1995` | WARN | Editor uses a local 3-chunk load radius and 5-chunk unload radius instead of scene/world-partition abstractions. |
| CPU-side scatter grouping and mesh load | `tools/aw_editor/src/viewport/engine_adapter.rs:1383-1465` | OK | Missing meshes are skipped; they are not replaced with cubes. |
| Species LOD threshold selection | `tools/aw_editor/src/viewport/engine_adapter.rs:1470-1474` | FAIL | Thresholds are chosen from raw mesh AABB height, not final world-space instance size or asset category metadata. |
| Per-instance distance test | `tools/aw_editor/src/viewport/engine_adapter.rs:1505-1514` | FAIL | Distance is measured in the XZ plane only. Vertical camera movement does not affect LOD classification. |
| Actual geometry modes used | `tools/aw_editor/src/viewport/engine_adapter.rs:1559-1564`, `tools/aw_editor/src/viewport/engine_adapter.rs:1776-1782` | FAIL | `Simplified` and `ImpostorCard` are not distinct geometry paths in the editor. |
| Spatial binning and per-quadrant upload | `tools/aw_editor/src/viewport/engine_adapter.rs:1584-1619`, `tools/aw_editor/src/viewport/engine_adapter.rs:1717-1802` | OK | Instances are split into up to 36 bins per species and uploaded as named submodels. |
| Shared renderer mesh conversion | `astraweave-render/src/renderer.rs:4376-4382`, `astraweave-render/src/mesh.rs:7-46`, `astraweave-render/src/types.rs:6-41`, `astraweave-render/src/types.rs:110-119` | OK | Billboards and full meshes both use the canonical P/N/T/UV vertex contract and the normal `InstanceRaw` layout. |
| Named-model draw submission | `astraweave-render/src/renderer.rs:5389`, `astraweave-render/src/renderer.rs:5583-5586`, `astraweave-render/src/renderer.rs:6658-6740` | OK | Renderer draws uploaded named models directly with instanced `draw_indexed`; no fallback LOD is forced here. |
| Final pixel output | `tools/aw_editor/src/viewport/renderer.rs:367-479`, `tools/aw_editor/src/viewport/renderer.rs:573-595` | OK | Scene renders to HDR, then the viewport blits HDR to LDR for display. |
| Scene/world-partition integration | local editor scatter code at `tools/aw_editor/src/viewport/engine_adapter.rs:1358-1364` and `tools/aw_editor/src/viewport/engine_adapter.rs:1968-1995`; no editor references found to `WorldPartition`, `PartitionedScene`, `SceneStreamer`, `gpu_resource_manager`, or `VegetationGpuPipeline` in `tools/aw_editor/src/**` | FAIL | The editor viewport owns a parallel vegetation streaming path. |

## Root Cause Analysis

### RC-1: LOD thresholds ignore final world-space size

Confidence: high.

Evidence:

- `ScatterPlacement::from_zone_placement()` applies large heuristic scale multipliers such as trees `14.0` and cactus `8.0` in `tools/aw_editor/src/terrain_integration.rs:2554-2562`
- BiomePack-driven placements receive an additional `PACK_SCALE_BOOST = 8.0` in `tools/aw_editor/src/terrain_integration.rs:2298-2300`
- the viewport computes raw model AABB and classifies `is_tree` with `model_height > 3.0` in `tools/aw_editor/src/viewport/engine_adapter.rs:1444-1474`
- the final scale is only applied when building the instance transform later in `tools/aw_editor/src/viewport/engine_adapter.rs:1530-1557`

Impact:

If a mesh is only `2.5` units tall in raw glTF space but is scaled to `20` to `35` world units by terrain integration, the viewport still treats it as small vegetation and assigns the non-tree thresholds:

- `lod0_max = 100.0`
- `lod1_max = 100.0`
- `lod2_max = 250.0`

from `tools/aw_editor/src/viewport/engine_adapter.rs:1472-1474`.

That is a plausible direct explanation for "large trees render as crude low-detail shapes until the camera is extremely close." Visually large assets are being LODed according to their unscaled import size, not their rendered size.

### RC-2: LOD refresh is stale and thresholded

Confidence: high.

Evidence:

- `update_camera()` only calls `refresh_scatter_lod()` when XZ movement exceeds `50.0` units in `tools/aw_editor/src/viewport/engine_adapter.rs:242-253`
- the distance check explicitly ignores Y with `let dist_sq = delta.x * delta.x + delta.z * delta.z` in `tools/aw_editor/src/viewport/engine_adapter.rs:248-250`
- the rebucketing baseline is cached in `scatter_lod_camera_pos` and reset during upload in `tools/aw_editor/src/viewport/engine_adapter.rs:106-110` and `tools/aw_editor/src/viewport/engine_adapter.rs:1326`

Impact:

The viewport does not continuously converge LOD to the current camera distance. It updates in bursts.

This creates two visible artifacts:

- small dolly/orbit changes can leave vegetation stuck in an out-of-date bucket
- large step changes happen once the threshold is crossed, producing hard pops instead of smooth transitions

This is especially bad for a top-down editor camera because large altitude changes can happen with minimal XZ delta.

### RC-3: The editor only has two real vegetation geometry modes

Confidence: high.

Evidence:

- `VegetationLod::FullMesh | VegetationLod::Simplified` both go to `lod0_instances` in `tools/aw_editor/src/viewport/engine_adapter.rs:1559-1560`
- `VegetationLod::CrossBillboard | VegetationLod::ImpostorCard` both go to `billboard_instances` in `tools/aw_editor/src/viewport/engine_adapter.rs:1563-1564`
- billboard upload always uses `generate_cross_billboard(...)` in `tools/aw_editor/src/viewport/engine_adapter.rs:1776-1782`
- the shared renderer's intended chain is explicitly richer: `full mesh -> simplified -> cross billboard -> impostor card` in `astraweave-render/src/vegetation_lod.rs:1-12` and `astraweave-render/src/vegetation_lod.rs:62-72`

Impact:

Even when the classification names imply four LOD bands, the editor only renders:

- full mesh
- cross billboard
- nothing

That amplifies visible popping and makes the low-detail state look much cruder than the shared renderer architecture suggests.

### RC-4: The editor bypasses the shared vegetation pipeline entirely

Confidence: high.

Evidence:

- the shared renderer exposes a dedicated compute-driven vegetation path in `astraweave-render/src/vegetation_gpu.rs:1-220`
- `astraweave-render/src/vegetation_lod.rs:11-12` states that the intended path writes per-instance LOD and issues four separate draw-indirect calls
- `tools/aw_editor/src/viewport/engine_adapter.rs:1471-1525` uses only helper pieces (`TreeLodDistances`, `select_lod`, `pcg_hash`) while doing its own CPU-side upload path
- no references were found in `tools/aw_editor/src/**` to `VegetationGpuPipeline`

Impact:

The editor has architecture drift. Any bug fix applied only to the shared renderer's vegetation path will not fix the viewport until the editor's parallel implementation is also changed.

## All Findings

### F-01 [Critical] Raw-mesh-size classification drives LOD, not rendered size

Evidence: `tools/aw_editor/src/terrain_integration.rs:2298-2300`, `tools/aw_editor/src/terrain_integration.rs:2554-2562`, `tools/aw_editor/src/viewport/engine_adapter.rs:1470-1474`, `tools/aw_editor/src/viewport/engine_adapter.rs:1530-1557`.

Why it matters: LOD bands are chosen before placement scale is applied, so visually large trees can stay in billboard mode until implausibly close ranges.

### F-02 [High] Scatter LOD only refreshes after `50` units of XZ camera movement

Evidence: `tools/aw_editor/src/viewport/engine_adapter.rs:242-253`.

Why it matters: This is a textbook "build once, then update rarely" pattern. It guarantees stale LOD buckets and visible pops.

### F-03 [High] Y distance is ignored during LOD classification

Evidence: `tools/aw_editor/src/viewport/engine_adapter.rs:1505-1508` and `tools/aw_editor/src/viewport/engine_adapter.rs:248-250`.

Why it matters: The editor camera commonly changes altitude dramatically. Ignoring Y means zooming or altitude changes can fail to update the vegetation band that should be shown.

### F-04 [High] `Simplified` LOD is not a real editor geometry path

Evidence: `tools/aw_editor/src/viewport/engine_adapter.rs:1559-1560`.

Why it matters: The viewport jumps straight from billboard back to full mesh. There is no intermediate decimated mesh to soften the transition.

### F-05 [High] `ImpostorCard` is not a real editor geometry path either

Evidence: `tools/aw_editor/src/viewport/engine_adapter.rs:1563-1564`, `tools/aw_editor/src/viewport/engine_adapter.rs:1776-1782`, `astraweave-render/src/vegetation_lod.rs:149-199`.

Why it matters: The farthest LOD in the editor is still a cross-billboard, not a single camera-facing impostor. The editor's low-detail geometry is therefore cruder and more visibly "primitive" than the shared design intends.

### F-06 [Medium] The viewport owns a parallel scatter streaming implementation

Evidence: active placement filter at `tools/aw_editor/src/viewport/engine_adapter.rs:1358-1364`; stream load/unload loop at `tools/aw_editor/src/viewport/engine_adapter.rs:1968-1995`; no editor references found to `WorldPartition`, `PartitionedScene`, `SceneStreamer`, `gpu_resource_manager`, or `VegetationGpuPipeline`.

Why it matters: This duplicates responsibility already present elsewhere in the codebase and increases the chance of behavior drift.

### F-07 [Medium] The viewport stats are misleading for scatter diagnosis

Evidence:

- toolbar triangle count is populated from terrain triangles only in `tools/aw_editor/src/viewport/widget.rs:745-752`
- that value is labeled simply as `Triangles` in `tools/aw_editor/src/viewport/toolbar.rs:331-334`
- `scatter_triangles()` and `scatter_vertices()` are stubs returning `0` in `tools/aw_editor/src/viewport/renderer.rs:1469-1476`
- scene/runtime aggregate stats then consume those stubs in `tools/aw_editor/src/subsystems/scene_stats.rs:26-37` and `tools/aw_editor/src/subsystems/docking_sync.rs:30-36`
- `scatter_placement_count` is assigned from all generated placements and `scatter_draw_call_count` is assigned from `loaded_groups` in `tools/aw_editor/src/viewport/engine_adapter.rs:1876-1877`
- actual uploads are per-quadrant named submodels such as `scatter_{key}_q{qi}` and `scatter_{key}_bb_q{qi}` in `tools/aw_editor/src/viewport/engine_adapter.rs:1717-1719` and `tools/aw_editor/src/viewport/engine_adapter.rs:1802`

Why it matters: the observed `1.02M triangles / 36,179 instances / 21 draws` style numbers cannot be trusted as a direct measure of scatter LOD behavior.

### F-08 [Low] Mesh/material/vertex-format fallback is not the driver of this bug

Evidence:

- missing scatter meshes are skipped, not replaced, in `tools/aw_editor/src/viewport/engine_adapter.rs:1394-1405`
- oversized scatter meshes are also skipped, not replaced, in `tools/aw_editor/src/viewport/engine_adapter.rs:1433-1442`
- billboards are converted to the same canonical `CpuMesh -> Mesh` path in `tools/aw_editor/src/viewport/engine_adapter.rs:1776-1782` and `astraweave-render/src/renderer.rs:4376-4382`
- the canonical vertex contracts are `MeshVertex` and `Vertex`/`InstanceRaw` in `astraweave-render/src/mesh.rs:7-46` and `astraweave-render/src/types.rs:6-41`, `astraweave-render/src/types.rs:110-119`

Why it matters: the crude look is coming from intentional billboard geometry, not from a broken mesh-format fallback.

### F-09 [Low] Camera-relative rendering is not the active cause here

Evidence:

- `update_camera_matrices()` contains camera-relative handling in `astraweave-render/src/renderer.rs:4039-4087`
- but `aw_editor` depends on `astraweave-render` with features `gltf-assets` and `textures` only in `tools/aw_editor/Cargo.toml:56`

Why it matters: this removes one plausible false lead. The current viewport issue does not require the camera-relative feature to explain the observed behavior.

## Triangle Canary Interpretation

The user-observed ratio `1,021,000 / 36,179 = 28.22 triangles per instance` is not a valid scatter-LOD diagnostic in the current editor.

Why:

- the toolbar `Triangles` value is terrain-only in `tools/aw_editor/src/viewport/widget.rs:745-752` and `tools/aw_editor/src/viewport/toolbar.rs:331-334`
- the aggregate stats path currently treats scatter triangles as zero because `scatter_triangles()` is stubbed in `tools/aw_editor/src/viewport/renderer.rs:1469-1476`

So the numerator and denominator are not describing the same rendered population.

For reference only:

- a cross-billboard generated by `generate_cross_billboard()` uses `24` indices, which is `8` triangles, in `astraweave-render/src/vegetation_lod.rs:149-199`
- the observed `28.22` value is therefore not consistent with a pure billboard population and is much more consistent with terrain triangles being mixed into the same UI panel while scatter triangles are omitted

## Remediation Plan

### 1. Remove the parallel editor-only LOD implementation

Preferred fix: route viewport vegetation through the shared `VegetationGpuPipeline` and `VegetationLodChain` infrastructure from `astraweave-render/src/vegetation_gpu.rs:111-220` and `astraweave-render/src/vegetation_lod.rs:369-436`.

Minimum acceptable fix if full unification is too large for one change: extract the classification and geometry-path policy into a shared module and make both runtime and editor call the same code.

### 2. Base thresholds on final world-space size, not raw import size

Replace `let is_tree = model_height > 3.0` from `tools/aw_editor/src/viewport/engine_adapter.rs:1470` with either:

- asset-category metadata from the BiomePack when available
- or a scaled test such as `scaled_height = model_height * representative_scale`

The LOD band decision must use the size that actually reaches the screen.

### 3. Stop thresholded rebucketing on camera movement

Remove or drastically reduce the `LOD_REFRESH_THRESHOLD = 50.0` policy in `tools/aw_editor/src/viewport/engine_adapter.rs:247-253`.

Minimum acceptable fix:

- include full 3D camera delta, not just XZ
- refresh every frame while the camera is moving
- or maintain hysteretic per-bin updates with much smaller thresholds

Preferred fix: let the shared GPU path classify visible instances every frame.

### 4. Implement the real four-band chain or rename the bands honestly

Today the editor's actual chain is `full mesh -> cross billboard -> cull`.

Either:

- implement real `Simplified` and `ImpostorCard` geometry paths in the editor
- or stop pretending those bands exist in viewport code and UI

The current mismatch makes debugging much harder.

### 5. Fix viewport observability before trusting future screenshots

Required fixes:

- report active uploaded scatter instance count, not total generated placement count
- report actual named-model draw count after quadrant/bin expansion, not `loaded_groups`
- implement non-stub `scatter_triangles()` and `scatter_vertices()`
- stop labeling terrain-only counts as total viewport triangles

Until these are fixed, the viewport stats panel will continue to mislead diagnosis.

### 6. Decide whether the editor should keep its own scatter streamer

If the editor must remain independent, document the local `3 / 5` chunk-ring policy clearly and treat it as a supported subsystem.

If not, migrate it onto the existing scene/world-partition abstractions so streaming, culling, and resource budgeting are not split across parallel systems.

## Final Assessment

The symptom is not caused by the shared renderer forcing a low-detail mesh, nor by a vertex-format/shader mismatch.

The viewport is classifying vegetation too aggressively because it uses raw import dimensions instead of rendered world-space size, updates those classifications too infrequently, and collapses the intended four-band LOD chain into a much harsher two-geometry implementation.

That combination is sufficient to produce exactly what the user described: trees and vegetation staying in crude billboard form until the camera is unusually close, then popping sharply to the full-detail mesh.