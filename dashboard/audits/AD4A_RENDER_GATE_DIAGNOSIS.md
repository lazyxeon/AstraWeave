# AD.4.A — Render-gate defect diagnosis (S1/S2/S3) + evidenced fixes

**Date:** 2026-07-07  **Branch:** `campaign/roadmap`  **Trigger:** director's rung-3 render review REJECTED (live editor, 2026-07-07).
**Method:** diagnose-before-fix. Six parallel forensic tracks (load-path, palette-mapping, ktx2-archaeology, channel-stats, normal-convention, toml-key-mapping) + a GPU-rendered pre/post baseline (Phase 3), all evidence cited before any fix was written. Fixes commit: `34973de63`. Zero LFS network ops; zero pushes; diagnostic artifacts in `d:/tmp/ad4a_staging/` (outside the tree).

---

## 1. Causal chains (each symptom → evidenced root cause)


### S1 — "Painting rock_lichen, cobblestone, OR roof_tile renders the SAME broken green-black geometric texture; thumbnails fine"

**PRE-EXISTING wiring defect. Not caused by AD.4 — exposed by the AD.4 review.** Chain (every link first-hand verified):

1. The paint palette is `MaterialLibrary::MATERIALS` — 21 named IDs (`material_library.rs:74+`): cobblestone=**10**, rock_lichen=**17**, roof_tile=**18**.
2. Paint writes the raw palette ID into terrain vertices; the splat builder identity-maps material_id N → splat channel N → shader layer N (`terrain_splat_builder.rs:88-100`; IDs ≥32 dropped). Paint does NOT collapse the three — they stay distinct channels.
3. The live terrain GPU array is populated ONLY from the 8-layer `assets/materials/biomes/` pack — the sole `set_biome_pack(Some(..))` call (`main.rs:5460-5470` → `engine_adapter.rs:1392-1452` → `renderer.rs:6190 set_terrain_materials`). Painting never re-populates it.
4. Layers 8–31 keep `TerrainLayerGpu::default()` = `texture_indices=[0,0,0,0]`, `uv_scale=[1,1]` (`terrain_material.rs:44-55`). The shader picks its albedo array slice from `texture_indices.x` (`pbr_terrain_forward.wgsl:246`).
5. → Painting ID 10, 17, or 18 all sample **albedo slice 0 = grass** at 1× tiling (vs the biome layer's 128×) — one grass image stretched ~128× across the chunk, run through the hex-tile per-cell rotation + pow-4 weight sharpening: **the identical green-black geometric texture, for all three.**
6. Thumbnails load a DIFFERENT file by a different path — root `assets/materials/{name}.png` at 64² (`terrain_panel.rs:1970-2001`), all of which exist — hence plausible thumbnails. A §7.7-class UI-identity vs renderer-identity divergence.

`material_library.rs`'s own docs corroborate: `:61-64` "reserved IDs render as the fallback (layer 0)" (the same fallback silently applies to *named* IDs 8–20, which the UI does **not** hide), and `:51-52` documents a root-file texture-loader contract that **no live code implements**. Real-Fix.D (2026-05-08) unified the *capacity* boundary (UI=22/splat=8/renderer=8 → 32) but never wired *content* for IDs 8–20.

**Latent bonus defect (same root):** the library's ID 0–7 identity (`grass,sand,forest_floor,mountain_rock,snow,mud,wood_planks,stone`) does not match the live biomes-pack slot content (`grassland,desert,forest,mountain,tundra,swamp,beach,river`): painting "Wood Planks"(6) paints beach sand; "Stone"(7) paints river gravel.

**Prediction the director can verify:** painting ANY palette ID ≥ 8 (tree_bark, plaster, gravel-by-name…) produces the same green-black result; painting IDs 0–5 paints the correct biome layers.

### S3 — "resembles channel-packed data displayed as color"

Explained by S1: the pixels are **stretched grass albedo** (green + dark soil structure), not MRA-as-albedo. Exculpatory evidence: the three families' actual `_mra` files render cyan/teal/orange as raw RGB — three different colors cannot produce one identical image.

### S2 — "all biomes acceptable at distance, wrong up close ('eyes hurt'), including traced-9"

**PRE-EXISTING (~92% confidence) — newly noticed, not newly caused.** Sealed by the Phase-3 GPU baseline (§3):

- Zero code changed `098be6e0c..HEAD` anywhere in the loader/formats/shader chain (git-verified).
- Runtime sampling resolution is UNCHANGED both eras (albedo@1024², normal+ORM@512² — the loader clamps regardless of disk size; the 2048→1024 recook does not halve runtime detail).
- The rendered pre-vs-post close-up difference (excluding the D1 river slot) is **max 5/255, 0.00% of pixels >8, frame Laplacian equal** — sub-perceptual. The pre-AD.4 assets render the same frame that "hurts".
- Prime pre-existing suspect (for a future render-quality beat): E3's `NORMAL_XY_STRENGTH=1.8` mip0 boost + hex-tile pow-4 rotation (`pbr_terrain_forward.wgsl:253-333`) — mip-gated (box-blurred far, full-strength up close), uniform across biomes; matches the symptom signature exactly. Plus 5–6 of 9 traced-9 materials have flat/degenerate roughness/AO data (pre-existing placeholders), giving uniform specular response up close.

### D1 — AD.4-ACTIVATED channel inversion (found during diagnosis; FIXED)

The 6 C6 `assets_src/materials/*_mra.png` are physically **ARM-ordered** (R=AO high, B=metal 0) — a 2026-05 A.1 acquisition artifact (PolyHaven ARM maps renamed `_mra.png` without channel reorder). `cook_1k.py` resized them verbatim into `derived_1k/`. The loader's `mra`→ORM swizzle (`canonical_terrain_pack.rs:204-210`) assumes true MRA (its doc-comment verified R=0 against the *root* files, which ARE true MRA) and swaps R↔B — double-flipping ARM input so the shader reads **AO≈0 (ambient killed) + metallic≈95–100% (mirror)**. Dormant pre-AD.4; activated by the AD.4 re-point on 8 toml lines: gravel (beach, biomes/river, mountain, river, tundra), moss (river, swamp), ice (tundra). Measured: `derived_1k/gravel_mra` R=247.1/B=0.0 vs root control `gravel_mra` R=0.6/B=202.8. Phase-3 render confirms: river slot ORM diff mean 152.6, 72% of close-up pixels changed.

### D2 — AD.4-INTRODUCED cook bug (found during diagnosis; FIXED)

`pack_mra()`'s `.convert("L")` on 16-bit (`I;16`) PolyHaven sources **clamps to flat 255**: destroyed `plaster_mra` AO (`plaster_ao.png` = I;16) and `tree_bark_mra` roughness + AO (`tree_bark_roughness.png` and `tree_bark_ao.png` both I;16). Reproduced byte-for-byte (Pillow 12.3.0).

---

## 2. The resolved-path table (what the live editor actually loads)

The live editor terrain array is ALWAYS `assets/materials/biomes/` (8 layers; grassland is loaded only by unit tests). Loader semantics: albedo → 1024² (Triangle if resizing), normal + mra/orm → 512²; `mra` key → R↔B swizzle to ORM; `orm` key → verbatim; missing file → silent `None` → grey/flat/neutral defaults.

| idx | slot | albedo | normal | orm source | AD.4 changed? |
|--|--|--|--|--|--|
| 0 | grassland | `../grass.png` | `../grass_n.png` | `mra ../grass_mra.png` | recook-in-place (bytes) |
| 1 | desert | `../sand.png` | `../sand_n.png` | `mra ../sand_mra.png` | recook-in-place |
| 2 | forest | `../../_downloaded/…/forest_leaves_albedo.png` | …`_normal` | `orm …_arm.png` (verbatim, correct) | **no (control)** |
| 3 | mountain | `../mountain_rock.png` | `../mountain_rock_n.png` | `mra ../mountain_rock_mra.png` | recook-in-place |
| 4 | tundra | `../snow.png` | `../snow_n.png` | `mra ../snow_mra.png` | recook-in-place |
| 5 | swamp | `../mud.png` | `../mud_n.png` | `mra ../mud_mra.png` | recook-in-place |
| 6 | beach | `../sand.png` | `../sand_n.png` | `mra ../sand_mra.png` | recook-in-place |
| 7 | river | `../derived_1k/gravel.png` | `…gravel_n.png` | `mra …gravel_mra.png` | **re-point (D1 hit; now fixed)** |
| 8–31 | — | grey 128 default | flat | neutral | `TerrainLayerGpu::default()` → S1 |

The live path reads ONLY `.png`. The 72 malformed `baked/*.ktx2` ("AW_TEX2", committed once 2025-10-07 `049bfe0a`-era WIP baker, never read since) are **affirmatively dormant** — most dispositive: `cobblestone` has zero `.ktx2` anywhere, so its S1 symptom categorically cannot be ktx2-caused.

## 3. Phase-3 baseline (pre-AD.4 `098be6e0c` vs HEAD, GPU renders)

Instrument: temp offscreen-render test (deleted after; tree clean), NVIDIA GTX 1660 Ti/Vulkan, `1 passed; 0 filtered out`, 6/6 renders. Pre-AD.4 pack reconstructed from the **local** LFS store (pointer→`.git/lfs/objects`, no network). Artifacts: `d:/tmp/ad4a_staging/phase3/{layers,renders,diffs}/`.

- Per-layer loader-output diff: forest control **byte-identical** (instrument valid); traced-9 layers differ subtly (offline-LANCZOS vs in-loader-Triangle: albedo mean diff 0.3–2.4/255); river differs massively (D1, predicted).
- Rendered frames: far max 1/255; close-up (layers 0–3) max **5/255**, 0.00% px >8; close-up (layers 4–7, incl. river) 72% px changed — **all of it D1**.
- Sharpness: HEAD mip0 albedo buffers carry 2.0–2.8× more high-frequency energy (LANCZOS), but rendered frames are Laplacian-equal (HEAD marginally softer). The LANCZOS normal-map unit-length degradation (grass_n 6.99%→11.14% out-of-range) is real but not visually material → recorded as a quality footnote, no re-cook warranted.

**Verdict: S2 pre-existing; the only material AD.4 render delta was D1 (now fixed).**

## 4. Fixes applied (commit `34973de63`) — each traceable to its evidenced cause

| fix | cause | change | post-fix evidence |
|--|--|--|--|
| D1 | ARM-ordered C6 sources × loader swizzle | `cook_mra_arm_to_mra()` (guarded R↔B swap; refuses non-ARM-profile input) + re-cooked all 6 C6 `derived_1k/*_mra.png` | 12/12 derived_1k mras true-MRA (R=0.0 on every file); swap guard engaged 6/6 |
| D2 | PIL `I;16` → `convert("L")` clamp | `to_l8()` bit-depth-safe conversion (÷257 before L) + re-cooked plaster/tree_bark mras | plaster AO recovered (sd 10.9); tree_bark roughness recovered (248.2±3.7); tree_bark AO source-genuinely flat white |
| tests | regression coverage for both | `test_pack_mra_16bit_safe` + `test_arm_to_mra_swap_and_guard` | 3/3 pass |

Re-verification: decode floor `aw_asset_cli validate derived_1k` **36/36 pass**; grassland load test `running 1 test … 1 passed; 0 failed; 4027 filtered out` (filter matched 1 — not an empty-filter pass).

**Deliberately NOT changed** (no speculative hardening): traced-9 normals (Phase-3: not visually material), the E3 shader constants, the palette wiring (architectural — §5), `assets_src` (§5), the ktx2 files (dormant, still an AD.4 outcome-note open item).

## 5. STOP — architectural decisions for the director

**5a. S1 fix — palette↔array wiring (choose one):**
- **Option A — implement the library's documented contract:** load layers 8–20 from root `assets/materials/{name}*.png` (the loader `material_library.rs:51-52` promises but nothing implements), and reconcile the ID 0–7 identity mismatch. Cost: medium engine change (engine_adapter/canonical_terrain_pack), +13 layers of GPU memory; makes all 21 palette entries actually paintable; needs the ID 0–7 naming conflict resolved (library names vs biome slots).
- **Option B — clamp the palette to loaded content:** UI shows only the 8 biome-pack slots (with slot-true names). Cost: small UI change; honest UX; loses (never-working) paintability of the other 13.
- **Option C — name-based remap at paint time:** map palette IDs to pack layer indices by name where a match exists, grey out the rest. Cost: small-medium; keeps the 21-ID identity but only biome-matching entries paint.

**5b. `assets_src` ARM-mislabel + shipped `materials-src` cascade:** `assets_src/materials/{cobblestone,gravel,ice,metal_rusted,moss,wood_planks,mountain_rock,mud,snow}_mra.png` carry ARM order under an `_mra` name (9 files; the last 3 are inert — nothing points at them). The **live** `materials-src.zip` on `assets-v1` archives these mislabeled sources (the archive is faithful to the tree; the defect is in the tree). Options: (i) leave + document (the derived_1k consumers are now fixed; the pack is a source archive), or (ii) fix `assets_src` channel order + re-cut/re-upload `materials-src` (new sha256, manifest pin update, re-loop). Recommend (i) now, (ii) folded into any future pack revision.
- Also: `assets_src/materials/ice_mra.png`'s R=flat-255 contradicts its provenance note (`asset_manifest.toml:148-152` claims "metallic synthesized as solid 0") — the A.1.C composition inverted the channel order it documented.

**5c. S2 queued render-quality item (pre-existing, not AD.4):** investigate `NORMAL_XY_STRENGTH=1.8` mip0 boost + hex-tile rotation aggressiveness + per-chunk tiling density, and the 5–6 traced-9 materials with flat roughness/AO placeholder data. Separate beat, director priority call.

## 6. Fresh rung-3 render-review repro (the gate re-runs)

1. `cargo editor` → terrain: verify the **river/gravel** areas are no longer dark-mirror (D1 fix; AO restored, metallic 0) — this is the biggest visible change.
2. Desert (plaster occlusion now real — D2), swamp/river (moss), tundra (ice: brighter, non-metallic).
3. **S1 verification, not a fix**: painting rock_lichen/cobblestone/roof_tile will STILL show the stretched-grass result — that defect is pre-existing palette wiring, awaiting the §5a decision. Painting IDs 0–5 (grass/sand/forest/mountain/snow/mud names) paints correctly.
4. S2 ("eyes hurt" up close) is expected to persist — Phase-3-proven pre-existing; queued per §5c. Comparison renders: `d:/tmp/ad4a_staging/phase3/renders/`.

**AD.4 remains open until this review passes.**
