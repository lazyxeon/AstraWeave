# L.3.C — The seam hypothesis: verified. The 4th cascade: built, and it carries a regression.

> **Beat:** L.3.C (executes the director's L.3.B ruling) · **Date:** 2026-07-29
> **Base:** `f11e06536` (L.3.B) · **Machine:** GTX 1660 Ti Max-Q · Vulkan · driver 592.82
> **Status: CODE LANDED, VERIFICATION INCOMPLETE, ONE REGRESSION FOUND.** The director's
> seam hypothesis is CONFIRMED with numbers (§1). The authorised 4th cascade is implemented
> and delivers its predicted capability gain (§2–3), and the L.3 station guarantee is intact
> (§4). But a matched control measured that it **triples far-field frame-to-frame stepping**
> — the same symptom the gate is about (§5). Snapping does not fix it (§6). The trade is the
> director's; §8 lists what is not yet verified.

## 1. Item 1 — the seam hypothesis is CONFIRMED

Measured on **the recording's world** (`l3c_relief_census.rs`, seed 12345, **radius 8 =
289 chunks**, asserted — the L.3/L.3.A harnesses flew radius 10 = 441 and were therefore
measuring a different world than the gate rejected).

Local prominence (height above the lowest ground within a disc — the drop a feature can
cast) against each cascade's minimum castable relief `h_min = b·sin(elev)`, where the total
depth slack `b` = receiver_bias_ndc × ortho_depth_range + caster_slope_scale × texel ×
cot(elev):

| dune scale | p50 relief | p75 | unrepresentable c1-side (<1.42 m) | c2-side (<9.12 m) | **lost across the seam** |
|---|---|---|---|---|---|
| R = 10 m | 2.12 m | 6.05 m | 35.9% | 78.7% | **42.8 pp** |
| R = 25 m | 3.86 m | 11.27 m | 18.7% | 72.9% | **54.2 pp** |
| R = 50 m | 6.67 m | 19.63 m | 8.3% | 60.9% | **52.6 pp** |

The director's "relief largely 2–8 m" is exact (p50 2.1–6.7 m), and **more than half of all
dune-scale features that cast a shadow on the c1 side cannot cast one on the c2 side.** The
prediction "dune shadows exist only c1-side" holds.

This also explains why L.3.B's aggregate band metrics stayed quiet: the loss is a
STEADY-STATE property of the far region, so as terrain sweeps across the seam individual
features flip while the band's total shadowed fraction — dominated by the big landforms
(p90 27–93 m) that still cast — barely moves.

## 2. Item 2 — the 4th cascade, as authorised

> **CORRECTED 2026-08-02 (L.3.C resolution).** The table below originally priced c2 with
> the **every-frame 2 m ortho pad** while the shipped renderer gives it the **400 m drift
> pad** — `FIRST_CACHED_CASCADE = 2`, so BOTH far cascades are cached and both carry the
> pad. The corrected shipped row and the (unchanged) figures the original row actually
> describes are both shown. The census now prints the policy each pad set belongs to, and
> `Renderer::shadow_cascade_fits()` publishes the fits the renderer actually computed, so
> the model can be checked against ground truth instead of trusted. **A model of the fit is
> not the fit.**

Layout `86.1 / 500 / 1400 / 3000 m`. Cascades 0 and 1 still fit against `shadow_mid = 500`,
so their matrices are arithmetically unchanged — that is what makes §4 possible.

| cascade | range | radius | pad | texel | depth range | h_min (sun 43.1°) |
|---|---|---|---|---|---|---|
| c0 | 0.5–86.1 m | 102.5 m | 2 m | 0.101 m | 355 m | **0.27 m** (unchanged) |
| c1 | 86.1–500 m | 569.9 m | 2 m | 0.559 m | 1762 m | **1.42 m** (unchanged) |
| c2 **as shipped (cached)** | 500–1400 m | 1554 m | **400 m** | **1.908 m** | **5112 m** | **3.39 m** |
| c2 *if run live (2 m pad)* | 500–1400 m | 1554 m | 2 m | 1.520 m | 4714 m | *2.82 m* |
| c3 | 1400–3000 m | 3286 m | 400 m | 3.600 m | 10309 m | **5.86 m** |

Seam losses (R = 25 m dune scale), **as shipped**: 500 m seam **54.2 pp → 26.7 pp**; new
1400 m seam 20.9 pp, sitting where dune shadows subtend few pixels. Worst castable relief
9.12 → 5.86 m. (The originally published 20.5 pp / 27.0 pp pair is what the layout delivers
with c2 running LIVE — see the L.3.C-resolution outcome doc, where that becomes a shipping
option rather than a modelling slip.)

Beyond the literal authorisation, and free: the receiver bias was a single **NDC** value, so
its WORLD magnitude scaled with each cascade's depth range — 0.18 m at c0, 5.36 m at the old
survey cascade, i.e. 40% of that cascade's total slack came from a bias nobody chose. The far
cascades are now **capped** at c1's world-space equivalent (`C2_BIAS_SCALE` 0.374,
`C3_BIAS_SCALE` 0.171 in `shadow_common.wgsl`). It is a cap, never a raise, so c0/c1 keep
exactly the bias they shipped with. Without it the authorised change alone would deliver
1.42 → 4.53 m at the 500 m seam (37.5 pp lost) instead of 3.39 m (26.7 pp).

> **Same correction applies to the constants.** `C2_BIAS_SCALE = 1762 / 4714` was derived
> from the *unpadded* c2 depth range; against the shipped padded 5112 m it caps c2's
> receiver bias at 0.96 m rather than c1's 0.88 m — 8.5% looser than intended. It is exact
> for a c2 that runs live. Tracked in the L.3.C-resolution outcome doc.

## 3. Implementation notes

- `CASCADE_COUNT` and `FIRST_CACHED_CASCADE` are the single sources of truth; the layer
  views, per-cascade UBOs, bind groups, profiler passes and caster stats are all
  `array::from_fn` over them.
- The cache and its **two-phase commit** generalised from one cascade to the far pair
  (`c_far_valid` / `c_far_pending`, set by the PASS, never by the matrix write). The L.3.A
  bug class — trusting a map because its matrix was computed — stays fixed.
- UBO 224 → **288 B**: 4 × mat4 + `splits` vec4 + `extras` vec4. `split2` rides `extras.z`,
  a lane that was already padding, so `splits`' lanes and `extras.x`/`.y` keep their offsets
  and the sentinel contract test (`test_shader_has_conditional_shadow_not_hardcoded`, which
  greps for `uLight.extras.x >= 0.0`) is unaffected.
- Memory: shadow array 3 → 4 layers at 2048² Depth32Float = **48 → 64 MiB (+16 MiB)**.

**The inventory earned its cost.** A pre-implementation sweep (5 parallel surface audits +
consolidation) found that both pass loops selected the culling matrix with
`match idx { 0 =>.., 1 =>.., _ => self.cascade2 }` — a 4th index would have fallen into `_`
and culled cascade 3 against cascade 2's frustum: wrong casters, no error, no crash. Converting
to `cascades[idx]` turned that into a bounds check. It then fired twice in testing, on two
arity surfaces I missed by hand *despite having them listed*: the per-cascade UBO/bind-group
arrays, and the GPU-profiler pass arrays. Both panicked on frame one in a test rather than
shipping wrong shadows. **Lesson: an inventory does not implement itself — drive the edit from
the checklist item by item, and prefer shapes (arrays over a count, destructuring over
indexing) that make the compiler or the bounds check the enforcer.** `l3_proof` now
destructures `let [c0, c1, c2, c3] = stats`, which failed to compile the moment the stats
array was still 3-wide.

## 4. The L.3 station guarantee — INTACT

| station | result vs L.3.A |
|---|---|
| desert_close_20m, grass_close_20m, grass_mid_47m (both suns + normals) | **0 differing pixels** |
| desert_boundary_y414 (default sun) | 33,271 / 481,962 px (6.9%), mean 116.31 → 119.06 |
| desert_boundary_y414_rake | 0 differing pixels |

The close/mid stations are untouched **by construction** (c0/c1 fits and biases unchanged),
not by luck. The y414 default-sun station is the distance-heavy framing whose ground sits at
400–700 m — exactly the cascades that were replaced — so its change is the fix acting where
aimed. Its rake variant is unchanged because at 20° elevation the big landforms already
shadow that framing, so finer cascades add nothing there.

## 5. THE REGRESSION — the 4th cascade triples far-field stepping

Measured with the L.3.B twin-flight instrument (identical deterministic path flown
shadows-on and shadows-off, differenced so all view-dependent shading cancels), pan route
3°/frame, 40 frames, **the recording's radius-8 world**, matched control:

| worst frame-to-frame step | 3-cascade (control) | 4-cascade (shipped) |
|---|---|---|
| survey band, mean effect | 0.90 luma | **2.69 luma** |
| survey band, shadow area | 2.01 pp | **6.44 pp** |
| near band | 0.67 luma / 1.83 pp | 0.67 / 1.83 (identical) |
| seam band (350–650 m) | 0.38 luma / 0.70 pp | 0.38 / 0.70 (identical) |

Same world, same route, same metric; only the cascade layout differs. Near and seam bands are
byte-identical, which corroborates that c0/c1 are untouched.

**Mechanism (hypothesis, not yet proven):** the regression is coupled to the benefit. c2's
finer 1.52 m texels now RESOLVE dune shadows, so when its cached window jumps at a refresh
there is real detail to shift — where the old 3.74 m cascade had almost none. Two cached
cascades also double the refresh-event surface touching the far field.

**This is the same class of artefact the gate is about.** It is measured, it is in the
opposite direction from the beat's purpose, and it is reported rather than absorbed.

## 6. Snapping does not fix it

Texel snapping applied to the cached cascades only (targeting the refresh jump, leaving
c0/c1 bit-identical): survey band 6.44 → **6.37 pp**, a 1% change. Removed rather than
shipped as inert complexity; the implementation is preserved in L3B_OUTCOME §2 R3 and in this
file's history. So the stepping is NOT sub-texel grid re-alignment — it is the window's
*coverage* moving and changing which occluders the frozen frustum culls in.

## 7. The obvious next lever, priced but NOT measured

**Stop caching the far cascades** (render c2/c3 every frame). That removes refresh jumps
entirely — there are no refreshes — and keeps the whole capability gain. The cost is what the
cache was introduced to avoid: L.3.A measured every-frame survey rendering at **+3.35 ms**
against a ~2.5 ms line, though that was ONE cascade covering 500–3000 on a 441-chunk world;
the split pair on a 289-chunk world will differ and must be measured, not assumed.
Intermediate options: cache only c3 (the far, low-detail one) and run c2 every frame; or
shorten the refresh interval for c2 only.

## 8. What is NOT yet verified — this beat is incomplete

1. **Perf re-gate** — not run. Two extra cached passes; the ~2.5 ms line is unconfirmed for
   this layout.
2. **Off-state bit-exactness** — not re-run since the 4-cascade change.
3. **Census re-run against the SHIPPED fits** — §2's c2/c3 numbers come from the calibrated
   model (`l3c_layout_comparison`), not from the renderer's own emitted radii. The one
   cross-check available: the harness printed the live outermost window radius as 3185 m on
   radius-8 vs the model's 3286 m (3% — the model is calibrated on radius-10 telemetry).
4. **The §5 mechanism** — hypothesis only; the decisive test is the uncached-far-cascade run
   in §7.
5. Traces (render, aw_editor), Invariant 20 wording, and the lessons-index entries for the
   two retractions are outstanding.

## 9. Files touched

- `astraweave-render/src/renderer.rs` — `CASCADE_COUNT` / `FIRST_CACHED_CASCADE`; arrays for
  cascades, layer views, per-cascade UBOs/BGs, profiler passes, caster stats; 4-cascade fit
  loop with the generalised cache + two-phase commit; `split2`; 288 B UBO write; skinned
  inline `MainLightUbo` widened for layout compatibility.
- `astraweave-render/shaders/shadow_common.wgsl` — 4-cascade `MainLightUbo`, 4-way select,
  per-cascade receiver-bias caps.
- `astraweave-render/src/renderer_tests.rs` — UBO size 288, cascade count 4, layer count 4.
- `tools/aw_editor/tests/l3c_relief_census.rs` — NEW: relief census + layout pricing.
- `tools/aw_editor/tests/l3a_proof.rs` — continuous flight defaults to the recording's
  radius-8 world (`L3B_RADIUS`).
- `tools/aw_editor/tests/l3_proof.rs` — stats destructured to make arity compiler-enforced.
