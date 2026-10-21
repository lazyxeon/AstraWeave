# Terrain Noise Code Audit — 2026-04-22

**Scope:** Phase 1.6-F.2-T-3.B of the Terrain Generation Quality Campaign. Code audit of sampling resolution, DomainWarpedNoise internals, vertex-assembly, and mountain-layer behavior in highland regions. Paired with the research document (`terrain_noise_research_2026-04-22.md`) to provide concrete evidence for or against hypotheses from that document.

**Method:** static code review of `astraweave-terrain/src/noise_gen.rs`, `astraweave-terrain/src/lib.rs`, `astraweave-terrain/src/heightmap.rs`, and `tools/aw_editor/src/terrain_integration.rs::generate_heightmap_mesh`. Arithmetic based on post-F.2-T-2 grassland preset values.

---

## §2.A — Sampling resolution findings

### Code-derived constants

Source: `astraweave-terrain/src/lib.rs:133-134`:
```rust
chunk_size: 256.0,        // world units
heightmap_resolution: 64,  // vertices per chunk edge
```

And `heightmap.rs:281`:
```rust
let step = chunk_size / (self.resolution - 1) as f32;
```

### Derived values

| Quantity | Value |
|---|---:|
| Chunk world-unit size | **256 world units** |
| Vertices per chunk edge | **64** |
| Vertex spacing (step) | **256 / 63 ≈ 4.063 world units** |
| Nyquist minimum wavelength | **≈ 8.13 world units** (2× vertex spacing) |
| Community rule-of-thumb minimum | **≈ 16.25 world units** (4× vertex spacing per Red Blob Games / GameDev.net) |

### Per-layer highest-octave wavelength analysis (grassland preset post-F.2-T-2)

**Base elevation (DomainWarped Fbm):**
- `base_scale = 0.004`, `base_octaves = 5`, `base_lacunarity = 2.0`
- Base wavelength (octave 1): `1 / 0.004 = 250` world units
- Octave 5 wavelength: `250 / 2^4 = 15.625` world units
- **At 4-unit spacing: 3.85 samples per octave-5 period.**
- Status: **marginally above Nyquist** (2×) **but below the community 4× rule-of-thumb.**

**Mountains (RidgedMulti):**
- `mountains_scale = 0.0025`, `mountains_octaves = 6`, `mountains_lacunarity = 2.2` (from `NoiseConfig::default()`; `BiomeNoisePreset` does not override lacunarity)
- Base wavelength (octave 1): `1 / 0.0025 = 400` world units
- Octave 6 wavelength: `400 / 2.2^5 ≈ 7.77` world units
- **At 4-unit spacing: 1.91 samples per octave-6 period.**
- Status: **violates Nyquist (< 2 samples per period).**

**Detail (Billow):**
- `detail_scale = 0.02`, `detail_octaves = 3` (NoiseConfig default), `detail_lacunarity = 2.0` (default)
- Base wavelength (octave 1): `1 / 0.02 = 50` world units
- Octave 3 wavelength: `50 / 2^2 = 12.5` world units
- **At 4-unit spacing: 3.08 samples per octave-3 period.**
- Status: **above Nyquist, borderline on community rule-of-thumb.**

### Nyquist analysis verdict

| Layer | Octave-N wavelength | Samples/period | Nyquist safe? |
|---|---:|---:|---|
| base (DomainWarped, pre-warp) | 15.625 | 3.85 | **Marginal** (> 2, < 4) |
| mountain (RidgedMulti) | 7.77 | 1.91 | **VIOLATES** (< 2) |
| detail (Billow) | 12.5 | 3.08 | **Marginal** (> 2, < 4) |

**Mountain layer is formally Nyquist-violating at 6 octaves.** However, F.2-T-2.A measured mountain curvature at only 0.008 regardless of octave count 4–7. This apparent contradiction is explained by:
- Mountain is continental-modulated; in the F.2-T-2.A sampled region (cont_01 ≈ 0.23), mountain effective amplitude was reduced by ~50%.
- RidgedMulti uses multiplicative combination rather than summation; higher octaves' contribution is further damped by persistence (default 0.4).
- **Net effect:** mountain's Nyquist violation exists in principle but is masked by F.2's amplitude envelope and the F.2-T continental suppression. In pure highland regions with full amplitude, mountain's contribution could be materially higher than F.2-T-2.A measured.

### Domain-warp-induced effective bandwidth

This is the **critical finding** per the research doc.

Grassland DomainWarped parameters (post-F.2-T-2.B.3):
- `warp_strength = 15.0` world units
- `warp_scale = 1.5` (relative to base scale)
- `warp_octaves = 3`
- `iterations = 1`

Effective post-warp displacement magnitude (from `DomainWarpedNoise::get` at `noise_gen.rs:260-274`):
```
dx = warp_x.get(p) * warp_strength  // Fbm<Perlin> output is [-1, 1] → displacement in [-15, 15]
dz = warp_z.get(p) * warp_strength  // same
```
**Maximum coordinate displacement per vertex: ±15 world units in both X and Z** (total Euclidean displacement up to ~21.2 units).

Comparison to base-layer wavelengths:
- Octave 5 wavelength = 15.625 units. **Warp displacement (15) ≈ 96% of octave 5 wavelength.**
- Octave 4 wavelength = 31.25 units. Warp displacement ≈ 48% of octave 4 wavelength.
- Octave 3 wavelength = 62.5 units. Warp displacement ≈ 24% of octave 3 wavelength.

**Interpretation:** at octave 5, adjacent vertices (4 units apart) can have coordinate displacements differing by up to 15 units. 15 units is nearly a full octave-5 period. Adjacent vertices therefore **sample essentially uncorrelated points in the octave-5 noise field**, producing the coordinate-folding artifact named in the research doc (3DWorld blog, World Creator).

This is AstraWeave's smoking gun. The pre-warp layer is Nyquist-safe at 5 octaves. The post-warp effective bandwidth exceeds Nyquist because the warp's coordinate displacement brings adjacent samples into distant octave-5 territory. **Capping base_octaves to 4 removes the violating octave; octave 4 wavelength (31.25) is 2× warp displacement, so adjacent vertices sample correlated octave-4 regions.**

---

## §2.B — DomainWarpedNoise internals

Source: `astraweave-terrain/src/noise_gen.rs:218-275`.

### Structure

```rust
struct DomainWarpedNoise {
    primary: Fbm<Perlin>,      // from `noise` crate 0.9; full octave count (5 for grassland)
    warp_x: Fbm<Perlin>,       // warp_octaves=3, persistence=0.5 (hardcoded), lacunarity=2.0 (hardcoded)
    warp_z: Fbm<Perlin>,       // same as warp_x, seed offset +100 and +200 for decorrelation
    iterations: u32,
    warp_strength: f64,
}
```

### Octave configuration

- **Primary Fbm:** uses `layer.octaves` (5 for grassland post-F.2-T-2.B.3), `layer.persistence` (0.5), `layer.lacunarity` (2.0). These match the layer's configured values.
- **Warp fields (warp_x, warp_z):** separate Fbm instances. Octaves = `dw.warp_octaves` (3), persistence = **hardcoded 0.5** (not configurable), lacunarity = **hardcoded 2.0** (not configurable). Seeds: `seed + 100` and `seed + 200` to decorrelate the axes.

### Iteration semantics

`noise_gen.rs:260-274`:
```rust
fn get(&self, point: [f64; 3]) -> f64 {
    let mut x = point[0];
    let y = point[1];
    let mut z = point[2];
    for _ in 0..self.iterations {
        let dx = self.warp_x.get([x, y, z]) * self.warp_strength;
        let dz = self.warp_z.get([x, y, z]) * self.warp_strength;
        x += dx;
        z += dz;
    }
    self.primary.get([x, y, z])
}
```

Per iteration:
1. Sample `warp_x` and `warp_z` Fbm fields at current `(x, z)` — each produces a value in ~[-1, 1].
2. Multiply by `warp_strength` — scaled displacement in world units.
3. Accumulate displacement into `x` and `z`.
4. (Next iteration samples warp fields at the new `(x, z)` — iterated warp per Quilez §2.)

Final step: sample `primary.get([x, y, z])` at the fully-warped coordinates.

### Frequency-limiting check

**None.** The `noise` crate's `Fbm` iterates all configured octaves unconditionally (confirmed in research doc). `DomainWarpedNoise` does not perform any filter-width propagation, Jacobian tracking, or Nyquist capping. This is consistent with Quilez's bandlimiting article's warning that domain-warped noise invalidates simple distance-based filtering.

### Max displacement at current params (grassland, post-F.2-T-2.B.3)

- Single iteration: `warp_strength = 15.0`, warp field output ~[-1, 1] → single-iteration displacement in [-15, 15] per axis.
- Total after 1 iteration: **up to ±15 world units per axis** (Euclidean up to ~21.2 units).
- **This displacement is ~4× the vertex spacing (4 units)** — far larger than the safe budget.
- **This displacement is ~96% of base-octave-5 wavelength (15.6 units)** — producing the coordinate folding.

### Verdict

The custom `DomainWarpedNoise` implementation is simple and follows Quilez's textbook definition of iterative warping. No bugs. But it makes no accommodation for Nyquist, and the combination of `warp_strength=15` + `base_octaves=5` creates the post-warp aliasing pattern predicted by the research.

---

## §2.C — Vertex-assembly

Source: `tools/aw_editor/src/terrain_integration.rs:728-800` (`generate_heightmap_mesh`).

### Vertex position derivation

`terrain_integration.rs:751-754, 770-773`:
```rust
for z in 0..resolution {
    for x in 0..resolution {
        heights.push(heightmap.get_height(x as u32, z as u32));
        normals.push(Self::calculate_normal(heightmap, x, z, cell_size));
    }
}
// ...
let world_x = world_offset.x + x as f32 * cell_size;
let world_z = world_offset.z + z as f32 * cell_size;
```

**Direct assignment.** Heights come straight from `heightmap.get_height(x, z)` (= `sample_height` output via the SIMD generator). No smoothing, no interpolation, no post-processing. Vertices sit exactly where the noise function places them.

### Normal computation

`Self::calculate_normal(heightmap, x, z, cell_size)` — looked up; this is a finite-difference computation from neighboring height values (standard central-difference normal approximation).

**Relevant observation:** finite-difference normals amplify high-frequency height variation because they are equivalent to a high-pass filter (∂h/∂x). A surface that is smooth-looking in height can have spiky-looking normals if the heights oscillate at vertex scale. Conversely, the spikes visible in the editor are **both** height spikes AND normal spikes (because normals follow height gradient). This is not a bug but a property of the standard normal pipeline.

**Implication:** if F.2-T-3 reduces height curvature, normals will follow — vertex-scale shading artifacts from normal oscillation will also diminish.

### Interpolation / smoothing / LOD

None. `generate_heightmap_mesh` is a 1:1 emitter: every heightmap cell becomes a vertex; no pre-smoothing; no LOD tessellation; no per-vertex perturbation.

**Implication:** no opportunity for mesh-generation to mask the noise aliasing. The spike pattern is fully transmitted from noise field to vertex positions to visible surface.

### Verdict

Vertex assembly is a thin pass-through from noise to mesh. There are no code bugs here; the pipeline faithfully transmits whatever the noise field produces. The spikes are a noise-field-side problem.

---

## §2.D — Mountain layer in highlands

### F.2-T-2.A measurement limitation

F.2-T-2.A's `per_layer_spikiness` test sampled a 200×200 world-unit grid at origin (x,z) = (0..199, 0..199). The continental field at these coords is in the mid-range (cont_01 ≈ 0.4–0.5), so mountain_effective amplitude is ~50% of full.

If spikes are MOST prominent in highland regions per Andrew's observation, we'd want to measure mountain's contribution specifically in cont_01 > 0.8 regions.

### Code-based reasoning (no re-measurement conducted this session)

In highland regions:
- continental_01 ≈ 0.85 (per F.2-T.A diagnostic; peak measured was 0.874)
- multiplier = 0.5 + 0.5 × 0.85 = **0.925** — nearly full amplitude.
- Mountain raw amplitude: 80 units.
- Mountain effective amplitude in highlands: **~74 units** (vs F.2-T-2.A's sampled region at ~40).

Mountain layer at 6 octaves violates Nyquist (octave 6 at 7.77-unit wavelength vs 4-unit vertex spacing). With 74-unit effective amplitude in highlands and Nyquist-violating top octave, highland mountain curvature contribution could be **material** — potentially higher than F.2-T-2.A's 0.008 measurement would suggest.

However, two factors dampen this:
1. RidgedMulti's multiplicative combination structure limits higher-octave contribution more than additive Fbm does.
2. RidgedMulti's persistence default is 0.4 (not 0.5), making octave 6 amplitude only `80 × 0.4^5 ≈ 0.82` units — tiny.

**Verdict:** Mountain top-octave Nyquist violation exists but mountain persistence 0.4 limits its practical impact. **Mountain is a possible secondary spike source in highlands but unlikely to dominate.** Base layer (DomainWarped) remains the primary source per F.2-T-2.A + post-warp bandwidth analysis.

### Recommendation

Not a priority for F.2-T-3 intervention. If F.2-T-3.C.1's base_octaves cap doesn't visibly resolve highland spikes, a follow-up diagnostic measuring mountain curvature specifically in highland regions (and the mountain_octaves=5 alternative) is the next lever.

---

## Integration with Task 1 research

### Which audit findings match named phenomena from the research?

- **§2.A's base-layer borderline Nyquist status + §2.B's post-warp displacement 96% of octave-5 wavelength** directly matches the "domain-warp coordinate folding" phenomenon named in the research doc (3DWorld blog, World Creator docs) and Quilez's bandlimiting article's warning about filter-width propagation failure through warped domains.
- **§2.A's mountain layer Nyquist violation at octave 6** matches PBR's Nyquist-limit octave-capping remedy, but §2.D's analysis shows the violation is dampened by mountain's multiplicative structure and persistence=0.4.
- **§2.C's finite-difference normal pipeline** matches research doc's observation that unprocessed noise terrain is expected to look spiky; no independent normal bug exists.

### Which audit findings are novel to AstraWeave?

- **The post-warp bandwidth expansion at warp_strength=15 + base_octaves=5 is an AstraWeave-specific configuration.** The research doc confirms the general phenomenon (3DWorld blog) but AstraWeave's specific warp_strength/base_octaves combination was not measured in any cited source. Our F.2-T-2.A 2373× curvature amplification is an AstraWeave-specific datum.
- **AstraWeave's `DomainWarpedNoise` is custom code** (confirmed by research doc's analysis of the noise-rs crate). Bugs in the implementation would be AstraWeave-specific; §2.B confirms there are none — the implementation matches Quilez's textbook iterative-warp definition.

### Do the findings collectively point to a specific remedy?

**Yes — two interventions are supported by both research and audit evidence:**

1. **Cap base_octaves from 5 to 4** on the five DomainWarped presets. Justification: §2.B's 96% displacement/wavelength ratio at octave 5 predicts the coordinate folding that produces the observed spikes; dropping octave 5 eliminates the regime where warp displacement nearly equals wavelength. PBR formula (research doc Rank 2) supports the intervention; F.2-T-2.A's exploratory matrix confirms a modest (6%) curvature reduction from octaves 5 → 4. **APPLIED as F.2-T-3.C.1.**

2. **Accept F.3 erosion as the canonical remedy for residual character.** Justification: research doc Rank 3 — Musgrave 1989, Quilez morenoise, dandrino, and multiple AAA pipelines all explicitly pair raw noise with erosion. Residual spike character after Nyquist cap is the expected behavior of un-eroded noise; expecting spike-free raw output is a category error. **ENDORSED in F.2-T-3.D closeout as plan-level position.**

### Findings that do NOT point to an F.2-T-3 intervention

- **Derivative-weighted fBm (research doc Rank 1)** is the most principled literature-backed fix but requires structural code changes (custom Fbm with gradient accumulation) that exceed tuning-pass scope. **Deferred as potential F.2-T-4 if Nyquist cap + F.3 erosion combined are insufficient.**
- **Bilateral/Gaussian post-smoothing (research doc remedy 3)** would work but adds a pipeline stage and is inferior to erosion for AAA-quality terrain. **Not recommended**; if post-processing is wanted, it should be erosion (F.3), not isotropic smoothing.
- **Simplex substitution** is explicitly not a remedy for Nyquist violation.

---

## Audit verdict

AstraWeave's terrain generation pipeline is faithfully implementing a spec-correct version of domain-warped multi-octave fBm with continental modulation. The bed-of-nails surface character in the grassland preset is caused by two interacting factors:

1. **Post-warp effective bandwidth expansion** (§2.B) — the primary mechanism. Warp displacement (~15 units) is comparable to base layer's octave-5 wavelength (~15.6 units), producing coordinate folding where adjacent vertices sample uncorrelated octave-5 regions.

2. **Marginal Nyquist status of the pre-warp signal** (§2.A) — base_octaves=5 is borderline per PBR formula even before warping; the post-warp effect compounds it.

The low-effort literature-justified intervention is **capping base_octaves at 4** (F.2-T-3.C.1) — removes the violating octave. The high-impact literature-preferred intervention is **derivative-weighted fBm** but is structural and deferred. The architecturally correct continuation is **F.3 erosion** as the canonical pairing for raw noise; literature is unambiguous that raw noise terrain is expected to look wrong before erosion.

**Recommended F.2-T-3 outcome:**
- Apply base_octaves=4 cap as a low-risk Nyquist-targeted fix.
- Document derivative-weighted fBm as potential F.2-T-4.
- Endorse F.3 erosion as the canonical resolver for residual character.
- Mark F.2 complete with explicit "F.3 is canonical solver" annotation once F.2-T-3.C.1 lands.

---

**End of audit document.**
