# W.0 — Water Successor: Pre-Deprecation Audit

**Campaign:** W-series (Water Successor) · **Phase:** W.0 · **Mode:** read-only reconnaissance
**Ratified by campaign director:** 2026-06-20 · **Mutations:** zero (this phase produced this report only)

Standing law: *built is not run; run is not seen; counted is not rendered;
measured-elsewhere is not measured-here.* Every claim ships with a `path:line`
reference; ambiguity goes to UNCERTAIN, not a guess. LoC herein were counted
firsthand (`wc -l`). The W.1 removal that this audit gated is recorded in
[`W1_EXECUTION_REPORT.md`](./W1_EXECUTION_REPORT.md).

---

## Orientation: the salvage architecture spans three crates

| Crate | Role | Touches `astraweave-fluids`? |
|---|---|---|
| `astraweave-water` | The `WaterQuery` facade (KEEP spine) | Only via a feature-gated `voxel` module (REMOVE collateral) |
| `astraweave-render` | The heightfield surface layer **already exists** here (`WaterRenderer`) | **No** — zero dependency |
| `astraweave-fluids` | The solver/voxel/effects/editor inventory being deprecated | Is the crate |

## Deliverable 1 — Crate inventory (dependency reality)

- **`astraweave-fluids` consumers (neither a game-loop crate):** `examples/fluids_demo` ([main.rs:18-21](../../../examples/fluids_demo/src/main.rs)) and `astraweave-water/src/voxel.rs:22` (behind feature `voxel`).
- **`astraweave-water` consumers:** only `astraweave-physics` ([Cargo.toml:32](../../../astraweave-physics/Cargo.toml)) — **without** `features = ["voxel"]`. So the voxel backend + the `water→fluids` edge were reachable only via `cargo test -p astraweave-water --features voxel`, never a shipped binary.
- **Seam confirmation:** physics reaches the facade through exactly two calls — `AnalyticWater::set_plane` and `WaterQuery::sample` ([physics/lib.rs:1429-1468](../../../astraweave-physics/src/lib.rs)). No external caller reaches past the facade into a solver internal. **Seam clean — no pre-deprecation fix required.**
- **`experimental`-gated modules** (`pcisph_system`, `multi_phase`, `turbulence`, `warm_start`, `particle_shifting`, `viscosity_gpu`) did not even compile in the default build.

Full per-module map (LoC, role, reachability, changed-since-F.0) is in the fluids
architecture trace `docs/architecture/fluids.md` (§5, as of pre-W.1 rev 1.5).

## Deliverable 2 — Salvage list (ratified)

**KEEP** — `WaterQuery`/`WaterSample`/`AnalyticWater` (`astraweave-water/src/lib.rs`, physics buoyancy's sole water source); `astraweave-render::WaterRenderer` (the surface-layer spine, see Deliverable 3).

**REMOVE** (zero production callers; the declined general-water ambition) — voxel sim (`volume_grid`/`gpu_volume`/`building`/`terrain_integration`) + facade `voxel` collateral; research/experimental SPH inventory (`research`/`pcisph_system`/`multi_phase`/`turbulence`/`warm_start`/`particle_shifting`/`viscosity_gpu`/`viscosity`/`boundary`/`validation`); `simd_ops` (SPH math, not rendering math).

**UNCERTAIN → resolved at the W.1 gate by director ratification:**
- ① F.4 accent substrate = **Reading A** (KEEP `FluidSystem`/`FluidRenderer`/optimization/sdf/lod/profiling/serialization + shaders); F.4 builds on it before any trim.
- ② Visual-effects layer = **DEFERRED to W.3+**.
- ③ `editor.rs` = **DEFERRED to the editor phase**.

## Deliverable 3 — Heightfield render-reuse recon

**Headline:** the W "surface layer (workhorse)" already exists in `astraweave-render`, fluids-independent and production-wired.

- `astraweave-render/src/water.rs` (`WaterRenderer`) + `shaders/water.wgsl` — 4-summed-Gerstner GPU-displaced heightfield, Fresnel, depth color blend, foam, rain ripples. Consumers: Veilweaver ([visual_renderer.rs:625-636](../../../examples/veilweaver_demo/src/visual_renderer.rs)), hello_companion ([visual_demo.rs:720-725](../../../examples/hello_companion/src/visual_demo.rs)), editor ([engine_adapter.rs:3739-3769](../../../tools/aw_editor/src/viewport/engine_adapter.rs)), core renderer ([renderer.rs:892,4536](../../../astraweave-render/src/renderer.rs)). **Zero `astraweave-fluids` dependency.**
- Reusable supporting machinery (verified): `mesh.rs:138` `compute_tangents` (MikkTSpace); `transparency.rs:33`/`:123` (sort + blend factory); `depth.rs`; `ssr.rs:76` `SsrPass`; `frame_graph.rs`.
- **Net-new required:** FFT spectral ocean (if chosen over Gerstner), camera-distance LOD/chunking, refraction/scene-color sampling, foam/caustics frame-graph wiring.
- **Recommendation (ratified direction):** the W surface layer should **extend `astraweave-render::WaterRenderer`** — not build net-new, and not lift anything from `astraweave-fluids` for the surface (this is also the §7.7 "no second implementation" path).

## Gate outcome

Salvage list ratified; render-reuse direction accepted; deprecation go. Proceeded
to W.1 (recorded separately). The W.1 removal preserved the entire removed corpus
(plus the deferred ②③ inventory) at tag `w0-pre-deprecation`.
