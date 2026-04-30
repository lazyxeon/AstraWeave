# Regional Archetype Variation Campaign — Phase 1.X

**Status**: F.0 (campaign plan) COMPLETE 2026-04-29, commit 0e51763d4. F.1 (climate field extension) COMPLETE 2026-04-29, commits `5fcde4e98` (F.1.A) + `744132c6a` (F.1.B) + `8e883beb5` (F.1.C) + df7636fa3 (F.1.D). F.2-F.8 NOT STARTED.
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
F.1 — Climate field extension: COMPLETE 2026-04-29, commits 5fcde4e98 (F.1.A PvFold + spline types) + 744132c6a (F.1.B ClimateSample extension) + 8e883beb5 (F.1.C unit tests + regression verification) + TBD (F.1.D closeout). New 5-field ClimateSample (3 D.1 + erosion + weirdness) with PvFold helper. 14 F.1.C extension tests + 8 F.1.A spline_types tests + 18 D.1 climate::tests (backward-compat verified) all green. Seed offsets +3000 / +4000 verified clear of all other terrain crate noise instances; PV formula hand-verified at 7 canonical points; D.1 climate state inspected pre-F.1.B and matches D.1 documented state. No Andrew-gate (F.1 produces no visible terrain change per campaign §0).
F.2 — BootstrapSplineSet infrastructure: NOT STARTED.
F.3 — Spline wiring (single archetype regression): NOT STARTED.
F.4 — RegionalArchetypeMask + falloff sampler: NOT STARTED.
F.5 — Editor UI for archetype painting: NOT STARTED.
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
