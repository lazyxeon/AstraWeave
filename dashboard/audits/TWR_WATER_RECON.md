# T.W-R — Water Recon: what exists, what integration costs, what "paint water" means

> **Beat:** T.W-R (terrain series, beat 3 — read-only recon, design gate) · **Date:** 2026-07-21 · **HEAD:** `1f0b48b5e` · **Serves:** `T_SERIES_RATIFICATION_2026-07-20.md` §3 — *"a true and fundamental water slot — the engine's existing water system fully surfaced in the editor as a paintable and usable material."*
> **Session discipline:** zero code/asset/config changes; evidence tiers are marked **built** (code exists) / **run** (executed this session) / **verified** (output seen). Screenshots in `d:/tmp/twr_staging/` (session-local, not committed). The editor crash log tail is quoted verbatim in §1.6.

---

## 0. Executive summary (the five findings the gate turns on)

1. **The water system survived E3 untouched and healthy.** `git log 7c29b8182..HEAD` over `astraweave-render/src/water.rs`, `shaders/water.wgsl`, `astraweave-render/src/renderer.rs`, and `astraweave-water/` is **empty** — the entire surface (chunked-LOD Gerstner + refraction + depth-foam + weave deformation + split pass) and the truth facade are byte-identical to what the W-series verified and budget-measured on this machine (≈0.26 ms worst case vs the provisional 2.0 ms budget). `water.md` rev 1.1's file claims hold at HEAD verbatim.
2. **The editor water path is wired (W-FU-2) but (a) unreachable and (b) broken.** Unreachable: the enable gate keys on the **legacy** `primary_biome` string (`main.rs:4217-4218`, `matches!(biome, "swamp"|"beach"|"river")`) — a field the E3 archetype dropdown **replaced** in the panel UI (`terrain_panel.rs:987-994`), so E3 archetype worlds can never enable water; the only live route is the World Wizard's Tropical Coast / Swampland templates. Broken: driving that route this session **panicked the editor on the first water frame** — `set_water_enabled` builds the pipeline against `surface_format()` (Rgba8UnormSrgb) while the split water pass draws into the Rgba16Float HDR target (§1.6, observed + root-caused; the working consumers pass `Rgba16Float` explicitly).
3. **The placement problem largely solves itself — water level is derivable and already canonical.** E3's Whittaker classifier defines Ocean/Coast **by elevation against `SEA_LEVEL = 2.0`** (`biome_lookup.rs:248-263`), the same value as the renderer's `DEFAULT_WATER_LEVEL = 2.0` (`water.rs:47`) — documented as matching (`elevation_biome.rs:26-31`) but held as two unlinked constants. In coastal zones (continentalness < 0.40), *below sea level ⇔ aquatic biome* by construction, so the existing camera-following water plane at Y=2.0 + ordinary depth occlusion renders water **exactly over the aquatic-biome basins with no mask, no new pass, no placement data**. The one divergence class is inland (continentalness ≥ 0.40) dips below Y=2 (§2.3) — plausibly the Desert "hidden oasis" *by design intent* (`spline_types.rs:368-377`), director ruling requested.
4. **"Paintable" has no existing substrate to inherit.** The E3 per-vertex `BiomeId` field is generation-only (read accessor exists, `chunk.rs:162-164`; **no mutation path**); the one biome-paint brush in the codebase (`apply_brush_paint`, legacy 8-slot `biome_map`) has **zero callers**; and **no terrain state persists at HEAD** — scene save is entities+obstacles only (`scene_serialization.rs:34-41`), `LevelDoc` carries seed/biome fields that loading never feeds back into generation (`main.rs:8967-8987`). Every paint option's persistence cost is new plumbing; that gap is inherited, not created, by T.W.
5. **Recommendation (§3.5): two-phase.** T.W v1 = surface the existing system honestly: fix the format bug, replace the legacy gate with an E3-aware condition, drive the level from `SEA_LEVEL`, delete the W.2c.2 scaffolding weaves, add the missing level/enable UI. That alone waters the ~21% aquatic coverage of E3 worlds. Paint lands as phase 2 with **carve-brush semantics** for sea-connected water (paint = honestly lower terrain below sea level; classification and the plane follow deterministically) plus **volume entities** for perched water (the truth facade already supports bounded volumes via `add_water_aabb`; entities are the only option with a working persistence path today). The freeform mask/level paint (option b) is the most expensive and introduces a second water authority — deferred unless the director wants freeform perched lakes now.

---

## 1. Question 1 — what IS the water system at HEAD?

### 1.1 Drift check (the license to cite the W-series record)

`git log --oneline 7c29b8182..HEAD -- astraweave-render/src/water.rs astraweave-render/src/shaders/water.wgsl astraweave-water/ astraweave-render/src/renderer.rs` → **empty** (run, verified). Everything `docs/architecture/water.md` rev 1.1 (verified 2026-06-25) says about those files is current at `1f0b48b5e`. The E3 build (`d506658d8`) did not touch the water surface. The editor-side wiring (`tools/aw_editor`) **did** drift — line numbers below are re-verified at HEAD.

### 1.2 The render surface (built + verified-by-record; run this session via weaving_playground, §1.7)

`astraweave-render/src/water.rs` (991 lines) — `WaterRenderer`:

- **Geometry:** camera-following chunk grid — `CHUNK_SIZE=64`, `GRID_RADIUS=8` → 289 active chunks/frame, 4 pre-baked LOD tiles (subdiv 32/16/8/4, distance bands 110/220/360/∞), per-chunk skirts (`SKIRT_DEPTH=8.0`) hiding LOD cracks (`water.rs:31-47`). **There is no placement concept** — the surface follows the camera unboundedly; where water *appears* is decided solely by depth occlusion against opaque geometry.
- **Level:** real uniform-driven world-Y (`set_water_level` / `water_level()` / immediate `write_uniforms`, `water.rs:750-764`), `DEFAULT_WATER_LEVEL = 2.0` (`water.rs:47`).
- **Shading:** 4-wave Gerstner (Q≤1 steepness cap), Fresnel, depth-based color (deep/shallow/foam colors settable — `set_water_colors`, `water.rs:770-774`), rain ripples, W.2b screen-space refraction + depth-delta shoreline foam (scene-color snapshot + read-only depth, 4-entry bind group).
- **Weave-response (W.2c):** ≤8 `WeaveInstance`s (Part/Raise/Freeze), deformation bounded to ±`SKIRT_DEPTH` (`water.rs:49-136, 784-811`).
- **Pass structure:** `Renderer::run_water_pass` (`renderer.rs:4679`) — post-opaque split pass: snapshot HDR → `water_scene_color`, `prepare_scene`, draw **into the Rgba16Float HDR target** with `depth_write_enabled: false` vs read-only depth, then the optional F.4.3 accent overlay. Skipped entirely when dormant (`has_visible_chunks()` false, `renderer.rs:4695`). Called from both `render()` (`:5279`) and the editor's `draw_into` (`:5932`, caller-supplied depth). Public API: `set_water_renderer`/`clear_water_renderer` (`:4605/:4610`), `update_water` (`:4632`), `set_water_level` (`:4643`), `set_water_weave_instances` (`:4658`), `set_hdr_overlay` (`:4622`).

**Transparency/draw-order for the editor: already solved.** The split pass is the transparent-surface plumbing, and the editor's `draw_into` already routes through it. Q2's render-pass cost class is zero.

### 1.3 The truth facade (built + wired; unchanged)

`astraweave-water::{WaterQuery, AnalyticWater, WaterSample}` — optional infinite plane + N bounded AABB volumes; deterministic (enforced by test); presentation permanently excluded from `WorldSnapshot`/world_hash/replay/net (gate-Q1 carve-out). Sole production consumer: physics buoyancy (`astraweave-physics/src/lib.rs:931` owns the facade, `:1438 apply_buoyancy_forces` called per tick at `:1095`; bounded volumes via `add_water_aabb` `:1496`). **Note for §3:** the truth side already models *bounded water volumes*; only presentation lacks them.

### 1.4 Deprecated-sim residue + F.4 status (unchanged from the W-series record)

SPH/voxel solver removed in W.1 (~58.8K LoC; recovery LOCAL-ONLY per `fluids.md` §0.5). `astraweave-fluids` remains as the F.4 accent substrate (wired mechanism: `set_hdr_overlay`; feed example-only in `weaving_playground`) + a W.3+-deferred effects layer; its `editor.rs` forward-design module is compiled (`fluids/src/lib.rs:54`) but not consumed by `tools/aw_editor`. F.4.0–F.4.3 complete; producer relocation to a shared crate still logged-not-done.

### 1.5 Consumption census at HEAD

| Consumer | Call sites | Tier | Notes |
|---|---|---|---|
| Physics buoyancy (truth) | `physics/lib.rs:931, :1095, :1438-1496` | built + wired; exercised by physics test suites | The only gameplay-truth consumer |
| **Editor viewport** | build: `engine_adapter.rs:3829-3900`; level `:3907`; per-frame `update_water` `viewport/renderer.rs:695` → `engine_adapter.rs:3930`; widget `widget.rs:2834-2841`; gate `main.rs:4212-4225` | built + wired; **run this session → PANIC** (§1.6) | Gate legacy-keyed; level knob frozen at 0.0 (no UI; `terrain_panel.rs:457,:706`); W.2c.2 scaffolding weaves hardcoded (`engine_adapter.rs:3863-3895`) |
| `weaving_playground` | `main.rs:148-153` (Rgba16Float), `:526-531` | built; **run + verified this session** (§1.7) | The canonical full demo (weaves + accents) |
| `hello_companion` visual demo | `visual_demo.rs:720, :1234, :1383` | built (verified in W-series record) | |
| `veilweaver_demo` | `visual_renderer.rs:625, :832, :863` | built (verified in W-series record) | |
| Budget probes | `astraweave-render/examples/water_budget_probe.rs`, `weaving_playground/examples/accent_budget_probe.rs` | built + measured (W-series, this machine) | Numbers current at HEAD by §1.1 drift check |
| Dead editor UI state | `terrain_panel.rs:202-399` (`WaterBodyPreset`, `FluidSimParams` incl. SPH particle counts/caustics, `DetectedWaterBodyInfo`, `auto_detect_water_bodies`), fields `:460-462` | **dead** — zero readers, zero UI render sites (`fluid_params` greps to its declaration + `Default` init only) | Pre-W SPH-era residue; T.W cleanup candidate |

### 1.6 Observed: the editor water path panics at HEAD (run + verified)

Route driven this session (the only reachable editor-water route): **File → New World (Wizard) → Tropical Coast** (template → `BiomePreset::MediterraneanCoast` → `terrain_biome_key() = "beach"`, `procedural_filler_panel.rs:336`; wizard Terrain step shows "Water: Yes", seed set to 12345) → Generate World → `configure_and_generate(…, "beach", …)` sets `primary_biome` → gate `main.rs:4218` trips → `set_water_enabled(true)` → **process abort on the first frame with visible water chunks**:

```
thread 'main' panicked at …wgpu-25.0.2\src\backend\wgpu_core.rs:3402:26:
wgpu error: Validation Error
  In RenderPass::end / set_pipeline
    Render pipeline targets are incompatible with render pass
      Incompatible color attachments at indices [0]: the RenderPass uses textures with
      formats [Some(Rgba16Float)] but the RenderPipeline with 'water_pipeline' label
      uses attachments with formats [Some(Rgba8UnormSrgb)]
```

(`d:/tmp/twr_staging/editor_err.log`; wizard flow screenshots `20…24_*.png`.)

**Root cause (code-confirmed):** `engine_adapter.rs:3832-3837` constructs `WaterRenderer::new(device, self.renderer.surface_format(), Depth32Float)`. The W.2b split pass draws water into the **HDR** target (Rgba16Float), not the surface. The working consumer passes `Rgba16Float` explicitly with an in-code warning comment ("HDR water format … matching the runtime demos"; the adjacent F.4.3 accent setup even says "NOT the surface format" and uses the existing `renderer.hdr_format()` accessor — `weaving_playground/main.rs:144-163`). The editor wrapper predates the editor viewport's HDR-target migration and was never re-exercised because the gate became unreachable when the archetype dropdown replaced the legacy primary-biome dropdown — a silent break behind a dead gate, and a §7.7-flavor wrapper divergence (wrapper argument vs wrapped-pass reality). **W-FU-2's "woke the formerly-dormant editor water" is no longer true at HEAD; the editor water is dormant again AND lethal when woken.** Fix is one argument (+ regression coverage); it is a mandatory T.W item regardless of which design is ratified.

### 1.7 The render surface at HEAD: run-attempted; claim rests on zero-drift + the W-series verified record

`weaving_playground` (the canonical full water demo) was **built and launched at HEAD** this session (release-fast). The window was created and painted early frames (`26_B_weaving_playground.png` — a blue-cyan scene consistent with the demo's water, **not claimed as a verified water render**), but the main thread never reached a responsive event loop in this session's environment (~2.5 min, 4.1 s total CPU — blocked, not computing; both stdout/stderr logs empty; process killed). Cause undiagnosed — out of scope for a read-only recon, logged for T.W's ladder.

The "does the production water surface render at HEAD" question is nevertheless answered **yes, by construction**: the surface code at HEAD is byte-identical to `7c29b8182` (§1.1 drift check), where the W-series verified it live on this machine with pixel-lit readbacks (62.8% lit near view W.2a; refraction 12.8-25% and foam 7.9% near-white W.2b) and GPU-timestamp budget probes (§2.4 numbers). What is **new** at HEAD is that the one in-engine consumer (the editor) can no longer reach it without crashing (§1.6) — which is exactly the T.W work item.

### 1.8 Observed: E3 worlds render their aquatic terrain dry (run + verified)

Terrain panel → Mediterranean, seed 12345, radius 6 (169 chunks, 3.18M tris) — the T.1 director-repro world. World-scale frame `16_A_overview.png`: green lowlands, beach-band patches, snow-capped ridges, **no water surface anywhere in the world** (and no blue anywhere in frame). Ground-level texture reading of the same world's shore patches is on record from T.1 (`d:/tmp/t1_staging/10_mediterranean.png`). This is E3-PF §6 row 7 made visible: Ocean/Coast vertices (~21% of Continental worlds at the commit-era measurement) render as bare gravel/beach ground because nothing enables the water plane in E3 worlds.

---

## 2. Question 2 — what does editor-viewport integration cost?

### 2.1 The hook (cost: zero — it exists)

Editor frame → `ViewportRenderer::render` → `update_water` every frame with the canonical `RenderView` (`viewport/renderer.rs:690-697`) → engine `draw_into` → `run_water_pass(enc, depth_view)` (`renderer.rs:5907-5932`) — post-opaque, transparent, refraction-capable, dormant-skipped. No new pass, no new plumbing. The three actual defects are construction-time: the pipeline format bug (§1.6), the legacy gate (§1.5), the frozen level.

### 2.2 Placement data: the world already says where water is — by elevation

The load-bearing chain (all code-verified at HEAD):

1. `elevation_biome::SEA_LEVEL = 2.0` (`elevation_biome.rs:31`) — explicitly documented as matching the water plane's default Y (`:26-30`).
2. `lookup_biome` aquatic-first precedence (`biome_lookup.rs:248-263`): elevation < sea−3 → **Ocean**; < sea → **Coast**; < sea+3 (wet) → **Beach**. The E3 production classifier `lookup_biome_coastal_gated` (`:315-342`) applies this **only where continentalness < 0.40**.
3. `WaterRenderer` default level = **the same 2.0** (`water.rs:47`), settable per frame.

Therefore, in coastal zones, *aquatic biome ⇔ terrain below Y=2.0*. Enabling the existing global plane at `SEA_LEVEL` waters **all and only** the sub-sea-level terrain there — ocean basins fill, beaches rim them, everything above stays dry — with zero placement data threaded anywhere. Depth occlusion IS the mask. Rivers are the one aquatic class this cannot produce (they sit above sea level) — and the biome field cannot produce them either (`biome_lookup.rs:89-95`: River requires hydrology, deferred to a future campaign). The constraint does **not** bind T.W; it bounds it: T.W's water = sea-connected water (+ optionally authored perched bodies, §3), rivers = the hydrology campaign, riverbed-gravel stays honest per ratification §2 row 7.

### 2.3 The divergence class the director must rule on: inland sub-sea dips

The A.2b `base_elevation_floor` is an **additive offset, not a clamp** (`spline_types.rs:368-377`), so inland (continentalness ≥ 0.40) terrain *can* still dip below Y=2.0 — the C.1 gate exists precisely because such dips occurred ("speckled temperate plains with sand/gravel patches at every noise low", `biome_lookup.rs:301-312`). Under the plane model those dips **flood** while their ground stays terrestrial. Two honest readings:

- **Defect**: water where the biome field says land — fix by masking water chunks to aquatic-biome regions (the one genuinely new render feature in this design space: a chunk-set filter fed from terrain data) or by raising inland floors in T.2 tuning.
- **Feature**: the Desert floor's documented intent is that "only the deepest troughs become the 'hidden oasis'" (`spline_types.rs:372-375`) — a flooded deep inland trough with a terrestrial bed IS an oasis/pond, for free, coherent with "water is a place: low elevation". (Interacts with deferred Q6.)

Recon could not measure the real-world frequency of such dips read-only (it is archetype- and spline-dependent); a T.W ladder item should count sub-Y=2 inland vertices per archetype before ruling.

### 2.4 Cost classes for the candidate integration shapes

| Work item | Class | Size anchor |
|---|---|---|
| Format fix (`surface_format()` → HDR format; `hdr_format()` accessor exists) | render-API call-site fix + regression test | trivial (one argument); the whole E3-terrain.1 multi-biome wiring — a strictly larger job — landed inside one build day of `d506658d8` |
| Gate replacement (legacy string → E3-aware: enable when the generated world's biome census contains aquatic ids, or simply whenever terrain exists) | editor data-threading | small — the chunk biome census already exists editor-side (slot-field build consumes per-vertex ids, `terrain_integration.rs:1293+`) |
| Level from `SEA_LEVEL` + shared-constant tie (single source or an equality assert) | data-threading + invariant | trivial; retires the frozen-0.0 knob defect |
| Level/enable UI (slider + checkbox on the Terrain panel) | editor-UI | small; the `water_level` field and plumbing to the uniform already exist end-to-end |
| Delete W.2c.2 scaffolding weaves + dead `FluidSimParams`/`WaterBodyPreset` block | hygiene | small, pure deletion |
| Archetype-driven `WaterStyle` colors (editor currently hardcodes Ocean) | editor data-threading | small, optional polish |
| **Chunk masking** (water only over aquatic-biome regions) | new `WaterRenderer` API + terrain→water data path | moderate — only if §2.3 is ruled a defect |
| **Bounded volumes / per-body levels** (perched lakes) | new render capability (instance-level water level or multiple bodies) + authoring + persistence | the large item; belongs to §3 options (b)/(c), not v1 |

Perf anchor (unchanged code, this machine, min-spec 1660 Ti): surface ≈0.18-0.24 ms + scene-copy ≈0.085 ms + accents ≈0.01 ms ≈ **0.26 ms worst case vs the provisional 2.0 ms budget** (W-series probes; valid at HEAD by §1.1).

---

## 3. Question 3 — what does "paintable water" honestly mean?

### 3.1 What is actually mutable at edit time (all verified at HEAD)

| Surface | Mutable? | Path |
|---|---|---|
| Heightmap | YES — sculpt brushes (stroke + undo snapshots) | terrain panel brush modes |
| `material_ids`/`material_weights` (splat) | YES — the SP5 palette-remap paint | `apply_brush_paint_material` (`terrain_integration.rs:2346`) |
| Legacy 8-slot `biome_map` | mutation surface exists (`chunk.rs:188-191` "for paint brushes") but its one caller `apply_brush_paint` (`terrain_integration.rs:2257`) has **zero callers itself** | dormant |
| E3 per-vertex `BiomeId` field | **NO** — read-only accessor (`chunk.rs:162-164`), no `_mut`, generation-only | — |
| Water (level/regions) | NO editor-mutable state exists (the panel `water_level` field has no UI) | — |

### 3.2 What persists across save/load today: nothing terrain-side

`SceneData` = version/time/entities/obstacles (`scene_serialization.rs:34-41`). `LevelDoc` (`level_doc.rs:11-14`) records `biome`, `seed`, `biome_paints` — but `on_open` only assigns the struct; nothing regenerates terrain from it (`main.rs:8967-8987`). `editor_preferences.rs` carries no terrain fields. Sculpt edits, splat paints, and any future water paint are all session-transient at HEAD. **Whatever option is ratified, persistence is new plumbing — and the director should decide whether T.W owes it or whether it rides a later terrain-persistence beat (open question §4).**

### 3.3 The option table

| | (a) Biome/carve painting — "water is where the land is low" | (b) Water-mask / painted-level — "water is where I say" | (c) Water bodies as entities/volumes |
|---|---|---|---|
| **Mechanism** | A *water brush* = sculpt-carve terrain below `SEA_LEVEL` + deterministically re-classify affected vertices (climate sampling is a pure function of seed+position — `climate.rs:393`, determinism tests `:833,:956-964` — so `lookup_biome_coastal_gated` can re-run per painted vertex); the global plane then waters it automatically. Painting biome ids *without* carving (a2) changes ground material only — it cannot make water appear under the plane model | A separate painted authority: per-region mask or per-region water level, rendered as bounded/leveled surfaces | An editor-placed entity carrying a water AABB/volume; feeds presentation AND the truth facade (`add_water_aabb` **already exists**, `physics/lib.rs:1496`) |
| **Render cost** | none beyond §2.4 v1 (the plane does the work) | the large one: per-region levels/mask in `WaterRenderer` (instance-level Y or multiple bodies) + mask authoring | bounded-surface rendering per volume (shared machinery with (b), but volume-scoped, simpler) |
| **Editor cost** | brush mode (sculpt+reclassify compound); undo rides existing height snapshots + an id-array snapshot | mask paint UI + region management + authority-conflict UX | entity workflow already native (place/move/inspect) |
| **Coherence** | total — generation and paint share one rule (elevation vs sea level); regenerate reproduces the same classification law | introduces "world says desert, painter says lake" — needs a who-wins rule and material consequences (does the bed become riverbed-gravel? does moisture change?) | volumes are explicitly authored overrides; no ambiguity, but they are "the editor's water", furthest from the terrain system's water |
| **What it forecloses / can't do** | perched water (above-sea lakes, pools on hills); rivers | nothing visually, but builds a second water authority the biome field can contradict | emergent coastline behavior; each body is manual |
| **Save/load** | needs terrain-edit persistence (§3.2 gap — same as sculpt today: none) | needs new mask/region persistence | **rides existing entity serialization today** — the only option with a working persistence path at HEAD |
| **Regeneration** | painted carves lost on regenerate (like all sculpt edits today) unless persisted+replayed | mask survives regenerate but can contradict the new terrain | entities survive regenerate; may float/submerge if terrain changes under them |
| **Truth coupling** | plane truth already synced by physics each tick | mask has no truth representation (new facade concern) | truth exists today (`add_water_aabb`) |

### 3.4 Why (b) is the expensive one

(b) is the only option requiring the renderer to learn *bounded, per-region water levels* AND a new authored data layer AND an authority rule against the biome field — all three of the hard costs at once, for the benefit of freeform perched water that (c) delivers with existing truth + persistence machinery at a fraction of the design surface.

### 3.5 Recommendation (director ratifies or amends)

**T.W v1 — surface the existing system (no paint yet):** format fix + E3-aware gate + level from `SEA_LEVEL` (constant tied, one source of truth) + scaffolding-weave deletion + dead-block deletion + Terrain-panel enable/level UI + archetype-appropriate `WaterStyle`. Rung ladder: unit/regression on the format + gate, editor observation of Mediterranean/CT/Desert seed 12345 (coastlines fill; Desert stays dry-or-oasis per §2.3 ruling), director render gate. This alone converts row 7's "~21% of the world renders as dry gravel" into rendered ocean/coast and is honestly described as *the engine's existing water system fully surfaced in the editor*.

**T.W v2 — paint, in ratified order:** (a) carve-brush for sea-connected water (one rule, no second authority, undo mostly existing) and, if perched water is wanted, (c) volume entities (truth + persistence already exist). Defer (b) unless freeform painted lakes are an explicit design goal. Persistence scope per the §4 answer.

---

## 4. Open questions for the director (the gate)

1. **Sea-level model:** ratify Y=2.0 as THE world sea level with the two constants tied (`SEA_LEVEL` ↔ `DEFAULT_WATER_LEVEL`, single source or asserted-equal)? Any future per-world sea level would thread one value through both classifier and renderer.
2. **Inland sub-sea dips (§2.3):** oasis-feature (accept plane flooding, tune floors in T.2) or defect (chunk masking in T.W)? A dip-census ladder item is proposed before ruling.
3. **Paint semantics:** ratify §3.5's order — carve-brush (a) first, volumes (c) for perched water, (b) deferred?
4. **Persistence expectation:** is session-transient paint acceptable for T.W (matching sculpt/splat today), with terrain-edit persistence as its own later beat — or does T.W owe persistence for whatever it makes paintable?
5. **River scope:** confirm rivers stay with the future hydrology campaign (`biome_lookup.rs:89-95`); T.W water = sea-connected (+ authored volumes).
6. **Editor hygiene rider:** confirm T.W deletes the dead `FluidSimParams`/`WaterBodyPreset`/detection block and the dormant `apply_brush_paint`, and removes the W.2c.2 scaffolding weaves (replaced by nothing until a real editor weave feed exists).
7. **Style/colors:** editor hardcodes `WaterStyle::Ocean`; per-archetype (or per-biome-census) style selection in v1 or later?

## 5. Out-of-scope confirmations

- **No hydrology / river-generation design** here (separate future campaign unless pulled in — §4.5).
- **No F.4 particle/accent work** (the editor accent feed remains the logged W-series follow-on).
- **No implementation** was performed: zero code/asset/config changes this session; the one commit is this report. The editor crash was *observed*, not fixed.
- **No truth-coupling design** (FreezeWater walkable-ice etc. remains the deferred phase per `water.md` §11); `water_movement` remains dormant, unaffected.
- The T.1 rung-3 director render check (beach vs desert) remains open and is untouched by this recon.

---

*Session evidence: `d:/tmp/twr_staging/` (screenshots `00`-`26`, `editor_err.log`, build logs). Orientation record: ratification §3 scope quote; E3-PF §6 row 7 (~21% aquatic, commit-era); `water.md` rev 1.1 (drift-checked §1.1); `fluids.md` §0.5. Parallel-session note: origin/main advanced 4 CI-workshop commits during this session (`2c1ee9f3b`, `cefebfed6`, `e91547a42`, `273ded3a9` — all CI workflow files) — no file overlap with this report; final push used pull-rebase per protocol.*

---

## Appendix (T.W.1 correction, 2026-07-21): the "~21% aquatic" figure is stale — measured

The T.W.1 dip census (`astraweave-terrain/tests/tw1_dip_census.rs`, seed 12345, radius 6, the editor's exact noise config) measured the CURRENT worlds: Ocean/Coast-classified vertices are **0.13%** of Continental Temperate (1,959 of 1,557,504) and **0.09%** of Mediterranean — not ~21%. The §1.8/§0 "~21%" repeated E3-PF §6 row 7's commit-era figure, which was a seam-fix-moment snapshot: the E3 texturing rounds 2-3 that followed (A.2b floor-to-spline, C.1 coastal gate) deliberately drove aquatic classification down ~200×. Consequence for §2.2's integration picture: enabling the plane at Y=2.0 produces small scattered coastal pools (≈464/465 flooded vertices per world on CT/Mediterranean; Desert and Boreal zero), not filled coastlines — how much MORE water the worlds should carry is a T.2 floor/coverage tuning question. The census also quantified §2.3's divergence direction: only 24-35% of classified-aquatic vertices flood at Y=2.0 (their FINAL median height sits above sea level at 3.12/2.39 — classification runs on provisional pre-erosion heights; erosion/deposition raises basin floors). This appendix corrects the record without altering it; details in `TW1_OUTCOME.md`.
