# T.1 — Distinct Beach Material (outcome)

> **Beat:** T.1 (terrain series, beat 2) · **Date:** 2026-07-21 · **Executes:** `T_SERIES_RATIFICATION_2026-07-20.md` §2 **row-6 amendment** (director, 2026-07-20)
> **Before:** slot 6 (beach) of `assets/materials/biomes/` shared the desert `sand.png` trio **byte-identically** (albedo/normal/mra all `../sand*`; `sand.png` sha256 `19f7242d…`).
> **After:** slot 6 = PolyHaven `coast_sand_01`, cooked to `assets/materials/derived_1k/beach{,_n,_mra}.png`.

## 1. Acquisition + provenance (Phase 1)

- **Suitability (binary PASS):** `api.polyhaven.com/info/coast_sand_01` (captured 2026-07-21): `type:1` texture, authors `{"Rob Tuytel":"All"}`, `max_resolution [8192,8192]`, CC0; `/files` capture: `Diffuse`/`nor_gl`/`Rough`/`AO`/`arm` each at 1k-8k (≥2K requirement met). Description "damp coastal sand with brown, rough grain, scattered pebbles and gravel" — the wet-sand read the amendment asked for.
- **Fresh provenance row:** `THIRD_PARTY_LICENSES.md` **§14** (the prior §11.1 `coast_sand_01_1k.glb` row traced the model wrapper; this acquisition got its own row per the AD.4 standard). API captures: `docs/audits/evidence/t1_beach_2026-07-21/polyhaven_api/`.
- **Fetch:** `tools/astraweave-assets fetch` (provider flow; one-entry session manifest mirroring the permanent `assets/asset_manifest.toml` T.1 block, to avoid re-downloading every polyhaven slug) → `assets/_downloaded/polyhaven/beach/beach_{albedo,normal,roughness,ao,arm}.png` (5 maps; the provider's "metallic not available" warning is expected — the slug ships no separate metallic map).
- **Known tool bug fired again:** `generate_attribution_file` OVERWRITES `assets/_downloaded/polyhaven/ATTRIBUTION.txt` (148→22 lines; the AD.4 finding, tool still unfixed). Restored from the committed version + beach entry appended: now 20 entries / 155 lines, tail note documents the second occurrence. (First restore attempt via a script corrupted the file — recovered via `git checkout` + manual Edit; the committed diff is the clean union.)

## 2. Cook + install (Phase 2)

- **Cook:** `cook_1k.py::cook_family_from_maps` (the AD.4 C7 path — MRA **packed from rough+ao**, so the ARM-order trap is avoided by construction; the fetched `beach_arm.png` is not used by the cook):
  - `beach.png` 1024×1024 RGBA 2,315,291 B · `beach_n.png` 1024×1024 RGBA 2,688,812 B · `beach_mra.png` 1024×1024 RGBA 1,513,483 B.
- **Channel measurements (the trap check):** `beach_mra.png` means **R=0.0** (metallic — true-MRA confirmed) / G=245.4 (roughness) / B=240.4 (AO). Albedo means R=129.7/G=114.3/B=91.4 (warm damp brown — visibly darker/browner than the pale desert dune sand). Normal means R≈128/G≈127/B≈248 (proper tangent-space map).
- **Install:** `assets/materials/biomes/materials.toml` slot 6 (key `beach`) → the derived_1k trio, `mra` key (R↔B-swizzled to ORM at load per `canonical_terrain_pack.rs:180-184`), tiling `[128,128]` (consistent with the other ground slots; integer per the pack's seam rule). **No slot reordering** — `arrays.toml` untouched.
- **Validation:** `aw_asset_cli validate` per file: `beach.png` Passed 1/Failed 0/Warnings 0 · `beach_n.png` 1/0/0 · `beach_mra.png` 1/0/**Warnings 2** ("R channel (Occlusion) appears unused", "G channel (Roughness) appears unused" — the validator's near-uniform-channel heuristic on a non-metal, uniformly-rough material; the live slot-7 `gravel_mra.png` produces the same warning class, cited for comparison).

## 3. The three pre-armed traps (Phase 3)

1. **Keeplist / ci-guard:** `cargo xtask gen-keeplist` → 22,593 cohabitant entries. The regen **dropped 612 stale entries** — every one verified to reference a path that no longer exists (the AD.6-rewrite-purged keeplist∩purge residue the close-out §2 noted); zero of the 612 exists on disk (scripted existence check over the full deletion set). The three beach files are **sample-set files outside cohabitant scope** (that is why the keeplist does not list them — same as every other `derived_1k/` file). `cargo xtask ci-guard`: **PASS** — "0 tracked pack members, 0 stray blobs under managed roots (22593 keeplist cohabitants); ignore surfaces match" (re-run after `git add` with the new files tracked: see §5).
2. **Loader test:** `canonical_terrain_pack.rs` full-stem assertion updated (slot 6 `"sand"` → `"beach"`). `cargo test -p aw_editor --lib canonical_terrain_pack` → **`2 passed; 0 failed`** (4,035 filtered out).
3. **Palette-remap shift:** the live-pack mirror fixture (`biomes_pack_stems()`) updated; with `beach` having no `MaterialLibrary` name (21-name set verified — no "beach"), slot 6 is **biome-paint-only by design**, `sand` now maps uniquely to slot 1 (same index as the old lowest-of-{1,6} rule), and the paintable set stays **7 entries** `[0,1,3,4,5,12,20]`. The duplicate-stem RULE remains covered by the synthetic fixture (`duplicate_stem_resolves_to_lowest_layer_index`, untouched). `cargo test -p aw_editor --lib palette_remap` → **`8 passed; 0 failed`** (4,029 filtered out).

## 4. Editor observation (Phase 4) — director repro

Editor launched from the unchanged release-fast binary (only `#[cfg(test)]` code + runtime data changed this beat) with the new pack data; three worlds generated at **seed 12345, chunk radius 6 (169 chunks / 1,622,400 vertices, ~57 FPS)**. Screenshots in `d:/tmp/t1_staging/` (session-local, not committed):

- `04_ct_overview.png` — Continental Temperate: litter/grass/snow as before; **no coastline in the spawn-camera frame** (beach rims low-continentalness basins; none in view from the fixed overview).
- `08_desert_overview.png` — Desert: pale dune plateau + barren ridges, **visually unchanged from the pre-T.1 E3-PF screenshot** (slot 1 untouched — scope evidence); no basin in frame.
- `10_mediterranean.png` — **Mediterranean (the useful frame)**: green lowlands with **scattered dark pebbled-sand patches at low-elevation shore transitions** (foreground and mid-frame), visually consistent with the new beach material and unambiguously different from the Desert world's pale sand. Pre-T.1, those patches rendered in the byte-identical desert sand.
- `07_texture_side_by_side.png` — texture-level evidence: desert `sand.png` (pale smooth ridged dunes, means R=199.2/G=178.6/B=142.5) vs `derived_1k/beach.png` (dark damp pebbled ground, means R=129.7/G=114.3/B=91.4).

**Director repro (the render verdict is yours — this note claims no rung 3):** `cargo editor` → Terrain panel → seed 12345, radius ≥6 → **Mediterranean** (its splines are the "pronounced coastline" set — fastest coastline density) → look at the low flat patches rimming depressions in the green lowlands: they should read as dark damp pebble-sand (beach), clearly not the pale dune sand (switch to Desert for the direct comparison; CT works too but needs flying to a basin). The check: **beach reads distinctly from desert at a coastline.**

## 5. Commits + verification ladder

Single beat commit (hash cited in the session report + memory): manifest entry, attribution union, `materials.toml` slot 6, 3 derived_1k files, keeplist regen, `THIRD_PARTY_LICENSES.md` §14, evidence dir, this note, trace v1.3, 2 test-fixture files. Ladder, all cited above: API suitability PASS → fetch 5 maps → cook (mra R=0.0 measured) → `aw_asset_cli validate` 3/3 pass → `ci-guard` PASS (re-run with files tracked: "0 tracked pack members, 0 stray blobs under managed roots (22593 keeplist cohabitants)") → loader suite `2 passed; 0 failed` (4,035 filtered) → resolver suite `8 passed; 0 failed` (4,029 filtered) → editor observation (§4). Rung 3 (director render check) closes the beat.

## 6. Bookkeeping

- `docs/architecture/terrain_materials.md` → **v1.3** (slot-6 row: executed, provenance pointer, biome-paint-only note).
- The ratification record stands unmodified; this outcome note is its row-6 execution record.
- Out of scope, unchanged: desert slot, all other materials, water (T.W-R next), the release/packs, MaterialLibrary.

## 7. Residue / open items

- The `generate_attribution_file` overwrite bug has now fired twice (AD.4, T.1) — the tool fix remains an open hygiene item (close-out §4 item family; candidate for T.2 or the CI-workshop's tooling slot).
- The fetched `beach_arm.png` (ORM-packed) sits unused in gitignored `_downloaded/`; if a future cook wants it, it must route through `cook_mra_arm_to_mra` (guarded swap) — per-file channel evidence first (close-out lesson 4).
