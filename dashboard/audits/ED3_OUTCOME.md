# ED-3 — No inert controls: shading modes, debug views, amplitude authority (outcome)

> **Beat:** ED-3 (editor tooling) · **Date:** 2026-07-26 · **Baseline commit:** `aca627436`
> **Governing principle (director):** a control that does nothing is worse than no control — each of
> the three ships working or gone. **Outcome: all three ship WORKING.** Nothing was removed; the one
> conditional surface (Wireframe) is feature-gated in the UI so it can never silently no-op.
> Anti-drift honoured: no other editor features, no renderer changes beyond what the modes require,
> no second render path, the ED-3 defects only.

---

## 0. What shipped

| concern | disposition | proof |
|---|---|---|
| 1 — shading dropdown (Lit/Unlit/Wireframe) | **All three work.** Unlit via the scene-env debug uniform; Wireframe via polygon-mode-Line pipeline variants, feature-gated in the UI | GPU: Unlit differs from Lit in **100%** of pixels, Wireframe **99.82%** |
| 2 — phantom Normals / UVs | **Both built** (world-space normals; UV visualisation), through the live viewport path, pinnable/capturable like any frame | GPU: each differs from Lit in **100%** of pixels; captures at `d:/tmp/ed3_staging/modes/` |
| 3 — dead Base Amplitude slider | **Re-plumbed as a multiplier on the spline-derived amplitude** (default 1.0), labeled "Amplitude × (spline scale)"; archetype splines stay the authority | Renders at 0.5/1.0/2.0 differ pairwise by **99.5-99.6%** of pixels — the exact check that proved the old slider dead now proves the new one alive |

**T.3 readiness: the amplitude-finality gate is now actionable.** The director can set 0.8 / 1.0 / 1.2 in the Terrain panel and see genuinely different worlds (regeneration per value), then confirm or amend 1.0 at the gate. The authority remains the ratified per-archetype splines (E3 Phase A.2); the slider is an explicit, labeled scale on top of them.

---

## 1. Concern 1+2 — the shading modes, and how they route

### 1.1 The mechanism: no second render path, no layout change

The scene-env UBO had a spare pad float (`_pad_align[0]` / WGSL `_pad1x`, offset 68 of the 96-byte struct — the same trick `wetness`/`snow_amount` used for `_pad0`). It is now **`debug_mode`**: 0 = lit, 1 = unlit albedo, 2 = world-space normals, 3 = UVs. Layout unchanged (size asserted at 96 B by test), so **zero** bind-group/pipeline-layout churn, and the value flows to the terrain shader for free through the existing `bytemuck::cast` into `TerrainSceneEnvGpu`.

Every surface the editor shades branches on it **uniformly, inside the same fragment shaders, in the same passes**:

- static PBR (`SHADER_SRC`), after material sampling — so Unlit shows the true sampled albedo;
- skinned (`SKINNED_SHADER_SRC`) — note the skinned vertex format carries **no UVs**, so mode 3 renders a mid-gray sentinel there rather than lying with garbage (documented in the enum); the skinned pipeline is additionally dormant today (created, never set — pre-existing);
- terrain forward (`pbr_terrain_forward.wgsl`) — the UV view shows the **chunk parameterization** (`fract(in.uv)`; per-layer tiling multiplies it by a uniform `uv_scale`), which is the level where wrap/scale/seam problems live.

The plumbing chain that was severed is now: toolbar `ShadingMode` → `widget.rs` → `ViewportRenderer::render(..., shading_mode)` (**the formerly `_shading_mode: u32` dead parameter — now consumed**) → `EngineRenderAdapter::set_debug_shading` → `Renderer::set_debug_shading` → scene-env UBO + wireframe pipeline swap.

**Labeling:** the UI entry is **"Normals (world)"** — world-space, N·0.5+0.5. One honest caveat, stated rather than hidden: debug encodings pass through the live tonemap chain like every other fragment (a direct consequence of routing through the live path, which the beat mandates). The views are qualitative instruments — direction-to-color is monotonic and discontinuities scream — not decodable vector data.

### 1.2 Wireframe: implemented, feature-gated, never a silent no-op

The editor's device **already requested `POLYGON_MODE_LINE`** when the adapter supports it (`main.rs` device descriptor) — the feature was anticipated and never used. Now: polygon-mode-Line variants of the two live mesh pipelines (`pipeline-wire` for static meshes in `renderer.rs`; `terrain-forward-pipeline-wire` in `terrain_material_manager.rs`, rebuilt alongside the fill pipeline on format changes), selected per frame by `wireframe_enabled`.

On a device without the feature the variants are `None` — and the **UI hides the Wireframe entry** (`toolbar.wireframe_supported`, synced each frame from the renderer), because a mode that cannot work must not be offered. The internal fill fallback is therefore unreachable from the UI.

### 1.3 The two dropdowns are unified

`ShadingMode` is now `{Lit, Unlit, Wireframe, Normals, Uvs}`; the toolbar combo iterates `all()`. The docked Viewport panel's five-entry dropdown (`tab_viewer/mod.rs`) maps **1:1** onto the same enum (`main.rs`) — the silent `Normals/UVs → Lit` collapse is gone.

### 1.4 Visual verification (this session, min-spec adapter)

Station: desert oblique (T.2a anchor focal, dist 46.7, pitch 40°), 1024×768, radius-6 Desert. Captures + numbers:

| mode | mean luma | differs from Lit |
|---|---|---|
| Lit | 115.02 | — |
| Unlit | 195.20 | 786,432/786,432 (**100%**) |
| Wireframe | 175.75 | 784,986/786,432 (**99.82%**) |
| Normals (world) | 233.22 | 786,432/786,432 (**100%**) |
| UVs | 131.26 | 786,432/786,432 (**100%**) |

Inspected: Normals renders the Y-up field (pale green, slopes tinting cyan/yellow); UVs renders the R=U/G=V gradient with hard chunk-boundary resets clearly visible; Wireframe renders the 96×96 chunk grid as clean lit edges over the sky background. `wireframe_supported: true` on the GTX 1660 Ti Max-Q (Vulkan, driver 592.82).

---

## 2. Concern 3 — the amplitude lever

### 2.1 What was dead, and what is live now

The old slider wrote `config.noise.base_elevation.amplitude`, which the climate path never reads — the consumed amplitude is `params.base_elevation_amplitude`, blended from the per-archetype splines (`regional_archetype_mask.rs::blend_bootstrap_params`). Proven byte-identical across three values (T.2d Experiment F; `T2DF_OUTCOME.md` §8).

Per the ratified direction, the splines **stay the authority**. The new `NoiseConfig::base_amplitude_scale` (serde-default 1.0, so old configs parse) multiplies the spline-derived amplitude at its single consumption site (`noise_gen.rs::sample_height_with_params`):

```rust
height += noise_val * (params.base_elevation_amplitude * self.config.base_amplitude_scale);
```

`x * 1.0` is exact in IEEE 754, so the default is **bit-identical** to pre-ED-3 output — the D5FIX byte-identity discipline holds (asserted by test). Because the classification provisional height uses the same function, biome classification follows the scaled terrain rather than diverging from it (the §7.7-class seam stays closed).

The panel's control is relabeled **"Amplitude × (spline scale)"**, range 0.25–2.0, default 1.0, with a hover note; the old absolute slider is gone. `TerrainState::set_amplitude_scale` is the panel→generator seam.

### 2.2 Proof

- **Unit (CPU):** `ed3_base_amplitude_scale_is_live_and_identity_at_default` — scale 1.0 bit-identical to default; scale 0.5 changes generated heights. `1 passed` (astraweave-terrain).
- **Render (GPU, live path):** scales 0.5 / 1.0 / 2.0, same station: pairwise **99.49% / 99.60% / 99.57%** of pixels differ. Captures at `d:/tmp/ed3_staging/amplitude/`. On pre-ED-3 code this comparison came back byte-identical — that was the defect's own proof, inverted.

---

## 3. How the regression tests bite (fails-on-old-code, per class)

| test | how it fails on pre-fix code |
|---|---|
| `ed3_shading_modes_render_differently` (GPU) | Pre-fix, `render`'s shading parameter was `_shading_mode` (unused) — all five frames rendered **byte-identical**, so every `>1% differing pixels` assertion fails. |
| `ed3_debug_shading_reaches_every_shader_surface` (source) | Pre-fix, `debug_mode` had **0 occurrences** in `renderer.rs` and 0 in `pbr_terrain_forward.wgsl` (verified against `git show HEAD:…` at baseline `aca627436`) — the `contains` assertions fail trivially. Also pins the UBO at 96 B so the pad-commandeering can't silently grow the layout. |
| `ed3_base_amplitude_scale_is_live_and_identity_at_default` | Compile-fails pre-fix (`base_amplitude_scale`: 0 occurrences at baseline); at runtime, the `assert_ne!` on scaled heights is exactly the check Experiment F showed failing (identical output). |
| `ed3_amplitude_scale_changes_terrain` (GPU) | Pre-fix, three scales rendered byte-identical (Experiment F, DEGENERATE flag) — the pairwise `>5%` assertions all fail. |
| `test_shading_mode_values` (extended) | Pins the 0–4 contract between the toolbar and the render path, and the 5-mode cycle. |

---

## 4. Verification

| rung | result |
|---|---|
| `cargo fmt` (render, terrain, aw_editor) | clean |
| `cargo check --workspace` | **Finished, exit 0** |
| `astraweave-render --lib` | **1284 passed / 2 failed** — the 2 are the known environmental `Device(Lost)` water-device flakes (stash-proven pre-existing at HEAD in T.2d.F §6.1; pass in isolation). Count +1 vs T.2d.F (the new `ed3_debug_shading_reaches_every_shader_surface`); the `terrain_scene_env_gpu_field_offsets_match_shader_src` contract was extended for `debug_mode` @68 / `_pad1` @72 / size 96 and passes. |
| `astraweave-terrain --lib` | **800 passed / 7 failed / 3 ignored** — the 7 are the identical by-design baseline set from the T.2a ledger (golden-value / fbm-param / spline-baseline / highland); +1 = the new amplitude-scale test. |
| `aw_editor --lib` | **4035 passed / 0 failed / 5 ignored** — two pre-existing enum-shape tests (`test_shading_mode_all`, `test_shading_mode_cycle`) updated to the 5-mode contract. |
| naga validation of the edited terrain shader | `test_pbr_terrain_forward_validates_with_prefix`: **1 passed / 0 failed** |
| scene-env + terrain-manager layout suites | scene_environment **43 passed / 0 failed**; terrain_material_manager **12 passed / 0 failed** |
| ED-3 GPU proofs | **2 passed; 0 failed** (`ed3_proof.rs`, 141.5 s, min-spec GTX 1660 Ti Max-Q · Vulkan · driver 592.82) |
| pre-fix symbol absence | `git show HEAD:` at baseline `aca627436`: `debug_mode` **0** occurrences in `renderer.rs` and in `pbr_terrain_forward.wgsl`; `base_amplitude_scale` **0** in `noise_gen.rs` |
| captures | `d:/tmp/ed3_staging/{modes,amplitude}/` |

---

## 5. Residue

- **The debug views pass through the live tonemap** (§1.1) — accepted consequence of the no-second-path rule; revisit only if a diagnosis ever needs raw-value readback (that is ED-2 capture + offline analysis territory anyway).
- **Skinned meshes**: no UVs in the vertex format (mode 3 = sentinel), and the skinned pipeline itself is dormant (created, never drawn — pre-existing, recorded).
- **`ShadingMode::has_lighting`** still has zero non-test callers (pre-existing helper; harmless).
- The old `base_elevation.amplitude` field and `set_noise_params`'s amplitude argument still exist for the legacy (non-climate) noise path; the editor UI no longer exposes them. A future cleanup could drop the argument if the legacy path is ever retired — not this beat's call.
