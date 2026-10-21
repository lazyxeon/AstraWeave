# AD.4 — Sample-slot re-cook + `materials-src` cut (outcome)

**Date:** 2026-07-07  **Branch:** `campaign/roadmap`  **Predecessor:** AD.4.R (recon, `d:\tmp\AD4R_RECON_REPORT.md`)
**Companion trace edits:** `THIRD_PARTY_LICENSES.md` §13, `docs/audits/DISPOSITION_REPORT_imported.md` §10.
**Method discipline:** every count/byte/sha256/loop result below is copied from tool output this session (PIL cook, `aw_asset_cli validate`, `cargo test`, the deterministic zip build, `cargo xtask fetch-assets`, `gh release`). Zero `git push`; zero git-LFS network ops. The `gh release upload` used the plain release API (not a push).

---

## 1. End state (the mission's five exit criteria)

| Exit criterion | State | Evidence |
|---|---|---|
| Every ratified sample slot filled by licensed, contract-conforming content | **DONE** | 36 `derived_1k/` PNGs (1024² RGBA, `aw_asset_cli` 36/36) + cobblestone re-point |
| Two debris rows retired (153 → 151) | **DONE** | §13.4; files untouched (quarantine-and-accept) |
| `materials-src` cut and live on `assets-v1` | **DONE** | zip `652458f3…`, asset id 469429382 `state=uploaded`, live loop ✓ |
| AD.4 → AD.6 hard-predecessor edge cleared | **DONE** | old occupants intact for AD.6 purge; §10 |
| Adversarial verification before this note | **DONE** | §7 (independent audit; found+repaired the ATTRIBUTION regression) |

---

## 2. The 39-slot final ledger

The 39 quarantine-destined sample slots resolve as: **36 filled** (derived_1k) + **1 substitute re-pointed** (cobblestone.png) + **2 retired** (debris).

### 2a. C6 — 18 files cooked from `assets_src` (traced, licensed history)
`cook_1k.py cook_family` downscales the pre-packed 3-map set (albedo/normal/mra) to the 1024² contract.

| family | slug (Poly Haven / ambientCG) | maps | wired consumer |
|---|---|---|---|
| cobblestone | rocky_trail | base+_n+_mra | `unified_showcase` (albedo only) |
| gravel | gravel_concrete_03 | base+_n+_mra | beach, biomes, mountain, river, tundra |
| ice | snow_03→Ice003 (ambientCG) | base+_n+_mra | tundra |
| metal_rusted | rust_coarse_01 | base+_n+_mra | *(sample fill — no live biome; see §6)* |
| moss | moss_01 | base+_n+_mra | river, swamp |
| wood_planks | *(first-party / source set)* | base+_n+_mra | *(sample fill — no live biome; see §6)* |

### 2b. C7 — 18 files re-acquired + cooked (fresh AD.1-standard provenance)
`cook_1k.py cook_family_from_maps` packs MRA (R=metallic=0, G=roughness, B=AO) from PolyHaven's separate roughness+ao maps. Provenance: `THIRD_PARTY_LICENSES.md` §13.1 + `docs/audits/evidence/ad4_c7_reacquire_2026-07-07/`.

| family | slug | author | wired consumer |
|---|---|---|---|
| plaster | plastered_wall_02 | Charlotte Baglioni | desert |
| tree_leaves | forest_leaves_02 | Rob Tuytel | forest |
| tree_bark | tree_bark_03 | Rob Tuytel | forest, swamp |
| cloth | fabric_leather_01 | Rob Tuytel | desert |
| roof_tile | roof_tiles_14 | Rob Tuytel | *(sample fill — no live biome; see §6)* |
| rock_lichen | lichen_rock | Rico Cilliers | forest, grassland, swamp |

`rock_moss_set_02` was **rejected** (type-2 model pack); `lichen_rock` selected search-first as the exact tileable-texture match. All 6 JSONs are `"type": 1`.

### 2c. Substitute + retirements
- **cobblestone.png substitute** (`assets/textures/cobblestone.png`, §6 unlicensed): retired by re-pointing its sole live consumer to `derived_1k/cobblestone.png`. Old file untouched.
- **Debris (2 rows)**: `assets/imported/verdant_trail/meshes/{sticks,grass}_debris_a.glb` — un-re-acquirable Poly Haven sub-assets; **quarantine-and-accept**, sample rows 153 → 151. Scatter thins gracefully (`biome.rs:1052,1065` skip-missing via `engine_adapter.rs:2578`). Files untouched.

### 2d. traced-9 in-place re-cook (27 files, licensed history — not part of the 39)
`{grass,forest_floor,mountain_rock,mud,stone,rock_slate,dirt,sand,snow}{,_n,_mra}.png` re-cooked **in place** 2048² → 1024² (licensed provenance permits in-place). All 27 verified 1024² RGBA (one flat MRA, `stone_mra.png`, is a near-degenerate placeholder — dimensions compliant).

---

## 3. Cook tool

`tools/material_cook/cook_1k.py` (+ `test_cook_1k.py`, contract test — 1024²/RGBA/PNG × 3 maps, PASS). Two entry paths: `cook_family` (pre-packed 3-map, for C6 + traced-9) and `cook_family_from_maps` (pack MRA from separate maps, for C7). `cook_one` is in-place-safe (loads+closes source before writing → no Windows sharing violation on the traced-9 in-place re-cook). Not `scripts/import_terrain_textures.py` (that is a 2048/9-family repo-tree importer — wrong pipeline, per AD.4.R).

`aw_asset_cli validate` on the 36 derived_1k: **36/36 pass** (17 benign "metallic channel flat" warnings — expected for terrain MRA, R=metallic=0).

---

## 4. Consumer re-point ledger

| Consumer | Change | Refs |
|---|---|---|
| 9 biome `materials.toml` (beach, biomes, desert, forest, grassland, mountain, river, swamp, tundra) | C6/C7 root refs → `../derived_1k/<family>.png` | 24 unique paths (8 families × 3 maps) |
| `examples/unified_showcase/src/main.rs:1130` | `assets/textures/cobblestone.png` → `assets/materials/derived_1k/cobblestone.png` | 1 (albedo) |

**Re-point completeness (adversarial):** 0 stale C6/C7 *root* refs remain in any biome toml; traced-9 root refs deliberately unchanged (licensed in place). Coverage: **25 of 36** derived_1k files are wired to a live consumer; **11 are sample-set fills** with no live biome layer (see §6). **No live consumer dangles on an old to-be-purged path** (dangling-consumer gate PASS).

---

## 5. Verification ladder

- **Rung 1 (refs resolve).** 168 biome-toml material path refs checked → **derived_1k missing: 0**. (10 MISSING are the pre-existing gitignored `assets/_downloaded/polyhaven/*` refs used by `assets/materials/polyhaven/materials.toml` — graceful all-None, unrelated to AD.4.)
- **Rung 2 (loader).** `aw_editor` unit test `viewport::canonical_terrain_pack::tests::loads_grassland_pack_when_present` (grassland now references `derived_1k/` for its C6/C7 layers). Actual output:
  ```
  running 1 test
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4027 filtered out; finished in 0.55s
  ```
  It did **not** skip (no "Skipping grassland…" under `--nocapture`), so the full assertions ran: pack parses, 5 layers, grass layer-0 albedo decoded at `CANONICAL_ALBEDO_RES² × 4` (1024²×4). *(Note: a first attempt with the filter `canonical` matched 0 tests — `cargo test` exits 0 on an empty filter, so exit-code alone is not pass evidence; this is the real test-name run.)*
- **Rung 3 (render).** Belongs to the director — see §8 repro.

---

## 6. `materials-src` cut — build, loop, upload, live loop

**Membership (verified to the byte):** `assets_src/materials/*.png` (63) − 18 untraceable C7 + `assets_src/textures/*.png` (3) = **48 files / 615,202,592 B uncompressed** (exact match to the ratified figure). 0 LFS pointer stubs (all content materialized; 6 members are 131–132 B degenerate placeholder MRA maps pre-existing in `assets_src` — real PNGs, shipped as a faithful source archive).

**Deterministic zip** (`tools/material_cook`-adjacent build, reproducing the AD.3 recipe): sorted repo-relative members, fixed `date_time=(2026,7,6,0,0,0)`, `ZIP_DEFLATED`, `external_attr=0o644<<16`, `_PACK_MANIFEST.txt` + `_ATTRIBUTION.txt` appended last.

| field | value |
|---|---|
| zip size | **570,810,569 B** |
| sha256 | `652458f3044ca16f71291e9639713ac4a8295d13e724fb3f3112434c5be3108d` |
| determinism | byte-identical rebuild (same sha256) — PASS |
| members | 50 (48 real + 2 synthetic); `testzip()` OK |
| release URL | `https://github.com/lazyxeon/AstraWeave/releases/download/assets-v1/materials-src.zip` |
| release asset | id 469429382, size 570,810,569, `state=uploaded` (release now 19 assets) |
| manifest | `assets/packs.manifest.toml` — new `[[pack]] name="materials-src"` |

**Recipe divergence (transparent):** AD.3's zips stamped their 2 synthetic files with Python's `writestr(str,…)` default `0o600<<16`; this build stamps all members uniformly `0o644<<16`. Cosmetic Unix-permission metadata on two in-zip text files — zero effect on the sha256-gated unpack (the Rust `zip` extractor ignores Unix mode on Windows).

**Local 3-run loop** (`cargo xtask fetch-assets --pack materials-src`, `source=file:///…`):
- Run 1 — `sha256 verified (652458f3…)` → unpacked → `1 fetched`.
- Run 2 — `= up to date (stamp sha256 matches manifest) — skipping` → `1 up-to-date, 0 fetched`.
- Content round-trip — 48 `.png` unpacked, both synthetic files present, 3/3 spot sha256 byte-identical to source.
- Run 3 — corrupt stamp → `stamp sha256 differs from manifest — re-fetching` → re-fetched + verified.
- Fixture suite `cargo test -p xtask`: **8 passed**.

**Live loop** (manifest `source` flipped to the release URL; **same sha256 pin** — that identity is the proof):
```
> materials-src: acquiring https://github.com/lazyxeon/AstraWeave/releases/download/assets-v1/materials-src.zip
  sha256 verified (652458f3044ca16f71291e9639713ac4a8295d13e724fb3f3112434c5be3108d)
+ materials-src: unpacked to …\assets\packs\materials-src
fetch-assets summary: 0 up-to-date, 1 fetched, 0 failed (of 1)
= materials-src: up to date (stamp sha256 matches manifest) — skipping   [Run 2, idempotent]
```

---

## 7. Bucket bookkeeping — successor denominator

AD.4 adds 36 LFS paths (`derived_1k/`); nothing deleted (old occupants leave in AD.6). The 18 C7 `assets_src` copies (untraceable) move **pack → quarantine** (excluded from `materials-src`).

| bucket | AD.1.C | AD.4 | delta |
|---|--:|--:|---|
| pack | 90,031 | 90,013 | −18 (C7 `assets_src` → quar) |
| quarantine | 1,094 | 1,112 | +18 |
| gate-unclassified | 0 | 0 | — |
| sample | 114 | 150 | +36 (`derived_1k`) |
| retained | 54 | 54 | — |
| **total** | **91,293** | **91,329** | **+36** |

Successor denominator **91,329 = 91,293 + 36** (foots; deltas net +36). Independently re-derived and confirmed consistent across `THIRD_PARTY_LICENSES.md` §13.5 and `DISPOSITION_REPORT_imported.md` §10. *(The debris "153 → 151" is the AD.0 sample-**row** count — a different denominator from this LFS sample-**file** bucket; §13.4 now disambiguates the two.)*

---

## 8. Adversarial verdict

Independent audit (separate agent, read-only, skeptical mandate): **bookkeeping sound, anti-drift respected on the three named surfaces** (`.gitattributes`, AD.3 pack entries, quarantined clusters — all untouched). Items 1–5 PASS with hard evidence (arithmetic footing, JSON cross-checks, PIL dimension/mode verification). One real defect surfaced **and was repaired this session**:

- **`assets/_downloaded/polyhaven/ATTRIBUTION.txt` regression (FIXED).** The C7 `fetch` invocation drove `astraweave-assets`'s `generate_attribution_file()`, which **overwrites rather than merges**, clobbering the git-tracked evidence file from **13 → 6** entries and dropping still-in-use, still-on-disk assets (aerial_rocks, metal_plate, wood_floor, sky_day, sky_indoor, gravel, metal_rusted, moss, snow, mud, cobblestone). `.gitignore` mandates this file be tracked ("license evidence must be tracked (AD.1)"). **Repair:** restored to the correct **19-entry union** (13 committed blocks byte-identical + 6 C7 appended; git diff = 45 ins / 3 del, none touching the 13); the stray duplicate download `lichen_rock/` (my isolated-manifest artifact; canonical handle is `rock_lichen`) removed so 19 handles ⇄ 19 dirs reconcile. Provenance for the dropped assets was never actually lost — `THIRD_PARTY_LICENSES.md` §11.1 independently traces them by slug — but the secondary tracked artifact had regressed silently. The underlying **tool bug is not fixed** (see §9).

**Render (rung 3) repro for the director:**
1. `cargo editor` → open a biome that consumes derived_1k: **desert** (plaster+cloth), **forest** (tree_bark+tree_leaves+rock_lichen), **swamp** (moss+tree_bark+rock_lichen), **tundra** (ice+gravel). Confirm terrain renders the re-cooked textures with no magenta/missing-material fallback.
2. `cargo run -p unified_showcase --release` → confirm the TowerStone ground shows the cobblestone albedo (now `derived_1k/cobblestone.png`).

---

## 9. Open items / handoffs

1. **Tree-wide validator fails 82 files (unowned, pre-existing — NOT AD.4).** `aw_asset_cli validate assets/materials` → 223 total, 141 pass, **82 fail** = **72 malformed `.ktx2`** ("Unknown texture format (not KTX2 or AWTEX2)", 36 under `assets/materials/` + 36 under `baked/` — the "fake AW_TEX2" files from the 2026-07 asset audit) + **10 `arrays.toml`** ("Missing 'name' field"). None are AD.4 outputs (the 36 derived_1k + 27 traced-9 are all `.png` and pass). Needs a future hygiene / cook-path beat to claim ownership.
2. **`generate_attribution_file()` overwrite bug (data repaired, tool not).** `tools/astraweave-assets/src/provider.rs` regenerates `ATTRIBUTION.txt` from a single fetch's asset list without merging existing content. A future partial-manifest fetch will re-clobber it. Fix options: make it read-merge, or route acquisitions through the append-mode `organize.rs::update_attribution` path. Out of AD.4 scope (tool change; not a "reroute").
3. **AD.6 path-purge inventory.** Old-occupant paths are still referenced by **non-live** surfaces the purge must account for: dead-orphan `examples/unified_showcase/src/main_bevy_v2.rs` (not a `[[bin]]`, not a declared `mod`), source-only `assets_src/environments/{grassland,desert}.toml` (no live loader found), and the procedural `texture_synth.rs` roof_tile generator (writes its own, does not read the occupant). No *live* consumer among them.
4. **11 sample-set fills without a live biome consumer** (expected, not a defect): `metal_rusted{,_n,_mra}`, `wood_planks{,_n,_mra}`, `roof_tile{,_n,_mra}`, `cobblestone_n`, `cobblestone_mra`. They fill their ratified slots with licensed content; no live biome layer references them (their old occupants were referenced only by the non-live surfaces in item 3). `unified_showcase` consumes only cobblestone's albedo, hence `cobblestone_n/_mra` are unwired.
5. **Rung 3 (render) is the director's** — §8 repro. This note does not claim render verification.

---

*Nothing in this session was pushed. Commits are local; hashes are cited in the session record. The `materials-src` upload used the `gh` release API against the pre-existing `assets-v1` tag (anchor `5b2c6c8bd`, #197) — not a git push, no new commit on the remote.*
