# W.2c.3.1 — Weave Gameplay Producer: Translation-Seam Recon

**Campaign:** W-series (Water Successor) · **Phase:** W.2c.3, stage 1 (recon) · **Mode:** read-only — propose-then-gate
**Branch at recon:** `campaign/water-successor` · **HEAD:** `8efa0d1d3` · **Date:** 2026-06-22
**Status:** recon complete; all forks ratified at the W.2c.3.1 gate; **built in W.2c.3.2.**

Persisted forensic record of the gameplay → render weave translation seam. Verified
by a fan-out workflow (4 readers + 4 adversarial verifiers, all load-bearing claims
confirmed). Settled decisions are in the gate table; the W.2c.3.2 build implements them.

---

## Headline

The ratified *coexist + translate* approach is sound, but ground truth is thinner than
the framing assumed, forcing three decisions to the surface:

1. **The existing `WeaveOpKind` barely overlaps water.** Of five variants, **only
   `LowerWater` touches water** — and it's a *global, positionless* `phys.clear_water()`
   that ignores `a/b` ([weaving.rs:70-78](../../../astraweave-gameplay/src/weaving.rs#L70-L78)).
   `RaisePlatform` is a **terrain** Fortify ([weaving.rs:79-93](../../../astraweave-gameplay/src/weaving.rs#L79-L93)).
   The other three are terrain/weather. **No op produces freeze.**
2. **Intensity/lifetime have no source in `WeaveOp`** (`{ kind, a, b, budget_cost }` —
   no magnitude, no duration; [types.rs:50-56](../../../astraweave-gameplay/src/types.rs#L50-L56)),
   and `AbilityState` (which *does* carry timing) lives in `astraweave-weaving`, which
   has **zero dependency on `astraweave-gameplay`** — abilities never emit `WeaveOp`s
   (verified confirmed). Ops are emitted only by the demo keyboard handler.
3. **The three crates are mutually independent** (render ⊥ gameplay ⊥ weaving; verified
   confirmed) — the translation must live in a **binary/glue** that owns both.

## Deliverable 1 — WeaveOpKind → render-weave-kind mapping

| `WeaveOpKind` | World effect (truth) | → render weave | Position |
|---|---|---|---|
| **LowerWater** | global `clear_water()`, ignores a/b | **part** | `op.a` (render reads it; truth global — coexist) |
| **RaisePlatform** | terrain Fortify at `a` | **raise** | `op.a` (water-displacement-from-terrain, ratified intended) |
| ReinforcePath / CollapseBridge | terrain | none | — |
| RedirectWind | weather (`set_wind`) | none | — |
| **FreezeWater** *(NEW)* | presentation-only (budget+carry `a`) | **freeze** | `op.a` |

**Freeze gap → ratified:** add a new `FreezeWater` `WeaveOpKind` (enum is
`#[non_exhaustive]`), **presentation-only** this phase (budget check/consume + carry
`op.a`; **no walkable-ice / buoyancy** — deferred to the post-arc truth-coupling phase).
**Coexist:** the translation reads ops in parallel; `LowerWater`/`RaisePlatform` truth
arms are untouched.

## Deliverable 2 — a/b → instance-transform

`a: Vec3`, `b: Option<Vec3>`. **Vec3→Vec2 = `(a.x, a.z)`** (drop y; matches the terrain
arms' `a.x/a.z`). **Single-point (`b=None`)** is operative — all three water-mapped ops
ignore `b`: `position = (a.x, a.z)`, `orientation = 0`, `radius =` profile default. The
two-point directional mapping (`orientation = atan2((b−a).z,(b−a).x)`, `radius = |(b−a).xz|`)
is **documented but not built** — no water op uses `b`.

## Deliverable 3 — intensity + lifetime sourcing

WeaveOp carries no magnitude/duration and ops are **instantaneous**. Ratified **option (a)**:
**per-kind default intensity** (part 0.7 / raise 0.6 / freeze 1.0, matching the editor
scaffolding) + a **synthetic envelope** (ramp-in → hold → fade, driven by each weave's age)
that gives the instantaneous op a visible duration. The **ability-sourced path stays
deferred** (would require an ability→weave link across decoupled crates). **Lifetime
mechanism:** `set_weave_instances` *replaces* the whole list each call, so the producer
holds the active set, ages/expires each, and re-pushes the survivors (≤8); the renderer
holds no lifetime state.

## Deliverable 4 — ECS plumbing

- **Dep direction (verified):** render ⊥ gameplay ⊥ weaving → translation lives in the
  **binary glue** that owns both. The only render-crate change is a method taking a
  *render* type (`&[WeaveInstance]`), so no gameplay type crosses into the renderer.
- **No water-effect bus exists** (verified). `astraweave-ecs` offers `Events<T>` +
  `insert_resource`/`get_resource`, but **no new event type is needed** — a
  producer-side active-weave list in the binary suffices.
- **Render gap:** `Renderer::water_renderer` is private with no `set_weave_instances`
  delegation → add `Renderer::set_water_weave_instances(&[WeaveInstance])`.
- **System order:** gameplay (`apply_weave_op`) → glue (translate + age + push) → render
  (`update_water` → `run_water_pass`). The push precedes `update_water`.
- **Seam continuity:** the runtime producer feeds the same `set_weave_instances` the
  W.2c.2 editor scaffolding ([engine_adapter.rs:3769-3801](../../../tools/aw_editor/src/viewport/engine_adapter.rs#L3769-L3801))
  feeds — they coexist (editor scaffolded, runtime gameplay-triggered).

---

## Gate — ratification outcomes (settled 2026-06-22, built in W.2c.3.2)

| # | Decision | Ratified |
|---|---|---|
| 1 | Op→weave mapping | LowerWater→part, RaisePlatform→raise, **FreezeWater→freeze (new op)**; others→none |
| 2 | Freeze-op gap | **Add `FreezeWater`**, presentation-only (walkable-ice deferred) |
| 3 | a/b→transform | single-point operative; two-point documented-not-built |
| 4 | Intensity/lifetime | per-kind defaults + synthetic envelope (option a); ability-sourced deferred |
| 5 | Plumbing | `Renderer::set_water_weave_instances` delegation + binary-glue producer; no new ECS event |

*Recon record. Construction authority is this gate table; the W.2c.3.2 build (the
`FreezeWater` op, the render delegation, and the `weaving_playground` producer) implements it.*
