# W.2.0 — Surface-Layer Recon + Water-Doc Deprecation Audit (Recon of Record)

**Campaign:** W-series (Water Successor) · **Phase:** W.2.0
**Branch:** `campaign/water-successor` · **HEAD:** `1a57fdd41` (W.1 removal; `volume_grid.rs`/`voxel.rs` confirmed gone)
**Mode:** READ-ONLY reconnaissance · **Mutations:** zero (no source written, no git ops, no cargo beyond read-only inspection)

This is the immutable recon of record persisted at W.2 Phase 1. All capability
claims are verified against current source; "absent" claims state the inspection
that found nothing. The two stale-doc anchors were read directly, not trusted.
Findings are transcribed verbatim from the W.2.0 gate report — not re-derived.

---

## Deliverable 1 — Current surface-layer capability map

`astraweave-render/src/water.rs` (369 LoC) + `src/shaders/water.wgsl` (223 LoC). W.0 findings **all confirmed**, with two corrections/additions:

| Capability | Status | Evidence |
|---|---|---|
| **Single hardcoded plane, no chunk/LOD** | Confirmed | Mesh built once via `Self::generate_water_plane(500.0, 128)` in `new()` (`astraweave-render/src/water.rs:189`); plane centered at origin, **fixed Y=2.0 baked into vertices** (`water.rs:232`). `render()` (`water.rs:303`) just draws the fixed index buffer. Full-file read: no LOD/chunk types, no per-frame regeneration. |
| **`set_water_level` is a stub** | Confirmed (+correction) | `water.rs:271-273` — `_level` unused, body is a comment. **Correction:** the comment claims "controlled by the uniform, already at y=0" but `WaterUniforms` (`water.rs:15-29`) has **no water-level field**, and the mesh Y is 2.0 not 0 — so there is no level plumbing at all. |
| **No refraction / scene-color sampling** | Confirmed | Bind group layout has a **single entry** (uniform buffer) (`water.rs:115-127`); `water.wgsl` has exactly one binding `@group(0)@binding(0)` (`water.wgsl:34`) — no texture/sampler bindings. Fragment uses a **constant** "fake sky reflection" `vec3(0.6,0.75,0.95)` (`water.wgsl:202`), not sampled scene/depth. |
| **Gerstner(4)/Fresnel/depth-color/foam/rain-ripple in shader** | Confirmed present | Gerstner ×4 summed: `water.wgsl:91,149-162`; Fresnel `:195`; depth-color blend `:198-199`; foam `:212-213`; rain-ripple `:40-87` applied `:183-192`; plus sun specular `:206-208`, alpha transparency `:219`. |
| **Production consumers wire it post-W.1** | All confirmed | Veilweaver `examples/veilweaver_demo/src/visual_renderer.rs:625,636`; hello_companion `examples/hello_companion/src/visual_demo.rs:720,725`; editor `tools/aw_editor/src/viewport/engine_adapter.rs:3739,3769,3794`; core renderer `Option<WaterRenderer>` field `astraweave-render/src/renderer.rs:892`, setter `:4536`, `update_water`/`update_player_biome` integration `:3826-3846`; terrain sea-level reference `astraweave-terrain/src/elevation_biome.rs:26`. **None depend on `astraweave-fluids`** — unaffected by W.1. |

**Extra finding (§7.7-adjacent, for D2):** the editor exposes a water-level UI control wired to a stub chain — `tools/aw_editor/src/viewport/widget.rs:2814` → editor `viewport/renderer.rs:1456` (`_level` unused) → render `water.rs:271` stub. A settable-but-unobserved knob; real `set_water_level` (D2) closes it. Also `cull_mode: None // DEBUG: Render both sides` (`water.rs:170`) is a debug artifact left in production.

---

## Deliverable 2 — W.2 extension-scope proposal

| Capability | Classification | Rationale |
|---|---|---|
| **Camera-distance LOD / chunking** | **EXTEND-EXISTING renderer + NET-NEW LOD core** | `WaterRenderer` is the right home and `generate_water_plane` is a reusable mesh primitive (`water.rs:216`), but there is **no LOD/chunk substrate** in it — the LOD scheme itself is net-new. (Terrain's chunked meshing is voxel-specific; not directly reusable.) |
| **Real `set_water_level`** | **EXTEND-EXISTING** | Add a level field to `WaterUniforms` + a Y offset in `vs_main` (or parameterize mesh gen). Small, contained to `water.rs`/`water.wgsl`. Also re-points the dead editor knob above. |
| **Refraction / scene-color sampling (submerged path)** | **EXTEND-EXISTING infra + NET-NEW shader bindings** | The water shader has no texture bindings today (net-new), but the render crate already provides the plumbing to reuse: `depth.rs`, scene-color routing via `frame_graph.rs`, and `ssr.rs` (`SsrPass`). Extend those rather than build a parallel path. |
| **Weave-response hooks (part / freeze / raise)** | **EXTEND-EXISTING (`WaterQuery` facade)** | The bounded-deformation vocabulary registers as backends/methods behind `astraweave-water`'s `WaterQuery` (the §7.7 single owner) — **not** a general solver. Note: `WaterQuery` currently exposes only `sample()`; this adds bounded surface so the three effects stay finite and authored. |

**Open design forks (surfaced at recon; resolved at the gate — see W2_DECISIONS.md):**

1. **Gerstner vs FFT spectral ocean.** Current = Gerstner 4-wave (extend-existing, cheap, art-directable, limited far-field realism). FFT = net-new compute pass, richer/spectrally-correct, heavier, needs a WGSL FFT. Recon recommendation: start by extending Gerstner; keep FFT as a later drop-in behind the same `WaterRenderer`.
2. **LOD scope: chunk grid vs continuous.** Chunk grid = discrete tiles, simpler, mirrors terrain's chunk model, must manage seams/skirts. Continuous (projected-grid / clipmap) = seamless, scales to horizon, more complex. Recon recommendation: chunk grid first.

---

## Deliverable 3 — Water-documentation deprecation list

Primary scope `docs/current/` (glob `*{WATER,FLUID}*` → exactly 3 files), plus the relevant adjacent docs. `docs/architecture/fluids.md` was already revised to **rev 1.6** in W.1 → **KEEP** (current/accurate).

| Doc | Stale direction asserted | Disposition |
|---|---|---|
| **`docs/current/WATER_SYSTEM_ENHANCEMENT_PLAN.md`** (anchor) | Status "PLANNED" (`:5`), built on Enshrouded "Wake of Water" (`:6`). Claims **"500k+ particles via PBD"** as a strength (`:17`) — F.2 measured the real min-spec ceiling at 15–20k. Lists **"Volumetric Water Grid — Voxel-based"** as a gap to build (`:26,:44`) and a 3-way hybrid sim (`:40-70`) — the exact F.3.S-declined / W.1-deleted ambition. Swimming/oxygen/fishing/water-wheels/dispensers/drains (`:27-33,:67-69`). | **DEPRECATE-OUTRIGHT** — pure killed-direction; highest leak risk into W.2. |
| **`docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md`** | "Research-grade" multi-solver SPH roadmap targeting **DFSPH/PCISPH/IISPH** (`:25,:36`), 500k-1M particles (`:30,:42`), multi-phase/non-Newtonian/turbulence (`:18-20,:39,:43`). Every target maps to inventory **deleted in W.1** (`research.rs`, `pcisph_system.rs`, `multi_phase.rs`, `turbulence.rs`, `viscosity.rs`…). | **DEPRECATE-OUTRIGHT** — roadmap to deleted code. (Note: the fluids trace rev 1.6 references it historically as "the audit doc"; on deprecation, soften that reference — later phase.) |
| **`docs/current/FLUIDS_MUTATION_TESTING_REPORT.md`** | Already carries an F.1 **staleness banner** (`:3-13`: "do not cite these numbers as current; re-run queued"). Post-W.1 even more modules it scores are gone (`viscosity.rs`, `particle_shifting.rs`, `pcisph_system.rs` at `:62,:65`). Historical report, not a forward plan. | **REVISE** — extend the existing banner to note the W.1 removals (low priority; it already self-disclaims). |

**Adjacent (flag for the director; not in the `docs/current/` water-doc set):**
- `docs/current/MASTER_COVERAGE_REPORT.md:196` — fluids entry cites "46,173 instrumented lines / 2,509 tests / ~67,800 source lines"; post-W.1 the crate is ~24K src and most of those tests were in removed modules → **REVISE** (master-report numeric refresh per CLAUDE.md maintenance rule).
- `docs/masters/MASTER_BENCHMARK_REPORT.md` — contains voxel-sparsity (F.3.S) benchmarks that now measure deleted code; the F.2 Option-A particle numbers stay relevant for F.4 → **REVISE** (annotate the voxel rows as measuring removed code).
- `docs/audits/water_system_architecture_2026-04-20.md` — pre-F.2 water-architecture audit → **KEEP** (dated historical audit; don't rewrite history).
- `docs/src/api/fluids.md` — aspirational wiki referencing non-existent `FluidWorld`/`FluidConfig` (CLAUDE.md "Documentation Hazards" class) → **REVISE/DEPRECATE** as part of the broader `docs/src/` drift, not a W-specific item.
- `docs/journey/**` (`WATER_SYSTEM_PHASE_4_COMPLETE.md`, `FLUIDS_RESEARCH_GRADE_COMPLETE.md`) and `docs/archive/**` (`PR_HYBRID_VOXEL.md`) → **KEEP** (journey/archive are historical-by-definition).

---

## Gate (as reported at W.2.0)

Recon produced these three deliverables and stopped. No W.2 construction and no doc deprecation begun. Director ratification of (1) extension scope, (2) the Gerstner-vs-FFT and LOD-scope forks, and (3) the water-doc deprecation list is recorded in `W2_DECISIONS.md` (this directory).
