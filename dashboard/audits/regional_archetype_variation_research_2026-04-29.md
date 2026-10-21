# Regional Archetype Variation Research Audit

**Date**: 2026-04-29
**Mode**: Investigation-only (research pass; no production code; precondition for new-campaign launch).
**Trigger**: F.4.B.3.D.5-diagnostic-3 (commit `319f74a13`, audit `docs/audits/f4b3d5_diagnostic_3_cross_archetype_2026-04-28.md`) measured cross-archetype Pearson 0.978-0.989 across measured pairs and 1-of-18 archetype-aware bootstrap parameters. Andrew's 2026-04-29 chat clarified the actual target: regional archetype variation with smooth blending, ~5-10 archetypes per world, one archetype per Tikva storyline region. This is architecturally larger than diagnostic-3's Path 1 sketch, which framed per-world archetype selection. The new target is "biome of biomes" — multiple archetypes coexist within a single world, organized regionally, with smooth transitions at flyover scale.
**Scope**: Survey shipping AAA prior art (Crimson Desert primary; Enshrouded concrete-implementation; Minecraft 1.18+ / NMS / Skyrim / Witcher 3 / Horizon supplementary) plus algorithmic literature, then produce an AstraWeave-specific architectural recommendation. The recommendation is the load-bearing artifact for the campaign-design session that follows.

---

## 1. Background

### 1.1 The architectural gap diagnostic-3 measured

Diagnostic-3 established that F.4.B.3.D's 6 archetypes produce visually identical worlds because:

- **Bootstrap dominates terrain shape (72-81% of variance)**: mountain bulk amplitude, scale, octaves, noise type, continental scale, base elevation — all are constants from `NoiseConfig::default()` and do not vary by archetype.
- **1 of 18 bootstrap parameters is archetype-aware** (the per-vertex `mountain_amplitude_multiplier`).
- **Per-archetype amplitude distributions cluster narrowly** (means 1.097-1.379, 60-85% of samples in `[1.0, 1.5)` band).

The architecture's design boundary chose to make biomes drive per-biome shape character via amplitude (D.3a's decision), not per-archetype shape character via bootstrap parameters.

### 1.2 The clarified Veilweaver target

Andrew's 2026-04-29 chat clarified the actual product requirement:

- **5-10 archetypes per world, organized regionally.** Not per-world archetype selection; per-region.
- **One archetype per Tikva storyline region.** Authoring intent maps one storyline (a writer-controlled lore unit) to one archetype zone in the playable world.
- **Smooth blending between adjacent archetype zones.** The transition between Continental Temperate and Mediterranean reads as natural geography at flyover scale, not as an axis-aligned biome boundary.
- **Atmosphere-globe-view test (Crimson Desert reference).** From high altitude the player sees the entire world's biome variation in a single frame, with regions visually distinct yet seamlessly composed.

### 1.3 Why this is its own campaign

Closing F.4.B.3.D as "PARTIAL with a docked tail" would attempt to address regional variation inside a campaign that drafted from biome-classification first principles. F.4.B.3.D delivered the biome-classification layer (climate field, Whittaker lookup, per-biome parameters, scattered-convolution blending, world archetype catalog + UI). Regional archetype variation requires architectural additions upstream of biome classification:

- Spatial bootstrap parameter variation (mountain amplitude, continental scale, base amplitude, etc. as functions of world position, not constants).
- Region-to-region blending at the bootstrap layer (analogous to D.4's blending at the per-biome layer, but operating on different parameters at a different scale).
- Authoring affordances for the writer to designate which world regions belong to which archetype.

The cleanest path is a new campaign. This audit's §7 sketches its scope; the next session drafts its ground-truth document.

---

## 2. Methodology

This research pass operates under the constraint that closed-source AAA references (Crimson Desert primarily, Enshrouded secondarily) require partial reverse-engineering from visual evidence, gameplay footage, dev interviews, and game journalism. Where claims rely on visual inference, this audit explicitly flags them as **[INFERRED]**. Published architectural facts are flagged as **[CITED]** with a URL.

Open-source / well-documented references (Minecraft 1.18+, NMS via GDC talk, Patel's noise terrain method, NoisePosti.ng scattered-convolution algorithm) provide the algorithmically-precise baseline against which AAA reverse-engineering observations can be triangulated.

The recommendation in §7 weighs published evidence more heavily than visual inference. Where Crimson Desert visual evidence and Enshrouded's published "handcrafted" admission converge with Minecraft 1.18+'s climate-driven density function architecture, the recommendation treats the convergence as load-bearing. Where evidence sources disagree, the recommendation chooses the path with the most published architectural detail (i.e., what AstraWeave can actually implement against, rather than what the aspirational reference appears to do).

Web searches conducted 2026-04-29; bibliography in §11.

---

## 3. Crimson Desert analysis (primary reference)

### 3.1 Methodological caveat

Pearl Abyss is more secretive than most AAA studios about engine internals, but the GDC 2025 closed-door presentation (March 2025) and post-launch technical journalism (Crimson Desert released March 19, 2026) yielded substantially more architectural detail than initially expected — particularly on rendering, GI, and streaming. Biome assignment, transition blending, and heightmap representation remain entirely proprietary. Engine has been publicly rebranded from "Black Spider" to "**BlackSpace**" (same lineage, ground-up redesign of the Black Desert Online 2015 engine).

### 3.2 Published architectural facts (BlackSpace engine)

**[CITED]** **Streaming pipeline** — three named primitives (GDC 2025):
- **Hierarchical level subdivision** — chunk hierarchy.
- **Hierarchical proxy load** — distance-LOD proxy assets.
- **Imposter rendering** — billboard/screen-space approximation for far field.

**[CITED]** Memory allocation and asset streaming are "hardcoded specifically for an open-world RPG's camera speed and draw distance" rather than a general engine. This is an architectural commitment baked into the engine's allocation budget at design time.

**[CITED]** **Rendering / GI architecture**:
- **Voxel SDF clipmaps** — narrow-band SDF bricks with empty-space culling, ~2 km coverage radius. Functionally analogous to UE5 Lumen's Global Distance Field.
- **Hardware BVH (TinyBVH)** — near-field (~100-150 m) ray-triangle acceleration.
- **Surfel buffer** — surface-point lighting cache backing the voxel SDF, continuously updated.
- **Froxel + fluid raymarching** for volumetric fog.
- **Full path tracing on PC** — all lighting unified under one photon-simulation model (no hybrid raster+RT).

**[CITED]** **Water simulation** — FFT Ocean (large bodies) + Shallow Water Simulation (rivers/shorelines). Water is volumetric data; terrain-water interaction resolved via volumetric coupling.

**[CITED]** **Climate parameters per region exist at runtime** — developer Q&A: weather is computed from "biome type, temperature, elevation, wind patterns, and time of day." Each region has its own climate profile producing distinct weather. This is the most architecturally relevant disclosure: confirms a per-region parameter block exists in the data model.

**[CITED]** **Procedural generation acknowledgment** — Kwaghyeon Go (Head of Game Engine Division), 2021 IGN interview: "Many aspects can be automated, procedurally generated, and customized." Which systems use procedural generation is not specified.

### 3.3 World structure

**[CITED]** Crimson Desert is a ~123-150 km² seamless open world (slightly larger than AstraWeave's Target B 115 km²). Five distinct regions on the continent of Pywel:

- **Hernand** — temperate forest/grassland, European high-fantasy (starter, ~east per narrative).
- **Pailune** — northern snow/mountains (only confirmed cardinal direction).
- **Demeniss** — central rolling farmland, political capital.
- **Delesyia** — exotic technology/sci-fi adversaries (technological frontier).
- **Crimson Desert** — lawless crimson-sand desert.

Per AllThings.How and Beebom region guides; spatial arrangement of regions other than Pailune is partially inferrable from narrative sequence but not officially mapped with cardinal coordinates.

### 3.4 Architectural inferences from visual evidence

**[INFERRED]** From gameplay footage, GDC 2025 hands-on coverage, and high-altitude flight scenes:

- **Macro layout appears hand-authored at region level.** Five archetypes across ~150 km² means each region averages 25-35 km². This grain favors coarse hand-authored placement (or large-scale noise-field assignment) over per-tile stochastic biome assignment.
- **Climate-per-region is the load-bearing data structure.** GDC 2025 disclosure that weather computes from biome type + temperature + elevation + wind patterns confirms per-region parameter blocks. AstraWeave's `WorldArchetype` is a structurally-aligned analog.
- **Transition zones are present but undocumented.** Reviewers (MMORPG.com GDC hands-on, GamingBolt PC analysis) explicitly noted absence of hard terrain seams during dragon/wyvern flight. No reviewer measured transition-zone width. Confidence: low — absence-of-reports is weak positive evidence only.
- **Latitude-gradient organizing axis (partial).** Pailune is explicitly north (snow); Hernand starter region likely east. Consistent with latitude-driven climate as one organizing dimension, with desert region likely south or interior.
- **Terrain is mesh geometry, not pure heightmap.** The voxel SDF clipmap GI pipeline ingests scene mesh geometry to build its distance field; close-range "extreme surface detail" (pebbles, weathered cliff faces) implies tessellation-from-heightmap or authored mesh in proximity zones. Heightmap-only rendering would not feed mesh-accurate SDF at close range.
- **No published evidence of explicit biome-blending shader or weight-map system.** The BlackSpace weather system suggests per-biome parameters at runtime, but the spatial blending curve is undocumented.

### 3.5 Modding-derived architectural signals

**[CITED]** Community PAZ/PAMT archive reverse-engineering (NattKh, lazorr410, faisalkindi GitHub repos): ChaCha20-Poly1305 encryption with key derived from filename hash via Bob Jenkins hashlittle, LZ4 block compression. **434 PABGB game data tables** with 3,708 fields documented (region/zone info, spawn density, quest data, vehicle, faction nodes). **No terrain heightmap, chunk, or biome spatial data has been extracted by the community to date.** This means whatever blending system Pearl Abyss uses is sufficiently bespoke that modders haven't surfaced it.

### 3.6 What we still don't know about Crimson Desert

- Heightmap representation (resolution, tile size, single 16-bit vs layered/virtual texture).
- Biome assignment mechanism (noise-field-driven, hand-painted splat map, spline-bounded regions, or hybrid).
- Transition zone blending methodology (no shader, blend-map, or algorithm details).
- Streaming cell geometry (named primitives but no tile dimensions, subdivision depth, memory layout).
- Terrain LOD algorithm (mesh tessellation, virtual heightmap, nanite-style mesh clusters, or proprietary).

### 3.7 What this means for AstraWeave

Crimson Desert at 123-150 km² with five archetype regions establishes the **product target is achievable at AstraWeave's scale**. The GDC 2025 disclosures provide three concrete architectural anchors:

1. **Per-region climate profile is the right data structure.** AstraWeave already has it (`WorldArchetype` from D.5).
2. **Streaming primitives are LOD-tiered hierarchy, not a single scheme.** AstraWeave's halo-based chunk pipeline composes with this; future LOD work can reuse the named-primitive vocabulary.
3. **Mesh-geometry terrain (not pure heightmap) at close range.** AstraWeave's current heightmap → mesh assembly pipeline already produces geometry; the Crimson Desert pattern just adds mesh-LOD tiers above it.

Crimson Desert tells AstraWeave "this is what good looks like at AstraWeave's scale" with a partial how. The implementation guidance for biome assignment / region blending must come from systems with deeper published architecture (§4-§5).

---

## 4. Enshrouded analysis (concrete-implementation reference)

### 4.1 World structure

**[CITED]** Enshrouded's world Embervale contains six major biomes arranged radially in a SW → E → N → NE progression axis (per Enshrouded wiki and 4netplayers biome guide):
- **Springlands** (Southwest, starter, woods/meadows; Low Meadows sub-biome east)
- **Revelwood** (North of Springlands, dense hardwood; Blackmire swamp sub-biome)
- **Nomad Highlands** (East of Springlands, limestone plateau; central divider between map halves)
- **Kindlewastes** (Southeast, sandstone mesas + desert canyons; *correction: not "Cinder Wastes" — that name appears nowhere in Keen Games sources*)
- **Albaneve Summits** (North of Kindlewastes, alpine/frost mountain, post-launch addition)
- **Veilwater Basin** (North, between Blackmire and Albaneve Summits, tropical jungle with ancient ruins)

**[CITED]** Biomes are gated by **Flame Level**, not by exploration radius. The Shroud is not a biome boundary in the terrain-authoring sense — it's an environmental hazard zone with Flame-Level-gated progression. Visually color-coded: white fog = survivable, red fog = deadly.

### 4.2 Published authoring pipeline (the architecturally most-relevant disclosure)

**[CITED]** Direct quote from Keen Games' World Design Team Reddit AMA (December 2023, recapped at enshrouded.com/news/ama-2-recap):

> "When tackling a new biome, the process starts with a very rough 3D model (usually sculpted) which serves as a foundation to get an idea for distances and possible POI locations. This is then added as a subbiome scene into the main scene and bit by bit smaller POI subscenes are added as placeholders to their locations while the environment connecting the POIs is being plastered with hand placed voxel stamps to give the rough 3D topology base as much detail as the voxel resolution can display."

**Authoring pipeline (5 stages)**:
1. Rough 3D sculpt (blockout for distance + POI placement)
2. Sub-biome scene injection (sculpt becomes a sub-scene in main scene)
3. POI subscene placeholders
4. Hand-placed voxel stamps (designers plaster stamps to fill topology to max voxel resolution)
5. Algorithmic detail pass (splines auto-generate roads/paths from designer-drawn lines; asset-scatter algorithm selects from hand-crafted prefab lists)

**This is a top-down authored pipeline, not noise-seeded procedural.** Procedural tools serve only fill and variation, not structural placement. Confirmed by multiple official sources (4netplayers, Gameoneer, Bleedingcool, MMORPG.com, official AMA recap).

**[CITED]** Authoring tools published (Art Director developer video, December 2023):
- **Voxel brushes** — free-form terrain sculpting.
- **Negative stamps** — subtractive carving (caves, canyons).
- **Voxel objects** — modular prefab elements placed in-world.
- **Splines** — roads/paths drawn as lines, auto-rendered by algorithm.
- **Asset scatter algorithm** — designer draws a forest region; algorithm selects from hand-crafted tree prefab list.
- **Real-time iteration** — changes visible live while running in-world.

**Key absence**: no biome-boundary painter, noise-mask, or weight-map authoring tool described. **Transitions appear to result from designers sculpting adjacent biomes**, letting the voxel geometry create natural terrain-type gradients, rather than from an explicit blend-region authoring step.

### 4.3 Engine architecture (Holistic engine published facts)

**[CITED]** **Voxel resolution: 0.5 m per voxel side** (developer AMA, December 2023). This is build/destruction granularity; world-display uses LOD reduction at distance.

**[CITED]** **Vulkan-exclusive rendering** (no DX12, no OpenGL). Custom memory manager, synchronization layer, pipeline compilation strategy — documented in GPC 2024 Vulkan talk.

**[CITED]** **Engine published research venue: Graphics Programming Conference (GPC) Breda, Netherlands**, NOT GDC. Six published technical talks with PDF slides + YouTube videos:

GPC 2024:
- "Volumetric Fog in Enshrouded" — Lukas Feller
- "Dynamic Diffuse & Specular GI in Enshrouded" — Jakub Kolesik (Vulkan Raytracing → custom SDF rays for GPU compatibility)
- "Vulkan in Enshrouded" — Julien Koenen + Lukas Feller (memory management, synchronization, pipeline compilation)

GPC 2025:
- "The Fog is Lifting: Volumetric Rendering Enshrouded" — Philipp Krause (dual froxel atmosphere + ray-marched Shroud + cloudscape ray-march)
- "Flooding the World of Enshrouded" — Andreas Mantler + Julien Koenen (water in dynamic voxel world, simulation + networking + rendering)
- "Lessons Learned from Shipping a GPU Particle System" — Lukas Feller

**[CITED]** **Shroud rendering: dual-system**:
- Froxel-based (frustum voxel grid): atmosphere + general weather.
- Ray-marched: the Shroud itself (sharp visible boundaries + volumetric density variation).

The Shroud boundary is defined in the voxel world representation, then rendered via separate ray-march pass — not as a texture-blended terrain region. **The "biome boundary" concept and the "environmental hazard zone" concept are implemented as separate systems.**

### 4.4 Scale comparison to AstraWeave

| Metric | Enshrouded | AstraWeave Target B |
|--------|-----------|---------------------|
| EA launch world | 24 km² | — |
| Full-release target | 64 km² | 115 km² (~1.8× Enshrouded) |
| Voxel/vertex resolution | 0.5 m | 5.39 m vertex spacing (heightmap) |
| Generation | Authored offline, streamed static | Runtime procedural |
| Biome layout | Handcrafted SW→E→N→NE arc | Currently single archetype with biome variation |

AstraWeave operates at significantly larger horizontal extent with much coarser vertex spacing. Enshrouded's 0.5 m voxels make hand-authored terrain practical at 64 km²; AstraWeave's runtime procedural pipeline at 115 km² makes hand-authored terrain impractical at the geometry layer but **practical at the macro-region layer** (a few hundred hand-painted region IDs, not 2 billion voxels).

### 4.5 What this means for AstraWeave

Three architecturally-actionable signals:

1. **Hand-authored macro layout + procedural detail filling is the dominant AAA pattern.** Enshrouded explicitly: "handcrafted by the developers." This matches Veilweaver's authoring intent (Tikva writer pins archetype regions). The architecturally-relevant question is *what authoring resolution AstraWeave commits to* — Enshrouded paints at 0.5 m voxels; AstraWeave should paint at archetype-region scale (one ID per region, falloff for blending), not at vertex scale.

2. **No published biome-boundary blending architecture.** Even Enshrouded — the most documented AAA reference — has no published transition-zone shader, weight map, or blend algorithm. Transitions are implicit from designer-sculpted adjacency. AstraWeave's published prior art (NoisePosti.ng scattered convolution) is more architecturally developed than Enshrouded's published method.

3. **Engine talks focus on rendering, not generation.** Six Keen Games GPC talks; zero on terrain generation/authoring pipeline. AstraWeave's published architectural detail (this audit, the F.4.B.3.D campaign doc, the diagnostic-3 audit) is already deeper than Keen Games' published terrain generation content. AstraWeave should not expect to find a directly-portable algorithm in Enshrouded sources.

---

## 5. Supplementary references

### 5.1 Minecraft 1.18+ "Caves & Cliffs" MultiNoise Biome architecture

**[CITED]** From Minecraft Wiki Noise Router page, MultiNoiseUtil Yarn API, Alan Zucconi deep-dive, Dawnosaur Substack, and Henrik Kniberg's Mojang talk:

**Six-parameter climate space** — every 4×4×4 block volume in the Overworld is assigned values across:
- `temperature` (5 quantized levels, ranges -1.0→-0.45 through 0.55→1.0; horizontal-only)
- `humidity` / `vegetation` (5 levels, ranges -1.0→-0.35 through 0.3→1.0; horizontal-only)
- `continentalness` / `continents` (7 levels, -1.2→-1.05 = mushroom fields/deep ocean → 0.3→1.0 = far inland; high = high terrain)
- `erosion` (7 levels, -1.0→-0.78 through 0.55→1.0; high = flat terrain; low = mountainous)
- `weirdness` / `ridges` (feeds the **Peaks and Valleys (PV)** function: `PV = 1 − |(3 × |weirdness|) − 2|` — folded weirdness produces 5 categorical terrain levels: Valleys/Low/Mid/High/Peaks)
- `depth` (only parameter with vertical component; ~0 at surface, +1/128 per block downward)

**Biome assignment: nearest-neighbor lookup in 6D hypercube space.** Each vanilla biome is a `MultiNoiseUtil.NoiseHypercube` record (Yarn API: deobfuscated public class) carrying six `ParameterRange` fields plus an `offset` weight. The game computes squared Euclidean distance between the sampled climate point and each registered biome's hypercube and assigns the closest. **No fallback default — closest still wins if outside all defined intervals.**

**Decoupled architecture**: "this field and the following five fields do not affect terrain shape, as terrain generation is defined in `final_density`." Biome assignment runs in parallel to terrain shape generation.

**Spline-driven terrain shape (the architecturally-critical pattern):**
1. Three 2D noise maps — `continentalness`, `erosion`, `peaks_and_valleys` — fed into **spline functions**.
2. Splines output two values: **`offset`** (terrain height baseline shift) and **`factor`** (vertical stretch/squash coefficient).
3. A 3D Perlin noise provides raw density.
4. Density adjusted: `output = (raw_density + offset) × factor`.

The splines are continuous piecewise functions with authoring control points mapping `(continentalness, erosion, PV)` tuples to terrain shape parameters. **High continentalness + low erosion + high PV → steep-sided mountains. High erosion → flat terrain regardless of other parameters.** Designers control which macro-climate combinations produce which terrain shapes via spline JSON (data-driven, no engine code changes).

**Biome transitions**: NOT explicit gradient lerp. Climate parameter values change continuously across space (gradient noise); biome assignment snaps to nearest hypercube at each sample point. **Snap boundary is only as sharp as the climate parameter gradient allows** — geographically natural transitions emerge as parameter-space trajectories rather than hard cuts.

**Visual color blending** (grass/foliage at biome borders): separate client-side post-process with configurable `biome_blend` radius operating on neighborhood kernel. Independent of climate noise logic. Better Biome Blend mod extends radius and uses OKLab perceptual color space for smoother hue transitions.

### 5.2 What Minecraft 1.18+ teaches AstraWeave

This is the **single most architecturally-relevant published reference** for AstraWeave's design space. Two critical patterns transfer directly:

**Pattern 1 — NoiseHypercube nearest-neighbor archetype lookup.** AstraWeave's D.5 `WorldArchetype` catalog (6 archetypes with mean + variance climate envelopes) is structurally aligned with Minecraft's NoiseHypercube design. Extending to N archetypes only requires adding more hypercube definitions; the lookup algorithm scales linearly. **Critical caveat for the new campaign: nearest-neighbor lookup gives implicit blending only if climate parameters are continuous noise; explicit author-painted regions need a different blending mechanism.**

**Pattern 2 — Spline-driven shape parameters.** AstraWeave's per-vertex `mountain_amplitude_multiplier` is currently a per-biome constant. Replacing it with a spline that reads `(continentalness, erosion, weirdness)` from the climate field would give terrain shape character that varies smoothly across the world driven by climate parameters. **This is the Approach F architecture** from §6.6, and Minecraft 1.18+ is its canonical implementation.

**Adaptation for heightmap (drop voxel-specific elements):**

```rust
// Pseudocode: climate-to-height spline pipeline for AstraWeave heightmap
let c = climate.continentalness;
let e = climate.erosion;          // new field; D.1 doesn't have it yet
let w = climate.weirdness;        // new field
let pv = 1.0 - ((3.0 * w.abs()) - 2.0).abs();  // Peaks-and-Valleys fold

let height_offset = eval_spline_3d(archetype.offset_spline, c, e, pv);
let height_factor = eval_spline_3d(archetype.factor_spline, c, e, pv);

let detail = sample_fbm(world_x, world_z);  // existing TerrainNoise output
let height = height_offset + detail * height_factor;
```

**NOT applicable from Minecraft**: 3D density function chain (`final_density`, aquifer logic, cave noise, depth parameter). These are voxel-specific. The `depth` parameter and cave biome placement have no heightmap analogue.

### 5.3 No Man's Sky (Hello Games, Horizon engine)

**[CITED]** Innes McKendrick's GDC 2017 "Continuous World Generation in No Man's Sky" + Sean Murray's GDC 2017 "Building Worlds Using Math(s)" + community modding wiki reverse-engineering of `VoxelGeneratorSettings`:

**Hierarchical deterministic seed cascade**: galaxy seed (single 64-bit value) → star coordinates → planet count + orbital positions → per-planet terrain archetype + climate. Fully deterministic at each level.

**One archetype per planet** (hard design constraint). 10 documented archetypes + 10 "extreme mountain" variants post-Origins (September 2020). Asset-budget rationale: multi-biome planets would require 2-3× the biome asset count.

**Per-archetype noise stack** (`VoxelGeneratorSettings`, 10 entries):
- 7 `TkNoiseUberLayerData` entries (each named: "Base", "Mountain", etc. — first drives general shape, second drives sharp vertical features). Each contributes additively or multiplicatively to density field.
- 9 `TkNoiseGridData` entries with `TurbulenceNoiseLayer` (column/pillar formation, floating islands, resource positioning).
- 7 `TkNoiseFeatureData` entries (`ArchesSmall`, `BlobsSmall`, etc. — discrete structural features).
- 1 `TkNoiseCaveData` entry (underground void carving).

**Uber Noise** (Hello Games proprietary) = fBm augmented with:
- **Domain warping** — each sample point offset by second fBm map before evaluation (twisted/eroded landscape character).
- **Absolute-value ridge functions** — `|noise|` creates sharp ridge lines.
- **Slope/altitude erosion** — octave amplitudes conditioned on local slope or altitude (suppresses high-frequency detail in flat areas, boosts on steep faces).

Parameters: scale, octave count, gain, lacunarity, domain warp amplitude. Modders edit per-biome `.MBIN` files (`GcTerrainControls`) to tune these.

**Intra-planet variation: implicit gradient.** No explicit regional biome zones within a planet. Geographic variation emerges from continuous noise field evolution: "In planets with oceans, terrain closer to the coast is flatter ... As one goes farther inland, it gradually becomes more and more mountainous." No blend zone authored — emergent property.

**Authoring philosophy** (McKendrick GDC 2017): "The engine is agnostic about where the content comes from, making no distinction between generative content and hand-authored content. [Goal:] enable our artists to produce more, rather than replacing them with an algorithm that doesn't quite do the same job." Grant Duncan: terrain generation code is ~1,400 lines; almost all variation is authored data, not algorithmic complexity.

### 5.4 What NMS teaches AstraWeave

**Pattern transfer — per-archetype noise stack as data.** NMS's `TkNoiseUberLayerData` per-archetype configuration is structurally what AstraWeave's `BootstrapSplineSet` (proposed in §7) becomes. Each archetype gets a multi-layer noise configuration; the engine instantiates terrain from the parameter block. NMS confirms this scales to 10+ archetypes with manageable authoring overhead.

**Pattern transfer — domain warping + abs-ridge as published primitives.** AstraWeave's F.2-T-4 already has derivative-weighted fBm (Quilez morenoise). Adding domain warping (NMS pattern) + abs-ridge (NMS / standard Musgrave) per-archetype gives the per-archetype shape variation diagnostic-3 measured as missing.

**NOT applicable from NMS**: spherical voxel projection (octree-on-cube), one-biome-per-planet hard constraint (a design choice for asset budget; AstraWeave needs intra-world variation), `TkNoiseCaveData` underground system.

**Critical limitation as a model for Veilweaver**: NMS's archetype assignment is per-planet, not intra-world. Veilweaver needs intra-world regional archetype variation; NMS's emergent-from-noise approach is too implicit for writer-controlled storyline regions. NMS's value is the **per-archetype noise stack as data structure**, not the assignment mechanism.

### 5.4 Witcher 3 (CD Projekt Red, REDengine 3)

**[CITED]** GDC 2014 talk by Marcin Gollent on REDengine 3 landscape (PDF inaccessible during this research, but search result summaries:
- Vegetation procedurally generated; rock formations and mountains hand-made.
- Terrain dimensions: Velen+Novigrad ~8625m × 8625m (~74 km²), Skellige 7472m × 7472m (~56 km²).
- Multiple separate worldspaces (Velen, Skellige, Toussaint, Kaer Morhen) — not one connected continent.

**[INFERRED]** Witcher 3's regional variation is **across worldspaces** (separate maps), not within them. Adjacent biomes within Velen (e.g., Crookback Bog vs Novigrad outskirts) appear hand-authored at the macro scale with procedural vegetation filling. Transition implementation: probably hand-painted region-marker layers driving vegetation rules, not parametric climate blending.

### 5.5 Horizon Zero Dawn / Forbidden West (Guerrilla Games, Decima)

**[CITED]** From 80.lv article on procedural nature of HZD:
- "Vegetation and rivers used procedural placement, rock formations and mountains were hand-made."
- "Every aspect of the world, although generated by a clever algorithm, could be easily modified by an artist. They could alter the placement logic and work with hand-authored assets."

**[INFERRED]** Decima follows Hytale-style philosophy: procedural systems are designer-controlled, not designer-replacement. Biome boundaries are likely hand-authored regions; procedural systems fill density.

### 5.6 Skyrim (Bethesda, Creation Engine)

**[CITED]** From Creation Kit wiki and modding documentation:
- Worldspace structure: 4096×4096-pixel limit, 128×128 cells centered on cell (0,0).
- Heightmap-based, with hand-authored regional placement.
- "Region" assignment via Construction Kit region records (manually placed by designers).
- LOD generated offline via Oscape/TES Annwyn.

**[INFERRED]** Skyrim's regions (Tundra, the Reach, Whiterun plains, Riften autumn) are **explicitly designer-defined zones** with associated rules for vegetation, weather, encounter tables. Boundaries are hand-painted. Closest analog to Veilweaver's authoring intent.

### 5.7 Hytale (Hypixel Studios)

**[CITED]** From allthings.how summary:
- "Procedural does not mean 'designer hands off' — Hytale's team treats world generation as an authored system where designers decide which biomes can meet, which structures are allowed in each region, and how resources are distributed."

The principle: **procedural systems serve authored intent.** Designers control biome adjacency rules, structure placement constraints, resource distributions; the procedural engine respects these constraints when generating detail.

### 5.8 Convergent finding across §3-§7

Five out of six AAA references (Crimson Desert [INFERRED], Enshrouded [CITED], Witcher 3 [INFERRED], Horizon [CITED], Skyrim [CITED], Hytale [CITED]) use **hand-authored macro layout + procedural detail filling**. NMS is the outlier (per-planet algorithmic archetype selection), and even NMS's design principle is "agnostic between authored and generated."

Minecraft 1.18+ is the algorithmic outlier that uses **climate-driven shape variation with no hand-authored layout**. But the spline-based density function pattern produces visibly distinct terrain shape character per climate region without explicit hand-authoring. This is the closest pure-procedural analog that achieves the visual target.

The convergence: Veilweaver's authoring intent (writer pins archetypes to storyline regions) aligns with the AAA dominant pattern. The implementation guidance comes from Minecraft's spline-based climate-driven shape variation as the procedural backbone.

---

## 6. Algorithmic approaches surveyed

For each approach, evaluation against five criteria: architectural complexity (vs D.1-D.5 baseline), authoring UX, blending quality, performance (vs current ~0.747s/chunk baseline), scalability to 5-10 archetypes.

### 6.1 Approach A — Voronoi region assignment with blend zones

**Concept**: World divided into Voronoi cells around author-placed (or procedurally seeded) archetype centers. Bootstrap parameters interpolate within a configurable blend distance of cell boundaries. Each cell carries one archetype's parameter set; vertices in transition zones blend between adjacent cells' parameters.

**References**: Patel's polygonal map generation (Stanford/redblobgames), NoisePosti.ng's scattered-convolution algorithm (already in use at AstraWeave's biome layer per D.4).

**Architectural complexity**: Moderate. Voronoi cell computation per chunk is cheap; cell-to-archetype assignment requires either a published seed list (deterministic procedural) or an authoring step (writer places seeds). Blend zone width is a global parameter or per-edge.

**Authoring UX**: Author places N archetype seeds; cells auto-derive. Easy to iterate; low-control over exact boundary shape (cells are convex polygons unless extended with cell-shape jittering).

**Blending quality**: Convex polygons can read as visible if blend zones are too narrow. The NoisePosti.ng scattered-convolution variant produces organic-looking blends. AstraWeave already has this code in `biome_param_blending.rs`.

**Performance**: Per-vertex Voronoi-cell lookup + N-sample weighted blend (N=4-9). Comparable to D.4's per-vertex cost (currently +50% vs F.4.B.2.G baseline). ~+20-30% on top of D.4 for the additional layer.

**Scalability to 5-10 archetypes**: Excellent. Voronoi handles arbitrary cell counts; blending only considers adjacent cells (not all archetypes globally).

### 6.2 Approach B — Climate-field-driven archetype selection (D.1 extension)

**Concept**: Extend the existing D.1 climate field with archetype-discriminating dimensions (e.g., "tectonic activity" or "geological era" as additional climate-like fields). Archetype identity emerges from the climate sample at each vertex via a polygon classifier analogous to D.2's Whittaker biome lookup. Bootstrap parameters interpolate based on climate sample distance to archetype envelope centers.

**References**: Minecraft 1.18+ noise router (six-parameter climate space → biome assignment via polygon matching). NMS continuous parameter blending.

**Architectural complexity**: Low. Reuses D.1's climate field architecture; adds 1-2 new fields and an archetype-classifier function analogous to `lookup_biome`. The challenge: selecting climate-like dimensions that discriminate archetypes (latitude alone won't work because Mediterranean and Continental Temperate share latitudes).

**Authoring UX**: Implicit (archetype emerges from climate). Author tunes archetype envelope centers (similar to D.5's `WorldArchetype` parameter block); cannot pin a specific region to a specific archetype except via climate-field hints (which the climate field's noise may override).

**Blending quality**: Smooth by construction (climate fields are continuous). Same caveats as D.1 — archetype boundaries follow climate isolines, which may not match writer intent.

**Performance**: Per-vertex climate sample (already computed in D.1) + archetype lookup (cheap, polygon match). Negligible cost vs D.4 baseline.

**Scalability to 5-10 archetypes**: Good in theory; degrades if climate dimensions don't have enough discrimination range. With 7 climate fields (D.1's 3 + 4 archetype-discriminating fields), 10 archetypes leaves headroom.

**Critical limitation**: Does not honor authoring intent. Veilweaver wants the writer to designate "this region is Boreal Subarctic for storyline X." Climate-driven assignment can't pin specific regions; the noise may put Boreal Subarctic in the wrong place.

### 6.3 Approach C — Author-painted archetype regions with falloff

**Concept**: Editor tool lets the writer paint archetype zones onto a 2D world map. Each painted region carries an archetype identifier; falloff width is per-region or global. Bootstrap parameters at any vertex are computed from a distance-weighted mix of painted regions within the falloff radius.

**References**: Unreal Landscape, Godot TerraBrush, Skyrim Construction Kit region records, Houdini terrain regions, Gaea masking.

**Architectural complexity**: Higher. Requires (1) a paintable 2D archetype mask, (2) a runtime sampler for the mask + falloff, (3) parameter blending across painted regions, (4) editor UX integration. Mask resolution choice has memory and quality implications.

**Authoring UX**: Excellent. Direct designer control over which regions get which archetypes. Matches Veilweaver's authoring intent precisely.

**Blending quality**: Configurable per-region. Falloff can be hard (sharp) or soft (gradient) per region; designers pick the look.

**Performance**: Mask sampling per vertex is cheap (single texture-style lookup). Multi-region blending costs scale with average overlap depth (typically 1-3 regions in transition zones).

**Scalability to 5-10 archetypes**: Excellent. Mask carries an archetype ID per pixel (or RGBA-encoded multi-archetype contributions); regions can be any shape and any count.

**Critical advantage**: Honors writer intent. Disadvantage: requires authoring time (writer must paint the world; tooling investment).

### 6.4 Approach D — Hierarchical noise pyramid (archetype as low-frequency noise)

**Concept**: Archetype identity is itself a low-frequency noise field (the "archetype noise"). Smaller-scale bootstrap parameters modulate by sampling this archetype field at coarse resolution. Pure procedural; no authoring.

**References**: Standard fractal noise terrain pipelines (Perlin octaves), AstraWeave's existing continental modulation pattern (which already implements a low-frequency-noise-modulates-mountain-amplitude scheme at one parameter).

**Architectural complexity**: Lowest. Adds one or more low-frequency noise fields; existing bootstrap parameters multiply by the archetype field at sample time. Implementation similar to current `continental_min` modulation but expanded to multiple fields.

**Authoring UX**: Poor. Author chooses noise seed; little control over specific region placement. Re-running with a different seed produces a different layout.

**Blending quality**: Excellent (continuous noise fields are inherently smooth).

**Performance**: Cheapest. One additional low-frequency noise sample per vertex.

**Scalability to 5-10 archetypes**: Hard. Mapping a continuous noise value to 5-10 discrete archetypes requires polygon classification or thresholding; transitions are coarse unless archetype boundaries follow noise isolines (which may not produce visually plausible regions).

**Critical limitation**: Does not honor authoring intent. Same limitation as Approach B but worse — at least Approach B exposes the archetype envelope as an editable parameter; Approach D buries it inside noise.

### 6.5 Approach E — Hybrid (painted regions + climate-field fallback)

**Concept**: Author paints high-priority regions (the Tikva storyline zones); climate-field-driven assignment fills in unpainted areas using Approach B's mechanism. Painted regions take priority over climate-driven assignment in regions where they overlap.

**References**: Hytale's "designers control which biomes can meet" philosophy + Minecraft 1.18+'s climate-driven backbone.

**Architectural complexity**: Highest. Combines Approach C's paintable mask with Approach B's climate-field architecture. Requires priority-merging logic at vertices where painted and climate-driven assignments disagree.

**Authoring UX**: Best of both worlds. Writer paints critical storyline regions; the rest of the world fills procedurally with archetype variation following climate.

**Blending quality**: Configurable per painted region; climate-driven regions blend smoothly.

**Performance**: Approach C cost + Approach B cost. ~2× per-vertex archetype assignment cost vs single approach.

**Scalability**: Excellent.

**Critical advantage**: Highest authorial control + procedural coverage. Critical disadvantage: highest implementation complexity; longest campaign timeline.

### 6.6 Approach F — Multi-scale noise with climate-driven splines (Minecraft-inspired)

**Concept**: Extend AstraWeave's existing climate field with continentalness + erosion-style parameters at world-spanning scale. Bootstrap parameters become spline functions of these climate parameters. Different splines for different parameters produce different terrain shape character at different climate regions. No archetype identity per se; archetype emerges from spline outputs at climate samples.

**References**: Minecraft 1.18+ noise router (canonical implementation).

**Architectural complexity**: Moderate. Requires (1) extending D.1 climate field with continentalness + erosion-style parameters at world-spanning scale, (2) spline-based density functions for mountain amplitude / scale / etc., (3) authoring affordance for the splines (designer tunes the spline curves).

**Authoring UX**: Implicit (archetype emerges from climate via splines). Author tunes spline curves; doesn't paint regions. Indirect control.

**Blending quality**: Excellent (splines are continuous; climate fields are continuous).

**Performance**: Per-vertex climate sample (D.1) + spline evaluations (cheap, ~10 ns per spline). Negligible vs D.4 baseline.

**Scalability**: Good for shape variation; limited for biome-specific authored content. Does not pin regions to authored archetypes; if writer requires "Boreal here, Mediterranean here," this approach won't deliver it.

**Critical limitation for Veilweaver**: Doesn't honor explicit authoring intent. Same as Approach B/D.

### 6.7 Cross-approach summary

| Approach | Complexity | Auth UX | Blending | Perf cost | Scale 5-10 | Honors author intent |
|----------|------------|---------|----------|-----------|------------|----------------------|
| A — Voronoi + blend zones | Moderate | Good (place seeds) | Good (NoisePosti.ng) | +20-30% | Excellent | Partial |
| B — Climate-field extension | Low | Implicit | Excellent | Negligible | Good | **No** |
| C — Painted regions + falloff | Higher | Excellent | Configurable | Moderate | Excellent | **Yes** |
| D — Hierarchical noise | Lowest | None (seed only) | Excellent | Lowest | Hard | **No** |
| E — Hybrid (C + B) | Highest | Best | Best | Highest | Excellent | **Yes** |
| F — Climate splines (Minecraft) | Moderate | Implicit | Excellent | Negligible | Good (shape only) | **No** |

The "Honors author intent" column is the deciding criterion for Veilweaver. Only Approaches A (with author-placed seeds), C, and E satisfy this criterion. Approach B/D/F are procedural-only and don't pin specific regions to specific archetypes.

---

## 7. AstraWeave + Veilweaver Recommendation

### 7.1 Recommended approach: **Hybrid C + F** (painted regions + climate-driven shape splines)

The recommendation: **Approach C (painted archetype regions with falloff) for archetype assignment + Approach F (climate-driven shape splines à la Minecraft 1.18+) for the per-archetype shape character.** Two layers of architectural commitment, neither alone sufficient:

- **Painted regions** honor writer intent: Tikva storyline regions get pinned archetype identities; the editor surfaces a paintable 2D archetype mask. **Confirmed pattern by Crimson Desert (climate profile per region runtime), Enshrouded (handcrafted macro), Skyrim (region records), Hytale (designer-controlled biome adjacency).**
- **Climate-driven splines** produce per-archetype shape character: each archetype's `WorldArchetype` parameter block carries spline curves for `mountains_amplitude`, `mountains_scale`, `continental_scale`, `base_elevation_amplitude`. Splines read the climate field (D.1's `ClimateMap::sample`, extended with `erosion` and `weirdness`) and compute per-vertex bootstrap parameters within each region. **Canonical implementation: Minecraft 1.18+ noise router with `(continentalness, erosion, PV) → (offset, factor)` splines.**
- **Per-archetype noise stack as data structure** (NMS pattern): each archetype carries a `BootstrapSplineSet` configuration block analogous to NMS's `TkNoiseUberLayerData`. AstraWeave's `WorldArchetype` becomes the parent record; `BootstrapSplineSet` is its shape-character payload.

The painted mask gives the writer **what** the region's character is. The splines + per-archetype noise stack give each region's character **how** the terrain expresses it (mountains where continentalness is high + erosion is low; flat where erosion is high; jagged where PV is high; etc.). The combination produces:

- Hand-authored macro layout (Crimson Desert pattern, Enshrouded pattern, Skyrim pattern).
- Per-region procedural shape character (Minecraft 1.18+ + NMS pattern, within each region).
- Smooth transitions (painted region falloff + climate-driven spline continuity within blend zones).
- **Convergent with all 6 AAA references in §3-§5.7**: the two architectural layers (authored macro + procedural shape) are exactly the design boundary every shipping AAA reference uses.

### 7.2 Integration with D.1-D.5 landed work

**Stays unchanged**:
- D.1 `ClimateMap::sample` (climate field per-vertex API).
- D.2 `lookup_biome` (Whittaker classification at the biome layer).
- D.3 `BiomeParameters` (per-biome shape character within a region).
- D.4 `blend_biome_parameters` (scattered-convolution biome blending).
- D.5 `WorldArchetype` (climate envelope; gets extended with spline curves and a paintable mask reference).

**Gets extended**:
- `WorldArchetype` adds `bootstrap_splines: BootstrapSplineSet` (per-archetype splines for mountains_amplitude, mountains_scale, continental_scale, base_elevation_amplitude, possibly noise type/octaves choice — NMS `TkNoiseUberLayerData`-equivalent).
- `ClimateMap::sample` adds `erosion` and `weirdness` fields. D.1's existing `continentalness` is already present; `erosion` is a new low-frequency noise field representing flatness propensity (high erosion → flat; low erosion → mountainous, per Minecraft 1.18+ canonical interpretation); `weirdness` is the input to the PV (Peaks-and-Valleys) fold.
- `WorldGenerator::generate_chunk_with_climate` reads the painted archetype mask + falloff, computes per-vertex archetype parameter blend (similar to D.4 blending mechanism, applied at the regional layer), then evaluates archetype splines on climate sample.

**New components**:
- `RegionalArchetypeMask` — author-paintable 2D mask resource. Resolution choice: probably 1024×1024 RGBA8 spanning the 115 km² world (~10.5 m per pixel, sufficient for archetype-region authoring; transitions handled by falloff at sampling time, not by mask resolution).
- `RegionalArchetypeBlend` — runtime sampler that reads the mask + falloff and produces per-vertex archetype contributions (analog to `BlendedBiomeParams`). Reuses NoisePosti.ng scattered-convolution algorithm at the regional scale (parameters: 4-8 samples, ~256 WU radius — vs D.4's 6 samples / 48 WU radius at the biome scale).
- `BootstrapSplineSet` — per-archetype spline definitions for bootstrap parameters. Each spline is a piecewise function with author-controlled control points; data-driven (JSON or RON), not engine-code.
- `PvFold` helper — implements `PV = 1 - |3 × |w| - 2|` per Minecraft 1.18+ canonical formula.
- Editor UI for painting archetype regions (analog to existing terrain paint/sculpt UX; brush + falloff + archetype-ID selector).

### 7.3 High-level sub-phase sketch (input to next session)

The campaign-design session expands these. Maximum 2 sentences each.

- **F.0 — Campaign plan + literature triangulation.** Draft the campaign ground-truth document. Mirror this audit's structure for §0-§2 (discipline, design summary, architecture); enumerate F.1-F.N sub-phases with success criteria.

- **F.1 — Climate field extension for shape parameters.** Add erosion-style fields to D.1's climate field; verify no regression of D.5-fix Path B amplitude reduction on existing archetypes. Backward-compat invariant: existing per-biome `mountains_amplitude` continues to work.

- **F.2 — `BootstrapSplineSet` infrastructure.** Define per-archetype spline structure carrying curves for mountains_amplitude, continental_scale, base_elevation_amplitude. Land splines as data; do not yet wire into `sample_height`.

- **F.3 — Spline wiring (single archetype).** Wire `BootstrapSplineSet` into `WorldGenerator` for one archetype (Continental Temperate). Verify it reproduces D.5-fix's measured behavior at that archetype's defaults (regression test). Per-archetype shape character is now expressible.

- **F.4 — `RegionalArchetypeMask` + falloff sampler.** Implement the paintable mask format, the runtime sampler, and the archetype-blend per-vertex aggregation. No editor UI yet (test via programmatic mask construction). Sub-campaign-internal Andrew-gate: mask-driven assignment produces correct archetype IDs at painted positions and smooth transitions in falloff zones.

- **F.5 — Editor UI for archetype painting.** New panel + paint-tool integration. Author places archetype regions with brush + falloff controls. Andrew-gate: writer paints a Veilweaver-realistic 5-region world (CT center, Boreal north, Mediterranean south, Desert east, Tropical west — or similar) and validates terrain output reads as Crimson-Desert-class regional variation.

- **F.6 — Scattered-convolution at the regional layer.** Apply NoisePosti.ng-style organic blending to mask-driven archetype assignments at boundaries (analogous to D.4 at the biome layer). Eliminates axis-aligned artifacts in transition zones if Voronoi-style cells emerge from the mask.

- **F.7 — Per-archetype tuning + Andrew-gate.** Tune the 6 catalog archetypes' splines (Continental Temperate, Equatorial Tropical, Boreal/Subarctic, Mediterranean, Desert, Custom) so each produces visibly distinct character. Re-run diagnostic-3-style cross-archetype Pearson measurement; target Pearson < 0.7 across measured pairs (vs current 0.978-0.989).

- **F.8 — Campaign closeout.** F.4.B.3.D.6 deliverables (deferred work: Climate Preview overlay, ET archetype-specific tuning, dead-write bug, 47.4 WU floor, behavioral_correctness_tests stale literals, MountainRocky/River unproduced biomes, per-biome runevision tuning). Some absorbed; some standalone.

Estimated sub-phase count: 8 (F.0 through F.8). Estimated session count: 8-12 (some sub-phases need diagnostic + remediation pairs per F.4.B.3.D's pattern). Comparable in scope to F.4.B.3.D campaign; smaller in research effort because architectural patterns are now documented.

### 7.4 Risk assessment

**Highest risk**: writer authoring time. If the editor UI is clumsy or the mask format is brittle, the writer will paint slowly, iterate poorly, or refuse to use the system. Equivalent of D.5 Andrew-gate: Andrew (or proxy writer) paints a Veilweaver world in <30 minutes; output reads as plausible.

**Second-highest risk**: spline tuning is iterative. Each archetype's bootstrap splines need calibration; miscalibration produces back-to-square-one cartoon-shape problems. Mitigation: F.7 includes diagnostic-3-style cross-archetype Pearson re-measurement as objective convergence signal.

**Third risk**: performance at radius 10 with 5-10 archetypes. Per-vertex archetype-blend + spline evaluation may push generation time over budget. Mitigation: F.4 measures perf early; if over-budget, optimize mask sampling (mipmap pyramid?) or reduce blend zone radius.

**Fourth risk**: D.1-D.5 invariants regression. Existing biome-classification + per-biome blending must continue working within each archetype region. Mitigation: F.1's regression tests verify D.5-fix Path B output unchanged at Continental Temperate defaults.

### 7.5 Equivalent of D.5 Andrew-gate for new campaign

After F.7's per-archetype tuning, run a modified diagnostic-3:
- 5 archetype zones painted on a single world (CT, Boreal, Mediterranean, Desert, Tropical).
- 5 chunks sampled per (archetype × archetype) pair.
- Cross-archetype Pearson must drop from 0.978-0.989 (current state) to **target < 0.7** (heightmaps appear visibly distinct, not nearly identical).
- Visual: writer/Andrew confirms each painted region produces archetype-appropriate terrain (Mediterranean coast looks like coast; Boreal mountains look alpine; etc.).

---

## 8. Deferred-work inheritance plan

### 8.1 Items absorbed into new campaign

- **Per-archetype world shape variety**: this is the core of the new campaign.
- **Equatorial Tropical archetype-specific tuning**: handled in F.7 as part of per-archetype tuning pass.
- **Bootstrap noise pipeline elevation skew (audit §7 of diagnostic-2, "26% of vertices land at elevation ≥ 450m")**: Path B reduced amplitudes; new campaign's spline wiring may further tune per-archetype distribution.
- **Climate Preview overlay (D.5c)**: usefulness amplified by regional archetypes (writer needs to see archetype regions during painting). Land in F.5 alongside the painting UI.

### 8.2 Items kept as standalone follow-ups

- **F.4.B.3.G (47.4 WU phase-2 grassland precision floor)**: this is an f32 precision issue at chunk shared edges, unrelated to regional archetype variation. Stays in F.4.B.3.G's scope.
- **Dead-write bug at terrain_panel.rs line 943**: small cleanup; lands as a one-shot fix anytime.
- **Stale literal `NoiseConfig {...}` constructors in `behavioral_correctness_tests.rs`**: pre-existing test file failure (lines 517, 911, 912 missing fields added by F.2-T-4 / F.4.B.3.B / F.4.B.2). Standalone fix; new campaign should avoid repeating the same pattern.
- **MountainRocky / River unproduced biomes**: River requires hydrology campaign (out of scope); MountainRocky reserved for slope-conditional expression (could be wired in new campaign if relevant, otherwise stays unproduced).
- **Per-biome runevision tuning**: F.4.B.3.C deferred work; not architecturally connected to regional archetypes. Standalone follow-up.

### 8.3 F.4.B.3.D closure session output

The closure session that runs after the campaign-design session should:
1. Mark F.4.B.3.D as **CLOSED VIA PIVOT** (not COMPLETE, not PARTIAL) in §9, with reference to the new campaign.
2. F.4.B.3.G inheritances and parent campaign (`TERRAIN_MATERIAL_SYSTEM_CAMPAIGN.md`) Phase 1 / Phase 1.5 re-mark COMPLETE per F.5's reduced closeout scope.
3. Enumerate the §8.1 vs §8.2 split above explicitly.

---

## 9. Risk register

### 9.1 Implementation risks

- **Iterative spline tuning may not converge.** Per-archetype splines for 5-10 archetypes is a wide tuning surface. Mitigation: F.7's diagnostic-3 re-measurement gives objective signal; bound iteration count per archetype.
- **Painting UX may be clumsy.** First-pass UI may require unrealistic writer effort. Mitigation: F.5 prototypes the UX with a small Veilweaver-realistic test world; iterate before committing.
- **Performance regression.** Each new layer (mask sampling, archetype blending, spline evaluation) adds per-vertex cost. Mitigation: measure at F.4 close; budget ≤ +30% over D.5-fix baseline (would put per-chunk time at ~1.0s/chunk at radius 10).
- **D.1-D.5 invariants regression.** Existing per-biome amplitudes and blending must continue working within each archetype. Mitigation: F.1 regression test against D.5-fix Path B output.

### 9.2 Architectural risks

- **Mask resolution choice.** Too coarse → boundary aliasing. Too fine → memory cost (115 km² × N pixels/m). Mitigation: F.4 prototypes 2-3 resolutions; pick based on quality + memory.
- **Archetype count drift.** Catalog of 6 may grow to 10-15 in writer iteration. Mitigation: data-driven catalog; no hardcoded archetype count assumptions.
- **Cross-archetype Pearson target may be unachievable for some pairs.** Adjacent archetypes (CT vs Mediterranean) inherently share more shape character than distant (CT vs Desert). Pearson < 0.7 may be too aggressive for adjacent pairs. Mitigation: F.7 sets per-pair Pearson targets; adjacent pairs target < 0.85, distant pairs target < 0.7.

### 9.3 Process risks

- **Campaign drift / scope creep.** F.4.B.3.D ran 6+ sub-phases; new campaign sketches 8. Risk of re-reframe mid-campaign. Mitigation: front-load research (this audit) so architectural commitment is solid before F.0; lock §0 anti-drift discipline tightly.
- **Andrew-gate regress at F.7.** Per-archetype tuning may not deliver visible regional variation despite Pearson drop. Mitigation: diagnostic-3-equivalent measurement is objective; visual verification is its complement.

---

## 10. Sources

### 10.1 Crimson Desert / Pearl Abyss (BlackSpace engine)

- [Crimson Desert official site (Pearl Abyss)](https://crimsondesert.pearlabyss.com/)
- [Pearl Abyss Dev Archives — BlackSpace Engine](https://crimsondesert.pearlabyss.com/en-us/News/Notice/Detail?_boardNo=40) — official, BlackSpace engine
- [TRUNRD — Crimson Desert and BlackSpace at GDC 2025](https://trunrd.com/crimson-desert-and-the-blackspace-engine-at-gdc/) — March 2025
- [8Bit/Digi — GDC 2025 BlackSpace inside look](https://8bitdigi.com/gdc-2025-an-inside-look-into-the-blackspace-engine-with-crimson-desert/) — March 2025
- [MMORPG.com — GDC 2025 under-the-hood hands-on](https://www.mmorpg.com/features/gdc-2025-we-saw-under-the-hood-of-crimson-deserts-engine-here-are-some-of-our-takeaways-2000134455) — March 2025
- [GamesBeat / VentureBeat — BlackSpace graphics overview](https://venturebeat.com/games/pearl-abyss-unveils-graphics-power-of-the-blackspace-engine-for-crimson-desert/)
- [GameGPU — Tracing the Path: BlackSpace Engine Technical Analysis](https://en.gamegpu.com/test-gpu/action-fps-tps/trassirovka-puti-v-crimson-desert-tekhnicheskij-razbor-blackspace-engine) — 2026
- [PCOptimizedSettings — Voxel SDF / PSO Dive ray tracing analysis](https://pcoptimizedsettings.com/crimson-desert-ray-tracing-analysis-ue5-lumen-inspired-lighting-with-voxel-sdfs-pso-dive/) — 2026
- [GamingBolt — PC Graphics Analysis](https://gamingbolt.com/crimson-desert-pc-graphics-analysis-an-ambitious-open-world-tech-showcase) — 2026
- [GamerBraves — Developer Q&A weather system](https://www.gamerbraves.com/crimson-desert-developer-qa-moving-forward/)
- [MMOBomb — Pearl Abyss engine interview (Kwaghyeon Go, 2021)](https://www.mmobomb.com/news/pearl-abyss-talks-about-new-proprietary-engine-behind-crimson-desert-dokev)
- [Inven Global — BDO graphic remaster lead programmer interview (2018)](https://www.invenglobal.com/articles/6046/interview-with-the-lead-engine-programmer-in-pearl-abyss-on-black-desert-online-graphic-remaster)
- [UNIMY — BlackSpace vs UE5 analysis](https://www.unimy.edu.my/news/crimson-deserts-blackspace-engine-vs-unreal-engine-5-the-real-lesson-for-game-dev-students/)
- [GameSpot — Crimson Desert Realistic Physics](https://www.gamespot.com/articles/crimson-desert-might-have-the-most-realistic-in-game-physics-ive-ever-seen/1100-6530297/)
- [AllThings.How — Pywel five regions](https://allthings.how/crimson-deserts-continent-of-pywel-map-size-five-regions-and-how-travel-works/) — 2025-26
- [Beebom — Crimson Desert map regions](https://beebom.com/crimson-desert-map/) — 2026
- [G FUEL — Crimson Desert Map Guide](https://gfuel.com/blogs/news/crimson-desert-map-full-region-map-cities-and-more)
- [FandomWire — How Big is Crimson Desert's Map?](https://fandomwire.com/how-big-is-crimson-deserts-map-size-and-regions-explained/)
- [Wikipedia — Crimson Desert](https://en.wikipedia.org/wiki/Crimson_Desert)
- [Pearl Abyss — Wikipedia](https://en.wikipedia.org/wiki/Pearl_Abyss)
- [Black Desert Online — Wikipedia](https://en.wikipedia.org/wiki/Black_Desert_Online)
- [GitHub — NattKh/CrimsonDesertModdingTools (434-table PABGB schema)](https://github.com/NattKh/CrimsonDesertModdingTools) — 2026
- [GitHub — lazorr410/crimson-desert-unpacker (PAZ/PAMT format)](https://github.com/lazorr410/crimson-desert-unpacker) — 2026
- [GitHub — faisalkindi/CrimsonDesert-UltimateModsManager](https://github.com/faisalkindi/CrimsonDesert-UltimateModsManager)
- [Nexus Mods — VAXIS Ground LOD mod (visual evidence of LOD pop-in)](https://www.nexusmods.com/crimsondesert/mods/733) — 2026
- [YouTube — Dev Archives: The Engine Behind Crimson Desert (GDC 2025)](https://www.youtube.com/watch?v=WEBAgTozBEU)
- [YouTube — Crimson Desert 4K BlackSpace Engine Tech Demo](https://www.youtube.com/watch?v=w92P1zdSNUg)

### 10.2 Enshrouded / Keen Games (Holistic engine)

- [Enshrouded official site](https://www.enshrouded.com/)
- [Enshrouded World Design AMA Recap (Dec 2023, official)](https://enshrouded.com/news/ama-2-recap) — primary architectural reference
- [Bleedingcool — Enshrouded World Building Dev Video (Dec 2023)](https://bleedingcool.com/games/enshrouded-releases-new-world-building-developer-video/)
- [Gameoneer — Enshrouded Dev World Showcase (Dec 2023)](https://gameoneer.com/enshrouded-developers-showcase-how-they-developed-a-detailed-world/)
- [MMORPG.com — Handcrafted + Algorithms article](https://www.mmorpg.com/news/enshrouded-showcases-how-a-blend-of-handcrafted-and-support-from-algorithms-make-the-world-more-interesting-2000129766)
- [Enshrouded Wiki — Embervale](https://enshrouded.wiki.gg/wiki/Embervale)
- [Enshrouded Wiki — Map](https://enshrouded.wiki.gg/wiki/Map)
- [Enshrouded Wiki — Shroud mechanics](https://enshrouded.wiki.gg/wiki/Shroud)
- [4netplayers biome guide (spatial layout)](https://www.4netplayers.com/en/blog/enshrouded/enshrouded-biome-guide-embervale/)
- [Gameskinny — World size article](https://www.gameskinny.com/tips/how-big-is-the-enshrouded-map-size-world-size-detailed/)
- [GTXGaming — Holistic engine voxel system overview](https://www.gtxgaming.co.uk/building-new-worlds-exploring-enshroudeds-voxel-based-system/?lang=en-us)
- [Foro3d — Technical Analysis of Holistic Engine](https://foro3d.com/en/2026/march/technical-analysis-of-the-holistic-engine-from-enshrouded.html)
- [Graphics Programming Conference — 2025 archive (Keen Games talks)](https://graphicsprogrammingconference.com/archive/2025/)
- [GPC 2024 archive (Vulkan, fog, GI talks)](https://graphicsprogrammingconference.com/archive/2024/)
- [GPC 2025 — "The Fog is Lifting" slides PDF](https://static.graphicsprogrammingconference.com/public/2025/slides/the-fog-is-lifting/Krause-the-fog-is-lifting-volumetric-rendering-enshrouded.pdf)
- [GPC 2025 — Water simulation slides PDF](https://static.graphicsprogrammingconference.com/public/2025/slides/water-simulation-and-rendering-in-enshrouded/Mantler-Koenen-water-simulation-rendering-in-enshrouded.pdf)
- [GPC 2025 — GPU particle system slides PDF](https://static.graphicsprogrammingconference.com/public/2025/slides/lessons-learned-from-shipping-a-gpu-particle-system/Feller-lessons-learned-from-shipping-a-gpu-particle-system.pdf)
- [GPC 2024 — Volumetric fog YouTube](https://www.youtube.com/watch?v=OR8HbFnQdlk)
- [GPC 2024 — Dynamic GI YouTube](https://www.youtube.com/watch?v=57F1ezwH7Mk)
- [GPC 2024 — Vulkan YouTube](https://www.youtube.com/watch?v=2LLXC9xCST4)
- [Back to the Shroud patch notes (visual rework)](https://www.enshrouded.com/en-US/news/enshrouded-back-to-the-shroud-update)

### 10.3 Minecraft 1.18+ (MultiNoise / noise router)

- [Minecraft Wiki — Noise router](https://minecraft.wiki/w/Noise_router) — authoritative, current
- [Minecraft Wiki — World generation](https://minecraft.wiki/w/World_generation) — primary technical reference
- [Minecraft Wiki — Noise settings](https://minecraft.wiki/w/Noise_settings)
- [Minecraft Wiki — Custom world generation / noise settings](https://minecraft.fandom.com/wiki/Custom_world_generation/noise_settings)
- [Minecraft Wiki — Biome](https://minecraft.wiki/w/Biome)
- [MultiNoiseUtil.NoiseHypercube — Fabric Yarn 1.19.2 API](https://maven.fabricmc.net/docs/yarn-1.19.2+build.15/net/minecraft/world/biome/source/util/MultiNoiseUtil.NoiseHypercube.html)
- [misode.github.io — Noise Settings Generator](https://misode.github.io/worldgen/noise-settings/)
- [Alan Zucconi — The World Generation of Minecraft (June 2022)](https://www.alanzucconi.com/2022/06/05/minecraft-world-generation/)
- [Dawnosaur Substack — How Minecraft Generates Worlds You Want to Explore](https://dawnosaur.substack.com/p/how-minecraft-generates-worlds-you)
- [Henrik Kniberg — Reinventing Minecraft World Generation (YouTube)](https://www.youtube.com/watch?v=ob3VwY4JyzE) — primary Mojang dev talk
- [cybrancee.com — How Minecraft Terrain Generation Works](https://cybrancee.com/blog/how-minecraft-terrain-generation-works/)
- [FlauschBert Devlog — Minecraftian Voxel Engine](https://flauschbert.itch.io/minecraftian-voxel-engine-demo/devlog/549859/minecraftian-voxel-engine-1-terrain-height-generation)
- [jacobsjo/MinecraftMultiNoiseVisualization (GitHub)](https://github.com/jacobsjo/MinecraftMultiNoiseVisualization)

### 10.4 No Man's Sky / Hello Games

- [GDC Vault — Continuous World Generation in No Man's Sky (McKendrick 2017)](https://www.gdcvault.com/play/1024265/Continuous-World-Generation-in-No)
- [GDC Vault — Building Worlds Using Math(s) (Murray 2017)](https://www.gdcvault.com/play/1024514/Building-Worlds-Using)
- [YouTube — Continuous World Generation in No Man's Sky (McKendrick GDC 2017)](https://www.youtube.com/watch?v=sCRzxEEcO2Y)
- [YouTube — Building Worlds in No Man's Sky Using Math(s) (Murray GDC 2017)](https://www.youtube.com/watch?v=C9RyEiEzMiU)
- [Game Developer — Video: How continuous world generation works in No Man's Sky](https://www.gamedeveloper.com/programming/video-how-continuous-world-generation-works-in-i-no-man-s-sky-i-)
- [Game Developer — Video: Building the Worlds of No Man's Sky Using Math(s)](https://www.gamedeveloper.com/design/video-building-the-worlds-of-i-no-man-s-sky-i-using-math-s-)
- [NMS Modding Wiki — Terrain Generation](https://nmsmodding.fandom.com/wiki/Terrain_Generation)
- [NMS Archived Wiki — Procedural Generation](https://nomanssky-archive.fandom.com/wiki/Procedural_generation)
- [NMS Fandom — Terrain Archetype](https://nomanssky.fandom.com/wiki/Terrain_Archetype)
- [NMS Fandom — Biome](https://nomanssky.fandom.com/wiki/Biome)
- [Procedural Generation blog — NMS GDC summary](https://procedural-generation.isaackarth.com/2017/03/20/continuous-world-generation-in-no-mans-sky-gdc.html)
- [Rambus — The Algorithms of No Man's Sky](https://www.rambus.com/blogs/the-algorithms-of-no-mans-sky-2/)
- [Ithy — Innovations in Procedural Noise for Terrain Generation](https://ithy.com/article/innovations-procedural-noise-terrain-35cvalyh)

### 10.5 Other AAA references

- [GDC Vault 2014 — Marcin Gollent, "Landscape Creation and Rendering in REDengine 3"](https://www.gdcvault.com/play/1020197/Landscape-Creation-and-Rendering-in)
- [GDC 2014 PDF — Landscape Creation and Rendering in REDengine 3](https://ubm-twvideo01.s3.amazonaws.com/o1/vault/GDC2014/Presentations/Gollent_Marcin_Landscape_Creation_and.pdf)
- [80.lv — The Procedural Nature of Horizon Zero Dawn](https://80.lv/articles/the-procedural-nature-of-the-horizon-zero-dawn)
- [Decima (game engine) — Wikipedia](https://en.wikipedia.org/wiki/Decima_(game_engine))
- [Skyrim:Worldspaces — UESP Wiki](https://en.uesp.net/wiki/Skyrim:Worldspaces)
- [Creation Kit — Creating a Custom Worldspace with LOD](https://ck.uesp.net/wiki/Creating_a_Custom_Worldspace_with_LOD)
- [allthings.how — How Hytale's Procedurally Generated World Works](https://allthings.how/how-hytales-procedurally-generated-world-actually-works/)

### 10.6 Algorithmic / pattern references

- [Minecraft Wiki — Noise router](https://minecraft.wiki/w/Noise_router)
- [Minecraft Wiki — World generation](https://minecraft.wiki/w/World_generation)
- [Minecraft Wiki — Custom world generation / noise settings](https://minecraft.fandom.com/wiki/Custom_world_generation/noise_settings)
- [misode.github.io — Noise Settings Generator](https://misode.github.io/worldgen/noise-settings/)
- [Red Blob Games — Making maps with noise (Amit Patel)](https://www.redblobgames.com/maps/terrain-from-noise/)
- [Red Blob Games — Polygonal Map Generation for Games](http://www-cs-students.stanford.edu/~amitp/game-programming/polygon-map-generation/)
- [NoisePosti.ng — Fast Biome Blending, Without Squareness](https://noiseposti.ng/posts/2021-03-13-Fast-Biome-Blending-Without-Squareness.html)
- [Game Genius Lab — Voronoi Diagrams in Game Development](https://www.gamegeniuslab.com/tutorial-post/voronoi-diagrams-in-game-development-procedural-maps-ai-territories-stylish-effects/)
- [Wayline — Heightmaps and Voronoi Diagrams: Revolutionizing Game World Generation](https://www.wayline.io/blog/heightmaps-voronoi-diagrams-game-world-generation)
- [Springer — AutoBiomes: procedural generation of multi-biome landscapes](https://link.springer.com/article/10.1007/s00371-020-01920-7)
- [ResearchGate — Parameterized and dynamic generation of an infinite virtual terrain with various biomes using extended voronoi diagram](https://www.researchgate.net/publication/307588539_Parameterized_and_dynamic_generation_of_an_infinite_virtual_terrain_with_various_biomes_using_extended_voronoi_diagram)
- [Daydreamsoft — Fractal-based terrain generation for infinite planetary worlds](https://www.daydreamsoft.com/blog/fractal-based-terrain-generation-for-infinite-planetary-worlds)

### 10.7 Authoring tool references

- [Unreal Documentation — Editing Landscape Terrain](https://docs.unrealengine.com/udk/Three/LandscapeEditing.html)
- [Godot — TerraBrush: A Terrain Editing Add-on for Godot](https://jettelly.com/blog/terrabrush-a-terrain-editing-add-on-for-godot-4-5)
- [QuadSpinner Gaea — Houdini integration](https://quadspinner.com/Gaea/Houdini)
- [QuadSpinner Gaea Documentation — Houdini](https://docs.quadspinner.com/Guide/Bridges/Houdini.html)
- [O3DE — Create Terrain Assets](https://docs.o3de.org/docs/learning-guide/tutorials/environments/create-terrain-from-images/create-terrain-assets/)
- [World Machine — Macro Library](https://www.world-machine.com/library/index.php?self=)

### 10.8 AstraWeave internal references

- `docs/audits/f4b3d5_diagnostic_3_cross_archetype_2026-04-28.md` (the architectural gap data; this audit's predecessor).
- `docs/audits/f4b3d5_diagnostic_2_real_heightmap_2026-04-28.md` (real-chunk biome distribution).
- `docs/current/TERRAIN_GENERATION_QUALITY_CAMPAIGN.md` (D.1-D.5 landed work + F.4.B.3.D.5-fix).
- `docs/audits/terrain_scale_diagnostic_2026-04-24.md` (Enshrouded comparison baseline; Target B selection).
- `docs/audits/uber_noise_research_2026-04-25.md` (F.4.B.3.A research; Murray-direct features ranked).
- `docs/audits/nms_streaming_architecture_summary_2026-04-24.md` (NMS streaming research; partially relevant).
- `docs/supplemental/WORLD_SCALE_CONVENTIONS.md` (1 WU = 1 m, Target B 115 km²).
- `astraweave-terrain/src/climate.rs` (D.1 climate field).
- `astraweave-terrain/src/biome_lookup.rs` (D.2 Whittaker classification).
- `astraweave-terrain/src/biome_parameters.rs` (D.3 per-biome parameters).
- `astraweave-terrain/src/biome_param_blending.rs` (D.4 scattered-convolution blending).
- `astraweave-terrain/src/world_archetypes.rs` (D.5 archetype catalog).

---

## 11. One-paragraph TL;DR

Veilweaver wants regional archetype variation with smooth blending across 5-10 archetype zones per world, one archetype per Tikva storyline region. AAA prior art (Crimson Desert at 80-110 km², Enshrouded explicitly "handcrafted by the developers," Witcher 3 / Horizon / Skyrim all hand-authored macro layout) overwhelmingly supports a hand-authored macro + procedural detail filling pattern. Minecraft 1.18+'s noise router is the canonical published architecture for climate-driven shape variation via spline-based density functions. The recommended approach for AstraWeave is **Hybrid C + F**: paintable archetype mask (Approach C — honors Veilweaver writer intent) + climate-driven shape splines per archetype (Approach F — Minecraft-pattern, integrates with D.1-D.5 architecture). The new campaign sketches 8 sub-phases (F.0-F.8); estimated 8-12 sessions including diagnostic+remediation pairs. The data-driven success criterion: cross-archetype Pearson drops from current 0.978-0.989 to target < 0.7 (distant pairs) / < 0.85 (adjacent pairs). The architectural commitment is real but bounded; D.1-D.5 work is preserved as the within-region machinery, with the new campaign adding the upstream regional layer.
