# W.2 — Water Successor: Ratification Record

**Campaign:** W-series (Water Successor)
**Source:** W.2.0 gate (surface-layer recon + water-doc deprecation audit; see `W2_0_RECON.md`)
**Ratified by campaign director:** 2026-06-21
**Branch at ratification:** `campaign/water-successor` · **HEAD:** `1a57fdd41`

These decisions are director-ratified at the W.2.0 gate. They are transcribed
here as the source of truth so they survive outside conversation memory. They are
not to be re-derived, re-opened, or "improved" — only executed against.

---

## A. Sequencing decision

**Surface-first (W.2) before accents (F.4).** Rationale: build *what water is*
before *what it does*; accents garnish a surface that must exist first, so doing
surface-first avoids building the garnish twice.

**Live-campaign state:** the Camera and Terrain Asset Quality campaigns are
**CLOSED** (prior memory indicating otherwise was stale). The **W-series is the
sole live constructive campaign.**

---

## B. Extension scope — ratified as proposed

| Capability | Ratified classification | Notes |
|---|---|---|
| **Real `set_water_level`** | **EXTEND-EXISTING** | `WaterUniforms` field + vertex-shader Y offset. Also **re-points the currently-dead editor water-level knob** (`tools/aw_editor/src/viewport/widget.rs:2814` → viewport stub → `astraweave-render/src/water.rs:271`). |
| **Weave-response hooks (part / freeze / raise)** | **EXTEND-EXISTING behind the `WaterQuery` facade** | Bounded authored vocabulary, **NOT a general solver.** Registers behind `astraweave-water`'s `WaterQuery` (§7.7 single owner). |
| **Camera-distance LOD / chunking** | **EXTEND renderer + NET-NEW LOD core** | Replaces the single hardcoded `generate_water_plane(500.0, 128)`. |
| **Refraction / scene-color sampling** | **EXTEND existing render infra + NET-NEW shader texture bindings** | Reuse `astraweave-render/src/{depth.rs, frame_graph.rs, ssr.rs}`; add the net-new texture/sampler bindings to the water shader. |

---

## C. Design forks — ratified

- **Surface math: Gerstner-first.** Extend the existing 4-wave Gerstner; keep FFT
  spectral ocean as a later drop-in behind the **same** `WaterRenderer` **only if
  the visual bar demands it.**
- **LOD scheme: chunk-grid-first.** Discrete tiles (manage seams/skirts); revisit
  continuous projected-grid/clipmap **only if** open-ocean horizon scenes require it.

---

## D. Production-hygiene items folded into W.2

- **Remove** the `cull_mode: None // DEBUG: Render both sides` artifact in
  `astraweave-render/src/water.rs:170` during the surface work.
- **Close** the settable-but-unobserved editor water-level knob when real
  `set_water_level` lands.

---

## E. Gemini brainstorm triage — outcome of record

A divergent-ideation pass (Gemini 3.5 Flash, "Virtual Fluids Ecosystem" spec) was
evaluated against W-series scope and architectural invariants. Disposition:

**FOLDED INTO W.2:**
1. **Profile A's Gerstner steepness cap `Q < 1.0`** — correctness guardrail against
   normal inversion / mesh tearing; folds into the Gerstner-first surface work.
2. **Profile C's depth-delta intersection foam** — sample scene depth, foam where
   delta → 0; rides on the refraction/scene-color bindings already scoped
   (near-free once they exist).

**LOGGED AS BOUNDED CANDIDATE (W.2/W.3, scoped separately):**
- **Profile B flow-map advection** (two scrolling planes, half-cycle offset,
  cross-faded) for Veilweaver rivers.

**DEFERRED to the later effects/set-piece phase** (alongside the ②-deferred
caustics/foam/god-rays layer), **pure-shader only with all CPU hooks stripped:**
- **Profile D** (springs / whirlpools / methane domes).
- **Profile F** (brine pools, **minus physics**).

**REJECTED — out of paradigm:**
- **Profile E** (Ekman / windrows — open-ocean planetary sim; Veilweaver is
  region-scoped to Chevel).
- **All of section 4** (CPU physics hooks — buoyancy override, toxic damage,
  Coriolis entity drift) — reintroduces gameplay-coupled water simulation the F→W
  deprecation exists to remove. If any such mechanic is ever wanted, it routes
  through gameplay systems and the `WaterQuery` bounded vocabulary, **never a
  renderer spec.**
- **Profile F's prescribed `cull_mode = None`** — directly contradicts the
  debug-artifact removal in §D.

**Note for future ideation:** the Gemini spec recited the deprecation correctly in
its intro then violated it by section 4 (CPU hooks). Useful for divergent technique
generation; **does not hold architectural invariants under pressure — not a scope
authority.**

---

## F. Research integration — ratified amendments

**Source:** water-rendering deep-dive, **2026-06-22** (primary sources and full
findings in [`WATER_RESEARCH_FINDINGS.md`](./WATER_RESEARCH_FINDINGS.md)).
**Disposition:** the committed W-series techniques (Gerstner, chunked-LOD,
scene-color refraction, depth-foam) are confirmed correct; the research **sharpens
W.2c** and **closes the FFT fork** — it overturns nothing already shipped.

### F.1 FFT fork — **CLOSED as Gerstner** (was "deferred" in §C)

§C held FFT as a possible later drop-in "only if the visual bar demands it." That
is now **resolved, not merely deferred — the fork is closed.** Reasoning:

- FFT's **O(log N)** scaling wins only at **open-ocean scale we do not have**.
- FFT's **fixed base cost** — the spectrum update is ≈ a full Gerstner displacement
  map — **penalizes a bandwidth-limited card** (min-spec 1660 Ti Max-Q).
- **Gerstner's linear-in-wave-count cost is precisely dial-able** against the 2.0 ms
  budget (the budget probe already measures it directly).
- **Decisive:** FFT would force maintaining **two water systems** (FFT for capable
  hardware + a min-spec fallback) — a production cost a solo dev **developing on
  min-spec** cannot justify.

**Do not re-open this fork.** Gerstner is the surface math, full stop. (Supersedes
the conditional FFT clause in §C.)

### F.2 W.2c approach — **adopt the Horizon Forbidden West model**

Weave-response (**part / freeze / raise**) = **authored, baked deformation
profiles replayed view-side**, NOT runtime simulation. The AAA proof is Guerrilla's
*Horizon Forbidden West* breaking waves: Houdini-baked localized deformation,
cross-section stored as an **XYZ-offset-to-RGB deformation texture**, authored
in-editor for art-directability and gameplay-driven parameter changes. This stays
inside the §B ratification ("bounded authored vocabulary, NOT a general solver,"
behind the `WaterQuery` facade).

- **If boundary-aware shoreline behavior is wanted:** the **Water Surface Wavelets**
  (Jeschke 2018) **pre-bake-and-interpolate** trick gives reflecting waves off
  static boundaries with **no runtime solver** — the technique to reach for, not a
  finite-difference solve.
- **If runtime-reactive local waves are ever needed:** **Wave Particles** (Yuksel
  2007) are the stable local layer; baked profiles remain the default.

### F.3 River candidate (Profile B, §E) — **Tiled Directional Flow**

When the B-profile river work is scoped, use **Tiled Directional Flow** (van Hoesel)
over plain Valve-style flow maps — it rotates the texture so it **carries waves**
rather than directionless noise, the better choice for non-turbulent flowing water.

### F.4 Effects phase (deferred, §E) — aesthetic-driven, normal-map-forward

For the deferred effects layer: **caustics aesthetic-driven** (look good cheaply,
realism out), and **normal-map manipulation** for apparent far-water detail at
near-zero vertex cost (reflection/refraction are functions of view + normal).

---

*This record is the W.2 construction authority. The recon evidence it rests on is
in `W2_0_RECON.md`; the research evidence behind §F is in
`WATER_RESEARCH_FINDINGS.md`; the deprecation list it references is executed in
W.2 Phase 2, not here.*
