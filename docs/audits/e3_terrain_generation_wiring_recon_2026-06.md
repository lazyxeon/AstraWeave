# E3-Terrain — Generation-Pipeline Wiring Recon (Diagnostic Note)

> **Campaign**: R-series · **Phase**: M2 / E3-terrain recon
> **Branch/commit**: `campaign/roadmap` @ `e0760327a`
> **Date**: 2026-06-30
> **Mode**: READ-ONLY recon — zero code changed
> **Status**: **RECON COMPLETE — SCOPE RATIFIED 2026-07-01.** Director ratified the full E3-terrain scope: **all three beats, order .1 → .3 → .2, with per-archetype terrain shape (.2 / F.7 splines) IN SCOPE for v1 (not deferred).** Building is a separate beat; this recon stays read-only. Deepening-pass results (empirical biome-variation + splat-path sizing) in §10.

---

## 0. Executive verdict

**The terrain gaps are NOT a bypass.** The editor's "Generate Terrain" button already calls the rich pipeline (`WorldGenerator::generate_chunk_with_climate`), which the terrain architecture trace names *"THE wired production consumer of generation."* The three observed gaps are **downstream consumption/collapse + present-but-off features + one unimplemented differentiation** — not a simpler generator running in place of the rich one.

This is the AIArbiter pattern *in spirit* (a rich system not fully realized at the button) but **not** in the hypothesized form: the button does not route to a dormant-alternative simple generator. It calls the rich system; the rich system's per-vertex outputs are then either **thrown away at the render boundary** (multi-biome) or **undifferentiated at the source** (per-archetype terrain shape).

**Consequence for scope:** E3-terrain is **bounded integration + tuning, with one build-or-defer decision** — not "wire the button to the dormant system," and not "build from scratch."

---

## 1. Recon methodology

Two multi-agent read-only workflows on `e0760327a`:

- **Pass 1 (this note, §2–§9):** 7 parallel deep-readers (editor UI, editor integration, climate/archetype, erosion, multi-biome, noise, arch-trace) + 4 adversarial verifiers on the crux claims. 11 agents, ~1.02M tokens, every claim carries `file:line`. One reader (`editor-ui`) returned a degenerate placeholder; its questions were fully covered by the integration + climate readers and the archetype verifier, so no hole.
- **Pass 2 (§10, in flight):** empirical biome-variation at editor scale + precise sizing of the multi-biome consumption beat, with an adversarial verifier on "does a real editor-scale world actually span multiple biomes, or collapse to one?"

The adversarial verifiers materially sharpened two findings (archetype *is* threaded to biome distribution; erosion *is* on by default) — recorded inline below.

---

## 2. What "Generate Terrain" actually calls

Button → `TerrainPanel::regenerate_terrain` (`tools/aw_editor/src/panels/terrain_panel.rs:2059-2097`) → `TerrainState::generate_terrain` (`tools/aw_editor/src/terrain_integration.rs:301`) → per chunk, **`WorldGenerator::generate_chunk_with_climate`** (`tools/aw_editor/src/terrain_integration.rs:343`).

The trace independently labels line 343 *"THE wired production consumer of generation"* (`docs/architecture/terrain.md:205`). The simpler `generate_chunk` exists (`astraweave-terrain/src/lib.rs:268`) but the editor **never** calls it (confirmed across `regenerate_terrain`, `world_panel.rs:865`, `tab_viewer/mod.rs:1389`).

The button therefore **does** thread the archetype, **does** call the multi-biome path, and **does** run erosion:

| Channel | Wired? | Evidence |
|---|---|---|
| **Archetype** | Threaded — to *biome distribution*, not *shape* | dropdown → `set_world_archetype` sets `config.climate.archetype`; `ClimateMap::sample` reads it at `climate.rs:378` |
| **Multi-biome** | Genuinely computed per-vertex | `apply_per_biome_modulation_to_halo` → `blend_biome_parameters` → per-vertex Whittaker `BiomeId` array (`lib.rs:583-640`) |
| **Erosion** | Runs by default | `AdvancedErosionSimulator::apply_preset_at_world_offset` (`lib.rs:419`), gated on `erosion_enabled` — **default `true`** (`noise_gen.rs:210`) |

The whole world is generated with **one** `primary_biome` string → **one** `ClimateBias` for every chunk (`terrain_integration.rs:304-308`); nothing varies `primary_biome` spatially.

---

## 3. Is the full pipeline assembled end-to-end? Yes — outputs collapsed at the render boundary

The rich chain — halo heightmap → per-vertex climate/Whittaker biome + `biome_param_blending` + archetype splines → world-coordinate droplet erosion on the halo → crop → `TerrainChunk` bundling post-erosion heightmap + `biome_map` + pre-erosion `biome_weights[8]` + per-vertex `biome_ids` — is fully assembled and is the trace's blessed §2 pipeline (`terrain.md:68-106`).

Erosion runs *in generation* (not test-only). **This refutes the CLAUDE.md "AdvancedErosionSimulator dormant/removed" note**, and the trace already flags that contradiction (`terrain.md:241`, `terrain.md:300-301`).

The disconnect is at **consumption**: the genuinely-multi-biome per-vertex `chunk.biome_ids()` has **zero non-test readers workspace-wide** (grep: 8 hits, all under `astraweave-terrain/tests/`). The editor render path discards it and rebuilds material from a **single** `primary_biome`.

---

## 4. Per-gap diagnosis

### Gap 1 — All World Archetypes render (nearly) identically
**Cause class: integration/consumption + missing-implementation + tuning (NOT simple "thread the archetype" wiring).**

- The archetype's **climate envelope IS threaded end-to-end**: dropdown → `set_world_archetype` (sets `config.climate.archetype`, a `climate::WorldArchetype` *struct*) → `ClimateMap::sample` reads seven envelope fields (`climate.rs:378-440`) → shifts temperature/moisture → relocates the point in the Whittaker table → different per-vertex `dominant_biome`. This is **not dormant**. Proven by the distribution tests (`world_archetypes.rs:317-343`): Desert → ~0% TropicalRainforest / ≥40% arid family; Boreal → 0% Savanna.
- **But that biome variation is collapsed by Gap 2**, so it is invisible — biomes aren't rendered per-vertex.
- The archetype → **terrain shape** channel is **doubly dead**: (i) `regional_archetype_mask` defaults to `None` with **no production caller** (`lib.rs:196`; `terrain.md:406` confirms the `Some`-branch is "unreachable in production"); (ii) **even wiring the mask would not help today** — all six `bootstrap_splines_*` factories return byte-identical `d5fix_baseline_spline_set()` (`spline_types.rs:505-538`), so per-archetype shape (F.7) is unimplemented.
- One **live-but-subtle** archetype→shape channel exists: per-biome `mountains_amplitude` (0.0–1.6) via the climate field multiplies the mountain noise layer (`noise_gen.rs:646`). A Boreal world (BorealForest/Tundra amp 1.5) yields mountains ~1.5–1.9× taller than a Desert world (SubtropicalDesert 1.0 / Savanna 0.8) — a real geometric difference, but subtle (multiplies only the mountain layer).

> **Verifier nuance (`verify:archetype-threading` → PARTIAL):** the hypothesis "archetype does not reach terrain shape" is *too strong*. The bootstrap-spline shape channel is dead across *all* archetypes (F.7 pending), but the per-biome-amplitude-via-climate channel is alive and archetype-dependent. Re-aim the diagnosis at (a) the **magnitude/visibility** of the per-biome-amplitude channel and (b) the **unimplemented F.7 spline differentiation**, not at "archetype is unwired."

### Gap 2 — Only single-biome worlds generate (no multi-biome / transitions)
**Cause class: integration/consumption gap — multi-biome is generated across space but NOT consumed for rendering.** *(The clearest, highest-value beat.)*

- The rich path computes a genuine per-vertex Whittaker `BiomeId` array (`chunk_biome_ids`) that **varies across space** (climate temperature noise scale 0.001 ≈ 1000 WU wavelength, ±8 °C, over a 2560–5120 WU world). Stored on the chunk (`lib.rs:452`; accessor `chunk.rs:162`).
- `chunk.biome_ids()` has **zero non-test readers** (grep: 8 hits, all tests). Computed-but-unused.
- The editor render path instead: (a) overwrites `biome_map` to a single `primary_biome` (`terrain_integration.rs:344-346`) — a **red herring**, since `biome_map` is discarded in the mesh (`let _ = biome_map`, `:767`); (b) drives the splat generator from a single `primary_biome` (`create_local_splat_generator(seed, primary_biome)`, `:778`) — **this is the primary material driver** (`splat_map` wins over the `biome_weights` fallback in every generated vertex, `:819-825`); (c) computes per-vertex `biome_weights` from a **single** `ClimateBias` derived from the primary-biome string (`:308-309`, `lib.rs:373-383`) — elevation-driven only, one climate for the whole world.

> **Verifier (`verify:single-biome-cause` → CONFIRMED):** every sub-part proven. The only wording correction: the exercised `biome_weights` path reads the chunk's precomputed `pre_erosion_biome_weights` (not the inline `elevation_to_biome_weights` fallback), but that precomputation uses the identical single-`ClimateBias` function, so the diagnosis is unchanged.

### Gap 3 — Pre-erosion + repetitive look
**Two separate sub-issues.**

- **Erosion — NOT absent → tuning + a dead button.** Erosion is wired and runs by default (`erosion_enabled: true` default, `noise_gen.rs:210`; executes at `lib.rs:419`). "Pre-erosion look" is a **tuning** issue: presets were deliberately de-tuned (droplet counts halved, 50k→25k / 100k→35k) for a 30 s budget (`advanced_erosion.rs:207-250`); the climate→preset map is coarse (4 presets, `advanced_erosion.rs:262-272`); the **Arid preset has no hydraulic pass** (`advanced_erosion.rs:149-160`). Note the manual **"Apply Erosion" UI button is a no-op stub** (`terrain_panel.rs:1911-1934`) — it logs and times but calls nothing. *(Note: `erosion_strength` 0.3 is only consumed by the legacy `generate_chunk` CA path; the advanced path's intensity is governed entirely by the selected `ErosionPreset`, not that scalar.)*
- **Repetitive — a config-default gap.** The base elevation layer is **single-octave raw Perlin** (the `NoiseType::Perlin` arm ignores octaves/persistence/lacunarity, `noise_gen.rs:465`) at scale 0.005 (~200 WU wavelength). Domain warping **exists but is never used** in the editor path (requires `NoiseType::DomainWarped`, never set). `continental_enabled` = false (no regional clustering, `noise_gen.rs:120-122`). `base_derivative_weighted` = false (`noise_gen.rs:166-168`). So the anti-repetition machinery is **present-but-off** in the editor default. The mountains layer *is* multi-octave (6-octave RidgedMulti) but not warped.

> **Verifier (`verify:erosion-runs` → CONFIRMED):** erosion is wired and enabled by default; the "pre-erosion" symptom is preset subtlety, not absence.

---

## 5. Architectural finding

**The rich pipeline is unambiguously the intended generation path, and the editor already calls it.** What is dormant/missing is narrow and specific:

1. per-vertex `biome_ids` consumed for rendering (Gap 2);
2. the multi-archetype mask `Some`-branch — a **deliberate** dormant branch per the trace (`terrain.md:288-289, 406`);
3. per-archetype **terrain-shape** splines (F.7 — all six identical today);
4. rich noise features (domain-warp / derivative-weighted / continental) on-by-default (Gap 3).

**§7.7 wrapped-component trap (recorded):** `RegionalArchetypePanel.mask` owns a painted mask that is **never handed to the generator**. `set_mask` has only `#[cfg(test)]` callers (`regional_archetype_panel.rs:262`, test call sites `:1227/:1332`); `regenerate_terrain` builds a fresh `TerrainState::new()` with no mask hand-off (`terrain_panel.rs:2059`). This is the exact wrapped-component resource-identity pattern from CLAUDE.md §7.7.

**Two-type distinction any wiring beat must respect:** `climate::WorldArchetype` (a *struct* — the climate envelope, `climate.rs:127`) vs `world_archetypes::WorldArchetypeId` (a 6-variant *enum*, `world_archetypes.rs:44`). The enum resolves to the struct via `WorldArchetypeId::default_archetype()` (`world_archetypes.rs:145-153`). The editor stores the enum, converts to the struct, and writes `config.climate.archetype`.

**Trace currency:** `docs/architecture/terrain.md` v1.1, last verified `7c29b8182` / 2026-06-25 — broadly current with the Phase-1.6-F.4.B.3.D / Phase-1.X-F campaign families, but it does **not** surface the editor-side reductions (single-`primary_biome` collapse, `biome_map` overwrite, primary-biome splat driver, `chunk_biome_ids` computed-but-unused). Those live below the trace's altitude (it defers the material slice to `terrain_materials.md`, which also does not flag them).

---

## 6. E3-terrain scope — RATIFIED 2026-07-01

Ratified: **all three beats, sequence .1 → .3 → .2, with .2 (per-archetype terrain shape / F.7 splines) IN SCOPE for v1** — archetypes should differ in *landform*, not only biome palette. The build is a separate beat (this doc is the read-only recon of record).

- **E3-terrain.1 — Multi-biome consumption wiring (integration; size pending §10).** Connect the already-generated per-vertex `chunk_biome_ids` (or a per-vertex climate-driven selection) into the splat generator + material weights, replacing the single-`primary_biome` / single-`ClimateBias` collapse. **This one beat fixes "single-biome" AND automatically makes the archetype's biome-distribution effect visible.** Lands in the intricate splat/material path (`terrain_materials.md` domain, §7.7 territory) — size being nailed down in §10.
- **E3-terrain.2 — Per-archetype terrain shape (RATIFIED IN SCOPE, build beat).** Author distinct `bootstrap_splines_*` per archetype (F.7) so archetypes differ in *landform* (mountain amplitude/scale, base elevation), not only biome palette. Today all six factories return the identical `d5fix_baseline_spline_set()` (`spline_types.rs:505-538`); this beat gives each of the 6 `WorldArchetypeId` variants its own spline set. Runs after `.1`+`.3`. (Wiring the regional mask `Some`-branch for *spatial* multi-archetype within one world remains a separate, larger feature — not part of `.2` unless separately ratified.)
- **E3-terrain.3 — Aesthetic/tuning pass (tuning, iterative).** Turn on rich noise features in the editor default (domain-warp / derivative-weighted / continental) to kill repetition; tune/expose erosion intensity; fix or delete the no-op "Apply Erosion" button.

**Golden coupling:** any of these changes terrain output → the held **E3.a-2 golden re-bake stays held** until the terrain is right.

---

## 7. Definitive answer to the recon's primary question

> *Editor-wiring-gap vs. pipeline-integration-gap?*

**Neither, exactly — it is a consumption/collapse gap plus present-but-off features.** The editor **is** wired to the rich pipeline (not a bypass), and the pipeline **is** internally assembled end-to-end (not an internal integration hole in *generation*). The failure is that the rich pipeline's per-vertex outputs are **collapsed at the render boundary** (multi-biome, single-climate splat) and its differentiation is **incomplete at the source** (per-archetype shape) or **switched off** (noise anti-repetition features). This is decisively **not** "wire the button to a dormant generator," and decisively **not** "build from scratch."

---

## 8. Evidence appendix — key anchors

| Fact | Anchor |
|---|---|
| Editor calls rich generator | `tools/aw_editor/src/terrain_integration.rs:343` |
| Rich generator body | `astraweave-terrain/src/lib.rs:317-456` |
| Single primary_biome / ClimateBias for whole world | `terrain_integration.rs:304-308` |
| `biome_map` overwrite (red herring) | `terrain_integration.rs:344-346` |
| `biome_map` discarded in mesh | `terrain_integration.rs:767` |
| Splat driven by single primary_biome | `terrain_integration.rs:778`, `:1209-1211` |
| `chunk_biome_ids` computed | `lib.rs:388-392`, stored `:452` |
| `chunk.biome_ids()` zero non-test readers | grep: 8 hits all under `astraweave-terrain/tests/` |
| Archetype read by ClimateMap::sample | `climate.rs:378-440` |
| Two archetype types | `climate.rs:127` (struct) vs `world_archetypes.rs:44` (enum) |
| Archetype→shape doubly dead | `lib.rs:196` (mask None) + `spline_types.rs:505-538` (identical splines) |
| Live per-biome amplitude channel | `noise_gen.rs:646` |
| Erosion enabled by default | `noise_gen.rs:210` |
| Erosion runs in editor path | `lib.rs:410-419` |
| Erosion presets de-tuned | `advanced_erosion.rs:207-250` |
| "Apply Erosion" button is a no-op | `terrain_panel.rs:1911-1934` |
| Base noise = single-octave Perlin | `noise_gen.rs:465` |
| Domain warp present-but-off | `noise_gen.rs:439-447` |
| §7.7 mask trap | `regional_archetype_panel.rs:262` (test-only `set_mask`), `tab_viewer/mod.rs:1492-1494` |
| Trace blesses climate path | `docs/architecture/terrain.md:65-66, 205` |

---

## 9. Tree state

Read-only recon — nothing changed. Clean working tree at `e0760327a` on `campaign/roadmap`.

---

## 10. Deepening recon (Pass 2) — results

> The Pass-2 multi-agent workflow **failed on schema-serialization** (agents produced valid analysis but could not fit the StructuredOutput schema; retry cap exceeded after ~22 min / 3 agents). Re-run **inline by the orchestrator**; conclusions below are first-hand file reads, not agent summaries.

### (A) Does a real editor-scale world span multiple biomes? **YES — confirmed analytically.** E3-terrain.1 is self-sufficient.

Whittaker threshold analysis (`biome_lookup.rs:200-366`) for the default **Continental Temperate** envelope (mean 12 °C ± 8 °C variance; 1100 mm ± 400 mm):

- Temperature range ≈ [4, 20] °C **straddles** `BOREAL_MAX_TEMP_C=5` and `COLD_DESERT_MAX_TEMP_C=18` → BorealForest (cold end) / TemperateDeciduousForest (dominant) / TemperateGrassland (warm-dry end).
- Moisture range ≈ [700, 1500] mm **reaches** `TEMPERATE_RAINFOREST_MIN_MOISTURE_MM=1500` → TemperateRainforest at the wet peaks.
- Mountain noise amplitude 480 (`noise_gen.rs`) **far exceeds** the elevation overlays (Scree ≥220 m, Alpine ≥280 m, SnowCap ≥350 m) → Alpine / SnowCap / Scree on peaks; Ocean / Coast / Beach below sea level.
- **Net:** a real world is genuinely multi-biome — temperate-forest-dominated, with boreal/rainforest patches and alpine/snow peaks. It does **not** collapse to one biome.

**Archetype sensitivity is strong:** archetype means differ sharply (Desert 25 °C / 150 mm → SubtropicalDesert/Savanna dominant; Boreal −3 °C / 500 mm → Tundra/BorealForest dominant), so once biomes render per-vertex, the archetypes would show **visibly different biome palettes** automatically.

**Scope implication:** wiring `chunk.biome_ids()` into rendering is **self-sufficient** to produce a visibly multi-biome world — **no climate-tuning companion is required** for correctness (tuning could make transitions more dramatic, but that's aesthetic, → E3-terrain.3). **Caveat:** a single 512 WU chunk is only ~½ a climate wavelength, so one chunk may be dominated by 1–2 biomes + elevation overlays; the full spread shows across the radius-5 world. The director's uniform-terrain screenshot is the **render collapse** (single-`primary_biome` splat), not a climate-field limitation.

*(Note: `terrain_sweep_results/summary.txt` + `aggregate.json` are a mutation-testing kill-rate sweep — NOT biome-distribution data. No empirical biome counts on disk; (A) answered rigorously from the Whittaker thresholds.)*

### (B) Size of the multi-biome consumption beat (E3-terrain.1): **MEDIUM.**

Why **not Large** (no pipeline re-architecture):

- The GPU material channel is **already per-vertex**: the render `TerrainVertex` carries `material_ids[4]` + `material_weights[4]` — an 8-layer per-vertex blend already uploaded and blended by the shader (`tools/aw_editor/src/viewport/types.rs:24-35`). **No shader / vertex-format change needed.**
- `generate_splat_map` is **already per-vertex and biome-agnostic** — it evaluates height/slope rules per vertex (`texture_splatting.rs:446-448`); only the *rule set* is single-biome (chosen by `create_local_splat_generator(primary_biome)`).
- The 8 material layers **already map to 8 biomes** (`terrain_biome_placeholder.rs:27-36` → `[Grassland, Desert, Forest, Mountain, Tundra, Swamp, Beach, River]`).
- Heights already vary per-biome (per-vertex `mountains_amplitude`, `noise_gen.rs:646`). **Only color is single-biome.**

Why **not Small** (real integration work):

- **Taxonomy bridge**: `chunk.biome_ids()` returns `BiomeId` (19 Whittaker variants); render layers are 8 `BiomeType` slots. `.1` must map 19 → 8 (or expand the layer set) and reconcile the overloaded splat semantics (texture-type layer: grass/sand/rock vs biome-color layer: Grassland/Desert/…).
- Thread `chunk.biome_ids()` into `generate_heightmap_mesh` (its signature doesn't currently receive it, `terrain_integration.rs:742-755`).
- Smooth transitions: `biome_ids` are hard per-vertex; avoiding hard color edges means blending (the D.4 biome-weight machinery or per-vertex weight blend).
- **§7.7 caution:** this is the texture-data attribute layer of the Real-Fix.C sibling-attribute drift trap — touch carefully.

**Files `.1` touches:** primarily `tools/aw_editor/src/terrain_integration.rs` (`generate_terrain` to thread biome_ids; `generate_heightmap_mesh` signature + material-slot derivation; retire/augment `create_local_splat_generator`), a new `BiomeId → layer` mapping, possibly the placeholder color set. **No `astraweave-render`/shader change** for a minimal version.

**Scoping fork inside `.1`:** stay on the editor's **8-layer placeholder path** (minimal, sufficient to show multi-biome — recommended for `.1`) vs migrate to the **32-layer canonical render path** (larger; `terrain_materials.md` calls the editor's 8-layer `SplatMapGenerator` "dormant/test-only" and the 32-layer path canonical — defer that migration to `.3` or a later beat).

### Ratified scope (2026-07-01)

- **E3-terrain.1 — Multi-biome consumption wiring: MEDIUM, self-sufficient.** Fixes single-biome AND surfaces archetype biome-palettes for free. First. Stay on the 8-layer placeholder path.
- **E3-terrain.3 — Tuning: iterative.** Noise anti-repetition (turn on domain-warp/derivative-weighted/continental), erosion visibility, kill the no-op "Apply Erosion" button. Optionally the 32-layer material migration.
- **E3-terrain.2 — Per-archetype terrain shape (F.7 splines): RATIFIED IN SCOPE (build beat).** Author distinct `bootstrap_splines_*` per archetype so archetypes differ in *landform*, not only biome palette. Runs after `.1`+`.3`.

Ratified order **.1 → .3 → .2**. Golden E3.a-2 re-bake stays held until the terrain-shaping beats land. **Build is a separate beat — this recon is read-only and complete.**
