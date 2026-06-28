---
layout: default
title: Rendering Subsystem
---

# Rendering (astraweave-render)

AstraWeave's renderer is built on **wgpu 25.0.2**, providing cross-platform GPU rendering with Vulkan, DX12, and Metal backends. The crate contains 62+ source files covering PBR materials, multiple rendering paths, post-processing, mesh optimization, animation, and environment systems.

## Feature Matrix

| Feature | Status | Module |
|---------|--------|--------|
| PBR materials (Cook-Torrance BRDF) | ✅ | `material.rs`, `material_extended.rs` |
| Extended materials (clearcoat, anisotropy, sheen, subsurface, transmission) | ✅ | `material_extended.rs` |
| TOML → GPU material pipeline | ✅ | `material_loader.rs`, `biome_material.rs` |
| Cascaded shadow maps (4 cascades, 2048px) | ✅ | `shadow_csm.rs` |
| Clustered forward lighting | ✅ | `clustered_forward.rs` |
| Clustered megalights | ✅ | `clustered_megalights.rs` |
| Deferred rendering path with G-Buffer | ✅ | `deferred.rs` |
| Post-processing: Bloom | ✅ | `post.rs` |
| SSAO (multiple quality presets) | ✅ | `ssao.rs` |
| Advanced post-FX: TAA, motion blur, DoF, color grading | ✅ | `advanced_post.rs` |
| GPU skinning (dual bone influence) | ✅ | `skinning_gpu.rs` |
| Skeletal animation (clips, channels, joints) | ✅ | `animation.rs` |
| Vertex compression (37.5% savings) | ✅ | `vertex_compression.rs` |
| LOD generation (quadric error metrics) | ✅ | `lod_generator.rs` |
| Instanced rendering | ✅ | `instancing.rs` |
| Image-based lighting (IBL) | ✅ | `ibl.rs` |
| HDRI catalog with day/night cycle | ✅ | `hdri_catalog.rs` |
| Skybox (procedural + cubemap + equirectangular) | ✅ | `environment.rs` |
| Weather system (7 weather types) | ✅ | `environment.rs`, `weather_system.rs` |
| GPU particle system (compute shader) | ✅ | `gpu_particles.rs` |
| Water rendering | ✅ | `water.rs` |
| Decal system (atlas-based) | ✅ | `decals.rs` |
| Biome transition effects (6 easing functions) | ✅ | `biome_transition.rs` |
| Texture streaming | ✅ | `texture_streaming.rs` |
| GPU memory management | ✅ | `gpu_memory.rs`, `residency.rs` |
| GPU frustum culling | ✅ | `culling.rs`, `culling_node.rs` |
| MSAA | ✅ | `msaa.rs` |
| Transparency management | ✅ | `transparency.rs` |
| Render graph | ✅ | `graph.rs`, `graph_adapter.rs` |
| Nanite-style GPU culling | ✅ | `nanite_gpu_culling.rs` (feature-gated) |
| Mesh loaders: glTF, OBJ | ✅ | `mesh_gltf.rs`, `mesh_obj.rs` (feature-gated) |
| Volumetric fog | 📋 Planned | — |

## Camera System

Camera types live in the `astraweave-camera` crate (separate from `astraweave-render`).
The canonical types are `FreeFly` (the engine's free-fly producer), `RenderView`
(the upload contract the renderer consumes), `Projection` (perspective projection
with the original parameters preserved), and the `CameraProducer` trait.

### Canonical upload contract

Every camera in AstraWeave produces a `RenderView` which the renderer consumes
via a single entry point:

```rust
renderer.update_view(&camera.to_render_view());
```

This is the only camera-upload path on `Renderer`. (The historical
`Renderer::update_camera(&Camera)` and `Renderer::update_camera_matrices(...)`
APIs were removed by the Unified Camera campaign, sub-phase C.3.C.)

### `FreeFly` producer

```rust
pub struct FreeFly {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fovy: f32,    // radians per CAMERA_CONVENTIONS.md §2.1
    pub aspect: f32,
    pub znear: f32,
    pub zfar: f32,
}
```

Methods: `view_matrix()`, `proj_matrix()`, `vp()`, `dir(yaw, pitch) -> Vec3`,
`view_matrix_camera_relative()`, `to_render_view()`,
`to_render_view_camera_relative()`. The last two are the producer-side bridges
that `CameraProducer::to_render_view` and the concrete camera-relative path
provide.

### Adding a new camera

Implement `CameraProducer`:

```rust
use astraweave_camera::{CameraProducer, RenderView, Projection};

impl CameraProducer for MyCamera {
    fn to_render_view(&self) -> RenderView {
        let projection = Projection::perspective(
            self.fovy, self.aspect, self.znear, self.zfar,
        );
        let view = self.compute_view_matrix();
        let view_dir = self.compute_view_direction();
        RenderView::new(view, &projection, self.position, view_dir)
    }
}
```

See `CAMERA_CONVENTIONS.md` in the repository's `docs/current/` directory for
the canonical convention reference (yaw=0 forward direction, FOV semantics,
near/far handling, aspect-ratio guards, coordinate handedness).

### The `FreeFly as Camera` alias pattern

Caller code throughout the workspace currently imports `FreeFly` via a local
alias:

```rust
use astraweave_camera::FreeFly as Camera;
```

This is a deliberate artifact of the Unified Camera campaign (C.3.C). The
canonical name is `FreeFly`; historically the type was named `Camera`. The
campaign renamed the type to its proper home crate but preserved the
historical name as a per-file alias to keep migration diffs small. The alias
appears in roughly 30 caller files (engine examples plus internal tests).

**When writing new code, prefer `FreeFly` directly without the alias:**

```rust
use astraweave_camera::FreeFly;

let camera: FreeFly = FreeFly { /* ... */ };
```

The alias is a migration convenience, not a recommended pattern for new code.

### `CameraController`

`CameraController` (also in `astraweave-camera`) supports orbit and fly modes
with keyboard, mouse, and scroll input:

- `process_keyboard()`, `process_mouse_delta()`, `process_scroll()`
- `toggle_mode()` — switch between FPS and orbit
- `set_orbit_target()` — focus on a world point
- `update_camera(&mut FreeFly, dt)` — apply pending input deltas to the camera state (note: this is the controller's input-application method, distinct from the renderer's `update_view` upload entry point)

### Two-camera architecture

AstraWeave has two production camera producers, each living in the crate
that owns its primary use case:

- **`FreeFly`** — engine-runtime camera. Lives in `astraweave-camera`.
  Implements `CameraProducer`. Used by every example crate, the cinematics
  renderer path, and any application embedding the engine. Free-look
  mouse + WASD navigation pattern; companion `CameraController` handles
  input.
- **`OrbitCamera`** — editor camera. Lives in
  `tools/aw_editor/src/viewport/camera.rs`. Implements `CameraProducer`
  (added in sub-phase C.4). Used exclusively by the editor's viewport.
  Spherical orbit around a focal point, with picking, frustum extraction,
  and screen-space queries built in. Has an editor-specific UI surface
  (smooth zoom animation, bookmark restore, sanitize-on-deserialize) that
  the runtime engine doesn't need.

Both producers converge at the `CameraProducer::to_render_view()`
contract. The renderer consumes `RenderView` exclusively (per
`CAMERA_CONVENTIONS.md` §2.9); it doesn't know which producer created
the view, and the canonical upload contract above is identical for both.

**Why two locations?** OrbitCamera's surface is editor-specific (~15
methods for interactive picking, frustum extraction, sanitize for
deserialization, smooth zoom/focal-point animation, view-bookmark
restore) versus FreeFly's runtime-camera surface (~6 methods for
view/projection/direction). Moving OrbitCamera to `astraweave-camera`
would either pull editor concerns into the producer crate or split
OrbitCamera across crates. Keeping it in the editor crate preserves
crate-boundary purity; the `CameraProducer` trait is the abstraction
that lets producers live anywhere.

**For new editor-only producers** (e.g., a debug-camera mode confined
to editing workflows), the pattern is the same: implement
`CameraProducer` in the editor crate; the trait flows from
`astraweave-camera`; the implementation stays with its concerns.

**For new engine-runtime producers** (Follow, Cinematic, Debug per the
SOTA roadmap): add them to `astraweave-camera` alongside `FreeFly`.

### Cinematics camera consolidation

AstraWeave's cinematics camera state consolidated to a single canonical
type during the Unified Camera campaign's C.7 chapter: `CameraKey`
(in `astraweave-cinematics`).

**The canonical type.** `astraweave_cinematics::CameraKey` carries a
keyframe's timestamp (`t`), world position (`pos`), look-at target
(`look_at`), and field of view (`fov_deg`, degrees). It implements
`lerp` for interpolation between adjacent keyframes and `sanitize` for
boundary hardening (clamps `fov_deg` to [10°, 170°]; resolves a
degenerate `look_at == pos` to canonical +X forward). The cinematics
crate has no `glam` dependency by design — `pos` and `look_at` are
`(f32, f32, f32)` tuples — which keeps it a self-contained data-layer
crate that any other crate can depend on without circular-dependency
risk.

**The upload path.** Cinematics camera state reaches the renderer via
`Renderer::tick_cinematics(dt, &mut camera)`, which advances a loaded
`Timeline` through its `Sequencer`, dispatches `CameraKey` events to
`apply_camera_key` (which sanitizes defensively, then converts each key
into the engine's `FreeFly` camera — `fov_deg` becomes `fovy` in radians
at this producer boundary). `FreeFly` then produces a `RenderView`
through the canonical `CameraProducer` contract described above. So
cinematics flows into the same canonical upload path every camera uses:
`CameraKey → FreeFly → RenderView → Renderer::update_view`. There is no
bespoke cinematics renderer API (per `CAMERA_CONVENTIONS.md` §2.9).

**The consolidation arc.** Before C.7, three parallel cinematics camera
systems coexisted with no conversion functions between them. The chapter
consolidated them per-system, with the strategy fitting each system's
role:

- **Gameplay cutscenes** (`astraweave_gameplay::cutscenes::Cue::CameraTo`):
  evolved their fields from yaw/pitch to look_at + fov_deg (C.7.A), so
  `CutsceneState::tick` now emits canonical `CameraKey` events via the
  `CutsceneTickEvent` enum. The gameplay layer gained an
  `astraweave-cinematics` dependency.
- **The cutscene demo** (`examples/cutscene_render_demo`): rewrote its
  tick loop to use `Renderer::tick_cinematics` (C.7.B), becoming the
  first production caller of the canonical cinematics path.
- **The canonical key**: hardened with `sanitize` at the
  `apply_camera_key` boundary (C.7.D), so the production path is
  defensive against degenerate keys.
- **The editor's keyframe type** (`CameraKeyframe`): retired entirely
  into `CameraKey` (C.7.C). The editor's `CinematicsPanel` now holds
  `Vec<CameraKey>` and operates on the canonical type directly. The
  editor's roll feature dropped (it was UI-state-only with no runtime
  effect).

### Known remaining parallel surface (deferred)

One cinematics-camera surface remains outside the canonical
consolidation as of the C.7 chapter's close: the dev-only "Simple
Cinematics" panel in `astraweave-ui` (`astraweave-ui/src/panels.rs`).

This panel already uses the canonical `CameraKey` type (it loads/saves
`Timeline` JSON and steps a `Sequencer`, displaying events as UI
labels), but it has no renderer connection — it's a developer debugging
tool for inspecting timeline data, not a content-authoring surface
wired to a viewport.

It's deferred to post-campaign cleanup rather than consolidated in C.7
because (a) it already uses canonical types, so it's not a parallel
*type* — it's a parallel *tool*; (b) wiring it to a renderer preview
would be a feature addition, not a consolidation; (c) the campaign's
scope was camera-system consolidation, and this panel's remaining work
is tool-integration, a different concern. This is a deliberate deferral,
not an oversight. The panel's future consolidation (if pursued) would
wire its `Sequencer` output to a preview viewport via
`Renderer::tick_cinematics` — the same path `cutscene_render_demo` now
uses.

## Material System

Materials are defined in TOML and compiled to GPU D2 array textures:

```
assets/materials/<biome>/
├── materials.toml    # Material definitions
└── arrays.toml       # Texture array config
```

### Standard Material (GPU)

```rust
pub struct MaterialGpu {
    // Bitfield: FLAG_HAS_ALBEDO | FLAG_HAS_NORMAL | FLAG_HAS_ORM | FLAG_TRIPLANAR
}
```

### Extended Materials

Advanced PBR features via `MaterialDefinitionExtended`:

| Feature | Flag |
|---------|------|
| Clearcoat | `MATERIAL_FLAG_CLEARCOAT` |
| Anisotropy | `MATERIAL_FLAG_ANISOTROPY` |
| Sheen | `MATERIAL_FLAG_SHEEN` |
| Subsurface scattering | `MATERIAL_FLAG_SUBSURFACE` |
| Transmission | `MATERIAL_FLAG_TRANSMISSION` |

### Material Manager

`MaterialManager` handles GPU resource creation and caching:
- `get_or_create_bind_group_layout()` — cached bind group layout
- `create_bind_group()` — assemble texture arrays into GPU bind groups
- `current_stats()` — `MaterialLoadStats` with `concise_summary()`

**WGSL Bindings (group=1)**:

| Binding | Resource |
|---------|----------|
| 0 | Albedo texture array |
| 1 | Sampler |
| 2 | Normal map array |
| 3 | Linear sampler |
| 4 | MRA (Metallic/Roughness/AO) array |

## Rendering Paths

### Clustered Forward

Default path using clustered light assignment for efficient multi-light rendering:

```rust
let renderer = ClusteredForwardRenderer::new(&device, config);
renderer.update_lights(&lights);
renderer.build_clusters();
```

WGSL shader: `CLUSTERED_LIGHTING_SHADER`

### Deferred

G-Buffer based deferred rendering for scenes with many lights:

```rust
let deferred = DeferredRenderer::new(&device, size);
// Geometry pass fills G-Buffer
let gbuffer = deferred.gbuffer();
// Light pass reads G-Buffer
deferred.light_pass(&lights);
```

### Shadow Mapping (CSM)

4-cascade cascaded shadow maps (2048px resolution per cascade):

```rust
let csm = CsmRenderer::new(&device);
csm.update_cascades(camera, sun_direction);
csm.render_shadow_maps(&scene_objects);
```

Constants: `CASCADE_COUNT=4`, `CASCADE_RESOLUTION=2048`, `DEPTH_BIAS=0.005`

## Post-Processing Pipeline

### Bloom

```rust
let bloom = BloomPipeline::new(&device, bloom_config);
bloom.execute(&render_pass, &scene_texture);
```

4-stage WGSL pipeline: threshold → downsample → upsample → composite.

### SSAO

Screen-space ambient occlusion with quality presets:

```rust
pub enum SsaoQuality {
    Low,     // fewer samples, smaller radius
    Medium,
    High,    // more samples, larger blur kernel
}
```

### Advanced Post-FX

Feature-gated (`advanced-post`) pipeline with TAA, motion blur, depth of field, and color grading:

```rust
pub struct TaaConfig {
    pub enabled: bool,
    pub blend_factor: f32,    // default 0.95
    pub jitter_scale: f32,
}

pub struct DofConfig {
    pub enabled: bool,
    pub focus_distance: f32,  // default 10.0
    pub focus_range: f32,     // default 5.0
    pub bokeh_size: f32,      // default 2.0
}

pub struct MotionBlurConfig {
    pub enabled: bool,
    pub sample_count: u32,    // default 8
    pub strength: f32,
}

pub struct ColorGradingConfig {
    pub enabled: bool,
    pub exposure: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub temperature: f32,
    pub tint: f32,
}
```

## Environment System

### Time of Day

```rust
pub struct TimeOfDay {
    pub current_time: f32,
    pub time_scale: f32,
}
```

Methods: `get_sun_position()`, `get_moon_position()`, `get_light_direction()`, `get_light_color()`, `is_day()`, `is_night()`, `is_twilight()`.

### Sky Rendering

`SkyRenderer` supports three sky modes:
- **Procedural**: Rayleigh/Mie scattering with sun/moon discs and clouds
- **Textured cubemap**: Traditional skybox from 6 textures
- **Equirectangular HDRI**: Panoramic HDR image projection

Integrated with `HdriCatalog` for automatic day/night HDRI selection by `DayPeriod`.

### Weather System

7 weather types with full GPU particle effects:

```rust
pub enum WeatherType {
    Clear, Cloudy, Rain, Storm, Snow, Fog, Sandstorm,
}
```

`WeatherSystem`: `set_weather()`, `current_weather()`, `get_rain_intensity()`, and more.

## GPU Skinning

```rust
pub struct SkinnedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub bone_indices: [u32; 2],  // dual bone influence
    pub bone_weights: [f32; 2],
}
```

Full skeletal animation pipeline: `AnimationClip` → `AnimationState` → `JointPalette` → GPU upload via `JointPaletteManager`. Supports up to `MAX_JOINTS` joints per skeleton.

## Mesh Optimization

| Optimization | Savings | Module |
|--------------|---------|--------|
| Vertex compression | 37.5% memory (32 → 20 bytes) | `vertex_compression.rs` |
| Octahedral normals | 12 → 2 bytes per normal | `vertex_compression.rs` |
| Half-float UVs | 8 → 4 bytes per UV pair | `vertex_compression.rs` |
| LOD generation | 3-5 levels via quadric error metrics | `lod_generator.rs` |
| Instanced rendering | 10–100× fewer draw calls | `instancing.rs` |
| GPU frustum culling | CPU-side + GPU-side culling | `culling.rs` |

### Instance Manager

```rust
let mut manager = InstanceManager::new(&device);
manager.add_instance(mesh_id, transform);
manager.update_buffers(&device, &queue);
// Batch rendered with a single draw call per mesh type
```

## Biome Transitions

Smooth world-space transitions between biome visual styles:

```rust
pub enum EasingFunction {
    Linear, SmoothStep, SmootherStep, EaseIn, EaseOut, EaseInOut,
}

pub struct BiomeVisuals {
    // Per-biome: fog color/density, sky tint, water color, cloud coverage
}

pub struct TransitionConfig {
    pub easing: EasingFunction,
    pub duration: f32,
}
```

## GPU Particle System

Double-buffered compute shader particle system:

```rust
pub struct GpuParticle {
    // 32 bytes, Pod + Zeroable for GPU mapping
}

pub struct EmitterParams {
    pub emission_rate: f32,
    pub gravity: f32,
    pub lifetime: f32,
}

let particles = GpuParticleSystem::new(&device, max_particles);
```

Uses ping-pong buffers for GPU-side simulation.

## Frame Budget

At 1,000 entities (`profiling_demo`, measured 2026-06-27):

- **Frame time**: 0.71 ms (default mimalloc) / 0.97 ms (system allocator)
- **FPS**: ~1,410 (mimalloc) / ~1,036 (system)
- **Budget headroom**: ~94–96% vs the 16.67 ms (60 FPS) budget. (The earlier "2.70 ms / 370 FPS" was the Week-8 *target*, not a measurement.)

## Test Coverage

- **806 tests** (lib + integration)
- Mutation testing campaigns for camera, weather, easing, and environment subsystems

[← Back to Home](index.html) · [Architecture](architecture.html) · [Physics](physics.html)
