---
schema_version: 1
trace_id: water
title: "Water (Successor) — Rendering, Query Facade & Weave-Response"
description: "Water Successor — `WaterQuery` facade + render water surface + weave-response (part/freeze/raise/FreezeWater) + F.4 accents"
primary_crate: astraweave-water
domain: physics-world
lifecycle_status: active
integration_status: mixed
owns: [astraweave-water]
doc_version: "1.1"
last_verified_commit: 7c29b8182
---

# Architecture Trace: Water (Successor) — Rendering, Query Facade & Weave-Response

## Metadata

| Field | Value |
|---|---|
| **System name** | Water (Successor) — layered rendering + gameplay-truth query facade + weave-response deformation |
| **Primary crates** | `astraweave-water` (truth facade), `astraweave-render` (surface + weave presentation), `astraweave-physics` (buoyancy consumer), `astraweave-gameplay` (`WeaveOp` source + `water_movement`), `astraweave-fluids` (particle accent substrate), `examples/weaving_playground` (weave/accent producers — example layer) |
| **Document version** | 1.1 |
| **Last verified against commit** | `7c29b8182` |
| **Last verified date** | 2026-06-25 |
| **Status** | Active (post-W.1 layered-rendering re-scope; surface + refraction + weave deformation + accents all wired; some layers example-only — see §5/§6) |
| **Owner notes** | First trace for the W-series (Water Successor) campaign. Derived from forensic reading of the code at `7c29b8182` plus the campaign docs under `docs/campaigns/water-successor/`. Cross-references the (DEPRECATED-solver) [`fluids.md`](./fluids.md) and the [render trace](./render_pipeline_material_system_shader_infrastructure.md). |

---

## 1. Executive Summary

**What this system does:**
Water in AstraWeave is a **layered rendering system with a thin bounded-deformation layer and a single deterministic gameplay-truth facade** — NOT a general fluid simulation. A heightfield Gerstner surface (chunked-LOD, GPU-displaced) is the ~90%-of-scenes workhorse; a small authored vocabulary (part / freeze / raise) deforms it in response to fate-weaving; GPU particle accents (splash/spray) garnish weave impacts; and all *gameplay* questions about water ("is there water here, how high, how dense") flow through the [`WaterQuery`](../../astraweave-water/src/lib.rs) trait in `astraweave-water`.

**Why it exists:**
The W-series campaign (W.0 ratification, 2026-06) re-scoped water from a general SPH/voxel *simulation* — which F.3.S measured could not carry general water on min-spec — to a cheap, authored, art-directable *rendering* system. The truth facade exists so physics (and later AI/gameplay) ask one logical owner instead of each carrying a private notion of "the water" (the §7.7 "no second implementation" mandate).

**Where it primarily lives:**
- `astraweave-water/src/lib.rs` — the `WaterQuery` trait + `AnalyticWater` backend (gameplay truth; CPU; deterministic).
- `astraweave-render/src/water.rs` (~990 LoC) + `astraweave-render/src/shaders/water.wgsl` — the surface layer, refraction, depth-foam, and weave-response *presentation* (`WaterRenderer`).
- `astraweave-render/src/renderer.rs` — the split water pass (`run_water_pass`) and the public `Renderer` water/weave/accent API.
- `astraweave-physics/src/lib.rs:1429-1489` — `apply_buoyancy_forces`, the one wired non-test `WaterQuery::sample` consumer.
- `astraweave-gameplay/src/{types.rs,weaving.rs,water_movement.rs}` — `WeaveOpKind` (the deform triggers) and the standalone swim/oxygen/wet-status state machine.
- `examples/weaving_playground/src/{weave_producer.rs,weave_accent_producer.rs,main.rs}` — the gameplay-`WeaveOp` → render-`WeaveInstance` translation and the accent emitter. **These live at the example/binary layer, not in a library crate** (see §5, §6).

**Status note:**
The truth facade and the render surface (Gerstner + refraction + depth-foam + weave deformation) are wired into the runtime. The weave-response **producers** (the gameplay-op→render-instance and gameplay-op→accent translations) currently live ONLY in the `weaving_playground` binary; they are not yet a shared library system. `FreezeWater` weave truth (walkable ice / buoyancy blocking) is deliberately **presentation-only** today — there is no truth-coupling for it (`astraweave-gameplay/src/weaving.rs:94-105`). The voxel `WaterQuery` backend was removed in W.1; only `AnalyticWater` remains.

---

## 2. Authoritative Pipeline

The system has two largely-independent data flows that meet only at the `WeaveOp` source. Both are shown.

### 2A. Gameplay-truth flow (CPU, deterministic)

```text
[Authoring / back-compat scalar plane / add_water_aabb]
    │
    │ AnalyticWater::set_plane(level, density) / ::add_aabb(min, max, density, drag)
    ▼
[AnalyticWater — single owner of analytic water truth]
    file: astraweave-water/src/lib.rs
    role: holds an optional infinite plane + N bounded AABB volumes
    key data: Plane{surface_height, density}, Aabb{min,max,density,linear_drag}
    │
    │ WaterQuery::sample(point) -> Option<WaterSample{surface_height, density}>
    ▼
[Physics buoyancy — the wired consumer]
    file: astraweave-physics/src/lib.rs:1429 (apply_buoyancy_forces)
    role: write-through-sync scalar plane into facade, sample per buoyant body,
          apply Archimedes impulse (volume * density * 9.81) * dt when below surface
    key data: per-body buoyancy impulse
    │
    ▼
[Rapier rigid body velocity change]
```

### 2B. Presentation flow (GPU; non-deterministic; excluded from world_hash/replay/net)

```text
[Fate-weaving applies a WeaveOp]                  [Per-frame camera + time]
    file: astraweave-gameplay/src/weaving.rs               │
    key data: WeaveOp{kind, a: Vec3, b, budget_cost}       │
    │                                                       │
    │ producer.ingest(&op)  (example layer)                 │
    ▼                                                       │
[WaterWeaveProducer / WaterAccentProducer]                 │
    file: examples/weaving_playground/src/weave_producer.rs │
          examples/weaving_playground/src/weave_accent_producer.rs
    role: map WeaveOpKind → WeaveKind/AccentKind, age through a
          synthetic lifetime envelope, snapshot the live set
    key data: Vec<WeaveInstance>, Vec<SecondaryParticle>    │
    │ snapshot()                                            │
    ▼                                                       ▼
[Renderer::set_water_weave_instances]          [Renderer::update_water(vp, cam, t)]
    file: astraweave-render/src/renderer.rs:4658     renderer.rs:4632
    │ delegates                                            │ delegates
    ▼                                                       ▼
[WaterRenderer — surface + weave presentation owner]
    file: astraweave-render/src/water.rs
    role: chunked-LOD Gerstner mesh; WaterUniforms (incl. weave_instances[8]);
          per-LOD instanced chunk assignment around the camera
    key data: WaterUniforms (512 B), per-LOD instance buffers, LOD tile meshes
    │ Renderer::run_water_pass (after opaque pass closes)
    ▼
[Split water pass]
    file: astraweave-render/src/renderer.rs:4679 (run_water_pass)
    role: snapshot opaque HDR → water_scene_color; prepare_scene wires
          scene-color + read-only depth + screen size; draw water into HDR;
          then fire optional HDR accent overlay (F.4.3)
    │ water.wgsl vs_main (Gerstner + weave deform) → fs_main (refraction/foam/freeze)
    ▼
[Refracted, foamed, weave-deformed water surface in the HDR target]
    │ (F.4.3) fire_hdr_overlay closure → FluidRenderer::render_accents
    ▼
[+ additive GPU-particle accents] → tonemap → present
```

### Stage-by-stage detail

#### Stage (truth): `AnalyticWater` sampling
**File:** `astraweave-water/src/lib.rs:186-215`
**Role:** The single sampling implementation for gameplay water truth.
**Inputs:** A world `Vec3` point.
**Outputs:** `Option<WaterSample{surface_height, density}>`.
**Notes:** Resolution is **topmost-surface-wins, ties broken by registration order** (`lib.rs:188-203`). The plane is considered first, then AABBs in push order; strict `>` keeps the first-registered on a tie. `Some` does NOT mean the point is submerged — the caller compares `point.y` to `surface_height` itself (`lib.rs:67-71`, and `astraweave-physics/src/lib.rs:1468`). `Aabb::linear_drag` is stored honestly but is authoring-only data; `sample` does not read it (`lib.rs:90-95`).

#### Stage (truth): Physics buoyancy consumption
**File:** `astraweave-physics/src/lib.rs:1429-1482`
**Role:** The one wired production `WaterQuery` consumer.
**Inputs:** Registered buoyant bodies (`add_buoyancy(body, volume, drag)`), the facade.
**Outputs:** A per-body Archimedes impulse.
**Notes:** Each tick it write-through-syncs the retired scalar `water_level`/`fluid_density` into the facade via `set_plane` (`lib.rs:1435`) so the scalar plane participates in the single sampling path, then samples per body. Buoyancy applies only when `pos.y < surface_height` (`lib.rs:1468`). Force is applied as a one-shot impulse `force * dt` (NOT `add_force`) because Rapier user forces persist until `reset_forces`, which has zero workspace call sites (`lib.rs:1437-1442`). Drag is per-body; the water is not queried for it.

#### Stage (presentation): Chunked-LOD Gerstner surface
**File:** `astraweave-render/src/water.rs` + `shaders/water.wgsl`
**Role:** The heightfield surface workhorse.
**Inputs:** `update(queue, view_proj, camera_pos, time)`.
**Outputs:** Per-LOD instance buffers (chunk world-XZ offsets) + uploaded `WaterUniforms`.
**Notes:** The surface is a discrete grid of `CHUNK_SIZE = 64` chunks; a `(2*GRID_RADIUS+1)² = 289`-chunk block around the camera is active each frame (`water.rs:31-35`). Each chunk picks an LOD by camera distance (`LOD_DISTANCES`/`LOD_SUBDIVS` at `water.rs:37-40`); chunks of the same LOD draw in **one instanced draw call** (`render()` at `water.rs:823-836`). Wave displacement is a pure function of world XZ, so chunk meshes are world-stable and shared LOD-boundary vertices agree exactly; the only mismatch is curve-vs-chord on a coarse neighbour's edge (≤ ~1.65 unit total wave amplitude), covered by per-chunk skirts that drop `SKIRT_DEPTH = 8.0` (`water.rs:41-44`, `601-654`). The vertex shader sums 4 Gerstner waves with a Profile-A `Q ≤ 1.0` steepness cap to prevent normal inversion (`water.wgsl:135-181, 246-259`). World Y comes from the `water_level` uniform (no baked mesh Y) — this is the real `set_water_level` (`water.rs:750-757`, `water.wgsl:282`).

#### Stage (presentation): Split water pass — refraction + depth-foam
**Files:** `astraweave-render/src/renderer.rs:4679-4768`, `shaders/water.wgsl:295-382`
**Role:** Compose water against the opaque scene.
**Inputs:** The closed opaque HDR pass, scene depth.
**Outputs:** Refracted, foamed water drawn into the HDR target.
**Notes:** After the opaque main pass closes, `run_water_pass` `copy_texture_to_texture`s the HDR into `water_scene_color` (`renderer.rs:4711-4726`), `prepare_scene` wires the snapshot + read-only depth + screen size into the 4-entry water bind group (`water.rs:721-746`), and water draws with `depth_write_enabled: false` against read-only depth. The fragment shader taps `scene_color` distorted by the surface-normal XZ tilt for refraction, reconstructs scene world position from `inv_view_proj` + sampled depth to compute water thickness (Beer-Lambert-ish absorption), and adds depth-delta shoreline foam where thickness → 0 (`water.wgsl:319-363`). The pass is skipped entirely when `has_visible_chunks()` is false (`renderer.rs:4695`, the editor dormant case). Both `render()` (runtime) and `draw_into()` (editor) call `run_water_pass`; the editor routes a caller-supplied depth (`renderer.rs:5279, 5932`).

#### Stage (presentation): Weave-response deformation
**Files:** `astraweave-render/src/water.rs:49-136, 784-811`, `shaders/water.wgsl:183-293, 368-376`
**Role:** Bounded part/freeze/raise deformation of the surface.
**Inputs:** `set_weave_instances(&[WeaveInstance])` (≤ `MAX_WEAVE_INSTANCES = 8`).
**Outputs:** Weave instances packed into `WaterUniforms.weave_instances` and consumed in the shader.
**Notes:** Each `WeaveInstance` is a normalized analytical profile placed at a world XZ location with radius/orientation/intensity/phase; **location lives only on the instance** — the profile is a position-agnostic shape in local space (`water.rs:78-104`, `water.wgsl:188-227`). The shader maps `world → local = rotate(world_xz − position, −orientation) / radius`, evaluates the profile, and applies: `Part` pushes the surface down, `Raise` lifts it up (both additive height, bounded to `±WEAVE_MAX_DEFORM = SKIRT_DEPTH = 8.0` so a deform can never outrun the skirt), `Freeze` accumulates a mask that damps wave displacement toward rest and flattens the normal, then the fragment shader tints frozen regions toward ice and adds sharp specular (`water.wgsl:214-227, 271-272, 368-376`). The weave is composed AFTER the Gerstner sum so the per-wave Q-cap is untouched, and freeze height is captured BEFORE the weave offset so a raise doesn't read as foam (`water.wgsl:261-289`).

#### Stage (presentation): GPU-particle accents
**Files:** `examples/weaving_playground/src/weave_accent_producer.rs`, `astraweave-render/src/renderer.rs:4614-4625, 4764-4768`, `astraweave-fluids` (`FluidSystem`/`FluidRenderer`/`secondary.wgsl`)
**Role:** Splash/spray garnish on weave impacts.
**Inputs:** Applied `WeaveOp`s → `WaterAccentProducer` emitters → `SecondaryParticle`s.
**Outputs:** Additive billboards composited into the HDR target after the water pass.
**Notes:** The accent producer (example layer) owns particle lifetime on the CPU, ages them ballistically, and snapshots `SecondaryParticle`s for `FluidSystem::set_secondary_particles`. The renderer exposes `set_hdr_overlay(closure)`; the binary registers a per-frame closure that calls `FluidRenderer::render_accents`. The closure receives ONLY `wgpu` types, so `astraweave-render` gains **no** `astraweave-fluids` dependency (`renderer.rs:735-744, 4614-4625`). The overlay fires inside `run_water_pass` on all exit paths, after `water_renderer` is restored, into HDR before tonemap (`F4_3_EXECUTION_REPORT.md` §1). Zero accents → `render_accents` early-returns → byte-identical frame.

---

## 3. Semantic Vocabulary

| Term | Definition | Used in |
|---|---|---|
| **Water truth** | The deterministic, CPU-resident answer to "is there water here, how high, how dense" | `astraweave-water/src/lib.rs` |
| **Water presentation** | The view-side rendered surface (Gerstner, refraction, foam, weave deformation, accents); non-deterministic, excluded from world_hash/replay/net | `astraweave-render/src/water.rs`, `water.wgsl` |
| **`WaterQuery`** | The read-side trait physics depends on — one method, `sample(point) -> Option<WaterSample>` | `astraweave-water/src/lib.rs:60-72` |
| **`AnalyticWater`** | The sole `WaterQuery` backend: optional infinite plane + N bounded AABB volumes | `astraweave-water/src/lib.rs:127-215` |
| **`WaterSample`** | What a consumer learns at a point: `surface_height` + `density` (exactly the two fields physics reads) | `astraweave-water/src/lib.rs:47-53` |
| **Surface layer** | The chunked-LOD Gerstner heightfield — the ~90%-of-scenes workhorse | `astraweave-render/src/water.rs` |
| **Chunk** | A `CHUNK_SIZE=64`-unit square tile of the surface; LOD chosen by camera distance | `astraweave-render/src/water.rs:31-40` |
| **Skirt** | Per-chunk perimeter wall (drops `SKIRT_DEPTH`) hiding LOD-boundary cracks | `astraweave-render/src/water.rs:601-654` |
| **Weave / `WeaveInstance` / `WeaveKind`** | A bounded authored deformation (Part/Raise/Freeze) placed at a world location; presentation-side | `astraweave-render/src/water.rs:64-104` |
| **`WeaveOp` / `WeaveOpKind`** | The gameplay-side fate-weaving op that triggers a weave (`LowerWater`/`RaisePlatform`/`FreezeWater` map to water weaves) | `astraweave-gameplay/src/types.rs:42-58` |
| **Accent** | A GPU-particle splash/spray garnishing a weave impact (`SecondaryParticle`) | `examples/weaving_playground/src/weave_accent_producer.rs`, `astraweave-fluids` |
| **Producer** | The example-layer object that translates gameplay `WeaveOp`s into render instances/accents and ages them | `examples/weaving_playground/src/weave_producer.rs`, `weave_accent_producer.rs` |
| **Submersion** | A `[0,1]` float consumed by the gameplay swim/oxygen state machine; computed by the caller, not by `WaterQuery` | `astraweave-gameplay/src/water_movement.rs` |

### Terms to NOT confuse

- **Water truth vs water presentation:** Truth lives in `astraweave-water` (`WaterQuery`/`AnalyticWater`), is CPU + deterministic, and physics buoyancy reads it. Presentation lives in `astraweave-render` (`WaterRenderer`), is GPU + non-deterministic, and is excluded from `WorldSnapshot`/`world_hash`/replay/net. A weave's *deformation* is presentation; a weave's *truth* (if a frozen patch ever becomes walkable) would be a separate facade concern (`W2_DECISIONS.md` §B.1).
- **`WeaveInstance` (render) vs `WeaveOp` (gameplay):** `WeaveOp`/`WeaveOpKind` are gameplay types in `astraweave-gameplay`. `WeaveInstance`/`WeaveKind` are render types in `astraweave-render`. Neither crate depends on the other; the translation is one-directional and confined to the `weaving_playground` binary glue (`weave_producer.rs:6-20`).
- **`astraweave-water` vs `astraweave-fluids`:** `astraweave-water` is the gameplay-truth facade (this trace's spine). `astraweave-fluids` is the DEPRECATED-solver crate now retained only as the F.4 particle-accent substrate plus a W.3+-deferred effects layer — see [`fluids.md`](./fluids.md). The voxel `WaterQuery` backend that once bridged them was removed in W.1.
- **Surface "wave height" vs weave "height":** Gerstner crest height drives foam and shallow tint; the weave height offset is an additive deformation captured *separately* so a raise doesn't read as foam (`water.wgsl:261-289`).

---

## 4. Cross-System Touchpoints

### Upstream (what feeds this system)

| Source system | Interface | Data | Notes |
|---|---|---|---|
| Physics (back-compat scalar plane) | `AnalyticWater::set_plane(level, density)` from `apply_buoyancy_forces` | Infinite plane truth | `astraweave-physics/src/lib.rs:1435`; `NEG_INFINITY` clears the plane (`water/lib.rs:142-151`) |
| Gameplay authoring | `Physics::add_water_aabb(min,max,density,drag)` → `AnalyticWater::add_aabb` | Bounded pool/tank truth | `astraweave-physics/src/lib.rs:1487` |
| Fate-weaving | `WeaveOp` (kind `LowerWater`/`RaisePlatform`/`FreezeWater`) applied in `astraweave-gameplay/src/weaving.rs:70-105` | Deform/accent trigger + world location (`op.a`) | Consumed by the example-layer producers via `ingest(&op)` |
| Camera / frame clock | `Renderer::update_water(view_proj, camera_pos, time)` → `WaterRenderer::update` | View-proj, camera pos, animation time | `astraweave-render/src/renderer.rs:4632` |
| Biome system | `Renderer::sync_biome_sky_water` → `WaterRenderer::set_water_colors(deep, shallow, foam)` | Deep/shallow/foam colours | `astraweave-render/src/renderer.rs:3878-3896` |
| Opaque render pass | `copy_texture_to_texture(hdr → water_scene_color)` + scene depth | Scene-color snapshot + depth for refraction/foam | `astraweave-render/src/renderer.rs:4711-4726` |

### Downstream (what consumes this system's output)

| Consumer system | Interface | Data | Notes |
|---|---|---|---|
| Physics buoyancy | `WaterQuery::sample(point)` | `WaterSample{surface_height, density}` → Archimedes impulse | `astraweave-physics/src/lib.rs:1464`. The one wired non-test/non-example consumer of the truth facade |
| Gameplay swim/oxygen | `WaterPlayerState::update(submersion, dt)` | Movement mode, wet status, drowning damage | `astraweave-gameplay/src/water_movement.rs:206`. Consumes a `submersion` float computed by the caller — does NOT itself call `WaterQuery` |
| HDR composite / tonemap | water draws into HDR target inside `run_water_pass` | Refracted/foamed/deformed water pixels + additive accents | `astraweave-render/src/renderer.rs:4679` |
| Fluids accent renderer | `FluidRenderer::render_accents` via `set_hdr_overlay` closure (wgpu types only) | Splash/spray billboards | Binary-orchestrated; no render↔fluids Cargo edge (`F4_3_EXECUTION_REPORT.md` §3) |

### Bidirectional / Coupled

- **Physics ↔ `astraweave-water`:** Physics owns an `AnalyticWater` field (`astraweave-physics/src/lib.rs:931`), writes the scalar plane into it each tick, and reads it back via `sample`. The dependency is `physics → water → glam`, verified acyclic (the facade is a Cargo leaf below physics; `W2_DECISIONS.md` standing red line #2).
- **`Renderer` ↔ `WaterRenderer`:** The renderer holds an `Option<WaterRenderer>` and `take()`s it for the duration of `run_water_pass` to keep field borrows disjoint (`renderer.rs:4679-4768`). See the W-FU-3 panic-gap caveat in §11.

---

## 5. Active File Map

| File | Role | Status | Notes |
|---|---|---|---|
| `astraweave-water/src/lib.rs` | `WaterQuery` trait + `WaterSample` + `AnalyticWater` backend | Active | The single owner of gameplay water truth. 9 inline tests incl. the gate-Q1 determinism test (`lib.rs:319-349`) |
| `astraweave-physics/src/lib.rs:1429-1489` | `apply_buoyancy_forces` + `add_water_aabb` | Active | The one wired `WaterQuery::sample` production consumer |
| `astraweave-render/src/water.rs` | `WaterRenderer`: chunked-LOD Gerstner surface, `WaterUniforms`, weave instances, set_water_level/colors/rain | Active | ~990 LoC; owns surface + weave *presentation*. 7 inline tests (incl. on-GPU `new_and_update`) (verified — 7 `#[test]` fns in the `mod tests` at `water.rs:839-991`; the prior "8" was a miscount) |
| `astraweave-render/src/shaders/water.wgsl` | Vertex (Gerstner + weave deform + skirt) + fragment (refraction, depth-foam, freeze material) | Active | Mirrors `WaterUniforms` byte layout; `WEAVE_MAX_DEFORM` const must equal `SKIRT_DEPTH` |
| `astraweave-render/src/renderer.rs` | `run_water_pass`, `update_water`, `set_water_level`, `set_water_weave_instances`, `set_hdr_overlay`, scene-color snapshot | Active | Split water pass called from both `render()` and `draw_into()` |
| `astraweave-gameplay/src/types.rs:42-58` | `WeaveOpKind` enum (incl. `LowerWater`/`RaisePlatform`/`FreezeWater`) + `WeaveOp` | Active | `#[non_exhaustive]` — the deform trigger source |
| `astraweave-gameplay/src/weaving.rs:70-105` | `WeaveOpKind` application (incl. `FreezeWater` presentation-only arm) | Active | `FreezeWater` consumes weather budget + carries `op.a`; NO truth mutation (W.2c.3) |
| `astraweave-gameplay/src/water_movement.rs` | Swim/dive/oxygen/wet-status state machine (`WaterPlayerState`, `WaterMovementHelper`) | Dormant (tested only) | Self-contained; consumes a scalar `submersion` float clamped in `update` (`water_movement.rs:206-207`), NOT `WaterQuery`; the crate has no `astraweave-fluids`/`astraweave-water` dep. Zero workspace callers — verified, and concordant with `gameplay.md` §5/§6 which marks it Dormant residue of an abandoned Enshrouded-style water plan. ~50 inline mutation-resistant tests. (Prior "Active" status corrected — it compiles but has no production driver) |
| `examples/weaving_playground/src/weave_producer.rs` | `WaterWeaveProducer`: `WeaveOp` → `WeaveInstance` + lifetime envelope | **Example-only** | Binary glue; the only place knowing BOTH gameplay+render weave types. 5 tests. NOT a library crate |
| `examples/weaving_playground/src/weave_accent_producer.rs` | `WaterAccentProducer`: `WeaveOp` → `SecondaryParticle` accent emitters | **Example-only** | Binary glue; CPU particle aging + xorshift PRNG. 8 tests. NOT a library crate |
| `examples/weaving_playground/src/main.rs` | Demo driver: builds `WaterRenderer`, drives producers, registers HDR overlay | **Example** | The wiring that lights the whole presentation flow live |
| `astraweave-render/examples/water_budget_probe.rs` | W.2a/W.2b GPU-timestamp budget instrument | Active (probe) | Isolated + full-frame water-pass cost on the documented min-spec |
| `examples/weaving_playground/examples/accent_budget_probe.rs` | F.4.3 combined surface+accent frame measurement | Active (probe) | Lives here so no render↔fluids Cargo edge is added |
| `tools/aw_editor/src/viewport/{engine_adapter.rs,renderer.rs}` | Editor `WaterRenderer` install + `update_water` wiring (W-FU-2) | Active | `engine_adapter.rs:3739` builds it; `renderer.rs:697` drives `update_water` — woke the formerly-dormant editor water |
| `astraweave-fluids` (`FluidSystem`/`FluidRenderer`/`secondary.wgsl`) | F.4 GPU-particle accent substrate | Active (accent substrate) | DEPRECATED-solver crate; see [`fluids.md`](./fluids.md). Only the accent surface is wired here |

**Status definitions:**
- **Active**: Canonical, load-bearing, edit with care.
- **Example-only**: Compiles and is wired into the `weaving_playground` binary, but is NOT a shared library system — no engine/library crate calls it (verified §6). A future phase relocates the producers to a shared crate (logged F.4.3-editor).

---

## 6. Conflict Map / Residue

### Wired-vs-dormant inventory (the §8/Key-Lesson-8 honesty signal)

Verified via `rg` for non-test, non-example production callers at `7c29b8182`:

| Subsystem | Production caller? | Disposition |
|---|---|---|
| `WaterQuery::sample` / `AnalyticWater` | **YES** — `astraweave-physics/src/lib.rs:1464` (`apply_buoyancy_forces`) | Wired into the runtime |
| `WaterRenderer` (surface + refraction + foam + weave) | **YES** — `tools/aw_editor` (`engine_adapter.rs:3739/3802`), `examples/{weaving_playground,hello_companion,veilweaver_demo}` per `W2A_EXECUTION_REPORT.md` §3 | Wired (editor is in-engine; the others are examples but real consumers) |
| `Renderer::set_water_weave_instances` | Example-only — driven solely by `examples/weaving_playground/src/main.rs:530` | Mechanism wired in `Renderer`; the *feed* is example-only |
| `Renderer::set_hdr_overlay` (accent composite) | Example-only — registered solely by `weaving_playground` | Mechanism wired; feed example-only |
| `WaterWeaveProducer` / `WaterAccentProducer` | **NO** library caller — `rg WaterWeaveProducer\|WaterAccentProducer` returns only `examples/weaving_playground/src/` | Example-only (in-design-but-tested at the binary layer) |
| `astraweave-gameplay::water_movement` (`WaterPlayerState`) | **NO** non-test caller — workspace `rg WaterPlayerState\|WaterMovementHelper` at `7c29b8182` returns only `astraweave-gameplay/src/{water_movement.rs,mutation_tests.rs}` and `astraweave-gameplay/tests/mutation_resistant_comprehensive_tests.rs` (all `#[cfg(test)]`/`tests/`); `pub use water_movement::*` re-exports it (`astraweave-gameplay/src/lib.rs:51`) but nothing consumes it | In-design-but-tested; not wired to a game loop (verified — no production caller) |
| `FreezeWater` weave *truth* (walkable ice / buoyancy block) | **NO** — `weaving.rs:94-105` is presentation-only by design | Deliberately deferred (truth-coupling phase) |

The honest summary: **water truth and the rendered surface are wired; the weave/accent *producers* and `water_movement` are not wired into a shipped library/game-loop — they live at the example layer or are tested-but-uncalled.** This matches the W-series gates (`F4_3_EXECUTION_REPORT.md` §3/§5: "relocate `WaterAccentProducer` out of the `weaving_playground` binary into a shared crate" is logged as follow-on, not done).

### Coexisting abstractions

| Abstraction | Files | Status | Notes |
|---|---|---|---|
| `AnalyticWater` (analytic plane + AABBs) | `astraweave-water/src/lib.rs` | Active | The sole `WaterQuery` backend post-W.1 |
| Voxel `WaterQuery` backend | (removed) | Deleted (W.1) | `astraweave-water/src/voxel.rs` (210 LoC) + the `voxel` feature/dep removed in W.1 (`W1_EXECUTION_REPORT.md` §2). `WaterQuery` stays a trait so a future backend can be added |
| SPH / voxel fluid *simulation* | (removed) | Deleted (W.1) | 58,796 deletions; preserved only at tag `w0-pre-deprecation` @ `3a8296038`. See [`fluids.md`](./fluids.md) §0.5 |
| `astraweave-fluids` effects layer (caustics/foam/god_rays/underwater/waterfall…) | `astraweave-fluids/src/*.rs` | DEFERRED (W.3+) | Untouched by W.1; not wired into the W surface layer. Disposition pending later W phases |
| Scalar `water_level`/`fluid_density` on physics | `astraweave-physics/src/lib.rs` | Active (back-compat) | NOT a parallel store — write-through-synced into `AnalyticWater::set_plane` so there is one sampling path (`water/lib.rs:118-126`) |

### Naming collisions

- **"Water":** `astraweave-water` = gameplay truth facade. `astraweave-render::water` = surface presentation. `astraweave-fluids` = particle/effects substrate (deprecated solver). `water_movement` (gameplay) = swim/oxygen state machine. Four crates, four meanings; always qualify.
- **"Weave":** `WeaveOp`/`WeaveOpKind` (gameplay) vs `WeaveInstance`/`WeaveKind` (render) vs `AccentKind` (accent producer). The gameplay→render→accent mapping is `LowerWater→Part`, `RaisePlatform→Raise`, `FreezeWater→Freeze` (`weave_producer.rs:49-56`, `weave_accent_producer.rs:56-63`).
- **"height":** Gerstner crest height (foam/tint driver) vs weave additive height offset vs water-surface world Y (`water_level`). Captured separately in `water.wgsl:261-289`.

### Known cognitive traps

- **Trap:** Assuming the part/freeze/raise effects register behind the `WaterQuery` facade (the original W.2 §B wording said so).
  **What's actually true:** That wording was **corrected** in W.2c.1 (`W2_DECISIONS.md` §B.1). Weave *presentation* is owned render-side by `WaterRenderer` because the facade contract excludes GPU/presentation state and `astraweave-render` does not depend on `astraweave-water`. The facade is untouched by the view-side weave work. Truth → facade; presentation → renderer.
- **Trap:** Reading `astraweave-fluids` and assuming it is the active water simulation.
  **What's actually true:** The SPH/voxel solver was deprecated and removed in W.1. `astraweave-fluids` now contributes only the GPU-particle *accent* substrate (plus a deferred effects layer). See [`fluids.md`](./fluids.md) §0.5.
- **Trap:** Treating the `weave_producer.rs`/`weave_accent_producer.rs` translation as a shipped library system.
  **What's actually true:** Both live in the `weaving_playground` binary and have zero library callers. Relocating them to a shared crate is logged but not done (`F4_3_EXECUTION_REPORT.md` §3).
- **Trap:** Expecting `FreezeWater` to make ice walkable or block buoyancy.
  **What's actually true:** It is presentation-only today (`weaving.rs:94-105`); truth-coupling is a deferred phase.

---

## 7. Decision Log

### Decision: Re-scope water from simulation to layered rendering (W.0)
- **Date:** 2026-06 (W.0 ratification)
- **Status:** Accepted
- **Context:** F.3.S measured that the SPH/voxel solver cannot carry general water on min-spec (full-extent flooding never fit; 2.35 ms floor at 5% fill on a 64³ grid). Independently, Veilweaver's design never requires general water — water is interactive-but-scripted, fate-weaving needs only a bounded part/freeze/raise vocabulary, and the camera relationship is a rendering-LOD concern (`w-series campaign.md` §W.0).
- **Decision:** Deprecate the solver core; build a layered rendering system (surface / weave-response / context-LOD / particle accents) with all gameplay-water truth behind `WaterQuery`.
- **Alternatives considered:** Continue the solver (declined — measured impossible on min-spec); a from-scratch hybrid grid-particle method (APIC/MLS-MPM with multigrid pressure) is the documented restart point IF min-spec ever rises, but explicitly NOT a resurrection of the tagged SPH/voxel solver (`w-series campaign.md` research pointer).
- **Consequences:** ~58.8K LoC removed in W.1; emergent fate-weaving runs on terrain (water scenes authored water-free); a permanent escape-hatch design constraint that the two do not overlap.

### Decision: `WaterQuery` is the single owner of gameplay water *truth*; one method
- **Date:** F.2 (predates W; carried forward)
- **Status:** Accepted
- **Context:** Physics (and later AI/gameplay) each carrying a private notion of "the water" violates the §7.7 no-second-implementation mandate. The only question the wired consumer (physics buoyancy) asks is "sample this point" (`astraweave-water/src/lib.rs:24-34`).
- **Decision:** A one-method `WaterQuery` trait returning `WaterSample{surface_height, density}` — exactly the two quantities physics reads. Flow/drag/temperature are deliberately NOT exposed (the dormant-speculative-API anti-pattern F.2 exists to avoid).
- **Alternatives considered:** A richer sample carrying flow/drag/temperature — rejected because no wired consumer reads them; they arrive with the consumer that needs them (`lib.rs:24-34, 44-46`).
- **Consequences:** A minimal, deterministic surface; new fields are added only when a real consumer needs them.

### Decision: Q1 determinism carve-out — presentation excluded from world hash/replay/net
- **Date:** Fluids campaign gate Q1 (binding, carried into W)
- **Status:** Accepted
- **Context:** GPU particle fluid state is non-deterministic by construction (atomic neighbor-list order × float non-associativity).
- **Decision:** The `astraweave-water` truth layer is CPU-resident and deterministic by construction (two backends with the same volumes return bit-identical samples; order-independent — enforced by the test at `lib.rs:319-349`). GPU particle/water *presentation* state is presentation-only and permanently excluded from `WorldSnapshot`, `world_hash`, replay, and net replication (`lib.rs:10-22`; `fluids.md` §0).
- **Alternatives considered:** [Reasoning not recovered from available sources beyond the stated non-determinism.] (Checked `docs/architecture/fluids.md` §0 (`Determinism carve-out (campaign gate Q1, policy)`, line 36-37) and `docs/campaigns/water-successor/w-series campaign.md` red line #1 — both state the carve-out as binding policy grounded in the same non-determinism rationale; neither records an alternatives-considered discussion. No further reasoning recovered.)
- **Consequences:** Any PR that hashes/replicates/replays particle state must be rejected at review. The standing red lines forbid water state in `WorldSnapshot` ever (`w-series campaign.md` red line #1).

### Decision: Weave *presentation* is render-side, not facade-side (W.2c.1 correction)
- **Date:** 2026-06-22 (W.2c.1 recon)
- **Status:** Accepted — supersedes the original W.2 §B wording
- **Context:** The W.2.0 ratification said weave-response "registers behind `astraweave-water`'s `WaterQuery`." The W.2c.1 recon proved this wrong: the facade contract excludes GPU/presentation state, and `astraweave-render` does not depend on `astraweave-water` (only physics does), so putting a render-consumed weave list in the facade would invert the dependency graph (`W2_DECISIONS.md` §B.1).
- **Decision:** Weave *presentation* (part/freeze/raise deformation) is owned render-side by `WaterRenderer`, exactly as Gerstner/refraction/depth-foam are. The facade stays the single owner of water *truth* and is untouched.
- **Alternatives considered:** Adding a `render → water` dependency (rejected — inverts the verified-acyclic graph).
- **Consequences:** Weave deformation rides in `WaterUniforms` (`water.rs:177-184`); weave *truth* would be a separate facade concern if ever needed.

### Decision: Gerstner-first; FFT fork CLOSED
- **Date:** §C ratification (2026-06-21), closed in §F.1 (2026-06-22)
- **Status:** Accepted — do not re-open
- **Context:** FFT's O(log N) scaling wins only at open-ocean scale AstraWeave does not have; its fixed base cost penalizes a bandwidth-limited min-spec card (1660 Ti Max-Q); maintaining FFT + a min-spec fallback is two water systems a solo dev developing on min-spec cannot justify (`W2_DECISIONS.md` §F.1, `WATER_RESEARCH_FINDINGS.md`).
- **Decision:** Extend the existing 4-wave Gerstner; FFT is closed, not merely deferred.
- **Alternatives considered:** FFT spectral ocean (rejected as above).
- **Consequences:** Gerstner's linear-in-wave-count cost is dial-able against the 2.0 ms budget.

### Decision: Chunk-grid LOD with skirts; weave deform bounded to skirt depth
- **Date:** W.2a (2026-06-22)
- **Status:** Accepted
- **Context:** Replaced the single hardcoded `generate_water_plane(500,128)` plane. LOD boundaries between coarse/fine chunks produce curve-vs-chord cracks (≤ total wave amplitude).
- **Decision:** Discrete `CHUNK_SIZE=64` tiles, per-LOD pre-baked meshes drawn instanced, outward-facing per-chunk skirts dropping `SKIRT_DEPTH=8.0` (≫ ~1.65 max amplitude). `WEAVE_MAX_DEFORM` is tied to `SKIRT_DEPTH` and net weave height is clamped so a deform can never outrun the skirt (`water.rs:60-60, 112-114`; `water.wgsl:224`).
- **Alternatives considered:** Continuous projected-grid/clipmap (deferred unless open-ocean horizon scenes require it; `W2_DECISIONS.md` §C).
- **Consequences:** Seam handling is structural; weave deformation is defence-in-depth bounded.

### Decision: Accent composite via pure-wgpu HDR overlay closure (no render↔fluids edge)
- **Date:** F.4.3 (2026-06-24)
- **Status:** Accepted
- **Context:** Accents live in `astraweave-fluids`; the water surface lives in `astraweave-render`; neither crate may depend on the other (standing red line #2 / invariant #18).
- **Decision:** `Renderer` exposes `set_hdr_overlay(Box<dyn FnMut(&mut Encoder, &TextureView, &TextureView, &Device, &Queue)>)`; the binary registers a closure whose body calls `FluidRenderer::render_accents`. Only wgpu types cross the seam; the combined budget probe lives in `weaving_playground` (`F4_3_EXECUTION_REPORT.md` §1/§3).
- **Alternatives considered:** A direct render→fluids call (rejected — adds a Cargo edge).
- **Consequences:** The composite is binary-orchestrated; `astraweave-render` is unchanged for non-demo consumers (zero-accent identity).

---

## 8. Known Invariants

| # | Invariant | Checkable? | Enforced by |
|---|---|---|---|
| 1 | No particle/water *presentation* state in `WorldSnapshot`, `world_hash`, replay, or net | Yes (grep) | Doc-policy + review (red line #1); `astraweave-water/src/lib.rs:10-22` |
| 2 | `AnalyticWater::sample` is deterministic: two backends with the same volumes return bit-identical samples; result is independent of query order | Yes | Enforced by test `determinism_identical_backends_and_order_independence` (`lib.rs:319-349`) |
| 3 | Overlap resolution is topmost-surface-wins, ties by registration order | Yes | Tests `overlap_resolution_topmost_surface_wins` / `overlap_tie_breaks_by_registration_order` (`lib.rs:283-301`) |
| 4 | `WaterQuery::sample` returning `Some` does NOT imply submersion; the caller compares `point.y` to `surface_height` | Yes | Doc contract (`lib.rs:67-71`) + physics `pos.y >= sample.surface_height` guard (`physics/lib.rs:1468`) |
| 5 | Water is a Cargo leaf: `physics → water → glam`; `render` does NOT depend on `water`; no cycle `physics → fluids → terrain → gameplay → physics` | Yes (cargo) | Cargo graph; red line #2; `W2_DECISIONS.md` §B.1 |
| 6 | A single weave deformation's height contribution stays within `±SKIRT_DEPTH` so it never re-exposes a LOD seam | Yes | `WEAVE_MAX_DEFORM == SKIRT_DEPTH` (`water.rs:60`), intensity clamp (`water.rs:112-114`), net clamp (`water.wgsl:224`) |
| 7 | `WaterUniforms` is 512 B with `inv_view_proj` at offset 160, `weave_count` at 240, `weave_instances` at 256; `WeaveInstanceRaw` is 32 B | Yes | Test `test_uniforms_size` (`water.rs:874-886`); WGSL struct mirrors it (`water.wgsl:35-65`) |
| 8 | The water pass is skipped when `has_visible_chunks()` is false (no wasted scene-color copy) | Yes | `run_water_pass` early-return (`renderer.rs:4695`); test `new_and_update` asserts dormant fresh renderer (`water.rs:928-945`) |
| 9 | No gameplay type crosses into `astraweave-render`; the `WeaveOp → WeaveInstance` translation lives only in the binary glue | Yes (grep) | `set_water_weave_instances` takes render types (`renderer.rs:4658`); producers in `examples/` only |
| 10 | Zero active weaves / accents → byte-identical frame (additive-zero identity) | Partly | `set_weave_instances([])` clears all (`water.rs:790-800`); `render_accents` early-returns at count 0 (`F4_3_EXECUTION_REPORT.md` §3) |

---

## 9. Performance & Resource Profile

Measured on the documented min-spec: **NVIDIA GeForce GTX 1660 Ti with Max-Q Design · Vulkan · DiscreteGpu · driver 592.27 · 1920×1080**, real wgpu `TIMESTAMP_QUERY` via the production `GpuProfiler`, medians over 240-300 frames after warm-up.

### Hot paths
- **Water surface pass** (`render()` per frame): near ≈ 0.24 ms, horizon ≈ 0.18 ms (`F4_3_EXECUTION_REPORT.md` §2). W.2a chunked surface added +0.04-0.06 ms over the single-plane baseline (`W2A_EXECUTION_REPORT.md` §2).
- **Scene-color copy** (full-res Rgba16Float, for refraction): ≈ 0.085 ms — the dominant *added* W.2b cost (bandwidth, not shader math) (`W2B2_EXECUTION_REPORT.md` §3).
- **Accent composite** (512 additive billboards): ≈ 0.006-0.013 ms — effectively free (`F4_3_EXECUTION_REPORT.md` §2).
- **Combined surface + accents:** ≈ 0.26 ms worst-case, **~8× under the 2.0 ms provisional budget** (`F4_3_EXECUTION_REPORT.md` §2/§4).
- **`AnalyticWater::sample`** (CPU, per buoyant body per tick): a linear scan over the AABB set + plane; intended for small registered volume counts.

### Cold paths
- **LOD tile mesh bake + buffer creation:** once at `WaterRenderer::new` (`water.rs:474-502`).
- **Bind-group rebuild in `prepare_scene`:** only on `resource_gen` change (resize), not per frame (`water.rs:730-740`).

### Resource ownership
- **`WaterRenderer`** (pipeline, bind group, uniform buffer, per-LOD meshes + instance buffers): owned by `Renderer` as `Option<WaterRenderer>`; `take()`/restored across `run_water_pass`.
- **`water_scene_color`** texture/view: owned by `Renderer`; recreated on resize (`renderer.rs:3963-3978`).
- **`AnalyticWater`**: owned by `PhysicsWorld` (`astraweave-physics/src/lib.rs:931`); lifetime = physics world lifetime.

**Budget caveat:** the 2.0 ms ceiling is **provisional** pending a real-scene capture; a representative full-frame headroom was not headless-measurable in the W.2a environment (`W2A_EXECUTION_REPORT.md` §2).

---

## 10. Testing & Validation

- **Unit tests (inline `#[cfg(test)]`):**
  - `astraweave-water/src/lib.rs` — 9 tests incl. determinism/order-independence, overlap resolution, AABB containment, plane sentinel.
  - `astraweave-render/src/water.rs` — 7 tests incl. on-GPU `test_water_renderer_new_and_update` (LOD mesh count, all chunks assigned via `MAX_CHUNKS`, `set_water_level`, weave instance clamp/ceiling/clear), uniform-size invariant (verified — 7 `#[test]` fns at `water.rs:839-991`; the prior "8" was a miscount).
  - `astraweave-gameplay/src/water_movement.rs` — ~50 mutation-resistant tests (threshold boundaries, oxygen math, wet-status transitions, force arithmetic).
  - `examples/weaving_playground/src/weave_producer.rs` — 5 tests (op→instance mapping, envelope, expiry, ceiling).
  - `examples/weaving_playground/src/weave_accent_producer.rs` — 8 tests (emitter mapping, continuous vs one-shot, expiry, budget cap, zero-state identity).
- **Integration tests:** `astraweave-render/src/renderer_tests.rs` and `tests/coverage_booster_render.rs` install a `WaterRenderer` and drive `update_water` (note: `coverage_booster_render.rs` is pre-existing broken test debt — W-FU-1, NOT a water regression; `W2A_EXECUTION_REPORT.md` §5).
- **GPU-execution tests (fluids accent substrate):** `astraweave-fluids/tests/gpu_execution_tests.rs` incl. `gpu_render_accents_smoke` (`F4_3_EXECUTION_REPORT.md` §3).
- **Benchmarks / budget probes (real GPU timestamps, min-spec):**
  - `astraweave-render/examples/water_budget_probe.rs` — isolated + full-frame water-pass cost.
  - `examples/weaving_playground/examples/accent_budget_probe.rs` — combined surface+accent frame.
  - `astraweave-render/examples/depth_sample_capability_probe.rs` — read-only-depth + same-texture sampling capability guard (re-run after any wgpu upgrade; `W2B2_EXECUTION_REPORT.md` §1).
- **Manual / visual validation:** render-correctness pixel-lit readbacks (62.8% lit near view, W.2a; refraction 12.8-25%, foam 7.9% near-white, W.2b) — `W2A/W2B2_EXECUTION_REPORT.md`. Per the evidence-discipline red line, log counters alone are insufficient — captures / GPU readback assertions are required.

---

## 11. Open Questions / Parked Decisions

- **Relocating the producers out of the example layer (F.4.3-editor follow-on).** `WaterWeaveProducer` and `WaterAccentProducer` live in the `weaving_playground` binary with zero library callers. The editor's accent pass mechanism is already live in `draw_into`, but visible editor accents need the producers in a shared crate + an editor producer feed (`F4_3_EXECUTION_REPORT.md` §3/§5). When/where is this relocation done?
- **Is `astraweave-gameplay::water_movement` (`WaterPlayerState`) wired to a game loop?** Confirmed **not wired** (it is dormant, tested-only). It is fully tested but in-design-only; the parked question is who would eventually wire it and who would compute the `submersion` float it consumes.
  - *Verification note (2026-06-25):* a workspace `rg WaterPlayerState|WaterMovementHelper` at `7c29b8182` confirms **zero non-test callers** — every hit is inside `#[cfg(test)]` modules (`water_movement.rs`, `mutation_tests.rs`) or `astraweave-gameplay/tests/mutation_resistant_comprehensive_tests.rs`. It is re-exported via `pub use water_movement::*` (`astraweave-gameplay/src/lib.rs:51`) but consumed by nothing. `update` takes a caller-supplied scalar `submersion` (`water_movement.rs:206-207`); there is NO `WaterQuery`/`astraweave-fluids` bridge — `gameplay.md` §6 records the crate has no such dep and frames the module as dormant residue of an abandoned Enshrouded-style water plan (`docs/current/WATER_SYSTEM_ENHANCEMENT_PLAN.md` staleness banner). The factual "no caller" question is now resolved across two independent traces; who (if anyone) wires it and computes `submersion` remains the parked decision.
- **`FreezeWater` (and parted-corridor / raised-platform) truth-coupling.** Today these are presentation-only (`weaving.rs:94-105`). The deferred "gameplay-truth-coupling phase" would make a frozen patch walkable / block buoyancy via a *separate facade concern* (with the existing `WeaveOpKind::LowerWater` precedent). When this lands, the truth side gains a new `WaterQuery` consumer or backend.
- **W-FU-3: `run_water_pass` take/restore panic gap.** A panic between `self.water_renderer.take()` and restore would drop the `WaterRenderer` for the process lifetime (`renderer.rs:4684`, `W2B2_EXECUTION_REPORT.md` §6). The F.4.3 overlay correctly fires *outside* that window but does not close the pre-existing gap. The normal path never panics (wgpu errors route through the error scope). Is closing this gap worth a refactor?
- **Provisional 2.0 ms budget ceiling.** Still provisional pending a real-scene capture; the headless probe renders a near-empty default scene (`W2A_EXECUTION_REPORT.md` §2/§7). Should a windowed-capture budget pass replace the assumption?
- **Context/LOD selector (distant / mid / submerged technique swap).** The W target architecture names a camera-distance context selector (surface shading / interaction ripples / underwater fog+caustics) as layer 3. The chunked-LOD surface and rain-ripple normals exist, but the submerged-fog/caustics/light-shaft mode is part of the W.3+-deferred effects layer (still in `astraweave-fluids`, untouched). Is the context selector a future W phase, or absorbed into the deferred effects layer?
- **Orphaned `parallel`/`rayon` in `astraweave-fluids`.** Janitorial item carried from W.1 (retained for a future scaled accent path; `W1_EXECUTION_REPORT.md` §3) — out of this trace's scope but cross-referenced for completeness.

---

## 12. Maintenance Notes

**Update this doc when:**
- Any Active file in §5 changes structurally (e.g. `WaterUniforms` layout, `WaterQuery` signature, the weave instance schema, `run_water_pass` ordering).
- A producer is relocated from the example layer into a shared crate (resolves the §11 follow-on and changes §5/§6 wired-vs-dormant status).
- `FreezeWater` (or any weave) gains truth-coupling (a new `WaterQuery` consumer/backend appears — update §2A, §4, §6).
- A decision in §7 is superseded (e.g. the FFT fork is re-opened — currently forbidden).
- An invariant in §8 is broken or newly enforced (especially the determinism test, the skirt/weave bound, or the Cargo-leaf graph).

**Verification process:**
- Spot-check the §2 pipeline diagrams against `astraweave-water/src/lib.rs`, `astraweave-render/src/{water.rs,renderer.rs}`, `shaders/water.wgsl`, and `astraweave-physics/src/lib.rs:1429`.
- Re-run the wired-vs-dormant grep in §6 (`rg 'WaterWeaveProducer|WaterAccentProducer|WaterQuery::sample|set_water_weave_instances' --type rust -g '!*test*' -g '!*example*'`).
- Re-measure the budget probes if the surface/refraction/accent code changes >10%.
- Stamp the new commit hash + date in the Metadata table after verification.

---

## Appendix A: Quick reference for agents

**If you're working on this system, remember:**
1. **Truth vs presentation is the load-bearing split.** Gameplay water truth → `astraweave-water` (`WaterQuery`/`AnalyticWater`, CPU, deterministic). Rendered surface + weave deformation → `astraweave-render` (`WaterRenderer`, GPU, non-deterministic, excluded from world_hash/replay/net). Do not put presentation state behind the facade, and do not make `render` depend on `water`.
2. **The weave producers are example-only.** `WaterWeaveProducer`/`WaterAccentProducer` live in the `weaving_playground` binary with zero library callers. Don't describe them as a shipped library system; relocating them is logged but undone.
3. **`FreezeWater` is presentation-only.** No walkable ice / buoyancy block exists yet.
4. **The solver is gone.** `astraweave-fluids` contributes only the particle-accent substrate (+ deferred effects). The SPH/voxel sim was removed in W.1 (tag `w0-pre-deprecation`). Don't resurrect it — the documented restart is a different method family (APIC/MLS-MPM).
5. **Keep weave deform within `±SKIRT_DEPTH`.** Otherwise a deformation re-exposes a LOD seam.

**Files you'll most likely touch:**
- `astraweave-render/src/water.rs` + `shaders/water.wgsl` (surface + weave presentation)
- `astraweave-render/src/renderer.rs` (`run_water_pass`, water/weave/accent API)
- `astraweave-water/src/lib.rs` (truth facade — change only with a real consumer)
- `examples/weaving_playground/src/{weave_producer.rs,weave_accent_producer.rs}` (op→instance/accent translation)

**Files you should NOT touch without strong reason:**
- `astraweave-water/src/lib.rs` API surface — adding fields/methods without a wired consumer is the dormant-speculative-API anti-pattern F.2 exists to avoid.
- The determinism test (`lib.rs:319-349`) — it enforces the gate-Q1 carve-out.

**Common mistakes when changing this system:**
- **Adding a `render → water` dependency** to "register weaves behind the facade." This was explicitly corrected in W.2c.1; weave presentation is render-side.
- **Letting `WEAVE_MAX_DEFORM` and `SKIRT_DEPTH` drift apart** (in `water.rs` and the WGSL const) — they must stay equal or seams reappear.
- **Hashing/replicating/replaying any GPU water/particle state** — forbidden by the Q1 carve-out.
- **Editing `WaterUniforms` without updating the WGSL mirror + the `test_uniforms_size` offsets** — the std140 layout is byte-matched.

---

## Appendix B: Historical context

Water reached its current shape through the F-series (Fluids Integration) and then the W-series (Water Successor) campaigns. F.2 introduced the `WaterQuery`/`AnalyticWater` facade as a deprecation seam below physics. F.3 added (and F.3.S then measured the limits of) a voxel water simulation. The W.0 ratification (2026-06) declared the general solver unnecessary for Veilweaver and impossible on min-spec, re-scoping water to a layered rendering system. W.1 removed ~58.8K LoC of SPH/voxel solver (preserved only at tag `w0-pre-deprecation` @ `3a8296038`) and deleted the voxel `WaterQuery` backend, leaving `AnalyticWater` as the sole truth backend. W.2a built the chunked-LOD Gerstner surface and real `set_water_level`; W.2b.2 added screen-space refraction + depth-delta foam in a post-opaque split pass; W-FU-2 woke the editor's dormant water; W.2c.2/W.2c.3 added the position-agnostic weave-response deformation and the gameplay-`WeaveOp` producer (incl. the `FreezeWater` op); and F.4.0-F.4.3 built and measured the GPU-particle accent layer. The full arc — surface that scales, refracts, foams, deforms from real weave triggers, and throws accents — measures ~0.26 ms worst-case (~8× under budget) on the documented min-spec (`F4_3_EXECUTION_REPORT.md`).
