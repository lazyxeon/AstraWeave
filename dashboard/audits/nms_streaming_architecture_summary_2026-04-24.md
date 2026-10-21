# NMS Streaming Architecture Summary — 2026-04-24

## Scope

Phase 1.6-F.4.B.3.A spin-off artifact. The Innes McKendrick GDC 2017 talk "Continuous World Generation in 'No Man's Sky'" is mostly Phase 1.7 (Streaming Terrain) territory rather than F.4.B.3 (Uber Noise transforms). This document captures the McKendrick summary verbatim for future Phase 1.7 reference, kept separate from the F.4.B.3 research document so that file stays focused on noise-pipeline transforms.

## Source

- YouTube: https://www.youtube.com/watch?v=sCRzxEEcO2Y
- GDC Vault (members-only): https://www.gdcvault.com/play/1024265/Continuous-World-Generation-in-No

**Provenance:** AI-generated (Gemini) summary based on auto-captions and visual content of the talk. Treat as primary-source proxy, not verbatim transcript.

## Summary (verbatim from F.4.B.3.A prompt)

Key Technical Pillars from McKendrick:

**World Structuring & Spherical Geometry (10:35 - 21:40):** Implicit sphere approach, surface curvature in shaders, seamless continuous coordinate system.

**Voxel-Based Generation (21:45 - 27:20):** 32×32×32 meter voxel regions generated, polygonized, streamed in real-time. Octrees for LOD/memory.

**Generation Pipeline & Parallelism (27:24 - 31:05):** Job system to offload heavy calculations from main thread. Noise-based terrain → polygonization → physics mesh → foliage and structures.

**Top-Down Generation Strategy (31:36 - 32:30):** Hierarchical approach where positional seeds are fed into successive generators. Solar system → planet characteristics (mountain density, cliff frequency) → local terrain data. Modularity allows debugging specific stages.

**2D Terrain Blocking (35:28 - 36:55):** Before creating 3D volumes, generate 2D height map data for the planet's surface. Defines broad, low-resolution shape — where hills, plains, rivers will exist. Stored as 2D arrays for performance.

**3D Voxel Generation (37:23 - 39:40):** 32×32×32 meter voxel regions populated using layered noise (Perlin, Simplex). Density controlled by height — tapers off as distance increases from surface — preventing floating artifacts. Allows complex features like caves and overhangs.

**Polygonization (39:56 - 41:15):** Density data → mesh via Dual Contouring. Marching Cubes was tried first but couldn't preserve sharp corners. Dual Contouring retains corner data, producing sharper terrain. Mesh projected onto sphere via cube-based grid.

**Stateless Generation:** "Individual voxels are stateless and do not know about their neighbors." Engine models geological features like erosion as an "end state" rather than running simulations. This is Murray's analytical-derivative approach in service of stateless generation.

**Mountain Heights:** 600m to over a kilometer (20:50 - 21:18) achieved by adding massive elevation offset to base sphere radius using noise-based functions over the planet's surface.

**Subtractive Modeling (39:19 - 39:40):** "Perlin worms" or "open spheres" carve into generated mass for caves and overhangs.

**Layered Noise Functions (37:23 - 38:00):** Perlin, Simplex, Voronoi/cellular. Combined with positional turbulence for natural complex forms (mountain pinching, cliff faces).

**Density-by-Height Reduction (38:02 - 38:20):** Prevents "blobby" floating islands. Noise field's density tapers off with distance from planet surface.

**Dithered Fade-in (50:24 - 51:23):** Critical visual optimization. Game objects fade in as they enter the player's view, not pop-in. Used for terrain, creatures, plants, LOD transitions.

**Object Placement (47:34 - 47:50):** Offset grid system + recursive object placement (Kate Compton approach: scale down from large trees to smaller bushes to pebbles).

**Triplanar Texturing (43:47 - 44:15):** Materials blended based on height maps and environmental parameters across the entire planet, no local map data required.

**"Simon's Smoke Test" (53:09 - 53:44):** Automated screenshot capture and data collection on every build. Tracks performance degradation over time across diverse procedurally-generated worlds.

**Data-Local Architecture (7:56 - 8:42):** Engine does NOT rely on global maps. Simulation is "blind" to the larger world — only knows immediate surroundings. Memory constraints drive this; planet is too large to load fully.

## Relevance for AstraWeave

### Phase 1.7 Streaming Terrain campaign (future)

McKendrick's content maps directly to a hypothetical Phase 1.7:
- 32³ voxel regions / dual contouring → AstraWeave's terrain currently uses 96² heightmap chunks; Phase 1.7 might shift to voxel for caves/overhangs.
- Octree LOD for streaming → AstraWeave currently generates all chunks at editor time; streaming requires LOD + radius management.
- Job-system pipeline parallelism → AstraWeave's F.4.B.2.D rayon parallelization is single-stage; Phase 1.7 would need multi-stage (terrain → mesh → physics → scatter as overlapping jobs).
- Subtractive Perlin worms → caves, currently absent.
- Density-by-height tapering → relevant if AstraWeave moves to volumetric.
- Triplanar texturing across entire planet → AstraWeave already has triplanar via splat shader.

### F.4.B.3 (current campaign) — relevant snippets only

Two parts of McKendrick are directly relevant to F.4.B.3:

1. **Hierarchical generation strategy (31:36-32:30):** "Solar system → planet characteristics (mountain density, cliff frequency) → local terrain data." This frames the multi-scale locality concept that F.4.B.3.D implements. For AstraWeave (single-world game, no solar system), the analog is "world parameters → regional parameters → chunk parameters."

2. **Stateless generation as composition strategy:** McKendrick's "individual voxels are stateless and do not know about their neighbors; the engine models geological features like erosion as an 'end state'." AstraWeave's particle erosion (F.3) is the OPPOSITE of stateless — droplets carry water/sediment across cells. This is why phases 3 + 4 had so much trouble with chunk boundaries. NMS's noise-side approach (Murray's slope-conditional cragginess; runevision's gradient-based gully filter) treats erosion as something that *appears* eroded via noise transforms, not by simulating physics. **AstraWeave can have both** — F.4.B.3 lands the noise-side; particle erosion remains as post-pass.

## Caveat about provenance

This summary is Gemini-generated based on YouTube auto-captions and visual content. Specific timestamps, parameter values, and quoted phrasing should be verified against the actual talk before any Phase 1.7 implementation work commits to a specific design choice. The summary is good enough for orientation but not for citation.
