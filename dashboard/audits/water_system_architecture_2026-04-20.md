# Water System Architecture and Quality Audit

**Date:** 2026-04-20
**Scope:** Read-only audit of AstraWeave's water system — rendering, physics, configuration — to (a) answer whether Phase 1.5 heightmap-driven Beach biome work can safely proceed without water system changes first, and (b) capture a ground-truth baseline of the water system's current state for future planning.
**Method:** Static analysis only. No code changes. No fixes. No recommendations on what to do next — this audit states what exists, not what should be done.
**Verification hardware reference:** Cost estimates target GTX 1660 Ti Max-Q / Vulkan backend based on static shader and mesh analysis (frame-time measurement was not practical — see §4.4).

---

## 1. Executive finding

**Verdict for Phase 1.5 integration: YELLOW — proceed with caveats, no water-system changes required first.** The water system is a minimalist single-plane renderer decoupled from terrain: it enables only when the editor's `terrain_primary_biome` string is one of `swamp | beach | river` and is otherwise fully off, and even when "enabled" the editor never calls `update_water` per-frame, so the water plane renders with stale identity view-projection — effectively non-functional in the editor as of this audit. Phase 1.5's per-vertex Beach biome work is orthogonal to the editor's `primary_biome` field: as long as visual verification uses a non-water-triggering primary biome (e.g. `grassland`, the default), there is zero spatial or rendering conflict with the ocean plane. Physics-side the picture is similarly disconnected — a simple Y-plane buoyancy hookup exists and works for Rapier dynamic bodies; everything else (Archimedes volumes, currents, splash/wake, swimming/oxygen/wet-status, SPH fluids) is dead or unwired code paths. The Gerstner wave shader visibly tiles (the "riptide grid") because three of four wave directions cluster within 26° of the same axis and their frequencies form exact 1:2:4 integer ratios, so the lattice repeats every ~25 world units; the "flat plane" reading follows from the shader having no depth absorption, no refraction, no screen-space or planar reflection, no caustics, and constants (sky color, sun direction) that do not sync with the scene — Fresnel blends between two hardcoded colors, so there is no optical volume to read. Static-analysis frame cost is low (~<1 ms at baseline), but the rain-ripple code path does 27 transcendental calls per pixel and is the dominant cost when rain is active.

---

## 2. Phase 1.5 integration verdict (detailed)

**Proximate question:** Can Phase 1.5 assign the Beach biome to terrain vertices near sea level without conflicting with the existing water system?

**Verdict: YELLOW (proceed, with two caveats documented below).**

### 2.1 What works in favour of proceeding

1. **The water system is off by default.** `tools/aw_editor/src/viewport/engine_adapter.rs:790` initialises `water_enabled: false`. `tools/aw_editor/src/main.rs:3968-3974` runs per-frame and only flips this true when `terrain_primary_biome` is exactly `"swamp" | "beach" | "river"`. With the default (empty or `"grassland"`) terrain primary biome, water is not installed and no water-side draw call is issued.

2. **`primary_biome` string and per-vertex biome weights are orthogonal concepts.** Phase 1.5 modifies `TerrainVertex.biome_weights_0/1` per vertex (populating slot 6 = Beach for vertices near sea level). This is authoring data that flows into the terrain splat pipeline (completed in Phase 1.F). The editor's `terrain_primary_biome` is a single string field consumed only by the water-enable gate and sky/biome preset code. The two do not read each other.

3. **Even when water "enables", it renders non-functionally in the editor.** `EngineRenderAdapter::update_water` is defined at `tools/aw_editor/src/viewport/engine_adapter.rs:3872-3877` but grep confirms **zero callers** inside `tools/aw_editor`. Only `examples/veilweaver_demo` (visual_renderer.rs:833, 864), `examples/hello_companion` (visual_demo.rs:1235, 1384), and unit tests call it. In the editor the `WaterUniforms` defaults (view_proj = identity, camera_pos = [0, 5, -10], time = 0, rain = 0) never change, so the water mesh renders with an identity projection matrix producing garbage clip positions — visually broken, not a clean overlap.

4. **No biome-awareness on the water side.** The shader (`astraweave-render/src/shaders/water.wgsl`) reads zero biome data. The plane is a uniform Y=2.0 sheet that neither knows nor cares what biome the terrain underneath holds. It does not check for "Beach biome = don't render here."

### 2.2 Caveats (not blockers)

- **Caveat 1 (visual testing practice):** If the user sets `terrain_primary_biome = "beach"` specifically to visually verify Beach biome rendering, the water-enable gate fires and a 500×500 m plane at Y=2.0 will be installed. Because the editor never calls `update_water`, that plane renders with stale identity view-projection — the user may see a corrupt-looking water artifact that is NOT a Phase 1.5 regression but a pre-existing editor bug. Recommend testing Beach biome with `primary_biome = "grassland"` (or any non-trigger value) so water stays off and the sand color is cleanly visible.

- **Caveat 2 (future interaction):** Once someone fixes the `update_water` hookup in the editor (whoever picks up water-system work next), the Y=2.0 plane will correctly animate — and then wherever the terrain surface is below Y=2.0, the water plane will alpha-blend over it at 85–95% opacity, hiding whatever biome color was there including Beach. This is a future interaction, not a Phase 1.5 bug. Phase 1.5 can ship before water-system work without creating it.

### 2.3 What Phase 1.5 can safely do

- Assign Beach biome weight to terrain vertices near sea level via `TerrainVertex.biome_weights_0/1`.
- Visually verify the new coastal band of pale sand using the existing forward-lit splat pipeline landed in Phase 1.F.
- Leave the water system unchanged.

### 2.4 What Phase 1.5 should not do

- Assume the water system will render meaningfully alongside Beach biome in the editor (it currently does not).
- Change the water-enable gate, the `primary_biome` mapping, or any water shader/pipeline code to "fix" shoreline appearance — that is water-system work and out of scope.

---

## 3. Architecture map

### 3.1 Rendering architecture

**Top-level shape.** One type, one shader, one draw call.

```
Rendering flow (aw_editor via EngineRenderAdapter):
  EngineRenderAdapter::set_water_enabled (engine_adapter.rs:3831-3868)
      → WaterRenderer::new (water.rs:94)
      → sets biome color preset via set_water_colors (water.rs:279)
      → Renderer::set_water_renderer (renderer.rs:4656) stores Some(...)

Per-frame (aw_editor):
  [NOT CALLED — no caller of EngineRenderAdapter::update_water exists in tools/aw_editor]
  Would: view_proj + camera_pos + time → Renderer::update_water (renderer.rs:4666)
         → WaterRenderer::update (water.rs:263) writes UBO

Per-frame in working examples (veilweaver_demo, hello_companion):
  visual_renderer.rs:833,864 | visual_demo.rs:1235,1384
      → renderer.update_water(view_proj, camera_pos, elapsed)

Draw:
  Renderer::draw_into (renderer.rs:5780-5782) in main forward pass,
  after opaque models + terrain_forward, before weather particles,
  before tonemap/post — draws into hdr_view (Rgba16Float).
  Renderer::render (renderer.rs:5156-5158) does the same for direct-to-surface.
```

**Mesh geometry.** Single flat plane, 500×500 world units, 128×128 quad subdivisions → 16,641 vertices, 32,768 triangles, one indexed draw. Generated at `astraweave-render/src/water.rs:216-260` by `generate_water_plane(500.0, 128)`. Vertices hardcoded at `y = 2.0` (water.rs:232, positional literal in the vertex push).

**Sea level.** Baked into the vertex buffer at construction. `WaterRenderer::set_water_level` (water.rs:271-273) is a **no-op stub**: the body is a single comment `// Water level is controlled by the uniform, already at y=0 in mesh`, the parameter is prefixed `_level` and discarded, and no Y-level uniform exists in `WaterUniforms` (water.rs:13-29). All water-level plumbing through the editor (`terrain_panel.rs:428` → `tab_viewer` → `widget.rs:2628` → `renderer.rs:1337`) terminates here. Runtime Y is immutable.

**Render target.** `hdr_view` at `wgpu::TextureFormat::Rgba16Float` (renderer.rs:5951-5953, 1292). Shared with terrain forward, models, particles — all in the main forward pass. The editor adapter passes `self.renderer.surface_format()` (the swapchain format) as the water pipeline's color target at engine_adapter.rs:3834 while the actual attachment is hdr_view — a latent format-mismatch that `hello_companion` and `veilweaver_demo` avoid by passing `renderer.hdr_format()` instead. It may currently happen to be compatible on the verification hardware but it is an inconsistency.

**Bind groups.** Exactly one group with exactly one binding: group 0 / binding 0 = `WaterUniforms` UBO (144 bytes, VS + FS visibility, water.rs:114-137). No group 1–N. No scene-env, no sun UBO, no IBL cubemap, no reflection/refraction texture, no depth sampler, no shadow map. The shader declares only `@group(0) @binding(0)` at water.wgsl:34.

**Terrain interaction.** The 500×500 plane extends unconditionally beneath terrain wherever terrain height crosses Y=2.0. There is no stencil mask, no shoreline alpha-fade, no foam at intersection, no depth-delta absorption. Because `depth_write_enabled = false` (water.rs:177) and `depth_compare = LessEqual` (water.rs:178), water fragments render only where the plane is closer to camera than prior geometry; terrain above Y=2.0 occludes water naturally, producing a hard 1-pixel edge where the two meet. Gerstner peak amplitude sums to ~1.65 world units, so wave crests clip through shoreline geometry silhouettes.

**Depth behaviour for submerged objects.** Objects drawn before water (all opaque geometry in the main pass) write depth; water alpha-blends over them wherever the water surface is in front. There is no depth-based colour attenuation — an object 0.1 m below the surface receives the same constant water alpha (0.85–0.95, water.wgsl:219) as an object 100 m below. No underwater fog, no camera-above/below-water state.

**Architecture diagram (ASCII):**

```
        terrain_forward (Phase 1.F splat, forward-lit)
              │
              ▼
        [main forward pass @ hdr_view Rgba16Float]
              │
   ┌──────────┼──────────┐
   │          │          │
 opaque    terrain     water        ← single WaterRenderer instance
 models    splat       (if enabled  │
   │        │          & installed) │ draws 500×500 mesh, UBO 144 B,
   │        │          │            │ 1 draw_indexed call
   │        │          ▼            │
   │        │   weather particles   │
   └────────┴──────────┘            │
              │                     │
              ▼                     │
        [postprocess → swapchain]   │
                                    │
  Config gate:  main.rs:3968 looks at terrain_primary_biome
                matches "swamp|beach|river" → set_water_enabled(true)
                else set_water_enabled(false) → drops WaterRenderer
```

### 3.2 Physics architecture

**Three disjoint subsystems with near-zero cross-wiring.** See `docs/audits/PHYSICS_SYSTEM_AUDIT_REPORT.md` for broader physics context; this audit focuses on water-specific integration.

1. **Simple plane buoyancy (wired).** `astraweave-physics/src/lib.rs:1418-1447` implements `PhysicsWorld::apply_buoyancy_forces`, called every step at `lib.rs:1084`. Iterates `buoyancy_bodies`, checks `body_y < self.water_level` (infinite horizontal plane, `water_level` default `f32::NEG_INFINITY` at lib.rs:619, so disabled by default), applies `force = volume * fluid_density * 9.81` and linear drag `-velocity * drag`. **Actually pokes Rapier**: `rb.add_force(total_force, true)` at lib.rs:1442. This is the only water-physics path that reaches a running simulation.

2. **Rich volume physics (unwired).** `astraweave-physics/src/environment.rs:308-320` defines `WaterVolume` with AABB, current vector, wave amplitude/length/direction, linear + angular drag. `EnvironmentManager` (environment.rs) exposes `water_current_at` (568-578), `water_drag_at` (555-565), `buoyancy_force_at` (528-539), `is_underwater` (542-552), and Archimedes-style `sphere_submerged_fraction` (372-388). **Nothing in `PhysicsWorld::step` calls any of this.** `add_water_aabb` in `lib.rs:1449` is a no-op stub. Pure dead math.

3. **Gameplay swimming/oxygen/wet-status (unwired).** `astraweave-gameplay/src/water_movement.rs` (1821 lines) defines `WaterMovementMode`, `WetStatus`, `WaterPlayerConfig`, `WaterPlayerState`, `WaterSkills`, and a `WaterMovementHelper::calculate_water_forces` (lines 490-519) that returns a `WaterForces { buoyancy, drag, swim }` Vec3 triple. `WaterPlayerState::update` (line 206) correctly orchestrates oxygen drain + wet-status transitions — but accepts `submersion: f32` as an externally-supplied scalar and **nothing in the workspace calls it**. Grep for `WaterPlayerState::update` outside the file's own tests returns zero hits. Grep for `rapier|rigid_body|apply_force|apply_impulse` inside water_movement.rs returns zero hits — no physics integration.

4. **`astraweave-fluids` SPH crate (unwired).** 3810 lines. Declares 33 modules (PBD, SPH, caustics, foam, waterfall). Only consumer workspace-wide is `examples/fluids_demo`. `astraweave-render` and `astraweave-physics` have zero references to it. Architecturally isolated research crate.

### 3.3 Configuration and authoring surface

**Enable/disable.**
- Runtime `bool` field `water_enabled` at engine_adapter.rs:633, default `false` at line 790.
- No compile-time feature flag (astraweave-render/Cargo.toml and tools/aw_editor/Cargo.toml have no `water` feature; `WaterRenderer` is unconditionally compiled; lib.rs:148-149 exports it unconditionally).
- No editor toggle UI (no checkbox/button in toolbar.rs, widget.rs, or any panel).
- **Biome-string gated**: main.rs:3968-3974 per-frame runs `matches!(biome, "swamp" | "beach" | "river")` to compute the boolean, then calls `set_water_enabled`.

**Sea level.**
- Hardcoded `y = 2.0` at water.rs:232.
- `set_water_level` no-op at water.rs:271-273.
- UI plumbing exists end-to-end (`terrain_panel.rs:428-657` → tab_viewer → widget.rs:2628 → renderer.rs:1337 → water.rs:271) but terminates in the no-op. Vestigial.

**User-facing parameters (effective).**
- `water_color_deep`, `water_color_shallow`, `foam_color` via `set_water_colors` (water.rs:279). Called from engine_adapter.rs:3864 using hardcoded `WaterStyle` presets — but the editor-side caller (renderer.rs:1345) always passes `WaterStyle::Ocean`, so the four `WaterStyle` variants (Ocean | River | Lake | Swamp at types.rs:123-129) collapse to one in practice.
- `rain_intensity` via `set_rain_intensity` (water.rs:289). **Not called from the editor**; default 0.0.
- `foam_threshold`, `ripple_scale`, `ripple_strength` — no setters, baked defaults (0.6, 4.0, 0.15).

**Biome-driven coupling that exists but is unwired.**
- `astraweave-render/src/scene_environment.rs:909-988` defines `BiomeVisuals.water_deep/shallow/foam` with interpolation; tests verify this. Never reaches `WaterRenderer::set_water_colors` from the editor.

**World-wizard authoring surface (separate concept).**
- `tools/aw_editor/src/panels/world_wizard.rs:187-367` defines `WaterBodyPreset` (CalmLake, MountainStream, etc.) + `water_color`, `auto_detect_water_bodies`, `waterfall_height_threshold`. This is procedural-filler authoring for fluid/particle data — no bridge to `WaterRenderer`.

---

## 4. Quality findings

### 4.1 Wave appearance — root cause of "riptide grid"

**The live engine shader is `astraweave-render/src/shaders/water.wgsl`** (223 lines, loaded via `include_str!` at water.rs:100). The other three candidate files are stale:
- `assets/shaders/water_surface.wgsl` (382 lines) — not referenced by any `include_str!`; mentioned only in docs. Dead asset.
- `examples/unified_showcase/src/water.wgsl` — referenced only in `main_backup*.rs`/`main_temp.rs` refactor leftovers; engine path does not use it.
- `examples/fluids_demo/src/ocean.wgsl` — demo-local only.

**Wave parameter table (water.wgsl:149-162):**

| # | Amplitude | Frequency | Wavelength (world u) | Speed | Direction (normalised) | Heading | Steepness |
|---|-----------|-----------|----------------------|-------|------------------------|---------|-----------|
| 1 | 0.80      | 0.15      | 41.9                 | 2.0   | (0.958, 0.287)         | 16.7°   | 0.50      |
| 2 | 0.50      | 0.25      | 25.1                 | 2.5   | (−0.447, 0.894)        | 116.6°  | 0.40      |
| 3 | 0.25      | 0.50      | 12.6                 | 3.5   | (0.707, −0.707)        | −45.0°  | 0.30      |
| 4 | 0.10      | 1.00      | 6.28                 | 4.0   | (−0.316, 0.949)        | 108.4°  | 0.20      |

**Finding 4.1.A — directions cluster on one axis (root cause).** Treating Gerstner crests as symmetric about their direction (antiparallel == same line), pairwise axis-angle differences are:

- W1 ↔ W2: 99.9°
- W1 ↔ W3: 61.7°
- W1 ↔ W4: 91.7°
- **W2 ↔ W3: 18.4°** (near-antiparallel = effectively the same crest line, moving opposite ways)
- **W2 ↔ W4: 8.2°** (near-parallel)
- W3 ↔ W4: 26.6°

Three of four waves (W2, W3, W4) sit within a ~26° band of the same axis. For 4 waves on a 180° half-circle, ideal spacing is ~45°. The observed clustering means three wave crests stack along a dominant NW–SE diagonal instead of covering the ocean surface uniformly — this is the "riptide" line the user sees.

**Finding 4.1.B — frequencies are integer-commensurable (root cause).** Ratios: f3/f4 = 1:2, f2/f4 = 1:4, f2/f3 = 1:2. W2, W3, W4 re-phase against each other every ≈25.1 world units (~6.4 mesh quads at 3.9 m/quad spacing), producing a periodic lattice that repeats visibly across the 500 m plane. For organic-looking ocean the frequencies should be non-rational and log-spaced.

**Finding 4.1.C — no detail normal map layer.** `fs_main` (water.wgsl:178-222) contains zero `textureSample` calls. No high-frequency detail normal texture, no flow map, no FFT ocean. With only 4 analytical Gerstner waves, every periodicity in the math is visible — nothing masks the underlying lattice.

**Finding 4.1.D — ripple tile at 0.25 world units.** `rain_ripple_normal` at water.wgsl:49-87 computes `uv = world_xz * ripple_scale` with default `ripple_scale = 4.0` (water.rs:44). `fract(uv)` at line 63 creates a tile that repeats every 1/4 world unit = 25 cm. At 500 m × 500 m, that is 2000 × 2000 identical tiles. Masked when `rain_intensity = 0` (the editor default) so currently invisible, but obvious when rain activates.

**Finding 4.1.E — waves hardcoded in shader source.** Amplitudes/frequencies/directions/speeds/steepnesses are `const`-style literals inline at water.wgsl:149-162. Retuning requires shader edit + recompile; there is no `WaveSpec[N]` storage-buffer uniform.

### 4.2 Flat-plane / volumetric cue inventory

Technique-by-technique state of `fs_main` (water.wgsl:178-222):

| Technique                          | State              | Evidence                                                                                   |
|------------------------------------|--------------------|--------------------------------------------------------------------------------------------|
| Depth-based colour absorption      | **ABSENT**         | `depth_factor` at line 198 uses `wave_height`, not scene depth. No depth sampler bound.    |
| Refraction through surface         | **ABSENT**         | No refraction texture sampled.                                                             |
| Underwater fog / turbidity         | **ABSENT**         | No Beer–Lambert integration.                                                               |
| Caustics on seafloor               | **ABSENT**         | No caustic pass / projection.                                                              |
| Fresnel reflection/refraction mix  | **PARTIAL**        | Fresnel computed at line 195 but blends only between hardcoded `sky_color` and water colour — never reaches a refracted scene sample. |
| Screen-space reflections           | **ABSENT**         | No prior-frame colour sampler.                                                             |
| Planar reflections                 | **ABSENT**         | No mirror camera.                                                                          |
| Environment/sky reflection         | **DISABLED / FAKE**| Line 202: `let sky_color = vec3<f32>(0.6, 0.75, 0.95);` — hardcoded constant, not IBL/skybox. Desyncs from daylight, weather, ToD. |
| Sun specular                       | **PARTIAL / DESYNCED** | Line 206: `let sun_dir = normalize(vec3<f32>(0.5, 0.8, 0.3));` — hardcoded direction, not the scene sun. Highlight always SE-up. |
| Foam texture                       | **ABSENT**         | Line 212 uses analytical `smoothstep` on `wave_height`. No foam texture, no flow.          |

**Finding 4.2.A — root cause of "painted membrane" reading.** Four of the five volume cues the eye uses (depth tint, refraction, SSR/planar reflection, caustics) are entirely absent. The surface reads as a lit membrane with two colour constants interpolated by Fresnel, plus a specular pow lobe. Wave geometry alone is insufficient to convey ocean depth — depth reads from optical properties under the surface.

**Finding 4.2.B — quality gaps on what IS there.**
- Fresnel exponent 3.0 (line 195) over-reflects at grazing, under-reflects head-on; Schlick for water with F0≈0.02 is closer to exponent 5.
- `depth_factor = clamp(wave_height*2+0.5, 0, 1)` (line 198) fakes "shallow vs deep" from crest height — on a calm day it collapses to a constant 0.5 blend.
- `alpha = mix(0.85, 0.95, fresnel)` (line 219) is near-opaque — combined with no refraction, you literally cannot see into the water, reinforcing the painted-plastic impression.

### 4.3 Physics completeness table

| # | Category                                   | Verdict  | Primary cites                                                                                         | Gap                                                                                          |
|---|--------------------------------------------|----------|-------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------|
| 1 | Water flow / current pushes bodies          | PARTIAL  | environment.rs:315, 568-578                                                                            | Field exists in `WaterVolume`, no force application reaches Rapier                            |
| 2 | Buoyancy (Archimedes) on Rapier bodies      | PARTIAL  | lib.rs:1418-1447 (wired, plane), environment.rs:364-388 (unwired, Archimedes)                          | Simple below/above-plane buoyancy works; rich submerged-fraction volume code is dead          |
| 3 | Splash / wake / displacement                | ABSENT   | — (grep returns no matches)                                                                            | Zero implementation; only a `splash_dash` cooldown flag at water_movement.rs:413              |
| 4 | Drag / viscosity underwater                 | PARTIAL  | lib.rs:1431-1436 (wired, linear), environment.rs:555-565, water_movement.rs:506 (unwired quadratic)   | Only linear drag reaches Rapier                                                               |
| 5 | Player swimming — physics layer             | ABSENT   | water_movement.rs has no rapier/add_force refs                                                         | `WaterForces` struct computed, never applied                                                   |
| 5'| Player swimming — gameplay layer            | FULL     | water_movement.rs:12-464                                                                              | Modes, multipliers, skills, config complete                                                   |
| 6 | `astraweave-fluids` SPH integration         | ABSENT   | Cargo.toml grep (only `fluids_demo`)                                                                   | Architecturally isolated research crate                                                       |
| 7 | Character controller water awareness        | ABSENT   | lib.rs:1206-1210 inline kinematic capsule                                                              | No water branch, no sensor/trigger, no `WaterVolume` component                                 |
| 8 | Submersion detection                        | PARTIAL  | lib.rs:1426 (plane check), environment.rs:542-552 (volume check, uncalled), water_movement.rs:206 (external scalar) | Plane check works for Rapier; gameplay expects externally-supplied scalar, no provider exists |
| 9 | Oxygen / drowning                           | PARTIAL  | water_movement.rs:289-312                                                                             | Correct driver; no caller invokes `WaterPlayerState::update`                                   |
| 10| Wet status / dry-off                        | PARTIAL  | water_movement.rs:247-287                                                                             | Correct transitions; no ECS system supplies submersion input                                   |

**Net:** A working plane-buoyancy hookup on Rapier dynamic bodies via `PhysicsWorld.water_level` and `add_buoyancy`. Everything else is disconnected math, config, or tick functions with no runtime caller.

### 4.4 Performance

Static-analysis estimates. Frame-time measurement requires either (a) wiring a runtime water enable/disable toggle into the editor (out of scope — no code changes) or (b) running two separate editor sessions with and without `terrain_primary_biome = "beach"` to trigger the bias gate, which is confounded because the editor never calls `update_water` so water rendering is garbage whether enabled or not. Neither practical option produces a clean delta on the verification hardware, so estimates follow.

**Buffer footprint.**
- Vertex buffer: 16,641 verts × 20 bytes (12 B position + 8 B UV) ≈ **325 KB** (water.rs:189).
- Index buffer: 98,304 indices × 4 bytes (u32) ≈ **384 KB**.
- Uniform buffer: **144 bytes** (water.rs:108 + struct layout at water.rs:13-29).
- **Total GPU memory: ~709 KB** per installed `WaterRenderer`. Negligible.

**Draw call count: 1 indexed draw per frame** (water.rs:308).

**Vertex shader cost.** Per vertex: 4× `gerstner_wave` + 4× `gerstner_normal` (each call: 1 `normalize`, 1 `dot`, 1 `sin`, 1 `cos`, MADs) + 1 mat4×vec4. ≈120 ALU + 16 transcendentals per vertex pre-CSE (compilers should CSE the shared `phase` between _wave/_normal pairs → effectively 4 sin + 4 cos per vertex). 16,641 vertices × ~8 transcendentals ≈ 133 k transcendentals/frame vertex-side. **Estimate <0.1 ms** on a 1660 Ti. Low.

**Fragment shader base cost (dry, rain_intensity = 0).** Per pixel: Fresnel (1 `pow`), two colour mixes, specular `pow(·, 128.0)`, foam `smoothstep`. ≈30 ALU + 2 `pow` calls. At 1080p fullscreen water coverage (~2 M pixels) ≈ 60 MALU + 4 M transcendentals. **Estimate <0.5 ms.** Low.

**Fragment shader cost with rain active (`rain_intensity > 0`).** `rain_ripple_normal` at water.wgsl:49-87 evaluates `ripple_ring` **9 times per pixel** (3 layers × 3 samples for finite-difference normals), plus 6 outer `fract(uv+offset)` ops. Each `ripple_ring` does 1 `length`, 1 `sin`, 1 `exp`, 1 `fract`, 1 `dot`, 1 `smoothstep` ≈ 10 ALU + 3 transcendentals. Net **≈27 transcendentals per pixel** on top of the base. At 1080p ≈ 56 M transcendentals/frame just for ripples. **Estimate 2–4 ms on midrange GPUs** when rain is active — this is the hot path. Finite-difference normals are the dominant cost; an analytical gradient derived from the same `phase` + `cos` would be ~3× cheaper.

**Overdraw.** `cull_mode: None` (water.rs:170, with comment `// DEBUG: Render both sides`) + `ALPHA_BLENDING` means both faces run the full fragment shader for every triangle. 2× fragment cost at all water pixels, plus incorrect blend ordering across the two faces; masked by near-opaque alpha (0.85–0.95) so the visible effect is minor.

**LOD / culling.** None. Fixed 128×128 grid over 500×500 world units = ~3.9 m per quad. W4's wavelength (6.28 u) ≈ 1.6 quads — aliasing-prone at the horizon. No frustum or distance culling: triangles off-camera still run vertex shader.

**Per-frame cost (editor, current state, non-animated):** ≤1 ms at 1080p on the reference hardware class, from vertex shader + base fragment pass. Rain ripple path would push to 3–5 ms if activated.

---

## 5. Incidental findings

1. **Editor update path is severed.** `EngineRenderAdapter::update_water` (engine_adapter.rs:3872-3877) has zero callers inside `tools/aw_editor`. When `set_water_enabled(true)` is invoked, the `WaterRenderer` is installed with `WaterUniforms::default()` — view_proj = identity, camera_pos = [0, 5, -10], time = 0 — and these are never overwritten. The water plane renders into HDR with garbage clip positions. This is the dominant visible bug, not a subtle one; it is latent only because the water-enable gate is off by default.

2. **Format mismatch risk in editor water pipeline.** engine_adapter.rs:3834 passes `self.renderer.surface_format()` (swapchain format, sRGB Bgra/Rgba8) as the water pipeline's color target. The actual attachment is hdr_view (Rgba16Float). `examples/hello_companion` and `examples/veilweaver_demo` correctly use `renderer.hdr_format()`. Wgpu may currently accept the mismatch on hardware where the formats happen to be compatible; it is not correct.

3. **`cull_mode: None` commented as DEBUG.** water.rs:170: `cull_mode: None, // DEBUG: Render both sides`. The debug flag ships to production.

4. **`set_water_level` is a lying stub.** water.rs:271-273. The body comment says "already at y=0 in mesh" but the mesh is at y=2.0. The parameter `_level` is discarded. No Y uniform exists. End-to-end UI plumbing (terrain_panel → tab_viewer → widget → renderer → water) exists only to terminate here. Misleading to future readers.

5. **`WaterStyle` enum effectively dead.** types.rs:123-129 defines four variants (Ocean | River | Lake | Swamp); engine_adapter.rs:3841-3862 maps each to a color preset; but the editor call path at renderer.rs:1345 always passes `WaterStyle::Ocean`. River/Lake/Swamp presets are unreachable from the editor.

6. **BiomeVisuals→WaterRenderer bridge missing.** scene_environment.rs:909-988 defines and tests `BiomeVisuals.water_deep/shallow/foam` with per-biome values and interpolation — but the editor never pushes these into `WaterRenderer::set_water_colors`. The colour data exists on both sides, disconnected.

7. **Three stale water shader files.** `assets/shaders/water_surface.wgsl` (382 lines), `examples/unified_showcase/src/water.wgsl` (referenced only by `main_backup*.rs`), `examples/fluids_demo/src/ocean.wgsl` (demo-local) — none used by the engine water pipeline. The docs `SHADER_AUDIT_REPORT.md` and `WATER_SYSTEM_PHASE_4_COMPLETE.md` still reference `water_surface.wgsl` as if live.

8. **Gameplay water types are driver-complete but unreached.** `WaterPlayerState::update`, `WaterMovementHelper::calculate_water_forces`, `WetStatus` transitions — all correctly implemented. Zero callers in the workspace outside the file's own tests. 1821 lines of playable mechanics sit dark.

9. **`astraweave-fluids` is a research island.** 3810 lines, 33 modules, zero consumers in production crates — only `examples/fluids_demo`. Not wired to water rendering or physics.

10. **Simple plane buoyancy defaults off.** `PhysicsWorld.water_level` default is `f32::NEG_INFINITY` (lib.rs:619). Even the one working physics path does nothing unless the caller sets a finite `water_level`.

11. **Environment.rs water code imports but has no instantiators.** `WaterVolume`, `EnvironmentManager::add_water_volume` are defined; grep for construction sites returns only this file's own tests.

---

## 6. Architecture vs. quality separation

The following classification informs future planning without making recommendations.

### Rendering

- **Architecture-sound:** Single forward-pass integration model (WaterRenderer slot on `Renderer`, drawn into hdr_view in main pass) matches the engine's overall design. Alpha blend + LessEqual depth is a reasonable forward water approach. The `set_water_renderer` / `clear_water_renderer` / `update_water` API surface is cohesive.
- **Architecture-questionable:** Single-plane 500×500 monolithic mesh (no tiling, no LOD, no infinite-ocean camera-following); single bind group with only a UBO (no scene-env / sun / reflection / depth inputs); sea level baked into vertex buffer (all Y-level plumbing is vestigial); water drawn inside the main pass rather than its own single-layer water pass (closes off AAA techniques like separate depth-only prepass for caustics, or SSR using main-pass colour).
- **Architecture-broken:** Editor never calls `update_water` (engine_adapter.rs:3872 has zero callers in tools/aw_editor — the data flow is severed at the editor seam); format mismatch between pipeline target (`surface_format`) and actual attachment (`hdr_view Rgba16Float`) in editor path; `WaterStyle` enum collapsed to one variant by editor hardcoding; `BiomeVisuals.water_*` data disconnected from `WaterRenderer::set_water_colors`.

**Rendering quality, independent of architecture:** Gerstner wave parameters are authored poorly (directions cluster on one axis, frequencies form 1:2:4 integer ratios — produces the riptide grid); hardcoded sky_color and sun_dir constants in shader desync water from scene; no detail normal map; missing depth absorption / refraction / SSR / caustics / Fresnel reference colour all contribute to the painted-membrane look; `cull_mode: None` is a debug flag shipped to production; Fresnel exponent 3.0 is physically off for water.

### Physics

- **Architecture-sound:** Existence of both a simple plane model (`PhysicsWorld.water_level`) for cheap buoyancy and a richer volume model (`environment::WaterVolume`) for Archimedes submerged-fraction is a reasonable layered design *in principle*.
- **Architecture-questionable:** Three water physics surfaces (`physics::lib.rs`, `physics::environment.rs`, `gameplay::water_movement.rs`) share zero types and have no bridge APIs; the gameplay layer expects an externally-supplied submersion scalar but the editor/runtime has no code that computes this from a water body.
- **Architecture-broken:** `astraweave-fluids` is an isolated research crate with no consumer in production; `environment::WaterVolume` has no instantiators outside tests; `WaterPlayerState::update` has no callers; the character controller has no water path at all.

**Physics quality, independent of architecture:** Simple plane buoyancy uses `volume * ρ * g` (not submerged-fraction Archimedes) — works but crude; drag is linear only (no quadratic); no turbulence/viscosity term; all three water physics surfaces implement overlapping but incompatible math.

### Configuration and authoring

- **Architecture-sound:** None of it — there is no coherent authoring surface.
- **Architecture-questionable:** Water-enable is gated on a single-biome string match (`terrain_primary_biome in {swamp, beach, river}`) rather than per-project/per-scene water body definitions.
- **Architecture-broken:** No dedicated water panel; no editor toggle; `terrain_panel.water_level` UI wired to a no-op; `WaterStyle` enum collapsed to one variant; `BiomeVisuals.water_*` disconnected; `world_wizard::WaterBodyPreset` lives in a separate authoring island with no bridge.

**Config quality:** Consistent with the "broken" architecture state — users cannot usefully tune water from the editor UI alone.

---

## 7. Appendix — stopped-short investigations

No compelling side-quests were started during this audit. The report above is the deliverable.

---

**End of audit. No recommendations follow — decisions about what to do next are downstream of this document.**
