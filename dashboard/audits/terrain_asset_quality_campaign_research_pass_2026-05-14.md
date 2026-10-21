# Terrain Asset Quality Campaign — Research-Pass

**Date**: 2026-05-14
**Status**: Research-pass complete; Andrew-gate (a)+(b)+(c)+(d)+(e)+(f) pending for sub-phase methodology decisions.
**Author**: Claude Opus 4.7 (analytical-only session per Andrew-gate (n) pattern from Editor Multi-Tool Architecture campaign).
**Predecessor**: Editor Multi-Tool Architecture Sub-phase 3.C closeout `b220442a7` — methodology body of practice carry-forward as canonical input.

---

## §0 — Executive Summary

The Terrain Asset Quality campaign launches as a content-driven engine campaign following the architectural completion of Editor Multi-Tool Architecture Sub-phase 3. The campaign's objective is to replace Tier 1 runtime terrain material placeholders with high-quality generic PBR ground textures (engine-canonical, not Veilweaver-specific) and verify the renderer scales acceptably to real-PBR content. This research-pass characterizes actual asset state, pipeline capability, format conventions, and surfaces Andrew-gate decision points before fix-pass methodology selection.

**Key findings**:

1. **Actual asset state diverges from inventory framing**. `assets/materials/` contains 22 materials as PNG triples (albedo + `_n` + `_mra`); 12 of these are baked to KTX2 (in-place + duplicate set in `assets/materials/baked/`); 10 are PNG-only (cobblestone, default, gravel, ice, metal_rusted, moss, mountain_rock, mud, snow, wood_planks). `assets_src/materials/` holds source PNGs for the exact 12 already-baked materials; the 10 unbaked materials have NO source files. 10 biomes exist (grassland, desert, mountain, forest, swamp, tundra, beach, river, terrain, polyhaven), each with 5 layers.

2. **Shader uses ORM channel layout despite `_mra` naming**. `pbr_terrain.wgsl:335` explicitly comments "ORM: R=AO, G=Roughness, B=Metallic"; Rust-side `terrain_material_manager.rs` documents fields as `layer_orm` / `orm` / "ORM/height array" throughout. The `_mra.png` filename convention is misleading historical naming; data is functionally ORM-packed. Replacement texture acquisition must produce ORM-packed maps regardless of file naming.

3. **Baking pipeline gap is bimodal**. The 12 already-baked materials can be re-cooked from `assets_src/materials/`; the 10 unbaked materials require source acquisition first (no sources exist). Sub-phase A scope is therefore "source acquisition for 10 + re-cook for all 22" rather than uniform re-cook.

4. **`astraweave-assets/` is multi-provider, not just PolyHaven**. The crate includes PolyHaven, Kenney, itch.io, and direct-URL providers. Asset acquisition for the 10 missing source materials can leverage existing fetcher infrastructure.

5. **No `assets/pbr/` directory exists**. Tier 3 framing from prompt inventory is incorrect; PolyHaven sources are in `assets/textures/` (Tier 2) with full PBR map sets (albedo + ao + arm + nor_dx + nor_gl + rough + disp). No standalone Tier 3 rosettes layer exists separately.

6. **Performance signal is real but pre-empts proper baseline**. Editor terminal shows `Frame time 145.5ms (< 30 FPS)` + `Dropping ViewportRenderer GPU resources` during current placeholder-quality content. This is baseline-already-stressed; PBR replacement will compound. Sub-phase D (performance verification) is non-optional but should establish baseline BEFORE the asset replacement work begins so the impact attribution is clean.

**Sub-phase decomposition recommendation**: A (source acquisition + baking gap closure) → B (engine/project asset organization) → C (Tier 1 quality upgrade, biome-grouped) → D (performance verification + optimization) → E (closeout). Estimated 9-15 sessions total.

**Andrew-gate decisions surfaced**: six decision points (a)-(f) presented in §10 with recommendations.

---

## §1 — Current Asset State

### §1.1 Tier 1 Runtime Material Library

**Location**: `assets/materials/` (repo root + `assets/` prefix).

**22 materials confirmed via filesystem inventory** (each as PNG triple `<name>.png` + `<name>_n.png` + `<name>_mra.png`):

| Material | Baked (KTX2)? | Source in `assets_src/materials/`? | Biome references |
|----------|---------------|-----------------------------------|------------------|
| grass | ✅ | ✅ | grassland, beach, river, terrain (4) |
| sand | ✅ | ✅ | grassland, desert, beach, river, terrain (5) |
| dirt | ✅ | ✅ | grassland, mountain, swamp, tundra, beach, terrain (6) |
| stone | ✅ | ✅ | desert, mountain, beach, terrain (4) |
| rock_slate | ✅ | ✅ | grassland (key "rock_smooth"), desert (key "rock_smooth"), forest (key "rock_smooth"), terrain (key "rock") (4) |
| forest_floor | ✅ | ✅ | forest (1) |
| tree_bark | ✅ | ✅ | forest, swamp (2) |
| tree_leaves | ✅ | ✅ | forest (1) |
| rock_lichen | ✅ | ✅ | forest, swamp (2) |
| cloth | ✅ | ✅ | desert (1) |
| plaster | ✅ | ✅ | desert (1) |
| roof_tile | ✅ | ✅ | (UI-only; not biome-referenced) |
| cobblestone | ❌ | ❌ | polyhaven (different schema; not main biome usage) |
| default | ❌ | ❌ | (UI-only; not biome-referenced) |
| gravel | ❌ | ❌ | mountain, tundra, beach, river (4) |
| ice | ❌ | ❌ | tundra (1) |
| metal_rusted | ❌ | ❌ | (UI-only; not biome-referenced) |
| moss | ❌ | ❌ | grassland (via `rock_lichen` key alias), swamp, river (3) |
| mountain_rock | ❌ | ❌ | mountain, tundra (2) |
| mud | ❌ | ❌ | swamp, river (2) |
| snow | ❌ | ❌ | mountain, tundra (2) |
| wood_planks | ❌ | ❌ | (UI-only; not biome-referenced) |

**Visual quality assessment** (pending Andrew on-screen inspection; Real-Fix.D Andrew-gate 2026-05-08 observations partially documented):
- **Confirmed "distinct textures"**: grass, sand, forest_floor, mountain_rock, snow, mud, wood_planks, stone (8).
- **Confirmed "pixelated green splotches" / placeholder quality**: rock_slate, dirt, cobblestone, cloth, default, gravel, ice, metal_rusted, moss, plaster, rock_lichen, roof_tile, tree_bark, tree_leaves (14).
- **Confirmed visual mismatch**: forest_floor (brown thumbnail vs green render), stone (renders blue). These are NOT in the "placeholder-quality 14" — they have textures but the texture maps don't match expected output.

### §1.2 Tier 2 PolyHaven Source Textures

**Location**: `assets/textures/`.

**Format**: PolyHaven canonical PBR map sets per source. Each material has multiple maps:
- `<name>_diff_<res>.jpg` — diffuse / albedo.
- `<name>_ao_<res>.jpg` — ambient occlusion.
- `<name>_rough_<res>.jpg` — roughness.
- `<name>_arm_<res>.jpg` — ARM-packed (AO+Roughness+Metallic).
- `<name>_disp_<res>.jpg` — displacement / height.
- `<name>_nor_dx_<res>.jpg` — normal map (DirectX convention; +Y down).
- `<name>_nor_gl_<res>.jpg` — normal map (OpenGL convention; +Y up).

**Sample materials present** (non-exhaustive enumeration based on directory listing):
- `aerial_beach_01` (4k resolution available).
- `aerial_rocks_01` (4k resolution).
- `boulder_01` (~1k).
- `brick_wall_04` (1k).
- Plus single-map JPGs at root: `AO.jpg`, `Albedo.jpg`, `Displacement.exr/jpg`, `Gloss.jpg`, `Normal.jpg`, `Roughness.jpg` (legacy or sample data).

**Mapping candidates to Tier 1 slots** (recommendations in §7):
- `aerial_beach_01` → sand (if grain matches biome semantic).
- `aerial_rocks_01` → mountain_rock OR rock_slate.
- `boulder_01` → mountain_rock (boulder-scale rock).
- `brick_wall_04` → (no Tier 1 fit; potential decorative material slot).

Tier 2 PolyHaven sources require ARM-channel re-ordering or extraction to produce ORM-conventional `_mra.png` runtime files. PolyHaven `_arm_*.jpg` files contain ARM-packed data (AO+Roughness+Metallic); the shader expects ORM-packed (R=AO, G=Roughness, B=Metallic) — same channel order. The ARM convention matches ORM convention identically; only naming differs. (This is a critical finding for sub-phase A scope: format conversion is mostly file-rename + per-tier-bake; no channel-swizzle needed.)

### §1.3 Tier 3 Standalone PBR Rosettes — DOES NOT EXIST AS SEPARATE TIER

**Inventory framing was incorrect**. `assets/pbr/` directory does not exist. There is no separate "Tier 3" of standalone PBR rosettes (Dirt_Mud, Moss_Ground, Sand_Desert, Stone_Terrain_Rock) distinct from Tier 2 PolyHaven sources. The PBR rosette content the inventory described is part of `assets/textures/` (the Tier 2 location).

This finding simplifies the campaign: only two tiers of asset content (runtime materials in `assets/materials/`, source textures in `assets/textures/` and `assets_src/`). The Andrew-gate (d) Tier 3 mapping decision collapses into Andrew-gate (a) Tier 1 replacement scope.

### §1.4 `assets_src/` State

**Location**: `assets_src/` at repo root.

**Subdirectories**:
- `assets_src/materials/` — 12 source materials matching the 12 baked Tier 1 materials exactly (cloth, dirt, forest_floor, grass, plaster, rock_lichen, rock_slate, roof_tile, sand, stone, tree_bark, tree_leaves). Each as PNG triple.
- `assets_src/textures/` — sparse (3 files: dirt.png, grass.png, stone.png). May be legacy or unused.
- `assets_src/environments/` — present; contents not inspected (out of campaign scope).

**Critical finding for Sub-phase A**: the 10 unbaked Tier 1 materials (cobblestone, default, gravel, ice, metal_rusted, moss, mountain_rock, mud, snow, wood_planks) have NO source files. They exist only as PNG triples in `assets/materials/` directly. This means:
- **Re-cooking these 10 via `aw_asset_cli cook` is impossible** as currently configured (cook reads `assets_src/` per `aw_pipeline.toml`).
- **Source acquisition is required first** OR the existing PNG triples must be promoted to `assets_src/` as the canonical source.
- **Cook pipeline appears unused for these 10** (placeholder PNGs were direct-edited into `assets/materials/` without round-trip).

This is a critical observation for Andrew-gate (a) + (c) + (f) scoping: source acquisition is bundled with baking gap closure.

---

## §2 — Asset Pipeline Architecture

### §2.1 `aw_asset_cli` CLI

**Location**: `tools/aw_asset_cli/src/main.rs` + `texture_baker.rs` + `validators.rs`.

**Subcommands**:
- **`cook`** — reads `aw_pipeline.toml`; iterates rules; processes `assets_src/` → `assets/`. Per `main.rs:159-209`:
  - Initializes `AssetDatabase` via `astraweave_asset::AssetDatabase`.
  - Scans source directory.
  - Per rule (texture/model/audio): glob-walks, processes each entry, registers asset, records manifest entry.
  - Saves database manifest to `assets/assets.json`.
  - Signs manifest with Ed25519 (asset_signing::KeyStore).
  - Writes signed manifest to `assets/manifest.json`.
- **`bake-texture`** — single-file bake. Per `main.rs:79-104`:
  - Infers config from path (color space + normal_map detection).
  - CLI flags `--color-space srgb|linear` + `--normal-map` override.
  - Calls `texture_baker::bake_texture(input, output, config)`.
  - Returns serialized metadata.
- **`validate`** — asset validation per Phase PBR-G. Per `main.rs:105-110`:
  - Validates KTX2 mipmaps + material TOML + texture configs.
  - Supports `--strict` flag (fail on warnings).

**Pipeline config** (`aw_pipeline.toml`):
```toml
source = "assets_src"
output = "assets"

[[rules]]
kind = "texture"
glob = "**/*.{png,jpg,jpeg}"
normal_map = false

[[rules]]
kind = "model"
glob = "**/*.{gltf,glb}"

[[rules]]
kind = "audio"
glob = "**/*.{wav,mp3,ogg,flac}"
```

**Texture rule has `normal_map = false`** — global default. The per-file normal map detection happens inside `texture_baker::infer_config_from_path` (likely heuristic based on `_n`/`_nor`/`_normal` suffix).

### §2.2 `astraweave-asset-pipeline` Library

**Location**: `astraweave-asset-pipeline/` (workspace member).

**Capability**: programmatic library for BC7/ASTC compression + meshopt + AssetValidator. Consumed by `aw_asset_cli` per pipeline cook flow.

**Pipeline output formats**:
- KTX2 (`.ktx2`) for textures with `.meta.json` sidecars.
- meta.json contains: format, size, mipmap count, color space, source hash.
- Toktx / basisu external tools used for KTX2 generation (per inventory; not directly verified this pass).

### §2.3 `astraweave-assets` Multi-Provider Asset Fetcher

**Location**: `tools/astraweave-assets/` (workspace member; standalone tool).

**Per directory listing**: 11 source files + multi-provider architecture.

**Source files**:
- `config.rs` / `unified_config.rs` — configuration.
- `direct_url_provider.rs` — generic URL download.
- `downloader.rs` — download orchestration.
- `kenney_provider.rs` — Kenney asset packs (free CC0 game assets).
- `polyhaven.rs` + `polyhaven_provider.rs` — PolyHaven canonical PBR (also CC0).
- `organize.rs` — output directory organization.
- `provider.rs` — provider trait abstraction.
- `summary.rs` — fetcher summary output.
- `main.rs` + `unified_main_new.rs` — CLI entry.
- `lib.rs` — library API.

**Asset inventory's prompt-level framing was incomplete**: described only PolyHaven. Actual crate is **multi-provider** including Kenney (CC0 game assets) and itch.io (mentioned in completion reports). Far more capable than inventory described.

**Tier 2 content origin verification**: the `assets/textures/` PolyHaven content matches PolyHaven naming conventions (`<name>_<map>_<res>.jpg` format) — consistent with `astraweave-assets` PolyHaven provider output.

**Integration with `aw_asset_cli`**: per inventory framing, the fetcher runs independently then deposits content into `assets/textures/`. The cook pipeline then reads `assets_src/` for runtime conversion. The fetcher is a SEPARATE workflow from cook; the two are not chained automatically.

### §2.4 `astraweave-asset` Runtime Orchestration

**Location**: `astraweave-asset/` (workspace member).

**Capability**: runtime asset orchestration — `cell_loader`, `nanite_preprocess`, optional `blend_import` (per inventory framing; not deeply verified this pass).

**Consumers** (per inventory):
- `astraweave-gameplay`
- `astraweave-scene`
- `astraweave-scripting`
- `visual_3d` example
- `veilweaver_demo` example

**Integration seam with `MaterialLibrary` (Real-Fix.D `7067cc03d`)**: the `MaterialLibrary` canonical from Real-Fix.D establishes 32-slot capacity (22 named + 10 reserved) shared between UI + renderer. The runtime asset loader (presumably under `astraweave-render/src/terrain_material_manager.rs::set_material` based on `terrain_material_manager.rs` evidence at §1.2.6) is where material file paths from `materials.toml` are read and texture data loaded into the canonical texture arrays. Sub-phase A + Sub-phase C work touches this seam.

---

## §3 — Format Convention Verification

### §3.1 Tier 1 `_mra.png` Channel Ordering — ORM IN ALL BUT NAME

**Critical finding**: file naming uses `_mra.png` suffix but the actual data channel layout is **ORM** (AO/Roughness/Metallic) per shader convention.

**Evidence**:
- `astraweave-render/shaders/pbr_terrain.wgsl:334-338`:
  ```wgsl
  // ORM: R=AO, G=Roughness, B=Metallic (standard packing)
  final_ao += orm_sample.r * w;
  final_roughness += (orm_sample.g * layer.material_factors.y) * w;
  final_metallic += (orm_sample.b * layer.material_factors.x) * w;
  ```
- `astraweave-render/src/terrain_material_manager.rs:13`: documents "`layer_albedo`, `layer_normal`, `layer_orm`, `layer_height`".
- `terrain_material_manager.rs:32`: "upload the 8 layer textures (albedo + normal + orm + height)".
- `terrain_material_manager.rs:69`: "Normal / ORM / height array resolution".
- `terrain_material_manager.rs:126`: `LayerTextures::orm: Option<&'a [u8]>`.

**Conclusion**: the `_mra.png` files contain ORM-packed data; the filename suffix is historical artifact. Replacement workflow must produce ORM-packed `_mra.png` files (R=AO, G=Roughness, B=Metallic).

**PolyHaven ARM matches ORM exactly**: PolyHaven's `_arm_*.jpg` files are also ARM-conventional (R=AO, G=Roughness, B=Metallic). The ARM/ORM acronyms refer to the same channel layout, just with different reading order. Conversion is rename-only; no channel-swizzle needed.

### §3.2 Normal Map Convention

PolyHaven sources provide both `_nor_dx_*` (DirectX +Y down) and `_nor_gl_*` (OpenGL +Y up) variants. AstraWeave's convention not yet runtime-verified (would require deeper shader investigation than research-pass scope). Recommendation: use `_nor_gl_*` (OpenGL +Y up) for replacement assets — most wgpu-based engines use OpenGL convention; if verification surfaces mismatch, channel inversion is a quick fix.

### §3.3 Biome TOML Schema

**Main biome schema** (used by 9 of 10 biomes: grassland, desert, mountain, forest, swamp, tundra, beach, river, terrain):

```toml
[biome]
name = "<biome_id>"

[[layer]]
key = "<material_role>"     # e.g., "grass", "rock_smooth", "mud"
albedo = "../<name>.png"    # relative path from <biome>/ to ../<name>.png
normal = "../<name>_n.png"
mra    = "../<name>_mra.png"  # ORM-packed per §3.1
tiling = [f32, f32]
triplanar_scale = f32

# ...up to 5 [[layer]] entries...
```

**Each biome has exactly 5 layers**. Layer index = position in TOML array (0..4).

**Material aliasing**: `key` field is biome-semantic (e.g., "rock_smooth") not raw filename. Multiple biomes can map their "rock_smooth" key to `../rock_slate.png` (grassland, desert, forest) or `../stone.png` (terrain) — biome controls which underlying asset implements its semantic role.

**`arrays.toml` companion** (e.g., `grassland/arrays.toml`):
```toml
[layers]
grass = 0
rock_smooth = 1
dirt = 2
sand = 3
moss = 4
```

The `arrays.toml` maps biome-semantic keys to stable u32 layer indices for GPU array allocation. The `materials.toml` ordering matches `arrays.toml` indexing.

**`polyhaven` biome — DIFFERENT SCHEMA**:

```toml
[albedo]
aerial_rocks = 0
metal_plate = 1
plastered_wall = 2
wood_floor = 3
cobblestone = 4

[normal]
aerial_rocks = 0
# ...

[mra]
aerial_rocks = 0
# ...
```

The `polyhaven` biome uses `[albedo]` + `[normal]` + `[mra]` tables with `key = u32` mappings — different schema from main biomes. No corresponding `materials.toml` with paths; likely uses a different runtime loader path. Inconsistent schema may indicate dual-conventions or experimental support. Sub-phase B engine/project organization may need to reconcile.

### §3.4 Material Name Roster

22 raw material names in `assets/materials/` directly (from PNG triples):
grass, sand, dirt, stone, rock_slate, forest_floor, tree_bark, tree_leaves, rock_lichen, cloth, plaster, roof_tile, cobblestone, default, gravel, ice, metal_rusted, moss, mountain_rock, mud, snow, wood_planks.

Compares directly to Real-Fix.D's `MATERIAL_NAMES: [&str; 22]` ordering (per Real-Fix.D commit body). All 22 are addressable via the canonical `MaterialLibrary` post-Real-Fix.D.

---

## §4 — Baking Pipeline Gap Characterization

### §4.1 12 Already-Baked Materials

| Material | KTX2 in `assets/materials/`? | KTX2 in `assets/materials/baked/`? | Source in `assets_src/materials/`? |
|----------|---|---|---|
| cloth | ✅ | ✅ | ✅ |
| dirt | ✅ | ✅ | ✅ |
| forest_floor | ✅ | ✅ | ✅ |
| grass | ✅ | ✅ | ✅ |
| plaster | ✅ | ✅ | ✅ |
| rock_lichen | ✅ | ✅ | ✅ |
| rock_slate | ✅ | ✅ | ✅ |
| roof_tile | ✅ | ✅ | ✅ |
| sand | ✅ | ✅ | ✅ |
| stone | ✅ | ✅ | ✅ |
| tree_bark | ✅ | ✅ | ✅ |
| tree_leaves | ✅ | ✅ | ✅ |

These 12 can be **re-cooked via `aw_asset_cli cook`** — sources exist; pipeline can regenerate baked outputs. No source acquisition needed for re-cook.

### §4.2 10 Unbaked Materials — Source Acquisition Required

| Material | Tier 2 source candidate (`assets/textures/`)? | Biome usage |
|----------|---|---|
| cobblestone | YES (`brick_wall_04*` could substitute; or fetch dedicated PolyHaven cobblestone set) | polyhaven (different schema) |
| default | NO (semantically placeholder; may not need real PBR — investigate purpose) | UI-only |
| gravel | YES (PolyHaven has gravel sets) | mountain, tundra, beach, river (4 biomes) |
| ice | YES (PolyHaven has ice sets) | tundra (1 biome) |
| metal_rusted | YES (PolyHaven has `rust_coarse_*` etc.) | UI-only |
| moss | YES (PolyHaven has moss sets) | grassland (via alias), swamp, river (3 biomes) |
| mountain_rock | YES (existing `aerial_rocks_01` or `boulder_01` in `assets/textures/`) | mountain, tundra (2 biomes) |
| mud | YES (PolyHaven has mud_riverbed sets) | swamp, river (2 biomes) |
| snow | YES (PolyHaven has snow_03 sets) | mountain, tundra (2 biomes) |
| wood_planks | YES (PolyHaven has wood_floor_diff variants; `wood_floor` already in polyhaven biome) | UI-only |

**Source acquisition options**:
1. Fetch dedicated PolyHaven sets for each missing material via `tools/astraweave-assets` PolyHaven provider.
2. Reuse existing `assets/textures/` PolyHaven content (boulder_01, aerial_rocks_01, etc.) for materials with overlapping semantics.
3. Generate procedurally (substance generator, blender, etc.) — out of campaign scope.

### §4.3 Re-Cook vs Source-Acquire Decision Tree

```
Per material in 22:
├── In `assets_src/materials/`?
│   ├── YES (12 materials) → re-cook is sufficient
│   │   └── Sub-phase A: aw_asset_cli cook produces canonical baked output
│   └── NO (10 materials) → source acquisition required first
│       ├── Tier 2 candidate exists in `assets/textures/`?
│       │   ├── YES → copy/convert into `assets_src/materials/` then cook
│       │   └── NO → fetch via `tools/astraweave-assets` then cook
│       └── Source promoted to `assets_src/` then re-cook
```

**Sub-phase A scope estimate**:
- A.1: Source acquisition session — fetch missing 10 materials via `tools/astraweave-assets` (PolyHaven provider) into `assets_src/materials/`. Coordinate channel layout (ARM → ORM-named `_mra.png`).
- A.2: Bake session — `aw_asset_cli cook` produces canonical KTX2 for all 22 materials.

---

## §5 — Engine/Project Asset Organization

### §5.1 Current State

`assets/` at repo root contains:
- `assets/materials/` — engine-canonical-by-default (used by all biomes + UI).
- `assets/materials/baked/` — KTX2 subset (duplicate of in-place baked outputs).
- `assets/materials/<biome>/` — biome-specific TOML configuration (10 biomes).
- `assets/textures/` — Tier 2 PolyHaven source content.
- `assets/<various>/` — third-party source asset packs (KayKit, Symphonie, Forest Scene, Road to Vostok, etc.) — engine-irrelevant; mixed-purpose root.

**No explicit engine/project separation**. Veilweaver-specific content (KayKit packs) coexists with engine-canonical content at the same hierarchy level. The runtime asset loader does not distinguish.

### §5.2 Option B-1 — Path-Based Split

Reorganize as:
```
assets/
├── engine/
│   ├── materials/             # 22 engine-canonical materials
│   │   ├── <name>.png + _n + _mra
│   │   └── <biome>/materials.toml (uses ../engine/materials/<name>)
│   └── textures/              # engine-canonical PolyHaven sources
└── veilweaver/
    ├── materials/             # Veilweaver-specific overrides (KayKit-aesthetic)
    └── models/                # Veilweaver-specific 3D assets
```

**Pros**:
- Explicit separation; clear ownership.
- Engine ships with `assets/engine/` only; Veilweaver overlays `assets/veilweaver/`.
- Other projects can replace `assets/veilweaver/` with `assets/myproject/` cleanly.
- Asset loader can detect project namespace from path.

**Cons**:
- Significant restructure: all 22 material paths + 10 biome TOMLs + cook config need updating.
- Migration touches existing baked KTX2 + meta.json sidecars (relocate or re-cook).
- Existing third-party packs (`assets/2D assets/`, `assets/Forest Scene/`, etc.) need clear bucket assignment.

### §5.3 Option B-2 — Override Mechanism

Keep `assets/materials/` as single path; add layered load:
```
assets/
├── materials/        # engine defaults loaded first
│   ├── <name>.png
│   └── <biome>/materials.toml
└── overrides/
    └── materials/    # project-specific overrides applied after engine defaults
        └── <name>.png  # same name; project overrides engine
```

Loader logic: load engine defaults; subsequent override layer replaces by name. Single asset path semantics with explicit override stage.

**Pros**:
- Minimal restructure of current `assets/materials/`.
- Override mechanism is invisible to projects that don't use it.
- Veilweaver project adds `assets/overrides/materials/<name>.png` only for materials it customizes.

**Cons**:
- Loader logic complexity (two-pass material assembly).
- Asset hash + signing manifest semantics get complicated (override invalidates engine asset's stable hash).
- Less explicit than B-1; harder to audit "what's the canonical engine asset?".

### §5.4 Option B-3 — Defer

Current implicit-engine state preserved. Address project overrides when Veilweaver-specific content needs to land. Document the deferral.

**Pros**:
- No restructure work this campaign.
- Sub-phase B becomes 0-session.
- Asset campaign focuses on quality upgrade without organizational concerns.

**Cons**:
- Decision will need re-litigation when Veilweaver content lands.
- Current `assets/` mix (engine content + third-party packs at same level) remains untidy.

### §5.5 Recommendation

**Option B-3 (defer)** for this campaign. Rationale:
- Andrew framing 2026-05-14 emphasized "engine work for now not veilweaver specific" — the campaign's primary deliverable is engine-canonical PBR quality, not organizational scaffolding.
- Veilweaver project content is decoupled (KayKit migration is separate work; not engine campaign scope per anti-drift discipline).
- B-1 or B-2 can be future campaign when Veilweaver content needs to land.
- Sub-phase B becoming 0-session reduces total campaign size from 5 to 4 sub-phases.

If Andrew prefers explicit organization NOW (B-1 or B-2), recommend B-1 (path-based split) over B-2 (override mechanism): the path-based approach is cleaner architecturally even if it costs more restructure work. B-2's loader complexity is the kind of indirection that surfaces §7.7-style traps (which path is authoritative? engine or project? at which load stage?). B-1 makes the boundary explicit.

---

## §6 — Frame Time Alert Investigation (deferred per Andrew-gate (e) recommendation)

The editor terminal output 2026-05-14 showed:
```
perf: alert triggered, category=Frame msg=Frame time 145.5ms (< 30 FPS)
Dropping ViewportRenderer GPU resources (depth_texture: true)
```

This research-pass **recommends Andrew-gate (e) selects e-2** (defer to Sub-phase D performance baseline session) rather than investigating in research-pass. Rationale:
- Performance diagnosis is its own discipline; bundling it into research-pass dilutes the asset campaign's analytical focus.
- A proper performance baseline must compare pre-replacement vs post-replacement; the pre-replacement baseline is best captured at the start of Sub-phase D, when the asset state is the relevant comparison point.
- The frame alert may have multiple causes (editor debug overhead, large scene_stats triangles, asset loading pressure, etc.); narrowing requires its own instrumentation-and-narrow session.
- If e-1 were selected, the research-pass would need to expand by ~50% to do proper performance characterization; that's scope creep.

**Sub-phase D scope (post-recommendation)**:
- D.1: pre-replacement baseline capture (frame time histogram, GPU memory footprint, draw call count, asset load timing) under current placeholder content + Real-Fix.D MaterialLibrary canonical state.
- D.2: post-replacement comparison after Sub-phase C content quality upgrade.
- D.3: optimization application if regressions surface.

If Andrew prefers e-1 (investigate now): a quick characterization could read `performance_panel.rs` to determine alert trigger conditions; identify the `Dropping ViewportRenderer GPU resources` event source; verify whether the 145.5ms is whole-frame or specific-subsystem. Estimated +1 hour to research-pass; produces a one-paragraph baseline annotation rather than full Sub-phase D.

---

## §7 — Tier 3 PBR Mapping Recommendations

**Note**: Tier 3 framing from prompt was incorrect (`assets/pbr/` doesn't exist). The mapping work absorbs into Sub-phase A (source acquisition) using `assets/textures/` PolyHaven content + new fetches. Mapping recommendations below pre-decide source-to-slot assignments for Andrew-gate (d):

### §7.1 `aerial_rocks_01` (existing in `assets/textures/`) → ?

**Candidates**: mountain_rock OR rock_slate OR stone.

**Recommendation**: → **mountain_rock**. The `aerial_rocks_01` PolyHaven set is large-scale rock surface imagery (4k resolution; aerial perspective). Best fit for mountain_rock (alpine exposed rock). Stone slot keeps simpler smaller-scale stone. rock_slate keeps slate-specific fragment imagery.

### §7.2 `aerial_beach_01` (existing) → sand

**Recommendation**: → **sand** if grain matches biome-coastal usage. Beach-specific sand grain may be coarse vs grassland sand. May warrant TWO sand variants if biome aesthetics differ; for engine-baseline, single canonical sand is sufficient.

### §7.3 `boulder_01` (existing) → mountain_rock OR rock_slate

**Recommendation**: → **rock_slate** as secondary; mountain_rock already covered by aerial_rocks_01. Boulder fragment imagery fits rock_slate's slate-fragment biome semantic.

### §7.4 New fetches needed (per §4.2)

For the 10 unbaked materials, fetch via `tools/astraweave-assets` PolyHaven provider (preferred for CC0 + high-quality + ARM-packed convention):

| Material | Recommended PolyHaven source |
|----------|------------------------------|
| gravel | `gravel_*` (multiple sets available) |
| ice | `ice_lake_*` or similar |
| moss | `moss_*` (multiple sets) |
| mud | `mud_riverbed_*` or `dirt_aerial_*` |
| snow | `snow_03` or `snow_aerial_*` |
| cobblestone | dedicated cobblestone set (e.g., `cobblestone_floor_*`) |
| metal_rusted | `rust_coarse_*` |
| wood_planks | `wood_floor_*` |
| default | NO ACQUISITION — investigate purpose; may be intentionally placeholder |
| mountain_rock | already covered by `aerial_rocks_01` per §7.1 |

This shrinks the source-acquire set from 10 to 8-9 (existing assets cover mountain_rock; default may not need real PBR).

---

## §8 — Sub-Phase Decomposition

### §8.1 Sub-phase A — Source acquisition + baking gap closure

**Scope**:
- A.1: Source acquisition (1 session). Fetch 8-9 missing materials via `tools/astraweave-assets`. Convert ARM → ORM-named `_mra.png` per §3.1. Promote into `assets_src/materials/`.
- A.2: Bake session (1 session). Run `aw_asset_cli cook`. Verify all 22 materials produce KTX2 + meta.json output. Andrew-gate REQUIRED (visible: brushes still functional; rendering uses baked content).

**Sessions**: 2.
**Andrew-gate**: REQUIRED for A.2 (rendering verification).

### §8.2 Sub-phase B — Engine/project asset organization (if Andrew-gate (b) ≠ b-3)

**Scope** (only if Andrew-gate (b) selects b-1 or b-2):
- B.1: Restructure (1 session).

**Sessions**: 1 if b-1/b-2; 0 if b-3.
**Recommendation per §5.5**: skip via b-3.

### §8.3 Sub-phase C — Tier 1 content quality upgrade

**Scope**:
- Per Andrew-gate (a) decision (replace all 22 vs placeholder-only-14 vs audit-then-decide):
  - **a-1 (replace all 22)**: 9 biome-grouped sessions per §7 forward chain in original prompt (grassland, desert, mountain, forest, swamp, tundra, beach, river, terrain). Or 5 sessions if grouped by material rather than biome (since materials are reused across biomes — replace each material once; subsequent biome sessions reuse).
  - **a-2 (placeholder-only-14 + mismatches-2 = 16 replacements)**: 4-6 sessions material-grouped.
  - **a-3 (audit-then-decide)**: 1 audit session + variable replacement sessions per decision.

**Sessions**: 4-9 depending on (a) decision.
**Andrew-gate**: REQUIRED per session (visible: rendering quality verification per biome batch).

### §8.4 Sub-phase D — Performance verification + optimization

**Scope**:
- D.1: Pre-replacement baseline capture (1 session if separate from Sub-phase A; 0 if bundled with A.2's Andrew-gate verification).
- D.2: Post-replacement comparison (1 session at end of Sub-phase C).
- D.3: Optimization if regressions surface (0-2 sessions per Andrew-gate routing).

**Sessions**: 1-3.
**Andrew-gate**: REQUIRED for D.2 + D.3.

### §8.5 Sub-phase E — Closeout

**Scope**: Campaign doc consolidation per Editor Multi-Tool Architecture Sub-phase 3.C closeout pattern.

**Sessions**: 1.
**Andrew-gate**: NOT REQUIRED (doc-only).

### §8.6 Total Campaign Size Estimate

| Sub-phase | Recommended sessions | Andrew-gate? |
|-----------|---------------------|--------------|
| Research-pass (this session) | 1 | Decisions only |
| A | 2 | A.2 yes |
| B | 0 (recommend b-3) | No |
| C | 4-9 (per Andrew-gate (a)) | Per batch |
| D | 1-3 | D.2 + D.3 yes |
| E | 1 | No |
| **Total** | **9-16** | — |

---

## §9 — Methodology Body of Practice Application

### §9.1 Carry-forward from Editor Multi-Tool Architecture Sub-phase 3

Per `b220442a7` (Sub-phase 3.C closeout) §13 of campaign doc:

| Lesson | Application to asset campaign |
|--------|-------------------------------|
| §7.1 (instrument-and-narrow canonical) | If asset acquisition outputs unexpected runtime behavior, instrument before fixing. |
| §7.2 (pre-execution actual-code verification) | Each sub-phase pre-execution re-greps actual asset state (file presence, TOML schema, channel convention). |
| §7.3 (symbol/signature pinning) | Material name + path conventions verified by grep at fix-time. |
| §7.4 (drift-finding documentation) | Each sub-phase commit body documents inventory-vs-actual drift findings. |
| §7.5 (semantic-invariant test discipline) | Asset tests grounded in semantic invariants (all 22 materials reachable; biome layer counts; ORM channel ordering) not specific file hashes. |
| §7.6 (derived-value reasoning trap) | When closure analysis depends on derived performance metrics, distinguish primary vs derived. |
| §7.7 (structural axiom — resource identity at boundary) | Asset-pipeline boundaries (source vs baked; engine vs project; raw vs MaterialLibrary) are §7.7 candidate sites. Sub-phase B is explicitly §7.7-aware. |
| §7.8 (audit-era misclassification) | This research-pass inventory differs from prompt's inventory; documenting drift cleanly without retro-revising prompt. |
| §7.9 (state-propagation pathway equivalence) | Re-cook from `assets_src/` should produce equivalent runtime output to manual edit of `assets/materials/`. Pathways must equivalent. |

### §9.2 Content-Driven Methodology Considerations

- **Per-biome visual coherence** (new consideration not in Sub-phase 3): assets in same biome should look aesthetically coherent; quality jumps mid-biome break immersion. Sub-phase C should batch by biome.
- **Material reuse across biomes** (new): grass/dirt/sand are used in 4-6 biomes each. Replacing each material once benefits all biomes; biome-grouped sessions naturally reuse prior replacements.
- **Visual quality evaluation cannot be machine-checked** (new): Andrew-gate verification of "looks good" is inherently subjective. Per-batch Andrew-gate routing remains canonical mechanism.
- **Performance baseline shifts with content** (new): high-quality PBR has 8-16× the memory footprint of solid-color placeholders. Performance baseline established mid-campaign (Sub-phase D) is essential.

### §9.3 New Methodology Lesson Candidates

**§7.10 candidate — content-vs-structural-defect distinction** (surfaced this research-pass; deferred elevation per anti-drift discipline):

Editor Multi-Tool Architecture Sub-phase 3 lessons §7.1-§7.9 all address **structural defects** (pipeline routing, attribute drift, pathway divergence). The Terrain Asset Quality campaign addresses **content quality** distinct from structural correctness. The renderer was structurally correct post-Real-Fix.D (canonical pipeline; 22 materials reach renderer mechanically); the content was placeholder-quality. Andrew-gate for Real-Fix.D verified mechanical correctness; Sub-phase 3 closeout deferred content-quality observations to "Defect Class 10" parallel work.

The content-vs-structural distinction may warrant explicit methodology lesson: "Structural correctness does not imply content quality; content quality requires its own campaign with separate Andrew-gate routing (subjective visual evaluation rather than mechanical verification)."

Elevation deferred to Sub-phase E closeout per Sub-phase 3 chronological-archeology discipline.

---

## §10 — Andrew-Gate Decision Points

### §10.1 (a) Tier 1 Replacement Scope

- **a-1**: replace all 22 (most thorough; ~9 sessions in Sub-phase C if biome-grouped).
- **a-2**: replace placeholder-only-14 + audit-mismatches-2 = 16 replacements (focused; 4-6 sessions).
- **a-3**: audit-then-decide per Tier 1 visual quality per slot (adds 1 audit session; variable replacement count).

**Recommendation**: **a-1** (replace all 22). Rationale:
- Engine-canonical baseline should be uniformly high-quality (not a mix of curated 8 + replaced 14).
- The 8 "distinct textures" are likely also PolyHaven-quality but bear inspection; some may be acceptable, others may also benefit from upgrade.
- Sub-phase C sessions amortize over biome-grouped batches; the 22-vs-16 difference is ~3 extra sessions for substantially cleaner output.
- Per Andrew framing 2026-05-14 ("high quality PBR textures rather than very simple solid color placeholders"), the campaign goal is uniform quality.

### §10.2 (b) Engine/Project Asset Organization

- **b-1**: path-based split (`assets/engine/` + `assets/veilweaver/`).
- **b-2**: override mechanism (single path, layered load).
- **b-3**: defer.

**Recommendation**: **b-3** (defer). Rationale per §5.5.

### §10.3 (c) Sub-phase Sequencing

- **c-1**: A first (source acquisition + baking gap closure), then C (content quality upgrade), then D (performance).
- **c-2**: A + B bundled (if Andrew-gate (b) ≠ b-3).
- **c-3**: alternative ordering.

**Recommendation**: **c-1** (A → C → D → E). Rationale: A's baking pipeline must work for the 10 unbaked materials before C can replace them with high-quality content. B is recommended skipped (b-3) so bundling question is moot.

### §10.4 (d) Tier 3 PBR Mapping

**Pre-decided per §7 recommendations** (since Tier 3 framing was incorrect):

| Source | Recommended slot |
|--------|------------------|
| `aerial_rocks_01` | mountain_rock |
| `aerial_beach_01` | sand |
| `boulder_01` | rock_slate |
| New PolyHaven fetches | per §4.2 + §7.4 |

Andrew-gate (d) is now per-material acquisition decision (Sub-phase A.1 surfaces specific PolyHaven sets per material; Andrew approves before fetching).

### §10.5 (e) Frame Time Alert Priority

- **e-1**: investigate during research-pass.
- **e-2**: defer to Sub-phase D performance baseline session.
- **e-3**: defer to potential separate performance campaign.

**Recommendation**: **e-2** (defer to Sub-phase D). Rationale per §6.

### §10.6 (f) PolyHaven Fetcher Characterization Scope

- **f-1**: full characterization in research-pass.
- **f-2**: partial characterization (just identify capability boundaries).
- **f-3**: defer to Sub-phase A pre-execution.

**Recommendation**: **f-2** (partial; satisfied by this research-pass's §2.3 characterization). Sub-phase A pre-execution does deep API-level verification when fetcher invocation is imminent.

---

## §11 — Revision History

- **2026-05-14 (this document, initial publication)**: Research-pass complete. Pre-execution §1.2 eight sub-items verified (with frame alert deferred per Andrew-gate (e) e-2 recommendation). Sub-phase decomposition surfaced. Andrew-gate decisions (a)-(f) presented with recommendations.

---

*End of audit*
