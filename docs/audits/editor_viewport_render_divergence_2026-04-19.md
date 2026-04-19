# Editor Viewport ↔ `astraweave-render` Divergence Audit

**Date**: 2026-04-19
**Scope**: `tools/aw_editor/src/viewport/` vs. `astraweave-render`
**Mode**: Read-only. No code edits, no commits beyond this report.
**Prior documents treated as hypotheses**:
`docs/current/ARCHITECTURE_MAP.md` §6 and §11,
`docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md`.

---

## 1. Executive summary

The editor is **partially consolidated** onto `astraweave-render`. The structural work (Phase 0–1 of the Fix 27 campaign plan) shipped: `entity_renderer.rs` and its four shader files are deleted, `astraweave-render` is a required dependency, all entity/terrain/scatter/sky/shadow/IBL rendering flows through `astraweave_render::Renderer::draw_into`, and there are zero `#[cfg(feature = "astraweave-render")]` guards remaining. The adapter (`engine_adapter.rs`, 3,985 LOC) is the sole owner of scene rendering.

However, the deeper unification (Phases 3–4 of the campaign plan — "Tonemap/Post-processing unification" and "Overlay Injection Protocol") **did not ship**. The editor still owns a multi-pass orchestration layer on top of the engine: it creates its own HDR `Rgba16Float` target, calls `draw_into` to fill it, then runs its own grid pass, physics debug pass, tonemap/HDR→LDR blit pass (using its own WGSL), and gizmo pass. The promised `EditorOverlayHooks` trait and `draw_into_with_hooks` engine API do not exist in the tree. The promised `RenderBackend` enum does not exist; `RenderMode { EnginePBR, FastPreview }` survives unchanged. `RenderMode::FastPreview` is a **zombie toggle** — the UI exposes a user-flippable "Engine PBR" checkbox and keyboard shortcut that set this field, but no code in the render path reads it, so flipping it has no visible effect.

Three parallel type/enum definitions diverge silently: the editor's `TerrainVertex` (~80 bytes, 8 biome weights + 4 material IDs + 4 material weights) vs. the engine's `TerrainVertex` (~28 bytes, a single `biome_id: u32`), lossily collapsed by `types.rs:35-63`; the editor's `WeatherKind` (6 variants including `Hail` and `Blizzard`) vs. the engine's `WeatherKind` (5 variants including `WindTrails`); and the editor's `tonemap.wgsl` tonemap-operator index table (`ACES=0, PBR_Neutral=1, Reinhard=2, AgX=3`) vs. the engine's internal `TONEMAP_SHADER` index table (`ACES=0, AgX=1, Reinhard=2, None=3`) — same numeric indices map to different operators.

---

## 2. Consolidation claim verification

Claims enumerated from `FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` (campaign plan, still marked "PLAN — awaiting approval" in its header) and `ARCHITECTURE_MAP.md` §6, §11 (describes the post-campaign state as if the plan had been fully executed).

| ID | Claim (source) | Verdict | Evidence |
|---|---|---|---|
| C1 | `viewport/entity_renderer.rs` was deleted | **CONFIRMED** | Absent from [tools/aw_editor/src/viewport/](tools/aw_editor/src/viewport/) |
| C2 | `viewport/mipmap_generator.rs` was deleted | **CONFIRMED** | Absent |
| C3 | `viewport/shaders/entity.wgsl` was deleted | **CONFIRMED** | Remaining WGSL: `grid.wgsl`, `gizmo.wgsl`, `tonemap.wgsl` only |
| C4 | `viewport/shaders/shadow.wgsl` was deleted | **CONFIRMED** | Absent |
| C5 | `viewport/shaders/brdf_lut.wgsl` was deleted | **CONFIRMED** | Absent |
| C6 | `viewport/shaders/tonemap.wgsl` was deleted (Campaign Phase 3 plan) | **FALSIFIED** | File present: [tools/aw_editor/src/viewport/shaders/tonemap.wgsl](tools/aw_editor/src/viewport/shaders/tonemap.wgsl), 134 lines |
| C7 | `viewport/shaders/mipmap_blit.wgsl` was deleted | **CONFIRMED** | Absent |
| C8 | `src/tab_viewer.rs` was deleted | **PARTIAL** | No `src/tab_viewer.rs` file, but `src/tab_viewer/` directory exists with `mod.rs`. Content relocated, not eliminated. |
| C9 | `tools/aw_editor/Cargo.toml`: `astraweave-render` is non-optional | **CONFIRMED** | [tools/aw_editor/Cargo.toml:72](tools/aw_editor/Cargo.toml#L72) — no `optional = true`. Only `astraweave-alloc` (line 71) and `egui_kittest` (line 95) are optional. |
| C10 | All `#[cfg(feature = "astraweave-render")]` guards removed (~30) | **CONFIRMED** | 0 occurrences in `tools/aw_editor/` |
| C11 | `RenderBackend { Engine, LegacyPreview, Headless }` replaces `RenderMode` (Campaign Phase 0) | **FALSIFIED** | `RenderMode { EnginePBR, FastPreview }` still present at [engine_adapter.rs:16-23](tools/aw_editor/src/viewport/engine_adapter.rs#L16-L23); re-exported from `renderer::RenderMode` in [mod.rs:73](tools/aw_editor/src/viewport/mod.rs#L73). No `RenderBackend` enum exists in the workspace. |
| C12 | `EditorOverlayHooks` trait added to `astraweave-render` (Campaign Phase 4) | **FALSIFIED** | Trait appears only in the campaign plan document and `HANDOFF.md:125` (where it is listed as pending). No definition in `astraweave-render/src/`. |
| C13 | `draw_into_with_hooks()` added to `astraweave-render::Renderer` | **FALSIFIED** | Only `pub fn draw_into` exists at [astraweave-render/src/renderer.rs:5184](astraweave-render/src/renderer.rs#L5184). No `draw_into_with_hooks` anywhere in the workspace. |
| C14 | `entity_renderer` module declaration removed from `viewport/mod.rs` | **CONFIRMED** | Not present in [viewport/mod.rs](tools/aw_editor/src/viewport/mod.rs) |
| C15 | `EngineRenderAdapter` expanded from ~568 LOC to ~740 LOC (§11) | **FALSIFIED** | Actual size: **3,985 LOC** (5.4× the claim). [engine_adapter.rs](tools/aw_editor/src/viewport/engine_adapter.rs) |
| C16 | `ViewportRenderer` ~900 LOC (§6.1) | **FALSIFIED** | Actual size: **1,681 LOC**. [renderer.rs](tools/aw_editor/src/viewport/renderer.rs) |
| C17 | §6.5: "`FastPreview` enum value is kept for API compatibility but behavior is now identical to EnginePBR" | **CONFIRMED** (and extended) | `render()` never reads `self.render_mode`; see §8 below. The field is set but never consulted during rendering. |
| C18 | §6.1 file list is complete | **PARTIAL** | §6.1 omits four files that exist: `terrain_splat.rs`, `terrain_splat_builder.rs`, `impostor_registry.rs` (feature-gated), `impostor_wiring.rs` (feature-gated). Map enumerates 13 items; the current viewport directory has 15 Rust files + 3 WGSL. |
| C19 | `EngineRenderAdapter` wraps `astraweave_render::Renderer` (single field) | **CONFIRMED** | [engine_adapter.rs:534](tools/aw_editor/src/viewport/engine_adapter.rs#L534): `renderer: astraweave_render::Renderer` |
| C20 | 4-cascade CSM in engine | Out of scope; unverified here |
| C21 | IBL prefiltered cubemap in engine | Out of scope; unverified here |
| C22 | `Pose.scale_y`, `Pose.scale_z` added | Out of scope; unverified here |

### Consolidation state discrepancies (items requiring attention)

The following FALSIFIED/PARTIAL verdicts indicate that documentation is ahead of the code:

1. **C6 — `tonemap.wgsl` still present**: Campaign Phase 3 planned to delete it; it survives. Architecture Map §6.1 *correctly* lists it as still present, but the Phase 3 plan (and its stated outcome "editor tonemap pass deleted") was never executed.
2. **C11 — `RenderBackend` never created**: Campaign Phase 0 planned this as the safety-net toggle; it was skipped. `RenderMode` remains with `FastPreview`.
3. **C12 / C13 — Overlay hooks never added to engine**: Campaign Phase 4 specified a formal `EditorOverlayHooks` trait + `draw_into_with_hooks`. Neither exists. The editor still orchestrates overlays externally.
4. **C15 — `EngineRenderAdapter` is 5.4× the documented size**: 3,985 LOC vs. the Architecture Map's claim of ~740 LOC. This is the most under-documented artifact in the audit.
5. **C17 — `FastPreview` is worse than "kept for API compatibility"**: it is an active UI toggle that users can flip with no effect.
6. **C8 — `tab_viewer` was relocated, not removed**: Architecture Map lists it as "deleted"; it actually became a module directory.

---

## 3. Import surface — what the editor pulls in from `astraweave-render`

All `use astraweave_render::*` and `astraweave_render::*` qualified references across `tools/aw_editor/src/`. Grouped by category. File:line citations are into the editor tree unless stated otherwise.

### Pipeline / renderer primitives

| File:Line | Symbol | Use site |
|---|---|---|
| `viewport/engine_adapter.rs:534` | `astraweave_render::Renderer` (field) | The single engine instance the adapter wraps |
| `viewport/engine_adapter.rs:678` | `astraweave_render::Renderer::new_from_device` | Construct the engine renderer from the editor's shared device/queue |
| `viewport/engine_adapter.rs:878, 882` | `fn renderer(&self) -> &astraweave_render::Renderer` (getter) | Panels reach into the engine via the adapter |
| `viewport/engine_adapter.rs:908` | `astraweave_render::MemoryCategory` | GPU memory snapshot reporting |
| `viewport/engine_adapter.rs:835` | `self.renderer.draw_into(target, encoder)` | **The scene-rendering handoff to the engine** |

### Post-processing / HDR / tonemap

| File:Line | Symbol | Use site |
|---|---|---|
| `viewport/engine_adapter.rs:932, 971, 1001, 1023, 3543, 3549` | `astraweave_render::hdr_pipeline::PostProcessChain` | Four `PostProcessChain` configurations per quality preset; set on the engine via `set_post_process_chain` |
| `viewport/engine_adapter.rs:943, 982, 1012, 1034` | `astraweave_render::hdr_pipeline::TonemapOperator::Aces` | Hard-coded ACES on all four presets |
| `viewport/engine_adapter.rs:3554` / `renderer.rs:954` / `subsystems/docking_sync.rs:169` | `astraweave_render::bloom::BloomConfig` | Bloom parameters forwarded to engine |

### Sky / IBL / weather / water

| File:Line | Symbol | Use site |
|---|---|---|
| `viewport/engine_adapter.rs:468, 1461, 3451, 3471, 3972` | `astraweave_render::SkyConfig` | Sky config read/write, weather defaults |
| `viewport/engine_adapter.rs:3459, 3464` | `astraweave_render::ibl::{SkyMode, IblQuality}` | IBL bake control (Medium quality, HdrPath and Procedural modes) |
| `viewport/engine_adapter.rs:3476-3477` / `main.rs:343, 3964` / `tab_viewer/mod.rs:1445-1450` / `widget.rs:2610` | `astraweave_render::WeatherKind` | **Two `WeatherKind` types in use simultaneously** (see §4 divergences) |
| `viewport/engine_adapter.rs:3563` | `astraweave_render::WaterRenderer` | Water pass enable/disable |
| `viewport/renderer.rs:1284, 1290` | `astraweave_render::ibl::{SkyMode, IblQuality}` | Procedural sky mode + bake trigger |

### Mesh / geometry / materials

| File:Line | Symbol | Use site |
|---|---|---|
| `viewport/engine_adapter.rs:60, 61, 67, 68, 578, 613, 1745, 3357` | `astraweave_render::mesh::CpuMesh` | Scatter primitive LOD assets, billboards, impostor cards |
| `viewport/engine_adapter.rs:1048, 1054, 1184, 1185, 2371, 2372` | `astraweave_render::mesh_gltf::{GltfOptions, load_gltf}` | glTF ingest for entities + scatter |
| `viewport/engine_adapter.rs:1572, 2427, 2488, 2503, 2523, 2552, 2596, 2724, 2829, 2947` | `astraweave_render::Instance` | Per-entity instance data for engine |
| `viewport/engine_adapter.rs:1995` | `astraweave_render::ModelSurfaceMaps` | Terrain surface material bindings |
| `viewport/impostor_wiring.rs:31` | `astraweave_render::mesh::{CpuImage, CpuMesh}`, `lod_generator::SimplificationMesh` | Impostor bake inputs |

### LOD / vegetation / impostor

| File:Line | Symbol | Use site |
|---|---|---|
| `viewport/engine_adapter.rs:1760, 1781, 1804, 1806, 1815, 2419, 2446, 2480-2530, 3368` | `astraweave_render::vegetation_lod::*` (`AtlasRegion`, `VegetationLodChain`, `generate_cross_billboard`, `generate_impostor_card`, `TreeLodDistances`, `adaptive_lod_distances`, `select_lod`, `VegetationLod`, `ImpostorAtlasSpec`) | **All LOD decisions delegated to the engine's `vegetation_lod` module** |
| `viewport/engine_adapter.rs:2439, 2569, 3358` | `astraweave_render::impostor_lod3::Lod3InstanceRaw` | LOD3 impostor instances |
| `viewport/engine_adapter.rs:3367` | `astraweave_render::impostor_pass::ImpostorPass` | Engine's impostor render pass wrapper |
| `viewport/engine_adapter.rs:2457, 2458` | `astraweave_render::vegetation_gpu::{pcg_hash, hash_to_float}` | Shared hash helpers |
| `viewport/impostor_registry.rs:45, 46` | `astraweave_render::impostor_bake::{load_or_bake_atlas, LoadedAtlas}`, `vegetation_lod::ImpostorAtlasSpec` | Atlas cache |
| `viewport/impostor_wiring.rs:27` | `astraweave_render::impostor_bake::{fit_ortho_camera, upload_simplification_mesh, Aabb, ImpostorBaker, ImpostorBakerConfig}` | Bake helpers |

### Terrain

| File:Line | Symbol | Use site |
|---|---|---|
| `viewport/types.rs:35, 56` | `astraweave_render::TerrainVertex` | Lossy conversion target from editor's 80-byte vertex |
| `viewport/terrain_splat.rs:31-33` | `astraweave_render::{ChunkKey, LayerTextures, TerrainMaterialConfig, TerrainMaterialGpu, TerrainMaterialManager}` | Thin editor wrapper around the engine's splat-array manager (feature-gated) |

### Camera

| File:Line | Symbol | Use site |
|---|---|---|
| `viewport/camera.rs:624-637` | `astraweave_render::camera::Camera` | `OrbitCamera::to_engine_camera()` conversion (yaw rotated by π, pitch negated) |

### Summary

The editor imports from **16 distinct public modules** of `astraweave-render`:
`Renderer`, `camera`, `mesh`, `mesh_gltf`, `Instance`, `MemoryCategory`, `hdr_pipeline`, `bloom`, `SkyConfig`, `ibl`, `WeatherKind`, `WaterRenderer`, `ModelSurfaceMaps`, `TerrainVertex`, `vegetation_lod`, `vegetation_gpu`, `impostor_lod3`, `impostor_pass`, `impostor_bake`, `lod_generator`, `ChunkKey`, `LayerTextures`, `TerrainMaterialConfig`, `TerrainMaterialGpu`, `TerrainMaterialManager`. The surface is broad and uses engine primitives directly (no editor-local reimplementations of these).

---

## 4. Reimplementation inventory — per-file (A)/(B)/(C) classification

`A` = editor-specific, no engine analog. `B` = adapter wrapping engine functionality. `C` = reimplements engine functionality.

| File | LOC | Category | Rationale |
|---|---:|---|---|
| `viewport/mod.rs` | 87 | A | Module declarations / re-exports |
| `viewport/widget.rs` | 2,855 | A | egui integration, input handling, panel wiring — no engine analog |
| `viewport/renderer.rs` | 1,681 | **B+C** | Primarily orchestrates the multi-pass flow on top of `draw_into` (B), but maintains its own HDR render target + tonemap pass + depth readback that duplicate responsibilities present in the engine's HDR pipeline (C). See §6. |
| `viewport/engine_adapter.rs` | 3,985 | **B+C** | Primarily wraps `astraweave_render::Renderer` (B). Also contains editor-specific logic: cluster-binning of terrain chunks for rebuild routing, scatter LOD rebucketing, impostor bake coordination, quality-preset → PostProcessChain translation. Much of this is editor-shaped plumbing; terrain cluster-binning (lines 480–531) is an editor-local spatial-bin algorithm. |
| `viewport/camera.rs` | 901 | A | `OrbitCamera` (spherical-coordinate editor camera). No engine analog — engine uses a free-fly `camera::Camera`. Exposes `to_engine_camera()` converter (camera.rs:624). |
| `viewport/types.rs` | 346 | **C (partial)** | `TerrainVertex`, `WeatherKind`, `SceneLight`, `GltfSkeleton`, `GltfAnimationClip` all defined locally despite engine having counterparts for two of them. Also includes genuinely editor-local types (`TerrainFogParams`, `TerrainLightingParams`, `WaterStyle`, `MATERIAL_NAMES`). See divergence table below. |
| `viewport/grid_renderer.rs` | 303 | A | Floor-grid overlay. No engine analog. |
| `viewport/gizmo_renderer.rs` | 365 | A | Transform-handle overlay. No engine analog. |
| `viewport/physics_renderer.rs` | 335 | A | Debug-line renderer for Rapier collider wireframes, component gizmos, brush cursors, zone overlays. No engine analog. |
| `viewport/blueprint_overlay.rs` | 279 | A | Blueprint zone visualization. Editor-only concept. |
| `viewport/toolbar.rs` | 781 | A | Toolbar UI. |
| `viewport/terrain_splat.rs` | 357 | B | Feature-gated thin wrapper around `astraweave_render::TerrainMaterialManager`. Owns a copy of state (`chunk_count`, `initialized`, `material_uploaded`) that is derivable from the manager. |
| `viewport/terrain_splat_builder.rs` | 174 | A (editor CPU helper) | Rasterises per-vertex biome weights into RGBA8 splat maps. Consumes editor's 80-byte `TerrainVertex`; engine manager consumes the resulting bytes. |
| `viewport/impostor_registry.rs` | 339 | B (feature-gated) | Caches `LoadedAtlas` instances keyed by mesh hash. Calls `astraweave_render::impostor_bake::load_or_bake_atlas`. |
| `viewport/impostor_wiring.rs` | 310 | B (feature-gated) | Deterministic content-hash of `CpuMesh` (`primitive_mesh_hash`) and `to_simplification_mesh` adapter for `ImpostorBaker`. |

### Reimplementation divergence table (C entries)

| Editor site | What it does | Analog in `astraweave-render` | Status | Evidence |
|---|---|---|---|---|
| `types.rs:17-29` — `TerrainVertex` | Editor vertex: `pos[3] + normal[3] + uv[2] + biome_weights_0[4] + biome_weights_1[4] + material_ids[4] + material_weights[4]` = ~80 bytes (~96 with Pod padding) | `astraweave-render/src/terrain.rs:18-23` — `TerrainVertex` with `pos[3] + normal[3] + uv[2] + biome_id: u32` = ~28 bytes | **DIVERGED** (parallel definition) | Lossy conversion in `types.rs:31-63` picks the dominant weight among 8 biomes, collapses to single `biome_id`. Material IDs and weights are discarded at the engine boundary. |
| `types.rs:134-143` — `WeatherKind` | `None=0, Rain=1, Snow=2, Hail=3, Sandstorm=4, Blizzard=5`, `#[repr(u32)]` | `astraweave-render/src/effects.rs:7-13` — `None, Rain, Snow, Sandstorm, WindTrails`, no explicit repr | **DIVERGED** (parallel definition, variants mismatch) | Editor adds `Hail` and `Blizzard`; engine adds `WindTrails`. Both types are imported simultaneously — editor's is used within the viewport UI; engine's is used when setting weather on the renderer (e.g. `main.rs:343, 3964`, `widget.rs:2610`, `tab_viewer/mod.rs:1445-1450`). No explicit converter between the two WeatherKinds; `tab_viewer/mod.rs:1445-1450` contains a custom mapping table. |
| `viewport/shaders/tonemap.wgsl` | WGSL tonemap applying one of `ACES=0, PBR_Neutral=1, Reinhard=2, AgX=3` based on `params.mode` | `astraweave-render/src/hdr_pipeline.rs:458-595` — `TONEMAP_SHADER` const applying `ACES=0, AgX=1, Reinhard=2, None=3` based on `u.tonemap_op` | **DIVERGED** (different operator/index table, different operator set) | See §5 below. |
| `renderer.rs:647-771` — tonemap pipeline | Editor's HDR→LDR blit pass, applies `shaders/tonemap.wgsl` | `hdr_pipeline.rs:347-417` — `HdrPipeline::tonemap_pass` | **DIVERGED** (two tonemap passes exist; the engine's runs inside `draw_into`, then the editor's runs again on the output). Comment at `renderer.rs:573-575` describes the editor pass as a "blit", but the shader is a full tonemap. See §6 for the double-pass question. |
| `engine_adapter.rs:480-531` — `cluster_terrain_chunks` | Spatial-bin algorithm for grouping terrain chunks by 2D cell, capped by vertex count | No direct analog | **EDITOR-LOCAL LOGIC** — kept on editor side because chunk lifetimes are editor-driven (brush edits rebuild individual clusters). Not strictly a divergence — there is nothing in the engine that does this. |
| `types.rs:275-280` — `SceneLight` | Point-light DTO for entity light components | No direct analog in engine's public API surface (engine uses internal `CpuLight`, not exported here) | **ADAPTER DTO** — carries data between the editor's ECS and the adapter's light-feeding path. |
| `types.rs:284-346` — `GltfSkeleton`, `GltfAnimationClip`, etc. | Skeleton + animation data extracted from glTF | Engine has `mesh_gltf::load_gltf` but does not publicly expose skeleton types at this granularity (based on the imports observed) | **EDITOR-OWNED** — comment at `types.rs:285-293` notes these were moved here from `entity_renderer.rs` during Fix 27. |

---

## 5. Shader divergence

### 5.1 WGSL inventory in `tools/aw_editor/src/`

| File | Status vs. Fix 27 Campaign Plan |
|---|---|
| `viewport/shaders/grid.wgsl` | Retained. Campaign kept grid on editor side. No engine analog exists. |
| `viewport/shaders/gizmo.wgsl` | Retained. No engine analog exists. |
| `viewport/shaders/tonemap.wgsl` | **Retained, despite Campaign Phase 3 intending deletion**. |
| `shaders/entity.wgsl`, `shadow.wgsl`, `brdf_lut.wgsl`, `mipmap_blit.wgsl` | Deleted (confirmed absent). |

No other WGSL files exist in the editor tree. No WGSL source is loaded from `astraweave-render` at runtime via `include_str!` or asset path.

### 5.2 `tonemap.wgsl` (editor) vs. `TONEMAP_SHADER` (engine)

Both are fullscreen-triangle fragment shaders that read a `texture_2d<f32>` and write tonemapped output.

| Aspect | Editor `viewport/shaders/tonemap.wgsl` | Engine `astraweave-render/src/hdr_pipeline.rs:458-595` (`TONEMAP_SHADER`) |
|---|---|---|
| Source form | Separate `.wgsl` file included via `include_str!` at `renderer.rs:728` | Embedded Rust `const` string |
| Entry points | `vs_main`, `fs_main` | `vs_main`, `fs_main` |
| Bind group 0, binding 0 | `texture_2d<f32>` (HDR source) | `texture_2d<f32>` (HDR source) |
| Bind group 0, binding 1 | `sampler` | `sampler` |
| Bind group 0, binding 2 | `uniform TonemapParams { mode: u32, _pad0, _pad1, _pad2 }` — one u32 used | `uniform TonemapUniforms { tonemap_op: u32, + color-grading fields … }` — richer uniform |
| Operator index table | 0=ACES, 1=PBR Neutral (Khronos), 2=Reinhard, 3=AgX | 0=ACES, 1=AgX, 2=Reinhard, 3=None (linear passthrough) |
| Operator set | ACES (Narkowicz 2015), PBR Neutral (Khronos 2024), Reinhard, AgX (Sobotka) | ACES (Narkowicz), AgX, Reinhard, None |
| Color grading | Not applied | Applied (color balance, saturation, contrast fields in uniform) |
| Target format assumption | `Bgra8UnormSrgb` (comment at line 131); outputs linear | Whatever `HdrPipeline::tonemap_pass` receives as `output_view` |
| sRGB gamma | Relies on sRGB target format applying gamma automatically | Handled inside shader / color grading pipeline |

**Divergences that matter**:

1. **Operator index is not a stable contract between editor and engine.** If any code path passed `tonemap_mode` from the editor to the engine (or vice versa), mode `1` would mean "PBR Neutral" in the editor and "AgX" in the engine. As of this audit, the editor does **not** pipe its `tonemap_mode` field into the engine's `PostProcessChain` — it hard-codes `TonemapOperator::Aces` in all four quality presets (`engine_adapter.rs:943, 982, 1012, 1034`). So the mismatch does not currently corrupt output, but it is a latent trap for any future code that tries to unify the selector.
2. **Operator sets differ**: PBR Neutral exists only in the editor; `None` (linear passthrough) exists only in the engine.
3. **Color grading is engine-only**. The editor's tonemap does not apply color grading; if the engine applied it during `draw_into`, the editor's second pass would not.

### 5.3 `grid.wgsl` and `gizmo.wgsl`

No analog in `astraweave-render`. The only `grid`-matching files in the engine crate (`instancing.rs`, `clipmap_terrain.rs`, `decals.rs`) are unrelated (they reference grids of instances, clipmap grids, and decal grids respectively — not an infinite floor grid). These editor shaders are (A) — editor-specific, no divergence.

---

## 6. Pipeline / render-graph divergence

### 6.1 Top-level frame sequence in the editor

`ViewportRenderer::render()` in [renderer.rs:367-645](tools/aw_editor/src/viewport/renderer.rs#L367-L645):

1. **Size / cache bookkeeping** (reads target size, resizes HDR & depth if needed, creates cached LDR view).
2. **Lazy init of engine adapter** if not yet initialized (renderer.rs:410-441; also done from terrain upload paths).
3. **Single `wgpu::CommandEncoder`** is created for the whole frame (renderer.rs:463-467).
4. **Scene pass** (renderer.rs:473-528): `adapter.update_camera` → `adapter.feed_entities` → `adapter.render_to_texture(hdr_view, encoder)` → `renderer.draw_into(hdr_view, encoder)` (engine_adapter.rs:828-876 / renderer.rs:5184 in the engine). If the adapter is `None`, a fallback clear pass writes `(0.12, 0.12, 0.15)` into the HDR view.
5. **Grid overlay pass** onto HDR view (renderer.rs:530-543). Uses `grid.wgsl`.
6. **Physics-debug pass** onto HDR view (renderer.rs:545-571). Combines up to four line-list sources (component gizmos, Rapier debug lines, brush cursor, zone overlay) into one draw call.
7. **Tonemap/blit pass** HDR → LDR (renderer.rs:573-596). Uses editor `tonemap.wgsl`.
8. **Gizmo pass** onto the LDR target, after tonemap (renderer.rs:598-630). Uses `gizmo.wgsl`.
9. **Single `queue.submit`** at end (renderer.rs:632-642).

### 6.2 Pass-by-pass comparison

| Pass | In `astraweave-render` (inside `draw_into`) | In editor (outside `draw_into`) | Notes |
|---|---|---|---|
| Cluster binning (lights) | Yes (renderer.rs:5197-5330, `cluster_bin` compute pass) | No | Engine-only, consumed via `draw_into`. |
| Shadow cascades | Yes (`shadow_csm.rs`, invoked inside `draw_into`) | No | Enabled/disabled via adapter's `apply_quality_preset` calls to `set_shadows_enabled`, `set_cascade_extents`, etc. (engine_adapter.rs:924-1037). |
| Main scene forward/deferred | Yes (`draw_into` `main_render` pass) | No | Engine writes into the HDR view the editor supplies. |
| Post-processing chain (SSAO, bloom, TAA, color grading, tonemap) | Yes (`hdr_pipeline.rs`; scheduled inside `draw_into` via `PostProcessChain`) | No (engine runs it) but **editor configures it** via `PostProcessChain` structs per preset | All four presets set `tonemap_operator: Aces` but disable different passes. The engine's tonemap pass runs as the terminal step of the post-processing chain. |
| Scene environment (sky + IBL) | Yes | No | Configured via `set_sky_config`, `ibl_mut().mode = SkyMode::Procedural` etc. |
| Water | Yes (`WaterRenderer`) | No | Installed on the engine when enabled (engine_adapter.rs:3563). |
| Weather particles | Yes | No | Engine-owned. |
| Impostor pass (LOD3) | Yes (`impostor_pass.rs`) | No | Editor installs via `renderer.install_impostor_pass` (see impostor_wiring.rs comments). |
| Grid overlay | No | **Yes** (editor, `grid.wgsl`) | Editor-local. Writes into HDR target **after** `draw_into`. If the engine's post-processing chain already tonemapped the HDR target, the grid is drawn on tonemapped pixels, then tonemapped again in the editor's pass. See incidental findings. |
| Physics debug lines | No | **Yes** (editor, inline shader in `physics_renderer.rs`) | Same post-`draw_into` placement as grid. |
| Editor tonemap | No | **Yes** (editor, `tonemap.wgsl`) | Editor-local. Runs HDR view → LDR target. See §5. |
| Gizmo overlay | No | **Yes** (editor, `gizmo.wgsl`) | Runs after editor tonemap, on LDR target, so not tonemapped — matches the intent documented in Architecture Map §6.3 Pass 5. |
| Brush depth readback (1-frame deferred async copy) | No | **Yes** (editor, renderer.rs:1145-1215) | Editor-local — used for brush cursor hit detection. Copies single-pixel depth to a staging buffer. |

### 6.3 The overlay-injection contract that never shipped

Campaign Phase 4 specified:

```rust
pub trait EditorOverlayHooks {
    fn render_scene_overlays(encoder, hdr_view, depth_view) -> Result<()>;
    fn render_ldr_overlays(encoder, ldr_view, depth_view) -> Result<()>;
}
// Plus: engine exposes draw_into_with_hooks(...) that calls the hooks at the correct points.
```

This trait and the `draw_into_with_hooks` entry point do not exist in the workspace (grep returned matches only in the campaign document itself and a HANDOFF.md forward-reference). Consequently, the editor's grid/physics-debug passes (intended for HDR + depth injection) and gizmo pass (intended for LDR injection) currently run as *post-`draw_into`* overlay passes inside the editor's `CommandEncoder`. The ordering described in Architecture Map §6.3 ("Grid overlay (on top of engine scene)" on HDR, then "HDR → LDR Blit", then "Gizmo Overlays") matches the current code, but it runs in the editor instead of via the engine hook.

---

## 7. LOD and streaming divergence

Strong convergence here. The editor delegates LOD selection to the engine's `vegetation_lod` module rather than maintaining parallel tables:

- **LOD distances**: `engine_adapter.rs:2419` — `astraweave_render::vegetation_lod::adaptive_lod_distances(...)` produces the `TreeLodDistances` used for per-instance selection.
- **LOD selection**: `engine_adapter.rs:2446` — `astraweave_render::vegetation_lod::select_lod(dist, &lod_distances)` is the only selector used in the hot path. No editor-local distance-threshold comparison code.
- **LOD chain build**: `engine_adapter.rs:1781` — `VegetationLodChain::build(...)` emits `SimplificationMesh`; editor wraps this with extra LOD-asset caches (`scatter_lod_asset_cache`).
- **LOD variants**: `engine_adapter.rs:2480-2530` — match arms dispatch on `VegetationLod::{FullMesh, Simplified, CrossBillboard, ImpostorCard}` (the engine's enum), not an editor-local variant.
- **Chunk streaming**: `engine_adapter.rs:614-625` — editor maintains `scatter_chunk_models: HashMap<ChunkId, Vec<String>>` keyed by `astraweave_terrain::ChunkId`, plus `active_scatter_chunks: HashSet<ChunkId>`. This is streaming *bookkeeping* (which chunks are currently uploaded to the engine) rather than a duplicate streaming system. Camera-driven rebucketing (`refresh_scatter_lod`, `needs_scatter_refresh` budget at engine_adapter.rs:823-825) is editor-local policy on top of the engine's LOD selection.

**No duplicate LOD distance constants or selection logic were found.** The LOD path is a legitimate adapter (B), not a reimplementation (C). This contradicts the April 15 LOD audit's claim that "the editor has its own parallel rendering path that diverged from this" — as of this audit, the LOD path is consolidated onto the engine's `vegetation_lod` module.

The single editor-local spatial-partitioning algorithm is `cluster_terrain_chunks` at `engine_adapter.rs:480-531` — it bins terrain chunks into a 2×2 grid by chunk center, capped by `TERRAIN_MAX_VERTICES_PER_CLUSTER = 5_000_000`. There is no engine analog for this, and it exists because the editor regenerates individual clusters in response to brush edits. Classify as editor-local control-plane logic, not divergence.

---

## 8. `RenderMode::FastPreview` status — deep probe

**Conclusion: zombie.**

### Enum definition

```rust
// tools/aw_editor/src/viewport/engine_adapter.rs:16-23
pub enum RenderMode {
    EnginePBR,
    FastPreview,
}
```

Default: `EnginePBR` (engine_adapter.rs:25-29).

### Where it is set

1. [`renderer.rs:175`](tools/aw_editor/src/viewport/renderer.rs#L175) — constructor initializes to `EnginePBR`.
2. [`renderer.rs:1490-1496`](tools/aw_editor/src/viewport/renderer.rs#L1490-L1496) — `set_use_engine_rendering(bool)` writes `EnginePBR` or `FastPreview`.
3. [`renderer.rs:1504-1506`](tools/aw_editor/src/viewport/renderer.rs#L1504-L1506) — `set_render_mode(mode)` writes directly.

### Where it is read

1. [`renderer.rs:1486`](tools/aw_editor/src/viewport/renderer.rs#L1486) — `use_engine_rendering()` returns `self.render_mode == EnginePBR`. Pure getter.
2. [`renderer.rs:1500`](tools/aw_editor/src/viewport/renderer.rs#L1500) — `render_mode()` returns it. Pure getter.

**Neither `render()` (renderer.rs:367-645) nor any `adapter.*` method reads `self.render_mode`.** Grep confirms: the only `self.render_mode` occurrences in renderer.rs are the field declaration, the constructor, the two writers, and the two getters.

### UI reachability

1. [`main.rs:5530-5538`](tools/aw_editor/src/main.rs#L5530-L5538) — toolbar checkbox labeled **"Engine PBR"**, hover text **"Enable full PBR mesh rendering instead of cube placeholders"**, calls `set_use_engine_rendering(use_pbr)`.
2. [`main.rs:8934-8944`](tools/aw_editor/src/main.rs#L8934-L8944) — `on_toggle_engine_rendering` keyboard-shortcut handler, calls `set_use_engine_rendering(!current)`, writes a console log saying "Engine rendering enabled/disabled".
3. [`main.rs:8947-8958`](tools/aw_editor/src/main.rs#L8947-L8958) — `on_show_engine_info` prints "Engine Rendering: {bool}" and "Adapter Initialized: {bool}" to the console log.
4. [`widget.rs:2375`](tools/aw_editor/src/viewport/widget.rs#L2375) — `renderer.set_use_engine_rendering(true)` called unconditionally (forces EnginePBR).

### Campaign-plan fate

The Fix 27 campaign plan (Phase 0) explicitly intended to replace `RenderMode` with `RenderBackend { Engine, LegacyPreview, Headless }` as a safety-net toggle for A/B-testing during Phase 1 migration. Phase 6 then planned to delete `LegacyPreview`. The replacement never happened; `RenderMode` still has both variants, and `FastPreview` behaves as described in Architecture Map §6.5: "kept for API compatibility but behavior is now identical to EnginePBR". The Architecture Map description is *correct* but understates the UI-level symptom: users can flip a visible checkbox and nothing changes.

Architecture Map §6.5 says: "The distinction is whether the `engine_adapter` field is `Some` (normal operation) or `None` (headless/CI fallback, which clears to a dark color)." This is accurate — the only surviving mode distinction is `engine_adapter.is_some()` at renderer.rs:475.

---

## 9. Dependency and feature-flag surface

### 9.1 Workspace-internal dependencies in `tools/aw_editor/Cargo.toml`

All `astraweave-*` path dependencies (lines 67-91), in declaration order:

- `astract` (non-prefixed path dep)
- `astraweave-core`
- `astraweave-ecs`
- `astraweave-profiling`
- `astraweave-alloc` (**OPTIONAL**, `workspace = true`, gated by `fast-alloc` feature — default on)
- `astraweave-render` — required, `features = ["gltf-assets", "textures"]`
- `astraweave-author`
- `astraweave-asset` (with `blend` feature)
- `astraweave-blend`
- `astraweave-audio`
- `astraweave-behavior`
- `astraweave-dialogue`
- `astraweave-quests`
- `astraweave-nav`
- `astraweave-observability`
- `astraweave-physics`
- `astraweave-security`
- `astraweave-terrain`
- `astraweave-asset-pipeline`

Also `egui_kittest` (line 95) is the only other `optional = true` dependency (gated by the `kittest` feature).

**`astraweave-render` is not optional.** CONFIRMED.

### 9.2 Feature flags that gate rendering behavior

| Feature | Default | What it gates |
|---|---|---|
| `editor-core` | On | Base editor behavior. No direct rendering gate. |
| `editor-graphs`, `editor-materials`, `editor-terrain`, `editor-nav`, `editor-sim`, `editor-full` | Off | Panel-layer features. No direct rendering gate. |
| `fast-alloc` | **On** | Pulls in `astraweave-alloc` (mimalloc global allocator). Non-rendering. |
| `kittest` | Off | Test harness. |
| `terrain-splat-arrays` | Off | Gates the entire editor-side wrapper `EditorTerrainSplat` in `viewport/terrain_splat.rs`. When off, every method is a no-op stub returning `Ok(())` or `false`. Forwards the same feature name to `astraweave-render/terrain-splat-arrays`. 44 `cfg(feature = "terrain-splat-arrays")` and `cfg(not(feature = "terrain-splat-arrays"))` occurrences across terrain_splat.rs, engine_adapter.rs, mod.rs. |
| `impostor-bake` | **On** | Gates the `impostor_registry` and `impostor_wiring` modules (`viewport/mod.rs:49-55`). Also gates fields `impostor_registry` and `installed_impostor_keys` on `EngineRenderAdapter` (engine_adapter.rs:640-647). Forwards to `astraweave-render/impostor-bake`. Per `Cargo.toml` comment at line 31-35, disabling this feature degrades LOD3 to a no-op. |
| `profiling` | Off | Forwards to `astraweave-profiling/profiling`. Non-rendering. |

### 9.3 `[patch]` / `[replace]` sections

No `[patch]` or `[replace]` sections were found in either `tools/aw_editor/Cargo.toml` or the root `Cargo.toml`. The editor builds against the workspace-default `astraweave-render`. No dependency injection of an alternate render crate is present.

---

## 10. Appendix: incidental findings (not pursued)

1. **Tonemap-pass double application (potential).** `engine_adapter.rs:943, 982, 1012, 1034` sets `tonemap_operator: TonemapOperator::Aces` in every `PostProcessChain` supplied to the engine. `hdr_pipeline.rs:347-417` shows the engine's `tonemap_pass()` is the terminal step of its post-processing chain and writes tonemapped output. The editor then runs **its own** `tonemap.wgsl` pass (`renderer.rs:573-596`) on the HDR target. The comment at `renderer.rs:573-575` describes the editor pass as a "blit" but the shader is a full tonemap that re-applies ACES/etc. Whether this causes visible double-tonemapping depends on the target-format routing in `draw_into` when the output view is `Rgba16Float` rather than an sRGB surface — not verified in this audit. If the engine's tonemap runs and the editor's tonemap also runs, output colors will be incorrect. Recommend a follow-up bench that draws a known HDR gradient through `draw_into` into an `Rgba16Float` view and reads back whether the values are linear-HDR or already-tonemapped-LDR.

2. **Dead "Engine PBR" UI control (see §8).** The toolbar checkbox and keyboard shortcut that write `RenderMode` have no effect on rendering. The console log line at `main.rs:8939-8941` announces "Engine rendering enabled/disabled" while nothing actually changes.

3. **`tab_viewer` reorganization is undocumented in Architecture Map.** §11 lists `src/tab_viewer.rs` as "deleted" without noting that it became a directory (`src/tab_viewer/mod.rs`) with the same content relocated.

4. **Two `WeatherKind` enums in simultaneous use.** `viewport/types.rs:WeatherKind` is used inside the viewport/toolbar UI (`WeatherKind::from_weather_type`, `from_world_panel`), while `astraweave_render::WeatherKind` is used at the engine interface (`main.rs:343, 3964`, `widget.rs:2610`, `tab_viewer/mod.rs:1445-1450`). The mapping between them lives in `tab_viewer/mod.rs:1445-1450` as a hand-written `match`. No shared-crate `From`/`Into` impl was found.

5. **`EngineRenderAdapter` actual size vs. documented.** 3,985 LOC actual vs. ~740 LOC claimed in Architecture Map §6.1. The adapter now contains extensive LOD-refresh state (`refresh_scatter_lod`, `scatter_lod_camera_pos`, `scatter_lod_camera_yaw`, `scatter_last_refresh`, `terrain_height_grid`, `terrain_clusters`, `terrain_chunk_slot_map`, etc.) that is not described in the map.

6. **`renderer::RenderMode` is re-exported from `engine_adapter`** (`renderer.rs:48`: `pub use super::engine_adapter::RenderMode;`). `mod.rs:73` further re-exports it as `pub use renderer::RenderMode`. The public API thus contains the zombie enum.

7. **Panel-layer imports of `astraweave_render::*`** are minimal: `main.rs:343, 3964`, `subsystems/docking_sync.rs:169`, `tab_viewer/mod.rs:1445-1450`, `widget.rs:2610` are the only non-viewport files that touch `astraweave_render::*`. Everything else flows through `viewport::` re-exports or adapter method calls.

8. **The four `PostProcessChain` configurations** in `apply_quality_preset` (engine_adapter.rs:921-1039) are almost-duplicates: they each construct a full `PostProcessChain { ssao_enabled: .., ssr_enabled: false, ssgi_enabled: false, god_rays_enabled: false, auto_exposure_enabled: false, ... tonemap_operator: Aces }`. Repetition is a maintenance surface but not a divergence.

9. **Architecture Map §6.1 omits four viewport files**: `terrain_splat.rs`, `terrain_splat_builder.rs`, `impostor_registry.rs`, `impostor_wiring.rs`. The map lists 13 files; the directory has 15 Rust files + 3 WGSL.

10. **`HANDOFF.md:125`** explicitly lists Fix 27 Phases 2-4 (Shadow/IBL unification, Tonemap/post-processing unification, `EditorOverlayHooks` trait) as pending work estimated at 11-14 days. This corroborates the falsified claims in §2.

---

*End of report.*
