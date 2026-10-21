# W.2a — Water Surface: LOD/Chunking + Real Water-Level — Execution Report

**Campaign:** W-series (Water Successor) · **Phase:** W.2a (W.2 construction, sub-phase a) · **Mode:** render-code mutation (first of the campaign)
**Branch:** `campaign/water-successor` · **Scope authority:** [`W2_DECISIONS.md`](./W2_DECISIONS.md)
**Min-spec measurement box:** NVIDIA GeForce GTX 1660 Ti with Max-Q Design · Vulkan · DiscreteGpu · driver 592.27 (adapter printed by the probe, not assumed)
**Date:** 2026-06-22 · **Status:** complete, build green, committed, stopped at the W.2a gate

This report records the execution of the director's W.2a ratifications. It is the
forensic companion to [`W2_0_RECON.md`](./W2_0_RECON.md) and [`W2_DECISIONS.md`](./W2_DECISIONS.md).

---

## 1. The three changes

| # | Change | Where |
|---|---|---|
| 1 | **Chunked-LOD surface** — replaced the single hardcoded `generate_water_plane(500,128)` plane with a camera-distance discrete chunk grid (`CHUNK_SIZE=64`, `(2·8+1)²=289`-chunk block). Per-LOD pre-baked tiles (`subdiv [32,16,8,4]`) drawn **instanced, one draw per LOD**. Seams handled by **outward-facing per-chunk skirts** dropping `SKIRT_DEPTH=8.0` (≫ ~1.65 max wave amplitude). | `water.rs`, `water.wgsl` |
| 2 | **Real `set_water_level`** — `WaterUniforms` gained a `water_level` field (struct 144→160 B); `vs_main` places world Y from the uniform (no baked mesh Y). Editor knob made live end-to-end: `widget.rs → viewport/renderer.rs (was a dead stub) → EngineRenderAdapter::set_water_level (new) → Renderer::set_water_level (new, immediate uniform upload) → WaterRenderer`. | `water.rs`, `water.wgsl`, `renderer.rs`, `engine_adapter.rs`, `viewport/renderer.rs` |
| 3 | **`cull_mode` debug-artifact removal** — `cull_mode: None // DEBUG` → `Some(Face::Back)`. Top surface (CCW from above) and outward-wound skirts render; the submerged/underside two-sided case stays deferred (Gemini triage §E). | `water.rs` |

**Ratified forks honored:** Gerstner-first (no FFT introduced); chunk-grid-first (discrete tiles, not projected-grid/clipmap). Per `W2_DECISIONS.md` §B/§C.

## 2. Measured min-spec result (real GPU timestamps, 1660 Ti Max-Q, 1080p)

| Camera | Baseline (single plane) | **Chunked (after)** | Δ | vs 2.0 ms budget |
|---|---|---|---|---|
| near (gameplay cam) | 0.107 ms | **0.177 ms** | +0.063 ms (accepted) | **~11× under** |
| worst-case horizon | 0.122 ms | **0.160 ms** | +0.038 ms | **~12× under** |

- **Render-correctness check: 62.8% of pixels lit** (near view, black clear, f16 readback) — proves the surface rasterizes and Back-culling did not eat the top face.
- **Budget is provisional.** 2.0 ms is the ratified provisional ceiling pending real-scene confirmation. A representative full-frame headroom was **not headless-measurable** in this environment (`Renderer::new_headless` renders a near-empty default scene; the demo is a windowed winit app). Target framerate assumed: 60 FPS → 16.67 ms. Anchors: < the ~3 ms deferred F.4 accent budget; ~12% of frame; leaves room for W.2b refraction.
- Instrument: `astraweave-render/examples/water_budget_probe.rs` (committed, per ratification (c)) — wgpu `TIMESTAMP_QUERY` via the production `GpuProfiler`, 60 warmup + 300 measured frames, two cameras + a render-correctness readback.

## 3. Verification (build stays green — verified, not assumed)

| Target | Result |
|---|---|
| `cargo check -p astraweave-render` (lib + probe example) | ✅ |
| `cargo check -p hello_companion -p veilweaver_demo` | ✅ |
| `cargo check -p aw_editor` | ✅ (pre-existing `gizmo/mod.rs:32` warning only) |
| `cargo check --workspace --exclude llm_integration` | ✅ (pre-existing warnings only) |
| `cargo test -p astraweave-render --lib water::tests` | ✅ 7 passed (incl. on-GPU `new_and_update`: 4 LOD meshes, all 289 chunks assigned, `set_water_level` lands) |

Four production consumers compile and the surface rasterizes: **astraweave-render lib, hello_companion, veilweaver_demo, aw_editor.** `WaterRenderer::new`/`update`/`render`/`set_water_colors`/`set_rain_intensity` signatures unchanged.

## 4. Pre-existing, unrelated workspace error (NOT a W.2a regression)

`cargo check --workspace` fails only in `examples/llm_integration/src/main.rs` (`DEFAULT_QWEN_INSTRUCT_MODEL` not in scope) — the identical W.1 finding. Its fix is a standalone commit on `campaign/fluids-f3s`/`main`, deliberately kept off this branch per the W.1 gate. `llm_integration` has no water/render dependency. Expected and off-branch; not acted on here.

## 5. Logged follow-ups (recorded, NOT fixed — out of W.2a scope)

These are surfaced-but-out-of-scope. Fixing either is scope creep that costs min-spec build time for no W.2a benefit. Tracked here as the campaign ledger entry.

| ID | Item | Detail | Risk / disposition |
|---|---|---|---|
| **W-FU-1** | Pre-existing broken render-test file | `astraweave-render/tests/coverage_booster_render.rs` was already non-compiling before W.2a (`TaaConfig` private, `CpuMesh` missing fields, `TerrainLayerGpu` 8-vs-32, method-arg drift — all unrelated to water). W.2a added **only** the two `..Default::default()` its own `WaterUniforms` field change required (lines ~7609, ~13484); those two `WaterUniforms` literals were *also* already broken pre-W.2a (missing the rain-ripple fields). | **Not W.2a damage.** Pre-existing render-test debt — candidate for the same janitorial sweep as the W.1 orphaned `parallel`/`rayon` (see `W1_EXECUTION_REPORT.md` §3). Noted explicitly so a future "why won't this build" resolves cleanly. |
| **W-FU-2** | Dormant `update_water` editor caller | `set_water_level` uploads the uniform **immediately** (`Renderer::set_water_level`) precisely because the editor's `update_water` path is a known dormant caller (zero call sites — `engine_adapter.rs:3794` note; camera audit L.5.19). The knob works now via the immediate-upload workaround. | **Latent seam.** If W.2b/W.2c expect editor-side per-frame updates to flow through the normal `update_water` channel, this dormancy may bite. Named now so it does not compound silently. Not a W.2a defect. |

## 6. Trace + housekeeping landed in this commit

- `docs/architecture/render_pipeline_material_system_shader_infrastructure.md` → v1.1: §5 water row + §8 invariant #14 record the chunked-LOD surface + uniform-driven water level (was a single fixed 500-unit plane, baked Y=2.0). Scoped to what W.2a changed.
- `.gitignore` → `/cache/` (the GPU pipeline cache written at repo root by `PipelineCacheManager`; runtime artifact, not source).

## 7. Gate

Committed + pushed; stopped after build green and this report + the trace revision landed. **W.2b (refraction / scene-color sampling / depth-foam) not begun** — the next ratified gate, drafted against this committed baseline. Open for director: (a) the 2.0 ms ceiling stays provisional until a real-scene capture confirms it; (b) the +0.06 ms near-floor cost is accepted (no tuning); (c) the probe is the committed budget instrument for W.2b re-measurement.
