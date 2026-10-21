# L.3.A — The camera-anchored shadow boundary (outcome)

> **Beat:** L.3.A (closes the L.3 render-gate rejection) · **Date:** 2026-07-29
> **Base:** `9fefac61e` (L.3) · **Machine:** GTX 1660 Ti Max-Q · Vulkan · driver 592.82 (min-spec)
> **The rejection (2026-07-29, screen-recorded):** "the shadows seem to be attached to the
> camera boundary instead of the world … always keep a sharp boundary line."
> **The lesson this beat mints:** camera-fitted systems cannot be verified at static
> stations alone — camera-MOTION legs are mandatory. `l3a_proof.rs` is that leg, and it
> is now part of the shadow system's regression surface.

> **Status: COMPLETE, awaiting the director's re-fly.** Motion leg passes (world-anchored
> shadow state at every path point, 2.1 km → 1.5 km view distance); coverage continuous to
> multi-km; close/mid stations bit-identical to L.3; y414 finally shadowed; off-state
> bit-exact; +0.88 ms typical against the ~2.5 ms line.

## 0. Summary

The boundary was the engine's **500 m view-distance coverage cap with a 100 m fade band
(400→500 m)** — both camera-relative, sized for ground-level play and plainly inside
visible range for the editor's survey camera (the rejection route flew at Y≈219). The
L.3 static stations sat entirely inside or outside the bubble, so the A/B never showed
it; the y414 station's zero-pixel result was the boundary's fingerprint, filed as
residue and now closed.

**Fix:** a **third (survey) cascade** covering view distance 500 → 3000 m, with the far
fade widened to the last 30% of coverage (2100 → 3000 m — a 900 m band). Cascades 0/1
keep their L.3 fits bit-for-bit (split formula still computed against 500), so the
shipped close/mid quality is pinned, not re-tuned. One shadow map array grew 2→3 layers
(2048², +16.8 MB); `MainLightUbo` grew to 3 matrices + a `vec4` splits (224 B); the
shared `csm_shadow_factor` selects 3-way. Both static and terrain shaders inherit the
fix through the single shared implementation (the L.3 hoist paying off).

## 1. Phase 1 — discrimination (director's frame analysis + measured confirmation)

The director's frame analysis of the rejection recording (8 frames) established: a
RADIAL arc at several hundred meters ground distance, fixed relative to the camera
(Y≈219 constant, ~470 units lateral movement, the arc swept across different terrain
unchanged), and exactly ONE boundary — the split0 ≈ 86 m cascade seam does not appear,
so cascade-1 sampling was exonerated before this session began.

Measured confirmation (`l3a_proof.rs::l3a_boundary_evidence`, HEAD leg `head_before`,
desert radius 10, raking sun 20°, survey camera pitch 25°):

- **Radial profile at alt 219**: shadow effect (per-row mean |on−off| luma, center
  columns) exists ONLY in a band at view distance ≈ 455–512 m — 0.00 nearer than
  ~410 m in that framing's flat foreground and 0.00 beyond 515 m. The measured band
  matches the fade band's predicted location (400→500 m view distance; flat-plane
  row mapping carries ±dune-height error).
- **World-anchored features flip with camera distance** (the defect, quantified): two
  strong-shadow world points chosen from the final path frame, tracked across a 600 m
  lateral camera sweep at Y≈219 by reprojection:
  - feature0 (world −359, 36, −2255): `0.0, 0.0, 15.7, 34.0, 39.5, 43.1, 44.6` luma of
    shadow effect as the camera approaches (view distance 906 → 604 m);
  - feature1 (world −757, 36, −2726): `0.0, 0.0, 5.6, 15.4, 41.8, 41.1, 42.5`
    (1509 → 1160 m).
  Shadow state was a function of CAMERA position — the recording's complaint, in
  numbers.
- **Method caveat, recorded:** the flat-plane (h = 36.3) row→distance mapping degrades
  at grazing rows over tall dunes (alt110's far rows mis-map nearby dune faces to
  multi-km distances; error grows as height_error/tan(depression)). The
  world-anchored-feature assertions are immune to this and carry the pass/fail; the
  radial profile is corroborating evidence with stated error bars.

## 2. Phase 2 — the live fade path, audited

The fade **exists and reaches pixels** (not the house pathology): `csm_shadow_factor`
(`shaders/shadow_common.wgsl`) applies `mix(shadow, 1.0, fade)` per fragment for
`frag_dist ∈ [0.8·shadow_far, shadow_far]`, and the delivered uniforms at HEAD were
`splits.y = split1 = shadow_far = 500.0.min(view.zfar) = 500` (`renderer.rs`
`update_cascade_splits`). So: a linear 100 m band at view distance 400→500 m, then the
`frag_dist < shadow_far` gate ends coverage entirely. Both are camera-relative. At
Y≈219 the band maps to ground ≈ 335→449 m; the entire raking-shadow effect collapses
across ~114 m of ground and then shadows cease to exist — in motion, a translating
edge. The defect is **coverage sized for the wrong camera regime**, not a missing or
broken fade.

## 3. Phase 3 — the fix

Constants (`renderer.rs::update_cascade_splits`): `SHADOW_FAR_SURVEY = 3000.0`,
`SHADOW_FADE_FRACTION = 0.7`; `shadow_mid = 500.min(zfar)` (the c1|c2 boundary),
`shadow_far = 3000.min(zfar).max(mid+1)`.

- **c0/c1 pinned**: the PSSM λ-blend split and both sphere fits are still computed
  against `shadow_mid = 500` — identical numbers to L.3, so close/mid shadow quality
  cannot silently regress (verified §4: the 20 m stations byte-match the L.3 frames).
- **c2 (survey)**: sphere-fit of the 500→3000 m frustum slice, same rotation-stable
  ortho construction, same per-cascade separate UBO (the write-race invariant), same
  per-cascade AABB chunk culling, drawn in both `render()` and `draw_into` pass loops.
- **Texel-density trade, stated with its arithmetic** — **CORRECTED by L.3.B, the figures
  below were ~2× optimistic; see `docs/audits/L3B_OUTCOME.md` §4 for the measured values
  (c2 radius 3424 m, half-extent 3826 m, 7649 m across, **3.737 m/texel**; c1 0.559 m;
  c0 0.102 m) and for the 6.4× c1|c2 rendition ratio that follows from them.** Original
  text, left standing per the append-only convention: c2's ortho square is the
  sphere fit of the 500→3000 m slice plus the 400 m drift margin — at the survey
  framing ≈ 1.5 km radius → ~3.8 km across 2048 texels ≈ **1.9 m/texel**. That is
  appropriate for content viewed at 500 m+ (a 1.9 m shadow texel subtends well under
  a pixel at that range) and it never touches fragments nearer than 500 m (hard
  select). c0 (≈ 11 cm/texel) and c1 (≈ 34 cm/texel) are bit-identical to L.3 — the
  close-up byte-guard in §4 is the proof, not an assertion.
- **The fade**: last 30% of coverage (2100→3000 m). At the rejection altitude the band
  subtends heavy angular compression near the horizon; no edge is perceptible in
  motion (§4 motion leg).
- **The cached survey cascade** (added after the first perf re-gate measured
  every-frame c2 at +3.35 ms — over the ~2.5 ms budget line; §5). c2 re-fits and
  re-renders on a **refresh**: sun direction change, ideal-window drift past
  `C2_DRIFT_LIMIT = 300 m`, a slow periodic tick (`C2_REFRESH_INTERVAL = 8` frames,
  so terrain edits appear promptly), or a map not known-good. **A frozen c2 is
  correct, not stale**: shadow positions are world-anchored — only the coverage
  WINDOW is held; sampling uses the same frozen matrix the map was rendered with
  (`shadow_cascade_bufs[2]` and `light_buf.view_proj2` are written only on refresh);
  and the ortho box is **padded by `C2_DRIFT_MARGIN = 400 m` > the 300 m drift
  limit**, so a drifted window still fully contains the ideal 500→3000 m slice — no
  coverage gap can open at its near edge between refreshes. Cost of the padding:
  c2's ortho radius grows ~400 m on ~1.5 km (texels ~25% coarser at survey range).
  This is the reduced-rate far-cascade option L.3 recorded as its perf fallback
  ("noted, not built") — now built, because the budget demanded it. It is an
  update-RATE amortization, not a stepped quality tier (invariant 19 governs shading
  falloffs; c2's content quality is unchanged on every frame it renders).

### 3.1 A defect I introduced, found by the byte-guard, and its fix

The first cached implementation regressed `desert_boundary_y414` from 116.27 to
**108.37** mean luma — the station byte-guard caught it because the capture frame's
coverage window was identical, so the frames should have matched exactly.

**Root cause:** the cache treated "the matrix was computed" as "the map was
rendered." On the first frame after terrain upload the terrain caster pipeline does
not exist yet (`ensure_shadow_pipeline` runs in `draw_into`'s terrain-forward prep
block, which is AFTER the shadow passes), so `has_shadow_casters` is false and every
shadow pass is skipped — while `update_cascade_splits` had already marked c2 valid
and committed its matrix. Frame 2 then honored the cache and sampled a shadow map
that was **never written** (garbage depth → spurious shadowing → the darker frame).
With every-frame rendering this one-frame latency is invisible; caching made it
lethal.

**Fix — two-phase commit.** `c2_render_pending` is STICKY (set when `cascade2`
changes, cleared only by a pass that actually renders layer 2), `c2_valid` is set by
the PASS, and the skip decision is `c2_rendered = has_shadow_casters && (!c2_valid ||
c2_render_pending)` — computed before the loop, committed after it. The generalized
rule, now in the code comment: never treat a cached GPU resource as filled because
its CPU-side parameters were updated; the write must acknowledge.
- **Layout**: `MainLightUbo` = 3×mat4 + `splits: vec4 (split0, split1, shadow_far,
  fade_start)` + extras — 224 B (buffer resized; the skinned module's dormant inline
  struct updated for offset compatibility, still sampling only c0/c1). The sentinel
  (`extras.x < 0`) is untouched.
- **Memory**: +1 × 2048² Depth32Float layer = +16.8 MB (total 50.3 MB shadow map).

Rejected alternatives, priced: raising `shadow_far` with 2 cascades either balloons
split0 (λ-blend against 3000 → split0 ≈ 322 m → c0 texel ~4× coarser — close-up
regression) or, with split0 pinned, stretches c1 over 86→3000 m (~2 m/texel at ALL mid
distances — the just-approved 100–300 m dune shadows go chunky). Both violate the
"close-up quality is not to regress silently" constraint; the third cascade preserves
both ends for one extra culled depth pass (perf in §5). The dormant 4-cascade
`shadow_csm.rs` was not wired (anti-drift); the live machinery was extended by one
layer.

## 4. Phase 4 — verification at the moving-camera standard

**Motion leg: PASS (assertions armed via `L3A_EXPECT=fixed`).** The same 600 m lateral
sweep at Y≈219, the same world-anchored feature protocol:

| Feature (world) | HEAD (the defect) | FIXED |
|---|---|---|
| feature at (−359, 36, −2255) | `0.0, 0.0, 15.7, 34.0, 39.5, 43.1, 44.6` — appears as the camera approaches (906→604 m) | `40.5, 41.9, 41.4, 40.7, 39.5, 43.1, 44.6` — shadowed at EVERY path point |
| far feature at (−1664, 36, −2280) | (outside HEAD coverage entirely) | `49.2, 50.7, 51.2, 51.1, 50.6, 50.1, 49.9` — constant at view distances **2079→1517 m** |

Shadow presence on a world feature is now a function of the world (sun + relief), not
the camera position. The alt219 radial profile shows continuous shadow effect from the
first relief rows (~370 m) out through the former bubble location and into the
multi-km rows (33–42 luma; the former profile was zero everywhere beyond 512 m). The
farthest-effect assertions (> 2 km at both altitudes) pass.

**Static stations (`l3_proof` re-run, label `l3a_stations_after`):**

- **Close-up quality guard — bit-exact, the strongest possible form**: on the SHIPPED
  build, desert_close, grass_close, grass_mid (default + rake + normals) are
  **0-differing-pixel identical** to the L.3 shipped frames (`l3_bias_rx0005`). c0/c1
  pinning worked exactly as designed — there is no close-up edge-sharpness cost
  because there is no close-up change at all (that content sits below 400 m view
  distance, untouched by the 3-way select, the fade move, and the c2 fit).
- **The y414 acceptance test — L.3's zero-pixel confession — PASSES**: default sun
  147.19 sd 9.90 → **116.31 sd 24.38** (286,116 of 481,962 pixels changed); rake
  130.07 sd 15.37 → **103.71 sd 16.48** (354,917 changed). The distance-heavy framing
  finally shows its relief shadowed (c2 draws 326–389 of 441 chunks there).
- ED-3 normals frames: byte-identical everywhere (geometry debug untouched).

**Sentinel off-state: bit-identical** to L.3's off-leg across all 12 frames (0 differing
pixels, max delta 0), and L.3's off-leg was itself proven bit-identical to pre-L.3 — so
the shadows-off renderer remains bit-exact to the pre-L.3 state transitively across both
beats. Caster passes skipped entirely (stats 0/0 ×3); `c2_valid` is cleared while
shadows are off, so the survey map is rebuilt on the frame they re-enable rather than
being sampled stale.

**Culling counts with the survey cascade** (per station): c0 0–7 and c1 18–33 of
441/169 (unchanged from L.3); **c2 draws 326–389 of 441** desert chunks (the survey
footprint plus its drift padding covers most of the radius-10 world) and 168 of 169
grass chunks — the far cascade is the caster-cost center, which is why it is cached
(§3.1, §5). On frames where c2 is not redrawn, `terrain_shadow_stats()[2]` reports
the counts from its last refresh rather than a misleading 0/0.

## 5. Perf re-gate

Same methodology as L.3 (wall-clock + forced GPU sync, 60 warm-up + 300 timed,
median/p10/p90; TIMESTAMP_QUERY hangs this driver), same two framings, same-build
sentinel on/off as the isolation.

Each row pairs an ON leg with an OFF leg measured in the **same session** (this machine
shows real run-to-run drift — p90s reach 22–30 ms under thermal/scheduler pressure — so
cross-session subtraction would be dishonest):

| Configuration | y414 1920×1080 (on / off → Δ) | desert_close 1024×768 (on / off → Δ) |
|---|---|---|
| L.3 (2 cascades) | 26.950 / 26.184 → **+0.77 ms** | 17.164 / 16.767 → **+0.40 ms** |
| L.3.A, c2 every frame | 29.520 / 26.172 → **+3.35 ms** | 18.605 / 16.669 → **+1.94 ms** |
| **L.3.A shipped** (cached, drift-limited, padded) | 27.430 / 26.551 → **+0.88 ms** | 17.238 / 17.175 → **+0.06 ms** |

(The shipped close-station Δ of 0.06 ms is noise-dominated — that session's OFF leg ran
hot, p90 21.3 ms. Read it as "within noise of L.3's +0.40 ms," not as a speedup.)

**Gate arithmetic.** The every-frame survey cascade costs **+3.35 ms** over
shadows-off at the distance-heavy framing — past the ~2.5 ms line the L.3 gate
established, which is why the cache is not optional. The shipped (cached) config costs
**+0.88 ms** at that framing and is noise-indistinguishable at the close-up — i.e.
**≈ +0.1 ms over L.3's own +0.77 ms**, for coverage that reaches 6× further. **PASS**,
comfortably inside the ~2.5 ms line.

**The honest bound, stated for the budget conscience:** the measured rows use a
stationary camera, so +0.88 ms is the amortized typical case. The hard worst case is a
camera moving fast enough to trip the 300 m drift limit every frame — that is exactly
the every-frame row, **+3.35 ms**. Sustaining it requires >300 m of camera travel per
frame (at 60 fps, 18 km/s), so realistic editor flight sits near the typical figure and
any excursion is bounded and transient. Shipped range: **+0.9 … +3.4 ms**, typical
≈ +0.9 ms at 1080p survey framing.

## 6. Verification suites

| Check | Result |
|---|---|
| naga shader validation | **4 / 4 passed** (the 3-cascade `MainLightUbo` + 3-way select validate in both concats) |
| render lib `shadow` filter | **50 / 50 passed** — including the sentinel contract tests and the cascade-count / light-UBO-size mirrors (updated to 3 layers / 224 B in `renderer_tests.rs`) |
| render lib suite | **1288 passed / 2 failed** — the two standing environmental water Device(Lost) flakes (identical counts to the L.2 and L.3 closes; 23/23 in isolation) |
| editor lib suite | **4039 passed / 0 failed / 5 ignored** (identical to the L.2 and L.3 closes) |
| `cargo check --workspace` | clean — no errors (only the pre-existing deferred warnings) |
| `aw_trace_sync --check` | **in sync** — 26 traces, 133 crates |
| `cargo fmt` | run on both touched crates |
| GPU byte-guards | close/mid stations 0-differing vs L.3; off-state 0-differing vs L.3's off-leg; normals frames 0-differing everywhere |

## 7. Residue

1. **`shadow_far = 3000` is still a cap** — a camera flying above ~1 km altitude or
   framing >3 km of ground will meet the (now 900 m wide, heavily compressed) fade.
   The editor's fog is inert (T2F), so no concealment layer exists behind it. If a
   future regime needs more, the options are a fourth cascade or fog — priced then.
- 2. **c2 updates every frame** like c0/c1 (the simple correct thing, within budget);
   reduced-rate far-cascade updates remain unneeded headroom.
3. The pre-L.3.A `set_cascade_splits` tuning API still exists and is still overwritten
   every frame by `update_cascade_splits` (L.3 residue family, unchanged).

## 8. Files touched

- `astraweave-render/shaders/shadow_common.wgsl` — MainLightUbo 3-cascade layout;
  3-way select; wide fade from `splits.w`.
- `astraweave-render/src/renderer.rs` — 3-layer shadow map + layer-2 view; third
  cascade buffer/BG; `cascade2` fit in `update_cascade_splits`; 224 B light UBO;
  both pass loops iterate 3 layers with per-cascade culling; profiler pass
  `shadow_cascade_2`; stats `[(u32,u32); 3]`; skinned inline struct layout.
- `astraweave-render/src/renderer_tests.rs` — light-UBO size + cascade-count mirrors.
- `tools/aw_editor/tests/l3a_proof.rs` — NEW: the motion-leg + radial-profile proof.
- `tools/aw_editor/tests/l3_proof.rs` — stats print gains c2.
- `docs/audits/L3_OUTCOME.md` — §7/§9 residue items closed by appendix.
