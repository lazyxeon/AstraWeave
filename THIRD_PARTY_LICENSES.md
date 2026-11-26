# Third-Party Asset Licenses

**Scope**: the ratified AD.0 sample set (153 in-repo rows + 1 ratified acquisition) and the release-pack contents traced to date (see `docs/audits/DISPOSITION_REPORT_imported.md`), **plus the AD.1.A missed-file-class sweep (§10)** — video/audio/fonts/SVG and other classes the original audit never enumerated. Quarantined material is **intentionally absent** from this manifest — untraceable assets are never shipped in-repo or on release pages. | **Produced**: AD.1, 2026-07-05; AD.1.A addendum (§4 Artlist rows closed, §10 added), 2026-07-05; AD.1.B pack-bucket remainder (§11), 2026-07-06; AD.1.C residual trace + ratification (§8 flipped to RATIFIED, §12 added), 2026-07-06. | **Standard**: an asset appears as TRACED only when a source document states its license terms (upstream license page fetched this session, a LICENSE/README shipped inside the pack, or a documented storefront license). Every URL cited below was fetched on the stated retrieval date; operative terms are quoted. Filename conventions and "site X is usually CC0" were not accepted as evidence. Byte-identity (SHA-256) to a file inside a licensed pack is accepted as evidence that the copy is that pack's asset.

Manifest row format: `paths | upstream source + URL | license (operative term quoted) | attribution requirement | evidence location | retrieval/verification date`.

---

## 1. Pack licenses (in-repo `License.txt` + upstream confirmation)

### 1.1 Kenney packs (sample set uses 5 packs; the wider in-repo Kenney library carries per-pack License.txt files of the same template)

Upstream: <https://kenney.nl/support> — fetched 2026-07-05: *"all game assets on the asset pages are public domain licensed (CC0) … You're free to use them, even in commercial projects. … Attribution is not required."* (Note: the legacy `kenney.nl/data/license.txt` URL now returns 404.)

| paths | upstream | license | attribution | evidence | date |
|---|---|---|---|---|---|
| `assets/3D assets/Nature Kit/**` (18 sample scatter GLBs + bed/tree/tent/campfire/grass twins) | Kenney, <https://kenney.nl> | "License: (Creative Commons Zero, CC0)" | not required ("this is not mandatory") | in-repo `assets/3D assets/Nature Kit/License.txt` (Nature Kit 2.1); upstream <https://kenney.nl/support> | 2026-07-05 |
| `assets/3D assets/Castle Kit/Models/GLB format/tower-square.glb` + `Textures/colormap.png` | Kenney | CC0 (same template) | not required | in-repo `assets/3D assets/Castle Kit/License.txt` | 2026-07-05 |
| `assets/3D assets/Survival Kit/Models/GLB format/{box-large,campfire-pit,rock-a}.glb` + `Textures/colormap.png` | Kenney | CC0 | not required | in-repo `assets/3D assets/Survival Kit/License.txt` | 2026-07-05 |
| `assets/3D assets/Fantasy Town Kit/Models/GLB format/cart.glb` + `Textures/colormap.png` | Kenney | CC0 | not required | in-repo `assets/3D assets/Fantasy Town Kit/License.txt` | 2026-07-05 |
| `assets/3D assets/Retro Medieval Kit/**` (source pack of `assets/models/barrels.glb`) | Kenney | CC0 | not required | in-repo `assets/3D assets/Retro Medieval Kit/License.txt` | 2026-07-05 |

### 1.2 KayKit (Kay Lousberg)

Upstream: <https://kaylousberg.itch.io/kaykit-complete> — fetched 2026-07-05: *"These assets are CC0 so you can use them freely. But please don't resell unmodified copies or claim them as your own. … Creative Commons Zero v1.0 Universal."*

| paths | upstream | license | attribution | evidence | date |
|---|---|---|---|---|---|
| `assets/The Complete KayKit Collection v4/KayKit Adventurers 2.0/Characters/gltf/{Rogue,Barbarian,Mage,Knight}.glb` | Kay Lousberg, <https://kaylousberg.itch.io/kaykit-complete> | "License: (Creative Commons Zero, CC0)" + <http://creativecommons.org/publicdomain/zero/1.0/> | optional ("not mandatory") | in-repo `assets/The Complete KayKit Collection v4/License.txt`; upstream itch page | 2026-07-05 |
| `…/KayKit Skeletons 1.1/characters/gltf/{Skeleton_Warrior,Skeleton_Golem}.glb` | same | CC0 | optional | same License.txt | 2026-07-05 |
| `…/KayKit Forest Nature Pack 1.0/Assets/gltf/Color{1,2}/*.gltf` (9 sample trees/rocks) + `Color{1,2}/forest_texture.png` | same | CC0 | optional | same License.txt | 2026-07-05 |

---

## 2. Byte-identical pack copies under `assets/models/` (SHA-256 evidence, computed 2026-07-05)

These 10 loose files are exact copies of files inside licensed Kenney packs; the pack license above applies.

| path | identical to (SHA-256 match) | license |
|---|---|---|
| `assets/models/barrels.glb` | `assets/3D assets/Retro Medieval Kit/Models/GLB format/barrels.glb` | CC0 (Kenney) |
| `assets/models/bed.glb` | `assets/3D assets/Nature Kit/Models/GLTF format/bed.glb` | CC0 (Kenney) |
| `assets/models/tree_pineDefaultA.glb` | Nature Kit twin | CC0 (Kenney) |
| `assets/models/tree_pineRoundA.glb` | Nature Kit twin | CC0 (Kenney) |
| `assets/models/tree_pineTallA.glb` | Nature Kit twin | CC0 (Kenney) |
| `assets/models/tent_smallOpen.glb` | Nature Kit twin | CC0 (Kenney) |
| `assets/models/tent_detailedClosed.glb` | Nature Kit twin | CC0 (Kenney) |
| `assets/models/campfire_logs.glb` | Nature Kit twin | CC0 (Kenney) |
| `assets/models/tree_default.glb` | Nature Kit twin | CC0 (Kenney) |
| `assets/models/grass.glb` | Nature Kit twin | CC0 (Kenney) |

---

## 3. Poly Haven assets (upstream verified per-slug this session)

Site license: <https://polyhaven.com/license> — fetched 2026-07-05: *"Our assets are all licensed as CC0, which is effectively Public Domain even in jurisdictions that do not support the Public Domain."* Attribution: not required. Per-slug existence verified via `https://api.polyhaven.com/info/<slug>` (HTTP 200 + asset metadata), 2026-07-05.

> **Evidence-tier disclosure (applies to §3.1 and §3.2, NOT §3.3):** the *license* of each slug is upstream-verified, but the link between the local file and its slug rests on the decomposition-preserved filename — there is no in-pack LICENSE and no byte-identity to a fresh download (the LFS bandwidth embargo precludes re-downloading for byte comparison this session). This is a weaker identity tier than §1 (in-pack license) or §2 (SHA-256): rows here are **TRACED (name-linked)**. Rationale for accepting it: these files are outputs of a Blender-scene decomposition whose every *resolvable* member is a published Poly Haven asset, the names carry Poly Haven's exact slug + suffix conventions, and CC0 imposes no attribution/redistribution obligation, so the residual risk is misidentification of CC0-for-CC0. Names that did NOT resolve were honestly excluded (§5/§6), not force-matched. A post-embargo hardening step may byte-compare one representative mesh against a fresh Poly Haven download. §3.3 is stronger: the import script's `SOURCES` map is repo-committed documentation of origin, independent of filenames.

### 3.1 Sample-set HDRIs and textures

| paths | slug (API-verified) | authors (from API) | evidence | date |
|---|---|---|---|---|
| `assets/hdri/polyhaven/kloppenheim_02_puresky_2k.hdr` | `kloppenheim_02_puresky` | Greg Zaal / Jarod Guest | API + <https://polyhaven.com/license> | 2026-07-05 |
| `assets/hdri/polyhaven/kloppenheim/kloppenheim_06_puresky_2k.hdr` | `kloppenheim_06_puresky` | (in-repo README + API) | in-repo `assets/hdri/polyhaven/kloppenheim/README.md`; polyhaven.com/license | 2026-07-05 |
| `assets/hdri/polyhaven/spruit_sunrise/spruit_sunrise_2k.hdr` | `spruit_sunrise` | (in-repo README) | in-repo README.md; polyhaven.com/license | 2026-07-05 |
| `assets/hdri/polyhaven/venice_sunset/venice_sunset_2k.hdr` | `venice_sunset` | (in-repo README) | in-repo README.md; polyhaven.com/license | 2026-07-05 |
| `assets/textures/grass_bermuda_01_diff_1k.jpg` | `grass_bermuda_01` | (API) | API + polyhaven.com/license | 2026-07-05 |

### 3.2 Sample-set scatter meshes (verdant_trail blend-decomposition, 22 files ≤3 MB)

Each mesh name maps to an API-verified Poly Haven model slug: `dead_tree_trunk` (+`dead_tree_trunk_02`), `jacaranda_tree`, `rock_07`, `rock_08`, `shrub_01`…`shrub_04` (all four verified individually, author Rico Cilliers), `grass_medium_02`, `stone_01`, `dry_branches_medium_01`, `tree_small_02`. Files: `assets/imported/verdant_trail/meshes/{dead_tree_trunk.001,dead_tree_trunk_02,jacaranda_tree_trunk,rock_07.001,rock_08.001,shrub_01_a,shrub_01_b,shrub_01_d,shrub_02_a,shrub_02_c,shrub_03_a,shrub_03_b,shrub_04_a,shrub_04_b,grass_medium_02_a..e,stone_01_LOD0.001,dry_branches_medium_01_a.002,tree_small_02_a}.glb`. License CC0 per polyhaven.com/license. Retrieval 2026-07-05. **Exceptions — see §5/§6:** `sticks_debris_a.glb`, `grass_debris_a.glb` (slugs do not resolve).

### 3.3 Biome material derivatives in `assets/materials/` (21 files: grass, forest_floor, mountain_rock, mud, stone, rock_slate, dirt × base/_n/_mra)

Chain of evidence: `scripts/import_terrain_textures.py:53-106` (`SOURCES` map) documents each family's Poly Haven source; the script downscales to 2048 and repacks MRA; adding commit `1fc266c93` (2026-03-18). Source slugs all API-verified 2026-07-05:

| material family | Poly Haven source slug | authors (API) |
|---|---|---|
| grass | `grass_medium_01` | Rob Tuytel / Rico Cilliers |
| forest_floor | `forest_ground_04` | Rob Tuytel / Rico Cilliers |
| mountain_rock | `rock_face_03` | Dario Barresi / Rico Cilliers |
| mud | `forest_ground_01` (script spells `forrest_ground_01`; API resolves) | Rob Tuytel |
| stone | `rocky_trail` | Amal Kumar |
| rock_slate | `rock_moss_set_01` | Kless Gyzen |
| dirt | `ganges_river_pebbles` | Amal Kumar |

License CC0 (polyhaven.com/license). The planned in-repo 1K derivatives (AD.4) inherit the same provenance. Transparency note: the adding commit (`1fc266c93`) titles these "backup placeholder textures," and repo docs describe the current material tier as pipeline scaffolding slated for replacement — provenance is sound regardless of that art-direction status.

---

## 4. First-party (project-original, MIT — engine license)

| paths | evidence | date |
|---|---|---|
| `assets/models/greybox/*.gltf` (6 files) | generated by `examples/greybox_generator/src/{echo_grove,boss_courtyard,fractured_cliffs,loom_crossroads,loomspire,side_alcove}.rs` (each `fs::write`s its gltf) | verified 2026-07-05 |
| `assets/materials/{sand,snow}{,_n,_mra}.png` (6 files) | procedurally generated: `scripts/import_terrain_textures.py:108` `PROCEDURAL=["sand","snow"]`, `gen_sand`/`gen_snow` (FBM noise, fixed seeds); commit `1fc266c93` | verified 2026-07-05 |
| `assets/textures/pbr/PBR_{2K,4K}/{Dirt_Mud,Moss_Ground,Sand_Desert,Stone_Terrain_Rock}/*.png` (56 files) | **procedurally generated (AD.1.C)** — `tools/pbr_gen/generate_pbr_textures.py` (header `# CC0 / Public Domain`): `MATERIALS = {Stone_Terrain_Rock, Dirt_Mud, Sand_Desert, Moss_Ground}` synthesised via deterministic `rand_sine_noise` with fixed seeds (200/500/600/700); `save_material` emits exactly `{BaseColor,Normal,Roughness,Metallic,Height,AO,ORM}` at `--res 2048`/`4096`. On-disk dims confirmed 2048²/4096²; structural match exact (4 materials × 7 channels × 2 resolutions). Same generator-identity tier as the sand/snow row above | verified 2026-07-06 |
| `assets/Astraweave_logo.jpg` (2048², 1,469,159 B; byte-identical tracked copies at `docs/src/assets/` + `gh-pages/assets/`, SHA-256 `1c8d90cba6dc…`; plus derived 1024² `docs/branding/Astraweave_logo.jpg`, 114,916 B, distinct hash — separately produced crop; plus one untracked mdBook build copy under `docs/book/`) | **CLOSED (AD.1.A, director-ratified)**: generated via Artlist **AI Toolkit** from the author's own prompt + the author's own ChatGPT-generated input concept image; classified **"AI Output"** under Artlist ToS — rights assigned to the author, no standalone-distribution restriction, survives subscription expiry; AI-generation disclosed (copyrightability of pure AI output contested; immaterial for MIT distribution). Evidence: generation-history screenshots, `docs/audits/evidence/artlist_generation_history_2026-07-05/` (archived; **name/visual-linked** tier). Live consumers: editor window icon + top-bar brand mark (`tools/aw_editor/src/ui/branding.rs:11,28` → `main.rs:10033`, `main.rs:5808`), splash logo phase (`tools/aw_editor/src/splash.rs:21`) | 2026-07-05 |
| `assets/8-second_Cinematic_logo_opening.mp4` (19,149,666 B; **plain git blob — `.mp4` is not an LFS pattern in `.gitattributes`**) | **CLOSED (AD.1.A, director-ratified)**: generated via Artlist **AI Toolkit** from the author's own prompt; classified **"AI Output"** under Artlist ToS — rights assigned to the author, no standalone-distribution restriction, survives subscription expiry; AI-generation disclosed. Same evidence directory as the logo (same generation session, prompt "logo forming in space", items badged "Video"). Live consumer: editor splash video, decoded at every launch via the `mp4` + `openh264` crates (`tools/aw_editor/src/splash.rs:22,444-500`; `tools/aw_editor/Cargo.toml:104-105`); missing-file fallback is graceful (0.8 s logo-only splash + one `tracing::warn`, `splash.rs:129-134,299-301`); click/key skips (`splash.rs:115-122`). **Scope limit (director instruction)**: this Artlist AI-Output disposition covers ONLY these two files; any other Artlist-sourced material requires its own Output-vs-Asset determination | 2026-07-05 |
| `astraweave-audio/tests/assets/*.wav` (13) + `astraweave-audio/tests/fixtures/{music_test.ogg,sfx_test.wav,voice_test.wav}` | **FIRST-PARTY-GENERATED** — synthetic sine/chord/formant audio produced by in-repo generators `astraweave-audio/tests/test_asset_generator.rs` and `tests/fixtures/generate_fixtures.rs`; every file's byte size matches the generator math exactly (e.g. `test_beep_440hz.wav` 22,094 = 44 + 0.5 s × 22,050 Hz × 2 B). Note: `fixtures/README.md:25` ("NOT checked into git") is stale — all are tracked. `music_test.ogg` is RIFF/WAVE despite the extension (generator's own note) | verified 2026-07-05 |
| `docs/src/assets/og-image.svg` (2,936 B) | **first-party** — project-authored Open Graph/Twitter social card (literal "AstraWeave / AI-Native Game Engine" text + project URL), wired via `docs/theme/head.hbs:5,12`; added `faf8ef43` | verified 2026-07-05 |

---

## 5. SUBSTITUTE-proposed (usable slot, current file untraceable — traced replacement identified)

> **Ratified path (2026-07-06, §8 item 4):** the substitute sources named here live in `assets_src/`, whose disposition is now **pack `materials-src`, deferred until after AD.4's re-cook**. The re-cook is the operation that resolves these slots; the untraceable current runtime copies are quarantine-destined and are not shipped in the interim.

| paths | problem | proposed substitute (traced) | consuming code path |
|---|---|---|---|
| `assets/materials/{cobblestone,gravel,ice,metal_rusted,moss,wood_planks}{,_n,_mra}.png` (18 files) | on-disk runtime copies added 2026-02-06 (`54d10f736`, message silent on source); no generator produces them; SHA-256 differs from the documented `assets_src` copies — origin unknown | **re-cook from `assets_src/materials/` same-name sources**, whose provenance is documented in commits `f56a76124`/`f5387f20e`/`76b15948d` and `assets/asset_manifest.toml`, and whose upstreams were all verified 2026-07-05: gravel→`gravel_concrete_03`✓, metal_rusted→`rust_coarse_01`✓, moss→`moss_01`✓, wood_planks→`wood_floor_deck`✓, cobblestone→`rocky_trail`✓ (all Poly Haven CC0), ice→ambientCG `Ice003` (https://ambientcg.com/view?id=Ice003 fetched 2026-07-05: *"All assets are released under the Creative Commons CC0 license"*) | `assets/materials/<biome>/materials.toml` layers; editor terrain `canonical_terrain_pack.rs:181,215` |
| `assets/imported/verdant_trail/meshes/{sticks_debris_a,grass_debris_a}.glb` (2 files) | slugs `sticks_debris`/`grass_debris` return 404 on the Poly Haven API and no search hit — unpublished scene sub-assets of the verdant_trail blend; no license doc names them | substitute with a traced debris asset in a later acquisition (Poly Haven has published debris-family slugs, e.g. `bark_debris_01` — **candidate only, not yet traced**), or QUARANTINE | `astraweave-terrain/src/biome.rs:1065` (sticks_debris), `:1052` (grass_debris) — scatter skips missing files gracefully (`engine_adapter.rs:2578`) |

---

## 6. QUARANTINE — RATIFIED (sample-set rows; 2026-07-06, §8 item 1)

> These rows are ratified into the quarantine list (untraceable class) — purged in the AD.6 rewrite, never uploaded. The former "QUARANTINE-RECOMMENDED / leave until traced" status is closed; they were not traceable under the standard and are not held open further.

| paths | why untraceable | code path that degrades |
|---|---|---|
| `assets/materials/{cloth,plaster,rock_lichen,roof_tile,tree_bark,tree_leaves}{,_n,_mra}.png` (18 files) | added 2025-09-22/29 (`c166bdc27`, `b24317195`) with source-silent messages; no generator emits them; the `assets_src` same-name copies are equally untraceable (no commit/doc states a source); circumstantial name-echoes to Poly Haven slugs did NOT meet the tracing standard | biome `materials.toml` layers naming these families (forest loses tree_bark/tree_leaves/rock_lichen layers; terrain/biomes sets lose cloth/plaster/roof_tile) — editor terrain falls back per-layer; grassland default (5 layers) is unaffected |
| `assets/textures/cobblestone.png` | added via GitHub web-UI uploads ("Add files via upload", `be6db6fb2`/`8cfdff8b8`/`d4d8bdfba`); no generator, no attribution linkage; NOT derived from the Poly Haven cobblestone download (pixel cross-correlation 0.0095) and not a Kenney colormap | `examples/unified_showcase/src/main.rs:1130` (ground texture) — substitute: re-point to a traced texture |

---

## 7. Ratified acquisition (D5)

| paths (incoming) | upstream | license | evidence | dates |
|---|---|---|---|---|
| Quaternius Stylized Nature MegaKit — `CommonTree_1` (gltf + bin + 3 PNGs, ~10.3 MB) | <https://quaternius.com/packs/stylizednaturemegakit.html> (CC0 zip mirror: opengameart.org, 104,088,529 B) | CC0 — <https://quaternius.com/faq.html> fetched 2026-07-05: *"All models are under the CC0 License. … attribution is not necessary."* | download parsed 2026-07-04 (AD.0): materials `alphaMode:"MASK"`, `alphaCutoff:0.2`, `doubleSided:true` verified | acquired 2026-07-04; license verified 2026-07-05 |

---

## 8. Director dispositions — RATIFIED (2026-07-06, AD.1.C)

Every item below was decided by the director at the AD.3.R / AD.1.B / AD.1.C gates. **After this section no disposition in the record is provisional.** The three formerly-pending checklist items (1–3) are closed as marked.

1. **Quarantine list — RATIFIED (item 1 + item 2 close).** The full quarantine list is ratified; its destiny is **purged from history in the AD.6 rewrite and never uploaded to any release page or pack**. The list has three classes that together populate the quarantine bucket (see §12 / `DISPOSITION_REPORT_imported.md` §9 for the partition):
   - **Untraceable** — 830 (AD.1) + 160 (AD.1.B) + 15 (AD.1.C `models/houses`, §12) = **1,005 files**.
   - **Hygiene / regenerable** — `archive/` (4 files, build debris) + `assets/cache/impostors/` (67 files, runtime cache) = **71 files** (G-2).
   - **Redundant duplicate** — `assets/tests/textures/texture-{a..r}.png` (18 files): traceable (Kenney Blocky Characters, §11.3) but zero-referenced byte-duplicates of a retained pack asset. **Traceability governs upload eligibility, not retention** — a redundant duplicate is dropped, not shipped (G-2).
   - Bucket total **1,094 files / 7,481,318,328 B**.
   - **AlkaKrab music** (`assets/audio/{Tracks,Loops}/`, 60 files, ~922.6 MB, §10.3) — **option (a): QUARANTINE.** Release-page redistribution of raw audio files is a stricter act than in-game use, and nothing in the engine consumes the tracks; the as-is-redistribution prohibition is dispositive. Removed in the AD.6 rewrite; never uploaded. (Option (b), emailing the licensor, is not pursued.)
2. **AD.1.A sweep quarantine additions — RATIFIED** (folded into item 1's untraceable class): `water_ambient_*` (2.196 GB), `assets/Other/Fonts` (16), `assets/Other/Miniguides`, `assets/Archive/Isometric Renders`, `assets/Goodies` (+ 2 root `.url` shortcuts), `assets/textures/billboard_foliage_billboards_*.psd` (3 files, 150.9 MB).
3. **Road to Vostok Vol.1 — RATIFIED: pack and upload with the traced set (item 3 closes).** Decision reversed from the earlier "hold back" caution: a deliberate strategic hold for the realistic-quality phase **plus** the upstream unpublication makes the release page the durable off-site copy of an irreplaceable CC0 asset. The release-page attribution note carries the Wayback citation and the unpublication fact: <http://web.archive.org/web/20260422060538/https://roadtovostok.itch.io/road-to-vostok-assets-vol1>.
4. **`assets_src/` (66 LFS files / 719,850,359 B) — pack `materials-src`, deferred until after AD.4's re-cook.** The acquisition record (`assets/asset_manifest.toml`) pins every source by upstream **slug/URL only — there is no content-hash field on any entry**, and several `assets_src` families are entirely unpinned (§6: `cloth`/`plaster`/`rock_lichen` sources are equally undocumented). Since not every source is pinned by URL **and** hash, the branch is the pack branch, not the purge-and-refetch branch (§3 of the AD.1.C slate; see `DISPOSITION_REPORT_imported.md` §9 for the pin evidence). The deferred `materials-src` pack inherits the per-family §5/§6 dispositions — the untraceable families are excluded when it is actually cut, after AD.4.
5. **Matching standard — RATIFIED** (binds future trace beats; recorded in `assets/packs.manifest.toml` methodology): a trailing-numeral variant name-link is admissible (`forrest_ground_01` / `moss_01`); a word-substitution name-link is not (`pine_trunk`). This closes the AD.1.B `moss`→`moss_01` lower-confidence flag in favour of TRACED (§11.5).

*Resolved 2026-07-05 (AD.1.A)*: the former item 1 — `Astraweave_logo.jpg` authorship — is closed as first-party Artlist AI Output (§4), together with the splash video.

---

## 9. Release-pack contents traced to date

Full cluster tables and quarantine recommendations: `docs/audits/DISPOSITION_REPORT_imported.md`. Summary: **Namaqualand** — 17 Poly Haven model/texture slugs API-verified CC0; `fine_leaf_01` quarantine-recommended. **verdant_trail** — all major slugs verified CC0; `dirt_bank`/`sticks_debris`/`grass_debris` quarantine-recommended. **Road to Vostok Vol.1** — CC0 per archived itch.io storefront terms. **`assets/models/pine_tree_01_1k.glb`** (the 1 GiB R2 file) — Poly Haven `pine_tree_01`, CC0 (API-verified 2026-07-05). **HDRI catalog set** — `goegap`, `table_mountain_2`, `misty_pines`, `rainforest_trail`, `qwantani_moonrise_puresky`, `rogland_clear_night`, `kloofendal_48d_partly_cloudy_puresky`, `rogland_sunset` all API-verified CC0. **Root `assets/*_4k.gltf` material sets** — `aerial_beach_01`, `aerial_rocks_01`, `coast_land_rocks_01`, `sandy_gravel_02`, `snow_02`, `leafy_grass`, `ganges_river_pebbles`, `forest_leaves_02` all verified CC0. **`assets/models/AnimationLibrary_Godot_Standard.glb`** — Quaternius Universal Animation Library (https://quaternius.itch.io/universal-animation-library, CC0 per Quaternius site-wide statement). **AD.1.A additions**: the 16 `assets/audio` Kenney packs + 3 ogg-bearing Kenney packs (§10.1) and `assets/textures/alps_field_2k.hdr` (§10.6) are pack-eligible.

---

## 10. AD.1.A — missed-file-class sweep (2026-07-05)

The original asset audit enumerated only 14 model/texture extensions (see `docs/audits/ASSET_AUDIT_REPORT.md`, Addendum A — the enumeration-coverage correction). This sweep covers the missed classes: video (`.mp4 .webm .mov .avi .mkv`), audio (`.wav .mp3 .ogg .flac .aiff`), fonts (`.ttf .otf .woff .woff2`), skipped image classes (`.gif .ico .svg .webp`), plus a full extension census for anything else binary/media-like. **6,419 files found** in the swept core classes (svg 4,849 · ogg 1,363 · wav 125 · ttf 36 · mp3 20 · otf 14 · woff2 11 · mp4 1); 6,406 tracked, 13 untracked (all gitignored `docs/book/` mdBook build output). Zero `.gif .ico .webp .mov .avi .mkv .flac .aiff .woff` exist repo-wide.

**File-level roll-up (sums to 6,419)**: 6,218 TRACED pack-eligible (Kenney) · 60 TRACED-but-redistribution-restricted → quarantine (AlkaKrab) · 18 first-party (§4) · 110 UNTRACEABLE → quarantine (84 water_ambient + 16 `Other/Fonts` + 4 Miniguides SVGs + 6 Forest Scene WAVs, the last extending the existing AD.1 quarantine cluster) · 13 excluded (build output). Zero silently unresolved.

### 10.1 TRACED — Kenney audio packs (in-repo `License.txt`, CC0)

All 16 pack dirs under `assets/audio/` ship a `License.txt` naming Kenney (www.kenney.nl) and CC0 — every one read in full 2026-07-05. Standard operative text (15 of 16, quoted from `assets/audio/Digital Audio/License.txt`): *"License: (Creative Commons Zero, CC0) http://creativecommons.org/publicdomain/zero/1.0/ / This content is free to use in personal, educational and commercial projects. / Support us by crediting Kenney or www.kenney.nl (this is not mandatory)"*. Casino Audio carries Kenney's older phrasing (*"by Kenney Vleugels (Kenney.nl)"* … *"You may use these assets in personal and commercial projects. Credit … would be nice but is not mandatory."*) — same CC0 grant. Packs (files/bytes incl. the license file itself): Casino Audio 57/962,713 · Digital Audio 64/944,151 · Foley Sounds 87/1,207,694 · Impact Sounds 132/1,312,195 · Interface Sounds 102/1,177,108 · Music Jingles 87/1,459,721 · Music Loops 31/6,029,702 · RPG Audio 53/838,630 · Retro Sounds 1 36/261,522 · Retro Sounds 2 67/825,887 · Sci-Fi Sounds 75/6,169,270 · Synth Voice 1 102/1,111,415 · Synth Voice 2 228/2,382,439 · UI Audio 53/650,115 · Voiceover Pack 96/1,992,578 (voice-actor credits in file: Giselle, Jeffrey M. Smith — non-mandatory) · Voiceover Pack Fighter 49/1,302,941. **Total 1,319 files, 28,628,081 B — pack-eligible.** Also TRACED via root `License.txt` (CC0, read 2026-07-05): `assets/2D assets/Desert Shooter Pack` (40 ogg), `assets/2D assets/New Platformer Pack` (10 ogg), `assets/UI assets/UI Pack` (6 ogg under `Sounds/`).

**Liveness note**: the editor's `EditorAudioBridge` blanket-scans `assets/audio` recursively (`tools/aw_editor/src/audio_bridge.rs:77,242`, ext filter `:31`) and plays any hit via `AudioEngine::{play_music,play_sfx_file,play_voice_file}` (`astraweave-audio/src/engine.rs:64-66,343-345,373-375`) — so **everything under `assets/audio` is live-reachable**, including the quarantine-recommended clusters below.

### 10.2 TRACED — Kenney collection coverage for the swept classes (svg/fonts/swf/ai/capx/tmx/url/zip/stl/3ds)

Per-pack-dir coverage check across the collection roots (counts of swept-class files: `2D assets` 1,425 · `Icons` 1,968 · `UI assets` 1,783 · `3D assets` 223 · `Other` 37 · `Archive` 46): **`Icons` (7 pack dirs), `UI assets` (8), `3D assets` (53) — full root-`License.txt` coverage, zero gaps.** In `2D assets`, 5 zip-only dirs (`Isometric Miniature {Dungeon,Farm,Library,Overworld,Prototype}`) and in `Archive`, 10 zip-only dirs each embed a `License.txt` **inside the zip** (verified by `unzip -l`/`unzip -p`, standard Kenney CC0 text) — TRACED, not gaps. Kenney fonts inside licensed packs (`Icons/Input Prompts/*/Fonts/` 28 files, `UI assets/UI Pack*/Font{,s}/` 6 files) are covered by those packs' licenses. **Three genuine coverage gaps → §10.4**: `assets/Other/Fonts`, `assets/Other/Miniguides`, `assets/Archive/Isometric Renders`.

### 10.3 TRACED but redistribution-restricted — AlkaKrab music → QUARANTINE-RECOMMENDED (pending §8 item 1)

| paths | source doc | terms | status |
|---|---|---|---|
| `assets/audio/Tracks/` + `assets/audio/Loops/` (index casing; worktree lowercase — see the case-mismatch discrepancy in the audit addendum): 10 titles × {mp3,ogg,wav} × 2 dirs = 60 audio files, ~922.6 MB, added `54d10f736`, reachable from `origin/main` | `AlkaKrab Music License Info.pdf`, shipped in-pack in 3 byte-identical copies (md5 `40d1fcff12a01920ca9a5aef3c67ead4`); authorship corroborated by embedded metadata (ID3 `TPE1=alkakrab`, `TYER=2024`, Studio One encoder; Vorbis `ARTIST=alkakrab`) | Royalty-free, commercial use allowed, credit optional — **but**: *"No reselling, sublicensing, or redistribution of the track as-is. If you want to use these tracks in open-source games, contact me directly for a permission."* and prohibited: *"Redistribution or resale of the music file as-is without permission (e.g., in sound packs, music libraries)."* | **The current public-repo distribution appears to violate the license.** Never upload to packs/release page. Remove in the AD.6 rewrite unless written permission is obtained (§8 item 1). |

### 10.4 UNTRACEABLE → QUARANTINE-RECOMMENDED (sweep additions; never uploaded anywhere)

| paths | why untraceable | notes |
|---|---|---|
| `assets/audio/water_ambient_{mono,stereo_1,stereo_2}/` — 84 WAV, **2,195,628,728 B (2.196 GB)**, tracked (LFS), added `54d10f736` | No license doc in-pack or anywhere in repo; `assets/README.md` license notes silent; `scripts/validate_assets.ps1:252` *asserts* "third-party" without a source. Measured signature (non-evidence, recorded honestly): REAPER-originated `bext` chunks, origination dates 2019-08, 96 kHz/24-bit, pre-rename dirs `loadless_WATER_*` (`docs/current/ASSET_CLEANUP_REPORT.md:76-80`), naming scheme `<Mono\|Stereo> CATEGORY (descriptors).wav` | Largest unresolved cluster by bytes in the entire repo. Live-reachable via the editor audio scan (§10.1 note) but no curated consumer. Quarantine = also excluded from release page. |
| `assets/Other/Fonts/` — 16 TTF ("Kenney Blocks" … "Kenney Thick") + Preview.png + 2 webfont zips | No `License.txt` in dir, tree, zips (woff/css only), or any ancestor up to `assets/` — the only Kenney sub-tree with fonts and no license doc | All 16 are **orphaned-in-code** (zero loaders reference them; UI text renders via egui's embedded fonts, §10.5) |
| `assets/Other/Miniguides/` (incl. 4 SVGs: miniguide{1_tiles,2_character,4_background,5_ui}.svg) | No license doc anywhere in tree or ancestors — the dir's `Readme.txt` only points to Kenney YouTube tutorials, not a rights grant | |
| `assets/Archive/Isometric Renders/` — 16 zips of PNG sprites | Only Archive dir whose zips embed **no** `License.txt` (all 16 verified) | |
| `assets/Goodies/` (9 files: 2 PDFs, Thanks.txt, 6 wallpaper JPGs) + `assets/Visit Kenney.url` + `assets/Visit Kenney Bluesky.url` | `Thanks.txt` is a thank-you note, not a license (*"…token of appreciationn for your purchase…"* — no rights grant); "Kenney is usually CC0" is inadmissible per the standard | Low practical risk; standard applied honestly |
| `assets/textures/billboard_foliage_billboards_{albedo,alphamask,normal}.psd` — 3 × 50,331,688 B (150.9 MB), added `4c611964` | No source doc, no in-repo generator writes PSD. Measured: bare metadata-free raw-PSD headers whose byte math (26 + 12 + 2 + 3×4096²) matches file size exactly; albedo/alphamask pixel data opens all-zero — a pattern real Photoshop saves essentially never produce | Referenced only INDIRECTly via the Namaqualand blend-decomposition manifest (`assets/imported/Namaqualand/manifest.json`, `biome_pack.rs:235,352`) — they are textures *named inside* the .blend, not importer-authored |

### 10.5 TRACED — runtime UI fonts (egui embedded; not repo files)

No workspace code registers custom fonts (broad grep for `FontDefinitions|FontData|add_font|set_fonts|include_bytes!(*.ttf/otf)` → zero; `egui_extras` absent from the dependency tree). All UI text renders from `epaint_default_fonts` 0.32.3 (via egui/eframe/epaint 0.32.3, `default_fonts` feature on — root `Cargo.toml:185`); license files read from the local cargo-registry crate copy (`…\.cargo\registry\src\index.crates.io-…\epaint_default_fonts-0.32.3\fonts\`):

| font | role | license | evidence file (in-crate) |
|---|---|---|---|
| Ubuntu-Light | proportional primary (all body text) | Ubuntu Font Licence 1.0 | `UFL.txt` — *"This licence allows the licensed fonts to be used, studied, modified and redistributed freely."* |
| Hack-Regular | monospace primary | MIT (Source Foundry, 2018; bundles Bitstream Vera terms — moot, unmodified) | `Hack-Regular.txt` |
| NotoEmoji-Regular | emoji, first priority | SIL OFL 1.1 | `OFL.txt` |
| emoji-icon-font | emoji/icons, secondary | MIT (John Slegers, 2014) | `emoji-icon-font-mit-license.txt` |

Crate SPDX: `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0` (`Cargo.toml.orig:9`); `deny.toml:23` already allowlists the UFL component. **No distributed font has an unknown license**: the 2026-07-05 sweep counted 50 tracked in-repo TTF/OTF, all orphaned-in-code (34 covered by Kenney pack licenses, 16 → §10.4). This head tracks 53 (39 `.ttf`, 14 `.otf`): those 50 plus `docs/theme/parchment/fonts/Cinzel-Variable.ttf`, `EBGaramond-Variable.ttf`, and `EBGaramond-Italic-Variable.ttf`. `@font-face` loads those three, so they are not in the orphaned 34 or the §10.4 16; §16 licenses them (SIL OFL 1.1). The 11 woff2 in that sweep are untracked `docs/book/` build output (their local license files state Open Sans = Apache-2.0, Source Code Pro = OFL 1.1); `gh-pages/` ships zero fonts (system font stacks only); `git ls-files` at the 2026-07-05 sweep showed zero tracked woff/woff2/eot. §16 later tracks `docs/theme/parchment/fonts/IBMPlexMono-Regular.woff2` and `IBMPlexMono-Bold.woff2` (SIL OFL 1.1). Cosmetic note: `docs/benchmarks/index.html:60` references a `../fonts/…css` that does not exist (dangling, zero font bytes behind it).

### 10.6 Remaining sweep rows

- **`assets/textures/alps_field_2k.hdr`** (6,531,494 B) — Poly Haven `alps_field` verified via `https://api.polyhaven.com/info/alps_field` (HTTP 200, name "Alps Field", author Andreas Mischok) 2026-07-05, same standard as §3 → **TRACED, pack-eligible**. (The other three `assets/textures` 8K HDRs were already API-verified in AD.1/§9.)
- **`assets/audio/ambient/manifest.toml`** — first-party placeholder config; the 13 `.ogg` files it names do not exist and nothing parses the TOML (`astraweave-render/src/biome_audio.rs:32-45` hardcodes the same paths independently; its only non-test caller is a demo example). No asset to license.
- **`assets/Forest Scene`** — the sweep adds 6 WAV (262,118 B) plus census classes (.tif 53/362.2 MB, .mdb 64, .dll 114, .cube 6, .psd 48, .unitypackage 3, .pdf 1) to the existing AD.1 QUARANTINE-RECOMMENDED cluster; **`assets/Road to Vostok Assets Vol.1`** adds 39 PSD (1,229,911,141 B) to its existing TRACED-CC0 cluster; **KayKit** adds 1 PDF + 8 .url to its TRACED cluster. Cluster verdicts unchanged.
- **`docs/book/`** — gitignored mdBook build output (`.gitignore:13`), excluded from the provenance denominator (13 files: 11 woff2 + og-image.svg + favicon copies).
- **SVG class liveness**: no SVG decoder exists workspace-wide (no resvg/usvg/egui_extras anywhere in `Cargo.lock`); the class is asset-library payload only.

---

## 11. AD.1.B — pack-bucket remainder trace (2026-07-06)

Closes the AD.3.R G-4 provenance gap: four scopes that were pack-bucket by destiny but upload-blocked because their provenance was UNTRACED/MIXED. Standard = AD.1 §3 name-linked tier (API-verified slug + site CC0) and §2 SHA-256 Kenney-twin. Evidence: `docs/audits/evidence/ad1b_provenance_2026-07-06/` (per-slug PolyHaven/ambientCG API captures). Binary per family: TRACED or QUARANTINE — no "probably fine." Silent commit `54d10f736` (which introduced pine_forest, the loose textures, and the tests copies) carries **no** source-URL breadcrumb — its `scripts/generate_extended_materials.py` has none — so every trace here rests on slug-API identity or byte-twin, never on the commit.

### 11.1 TRACED — new Poly Haven slugs (name-linked, CC0, API-verified 2026-07-06)

All 200 at `https://api.polyhaven.com/info/<slug>`; CC0 per polyhaven.com/license.

- **pine_forest scope** (`assets/textures/pine_forest/`): `aerial_rocks_04` (Rob Tuytel), `evening_road_01_puresky` (HDRI; Jarod Guest/Sergej Majboroda), `fern_02`, `pine_bark` (Dimitrios Savva; collection `pine_forest`), `rock_moss_set_02` (Kless Gyzen; collection `pine_forest`), `moss`→`moss_01` (Rob Tuytel; trailing-`_01` obvious-variant — the one lower-confidence call, see §11.5). Plus the already-AD.1-verified §3 slugs present here (`rocky_trail`, `forest_ground_04`, `grass_medium_01`, `rock_moss_set_01`, `rock_face_03`, `ganges_river_pebbles`, `forrest_ground_01`, `forest_leaves_04`, `dry_branches_medium_01`, `namaqualand_cliff_02`).
- **textures-loose scope** (`assets/textures/*`): `boulder_01`, `brick_wall_04`, `damp_sand`, `dead_quiver_branch_01`, `dead_quiver_branch_02`, `flower_stinkkruid`, `metal_plate`, `namaqualand_boulder_04`, `namaqualand_boulders_01`, `namaqualand_cliff_01`, `rock_face_01`, `rock_face_2`→`rock_face_02`, `rock_face`, `rocks_ground_01`, `root_cluster_01`, `root_cluster_2`→`root_cluster_02`, `rubber_duck_toy`, `sand_rocks_small_01`, `single_root`, `tree_bark_03`, `wooden_stool_02` (21 new slugs / 75 files) — plus 47 already-AD.1-verified slug families (233 files) inherited name-linked by the loose copies.
- **models-loose scope** (`assets/models/`): `coast_sand_01_1k.glb` — `coast_sand_01` API-verified (Rob Tuytel, texture-type); the `.glb` is a Blender-baked wrapper of the CC0 texture set (glTF `generator: "Khronos glTF Blender I/O"`, embedded image names `coast_sand_01_diff_8k` etc.), same pattern as the traced `pine_tree_01_1k.glb`.

### 11.2 TRACED — ambientCG (CC0)

`assets/models/3DTreeStump001*` (6 files): `https://ambientcg.com/api/v2/full_json?id=3DTreeStump001` → `assetId 3DTreeStump001`, `dataType 3DModel`; local `*_SQ-1K-JPG*` names match the API's download-variant naming exactly. ambientCG site-wide CC0.

### 11.3 TRACED — Kenney byte-twins (SHA-256, CC0; §2 method)

- **models-loose**: 442 loose `.glb`/`.fbx` are byte-identical (SHA-256) to files inside CC0 Kenney packs — Nature Kit 320, Retro Medieval Kit 104, Blocky Characters 18 (evidence `models_twin_check.tsv`, 620-row hash table).
- **textures-loose non-slug tail**: `barrel.png`, `cobblestonePainted.png`, `details.png`, `fence.png`, `water.png` (5 files) — byte-identical to Kenney Retro Medieval Kit `Models/*/Textures/`.
- **assets/tests**: `assets/tests/textures/texture-{a..r}.png` (18 files) — Kenney **Blocky Characters** character textures; 16/18 byte-identical, 2/18 pixel-identical (lossless palette→RGBA re-encode, verified zero pixel diff) to `assets/3D assets/Blocky Characters/Models/*/Textures/`. (These 18 were AD.0-rejected as zero-reference; reference-check reconfirmed zero live consumers at HEAD — the only hits are dangling refs in dead orphan source `unified_showcase/src/main_bevy_v2.rs`. TRACED, so not purge-by-untraceability; they are redundant duplicates of a retained pack asset — pack under Blocky Characters CC0 or safe to drop as duplicates. Correct the `ASSET_REGISTRY.csv` "deliberately-corrupt fixture" label — none are corrupt.)

### 11.4 QUARANTINE-RECOMMENDED (untraceable; join the ratified quarantine destiny — never uploaded)

| scope | families | files | bytes | why |
|---|---|--:|--:|---|
| pine_forest | `dead_tree`, `dead_tree_tiled`, `fir_bark`, `fir_trunk_01`, `fir_trunk_03`, `fir_twig`, `pine_trunk_01/02/03`, `pine_twig`, `pine_cover_01`, `tree_trunk`, `tree_roots_01/02`, `montaigle_ruins_01/02/03` | 60 | 1,003,320,838 | slugs 404; names are word-substitutions of pine_forest models (not spelling/number/plural variants) → no name-link. `montaigle_ruins` is not a Poly Haven asset at all (zero index hits). Likely CC0 bakes of pine_forest models but uncertifiable under the standard. |
| textures-loose | `ivy` (404), `tiny_purple_succulant` (404), + non-slug tail `cobblestoneAlternative`, `roof`, `planks`, `tree`, `bee`, `square_alpha`, `transmission`, `ground_mask_01`, `LoL_diff`, `LoL_nor_gl` | 19 | 79,306,263 | 404 with non-Poly-Haven map-suffix conventions; the tail has no byte-twin to any licensed in-repo copy (same-name Kenney/import files differ by SHA). |
| models-loose | `house1..5.glb`; the primitive/greybox family (`cube4-9`, `cylinder1-5`, `cone1-4`, `sphere1-2`, `pillar1-4`, `wall corner*`, `door1-3`, `stairs*`, `fence edge/wood`, `ramp1`, `toggle switch`, `torus`, `arrow`, `box`, `coin`, `key`, …) | 81 | 1,742,733 | no same-basename Kenney twin (64 NO-CANDIDATE); the 12 base names that exist in Kenney kits DIFFER by SHA against every candidate (checked ≤11 kits each) — affirmatively rules out Kenney lineage. Zero production callers, zero license evidence, single bulk commit. |
| **total new quarantine** | | **160** | **1,084,369,834** | |

### 11.5 Confidence + open ratification calls

- The 68 already-AD.1-verified slugs and 449 byte-twins are **high confidence** (direct API identity / SHA-256). The 27 new API-200 slugs are name-linked (same tier AD.1 §3 ratified).
- **`moss`→`moss_01`** (pine_forest, 3 files / 10,718,929 B) was the single lower-confidence TRACED — name-linked only via the trailing-`_01` variant rule + `collection: pine_forest`. **RESOLVED 2026-07-06 (AD.1.C, §8 item 5): stays TRACED.** The director ratified the matching standard — a trailing-numeral variant name-link is admissible (`forrest_ground_01`/`moss_01`), a word-substitution is not (`pine_trunk`) — so `moss_01` is TRACED under the binding rule, not a discretionary call. (Under the rejected strict-exact alternative it would have moved 3 files to QUARANTINE, making pine_forest TRACED 70 / 1,664,068,578 and its quarantine 63 / 1,014,039,767.)
- Repo-side note (non-provenance): pine_forest `pine_trunk_02`≡`fir_trunk_01` and `pine_trunk_03`≡`fir_trunk_03` are byte-identical under divergent names (duplicated content); does not change the verdict.

---

## 12. AD.1.C — residual subdirectory trace + final partition (2026-07-06)

Closes the last untraced set: the 100-file subdirectory residual AD.1.B flagged out of scope inside `textures-environment` (`assets/textures/{pbr,Fabrics,grass_hd,models}`). Scope reconfirmed at HEAD (`git ls-files`): 56 + 24 + 5 + 15 = 100 tracked, of which **99 are LFS** (one `grass_hd` file is a plain blob). Paths in index casing (`Fabrics/`; the registry/worktree spell it `fabrics/` — the case-mismatch directory). Standard = AD.1 §3 name-linked (API-verified slug + site CC0) and §4 first-party generator-identity. Evidence: `docs/audits/evidence/ad1c_residual_2026-07-06/` (raw Poly Haven API captures). The ratified matching standard (§8 item 5) governs: trailing-numeral variant name-links admissible, word-substitutions not.

### 12.1 TRACED — first-party procedural (`assets/textures/pbr/`, 56 files)

`PBR_{2K,4K}/{Dirt_Mud,Moss_Ground,Sand_Desert,Stone_Terrain_Rock}/*.png` are **first-party procedurally-generated CC0** — full row + generator evidence in §4. Not name-linked; generator-identified (the strongest tier available for synthetic content). AD.1.B's provisional "ambientCG-style" read was wrong; the in-repo generator settles it.

### 12.2 TRACED — Poly Haven CC0 (`Fabrics/` 24 + `grass_hd/` 4 = 28 files)

- **`assets/textures/Fabrics/`** (24): three slugs, all API-200 (2026-07-06): `fabric_leather_01` (Rob Tuytel), `hessian_230` (colormass / Rico Cilliers), `rough_linen` (colormass / Rico Cilliers) — all `type:1` texture, CC0 per polyhaven.com/license. The two `*_1k.glb` are Poly Haven preview-mesh bundles inheriting the slug CC0 (same pattern as the traced `coast_sand_01_1k.glb`, §11.1).
- **`assets/textures/grass_hd/`** (4 LFS): `grass_medium_01` (API-200, `type:2`; already AD.1-verified §3.3/§11.1) — the `_alpha`/`_diff`/`_dry_diff`/`_rough` maps inherit it. The 5th tracked file `grass_hd/grass` is a **1-byte plain-blob stray** (a lone newline), not an asset and outside the LFS partition — recommend deletion; no license question.

### 12.3 QUARANTINE-RECOMMENDED → RATIFIED (`assets/textures/models/houses/`, 15 files / 25,879,389 B)

`house1..5_tex{1,2,3}.png` are the textures for `house1..5.glb`, which AD.1.B QUARANTINED (no Kenney byte-twin; DIFFER hashes affirmatively rule out Kenney lineage). Their textures are equally untraceable — no slug, no source doc, single bulk commit `54d10f736`. Join the ratified quarantine (§8 item 1, untraceable class).

### 12.4 Roll-up + `textures-environment` status

Of the 99 LFS residual files: **84 TRACED / 226,760,432 B** (56 pbr first-party + 24 Fabrics + 4 grass_hd) · **15 QUARANTINE / 25,879,389 B** (houses). Net partition move: **15 files / 25,879,389 B pack → quarantine**; the 84 stay pack-eligible. After AD.1.C the `textures-environment` tree (`assets/textures/` minus `pine_forest`, 446 LFS files / 4,998,449,523 B) has **zero untraced files** — its only exclusions are ratified quarantine (19 loose-tail from §11.4 + 15 houses = 34 files / 105,185,652 B). The pack is now fully classified and upload-clear for its TRACED portion (~4.56 GiB, ~3 zips at the ratified name-prefix split, G-3); the blocker AD.1.B flagged is cleared.

### 12.5 Final five-bucket partition (gate-unclassified emptied to zero)

After the AD.1.C bucket moves — Half-1 houses (pack→quarantine, 15), G-2 `archive/`+`assets/cache/impostors/` (gate→quarantine, 71), `assets/tests/` (pack→quarantine, 18, redundant-duplicate), and `assets_src/` (gate→pack `materials-src`, 66) — the partition is:

| bucket | files | bytes |
|---|--:|--:|
| pack | 90,031 | 20,547,244,167 |
| quarantine | 1,094 | 7,481,318,328 |
| gate-unclassified | **0** | **0** |
| sample | 114 | 182,607,234 |
| retained | 54 | 32,049,883 |
| **total (invariant)** | **91,293** | **28,243,219,612** |

The gate bucket (137 / 791,711,859 B) was composed exactly of `archive/` (4 / 67,109,616) + `assets/cache/impostors/` (67 / 4,751,884) + `assets_src/` (66 / 719,850,359); decisions §8 items 1+4 dispose all three to the last byte. Cross-foot verified against the full-history LFS enumeration this session. (The 3 `assets_src/environments/*.toml` and the `grass_hd/grass` stray are plain blobs outside the 91,293-path LFS denominator and unaffected.)

---

## 13. AD.4 — sample-slot re-cook + C7 re-acquisition (2026-07-07)

Refills the 39 quarantine-destined sample slots (AD.4.R) so AD.6's path purge does not strand a ratified sample row. Cook tool: `tools/material_cook/cook_1k.py` (1024² RGBA PNG × {albedo,normal,mra}; contract-tested). Quarantine-destined texture slots get **new paths** (`assets/materials/derived_1k/`); the 37 old occupants are untouched and leave in AD.6's path purge (overwriting them would trap unlicensed blobs in surviving-path history). Evidence: `docs/audits/evidence/ad4_c7_reacquire_2026-07-07/`.

### 13.1 TRACED — C7 re-acquisition (six §6-untraceable families re-sourced, CC0 Poly Haven, API-verified 2026-07-07)

Site license: <https://polyhaven.com/license> (CC0). Each slug director-ratified 2026-07-07; API captures in the evidence dir. Cooked to `assets/materials/derived_1k/<family>{,_n,_mra}.png` (albedo + normal downscaled; mra packed R=metallic(0)/G=roughness/B=AO from the acquired rough+ao maps).

| slot family | Poly Haven slug | name / author (API) | type | acquired dims |
|---|---|---|---|---|
| `cloth` | `fabric_leather_01` | Fabric Leather 01 / Rob Tuytel | 1 (texture) | 2048² |
| `plaster` | `plastered_wall_02` | Plastered Wall 02 / Charlotte Baglioni | 1 | 2048² |
| `rock_lichen` | `lichen_rock` | Lichen Rock / Rico Cilliers | 1 | 2048² |
| `roof_tile` | `roof_tiles_14` | Roof Tiles 14 / Rob Tuytel | 1 | 2048² |
| `tree_bark` | `tree_bark_03` | Tree Bark 03 / Rob Tuytel | 1 | 2048² |
| `tree_leaves` | `forest_leaves_02` | Forest Leaves 02 / Rob Tuytel | 1 | 2048² |

Acquired via `tools/astraweave-assets` (plain-HTTP Poly Haven Files API; no LFS traffic) to `assets/_downloaded/polyhaven/<family>/` (gitignored). Manifest entries: `assets/asset_manifest.toml` (AD.4 C7 block). **Ratification note:** the swap was director-ratified; these rows are the *provenance* trace (ratification authorizes the substitution, it is not itself provenance). `rock_moss_set_02` was **REJECTED** (type-2 model pack, not a tileable texture); `lichen_rock` was selected search-first (type-1 texture, exact semantic match).

### 13.2 C6 + traced-9 derivatives (cooked from already-traced sources)

- **C6** (`cobblestone, gravel, ice, metal_rusted, moss, wood_planks`): cooked from the **traced `assets_src/materials/` sources** (§5, AD.1-verified upstreams) → `derived_1k/`. Provenance = §5 (unchanged); the derivative inherits it.
- **traced-9** (`grass, forest_floor, mountain_rock, mud, stone, rock_slate, dirt` §3.3 + `sand, snow` §4): re-cooked **in place** (licensed history) to 1024². Provenance = §3.3/§4 (unchanged).

### 13.3 `assets/textures/cobblestone.png` — substitute (re-pointed)

The §6 unlicensed `assets/textures/cobblestone.png` is retired by **re-pointing its sole consumer** (`examples/unified_showcase/src/main.rs:1130`, TowerStone ground) to the re-cooked C6 `assets/materials/derived_1k/cobblestone.png` (traced). The old file is untouched (leaves in AD.6).

### 13.4 Debris rows retired

`assets/imported/verdant_trail/meshes/{sticks,grass}_debris_a.glb` (§5): originals are unpublished Poly Haven sub-assets (un-re-acquirable). Director-ratified **quarantine-and-accept** — the two rows retire from the sample set (**153 → 151**; this is the AD.0 sample-**row** manifest count, distinct from the LFS sample-**file** bucket in §13.5 which moves 114 → 150); scatter thins gracefully (`astraweave-terrain/src/biome.rs:1052,1065` skip missing via `engine_adapter.rs:2578`). Files untouched (already quarantine; leave in AD.6).

### 13.5 Bucket bookkeeping — the 91,293 invariant retires

AD.4 adds **36 new LFS paths** (`derived_1k/` 18 C6 + 18 C7); nothing is deleted (old occupants leave in AD.6). The 18 C7 `assets_src` copies (untraceable) move **pack → quarantine** (excluded from `materials-src`). Successor partition:

| bucket | AD.1.C | AD.4 | delta |
|---|--:|--:|---|
| pack | 90,031 | 90,013 | −18 (C7 assets_src → quar) |
| quarantine | 1,094 | 1,112 | +18 |
| gate-unclassified | 0 | 0 | — |
| sample | 114 | 150 | +36 (derived_1k) |
| retained | 54 | 54 | — |
| **total** | **91,293** | **91,329** | **+36** |

Successor denominator **91,329 = 91,293 + 36** (foots). The `materials-src` pack (48 files / 615,202,592 B = `assets_src` 66 − 18 C7) is cut as AD.4's closing step — **now cut and live** on the `assets-v1` release: zip **570,810,569 B**, sha256 `652458f3044ca16f71291e9639713ac4a8295d13e724fb3f3112434c5be3108d`, `https://github.com/lazyxeon/AstraWeave/releases/download/assets-v1/materials-src.zip`; deterministic build (byte-identical rebuild) + local 3-run loop + live-URL loop all verified 2026-07-07 (full evidence, per-slot table, and open items: `docs/audits/AD4_RECOOK_OUTCOME.md`).

## 14. T.1 — distinct beach material acquisition (2026-07-21)

Executes the T-series ratification §2 **row-6 amendment** (director, 2026-07-20 — `docs/audits/T_SERIES_RATIFICATION_2026-07-20.md`): slot 6 (beach) of `assets/materials/biomes/` no longer shares the desert `sand.png` trio; it gets its own traced material. Evidence: `docs/audits/evidence/t1_beach_2026-07-21/` (raw `/info` + `/files` API captures). Site license: <https://polyhaven.com/license> (CC0).

| slot family | Poly Haven slug | name / author (API) | type | acquired dims |
|---|---|---|---|---|
| `beach` | `coast_sand_01` | Coast Sand 01 / Rob Tuytel | 1 (texture) | 2048² |

API-verified live 2026-07-21 (`api.polyhaven.com/info/coast_sand_01`: `type:1`, `authors {"Rob Tuytel":"All"}`, `max_resolution [8192,8192]`; description "damp coastal sand with brown, rough grain, scattered pebbles and gravel"). Prior related trace: §11.1's `coast_sand_01_1k.glb` (the Blender-baked wrapper in `assets/models/`) — that row traced the *model wrapper*; this row is the fresh provenance for the *texture-set acquisition* (a prior trace authorizes nothing by itself). Acquired via `tools/astraweave-assets fetch` (plain-HTTP Files API, no LFS traffic) → `assets/_downloaded/polyhaven/beach/beach_{albedo,normal,roughness,ao,arm}.png` (gitignored; manifest-pinned in `assets/asset_manifest.toml` T.1 block). Cooked to `assets/materials/derived_1k/beach{,_n,_mra}.png` — outcome doc: `docs/audits/T1_BEACH_OUTCOME.md`.

## 15. T.2c — real PBR materials for the three synthetic biome slots (2026-07-25)

Executes the director's T.2a-gate observation (2026-07-25): slots 0/1/4 of `assets/materials/biomes/` "read as flat and shiny … they definitely don't read as proper PBR materials". Slot 0 was a synthetic flat-green albedo paired with the normal map of an **alpha-cutout foliage card** (`grass_medium_01`), whose transparent regions shaded as hard black shards; slots 1 and 4 were 100% procedural (`tools/pbr_gen`) with effectively flat normals. Diagnosis: `docs/audits/T2A_OUTCOME.md` §3.1–3.2. Evidence: `docs/audits/evidence/t2c_materials_2026-07-25/` (raw `/info` + `/files` API captures, plus a capture of the site licence page). Site license: <https://polyhaven.com/license> (CC0 — "All assets … are all licensed as CC0").

| slot | Poly Haven slug | name / author (API) | type | scan dims | acquired |
|---|---|---|---|---|---|
| 0 grassland | `aerial_grass_rock` | Aerial Grass Rock / Rob Tuytel | 1 (texture) | 15.0 × 15.0 m | 2048² |
| 1 desert | `sand_01` | Sand 01 / Rob Tuytel | 1 (texture) | 1.5 × 1.5 m | 2048² |
| 4 tundra | `snow_02` | Snow 02 / Rob Tuytel | 1 (texture) | 2.0 × 2.0 m | 2048² |

All three API-verified live 2026-07-25 (`api.polyhaven.com/info/<slug>` + `/files`): `type:1` texture, `max_resolution [8192,8192]`, each shipping Diffuse + `nor_gl` + Rough + AO at 1k–8k. Each acquisition carries its **own fresh row** — no prior trace of any of these slugs is relied upon. Note `snow_02` is a *different* slug from the pre-existing `snow` manifest handle (which pins `snow_03`); the T.2c entries use distinct handles (`grassland`, `desert_sand`, `tundra_snow`) so no existing manifest row is disturbed.

Acquired via `tools/astraweave-assets fetch` (provider flow, session manifest mirroring the permanent `assets/asset_manifest.toml` T.2c block) → `assets/_downloaded/polyhaven/{grassland,desert_sand,tundra_snow}/*_{albedo,normal,roughness,ao,arm}.png` (gitignored). Cooked via `cook_1k.py::cook_family_from_maps` (rough+ao path — the ARM-order trap avoided by construction; the fetched `_arm.png` files are deliberately unused) to `assets/materials/derived_1k/{grass,sand,snow}{,_n,_mra}.png`. Outcome doc: `docs/audits/T2C_OUTCOME.md`.

**Attribution merge verified, not assumed:** the `generate_attribution_file` overwrite bug that fired in AD.4 and again in T.1 did **not** fire this time (the 5.C fix holds) — `assets/_downloaded/polyhaven/ATTRIBUTION.txt` went 18 → 21 slugs with **zero** lost entries (set-difference check against a pre-fetch baseline copy).

**Also evaluated and rejected, recorded so the reasoning is not re-derived:** `leafy_grass` (2 m, real detail — but its shipped AO map is a hard constant 255, modal 100%, i.e. a flat placeholder, and its tone duplicates slot 2's leaf litter); `snow_04` (4.0 m, exact 1:1 tiling and the highest variance of any candidate — but it is a **plowed field** whose furrows would tile as directional stripes); `aerial_sand` (tags `costal`/`seaside`/`ocean`, which would undo the T.1 desert≠beach distinction); `sparse_grass` (too dark, R79/G61/B21, for a surface covering 91% of a world); `grass_path_2`/`_3` (a path runs through the scan — it would repeat as a visible stripe every 4 m).

## 16. Candlelit grimoire display faces (2026-11-25)

Owner exception, 2026-11-25: these three families are SIL Open Font License 1.1, not CC0. They are display faces for the architecture map and the benchmark dashboard. They are not engine assets. `@font-face` loads only the files under `docs/theme/parchment/fonts/`. No remote font fetch.

| paths | upstream | license | attribution | evidence | date |
|---|---|---|---|---|---|
| `docs/theme/parchment/fonts/Cinzel-Variable.ttf` | The Cinzel Project Authors, <https://github.com/NDISCOVER/Cinzel>. Bytes match `ofl/cinzel/Cinzel[wght].ttf` on <https://github.com/google/fonts> `main`, fetched 2026-11-26. SHA-256 `f4d83d34d1f6c741193e4acf4b3dff9531e5a67b6aa65228d00a7db72a4e0f34`. | SIL Open Font License 1.1. The vendored copyright line names no reserved font name. | required (OFL notice kept) | `docs/theme/parchment/licenses/OFL-Cinzel.txt` | 2026-11-25 |
| `docs/theme/parchment/fonts/EBGaramond-Variable.ttf` | The EB Garamond Project Authors, <https://github.com/octaviopardo/EBGaramond12>. Bytes match `ofl/ebgaramond/EBGaramond[wght].ttf` on <https://github.com/google/fonts> `main`, fetched 2026-11-26. SHA-256 `ef9512f92f6d579e5dc75af59a5a4b1b8b47d2eda89e00b954d44520e5369027`. | SIL Open Font License 1.1 | required (OFL notice kept) | `docs/theme/parchment/licenses/OFL-EBGaramond.txt` | 2026-11-25 |
| `docs/theme/parchment/fonts/EBGaramond-Italic-Variable.ttf` | Same project. Bytes match `ofl/ebgaramond/EBGaramond-Italic[wght].ttf` on <https://github.com/google/fonts> `main`, fetched 2026-11-26. SHA-256 `bba2c4499c93c9612b90b9825d32b07da52fce2fe57562a1eb6b833553f93c4e`. | SIL Open Font License 1.1 | required (OFL notice kept) | `docs/theme/parchment/licenses/OFL-EBGaramond.txt` | 2026-11-25 |
| `docs/theme/parchment/fonts/IBMPlexMono-Regular.woff2` | IBM Corp., <https://github.com/IBM/plex>. Bytes match `packages/plex-mono/fonts/complete/woff2/IBMPlexMono-Regular.woff2` on `master`, fetched 2026-11-26. SHA-256 `ba204497f16b6d334cee9d1e963a831b73e3a56e1d6300a8489d18df7214b350`. | SIL Open Font License 1.1, with Reserved Font Name "Plex" | required (OFL notice kept; the reserved name is "Plex") | `docs/theme/parchment/licenses/OFL-IBMPlexMono.txt` | 2026-11-25 |
| `docs/theme/parchment/fonts/IBMPlexMono-Bold.woff2` | Same project. Bytes match `packages/plex-mono/fonts/complete/woff2/IBMPlexMono-Bold.woff2` on `master`, fetched 2026-11-26. SHA-256 `ea576f38d05cc44cca48c45314984beb8cc1d2b886f58e1dce99f15dc344eb1d`. | SIL Open Font License 1.1, with Reserved Font Name "Plex" | required (OFL notice kept; the reserved name is "Plex") | `docs/theme/parchment/licenses/OFL-IBMPlexMono.txt` | 2026-11-25 |
