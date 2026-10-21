# W.2b.1 — Refraction / Scene-Color Plumbing Recon (Recon of Record)

**Campaign:** W-series (Water Successor) · **Phase:** W.2b, stage 1 (recon)
**Branch:** `campaign/water-successor` · **HEAD:** `1497e9387` (W.2a chunked-LOD surface)
**Mode:** READ-ONLY reconnaissance · **Mutations:** zero (no bindings, no shader/frame-graph edits, nothing built)
**Authority:** [`W2_0_RECON.md`](./W2_0_RECON.md) (flagged depth.rs / frame_graph.rs / ssr.rs as reuse candidates) · [`W2_DECISIONS.md`](./W2_DECISIONS.md)

This is the immutable recon persisted at the W.2b.1 gate. Every infra claim ships
with a `file:line` reference verified against current source on `1497e9387`. The
W.2.0 reuse suggestion was a *starting hypothesis*; this stage tested it. Findings
are transcribed from the gate report, not re-derived.

**Headline:** the W.2.0 reuse hypothesis (depth.rs / frame_graph.rs / ssr.rs)
**largely does not survive contact with current code.** Two of the three candidates
are dormant. The real plumbing problem is **structural** (water lives inside the
shared main pass), not a missing binding.

---

## Deliverable 1 — Scene-color + depth availability map

### Scene color — NOT available as a sampleable texture at the water pass

Water renders **inside the shared "main pass"** (`renderer.rs:4931` `begin_render_pass`
labelled `"main pass"`) into `self.hdr_view` with `LoadOp::Load` (`renderer.rs:4934-4942`);
the water draw is at `renderer.rs:5036-5038`, after opaque geometry + impostors. The
opaque scene **is** rendered before water (correct order) — but only as the
**attachment being written**, which a shader in that same pass cannot sample
(read-write hazard). There is **no scene-color resolve/snapshot** in the render path
(`copy_texture_to_texture` → zero matches in `renderer.rs`).

The HDR target itself is sampleable (`Rgba16Float`, `RENDER_ATTACHMENT | TEXTURE_BINDING`,
e.g. `renderer.rs:1290-1291`), and several spare full-res HDR textures already exist
(`hdr_tex` / `hdr_aux` / `fx_gi` / `fx_ao`, `renderer.rs:1280-1403`) used by the post
chain. So the **machinery to expose a sampleable scene color is cheap and present** —
it is simply never captured before the water draw today.

### Depth — sampleable in principle, blocked inside the current pass

`depth.rs:9,22`: `Depth32Float`, usage `RENDER_ATTACHMENT | TEXTURE_BINDING` → sampleable.
It holds the scene depth (opaque writes it; depth attachment at `renderer.rs:4943-4944`).
Format **matches** the foam delta's need — consumers already build `WaterRenderer` with
`Depth32Float`. But inside the current shared main pass it is the bound depth
*attachment*, so it cannot be sampled there. After that pass closes, it can.

### Frame graph — DORMANT, not the integration point

`frame_graph` / `FrameGraph` → **zero matches in `renderer.rs`**. The live render path
is entirely hand-coded `begin_render_pass` in `render()` and `draw_into()`. Matches the
render trace §2.3 ("parallel scaffolding") and §8 invariant #4. **There is no live
frame-graph reader pattern to reuse** — the W.2.0 hypothesis here is a red herring; the
water-pass restructure happens in hand-coded `renderer.rs`.

### SSR — DORMANT and wrong-shaped

`SsrPass` / `SsrConfig` appear only in `ssr.rs` (definition + tests) + `pub mod ssr`
(`lib.rs:99`) — **zero callers**, never instantiated/executed (consistent with the
"SSR disabled" note at `renderer.rs:5044-5051`). Structurally it is a **compute pass**
(`ssr.rs:167`) consuming a **G-buffer** (normals / metallic-roughness / velocity,
`ssr.rs:143-156`, all `ShaderStages::COMPUTE` `ssr.rs:348`) and writing a storage
texture — deferred-path machinery the forward water path does not have. **Its binding
setup is not a usable template** for a fragment-raster water refraction. The *concept*
(sample scene color + depth) is shared; the *machinery* is not.

---

## Deliverable 2 — Plumbing approach proposal: **net-new water-specific path** (reuse blocked)

**Recommendation: build a small net-new sampling path**, reusing only the real
primitives that exist. The two headline reuse candidates (ssr, frame_graph) are
dormant; forcing either costs more than it saves.

| Candidate | Verdict | Evidence |
|---|---|---|
| `depth.rs` `Depth32Float` texture | **REUSE (real)** — already sampleable, format matches | `depth.rs:9,22` |
| HDR-texture allocation pattern + `bind_group_cache` | **REUSE (real)** | `renderer.rs:1280-1403` |
| existing `WaterRenderer` bind group | **EXTEND** (1→4 entries) | water.rs BGL `:115-127` |
| `ssr.rs` `SsrPass` | **DO NOT REUSE** — dormant, compute/G-buffer-coupled, wrong path | `ssr.rs:167,143-156,348`; callers = 0 |
| `frame_graph.rs` | **DO NOT REUSE** — dormant, absent from live path | zero matches in `renderer.rs` |

### Net-new glue required

1. **Split the water draw out of the shared main pass** into its own raster pass that
   runs after the opaque main pass closes (`renderer.rs:5036` runtime path; mirror for
   the editor `draw_into` path, water at `renderer.rs:5668`).
2. **Snapshot opaque scene color:** after the main pass, `copy_texture_to_texture(hdr_view → water_scene_color)`
   — one dedicated full-res `Rgba16Float` texture + one copy/frame. Reuses the allocation
   pattern; decoupled from the post chain's ping-pong rather than fighting it.
3. **Water pass samples** `water_scene_color` + the scene `depth` texture. Bind depth as
   a **read-only depth attachment** for testing *and* sample it for the foam delta (standard
   soft-particle pattern; validate wgpu read-only-depth-plus-sample in W.2b.2, else sample
   a depth copy).

### Binding-layout delta (concrete)

Water bind group goes from **1 entry** (uniform @0; BGL `water.rs:115-127`, shader
`water.wgsl:34`) → **4 entries**: uniform @0 + `scene_color` texture @1 + `scene_depth`
texture @2 + sampler @3. This is a **BGL + pipeline-layout rebuild, not an additive
change**; the shader gains `@group(0) @binding(1..3)`.

**Ordering constraint imposed:** the water pass must run *after* the opaque main pass
completes *and* after the scene-color copy.

---

## Deliverable 3 — Budget-risk read (pre-build estimate — to be measured in W.2b.2)

Against the **2.0 ms provisional ceiling** and the W.2a measured **~0.16 ms** chunked
surface (1660 Ti Max-Q, 1080p):

| Added component | Estimate | Why |
|---|---|---|
| Scene-color copy (~16 MB full-res Rgba16Float) | ~0.1–0.2 ms | memory bandwidth |
| Pass split (extra full-res hdr load+store) | ~0.1–0.2 ms | bandwidth |
| Fragment refraction taps (scene-color + depth + edge-soften, ~2–4 taps × ~1.3–2M water frags) | ~0.1–0.3 ms | texture-bound |
| **Estimated water pass w/ refraction + depth-foam** | **~0.5–0.9 ms** | **comfortably under 2.0 ms** |

**This is a pre-build estimate, NOT a measured number** (measurement is W.2b.2 work,
via the committed `water_budget_probe.rs`). Refraction is *unlikely* to blow the 2.0 ms
ceiling. **The risk to watch is bandwidth, not shader math** — the copy + extra full-res
pass load/store dominate on the bandwidth-limited 1660 Ti Max-Q, not the texture taps. If
W.2b.2 measurement shows pressure, mitigations (measurement-driven, not now): **ping-pong
instead of copy** (opaque→X, water→Y — kills the copy but complicates post's input
selection), half-res refraction, or copy-only-when-water-on-screen. No soft-ceiling breach
is foreseen; the bandwidth cost is surfaced now rather than after the build, per the gate.

---

## Gate (as reported at W.2b.1)

Three deliverables produced; recon stopped. No build, no bindings, no decision taken.
Director ratification required before W.2b.2 builds refraction + depth-foam:

1. **Reuse-vs-new** — recommendation: **net-new** (ssr + frame_graph both dormant; only
   `depth.rs` + the alloc pattern are genuinely reusable). Confirm or push back.
2. **Binding-layout delta** — 1→4 entries, **BGL rebuild**, water split into its own pass
   after a scene-color copy; and whether the **editor `draw_into` path** is in W.2b.2 scope
   or deferred.
3. **Budget** — ~0.5–0.9 ms estimate sits under 2.0 ms; the **bandwidth flag** is the
   tolerance decision to make before the build.
