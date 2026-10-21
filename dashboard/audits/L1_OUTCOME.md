# L.1 — Lighting calibration & honesty (outcome)

> **Beat:** L.1 (lighting lane, first of L.1 → L.2 IBL → L.3 CSM) · **Date:** 2026-07-26
> **Baseline commit:** `1d8e1c792` (T.2f recon) · **Spec:** `docs/audits/T2F_LIGHTING_RECON.md` §3, §1.2 rows 8/10/11, §5 option E, §7
> **Binding constraint:** visually neutral at defaults. **Result: bit-perfect — 0 differing
> pixels** (not merely ≤1 LSB) across all four pinned stations, 2,841,258 pixels compared (§4.1).
> Anti-drift honoured: no IBL, no shadow work, no material/terrain/water work, no rig redesign;
> dormant SSAO/TAA/bloom machinery untouched — only the false claims about it were removed.

---

## 0. What shipped

| concern | disposition | proof |
|---|---|---|
| 1 — frame-1 lighting-push race | **FIXED by defer-and-deliver**: a push arriving before the async engine adapter exists is PARKED (`ViewportRenderer::pending_lighting_params`) and delivered by `init_engine_adapter`; parking and delivery are both logged (`tracing::info`). Never silently dropped; the caller's change-cache stays truthful by construction. Panel defaults reconciled to the delivered state via pinned constants. | `l1_late_push_lands`: pre-init push of dark params renders mean **46.05** post-fix vs **112.63** (default brightness = push dropped) at the pre-L.1 commit — the same test run at `1d8e1c792` FAILED with exactly that number (§3) |
| 2 — inert Exposure slider | **LIVE end-to-end**: slider → `TerrainLightingParams.exposure` → `set_lighting_params` → `SceneEnvironment.exposure` → scene-env UBO offset 72 (the second commandeered pad float — ED-3's `debug_mode` pattern; 96-B layout unchanged) → `uPostScene.exposure` in both POST shaders. Default pinned to **1.35**, the exact former hardcode. | `l1_exposure_is_live`: 0.5 / 1.35 / 3.0 → mean luma 54.91 / 112.63 / 168.58, pairwise **100.00%** differing pixels; at the pre-L.1 commit the same three frames were **byte-identical** (0/786,432) |
| 3 — preset fictions | **DELETED**: `EditorTerrain` + `EditorDefault` no longer set `ssao_enabled: true` (no SSAO pass exists — the renderer consumes only `bloom_enabled`); `GameQuality` no longer sets `taa_enabled: true` (no TAA pass exists). Flags zeroed (behaviour-neutral — nothing read them), preset comments, enum variant docs, and `set_post_process_chain`'s doc rewritten to state what actually runs. Machinery untouched. | flags were write-only (T2F §1.2 row 8; renderer.rs has exactly one `post_chain.` read) — zeroing them cannot change a frame, and §4.1's 0-pixel diff confirms |

**Design statement (Concern 1, as the beat asked):** the fix is *defer-and-deliver at the layer
that owns the adapter option*, not retry-from-the-caller. `ViewportRenderer` is the only layer
that knows whether the adapter exists, so it parks the push and `init_engine_adapter` drains it;
`main.rs`'s cached-compare then needs no change and its cache is truthful by construction (a
cached push is a push that has been or will be delivered). An undeliverable push is observable:
parking and delivery each emit a `tracing::info` line.

## 1. The pinned defaults (the single source of truth)

`tools/aw_editor/src/viewport/types.rs` now carries the delivered-state constants — used by the
World panel's startup defaults, `TerrainLightingParams::default()`, and the engine adapter's
terrain-upload block:

| constant | value | provenance |
|---|---|---|
| `DEFAULT_SUN_DIR` | [0.5, 0.6, 0.4] (TO-sun) | negated+normalized ≡ the terrain-upload hardcode `normalize(-0.5,-0.6,-0.4)` — **bit-identity asserted by test** |
| `DEFAULT_SUN_COLOR` / `DEFAULT_SUN_INTENSITY` | [1.0, 0.98, 0.9] × 1.0 | `SceneEnvironment::default()` — what the renderer delivers |
| `DEFAULT_AMBIENT_COLOR` / `_INTENSITY` | [0.45, 0.50, 0.55] × 0.35 | the terrain-upload block (every T-series judgment's state) |
| `DEFAULT_EXPOSURE` | 1.35 | the former POST hardcode; equality with `astraweave_render::scene_environment::DEFAULT_EXPOSURE` asserted by test |
| `DEFAULT_SUN_ELEVATION_DEG` / `_AZIMUTH_DEG` | 43.15° / 38.66° | `DEFAULT_SUN_DIR` in panel coordinates; panel-trig round-trip within <0.1° (asserted) |

The panel therefore displays the truth from frame 1. Pre-L.1 it displayed sun 2.2 ×
[1.0, 0.96, 0.88], ambient [0.65, 0.58, 0.50] × 0.45, exposure 1.3 — values **no path delivered**
(T2F §3). Residuals, stated: the panel's elevation/azimuth reconstruct the sun direction through
trig, so a user *touching* a sun slider re-derives a direction within <0.1° of the hardcode
(sub-visible; the delivered direction stays the upload hardcode until then). The "Bright Day /
Golden Hour / Overcast" preset buttons are explicit user actions, not defaults — left as
creative presets. `set_light_direction_override`'s write-only intensity argument is now
documented at the call site (T2F §7 item 5, still open by design).

## 2. Exposure: range and mechanism

- **Mechanism:** the scene-env UBO's second pad float (offset 72) — the identical trick ED-3
  used for `debug_mode` at offset 68. Zero new bindings, zero layout churn (96 B pinned by
  test); the post pass already binds this UBO at group(1) (`scene_env_bg`), so the value
  arrives for free. `TerrainSceneEnvGpu` mirrors the field for byte parity (terrain never
  reads it — exposure is a post-pass concern; noted in the WGSL struct).
- **Range:** the pre-existing slider contract **0.1–3.0** is kept (the beat suggested
  ~0.5–3.0). Justification: 3.0 reaches deep into the ACES shoulder, 0.5 reaches the toe, and
  the 0.1 bottom is a cheap diagnostic black-out; narrowing an existing contract bought
  nothing. At the default 1.35 the curve position is exactly the pre-L.1 one.
- Both `POST_SHADER` and `POST_SHADER_FX` (postfx feature) read the uniform; the source-level
  contract (`uPostScene.exposure` present, `let exposure = 1.35` absent) is enforced by tests
  in both cfg branches.

## 3. How each proof fails on pre-fix code (demonstrated, not argued)

The full `l1_proof.rs` suite was **run at the pre-L.1 commit `1d8e1c792`** before any change:

| test | result at `1d8e1c792` | result post-L.1 |
|---|---|---|
| `l1_exposure_is_live` | **FAILED** — exposure 0.50/1.35/3.00 rendered 0/786,432 differing pixels (the inert slider's own signature) | ok — pairwise 100.00% |
| `l1_late_push_lands` | **FAILED** — "mean luma 112.63 indicates the push was dropped" (the race's own signature) | ok — mean 46.05 |
| `l1_neutrality_stations` | ok (captures `before/`) | ok (captures `after/`) |
| honesty invariant (inside the exposure test) | 0 differing pixels — the pinned constants were verified to BE the delivered state even pre-fix | 0 differing pixels |

CPU-side regression tests (run in the suites): `types.rs::l1_lighting_defaults_tests` (4 —
bit-identity of the pinned direction, cross-crate exposure-constant equality, delivered-state
defaults, elevation/azimuth round-trip <0.1°); `scene_environment.rs` L.1 tests (exposure at
byte offset 72, default = 1.35 everywhere, env→UBO flow, POST-shader consumption contract);
`terrain_material_manager` offsets contract extended (`exposure` @72, `_pad1` @76, size 96).

## 4. Verification

### 4.1 The neutrality proof (headline)

Default-state captures at the pinned stations, pre-L.1 (`1d8e1c792` build) vs post-L.1, same
harness, min-spec GPU (GTX 1660 Ti Max-Q · Vulkan · 592.82):

| station | pixels compared | differing | max channel delta |
|---|---|---|---|
| desert boundary_y414 (director's recovered camera, radius-10 world, 962×501) | 481,962 | **0** | **0** |
| desert close 20 m (T.2a anchor, 1024×768) | 786,432 | **0** | **0** |
| grass close 20 m (T.2f station) | 786,432 | **0** | **0** |
| grass mid 47 m (T.2f station) | 786,432 | **0** | **0** |

Frames: `d:/tmp/l1_staging/{before,after}/`. The criterion was ≤1 LSB; the result is
bit-identical. L.2's IBL A/B baseline is therefore exactly the T-series baseline.

### 4.2 Suites and rungs

| rung | result |
|---|---|
| `cargo fmt` (render, aw_editor) | applied |
| `cargo check -p astraweave-render` / `-p aw_editor` | exit 0 each |
| `cargo check --workspace` | **Finished, exit 0** |
| `astraweave-render --lib` | **1288 passed / 2 failed** — the 2 are the known environmental `Device(Lost)` water-device flakes (stash-proven pre-existing, T2DF_OUTCOME §6.1; membership rotates per run); +4 vs the ED-3 baseline = the four new L.1 tests |
| `aw_editor --lib` | **4039 passed / 0 failed / 5 ignored**; +4 vs ED-3 baseline = the four `types.rs` L.1 tests |
| clippy | **L.1 adds ZERO findings — stash-proven**: at clean baseline `1d8e1c792` the canonical `-D warnings` invocations are already red (aw_editor `--no-deps`: 62 errors/68 sites; astraweave-render: 3 errors — impostor `div_ceil` + `too_many_arguments`, `duplicated_attributes` at terrain_material_manager.rs:40; `--all-features` additionally fails compiling the `egui-winit` dependency, E0027). Post-L.1 the per-file error distribution is **byte-identical to baseline** (`Compare-Object` empty, 62/68 both sides); none of the error sites is in an L.1 hunk. Pre-existing surface, consistent with the E3-PF ledger's "Clippy-red not terrain-caused"; recorded in §6 for the HEALTH lane, not fixed here (anti-drift). |
| GPU proofs (`l1_proof.rs`) | **3 passed / 0 failed** post-fix (147.7 s); **1 passed / 2 failed** at `1d8e1c792` — the two failures being the defects' own signatures (§3) |
| trace-sync | `aw_trace_sync --check`: in sync (26 traces validated) |

## 5. L.2 readiness — explicit statement

**Yes: the scene state is now stable and honest enough that an IBL A/B can be trusted.**
Specifically:

1. **The baseline cannot silently shift.** The +42% bistable-sun jump is gone: the frame-1 push
   is delivered (with the reconciled values it is a bit-exact no-op), and any *future* panel
   change is an explicit, logged, user-initiated act. L.2's before/after frames compare against
   a scene that renders bit-identically to every T-series capture (§4.1).
2. **The panel is truthful.** What the Lighting section displays is what the renderer holds —
   pinned by constants and tests — so "what state was the editor in?" is answerable from the UI.
3. **Exposure is a controlled variable.** L.2's IBL will change scene radiance; the director can
   now place the result on the ACES curve deliberately (and any exposure used in a judgment is
   recorded in the panel state) instead of being welded to 1.35.
4. **The attribution field is clean.** Nothing else in the render path changed: presets alter
   no passes they didn't before (flag zeroing touched write-only state), and the cascade-splits
   behaviour is unchanged (left running, documented why — it delivers the shadows-off sentinel;
   L.3 will consume the cascades).

## 6. Residue

- T2F §7 ledger updated in place: items 1–3 struck CLOSED; **4** (bloom uncomposited), **5**
  (write-only override intensity), **6** (cascade splits — now documented in code as
  deliberately-left), **7** (statics sample no texture AO) remain open for the ED-x/HEALTH/L.3
  lanes.
- The sibling caches (`cached_sky_colors`, `cached_fog_params`, `cached_weather_kind` in
  `main.rs`) share the same drop-then-cache shape for *their* frame-1 pushes. Not fixed here —
  the beat names the lighting push, and their delivered defaults happen to match steady state
  post-terrain-upload (fog/sky are overwritten by the upload block the same way ambient is) —
  but the pattern is now on record; flagged for the ED-x lane.
- The World panel presets ("Bright Day" etc.) intentionally produce non-default states — they
  are user actions, not lies. Recorded so nobody "reconciles" them later.
- **The workspace's canonical clippy command is red at HEAD, independent of L.1** (stash-proven,
  §4.2): 62 pre-existing errors in aw_editor, 3 in astraweave-render, plus an `egui-winit`
  dependency compile failure under `--all-features` feature-unification (E0027 — egui version
  drift). HEALTH-lane item; matches the E3-PF close-out ledger's standing "Clippy-red not
  terrain-caused" note. L.1's contribution verified zero by per-file distribution comparison.
