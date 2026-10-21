# Terrain Material Data-Flow Investigation

**Date**: 2026-04-19
**Mode**: Read-only static analysis. No code changes.
**Scope**: `tools/aw_editor/src/viewport/types.rs`, `tools/aw_editor/src/viewport/engine_adapter.rs`, `tools/aw_editor/src/viewport/terrain_splat*.rs`, `tools/aw_editor/src/terrain_integration.rs`, `astraweave-render/src/terrain.rs`, `astraweave-render/src/terrain_material_manager.rs`, `astraweave-render/src/mesh.rs`, `astraweave-render/src/renderer.rs`, `astraweave-render/shaders/*.wgsl`.
**Related prior work**: `docs/audits/editor_viewport_render_divergence_2026-04-19.md` §4, `docs/audits/tonemap_double_application_investigation_2026-04-19.md`.

---

## 1. Executive finding

Under default features, the editor's 80-byte `TerrainVertex` authoring (8 biome weights + 4 material IDs + 4 material weights per vertex) is **aggregated at the CPU to a single per-cluster histogram and then discarded**. The GPU receives only position, normal, tangent, and UV (the engine's `MeshVertex`, 48 bytes) plus a per-instance tint colour and a single shared texture set chosen by argmax of the aggregated material histogram. Each terrain cluster therefore renders as **one material with one tint** — no per-vertex biome lookup, no per-fragment material blend, no splat-map sampling.

The splat-map infrastructure (`EditorTerrainSplat`, `build_chunk_splat_maps`, `TerrainMaterialManager`, `pbr_terrain.wgsl`) exists but is **triply dormant**: (a) `terrain-splat-arrays` is not in any default feature set, (b) the engine-side `TerrainMaterialManager` module is `#![cfg(feature = "terrain-splat-arrays")]` and therefore compiled out entirely when the feature is off, and (c) `EditorTerrainSplat` has zero call sites outside its own module — it is never instantiated by `engine_adapter.rs`, `renderer.rs`, or `main.rs`. Even if the feature were turned on, the editor would still not use the splat path because no code constructs an `EditorTerrainSplat`. And even in that hypothetical, `terrain_splat_builder::build_chunk_splat_maps` reads only `biome_weights_0/1` — the `material_ids` / `material_weights` fields do not reach the GPU through the splat path either.

**Answer to the motivating question**: the editor's terrain renders as a near-monochromatic surface because the path from authoring to GPU collapses all per-vertex material variation to a single per-cluster (cluster = up to 4 chunks merged) material choice + tint. The rich authoring is not reaching any active shader.

---

## 2. Phase 1 — editor authoring

### 2.1 Three `TerrainVertex` definitions exist in the workspace

| Site | LOC | Size | Fields | Status |
|---|---|---:|---|---|
| `astraweave-render/src/terrain.rs:18-23` | 6 | ~28 B | `position[3]`, `normal[3]`, `uv[2]`, `biome_id: u32` | Engine type. Used only by `astraweave_render::TerrainRenderer` (a standalone terrain renderer) — **never by the editor path**. See §2.3. |
| `tools/aw_editor/src/viewport/types.rs:17-29` | 13 | ~80 B (96 B with `Pod` padding in the audit quote) | `position[3]`, `normal[3]`, `uv[2]`, `biome_weights_0[4]`, `biome_weights_1[4]`, `material_ids[4]`, `material_weights[4]` | Editor viewport type. The one the adapter consumes. |
| `tools/aw_editor/src/terrain_integration.rs:75-87` | 13 | ~80 B | Same fields as viewport version | Editor CPU-side type used by `astraweave-terrain` chunk generation. Converted to `viewport::TerrainVertex` field-by-field in `viewport/renderer.rs:1094-1107`. |

`Pod + Zeroable` impls are on all three. None of them have a `VertexBufferLayout` helper; the engine's `MeshVertex` (`astraweave-render/src/mesh.rs:7-12`) provides the only active terrain vertex layout via `MeshVertexLayout::buffer_layout()` at mesh.rs:46 — and the editor path uploads to that layout, not to any `TerrainVertex` layout.

### 2.2 Editor `TerrainVertex` construction sites

Searching `TerrainVertex\s*\{` across the workspace (production code only, excluding tests):

| File:Line | Site | Population source |
|---|---|---|
| `tools/aw_editor/src/terrain_integration.rs:778-786` | `TerrainState::build_chunk_mesh`, main chunk mesh generator | `biome_weights_0/1` from `BiomeBlender::packed_biome_to_weight_sets`; `material_ids`/`material_weights` from `SplatMapGenerator::splat_weights_to_material_slots` (see `terrain_integration.rs:765-769`) |
| `tools/aw_editor/src/terrain_integration.rs:823-831` | `add_skirt` closure, edge-skirt vertex generation | Copies all fields from the adjacent surface vertex |
| `tools/aw_editor/src/terrain_integration.rs:1465-1523` | `update_chunk_material` / brush material-update path | Overwrites `material_ids` and `material_weights` in place from brush inputs |
| `tools/aw_editor/src/terrain_integration.rs:2017-2095` | `paint_material_at` brush path | Manipulates `material_ids` and `material_weights` in-place using min-weight-slot replacement |
| `tools/aw_editor/src/viewport/renderer.rs:1094-1107` | `upload_terrain_chunks_raw` — converts `terrain_integration::TerrainVertex` → `viewport::TerrainVertex` | Field-by-field copy — no data loss at this hop |
| `astraweave-render/src/terrain.rs:107-112` | `TerrainRenderer::create_terrain_mesh` (engine-side standalone renderer) | Only `biome_id` (single u32). Not called by editor. |

Population summary for the editor's 80-byte vertex:

| Field | Type | Populated by | Semantic meaning |
|---|---|---|---|
| `position` | `[f32; 3]` | `astraweave-terrain` heightmap + chunk offset | World-space XYZ |
| `normal` | `[f32; 3]` | `astraweave-terrain` biome normal lookup at the vertex | Surface normal |
| `uv` | `[f32; 2]` | Originally world-space per-tile UV; **overwritten** to `position.xz * 0.125` in `engine_adapter.rs:1706-1710` before upload | Detail-texture UV |
| `biome_weights_0[4]` | `[f32; 4]` | `BiomeBlender::packed_biome_to_weight_sets` | Weights for biomes 0–3 (Grassland, Desert, Forest, Mountain) |
| `biome_weights_1[4]` | `[f32; 4]` | Same | Weights for biomes 4–7 (Tundra, Swamp, Beach, River) |
| `material_ids[4]` | `[f32; 4]` | `SplatMapGenerator::splat_weights_to_material_slots` (authored splat rules), or overwritten by brush paint (`terrain_integration.rs:2017+`) | Up to 4 material atlas layer indices in `[0, 22)` |
| `material_weights[4]` | `[f32; 4]` | Same source; sum normalised to 1.0 during painting (`terrain_integration.rs:2093-2098`) | Blend weights for the four material slots |

### 2.3 Engine `TerrainVertex` (the 28-byte one) is not in the editor's active path

`astraweave-render::TerrainRenderer` is constructed in workspace code only at:
- `astraweave-render/src/terrain.rs:279, 286, 301` (module-internal tests)
- `astraweave-render/tests/*.rs` (integration tests — 10+ sites)
- `examples/biome_showcase/src/main.rs:126`
- `examples/terrain_demo/src/main.rs:79`
- `examples/weaving_playground/src/main.rs:68`

No construction in `tools/aw_editor/` or `astraweave-render/src/renderer.rs::Renderer`. The editor path never touches `astraweave_render::TerrainRenderer`, and its 28-byte `TerrainVertex { position, normal, uv, biome_id }` is therefore inert in the editor.

`TerrainVertex::to_engine_vertex()` ([`tools/aw_editor/src/viewport/types.rs:35-63`](../../tools/aw_editor/src/viewport/types.rs#L35-L63)) — the conversion that picks the dominant biome weight and produces the 28-byte engine vertex — is called in exactly one place: `tools/aw_editor/benches/editor_performance.rs:181` (a benchmark). **It has no production call site.** The prior audit's claim that this conversion discards material data on the active path is misleading; the conversion is simply not invoked at all.

---

## 3. Phase 2 — per-vertex trace to the shader

### 3.1 Full trace table (editor → GPU)

| Stage | File:Line | Data at this stage | Notes |
|---|---|---|---|
| Editor authoring (CPU) | `terrain_integration.rs:75-109, 778-786` | 80 B: pos, normal, uv, 8 biome weights, 4 material IDs, 4 material weights | Full-fidelity authoring |
| Copy to viewport vertex | `viewport/renderer.rs:1094-1107` | 80 B: identical field-by-field copy | No loss |
| Adapter entrypoint | `engine_adapter.rs:1329` — `upload_terrain_chunks(&[(Vec<TerrainVertex>, Vec<u32>)])` | 80 B viewport vertices | Receives the full authoring |
| Per-chunk conversion | `engine_adapter.rs:1665-1729` — `convert_terrain_chunk` | Drops biome_weights_* and material_* at the vertex level; aggregates them into `surface_summary: TerrainSurfaceSummary { biome_weights[8], material_weights[22] }` (single histogram per chunk) | **First data-loss point for per-vertex material data.** See §3.3. |
| Per-cluster merge | `engine_adapter.rs:1526-1615` — `rebuild_terrain_cluster` | Merges up to 4 chunks (per `TERRAIN_CLUSTER_GRID = 2`); further aggregates surface_summary; picks ONE `dominant_material_index()` and ONE `resolve_tint()` | Second collapse: cluster-level single material + single tint |
| GPU mesh build | `engine_adapter.rs:1582-1588` / `1601-1607` — `renderer.create_mesh_from_full_arrays(positions, normals, tangents, uvs, indices)` | 48 B per vertex: pos, normal, tangent, uv (engine `MeshVertex` layout) | No biome_id, no material_ids, no material_weights |
| Engine mesh upload | `astraweave-render/src/renderer.rs:4315` — `create_mesh_from_full_arrays` | Builds a `Mesh` with `wgpu::Buffer` of `MeshVertex` bytes | `MeshVertex` layout at `astraweave-render/src/mesh.rs:7-52` |
| Engine model registration | `astraweave-render/src/renderer.rs:6602, 7092` — `add_model_with_bounds` / `add_model_sharing_texture_with_bounds` | Cluster becomes a named model in `self.models`, with per-instance colour (tint) and a shared texture bind group | Texture bind group chosen by prototype name |
| Prototype texture set | `engine_adapter.rs:1977-2009` — `ensure_terrain_material_prototype` | Loads `assets/materials/{name}.png` + `_n.png` + `_mra.png` for ONE material name from `MATERIAL_NAMES[dominant_material_index]`, attached via `ModelSurfaceMaps` | Editor binds ONE layer's albedo+normal+MR per cluster |
| Shader input (`SHADER_SRC`) | `astraweave-render/src/renderer.rs:22-35` (inline WGSL `VSIn`) | `position (loc 0)`, `normal (loc 1)`, `tangent (loc 12)`, `uv (loc 13)`, plus instance matrix rows and `color (loc 9)` | **No biome_id input. No material_ids input.** |
| Fragment material lookup | `SHADER_SRC` + `brdf_common.wgsl` — standard PBR with IBL, shadow cascades, cloud shadows | Samples `uMaterial.base_color`, bind-group-3 albedo/normal/MR textures, uScene fog/ambient/sun, IBL cubemaps | Single-material, single-tint per draw |
| Fragment output | Returns `aces_tonemap(lit_color)` — wait, **no**: the main shader returns linear HDR (tonemap is done downstream, per the tonemap investigation). Final fragment colour = PBR-lit, fog-applied, tint-applied, linear HDR | Written into `self.hdr_view`, picked up by the editor's tonemap pass |

### 3.2 Engine shader's vertex input — exact bindings

From `astraweave-render/src/renderer.rs:22-35` (inline WGSL `VSIn` inside `SHADER_SRC`):

```wgsl
struct VSIn {
    @location(0)  position: vec3<f32>,   // per-vertex
    @location(1)  normal:   vec3<f32>,   // per-vertex
    @location(12) tangent:  vec4<f32>,   // per-vertex
    @location(13) uv:       vec2<f32>,   // per-vertex
    @location(2)  m0: vec4<f32>,         // per-instance (model matrix row 0)
    @location(3)  m1: vec4<f32>,
    @location(4)  m2: vec4<f32>,
    @location(5)  m3: vec4<f32>,
    @location(6)  n0: vec3<f32>,         // per-instance (normal matrix row 0)
    @location(7)  n1: vec3<f32>,
    @location(8)  n2: vec3<f32>,
    @location(9)  color: vec4<f32>,      // per-instance tint
};
```

Matches the `MeshVertex` layout at `astraweave-render/src/mesh.rs:46-51`. **Zero locations for biome_id, material_ids, or material_weights.** The shader that the editor's terrain clusters are drawn with has no input for per-vertex material authoring.

### 3.3 What `TerrainSurfaceSummary` does with the authoring

`engine_adapter.rs:113-188` defines a per-chunk histogram that aggregates the authoring:

- `biome_weights: [f32; 8]` — sums `biome_weights_0[0..4]` + `biome_weights_1[0..4]` across every vertex in the chunk.
- `material_weights: [f32; 22]` — for each vertex, for each of its 4 `(material_id, material_weight)` slots, adds `weight` into the bucket at `material_id` (when `weight > 0` and `material_id.round() ∈ [0, 22)`).

This histogram is merged across all chunks in a cluster (engine_adapter.rs:1538-1555) and then reduced to two scalars:

- `dominant_material_index()` = `argmax` of `material_weights[]` (engine_adapter.rs:168-175). Used to pick a single prototype texture name.
- `resolve_tint()` = a weighted blend of two palette lookups: `TERRAIN_BIOME_TINTS[0..8]` with `biome_weights`, and `TERRAIN_MATERIAL_TINTS[0..22]` with `material_weights`, combined 65% biome + 35% material (engine_adapter.rs:177-187). Used as the per-instance vertex colour.

Per-vertex material blend weights therefore affect two things: (1) which single-layer texture set is chosen for the whole cluster, and (2) the cluster's constant tint colour. They do not control any per-fragment material variation.

---

## 4. Phase 3 — splat-map path trace

### 4.1 Splat builder input and output

[`tools/aw_editor/src/viewport/terrain_splat_builder.rs:39-79`](../../tools/aw_editor/src/viewport/terrain_splat_builder.rs#L39-L79) — `build_chunk_splat_maps`:

- **Input**: `&[TerrainVertex]` (viewport vertex, 80 B), plus chunk dimensions.
- **Reads**: `v.biome_weights_0[4]` and `v.biome_weights_1[4]` only.
- **Ignores**: `v.material_ids[4]` and `v.material_weights[4]` (see `encode_weight` and the two inner loops at lines 64-71 — they iterate `biome_weights_0/1` and nothing else).
- **Output**: two `RGBA8` buffers (`splat_0`, `splat_1`) of size `width * height * 4` bytes each, encoding each biome weight as a `u8` in `[0, 255]`.

So if the splat path were active, it would convey biome-weight data to the GPU but not material-ID/material-weight data.

### 4.2 Splat-map trace table

| Stage | File:Line | Data | Feature-gated? | Status |
|---|---|---|---|---|
| Editor splat build (CPU) | `viewport/terrain_splat_builder.rs:39-79` | Reads `biome_weights_0/1` only; writes two `RGBA8` buffers | **NO** — compiles unconditionally | Callable but never called in production (only from its own tests and, theoretically, `EditorTerrainSplat::upload_chunk_from_vertices`) |
| Editor splat wrapper | `viewport/terrain_splat.rs` (whole file) | `EditorTerrainSplat` wraps `TerrainMaterialManager`; holds chunk count, init flag, material-uploaded flag | **YES** — most methods are `#[cfg(feature = "terrain-splat-arrays")]`; the `#[cfg(not)]` counterparts are no-op stubs | **Never constructed anywhere** outside of its own tests. Grep: `EditorTerrainSplat::new\|EditorTerrainSplat\s*\{` shows zero production call sites. |
| Splat manager upload | `astraweave-render/src/terrain_material_manager.rs` (whole file) | Owns 4× `texture_2d_array<f32>` (albedo, normal, ORM, height, each with 8 layers), per-chunk splat maps, a `TerrainMaterialGpu` uniform, camera uniform, bind groups | **YES** — line 36: `#![cfg(feature = "terrain-splat-arrays")]` — entire module compiled out when feature is off | Inert without feature |
| Splat shader | `astraweave-render/shaders/pbr_terrain.wgsl` + `pbr_terrain_vs.wgsl` | Vertex input at `TerrainSplatVertex::LAYOUT` (terrain_material_manager.rs:142-150): pos[3], normal[3], uv[2] — 32 B; no biome/material IDs. Fragment bindings (`pbr_terrain.wgsl:47-55`): `camera`, `terrain` params, `splat_map_0`, `splat_map_1`, `layer_albedo/normal/orm/height` arrays | Pipeline is feature-gated via `TerrainMaterialManager::ensure_pipeline` | Would sample splat maps if active; the shader itself is a file on disk regardless of feature, but nothing compiles its pipeline |
| Splat pipeline | `TerrainMaterialManager::ensure_pipeline` | Builds the render pipeline with `TerrainSplatVertex::LAYOUT`, 3 bind group layouts | **YES** | Never called unless feature on and `EditorTerrainSplat::initialize` invoked, which itself has no call sites |
| Fragment material blend | `pbr_terrain.wgsl:63+` | Splat maps → per-fragment blend weights → 8-layer texture arrays → PBR output | — | Would run only if the splat pipeline were built and its `draw_chunk` invoked — neither happens in the editor |

### 4.3 Feature-flag state

Defaults in both `Cargo.toml` files:

- `astraweave-render/Cargo.toml:8`: `default = ["postfx", "textures"]` — **no `terrain-splat-arrays`**.
- `astraweave-render/Cargo.toml:24`: `terrain-splat-arrays = []` — defined, off.
- `tools/aw_editor/Cargo.toml:17`: `default = ["editor-core", "impostor-bake", "fast-alloc"]` — **no `terrain-splat-arrays`**.
- `tools/aw_editor/Cargo.toml:30`: `terrain-splat-arrays = ["astraweave-render/terrain-splat-arrays"]` — defined, off.

No other editor-relevant `Cargo.toml` enables the feature. No binary build config in the workspace opts in.

Full `cfg(feature = "terrain-splat-arrays")` occurrences (44 in 3 files, per prior audit and `rg -c`):

- `astraweave-render/src/terrain_material_manager.rs:36` — whole module
- `astraweave-render/src/lib.rs:107, 253` — module declaration + public re-export
- `tools/aw_editor/src/viewport/terrain_splat.rs` — 30+ per-method gates, each paired with a stub `#[cfg(not(...))]` that returns `Ok(())` / `false` / `0` / etc.

Zero occurrences in `engine_adapter.rs`, `renderer.rs`, or `main.rs`. The feature's effect is entirely contained within the splat module itself; no other part of the editor branches on it.

### 4.4 Call-site analysis for `EditorTerrainSplat`

```bash
rg -n 'EditorTerrainSplat' --type rust
```
returns matches only from `tools/aw_editor/src/viewport/terrain_splat.rs` itself (docstrings, struct definition, tests). There is no field of type `EditorTerrainSplat` on any struct in the editor. It is not constructed in `main.rs`, `engine_adapter.rs`, `renderer.rs`, `widget.rs`, or anywhere else.

The wrapper is **feature-gated code that, if compiled, still has no user**. Even flipping `terrain-splat-arrays` to `on` would not activate the splat path in the editor — a call site would need to be added.

---

## 5. Phase 4 — which path actually runs under default features

### 5.1 Determinations

1. **Is the per-vertex material path wired to a shader that consumes per-vertex material IDs?** **No.** The editor's 80-byte `TerrainVertex` is collapsed at `engine_adapter.rs:1665-1729 (convert_terrain_chunk)` into per-cluster histograms plus plain `MeshVertex`-format geometry. The shader the GPU runs (`SHADER_SRC` at renderer.rs:18-445) has no input for biome_id or material_ids. `astraweave_render::TerrainVertex` exists with a `biome_id: u32` field but no shader consumes a terrain-specific vertex layout in the editor's path — `TerrainRenderer` is not used.

2. **Is the splat-map path wired to a shader that samples splat textures?** **No.** `TerrainMaterialManager` would drive that path, but (a) its module is `#![cfg(feature = "terrain-splat-arrays")]` — compiled out; (b) even if compiled in, `EditorTerrainSplat` (its editor-side wrapper) has zero production call sites; (c) even if wrapper were constructed, the splat shader `pbr_terrain.wgsl` would receive `biome_weights_0/1` only — not `material_ids`/`material_weights`.

3. **Under default feature flags, which terrain shader actually runs when the editor renders a terrain chunk?** The engine's main PBR shader `SHADER_SRC` (defined inline in `astraweave-render/src/renderer.rs:18-445`, composed from `shaders/constants.wgsl` + `shaders/brdf_common.wgsl` + inline WGSL). Vertex format is `MeshVertex` (mesh.rs:7-12). Per-cluster inputs to this shader that drive material variation: (a) the bind-group-3 material textures (ONE set of albedo/normal/MR per cluster, chosen by dominant material index via `add_model_sharing_texture_with_bounds`), (b) the per-instance `color` attribute at location 9 (the blended biome+material tint). The `uMaterial.base_color` uniform at bind group 1 binding 0 is the engine's global material — same for all clusters unless changed.

### 5.2 Reality statement

Under default features, the editor's terrain is drawn by the engine's main PBR shader (`SHADER_SRC`). This shader receives per-vertex position, normal, tangent, and UV — no per-vertex biome or material identifier — plus per-instance model matrices and a tint colour. For each terrain cluster (up to 4 authored chunks merged), the editor picks the single dominant material from the aggregated 22-bucket histogram, loads that material's albedo/normal/metallic-roughness PNGs from `assets/materials/`, and binds them as the cluster's sole texture set. The cluster's per-instance tint is a 65/35 blend of the biome-palette average and the material-palette average. The result is that each cluster renders as one material with one tint, with no spatial material variation inside the cluster beyond what the single texture set's tiling provides (world-space UV at `position.xz * 0.125`, i.e. one texture tile every 8 metres). This is why the terrain reads as monochromatic.

---

## 6. Phase 5 — design-gap characterization

### 6.1 Authored but discarded

| Data | Authored at | Aggregated at (first collapse) | Final use |
|---|---|---|---|
| Per-vertex `biome_weights_0[4]`, `biome_weights_1[4]` (8 weights) | `terrain_integration.rs:760-764`, `778-786` | Summed into `TerrainSurfaceSummary.biome_weights[8]` per chunk at `engine_adapter.rs:121-133` | Used only to compute half of the per-cluster tint (`resolve_tint`, engine_adapter.rs:177-187). Per-vertex weights never reach the GPU. |
| Per-vertex `material_ids[4]` | `terrain_integration.rs:765-769` (from splat rules) or `terrain_integration.rs:2017+` (brush paint) | Used as histogram index at `engine_adapter.rs:135-148` to accumulate into `material_weights[22]` per chunk | Drives `dominant_material_index()` which picks ONE prototype texture per cluster. Per-vertex IDs discarded. |
| Per-vertex `material_weights[4]` | Same as above | Same | Drives the per-cluster material palette tint. Per-vertex weights discarded. |
| Per-vertex UV authored | `terrain_integration.rs:774-776` (world-space × chunk-size normalisation) | Overwritten at `engine_adapter.rs:1706-1710` with `position.xz * 0.125` | The authored UV is never used; UV is recomputed at upload. |

### 6.2 Built but dormant

| Item | File:Line | Why dormant |
|---|---|---|
| `astraweave_render::TerrainMaterialManager` (whole module) | `astraweave-render/src/terrain_material_manager.rs:36` — `#![cfg(feature = "terrain-splat-arrays")]` | Feature off in default |
| `astraweave_render` re-exports of manager | `astraweave-render/src/lib.rs:107, 253` — `#[cfg(feature = "terrain-splat-arrays")]` | Same |
| `pbr_terrain.wgsl` + `pbr_terrain_vs.wgsl` | `astraweave-render/shaders/pbr_terrain*.wgsl` (files on disk) | Consumed only by the feature-gated `TerrainMaterialManager::ensure_pipeline` |
| `EditorTerrainSplat` wrapper | `tools/aw_editor/src/viewport/terrain_splat.rs` (whole file, ~357 lines) | No production call sites anywhere in the editor — not even behind the feature flag. Would need wiring into `engine_adapter.rs::upload_terrain_chunks` to become active. |
| `TerrainSplatVertex` layout | `astraweave-render/src/terrain_material_manager.rs:132-151` | Pipeline that uses it is feature-gated |
| `astraweave_render::TerrainVertex` (28-byte engine vertex) | `astraweave-render/src/terrain.rs:18-23` | Used only by `TerrainRenderer`, which is constructed only in tests and standalone examples. Not used by the editor's `EngineRenderAdapter`. |
| `astraweave_render::TerrainRenderer` (standalone terrain renderer) | `astraweave-render/src/terrain.rs:26+` | Same — never instantiated by the editor |
| `TerrainVertex::to_engine_vertex()` conversion | `tools/aw_editor/src/viewport/types.rs:35-63` | Production call sites: zero. Only called by `benches/editor_performance.rs:181`. The editor's actual flow goes directly from viewport `TerrainVertex` to `MeshVertex` via `convert_terrain_chunk`, bypassing this conversion entirely. |

### 6.3 Missing

Each item below is what would have to exist for the editor's per-vertex or splat-map authoring to reach a fragment shader; none of these currently exist in the codebase.

| Missing item | Where it would need to live |
|---|---|
| A shader vertex input that reads per-vertex biome_weights or material_ids | The `VSIn` struct in `SHADER_SRC` (or a new terrain-specific shader) would need additional `@location(N)` fields, plus a matching `MeshVertex` extension or a dedicated `TerrainVertex` layout wired into a terrain pipeline |
| A fragment path that samples multiple albedo/normal/MR textures and blends per fragment | Either (a) activate `pbr_terrain.wgsl` (splat path), or (b) author a new 4-way-material-blend shader that consumes per-vertex `material_ids` and `material_weights` directly |
| A wiring call that constructs `EditorTerrainSplat` and drives `upload_chunk_from_vertices` from `engine_adapter.rs::upload_terrain_chunks` | `engine_adapter.rs` around line 1371-1375 (the chunk-accept loop) |
| A feature flag default that enables `terrain-splat-arrays` | `tools/aw_editor/Cargo.toml:17` default = [...] and/or a per-binary override |
| A splat-builder extension that writes `material_ids`/`material_weights` to GPU (if the intent is per-vertex 4-way material blending rather than 8-biome blending) | `terrain_splat_builder.rs::build_chunk_splat_maps` currently reads only `biome_weights_0/1`. A new splat builder — or an extension of this one to output additional maps — would be needed to convey the material data. |
| A fragment pathway that reads per-vertex material tangent-space detail maps (normal mapping per material, slope-based material selection) | Not present in either the main shader or the splat shader — both use a single normal map |

---

## 7. Appendix — incidental findings

1. **Editor UV authoring is always overwritten**. `terrain_integration.rs:774-776` computes `[tiled_u, tiled_v]` = `[world_x / 256.0, world_z / 256.0]`, but `engine_adapter.rs:1706-1710` replaces the UV with `[position.x * 0.125, position.z * 0.125]` = one tile per 8 m. The authored UV value is never consumed. Comment at engine_adapter.rs:1682-1710 explains this was an intentional fix for the "flat green" look but calls out that "the PROPER fix is the splat-array pipeline (Phase 2.2 of the editor fidelity plan)".

2. **The comment at `engine_adapter.rs:1700-1705` states it explicitly**: "*Per-vertex biome/material weights remain the sole source of tint variation via `TerrainSurfaceSummary::resolve_tint`.*" This is the CPU-side acknowledgement that per-vertex material data is used for tint only, not per-fragment shading — matching this audit's trace.

3. **`ARCHITECTURE_MAP.md` §4.5 misstates the conversion**. The map says: "`TerrainVertex::to_engine_vertex()` — extracts dominant biome from 8 weight slots, discards per-vertex material_ids" — implying this conversion runs. The conversion is present in source but not in the active data path. The effective vertex conversion is `convert_terrain_chunk` in `engine_adapter.rs:1665-1729`, which doesn't use `to_engine_vertex()` at all.

4. **Three `TerrainVertex` types make refactoring brittle**. Adding a new field to the editor's `viewport::TerrainVertex` requires touching `terrain_integration::TerrainVertex` (same 80-byte layout, field names must match for the field-by-field copy at `viewport/renderer.rs:1094-1107`) and potentially the splat builder. The engine's 28-byte `TerrainVertex` is disconnected but its existence invites confusion. The prior audit also noted this — both in §4 and as an incidental finding.

5. **`TerrainRenderer` in the engine is unused dead code from the editor's perspective**. It has 160+ LOC and is maintained via tests and standalone examples. Not investigated further: whether any runtime code outside `tools/aw_editor` uses it (a full cross-crate trace would be needed).

6. **`terrain_integration.rs:TerrainVertex::new` does not validate `material_weights` sum to 1.0 or `material_ids` are in `[0, 22)`**. The brush-paint path normalises weights (line 2093-2098), but the initial splat-map-derived population (line 765-769 via `splat_weights_to_material_slots`) relies on the callee for sanity. The `TerrainSurfaceSummary::add_vertex` path defensively handles out-of-range IDs (`engine_adapter.rs:140-148`) — which means bad authoring can pass through silently. Not a bug on the current path because nothing downstream reads per-vertex material data; would matter if the path were activated.

7. **`set_terrain_surface_maps` writes to the engine's GLOBAL material textures**. `engine_adapter.rs:1890-1906` calls `self.renderer.set_albedo_from_rgba8(...)` etc. — these modify the engine-wide default albedo/normal/MR (`self.tex_bg` bindings in the renderer). The prototype-sharing path at `ensure_terrain_material_prototype` creates per-cluster bind groups via `ModelSurfaceMaps`, which is the more precise path. So two texture paths coexist: a global default (used by anything without an override, including the ground fill plane) and the per-prototype cluster textures. Not a divergence, but worth noting as separate concerns.

8. **Blend ratio `0.65` for biome-vs-material tint is magic-numbered** at `engine_adapter.rs:182`: `blend_tints(biome, material, 0.65)`. No configuration surface; silently tuned. Not a bug — noted as a constant that would need re-authoring if the tint approach were replaced by per-fragment material blend.

9. **Several prior docs claim the editor "authors rich terrain material data" that is "discarded at the to_engine_vertex boundary"**. This audit refines that: the discarding happens in `convert_terrain_chunk` at `engine_adapter.rs:1665-1729`, not in `to_engine_vertex`. The end result (no per-vertex material reaches GPU) is the same, but the docs' pointer to the wrong loss site would mislead anyone trying to fix it by changing `to_engine_vertex`.

---

*End of report.*
