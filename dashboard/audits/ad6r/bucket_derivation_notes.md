# AD.6 KEEP/QUARANTINE Partition — Derivation Notes

**Input:** `d:/tmp/ad6r/lfs_nonpack_head.txt` — 1,316 LFS-routed paths at HEAD that are **not** members of the 19 committed packlists (pack bucket = 90,013, handled separately).

**Output partition (cross-foots EXACTLY):**

| bucket | count | target | status |
|---|--:|--:|---|
| SAMPLE | 150 | 150 | ✓ |
| RETAINED | 54 | 54 | ✓ |
| QUARANTINE | 1,112 | 1,112 | ✓ |
| **TOTAL** | **1,316** | **1,316** | ✓ |

**Verification (see §Verification):** three files disjoint (0 cross-bucket duplicates); union == input exactly (0 diff); QUARANTINE has **0 overlap with the 90,013 pack members** (independent confirmation).

The authoritative denominators are ratified in **`AD4_RECOOK_OUTCOME.md` §7** and **`THIRD_PARTY_LICENSES.md` §13.5 / `DISPOSITION_REPORT_imported.md` §10** (post-AD.4 successor: sample 150, retained 54, quarantine 1,112, total 91,329; the 1,316 here are exactly these three non-pack buckets).

---

## 1. SAMPLE (150) — the AD.0 ratified sample set's current LFS backing files

Source of truth: `SAMPLE_SET_PROPOSAL.md` tables A–F, H, I (153 rows), reconciled with `LFS_REMOVAL_PLAN.md` §4 (row-extension histogram) and the AD.4 re-cook (`AD4_RECOOK_OUTCOME.md` §2, §7).

**Row → file reconciliation (why 153 rows → 150 files):**
`LFS_REMOVAL_PLAN.md` §4: 153 sample **rows** = 69 png + 63 glb + 15 gltf + 4 hdr + 2 jpg.
- −2 debris glb retired (`{grass,sticks}_debris_a.glb`; AD.4 §2c, §10) → 151 rows.
- Table-H materials (63 root PNGs): 27 traced-9 stay **in place** as sample; 36 C6/C7 old occupants leave sample (→ quarantine) and are **replaced by 36 `derived_1k/` PNGs** (AD.4 §2a/§2b/§2d). Net png sample unchanged in count.
- `assets/textures/cobblestone.png` **retired** from sample, re-pointed to `derived_1k/cobblestone.png` (AD.4 §2c, TPL §13.3) → −1 png.
- Net: 151 rows → **150 files** (68 png + 61 glb + 15 gltf + 4 hdr + 2 jpg).

| sub-cluster | count | bucket | citation |
|---|--:|---|---|
| `assets/materials/derived_1k/*.png` (12 C6/C7 families × 3) | 36 | SAMPLE | `AD4_RECOOK_OUTCOME.md` §2a/§2b/§7 (+36 derived_1k); TPL §13.5 |
| `assets/materials/{grass,forest_floor,mountain_rock,mud,stone,rock_slate,dirt,sand,snow}{,_n,_mra}.png` (traced-9, in-place re-cook) | 27 | SAMPLE | `AD4_RECOOK_OUTCOME.md` §2d (27 files, licensed history, re-cooked in place); `SAMPLE_SET_PROPOSAL.md` Table H |
| Table A — editor-spawn GLBs (KayKit + Kenney) | 11 | SAMPLE | `SAMPLE_SET_PROPOSAL.md` Table A-editor-spawn |
| Table B — debug-props (`barrels.glb`, `bed.glb`) | 2 | SAMPLE | Table B-debug-props |
| Table C — example model GLBs | 7 | SAMPLE | Table C-examples |
| Table C — jpg (`grass_bermuda_01_diff_1k.jpg`, `assets/Astraweave_logo.jpg`) | 2 | SAMPLE | Table C-examples; LFS_REMOVAL_PLAN §4 (2 jpg sample rows) |
| Table D — HDRIs | 4 | SAMPLE | Table D-hdri |
| Table E — scatter GLBs (Nature Kit 18 + verdant_trail 22 + models/grass 1) | 41 | SAMPLE | Table E-scatter (minus 2 debris_a retired, AD.4 §2c) |
| Table E — KayKit Forest scatter gltf | 9 | SAMPLE | Table E-scatter |
| Table F — greybox gltf | 6 | SAMPLE | Table F-greybox |
| Table I — closure textures (colormap ×3 + forest_texture ×2) | 5 | SAMPLE | Table I-closure-textures |
| **SAMPLE total** | **150** | | |

Every sample file is under `assets/` and present in the input (verified: 0 missing, 0 outside `assets/`).

---

## 2. RETAINED (54) — retained-non-sample micro-bucket

Source of truth: **`AD3R_PACK_PARTITION_PROPOSAL.md` Phase 2** (the recon that constituted this bucket; ratified count "retained 54" carried unchanged through `DISPOSITION_REPORT_imported.md` §8/§9.3, `AD3_RELEASE_OUTCOME.md` §2, `AD4_RECOOK_OUTCOME.md` §7). Constitution = every LFS-routed HEAD file **outside `assets/`** that is not `assets_src/` (quarantine) or `archive/` (quarantine).

| dir | count | bucket | citation |
|---|--:|---|---|
| `astraweave-audio/tests/{assets,fixtures}/*` (first-party test audio) | 16 | RETAINED | AD3R Phase 2 ("astraweave-audio/ 16 first-party test audio") |
| `examples/` (fluids_demo captures 10 + hello_companion 3 + veilweaver_demo 1) | 14 | RETAINED | AD3R Phase 2; `DISPOSITION_REPORT_imported.md` §9.2 item 7 (fluids_demo live → captures retained) |
| `docs/` (3 artlist evidence + docs/branding logo + docs/Veilweaver + 6 screenshots) | 11 | RETAINED | AD3R Phase 2 ("docs/ 11") |
| `gh-pages/` (assets logo + graphs 3) | 4 | RETAINED | AD3R Phase 2 ("gh-pages/ 4") |
| `astraweave-render/tests/visual_regression/golden/*` | 3 | RETAINED | AD3R Phase 2 ("astraweave-render/ 3") |
| `tools/benchmark-dashboard/graphs/*` | 3 | RETAINED | AD3R Phase 2 ("tools/ 3") |
| repo-root editor screenshots (`editor_ss.png`, `editor_screenshot.{png,jpg}`) | 3 | RETAINED | AD3R Phase 2 ("3 repo-root editor screenshots") |
| **RETAINED total** | **54** | | |

**Reconciliation of the AD3R off-by-one (documented, not ungrounded):** AD3R Phase 2's prose enumeration *textually* also lists `assets/Astraweave_logo.jpg`, which would sum its list to 55. But `assets/Astraweave_logo.jpg` is a **sample row** (`SAMPLE_SET_PROPOSAL.md` Table C; `LFS_REMOVAL_PLAN.md` §4's 2 jpg sample rows). Placing it in SAMPLE (as done here) makes RETAINED = 54 exactly. Note the three distinct logo files: `assets/Astraweave_logo.jpg` (SAMPLE), `docs/branding/Astraweave_logo.jpg` (RETAINED, "logo derivative"), `gh-pages/assets/Astraweave_logo.jpg` (RETAINED) — all correctly separated.

---

## 3. QUARANTINE (1,112) — ratified untraceable + hygiene + redundant-duplicate + AD.4 moves

Ratified constitution: `DISPOSITION_REPORT_imported.md` §9.2 item 1 (untraceable 1,005 = 830 AD.1 + 160 AD.1.B + 15 AD.1.C; hygiene 71; redundant-duplicate 18 = **1,094**) **+18** C7 `assets_src` copies (AD.4 §7 / DISPOSITION §10) = **1,112**.

By construction: QUARANTINE = (all `assets/` in input − 150 sample) + `assets_src/` 18 + `archive/` 4 = 1,090 + 22 = 1,112.

| cluster | count | bucket | citation |
|---|--:|---|---|
| `assets/Forest Scene/**` (all LFS types: mat 96, tga 69, tif 53, prefab 49, png 47, asset 38, psd 17, fbx 17, wav 6, FBX 6, obj 5, jpg 4, pdf 1, flare 1) | 409 | QUAR | `DISPOSITION_REPORT_imported.md` §6 (Forest Scene scene art UNTRACEABLE); TPL §10.6 (census extends AD.1 cluster); ratified §9.2. **COUNT NOTE below.** |
| `assets/models/Amber-Npc/**` | 158 | QUAR | `DISPOSITION_REPORT_imported.md` §6 (UNTRACEABLE, elevated risk — CC-export, non-redistributable) |
| `assets/models/` loose (`house1..5.glb` + primitive/greybox family) | 81 | QUAR | TPL §11.4 models-loose (81 QUARANTINE); DISPOSITION §8 (AD.1.B) |
| `assets/audio/water_ambient_{mono 42, stereo_1 26, stereo_2 16}/*.wav` | 84 | QUAR | TPL §11 line 146 (AD.1.A sweep RATIFIED) + §10.4 line 187 (84 WAV) |
| `assets/audio/{Loops 32, Tracks 31}/` — AlkaKrab (20 mp3 + 20 ogg + 20 wav + 3 license pdf) | 63 | QUAR | TPL §11 line 145 (option a QUARANTINE) + §10.3 line 181. **COUNT NOTE below.** |
| `assets/textures/pine_forest/` untraceable | 60 | QUAR | TPL §11.4 pine_forest (60 QUARANTINE); DISPOSITION §8 |
| `assets/cache/impostors/*` (runtime cache tracked by mistake) | 67 | QUAR | DISPOSITION §9.2 item 1 (hygiene 71: impostors 67); AD3R G-2 |
| `assets/materials/*.png` — 12 C6/C7 old occupants × 3 maps | 36 | QUAR | `AD4_RECOOK_OUTCOME.md` §2a/§2b/§7 (replaced by derived_1k); DISPOSITION §10 ("37 old occupants leave in AD.6" = these 36 + cobblestone) |
| `assets_src/materials/*.png` — 18 untraceable C7 copies | 18 | QUAR | `AD4_RECOOK_OUTCOME.md` §6/§7 (18 C7 assets_src pack→quarantine); DISPOSITION §10 |
| `assets/tests/textures/texture-{a..r}.png` (redundant Kenney dup) | 18 | QUAR | DISPOSITION §9.2 item 1 (redundant-duplicate 18); TPL §11.3 |
| `assets/textures/` loose-19 (ivy 5 + tiny_purple_succulant 4 + non-slug tail 10) | 19 | QUAR | TPL §11.4 textures-loose (19 QUARANTINE, exact family list) |
| `assets/textures/models/houses/*` | 15 | QUAR | TPL §12.3 (15 QUARANTINE ratified) |
| `assets/imported/Namaqualand/` fine_leaf (3 glb + 6 tex) | 9 | QUAR | DISPOSITION §1 (fine_leaf_01 exception, QUARANTINE) |
| `assets/imported/verdant_trail/meshes/` (dirt_bank ×2 + debris ×6) | 8 | QUAR | DISPOSITION §2 (dirt_bank/sticks_debris/grass_debris); AD.4 §2c (2 debris_a retired, sample 153→151) |
| `assets/Other/` (Fonts/Preview.png 1 + Miniguides 15) | 16 | QUAR | TPL §11 line 146 (AD.1.A RATIFIED: Other/Fonts + Other/Miniguides) + §10.4 line 189 |
| `assets/textures/` fine_leaf_01_* loose (ao/diff/mask/norm_gl/rough/translucency) | 6 | QUAR | DISPOSITION §1 (fine_leaf_01 textures QUARANTINE — family rule) |
| `assets/Texture/*` (8) | 8 | QUAR | DISPOSITION §6 (UNTRACEABLE) |
| `assets/Goodies/` (6 wallpaper jpg + 2 pdf) | 8 | QUAR | TPL §11 line 146 (AD.1.A RATIFIED) + §10.4 line 191 |
| `assets/root Albedo-set` (Albedo/AO/Displacement/Gloss/Normal/Roughness.jpg + Displacement.exr) | 7 | QUAR | DISPOSITION §6 (UNTRACEABLE root PBR-set) |
| `assets/castles_forts_asset_pack/*` (5) | 5 | QUAR | DISPOSITION §6 (UNTRACEABLE) |
| `assets/Symphonie/*` (5) | 5 | QUAR | DISPOSITION §6 (UNTRACEABLE) |
| `archive/test_outputs/*` (build debris ktx2) | 4 | QUAR | DISPOSITION §9.2 item 1 (hygiene 71: archive 4); AD3R G-2 |
| `assets/textures/billboard_foliage_billboards_*.psd` (3) | 3 | QUAR | TPL §11 line 146 (AD.1.A RATIFIED) + §11.4 line 192 |
| `assets/Mesh/*` (2) | 2 | QUAR | DISPOSITION §6 (UNTRACEABLE) |
| `assets/{Namaqualand,verdant_trail}.blend` (2 monoliths) | 2 | QUAR | DISPOSITION §3 (root .blend monoliths QUARANTINE-RECOMMENDED, ratified) |
| `assets/textures/cobblestone.png` (unlicensed, retired) | 1 | QUAR | TPL §6 + §13.3; AD.4 §2c (re-pointed to derived_1k) |
| **QUARANTINE total** | **1,112** | | |

---

## 4. Ungrounded / gate items and discrepancies

**Zero ungrounded assignments.** Every one of the 1,316 files maps to a cluster grounded in a **committed** ratified doc (`THIRD_PARTY_LICENSES.md`, `DISPOSITION_REPORT_imported.md`, `AD4_RECOOK_OUTCOME.md`, `LFS_REMOVAL_PLAN.md`, `SAMPLE_SET_PROPOSAL.md`). `AD3R_PACK_PARTITION_PROPOSAL.md` (uncommitted, `d:/tmp`) supplied the retained-54 *constitution*, but every retained/quarantine cluster it names is independently ratified in a committed doc (retained via DISPOSITION §9.3 carry-through; water_ambient/Goodies/Other/Miniguides via TPL §11 line 146 AD.1.A-RATIFIED).

The following are **count-reconciliation notes**, not bucket ambiguities — each cluster's disposition is unambiguous:

1. **Forest Scene count granularity (409 vs "150").** `DISPOSITION_REPORT_imported.md` §6 says "assets/Forest Scene/ scene art (150 tracked files)". The actual LFS-routed, non-pack HEAD population is **409** (extension census above). The doc's "150" was a scene-art subset; the LFS partition captures the full binary tree (mat/tga/tif/prefab/png/asset/psd/fbx/wav/obj/jpg/pdf/flare). The **entire** Forest Scene tree is wholesale UNTRACEABLE (only Unity SDK `Library/` licenses exist), so all 409 → QUARANTINE with no ambiguity. `.dll`/`.mdb`/`.cube`/`.unitypackage` under Forest Scene are non-LFS plain blobs (outside this accounting; removed by the AD.6 path-purge with the cluster).

2. **AlkaKrab 63 vs 60.** TPL §10.3 counts "60 audio files"; the cluster has **63** = 60 audio (20 mp3 + 20 ogg + 20 wav) + **3 byte-identical `AlkaKrab Music License Info.pdf` copies** shipped in the Loops/Tracks dirs (TPL §10.3 line 181 notes the 3 license copies). PDFs are LFS-routed; all 63 → QUARANTINE.

3. **AD3R retained off-by-one (Astraweave_logo).** Resolved in §2 above — `assets/Astraweave_logo.jpg` is SAMPLE (Table C / LFS_REMOVAL_PLAN §4); AD3R's retained list minus it = 54.

4. **fine_leaf loose textures (6) grounded by family rule.** `assets/textures/fine_leaf_01_*.png` (6 loose copies, distinct from the 6 Namaqualand imported copies) are grounded via `DISPOSITION_REPORT_imported.md` §1's family disposition ("fine_leaf_01 meshes **+ textures** → QUARANTINE"), not by an individually-enumerated path. Low-risk: `fine_leaf_01` is a 404 slug (untraceable) everywhere it appears; both the imported and loose copies are the same uncertifiable sub-asset.

5. **loose-texture non-slug tail (billboard psd, cobblestoneAlternative, roof, tree, square_alpha, etc.).** All fall inside TPL §11.4's exact 19-file textures-loose list (or the AD.1.A ratified billboard psd line 146/192). No file relies on an "etc." — the §11.4 table enumerates every family.

---

## 5. Verification (all PASS)

- `wc -l`: sample 150 · retained 54 · quarantine 1,112 · sum **1,316** ✓
- Pairwise intersections (sample∩retained, sample∩quar, retained∩quar): **0, 0, 0** ✓
- Cross-bucket duplicate lines: **0** ✓
- Union (sort -u of all three) vs `lfs_nonpack_head.txt` (`comm -3`): **0 diff** ✓
- Every sample file starts with `assets/` and ∈ input: **0 violations** ✓
- QUARANTINE ∩ the 90,013 committed pack members (`comm -12`): **0 overlap** ✓ (independent confirmation that no quarantine path is a pack member)
- Sample `assets/materials/*.png` quarantine set = exactly the 12 C6/C7 families (cloth, cobblestone, gravel, ice, metal_rusted, moss, plaster, rock_lichen, roof_tile, tree_bark, tree_leaves, wood_planks); zero traced-9 leakage ✓

**Files written:** `d:/tmp/ad6r/bucket_sample.txt` (150), `d:/tmp/ad6r/bucket_retained.txt` (54), `d:/tmp/ad6r/bucket_quarantine.txt` (1,112), this notes file.
