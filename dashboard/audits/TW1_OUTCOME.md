# T.W.1 — Surface the Water System in the Editor (outcome)

> **Beat:** T.W.1 (terrain series, first water beat) · **Date:** 2026-07-21/22 · **Executes:** `TW1_RATIFICATION_2026-07-21.md` (T.W-R gate, seven answers + beat split)
> **Commits:** ratification record `f398a3778` (doc) · code `88c6ba588` (12 files, +667/−959) · docs (this note + trace bumps + corrections) — hash in the session report.
> Evidence tiers: **built** / **run** / **verified**. Session screenshots in `d:/tmp/tw1_staging/` (local, not committed). Recon spec: `TWR_WATER_RECON.md` (`09c589a5d`).

## 0. Summary

The editor renders water again — correctly, gated by what the world actually contains, at the world's one sea level. The route that killed the process in the recon (Wizard → Tropical Coast → Generate) now generates and renders at steady frame rate; water was observed live in the viewport (animated surface + depth-delta shoreline foam) through the fixed HDR pipeline; the enable checkbox census-defaults ON for the Mediterranean world and OFF for Desert. **The beat also falsified its own mission premise**: the "~21% aquatic renders dry" figure was a stale seam-fix-era snapshot — the dip census measured the real aquatic fraction at **~0.1%**, so default-sea-level water is small scattered coastal pools, not filled coastlines. How much more water the worlds should carry is a T.2 floor/coverage tuning question feeding the director's gate. All seven ratified items landed; three pre-existing masks that had hidden the format bug from CI are now removed or documented.

## 1. Work items — what shipped, with evidence

### 1.1 The format fix (+ the three masks that hid the bug)

- **Fix:** `engine_adapter.rs::set_water_enabled` now constructs `WaterRenderer` with `renderer.hdr_format()` (was `surface_format()` — Rgba8UnormSrgb pipeline vs the Rgba16Float HDR water pass target, the recon §1.6 panic). `WaterRenderer` stores `target_format` (+ getter); **`Renderer::set_water_renderer` asserts the match at install** — any future consumer repeating the mistake fails loudly at install time, not at first draw.
- **Regression tests, designed to fail on pre-fix code — demonstrated, not assumed:**
  1. `test_renderer_water_initialization` (rewritten): the pre-existing version never drew — **`render()` is a headless NO-OP** (`acquire_surface_texture` → `None` → early `Ok`), and even a forced draw's validation error would have been **swallowed by the headless uncaptured-error handler, which only logs**. The rewrite drives `update_water` (dormant-skip off) through **`draw_into`** (the editor path) into an offscreen color+depth pair under a wgpu **validation error scope**. Run against the unfixed code it failed with the byte-identical editor error — "`Incompatible color attachments … [Rgba16Float]` vs `'water_pipeline'` `[Rgba8UnormSrgb]`" (`FAILED. 0 passed; 1 failed; 1278 filtered out` — cited run, 2026-07-21). With the fix: passes.
  2. `test_water_renderer_wrong_format_rejected_at_install` (`#[should_panic]`): installing a surface-format water renderer panics at install. On pre-fix code (no assert) this test fails by not panicking.
- **Editor crash route re-driven (run + verified):** Wizard → Tropical Coast → seed 12345 → Generate World — the exact recon §1.6 route. Editor **alive 25 s past the click** (`ALIVE pid=402536`, stderr panic-free), world rendered at 68 FPS (`03_crash_route_survived.png`; console shows the full wizard action line). Pre-fix this was a process abort.
- Suites: `cargo test -p astraweave-render --lib water` → **18 passed; 0 failed** (1,263 filtered out).

### 1.2 The gate replacement

Legacy `matches!(primary_biome, "swamp"|"beach"|"river")` (unreachable since the archetype dropdown replaced that field) → **aquatic-biome census default**: `ids_contain_aquatic` (Ocean/Coast/River — the sub-sea ids; Beach/Wetland sit above sea level and do not imply plane water; River included so hydrology worlds auto-enable when they arrive) + `TerrainState::has_aquatic_biomes()` over the generated chunks' per-vertex `BiomeId`s; set on every generation completion in `poll_generation`; the Water checkbox overrides in both directions until the next generation. `main.rs` water sync caches `(enabled, style)` and drives `set_water_enabled`/`set_water_level`.
Tests: `cargo test -p aw_editor --lib tw1` → **2 passed; 0 failed** (4,021 filtered out). Observed live: checkbox auto-ON for Mediterranean r6 (`07_med_r6_overview.png`), auto-OFF for Desert r6 (`20_desert_overview.png`) and for the aquatic-free radius-1 wizard world (`05_terrain_panel.png`).

### 1.3 Sea-level unification

`astraweave-render` already depends on `astraweave-terrain` (Cargo.toml:48) — no new edge needed: **`pub const DEFAULT_WATER_LEVEL: f32 = astraweave_terrain::SEA_LEVEL;`** (single source), value-pinned by `test_default_water_level_is_world_sea_level` (== SEA_LEVEL, == 2.0, == `WaterUniforms::default().water_level`) so changing SEA_LEVEL upstream is a loud cross-system decision. The panel's `water_level` now defaults to `SEA_LEVEL` — the frozen-at-0.0, UI-less knob defect is dead.

### 1.4 Hygiene deletions (ratified in full; caller censuses re-verified at HEAD)

- **Dead SPH-era panel block** (~210 lines of types + fields): `FluidSimParams` (65,536-particle SPH config), `WaterBodyPreset`, `FluidQualityPreset`, `DetectedWaterBodyInfo`, `FluidStats`, `auto_detect_water_bodies`, `show_fluid_debug` — re-verified zero readers/zero UI sites; **~200 tested-but-uncalled tests** deleted with them (panel-internal + `mutation_resistant_terrain.rs` sections; surviving suites: mutation_resistant_terrain **70 passed**, terrain_panel lib **58 passed**, 3,965 filtered). `astraweave-fluids` has its own unrelated `WaterBodyPreset` — untouched.
- **`apply_brush_paint`** (~85 LoC legacy biome-map paint): zero callers re-verified; deleted, along with its only-callee `id_to_biome`. `chunk.biome_map_mut()` **stays** — its second caller is generation's primary-biome override (`generate_terrain`).
- **W.2c.2 scaffolding weaves** (`engine_adapter.rs` hardcoded Part/Raise/Freeze set pieces): deleted; the editor surface starts weave-free. A real editor weave feed remains future work.

### 1.5 UI + per-archetype style

Terrain panel Water row: enable checkbox (census-defaulted, hover-documented) + level slider (−5..15 m, clamped) + "Sea level" reset button — wired through the existing `set_water_level` plumbing (observed working: level raised to 12.5 m and reset live). `WaterStyle` per archetype (trivial map, ratification #7): ContinentalTemperate/EquatorialTropical/Mediterranean/Custom → Ocean; BorealSubarctic/Desert → Lake (cold inland water / oasis pockets). Threaded as `set_water_enabled(enabled, style)` through widget → viewport renderer → adapter; `main.rs` cache keys on the pair.

### 1.6 Observed: water live in the editor (run + verified)

Mediterranean seed 12345 r6 (169 chunks, 3.18M tris, ~55 FPS): with the level raised to 12.5 m, an **animated water surface with white depth-delta shoreline foam fills the foreground basin, wrapping the beach-sand shore** — the W-series surface (Gerstner + refraction + foam) drawn live through the editor's fixed HDR pipeline (`08_med_level15.png` — the beat's money frame). At the default sea level 2.0, the same world shows no water at overview distance (`09_med_sealevel.png`) — exactly the census prediction (§2: 465 flooded vertices ≈ 0.03% of the world).

## 2. The dip census (item 7) — and the two findings it forced

`astraweave-terrain/tests/tw1_dip_census.rs` (`#[ignore]`, release): seed 12345, radius 6, **the editor's exact noise config** (the panel routes `set_noise_params(6, 2.0, 0.5, 50.0)` over `NoiseConfig::default()`'s amplitude-100/octaves-4 — an early census draft missed this; the final run mirrors `regenerate_terrain` faithfully). Partition is **by per-vertex `BiomeId`**, not by re-sampling continentalness: classification runs through the D.4 scattered-convolution (jittered multi-sample at provisional elevation, `biome_param_blending.rs:195-209`), so an independent point-sample cannot reproduce the classifier's effective value near the 0.40 gate — a draft tried and mis-bucketed every flooded coastal vertex. Run: **1 passed (132.84 s)**; both internal partition asserts held.

| archetype | vertices | below-sea | emergent dips (terrestrial-classified) | aquatic-flooded (Ocean/Coast-classified) | aquatic-classified total | flooded % of classified | aquatic final min / median |
|---|---|---|---|---|---|---|---|
| ContinentalTemperate | 1,557,504 | 741 (0.05%) | 277 | 464 | 1,959 (**0.13%**) | 23.7% | 0.30 / **3.12** |
| Mediterranean | 1,557,504 | 1,031 (0.07%) | 566 | 465 | 1,340 (**0.09%**) | 34.7% | 0.31 / **2.39** |
| Desert | 1,557,504 | **0** | 0 | 0 | 0 | — | — |
| BorealSubarctic | 1,557,504 | **0** | 0 | 0 | 0 | — | — |

**Finding A — the mission premise was stale.** "~21% aquatic renders dry" (recon §1.8 / E3-PF §6 row 7) was a seam-fix-moment snapshot; the E3 rounds that followed (A.2b floor-to-spline, C.1 coastal gate) deliberately drove aquatic classification down ~200×, to ~0.1%. Corrected append-only in `TWR_WATER_RECON.md` (new appendix). Consequence: at sea level the fixed water system shows small scattered coastal pools; **how much of the world should be ocean is a T.2 coverage/floor tuning decision**, not a plumbing gap — the plumbing is now correct at any coverage.

**Finding B — classification and rendered heights disagree about the sea.** Biome ids classify on provisional pre-erosion heights; erosion/deposition then raises basin floors, so the median FINAL height of an "Ocean/Coast" vertex sits *above* sea level (3.12 / 2.39) and only 24-35% of classified water actually floods at Y=2.0. A §7.7-family divergence, quantified for T.2. (Also: at this seed/config the emergent-dip class — plane water over terrestrial-classified bed, the ruling-#2 case — is 277/566 vertices on CT/Med and **zero** on Desert/Boreal; no oasis exists to observe at this seed.)

## 3. Verification ladder

1. **Unit/regression:** render water suite **18 passed** (1,263 filtered; incl. the demonstrated-fails-on-old-code draw-through + `should_panic` install guard + sea-level pin); editor `tw1` census tests **2 passed** (4,021 filtered); `mutation_resistant_terrain` **70 passed**; `terrain_panel` lib **58 passed** (3,965 filtered); dip census **1 passed** (132.84 s). `cargo check --workspace` **exit 0**; `cargo run -p aw_trace_sync -- --check` in sync; Doc-Truth Lint clean on the new docs.
2. **Crash route:** re-driven live, green (§1.1; screenshots `01`-`03`).
3. **Observation:** Mediterranean (census-ON, pools at sea level, visible water at raised level — `07`-`09`), Desert (census-OFF, bone-dry — `20`), wizard world (census-OFF at radius 1 — `05`). **CT and Boreal were not screenshot-captured**: mid-session the archetype ComboBox stopped opening under synthetic input (checkbox/slider/buttons kept working; one relaunch recovered it once, spent on Desert as the ruling-relevant archetype). Their expected states are census-pinned (CT ≈ Mediterranean with 1,959 aquatic ids → checkbox ON, small pools; Boreal zero → OFF, dry) — **one dropdown click each at the director's gate confirms**. The combo quirk is an automation-input observation, not a shipped defect claim; noted for a future editor-UI health pass.

## 4. Director gate repro (the render verdict is yours — this note claims no rung 3)

`cargo editor` → Window → Panels → Terrain → radius 6, seed 12345:
- **Mediterranean**: Water checkbox should arrive CHECKED; at sea level expect small pools at low coast transitions (the census's 465 vertices); drag the level slider up (e.g. 12 m) to see the surface + shoreline foam unmistakably, then "Sea level" to reset. Switch checkbox off/on to verify the override.
- **Desert**: checkbox arrives UNCHECKED, world bone-dry.
- **ContinentalTemperate / Boreal**: expect ON-with-pools / OFF-dry per the census table.
- **Crash route**: File → New World (Wizard) → Tropical Coast → Generate World — must render, not abort.
The gate also owes the **final §2.3 ruling** (emergent dips = feature/defect) on §2's numbers, and — new, from Finding A — a **T.2 scope note on aquatic coverage** (should worlds carry more ocean than ~0.1%?).

## 5. Residue / open items

- **Repo is not fmt-clean** in astraweave-render/terrain/aw_editor: a crate-wide `cargo fmt` this session reformatted ~50 files this beat never touched (real formatting drift, not line endings — e.g. `tonemap_probe.rs` tuple layout). Restored to keep the beat diff honest; `cargo fmt --all --check` would fail today → CI-workshop candidate.
- **7 pre-existing clippy warnings** in astraweave-terrain lib (`noise_gen.rs:673/675`, `regional_archetype_mask.rs:391/613/656/690`, `lib.rs:489`) — in CI's warn-tier matrix; block any `-D warnings` clippy run that traverses terrain. This beat's files: zero new warnings (render clippy exit 0).
- The headless-render masks (§1.1) are documented in `water.md` v1.2 §5; other GPU "doesn't crash during render" headless tests may be similarly hollow — future test-health sweep candidate.
- The archetype-ComboBox synthetic-input quirk (§3.3). The `weaving_playground` wedge from T.W-R remains open (demo-health pass; untouched per anti-drift).
- T.W.2 (carve-brush + volume entities) is drafted only after this beat's gate.

## 6. Doc riders landed

`TW1_RATIFICATION_2026-07-21.md` (`f398a3778`); `water.md` → v1.2 (invariants 11-12, §5 rows, Appendix B correction of "W-FU-2 woke the editor water"); `aw_editor.md` → v1.6 (Invariant 28, metadata narrative); `F4_3_EXECUTION_REPORT.md` append-only T.W.1 correction footnote; `TWR_WATER_RECON.md` append-only census-correction appendix.
