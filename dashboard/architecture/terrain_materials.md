---
schema_version: 1
trace_id: terrain_materials
title: "Terrain Material System"
description: "Terrain Material System (canonical reference example)"
primary_crate: astraweave-terrain
domain: physics-world
lifecycle_status: active
integration_status: wired
summary: "Terrain material slice: 8-slot canonical biome pack + loader, splat bake, 32-layer GPU blend (complements terrain.md). terrain_materials.md"
owns: []
doc_version: "1.6"
last_verified_commit: c0753b551
---

# Architecture Trace: Terrain Material System

## Metadata

| Field | Value |
|---|---|
| **System name** | Terrain Material System |
| **Primary crates** | `astraweave-render`, `astraweave-terrain`, `tools/aw_editor` |
| **Document version** | 1.6 |
| **Last verified against commit** | L.3 working tree over `d6f53c00e` (v1.6: §2 Stage 3 bind-group table completed — the L.2 group(3) IBL row was missing (two-beat drift) and the L.3 group(4) shadow-receive row + depth-only caster pipeline added; see `docs/audits/L3_OUTCOME.md`); prior `bf57b5f1d` (v1.5: T.2c — real PBR materials for slots 0/1/4; see the v1.5 addendum, the slot table rows and Invariant 9. The T.2c beat commit follows this one); prior `c0753b551` (v1.4: T.2a Phase-1 aux-channel repair — see §8 Invariant 8 and the Revision note); prior `7e52c290c`+T.1 worktree (v1.3: slot-6 beach material executed — T-series ratification §2 row-6 amendment); prior `8232b150b` (v1.2, T.0 trace-sync: canonical-pack era — E3 build `d506658d8` + AD.4/AD.5.A re-points; see Revision note in §2.0 and the Appendix B addendum); `67c9de7e1` |
| **Last verified date** | 2026-07-24 (v1.4 T.2a aux-channel repair); 2026-07-21 (v1.3 slot-6 update; v1.2 full pass); 2026-05-10 (full trace) |
| **Status** | Active (canonical 32-layer pipeline) with transitional legacy residue |
| **Owner notes** | Canonical reference example for the architecture trace campaign. Derived from forensic data-flow analysis on 2026-05-11. |

---

## 1. Executive Summary

**What this system does:**
Authors terrain material assignments on the CPU side as sparse per-vertex material slot data, bakes that data into dense splat textures, and consumes the splat textures at fragment time to blend across a canonical 32-layer material library producing the final terrain surface shading.

**Why it exists:**
Provides AAA-parity terrain rendering with rich per-vertex material authoring informing per-fragment PBR material blending.

**Where it primarily lives:**
- `astraweave-render/` — canonical 32-layer runtime pipeline (material manager, shaders, material library)
- `tools/aw_editor/src/viewport/` — authoring representation and CPU bake step
- `astraweave-terrain/` — biome classification, legacy 8-layer splat code, terrain mesh generation

**Status note:**
The active editor → splat-bake → fragment-shader pipeline is structurally clean and end-to-end coherent. The codebase contains additional terrain abstractions — biome semantics, an older 8-layer procedural splat system, and a simpler single-biome-id render path — that coexist with the canonical 32-layer system. These are sources of cognitive friction but do not invalidate the active architecture.

---

## 2. Authoritative Pipeline

### 2.0 The canonical-pack era (E3 build `d506658d8`, 2026-07-03 + AD.4/AD.5.A re-points — v1.2 addition)

Two upstream stages landed after v1.1 and now feed Stage 1; both verified first-hand at `8232b150b`:

**(a) Canonical material content — the 8-slot biomes pack.** `assets/materials/biomes/{materials.toml, arrays.toml}` defines the live 8-layer terrain material set; `arrays.toml` fixes layer index = slot and its header names `biome_id_to_slot()` as its contract (`arrays.toml:2-3`). Slots at HEAD (all files verified on disk; every slot uses the `mra` key):

| Slot | Key | Albedo (resolves to) | Tiling | Note |
|---|---|---|---|---|
| 0 | grassland | `assets/materials/derived_1k/grass.png` | 128 | **T.2c (2026-07-25): real material EXECUTED.** Was a synthetic flat-green albedo (lap 3.62) paired with the normal map of `grass_medium_01`, an **alpha-cutout foliage card** whose transparent-region normals shaded as hard black shards. Now PolyHaven `aerial_grass_rock` via `cook_family_from_maps` (true-MRA, measured R=0.00/sd 0.00). Provenance §15; outcome `docs/audits/T2C_OUTCOME.md`. Stem stays `grass` — see Invariant 9 |
| 1 | desert | `assets/materials/derived_1k/sand.png` | 128 | **T.2c (2026-07-25): real material EXECUTED.** Was 100% procedural (`tools/pbr_gen`) with a normal map at sd **0.07** (no relief at all). Now PolyHaven `sand_01`; normal sd 0.07 -> 17.41, albedo lap 5.55 -> 17.62. Chosen to stay distinct from slot 6's damp coastal sand (T.1 row-6 amendment) — `aerial_sand` was rejected for being coastal. Stem stays `sand` |
| 2 | forest | `assets/materials/derived_1k/tree_leaves.png` | 128 | AD.5.A Fix-1 re-point (2026-07-15) off gitignored `_downloaded/`; same upstream `forest_leaves_02`, true-MRA (in-file comment, `materials.toml:40-47`) |
| 3 | mountain | `assets/materials/mountain_rock.png` | **64** | one repeat / 8 m vs 4 m for ground slots |
| 4 | tundra | `assets/materials/derived_1k/snow.png` | 128 | **T.2c (2026-07-25): real material EXECUTED.** Was 100% procedural, albedo sd 2.16 / normal sd **0.02**. Now PolyHaven `snow_02`; normal sd 0.02 -> 3.99. `snow_04` was rejected despite winning every variance metric — it is a plowed field whose furrows would tile as directional stripes. Stem stays `snow` |
| 5 | swamp | `assets/materials/mud.png` | 128 | |
| 6 | beach | `assets/materials/derived_1k/beach.png` | 128 | **T.1 (2026-07-21): distinct material EXECUTED** (was byte-identical to slot 1) — PolyHaven `coast_sand_01` cooked via `cook_family_from_maps` (true-MRA, measured R=0.0); provenance `THIRD_PARTY_LICENSES.md` §14, evidence `docs/audits/evidence/t1_beach_2026-07-21/`, outcome `docs/audits/T1_BEACH_OUTCOME.md`. Slot 6 has no MaterialLibrary palette name → biome-paint-only by design (paintable set stays 7; `aw_editor.md` Invariants 26/27 unaffected) |
| 7 | river | `assets/materials/derived_1k/gravel.png` | 128 | ratified 2026-07-20 as honest **riverbed**; water surfacing is the T.W beat pair |

Loaded by `tools/aw_editor/src/viewport/canonical_terrain_pack.rs` (`load_canonical_terrain_pack`): paths join onto the biome dir; slot order is `arrays.toml`-driven; **channel-key semantics** (`canonical_terrain_pack.rs:178-184`): an `orm` key loads verbatim (already AO-R/rough-G/metal-B, e.g. PolyHaven ARM), an `mra` key (legacy metal-R/rough-G/AO-B) is swizzled to ORM at load via `load_mra_as_orm_bytes` (`:221-227`, per-pixel R↔B swap); `orm` wins when both present. The lowercase albedo file stem is retained per layer (`albedo_stem`) as the palette-remap join key. Upload path: `EngineRenderAdapter::reupload_terrain_layers_from_pending_pack` → `set_terrain_materials`, re-resolving the palette remap on every upload (see `aw_editor.md` v1.4/v1.5 + Invariants 26/27 there).

**(b) Biome-driven per-vertex material authoring — E3-terrain.1.** Worldgen's per-vertex Whittaker `BiomeId` (see `terrain.md` §2 Stage 2) reaches Stage 1 via `tools/aw_editor/src/terrain_integration.rs`: the exhaustive 19→8 `biome_id_to_slot` (`:1250-1274`) + `build_biome_slot_field` (slope-rock overlay toward slot 3 + 1-ring blur, `:1293-1370`) produce the per-vertex `material_ids[4]`/`material_weights[4]` (`:837-840` → `TerrainVertex::new` `:861-867`). This RESOLVES v1.1's §4 "[NEEDS VERIFICATION — exact interface not traced]" worldgen touchpoint. The pre-E3 single-`primary_biome` splat-rule path (`create_local_splat_generator`, `:1372+`) survives as the fallback when per-vertex biome data is absent (`:797-802`, `:841-852`) and behind the "Regenerate Splatmaps" panel action (`:1748+`).

```text
[CPU authoring / editor]
    │
    │ TerrainVertex authored with material_ids[4], material_weights[4]
    ▼
[Stage 1: Authoring representation]
    file: tools/aw_editor/src/viewport/types.rs
    role: Canonical editor-side terrain vertex format
    key data: position, normal, uv, material_ids[4], material_weights[4]
    │
    │ build_chunk_splat_maps(vertices, width, height)
    ▼
[Stage 2: Sparse-to-dense bake]
    file: tools/aw_editor/src/viewport/terrain_splat_builder.rs
    role: Bridge from sparse per-vertex material data to dense splat textures
    key data: 8 RGBA8 splat textures (32 channels total) per chunk
    │
    │ set_chunk_splat_forward(chunk, splats, dims)
    ▼
[Stage 3: GPU upload and binding]
    file: astraweave-render/src/terrain_material_manager.rs
    role: Runtime GPU management for terrain layer arrays + per-chunk splats
    key data: bound bind groups, uploaded splat textures, shared material library
    │
    │ draw_chunk_forward(...)
    ▼
[Stage 4: Vertex stage]
    files: astraweave-render/shaders/pbr_terrain_vs.wgsl,
           astraweave-render/src/terrain_material_manager.rs
    role: Pass spatial basis (position, normal, uv) to fragment stage
    key data: interpolated world position, normal, uv
    note: Material slot data is NOT carried in the render vertex format —
          it has already been baked into splat textures
    │
    ▼
[Stage 5: Fragment material evaluation]
    file: astraweave-render/shaders/pbr_terrain.wgsl
    role: Reconstruct material weights from splat textures and blend layers
    key data: per-fragment material blend
    │
    ▼
[Final shaded terrain pixel]
```

### Stage-by-stage detail

#### Stage 1: Authoring representation
**File:** `tools/aw_editor/src/viewport/types.rs`
**Role:** Defines the canonical editor-side `TerrainVertex` format.
**Inputs:** Authoring tools, worldgen, biome system outputs.
**Outputs:** `TerrainVertex` instances with sparse 4-slot material data.
**Notes:** Each vertex carries up to 4 `(material_id, material_weight)` pairs describing which canonical material layers contribute at that point and by how much. Comments in this file indicate that older `biome_weights_0/1` fields and the newer `material_ids/material_weights` were unified into a single canonical material attribute set.

#### Stage 2: Sparse-to-dense bake
**File:** `tools/aw_editor/src/viewport/terrain_splat_builder.rs`
**Role:** Converts sparse per-vertex material assignments into dense splat textures.
**Inputs:** Slice of `TerrainVertex` for a chunk plus chunk dimensions.
**Outputs:** `ChunkSplatMaps` containing 8 RGBA8 textures encoding 32 material-layer weights.
**Notes:** Allocates a dense `channels[32]` per texel (line 95), accumulates sparse `(id, weight)` entries into that vector (lines 96-100), and packs the result into 8 RGBA8 textures (lines 102-106). Weights are encoded to `u8 [0..255]`. File comments at lines 17-24 document the switch from biome-field sources to material-field sources.

#### Stage 3: GPU upload and binding
**File:** `astraweave-render/src/terrain_material_manager.rs`
**Role:** Owns the runtime GPU resources for terrain material rendering.
**Inputs:** Baked splat textures, material library content.
**Outputs:** Bound bind groups ready for terrain draw calls.
**Notes:** Owns the shared material library (albedo, normal, ORM, height texture arrays plus uniform config) and per-chunk splat textures. Bind group layout for the forward pipeline (verified against `terrain_material_manager.rs:466-545` and `draw_chunk_forward` at `terrain_material_manager.rs:1229-1251`):
- Group 0: camera UBO (`forward_camera_bg`)
- Group 1: `TerrainMaterialGpu` UBO + `TerrainSceneEnvGpu` UBO + filtering sampler + 3 layer texture arrays (albedo, normal, ORM; height omitted in Phase 1 forward path)
- Group 2: per-chunk splat textures — 8 splat textures (`splat_map_0..7`) + ClampToEdge sampler
- Group 3 (L.2, 2026-07-27): terrain IBL — spec cube, irradiance cube, BRDF LUT, sampler, `IblParams` UBO. Layout from the shared `TerrainMaterialManager::create_terrain_ibl_bgl`; bind group renderer-owned (`terrain_ibl_bind_group`), rebuilt only in `rebuild_ibl_bind_group`, bound once before the chunk loop in `draw_into`.
- Group 4 (L.3, 2026-07-28): CSM shadow receive — `MainLightUbo` (cascade matrices/splits/extras sentinel), 2-layer depth array, comparison sampler. Layout from the shared `TerrainMaterialManager::create_shadow_bgl`; the bound group is the renderer's existing `light_bg` (the static path's group 2 — one owner, §7.7). Fragment stage totals 15/16 sampled textures.
- L.3 also adds the depth-only shadow-caster pipeline (`ensure_shadow_pipeline`: `TerrainSplatVertex` position-only, single per-cascade mat4 UBO at group 0, no instancing) — terrain chunks draw into both cascade layers of the renderer's shadow passes with per-cascade AABB culling (`TerrainChunkGpu.aabb_*`, computed at upload).

#### Stage 4: Vertex stage
**Files:** `astraweave-render/shaders/pbr_terrain_vs.wgsl`, `astraweave-render/src/terrain_material_manager.rs`
**Role:** Interpolates spatial basis for the fragment stage.
**Inputs:** `TerrainSplatVertex { position, normal, uv }`.
**Outputs:** Interpolated world position, world normal, uv at each fragment.
**Notes:** The render-side vertex format does NOT carry material IDs or weights. Those have already been baked into the per-chunk splat textures before reaching the GPU.

#### Stage 5: Fragment material evaluation
**File:** `astraweave-render/shaders/pbr_terrain.wgsl`
**Role:** Reconstructs per-fragment material weight vector and blends layers.
**Inputs:** Splat textures, material library texture arrays, interpolated UV/normal.
**Outputs:** Final shaded terrain pixel.
**Notes:** Computes slope from interpolated normal, decides whether to use triplanar sampling, samples `splat_map_0..7` at fragment UV, reconstructs a `raw_weights[32]` vector, normalizes across active layers, and blends material layer albedo/normal/ORM contributions using the normalized weights.

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **Material layer** | A render material entry in the canonical library, indexed 0-31, containing albedo/normal/ORM/height textures and config | `astraweave-render/src/material_library.rs`, fragment shader |
| **Splat texture** | An RGBA8 texture where each channel represents a weight for one material layer | `terrain_splat_builder.rs`, `pbr_terrain.wgsl` |
| **Material slot** | One of 4 `(material_id, material_weight)` pairs per vertex in the authoring representation | `tools/aw_editor/src/viewport/types.rs` |
| **Biome** | An ecological/regional classification (Grassland, Desert, Forest, Mountain, Tundra, Swamp, Beach, River) — 8 categories in the elevation-biome model | `astraweave-terrain/src/elevation_biome.rs` |
| **Chunk** | Spatial subdivision of the terrain that owns its own splat textures | `terrain_material_manager.rs` |
| **Active layer count** | Runtime-configurable number of currently-meaningful material layers (≤32), used to normalize fragment weights | `pbr_terrain.wgsl`, `TerrainMaterialGpu` uniform |
| **Triplanar sampling** | Sampling textures from three world-axis planes and blending by surface normal, used for steep terrain | `pbr_terrain.wgsl` |

### Terms to NOT confuse

- **Biome vs material**: Biome semantics describe ecological regions ("this is forest"). Material semantics describe render surface layers ("this fragment is 60% grass_short + 40% wet_soil"). A biome may imply material assignments but is not itself a material-weight vector.
- **8-layer splat (legacy) vs 32-layer splat (active)**: The `astraweave-terrain/src/texture_splatting.rs` module uses `MAX_SPLAT_LAYERS = 8`. The active editor → render pipeline uses 32 canonical layers packed into 8 RGBA textures. The word "splat" alone is ambiguous; specify which system.
- **`biome_id` (single-biome path) vs `material_ids[4]` (rich path)**: The simple render path in `astraweave-render/src/terrain.rs` uses a single `biome_id: u32` per vertex. The canonical path uses sparse 4-slot weighted material assignments. These are different fidelities, not equivalent.
- **Material authoring representation vs render transport representation**: Sparse per-vertex slots and dense splat textures are two encodings of the same material intent. They are NOT competing semantic systems in the active path.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| Worldgen / biome classification | `chunk.biome_ids()` → `biome_id_to_slot` + `build_biome_slot_field` (`terrain_integration.rs:1250-1370`) → per-vertex `material_ids/material_weights` | Per-vertex Whittaker `BiomeId` → pack-layer slot weights | **RESOLVED at v1.2** (was NEEDS VERIFICATION): the E3-terrain.1 path, see §2.0(b). Fallback when biome data absent: elevation-driven weights + single-`primary_biome` splat rules |
| Editor authoring tools | Direct construction of `TerrainVertex` | Material slot assignments | `tools/aw_editor/src/viewport/types.rs` |
| Material library content | Texture arrays bound at startup | Albedo / normal / ORM / height textures for 32 layers | `astraweave-render/src/material_library.rs` |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| Render pipeline | `draw_chunk_forward(...)` | Bound terrain draw calls | `terrain_material_manager.rs` |
| Simpler legacy render path | `TerrainVertex::to_engine_vertex()` | Single-biome-ID vertex format | Adapter in `tools/aw_editor/src/viewport/types.rs` collapses rich material data to dominant biome ID |

### Bidirectional / Coupled

- **GPU resource lifecycle**: The terrain material manager and the broader render pipeline share lifecycle of texture arrays, samplers, and bind groups. Changes to material library schema affect both.

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `tools/aw_editor/src/viewport/types.rs` | Canonical editor terrain vertex format | Active | Defines `TerrainVertex`; also contains `to_engine_vertex()` collapse adapter |
| `tools/aw_editor/src/viewport/terrain_splat_builder.rs` | Sparse-to-dense splat bake | Active | Bridge between authoring and runtime representations |
| `astraweave-render/src/terrain_material_manager.rs` | Runtime GPU resource management | Active | Owns layer arrays and per-chunk splats |
| `astraweave-render/shaders/pbr_terrain.wgsl` | Fragment material blend shader | Active | Authoritative runtime blend logic |
| `astraweave-render/shaders/pbr_terrain_vs.wgsl` | Terrain vertex stage | Active | Pass-through spatial basis |
| `astraweave-render/src/terrain_material.rs` | GPU schema / descriptors for terrain materials (`TerrainLayerGpu`, `TerrainMaterialGpu`) | Active | Verified — defines `TerrainLayerGpu` (64 B) and `TerrainMaterialGpu` (2112 B = 32 × 64 + 64); imported by `terrain_material_manager.rs` and matched byte-for-byte by `pbr_terrain.wgsl`. Capacity tied to `material_library::MAX_TERRAIN_LAYERS = 32` (see file header lines 1-12) |
| `astraweave-render/src/material_library.rs` | Canonical material layer library | Active | Library truth for the 32-layer count (`MAX_TERRAIN_LAYERS = 32` at line 32; `NUM_SPLAT_MAPS = 8` at line 39) |
| `astraweave-terrain/src/elevation_biome.rs` | Height/climate-driven biome generation | Active (biome layer, not material layer) | Produces 8-slot biome weights; semantically distinct from material layers |
| `astraweave-terrain/src/biome.rs` | `BiomeType` enum (8 variants) and biome configuration types | Active (biome layer) | Verified — defines `BiomeType`, `BiomeConfig`, `BiomeVegetation`, `VegetationType`; consumed by `biome_blending.rs`, `biome_pack.rs`, and editor code |
| `astraweave-terrain/src/biome_blending.rs` | Multi-biome interpolation with `MAX_BLEND_BIOMES = 4` and GPU-friendly packed weights | Active (biome layer) | Verified — defines `BiomeBlender`, `BiomeWeight`, `BiomeBlendConfig`, `PackedBiomeBlend`; module header documents purpose at lines 1-7 |
| `astraweave-terrain/src/biome_pack.rs` | Data-driven asset pack format bridging the `.blend` decomposition pipeline (`manifest.json`) to terrain biome/scatter | Active (biome layer) | Verified — defines `BiomePack`, `BiomePackAsset`, `BiomePackScatter`; consumed by `tools/aw_editor` panels and tests |
| `astraweave-terrain/src/biome_param_blending.rs` | Phase 1.6-F.4.B.3.D.4 scattered-convolution blending of biome parameters | Active (biome layer) | Verified — module header (lines 1-30) documents jittered-sample blending of `mountains_amplitude` / `scatter_density` while preserving discrete `BiomeId` per vertex |
| `astraweave-terrain/src/biome_parameters.rs` | Phase 1.6-F.4.B.3.D.3 per-`BiomeId` terrain parameter table (replaces `BiomeNoisePreset`) | Active (biome layer) | Verified — module header (lines 1-30) documents climate→biome→parameter lookup chain; partially wired (`mountains_amplitude` wired, `ridge_strength` defined but not yet wired) |
| `astraweave-terrain/src/texture_splatting.rs` | Older 8-layer procedural splat system | **Legacy-fallback (editor)** — v1.2 correction | v1.1's "zero production call sites" is superseded: `tools/aw_editor/src/terrain_integration.rs` imports `SplatMapGenerator`/`SplatRule`/`SplatWeights` (`:4`) and `create_local_splat_generator` (`:1372+`) drives (a) the no-biome-data fallback mesh path (`:797-802`) and (b) the "Regenerate Splatmaps" panel action (`:1748+`). On the live E3 path (per-vertex biome data present) it is NOT consulted. See Section 11 |
| `astraweave-render/src/terrain.rs` | Simpler single-`biome_id` render path (`TerrainRenderer`, `TerrainVertex`, `TerrainMesh`) | Transitional | Verified — `pub use terrain::{TerrainMesh, TerrainRenderer, TerrainVertex, VegetationRenderInstance}` at `astraweave-render/src/lib.rs:145`. Production callers outside tests: only `examples/weaving_playground/src/main.rs:6` (`RenderTerrainRenderer`). Not used by editor or any in-engine subsystem. See Section 11 |
| `TerrainVertex::to_engine_vertex()` in `tools/aw_editor/src/viewport/types.rs` | Adapter collapsing rich vertex to simple engine vertex | Deprecated (bench-only) | Workspace grep for `.to_engine_vertex(` returns only the definition at `types.rs:41` and a single call from `tools/aw_editor/benches/editor_performance.rs:179`. Zero production call sites — consistent with `docs/audits/terrain_material_flow_investigation_2026-04-19.md:222` which documents the bypass via `convert_terrain_chunk` |
| `tools/aw_editor/src/viewport/terrain_biome_placeholder.rs` | Placeholder biome-colored terrain materials (8 flat-color swatches; slot order matches `biome_id_to_slot`) | Fallback (no-pack) | v1.2: the "Phase 3 real materials" plan is DONE — the canonical biomes pack (§2.0(a)) is the replacement. The placeholder now serves only when no pack loads (`engine_adapter.rs:628` comment; import at `:1415`). Slot 7's muted-blue swatch (`:35`) is the origin of the mapping's "only blue slot" rationale — the texture pack later made slot 7 gravel (riverbed, ratified 2026-07-20) |
| `tools/aw_editor/src/viewport/canonical_terrain_pack.rs` | Canonical 8-slot biome pack loader (`load_canonical_terrain_pack`; `mra`→ORM swizzle / `orm` verbatim; `albedo_stem` retention) | Active (v1.2 addition) | See §2.0(a). Loader tests incl. `loads_grassland_pack_when_present`, `loads_biomes_pack_forest_slot_from_derived_1k` are the Tier-2 machine floor (close-out addendum 2026-07-19) |
| `tools/aw_editor/src/viewport/palette_remap.rs` | Paint-palette→pack-layer name remap (manual painting only; biome-driven assignment bypasses it) | Active (v1.2 addition) | Owned in detail by `aw_editor.md` (v1.4 entry + Invariants 26/27); listed here because it consumes this trace's `albedo_stem` surface |

**Status definitions used here:**
- **Active**: Canonical, load-bearing, edit with care
- **Transitional**: Active but its long-term role is unresolved; pending decision (see Section 11)
- **Deprecated (test-only)** / **Deprecated (bench-only)**: Exported and compiled, but zero non-test/non-bench call sites in the workspace. Candidate for removal pending Section 11 resolution

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Notes |
|---|---|---|---|
| Biome semantics (8-slot ecological) | `elevation_biome.rs`, `biome*.rs` | Active (separate layer) | Serves worldgen/ecology, not render layer blending |
| Material semantics (32-layer canonical) | `material_library.rs`, `terrain_material_manager.rs`, `pbr_terrain.wgsl` | Active (canonical) | The render-time material representation |
| Older 8-layer procedural splat | `texture_splatting.rs` | Deprecated (test-only) | Same word "splat" but different capacity and generation model; no production callers |
| Single-biome-ID render path | `terrain.rs`, `to_engine_vertex()` | `terrain.rs` Transitional (one example caller), `to_engine_vertex()` Deprecated (bench-only) | Lossy collapse of rich material data |
| Biome-named material placeholders | `terrain_biome_placeholder.rs` | Transitional | Encourages mental conflation of biome with material |

### Naming collisions

- **"Splat"**: In `astraweave-terrain/src/texture_splatting.rs`, refers to an 8-layer procedural splat system with rule-based generation. In the active render pipeline (`terrain_splat_builder.rs`, `terrain_material_manager.rs`, `pbr_terrain.wgsl`), refers to the 32-layer canonical splat textures baked from per-vertex material data. Without qualification, "splat" is ambiguous.
- **"Material"**: In `astraweave-render` generally refers to canonical 32-layer material library entries. In `terrain_biome_placeholder.rs` is used to map biome names into material indices, creating semantic overlap between biome identity and material identity.
- **"Biome"**: Used in `astraweave-terrain` for ecological/regional classification. Used in `terrain.rs` and `terrain_biome_placeholder.rs` in a manner that conflates with material identity.

### Known cognitive traps

- **Trap**: Reading `texture_splatting.rs` and assuming it represents the active render pipeline.
  **What's actually true**: It is a separate, older 8-layer system. The active 32-layer pipeline lives in `astraweave-render`. Verified workspace-wide grep for `texture_splatting::`, `SplatMapGenerator`, and `SplatRule` returns only test-file callers — no production call sites — so this module is dormant (test-only) and a candidate for deprecation.
- **Trap**: Treating the historical audit at `docs/audits/terrain_material_flow_investigation_2026-04-19.md` as current truth.
  **What's actually true**: That audit documents an earlier inconsistent state where the splat builder read `biome_weights_0/1` and ignored `material_ids/material_weights`. The current builder reads `material_ids/material_weights`. The audit is useful historical context but does not describe current behavior.
- **Trap**: Assuming the simple `biome_id` render path in `astraweave-render/src/terrain.rs` is the canonical terrain render path because it appears in the render crate.
  **What's actually true**: The canonical render path uses the splat-textured material system in `terrain_material_manager.rs`. The `terrain.rs` path is a lower-fidelity alternative. Verified current use: a single production caller in `examples/weaving_playground/src/main.rs:6` (aliased `RenderTerrainRenderer`), plus dedicated tests in `astraweave-render/tests/`. No editor or in-engine subsystem uses it.

---

## 7. Decision Log

### Decision: Use 32-layer canonical material library
- **Date:** [Reasoning not recovered from available sources — predates current trace]
- **Status:** Accepted (in active code)
- **Context:** [Reasoning not recovered from available sources]
- **Decision:** The terrain material system uses 32 canonical material layers packed into 8 RGBA8 splat textures.
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** Replaces an earlier 8-layer model (still present in `texture_splatting.rs`). Permits richer material variation per fragment at the cost of larger per-chunk texture storage.

### Decision: Sparse 4-slot per-vertex material authoring
- **Date:** [Reasoning not recovered from available sources]
- **Status:** Accepted (in active code)
- **Context:** [Reasoning not recovered]
- **Decision:** Each `TerrainVertex` carries up to 4 `(material_id, material_weight)` pairs.
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** Limits to 4 contributing materials per vertex while keeping vertex storage bounded. Per-fragment blending of more than 4 materials is achievable because adjacent vertices can carry different slot assignments, blended in the bake step.

### Decision: Splat textures as runtime representation
- **Date:** [Reasoning not recovered from available sources]
- **Status:** Accepted (in active code)
- **Context:** [Reasoning not recovered]
- **Decision:** Per-vertex material data is baked into per-chunk RGBA8 splat textures at CPU side; the fragment shader consumes splat textures, not per-vertex material attributes.
- **Alternatives considered:** [Reasoning not recovered — plausible alternative would be passing material slot data through vertex shader interpolation, but this was not chosen]
- **Consequences:** Decouples authoring ergonomics (sparse, easy to edit) from runtime ergonomics (dense texture sampling, GPU-friendly). Adds a bake step between authoring and rendering.

### Decision: Unify `biome_weights_0/1` and `material_ids/material_weights` into a single canonical material attribute set
- **Date:** [Reasoning not recovered — visible in code comments per file analysis]
- **Status:** Accepted (visible in `tools/aw_editor/src/viewport/types.rs` comments and `terrain_splat_builder.rs` lines 17-24)
- **Context:** Prior state (documented in `docs/audits/terrain_material_flow_investigation_2026-04-19.md`) had `biome_weights_0/1` and `material_ids/material_weights` as separate vertex fields, with the splat builder reading the biome path and ignoring the material path.
- **Decision:** The vertex format was unified so that `material_ids/material_weights` is the canonical material attribute set, and the splat builder now reads from it.
- **Alternatives considered:** [Reasoning not recovered]
- **Consequences:** Eliminated the prior mismatch where authoring set material weights but the renderer used biome weights. The biome layer continues to exist for ecological/worldgen purposes but no longer drives splat generation.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | The fragment shader consumes splat textures only; per-vertex material slot data is not present in the render vertex format | Yes (inspect vertex struct + shader binding) | Doc-only currently |
| 2 | `material_weights` for a vertex should be normalizable to sum ≤ 1.0 after slot expansion | Yes | Partially enforced. The brush-paint upstream path in `tools/aw_editor/src/terrain_integration.rs:2140-2146` explicitly renormalizes per-vertex weights to sum = 1.0 after each stroke. The downstream bake (`terrain_splat_builder.rs:118-125`) clamps each weight to `[0.0, 1.0]` individually but does NOT constrain the per-vertex sum, and inline tests use weights summing to 3.25 (line 175) and pass. Final normalization happens in `pbr_terrain.wgsl` via per-fragment `raw_weights[32]` normalize across active layers. NEEDS VERIFICATION on whether non-brush upstream paths (worldgen seeding, biome system) also normalize |
| 3 | Per-chunk splat textures correspond 1:1 with the chunk's vertex data and dimensions | Yes (the bake produces them together) | Structural (single function) |
| 4 | The 32 material layers in the canonical library are global/shared; per-chunk variation is encoded only in splat texture weights | Yes (inspect material library + manager) | Doc-only currently |
| 5 | Material layer indices in `material_ids[i]` must be valid indices into the canonical material library (0-31) | Yes | Enforced by silent-drop bounds check in `terrain_splat_builder.rs:97-100` (`if layer >= 0 && (layer as usize) < max_layers`); out-of-range layers are dropped rather than asserted. Verified by inline tests (`clamps_out_of_range_weights`, `encodes_high_layer_weights_in_higher_splats` at lines 186-201 and 204+) — these confirm layer 32 and layer -1 produce no contribution |
| 6 | Pack slot order is `arrays.toml`-driven and matches `biome_id_to_slot` 1:1 — the three surfaces (arrays.toml indices · `biome_id_to_slot` match arms · `terrain_biome_placeholder` slot order) move together or biome coloring breaks silently (v1.2) | Yes (greppable + compile-time) | `arrays.toml:2-3` contract comment; exhaustive match (new `BiomeId` variant = compile error); `aw_editor.md` Invariant 27 |
| 7 | Aux-channel semantics at load: an `orm` key is verbatim (AO-R/rough-G/metal-B); an `mra` key is R↔B-swizzled to ORM; `orm` wins when both present. A mislabeled file (ARM bytes under an `mra` key or vice versa) double-flips channels and zeroes AO / inflates metal — the AD.4.A D1 defect class (v1.2) | Yes | `canonical_terrain_pack.rs:178-184`, `load_mra_as_orm_bytes` `:221-227`; per-file channel measurement per the close-out lesson ("ARM/MRA channel keys must be evidenced per file, never pattern-copied") |
| 8 | **No aux channel of a live pack slot may be a placeholder constant, and AO must rise with height.** A channel whose modal value covers > 90% of pixels (or whose IQR is 0) is a placeholder, not data — standard deviation does **not** detect this (`grass_mra` roughness had sd 13.43 while 99.5% of it was exactly 255). Metallic is the sole exception: a hard constant 0 is the post-D1 dielectric contract. Separately, occlusion rises with displacement — a peak is exposed — so an AO derivation must be positively correlated with its height source (T.2a; the shipped `1.0 - h` inversion had reached `mud_mra` at r = +0.991) | Yes | `tools/material_cook/channel_stats.py` (modal/IQR detector); `tools/material_cook/test_cook_1k.py::test_ao_orientation_and_normal_integration` pins the orientation of both AO derivations |
| 9 | **A pack layer's palette identity is its ALBEDO FILE STEM, not its path or its `key`.** `palette_remap.rs::resolve` joins each `MaterialLibrary` entry name to a layer by `albedo_stem.eq_ignore_ascii_case(name)`, so re-pointing a slot to a new file is safe **only if the stem is preserved**. Changing slot 0/1/4's stems from `grass`/`sand`/`snow` would silently drop the paintable set from 7 entries to 4 — no error, no warning, the UI simply greys them out. This is why T.2c cooked to `derived_1k/{grass,sand,snow}.png` rather than to slug-named files (v1.5). Related: `assets/materials/grass.png` is additionally load-bearing as the `find_assets_dir()` sentinel (`viewport/types.rs:224,235`) and must not be deleted as "unused" | Yes | `palette_remap.rs::biomes_pack_resolves_exactly_seven_entries` asserts the paintable set `[0,1,3,4,5,12,20]`; `canonical_terrain_pack.rs::loads_biomes_pack_forest_slot_from_derived_1k` asserts the full 8-stem set against the live pack on disk |

---

## 9. Performance & Resource Profile

### Hot paths
- **Fragment shading**: Per fragment, samples 8 splat textures plus up to 32 material layer texture arrays (albedo, normal, ORM each). Cost scales with active layer count. Normalization across active layers happens per fragment.
- **Triplanar branch**: Triggered by slope from interpolated normal; cost increases on steep terrain.

### Cold paths
- **Splat bake**: Runs CPU-side at chunk authoring or chunk update time, not per frame. Cost scales with chunk dimensions × max material layers.
- **GPU upload**: Runs per chunk update, not per frame.

### Resource ownership
- **Shared material library** (texture arrays + uniform): owned globally by `terrain_material_manager.rs`. Lifetime = engine lifetime (or render context lifetime).
- **Per-chunk splat textures**: owned per chunk by the terrain material manager. Lifetime = chunk lifetime.
- **`TerrainVertex` authoring data**: owned by the editor / worldgen side; converted into splat textures and then no longer needed at render time.

---

## 10. Testing & Validation

- **Unit tests (inline `#[cfg(test)]` modules):**
  - `astraweave-render/src/terrain_material.rs` — 16 tests
  - `astraweave-render/src/terrain_material_manager.rs` — 12 tests
  - `astraweave-render/src/material_library.rs` — 9 tests
  - `tools/aw_editor/src/viewport/terrain_splat_builder.rs` — 12 tests (covering sparse→dense bake, out-of-range layer clamping, encoding boundaries)
  - `tools/aw_editor/src/viewport/terrain_biome_placeholder.rs` — 4 tests
- **Integration tests (`astraweave-render/tests/`):**
  - `terrain_splat_pipeline.rs`
  - `test_terrain_material.rs`
  - `wave2_ssao_texture_terrain_material_remediation.rs`
  - `wave2_terrain_quad_registry_overlay_remediation.rs`
- **Editor mutation/integration tests:** `tools/aw_editor/tests/mutation_resistant_terrain.rs`
- **Mutation testing (Wave 2):** Per `docs/current/MUTATION_WAVE2_PLAN.md:51`, `texture_splatting.rs` was scoped at 152 mutants (P1). Per `docs/current/MUTATION_TESTING_REMEDIATION_REPORT.md:182`, 6 `texture_splatting_tests` were added for `MAX_SPLAT_LAYERS` and weight normalization. Wave 2 remediation also covered `astraweave-terrain` partition-splatting and splatmap-voxel paths (see `wave2_partition_splatting_remediation.rs`, `wave2_shard20_modifier_persistence_splat.rs`, `wave2_shard21_splatmap_voxel_remediation.rs`).
- **Visual validation:** Editor viewport divergence audits and tonemap investigations have been run historically; some terrain rendering issues (normal maps sampled as sRGB, broken tangent space fallbacks, splat material ID mismatch) were identified and addressed in prior work. The most recent terrain-material capacity audit (Real-Fix.D, 2026-05-08) bumped layer count from 8 to 32 per Andrew-gate decision (h) Option D-2 and is documented inline in `terrain_material.rs`, `pbr_terrain.wgsl`, and `terrain_splat_builder.rs` headers.

---

## 11. Open Questions / Parked Decisions

- **Is `astraweave-terrain/src/texture_splatting.rs` still load-bearing anywhere, or is it dead code awaiting deletion?** Resolving this determines whether the file should be marked Deprecated, fully removed, or scoped to a specific procedural-generation role distinct from the active 32-layer pipeline.
  - *2026-05-10 verification note:* Workspace-wide grep for `texture_splatting::`, `SplatMapGenerator`, `SplatRule`, `SplatWeights`, `TriplanarWeights`, and `use astraweave_terrain::texture_splatting` returns ZERO production callers. All non-self consumers are in `astraweave-terrain/tests/` (mutation-resistant suites and Wave 2 remediation tests) or `astraweave-terrain/src/mutation_tests.rs`. The module is still `pub mod texture_splatting` and `pub use texture_splatting::…` in `astraweave-terrain/src/lib.rs:42,97`. This is consistent with Deprecated (test-only) status pending Andrew decision.
- **What is the role of `astraweave-render/src/terrain.rs` and the `to_engine_vertex()` adapter?** Is the simple single-`biome_id` render path intended as a permanent LOD/fallback, a transitional artifact awaiting removal, or something else?
  - *2026-05-10 verification note:* `astraweave-render/src/terrain.rs` has exactly one production caller workspace-wide: `examples/weaving_playground/src/main.rs:6`. The editor and all other in-engine subsystems use `terrain_material_manager.rs` instead. `TerrainVertex::to_engine_vertex()` has zero production callers and one bench caller (`tools/aw_editor/benches/editor_performance.rs:179`), consistent with the prior audit at `docs/audits/terrain_material_flow_investigation_2026-04-19.md:222`.
- **Should `terrain_biome_placeholder.rs` be relocated or renamed?** Its current location in `tools/aw_editor/src/viewport/` and its biome-named slots may encourage confusion between biome identity and material identity.
- **Are there machine-checkable enforcements for invariants 2 and 5 in Section 8?** If not, should they be added as debug-assert checks in the splat builder?
- **Should the historical audit at `docs/audits/terrain_material_flow_investigation_2026-04-19.md` be explicitly marked as historical** to prevent future readers from treating it as current truth?
- ~~**Cross-system touchpoint with worldgen / biome classification**~~ **RESOLVED at v1.2**: the interface is the E3-terrain.1 path — `chunk.biome_ids()` → `biome_id_to_slot` + `build_biome_slot_field` → per-vertex `material_ids/material_weights` (§2.0(b), §4 upstream row).
- **v1.2 note on the `texture_splatting.rs` question above:** the "zero production callers" premise changed — the editor's fallback/`Regenerate Splatmaps` paths now call it (§5 row). The deprecation question becomes: retire it when the fallback path is removed, or keep it as the no-biome-data degradation? Rides with terrain beat T.2/T.G scoping.

---

## 12. Maintenance Notes

**Update this doc when:**
- Any file in Section 5 marked Active is structurally modified (new fields, new pipeline stages, changed interfaces)
- A decision in Section 7 is superseded by new code or new docs
- An invariant in Section 8 is broken, relaxed, or newly enforced
- An item in Section 11 (Open Questions) is resolved — move resolution into the appropriate section and remove from Open Questions
- A transitional file in Section 5 is migrated to Active, deleted, or has its role clarified

**Verification process:**
- Spot-check the pipeline diagram in Section 2 against current code in the cited files
- Verify the file map in Section 5 still reflects actual file roles
- Update the metadata commit hash and date after verification

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**

1. **The fragment shader consumes splat textures, not vertex material attributes.** Any change that tries to pass material IDs/weights through to the fragment stage via vertex interpolation is fighting the architecture. Material data is baked into textures upstream.
2. **Biome ≠ material.** Biome is an ecological classification used by worldgen. Material is a render layer identity. They live in different parts of the code and answer different questions.
3. **"Splat" is ambiguous in this codebase.** Always check whether you're looking at the active 32-layer pipeline (in `astraweave-render` and `tools/aw_editor`) or the older 8-layer procedural system (in `astraweave-terrain/src/texture_splatting.rs`).
4. **The historical audit doc describes a prior state, not current behavior.** Cite current code, not the audit, for claims about how the system works today.

**Files you'll most likely touch:**
- `tools/aw_editor/src/viewport/types.rs` (vertex format)
- `tools/aw_editor/src/viewport/terrain_splat_builder.rs` (bake logic)
- `astraweave-render/src/terrain_material_manager.rs` (GPU management)
- `astraweave-render/shaders/pbr_terrain.wgsl` (fragment blend)

**Files you should NOT touch without strong reason:**
- `astraweave-terrain/src/texture_splatting.rs` — transitional/unclear status; do not extend or build on this without first resolving Section 11's question about its role
- `astraweave-render/src/terrain.rs` — transitional/unclear status; do not extend the simple-biome-id path without first resolving its long-term role
- The historical audit doc — read for context, do not treat as current spec

**Common mistakes when changing this system:**
- **Mistake**: Trying to add per-vertex material attributes to the render vertex format.
  **Why wrong**: Material data is baked into splat textures upstream. The render vertex carries only spatial basis. Adding material attrs duplicates state.
- **Mistake**: Treating biome data as if it were material data (or vice versa).
  **Why wrong**: They are separate semantic layers serving different purposes. Conflating them produces confused code and tickles the legacy collapse path.
- **Mistake**: Adding a new file named with the word "splat" or "material" without checking which existing system it conceptually belongs to.
  **Why wrong**: The naming collisions in this area are already a source of friction. New files should be unambiguous about whether they belong to the active 32-layer pipeline, the legacy 8-layer system, or the biome layer.

---

## Appendix B: Historical context

The current architecture is the result of a unification: at an earlier stage, terrain vertices carried both `biome_weights_0/1` and `material_ids/material_weights` as separate fields, with the splat builder reading the biome path and ignoring the material path. The audit doc `docs/audits/terrain_material_flow_investigation_2026-04-19.md` documents this prior state forensically. The fields were subsequently unified so that `material_ids/material_weights` is the canonical material attribute set and the splat builder now reads from it. The biome layer (in `astraweave-terrain`) continues to exist for worldgen and ecological purposes but no longer drives splat generation.

The 32-layer canonical material system in `astraweave-render` represents a separate evolution from the older 8-layer procedural splat system in `astraweave-terrain/src/texture_splatting.rs`. Both still exist in the codebase; their relationship is one of the open questions in Section 11.

**v1.5 addendum (2026-07-25, beat T.2c — real materials for the three synthetic slots).** The
director's T.2a-gate verdict on slots 0/1/4: they "read as flat and shiny ... they definitely don't
read as proper PBR materials". T.2a had already surfaced why and ruled it an art-direction purchase
rather than a defect repair (its §3.1-3.2): slot 0 was a synthetic flat-green albedo paired with the
normal map of `grass_medium_01`, an **alpha-cutout foliage card** whose transparent-region normals
shaded as hard black shards; slots 1 and 4 were 100% procedural with normals at sd 0.07 and 0.02 —
i.e. no relief whatsoever. All three are now API-verified CC0 PolyHaven ground scans
(`aerial_grass_rock`, `sand_01`, `snow_02`), cooked via `cook_family_from_maps` (rough+ao, so the
ARM-order trap is avoided by construction) with **R metallic = 0.00 / sd 0.00** measured on all
three and **0 flat channels** pack-wide. The black shards are gone. Two consequences are recorded as
the director's calls rather than as wins: the tundra now reads considerably darker (a real snow
scan's de-lit albedo is a 165 grey where the procedural was 231 near-white), and the hex-tile
lattice is now *visible* on tundra because the flat material previously had no contrast to reveal
it — exactly what T.2a predicted. Slot stems were deliberately preserved (Invariant 9). Two
non-material findings: the `generate_attribution_file` overwrite bug did **not** fire this time (the
5.C fix holds — 18 -> 21 slugs, zero lost), and **T.2a's tundra station no longer frames tundra**,
because T.2a's own Phase 3 reclassified the ground under the pin — station coordinates are only
valid for the classification that produced them. Full evidence, per-channel tables, rejected
candidates and station A/B: `docs/audits/T2C_OUTCOME.md`.

**v1.4 addendum (2026-07-24, beat T.2a — Phase 1 data repair, commit `c0753b551`).** The pack's
aux content was measurably corrupt and had been since the traced-9 import. Two independent defects
in `scripts/import_terrain_textures.py`, both now fixed at the root: (a) `load()` clamped 16-bit
(`I;16`) sources to 255 instead of rescaling, flattening slot 0's roughness to 99.5%-constant 255
and slots 3/5's AO to the constant 140 (= `0.55 x 255`, the curve's floor at uniform height) — the
same class as the AD.4.A "D2" defect that `cook_1k.py::to_l8` had already fixed on the other cook
path; (b) `build_mra` computed AO as `0.55 + 0.45*blur(1 - h)`, **inverted**, darkening peaks and
lighting crevices — measured against same-scan ground truth at r = +0.478/+0.230/+0.122 upright vs
negative inverted, and the inversion had shipped into `mud_mra` at r = +0.991. The `else: 217`
flat-constant fallback was removed outright: a constant is indistinguishable downstream from
measured data. Three slots were surgically re-cooked (`_mra` only; albedo and normal untouched) —
slot 0 roughness + AO, slot 3 AO, slot 5 AO — taking the pack from **3 flat channels to 0**.
`cook_1k.py` gained `ao_from_displacement` / `ao_from_normal_map` (Frankot-Chellappa, slope-sign
calibrated against the two families that ship a real displacement) / `ao_from_albedo_cavity` /
`write_mra` / `roughness_from_mra`, plus `channel_stats.py` as the measurement instrument. Full
evidence, station A/B and the two surfaced art-direction findings (slot 0's albedo is a synthetic
flat green paired with an alpha-cutout card's normal map; slots 1 and 4 are 100% procedural) are in
`docs/audits/T2A_OUTCOME.md`.

**v1.2 addendum (2026-07-21, T.0 trace-sync).** Between v1.1 and v1.2 the material *content* story changed twice without this trace being updated: the E3 build (`d506658d8`, 2026-07-03) introduced the canonical 8-slot biomes pack + `canonical_terrain_pack.rs` loader + the biome-driven per-vertex authoring path (E3-terrain.1) and wired hex-tile stochastic sampling / mip chains / aniso-8 on the render side (that half is traced in `render_pipeline_material_system_shader_infrastructure.md` v1.10); the AD campaign then re-pointed pack slots onto shipped `derived_1k/` cooks (AD.4 `06780433d`, AD.5.A `21bc53333`) and added the paint-palette remap (`ae9b98ef3`). The build session paid no trace debt; the reconstruction lives in `docs/audits/E3_PREFLIGHT_2026-07.md` and the director's dispositions in `docs/audits/T_SERIES_RATIFICATION_2026-07-20.md`. This v1.2 pass verified every claim it restates first-hand at `8232b150b`.
