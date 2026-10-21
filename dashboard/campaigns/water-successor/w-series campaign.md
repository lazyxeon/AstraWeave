# W-Series Campaign — Water Successor

**AstraWeave · Game Campaign Protocol**
**Status:** DRAFT for ratification
**Supersedes:** the "world-water successor campaign" deferral recorded at F.3.S
**Predecessor:** F-Series (Fluids Integration) — closed at F.4 envelope ratification

---

## W.0 — Premise & Scope Decision (the ratification this campaign exists to record)

### The decision in one sentence

AstraWeave's water subsystem is re-scoped from a **general fluid *simulation*** to a **layered fluid *rendering* system with a thin bounded-deformation layer**, and the SPH/voxel solver core is **deprecated**. Emergent fluid behavior is not a requirement; emergent behavior that the design needs is carried by the **terrain solver**, in scenes authored to be water-free.

### Why this is a deliberate settle, not a retreat

Two independent lines of evidence converge on the same conclusion:

1. **F.3.S measured the solver cannot carry general water on min-spec.** Active-cell sparsity clears the 1 ms gameplay-water budget only for small localized volumes (≲16³ active cells); full-extent flooding never fits (2.35 ms floor at 5% fill on a 64³ grid). Two structural walls were identified: column-coupled hydrostatic pressure, and forward-cascade order-dependence introduced by the F.3 conservation fix. Verdict ratified: **decline A→C for general water.**

2. **Veilweaver's design never requires general water.** Scope interrogation (W.0 derivation, below) establishes that water in Veilweaver is *interactive-but-scripted*, that fate-weaving needs only a *bounded* set of fluid effects (part / freeze / raise), and that the camera relationship to water varies by scene (distant shading / mid interaction / submerged) — which is a **rendering-LOD** concern, not a simulation one.

The thing F.3.S proved impossible is precisely the thing the game design confirms unnecessary. The solver's general-simulation core has no remaining caller in the shipped product. We delete it deliberately, preserve it as a recoverable reference, and document the gap so the decision stays revisable if min-spec hardware ever rises to justify revisiting it.

### W.0 derivation of record (the answers that bound the spec)

| Question | Answer | Implication |
|---|---|---|
| Where does water appear in gameplay? | Interactive but scripted (waterfalls, pools moved through) | Placement & shape known at author time → no general solver needed |
| Does fate-weaving deform fluid in ways art can't pre-author? | Somewhat — bounded set (part / freeze / raise) | Finite effect vocabulary → animation/effect territory, not physics |
| Camera relationship to water? | Mix across scenes (distant / mid / submerged) | Rendering-LOD selection problem, not a unified-sim problem |
| Any scene needing genuinely emergent, uncontrollable water? | No — such scenes authored water-free; emergence handled by terrain solver | Closes the last edge that could re-justify a solver |

### The escape-hatch principle (records the design constraint that makes this clean)

Emergent fate-weaving runs on **terrain**, which legitimately needs full emergent capability. Water scenes use the bounded vocabulary. **The two do not overlap by design.** This is a level-design constraint, not an engineering compromise — legible, learnable affordance boundaries ("where can the player reshape reality") are better game design than universal deformability. One emergent solver (terrain) carries the unpredictable load; water becomes the cheap, authored, beautiful layer.

---

## Target Architecture (what W builds toward)

A layered water system, contexts selected per scene, all gameplay-water truth behind the existing `WaterQuery` trait.

1. **Surface layer (workhorse).** Heightfield water — Gerstner / FFT waves, GPU-displaced mesh. Handles ambient "water is present and alive" for ~90% of scenes. Cheap, scalable, art-directable.

2. **Weave-response layer (the mechanic).** A small authored library — **part / freeze / raise** — parameterized and triggered by fate-weaving. Implemented *behind `WaterQuery`* as new backends:
   - **Raise / part** → heightfield displacement / authored deformation
   - **Freeze** → representation/state swap + shader transition
   Small surface area by construction; this is the entire "somewhat deformable" requirement.

3. **Context / LOD selector.** Camera-distance-driven technique swap:
   - **Distant** → surface shading (FFT/Gerstner, SSR, foam); depth irrelevant
   - **Mid** → interaction ripples, shoreline, splash particles
   - **Submerged** → underwater fog, caustics, light shafts
   The "mix across scenes" answer, handled as rendering modes — no shared simulation substrate required.

4. **Particle accents.** The ratified **F.4 Option A envelope** (iteration-capped GPU particles, ~3 ms, ~15–20k ceiling on 1660 Ti Max-Q class) is **retained intact** for splash/spray. W does not re-litigate F.4; it inherits it as the accent layer.

---

## Salvage Map (preserves F.2's architectural investment)

The F.2 facade is the deprecation seam. `astraweave-water` + `WaterQuery` sit as a leaf below physics (`physics → water → glam`, verified acyclic) and already retired the three prior water abstractions onto a single owner. The solver lives *behind* this seam, which is exactly why it can be removed without the rest of the engine noticing.

**LIFT FORWARD (becomes the W architecture's spine):**
- `astraweave-water` facade + `WaterQuery` trait — the part/freeze/raise effects become new implementations behind it; it remains the single logical owner of gameplay-water truth.
- F.4 Option A GPU-particle path — retained for accents.
- Renderer-agnostic substrate: buffer management, GPU plumbing, math/utility code (verify exact boundaries in W.1 audit).

**TAG & REMOVE:**
- SPH particle solver core.
- `WaterVolumeGrid` voxel sim and the active-cell sparsity machinery — the general-simulation core whose only purpose was the ambition F.3.S declined.

**PRESERVE AS DOCUMENTED OPEN QUESTION (not as code):**
- The T3 full-flooding ambition and its three documented paths (tiling / larger budget / future GPU-voxel campaign) — already deferred at F.3.S. **This deferral is the dormancy mechanism.** No dormant code is retained; a git tag (e.g. `w-pre-deprecation` / `fluids-solver-final`) on the pre-removal commit gives the deferral a concrete reference. If min-spec ever rises, the prior art is one `git checkout` away as *reference*, and would be rewritten against the then-current engine regardless. Dormant code in the active tree preserves the liability (maintenance gravity, mental overhead) without the asset (real reusability); the tag preserves the asset without the liability.

---

## Research Pointer (W.0 literature pass — restart guidance for the deferred successor)

A literature review was run against the two structural walls F.3.S identified, to determine whether they were fundamental or merely unsolved-in-our-implementation. Finding: **both walls have known answers in the literature; the answers vindicate deprecation rather than reopening the solver.** The solutions are not patches to the existing voxel/SPH core — they are different simulation architectures entirely. This strengthens the W.0 decision: the thing F.3.S declined is a from-scratch rewrite regardless.

**Wall 1 — column-coupled hydrostatic pressure.** Directly addressed by the **tall cell grid** (Chentanez & Müller-Fischer 2011): full-resolution cells near the surface, deep hydrostatic water collapsed into single "tall cells," paired with a specialized multigrid Poisson solver. The general enabling property is multigrid's resolution-independent convergence (iteration count ~constant as grid grows) — the standard answer to "iterative pressure solve won't converge in budget," which was precisely the F.3.S flooding failure. **Caveat: the tall cell grid is patented by NVIDIA (US10055875B2 / US20130035917A1).** For a permanently-MIT engine this is a genuine constraint, not a footnote — implementing from the patent is the kind of encumbrance the no-middleware stance exists to avoid.

**Wall 2 — forward-cascade order-dependence.** Not so much "solved" as designed out. The order-dependence was an artifact of enforcing conservation via a sequential forward sweep (the F.3 fix). Modern real-time hybrids (PIC/FLIP, APIC, MLS-MPM) don't do ordered conservation passes at all: the grid step is a global, parallel, order-independent Poisson projection and advection rides on particles, so conservation falls out of the projection rather than an ordered cascade. Hybrid methods also sidestep SPH's neighborhood-search bottleneck, which is the real performance story.

**The field's own large-water answer is layered, not monolithic** — FFT far-field + particle volume + whitewater accents — which is structurally the same decomposition as this campaign's surface / weave-response / particle-accent architecture. The gameplay-side derivation and the simulation-side state-of-the-art converge on the same shape.

**Restart guidance (supersedes "resurrect the git tag"):** if the deferred world-water successor ever activates (min-spec rises, ambition returns), **do not resurrect the tagged SPH/voxel solver.** Start from a hybrid grid-particle method (APIC / MLS-MPM) with a multigrid pressure projection, and avoid the NVIDIA tall-cell patent. The tagged solver is preserved as historical reference only; the field has moved away from that architecture. The correct restart point is a different family of method, not the prior art.

---

## Phasing

### W.1 — Recon / Audit (read-only, gates everything)

**Mandatory before any removal.** Standing law: *built is not run; run is not seen; counted is not rendered; measured-elsewhere is not measured-here.* The crate has moved through F.1–F.3.S since the F.0 audit; salvage boundaries must be verified against **current** crate state, not assumed.

Deliverables:
- Current component inventory of the ~60K-LoC crate (solver core / voxel grid / facade / rendering / editor), with LoC and current call-site map.
- **Precise salvage boundary**: which modules are renderer-agnostic substrate (lift) vs. solver-specific (remove). The lift/remove line must be drawn against real `use`/dependency edges, not the categories above.
- Confirmation the `WaterQuery` seam fully isolates the solver (no caller reaches past the facade into solver internals). Any leak found here is a W.1 fix before deprecation proceeds.
- Test inventory: which of the crate's green tests cover lift-forward code (keep) vs. solver core (remove with it).
- Editor surface: what fluids editor code references the solver and must be re-pointed or removed.

**Gate:** no W.2 work until the salvage boundary is verified and the seam is confirmed clean.

### W.2 — Deprecation

- Tag the pre-removal commit.
- Lift renderer-agnostic substrate into the W water crate / target module per the W.1 boundary.
- Remove SPH solver core + `WaterVolumeGrid` + sparsity machinery.
- Re-point or remove solver-referencing editor code.
- Update the deferral record to cite the tag; record the removal in the campaign log and CLAIMS_REGISTRY.
- **Red line:** the build stays green and `WaterQuery`'s existing analytic callers keep working throughout. Removal must not regress the facade.

### W.3+ — New Water Architecture (scoped, not yet specced in detail)

Surface layer → weave-response layer → context/LOD selector → particle-accent integration, each as its own sub-phase with its own evidence gate. **Detailed W.3+ specs deferred** until W.0 ratified and W.1 audit returns — the architecture above is the envelope, not the build order.

---

## Standing Red Lines (inherited from F, still binding)

1. No particle/water state in `WorldSnapshot`, `world_hash`, replay, or net replication — ever.
2. Water stays a Cargo leaf; the cycle `physics → fluids → terrain → gameplay → physics` must not close. Glue stays in `astraweave-water` below physics.
3. No second implementation: `WaterQuery` remains the single logical owner of gameplay-water truth. The part/freeze/raise backends register *behind* it; they do not introduce a parallel abstraction.
4. Prove it, don't hype it: no rendering/budget claim ships without a recorded baseline + machine context. Built is not run; run is not seen; counted is not rendered; measured-elsewhere is not measured-here.
5. Evidence discipline: log counters are never sufficient for visual/interactive features; only captures or GPU readback assertions qualify.

---

## Open Questions for Ratification

1. **Campaign letter** — confirm `W` (water successor) vs. another letter that doesn't collide with existing campaigns.
2. **Tag name** — `fluids-solver-final` vs. `w-pre-deprecation` vs. other convention.
3. **W.3+ sequencing** — surface-first (de-risk the workhorse) vs. weave-response-first (de-risk the mechanic). Recommendation: surface-first; it's the larger visual surface and the weave layer sits on top of it.
4. **Editor scope** — does the fluids editor tooling get rebuilt for the new layers in W, or deferred to a later editor-parity pass?