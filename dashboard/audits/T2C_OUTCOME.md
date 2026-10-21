# T.2c — Real PBR materials for the synthetic biome slots (outcome)

> **Beat:** T.2c (terrain series) · **Date:** 2026-07-25 · **Executes:** the director's observation at the T.2a gate — slots 0/1/4 *"still read as flat and shiny … they definitely don't read as proper PBR materials"*
> **Baseline commit:** `bf57b5f1d`.
> **Before:** slot 0 = synthetic flat-green albedo + an **alpha-cutout foliage card's** normal map; slots 1 and 4 = 100% procedural (`tools/pbr_gen`) with effectively flat normals.
> **After:** slot 0 = PolyHaven `aerial_grass_rock`, slot 1 = `sand_01`, slot 4 = `snow_02`, each cooked to `assets/materials/derived_1k/{grass,sand,snow}{,_n,_mra}.png`.
> Station frames + `metrics.csv` in `d:/tmp/t2c_staging/render/{before_t2c,after_t2c}/` (session-local, not committed).
> Anti-drift honoured: three slots only; no shader constants; no scatter, water, `pbr_gen` or classification work; `arrays.toml` untouched.

---

## 0. Summary

The three slots were not photographed materials, so no amount of tuning could fix them — T.2a said so explicitly and put them on the director's side of the line as art-direction purchases. This beat buys them.

All three replacements are API-verified CC0 PolyHaven ground scans, cooked through the ratified `cook_1k.py` path, measured at every stage, and installed without disturbing the palette-remap join. **The visible defect the beat was named for — the black shards scattered through the grassland — is gone.**

Two things are reported as findings rather than wins, because they are the director's call and because a beat that only reports its successes is not evidence:

1. The **tundra reads noticeably darker** (station mean 172.8 → 113.1). This is the true de-lit albedo of a real snow scan (165 grey) replacing a synthetic near-white (231). It is physically honest and may still be artistically wrong. §4.3.
2. The **hex-tile lattice is now visible on tundra**, where before there was no contrast to reveal it. T.2a predicted exactly this: for the procedural slots "the only discernible pattern is the hex-tile lattice itself". §4.3.

A third finding is structural and belongs to the harness, not the materials: **T.2a's tundra station no longer frames tundra**, because T.2a's own Phase 3 reclassified the ground under it. §4.2.

---

## 1. Acquisition + provenance (Phase 1)

Site licence: <https://polyhaven.com/license> — "All assets … are all licensed as CC0". Captured to `docs/audits/evidence/t2c_materials_2026-07-25/polyhaven_license_page.html`; raw `/info` + `/files` JSON per slug under `polyhaven_api/`. Fresh provenance rows: `THIRD_PARTY_LICENSES.md` **§15** (a prior trace of any slug authorizes nothing).

| slot | slug | name / author | type | scan dims | maps shipped |
|---|---|---|---|---|---|
| 0 grassland | `aerial_grass_rock` | Aerial Grass Rock / Rob Tuytel | 1 (texture) | 15.0 m | Diffuse, nor_gl, Rough, AO, Displacement, arm, rough_ao |
| 1 desert | `sand_01` | Sand 01 / Rob Tuytel | 1 (texture) | 1.5 m | Diffuse, nor_gl, Rough, AO, Displacement, arm, Bump, spec |
| 4 tundra | `snow_02` | Snow 02 / Rob Tuytel | 1 (texture) | 2.0 m | Diffuse, nor_gl, Rough, AO, Displacement, arm, Bump, spec |

All three: `type:1`, `max_resolution [8192,8192]`, CC0, fetched at 2K — comfortably over the ≥2K floor, and over what the pipeline can consume (the loader clamps albedo to 1024² and aux to 512²).

### 1.1 Scan scale was a selection criterion, not an afterthought

`materials.toml` documents tiling 128 as **one repeat per 4 m**. So the scan's real-world size decides whether the material reads at native scale:

- `aerial_grass_rock` 15 m → shrunk 3.75×, which brings aerial-scale features (0.5–1 m grass clumps) down to a plausible 13–27 cm ground-detail scale.
- `sand_01` 1.5 m → enlarged 2.7×; fine sand survives enlargement.
- `snow_02` 2.0 m → enlarged 2×.

### 1.2 The normal-map sanity check — the defect being fixed

The slot-0 defect was a **cutout card's normal map** used as ground. The discriminator is not "steep normals" (genuine deep relief is also steep — the forest control proves it) but *meaningless* normals in regions the source treated as transparent, which show up as non-unit vectors plus an alpha channel on the source.

| file | B mean/sd | \|n\| | B<200 | non-unit >0.30 | alpha |
|---|---|---|---|---|---|
| **LIVE slot 0 `grass_n.png` — the defect** | 225.76 / 38.78 | 0.939 | **18.22%** | **6.41%** | **yes** |
| LIVE slot 1 `sand_n.png` (procedural) | 254.00 / 0.07 | 0.992 | 0.00% | 0.00% | yes |
| LIVE slot 4 `snow_n.png` (procedural) | 254.00 / 0.02 | 0.992 | 0.00% | 0.00% | yes |
| CONTROL slot 2 `tree_leaves_n.png` | 218.87 / 18.83 | 0.880 | 16.94% | 7.00% | yes |
| CONTROL slot 6 `beach_n.png` | 247.86 / 8.23 | 0.978 | 0.24% | 0.00% | yes |
| **CAND `aerial_grass_rock`** | 246.02 / 6.07 | — | **0.00%** | **0.01%** | **no** |
| **CAND `sand_01`** | 234.41 / 17.41 | 0.899 | **5.17%** | **2.38%** | **no** |
| **CAND `snow_02`** | 249.86 / 3.99 | 1.008 | **0.01%** | **0.00%** | **no** |

All three candidates are genuine tileable tangent-space ground normals: no alpha channel, near-unit length, no transparent-region garbage. The two procedural slots being at `B sd 0.02–0.07` is the quantitative form of "no relief at all".

**Read the forest control before reading this table as a simple threshold.** `tree_leaves_n` sits at 16.94% / 7.00% — statistically close to the defect — and is a perfectly good deep-relief leaf-litter scan. What condemned `grass_n` was not those numbers alone but their provenance: T.2a traced it byte-for-byte to `grass_medium_01_nor_gl.png`, a card whose alpha is mean 45.5 / sd 77.3 (mostly transparent) and whose diffuse is blades on black (mean 15.5).

### 1.3 Candidates evaluated and rejected

Recorded so the reasoning is not re-derived later. Each was downloaded and measured, not judged from its thumbnail.

| candidate | why rejected |
|---|---|
| `leafy_grass` | Real detail (albedo lap 33.19, the highest of any candidate) — but its **shipped AO map is a hard constant 255, modal 100.00%, IQR 0**: a flat placeholder by T.2a's own detector. Would have needed AO derived from displacement. Also duplicates slot 2's leaf-litter tone. |
| `snow_04` | **Wins every variance metric** (albedo sd 59–66, AO sd 55.32, and an exact 4.0 m → 1:1 tiling match) and is still wrong: it is a **plowed field**, and its furrows would tile as hard directional stripes across the tundra. The clearest case in this beat of statistics alone giving the wrong answer. |
| `aerial_sand` | Tagged `costal`/`seaside`/`ocean`/`sea` — a coastal sand, which would undo the T.1 desert≠beach amendment. |
| `sparse_grass` | R79/G61/B21 — too dark for a surface covering 91.3% of a world. |
| `grass_path_2`, `grass_path_3` | A path runs through the scan; it would repeat as a visible stripe every 4 m. |

### 1.4 The attribution-merge regression check

The `generate_attribution_file` overwrite bug fired in AD.4 and again in T.1. The prompt asked for it to be verified, not assumed. Baseline copied before fetching, then compared by slug set:

```
baseline slugs: 18
now slugs     : 21
LOST  : NONE
GAINED: ['aerial_grass_rock', 'sand_01', 'snow_02']
```

**The 5.C fix holds** — merge, not clobber. First clean run of this tool across three beats.

---

## 2. Cook + install (Phase 2)

**Cook:** `cook_1k.py::cook_family_from_maps` — the rough+ao path, so the ARM-order trap (the D1 defect class: AO-dead and mirror-metallic) is avoided *by construction*. The fetched `_arm.png` files are deliberately unused. All nine outputs 1024×1024 RGBA.

### 2.1 MRA channel verification (the mandatory trap check)

Flat detector is T.2a's: **modal > 90% OR IQR == 0**; metallic exempt (constant 0 is the post-AD.4.A-D1 dielectric contract).

| file | R metallic | G roughness | B AO |
|---|---|---|---|
| `grass_mra` | **0.00 / sd 0.00** | 206.52 / sd 10.61, modal 4.5%, IQR 13 | 136.55 / sd 26.00, modal 1.6%, IQR 35 |
| `sand_mra` | **0.00 / sd 0.00** | 252.59 / sd 9.25, modal 54.6%, IQR 2 | 219.42 / sd 16.94, modal 3.6%, IQR 17 |
| `snow_mra` | **0.00 / sd 0.00** | 204.33 / sd 3.76, modal 18.5%, IQR 4 | 140.11 / sd 7.48, modal 6.0%, IQR 9 |
| *(control)* `beach_mra` | 0.00 / sd 0.00 | 245.45 / sd 2.25, modal 18.9%, IQR 3 | 240.43 / sd 14.41, modal 7.0%, IQR 16 |
| *(control)* `tree_leaves_mra` | 0.00 / sd 0.00 | 237.92 / sd 10.75, modal 4.8%, IQR 12 | 153.36 / sd 53.02, modal 0.7%, IQR 80 |

**R = 0.00 with sd 0.00 on all three — true-MRA confirmed, no ARM mis-order.** No roughness or AO channel trips the flat detector. `sand` roughness at modal 54.6% / IQR 2 is genuinely near-uniform (dry sand is uniformly rough) and sits in the same class as the shipped `beach` and `gravel` roughness — it is low-variance, not a placeholder.

`tools/material_cook/channel_stats.py` over the whole live pack, both on-disk and at runtime-512 resolution:

```
0 flat channel(s) on disk (modal > 90% or IQR == 0; metallic exempt).
```

Slots 0/1/4 now read `real` on both roughness and AO at both resolutions.

### 2.2 Albedo and normal — the "flat" complaint, measured

| slot | albedo lap: before → after | normal sd: before → after |
|---|---|---|
| 0 grassland | 3.62 → **21.85** (6.0×) | 38.78 (cutout garbage) → 6.58 (real relief) |
| 1 desert | 5.55 → **17.62** (3.2×) | **0.07 → 17.41** |
| 4 tundra | 3.37 → **9.25** (2.7×) | **0.02 → 3.99** |

The desert and tundra normal figures are the headline: those maps previously carried *no relief whatsoever*.

### 2.3 Install

`assets/materials/biomes/materials.toml` slots 0, 1, 4 re-pointed to the `derived_1k` trio, `mra` key (R↔B-swizzled to ORM at load), tiling `[128,128]` unchanged and consistent with neighbours. **`arrays.toml` untouched** — slot order maps 1:1 to `biome_id_to_slot`.

### 2.4 Validation

`aw_asset_cli validate`, per file: **9 passed / 0 failed.** Albedo and normal files: 0 warnings. The three `_mra` files warn:

- all three: `R channel (Occlusion) appears unused` — the validator assumes **ORM** order and reads R as occlusion, but these are **MRA** files where R is metallic = 0 by contract. A naming mismatch in the validator, not a data defect.
- `sand_mra` additionally: `G channel (Roughness) appears unused` — the near-uniform heuristic firing on dry sand's genuinely uniform roughness (186 unique values at runtime resolution).

Both are the same warning class T.1 recorded on `beach_mra`, and the live `gravel_mra` produces them too.

---

## 3. The pre-armed traps (Phase 3)

1. **PALETTE-STEM TRAP — resolved by construction.** The editor's palette remap joins `MaterialLibrary` names to pack layers by **lowercase albedo file stem** (`palette_remap.rs::resolve`, `stem.eq_ignore_ascii_case(name)`). Slots 0/1/4 resolve via stems `grass`, `sand`, `snow`, which are three of the seven paintable entries. The cooked outputs were therefore named `derived_1k/{grass,sand,snow}.png` **so the stems are preserved** — the path changed, the join key did not. No `MaterialLibrary` rename was needed and no test fixture required editing.
   - `cargo test -p aw_editor --lib palette_remap` → **8 passed; 0 failed** (4,022 filtered), including `biomes_pack_resolves_exactly_seven_entries` asserting the paintable set is exactly `[0,1,3,4,5,12,20]`.
2. **Loader stem assertion — passes unchanged.** `cargo test -p aw_editor --lib canonical_terrain_pack` → **2 passed; 0 failed** (4,028 filtered). This is live verification, not fixture-only: `loads_biomes_pack_forest_slot_from_derived_1k` loads the real `assets/materials/biomes/materials.toml` from disk and asserts the full 8-stem set `[grass, sand, tree_leaves, mountain_rock, snow, mud, beach, gravel]` — so it confirms the new paths resolve *and* the join keys survived.
3. **A sentinel that would have bitten a tidier.** `find_assets_dir()` (`viewport/types.rs:224,235`) probes for `assets/materials/grass.png` to locate the assets directory. The pre-T.2c albedo files are therefore **deliberately left on disk**; deleting them as "now unused" would break asset-dir discovery for both CWD and exe-walk-up resolution.
4. **ci-guard rail:** `cargo xtask gen-keeplist` → 22,593 cohabitant entries, **byte-identical to the previous keeplist** (the nine `derived_1k/` files are sample-set files outside cohabitant scope — the same reason T.1's three beach files did not appear). `cargo xtask ci-guard` → **PASS**: "0 tracked pack members, 0 stray blobs under managed roots (22593 keeplist cohabitants); ignore surfaces match".
5. **Slot-1/6 distinctness preserved.** Desert `sand.png` R165.4/G149.5/B111.1 (dry, light, warm) vs beach `beach.png` R129.7/G114.3/B91.4 (damp, dark, brown). The T.1 row-6 amendment is not undone; `aerial_sand` was rejected specifically to protect it (§1.3).

---

## 4. Observation (Phase 4) — the director's gate

Rendered through the T.2a pinned-station harness, which renders the editor's own viewport path offscreen at exactly-specified cameras — the only way to get a true A/B, since the editor cannot pin a camera. Frames in `d:/tmp/t2c_staging/render/{before_t2c,after_t2c}/`. Both legs rendered this session at the same commit, differing **only** in the three `materials.toml` layer paths.

| station | before mean / std / lap | after mean / std / lap | reading |
|---|---|---|---|
| 01 med grassland close-up | 97.960 / 20.041 / 16.2003 | 60.019 / 6.610 / **6.7136** | shards gone (see below) |
| 02 ct forest close-up **(CONTROL)** | 78.059 / 12.753 / 16.7641 | **BYTE-IDENTICAL** | scope proof |
| 03 med mountain close-up | 94.033 / 24.560 / 9.9296 | 72.246 / 14.944 / 9.1043 | blended slot-0 weight |
| 04 desert close-up | 142.684 / 3.753 / 2.0802 | 112.633 / 11.273 / **9.0342** | **4.3× detail** |
| 05 boreal tundra (T.2a pin) | 77.585 / 12.502 / 16.2794 | **BYTE-IDENTICAL** | frames no tundra — §4.2 |
| 06 boreal overview | 142.253 / 63.633 / 11.8269 | 121.122 / 60.903 / **19.3772** | **+64% detail** |
| 07 boreal tundra (T.2c pin) | 172.786 / 4.960 / **1.1342** | 113.094 / 14.219 / **5.9931** | **5.3× detail** |

**Station 02 is byte-identical** — the strongest scope evidence available. The forest frame carries zero slot-0/1/4 weight, so an unchanged hash proves nothing global drifted.

**Station 03 moves although slot 3 is untouched**, and that is expected rather than alarming: terrain vertices blend slots by weight, so a mountain-*dominant* frame still carries nonzero grassland weight. The signature fits — mean falls 23% (the new grassland is darker) while lap barely moves (−8%), which is a blended tint shift, not a material swap. `mountain_rock*` is unmodified on disk.

### 4.1 Grassland — the black shards are gone

The before frame is the defect in its clearest form: saturated green shot through with hard black slashes, which are the cutout card's meaningless normals being shaded. The after frame has **none**.

The rendered Laplacian **falls** (16.20 → 6.71) while the albedo's own Laplacian **rises** (3.62 → 21.85). That is not a contradiction — it is the measurement working. Most of the before frame's high-frequency energy *was the defect*; removing hard black shards necessarily removes local contrast. Per T.2a §1.2, direction of "better" is not constant, and here the metric is confirming the shards are gone rather than scoring the material.

### 4.2 A harness finding: T.2a's tundra station no longer frames tundra

Station 05 is **byte-identical across the swap** while the boreal overview moves — arithmetically impossible unless station 05 frames no slot-4 texels at all. The cause is T.2a's own Phase 3: station 05 was pinned from a survey at `89fbe97eb`, when the Boreal world was 93.3% tundra; widening the boreal band (`TUNDRA_MAX_TEMP_C` 0.0 → −5.0) took that world to 40.9% forest / 52.4% tundra and reclassified the ground under the pin.

**Station 05 was deliberately left in place** — it is a T.2a A/B anchor, and silently re-aiming it would invalidate every frame already registered to it. A new station `07_boreal_tundra_t2c` was added at the current tundra representative vertex (weight ≥ 0.85, median-x, from a survey at T.2c HEAD), which is how the slot-4 evidence above was obtained. Station 07's before-leg lap of **1.1342** is the flattest reading in the entire set — the quantitative signature of the procedural snow this beat replaced.

This is worth carrying forward: **any pinned station is only valid for the classification that produced it.** T.G and any later A/B beat should re-survey before trusting station coordinates.

### 4.3 Two findings for the director, stated plainly

Neither is a defect in the cook; both are art-direction calls, and the render verdict is the director's.

1. **The tundra is now considerably darker** — station 07 mean 172.8 → 113.1, station 06 mean 142.3 → 121.1. `snow_02`'s de-lit albedo is a mid-grey (165/165/167) where the procedural snow was near-white (231/234/239). This is what a real snow scan measures, but under this scene's lighting it can read as grey ground rather than snow. If the director wants brighter snow, the honest fixes are a brighter scan or an albedo-tint decision — **not** re-flattening the material.
2. **The hex-tile lattice is now visible on the tundra close-up.** T.2a predicted this precisely: for the procedural slots "the only discernible pattern is the hex-tile lattice itself". The lattice was always there; the flat material had no contrast to reveal it. T.2a already reduced the sharpening exponent (pow 4 → 2) and further shader work is explicitly out of this beat's scope.

### 4.4 Director repro

`cargo editor` → Terrain panel → seed **12345**, radius **6**, then compare against T.2a's own frames (same station coordinates):

- **Mediterranean** — grassland is 91.3% of this world. The check is binary and needs no measurement: **the hard black shards scattered through the grass must be absent.** The ground now reads olive/brown rather than saturated green; that colour shift is the second thing to judge.
- **Desert** — the clearest improvement. A featureless tan wash becomes sand with visible surface relief.
- **Boreal** — snow has real surface structure at both overview and close-up, and is darker than before (§4.3).

Or reproduce the exact frames:

```
T2A_OUT=d:/tmp/t2c_staging/render T2A_LABEL=<leg> \
  cargo test -p aw_editor --profile release-fast --test terrain_ab_stations -- --ignored --nocapture
```

---

## 5. Verification ladder

| rung | result |
|---|---|
| API suitability (3 slugs, live) | **PASS** — type:1, CC0, 8K max, Diffuse+nor_gl+Rough+AO |
| Normal-map sanity check | **PASS** — all three genuine ground normals, no alpha, near-unit |
| Fetch | 3 assets, 5 maps each, 0 failed; licence summary "CC0-1.0 — 3 assets" |
| ATTRIBUTION merge | **18 → 21 slugs, zero lost** (5.C fix verified, not assumed) |
| Cook | 9 files, 1024² RGBA; **R metallic = 0.00 / sd 0.00** on all three |
| `channel_stats.py` | **0 flat channels**; slots 0/1/4 `real` on roughness and AO |
| `aw_asset_cli validate` | **9 passed / 0 failed** (mra warnings = ORM-vs-MRA naming + near-uniform heuristic) |
| `cargo fmt -p aw_editor` | clean |
| `cargo check -p aw_editor --tests` | **exit 0** (pre-existing warnings only) |
| `cargo test -p aw_editor --lib palette_remap` | **8 passed; 0 failed** (4,022 filtered) |
| `cargo test -p aw_editor --lib canonical_terrain_pack` | **2 passed; 0 failed** (4,028 filtered) |
| `cargo xtask gen-keeplist` | 22,593 entries, unchanged |
| `cargo xtask ci-guard` | **PASS** |
| Station render, both legs | 7 stations × 2 legs; control byte-identical |

**Rung 3 (the director's render verdict) is not claimed.**

---

## 6. Bookkeeping

- `docs/architecture/terrain_materials.md` → **v1.5** (slot 0/1/4 rows, new **Invariant 9** for the stem join, v1.5 addendum, `last_verified_commit`). The beat brief asked for "v1.3 → v1.4"; the trace was already at **v1.4**, because T.2a bumped it when it landed the Phase-1 channel repair. Bumped to v1.5 so the version still increases — flagged rather than silently overwriting a live version number.
- `THIRD_PARTY_LICENSES.md` → **§15** (three fresh provenance rows + rejected-candidate record).
- `assets/asset_manifest.toml` → T.2c block, handles `grassland` / `desert_sand` / `tundra_snow` (distinct from the pre-existing `snow` handle, which pins `snow_03` and is untouched).
- Out of scope and unchanged: forest, mountain, swamp, beach, river slots; all shader constants; scatter; water; `pbr_gen`; `arrays.toml`.

## 7. Residue / open items

- **The two art-direction findings in §4.3** (tundra darkness, hex lattice now visible) are the director's to rule on.
- **Station-validity rot is now a known hazard.** Station 05 is stale for its stated purpose and is kept only as a T.2a anchor. Any future A/B beat should re-survey before trusting pinned coordinates.
- The pre-T.2c `assets/materials/{grass,sand,snow}*.png` remain on disk and are now referenced by no pack layer, but `grass.png` is load-bearing as the `find_assets_dir()` sentinel (§3.3). If a later beat wants to retire them, the sentinel must move first.
- The fetched `_arm.png` files sit unused in gitignored `_downloaded/`; any future use must route through `cook_mra_arm_to_mra` (the guarded swap), per the AD.4 close-out lesson.
- `leafy_grass`'s flat shipped AO (§1.3) is an upstream data fact worth remembering if that slug is ever reconsidered — it needs AO derived from displacement.
