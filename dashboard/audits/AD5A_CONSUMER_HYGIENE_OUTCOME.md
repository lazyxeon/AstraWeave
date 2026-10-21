# AD.5.A Consumer Hygiene — Outcome Note

**Session:** AD.5.A (ratified at the AD.5 gate, director, 2026-07-08; executed 2026-07-15)
**Scope:** four consumer-side fixes — forest slot re-point, unified_showcase startup panic, S1 Option-C palette remap, provenance doc correction.
**Constraints honored:** zero LFS network ops, zero pushes (all commits local-only), zero asset-file changes (config/code/docs only — no image, zip, or release artifact was written).

---

## 1. Fix 1 — Forest slot re-point (closes AD.5 finding 5.1)

### 1a. The 13-ref ledger

AD.5's verify-assets counted 13 unmanaged `_downloaded` refs across 2 biome tomls on the rehearsal (fresh-clone-equivalent) tree. Enumerated:

| # | File:line (pre-fix) | Ref | Disposition |
|---|---|---|---|
| 1 | `assets/materials/biomes/materials.toml:47` | `../../_downloaded/polyhaven/forest_leaves/forest_leaves_albedo.png` | **RE-POINTED** → `../derived_1k/tree_leaves.png` |
| 2 | `assets/materials/biomes/materials.toml:48` | `../../_downloaded/polyhaven/forest_leaves/forest_leaves_normal.png` | **RE-POINTED** → `../derived_1k/tree_leaves_n.png` |
| 3 | `assets/materials/biomes/materials.toml:49` | `../../_downloaded/polyhaven/forest_leaves/forest_leaves_arm.png` (key `orm`) | **RE-POINTED** → `../derived_1k/tree_leaves_mra.png` (key **`mra`**) |
| 4–13 | `assets/materials/polyhaven/materials.toml:13,14,26,27,39,40,52,53,65,66` | `../../_downloaded/{aerial_rocks,metal_plate,plastered_wall,wood_floor,cobblestone}/*_{albedo,normal}.png` | **EXPLAINED, left as-is** (below) |

The current working tree shows only 10 warnings (refs 4–13) because the gitignored `_downloaded/polyhaven/forest_leaves/` fetcher output happens to exist locally; a fresh clone shows all 13. Refs 4–13 belong to `assets/materials/polyhaven/materials.toml` — a **consumer-less legacy demo biome pack**: the only code that ever loads it (`load_biome_toml` in `examples/unified_showcase/src/main_backup.rs` / `main_backup_before_refactor.rs`) is orphan source — the live `main.rs` declares only `mod gltf_loader` and never reads any biome toml. Re-pointing an unconsumed toml would be motion without effect; it is logged in §6 as AD.6 purge-inventory material instead.

### 1b. Provenance chain (content-equivalence of the swap)

- Old refs: `assets/_downloaded/polyhaven/forest_leaves/*` ← fetched from Poly Haven **`forest_leaves_02`** (E3 acquisition block, `assets/asset_manifest.toml` handle `forest_leaves`, id `forest_leaves_02`).
- New refs: `assets/materials/derived_1k/tree_leaves{,_n,_mra}.png` ← cooked by `tools/material_cook/cook_1k.py` from the AD.4 C7 re-acquisition, handle `tree_leaves` = Poly Haven **`forest_leaves_02`** (`assets/asset_manifest.toml` C7 block; `docs/audits/AD4_RECOOK_OUTCOME.md` ledger row 43: `tree_leaves | forest_leaves_02 | Rob Tuytel | forest`).

Same upstream ID on both sides → content-equivalent (1k cook of the same 2k source). The stale E3 comment in `asset_manifest.toml` (which described direct `_downloaded` consumption) was updated in the same beat.

### 1c. Channel-key correctness per line (the D1 lesson)

| Line | New file | Measured channels (mean) | Key | Why |
|---|---|---|---|---|
| albedo | `tree_leaves.png` | R=141.9 G=112.2 B=58.2 (sRGB color) | `albedo` | color map, unchanged semantics |
| normal | `tree_leaves_n.png` | R=128.9 G=129.2 **B=218.9** (+Z-dominant) | `normal` | tangent-space GL normal |
| orm→**mra** | `tree_leaves_mra.png` | **R=0.0** (metal) G=237.9 (rough) B=153.4 (AO) | **`mra`** | true-MRA (post-D1 cook; R=0.0 measured 2026-07-15) → must take the loader's R↔B swizzle path. The old `_downloaded` file was ARM/ORM-packed and correctly used `orm` (verbatim); pattern-copying that key onto the MRA file would have re-created D1. |

Tiling unchanged (`[128.0, 128.0]`).

### 1d. Verification

- **Load test** (extends the grassland-style loader test): new `loads_biomes_pack_forest_slot_from_derived_1k` in `tools/aw_editor/src/viewport/canonical_terrain_pack.rs` — loads the real pack from disk, asserts `active_layer_count == 8`, forest slot 2 albedo+normal+mra all decode, stem == `tree_leaves`, and the full 8-stem set. Output:
  `running 2 tests … test viewport::canonical_terrain_pack::tests::loads_biomes_pack_forest_slot_from_derived_1k ... ok … test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 4035 filtered out`
- **verify-assets after:** `169 references checked — all pack-managed references resolve (10 unmanaged warnings)` — zero forest_leaves refs remain (grep for `"_downloaded` path values in the biomes toml: no matches). Fresh-clone unmanaged count drops 13 → 10, remainder explained in §1a.
- **Baseline note:** this **retires the AD.4.A "forest = control" baseline** — the forest slot no longer renders the uncooked 2k `_downloaded` ARM set; it renders the same derived_1k cook family as the other C7 slots. Future render comparisons must not treat forest as the untouched control.

---

## 2. Fix 2 — unified_showcase startup panic (closes AD.5 finding 5.2)

### 2a. Quarantine check before path correction

The hardcoded dir `assets/textures/pine forest textures/` does not exist; the real dir is `assets/textures/pine_forest/`. Referenced files, checked against the AD.1.B disposition (`THIRD_PARTY_LICENSES.md` §11, quarantine row at line 243):

| Slug | Files used | Disposition | Action |
|---|---|---|---|
| `grass_medium_01` | `_diff/_nor_gl/_rough.png` | TRACED (AD.1-verified §3; API-200 capture `ad1c_residual_2026-07-06/polyhaven_api/grass_medium_01.json`) | path fix only |
| `rock_moss_set_01` | `_diff/_nor_gl/_rough.png` | TRACED (AD.1-verified §3 slug, `THIRD_PARTY_LICENSES.md:225`) | path fix only |
| `pine_bark` | `_diff.png` | TRACED (API-200, `ad1b…/polyhaven_api/pineforest_pine_bark.json`) | path fix only |
| `pine_twig` | `_diff.png` | **QUARANTINE** (404 slug, `THIRD_PARTY_LICENSES.md:243`) | **SUBSTITUTED** → `fern_02_diff.png` (traced foliage sibling: API-200, `pineforest_fern_02.json`) |

### 2b. Changes (`examples/unified_showcase/src/main.rs` only)

1. All 17 `"assets/textures/pine forest textures/"` refs corrected to `assets/textures/pine_forest/`; `pine_twig_diff.png` → `fern_02_diff.png` (PineLeaves material).
2. New `open_texture_or_fallback(path, label)`: missing/corrupt texture → loud `eprintln!` warning naming the path and pointing at `cargo run -p xtask -- fetch-assets --all`, plus a label-appropriate flat fallback (normal → `[128,128,255]`, rough → mid-grey, sky → sky-blue, albedo → neutral grey). **Sweep of same-class same-file offenders — all four panicking load sites rewired:** the `load_texture` closure (~line 924), the `load_terrain_texture` closure (~line 1168), `create_material_from_texture` (~line 1770), and the skybox loads (HDR `.expect` + PNG `.expect`, ~lines 790/826). No other `image::open` sites exist in the file. Not touched: `gltf_loader.rs` (different file — out of the ratified sweep scope).
3. Bonus same-class wrong-path fix: the skybox PNG fallback pointed at `assets/sky_equirect.png` (ships nowhere); corrected to the tracked `assets/hdri/sky_equirect.png`, making the degradation ladder (HDR → PNG → flat colour) real.

### 2c. Launch verification

`cargo run -p unified_showcase --release` equivalent (release binary, repo root cwd), 35 s run, killed externally. Log (236 lines): **0 WARNING lines** (every texture loaded for real — fallback present but not engaged), no panic, past both former panic sites:

```
=== MATERIAL CREATION START ===
  -> Grass material index: 0
  -> PineLeaves material index: 4        ← fern_02 substitute decodes
=== MATERIAL CREATION END ===
Terrain generation DONE
Starting Texture Load                    ← former line-1146 panic site
...
Floating island scene complete. Objects: 101   ← steady state
```

### 2d. AD.6 Tier-2 upgrade (recorded for the checklist)

**AD.6 Tier 2 upgrades from existence checks to LAUNCHING `unified_showcase`** on the post-rewrite `--all` clone: launch must reach `Floating island scene complete` with zero `WARNING: texture … unavailable` lines. On a Tier-1 (starter-profile) clone the same launch must reach steady state WITH fallback warnings and no crash.

---

## 3. Fix 3 — S1 Option-C palette remap (AD.4.A §5a, director-ratified)

### 3a. Join key (evidence-based, justified)

Join: **loaded layer's albedo source-file stem** (lowercase) ↔ **`MaterialLibrary` entry `name`**. Both sides are the same convention derived from data: the library `name` is *defined* as an asset-path stem (`assets/materials/{name}.png`, `astraweave-render/src/material_library.rs:47-56`), and the pack layer's albedo stem is the pack's material identity (its `key` field names the *biome slot* — "grassland", "desert" — not the material, so it cannot join). For 5 of the 7 matches the two sides literally denote the same file (`assets/materials/grass.png` etc.). No hardcoded index table exists to rot on pack change: `EngineRenderAdapter` re-resolves on **every** terrain-layer upload, pack swap included. Duplicate stems (desert/beach share `sand.png`) resolve to the lowest layer index — deterministic, and both layers are byte-identical file + tiling.

### 3b. Paintable entries against the current biomes pack (7 of 21)

| Palette ID | Library name | → pack layer | Layer key |
|---|---|---|---|
| 0 | grass | 0 | grassland |
| 1 | sand | 1 | desert (beach=6 is the same file; lowest index wins) |
| 3 | mountain_rock | 3 | mountain |
| 4 | snow | 4 | tundra |
| 5 | mud | 5 | swamp |
| 12 | gravel | 7 | river |
| 20 | tree_leaves | 2 | forest (**enabled by Fix 1** — the pre-fix stem `forest_leaves_albedo` matched nothing) |

The other 14 (forest_floor, wood_planks, stone, rock_slate, dirt, cobblestone, cloth, ice, metal_rusted, moss, plaster, rock_lichen, roof_tile, tree_bark) render disabled (dimmed thumbnail, not clickable, tooltip "*name*: not in the loaded biome pack"). This resolves the 0–7 mislabel: "Wood Planks" no longer silently paints beach sand — it is disabled; every enabled entry paints the material of its own name. Painting is a no-op (with a `tracing` warning) rather than a layer-0 collapse for any unresolved state.

### 3c. Implementation shape

- `viewport/palette_remap.rs` (new): `PaletteRemap::resolve(stems)` / `placeholder_identity(8)` / `layer_for` / `paintable_ids`; pure, 8 unit tests.
- `viewport/canonical_terrain_pack.rs`: `CanonicalLayerBytes.albedo_stem` retained at load (only when the albedo actually decoded — a failed decode must not advertise an identity).
- `viewport/engine_adapter.rs`: remap re-resolved in `reupload_terrain_layers_from_pending_pack` (canonical branch = stem join; placeholder branch = identity 0..8, which also retires the ≥8 collapse in placeholder mode); getter `palette_remap()`.
- `main.rs` `update()`: per-frame `try_lock` sync into the live `dock_tab_viewer.terrain_panel` (the dispatcher's structurally-distinct panel instance is inert per the Sub-phase 3 note — verified, not assumed).
- `panels/terrain_panel.rs`: both paint call sites (`apply_brush_at` + Apply Brush button) write the **remapped layer index**; the button disables with "Material not in loaded pack"; selection auto-moves to the first paintable entry on remap change.
- Trace updated in the same commit: `docs/architecture/aw_editor.md` v1.4 (§5 row, §8 invariant 26).

### 3d. Tests + adversarial verification

Resolver tests (match, no-match, duplicate-stem, pack-swap re-resolution, placeholder identity, out-of-range, unloaded-albedo skip, exact 7-entry biomes fixture):
`running 8 tests … test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 4029 filtered out`
Loader tests: `running 2 tests … ok. 2 passed` (§1d). `cargo check -p aw_editor` clean; `cargo clippy -p aw_editor` clean except the pre-existing documented gizmo unused-import warning.

Adversarial verification (independent fix-verifier agent, report-only): **GO — all 9 checks PASS**. Highlights: bypass hunt found exactly 2 `apply_brush_paint_material` callers, both remapped; the mediator drain (`main.rs:4024`) and the remap sync feed the **same** live panel instance; undo replays vertex snapshots, never raw IDs; the dispatcher's inert panel fails closed (remap `None`); pack-swap ordering is atomic at the adapter and ≤1-frame lagged at the panel, always a valid `[0,8)` write; `TerrainPanel` is not serialized (no prefs breakage); feature-off builds fail closed; the 7-entry table was **independently re-derived from on-disk assets and confirmed exactly**; splat builder + shader treat painted values consistently as layer indices. Full-crate regression net: `cargo test -p aw_editor --lib` → `4032 passed; 0 failed; 5 ignored`. Non-blocking observations: (i) transient paint-block window between terrain generation and first layer upload (fail-closed UX nit); (ii) beach layer 6 unpaintable by name — shares `sand.png` with desert, lowest index wins, visually identical (by design, documented); (iii) the bounded one-frame sync lag.

The paint-**correctness** claim itself (enabled entry paints its named material; disabled entries visibly grey) remains the **director's at gate** — rung-3 discipline.

Explicitly NOT done (per brief): Option A (21-layer loader) stays logged as a post-campaign engine item; no shader-constant changes; no S2 work; no VFS/loader architecture.

---

## 4. Fix 4 — Provenance doc correction (doc-only)

`assets/asset_manifest.toml`:

1. **Ice note corrected** (A.1.C block): the note claimed "Metallic channel synthesized as solid 0" under an `_mra` name; measured 2026-07-15: `ice_mra.png` R=255.0 (flat AO) / G=35.7 (rough) / B=0.0 (metal) — the composition wrote ARM/ORM order, inverting the documented convention. Correction appended in place, citing AD.4.A §5b.
2. **ARM-ORDER ANNOTATION block added**: all 9 `assets_src/materials/*_mra.png` files (cobblestone, gravel, ice, metal_rusted, moss, wood_planks + inert mountain_rock, mud, snow) annotated with freshly measured channel means (B≈0 metallic-in-blue signature on all 9) and the instruction that any future cook must route through the guarded `cook_mra_arm_to_mra()` swap. Sources left as-is per the §5b option (i) decision (the live `materials-src.zip` archives them faithfully; fixing sources would force a pack re-cut).

---

## 5. verify-assets before / after

| | Before | After |
|---|---|---|
| References checked | 169 | 169 |
| Pack-managed failures | 0 | 0 |
| Unmanaged warnings (this tree) | 10 (all `[biome:polyhaven]`) | 10 (all `[biome:polyhaven]`) |
| Unmanaged warnings (fresh clone) | 13 (3 forest_leaves + 10 polyhaven) | **10** (forest_leaves refs eliminated; remainder = consumer-less demo toml, §1a) |

---

## 6. Findings for the director (observed, not acted on)

1. **`assets/materials/polyhaven/materials.toml` is consumer-less** (only orphan `main_backup*.rs` reference the loader path). Candidate for the AD.6 purge inventory together with the orphan `examples/unified_showcase/src/main_backup*.rs` / `main_bevy*.rs` / `main_clean.rs` / `main_temp.rs` sources; that would take fresh-clone unmanaged warnings to 0.
2. **`cargo clippy -p aw_editor --all-features` fails upstream** (`egui-winit` E0027 `accesskit_update` pattern) — pre-existing, reproduced on the unmodified tree; default-features clippy is clean. An accesskit/egui feature-unification issue, not editor code.
3. **BiomePack ground-texture injection seam** (`main.rs` ~5580): when a decomposed scene BiomePack is loaded, its ground textures are injected into GPU layers 0–7 by name heuristics (and ≥12 for unmapped — dormant, beyond `active_layer_count`). This can replace canonical-pack layer *content* after the palette remap resolved against pack stems (identity drifts for scene-pack workflows). Not exercised by the canonical E3 flow; queued observation for the terrain-quality stream.
4. Fix 2's fallback intentionally reads as flat grey/blue, not magenta: the flagship should *degrade*, not scream, on a Tier-1 clone; the loud signal is the stderr warning naming the missing path.

## 7. Gate review repro (director)

1. `cargo editor` → generate/open terrain → **forest biome areas render the derived_1k tree_leaves cook** (Fix 1 visual; forest ≠ control anymore).
2. `cargo run -p unified_showcase --release` → launches past material creation, reaches "Floating island scene complete. Objects: 101" (Fix 2).
3. In the editor terrain panel, Paint mode: exactly the 7 entries of §3b are enabled; paint each → the material of that name appears; the 14 disabled entries are visibly dimmed with tooltip (Fix 3). Rung-3 render claims are the director's.

## 8. Commits (local-only, zero pushes)

| Hash | Content |
|---|---|
| `404710184` | fix(ad5a-1): forest slot re-point (biomes/materials.toml) |
| `6060878da` | fix(ad5a-2): unified_showcase graceful degradation + path/quarantine fixes |
| `dc32e37bf` | feat(ad5a-3): Option-C palette remap + tests + aw_editor trace v1.4 |
| `ec3cc8d91` | docs(ad5a-4): asset_manifest.toml ice correction + ARM-order annotation + E3 comment refresh |
| (this commit) | docs(ad5a): this outcome note |
