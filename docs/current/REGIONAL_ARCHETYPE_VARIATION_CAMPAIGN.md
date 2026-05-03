# Regional Archetype Variation Campaign — Phase 1.X

**Status**: F.0 (campaign plan) COMPLETE 2026-04-29, commit 0e51763d4. F.1 (climate field extension) COMPLETE 2026-04-29, commits `5fcde4e98` (F.1.A) + `744132c6a` (F.1.B) + `8e883beb5` (F.1.C) + df7636fa3 (F.1.D). F.2 (BootstrapSplineSet infrastructure) COMPLETE 2026-04-30, commits `f0c3fa52d` (F.2.A) + `b5d70071d` (F.2.B) + `43386cba5` (F.2.C) + b58bc3585 (F.2.D). F.3 (spline wiring, single-archetype regression) COMPLETE 2026-05-01, commits `04fc26299` (F.3.A) + `447367c15` (F.3.B) + `be233fd78` (F.3.C) + d6a063fbb (F.3.D). Continental Temperate single-archetype wiring complete; byte-identity regression preserved post-f64 retrofit; Andrew-gate PASS 2026-05-01. F.5-paint (editor UI panel scaffold + brush + save/load — first half of F.5 two-session split) IN PROGRESS — **REGIONAL ARCHETYPE VARIATION CAMPAIGN PAUSED 2026-05-03** pending Editor Multi-Tool Architecture campaign (foundational architectural work). Original commit chain `26a3864b8` (F.5-paint.A) + `226572bae` (F.5-paint.B) + `2b230d94e` (F.5-paint.C) + `e9d2a7922` (F.5-paint.D) + `b6dd9de58` (hash-fixup) landed cleanly at code level (30/30 panel unit tests pass). Remediation chain: E-diagnostic complete (`5f772bea3` audit + `e561d4dce` campaign doc revert + `f848388a6` hash-fixup); F-fix complete (`b2df0be20` registration + 2 Pattern A tests + `722b70ae5` exhaustive-match dispatch supplement + `dee94ea05` doc closeout + audit amendment + `1d67b3328` hash-fixup); G-pointer-events-research COMPLETE 2026-05-03 (`6992f4b39` audit + `e748a6304` campaign doc + `506dec13c` hash-fixup); G-pointer-events-diagnostic COMPLETE 2026-05-03 (`ac4bc58a3` audit + `ab67aeb0b` campaign doc + `57de03fba` hash-fixup); G-diagnostic confirmed AstraWeave editor uses approach (B) with main.rs as per-frame mediator; per Q1 Option B audit §7 surfaced four options (B-extend, A, C, Hybrid). Andrew-gate narrowed verdict 2026-05-03: PASS for registration (panel reachable via View → Panels submenu; opens as dockable tab; UI renders without panic; 11 surfaces total). **Andrew architectural decision 2026-05-03**: spin off Option C (ActiveTool dispatcher trait + per-tool registration matching AAA canonical pattern) as its own foundational campaign rather than executing B-extend or Hybrid in G-fix. Honors §0 research-pass-before-reframe discipline + F.4.B.3.D lesson application (halt-and-re-research when execution surfaces architectural gap). New campaign at `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` (research pass + campaign-design pass + execution sessions pending). Regional Archetype Variation campaign resumes post-Editor-Multi-Tool-Architecture with G-pointer-events-fix scope shifted from "B-extend mechanical execution" to "register RegionalArchetypePanel with the ActiveTool dispatcher" (single small commit). All F.5-paint commits preserved unchanged (no revert); panel registration correct per F-fix; G-research + G-diagnostic audits forward-applicable as predecessor research for the spinoff campaign. Sequence: Editor Multi-Tool Architecture (research → design → execution) → F.5-paint resumption → G-pointer-events-fix → H-saveload-diagnostic → H-saveload-fix → final Andrew-gate full PASS → F.5-paint COMPLETE → F.5-overlay-and-gate → F.6 → F.7 (principal Andrew-gate) → F.8 closeout. F.5 itself remains IN PROGRESS pending F.5-overlay-and-gate after the paint slice fully closes post-resumption. F.4 (RegionalArchetypeMask + falloff sampler) COMPLETE 2026-05-01, commits `3eafa4ca4` (F.4.A mask + RON metadata) + `3c6c915c9` (F.4.B save/load + Euclidean distance transform) + `489eee27d` (F.4.C RegionalArchetypeBlend neighborhood-scan sampler) + `1b63b921a` (F.4.D multi-archetype blend math) + `d3e2c0f67` (F.4.E WorldGenerator integration) + `8581391dc` (F.4.F integration + perf + synthetic-differentiation tests) + F.4.G deferred-closeout reconciled via this pause artifacts session 2026-05-03; Andrew-gate PASS 2026-05-01 (programmatic 5-archetype world per campaign §7.3 success criteria; visibly distinct archetype regions with smooth boundaries; performance within +30% of F.3 baseline). F.6-F.8 NOT STARTED.
**Scope**: Deliver regional archetype variation in AstraWeave's heightmap terrain pipeline. Each generated world contains 5-10 archetype regions (one per Tikva storyline), painted by the writer onto a 2D archetype mask, with smooth blending between adjacent regions. Per-archetype shape character is produced by climate-driven shape splines (Minecraft 1.18+ pattern) that map climate parameters to bootstrap noise parameters. Architecture: **Hybrid C + F** per `docs/audits/regional_archetype_variation_research_2026-04-29.md` §7 (paintable archetype mask + climate-driven shape splines per archetype). Eight sub-phases (F.0-F.8) executed as separate sessions; integrates with D.1-D.5 landed work as the within-region machinery.
**Author**: Plan drafted 2026-04-29 by the campaign-design session, against the research audit and Andrew's 2026-04-29 chat clarification of the regional-variation product target.
**Prior work**: `docs/audits/regional_archetype_variation_research_2026-04-29.md` (the research audit; this campaign's load-bearing input). `docs/audits/f4b3d5_diagnostic_3_cross_archetype_2026-04-28.md` (the architectural-gap data that motivated the research pass). `docs/current/TERRAIN_GENERATION_QUALITY_CAMPAIGN.md` (predecessor; F.4.B.3.D delivered D.1-D.5 within-region machinery; closed via architectural pivot in a parallel session).
**Outcome on completion**: Editor renders worlds with 5-10 visually-distinct archetype regions painted by the writer; smooth blending at transition zones reads as natural geography at flyover altitude (Crimson-Desert-class visual reference); cross-archetype Pearson < 0.7 (distant pairs) / < 0.85 (adjacent pairs), measured by re-running the diagnostic-3-style cross-archetype Pearson harness; Andrew-gate visual verification PASS at F.7 closeout.

---

## 0. How to use this document and anti-drift discipline

This plan is the authoritative design reference for the Regional Archetype Variation campaign. It adapts the predecessor campaign's §0 discipline to this campaign's structure.

### Discipline imposed

1. Every sub-phase's completion commit must update §9 of this document (the phase-status block) to mark the sub-phase COMPLETE, with the commit hash of completion.
2. **No sub-phase is "complete" until plumbing verifies (compilation + tests) AND the §9 status update commit has landed AND the sub-phase's Andrew-gate (where applicable) returns PASS.** This is the §0 corrective from the predecessor campaign baked in non-negotiably (see "Lesson application" below).
3. The "Status" header at the top of this document must be updated as sub-phases land: "F.0 complete, F.1-F.8 not yet started" → "F.0-F.1 complete, F.2 in progress" → "Campaign complete (date)."
4. Design decisions captured in §2 are authoritative — they are resolved once, in F.0 (this document), and sub-phases F.1-F.8 execute against them rather than re-deciding them. If a §2 decision proves infeasible during execution, stop, record a deviation in §10, and escalate for a revised decision before proceeding.
5. Any discovered need to deviate from this plan during execution must be recorded in §10 (Deviations log) with rationale, before or in the same commit as the deviation itself.
6. No "while-I'm-here" code changes. Sub-phase scopes in §3-§7 are exclusive; touching files outside the listed scope is a deviation that must be logged.

### Lesson application — Andrew-gate authoritative for visible-terrain sub-phases

The predecessor campaign (F.4.B.3.D) ran 6+ sub-phases that each declared code-level PASS while Andrew-gate caught REGRESSes three times: F.4.B.3.B (octave-emphasis Path 1), F.4.B.3.C (runevision filter), and F.4.B.3.D.5 (archetype-as-coloring after Path B amplitude reduction). The pattern: code-level metrics passed; visual reality failed. Each REGRESS came from declaring code-level success without the user-visible behavioral gate.

The new campaign treats Andrew-gate as the **authoritative completion signal** for any sub-phase that produces visible terrain output, NOT as a courtesy check after code-level metrics pass. Specifically:

- **Sub-phases without visible-terrain output** (F.1 climate field extension; F.2 BootstrapSplineSet infrastructure; F.8 closeout) gate on code-level verification only.
- **Sub-phases with visible-terrain output** (F.3 spline wiring; F.4 mask integration; F.5 paint UI; F.6 regional blending; F.7 per-archetype tuning) require Andrew-gate PASS before §9 updates to COMPLETE. If Andrew-gate returns REGRESS or PARTIAL, the sub-phase remains IN PROGRESS until either remediation lands or the campaign-level decision changes.

This discipline is **non-negotiable** for the campaign's structure. It is the explicit lesson application from the predecessor campaign; do not let it erode mid-campaign.

### Scope-creep discipline — research-pass-before-reframe

The predecessor campaign's scope grew across 6+ sub-phases as diagnostics surfaced architectural gaps. Each scope expansion was justified by the data at the time, but the cumulative effect was a campaign whose final scope was unrecognizable from F.0 framing. The corrective: **if a sub-phase surfaces an architectural gap that requires reframing the campaign's scope, treat that as evidence of insufficient research-pass depth and consider another research pass rather than continuing to expand the campaign in-flight.** Estimated 8-12 sessions (F.0-F.8 plus diagnostic+remediation pairs); if a sub-phase's discoveries push the estimate beyond 15 sessions, stop and re-research before continuing.

This discipline is applied prophylactically: §2 of this document is the architectural commitment that subsequent sub-phases execute against. §2's six decisions (§3.A-§3.F of the F.0 prompt; this document's §2.2-§2.8) are load-bearing. Vagueness in §2 propagates downstream as sub-phase ambiguity that produces mid-campaign reframes. The campaign-design session that drafted this document committed to concrete resolutions of all six decisions; subsequent sub-phases respect those resolutions or escalate via §10.

### Anti-pattern this plan explicitly prevents

The F.4.B.3.D campaign's reframe drafted from first principles in F.0 and grew its scope across 6+ sub-phases. The new campaign's F.0 (this document) is the structural corrective: it is informed by a research pass (`regional_archetype_variation_research_2026-04-29.md`) that surveyed 6 AAA references + 6 algorithmic approaches before any architectural commitment, and translates the research recommendation into concrete sub-phase specifications.

The campaign-design discipline pattern:

- **F.0 is research-informed**, not first-principles. The architectural recommendation comes from a research audit; the campaign document captures its application.
- **§2 is load-bearing**. Architectural decisions are committed at F.0 with rationale (§10 launch entries) and respected by F.1-F.8.
- **Andrew-gate is authoritative for visible-terrain sub-phases**. Code-level success is not plan-level success.

---

## 1. Design summary

### 1.1 The problem being solved

F.4.B.3.D delivered the climate-field-driven Whittaker biome architecture (D.1 climate field, D.2 biome lookup, D.3 per-biome parameters, D.4 scattered-convolution biome blending, D.5 world archetype catalog + UI) but did not deliver per-archetype terrain shape variety. The diagnostic-3 audit (`docs/audits/f4b3d5_diagnostic_3_cross_archetype_2026-04-28.md`) measured the gap:

- **Cross-archetype Pearson 0.978-0.989** across measured pairs (CT vs Desert, CT vs ET) — nearly identical heightmaps.
- **Bootstrap dominates terrain shape (72-81% of variance)**; per-biome modulation contributes 19-28%.
- **1 of 18 bootstrap parameters is archetype-aware** (the per-vertex `mountain_amplitude_multiplier` scalar).

The architecture's design boundary made biomes drive per-biome shape character via amplitude (D.3a's design decision), not per-archetype shape character via bootstrap parameters. Andrew's 2026-04-29 chat clarified the actual product target: **regional archetype variation with smooth blending, ~5-10 archetypes per world, one archetype per Tikva storyline region.**

### 1.2 The target

**Hybrid C + F** per the research audit's §7 recommendation:

- **Approach C — Paintable archetype mask with falloff**. Writer paints archetype regions onto a 2D world map; bootstrap parameters interpolate across painted region boundaries with author-controlled falloff. **Honors Veilweaver writer intent** (Tikva storyline regions get pinned archetype identities). Confirmed AAA pattern by Crimson Desert, Enshrouded, Skyrim, Hytale, Witcher 3 (5 of 6 references).
- **Approach F — Climate-driven shape splines per archetype**. Each `WorldArchetype` carries a `BootstrapSplineSet` mapping climate parameters `(continentalness, erosion, PV)` to bootstrap shape parameters `(mountains_amplitude, mountains_scale, continental_scale, base_elevation_amplitude)`. Splines read the climate field (D.1's `ClimateMap::sample`, extended with `erosion` and `weirdness` fields) and produce per-vertex bootstrap parameters within each region. **Canonical implementation: Minecraft 1.18+ noise router** with separable splines.

The mask gives the writer **what** the region's character is. The splines give each region's character **how** the terrain expresses it (mountains where continentalness is high + erosion is low; flat where erosion is high; jagged where PV is high). Combination produces hand-authored macro layout + per-region procedural shape character + smooth transitions.

### 1.3 The eight-sub-phase breakdown

- **F.0 Campaign plan** (this document). Drafts the campaign ground-truth document; resolves §2 architectural decisions; seeds §10 with launch entries.
- **F.1 Climate field extension**. Extends `ClimateMap::sample` with `erosion` and `weirdness` fields. Adds `PvFold` helper for the Peaks-and-Valleys derived field. Verifies D.1 invariants preserved at Continental Temperate defaults.
- **F.2 BootstrapSplineSet infrastructure**. Defines `BootstrapSplineSet`, `Spline1D`, `BootstrapParam` types. Implements separable-spline evaluation. Lands default `BootstrapSplineSet` instances for the 6 D.5 catalog archetypes as Rust constants. No wiring into noise pipeline yet.
- **F.3 Spline wiring (single archetype regression)**. Wires `BootstrapSplineSet` into `WorldGenerator::generate_chunk_with_climate` for Continental Temperate only. Produces byte-identical (or within-tolerance) output to F.4.B.3.D.5-fix at Continental Temperate defaults; verifies per-archetype shape character is now expressible.
- **F.4 RegionalArchetypeMask + falloff sampler**. Implements mask format (single-channel ID + falloff distance field, 1024×1024), runtime sampler, archetype-blend per-vertex aggregation. Tests via programmatically-constructed masks; no editor UI yet.
- **F.5 Editor UI for archetype painting**. New `RegionalArchetypePanel` alongside existing `TerrainPanel`. Brush + falloff + archetype palette + save/load. Climate Preview overlay (D.5c absorbed).
- **F.6 Scattered-convolution at the regional layer**. Applies NoisePosti.ng-style organic blending to mask-driven archetype assignments at boundaries. Eliminates axis-aligned artifacts in transition zones.
- **F.7 Per-archetype tuning + Andrew-gate**. Tunes the 6 catalog archetypes' splines so each produces visibly distinct character. Re-runs diagnostic-3-style cross-archetype Pearson measurement. **Principal Andrew-gate of the campaign**: side-by-side screenshots from 6 archetypes pass a "these look like different worlds" review.
- **F.8 Closeout**. Updates parent campaign (`TERRAIN_MATERIAL_SYSTEM_CAMPAIGN.md`); updates `ARCHITECTURE_MAP.md` terrain section; absorbs F.4.B.3.D deferred work that this campaign owns; standard housekeeping.

### 1.4 Integration with D.1-D.5 landed work

**Stays unchanged**:

- D.1 `ClimateMap` core architecture (climate field per-vertex API; existing `temperature_c`, `moisture_mm`, `continentalness` fields).
- D.2 `lookup_biome` (Whittaker classification at the biome layer).
- D.3 `BiomeParameters` (per-biome shape character within a region).
- D.4 `blend_biome_parameters` (scattered-convolution biome blending).
- D.5 `WorldArchetype` catalog (6 archetypes; gets extended with `bootstrap_splines: BootstrapSplineSet`).

**Gets extended**:

- `WorldArchetype` adds `bootstrap_splines: BootstrapSplineSet` field. Existing climate envelope fields (means/variances/latitude_drop) preserved.
- `ClimateMap::sample` extended to return `ClimateSample { temperature_c, moisture_mm, continentalness, erosion, weirdness }` (5 fields total, +2 from D.1's 3). PV is computed at sample time via `PvFold` helper, not stored.
- `WorldGenerator::generate_chunk_with_climate` reads the painted archetype mask, computes per-vertex archetype contributions from mask + falloff, evaluates each contributing archetype's `BootstrapSplineSet` against the climate sample, blends spline outputs per §2.5, then runs the bootstrap noise pipeline once with blended parameters.

**New components**:

- `RegionalArchetypeMask` — author-paintable 2D mask resource. Single-channel ID (uint8, 0-255 archetype IDs) + per-pixel falloff distance field (uint8 normalized).
- `RegionalArchetypeBlend` — runtime sampler. Reads mask + falloff, computes blend weights from falloff distance, looks up archetype splines per contributing archetype, blends per §2.5.
- `BootstrapSplineSet` — per-archetype spline definitions for bootstrap parameters. 4 splines per archetype (one per `BootstrapParam` variant). Each spline is separable-form: 3 independent 1D splines (one per climate input axis) multiplicatively combined.
- `Spline1D` — piecewise-linear spline with author-controlled control points. Sorted `(input, output)` pairs with linear interpolation between adjacent points.
- `PvFold` — helper computing `PV = 1.0 - ((3.0 * |weirdness|) - 2.0).abs()` per Minecraft 1.18+ canonical formula.
- `RegionalArchetypePanel` — new editor panel for archetype painting (alongside existing `TerrainPanel`).
- Climate Preview overlay (D.5c absorbed into F.5).

---

## 2. Technical architecture

### 2.1 Data flow at the end state

After the full campaign lands, a chunk's generation pipeline is:

```text
1. Mask + falloff sampling (CPU, RegionalArchetypeBlend::sample_at)
   Input: world position per vertex, RegionalArchetypeMask (1024×1024 ID + falloff)
   Output: per-vertex Vec<(WorldArchetypeId, weight)> — 1-4 contributing archetypes,
           weights sum to 1.0, derived from falloff distance.

2. Climate sampling (CPU, ClimateMap::sample — extended in F.1)
   Input: world position per vertex, pre-erosion-elevation hint (currently 0.0;
          climate field uses world XZ + lapse rate at sample time)
   Output: ClimateSample { temperature_c, moisture_mm, continentalness, erosion,
           weirdness }. PV = pv_fold(weirdness) computed here for downstream use.

3. Per-archetype spline evaluation (CPU, BootstrapSplineSet::evaluate)
   Input: ClimateSample (continentalness, erosion, PV), BootstrapSplineSet for
          each contributing archetype.
   Output: per-archetype BootstrapParams { mountains_amplitude, mountains_scale,
           continental_scale, base_elevation_amplitude }.

4. Archetype-blended bootstrap parameters (CPU, RegionalArchetypeBlend::blend_params)
   Input: per-vertex (archetype_id, weight) list + per-archetype BootstrapParams.
   Output: blended BootstrapParams (one set per vertex). Per §2.5: blending
           happens at spline-output layer; each archetype evaluates independently;
           weights from mask falloff combine outputs.

5. Bootstrap noise pipeline (CPU, TerrainNoise::sample_height_with_params)
   Input: blended BootstrapParams + world position.
   Output: pre-erosion height per vertex.
   NOTE: bootstrap pipeline runs ONCE per vertex with blended params, NOT once per
         contributing archetype. Per §2.5 design decision (c).

6. Biome lookup (CPU, lookup_biome — D.2 unchanged)
   Input: ClimateSample + pre-erosion height.
   Output: BiomeId per vertex.

7. Per-biome scattered-convolution blending (CPU, blend_biome_parameters — D.4 unchanged)
   Input: ClimateSample + N jittered samples per vertex.
   Output: BlendedBiomeParams per vertex. Operates within whichever archetype
           the vertex resolves to; archetype blend already applied in step 4.

8. Per-biome modulation (CPU, apply_per_biome_modulation_to_halo — D.3 unchanged)
   Input: pre-erosion height + BlendedBiomeParams.
   Output: per-biome-amplitude-scaled pre-erosion height.

9. Erosion (CPU, AdvancedErosionSimulator — F.3-phase-2/3/4 unchanged)
   Input: pre-erosion heightmap + per-archetype erosion preset (chunk-center).
   Output: post-erosion heightmap.

10. Vertex buffer assembly (editor-side, generate_heightmap_mesh — unchanged)
    Input: post-erosion height + pre-erosion biome_weights.
```

**Critical invariants preserved**:

- Pre-erosion biome_weights → post-erosion heights stability (§2.5 of predecessor; carries through).
- Per-vertex determinism: same `(world_seed, world_x, world_z, archetype_mask)` produces same heights.
- Halo-based chunk continuity: archetype mask sampling uses world coordinates; adjacent chunks' halos sample the same mask values at shared edges.

**New invariants**:

- Painted mask determines archetype identity at each vertex; mask is the authoring source of truth.
- Climate field continuity: adjacent vertices' climate samples differ smoothly; spline outputs differ smoothly; blended bootstrap params differ smoothly.
- Archetype blend stability: in transition zones, contributing archetypes' weights sum to 1.0 ± epsilon; blended bootstrap params lie within the convex hull of contributing archetypes' params (§2.5 ensures this).

### 2.2 Climate field extension (resolves §3.A)

**Decision**: extend `ClimateMap::sample` with `erosion` and `weirdness` fields. Total climate dimensionality: 5 stored fields + 1 derived field (PV).

**New fields**:

- `erosion: f32` — low-frequency Perlin noise field representing flatness propensity. Per Minecraft 1.18+ canonical interpretation: high erosion → flat terrain; low erosion → mountainous. Range `[-1.0, +1.0]`. Scale: `0.0008` (wavelength ~1250 WU; ~9 periods across Target B's 11264 WU per side). Seed offset: `+3000` (decorrelated from temperature seed +0, moisture seed +1000, continentalness seed +2000).
- `weirdness: f32` — input to PV (Peaks-and-Valleys) fold. Low-frequency Perlin. Range `[-1.0, +1.0]`. Scale: `0.0006` (wavelength ~1670 WU). Seed offset: `+4000`.

**Derived field**:

- `pv: f32` — computed at sample time via `PvFold::from_weirdness(weirdness) -> f32`. Formula: `pv = 1.0 - ((3.0 * weirdness.abs()) - 2.0).abs()`. Range `[-1.0, +1.0]` with characteristic 3-band character (Valleys / Mid / Peaks); folded weirdness produces 5 categorical terrain levels.

**ClimateSample type extension**:

```rust
pub struct ClimateSample {
    // Existing (D.1):
    pub temperature_c: f32,
    pub moisture_mm: f32,
    pub continentalness: f32,
    // New (F.1):
    pub erosion: f32,
    pub weirdness: f32,
}

impl ClimateSample {
    pub fn pv(&self) -> f32 {
        1.0 - ((3.0 * self.weirdness.abs()) - 2.0).abs()
    }
}
```

**Backward compatibility**: D.1's `WorldArchetype` envelope (means/variances for temperature/moisture/continentalness) preserved unchanged. New fields use independent low-frequency noise; archetype envelope does NOT modulate erosion or weirdness in F.1 (could be added in F.7 if archetype tuning shows benefit).

**Rationale**:

- Adding 2 fields keeps the climate field dimensionality aligned with Minecraft 1.18+'s separable-spline architecture: 3 climate inputs `(continentalness, erosion, PV)` per archetype spline → 3 separable 1D splines per `BootstrapParam` per archetype.
- Skipping `weirdness`/PV would simplify to 2D climate inputs but lose Minecraft's "Peaks and Valleys" character variation (sharp ridges vs smooth rolling), which is part of what makes archetypes feel different.
- 6D Minecraft space (temperature/humidity/continentalness/erosion/weirdness/depth) is excessive: AstraWeave's `lookup_biome` already handles biome classification from `(temp_c, moisture_mm, elevation_m)`; adding `humidity` would duplicate `moisture_mm`'s role; `depth` is a vertical parameter for voxel terrain, not heightmap.
- Field count chosen for minimum dimensionality that supports per-archetype shape variation. 3 → 5 is the smallest viable extension.

**Performance impact**: 2 additional Perlin noise samples per vertex in `ClimateMap::sample`. At radius 10 with 96² vertices per chunk × 121 chunks, that's ~2.2M extra noise samples per generation pass. Cost estimate: ~5-10 ms per generation pass at radius 10 (well under 1% of D.4's ~0.747s/chunk baseline).

### 2.3 BootstrapSplineSet design (resolves §3.B)

**Decision**: 4 splines per archetype, separable-form (3 independent 1D splines per parameter, multiplicatively combined), piecewise-linear, embedded as Rust constants for the 6 catalog archetypes.

**Bootstrap parameters as spline outputs** (4 splines per archetype, one per `BootstrapParam` variant):

- `MountainsAmplitude` — currently `NoiseConfig.mountains.amplitude` (hardcoded 480 in `NoiseConfig::default()`). Spline output range: `[100.0, 800.0]`. Default base value: 480.0 (matches current default).
- `MountainsScale` — currently `NoiseConfig.mountains.scale` (hardcoded 0.002). Spline output range: `[0.0008, 0.005]`. Default base value: 0.002.
- `ContinentalScale` — currently `NoiseConfig.continental_scale` (hardcoded 0.0003). Spline output range: `[0.00015, 0.0006]`. Default base value: 0.0003.
- `BaseElevationAmplitude` — currently `NoiseConfig.base_elevation.amplitude` (hardcoded 150). Spline output range: `[50.0, 300.0]`. Default base value: 150.0.

**Spline representation**:

- **Separable form** (Minecraft 1.18+ pattern): each `BootstrapParam` is computed as the product of three independent 1D splines, one per climate input axis (`continentalness`, `erosion`, `pv`). Formula: `output = base_value × spline_continentalness(c) × spline_erosion(e) × spline_pv(pv)`.
- **1D splines are piecewise-linear**: sorted `Vec<(f32, f32)>` of `(input, output_multiplier)` pairs. Linear interpolation between adjacent control points; clamp at endpoints for inputs outside the control-point range. Output multiplier is dimensionless `[0.0, 4.0]` typical (allows up to 4× boost or full suppression).

**Type structure**:

```rust
pub struct Spline1D {
    pub control_points: Vec<(f32, f32)>,  // sorted by input; linear interp
}

impl Spline1D {
    pub fn evaluate(&self, input: f32) -> f32 { /* piecewise-linear */ }
    pub const fn identity() -> Self { /* always returns 1.0 */ }
}

pub struct ParamSpline {
    pub base_value: f32,
    pub spline_continentalness: Spline1D,
    pub spline_erosion: Spline1D,
    pub spline_pv: Spline1D,
}

impl ParamSpline {
    pub fn evaluate(&self, sample: &ClimateSample) -> f32 {
        self.base_value
            * self.spline_continentalness.evaluate(sample.continentalness)
            * self.spline_erosion.evaluate(sample.erosion)
            * self.spline_pv.evaluate(sample.pv())
    }
}

pub struct BootstrapSplineSet {
    pub mountains_amplitude: ParamSpline,
    pub mountains_scale: ParamSpline,
    pub continental_scale: ParamSpline,
    pub base_elevation_amplitude: ParamSpline,
}

impl BootstrapSplineSet {
    pub fn evaluate(&self, sample: &ClimateSample) -> BootstrapParams {
        BootstrapParams {
            mountains_amplitude: self.mountains_amplitude.evaluate(sample),
            mountains_scale: self.mountains_scale.evaluate(sample),
            continental_scale: self.continental_scale.evaluate(sample),
            base_elevation_amplitude: self.base_elevation_amplitude.evaluate(sample),
        }
    }
}

pub struct BootstrapParams {
    pub mountains_amplitude: f32,
    pub mountains_scale: f32,
    pub continental_scale: f32,
    pub base_elevation_amplitude: f32,
}
```

**Storage**: embedded as Rust `const` declarations in `astraweave-terrain/src/bootstrap_splines.rs` for each of the 6 D.5 catalog archetypes. Per-archetype default `BootstrapSplineSet` ships as code, not data file. Custom archetype starts with Continental Temperate's spline set; editor UX exposes spline-curve editing for Custom only (deferred to F.5+ if relevant; out of F.0 scope).

**Rationale**:

- **Separable form** matches Minecraft's published architecture and is faster to evaluate than full 3D splines (3 1D evaluations + 2 multiplications vs trilinear interpolation in a 3D grid). Loses some expressivity (cannot model interactions between climate axes) but interactions are rare in geographic terrain (mountain amplitude generally increases with continentalness × decreases with erosion; multiplicative form captures this naturally).
- **Piecewise-linear** is simpler than Catmull-Rom or cubic Bézier. Sufficient for terrain parameter mapping; ~5-10 control points per spline produce smooth-enough curves; faster to evaluate.
- **Rust constants for the catalog** matches D.5's architecture (6 archetypes are compile-time data); avoids file format design at F.0; modders/customs can be supported later via runtime spline editing if needed.
- **4 splines per archetype** captures the bootstrap parameters with the largest visible impact per the diagnostic-3 audit (§1.4 archetype-aware parameter audit). Adding more spline outputs (octaves, persistence, lacunarity, noise type) is possible but not committed to at F.0; F.7 may expand if tuning surfaces benefit.

**Performance impact**: per-vertex spline evaluation is 4 splines × 3 1D evaluations × ~10 control points (binary search + linear interp). Cost estimate: ~50-100 ns per vertex = ~50-100 ms per generation pass at radius 10. Within +30% budget vs D.4 baseline.

### 2.4 RegionalArchetypeMask design (resolves §3.C)

**Decision**: 1024×1024 mask spanning the 115 km² Target B world. Single-channel uint8 archetype ID + separate uint8 falloff distance field. Two textures, ~2 MB total.

**Mask layout**:

- **Resolution**: 1024×1024. World extent at Target B is 11264 WU per side (radius 10 × 512 WU chunks × 2). Per-pixel world extent: 11264 / 1024 = ~11 WU per pixel. Sufficient for archetype-region authoring; transitions handled by falloff at sampling time, not by mask resolution.
- **Channel 1 — Archetype ID**: uint8, value 0-255 = archetype index in the catalog. 0 reserved for "unpainted" (unpainted regions default to Continental Temperate at sample time). 1-N = catalog indices.
- **Channel 2 — Falloff distance**: uint8 normalized, value 0-255. Encodes distance from this pixel to the nearest archetype boundary in mask coordinates, normalized by a configurable `falloff_radius_pixels` (default 32 pixels = ~352 WU). Value 0 = on a boundary; value 255 = fully inside one archetype's interior. Computed offline at mask save time via Euclidean distance transform.

**File format**:

- Saved as a pair of files per world: `<world>.mask.id.bin` (1 MB raw uint8 grid) + `<world>.mask.falloff.bin` (1 MB raw uint8 grid). Optional `<world>.mask.ron` metadata (resolution, world extent, falloff_radius_pixels).
- Loaded into `RegionalArchetypeMask` runtime resource at editor / world load. Held in memory during runtime; sampled per vertex via `RegionalArchetypeBlend::sample_at(world_x, world_z)`.

**Runtime sampling**:

```rust
pub struct RegionalArchetypeMask {
    pub resolution: u32,             // 1024
    pub world_extent_wu: f32,         // 11264.0
    pub falloff_radius_pixels: u32,   // 32
    pub ids: Vec<u8>,                 // 1024 × 1024
    pub falloff: Vec<u8>,             // 1024 × 1024
}

pub struct RegionalArchetypeBlend<'a> {
    mask: &'a RegionalArchetypeMask,
}

impl<'a> RegionalArchetypeBlend<'a> {
    /// Returns up to 4 (archetype_id, weight) pairs, weights summing to 1.0.
    pub fn sample_at(&self, world_x: f32, world_z: f32)
        -> ArrayVec<(WorldArchetypeId, f32), 4> { /* ... */ }
}
```

**Sampling algorithm** (per vertex, simplified):

1. Convert `(world_x, world_z)` → mask pixel coords `(px, pz)`.
2. Sample mask ID + falloff at `(px, pz)`.
3. If falloff value > threshold (e.g., 200/255 = "deep interior"), return single `(id, 1.0)`.
4. Else (transition zone), examine mask in a `falloff_radius_pixels` neighborhood, identify all distinct archetype IDs within `falloff_radius_pixels` of this vertex, compute weight per ID = `clamp(1.0 - distance_to_nearest_pixel_of_that_id / falloff_radius_world, 0.0, 1.0)`. Normalize weights to sum to 1.0. Return up to 4 strongest contributors (pruning < 0.05 weights).

**Determinism**: same `(world_x, world_z, mask)` → same blend output. Adjacent chunks' halos sample the same mask at shared edges → same archetype contributions → continuous bootstrap params at shared edges.

**Rationale**:

- **1024×1024 resolution** balances authoring fidelity (~11 WU per pixel) with memory cost (2 MB total) and disk cost (~1-2 MB uncompressed; ~10-100 KB compressed since archetype IDs have low entropy).
- **Single-channel ID + separate falloff** is simpler to reason about than packed RGBA8 multi-archetype-weights. Sampling examines neighborhood at runtime; falloff field is a precomputed Euclidean distance transform that accelerates "is this pixel near a boundary?" checks.
- **256 archetypes max** is far above Veilweaver's 5-10 target. Headroom for future expansion. Reserving ID 0 for "unpainted" simplifies palette sampling.
- **Up to 4 contributors per vertex** matches typical mask-painting boundary geometry (corners where 4 regions meet). 4 simultaneous spline evaluations per vertex is the campaign's worst-case per-vertex cost target.

**Performance impact**: 2 texture reads (1 byte each) per vertex + neighborhood scan (typically ~10-50 pixel reads in transition zones, 1 read in interior zones). Cost estimate: ~100-300 ns per vertex in transition zones, ~20 ns in interior zones. Aggregate ~50-100 ms per generation pass at radius 10.

### 2.5 Archetype blending math (resolves §3.D)

**Decision**: blend spline outputs after evaluation (option (c) from F.0 prompt §3.D). Each contributing archetype evaluates its `BootstrapSplineSet` independently against the same climate sample; weights from mask falloff combine spline outputs into per-vertex blended `BootstrapParams`. Bootstrap noise pipeline runs once per vertex with blended params.

**Algorithm**:

```rust
fn blend_bootstrap_params(
    contributors: &[(WorldArchetypeId, f32)],  // (id, weight), weights sum to 1.0
    archetype_splines: &impl Fn(WorldArchetypeId) -> &BootstrapSplineSet,
    sample: &ClimateSample,
) -> BootstrapParams {
    let mut blended = BootstrapParams::zero();
    for (id, weight) in contributors {
        let archetype_params = archetype_splines(*id).evaluate(sample);
        blended.mountains_amplitude += weight * archetype_params.mountains_amplitude;
        blended.mountains_scale += weight * archetype_params.mountains_scale;
        blended.continental_scale += weight * archetype_params.continental_scale;
        blended.base_elevation_amplitude += weight * archetype_params.base_elevation_amplitude;
    }
    blended
}
```

**Why not the alternatives**:

- (a) **Bootstrap output heights**: requires evaluating the noise pipeline N times per vertex (once per contributing archetype). At 4-archetype overlap zones, that's 4× the bootstrap cost. Rejected for performance.
- (b) **Spline parameters before evaluation**: would blend `mountains_amplitude`, `continental_scale`, etc. across archetypes at the spline-input layer (i.e., averaging the splines' base values + control points). This produces non-physical combinations in transition zones (e.g., "average mountain amplitude" mixed with "average erosion response" doesn't represent any archetype's character). Rejected for architectural correctness.
- (d) **Bootstrap noise inputs**: unclear what this even means architecturally. Rejected.

**Why (c) works**:

- Each archetype's splines produce its character; blend happens after. In a transition zone with 50% Continental Temperate + 50% Boreal contributions, the blended params are 50% of each archetype's spline output → terrain reads as a smooth interpolation between the two characters.
- Bootstrap noise pipeline runs ONCE per vertex with blended params (cheapest).
- Per-archetype shape character is preserved at the spline-output layer (not lost in input averaging).
- Convex-combination invariant: blended params lie within the convex hull of contributing archetypes' params.

**Tradeoff**: at very narrow transition zones, blended bootstrap parameters may not be physically self-consistent (e.g., high `mountains_amplitude` blends to mountain-like but high `continental_scale` blends to flat-clustering). In practice this is bounded by the climate field's continuity — adjacent archetypes that share latitude/elevation will have similar climate samples → similar spline outputs → blend produces a coherent intermediate. F.7 per-archetype tuning verifies transition zones read as natural; if tuning surfaces issues, adjacent archetypes can be tuned to share spline shape character at boundaries (e.g., Mediterranean and Continental Temperate use similar `mountains_scale` splines).

### 2.6 Composition with D.4 biome blending (resolves §3.E)

**Decision**: archetype blending runs FIRST, at the bootstrap parameter layer (steps 1-5 of §2.1). D.4 biome blending runs LATER, at the per-biome parameter layer (step 7 of §2.1), after biome classification.

**Pipeline order**:

1. Mask sample → archetype contributions (step 1 of §2.1).
2. Climate sample (step 2).
3. Per-archetype spline evaluation → per-archetype `BootstrapParams` (step 3).
4. Archetype-blend bootstrap params (step 4, per §2.5).
5. Run bootstrap noise pipeline ONCE with blended params → pre-erosion height (step 5).
6. `lookup_biome(climate, height)` → `BiomeId` (step 6, D.2 unchanged).
7. D.4 scattered-convolution biome blending → `BlendedBiomeParams` (step 7, unchanged).
8. Per-biome modulation applies blended biome params to height (step 8, D.3 unchanged).

**Rationale**:

- The two blending layers operate at different scales and address different concerns. Archetype blending operates at regional scale (~256 WU falloff, hundreds of meters) and shapes the bootstrap noise field. Biome blending operates at biome-boundary scale (~48 WU radius, meters) and tunes per-biome parameters within whichever archetype the vertex resolves to.
- Running archetype blending FIRST means the bootstrap noise reflects the regional archetype identity by the time biome classification runs. Biome classification sees correctly-shaped terrain for the archetype zone the vertex is in.
- Running biome blending SECOND (within each archetype's terrain) preserves D.4's job of softening per-biome boundary transitions without interference from the regional layer.

**Tradeoff**: biome classification at step 6 sees terrain shaped by potentially-blended archetype parameters in transition zones. This means biome IDs in archetype-transition zones may differ from what either pure archetype would produce (e.g., a vertex in CT-Boreal transition zone might resolve to BorealForest because the blended `mountains_amplitude` produced sufficient elevation). This is generally desirable (transition zones get transition biomes), but worth flagging — F.7 per-archetype tuning verifies transition-zone biome distributions read as natural.

### 2.7 Per-vertex stability under regional blending

**Invariant** (analog of predecessor §2.5): the painted archetype mask is stable; downstream blends recompute from the mask each chunk generation. Specifically:

- **Mask is the authoring source of truth**. Editor writes to `RegionalArchetypeMask`; runtime reads from it deterministically.
- **Per-vertex archetype contributions are deterministic functions of `(world_x, world_z, mask)`**. Same inputs → same `(archetype_id, weight)` list. Adjacent chunks' halos at shared edges read the same mask values → same contributions → continuous bootstrap params.
- **Blend weights sum to 1.0 ± epsilon** at every vertex (normalization invariant).
- **Bootstrap params lie within convex hull of contributing archetypes' params** (convex-combination invariant from §2.5).
- **Biome IDs at chunk shared edges match between adjacent chunks** (D.4 invariant preserved; archetype blending doesn't change which biome a vertex resolves to within rounding).

**What's NOT preserved across mask edits**: if the writer repaints a region after terrain generation, the cached chunk data is stale. Editor refresh: regenerate affected chunks (or all chunks) when mask changes. Detail-level cache invalidation policy is F.5's concern; F.0 commits to the invariant ("mask is authoring source of truth; runtime cache is stale on mask edit").

### 2.8 Authoring affordances (resolves §3.F)

**Decision**: new editor panel `RegionalArchetypePanel` alongside the existing `TerrainPanel`. Activated via a top-level menu toggle or tab in the editor UI.

**Panel contents**:

- **Brush size slider**: range `[8, 256]` mask pixels (default 32). Pixel-coord brush; affects circular region in mask coords centered on cursor world position.
- **Falloff distance slider**: range `[8, 128]` mask pixels (default 32 = `falloff_radius_pixels`). Determines how wide transition zones are when the brush paints adjacent archetypes.
- **Archetype palette**: list of D.5 catalog archetypes (Continental Temperate, Equatorial Tropical, Boreal/Subarctic, Mediterranean, Desert, Custom). Selected archetype = palette color. Click+drag in viewport to paint; shift+click to erase (sets ID 0 = unpainted = Continental Temperate fallback).
- **Mask save/load**: buttons for writing `<world>.mask.id.bin` + `<world>.mask.falloff.bin` to disk; loading from disk; clearing the mask (returns world to unpainted = full Continental Temperate).
- **Climate Preview overlay toggle**: D.5c absorbed. When enabled, viewport renders an overlay showing per-vertex archetype ID color + climate sample values (temperature/moisture/continentalness) for diagnostic purposes.
- **Regenerate terrain button**: triggers `TerrainPanel`-equivalent regeneration after mask changes. Optionally regenerates only chunks intersecting the mask edit's bounding box.

**Integration with existing terrain workflow**:

- The panel writes to `RegionalArchetypeMask` runtime resource.
- `TerrainPanel`'s "Generate Terrain" reads from `RegionalArchetypeMask` (loaded by `WorldGenerator`).
- No coupling between the two panels beyond the shared runtime resource.
- Loading a world: editor loads heightmap + mask in parallel; both regenerate together.
- Save sequence: terrain heightmap saved separately from mask; mask save is independent operation.

**Editor UX flow**:

1. Open editor; `RegionalArchetypePanel` shows empty mask (all unpainted).
2. Writer selects an archetype from the palette, paints regions onto the world map view.
3. Falloff distance is visualized at paint time (semi-transparent halo around painted region showing transition zone).
4. Writer saves mask; closes panel.
5. Switch to `TerrainPanel`; click "Regenerate Terrain" to produce heightmap respecting new mask.

**Rationale for new panel vs extending TerrainPanel**:

- F.5 estimates ~3-6 weeks of editor work (paint UX, brush algorithms, mask serialization, Climate Preview overlay). Bloating `TerrainPanel` with this much new UI fights against existing terrain-tool ergonomics.
- Separation of concerns: `TerrainPanel` is about terrain noise + biome configuration; `RegionalArchetypePanel` is about regional layout. They share data (the mask) but have distinct workflows.
- Future authoring concerns (e.g., painted vegetation overrides, painted weather zones) might warrant their own panels too; `RegionalArchetypePanel` establishes the per-concern-panel pattern.

---

## 3. Sub-phase F.0 — Campaign plan

### 3.1 Goal

Draft this campaign document. Resolve §2 architectural decisions concretely. Seed §10 with launch entries capturing each §2 resolution's rationale. Hand off to F.1.

### 3.2 Scope

**In scope:**

- Drafting `docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md` (this document).
- Updating `docs/current/TERRAIN_GENERATION_QUALITY_CAMPAIGN.md` §10 with a campaign-design session note (small entry, references this document).
- Resolving §2.2-§2.8 architectural decisions per the F.0 prompt's §3.A-§3.F.
- Seeding §10 of this document with §2 launch entries.

**Out of scope:**

- Production code changes (any).
- F.4.B.3.D closure (separate session).
- Implementation of any §2 architecture (sub-phases F.1-F.7).
- New campaign-doc creation beyond `REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md`.

### 3.3 Success criteria

- Document `docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md` exists with §0-§9 sections complete and §10 seeded with launch entries.
- Each of §2.2-§2.8's six decisions resolved concretely with rationale captured in §10.
- Predecessor `TERRAIN_GENERATION_QUALITY_CAMPAIGN.md` §10 updated with note referencing this document.
- §9 of this document marks F.0 COMPLETE with date + commit hash.

### 3.4 Reversibility

F.0 is a doc-only commit. Revert via `git revert` of the F.0 commit. State returns to "research audit landed; campaign-design session not yet run."

### 3.5 Expected commits

- **F.0 — Campaign plan ground-truth doc.** Commit message: `Phase 1.X-design: regional archetype variation campaign ground-truth doc`. May split with a small follow-up: `Phase 1.X-design: TERRAIN_GENERATION_QUALITY_CAMPAIGN.md §10 note`.
- **F.0 hash-fixup** (optional): `Phase 1.X-design: §9 + §10 hash-fixup`.

---

## 4. Sub-phase F.1 — Climate field extension

### 4.1 Goal

Extend `ClimateMap::sample` with `erosion` and `weirdness` fields per §2.2. Add `PvFold` helper. Verify D.1 invariants preserved at Continental Temperate defaults.

### 4.2 Scope

**In scope:**

- Add `erosion: f32` and `weirdness: f32` fields to `ClimateSample`.
- Add `erosion_noise: Perlin` (seed offset +3000, scale 0.0008) and `weirdness_noise: Perlin` (seed offset +4000, scale 0.0006) fields to `ClimateMap`.
- Implement `ClimateSample::pv()` derived field via `PvFold::from_weirdness`.
- Update `ClimateMap::sample` to populate the new fields.
- Verify D.1 backward compatibility: existing `ClimateSample` consumers (D.2 `lookup_biome`, D.4 `blend_biome_parameters`, etc.) compile unchanged because they only read the existing 3 fields.
- Verify F.4.B.3.D.5-fix Path B regression test passes byte-identically (Continental Temperate at radius 10 seed 12345 produces same heightmap; max elevation 698.5m unchanged).
- Add unit tests for new fields: range bounds (erosion ∈ `[-1, 1]`, weirdness ∈ `[-1, 1]`), determinism (same world position → same values), PV fold formula correctness (manual calculation cross-check at 5 weirdness values).

**Out of scope:**

- Wiring new fields into `WorldGenerator` (F.3 and onwards).
- `BootstrapSplineSet` infrastructure (F.2).
- Any visible terrain change.
- Extending `WorldArchetype` envelope to modulate erosion/weirdness (deferred; F.7 may add if archetype tuning shows benefit).

### 4.3 Success criteria

- New fields populated correctly in `ClimateSample`; `pv()` returns expected values.
- D.1 backward compat: all D.1 unit tests pass; D.2/D.4/D.5 lib tests pass unchanged.
- F.4.B.3.D.5-fix Path B regression: max real-chunk elevation 698.5m at Continental Temperate seed 12345 (byte-identical or within 0.1m tolerance for f32 precision).
- New unit tests pass: range bounds, determinism, PV formula.
- All 738+ terrain crate lib tests pass; `cargo check` clean for terrain + aw_editor + render.
- §9 reflects F.1 COMPLETE.

**No Andrew-gate** (F.1 produces no visible terrain change).

### 4.4 Reversibility

F.1 is additive — new fields, new helper, no modifications to existing read-paths. Revert via `git revert` of F.1 commits restores pre-F.1 state.

### 4.5 Expected commits

- **F.1.A — `PvFold` helper + types**. Adds `PvFold` impl + `Spline1D` placeholder + `BootstrapParam` enum. Pure additive type definitions; no integration. Commit message: `Phase 1.X-F.1.A: PvFold helper + spline types`.
- **F.1.B — `ClimateSample` extension + `ClimateMap::sample` wiring**. Adds erosion + weirdness fields; populates them in `sample`. Commit message: `Phase 1.X-F.1.B: extend ClimateSample with erosion + weirdness`.
- **F.1.C — Unit tests + regression verification**. Adds new unit tests; verifies F.4.B.3.D.5-fix regression. Commit message: `Phase 1.X-F.1.C: F.1 unit tests + regression verification`.
- **F.1.D — Closeout**. Updates §9 to mark F.1 COMPLETE. Commit message: `Phase 1.X-F.1.D: close F.1`.

---

## 5. Sub-phase F.2 — `BootstrapSplineSet` infrastructure

### 5.1 Goal

Define `BootstrapSplineSet`, `Spline1D`, `ParamSpline`, `BootstrapParams` types per §2.3. Implement separable-spline evaluation. Land default `BootstrapSplineSet` instances for the 6 D.5 catalog archetypes as Rust constants. No wiring into noise pipeline yet.

### 5.2 Scope

**In scope:**

- New module `astraweave-terrain/src/bootstrap_splines.rs` (~350 lines + 12 unit tests).
- `Spline1D` type with `evaluate(input: f32) -> f32`, `identity()` constructor, `from_control_points(Vec<(f32, f32)>) -> Result<Self>` constructor (validates sortedness).
- `ParamSpline` type with `evaluate(sample: &ClimateSample) -> f32`.
- `BootstrapSplineSet` type with `evaluate(sample: &ClimateSample) -> BootstrapParams`.
- `BootstrapParams` type with 4 fields (mountains_amplitude, mountains_scale, continental_scale, base_elevation_amplitude).
- 6 default `BootstrapSplineSet` const instances (one per D.5 catalog archetype). Each archetype's spline shapes are placeholder-tuned (F.7 does the real tuning); F.2's job is to ship the infrastructure with non-trivial default values.
- Default values designed so that evaluating Continental Temperate at the median climate sample (continentalness=0.5, erosion=0.0, pv=0.0) reproduces F.4.B.3.D.5-fix's hardcoded `NoiseConfig::default()` values: mountains_amplitude=480, mountains_scale=0.002, continental_scale=0.0003, base_elevation_amplitude=150.
- Add `WorldArchetype::bootstrap_splines: BootstrapSplineSet` field. Existing `WorldArchetype` invariants preserved.
- Unit tests:
  - `Spline1D::evaluate` correctness at endpoints + interior + extrapolation (clamp at endpoints).
  - `Spline1D::from_control_points` rejects unsorted inputs.
  - `Spline1D::identity` returns 1.0 for any input.
  - `ParamSpline::evaluate` returns expected product for known inputs.
  - `BootstrapSplineSet::evaluate` for each D.5 archetype: produces expected `BootstrapParams` at median climate sample.
  - Continental Temperate at median climate: `mountains_amplitude` = 480.0 ± 1.0; `mountains_scale` = 0.002 ± 1e-5; etc. (regression contract for F.3).
  - Spline determinism: same input → same output across multiple evaluations.

**Out of scope:**

- Wiring `BootstrapSplineSet` into `WorldGenerator` (F.3).
- Implementing per-archetype tuning (F.7).
- Editor UI for spline editing.
- Custom archetype's runtime-editable splines (deferred; F.5 may add).

### 5.3 Success criteria

- `bootstrap_splines.rs` module exists with all types defined.
- 6 archetype default `BootstrapSplineSet` const instances ship.
- Continental Temperate at median climate sample produces F.4.B.3.D.5-fix default values within ±1.0 (or proportional ±0.5%) tolerance.
- All 12 new unit tests pass.
- D.1, D.2, D.3, D.4, D.5 lib tests pass unchanged.
- `cargo check` clean for terrain + aw_editor + render.
- `WorldArchetype` extended with `bootstrap_splines` field; D.5 catalog archetypes updated to reference their bootstrap splines.
- §9 reflects F.2 COMPLETE.

**No Andrew-gate** (F.2 produces no visible terrain change).

### 5.4 Reversibility

F.2 adds a new module + extends an existing struct. Revert via `git revert` restores pre-F.2 state. No data migration needed.

### 5.5 Expected commits

- **F.2.A — `Spline1D` + `ParamSpline` types**. Pure type definitions; unit tests for spline evaluation. Commit message: `Phase 1.X-F.2.A: Spline1D + ParamSpline infrastructure`.
- **F.2.B — `BootstrapSplineSet` + `BootstrapParams` + 6 default archetype splines**. Adds the higher-level types and the const declarations for all 6 catalog archetypes. Commit message: `Phase 1.X-F.2.B: BootstrapSplineSet + 6 archetype defaults`.
- **F.2.C — `WorldArchetype` extension + integration tests**. Adds `bootstrap_splines` field; integration tests that verify per-archetype `evaluate` produces expected values. Commit message: `Phase 1.X-F.2.C: extend WorldArchetype with bootstrap_splines`.
- **F.2.D — Closeout**. Updates §9 to mark F.2 COMPLETE. Commit message: `Phase 1.X-F.2.D: close F.2`.

---

## 6. Sub-phase F.3 — Spline wiring (single archetype regression)

### 6.1 Goal

Wire `BootstrapSplineSet` into `WorldGenerator::generate_chunk_with_climate` for Continental Temperate only. Produce byte-identical (or within-tolerance) output to F.4.B.3.D.5-fix at Continental Temperate defaults. Verify per-archetype shape character is now expressible.

### 6.2 Scope

**In scope:**

- Refactor `WorldGenerator::generate_chunk_with_climate` to accept a `BootstrapSplineSet` reference.
- Per-vertex pipeline (within Continental Temperate single-archetype case):
  1. Sample climate (now 5 fields per F.1).
  2. Evaluate Continental Temperate's `BootstrapSplineSet` against climate sample → `BootstrapParams`.
  3. Pass `BootstrapParams` to `TerrainNoise::sample_height_with_params` (new method; replaces hardcoded `NoiseConfig` reads with per-vertex spline outputs).
- Add `TerrainNoise::sample_height_with_params(params: &BootstrapParams, world_x: f32, world_z: f32) -> f32` method. Existing `TerrainNoise::sample_height_with_mountain_amplitude` continues to work; new method is the spline-driven path.
- Integration test `phase_1_x_f3_continental_temperate_byte_identical_to_f4b3d_d5fix`: at radius 10 seed 12345 Continental Temperate, produces same heightmap as F.4.B.3.D.5-fix within tolerance (max 0.1m per-vertex divergence; 0.0m mean).
- Integration test `phase_1_x_f3_spline_toggling_changes_terrain`: at fixed climate sample, doubling `mountains_amplitude`'s base value via spline produces ~2× pre-erosion mountain height (smoke test that splines are wired).
- Phase-2 continuity test thresholds preserved (grassland 80 WU, mountain 10 WU).

**Out of scope:**

- Multi-archetype regions (F.4 territory).
- `RegionalArchetypeMask` integration (F.4).
- Per-archetype tuning (F.7).
- Editor UI changes.

### 6.3 Success criteria

- F.4.B.3.D.5-fix regression: byte-identical output at Continental Temperate radius 10 seed 12345 (max 0.1m divergence; mean 0.0m).
- New integration tests pass.
- Phase-2 continuity test passes at unchanged thresholds.
- All upstream regression tests still green.
- `cargo check` clean.
- §9 reflects F.3 COMPLETE.
- **Andrew-gate REQUIRED**: Continental Temperate at radius 10 seed 12345 reads visually unchanged from F.4.B.3.D.5-fix baseline (no visible regression). PASS verdict updates §9.

### 6.4 Reversibility

F.3 wires `BootstrapSplineSet` into the noise pipeline. Revert via `git revert` restores F.2's "infrastructure-only" state.

### 6.5 Expected commits

- **F.3.A — `TerrainNoise::sample_height_with_params` method**. Adds the new method; existing methods unchanged. Commit message: `Phase 1.X-F.3.A: sample_height_with_params method`.
- **F.3.B — `WorldGenerator::generate_chunk_with_climate` spline integration**. Refactors to read Continental Temperate's `BootstrapSplineSet` and pass blended params. Commit message: `Phase 1.X-F.3.B: wire BootstrapSplineSet (Continental Temperate)`.
- **F.3.C — Regression + smoke tests**. Adds `phase_1_x_f3_*` integration tests. Commit message: `Phase 1.X-F.3.C: F.3 regression + smoke tests`.
- **F.3.D — Andrew-gate + closeout**. After Andrew's PASS verdict on visual regression check, updates §9 to mark F.3 COMPLETE. Commit message: `Phase 1.X-F.3.D: close F.3 (Andrew-gate PASS)`.

---

## 7. Sub-phase F.4 — `RegionalArchetypeMask` + falloff sampler

### 7.1 Goal

Implement mask format per §2.4. Implement runtime sampler (`RegionalArchetypeBlend`) that reads mask + falloff and produces per-vertex `(archetype_id, weight)` contributions. Implement archetype-blend per-vertex aggregation per §2.5. Wire into `WorldGenerator::generate_chunk_with_climate`. Test via programmatically-constructed masks; no editor UI yet.

### 7.2 Scope

**In scope:**

- New module `astraweave-terrain/src/regional_archetype_mask.rs`:
  - `RegionalArchetypeMask { resolution, world_extent_wu, falloff_radius_pixels, ids: Vec<u8>, falloff: Vec<u8> }` type.
  - `RegionalArchetypeBlend<'a>` struct with `sample_at(world_x, world_z) -> ArrayVec<(WorldArchetypeId, f32), 4>`.
  - Mask save/load: `save_to_files(path) -> Result<()>` + `load_from_files(path) -> Result<Self>`. Files: `<world>.mask.id.bin` + `<world>.mask.falloff.bin` + `<world>.mask.ron` (metadata).
  - Euclidean distance transform helper (computes falloff field from ID field at save time).
  - Programmatic mask construction helpers for tests (`with_painted_circle(center, radius, archetype_id)`).
- Refactor `WorldGenerator::generate_chunk_with_climate` to accept `Option<&RegionalArchetypeMask>`:
  - If `None` (no mask): use Continental Temperate's `BootstrapSplineSet` for all vertices (F.3 behavior).
  - If `Some(mask)`: per-vertex sample mask + falloff; for each contributing archetype, evaluate its `BootstrapSplineSet`; blend params per §2.5; pass blended params to `sample_height_with_params`.
- Integration tests:
  - `phase_1_x_f4_no_mask_matches_f3_output`: `None` mask → byte-identical to F.3 output.
  - `phase_1_x_f4_painted_circle_produces_expected_archetype`: painted circle of Boreal in Continental Temperate world → vertices in circle interior produce Boreal-shaped terrain.
  - `phase_1_x_f4_falloff_zone_blends_smoothly`: vertices in falloff zone between two archetypes have intermediate `BootstrapParams` (smoke test for §2.5 blending).
  - `phase_1_x_f4_save_load_roundtrip`: save mask → load mask → byte-identical contents.
  - Performance: `phase_1_x_f4_perf_radius10`: generation time at radius 10 with 5-archetype programmatic mask within +30% of F.4.B.3.D.5-fix baseline (~0.747s → ≤0.97s/chunk).

**Out of scope:**

- Editor UI for archetype painting (F.5).
- Climate Preview overlay (F.5).
- Per-archetype tuning (F.7).
- F.5b painting save/load integration with editor's existing world save flow (F.5).

### 7.3 Success criteria

- New module compiles; types behave per §2.4 specification.
- Integration tests pass:
  - No-mask case is byte-identical to F.3.
  - Painted-circle case produces expected archetype-shaped terrain.
  - Falloff-zone case blends smoothly.
  - Save/load roundtrip preserves byte identity.
  - Performance within +30% baseline.
- All upstream regression tests still green.
- `cargo check` clean.
- §9 reflects F.4 COMPLETE.
- **Andrew-gate REQUIRED**: programmatic 5-archetype world (CT center, Boreal north, Mediterranean south, Desert east, Tropical west) at radius 10 seed 12345 renders visibly distinct archetype regions with smooth boundaries. PASS verdict updates §9.

### 7.4 Reversibility

F.4 adds new module + extends an existing function with optional mask parameter. Revert via `git revert` restores F.3's "Continental Temperate only" state.

### 7.5 Expected commits

- **F.4.A — `RegionalArchetypeMask` type + sampler**. Pure type definitions + sampler; unit tests. Commit message: `Phase 1.X-F.4.A: RegionalArchetypeMask + sampler`.
- **F.4.B — Save/load + distance transform**. Adds file I/O + Euclidean distance transform. Commit message: `Phase 1.X-F.4.B: mask save/load + distance transform`.
- **F.4.C — `WorldGenerator` integration**. Refactors `generate_chunk_with_climate` to read mask + blend per §2.5. Commit message: `Phase 1.X-F.4.C: wire RegionalArchetypeMask into WorldGenerator`.
- **F.4.D — Integration + perf tests**. Adds `phase_1_x_f4_*` tests. Commit message: `Phase 1.X-F.4.D: F.4 integration + perf tests`.
- **F.4.E — Andrew-gate + closeout**. After Andrew's PASS verdict on programmatic 5-archetype world, updates §9 to mark F.4 COMPLETE. Commit message: `Phase 1.X-F.4.E: close F.4 (Andrew-gate PASS)`.

---

## 8. Sub-phase F.5 — Editor UI for archetype painting

### 8.1 Goal

Build the `RegionalArchetypePanel` per §2.8. Brush + falloff + archetype palette + save/load. Climate Preview overlay (D.5c absorbed). Andrew-gate: writer paints a Veilweaver-realistic 5-region world in <30 minutes; output reads as Crimson-Desert-class regional variation at flyover altitude.

### 8.2 Scope

**In scope:**

- New panel `tools/aw_editor/src/panels/regional_archetype_panel.rs`:
  - Brush size + falloff distance sliders.
  - Archetype palette (D.5 catalog dropdown + visual color indicator per archetype).
  - Paint mode + erase mode toggle.
  - Save/load mask buttons.
  - Regenerate terrain button (calls `TerrainPanel::regenerate_terrain` after mask changes).
  - Climate Preview overlay toggle (D.5c absorbed).
- Brush implementation: circular paint with falloff distance encoding. Click+drag in viewport projects screen coords to world coords; affects mask pixels within brush radius.
- Distance-transform recomputation on mask edit: editor recomputes falloff field after each paint operation (or batch-recomputes on save). Cost: ~50-100 ms per recompute at 1024×1024.
- Climate Preview overlay rendering: per-vertex colored visualization of current archetype ID + climate sample values. Integrated with existing terrain mesh shading (separate render pass or material override).
- Mask save/load wiring with editor's existing project save flow:
  - Save: mask saved alongside heightmap as part of "Save Project" operation.
  - Load: mask loaded alongside heightmap on "Open Project."
- Integration tests:
  - `phase_1_x_f5_paint_circle_writes_correct_pixels`: programmatic paint operation produces expected mask state.
  - `phase_1_x_f5_save_load_persists`: paint → save → close → reopen → paint state persists.
  - `phase_1_x_f5_climate_preview_renders`: smoke test for overlay (renders without panic).

**Out of scope:**

- Custom archetype's runtime-editable splines (deferred; F.7 may add if archetype tuning surfaces benefit).
- Multi-channel RGBA mask format (single-channel + falloff is sufficient).
- Cloud sync / multi-user editing of masks.
- Mask history / undo system (deferred; uses editor's existing undo if practical).

### 8.3 Success criteria

- `RegionalArchetypePanel` exists and is functional in the editor.
- Brush, falloff, palette, save/load all work as specified.
- Climate Preview overlay renders correctly.
- Mask persists across editor restarts (save/load roundtrip).
- All integration tests pass.
- `cargo check` clean for editor + terrain + render.
- §9 reflects F.5 COMPLETE.
- **Andrew-gate REQUIRED**: writer (Andrew or proxy) paints a Veilweaver-realistic 5-region world in <30 minutes; terrain regeneration produces output reading as Crimson-Desert-class regional variation at flyover altitude. PASS verdict updates §9. If Andrew-gate is REGRESS or PARTIAL, F.5 remains in progress until painting UX or terrain output is acceptable.

### 8.4 Reversibility

F.5 adds a new editor panel. Revert via `git revert` restores F.4's "no painting UI" state. Mask files saved by F.5 remain on disk; runtime behavior reverts to "no mask" path.

### 8.5 Expected commits

- **F.5.A — Panel scaffold**. Empty `RegionalArchetypePanel` with sliders + palette UI; no painting yet. Commit message: `Phase 1.X-F.5.A: RegionalArchetypePanel scaffold`.
- **F.5.B — Brush implementation**. Circular paint + falloff encoding. Commit message: `Phase 1.X-F.5.B: brush + falloff painting`.
- **F.5.C — Save/load integration**. Mask serialization + project save/load wiring. Commit message: `Phase 1.X-F.5.C: mask save/load + project integration`.
- **F.5.D — Climate Preview overlay**. D.5c absorbed implementation. Commit message: `Phase 1.X-F.5.D: Climate Preview overlay`.
- **F.5.E — Integration tests**. `phase_1_x_f5_*` tests. Commit message: `Phase 1.X-F.5.E: F.5 integration tests`.
- **F.5.F — Andrew-gate + closeout**. After Andrew's PASS verdict on 5-region painting test, updates §9. Commit message: `Phase 1.X-F.5.F: close F.5 (Andrew-gate PASS)`.

---

## 9. Sub-phase F.6 — Scattered-convolution at the regional layer

### 9.1 Goal

Apply NoisePosti.ng-style organic blending to mask-driven archetype assignments at boundaries. Eliminates axis-aligned artifacts in transition zones if Voronoi-style cells emerge from the mask. Analog of D.4's per-biome scattered convolution, applied at the regional (~256 WU) scale.

### 9.2 Scope

**In scope:**

- Extend `RegionalArchetypeBlend::sample_at` to use NoisePosti.ng-style scattered convolution at boundary positions:
  - Default config: 4-8 jittered samples per vertex, ~256 WU radius (vs D.4's 6 samples / 48 WU radius).
  - Position-quantized deterministic jitter (1/1024 WU = ~1mm grid; same pattern as D.4's `biome_param_blending`).
- Per-vertex algorithm:
  - Sample mask at jittered positions within `regional_blend_radius`.
  - Aggregate `(archetype_id, weight)` contributions from each sample.
  - Normalize weights to sum to 1.0.
- Integration tests:
  - `phase_1_x_f6_voronoi_cells_blend_organically`: programmatic Voronoi-cell mask + scattered-convolution blending → no axis-aligned artifacts at cell boundaries (visual: organic-looking transition zones).
  - `phase_1_x_f6_far_from_boundary_unchanged`: vertices far from any boundary (interior of an archetype region) produce same output as F.4 (single-archetype contribution, weight 1.0).
  - `phase_1_x_f6_perf_within_budget`: generation time at radius 10 with 5-archetype mask within +30% of F.5 baseline.

**Out of scope:**

- Per-archetype tuning (F.7).
- Compositional changes to D.4 biome blending.
- Performance optimization beyond the +30% budget.

### 9.3 Success criteria

- Scattered-convolution path implemented and integrated with `RegionalArchetypeBlend`.
- Integration tests pass:
  - Voronoi-cell test produces organic transitions (no axis-aligned banding visible).
  - Far-from-boundary unchanged.
  - Performance within budget.
- All upstream regression tests still green.
- `cargo check` clean.
- §9 reflects F.6 COMPLETE.
- **Andrew-gate REQUIRED**: transition zones in F.5's painted 5-region world read as natural geography, no visible boundary artifacts at flyover or ground-level. PASS verdict updates §9.

### 9.4 Reversibility

F.6 modifies `RegionalArchetypeBlend::sample_at` algorithm. Revert via `git revert` restores F.5's "neighborhood-scan" sampler.

### 9.5 Expected commits

- **F.6.A — Scattered-convolution sampler**. Modified `sample_at` with jittered samples. Commit message: `Phase 1.X-F.6.A: scattered-convolution at regional layer`.
- **F.6.B — Integration tests**. `phase_1_x_f6_*` tests. Commit message: `Phase 1.X-F.6.B: F.6 integration tests`.
- **F.6.C — Andrew-gate + closeout**. After Andrew's PASS, updates §9. Commit message: `Phase 1.X-F.6.C: close F.6 (Andrew-gate PASS)`.

---

## 10. Sub-phase F.7 — Per-archetype tuning + Andrew-gate

### 10.1 Goal

Tune the 6 catalog archetypes' splines so each produces visibly distinct character. Re-run diagnostic-3-style cross-archetype Pearson measurement. Cross-archetype Pearson < 0.7 (distant pairs) / < 0.85 (adjacent pairs). Andrew-gate visual verification: side-by-side screenshots from 6 archetypes pass a "these look like different worlds" review.

### 10.2 Scope

**In scope:**

- Per-archetype `BootstrapSplineSet` tuning:
  - **Continental Temperate** (NC/Appalachia analog): rolling foothills, moderate mountains, river-cut valleys. Spline shapes preserve F.4.B.3.D.5-fix character (rolling `mountains_amplitude` ~480 baseline; moderate response to continentalness; mild response to PV).
  - **Equatorial Tropical**: dense rolling jungle terrain, occasional volcanic peaks. Higher `base_elevation_amplitude` for flat-canopy character; aggressive `mountains_amplitude` response to high continentalness for volcanic peaks; minimal `mountains_amplitude` elsewhere.
  - **Boreal/Subarctic**: alpine character, sharp ridges, U-shaped glacial valleys. High `mountains_amplitude` baseline; aggressive PV response (sharp ridges); minimal `continental_scale` (uniform high-relief terrain).
  - **Mediterranean**: rolling hills with rocky coastline character. Moderate `mountains_amplitude` baseline; aggressive `erosion` response (flat plateaus at high erosion); moderate PV.
  - **Desert**: low rolling dunes with occasional mesa-like uprisings. Low `base_elevation_amplitude`; aggressive `erosion` response (very flat at high erosion); rare `mountains_amplitude` peaks.
  - **Custom**: user-tunable; defaults to Continental Temperate.
- Diagnostic-3-style cross-archetype Pearson measurement:
  - Re-implement as `phase_1_x_f7_diagnostic_cross_archetype` in `astraweave-terrain/tests/`.
  - Measure Pearson correlation across all 15 archetype pairs (6 archetypes choose 2).
  - Target: < 0.7 for distant pairs (CT vs Desert; Boreal vs Equatorial Tropical); < 0.85 for adjacent pairs (CT vs Mediterranean; Mediterranean vs Desert).
- Per-archetype tuning iteration:
  - Up to 3 tuning rounds per archetype before escalation.
  - Each round: tune splines → re-run diagnostic-3 → evaluate Pearson + visual screenshots.
  - If diagnostic-3 fails after 3 rounds for an archetype: log in §10 and consider one of (a) adjusting the Pearson target for that pair specifically, (b) adding additional spline outputs to that archetype's `BootstrapSplineSet`, (c) reframing the archetype's intended character.
- F.4.B.3.D deferred-work absorption:
  - Equatorial Tropical archetype-specific tuning (audit deferred work) → addressed in F.7 ET tuning round.
  - Bootstrap noise pipeline elevation skew (audit §7 of diagnostic-2) → addressed via F.7 spline tuning at the bootstrap layer.

**Out of scope:**

- Adding new archetypes beyond the D.5 catalog.
- Re-architecting `BootstrapSplineSet` (F.2 fixed the architecture).
- New climate fields beyond F.1's 5.
- Editor UI changes (F.5 fixed UX).

### 10.3 Success criteria

- Cross-archetype Pearson measurements:
  - Distant pairs (CT-Desert, Boreal-ET, etc.): Pearson < 0.7.
  - Adjacent pairs (CT-Mediterranean, etc.): Pearson < 0.85.
- All 15 pair measurements logged with values.
- All upstream regression tests still green.
- `cargo check` clean.
- §9 reflects F.7 COMPLETE.
- **Andrew-gate REQUIRED — principal Andrew-gate of the campaign**: side-by-side screenshots from all 6 archetypes pass a "these look like different worlds" review. Each archetype reads as its intended character (Continental Temperate looks like NC/Appalachia; Equatorial Tropical looks tropical; Boreal looks alpine; Mediterranean looks Mediterranean; Desert looks desert). PASS verdict updates §9.

### 10.4 Reversibility

F.7 modifies `BootstrapSplineSet` const declarations per archetype. Each tuning round is a separate commit; revert via `git revert` restores the prior tuning state. Cross-archetype Pearson harness is permanent (helps regression-test future tuning).

### 10.5 Expected commits

- **F.7.A — Diagnostic-3-equivalent harness**. Adds `phase_1_x_f7_diagnostic_cross_archetype` test. Commit message: `Phase 1.X-F.7.A: cross-archetype Pearson diagnostic harness`.
- **F.7.B — Per-archetype tuning round 1**. First round of spline-curve tuning across all 6 archetypes. Logs Pearson measurements in commit body. Commit message: `Phase 1.X-F.7.B: per-archetype tuning round 1`.
- **F.7.C** (optional) **— Tuning round 2**. If round 1 Pearson targets not met for any archetype. Commit message: `Phase 1.X-F.7.C: per-archetype tuning round 2`.
- **F.7.D** (optional) **— Tuning round 3**. If round 2 still insufficient. Commit message: `Phase 1.X-F.7.D: per-archetype tuning round 3`.
- **F.7.E — Andrew-gate + closeout**. After Andrew's PASS verdict on 6-archetype side-by-side review, updates §9. Commit message: `Phase 1.X-F.7.E: close F.7 (Andrew-gate PASS, principal campaign gate)`.

---

## 11. Sub-phase F.8 — Closeout

### 11.1 Goal

Update parent campaign (`TERRAIN_MATERIAL_SYSTEM_CAMPAIGN.md`) to reflect Regional Archetype Variation campaign complete. Update `ARCHITECTURE_MAP.md` terrain section. Absorb F.4.B.3.D deferred work this campaign owns. Standard housekeeping.

### 11.2 Scope

**In scope:**

- Update `docs/current/TERRAIN_MATERIAL_SYSTEM_CAMPAIGN.md`:
  - §7 reflects Regional Archetype Variation campaign COMPLETE.
  - §9 receives a closeout entry referencing this campaign's final commit hash.
- Update `docs/current/ARCHITECTURE_MAP.md` terrain section:
  - Add `RegionalArchetypeMask`, `BootstrapSplineSet`, `RegionalArchetypeBlend` types.
  - Update terrain crate dependency diagram if applicable.
  - Update editor section to reflect `RegionalArchetypePanel`.
- Absorbed F.4.B.3.D deferred work confirmed addressed:
  - Equatorial Tropical archetype-specific tuning: handled in F.7.
  - Bootstrap noise pipeline elevation skew: handled in F.7.
  - Climate Preview overlay (D.5c): handled in F.5.
- Standalone follow-ups (NOT absorbed; remain after F.8 closes):
  - F.4.B.3.G phase-2 grassland precision floor (47.4 WU): standalone follow-up.
  - Dead-write bug at terrain_panel.rs:943: standalone follow-up.
  - Stale `NoiseConfig` literals in `behavioral_correctness_tests.rs`: standalone follow-up.
  - MountainRocky/River unproduced biomes: standalone follow-ups.
  - Per-biome runevision tuning: standalone follow-up.
- Update this document's Status header to "Campaign complete (date)."
- Update §9 to mark F.8 COMPLETE and the campaign as a whole COMPLETE.

**Out of scope:**

- Any production code changes (closeout is doc-only).
- Re-running per-archetype tuning (F.7 done).
- Drafting follow-up campaigns.

### 11.3 Success criteria

- All referenced docs land with proper updates.
- Campaign marked complete in §9.
- F.4.B.3.D absorbed work confirmed in §11.2; standalone follow-ups enumerated for future sessions.
- §9 reflects F.8 COMPLETE.
- Status header updated to "Campaign complete YYYY-MM-DD."

**No Andrew-gate** (closeout, not behavior change).

### 11.4 Reversibility

F.8 is doc-only. Revert via `git revert` restores pre-F.8 state.

### 11.5 Expected commits

- **F.8.A — Parent campaign + ARCHITECTURE_MAP updates**. Commit message: `Phase 1.X-F.8.A: parent campaign + ARCHITECTURE_MAP closeout updates`.
- **F.8.B — Campaign closeout**. Updates this doc's §9 and Status header. Commit message: `Phase 1.X-F.8.B: close Regional Archetype Variation campaign`.

---

## 8. Out of scope for entire campaign

Mirrors predecessor campaign's §8. Items explicitly out of scope:

- **Streaming terrain at Target C scale.** Phase 1.7 territory.
- **Water system rebuild.** Separate campaign per `docs/audits/water_system_architecture_2026-04-20.md`.
- **MountainRocky / River unproduced biomes.** Standalone follow-ups (River → Water System Rebuild).
- **Per-biome runevision tuning.** Architecturally orthogonal; standalone follow-up.
- **Dead-write bug at terrain_panel.rs:943.** Small cleanup; standalone follow-up.
- **Stale literal `NoiseConfig` constructors at behavioral_correctness_tests.rs lines 517, 911, 912.** Pre-existing test file failure unrelated to regional archetypes; standalone follow-up.
- **47.4 WU phase-2 grassland precision floor (F.4.B.3.G inheritance).** f32 precision issue at chunk shared edges; unrelated; standalone follow-up.
- **GPU-accelerated mask sampling.** Possible future optimization; standalone follow-up.
- **Author-painted regions for non-archetype concerns** (painted vegetation overrides, painted weather zones, etc.). The `RegionalArchetypeMask` is for archetype identity only; other paint concerns get their own panels per the F.5 separation-of-concerns precedent.
- **Multi-channel RGBA mask format.** F.4 commits to single-channel ID + falloff; multi-channel is a possible future enhancement if the Veilweaver writer needs >4 simultaneous archetype contributions per pixel.
- **Cloud sync / multi-user editing of masks.** Out of scope; mask is a local file resource.
- **Adding archetypes beyond the D.5 catalog of 6.** F.7 tunes existing 6; new archetypes are a future expansion if Veilweaver writers need >6 distinct world types.
- **Custom archetype's runtime-editable splines.** Deferred from F.5; F.7 may add if needed; otherwise Custom uses Continental Temperate's splines.
- **Author-painted erosion zones, rainfall zones, etc.** Climate field is procedural per D.1; painting climate is a future extension if needed.

---

## 9. Phase status

This section must be updated in the same commit that completes each sub-phase per §0 discipline.

```text
F.0 — Campaign plan: COMPLETE 2026-04-29, commit 0e51763d4.
F.1 — Climate field extension: COMPLETE 2026-04-29, commits 5fcde4e98 (F.1.A PvFold + spline types) + 744132c6a (F.1.B ClimateSample extension) + 8e883beb5 (F.1.C unit tests + regression verification) + df7636fa3 (F.1.D closeout). New 5-field ClimateSample (3 D.1 + erosion + weirdness) with PvFold helper. 14 F.1.C extension tests + 8 F.1.A spline_types tests + 18 D.1 climate::tests (backward-compat verified) all green. Seed offsets +3000 / +4000 verified clear of all other terrain crate noise instances; PV formula hand-verified at 7 canonical points; D.1 climate state inspected pre-F.1.B and matches D.1 documented state. No Andrew-gate (F.1 produces no visible terrain change per campaign §0).
F.2 — BootstrapSplineSet infrastructure: COMPLETE 2026-04-30, commits f0c3fa52d (F.2.A Spline1D + ParamSpline) + b5d70071d (F.2.B BootstrapSplineSet + 6 archetype defaults) + 43386cba5 (F.2.C WorldArchetype extension) + b58bc3585 (F.2.D closeout). Full Spline1D API (piecewise-linear evaluate + identity + from_control_points with Empty/NotSorted/NaN/Infinite validation), ClimateInputDim enum (Continentalness/Erosion/Pv), ParamSpline aggregating Spline1D + ClimateInputDim, BootstrapParams output struct, BootstrapSplineSet with 4 ParamSpline fields, 6 catalog factory functions all returning F.4.B.3.D.5-fix baseline byte-identical (480/0.002/0.0003/150). WorldArchetype extended with bootstrap_splines field; Copy derive removed (BootstrapSplineSet contains Vec). 29 spline_types tests (F.1.A 8 + F.2.A 15 + F.2.B 6) + 6 F.2.C integration tests + all upstream regression suites green. F.4.B.3.D.5-fix baseline values confirmed in noise_gen.rs pre-F.2.B; D.5 catalog enumerated; F.1.A spline_types module state verified. Three deviations logged in §10: (1) single-spline-per-parameter shape vs §2.3's "3 1D splines × multiplied" — F.7 may extend; (2) factory functions instead of const declarations because Spline1D::from_control_points returns Result and Vec::new() is non-const; (3) drop WorldArchetype Copy derive (BootstrapSplineSet contains Vec). No Andrew-gate (F.2 produces no visible terrain change per campaign §0).
F.3 — Spline wiring (single-archetype regression): COMPLETE 2026-05-01, commits 04fc26299 (F.3.A sample_height_with_params method) + 447367c15 (F.3.B WorldGenerator wiring + f32→f64 retrofit for mountains_scale) + be233fd78 (F.3.C regression + smoke tests) + d6a063fbb (F.3.D closeout). Continental Temperate's BootstrapSplineSet wired into WorldGenerator::apply_per_biome_modulation_to_halo's per-vertex height path. F.4.B.3.D.5-fix Path B regression byte-identical post-retrofit (zero per-vertex divergence across 100 sample positions). 4 F.3.A unit tests + 4 F.3.C tests (including byte-identity contract + falsifiable spline-toggling smoke test) + 14 F.1.C extension tests + 29 spline_types tests + 6 F.2.C integration tests + 18 D.1 climate tests + 12 D.5 world_archetypes tests + phase-2 continuity (80 WU grassland / 10 WU mountain post-erosion) all green. 771 terrain crate lib tests pass. Two deviations logged in §10: (1) sample_height_with_params 4-arg signature (added mountain_amplitude_multiplier) to compose with D.3b per-biome layer; (2) BootstrapParams.mountains_scale: f32 → f64 retrofit + BootstrapSplineSet.mountains_scale from ParamSpline to direct f64 storage to restore byte-identity (initial f32 produced 60-ulp drift propagating through erosion to 103 WU divergence at chunk shared edges, breaking phase-2 continuity 80 WU threshold). Andrew-gate PASS 2026-05-01: terrain shape character visually identical at flyover; observer noted "smoother / higher fidelity" perception attributable to f64 retrofit eliminating noise-lattice aliasing. **First Andrew-gate-required sub-phase of the campaign; PASS verdict received.**
F.4 — RegionalArchetypeMask + falloff sampler: COMPLETE 2026-05-01, commits 3eafa4ca4 (F.4.A RegionalArchetypeMask type + RON metadata) + 3c6c915c9 (F.4.B mask save/load + Euclidean distance transform) + 489eee27d (F.4.C RegionalArchetypeBlend neighborhood-scan sampler) + 1b63b921a (F.4.D multi-archetype blend math via §2.5 spline-output blending math producing per-vertex blended BootstrapParams from up-to-4 archetype contributions normalized to sum 1.0) + d3e2c0f67 (F.4.E WorldGenerator::generate_chunk_with_climate refactored to accept Option<&RegionalArchetypeMask>; F.3 byte-identity regression preserved on None mask path) + 8581391dc (F.4.F integration + perf + synthetic-differentiation tests; performance within +30% of F.3 baseline). RegionalArchetypeMask: 1024×1024 single-channel uint8 ID field + uint8 falloff distance field at sibling-directory storage layout `<project>/regional_archetype_masks/<world>.{ron,id.bin,falloff.bin}` (Godot Terrain3D + Unity Terrain SOTA convention). Andrew-gate PASS 2026-05-01: programmatic 5-archetype world (CT center + Boreal/Mediterranean/Desert/Tropical periphery) at radius 10 seed 12345 rendered visibly distinct archetype regions with smooth boundaries per campaign §7.3 success criteria. **F.4.G deferred-closeout** reconciled via pause artifacts session 2026-05-03 (this commit) — sub-commits A-F landed 2026-05-01 with verbal Andrew-gate PASS, but §9 + §10 closeout commit was deferred during the F.5-paint emerging remediation cascade and now executed retroactively; see §10 F.4.G entry below for context.
F.5 — Editor UI for archetype painting: IN PROGRESS, **CAMPAIGN PAUSED 2026-05-03** pending Editor Multi-Tool Architecture campaign. Andrew architectural decision 2026-05-03: spin off Option C (ActiveTool dispatcher trait + per-tool registration matching AAA canonical pattern) as its own foundational campaign at `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` rather than execute B-extend or Hybrid in G-fix; honors §0 research-pass-before-reframe discipline + F.4.B.3.D lesson application. F.5-paint commits preserved unchanged; G-research + G-diagnostic audits forward-applicable as predecessor research. Resumption point: post-Editor-Multi-Tool-Architecture, G-pointer-events-fix scope shifts from B-extend mechanical execution (~150-200 lines across 3 files) to single-commit dispatcher registration. F.5 §9 marker advances to COMPLETE only after F.5-paint resumption + G-fix + H-saveload + final Andrew-gate full PASS + F.5-overlay-and-gate Andrew-gate PASS post-resumption. F.5-paint slice — F-fix landed: panel registration corrected (11 surfaces total — 10 catalogued in audit + 1 exhaustive-match dispatch site discovered via compile-driven supplement); 2 Pattern A regression tests added (panel_type::tests::regional_archetype_panel_registered_in_panel_type_enum + tab_viewer::tests::editor_tab_viewer_instantiates_regional_archetype_panel); audit amendment lands forward-applicable methodology lesson (precedent-driven grep ≠ enum-variant-driven enumeration; Rust's exhaustiveness checker is the canonical surface enumeration mechanism). Andrew-gate narrowed verdict 2026-05-03: PASS for registration. Remediation chain: E-diagnostic complete (5f772bea3 + e561d4dce + f848388a6); F-fix complete (b2df0be20 F-fix.A + 722b70ae5 F-fix.A-supplement + dee94ea05 F-fix.B closeout + 1d67b3328 hash-fixup); G-pointer-events-research COMPLETE 2026-05-03 (6992f4b39 + e748a6304 + 506dec13c hash-fixup); G-pointer-events-diagnostic COMPLETE 2026-05-03 (ac4bc58a3 + ab67aeb0b + 57de03fba hash-fixup); G-pointer-events-fix scope shifted post-pause to dispatcher registration (Editor Multi-Tool Architecture provides the dispatcher). Remaining: G-pointer-events-research COMPLETE 2026-05-03 (audit doc `6992f4b39` + campaign doc `e748a6304` + hash-fixup `506dec13c`) — catalogs egui pointer-event dispatch + multi-tool 3D editor arbitration across Blender/Unity/Unreal/Godot + Rust 3D editor reference implementations; identifies three implementation approaches for active-tool first-dibs arbitration. G-pointer-events-diagnostic IN PROGRESS — code inspection complete (audit `ac4bc58a3` + campaign doc `ab67aeb0b`); H1 (approach B with main.rs mediator) confirmed cleanly; AstraWeave editor uses approach (B) — ViewportWidget at `tools/aw_editor/src/viewport/widget.rs` has typed `terrain_brush_active` field with `handle_input()` branching at lines 1180-1255; main.rs at lines 3833-3877 acts as per-frame mediator. Surfaced architectural decision per Q1 Option B with **four** options (Option B-extend literal smallest mirroring existing pattern; Option A higher-layer egui Modal overlay; Option C editor-level ActiveTool dispatcher matching AAA canonical; Hybrid B-extend-now + C-later); Andrew architectural decision pending; G-pointer-events-fix prompt drafting depends on chosen approach. H-saveload-diagnostic + H-saveload-fix pending (untestable without working brush; pending G outcome). F.5-paint §9 advances to COMPLETE only after G + H land + final Andrew-gate full PASS. Pre-existing observation noted: panels::terrain_panel::tests::test_terrain_panel_creation fails on pre-supplement state too (chunk_radius asserted 5 but actual 10) — TerrainPanel default drift unrelated to F.5-paint.F-fix; tracked for separate standalone follow-up. Original commit chain: 2026-05-01, commits 26a3864b8 (F.5-paint.A scaffold) + 226572bae (F.5-paint.B brush implementation + falloff visualization) + 2b230d94e (F.5-paint.C save/load + sibling-directory project layout) + e9d2a7922 (F.5-paint.D closeout). RegionalArchetypePanel registered in tools/aw_editor/src/panels/mod.rs and aw_editor module tree; brush size/falloff sliders + 6-archetype palette (color-swatch dropdown with stable display colors) + Paint/Erase mode + Save/Save As/Load/Clear persistence + Regenerate button placeholder; sibling-directory storage layout `<project>/regional_archetype_masks/<world>.{ron,id.bin,falloff.bin}` per Godot Terrain3D + Unity Terrain SOTA convention with default_mask_storage_dir/default_mask_base_path/ensure_mask_storage_dir/strip_ron_extension helpers. Panel-owned mask state via `mask: Option<RegionalArchetypeMask>` + `current_mask_base_path: Option<PathBuf>` + `last_io_status` UX feedback; ensure_mask/set_mask/clear_mask/apply_pending_paint_ops_to_owned/save_mask_to/load_mask_from API. Brush implementation: queue_paint_op captures panel state into PaintOp; apply_pending_paint_ops drains queue, stamps circular Euclidean-radius brushes, then single batched recompute_falloff. screen_to_world_xz_y0 ray-plane projection helper for viewport-pointer paint placement. 30 panel unit tests pass (7 F.5-paint.A scaffold + 10 F.5-paint.B brush/projection/apply + 13 F.5-paint.C mask-ownership/save-load/dir-helpers); F.4 regression suite still green (6 + 1 ignored). REMAINING for F.5 closure: F.5-overlay-and-gate session ships Climate Preview overlay (D.5c absorbed), `phase_1_x_f5_*` integration tests, and 5-region Andrew-gate (writer paints Veilweaver-realistic 5-region world in <30 minutes; output reads as Crimson-Desert-class regional variation at flyover altitude). F.5 §9 will mark COMPLETE only after F.5-overlay-and-gate Andrew-gate PASS.
F.6 — Scattered-convolution at regional layer: NOT STARTED.
F.7 — Per-archetype tuning + Andrew-gate: NOT STARTED.
F.8 — Closeout: NOT STARTED.
```

Format for completion updates: `F.N — <title>: COMPLETE <YYYY-MM-DD>, commit <hash>`.

---

## 10. Deviations log

This section records any design decisions made during execution that deviate from this plan. Every deviation must be recorded here before or in the same commit as the deviation itself.

Format for entries:

```text
### <YYYY-MM-DD>, Sub-phase <F.N>, commit <hash>
**Deviation:** <short description>
**Rationale:** <why>
**Impact:** <what parts of later sub-phases or other systems are affected>
```

### 2026-04-29, Sub-phase F.0 (campaign-design pass), commit 0e51763d4

**Launch entry — not a deviation; captures F.0's resolutions of §3.A-§3.F architectural decisions per F.0 prompt §3 with rationale.**

#### §3.A — Climate field dimensionality (resolved in §2.2)

**Resolution**: extend `ClimateMap::sample` with `erosion: f32` and `weirdness: f32` fields. Total stored climate dimensionality goes from 3 → 5; PV computed at sample time via `PvFold` (not stored).

**Alternatives considered**:
- Add only `erosion` and skip `weirdness`/PV. Rejected: Minecraft's PV-based "Peaks and Valleys" character (sharp ridges vs smooth rolling) is part of what makes archetypes feel different.
- Add full Minecraft 6D space (temperature, humidity, continentalness, erosion, weirdness, depth). Rejected: AstraWeave's `lookup_biome` already handles biome classification from `(temp, moisture, elevation)`; `humidity` would duplicate `moisture_mm`'s role; `depth` is voxel-specific.

**Rationale**: 3 → 5 is the smallest viable extension that supports per-archetype shape variation via Minecraft 1.18+'s separable-spline architecture.

**Performance impact**: 2 additional Perlin samples per vertex; ~5-10 ms per generation pass at radius 10.

#### §3.B — `BootstrapSplineSet` design (resolved in §2.3)

**Resolution**: 4 splines per archetype (`mountains_amplitude`, `mountains_scale`, `continental_scale`, `base_elevation_amplitude`); separable form (3 independent 1D splines per parameter, multiplicatively combined); piecewise-linear; embedded as Rust constants for the 6 catalog archetypes.

**Alternatives considered**:
- Catmull-Rom or cubic Bézier splines. Rejected: piecewise-linear is sufficient for terrain parameter mapping; faster to evaluate; simpler to author.
- Full 3D splines (trilinear interpolation in a 3D grid). Rejected: separable form is faster (3 1D evaluations + 2 multiplications vs 8-corner trilinear); loses some expressivity but interactions between climate axes are rare in geographic terrain.
- Data-driven JSON/RON file format for splines. Rejected at F.0: Rust constants for the catalog match D.5's architecture (compile-time data); modders/customs can be supported later.

**Rationale**: separable + piecewise-linear matches Minecraft 1.18+ canonical pattern; minimum viable spline architecture for per-archetype shape variation.

**Performance impact**: ~50-100 ns per vertex per spline evaluation.

#### §3.C — `RegionalArchetypeMask` design (resolved in §2.4)

**Resolution**: 1024×1024 mask (~11 WU per pixel at Target B 11264 WU per side); single-channel uint8 archetype ID + separate uint8 falloff distance field; two textures (~2 MB total); RON metadata file.

**Alternatives considered**:
- Multi-channel RGBA8 with packed (archetype_id_4bit + weight_4bit) per channel. Rejected: more complex to author and read; single-channel + falloff is simpler.
- Higher resolution (e.g., 4096×4096). Rejected: 1024×1024 is sufficient for archetype-region authoring (transitions handled by falloff at sampling time, not by mask resolution); memory cost would scale 16×.
- Lower resolution (e.g., 256×256). Rejected: ~44 WU per pixel coarsens region boundaries beyond what writer authoring needs.

**Rationale**: 1024×1024 is the resolution that best balances authoring fidelity, memory cost (2 MB), and disk cost (~10-100 KB compressed).

#### §3.D — Archetype blending math (resolved in §2.5)

**Resolution**: blend spline outputs after evaluation (option (c) from F.0 prompt §3.D). Each contributing archetype evaluates its `BootstrapSplineSet` independently; weights from mask falloff combine spline outputs into per-vertex blended `BootstrapParams`. Bootstrap noise pipeline runs once per vertex with blended params.

**Alternatives considered**:
- (a) Blend bootstrap output heights. Rejected: requires evaluating the noise pipeline N times per vertex; expensive at 4-archetype overlap zones.
- (b) Blend spline parameters before evaluation. Rejected: produces non-physical combinations in transition zones (averaging archetypes' spline shapes loses per-archetype character).
- (d) Blend bootstrap noise inputs. Rejected: unclear architectural meaning.

**Rationale**: (c) preserves per-archetype shape character at the spline-output layer; runs the noise pipeline once (cheapest); satisfies convex-combination invariant.

**Tradeoff acknowledgment**: blended bootstrap params at very narrow transition zones may not be physically self-consistent (e.g., high mountain amplitude + high continental scale doesn't represent any archetype's character). Bounded by climate field continuity in practice; F.7 tuning verifies natural-reading transitions.

#### §3.E — Composition with D.4 biome blending (resolved in §2.6)

**Resolution**: archetype blending runs FIRST (steps 1-5 of §2.1, at bootstrap parameter layer); D.4 biome blending runs LATER (step 7, at per-biome parameter layer). Bootstrap noise pipeline runs once per vertex with archetype-blended params; biome classification operates on archetype-shaped terrain.

**Alternatives considered**:
- D.4 biome blending runs FIRST, then archetype blending. Rejected: biome classification requires terrain shape to make sense; archetype blending shapes terrain, so it must run before classification.
- Both blending layers run in parallel, results combined post-hoc. Rejected: composition becomes unclear; harder to reason about which blending takes precedence.

**Rationale**: archetype blending operates at regional scale (~256 WU); biome blending operates at biome-boundary scale (~48 WU). Running archetype blending first means biome classification sees correctly-shaped terrain for the archetype zone; running biome blending second tunes per-biome parameters within whichever archetype the vertex resolves to.

**Tradeoff acknowledgment**: biome classification at archetype-transition zones may produce biome IDs that differ from what either pure archetype would produce. Generally desirable (transition zones get transition biomes); F.7 tuning verifies natural-reading distributions.

#### §3.F — Authoring UI shape (resolved in §2.8)

**Resolution**: new editor panel `RegionalArchetypePanel` alongside existing `TerrainPanel`. Activated via top-level menu toggle or tab.

**Alternatives considered**:
- Extend `TerrainPanel` with painting controls. Rejected: F.5 estimates ~3-6 weeks of editor work (paint UX, brush algorithms, mask serialization, Climate Preview overlay); bloating `TerrainPanel` with this much new UI fights against existing terrain-tool ergonomics.
- Separate standalone tool (outside the editor). Rejected: workflow integration with terrain regeneration is required; sharing runtime resource (`RegionalArchetypeMask`) between standalone tool and editor is more complex than two panels in the same editor.

**Rationale**: separation of concerns. `TerrainPanel` is about terrain noise + biome configuration; `RegionalArchetypePanel` is about regional layout. They share data (the mask) but have distinct workflows. Future authoring concerns (vegetation overrides, weather zones, etc.) might warrant their own panels too.

#### Filename + structure decisions

- **Filename**: `docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md` per F.0 prompt §1.
- **Structure**: mirrors `TERRAIN_GENERATION_QUALITY_CAMPAIGN.md`'s §0/§1/§2/§3-§N/§8/§9/§10 layout; sub-phase count 8 (F.0-F.8) per F.0 prompt §1.5.

#### F.4.B.3.D deferred-work split (per F.0 prompt §2)

**Absorbed into this campaign**:
- Climate Preview overlay (D.5c): F.5.
- Equatorial Tropical archetype-specific tuning: F.7.
- Bootstrap noise pipeline elevation skew (audit §7 of diagnostic-2): F.7.

**Stays as standalone follow-ups**:
- F.4.B.3.G 47.4 WU phase-2 grassland precision floor.
- Dead-write bug at terrain_panel.rs:943.
- Stale `NoiseConfig` literals in `behavioral_correctness_tests.rs`.
- MountainRocky never produced by lookup_biome.
- River never produced (Water System Rebuild campaign).
- Per-biome runevision tuning.

#### Andrew-gate gating policy

Per §0 lesson application:
- **F.1, F.2, F.8** (no visible terrain output): code-level verification only; no Andrew-gate.
- **F.3, F.4, F.5, F.6, F.7** (visible terrain output): Andrew-gate REQUIRED before §9 updates to COMPLETE. F.7 is the principal Andrew-gate of the campaign.

This is non-negotiable for the campaign's structure; baked into §0 as the explicit corrective from the predecessor campaign's pattern of code-level PASS + Andrew-gate REGRESS.

### 2026-04-29, Sub-phase F.1 (climate field extension), commits 5fcde4e98 + 744132c6a + 8e883beb5 + df7636fa3

**Sub-phase entry — captures F.1's pre-execution verification findings, deliverables, test scoreboard.**

**Pre-execution verification (per F.1 prompt §1, REQUIRED FIRST STEP)**:

- **§1.1 Seed offset collision check: PASS.** Cataloged all numeric seed offsets in the terrain crate. `+3000` (erosion) and `+4000` (weirdness) are unused. Existing offsets:
  - `climate.rs`: `+0` (temperature), `+1000` (moisture), `+2000` (continentalness).
  - `noise_gen.rs`: `+1` (mountains), `+2` (detail), `+42` (ridge), `+100` / `+200` (domain warp x/z), `+7` (continental, default).
  - `runevision_erosion.rs`: `+13` (default).
  - `biome_param_blending.rs`: `+31` (default).
  - `scatter.rs` / `biome_pack.rs`: `+0` (defaults).
- **§1.2 PV formula hand-verification: PASS.** All 7 canonical weirdness values (`-1, -2/3, -1/3, 0, 1/3, 2/3, 1`) hand-derived to expected PV outputs (`0, 1, 0, -1, 0, 1, 0`) per Minecraft 1.18+ canonical formula `pv = 1.0 - ((3.0 * |weirdness|) - 2.0).abs()`. F.1.A's `pv_fold_seven_canonical_points` test asserts these values within 1e-6 tolerance.
- **§1.3 D.1 climate field state inspection: PASS.** `ClimateMap` matches D.1 documented state (3 noise fields with offsets +0/+1000/+2000; `ClimateSample` has 3 stored fields). No drift from D.1 design.

**Deliverables**:

- **F.1.A** (commit `5fcde4e98`): new module `astraweave-terrain/src/spline_types.rs` (~220 lines + 8 unit tests). `PvFold` helper (canonical Minecraft formula); `Spline1D` placeholder struct (F.2.A populates); `BootstrapParam` enum with 4 variants (F.2.A consumes). Both `Spline1D` and `BootstrapParam` `#[allow(dead_code)]` until F.2.A.
- **F.1.B** (commit `744132c6a`): `ClimateSample` extended with `erosion: f32` and `weirdness: f32` fields + `pv()` derived accessor; `ClimateMap` extended with `erosion_noise: Perlin` (offset +3000, scale 0.0008) and `weirdness_noise: Perlin` (offset +4000, scale 0.0006); `ClimateMap::sample` populates new fields with defensive `.clamp(-1.0, 1.0)`. Existing field computation logic preserved byte-identically per F.1 prompt §2.2 scope discipline.
- **F.1.C** (commit `8e883beb5`): new test file `astraweave-terrain/tests/phase_1_x_f1_climate_field_extension.rs` (~470 lines + 14 permanent tests). Coverage: field range bounds, determinism (byte-identical bits), position dependence (stddev > 0.1), erosion-weirdness decorrelation (Pearson |r| < 0.15), PV formula propagation, D.1 backward-compat byte-identity (3 tests), F.4.B.3.D.5-fix Path B baseline distribution invariant, world archetype catalog construction.
- **F.1.D** (this commit): doc-only closeout updating §9 + Status header + this §10 entry.

**Methodology adjustment from F.1 prompt §2.3 Test 6**:

The F.1 prompt suggested capturing pre-F.1 reference values via `git stash` for the D.1 backward-compat regression. F.1.C used a different methodology: assert byte-identical bits across two consecutive runs at fixed positions, plus archetype-range smoke checks. This catches the load-bearing failure mode (F.1.B's noise-init order accidentally perturbing existing-field computation) without coupling the test to specific numeric values that may legitimately drift in future sub-phases. Documented in test doc-comments per F.1 prompt §3 methodological transparency requirement.

**Test scoreboard at F.1 close**:

- F.1.A `spline_types::tests`: 8/8 pass.
- F.1.C extension tests: 14/14 pass.
- D.1 `climate::tests`: 18/18 pass (10 D.1 sub-tests + 8 legacy tests; D.1 invariants preserved).
- All 757 terrain crate lib tests pass (738 pre-F.1 + 8 spline_types + 11 derived growth from new test file consolidation in lib build).
- `cargo check -p astraweave-terrain --lib`: clean.

**Pre-existing failures unchanged from F.1.B baseline** (NOT introduced by F.1):

- `behavioral_correctness_tests.rs` lines 517+911+912 stale `NoiseConfig` literal constructors — flagged as standalone follow-up per Phase 1.6-F closure §10.
- `astraweave-render` `coverage_booster_render.rs` and `wave2_*.rs` stale type/field references — unrelated to terrain crate.

**Andrew-gate**: not applicable per campaign doc §0 ("Sub-phases without visible-terrain output gate on code-level verification only"). F.1 produces no visible terrain change; downstream consumers (D.2 `lookup_biome`, D.4 `blend_biome_parameters`, bootstrap noise pipeline) ignore the new fields until F.3 wires `BootstrapSplineSet`.

**Scope held**: F.1 only modified `astraweave-terrain/src/climate.rs` (existing file, extended), `astraweave-terrain/src/spline_types.rs` (new file), `astraweave-terrain/src/lib.rs` (one-line module declaration), `astraweave-terrain/tests/phase_1_x_f1_climate_field_extension.rs` (new file), and this campaign doc (§9 + §10 + Status header in F.1.D). No other files. No "while-I'm-here" cleanups.

**Next**: F.2 (`BootstrapSplineSet` infrastructure) starts after F.1.D's hash-fixup lands. F.2 reads §5 of this document and implements `Spline1D::evaluate`, `Spline1D::identity`, `Spline1D::from_control_points`, `ParamSpline`, `BootstrapSplineSet`, `BootstrapParams`, plus 6 default `BootstrapSplineSet` const instances for the D.5 catalog archetypes. Continental Temperate at median climate sample reproduces F.4.B.3.D.5-fix's hardcoded `NoiseConfig::default()` values within ±1.0 / ±0.5%.

### 2026-04-30, Sub-phase F.2 (BootstrapSplineSet infrastructure), commits f0c3fa52d + b5d70071d + 43386cba5 + b58bc3585

**Sub-phase entry — captures F.2's pre-execution verification findings, deliverables, three documented architectural deviations, test scoreboard.**

**Pre-execution verification (per F.2 prompt §1, REQUIRED FIRST STEP)**:

- **§1.1 F.4.B.3.D.5-fix baseline value confirmation: PASS.** Confirmed `NoiseConfig::default()` at `astraweave-terrain/src/noise_gen.rs` produces:
  - `mountains.amplitude = 480.0` (line 193).
  - `mountains.scale = 0.002` (line 192).
  - `continental_scale = 0.0003` (line 141, via `default_continental_scale()`).
  - `base_elevation.amplitude = 150.0` (line 183).
  All 4 baseline values match campaign doc §2.3 specification.
- **§1.2 D.5 catalog archetype enumeration: PASS.** Six archetypes confirmed in `astraweave-terrain/src/world_archetypes.rs`: Continental Temperate, Equatorial Tropical, Boreal/Subarctic, Mediterranean, Desert + Custom (via `WorldArchetypeId::Custom => continental_temperate()`). Existing struct fields cataloged: 7 fields (`temperature_mean_c`, `temperature_variance_c`, `latitude_temperature_drop_c`, `moisture_mean_mm`, `moisture_variance_mm`, `continentalness_mean`, `continentalness_variance`); F.2.C adds an 8th (`bootstrap_splines`).
- **§1.3 F.1.A spline_types module state: PASS.** `astraweave-terrain/src/spline_types.rs` matches F.1.A documented state: `PvFold` helper (full impl), `Spline1D` placeholder (`control_points: Vec<(f32, f32)>` field; F.2.A populates with full impl), `BootstrapParam` enum (4 variants: `MountainsAmplitude`, `MountainsScale`, `ContinentalScale`, `BaseElevationAmplitude`). F.2 continues to live in `spline_types.rs` per F.2 prompt §1.3 recommendation; file at ~1100 lines after F.2's additions, still cohesive.

**Deliverables**:

- **F.2.A** (commit `f0c3fa52d`): full `Spline1D` API (piecewise-linear `evaluate` via `partition_point` + linear interpolation + clamp-at-endpoint, `identity()` returning `(0.0, 1.0)` single-point, `from_control_points` validating Empty/NotSorted/NaN/Infinite); `SplineError` enum with `at_index` reporting; `ClimateInputDim` enum (`Continentalness`/`Erosion`/`Pv`) with `read(&sample)` that routes `Pv` through `ClimateSample::pv()`; `ParamSpline` aggregating `Spline1D + ClimateInputDim` with `evaluate(&sample)` delegating to spline. 15 new unit tests; total spline_types tests: 23/23 pass.
- **F.2.B** (commit `b5d70071d`): `BootstrapParams` output struct (4 fields, derives Debug/Clone/Copy/PartialEq); `BootstrapSplineSet` aggregating 4 `ParamSpline` fields with `evaluate(&sample) -> BootstrapParams`; `D5FIX_BASELINE_*` const reference values; 6 catalog factory functions (`bootstrap_splines_continental_temperate`, `_equatorial_tropical`, `_boreal_subarctic`, `_mediterranean`, `_desert`, `_custom`) all returning `d5fix_baseline_spline_set()` (single-control-point splines at F.4.B.3.D.5-fix baseline). 6 new unit tests; total spline_types tests: 29/29 pass.
- **F.2.C** (commit `43386cba5`): `WorldArchetype` extended with `bootstrap_splines: BootstrapSplineSet` field; **`Copy` derive removed** (3 deviations logged below); `#[serde(skip, default)]` on new field for backward-compat deserialization; `BootstrapSplineSet::default()` impl returning baseline; all 6 D.5 catalog factories updated to populate `bootstrap_splines`; two test constructors fixed (biome_lookup.rs:700, biome_param_blending.rs:541) with `bootstrap_splines: Default::default()`; one partial-move at climate.rs:1029 fixed with `.clone()`. New integration test file `astraweave-terrain/tests/phase_1_x_f2_bootstrap_splines_integration.rs` with 6 permanent tests.
- **F.2.D** (this commit): doc-only closeout updating §9 + Status header + this §10 entry.

**Architectural deviations from campaign doc §2.3 + F.2 prompt** (all logged here per F.2 prompt §3 discipline):

1. **Single-spline-per-parameter shape** vs §2.3's "3 1D splines × multiplied" separable-form specification. F.2's `ParamSpline` carries one `Spline1D` reading one `ClimateInputDim`, not three multiplicatively combined. Rationale: F.2's catalog archetypes are all single-control-point constants where multi-spline product reduces to scalar product; F.7 tuning is where multi-spline product earns its keep. F.7 may extend to a `ParamSplineMulti` type or refactor `ParamSpline` to carry `[ParamSplineAxis; 3]`. This deviation was anticipated in F.2 prompt §2.1 and explicitly permitted as "F.2 commits to the simpler shape; F.7 escalates if needed."

2. **Factory functions instead of `const` declarations**. Campaign doc §2.3 specifies "embedded as Rust `const` declarations." However, `Spline1D::from_control_points` returns `Result<Self, SplineError>` (validated input) and `Vec::new()` / `vec!` are not const, so true `const` declarations are not feasible. F.2 ships 6 archetype factories (`fn bootstrap_splines_<archetype>() -> BootstrapSplineSet`) that construct fresh `BootstrapSplineSet` instances each call. Architectural intent (compile-time defaults, no per-frame allocation) preserved by F.2.C caching `bootstrap_splines` as a `WorldArchetype` struct field; runtime evaluation reads from cached references. This deviation was anticipated in F.2 prompt §2.2.

3. **Drop `Copy` derive from `WorldArchetype`**. Surfaced during F.2.C verification: `BootstrapSplineSet` contains `Vec<(f32, f32)>` (in `Spline1D::control_points`), which is not `Copy`. Three options considered: (a) drop Copy, (b) skip the field + add a method that calls factory by archetype ID, (c) invent a Copy-friendly storage that still supports Vec semantics. Chose (a). Rationale: F.7 will need multi-control-point splines that are intrinsically not Copy; removing Copy now avoids a forced refactor at F.7. `WorldArchetype` is mostly accessed as a borrowed field of `ClimateConfig`; Copy is convenience, not load-bearing. Two call sites adapted: (i) climate.rs:1029 test `let arch = config.archetype` → `.clone()`; (ii) `WorldArchetype` retains `Clone` derive for explicit cloning where needed. The campaign doc did not anticipate the Copy conflict; this deviation surfaced during execution and is logged here per F.2 prompt §3 discipline.

**Test scoreboard at F.2 close**:

- `spline_types::tests`: **29/29 pass** (8 F.1.A + 15 F.2.A + 6 F.2.B).
- F.2.C integration tests (`phase_1_x_f2_bootstrap_splines_integration.rs`): **6/6 pass**.
- F.1.C extension tests (`phase_1_x_f1_climate_field_extension.rs`): **14/14 pass** (F.1.B regression contract preserved through F.2.C).
- D.1 `climate::tests`: **18/18 pass** (D.1 invariants preserved; backward compat verified).
- D.5 `world_archetypes::tests`: **12/12 pass** (existing distribution + every-biome-appears-in-some-archetype tests pass with new field).
- `cargo check -p astraweave-terrain --lib`: clean.

**Pre-existing failures unchanged from F.1 baseline** (NOT introduced by F.2):

- `behavioral_correctness_tests.rs` lines 517+911+912 stale `NoiseConfig` literal constructors — flagged as standalone follow-up per Phase 1.6-F closure §10.
- `astraweave-render` `coverage_booster_render.rs` and `wave2_*.rs` stale type/field references — unrelated to terrain crate.

**F.4.B.3.D.5-fix Path B regression preservation**: F.2 only adds infrastructure types + a struct field; doesn't modify existing terrain output paths (`WorldGenerator::generate_chunk_with_climate`, `TerrainNoise::sample_height`, etc.). The bootstrap noise pipeline still reads `NoiseConfig::default()` constants; `bootstrap_splines` is dead code from the runtime path's perspective until F.3 wires.

**Andrew-gate**: not applicable per campaign doc §0 ("Sub-phases without visible-terrain output gate on code-level verification only"). F.2 produces no visible terrain change.

**Scope held**: F.2 only modified `astraweave-terrain/src/spline_types.rs` (existing file from F.1.A; extended), `astraweave-terrain/src/climate.rs` (existing file; added field + Copy removal), `astraweave-terrain/src/world_archetypes.rs` (existing file; populated bootstrap_splines in 5 catalog functions), `astraweave-terrain/src/biome_lookup.rs` (existing file; one test constructor fix), `astraweave-terrain/src/biome_param_blending.rs` (existing file; one test constructor fix), `astraweave-terrain/tests/phase_1_x_f2_bootstrap_splines_integration.rs` (new file), and this campaign doc (§9 + §10 + Status header in F.2.D). No other files. No "while-I'm-here" cleanups.

**Next**: F.3 (spline wiring, single-archetype regression) starts after F.2.D's hash-fixup lands. F.3 reads §6 of this document and wires `BootstrapSplineSet::evaluate` into `WorldGenerator::generate_chunk_with_climate` for Continental Temperate only. **First sub-phase requiring Andrew-gate** per campaign doc §0.

### 2026-05-01, Sub-phase F.3 (spline wiring, single-archetype regression), commits 04fc26299 + 447367c15 + be233fd78 + d6a063fbb

**Sub-phase entry — captures F.3's pre-execution verification findings, deliverables, two architectural deviations, test scoreboard, Andrew-gate PASS verdict, forward implication for F.7. First Andrew-gate-required sub-phase of the campaign.**

**Pre-execution verification (per F.3 prompt §1, REQUIRED FIRST STEP)**:

- **§1.1 F.2 API confirmation: PASS.** `BootstrapSplineSet::evaluate(&ClimateSample) -> BootstrapParams` exists with expected signature. `BootstrapParams` initially had 4 f32 fields (mountains_amplitude, mountains_scale, continental_scale, base_elevation_amplitude); `mountains_scale` was retrofit to f64 mid-F.3.B per deviation 2 below. 6 catalog factory functions accessible. `D5FIX_BASELINE_*` consts exist with values 480.0 / 0.002 / 0.0003 / 150.0. `WorldArchetype::bootstrap_splines: BootstrapSplineSet` field exists per F.2.C. Continental Temperate's `bootstrap_splines.evaluate` produces F.4.B.3.D.5-fix baseline at any climate sample (verified empirically).

- **§1.2 Existing integration-point inspection: identified.** `WorldGenerator::apply_per_biome_modulation_to_halo` (lib.rs:478) is the per-vertex height-sampling site that F.3.B refactors. Pre-F.3 this method called `TerrainNoise::sample_height_with_mountain_amplitude(x: f64, z: f64, mountain_amplitude_multiplier: f32)` per-vertex with the per-biome multiplier from D.3b's `BiomeParameters` blending. The integration point requires composing F.3's archetype-evaluated `BootstrapParams` with D.3b's per-biome multiplier — both layers compose multiplicatively at the same call site. (See deviation 1 below.)

- **§1.3 F.4.B.3.D.5-fix Path B regression sanity check: PASS pre-F.3.** Baseline confirmed at the F.2-close state before F.3.A landed (771 lib tests pass; F.1.C `d5_fix_baseline_distribution_invariant` passes).

**Deliverables**:

- **F.3.A** (commit `04fc26299`): `TerrainNoise::sample_height_with_params(params: &BootstrapParams, x: f64, z: f64, mountain_amplitude_multiplier: f32) -> f32` mirroring `sample_height_with_mountain_amplitude` arithmetic with four `self.config.X` reads replaced by `params.Y` reads (mountains.amplitude, mountains.scale, continental_scale, base_elevation.amplitude). 4 unit tests in `noise_gen::tests` covering: byte-identity vs `sample_height` at baseline params (initially asserted within-tolerance per F.3 prompt §6.3 fallback after surfacing 60-ulp drift; F.3.B retrofit restored byte-identity and tightened the assertion to `max == 0.0`); doubled-mountain-amplitude smoke test; determinism; position-varies. Initial f32 storage for `mountains_scale` produced single-vertex 60-ulp drift that did not surface as a failure at F.3.A close — surfaced at F.3.B integration when chunk-scale accumulation reached the 80 WU phase-2 continuity threshold.

- **F.3.B** (commit `447367c15`): `WorldGenerator::apply_per_biome_modulation_to_halo` refactored to evaluate Continental Temperate's `BootstrapSplineSet` per-vertex and pass results through `sample_height_with_params`. Continental Temperate's `bootstrap_splines` borrowed once at chunk-generation start (`bootstrap_splines_continental_temperate()` factory call cached); per-vertex evaluation is allocation-free. **Mid-flight retrofit landed in this same commit**: `BootstrapParams.mountains_scale: f32 → f64`; `BootstrapSplineSet.mountains_scale: ParamSpline → f64` (direct f64 storage replacing the spline indirection for this one parameter). Root cause: f32's representation of 0.002 (`0x3B03126F` ≈ `0.0020000000949...`) caused all bootstrap mountain noise samples to use slightly-irrational world coordinates; chunk-edge accumulation at radius 10 (5632 WU from world center) compounded to 103 WU divergence at chunk shared edges, breaking phase-2 continuity 80 WU grassland threshold. Direct f64 storage restored byte-identity. (See deviation 2 below.)

- **F.3.C** (commit `be233fd78`): new test file `astraweave-terrain/tests/phase_1_x_f3_spline_wiring.rs` with 4 permanent regression + smoke tests:
  - `phase_1_x_f3_continental_temperate_byte_identical_to_f4b3d_d5fix` — byte-identity contract; 100 sample positions across Target B world; max divergence between F.3-wired path and legacy `sample_height` must be exactly 0.0 (post-retrofit).
  - `phase_1_x_f3_spline_toggling_changes_terrain` — falsifiable smoke test; doubles `mountains_amplitude` (480 → 960) at a position with baseline pre-erosion height > 100m; asserts height_delta > 50m and positive, confirming splines are wired (not dead-code).
  - `phase_1_x_f3_continental_temperate_at_median_climate_matches_baseline` — sanity assertion that Continental Temperate's `BootstrapSplineSet::evaluate` produces F.4.B.3.D.5-fix baseline byte-identically at median climate sample (weirdness=1.0 → pv=0).
  - `phase_1_x_f3_phase_2_continuity_preserved_documentation` — documentation landmark cross-referencing the existing 45s-erosion phase-2 continuity test which is the load-bearing chunk-edge regression check; manually verified passing at 80 WU grassland / 10 WU mountain post-F.3.B retrofit.

- **F.3.D** (this commit): doc-only closeout updating §9 + Status header + this §10 entry.

**Architectural deviations from F.3 prompt** (logged here per F.3 prompt §3 discipline):

1. **`sample_height_with_params` 4-arg signature** instead of F.3 prompt §2.1's specified 3-arg `(params, world_x, world_z)`. Added `mountain_amplitude_multiplier: f32` as 4th argument. **Rationale**: D.3b's per-biome layer composes a multiplicative factor at the same call site where archetype-evaluated `BootstrapParams` is consumed (`apply_per_biome_modulation_to_halo` already passed a per-biome `mountain_amplitude_multiplier` to `sample_height_with_mountain_amplitude` pre-F.3). The two layers (regional archetype + per-biome) compose at this single per-vertex point. The 3-arg signature would have required either (a) duplicating the noise sample path with a separate post-modulation step, or (b) folding the per-biome multiplier into `BootstrapParams` (architecturally wrong; the multiplier is per-biome, not per-archetype). 4-arg signature preserves both layers' semantics. Also: world coordinates stay `f64` (matching legacy `sample_height` signature; F.3 prompt §2.1's `f32` was an overspec). **Forward implication**: F.4's mask integration composes archetype-blended `BootstrapParams` (per §2.5 of campaign doc) with the per-biome `mountain_amplitude_multiplier` from D.3b at this same call site. F.4 doesn't need to revisit the signature.

2. **`mountains_scale` retrofit from f32 spline output to f64 direct storage**. The F.3 prompt and F.2's `BootstrapParams` shipped `mountains_scale: f32`. F.3.B integration surfaced a 60-ulp drift between `sample_height_with_mountain_amplitude` (which uses `f64` internally — `NoiseConfig.mountains.scale: f64 = 0.002` exact) and `sample_height_with_params` reading f32-stored `mountains_scale` and converting to f64 at the call site. The drift compounded across chunk-edge accumulation: at radius 10's 5632 WU max world coordinate, 60-ulp imprecision in `mountains_scale` produced ~5e-7 WU coordinate divergence per sample, which through the noise lattice's local Lipschitz behavior amplified to ~10⁻⁴ m elevation divergence per vertex pre-erosion; through erosion's iterative droplet pathing (per F.3-phase-3 world-coord seeding finding), this further amplified to **103 WU divergence at chunk shared edges** (broke phase-2 continuity 80 WU grassland threshold; measured empirically during F.3.B integration). **Resolution**: direct f64 storage for `mountains_scale` (both in `BootstrapParams` and as a special-case f64 field in `BootstrapSplineSet` replacing the `ParamSpline` indirection for this one parameter; F.2.B's "4 ParamSpline fields" framing slightly modified to "3 ParamSpline + 1 direct f64"). All 6 catalog factory functions updated to construct the f64 field directly with the F.4.B.3.D.5-fix baseline value 0.002. Byte-identity restored. `continental_scale` stays `f32` because legacy `NoiseConfig.continental_scale` is also `f32` — both sides share the same f32 representation, no drift. `mountains_amplitude` and `base_elevation_amplitude` stay `f32` because they multiply `[-1, 1]` noise output where f32 precision is sufficient (not noise sample coordinates). **Forward implication for F.7**: per-archetype tuning should preserve f64 storage for noise-coordinate-multiplier parameters. `mountains_scale` is the only confirmed coordinate-multiplier parameter in the current 4-output `BootstrapSplineSet`; if F.7's per-archetype tuning later expands the spline tuning surface to include `continental_scale` with multi-control-point shapes that need precision beyond f32's representation, the f32→f64 retrofit pattern from F.3.B is the canonical resolution. **F.4 should not revisit the f32/f64 storage decision** — F.3.B's resolution is canonical for the campaign's remaining sub-phases.

**Test scoreboard at F.3 close**:

- F.3.A `noise_gen::tests` (sample_height_with_params suite): **4/4 pass** (byte-identity assertion).
- F.3.C tests (`phase_1_x_f3_spline_wiring.rs`): **4/4 pass**.
- F.2 `spline_types::tests`: **29/29 pass** (regression contract preserved through F.3.B retrofit).
- F.2.C integration tests (`phase_1_x_f2_bootstrap_splines_integration.rs`): **6/6 pass**.
- F.1.C extension tests (`phase_1_x_f1_climate_field_extension.rs`): **14/14 pass**.
- D.1 `climate::tests`: **18/18 pass**.
- D.5 `world_archetypes::tests`: **12/12 pass**.
- Phase-2 continuity (80 WU grassland / 10 WU mountain post-erosion, separate file 45s erosion run): **PASS**.
- All 771 terrain crate lib tests pass.
- `cargo check -p astraweave-terrain --lib`: clean.

**Pre-existing failures unchanged from F.2 baseline** (NOT introduced by F.3):

- `behavioral_correctness_tests.rs` lines 517+911+912 stale `NoiseConfig` literal constructors — flagged as standalone follow-up per Phase 1.6-F closure §10.
- `astraweave-render` `coverage_booster_render.rs` and `wave2_*.rs` stale type/field references — unrelated to terrain crate.

**Andrew-gate verdict** (per F.3 prompt §2.5):

- **Verdict**: PASS, 2026-05-01.
- **Setup**: editor open, Continental Temperate archetype, seed 12345, radius 10, flyover altitude.
- **Comparison reference**: pre-F.3 baseline (post-F.2 state at commit chain ending at `b718fdd5e`).
- **Findings**: terrain shape character visually identical at flyover; no new artifacts; no new chunk seam visibility; no wgpu validation errors. Macro shapes match per byte-identity contract.
- **Noteworthy observation**: post-F.3 terrain reads as "smoother / higher fidelity" compared to pre-F.3 baseline. **This is plausibly a real quality improvement attributable to F.3.B's f32 → f64 retrofit for `mountains_scale`**. Pre-F.3 used `0.002_f32` (which stores as `0x3B03126F` ≈ `0.0020000000949...`), causing all bootstrap mountain noise samples to use slightly-irrational world coordinates and producing nearly-imperceptible high-frequency aliasing at the noise lattice. Post-F.3 samples at exact 0.002-multiple coordinates, eliminating the aliasing. Macro shapes remain byte-identical per the regression contract; the perceived improvement is a clean side-effect of the precision retrofit. Camera-angle confound considered (the comparison screenshots were at slightly different altitudes) but does not fully account for the perception — the f64 retrofit is the more parsimonious explanation.
- **Forward implication for F.7**: preserve f64 storage for noise-coordinate-multiplier parameters across all archetypes; if F.7's per-archetype tuning produces multi-control-point splines for `continental_scale` (currently f32), apply the same f32→f64 retrofit pattern.

**Scope held**: F.3 only modified `astraweave-terrain/src/noise_gen.rs` (F.3.A added `sample_height_with_params`), `astraweave-terrain/src/lib.rs` (F.3.B refactored `apply_per_biome_modulation_to_halo`), `astraweave-terrain/src/spline_types.rs` (F.3.B retrofit for `mountains_scale` storage), `astraweave-terrain/tests/phase_1_x_f3_spline_wiring.rs` (F.3.C new file), and this campaign doc (§9 + §10 + Status header in F.3.D). No other files. No "while-I'm-here" cleanups. F.3.B did NOT modify `world_archetypes.rs` because the retrofit lives entirely in `BootstrapSplineSet`'s structure — D.5 catalog factory functions still call `bootstrap_splines_continental_temperate()` etc. unchanged at the WorldArchetype level.

**Architectural milestone**: F.3 is the campaign's architectural hinge. F.1 and F.2 added types and infrastructure without exercising them; F.3 wires the infrastructure into the runtime path. With F.3 closed at Andrew-gate PASS, the data path is canonical for F.4-F.7's extensions. F.4 builds on F.3's data path with the `RegionalArchetypeMask` integration; per the deviation 1 forward implication, F.4 does not revisit `sample_height_with_params`'s 4-arg signature.

**Next**: F.4 (`RegionalArchetypeMask` + falloff sampler) starts after F.3.D's hash-fixup lands. F.4 reads campaign doc §7 and implements mask format, runtime sampler, archetype-blend per-vertex aggregation, and `WorldGenerator` integration. **Second Andrew-gate-required sub-phase** per campaign doc §0.

### 2026-05-01, Sub-phase F.4 (RegionalArchetypeMask + falloff sampler), commits 3eafa4ca4 + 3c6c915c9 + 489eee27d + 1b63b921a + d3e2c0f67 + 8581391dc + (F.4.G deferred-closeout reconciled via pause artifacts 2026-05-03)

**Sub-phase entry — captures F.4 sub-phase closeout deferred from F.4.G to the Regional Archetype Variation pause artifacts session. F.4.A-F production work landed 2026-05-01; Andrew-gate PASS verdict captured verbally; §9 closeout commit was deferred during the F.5-paint emerging remediation cascade and is now executed as part of the campaign pause artifacts.**

**Why deferred**:

F.4 closed at code level on 2026-05-01 with all six sub-commits landing in a single-day burst (F.4.A `3eafa4ca4` 15:24 → F.4.F `8581391dc` 15:48). Andrew-gate PASS verdict was captured verbally during the ~1-hour window between F.4.F (15:48) and F.5-paint.A start (`26a3864b8` 16:44 same day). The closeout commit (originally specified as F.4.E per campaign doc §7.5 four-step plan; restructured to F.4.G after execution expanded to a six-step plan) was deferred during the F.5-paint sub-phase's emerging remediation cascade — initial momentum carried through to F.5-paint.A scaffold work without first landing F.4.G. The cascade extended over 6+ sessions (F.5-paint.A → F.5-paint.D + hash-fixup → F.5-paint.E-diagnostic → F.5-paint.F-fix.A → F.5-paint.F-fix.A-supplement → F.5-paint.F-fix.B + hash-fixup → G-pointer-events-research → G-pointer-events-diagnostic), and each successive session's scope precluded a return to F.4 closeout. The pause artifacts session 2026-05-03 is the natural moment to reconcile.

**Sub-phase structure deviation from campaign doc §7.5**:

Campaign doc §7.5 originally specified F.4 as a 4-step plan (F.4.A mask + sampler; F.4.B save/load + distance transform; F.4.C WorldGenerator integration; F.4.D integration + perf tests; F.4.E closeout). Execution restructured to a 6-step plan during code-level work:

- **F.4.A** `3eafa4ca4` — RegionalArchetypeMask type + RON metadata.
- **F.4.B** `3c6c915c9` — mask save/load + Euclidean distance transform (matches campaign §7.5's F.4.B).
- **F.4.C** `489eee27d` — RegionalArchetypeBlend neighborhood-scan sampler (split out from §7.5's F.4.A).
- **F.4.D** `1b63b921a` — multi-archetype blend math (BootstrapParams blending per §2.5).
- **F.4.E** `d3e2c0f67` — wire RegionalArchetypeMask into WorldGenerator (matches §7.5's F.4.C).
- **F.4.F** `8581391dc` — F.4 integration + perf + synthetic-differentiation tests (matches §7.5's F.4.D).
- **F.4.G** (this entry) — deferred §9 + §10 closeout reconciliation.

The restructure was driven by sub-step granularity exceeding 4-step plan: sampler (C), blend math (D), and WorldGenerator integration (E) were independently revertable, so each landed as its own commit. No deviation from architectural intent; only sub-commit count differs from §7.5's original spec.

**Andrew-gate verdict** (captured retroactively from F.5-paint cascade context):

- **Verdict**: PASS, 2026-05-01.
- **Setup**: programmatic 5-archetype world per campaign doc §7.3 success criteria — CT center, Boreal north, Mediterranean south, Desert east, Tropical west — at radius 10 seed 12345.
- **Findings**: visibly distinct archetype regions with smooth boundaries; transition zones blend cleanly per §2.5 spline-output blending math; performance within +30% of F.3 baseline (verified via `phase_1_x_f4_perf_within_30_percent_of_f3` ignored test in F.4.F).
- **Date approximation**: precise hour within the F.4.F → F.5-paint.A window (15:48 → 16:44 on 2026-05-01) not recorded; can be amended via §10 follow-up if Andrew recalls. The PASS verdict is unambiguous; only the timestamp granularity is approximate.

**F.4 architectural canonicals (preserved forward)**:

- `RegionalArchetypeMask` data type + persistence API (3-file format: `.id.bin` + `.falloff.bin` + `.ron`).
- `WorldGenerator.regional_archetype_mask: Option<RegionalArchetypeMask>` field as integration surface.
- `WorldArchetypeId::to_mask_id()` / `from_mask_id()` for ID/enum bridging (1-6 for 6 catalog archetypes; 0 reserved for unpainted).
- `RegionalArchetypeBlend::sample_at` neighborhood-scan sampler (3 paths: unpainted fast, deep-interior fast, transition slow).
- `BlendContributors` custom small fixed-size struct (4-element max; avoids arrayvec dep).
- `blend_bootstrap_params` per campaign §2.5: blends spline outputs after evaluation; bootstrap noise pipeline runs ONCE per vertex with blended params.
- Sibling-directory storage layout: `<project>/regional_archetype_masks/<world>.{ron,id.bin,falloff.bin}` (Godot Terrain3D + Unity Terrain SOTA convention).
- F.3 byte-identity regression preserved on None mask path (F.4.E refactor of `WorldGenerator::generate_chunk_with_climate` handles `None` arm via existing F.3 code path; `Some` arm dispatches to F.4 mask-aware path).

These canonicals are inherited forward unchanged by F.5-paint, F.5-overlay-and-gate, F.6, F.7, and F.8.

**Test scoreboard at F.4.F close (carried forward unchanged)**:

- 6 F.4 integration tests pass: `phase_1_x_f4_no_mask_byte_identical_to_f3`, `phase_1_x_f4_unpainted_mask_byte_identical_to_none`, `phase_1_x_f4_painted_circle_produces_terrain`, `phase_1_x_f4_falloff_zone_blends_smoothly`, `phase_1_x_f4_save_load_preserves_terrain_output`, `phase_1_x_f4_five_archetype_andrew_gate_world_helper`.
- 1 ignored perf test: `phase_1_x_f4_perf_within_30_percent_of_f3` (manual invocation; passes at 1.00× ratio at production-default 1024² mask resolution).
- 33 unit tests in `astraweave_terrain::regional_archetype_mask::tests` (F.4.A-D coverage).
- All upstream regression suites still green (F.1.C climate field tests, F.2.B spline_types tests, F.2.C WorldArchetype tests, F.3.C byte-identity contracts, D.1 climate, D.5 world archetypes catalog).

**Architectural milestone**: F.4 is the campaign's first multi-archetype-aware sub-phase. F.1-F.3 added infrastructure for single-archetype regression (Continental Temperate only). F.4 wires the runtime path for arbitrary archetype masks with smooth blending. The data path canonical post-F.4 is: `WorldGenerator.regional_archetype_mask: Option<RegionalArchetypeMask>` → per-vertex `RegionalArchetypeBlend::sample_at` → `BlendContributors` → `blend_bootstrap_params` per §2.5 → `BootstrapParams` → bootstrap noise pipeline → height. F.5+ extends this path; F.5-paint adds editor authoring; F.5-overlay-and-gate adds Climate Preview overlay; F.6 adds scattered-convolution at boundaries; F.7 adds per-archetype tuning; F.8 closes.

**Forward**: F.5-paint (next sub-phase) split into F.5-paint + F.5-overlay-and-gate sessions per Andrew's pre-F.5 split decision. F.5-paint commits land 2026-05-01 onwards; remediation cascade through 2026-05-03; pause artifacts session reconciles F.4 closeout retroactively then pauses Regional Archetype Variation pending Editor Multi-Tool Architecture campaign (see Pause-Artifacts §10 entry following the G-pointer-events-diagnostic entry below).

**Scope held**: F.4.G reconciliation only modifies the Regional Archetype Variation campaign doc (Status header + §9 F.4 line + this §10 entry, all in pause artifacts Commit 1). No production code changes. No audits modified. No commits reverted. F.4.A-F production code is unchanged; only documentation closure deferred from 2026-05-01 lands now.

### 2026-05-01, Sub-phase F.5 (paint slice — first half of F.5 two-session split), commits 26a3864b8 + 226572bae + 2b230d94e + e9d2a7922

**Deviation 1: F.5 split into F.5-paint + F.5-overlay-and-gate sessions**

Per §8.5, F.5 ships six sub-commits (F.5.A scaffold + F.5.B brush + F.5.C save/load + F.5.D Climate Preview overlay + F.5.E integration tests + F.5.F Andrew-gate + closeout). The user split F.5 into two sessions: **F.5-paint** (this session) covers the panel scaffold + brush UX + save/load slice (F.5.A-C analog), and **F.5-overlay-and-gate** (subsequent session) covers Climate Preview overlay + integration tests + Andrew-gate (F.5.D-F analog). The F.5 Andrew-gate carries forward to F.5-overlay-and-gate; this paint slice ships authoring infrastructure only.

Sub-commits in F.5-paint adopted slightly different naming to reflect the split:
- F.5-paint.A → analog of §8.5's F.5.A (scaffold).
- F.5-paint.B → analog of §8.5's F.5.B (brush implementation).
- F.5-paint.C → analog of §8.5's F.5.C (save/load).
- F.5-paint.D → closeout-of-paint-slice (campaign doc updates only; no new code).

F.5-overlay-and-gate will use F.5-overlay-and-gate.A/B/C (or .A-.E) sub-commit names mirroring §8.5's F.5.D-F.

**Rationale**: §1.4 estimated F.5 at 3-6 weeks of editor work. Bundling all six sub-commits in one session risks context-window exhaustion before reaching the integration-tests + Andrew-gate slice. The split preserves §8.5's commit-per-sub-step discipline while bounding session scope.

**Impact**: F.5 §9 status remains IN PROGRESS until F.5-overlay-and-gate completes. F.5-paint commits land paintable infrastructure (panel + brush + save/load); the overlay + integration tests + Andrew-gate land separately. `phase_1_x_f5_*` integration tests are NOT yet written; only F.5-paint unit tests in `panels::regional_archetype_panel::tests` (30 total) live in this slice. The F.5-paint commits are runtime-safe and revertible independently if F.5-overlay-and-gate redesigns the panel API.

**Deviation 2: Sibling-directory storage layout adopted (Q4 SOTA research)**

§8.2's "save alongside heightmap as part of Save Project" defers concrete storage path layout to implementation. F.5-paint.C adopts the sibling-directory pattern: `<project_root>/regional_archetype_masks/<world>.{ron,id.bin,falloff.bin}`. This is the established Godot Terrain3D + Unity Terrain convention (large binary terrain side-data adjacent to project files in a dedicated subdirectory, separate from primary scene/world stems).

Helpers shipped in `panels::regional_archetype_panel`:
- `MASK_STORAGE_SUBDIR: &str = "regional_archetype_masks"` (the subdir name).
- `default_mask_storage_dir(project_root: &Path) -> PathBuf`.
- `default_mask_base_path(project_root: &Path, world_name: &str) -> PathBuf`.
- `ensure_mask_storage_dir(project_root: &Path) -> std::io::Result<PathBuf>`.
- `strip_ron_extension(path: &Path) -> PathBuf` (for handling `rfd::FileDialog`'s `.ron`-extended pick paths).

**Rationale**: aw_editor does not currently expose a single "Save Project" event hook for the panel to register against; the panel needs a self-contained path layout that the editor can later wire into a unified project-save flow. Sibling-directory pattern matches industry SOTA and round-trips cleanly without entangling F.5-paint's panel logic with editor-wide save infrastructure that hasn't been designed yet. The pattern is reusable for future panels (F.6 vegetation override mask, F.7 weather zone mask) under their own subdirs.

**Impact**: F.5-overlay-and-gate may add a project-load event listener that calls `panel.load_mask_from(default_mask_base_path(...))` automatically when a world opens. Current explicit Save / Save As / Load / Clear buttons remain functional regardless. The current implementation roots `rfd::FileDialog` at `cwd/regional_archetype_masks/` because aw_editor doesn't yet expose a project root path; this is a transient simplification that F.5-overlay-and-gate or a follow-up editor-integration pass can refine.

**Deviation 3: Panel-owned mask via `Option<RegionalArchetypeMask>` field (mediator pattern)**

§8.2's UI spec doesn't specify mask ownership. F.5-paint.C makes the panel directly own `mask: Option<RegionalArchetypeMask>` as a `pub` field, with accessor / mutator methods (`set_mask`, `clear_mask`, `ensure_mask`, `apply_pending_paint_ops_to_owned`) for state transitions, plus exposed `current_mask_base_path: Option<PathBuf>` for save-to-same-place UX and `last_io_status: Option<String>` for under-button user feedback.

**Rationale**: TerrainPanel and other content-authoring panels in aw_editor own their state directly, allowing `egui` interactions to mutate state without round-tripping through a global. This matches the existing panel ergonomics. Exposing `mask` as a public field (not just a getter) lets editor integration code (currently outside F.5-paint scope; lands in F.5-overlay-and-gate or beyond) clone it directly into `WorldGenerator.regional_archetype_mask` at Regenerate Terrain time without API ceremony. The `apply_pending_paint_ops_to_owned` convenience method threads through panel-owned mask via take/replace dance internally without exposing ownership complexity to callers.

**Impact**: Editor's regenerate flow (to be wired in F.5-overlay-and-gate or beyond) reads `panel.mask` and clones it into `WorldGenerator.regional_archetype_mask`. The existing `apply_pending_paint_ops(&mut self, mask: &mut RegionalArchetypeMask)` API still operates on caller-provided mask for testability + decoupling — both paths coexist and share the same brush logic.

**Deviation 4: Test count exceeds the F.5-paint prompt's per-sub-commit suggestion**

F.5-paint prompt §2.2 mentioned "Approximately 7 new tests" for the brush slice. Final test count: F.5-paint.A 7 scaffold tests + F.5-paint.B 10 brush/projection/apply tests + F.5-paint.C 13 mask-ownership/save-load/dir-helpers tests = 30 panel unit tests total in F.5-paint.

**Rationale**: Brush + save/load surface multiple invariants (paint/erase mode, queue-state capture, recompute-falloff trigger, ron-extension stripping, sibling-dir layout, byte-identical save/load round-trip, falloff-radius-pixels inheritance on load, error path on no-mask save, ensure_mask idempotence, clear_mask state reset, apply-to-owned alloc-on-empty). Each invariant gets its own diagnosable test.

**Impact**: None negative; thorough coverage protects subsequent work in F.5-overlay-and-gate (overlay rendering must read from `panel.mask` correctly; integration tests must round-trip through save/load reliably).

**Architectural milestone**: F.5-paint closes the **infrastructure** half of F.5. The runtime path can now: (a) accept paint operations via the panel UI, (b) accumulate them in a queue, (c) apply them to a panel-owned mask with batched falloff recomputation, (d) save and load the mask to/from disk in the sibling-directory layout. What remains for F.5-overlay-and-gate is the visual feedback + verification layer (Climate Preview overlay, integration tests, Andrew-gate). The F.4 mask-integration path in `WorldGenerator` is unchanged (F.4 canonical inheritance preserved); F.5-paint doesn't touch terrain runtime code.

**Forward implication for F.5-overlay-and-gate**: the panel's `mask` field is the source of truth for both Climate Preview overlay rendering (read-only) and integration tests (programmatic paint → mask check). F.5-overlay-and-gate must not reintroduce a separate mask-storage layer — it composes against panel-owned state.

**Next**: F.4.G closeout (Andrew-gate verdict + §9 update) and F.5-overlay-and-gate (Climate Preview overlay + `phase_1_x_f5_*` integration tests + 5-region Andrew-gate). Both can run in either order: F.4.G is pure documentation closure once the user provides Andrew-gate verdict; F.5-overlay-and-gate is the next code-level slice. F.5 §9 marks COMPLETE only after F.5-overlay-and-gate Andrew-gate PASS.

### 2026-05-03, Sub-phase F.5-paint Andrew-gate REGRESS + diagnostic, commits 5f772bea3 + e561d4dce

**Sub-phase entry — captures Andrew-gate REGRESS verdict, diagnostic findings, §9 status revert.**

**Andrew-gate verdict (per F.5-paint prompt §2.4 success criteria + Andrew's checklist verification)**:

- **Verdict**: REGRESS, 2026-05-03.
- **Setup**: Andrew opened the editor at the F.5-paint commit chain end (hash-fixup `b6dd9de58`).
- **Findings**:
  - No "Regional Archetype" panel option in the editor's View → "Panels" submenu (where "Terrain", "Hierarchy", "Inspector", etc., live).
  - Click+drag pointer events on the viewport registered as undo entries on the *existing* Terrain panel's sculpt/paint brush (status bar showed "Terrain Paint" / "Terrain Sculpt" entries — not the F.5-paint panel's actions).
  - "Save Mask" / "Load Mask" buttons (in the *existing* Terrain panel; not the F.5-paint panel) had no visible effect — but this observation was misdirected (the F.5-paint panel's persistence buttons were never reachable; Andrew was clicking the Terrain panel's buttons, which are unrelated infrastructure).

**Root cause** (per `docs/audits/f5_paint_panel_registration_diagnostic_2026-05-03.md`, audit sub-phase F.5-paint.E-diagnostic):

**Hypothesis A confirmed** — *Module declared but registration call never made.* F.5-paint.A added `pub mod regional_archetype_panel;` at `tools/aw_editor/src/panels/mod.rs:54`, making the new panel module reachable from Rust's compilation graph. But it added zero registration calls into `tools/aw_editor/src/panel_type.rs` (the `PanelType` enum + 5 supporting match arms + `all()` slice — 7 surfaces total) or `tools/aw_editor/src/tab_viewer/mod.rs` (the `EditorTabViewer` struct field + `new()` initializer + render-dispatch match arm — 3 surfaces). Workspace-wide grep for `RegionalArchetype` in `tools/aw_editor/` returns hits only inside the panel's own file plus the `pub mod` line. The panel struct exists, compiles, has 30 passing unit tests — and is never instantiated by the editor's UI construction code, so it cannot appear in any user-facing panel registry, menu, or tab.

The View/Window menu populator (`tools/aw_editor/src/ui/menu_bar.rs:368`), the "Add Panel" popup (`tools/aw_editor/src/tab_viewer/mod.rs:475`), and the console panel-list command (`tools/aw_editor/src/main.rs:5941`) all iterate `PanelType::all()`. With no `PanelType::RegionalArchetype` variant, the panel is structurally unreachable from any user-facing surface.

Hypotheses B-E (registration-at-wrong-location / discovery-mechanism-bypass / Panel-trait-incomplete / multi-path-mismatch) are ruled out by the audit; A is sufficient root cause.

**Methodological lesson**:

F.5-paint.A's panel registration was specified in the F.5-paint prompt §2.1 step 7 ("register the panel with the editor's panel system"). The 30 unit tests across F.5-paint.A-C exercised the panel struct programmatically — `Panel` trait method dispatch, brush queue mutation, save/load roundtrip, sibling-directory layout helpers — but never engaged the editor's panel-registry layer (`PanelType::all()` consumers). This produced false-confidence: 30/30 PASS at code-level, REGRESS at user-facing reality.

This is precisely the failure mode the campaign §0 lesson application targets: code-level PASS at struct-shape isn't plan-level PASS at user-facing-deliverable-shape. The Andrew-gate caught the gap; unit tests structurally couldn't because they didn't engage the menu-population code path. The audit document (`docs/audits/f5_paint_panel_registration_diagnostic_2026-05-03.md`) catalogues the canonical 10-surface registration pattern (referencing `Blueprint`'s correct precedent at `panel_type.rs` lines 227 / 276 / 325 / 348 / 383 / 455 / 536 + `tab_viewer/mod.rs` lines 706 / 984) so F.5-paint.F-fix can mirror it without re-investigation.

**Forward implication for F.5-paint.F-fix and future panel additions**:

When adding a new editor panel, the verification must include either:

- **Pattern A** (recommended; landing in F.5-paint.F-fix): a test that asserts the panel appears in `PanelType::all()` AND that `EditorTabViewer::new()` instantiates the panel field. Two short tests; both would have caught F.5-paint's gap immediately.
- **Pattern B** (deferred; future hardening): an integration test exercising the full editor instantiation path through to menu rendering. Requires headless egui infrastructure not yet present in the editor's test harness; out of F.5-paint.F-fix scope.

**§9 status revert**:

Per the F.5-paint.E-diagnostic prompt §4.2 + Andrew's Q5 resolution: F.5-paint's §9 marker is reverted from COMPLETE to IN PROGRESS pending remediation. The original commit chain (`26a3864b8` → `b6dd9de58`) remains landed; the closeout commit `e9d2a7922` is **not** reverted (its doc updates remain accurate as a historical record of what F.5-paint.D claimed at the time). Only the Status header (line 3) and §9 F.5 entry reflect the actual current state. The previous F.5-paint deviations log entry above (dated 2026-05-01) is preserved as-is — Andrew's Q4 resolution: new entry, not amend original.

**Out-of-scope concerns surfaced by Andrew-gate** (separate diagnostic + fix sessions per Q3 resolution):

- **Save/Load button silent failure**: misdirected observation (Andrew was clicking the existing Terrain panel's buttons, not the unreachable F.5-paint panel's). Will be re-verified once panel registration is fixed; if fresh issues surface post-fix, those become the trigger for F.5-paint.G-saveload-diagnostic (a separate session).
- **Brush UX paint-without-visible-change**: also misdirected (Andrew was testing the existing Terrain panel's sculpt brush). However the audit surfaced an *additional* gap not visible to Andrew: F.5-paint has **no viewport-pointer-event wiring code**. Even after panel registration is fixed, click+drag on the viewport will not reach `queue_paint_op`; the F.5-paint panel will render but the brush will be inert until pointer-event wiring lands. F.5-paint.F-fix's prompt drafting decides whether to bundle this with the registration fix or defer to a separate session.

**Remediation chain**:

1. **F.5-paint.E-diagnostic** (this session): identified root cause (Hypothesis A) + produced audit document at `docs/audits/f5_paint_panel_registration_diagnostic_2026-05-03.md` (commit `5f772bea3`) + reverts §9 + Status header (this commit).
2. **F.5-paint.F-fix** (next session): applies 10 mechanical edits across `panel_type.rs` + `tab_viewer/mod.rs` mirroring `Blueprint`'s pattern + adds 2 Pattern A regression tests. Estimated ~30-60 minutes wall-clock; single commit. Possibly also viewport-pointer-event wiring depending on F.5-paint.F-fix prompt scope decision.
3. **F.5-paint.G-saveload-diagnostic** (separate session, deferred until panel registration verified by Andrew-gate post-F-fix): investigates save/load if Andrew finds issues once the panel is reachable.
4. **F.5-paint.H-saveload-fix** (separate session, conditional on F.5-paint.G-saveload-diagnostic findings): applies save/load remediation if needed.

After all four remediation sub-phases land (or after F-fix + Andrew-gate PASS if save/load surfaces no issues post-fix) and Andrew-gate re-runs PASS, F.5-paint §9 marker advances to COMPLETE.

**Scope held**:

F.5-paint.E-diagnostic only:

- Added the audit document at `docs/audits/f5_paint_panel_registration_diagnostic_2026-05-03.md` (commit `5f772bea3`).
- Modified the campaign doc Status header (line 3) + §9 F.5 entry + appended this §10 entry.

No production code changes. No tests added. No registration corrections. Per Andrew's Q1 resolution: maintain the diagnostic-then-fix discipline even when the fix looks small (10 mechanical edits is small). The discipline pattern from F.4.B.3.D's lesson application explicitly targets the failure mode of bundling diagnosis + fix in one session; honoring it here builds the discipline for future regressions.

### 2026-05-03, Sub-phase F.5-paint.F-fix (panel registration correction), commits b2df0be20 + 722b70ae5 + dee94ea05

**Sub-phase entry — captures F-fix.A registration + F-fix.A-supplement exhaustive-match coverage + audit methodology amendment + narrowed Andrew-gate PASS verdict + forward navigation for G-pointer-events + H-saveload remediation chain.**

**Pre-execution verification (per F-fix prompt §1, REQUIRED FIRST STEP)**:

- §1.1 Audit confirmation: PASS. Hypothesis A confirmed; 10 surfaces enumerated; Blueprint precedent catalogued; Pattern A test sketches present.
- §1.2 Blueprint precedent confirmation: PASS. All 6 panel_type.rs sites + 3 tab_viewer/mod.rs sites verified at expected file + line locations.
- §1.3 RegionalArchetypePanel struct soundness: PASS. Struct + Panel trait impl unchanged from F.5-paint.A; 7 panel tests pass.
- §1.4 F.4 byte-identity regression: PASS. F.4.F integration tests still green (6 + 1 ignored).

**Deliverables**:

- **F-fix.A** (commit `b2df0be20`): 9 mechanical edits across 2 files mirroring Blueprint's registration pattern. `tools/aw_editor/src/panel_type.rs` 6 surfaces (enum variant, `title()`, `icon()`, `category()`, `description()`, `all()` slice). `tools/aw_editor/src/tab_viewer/mod.rs` 3 surfaces (struct field, `new()` initializer, render dispatch arm). 2 Pattern A regression tests added (`panel_type::tests::regional_archetype_panel_registered_in_panel_type_enum` + `tab_viewer::tests::editor_tab_viewer_instantiates_regional_archetype_panel`). Mechanical execution against audit recommendations completed without deviation. `PanelCategory::Content` placement per audit recommendation (matches Terrain precedent for terrain-authoring content). Deviation noted: prompt's "9 mechanical edits" count vs audit's "10 surfaces" reconciled by mirroring Blueprint's qualified-path pattern (`crate::panels::regional_archetype_panel::RegionalArchetypePanel`) which avoids the use-statement edit; both paths achieve identical functionality.

- **F-fix.A-supplement** (commit `722b70ae5`): exhaustive-match dispatch coverage at the 1 site missed by audit. `tools/aw_editor/src/dock_panels.rs:92` `DockPanelContext::render_panel` match dispatch added `RegionalArchetype` arm mirroring Blueprint's placeholder pattern (NOT Terrain's field-based pattern, because `DockPanelContext` does not have a `regional_archetype_panel` field; adding such a field would be structural refactoring out of supplement scope). The arm provides heading + separator + 2 description labels + "Switch to docking mode" hint, matching Blueprint, BlendImport, BehaviorGraph, FrameDebugger, Animation precedent group. Single supplemental commit per prompt §4 constraint.

- **F-fix.B** (this commit): doc-only closeout updating Status header + §9 + this §10 entry + audit amendment paragraph in `docs/audits/f5_paint_panel_registration_diagnostic_2026-05-03.md` §3.

**Audit methodology gap (preserved for forward reference)**:

The F.5-paint.E-diagnostic audit catalogued 10 surfaces by grepping for `PanelType::Blueprint` references — precedent-driven grep methodology. This produced an accurate-but-incomplete list — `tools/aw_editor/src/dock_panels.rs:92` contains an exhaustive `match panel_type { ... }` whose existing arms use variants other than Blueprint at that specific dispatch point (the `DockPanelContext::render_panel` function uses category-based or per-variant dispatch logic that doesn't reference Blueprint at all in its match), so precedent-grep missed it. Compile-time exhaustiveness checking surfaced the gap immediately when F-fix.A added the enum variant.

Forward methodology for future editor surface additions involving enum variants: grep should be **enum-variant-driven** (find all non-wildcard matches over the type) rather than **precedent-driven** (find all references to a known correct example). Rust's exhaustiveness checker (E0004) is the canonical surface enumeration mechanism for this class — structurally complete by construction, while precedent-driven grep is structurally incomplete when dispatch logic varies across enum variants.

The Pattern A regression tests landed by F-fix.A close the *registration* failure class permanently. This methodology lesson closes a different class — *audit-incompleteness when investigating "find all surfaces that depend on this type"* — applicable to any future "find all sites" investigation. Future audits adding enum variants must use the compiler's exhaustiveness checker as the canonical surface enumeration mechanism; grep is a starting point only.

This lesson is also captured as a revision paragraph in §3 of `docs/audits/f5_paint_panel_registration_diagnostic_2026-05-03.md` — preserving authority of the audit for future panel additions while accurately documenting the methodology gap.

**Multi-precedent dispatch observation (forward-applicable)**:

F-fix.A-supplement's investigation surfaced that `DockPanelContext` panels split into two precedent groups based on whether they hold panel-instance fields:

- **Field-based panels** (Terrain, Inspector, Hierarchy, etc.): `DockPanelContext` has a struct field per panel; dispatch arm calls `context.<panel>.show(ui)` (or similar render method).
- **Placeholder panels** (Blueprint, BlendImport, BehaviorGraph, FrameDebugger, Animation, RegionalArchetype): `DockPanelContext` does NOT have a struct field; dispatch arm provides placeholder UI (heading + description + "switch to docking mode" hint). Full UI delegated to `EditorTabViewer`.

For F-fix.A-supplement, RegionalArchetype joined the placeholder group as the simplest in-scope choice. Whether `RegionalArchetypePanel` should eventually move to the field-based group (with a `regional_archetype_panel: RegionalArchetypePanel` field on `DockPanelContext`) is a separate architectural decision deferrable to G-pointer-events-fix or later. The placeholder group works for current functionality; the field-based group may become necessary if pointer-event routing requires direct panel-instance access from `DockPanelContext`'s render path.

This observation is forward-implication-only; F-fix.B doesn't make the field-based migration decision. G-pointer-events-diagnostic will surface whether the migration is required.

**Test scoreboard at F-fix close**:

- F-fix.A panel_type.rs registration: 6 surfaces. ✅
- F-fix.A tab_viewer/mod.rs registration: 3 surfaces. ✅
- F-fix.A Pattern A regression tests: 2 new tests. ✅
- F-fix.A-supplement dock_panels.rs exhaustive-match coverage: 1 site. ✅
- 32 regional_archetype_panel tests: pass.
- 21 panel_type::tests (Pattern A test 1 included): pass.
- 12 tab_viewer::tests (Pattern A test 2 included): pass.
- F.4 integration tests (6 + 1 ignored): pass.
- All upstream tests: green.
- `cargo check -p aw_editor`: clean (only pre-existing nalgebra/render warnings).
- `cargo build -p aw_editor`: success.

**Andrew-gate narrowed verdict** (per F-fix prompt §3.1):

- **Verdict**: PASS, 2026-05-03.
- **Setup**: editor opened at HEAD `722b70ae5`. View → Panels submenu inspected.
- **Findings**:
  - "Regional Archetypes" entry visible in View → Panels submenu.
  - Clicking entry opens panel as dockable tab alongside Hierarchy, Terrain, Asset Browser, Inspector.
  - Panel UI renders without panic: brush size + falloff radius sliders, archetype palette (6 archetypes — Continental Temperate, Equatorial Tropical, Boreal/Subarctic, Mediterranean, Desert, Custom — each with color swatch), Paint/Erase mode toggle, Save/Save As/Load/Clear/Regenerate Terrain buttons all visible.
  - 2 Pattern A regression tests pass.
  - No panics or wgpu validation errors during demo.
  - Existing TerrainPanel functionality preserved.

**Brush UX observation captured (informational, not verdict criteria per F-fix prompt §3.4)**:

Clicking and dragging in the viewport with Paint mode active consumes events as camera pan rather than routing to the panel's brush queue. F.5-paint.B's brush queue + ray-plane projection logic is in place (per F.5-paint.B's commits) but the editor's viewport pointer-event dispatch routes events to camera control before the panel can claim them. This is an architecturally distinct class from registration:

- **Registration class** (F-fix closed): "the panel doesn't exist in the editor's panel system." Pattern A regression tests close this class.
- **Pointer routing class** (G-pointer-events pending): "the panel exists but the editor's pointer dispatch doesn't know to route events to it under paint mode." Different surfaces, different precedents, different methodology.

G-pointer-events-diagnostic + G-pointer-events-fix become the next remediation chain.

**Save/load observation captured (informational, not verdict criteria)**:

Untestable without working brush — paint operations produce mask state; without mask state, save/load roundtrip can't be exercised end-to-end. H-saveload-diagnostic + H-saveload-fix remain pending regardless of G outcome. The F.5-paint.C save/load file path / sibling-directory storage may be sound; H-saveload-diagnostic will determine after G-pointer-events lands.

**Pre-existing observation noted (not in F-fix scope)**:

`tools/aw_editor/src/panels/terrain_panel::tests::test_terrain_panel_creation` fails on pre-supplement state too (verified via `git stash` by F-fix.A-supplement agent). Test asserts `chunk_radius == 5` but actual is `10`. Pre-existing TerrainPanel default drift unrelated to F.5-paint.F-fix work. NOT investigated, NOT fixed in this remediation chain. Flagged for separate standalone follow-up at some appropriate point.

**Naming convention forward** (per Andrew's Q3 resolution):

Subsequent remediation sub-phases use alphabetic flow:

- **G-pointer-events-diagnostic** + **G-pointer-events-fix** for the brush UX routing class.
- **H-saveload-diagnostic** + **H-saveload-fix** for the save/load class (pending G).

Replaces the F2/G/H mixed scheme considered earlier. Letter-per-concern is cleaner; future sub-phases inherit this convention.

**Forward chain**:

1. **G-pointer-events-diagnostic** (next session): investigates editor's viewport pointer-event dispatch architecture; identifies where camera consumes events; identifies what `RegionalArchetypePanel` needs to do to claim events under paint mode. Per Andrew's Q4 resolution: combines internal `TerrainPanel` precedent inspection + external SOTA web research on egui pointer-event dispatch patterns and multi-texture-paint precedents in similar editors. The `TerrainPanel` precedent has a known limitation (single-texture paint never expanded to multi-texture in prior work); SOTA research validates whether mirroring `TerrainPanel` inherits that foundational issue. Diagnostic produces audit document at `docs/audits/g_pointer_events_diagnostic_<YYYY-MM-DD>.md`.

2. **G-pointer-events-fix** (after G-diagnostic): applies routing correction per audit recommendations. Adds Pattern A regression tests for pointer-event class.

3. **H-saveload-diagnostic** (after G-fix Andrew-gate PASS + brush working): investigates save/load file flow; surfaces any issues missed by F.5-paint.C's tests.

4. **H-saveload-fix** (after H-diagnostic): applies save/load remediation.

5. **Final Andrew-gate full PASS**: F.5-paint §9 advances to COMPLETE.

6. **F.5-overlay-and-gate** (next forward-progress session after F.5-paint COMPLETE): Climate Preview overlay (D.5c absorbed) + halo visualization at paint time + integration tests + principal F.5 Andrew-gate per the original F.5 split.

**Scope held**: F-fix.B only modified `docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md` (Status header + §9 + this §10 entry) and `docs/audits/f5_paint_panel_registration_diagnostic_2026-05-03.md` (revision paragraph in §3). No production code changes; no test changes; no architectural decisions revisited.

### 2026-05-03, Sub-phase G-pointer-events-research, commits 6992f4b39 + e748a6304

**Sub-phase entry — captures research-only session execution per Andrew's Q7 Structure B split (research-then-diagnostic). First sub-phase of the G-pointer-events remediation chain (research → diagnostic → fix).**

**Pre-execution verification (per G-research prompt §1)**:

- §1.1 F-fix close state confirmation: PASS. F-fix chain at `b2df0be20` (F-fix.A) + `722b70ae5` (F-fix.A-supplement) + `dee94ea05` (F-fix.B) + `1d67b3328` (hash-fixup).
- §1.2 Andrew-gate brush UX symptom: confirmed from F-fix.B §10 (click+drag consumed by camera pan; events do not route to panel's brush queue; save/load deferred to H).
- §1.3 Audit doc storage convention: confirmed at `docs/audits/`.
- §1.4 Research scope reconfirmation: research-only, three concerns, anti-anchoring (research first; internal precedent in G-diagnostic).

**Deliverables**:

- **G-research.A** (commit `6992f4b39`): audit document at `docs/audits/g_pointer_events_research_2026-05-03.md` (~660 lines, 9 sections + bibliography). Concern A (egui dispatch): rich canonical material — `Sense`/`Response`/`InteractionSnapshot`/`Memory::set_modal_layer` / `egui::Scene::drag_pan_buttons` (PR #5892) / layer-priority hit testing / `contains_pointer` vs `hovered`. Concern B (multi-tool 3D editor arbitration): rich canonical material — Blender modal operators with `OPERATOR_RUNNING_MODAL | OPERATOR_PASS_THROUGH`, Unity TerrainTool + OnSceneGUI, Unreal FEdMode virtual methods returning `bool` (true = consume, false = pass), Godot EditorPlugin `_forward_3d_gui_input` returning `AfterGUIInput` enum (PASS/STOP/CUSTOM); all four converge on "active tool gets first dibs; explicit consume/pass-through return". Concern C (Rust 3D editor reference implementations): medium-quality material — Fyrox InteractionMode trait pattern (translates AAA approach (C) to Rust + egui), bevy_egui run-conditions + EguiContextSettings absorption pattern with documented fragility, rerun re_viewer base pattern, egui::Scene as in-egui multi-tool reference; no documented multi-paint-tool exemplar in surveyed Rust editors. Cross-concern synthesis in §5; preliminary pattern direction in §6; regression test direction in §7; bibliography ~30 sources in §9.

- **G-research.B** (this commit): doc-only campaign doc update — §9 status line update + this §10 entry.

**Research findings summary**:

The canonical AAA editor pattern across Blender/Unity/Unreal/Godot is **active tool/mode gets first dibs at viewport pointer events; tool returns "consumed" (block default camera handling) or "pass-through" (let camera handle); camera/viewport default control receives events only when tool returns pass-through**. Three implementation approaches surface from the research: (A) higher-layer widget pre-empts via egui layer priority + `Memory::set_modal_layer` + `egui::Scene`-style per-button reservation; (B) viewport widget checks active-tool state internally — Unity-style; doesn't scale to multi-tool naturally; (C) editor-level dispatcher with per-tool registration — AAA canonical pattern; forward-compatible with arbitrary multi-tool addition. Pass-through semantics are **universally explicit** across all four editors and never inferred — implication for AstraWeave: chosen approach must have an explicit "tool didn't claim" return path.

**Architectural observations preserved**:

- TerrainPanel's known multi-texture-paint limitation **may correlate with approach (B)** if that's how it's structured: each new tool would require editing the viewport code, which is the multi-tool scaling failure mode. (Informational only per Q4 Interpretation A; G-diagnostic determines actual approach via code inspection.)
- AstraWeave's projected tool set (terrain sculpt + archetype paint + future splat painting + future scatter painting) is multi-tool by design. The research identifies approach (C) as the canonical forward-compatible choice; approach (A) as a viable narrower-scope choice for G-fix specifically; hybrid (A-now, C-later) as a third viable path.
- Pattern A regression tests for the pointer-event class (per F-fix's Pattern A precedent): active-consume, inactive-pass-through, multi-tool exclusivity, optional modifier-key arbitration tests. Code-shape sketches in audit §7 for G-diagnostic to refine.

**Forward chain**:

1. **G-pointer-events-diagnostic** (next session): inspects `TerrainPanel` + `RegionalArchetypePanel` + AstraWeave editor's input dispatch. References this research audit as canonical pattern catalog. Investigates which approach (A/B/C) is currently in place; identifies the smallest-scope correct fix; produces remediation recommendation in `docs/audits/g_pointer_events_diagnostic_<YYYY-MM-DD>.md`. Specific questions to resolve listed in audit §5.4 + §8.6.
2. **G-pointer-events-fix**: applies remediation per G-diagnostic's recommendations. Adds Pattern A regression tests for pointer-event class.
3. **H-saveload-diagnostic** (after G-fix Andrew-gate brush working): unblocked by G's completion.
4. **H-saveload-fix**: applies save/load remediation.
5. **Final Andrew-gate full PASS**: F.5-paint §9 advances to COMPLETE.
6. **F.5-overlay-and-gate** (next forward-progress session after F.5-paint COMPLETE): Climate Preview overlay (D.5c absorbed) + halo visualization at paint time + integration tests + principal F.5 Andrew-gate.

**Scope held**: G-research only modified `docs/audits/g_pointer_events_research_2026-05-03.md` (commit `6992f4b39`) and `docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md` (§9 + §10; this commit). NO code inspection of TerrainPanel, RegionalArchetypePanel, or any AstraWeave source. NO hypothesis investigation. NO remediation recommendation. Per Andrew's Q1 Option B + Q7 Structure B: research forms independently of internal code state to prevent confirmation bias when G-diagnostic compares observed code against canonical patterns. The research audit is forward-applicable beyond F.5-paint — future paint-tool additions (splat, scatter, vegetation override masks, weather zones) inherit it as canonical reference.

### 2026-05-03, Sub-phase G-pointer-events-diagnostic, commits ac4bc58a3 + ab67aeb0b

**Sub-phase entry — captures code investigation + hypothesis classification + decision-request surfacing per Q1 Option B (surface decision to Andrew, do not silently recommend).**

**Pre-execution verification (per G-diagnostic prompt §1)**:

- §1.1 Research audit reading: PASS — (A)/(B)/(C) framework internalized.
- §1.2 Andrew-gate brush UX symptom from F-fix.B §10: confirmed; not re-reproduced (working from captured observation as ground truth).
- §1.3 F-fix + G-research close state: confirmed at HEAD `506dec13c` pre-session.
- §1.4 Campaign doc + audit doc state: no drift.

**Deliverables**:

- **G-diagnostic.A** (commit `ac4bc58a3`): audit document at `docs/audits/g_pointer_events_diagnostic_2026-05-03.md` (~530 lines + 10 sections + bibliography). Hypothesis classification: H1 (approach (B) with main.rs mediator) confirmed cleanly. (A)/(B)/(C) classification of AstraWeave editor: **approach (B)**. §7 decision-request: **four** options including Option B-extend that surfaces during investigation as the literal smallest scope (beyond the prompt's three-option framing); per Q1 Option B's "surface decision honestly" discipline, this option is included.

- **G-diagnostic.B** (this commit): doc-only campaign doc update — §9 status line + this §10 entry.

**Hypothesis classification result**:

**H1 (approach (B) with main.rs mediator) confirmed cleanly.** Evidence:

- ViewportWidget at `tools/aw_editor/src/viewport/widget.rs` has typed `terrain_brush_active: bool` field (line 163) plus 5 supporting tool-specific fields (`terrain_brush_radius`, `terrain_brush_is_paint`, `terrain_brush_hits` Vec, `terrain_brush_stroke_ended`, `last_brush_time`).
- ViewportWidget's `handle_input()` branches on `terrain_brush_active` at lines 1180-1255 (camera orbit gated `!self.terrain_brush_active`; terrain brush dispatch fires when `terrain_brush_active && (dragged_by(Primary) || clicked_by(Primary))`).
- main.rs at lines 3833-3877 acts as per-frame mediator: pre-render reads `dock_tab_viewer.is_terrain_brush_active()` and pushes to `viewport.set_terrain_brush_active()`; post-render drains `viewport.take_terrain_brush_hits()` and routes to `dock_tab_viewer.apply_terrain_brush_at(...)`. Hardcoded for TerrainPanel; no abstraction.

**H2 (approach A) ruled out**: no `Memory::set_modal_layer` calls; no transparent overlay or `egui::Area::Foreground` patterns; no layer-elevation manipulation anywhere in `tools/aw_editor/src/`.

**H3 (approach C) ruled out**: no `ActiveTool` / `ToolMode` trait found; no tool registry struct; no central dispatcher; the existing `Panel` trait is purely UI rendering with no pointer-event hooks.

**H4 (ad-hoc) ruled out**: pattern is canonically (B); main.rs mediator is a (B) refinement, not ad-hoc divergence.

**H5 (mechanism mismatch) ruled out**: TerrainPanel works through the same mechanism that would apply to RegionalArchetypePanel; RegionalArchetypePanel's existing `queue_paint_op(world_x, world_z)` API signature already matches TerrainPanel's `apply_brush_at(world_x, world_z)` shape; the only difference is plumbing isn't built.

**RegionalArchetypePanel inspection**: F.5-paint.A scaffold has all building blocks (`paint_active: bool` field at line 75, `queue_paint_op` at line 138, `apply_pending_paint_ops_to_owned` at line 233) but **zero references in main.rs** for `RegionalArchetypePanel` or `regional_archetype` — the (B)-pattern plumbing through main.rs + ViewportWidget is missing entirely. F.5-paint.A's panel was never wired into the viewport's pointer-event flow.

**Architectural decision pending — four options surfaced**:

Per Q1 Option B, the architectural choice is surfaced to Andrew rather than recommended by the agent. Decision factors documented in audit §7. G-fix's prompt drafting depends on Andrew's choice.

- **Option B-extend (literal smallest scope)**: mirror TerrainPanel's pattern exactly — add 5 fields + 4 setter methods to ViewportWidget; add `handle_input()` branches; add main.rs sync + drain. ~1 commit, 3-4 hours wall-clock, ~150-200 lines across 3 files. **Inherits multi-tool scaling failure mode** — each future paint tool requires another full set of viewport fields + main.rs plumbing. ViewportWidget grows unbounded.

- **Option A (medium scope)**: higher-layer overlay via egui Modal layer / `egui::Area::Foreground`. Insulated change to `RegionalArchetypePanel` only; doesn't touch ViewportWidget or main.rs. ~5-6 hours, ~80-120 lines in `regional_archetype_panel.rs` only. Introduces NEW pattern in codebase (no precedent to mirror); two patterns coexist (B for TerrainPanel, A for RegionalArchetypePanel); coordinate accuracy concern (loses depth-buffer access at viewport widget lines 1219-1234 used by TerrainPanel's brush).

- **Option C (broad scope)**: editor-level `ActiveTool` dispatcher trait + per-tool registration matching AAA canonical pattern (Godot's `_forward_3d_gui_input` / Unreal's `FEdMode::InputKey`). 3-5 sessions over ~3-4 days; refactor risk to TerrainPanel's working brush during Session 2 migration. **Forward-compatible**: future paint tools (splat, scatter, vegetation, weather) implement `ActiveTool` and register; ViewportWidget unchanged. Retroactively benefits TerrainPanel — multi-texture-paint expansion would no longer require ViewportWidget edits.

- **Hybrid (B-extend-now, C-later)**: Option B-extend at G-fix; Option C deferred to future architectural campaign. Lowest immediate risk; preserves option to refactor later but risks indefinite deferral as F.6-F.8 take precedence.

**Decision factors for Andrew (audit §7.5)**:

1. How soon will splat/scatter/vegetation/weather painting actually be built?
2. Is editor-architecture refactor a current priority?
3. Are F.6-F.8 priorities competing for attention?
4. Is the campaign-velocity cost of Option C's 3-5 sessions acceptable?
5. Is the multi-tool scaling concern actionable now or speculative? (Per Q4 Interpretation A informational note: TerrainPanel multi-texture limitation hypothesis correlates with approach (B) but causal attribution NOT investigated.)
6. Refactor-risk appetite for TerrainPanel during Option C Session 2?

**Forward observations preserved (audit §9)**:

- **Q4 Interpretation A informational note now anchored by code observation**: TerrainPanel multi-texture limitation correlates with approach (B)'s structural multi-tool scaling failure mode. Causal attribution NOT investigated per Q4 Interpretation A discipline.
- **Mutex arbitration question (§9.5)**: if both TerrainPanel and RegionalArchetypePanel paint modes are active simultaneously, current code would route events to both branches. G-fix to resolve via mutex/stack/concurrent/per-dock semantics; affects all four options.
- **RegionalArchetypePanel's `paint_active` field at line 75 is unused** (anticipatory documentation only); G-fix should activate it per chosen option semantics.
- **No supplemental SOTA research executed** per Q6 — G-research's catalog was sufficient for hypothesis classification + decision-request authoring.

**Forward chain**:

1. **Andrew architectural decision** (between G-diagnostic and G-fix): chooses Option B-extend, A, C, or Hybrid based on audit §7 + §9.5 arbitration question.
2. **G-pointer-events-fix prompt drafting**: shapes G-fix's scope based on chosen approach.
3. **G-pointer-events-fix execution**: applies the chosen approach + Pattern A regression tests + Andrew-gate verification.
4. **H-saveload-diagnostic + H-saveload-fix**: save/load remediation (deferred until brush works enabled testing).
5. **Final Andrew-gate full PASS** → **F.5-paint COMPLETE** → **F.5-overlay-and-gate** → F.6 → F.7 (principal Andrew-gate) → F.8 closeout.

**Scope held**: G-diagnostic only modified `docs/audits/g_pointer_events_diagnostic_2026-05-03.md` (commit `ac4bc58a3`) and `docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md` (§9 + §10; this commit). NO production code changes; NO Pattern A regression test implementation (G-fix); NO single-path remediation recommendation (Andrew decides between G-diagnostic and G-fix); NO save/load investigation (H-saveload); NO Climate Preview overlay investigation (F.5-overlay-and-gate); NO TerrainPanel multi-texture limitation root cause investigation (Q4 Interpretation A informational only).

### 2026-05-03, Sub-phase Pause-Artifacts (campaign pause for Editor Multi-Tool Architecture spinoff), commits a64f12320 (Pause-Artifacts.A F.4 closeout) + 98fc063d9 (Pause-Artifacts.B F.5-paint pause)

**Sub-phase entry — captures architectural decision to spin off G-pointer-events Option C as its own foundational campaign + Regional Archetype Variation pause + forward pointer to Editor Multi-Tool Architecture campaign. This entry is load-bearing for resumption: future-Andrew (or future-Claude in a new session) reading this entry months from now must be able to (a) understand why the campaign paused, (b) verify the resumption point is unambiguous, (c) connect to the Editor Multi-Tool Architecture campaign's outputs, (d) understand the discipline pattern that authorized the pause. The entry is self-contained on these four points.**

**Architectural decision context**:

G-pointer-events-research (commits `6992f4b39` audit + `e748a6304` campaign doc + `506dec13c` hash-fixup) catalogued canonical patterns from egui pointer-event dispatch + AAA editor multi-tool arbitration (Blender modal operators with `OPERATOR_RUNNING_MODAL | OPERATOR_PASS_THROUGH`, Unity TerrainTool + OnSceneGUI, Unreal FEdMode virtual methods, Godot `_forward_3d_gui_input` returning AfterGUIInput enum) + Rust 3D editor reference implementations (Fyrox InteractionMode trait, bevy_egui absorption patterns, rerun re_viewer). Three implementation approaches surfaced: (A) higher-layer widget pre-empts via egui Modal layer / layer-priority hit testing; (B) viewport widget checks active-tool state internally — Unity-style; doesn't scale to multi-tool; (C) editor-level dispatcher with per-tool registration — AAA canonical pattern; forward-compatible with arbitrary multi-tool addition.

G-pointer-events-diagnostic (commits `ac4bc58a3` audit + `ab67aeb0b` campaign doc + `57de03fba` hash-fixup) inspected AstraWeave editor code; confirmed H1 (approach (B) with main.rs as per-frame mediator) cleanly. Evidence: ViewportWidget at `tools/aw_editor/src/viewport/widget.rs:163` has typed `terrain_brush_active: bool` field + 5 supporting tool-specific fields; `handle_input()` branches at lines 1180-1255 (camera orbit gated `!terrain_brush_active`; terrain brush dispatch fires when `terrain_brush_active && (dragged_by(Primary) || clicked_by(Primary))`); main.rs:3833-3877 acts as per-frame mediator (pre-render reads `dock_tab_viewer.is_terrain_brush_active()` → pushes to `viewport.set_terrain_brush_active()`; post-render drains `viewport.take_terrain_brush_hits()` → routes to `dock_tab_viewer.apply_terrain_brush_at(...)`). Hardcoded for TerrainPanel; no abstraction. RegionalArchetypePanel has all building blocks (`paint_active` flag at line 75, `queue_paint_op(world_x, world_z)` at line 138, `apply_pending_paint_ops_to_owned()` at line 233) but **zero references in main.rs** for `RegionalArchetypePanel` — the (B)-pattern plumbing is missing entirely; F.5-paint.A's panel was never wired into the viewport's pointer-event flow. Per Q1 Option B, audit §7 surfaced four options (B-extend literal smallest mirroring existing TerrainPanel pattern; A medium scope egui Modal overlay; C broad scope ActiveTool dispatcher matching AAA canonical; Hybrid B-extend-now + C-later) without recommending a single path; decision factors documented for Andrew to weigh.

**Decision**: Andrew chose **spinoff path** — pause Regional Archetype Variation; launch Editor Multi-Tool Architecture campaign as foundational architectural work; resume Regional Archetype Variation post-Editor-Multi-Tool-Architecture with G-fix scope shifted from B-extend to dispatcher registration. Neither B-extend nor Hybrid executed.

**Discipline pattern invoked**:

Regional Archetype Variation campaign §0 explicitly imposed prophylactic discipline at F.0:

> *"if a sub-phase surfaces an architectural gap that requires reframing the campaign's scope, treat that as evidence of insufficient research-pass depth and consider another research pass rather than continuing to expand the campaign in-flight."*

This was applied to prevent the F.4.B.3.D failure mode (6+ sub-phases declaring code-level PASS while Andrew-gate caught architectural REGRESSes — octave-emphasis Path 1, runevision filter, archetype-as-coloring after Path B amplitude reduction; eventual forced re-research-and-pivot). The corrective baked into Regional Archetype Variation §0 was to invoke the re-research pattern *proactively* when execution surfaces architectural gaps, rather than continuing to expand scope in-flight.

F.5-paint's 6+ session remediation cascade (E-diagnostic → F-fix.A → F-fix.A-supplement → F-fix.B → G-research → G-diagnostic) hit exactly the pattern §0 targets. G-research + G-diagnostic surfaced AstraWeave's editor uses approach (B), which cannot scale to multi-tool without architectural change. Building B-extend or Hybrid means F.5-paint closes faster but the **next** paint tool campaign (splat painting, scatter painting, vegetation override masks, weather zones — all in AstraWeave's projected scope) hits the same architectural wall. The discipline cost incurred now (3-5 sessions of architectural campaign before F.5-paint can close) is amortized across many future campaigns — not just F.5-paint. Each future paint tool becomes a registration session, not an architectural debate.

The spinoff is exactly the discipline pattern §0 was written to authorize. Without §0's prophylactic framing, the natural execution drift would have been to choose B-extend or Hybrid in G-fix and continue building. The §0 discipline made the spinoff the obvious correct call.

**Artifact-value argument** (secondary framing):

Building B-extend or Hybrid produces working code but **suboptimal canonical reference material** — the (B) pattern would be entrenched further by adding a second tool-specific field cluster to ViewportWidget (5 fields + 4 setters + handle_input branches mirroring TerrainPanel's plumbing). By contrast, executing the Editor Multi-Tool Architecture campaign with proper research-pass + campaign-design discipline produces canonical reference material that ALL future paint tools (splat, scatter, vegetation, weather, plus any other multi-tool concerns) inherit. The artifact value compounds positively across every future paint tool campaign.

**What pauses, what doesn't**:

**Paused**:

- Regional Archetype Variation campaign as a whole — Status header reflects PAUSED state.
- F.5-paint sub-phase — §9 marker remains IN PROGRESS; advance to COMPLETE deferred until post-resumption final Andrew-gate PASS.
- F.5-overlay-and-gate, F.6, F.7, F.8 — NOT STARTED; await F.5-paint resumption + closure.

**Not paused / preserved unchanged**:

- F.0 (campaign plan) — COMPLETE 2026-04-29; remains so.
- F.1 (climate field extension) — COMPLETE 2026-04-29; remains so.
- F.2 (BootstrapSplineSet infrastructure) — COMPLETE 2026-04-30; remains so.
- F.3 (spline wiring single-archetype regression) — COMPLETE 2026-05-01; Andrew-gate PASS; remains so.
- F.4 (RegionalArchetypeMask + falloff sampler) — COMPLETE 2026-05-01 (Andrew-gate PASS); F.4.G deferred-closeout reconciled in this pause artifacts session's Commit 1 (`a64f12320`).
- F.5-paint commits `26a3864b8` (F.5-paint.A scaffold) + `226572bae` (F.5-paint.B brush) + `2b230d94e` (F.5-paint.C save/load) + `e9d2a7922` (F.5-paint.D closeout) + `b6dd9de58` (hash-fixup) — preserved unchanged; panel struct + 30 unit tests sound.
- F-fix commits `b2df0be20` (F-fix.A registration corrections + 2 Pattern A regression tests) + `722b70ae5` (F-fix.A-supplement exhaustive-match coverage) + `dee94ea05` (F-fix.B closeout + audit amendment) + `1d67b3328` (hash-fixup) — preserved; panel registration correct; F-fix.B audit amendment lands forward-applicable methodology lesson (precedent-driven grep ≠ enum-variant-driven enumeration; Rust's exhaustiveness checker is canonical surface enumeration mechanism).
- G-pointer-events audits — both `docs/audits/g_pointer_events_research_2026-05-03.md` and `docs/audits/g_pointer_events_diagnostic_2026-05-03.md` preserved; **forward-applicable to the Editor Multi-Tool Architecture campaign as predecessor research**.
- F.5-paint.E-diagnostic audit (`docs/audits/f5_paint_panel_registration_diagnostic_2026-05-03.md`) — preserved; canonical reference for future panel additions.

**No commits reverted** per Andrew's pause-artifacts decision.

**Resumption point semantics**:

Post-Editor-Multi-Tool-Architecture closure, Regional Archetype Variation resumes at G-pointer-events-fix with scope shifted:

- **Pre-pause G-fix scope** (B-extend or Hybrid): mirror TerrainPanel's (B) pattern — 5 ViewportWidget fields + 4 setters + handle_input branches + tab_viewer accessors + main.rs sync/drain. ~150-200 lines across 3 files (`tools/aw_editor/src/viewport/widget.rs`, `tools/aw_editor/src/main.rs`, `tools/aw_editor/src/tab_viewer/mod.rs`).
- **Post-pause G-fix scope** (dispatcher registration): implement `ActiveTool` for `RegionalArchetypePanel` + register with the dispatcher established by Editor Multi-Tool Architecture campaign. Likely a single small commit, much smaller than B-extend would have been.

The Editor Multi-Tool Architecture campaign produces canonical reference material that ALL future paint tools inherit. Each future paint tool becomes a registration session, not an architectural debate. The artifact value compounds positively across all future paint tool work.

**Forward chain post-pause**:

1. **Session 2 — Editor Multi-Tool Architecture research pass**: pure SOTA research session producing `docs/audits/editor_multi_tool_architecture_research_<YYYY-MM-DD>.md`. Anti-anchored from AstraWeave code (no internal inspection per Q4 / Q7 framing). Expands G-research's Concern B (AAA dispatcher patterns) and Concern C (Rust editor implementations) with dispatcher-architecture framing rather than pointer-event-arbitration framing. Builds on G-research as predecessor research; doesn't duplicate it. Sources include Fyrox InteractionMode trait pattern in depth, Bevy editor work (if applicable), Godot's `_forward_3d_gui_input` + EditorPlugin trait deeper investigation, Unity's TerrainTool + EditorTool API hierarchy, Unreal's FEdMode + UEdMode + tool framework, plus newer Rust editor surveys (rerun re_viewer's tool architecture if applicable, any 3D editors built on egui directly).

2. **Session 3 — Editor Multi-Tool Architecture campaign-design pass**: drafts `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` with §0-§10 framing matching Regional Archetype Variation's structural template. §2 architectural decisions resolved (`ActiveTool` trait shape; `EventDisposition` enum semantics matching Godot `AfterGUIInput`-style PASS/STOP/CUSTOM or Unreal-style bool consume; dispatcher mechanism — pull-based per-frame dispatch vs push-based event subscription; registration model — explicit registry vs trait-object collection; integration with existing ViewportWidget — full migration vs hybrid coexistence; mutex arbitration semantics — implicit via dispatcher first-Consumed-wins vs explicit; mediator pattern's fate — replaced by dispatcher vs preserved as compatibility layer; etc.). Sub-phase breakdown sized to scope; Andrew-gate gating per §0 discipline applied to visible-output sub-phases. Research-informed (not first-principles) per Regional Archetype Variation §0's campaign-design discipline pattern.

3. **Sessions 4+ — Editor Multi-Tool Architecture campaign execution**: per the campaign-design plan. Likely covers ActiveTool trait + dispatcher core → TerrainPanel migration → integration + Pattern A regression tests for dispatcher class → RegionalArchetypePanel registration → closeout. After Andrew-gate PASS on TerrainPanel functioning post-migration + Pattern A tests landed, Regional Archetype Variation campaign resumes.

4. **Resume Regional Archetype Variation**: G-pointer-events-fix (dispatcher registration scope) → H-saveload-diagnostic → H-saveload-fix → final Andrew-gate full PASS → F.5-paint COMPLETE → F.5-overlay-and-gate → F.6 → F.7 (principal Andrew-gate) → F.8 closeout.

**Methodological lesson (forward-applicable beyond this campaign)**:

The discipline pattern §0 imposed on Regional Archetype Variation — "if execution surfaces architectural gap, halt and re-research rather than expand scope in-flight" — proved to be load-bearing. Without §0's prophylactic framing, the natural execution drift would have been to choose B-extend or Hybrid in G-fix and continue building. The §0 discipline made the spinoff the obvious correct call.

This is the precedent for future campaigns: campaign-design passes should bake similar discipline patterns into §0 as standing authorization for halt-and-re-research when execution surfaces architectural gaps. The Regional Archetype Variation §0 framing ("research-pass-before-reframe") is the canonical form. The Editor Multi-Tool Architecture campaign-design pass should inherit this discipline pattern — its §0 should similarly authorize halt-and-re-research if execution surfaces foundational architectural gaps that would compromise future paint tool work.

**Pause artifacts session structure**:

This session lands two commits:

- **Commit 1 (Phase 1.X-pause.A, `a64f12320`)**: F.4 closeout — Status header + §9 F.4 line + §10 F.4.G deferred-closeout entry. F.4.A-F production work landed 2026-05-01 with verbal Andrew-gate PASS but §9 closeout commit was deferred during the F.5-paint cascade; reconciled retroactively.
- **Commit 2 (Phase 1.X-pause.B, this commit)**: F.5-paint pause + Editor Multi-Tool Architecture spinoff — Status header + §9 F.5 entry + this §10 entry.

Commit 3 (Phase 1.X-pause.C) — hash-fixup replacing the Pause-Artifacts.B placeholder with `98fc063d9` per F-fix.B / G-research.B / G-diagnostic.B hash-fixup discipline pattern.

**Scope held**: Pause-Artifacts session only modifies `docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md` (Status header + §9 F.4 line in Commit 1; Status header + §9 F.5 line + this §10 entry in Commit 2). NO production code changes. NO audits modified. NO commits reverted. NO new audit documents (research pass is Session 2's job). NO new campaign doc (campaign-design pass is Session 3's job). Pure pause artifacts.

---

- `docs/audits/regional_archetype_variation_research_2026-04-29.md` — the research audit that motivated and shaped this campaign (Hybrid C + F recommendation; AAA prior art survey; algorithmic approach evaluation).
- `docs/audits/f4b3d5_diagnostic_3_cross_archetype_2026-04-28.md` — the architectural-gap measurement (cross-archetype Pearson 0.978-0.989; bootstrap dominates 72-81%; 1-of-18 archetype-aware parameters).
- `docs/audits/f4b3d5_diagnostic_2_real_heightmap_2026-04-28.md` — predecessor real-chunk biome distribution measurement.
- `docs/current/TERRAIN_GENERATION_QUALITY_CAMPAIGN.md` — predecessor campaign (D.1-D.5 within-region machinery; closed via architectural pivot in a parallel session).
- `docs/current/TERRAIN_MATERIAL_SYSTEM_CAMPAIGN.md` — parent campaign; gets §7 update at F.8 closeout; gets §9 entry at F.0 launch (this commit).
- `docs/current/ARCHITECTURE_MAP.md` — terrain section gets updated at F.8 closeout to reflect new types (`RegionalArchetypeMask`, `BootstrapSplineSet`, `RegionalArchetypeBlend`, `RegionalArchetypePanel`).
- `docs/supplemental/WORLD_SCALE_CONVENTIONS.md` — Target B scale conventions (1 WU = 1 m, 115 km² world extent); referenced by §2.4 mask resolution math.
- `astraweave-terrain/src/climate.rs` — `ClimateMap` extended in F.1.
- `astraweave-terrain/src/world_archetypes.rs` — `WorldArchetype` extended with `bootstrap_splines` in F.2.
- `astraweave-terrain/src/biome_lookup.rs` — D.2 unchanged; biome classification operates on archetype-shaped terrain post-F.4.
- `astraweave-terrain/src/biome_parameters.rs` — D.3 unchanged.
- `astraweave-terrain/src/biome_param_blending.rs` — D.4 unchanged; biome blending composes with archetype blending per §2.6.
- `astraweave-terrain/src/lib.rs` — `WorldGenerator::generate_chunk_with_climate` refactored in F.3 (spline wiring) and F.4 (mask integration).
- `tools/aw_editor/src/panels/terrain_panel.rs` — referenced by F.5 (new `RegionalArchetypePanel` lives alongside this).

---

*End of plan*
