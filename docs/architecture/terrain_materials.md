---
schema_version: 1
trace_id: terrain_materials
title: "Terrain Material System"
description: "Terrain Material System (canonical reference example)"
primary_crate: astraweave-terrain
domain: physics-world
lifecycle_status: active
integration_status: wired
summary: "Voxel meshing/biome/noise/scatter/streaming (complements terrain_materials.md). terrain.md"
owns: []
doc_version: "1.1"
last_verified_commit: 67c9de7e1
---

# Architecture Trace: Terrain Material System

## Metadata

| Field | Value |
|---|---|
| **System name** | Terrain Material System |
| **Primary crates** | `astraweave-render`, `astraweave-terrain`, `tools/aw_editor` |
| **Document version** | 1.1 |
| **Last verified against commit** | `67c9de7e1` |
| **Last verified date** | 2026-05-10 |
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
| Worldgen / biome classification | [NEEDS VERIFICATION — exact interface not traced] | Biome assignments influencing material slot selection | `astraweave-terrain/src/elevation_biome.rs` produces biome data; how it flows into editor material slot assignment was not fully traced |
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
| `astraweave-terrain/src/texture_splatting.rs` | Older 8-layer procedural splat system | Deprecated (test-only) | Verified — `pub mod texture_splatting` is exported from `astraweave-terrain/src/lib.rs:42,97`, but workspace-wide grep for `texture_splatting::`, `SplatMapGenerator`, `SplatRule` returns ONLY test-file callers (`mutation_resistant_comprehensive_tests.rs:988`, `wave2_shard20_modifier_persistence_splat.rs:11`). Zero production call sites. See Section 11 |
| `astraweave-render/src/terrain.rs` | Simpler single-`biome_id` render path (`TerrainRenderer`, `TerrainVertex`, `TerrainMesh`) | Transitional | Verified — `pub use terrain::{TerrainMesh, TerrainRenderer, TerrainVertex, VegetationRenderInstance}` at `astraweave-render/src/lib.rs:145`. Production callers outside tests: only `examples/weaving_playground/src/main.rs:6` (`RenderTerrainRenderer`). Not used by editor or any in-engine subsystem. See Section 11 |
| `TerrainVertex::to_engine_vertex()` in `tools/aw_editor/src/viewport/types.rs` | Adapter collapsing rich vertex to simple engine vertex | Deprecated (bench-only) | Workspace grep for `.to_engine_vertex(` returns only the definition at `types.rs:41` and a single call from `tools/aw_editor/benches/editor_performance.rs:179`. Zero production call sites — consistent with `docs/audits/terrain_material_flow_investigation_2026-04-19.md:222` which documents the bypass via `convert_terrain_chunk` |
| `tools/aw_editor/src/viewport/terrain_biome_placeholder.rs` | Placeholder biome-colored terrain materials | Transitional | Verified — imported by `tools/aw_editor/src/viewport/engine_adapter.rs:1434` behind the `terrain-splat-arrays` feature flag; uploads 8 placeholder biome material texture sets to `TerrainMaterialManager`. Phase 3 plans to replace placeholders with real materials loaded from `assets/materials/{biome}/` (see comments at `engine_adapter.rs:1426-1431`) |

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
- **Cross-system touchpoint with worldgen / biome classification**: the exact interface by which biome data influences editor material slot assignment was not fully traced and should be documented when this trace is next updated.

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