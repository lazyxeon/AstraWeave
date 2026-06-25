# F.4.3 — Live Accent Composite + Combined-Frame Measurement + Close-Out

**Campaign:** W-series (Water Successor) · **Phase:** F.4.3 (final water-story sub-phase) · **Mode:** mutation
**Branch:** `campaign/water-successor` · **Base HEAD:** `3357c0c7a` (F.4.2) · **Date:** 2026-06-24
**Status:** build green; combined frame measured; **the water story is complete end-to-end.** Uncommitted at the gate.

F.4.2 built the accent machinery but left it not-visually-live (the in-frame composite was deferred here because it modifies the monolithic `render()`). F.4.3 wires that composite, measures the combined frame on min-spec, and closes the arc.

---

## 1. The live in-frame composite (the careful touch)

The injection point was de-risked first by a recon-and-design workflow (4 parallel readers → 1 synthesis → 1 adversarial verifier; verdict **GO**, all 7 checks sound). The verifier confirmed the design lands accents **after the water pass, before tonemap, into the HDR target** (not the LDR surface), depth-tested read-only against the scene depth, with no crate-boundary leak and no zero-accent regression.

**Engine mechanism (`astraweave-render/src/renderer.rs`) — minimal touch, render()/draw_into() bodies UNCHANGED:**
- New field `hdr_overlay: Option<Box<dyn FnMut(&mut Encoder, &TextureView, &TextureView, &Device, &Queue)>>` — a **pure-`wgpu` callback**, so `astraweave-render` gains no `astraweave-fluids` dependency.
- New `set_hdr_overlay(...)` setter.
- New private `fire_hdr_overlay(...)` invoked inside `run_water_pass` on **all three exit paths** (no-water-renderer, no-visible-chunks, normal end) — accents are weave-driven and independent of water visibility. It resolves depth identically to the water pass (`depth_view.unwrap_or(&self.depth.view)`) and fires **after `water_renderer` is restored** (outside the take/restore window — a panic in the closure cannot drop the `WaterRenderer`).
- Because **both `render()` (5220) and `draw_into()` (5873) already call `run_water_pass`**, this single edit lights up the demo AND the editor pass path — zero duplication of the ~530-line `render()` body.

**Demo wiring (`examples/weaving_playground/src/main.rs`):**
- `resumed()` builds `fluid_renderer: Rc<FluidRenderer>` + `fluid_system: FluidSystem(2048)` against `renderer.hdr_format()` (Rgba16Float — matches `hdr_view`); recreated on `Resized`.
- Each frame: `accent_producer.tick(dt)` → `snapshot()` → `fluid_system.set_secondary_particles(queue, &accents)` → register a per-frame boxed closure capturing `Rc::clone(fluid_renderer)`, an Arc-cheap `wgpu::Buffer` clone, the live count, and the `CameraUniform` (from `RenderView`) → `renderer.render()`. The `Rc` + buffer-clone make the closure `'static`, avoiding the self-referential-borrow wall of a closure that borrows a sibling field.

Render order achieved: **opaque → water surface pass → accent composite (additive, HDR) → tonemap → present.**

## 2. Combined-frame measurement (the real budget validation)

The headline deliverable: surface + accents rendering **together in one frame** on the documented min-spec, via a headless probe (`examples/weaving_playground/examples/accent_budget_probe.rs`). It lives in `weaving_playground` (which legitimately depends on both crates) so the measurement adds **no render↔fluids Cargo edge** — invariant #18 stays literally true. Real GPU timestamps: water surface via `GpuProfiler` "water" span; accent composite via a manual `write_timestamp` pair around `render_accents`.

**Adapter:** NVIDIA GeForce GTX 1660 Ti with Max-Q Design · Vulkan · DiscreteGpu · driver 592.27 · 1920×1080 · 512 accents · median of 240 frames (60 warm-up).

| Camera | water surface | accent composite | **combined** | accent vs 0.5 ms | total vs 2.0 ms |
|---|---|---|---|---|---|
| near | 0.2445 ms | **0.0126 ms** | **0.2571 ms** | PASS (~40×) | PASS (~8×) |
| horizon | 0.1784 ms | **0.0061 ms** | **0.1846 ms** | PASS (~80×) | PASS (~11×) |

**Result:** the accent composite is **effectively free (~0.006–0.013 ms)** at representative weave-impact density — *better* than the arc's ~0.1–0.2 ms estimate (512 small additive billboards, minimal overdraw). The whole water system is **~0.26 ms worst-case, ~8× under the 2.0 ms budget**. The arc's arithmetic-on-isolated-numbers (~0.3–0.4 ms) is now a measured fact, and it was conservative. **No budget surprise; nothing to escalate.**

*Method honesty:* the ground geometry of the W.2a probe is omitted — it shaped refraction *appearance*, not water/accent *cost*; the GPU work of both passes is identical against a cleared scene-color/depth. Accent count fixed at 512 (mid-range of the demo's ~1–2k peak); the pass scales ~linearly in particle count and is so far under budget that the headroom is not in question.

## 3. Editor parity + close-out

**Editor parity — pass mechanism free, producer feed deferred (honest scope).** Because the overlay fires inside `run_water_pass`, the editor's `draw_into` path composites any registered overlay at the identical pre-tonemap point against its caller-supplied depth — **no `engine_adapter.rs`/`viewport` edit needed to enable the pass**. What the editor still needs for *visible* accents is its own producer feed (own a `FluidSystem`+`FluidRenderer`, drive a producer, upload, `set_hdr_overlay`). That requires **relocating `WaterAccentProducer` out of the `weaving_playground` binary into a shared crate**, which is out of scope for a minimal-touch render change — **logged as F.4.3-editor follow-on.** The engine is ready; only the editor-side data feed remains.

**Zero-accent identity preserved.** `render_accents` early-returns at count 0 (records no pass); a `None` overlay (every non-demo consumer's default) records zero GPU work and begins no pass. With no active weaves the producer uploads 0 particles → the frame is byte-identical to the F.4.2/W.2c.3 rendered state. `astraweave-render` is otherwise unchanged for all existing consumers.

**Crate boundaries clean.** `astraweave-render ⊥ astraweave-fluids` confirmed both directions (neither Cargo.toml references the other); only `wgpu` types cross the overlay seam; the composition is binary-orchestrated. The combined probe lives in `weaving_playground`, not `astraweave-render`.

**Consumers green:** hello_companion, veilweaver_demo, aw_editor (render consumers — the `run_water_pass` change is behavior-identical with no overlay), fluids_demo, weaving_playground. Tests: 677 fluids-lib + 8 fluids-GPU (incl. `gpu_render_accents_smoke`) + 12 producer.

## 4. The W-series water story — complete end-to-end

| Sub-phase | Delivers | State |
|---|---|---|
| **W.1** | Deprecate the SPH solver + voxel sim; re-scope water sim → layered rendering | ✅ |
| **W.2a** | Chunked-LOD Gerstner surface + real `set_water_level` | ✅ |
| **W.2b.2** | Screen-space refraction + depth-delta shoreline foam (post-opaque pass) | ✅ |
| **W-FU-2** | Editor's dormant water woken (live in the viewport) | ✅ |
| **W.2c.2** | Weave-response deformation (part/freeze/raise), position-agnostic, instanced | ✅ |
| **W.2c.3** | Weave gameplay producer + `FreezeWater` op — deformation from real fate-weaving | ✅ |
| **F.4.0–F.4.3** | GPU-particle accent layer: substrate audit → style/trigger recon → CPU producer + render split → **live composite + measured** | ✅ |

Water now **scales, refracts, foams, deforms from real weave triggers, and throws live splash/spray accents** — all on measured min-spec discipline (GTX 1660 Ti Max-Q), with the full surface+accent frame at **~0.26 ms, ~8× under the 2.0 ms budget**. The view-side water system the whole W-series re-scope aimed at is delivered.

## 5. Deferred (logged, post-arc, none begun)
- **F.4.3-editor** — relocate `WaterAccentProducer` to a shared crate + wire the editor's accent producer feed (the pass mechanism is already live in `draw_into`).
- **Crest + shoreline accent triggers** — the two ambient accent sites (crest is CPU-wave-analytic; shoreline wants the A1 GPU emission kernel).
- **Truth-coupling phase** — walkable ice / buoyancy for `FreezeWater` (presentation-only today).
- **W-FU-1** — pre-existing `coverage_booster_render.rs` test debt (E0432/E0063/E0061). **W-FU-3** — the `run_water_pass` take/restore panic gap (a single panic silently disables water; the F.4.3 overlay correctly fires *outside* that window but does not close the pre-existing gap).

---

*Execution record. Recon/design authority: the F.4.3 injection-point workflow (GO verdict). Construction is the engine `hdr_overlay` mechanism + the demo wiring + the combined probe.*
