# L.2 Phase 0 — STOP: the IBL premise is falsified (irradiance convolution is face-blind)

**Session**: L.2 — IBL for editor terrain (bake at init, bind, sample)
**Date**: 2026-07-26 · **HEAD**: `2e938ff9d` (unchanged; nothing pushed since L.1)
**Verdict**: **STOP at end of Phase 0**, per the beat's anti-drift clause — *"Do not redesign
`ibl.rs` — it is finished; you are its first editor consumer. If it genuinely cannot serve the
terrain path without modification, STOP and present the gap."* It cannot: the diffuse half of
the split-sum is mathematically wrong at HEAD. No code was changed this session; this report
is the sole commit.

---

## 0. The gap in one paragraph

`IblManager::bake_environment` convolves the irradiance cube with a shader that derives its
normal as `normalize(vec3(clip_pos.x, clip_pos.y, 1.0))` — **z hard-pinned to 1.0**
(`astraweave-render/src/ibl.rs:1058`) — and the irradiance pass binds **no face-index uniform**
(shader declares only `env_cube` + `samp`, `ibl.rs:1051-1052`; the bake loop issues six
byte-identical draws, `ibl.rs:821-851`, binding only `env_bg` at `:848`). All six faces of the
irradiance cube therefore receive the **same +Z-hemisphere convolution**. The in-source comment
*"This works because we render one face at a time"* (`ibl.rs:1056`) is false — nothing
communicates *which* face to the shader. Consequence for the ratified change: terrain sampling
`irradiance_cube` by its world normal gets light convolved around a wrong, fixed basis — flat
(+Y-facing) ground receives the environment's **horizon-region** average instead of the sky
overhead, and because the six faces hold identical content the field is discontinuous at cube
seams, so slopes whose normals cross a face boundary would show hue/level steps in the ambient
term. The "flat → form shading returns" half of T2F option B's rationale would be delivered in
a wrong basis. The **specular half is correct** (see §2), so wiring at HEAD would pair correct
reflections with a wrong diffuse — and the beat's A/B would be measuring a defect.

## 1. Premise verification at HEAD (Phase 0 item 1)

| Premise (from the beat / T2F) | Verdict | Evidence |
|---|---|---|
| `ibl.rs` provides irradiance convolution, specular prefilter, BRDF LUT, quality tiers | **Partially holds** | Bake chain exists end-to-end (`ibl.rs:543-983`); tiers Low/Med/High = env 256/512/1024, spec 128/256/512 (mips 8/9/10), irradiance 64, LUT 256 (`ibl.rs:34-53`). **But the irradiance convolution is face-blind — §0/§2.** |
| Editor never bakes at init | **Holds** | Exactly two editor bake sites, both user-initiated: `load_hdri` (`engine_adapter.rs:3780-3781`) and `clear_hdri` (`viewport/renderer.rs:1556-1558`). `Renderer::new` sets `ibl_resources: None` (`renderer.rs:3438`). |
| A bake is callable by the editor today | **Holds** | `bake_environment` carries no feature gate (`ibl.rs:543`, `renderer.rs:3650`); the non-default `ibl` cargo feature gates only the three dormant `ensure_*` helpers (`ibl.rs:473/494/517`). |
| HDRI assets available; neutral daylight present in checkout | **Holds — better than required** | `assets/hdri/polyhaven/kloppenheim_02_puresky_2k.hdr` (catalog entry `kloppenheim_daytime`, day/grassland, `assets/hdri/hdri_catalog.toml:13-19`) is **git-tracked** — present even in a bare clone, no pack fetch needed (`git ls-files assets/hdri`; 5,563,538 B on disk; CC0 per `THIRD_PARTY_LICENSES.md:64-67`). Starter profile additionally fetches the other 7 catalog HDRIs. |
| Terrain's environment term is the flat ambient worth ~4% | **Holds** | `pbr_terrain_forward.wgsl:396-397`; terrain binds groups 0/1/2 only (`:103-128`), no IBL. |
| Recon cost estimate ~+0.5 ms | Untested (blocked before Phase 4) | — |
| T2F: static IBL "multiplies 1×1 **black** fallbacks → contributes exactly zero" | **FALSIFIED** | The fallback cube is a **sky-fill**, not black: +Y `(140,160,200)`, sides `(100,115,140)`, −Y `(70,65,50)` sRGB (`renderer.rs:2167-2174`, landed 2026-04-10 in `07b3d2137` — *before* T.2f, so the claim was wrong when written; the stale comment at `renderer.rs:2139` still says "black"). Fallback BRDF LUT `[255,0,0,255]` → `spec = prefiltered * F` (`renderer.rs:2214`). T2F §1.2 row 6 (never bakes) stands; §5 option B's "currently multiplying black" does not. |

## 2. The defect, precisely (all cited, verified first-hand this session)

Three of the four bake passes are face-correct; irradiance is the outlier:

| Pass | Face handling | Cite |
|---|---|---|
| Equirect→cube (HDRI) | ✅ per-face uniform `u_face.idx` + `uv_to_dir` face table | `ibl.rs:1215-1216, 1227-1234, 1246` |
| Specular prefilter | ✅ `params.face_idx` + `uv_to_cube_dir` (same table) | `ibl.rs:1106-1107, 1133-1141, 1144` |
| BRDF LUT | n/a (2D) | `ibl.rs:1172+` |
| **Irradiance convolution** | ❌ **no face uniform; N = normalize(clip.x, clip.y, 1.0)** | `ibl.rs:1041-1101` (N at `:1058`); loop `ibl.rs:821-851` |

Secondary, related findings (context for ratification, not the STOP trigger):

1. **`SKY_WGSL` (procedural source) is a self-labelled placeholder and also face-blind** —
   `dir = normalize(vec3(uv.x*2-1, **1.0**, uv.y*2-1))`, zero bind groups
   (`ibl.rs:1027-1036`, pipeline layout `ibl.rs:273-277`). A procedural bake yields a
   near-isotropic blue cube (`dir.y ∈ [0.577,1]` → ≈ the zenith constant). Sidestepped by
   baking the HDRI, but it means today's **"Remove HDRI" editor path already rebakes into this
   placeholder** (`viewport/renderer.rs:1552-1558`), and `IblManager.sun_elevation/azimuth`
   are dead fields (written `ibl.rs:442-443`, never readable — no bindings).
2. **Specular solid-angle hardcode**: `texel_solid_angle = 4π/(6·512·512) // Assume 512`
   (`ibl.rs:1159`) — exact for **Medium** (env 512, the tier L.2 would use, matching both
   existing editor bake sites), off by ±2 mips of bias for Low/High.
3. **`compute_hdr_avg_luminance` clamps HDR to 8-bit** (`ibl.rs:1259+` via `to_rgba8()`),
   so `ibl_intensity = clamp(0.35/avg, 0.3, 3.0)` (`renderer.rs:3683-3686`) is computed from
   an LDR proxy. Deterministic bias, director has the L.1 exposure slider; non-blocking.
4. **A bake flips the visible sky** — `draw_into` selects `sky_tex` from `ibl_resources`
   (`renderer.rs:5830`) and `SkyRenderer` prefers the HDR equirect (`environment.rs:602-721`),
   so bake-at-init makes the editor sky the HDRI (static; the ToD slider keeps driving sun/fog
   but no longer the sky's appearance). Same behaviour a manual "Load HDRI" produces today;
   as a *default* it needs the director's eyes. The A/B must split sky vs ground pixels.
5. **Catalog default-name defect**: `hdri_catalog.toml:7` `default = "kloppenheim_02_puresky"`
   matches no `[[hdri]].name` (the entry is `kloppenheim_daytime`); masked by the fallback
   matrix. L.2's design cites the file directly and does not depend on catalog resolution.

## 3. The fix, sized (presented, not executed)

Mirror the two face-correct passes in the same file — this is extension by in-file convention,
~55-65 lines total:

- **WGSL**: `IRRADIANCE_WGSL` gains `struct FaceIndex { idx: u32 }` at `@group(1) @binding(0)`
  (the equirect shader's exact shape, `ibl.rs:1215-1216`), passes `uv` from the vertex stage,
  and derives `N = uv_to_dir(face, uv)` using the same face table already duplicated in the
  equirect and specular shaders (`ibl.rs:1227-1234` / `:1133-1141`).
- **Rust**: the irradiance pipeline layout gains the existing face-uniform BGL; the bake loop
  writes a per-face 16-B uniform + bind group exactly as the equirect pass does
  (`ibl.rs:742-756`).
- **Falsification test**: a GPU test baking the tracked HDRI and asserting the six irradiance
  faces are **not** byte-identical and +Y is brighter than −Y for a sky HDRI (readback per
  face). Fails at HEAD by construction.

Risk: changes the numeric output of every future HDRI bake (that is the point). Non-editor
bake consumers (`hello_companion` visual demo, `veilweaver_demo`, `biome_gpu_demo`) get the
corrected irradiance too — a visual improvement, but stated for blast-radius honesty.

## 4. Options

**A (recommended) — ratify the irradiance fix as part of L.2, one beat.** The fix is small,
uses the file's own established pattern, is independently falsifiable, and L.2's A/B stations
then measure the *correct* split-sum the ratified decision intended. Without it, any L.2
numbers would have to be re-measured after the fix anyway.

**B — proceed wiring at HEAD without the fix.** Not recommended: correct reflections
multiplied against a wrong-basis diffuse, face-seam steps on slopes, and a permanently tainted
A/B baseline.

**C — separate ibl-fix beat, then L.2.** Technically clean but costs an extra round-trip for
~60 lines that L.2's own harness would validate; no advantage over A unless the director wants
the fix reviewed in isolation.

Separately ratifiable (not required for L.2): the `SKY_WGSL` placeholder fix (affects only the
"Remove HDRI" rebake path, already producing the placeholder today), and the
`compute_hdr_avg_luminance` LDR clamp.

## 5. The ready-to-execute L.2 design (contingent on ratification, for one-round-trip approval)

1. **Bake source**: `assets/hdri/polyhaven/kloppenheim_02_puresky_2k.hdr` — catalog entry
   `kloppenheim_daytime`, neutral daylight, **git-tracked** (present in a bare clone), CC0.
   Path probed like the harnesses probe assets (`assets`, `../../assets`); **absence →
   `tracing::warn!` + skip** — terrain then samples the existing sky-fill fallback bind group,
   and a fresh clone still launches. Failure policy: warn-and-continue, never fail adapter init.
2. **Bake seam**: inside `EngineRenderAdapter::new` after the preset apply
   (`engine_adapter.rs:785`), so every construction path (live editor + all harnesses) bakes
   exactly once; `tracing::info!` with source, tier, and wall-clock; init cost measured and
   reported. Tier **Medium** — the convention of both existing editor bake sites.
3. **Bind**: a dedicated 5-entry terrain IBL bind group at **group(3)** (spec cube, irradiance
   cube, BRDF LUT, sampler, the *same* `ibl_params_buf`), owned by `Renderer` and rebuilt
   inside `rebuild_ibl_bind_group` itself — one owner, one rebuild point (§7.7-compliant; the
   views and params buffer are shared, only the binding table is terrain-shaped). Reusing the
   full 9-entry group-5 layout instead would put the terrain fragment stage at **16/16**
   sampled textures (3 layer arrays + 8 splats + 5) — at the device default limit, blocking
   L.3's shadow map; the 5-entry group lands at 14/16.
4. **Sample**: hoist `compute_ibl` (`renderer.rs:179-203`) verbatim into a shared
   `shaders/ibl_common.wgsl` consumed by both `SHADER_SRC` and the terrain concat (CLAUDE.md
   second-implementation rule; flagged independently by recon). Terrain declares the group(3)
   bindings under the same names. Replacement at `pbr_terrain_forward.wgsl:395-397`:
   the flat ambient is **replaced** (no double-count) by
   `compute_ibl(N, V, base_color, metallic, roughness, F0) * final_ao` — AO applied to the
   indirect terms only, per the beat's directive (this deviates from the static path, which has
   no AO signal; deviation stated). Consequence to surface: the World-panel ambient sliders
   stop affecting terrain pixels (they keep affecting statics' ambient floor) — honesty note
   for the panel, director's call on UI treatment.
5. **A/B**: the four L.1 stations (BEFORE already banked — §6), whole-frame + ground-crop
   metrics, sky/ground split (bake flips the sky, §2 item 4), AO-forced-to-1.0 delta (the
   number that was 0.00%), normals-debug capture (terrain pixels must be bit-identical; any
   diff isolates the sky). Perf: `t2df_perf_main_pass` method before/after on the 1660 Ti;
   STOP above ~1.5 ms per the beat.

## 6. Banked evidence

- **BEFORE captures** at the four pinned stations, `d:/tmp/l2_staging/before/` — mean lumas
  114.31 / 112.63 / 82.31 / 83.93 (`desert_boundary_y414`, `desert_close_20m`,
  `grass_close_20m`, `grass_mid_47m`) — byte-stable vs the L.1 record, harness
  `l1_proof::l1_neutrality_stations` on the min-spec adapter (GTX 1660 Ti Max-Q · Vulkan ·
  592.82), `1 passed; 2 filtered out`, 117.84 s.
- Phase 0 recon (5 parallel readers, all claims re-verified against HEAD where load-bearing;
  the STOP-triggering facts verified first-hand in this session's main context).

## 7. What this session changed

This document only. No shader, code, or asset changes; `git status` clean before this commit
(sole untracked file: the pre-existing `.recent_files.json`). The L.1 baseline is untouched.
