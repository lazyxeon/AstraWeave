# Asset Ground-Truth Inventory — Audit Report

**Date**: 2026-07-04 | **Session type**: read-only static audit | **Registry**: [`ASSET_REGISTRY.csv`](ASSET_REGISTRY.csv) (91,034 rows, one per asset)

> **Evidence-tier limitation (read first).** This is a static audit. It establishes *exists*, *parses* (with a stated tool), and *referenced* (with a stated search method). It does **not** establish *loads in engine* or *renders correctly* — no engine binary was run against any asset. No verdict below is a claim about runtime behavior. Every number in this report was produced by tool output this session; where something could not be measured it says `UNKNOWN`. No asset file was modified, moved, renamed, or deleted.

**Measurement tools used** (all runs this session):
- Enumeration: Python 3.14.2 `os.walk` over the full repo tree (script: `enumerate_assets.py`).
- glTF/GLB: dependency-free Python parser — GLB 12-byte header + chunk-structure validation, JSON-chunk extraction; triangle counts from accessor counts per primitive mode (script: `gltf_audit.py`). `pygltflib`/`trimesh` were not available; the `gltf` Rust crate was not used to avoid building tooling into the repo.
- Textures: PIL 12.3.0 **full decode** (`Image.load()`) for PNG/JPG/TGA/BMP; **header-only** manual parsers for KTX2 (magic + 9-field header), Radiance HDR (magic + resolution line), EXR (magic + `dataWindow`/`chlist` attributes). "Decodes cleanly" for KTX2/HDR/EXR therefore means *header-valid only*.
- OBJ: full text parse (v/vt/vn/f counts, fan-triangulation tri count, `mtllib` resolution, `.mtl` `map_*` reference resolution). DAE: `xml.etree` well-formedness + declared `<triangles>/<polylist>` counts + `<init_from>` resolution. FBX/BLEND/USDC: **header sniff only — parse status `UNKNOWN`** (no FBX/Blender/USD parser available this session).
- References: ripgrep 14.1.1 line extraction over code/config/docs + manifests inside asset dirs, Python path resolution (exact / relative-to-source / `assets/`-prefixed / unambiguous path-suffix). Basename-only matches were deliberately **not** counted as references (too weak).
- Code evidence (loader support, ingestion mechanisms, provenance): 4 subagents (read-only), findings cited as file:line below; every citation was read this session.
- **Verification**: a 3-agent adversarial pass re-checked this report before publication — 25 stratified registry rows re-measured end-to-end (independent GLB/OBJ/KTX2/PIL re-parses, twin-existence checks, citation reads), all 13 headline claims re-established with independent methods (from-scratch material scanner tallied every material: OPAQUE 12,538 / BLEND 178 / MASK 0), and 7 CSV integrity checks (row count, path existence for all 91,034 rows, size spot-checks, domain constraints) — zero refutations; one arithmetic typo in a §1 breakdown was found and corrected.

---

## 1. Executive summary

| Metric | Value |
|---|---|
| Asset files audited | **91,034** (models 34,108; textures 56,926) |
| Total size | ~24.0 GB |
| Excluded from audit | 20 files under `target*/` (build output — 19 `flag_check.obj` compiler probes, 1 rustdoc favicon); `.git/` skipped entirely |
| Verdicts | **GOOD 66,605 · FIX 1,023 · DELETE 23,399 · REPLACE 3 · UNKNOWN 4** |
| Reference status | REFERENCED 938 · INDIRECT 32,878 · ORPHANED 57,218 (182 of these have docs-only mentions) |
| Broken references (real, after noise filtering) | 1,386 raw / headline findings in §6 |
| Provenance | CC0 evidence 88,257 files · Unity-pkg license 137 · unclear 5 · **PROVENANCE UNKNOWN 2,635** |

### Special interest: MASK / doubleSided (renderer question)

- **`alphaMode: MASK` — zero.** Not one of the 9,905 parsed glTF/GLB files contains a MASK material. The masked/alpha-cutoff render path has **never been exercisable from repo assets**.
- **`doubleSided: true` — 3,443 files** carry at least one double-sided material (Kenney `3D assets` 3,212; `assets/imported` 137; KayKit 63; `assets/textures` glTF material sets 13; `assets/models` 9; others 9 — the eight root-level `assets/*_4k.gltf` PolyHaven-named sets + `castles_forts_asset_pack`). So two-sided data exists in bulk, but almost all of it sits in packs reachable only via the editor catalog (§5); of the 3,443, the directly-REFERENCED subset is small — filter the CSV on `double_sided_any=yes` + `referenced=REFERENCED`.
- `alphaMode: BLEND` — 162 files (Kenney window/door/leaf pieces, imported vegetation).

---

## 2. Engine format support (ground truth for verdicts)

Established by reading loader code, not docs. "Runtime" = engine crates compiled with default features; "editor" = `tools/aw_editor`; "tool" = offline CLI.

### 2.1 Model formats

| Format | Supported | Where | Evidence |
|---|---|---|---|
| `.glb` | **yes** | runtime + editor + tools | `gltf` is a **default** feature of astraweave-asset (`astraweave-asset/Cargo.toml:43`); GLB parse `astraweave-asset/src/lib.rs:419,423,485`; streaming-path validation `astraweave-asset/src/cell_loader.rs:273`; editor `astraweave-render/src/mesh_gltf.rs:21` via `tools/aw_editor/src/viewport/engine_adapter.rs:1094` |
| `.gltf` | **yes (two-tier)** | runtime (data-URI only) / editor (full) | runtime JSON path handles **embedded `data:` URIs only** — external `.bin`/`.png` not resolved (`astraweave-asset/src/lib.rs:572-573`); full external-ref import is feature-gated `gltf-assets`, enabled by editor (`astraweave-render/src/mesh_gltf.rs:21`) |
| `.obj` | **dormant** | none as shipped | real tobj loader exists (`astraweave-render/src/mesh_obj.rs:11-12`) but gated on `obj-assets`, which **no production crate or live example enables** (`astraweave-render/Cargo.toml:28`; feature-off stub returns Err at `mesh_obj.rs:63`); only callers are non-compiled backup files in unified_showcase |
| `.fbx` | **no** | none | no FBX parser dep in any Cargo.toml; editor lists `.fbx` in its catalog (`tools/aw_editor/src/panels/entity_catalog.rs:90`) but the spawn loader is glTF-only (`engine_adapter.rs:1094`) |
| `.dae` | **no** | none | no COLLADA dep or code path anywhere (`astraweave-render/Cargo.toml:58-59` — mesh deps are gltf + tobj only) |
| `.blend` | **editor-only, external** | editor import panel | conversion shells out to an installed Blender executable (`crates/astraweave-blend/src/conversion.rs:302`); wired via editor `blend` feature (`tools/aw_editor/Cargo.toml:82`, `main.rs:7156`); output is GLB consumed by the normal glTF path |
| `.usd/.usdc` | **no** | none | no USD dep or code path in any crate |

### 2.2 Texture formats

Pinned `image` crate is 0.25.8 with default features in render/editor/asset-cli (default-formats: bmp, dds, exr, hdr, jpeg, png, tga, tiff, webp, …). astraweave-asset standalone compiles only `png`,`jpeg` (`astraweave-asset/Cargo.toml:12`).

| Format | Supported | Notes / evidence |
|---|---|---|
| `.png` `.jpg/.jpeg` | **yes, runtime decode** | `astraweave-render/src/texture.rs:212` (`image::load_from_memory`); explicit features `astraweave-asset/Cargo.toml:12` |
| `.tga` `.bmp` | **yes** (via image default-formats) | any `image::open` site; recognized by `tools/aw_asset_cli/src/main.rs:538` |
| `.ktx2` | **yes, dedicated runtime path** | `astraweave-render/src/material_loader.rs:286` routes `.ktx2` to `load_ktx2_to_rgba`; `ktx2` + `basis-universal` + `texture2ddecoder` deps (`astraweave-render/Cargo.toml:62-64`); supercompressed → Basis transcode, raw BC1/3/5/7 → texture2ddecoder |
| `.hdr` | **yes, runtime (IBL)** | `astraweave-render/src/ibl.rs:1292` `load_hdr_equirectangular` |
| `.exr` | **partial/incidental** | exr decoder compiled via default-formats; IBL generic loader accepts its output (`ibl.rs:1338`); deliberately enabled only in astraweave-blend (`crates/astraweave-blend/Cargo.toml:36`) |
| `.dds` | **partial/incidental** | image-crate dds decoder compiled (BC1/2/3, not BC7); no dedicated path; not accepted by asset-cli input allow-list (`tools/aw_asset_cli/src/main.rs:538`). Zero `.dds` files exist in the repo |
| `.basis` | **no** (standalone) | basis payloads decoded only inside KTX2 containers; no `.basis` loader; zero `.basis` files exist |

---

## 3. Enumeration totals (Phase 1)

Per-extension counts and byte totals (raw per-file list = the CSV registry, whose first four columns are path/type/format/size — see Appendix A note):

| Ext | Files | Size (MB) | | Ext | Files | Size (MB) |
|---|---|---|---|---|---|---|
| png | 56,416 | 12,128.1 | | jpg | 210 | 843.4 |
| fbx | 13,401 | 815.3 | | blend | 166 | 762.1 |
| obj | 9,370 | 265.7 | | exr | 107 | 1,311.9 |
| glb | 5,589 | 6,060.2 | | ktx2 | 89 | 1,280.0 |
| gltf | 4,317 | 13.6 | | tga | 87 | 231.9 |
| dae | 1,264 | 34.1 | | hdr | 15 | 354.5 |
| bmp | 2 | 7.0 | | usdc | 1 | 3.4 |

Top-level distribution: `assets/` 90,908 · `assets_src/` 66 · `examples/` 14 · `baked_materials/` 13 · `docs/` 13 · others 20 (crate test fixtures, gh-pages, repo-root screenshots).

Major sub-roots of `assets/`: `2D assets` 37,059 files (Kenney sprite packs) · `3D assets` 20,698 (Kenney 3D packs) · `The Complete KayKit Collection v4` 17,459 · `Icons` 7,145 · `UI assets` 4,746 · `Archive` 1,064 · `models` 725 · `textures` 576 (7.2 GB) · `imported` 567 (9.0 GB) · `Road to Vostok Assets Vol.1` 186 · `Forest Scene` 287 · `materials` 162.

---

## 4. Format validation results (Phase 2)

**glTF/GLB (9,906 files):** 9,905 parse cleanly. The single failure is `astraweave-asset/tests/fixtures/corrupted.gltf` — an **intentional** corrupt test fixture (verdict GOOD). Totals: 62,458,094 triangles; 166 files contain animations, 108 contain skins (concentrated in character packs: Kenney `3D assets` 115, KayKit 32, `assets/models` 19 — expected for character assets, none flagged in env-only material sets). UV coverage: only 5 files lack UVs entirely. Tangents: 5,927 files have no tangent attribute on any primitive — **not counted as a defect** because the render crate generates MikkTSpace tangents at load (`astraweave-render/src/mesh.rs:138`, called from the glTF path; Lengyel fallback `mesh.rs:189`). Note the caveat: astraweave-asset's *runtime* `MeshData` extracts positions+normals only, so tangent policy there is moot until that path grows normal mapping.

**OBJ (9,370):** 9,367 parse (4,550,165 triangles total; zero missing `mtllib` targets; zero missing `.mtl`-referenced textures). 3 files are **empty Kenney exports** — header + `mtllib` line, zero geometry: `assets/3D assets/Mini Arena/Models/OBJ format/character-soldier.obj`, `assets/3D assets/Mini Skate/Models/OBJ format/character-skate-{boy,girl}.obj` (verdict REPLACE — glTF twins of the same characters exist in the same packs).

**DAE (1,264):** all well-formed XML; 11 files carry unresolvable `<init_from>` image refs.

**FBX (13,401):** parse status `UNKNOWN` (no parser this session). Header sniff: 11,872 binary (versions: 7400 ×8,392; 7700 ×3,461; 7100 ×14; 7300 ×3; 7200 ×2), 1,529 ASCII-probable, 0 unrecognized.

**BLEND (166):** 164 `BLENDER-v*` headers, 2 zstd-compressed. Parse `UNKNOWN`. **USDC (1):** `PXR-USDC` magic OK, parse `UNKNOWN`.

**Textures (56,926):** 56,838 fully decode. **All 88 failures are the same defect**: files with `.ktx2` extension whose payload is a legacy custom `AW_TEX2\0` container — see finding F1. Non-PoT: 24,659 (20,306 in `2D assets`, 1,489 `UI assets`, 987 `Icons` — normal for sprites/UI; only 22 in `assets/textures`). Oversize (>4096): 12 files (8K HDRIs/normals in `assets/textures` and `assets/imported`, 2 Kenney tilesheets). Tiny (≤2×2): 3 — Kenney "Development Essentials 1×1 Pixels" utility sprites, intentional pack content.

---

## 5. Reference analysis (Phase 3)

Classification rules (stated so absence is meaningful): a file is **REFERENCED** when a code/manifest/config/test/pack file names it and the name resolves by exact path, path relative to the referencing file, `assets/`-prefix, or unambiguous path-suffix. Basename-only string matches were *rejected* as evidence. **INDIRECT** = reachable through a cited scan/manifest mechanism. **ORPHANED** = neither. Whole-tree scans that reach *everything* (`aw_build` distribution copy `tools/aw_build/src/main.rs:248`, editor asset browser `asset_browser.rs:719`, editor AssetDatabase scan `tools/aw_editor/src/main.rs:8891`) were **not** counted as INDIRECT — counting them would make the classification vacuous. Archive-quality caveat: refs were not extracted from `archive/`, `gh-pages/`, build logs, or lockfiles.

Counted mechanisms (all cited from code read this session):
1. **Editor entity catalog** — scans `assets/` for `.glb/.gltf/.fbx/.obj` and auto-discovers any top-level dir containing models (`tools/aw_editor/src/panels/entity_catalog.rs:96,195`). This is what makes 32k+ pack models INDIRECT — **editor-only reachability**.
2. **Biome material packs** — `assets/materials/<biome>/materials.toml` + `arrays.toml` (`astraweave-render/src/material.rs:366`; hardcoded biome list `biome_material.rs:181,229`; production caller `examples/unified_showcase/src/material_integration.rs:169`).
3. **HDRI catalog** — `assets/hdri/hdri_catalog.toml` (`astraweave-render/src/hdri_catalog.rs:3`, `biome_material.rs:101`). 11/12 HDRIs referenced; 1 orphaned.
4. **World-partition cells** — `assets/cells/{x}_{y}_{z}.ron` (`astraweave-scene/src/streaming.rs:180,190`) — engine runtime path; see finding F2.
5. **Imported biome packs** — `assets/imported/<pack>/manifest.json` (`astraweave-terrain/src/biome_pack.rs:235,352`).
6. **Editor impostor cache** — `assets/cache/impostors/<hash>/atlas.{png,toml}` (`tools/aw_editor/src/viewport/impostor_registry.rs:4`).
7. Hardcoded example paths (unified_showcase `main.rs:1274,1369,944`; hello_companion; splash `tools/aw_editor/src/splash.rs:21`).

Result: REFERENCED 938 · INDIRECT 32,878 · ORPHANED 57,218. Caveat: REFERENCED counts any path-resolving literal in code/manifests/tests — including non-compiled backup files and test string literals (e.g. `assets/models/character-a.glb` is cited only by the non-compiled `examples/unified_showcase/src/main_clean.rs:1160` and a throwaway script; `assets/materials/baked/grass.ktx2` — the lone FIX-class AW_TEX2 — only by a serialization-test literal that never loads the file). Where a verdict boundary rests on such a weak surface, the `referenced_by` column shows the exact citation so the reader can weigh it. The dominant orphan mass is the **2D sprite/icon/UI library (~50k files): the engine has no 2D sprite runtime that loads them** — they are reachable only through generic file browsing. A further 182 orphans have docs-only mentions.

**Key structural fact:** the shipping engine runtime reaches assets only via mechanisms 2–5 plus hardcoded example paths. Everything else — the entire Kenney/KayKit model library included — is reachable **only through the editor**.

---

## 6. Broken references (code/manifests → nonexistent files)

Raw unresolved candidates: 1,984; after removing synthetic/test-utility strings (astraweave-blend path-handling corpus, generator-tool *output* names, unit-test literals) and Kenney atlas-XML subtexture *keys* (128+128+39+26 entries in `UI Pack*` spritesheet XMLs are atlas entry names, not file paths): the load-bearing findings are:

| # | Referencing surface | Count | Sample / detail |
|---|---|---|---|
| B1 | `assets/cells/*.ron` (engine streaming manifests, `streaming.rs:180`) | **31 distinct paths** | `models/nature/pine_tree.glb`, `models/nature/oak_tree.glb`, `textures/nature/birch_bark.png`, `models/spawners/invisible.glb`… — **none exist anywhere in the repo** (`assets/models/` has no `nature/` subdir). Every world-partition cell that lists assets points at missing files. |
| B2 | `astraweave-terrain/src/structures.rs:540-571` | **23 filenames** | `get_model_path()` builds `assets/models/structures/<name>.glb` for all 23 `StructureType`s — **`assets/models/structures/` does not exist**. |
| B3 | `astraweave-render/src/terrain_material.rs:265-302+` | **49 texture names** | `TerrainMaterialDesc::grassland()` (and sibling presets) reference 51 distinct texture filenames (`grassland_splat.png`, `grass_albedo.png`, `dirt_normal.png`…); only 2 exist anywhere in the repo (`moss_albedo.png`, `moss_normal.png`) — the other **49 exist nowhere**. |
| B4 | `assets/imported/verdant_trail/manifest.json` + `decomposition_result.json` | 58 + 58 | `rocks_ground_01_diff_4k.png`, `forest_leaves_04_diff_8k.png`… — blend-decomposition manifests reference textures that were never extracted/committed. Same for `assets/imported/Namaqualand/`: 55 + 55. |
| B5 | `assets/textures/<biome>/materials.toml` fallback packs | 48 (grassland) + 48 (demo) + 16 (forest) + 14 (desert) | the `assets/textures/{biome}` fallback material dirs (fallback base cited at `material_integration.rs:169`) reference missing texture files. |
| B6 | `assets/textures/atlas_config.toml` + `examples/unified_showcase/src/material.rs:149-242` + `examples/unified_showcase/assets/textures/atlas_config.toml` | 44 + 16 + 21 | whole `structures/`, `characters/`, `skybox/`, `effects/` texture families (leaves_oak, wood_wall, thatch_roof, adobe_wall, day/night sky…) do not exist. |
| B7 | `examples/visual_3d/src/main.rs:67,363` | 2 | `assets/default_n.png`, `assets/skinned_demo.gltf` missing. |
| B8 | **glTF external texture refs**: 123 GLB/GLTF under `assets/models/` | 123 files | pack models copied into `assets/models/` **without their `Textures/` folders** (`barrels.glb → Textures/barrel.png`, `battlement*.glb → Textures/cobblestone.png`, `character-a.glb → Textures/texture-a.png`…). Geometry loads; materials cannot resolve. These are the models the editor entity catalog spawns. |
| B9 | `assets/models/Amber-Npc/Amber.json` | 69 | Character-Creator export JSON references source textures not present (pack-internal). |
| B10 | DAE `<init_from>` | 11 files | unresolvable image refs inside Kenney DAE variants. |

The one intentional case: `astraweave-asset/tests/fixtures/missing_buffer.gltf` (missing-buffer test fixture).

---

## 7. Finding F1 — the fake-KTX2 (`AW_TEX2`) population

All **88** texture-decode failures are files named `*.ktx2` whose first 8 bytes are `AW_TEX2\0` (each file's magic was read and checked individually): 36 in `assets/materials/baked/`, 36 in `assets/materials/`, 13 in `baked_materials/`, 3 in `archive/test_outputs/test_baked/`. Facts established from code:
- **No Rust code reads AW_TEX2** — the only mentions in the repo are two Python migration scripts (`tools/scripts/migrate_awtex2_to_ktx2.py`, `tools/scripts/validate_ktx2_migration.py`) and archived docs.
- The runtime KTX2 loader (`material_loader.rs:286` → `ktx2::Reader`) would reject them (bad magic).
- The *current* baker writes real KTX2 magic (`tools/aw_asset_cli/src/texture_baker.rs:278-297`), so these are **stale outputs of an older baker version**.
- They are **not on the live runtime path**: biome `materials.toml` files reference the PNG sources (e.g. `assets/materials/forest/materials.toml:6` → `../forest_floor.png`), not the `.ktx2` bakes.

Exactly **one real KTX2** exists in the repo: `archive/test_outputs/test_output_dir/grass.ktx2` — meaning the dedicated runtime KTX2 decode path has essentially no committed assets exercising it either.

Verdicts: 87 DELETE (stale artifacts, regenerable), 1 FIX (`baked_materials/dirt_mra.ktx2`-class entries where a `.meta.json` chain is the only reference were still classed DELETE; the single FIX is a directly-referenced case — see CSV).

---

## 8. Provenance (Phase 4)

Method: 292 license/readme sidecars located by filename scan; grouped by SHA-256; one representative per group read; classification mapped to every asset under the sidecar's directory. Filenames suggesting a source (PolyHaven naming, Quixel naming) were recorded as hints only, never as licenses.

| Provenance | Files | Evidence example |
|---|---|---|
| CC0 (Kenney) | 70,737 | per-pack `License.txt` (e.g. `assets/2D assets/1-Bit Pack/License.txt`); umbrella `assets/Readme.html` (Kenney All-in-1 bundle) |
| CC0 (KayKit / Kay Lousberg) | 17,459 | `assets/The Complete KayKit Collection v4/License.txt` |
| CC0 (Poly Haven) | 61 | `assets/_downloaded/polyhaven/ATTRIBUTION.txt`; per-HDRI `README.md` (kloppenheim, spruit_sunrise, venice_sunset) |
| Unity Companion License / MIT | 137 | `assets/Forest Scene/Library/PackageCache/*/LICENSE.md` — covers Unity SDK packages only, **not** the Forest Scene art itself |
| Unclear (support doc only) | 5 | `assets/Symphonie/Ruins/README.md` — contact page, no license text |
| **PROVENANCE UNKNOWN** | **2,635** | see below |

**Unknown-provenance risk list** (engine is MIT-licensed and public; these ship with zero on-disk license evidence):
- `assets/imported/` (567 files, 9.0 GB) — PolyHaven-style *naming* hints only; zero license files.
- `assets/Road to Vostok Assets Vol.1` (186) — a known commercial pack name; **no license/receipt/readme found anywhere in the folder** (targeted search).
- `assets/materials/` (162) + `assets_src/materials` (63) — biome source textures, no license file (root `assets/README.md` self-reports "Mixed (see per-asset)" and explicitly says "Check original … license" for two packs, i.e. the project's own doc admits non-verification).
- `assets/Forest Scene/` scene art (150) — only Unity's own package licenses exist, nothing for the tree/terrain content.
- `assets/models/` incl. `Amber-Npc` (158) — README.md is a workflow doc, not a license; Amber-Npc filenames hint Character-Creator origin (hint only).
- `assets/textures/` non-polyhaven (loose 8K HDRIs, pbr/, pine_forest/, fabrics/…), `assets/castles_forts_asset_pack` (5), `assets/Texture` (8), `assets/Mesh` (2), `assets/Goodies` (6), `assets/hdri` loose files (9), `assets/cache` (67), root-level loose files (`assets/Albedo.jpg`, `assets/Namaqualand.blend` 122.7 MB, `assets/verdant_trail.blend` 222.3 MB, eight `*_4k.gltf` PolyHaven-named material sets).

---

## 9. Verdict rules and totals (Phase 5)

Rules applied mechanically, one verdict per asset, evidence string per row in the CSV:

- **GOOD** — parses/decodes clean in a supported format; or intentional test fixture. Orphaned-but-clean pack content stays GOOD (status column carries the orphan signal). Non-PoT noted but not penalized (sprites/UI).
- **FIX** — specific correctable defect: broken external texture refs (123 GLBs, B8); referenced-or-catalog-listed unsupported format → convert (FBX 326, OBJ 428, BLEND 138); oversize runtime textures (5); AW_TEX2 with live reference (1); USDC (1); HDR/EXR 8K in runtime dirs (5).
- **DELETE** — orphaned AND unsuitable: unsupported-format duplicates whose same-stem `.glb/.gltf` twin exists in the same pack (FBX 13,075 / OBJ 8,939 / DAE 1,264 / BLEND 28 — KayKit ships every model ×4 formats; the GLB is the only loadable one), stale AW_TEX2 bakes (87), repo-root screenshots (3 files: `editor_screenshot.jpg/png`, `editor_ss.png`), unreferenced docs images (5 png / 1 jpg).
- **REPLACE** — referenced/catalog-listed but fundamentally defective: the 3 empty Kenney OBJ character exports.
- **UNKNOWN** — 4 gh-pages images (site-branch mirror; references not scanned).

| Verdict | Count | Dominant composition |
|---|---|---|
| GOOD | 66,605 | Kenney/KayKit GLB+GLTF, all sprite/UI PNGs, biome PNGs |
| FIX | 1,023 | 754 format-conversions flagged by editor-catalog mismatch, 123 broken-texture GLBs, rest above |
| DELETE | 23,399 | 23,306 unsupported-format pack duplicates + 87 AW_TEX2 + 6 misc |
| REPLACE | 3 | empty OBJs |
| UNKNOWN | 4 | gh-pages |

Cross-check: 66,605+1,023+23,399+3+4 = 91,034 ✓; 938+32,878+57,218 = 91,034 ✓.

**Caution on the DELETE class**: "unsupported-format duplicate" is a *static* judgment — the FBX/OBJ/DAE copies are dead weight for the engine as shipped, but deleting them alters vendored pack integrity. That trade-off is a director decision; this audit only establishes that no code path can load them and a loadable twin exists.

---

## 10. Additional observations (in-scope, not verdict-bearing)

- **Editor catalog lists formats it cannot spawn** — `entity_catalog.rs:90` accepts `fbx`/`obj`, spawn path is glTF-only (`engine_adapter.rs:1094`): 754 files produce catalog entries that would fail on spawn (the FIX-class conversion list).
- **233 double-extension textures** (`*.jpg.png`, `*.hdr.png`, `*.exr.png`) in `assets/imported/*/textures/` — blend-import artifact naming; files decode fine (they are real PNGs) but naming implies a lossy/converted chain.
- `AnimationLibrary_Godot_Standard.glb` and Godot-flavored KayKit variants sit in `assets/models/` (engine-agnostic GLB, loads fine — naming only).
- The 12 oversize (>4096) textures include three 8K HDRI *PNG conversions* in `assets/imported/Namaqualand/textures/` and an 8192² normal map (`assets/textures/rock_07_nor_gl.png`).
- `assets/hdri/`: 11 of 12 HDRIs referenced via `hdri_catalog.toml`; several PolyHaven-named `.hdr` files lack the per-file README the three documented ones have (provenance UNKNOWN).

## 11. Exclusions

- `target/`, `target-test/`, `target-test2/` — build output; 20 asset-extension matches found there (19 `flag_check.obj` cc-crate compile probes, 1 rustdoc favicon PNG), all excluded, none are art.
- `.git/` — VCS internals, skipped.
- No vendored *tool* asset dirs were found to exclude (searched; the third-party content under `assets/` is the audit subject itself, not tool baggage). `gh-pages/` (4 images) and `archive/` (4 files) were enumerated and appear in the CSV; their references were not scanned (site mirror / archived debris), statuses marked accordingly.

## Appendix A — raw enumeration

The complete per-file list (path, type, format, size in bytes — plus all Phase 2/3/5 fields) is delivered as [`ASSET_REGISTRY.csv`](ASSET_REGISTRY.csv) (91,034 rows). Embedding 91k rows in this markdown file would make it unusable; the CSV **is** Appendix A. Per-extension and per-directory totals are in §3. The 20 excluded build-output files are listed in scratchpad `enum_assets.json` (`excluded` array) and summarized in §11.

## Appendix B — registry column notes

`referenced_by` holds up to 2 citations (`file:line` or `mechanism: …`). `has_uv`/`has_tangents` for glTF are per-primitive rollups (`all`/`some`/`none`); tangents `UNKNOWN` for FBX/BLEND/USDC (unparsed). `parses=UNKNOWN` for FBX/BLEND/USDC (header sniff only). Extra columns beyond the mandated set: `width`, `height`, `pot`, `flags` (anim/skin).

---

## Addendum A — AD.1.A missed-file-class sweep (2026-07-05)

### A.1 Enumeration-coverage correction

The Phase-1 enumeration (§1, §3) kept only 14 model/texture extensions: `png jpg/jpeg fbx blend obj exr glb ktx2 gltf tga dae hdr bmp usdc`. **That list is documented as incomplete as of 2026-07**: it silently excluded video, audio, font, and several image classes that exist in the repo. This addendum supersedes §3 as the statement of enumeration coverage; the 91,034-row registry remains valid for the classes it covers, but the audit's totals were an undercount of the repo's media surface by **6,419 files** in the core missed classes plus the census classes below. Success-criterion arithmetic that used "everything in-repo" (e.g. AD-series criterion 3, 100% documented provenance) must use this corrected denominator.

### A.2 Sweep scope and method

Enumerated 2026-07-05 with `find` over the worktree, excluding `target*/`, `.git/`, `node_modules/` (same exclusion intent as §11; the two crate-local `target/` dirs found — `astraweave-audio/target`, `astraweave-scene/target` — were also excluded). Classes swept: video `.mp4 .webm .mov .avi .mkv`; audio `.wav .mp3 .ogg .flac .aiff`; fonts `.ttf .otf .woff .woff2`; skipped image classes `.gif .ico .svg .webp`; plus a **full extension census** of every remaining file to surface anything else binary/media-like. Reference/liveness and provenance follow the AD.0/AD.1 standards (live-code citation or ORPHANED with method; source-document-or-UNTRACEABLE, no inference). Full per-cluster license evidence: `THIRD_PARTY_LICENSES.md` §10.

### A.3 Counts per class (core missed classes)

| ext | files | bytes | tracked | notes |
|---|---:|---:|---:|---|
| svg | 4,849 | 53,002,252 | 4,847 | 4,842 in Kenney packs (License.txt-covered, incl. zip-embedded); 4 `Other/Miniguides` UNTRACEABLE; 1 first-party `docs/src/assets/og-image.svg`; 2 untracked `docs/book/` build outputs. **No SVG decoder exists workspace-wide** — class is asset-library payload only |
| ogg | 1,363 | 131,723,692 | 1,363 | 1,286 in the 16 Kenney `assets/audio` packs; 56 in 3 other Kenney packs; 20 AlkaKrab; 1 first-party test fixture |
| wav | 125 | 2,865,692,824 | 125 | **84 = `water_ambient_*` 2.196 GB UNTRACEABLE**; 20 AlkaKrab; 15 first-party test audio; 6 Forest Scene (existing quarantine cluster) |
| mp3 | 20 | 151,661,502 | 20 | all AlkaKrab |
| ttf | 36 | 995,528 | 36 | all in Kenney packs; 16 (`assets/Other/Fonts`) lack any license doc |
| otf | 14 | 368,008 | 14 | all in Kenney Input Prompts packs (licensed) |
| woff2 | 11 | 486,652 | 0 | all `docs/book/` gitignored mdBook build output (Open Sans Apache-2.0, Source Code Pro OFL 1.1 per local license files) |
| mp4 | 1 | 19,149,666 | 1 | editor splash video — first-party Artlist AI Output; **plain git blob (`.mp4` is not an LFS pattern)** |
| **total** | **6,419** | **3,223,080,124** | **6,406** | 13 untracked = all `docs/book/` |

Zero-hit classes (confirmed by find, exit 0, empty): `.gif .ico .webp .mov .avi .mkv .flac .aiff .woff`. Font classes `.eot .pfb .fnt .bdf .pfa .pfm`: also zero.

### A.4 Census classes (binary/media-like, previously unenumerated; attributed to clusters, no per-file registry rows)

`.swf` 264 (Kenney 2D/Icons/UI/Archive pack sources) · `.psd` 90 (Forest Scene 48, Road to Vostok 39, **`assets/textures/billboard_foliage_*` 3 × 50,331,688 B UNTRACEABLE**) · `.stl` 1,264 (Kenney 3D-print variants, 9.3 MB) · `.tif` 53 + `.mdb` 64 + `.dll` 114 + `.cube` 6 (all Forest Scene, existing quarantine cluster) · `.ai` 10, `.capx` 16, `.tmx` 20, `.url` 228, `.zip` 58, `.3ds` 8, `.unitypackage` 7 (Kenney/Archive/KayKit pack internals) · `.pdf` 7 (3 AlkaKrab license copies, 2 Goodies, 1 KayKit, 1 Forest Scene). Non-media census classes (`.ron .bin .log .cs .meta` etc.) are engine/Unity-SDK data, out of audit scope.

### A.5 Findings

1. **F-A1 (HIGH): AlkaKrab music is tracked + pushed against its license.** `assets/audio/Tracks/` + `Loops/` (60 files, ~922.6 MB, commit `54d10f736`, on `origin/main`) ship an in-pack license PDF that prohibits redistribution as-is and requires direct permission for open-source games. Manifest §10.3; director decision §8 item 1 (quarantine+purge vs. permission email).
2. **F-A2 (HIGH): `water_ambient_*` — 2.196 GB, zero license basis.** 84 REAPER-produced WAVs (bext dates 2019-08), renamed from `loadless_WATER_*` (`ASSET_CLEANUP_REPORT.md:76-80`); `validate_assets.ps1:252` asserts "third-party" with no source. UNTRACEABLE → quarantine-recommended. Largest unresolved cluster in the repo.
3. **F-A3: second case-only index/worktree mismatch.** Index holds `assets/audio/Tracks/`+`Loops/`, worktree `tracks/`+`loops/` — same class as the `Fabrics/`↔`fabrics/` finding; breaks case-sensitive clones and defeated naive pathspec queries during this sweep.
4. **F-A4: fonts are clean.** UI text renders exclusively from egui 0.32.3's four embedded fonts (UFL-1.0 / MIT / OFL-1.1 / MIT — license files read from the cargo-registry crate copy). All 50 tracked in-repo TTF/OTF are orphaned-in-code; no distributed font has an unknown license. One gap: `assets/Other/Fonts` (16 Kenney TTF) has no license doc — quarantine-recommended as inert library payload.
5. **F-A5: the splash video is a 19.1 MB plain git blob** (not LFS), decoded at every editor launch (`splash.rs:22,444-500`, `mp4`+`openh264`); missing-file fallback is graceful (0.8 s logo-only splash + one `tracing::warn`). It therefore survives the AD.6 LFS-elimination rewrite automatically.
6. **F-A6: `EditorAudioBridge` blanket-scans `assets/audio`** (`audio_bridge.rs:77,242`) — every audio file incl. quarantine candidates is live-reachable from the editor's audio panel; there is no curated allow-list. (Observation, not a defect per se; relevant to what "removing" a cluster changes: nothing breaks, entries simply vanish from the scan.)
7. Minor: `assets/audio/ambient/manifest.toml` is an orphan placeholder (nothing parses it; its 13 `.ogg` targets don't exist; `biome_audio.rs:32-45` hardcodes the same paths); `astraweave-audio/tests/fixtures/README.md:25` ("NOT checked into git") is stale — the fixtures are tracked; `docs/benchmarks/index.html:60` references a nonexistent fonts CSS.

### A.6 Sample-set / budget recommendation (recommendation only, no action taken)

- **Splash video (19,149,666 B)**: live production consumer at every editor launch; first-party rights (Artlist AI Output, manifest §4); already in-repo as a plain blob. Counting it into the AD.0 sample-set accounting raises ~93.7 MB → **~112.8 MB**: over the 100 MB target, well under the 250 MB hard ceiling. **Recommendation: keep in-repo and accept the target overage** — the fallback (0.8 s logo splash) makes pack-relegation *feasible*, but a first-party file with a live unconditional consumer is exactly what the sample set is for. Optional later beat: re-encode at lower bitrate/resolution (mutating; could plausibly reach a fraction of the size).
- **Logo (1,469,159 B + 114,916 B derivative)**: live ×3 (window icon, brand mark, splash) — belongs in the sample set; the docs/gh-pages copies are site infrastructure, not asset-pack content.
- **Kenney audio packs (28.6 MB) + 3 ogg-bearing packs + `alps_field_2k.hdr`**: pack-eligible (release artifacts), **not** sample-set — no curated live consumer beyond the blanket scan.
- **AlkaKrab (922.6 MB) + water_ambient (2.196 GB)**: quarantine per manifest §10.3/§10.4 — together ~3.1 GB of LFS storage the AD.6 rewrite reclaims.
- Evidence-file note: the three Artlist screenshots under `docs/audits/evidence/` are `.png` and therefore currently ride the blanket LFS rule when staged. License evidence must survive the LFS elimination — AD.2's `.gitattributes` work should add a `docs/audits/evidence/**` exemption (the same pattern as the existing `docs/src/assets` exemptions).
