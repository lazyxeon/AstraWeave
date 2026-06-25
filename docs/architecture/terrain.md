---
schema_version: 1
trace_id: terrain
title: "Terrain System (Generation, Voxels, Biomes, Noise, Scatter, Streaming)"
description: "Terrain System — voxel meshing, biomes, noise, scatter, streaming (complements `terrain_materials.md`, which is the material-splat slice)"
primary_crate: astraweave-terrain
domain: physics-world
lifecycle_status: active
integration_status: mixed
owns: [astraweave-terrain]
doc_version: "1.1"
last_verified_commit: 7c29b8182
---

# Architecture Trace: Terrain System (Generation, Voxels, Biomes, Noise, Scatter, Streaming)

## Metadata

| Field | Value |
|---|---|
| **System name** | Terrain System (procedural generation, voxel meshing, biome/noise pipeline, scatter, chunk streaming) |
| **Primary crates** | `astraweave-terrain` (with reverse dep into `astraweave-gameplay`); consumers in `tools/aw_editor`, `examples/hybrid_voxel_demo`, `astraweave-render` |
| **Document version** | 1.1 |
| **Last verified against commit** | `7c29b8182` |
| **Last verified date** | 2026-06-25 |
| **Status** | Active (editor heightmap path wired; voxel-meshing, streaming/LOD, and multi-archetype paths are in-design-but-tested / dormant — see §5, §6) |
| **Owner notes** | Complements [`terrain_materials.md`](terrain_materials.md), which covers ONLY the material/splat-weight slice. This trace covers the REST: noise → heightmap → biome → erosion → chunk generation, voxel meshing, scatter/vegetation, and chunk streaming. Where the two overlap (biome semantics, `astraweave-render` terrain paths) this doc cross-references rather than duplicates. |

---

## 1. Executive Summary

**What this system does:**
Procedurally generates terrain from a seed: layered fractal noise produces heightmaps, a climate field plus biome configuration classify each vertex, optional erosion reshapes the surface, and the result is packaged into `TerrainChunk`s; a parallel voxel subsystem (Sparse Voxel Octree + Dual Contouring) supports destructible/3D terrain; a scatter subsystem places vegetation and resource nodes; and streaming/LOD subsystems exist to load/unload chunks around a viewpoint.

**Why it exists:**
Provides the world substrate — heightfield + biome + vegetation — that the editor authors and that downstream render/collision systems consume.

**Where it primarily lives:**
- `astraweave-terrain/src/` — the entire generation, voxel, biome, noise, scatter, and streaming surface (42 source modules, ~35K LoC incl. tests).
- `tools/aw_editor/src/terrain_integration.rs` + `panels/terrain_panel.rs` + `panels/regional_archetype_panel.rs` — the production consumer of the **heightmap** generation path.
- `examples/hybrid_voxel_demo/src/main.rs` — the only production consumer of the **voxel meshing** path (`DualContouring`, `LodMeshGenerator`).
- `astraweave-render/src/{terrain.rs, clipmap_terrain.rs, terrain_gpu_bridge.rs}` — render-side terrain consumers (traced in [`render_pipeline_material_system_shader_infrastructure.md`](render_pipeline_material_system_shader_infrastructure.md); see §4).

**Status note:**
This is **not one system but several loosely-coupled subsystems** sharing a crate. The wired-into-the-editor path is the **heightmap** path (`WorldGenerator::generate_chunk_with_climate` → `TerrainChunk` → editor splat/mesh conversion). The **voxel** path (`VoxelChunk` / `DualContouring`) is wired only into one example. The **async streaming/LOD** subsystems (`BackgroundChunkLoader`, `LodManager`, `StreamingDiagnostics`, `MorphingLodManager`) and the **multi-archetype** data path (`RegionalArchetypeMask` Some-branch) are present and tested but have **no production caller** that exercises them end-to-end (§5, §6, §11). Treat per-subsystem wiredness as the load-bearing honesty signal.

---

## 2. Authoritative Pipeline

The wired editor heightmap path (the path an agent will most often touch):

```text
[WorldConfig { seed, chunk_size, heightmap_resolution, noise, climate, biomes, structures }]
    │
    │ WorldGenerator::new(config)
    ▼
[Stage 0: Generator construction]
    file: astraweave-terrain/src/lib.rs:180-198
    role: builds TerrainNoise(seed), ClimateMap(seed+1), ChunkManager, StructureGenerator(seed+2)
    key data: deterministic noise/climate functions seeded off config.seed
    │
    │ generate_chunk_with_climate(chunk_id, climate_bias)   (editor path)
    │   OR generate_chunk(chunk_id)                          (legacy/simple path)
    ▼
[Stage 1: Halo heightmap sampling]
    file: astraweave-terrain/src/lib.rs:683-730 (generate_halo_heightmap)
    role: samples TerrainNoise::sample_height over a 3×3-chunk halo at per-vertex world coords
    key data: Heightmap covering target chunk + 1-chunk halo (seam-safe erosion input)
    │
    │ apply_per_biome_modulation_to_halo(...)
    ▼
[Stage 2: Per-vertex biome modulation + archetype splines]
    file: astraweave-terrain/src/lib.rs:488-645
    role: per vertex: climate sample → Whittaker biome lookup → scattered-convolution
          parameter blend (biome_param_blending) → archetype BootstrapParams
          (spline_types / regional_archetype_mask) → re-sample height with blended
          mountains_amplitude (noise_gen::sample_height_with_params)
    key data: modulated halo heights + per-vertex Vec<BiomeId>
    │
    │ crop_halo_to_chunk (pre-erosion) → biome_weights via elevation_to_biome_weights
    ▼
[Stage 3: Erosion on the halo]
    file: astraweave-terrain/src/advanced_erosion.rs (apply_preset_at_world_offset)
          driven from lib.rs:395-428
    role: world-coordinate droplet hydraulic/thermal erosion on the full halo (seam-safe)
    key data: post-erosion halo heightmap
    │
    │ crop_halo_to_chunk (post-erosion) + assign_biomes(pre-erosion heights, climate)
    ▼
[Stage 4: Chunk assembly]
    file: astraweave-terrain/src/chunk.rs (TerrainChunk::new_with_climate_field)
    role: bundles post-erosion heightmap + biome_map + pre-erosion biome_weights[8] + biome_ids
    key data: TerrainChunk
    │
    ├──► [Scatter] scatter_chunk_content (lib.rs:218-264)
    │       file: astraweave-terrain/src/scatter.rs (VegetationScatter), structures.rs
    │       role: Poisson-disk vegetation + spawn_resources (gameplay) + structures
    │       key data: ScatterResult { vegetation, resources, structures }
    │
    └──► [Editor consumption] tools/aw_editor/src/terrain_integration.rs:343
            role: converts TerrainChunk → editor vertices → splat bake (see terrain_materials.md)
            key data: render-ready terrain (material path is terrain_materials.md's domain)
```

The parallel **voxel** path (wired only in `examples/hybrid_voxel_demo`):

```text
[VoxelChunk (Sparse Voxel Octree of Voxel{density, material})]
    file: astraweave-terrain/src/voxel_data.rs (CHUNK_SIZE=32, MAX_OCTREE_DEPTH=5)
    │
    │ DualContouring::generate_mesh(chunk)  (meshing.rs)
    │   uses marching_cubes_tables.rs (MC_EDGE_TABLE / MC_TRI_TABLE / EDGE_ENDPOINTS)
    ▼
[ChunkMesh { Vec<MeshVertex{position,normal,material:u16}>, Vec<u32> indices }]
    file: astraweave-terrain/src/meshing.rs
    role: isosurface extraction; add_skirts() hides LOD seam cracks
    │
    │ LodMeshGenerator::generate_mesh_lod(chunk, distance, ...)  (LOD decimation)
    ▼
[render-ready voxel mesh]   (consumed by examples/hybrid_voxel_demo only)
```

### Stage-by-stage detail

#### Stage 0: Generator construction
**File:** `astraweave-terrain/src/lib.rs:180-198`
**Role:** Constructs the deterministic generators. `TerrainNoise::new(&config.noise, config.seed)`, `ClimateMap::new(&config.climate, config.seed + 1)`, `StructureGenerator` at `config.seed + 2`. Within `TerrainNoise::new` (noise_gen.rs:410-461) the layers are seeded `seed`, `seed+1`, `seed+2`, ridged mountains `seed+42`, continental `seed + continental_seed_offset` (default 7).
**Notes:** `regional_archetype_mask` defaults to `None` (lib.rs:196), which preserves the single-archetype "Continental Temperate" byte-identity contract documented inline.

#### Stage 1: Halo heightmap sampling
**File:** `astraweave-terrain/src/lib.rs:683-730`
**Role:** Samples a heightmap covering the target chunk plus a 1-chunk halo on each side, so erosion droplets can travel across chunk boundaries within the halo without seams. Uses `TerrainNoise::sample_height(wx, wz)` directly (not the SIMD ChunkId-tied path) and relies on the noise field being a pure function of world coordinates for center-crop byte-identity (comment at lib.rs:705-710).
**Notes:** f32 step arithmetic is load-bearing — lib.rs:496-504 documents that an f64 `step` produced a 125-WU divergence at chunk borders.

#### Stage 2: Per-vertex biome modulation + archetype splines
**File:** `astraweave-terrain/src/lib.rs:488-645`
**Role:** For each halo vertex: samples the climate field, looks up a dominant `BiomeId` (Whittaker climate × elevation → biome via `biome_lookup`), runs `biome_param_blending::blend_biome_parameters` (N jittered samples, default 6 / 48 WU radius) to get a continuous blended `mountains_amplitude` + `scatter_density`, evaluates an archetype `BootstrapSplineSet`, then re-samples height with `noise_gen::sample_height_with_params`.
**Notes:** The `None`-mask branch evaluates Continental Temperate splines only (F.3 byte-identity contract). The `Some`-mask branch (regional_archetype_mask) blends per-vertex across up to multiple archetypes — present and unit-tested but **not reached in production** because no caller assigns `regional_archetype_mask` (§6, §11).

#### Stage 3: Erosion on the halo
**File:** `astraweave-terrain/src/advanced_erosion.rs`, driven from `lib.rs:395-428`
**Role:** Maps the climate bias to an `ErosionPreset` (`erosion_preset_for_climate`) and runs `AdvancedErosionSimulator::apply_preset_at_world_offset` over the full halo using world-cell-hashed droplet spawns for cross-chunk determinism. Only runs when `config.noise.erosion_enabled`.
**Notes:** The simulator's `new(seed)` is no longer the primary determinism driver in the world-coord path; per-droplet RNG derives from world-cell hash (lib.rs:401-407). The legacy simple cellular-automaton erosion (`TerrainChunk::apply_erosion`, chunk.rs:204) is bypassed by this path and used only by the simpler `generate_chunk` route (lib.rs:299-301).

#### Stage 4: Chunk assembly
**File:** `astraweave-terrain/src/chunk.rs` (`TerrainChunk::new` / `new_with_biome_weights` / `new_with_climate_field`)
**Role:** Bundles the post-erosion heightmap, the `Vec<BiomeType>` biome_map (from PRE-erosion heights + climate), optional pre-erosion `biome_weights: Vec<[f32;8]>`, and optional per-vertex `biome_ids: Vec<BiomeId>`.
**Notes:** §2.5 authorial-intent invariant: biome weights/IDs are computed from PRE-erosion heights so that erosion does not geologically reclassify a vertex's biome (chunk.rs:69-91).

#### Voxel meshing detail
**File:** `astraweave-terrain/src/meshing.rs`, `voxel_data.rs`, `marching_cubes_tables.rs`
**Role:** `DualContouring::generate_mesh` extracts an isosurface from a `VoxelChunk` (32³ SVO of `Voxel{density:f32, material:u16}`), using marching-cubes edge/tri tables for connectivity. `ChunkMesh::add_skirts` (meshing.rs:68) generates vertical skirt geometry on the 6 boundary faces to hide LOD-seam cracks. `LodMeshGenerator` / `AsyncMeshGenerator` provide LOD and async variants.
**Notes:** This voxel representation is **independent** of the heightmap `TerrainChunk` path — they share the crate, not the data flow. `ChunkCoord` (3D, voxel_data.rs:29) and `ChunkId` (2D, chunk.rs:11) are different coordinate types (§3, §6).

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **`ChunkId`** | 2D `{x:i32, z:i32}` identifier for a heightmap terrain chunk | `chunk.rs:11`, `lib.rs`, `background_loader.rs` |
| **`ChunkCoord`** | 3D `{x,y,z:i32}` identifier for a voxel chunk (32³) | `voxel_data.rs:29`, `meshing.rs` |
| **`TerrainChunk`** | Heightmap-based chunk: heightmap + biome_map + optional biome_weights/biome_ids | `chunk.rs:65` |
| **`VoxelChunk`** | Sparse-voxel-octree chunk of `Voxel{density, material}` | `voxel_data.rs` |
| **`Heightmap`** | 2D grid of f32 heights at a given resolution | `heightmap.rs`, `noise_gen.rs` |
| **`BiomeType`** | 8-variant ecological enum (Grassland/Desert/Forest/Mountain/Tundra/Swamp/Beach/River) | `biome.rs:12` |
| **`BiomeId`** | Whittaker climate-lookup biome identity used by the F.4.B.3.D climate-field path | `biome_lookup.rs`, `chunk.rs:91` |
| **`biome_weights`** | Per-vertex `[f32;8]` soft biome blend, pre-erosion | `chunk.rs:81`, `elevation_biome.rs` |
| **`WorldArchetypeId`** | One of 6 climate-envelope presets (ContinentalTemperate, EquatorialTropical, BorealSubarctic, Mediterranean, Desert, Custom) | `world_archetypes.rs`, `lib.rs:542-561` |
| **`RegionalArchetypeMask`** | Paintable 1024² uint8 archetype-id grid + falloff distance field | `regional_archetype_mask.rs:67` |
| **`BootstrapParams` / `BootstrapSplineSet`** | Per-archetype noise bootstrap parameters and the climate→params splines that produce them | `spline_types.rs` |
| **`NoiseLayer`** | One fractal noise layer (scale, amplitude, octaves, persistence, lacunarity, type, domain-warp) | `noise_gen.rs` |
| **Continental modulation** | Low-frequency spatial field that scales mountain amplitude for regional highland/lowland clustering | `noise_gen.rs:29-50` |
| **Halo** | A chunk plus its 1-chunk border, generated together so erosion is seam-safe | `lib.rs:683` |
| **Scatter** | Procedural placement of vegetation instances and resource nodes | `scatter.rs`, `zone_scatter.rs` |
| **Blueprint zone** | Polygon region driving `.blend`-sourced replica/inspired scatter + heightmap injection | `blueprint_zone.rs`, `zone_scatter.rs` |

### Terms to NOT confuse

- **`ChunkId` (2D heightmap) vs `ChunkCoord` (3D voxel):** different types, different subsystems. `ChunkId` indexes the editor heightmap world; `ChunkCoord` indexes the voxel demo world. They do not interoperate.
- **`BiomeType` (8-enum) vs `BiomeId` (Whittaker lookup):** `BiomeType` is the original ecological enum scored from height/temp/moisture (`find_best_biome`, lib.rs:859). `BiomeId` is the newer climate-field lookup identity carried per-vertex. A `TerrainChunk` may carry both (`biome_map: Vec<BiomeType>` AND `biome_ids: Option<Vec<BiomeId>>`).
- **"chunk" in this crate vs "chunk" in `terrain_materials.md`:** here a chunk is a generation/storage unit (`TerrainChunk`/`VoxelChunk`); in the material trace a "chunk" owns splat textures on the render side. Related spatially, distinct ownership.
- **Erosion: simple CA vs `AdvancedErosionSimulator`:** `TerrainChunk::apply_erosion` (chunk.rs:204) is a cheap cellular automaton used by `generate_chunk`. The climate path uses `AdvancedErosionSimulator` (droplet hydraulic/thermal). They are different algorithms; the climate path explicitly bypasses the CA (lib.rs:336-337).
- **"Splat":** see [`terrain_materials.md`](terrain_materials.md) §3 — `texture_splatting.rs` (8-layer, test-only) vs the render 32-layer pipeline. This trace does not own the splat path.

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| Caller config | `WorldGenerator::new(WorldConfig)` | seed, chunk_size, resolution, noise/climate/biome/structure config | `lib.rs:180`. Editor builds this in `terrain_integration.rs:302` |
| `astraweave-gameplay` (**reverse dep**) | `astraweave_gameplay::{spawn_resources, ResourceNode}` (scatter.rs:4); `astraweave_gameplay::{types::ResourceKind, BiomeRule}` (biome.rs:3-4) | resource-node spawning + biome rule types | **Directional anomaly** documented in [`ARCHITECTURE_MAP.md`](ARCHITECTURE_MAP.md):108,148 and Cargo.toml:14. World-gen imports gameplay types; conceptually should flow the other way. Verified present at commit `7c29b8182` |
| `astraweave-blend` (schema-mirror, no dep) | local `FixedPlacement` / `SourceHeightmap` structs mirroring `astraweave_blend::heightmap_raster` JSON | `.blend` placements + rasterized heightmaps | `zone_scatter.rs:31-70` deliberately re-declares the schema to AVOID a cross-crate dep |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| `tools/aw_editor` (heightmap path) | `WorldGenerator::generate_chunk_with_climate(chunk_id, climate_bias)` | `TerrainChunk` | `terrain_integration.rs:343`. THE wired production consumer of generation |
| `tools/aw_editor` (vegetation) | `VegetationScatter::new(...).scatter_vegetation_hierarchical(...)` | `Vec<VegetationInstance>` | `terrain_integration.rs:2728-2760` |
| `tools/aw_editor` (blueprint zones) | `ZoneScatterGenerator::new(64.0, 65).generate_zone_scatter(&bz, &chunk_refs, seed)` | `ZoneGenerationResult` | `main.rs:7458-7465`. Note: editor seeds this from `SystemTime` (main.rs:7459) — non-deterministic at that call site |
| `examples/hybrid_voxel_demo` (voxel path) | `DualContouring::generate_mesh(chunk)`, `LodMeshGenerator::generate_mesh_lod(...)` | `ChunkMesh` | `examples/hybrid_voxel_demo/src/main.rs:120,183`. Only production caller of the voxel meshing path |
| `astraweave-render` (terrain GPU bridge) | `astraweave_terrain::gpu_bridge::TerrainGpuAccelerator` impl `WgpuTerrainAccelerator` | `GpuHeightmapRequest`/`GpuNoiseRequest`/`GpuErosionRequest` | `astraweave-render/src/terrain_gpu_bridge.rs:11-67`. The impl exists but has no production caller outside its own file (§11). Traced in [`render_pipeline_material_system_shader_infrastructure.md`](render_pipeline_material_system_shader_infrastructure.md) |
| Material/splat path | biome data → editor splat bake | biome → material slot influence | Out of scope here; see [`terrain_materials.md`](terrain_materials.md) §4 (the exact biome→material-slot interface is marked NEEDS VERIFICATION there) |

### Bidirectional / Coupled

- **`BiomeType` shared with gameplay/render:** `BiomeType` (terrain) is consumed by render and scatter and flows through the gameplay reverse-dep. [`ARCHITECTURE_MAP.md`](ARCHITECTURE_MAP.md):439 rates this HIGH blast-radius: changing gameplay biome types breaks terrain generation.
- **`RegionalArchetypeMask` ↔ editor panel:** `regional_archetype_panel.rs` owns a `RegionalArchetypeMask` and documents `WorldGenerator.regional_archetype_mask` as its "integration surface" (regional_archetype_panel.rs:12), but no code assigns the mask onto a generator — the coupling is documented, not yet wired (§6, §11).

---

## 5. Active File Map

Wiredness verified by workspace grep for non-test/non-example production callers at commit `7c29b8182`.

| File | Role | Status | Notes |
|---|---|---|---|
| `lib.rs` | `WorldGenerator`, `WorldConfig`, halo/crop/biome-modulation orchestration | Active | Wired via editor `terrain_integration.rs`. Central entry point |
| `noise_gen.rs` | `TerrainNoise`, `NoiseConfig`, layered fBm + domain warp + continental + derivative-weighted + per-octave weights | Active | Deterministic; seeded off `config.seed`. `mountain_octave_weights` field is read-but-no-effect (infrastructure-only, noise_gen.rs:84-97) |
| `noise_simd.rs` | `SimdHeightmapGenerator` (SIMD heightmap, default `simd-noise` feature) | Active | Used by `generate_chunk` under `#[cfg(feature="simd-noise")]` (lib.rs:270) |
| `perlin_gradient.rs` | Analytical-derivative Perlin + derivative-weighted fBm | Active (noise layer) | Backs `base_derivative_weighted` path |
| `heightmap.rs` | `Heightmap`, `HeightmapConfig` | Active | Core storage type |
| `chunk.rs` | `ChunkId`, `TerrainChunk`, `ChunkManager`, `smooth_shared_vertices` | Active | Heightmap chunk + manager; legacy `apply_erosion` CA lives here (line 204) |
| `climate.rs` | `ClimateMap`, `ClimateConfig` (temperature/moisture field) | Active | Feeds biome classification |
| `biome.rs` | `BiomeType` (8), `BiomeConfig`, `BiomeVegetation`, `VegetationType` | Active | Imports gameplay `ResourceKind`/`BiomeRule` (reverse dep) |
| `elevation_biome.rs` | `elevation_to_biome_weights`, `ClimateBias`, `SEA_LEVEL` | Active | Produces 8-slot pre-erosion biome weights |
| `biome_lookup.rs` | Whittaker `BiomeId` lookup (climate × elevation) | Active | F.4.B.3.D.2 |
| `biome_parameters.rs` | Per-`BiomeId` parameter table (mountains_amplitude wired; ridge_strength defined-unwired) | Active (partial) | F.4.B.3.D.3 |
| `biome_param_blending.rs` | Scattered-convolution blend of biome parameters | Active | F.4.B.3.D.4; called in lib.rs:583 |
| `world_archetypes.rs` | 6-archetype climate-envelope catalog | Active | F.4.B.3.D.5 |
| `spline_types.rs` | `BootstrapParams`, `Spline1D`, per-archetype `BootstrapSplineSet` | Active | Evaluated in lib.rs:534-601 |
| `biome_blending.rs` | `BiomeBlender`, `PackedBiomeBlend` (MAX_BLEND_BIOMES=4) | Active (biome layer) | Also covered by `terrain_materials.md` §5 |
| `biome_pack.rs` | `.blend`-decomposition asset-pack format (`BiomePack`, manifest bridge) | Active (biome layer) | Consumed by editor panels; see `terrain_materials.md` §5 |
| `advanced_erosion.rs` | `AdvancedErosionSimulator`, presets, `erosion_preset_for_climate` | Active | Wired via `generate_chunk_with_climate` (lib.rs:395-428). NOTE: contradicts the CLAUDE.md "AdvancedErosionSimulator dormant/removed" claim — see §6 |
| `erosion.rs` | Small standalone erosion helpers | Active (small) | 155 LoC |
| `runevision_erosion.rs` | Gradient-aligned gully extrusion filter (Skovbo Johansen, MPL-2.0) | Active (opt-in) | Applied only when `base_derivative_weighted` + `runevision: Some` |
| `scatter.rs` | `VegetationScatter`, `ScatterConfig`, `VegetationInstance`, `density_at_distance`, `spawn_resources` bridge | Active | Wired via editor `terrain_integration.rs:2728` and `generate_chunk_with_scatter` |
| `zone_scatter.rs` | `ZoneScatterGenerator`, `.blend` replica/inspired scatter, heightmap injection | Active | Wired via editor `main.rs:7458` (blueprint zones) |
| `blueprint_zone.rs` | `BlueprintZone`, `ZoneRegistry`, polygon math | Active | Backing data for zone_scatter |
| `structures.rs` | `StructureGenerator`, `StructureType`, structure scatter | Active | Wired via `scatter_chunk_content` (lib.rs:256) |
| `regional_archetype_mask.rs` | `RegionalArchetypeMask` (paint + I/O), `RegionalArchetypeBlend` sampler, `blend_bootstrap_params` | Active type / **dormant data path** | Type wired into editor `RegionalArchetypePanel`; the generation `Some`-branch (lib.rs:600-626) is unreachable because no caller assigns the mask (§6, §11) |
| `meshing.rs` | `DualContouring`, `LodMeshGenerator`, `AsyncMeshGenerator`, `ChunkMesh`, `MeshVertex` | Active in example only | Production caller: `examples/hybrid_voxel_demo` only |
| `voxel_data.rs` | `VoxelChunk` (SVO), `Voxel`, `ChunkCoord`, `VoxelGrid`, `CHUNK_SIZE=32` | Active in example only | Same single example consumer |
| `marching_cubes_tables.rs` | MC edge/tri tables for dual contouring | Active in example only | Data tables for `meshing.rs` |
| `compressed_voxels.rs` | Palette + RLE compression for voxel chunks | In-design-but-tested | P4-8; no production caller found |
| `collision.rs` | `collision_mesh_from_chunk` / `_from_heightmap`, `CollisionMesh` | In-design-but-tested | No non-test production caller found |
| `lod_manager.rs` | `LodManager` (hysteresis), `LodLevel`, `ViewParams`, `compute_pixel_error` | In-design-but-tested | No production caller; `clipmap_terrain.rs` mentions it only in a doc-comment |
| `lod_blending.rs` | `MorphingLodManager`, `LodBlender`, `MorphedMesh` | In-design-but-tested | Same — doc-comment reference only |
| `background_loader.rs` | `BackgroundChunkLoader`, `StreamingConfig`, `StreamingStats` (tokio async streaming) | In-design-but-tested | No production caller; editor uses synchronous `generate_chunk_with_climate` instead |
| `streaming_diagnostics.rs` | `StreamingDiagnostics`, `HitchDetector`, `DiagnosticReport` | In-design-but-tested | No production caller found |
| `partition_integration.rs` | `VoxelPartitionManager`, World-Partition streaming events | In-design-but-tested | No production caller in examples/tools/scene/render |
| `gpu_bridge.rs` | `TerrainGpuAccelerator` trait + GPU request/result types | Trait active, caller dormant | Impl `WgpuTerrainAccelerator` exists in `astraweave-render/src/terrain_gpu_bridge.rs` but has no production caller (§11) |
| `solver.rs` | `TerrainSolver` (Phase 10 AI-orchestrated terrain), `ResolvedLocation` | In-design-but-tested | No production caller found |
| `terrain_modifier.rs` | `TerrainModifier`, batched `VoxelOp` updates | In-design-but-tested | Phase 10; no production caller found |
| `terrain_persistence.rs` | Terrain save/load (Phase 10) | In-design-but-tested | No production caller found |
| `texture_splatting.rs` | 8-layer procedural splat (`MAX_SPLAT_LAYERS`) | Deprecated (test-only) | See `terrain_materials.md` §5/§11 — zero production callers |
| `mutation_tests.rs` / `chunk_tests.rs` / `voxel_data_tests.rs` | `#[cfg(test)]` mutation/comprehensive suites | Test-only | 3,670 + 532 + 532 LoC |

**Status definitions:**
- **Active**: load-bearing in the wired editor path; edit with care.
- **Active in example only**: exercised by exactly one example (`hybrid_voxel_demo`), not by the engine runtime or editor.
- **In-design-but-tested**: compiles, has passing tests, but **zero production callers** (per CLAUDE.md Key Lesson 8 taxonomy). Dormant until wired.
- **Deprecated (test-only)** / **Trait active, caller dormant**: see notes.

---

## 6. Conflict Map / Residue

### Coexisting abstractions

| Abstraction | Files | Status | Notes |
|---|---|---|---|
| Heightmap terrain (`TerrainChunk`/`Heightmap`/`ChunkId`) | `chunk.rs`, `heightmap.rs`, `lib.rs` | Active (wired) | The editor's terrain world |
| Voxel terrain (`VoxelChunk`/`ChunkCoord`/`DualContouring`) | `voxel_data.rs`, `meshing.rs` | Active in example only | Independent 3D representation; shares crate, not data flow |
| `BiomeType` (8-enum, scored) | `biome.rs`, `lib.rs:859` | Active | Original classification |
| `BiomeId` (Whittaker climate lookup) | `biome_lookup.rs`, `chunk.rs:91` | Active | Newer per-vertex identity; coexists with `BiomeType` on the same chunk |
| Simple CA erosion (`apply_erosion`) | `chunk.rs:204` | Active (legacy `generate_chunk` path) | Bypassed by the climate path |
| Droplet erosion (`AdvancedErosionSimulator`) | `advanced_erosion.rs` | Active (climate path) | Both erosion algorithms coexist |
| Synchronous chunk gen (editor) | `lib.rs` + `terrain_integration.rs` | Active (wired) | What the editor actually uses |
| Async streaming (`BackgroundChunkLoader` + `LodManager` + `StreamingDiagnostics`) | `background_loader.rs`, `lod_manager.rs`, `streaming_diagnostics.rs` | In-design-but-tested | A complete alternative loading subsystem with no production caller |
| Single-archetype gen (mask = None) | `lib.rs:600-601` | Active (wired) | F.3 byte-identity path |
| Multi-archetype gen (mask = Some) | `lib.rs:602-626`, `regional_archetype_mask.rs` | Dormant data path | Reachable only if a caller assigns `WorldGenerator.regional_archetype_mask`; none does |

### Naming collisions

- **`ChunkId` vs `ChunkCoord`:** 2D heightmap chunk id vs 3D voxel chunk coord. Both named "chunk". Always check the type.
- **`LodConfig`:** `meshing::LodConfig` (voxel mesh LOD) vs `lod_manager::LodConfig` (re-exported as `LodHysteresisConfig` in lib.rs:71 to disambiguate). The re-export rename is the existing mitigation.
- **`MeshVertex` (meshing.rs:18) vs render/editor terrain vertex formats:** the voxel `MeshVertex{position,normal,material:u16}` is distinct from the editor `TerrainVertex` (see `terrain_materials.md`). Do not cross-wire.
- **"Erosion":** four modules touch erosion (`erosion.rs`, `advanced_erosion.rs`, `runevision_erosion.rs`, `chunk.rs::apply_erosion`). Specify which.

### Known cognitive traps

- **Trap:** The CLAUDE.md "Integration Completeness" list states `AdvancedErosionSimulator` "shipped — and [was] later removed or rewritten" and that `RegionalArchetypePanel` was shipped then removed.
  **What's actually true at `7c29b8182`:** Both are PRESENT. `AdvancedErosionSimulator` is wired into `generate_chunk_with_climate` (lib.rs:395-428; `apply_preset_at_world_offset` call at lib.rs:419) and exported from lib.rs:46-49. `RegionalArchetypePanel` is instantiated (`tab_viewer/mod.rs:994` `RegionalArchetypePanel::new()`; dispatcher tool at `main.rs:2664`) and shown via the `PanelType::RegionalArchetype` handler (`tab_viewer/mod.rs:7796`, `dock_panels.rs:234`); the variant is declared at panel_type.rs:232. The aw_editor trace ([`aw_editor.md`](aw_editor.md):203) records that its *multi-tool migration* (not the panel itself) was deferred to Sub-phase 5. The CLAUDE.md statement is stale/imprecise for these two; treat the code as ground truth. [NEEDS VERIFICATION whether a prior commit truly removed them and they were re-added — only current state was traced. Git evidence found: `astraweave-terrain/src/advanced_erosion.rs` shows zero `--diff-filter=D` (delete) commits, and `regional_archetype_panel.rs` was added once (`26a3864b8`, F.5-paint.A) with no prior delete — i.e. no delete+re-add of these two files is visible. The "rewritten" half of the CLAUDE.md claim is broader than a file-delete check and remains unverified; a full per-symbol history diff would resolve it.]
- **Trap:** Assuming the whole crate is wired because the editor uses "terrain."
  **What's actually true:** Only the heightmap generation + scatter + biome path is wired into the editor. The voxel meshing path is example-only; streaming/LOD, partition, persistence, terrain-modifier, solver, collision, and the multi-archetype data path are in-design-but-tested with no production caller.
- **Trap:** Editing the `Some`-mask branch in `apply_per_biome_modulation_to_halo` expecting to see a visual change.
  **What's actually true:** No production code assigns `regional_archetype_mask`, so the branch is never taken at runtime — only by `tests/phase_1_x_f4_regional_mask_integration.rs`.
- **Trap:** Expecting `ZoneScatterGenerator` scatter to be reproducible across editor runs.
  **What's actually true:** The editor seeds it from `SystemTime::now()` (main.rs:7459), so that call site is non-deterministic even though the generator itself is seed-deterministic.

---

## 7. Decision Log

### Decision: Reverse dependency `terrain → gameplay`
- **Date:** [Reasoning not recovered from available sources]
- **Status:** Accepted (in active code; flagged as anomaly)
- **Context:** Terrain biome/scatter needs gameplay's `ResourceKind`, `BiomeRule`, `ResourceNode`, and `spawn_resources`.
- **Decision:** `astraweave-terrain` depends on `astraweave-gameplay` (Cargo.toml:14) and imports those types directly (biome.rs:3-4, scatter.rs:4).
- **Alternatives considered:** [Reasoning not recovered]. The `zone_scatter.rs` schema-mirror approach (re-declaring `astraweave_blend` structs locally to avoid a dep, zone_scatter.rs:28-35) shows the codebase sometimes avoids cross-crate deps; that pattern was NOT applied to gameplay types.
- **Consequences:** Directional anomaly recorded in [`ARCHITECTURE_MAP.md`](ARCHITECTURE_MAP.md):148. HIGH blast radius via `BiomeType`. Not a cycle (ARCHITECTURE_MAP.md:157).

### Decision: Halo-based seam-safe erosion + pre-erosion biome stability
- **Date:** [Reasoning visible in code comments; exact date not recovered]
- **Status:** Accepted (lib.rs:317-456 documents it extensively)
- **Context:** Per-chunk erosion would create seams at chunk boundaries; and erosion lowering a peak would otherwise reclassify a "mountain" vertex as lowland.
- **Decision:** Generate a 1-chunk halo, erode the full halo with world-cell-hashed droplet RNG, then crop. Compute biome weights/IDs from PRE-erosion heights (§2.5 authorial-intent invariant).
- **Alternatives considered:** [Reasoning not recovered]. Phase-0 measured droplet p95 travel = 120 WU < one chunk width, justifying halo=1 (lib.rs:329-331).
- **Consequences:** Adjacent chunks' overlapping halos iterate identical world cells → near-identical erosion in overlap (seam-safe). Requires f32 step arithmetic to match the SIMD path exactly (lib.rs:496-504).

### Decision: f32 (not f64) step arithmetic in halo generation
- **Date:** [Reasoning visible in code comments]
- **Status:** Accepted (lib.rs:496-504)
- **Context:** A regression (F.4.B.3.D.3b) found f64 `step` produced 125 WU divergence at chunk borders, breaking the shared-edge invariant.
- **Decision:** Mirror `generate_halo_heightmap`'s f32 arithmetic exactly in `apply_per_biome_modulation_to_halo`.
- **Consequences:** Operation-order discipline is load-bearing for byte-identity; documented again in noise_gen.rs:544-560 for `sample_height_with_params`.

### Decision: `regional_archetype_mask` defaults to `None`
- **Date:** Phase 1.X-F.4.E (per inline comment)
- **Status:** Accepted (lib.rs:170-197)
- **Context:** F.3 established a single-archetype byte-identity contract; F.4 adds a multi-archetype mask path.
- **Decision:** Default `None` preserves the F.3 byte-identical output; `Some(mask)` opts into the multi-archetype blend.
- **Consequences:** The multi-archetype path is a strictly additive, opt-in branch. Because no production caller sets it, the branch is currently dormant (§6, §11).

### Decision: Continental modulation defaults retuned for Target B scale
- **Date:** Phases F.2-T.B.1 / F.4.B.2.F (per inline comments)
- **Status:** Accepted (noise_gen.rs:120-162)
- **Context:** At the original `continental_scale=0.0004`, seed-12345 produced no highland regions across the editor extent (diagnostic at noise_gen.rs:124-132); `continental_min` was too low, producing a "bed-of-nails" surface.
- **Decision:** `continental_scale` retuned 0.0004 → 0.0012 → 0.0003 (Target B 11264 WU extent); `continental_min` raised 0.15 → 0.50.
- **Consequences:** Regional highland/lowland clustering exists at practical seeds; documented numerically inline.

### Decision: Dual Contouring over Marching Cubes for voxel meshing
- **Date:** [Reasoning visible in module doc-comment]
- **Status:** Accepted (meshing.rs:1-9)
- **Decision:** Use Dual Contouring (preserves sharp features, fewer artifacts, more uniform triangles, handles hermite data) for voxel isosurface extraction.
- **Consequences:** Marching-cubes tables retained for connectivity (`marching_cubes_tables.rs`). Skirt geometry (`add_skirts`) added for LOD-seam crack hiding.

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | `TerrainNoise::sample_height(x,z)` is a pure deterministic function of `(seed, world_x, world_z)` — same inputs → byte-identical output across runs/platforms | Yes | Doc-only + relied on for halo center-crop byte-identity (lib.rs:705-710); `halo_seed` determinism has explicit tests (lib.rs:895-917) |
| 2 | The halo center-crop is byte-identical to single-chunk `generate_chunk` output at the same world coords | Yes | Tests `phase_1_6_f3_phase_*`; f32-step discipline (lib.rs:496-504) is the mechanism |
| 3 | Biome weights/IDs are computed from PRE-erosion heights (authorial intent over post-erosion shape) | Yes | Structural (lib.rs:364-392); test `phase_1_6_f3_phase_1_biome_weights_pre_erosion.rs` |
| 4 | `mask = None` produces byte-identical output to the F.3 single-archetype path | Yes | Structural branch (lib.rs:600-601); regional-mask integration test |
| 5 | Adjacent chunks' overlapping halos produce near-identical erosion in the overlap region (seam-safety) | Partially | `apply_preset_at_world_offset` world-cell determinism; `phase_1_6_f3_phase_2_continuity.rs`, `phase_1_6_f3_phase_3_diagnostic.rs` |
| 6 | `ChunkId` (2D) and `ChunkCoord` (3D) are never interchanged | No (type system enforces non-coercion, but no cross-check) | Type system |
| 7 | A `VoxelChunk` mesh's skirt geometry is generated only for vertices on a boundary face within `eps` | Yes | `add_skirts` boundary test (meshing.rs:74-94) + meshing tests |

---

## 9. Performance & Resource Profile

### Hot paths
- **`apply_per_biome_modulation_to_halo`** (lib.rs:488): per halo vertex does 1 raw noise sample + N (default 6) climate samples + N biome lookups + N parameter lookups + 1 modulated noise sample. For a 96-resolution chunk with halo=1 that is ~288² ≈ 83K vertices. Cost analysis is documented inline (lib.rs:481-487). Climate sampling and biome lookup are pure functions — cache-friendly.
- **`AdvancedErosionSimulator` droplet pass:** droplet_count × max_droplet_lifetime steps over the full halo. Cost scales with preset droplet count and halo area.
- **`DualContouring::generate_mesh`:** per voxel cell over a 32³ chunk; HashMap-based vertex dedup (meshing.rs:14).

### Cold paths
- **Chunk generation** runs at chunk-load time, not per frame. The editor invokes it on demand.
- **Scatter / structures** run per-chunk at generation time.

### Resource ownership
- **`WorldGenerator`** owns `TerrainNoise`, `ClimateMap`, `ChunkManager`, `StructureGenerator`, and the optional `regional_archetype_mask`. Lifetime = generator lifetime.
- **`ChunkManager`** owns loaded `TerrainChunk`s and handles radius load/unload (`stream_chunks`, lib.rs:822).
- **`VoxelChunk`** owns its SVO; `ChunkMesh` owns vertex/index buffers (`memory_usage` at meshing.rs:54).
- **SIMD/rayon:** `noise_simd` (SIMD heightmap) is feature-gated default-on; `rayon` is a declared dependency (Cargo.toml:16). Verified: the only rayon call site in the crate is `AsyncMeshGenerator::generate_meshes_parallel` (`meshing.rs:472-484`, `into_par_iter` at meshing.rs:478) — i.e. parallel VOXEL meshing, part of the example-only voxel path. There is NO rayon usage in the heightmap generation/noise path (lib.rs/noise_gen.rs run single-threaded); the "parallel generation" reading is not supported by the code.

---

## 10. Testing & Validation

- **Inline unit tests (`#[cfg(test)]`):** high density across modules — `regional_archetype_mask.rs` (33), `chunk.rs` (35), `lod_manager.rs` (28), `noise_gen.rs` (22), `background_loader.rs` (20), `zone_scatter.rs` (16), `scatter.rs` (12), `biome_pack.rs` (9), `biome.rs` (6), `voxel_data.rs` (6), `advanced_erosion.rs` (6), `meshing.rs` (5), `structures.rs` (4).
- **Comprehensive/mutation test modules:** `mutation_tests.rs` (3,670 LoC), `chunk_tests.rs` (532), `voxel_data_tests.rs` (532), all `#[cfg(test)]`.
- **Integration tests (`astraweave-terrain/tests/`):** 54 files, heavily weighted toward the `phase_1_6_f3*` / `phase_1_6_f4_b_3_d*` / `phase_1_x_f4*` campaign families (halo scaffolding, biome-weight pre-erosion, continuity, cross-archetype, regional-mask integration) plus Wave 2/3 mutation-remediation suites (scatter, meshing, voxel, erosion, structures, partition).
- **Benchmarks:** `benches/terrain_generation.rs`.
- **Mutation testing:** Wave 2/3 remediation present (multiple `wave2_*`/`wave3_*` test files). `texture_splatting.rs` mutation coverage is documented in `terrain_materials.md` §10.
- **Manual validation:** the F-series campaign used editor-visual diagnostics (continental field highland presence, highland-Y-max regression, bed-of-nails surface) recorded inline in `noise_gen.rs` and in `docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md` (referenced in noise_gen.rs:552).
- **Determinism note:** seed-determinism is strong inside the crate, but the editor's `ZoneScatterGenerator` call seeds from `SystemTime` (main.rs:7459), so that one production path is non-deterministic by construction.

---

## 11. Open Questions / Parked Decisions

- **Is the async streaming/LOD subsystem (`BackgroundChunkLoader`, `LodManager`, `MorphingLodManager`, `StreamingDiagnostics`, `partition_integration`) intended to become the runtime loader, or is it dormant scaffolding?** It is fully implemented and tested but has zero production callers at `7c29b8182`; the editor uses synchronous `generate_chunk_with_climate`. Resolving this determines whether these ~5K LoC are "in-design-but-tested" (per CLAUDE.md Lesson 8) or destined for wiring.
- **Should the `RegionalArchetypeMask` `Some`-branch be wired into the editor?** The panel exists, owns a mask, and documents `WorldGenerator.regional_archetype_mask` as the integration surface (regional_archetype_panel.rs:12), but no code assigns the mask. The entire multi-archetype generation branch (lib.rs:602-626) is therefore unreachable in production. Is this a deferred wiring (aw_editor Sub-phase 5) or a parked feature?
- **What is the runtime role of the voxel terrain path (`VoxelChunk`/`DualContouring`)?** It is wired only into `examples/hybrid_voxel_demo`. Is it the intended destructible-terrain runtime, or a research prototype distinct from the heightmap editor world? `compressed_voxels.rs`, `terrain_modifier.rs`, `terrain_persistence.rs`, and `solver.rs` (all Phase 10, all without production callers) appear to be the supporting cast for a voxel runtime that is not yet assembled.
- **Does `WgpuTerrainAccelerator` (the `TerrainGpuAccelerator` impl in `astraweave-render`) have any intended caller?** The impl compiles but nothing invokes it outside its own file. GPU-accelerated heightmap/erosion is therefore dormant.
- **Is the CLAUDE.md claim that `AdvancedErosionSimulator` and `RegionalArchetypePanel` were "removed" stale, or did a prior commit remove and re-add them?** Both are present and (for erosion) wired at `7c29b8182`. This trace verified only current state. [NEEDS VERIFICATION of git history.]
- **Should the gameplay reverse dependency be inverted?** [`ARCHITECTURE_MAP.md`](ARCHITECTURE_MAP.md):148 flags it as a directional anomaly with HIGH blast radius. It is working code; the question is parked, not a recommendation.

---

## 12. Maintenance Notes

**Update this doc when:**
- `lib.rs` generation orchestration changes (halo size, modulation order, erosion wiring).
- A subsystem currently "in-design-but-tested" gains a production caller (move it to Active and update §5/§6).
- The `regional_archetype_mask` data path is wired into the editor (closes a §11 question; update §6 dormant-branch note).
- The voxel path gains a runtime consumer beyond `hybrid_voxel_demo`.
- The `terrain → gameplay` dependency direction changes.
- Determinism guarantees (Invariants 1-5) are relaxed or newly enforced.

**Verification process:**
- Re-run the wiredness grep: `rg '<Type>::new|<fn>\(' --type rust -g '!*test*' -g '!*example*'` for each subsystem entry point in §5.
- Spot-check the §2 pipeline against `lib.rs` (halo/crop/modulation/erosion order) and `meshing.rs` (voxel path).
- Confirm the gameplay reverse dep at `astraweave-terrain/Cargo.toml:14`.
- Stamp the new commit hash and date here after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. **This crate is several subsystems, not one.** Only the heightmap generation + biome + scatter path is wired into the editor. Voxel meshing is example-only; streaming/LOD, partition, persistence, terrain-modifier, solver, collision, GPU bridge, and the multi-archetype mask branch are in-design-but-tested with no production caller. Check wiredness before assuming a change is observable.
2. **Determinism is a contract.** `TerrainNoise::sample_height` must stay a pure function of `(seed, x, z)`. The halo path's f32 step arithmetic is load-bearing for seam byte-identity (lib.rs:496-504). Do not "clean up" to f64.
3. **Biome lives in two type systems** (`BiomeType` enum + `BiomeId` Whittaker lookup) and both can ride on the same `TerrainChunk`. The material/splat side is a *third* concern owned by [`terrain_materials.md`](terrain_materials.md).
4. **The `terrain → gameplay` dependency is intentional-but-anomalous.** Adding more gameplay imports widens an already-flagged reverse dep.
5. **CLAUDE.md's "AdvancedErosionSimulator/RegionalArchetypePanel removed" note is stale** for current code — both exist; erosion is wired.

**Files you'll most likely touch:**
- `astraweave-terrain/src/lib.rs` (generation orchestration)
- `astraweave-terrain/src/noise_gen.rs` (noise tuning)
- `astraweave-terrain/src/biome*.rs` (biome classification/parameters)
- `astraweave-terrain/src/scatter.rs` / `zone_scatter.rs` (vegetation)
- `tools/aw_editor/src/terrain_integration.rs` (the production consumer)

**Files you should NOT touch without strong reason:**
- `texture_splatting.rs` — deprecated/test-only; see `terrain_materials.md`.
- The in-design-but-tested streaming/LOD/partition/persistence/solver modules — extending them adds dormant surface; confirm a wiring plan first (§11).
- The `Some`-mask branch in `apply_per_biome_modulation_to_halo` — unreachable in production until the mask is wired.

**Common mistakes when changing this system:**
- **Mistake:** Conflating `ChunkId` (2D heightmap) with `ChunkCoord` (3D voxel). They belong to different subsystems.
- **Mistake:** Computing biome weights from post-erosion heights. The invariant (§8.3) requires pre-erosion heights.
- **Mistake:** Assuming a voxel-path or streaming-path edit affects the editor. It does not — the editor uses the synchronous heightmap path.
- **Mistake:** Changing halo step arithmetic precision; it breaks seam byte-identity.

---

## Appendix B: Historical context

The terrain crate accreted in distinct campaign waves visible in module names and inline comments: an original heightmap+biome generator (`WorldGenerator`, `BiomeType`, simple CA erosion), a "hybrid voxel" subsystem (`voxel_data`, `meshing`, marching-cubes tables, Phase 10 modifier/persistence/solver), a Week-3/4 performance + streaming wave (`noise_simd`, `background_loader`, `lod_manager`, `streaming_diagnostics`), and a long Phase-1.6 "F-series" / Phase-1.X regional-archetype campaign that layered a climate-field architecture (`biome_lookup`, `biome_parameters`, `biome_param_blending`, `world_archetypes`, `spline_types`, `regional_archetype_mask`, halo-based seam-safe erosion) on top of the original generator. The F-series is the most heavily commented and tested area and is the active locus of the wired editor path. The voxel and streaming waves left substantial, well-tested surface that has not (yet) been wired into the engine runtime — the single largest honesty signal for agents working here.
