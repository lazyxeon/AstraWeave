# W.2b.2 — Refraction + Scene-Color Sampling + Depth-Foam — Execution Report

**Campaign:** W-series (Water Successor) · **Phase:** W.2b, stage 2 (build) · **Mode:** render-code mutation (pass-split + new bindings + refraction + depth-foam)
**Branch:** `campaign/water-successor` · **Scope authority:** [`W2B1_RECON.md`](./W2B1_RECON.md) (ratified) + [`W2_DECISIONS.md`](./W2_DECISIONS.md)
**Min-spec measurement box:** NVIDIA GeForce GTX 1660 Ti with Max-Q Design · Vulkan · DiscreteGpu · driver 592.27 (real GPU timestamps)
**Date:** 2026-06-22 · **Status:** complete, build green, committed, stopped at the W.2b.2 gate

Forensic companion to [`W2B1_RECON.md`](./W2B1_RECON.md). Records the build of the
ratified net-new refraction path.

---

## 1. Step 0 — depth-copy fork: ONE copy (resolved empirically)

A validation probe with a **negative control** settled the fork against wgpu 25 /
Vulkan / 1660 Ti: writable-depth + same-texture-sampling **correctly errored**
(harness detects the hazard); **read-only depth attachment (`depth_ops: None`) +
sampling that same depth texture in the pass is ACCEPTED**. So depth is read-only-
attached *and* sampled directly — **only the scene-color copy is needed, no second
depth copy**. The probe is kept as a committed regression guard
(`examples/depth_sample_capability_probe.rs`) because the capability is
wgpu-version-dependent; re-run after any wgpu upgrade.

## 2. The build

| Piece | Where |
|---|---|
| **Pass-split** — water removed from the shared main pass into `Renderer::run_water_pass`, run after the opaque pass closes, in BOTH `render()` (runtime) and `draw_into()` (editor) | `renderer.rs` |
| **Scene-color snapshot** — `copy_texture_to_texture(hdr_tex → water_scene_color)` after the opaque pass; `hdr_tex` made the authoritative target via a §7.7 consistency fix (construction now backs `hdr_view` with `hdr_tex`, matching resize) | `renderer.rs` |
| **1→4 bind group** — uniform @0 + scene_color @1 + scene_depth @2 + sampler @3; `prepare_scene` wires the snapshot/depth + screen size each frame, gen-gated rebuild | `water.rs` |
| **Refraction** — screen-space scene-color tap distorted by the surface normal; absorption blend toward the body colour by reconstructed water thickness | `water.wgsl` `fs_main` |
| **Depth-delta foam (Profile C)** — world pos of the opaque scene reconstructed from `inv_view_proj` + sampled depth; scrolling foam where thickness → 0 | `water.wgsl` |
| **Profile A Q≤1.0 steepness cap** — Gerstner guardrail in both `gerstner_wave`/`gerstner_normal` | `water.wgsl` |

**Ratified decisions honored:** net-new sampling path (ssr.rs / frame_graph.rs are
dormant — not reused, per `W2B1_RECON.md`); editor `draw_into` in scope; Gerstner-first;
read-only-depth single-copy.

## 3. Measured min-spec result (real GPU timestamps, 1660 Ti Max-Q, 1080p)

| Line item | near | horizon |
|---|---|---|
| Scene-color copy (full-res Rgba16Float) | 0.089 ms | 0.083 ms |
| Water pass (raster + refraction + foam + load/store) | 0.090 ms | 0.119 ms |
| **Total water cost** | **~0.18 ms** | **~0.20 ms** |

**~10× under the 2.0 ms provisional ceiling** (worst-case full-screen ~0.35–0.4 ms,
still ~5×). As the recon predicted, **the copy (~0.085 ms) is the dominant added cost
— bandwidth, not shader math.** No budget pressure → no mitigation spent (ping-pong /
half-res / copy-gating unused). Refraction confirmed LIVE (12.8% warm, mid-band 25% —
ground checker visibly refracts); foam RENDERS (7.9% near-white). Instrument:
`examples/water_budget_probe.rs` (extended with the refraction/foam measurement).

## 4. Adversarial review (3 independent agents) — outcome

- **CRITICAL "depth Y-flip" — REJECTED as a hallucination.** The reviewer cited
  `aerial_perspective.wgsl` / `dfao.wgsl` as canonical — **neither file exists**. The
  real convention (`nanite_material_resolve.wgsl:166` builds a top-origin uv) confirms
  `ndc.y = 1 - 2·uv.y` is correct; the non-mirrored refraction proves it empirically.
- **HIGH — editor-path depth bug — FIXED.** `run_water_pass` hardcoded `self.depth.view`,
  but `draw_into()` routes opaque depth to a caller-supplied external texture. Threaded
  `depth_view: Option<&TextureView>` through, used for both the read-only attachment and
  the sampled binding (`render()` → `None`, `draw_into()` → its depth).
- **MEDIUM — wasted copy when water has no chunks — FIXED** (`has_visible_chunks` early-return).
- **LOW — non-default build break (`hdr_view` E0425 under `--no-default-features`) — FIXED**
  in the edited region; non-default build now compiles.
- Reviewer-verified sound (no change): read-write hazard avoided, copy source authoritative,
  resize correct, generation gate correct, dummy textures unreachable, 240-byte uniform layout matches.

## 5. Verification (build stays green — verified, not assumed)

| Target | Result |
|---|---|
| `cargo check -p astraweave-render` (default) | ✅ |
| `cargo check -p astraweave-render --no-default-features --features textures` | ✅ (E0425 fix) |
| `cargo test -p astraweave-render --lib water` (+ integrated `render()` water test) | ✅ 16 + 1 |
| `cargo check --workspace --exclude llm_integration` | ✅ (pre-existing warnings only) |

Consumers: **renderer / veilweaver_demo / hello_companion** render refracted water via the
validated `render()` path (probe + integrated test). **aw_editor**: correctly plumbed
(depth bug fixed) but water is **dormant** — see W-FU-2.

## 6. Logged follow-ups (carried; NOT this commit's work)

| ID | Item | Disposition |
|---|---|---|
| **W-FU-2** | Dormant editor `update_water` (zero callers) → editor water never gets view/time/chunks; `has_visible_chunks` cleanly skips the pass. **Now the gating item:** editor refraction is plumbed-but-not-rendering until `update_water` is wired. The **next standalone step** before W.2c, NOT part of this commit. | Tracked, next step |
| **W-FU-1** | Pre-existing broken `coverage_booster_render.rs` (TaaConfig/CpuMesh/TerrainLayerGpu drift, unrelated to water). | Tracked, janitorial sweep |
| **W-FU-3** | `run_water_pass` `take()`/restore is not panic-safe — a panic between take and restore would drop the water renderer for the process lifetime. Normal path never panics (wgpu errors route through the error scope, not panics). Minor robustness only. | Tracked, robustness |

## 7. Housekeeping landed in this commit

- `docs/architecture/render_pipeline_material_system_shader_infrastructure.md` → v1.2: §5 water row + §8 invariant #15 (the post-opaque pass-ordering + read-only-depth + same-texture rule).
- `W2B1_RECON.md` persisted (the ratified recon that found ssr.rs / frame_graph.rs dormant — keeps a future session from re-suggesting that reuse).
- `examples/depth_sample_fork_probe.rs` → renamed `depth_sample_capability_probe.rs` and re-headered as a wgpu-upgrade regression guard.
- `.gitignore`: added `**/cache/pipeline_cache.bin`; untracked the crate-local `astraweave-render/cache/pipeline_cache.bin` (kept on disk) so build-artifact churn stops polluting diffs.

## 8. Gate

Committed + pushed; stopped after build green and this report + the trace revision + the
recon persist landed. **W.2c (weave-response) not begun; W-FU-2 (editor `update_water`
wiring) not begun** — the next standalone step, drafted against this committed baseline.
