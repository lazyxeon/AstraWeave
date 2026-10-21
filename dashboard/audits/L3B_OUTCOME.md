# L.3.B — The frame-to-frame shadow stepping: diagnosis, four refutations, and a STOP

> **Beat:** L.3.B (follows the L.3.A render-gate rejection) · **Date:** 2026-07-29
> **Base:** `cc17a99da` (L.3.A) · **Machine:** GTX 1660 Ti Max-Q · Vulkan · driver 592.82
> **The rejection:** "the hard arc is gone but the far-field shadow region still tracks the
> camera, now as a broad irregular band that CHANGES IN STEPS between adjacent frames —
> shadows appearing/vanishing discretely during motion." Director's conviction target: the
> cached survey cascade's refresh behaviour (300 m drift / 8-frame tick).
> **Status: STOPPED — no fix shipped.** Every candidate within mandate was tested and
> refuted by measurement; the one structural defect that survives scrutiny is a
> cascade-LAYOUT trade that belongs to the director. Instrumentation shipped.

## 0. What this beat delivers

1. **The continuous-verification instrument** the L.3.A gate minted, built and in-repo:
   `l3b_continuous_flight` (per-frame capture along strafe or pan routes with no settle
   and no shadow toggling), survey-cascade refresh telemetry
   (`Renderer::shadow_survey_telemetry` / `shadow_survey_window`, plus `AW_CSM_LOG=1`),
   and the twin-flight differencing analysis that isolates the shadow term from shading.
2. **Four refutations, each with numbers** (§2) — including the director's own conviction
   target and preferred fix, and two candidate fixes I implemented and then rejected.
3. **One retraction of my own** (§3): a mid-beat "correction" I sent the director was
   itself wrong, built on a metric that measured my harness rather than the renderer.
4. **The surviving structural finding** (§4): a 6.4× rendition discontinuity across the
   500 m c1|c2 seam, with the arithmetic — plus a correction to `L3A_OUTCOME.md` §3,
   whose texel figure was 2× wrong.
5. **The trade table** (§5) and **what I need to close the reproduction gap** (§6).

## 1. The instrument (and why L.3.A's motion leg could not see this)

L.3.A sampled 7 stations with restore→settle→capture at each, which guarantees the cache
is *fresh* at every sample — structurally blind to anything living between refreshes. The
new leg flies one continuous path, captures **every frame**, never settles, and never
toggles shadows mid-flight (`set_shadows_enabled(false)` clears `c2_valid`, so toggling
would perturb the very cache under test).

Three metric generations were needed, and the first two failed — recorded because the
failures are the lesson:

| Metric | Verdict |
|---|---|
| Whole-frame / band mean luma | **Too coarse.** Most pixels are near/mid field (not cached); a far-field band change is swamped. |
| Raw luma of a tracked world point | **Contaminated.** At 40 m/frame a fixed point's patch moves several luma from terrain mip/hex-phase/LOD alone — measured 6.35 luma at a point whose shadow factor was provably constant (cache frozen, no boundary crossed). |
| **Twin deterministic flights, differenced** (`off[i] − on[i]`) | **Sound.** Identical camera path, shadows on vs off; all view-dependent shading cancels exactly, leaving the shadow field. This is the metric of record. |

A fourth attempt — tracking world points *inside* the isolated field — also failed, and
§3 retracts it.

## 2. Refutations (all on the isolated shadow field unless stated)

Routes: strafe at 1 / 10 / 40 / 120 m/frame and pan at 3°/frame, Y≈219, survey pitch 25°,
desert radius-10, raking sun 20°, 40 frames each.

**R1 — the refresh snap is not the dominant term (the director's conviction target).**
Worst frame-to-frame survey-band shadow-area step, split by whether the step landed on a
refresh frame:

| Route | ON refresh frames | BETWEEN refreshes |
|---|---|---|
| 40 m/frame | 1.72 pp | **7.02 pp** (raw-luma metric) |
| 10 m/frame (all-tick refreshes) | 0.51 pp | 1.23 pp |

Steps between refreshes were ~4× those at refreshes. On the sound metric the whole-route
worst is **1.24 luma / 2.30 pp** (strafe) and **0.87 luma / 1.69 pp** (pan) — small, and
not concentrated at refresh events.

**R2 — cascades 0/1 do not meaningfully crawl** (director item 3). At 1 m/frame, where
scene change is negligible and texel quantisation is the only moving variable, the near
band's worst step is **0.27 pp / 0.06 luma**.

**R3 — texel snapping is not the fix.** Implemented for all three cascades (the survey
cascade included — the one whose window jumps furthest). Result: **2420 differing pixels
per frame out of 786,432 (0.25%), max delta 26**, and **no improvement in any robust
metric** (survey band 1.24 → 1.24 luma; pan 0.86 → 0.86). Not shipped: it would break the
L.3 station byte-identity guarantee for no demonstrated benefit. The implementation is
recorded here for whichever beat demonstrates a need:

```rust
fn texel_snap_cascade(light_vp: Mat4, shadow_size: f32) -> Mat4 {
    let origin = light_vp * Vec4::new(0.0, 0.0, 0.0, 1.0);
    let half = shadow_size * 0.5;
    let (tx, ty) = (origin.x * half, origin.y * half);
    let dx = (tx.round() - tx) / half;
    let dy = (ty.round() - ty) / half;
    Mat4::from_translation(Vec3::new(dx, dy, 0.0)) * light_vp
}
```

**R4 — caster culling omits nothing that matters.** The classic CSM bug (an occluder
outside a cascade's box whose long shadow falls *inside* it gets culled) is real in
principle and looked likely at a 20° sun, where shadows run ~2.75× the caster height.
Tested by drawing **every** chunk into **every** cascade: frames are
**byte-identical — 0 / 786,432 differing pixels**. The cull is correct as written.

**R5 — the UV edge-fade band does not intrude on the visible slice.** Measured window:
centre 1750 m ahead, radius 3424 m → half-extent 3824 m (7649 m across), so the 10%
edge-fade band is 765 m per side; the visible slice's far corners (±2309 m lateral at
3000 m) sit **1515 m inside** the window edge, clear of the fade.

**R6 — cascade blending makes it WORSE, and was reverted.** A 15% blend band before each
split (the canonical fix for a hard cascade seam) measured, in the seam band (350–650 m):

| | hard seam | 15% blend |
|---|---|---|
| worst mean-effect step | 0.89 luma | **1.22 luma** |
| worst shadow-area step | 2.76 pp | **4.52 pp** |
| band shadow area | 50.0% | 60.5% |

Structural reason: blending mixes c2's *coarse, cached* rendition into the 425–500 m
region that was previously pure c1 and immune to c2's refresh, so the worst step moved
onto a refresh frame. **You do not blend toward the unstable party.**

## 3. Retraction

Mid-beat I told the director that texel snapping *was* the fix, on the strength of
world-anchored features stepping 13.24 luma (pan) and 22.57 luma (strafe) in the isolated
field. **That was wrong and I withdraw it.** Those numbers are reprojection artefact: on a
3°/frame pan a tracked point's screen position moves ~35 px/frame, my patch is centred by
my own camera model, and a couple of pixels of model error drags an 11×11 patch across
high-gradient shadow edges. Proof: the identical figures (13.24 / 22.57, same frames, same
per-feature ranges) reproduced across three materially different builds — baseline,
texel-snapped, and cull-disabled — including one pair that is byte-identical. A metric
that cannot distinguish those builds is measuring the harness.

Chain of reasoning that led me astray, recorded so it isn't repeated: an aggregate metric
was too insensitive → I built a per-feature metric to sharpen it → the per-feature metric
was dominated by tracking error → I read its output as signal. The correct guard is the one
applied at the end: **before believing a metric, verify it responds to a known code
change.**

## 4. The surviving structural finding: the 500 m rendition discontinuity

Not shown to produce the reported steps, but real, unfixed, and the only remaining
candidate for "the far field looks different and the difference is camera-anchored."

Per-cascade geometry at the editor survey framing (λ=0.7 → split0 86.1 m, split1 500 m,
shadow_far 3000 m; 2048² layers):

| | radius | half-extent | **m/texel** | depth range |
|---|---|---|---|---|
| c0 (0.5→86.1) | 102.5 | 104.5 | **0.102** | 359 |
| c1 (86.1→500) | 569.9 | 571.9 | **0.559** | 1762 |
| c2 (500→3000, +400 pad) | 3424.3 | 3826.3 | **3.737** | 10725 |

Across the 500 m seam, with total depth slack = receiver bias × depth range + caster
slope-scaled bias × texel × cot(elevation):

| | c1 | c2 | ratio |
|---|---|---|---|
| depth slack (sun 43.1°) | 2.08 m | 13.35 m | **6.4×** |
| minimum relief that can cast at all | 1.42 m | **9.1 m** | 6.4× |
| PCF penumbra (3×3, r=1.5) | 1.68 m | 11.21 m | 6.7× |

The ratio is **invariant in sun elevation**, so no lighting change hides it. Every dune
feature with 1.4–9.1 m of relief therefore casts in c1 and not in c2, and the boundary is
welded to a camera-relative sphere whose screen row spreads ~142 px as it intersects
topography — the shape of "broad irregular band." Measured statically at frame 20, the
seam contributes **+4.56 luma on-trend** (the large jump in that frame is a real cast
shadow edge at 400→450 m, +27 luma), so it is present but not a large static step.

**Correction to `L3A_OUTCOME.md` §3:** that document states c2 is "≈1.5 km radius →
~1.9 m/texel" and c0/c1 as "≈11 cm / ≈34 cm". Measured/derived truth: **3424 m radius,
3.737 m/texel**, c1 0.559 m, c0 0.102 m. Every quantisation and bias magnitude L.3.A
priced from that figure was understated ~2×.

## 5. Trade table — reducing the rendition gap (director's call)

Three cascades cannot span 0.5→3000 m at 2048² with a gap below ~6×: the texel ratios are
already near-uniform (5.6× and 6.7×), i.e. the split placement is balanced and the gap is
a *budget* consequence, not a tuning error.

| Option | Gap after | Cost | Risk |
|---|---|---|---|
| **A. 4th cascade** (86 / 500 / 1400 / 3000) | ~2.5× per step | +16.8 MB (one 2048² layer); +1 caster pass, cacheable like c2 (est. +0.3–0.4 ms) | Most invasive; another cached window to keep coherent (two-phase commit applies) |
| **B. 4096² for the far cascade only** | 3.3× | +67 MB for that layer; 4× caster raster area on refresh frames | Memory on min-spec; refresh-frame cost spike |
| **C. Tighten c2** (drop the 400 m pad; fit 500→2200 since the fade kills shadows by 2100) | ~4.3× | Free; requires more frequent refresh or snapping to stay stable | Partial improvement only |
| **D. Move split1 outward** (500→900) | 2.7× | Free | c1 coarsens 0.56→1.01 m — degrades the 86–500 m band the director already approved in L.3 |
| **E. Accept and document** | 6.4× | Free | The far field keeps reading coarser than the near field |

My recommendation if the director wants it closed: **A**, because it is the only option
that improves the far field without degrading the approved near/mid field, and the perf
headroom exists (L.3.A shipped at +0.88 ms against a ~2.5 ms line). But it is a real
memory/perf spend on a defect I could not reproduce at the reported magnitude — which is
why it is a decision, not an action.

## 6. The reproduction gap — what would close it fastest

In every scripted route I can construct, the shadow system is temporally stable to
**≤1.24 luma / ≤2.3 pp** of band shadow area per frame. I cannot reproduce
"appearing/vanishing discretely" at a magnitude matching the recording. Most useful from
the director, cheapest first:

1. **The sun elevation and camera mode** used in the recording (my routes assume the
   L.3 rake convention, elevation 20°, and a fixed survey pitch of 25°; the editor default
   sun is 43.1°, and orbit-zoom vs freefly change the path shape entirely).
2. **Was it the same world?** (desert, seed 12345, radius 10 — or a different archetype /
   the mesa region?)
3. **Run the editor with `AW_CSM_LOG=1`** and note whether the observed steps coincide with
   the printed `[csm] survey refresh #N (trigger)` lines. That single correlation settles
   R1 in the live app rather than in my harness.
4. If cheap: the recording itself, or a saved camera path (`.editor_preferences.json`
   stations along the route) so the harness can fly exactly it.

## 7. Verification (nothing behavioural shipped)

| Check | Result |
|---|---|
| naga shader validation | 4 / 4 passed (shader reverted to the L.3.A state — `git checkout` verified) |
| render lib `shadow` filter | 50 / 50 passed |
| Station frames | unchanged from L.3.A — the only renderer edits that survive are the telemetry fields/accessors, which are write-and-read-only, plus comments |
| Diff at close | `renderer.rs` (+telemetry, +comments recording the refutations), `l3a_proof.rs` (+continuous leg, +pan mode, +band metrics) |

## 8. Files touched

- `astraweave-render/src/renderer.rs` — survey-cascade refresh telemetry
  (`c2_refresh_count`/`c2_refresh_reason`/`c2_last_drift`/`c2_window`,
  `shadow_survey_telemetry()`, `shadow_survey_window()`, `AW_CSM_LOG=1` stream); comments
  recording R4's byte-identical cull result and R3's measured-and-not-shipped snapping.
- `tools/aw_editor/tests/l3a_proof.rs` — `l3b_continuous_flight`: per-frame capture with no
  settle and no toggling, strafe + pan routes (`L3B_STEP_M`, `L3B_YAW_STEP`, `L3B_FRAMES`),
  shadows-off reference leg (`L3B_SHADOWS=off`), survey/near/seam band metrics with
  on-refresh vs between-refresh attribution, `L3B_EXPECT=stable` assertions.
- `docs/audits/L3B_OUTCOME.md` (this file); `docs/audits/L3A_OUTCOME.md` §3 texel
  correction.
- Analysis scripts (twin-flight differencing, seam profiling) — paths cited in §1; they
  belong in-repo if the director wants this as a standing gate, which is an open question
  in §6.
