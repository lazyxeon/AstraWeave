# Tonemap Double-Application Investigation

**Date**: 2026-04-19
**Subject**: Does `astraweave_render::Renderer::draw_into` apply tonemapping when the external view is `Rgba16Float` (editor mode, `surface = None`)?
**Precondition**: Prior audit, `docs/audits/editor_viewport_render_divergence_2026-04-19.md` §10 Appendix Item 1, flagged this as a potential colour-correctness bug.
**Harness**: [astraweave-render/examples/tonemap_probe.rs](../../astraweave-render/examples/tonemap_probe.rs)

---

## 1. Executive finding

**H2 confirmed.** `Renderer::draw_into` **does not tonemap** when the external view is `Rgba16Float` and `surface = None`. It emits linear HDR, preserving input values up to at least 983.0 in the red channel (observed). The editor's `viewport/shaders/tonemap.wgsl` pass is therefore the **single, correct** tonemap application in the editor pipeline. There is no double-tonemapping.

Additionally, the `PostProcessChain.tonemap_operator` field **has no effect** on output in editor mode: swapping it between `Aces` and `None` produces byte-identical readbacks. This confirms the engine's editor-mode handoff uses the `hdr_blit_pipeline` passthrough shader, not the `post_pipeline` ACES shader.

---

## 2. Static analysis (Phase 1)

### 2.1 `Renderer::draw_into` branches on `self.surface.is_none()`

The explicit branch that determines whether tonemapping happens inside `draw_into` is [astraweave-render/src/renderer.rs:5757-5794](../../astraweave-render/src/renderer.rs#L5757-L5794):

```rust
// Blit internal HDR → external view.
// Editor mode (surface=None): use passthrough blit (no tonemapping) —
// the editor has its own tonemap pass.
// Standalone mode: use the full post pipeline (ACES tonemap + tint).
{
    let mut pp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("post pass (external)"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            ...
        })],
        ...
    });
    if self.surface.is_none() {
        // Editor: passthrough blit (HDR → HDR, no tonemap)
        pp.set_pipeline(&self.hdr_blit_pipeline);
        pp.set_bind_group(0, &self.hdr_blit_bind_group, &[]);
        pp.draw(0..3, 0..1);
    } else {
        // Standalone: full post-processing (tonemap + tint)
        pp.set_pipeline(&self.post_pipeline);
        ...
    }
}
```

### 2.2 The two pipelines

- **`post_pipeline`** ([renderer.rs:2268-2298](../../astraweave-render/src/renderer.rs#L2268-L2298)) is built against `POST_SHADER` ([renderer.rs:331-384](../../astraweave-render/src/renderer.rs#L331-L384)) or `POST_SHADER_FX` ([renderer.rs:387-445](../../astraweave-render/src/renderer.rs#L387-L445)) when the default `postfx` feature is on. Both shaders call `aces_tonemap(hdr * 1.35)` and clamp to `[0, 1]`. The pipeline's target format is `config.format` (Bgra8UnormSrgb in the editor's configuration — a format mismatch if this path were taken).
- **`hdr_blit_pipeline`** ([renderer.rs:2300-2425](../../astraweave-render/src/renderer.rs#L2300-L2425)) is built against an inline WGSL shader defined at [renderer.rs:2308-2330](../../astraweave-render/src/renderer.rs#L2308-L2330):

    ```wgsl
    @fragment fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
        var color = textureSampleLevel(hdr_tex, samp, in.uv, 0.0);
        let bloom = textureSampleLevel(bloom_tex, samp, in.uv, 0.0);
        color = vec4(color.rgb + bloom.rgb * pfx.bloom_intensity, color.a);
        return color;
    }
    ```

  No tonemap, no exposure, no clamp. An **additive bloom composite** and nothing else. When `post_chain.bloom_enabled == false`, the bloom binding is the 1×1 black `postfx_dummy_view` with `bloom_intensity = 0.0` (see [renderer.rs:2388](../../astraweave-render/src/renderer.rs#L2388) and the intensity plumbing at [renderer.rs:5750-5755](../../astraweave-render/src/renderer.rs#L5750-L5755)), reducing the shader to a pure passthrough. Target format is `Rgba16Float` ([renderer.rs:2415](../../astraweave-render/src/renderer.rs#L2415)).

### 2.3 `HdrPipeline::tonemap_pass` is dormant

[astraweave-render/src/hdr_pipeline.rs:348-399](../../astraweave-render/src/hdr_pipeline.rs#L348-L399) defines a separate tonemap pass with a richer shader (exposure + tonemap + color grading). **It is never called from `renderer.rs`.** Grep confirms zero `tonemap_pass(` call sites in the renderer module. `HdrPipeline` is allocated but its main entry point is not wired into the frame pipeline; the active tonemap path is the `POST_SHADER` route inside `post_pipeline`.

### 2.4 Editor handoff confirms `surface = None`

[tools/aw_editor/src/viewport/engine_adapter.rs:677-679](../../tools/aw_editor/src/viewport/engine_adapter.rs#L677-L679) constructs the engine with `surface = None`:

```rust
let renderer_result =
    astraweave_render::Renderer::new_from_device(device_owned, queue_owned, None, config)
        .await;
```

and the editor always uses this construction path (no alternate code path passes a `Surface`).

### 2.5 All four `PostProcessChain` presets hard-code `Aces`

[tools/aw_editor/src/viewport/engine_adapter.rs:921-1039](../../tools/aw_editor/src/viewport/engine_adapter.rs#L921-L1039) — the four `apply_quality_preset` arms each construct a full `PostProcessChain { ..., tonemap_operator: TonemapOperator::Aces }`. None of them sets a field that could disable the tonemap pass *inside the chain*, but that is moot because — per §2.1 — the `chain.tonemap_operator` is consumed only by `post_pipeline`, which is not reached when `surface = None`.

### 2.6 Pre-run hypothesis

Given (2.1) — (2.5), the static-analysis hypothesis is **H2**: `draw_into` writes linear HDR into the external `Rgba16Float` view. The editor's own `tonemap.wgsl` is the sole tonemap pass, and no double-application occurs. Corollaries:

- Output in the external view will contain values > 1.0 when the scene radiance is high.
- Swapping `tonemap_operator` in the `PostProcessChain` should not affect the output in editor mode.

---

## 3. Harness (Phase 2–3)

### 3.1 Construction

[astraweave-render/examples/tonemap_probe.rs](../../astraweave-render/examples/tonemap_probe.rs) builds a headless wgpu device + queue, then constructs a `Renderer` exactly the way `EngineRenderAdapter::new` does: via `Renderer::new_from_device(device, queue, None, config)` with `config.format = Bgra8UnormSrgb`. It requests `max_bind_groups: 8` because the engine uses bind groups 4 and 5 (scene_env and IBL). No other engine modifications.

### 3.2 Deterministic HDR signal via the sky pass

The prompt's preferred approach — inject a known HDR ramp into the scene pass — would require either an engine-side test hook or carefully tuned emissive materials. The engine does not expose either, and adding a hook is out of scope. The harness instead generates a **deterministic HDR signal via `Renderer::set_sky_config`**, which writes linear RGB directly into the sky pass output:

```rust
let mut sky = SkyConfig::default();
sky.day_color_top      = vec3(sun_intensity * 50.0, sun_intensity * 50.0, sun_intensity * 50.0);
sky.day_color_horizon  = vec3(sun_intensity * 10.0, sun_intensity * 10.0, sun_intensity * 10.0);
sky.cloud_coverage     = 0.0;  // remove cloud modulation for a clean signal
renderer.set_sky_config(sky);
```

At `sun_intensity = 1.0` the zenith target is linear RGB = 50.0. At `sun_intensity = 20.0` it is 1000.0. The sky pass is scheduled *before* the main render pass inside `draw_into` ([renderer.rs:5465-5478](../../astraweave-render/src/renderer.rs#L5465-L5478)) and writes its output into the engine's internal `hdr_view`. The final blit copies the internal HDR to the external view via `hdr_blit_pipeline` (editor mode) or `post_pipeline` (standalone mode).

### 3.3 Probe configuration matrix

Four runs per harness execution:

| # | `PostProcessChain.tonemap_operator` | `sun_intensity` (HDR scale) |
|---|---|---|
| 1 | `Aces` | 1.0  (zenith = 50.0) |
| 2 | `None` | 1.0  (zenith = 50.0) |
| 3 | `Aces` | 20.0 (zenith = 1000.0) |
| 4 | `None` | 20.0 (zenith = 1000.0) |

Each run: render one warm-up frame (first-frame caches settle), then one measurement frame, then `copy_texture_to_buffer` the full 512×512 `Rgba16Float` view into a readback buffer, map, decode every pixel via `half::f16::to_f32`, compute (min, max, mean) red plus five horizontal-mid samples plus four corner samples plus one center sample plus a distinct-red-value count.

Bloom, SSAO, TAA, and color-grading are all disabled in the `PostProcessChain` so the signal is the sky pass output only, modulated by the sky shader's sampling.

### 3.4 Hardware

Adapter: NVIDIA GeForce GTX 1660 Ti with Max-Q Design (DiscreteGpu, Vulkan backend).

---

## 4. Raw measurements (Phase 3–4)

### 4.1 Measurement table

Red channel, 512×512 Rgba16Float external view, after `draw_into` returns:

| Probe | max R | min R | mean R | max G | max B | distinct R |
|---|---:|---:|---:|---:|---:|---:|
| ACES, sun=1.0   | 49.1562 | 26.6562 | 40.2291 | 49.1562 | 49.1562 | 892 |
| None, sun=1.0   | 49.1562 | 26.6562 | 40.2291 | 49.1562 | 49.1562 | 892 |
| ACES, sun=20.0  | 983.0000 | 533.0000 | 804.6221 | 983.0000 | 983.0000 | 901 |
| None, sun=20.0  | 983.0000 | 533.0000 | 804.6221 | 983.0000 | 983.0000 | 901 |

Horizontal mid-line red samples (`u=0.00, 0.25, 0.50, 0.75, 1.00`):

| Probe | u=0.00 | u=0.25 | u=0.50 | u=0.75 | u=1.00 |
|---|---:|---:|---:|---:|---:|
| ACES, sun=1.0   | 39.7812 | 41.6875 | 42.4375 | 41.6875 | 39.7812 |
| None, sun=1.0   | 39.7812 | 41.6875 | 42.4375 | 41.6875 | 39.7812 |
| ACES, sun=20.0  | 795.5000 | 834.0000 | 849.0000 | 833.5000 | 795.5000 |
| None, sun=20.0  | 795.5000 | 834.0000 | 849.0000 | 833.5000 | 795.5000 |

Corner + center red samples (`TL, TR, BL, BR, CE`):

| Probe | TL | TR | BL | BR | CE |
|---|---:|---:|---:|---:|---:|
| ACES, sun=1.0   | 46.6250 | 46.6250 | 26.6562 | 26.6562 | 42.4375 |
| None, sun=1.0   | 46.6250 | 46.6250 | 26.6562 | 26.6562 | 42.4375 |
| ACES, sun=20.0  | 932.5000 | 932.5000 | 533.0000 | 533.0000 | 849.0000 |
| None, sun=20.0  | 932.5000 | 932.5000 | 533.0000 | 533.0000 | 849.0000 |

### 4.2 Reference values for the alternate hypothesis (H1)

If the engine were tonemapping with ACES-Narkowicz into the external view, the red values would be bounded by the Narkowicz output, which clamps to `[0, 1]`. Using `f(x) = clamp((x*(2.51*x+0.03)) / (x*(2.43*x+0.59)+0.14), 0, 1)`:

| Input linear | ACES-Narkowicz output |
|---|---|
| 0.0 | 0.0000 |
| 1.0 | 0.6054 |
| 2.5 | 0.7933 |
| 4.0 | 0.8527 |
| 5.0 | 0.8732 |
| 50.0 | clamped → 1.0 |
| 1000.0 | clamped → 1.0 |

No H1-consistent scenario produces any value above 1.0. Observed maxima of 49.16 and 983.0 are 49× and 983× above the H1 ceiling.

### 4.3 Operator-swap delta

| Quantity | Value |
|---|---:|
| `|max_red(ACES, sun=20.0) − max_red(None, sun=20.0)|`   | **0.000000** |
| `|mean_red(ACES, sun=20.0) − mean_red(None, sun=20.0)|` | **0.000000** |

Identical to the full precision of the f16 readback path. All four probes at the same `sun_intensity` produce byte-identical images.

---

## 5. Conclusion

The evidence converges unambiguously on **H2**. Three independent observations establish it:

1. **HDR values survive `draw_into`.** The external `Rgba16Float` view contains values up to 983.0 after `draw_into` returns. No tonemap that clamps its output to `[0, 1]` could produce these values. The engine is writing linear HDR.
2. **`tonemap_operator` is a no-op in editor mode.** Swapping the operator between `Aces` and `None` produces identical readbacks at all sample points. This is consistent with the `hdr_blit_pipeline` passthrough shader being used (which does not consume the operator), and inconsistent with `post_pipeline` being used (which would).
3. **Output scales linearly with input.** 20× the sky radiance → 20× the observed output (50 → 1000 theoretical; 49.16 → 983.0 observed, a consistent ratio of ~20.0×). A tonemap would produce a compressed, non-linear mapping.

Combined with the static analysis — which identifies a single explicit branch at `renderer.rs:5783` that selects the passthrough pipeline when `self.surface.is_none()` — this closes the question.

**The editor's `viewport/shaders/tonemap.wgsl` is the sole tonemap application in the editor pipeline, and the engine honours its "editor has its own tonemap" contract.** The prior audit's §10 Appendix Item 1 flag of potential double-tonemapping is resolved as: not happening.

---

## 6. Appendix — incidental findings

These were observed during the investigation but are out of scope for remediation here.

1. **`HdrPipeline::tonemap_pass` is dead code.** The `HdrPipeline` struct in [astraweave-render/src/hdr_pipeline.rs](../../astraweave-render/src/hdr_pipeline.rs) allocates intermediate HDR targets, a tonemap uniform buffer, a bind-group layout, and a render pipeline — but `HdrPipeline::tonemap_pass` is never called from the `Renderer` module. The tonemap that actually runs in standalone mode is `POST_SHADER` embedded in `renderer.rs`. The `HdrPipeline` module appears to have been designed as the intended tonemap implementation but was never wired in.

2. **Three independent tonemap shader implementations exist in the workspace.** (a) `renderer.rs::POST_SHADER` — ACES only, exposure 1.35, used by `post_pipeline` in standalone mode. (b) `hdr_pipeline.rs::TONEMAP_SHADER` — ACES, AgX, Reinhard, None with color grading; dormant. (c) `tools/aw_editor/src/viewport/shaders/tonemap.wgsl` — ACES, PBR Neutral, Reinhard, AgX; active in editor. Each has its own operator index table; none are mutually consistent with the others.

3. **`Renderer::scene_environment_mut` changes to `sun_intensity` did not visibly affect the ground plane radiance at the tested camera angles.** Replacing the sun/ambient-boost approach with an HDR sky injection gave the signal the measurement needed, but the lit-shader path may have an issue where `scene_env.sun_intensity` isn't reaching the active pipeline under the default `postfx + textures` features. Not investigated further — the sky-injection path delivered a clean HDR signal that was sufficient to answer the binary question.

4. **Additive bloom composites even when `bloom_enabled = false`.** The `hdr_blit_pipeline` shader always computes `color + bloom * bloom_intensity`. When bloom is disabled, `bloom_intensity = 0.0` and `bloom_tex` is a 1×1 dummy, so the addition is a no-op. Not a correctness issue, but it means the `hdr_blit_pipeline` always samples two textures regardless of bloom state — a tiny wasted bandwidth cost.

5. **`Camera::view_matrix` is gimbal-unstable at `pitch = ±π/2`.** The initial harness attempted to render a straight-down camera and produced a degenerate view matrix (flat output with only ~2 distinct red values). Using a 45° pitch solved it. This is not a problem the editor would encounter in practice (its `OrbitCamera::to_engine_camera` constrains pitch similarly), but it is a latent footgun in the engine's `camera::Camera` API.

---

*End of report.*
