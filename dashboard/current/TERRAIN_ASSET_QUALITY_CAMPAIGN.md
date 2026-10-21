# Terrain Asset Quality Campaign

**Status**: **CAMPAIGN CLOSED 2026-06-02** (E-closeout, this commit). Durable record: **`docs/audits/terrain_asset_quality_outcome_2026-06.md`** (immutable outcome doc, mirrors the Unified Camera closeout precedent). Substantive work complete: Sub-phase A acquisition through-line (9 Tier 1 PNGs) → A.3 premise-falsification (already deployed + consumed) → resumption reconciliation chain → D-core absolute footprint baseline (`5881b777f`, verdict ACCEPTABLE + optimization-candidate) → asset-hygiene findings (`ad2ed4e82`). §7.10 + §7.11 lesson candidates ELEVATED at E (outcome doc §6 Layer 1); resumption-arc methodology codified (outcome doc §6 Layer 2). Cleanup queue (cook-path / 27-PNG / polyhaven / editor-frame-alert / demo-disposition) + pause-stack unblock (Editor Multi-Tool Architecture, Regional Archetype Variation) handed off in outcome doc §5/§7. Sub-phase C retired as demand-driven (A.3); visual gate routed out as cross-campaign (Terrain Material System). See §11 for the per-sub-phase log.

**(Historical, pre-closeout)** **Sub-phase A.3 COMPLETE 2026-05-16**. Validate-only sub-phase per Andrew-gate re-scope after Phase 1 audit revealed premise mismatch: grassland biome was ALREADY wired with 5 PNG-native layers; all 27 Tier 1 PNGs already deployed at runtime root. Validation evidence: `headless_integration` test passes against `assets/materials/grassland/` via the canonical `MaterialManager::load_pack_from_toml` path — confirms PNG decode + GPU texture array upload + bind group layout match end-to-end. **Tier 1 acquisition through-line COMPLETE**: A.0.A (fetcher) + A.1 (6 mats) + A.1.B (2 mats) + A.1.C (1 mat ambientCG) + A.2 (cook deferred) + A.3 (grassland validate). §7.11 codified at §13.5 with 5 empirical pillars (stale-slug / verified-slug / provider-fit / tool-maturity / scope-fit). Forward chain pending Andrew decision: re-evaluate per A.3 §12 entry "Forward chain decision".

**Research-pass landed 2026-05-14 commit `b1223b49f`** (audit at `docs/audits/terrain_asset_quality_campaign_research_pass_2026-05-14.md`). Sub-phase decomposition + Andrew-gate decisions (a)-(f) routed.

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
Sub-phase A.1 — Source acquisition: BLOCKED 2026-05-14, this commit (marker). Pre-execution surfaced fetcher capability gap (no ARM-packed map fetch in tools/astraweave-assets PolyHaven provider) + missing local ImageMagick tooling for post-fetch channel-packing. Andrew-gate routing 2026-05-14 chose "Pause + extend fetcher" path; A.0 fetcher extension session opens before A.1 acquisition can proceed. `default` material reclassified per Andrew-gate (iii): flag for separate cleanup (dead-code removal from MaterialLibrary; out of this campaign scope; reduces acquisition target from 10 to 9 materials).
Sub-phase A.0 — Fetcher capability extension: COMPLETE 2026-05-14, this commit. Added "arm" map name to `tools/astraweave-assets/src/polyhaven.rs::polyhaven_map_names` (line ~341 match arm: `"arm" => vec!["arm", "ARM", "Arm"]`); added "arm" to default `requested_maps` in `polyhaven_provider.rs::resolve` AssetType::Texture branch (line ~62); added ARM assertion to `test_polyhaven_map_names_mappings` test. cargo check + cargo test (124 lib tests pass) clean. Restores audit §3.1's "rename-only" conversion premise. §7.11 methodology candidate empirically validated (deferred elevation to Sub-phase E).
Sub-phase A.1 — Source acquisition (re-scoped to 9 materials post-A.0): COMPLETE 2026-05-15 commit `f56a76124` (6 of 9; 3 deferred to A.1.B on stale-slug 404s). §11 not updated by that commit per anti-drift hold; recorded here at A.3 closeout.
Sub-phase A.1.B — Verified-slug acquisition: COMPLETE 2026-05-15 commit `f5387f20e` (2 of 3; ice deferred to A.1.C on PolyHaven provider-fit gap).
Sub-phase A.1.C — Ice from ambientCG: COMPLETE 2026-05-16 commit `76b15948d` (1 of 1; manual download + image-crate MRA pack; final Tier 1 material).
Sub-phase A.2 — Bake: STOPPED at Phase 3 gate 2026-05-16 (no commit; session deliverable). Cook deferred indefinitely per scaffolding framing — toktx/basisu missing on dev env, bake-texture parallel path has placeholder BC encoder + DFD sRGB bug. Channel-order audit confirmed 9 of 9 sources are ARM-in-MRA-filename (ice consistent; no repack).
Sub-phase A.3 — Grassland biome validate-only: COMPLETE 2026-05-16, this commit. Phase 1 audit revealed grassland already wired (5 layers PNG-native: grass/rock_smooth/dirt/sand/moss-via-rock_lichen); all 27 Tier 1 PNGs already deployed at runtime root. Re-scoped per Andrew-gate to validation-only. Headless integration test (`cargo test -p astraweave-render --features textures --test headless_integration`) passes — confirms canonical `load_pack_from_toml` path consumes existing grassland config without error. Splat-shader visual validation deferred (CLI agent cannot capture screenshots; manual visual validation Andrew runs locally post-commit).
Sub-phase B — Engine/project asset organization: NOT STARTED (recommend skip via Andrew-gate (b) b-3).
Sub-phase C — Tier 1 content quality upgrade (biome-grouped): NOT STARTED — re-evaluated after A.3 per §12 entry. Forward chain may skip C entirely if existing biome configs already consume Tier 1 PNGs at acceptable visual quality.
Sub-phase D-core — Absolute texture-footprint baseline: COMPLETE 2026-06-02, this commit. Reframed from §6.2's dissolved placeholder→replaced delta (C demand-driven; real PBR already deployed; no clean placeholder baseline). Analytical (no code): runtime terrain textures upload uncompressed RGBA8 at a hardcoded 1024² (`material_loader.rs:523-524` `build_arrays`), so footprint is exact from format+mip math. Verdict vs budget = ACCEPTABLE-with-optimization-candidate. See §12 D-core entry. D.2 (post-C comparison) retired with C; D.3 (optimization) deferred = "fix the broken BC7/KTX2 cook path" (4× VRAM overspend vs §2.7 budget model), conditional separate session.
Sub-phase D editor frame-perf (145.5ms / ViewportRenderer-drop): ROUTED OUT of TAQ per §6.3 escape clause — editor-runtime perf, asset-content-independent (observed with real PBR already deployed). Separate editor-perf work.
Sub-phase E — Closeout: COMPLETE 2026-06-02, this commit. Outcome doc `docs/audits/terrain_asset_quality_outcome_2026-06.md` (immutable durable record). §7.10 + §7.11 elevated (outcome §6 Layer 1); resumption-arc methodology codified (outcome §6 Layer 2, 5 findings); 5-item cleanup queue + pause-stack unblock handed off (outcome §5/§7). CAMPAIGN CLOSED.
```

**CAMPAIGN CLOSED 2026-06-02.** This §11 block is the per-sub-phase log; the consolidated outcome is the immutable `docs/audits/terrain_asset_quality_outcome_2026-06.md`. The closeout adds nothing functional — assets acquired/deployed/baselined, hygiene recorded, methodology codified, cleanup queue handed off. The campaign characterized and baselined terrain asset quality; it did not optimize it (cook-path 4× VRAM win deferred to its owner) and did not perfect it (the cleanup queue is the proof of remaining work).

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

### 2026-05-14, Sub-phase A.1 BLOCKED (marker commit), this commit

**Sub-phase A.1 source acquisition BLOCKED pending Sub-phase A.0 fetcher capability extension. NO assets fetched; NO assets_src/materials/ modifications; NO production code changes. Marker commit pattern analogous to Editor Multi-Tool Architecture Cleanup-B (`f7732d5d9`) — preserves pre-execution work + Andrew-gate decisions for the next session.**

**Pre-execution verification per §1.2 of A.1 prompt (7+1 sub-items)**:

- **§1.2.1 `astraweave-assets` PolyHaven provider characterization**:
  - CLI: `cargo run -p astraweave-assets -- fetch [--provider polyhaven]`.
  - Manifest at `assets/asset_manifest.toml` (TOML); per-asset `[[assets]]` entries with `handle`, `provider`, `type`, `id`, `resolution`, `format`.
  - Output: `assets/_downloaded/<handle>/<handle>_<map>.<ext>` (per `organize.rs`).
  - Default maps per material: `albedo`, `normal`, `roughness`, `metallic`, `ao` (5 SEPARATE maps per `polyhaven_provider.rs:56-62`).
  - Map name aliases per `polyhaven.rs:340-351`: "albedo"→[Diffuse/diff/Color], "normal"→[nor_gl/nor_dx/Normal], "roughness"→[Rough/Roughness], "metallic"→[Metal/Metalness], "ao"→[AO/ambient_occlusion], "height/displacement"→[Displacement/disp/Bump/Height]. **NO "arm" map name supported.**

- **§1.2.2 Invocation example**: `cargo run -p astraweave-assets -- fetch --provider polyhaven` reads default manifest; downloads each entry's 5 maps; deposits in `assets/_downloaded/<handle>/`.

- **§1.2.3 Candidate PolyHaven sets per material** (Andrew-gate surface point 1 enumerated):
  - cobblestone → `old_stone_path` (existing manifest entry; preserved).
  - default → SKIP per Andrew-gate (iii) reclassification (dead code; separate cleanup).
  - gravel → `gravel_concrete_03` or `gravel_embedded_concrete` (2K).
  - ice → `ice_001` (2K).
  - metal_rusted → `rust_coarse_01` (2K).
  - moss → `moss_01` or `mossy_forest_floor` (2K).
  - mountain_rock → REUSE existing `assets/textures/aerial_rocks_01_*_4k.jpg` (downsample to 2K).
  - mud → `mud_riverbed_01` (2K).
  - snow → `snow_03` (2K).
  - wood_planks → `wood_floor` (existing manifest entry maps to `wood_floor_deck`; preserved).

- **§1.2.4 `assets_src/materials/` pre-state**: 12 source triples confirmed (cloth, dirt, forest_floor, grass, plaster, rock_lichen, rock_slate, roof_tile, sand, stone, tree_bark, tree_leaves). Unchanged this session.

- **§1.2.5 ARM-to-ORM rename verification — CAPABILITY GAP IDENTIFIED**: audit §3.1 anticipated PolyHaven's `_arm_*.jpg` as fetcher output (rename-only to `_mra.png`). Fetcher actually delivers FIVE separate maps (roughness/metallic/ao as individual PNGs). Producing AstraWeave's ORM-packed `_mra.png` requires channel-packing post-processing (combine R=AO + G=Roughness + B=Metallic into one RGB PNG).

- **§1.2.6 Per-material conversion pipeline — TOOLING GAP IDENTIFIED**: image manipulation tooling NOT available in dev environment. `which magick` returns not-found; only Windows file-system `convert` (NOT ImageMagick) is on PATH. No ffmpeg. The fetcher's individual-map output cannot be channel-packed without external tooling install.

- **§1.2.7 `default` material investigation**: defined at `astraweave-render/src/material_library.rs:125-129` as canonical material slot ID=12, display_name="Default". Not referenced by any biome `materials.toml`. UI-only presence (in `MATERIAL_NAMES` constant). Treatment recommendation surfaced (i)/(ii)/(iii).

- **§1.2.8 Anti-drift discipline**: 17 named temptations held throughout; explicitly avoided fetcher code modification (resisted #6).

**Andrew-gate surface points processed inline 2026-05-14**:

- **Surface 1 (A.1 path)**: User selected **"Pause + extend fetcher (recommended)"**. Open Sub-phase A.0 fetcher capability extension session before A.1 acquisition. Modify `polyhaven_map_names` to add `"arm" => vec!["arm", "ARM"]` mapping; verify PolyHaven serves `<id>_arm_<res>.jpg` directly; ~5-line change + tests.

- **Surface 2 (`default` treatment)**: User selected **"(iii) Flag for separate cleanup"**. `default` reclassified as dead-code; remove from `MaterialLibrary::MATERIALS` + `MATERIAL_NAMES` + `MATERIAL_DISPLAY_NAMES` in a separate cleanup commit out of this campaign's scope. Acquisition target reduced from 10 to 9 materials.

**Sub-phase chain reframe**:

| Sub-phase | Status before | Status after |
|-----------|--------------|--------------|
| A.0 — Fetcher capability extension (NEW) | n/a | NOT STARTED |
| A.1 — Source acquisition (re-scoped: 9 materials) | NOT STARTED | BLOCKED on A.0 |
| A.2 — Bake (re-scoped: 21 materials; default excluded) | NOT STARTED | BLOCKED on A.1 |
| `default` cleanup (NEW; out-of-campaign) | n/a | NOT STARTED |
| Sub-phases B, C, D, E | NOT STARTED | unchanged |

**Methodology lessons applied**:

- **§7.1 instrument-and-narrow**: not applicable (analytical pre-execution; no instrumentation needed for fetcher characterization).
- **§7.2 pre-execution actual-code verification**: ✅ applied (8 sub-items mandatory; surfaced critical capability gap before any irreversible operation).
- **§7.3 symbol/signature pinning**: ✅ applied (re-greped `polyhaven_map_names`, `MATERIAL_NAMES`, manifest format, `default` material location).
- **§7.4 drift documentation**: ✅ applied (this entry documents inventory-vs-actual drift — fetcher capability gap differs from audit §3.1's "rename-only" presumption).
- **§7.5 semantic-invariant tests**: deferred.
- **§7.8 audit-era misclassification**: ✅ applied — audit §3.1's "rename-only" framing was retrospectively imprecise once fetcher capability was characterized. Marker commit records drift without retro-revising audit (per Sub-phase 3 chronological-archeology discipline).
- **§7.9 state-propagation pathway equivalence**: applicable — fetcher pathway (5 individual maps) and audit-anticipated pathway (single `_arm` packed) currently diverge. A.0 fetcher extension unifies pathways.
- **§7.10 candidate (content-vs-structural-defect distinction)**: empirically tested this session — content-driven work surfaces capability gaps in tooling that wouldn't arise in structural-defect investigation. Reinforces §7.10 candidate elevation at Sub-phase E.

**New methodology lesson candidate surfaced** (deferred elevation per anti-drift discipline):

**§7.11 candidate — research-pass-recommendation falsification at pre-execution**: research-pass audits surface recommendations based on code-reading + inventory characterization. Pre-execution at sub-phase start may falsify those recommendations when tooling boundaries surface that the research-pass couldn't characterize without invoking the tool. This is sibling to §7.1 (three-rounds-wrong-from-code-reading) but at content-driven granularity: research recommendations about content-pipeline behavior are falsifiable by pre-execution. Cleanup-D research-pass recommended (q) q-1 direct fix based on code-reading; this proved correct. A.1 audit recommended "rename-only" PolyHaven conversion based on file-name inspection; this proved incorrect once fetcher behavior was characterized. §7.11 candidate: when research-pass recommendations touch tooling behavior (not just code structure), pre-execution must invoke or characterize the tooling, not just inspect its output artifacts. Elevation deferred to Sub-phase E closeout.

**Out of scope per A.1 prompt §0.1 + §1.2.8 anti-drift discipline (17 named temptations); all 17 held**:
- NO fetcher invocation (per "Pause" decision).
- NO `aw_asset_cli cook` invocation (A.2's scope; further deferred).
- NO Tier 1 runtime PNG replacement (Sub-phase C's scope).
- NO biome TOML modifications.
- NO modifications to `MaterialLibrary` or any canonical pipeline primitive (the `default` cleanup is a SEPARATE commit; NOT this commit).
- NO ARM-to-ORM channel swizzle attempts.
- NO `polyhaven` biome schema reconciliation.
- NO `astraweave-assets` fetcher code modifications (A.0's scope; separate session).
- NO additional PolyHaven sets beyond 9-material scope (post-reclassification).
- NO pre-baking source PNGs to KTX2.
- NO performance measurements.
- NO CLAUDE.md amendment elevation.
- NO ARCHITECTURE_MAP.md updates.
- NO modifications to `assets/materials/placeholder_backup/`.
- NO higher-than-2K acquisition.
- NO Editor Multi-Tool Architecture Sub-phase 4+ work.
- NO Regional Archetype Variation resumption.

**Forward chain post-this-marker-commit**:

1. **Sub-phase A.0 prompt drafted** (next session): fetcher capability extension. Add `"arm"` map name to `polyhaven_map_names`; verify PolyHaven serves `<id>_arm_<res>.jpg` directly; minimal test coverage. ~5-10 line code change.
2. **Sub-phase A.0 lands**: code change committed; cargo check + tests clean.
3. **`default` cleanup commit** (parallel-work; out of this campaign): remove `default` from `MaterialLibrary::MATERIALS` + `MATERIAL_NAMES` + `MATERIAL_DISPLAY_NAMES`. Reclassify slot 12 as reserved. Verify no biome TOML references; verify UI gracefully handles 21-material library.
4. **Sub-phase A.1 (re-scoped) lands**: fetch 9 materials via extended fetcher; rename `_arm.png` → `_mra.png`; deposit in `assets_src/materials/`. Andrew-gate per per-material approval (Surface point 1 recommendations now load-bearing).
5. **Sub-phase A.2 (re-scoped)**: bake 21 materials via `aw_asset_cli cook`.
6. **Sub-phase C biome batches → D → E** per campaign forward chain.

**Files modified this commit**:
- `docs/current/TERRAIN_ASSET_QUALITY_CAMPAIGN.md` (this doc) — Status header + §11 + §12 marker entry.

**Files NOT modified this commit**:
- `assets_src/materials/` — UNCHANGED.
- `assets/materials/` — UNCHANGED.
- `tools/astraweave-assets/src/` — UNCHANGED (A.0's scope).
- `astraweave-render/src/material_library.rs` — UNCHANGED (separate `default` cleanup commit).
- `aw_pipeline.toml` — UNCHANGED.
- Any biome `materials.toml` / `arrays.toml` — UNCHANGED.

**Scope held**: marker-commit session produced only this campaign doc's Status header + §11 + §12 update. NO production code changes. NO asset file changes. NO modifications to predecessor commits. Working tree unrelated changes intentionally not staged.

---

### 2026-05-14, Sub-phase A.0 (fetcher capability extension), this commit

**Sub-phase A.0.A landed. `astraweave-assets` PolyHaven provider extended to fetch ARM-packed map type. Audit §3.1 "rename-only" conversion premise RESTORED. Sub-phase A.1 acquisition UNBLOCKED (pending `default` cleanup parallel-commit + A.1 prompt re-draft for 9 materials).**

**§7.11 methodology lesson candidate empirically validated**: research-pass-recommendation falsification at pre-execution. Audit §3.1's "rename-only" premise was tooling-behavior-dependent; A.1 pre-execution surfaced the gap (per `eab972aea` marker); A.0 closes it. The pattern: research-pass recommendations touching tooling behavior require pre-execution that invokes or characterizes the tooling, not just inspects output artifacts. Elevation deferred to Sub-phase E closeout per chronological-archeology discipline.

**Pre-execution verification per §1.2 of A.0 prompt (6 sub-items mandatory)**:

- **§1.2.1 `polyhaven_map_names` schema**: `fn polyhaven_map_names(&self, user_map_name: &str) -> Vec<&str>` at `polyhaven.rs:341`. Match-expression-based (NOT HashMap). 5 existing entries pre-A.0: `albedo` → [Diffuse/diff/diffuse/Color]; `normal` → [nor_gl/nor_dx/Normal]; `roughness` → [Rough/Roughness]; `metallic` → [Metal/Metallic/Metalness]; `ao` → [AO/ao/ambient_occlusion]; `height|displacement` → [Displacement/disp/Bump/Height].

- **§1.2.2 Downstream consumers**: single call site at `polyhaven.rs:145` inside `resolve_texture()`. Iterates external `requested_maps: &[String]`; calls `polyhaven_map_names` per entry; tries each alternative name against PolyHaven API `files.maps[ph_name]` response. The `requested_maps` default list constructed at `polyhaven_provider.rs:56-62` in AssetType::Texture branch — must add "arm" there too for fetcher to actually request ARM by default.

- **§1.2.3 ARM URL convention**: `rg 'arm|ARM'` against `tools/astraweave-assets` returned NO existing references — URL construction is fully mapping-driven (PolyHaven API metadata `files.maps[ph_name]` returns nested `{<res>: {<format>: {url, size, md5}}}` for whichever names match). Adding "arm" to mapping is structurally sufficient; no hardcoded ARM URL paths to add.

- **§1.2.4 Test fixture coverage**: `test_polyhaven_map_names_mappings` at `polyhaven.rs:405` tests existing keys (`albedo`, `normal`, `does_not_exist` empty). Added ARM assertion in same test: verifies `arm` key returns Vec containing "arm" + "ARM". `test_resolve_texture_selects_fallback_resolution_and_urls` at line ~470 uses `["albedo", "normal"]` request list; doesn't need ARM update.

- **§1.2.5 PolyHaven ARM availability**: Existing `assets/textures/` content includes `aerial_beach_01_arm_4k.jpg`, `aerial_rocks_01_arm_4k.jpg` — confirms PolyHaven CDN serves ARM-packed files for these sets. PolyHaven's `/files/<id>` API endpoint exposes a top-level "arm" key in the response per filename evidence + per inferred consistent naming convention with `_arm_*` URL paths. Live API verification deferred to A.1's first fetch invocation per single-concern session discipline + anti-drift temptation #1 ("Resist fetching A.1 sources during A.0").

- **§1.2.6 Anti-drift discipline**: 15 named temptations held throughout — no opportunistic fetch, no schema refactor, no other-provider extension, no biome TOML modifications, no MaterialLibrary modifications, no `default` cleanup bundling, no frame alert investigation, no other-asset-crate modifications, no CLI flag additions, no documentation updates, no pre-baking, no schema reconciliation.

**Code change applied (3 minimal edits across 2 files)**:

1. **`tools/astraweave-assets/src/polyhaven.rs:341+`** — added "arm" match arm in `polyhaven_map_names`: `"arm" => vec!["arm", "ARM", "Arm"]`. Plus 3-line comment documenting ARM = AO+Roughness+Metallic packing per PolyHaven CDN; channel layout R=AO, G=Roughness, B=Metallic matches AstraWeave ORM convention per `pbr_terrain.wgsl:334-338` + audit §3.1.

2. **`tools/astraweave-assets/src/polyhaven_provider.rs:56-62+`** — added `"arm".to_string()` to default `requested_maps` Vec in AssetType::Texture branch of `resolve()`. Plus comment documenting addition.

3. **`tools/astraweave-assets/src/polyhaven.rs:411+`** — added ARM assertion to `test_polyhaven_map_names_mappings`: calls `polyhaven_map_names("arm")`; asserts result contains both `"arm"` and `"ARM"` aliases.

Net change: +18/-0 lines across 2 files (production + test inline).

**Verification outcomes**:

- `cargo check -p astraweave-assets`: clean (31s incremental build).
- `cargo check --workspace [exclusions]`: clean (only pre-existing nalgebra + astraweave-ai dead_code warnings; unrelated).
- `cargo test -p astraweave-assets --lib polyhaven`: 10/10 polyhaven module tests pass including updated `test_polyhaven_map_names_mappings`.
- `cargo test -p astraweave-assets --lib`: **124/124 lib tests pass** (full astraweave-assets test suite preserved; no regressions).
- Live-network fetch verification: **DEFERRED to A.1's first fetch invocation** per single-concern discipline + anti-drift #1. A.0 verification rests on structural correctness (mapping addition correctly returns ARM aliases; test asserts contract; fetcher infrastructure is mapping-driven per §1.2.3 so addition is purely additive).

**A.1 unblocking**:

- A.1 acquisition can now fetch ARM-packed map directly via `cargo run -p astraweave-assets -- fetch --provider polyhaven`. Fetcher will pull `_arm_<res>.<ext>` for any PolyHaven set that exposes ARM in its API response.
- Per-material recommendations preserved from `eab972aea`:
  - cobblestone → `old_stone_path` (existing manifest entry).
  - gravel → `gravel_concrete_03` (2K).
  - ice → `ice_001` (2K).
  - metal_rusted → `rust_coarse_01` (2K).
  - moss → `moss_01` (2K).
  - mountain_rock → REUSE existing `aerial_rocks_01` (4K → 2K downsample).
  - mud → `mud_riverbed_01` (2K).
  - snow → `snow_03` (2K).
  - wood_planks → `wood_floor` (existing manifest entry maps to `wood_floor_deck`).
- Acquisition target: **9 materials** (`default` excluded per Andrew-gate Surface 2 (iii) reclassification 2026-05-14).
- A.1 workflow: extend `assets/asset_manifest.toml` with 9 new entries → `cargo run -p astraweave-assets -- fetch --provider polyhaven` → rename `<set>_arm_2k.png` → `<material>_mra.png` → deposit in `assets_src/materials/`. Audit §3.1 "rename-only" conversion now restored end-to-end.

**Methodology lessons applied**:

- §7.2 pre-execution actual-code verification: ✅ (6 sub-items mandatory; surfaced no scope expansion needs).
- §7.3 symbol/signature pinning: ✅ (verified schema shape, call sites, URL construction).
- §7.4 drift documentation: ✅ (this entry + commit body documents schema-vs-A.1-recommendation alignment now restored).
- §7.5 semantic-invariant: code change preserves all existing map types; only adds ARM (additive).
- §7.7 structural axiom: not directly applicable (single mapping extension; not boundary-spanning resource identity).
- §7.8 audit-era misclassification: applied — audit §3.1's framing was tooling-behavior-dependent; A.0 restores premise without retro-revising audit.
- §7.9 state-propagation pathway equivalence: applied — A.0 enables A.1's fetcher-driven pathway to produce equivalent end-state (ORM-packed `_mra.png`) to the audit-anticipated rename-only pathway. Cook pathway will then produce equivalent KTX2 to manual-edit pathway, closing the §7.9 gap at A.2.
- §7.10 candidate (content-vs-structural-defect distinction): not the active lesson this commit; deferred.
- §7.11 candidate (research-pass-recommendation falsification at pre-execution): empirically validated by A.0's existence + outcome. Pre-execution invoked the tooling characterization; surfaced the gap; A.0 closed it. Validates §7.11 elevation candidate; deferred to Sub-phase E closeout.

**Discipline shift observation**: A.0 demonstrates Sub-phase 3's single-concern session discipline (§13.4) scales DOWN as well as up. Editor Multi-Tool Architecture Sub-phase 3 single-concern sessions were at instrument/closure/revert/fix granularity (substantial work per session). A.0 is single-concern at ~5-10-line-code-change granularity. The discipline absorbs both scales: each session does ONE thing well, regardless of size.

**Forward chain post-A.0**:

1. **`default` cleanup parallel commit** (out-of-campaign; can land before or after A.1):
   - Modify `astraweave-render/src/material_library.rs` to remove `default` from MATERIALS array (id=12); reduce named count from 22 to 21; increase reserved count from 10 to 11.
   - Modify `MATERIAL_NAMES` + `MATERIAL_DISPLAY_NAMES` constants similarly.
   - Remove `assets/materials/default.png` + `_n.png` + `_mra.png`.
   - Verify no biome TOML references default; verify UI gracefully handles 21-material library.
2. **Sub-phase A.1 prompt re-drafted** (next session): 9 materials (per `eab972aea` recommendations); rename-only conversion workflow (audit §3.1 premise restored).
3. **Sub-phase A.1 lands**: 9 materials acquired into `assets_src/materials/`; Andrew-gate per-material approval inline.
4. **Sub-phase A.2 lands**: `aw_asset_cli cook` produces canonical KTX2 for 21 materials.
5. **Sub-phase C biome batches → D → E** per campaign forward chain.

**Files modified this commit**:
- `tools/astraweave-assets/src/polyhaven.rs` (mapping + test).
- `tools/astraweave-assets/src/polyhaven_provider.rs` (default requested_maps).
- `docs/current/TERRAIN_ASSET_QUALITY_CAMPAIGN.md` (Status header + §11 + §12 entry).

**Files NOT modified this commit**:
- `assets_src/materials/` — UNCHANGED (A.1's scope).
- `assets/materials/` — UNCHANGED (Sub-phase C's scope).
- `aw_pipeline.toml` — UNCHANGED.
- Any biome `materials.toml` / `arrays.toml` — UNCHANGED.
- `astraweave-render/src/material_library.rs` — UNCHANGED (`default` cleanup is separate commit).
- `tools/aw_asset_cli/` — UNCHANGED.
- `astraweave-asset/` + `astraweave-asset-pipeline/` — UNCHANGED.
- Any prior campaign chain commit — UNCHANGED.

**Out of scope per A.0 prompt §0.1 + §1.2.6 anti-drift discipline (15 temptations held)**:
- NO A.1 acquisition fetch (A.1's scope).
- NO `default` cleanup (separate commit).
- NO `aw_asset_cli cook` (A.2's scope).
- NO Sub-phase C / D / E work.
- NO MaterialLibrary or canonical pipeline modifications.
- NO biome TOML modifications.
- NO modifications to other asset crates.
- NO `polyhaven_map_names` schema refactor.
- NO other-provider (Kenney, itch.io, direct URL) extensions.
- NO frame alert investigation.
- NO CLI flag additions.
- NO live-network fetch verification (deferred to A.1).
- NO CLAUDE.md amendment elevation.
- NO ARCHITECTURE_MAP.md updates.
- NO Editor Multi-Tool Architecture Sub-phase 4+ work.

**Scope held**: A.0.A session modified `tools/astraweave-assets/src/polyhaven.rs` + `polyhaven_provider.rs` (production code change scope ≤ 20 lines net) + this campaign doc (Status header + §11 + §12 entry). All anti-drift temptations held. Single-concern session pattern preserved.

### 2026-05-16, Sub-phase A.3 (grassland biome validate-only), this commit

**Andrew-gate re-scope at Phase 2: original A.3 premise falsified by Phase 1 audit.** Sub-phase entered as "wire up grassland to consume the 9 Tier 1 PNG source materials" per prompt. Pre-execution audit revealed all four premise components were already satisfied:

1. **Grassland was already wired.** `assets/materials/grassland/materials.toml` declares 5 layers (grass / rock_smooth / dirt / sand / moss-via-rock_lichen) with PNG paths to `../<material>.png` runtime root. `arrays.toml` maps each layer to a u32 index. All paths resolve to git-tracked PNGs that also have corresponding `.ktx2` outputs from a prior cook pass.
2. **All 27 Tier 1 PNGs were already deployed at runtime root.** `assets/materials/<material>.png|_n.png|_mra.png` for all 9 of gravel/metal_rusted/moss/snow/wood_planks/mountain_rock/mud/cobblestone/ice (27 files). All git-tracked. Deployment provenance is outside this session's commit chain — A.1/A.1.B/A.1.C only wrote to `assets_src/materials/`; the runtime root copies must have been added by separate automation or manual placement in an earlier commit.
3. **The renderer is PNG-native via canonical loader.** Verified pre-A.3 at [`material_loader.rs:284-307`](../../astraweave-render/src/material_loader.rs) (extension dispatches PNG/JPG/TGA via `image` crate; KTX2 via basis_universal transcoder); [`material.rs:706-714`](../../astraweave-render/src/material.rs) (loader's own docs use `.png` paths); [`terrain_material.rs:265-344`](../../astraweave-render/src/terrain_material.rs) (default `TerrainMaterialConfig` hardcodes `.png` paths); [`renderer.rs:1548`](../../tools/aw_editor/src/viewport/renderer.rs) (editor's biome surface-map loader uses `image::open` directly); [`terrain_material_manager.rs:118-128`](../../astraweave-render/src/terrain_material_manager.rs) (`LayerTextures` consumes raw `Option<&[u8]>` RGBA8 slices).
4. **`load_pack_from_toml` schema accepts the existing grassland config.** Verified at [`material.rs:410-555`](../../astraweave-render/src/material.rs) — required fields `key` + optional `albedo`/`normal`/`mra`/`tiling`/`triplanar_scale`; path resolution via `base_dir.join(p).normalize()`. Existing grassland config uses all permitted fields correctly.

**Andrew-gate decision**: re-scope from "wire up" → "validate-only" with these adjustments:
- Commit content = validation evidence + discovery doc (not assets / config / docs-only)
- §7.11 fifth pillar codified (scope-fit validation)
- 27 Tier 1 PNG runtime root deployment documented; disposition deferred
- Harness path + cost surfaced before Phase 3 execution

**Validation harness selected**: `cargo test -p astraweave-render --features textures --test headless_integration`. Cost: 32.5s rebuild + 1.4s test execution. Visual splat-shader rendering verification infeasible from CLI agent context (no screenshot capture). Visual confirmation deferred to Andrew running biome_showcase / unified_showcase / editor locally post-commit.

**Validation evidence** (verbatim test output):

```text
running 1 test
test headless_biome_pack_and_pipeline_compat ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.42s
```

The test loads `assets/materials/grassland/materials.toml` + `arrays.toml` via `MaterialManager::load_pack_from_toml` against a headless wgpu device, creates the canonical 5-binding material BGL (albedo array + sampler, normal array + sampler, mra array), and confirms no panics. PNG decode succeeded for all 5 layers' albedo / normal / mra triples. GPU texture array upload succeeded. Bind group layout match confirmed.

**What this evidence does NOT cover**: the splat shader does not run in this harness. "Splat blending visibly working on GPU" per Andrew's adjustment can only be confirmed by running a windowed harness manually — biome_showcase / unified_showcase / editor with grassland scene loaded. Deferred to manual run.

**§7.11 fifth pillar codified** (see §13.5 below for full elevation candidate): scope-fit validation as a distinct pre-execution concern. The A.1 → A.2 → A.3 chain solved a problem the codebase had already solved — Tier 1 PNGs were already deployed, grassland was already wired. Pre-execution validation must include "does this work need doing" alongside slug-correctness, provider-fit, and tool-maturity. Five empirical pillars now grounded.

**27 Tier 1 PNG runtime root deployment — disposition deferred**: All 27 files at `assets/materials/<material>{,_n,_mra}.png` for 9 materials are git-tracked. They co-exist with `assets_src/materials/` copies (true source location for the campaign). Two-location duplication has cleanup implications:
- **Option 1**: Treat runtime root as canonical for runtime-consumed materials; delete `assets_src/materials/` copies post-cook-tooling-fix (future sub-phase).
- **Option 2**: Treat `assets_src/materials/` as canonical source; delete runtime root copies; rely entirely on `aw_asset_cli cook` pipeline (which is currently broken per A.2; would block).
- **Option 3**: Accept dual location as load-bearing redundancy; document but don't fix.

Decision deferred to Andrew. A.3 does NOT delete, move, or rename either copy.

**Forward chain decision** (per Andrew adjustment "Forward chain after A.3 commits depends on validation outcome"):

The validation outcome is **positive at the load-path level** (headless test passes), **unverified at the visual-rendering level** (deferred to manual run). Two branch points:

- **If splat blending visibly works on existing grassland (Andrew confirms manually)**: Sub-phase C (per-biome content upgrade) is largely unnecessary — biomes are already consuming Tier 1 PNGs with acceptable visual quality. Forward chain jumps to Sub-phase D (performance verification) or directly to Phase 2 per-vertex material data work per `docs/current/TERRAIN_MATERIAL_SYSTEM_CAMPAIGN.md`.
- **If splat blending is broken or aesthetically incoherent**: new sub-phase scoped to the specific defect; biome-rationalization audit may also be appropriate before per-biome work.

**Methodology lessons applied** (from Editor Multi-Tool Architecture Sub-phase 3 §13):
- §7.2 pre-execution actual-code verification: ✅ applied (Phase 1 audit caught premise falsification before any file modification).
- §7.3 symbol/signature pinning: ✅ applied (re-grepped `load_pack_from_toml`, `grassland` references, harness candidates).
- §7.4 drift-finding documentation: ✅ applied (this entry documents 4-component premise falsification + 27 PNG deployment finding).
- §7.7 structural axiom: not directly invoked (no wrapped-component identity question).
- **§7.11 candidate** (this campaign's fifth-pillar accumulation): scope-fit validation added; full elevation codified at §13.5.

**Out of scope (17 named temptations); all 17 held**:
- NO renderer source touched (no `astraweave-render/`, no `astraweave-terrain/`).
- NO cook tool touched.
- NO desert / forest / mountain / tundra / swamp / beach / river / terrain / polyhaven biome configs touched.
- NO MaterialLibrary change.
- NO `asset_manifest.toml` modifications.
- NO symlink (n/a — no asset placement happened).
- NO mid-execution KTX2 conversion.
- NO invented `materials.toml` schema fields.
- NO grassland layer count change (still 5).
- NO aesthetic parameter retuning (no tiling / blend_sharpness / triplanar changes).
- NO desert / forest / Tier 2 / Tier 3 work bundled.
- NO `sky_night` fix.
- NO unrelated test-file warning fixes (`coverage_booster_render.rs` E0432/E0603/E0061 errors observed but left alone).
- NO runtime `assets/materials/` directory restructure.
- NO README / external campaign doc updates.
- Phase 1 audit was non-skippable (executed); Phase 2 gate was non-skippable (executed; Andrew approved validate-only re-scope).

**Scope held**: A.3 session modified only this campaign doc (Status header + §11 + §12 entry + §13.5 codification). No production code change. No asset file change. No test-file change. No cook tool change. Single-concern session pattern preserved.

---

### 2026-06-02, Sub-phase D-core (absolute texture-footprint baseline), this commit

**Reframe basis.** D-as-§6.2 was a placeholder→replaced perf *delta* (D.1 placeholder baseline → D.2 post-Sub-phase-C comparison). The D-scope fork (2026-06-02) established both endpoints no longer exist: Sub-phase C is demand-driven (gate-scope reconciliation), the real PBR Tier 1 set is already deployed and consumed (A.3), and no clean placeholder baseline is recoverable (`assets/materials/placeholder_backup/` is a heterogeneous partial archive of a different material set, not a swap-in). The delta is dissolved. D-core measures the **absolute GPU texture-memory footprint of the deployed real-PBR set**, verdicted against a stated budget. Analytical-only (no code, no harness): the upload model makes the math exact.

**Upload model (load-bearing for exactness).** `MaterialManager::build_arrays` ([`material_loader.rs:510-536`](../../astraweave-render/src/material_loader.rs)) **hardcodes `width=1024, height=1024`** and resizes every layer (Lanczos3) to 1024² regardless of source PNG dimensions. Three D2Array textures per pack — albedo `Rgba8UnormSrgb`, normal `Rgba8Unorm`, mra `Rgba8Unorm` (all 4 bytes/px). Full mip chain: `mip_level_count_for(1024) = 11` (multiplier Σ(1/4)^i ≈ 1.3333). Cook-to-BC path remains broken (A.2) → runtime is uncompressed RGBA8.

**Footprint (exact, per actual deployed dimensions):**
- Per layer-slot (1024² RGBA8 + 11 mips): **5.333 MiB**. Per material (albedo+normal+mra): **16.0 MiB**.
- Standard 5-layer biome pack (3 arrays × 5 layers): **80.0 MiB**.
- All 10 packs resident (worst case): **~800 MiB**.
- Residency is one active biome pack at a time on the terrain forward path (`terrain_material_manager` singular `material_cache`, "current biome pack") → **realistic terrain-material VRAM = 80 MiB**.
- Tier 1 consumption (6 of 9 materials, across 5 biomes): beach[gravel]; mountain[mountain_rock,snow,gravel]; river[mud,gravel,moss]; swamp[mud,moss]; tundra[snow,ice,gravel,mountain_rock]. Each Tier 1 material, as a pack layer, costs the uniform 16.0 MiB (3 maps) — no different from any other layer, because all are normalized to 1024².

**Budget stated (doc-derived + engine-derived):**
- **§2.7 (doc):** target 2048² **BC7/BC5-compressed** ≈ 12 MB/material × 32 slots = **~384 MB peak**; 1024² fallback "~96 MB total". The budget model assumes compression.
- **Engine (`gpu_memory.rs` `CategoryBudget` defaults):** Textures **soft 256 MB / hard 512 MB** (total budget 2 GB default).

**Verdict against the stated budget: ACCEPTABLE (realistic residency) + OPTIMIZATION-CANDIDATE (format), NOT blocking.**
- One active biome pack at 80 MiB is **31% of the 256 MB soft texture budget** and **21% of §2.7's 384 MB peak** → acceptable at typical residency.
- BUT the deployment is uncompressed RGBA8 = **4× the VRAM-per-material §2.7's own budget assumed** (16 MiB vs 4 MiB compressed). The single highest-value terrain-texture optimization is **fixing the broken BC7/KTX2 cook path** (A.2) → 80 MiB → ~20 MiB per pack. Deferred to a conditional separate session per §6.2 D.3 (D-core verdicts only; it does not optimize).
- Forward-looking risk (not current): if multi-biome blending ever holds many packs resident, N×80 MiB scales toward the 512 MB hard budget (~6 packs ≈ 480 MiB). Single-pack forward path makes this non-blocking today.

**Claim bounds (explicit):**
- IS: texture-memory footprint of the deployed Tier 1 set, verdicted against a stated budget.
- NOT frame-time: the editor 145.5ms / `Dropping ViewportRenderer GPU resources` alert is **editor-runtime perf routed OUT of TAQ** per §6.3 (observed with real PBR already deployed → asset-content-independent).
- NOT a placeholder delta (dissolved).
- Does NOT touch the 27-PNG runtime-vs-source divergence (its own session; D-core measures the runtime set the loader consumes).

**Incidental findings surfaced (flagged, NOT fixed this session — single-concern discipline):**
1. **Source-disk resolution inconsistency is GPU-invisible.** Source PNGs span 1024²/2048²/4096² (cloth/plaster/rock_lichen/tree_bark/tree_leaves albedo+normal are 4096²); ALL are downsampled to 1024² on upload. The inconsistency costs repo size + load-time decode (decoding 4096² to discard ~93%), **not VRAM**. This *falsifies the D-scope-fork's "inconsistent VRAM" hypothesis* and routes the concern to the 27-PNG cleanup session (disk hygiene), not GPU optimization.
2. **3 of 9 Tier 1 materials unreferenced by any biome pack** (cobblestone, metal_rusted, wood_planks) → zero terrain VRAM; UI-only per §5.2 C.9.
3. **`polyhaven` pack references 10 missing files** (`aerial_rocks_albedo/normal`, `cobblestone_albedo/normal`, `metal_plate_*`, `plastered_wall_*`, `wood_floor_*` — `_albedo`/`_normal` naming, none found anywhere in `assets/materials/`). The pack would fail/fallback on load — a silent-failure/missing-asset finding. Routes to a separate decision (polyhaven is a showcase pack, not a real biome, not Tier 1).
4. **Engine self-reported material memory is a stub.** `MaterialManager` hardcodes `gpu_memory_bytes: 1024*1024*10` (10 MiB placeholder, [`material.rs:841`](../../astraweave-render/src/material.rs)) — any in-engine "material memory" readout is fake, which is why analytical measurement was required.

**Scope held**: D-core modified only this campaign doc (§11 status + this §12 entry). No production code change. No asset change. No test change. No optimization performed (verdicts only). 27-PNG divergence untouched; editor frame-alert untouched; Sub-phase C untouched; splat-coherence untouched. Single-concern session preserved.

---

### 2026-06-02, Asset-hygiene cleanup-queue findings (durable records), this commit

Two asset-hygiene items surfaced during the TAQ resumption, both **out of TAQ execution scope** (separately owned), recorded here as durable ground truth so their eventual cleanup sessions start from established facts rather than re-deriving them. **These are records, not fixes** — nothing is moved, deleted, re-encoded, or repaired this session. The E-closeout cleanup-queue hand-off references this entry.

#### Cleanup-queue Finding 1 — 27-PNG runtime-vs-source divergence

**What.** The 9 Tier 1 materials exist as PNG triples in two locations: `assets_src/materials/` (source/archival — where A.1/A.1.B/A.1.C acquisition wrote) and the runtime root `assets/materials/` (load-bearing — the loader reads here via each biome `materials.toml` → `../<material>.png`). A.3 surfaced the dual location; the D-scope-establishment session (2026-06-02) characterized the divergence.

**The divergence.** All 9 runtime-root albedo PNGs differ by md5 from their `assets_src/` copies — a *processed deployment with inconsistent processing* of undocumented provenance (A.3: "added by separate automation or manual placement"). Two illustrative cases: **cobblestone** — runtime 1024², source 2048² (downscaled); **snow** — both 2048², different md5 (same-resolution re-encode). md5 pairs recorded at characterization time (albedo `.png`; `_n`/`_mra` not individually hashed — cleanup re-confirms all 27 at execution):

| material | runtime-root md5 | assets_src md5 |
|---|---|---|
| cobblestone | `849c5903e38dcb126eb936d2a37b47bd` | `b30275e28ce5cdcea7b2e027b662e446` |
| gravel | `5c8261525a97b080b93aac2c9456ab94` | `7fb2eac0bfa2d9321a3417ff49d870f2` |
| ice | `9d8d23c1aa994efb1aefece3f5bdabb1` | `b81e6d720f3c5aa7cf270f5193471778` |
| metal_rusted | `9cdeaf02cb7f1a9f3ffb0a671e4d51a8` | `ccfbc8f04be9a57c4bf4f57e13392ef2` |
| moss | `6b8706ed83bb5da844aa58922a8495d7` | `b768786e5303fb811853e6d1e7d1ba57` |
| mountain_rock | `a1d56f6a3f5fc31f8e163e60c90912f0` | `9d5d99885cc7e62e0545bc6443f11de6` |
| mud | `31b863ae1910ab1b9b1cd241f41bcbd7` | `a92d0aa18135c85cd6578663e94b9a94` |
| snow | `e12ad20163e9e29ba73ffda6d776e875` | `5b2d39f81eb73a39d277c6f3dcd58500` |
| wood_planks | `612fed3ed6109c02e84cb0a77b78e3f7` | `42ff36d36dc360935990b1139e787adf` |

**GPU-invisible (D-core qualifier — load-bearing for priority).** The divergence costs **disk size + load-time decode, NOT VRAM.** `material_loader.rs` `build_arrays` hardcodes a 1024² array dimension and Lanczos3-resizes every layer on upload (`material_loader.rs:523-524`), so the runtime resolution spread (and the wider source spread of 1024²/2048²/4096²) is invisible to the GPU — all normalize to 1024². **This reframes the cleanup as disk/decode hygiene, not a perf optimization** (D-core baseline, commit `5881b777f`).

**Open questions the cleanup inherits** (NOT decided here): which set is canonical (runtime-root the loader reads, or `assets_src/` the acquisition wrote)? document or normalize the deployment processing? regenerate the runtime set deterministically from source (and at what target resolution, given GPU only uses 1024²)? prune the redundant set?

**Scope ownership.** Out of TAQ execution scope — disk/asset hygiene, separately owned.

#### Cleanup-queue Finding 2 — `polyhaven` pack references 10 missing files (silent-fallback)

**What.** The `polyhaven` material pack's `materials.toml` references 10 texture files not present on disk (verified absent anywhere under `assets/materials/`, D-core session): `aerial_rocks_albedo.png`, `aerial_rocks_normal.png`, `cobblestone_albedo.png`, `cobblestone_normal.png`, `metal_plate_albedo.png`, `metal_plate_normal.png`, `plastered_wall_albedo.png`, `plastered_wall_normal.png`, `wood_floor_albedo.png`, `wood_floor_normal.png` (5 materials × albedo+normal; polyhaven uses `_albedo`/`_normal` naming and declares no mra map).

**Failure mode (precise).** Missing files do **not** fail the load. `build_arrays` hits `load_rgba(p)` → `Err`, logs `log::warn!("[materials] WARN missing/bad albedo for {}/{}: … → substituting neutral")`, increments `stats.*_substituted`, and continues (`material_loader.rs:785-788`). The pack **reports success** with neutral-substituted layers — so a consumer not watching `log::warn` output sees the pack "work" while it renders flat neutral placeholders for the 10 missing maps. Classic warn-logged silent fallback (no error propagated, no surfaced failure) per the CLAUDE.md silent-failure taxonomy.

**Context.** `polyhaven` is a *showcase pack, not a real biome* (the 8 live `BiomeType` biomes have their own packs). It is not a live-biome blocker, but it is a broken showcase asset misrepresenting its own completeness. Note §10 already flags polyhaven's *schema structure* as preserved-as-is; this finding is the distinct *missing-content* defect.

**Open questions the cleanup inherits** (NOT decided here): acquire the 10 missing files, prune the dangling references, or retire the showcase pack entirely?

**Scope ownership.** Out of TAQ execution scope — showcase-asset hygiene / silent-failure surfacing, separately owned.

**Scope held**: this session modified only this campaign doc (this §12 entry). No code, no asset files moved/deleted/re-encoded, no repairs. Neither finding acted on or decided. Single-concern session preserved.

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

### §13.5 New lesson candidate codified post-A.3 (deferred elevation per anti-drift discipline; Sub-phase E closeout consolidates)

**§7.11 candidate — pre-execution validation must extend beyond input correctness to cover the full pipeline-stage maturity AND scope-fit.**

Each Tier 1 acquisition sub-phase surfaced a distinct pre-execution validation gap that input-correctness checks alone would not catch. Five empirical pillars accumulated across A.1 → A.3:

| Pillar | Sub-phase | Concrete failure | Pre-execution validation that would catch |
|---|---|---|---|
| **1. Stale-slug detection** | A.1 (`f56a76124`) | 3 of 8 code-read PolyHaven slug recommendations 404'd at live fetch (`ice_001`, `mud_riverbed_01`, `old_stone_path`). | Live-browse verification of each slug against the live provider catalog at acquisition time, not at recommendation time. |
| **2. Live-verified-slug success** | A.1.B (`f5387f20e`) | 2 of 2 human-browse-verified slugs (`brown_mud_rocks_01`, `rocky_trail`) fetched cleanly first-try. | Confirms pillar 1's complement: live verification works as the safety net. |
| **3. Provider-fit failure** | A.1.C (`76b15948d`) | PolyHaven's photoscan catalog has no suitable PBR ice texture (Andrew browse 2026-05-15). All 8 slugs in the catalog returned 404 or off-topic content for "ice". | Per-material provider-fit check before slug selection: does this provider's catalog cover this material class at all? Slug-correctness within an assumed-correct provider is insufficient. |
| **4. Tool-maturity gap** | A.2 (no commit) | All 9 input PNGs verifiably correct (Phase 2 channel-order audit), but cook tool nonfunctional on dev env (no toktx/basisu) AND parallel `bake-texture` path produces broken output (placeholder BC encoder writes monochrome 4×4 blocks; DFD always marks sRGB regardless of `ColorSpace::Linear` per [texture_baker.rs:393](../../tools/aw_asset_cli/src/texture_baker.rs#L393)). | Per-pipeline-stage tool-maturity audit: each stage (fetch → relocate → cook → runtime consume) must be independently verified mature; an upstream-correct input cannot rescue a broken downstream stage. |
| **5. Scope-fit validation** | A.3 (this commit) | A.1 → A.2 → A.3 chain wrote prompts and code to "wire up grassland to consume the 9 Tier 1 materials". Phase 1 audit revealed grassland was already wired (5 layers PNG-native) AND all 27 Tier 1 PNGs were already deployed at runtime root. The entire forward chain solved a problem the codebase had already solved. | Pre-execution "does this work need doing" check: read the current state of the artifact the work is supposed to produce, before authoring the work. The Editor Multi-Tool Architecture §7.2 pre-execution actual-code verification lesson generalizes to "verify the work's necessity, not just its design". |

**Codified general rule** (candidate for §7.11 elevation at Sub-phase E closeout):

> Pre-execution validation for any pipeline-touching work must verify (a) input correctness at the relevant boundary, (b) every downstream stage's tool-maturity independently, (c) provider-fit at the material/asset/data-class level (not just within an assumed-correct provider), and (d) scope-fit — does this work need doing given the current state of the target artifact. Any of (a)-(d) failing nullifies upstream correctness; all four are independently verifiable and must be checked independently. The lesson generalizes §7.2 (pre-execution actual-code verification) from "verify the design" to "verify the necessity, the design, and the pipeline maturity at each stage".

**Implications for methodology**:
- The "audit-before-execute" discipline established at A.2 is the operational embodiment of this candidate. Sub-phase prompts that mandate Phase 1 read-only audit + Phase 2 Andrew-gate before any execution are the pattern that catches all four failure modes.
- This generalizes the Editor Multi-Tool Architecture campaign's §7.2 (pre-execution actual-code verification). §7.11 extends §7.2 from "verify the design contract" to "verify the entire pipeline's readiness and the work's necessity".
- Scope-fit (pillar 5) is the highest-impact failure mode: solving a non-problem costs the entire sub-phase chain's effort. Provider-fit (pillar 3) and tool-maturity (pillar 4) are mid-impact (sub-phase blocked but recoverable). Stale-slug (pillar 1) is lowest-impact (one acquisition retry).
- The pillar 5 detection mechanism (read current state of target artifact) is the cheapest pre-execution check of the five; it should always run first.

§7.11 elevation deferred to Sub-phase E closeout per Sub-phase 3 chronological-archeology discipline. §13.3's §7.10 candidate and §13.5's §7.11 candidate consolidate together at E.

> **RESOLVED at E-closeout (2026-06-02).** Both candidates are elevated in the immutable outcome doc `docs/audits/terrain_asset_quality_outcome_2026-06.md` §6 Layer 1: **§7.11** (these five pillars) elevated as-is; **§7.10** (content-vs-structural, §13.3) elevated *as-amended* — its abstract distinction holds, but its operational scaffolding (the placeholder→replaced delta baseline + biome-batched Sub-phase C) was falsified by the resumption arc (no placeholder endpoint; C demand-driven; D-core replaced the delta with an absolute budget-anchored baseline). The outcome doc §6 Layer 2 additionally codifies five *resumption-arc* methodology findings the original campaign plan could not contain.

---

*End of plan*
