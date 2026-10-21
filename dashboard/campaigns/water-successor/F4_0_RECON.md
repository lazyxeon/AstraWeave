# F.4.0 — GPU-Particle Accent Substrate Audit + Envelope Re-validation

**Campaign:** W-series (Water Successor) · **Phase:** F.4 (accent layer), stage 0 (recon) · **Mode:** read-only audit + measurement — propose-then-gate
**Branch at recon:** `campaign/water-successor` · **HEAD:** `3edf15e25` · **Date:** 2026-06-24
**Status:** recon complete; ratified at the F.4.0 gate (Path A, additive billboards, ≤0.5 ms / ~1–4k, adapt-build-the-emission-half). F.4.1 builds the style/trigger proposal on this; F.4.2 builds the emission half.

Persisted forensic record of the F.4 accent substrate. Verified by three parallel read-only
audit agents (FluidSystem / FluidRenderer+budget / fluids_demo reachability) plus a live
min-spec measurement (the crate's own `fluid_baselines` GPU-timestamp bench). Nothing built
or mutated; the measurement ran the kept bench on the 1660 Ti Max-Q.

---

## Headline

The W.1-kept substrate is a **working, fully-functional GPU PBF *simulation*** — but it is a
*simulation*, not an accent emitter, and at simulation scale it costs **2.5–3.5× the entire
2.0 ms water budget**. The F.2 "~3 ms / 15–20k" envelope survives **only at the iteration-cap
floor**, and even there overshoots the surface-inclusive ceiling. But the *accent* mechanism
the substrate was designed for (secondary splash/spray particles) exists at the
struct/shader/renderer/buffer level with **one missing piece — the emission kernel** — and the
cheap accent-shaped path (ballistic, low-count, no PBF/SDF/heat) is already present in the kept
passes. F.4 is therefore **adapt-and-build-the-emission-half**, not wire-a-finished-system.

## Deliverable 1 — Kept-substrate capability audit

W.1 (`1a57fdd41`, 2026-06-20) deleted the SPH-*physics* + voxel-sim inventory (58,796
deletions) and **explicitly KEPT** the F.4 accent substrate (Reading A, named in the commit
body; `fluids.md` §0.5).

- **`FluidSystem` (lib.rs)** — complete, tested GPU PBF solver. `step()`
  ([lib.rs:1159](../../../astraweave-fluids/src/lib.rs#L1159)) records a 7-kernel chain:
  SDF-gen → predict (gravity+buoyancy, [fluid.wgsl:128](../../../astraweave-fluids/shaders/fluid.wgsl#L128))
  → clear/build neighbor grid → PBD ×iterations (SPH density + PBF lambda + Δpos + collision +
  surface tension, `fluid.wgsl:182-327`) → integrate (SDF collision, boundary clamp, vorticity,
  XSPH, `fluid.wgsl:329`) → mix_dye (heat, `fluid.wgsl:453`). 7 `gpu_execution_tests.rs` prove
  end-to-end dispatch+submit+readback with physical invariants (containment, settling). NOT a
  hollow shell.
- **`FluidRenderer` (renderer.rs)** — SSFR (depth→smooth→shade→optional secondary billboards,
  [renderer.rs:591-737](../../../astraweave-fluids/src/renderer.rs#L591-L737)). Consumes the
  particle buffer with **zero adaptation** (80 B stride matches `array_stride: 80`; buffer carries
  `VERTEX` usage). Only non-test caller = `fluids_demo` ([laboratory.rs:173](../../../examples/fluids_demo/src/scenarios/laboratory.rs#L173)).
- **`optimization.rs`** — the iteration-cap envelope mechanism, intact and tested. `SimulationBudget`
  ([optimization.rs:307](../../../astraweave-fluids/src/optimization.rs#L307)) floors
  `recommended_iterations` at 2 (the cap). **Caveat:** throttles *iterations only*, not particle
  count (`quality_scale` defined but never uploaded, `optimization.rs:1369`).
- **`profiling.rs`** — `FluidProfiler` dormant (no production filler). The functional measurement
  path is `FluidSystem::enable_gpu_timing`/`read_gpu_timings` (real wgpu timestamps,
  [lib.rs:858-923](../../../astraweave-fluids/src/lib.rs#L858-L923)) — used for Deliverable 2.
- **Reachability** — `cargo run -p fluids_demo` constructs `FluidSystem::new(&device, 20000)`
  ([main.rs:355](../../../examples/fluids_demo/src/main.rs#L355)), steps every frame
  (`main.rs:665`), renders Lab via SSFR. Zero references to removed modules → builds.
- **The accent gap** — `SecondaryParticle` (48 B, [lib.rs:382](../../../astraweave-fluids/src/lib.rs#L382)),
  a 65,536-slot secondary buffer + counter (`lib.rs:637-652`), `secondary.wgsl` billboard shader,
  and the renderer draw (`renderer.rs:713`) **all exist**. But **no kernel writes
  `secondary_particles[]` or `atomicAdd`s the counter** — the "Dye & Whitewater" label
  (`lib.rs:1330`) dispatches only `mix_dye`. `secondary_particle_count()` returns the hardcoded
  capacity 65536 ([lib.rs:1454](../../../astraweave-fluids/src/lib.rs#L1454)) — a bug. **This
  missing emission kernel is the central F.4 gap.**

**Verdict (D1): (b) working-but-needs-adaptation** — the solver is essentially (a) a working
capped GPU-particle system, but it is a full PBF *simulation*; the accent half (secondary
emission) is the narrowly-gapped piece. Not (c).

## Deliverable 2 — F.2 envelope re-validation (MEASURED, 1660 Ti Max-Q, HighPerformance adapter)

Per-pass GPU timings, median of 60 frames, default quality `iterations=8`
(`benches/fluid_baselines.rs`):

| Particles | sdf | pbd_iterations | integrate | mix_dye | **total GPU** | submit+wait wall |
|---|---|---|---|---|---|---|
| 10,000 | 0.886 | **3.323** | 0.441 | 0.215 | **4.895 ms** | 5.71 ms |
| 20,000 | 0.917 | **4.988** | 0.806 | 0.356 | **7.109 ms** | 8.28 ms |
| 50,000 | 0.972 | 14.461 | 2.429 | 1.077 | 19.009 ms | 20.71 ms |

- **The F.2 "~3 ms / 15–20k" does NOT hold at default quality** — 20k = **7.1 ms GPU / 8.3 ms wall**,
  ~2.5× the claim. It re-validates **only at the iteration-cap floor (2 iter)**: `pbd_iterations`
  dominates and scales linearly, so 20k @ 2 iter ≈ **~3.3 ms**. The "iteration-capped" qualifier
  is load-bearing.
- **Even the capped ~3.3 ms exceeds the entire 2.0 ms water budget** — and it is a full simulation.
- **What accents don't need:** SDF ~0.9 ms (flat fixed cost, dynamic-object collision),
  mix_dye ~0.2–0.36 ms (heat), and **pbd_iterations (the 3–5 ms dominant cost) is incompressibility**
  — splash/spray is ballistic.
- **Accent-shaped cost (same measurement):** `predict + integrate + grid` only ≈ **~0.5 ms at 10k**;
  at ~2–4k accent particles ≈ **~0.1–0.2 ms**.
- **Method honesty:** measured `step()` compute in isolation (clean apples-to-apples with F.2) via
  real GPU timestamps; did NOT measure inside a live frame sharing budget with the surface (a build
  step). Surface cost separately measured (`W2B2_EXECUTION_REPORT.md`: ~0.18–0.20 ms typical,
  ~0.35–0.4 ms worst). Coexistence is arithmetic on two independent measurements.
- **Proposed F.4 budget target:** surface claims ~0.2–0.4 ms of the 2.0 ms ceiling → ~1.6 ms
  headroom. Accents target **≤ 0.5 ms** at ~1–4k particles, **stripped path** (no PBF/SDF/heat),
  **not the full solver**.

## Deliverable 3 — Surface-integration recon

- **Triggers already in the surface shader:** depth-delta shoreline foam
  ([water.wgsl:360](../../../astraweave-render/src/shaders/water.wgsl#L360), water-meets-geometry),
  crest foam (`water.wgsl:355`, wave crests), weave-deformation sites (W.2c.2 instance array;
  freeze suppresses — surface scales foam by `(1-freeze)`, `water.wgsl:288`).
- **Composition:** water is a split post-opaque pass
  ([renderer.rs:4635](../../../astraweave-render/src/renderer.rs#L4635)); accents render after the
  surface pass, additively, into the same HDR target.
- **SSFR vs additive:** SSFR is built for a coherent fluid *volume* (expensive screen-res passes);
  sparse accents want **additive billboards** — the `secondary.wgsl` path already exists. Recommend
  additive billboards.

## Deliverable 4 — Build-vs-integrate scope + sub-phasing

**Scope verdict: adapt + build the emission half.** Substrate intact and integration-ready, but
using it as-is (full PBF sim) neither fits budget nor *is* an accent.

- **Path A (recommended, ratified):** drive accents off the existing secondary-particle machinery —
  write the missing emission mechanism + fix `secondary_particle_count()`; render via the existing
  additive billboard draw. Reuses ~90% of dormant-but-wired code.
- **Path B:** a fresh lightweight ballistic emitter.

**Sub-phasing:** F.4.1 (style + trigger recon → ratify) · F.4.2 (build the emission mechanism +
counter-readback fix + spawn-from-triggers + additive render) · F.4.3 (combined-frame min-spec
measurement vs ≤0.5 ms + wire + close-out).

## Gate — ratification outcomes (settled 2026-06-24)

| # | Decision | Ratified |
|---|---|---|
| 1 | Substrate verdict | adapt + build the emission half (substrate intact, accent half gapped) |
| 2 | Emission approach | **Path A** — reuse the secondary-particle machinery |
| 3 | Render approach | **additive billboards**, not SSFR |
| 4 | Budget target | **≤ 0.5 ms provisional / ~1–4k particles**; real validation in F.4.3 combined-frame |
| 5 | Aesthetic direction | **stylized, art-directed not simulated** — character from texture/motion/color, not count/cost; expensive realism (volumetric, high-count, per-particle-lit) out |

*Recon record. The F.4.1 style/trigger proposal and the F.4.2 build implement these.*
