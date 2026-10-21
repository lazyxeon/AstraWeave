# L.3.C resolution — measure before shipping: the far-cascade refresh policy

> **Beat:** L.3.C resolution (executes the director's "MEASURE BEFORE SHIPPING" ruling)
> **Date:** 2026-08-02 · **Base:** `154d723aa` (L.3.C) · **Shipped:** `ed35b3b35`
> **Machine:** GTX 1660 Ti Max-Q · Vulkan · driver 592.82 · 1920×1080 and 1024×768
> **World:** the recording's — seed 12345, Desert, **radius 8 = 289 chunks**, survey
> framing (eye Y ≈ 219 m, pitch 25°, yaw 45°), default sun (elevation 43.14°).
>
> **STATUS: L.3.C CLOSED** (director ruling, 2026-08-02). `FarCascadePolicy::LiveC2` is the
> shipped default. The far-policy trade (§8) is **decided at the director's gate flight** —
> both policies via the `AW_CSM_FAR_POLICY` knob, on the recording's route — not here. The
> unbuilt levers in §8 are HELD and activate only if that flight rejects both. Invariant 20's
> final wording and the trace updates are accepted as landed. Still pending from the flight:
> the policy ruling and the `AW_CSM_LOG=1` observation.

> ## ⚠ HEADLINE: fallback (a) fits the budget but does NOT fix the stepping
>
> The ruling's premise was that c2 "carries the newly-resolved detail — the jump-visible
> party". **Measured on the same binary with a control that provably distinguishes the two
> policies, uncaching c2 changes the far-field frame-to-frame step by nothing at all:
> 6.44 pp cached, 6.44 pp live-c2.** Uncaching c3 as well takes it to **2.81 pp** — back to
> the 3-cascade baseline of 2.01 pp. **The stepping is c3's cached window, not c2's.** The
> L.3.C mechanism hypothesis is refuted. Full numbers and the resulting trade in §8.

## 0. What this beat delivers

1. All four refresh policies measured on min-spec against the ~2.5 ms line, **three
   replicates each**, with a shadows-off control leg per run (§2).
2. The shipped policy chosen by that measurement, not by preference (§3).
3. The census re-run against the renderer's **emitted** fits, closing the L.3.C modelling
   error and correcting the published seam figures (§1).
4. Four defects found by a pre-implementation inventory and fixed (§4) — one of them a
   pre-existing divergence between the two draw paths.
5. The verification list the L.3.C outcome doc left open (§5).
6. **The stepping mechanism, identified and attributed** — and the finding that the shipped
   policy does not address it (§8). This is a premise failure in the ruling, so it is a STOP
   with a trade table rather than a decision I took on the director's behalf.

## 1. The census correction — a model of the fit is not the fit

`FIRST_CACHED_CASCADE = 2`, so the shipped renderer gives **both** far cascades the 400 m
drift pad. The L.3.C census priced c2 with the **2 m every-frame pad**. Everything derived
from that row was therefore optimistic:

| c2 (500–1400 m) | L.3.C published | shipped (cached, 400 m pad) |
|---|---|---|
| ortho pad | 2 m | **400 m** |
| texel | 1.520 m | **1.908 m** |
| ortho depth range | 4714 m | **5112 m** |
| minimum castable relief | 2.82 m | **3.39 m** |
| 500 m seam loss (R = 25 m dune scale) | 20.5 pp | **26.7 pp** |

The 4-cascade layout still recovers **54.2 pp → 26.7 pp** at the 500 m seam — the seam
hypothesis and the authorised fix both stand. The overstatement was 6.2 pp, and the
originally-published row turns out to describe *a configuration worth shipping* rather than
a fiction (§3).

The same error propagated into a shader constant: `C2_BIAS_SCALE = 1762 / 4714` was derived
from the unpadded depth range, so against the shipped 5112 m it capped c2's receiver bias at
0.96 m instead of c1's 0.88 m — **8.5% looser than intended**. `C3_BIAS_SCALE = 1762 / 10309`
was derived *with* the pad and is correct. Two constants, two different pad assumptions,
neither stated in the derivation comment.

**Fix (§4, H1):** the cap is no longer a constant. The renderer now publishes every
cascade's fit (`Renderer::shadow_cascade_fits()` → near/far, centre, radius, pad, depth
range, texel) and computes the per-cascade bias scale from the depth range it *just fitted*,
delivered in the UBO. It cannot go stale when a split, a pad or a policy changes.

## 2. Item 1 + 2 — every policy measured

An `AW_CSM_FAR_POLICY` lever (`cached` | `live` | `live-c2` | `alternate`, same diagnostic
family as `AW_CSM_LOG`) puts all four in ONE binary, so the comparison is a matched control
rather than four builds. Each run times the survey framing twice — shadows on, then off —
and reports **median ON − median OFF**, the shadow system's own cost. The off leg is
policy-independent, so it doubles as a drift control.

`l3a_proof.rs::l3c_perf_survey`, L.3's methodology: 60 warm-up + 300 timed frames, wall
clock with `device.poll(Wait)` (TIMESTAMP_QUERY hangs this driver), median / p10 / p90.

**Three replicates of all four policies, 12 runs, 1920×1080:**

| policy | rep 1 | rep 2 | rep 3 | **median** | 1024×768 | vs the ~2.5 ms line |
|---|---|---|---|---|---|---|
| `cached` (L.3.C shipped) | 1.222 | 1.069 | 1.029 | **1.069** | 0.822 | fits |
| **`live-c2`** (fallback a) | 1.753 | 1.674 | 1.726 | **1.726** | 1.238 | **fits, 0.77 ms spare** |
| `alternate` (fallback b) | 2.503 | 2.315 | 2.363 | **2.363** | 1.735 | at the line |
| `live` (item 1) | 2.860 | 3.034 | 2.836 | **2.860** | 2.122 | **over** |

Replication was not ceremony: on rep 1 the `live` run's off leg came in at 27.247 ms against
28.128 / 28.143 / 28.300 for the other three — a 0.9 ms outlier, enough to move that policy
across the line by itself. Reps 2 and 3 settled it (2.836–3.034); the verdict did not rest on
the run whose control was odd.

**The arithmetic, decomposed.** The four policies differ only in far-cascade work, so the
differences ARE the per-cascade prices:

```
    shadows off                                      0     ms   (the baseline the line is measured from)
  + near pair (c0,c1) every frame + far pair cached  1.069 ms   = `cached`
  + c2 re-fit and re-rendered every frame           +0.657 ms   = 1.726 ms  `live-c2`   <- SHIPPED
  + c3 re-fit and re-rendered every frame           +1.134 ms   = 2.860 ms  `live`
                                                     ~2.5  ms   L.3 STOP threshold
```

c3 is the expensive cascade — radius 3185 m, 202 of 289 chunks drawn — and it is the one
whose coarse stepping was accepted for three beats. c2 is the affordable one, and it is the
one carrying the newly-resolved dune detail that made the jumps visible. Fallback (a) is not
a compromise between the two; it is the split the cost structure was already pointing at.

For lineage: L.3 measured the whole shadow system at **+0.766 ms** (2 cascades, y414 framing,
radius-10 world) and L.3.A at **+0.88 ms**. Those are different framings on a different
world, so they are a trend, not addends — this beat's numbers are all same-framing,
same-world, same-binary.

## 3. Decision — `live-c2` ships

Item 1's answer is *no*: the fully-uncached far pair costs 2.860 ms against a ~2.5 ms line.
Per the director's ordering, fallback (a) is next and it fits with 0.77 ms to spare.
`FarCascadePolicy::LiveC2` is now `#[default]`.

Three things fall out of that choice rather than being argued for:

- **The stepping source is removed where it was visible.** c2 no longer freezes, so it has no
  refresh to jump at. c3 still does — its stepping is the behaviour that was already accepted
  through L.3, L.3.A and L.3.B, at 3.501 m texels where dune shadows subtend few pixels.
- **c2 gets its drift pad back as quality.** A window re-fitted every frame does not need a
  pad sized for 300 m of drift. Emitted fit: **texel 1.880 → 1.491 m**, depth range 5024 →
  4626 m, and the AABB cull tightens with the box — **102 → 70** of 289 chunks drawn.
- **The bias constants become right.** `C2_BIAS_SCALE = 1762/4714` was exact for an unpadded
  c2 all along; it was only wrong because a cached c2 is padded. This is not why the policy
  was chosen, and the constants are gone regardless (§4 H1) — a value that is accidentally
  correct under one policy is a trap, not a fix.

**Relief census against the EMITTED fits, shipped configuration** (`l3c_perf_survey`, dune
scale R = 25 m, 160,000 samples, sun 43.14°, bias scale as the shader receives it):

| cascade | bias cap | h_min | unrepresentable | seam loss |
|---|---|---|---|---|
| c0 | ×1.000 | 0.27 m | 1.5% | — |
| c1 | ×1.000 | 1.42 m | 18.7% | 17.2 pp at 86 m |
| c2 (live) | ×0.381 | **2.78 m** | 38.7% | **20.0 pp at 500 m** |
| c3 (cached) | ×0.176 | 5.71 m | 65.5% | 26.8 pp at 1400 m |

c0 and c1 cap at exactly ×1.000 — the cap never raises — which is what preserves the pinned
close/mid stations by construction rather than by luck. **The 500 m seam the director
identified as prime suspect goes 54.2 pp → 20.0 pp.**

## 4. Defects found by the pre-implementation inventory

A six-agent read-only inventory ran before any code was written (the L.3.C lesson: an
inventory does not implement itself). Four of its findings were real and are fixed here.

**H1 — the receiver-bias cap was a constant derived from a model.** Detailed in §1. The cap
is now computed from `cascade_fits[i].depth_range` and delivered per cascade in
`MainLightUbo.bias_scales` (UBO **288 → 304 B**; `bias_scales` is appended, so every earlier
offset — including the `extras.x` sentinel the contract test greps for — is untouched). Four
sites carry that size and are called out in the code comment, one of which (`debug_assert_eq!`)
is compiled out in the editor's release profile and therefore is *not* the guard.

**H4 — the two draw paths disagreed on when to run shadow passes (pre-existing).** *(Named
engine-wide win — see §9 for the landing confirmation and its evidence boundary.)*
`draw_into` has always gated on `self.shadows_enabled`; `Renderer::render()` never did. With
shadows off, `update_cascade_splits` clears `c_far_valid` every frame, so `render()` re-ran
all four caster passes every frame to fill maps the shader is sentinel-gated never to sample
— an accidental every-frame policy paid for by the game runtime and 13 examples. The editor
uses `draw_into`, so no gate evidence or measurement in this campaign is affected. Fixed;
`terrain_casts` deliberately not added to that path, which draws no terrain chunks.

**H5 — refresh telemetry was blind to the cascade that matters.** `c2_refresh_*` tracked only
the OUTERMOST cascade. Under the shipped `LiveC2` that is c3 — the one the policy does not
change — so a `LiveC2` run would have been numerically indistinguishable from `Cached`. The
telemetry is now per far cascade (`shadow_far_telemetry(far_index)`), and the continuous leg
derives which cascades are *jump-capable* from the data (a cascade that refreshes on every
frame pair has no "between refreshes" and is excluded from the attribution) rather than from
the policy name.

**H2 — the continuous-flight harness hard-coded the 400 m pad.** It reported half-extent,
texel and edge-fade band from `radius + 400.0`. Under any live policy the real pad is 2 m, so
it over-reported the half-extent by 398 m and the texel by ~26% — as `println!`, not
`assert!`, so no test would ever have caught it. It now reads `shadow_cascade_fits()`.

**Structural (H8).** `c_far_pending[c] = true` is the single write that makes both pass gates
policy-aware; the gates know nothing about `FarCascadePolicy`. Bypassing it for a live cascade
would update the matrix while the depth map stayed frozen — right matrix, stale map, wrong
shadows, no panic: the L.3.A bug in mirror image. It now says so in the code.

Two inventory findings were **refuted** and are recorded so they are not re-raised: that
`Alternate` needs new per-cascade phase state (it does not — `C2_REFRESH_INTERVAL` is even, so
`c2_tick`'s parity alternates cleanly), and that `l3c_relief_census.rs::shipped_three()` is
stale (it is the retired-layout baseline row, and its numbers model the 3-cascade survey
cascade *with* its pad, correctly).

**Duplication removed.** `tests/common/mod.rs` holds the relief arithmetic
(`min_castable_relief`, `prominence`, `main_grid`), included by both the CPU census and the
GPU perf leg, so the two cannot drift apart the way the model and the renderer did.

### Measured and rejected

`Alternate` (fallback b) is dominated on both axes and does not ship: 2.363 ms against
`live-c2`'s 1.726, **and** it keeps the padded 1.880 m texels, because a window that freezes
for even one frame still needs a pad sized for the drift limit. Its cost is half `Live`'s but
roughly **four times** `Cached`'s under a static camera (one far refresh per frame versus
~0.25). Fallback (c) — reduced caster sets for far cascades — was not built: (a) fits.

## 5. Verification — the list L.3.C left open

| Check | Result |
|---|---|
| naga shader validation | **4 / 4 passed** (the 304 B `MainLightUbo` + `bias_scales` select) |
| editor WGSL validation | **2 / 2 passed** |
| render lib, `shadow` filter | **50 / 50 passed** |
| `cargo check -p astraweave-render --features terrain-splat-arrays` | clean |
| `cargo check -p aw_editor --tests` | clean (no new warnings) |
| **Perf re-gate, shipped config** | **1.463 ms** @1920×1080, **1.230 ms** @1024×768 — against the ~2.5 ms line, ~1.0 ms spare |
| **L.3 station guarantee** | **11 / 12 byte-identical** vs L.3.C. Every close/mid station, both suns, plus normals and rake variants: **0 differing pixels**. Only `desert_boundary_y414` moved — 4,692 / 481,962 px (**0.97%**), mean 119.06 → 119.02 — the distance-heavy framing whose ground sits at 400–700 m, i.e. c2's own domain. (L.3.C's own change to that station was 6.9%.) |
| **Census against EMITTED fits** | §3 table — run by `l3c_perf_survey` from `shadow_cascade_fits()`, no model |
| CPU census after the `tests/common` extraction | **2 / 2 passed**, numbers bit-identical to before the refactor |
| **Continuous flight, shipped config** | ran; **the stepping is unchanged from `cached`** — see §8 |

**How far off the model still was.** With the pad corrected, the CPU model gives the shipped
c2 `h_min` 2.82 m / 20.5 pp; the renderer's own fits give **2.78 m / 20.0 pp**. The residue is
`FIT_K` extrapolation: the model predicts far radii 1554 / 3286 m against the emitted
**1524.7 / 3185.0 m** (2–3% high), because `K` was calibrated on c0 and c1 — which it
reproduces exactly — and then extrapolated outward. Small, and in the pessimistic direction,
but it is the same mechanism as the pad error at a smaller amplitude, which is why §3's
numbers are the emitted ones and the model is now only a *layout-comparison* instrument for
configurations that do not exist yet.

Station diffs are exact-pixel (`scripts/l3b/l3_station_diff.py`, which also lists stations
present on only one side — a silently missing capture would otherwise read as "no
differences").

## 6. Invariant 20, final wording

The director ratified the amendment in principle at L.3.B and asked for it to be finalised
against the 4-cascade layout. It is now clauses (e)–(i) of invariant 20 in
`docs/architecture/render_pipeline_material_system_shader_infrastructure.md` (v1.19):

- **(e)** four cascades, `CASCADE_COUNT` the single source of truth, every per-cascade array
  `array::from_fn` over it;
- **(g)** cascades `>= FIRST_CACHED_CASCADE` run under `FarCascadePolicy` (shipped
  `LiveC2`); a frozen window is safe to sample only through the **two-phase commit**, and the
  `c_far_pending` write is load-bearing for both pass gates; a cascade carries the drift pad
  **iff** its window can freeze, because the pad costs texel density;
- **(h)** the receiver-bias cap is **derived from the emitted fits**, never a constant;
- **(i)** the verification standard: **continuous per-frame legs** (no settle, no toggling),
  **per-far-cascade refresh telemetry** wherever any cache survives, **twin deterministic
  flights differenced** so view-dependent shading cancels, and **no metric is trusted until it
  has been shown to respond to a change whose sign is already known**.

Clause (i) is the campaign's own history compressed: L.3 passed static stations and failed a
moving camera; L.3.A passed a settle-at-each-station motion leg and failed continuous flight;
L.3.B produced two retracted findings from metrics that were never challenged.

## 8. The stepping is c3's — and fallback (a) does not touch it

### The measurement

Twin deterministic flights (identical orbit route, 3°/frame, 40 frames, radius-8 world),
flown shadows-on and shadows-off and differenced so all view-dependent shading cancels.
**All three configurations below were captured from the SAME binary** — only
`AW_CSM_FAR_POLICY` differed:

| configuration | c2 refreshes / 40 | c3 refreshes / 40 | survey band worst step | near band | seam band |
|---|---|---|---|---|---|
| 3-cascade (L.3.A, prior build) | — | 9 | 0.90 luma / **2.01 pp** | 0.67 / 1.83 | 0.38 / 0.70 |
| 4-cascade `cached` | 5 | 9 | 2.69 luma / **6.44 pp** | 0.67 / 1.83 | 0.38 / 0.70 |
| 4-cascade **`live-c2` (shipped)** | 39 | 9 | 2.69 luma / **6.44 pp** | 0.67 / 1.83 | 0.38 / 0.70 |
| 4-cascade `live` | 39 | 39 | 1.34 luma / **2.81 pp** | 0.67 / 1.83 | 0.38 / 0.70 |

Near and seam bands are identical in all four — c0/c1 are untouched, as designed.

### Why this is believed, given the campaign's history with metrics

Identical figures across configurations is precisely the L.3.B artefact signature, so the
control was validated before the result was accepted:

- `cached` and `live-c2` differ in the render: **4,656 differing pixels across the 40
  frames**, on 15 of them, peaking at 1,475 px. The policies are not the same run.
- They differ in behaviour: c2 refreshes **5** times under `cached`, **39** under `live-c2`.
- The two large steps land at **frames 28 and 32** — four frames apart, matching c3's
  refresh cadence (9 refreshes over 40 frames ≈ every 4.4). Neither is one of the frames
  where the two policies' pixels differ.
- The instrument responds to a change of known sign: uncaching c3 as well moves the step
  6.44 → 2.81 pp.

**A silent-fallback bug was caught by exactly this check.** When `LiveC2` became the
`#[default]`, `AW_CSM_FAR_POLICY=cached` had no match arm and fell through `_` to the
default — so the first "cached control" run measured `LiveC2` against itself and returned
0 differing pixels, which reads exactly like "the change has no effect". Every policy now
has an arm and an unrecognised value is loud. The 12-run perf sweep predates the default
change and is unaffected (each run printed its own policy and its own distinct emitted
fits — pad 400 vs 2, casters 102 vs 70).

### The trade — this is the director's call

| configuration | cost @1080p | 500 m seam loss | far-field worst step |
|---|---|---|---|
| 3-cascade (L.3.A) | ~0.88 ms | 54.2 pp | **2.01 pp** |
| 4-cascade `cached` (L.3.C) | 1.069 ms | 26.7 pp | 6.44 pp |
| 4-cascade **`live-c2`** (shipped now) | **1.726 ms** | **20.0 pp** | 6.44 pp |
| 4-cascade `live` | 2.860 ms | ~19.5 pp | **2.81 pp** |

`live` is the only configuration that recovers both the capability and the stability — at
**14% over the ~2.5 ms line**. `live-c2` is shipped because it is what the ruling's
budget-ordered sequence selects and it is a real capability gain, but it should not be
mistaken for a fix to the gate symptom, and it costs +0.657 ms for a seam improvement alone.

### The remaining lever, priced but NOT built

Fallback (c) — **reduced caster sets for the far cascades** — is the authorised way to make
`live` affordable. c3 draws **202 of 289** chunks every frame; `live` needs to shed ~0.36 ms,
roughly a third of c3's every-frame cost, to come under the line.

It is not built because the obvious criterion is unsound and this beat does not guess at
correctness. Skipping chunks whose AABB height range is below c3's `h_min` (5.71 m) would
drop *internally flat* chunks — but a flat chunk sitting 50 m above its neighbour casts a
large shadow off its edge, and its AABB height range says nothing about that. A sound
criterion needs inter-chunk height discontinuity, not just intra-chunk relief. Designing and
verifying that is a beat of its own.

A cheaper untried lever in the same family: tighten `C2_DRIFT_LIMIT` **for c3 only**. Jump
magnitude scales with how far the window is allowed to lag, so a 100 m limit should give
roughly a third of the current step at roughly three times the refresh rate — landing
between `cached` and `live` on both axes. One constant, directly interpolating the trade.

> **Director ruling 2026-08-02 — both levers are HELD, priced-but-unbuilt.** They activate
> only if the gate flight rejects *both* `cached`/`live-c2` and `live`. Do not build either
> speculatively: the caster-reduction criterion is unsound as stated above, and the
> drift-limit tightening would move a shipped constant ahead of the verdict that decides
> whether it is needed. L.3.C closes with `live-c2` as the shipped default; the far-policy
> trade is decided at the director's gate flight — both policies via the `AW_CSM_FAR_POLICY`
> knob, on the recording's route.

## 9. Named engine-wide win: the shadows-off waste, confirmed landed

**Landed in `ed35b3b35`.** The committed hunk, verified at HEAD:

```rust
-        let has_shadow_casters_r =
-            vis_count > 0 || self.mesh_external.is_some() || !self.models.is_empty();
+        let has_shadow_casters_r = self.shadows_enabled
+            && (vis_count > 0 || self.mesh_external.is_some() || !self.models.is_empty());
```

`draw_into` (the editor path) has always carried `self.shadows_enabled`. `Renderer::render()`
never did. Because `update_cascade_splits` clears `c_far_valid` on every frame the
shadows-off sentinel is set, that path re-ran **all four caster passes every frame** to fill
depth maps the shader is sentinel-gated never to sample — an accidental every-frame policy,
paid for by `aw_game_runtime` and 13 examples, for exactly as long as shadows were off.
One logical decision that had two expressions, and only one of them was right.

**Evidence, stated precisely — the off-state 12/12 is bit-exactness, not path coverage.**

| what | result | what it establishes |
|---|---|---|
| Off-state stations, 12/12 byte-identical vs the L.3.A baseline | **0 differing pixels on every station, both suns, normals and rake variants** | The shadows-off *output* is unchanged by everything in this push, including this fix. The waste was pure cost: no pixel ever depended on those passes. |
| The sentinel contract (`extras.x < 0` → `shadow = 1.0`, contract-tested) | holds | Skipping the passes **cannot** change output while shadows are off, whatever the maps contain. This is why the win is free rather than a trade. |

**What it does not establish.** Those stations are captured through `draw_into`, which already
had the gate — so 12/12 proves the fix is *inert to output*, and the sentinel contract proves
it *must* be, but neither exercises `Renderer::render()` at runtime. No frame was timed
through the fixed path. Closing that properly means hoisting the gate into one shared
expression both paths call, with a unit test — logged in §10 rather than done here, because
the push is verified and the director's ask was the record, not new code.

`terrain_casts` was deliberately **not** added to `render()`'s gate: that path draws no
terrain chunks, so gating on them would be a lie in the other direction.

## 10. HEALTH lane handoff — the clippy un-abort

**The canonical gate is GREEN at HEAD**, verified after the push:

```text
cargo clippy -p astraweave-render --features terrain-splat-arrays -- -D warnings   → exit 0
```

It was red before `ed35b3b35` on a **duplicated `#[cfg]`**: `terrain_material_manager.rs`
carried a module-level `#![cfg(feature = "terrain-splat-arrays")]` duplicating the `#[cfg]`
on its `pub mod` declaration in `lib.rs:107`. That is an `error`-class lint, so it aborted
compilation of the crate and clippy reported *nothing behind it*.

**Correction to my first report: the revealed backlog is not "~12".** That figure came from a
run that was still truncated. The honest inventory, taken with the aborting lint downgraded:

> **153 distinct findings across 41 files** — 31 in `src/` (all inside `#[cfg(test)]`
> modules, which is why the lib gate is green) and 122 in test/example targets.

| count | class |
|---|---|
| **74** | `unexpected cfg condition value` — `ssao` / `bloom` |
| 17 | `field_reassign_with_default` |
| 13 | manual `Range`/`RangeInclusive::contains` |
| 9 | `needless_range_loop` |
| 7 | unused import |
| 5 | `assert!(true)` — optimized out |
| 5 | unused variable |
| 5 | expression always evaluates to false |
| 4 | unnecessary `mut` |
| 14 | singletons (`unwrap()` on `Ok`, identity op, absurd comparison, useless `vec!`, bit-rotation, call-inside-`expect`, unit let-binding, too-many-args, useless type-limit comparison) |

**The masking was two-layered, and the second layer is still live.** With the duplicated
attribute fixed, the inventory *still* truncated: two deny-by-default
`clippy::absurd_extreme_comparisons` errors in
`tests/wave2_vertex_compression_remediation.rs:145-146` abort that target and hide everything
after it. Only downgrading that lint produced the 153 above. **Any `-D warnings` sweep of this
crate reports a floor, not a total, until those two are fixed** — the same lesson as the
duplicated attribute, one layer down.

**The 74 are not a style nit.** `#[cfg(feature = "ssao")]` and `#[cfg(feature = "bloom")]`
gate test bodies on features that do not exist in `astraweave-render/Cargo.toml`, so that
code **never compiles and never runs**. This is silent test vacuity, the same family as the
already-recorded `shader_validation` vacuity floor (58 validated < floor 60). It should be
triaged first: each site is either a test that should be running under a real feature name,
or dead code to delete.

**Not fixed ad hoc, by ruling.** These are pre-existing, span subsystems this beat did not
touch (particles, TAA, virtual texture, snow, LTC area lights, vegetation, Disney material),
and several are in golden/remediation suites where a "fix" could change what the test asserts.
They are handed to the HEALTH lane as a named batch, alongside the two items this beat
generated: the two `absurd_extreme_comparisons` errors above, and the shared shadow-pass gate
expression from §9.

## 11. Files touched

- `astraweave-render/shaders/shadow_common.wgsl` — `bias_scales: vec4` in `MainLightUbo`;
  per-cascade bias from that lane; the two hard-coded scale constants deleted.
- `astraweave-render/src/renderer.rs` — `FarCascadePolicy` (+ `from_env` / `frozen` /
  `forces_refresh`, measured table in the doc-comment) and `CascadeFit`; `far_policy` and
  `cascade_fits` fields; policy-driven margin and refresh trigger; fit-derived `bias_scales`
  in a 304 B UBO; per-far-cascade refresh telemetry (`shadow_far_telemetry`,
  `shadow_far_window`, `shadow_far_cascade_count`); `shadow_cascade_fits`,
  `shadow_far_policy`; `render()`'s shadow gate honours `shadows_enabled`; `AW_CSM_LOG`
  suppresses policy-forced refreshes and reports pad/texel/depth.
- `astraweave-render/src/lib.rs` — `CascadeFit`, `FarCascadePolicy` re-exports.
- `astraweave-render/src/renderer_tests.rs` — light UBO 288 → 304 B.
- `astraweave-render/src/terrain_material_manager.rs` — removed the duplicated module-level
  `#![cfg]` that was aborting clippy for the whole crate (§10).
- `tools/aw_editor/tests/common/mod.rs` — NEW: shared relief arithmetic.
- `tools/aw_editor/tests/l3a_proof.rs` — `build_survey_session` (single-sourced world/viewport
  setup, now returning the `TerrainState`); `survey_camera_res`; `l3c_perf_survey` (min-spec
  survey-framing timing + emitted-fit census); per-far-cascade telemetry and the
  jump-capable attribution rule; window geometry from the fits.
- `tools/aw_editor/tests/l3c_relief_census.rs` — pad correction + policy-labelled layout rows;
  helpers moved to `tests/common`.
- `scripts/l3b/l3_station_diff.py` — NEW: exact-pixel station A/B.
- `docs/audits/L3C_OUTCOME.md` — §2 corrected in place (pad, texel, depth, `h_min`, seam
  loss) with the original figures retained and labelled.
- `docs/lessons/WHAT_DIDNT.md` — entries 19 (the metric-validation guard, both retractions)
  and 20 (a model of the fit is not the fit).
- `docs/architecture/render_pipeline_material_system_shader_infrastructure.md` v1.19,
  `docs/architecture/aw_editor.md` v1.16.
