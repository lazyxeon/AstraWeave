# T.2d — The camera-light defect: diagnosis (FIXED as of §11/T.2d.F — awaiting the director’s closing check)

> **STATUS, fourth pass (T.2d.F, 2026-07-25): FIXED.** The director ratified §10.7 option A the same
> day; the tiers are deleted, the boundary signature and dithering are measured gone, close range is
> untouched to 1 LSB, and min-spec is 1% faster. **See §11** (closing pointer) and
> `docs/audits/T2DF_OUTCOME.md` (the evidence). What remains open: the §2.1 near-field gradient, the
> §9.1 far-field normal-variance gradient, and the director's closing station check.
>
> **STATUS, third pass (2026-07-25):** the boundary is **identified** — it is `compute_material_lod`'s
> **LOD1|2 threshold, pixel footprint = 2.0** (`brdf_common.wgsl:63`), matched in both director frames
> to within 4 screen pixels. It is a **detail** edge, not a brightness edge, which is why §2.2 and §9.2
> could not see it with a row-mean-luminance metric. At this pass no fix was applied —
> `compute_material_lod` was called engine-wide, so the choice was a STOP-with-options (§10.7).
> Read **§10 first**; §§1-9 are the two earlier passes and their retired hypotheses, kept as the ledger.
>
> **Beat:** T.2d (terrain series) · **Date:** 2026-07-25 · **Baseline commit:** `c2dbf8400`
> **Symptom (director, 2026-07-25):** *"the camera itself is a light source — as I get really close to the terrain it gets brighter and casts shadows at the borders of the light boundary like a light source."*
> **Outcome:** the phenomenon is **partially reproduced**. Camera-dependent shading is **confirmed and measured**. The *dramatic* part of the description — the bright region with a visible boundary — **did not reproduce** in the offscreen path. **No fix has been applied**, because nothing has been convicted at the magnitude the symptom describes.
> Frames + CSVs: `d:/tmp/t2d_staging/{head,lod_const,flat_normal,nofog}/` (session-local).
> Anti-drift honoured: no material acquisition, no shader tuning retained, no scatter/water/hex work. Every shader edit in this beat was a temporary A/B leg and **all were reverted** (`git diff astraweave-render/` is empty at the commit).

---

## 0. Summary — what is and is not established

**Established (measured):** shading in the terrain path *is* camera-dependent. Holding the world fragment fixed at screen centre and moving only the camera, its luminance rises **+5.9%** as the camera approaches (110 m → 12 m). Under a correct lighting model that number must be zero. This is a real defect and it means close-up material judgments are made under a camera-varying exposure.

**Not established:** that this is *the* thing the director saw. The measured effect is a **smooth ~6% gradient with no boundary**. The largest single-row luminance step found anywhere in the sweep is ~1.1 out of ~62 (1.8%), and the position of that step **moves with terrain content, not with the camera** — i.e. it is a world feature, not a camera-anchored ring.

I am not fixing anything on that basis. A 6% smooth drift does not read as "the camera is a light source", and per this beat's own rule a fix applied before the causal chain is evidenced is the named failure mode. §6 is the STOP.

---

## 1. The instrument

`tools/aw_editor/tests/t2d_camera_light.rs` (new, permanent). Renders the editor's own viewport path offscreen at exactly-specified cameras — the same contract as the T.2a station harness, and necessary for the same reason: the editor cannot pin a camera (T2A_OUTCOME.md §1). Verified to call the **same** `ViewportRenderer::render` the live widget calls at `widget.rs:875`; the `None`/`false` arguments are gizmo/grid/physics overlays, not post-processing.

Three experiments:

- **A — distance sweep.** Focal point held fixed, so the **same world fragment** sits at screen centre in every frame. If its luminance moves, shading is camera-dependent. This is the decisive test.
- **B — lateral translation.** Camera and focal translated together, so camera-relative geometry is identical and only world content changes. A boundary that stays at the same screen row is camera-anchored; one that tracks terrain is world-anchored.
- **C — row profile.** Mean luminance per screen row, plus the largest adjacent-row step. A hard `if`-threshold shading term produces a step; fog and smooth falloffs do not.

### 1.1 A methodological failure worth recording

The first run of this harness reported four clean-looking rows at distances 4/6/8/12 — **byte-identical metrics across four different camera distances**, which no real camera move can produce. The camera was *inside the terrain* and every frame was pure fog.

Cause: `OrbitCamera` places the eye at `focal + distance·dir(yaw,pitch)`, so altitude above the focal point is `distance · sin(pitch)`. At the 35° pitch first chosen, a 4 m orbit is 2.3 m over the surface — well inside local relief (base amplitude 50).

Two changes followed, both permanent: the sweep uses an 80° look-down (altitude = 0.985 × distance), and every row now reports **`sky_frac`** and self-flags `<-- DEGENERATE: camera inside terrain, NOT a measurement` above 0.9. Degenerate frames can no longer be silently reported as data. The 3/5/8 m rows below are left in, flagged, as the honest record of the geometric floor at this focal point.

---

## 2. Phase 1 — characterization

### 2.1 Experiment A — camera dependence is real (the decisive result)

Same world fragment at screen centre in every row; only the camera moved.

| distance (m) | 3 | 5 | 8 | 12 | 16 | 20 | 30 | 45 | 70 | 110 |
|---|---|---|---|---|---|---|---|---|---|---|
| centre luma | *degen* | *degen* | *degen* | **65.58** | 64.78 | 64.13 | 63.93 | 62.82 | 61.65 | **61.72** |

**Monotonic: +5.9% brighter at 12 m than at 110 m, for the same piece of ground.** That is the camera-dependence the director's description implies, confirmed numerically.

**Fog cannot explain it, and this was tested rather than argued** (§3.5): the fog colour is *brighter* than the terrain, so fog would make distant fragments brighter, not darker — the observed sign is opposite.

### 2.2 Experiments B and C — the boundary did NOT reproduce

Largest adjacent-row luminance step, by lateral offset (identical camera geometry, different world content):

| lateral offset (m) | 0 | 15 | 30 | 60 |
|---|---|---|---|---|
| max row step (luma) | 0.89 | 0.63 | 0.76 | 0.68 |
| **at screen row** | **305** | **328** | **378** | **283** |

**Verdict: the largest step is WORLD-anchored, not camera-anchored.** The row wanders with terrain content. A camera-anchored ring would have pinned the same row in all four frames, because the camera-relative geometry is identical across them.

And the magnitude is not a boundary in any case: ~0.6–1.1 luma against a frame mean of ~62, i.e. **under 2%**. Visual inspection of the frames confirms no bright region and no edge; what *is* visible up close is the hex-tile lattice, which is a separate known item and explicitly out of scope here.

**So: brightening — reproduced and camera-anchored. Boundary — not reproduced.**

---

## 3. Phase 2 — hypothesis census, with verdicts

Each A/B leg is a temporary edit to `pbr_terrain_forward.wgsl`, re-run through the same harness, then reverted.

### 3.1 A camera-attached light ("headlight") in the editor's scene setup — **EXONERATED**

By code read, three independent ways:

- `EngineRenderAdapter::set_lighting_params` (`engine_adapter.rs:3798`) writes only ambient colour/intensity and a **sun direction** override. No positional light.
- No point light exists anywhere in the editor's viewport layer: `rg 'point_light|PointLight|light_pos'` over `engine_adapter.rs` returns nothing.
- `ViewportRenderer::set_scene_lights` (`renderer.rs:1477`) is an explicit **no-op** — entity `Light` components are collected in `widget.rs:777-800` and then dropped. Even those are entity-positional, never camera-derived.

Nothing in the editor derives a light's position or direction from the camera.

### 3.2 The LOD-tiered BRDF (`compute_material_lod` → Kulla-Conty multiscatter) — **IMPLICATED for a brightness offset, EXONERATED as the distance trend**

This was the strongest-looking suspect on inspection and it failed its own test, which is exactly why the test was run.

`brdf_common.wgsl:57` selects a shading tier from `length(fwidth(world_pos))` — the world-space pixel footprint, which is camera-distance dependent — and **LOD 0 adds a multiscatter energy term that LOD 1 does not have** (`brdf_common.wgsl:124-140`). Hard `if` thresholds at 0.5 and 2.0 on a continuous quantity. That is a textbook recipe for a camera-anchored ring, and it is called from three places: `renderer.rs:231` (static PBR), `renderer.rs:571` (skinned PBR), and `pbr_terrain_forward.wgsl:369` (terrain) — so it is **engine-wide, not editor-only**.

A/B leg: `let mat_lod = 1u;` (constant).

| distance | 12 | 20 | 45 | 110 | trend 12→110 |
|---|---|---|---|---|---|
| HEAD | 65.58 | 64.13 | 62.82 | 61.72 | **−5.9%** |
| constant LOD | 59.88 | 58.48 | 57.14 | 55.91 | **−6.6%** |

**The trend survives at full strength.** Forcing the LOD constant did not remove the camera dependence; it only shifted the absolute level (LOD 0's multiscatter is worth **+9.5%** at 12 m and **+10.4%** at 110 m).

That the offset is nearly *equal at both ends* is the informative part: the screen-centre fragment is in LOD 0 at **both** 12 m and 110 m, i.e. the sweep never crosses a threshold. The LOD boundary is real but sits far outside the tested range — order 240 m perpendicular at this FOV and resolution, and much nearer at grazing incidence where the footprint is divided by `cos(incidence)`.

**Verdict: convicted of adding a ~10% camera-tier-dependent energy step, acquitted of producing the measured gradient. Its threshold ring remains a live candidate for the director's boundary — see §6.**

### 3.3 Mip-gated / normal-map terms (`NORMAL_XY_STRENGTH`) — **IMPLICATED (partial)**

A/B leg: `let N = N_geom;` — normal mapping removed entirely, which subsumes `NORMAL_XY_STRENGTH` and all mip-dependent normal behaviour.

| distance | 12 | 20 | 45 | 110 | trend |
|---|---|---|---|---|---|
| HEAD | 65.58 | 64.13 | 62.82 | 61.72 | −5.9% |
| geometric normal | 64.01 | 63.69 | 62.95 | 61.64 | **−3.7%** |

The trend **shrinks but survives**, and the near field flattens markedly (HEAD falls monotonically 65.58→63.93 over 12–30 m; with the geometric normal it is essentially flat at 64.01/63.69/63.98). So the normal-map mip chain owns roughly the near-field third of the effect and something else owns the rest.

**Verdict: implicated, partial cause. Not sufficient alone.**

### 3.4 IBL / ambient / specular with a bad view-dependent factor — **NOT EXONERATED, NOT TESTED**

The ambient term (`pbr_terrain_forward.wgsl:379-380`) is `ambient_color · ambient_intensity · 0.35 · AO` with no view term, so it cannot vary with camera position. The specular path is inside `evaluate_brdf_lod`, which §3.2's constant-LOD leg held fixed while the trend persisted — but that leg did not isolate `V` itself. The residual −3.7% after removing normal mapping is unattributed and this is where it most likely lives.

**Verdict: open. This is the strongest remaining lead for the measured gradient**, and the honest reason it is unresolved is that I stopped on the STOP rather than continuing to bisect a ~6% effect that does not match the reported symptom.

### 3.5 Fog / exposure / tonemap — **EXONERATED**

A/B leg: fog term removed.

Result: **numerically identical to HEAD in every row** (65.5821 / 64.7770 / 64.1251 / 63.9280 / 62.8191 / 61.6543 / 61.7212). Fog contributes exactly nothing over 12–110 m.

That identity is also a free control: two independent full runs producing byte-identical numbers establishes the harness is deterministic, which is what makes the other legs' differences trustworthy.

Exposure and tonemap are separately excluded by configuration: `PostProcessChain::default()` (`hdr_pipeline.rs:110-125`) has `auto_exposure_enabled: false`, `bloom_enabled: false`, `ssao_enabled: false`, and **no editor code path enables any of them** — the only `bloom_enabled` writes in the editor are panel-local state in `post_process_panel.rs` / `project_settings_panel.rs` that never reach `set_post_process_chain`.

**Verdict: exonerated.** (SSAO was worth checking because "brighter up close + shadows at borders" fits it well — a screen-space AO radius covers less world space as you approach, and SSAO haloes at depth discontinuities. It is simply not enabled.)

---

## 4. Phase 3 — conviction: NOT REACHED

The causal chain this beat asked for cannot be honestly written, because the two halves of the symptom do not both reproduce:

| element of the director's description | status |
|---|---|
| "as I get really close … it gets brighter" | **reproduced**, +5.9% at fixed world point, camera-anchored |
| "casts shadows at the borders of the light boundary" | **not reproduced** — largest step <2%, and world-anchored, not camera-anchored |
| "like a light source" (a bounded bright region) | **not reproduced** — no bounded region found at any tested station |

Two contributors to the *brightening* are identified and separated (§3.2 ~10% tier offset, §3.3 ~2.2 points of the 5.9% gradient), with a residual that is not yet attributed. None of them produces a boundary.

**Why I am not fixing the identified pieces now:** removing the multiscatter tier step or flattening `NORMAL_XY_STRENGTH` would each change close-up appearance measurably, and would be tuning against a defect I have not shown is the one the director is looking at. If the real cause turns out to be motion- or configuration-dependent (§6), those edits would have moved the materials for nothing — and material appearance is precisely what T.2a and T.2c spent two beats stabilising.

---

## 5. Phase 5 — the re-judgment list (independent of the STOP)

This list stands **regardless** of what the remaining diagnosis finds, because §2.1 already proves close-up shading varies with camera distance. Judgments made at an unrecorded camera distance carry an unrecorded ±6% exposure. **I have re-tuned nothing; the list is the deliverable and the director decides what gets re-judged.**

1. **The T.2a "flat and shiny up close" complaint** — the complaint that started the whole terrain-material line. Made at close range, where luminance runs ~6% hot relative to the far field.
2. **T.2c's three close-up reads** (grassland shards gone, desert relief, tundra structure). The *structural* findings are safe — a black shard is present or absent, and the desert's 4.3× Laplacian gain is a contrast measure, not a brightness one. The **brightness** findings are not: specifically the T.2c §4.3 report that **the tundra now reads too dark**. That was judged at station distance 20, inside the affected range.
3. **The T.2c grassland colour verdict** ("reads olive/brown rather than green") — a tone judgment at close range.
4. **Every T.2a shader-constant A/B** (`NORMAL_XY_STRENGTH` 1.8→1.4, hex pow 4→2). These were measured at *pinned, identical* camera stations per leg, so the **comparisons remain valid** — both legs carried the same camera-dependent offset and it cancels. Their *absolute* appearance verdicts do not.

The pinned-station method is what protects items 4 and most of 2: because T.2a and T.2c registered every leg to identical cameras, their A/B deltas survive this defect even though their absolute reads do not.

---

## 6. Phase 4 — STOP, with what I need and what the options are

The beat directs a STOP when the choice is not obvious. It is not obvious, so:

### 6.1 What would let me finish the diagnosis

1. **A screenshot of the effect**, and the approximate camera distance. One frame settles whether the boundary is a ring, a step, or a gradient, and gives its radius — which discriminates §3.2's LOD threshold (a ring at a fixed world radius, order 240 m perpendicular / much closer at grazing) from everything else.
2. **Does it appear while the camera is MOVING, or does it persist when parked?** This is my leading hypothesis for the non-reproduction: **TAA is enabled by default** (`hdr_pipeline.rs:119`, in the pass list at `:152`) and my harness renders *static* cameras with a two-frame settle, so it cannot exhibit motion-dependent history artifacts. A TAA reprojection fault would brighten during approach and produce edges at disocclusions — which matches "brighter as I get close" and "shadows at the borders" far better than anything I measured. If the effect vanishes when you stop moving, that is close to conclusive.
3. **Which world/biome and roughly where**, so I can re-pin a station on the ground you were actually looking at.
4. **Does it affect props and water, or terrain only?** Phase 1 item 3. I can predict but not yet confirm: `compute_material_lod` is called from the static and skinned PBR shaders too (`renderer.rs:231`, `:571`), so a LOD-threshold cause *should* affect props; a terrain-shader-only cause should not.

### 6.2 Options on the piece that IS convicted, when you want it dealt with

The LOD tier step (§3.2) is a genuine defect independent of the boundary question: shading quality that changes discontinuously with camera distance means **any** close-up judgment sits on a different BRDF than the far field. Options, with costs:

| option | effect | cost |
|---|---|---|
| **A. Force LOD 0 everywhere in the terrain shader** | Removes the tier discontinuity; close-up appearance unchanged, far field gains multiscatter (~+10%) | Loses the ALU saving the tiers were added for; far-field terrain brightens ~10%, which rots T.2a/T.2c far-field frames |
| **B. Smoothly blend the multiscatter term across the threshold** | Removes the visible step while keeping most of the saving | Real shader work; needs its own A/B |
| **C. Leave it, document it as a known camera-dependence** | No churn | Every future close-up judgment keeps the ±10% tier offset |

I recommend deciding this **after** §6.1, not before: if the reported symptom turns out to be TAA, option B is wasted work on the wrong term, and if it turns out to be the LOD ring then the choice is A or B on evidence rather than on my guess.

---

## 7. Verification

| rung | result |
|---|---|
| `cargo fmt -p aw_editor` | clean |
| `cargo check -p aw_editor --tests` | **exit 0** (pre-existing warnings only) |
| `git diff astraweave-render/` | **empty** — all four A/B shader legs reverted |
| T.2d harness, 4 legs | 1 passed / 0 failed each; `head` and `nofog` byte-identical (determinism control) |
| `cargo test -p aw_editor --lib palette_remap` | **8 passed; 0 failed** (4,022 filtered) |
| `cargo test -p aw_editor --lib canonical_terrain_pack` | **2 passed; 0 failed** (4,028 filtered) |

No regression test is added: this beat convicted no defect, and a test asserting the *current* camera-dependent behaviour would pin the bug rather than catch it. The harness itself is the permanent instrument (ED-1 precedent) — it is `#[ignore]`d like the T.2a stations, so it does not slow CI, and it now self-flags degenerate frames.

## 8. Residue

- The **residual unattributed −3.7% gradient** (§3.4) is the open technical thread. `V`-vector normalization and the specular path are where I would look next.
- The **LOD threshold ring** is predicted but never observed; §6.1 item 1 would confirm or kill it in one frame.
- **The geometric floor at the T.2a station-01 focal is ~12 m** — closer cameras are inside local relief. A genuine "really close" sweep needs a focal on flat or raised ground; `sky_frac` now makes the failure self-evident rather than silent.
- Not investigated: whether `camera-relative` (a declared feature of `astraweave-render`) is active in this path and interacts with `world_pos`-keyed terms.

---

# 9. Continuation (director, 2026-07-25) — census gap closed, boost hypothesis falsified, STOP for ED-2

Three director corrections drove this pass: the symptom is **stationary-visible** (TAA demoted), the observations are from **camera Y 414.5 / 536.2** over **Desert** (not my 12–110 m range), and the sign is **nearer DARKER / farther BRIGHTER** — opposite to §2.1's gradient. Treating them as two phenomena, as instructed.

## 9.1 Range correction — the director's SIGN is reproduced

Experiment D added: Desert, camera placed at the director's two altitudes, profiled out past 1000 m.

| ground distance | 200–300 | 400–500 | 600–700 | 800–900 | 1000–1100 m |
|---|---|---|---|---|---|
| mean luma (camY 414.5, pitch 30°) | 104.73 | 107.56 | 111.19 | 114.12 | **114.63** |

**+9.5% from 250 m to 1050 m — nearer darker, farther brighter.** This is the director's sign, reproduced, and it is *opposite* to the §2.1 Mediterranean near-field gradient (+5.9% toward the camera). Confirms the two-phenomena split.

## 9.2 But the BOUNDARY still does not reproduce — and my earlier method could not have seen one

A methodological correction first: §2.2's row-profile **averages each screen row**, but iso-distance contours on a ground plane are curves, so a ring is smeared into a ramp by row-averaging. That method could not have detected the reported edge.

Redone properly: every pixel's ray is intersected with the ground plane and luminance binned by **true 3D camera-to-fragment distance** — the quantity `compute_material_lod` and the CSM splits are keyed to.

Result at camY 414.5 / pitch 30°, 25 m bins: largest binned step **−2.56 luma at ~962 m**, and the sequence oscillates (−1.55, +0.54, +0.29, −0.06, −1.28, +0.93, +1.33, …). That is dune and biome content, not a monotone edge. **No hard camera-anchored boundary at the director's altitudes either.**

## 9.3 The mip0-boost hypothesis — FALSIFIED, three independent ways

1. **It is not mip-gated.** `NORMAL_XY_STRENGTH` has exactly four occurrences workspace-wide (`pbr_terrain_forward.wgsl:327,331,333` plus one doc reference). It is an unconditional `let` inside the per-layer loop, multiplied into `n_ts.xy` for every fragment at every distance. There is no mip query, LOD branch, or distance term. The premise "at the mip0→mip1 crossover the boost stops" does not hold in this code.

2. **The predicted crossover is off by two orders of magnitude** — the calculation requested, which turns out to falsify rather than confirm. At tiling 128 over a 512 WU chunk (one repeat per 4 m), fovy 60°, 768 px:

   | array | texel density | mip0→mip1 crossover |
   |---|---|---|
   | albedo 1024² | 256 texels/m | **2.60 m** |
   | aux (normal/ORM) 512² | 128 texels/m | **5.20 m** |

   The director observes at **400–1500 m** ground distance — roughly eight mip levels past mip 0. Even if the boost *were* mip0-gated, its boundary would be a ~3–5 m ring at the camera's feet, not an edge at hundreds of metres.

3. **Neutralizing it changes nothing at those altitudes.** A/B leg `NORMAL_XY_STRENGTH = 1.0`, Experiment D re-run:

   | station | HEAD mean / max step | boost = 1.0 |
   |---|---|---|
   | camY 414.5, pitch 30° | 121.315 / 3.935 | 121.675 / 3.966 |
   | camY 536.2, pitch 30° | 123.768 / 3.302 | 124.132 / 3.389 |

   Under 0.3% difference. Expected on reflection: at 400–1000 m the sampled normals are heavily mipped toward flat, so scaling a near-zero XY by 1.4 versus 1.0 does almost nothing. **The boost is a near-field term; the observation is far-field.**

The director's underlying *physical* intuition — a hard mip transition amplified by a normal-detail term — was worth testing and is also ruled out: `terrain-layer-sampler` (`terrain_material_manager.rs:1405-1417`) uses `mipmap_filter: Linear` with `anisotropy_clamp: 8`, so mip transitions are trilinear-interpolated and cannot hard-edge.

## 9.4 The owed census: shadows and every other distance-tiered system

| system | camera-derived? | range | can it ring the terrain? |
|---|---|---|---|
| **CSM** (`shadow_csm.rs`) | **YES** — `update_cascades(camera_pos, camera_view, camera_proj, near, far)` | splits **10 / 50 / 200 / 1000 m** | **NO — terrain never samples it.** `pbr_terrain_forward.wgsl` contains zero shadow sampling; the only two occurrences of "shadow" are comments saying there are none (`:20`, `:374`). |
| `compute_material_lod` tiers | YES (`fwidth(world_pos)`) | thresholds 0.5 / 2.0 footprint | Possible in principle; §3.2 showed the sweep never crossed a threshold, and the boundary would sit ~240 m perpendicular |
| terrain layer mips | YES | crossover 2.6 / 5.2 m | No — trilinear (§9.3) |
| splat textures | n/a | — | **No — `mip_level_count: 1`** (`terrain_material_manager.rs:825,1165`), so the splat sampler's `mipmap_filter: Nearest` has no mip chain to step between |
| fog | YES | — | Exonerated §3.5 (byte-identical with it removed) |

**The CSM finding is the sharpest thing in this pass and it is a genuine contradiction to sit with:** the cascade system *is* camera-anchored and its 1000 m split falls squarely inside the director's observed 400–1500 m range — the right shape, the right range, and the symptom's own word ("shadows"). But the terrain shader cannot express it. Either the boundary is on something other than terrain (props, water, or a pass I have not instrumented), or the editor's live path shades terrain differently from this shader. Both possibilities point at §9.5.

## 9.5 STOP — this needs ED-2 first

Everything above is the **offscreen** path. The director's Gap #2 is now the leading explanation and I cannot close it with the current tooling.

The offscreen harness reproduces the *gradient* and the *sign* but not the *boundary*. That split is exactly the ED-1 / T.W.1 precedent the director cited — headless `render()` being a no-op, the editor rendering via `draw_into`. To measure in the editor's own path I need to pin a camera to the observed coordinates and capture a frame from the live editor, and today:

- `OrbitCamera::set_yaw` / `set_pitch` do not set the smoothing targets, so any programmatic camera restore drifts back within ~50 ms (T2A_OUTCOME.md §3.4);
- there is no screenshot command — `polish.rs`'s `include_screenshot` / `screenshot: Option<PathBuf>` are settable-but-never-read dormant fields (T2A_OUTCOME.md §1).

**That is ED-2, which the director has queued and offered to run first. I am stopping here rather than building a parallel one-off capture path** — a second camera-pinning mechanism is precisely the duplicate-implementation trap the CLAUDE.md scope rules forbid, and ED-2 produces the reusable version.

**Also still needed:** the two Desert frames. They are not at `d:/tmp/t2d_staging/` (which holds only my six A/B legs), and there are no new PNGs under the repo, `Pictures`, `Desktop`, or `Downloads`. A path would let me measure the boundary's screen position and, with the Camera readout, convert it to a ground distance — which discriminates the CSM 1000 m split from the `compute_material_lod` ~240 m threshold in a single measurement.

## 9.6 What is now established, regardless

- The **+5.9% near-field gradient** (§2.1) and the **+9.5% far-field gradient with the director's sign** (§9.1) are both real, both camera-dependent, and both distinct from the boundary.
- The **+9.5% multiscatter tier step** (§3.2) stands as a real defect.
- `NORMAL_XY_STRENGTH` is **exonerated** for the reported symptom — and, usefully, is confirmed to be a near-field-only term, which bounds what T.2a's 1.8→1.4 change could ever have affected.
- No fix applied. Options for the one convicted item remain as §6.2.

---

# 10. Third pass (2026-07-25) — the boundary IDENTIFIED, the shading path settled, three new defects

Two things this pass had that neither predecessor did: the **director's two frames**, and **ED-2**. The frames turned out to be worth more than the tooling, because they carry the camera readout — and the camera is recoverable from it.

**Headline:** the boundary is not a brightness edge at all. It is a **detail edge**, and it sits at **`compute_material_lod`'s LOD1|2 threshold — pixel footprint = 2.0** — in both frames, to within 4 screen pixels of a prediction with one fitted parameter. That threshold is a hard `if` on a per-pixel continuous quantity, called engine-wide.

**Why two passes missed it:** §2.2 and §9.2 both measured **row-mean luminance**. Across this boundary luminance moves under 2% — §2.2's own number, and it was correct. Grain energy across the same edge moves **68%**. The metric was wrong, not the observation.

---

## 10.1 Phase 5 — the editor shading path (ANSWERED; the CSM contradiction dissolves)

Traced end to end, then adversarially re-verified against the cited lines:

- `ViewportRenderer::render` → `EngineRenderAdapter::render_to_texture` (`renderer.rs:703`) → `Renderer::draw_into` (`engine_adapter.rs:889`) → `main render pass` → `TerrainMaterialManager::draw_chunk_forward` per chunk (`astraweave-render/src/renderer.rs:5933-5943`).
- That pipeline's module is `TERRAIN_FORWARD_SHADER` (`terrain_material_manager.rs:171-179`), which is `constants.wgsl + brdf_common.wgsl + stochastic_tiling.wgsl + **pbr_terrain_forward.wgsl**`, built by `ensure_forward_pipeline` (`:1005-1054`) and bound at `:1247-1253`.
- The rival `pbr_terrain.wgsl` deferred pipeline is **dead**: its only callers are in `viewport/terrain_splat.rs`, whose own header says `//! **SUPERSEDED**`, and that type is never constructed. `clipmap_terrain.wgsl` has zero references.

**So `pbr_terrain_forward.wgsl` IS what the director sees**, and §9.4's contradiction resolves the boring way: the terrain genuinely samples no shadow map, so **CSM cannot produce a boundary on terrain**. Cascade splits are retired as a hypothesis. (The census also corrected a stale number carried in §9.4: the editor sets `set_cascade_extents(80.0, 250.0)`, lambda 0.7 — not 10/50/200/1000.)

### 10.1.1 The "Lit" dropdown is inert — a defect found in passing

`ShadingMode { Lit, Unlit, Wireframe }` (`toolbar.rs:455-464`) is converted at `widget.rs:880` and passed to `ViewportRenderer::render`, whose parameter is **`_shading_mode: u32`** (`renderer.rs:570`) — unused. No match, no branch, no uniform write. `has_lighting()` / `is_wireframe()` have zero non-test callers.

Worse, there are **two** dropdowns: the docked Viewport panel offers five entries (`tab_viewer/mod.rs:2082` — Shaded/Wireframe/Unlit/**Normals**/**UVs**) and `main.rs:5208-5214` maps Normals and UVs silently to `Lit`. So the editor advertises a debug-visualisation facility it does not have. Selecting any mode changes nothing.

This is not the cause of the defect, but it is a live Integration-Completeness §3 violation and it is why "check it in Normals mode" was never an option.

---

## 10.2 Recovering the camera from the two readouts

The readouts give eye positions only. `OrbitCamera::position()` (`camera.rs:530-537`) places the eye at `focal + distance·(cos y·cos p, sin p, sin y·cos p)`, so a pure zoom — the mouse wheel, which changes `distance` alone — translates the eye exactly along that unit vector. The two eyes differ by `(83.6, 121.7, 85.0)`, `|Δ| = 170.4`:

| component | value | implies |
|---|---|---|
| `sin(pitch) = 121.7/170.4` | 0.7142 | **pitch 45.6°** |
| `cos(yaw) = (83.6/170.4)/cos p` | 0.7010 | **yaw 45.5°** |
| `sin(yaw) = (85.0/170.4)/cos p` | 0.7127 | yaw 45.4° |

Two independent components agreeing to 0.1°, landing on `OrbitCamera::default()`'s 45° yaw, is what makes this a recovery rather than a guess. It is confirmed by the render: at this pitch the harness reports **`sky_frac` 0.000** — no horizon in frame, exactly as in both director frames.

The focal point is underdetermined (any focal on the same ray reproduces the eye) and does not need determining: the image depends only on eye + yaw + pitch + fov + aspect.

**Viewport geometry**, measured from the UI chrome in both screenshots (the render area runs x 224..1186, y 105..606): **962 × 501 px, aspect 1.920**. Identical in both frames.

---

## 10.3 Deliverable 1 — the boundary's position, and what it matches

Edge rows measured from grain energy (`|L − 3×3 box mean|`), not luminance:

| frame | edge (normalised row) | camera above ground | ground distance at the edge | **pixel footprint there** |
|---|---|---|---|---|
| y414.5 | 0.527 | 378.5 m | **514 m** (3D) / 348 m (horizontal) | **2.000** |
| y536.2 | 0.749 | 500.2 m | **569 m** (3D) / 270 m (horizontal) | **1.984** |

`compute_material_lod`'s LOD1|2 constant is **2.0** (`brdf_common.wgsl:63`).

**The distance is not constant** (514 vs 569 m 3D; 348 vs 270 m horizontal), which by itself rules out anything keyed to a distance threshold. The footprint is constant, at the value the code thresholds on.

### 10.3.1 How much of that is a fit

One free parameter: the local ground height under the camera. Fitted to frame y414 it comes out at **36.0 m** — against T.2a's independently established desert ground height of **36.3 m**. Using it, frame y536's edge lands at footprint **1.984**, i.e. **0.8% from the constant, −4 px on a 501-row frame.** That is a genuine prediction, not a second fit.

### 10.3.2 The rivals, tested the same way

Each rival has two unknowns (threshold + ground) against two constraints, so each fits *exactly* — the question is what it must assume:

| keyed quantity | required ground height | required threshold | corresponds to a code constant? |
|---|---|---|---|
| **pixel footprint** | **+36.0 m** (T.2a: 36.3 m) | **2.0** — fixed, not fitted | **yes — `brdf_common.wgsl:63`** |
| view-space depth | −86.1 m | 680 m | no; and terrain samples no shadow (§10.1) |
| true 3D distance | −208.1 m | 846 m | no; fog is 60000/120000 at density 0 |

Both rivals need the desert floor tens to hundreds of metres below sea level and a threshold matching nothing in the source.

### 10.3.3 LOD 0 is unreachable at these altitudes

Directly measured from a false-colour render of `mat_lod`: **LOD 0 share = 0.000** in every configuration tested, and the analytic model agrees (footprint 0.5 is never reached anywhere in either frame).

**This retires §3.2 and §6.2 from this symptom entirely.** The Kulla-Conty multiscatter tier step — the one thing pass 1 convicted, at +9.5% — lives at the LOD0|1 boundary, which the director's view never contains. The two phenomena are now fully separated: the multiscatter step is a **close-range** defect (§5 items 1-4 stand), the boundary is a **far-range** one.

---

## 10.4 Phase 2 — reproduction: partial, and the delta is named

Experiment E (new, permanent) renders the recovered camera against the editor's own world settings — chunk_radius **10** (21×21 = 441 chunks, matching the "Terrain (441 chunks)" in the director's Hierarchy panel; both prior harnesses used radius 6) — at aspect 1.920 and three resolutions.

**What reproduced:** the camera (`sky_frac` 0.000), the desert tone (frame mean luma 110.1/109.4 vs the director's 117.7/119.0; the ContinentalTemperate alternative gives 74.7 and is excluded), and a real LOD1|2 contour in the frame.

**What did not:** a *clean line*. In the harness the contour is a wide band of **per-pixel salt-and-pepper** instead.

**The delta, named:** it is **world content**, not editor state. The census checked every state surface the live editor touches that the harness does not — fog (60000/120000, density 0), ambient ([0.45,0.50,0.55] @ 0.35), sun ((−0.5,−0.6,−0.4) @ 1.5), cascade extents/lambda/filter, quality preset, time-of-day (12.0 both), biome pack, post-process — and found them **identical in steady state**. What differs is the terrain: radius-10 Desert generates a mountainous world; the director's frames are gentle dunes.

That matters mechanically, not cosmetically: **`fwidth(world_pos)` differentiates the full world position, height included.** On rough ground the height derivative dominates and swings the footprint across 2.0 from one pixel to the next, so the tier boundary shatters into dithering; on smooth ground the footprint grows monotonically with distance and the same threshold resolves into a single contour. The false-colour LOD map shows exactly this: the per-column LOD1|2 boundary row scatters over **53 rows** in the rocky repro, where the flat-ground model predicts a smooth curve.

**This is the leading explanation for the reproduce/not-reproduce split and it is NOT proven** — see §10.6 for why the test that would prove it could not be run.

---

## 10.5 Phase 3 — the A/B, and a second real defect

Leg: `let mat_lod = 1u;` (LOD 2 removed everywhere; LOD 0 never occurs, so this isolates LOD 2 exactly). Reverted — `git diff astraweave-render/` is empty.

**Result 1 — the threshold is nearly invisible as a luminance step.** Comparing LOD1 against LOD2 pixels within the mixed band, where terrain content is comparable: **+1.4% (y414), −2.0% (y536)** — and **the sign flips between frames**. The two tiers happen to agree near the threshold and diverge only deep in the far field (up to **+11.5 luma, +10.4%**, in the top 70 rows). So `compute_material_lod` produces a **gradient**, not an edge, in *luminance*.

**Result 2 — LOD 2 adds per-pixel dithering.** Pinning the tier removes **40-55%** of far-field high-frequency energy (grain ratio 0.44-0.58 in the top rows) while changing the near field not at all (ratio 1.00 at rows 400-480). The difference image is a field of salt-and-pepper confined exactly to the LOD2 region.

**This is a genuine, previously unrecorded defect**: distant terrain carries shading noise from tier flicker, and it will shimmer under camera motion. It is independent of whether the boundary question resolves the same way.

**The honest tension:** the geometry says footprint = 2.0 to within 4 px in both frames; the A/B says that threshold produces dithering rather than a clean edge *on a rough world*. §10.4's mechanism reconciles them. It is a hypothesis with direct supporting evidence (the 53-row contour scatter), not a proof.

---

## 10.6 The test that would close it — and why it did not run

Experiment F sweeps terrain amplitude (50 / 20 / 6) at the recovered camera, to show the contour sharpening as the ground smooths. **All three legs came back byte-identical.** Per this harness's own rule (§1.1), identical output across a swept parameter is not a measurement — so the guard fired, and the cause is worth recording on its own:

> **The editor's Terrain-panel "Base Amplitude" slider is inert.**
> `terrain_panel.rs:1064` (slider, 10..200) → `terrain_panel.rs:2016` `set_noise_params(..)` → `terrain_integration.rs:167` `config.noise.base_elevation.amplitude`. But `noise_gen.rs:575` documents that read as **replaced** by `params.base_elevation_amplitude`, applied at `:663`, and `regional_archetype_mask.rs:469` blends that from **archetype splines** — never from `config`. Octaves, lacunarity and persistence are still read; amplitude alone is dead. `terrain_panel.rs:2004-2005`'s comment still claims all four "flow through", which is doc-drift.

Third Integration-Completeness §3 violation this pass (with the Lit dropdown and the Normals/UVs modes). Experiment F is kept, with a permanent `DEGENERATE` flag generalising the `sky_frac` rule: **a swept parameter that produces identical output has not been measured.**

---

## 10.7 STOP — this is a cross-material change, so it is the director's call

`compute_material_lod` is called from **three** places — `renderer.rs:231` (static PBR), `renderer.rs:571` (skinned PBR), `pbr_terrain_forward.wgsl:369` (terrain). Any change alters the appearance of every material at distance, engine-wide. Per this beat's own rule that is a STOP-with-options, not an agent decision.

| option | what it fixes | what it costs |
|---|---|---|
| **A. Delete the tiers — always LOD 0** | Removes the boundary, the dithering, and the §3.2 close-range multiscatter step in one move. One BRDF everywhere; every judgment becomes comparable. | Loses the ALU saving (~30 ALU/fragment at LOD 2). Far-field terrain gains multiscatter and brightens; rots far-field frames in T.2a/T.2c. |
| **B. Blend across the thresholds** instead of `if` | Removes the visible boundary and most of the dithering while keeping the saving. | Real shader work + its own A/B. Does not remove the *tier* concept, so some camera-dependence remains. |
| **C. Key the tier on view-space depth instead of `fwidth`** | Kills the dithering outright (depth is smooth where `fwidth` is not) and makes the tier boundary predictable. | Loses the property the tiers were designed around — screen coverage, which is what actually justifies simplifying the BRDF. Boundary becomes a hard horizontal line, arguably *more* visible. |
| **D. Leave it, document it** | No churn. | Distant terrain keeps shimmering; the boundary stays. |

**My recommendation: A**, and I would not have said that before this pass. The tiers are buying ~30 ALU/fragment on a shader whose cost is dominated by **24 `textureSampleGrad` calls per fragment** (8 layers × 3 hex taps); the saving is in the noise. Against that they cost a visible boundary, far-field dithering, a ±10% close-range brightness step, and — the thing that has cost this campaign the most — **every appearance judgment sitting on a different BRDF depending on where the camera was**. Deleting the tiers makes T.G's gate frames comparable to each other for the first time.

If A is chosen, the far-field brightening must be measured and the affected T.2a/T.2c frames re-shot — that is a beat, not a patch.

---

## 10.8 What I still need, if you want certainty before choosing

One capture, and ED-2 now makes it cheap. Pin a station at the recovered camera and shoot it:

```
focal (−1029.9, 36.3, 254.7)   yaw 45.5°   pitch 45.6°   distance 529.6  (= eye 414.5)
                                                          distance 700.0  (= eye 536.2)
fovy 60°
```

**Camera → name → Pin**, then **Shot**. The `.camera.json` sidecar records the exact state and the render size — which also settles whether the viewport is natively 962×501 (my assumption throughout §10.3; the whole footprint calculation scales with it).

Alternatively, just the terrain settings from those sessions (archetype, and whether the ground was sculpted) would let me regenerate the world and finish Experiment F's job with a live lever.

---

## 10.9 Re-judgment list — updated

§5's list stands, with one narrowing and one addition:

- **Narrowed:** items 1-4 are close-range judgments and are affected by the §3.2 multiscatter tier step, which is a **LOD0|1** effect. §10.3.3 confirms LOD 0 is only reachable close in — so the list is correctly scoped, and the far-range boundary does not add to it.
- **Added (5):** any judgment of *distant* terrain — silhouette, biome banding, large-scale colour — carries the LOD2 dithering measured in §10.5 and the LOD1|2 boundary. T.2a's far-field station frames are the ones at risk. Their A/B *deltas* survive (identical cameras per leg, so the offset cancels); their absolute reads do not.

---

## 10.10 Verification

| rung | result |
|---|---|
| `cargo fmt -p aw_editor` | clean |
| `cargo check -p aw_editor --test t2d_camera_light` | **exit 0** |
| `git diff astraweave-render/` | **empty** — both temporary shader legs (LOD false-colour, `mat_lod = 1u`) reverted |
| Experiment E, 12 renders × 2 legs | 1 passed / 0 failed each |
| Experiment F (amplitude sweep) | 1 passed / 0 failed; **DEGENERATE flag fires on all 3 legs** — the finding of §10.6 |
| LOD false-colour leg | LOD 0 share **0.000** in all 12 configurations |

Frames and CSVs: `d:/tmp/t2d_staging/{E_head,E_lodviz,E_lod1const,F_head}/`.

No regression test is added, for the same reason as §7: nothing is fixed yet, and a test asserting today's behaviour would pin the bug. The instrument is the deliverable — Experiments E and F are permanent and `#[ignore]`d like the rest.

---

# 11. T.2d.F (2026-07-25) — the convicted defect is FIXED; this file closes

The director ratified §10.7 **option A** the same day: delete the tiers, with a pre-authorized
continuous-falloff fallback if min-spec perf regressed materially (a stepped tier may not return
under any outcome). Executed as beat **T.2d.F**; the full evidence chain is
**`docs/audits/T2DF_OUTCOME.md`** — this section is only the closing pointer.

What the fix measured, against this file's findings:

| this file's finding | T.2d.F result |
|---|---|
| LOD1\|2 boundary at footprint 2.0 (§10.3) | threshold deleted from every shader; no contour step at the footprint-2.0 row (grain ratio ×1.05/×0.97 ≈ 1) |
| LOD 2 tier dithering = 40–55% of far-field HF energy (§10.5) | far-field grain **−48.6% / −51.7%** at the two boundary stations — inside the diagnosed band; grain field now flat across the frame |
| multiscatter tier step, close range (§3.2, §6.2) | gone by construction (no LOD0\|1 divide); close range measured unchanged — **1 px of 786,432, 1 LSB** |
| far field expected to brighten ~10% under option A (§6.2) | **+2.9 → +10.7 luma over 300–1500 m (≤ +9.5%)** — accepted consequence; T.2a/T.2c far-field frames rot |
| §2.1 near-field gradient (+5.9%, 12–110 m) | **NOT closed** — §3.2 already proved the tiers were not its cause; residual still open (§3.4 specular lead) |
| §9.1 far-field gradient (near-darker/far-brighter) | **survives as it must** (+13.5% over 300–1500 m post-fix) — normal-variance class, unratified, untouched |
| min-spec cost of the tiers' removal | **−1.0%** (27.055 → 26.773 ms median, 1660 Ti Max-Q, 1080p, n=300) — the tiers were saving nothing measurable; fallback not triggered |

The re-judgment list (§5, §10.9) transfers to `T2DF_OUTCOME.md` §6.2 unchanged in substance, with
one addition: distant-terrain absolute reads made before the fix are superseded by post-fix frames.

The director's closing check is **Camera → Go** on the five `t2df_*` stations now pinned in
`.editor_preferences.json` (script: `T2DF_OUTCOME.md` §7). That verdict closes T.2d and releases
the terrain lane.
