# Terrain, Worldgen, and Scatter Archaeology

**Workstream:** WS-RENDER-ENV-B. §1–§11 are the survey. §12 is what P0–P4 implemented. §13 is the unification plan (not started).
**Evidence commit (engine `main` at rebase):** `85d09a2c`.
**Demo evidence:** PR [#205](https://github.com/lazyxeon/astraweave/pull/205). Terrain edits here are written to merge into that branch. The demo agent still owns the rest of #205.
**Date:** 2026-10-14.
**Authority already on disk:** `docs/architecture/terrain.md` (v1.7, verified 2026-07-26), `docs/architecture/terrain_materials.md`, `docs/architecture/pcg.md` (v1.1), `docs/architecture/scene.md` (v1.1). This note is the cross-check of those traces against `main` and against the Aravath greybox. Where a trace and the code disagree, the code citation below wins.

Phase 1 was the survey. After review, P0–P4 were approved with the amendments in §12. P5 as written (leave the dormant stacks untouched) was rejected. §13 is the unification plan. The f32 `WorldGenerator` path is still the editor heightmap. The sim ground is `column_gen`.

---

## 1. Verdict

The engine already contains a large, tested terrain crate and several neighboring generators. They are not one system, and the Aravath demo does not run the editor heightmap pipeline.

What is live today:

- **Editor heightmap world.** `WorldGenerator::generate_chunk_with_climate` in `astraweave-terrain` is the production generator. The editor calls it from `tools/aw_editor/src/terrain_integration.rs` and then places vegetation with `VegetationScatter::scatter_vegetation_hierarchical`. Biome presets in `biome.rs` already point at KayKit Forest Nature Pack glTFs and at a separate Verdant Trail pack.
- **Aravath greybox ground.** PR #205 builds its own integer heightfield into `astraweave_terrain::VoxelGrid`, meshes it with `DualContouring`, and scatters KayKit nature glbs (`tree_oak`, `tree_pineDefaultA`, `tree_palmShort`, `plant_bush`, `rock_largeA`, `grass`) on a 5-voxel lattice. Dig (`E` / Interact, radius 2.5) and place (`R`, radius 1.1, materials 1–5) write density `0.0` / `1.0` voxels. Streaming is a Chebyshev ring of y=0 chunks, `VIEW_RADIUS = 3` (~224 m). This path never calls `WorldGenerator`, `VegetationScatter`, climate, erosion, or biomes from the terrain crate. It uses a local `Biome` of Grass / Sand / Rock driven by `detmath` sin/cos.
- **Editable voxels exist, on the voxel type, not the heightmap type.** `Voxel { density: f32, material: u16 }` in 32³ chunks is the only representation that can dig and place. `TerrainChunk` is a heightmap plus biome weights. Those two types do not share a data flow (`ChunkId` is 2D; `ChunkCoord` is 3D).

What is real code with tests and no production caller: async chunk streaming, heightmap LOD, clipmap terrain (the file is not even a module), GPU noise/erosion, the regional-archetype mask branch, Phase 10 voxel persistence/solver/modifier, `astraweave-pcg` (WFC / rooms / encounters), and scene world-partition streaming beyond two examples.

The demo’s mesher is not the mesher on `main`. PR #205 rewrites `astraweave-terrain/src/meshing.rs` (Bourke corner order, one vertex per crossed edge). On `main`, triangle generation looks up `vertex_cache` with `edge_key.min` while the cache is keyed by cell position (`meshing.rs` around the `generate_cell_triangles` edge loop). A later implementation phase has to take the demo’s mesher, with tests, rather than re-meshing the greybox with the `main` function.

---

## 2. Status table

Status words: **Wired** = a non-test, non-example caller on `main` exercises it. **Demo** = exercised by PR #205 and not by `main`. **Example** = an `examples/` binary only. **Tested-dormant** = compiles, has tests, zero production callers. **Orphan** = on disk, not in the crate module tree, or calls symbols that do not exist.

| Surface | Path | LoC (src) | Status | Caller evidence | Determinism |
|---|---|---:|---|---|---|
| Climate heightmap generator | `astraweave-terrain/src/lib.rs` `generate_chunk_with_climate` | 1,030 (`lib.rs`) | Wired (editor) | `terrain_integration.rs` builds `WorldGenerator` and calls this | Seeded. f32 step arithmetic is load-bearing. `noise` crate f32 Perlin/Fbm. Halo erosion uses an integer world-cell hash (`halo_seed`) |
| Legacy heightmap chunk | `lib.rs` `generate_chunk` | (same) | Example | `astraweave-render/src/terrain.rs` `TerrainRenderer`; `examples/terrain_demo`, `examples/biome_showcase` | `StdRng` + `noise` crate. Simple CA erosion, not the droplet simulator |
| Noise | `noise_gen.rs`, `perlin_gradient.rs`, `noise_simd.rs` | 1,901 + 415 + 204 | Wired via the generator | Editor path samples `TerrainNoise::sample_height` / `sample_height_with_params` | `noise` 0.9 `Fbm<Perlin>` / `Billow` / `RidgedMulti`, seed cast to `u32`. SIMD path is feature-default and used by `generate_chunk` only |
| Heightmap + 2D chunks | `heightmap.rs`, `chunk.rs` | 428 + 858 | Wired | Editor and `TerrainRenderer` | f32 heights |
| Climate + Whittaker biomes | `climate.rs`, `biome_lookup.rs`, `biome_parameters.rs`, `biome_param_blending.rs`, `world_archetypes.rs`, `spline_types.rs`, `elevation_biome.rs` | 1,134 + 836 + 641 + 603 + 605 + 1,423 + 565 | Wired (editor climate path) | Six archetypes, selected archetype’s splines (E3, `d506658d`, 2026-07-03) | Pure functions of seed and world xz, in f32 |
| 8-biome enum + KayKit presets | `biome.rs` | 2,082 | Wired as data | Editor scatter reads `BiomeConfig`. Model paths include KayKit Forest Nature Pack and `assets/imported/verdant_trail/...` and `assets/models/grass.glb` | n/a (data) |
| Droplet erosion | `advanced_erosion.rs` | 1,232 | Wired (climate path, when `erosion_enabled`) | `lib.rs` `apply_preset_at_world_offset` | World-cell hash, not `HashMap` order |
| CA erosion / small helpers / runevision gullies | `chunk.rs::apply_erosion`, `erosion.rs`, `runevision_erosion.rs` | 155 + 340 | Legacy path / opt-in | CA runs on `generate_chunk`. Runevision runs only when derivative-weighted base + `runevision: Some` | Separate algorithms from droplet erosion |
| Editor vegetation scatter | `scatter.rs` `VegetationScatter` | 1,104 | Wired (editor) | `terrain_integration.rs` ~2856 calls `scatter_vegetation_hierarchical` | `StdRng::seed_from_u64`. Poisson disk. Tint hash mixes `position as i32` after an f32 multiply |
| Blueprint / .blend zone scatter | `zone_scatter.rs`, `blueprint_zone.rs`, `biome_pack.rs` | 1,303 + 918 + 1,358 | Wired (editor action) | `tools/aw_editor/src/main.rs` ~7846 | Generator is seed-deterministic. **The editor passes `SystemTime::now()` as the seed** |
| Structures | `structures.rs` | 665 | Wired inside `scatter_chunk_content` | `lib.rs` ~218. `TerrainRenderer::generate_chunk_complete` is the example caller of that bundle | `HashMap<StructureType, u32>` for counts (summary only) |
| Demo scatter | `aravath_runtime/src/ground.rs` `scatter_chunk` (PR #205 only) | ~90 | Demo | Called from `Ground::stream_around` | Integer `mix(x, z)`. Grid stride 5. Yaw is `(n & 255) as f32 / 255.0 * TAU` |
| Demo height + materials | `aravath_runtime/src/island.rs`, `ground.rs` `fill_height_chunk` | 128 + part of 641 | Demo | `Greybox::open` | `detmath::sin` / `cos` then `floor` to `i32`. Materials are `u16` |
| Voxel storage | `voxel_data.rs` `CHUNK_SIZE = 32` | 510 | Demo + example | PR #205 `VoxelGrid`; `examples/hybrid_voxel_demo` | Density is `f32`. Demo writes only `0.0` and `1.0` |
| Dual contouring | `meshing.rs` | 641 on `main`; PR #205 changes +67/−116 | Demo uses the **branch** mesher; `main` mesher is example-only and the edge lookup key does not match the cache key | `hybrid_voxel_demo` on `main`. Aravath on the branch | Vertex push order follows the cell walk, not `HashMap` iteration. Positions are f32 lerps |
| Compressed voxels, modifier, persistence, solver | `compressed_voxels.rs`, `terrain_modifier.rs`, `terrain_persistence.rs`, `solver.rs` | 647 + 762 + 608 + 622 | Tested-dormant | Phase 10. No production caller | — |
| Collision meshes | `collision.rs` | 219 | Tested-dormant | No non-test caller. The demo builds Rapier trimeshes from `ChunkMesh` in `ground.rs`, not from this module | — |
| Async streaming + LOD | `background_loader.rs`, `lod_manager.rs`, `lod_blending.rs`, `streaming_diagnostics.rs`, `partition_integration.rs` | 1,456 + 987 + 566 + 795 + 528 | Tested-dormant | Editor generates synchronously. `clipmap_terrain.rs` mentions `LodManager` only in a deprecation comment | Loaders store chunks in `HashMap` |
| Regional archetype mask | `regional_archetype_mask.rs` | 1,482 | Type wired, generation branch dormant | Editor panel paints a mask. No caller assigns it onto `WorldGenerator` | — |
| GPU height/erosion bridge | `gpu_bridge.rs` + `astraweave-render/src/terrain_gpu_bridge.rs`, `gpu_erosion.rs` | 161 + render | Trait compiled, caller dormant | No caller outside the impl file | GPU floats, not a determinism candidate |
| Geometry clipmap | `astraweave-render/src/clipmap_terrain.rs` (782 LoC) + `shaders/clipmap_terrain.wgsl` | 782 | **Orphan** | Not declared from `astraweave-render/src/lib.rs`. File header marks it deprecated in favor of `LodManager`, which itself has no production caller | — |
| Render-side terrain mesh helper | `astraweave-render/src/terrain.rs` `TerrainRenderer` | — | Example | `terrain_demo`, `biome_showcase`, `weaving_playground`. Uses **legacy** `generate_chunk`, not the climate path. A test comment says `generate_chunk_complete` was removed from the suite because it took ~60s+ | f32 mesh |
| Vegetation LOD / impostors | `astraweave-render/src/vegetation_lod.rs`, `impostor_*.rs`; editor `viewport/engine_adapter.rs` `upload_scatter_placements`, `impostor_registry.rs` | LOD module ~745 | Wired in the **editor** viewport | CPU `select_lod`: full mesh &lt; 50 m, simplified &lt; 150 m, cross-billboard &lt; 500 m, impostor &lt; 1500 m, else cull. Impostor bake is an editor default feature (landed 2026-04-16, see `PROJECT_STATUS.md`) | LOD pick is a function of distance. Instance lists are rebuilt on the CPU |
| Demo prop drawing | PR #205 `main.rs` `sync_foliage` | — | Demo | One instanced model per `(PropKind, primitive)`. `Renderer::add_model` / `update_model_instances` | Instance order follows `HashMap::values()` of chunk coords. The set of spots is deterministic; the buffer order is not |
| Scene partition streaming | `astraweave-scene/src/{world_partition,streaming,partitioned_scene,gpu_resource_manager}.rs` | — | Example | `examples/world_partition_demo`, `examples/veilweaver_slice_loader`. Trace `scene.md` v1.1: streaming is in-design-but-tested | Cell loads from `.ron`, not from a world seed |
| PCG crate | `astraweave-pcg/src/{seed_rng,wfc,layout,encounters}.rs` | — | Tested-dormant | Trace `pcg.md`: zero `use astraweave_pcg` in the workspace. `astraweave-weaving` declares the dep and does not use it. `astraweave-ai-gen/src/generator.rs` imports `MeshGenerator` / `TextureGenerator` / `PcgMesh`, which **do not exist**; that tree has no `Cargo.toml` | `SeedRng` is ChaCha via `StdRng`. WFC takes `rand::Rng`, not `SeedRng` |
| Biome presentation demos | `examples/biome_{gpu,transition,weather}_demo` | — | Example | They consume `BiomeType` / `ClimateMap` and render fog/weather/UBO. They do not generate a walkable world | — |
| Greybox meshes (Veilweaver) | `examples/greybox_generator` | — | Unrelated | Hand-built sanctum rooms. Not terrain | — |

`astraweave-terrain/src` is 36,029 lines across 42 files, of which `mutation_tests.rs` (3,678), `chunk_tests.rs` (532), and `voxel_data_tests.rs` (532) are `#[cfg(test)]`. Integration tests under `astraweave-terrain/tests/` are 32,177 lines in 55 files. `#[test]` attributes: 786 in `src/`, 1,666 in `tests/` (counted on `a3fe3ecd`).

---

## 3. How the Aravath demo uses this today

PR #205 (`aravath_runtime`, bin `aravath_greybox`) depends on `astraweave-terrain`, `astraweave-render`, `astraweave-physics`, and `astraweave-camera`. It does not depend on `astraweave-pcg`.

Startup calls `Greybox::open(VIEW_RADIUS)` with `VIEW_RADIUS = 3` (`aravath_runtime/src/sim.rs`). `Ground::stream_around` keeps a Chebyshev disk of `ChunkCoord` at `y = 0`:

1. `fill_height_chunk` writes solid columns. Height is `island::surface_height`: `detmath` sin/cos, then `floor` to an `i32` clamped to 1..=24. Surface material comes from a second sin/cos biome (Grass / Sand / Rock), except the east road `on_yaqar_road` (`|z - 16| <= 1 && x >= 8`) which forces dirt on the top two voxels. Occasional 3-voxel wood logs are placed when `mix(wx, wz) % 53 == 0` on grass off the road.
2. Edits replay on top (`VoxelEdit { x, y, z, density, material }`).
3. `scatter_chunk` walks x and z from 2 to 29 step 5. Road cells are skipped. Each cell may emit grass (`n % 5`), bush (`n % 9`), oak/pine (`n % 7`) on grass; palm and rock on sand; rock and pine on rock. Scale is a constant per kind. Yaw uses f32 `TAU`.
4. `DualContouring::generate_mesh` (the **branch** mesher) builds a `ChunkMesh`. `chunk_parts` splits triangles by `material` so stone, dirt, grass, sand, and wood upload as separate meshes. Physics gets one static trimesh per chunk.
5. Props are hidden when the voxel under them is not solid (`visible_props`).
6. `sync_foliage` instances each KayKit primitive. Assets on that branch: `assets/aravath/models/nature/{grass,plant_bush,rock_largeA,tree_oak,tree_palmShort,tree_pineDefaultA}.glb`. Diffuse terrain textures are `assets/aravath/textures/{dirt,grass,sand,stone,wood}_diff_1k.jpg`.
7. `E` digs a sphere of air (radius 2.5). `R` places a supported sphere (radius 1.1) of the material selected with keys 1–5 (stone, dirt, grass, sand, wood). Saves store the edit log, not the whole grid.

The opening fantasy in the PR body — forested western hills, dirt road east toward Yaqar, engine sky, fog, and terrain materials — is this heightfield plus the engine atmosphere. It is not `WorldGenerator`, and the “forest” is a 5 m lattice that also plants palms wherever the sand predicate hits. There is no scatter LOD. Chunks outside the ring are dropped and regenerated from the height function plus the edit log, so streaming does not need a chunk cache beyond the edits.

`main.rs` on that branch imports `astraweave_terrain::BiomeType` and does not use it to generate the ground. That import is leftover surface, not a second biome pipeline.

---

## 4. Git history

`git log` on the cloud snapshot is shallow. The history below is the GitHub commit list for `astraweave-terrain` (142 commits, 2025-09-14 through 2026-07-26). Dates are commit dates. Messages are the subject lines that mark a wave, not every commit.

| Wave | When | What landed |
|---|---|---|
| Birth | 2025-09-14 | `#77` procedural biomes and worldgen, `#85` biome building. `scatter.rs` is in this generation |
| Compile repair | 2025-09-16 | `#93`, `#96` terrain/environment compile and test fixes |
| Hybrid voxel | 2025-10-02 | `#112` voxelization, `voxel_data.rs`, `meshing.rs`. Scene streaming file births the same week |
| Week 3–4 | 2025-10-10 .. 2025-10-11 | SIMD noise, async `background_loader`, LOD manager. PCG crate births ~2025-10-01 |
| Mutation wave | 2026-02-09 .. 2026-02-21 | Large `wave2_*` / `wave3_*` test files. Scatter tests sped from a 300 s case to ~2 s (commit message, not a runtime benchmark) |
| F-series quality | 2026-04-20 .. 2026-04-28 | Halo erosion, continental noise, Whittaker lookup, six archetypes, scale retune (chunk 256→512 WU, resolution 64→96). Editor scatter cull/streaming note on 2026-04-15. Vegetation LOD + clipmap file on 2026-04-10 .. 2026-04-11. Compressed voxels 2026-04-10 |
| Splines + mask | 2026-04-30 .. 2026-05-01 | Phase 1.X-F: `Spline1D`, per-archetype bootstrap, `RegionalArchetypeMask` wired into the generator as an opt-in `Some` branch |
| E3 | 2026-07-03 `d506658d` | Climate-driven splines, per-archetype landform, provisional-height biome classification. Retires the old D5FIX byte-identity contract on purpose |
| T.2a / ED-3 | 2026-07-24 .. 2026-07-26 | Boreal temperature threshold, spline `expect()` removal, amplitude slider actually multiplies `params.base_elevation_amplitude` |

No commit on `main` after 2026-07-26 touches `astraweave-terrain`. The meshing rewrite exists only on `cursor/aravath-greybox-b3de`.

---

## 5. Duplicates and dead ends

These are the paths an implementation pass must not grow a third copy of.

1. **Two ground types.** Heightmap `TerrainChunk` (editor, climate, scatter-on-height) and voxel `VoxelChunk` (demo, dig/place). The demo’s requirement for editable blocks selects the voxel type for the runtime. The heightmap remains the editor authoring world.
2. **Three heightmap entry points.** `generate_chunk_with_climate` (editor), `generate_chunk` (legacy, `TerrainRenderer`), `generate_chunk_with_scatter` (legacy chunk + scatter + structures, used by `terrain.rs`). The editor does not use `generate_chunk_with_scatter`; it calls hierarchical scatter itself.
3. **Two scatter implementations.** `VegetationScatter` (Poisson, biomes, slope filter, KayKit paths in `biome.rs`, editor LOD) and Aravath `scatter_chunk` (integer lattice). Zone scatter is a third placer, for `.blend` polygons, and its editor call site is unseeded.
4. **Two meshers for the same voxels.** `main` `DualContouring` (one vertex per cell, QEF average, edge lookup by `edge_key.min`) and the PR #205 mesher (one vertex per crossed edge, Bourke corners). `hybrid_voxel_demo` still calls the `main` mesher, including a comment that it fills 10 km (`examples/hybrid_voxel_demo/src/main.rs` `generate_procedural_terrain(..., 10000.0)`).
5. **Two streaming stacks, neither used by the editor.** `BackgroundChunkLoader` + `LodManager` + `MorphingLodManager`, and `astraweave-scene` `WorldPartitionManager`. The demo invented a third, smaller one (`stream_around` + sorted eviction). Clipmap terrain is a fourth design, and it is not compiled.
6. **Two RNG doctrines.** `astraweave-pcg::SeedRng` is the determinism primitive named by `docs/audits/DETERMINISM_AUDIT_JAN_2026.md` and by `pcg.md`. Terrain scatter uses `StdRng` directly. WFC does not accept `SeedRng`. The demo uses an integer mix and `detmath`, which is the right shape for a cross-OS hash and is not shared with the crate.
7. **Asset path split.** Editor biomes reference `assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/...` and `assets/imported/verdant_trail/...`. The demo vendors six glbs under `assets/aravath/models/nature/`. Block Bits are not referenced anywhere in `main` or in the PR #205 file list. High-fidelity PBR pine `.glb` / `.exr` assets are not in tree.

`docs/src/core-systems/terrain.md` is the aspirational wiki (`CLAUDE.md` documentation-hazard class). It quotes a single “15.06 ms/chunk” figure. Do not treat it as a measurement.

---

## 6. Tests

Terrain tests are extensive and campaign-shaped (`phase_1_6_f3_*`, `phase_1_6_f4_*`, `phase_1_x_f4_*`, `wave2_*`, `wave3_*`, plus in-module tests). They lock halo cropping, pre-erosion biome weights, seam continuity, archetype census, scatter filters, meshing, and voxel storage.

Last measured full run, recorded in `docs/architecture/terrain.md` §10 and **not re-run for this note**:

- 2026-07-20: 2,439 passed / 63 failed. The failures are pre-E3 golden and baseline assertions (D5FIX splines, old Perlin/RidgedMulti defaults, `golden_sample_height_*`). Adjudicated in `docs/audits/E3_PREFLIGHT_2026-07.md` §2.4 as held for a single re-bake (beat T.G), not as piecemeal fixes.
- 2026-07-24, T.2a, `--release`: 2,440 passed / 64 failed / 15 ignored. The extra failure is `phase_1_6_f4_b_3_d_3_mixed_climate_chunk_produces_varied_biomes` after `TUNDRA_MAX_TEMP_C` moved from 0.0 to −5.0. The trace classifies it as structural, not a re-bake golden. Detail: `docs/audits/T2A_OUTCOME.md` §8.1.

Ignored tests include the D.3 chunk-time probe (`phase_1_6_f4_b_3_d_3_perf.rs`, `#[ignore]`) and other diagnostic grids. `streaming_integrity` was marked `#[ignore]` in 2026-02 because it was slow under mutation testing.

There is no cross-OS worldgen hash in CI. The Aravath determinism workflow on PR #205 (`.github/workflows/aravath-determinism.yml`) hashes a **recorded gameplay replay** (`replay_matches_golden_hash`, `aravath_runtime/golden/replay.hash`) on Linux and Windows. That hash covers player state after a script that includes a dig. It does not hash an unbounded generated world, and it does not cover `WorldGenerator`.

`gh issue list --label deferred-failure` returned no issues on 2026-10-14. This agent’s `gh` is read-only, so the held golden failures and the `SystemTime` zone-scatter seed are recorded here instead of filed. The implementation phase files one `deferred-failure` issue per item that stays unfixed, with root cause and plan.

---

## 7. Performance (cited, not re-benchmarked)

This phase did not run Criterion or the ignored perf tests. Numbers below are prior measurements. They describe different functions, which is why they disagree.

| Measurement | Figure | Source | What it actually timed |
|---|---|---|---|
| Heightmap 64² | 1.98 ms | `docs/journey/weeks/WEEK_1_COMPLETION_SUMMARY.md` (2025-10 week 1) | `TerrainNoise::generate_heightmap`, erosion off |
| Heightmap 128² | 6.85 ms | same | same |
| `world_chunk_generation` | 19.8 ms | same | `WorldGenerator::generate_chunk`, that week’s config |
| Bench table “world chunk” | 48.6 ms, and also “15.06 ms under 60 FPS budget” | `docs/masters/MASTER_BENCHMARK_REPORT.md` §5.6 | The two cells contradict each other. Treat as stale summary, not a spec |
| Climate path, 121 chunks, Temperate | ~60 s (over a 30 s editor budget). Cold/Highland 36–39 s. Arid/Tropical/Wetland 16–27 s | `docs/current/TERRAIN_GENERATION_QUALITY_CAMPAIGN.md` F.3-phase-2 closeout, 2026-04-23 | `generate_chunk_with_climate` with halo erosion. Rayon was deferred |
| D.3 mean chunk | 0.617 s (+24.6% vs the F.4.B.2.G baseline) | same campaign doc, D.3 perf line | One climate chunk after per-vertex biome lookup. Later E3 spline work is not in this number |
| Scatter test wall time | ~300 s → ~2 s | commit message 2026-02-21 | A test, not frame time |
| `generate_chunk_complete` | “~60s+”, dropped from a render test | `astraweave-render/tests/wave2_terrain_quad_registry_overlay_remediation.rs` comment | Legacy chunk + full scatter |

The hot editor function is `apply_per_biome_modulation_to_halo`: for a 96-resolution chunk with a 1-chunk halo that is about 288² samples, each doing climate, spline, and a second noise evaluation, then a droplet pass over the halo (`terrain.md` §9). Heightmap generation is single-threaded. The only `rayon` use in the crate is `AsyncMeshGenerator` on the voxel path.

Editor vegetation is GPU-instanced per mesh, with CPU LOD selection at the distances in §2. The 2026-04-08 `docs/current/TERRAIN_SCATTER_FIX_PLAN.md` predates the impostor work and still describes “upload all instances, no LOD”. The impostor path landed afterward (editor default feature `impostor-bake`). The floating-tree / pivot / bilinear-vs-triangle notes in that plan were not re-verified visually in this phase; treat them as an open defect list, not as closed.

Demo cost model, from the code rather than a profile: 49 chunks in the ring (radius 3), each 32×32 columns, scatter stride 5 so 6×6 candidate cells per chunk (1,764 cells). Several of those cells emit more than one prop. Each prop is a low-poly KayKit glb, instanced per primitive, with no LOD and no distance cull. The source comment says that ring is comfortable on a GTX 1660. A frame-time number was not captured here.

---

## 8. Determinism

The mandate for this workstream is bit-identical generation across Windows, Linux, and macOS. Current pieces, and whether they meet it:

| Piece | Meets a cross-OS bit hash? | Why |
|---|---|---|
| Demo column height and material | Yes, if the hash is of the integers | `detmath` sin/cos, then `floor` to `i32`. Materials are `u16` |
| Demo `mix` and which cell gets which prop kind | Yes | Wrapping integer hash. Kind tests are `%` on that integer |
| Demo yaw and prop scale | Yaw is a risk | `(n & 255) as f32 / 255.0 * TAU`. Scale constants are f32 but they are literals. Prefer storing yaw as the `u8` and expanding in the renderer |
| Demo instance buffer order | No | `sync_foliage` iterates `HashMap::values()`. Sort by `(x, z, kind)` before upload if a buffer hash is required. The replay hash sorts gameplay state; it does not hash foliage order |
| Demo mesh, binary densities | Likely, after the branch mesher | Densities are 0 or 1, so the edge lerp `t` is 0.5, which is exact in f32 for these magnitudes. Vertex order follows the cell walk. Confirm with a hash rather than assuming |
| `StdRng` / `SeedRng` | The stream is cross-OS | ChaCha. `gen_f32` values are still f32 |
| `noise` crate Perlin/Fbm used by `WorldGenerator` | Not a bit-hash candidate | f32 evaluation, seed narrowed to `u32`, platform libm and FMA policy can move ulps. The crate already documents an f32-vs-f64 step bug of 125 world units (`lib.rs` halo comments, terrain trace invariant 2) |
| Editor zone scatter | No | Seed is `SystemTime` (`tools/aw_editor/src/main.rs` ~7846) |
| Droplet erosion | Integer spawn hash, f32 height writes | Overlap can be “near-identical” (trace invariant 5), which is weaker than bit-identical |
| `HashMap` in meshing, LOD, partition, structures | Safe only when used as a key lookup | Unsafe when iteration order becomes output order. Demo eviction already sorts (`sort_coords`) for this reason |

`astraweave-pcg::SeedRng` is the right crate-level RNG if a later phase needs a named stream. It is the wrong tool for heights and scatter that must be bit-identical: those should be integers until a render conversion.

### Proposed cross-OS worldgen hash

Add one test, owned by the runtime that actually generates the demo world (today `aravath_runtime`, moved next to the generator if that generator moves into `astraweave-terrain`).

1. Fixed seed. The demo has no world seed yet; introduce `WORLD_SEED: u64` and thread it through `mix` and the height function without changing the integer character of the hash.
2. For a fixed list of chunk coords (include chunk `(0,0,0)`, a negative coord, and a coord on the road), build the canonical record:
   - columns: `(x: i32, z: i32, height: i32, materials: [u16; height+1])`
   - props: `(kind: u8, x: i32, z: i32, yaw_quanta: u8, scale_milli: u16)`
3. Sort columns by `(x, z)` and props by `(x, z, kind, yaw_quanta)`.
4. Feed the bytes to a stable hash (SHA-256, or the demo’s existing `Canon` if it is specified as little-endian integers only). Do not use `std` `DefaultHasher`.
5. Assert against a golden file checked in next to `aravath_runtime/golden/replay.hash`.
6. Run it in the existing Linux and Windows determinism workflow. macOS joins when a macOS job exists; until then the test is still the contract, and Linux vs Windows is the CI proof.
7. Keep this hash on the **integer world**, not on GPU pixels and not on `WorldGenerator` f32 samples. A second, explicitly named fingerprint of `sample_height(...).to_bits()` may be stored as a diagnostic. It must not be the gate until the noise that feeds it is integer or fixed-point.

Acceptance for the hash itself: two processes on the same OS match, and the Linux job matches the Windows job, for the golden seed. Changing the seed changes the digest. Digging a voxel changes a later regenerate-from-edits digest in a second golden, so edits stay in the contract.

---

## 9. What “forested western hills” should reuse

KayKit CC0, already integrated by the demo agent and already named from `biome.rs`:

- Demo glbs to keep as the greybox library: oak, pine, palm, bush, rock, grass. Palm stays a sand species. The start ring should come up forest (oak, pine, bush, rock, grass tufts) with the road empty. The current sand predicate will keep planting palms inside the western hills wherever `detmath` sin/cos exceeds 0.55. That is a content bug relative to the brief, not a missing system.
- Editor `BiomeConfig` paths under `assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/` and `Color2/` are the fuller set (bare trees, extra rocks). `THIRD_PARTY_LICENSES.md` records nine of those plus the forest textures as the vendored sample. Scatter for the demo should prefer the six glbs the demo already loads, and grow into the Color1/Color2 set only when those files are on the demo’s asset path.
- Block Bits: no crate, path, or demo file references them yet. When they arrive, they map onto the existing `u16` materials (stone, dirt, grass, sand, wood) as the mesh source for voxel faces. They do not replace `VoxelGrid`.
- Later PBR pines (`.glb`, PBR maps, `.exr`) replace the KayKit mesh on the same prop records. LOD (the editor’s 50 / 150 / 500 / 1500 m chain, or a shorter one) belongs to that swap. The greybox lattice does not need impostors to be a valid showcase.

---

## 10. Proposed plan (not started)

Ordered. Each item has an acceptance check. Items assume review approval and a dependency on PR #205 for the greybox crate and the mesher rewrite. This branch does not merge that PR and does not push to `main`.

### P0 — Keep one runtime ground

Leave `WorldGenerator` as the editor heightmap. Drive Aravath from the voxel column generator that already exists on PR #205. Move `fill_height_chunk`, `surface_height`, `biome_at`, `mix`, and `scatter_chunk` behind a named API only if the demo crate is the wrong long-term home; do not reimplement them on top of `generate_chunk_with_climate`.

Acceptance: the greybox still digs, places, saves, and streams. A grep of `aravath_runtime` shows no call to `generate_chunk_with_climate`. A chunk of the start area is grass and forest props, the road is dirt and empty of props, and palm appears only on the sand predicate after that predicate is pushed out of the opening ring (or the opening ring’s biome is forced to grass).

### P1 — Land the demo mesher as the voxel mesher

Take the PR #205 `meshing.rs` behavior (Bourke corners, one vertex per crossed edge) onto the engine crate with a test the `main` mesher fails today: a single flipped edge produces a triangle whose vertices lie on that edge, and two runs emit identical vertex bytes.

Acceptance: `cargo test -p astraweave-terrain --lib meshing` passes, including a byte-identity test of a 32³ binary chunk. `hybrid_voxel_demo` still compiles against the same mesher. No new `unsafe`.

### P2 — Cross-OS worldgen hash

Implement §8. Wire it beside the existing Aravath replay hash.

Acceptance: golden file matches on Linux and on Windows in CI. The test does not read the clock, does not iterate a `HashMap`, and hashes integers (yaw as quanta, height as `i32`).

### P3 — Forest scatter on that hash

Replace the `% 5 / % 7 / % 9` stacking (one cell can emit grass and a bush and a tree) with a single integer draw per cell: empty, grass, bush, rock, or tree, with tree split into oak and pine. Keep the stride, or tighten it, so the opening view reads as a forest rather than an orchard. Clear the road. Drop props whose support voxel was dug (already implemented). Sort instance lists before `update_model_instances`.

Acceptance: the §8 golden covers the new draw. A start-chunk census test: road cells have zero props; grass cells have pine+oak+bush+grass and zero palms; at least one rock exists in the opening ring. Instance groups are one draw per primitive, as today.

### P4 — Block edit contract

Keep sphere dig/place as the greybox tool, on density `0.0`/`1.0` and the five material ids, so Block Bits can replace the flat color later without a new edit log. Persist edits as integers. Rebuild a streamed-out chunk from the height function plus the edit log (already the design of `restore_edits`).

Acceptance: existing dig/place/save tests in `aravath_runtime` stay green, and the worldgen hash after a scripted dig matches a second golden.

### P5 — Do not wire the dormant stacks in this workstream

`BackgroundChunkLoader`, `LodManager`, clipmap, GPU erosion, `RegionalArchetypeMask`’s `Some` branch, Phase 10 persistence, and `astraweave-pcg` WFC stay dormant. Scatter LOD waits for the PBR pines. Filing or fixing the held terrain goldens (64 failures as of 2026-07-24) is a separate director decision (re-bake T.G, or one `deferred-failure` issue). The `SystemTime` zone-scatter seed is a one-line determinism bug in the editor; fix it when that call site is touched, otherwise file it the same way.

Acceptance: the workstream diff does not add a production caller for those dormant types.

### Explicitly out of scope until approved

New noise, a second voxel format, GPU grass, impostor bakes for KayKit, Block Bits import, PBR pine import, and any change to the editor climate pipeline beyond the mesher test in P1.

---

## 11. Evidence index

| Claim | Where it was checked |
|---|---|
| Module list and line counts | `wc -l astraweave-terrain/src/*.rs` on `a3fe3ecd` |
| `#[test]` counts | `rg -c '#\[test\]'` in `src/` and `tests/` |
| Editor climate caller | `tools/aw_editor/src/terrain_integration.rs` (`WorldGenerator::new`, `generate_chunk_with_climate`, `VegetationScatter` ~2856) |
| Zone scatter clock seed | `tools/aw_editor/src/main.rs` ~7846 |
| Legacy renderer path | `astraweave-render/src/terrain.rs` `generate_chunk` / `generate_chunk_with_scatter` |
| Clipmap not compiled | `rg clipmap astraweave-render/src/lib.rs` has no match; `clipmap_terrain.rs` exists |
| `main` mesher key bug | `astraweave-terrain/src/meshing.rs` `vertex_cache` keyed by `IVec3` cell; edge loop reads `edge_key.min` |
| Demo behavior | `git show origin/cursor/aravath-greybox-b3de:aravath_runtime/src/{ground,island,sim,main}.rs` at `9e548c18` |
| Demo mesher delta | `git diff a3fe3ecd origin/cursor/aravath-greybox-b3de -- astraweave-terrain/src/meshing.rs` |
| History | `gh api repos/lazyxeon/astraweave/commits?path=astraweave-terrain` (142 commits) |
| Held failures | `docs/architecture/terrain.md` §10. Not re-executed on 2026-10-14 |
| No `deferred-failure` issues | `gh issue list --label deferred-failure` empty |
| PCG dormant | `docs/architecture/pcg.md` §1 and a workspace grep for `astraweave_pcg::` outside the crate |
| KayKit paths | `astraweave-terrain/src/biome.rs` model_path strings; PR #205 `assets/aravath/models/nature/*.glb` |

---

## 12. What landed after approval

Rebased onto `main` at `85d09a2c` (#210 replay gate, #211 `SimRng`). `Fixed64` is not on `main`. It lives in `astraweave-veil` on PR #205 (`Q32.32`). Yaw uses `astraweave_core::sim::SimRng`, which is on `main`.

| Item | Where |
|---|---|
| P1 mesher | `astraweave-terrain/src/meshing.rs`. `vertex_cache` is `HashMap<EdgeKey, u32>`. `edge_midpoint_is_stored_and_two_runs_match_bits` requires the midpoint `(3, 0, 0.5)` and identical position bits across two runs |
| P0/P3 ground | `column_gen.rs`. Opening `[-96, 128)` is grass. Road `\|z-16\|<=1 && x>=8` is dirt and has no props. Palms only outside the opening. `aravath_runtime` fills and scatters through this module |
| P2 hash | Required case `worldgen_columns` in `determinism-replay.yml`. Test `astraweave-terrain/tests/worldgen_hash.rs` appends the row after the core replay cases |
| Foliage order | `sync_foliage` sorts by `(kind, grid_x, grid_z, yaw_quanta)` before upload |
| Yaw | `SimRng` stream `SIM_SYSTEM_FOREST_SCATTER` (`0x7E22_7101`). The record stores `yaw_quanta: u8`. The render yaw is `quanta * (TAU/256)` and is not hashed |
| Zone scatter seed | `tools/aw_editor/src/main.rs` passes `zone_seed` (`terrain_seed()`), not `SystemTime` |
| LOD | `column_gen::lod_keeps`. From spawn `(16, 16)`, a radius-3 ring produced **2009** props and kept **727** after LOD. CPU generation of that ring was **1.54 ms** (`view_ring_lod_keeps_the_forest_under_budget`). This host has no Vulkan device, so there is no RTX 4060 Ti frame time in this change |
| PBR set | `assets/terrain/polyhaven/pine_forest/manifest.json`. CC0 only. Pine-forest ground (`forest_ground_04`, `rocky_trail`, `pine_bark`), `forest_leaves_02`, `leafy_grass`, rocks `aerial_rocks_01` and `coast_land_rocks_01`, models `tree_small_02`, `island_tree_01`, `dry_branches_medium_01`, conifer `pine_tree_01` (<https://polyhaven.com/a/pine_tree_01>, <https://polyhaven.com/license>). KayKit Forest Nature Pack stays the greybox props on PR #205 (<https://kaylousberg.itch.io/kaykit-complete>, CC0). `spruit_sunrise` 2k EXR is the opening IBL file. PR #216 owns binding it. Photogrammetry `.bin` buffers are hashed and not vendored. No Unity Asset Store files |

`TerrainNoise::sample_height` takes `f64` and returns `f32` (`noise_gen.rs`). On rustc 1.89.0, debug and release both return exactly `477.0` for `sample_height(0.0, 0.0)` with seed 42 and the default config. The checked-in golden `35.8488` is the pre-E3 number, and it fails in both profiles the same way. That sample is not an opt-level split on this host. The sim and the replay hash do not call it. A later unification that saves `WorldGenerator` output has to stop using this f64 Perlin or prove the bits in the three-OS job. Until then it stays off `worldgen_columns`.

The Aravath gameplay golden moved from `9b607d1a840ecd64` to `fd303f2adf402881` because the ground under the recorded script changed. Two independent runs of that script match.

---

## 13. Unification plan

P5 as “leave the dormant stacks dormant” is rejected. U1, U2, and U4 are in this branch. U3 binds voxel materials through `ModelSurfaceMaps`. KayKit props are still the greybox meshes.

One pipeline means one function from `(seed, x, z)` to a column, one mesher, one scatter record, and one LOD predicate. The editor and the demo both call it. `WorldGenerator`’s f32 path remains an authoring preview until U2 replaces its saved output.

| Surface | Decision | Phase |
|---|---|---|
| `column_gen` | Sim and save ground. Editor scene save writes `worldgen_record()` plus `erosion.txt` saying the preview is not saved | U2 done |
| `DualContouring` edge cache | The only voxel mesher | done |
| `WorldGenerator` / `noise` crate f64 Perlin | Editor preview only. Not written to saves and not in the replay hash. U2 either replaces `sample_height` with the integer field or adds a three-OS bit lock before any save reads it | U2 |
| Demo `stream_around` | `ColumnStream` in `astraweave-terrain`. `aravath_runtime` calls it | U1 done |
| Heightmap async loader, LOD hysteresis, LOD morph, streaming diagnostics | Deleted in the same change as `ColumnStream` | U1 done |
| Geometry clipmap | Deleted. Distance LOD is `lod_keeps` | U4 done |
| Droplet erosion | Offline bake into the integer height field for the editor’s “generate” button. Not a per-frame sim step | U2 |
| `VegetationScatter` Poisson + `StdRng` | Editor keeps it until U3, then it places from `column_gen` prop records for forest biomes and keeps Poisson only for authored densities. Yaw already has to be `SimRng` if a new caller appears | U3 |
| `ZoneScatterGenerator` | Stays for `.blend` polygons. Seed is the world seed (done). U3 adds it to the replay hash if a save stores the placements | U3 |
| `astraweave-pcg` | Wired. `SeedRng: RngCore`, `WfcGrid::collapse_seeded`, editor dungeon button calls `seeded_dungeon_tile_count` | U4 done |
| `RegionalArchetypeMask` Some-branch | Deleted. The panel can store a mask. Generation uses the selected archetype. Painted and unpainted heights match | U4 done |
| GPU erosion / the accelerator trait | Deleted, with the render impl and shader. CPU erosion preview is not the save | U4 done |
| Phase 10 solver / modifier / persistence / compressed voxels | Solver kept. Modifier, persistence format, and compressed voxels deleted. Aravath save is the edit log plus the integer record | U4 done |
| Poly Haven 2k set | Voxel materials 1–5 load `diff` / `nor_gl` / `arm` through `pbr_maps::split_polyhaven_arm` and `Renderer::add_model_with_pbr_textures`. KayKit props stay colored meshes | U3 plumbing done |

### U1 — one streamer

Move `Ground::stream_around`’s keep/evict/sort into `ColumnStream` in `astraweave-terrain`. `aravath_runtime` calls it. Delete `background_loader.rs`, `lod_manager.rs`, `lod_blending.rs`, and `streaming_diagnostics.rs` in that change, with their tests rewritten against `ColumnStream` or removed with the files.

Acceptance: `rg BackgroundChunkLoader` is empty outside git history. A test streams to a negative chunk and back without `HashMap` iteration in the eviction order. `worldgen_columns` still matches.

### U2 — one height

`surface_height` is the only height the editor writes into a chunk it saves. The climate path may still draw a preview, but the save stores the integer columns. Erosion, if enabled, is a deterministic bake over those integers (fixed-point droplet or an explicit “not saved” banner). `sample_height`’s f64 path is not in the save.

Acceptance: a save/load round trip of a chunk hashes into `worldgen_columns` or a sibling required case. Debug and release on the three-OS job match.

### U3 — one scatter and one material bind

Editor forest scatter reads `prop_at`. `lod_keeps` is the editor’s distance test too. Voxel materials 1–5 sample the pine-forest manifest (`nor_gl`, `arm`, `diff`). KayKit props stay instanced and sorted.

Acceptance: opening census test still has trees, no palms, an empty dirt road. A headless frame on a real GPU records frame time for the 727-instance LOD set. The manifest test still checks SHA-256.

### U4 — delete or wire the rest

For each row in the table that says “or”: the change either adds a non-test caller and a test, or deletes the file. `astraweave-pcg` is the first of these. `clipmap_terrain.rs` is a delete.

Acceptance: `rg` for each deleted type is empty. `cargo test -p astraweave-terrain --lib` passes. The determinism workflow stays required for `worldgen_columns`.

---

## 14. Done and remaining (2026-10-14)

| Item | Status |
|---|---|
| P1 mesher, edge-keyed `vertex_cache`, byte-identity regression | Done |
| P2 `worldgen_columns` required in `determinism-replay.yml` | Done |
| P2 sorted foliage upload, `SimRng` yaw quanta, editor zone seed is `terrain_seed()` | Done |
| P0 integer columns, dirt road, forest opening, no palms in the opening | Done |
| P3 scatter LOD and instancing. 2009 props, 727 kept, 1.54 ms CPU | Done on CPU. RTX 4060 Ti frame time is still unmeasured: this host has no Vulkan device |
| P4 dig (`E`) and place (`R`) on density 0/1 voxels, save/replay | Done. `aravath_runtime` lib tests cover dig, place, and the replay golden `fd303f2adf402881` |
| Game-ready scatter meshes | Done. `gltfpack` 1.3, command in `build_game_lod.py` and `manifest.json`. LOD0 is under 15k triangles (`pine_tree_01` 14,790). LOD1, LOD2, and a 4-triangle KTX2 cross billboard are committed with SHA-256. The 949 MB source buffer stays a build input |
| Greybox still draws KayKit | Remaining. `load_colored` reads vertex colors and ignores images. The new glbs are textured KTX2. Swapping them in without a texture bind would draw flat white trees. That bind is U3, with the render agent’s IBL bind of `spruit_sunrise` (PR #216) |
| U1 `ColumnStream` and deletion of the unused heightmap streamer | Done. Negative-chunk eviction test. `worldgen_columns` still matches |
| U2 integer height as the saved ground | Done. Editor `.columns` file and Aravath meta `ground`. Erosion preview writes `erosion.txt` and is not the ground bytes |
| U3 textured voxel materials via `ModelSurfaceMaps` | Plumbing done. KayKit scatter meshes are still the greybox draw |
| U3 GPU frame time | `--frame-times` writes `aravath_frame_times.csv` beside the exe (`frame,cpu_ms`). This host has no Vulkan device, so the RTX 4060 Ti number is still the owner's |
| U4 wire or delete dormant stacks | Done. Decisions are the table in §13 |
| Editor forest scatter still uses `VegetationScatter` for non-forest authoring | Remaining. Forest props in the demo already come from `prop_at` |
