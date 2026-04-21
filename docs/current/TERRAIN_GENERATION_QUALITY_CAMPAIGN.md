# Terrain Generation Quality Campaign — Phase 1.6-F

**Status**: F.1–F.2 complete, F.3–F.5 not yet started. Drafted 2026-04-21 as F.0 artifact.
**Scope**: Wire the already-implemented-but-unused terrain-generation components (`AdvancedErosionSimulator`, `DomainWarpedNoise`, `ClimateMap`) into the runtime biome-noise preset path, tune preset amplitudes to match Phase 1.5's elevation bands, and rewire climate as a per-vertex spatial field. Five sub-phases (F.1–F.5) executed as separate sessions.
**Author**: Plan drafted from `docs/audits/heightmap_generator_audit_2026-04-21.md` findings and design decisions captured in the F.0 prompt session 2026-04-21 between Andrew and Claude. Code references accurate as of 2026-04-21; verify before execution.
**Prior work**: `docs/audits/heightmap_generator_audit_2026-04-21.md` (the audit that surfaced the unwired components and selected Option F as the intervention path); `docs/audits/phase_1_5_tuning_investigation_2026-04-20.md` (records the stale 125-unit measurement that F.1 corrects); `docs/current/TERRAIN_MATERIAL_SYSTEM_CAMPAIGN.md` (parent campaign — Phase 1 and Phase 1.5 re-mark-COMPLETE is deferred to F.5 closeout).
**Outcome on completion**: Editor terrain generation uses AAA-parity multi-stage pipeline (domain-warped base noise + per-climate preset-driven `AdvancedErosionSimulator` + spatial climate field) producing geologically coherent topology across six climate biases. Forest and Mountain biomes visibly render in the default `grassland` project. The "Apply Erosion" editor button functions. Phase 1 and Phase 1.5 are re-marked COMPLETE in the parent campaign.

---

## 0. How to use this document and anti-drift discipline

This plan is the authoritative design reference for Phase 1.6-F. It adapts the parent campaign's §0 discipline to sub-phase execution.

### Discipline imposed

1. Every sub-phase's completion commit must update §9 of this document (the phase-status block) to mark the sub-phase COMPLETE, with the commit hash of completion.
2. No sub-phase is "complete" until both its plumbing verifies (compilation + tests + behavioral check per each sub-phase's success criteria) AND the §9 status update commit has landed.
3. The "Status" header at the top of this document must be updated as sub-phases land: "F.1 complete, F.2–F.5 not yet started" → "F.1–F.2 complete, F.3 in progress" → "Campaign complete (date)."
4. Design decisions captured in §2 are authoritative — they are resolved once, in F.0, and sub-phases F.1–F.5 execute against them rather than re-deciding them. If a design decision in §2 proves infeasible during execution, stop, record a deviation in §10, and escalate for a revised decision before proceeding.
5. Any discovered need to deviate from this plan during execution (including F.0 itself if edits prove necessary during draft review) must be recorded in §10 (Deviations log) with rationale, before or in the same commit as the deviation itself.
6. No "while-I'm-here" code changes. Sub-phase scopes in §3–§7 are exclusive; touching files outside the listed scope is a deviation that must be logged.

### Anti-pattern this plan explicitly prevents

The Phase 1 / Phase 1.5 cycle landed twice with COMPLETE markers that had to be reverted when visual inspection exposed issues that code-level checks had not caught (see parent campaign §9 entries for `7edb15515` and Phase 1.5 close-out). The parent campaign's §0 discipline learned the lesson that code-level success is not the same as plan-level success until the user-visible behavioral gate is verified. This campaign must not repeat that failure: each sub-phase's success criteria in §3–§7 include a behavioral verification item, not just compilation and unit tests.

---

## 1. Design summary

### 1.1 The problem being solved

The editor's terrain generation is "golf-course gentle": runtime Y span for the default grassland-primary project is ~40 units, whereas Phase 1.5's elevation bands (commit `990dbac63`) assume a ~125-unit span. Forest and Mountain biomes are consequently near-zero weight at almost every vertex and do not visibly render. The audit (§4, §5 of `heightmap_generator_audit_2026-04-21.md`) established this is driven by three independent factors:

1. **Conservative preset amplitudes** — the grassland `BiomeNoisePreset` sets `mountains_amplitude = 15.0`; the same generator code path with mountain-preset amplitudes (`mountains_amplitude = 210.0`) produces a 252-unit Y span. 14× difference in a single parameter accounts for ~90% of the topology gap.
2. **DomainWarpedNoise never used by any runtime preset** — the `DomainWarped` variant of `NoiseType` is fully implemented at `astraweave-terrain/src/noise_gen.rs:154-211` and selectable by `NoiseConfig`, but the editor's `BiomeNoisePreset` struct does not carry a `noise_type` field and no preset sets it. Plain Perlin and analytical noises produce visibly layered octave structure rather than organic geological features.
3. **`AdvancedErosionSimulator` unwired, `ClimateMap` output discarded** — 902 lines of production-quality particle hydraulic + thermal + wind erosion with five named presets sit at `astraweave-terrain/src/advanced_erosion.rs` with zero production callers. The editor's "Apply Erosion" button handler at `tools/aw_editor/src/panels/terrain_panel.rs:1707-1730` is a stub. Climate data is generated at `astraweave-terrain/src/lib.rs:263-267`, feeds `assign_biomes` to produce a `biome_map`, then the editor at `tools/aw_editor/src/terrain_integration.rs:344` overwrites every entry of that biome_map — so climate never reaches the rendered result.

### 1.2 The target

AAA-parity terrain generation comparable to Enshrouded and Crimson Desert, achieved by combining: (a) amplitude tuning so each climate preset produces a runtime Y span matching Phase 1.5's elevation bands; (b) `DomainWarpedNoise` enabled on appropriate presets for organic macro-features; (c) wired `AdvancedErosionSimulator` with per-climate erosion presets for geologically coherent shape (rivers, alluvial fans, talus slopes, wind-streaked dunes); (d) climate as a per-vertex spatial field rather than a single-string project-level bias; (e) chunk-boundary halo strategy that prevents visible seams from particle-based erosion.

### 1.3 The five-sub-phase breakdown

- **F.1 Amplitude tuning.** Raise `base_amplitude` / `mountains_amplitude` / `detail_amplitude` in the six biome-noise presets at `terrain_panel.rs:1816-1991` so runtime Y span matches Phase 1.5's bands. Forest and Mountain become visibly rendered. Resolves parent campaign's Issue 2.

- **F.2 DomainWarped noise integration.** Extend `BiomeNoisePreset` with a `noise_type` field. Enable `DomainWarpedNoise` on grassland and mountain base-elevation layers. Organic macro-features (meandering ridges, irregular valleys) replace the current smoothly-varying analytical noise where the design calls for it.

- **F.3 AdvancedErosionSimulator wiring with halo.** Replace the simple `chunk.apply_erosion(strength)` call at `astraweave-terrain/src/lib.rs:276-278` with `AdvancedErosionSimulator::apply_preset` keyed on the chunk's climate bias. Implement halo expansion (1-chunk halo, generate-erode-crop) for chunk-boundary continuity. Capture pre-erosion Y for biome-weight computation per §2.5.

- **F.4 Climate as spatial field.** Rewire `ClimateMap` output so it reaches `elevation_to_biome_weights` as a per-vertex `ClimateSample { temperature, moisture }`. Add a new `"mixed"`/`"auto"` value to the editor's primary-biome dropdown that enables climate-driven biome assignment across lat/long/elevation/water-distance gradients. Existing single-string primary-biome values continue to work (backward compat).

- **F.5 Editor UI wiring + integration tuning + closeout.** Wire `TerrainPanel::apply_erosion` to invoke the actual erosion pipeline. Final integration tuning across all six climate presets (tropical vs. arid vs. wetland should read as visibly different worlds). Update `ARCHITECTURE_MAP.md` terrain section. Re-mark Phase 1 and Phase 1.5 COMPLETE in the parent campaign.

Each sub-phase's plumbing must be correct and behaviorally verified in isolation before the next sub-phase starts.

---

## 2. Technical architecture

### 2.1 Data flow at the end state

After the full campaign lands, a chunk's generation pipeline is:

```
1. Heightmap generation (CPU, SimdHeightmapGenerator → TerrainNoise)
   Input: world position, chunk seed, BiomeNoisePreset (with noise_type per F.2)
   Output: pre-erosion heightmap Y values (per-vertex f32 grid)

2. Climate sampling (CPU, ClimateMap::sample_chunk)
   Input: world position per vertex, pre-erosion Y (for height lapse rate)
   Output: per-vertex (temperature, moisture) = ClimateSample

3. Per-vertex biome_weights computation (CPU, elevation_to_biome_weights)
   Input: pre-erosion Y, SEA_LEVEL, ClimateBias (from primary_biome string
          OR from ClimateSample for "mixed"/"auto" primary per F.4)
   Output: [f32; 8] biome weights per vertex (slots match Phase 1.5's layout)

4. Erosion simulation (CPU, AdvancedErosionSimulator::apply_preset)
   Input: pre-erosion heightmap + erosion preset (per §2.2 mapping)
   Precondition: heightmap is expanded by halo per §2.3 before this call
   Output: post-erosion heightmap Y values (biome_weights unchanged — held
           stable from step 3 per user precondition)
   Post-step: crop expanded heightmap back to original chunk extent.

5. Vertex buffer assembly (CPU, generate_heightmap_mesh)
   Input: post-erosion Y (for TerrainVertex.position), pre-erosion
          biome_weights (from step 3)
   Output: Vec<TerrainVertex> with post-erosion Y and pre-erosion
           biome_weights per the user's §2 precondition.

6. Splat builder (CPU, terrain_splat_builder) — UNCHANGED from Phase 1.5.
   Input: per-vertex biome_weights (from vertex buffer).
   Output: RGBA8 splat textures for TerrainMaterialManager upload.

7. GPU upload (TerrainMaterialManager::set_chunk_splat_forward) —
   UNCHANGED from Phase 1.
```

**Key code-location invariants the campaign's sub-phases must preserve:**

- **Pre-erosion Y capture location (for biome assignment):** today `generate_heightmap_mesh` at `tools/aw_editor/src/terrain_integration.rs:706-790` reads Y from `chunk.heightmap()` which has already had `chunk.apply_erosion` applied inside `WorldGenerator::generate_chunk` at `astraweave-terrain/src/lib.rs:276-278`. This means the current implementation derives biome_weights from POST-erosion Y — which works today only because the simple CA erosion at `heightmap.rs:199-272` produces near-negligible shape change. Once F.3 wires the real `AdvancedErosionSimulator`, this ordering breaks the user's authorial-intent precondition (§2.5). F.3 must restructure `WorldGenerator::generate_chunk` so biome_weights are computed from the heightmap BEFORE erosion runs, then erosion runs, then the eroded heightmap is returned to the caller alongside the pre-erosion biome_weights. One clean shape: `generate_chunk` returns a `TerrainChunk` whose `.heightmap()` is post-erosion and whose `.biome_weights()` are pre-erosion (new accessor on `TerrainChunk`). An alternative: keep biome-weight computation in `generate_heightmap_mesh` on the editor side, but have the editor call the generator through a two-step API (`generate_chunk_pre_erosion` → compute biome_weights → `apply_erosion_preset_with_halo`). F.3 selects one and records the choice in §10.

- **Erosion location:** today at `astraweave-terrain/src/lib.rs:276-278`. After F.3, the same call site receives a preset + halo-expanded heightmap rather than a single `erosion_strength`.

- **Vertex buffer assembly location:** `generate_heightmap_mesh` at `tools/aw_editor/src/terrain_integration.rs:706-790`. After F.3, this function receives post-erosion Y for the vertex buffer and pre-erosion biome_weights from the generator (or computes them itself from pre-erosion heightmap if the two-step API is chosen). After F.4, it also receives a per-vertex `ClimateSample` for the `elevation_to_biome_weights` call when primary_biome == "mixed"/"auto".

All three locations exist today and form the current call chain; the campaign changes what data flows through them, not the call chain itself.

### 2.2 Preset-to-erosion-preset mapping (design decision — adopted)

Each of Phase 1.5's six `ClimateBias` values maps to one of `AdvancedErosionSimulator`'s named presets. Preset definitions are at `astraweave-terrain/src/advanced_erosion.rs:147-206`.

| ClimateBias | Erosion Preset | Rationale |
|---|---|---|
| Temperate | `ErosionPreset::default()` (hydraulic + thermal, balanced) | Rolling hills with water-carved valleys. `default()` at `advanced_erosion.rs:128-145` uses 50k droplets + 20-iter thermal at 45° talus. |
| Cold | `ErosionPreset::mountain()` | Heavy hydraulic (100k droplets, erode_speed 0.4) + 30-iter thermal at 50° talus. Produces sharp peaks and scree slopes appropriate for tundra highlands. |
| Arid | `ErosionPreset::desert()` | Thermal + wind, no hydraulic. Talus 35° for steeper sand slopes. Produces aeolian features (dunes, wind-streaked ridges). |
| Tropical | `ErosionPreset::coastal()` | Hydraulic (30k droplets, higher evaporation) + thermal (40° talus) + gentle wind. Strong sediment transport; tropical erosion is water-dominated. |
| Wetland | `ErosionPreset::coastal()` | Similar reasoning; sediment deposition characteristic of swamps matches coastal presets' evaporation-heavy hydraulic profile. |
| Highland | `ErosionPreset::mountain()` | Same preset as Cold — heavy hydraulic + aggressive thermal produces the characteristic rocky alpine ridgelines with scree-slope redistribution. |

**F.3 may refine this mapping if initial testing reveals a preset choice doesn't visually match its climate.** Refinements must be logged in §10 with before/after visual evidence. Any preset parameters that need adjustment beyond what the five named presets provide should be expressed as a new named preset inside `advanced_erosion.rs` (not as per-call parameter mutation inside `WorldGenerator`), keeping the preset-is-the-unit-of-configuration discipline.

### 2.3 Chunk-boundary halo strategy (design decision — adopted)

**Adopted: Approach A (halo buffer).** When generating chunk X, `WorldGenerator::generate_chunk` generates the heightmap for an expanded region (X plus a one-chunk halo on each side = 3×3 chunk-widths centered on X), runs `AdvancedErosionSimulator::apply_preset` on the expanded region, then crops back to X's original extent. Adjacent chunks' halos overlap at their shared edge (each sees the other's interior within its halo), so the erosion result at the shared edge converges as long as the simulator is deterministic per-halo-region-and-seed.

**Halo size: 1 chunk on each side.** The initial droplet-travel upper bound for the default preset (50k droplets × up to 30 steps × average per-step displacement ~1-2 heightmap cells) places most water-droplet trajectories within ~50-100 world units, which at the editor's 256-unit chunk_size is a fraction of a chunk-width. A one-chunk halo (256 world units on each side) comfortably exceeds this upper bound and matches the natural generator unit. F.3 confirms this empirically during its integration test (§5.3) and logs any adjustment in §10.

**Performance implication: chunk generation cost scales with halo area.** Halo=1 means the erosion operates on a 3×3 grid around each target chunk (9× the erosion work per chunk vs. no halo). For 121 chunks (radius 5), this multiplies erosion work by 9×, so the editor's chunk-generation time grows proportionally. This is acceptable for editor-time generation and is measured explicitly in F.3's verification pass (§5.3 success criteria).

**Deterministic seeding:** the `AdvancedErosionSimulator::new(seed)` constructor's `seed: u64` parameter must be derived deterministically from the halo region's world position (not from the target chunk's position), so that adjacent halos that overlap produce identical droplet trajectories in the overlap region. A practical scheme: `seed = world_seed.wrapping_add(hash(halo_origin_chunk_id))` where `halo_origin_chunk_id` is the lower-left chunk of the 3×3 halo (i.e. `(x-1, z-1)` for target chunk `(x, z)`). This makes the halo's erosion output a function of `(world_seed, target_chunk_id)` only.

**Fallback if Approach A's cost proves prohibitive:** if F.3 measurement shows chunk-generation time above ~30 seconds for 121 chunks on a midrange CPU (baseline to establish), the campaign's fallback is to lower droplet counts per preset (from 50k/100k to 10k/25k) rather than switch strategies mid-campaign. If even reduced-droplet-count halo erosion is too slow, F.3 logs the finding in §10 and a follow-up session considers Approach C (per-chunk erosion + post-process seam smoothing). **GPU erosion via `astraweave-render::gpu_erosion` is explicitly out of scope** for this campaign (§8) — it is flagged as a post-campaign future project.

### 2.4 Climate as a spatial field (design decision — adopted)

**Current state.** `ClimateMap` at `astraweave-terrain/src/climate.rs` generates per-vertex `(temperature, moisture)` grids with atmospheric lapse rate, latitude gradient, and water-distance falloff. `WorldGenerator::generate_chunk` calls `self.climate.sample_chunk(...)` at `lib.rs:263-267`, feeds it to `self.assign_biomes(&heightmap, &climate_data)` producing a `biome_map`, and the editor at `terrain_integration.rs:344` then overwrites every `biome_map` entry with `primary_biome`. Phase 1.5's `elevation_to_biome_weights` at `astraweave-terrain/src/elevation_biome.rs` uses `ClimateBias::from_primary_biome_str(primary_biome)` — a single enum per project, not per-vertex.

**F.4 design.** `ClimateMap` output becomes a per-vertex input to biome-weight computation. The change is additive:

1. **New factory method:** `ClimateBias::from_climate_field(temperature: f32, moisture: f32) -> ClimateBias`. Maps the (temp, moisture) pair to one of the six existing `ClimateBias` variants using bucket thresholds. Proposed mapping (F.4 may refine with tests):
   - `temp < 0.25` → `Cold` (any moisture)
   - `temp > 0.75 && moisture < 0.25` → `Arid`
   - `temp > 0.6 && moisture > 0.6` → `Tropical`
   - `temp >= 0.35 && temp <= 0.6 && moisture > 0.65` → `Wetland`
   - `temp >= 0.4 && height_implicit_via_caller > 0.7` → `Highland` (F.4 decides how "highland-ness" enters the mapping; one option: pass a third `elevation_percentile` parameter)
   - otherwise → `Temperate`

2. **Extended function signature:** `elevation_to_biome_weights(world_y: f32, sea_level: f32, climate: ClimateBias) -> [f32; 8]` stays as the primary API. F.4 adds a thin wrapper `elevation_to_biome_weights_with_sample(world_y, sea_level, sample: ClimateSample) -> [f32; 8]` where `ClimateSample` holds temperature/moisture/optional-elevation-percentile; the wrapper derives `ClimateBias` via the new factory and delegates. Existing callers (unit tests, Phase 1.5's current wiring) continue to work unchanged; the primary-biome string path continues to use `from_primary_biome_str` verbatim.

3. **Editor UI change:** add `"mixed"`/`"auto"` (F.4 picks one; `"mixed"` favored for clarity) to the primary-biome dropdown at `tools/aw_editor/src/panels/terrain_panel.rs` (location: the dropdown population site for `primary_biome`). When this value is selected:
   - `ClimateBias::from_primary_biome_str("mixed")` returns a new variant `ClimateBias::Mixed` (or the function switches on the `"mixed"` string at its call site — F.4 selects the clean form).
   - `generate_heightmap_mesh` branches: if climate is `Mixed`, call `elevation_to_biome_weights_with_sample` using `ClimateMap::sample_climate(world_x, world_z, height)` at the per-vertex world position; otherwise call the existing string-based path verbatim.
   - The biome-noise preset lookup at `terrain_panel.rs:1816-1991` needs a `"mixed"` entry that blends toward a neutral/Temperate shape (F.4 defines this).

4. **Existing `ClimateMap` calls in `WorldGenerator::generate_chunk` (`lib.rs:263-267`) stay.** Their output is currently discarded by the editor; F.4 stops the discarding only on the `"mixed"` path. For non-`"mixed"` primary-biome values, the existing overwrite at `terrain_integration.rs:344` continues (the single-string climate dominates). This keeps all existing behavior bit-identical for all current projects.

5. **Edge cases:** `"mixed"` + erosion preset selection. If the primary-biome is `"mixed"`, the whole chunk's erosion preset selection can't come from the primary-biome string. F.4 resolves this by selecting an erosion preset per chunk based on the chunk-center's sampled `ClimateBias` (one `apply_preset` call per chunk, using the chunk-center climate). This is a documented approximation — it produces visible transitions between erosion styles at chunk boundaries under mixed climates, but the halo strategy from §2.3 keeps the transitions coherent within any one chunk's erosion run.

### 2.5 Biome-weight stability under erosion (user precondition, documented here)

**Decision:** biome_weights are computed from pre-erosion Y and held stable through erosion. A vertex whose Y drops from 50 to 30 during erosion keeps its Mountain biome_weight profile even though post-erosion geometry would geologically classify it as a hill.

**Why:** authorial intent. If a world author paints "this region is a mountain range" via primary-biome selection or a future paint tool, erosion should shape the geometry of that region without reclassifying it as a lowland biome. Phase 1.5's `elevation_biome` bands treat post-erosion Y as the reclassification input would be trivially straightforward but would produce authorially surprising results: every freshly-eroded Mountain becomes a Forest band, every wave-smoothed Coast becomes a Beach band. The stable-under-erosion decision matches the parent campaign's Phase 2 direction (per-vertex painted material IDs are also held stable through splat sampling) and aligns with how Unreal Landscape and Frostbite treat painted layer weights.

**Consequences:**

- The final rendered terrain may have visual inconsistency zones — e.g., a gently sloped hillside rendered with Mountain textures — but only where erosion significantly modified terrain that was already near a band boundary. In practice this is authorially beneficial (painted-biome stability) and geologically plausible (real mountains have weathered foothills that still read as "mountain range" at the landscape scale).
- The Phase 1.5 `elevation_biome.rs` module stays unchanged. This campaign does not modify biome-assignment algorithms; it modifies the terrain those assignments run against.
- F.3 is responsible for restructuring `WorldGenerator::generate_chunk` (or introducing a two-step API) so the pre-erosion-Y-for-biome-assignment invariant is enforced, as §2.1 describes.

### 2.6 Continental-scale macro-feature (design decision — adopted 2026-04-21)

**Problem:** F.1's amplitude tuning produced dramatic per-vertex elevation, but the mountain-feature wavelength (~400 world units) is much shorter than the terrain's visible extent (~2800 world units at radius 5). Every local peak reaches Mountain elevation and gets the full Beach→Grassland→Forest→Mountain biome progression on its slopes, producing dozens of visible repetitions of the sequence across a single aerial view. This reads as a repeating striped pattern rather than as a coherent landscape.

**Design decision:** Add a continental-scale noise field that spatially modulates `mountains_amplitude` across the world. The field is a single low-frequency noise octave (wavelength ≈ world extent) whose output ∈ [0, 1] multiplies each vertex's effective `mountains_amplitude`. Regions where the continental field is high receive full mountain amplitude (mountain country); regions where it is low receive greatly reduced mountain amplitude (lowlands, rolling hills). This breaks the uniform-distribution-of-peaks pattern and establishes regional geographic structure — a foundation that F.3's erosion and F.4's climate field build on.

**Implementation location:** `TerrainNoise::sample_height` in `astraweave-terrain/src/noise_gen.rs:316-353`. Before the mountain layer is accumulated into the output, sample the continental field at `(x, z)` and multiply the mountain contribution by `mix(continental_min, 1.0, continental_sample)`, where `continental_min` is the minimum mountain amplitude multiplier (so even "lowlands" regions have some mountain-ish micro-features, just much reduced).

**Config shape:**
- `NoiseConfig.continental_scale: f32` — frequency of the continental noise (default: 0.0004, giving a wavelength of ~2500 world units, approximately matching the radius-5 terrain extent).
- `NoiseConfig.continental_min: f32` — minimum mountain-amplitude multiplier where continental noise is at its minimum (default: 0.15, so "lowlands" have 15% of full mountain amplitude — subtle topography, not flat).
- `NoiseConfig.continental_seed_offset: u32` — offset from the world seed for continental noise determinism (default: 7; plain Perlin, not DomainWarped, since the continental feature is meant to be smooth).
- `NoiseConfig.continental_enabled: bool` — whether the active configuration applies continental modulation. Default: false (backward-compat — F.1 / pre-F.2 configs produce unchanged output).

**Per-preset opt-in:** Each `BiomeNoisePreset` carries a new boolean field `continental_modulation: bool`. Presets that should show regional clustering (grassland, mountain, forest, tundra, desert) set this to `true`; presets for inherently gentle terrain (swamp, beach, river) set it to `false` — their mountain amplitude is already small enough that continental modulation would produce no visible effect. `apply_biome_noise_preset` propagates the preset's `continental_modulation` to `NoiseConfig.continental_enabled`.

**Determinism:** `seed_continental = world_seed.wrapping_add(continental_seed_offset as u64)`. The continental field is purely a function of `(world_seed, world_x, world_z)` — no chunk state, no per-chunk caching, no boundary concerns.

**Interaction with F.3 (forward reference):** when F.3 wires AdvancedErosionSimulator, the continental field's regional variation will naturally produce more dramatic erosion in high-amplitude regions (because there's more relief to erode) and subtler erosion in low-amplitude regions (flatter terrain, less sediment transport). This is geologically correct; F.3 does not need to do anything special to get this behavior — it emerges from the continental field + erosion preset acting on heightmaps with pre-existing regional variation.

**Interaction with F.4 (forward reference):** climate's spatial variation (temperature, moisture) is mostly orthogonal to the continental field (climate follows latitude, altitude, water-distance; continental feature follows its own low-frequency noise). But they interact positively: regions where continental is high (mountain country) tend to have lower temperature (altitude), which F.4's climate field naturally captures. The two systems compose.

**Isotropy:** The continental field is isotropic in F.2. Adding directional bias (e.g., the NC southwest-northeast axis) is deferred to F.5 integration tuning or a follow-up pass.

---

## 3. Sub-phase F.1 — Amplitude tuning

### 3.1 Goal

Tune the eight biome-noise presets at `tools/aw_editor/src/panels/terrain_panel.rs:1816-1991` so each produces a runtime Y span matching Phase 1.5's elevation bands. After F.1, the editor's grassland terrain produces Y span ≥ 100 units, Forest and Mountain biomes render visibly in the default project, and the parent campaign's Issue 2 is resolved at the data level.

### 3.2 Scope

**In scope:**

- Adjust `base_amplitude`, `mountains_amplitude`, `detail_amplitude` (and related octaves/scale/persistence/lacunarity parameters if needed) for the eight presets defined at `terrain_panel.rs:1861-1989`: `mountain`, `desert`, `forest`, `tundra`, `swamp`, `beach`, `river`, grassland/default (catch-all `_ =>`).
- Verify runtime Y spans via a diagnostic test that drives `TerrainState::configure + state.set_noise_params + state.apply_biome_noise_preset + state.generate_terrain(5)` (the exact call chain from `terrain_panel.rs::regenerate_terrain` so the measurement reflects actual editor output). Reuse the pattern from Phase 1.5-T's `tools/aw_editor/tests/phase_1_5_heightmap_diagnostic.rs`. Test lands and removes in the same sub-phase (F.1.A or F.1.C) — do not land temporary test infrastructure permanently.
- Add a correction note to `docs/audits/phase_1_5_tuning_investigation_2026-04-20.md` referencing this campaign's findings (audit incidental #6).

**Out of scope:**

- DomainWarped changes (F.2).
- Erosion wiring (F.3).
- Climate rewiring (F.4).
- Editor UI changes (F.5).
- Any changes to `BiomeNoisePreset` struct layout — F.1 only changes constant values within the existing struct shape.

### 3.3 Success criteria

- Launching editor with seed 12345 + grassland primary biome shows Beach/Grassland/Forest/Mountain bands all visibly rendering across elevation bands. (Interactive visual verification — Andrew's gate.)
- Measured runtime Y span for grassland ≥ 100 units. (Code-level measurement via diagnostic test.)
- All five other presets (Cold/Arid/Tropical/Wetland/Highland, plus `beach`/`river`/`swamp`/`desert`/`forest`/`tundra`/`mountain` as appropriate mapped presets) produce appropriately varied Y spans that match their climate's implied terrain character (Highland/Mountain dramatic, Wetland/Tropical gentler but not golf-course). Minimum guideline: each preset's measured span ≥ 60 units; presets named after high-relief biomes (`mountain`, `tundra`, `highland` where applicable) ≥ 150 units.
- All three `cargo check` invocations pass (all-features, default, postfx+textures fallback).
- All existing tests pass.
- `docs/audits/phase_1_5_tuning_investigation_2026-04-20.md` has a correction note referencing audit §6 and this campaign's F.1 measurements.

### 3.4 Reversibility

Each commit is a small per-preset constant adjustment. Revert = `git revert` the commit. The generator code path is unchanged; only preset constants shift. Phase 1.5's elevation bands are unchanged by F.1.

### 3.5 Expected commits

- **F.1.A — Grassland preset amplitude tuning.** Tune the `_ => ...` fallback preset at `terrain_panel.rs:1973-1989` to produce runtime Y span ≥ 100 units. Land temporary diagnostic test that measures the span. Commit message: `Phase 1.6-F.1.A: tune grassland preset to 100+ unit Y span`.
- **F.1.B — Remaining preset amplitude tuning.** Tune the seven other presets. Same test extended to cover all presets. Commit message: `Phase 1.6-F.1.B: tune remaining seven biome-noise presets`.
- **F.1.C — Closeout.** Remove diagnostic test. Add correction note to Phase 1.5-T investigation doc. Update this plan's §9 to mark F.1 COMPLETE. Commit message: `Phase 1.6-F.1.C: close F.1, Phase 1.5-T correction note`.

---

## 4. Sub-phase F.2 — DomainWarped noise integration + continental-scale macro-feature

### 4.1 Goal

Extend `BiomeNoisePreset` at `tools/aw_editor/src/terrain_integration.rs:27-47` with per-layer `NoiseType` selection AND with a `continental_modulation` opt-in for continental-scale mountain-amplitude modulation (new design decision per §2.6). Enable `DomainWarpedNoise` for five presets (grassland, mountain, forest, tundra, desert) base-elevation layers. Implement continental-scale macro-feature in `TerrainNoise::sample_height`. After F.2, the five presets produce (a) organic macro-features within their local noise, and (b) visible regional clustering of mountain zones vs. lowland zones, breaking the repetition pattern observed after F.1.

### 4.2 Scope

**In scope:**

- Extend `BiomeNoisePreset` struct with new fields: `base_noise_type: NoiseType`, optional `base_domain_warp: Option<DomainWarpConfig>`, and `continental_modulation: bool`.
- Extend `NoiseConfig` with `continental_scale`, `continental_min`, `continental_seed_offset`, and `continental_enabled` per §2.6 with the specified defaults.
- Extend `apply_biome_noise_preset` at `terrain_integration.rs:166-190` to apply the new fields: set `self.config.noise.base_elevation.noise_type`, `self.config.noise.base_elevation.domain_warp`, and `self.config.noise.continental_enabled` from the preset.
- Extend `TerrainNoise::sample_height` to sample the continental noise and multiply the mountain layer's contribution per §2.6.
- Update all eight preset definitions at `terrain_panel.rs:1861-1989`:
  - grassland/default: `NoiseType::DomainWarped` with `iterations: 2, warp_strength: 40, warp_octaves: 3, warp_scale: 1.5`; `continental_modulation: true`.
  - mountain: `NoiseType::DomainWarped` with `iterations: 2, warp_strength: 60, warp_octaves: 3, warp_scale: 2.0`; `continental_modulation: true`.
  - forest: `NoiseType::DomainWarped` with `iterations: 2, warp_strength: 35, warp_octaves: 3, warp_scale: 1.2`; `continental_modulation: true`.
  - tundra: `NoiseType::DomainWarped` with `iterations: 2, warp_strength: 50, warp_octaves: 3, warp_scale: 1.7`; `continental_modulation: true`.
  - desert: `NoiseType::DomainWarped` with `iterations: 2, warp_strength: 45, warp_octaves: 3, warp_scale: 1.6`; `continental_modulation: true`.
  - swamp, beach, river: stay on `NoiseType::Perlin`; `continental_modulation: false`. Their mountain amplitudes are already small enough that continental modulation would produce no visible effect.
- Add a unit test inside `terrain_integration.rs` (or a sibling `tests/` module) asserting that after `apply_biome_noise_preset`, the generator's `NoiseConfig` has the preset's `base_noise_type`, `base_domain_warp`, and `continental_enabled` values.
- Add a diagnostic test that samples two configured `TerrainNoise` instances at identical world positions — one with DomainWarped, one with plain Perlin — and confirms the outputs differ (sanity-check that DomainWarped is being applied to the layer).
- Add a diagnostic test that samples the continental field at a grid of world positions and confirms its output range is [0, 1] with meaningful variation (not constant).
- Measure and record chunk-generation-time delta from F.1 to F.2 (DomainWarped is iterative so it is slower than plain Perlin; continental field adds one extra sample per vertex).

**Qualitative success criterion (Andrew-gate):** aerial view at seed 12345 grassland shows distinct lowland zones and distinct highland zones rather than uniformly-distributed peaks. This is F.2's core visual gate.

**Out of scope:**

- Anisotropic / directional bias in domain warping (the NC southwest-northeast axis) — F.5 or follow-up.
- Erosion wiring (F.3).
- Climate rewiring (F.4).
- Any changes to `DomainWarpedNoise` itself at `noise_gen.rs:154-211`.
- Enabling DomainWarped on non-base layers (mountains, detail) unless F.2.A design review decides otherwise.
- Any F.1 preset amplitude changes — F.1's tuning preserved byte-for-byte.

### 4.3 Success criteria

- `BiomeNoisePreset` struct extended with `base_noise_type`, optional `base_domain_warp`, and `continental_modulation` fields.
- `NoiseConfig` extended with `continental_scale`, `continental_min`, `continental_seed_offset`, `continental_enabled` fields.
- `apply_biome_noise_preset` applies all new fields.
- Five presets (grassland, mountain, forest, tundra, desert) use `NoiseType::DomainWarped` for base elevation; three presets (swamp, beach, river) stay on `NoiseType::Perlin`.
- All five DomainWarped presets have `continental_modulation = true`; all three plain-Perlin presets have `continental_modulation = false`.
- `TerrainNoise::sample_height` samples the continental field and modulates the mountain layer accordingly when `continental_enabled` is true.
- **Qualitative visual gate (Andrew's interactive verification):** seed 12345 grassland aerial view shows distinct lowland zones (regions where mountain-scale peaks are absent or much-reduced) and distinct highland zones (regions where peaks concentrate). The uniform-repetition pattern of the F.1 stills is broken. This is the core F.2 gate — without it, F.2 has not delivered.
- **Qualitative visual gate (Andrew's interactive verification):** visible organic macro-features in grassland and mountain terrain — meandering ridges, irregular valleys, curved rather than axis-aligned features.
- Performance: chunk-generation time for 121 chunks stays ≤ 2× F.1's baseline (measured and documented in F.2.D's commit message). If the delta exceeds 2×, F.2.D reduces DomainWarp iteration count (from 2 to 1) on the most expensive presets before declaring complete.
- All three `cargo check` invocations pass.
- All tests pass, including the new F.2 unit and diagnostic tests.
- This plan's §9 reflects F.2 COMPLETE.

### 4.4 Reversibility

F.2.A (struct extension) can be reverted in isolation; F.2.B (preset DomainWarped + continental activation) reverts to the F.1-tuned preset constants; F.2.C (continental implementation) can be reverted independently since `continental_enabled: false` is the default and makes the code path a no-op. Full revert = `git revert` F.2.A through F.2.D in reverse order; F.1 state is restored.

Continental feature is purely additive to `NoiseConfig` / `TerrainNoise`; reverting the `continental_enabled` flag in presets is sufficient to disable it without removing code. Full revert of F.2.A-F.2.D restores F.1 state.

### 4.5 Expected commits

- **F.2.A — `BiomeNoisePreset` + `NoiseConfig` API extension.** Add `base_noise_type`, optional `base_domain_warp`, `continental_modulation` fields to `BiomeNoisePreset`. Add `continental_scale`, `continental_min`, `continental_seed_offset`, `continental_enabled` fields to `NoiseConfig`. Update `apply_biome_noise_preset` to apply them. All existing preset definitions get `base_noise_type: NoiseType::Perlin` and `continental_modulation: false` to preserve F.1 behavior (struct-extension-only commit; no behavior change). Commit message: `Phase 1.6-F.2.A: extend BiomeNoisePreset + NoiseConfig with noise_type and continental fields`.
- **F.2.B — Enable DomainWarped + continental modulation on five presets.** Change grassland, mountain, forest, tundra, desert preset definitions to `NoiseType::DomainWarped` with tuned `DomainWarpConfig`. `continental_modulation: true` for the same five. Plain Perlin + `continental_modulation: false` for swamp, beach, river. Commit message: `Phase 1.6-F.2.B: enable DomainWarpedNoise + continental modulation for five presets`.
- **F.2.C — Continental-scale macro-feature implementation.** Extend `TerrainNoise::sample_height` to sample a continental noise field and modulate the mountain layer per §2.6. Add the continental-output-range and DomainWarped-differs-from-Perlin diagnostic tests. Commit message: `Phase 1.6-F.2.C: continental-scale mountain-amplitude modulation in TerrainNoise`.
- **F.2.D — Closeout.** Record performance measurements (F.1 baseline vs. F.2 chunk-generation time for 121 chunks). Update this plan's §9 to mark F.2 COMPLETE. Remove any temporary diagnostics (keep the permanent unit tests from F.2.A-F.2.C). Commit message: `Phase 1.6-F.2.D: close F.2`.

---

## 5. Sub-phase F.3 — AdvancedErosionSimulator wiring with halo

### 5.1 Goal

Wire `AdvancedErosionSimulator::apply_preset` into terrain chunk generation. Implement the halo strategy (§2.3 Approach A) for chunk-boundary continuity. Apply the per-climate erosion preset mapping from §2.2. Restructure the generator call path to enforce the biome-weights-from-pre-erosion-Y invariant (§2.5).

### 5.2 Scope

**In scope:**

- Restructure `WorldGenerator::generate_chunk` at `astraweave-terrain/src/lib.rs:243-282` so biome-weight computation runs BEFORE erosion. Pick one of the two shapes from §2.1 and log the choice in §10:
  - Shape A (generator-side): `TerrainChunk` grows a `biome_weights: Option<Vec<[f32; 8]>>` field populated pre-erosion; `generate_heightmap_mesh` on the editor side reads from that field instead of computing biome_weights itself.
  - Shape B (editor-side two-step): `WorldGenerator` exposes `generate_chunk_pre_erosion(chunk_id) -> TerrainChunk` and `apply_erosion_preset_with_halo(&mut chunk, preset, halo_chunks)` as separate calls; the editor's `generate_terrain` calls the first, computes biome_weights from the pre-erosion heightmap inside its own loop, then calls the second.
  - Prefer Shape A unless F.3 investigation reveals it would require disruptive changes to `TerrainChunk`. Shape A is clean and keeps the halo logic internal to the generator.
- Replace `chunk.apply_erosion(erosion_strength)` at `lib.rs:276-278` with `AdvancedErosionSimulator::apply_preset(&mut halo_heightmap, preset) -> ErosionStats`, where `halo_heightmap` is the 3×3-chunk expanded region generated inside the same function per §2.3.
- Implement halo expansion: when generating chunk X, generate the heightmap over X + 1-chunk border (nine sub-chunks' worth of vertex samples assembled into one larger heightmap), pass that to the simulator, then extract the center third back into the chunk's heightmap.
- Derive the simulator seed deterministically from `(world_seed, halo_origin_chunk_id)` per §2.3 so adjacent halos produce matching trajectories in their overlap.
- Implement preset-to-climate mapping per §2.2 as a lookup function `erosion_preset_for_climate(climate: ClimateBias) -> ErosionPreset`. Placement: new helper in `astraweave-terrain/src/advanced_erosion.rs` or a new `astraweave-terrain/src/erosion_selection.rs` module.
- Measure erosion cost per chunk; verify total generation time for 121 chunks stays within the performance envelope (F.3's initial target: ≤ 30 seconds on a midrange CPU; adjust if audit measurements motivate a different number). If the envelope is exceeded, apply the droplet-count fallback from §2.3.
- Write integration test `tests/phase_1_6_f3_chunk_boundary_continuity.rs` that generates two adjacent chunks and verifies their shared-edge Y values match to within a small tolerance (e.g., ≤ 0.01 world units).

**Out of scope:**

- GPU erosion (`gpu_erosion.rs`). Flagged as post-campaign per §8.
- Editor UI wiring for "Apply Erosion" button (deferred to F.5).
- Climate rewiring (F.4).
- Changes to `AdvancedErosionSimulator` internals at `advanced_erosion.rs`.
- Changes to the five named `ErosionPreset` methods (`default`, `desert`, `mountain`, `coastal`) unless F.3's visual validation shows a specific preset needs adjustment — in which case the adjustment is a new named preset, not a mutation of existing ones.

### 5.3 Success criteria

- `AdvancedErosionSimulator::apply_preset` runs during chunk generation (verified by a log line or a test counter that increments on each invocation; test removes at F.3 closeout).
- Chunk-boundary halo strategy produces no visible seams at chunk edges. Automated verification: `phase_1_6_f3_chunk_boundary_continuity.rs` integration test passes. Visual verification: Andrew's interactive gate — overhead view of editor terrain shows no grid-aligned seam lines at chunk boundaries (this is the same visual artifact the parent campaign's Issue 1 fix at `983b61a16` addressed for splat textures; F.3 must not regress that fix and must also not introduce its equivalent in heightmap Y).
- Erosion produces characteristic features: water-carved channels, alluvial deposits, talus slopes at climate-appropriate slopes (Andrew's visual gate; Mountain preset shows scree slopes, Coastal shows sediment deposition, Desert shows wind-streaked surfaces).
- Biome weights reflect pre-erosion terrain. Verified by a diagnostic test that generates a chunk, captures biome_weights, then captures the post-erosion heightmap's band-classification of the same vertex positions, and confirms the biome_weights correspond to pre-erosion Y bands even where post-erosion Y would classify differently.
- Terrain generation time for 121 chunks ≤ 30 seconds on Andrew's reference CPU (baseline to establish; adjust in §10 if a measurement during F.3 motivates a different threshold).
- All three `cargo check` invocations pass.
- All existing tests pass. New F.3 tests (chunk-boundary continuity, biome-weights-from-pre-erosion-Y, optional invocation-counter) pass.

### 5.4 Reversibility

F.3 is the largest sub-phase. Each F.3 sub-commit is small and revertable independently. Full F.3 revert = `git revert` of F.3.A through F.3.E in reverse order; the system reverts to F.2's state (DomainWarped enabled, simple CA erosion still running).

### 5.5 Expected commits

- **F.3.A — Biome-assignment-before-erosion ordering.** Introduce Shape A or Shape B per §2.1's selection. If Shape A: add `biome_weights` field to `TerrainChunk`, populate it from `elevation_to_biome_weights` inside `generate_chunk` before the erosion call site, update `generate_heightmap_mesh` on the editor side to read from it. Simple CA erosion still running at this point — this commit is pure restructure, no behavioral change. Verification: an integration test confirms that when simple CA erosion is bypassed, the biome_weights exactly match the equivalent of the pre-F.3 computation. Commit message: `Phase 1.6-F.3.A: capture biome_weights pre-erosion`.
- **F.3.B — Halo expansion logic.** Add a private `generate_halo_heightmap(target_chunk, halo_chunks)` helper on `WorldGenerator` that returns a heightmap covering a 3×3 chunk region. Replace the existing single-chunk heightmap generation in `generate_chunk` with a halo generation + center crop. Simple CA erosion still running. Visual check: editor terrain looks identical to F.3.A because halo + crop is a no-op when the erosion doesn't cross chunk boundaries. Commit message: `Phase 1.6-F.3.B: halo expansion scaffolding`.
- **F.3.C — AdvancedErosionSimulator wired with preset mapping.** Replace simple CA `chunk.apply_erosion(strength)` with `AdvancedErosionSimulator::apply_preset`. Add `erosion_preset_for_climate`. Derive deterministic halo seed. Commit message: `Phase 1.6-F.3.C: wire AdvancedErosionSimulator with per-climate presets`.
- **F.3.D — Chunk-boundary continuity integration test.** Add `tests/phase_1_6_f3_chunk_boundary_continuity.rs` with two-adjacent-chunks Y-match assertion. Add the biome-weights-from-pre-erosion-Y diagnostic test. Commit message: `Phase 1.6-F.3.D: chunk-boundary continuity tests`.
- **F.3.E — Closeout.** Remove invocation-counter diagnostic if used. Record performance measurements. Update this plan's §9 to mark F.3 COMPLETE. Commit message: `Phase 1.6-F.3.E: close F.3`.

---

## 6. Sub-phase F.4 — Climate as spatial field

### 6.1 Goal

Rewire `ClimateMap` so its output reaches `elevation_to_biome_weights` as a per-vertex `ClimateSample`. Introduce `"mixed"` primary-biome option in the editor UI that enables climate-driven biome assignment. Existing single-string primary-biome options continue to work unchanged (backward compat).

### 6.2 Scope

**In scope:**

- Add `ClimateSample { temperature: f32, moisture: f32 }` struct to `astraweave-terrain/src/elevation_biome.rs` (or `climate.rs`, whichever location best matches the crate's style).
- Add factory method `ClimateBias::from_climate_field(sample: ClimateSample) -> ClimateBias` using the bucket mapping from §2.4.
- Add wrapper `elevation_to_biome_weights_with_sample(world_y: f32, sea_level: f32, sample: ClimateSample) -> [f32; 8]` that derives a `ClimateBias` from the sample and delegates to the existing function.
- Update `tools/aw_editor/src/terrain_integration.rs::generate_heightmap_mesh` at lines 706-790: when `primary_biome` string is `"mixed"`, per-vertex compute `ClimateSample` via `WorldGenerator::climate().sample_climate(world_x, world_z, height)` (requires exposing the climate map from `WorldGenerator` — one-line accessor), call `elevation_to_biome_weights_with_sample` instead of the string-based path.
- Add `"mixed"` to the editor's primary-biome dropdown in `tools/aw_editor/src/panels/terrain_panel.rs`. Add a matching `"mixed"` branch in `noise_preset_for_biome` at `terrain_panel.rs:1816` — starting configuration: use the `_ =>` grassland preset with `base_noise_type: NoiseType::DomainWarped` (so mixed climates benefit from F.2's DomainWarped macro-features even though no single climate drives amplitude).
- `"mixed"` erosion preset selection: apply `erosion_preset_for_climate` to the chunk-center sampled `ClimateBias` (one preset per chunk, based on chunk-center climate). This is a documented approximation per §2.4 step 5.
- Preserve existing single-string primary-biome behavior for all current values (Temperate/Cold/Arid/Tropical/Wetland/Highland via `beach/river/grassland/tundra/desert/forest/swamp/mountain`). Verified via regression test that for each existing primary-biome string, the biome_weights at seed 12345 match the pre-F.4 output byte-for-byte.
- Write unit tests for `ClimateBias::from_climate_field` covering each mapping bucket.
- Write integration test that verifies `"mixed"` primary produces a measurably different biome distribution from any single-climate primary (e.g., `"mixed"` produces > 3 dominant biomes per-chunk; `"grassland"` produces ≤ 2).

**Out of scope:**

- Removing the discarded `ClimateMap` code path at `lib.rs:263-267` — it's now used, not discarded.
- Any changes to biome definitions, climate noise parameters in `ClimateConfig::default`, or `ClimateMap` internals.
- Changes to `assign_biomes` at `lib.rs:320-334` (still used for the `biome_map` that feeds splat-rule selection in F.2's material-ID path).
- New climate gradient models (rain-shadow, wind-patterns, etc.) beyond what `ClimateMap` already provides.

### 6.3 Success criteria

- Setting primary-biome = `"mixed"` produces terrain where biomes vary naturally across latitude/longitude/elevation per `ClimateMap` output. Verified: integration test shows > 3 dominant-biome slots across a 121-chunk seed-12345 mixed project.
- All existing primary-biome values continue to produce byte-identical biome-weight output at seed 12345 as before F.4 (regression guard).
- `cargo check` + tests pass.
- Visual verification (Andrew's gate): `"mixed"` primary on seed 12345 shows visibly smooth climate-driven biome transitions across the 11×11 chunk grid.
- This plan's §9 reflects F.4 COMPLETE.

### 6.4 Reversibility

F.4 is additive — all existing behavior preserved, new functionality added. Revert via `git revert` of F.4.A–F.4.D in reverse order restores pre-F.4 behavior exactly.

### 6.5 Expected commits

- **F.4.A — `ClimateSample` + `from_climate_field` + `with_sample` wrapper.** New types and functions in `elevation_biome.rs`. Unit tests for each bucket. No editor-side changes yet. Commit message: `Phase 1.6-F.4.A: add ClimateSample + per-sample biome-weight API`.
- **F.4.B — Editor rewiring for `"mixed"` primary.** Update `generate_heightmap_mesh` to branch on `"mixed"`. Expose `climate()` accessor on `WorldGenerator`. Integration test for mixed-vs-single-climate distribution difference. Commit message: `Phase 1.6-F.4.B: wire ClimateMap output into per-vertex biome weights for mixed primary`.
- **F.4.C — Editor UI `"mixed"` option.** Add dropdown entry. Add `"mixed"` preset in `noise_preset_for_biome`. Commit message: `Phase 1.6-F.4.C: add "mixed" primary-biome editor option`.
- **F.4.D — Closeout.** Regression test for byte-identical existing-behavior. Update this plan's §9 to mark F.4 COMPLETE. Commit message: `Phase 1.6-F.4.D: close F.4`.

---

## 7. Sub-phase F.5 — Editor UI wiring + integration tuning + closeout

### 7.1 Goal

Wire the editor's `TerrainPanel::apply_erosion` action handler so the "Apply Erosion" button invokes the real erosion pipeline. Tune all eight `BiomeNoisePreset` configurations end-to-end so each produces distinctive terrain. Update `ARCHITECTURE_MAP.md` terrain section. Re-mark parent campaign's Phase 1 and Phase 1.5 COMPLETE.

### 7.2 Scope

**In scope:**

- Replace the stub at `tools/aw_editor/src/panels/terrain_panel.rs:1707-1730` with a real erosion invocation that triggers re-running the chunk-generation path (which now includes `AdvancedErosionSimulator` per F.3) for the loaded terrain and queues GPU re-upload.
- Decide: does "Apply Erosion" regenerate from scratch (simple, slow) or apply erosion to existing chunks in place (fast, requires the stateful generator to support mid-lifecycle erosion)? Default: regenerate from scratch; the loaded seed + preset + erosion-preset combination determines the output deterministically. F.5 logs the decision in §10 if the in-place option is chosen.
- End-to-end integration tuning pass across all eight presets plus `"mixed"`. Tropical vs. arid vs. wetland should read as visibly different worlds, not just different colors. Tuning touches preset `base_noise_type`/`DomainWarpConfig`/amplitudes and the §2.2 erosion-preset mapping. Record any mapping adjustments in §10.
- Update `docs/current/ARCHITECTURE_MAP.md` terrain section to describe the new generation pipeline (halo-based erosion, climate-as-spatial-field, preset-driven DomainWarp). May defer to parent campaign's Phase 3 closeout if that closeout lands first — F.5 picks whichever order is natural and logs the choice.
- Update parent campaign `docs/current/TERRAIN_MATERIAL_SYSTEM_CAMPAIGN.md` §7 to re-mark Phase 1 and Phase 1.5 COMPLETE. Update parent campaign's §9 with a close-out entry referencing this campaign's final commit.
- Remove any temporary diagnostic tests added during F.1–F.4.

**Out of scope:**

- Any new editor UI (parameter sliders, advanced controls) beyond wiring the existing "Apply Erosion" button.
- Changes to the editor-side `HydraulicErosionParams`/`ThermalErosionParams`/`WindErosionParams` duplication from audit incidental #4 — flagged for a future cleanup pass.
- Phase 2 or Phase 3 parent-campaign work.
- GPU erosion (`gpu_erosion.rs`).

### 7.3 Success criteria

- Clicking "Apply Erosion" in the editor actually modifies terrain (Andrew's gate: before/after visual difference visible).
- Eight climate presets plus `"mixed"` produce visually distinct terrain with appropriate character. Tropical presets show lush water-carved valleys; arid presets show wind-streaked dunes; wetland presets show gentle deposition terrain; highland presets show dramatic talus slopes. (Andrew's gate: side-by-side screenshots from eight seeds × eight presets pass a qualitative "these look like different climates" review.)
- Parent campaign's §7 reflects Phase 1 + Phase 1.5 COMPLETE.
- `ARCHITECTURE_MAP.md` terrain section updated (or deferred per the choice above, with a commit-log reference either way).
- `cargo check` + all tests pass.
- This plan's §9 reflects F.5 COMPLETE and campaign complete.

### 7.4 Reversibility

F.5 closeout changes touch documentation, the `apply_erosion` handler, and preset tuning values. Revert of documentation + handler reverts cleanly. Revert of preset tuning values reverts to the F.1-through-F.4-tuned state (which is still a functional improvement over pre-campaign). Re-marking COMPLETE is conservative: if the parent campaign's Phase 1 or 1.5 surface a fresh regression after F.5 lands, a revert of the §7 update restores the LANDED-with-known-regressions status per the parent campaign's §0 discipline.

### 7.5 Expected commits

- **F.5.A — `apply_erosion` handler wired.** Replace stub at `terrain_panel.rs:1707-1730`. Commit message: `Phase 1.6-F.5.A: wire apply_erosion handler`.
- **F.5.B — Integration tuning pass.** End-to-end preset tuning across all climates. Commit message: `Phase 1.6-F.5.B: integration tuning across eight climate presets`.
- **F.5.C — Documentation updates.** `ARCHITECTURE_MAP.md` terrain section. Parent campaign §7 (Phase 1 + 1.5 re-mark COMPLETE) + parent campaign §9 close-out entry. Commit message: `Phase 1.6-F.5.C: update architecture map + parent campaign status`.
- **F.5.D — Closeout.** Remove remaining diagnostic tests. Update this plan's §9 to mark F.5 COMPLETE + campaign complete. Update this plan's header Status line. Commit message: `Phase 1.6-F.5.D: close campaign`.

---

## 8. Out of scope for entire campaign

- **GPU erosion path (`astraweave-render/src/gpu_erosion.rs`).** Complete-but-disconnected per audit §4 and incidental #8. Flagged as post-campaign future project if F.3 performance measurements expose CPU erosion as a bottleneck.
- **Changes to the parent campaign's Phase 2 or Phase 3 scope.** This campaign is intercalated between Phase 1 cleanup and Phase 2; Phase 2 proceeds after F.5 completes.
- **New biome types beyond the existing 8** (Grassland, Desert, Forest, Mountain, Tundra, Swamp, Beach, River). F.4's `"mixed"` is a climate selector, not a new biome slot.
- **Texture quality improvements, normal-map work, or material-array extensions.** Covered by parent campaign Phase 2/3.
- **Water system changes.** Out of scope per `docs/audits/water_system_architecture_2026-04-20.md`.
- **`TerrainPanel`'s duplicated erosion parameter structs (`HydraulicErosionParams` etc.).** Audit incidental #4; flagged for a future cleanup pass that replaces them with direct use of `advanced_erosion::HydraulicErosionConfig` etc.
- **Removal of the superseded `BiomeBlender` at `astraweave-terrain/src/biome_blending.rs`.** Kept on disk as reference; not touched by this campaign.
- **Removal of the discarded `assign_biomes` call in `WorldGenerator::generate_chunk`.** Still feeds `biome_map` for splat-rule selection; F.4 uses climate data separately without disturbing this existing usage.
- **Any work the audit explicitly flagged as out-of-scope** (e.g., `terrain_modifier.rs` voxel paint integration).

---

## 9. Phase status

This section must be updated in the same commit that completes each sub-phase per §0 discipline.

```
F.0 — Draft campaign plan: COMPLETE 2026-04-21, commit 0bf337caf.
F.1 — Amplitude tuning: COMPLETE 2026-04-21, commits fff581aa4 (F.1.A) + a05b856d8 (F.1.B) + c76179bdd (F.1.C).
F.2 — DomainWarped noise integration + continental-scale macro-feature: COMPLETE 2026-04-21, commits ed65a1fc7 (plan amend) + a4b76fb1e (F.2.A) + 1cda72d8c (F.2.B) + 95a50f4c7 (F.2.C) + 566cdb323 (F.2.D).
F.3 — AdvancedErosionSimulator wiring with halo: NOT STARTED
F.4 — Climate as spatial field: NOT STARTED
F.5 — Editor UI wiring + integration tuning + closeout: NOT STARTED
```

Format for completion updates: `F.N — <title>: COMPLETE <YYYY-MM-DD>, commit <hash>`

---

## 10. Deviations log

This section records any design decisions made during execution that deviate from this plan. Every deviation must be recorded here before or in the same commit as the deviation itself.

Format for entries:

```
### <YYYY-MM-DD>, Sub-phase <F.N>, commit <hash>
**Deviation:** <short description>
**Rationale:** <why>
**Impact:** <what parts of later sub-phases or other systems are affected>
```

Initial state: no deviations logged. F.0's draft execution did not surface any deviation-worthy decisions; all design choices were made within the F.0 prompt's guidance and are captured in §2.

### 2026-04-21, Sub-phase F.2 (pre-execution), commit TBD

**Deviation:** F.2 scope expanded beyond F.0's original plan to include a continental-scale macro-feature modulating `mountains_amplitude` spatially across the world. F.0 specified F.2 as "DomainWarped on grassland + mountain" only; this amendment expands to "DomainWarped on five presets + continental modulation on the same five." §2.6 and §4 (entirely) are rewritten; §9 and §10 are updated.

**Rationale:** F.1 post-landing visual verification (Andrew, 2026-04-21 stills) revealed a repeating Beach→Grassland→Forest→Mountain pattern in aerial views — every local peak reaches Mountain elevation and gets the full biome sequence on its slopes. DomainWarped alone (F.0's original F.2 scope) would break the _within-peak_ repetition but not the _distribution-of-peaks_ repetition; a continental-scale amplitude modulation is the architectural intervention that addresses the latter. User target is North Carolina-style continental geography (Coastal Plain → Piedmont → Blue Ridge), which is a continental-scale shape concern, not a within-noise-field concern.

**Impact:** F.2 complexity and duration grow modestly (estimated +4-8 hours of agent time). The continental field provides architectural foundation for F.3's erosion (natural region-appropriate erosion intensity) and F.4's climate-as-spatial-field (continental feature composes with climate gradients). F.5 integration tuning gets one additional tuning knob (continental scale / min). Directional bias (the NC southwest-northeast axis) is NOT included; deferred to F.5 or follow-up. F.2 sub-commit list grows from three (F.2.A/B/C) to four (F.2.A/B/C/D).

---

## 11. References

- `docs/audits/heightmap_generator_audit_2026-04-21.md` — the audit that surfaced the unwired components, catalogued the six intervention options, and motivated this campaign (Option F selected).
- `docs/audits/phase_1_5_tuning_investigation_2026-04-20.md` — Phase 1.5-T's investigation with the stale 125-unit measurement that F.1's correction note addresses.
- `docs/current/TERRAIN_MATERIAL_SYSTEM_CAMPAIGN.md` — parent campaign. Its §7 gets updated when Phase 1 and Phase 1.5 are re-marked COMPLETE at F.5; its §9 receives the F.0 launch entry in the same commit as this document.
- `astraweave-terrain/src/advanced_erosion.rs` — the erosion simulator wired in F.3 (902 lines, 5 named presets).
- `astraweave-terrain/src/noise_gen.rs:154-211` — `DomainWarpedNoise` implementation used in F.2.
- `astraweave-terrain/src/climate.rs` — `ClimateMap` rewired in F.4.
- `astraweave-terrain/src/elevation_biome.rs` — Phase 1.5's biome-weight computation; F.4 extends its API (not its algorithm).
- `tools/aw_editor/src/panels/terrain_panel.rs:1707-1730` — stubbed `apply_erosion` handler wired in F.5.
- `tools/aw_editor/src/panels/terrain_panel.rs:1816-1991` — eight biome-noise presets tuned across F.1/F.2/F.5.
- `tools/aw_editor/src/terrain_integration.rs:317-397` — editor terrain-generation entry point; restructured in F.3 and F.4.

---

*End of plan*
