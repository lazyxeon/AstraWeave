# T.2f — The lighting model: recon (read-only census, options to the director)

> **Beat:** T.2f (terrain series) · **Date:** 2026-07-26 · **HEAD:** `09ea417af`
> **Session discipline:** read-only. Every experiment below was either driven through public
> editor APIs (no code change) or a temporary shader/mapping edit **reverted before this
> commit** — at commit time `git status` shows only this report (§7). No fix was applied.
> **Frames:** `d:/tmp/t2f_staging/{phase_a,phase_b}/` (session-local).
>
> **The standing complaint:** after T.2a/T.2c/T.2d.F, close-range terrain "still reads flatish
> and shinyish — not as bad as before, but still there."
>
> **Headline answer (Question 4):** it is a **lighting-completeness gap, with a calibration
> defect stacked on top — not a material gap.** The materials now carry real albedo, normals,
> roughness, and AO; the lighting model consuming them is one unshadowed directional light plus
> a fill worth ~4% of the frame. Measured: not a single pixel in any captured frame is occluded
> from the sun (§4.2); the entire ambient term is a uniform wash contributing −6.7 luma when
> removed (§4.2); the repaired AO channel changes **zero pixels** when forced to 1.0 (§4.3);
> and the editor's sun intensity is **bistable at startup** — a dropped-push race decides
> whether the scene renders at 1.0× or 2.2× sun (§3). No albedo/normal/roughness improvement
> can add the missing terms. The one caveat that keeps "both" partially true: the AO textures
> themselves are near-white (§4.3), and the grassland's hex-cell repetition is still visible at
> mid range — material-side residues, but second-order next to the missing terms.

---

## 1. Question 1 — what lights editor terrain today

### 1.1 The complete equation

`pbr_terrain_forward.wgsl` is the shader the director sees (T2D_CAMERA_LIGHT.md §10.1 settled
this; the deferred rival is dead). Its **entire** lighting model, `fs_main` lines 387–401:

```
lit = evaluate_brdf(N,V,L, albedo, metal, rough, F0) · (sun_color · sun_intensity)   [:387-391]
    + albedo · (ambient_color · ambient_intensity · 0.35) · AO                        [:393-395]
lit = fog(lit, dist)          // 60000/120000, density 0 → inert                     [:397-399]
```

then `draw_into`'s terminal post pass: `ACES(lit × 1.35)` (renderer.rs:373-390, :6166-6199).
That is everything. No shadow term, no environment term, no GI term, no occlusion beyond the
starved AO factor, no exposure control.

### 1.2 Term census

| # | term | on terrain? | data source at runtime (editor, steady state) | citations |
|---|------|-------------|-----------------------------------------------|-----------|
| 1 | **Direct sun** | YES | Direction: hardcoded `normalize(-0.5,-0.6,-0.4)` (elev ≈ 43°; the comment says "~35°") written at terrain upload. Colour/intensity: **bistable** — SceneEnvironment defaults `[1.0,0.98,0.9] × 1.0` OR the World-panel push `[1.0,0.96,0.88] × 2.2`, decided by a startup race (§3). Art-directed placeholder either way; TimeOfDay exists but is permanently overridden. | engine_adapter.rs:1788-1789; scene_environment.rs:114-115; tab_viewer/mod.rs:951-954; §3 |
| 2 | **CSM shadows** | **NO** | Terrain shader has zero shadow bindings — its group(2) is the 8 splat maps + sampler (`pbr_terrain_forward.wgsl:112-126`). Confirmed at HEAD: the only "shadow" occurrences are the two comments saying there are none (`:20`, `:389`). | shader :101-126 |
| 3 | CSM consumers, exhaustively | — | 2048²×2-layer Depth32Float (renderer.rs:2478-2493). Samplers: **static PBR fs** (PCF 3×3, :259-301) and **skinned fs** (:607-632, pipeline dormant — created, never drawn; ED3_OUTCOME §5). That is the complete list. In the editor terrain scene **even those never fire**: terrain upload auto-applies the `EditorTerrain` preset (engine_adapter.rs:1799-1804) which sets `set_shadows_enabled(false)` (:1056) → `extras.x = −1` (renderer.rs:4668-4677) → shader guard skips; the shadow render passes are also skipped (`has_shadow_casters`, renderer.rs:5671-5680). **Yet `update_cascade_splits` still runs every frame** (renderer.rs:4243-4250: matrix math + 3 buffer writes) — cascades computed, consumed by nothing. Also: forward terrain chunks live in `tf.chunks`, not `self.models`, so terrain is **not rendered into the shadow maps at all** — even under GameQuality, terrain neither receives *nor casts*. The `terrain_c*` shadow-caster comments (:5732-5760) refer to the legacy model path deleted in Cleanup-A. | as cited |
| 4 | Cloud shadows | NO (never sampled by terrain) | Static-only (`sample_cloud_shadow`, :128-135, :303); pass disabled by the preset anyway (:1057, :5833). | as cited |
| 5 | **Ambient** | YES — flat constant | `[0.45,0.50,0.55] × 0.35 × 0.35` ≈ RGB(0.055,0.061,0.067) — a uniform, non-directional wash set at terrain upload ("so shadowed areas aren't pitch black" — there are no shadowed areas). No hemisphere, no sky/ground split, no directionality. | engine_adapter.rs:1771-1773; shader :393-395 |
| 6 | **IBL** | **NO — structurally** | Terrain shader binds no environment resources of any kind. **Engine-vs-editor split:** the engine machinery is complete (`IblManager`, default `SkyMode::Procedural`, `bake_environment` → irradiance + prefiltered-specular + BRDF-LUT + intensity normalisation; used by `hello_companion`/`veilweaver_demo`). The editor **never bakes at init** — the only bake callers are the user-initiated HDRI load (main.rs:5349) and HDRI clear (viewport/renderer.rs:1541-1553). So even the static-mesh shader's full `compute_ibl` (:179-203) multiplies the **1×1 black fallback cubemaps** (:2129, :2180) → contributes exactly zero. The HDRI catalog (`hdri_catalog.rs`) is only reachable via `sync_biome_time_of_day`/biome transitions — zero production callers. The procedural sky the camera *sees* is a backdrop gradient (renderer.rs:5795-5821), not an input to any lighting term. | as cited |
| 7 | SSGI / GI | NO | Terrain never samples `gi_tex`; static samples the 1×1 black fallback (:2227) → +0. | :313-318 |
| 8 | SSAO | **does not exist at runtime** | The `EditorTerrain` preset writes `ssao_enabled: true` with a comment claiming it "restores crevice/contact shading" (engine_adapter.rs:1063-1067) — but the renderer reads **only** `post_chain.bloom_enabled` (renderer.rs:6109, the single read); the SSAO pipeline objects are built into discarded `_`-bindings (:1673, :1694) and the pass is explicitly disabled (:5455-5460). The preset's claim is fiction. (Same for `taa_enabled` — zero TAA implementation in renderer.rs.) | as cited |
| 9 | **Texture AO** (ORM.r) | YES — applied **correctly** (ambient only, not direct), but starved | Blended per layer (:337), multiplied into ambient only (:395) — the right place. But the ambient it modulates is ≈0.06, so its measured contribution is **zero pixels** (§4.3). | shader :337, :395 |
| 10 | **Tonemap/exposure** | ACES Narkowicz, **fixed exposure 1.35**, unconditional | The editor builds without `postfx` (aw_editor Cargo.toml:73); `draw_into` ends in the canonical POST_SHADER (curve :373-380, exposure :385; pass :6166-6199 — "P.3: runs unconditionally"). No auto-exposure. The World panel's **Exposure slider is inert**: `TerrainLightingParams.exposure` (types.rs:99, panel default 1.3, slider tab_viewer/mod.rs:4716) is dropped by `set_lighting_params` (engine_adapter.rs:3808-3823 never reads it). `PostProcessChain.tonemap_operator`/`auto_exposure_enabled` are unread. | as cited |
| 11 | Bloom | off in editor presets; when on, computed then **never composited** | renderer.rs:6109-6152 (comment records it) | |
| 12 | Point/spot shadows | dormant module (own tests only) | shadow_point.rs | |

Static-vs-terrain summary: the static-mesh shader carries CSM + cloud shadow + IBL + SSGI + tint
on top of the shared BRDF; terrain carries **none of them**. In the editor's terrain scene the
difference is currently academic — the preset disables the shadows statics would sample, and the
never-baked IBL is black — so *nothing in the scene* is lit by more than sun + flat fill.

## 2. What the ~27 ms buys

For the cost table's context: the T.2d.F min-spec median was 26.77 ms (1080p, radius-10 desert,
GTX 1660 Ti Max-Q). The lighting share of that is a single `evaluate_brdf` + one multiply-add of
a constant ambient; the frame is dominated by the 24 `textureSampleGrad`/fragment material
sampling. The lighting model is **not** where the frame time goes — there is real headroom to
buy lighting terms before touching the material sampling budget.

---

## 3. A defect found by the census: the sun is bistable at startup

**The live editor's sun intensity depends on a startup race, and the losing branch is silent.**

- Every UI frame, main.rs:4221-4259 pushes the World panel's lighting when it differs from a
  cache. First frame: cache is `None`, so the panel defaults (sun `[1.0,0.96,0.88] × 2.2`,
  ambient `[0.65,0.58,0.50] × 0.45` — tab_viewer/mod.rs:946-956) are pushed once.
- `ViewportRenderer::set_lighting_params` **silently drops** the push if the async engine
  adapter isn't initialised yet (viewport/renderer.rs:1581-1586, `if let Some(adapter)`).
- main.rs:4258 then caches the params as delivered — **the push is never retried** until the
  user changes a slider.

Since `init_engine_adapter` is async GPU setup and the first egui frames run before it
completes, the push is lost in practice: the editor runs at the SceneEnvironment defaults
(sun × **1.0**) while the panel *displays* 2.2. The first time anyone touches any Lighting
slider, the full panel state finally lands and the scene jumps ~+47 mean luma (§4.1) — the
+42% step measured as L0→L1. Three corroborations that the director's editor sits on the
1.0× branch: T.2d §10.4 measured the director's frames within ~7% of the 1.0× harness; the
terrain-upload block's own ambient/direction overwrites are what the T.2d state census found
live; and the panel's 2.2 would place desert luma ~160 (§4.1), far above the director's 117.7.

Consequences worth naming now:

1. **Judgment integrity:** every appearance judgment this campaign has made sits on the 1.0×
   branch; a stray slider touch silently moves the ground truth +42%. This is the same class of
   trap as the T.2d camera-exposure finding, and it should be fixed (or at least pinned) before
   T.3's amplitude-finality gate and any T.G gate frames.
2. The harness (ED-2 captures, all T-series stations) matches the *untouched* live editor —
   the T-series absolute numbers remain valid for the state the director actually sees.
3. `set_light_direction_override`'s intensity argument is **write-only** (packed into
   `light_dir_pad[3]`, renderer.rs:4222; only xyz are ever read back, :5301, :5869) — the "1.5"
   the terrain-upload block passes goes nowhere. Radiance comes solely from
   `env.sun_color × env.sun_intensity`.

---

## 4. Question 2 — observation: the gap made visible

**Instrument:** a temporary harness (`t2f_recon.rs`, archived off-repo, deleted before this
commit) driving the editor's live path — `ViewportRenderer::render` → ED-2 `capture_frame_png`
— at four stations: Desert close (T.2a anchor focal, dist 20 m, pitch 55°) and mid (dist 46.7,
pitch 40° — the T.2d.F contour-station geometry), and Cont-Temp grassland close/mid (focal on
measured ground 41.0 m at origin, same orbits). 1024×768, radius-6 worlds, min-spec adapter
(GTX 1660 Ti Max-Q · Vulkan · 592.82). Isolation legs:

- **API-driven (no code change):** L0 = harness/untouched-live baseline (sun ×1.0); L1 =
  panel-delivered state ("live-parity", sun ×2.2 — what the editor becomes after one slider
  touch); L2 = L1 with ambient **zeroed** (pure direct light); L3 = L1 with sun zeroed (the
  fill in isolation). Plus the ED-3 Normals view at both close stations.
- **Temporary shader legs (reverted):** M10 = AO forced to 1.0; M11 = AO channel visualised;
  M12 = diffuse-only BRDF (specular + multiscatter removed).

### 4.1 The numbers

8-bit luma; grain = mean |L − 3×3 box mean|; "<½-mean" = fraction of pixels darker than half
the frame mean (a proxy for *any* occluded/shaded region existing at all).

| station | leg | mean | sd | p1 | p99 | <½-mean |
|---|---|---|---|---|---|---|
| desert close | L0 baseline | 112.6 | 11.3 | 81.7 | 136.8 | 0.0% |
| desert close | L1 live-parity | 159.9 | 12.0 | 124.7 | 182.9 | 0.0% |
| desert close | L2 sun-only | 153.2 | 12.9 | 115.7 | 177.9 | 0.0% |
| desert close | L3 ambient-only | 28.4 | 3.7 | 18.9 | 36.8 | 0.2% |
| desert mid | L1 / L2 / L3 | 162.5 / 156.0 / 28.5 | 11.9 / 13.0 / 3.0 | | | all ≈0% |
| grass close | L0 baseline | 82.3 | 12.5 | 53.8 | 114.3 | 0.1% |
| grass close | L1 live-parity | 128.5 | 14.8 | 91.5 | 163.2 | 0.0% |
| grass close | L2 sun-only | 124.5 | 14.8 | 87.3 | 158.9 | 0.0% |
| grass close | L3 ambient-only | 10.8 | 3.7 | 4.2 | 22.0 | 3.9% |
| grass mid | L1 / L2 / L3 | 130.8 / 127.0 / 10.5 | 11.7 / 11.8 / 2.5 | | | all ≈0% |

### 4.2 What a viewer is missing, stated plainly

1. **No occlusion of any kind, anywhere.** In the pure-direct legs (L2) the *darkest 1% of
   pixels* still read 115.7 (desert) / 87.3 (grass) — ≥70% of the frame mean. Zero pixels fall
   below half the mean in any lit frame, at any station, in either biome. Real ground under a
   43°-elevation sun self-shadows constantly at every scale; here it is geometrically
   impossible for any pixel to darken, because no shadow term exists. This is the single
   largest absence.
2. **The fill is a wash, worth ~4% of the image.** Deleting the entire ambient term (L2 vs L1)
   costs −6.7 luma (desert) / −4.0 (grass) on means of 160/128 — and the sun-only frames are
   visually indistinguishable from the full frames. The ambient-only legs (L3: sd 2.5–3.7)
   show why it can't be more: it is direction-less and colour-flat, so scaling it up would
   only lift blacks, not model anything.
3. **Contrast is compressed to a sliver.** sd/mean at L1: 7.5% (desert close) to 11.5% (grass
   close). The whole image lives inside ±30 luma. Two compounding causes: nothing occludes
   (cause 1), and the ACES shoulder — L0→L1 raises the mean +42% while sd rises barely 6%
   (11.3→12.0), i.e. the panel-delivered 2.2× sun makes the scene *flatter*, not punchier.
   The fixed 1.35 exposure with no exposure control leaves no way to place the scene on the
   curve's useful slope.
4. **The shading-relevant normal field is nearly uniform at close range.** The ED-3 Normals
   captures at 20 m read sd 1.1–1.4 (a near-constant pale Y-up field): after mip-filtering,
   what N·L has to work with at these stations is almost constant — so a lighting model that
   is *only* N·L has almost nothing to say. ("Flatish" is over-determined.)
5. **"Shinyish" is a broad, uniform, un-occluded sheen — not misplaced highlights.** Removing
   specular+multiscatter (M12 vs L1) subtracts a spatially smooth −6.2 luma (desert) /
   −11.7 (grass) (p1..p99 of the delta: −16..−3). Grass carries ~2× the desert's specular
   energy. The term is physically modest; it reads "shiny" because it is everywhere at once,
   never shadowed, never contrasted against an environment reflection, and sits on a
   near-uniform normal field.

### 4.3 AO: correctly applied, doubly starved

Forcing AO to 1.0 (M10) changes **no pixel by more than 0.7 luma in any of the four stations —
0.00% of pixels differ beyond 1 LSB.** The channel T.2a repaired (the PROVEN-inverted AO fix)
currently buys nothing, for two stacked reasons, both measured:

- the only term it modulates is the ≈0.06 ambient (§1.2 row 9) — right place, no budget;
- the AO data itself is nearly white: the visualisation legs (M11) read mean 235.3 (desert —
  AO ≈ 1.0, sd 1.1) and 224.9 (grass — AO ≈ 0.93, sd 4.5). Even under a real ambient/IBL
  term, today's desert AO would do almost nothing; grass would gain mild crevice shading.

So Question 1.5's answer: **not misapplied — starved.** The fix is not "move AO", it is "give
indirect light enough weight that AO has something to occlude", plus (later, material lane)
richer AO bakes.

---

## 5. Question 3 — options, costs, and a ranked recommendation

Frame-time context: 26.77 ms min-spec median (§2); water precedent budgeted 2.0 ms.

| option | what exists already | what must be built | expected visual effect | cost class | risk |
|---|---|---|---|---|---|
| **A. Terrain samples the CSM** | Cascades computed every frame (currently for nobody); 2048²×2 maps + comparison sampler + PCF-3×3 reference code in SHADER_SRC :259-301; per-cascade buffers live | (i) receiver: +1 bind group (light UBO + shadow tex + sampler) in the terrain pipeline layout + cascade-select/PCF in the fragment shader; (ii) **caster: terrain depth-only pipeline + chunk draws into cascade 0** — forward chunks are in `tf.chunks`, drawn into no shadow pass today (§1.2 row 3); (iii) re-enable shadows in the EditorTerrain preset (it was disabled for exactly this cost) | Cast + self-shadowing on relief — the strongest missing depth cue; at cascade-0's 80 m extent ≈ 7.8 cm/texel, resolves metre-scale relief, not pebble grain. Transforms mid-range dunes; at 20 m on smooth ground the effect is real but subtler than at mid | **Medium-high**: shader + pipeline-layout + new depth pipeline + preset policy; GPU est. +2–5 ms min-spec (depth-raster of culled chunks × 1–2 cascades + 9 comparisons/px in range) | Perf on min-spec (the preset disabled shadows once already); acne/peter-pan bias tuning on steep slopes; every T-series frame rots |
| **B. IBL for terrain + bake at editor init** | **Everything engine-side**: IblManager (procedural default), full bake chain, intensity normalisation, `compute_ibl` reference in SHADER_SRC :179-203; editor device already requests max_bind_groups 8 (terrain uses 3) | One `bake_environment(Medium)` call at adapter init (statics get real IBL for free — currently multiplying black); terrain: +1 bind group (irradiance + prefiltered + LUT + params) and a `compute_ibl` port replacing/augmenting the flat ambient (diffuse-IBL × AO, spec-IBL) | Replaces the 4% wash with **directional sky light** (blue-ish above, warm horizon → normals become visible in fill), makes AO live (§4.3), and gives specular a sky to reflect (roughness-varying, not uniform sheen). Attacks *both* halves of "flat and shinyish" at close range | **Medium**: ~3 `textureSampleLevel`/fragment ≈ 1–3% of the material-sampling cost, est. +0.2–0.8 ms; one-off bake tens of ms | Global brightness recalibrates (frames rot; ibl_intensity normalisation exists to tame it); procedural-sky env is provisional art direction; bake-on-time-of-day-change policy needed |
| **C. Hemisphere ambient** (sky/ground two-colour, keyed on N·up) | trivially derivable from SkyConfig colours | ~5 shader lines + 2 colours plumbed | Normals become visible in the fill — a strict subset of B's diffuse half | **Low** (hours) | Superseded the day B lands; another tuned constant |
| **D. AO application** | already correct (ambient-only) | nothing to fix now; becomes meaningful the moment B/C exists; longer-term: richer AO bakes (desert AO is ≈1.0 white, §4.3) and the statics' missing AO channel (mr_tex has no occlusion input — props lane) | close-range crevice contact — *through* B/C | **Zero now** | none |
| **E. Calibration & honesty batch** | postfx params buffer already uploaded per frame with 3 spare floats (renderer.rs:6160-6164) — a free seam for a real exposure uniform | (1) fix the §3 dropped-push race (don't cache undelivered params) **and** reconcile panel defaults with the delivered state so the UI stops lying; (2) wire the Exposure slider into the post pass or remove it; (3) delete the preset's SSAO/TAA fictions; (4) optionally skip cascade updates when shadows are off | No new light — but it pins *which* scene everyone is judging, and exposure control directly counters the ACES-shoulder flattening (§4.2 item 3) | **Low** (the race fix + one uniform) | ED-3-class; near-zero |

**Ranking by visual-improvement-per-unit-work for the standing complaint:**

1. **B — IBL for terrain (with the init bake).** It is the only option that addresses both
   named symptoms at once — "flat" (directional fill, AO finally expressed, normals visible
   outside direct sun) and "shiny" (environment-shaped specular instead of uniform sun sheen)
   — at a fraction of A's GPU cost, reusing a complete, already-tested engine subsystem whose
   only gap is that the editor never turns it on.
2. **E — the calibration batch.** Cheapest item on the board and a judgment-integrity
   prerequisite: until the bistable sun is pinned, any before/after for A/B/C is measured
   against a scene that can silently shift +42%. If beats are sequenced, E rides along with
   or immediately before B.
3. **A — terrain CSM.** The largest single realism win and the true fix for "no occlusion
   anywhere" — but the most expensive on min-spec, needs a caster path that doesn't exist,
   and its close-range payoff on smooth ground is smaller than its mid-range payoff. Right
   beat: after B, with its own perf gate against the 26.8 ms baseline.
4. **C** only if B is deferred; **D** rides on B.

**If the director authorises exactly one change: Option B.** One beat: bake the procedural
environment at editor init, bind IBL into the terrain pipeline, fold the flat ambient into it,
A/B at the pinned stations, re-shoot the rotting frames. Fallback if min-spec objects
(unlikely at +≤1 ms): C, the hemisphere, which keeps the directional-fill half.

**What this recon does *not* recommend:** a lighting-rig rebuild. The rig's architecture is
sound — one sun + indirect + post; the gaps are that two of its three legs (indirect,
occlusion) are stubs in the terrain path and its calibration layer (sun push, exposure) is
broken. Filling legs beats rebuilding the rig.

---

## 6. Verification

| rung | result |
|---|---|
| `git status --short` at commit time | only this report (+ the untracked `.recent_files.json`) — both temporary edits (`pbr_terrain_forward.wgsl` isolation legs, `viewport/renderer.rs` mode-passthrough arm) reverted via `git checkout --`; bytes = HEAD `09ea417af`, which passed the full ED-3 verification ladder |
| temp harness | `t2f_recon.rs` deleted from the repo (archived in session scratchpad); its two runs: `1 passed / 0 failed` each (61.8 s / 58.7 s, min-spec GTX 1660 Ti Max-Q · Vulkan · 592.82) |
| harness compile gate before runs | `cargo check -p aw_editor --test t2f_recon` — exit 0 |
| captures | 30 frames, `d:/tmp/t2f_staging/phase_a` (18: L0–L3 + normals) and `phase_b` (12: M10–M12) |
| determinism control | M10 (AO→1, a no-op by prediction) reproduced L1 to ≤0.7 luma max across all four stations — the render+capture chain is stable across a full rebuild |

No regression tests added and no trace bumped: nothing changed. The census corrections that
touch trace-recorded facts (the §3 bistable sun; SSAO/TAA fiction; terrain absent from shadow
passes) are flagged here for the trace update **in the beat that fixes them**, per the
"structural liveness guard is never re-baked" discipline — recording them as invariants now
would pin defects.

## 7. Residue for the ledgers (found in passing, not fixed — recon only)

> **L.1 update (2026-07-26):** items 1–3 are **CLOSED** by beat L.1 (see
> `docs/audits/L1_OUTCOME.md`); items 4–7 remain open, with 6 now documented in code as
> deliberately-left (not trivially safe to skip).

1. ~~**§3 dropped-push race + poisoned cache**~~ — **CLOSED by L.1**: pre-adapter pushes are
   parked and delivered at `init_engine_adapter` (never dropped, observable via log); panel
   defaults reconciled to the delivered state, pinned by constants + tests.
2. ~~**Inert Exposure slider**~~ — **CLOSED by L.1**: exposure rides the scene-env UBO's second
   pad float (offset 72) into the post pass (`uPostScene.exposure`); default 1.35 = the former
   hardcode, proven pixel-neutral.
3. ~~**EditorTerrain preset claims SSAO + GameQuality claims TAA**~~ — **CLOSED by L.1**: flags
   zeroed, preset/enum/setter docs rewritten to state what actually runs; dormant machinery
   untouched.
4. **Bloom computes but never composites** when enabled (renderer.rs bloom dispatch comment) —
   pre-recorded in P.3, still true.
5. **`set_light_direction_override` intensity is write-only** (`light_dir_pad[3]`; now noted at
   the terrain-upload call site).
6. ~~**Cascade splits computed for no consumer** every frame while shadows are disabled — left
   deliberately (L.1): `update_cascade_splits` is also the delivery path for the `extras.x`
   shadows-off sentinel, so skipping it wholesale is not safe; L.3 consumes the cascades anyway.~~
   — **CLOSED by L.3** (2026-07-28, docs/audits/L3_OUTCOME.md): terrain casts into and receives
   from both cascades in the editor terrain scene (`EditorTerrain` preset re-enables shadows);
   the cascades have consumers every frame. The sentinel-delivery coupling stands unchanged.
7. **Static meshes sample no texture AO at all** (mr_tex = metallic/roughness only) — props
   lane, relevant the day statics matter in terrain scenes.

The director ratifies which of §5's options become execution beats.

---

## 8. L.2 corrections (append-only, 2026-07-27) — the recon's record self-corrects

L.2's Phase-0 re-verification (`docs/audits/L2_PHASE0_STOP.md`) and its fixing campaign
falsified three claims this recon made. Per the append-only convention the original text
above stands; this section is the correction of record.

1. **"Statics… currently multiplying black" (§5 option B cell; also the census's
   1×1-black-fallback framing) — FALSE, including at this recon's own commit.** The
   fallback cube has been a moderate, mildly-directional **sky-fill** (+Y `(140,160,200)`,
   sides `(100,115,140)`, −Y `(70,65,50)` sRGB) since commit `07b3d2137` (2026-04-10) —
   ten weeks before T.2f. The recon read the stale `// Fallback 1x1 black cubemap`
   comment instead of the texel upload seven lines below it. The corrected pre-L.2
   statement: un-baked static IBL contributed a small non-zero directional fill, and the
   fallback BRDF LUT (`[255,0,0]`) made its specular term `prefiltered * F`,
   roughness-invariant.
2. **"Full bake chain complete / reference `compute_ibl`" — the chain existed but was
   defective in two ways this recon did not audit.** (a) The irradiance convolution was
   **face-blind** (normal derived with z pinned to 1.0, no face index bound — all six
   faces held the same +Z convolution); (b) **every bake write pass stored its target
   vertically flipped** (uv derived from clip space without the v-flip), which mangled
   ±Y irradiance, roughness-flipped the BRDF LUT, and left specular only *looking* right
   through a double-flip cancellation. Both fixed and regression-locked in L.2
   (`b1295b93f`, `astraweave-render/tests/ibl_irradiance_faces.rs`: six-face probe vs an
   independent CPU reference, agreement ≤2.1% post-fix, spread 0.00% → 74% pre→post).
3. **Option B's "procedural default" framing** — the procedural sky bake (`SKY_WGSL`) is
   a self-labelled face-blind placeholder producing a near-isotropic blue cube; it could
   never have delivered the promised directional fill. L.2 baked the **git-tracked**
   `kloppenheim_02_puresky_2k.hdr` (catalog `kloppenheim_daytime`, CC0) instead. The
   placeholder remains the "Remove HDRI" rebake source — logged residue.
4. **New residue found by L.2** (logged, not fixed — director-ratified): the IBL intensity
   normaliser `compute_hdr_avg_luminance` clamps HDR to 8-bit before averaging;
   `hdri_catalog.toml`'s `default` names a non-existent entry; the specular prefilter
   hardcodes a 512 env size in its solid-angle term (exact for Medium, biased for
   Low/High); the deferred `TERRAIN_SPLAT_SHADER` fails naga under `--features gpu-tests`
   (`height_blend` invalid call — pre-existing, stash-proven); the shader-validation
   vacuity floor was stale-red at baseline (recalibrated 60 → 55 in L.2 with forensics).

See `docs/audits/L2_OUTCOME.md` for the full fixing record and the A/B.

## 9. L.3 corrections (append-only, 2026-07-28) — the CSM consumption beat's record

L.3 (terrain CSM cast + receive, `docs/audits/L3_OUTCOME.md`) confirmed this recon's §1.2
row 3 census in full — 2048²×2-layer Depth32Float, static/skinned samplers the complete
receiver list, terrain absent from both caster and receiver sides, the `extras.x`
sentinel — with two corrections and one attribution:

1. **§5 option A's "cascade-0's 80 m extent ≈ 7.8 cm/texel" was never a real number.**
   The 80 m figure traces to the terrain-upload block's `set_cascade_extents(80.0, 250.0)`
   call — which is a **no-op**: `cascade0_extent`/`cascade1_extent` are written and never
   read (sphere-fitted ortho bounds replaced them; the fields survived). The live cascade-0
   ortho extent is the sphere fit of the 0.5→split0 frustum slice. At the editor camera
   (fovy 60°, split0 ≈ 86 m at the upload path's λ=0.7) that is a ~230 m square →
   ~11 cm/texel — same order, different mechanism, and tunable only through the splits,
   not through the dead setter. Logged residue: the no-op `set_cascade_extents` API and
   its two editor call sites.
2. **The in-source "split0 ≈ 16 units" comment was the λ=1.0 pure-log value.** Actual
   splits at the editor camera: split0 ≈ 74.4 (λ=0.75) / ≈ 86.1 (the terrain-upload
   λ=0.7); split1 = 500. Comment corrected in L.3.
3. **Attribution note:** a dormant parallel CSM implementation exists at
   `astraweave-render/src/shadow_csm.rs` (`CsmRenderer`: 4 cascades, splits 10/50/200/1000,
   zero production callers — tests/examples only). Any account of "the CSM" quoting
   4 cascades or those splits describes that dormant module, not the live 2-cascade
   machinery in `renderer.rs` that L.3 consumed. Logged as architecture-drift residue.
