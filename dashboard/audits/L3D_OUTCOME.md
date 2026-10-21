# L.3.D — cascade-3 temporal continuity: the last shadow defect

> **Beat:** L.3.D (kill the far-field temporal popping) · **Date:** 2026-08-02
> **Base:** `724f13de0` (L.3.C-resolution close-out)
> **Machine:** GTX 1660 Ti Max-Q · Vulkan · driver 592.82
> **World:** the recording's — seed 12345, Desert, radius 8 = 289 chunks, survey framing
> (eye Y ≈ 219 m, pitch 25°, yaw 45°).

## 0. The defect, as inherited

L.3.C's policy table isolated it and L.3.C-resolution confirmed the attribution on one
binary with a validated control:

| configuration | survey-band worst frame-over-frame step |
|---|---|
| 3-cascade (L.3.A) | 2.01 pp |
| 4-cascade `cached` | 6.44 pp |
| 4-cascade `live-c2` (shipped default) | **6.44 pp — uncaching c2 changed nothing** |
| 4-cascade `live` | 2.81 pp |

The steps land on c3's refresh cadence (frames 28 and 32 of a 40-frame 3°/frame orbit;
9 c3 refreshes over that flight). **c3 freezes for several frames, then replaces its window
and caster set atomically.** The director flew the shipped `live-c2` default and failed the
gate on it, exactly as the table predicted.

The fix requirement is temporal, not spatial: **no multi-frame freeze-then-snap anywhere in
the shadow system.** Per-frame micro-stepping — what `live` exhibits at 2.81 pp — is
accepted; atomic multi-frame jumps are not.

An independent frame analysis of the rejection recording (1,121 frames) agrees on every
point it touches: multi-frame far-field freezes with atomic coherent-region reconfiguration,
bias combs on steep slopes, split-crossing pops on mid-distance ridges. It is treated here
as corroboration of measurements already in hand, not as a new source of findings.

## 1. Phase 0 — the instrument, and why it is validated before it is used

The cascade-index view is shading mode 5 (`ShadingMode::CascadeIndex`), routed through the
scene-env UBO's `debug_mode` float — ED-3's plumbing, extended by one value. Flat colours:
**c0 red, c1 green, c2 blue, c3 yellow**, beyond-coverage dark grey. Beyond-coverage is a
distinct swatch on purpose: "the shadow ends here" and "c3 is stale here" look identical in
a lit frame and are completely different defects.

**Refresh flash.** The last free pad float in the scene-env UBO becomes `debug_flags` — bit 0
= c2 re-fitted this frame, bit 1 = c3 — and the re-fitting cascade is brightened for that one
frame. This is what turns a temporal event into something an eye can catch: the whole defect
is a freeze followed by an atomic replacement, and without the flash you must frame-step to
see the cause. The director's gate is a flight, not a frame-stepper. The bits are derived in
the renderer from `cascade_fits[..].refreshed`, which the fit loop writes every frame, so no
caller can forget to set them and they cannot disagree with what actually happened.

**One boundary implementation.** `csm_cascade_index()` was factored out of
`csm_shadow_factor()` in `shadow_common.wgsl`, and both the sampler and the overlay call it.
Invariant 20(a) is "one CSM sampling implementation"; an overlay that re-derived the split
comparisons would be a second one, and would drift silently the first time a split moved —
making the overlay lie in exactly the situation it exists to diagnose.

**Validation before use** (`l3a_proof.rs::l3d_cascade_view_proof`). This campaign has been
burned three times by instruments that looked right and were not: two L.3.B metric
retractions, and the L.3.C control that silently measured a policy against itself. So the
overlay is not trusted until it passes two checks on the same continuous route the defect
lives on:

1. **Band order** — every pixel is classified by hue, and the mean screen row of c0 must lie
   below c1, c1 below c2, c2 below c3. View distance increases up the frame in the survey
   framing, so a genuine cascade-index view must stack in that order; a fall-through to
   another debug mode cannot satisfy it by accident.
2. **Flash correlation** — the frames whose c3 band is bright must be exactly the frames on
   which `shadow_far_telemetry(1)`'s counter advanced. This is a GPU-side uniform bit checked
   against an independently-derived CPU-side counter; agreement is what makes either usable
   as evidence.

The classifier works on hue rather than exact RGB because the debug colours pass through the
live tonemap (ED-3's documented caveat) — but the four hues are far enough apart that
dominant-channel classification survives it.

## 2. What validating the instrument actually caught

The validation refused to pass three times, and every refusal was a real defect. None of
them would have announced itself: each produced a confident, plausible-looking result.

**(a) The classifier's thresholds were guessed, not measured.** Symmetric ±40 channel gaps,
picked by eye. The live tonemap desaturates hard — c1's `(0.22, 0.75, 0.28)` lands at
`(176, 232, 191)`, green only 41 above red and **1** above the blue threshold — and the
beyond-coverage swatch is a *blue-dominant grey* that a naive "blue is max" test reports as
c2. Thresholds are now taken from a captured frame and the reference points are recorded in
the code.

**(b) The pulse was invisible to analysis and ambiguous to the eye.** Mixing 65% toward
white desaturated the band past any hue classifier *and* landed it on the sky colour. A
pulsing c3 was therefore indistinguishable from "c3 is not in frame". The pulse now **scales
the cascade's own colour by 0.35** instead: hue is preserved, so the band stays classifiable
while it pulses, and the luma delta is unambiguous in both directions.

**(c) A cascade that re-fits every frame was pulsing every frame.** Under the shipped
`live-c2`, c2 re-fits on all 40 of 40 frames, so its bit was set on every frame and the band
was *permanently* whited-out. A signal that fires every frame is not a signal. The renderer
now sets the bit only for cascades the policy allows to freeze — the same reasoning that
suppresses policy-forced refreshes in the `AW_CSM_LOG` stream.

**(d) The refresh counters were seeded from zero.** Warm-up frames have already re-fitted the
cached cascades, so frame 0 always reported a spurious refresh. Seeded from the post-warm-up
state instead.

### RETRACTION — my "c2 is occluded at the gate framing" reading was wrong

Between (b) and (c) the harness reported **c2 = 0 px across 40 frames**, and I read that as a
property of the world: a near dune ridge cutting the terrain silhouette at ~567 m and
occluding the entire 500–1400 m ring. I reported it as a finding, including the inference
that this explained why uncaching c2 changed nothing. **That was wrong and I withdraw it.**

The refutation is an independent lit frame of the identical world, route and frame index
(`l3cres_lc2_on/f000.png`, from the L.3.C-resolution continuous leg): it shows sandy terrain
at **every** row from 0 to 767. There is no sky gap. The rows I called sky were c2, rendered
permanently white by defect (c). With the pulse fixed, c2 measures **8,570,958 px (27.25%) at
mean row 218.3** — precisely the band I had declared empty.

The L.3.C-resolution conclusion that the stepping is c3's does **not** depend on the retracted
claim and still stands on its own evidence: the steps land on c3's refresh cadence, and
`cached` vs `live-c2` differ by only 4,656 pixels across the whole flight.

The process failure is worth naming precisely, because the guard existed and I stepped
around it: **the harness had not passed yet.** I read an intermediate number out of an
instrument whose own validation was still failing, and narrated it as a result. The rule is
not "validate the instrument eventually" — it is *do not quote an instrument that has not
passed its validation.*

### The frame, once the instrument is trustworthy

Survey framing, eye Y ≈ 219 m, pitch 25°, over 40 frames / 31.5 M pixels:

| class | share | mean screen row | luma sd |
|---|---|---|---|
| c1 | 56.72% | 547.4 | 0.00 |
| c2 | 27.25% | 218.3 | 0.00 |
| c3 | 6.01% | 123.3 | 18.56 (it pulses) |
| beyond coverage | 8.83% | 68.1 | — |
| sky / other | 1.19% | — | — |
| c0 | 9 px | — | — |

**c0 is genuinely absent**, and that is geometry rather than defect: at this altitude and
pitch the nearest ground in frame is ~223 m, and c0 covers 0–86 m. Band order is exactly
near-to-far bottom-to-top — `c1@547 > c2@218 > c3@123 > beyond@68` — and the flat-band check
(luma sd 0.00 within a class) rules out a fall-through to the UV or normal views, which vary
continuously by construction.

## 3. Phase 0.3 — the baseline, and proof the instrumentation is inert

The cascade-index view touches the live sampler: `csm_cascade_index` was factored out of
`csm_shadow_factor`, which is shipped code on the terrain and static paths. That is a claim
of semantic identity, so it is measured rather than asserted.

| check | result |
|---|---|
| Pinned stations vs the L.3.C-resolution shipped frames | **12 / 12 byte-identical, 0 differing pixels** |
| Baseline continuous leg, survey band worst step | **6.44 pp at frame 28** (2.69 luma) |
| near band / seam band | 1.83 pp / 0.70 pp |
| c3 refreshes over the 40-frame flight | 9 |

Every figure reproduces L.3.C-resolution exactly, on the recording's world and route. The
baseline is unchanged and the Phase-0 work is invisible to the lit path.

## 4. Phase 1 — the fix ladder

Pass criteria, both required: **(a)** survey-band worst frame-over-frame step ≤ `live`'s
2.81 pp, with the refresh correlation gone; **(b)** total shadow cost ≤ 2.5 ms at the L.3
methodology (median of 3 replicates, shadows-off control per run).

### Rung 1 — c3 on alternate frames, c2 live (`live-c2-alt-c3`)

The boolean `frozen(c)` became `freeze_bounds(c) -> Option<(drift_limit, pad)>`: one value,
because it is one decision. The pad exists solely to keep a frozen window covering its slice
while it lags, so it must exceed the lag the drift trigger permits — and it is not free, it
coarsens texels and widens the AABB cull. Splitting the two into a boolean and a global
constant is precisely what let `Alternate` pay `Cached`'s 400 m pad for a one-frame lag. A
cascade that lags one frame now gets a one-frame budget (limit 150 m, pad 200 m).

| | baseline `live-c2` | rung 1 | criterion |
|---|---|---|---|
| survey-band worst step | 6.44 pp (f28) | **4.09 pp** (f26) | ≤ 2.81 pp — **FAIL** |
| cost @1920×1080, median of 3 | 1.726 ms | **2.413 ms** (1.943 / 2.444 / 2.413) | ≤ 2.5 ms — pass |
| cost @1024×768 | 1.238 ms | 1.678 ms | — |
| c3 refreshes / 40 frames | 9 | 19 | — |

The smaller pad delivered its predicted side benefit, measured from the emitted fits: c3
texel **3.501 → 3.306 m**, depth range 10005 → 9805 m, `h_min` 5.71 → 5.43 m, 1400 m seam
loss **26.8 → 25.0 pp**, casters **202 → 187** of 289.

**Verdict: rung 1 fails (a).** Halving the refit interval halved the lag but returned only a
36% step reduction, not the ~50% first-order estimate — the jump is not the only term. It is
also poor value: 2.413 ms consumes nearly the whole budget for a partial fix, where `live`
reaches 2.81 pp for 0.45 ms more. Proceeding to rung 2 per the ladder.

### Rung 2 — incremental c3 transition (`live-c2-fade-c3`)

Rung 1 attacked the jump's SIZE. Rung 2 attacks its ATOMICITY: c3 keeps its normal cadence,
but the outgoing window is retained in a fifth array layer and the receiver blends between
the two over `C3_FADE_FRAMES = 4`.

The property that makes it cheap: both maps are **valid at their own time** — the incoming
one is fitted and rendered on the refresh frame and is immediately authoritative — so neither
needs an inflated pad, and the caster pass stays at `Cached`'s rate rather than `Live`'s.
Implementation notes that are load-bearing rather than incidental:

- the layer snapshot is issued **in the draw encoder immediately before the pass that
  overwrites it**, gated on `has_shadow_casters` for the same reason the two-phase commit is;
  a copy issued anywhere else either reads the new contents or races the pass writing them;
- the PCF helper is branch-free (taps unconditional, out-of-range folded in with `select`)
  because it is now called twice per fragment — an early return would make the taps
  conditional;
- `C3_PREV_LAYER` is named in both Rust and WGSL with nothing pairing them, so it carries a
  contract test — the same silent-drift class as the `match idx { _ => last }` the L.3.C
  inventory caught.

| | baseline | rung 1 | **rung 2** | criterion |
|---|---|---|---|---|
| survey-band worst step | 6.44 pp | 4.09 pp | **3.11 pp** | ≤ 2.81 pp — **FAIL by 0.30** |
| survey-band worst mean effect | 2.69 luma | 1.81 luma | **1.21 luma** | (`live` = 1.34) |
| cost @1920×1080, median of 3 | 1.726 ms | 2.413 ms | **1.601 ms** (1.601/1.724/1.546) | ≤ 2.5 ms — **pass** |
| cost @1024×768 | 1.238 ms | 1.678 ms | **1.140 ms** | — |
| c3 refreshes / 40 frames | 9 | 19 | 9 | — |

Rung 2 removes **52%** of the defect for **44% less cost than `live`**, and its worst
mean-effect step (1.21 luma) is *below* `live`'s (1.34). It misses the area criterion by
0.30 pp, and the residual peaks still fall on c3's refresh cadence (frames 26, 30, 34 at a
~4.4-frame interval), so the refresh correlation is weakened but not gone.

### Why combining rungs 1 and 2 was NOT measured

The obvious next move — rung 1's tighter cadence *plus* rung 2's blend — is arithmetically a
no-op, so it does not deserve a GPU run:

- rung 2: blend weight moves `1/4` per frame between windows separated by a jump `J`
  → per-frame pixel change `J/4`;
- rung 1 + rung 2: cadence halves so the jump is `J/2`, and the fade must shorten to ~2
  frames to fit inside the interval, so the weight moves `1/2` per frame
  → per-frame pixel change `(J/2)/2 = J/4`.

**Identical.** Once a transition is spread across the whole interval between transitions, the
per-frame change is just the intrinsic rate at which the shadow field is moving — which is
also what `live` exhibits, and is why rung 2 (3.11 pp) lands near `live` (2.81 pp) rather than
far below it. The residual 0.30 pp is the quality of a 4-step linear approximation to a
continuous refit, not a rate that more frequent refreshing can reduce. Raising the drift limit
and lengthening the fade proportionally is the same identity in the other direction, and costs
texel density through the larger pad.

### Rung 3 — STOP, with the ladder in evidence

Both rungs fail criterion (a); rung 2 by 0.30 pp. Per the ladder this is the director's
budget-line ruling, now with alternatives priced:

| | worst step | worst mean effect | cost @1080p | against the line |
|---|---|---|---|---|
| baseline `live-c2` (shipped) | 6.44 pp | 2.69 luma | 1.726 ms | the gate failure |
| rung 1 | 4.09 pp | 1.81 luma | 2.413 ms | fails (a) |
| **rung 2** | **3.11 pp** | **1.21 luma** | **1.601 ms** | fails (a) by 0.30 pp |
| rung 3 `live` | 2.81 pp | 1.34 luma | 2.860 ms | **14% over the 2.5 ms line** |

The two framings genuinely differ, and the difference is not rhetorical:

- against the **numeric proxy** (≤ 2.81 pp), only `live` passes, at 14% over budget;
- against the **stated requirement** — *no multi-frame freeze-then-snap; per-frame
  micro-stepping is acceptable* — rung 2 spreads every transition across the whole interval
  between transitions, at 1.601 ms with 0.9 ms of headroom.

**This report does not claim rung 2 clears (a).** Criterion (a) required the refresh
correlation to vanish, and it has not: the residual peaks still fall on c3's ~4.4-frame
cadence. It is weakened, not gone. Recommendation is rung 2 with that stated plainly — it is
the only option that improves both axes at once and it leaves budget for the Phase 2 defects
— but the ruling is the director's.

## 5. Phase 2.3 — camera-through-terrain: ED-lane handoff, not this beat's

The external frame analysis found two intervals in the rejection recording where the viewport
goes black (~0:27.4 and ~0:32.5): the editor's free camera passes **through terrain** at
speed. That is an editor-camera defect — no terrain collision or near-plane handling on the
survey camera — and it is explicitly **not** this beat's to fix.

Two consequences are this beat's, though:

1. **Handoff logged** for the ED lane: `OrbitCamera`/free-fly has no terrain-height clamp or
   near-plane push-out, so a fast survey pass can put the eye inside a dune. Timestamps
   ~0:27.4 and ~0:32.5 of the L.3.C-resolution rejection recording.
2. **Exclusion honoured**: every metric in this beat comes from the deterministic offscreen
   harness on a scripted orbit that never intersects terrain, not from the recording, so no
   camera-clip window enters any number reported here. The exclusion is structural rather
   than a windowing rule applied after the fact.

## 6. State at HEAD

`FarCascadePolicy::LiveC2` remains `#[default]`, so **HEAD renders exactly as the shipped
configuration**: 12/12 pinned stations byte-identical against the L.3.C-resolution frames,
with the whole Phase-0 instrument and both rungs present behind `AW_CSM_FAR_POLICY`. That was
measured twice — once after Phase 0 and again after rung 2 — because rung 2's PCF refactor
moved the ortho edge fade inside the sampling helper, reordering it relative to the distance
fade. The two orders reduce to the same expression (`1 + e(1−d)(pcf−1)`), but this campaign
does not ship algebra in place of a measurement.

Pending the director's ruling on which configuration ships:

- Phase 2.1 (bias combs on steep slopes) and 2.2 (split-crossing pops) are scoped but not
  started — both must be measured against the shipped config, and 2.2's viability depends on
  every cascade being temporally continuous first (L.3.B R6: *you do not blend toward the
  unstable party*).
- Phase 3's station/off-state guarantees and the perf re-gate must likewise be run on the
  shipped config.

## 9. Prior findings this beat must not re-learn

- **L.3.B R6 — you do not blend toward the unstable party.** A 15% cascade blend band was
  measured and reverted: in the seam band it made the worst shadow-area step *worse*
  (2.76 → 4.52 pp), because blending mixed c2's coarse cached rendition into a region that
  had been pure c1 and immune to c2's refresh. Any split-blend proposal in Phase 2.2 is
  judged against that result, and is only viable at all once every cascade is temporally
  continuous.
- **L.3.B — verify a metric responds to a change of known sign before believing it.** Two
  retractions came from metrics that could not distinguish materially different builds.
- **L.3.C — a model of the fit is not the fit.** Quality figures are priced from
  `Renderer::shadow_cascade_fits()`, never from a CPU model of the cascade fit.
