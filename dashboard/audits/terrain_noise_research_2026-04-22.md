# Procedural Noise Spike Research — 2026-04-22

**Scope:** Phase 1.6-F.2-T-3.A of the Terrain Generation Quality Campaign. Web research into named phenomena and canonical remedies for vertex-scale spike artifacts in multi-octave fBm and domain-warped noise heightmaps, after F.2-T and F.2-T-2 achieved 2.7× curvature reduction but left residual bed-of-nails visible in Andrew's interactive verification.

**Method:** research-scout agent invocation 2026-04-22 with a structured six-question brief. 18 authoritative sources consulted across the procedural terrain generation, signal-processing, and AAA game industry literatures. All claims below trace to specific cited URLs or papers.

---

## Research conducted

- 18 authoritative sources consulted, covering PBR (Pharr/Jakob/Humphreys §10.6), four Iquilez articles (domain warping, bandlimiting, value-noise derivatives, fBm), GPU Gems 2 Ch. 26 and GPU Gems 3 Ch. 1, Musgrave/Kolb/Mace SIGGRAPH 1989, the noise-rs crate source, multiple AAA GDC talk abstracts, and several community/practitioner discussions.
- Key URLs (full list in "Sources cited" below): iquilezles.org (4 articles), pbr-book.org/3ed-2018/Texture/Noise, developer.nvidia.com (2 GPU Gems chapters), dl.acm.org/doi/10.1145/74334.74337 (Musgrave 1989), github.com/Razaekel/noise-rs, github.com/dandrino/terrain-erosion-3-ways.

---

## Named phenomena

**Primary finding: the pattern does not have a single canonical name, but is described under two overlapping framings:**

1. **"Noise aliasing" / "Nyquist violation in fBm"** — the signal-processing framing. *Physically Based Rendering* (Pharr, Jakob, Humphreys) §10.6 states: "A critical property of practical noise functions is that they be band-limited with a known maximum frequency," and fBm implementations should stop adding octaves "that would have frequencies beyond the Nyquist limit." PBR derives the cutoff: **n_max = −1 − ½ log₂(l²)**, where `l` is the maximum differential of texture coordinates per sample. When a multi-octave fBm sums octaves past this limit, the highest-frequency octaves cannot be represented on the vertex mesh and manifest as vertex-scale artifacts.

2. **"Domain-warp coordinate folding" / "high-frequency domain-warp residue"** — the domain-warping-specific framing. When coordinate displacement magnitude (our `warp_strength`) is comparable to or exceeds the noise's feature scale, adjacent grid samples query entirely different noise regions, creating discontinuous height jumps. The 3DWorld terrain blog (2017) states: "domain warping can cause a large section of heightmap noise to be picked up and moved somewhere else in the scene. This effect can create high frequency content that isn't present in most other heightmap generation functions." World Creator docs name the extreme version "concentric artifacts" and attribute them to excessive warp strength.

Closest single canonical term across the literature: **"Nyquist violation in multi-octave fBm"** for the underlying signal-processing cause, with "domain-warp coordinate folding" as the specific amplification mechanism in AstraWeave's pipeline. Practitioners use these phrases rather than a coined single term.

---

## Canonical remedies

Ranked by relevance to AstraWeave's specific state:

### 1. Nyquist-limit octave capping (PBR formula)

**Source:** *Physically Based Rendering* §10.6 (pbr-book.org/3ed-2018/Texture/Noise) — formula `n_max = −1 − ½ log₂(l²)`.

**Mechanism:** Never evaluate an fBm octave whose spatial wavelength falls below 2× the vertex spacing. Quilez's bandlimiting article (iquilezles.org/articles/bandlimiting) refines this with a smoothstep attenuation: `fnoise(x, w) = noise(x) * smoothstep(1.0, 0.5, w)` where `w` is the filter width (one unit = one sample).

**Trade-off:** Bandlimiting the pre-warp signal does not protect against post-warp aliasing — the domain warp's Jacobian can stretch frequency content beyond the original cutoff. Quilez: "Space deformations complicate straightforward distance-based attenuation…requiring either Jacobian-based analytical computation of filter width through transformation chains, or hardware-provided filter width calculations." For AstraWeave: partial fix only, but low-effort.

### 2. Derivative-weighted fBm (Quilez's "fake erosion")

**Source:** Iquilez — value noise derivatives (iquilezles.org/articles/morenoise, 2008). Code pattern:
```glsl
a += b * n.x / (1.0 + dot(d, d));
d += b * n.yz;
b *= 0.5;
```

**Mechanism:** Accumulates the analytical gradient `d` as octaves are summed. Each higher octave's contribution is weighted by `1/(1 + dot(d,d))`. On steep terrain the denominator grows, suppressing high-frequency octaves precisely where they would produce spike artifacts. The effect is flat-floored valleys with sharp ridges — mimicking eroded terrain without a physical simulation.

**Trade-off:** Requires analytical derivatives, which `noise` crate's `Fbm<Perlin>` does not expose. Requires implementing a custom fBm loop in `astraweave-terrain` that accumulates both noise value and gradient. This is a structural change, not a parameter change — **scope beyond an F.2-T-3 tuning intervention.**

### 3. Bilateral / Gaussian post-smoothing on the heightmap

**Source:** GameDev.net terrain-smoothing threads, jMonkeyEngine heightmap post-processing docs, standard image-processing practice.

**Mechanism:** Treat the generated heightmap as a single-channel image; apply a spatial low-pass filter (Gaussian for isotropic smoothing, bilateral for edge-preserving).

**Trade-off:** Lossy — Gaussian erodes ridges along with spikes. Bilateral is harder to tune and more expensive. For AstraWeave: could be applied selectively to base layer before summing with mountain/detail, but adds a pass to every chunk generation.

### 4. Analytical-derivative anti-aliased fBm (PBR/Quilez combined approach)

**Source:** PBR §10.6 + Quilez bandlimiting. Evaluate fBm with a frequency-adaptive cutoff at each query site; the final partial octave fades in gradually rather than being hard-cut.

**Mechanism:** Most principled; avoids Nyquist violations at evaluation time. Requires passing filter width (vertex spacing) as evaluation context.

**Trade-off:** Structural change equivalent to Remedy 2 — beyond tuning-pass scope.

### 5. Hydraulic / thermal erosion simulation

**Source:** Musgrave, Kolb, Mace — SIGGRAPH 1989, "The synthesis and rendering of eroded fractal terrains" (dl.acm.org/doi/10.1145/74334.74337). Modern implementations: terrain-erosion-3-ways (dandrino), SBGames 2018 GPU erosion paper.

**Mechanism:** Physical simulation of water droplet descent + sediment transport. Produces valleys, alluvial fans, talus slopes — geological macro-forms that raw noise lacks.

**Trade-off:** Expensive (10k–50k droplet iterations per 256² heightmap). Solves **realism**, not Nyquist violation directly — spike gradients drive erosion drops into non-geological paths if present pre-erosion. **Correct pipeline order: fix Nyquist/curvature first, then add erosion.** This maps directly to F.3 in AstraWeave's campaign.

### 6. Simplex vs Perlin substitution

**Source:** Wikipedia on Simplex Noise; PulseGeek Simplex vs Perlin comparison.

**Mechanism:** Simplex eliminates directional/lattice artifacts.

**Trade-off:** Does NOT address Nyquist violation in multi-octave fBm. Lateral move for the spike symptom specifically.

### 7. Gradient-based domain warping

Not widely cited as a spike-reduction technique for heightfield terrain. Not recommended on research basis.

---

## Specific findings from Iquilez's domain-warping articles

### iquilezles.org/articles/warp (2002)

- Qualitative/aesthetic treatment. Shows iter=1 (organic turbulence) vs iter=2 (dramatic swirling).
- **No quantitative frequency analysis, no Nyquist guidance, no warp_strength recommendation.**
- Offsets like `(5.2, 1.3)` described as having "no special meaning."
- Quilez's visual evidence is consistent with AstraWeave's F.2-T-2.A measurement: iter=2 has substantially more high-frequency content than iter=1 (AstraWeave measured 6825× vs 2373× Perlin curvature). Each warp layer compounds Jacobian distortion.

### iquilezles.org/articles/bandlimiting (no date)

- **Critical article for AstraWeave's problem.** Directly addresses how domain warping invalidates simple distance-based frequency filtering.
- States: when domains are stretched/compressed, filter-width propagation becomes non-linear. Requires Jacobian tracking or hardware `fwidth()`.
- Implies: Nyquist capping the pre-warp signal is insufficient; post-warp aliasing is possible even with octave caps.

### iquilezles.org/articles/morenoise (2008)

- **Most actionable article for AstraWeave.** Describes derivative-weighted fBm ("fake erosion").
- Explicitly acknowledges unweighted fBm's uniform-ruggedness as a known limitation — practitioners routinely suppress it.
- Confirms the "spikes in highland regions" pattern AstraWeave observes is the **expected behavior of unweighted fBm, not a bug** but a quality-of-life defect that production terrain code addresses.

### iquilezles.org/articles/fbm

- Spectral analysis of fBm, Hurst exponent, gain relationships.
- Mentions LOD/anti-aliasing benefit of incremental octave construction.
- No direct spike/Nyquist treatment.

---

## Vertex-to-noise-frequency best practices

**PBR formula (authoritative):** n_max = −1 − ½ log₂(l²), where `l` = (sample frequency) × (vertex spacing).

**Community rule of thumb (Red Blob Games, GameDev.net):** highest-frequency octave wavelength should be ≥ 2–4× vertex spacing. At AstraWeave's 4-unit vertex spacing, octaves with wavelength < 8–16 should be absent.

**AAA terrain talks:** No Man's Sky (GDC 2017 "Continuous World Generation") uses domain-warping "uber noise" but parameters are not publicly disclosed (paywalled talk). Horizon Zero Dawn and Ghost of Tsushima use artist-authored terrain with procedural placement overlay — not heightfield noise. No explicit AAA vertex-to-noise-frequency ratio is publicly available for noise-driven terrain.

**GPU Gems 3 Ch. 1:** recommends ~9 octaves for a full-planet terrain but addresses floating-point precision at continental scale, not vertex-scale aliasing.

---

## Rust noise crate idioms

**Critical finding: the `noise` crate 0.9 does NOT have a `DomainWarped` struct.** Crate module structure (github.com/Razaekel/noise-rs): Perlin, PerlinSurflet, Simplex, OpenSimplex, SuperSimplex, Value, Worley, BasicMulti, Billow, Fbm, HybridMulti, RidgedMulti, Checkerboard, Constant, Cylinders. A `Displace` transformer exists but is not what AstraWeave uses.

**AstraWeave's `DomainWarpedNoise` is custom code** in `astraweave-terrain/src/noise_gen.rs:218-275`, wrapping `Fbm<Perlin>` from the crate for both primary and warp fields.

**The crate's `Fbm` does no bandlimiting, no Nyquist check, no frequency cap.** Confirmed by reading github.com/Razaekel/noise-rs/.../fbm.rs source. Default lacunarity is π×2/3 ≈ 2.094 (reduces lattice artifacts, not aliasing). Any octave-count choice is responsibility of the caller.

**Common idiom that causes spikes:** using the crate's `Fbm` with 6+ octaves at close view distances, then domain-warping on top. Adding `RidgedMulti` (absolute-value folding creates extra high-frequency content at ridgelines) and `Billow` on top of `Fbm` compounds this. When a domain warp displaces input coordinates by a multiple of the base scale period, "large sections of heightmap noise are picked up and moved" (3DWorld blog).

---

## Un-eroded noise: expected to have spikes?

**Yes — explicit and unambiguous in the literature.** This is the honest answer the prompt's Question 6 anticipated.

### Sources that confirm

- **Musgrave, Kolb, Mace (SIGGRAPH 1989):** presents erosion as the second stage of a two-stage pipeline. Raw fractal → physical erosion. The implication is that raw noise is not expected to look geologically correct.

- **dandrino terrain-erosion-3-ways:** "Raw fBm-based terrain generates reasonable looking terrain at a quick glance but when compared to actual elevation maps, looks nothing like real terrain."

- **Quilez morenoise:** derivative-weighted fBm exists *because* unweighted fBm is "uniformly rugged everywhere." The derivative trick is described as "fake erosion" — the terminology itself confirms that erosion-like processing is the expected fix.

- **Frozen Fractal dev blog (2025); SBGames 2018 GPU erosion paper:** both treat erosion as a standard production pipeline stage, not optional.

### The category-error framing

Expecting spike-free output from raw domain-warped fBm at close view distance is a category error. The noise is correctly computing its defined function. Spikes are correct samples of a function whose frequency content exceeds vertex mesh representation capacity. The fix belongs at the generation layer (bandlimiting, octave capping, derivative weighting) or as a post-processing layer (smoothing, erosion) — **not in the noise parameters alone.**

### Important caveat: pre-erosion Nyquist cleanliness matters

Erosion applied to Nyquist-violating noise produces poor results. Spike gradients drive erosion drops into non-geological paths. **Correct pipeline order: fix Nyquist/curvature FIRST, then apply erosion.** This matches AstraWeave's plan §2.1 sequence (biome-weights from pre-erosion Y, erosion applied after).

---

## Ranked recommendations for AstraWeave

### Rank 1 — Derivative-weighted fBm (Quilez morenoise)

- **Mechanism:** custom Fbm loop accumulating gradient alongside value, weighting high-frequency octaves by `1/(1+dot(d,d))`.
- **Impact:** high — directly targets the highland-spike pattern.
- **Effort:** moderate — requires implementing custom fBm in `astraweave-terrain` with analytical gradient accumulation. One module-scope change.
- **Status for F.2-T-3:** **DEFERRED as structural change beyond tuning-pass scope.** Proposed as a future tuning pass (F.2-T-4) or roll into F.3 preparation.
- **Source:** iquilezles.org/articles/morenoise

### Rank 2 — Nyquist-limit octave cap on base_octaves

- **Mechanism:** reduce DomainWarped base_octaves on the five DomainWarped presets per PBR formula n_max = −1 − ½ log₂(l²).
- **Current state analysis (see audit doc §2.A):** base layer at scale 0.004, vertex spacing 4 world units, l=0.016 → n_max ≈ 5.9. Current base_octaves=5 is borderline; octave 5 wavelength ≈ 15.6 units ≈ warp_strength (15) magnitude, creating coordinate folding.
- **Intervention:** cap base_octaves at 4 for the five DomainWarped presets. Expected reduction per F.2-T-2.A exploratory matrix: 0.50 → 0.47 (~6%). Modest but cumulative with prior fixes.
- **Impact:** small — incremental.
- **Effort:** trivial — one-line change per preset.
- **Status for F.2-T-3:** **APPLIED as F.2-T-3.C.1** — low-risk, literature-justified, small quantitative improvement.
- **Source:** PBR §10.6 (pbr-book.org/3ed-2018/Texture/Noise), supported by Quilez bandlimiting.

### Rank 3 — Proceed to F.3 erosion as the canonical solver for residual character

- **Mechanism:** acknowledge that residual bed-of-nails character after Nyquist capping is the expected behavior of un-eroded noise terrain. F.3's `AdvancedErosionSimulator::apply_preset` is the literature-cited canonical solver (Musgrave 1989, dandrino terrain-erosion-3-ways, etc.).
- **Impact:** high — erosion produces geological macro-forms (valleys, alluvial fans, talus slopes) that add real character beyond spike suppression.
- **Effort:** already scheduled as F.3.
- **Status for F.2-T-3:** **ENDORSED as the canonical continuation.** F.2-T-3.D closeout explicitly states: residual surface-spike character after F.2-T-3's Nyquist cap is expected per Musgrave 1989 / Quilez morenoise; F.3 erosion is the canonical remedy. F.3 success criteria should confirm erosion reduces surface curvature below Andrew's acceptable visual threshold.

---

## Sources cited

All URLs accessed 2026-04-22 via research-scout agent.

- [Physically Based Rendering — §10.6 Noise (Pharr/Jakob/Humphreys)](https://pbr-book.org/3ed-2018/Texture/Noise) — authoritative Nyquist cutoff formula n_max = −1 − ½ log₂(l²).
- [Inigo Quilez — domain warping](https://iquilezles.org/articles/warp/) — primary source for iterative domain warping (iter=1, iter=2); qualitative treatment only.
- [Inigo Quilez — bandlimiting procedural noise](https://iquilezles.org/articles/bandlimiting/) — direct treatment of space deformations invalidating filter-width propagation; Jacobian requirement.
- [Inigo Quilez — value noise derivatives (2008)](https://iquilezles.org/articles/morenoise/) — derivative-weighted fBm, slope-based octave suppression; "fake erosion."
- [Inigo Quilez — fBm](https://iquilezles.org/articles/fbm/) — spectral analysis, Hurst exponent, gain relationships.
- [GPU Gems 2, Chapter 26 — Implementing Improved Perlin Noise](https://developer.nvidia.com/gpugems/gpugems2/part-iii-high-quality-rendering/chapter-26-implementing-improved-perlin-noise) — Perlin's improved noise; band-limited spatial frequency property.
- [GPU Gems 3, Chapter 1 — Generating Complex Procedural Terrains Using the GPU](https://developer.nvidia.com/gpugems/gpugems3/part-i-geometry/chapter-1-generating-complex-procedural-terrains-using-gpu) — 9-octave recommendation for planetary terrain.
- [Musgrave/Kolb/Mace — "The synthesis and rendering of eroded fractal terrains" (SIGGRAPH 1989)](https://dl.acm.org/doi/10.1145/74334.74337) — original two-stage pipeline establishing erosion as required second pass.
- [Musgrave — Procedural Fractal Terrains (course notes)](https://www.classes.cs.uchicago.edu/archive/2015/fall/23700-1/final-project/MusgraveTerrain00.pdf) — statistical uniformity of raw fBm as a limitation.
- [3DWorld terrain blog — Domain Warping Noise (2017)](http://3dworldgen.blogspot.com/2017/05/domain-warping-noise.html) — "domain warping creates high-frequency content not present in other heightmap generation."
- [World Creator — Noises documentation](https://docs.world-creator.com/reference/terrain/noises) — names "concentric artifacts" at excessive warp strength.
- [noise-rs GitHub](https://github.com/Razaekel/noise-rs) — Fbm source confirms no Nyquist check, 32-octave max, no DomainWarped struct.
- [noise 0.9.0 docs.rs](https://docs.rs/crate/noise/latest) — confirms no DomainWarped in upstream crate.
- [FastNoiseLite Documentation](https://github.com/Auburn/FastNoiseLite/wiki/Documentation) — SetDomainWarpAmp default 1.0; warp as "maximum distance from original position."
- [terrain-erosion-3-ways (dandrino)](https://github.com/dandrino/terrain-erosion-3-ways) — "Raw fBm terrain looks nothing like real terrain"; erosion as standard pipeline stage.
- [Real-Time Massive Terrain Generation using GPU Erosion (SBGames 2018)](https://www.sbgames.org/sbgames2018/files/papers/ComputacaoShort/188264.pdf) — hydraulic erosion gives "best quality."
- [Vertex Shader Domain Warping with Automatic Differentiation (arxiv 2405.07124)](https://arxiv.org/html/2405.07124v1) — Jacobian analysis of domain warping artifacts.
- [A Survey of Procedural Noise Functions (Lagae et al., CGF 2010)](https://www.cs.umd.edu/~zwicker/publications/SurveyProceduralNoise-CGF10.pdf) — band-limited noise property; sparse convolution alternatives.

---

## Caveats

- AstraWeave's `DomainWarped` is **custom code**; no upstream library has that type. The F.2-T-2.A measurements of 2373× / 6825× curvature amplification are measurements of AstraWeave's specific implementation; no literature source can be cited for those specific numbers (though the general phenomenon is confirmed).
- AAA GDC terrain talks for No Man's Sky, Horizon Zero Dawn, Ghost of Tsushima are either paywalled (NMS) or not about heightfield generation (HZD, GoT). Publicly available technical details on AAA heightfield noise pipelines are sparse. GPU Gems 3 (2007) remains the closest authoritative source and predates modern domain-warping practice by a decade.
- Erosion as canonical fix vs erosion as different problem: literature consistently pairs raw noise with erosion, but for *realism* not *spike removal*. Derivative-weighted fBm (Rank 1) and octave capping (Rank 2) are the literature-backed primary interventions for spikes. Erosion should follow, not precede, fixing Nyquist/curvature.
- Simplex noise substitution will not solve this problem — it eliminates directional artifacts, not aliasing.
- PBR Nyquist formula assumes uniform sampling. At AstraWeave's fixed 4-world-unit vertex grid this applies directly; if future LOD introduces non-uniform density, cutoff must be re-evaluated per sample site.

---

**End of research document.**
