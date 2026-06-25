# F.4.1 — Accent Style + Trigger Selection

**Campaign:** W-series (Water Successor) · **Phase:** F.4 (accent layer), stage 1 (style recon) · **Mode:** read-only recon + style proposal — propose-then-gate
**Branch at recon:** `campaign/water-successor` · **HEAD:** `3edf15e25` · **Date:** 2026-06-24
**Status:** recon complete; ratified at the F.4.1 gate; **built in F.4.2.**

Persisted record of the accent style + trigger decisions. Grounded by a direct read
of the secondary-particle machinery: the additive billboard pass blends
`SrcAlpha · One · Add` into the HDR target ([renderer.rs:410-412](../../../astraweave-fluids/src/renderer.rs#L410-L412))
— so glow is free (tint > 1.0 blooms) — the sprite is a procedural round soft dot with
**hardcoded** colour and an **unused `type` field carried** in `info.y`
([secondary.wgsl:22,68](../../../astraweave-fluids/shaders/secondary.wgsl)), and the
secondary buffer is `STORAGE | COPY_DST | VERTEX` ([lib.rs:641](../../../astraweave-fluids/src/lib.rs#L641))
so both a GPU kernel and a CPU upload work with zero buffer change.

---

## Deliverable 1 — The three cheap style levers

**Lever A — Billboard texture/shape** (procedural soft dot today, no texture binding):
- **A1 procedural** (round / teardrop / streak via UV math; zero binding, zero asset) — **ratified**
- A2 texture-atlas sprite (1 texture + sampler; authored sprite/flipbook) — deferred upgrade
- A3 hybrid (procedural + noise erosion)

**Lever B — Motion arc + lifetime** (the ballistic model is the kept `predict` pass,
`fluid.wgsl:139`): knobs = initial speed + spread, gravity scale, lifetime, fade curve, scale curve.
- B1 snappy/energetic · B2 floaty/magical · **B3 per-trigger differentiated — ratified, weave → B2**

**Lever C — Colour/blend + fate-weaving tie-in** (`info.y` carried but unused for colour;
additive HDR target → glow free):
- C1 uniform pale-white · C2 per-kind tint · **C3 per-kind tint + HDR glow — ratified**
  (part = silt/earthy, raise = clean white-blue, freeze = frost-cyan; weave colours pushed > 1.0).

## Deliverable 2 — Trigger → accent mapping

| Trigger | Signal | Character | Emission home | Priority |
|---|---|---|---|---|
| **Weave-impact** | W.2c.2 instance array (CPU-owned in `WaterWeaveProducer`) | floaty-magical, per-kind tint+glow; raise = lift-burst, part = outward+down silt, freeze = one-shot shimmer then suppress | **CPU producer** | **FIRST (F.4.2)** |
| Crest | `wave_height` foam ([water.wgsl:355](../../../astraweave-render/src/shaders/water.wgsl#L355)) | medium white-blue spray | CPU (wave-analytic) | deferred |
| Shoreline | depth-delta ([water.wgsl:360](../../../astraweave-render/src/shaders/water.wgsl#L360)) | snappy white spume | GPU (screen-space) | deferred |

Freeze inherits the surface's `(1-freeze)` foam suppression (`water.wgsl:288`): shimmer once, then quiet.

## Deliverable 3 — Emission-mechanism shape

Two Path-A-compatible shapes (both reuse `SecondaryParticle` + `secondary.wgsl` + the additive draw):
- **A2 — CPU producer + upload (ratified).** A binary-glue producer mirroring `WaterWeaveProducer`:
  reads weave triggers, spawns + ballistically ages `SecondaryParticle`s on the CPU, uploads via
  `queue.write_buffer` (buffer already `COPY_DST`). **Sidesteps** the per-step `secondary_counter`
  clear (`lib.rs:1233`) that is incompatible with multi-frame arcs. Natural fix for the
  hardcoded-65536 count bug ([lib.rs:1454](../../../astraweave-fluids/src/lib.rs#L1454)).
- A1 — GPU whitewater kernel (available; for shoreline/high-count later). Bindings already support
  it (`read_write` storage `lib.rs:567`, `atomic<u32>` counter `fluid.wgsl:57`).

**Crate-boundary seam (non-negotiable):** `astraweave-render` and `astraweave-fluids` are mutually
independent (verified — neither Cargo.toml references the other). Accents (fluids machinery)
composite over the surface (render's `WaterRenderer`) via the **binary glue**; F.4.2 must split a
"render accents only" path out of `FluidRenderer::render()` so the SSFR surface chain is not
double-invoked, and add NO render↔fluids dependency.

## Deliverable 4 — Budget sanity + F.4.2 scope

Estimated ~1–2k live particles peak ⇒ ~0.1 ms (from F.4.0's accent-path measurement); real check
is F.4.3's combined-frame measurement (≤ 0.5 ms target). **F.4.2 scope:** the CPU accent producer
(weave-impact), the counter-readback fix, the standalone additive-billboard render split, and the
style as tunable parameters.

---

## Gate — ratification outcomes (settled 2026-06-24, built in F.4.2)

| # | Decision | Ratified |
|---|---|---|
| 1 | Emission shape | **A2** — CPU accent producer in binary glue |
| 2 | Trigger | **weave-impact ONLY** (crest/shoreline deferred) |
| 3 | Shape (Lever A) | **A1 procedural** (round/teardrop/streak; texture deferred) |
| 4 | Motion (Lever B) | **B2 floaty/magical** for weave |
| 5 | Colour (Lever C) | **C3 per-kind tint + HDR glow** |
| 6 | Budget | **≤ 0.5 ms provisional / ~1–4k**; real check = F.4.3 combined-frame |

*Recon record. Construction authority is this gate table; the F.4.2 build (the `WaterAccentProducer`,
the `secondary_particle_count` fix, `FluidRenderer::render_accents`, and the `secondary.wgsl`
per-kind tint/shape) implements it.*
