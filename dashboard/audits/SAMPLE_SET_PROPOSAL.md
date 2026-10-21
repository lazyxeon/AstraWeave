# Sample Set Proposal — AD.0 (Curation, Read-Only)

**Date**: 2026-07-05 | **Status**: DRAFT — awaiting director ratification | **Session**: read-only; nothing in the repo was moved, deleted, or generated.

**Evidence sources**: `docs/audits/ASSET_REGISTRY.csv` (91,034 rows) + `ASSET_AUDIT_REPORT.md` (2026-07 static audit); plus measurements taken this session (cited per row/section). Where this proposal needed liveness data the registry lacks (its `referenced` column is a static string-match), each candidate citation was re-verified this session as LIVE / DEAD / TEST-LITERAL by checking bin/module wiring and reading the code at the cited line (method notes in section 2).

## 1. Summary & budget status

| Bucket | Files | Bytes | MB |
|---|---|---|---|
| Tier 1+2 kept **as-is** (tables A-F, I) | 90 | 39,030,949 | 37.22 |
| Tier 2 kept as **DERIVATIVE(1K)** (table H — estimates; generated in AD.4) | 63 | ~48,447,747 | ~46.20 |
| Tier 3 acquisition (director-assisted, D5) | 1 | ~10,800,000 | ~10.30 |
| **Proposed total** | **154** | **~98,278,696** | **~93.7** |

**Budget: ~93.7 MB proposed vs 100 MB target / 250 MB hard ceiling / 50 MB per-file cap — UNDER TARGET.** Largest kept file: `spruit_sunrise_2k.hdr` (5,934,209 B). No kept file approaches the 50 MB cap. Ranked further cuts in section 7.

All derivative sizes are **estimates** (source size x area ratio to 1024^2) — the one permitted estimate class per the ratified budget; actual sizes are produced and measured in AD.4.

## 2. Method: liveness verification over the registry

The registry's `referenced`/`referenced_by` columns are static string matches (audit section 5 caveat). For Tier 1, every candidate citation was re-verified this session. Surfaces ruled **DEAD or TEST-LITERAL** (their assets dropped from Tier 1 unless independently live):

- `examples/unified_showcase/src/main_bevy_v2.rs` and all `main_temp/main_backup*/main_clean/material*.rs` — orphan source: `autobins=false`, single `[[bin]] src/main.rs`, which declares only `mod gltf_loader` (unified_showcase/Cargo.toml:8,29-31; main.rs:46). **Drops**: `character-a/b/c.glb`, `tree_oak/simple/detailed.glb`, the `assets/models/rock_large*/stone_large*/plant_bush.glb` copies, `planks/roof/cobblestonePainted.png`, materials `*_n/_mra` example refs. (The similarly-named `rock_largeB.glb`/`stone_largeA.glb` kept in table E are DIFFERENT files — Kenney Nature Kit pack copies with independent live biome.rs references.)
- `examples/unified_showcase/src/material_integration.rs` — same orphan status. **Consequence**: the compiled unified_showcase never loads `assets/materials/<biome>` packs; the live consumer is the editor terrain path (see table H rationale).
- `examples/hello_companion/src/scene.rs` — `#![allow(dead_code)]` (scene.rs:9); `hdri_path()`/`npc_model_path()` and the `assets::` constants have zero callers. **Drops**: `Amber.Fbx` (226.7 MB), `Amber_Motion.Fbx`, scene.rs HDRI refs. The visual demo loads via its own literals in `visual_demo.rs` (LIVE).
- `astraweave-render/src/asset_index.rs:242` — `#[cfg(test)]` TOML fixture. **Drops** `rainforest_trail_2k.hdr` from Tier 1.
- `astraweave-terrain/tests/blend_pipeline_e2e.rs` + `biome_pack.rs:1021` — mock manifests, stub bytes, no Blender, never opens the file. **Drops** `assets/Namaqualand.blend` (122.7 MB).
- `tools/aw_asset_cli/tests/mutation_resistant_comprehensive_tests.rs:193-205` — serde round-trip over path strings, no file open. **Drops** `assets/materials/baked/grass.ktx2` + `assets_src/textures/grass.png` (actual literals are `textures/grass.png`/`baked/grass.ktx2`; the registry's suffix-match was generous).

Surfaces verified **LIVE** (citations in tables): editor archetype spawn maps x2 (main.rs:156-188 with callers at 3600/4508; entity_panel.rs:319-328 with caller at 671 -> drained at main.rs:9698-9702), editor Debug-menu loads (menu_bar.rs:388-405 -> main.rs:9357-9373), editor assets-dir sentinel probe (viewport/types.rs:185-207, callers main.rs:5461 / engine_adapter.rs:2080 / branding.rs:28 / terrain_panel.rs:1976), editor terrain scatter (biome.rs BiomeConfig constructors -> terrain_integration.rs:288-296,3220 -> engine_adapter.rs:2443,2600, live viewport path renderer.rs:1545), editor canonical terrain pack (canonical_terrain_pack.rs:5,181,215,293), unified_showcase main.rs loads (1273-1421), hello_companion + veilweaver_demo `--features visual` loads (visual_demo.rs:685,729; visual_renderer.rs:573,592,639), Renderer-owned BiomeMaterialSystem + hdri_catalog (renderer.rs:910,3348; biome_material.rs:99-175; assets/hdri/hdri_catalog.toml).

Rows whose only references are docs, orphan source, or test literals are NOT proposed; they move to packs/quarantine in later beats.

## 3. Tier 1 — referenced by live code (tables A, B, C, E, F, I) and Tier 2 sets (D, H)

### TABLE A-editor-spawn (subtotal 2,380,376 bytes = 2.27 MB)

| path | size_bytes | tier | why | provenance | derivative? | notes |
|---|---|---|---|---|---|---|
| assets/The Complete KayKit Collection v4/KayKit Adventurers 2.0/Characters/gltf/Rogue.glb | 409,188 | 1 | tools/aw_editor/tests/archetype_mesh_assets.rs:18; tools/aw_editor/src/panels/entity_panel.rs:321 | CC0 (KayKit / Kay Lousberg) | as-is |  |
| assets/The Complete KayKit Collection v4/KayKit Adventurers 2.0/Characters/gltf/Barbarian.glb | 386,648 | 1 | tools/aw_editor/tests/archetype_mesh_assets.rs:22; tools/aw_editor/src/main.rs:163 | CC0 (KayKit / Kay Lousberg) | as-is |  |
| assets/The Complete KayKit Collection v4/KayKit Adventurers 2.0/Characters/gltf/Mage.glb | 352,472 | 1 | tools/aw_editor/tests/archetype_mesh_assets.rs:26; tools/aw_editor/src/panels/entity_panel.rs:325 | CC0 (KayKit / Kay Lousberg) | as-is |  |
| assets/The Complete KayKit Collection v4/KayKit Adventurers 2.0/Characters/gltf/Knight.glb | 341,688 | 1 | tools/aw_editor/tests/archetype_mesh_assets.rs:34; tools/aw_editor/src/panels/entity_panel.rs:322 | CC0 (KayKit / Kay Lousberg) | as-is |  |
| assets/The Complete KayKit Collection v4/KayKit Skeletons 1.1/characters/gltf/Skeleton_Warrior.glb | 371,728 | 1 | tools/aw_editor/tests/archetype_mesh_assets.rs:30; tools/aw_editor/src/panels/entity_panel.rs:323 | CC0 (KayKit / Kay Lousberg) | as-is |  |
| assets/The Complete KayKit Collection v4/KayKit Skeletons 1.1/characters/gltf/Skeleton_Golem.glb | 397,816 | 1 | tools/aw_editor/src/panels/entity_panel.rs:324 | CC0 (KayKit / Kay Lousberg) | as-is |  |
| assets/3D assets/Castle Kit/Models/GLB format/tower-square.glb | 16,564 | 1 | tools/aw_editor/tests/archetype_mesh_assets.rs:38; tools/aw_editor/src/main.rs:178 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Survival Kit/Models/GLB format/box-large.glb | 14,072 | 1 | tools/aw_editor/tests/archetype_mesh_assets.rs:42; tools/aw_editor/src/main.rs:180 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Survival Kit/Models/GLB format/campfire-pit.glb | 26,468 | 1 | tools/aw_editor/tests/archetype_mesh_assets.rs:46; tools/aw_editor/src/main.rs:181 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Survival Kit/Models/GLB format/rock-a.glb | 10,812 | 1 | tools/aw_editor/tests/archetype_mesh_assets.rs:50; tools/aw_editor/src/main.rs:183 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Fantasy Town Kit/Models/GLB format/cart.glb | 52,920 | 1 | tools/aw_editor/tests/archetype_mesh_assets.rs:54; tools/aw_editor/src/main.rs:185 | CC0 (Kenney) | as-is |  |

### TABLE B-debug-props (subtotal 82,304 bytes = 0.08 MB)

| path | size_bytes | tier | why | provenance | derivative? | notes |
|---|---|---|---|---|---|---|
| assets/models/barrels.glb | 63,196 | 1 | tools/aw_editor/src/ui/menu_bar.rs:391 | UNKNOWN | as-is |  |
| assets/models/bed.glb | 19,108 | 1 | tools/aw_editor/src/ui/menu_bar.rs:398 | UNKNOWN | as-is |  |

### TABLE C-examples (subtotal 3,473,031 bytes = 3.31 MB)

| path | size_bytes | tier | why | provenance | derivative? | notes |
|---|---|---|---|---|---|---|
| assets/models/tree_pineDefaultA.glb | 17,220 | 1 | examples/unified_showcase/src/main.rs:1274 | UNKNOWN | as-is |  |
| assets/models/tree_pineRoundA.glb | 14,488 | 1 | examples/unified_showcase/src/main.rs:1276 | UNKNOWN | as-is |  |
| assets/models/tree_pineTallA.glb | 7,200 | 1 | examples/unified_showcase/src/main.rs:1275 | UNKNOWN | as-is |  |
| assets/models/tent_smallOpen.glb | 13,604 | 1 | examples/unified_showcase/src/main.rs:1369 | UNKNOWN | as-is |  |
| assets/models/tent_detailedClosed.glb | 15,460 | 1 | examples/unified_showcase/src/main.rs:1421 | UNKNOWN | as-is |  |
| assets/models/campfire_logs.glb | 9,284 | 1 | examples/unified_showcase/src/main.rs:1393 | UNKNOWN | as-is |  |
| assets/models/tree_default.glb | 9,428 | 1 | examples/hello_companion/src/visual_demo.rs:729; examples/veilweaver_demo/src/visual_renderer.rs:639 | UNKNOWN | as-is |  |
| assets/textures/cobblestone.png | 1,741,763 | 1 | examples/unified_showcase/src/main.rs:1130 | UNKNOWN | as-is |  |
| assets/textures/grass_bermuda_01_diff_1k.jpg | 175,425 | 1 | examples/veilweaver_demo/src/visual_renderer.rs:592 | UNKNOWN | as-is |  |
| assets/Astraweave_logo.jpg | 1,469,159 | 1 | tools/aw_editor/src/ui/branding.rs:11; tools/aw_editor/src/splash.rs:21 | UNKNOWN | as-is |  |

### TABLE D-hdri (subtotal 21,655,365 bytes = 20.65 MB)

| path | size_bytes | tier | why | provenance | derivative? | notes |
|---|---|---|---|---|---|---|
| assets/hdri/polyhaven/kloppenheim_02_puresky_2k.hdr | 5,563,538 | 2 | examples/hello_companion/src/visual_demo.rs:685; hdri_catalog.toml:15 (catalog default) | UNKNOWN | as-is |  |
| assets/hdri/polyhaven/kloppenheim/kloppenheim_06_puresky_2k.hdr | 4,434,394 | 2 | examples/unified_showcase/src/main.rs:764; examples/veilweaver_demo/src/visual_renderer.rs:573 | CC0 (Poly Haven) | as-is |  |
| assets/hdri/polyhaven/spruit_sunrise/spruit_sunrise_2k.hdr | 5,934,209 | 2 | hdri_catalog.toml:63 (morning fallback all biomes; Renderer-owned BiomeMaterialSystem, renderer.rs:910) | CC0 (Poly Haven) | as-is |  |
| assets/hdri/polyhaven/venice_sunset/venice_sunset_2k.hdr | 5,723,224 | 2 | hdri_catalog.toml:71 (evening fallback all biomes; renderer.rs:910) | CC0 (Poly Haven) | as-is |  |

### TABLE E-scatter (subtotal 11,299,439 bytes = 10.78 MB)

| path | size_bytes | tier | why | provenance | derivative? | notes |
|---|---|---|---|---|---|---|
| assets/3D assets/Nature Kit/Models/GLTF format/grass_large.glb | 18,504 | 1/2 | astraweave-terrain/src/biome.rs:1803 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/grass_leafs.glb | 4,608 | 1/2 | astraweave-terrain/src/biome.rs:1192 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/grass_leafsLarge.glb | 13,988 | 1/2 | astraweave-terrain/src/biome.rs:1481; astraweave-terrain/src/biome.rs:1818 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/hanging_moss.glb | 4,776 | 1/2 | astraweave-terrain/src/biome.rs:1540 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/lily_large.glb | 8,044 | 1/2 | astraweave-terrain/src/biome.rs:1525 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/lily_small.glb | 5,664 | 1/2 | astraweave-terrain/src/biome.rs:1832 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/mushroom_redTall.glb | 6,504 | 1/2 | astraweave-terrain/src/biome.rs:1496 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/mushroom_tanTall.glb | 6,500 | 1/2 | astraweave-terrain/src/biome.rs:1511 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/plant_bushSmall.glb | 3,212 | 1/2 | astraweave-terrain/src/biome.rs:1354 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/rock_largeB.glb | 8,560 | 1/2 | astraweave-terrain/src/biome.rs:1207 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/rock_smallB.glb | 3,528 | 1/2 | astraweave-terrain/src/biome.rs:1847 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/rock_smallFlatA.glb | 3,300 | 1/2 | astraweave-terrain/src/biome.rs:1324 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/rock_tallA.glb | 12,072 | 1/2 | astraweave-terrain/src/biome.rs:1221 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/stone_largeA.glb | 7,124 | 1/2 | astraweave-terrain/src/biome.rs:1236 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/stone_smallA.glb | 2,620 | 1/2 | astraweave-terrain/src/biome.rs:1339 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/stump_old.glb | 9,564 | 1/2 | astraweave-terrain/src/biome.rs:1466 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/tree_pineGroundA.glb | 6,704 | 1/2 | astraweave-terrain/src/biome.rs:1309 | CC0 (Kenney) | as-is |  |
| assets/3D assets/Nature Kit/Models/GLTF format/tree_pineSmallA.glb | 12,820 | 1/2 | astraweave-terrain/src/biome.rs:1149 | CC0 (Kenney) | as-is |  |
| assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Rock_3_A_Color1.gltf | 3,056 | 1/2 | astraweave-terrain/src/biome.rs:1251 | CC0 (KayKit / Kay Lousberg) | as-is |  |
| assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Tree_3_C_Color1.gltf | 3,066 | 1/2 | astraweave-terrain/src/biome.rs:1163 | CC0 (KayKit / Kay Lousberg) | as-is |  |
| assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Tree_5_C_Color1.gltf | 3,065 | 1/2 | astraweave-terrain/src/biome.rs:1774 | CC0 (KayKit / Kay Lousberg) | as-is |  |
| assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Tree_7_A_Color1.gltf | 3,062 | 1/2 | astraweave-terrain/src/biome.rs:1788 | CC0 (KayKit / Kay Lousberg) | as-is |  |
| assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/Tree_Bare_1_A_Color1.gltf | 3,075 | 1/2 | astraweave-terrain/src/biome.rs:1177 | CC0 (KayKit / Kay Lousberg) | as-is |  |
| assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color2/Tree_6_C_Color2.gltf | 3,065 | 1/2 | astraweave-terrain/src/biome.rs:1452 | CC0 (KayKit / Kay Lousberg) | as-is |  |
| assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color2/Tree_Bare_1_B_Color2.gltf | 3,074 | 1/2 | astraweave-terrain/src/biome.rs:1437 | CC0 (KayKit / Kay Lousberg) | as-is |  |
| assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color2/Tree_Bare_1_C_Color2.gltf | 3,079 | 1/2 | astraweave-terrain/src/biome.rs:1384 | CC0 (KayKit / Kay Lousberg) | as-is |  |
| assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color2/Tree_Bare_2_B_Color2.gltf | 3,077 | 1/2 | astraweave-terrain/src/biome.rs:1369 | CC0 (KayKit / Kay Lousberg) | as-is |  |
| assets/imported/verdant_trail/meshes/dead_tree_trunk.001.glb | 2,295,256 | 1/2 | astraweave-terrain/src/biome.rs:342; assets/imported/verdant_trail/verdant_trail.biomepack.json:605 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/dead_tree_trunk_02.glb | 1,975,828 | 1/2 | astraweave-terrain/src/biome.rs:355; astraweave-terrain/src/biome.rs:959 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/dry_branches_medium_01_a.002.glb | 168,716 | 1/2 | astraweave-terrain/src/biome.rs:368; assets/imported/verdant_trail/verdant_trail.biomepack.json:1170 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/grass_debris_a.glb | 12,688 | 1/2 | astraweave-terrain/src/biome.rs:1052; assets/imported/verdant_trail/verdant_trail.biomepack.json:1817 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/grass_medium_02_a.glb | 31,508 | 1/2 | astraweave-terrain/src/biome.rs:502; assets/imported/verdant_trail/verdant_trail.biomepack.json:1927 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/grass_medium_02_b.glb | 74,992 | 1/2 | astraweave-terrain/src/biome.rs:515; astraweave-terrain/src/biome.rs:1026 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/grass_medium_02_c.glb | 51,600 | 1/2 | astraweave-terrain/src/biome.rs:528; assets/imported/verdant_trail/verdant_trail.biomepack.json:2037 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/grass_medium_02_d.glb | 104,248 | 1/2 | astraweave-terrain/src/biome.rs:541; astraweave-terrain/src/biome.rs:1039 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/grass_medium_02_e.glb | 73,096 | 1/2 | astraweave-terrain/src/biome.rs:554; assets/imported/verdant_trail/verdant_trail.biomepack.json:2147 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/jacaranda_tree_trunk.glb | 3,119,536 | 1/2 | astraweave-terrain/src/biome.rs:933; assets/imported/verdant_trail/verdant_trail.biomepack.json:3302 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/rock_07.001.glb | 72,044 | 1/2 | astraweave-terrain/src/biome.rs:582; astraweave-terrain/src/biome.rs:1078 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/rock_08.001.glb | 36,944 | 1/2 | astraweave-terrain/src/biome.rs:595; assets/imported/verdant_trail/verdant_trail.biomepack.json:3841 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/shrub_01_a.glb | 82,160 | 1/2 | astraweave-terrain/src/biome.rs:383; assets/imported/verdant_trail/verdant_trail.biomepack.json:4976 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/shrub_01_b.glb | 56,528 | 1/2 | astraweave-terrain/src/biome.rs:396; astraweave-terrain/src/biome.rs:973 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/shrub_01_d.glb | 39,104 | 1/2 | astraweave-terrain/src/biome.rs:409; assets/imported/verdant_trail/verdant_trail.biomepack.json:5123 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/shrub_02_a.glb | 251,984 | 1/2 | astraweave-terrain/src/biome.rs:422; assets/imported/verdant_trail/verdant_trail.biomepack.json:5417 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/shrub_02_c.glb | 251,984 | 1/2 | astraweave-terrain/src/biome.rs:435; astraweave-terrain/src/biome.rs:986 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/shrub_03_a.glb | 83,636 | 1/2 | astraweave-terrain/src/biome.rs:448; assets/imported/verdant_trail/verdant_trail.biomepack.json:6297 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/shrub_03_b.glb | 75,452 | 1/2 | astraweave-terrain/src/biome.rs:461; astraweave-terrain/src/biome.rs:999 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/shrub_04_a.glb | 124,324 | 1/2 | astraweave-terrain/src/biome.rs:474; assets/imported/verdant_trail/verdant_trail.biomepack.json:6469 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/shrub_04_b.glb | 217,788 | 1/2 | astraweave-terrain/src/biome.rs:487; astraweave-terrain/src/biome.rs:1012 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/sticks_debris_a.glb | 26,600 | 1/2 | astraweave-terrain/src/biome.rs:1065; assets/imported/verdant_trail/verdant_trail.biomepack.json:6702 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/stone_01_LOD0.001.glb | 1,215,000 | 1/2 | astraweave-terrain/src/biome.rs:569; assets/imported/verdant_trail/verdant_trail.biomepack.json:6886 | UNKNOWN | as-is |  |
| assets/imported/verdant_trail/meshes/tree_small_02_a.glb | 681,216 | 1/2 | astraweave-terrain/src/biome.rs:329; astraweave-terrain/src/biome.rs:946 | UNKNOWN | as-is |  |
| assets/models/grass.glb | 11,496 | 1/2 | astraweave-terrain/src/biome.rs:212 | UNKNOWN | as-is |  |

### TABLE F-greybox (subtotal 66,122 bytes = 0.06 MB)

| path | size_bytes | tier | why | provenance | derivative? | notes |
|---|---|---|---|---|---|---|
| assets/models/greybox/boss_courtyard_greybox.gltf | 20,912 | 1 | assets/cells/Z4_boss_courtyard.ron:12; assets/navmeshes/boss_courtyard_navmesh.ron:3 | UNKNOWN | as-is |  |
| assets/models/greybox/echo_grove_greybox.gltf | 12,228 | 1 | assets/cells/Z1_echo_grove.ron:13; assets/navmeshes/echo_grove_navmesh.ron:3 | UNKNOWN | as-is |  |
| assets/models/greybox/fractured_cliffs_greybox.gltf | 6,755 | 1 | assets/cells/Z2_fractured_cliffs.ron:11; assets/navmeshes/fractured_cliffs_navmesh.ron:3 | UNKNOWN | as-is |  |
| assets/models/greybox/loom_crossroads_greybox.gltf | 11,985 | 1 | assets/cells/Z3_loom_crossroads.ron:12; assets/navmeshes/loom_crossroads_navmesh.ron:3 | UNKNOWN | as-is |  |
| assets/models/greybox/loomspire_sanctum_greybox.gltf | 3,197 | 1 | assets/cells/Z0_loomspire_sanctum.ron:13; assets/navmeshes/loomspire_sanctum_navmesh.ron:3 | UNKNOWN | as-is |  |
| assets/models/greybox/side_alcove_greybox.gltf | 11,045 | 1 | assets/cells/Z2a_side_alcove.ron:12; assets/navmeshes/side_alcove_navmesh.ron:3 | UNKNOWN | as-is |  |

### TABLE I-closure-textures (subtotal 74,312 bytes = 0.07 MB)

| path | size_bytes | tier | why | provenance | derivative? | notes |
|---|---|---|---|---|---|---|
| assets/3D assets/Castle Kit/Models/GLB format/Textures/colormap.png | 7,529 | 1 | texture dependency of a selected model (glTF images[].uri) | CC0 (Kenney) | as-is |  |
| assets/3D assets/Fantasy Town Kit/Models/GLB format/Textures/colormap.png | 11,143 | 1 | texture dependency of a selected model (glTF images[].uri) | CC0 (Kenney) | as-is |  |
| assets/3D assets/Survival Kit/Models/GLB format/Textures/colormap.png | 7,440 | 1 | texture dependency of a selected model (glTF images[].uri) | CC0 (Kenney) | as-is |  |
| assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color1/forest_texture.png | 24,100 | 1 | texture dependency of a selected model (glTF images[].uri) | CC0 (KayKit / Kay Lousberg) | as-is |  |
| assets/The Complete KayKit Collection v4/KayKit Forest Nature Pack 1.0/Assets/gltf/Color2/forest_texture.png | 24,100 | 1 | texture dependency of a selected model (glTF images[].uri) | CC0 (KayKit / Kay Lousberg) | as-is |  |

### TABLE H-materials-1K (derivative-estimate subtotal 48,447,747 B = 46.20 MB est; source size_bytes column sums to 255,572,304 B = 243.7 MB)

| path | size_bytes | tier | why | provenance | derivative? | notes |
|---|---|---|---|---|---|---|
| assets/materials/cloth.png | 9,507,020 | 2 | assets/materials/desert/materials.toml:38 | UNKNOWN | DERIVATIVE(1K) est 594,188 B | src 4096x4096 |
| assets/materials/cloth_mra.png | 262,606 | 2 | assets/materials/desert/materials.toml:40 | UNKNOWN | DERIVATIVE(1K) est 262,606 B | src 1024x1024 |
| assets/materials/cloth_n.png | 12,393,733 | 2 | assets/materials/desert/materials.toml:39 | UNKNOWN | DERIVATIVE(1K) est 774,608 B | src 4096x4096 |
| assets/materials/cobblestone.png | 213,597 | 2 | biome material source (canonical_terrain_pack.rs:181,215; materials.toml) | UNKNOWN | DERIVATIVE(1K) est 213,597 B | src 1024x1024 |
| assets/materials/cobblestone_mra.png | 255,113 | 2 | biome material source (canonical_terrain_pack.rs:181,215; materials.toml) | UNKNOWN | DERIVATIVE(1K) est 255,113 B | src 1024x1024 |
| assets/materials/cobblestone_n.png | 175,726 | 2 | biome material source (canonical_terrain_pack.rs:181,215; materials.toml) | UNKNOWN | DERIVATIVE(1K) est 175,726 B | src 1024x1024 |
| assets/materials/dirt.png | 6,335,509 | 2 | assets/materials/tundra/materials.toml:41; assets/materials/terrain/materials.toml:17 | UNKNOWN | DERIVATIVE(1K) est 1,583,877 B | src 2048x2048 |
| assets/materials/dirt_mra.png | 2,045,189 | 2 | assets/materials/tundra/materials.toml:43; assets/materials/terrain/materials.toml:19 | UNKNOWN | DERIVATIVE(1K) est 511,297 B | src 2048x2048 |
| assets/materials/dirt_n.png | 9,015,271 | 2 | assets/materials/tundra/materials.toml:42; assets/materials/terrain/materials.toml:18 | UNKNOWN | DERIVATIVE(1K) est 2,253,817 B | src 2048x2048 |
| assets/materials/forest_floor.png | 10,681,898 | 2 | assets/materials/forest/materials.toml:6 | UNKNOWN | DERIVATIVE(1K) est 2,670,474 B | src 2048x2048 |
| assets/materials/forest_floor_mra.png | 3,573,683 | 2 | assets/materials/forest/materials.toml:8 | UNKNOWN | DERIVATIVE(1K) est 893,420 B | src 2048x2048 |
| assets/materials/forest_floor_n.png | 11,628,610 | 2 | assets/materials/forest/materials.toml:7 | UNKNOWN | DERIVATIVE(1K) est 2,907,152 B | src 2048x2048 |
| assets/materials/grass.png | 4,825,369 | 2 | tools/aw_editor/src/viewport/types.rs:188; tools/aw_editor/src/viewport/types.rs:199 | UNKNOWN | DERIVATIVE(1K) est 1,206,342 B | src 2048x2048 |
| assets/materials/grass_mra.png | 38,565 | 2 | assets/materials/terrain/materials.toml:11; assets/materials/river/materials.toml:43 | UNKNOWN | DERIVATIVE(1K) est 9,641 B | src 2048x2048 |
| assets/materials/grass_n.png | 7,352,393 | 2 | assets/materials/terrain/materials.toml:10; assets/materials/river/materials.toml:42 | UNKNOWN | DERIVATIVE(1K) est 1,838,098 B | src 2048x2048 |
| assets/materials/gravel.png | 196,613 | 2 | assets/materials/tundra/materials.toml:25; assets/materials/river/materials.toml:17 | UNKNOWN | DERIVATIVE(1K) est 196,613 B | src 1024x1024 |
| assets/materials/gravel_mra.png | 256,319 | 2 | assets/materials/tundra/materials.toml:27; assets/materials/river/materials.toml:19 | UNKNOWN | DERIVATIVE(1K) est 256,319 B | src 1024x1024 |
| assets/materials/gravel_n.png | 174,736 | 2 | assets/materials/tundra/materials.toml:26; assets/materials/river/materials.toml:18 | UNKNOWN | DERIVATIVE(1K) est 174,736 B | src 1024x1024 |
| assets/materials/ice.png | 141,369 | 2 | assets/materials/tundra/materials.toml:17 | UNKNOWN | DERIVATIVE(1K) est 141,369 B | src 1024x1024 |
| assets/materials/ice_mra.png | 119,417 | 2 | assets/materials/tundra/materials.toml:19 | UNKNOWN | DERIVATIVE(1K) est 119,417 B | src 1024x1024 |
| assets/materials/ice_n.png | 173,472 | 2 | assets/materials/tundra/materials.toml:18 | UNKNOWN | DERIVATIVE(1K) est 173,472 B | src 1024x1024 |
| assets/materials/metal_rusted.png | 225,054 | 2 | biome material source (canonical_terrain_pack.rs:181,215; materials.toml) | UNKNOWN | DERIVATIVE(1K) est 225,054 B | src 1024x1024 |
| assets/materials/metal_rusted_mra.png | 307,494 | 2 | biome material source (canonical_terrain_pack.rs:181,215; materials.toml) | UNKNOWN | DERIVATIVE(1K) est 307,494 B | src 1024x1024 |
| assets/materials/metal_rusted_n.png | 175,670 | 2 | biome material source (canonical_terrain_pack.rs:181,215; materials.toml) | UNKNOWN | DERIVATIVE(1K) est 175,670 B | src 1024x1024 |
| assets/materials/moss.png | 177,433 | 2 | assets/materials/swamp/materials.toml:17; assets/materials/river/materials.toml:33 | UNKNOWN | DERIVATIVE(1K) est 177,433 B | src 1024x1024 |
| assets/materials/moss_mra.png | 190,049 | 2 | assets/materials/swamp/materials.toml:19; assets/materials/river/materials.toml:35 | UNKNOWN | DERIVATIVE(1K) est 190,049 B | src 1024x1024 |
| assets/materials/moss_n.png | 177,438 | 2 | assets/materials/swamp/materials.toml:18; assets/materials/river/materials.toml:34 | UNKNOWN | DERIVATIVE(1K) est 177,438 B | src 1024x1024 |
| assets/materials/mountain_rock.png | 6,691,325 | 2 | assets/materials/tundra/materials.toml:33; assets/materials/mountain/materials.toml:9 | UNKNOWN | DERIVATIVE(1K) est 1,672,831 B | src 2048x2048 |
| assets/materials/mountain_rock_mra.png | 2,645,625 | 2 | assets/materials/tundra/materials.toml:35; assets/materials/mountain/materials.toml:11 | UNKNOWN | DERIVATIVE(1K) est 661,406 B | src 2048x2048 |
| assets/materials/mountain_rock_n.png | 8,737,225 | 2 | assets/materials/tundra/materials.toml:34; assets/materials/mountain/materials.toml:10 | UNKNOWN | DERIVATIVE(1K) est 2,184,306 B | src 2048x2048 |
| assets/materials/mud.png | 8,454,500 | 2 | assets/materials/swamp/materials.toml:9; assets/materials/river/materials.toml:9 | UNKNOWN | DERIVATIVE(1K) est 2,113,625 B | src 2048x2048 |
| assets/materials/mud_mra.png | 2,636,255 | 2 | assets/materials/swamp/materials.toml:11; assets/materials/river/materials.toml:11 | UNKNOWN | DERIVATIVE(1K) est 659,063 B | src 2048x2048 |
| assets/materials/mud_n.png | 9,830,700 | 2 | assets/materials/swamp/materials.toml:10; assets/materials/river/materials.toml:10 | UNKNOWN | DERIVATIVE(1K) est 2,457,675 B | src 2048x2048 |
| assets/materials/plaster.png | 2,586,757 | 2 | assets/materials/desert/materials.toml:30 | UNKNOWN | DERIVATIVE(1K) est 161,672 B | src 4096x4096 |
| assets/materials/plaster_mra.png | 287,879 | 2 | assets/materials/<biome>/materials.toml layer (canonical_terrain_pack.rs:181,215) | UNKNOWN | DERIVATIVE(1K) est 287,879 B | src 1024x1024 |
| assets/materials/plaster_n.png | 4,028,026 | 2 | assets/materials/<biome>/materials.toml layer (canonical_terrain_pack.rs:181,215) | UNKNOWN | DERIVATIVE(1K) est 251,751 B | src 4096x4096 |
| assets/materials/rock_lichen.png | 11,005,733 | 2 | assets/materials/swamp/materials.toml:41; assets/materials/grassland/materials.toml:38 | UNKNOWN | DERIVATIVE(1K) est 687,858 B | src 4096x4096 |
| assets/materials/rock_lichen_mra.png | 374,698 | 2 | assets/materials/swamp/materials.toml:43; assets/materials/grassland/materials.toml:40 | UNKNOWN | DERIVATIVE(1K) est 374,698 B | src 1024x1024 |
| assets/materials/rock_lichen_n.png | 16,405,251 | 2 | assets/materials/swamp/materials.toml:42; assets/materials/grassland/materials.toml:39 | UNKNOWN | DERIVATIVE(1K) est 1,025,328 B | src 4096x4096 |
| assets/materials/rock_slate.png | 5,728,700 | 2 | assets/materials/terrain/materials.toml:41; assets/materials/grassland/materials.toml:14 | UNKNOWN | DERIVATIVE(1K) est 1,432,175 B | src 2048x2048 |
| assets/materials/rock_slate_mra.png | 1,512,972 | 2 | assets/materials/terrain/materials.toml:43; assets/materials/grassland/materials.toml:16 | UNKNOWN | DERIVATIVE(1K) est 378,243 B | src 2048x2048 |
| assets/materials/rock_slate_n.png | 8,221,641 | 2 | assets/materials/terrain/materials.toml:42; assets/materials/grassland/materials.toml:15 | UNKNOWN | DERIVATIVE(1K) est 2,055,410 B | src 2048x2048 |
| assets/materials/roof_tile.png | 6,933,147 | 2 | biome material source (canonical_terrain_pack.rs:181,215; materials.toml) | UNKNOWN | DERIVATIVE(1K) est 433,321 B | src 4096x4096 |
| assets/materials/roof_tile_mra.png | 334,540 | 2 | assets/materials/<biome>/materials.toml layer (canonical_terrain_pack.rs:181,215) | UNKNOWN | DERIVATIVE(1K) est 334,540 B | src 1024x1024 |
| assets/materials/roof_tile_n.png | 9,786,987 | 2 | assets/materials/<biome>/materials.toml layer (canonical_terrain_pack.rs:181,215) | UNKNOWN | DERIVATIVE(1K) est 611,686 B | src 4096x4096 |
| assets/materials/sand.png | 3,898,226 | 2 | assets/materials/terrain/materials.toml:33; assets/materials/river/materials.toml:25 | UNKNOWN | DERIVATIVE(1K) est 974,556 B | src 2048x2048 |
| assets/materials/sand_mra.png | 1,729,290 | 2 | assets/materials/terrain/materials.toml:35; assets/materials/river/materials.toml:27 | UNKNOWN | DERIVATIVE(1K) est 432,322 B | src 2048x2048 |
| assets/materials/sand_n.png | 1,773,649 | 2 | assets/materials/terrain/materials.toml:34; assets/materials/river/materials.toml:26 | UNKNOWN | DERIVATIVE(1K) est 443,412 B | src 2048x2048 |
| assets/materials/snow.png | 2,869,720 | 2 | assets/materials/tundra/materials.toml:9; assets/materials/mountain/materials.toml:17 | UNKNOWN | DERIVATIVE(1K) est 717,430 B | src 2048x2048 |
| assets/materials/snow_mra.png | 1,762,830 | 2 | assets/materials/tundra/materials.toml:11; assets/materials/mountain/materials.toml:19 | UNKNOWN | DERIVATIVE(1K) est 440,707 B | src 2048x2048 |
| assets/materials/snow_n.png | 1,613,255 | 2 | assets/materials/tundra/materials.toml:10; assets/materials/mountain/materials.toml:18 | UNKNOWN | DERIVATIVE(1K) est 403,313 B | src 2048x2048 |
| assets/materials/stone.png | 9,790,293 | 2 | assets/materials/terrain/materials.toml:25; assets/materials/mountain/materials.toml:33 | UNKNOWN | DERIVATIVE(1K) est 2,447,573 B | src 2048x2048 |
| assets/materials/stone_mra.png | 21,240 | 2 | assets/materials/terrain/materials.toml:27; assets/materials/mountain/materials.toml:35 | UNKNOWN | DERIVATIVE(1K) est 5,310 B | src 2048x2048 |
| assets/materials/stone_n.png | 11,943,403 | 2 | assets/materials/terrain/materials.toml:26; assets/materials/mountain/materials.toml:34 | UNKNOWN | DERIVATIVE(1K) est 2,985,850 B | src 2048x2048 |
| assets/materials/tree_bark.png | 2,191,920 | 2 | assets/materials/swamp/materials.toml:33; assets/materials/forest/materials.toml:14 | UNKNOWN | DERIVATIVE(1K) est 136,995 B | src 4096x4096 |
| assets/materials/tree_bark_mra.png | 410,597 | 2 | assets/materials/<biome>/materials.toml layer (canonical_terrain_pack.rs:181,215) | UNKNOWN | DERIVATIVE(1K) est 410,597 B | src 1024x1024 |
| assets/materials/tree_bark_n.png | 4,803,725 | 2 | assets/materials/<biome>/materials.toml layer (canonical_terrain_pack.rs:181,215) | UNKNOWN | DERIVATIVE(1K) est 300,232 B | src 4096x4096 |
| assets/materials/tree_leaves.png | 10,768,447 | 2 | assets/materials/forest/materials.toml:22 | UNKNOWN | DERIVATIVE(1K) est 673,027 B | src 4096x4096 |
| assets/materials/tree_leaves_mra.png | 336,292 | 2 | assets/materials/<biome>/materials.toml layer (canonical_terrain_pack.rs:181,215) | UNKNOWN | DERIVATIVE(1K) est 336,292 B | src 1024x1024 |
| assets/materials/tree_leaves_n.png | 14,236,230 | 2 | assets/materials/<biome>/materials.toml layer (canonical_terrain_pack.rs:181,215) | UNKNOWN | DERIVATIVE(1K) est 889,764 B | src 4096x4096 |
| assets/materials/wood_planks.png | 202,688 | 2 | biome material source (canonical_terrain_pack.rs:181,215; materials.toml) | UNKNOWN | DERIVATIVE(1K) est 202,688 B | src 1024x1024 |
| assets/materials/wood_planks_mra.png | 220,608 | 2 | biome material source (canonical_terrain_pack.rs:181,215; materials.toml) | UNKNOWN | DERIVATIVE(1K) est 220,608 B | src 1024x1024 |
| assets/materials/wood_planks_n.png | 178,584 | 2 | biome material source (canonical_terrain_pack.rs:181,215; materials.toml) | UNKNOWN | DERIVATIVE(1K) est 178,584 B | src 1024x1024 |

## 4. Tier 2 — feature-coverage rationale

- **Scatter/vegetation (dead tree)**: `assets/imported/verdant_trail/meshes/dead_tree_trunk.001.glb` (biome.rs:342) and `dead_tree_trunk_02.glb` (biome.rs:355,959) — already in table E. The scatter loader skips missing files gracefully (engine_adapter.rs:2578 warn+continue, catch_unwind at 2598), so the 29 biome.rs-referenced files **>3 MB (744.2 MB total — e.g. `island_tree_01_a/b.glb` 81.7 MB each, the Namaqualand LOD sets)** go to release packs; scatter density thins until `fetch-assets` restores them. Table E keeps all 52 biome.rs-referenced files <=3 MB so every biome renders some vegetation out of the box.
- **Terrain/biome materials (table H)**: the editor's E3 terrain texturing reads `assets/materials/<biome>/materials.toml` + `arrays.toml` and image::open's the referenced root PNGs (canonical_terrain_pack.rs:5,181,215; grassland default with 5 layers at :293). Keeping **all 63 root material PNGs as 1K derivatives (~46.2 MB est)** keeps all 11 biome manifests coherent. Full-res sources (243.7 MB) ship in packs. The `materials.toml`/`arrays.toml` manifests are small text files outside the registry's scope and stay in-repo regardless. Serves: editor terrain (all biome presets) + Renderer BiomeMaterialSystem switching.
- **Editor spawn path (table A)**: union of BOTH live archetype-to-mesh maps (main.rs:156-188 and entity_panel.rs:319-328; they diverge — red flag R5). All 11 GLBs parse clean per registry; textures embedded (KayKit) or tiny external colormaps (table I). None of the audit's 123 broken-texture GLBs selected except `barrels.glb` (unavoidable: live Debug-menu item; red flag R3).
- **HDRI/IBL (table D)**: `kloppenheim_02` (hello_companion visual_demo.rs:685) and `kloppenheim_06` (unified_showcase main.rs:764; veilweaver visual_renderer.rs:573) are live example loads. `spruit_sunrise` + `venice_sunset` are the catalog's morning/evening fallbacks for every biome (hdri_catalog.toml:117-134) and the only other PolyHaven-README-documented CC0 HDRIs. The remaining 6 catalog HDRIs (33.8 MB, all PROVENANCE UNKNOWN) go to packs — red flag R6.
- **Skinned/animated: NOTHING INCLUDED.** Verified this session: no compiled example, tool, or engine path plays a skeletal animation from a repo asset. Evidence chain: visual_3d's `assets/skinned_demo.gltf` does not exist (visual_3d/src/main.rs:363, graceful skip at :426); Renderer's `skinned_mesh` field is written once and read nowhere, `skinned_pipeline` created but never bound (renderer.rs:6863, 2826); the editor animation bridge's data source is hardcoded empty (viewport/renderer.rs:1014-1027: get_mesh_skeleton -> None, get_mesh_animations -> empty, apply_cpu_skinning no-op) and engine_adapter.rs has zero skinning code; skinning_demo is synthetic + console-only; scene-crate CSkeleton/CAnimator/CSkinnedMesh/CJointMatrices are never inserted into any World. When a live playback path exists, add an asset then.
- **Foliage MASK verification**: Tier 3 below (D5) — the audit established zero MASK materials exist in the repo, so this must be acquired.

## 5. Tier 3 — MASK-foliage acquisition candidates (director downloads; agent parse-verifies before entry)

| # | Candidate | Source | License | Format | Size | MASK status |
|---|---|---|---|---|---|---|
| 1 (recommended) | **Quaternius Stylized Nature MegaKit — `CommonTree_1`** (alt: TwistedTree_1) | https://quaternius.com/packs/stylizednaturemegakit.html (CC0 zip mirror: https://opengameart.org/sites/default/files/stylized_nature_megakitstandard.zip, 104,088,529 B whole pack) | CC0 1.0 (https://quaternius.com/faq.html) | glTF + .bin + PNG | ~10.3 MB per tree | **documented (independently verified this session)**: pack downloaded to session scratchpad and parsed — materials `Bark_NormalTree` and `Leaves_NormalTree` both `alphaMode:"MASK"`, `alphaCutoff:0.2`, `doubleSided:true`; leaf PNG is real RGBA 1024^2 with alpha histogram 75.0% transparent / 14.0% opaque / 11.1% edge — genuine cutout data. `TwistedTree_1` = mixed OPAQUE trunk + MASK canopy; `DeadTree_1` = opaque-only control. |
| 2 | Poly Haven `shrub_01` | https://polyhaven.com/a/shrub_01 | CC0 (https://polyhaven.com/license) | glTF 1k bundle | ~6.9 MB | needs-parse-verification-plus-editing: shipped glTF (parsed this session) declares MASK+doubleSided but baseColor is JPEG (no alpha channel) — exercises flag parsing only; no visual cutout without manually wiring Poly Haven's separate alpha map. |
| 3 | Poly Haven `fir_sapling` | https://polyhaven.com/a/fir_sapling | CC0 (https://polyhaven.com/license) | glTF 1k bundle | ~24.2 MB | needs-parse-verification-plus-editing: shipped glTF is OPAQUE-by-default (parsed this session); `twigs_alpha`/`twigs_mask` maps exist on the site but are not bundled. An actual tree, but not plug-and-play. |

Candidate 1 is the only one that verifies the masked/two-sided path without hand-editing. Budget carries it at ~10.3 MB (estimate until the exact file set is chosen and parse-verified after the director's download).

## 6. Red flags (surfaced, not resolved)

- **R1 — Tier-1/2 files with PROVENANCE UNKNOWN** (collision course: live code depends on them, but they cannot stay untraced in a public MIT repo). AD.1 priority queue: all 63 `assets/materials/*.png` (table H, incl. the editor sentinel `grass.png`); the 24 `assets/imported/verdant_trail/meshes/*.glb` entries in table E; `assets/models/{barrels,bed,tree_pine*,tent_*,campfire_logs,tree_default}.glb` (B/C); `assets/textures/cobblestone.png`; `assets/textures/grass_bermuda_01_diff_1k.jpg`; `assets/hdri/polyhaven/kloppenheim_02_puresky_2k.hdr` (live in hello_companion but no per-file README, unlike its kloppenheim_06 sibling); `assets/models/greybox/*` (F); `assets/Astraweave_logo.jpg` (presumed project-own art — confirm and record). Registry naming hints (PolyHaven conventions) are hints, not evidence.
- **R2 — live-referenced file EXCLUDED by the 50 MB per-file cap**: `assets/models/pine_tree_01_1k.glb` (**1,119,297,560 B ~= 1,067 MiB** — re-measured via os.stat after a director sniff-test; the registry agrees; an earlier draft of this section carried a transcription typo 1,119,301,924). Despite the `_1k` texture-suffix name, the payload really is ~1.04 GiB (contents do not match the naming convention — likely re-exported with dense geometry; parsing showed it GOOD/62 MTri-class). Editor Debug menu 'Load Pine Tree': menu_bar.rs:402-405 -> main.rs:9357-9373; PROVENANCE UNKNOWN. It reached GitHub as a 135-byte **Git LFS pointer** (verified in `origin/main`; LFS payload upload verified complete — see section 10), so GitHub's 100 MB blob limit never applied. The Debug handler picks the first existing candidate path and surfaces errors non-fatally (main.rs:9379-9381), so exclusion degrades gracefully. Goes to a release pack (or the Debug button gets retargeted later — director's call).
- **R3 — live ref to a defective asset**: `assets/models/barrels.glb` (Debug menu, menu_bar.rs:391) is one of the audit's 123 broken-texture GLBs (`Textures/barrel.png`/`planks.png` missing repo-wide) — kept because live; renders untextured; registry verdict FIX stands. (`character-a.glb`, also FIX, drops — its refs are all dead surfaces.)
- **R4 — live loads of files that DON'T EXIST** (code-side defects, out of AD.0 scope, recorded): unified_showcase main.rs:944-968 loads `assets/textures/pine forest textures/*.png` — that directory contains zero files; main.rs:806 expects `sky_equirect.png` (also a dangling `hdri_catalog.toml:95` entry).
- **R5 — divergent duplicate archetype-to-mesh maps (CLAUDE.md 7.7)**: main.rs:156-188 (whose own comment at :154-155 says 'do not duplicate this match') vs entity_panel.rs:319-328 — mappings disagree (Companion -> Barbarian vs Knight; Boss -> Knight vs Skeleton_Golem). The sample set carries the union so both behave; the duplication itself is a code defect for a later beat.
- **R6 — HDRI catalog partial coverage after packing**: 6 of 10 catalog `.hdr` files (goegap 5.19, table_mountain_2 6.09, rainforest_trail 7.32, misty_pines 6.72, qwantani_moonrise 4.69, rogland 6.52 MB — all PROVENANCE UNKNOWN) move to packs; `hdri_catalog.toml` day/night entries for forest/desert/mountain/swamp/tundra will point at absent files until fetch. These also need AD.1 provenance tracing BEFORE they can be uploaded to a public release artifact at all.
- **R7 — parse caveats / DELETE-REPLACE closure**: every proposed model parses clean per the audit; the 3 REPLACE-verdict empty OBJs and all FBX/DAE/blend/USDC are excluded by construction. Checked explicitly: no live code references any registry DELETE- or REPLACE-verdict file, with two disclosed exceptions — `barrels.glb` (FIX, R3) and the dropped test-literal `grass.ktx2` (FIX, section 2).

## 7. Ranked cuts (currently under target; apply only if the director wants more headroom)

1. **Table H -> forest+grassland union only** (24 files, ~23.3 MB est instead of 46.2): -22.9 MB -> total ~71 MB. Cost: 9 of 11 biome manifests reference absent textures until fetch (editor terrain defaults to grassland — canonical_terrain_pack.rs:293 — so the default path stays intact).
2. **Drop spruit_sunrise + venice_sunset** (catalog fallbacks only, no direct example load): -11.1 MB. Cost: morning/evening catalog entries dangle for all biomes.
3. **Defer the Tier 3 acquisition**: -10.3 MB. Cost: D5 unfulfilled; the masked render path remains unexercisable.

## 8. Discrepancies noted against the registry (not corrected; the audit is not re-litigated)

- Registry `referenced=REFERENCED` rows whose only citations are now-verified DEAD/TEST-LITERAL surfaces: `spruit_sunrise_2k.hdr`/`venice_sunset_2k.hdr` (main_bevy_v2/scene.rs), `character-a/b/c.glb`, `Amber.Fbx`/`Amber_Motion.Fbx` (scene.rs dead_code), `Namaqualand.blend` (mock-manifest tests), `assets/materials/baked/grass.ktx2` + `assets_src/textures/grass.png` (serde string round-trip; literals actually `textures/grass.png`/`baked/grass.ktx2`), `rainforest_trail_2k.hdr` (cfg(test) fixture), `rock_large*/stone_large*/tree_oak/tree_simple/tree_detailed/plant_bush.glb` + `planks/roof/cobblestonePainted.png` + materials `*_n/_mra` refs (main_bevy_v2.rs). All CONSISTENT with the registry's disclosed static-match method (audit section 5 caveat) — this proposal applies the finer liveness tier the registry deliberately did not.
- `assets/textures/pine forest textures/` is loaded by compiled code (unified_showcase main.rs:944-968) yet contains zero files — the audit's broken-ref list covers individual paths; the whole-directory absence is flagged here because it affects the showcase.
- `assets/tests/` fixtures (18 files, 346,863 B) were candidates and were REJECTED after a critic pass: repo-wide grep found zero live code or tests opening them (only docs/registry mentions and dead orphan-source refs to a different directory). They follow the default pack/quarantine route with the other orphans.
- The E3 editor terrain path (`canonical_terrain_pack.rs`) is uncommitted work-in-progress on branch `campaign/roadmap`; its citations were verified against the current working tree.

## 9. Out of scope for AD.0 (later beats)

Pack manifest composition for everything not listed (AD.2/AD.3); provenance deep-tracing (AD.1 — the R1 list is its priority queue); derivative generation + verification (AD.4); the quarantine list for untraceable files (D4; candidates visible in R1/R6). Nothing in the repo changed this session.

## 10. Addendum (2026-07-05) — Git LFS reality + tracked/untracked cross-check

Triggered by a director sniff-test on R2 ("1,067 MB can't coexist with GitHub's 100 MB push block"). Both horns of that dilemma dissolve on a third fact the audit never surfaced: **the asset library rides Git LFS.**

**Measured this session:**

- `.gitattributes` routes `*.png`, `*.jpg`, `*.glb`, `*.gltf`-adjacent binaries, `*.blend`, `*.obj` (and more) through LFS; `git lfs ls-files` counts **91,290 files** — effectively the entire audited library.
- `pine_tree_01_1k.glb`: disk size 1,119,297,560 B (matches the registry; the R2 narrative had a 4,364-byte transcription typo, now fixed). In git trees it is a **135-byte LFS pointer**, present in `origin/main` — it has genuinely been pushed. GitHub's 100 MB hard block applies to git blobs, not LFS payloads.
- **Remote LFS completeness**: `git lfs push --dry-run origin campaign/roadmap` returns zero objects to upload — GitHub's LFS storage holds every payload this branch references. Release packs (AD.3) can safely be built from this machine's fully-smudged checkout.
- **Tracked/untracked cross-check over all 91,034 registry rows** (case-normalized against `git ls-files`): **198 rows are local-only**, all inside deliberately gitignored dirs — `assets/Forest Scene/**/Library/` 137 (`.gitignore:235`, Unity convention), `assets/_downloaded/` 58 (`.gitignore:48`, fetch-tool output), `docs/book/` 3 (`.gitignore:13`, mdbook output). **Zero proposed sample-set rows are untracked.** A fresh clone therefore lacks those 198 files (including the PolyHaven `ATTRIBUTION.txt` that is the provenance evidence for `assets/_downloaded/polyhaven/*` — the *evidence file itself* is not in the public repo; AD.1 should note that).
- **Case-only index/worktree mismatch (cross-platform hazard, not untracked)**: 24 files tracked as `assets/textures/Fabrics/*` while the working tree spells `fabrics/` — one directory on Windows, two different outcomes on a case-sensitive clone. Flagged for a later hygiene fix; not an AD.0 action.

**Registry discrepancy class (director's point, confirmed in miniature):** the audit's *exists* tier measured the local working tree and did not distinguish tracked / untracked / LFS-pointer states. At 198/91,034 rows (0.22%), all by-design ignores, no verdicts change — but the class is real and is now recorded.

**AD-series premises this reframes (flagged for director decision, not redesigned here):**

1. **Clone-size math (success criterion 4, AD.6 rationale).** The git object store is mostly 135-byte pointers — a clone *without* git-lfs is already small but asset-less; a clone *with* git-lfs smudges ~22 GB through GitHub's **LFS bandwidth quota** (a billing/limits question, not a git-history question). "Every future clone at ~24 GB" is true only for LFS-enabled clones, and the cost shows up as LFS bandwidth, not repo size.
2. **AD.6 history purge scope.** `git filter-repo` over pack paths removes tiny pointer blobs — it does **not** reclaim GitHub-side LFS storage. Actual reclamation of the ~22 GB is an LFS-storage operation (GitHub support / repo re-creation / `git lfs migrate` away). AD.6's method, backup plan (`git clone --mirror` does not fetch LFS payloads by default — the mirror-backup step needs `git lfs fetch --all`), and its clone-size success measurement all need an LFS-aware rewrite before execution.
3. **AD.2 fetch tool**: the packs-on-releases model effectively *replaces* LFS distribution; a beat should decide LFS's end state (keep for sample set? untrack entirely?) — otherwise the sample set's 154 files remain LFS-routed by the blanket `.gitattributes` rules and still cost LFS bandwidth per clone.
