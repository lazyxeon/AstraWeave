# Terrain Asset Quality Campaign

**Status**: **Research-pass landed 2026-05-14**, this commit. Audit document at `docs/audits/terrain_asset_quality_campaign_research_pass_2026-05-14.md` is load-bearing input. Sub-phase decomposition + Andrew-gate decisions (a)-(f) surfaced for routing. Sub-phase A NOT STARTED (gated on Andrew-gate (a)+(b)+(c) decisions).

**Methodology body of practice inherited from Editor Multi-Tool Architecture Sub-phase 3.C closeout `b220442a7`** (§13 of that campaign doc). Nine canonical methodology lessons (§7.1-§7.9) + §7.7 structural axiom + Edit 2 multi-granularity discipline + canonical pipeline composability + single-concern session discipline + Andrew-gate routing + multi-session forward-chain structure apply.

**Scope**: Replace Tier 1 runtime terrain material placeholders (currently 14 of 22 materials are placeholder-quality per Real-Fix.D Andrew-gate observations 2026-05-08) with high-quality generic PBR ground textures suitable for any project built on AstraWeave. Engine-canonical baseline; Veilweaver-specific KayKit-aesthetic content is decoupled separate work. Verify renderer performance scales acceptably to real-PBR content load.

**Author**: Plan drafted 2026-05-14 by research-pass session, against asset state inventory + Editor Multi-Tool Architecture Sub-phase 3 body of practice + Andrew framing 2026-05-14 ("engine work for now not veilweaver specific"; "high quality PBR textures rather than very simple solid color placeholders"; "show performance issue with using real textures instead of placeholders that might need to be optimized"; "make sure textures and assets are right before we start trying to build multi biome regional archetype environments").

**Prior work**:
- `b220442a7` — Editor Multi-Tool Architecture Sub-phase 3.C closeout; methodology body of practice.
- `7067cc03d` — Real-Fix.D canonical `MaterialLibrary` (32-slot capacity: 22 named + 10 reserved).
- `609f85357` — Real-Fix.E ZoneBlend; canonical pipeline composability empirically demonstrated.
- `docs/audits/terrain_asset_quality_campaign_research_pass_2026-05-14.md` — research-pass audit (load-bearing input).

**Outcome on completion**:
- All 22 Tier 1 materials have high-quality PBR content (albedo + normal + ORM-packed).
- All 22 materials are baked to canonical KTX2 format via `aw_asset_cli cook`.
- `assets_src/materials/` is canonical source location for ALL 22 (closes the source-acquisition gap for 10 unbaked).
- 10 biomes (grassland, desert, mountain, forest, swamp, tundra, beach, river, terrain, polyhaven) render with coherent visual quality.
- Frame time + GPU memory footprint baseline + post-replacement comparison documented.
- Editor Multi-Tool Architecture campaign Sub-phase 4 + 5 + Mediator Removal + Sub-phase 6 can resume with stable visual foundation.

---

## §0 — How to use this document and anti-drift discipline

This plan is the authoritative design reference for the Terrain Asset Quality campaign. It inherits Editor Multi-Tool Architecture campaign's §0 structure with content-driven framing.

### Discipline imposed

- **Sub-phase completion**: each sub-phase's success criteria must be met before §11 status block advances. Andrew-gate authoritative for visible-output sub-phases (A.2, C per batch, D.2 + D.3).
- **Status header maintenance**: §11 phase status block updated in same commit as sub-phase closeout; Status header updated similarly.
- **§2 architectural commitments respected**: §2 decisions are load-bearing; sub-phase execution implements per §2; revisions require explicit halt-and-re-research.
- **Single-concern session discipline** (codified at Sub-phase 3 Round 7; sustained): instrument / closure / fix in separate sessions where appropriate; closures don't bundle fixes; reverts don't bundle analytical content. Sub-phase A.1 (source acquisition) and A.2 (baking) likely separate sessions; Sub-phase C biome batches separate per session.

### Lesson application — Andrew-gate authoritative for visible-output sub-phases

- **Research-pass** (this session): Andrew-gate routes decisions (a)-(f); no implementation gate.
- **Sub-phase A.1 (source acquisition)**: NOT Andrew-gated (code-level fetcher invocation; no visible output yet).
- **Sub-phase A.2 (baking)**: Andrew-gate REQUIRED (verify all 22 baked + rendering uses baked content + no regression).
- **Sub-phase B (engine/project organization)**: Andrew-gated only if b-1 (path-based split) is chosen and visible-rendering impact possible. Recommendation b-3 (defer) → NOT a sub-phase.
- **Sub-phase C (content quality upgrade)**: Andrew-gate REQUIRED per biome batch (visual quality verification).
- **Sub-phase D.1 (pre-replacement baseline)**: NOT Andrew-gated (measurement only).
- **Sub-phase D.2 (post-replacement comparison)**: Andrew-gate REQUIRED (performance acceptance).
- **Sub-phase D.3 (optimization if needed)**: Andrew-gate REQUIRED.
- **Sub-phase E (closeout)**: NOT Andrew-gated (doc-only).

### Scope-creep discipline — research-pass-before-reframe

Per inheritance from Regional Archetype Variation §0 + Editor Multi-Tool Architecture §0. Standing authorization for halt-and-spinoff if Terrain Asset Quality surfaces foundational architectural gaps it doesn't cover.

**Specific halt-and-spinoff scenarios**:
- Sub-phase A surfaces baking pipeline issues requiring `aw_asset_cli` refactor → Andrew-gate; halt; assess whether asset pipeline campaign needed first.
- Sub-phase D surfaces performance regression requiring renderer changes → Andrew-gate; halt; assess whether renderer optimization campaign needed.
- Sub-phase B surfaces architectural complexity in engine/project split → b-3 (defer) is escape hatch; don't expand B's scope.

### Anti-pattern this plan explicitly prevents

- **Bundled high-risk content + structural work**: per Editor Multi-Tool Architecture Sub-phase 3 §13.4 single-concern session discipline. Asset content work and structural rendering changes are kept in separate campaigns.
- **Premature optimization**: Sub-phase D performance work happens AFTER content replacement so baseline-vs-post comparison is clean.
- **Project-specific creep**: KayKit / Veilweaver-specific aesthetic decisions are out of campaign scope. Engine baseline is generic high-quality PBR.
- **Inventory-as-truth substitution**: research-pass surfaces actual state via filesystem + code grep; doesn't accept prompt-level inventory at face value.

### Methodological inheritance from Editor Multi-Tool Architecture Sub-phase 3

This campaign inherits Sub-phase 3's §13 methodology body of practice wholesale. Specifically:

- §7.1-§7.9 nine canonical methodology lessons.
- §7.7 structural axiom (resource identity at every boundary — applies at source vs baked, engine vs project, raw vs MaterialLibrary).
- §7.9 state-propagation pathway equivalence (re-cook should produce equivalent output to manual edit).
- Edit 2 multi-granularity discipline (no second baking pipeline; no parallel asset organization).
- Canonical pipeline composability (Real-Fix.D `MaterialLibrary` composes with content).
- Single-concern session discipline.
- Andrew-gate routing.
- Multi-session forward-chain structure.

The research-pass-and-spinoff workflow Editor Multi-Tool Architecture Sub-phase 3 demonstrated is the canonical pattern; this campaign follows it.

---

## §1 — Design summary

### §1.1 The problem being solved

Per Real-Fix.D Andrew-gate verification 2026-05-08 + Andrew framing 2026-05-14:
- 14 of 22 Tier 1 materials are placeholder-quality ("pixelated green splotches"): rock_slate, dirt, cobblestone, cloth, default, gravel, ice, metal_rusted, moss, plaster, rock_lichen, roof_tile, tree_bark, tree_leaves.
- 2 of 22 have audit-confirmed visual mismatch: forest_floor (brown thumbnail vs green render), stone (renders blue).
- The remaining 6 materials have acceptable but generic quality.
- Performance under current content is already stressed (`Frame time 145.5ms` observed); real PBR replacement will compound load.

The engine should ship with high-quality generic PBR ground textures so any project built on AstraWeave inherits a strong visual baseline.

### §1.2 The target

- 22 Tier 1 materials replaced with high-quality PolyHaven-quality PBR content (or equivalent CC0 sources).
- All 22 baked via `aw_asset_cli cook` (closing the 10-material baking gap).
- `assets_src/materials/` complete (canonical source for all 22).
- Performance baseline + post-replacement comparison documented; optimization applied if regressions surface.
- Engine-canonical baseline preserved (no Veilweaver-specific content this campaign).

Per Editor Multi-Tool Architecture campaign §13.3 (canonical pipeline composability): once Real-Fix.D's MaterialLibrary is paired with high-quality content, the rendering pipeline empirically supports the full breadth of project use cases. This campaign tests that composability claim under real content load.

### §1.3 Sub-phase breakdown

Per Andrew-gate decisions (a)-(f) in research-pass audit §10:

- **Research-pass** (this session): audit + campaign doc + Andrew-gate routing.
- **Sub-phase A** — Source acquisition + baking gap closure. 2 sessions. Andrew-gate at A.2.
- **Sub-phase B** — Engine/project organization. 0-1 sessions (recommend b-3 defer → 0 sessions).
- **Sub-phase C** — Tier 1 content quality upgrade. 4-9 sessions per Andrew-gate (a) decision (recommend a-1: replace all 22).
- **Sub-phase D** — Performance verification + optimization. 1-3 sessions.
- **Sub-phase E** — Closeout. 1 session.

Total: 9-16 sessions. Detailed per-sub-phase scope in §3-§8 below.

### §1.4 Integration with Editor Multi-Tool Architecture campaign

Editor Multi-Tool Architecture campaign Sub-phase 4 (Pattern A regression infrastructure) + Sub-phase 5 (RegionalArchetypePanel + ActiveTool) + Mediator Removal session + Sub-phase 6 closeout are **PAUSED** until this Terrain Asset Quality campaign closes. Resumption point is unambiguous: Sub-phase E closeout of this campaign → Editor Multi-Tool Architecture Sub-phase 4 prompt drafting.

Regional Archetype Variation campaign (paused 2026-05-03) resumption is similarly post-this-campaign + post-Editor-Multi-Tool-Architecture-Sub-phase-5 (G-pointer-events-fix likely subsumed by Sub-phase 5 per research audit §8.4).

---

## §2 — Technical architecture

### §2.1 Tier 1 runtime material library — single canonical location

**Decision**: `assets/materials/` remains the single canonical Tier 1 runtime location. No restructure this campaign (per Andrew-gate (b) recommendation b-3).

Each Tier 1 material is a PNG triple:
- `<name>.png` — albedo (sRGB color space).
- `<name>_n.png` — normal map (linear color space; OpenGL +Y up convention).
- `<name>_mra.png` — ORM-packed (R=AO, G=Roughness, B=Metallic; linear color space). Filename is historical artifact; data is ORM per shader convention (verified at `pbr_terrain.wgsl:334-338`).

Baked outputs:
- `<name>.ktx2` + `<name>.ktx2.meta.json` (sRGB BC7).
- `<name>_n.ktx2` + sidecar (BC5 normal).
- `<name>_mra.ktx2` + sidecar (BC7 linear).

Baked outputs are deposited both in `assets/materials/` directly and in `assets/materials/baked/` subdirectory (verified via filesystem inventory). The dual-location pattern is preserved.

### §2.2 Source location

**Decision**: `assets_src/materials/` is canonical source for ALL 22 Tier 1 materials post-Sub-phase A.

Currently `assets_src/materials/` has 12 of 22 (sources for the 12 already-baked). Sub-phase A.1 source acquisition populates the remaining 10.

This implies the cook pipeline (`aw_asset_cli cook` per `aw_pipeline.toml`) becomes authoritative for all 22 materials post-Sub-phase A. The current state (where 10 materials have placeholder PNGs directly in `assets/materials/` without `assets_src/` round-trip) is an anti-pattern that §7.9 (state-propagation pathway equivalence) flags: the manual-edit pathway and the cook pathway should produce equivalent runtime output. Currently they bypass each other for 10 materials.

### §2.3 Channel convention canonicalization

**Decision**: ORM channel layout (R=AO, G=Roughness, B=Metallic) is canonical. File naming continues with `_mra.png` suffix for compatibility with existing biome TOMLs + tooling. Documentation should explicitly note "ORM-packed despite `_mra` filename" at:
- README in `assets/materials/`.
- TOML field schema documentation.
- Renderer documentation.

Future renaming `_mra.png` → `_orm.png` is out of campaign scope (touches biome TOMLs + cook config + asset signing manifest hashes).

### §2.4 Format conversion at acquisition

**Decision**: PolyHaven `_arm_*.jpg` content is ARM-named but channel-identical to ORM. Acquisition pipeline:
1. Fetch PolyHaven set via `tools/astraweave-assets` PolyHaven provider.
2. Identify maps: `_diff` → albedo; `_nor_gl` → normal; `_arm` → ORM (RENAME ONLY).
3. Resize / downsample to target resolution (likely 2048×2048 for engine baseline; configurable per material role).
4. Deposit into `assets_src/materials/<name>.png` + `<name>_n.png` + `<name>_mra.png`.

No channel swizzle needed (ARM/ORM use identical R=AO, G=Roughness, B=Metallic ordering).

### §2.5 Baking convention

**Decision**: `aw_asset_cli cook` is canonical baker; no second baking pipeline. Per Edit 2 multi-granularity discipline (§13.2 of Editor Multi-Tool Architecture campaign), no parallel implementation surviving in tree.

The current dual-location pattern (KTX2 in `assets/materials/` directly + duplicate set in `assets/materials/baked/`) is preserved as canonical output. Investigation whether dual-location is intentional or accidental is out of campaign scope (preserves baker behavior unchanged).

### §2.6 Biome TOML schema preservation

**Decision**: existing biome TOML schema preserved unchanged. 9 main biomes (grassland, desert, mountain, forest, swamp, tundra, beach, river, terrain) use `[biome]` + `[[layer]]` structure. `polyhaven` biome uses different `[albedo]` + `[normal]` + `[mra]` table structure; preserved as-is (separate concern; reconciliation out of campaign scope).

Sub-phase C replaces texture content per `materials.toml` path references; does not modify the TOML structure or biome composition.

### §2.7 Resolution + memory budget

**Decision**: target resolution 2048×2048 (4 MB albedo BC7 + 4 MB normal BC5 + 4 MB ORM BC7 = ~12 MB per material × 32 slots = ~384 MB peak GPU memory).

Existing baked materials may be at different resolution (verifiable via meta.json sidecars). Sub-phase A.2 may renormalize all resolutions.

If 2048×2048 stresses Sub-phase D performance baseline beyond acceptable, fallback to 1024×1024 (~96 MB total). Resolution decision is per Andrew-gate verification at Sub-phase D.

### §2.8 Manifest + asset signing

**Decision**: cook pipeline's Ed25519 manifest signing preserved unchanged. Sub-phase A.2 produces new signed manifest with all 22 baked.

### §2.9 Performance baseline + acceptance criteria

**Decision**: Sub-phase D establishes pre-replacement + post-replacement baselines. Acceptance criteria for post-replacement:
- Frame time at default scene: not worse than pre-replacement (current 145.5ms alert is pre-empted; baseline establishes "current normal" for comparison).
- GPU memory footprint: within 2× of pre-replacement (PBR content is expected to ~2-4× memory; target is "not pathologically worse").
- Editor responsiveness: cursor + brush + viewport interaction not regressed.
- 8/8 brush modes operational (Sub-phase 3 Real-Fix.E PASS criterion preserved).

Specific thresholds finalized at Sub-phase D.1 baseline.

### §2.10 Out-of-scope content categories

**Decision**: this campaign is terrain materials only. Out of scope:
- Audio assets.
- 3D model assets (KayKit, etc.).
- Particle textures.
- UI textures / icons.
- Sky / atmosphere / IBL assets.
- Animation clips.

Future asset campaigns per category as needed.

### §2.11 Project override deferral

**Decision**: per §0.1 recommendation, engine/project asset organization (Andrew-gate (b)) is deferred via b-3. Veilweaver-specific KayKit-aesthetic content (currently in `assets/Complete KayKit Collection v4/` etc. at repo root) remains where it is; engine canonical content in `assets/materials/`. Project override mechanism is future-campaign work.

---

## §3 — Sub-phase A — Source acquisition + baking gap closure

### §3.1 Goal

Acquire source material PNGs for the 10 unbaked Tier 1 materials (cobblestone, default, gravel, ice, metal_rusted, moss, mountain_rock, mud, snow, wood_planks). Promote into `assets_src/materials/`. Run `aw_asset_cli cook` to produce canonical KTX2 outputs for all 22 materials.

### §3.2 Scope

**In-scope**:
- **A.1 (acquisition session)**:
  - Use `tools/astraweave-assets` PolyHaven provider to fetch dedicated sets for missing 8-9 materials.
  - For materials with existing Tier 2 fit (mountain_rock via `aerial_rocks_01`), prefer reuse.
  - Convert ARM → ORM-named `_mra.png` per §2.4.
  - Investigate "default" material purpose; may not need real PBR (preserve as placeholder if intentional).
  - Deposit into `assets_src/materials/`.
- **A.2 (baking session)**:
  - Run `aw_asset_cli cook`.
  - Verify all 22 materials produce `.ktx2` + `.meta.json` output.
  - Verify channel ordering (`_mra.ktx2` data matches ORM convention).
  - Andrew-gate verification: editor renders all 22 materials with baked content; no regression in 8/8 brush modes.

**Out-of-scope**:
- Tier 1 content REPLACEMENT for the 12 already-baked materials (Sub-phase C).
- Engine/project organization restructure (Sub-phase B).
- Performance baseline (Sub-phase D).
- Modifying `aw_asset_cli` or `astraweave-assets` code beyond invocation.

### §3.3 Success criteria

- 22 `<name>.png` + `<name>_n.png` + `<name>_mra.png` triples present in `assets_src/materials/`.
- 22 `<name>.ktx2` + `<name>_n.ktx2` + `<name>_mra.ktx2` (+ meta.json sidecars) present in `assets/materials/` + `assets/materials/baked/`.
- `cargo check -p aw_editor --features terrain-splat-arrays`: clean.
- Editor opens; renders default scene; all 22 materials reach renderer (Real-Fix.D PASS criterion preserved).
- 8/8 brush modes operational.
- Andrew-gate: editor visual inspection PASS.

### §3.4 Andrew-gate

A.1: NOT Andrew-gated (acquisition; no visible output yet).

A.2: Andrew-gate REQUIRED.

### §3.5 Reversibility

A.1: trivial revert (new files only in `assets_src/materials/`).
A.2: trivial revert (regenerated KTX2 + manifest; restoring prior baked state requires checking out earlier commit).

### §3.6 Expected commits

- **A.1.A**: source acquisition commit (10 new `<name>.png` + `<name>_n.png` + `<name>_mra.png` triples in `assets_src/materials/`).
- **A.2.A**: bake commit (regenerated `.ktx2` + `.meta.json` for all 22; updated `manifest.json` + `assets.json`).
- **A.2.B** (Andrew-gate PASS): closeout entry in §11 + §12.

---

## §4 — Sub-phase B — Engine/project asset organization (RECOMMEND SKIP)

### §4.1 Goal

If Andrew-gate (b) selects b-1 or b-2: restructure asset organization to explicit engine/project separation. If b-3 (recommended): SKIP this sub-phase.

### §4.2 Scope (b-1 path-based split, if selected)

- Move `assets/materials/` → `assets/engine/materials/`.
- Update biome `materials.toml` path references.
- Update cook pipeline config + asset loader.
- Update biome `arrays.toml` if necessary.

### §4.3 Scope (b-2 override mechanism, if selected)

- Add overlay loader logic to asset runtime.
- Document override path semantics.

### §4.4 Recommendation: SKIP

Per audit §5.5 + Andrew-gate (b) recommendation b-3 (defer). Sub-phase B is 0-session.

If Andrew picks b-1 or b-2, this sub-phase becomes 1 session (b-1) or 2-3 sessions (b-2 needs loader work + tests).

---

## §5 — Sub-phase C — Tier 1 content quality upgrade

### §5.1 Goal

Replace Tier 1 material content with high-quality PBR per Andrew-gate (a) scope decision (a-1 recommended: all 22 materials replaced).

### §5.2 Scope

**In-scope**:
- Per Andrew-gate (a): replace 22 (a-1) or 16 (a-2) or audit-then-decide (a-3) materials.
- Group replacements by biome batch or by material role; biome-grouped recommended for visual coherence checking.
- Each batch: identify PolyHaven source(s), fetch via `tools/astraweave-assets`, convert format, deposit `assets_src/materials/`, run `aw_asset_cli cook`, Andrew-gate verification.

**Recommended biome batching** (a-1 scope, ~5-9 sessions):
- **C.1 — grassland**: grass, rock_smooth(rock_slate), dirt, sand, moss.
- **C.2 — forest**: forest_floor, tree_bark, tree_leaves, rock_lichen, rock_smooth(rock_slate)*.
- **C.3 — mountain**: mountain_rock, snow, gravel, stone, dirt*.
- **C.4 — tundra**: snow*, ice, gravel*, mountain_rock*, dirt*.
- **C.5 — desert**: sand*, rock_smooth(rock_slate)*, stone*, plaster, cloth.
- **C.6 — swamp**: mud, moss*, dirt*, tree_bark*, rock_lichen*.
- **C.7 — beach**: sand*, gravel*, stone*, grass*, dirt*.
- **C.8 — river**: mud*, gravel*, sand*, moss*, grass*.
- **C.9 — UI-only + cleanup**: wood_planks, metal_rusted, roof_tile, default, cobblestone (these have UI presence but limited biome usage).

(*) materials marked already replaced in earlier biome session; visible in this biome's batch but not re-fetched.

Aggregated: 22 unique material replacements across 9 sessions. Materials reused across biomes amortize.

**Out-of-scope**:
- Performance measurement (Sub-phase D).
- TOML schema changes (preserves §2.6 commitment).
- Engine/project restructure (Sub-phase B; recommend skip).
- Tier 2 content reorganization.

### §5.3 Success criteria per batch

- Replacement materials baked via `aw_asset_cli cook`.
- Editor renders biome with replaced materials.
- Visual coherence with biome semantic (Andrew subjective evaluation).
- No regression in 8/8 brush modes.
- No regression in renderer (no new wgpu validation errors).

### §5.4 Andrew-gate per batch (REQUIRED)

Per Editor Multi-Tool Architecture campaign §13.5 (Andrew-gate routing). Each biome batch's PASS/REGRESS verdict shapes next batch. PARTIAL-PASS may surface unexpected material acquisition needs (e.g., a PolyHaven source aesthetic doesn't fit biome semantic; re-fetch needed).

### §5.5 Reversibility per batch

Per-batch: trivial revert via `git revert` of biome batch commit.

### §5.6 Expected commits per batch

- **C.X.A**: source acquisition for batch materials.
- **C.X.B**: cook + Andrew-gate verification commit (closeout-style; updated `manifest.json` + Andrew-gate verdict in §12 entry).

---

## §6 — Sub-phase D — Performance verification + optimization

### §6.1 Goal

Establish performance baseline pre-replacement; measure post-replacement impact; optimize if regressions surface.

### §6.2 Scope

**In-scope**:
- **D.1 (baseline session)**:
  - Capture frame time histogram at default scene under current placeholder content.
  - Measure GPU memory footprint, draw call count, asset load timing.
  - Investigate the `Frame time 145.5ms` alert observed 2026-05-14 — narrow to specific subsystem (editor debug overhead? scene triangle count? asset loading?).
  - Document the `Dropping ViewportRenderer GPU resources (depth_texture: true)` event — verify whether OOM-driven or lifecycle-driven.
  - Establish acceptance thresholds for D.2.
- **D.2 (post-replacement comparison)**:
  - Re-run baseline measurements with high-quality content.
  - Compare against D.1 baseline.
  - Categorize regressions (acceptable / requires optimization / blocking).
- **D.3 (optimization session, if needed)**:
  - Apply targeted optimization (resolution reduction, mipmap streaming, texture compression tuning, deferred loading, etc.) per regression category.
  - Re-measure; Andrew-gate accepts.

**Out-of-scope**:
- Renderer architecture changes (e.g., compute-driven culling). Out of campaign; potential separate renderer campaign.
- Asset format changes (BC7 → ASTC). Out of campaign.
- Sub-phase D.3 is conditional; if D.2 shows no regression, D.3 is 0 sessions.

### §6.3 Success criteria

- D.1: baseline established.
- D.2: post-replacement within acceptance thresholds OR D.3 brings within threshold.
- 8/8 brush modes operational throughout.
- Frame alert characterized; if pre-existing (independent of asset content), surfaced for separate work.

### §6.4 Andrew-gate

D.1: NOT Andrew-gated.
D.2: Andrew-gate REQUIRED.
D.3: Andrew-gate REQUIRED.

### §6.5 Reversibility

D.1 + D.2: doc-only (measurements).
D.3: depends on optimization shape; should be small-scope reversible commits.

### §6.6 Expected commits

- **D.1.A**: baseline capture + measurement methodology documentation.
- **D.2.A**: post-replacement comparison + verdict.
- **D.3.X** (if needed): optimization commits + Andrew-gate verification.
- **D.E**: sub-phase closeout entry in §12.

---

## §7 — Sub-phase E — Closeout

### §7.1 Goal

Campaign chain consolidation; methodology body of practice carry-forward; resumption pointer for Editor Multi-Tool Architecture Sub-phase 4+ and Regional Archetype Variation.

### §7.2 Scope

Per Editor Multi-Tool Architecture Sub-phase 3.C closeout pattern (`b220442a7`):

- Status header update: campaign COMPLETE.
- §11 entries: all sub-phase entries COMPLETE.
- §12 Sub-phase E closeout entry: campaign chain commit summary table + key findings + methodology body of practice carry-forward + Andrew-gate verification archeology.
- §13 methodology body of practice update: any new lesson candidates elevated (e.g., §7.10 content-vs-structural-defect distinction per audit §9.3).
- Resumption pointer: Editor Multi-Tool Architecture Sub-phase 4 prompt drafting next; Regional Archetype Variation resumption follows.

### §7.3 Success criteria

- Doc updates land.
- Campaign marked COMPLETE.
- Resumption point for Editor Multi-Tool Architecture Sub-phase 4 unambiguous.

### §7.4 Andrew-gate

NOT Andrew-gated (doc-only).

### §7.5 Reversibility

Doc-only; trivial revert.

### §7.6 Expected commits

- **E.A**: campaign doc closeout commit (Status header + §11 + §12 Sub-phase E entry + §13 update).
- Optional hash-fixup.

---

## §8 — Forward chain post-campaign

1. **Editor Multi-Tool Architecture Sub-phase 4** (Pattern A regression infrastructure for dispatcher class). Resumed after Sub-phase E closeout.
2. **Editor Multi-Tool Architecture Sub-phase 5** (RegionalArchetypePanel ActiveTool implementation + registration). Likely subsumes G-pointer-events-fix per research audit §8.4.
3. **Editor Multi-Tool Architecture Mediator Removal session** (dedicated; per Q6 of that campaign).
4. **Editor Multi-Tool Architecture Sub-phase 6 closeout** (editor multi-tool architecture campaign COMPLETE).
5. **Regional Archetype Variation campaign resumption** (paused 2026-05-03; resumes post-Sub-phase 5 with G-pointer-events-fix likely subsumed). H-saveload-diagnostic; F.5-overlay-and-gate; F.6-F.7-F.8 closeout.

**Parallel work items** (any time, decoupled):
- CLAUDE.md amendment cycle elevation (§7.7 resource-identity rule + §7.9 state-propagation pathway equivalence to first-class top-level Edits).
- KayKit asset migration (Veilweaver-project content; engine campaign provides baseline, project adds aesthetic-specific overrides).
- Defect Class 6/7/8 brush mathematics campaign (potential Sub-phase 7 of Editor Multi-Tool Architecture).
- Audio asset campaign (if/when needed).
- 3D model asset campaign (if/when needed).

---

## §10 — Out of scope for entire campaign

- **Renderer architecture changes** (e.g., compute-driven culling, deferred lighting, occlusion queries). Out of campaign; potential future renderer optimization campaign.
- **Asset format changes** (BC7 → ASTC, KTX2 → DDS, etc.). Out of campaign.
- **Non-terrain asset categories**: audio, models, particles, UI textures, sky/atmosphere/IBL, animation clips. Out per §2.10.
- **Engine/project organization** if Andrew-gate (b) selects b-3 (recommended). If b-1/b-2 chosen, B is in-scope but limited to single restructure session.
- **KayKit asset migration**: Veilweaver-project work; decoupled. Engine campaign provides generic PBR baseline; project adds KayKit-aesthetic overrides separately.
- **`aw_asset_cli` refactor** beyond invocation. Tool is used as-is; tool improvements are separate work.
- **`astraweave-assets` refactor** beyond invocation.
- **Biome TOML schema changes** beyond path updates within `materials.toml`.
- **`polyhaven` biome schema reconciliation** with main biome schema (different `[albedo]`/`[normal]`/`[mra]` table structure; preserved as-is).
- **MaterialLibrary modifications** (Real-Fix.D canonical; preserved unchanged).
- **Shader changes** (`pbr_terrain.wgsl` + `pbr_terrain_forward.wgsl` canonical post-Sub-phase 3; preserved unchanged).
- **Frame alert investigation** if Andrew-gate (e) selects e-2 (recommended; defer to Sub-phase D).
- **Performance optimization beyond what regressions require** (D.3 is conditional; no proactive optimization).

---

## §11 — Phase status

This section must be updated in the same commit that completes each sub-phase per §0 discipline.

```text
Terrain Asset Quality Campaign research-pass: COMPLETE 2026-05-14, this commit. Audit at docs/audits/terrain_asset_quality_campaign_research_pass_2026-05-14.md. Andrew-gate (a)+(b)+(c)+(d)+(e)+(f) pending for sub-phase routing.
Sub-phase A — Source acquisition + baking gap closure: NOT STARTED (gated on Andrew-gate decisions).
Sub-phase B — Engine/project asset organization: NOT STARTED (recommend skip via Andrew-gate (b) b-3).
Sub-phase C — Tier 1 content quality upgrade (biome-grouped): NOT STARTED (gated on Sub-phase A PASS + Andrew-gate (a)).
Sub-phase D — Performance verification + optimization: NOT STARTED (gated on Sub-phase C PASS).
Sub-phase E — Closeout: NOT STARTED (gated on Sub-phase D PASS).
```

Format for completion updates: `<sub-phase>: COMPLETE <YYYY-MM-DD>, commit <hash>`.

---

## §12 — Deviations log

This section records design decisions made during execution that deviate from this plan.

### 2026-05-14, Research-pass, this commit

**Research-pass landed. Analytical-only session per Andrew-gate (n) pattern from Editor Multi-Tool Architecture Sub-phase 3 Cleanup-D research-pass precedent (`47a33e476` + `e44f74c56`). NO production code changes; NO asset file changes; NO baking pipeline invocation.**

**Pre-execution verification per §1.2 of prompt (8 sub-items)**:

- **§1.2.1 Asset directory structure**: `assets/materials/` contains 22 materials as PNG triples (verified via glob); `assets/materials/baked/` contains 12 KTX2 sets duplicating in-place baked outputs; `assets/textures/` contains PolyHaven sources; 10 biome subdirectories present (grassland, desert, mountain, forest, swamp, tundra, beach, river, terrain, polyhaven). `assets/pbr/` DOES NOT EXIST (inventory error). `assets/` root contains third-party source asset packs (KayKit, etc.) mixed with engine content.

- **§1.2.2 `assets_src/` state**: EXISTS at repo root. Contains `materials/` (12 source PNG triples matching the 12 already-baked materials exactly), `textures/` (sparse; 3 files), `environments/`. **CRITICAL FINDING**: 10 unbaked Tier 1 materials have NO source files in `assets_src/`. Cook pipeline cannot regenerate them without source acquisition first. This significantly shapes Sub-phase A scope.

- **§1.2.3 aw_asset_cli pipeline**: 3 subcommands (cook, bake-texture, validate). Cook reads `aw_pipeline.toml`, processes `assets_src/` → `assets/`, signs manifest with Ed25519. Pipeline config has 3 rules: texture (PNG/JPG), model (glTF/GLB), audio (WAV/MP3/OGG/FLAC). Per-file normal_map detection inside `texture_baker::infer_config_from_path`.

- **§1.2.4 `astraweave-assets` fetcher**: Multi-provider crate (PolyHaven + Kenney + itch.io + direct URL) — substantially more capable than inventory framed. 11 source files including `polyhaven_provider.rs`, `kenney_provider.rs`, `direct_url_provider.rs`. Per Andrew-gate (f) f-2 recommendation, partial characterization sufficient; deep API-level verification deferred to Sub-phase A pre-execution.

- **§1.2.5 TOML schema**: 9 main biomes use `[biome]` + 5× `[[layer]]` structure with `key`, `albedo`, `normal`, `mra`, `tiling`, `triplanar_scale` fields. `arrays.toml` companion maps biome-semantic keys to u32 layer indices. `polyhaven` biome uses different `[albedo]` + `[normal]` + `[mra]` table structure (inconsistent with main biomes; preserved as-is per §2.6).

- **§1.2.6 Shader `_mra.png` channel ordering — CRITICAL FINDING**: `pbr_terrain.wgsl:334-338` explicitly comments "ORM: R=AO, G=Roughness, B=Metallic". `terrain_material_manager.rs` documents fields as `layer_orm` / `orm` throughout. The `_mra.png` filename convention is **historical artifact**; data is functionally ORM-packed. PolyHaven `_arm_*.jpg` ARM convention matches ORM channel layout identically — conversion is rename-only.

- **§1.2.7 Asset crate workspace**: 4 asset crates per inventory confirmed (astraweave-asset runtime, astraweave-asset-pipeline library, aw_asset_cli CLI, astraweave-assets multi-provider fetcher). Integration seam at `astraweave-render/src/terrain_material_manager.rs::set_material` where TOML-referenced paths meet GPU texture array allocation.

- **§1.2.8 Frame time alert**: DEFERRED per Andrew-gate (e) e-2 recommendation. The `Frame time 145.5ms (< 30 FPS)` + `Dropping ViewportRenderer GPU resources (depth_texture: true)` observed 2026-05-14 is documented as pre-existing baseline-stressed state; investigation in Sub-phase D where pre-vs-post-replacement comparison is the relevant context.

**Deliverables**:

- **Audit document** at `docs/audits/terrain_asset_quality_campaign_research_pass_2026-05-14.md` (~5,500 words; 11 top-level sections; per-slot characterization + pipeline architecture + format conventions + baking gap + organization options + frame alert + Tier 3 mapping + sub-phase decomposition + methodology application + Andrew-gate decisions + revision history).
- **New campaign doc** at `docs/current/TERRAIN_ASSET_QUALITY_CAMPAIGN.md` (this document, ~7,000 words; Status header + §0-§13 structure mirroring Editor Multi-Tool Architecture campaign doc).
- **Andrew-gate decisions surfaced** (recommendations):
  - **(a) Tier 1 replacement scope**: a-1 (replace all 22).
  - **(b) Engine/project asset organization**: b-3 (defer).
  - **(c) Sub-phase sequencing**: c-1 (A → C → D → E; B skipped).
  - **(d) Tier 3 PBR mapping**: pre-decided per audit §7 (aerial_rocks_01 → mountain_rock; aerial_beach_01 → sand; boulder_01 → rock_slate; new fetches per missing materials).
  - **(e) Frame time alert priority**: e-2 (defer to Sub-phase D).
  - **(f) PolyHaven fetcher characterization scope**: f-2 (partial; satisfied by audit §2.3).

**Findings diverging from prompt inventory** (documented in audit §1):
- `assets/pbr/` directory does not exist; Tier 3 framing was incorrect. PolyHaven content is in `assets/textures/` (Tier 2 location).
- Multi-provider fetcher (not just PolyHaven; also Kenney + itch.io + direct URL).
- 10 biomes not 9 (`polyhaven` biome is extra; uses different schema).
- 10 unbaked materials (not 9; the inventory's "9 missing-baked" omitted `default`).
- `_mra.png` naming describes filename only; channel data is ORM (R=AO, G=Roughness, B=Metallic).

**Methodology lessons applied** (from Editor Multi-Tool Architecture Sub-phase 3 §13):
- §7.1 instrument-and-narrow canonical: not applicable yet (no instrumentation needed for analytical-only research).
- §7.2 pre-execution actual-code verification: ✅ applied (8 sub-items mandatory).
- §7.3 symbol/signature pinning: ✅ applied (re-grepped material paths, channel conventions, biome schemas).
- §7.4 drift-finding documentation: ✅ applied (this entry documents inventory-vs-actual drift findings).
- §7.5 semantic-invariant tests: deferred to sub-phase scope (this research-pass writes no tests).
- §7.7 structural axiom (resource identity at boundary): applicable to source-vs-baked + engine-vs-project boundaries; surfaced in §2 architectural commitments.
- §7.9 state-propagation pathway equivalence: applicable to manual-edit-vs-cook pathways; surfaced as anti-pattern in §2.2.

**Methodology lesson candidate surfaced** (deferred elevation per anti-drift discipline):
- **§7.10 candidate — content-vs-structural-defect distinction**: structural correctness (Real-Fix.D MaterialLibrary mechanical correctness) does not imply content quality (placeholder PNGs). Andrew-gate routing for content quality uses subjective visual evaluation rather than mechanical verification. Elevation deferred to Sub-phase E closeout per Sub-phase 3 chronological-archeology discipline.

**Out of scope per §0.1 + §1.4 anti-drift discipline (15 named temptations); all 15 held**:
- NO production code changes (research-pass is analytical-only).
- NO asset file changes (no Tier 1 PNG replacements; no TOML modifications).
- NO baking pipeline invocation.
- NO modifications to Real-Fix.A/B/C/D/E or Cleanup-A/B or Cleanup-D or any Editor Multi-Tool Architecture campaign commit.
- NO touching MaterialLibrary or canonical pipeline primitives.
- NO ARCHITECTURE_MAP.md updates.
- NO CLAUDE.md amendment elevation application.
- NO Sub-phase 4+ work from Editor Multi-Tool Architecture campaign (paused).
- NO Regional Archetype Variation resumption (paused).
- NO Defect Class 6/7/8 brush mathematics work.
- NO KayKit asset migration scope (decoupled Veilweaver work).
- NO broadening to non-terrain asset categories.
- NO new methodology lesson generation beyond surfacing §7.10 candidate.
- NO pre-deciding Tier 3 → Tier 1 mapping beyond research-pass recommendations.
- NO pre-deciding engine/project organization beyond recommendation b-3.

**Forward chain**:
1. Andrew-gate decisions (a)+(b)+(c)+(d)+(e)+(f) per audit §10.
2. Sub-phase A.1 prompt drafted next session (after Andrew-gate).
3. Sub-phase A.1 lands; A.2 follows; Sub-phase B if applicable; Sub-phase C biome batches; Sub-phase D measurement + optimization; Sub-phase E closeout.
4. Terrain Asset Quality campaign COMPLETE.
5. Resume Editor Multi-Tool Architecture campaign Sub-phase 4+.

**Scope held**: research-pass session only produced the new audit document + this new campaign doc. NO production code changes. NO asset file changes. NO modifications to Editor Multi-Tool Architecture campaign doc or any prior campaign chain commit. Working-tree unrelated changes intentionally not staged.

---

## §13 — Methodology Body of Practice (inheritance + this campaign's contributions)

### §13.1 Inheritance from Editor Multi-Tool Architecture Sub-phase 3

This campaign inherits the §13 methodology body of practice from `b220442a7` (Editor Multi-Tool Architecture Sub-phase 3.C closeout) wholesale. Specifically:

- **§13.1** — Nine canonical methodology lessons §7.1-§7.9.
- **§13.2** — Edit 2 multi-granularity discipline (four granularity scales).
- **§13.3** — Canonical pipeline composability (empirical payoff).
- **§13.4** — Single-concern session discipline.
- **§13.5** — Andrew-gate routing.
- **§13.6** — Multi-session forward-chain structure.
- **§13.7** — Body of practice carry-forward.

Refer to `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` §13 for canonical reference. This document does NOT duplicate; only documents this campaign's specific applications + any new lesson candidates surfaced.

### §13.2 This campaign's specific applications

| Sub-phase 3 lesson | This campaign's application |
|--------------------|----------------------------|
| §7.1 instrument-and-narrow | If asset acquisition runtime behavior is unexpected, instrument before retry. |
| §7.2 pre-execution verification | Each sub-phase pre-executes actual-state grep before fix. |
| §7.3 symbol/signature pinning | Material name + path conventions re-greped at fix-time. |
| §7.4 drift documentation | Each commit body documents inventory-vs-actual + audit-vs-actual drift. |
| §7.5 semantic-invariant tests | Asset tests grounded in invariants (all 22 reach renderer; ORM ordering; biome layer counts). |
| §7.6 derived-value reasoning trap | Performance measurements distinguish primary (raw frame time) vs derived (FPS = 1000/frame_time). |
| §7.7 structural axiom | Source-vs-baked, engine-vs-project, raw-vs-MaterialLibrary boundaries are §7.7 candidate sites. Sub-phase B is §7.7-aware. |
| §7.8 audit-era misclassification | Inventory drift documented cleanly (§1 of audit + §12 of this doc). |
| §7.9 state-propagation pathway equivalence | Cook pathway and manual-edit pathway must produce equivalent runtime output (currently violated for 10 unbaked materials). Sub-phase A closes this gap. |

### §13.3 New lesson candidate surfaced this research-pass

**§7.10 candidate — content-vs-structural-defect distinction** (deferred elevation per anti-drift discipline; Sub-phase E closeout consolidates):

Editor Multi-Tool Architecture Sub-phase 3 lessons §7.1-§7.9 all address **structural defects** — pipeline routing failures, attribute drift, pathway divergence. Mechanical correctness verification (8/8 brush modes; 22/22 materials reach renderer) is the Andrew-gate criterion.

The Terrain Asset Quality campaign addresses **content quality** distinct from structural correctness. The renderer was structurally correct post-Real-Fix.D; placeholder content was the residual problem. Andrew-gate verification for content quality requires subjective visual evaluation rather than mechanical pass/fail.

Implications for methodology:
- Content-quality Andrew-gates are inherently more subjective than structural-correctness gates.
- Per-biome batched verification preserves visual coherence as a first-class gate criterion.
- Pre-replacement baseline (Sub-phase D.1) establishes "this is what acceptable looks like before content upgrade"; post-replacement comparison checks for unintended regressions in both quality and performance.
- Content campaigns differ from architecture campaigns in instrumentation-and-narrow applicability: structural defects benefit from runtime instrumentation; content defects benefit from visual inspection + comparative measurement.

§7.10 elevation deferred to Sub-phase E closeout consolidation per Sub-phase 3 chronological-archeology discipline.

### §13.4 Content-driven methodology contributions

This campaign's content-driven framing potentially contributes additional sub-lessons:

- **Per-biome visual coherence as Andrew-gate criterion**: assets within same biome should look aesthetically coherent; quality jumps mid-biome break immersion. Sub-phase C is biome-batched specifically for this.
- **Material reuse amortization**: same material used across 4-6 biomes; replacing once benefits all. Biome-grouped sub-phase ordering naturally exploits this.
- **Performance baseline temporality**: pre-vs-post comparison requires baseline captured BEFORE content replacement; opportunistic post-hoc baseline is contaminated.

These sub-lessons may consolidate into §7.10 at Sub-phase E or emerge as separate methodology candidates.

---

*End of plan*
