# E3-PF — Terrain Campaign Resumption Pre-Flight (read-only reconstruction)

> **Campaign**: Terrain (E3 resumption) · **Session**: E3-PF (director-ratified next work per `BOARD_STATE_2026-07-19.md` §1.4)
> **Branch/commit**: `main` @ `5e59136ac599f175a20a93a670e893293626282b`
> **Date**: 2026-07-20
> **Mode**: READ-ONLY reconstruction — zero code/asset/editor-state changes; this doc is the session's only commit.
> **Method**: 3 parallel read-only recon agents (record / substrate-drift / SP5) + first-hand code trace + editor observation. Every claim carries a citation; observation claims carry screenshots (`d:/tmp/e3pf_staging/`, not committed).

---

## 0. Executive verdict — the remembered plan is one build behind the record

**The E3 build was not "hard-stopped before build." It landed**, on 2026-07-03, as **`d506658d8`** — *"feat(terrain): E3 real build — climate-driven splines, per-archetype landform, coherent biomes, PBR terrain texturing"* — a verified ancestor of HEAD (`git merge-base --is-ancestor`; the pre-rewrite hash `79be1fab9` was renumbered by the AD.6 history rewrite, which is why it no longer resolves). The chat sessions holding this context were lost, but the commit survived, and **`astraweave-terrain`, `astraweave-render`, and `tools/aw_editor/src/terrain_integration.rs` have ZERO commits since it** (`git log d506658d8..HEAD -- <path>` empty for all three).

Consequences that reshape this pre-flight's mission:

1. **E3-terrain.1 (multi-biome consumption wiring) is BUILT.** The single-`primary_biome` render collapse the June recon diagnosed is gone at HEAD (§2). The 19→8 biome→slot mapping the director remembers as the *gating pre-build decision* was **implemented in code during the build** (`terrain_integration.rs:1250-1274`) — Phase 5's package is therefore a **retroactive ratification** of the implemented mapping, not a pre-build decision.
2. **E3-terrain.2 (per-archetype landform / F.7) is BUILT** (build Phase A.2): six distinct `bootstrap_splines_*` factories + the None-mask path reading the *selected* archetype's splines (§2.1, §2.2).
3. **E3-terrain.3 (tuning) is PARTIALLY built** — five director-feedback rounds of texturing landed inside the build commit; the remaining tuning surface is enumerated in §6.
4. The campaign to ratify is therefore **not ".1 → .3 → .2"** (that order is overtaken by events) but: **ratify-what-shipped → tune → re-bake → close** (§6).
5. **Documentation debt is the loudest finding**: no build outcome doc exists (the ~50-line commit message is the sole engineering record), and the architecture-trace sync owed by the build session is unpaid across all four relevant traces — `docs/architecture/terrain.md:141` still asserts the exact pre-E3 behavior the build reversed (§3.6).

---

## 1. Phase 0 orientation (evidence)

- `git status`: clean, `main` up to date with `origin/main`; HEAD `5e59136ac`.
- Close-out (`AD6_CAMPAIGN_CLOSEOUT.md`) + board-state (`BOARD_STATE_2026-07-19.md`) read. Binding rails honored: no history operations, no asset mutations, `actionlint` for workflow edits (none made), read-only session.
- **CI board on HEAD `5e59136ac`** (`gh run list`): `Clippy Lint (Unwrap Prevention)` FAIL + `Rust Cache Optimized Build` FAIL (the two named handoffs) · `Performance Benchmarks` / `Benchmark Regression Alerts` / `Comprehensive CI Pipeline` CANCELLED (the board-state §5.2 item-12 escalation trigger, third consecutive data point) · `Rust Toolchain Management` did not trigger on this push (board-state §5.1 baseline gap stands) · all other observed workflows success (Mutation Testing, Integration Tests, Sanitizers, Benchmark Dashboard, Dependency Management, Documentation, SDK/C-Harness, CodeQL, Net Tests, OpenSSF, Pack Guard, Doc-Truth Lint). **Matches the three-exception baseline; no new regression.**
- **Assets**: `cargo xtask fetch-assets` → `"= hdri-catalog: up to date … = kenney-audio: up to date … = kaykit-complete: up to date … fetch-assets summary: 3 up-to-date, 0 installed, 0 failed (of 3)"` — no-op confirmed.
- **Board change observed mid-session (director action, recorded not initiated)**: the `w0-pre-deprecation` tag is now **deleted from the remote** — `git ls-remote --tags origin` returns only `refs/tags/assets-v1` (verified 19:20 EDT), and a 956 MiB safety bundle exists at `d:\w0-pre-deprecation.bundle` (mtime 2026-07-20 17:50). This is board-state §3 **option (a)** executed. Consequences per that note: D8's "fresh clone is pointer-free" claim becomes true again (D8 unblocked), and the docs citing the tag as a *remote* recovery path (`docs/architecture/fluids.md`, `water.md`, W-series campaign docs) now need the option-(a) wording update — that doc sweep is NOT part of this read-only session; it belongs to the director's D8 path or a hygiene beat.

---

## 2. Phase 2 — the load-bearing finding, re-derived at HEAD

### 2.1 The June recon's finding, and its current truth

The committed recon (`docs/audits/e3_terrain_generation_wiring_recon_2026-06.md`, ratified 2026-07-01) found: the rich pipeline (WorldArchetype → climate → per-vertex Whittaker classification → erosion) runs, but the render boundary discards per-vertex `chunk.biome_ids()` (**zero non-test readers**) and collapses texturing to a single `primary_biome` splat.

**At HEAD that finding is RESOLVED.** `chunk.biome_ids()` now has three production readers, all in `tools/aw_editor/src/terrain_integration.rs`: `:378` (generation), `:660` (zone-stamp path), `:2313` (paint path). The full chain, cited at HEAD:

| Stage | Code (verified this session) |
|---|---|
| Archetype selection | dropdown `panels/terrain_panel.rs:995-1019` (auto-`regenerate_terrain()` on change, `:1018`) → `set_world_archetype` `terrain_integration.rs:168-170` → `config.climate.archetype`. `WorldArchetypeId` = **6 variants** (`world_archetypes.rs:44-63`): ContinentalTemperate, EquatorialTropical, BorealSubarctic, Mediterranean, Desert, Custom |
| Climate fields | `ClimateMap::sample` `climate.rs:393` (archetype getter `:369`) |
| Per-archetype landform (build Phase A.2) | None-mask path reads the **selected** archetype's splines: `lib.rs:568-577` (`self.climate.archetype().bootstrap_splines`, replacing the pre-E3 hardcoded Continental Temperate) |
| Seam fix (build "integration-seam fix") | biome classification runs on a **params-based provisional height** — same spline regime as the render height: `lib.rs:587-651` (`sample_height_with_params(&bootstrap_params, wx, wz, 1.0)` at `:649-651`). Pre-fix, snow/alpine painted onto 20-30 m flats (commit message: 48.8% → 3.9% mountain-character biomes, snow median 29 m → 526 m) |
| Per-vertex classification | `blend_biome_parameters` → `blended.dominant_biome` `lib.rs:657-677`; coastal gate `lookup_biome_coastal_gated` `biome_lookup.rs:315`, production call `biome_param_blending.rs:204` |
| Biome taxonomy | **`BiomeId` = exactly 19 variants** (`biome_lookup.rs:55-108`: 11 terrestrial + 5 aquatic + 3 elevation-overlay; `BiomeId::all()` `:113-135`). The remembered "~19" is confirmed |
| Chunk storage | halo biome_ids cropped `lib.rs:388`, stored on `TerrainChunk` `lib.rs:452` |
| Render boundary (E3-terrain.1) | `generate_terrain` threads `chunk.biome_ids()` (`terrain_integration.rs:375-388`) → `generate_heightmap_mesh(…, pre_erosion_biome_ids)` (`:747`, `:766`) → `build_biome_slot_field` (`:1293-1370`: 19→8 `biome_id_to_slot` + slope-rock overlay (consts `:1304-1306`) + 1-ring blur `:1339-1369`) → per-vertex `material_ids`/`material_weights` (`:837-840`) → `TerrainVertex` (`:861-867`) |
| Legacy path | the old single-`primary_biome` splat generator survives **only as fallback** when `pre_erosion_biome_ids` is `None` (`:797-802`, `:841-852`) — backward compat, not the live editor path |

**Where does per-vertex biome information stop propagating today? It doesn't** — it reaches the GPU as per-vertex `material_ids[4]`/`material_weights[4]`. The single-biome collapse no longer exists on the live path.

### 2.2 Spline differentiation at HEAD (the F.7 substance)

`spline_types.rs:661-758`: Continental Temperate = `climate_driven_spline_set()` (`:661-663`); Equatorial Tropical (`:669-685`, mountain scale 0.0016, broad massifs); Boreal Subarctic (`:690-706`, 0.0026, amplitudes to 880 — sharp alpine); Mediterranean (`:710-726`, 0.0020); Desert (`:735-753`, 0.0024, flat-biased curve + sharp tail + high floor — dune plateau with rare ridges); Custom = CT baseline by design (`:758-760`). Distinct control points per factory verified by direct read. (Commit message's measured distinctness: "Boreal p90=218m/7% mtn vs others 58-83m" — commit-message claim, not re-measured this session.)

### 2.3 Observational confirmation (editor run, 2026-07-20 19:12-19:47 EDT)

Editor built (`cargo build --bin aw_editor --profile release-fast`, 20m35s) and launched from HEAD; terrain generated live at seed 12345, chunk radius 6 (169 chunks), across three archetypes via the Terrain panel (opened through Window → Panels; the archetype dropdown auto-regenerates on change, `terrain_panel.rs:1018`). Screenshots in `d:/tmp/e3pf_staging/` (session-local): `10_gen_check.png` (CT), `12_boreal_generating.png` (Boreal), `16_desert.png` (Desert).

| Archetype | Observed |
|---|---|
| Continental Temperate | **Multi-material world confirmed**: dominant tan ground (forest slot — leaf-litter albedo), bright-green grassland patches, gray-brown rock on steep flanks, white snow caps on tall isolated massifs. Rolling plains + rare sharp ranges (the Phase-A "distinct occasional mountain ranges" landform). 3,178K triangles, 54-55 FPS, ~1.8 GB process memory |
| Boreal Subarctic | **Complete palette + landform flip**: snow-white dominant lowlands (Tundra/SnowCap slot), dense jagged alpine ridge networks, brown rock bands on steep upper flanks with white caps on the tallest peaks (the absolute-Y elevation overlays + slope rock overlay, visibly banded). No green. Matches the build-era "~96% tundra/snow — honest subarctic" side-finding |
| Desert | **Flat pale dune plateau** with one cluster of sharp barren ridges (the flat-biased spline + sharp low-erosion tail), no water, no green, and **no snow on peaks** — rock instead (the temp-gated SnowCap behaving per `terrain_integration.rs:1264-1266`) |

**Verdict**: the renderer visibly consumes biome variety — multiple materials per world and archetype-driven differences in both palette and landform. The June recon's collapse is refuted by direct observation at HEAD. Regeneration at radius 6 completes in seconds-to-a-couple-minutes on this machine (parallelized), far below the E3-era radius-10 timings.

**Observed defect-of-legibility (feeds §6/§7, not a regression):** the Continental Temperate world reads *predominantly tan* at distance because its dominant biome family (forest, 5-to-1 collapsed) renders a brown leaf-litter floor with tree scatter still disabled — the build's own recorded follow-up ("world reads TAN … until tree scatter returns"). These are the first recorded screenshots on the post-AD.4/AD.5.A material substrate (forest slot re-pointed to `derived_1k/tree_leaves`), i.e. the first post-re-point visual evidence: the re-point preserved the litter-brown character. No gravel-water areas were distinguishable at the observed camera angles; the water-slot question (§6 row 7) stands on code+pack evidence.

### 2.4 Residue the build left in the tree (verified at HEAD)

- **TEMPORARY terrain-judging aids, still present** (revert-before-v1 debt): scatter suppressed (`main.rs:5477-5478`, `:7812` — "E3-terrain (temporary): scatter cleared / upload no scatter for shape judging"); fog 800/1800 → 60000/120000 (`viewport/engine_adapter.rs:1751-1754`); camera far 5000 → 40000 (`viewport/camera.rs:262-264`).
- **"Apply Erosion" button is still a no-op stub** (`panels/terrain_panel.rs:1980-2003` — "In a real implementation, this would call the erosion systems. For now, just track the timing"), and the entire hydraulic/thermal/wind erosion UI config section (`:1926-1956` presets) feeds no engine path — a pre-E3 Integration-Completeness violation (settable-but-never-read) carried into the tuning beat.
- **Test surface measured this session** (`cargo test -p astraweave-terrain --no-fail-fast`, full log `d:/tmp/e3pf_staging/terrain_full_tests.log`): **2,439 passed / 63 failed / 56 targets, 11 targets failing**. Lib target: `797 passed; 7 failed; 3 ignored` (3 `spline_types` D5FIX-baseline + 1 `noise_gen` highland-continental + 3 `mutation_tests` golden/fbm — the build commit's "4 lib" was an undercount). Every failing test inspected or classified by name is a **pre-E3 baseline/golden assertion**: D5FIX spline baselines (`bootstrap_spline_set_default_is_d5fix_baseline`, CT-median-climate matches), old noise-config defaults (`noise_config_base_elevation_type_is_perlin`, `…mountains_type_is_ridged`, amplitude literals 50/80/8 — all deliberately changed by the build, §2.4 bullet 4), `golden_sample_height_*`/`golden_density_*` fingerprints, and the halo byte-identity contract (`halo_cropped_heightmap_matches_single_chunk_generation` — the open E3.b disposition). Two suspicious-sounding names adjudicated from panic output: `sample_height_has_nonzero_range` fails on a **stale amplitude bound** ("Max height should be bounded by amplitudes, got 477" — inside the new 480 regime), and `config_returns_actual_config_all_fields` pins base amplitude 50.0 (now 100). **Zero failures indicate real defects**; the count grew from the June recon's ≥45 because the build's spline/noise changes added rot on top (E3.a-1 had fixed 13). E3.a-2 golden re-bake stays held; full re-bake is T.G.
- Old ".3 backlog" partially executed/superseded in-tree: base noise Perlin→**Fbm** (`noise_gen.rs:216-227`), mountains RidgedMulti→**Billow** (`:229-242`), detail amplitude 8→3 (`:247`); `continental_enabled` **deliberately false** — the erosion-driven `mountains_amplitude` spline replaced continentalness gating (`:120-139`, would double-gate); `continental_min` 0.0 (`:191-197`); domain-warp/derivative-weighted still off by default (`:202-204`) — texture repetition was instead solved by hex-tiling, landform repetition by splines.

---

## 3. Phase 1 — record vs. memory gap table

| # | Remembered claim (UNVERIFIED input) | Record status | Evidence |
|---|---|---|---|
| 1 | E3 recon found the world-gen pipeline runs but the render boundary discards per-vertex biome data and collapses to one biome | **CONFIRMED** (as of its date) | `e3_terrain_generation_wiring_recon_2026-06.md` §3, §4 Gap 2 (committed `187b8a8d6`, ancestor of HEAD) |
| 2 | A beat "E3-terrain.1" (multi-biome consumption wiring) was **drafted** | **CONTRADICTED — it was BUILT.** Scope was ratified 2026-07-01 (recon §6/§10), then the build landed 2026-07-03 as `d506658d8` with E3-terrain.1 explicitly named in the commit body and in code comments (`terrain_integration.rs:1236`, `:1276`) | `git show d506658d8`; §2 above |
| 3 | Hard stop on a director decision mapping ~19 biomes onto the 8-slot pack | **CONTRADICTED in outcome, PARTIALLY CONFIRMED in provenance.** The mapping shipped in code (`terrain_integration.rs:1250-1274`) with an inline note "Legibility calls (ratified 2026-07-01)" — chat-level ratification of the *collapse decisions*; **no mapping ratification table was ever committed** (grep of `docs/` + git log: none). The dated ratification is the recon's *scope* ratification, not a row-by-row sign-off | Phase-1 agent §5; recon §7 |
| 4 | Subsequent beats sketched: .3 (tuning) then .2 (per-archetype spline shaping "F.7") | **CONFIRMED as ratified scope (order .1→.3→.2, .2 in scope for v1)** — but **.2 subsequently LANDED** in the build (Phase A.2) and .3 partially landed (§2.4) | recon §6 + §10 (ratified block); `d506658d8` |
| 5 | Campaign closes with golden-image re-bake "E3.a-2" | **CONFIRMED — still held.** "the held E3.a-2 golden re-bake stays held until the terrain is right" (recon `:116`); reaffirmed in the research doc (`terrain_generation_techniques_research_2026-07.md:69`) and the build commit ("E3.a-2 golden re-bake stays held until the terrain build stabilizes") | cited docs |
| 6 | ~19 classified biomes | **CONFIRMED**: exactly 19 (`biome_lookup.rs:55-108`) | §2 |
| 7 | 8-slot render material pack | **CONFIRMED**: 8 layers (`assets/materials/biomes/arrays.toml`) | §4.1 |
| — | (Not remembered) E3 test-surface beats | **RECORD-ONLY FINDING**: a second recon (`e3_terrain_test_surface_recon_2026-06.md`, `8405e9263`) found the roadmap's "8 failing tests" was an undercount — **≥45 test-rot across ≥13 targets, 0 real defects**; beats E3.a (test-rot batch) / E3.b (halo contract disposition) / E3.c (**dissolved** — the "scatter Z-insensitivity real-defect" was falsified, `39b85ba8e`) / E3.d (optional CI hygiene). `E3.a-1` (13-test amplitude-independent batch) landed as `500484126` (ancestor of HEAD) though no doc names it. E3.b disposition and the amplitude-finality gate (ROADMAP_R1 §6.4 `:213-215`) remain open | Phase-1 agent §1-2 |
| — | (Not remembered) roadmap altitude | The R.1 roadmap (`ROADMAP_R1_PATH_TO_V1.md:58,75,143-149,230`) defines E3 only as the M2 criterion "`terrain` VP + `materials` VP"; beat names live in the two recons. `MASTER_ROADMAP.md` does not mention the E3 build at all | Phase-1 agent §1 |

**Authority note:** rows 2-4's downstream statements rest on the commit record (`d506658d8` and its ancestors) + first-hand HEAD reads; the lost-chat narrative contributes nothing load-bearing that the record contradicts. Where the record is silent (why no outcome doc was written; whether the mapping's "director-approved" wording reflects an explicit chat sign-off), this doc flags rather than infers.

---

## 4. Phase 3 — substrate-drift audit (what AD changed under E3's feet)

Post-build commits touching E3-adjacent surfaces (complete list): `06780433d` (AD.4 re-point, 2026-07-07, `materials.toml` 3 lines), `21bc53333` (AD.5.A forest re-point, 2026-07-15, `materials.toml` 10 lines), `ae9b98ef3` (AD.5.A palette remap, editor viewport), `bc284bb65` (rustfmt churn), plus editor-chrome commits (grid depth, spawn visibility, theme). **No terrain generation or render logic changed.**

### 4.1 The 8-slot biomes pack at HEAD (`assets/materials/biomes/materials.toml` + `arrays.toml`; all 24 files verified on disk)

| Slot | Key | Albedo (resolves to) | Tiling | Source dir |
|---|---|---|---|---|
| 0 | grassland | `assets/materials/grass.png` | 128 | materials/ |
| 1 | desert | `assets/materials/sand.png` | 128 | materials/ |
| 2 | forest | `assets/materials/derived_1k/tree_leaves.png` | 128 | **derived_1k/** (AD.5.A Fix-1 re-point off gitignored `_downloaded/polyhaven/`) |
| 3 | mountain | `assets/materials/mountain_rock.png` | 64 | materials/ |
| 4 | tundra | `assets/materials/snow.png` | 128 | materials/ |
| 5 | swamp | `assets/materials/mud.png` | 128 | materials/ |
| 6 | beach | `assets/materials/sand.png` (**same file as desert**) | 128 | materials/ |
| 7 | river | `assets/materials/derived_1k/gravel.png` | 128 | **derived_1k/** |

All slots use the `mra` key → R↔B-swizzled to ORM at load (`canonical_terrain_pack.rs:180-184`, `load_mra_as_orm_bytes` `:221-227`); the `orm` key (verbatim load) exists but is unused by the live pack. Slot order is `arrays.toml`-driven and **matches `biome_id_to_slot` 1:1**.

### 4.2 Palette-remap interaction — COEXISTS, does not intersect

`PaletteRemap` (`viewport/palette_remap.rs`) maps the 21-entry MaterialLibrary palette onto pack layers by **albedo file stem** (case-insensitive; duplicate stems → lowest index, so shared `sand` resolves to slot 1). Re-resolved on every layer upload (`engine_adapter.rs:1446-1453`) beside `set_terrain_materials` (`:1455-1477`).

**Biome-driven layer assignment BYPASSES the remap**: `build_biome_slot_field` writes pack-layer indices 0-7 directly (`w[biome_id_to_slot(b)] = 1.0`), flowing to the GPU untouched. The remap governs only **manual palette painting** (`terrain_panel.rs:872-904`: paint writes the *remapped* layer index; unresolvable selection paints nothing). The two paths coexist cleanly; the wiring beat the June plan anticipated would have had to route *through* this surface — the build predated the remap, and the remap was designed around the build's pack loader. **No conflict found at HEAD.**

### 4.3 S2 render-quality items — confirmed at HEAD

`NORMAL_XY_STRENGTH = 1.8` (`astraweave-render/shaders/pbr_terrain_forward.wgsl:331`, applied `:332-333`); hex-tile pow-4 weight sharpening (`:264-266`) + per-cell rotation from `hex_cells()` (`stochastic_tiling.wgsl:183-225`; UVs+gradients rotated `:271-279`, normals counter-rotated `:313-321`). Flat roughness/AO: AD4A records "5-6 of 9 traced-9 materials flat/degenerate" (`AD4A_RENDER_GATE_DIAGNOSIS.md:39,101`) with named cases plaster (AO recovered by the D2 fix, `34973de63`), tree_bark (roughness recovered; **AO source-genuinely flat**), and ice (**flat AO, R=255**, + ARM-order inversion, `AD5A_CONSUMER_HYGIENE_OUTCOME.md:137`). Folds into the tuning beat per board-state §1.3.

### 4.4 The terrain `expect()` handoff — close-out §4 item 1 needs TWO corrections

1. **Locations**: production-path `expect()`s exist **only in `spline_types.rs`** — six: `:571`, `:590`, `:605` (in `climate_driven_spline_set`) + `:637`, `:647`, `:652` (in `archetype_spline_set`), all unwrapping `Spline1D::from_control_points` on hardcoded control points. `regional_archetype_mask.rs` and `noise_gen.rs` `expect()`s are **all inside `#[cfg(test)]`** (boundaries at `:742` and `:1138` respectively) — the ledger's naming of those two files as production-`expect()` carriers is wrong.
2. **Attribution**: the Clippy Unwrap-Prevention workflow (`.github/workflows/clippy-unwrap-prevention.yml`) **does not scan `astraweave-terrain` at all** (not in the P0 matrix `:22-33` nor the warn-only P1 matrix `:109-113`). The workflow's standing red is therefore **not caused by the terrain crate** — its actual cause belongs to the workflow-hygiene stream, and fixing the six spline `expect()`s will not flip it green. Hygiene placement proposal: §6 (T.2).

### 4.5 Showcase removal — zero terrain-path consumers died

`unified_showcase` was fully self-contained ("no dependency on engine render crates", its own procedural terrain; parent-commit `Cargo.toml` lists neither `astraweave-terrain` nor `astraweave-render`). Only two inert references remain workspace-wide (a comment in `assets/textures/atlas_config.toml:2,5` and a stale doc-comment in `astraweave-render/tests/test_pbr_visual_validation.rs:315`). Current `astraweave-terrain` consumers: `astraweave-render`, `tools/aw_editor`, and 9 example crates — all intact.

### 4.6 Trace-sync debt — UNPAID (and one active contradiction)

The build touched no `docs/architecture/` file; the debt the build session recorded as owed is still open:

| Trace | Version / last-verified | E3 build reflected? |
|---|---|---|
| `terrain.md` | v1.1 / 2026-06-25 | **NO — actively contradicts HEAD**: `:141` "The `None`-mask branch evaluates Continental Temperate splines only" is precisely what Phase A.2 reversed |
| `render_pipeline_material_system_shader_infrastructure.md` | v1.9 / 2026-06-25 | NO (no stochastic-tiling wiring, mip chains, anisotropy) |
| `aw_editor.md` | v1.4 (bumped 2026-07-15 for AD work only) | NO (documents `canonical_terrain_pack.rs` solely as the palette-remap's loader; no `biome_id_to_slot`, no multi-biome render) |
| `terrain_materials.md` | v1.1 / 2026-05-10 | NO (entirely pre-E3) |

Also: **no E3 build outcome doc exists** — no file in `docs/` references `d506658`. The commit message is the sole record. (This pre-flight doc now carries the reconstruction, but a proper trace-sync remains owed — see T.0.)

---

## 5. Phase 4 — Editor Multi-Tool SP5 gate disposition (recommendation only)

### 5.1 The recorded gate

`docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` §7 (line 731). Success criteria §7.3 (`:761-770`) and the required Andrew-gate §7.4 (`:772-779`), whose exact wording matches the remembered criterion: *"Click+drag routes to RegionalArchetypePanel's brush queue (not consumed by camera pan, which was the F.5-paint.F-fix Andrew-gate REGRESS)"* (`:777`) / *"Brush UX click+drag does NOT route to camera pan (the original F.5-paint REGRESS is fixed)"* (`:779`).

### 5.2 The record vs. the code

- **SP5.A (`2f6f853a5`) and SP5.B (`9bae816e3`) landed 2026-06-06** and are ancestors of HEAD: `impl ActiveTool for RegionalArchetypePanel` (`regional_archetype_panel.rs:548-629`, paint handlers return `EventDisposition::Consumed` at `:593/:611/:627` with the explicit pan-prevention rationale `:540-541`, `:592`); dispatcher registration at editor init (`main.rs:2780-2784`); side-channel paint queue → per-frame drain → canonical panel + undo (`command.rs:1748+`, `tab_viewer/mod.rs:1501`, `main.rs:4156`).
- **The campaign doc is stale**: §11 `:977` still records Sub-phase 5 "NOT STARTED"; no 5.C closeout commit exists.
- **No recorded verification of the gate criterion**: no automated test instantiates `RegionalArchetypePanel` and asserts its paint disposition (coverage is `MockActiveTool`-generic only, `active_tool/tests.rs:262,758-759`); `viewport/widget.rs` has zero tests.
- **AD.5.A's adversarial paint-dispatch verification does NOT cover this gate** — it verified *TerrainPanel* palette-remap layer-index correctness (9 checks GO, `AD5A_CONSUMER_HYGIENE_OUTCOME.md:125`), a different panel and a different axis (data correctness downstream of dispatch, not pointer routing), and it explicitly treats the dispatcher's panel instance as inert. The shared "5.A" numeral is coincidence (asset-series AD.5.A vs Multi-Tool SP5.A) — flagged to prevent conflation.

### 5.3 Recommendation

**CONDITIONAL PASS — the only missing element is the director's formal Andrew-gate declaration.** The implementation is in HEAD's ancestry, the disposition design directly encodes the gate requirement, and the director reports paint works end-to-end today. §7.4 makes the gate a *visible-output director check* by construction — no committed evidence can substitute for it. Proposed disposition: the director performs the §7.4 check (open editor → RegionalArchetypePanel → archetype + Paint mode → click+drag → paint ops accumulate, no camera pan) and declares PASS/REGRESS; on PASS, **5.C closeout** (doc §11/§12 update + RAV cross-reference + G-pointer-events-fix subsumption confirmation, §7.7 `:795`) becomes a small doc-only beat. Optionally fold into 5.C: a dedicated automated regression test asserting the real panel's `Consumed` disposition (currently absent).

### 5.4 Multi-Tool queue inventory + RAV

- **5.C closeout** — blocked on the Andrew-gate PASS (above).
- **Mediator-Removal** — §8 (`:799-865`): dedicated session (explicitly not a sub-phase), commits .A-.D, own Andrew-gate.
- **SP6 closeout** — §9 (`:869-912`): doc-only, marks the campaign COMPLETE.
- **RAV** = the Regional Archetype Variation campaign (`docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md`), **PAUSED 2026-05-03** pending Multi-Tool; resumes at G-pointer-events-fix — which SP5 "likely subsumes" (§7.5 `:781-785`); 5.C confirms.
- **Cross-campaign fact for the terrain plan**: the panel's painted mask still never reaches `WorldGenerator.regional_archetype_mask` at HEAD (`set_mask` callers are test-only `regional_archetype_panel.rs:1245/:1356`; `regenerate_terrain` builds a fresh `TerrainState::new()` `terrain_panel.rs:2095/:2128`; `lib.rs:196` default `None`). SP5's gate does not require it (routing, not generator wiring), and the June recon scoped spatial multi-archetype OUT of E3-terrain.2 "unless separately ratified" — it stays a separately-ratifiable future feature (§7, "Explicitly NOT in this campaign").

---

## 6. Phase 5 — the biome→slot mapping ratification package

**Standing of this table**: the mapping below is **what ships at HEAD** (`terrain_integration.rs:1250-1274`, exhaustive match — a new `BiomeId` variant is a compile error). It was implemented during the build under chat-level "legibility calls"; no ratification table was ever committed (§3 row 3). This table converts it into the formal decision surface. Design bar (director-ratified): **Minecraft-grade biome legibility on realistic terrain** — adjacent-biome distinctiveness beats taxonomic fidelity.

| Slot (material) | BiomeIds collapsed in | Climate niche(s) (from `biome_lookup.rs:55-107` doc comments) | Rationale as shipped | Flags for the director |
|---|---|---|---|---|
| 0 Grassland (`grass.png`) | Savanna · TemperateGrassland | hot dry-to-moderate grassland w/ scattered trees · cool prairie/steppe | both read "open grass" at a glance | Savanna loses its tan/dry-grass African read (renders temperate-green). Acceptable at 8 slots; a Savanna accent would need a slot-content change |
| 1 Desert (`sand.png`) | SubtropicalDesert · ColdDesert | hot arid · cold arid (Gobi/Patagonia) | "arid = sand" legibility | ColdDesert reads as hot sand. Taxonomic loss, low stakes (ColdDesert is rare outside Boreal/Custom worlds) |
| 2 Forest (`derived_1k/tree_leaves.png`) | TropicalRainforest · TropicalSeasonalForest · TemperateRainforest · TemperateDeciduousForest · BorealForest | the full forest family, hot-wet through cold-conifer | **the biggest collapse (5→1)**, deliberately deferred to "real per-biome textures / 32-layer path" per the in-code note (`:1242-1244`) | **Worth keeping distinct eventually**: tropical vs boreal forest is a Minecraft-legibility distinction (jungle vs taiga). With scatter (trees) still disabled, the floor texture is the *only* forest signal today. Recommend: accept for now; revisit at the 32-layer migration (T.2/T.3 option) |
| 3 Mountain (`mountain_rock.png`) | MountainRocky · Scree · Alpine | bare rock face · loose rock slopes · subalpine sparse vegetation | all read "gray rock"; also the slope-overlay target (`:1304-1306`) | Honest. No action |
| 4 Tundra (`snow.png`) | Tundra · SnowCap | very cold dwarf-shrub permafrost · permanent snow/ice | "cold = white" legibility; SnowCap is temp-gated so hot peaks stay rock (`:1264-1266`) | Tundra-as-pure-snow loses the "cold steppe" read (Tundra is not snow-covered year-round). Acceptable under the legibility bar; a dedicated tundra ground would need a slot-content change |
| 5 Swamp (`mud.png`) | Wetland | low-elevation very-high-moisture marsh/bog | solo mapping, honest | No action |
| 6 Beach (`sand.png` — **same file as slot 1**) | Beach · Coast | sand/gravel shoreline · shallow water transition | shoreline = sand | **Beach and Desert are byte-identical up close** (shared albedo/normal/mra). Adjacent-distinctiveness loss where deserts meet coasts. Candidate: distinct beach material (wet-sand/shingle) in the tuning beat |
| 7 River (`derived_1k/gravel.png`) | River · Ocean | linear water bodies (River currently unproducible — needs hydrology, `:89-95`) · open ocean | "only blue slot" per the in-code note — written against the flat-color placeholder era (`terrain_biome_placeholder.rs:35`, River = muted blue `[80,120,165]`); the canonical texture pack then assigned **gravel** to this slot | **No honest slot exists for water.** Ocean/river areas render as gravel ground; the terrain path draws no water surface. Post-seam-fix Aquatic coverage rose to ~21% on Continental (commit-era measurement), so this is visually prominent. Flagged as a slot-content decision (water-colored material now?) and/or a Water-campaign interlock (the `water.md` surface exists engine-side). **Do not design here — director call** |

**Many-to-one collapses that lose a distinction worth keeping** (per the design bar): (a) the 5-forest collapse — jungle/taiga/deciduous are Minecraft-distinct; (b) Beach≡Desert shared file; (c) Ocean-as-gravel. (a) is scoped to the 32-layer future; (b)/(c) are slot-content changes ratifiable into the tuning beat.

---

## 7. Phase 6 — proposed campaign plan (director ratifies before any beat is drafted)

The ratified June order (.1 → .3 → .2) is overtaken: .1 and .2 shipped in `d506658d8`. Proposed resumption ("T-series"), each beat rung-laddered (automated floor → load/scale check → director render gate):

| Beat | Scope | Folded items | Verification design |
|---|---|---|---|
| **T.0 — Record reconciliation (doc-only, first)** | Pay the build session's trace-sync debt: `terrain.md` (fix the `:141` contradiction + Phase A/A.2/seam-fix/E3-terrain.1 sections), render trace (stochastic-tiling wiring, mip chains, anisotropy), `aw_editor.md` (`biome_id_to_slot`, biome-slot field, judging aids), `terrain_materials.md` (canonical pack, mra/orm, remap coexistence). Correct close-out §4 item 1 (per §4.4: six `expect()`s in `spline_types.rs` only; Clippy-red attribution false) by append-note. This pre-flight doc serves as the build-outcome record | ledger item-1 wording fix | `aw_trace_sync --check` green; Doc-Truth Lint green; director reads |
| **T.1 — Mapping + slot-content ratification execution** | Apply whatever §6 rows the director amends (e.g. water-slot material, distinct beach). If zero amendments: T.1 collapses into a ratification note inside T.0 | Phase-5 outcomes | `cargo test -p aw_editor --lib` (loader + remap suites); director render gate on affected slots |
| **T.2 — Tuning (the fused E3-terrain.3 + S2 beat)** | (i) S2 "eyes hurt up close": aux-map resolution decision (512²→1024² costs ~+400 MB GPU — decision), parallax/micro-relief, `NORMAL_XY_STRENGTH` retirement-or-keep, flat rough/AO materials (ice AO, tree_bark AO; the un-enumerated "5-6 of 9" remainder needs a measurement pass); (ii) erosion visibility: preset tuning, wire-or-delete the no-op "Apply Erosion" button + unread hydraulic/thermal/wind UI section (`terrain_panel.rs:1926-2003`); (iii) residual look items from the build's known-follow-ups: Boreal ~96% tundra balance, Desert oasis, view-distance wall; (iv) `expect()` hygiene sub-beat rides HERE (same-file work: six `Spline1D::from_control_points().expect()` → const-validated or error-propagating construction) | board-state §1.3 (S2 folds into tuning); close-out §4 items 1 (hygiene) + 3 (S2) | per-change `cargo check -p astraweave-terrain` + lib tests; a repeatable measurement diagnostic (biome-distribution counts per archetype); director render gate per §7.4-style visible-output check |
| **T.3 — Judging-aids revert + scatter re-enable** | Revert the four "E3-terrain (temporary)" sites (scatter `main.rs:5477/:7812`, fog `engine_adapter.rs:1751-1754`, camera far `camera.rs:262-264`); re-enable scatter and re-judge forest read (tree cover changes the forest-slot calculus of §6 row 2) | build known-follow-ups | editor smoke + director render gate (scatter density/LOD at the new landforms) |
| **T.G — Test-surface close + golden re-bake (E3.a-2)** | Amplitude-finality gate FIRST (ROADMAP_R1 §6.4 `:213-215` — director confirms current landform amplitudes are final); then re-bake the held goldens + the baseline assertions. **Surface measured this session** (§2.4): **2,439 passed / 63 failed across 56 targets (11 targets failing), all failures in the pre-E3 baseline/golden family, zero real defects** — the work is re-baking ~63 assertions across ~11 targets, not bug-fixing. E3.b halo-contract disposition rides here (its contract test is one of the 63) | E3.a / E3.a-2 / E3.b; the recon's CI-hygiene E3.d option | full suite green (`N passed` lines cited per target); Pack Guard + Editor CI green; campaign close-out doc |

**Sequencing rationale**: T.0 before anything (the record must match HEAD before new work lands on it); T.1/T.2 order flexible but mapping amendments (T.1) change what T.2's render gate judges; T.3 after T.2 (scatter re-enable re-judges the tuned look); T.G last (goldens bake only on final terrain — the recon's "golden coupling" rule).

**Explicitly NOT in this campaign** (separately ratifiable): spatial multi-archetype (RegionalArchetypePanel mask → `WorldGenerator.regional_archetype_mask` wiring — the §7.7 gap of §5.4); the 32-layer material migration (recon's scoping fork — the 8-layer path was the ratified .1 choice); hydrology/rivers (River biome unproducible pending a Water-campaign interlock); Multi-Tool 5.C/Mediator-Removal/SP6 (that campaign's own queue, §5.4).

### Open questions the director must answer at the gate

1. **Ratify §6 as-shipped, or amend rows** — specifically: water slot (row 7), beach≡desert (row 6), tundra-as-snow (row 4)?
2. **SP5 Andrew-gate**: perform §7.4 and declare PASS/REGRESS (unblocks 5.C; recommendation §5.3 is CONDITIONAL PASS).
3. **Amplitude finality**: are the shipped per-archetype landforms (Phase A.2 splines) final enough to unhold E3.a-2 after T.2, or is T.2 expected to move them again?
4. **Aux-resolution vs memory**: approve 512²→1024² aux arrays (~+400 MB GPU) for T.2's micro-relief, or keep the `NORMAL_XY_STRENGTH` compensation?
5. **Boreal balance**: ~96% tundra/snow at radius-2 was measured build-era — honest subarctic, or tune the temperature band in T.2?
6. **Desert oasis**: build the moisture-spike oasis (climate feature) in T.2, or defer?
7. **Trace-sync depth for T.0**: full four-trace update (proposed) vs terrain.md-only minimum?
8. **The three cancelled benchmark workflows** (item-12 escalation, §1): fold the workflow-budget beat into this campaign's CI stream or keep it separate? (Terrain benches ride those workflows; T.G's verification would benefit from them being green.)

---

## 8. Deliverables index

1. Gap table — §3. 2. Re-derived finding + observation — §2. 3. Drift audit — §4. 4. SP5 disposition + queue — §5. 5. Mapping ratification table — §6. 6. Campaign plan + open questions — §7. Screenshots: `d:/tmp/e3pf_staging/` (session-local, not committed).

*The director ratifies §5's disposition, §6's mapping, and §7's plan; execution beats are drafted only against the ratified versions.*
