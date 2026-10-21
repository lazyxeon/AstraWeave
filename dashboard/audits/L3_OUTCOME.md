# L.3 — Terrain shadows: cast and receive the CSM (outcome)

> **Beat:** L.3 (lighting lane; follows L.1 exposure/honesty and L.2 IBL) · **Date:** 2026-07-28
> **Base:** `d6f53c00e` (L.2 close) · **Machine:** GTX 1660 Ti Max-Q · Vulkan · driver 592.82 (min-spec)
> **Ratified decision:** terrain casts into and receives from the existing 2-cascade CSM;
> terrain↔terrain only — statics casting onto terrain is the recorded follow-up.
> **Status: COMPLETE, awaiting the director's render gate** — relief shadows relief at
> all close/mid stations under both sun angles, no seam, no acne, no peter-panning
> (bracketed by measured failure legs), off-state bit-exact, perf +0.77 ms worst case
> vs the ~2.5 ms STOP. §4–§7 hold the evidence.

## 0. Summary

- **Caster:** new depth-only terrain pipeline (`terrain-shadow-pipeline`,
  `TerrainMaterialManager::ensure_shadow_pipeline`) draws `tf.chunks` into BOTH cascade
  layers inside the existing per-cascade shadow passes in `draw_into`, with per-cascade
  AABB frustum culling (chunk bounds now computed at upload). Statics keep casting via the
  existing `shadow_pipeline` in the same passes — future static casters are additive.
- **Receiver:** `pbr_terrain_forward.wgsl` gains group(4) = the renderer's EXISTING
  `light_bg` (light UBO + 2-layer depth array + comparison sampler — one owner, §7.7) and
  multiplies the CSM shadow factor into the **direct sun term only**; IBL stays
  unshadowed, so shadowed ground remains skylit. The cascade-select + PCF-3×3
  implementation is the static path's, hoisted verbatim to `shaders/shadow_common.wgsl`
  and consumed by both shaders — no parallel CSM sampling implementation exists.
- **Sentinel:** the `extras.x < 0` shadows-off gate works identically in the terrain
  shader; the off-state is byte-identical to pre-L.3 frames (§7 toggle proof).
- **Preset policy:** `EditorTerrain` re-enables shadows (T2F §5 option A item iii — the
  preset disabled them for exactly this cost, now paid deliberately and measured). This
  also re-arms the static-mesh receiver/caster machinery wherever statics exist —
  pre-existing wiring, disclosed; the pinned stations contain no statics, so the A/B is
  pure terrain↔terrain.

## 1. Phase 0 — premise verification (the beat text needed three corrections)

The ratified work items were exactly T2F §5 option A's pricing and all held. The premise
*narrative* needed corrections, verified against HEAD (`d6f53c00e`) by two independent
recon passes:

1. **The live CSM is 2 cascades, not 4, and its splits are not 10/50/200/1000 m.**
   The live machinery (`renderer.rs`): 2048²×**2-layer** Depth32Float array,
   per-layer render views + D2Array sampling view + LessEqual comparison sampler;
   `update_cascade_splits` runs unconditionally every frame from `update_view`,
   PSSM λ-blend with n=0.5, shadow_far=500 → **split0 ≈ 86.1 m** (the terrain-upload
   path sets λ=0.7; λ=0.75 would give 74.4) and **split1 = 500 m**; sphere-fitted,
   rotation-stable ortho matrices; per-cascade 64-byte UBOs (deliberately separate
   buffers — the write-race invariant); 144 bytes of `light_buf` (2×mat4 + splits +
   extras). The 4-cascade / 10/50/200/1000 numbers describe
   `astraweave-render/src/shadow_csm.rs` — a **dormant parallel CSM implementation**
   with zero production callers (tests/examples only). Logged residue + T2F §9
   correction; L.3 consumed the live machinery and did not touch the dormant module.
2. **"Zero consumers / no shader samples them" was false for statics, true for terrain.**
   The static PBR fs and the (dormant-pipeline) skinned fs both sample the maps
   (PCF 3×3 behind the `extras.x >= 0` gate), and both `render()` and `draw_into` have
   live render-into passes for plane/spheres/external/models. What was true — exactly as
   T2F §1.2 row 3 recorded — is that **terrain** neither casts (chunks live in
   `tf.chunks`, never in `self.models`) nor receives (no shadow bindings in the terrain
   shader), and in the editor terrain scene the `EditorTerrain` preset held shadows off so
   nothing fired.
3. **The editor terrain sun is NOT TimeOfDay.** The terrain-upload block pins the World
   panel's default override (`set_light_direction_override`; bit-identity with
   `DEFAULT_SUN_DIR` L.1-pinned) — **elevation 43.2°, azimuth 51.3°** — which is the sun
   every T-series/L-series frame was shot under. `set_time_of_day` is inert in this scene
   (proven: t=12.0 vs t=16.97 harness legs byte-identical). The raking-sun leg therefore
   drives `set_lighting_params` (the honest L.1 panel path), varying ONLY `sun_dir`:
   elevation **20.0°**, same 51.3° azimuth. Recorded per-leg in the `sun.txt` sidecars.

Defects found in passing (logged, not fixed — see §9): the no-op `set_cascade_extents`
API (fields written, never read — sphere fitting superseded them; two editor call sites
believe they tune cascades, and T2F §5's "80 m extent" figure came from there);
`shadow_slope_scale` stored/set but never uploaded (the caster pipelines' baked
`DepthBiasState` is the real slope bias); `render()`'s caster gate misses the
`shadows_enabled` check `draw_into` has (disabled shadows still pay two depth passes in
the windowed path); the skinned fs lacks the sentinel gate (dormant pipeline).

## 2. Phase 1 — the caster path

- **Pipeline** (`terrain_material_manager.rs::ensure_shadow_pipeline`): vertex-position-
  only (`@location(0)` of the 32-byte `TerrainSplatVertex` stride; positions are already
  world-space, so the VS is one cascade view-proj multiply — no model matrix, no
  instancing), `fragment: None`, Ccw/back-face culling (the engine's uniform convention —
  no front-face-culling precedent exists in the codebase), Depth32Float LessEqual,
  hardware slope-scaled bias (see §5 tuning). Built once, cached.
- **Pass integration** (`renderer.rs::draw_into`): the existing per-cascade shadow passes
  gained a terrain block after the model draws — same pass, same per-cascade
  `shadow_cascade_bgs[idx]` UBO bind groups (reused as-is), pipeline switch, then
  culled chunk draws. `has_shadow_casters` now includes `!tf.chunks.is_empty()`.
  **Statics-additive by construction**: statics already draw into these passes via
  `shadow_pipeline`; a future static-caster beat changes nothing here.
- **Per-cascade culling:** chunk AABBs are computed inside `upload_terrain_chunk` from
  the uploaded world-space vertices (single source of truth — no caller-supplied
  duplicate; `TerrainChunkGpu` gains `aabb_min/aabb_max`) and tested against
  `FrustumPlanes::from_view_proj(cascade_vp)` — the same culling idiom the model path
  uses. Counts are observable via `Renderer::terrain_shadow_stats()` (`[(drawn, total); 2]`)
  and printed by the harness per station (§4).
- **Update rates:** both cascades re-render every frame (the simple correct thing).
  Measured cost is in §6; if a future budget needs it, the far cascade can update at
  reduced rate — noted, not built (and any distance-dependent quality reduction must be
  CONTINUOUS per invariant 19).
- **Dead code removed in the touched region:** the `terrain_c*` cascade-1 skip in both
  shadow passes (no producer of that naming since the 2026-05-08 cluster-path deletion)
  and its false comments ("~16 units", "terrain only in cascade 0").

## 3. Phase 2 — the receiver path

- **One CSM sampling implementation.** The static shader's inline cascade-select +
  PCF-3×3 block was hoisted verbatim into `shaders/shadow_common.wgsl`
  (`MainLightUbo` + `csm_shadow_factor(light, map, sampler, world_pos, frag_dist)`;
  textures/samplers passed as function parameters, consumers declare their own globals —
  the ibl_common.wgsl contract). SHADER_SRC now calls it; the terrain shader calls the
  same function at its group(4) bindings. Behavior-identical for statics: the function
  body is the same code, the `extras.x >= 0.0` gate stays at both call sites (uniform
  branch; the textual gate is contract-tested), naga validates both concats.
- **PCF: 3×3, hardware-comparison taps** (9 `textureSampleCompare`s scaled by
  `extras.x` px radius) — the engine's shipped filter, inherited rather than redesigned.
- **Term discipline:** `lit = brdf · radiance · shadow + compute_ibl(...) · final_ao` —
  the shadow multiplies DIRECT sun only; the IBL indirect terms stay unshadowed (the same
  only-the-right-term rule as L.2's AO-on-indirect-only). Shadowed ground reads as
  skylit, not black.
- **Cascade-boundary quality:** hard select at split0 by view distance, softened by the
  engine's two existing fades (distance fade over the outer 20% of shadow range; UV edge
  fade at each cascade's ortho boundary) — ported unchanged. Seam check at the stations
  in §4.
- **Binding budget:** terrain fragment stage lands at **15/16 sampled textures**
  (L.2 landed at 14/16 reserving exactly this slot). Group(4) is the renderer's existing
  `light_bg` — zero new bind groups, zero new GPU resources; the layout comes from the
  shared `TerrainMaterialManager::create_shadow_bgl` constructor now used by
  `Renderer::new` AND the pipeline tests (no drift). The pipeline layout is now
  5 bind groups — above wgpu's downlevel default of 4; every Renderer-hosting device
  already requests 8, and the render-crate test helper was raised to 5
  (`tests/test_utils.rs`, following `headless_integration.rs`).

## 4. Phase 3 — A/B at the stations

Harness: `tools/aw_editor/tests/l3_proof.rs::l3_ab_stations` (the l2_proof clone — same
worlds, same stations, live `ViewportRenderer::render` + ED-2 capture, `wait_env_bake`).
Two sun legs per station (sun geometry in each label's `sun.txt` sidecar):
**default** = the panel-pinned override, elevation 43.2°/azimuth 51.3° — the angle every
T/L-series frame was shot under; **rake** = elevation 20.0°, same azimuth, via
`set_lighting_params` varying ONLY `sun_dir`. A-side: the noon frames are `l3_before`
(= the L.2 `after_final` state, reproduced bit-level); the rake A-side is the
`l3_off_toggle` leg (certified pre-L.3-identical by the §7 byte proof — `l3_before`'s own
rake files were inert-TimeOfDay duplicates of noon, a harness defect found and fixed in
Phase 0).

Mean luma (Rec.709) / sd, BEFORE → AFTER, at the shipped calibration (§5):

| Station | default sun 43.2° | rake sun 20.0° | casters drawn c0 / c1 (of total) |
|---|---|---|---|
| desert_boundary_y414 | 147.19 sd 9.90 → **147.19 sd 9.90** (0 px differ) | 130.07 sd 15.37 → **130.07 sd 15.37** (0 px differ) | 2 / 23 (441) · rake 0 / 28 |
| desert_close_20m | 146.09 sd 12.79 → **106.88 sd 23.67** | 129.92 sd 17.39 → **95.56 sd 12.04** | 5 / 18 (441) · rake 7 / 33 |
| grass_close_20m | 104.76 sd 15.55 → **65.35 sd 27.31** | 89.63 sd 17.39 → **51.42 sd 12.71** | 4 / 19 (169) · rake 5 / 29 |
| grass_mid_47m | 106.31 sd 11.91 → **78.57 sd 29.41** | 91.44 sd 13.44 → **73.14 sd 23.32** | 4 / 18 (169) · rake 4 / 27 |

- **Relief shadows relief.** The grass 20 m close-up shows a soft-edged hillside shadow
  with its terminator sweeping the frame; grass_mid under the raking sun shows the ridge
  shadow filling the valley — and the framing spans the split0 ≈ 86 m boundary with **no
  visible cascade seam** (the shadow is continuous across it; the engine's distance +
  UV-edge fades are the only softening, as designed). The desert close-ups show the dune
  ridge's cast shadow with a bright sunlit band where the shadow doesn't reach. Shadowed
  ground everywhere remains readable — texture and hue survive because the IBL sky term
  is deliberately unshadowed (§3).
- **sd signatures corroborate**: partial-shadow framings raise sd (desert_close noon
  12.8→23.7, grass_mid noon 11.9→29.4 — a terminator splits the frame); mostly-inside-
  one-shadow framings lower it (desert/grass close rake — the sun-facing modulation is
  suppressed inside the shadow).
- **The y414 boundary framing is unchanged — 0 differing pixels, shadows on vs off,
  both sun angles.** Attribution: the camera sits at ~414 m altitude and the nearest
  framed ground is ≈ 430 m view distance — inside the engine's 400–500 m shadow
  distance-fade band and mostly beyond `shadow_far = 500` (the deliberate cap:
  "shadows beyond ~500 units are imperceptible"), where the far-LOD terrain is also
  smoothest. The caster passes still draw (2 + 23 chunks) — the samples simply resolve
  fully lit. This is the shipped engine convention consumed as-is (anti-drift: splits
  and ranges not redesigned); raising `shadow_far` for distance-heavy editor framings
  is a director option, priced as residue (§9).
- **Culling counts** (per station, printed live by the harness): cascade 0 draws
  0–7 chunks of 441/169 (≥98% culled — its footprint is the 0.5→86 m frustum slice);
  cascade 1 draws 18–33 (≥80% culled). The "441 chunks × N cascades uncullled" death
  the beat feared never materializes: worst-case total caster draws per frame = 40.
- **Normals-debug control**: ED-3 normals frames are byte-identical across every leg
  (0 differing pixels vs `l3_before`) — geometry debug untouched by shadows, and the
  pinned framings contain zero sky pixels, so every changed lit pixel is terrain
  lighting.

## 5. Bias tuning — the deliberate failure pair

Shipped values: **caster hardware bias constant 2 / slope_scale 2.0** (the static
caster pipeline's convention, `ensure_shadow_pipeline`) + **receiver comparison bias
`extras.y = 0.0005`** (the terrain-upload block's `set_shadow_filter`, the delivered
config at editor defaults — §7; the `EditorTerrain` preset now carries the same values
for the explicit-selection path). PCF radius 1.5 px.

The bracket, measured (all legs in `d:/tmp/l3_staging/`; a finding in itself — on
smooth LOD terrain the classic failure pair expresses at the shadow TERMINATOR, not as
open-surface speckle, because the 3×3 PCF footprint plus meters of occluder clearance
put every marginal depth comparison at the boundary):

| Leg | caster bias | receiver bias | Result |
|---|---|---|---|
| `l3_fail_acne` (floor) | 0 / 0.0 | 0.0 (shader floor 1e-5) | **No visible acne at zero bias** — station means move ≤0.8 luma vs shipped (desert_close 106.88→106.07; rake legs identical to 2 dp). The shipped margins are headroom, not a knife edge. |
| `l3_fail_acne_neg` | −2000 / −2.0 | 0.0 | Still no visible failure (sub-pixel terminator shift) — the caster bias unit is tiny on a Depth32Float cascade; ±2000 units ≈ centimeters. Demonstrates the bracket must be probed at failure SCALE. |
| `l3_fail_acne_50k` (margin inverted, the acne-side failure) | **−50,000 / 0.0** | 0.0 | **Over-shadowing demonstrated**: the terminator visibly encroaches into the sunlit band (grass_close 65.35→61.70, desert_close 106.88→103.41; the lit band thins frame-visibly). This is the acne family's first expression on this content — self-comparisons flip at the boundary and the shadow grows past its true edge. |
| `l3_fail_peterpan` (the peter-pan-side failure) | 2 / 2.0 | **0.05** (100×) | **Detachment demonstrated**: ~12 m of world-space comparison bias at cascade-0 depth ranges deletes every shadow whose occluder clearance is under it. grass_close: the hillside shadow that covers ~2/3 of the shipped frame RETREATS to a thin band at the ridge crest, visibly detached from its base (65.35→98.34, i.e. the −39 luma shadow effect collapses to −6); desert_close 106.88→132.38 (−39 → −14); only deep-clearance shadow cores survive (the rake desert framing, occluders >12 m above receivers, is bias-invariant — 95.56 both legs). Frames: `l3_fail_peterpan/*`. |
| **shipped** (`l3_bias_rx0005` ≡ `l3_after_bias_c2s2`, bit-identical) | 2 / 2.0 | 0.0005 | §4's A/B — no over-shadowing, no detachment, terminators anchored at their ridges. |

The shipped values sit demonstrably between the two measured failure modes: the
over-shadow side needs the caster margin INVERTED by ~50,000 constant-bias units before
the terminator visibly encroaches, and the peter-pan side needs 100× the shipped
receiver bias before shadows detach — wide, evidenced margins on both flanks.

## 6. Perf gate (min-spec, wall-clock+sync — TIMESTAMP_QUERY unavailable on this driver)

Method: `l3_proof.rs::l3_perf_stations` — wall time per frame with forced GPU sync
(`device.poll(Wait)`), 60 warm-up + 300 timed, median/p10/p90 (the T.2d.F methodology;
TIMESTAMP_QUERY hangs this driver). Two framings: the distance-heavy y414 boundary at
1080p (the T-series perf framing) and the 20 m close-up (caster cost differs).

**The gate measurement — same build, shadows on vs off through the live sentinel**
(the cleanest isolation: identical code, identical scene, the toggle is the product's
own switch):

| Station | shadows OFF | shadows ON | **added cost** |
|---|---|---|---|
| `perf_boundary_y414` 1920×1080 | 26.184 ms (p10 25.712 / p90 27.296) | 26.950 ms (p10 26.477 / p90 28.117) | **+0.766 ms** |
| `perf_desert_close_20m` 1024×768 | 16.767 ms (p10 16.351 / p90 17.744) | 17.164 ms (p10 16.735 / p90 18.453) | **+0.397 ms** |

Cross-check vs pre-L.3 HEAD (`d6f53c00e`, separate build/run): y414 26.986 ms median —
the L.3 shadows-ON leg (26.950) sits within run-to-run noise of the pre-L.3 frame. (The
HEAD close-station run was scheduler-contaminated — median 33.5 with p10 18.1 / p90
64.6 — which is why the same-build off-leg is the honest baseline for the delta.)

**Gate arithmetic: worst-case added cost 0.77 ms ≪ the ~2.5 ms STOP threshold — PASS,
no options menu needed.** The per-cascade culling is what buys this: ≤40 caster draws
per frame against the feared 441×2. Both cascades update every frame (the simple
correct thing); the reduced-rate far-cascade option remains unneeded headroom.

## 7. Toggle proof

`L3_SHADOWS=off` leg (`set_shadows_enabled(false)` through the live sentinel path,
applied after the terrain upload) vs the pre-L.3 `l3_before` captures, byte-compared:

| Frame class | Result |
|---|---|
| desert_close_20m + ALL normals frames (5) | **0 / 786,432 (or 481,962) differing pixels, max delta 0** — bit-identical |
| desert_boundary / grass_close / grass_mid (noon) | 3 / 2 / 1 differing pixels, **max delta 1 LSB** (cross-process float nondeterminism; the L.1 criterion is ≤1 LSB) |
| rake frames | expected mismatch — `l3_before`'s rake files were the inert-TimeOfDay noon duplicates (Phase-0 harness defect); the off-leg's rake frames ARE the certified pre-L.3 rake baseline (same shader path proven bit-identical at noon; with the sentinel set, `shadow` stays 1.0 and block 6 is arithmetically identical to the pre-L.3 shader at any sun angle) |

With shadows off the caster passes are skipped entirely (`has_shadow_casters` gate;
stats read 0/0) and the terrain fragment gate skips all 9 comparisons per pixel —
the off state is the pre-L.3 renderer, bit-exactly.

**Configuration truth discovered by the bias legs (§5):** two legs differing only in the
`EditorTerrain` preset's `set_shadow_filter` bias (0.005 vs 0.0005) rendered
**bit-identical across all 12 frames** — because the editor's default path never applies
that preset: adapter init applies `GameQuality`, and the terrain-upload auto-switch to
`EditorTerrain` is gated on `quality_preset != GameQuality`. The delivered filter config
in every editor terrain scene at defaults is the terrain-upload block's
`set_shadow_filter(1.5, 0.0005, 1.0)` (pcf 1.5 px, receiver bias 0.0005, slope arg
write-only) with `set_cascade_lambda(0.7)` (split0 ≈ 86.1 m). Shadow ENABLEMENT at
defaults likewise comes from GameQuality-at-init, not from the preset flip — the
`EditorTerrain` change matters on the explicit-selection path (it must not silently
kill terrain shadows, per the ratified T2F A(iii)). This also corrects T2F §1.2 row 3's
"terrain upload auto-applies the EditorTerrain preset" — mechanism real, unreachable at
defaults; pre-L.3 the distinction was unobservable (no terrain shadow surface existed).

## 8. Verification

| Check | Result |
|---|---|
| naga shader validation (`shader_validation`) | **4 / 4 passed** (terrain concat now constants → brdf_common → ibl_common → **shadow_common** → stochastic_tiling → pbr_terrain_forward, mirrored in the test; `shadow_common.wgsl` registered as a concatenation fragment) |
| render lib suite | **1288 passed / 2 failed** — the two failures are the standing environmental water Device(Lost) flakes under full-suite churn (L.1/L.2 record; re-certified this session: `--lib water` in isolation **23 / 23 passed**). Identical counts to the L.2 close. |
| render lib `shadow` filter | **50 / 50 passed** — includes the sentinel contract tests (`test_shadow_override_sentinel_logic`, `test_shader_has_conditional_shadow_not_hardcoded` — the textual `extras.x >= 0.0` gate survives the shadow_common hoist) and `test_shadow_bind_group_layout` against the shared-constructor BGL |
| editor lib suite | **4039 passed / 0 failed / 5 ignored** (identical to the L.2 close) |
| `cargo check --workspace` | clean — no errors; the only warning is the pre-existing deferred `TranslateGizmo` unused import (CLAUDE.md Known Build Issues) |
| `cargo run -p aw_trace_sync -- --check` | **in sync** — 26 traces validated, 133 crates (render trace v1.17, aw_editor v1.14, terrain_materials v1.6 front-matter consistent) |
| `cargo fmt` | run on both touched crates (`astraweave-render`, `aw_editor`) |
| L.1 controls re-certification | **2 / 2 passed** under the shadowed default state (each lighting beat re-proves the controls): defaults-push honesty **0 differing pixels / 0 LSB**; exposure 0.5/1.35/3.0 → 51.96/106.88/161.82 mean luma, all pairs 100% differing; late-push lands at 45.53 vs the ~107 dropped state (threshold 140). |
| GPU byte-proofs | §7 toggle proof (0-px off-state); §5 failure-bracket legs; §4 A/B + normals masks |

## 9. Residue (logged, unowned — director ratifies ownership)

1. **Dormant parallel CSM** (`shadow_csm.rs`, 4-cascade `CsmRenderer`, zero production
   callers; plus never-included `shaders/shadow_sampling.wgsl` PCSS/Poisson and
   `shaders/pbr.wgsl` duplicate sampling code) — architecture-drift cleanup candidate.
   The render trace §5 lists `shadow_csm.rs`/`shadow_quality.rs` "Active" with empty
   notes — corrected to dormant in the trace this beat.
2. **No-op `set_cascade_extents`** (`cascade0_extent`/`cascade1_extent` written, never
   read) + its two `engine_adapter.rs` call sites — settable-but-unobserved API; remove
   or wire. Same family: `shadow_slope_scale` stored/set but never uploaded (the caster
   pipelines' `DepthBiasState` is the real slope bias).
3. **`render()` gate asymmetry**: `has_shadow_casters_r` omits `self.shadows_enabled`,
   so the windowed path pays both depth passes with shadows disabled. One-line fix in the
   statics lane; terrain does not draw in `render()` at all (parity gap recorded — the
   terrain caster path, like the terrain forward path, is `draw_into`-only).
4. **Skinned fs lacks the sentinel gate** (samples shadows unconditionally when its
   dormant pipeline would ever draw) — statics/skinned lane.
5. **The terrain-upload block's `set_shadow_filter(1.5, 0.0005, 1.0)` is overridden**
   moments later by the `EditorTerrain` preset apply (which now owns the filter values,
   set explicitly in L.3); under a user-chosen GameQuality preset the upload's values
   win instead. Config-layering fog; one owner should remain.
6. **`PROJECT_STATUS.md` has never recorded the terrain/lighting lane** (stale at
   2026-06-10; no L.1/L.2/L.3/T-series entries). Surfaced as a director-level decision —
   not backfilled inside this beat (scope discipline).
7. **`shadow_far = 500` + the 400 m fade leave distance-heavy editor framings
   shadow-free** (the y414 station renders 0 changed pixels — §4). The cap is the
   engine's deliberate convention ("imperceptible beyond ~500"), tuned for ground-level
   play, not for a 414 m-altitude editor survey camera. Raising it (or scaling with
   camera altitude) is a quality/perf trade the director owns; cascade-1 texel density
   drops proportionally.
8. **The upload block's `set_cascade_lambda(0.7)` vs the presets' `0.75`** — two owners
   for the split parameter (same family as residue 5); the delivered value in terrain
   scenes is 0.7 (split0 ≈ 86.1 m).

## 10. Files touched

- `astraweave-render/shaders/shadow_common.wgsl` — NEW shared fragment.
- `astraweave-render/shaders/pbr_terrain_forward.wgsl` — group(4) + shadow×direct.
- `astraweave-render/src/renderer.rs` — SHADER_SRC concat + inline PCF → shared call;
  shared `create_shadow_bgl` use; `shadow_bgl_light` retained; `TerrainChunkGpu` AABBs;
  caster block in `draw_into`'s cascade passes + gate + stats; group(4) bind;
  cascade-splits comment corrections; dead `terrain_c*` guards removed.
- `astraweave-render/src/terrain_material_manager.rs` — `TERRAIN_SHADOW_SHADER`,
  `ensure_shadow_pipeline`/`shadow_pipeline()`, `create_shadow_bgl`,
  `ensure_forward_pipeline` + shadow_bgl param, concat + shadow_common.
- `tools/aw_editor/src/viewport/engine_adapter.rs` — `EditorTerrain` preset: shadows ON
  + explicit filter params; honest preset docs.
- `tools/aw_editor/tests/l3_proof.rs` — NEW harness (A/B + rake + toggle + perf + stats).
- `astraweave-render/tests/{shader_validation,terrain_splat_pipeline,test_utils}.rs` —
  concat mirror + skips; shared-BGL call sites; bind-group limit.
- `docs/audits/T2F_LIGHTING_RECON.md` — §7 item 6 struck (CLOSED by L.3); §9 appended.

---

## Appendix (L.3.A, 2026-07-29) — the render gate rejected; the boundary fixed

The director's render gate REJECTED this beat from a moving survey camera: "the
shadows seem to be attached to the camera boundary instead of the world." The cause
was §4's own residue item — the 500 m `shadow_far` cap + 100 m fade, a camera-centered
bubble that the static pinned stations could not see (y414's zero-pixel result was its
fingerprint). §9 residue item 7 is CLOSED by L.3.A: a third (survey) cascade extends
coverage to 3000 m with the fade widened to the last 30%, c0/c1 kept bit-identical
(close/mid frames byte-match this document's A/B), and the y414 station now shows its
relief shadowed (147.19→116.27 default / 130.07→103.71 rake). The verification bar
this minted: camera-fitted systems get camera-MOTION legs (`l3a_proof.rs`). Full
record: `docs/audits/L3A_OUTCOME.md`.
