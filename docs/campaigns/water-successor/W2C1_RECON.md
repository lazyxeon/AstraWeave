# W.2c.1 — Weave-Response Deformation: Recon + Architecture Proposal

**Campaign:** W-series (Water Successor) · **Phase:** W.2c, stage 1 (recon) · **Mode:** read-only — propose-then-gate
**Branch at recon:** `campaign/water-successor` · **HEAD:** `c7531c944`
**Date:** 2026-06-22 · **Status:** recon complete; all forks ratified at the W.2c.1 gate (see §Gate). Built in W.2c.2.

This is the persisted forensic record of the W.2c.1 reconnaissance. The ratifications
it surfaced are now settled in [`W2_DECISIONS.md`](./W2_DECISIONS.md) §B.1 + §F and
were built in W.2c.2 — this doc keeps the *why* legible outside conversation.

---

## Headline finding (drove the §B correction)

The W2.0 §B line "weave-response registers *behind WaterQuery* (§7.7 single owner)"
collided with what the facade actually is. `astraweave-water` is a **CPU-resident,
deterministic gameplay-truth** layer whose own contract **excludes GPU/presentation**
(`astraweave-water/src/lib.rs:10-34`), and **`astraweave-render` does not depend on
`astraweave-water`** (only `astraweave-physics` does). A view-side deformation cannot
live in the gameplay-truth facade without violating its determinism contract *and*
inverting the dependency graph. → §B corrected in `W2_DECISIONS.md` §B.1.

---

## Deliverable 1 — WaterQuery facade extension map

**What's there to extend** (`astraweave-water/src/lib.rs`): `WaterQuery` trait, one
method `sample(point) -> Option<WaterSample>` (lib.rs:60-72); `WaterSample {
surface_height, density }` (lib.rs:47-53); `AnalyticWater` (plane + AABBs)
(lib.rs:127-215). All gameplay-truth; none carries a presentation/deformation concept.

**The three effects as view-side heightfield deformations:**
- **part** — additive *negative* displacement over an oriented corridor; exposes the
  bed (W.2b depth-foam + refraction render it).
- **raise** — additive *positive* displacement (dome/ridge/wall).
- **freeze** — *not* a displacement: a state mask that (a) lerps Gerstner displacement
  → 0 + damps the normal toward flat, and (b) drives a fragment-shader material-state
  change (ice/glass). Mechanically distinct from part/raise.

**Reconciliation (ratified):** distinguish weave *truth* from weave *presentation*,
exactly as the existing system separates `surface_height` (truth, facade) from
Gerstner/refraction (presentation, renderer — none in the facade). Weave presentation
is owned **render-side by `WaterRenderer`** — the §7.7 single owner for presentation,
not a parallel system. The facade stays gameplay-truth, untouched. Weave *truth*
(parted-corridor buoyancy, walkable freeze) is a separate facade concern with the
existing `WeaveOpKind::LowerWater` precedent (`astraweave-gameplay/src/weaving.rs:75`),
**out of W.2c view-side scope.**

## Deliverable 2 — Deformation profile format (the keystone)

A **profile** = a normalized, position-agnostic deformation SHAPE in local space
(unit height ∈ [-1,1] + freeze-mask ∈ [0,1]); **no world position/scale/magnitude.**
The runtime **instance** = `{ kind, position (world XZ), radius, orientation, intensity,
phase }`; the shader maps `world → local = rotate(world_xz − position, −orientation) /
radius`, evaluates the profile, scales by intensity. **Location lives only in the
instance** — a single authored `Part` profile serves part-anywhere.

Ratified: **analytical** profile for W.2c.2 (zero authoring pipeline), with a
**representation-agnostic instance interface** so a deformation-texture profile drops
in later. **Ceiling = 8.** **Freeze = unified dual-channel** (height + freeze-mask,
kind-tagged, one pipeline).

## Deliverable 3 — Shader-side playback recon

Post-W-FU-2 shader: 4-entry bind group (water.wgsl); deformation applies in `vs_main`
where `pos.y` is set (water.wgsl:~194), composing **after** the 4-Gerstner sum so the
Q-cap (internal to each `gerstner_wave`, water.wgsl:121/143) is untouched.

Ratified mechanism: **uniform/storage instance array** (mechanism A), not a
deformation-accumulation texture — small ceiling, per-vertex evaluation, ~zero extra
bandwidth (the recurring min-spec risk), best fit for the 1660 Ti. (Estimate: ~8
instances × ~20 ALU on a 1089-vertex tile = negligible vs 2.0 ms, linear in N —
**confirmed by W.2c.2 measurement: per-instance ≈ 0.0005–0.0008 ms, 8-instance total
≈ 0.004–0.006 ms.**)

**Skirt constraint (build rule, honored in W.2c.2):** bound deformation to ±`skirt_depth`
and sample at world XZ so matched LOD-boundary vertices agree (same as Gerstner) — no
new seam mechanism.

## Deliverable 4 — Build sequencing

- **W.2c.2** — playback layer (view-side, render-only): profile + WaterRenderer-owned
  instance list + shader deformation, validated with hardcoded/editor-placed test
  instances. Acceptance: part/freeze/raise render < 2.0 ms.
- **W.2c.3** — gameplay trigger wiring: feed the *same* instance list from
  fate-weaving. Seam identified: `AbilityState` cooldown/duration (abilities.rs:15-87)
  → intensity ramp; `Player::use_dash` (level.rs:111) / `WeaveOpKind` path → emit a
  `(profile, position, intensity, orientation)` tuple. **Playback layer unchanged.**
  Producer doesn't exist yet (no positional water deformation, no water-effect event
  bus — only the global `WeaveOpKind::LowerWater`).

---

## Gate — ratification outcomes (settled 2026-06-22)

| # | Fork | Ratified |
|---|---|---|
| 0 | §B owner reconciliation | **§B corrected** — render-side owner (W2_DECISIONS §B.1) |
| 1 | Weave-list owner | **Render-side `WaterRenderer`**; facade untouched |
| 2 | Profile representation | **Analytical**, representation-agnostic instance interface |
| 3 | Shader-feeding | **Uniform/storage instance array** |
| 4 | Concurrent-instance ceiling | **8** |
| 5 | Freeze handling | **Unified dual-channel, kind-tagged** |
| — | part-depth vs skirt_depth | Build constraint — bound to ±skirt_depth, world-XZ sampled |

*Recon record. Construction authority is `W2_DECISIONS.md` §B.1 + §F; the W.2c.2 build
is recorded in the W.2c.2 execution report.*
