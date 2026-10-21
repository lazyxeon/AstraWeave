# Terrain Generation — Established-Techniques Research (E3-terrain "real build" grounding)

> **Date**: 2026-07-01 · **Campaign**: R-series / E3-terrain · **Branch**: `campaign/roadmap`
> **Purpose**: Ground the E3-terrain "real build" in first-principles + established, proven techniques (not the fastest-to-wire path). Produced by the deep-research harness: 5 search angles → 20 sources fetched → 100 claims → 25 adversarially verified (21 confirmed / 4 refuted).
> **Trigger**: after many noise-tuning iterations plateaued at "same noise scaled up/down everywhere," the director asked for a first-principles + established-techniques analysis before choosing the terrain architecture.

---

## Verdict (what actually produces distinct biome-matched terrain)

The failure mode we hit — *"the same noise scaled up and down everywhere, so plains and mountains share the identical ridge pattern and color"* — is **structural**, not a tuning problem. A single height-noise stack cannot escape it. The established fix is a different architecture.

### 1. ROOT technique — Minecraft 1.18 multi-noise + splines *(verified 3-0)*
Distinct terrain comes from **separate low-frequency fractal noise fields — continentalness, erosion, peaks-and-valleys (PV) — mapped through hand-authored splines** into a height *offset* + a vertical *stretch factor*. Different regions of (continentalness, erosion, PV) space produce *categorically different* terrain shapes.
- Sources: Henrik Kniberg (Mojang) "Reinventing Minecraft world generation" ([video](https://www.youtube.com/watch?v=ob3VwY4JyzE)); [Minecraft Wiki: World generation](https://minecraft.wiki/w/World_generation); [Erosion](https://minecraft.wiki/w/Erosion).

### 2. The `erosion` parameter is the flat-vs-mountainous knob *(verified 3-0)*
> Kniberg: "high Continentalness, low Erosion and high Peaks & Valleys → massive steep-sided mountains … high erosion → very flat."
> Wiki: "the higher the erosion, the lower and flatter the terrain"; peak biomes generate only at "low erosion, high PV, high continentalness."
This same-parameter coupling is how terrain shape and biome stay **coherent** — both read the shared continentalness/erosion/PV fields.
- **Refuted nuance (0-3)**: the stronger claim that biomes are picked purely via temperature×humidity "keeping coherent" is *not* supported — coherence lives at the **continentalness/erosion/PV shape level**, so biome gating must read those fields, not temp/moisture alone.

### 3. Cheap organic layers — domain warping + derivative-modulated fBm *(verified 3-0)*
- **Derivative-modulated fBm** (Quílez "failed erosion"): accumulate analytic noise derivatives and divide each octave's amplitude by `(1 + dot(d,d))` → flattens already-steep areas, producing **both flat and rough regions from one function** — per-chunk, deterministic, no simulation. Powers Quílez's "Elevated." ([morenoise](https://iquilezles.org/articles/morenoise/), [function2009](https://iquilezles.org/articles/function2009/function2009.pdf))
- **Domain warping**: `f(p) → f(p + h(p))` de-grids feature boundaries, organic shapes. ([warp](https://iquilezles.org/articles/warp/))
- **fBm knobs**: Hurst exponent H (single roughness control), gain `G = 2^(-H)` for lacunarity 2. ([fbm](https://iquilezles.org/articles/fbm/))

### 4. Erosion *simulation* — realism, but offline *(verified 3-0)*
Hydraulic/fluvial simulation (shallow-water / virtual-pipes, Mei 2007; graph-fluvial, Cordonnier/arXiv:2210.14496) converts isotropic noise "bumps" into naturally-drained terrain (V-valleys, ridges, drainage). **But it's offline-to-batch**: ~7.5 s (256²) → ~115 s (1024²) per tile at 100 iterations; GPU per-iteration is interactive (~4–15 ms/512²–1024²) but needs many iterations, and global water-flow **breaks per-chunk determinism** (seam problem). → Best as an **offline/coarse bake**, not live per-chunk.
- **Refuted (0-3 / 1-2)**: "erosion sim is *what* makes terrain realistic" (overstated — splines/climate do the distinctness; erosion adds naturalness) and "Mei shallow-water runs at interactive rates on commodity HW" (not asserted by the seminal paper).

### 5. Per-biome terrain blending — legitimate *only as detail* *(verified 3-0)*
AutoBiomes (CGI 2020) blends per-biome DEM **detail over a shared base**, weighted by an area-convolution kernel with **noise-distorted borders**. It is a **patchy antipattern** when it swaps *whole-terrain profiles* at hard/straight boundaries. → My initially-proposed "each biome carries its own noise profile, blended" is the *wrong primary technique*; it's only safe as a detail layer.

### 6. Pipeline ordering (AutoBiomes) *(verified 3-0)*
`coarse noise base → climate sim (temp→wind→precip) → biome classification → biome-specific DEM detail refinement → asset placement`. Biome distribution derives **from** the climate/terrain field, not painted independently.

### Ranked believability-per-unit-effort (synthesis)
1. **Multi-noise + splines** — highest ROI, cheap, deterministic, per-chunk; the only layer that produces *distinct* biome-matched character. (Minecraft ships it at planet scale, real time.)
2. **Domain warp + derivative-modulated fBm** — cheap organic/erosion-like variety, per-chunk, deterministic.
3. **Full erosion simulation** — highest realism ceiling, but offline bake / heavy GPU; hardest to make per-chunk deterministic.

---

## How this maps onto AstraWeave (the key realization)

**The codebase already chose this architecture and mostly never implemented it.** From the `.2` archetype-differentiation recon:

| Established technique | AstraWeave state |
|---|---|
| Minecraft-1.18 continentalness/erosion/PV **splines** | **F.7 was designed as exactly this** — climate field carries continentalness/erosion/PV dims; `BootstrapParams` splines map them to 4 shape params. **But all splines are single-control-point (`d5fix`) → inert.** ← the ROOT gap |
| `erosion` = flat-vs-mountain driver | present as a climate dim; **not driving shape** (splines flat). *(Verify it's a real per-vertex noise field, not a constant — Phase A task 1.)* |
| Derivative-modulated fBm ("failed erosion") | **Exists** (`perlin_gradient.rs`, `base_derivative_weighted`) — **disabled by default**. |
| Domain warping | **Exists** (`DomainWarpConfig`, `NoiseType::DomainWarped`) — **off**. |
| Hydraulic/thermal/wind erosion **simulation** | **Exists** (`AdvancedErosionSimulator`) — de-tuned; correct role = offline/coarse bake. |
| Whittaker climate biomes | **Exists** (`biome_lookup.rs`) — needs erosion/continentalness **coherence gating** added. |

So "the real build" is largely **implementing what was architected + enabling disabled layers**, not inventing new architecture.

---

## Ratified build plan (E3-terrain "real build")

- **Phase A — the spline spine (ROOT fix):** verify erosion/PV are real per-vertex fields; author **multi-control-point splines** per archetype mapping continentalness/erosion/PV → shape params (erosion → flat-vs-mountain). Wire the None-mask path to the selected archetype's splines (the recon's 1-line hook + spline differentiation). *80% of the win.*
- **Phase B — cheap organic detail:** enable derivative-weighted fBm + domain warp (both already coded).
- **Phase C — biome coherence:** gate biome assignment on erosion/continentalness/PV (mountains only where low-erosion, etc.).
- **Phase D (later/optional):** hydraulic-erosion offline/coarse bake for drainage naturalness.

Goldens (E3.a-2) stay held; output changes throughout.

---

## Caveats / open questions (from verification)
- **Biome↔terrain coherence** is well-supported at the continentalness/erosion/PV *shape* level (peak biomes gated on low-erosion/high-PV/high-continentalness); the exact temperature/humidity-lookup "coherent by construction" guarantee was **refuted** — so gate biomes on the shape fields, don't assume temp/moisture suffices.
- **Per-chunk deterministic erosion** has no validated seam-free production algorithm in the verified corpus — reinforces "erosion sim = offline bake," and matches AstraWeave's existing halo-based approach for the analytic layers.
- Aeolian dune / stratified-mesa specific real-time methods were **not** surfaced — treat dunes/mesas as spline+warp+derivative-fBm approximations for now, not dedicated simulators.
- Source-quality: the GPU-erosion benchmarks are a non-peer-reviewed 2011 student paper on 2008 hardware (directional only); the ranked-ROI list is a synthesis (medium confidence).

**Sources (verified):** Minecraft Wiki (World generation, Erosion, Density function); Kniberg video; Alan Zucconi / dawnosaur breakdowns; Iñigo Quílez (warp, morenoise, fbm, function2009); Mei et al. 2007 (shallow-water GPU erosion); Jákó & Balázs 2011; arXiv:2210.14496 (graph-fluvial); AutoBiomes CGI 2020; Red Blob Games; NoisePosti.ng.
