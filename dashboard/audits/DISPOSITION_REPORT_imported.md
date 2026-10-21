# Disposition Report — `assets/imported/` (9.0 GB) + Road to Vostok Vol.1 (+ adjacent unknowns)

**Date**: 2026-07-05 (AD.1); §8 added AD.1.B 2026-07-06; §9 + ratification AD.1.C 2026-07-06 | **Method**: clusters traced against upstream license documents fetched this session (Poly Haven API `api.polyhaven.com/info/<slug>` + <https://polyhaven.com/license>; itch.io + Wayback Machine for RtV). No git-lfs traffic; all file inspection used the local checkout. Tracing standard per AD.1: a cluster is TRACED only when a source document states license terms; no inference. **Quarantine applies to the release page as well as the repo** — quarantined clusters are never uploaded anywhere.

> **RATIFIED (2026-07-06, AD.1.C — see §9):** every "QUARANTINE-RECOMMENDED" verdict in §1–§6 below is now **QUARANTINE (RATIFIED)** — the director ratified the full quarantine list (destiny: purged in the AD.6 rewrite, never uploaded). Road to Vostok Vol.1 is ratified **pack-and-upload** (not the reversible-quarantine alternative §4 offered). No disposition in this report is provisional after §9.

## 1. `assets/imported/Namaqualand/` — TRACED (CC0, Poly Haven) with 1 exception

The directory is a Blender-scene decomposition whose mesh/texture names map to published Poly Haven Namaqualand-collection assets. 17 slugs verified via API (HTTP 200, model/texture metadata + authors), 2026-07-05: `quiver_tree_01`, `quiver_tree_02`, `namaqualand_boulder_02/03/05/06`, `namaqualand_rocks_01`, `namaqualand_stones_01`, `namaqualand_cliff_02`, `othonna_cerarioides`, `didelta_spinosa` (large/med/small = LOD/size variants of the one slug), `crystalline_iceplant`, `flower_gazania`, `flower_ursinia`, `flower_heliophila`, `searsia_lucida`, plus textures `gravelly_sand`, `cliff_side`. Associated 8K HDRI-derived PNGs trace to `goegap_road`, `kloofendal_48d_partly_cloudy_puresky`, and `rogland_sunset` — all API-verified CC0 2026-07-05 (`goegap_road` appears in both packs' texture sets; see §5). Site license quote (fetched 2026-07-05): *"Our assets are all licensed as CC0, which is effectively Public Domain…"*

**Exception — QUARANTINE-RECOMMENDED**: `fine_leaf_01_{a,b,c}` meshes + textures. `api.polyhaven.com/info/fine_leaf_01` → HTTP 404; site search finds no such slug. Very likely an unpublished sub-asset of the Poly Haven scene, but no source document states its license → untraceable by the standard. Code impact: `astraweave-terrain/src/biome.rs:810` (one desert-scatter species; loader skips missing files gracefully, `engine_adapter.rs:2578`).

**Recommendation**: TRACED → **eligible for release packs**, excluding the `fine_leaf_01_*` files (local quarantine). Note for AD.3 packaging: the `.jpg.png`/`.exr.png` double-extension files are conversion artifacts of the import pipeline; they inherit the source slug's CC0.

## 2. `assets/imported/verdant_trail/` — TRACED (CC0, Poly Haven) with 3 exceptions

API-verified slugs (2026-07-05): `jacaranda_tree`, `island_tree_01`, `dead_tree_trunk`, `dead_tree_trunk_02` (separate published slug), `coast_rocks_05`, `coastal_cliff_02`, `coastal_cliff_04`, `rock_07`, `rock_08`, `shrub_01`–`shrub_04` (all four individually), `grass_medium_02`, `stone_01`, `dry_branches_medium_01`, `tree_small_02`, `forest_leaves_04`, `dirt_floor`, `goegap_road` (HDRI).

**Exceptions — QUARANTINE-RECOMMENDED**: `dirt_bank*` (mesh 368 MB + any textures; slug 404), `sticks_debris*`, `grass_debris*` (slugs 404; unpublished scene sub-assets). Code impact: `biome.rs:1052,1065` scatter entries (graceful skip); `dirt_bank` has no sample-set consumer.

**Recommendation**: TRACED → **eligible for release packs** minus the three exception families.

## 3. The two root `.blend` monoliths — QUARANTINE-RECOMMENDED (as wholes)

`assets/Namaqualand.blend` (122.7 MB) and `assets/verdant_trail.blend` (222.3 MB) are the scene assemblies the above directories were decomposed from. Their contents are ~95% traced-CC0 Poly Haven assets, **but each embeds the untraceable sub-assets listed above** (fine_leaf, dirt_bank, debris), and no license document covers the assembly as published. The decomposed, per-asset-traceable outputs supersede them for engine use (BiomePack manifests consume the decomposition, not the blend). **Recommendation**: keep both blends in local quarantine; ship only the traced decomposed files in packs. (If the director wants the blends shipped as authoring sources, the untraced sub-assets must be stripped first — a mutating operation for a later beat.)

## 4. `assets/Road to Vostok Assets Vol.1/` (186 files, 41 prop dirs) — TRACED (CC0) with a takedown caveat

- **Source**: <https://roadtovostok.itch.io/road-to-vostok-assets-vol1> (Road to Vostok, indie game by a Finnish solo developer).
- **License evidence** (fetched 2026-07-05): Wayback snapshot <http://web.archive.org/web/20260422060538/https://roadtovostok.itch.io/road-to-vostok-assets-vol1> — page states *"5. Usage – 100% Free (CC0) / No attribution needed / Commercial use allowed"* and the itch.io storefront metadata field reads *"Asset license: Creative Commons Zero v1.0 Universal."*
- **Caveat**: the live page now returns itch.io's own HTTP 404 (verified 2026-07-05) — the pack was unpublished sometime after 2026-04-22. CC0 is an irrevocable public-domain dedication made at publication (snapshot shows a long-standing public release: 36 ratings, 48 comments), so the dedication stands; the archived snapshot is the durable evidence and is pinned in `THIRD_PARTY_LICENSES.md` §8.
- **Recommendation**: TRACED → **eligible for release packs**. Include the Wayback URL in the release page's attribution notes. If the director prefers extra caution given the takedown, quarantine is the reversible alternative — but the evidence for the CC0 dedication is unambiguous.

## 5. Corrections / cross-references

`goegap_road` was listed by the tracing agent under the verdant_trail cluster (it appears in both packs' texture sets); the slug is verified regardless of which pack cites it. `assets/models/pine_tree_01_1k.glb` (the 1 GiB AD.0 R2 file) traces to Poly Haven `pine_tree_01` (API-verified 2026-07-05, authors Rob Tuytel/Rico Cilliers) — CC0, pack-eligible; its `_1k` name refers to its texture tier, not its geometry weight.

## 6. Adjacent unknowns (recorded for completeness; all QUARANTINE-RECOMMENDED unless traced later)

| cluster | status | note |
|---|---|---|
| `assets/Forest Scene/` scene art (150 tracked files) | UNTRACEABLE | only Unity SDK package licenses exist (machine-generated `Library/`); no license doc for the scene content; `.sln` name is filename-inference only |
| `assets/Symphonie/` (5) | UNTRACEABLE | support/contact README only, no license text |
| `assets/models/Amber-Npc/` (158) | UNTRACEABLE — **elevated risk** | Character-Creator-style export; such exports are typically licensed to the exporting user and NOT redistributable; treat as quarantine-first |
| `assets/castles_forts_asset_pack/` (5), `assets/Texture/` (8), `assets/Mesh/` (2), root `assets/{Albedo,AO,Displacement,Gloss,Normal,Roughness}.jpg` + `Displacement.exr` | UNTRACEABLE | zero license evidence on disk; no name-match to a verifiable source |
| `assets/hdri/polyhaven/*` catalog HDRIs | **TRACED** | all remaining catalog slugs API-verified CC0 2026-07-05 (see manifest §9) |
| Root `assets/*_4k.gltf` material sets (8) | **TRACED** | all 8 slugs API-verified CC0 2026-07-05 (manifest §9) |
| `assets/models/AnimationLibrary_Godot_Standard.glb` | **TRACED** | Quaternius Universal Animation Library, CC0 (manifest §9) |
| `assets/2D assets`, `3D assets`, `Icons`, `UI assets`, `Archive`, `Other` (Kenney library) | TRACED | per-pack in-repo `License.txt` (CC0) + kenney.nl/support |

## 7. Summary for the AD.1 gate

- **Pack-eligible now**: Namaqualand (minus fine_leaf), verdant_trail (minus dirt_bank/debris), Road to Vostok Vol.1, pine_tree_01_1k.glb, all catalog HDRIs, root glTF material sets, the Kenney/KayKit libraries.
- **Local quarantine (never uploaded)**: fine_leaf_01_*, dirt_bank*, sticks_debris*, grass_debris*, both root .blend monoliths, Forest Scene art, Symphonie, Amber-Npc, castles_forts, Texture/, Mesh/, root Albedo-set, the 18 `assets/materials` C7 files + `assets/textures/cobblestone.png` (sample-set rows — see manifest §6), and the 18 C6 runtime materials pending their re-cook (manifest §5).
- **Blocked on director**: optional RtV caution call (§4). (The former "logo authorship confirmation" item was **closed 2026-07-05 (AD.1.A)** — `Astraweave_logo.jpg` + the splash video are first-party Artlist AI Output; see `THIRD_PARTY_LICENSES.md` §4. Corrected here per AD.3.R finding P0-2.)

## 8. AD.1.B — pack-bucket remainder trace (2026-07-06)

Closes AD.3.R's G-4 gap by tracing the four upload-blocked pack-bucket scopes to the AD.1 standard. Full family tables + evidence: `THIRD_PARTY_LICENSES.md` §11; API captures in `docs/audits/evidence/ad1b_provenance_2026-07-06/`.

- **`assets/textures/pine_forest/`** (133 files) — **MIXED**. 73 files TRACED (Poly Haven CC0, incl. new slugs `pine_bark`/`fern_02`/`rock_moss_set_02`/`aerial_rocks_04`/`evening_road_01_puresky` + inherited §3 slugs); **60 files QUARANTINE-RECOMMENDED** (`pine_trunk_*`, `fir_trunk_*`/`fir_bark`/`fir_twig`, `tree_trunk`, `tree_roots_*`, `pine/fir_twig`, `pine_cover_01`, `dead_tree*`, `montaigle_ruins_*` — 404 slugs, no name-link).
- **`assets/textures/` loose** (347; 15 pre-dispositioned) — 313 TRACED (21 new Poly Haven slugs + 47 inherited + 5 Kenney twins), **19 QUARANTINE** (`ivy`, `tiny_purple_succulant`, non-slug tail `planks`/`transmission`/`ground_mask_01`/`LoL_*`/`bee`/etc.).
- **`assets/models/` loose** (560; 30 pre-dispositioned) — 449 TRACED (442 Kenney SHA-256 twins across Nature Kit/Retro Medieval/Blocky Characters + 6 ambientCG `3DTreeStump001` + 1 Poly Haven `coast_sand_01`), **81 QUARANTINE** (`house1..5.glb` + the primitive/greybox family — no Kenney twin, DIFFER hashes rule out Kenney lineage).
- **`assets/tests/textures/texture-{a..r}.png`** (18) — **TRACED CC0** (Kenney Blocky Characters; 16 byte-identical + 2 pixel-identical). Zero references reconfirmed at HEAD. Redundant duplicate of a retained pack asset → pack under Blocky Characters or safe-drop as duplicate; NOT purge (traceable). Registry "corrupt fixture" label is an overclaim to correct.

**Bucket-move ledger**: new pack→quarantine = **160 files / 1,084,369,834 B**; scope 4 = 18 files gate-unclassified→pack. Updated partition (AD.3.R buckets): pack **89,998 / 19,853,620,060 B**, quarantine **990 / 7,383,230,576 B**, gate-unclassified **137 / 791,711,859 B** (sample 114 + retained 54 unchanged); grand total invariant **91,293 / 28,243,219,612 B**. **Residual (out of AD.1.B scope, flagged)**: `assets/textures/{pbr,Fabrics,grass_hd,models}` (100 files, ~253 MB) are subdirectories, not the depth-2 loose set — still untraced inside `textures-environment`; a follow-up beat must trace them before that pack is fully upload-clear. **[CLOSED by AD.1.C §9.1.]**

## 9. AD.1.C — residual trace + ratification (2026-07-06)

Finalizes the disposition record. Full trace tables + evidence: `THIRD_PARTY_LICENSES.md` §12 (residual) + §8 (ratified dispositions); API captures `docs/audits/evidence/ad1c_residual_2026-07-06/`.

### 9.1 Residual trace — the 100 flagged files (closes §8's residual flag)

`assets/textures/{pbr,Fabrics,grass_hd,models}`, 100 tracked (99 LFS + 1 stray). Paths in index casing; `Fabrics/` is the case-mismatch dir (registry/worktree: `fabrics/`).

| scope | files | verdict | basis |
|---|--:|---|---|
| `pbr/` (`PBR_{2K,4K}/{Dirt_Mud,Moss_Ground,Sand_Desert,Stone_Terrain_Rock}`) | 56 | **TRACED — first-party procedural CC0** | `tools/pbr_gen/generate_pbr_textures.py` (CC0 header, fixed seeds, exact 4×7×2 structural match, dims 2048²/4096²). AD.1.B's "ambientCG-style" guess overturned by in-repo generator. |
| `Fabrics/` | 24 | **TRACED — Poly Haven CC0** | `fabric_leather_01`, `hessian_230`, `rough_linen` API-200 (2026-07-06); `*_1k.glb` preview bundles inherit slug CC0. |
| `grass_hd/` | 4 LFS | **TRACED — Poly Haven CC0** | `grass_medium_01` API-200 (AD.1-verified). 5th file `grass_hd/grass` = 1-byte plain-blob stray (non-asset, outside LFS partition). |
| `models/houses/` | 15 | **QUARANTINE (ratified)** | `house1..5_tex{1,2,3}` textures for the AD.1.B-quarantined `house1..5.glb`; equally untraceable. |

**84 TRACED / 226,760,432 B · 15 QUARANTINE / 25,879,389 B.** Net move: 15 files pack→quarantine. `textures-environment` (446 LFS / 4,998,449,523 B) now has **zero untraced files** — the blocker is cleared; upload-clear for its TRACED portion (~4.56 GiB, ~3 zips at the G-3 name-prefix split), excluding 34 ratified-quarantine files (19 loose-tail §11.4 + 15 houses).

### 9.2 Ratified director decisions (record; do not reopen)

1. **Quarantine list RATIFIED** — untraceable 1,005 (830 AD.1 + 160 AD.1.B + 15 AD.1.C) + hygiene 71 (`archive/` 4 + `assets/cache/impostors/` 67, G-2) + redundant-duplicate 18 (`assets/tests/`, G-2) = **1,094 files / 7,481,318,328 B**. AlkaKrab → **option (a) quarantine**. Destiny: purged in AD.6, never uploaded. (§8 items 1–2.)
2. **`assets_src/` → pack `materials-src`, deferred until after AD.4** (§8 item 4). **Pin evidence:** `assets/asset_manifest.toml` pins by upstream Poly Haven/ambientCG slug/URL only — **no `sha256`/content-hash field on any entry** (grep-confirmed), and `cloth`/`plaster`/`rock_lichen` are entirely unpinned (§6). Not (URL ∧ hash) → the pack branch, not purge-and-refetch. The deferred pack inherits the per-family §5/§6 dispositions (untraceable families excluded when cut).
3. **Road to Vostok Vol.1 → pack + upload** with the traced set (§4 caution reversed; release page becomes the durable off-site copy of an unpublished irreplaceable CC0 asset). §8 item 3 closes.
4. **G-3 split-rule amendment RATIFIED** — within a >2 GB loose-file directory, split at name-prefix (slug-family) boundaries; one slug never straddles zips; each zip independently verifiable. Recorded in `assets/packs.manifest.toml`.
5. **Matching standard RATIFIED** — trailing-numeral variant admissible, word-substitution not. Recorded in `assets/packs.manifest.toml`; closes the `moss`→`moss_01` flag as TRACED.
6. **Sequencing constraint** — **AD.4 is a hard predecessor of AD.6** (39 ratified sample rows point at quarantine-destined files whose slots AD.4 refills; purge-before-re-cook would break the fresh-clone render criterion). Recorded in `LFS_REMOVAL_PLAN.md`.
7. **Retained-non-sample verification** — the 10 `examples/fluids_demo/captures/*.png` in the retained bucket: the `fluids_demo` example is **live** (workspace member, root `Cargo.toml:142`; `cargo metadata` lists package `fluids_demo`). Verified alive; the 10 files remain retained.

### 9.3 Final five-bucket partition (gate-unclassified → 0)

| bucket | files | bytes |
|---|--:|--:|
| pack | 90,031 | 20,547,244,167 |
| quarantine | 1,094 | 7,481,318,328 |
| gate-unclassified | **0** | **0** |
| sample | 114 | 182,607,234 |
| retained | 54 | 32,049,883 |
| **total (invariant)** | **91,293** | **28,243,219,612** |

Gate (137 / 791,711,859 B) = `archive/` 4 + `assets/cache/impostors/` 67 + `assets_src/` 66, disposed exactly to zero. Cross-foot verified against the full-history LFS enumeration this session.

## 10. AD.4 close-out (2026-07-07)

Executed the AD.4.R plan; full record in `THIRD_PARTY_LICENSES.md` §13 + `docs/audits/AD4_RECOOK_OUTCOME.md`. Disposition-relevant outcomes:

- **39 quarantine-destined slots refilled**: 18 C6 cooked from traced `assets_src` + 18 C7 re-acquired from traced CC0 Poly Haven slugs (§13.1) → `assets/materials/derived_1k/` (new paths); `assets/textures/cobblestone.png` retired by re-pointing `unified_showcase` to the C6 cobblestone derivative.
- **Two debris rows RETIRED** (quarantine-and-accept, director-ratified): `verdant_trail/meshes/{sticks,grass}_debris_a.glb` originals are unpublished Poly Haven sub-assets (un-re-acquirable); the sample set drops **153 → 151 rows**; scatter thins gracefully (`biome.rs:1052,1065` → `engine_adapter.rs:2578` skip-missing). The files stay in quarantine and leave in AD.6's purge.
- **The 37 old occupants are untouched** — they leave in AD.6's path purge (overwriting them would trap unlicensed history in surviving paths).
- **Bucket successor**: the 91,293 LFS invariant retires → **91,329** (+36 `derived_1k`); the 18 C7 `assets_src` copies move pack→quarantine. Partition table: §13.5.
- **The AD.4→AD.6 hard-predecessor edge clears**: no ratified sample row now points at a quarantine-destined file.
