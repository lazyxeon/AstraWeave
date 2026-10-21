# Uber Noise Research — 2026-04-25

## Scope

Phase 1.6-F.4.B.3.A: literature research on Hello Games' Uber Noise system and related procedural-terrain noise transforms. Produces decision-quality input for F.4.B.3.B-onward implementation phases. Identifies which transforms to implement, in what order, with what algorithmic specifics, and how they compose with AstraWeave's existing pipeline.

## Source provenance

Throughout this document, claims are tagged with provenance level:

- **[Murray-direct]** — directly named in the Gemini AI summary of Murray's GDC 2017 "Building Worlds in No Man's Sky Using Math(s)" talk. Not a verbatim transcript; the summary is the best available primary-source proxy because the talk has no public transcript and GDC Vault requires subscription.
- **[Murray-implied]** — strongly suggested by the Murray summary but not explicitly named. E.g., "non-repeating environments" hints at locality without naming a system.
- **[McKendrick-direct]** — directly named in the Gemini AI summary of McKendrick's GDC 2017 "Continuous World Generation in 'No Man's Sky'" talk. Same provenance caveat.
- **[Quilez]** — from Inigo Quilez's procedural-noise articles (https://iquilezles.org/articles/). Canonical reference for fBm, domain warping, and analytical-derivative noise.
- **[Musgrave]** — from F. Kenton Musgrave's published procedural-terrain work (1994 onward). Predates NMS; standard academic reference for ridged multifractal.
- **[runevision]** — from Rune Skovbo Johansen's blog posts (2026), specifically the gradient-based erosion filter and Phacelle directional noise.
- **[Minecraft-wiki]** — from the Minecraft 1.18+ MultiNoise system documentation (https://minecraft.wiki/w/World_generation).
- **[Secondary]** — appears in tertiary sources (ithy.com, NMS modding wiki extracts) attributed to Uber Noise but NOT visible in Murray's summary or with weak/uncertain primary attribution.
- **[Inferred]** — research-output reasoning, not directly published anywhere.

## What AstraWeave already has

(All references are to phases of the Phase 1.6-F campaign — see `docs/current/TERRAIN_GENERATION_QUALITY_CAMPAIGN.md` §9 for full list.)

- **Domain warping** (F.2.B, 2026-04-21). Multi-iteration warp of fBm by another fBm. **[Murray-direct: confirmed Uber Noise feature.]** Active on 5 of 8 biome presets (grassland, mountain, forest, tundra, desert).
- **Continental amplitude modulation** (F.2.6, 2026-04-21). One low-frequency continental field multiplies mountain layer amplitude. Currently scale 0.0003 / wavelength ~3300 WU at Target B (F.4.B.2.F). **[Inferred partial locality]** — current implementation is a single-parameter Path B (continuous blend field) varying amplitude only.
- **Derivative-weighted fBm** (F.2-T-4, 2026-04-22). Quilez "morenoise" formula: `a += b*n.x / (1 + dot(d,d))`. Uses analytical derivatives **[Quilez]** to attenuate high-frequency octaves on steep slopes. **[Murray-direct primitive: analytical derivatives.]** The application (octave attenuation) is one of multiple possible uses.
- **Nyquist octave cap** (F.2-T-3.C.1) per PBR §10.6 formula `n_max = -1 - log2(l)`.
- **Analytical-gradient Perlin** (`astraweave-terrain/src/perlin_gradient.rs`). Provides the gradient primitive Murray identifies as load-bearing.
- **Particle hydraulic + thermal erosion** (F.3-phase-2.C with phase-3.C world-coord seeding + phase-4.B shared-vertex averaging). **Stateful** simulation. Composes with Uber Noise rather than being replaced.
- **Mountain Drama slider** (F.4.B.2.B). User-tunable 0.4-2.0 multiplier on `mountains_amplitude`.

## Source attribution table for Uber Noise features

| Feature | Murray-direct | Secondary support | Confidence |
|---|---|---|---|
| Domain warping of fBm | YES | Strong | HIGH |
| Slope-conditional cragginess (analytical derivatives) | YES | Strong (ithy.com "slope erosion") | MEDIUM-HIGH |
| Octave-emphasis tuning ("first octave no longer dominates") | YES (concept named) | NONE published with numeric weights | MEDIUM (concept) / LOW (specifics) |
| Altitude-based modulation | NO (not in Murray summary) | Plausibly Musgrave-derived; ithy.com claims it without primary citation | LOW-MEDIUM |
| Concavity / curvature modification | NO | Not corroborated in any accessible source as Uber Noise feature | NOT SUPPORTED |
| Plateaus / terraces | NO | ithy.com only, no primary citation | VERY LOW |
| Multi-scale locality (parameter-variation across regions) | IMPLIED ("non-repeating environments") | McKendrick-direct ("solar system → planet → local") | MEDIUM (concept) |

**Critical takeaway:** Murray's summary directly names exactly THREE Uber Noise transforms — domain warping, slope-conditional analytical-derivative cragginess, and octave-emphasis. Everything else commonly attributed to "Uber Noise" in tertiary sources has weak or absent primary attribution. AstraWeave should be honest about this: F.4.B.3 implements the Murray-direct features rigorously; "altitude erosion", "concavity", "terraces" are out of scope unless Andrew-gate post-implementation reveals a specific need.

## What Uber Noise adds (per literature) — ranked for Veilweaver

### Rank 1: Octave-emphasis tuning [Murray-direct concept; LOW-confidence specifics]

**Concept:** Murray names damping octave 0 to shift emphasis to higher octaves, breaking the "first octave dominates" pattern that produces uniform peak shapes. **[Murray-direct]**

**Critical research finding:** **No published source provides specific numeric octave weights** (e.g., "octave 0 = 0.5, octave 1 = 0.7, octave 2 = 0.6"). The literature uses either:
- **Standard H-parameter fBm** with G = 2^(-H) uniform exponential decay. **[Quilez]** validates H=1 (G=0.5) as physically motivated for terrain (matches -9 dB/octave natural mountain spectra).
- **Musgrave signal-feedback**: `weight = clamp(prev_signal * gain, 0, 1)` — gain modulated by previous octave's signal, not a static array. **[Musgrave]**
- **Multifractal alpha functions**: `weight_k = f(accumulated_value_so_far)`. Dynamic, not static.

The "hypothetical per-octave weight array" pattern in F.4.B.3.A's prompt has no published backing. AstraWeave would be doing a bespoke tuning exercise.

**Risk:** Standard H=1, G=0.5 is **physically validated for terrain realism** per Quilez's article (https://iquilezles.org/articles/fbm/). Naively damping octave 0 may produce visually different output but at the cost of departing from the spectral profile that makes natural-looking mountains. The Andrew-gate "cartoon shape" perception may not be primarily caused by octave 0 dominance — could also be uniform amplitude (continental field's continental_min = 0.50 caps lowland reduction), uniform character (all peaks pass same pipeline), or absence of regional variation.

**Implementation paths:**

1. **Static per-octave amplitude weights array.** Simplest. Replace `b *= 0.5` (standard) with `b = WEIGHTS[i]` (custom). Try octave 0 = 0.6, octave 1 = 0.8, octave 2 = 0.6, octave 3 = 0.4 as starting point — boost mid-frequencies. Iterate via Andrew-gate. **[Inferred]**

2. **Modify Hurst parameter dynamically per layer.** Different H for base/mountain/detail layers. E.g., base H=1.0 (smooth), mountain H=0.7 (rougher), detail H=1.3 (smoother high-frequency). **[Inferred]** No published guidance on per-layer H values.

3. **Adopt Musgrave signal-feedback for one or more layers.** Replaces fixed octave weighting with `weight = signal * gain`. Higher-altitude regions get more fine detail; lowlands get less. **[Musgrave]** Has published reference implementations (SharpNoise, Isara). Notably, this is *altitude-conditional octave modulation*, not just octave-emphasis — it conflates Rank 1 with secondary "altitude erosion" claims.

**Recommendation:** Path 1 (static per-octave weights) for F.4.B.3.B as the cheapest test. If Andrew-gate shows visual improvement, refine. If gate shows no improvement, investigate Path 3 (Musgrave signal-feedback) as a more structurally-motivated alternative.

**Implementation complexity:** TRIVIAL (~5 lines per fBm call site).
**Per-vertex cost:** ZERO (same number of samples, different weights).
**Veilweaver fit:** UNCERTAIN. Could materially improve cartoon-shape problem; could be no-op if root cause is locality/repetition rather than spectral profile.
**Recommendation:** F.4.B.3.B with explicit Andrew-gate decision point — if cheapest fix doesn't visibly help, don't iterate within this phase, move to Rank 2/3.

### Rank 2: Extended derivative-conditional modulation [Murray-direct framing; runevision-direct algorithm]

**Concept:** Murray names cragginess on slopes via analytical derivative as the slope-awareness application. **[Murray-direct]** F.2-T-4's `derivative_weighted_fbm` uses derivatives for octave attenuation — Quilez's morenoise smoothing. **Murray's framing is broader: derivatives drive multiple applications, not just one.**

**Critical observation about direction:** Quilez morenoise (F.2-T-4 already lands this) **suppresses** high-frequency octaves on steep terrain (smoothing → soft slopes, sharp ridges). Murray's "cragginess on slopes" implies the **opposite direction** — *adding* high-frequency detail on steep terrain. These are complementary operations, not the same.

**runevision filter (March 2026)** is the most complete published implementation of Murray's broader vision **[runevision-direct]**:

- Takes height function with pre-computed gradient as input (AstraWeave already has analytical gradient via Quilez morenoise).
- Adds "extruded sine wave" gullies oriented along gradient direction. Gully height is cosine-wave; gully slope is sine-wave.
- Uses gradient direction to align gullies with downslope flow.
- Fades gullies via `fadeTarget = inverse_lerp(valleyAlt, peakAlt, h) * 2.0 - 1.0` — at valleys (low h) fades toward -1 (no gullies), at peaks (high h) fades toward +1 (full gullies). **This is altitude conditioning, layered on top of slope conditioning.**
- Multi-octave with `combiMask = pow_inv(combiMask, detail) * newMask` — restricts smaller gullies to steeper sub-features of larger ridges.
- **Layers ON TOP of existing particle erosion**, doesn't replace. Composition: `base_noise → particle_erosion → runevision_filter`.
- Outputs modified heights, analytical derivatives, and a ridge map (white = ridges, black = creases).

**Source:** https://blog.runevision.com/2026/03/fast-and-gorgeous-erosion-filter.html (full GLSL formulas in the blog, MPL-2.0 licensed).

**Why this matters for Veilweaver:** the cartoon-shape problem and the "all peaks look the same" problem are partly about *insufficient mesoscale detail*. Quilez morenoise smooths steep slopes — the ridges and valleys that would naturally exist *between* the smoothed soft slopes are absent because morenoise alone doesn't add them. runevision's filter adds exactly those features, in the right places (along gradient flow), at the right altitudes (faded by altitude), at the right scales (multi-octave, restricted to sub-slopes).

**Implementation complexity:** MEDIUM. New module `astraweave-terrain/src/runevision_erosion.rs` with the filter algorithm. Compose into `TerrainNoise::sample_height` as a post-pass on the existing layered output.

**Per-vertex cost:** ~+5-10%. Filter is GPU-friendly per the blog (no per-vertex iteration required); CPU runs still serial-per-vertex but cheap.

**Veilweaver fit:** HIGH. Adds Blue-Ridge-style mesoscale gully/ridge structure that the current pipeline lacks.

**Recommendation:** F.4.B.3.C. May produce more visible improvement than Rank 1; consider re-ordering if Andrew-gate after Rank 1 is unimpressive.

### Rank 3: Multi-scale locality [Murray-implied; McKendrick-direct framing; Minecraft-wiki implementation reference]

**Concept:** Per-region parameter variation across the world, beyond the single-amplitude variation that AstraWeave's continental modulation provides. **[McKendrick-direct]** "Solar system → planet characteristics (mountain density, cliff frequency) → local terrain data."

**Best published implementation: Minecraft 1.18+ MultiNoise + spline system [Minecraft-wiki]:**

- 6-dimensional noise parameter space: `temperature`, `humidity`, `continentalness`, `erosion`, `weirdness` (ridges), `depth`.
- Each parameter is an independent low-frequency noise map.
- Biomes are defined as intervals in the 6D hypercube (NoiseHypercube records). Game picks closest biome.
- **Crucially:** the same noise maps drive biome selection AND terrain shape via spline functions. From the Minecraft wiki: "The density function contains a complex spline that functionally combines the continents, erosion, and ridges_folded density functions."
- Splines allow non-linear regionalized terrain response: high continentalness + low erosion + high weirdness = steep mountains; high erosion = flat regardless of continentalness.

**Biome blending: noiseposti.ng scattered convolution [Minecraft-wiki / noiseposti.ng]**

The canonical reference (https://noiseposti.ng/posts/2021-03-13-Fast-Biome-Blending-Without-Squareness.html) for biome blending without squared/axis-aligned artifacts:

1. Scatter biome data points on a jittered triangular/hexagonal lattice.
2. For each query coordinate, find nearby points within blending radius.
3. Weight: `w = max(0, radius² - dx² - dy²)²`.
4. Normalize: `total_weight = sum(all w); biome_blend = w_i / total_weight`.
5. Apply to any per-biome parameter: `height = sum(biome_i.GetHeight(p) * w_i / total_weight)`.

This eliminates square-grid axis-aligned banding. Applies to any per-biome parameter, not just height — frequency, amplitude, H, domain-warp strength can all be per-biome and blended.

**For AstraWeave's "world → regional → chunk" mapping (single-world, no solar system) [Inferred]:**

| Scale | Frequency (cycles/m) | Drives |
|---|---|---|
| Continent | 0.0001-0.001 | continental amplitude (already exists), sea-level offset, oceanic-vs-continental selection |
| Region | 0.001-0.01 | Hurst parameter (roughness), domain-warp strength, ridge offset (smooth-vs-ridged), runevision erosion strength |
| Biome | 0.01-0.1 | per-biome parameter blending (e.g., dense-forest-lowland vs exposed-ridge) |

**Implementation paths:**

- **Path A: Voronoi cells with per-cell parameter sets.** Hard transitions at cell boundaries; smoothable via cell-influence falloff.
- **Path B: Continuous parameter blend fields.** Multiple independent low-frequency noise fields each driving one parameter. AstraWeave's continental modulation is a 1-parameter Path B.
- **Path C: Layered biome regions with per-region preset definitions, blended at boundaries.** Closer to Minecraft's MultiNoise + spline approach.
- **Path D: Hybrid.** Use Path C for parameter selection (biome-like discrete regions), Path B for smooth blending of values within regions.

**Recommendation:** Path D, with the Minecraft-wiki + noiseposti.ng combination as the reference architecture. Specifically:

1. Define a small set of "region archetypes" (3-5 max) — each archetype is a parameter set (H, mountains_amplitude_scale, ridge_offset, runevision_strength). Examples: "Plain", "Hill Country", "Mountain Range", "Plateau", "Coastal".
2. Use 2-3 independent low-frequency noise fields (analogous to Minecraft's continentalness/erosion/weirdness) to determine which archetype dominates at each world position.
3. Blend per-vertex parameters via noiseposti.ng scattered convolution.
4. Each chunk's `TerrainNoise::sample_height` reads its blended parameters and evaluates locally.

**Locality vs F.4.A (climate-as-spatial-field) unification — KEY DECISION:**

F.4.A was originally planned to extend `ClimateMap` so that climate (temperature/moisture) varies per vertex and feeds `elevation_to_biome_weights` directly. The Minecraft-style architecture reveals this is the SAME problem as locality: climate IS one of the regional parameters. Implementing locality (F.4.B.3.D) and climate-as-spatial-field (F.4.A) as separate systems would duplicate the regional-parameter infrastructure.

**Recommendation: MERGE F.4.B.3.D with F.4.A.** F.4.A becomes a specialization of F.4.B.3.D — climate (temperature, moisture) is one of the regional parameters that varies; biome selection from climate is a downstream consumer. This avoids architectural duplication and produces a unified per-vertex-parameter system.

**Implementation complexity:** HIGH. This is the architectural keystone of F.4.B.3.
**Per-vertex cost:** medium (~+5-10% for parameter sampling; spline evaluation is fast).
**Veilweaver fit:** CRITICAL (this is the repetition fix).
**Recommendation:** F.4.B.3.D, absorbing the original F.4.A.

### Rank 4: Explicit ridge noise integration [Musgrave-canonical, dandrino-confirmed]

**Concept:** Fold-and-square noise to produce sharp linear ridges. `signal = offset - |noise|; signal *= signal`. **[Musgrave]** Standard form predates NMS by 15+ years.

**Critical Quilez caveat [Quilez]:** the `abs()` fold is non-differentiable at zero — analytical derivatives through ridge-folded noise are unavailable for that layer. AstraWeave's morenoise base layer would lose derivative access if directly fold-transformed. Musgrave's signal-feedback ridged multifractal works because the per-octave signal is consumed within the same octave loop without further differentiation.

**AstraWeave status:** the `noise` crate has `RidgedMulti` (Musgrave's full ridged multifractal with `H`, `lacunarity`, `attenuation`, `frequency`). Currently NOT used in any preset's primary mountain layer (the `mountains` layer is `NoiseType::RidgedNoise` per `noise_gen.rs`, but the editor presets use `NoiseType::Perlin` for the mountain layer construction in `apply_biome_noise_preset`). **Worth verifying this in implementation phase** — there may already be partial ridge integration.

**Implementation:**
- Replace or augment current mountain layer with `RidgedMulti` (offset 1.0, gain 2.0, H 1.0, lacunarity 2.0).
- For finer control, implement Musgrave's signal-feedback explicitly per `astraweave-terrain/src/noise_gen.rs` (~30 lines).
- Tune offset (0.8-1.2) and gain (1.0-3.0) per preset.

**Composition with Rank 1 (octave-emphasis):** Musgrave signal-feedback IS a form of octave emphasis (per-octave weight depends on previous-octave signal). If F.4.B.3.B adopts static weights, Rank 4 with Musgrave signal-feedback overrides those weights for the ridge layer specifically. Document the precedence.

**Composition with Rank 2 (runevision filter):** runevision adds gully structure ON TOP of any height function; ridged ridges as input feed the filter naturally. They compose orthogonally.

**Implementation complexity:** LOW (use existing `noise` crate `RidgedMulti`) to MEDIUM (implement Musgrave signal-feedback explicitly).
**Per-vertex cost:** zero net (replaces existing mountain layer).
**Veilweaver fit:** HIGH. Blue Ridge IS a ridge structure; AstraWeave's current noise produces round-topped peaks rather than linear ridge crests.
**Recommendation:** F.4.B.3.E. Conditional on locality (Rank 3) so per-region ridge-on/off decisions are possible.

### Rank 5: Altitude-based modulation [Secondary; possibly Musgrave-derived]

ithy.com names "altitude erosion" as Uber Noise feature; Murray's summary does NOT directly name it. Plausibly conflated with Musgrave's signal-feedback (which IS altitude-conditional via prev-signal feedforward). **[Secondary]**

If Rank 4 (ridge noise via Musgrave signal-feedback) is implemented, altitude modulation comes for free — `weight = clamp(prev_signal * gain, 0, 1)` already conditions next-octave amplitude on accumulated altitude.

If a separate altitude-modulation primitive is wanted (e.g., for non-ridge layers), the cleanest form is `amplitude *= smoothstep(low_altitude, high_altitude, current_height)` — vanishing detail in valleys, full detail at peaks.

**Implementation complexity:** LOW (if separate primitive needed) to ZERO (subsumed by Rank 4).
**Recommendation:** Evaluate after Rank 1-4 land. May not need as distinct work.

### Rank 6: Concavity / curvature modification [Not supported by primary sources]

**No primary source attributes this to Uber Noise.** ithy.com does NOT mention it (the prompt's concern was correct). Concavity in the procedural-noise literature appears in geomorphology / terrain-analysis contexts (drainage networks, valley curvature classification), not in Uber Noise.

Implementation would require Laplacian or second-derivative information not currently in `perlin_gradient.rs`. Could be approximated via finite difference of analytical gradient.

**Recommendation:** SKIP unless Andrew-gate post-Rank-1-through-4 still inadequate AND a specific use case for curvature emerges.

### Rank 7: Plateaus, terraces [Secondary; weak primary attribution]

ithy.com names these as Uber Noise features. **[Secondary]** Veilweaver-fit: LOW (canyon/mesa/agricultural features, not Appalachian-continental). Plateaus typically use `floor(height / step) * step + smooth(height % step)` quantization.

**Recommendation:** SKIP for F.4.B.3. Flag for future biome-specific work.

### Out-of-campaign: DEM-augmented hybrid generation [Inferred / Murray-rejected]

Murray's summary 24:46-25:05 explicitly rejects DEM data as "super boring" for NMS's stylized aesthetic. Veilweaver's Andrew-gate-driven aesthetic is "honest Appalachian" — closer to the geographic reality Murray rejected. This suggests an alternative path that NMS didn't take: **USGS DEM data for the Appalachian region as base layer, Uber Noise transforms applied on top for stylization and detail.**

USGS DEM data is freely available (https://www.usgs.gov/the-national-map-data-delivery) at 1/3 arc-second (~10 m) and 1 arc-second (~30 m) resolutions. A hybrid approach:

1. Sample USGS DEM at world position to get base elevation.
2. Apply F.4.B.3 transforms (octave-emphasis fBm detail, runevision gullies, regional parameter variation) on top.
3. Particle erosion as final pass.

Tradeoffs vs pure-procedural:
- **Authenticity:** real Appalachian topology — Mt. Mitchell, Black Mountains, Blue Ridge Escarpment are all there if you sample the right world coords.
- **Replayability:** lost. Each seed gives different procedural detail but same continental layout.
- **Control:** different — author chooses which real-world region to sample, vs procedural seed-driven generation.
- **Memory:** ~30 MB for 1-degree-square at 30m, manageable.

**Recommendation:** OUT OF SCOPE for F.4.B.3. Flag as alternative approach in §10 if Andrew-gate post-F.4.B.3 reveals procedural Uber Noise still feels insufficiently grounded.

## API shape proposal

The current `TerrainNoise::sample_height(x, z) -> f32` produces a single output. Uber Noise pipeline becomes a sequence of conditional transforms applied in order. Proposal:

```rust
fn sample_height_uber(world_x: f64, world_z: f64) -> f32 {
    // Stage 0: locality — sample regional parameters (F.4.B.3.D / F.4.A unified).
    // Returns archetype blend weights + interpolated parameters at this world position.
    let region_params = sample_regional_parameters(world_x, world_z);

    // Stage 1: domain-warped base coords (existing F.2.B).
    let (warped_x, warped_z) = domain_warp(world_x, world_z, region_params.warp_strength);

    // Stage 2: base layer — derivative-weighted fBm (existing F.2-T-4) with
    //         octave-emphasis tuning (F.4.B.3.B).
    let (base, gradient) = derivative_weighted_fbm_with_emphasis(
        warped_x, warped_z,
        region_params.base_amplitude,
        region_params.base_octaves,
        region_params.octave_emphasis_weights,  // F.4.B.3.B addition
    );

    // Stage 3: mountain layer — ridges (F.4.B.3.E) or rounded peaks based on region.
    let mountain = if region_params.use_ridges {
        ridged_multifractal(warped_x, warped_z, region_params.ridge_offset, region_params.ridge_gain)
    } else {
        emphasis_tuned_fbm(warped_x, warped_z, region_params.mountains_amplitude)
    };

    // Stage 4: continental modulation (existing F.2.6) — could be subsumed
    //         by region_params.continental_amplitude_scale.
    let continental_mult = continental_modulation(world_x, world_z);

    // Stage 5: combined raw output.
    let raw = (base + mountain * continental_mult);

    // Stage 6: runevision filter (F.4.B.3.C) — adds gully/ridge mesoscale
    //         detail via gradient. Layers on top.
    let with_gullies = runevision_erosion_filter(
        raw, gradient,
        region_params.runevision_strength,
        region_params.valley_altitude,
        region_params.peak_altitude,
    );

    with_gullies
}
```

This is a sketch. Refinement during F.4.B.3.B-E based on actual research findings + Andrew-gate measurements.

**Compatibility with F.3 particle erosion:** all Uber Noise transforms operate at noise-sample time, BEFORE `WorldGenerator::generate_chunk_with_climate` runs particle erosion via `apply_preset_at_world_offset`. F.3's pipeline reads heightmap values from the noise-side output as input. No interaction or conflict — pure composition.

## Calibration strategy

Each new transform requires calibration per Andrew-gate visual + measurement. F.4.B.3 sub-phases land transforms incrementally with measurement after each:

- Y span / amplitude impact (F.2 regression test thresholds may need re-recalibration).
- Surface curvature impact (F.2-T-2 spike-curvature test threshold may need adjustment).
- Visual character via Andrew-gate.

If any transform reveals it's worse than current state, revert that sub-phase. Each sub-phase is independently revertable per the campaign's discipline.

## Dependencies and ordering

- **Rank 1 (octave-emphasis)** is INDEPENDENT — can land first as cheapest test. Zero cost.
- **Rank 2 (runevision filter)** is INDEPENDENT — extends existing F.2-T-4 morenoise primitive (already provides gradients). Composes on top of any height function.
- **Rank 3 (locality)** is ARCHITECTURAL — once landed, subsequent transforms read region parameters. Locality is the natural infrastructure for ranks 4+.
- **Rank 4 (ridges)** can work standalone (single global ridge/no-ridge choice) but benefits from landing AFTER Rank 3 so per-region ridge selection is possible.
- **Rank 5 (altitude)** likely subsumed by Rank 4's Musgrave signal-feedback. Re-evaluate after Rank 4.

**Recommended order:**

1. **F.4.B.3.B: Octave-emphasis tuning** — cheapest test, immediate Andrew-gate decision point. If visible improvement → continue; if no improvement → don't iterate, move to Rank 2.
2. **F.4.B.3.C: runevision filter integration** — most algorithmically grounded transform with full published formulas. Likely highest visible impact for cartoon-shape problem. May overshadow Rank 1's effect.
3. **F.4.B.3.D: Multi-scale locality** — architectural keystone. Absorbs original F.4.A (climate-as-spatial-field). Highest complexity.
4. **F.4.B.3.E: Ridge noise integration with locality** — per-region ridge selection enabled by Rank 3.
5. **F.4.B.3.F (conditional):** altitude / concavity / additional transforms only if Andrew-gate post-E still inadequate.
6. **F.4.B.3.G: closeout** — F.2 regression test recalibration if needed, §9 + §10, conventions doc updates.

This order delivers visible Andrew-gate improvements early (B is zero-cost, C has full published algorithm) while building architectural foundation incrementally.

## Performance projection

Current generation at radius 10 with rayon (F.4.B.2.D): Temperate ~3 minutes (projected; not yet measured post-F.4.B.2.G).

Per-transform cost estimates [Inferred]:

| Transform | Per-vertex cost delta | Cumulative |
|---|---:|---:|
| Octave-emphasis (B) | 0% | 100% |
| runevision filter (C) | +5-10% | 105-110% |
| Locality (D) | +5-10% | 110-120% |
| Ridge noise (E) | 0% (replaces existing) | 110-120% |

**Cumulative projection:** Temperate at radius 10 with full Uber Noise stack = ~3.3-3.6 min. Well under 5-minute budget.

**Risk:** runevision filter is cell-based; if the cell evaluation cost is higher than estimated, could reach +15-20%. Measure after C lands.

## Locality vs F.4.A unification — KEY DECISION

**Recommend: MERGE F.4.A into F.4.B.3.D.**

Rationale:
- The Minecraft-style multi-parameter regional system reveals climate (temperature, moisture) is just one of several regional parameters that vary across the world.
- Implementing locality (F.4.B.3.D) and climate-as-spatial-field (F.4.A) as separate systems would duplicate the regional-parameter sampling/blending infrastructure.
- The unified system: regional parameters include `temperature`, `moisture`, `H`, `mountains_amplitude_scale`, `ridge_offset`, `runevision_strength`, `archetype_blend_weights`. Each is sampled at world position and blended via noiseposti.ng scattered convolution.
- F.4.A's downstream consumer (`elevation_to_biome_weights_with_sample`) becomes one of multiple consumers of the unified regional parameter sample.

**Impact on plan §9:**
- F.4.A (climate-as-spatial-field) merges into F.4.B.3.D. Plan §9 should be updated to reflect this when F.4.B.3.D lands.
- F.4.A's deliverables (per-vertex `ClimateSample`, `from_climate_field` factory, `"mixed"` primary biome option) are subsumed by F.4.B.3.D's general regional parameter system.
- F.5 closeout still works as planned — Phase 1 / 1.5 re-mark, editor UI wiring, integration tuning.

**Acknowledgment:** this is a scope expansion within F.4. F.4.B.3.D becomes more ambitious than originally framed. Time estimate: 1-2 sessions vs F.4.A's hypothetical 1 session — net zero or +1 session vs separate implementations.

## F.4.B.3 sub-phase plan (recommended)

| Sub-phase | Focus | Estimated sessions | Murray-attribution | Andrew-gate trigger |
|---|---|---:|:---:|---|
| F.4.B.3.A | Research (this) | 1 | — | — |
| F.4.B.3.B | Octave-emphasis tuning | 1 | Direct | After |
| F.4.B.3.C | runevision filter integration | 1-2 | Indirect (uses Murray primitive) | After |
| F.4.B.3.D | Multi-scale locality (absorbs F.4.A) | 1-2 | McKendrick-direct | After |
| F.4.B.3.E | Ridge noise integration with locality | 1 | Standard practice | After |
| F.4.B.3.F (conditional) | Altitude / concavity if needed | 0-1 | Secondary | After |
| F.4.B.3.G | Closeout (regression tests, §9, §10, conventions doc) | 1 | — | — |

Total estimated agent time: 5-7 sessions across the campaign. Comparable to F.3's lifecycle (4 phases) but each phase is more isolated.

**Andrew-gate decision points:**
- After F.4.B.3.B: cheapest test. If octave-emphasis visibly improves cartoon-shape problem → continue; if no improvement → don't iterate, move to C.
- After F.4.B.3.C: runevision filter is highest-likelihood visible impact transform. If gate passes → high confidence on remainder. If fails → diagnose (filter parameters? composition order?).
- After F.4.B.3.D: full locality-system test. The repetition problem should be substantially resolved.
- After F.4.B.3.E: full Uber Noise stack landed. Andrew evaluates comprehensive result. F.4.B.3.F is conditional on this gate.

## Out of scope for F.4.B.3 entirely

- DEM-augmented hybrid generation (flagged for future consideration if pure-procedural still feels insufficiently grounded).
- Voxel terrain / dual contouring / sparse virtual textures — Phase 1.7 territory.
- Phacelle directional noise (runevision predecessor) — already absorbed into the runevision filter.
- Atmospheric scattering / volumetric clouds — Phase 1.7 / separate aesthetic campaign.
- New biome / climate variants — F.5 integration tuning territory.
- Plateaus, terraces, mesa generation — biome-specific work for non-Appalachian aesthetics.
- Concavity / curvature modulation — not supported by primary sources for Uber Noise; defer.

## Sources cited

### Primary

- **Sean Murray, GDC 2017 "Building Worlds in No Man's Sky Using Math(s)"** via Gemini AI summary of YouTube video (https://www.youtube.com/watch?v=C9RyEiEzMiU, GDC Vault https://www.gdcvault.com/play/1024514/Building-Worlds-Using). Not a verbatim transcript. Accepted as primary-source proxy per F.4.B.3.A constraint 3.
- **Innes McKendrick, GDC 2017 "Continuous World Generation in 'No Man's Sky'"** via Gemini AI summary of YouTube video (https://www.youtube.com/watch?v=sCRzxEEcO2Y, GDC Vault https://www.gdcvault.com/play/1024265/Continuous-World-Generation-in-No). Saved separately as `docs/audits/nms_streaming_architecture_summary_2026-04-24.md` for Phase 1.7 reference.

### Quilez (canonical procedural-noise references)

- **fBm article** — https://iquilezles.org/articles/fbm/ — spectral theory, H/G relationship, terrain H=1 validation, octave detuning.
- **morenoise (derivative-weighted fBm, 2008)** — https://iquilezles.org/articles/morenoise/ — F.2-T-4's already-implemented base. Critical caveat: smooths slopes (suppresses high-frequency on steep terrain).
- **Domain warping** — https://iquilezles.org/articles/warp/ — multi-iteration warp formulas. Already implemented in F.2.B.

### Musgrave (canonical ridge / multifractal references)

- **Original ridged multifractal C source** — https://engineering.purdue.edu/~ebertd/texture/1stEdition/musgrave/musgrave.c (1994).
- **SharpNoise C# reference port** — https://github.com/rthome/SharpNoise/blob/master/SharpNoise/Modules/RidgedMulti.cs.
- **Isara documentation on ridged multifractal parameters** — https://docs.isaratech.com/ue4-plugins/noise-library/generators/ridged-multi.

### runevision (gradient-based erosion filter, primary reference for Rank 2)

- **Fast and Gorgeous Erosion Filter** — https://blog.runevision.com/2026/03/fast-and-gorgeous-erosion-filter.html (March 2026). MPL-2.0 licensed.
- **Phacelle directional noise** — https://blog.runevision.com/2026/01/phacelle-cheap-directional-noise.html (January 2026). Predecessor technique enabling the erosion filter.
- **Companion video** — https://www.youtube.com/watch?v=r4V21_uUK8Y.
- **80.lv coverage** — https://80.lv/articles/fast-terrain-erosion-filter-that-emulates-erosion-without-simulation.
- **Shadertoy implementations** — https://www.shadertoy.com/view/wXcfWn (advanced), https://www.shadertoy.com/view/33cXW8 (basic).

### Locality reference (Minecraft 1.18+ MultiNoise)

- **Minecraft world generation wiki** — https://minecraft.wiki/w/World_generation. 6D MultiNoise parameter space, spline terrain shaping.
- **Substack technical breakdown** — https://dawnosaur.substack.com/p/how-minecraft-generates-worlds-you.

### Biome blending (canonical method reference)

- **noiseposti.ng "Fast Biome Blending Without Squareness"** — https://noiseposti.ng/posts/2021-03-13-Fast-Biome-Blending-Without-Squareness.html (March 2021). Normalized sparse convolution on jittered hexagonal grid.

### Tertiary / supporting

- **dandrino terrain-erosion-3-ways** — https://github.com/dandrino/terrain-erosion-3-ways. Ridge noise formula `abs(fbm - 0.5)`. Particle simulation reference.
- **redblobgames "Making maps with noise"** — https://www.redblobgames.com/maps/terrain-from-noise/. Pedagogical overview.
- **NMS modding wiki — Terrain Generation** — https://nmsmodding.fandom.com/wiki/Terrain_Generation. **Returned 403 during research; field-level parameter taxonomy not retrievable.** Recommend NMSModBuilder GitHub for follow-up.
- **Step Modifications NMS wiki** — https://stepmodifications.org/wiki/NoMansSky:Tutorials/Terrain_Generation. **Returned 403.**
- **NMSModBuilder** — https://github.com/cmkushnir/NMSModBuilder. Recommended follow-up source for `TkNoiseUberLayerData` field names if specific NMS parameter taxonomy needed.
- **ithy.com "Innovations in Procedural Noise"** — https://ithy.com/article/innovations-procedural-noise-terrain-35cvalyh. **Treated with skepticism per F.4.B.3.A constraint 7.** Lists altitude erosion / slope erosion / ridge / plateau / terraces as Uber Noise features without primary citation. Slope-conditional cragginess is the only feature consistently corroborated against Murray's summary.
- **W-M function multifractal arXiv (January 2025)** — https://arxiv.org/abs/2501.02172.
- **World Creator noise reference** — https://docs.world-creator.com/reference/terrain/noises. 10 noise types; no slope/altitude/curvature-conditional variants.
- **Valheim wiki / Better Continents mod** — https://valheim.fandom.com/wiki/World_seed. Distance-radial + Perlin biome distribution (concentric model, not regional-parameter).
- **Red Blob Games procedural-map article** — https://www-cs-students.stanford.edu/~amitp/game-programming/polygon-map-generation/. Voronoi cell approach for biome regions.

### Internal AstraWeave references

- `docs/current/TERRAIN_GENERATION_QUALITY_CAMPAIGN.md` — campaign plan, §0 discipline, §9 phase status, §10 deviations.
- `docs/audits/terrain_noise_research_2026-04-22.md` — F.2-T-3's research pass establishing the current noise pipeline canon.
- `docs/audits/terrain_noise_audit_2026-04-22.md` — F.2-T-3's code audit.
- `docs/audits/terrain_seamless_erosion_research_2026-04-24.md` — F.3-phase-3.B's research-pass methodology template.
- `docs/audits/terrain_scale_diagnostic_2026-04-24.md` — F.4.B.1 diagnostic methodology template.
- `docs/audits/nms_streaming_architecture_summary_2026-04-24.md` — McKendrick summary saved separately for Phase 1.7.
- `docs/supplemental/WORLD_SCALE_CONVENTIONS.md` — 1 WU = 1 m convention.
- `astraweave-terrain/src/noise_gen.rs` — current `TerrainNoise` and `NoiseConfig`.
- `astraweave-terrain/src/perlin_gradient.rs` — F.2-T-4's analytical-gradient Perlin and derivative-weighted fBm.
- `astraweave-terrain/src/advanced_erosion.rs` — F.3 particle erosion. Composes after Uber Noise.

## Top-level recommendation

**Implement F.4.B.3.B → F.4.B.3.E in order, with explicit Andrew-gate after each.** Honest about provenance: the Murray-direct list is short (domain warp ✓ already done; slope-conditional cragginess; octave-emphasis). Don't over-claim Uber Noise feature parity; under-promise and let measurements drive iteration. Merge F.4.A into F.4.B.3.D's locality system. Defer altitude / concavity / DEM-hybrid to post-F.4.B.3 contingencies.

**The single most important takeaway:** the runevision filter (Rank 2) is the most algorithmically grounded transform with full published formulas, MPL-2.0 license, direct relevance to AstraWeave's gradient-rich pipeline, and likely highest visible impact for the cartoon-shape problem. F.4.B.3.C should not be considered second priority just because it's ranked second by Murray-attribution; it may be the highest-leverage transform in this campaign.
