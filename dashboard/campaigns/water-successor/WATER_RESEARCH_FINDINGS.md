# Water Successor — Research Findings (Deep-Dive Record)

**Campaign:** W-series (Water Successor)
**Date:** 2026-06-22 · **Branch:** `campaign/water-successor`
**Purpose:** durable forward-reference record of the water-rendering research deep-dive.
**Status:** findings record — primary sources named, not a re-derivation.

This document exists so the research does not live only in conversation. It is a
*reference*, not a construction authority: ratified decisions derived from it live
in [`W2_DECISIONS.md`](./W2_DECISIONS.md) §F. Where a finding feeds a specific
phase, the phase is named so a future session can pull the right technique without
re-running the search.

---

## 0. Headline

The committed W-series techniques are the **correct workhorse choices** and the
research does **not** overturn them:

- **Gerstner** surface math (4-wave, steepness-capped) — confirmed.
- **Chunked-LOD** camera-following surface — confirmed.
- **Screen-space scene-color refraction** (post-opaque pass, single copy) — confirmed.
- **Depth-delta foam** (Profile C) — confirmed.

The research **sharpens W.2c** (weave-response) and **resolves the FFT fork**
(closed as Gerstner, see §6 and `W2_DECISIONS.md` §F) rather than revising any
shipped W.2a / W.2b work. The strategic payload is §1: the authored-deformation
model for weave-response.

---

## 1. The Horizon Forbidden West model — PRIMARY finding for W.2c

**Weave-response (part / freeze / raise) should be _authored deformation data_ —
baked, instanced, and replayed view-side — not runtime simulation.**

Guerrilla Games' breaking-wave system for *Horizon Forbidden West* is the AAA-tier
proof of the paradigm the W-series already chose (view-side, art-directed, no
gameplay-coupled solver):

- Localized wave deformations are **authored/simulated offline in Houdini**, then
  **baked out**.
- A wave cross-section is stored as a **deformation texture**: per-texel **XYZ
  offset encoded to RGB**, replayed on the surface mesh at runtime.
- The explicit motivation was **art-directability and gameplay-driven changes to
  wave parameters** — exactly the part/freeze/raise authoring surface W.2c needs.

This is the model for W.2c: the weave-response vocabulary is a set of **authored,
baked deformation profiles** keyed to in-world events, replayed on the existing
chunked-LOD surface — not a simulation the engine steps each tick.

> **Primary source:** *Horizon Forbidden West* water rendering talk, **SIGGRAPH
> 2022** (Guerrilla Games; presenter Malan). Breaking-wave deformation: Houdini
> bake → XYZ-to-RGB deformation texture, authored in-editor.

---

## 2. Water Surface Wavelets (Jeschke et al. 2018) — boundary-aware, solver-free for static boundaries

Bridges the gap between "Fourier/Gerstner can't interact with boundaries" and
"finite-difference solvers are too expensive for min-spec."

- Boundary-aware surface waves that **reflect off obstacles** (shorelines, rocks).
- **Key exploit for us:** for **static** boundaries, the steady-state can be
  **pre-baked and interpolated** at runtime — reflecting waves with **no runtime
  solver**. Fits the W-series constraints exactly: view-side only, min-spec,
  bandwidth-limited.

This is the technique to reach for **if W.2c shoreline behavior wants boundary-aware
reflection** beyond simple depth-driven foam. Pre-bake the steady-state per static
boundary, interpolate — no per-frame solve.

> **Primary source:** Jeschke, Skřivan, Müller-Fischer, Chentanez, Macklin, Wojtan,
> "**Water Surface Wavelets**," *ACM TOG* (SIGGRAPH) **2018**.
> **Follow-up:** "**Making Procedural Water Waves Boundary-aware**," Jeschke et al.,
> **2020** (extends the boundary handling).

---

## 3. Wave Particles (Yuksel 2007) + the Nov 2025 hybrid — the local-interaction reference

The reference technique for **local** water interaction in constrained areas:

- **Wave Particles** (Yuksel, House, Keyser): unconditionally **stable**, GPU
  height-field, supports **boundary reflections** in bounded regions. The canonical
  way to do localized disturbances without a stability-limited solver.
- **Current frontier (Nov 2025):** couple a **global field** with **wave particles
  for local disturbances** — a global base layer + a local interaction layer.

Mapping to our architecture: the **weave-response is the local layer** in that
two-layer model (global Gerstner field + local authored/particle disturbance). If
weave-response ever needs *runtime-reactive* local waves rather than purely baked
profiles, wave particles are the stable mechanism — but §1's baked model is the
default for W.2c.

> **Primary source:** Yuksel, House, Keyser, "**Wave Particles**," *ACM TOG*
> (SIGGRAPH) **2007**. Hybrid global-field + wave-particle coupling: **2025**
> frontier work.

---

## 4. Tiled Directional Flow (van Hoesel) — for the logged river candidate (Profile B)

For the **B-profile river** work logged in `W2_DECISIONS.md` §E:

- **Tiled Directional Flow** **rotates the texture so it carries waves**, not
  directionless noise — visibly better than Valve-style flow maps for **non-turbulent
  flowing water** (rivers, streams).
- Adopt it **for the B-profile river work when that is scoped** (over plain
  Valve/Portal-2 flow-map advection).

> **Primary source:** Frans van Hoesel, "**Tiled Directional Flow**" (SIGGRAPH
> poster / GPU Pro lineage). Contrast: Valve flow maps (*Portal 2* water).

---

## 5. Photon pressure-only shallow water — the cheap runtime-interactive fallback

If runtime **interactive ripples** are ever genuinely required (baked deformations
proving insufficient):

- Photon's approach solves **only the pressure function** of the shallow-water
  equations, using **two depth render targets** — **mobile-capable**, the cheap
  runtime-interactive option.
- **Notably they rejected Lattice Boltzmann as too memory-heavy for runtime** —
  the **same conclusion the F-series reached about voxel water** (the deprecation
  the W-series acts on). Independent corroboration that grid/voxel-class runtime
  fluid is the wrong cost class for this target.

This is a **fallback**, not a plan: the §1 baked-deformation model is the default.
Recorded so the option is on the shelf if W.2c authored profiles ever fall short.

> **Primary source:** Photon water-rendering writeup (pressure-only shallow-water,
> dual depth-target, mobile). LBM-rejected-for-runtime note corroborates the
> F→W voxel deprecation.

---

## 6. FFT vs Gerstner — resolved as Gerstner (reasoning recorded so the fork stays closed)

The W.2.0 ratification (`W2_DECISIONS.md` §C) chose **Gerstner-first** and held FFT
as a possible later drop-in. The research **closes** that fork rather than leaving
it deferred. Reasoning (full version in `W2_DECISIONS.md` §F):

- FFT's **O(log N)** scaling only wins at **open-ocean scale we do not have**.
- FFT's **fixed base cost** (spectrum update ≈ a full Gerstner displacement map)
  **penalizes a bandwidth-limited card** (the min-spec 1660 Ti Max-Q).
- **Gerstner's linear-in-wave-count cost is precisely dial-able** against the 2.0 ms
  budget — the budget instrument already measures it.
- **Decisive:** FFT would force maintaining **two water systems** (FFT for capable
  hardware + a min-spec fallback) — a production cost a solo dev **developing on
  min-spec** cannot justify.

**Fork closed: Gerstner.** See `W2_DECISIONS.md` §F.

---

## 7. Effects phase (deferred) — aesthetic-driven, normal-map-forward

For the deferred effects/set-piece layer (`W2_DECISIONS.md` §E "DEFERRED"):

- **Caustics:** **aesthetic-driven** — leave physical realism out; look good
  cheaply. (Pairs with the §E-deferred caustics/foam/god-rays layer.)
- **Normal-map manipulation** buys **apparent detail for distant water at near-zero
  vertex cost** — reflection and refraction are functions of **view + normal**, so
  perturbing the normal alone adds perceived surface complexity without geometry.
  The lever for far-field water richness once the effects phase opens.

---

## Source index

| # | Technique | Primary source |
|---|---|---|
| 1 | Authored/baked breaking-wave deformation | Horizon Forbidden West water talk, SIGGRAPH 2022 (Guerrilla / Malan) |
| 2 | Boundary-aware solver-free (static) | Water Surface Wavelets, Jeschke et al., TOG 2018; Boundary-aware follow-up 2020 |
| 3 | Stable local interaction | Wave Particles, Yuksel et al., TOG 2007; global+particle hybrid, 2025 |
| 4 | Flowing-water texturing | Tiled Directional Flow, van Hoesel |
| 5 | Cheap runtime ripples | Photon pressure-only shallow water (dual depth-target, mobile) |
| 6 | FFT vs Gerstner | resolved as Gerstner — see `W2_DECISIONS.md` §F |
| 7 | Effects-phase detail | aesthetic caustics + normal-map perturbation (deferred layer) |

*Findings record only. Construction decisions derived from this live in
`W2_DECISIONS.md` §F and are the authority W.2c executes against.*
