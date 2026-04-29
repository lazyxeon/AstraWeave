# AstraWeave Architecture Map

> **Generated**: 2026-04-04 | **Version**: 0.5.0 | **Rust**: 1.89.0
> Living document — used by all agents as the primary architectural reference.
> **0.5.0 update**: Reflects Fix 27 Unified Pipeline Campaign — EntityRenderer deleted, astraweave-render is now non-optional in aw_editor.

---

## 1. Workspace Overview

- **Total workspace members**: 140 (listed in root Cargo.toml)
- **Production crates**: ~49
- **Examples**: ~45
- **Tools**: 10 (`aw_editor`, `aw_asset_cli`, `astraweave-assets`, `aw_debug`, `aw_build`, `aw_texture_gen`, `aw_headless`, `ollama_probe`, `asset_signing`, `aw_release`, `aw_demo_builder`, `aw_save_cli`)
- **Networking sub-crates**: 3 (`aw-net-proto`, `aw-net-server`, `aw-net-client`)
- **Persistence**: `aw-save`, `aw_save_cli`, `astraweave-persistence-ecs`, `astraweave-persistence-player`
- **Resolver**: 2
- **Build profiles**: dev (opt-level 0, deps opt-level 2), release-fast (no LTO), release (thin LTO)

---

## 2. Crate Dependency Graph

### 2.1 Domain Groupings

**Core (foundation — minimal deps)**

| Crate | Workspace Deps |
|-------|---------------|
| `astraweave-math` | _(none)_ |
| `astraweave-ecs` | profiling |
| `astraweave-core` | ecs, behavior |
| `astraweave-sdk` | core |

**AI (orchestration, planning, LLM)**

| Crate | Workspace Deps |
|-------|---------------|
| `astraweave-behavior` | ecs, profiling |
| `astraweave-ai` | core, ecs, nav, behavior (opt), observability (opt), llm (opt), profiling |
| `astraweave-llm` | core, observability |
| `astraweave-director` | core, llm, rag, context, prompts |
| `astraweave-memory` | llm, embeddings, rag |
| `astraweave-coordination` | llm, rag, context, prompts |
| `astraweave-npc` | physics, audio, gameplay (**NOT** ai or behavior — architectural surprise) |
| `astraweave-dialogue` | llm, embeddings, context, prompts, rag, persona |
| `astraweave-embeddings` | _(LLM foundation)_ |
| `astraweave-context` | _(LLM foundation)_ |
| `astraweave-prompts` | _(LLM foundation)_ |
| `astraweave-rag` | _(LLM foundation)_ |
| `astraweave-persona` | _(LLM foundation)_ |

**Rendering & Assets**

| Crate | Workspace Deps |
|-------|---------------|
| `astraweave-materials` | _(none)_ |
| `astraweave-cinematics` | _(none)_ |
| `astraweave-render` | core, materials, cinematics, terrain, profiling, asset, **aw_asset_cli** (tool — unusual direction) |
| `astraweave-asset` | blend |
| `astraweave-asset-pipeline` | _(none)_ |
| `astraweave-blend` | _(in crates/)_ |

**Physics & World**

| Crate | Workspace Deps |
|-------|---------------|
| `astraweave-nav` | _(none)_ |
| `astraweave-physics` | profiling, ecs (opt), scene (opt) |
| `astraweave-scene` | ecs, asset |
| `astraweave-terrain` | core, **gameplay** (reverse dep — world-gen imports biome types) |
| `astraweave-fluids` | _(none)_ |

**Gameplay**

| Crate | Workspace Deps |
|-------|---------------|
| `astraweave-input` | _(none)_ |
| `astraweave-pcg` | _(none)_ |
| `astraweave-gameplay` | core, physics, nav, ecs, input, scene |
| `astraweave-quests` | llm, rag, context, prompts |
| `astraweave-weaving` | pcg |
| `astraweave-audio` | gameplay |
| `astraweave-ui` | input, gameplay, cinematics |

**Networking**

| Crate | Workspace Deps |
|-------|---------------|
| `astraweave-net` | core |
| `astraweave-net-ecs` | aw-net-proto, ecs, core |
| `astraweave-persistence-ecs` | aw-save, ecs, core, memory |

**Tools**

| Crate | Workspace Deps |
|-------|---------------|
| `aw_editor` | astract, core, ecs, profiling, **render (REQUIRED, non-optional)**, author, asset, audio, behavior, dialogue, quests, nav, observability, physics, security, terrain, asset-pipeline |

### 2.2 Leaf Crates (zero workspace deps)
`math`, `nav`, `materials`, `cinematics`, `asset-pipeline`, `input`, `pcg`, `fluids`

### 2.3 Architectural Anomalies

1. **`terrain` → `gameplay`** (reverse dependency): World-gen crate imports gameplay biome types. Should flow the other way. Blast radius: changing gameplay biome types breaks terrain generation.
2. **`render` → `aw_asset_cli`** (production → tool): A runtime rendering crate depending on a CLI tool crate. Unusual dependency direction.
3. **`npc` does NOT depend on `ai` or `behavior`**: NPC crate depends on physics/audio/gameplay instead. NPC behavior logic is decoupled from the AI orchestration layer.
4. **`core` → `behavior`**: Core foundation depends on behavior trees (behavioral is part of the core abstraction).
5. **`aw_editor` → `render` is now REQUIRED** (Fix 27): Previously optional (`#[cfg(feature = "astraweave-render")]`), now always compiled. ~30 `#[cfg]` guards removed. Build time increased ~45s but editor always has engine-grade PBR rendering.

---

## 3. Public API Surface

### Core

**astraweave-ecs**
- Types: `World`, `Entity`, `EntityAllocator`, `CommandBuffer`, `Events`, `EventReader`, `Query`, `Query2`, `Query2Mut`, `Rng`, `TypeRegistry`
- Traits: `Component`, `Resource`, `SystemParam`
- Stages: `PRE_SIMULATION`, `PERCEPTION`, `SIMULATION`, `SYNC`, `AI_PLANNING`, `PHYSICS`, `POST_SIMULATION`, `PRESENTATION` — executed deterministically on a single thread per tick.
- Modules: archetype, blob_vec, command_buffer, component_meta, entity_allocator, events, rng, sparse_set, type_registry
- **Note (2026-04-18)**: `ParallelSchedule` was removed from this crate. See `docs/audits/parallel_schedule_removal_2026-04-18.md`.

**astraweave-math**
- SIMD batch ops: Vec3/Vec4 dot/cross/normalize, Mat4 multiply/transpose, Quat multiply/normalize/slerp
- Module: simd_vec, simd_mat, simd_quat, simd_movement
- Function: `enable_flush_to_zero()`

**astraweave-sdk**
- Types: `Version`, `AWVersion`, `SdkError`
- Trait: `GameAdapter`
- C ABI: `aw_version()`, `aw_world_create()`, `aw_world_destroy()`, `aw_world_tick()`, `aw_world_snapshot_json()`, `aw_world_submit_intent_json()`

**astraweave-core**
- Types: `WorldSnapshot`, `CompanionState`, `PlayerState`, `EnemyState`, `Poi`, `PlanIntent`, `ActionStep`
- Bridges ECS ↔ AI via WorldSnapshot serialization

### AI

**astraweave-ai**
- Types: `AIArbiter`, `AIControlMode`, `LlmExecutor`, `AsyncTask`, `VeilweaverCompanionOrchestrator`
- Trait: `Orchestrator`
- Function: `build_app_with_ai()`
- Modules: core_loop, orchestrator, tool_sandbox, arbiter, goap (feature-gated)

**astraweave-behavior**
- Enum: `BehaviorNode` { Sequence, Selector, Action, Condition, Decorator, Parallel }
- Modules: ecs, goap, goap_cache, interner

### Rendering

**astraweave-render**
- Types: `Renderer`, `Camera`, `CameraController`, `Texture`, `Vertex`, `Mesh`, `MaterialManager`, `Skeleton`, `AnimationClip`, `JointPalette`, `GBuffer`
- 60+ modules: clustered lighting, MegaLights, CSM shadows, bloom, TAA, SSAO, SSGI, SSR, volumetric fog, god rays, atmosphere, auto-exposure, particle system, terrain materials, weather
- Features: PBR, deferred, forward+, lumen GI, GPU particles, decals, water
- Key methods used by editor: `Renderer::new_from_device()`, `draw_into(target, encoder)`, `update_camera()`, `add_model()`, `clear_model()`, `has_model()`, `update_instances()`, `create_mesh_from_cpu_mesh()`, `create_mesh_from_arrays()`, `create_mesh_from_full_arrays()`, `set_sky_config()`, `set_weather()`, `set_water_renderer()`, `scene_environment_mut()`, `time_of_day_mut()`, `shadows_enabled()`, `set_shadows_enabled()`
- Key re-exports: `WeatherKind` (via `effects::WeatherKind`), `SkyConfig`, `WaterRenderer`, `TerrainVertex`, `Instance`

**astraweave-materials**
- Enum: `Node` { Texture2D, Constant3, Constant1, Multiply, Add, MetallicRoughness, Clearcoat, Anisotropy, Transmission, NormalMap }
- Material graph node system (Phase 2 foundation)

### Physics

**astraweave-physics**
- Types: `PhysicsWorld`, `CharacterController`, `SpatialHash`, `ProjectileManager`, `GravityManager`, `Ragdoll`, `Vehicle`, `ClothManager`, `DestructionManager`, `EnvironmentManager`
- Type used by editor viewport: `DebugLine` (debug geometry for physics wireframes, brush cursor, zone overlays)
- Wraps Rapier3D 0.22, re-exports all Rapier types
- Features: async-physics, profiling, ecs

**astraweave-fluids**
- 40+ modules: SPH, boundary, caustics, foam, god_rays, LOD, multi-phase, turbulence, underwater, waterfall
- Types: `WaterBuildingManager`, `WaterEffectsManager`, `CausticsSystem`, `WaterQualityPreset`

### World & Scene

**astraweave-scene**
- Types: `Transform`, `Node`, `Scene`, `SceneError`
- Modules: gpu_resource_manager, partitioned_scene, streaming, world_partition

**astraweave-terrain**
- Types: `Biome`, `BiomeType`, `ChunkManager`, `TerrainChunk`, `Heightmap`, `VoxelGrid`, `WorldConfig`, `ZoneRegistry`
- Climate-field architecture (Phase 1.6-F D.1): `ClimateMap`, `ClimateSample` (real-world units: temperature_c, moisture_mm, continentalness), `WorldArchetype` (climate envelope: means + variances + latitude_temperature_drop_c), `ClimateMap::sample(x, z, elevation) → ClimateSample` per-vertex API.
- Whittaker biome lookup (Phase 1.6-F D.2): `BiomeId` enum with 19 fixed variants (11 terrestrial Whittaker + 5 aquatic + 3 elevation overlays); `lookup_biome(temp_c, moisture_mm, elevation_m) → BiomeId` deterministic four-layer-ordered classifier.
- Per-biome parameter system (Phase 1.6-F D.3): `BiomeParameters` (mountains_amplitude, ridge_strength, runevision_config, erosion_preset, scatter_density, scatter_species_set, surface_color_palette); `BiomeParameters::for_biome(BiomeId)` total over all 19 variants. Replaces legacy `BiomeNoisePreset` (removed D.3c).
- Scattered-convolution biome blending (Phase 1.6-F D.4, NoisePosti.ng-style): `BiomeParamBlendConfig`, `BlendedBiomeParams`, `blend_biome_parameters()` — N jittered samples per vertex with distance-weighted parameter blending and dominant-biome assignment, position-quantized for shared-edge invariance.
- World archetype catalog (Phase 1.6-F D.5): `WorldArchetypeId` enum (6 variants: Continental Temperate, Equatorial Tropical, Boreal/Subarctic, Mediterranean, Desert, Custom); `world_archetypes::all()`, `display_name()`, `description()`, `default_archetype()` for editor consumption.
- 30+ modules: erosion (`advanced_erosion`, `runevision_erosion`), marching cubes, LOD, noise (`noise_gen`, `perlin_gradient`), scatter, climate (`climate`), biome lookup (`biome_lookup`), per-biome parameters (`biome_parameters`), biome param blending (`biome_param_blending`), legacy splat blending (`biome_blending`), world archetypes (`world_archetypes`), blueprint zones.
- Phase 1.6-F (Terrain Generation Quality Campaign) CLOSED VIA ARCHITECTURAL PIVOT 2026-04-29; D.1-D.5 deliverables preserved as within-region machinery for new Phase 1.X (Regional Archetype Variation) campaign at `docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md`.

**astraweave-nav**
- Types: `NavMesh`, `NavTri`, `Triangle`, `Aabb`
- A* pathfinding, navmesh baking, dirty region tracking

### Gameplay

**astraweave-gameplay**
- 20+ modules: combat, combat_physics, crafting, dialogue, quests, items, stats, biome, harvesting, weaving, weave_portals

**astraweave-audio**
- Types: `AudioEngine`, `DialoguePlayer`, `VoiceBank`, `EmitterId`, `ListenerPose`
- 4-bus mixer (master, music, SFX, voice), 3D spatial audio, TTS adapter

**astraweave-ui**
- Types: `HudManager`, `MenuManager`, `AccessibilitySettings`, `GamepadManager`
- Modules: accessibility, gamepad, hud, layer, menu, panels, persistence, state

---

## 4. Integration Seams

### 4.1 Shared Types (High Blast Radius)

| Type | Defined In | Consumed By | Risk |
|------|-----------|-------------|------|
| `WorldSnapshot` | astraweave-core | ai, llm, director, memory, examples | **CRITICAL** — field names hard-coded in LLM prompts; renaming breaks both Rust AND AI behavior |
| `CompanionState` | astraweave-core | ai, llm, examples | HIGH — LLM prompt dependency |
| `PlanIntent` / `ActionStep` | astraweave-core | ai, tool_sandbox, examples | HIGH |
| `Entity` | astraweave-ecs | core, ai, scene, gameplay, physics, net-ecs, persistence-ecs, editor | HIGH |
| `World` | astraweave-ecs | nearly everything | CRITICAL |
| `BehaviorNode` | astraweave-behavior | core, ai, editor | MEDIUM |
| `BiomeType` | astraweave-gameplay | terrain, render, scatter | HIGH — terrain reverse-dep |
| `Transform` / `Node` | astraweave-scene | render, editor, physics | HIGH |
| `NavMesh` | astraweave-nav | ai, gameplay, terrain | MEDIUM |
| `PhysicsWorld` | astraweave-physics | gameplay, npc, editor | MEDIUM |

### 4.2 Cross-Crate Trait Implementations

| Trait | Defined In | Implemented By |
|-------|-----------|----------------|
| `Component` (auto) | ecs | Any `T: 'static + Send + Sync` |
| `Resource` (auto) | ecs | Any `T: 'static + Send + Sync` |
| `Orchestrator` | ai | GOAP, BT, LLM orchestrators |
| `GameAdapter` | sdk | User game implementations |

### 4.3 Event Channels

- `astraweave-ecs::Events<T>` — typed event bus, used across physics (CollisionEvent, ContactForceEvent), gameplay, AI
- `astraweave-core::ecs_events` — bridge events between ECS and AI systems

### 4.4 System Stage Registration

All systems register into the 7-stage pipeline via `app.add_system(SystemStage::X, fn)`. Cross-crate system registration happens in examples and the editor, where multiple crates' systems are wired into a single `World`.

### 4.5 Editor↔Engine Rendering Boundary (Fix 27 — NEW)

The most critical integration seam after the Fix 27 campaign. All data flowing from the editor to the engine renderer:

| Data | Editor Type | Engine Type | Conversion | File |
|------|-------------|-------------|------------|------|
| Camera | `OrbitCamera` | `astraweave_render::camera::Camera` | `OrbitCamera::to_engine_camera()` — copies yaw, pitch, fov, aspect, znear, zfar | `viewport/camera.rs:567` |
| Entity pose | `astraweave_core::World::pose()` → `Pose` (with `float_x`, `float_z`, `height`, `scale`, `scale_y`, `scale_z`, `rotation`, `rotation_x`, `rotation_z`) | `astraweave_render::Instance` with `glam::Mat4` transform | Constructed via `Mat4::from_scale_rotation_translation(Vec3(scale, scale_y, scale_z), Quat::from_euler(XYZ, rot_x, rot, rot_z), Vec3(x, h, z))` | `engine_adapter.rs:160-213` |
| Entity selection | `Vec<astraweave_core::Entity>` | Per-instance `color: [f32; 4]` (orange `[1.0, 0.6, 0.2, 1.0]`) | Applied when `selected_entities.contains(&entity)` | `engine_adapter.rs:195-202` |
| Terrain chunks | `Vec<(Vec<TerrainVertex>, Vec<u32>)>` — editor `TerrainVertex` is 96 bytes (pos+norm+uv + 8 biome weights + material IDs) | `astraweave_render::TerrainVertex` — 36 bytes (pos+norm+uv+biome_id) | `TerrainVertex::to_engine_vertex()` — extracts dominant biome from 8 weight slots, discards per-vertex material_ids | `engine_adapter.rs:396-453`, `types.rs:35-63` |
| Scatter placements | `Vec<ScatterPlacement>` (pos+scale+rot+tint+mesh_key) | `astraweave_render::Instance` per placement | `Mat4::from_scale_rotation_translation(splat(p.scale), Quat::from_rotation_y(p.rotation), p.position)` | `engine_adapter.rs:485-573` |
| Fog parameters | `TerrainFogParams` | `astraweave_render::scene_environment::SceneEnvironment` | `env.visuals.fog_density`, `env.visuals.fog_color` | `engine_adapter.rs:630-638` |
| Lighting parameters | `TerrainLightingParams` | `SceneEnvironment` | `env.visuals.ambient_color`, `env.visuals.ambient_intensity` | `engine_adapter.rs:641-646` |
| Sky config | `astraweave_render::SkyConfig` | `astraweave_render::SkyConfig` | Direct pass-through | `engine_adapter.rs:597-605` |
| Weather | `WeatherKind` (editor enum, 6 variants) | `astraweave_render::WeatherKind` (engine enum) | Passed directly after mapping from 11-type `world_panel` weather via `WeatherKind::from_world_panel()` | `engine_adapter.rs:607-610`, `types.rs:154-165` |
| Water style | `WaterStyle` { Ocean, River, Lake, Swamp } | `astraweave_render::WaterRenderer` | Color presets applied during `WaterRenderer::new()` then `renderer.set_water_renderer()` | `engine_adapter.rs:649-686` |

**Format mismatch note (partially unresolved)**: Editor `TerrainVertex` is 96 bytes; engine is 36 bytes. The `to_engine_vertex()` conversion is lossy — biome blend information (8 weights) is collapsed to the single dominant biome, and per-vertex `material_ids` and `material_weights` are discarded. This was classified as a known HIGH divergence in the audit.

---

## 5. Data Flow Paths

### 5.1 AI Loop

```
World state
  → build_ai_snapshots() [PERCEPTION stage, astraweave-ai/src/core_loop.rs]
  → WorldSnapshot [astraweave-core]
  → AIArbiter.update() [astraweave-ai/src/arbiter.rs]
  → mode decision: GOAP | BehaviorTree | ExecutingLLM
  → Orchestrator.plan() → PlanIntent [astraweave-core]
  → tool_sandbox validation [astraweave-ai/src/tool_sandbox.rs]
  → ActionStep execution → World mutation
```

### 5.2 Render Pipeline (astraweave-render standalone)

```
Scene (Transform, Mesh, Material)
  → RenderGraph DAG [astraweave-render/src/graph.rs]
  → Depth pre-pass → GBuffer fill (deferred)
  → Clustered lighting / MegaLights [clustered.rs, clustered_megalights.rs]
  → Shadow CSM (4-cascade) + point shadows [shadow_csm.rs, shadow_point.rs]
  → SSAO/GTAO → SSR → SSGI [gtao.rs, ssr.rs, ssgi.rs]
  → Bloom → Auto-exposure → Tonemapping (ACES/AgX/PBR-Neutral) [bloom.rs, auto_exposure.rs, hdr_pipeline.rs]
  → TAA → Final output [taa.rs]
  → Post-process chain (motion blur, DoF, vignette) [post.rs]
```

### 5.3 Editor Viewport Rendering Pipeline (Unified — post Fix 27)

See Section 6 for the complete frame sequence diagram.

### 5.4 Physics Pipeline

```
Forces / character input
  → PhysicsWorld.step() [astraweave-physics, wraps Rapier3D]
  → SpatialHash broad-phase [spatial_hash.rs]
  → Rapier3D narrow-phase collision
  → CollisionEvent / ContactForceEvent → Events<T>
  → CharacterController resolution [character_controller.rs]
  → Subsystems: Ragdoll, Vehicle, Cloth, Destruction, Fluids
```

### 5.5 Asset Pipeline

```
Source files (.blend, .gltf, textures)
  → BlendImporter / decomposer [astraweave-blend/src/decomposer.rs]
  → Texture processor (HDR→PNG, thumbnails) [texture_processor.rs]
  → BiomePack bridge [astraweave-terrain/src/biome_pack.rs]
  → MaterialManager (TOML → GPU D2 arrays) [astraweave-render]
  → Runtime: cell_loader, mesh_obj/mesh_gltf, texture_streaming
```

### 5.6 Editor Command Flow

```
User input (mouse, keyboard, gamepad)
  → egui event handling [aw_editor/src/main.rs]
  → Command system [aw_editor/src/command.rs]
  → Undo/Redo stack (all 9 registered operations — Fix campaign)
  → Scene state mutation [scene_serialization.rs]
  → Viewport camera [viewport/camera.rs]
  → ViewportRenderer.render() [viewport/renderer.rs]
  → Panel updates (asset browser, terrain, profiler, etc.) [panels/*.rs]
```

---

## 6. Editor Viewport Architecture (Unified Pipeline — Fix 27)

### 6.1 Module Structure

**Files present after the Fix 27 campaign:**

| File | Role | LOC (approx) |
|------|------|-------------|
| `tools/aw_editor/src/viewport/mod.rs` | Module declarations and re-exports | ~70 |
| `tools/aw_editor/src/viewport/renderer.rs` | `ViewportRenderer` — rendering coordinator, pass sequencing | ~900 |
| `tools/aw_editor/src/viewport/engine_adapter.rs` | `EngineRenderAdapter` — wraps `astraweave_render::Renderer` | ~740 |
| `tools/aw_editor/src/viewport/camera.rs` | `OrbitCamera` — spherical coords, `to_engine_camera()` conversion | ~620 |
| `tools/aw_editor/src/viewport/widget.rs` | `ViewportWidget` — egui integration, input handling | ~large |
| `tools/aw_editor/src/viewport/types.rs` | Shared types: `TerrainVertex`, `SceneLight`, `GltfSkeleton`, `TerrainFogParams`, `TerrainLightingParams`, `WaterStyle`, `WeatherKind`, `MATERIAL_NAMES` | ~340 |
| `tools/aw_editor/src/viewport/gizmo_renderer.rs` | `GizmoRendererWgpu` — transform handle overlays | ~medium |
| `tools/aw_editor/src/viewport/grid_renderer.rs` | `GridRenderer` — floor grid + axes | ~medium |
| `tools/aw_editor/src/viewport/physics_renderer.rs` | `PhysicsDebugRenderer` — collider wireframes, brush cursor | ~medium |
| `tools/aw_editor/src/viewport/blueprint_overlay.rs` | `BlueprintOverlay` — zone visualization | ~medium |
| `tools/aw_editor/src/viewport/toolbar.rs` | `ViewportToolbar`, `GridType` | ~medium |
| `tools/aw_editor/src/viewport/shaders/tonemap.wgsl` | HDR→LDR blit: ACES (mode=0), PBR Neutral/Khronos (mode=1), Reinhard (mode=2) | ~small |
| `tools/aw_editor/src/viewport/shaders/grid.wgsl` | Floor grid with axes, anti-aliased lines | ~small |
| `tools/aw_editor/src/viewport/shaders/gizmo.wgsl` | Transform handle geometry | ~small |

**Files DELETED in the Fix 27 campaign:**

| Deleted File | What It Was |
|-------------|------------|
| `tools/aw_editor/src/viewport/entity_renderer.rs` | ~3,600 LOC — editor's own PBR shader, shadow system, BRDF LUT, IBL computation, glTF/texture pipeline, material uniforms. The "FastPreview" path. |
| `tools/aw_editor/src/viewport/mipmap_generator.rs` | GPU mipmap generation using compute shaders |
| `tools/aw_editor/src/viewport/shaders/entity.wgsl` | Editor's own PBR WGSL shader (full Cook-Torrance BRDF) |
| `tools/aw_editor/src/viewport/shaders/shadow.wgsl` | Editor's own shadow map WGSL shader |
| `tools/aw_editor/src/viewport/shaders/brdf_lut.wgsl` | BRDF integration LUT compute shader |
| `tools/aw_editor/src/viewport/shaders/mipmap_blit.wgsl` | GPU mipmap generation shader |
| `tools/aw_editor/src/tab_viewer.rs` | Tab viewer logic (moved/unified) |

### 6.2 Viewport Module Hierarchy

```
ViewportWidget (widget.rs)              — egui integration layer, input handling
  |
  └─> ViewportRenderer (renderer.rs)   — rendering coordinator
        |
        ├─ EngineRenderAdapter          — engine bridge (primary scene renderer)
        |   (engine_adapter.rs)             wraps astraweave_render::Renderer
        |
        ├─ GridRenderer                 — floor grid + axes overlay
        |   (grid_renderer.rs)              uses shaders/grid.wgsl
        |
        ├─ GizmoRendererWgpu            — transform handles (LDR overlay)
        |   (gizmo_renderer.rs)             uses shaders/gizmo.wgsl
        |
        └─ PhysicsDebugRenderer         — collider wireframes, brush cursor, zone overlays
            (physics_renderer.rs)           uses physics_renderer.wgsl (inline)
```

### 6.3 Frame Rendering Sequence

`ViewportRenderer::render()` in `tools/aw_editor/src/viewport/renderer.rs`:

```
Frame Start
  |
  ├─ [Size check] If viewport resized → resize():
  |     Creates: Depth32Float texture, Rgba16Float HDR texture
  |     Creates: Tonemap pipeline (tonemap.wgsl), Tonemap bind group
  |     Creates: Depth staging buffer (256 bytes, for 1-pixel readback)
  |     Resizes: EngineRenderAdapter (propagates to engine Renderer)
  |
  ├─ [PASS 1: Engine Scene — HDR target (Rgba16Float)]
  |     adapter.update_camera(camera)           → engine camera update
  |     adapter.feed_entities(world, meshes, selected)  → group by mesh, upload instances
  |     adapter.render_to_texture(hdr_view, encoder)    → engine draw_into()
  |         Engine draws: sky, IBL, terrain, scatter, water, weather particles,
  |                       entity meshes (PBR), 4-cascade CSM shadows,
  |                       post-processing chain (bloom, TAA, etc.)
  |     [Fallback if no adapter: clear to dark (0.12, 0.12, 0.15)]
  |
  ├─ [PASS 2: Grid Overlay — HDR target]
  |     Conditional on show_grid flag
  |     grid_renderer.render(encoder, hdr_view, depth_view, camera, ...)
  |         Uses shaders/grid.wgsl, reads GizmoUniforms (224 bytes)
  |
  ├─ [PASS 3: Physics/Debug Lines — HDR target]
  |     Conditional on any of: component_gizmo_lines, physics_debug_lines,
  |                             brush_cursor_lines, zone_overlay_lines
  |     PhysicsDebugRenderer: merges all line arrays, single render call
  |
  ├─ [PASS 4: HDR → LDR Blit — LDR target (Bgra8UnormSrgb)]
  |     Tonemap pipeline: draws full-screen triangle (3 vertices, 0 index buffer)
  |     Reads HDR texture (binding 0) + sampler (binding 1) + params (binding 2)
  |     Params uniform [u32; 4]: tonemap_mode (0=ACES, 1=PBR Neutral, 2=Reinhard)
  |     Output: final display surface for egui
  |
  └─ [PASS 5: Gizmo Overlays — LDR target]
        Only if entity selected AND gizmo mode != Inactive
        gizmo_renderer.render(encoder, ldr_target_view, depth_view, camera, ...)
        Renders AFTER tonemapping for crisp LDR overlays
        Uses shaders/gizmo.wgsl
```

**Command submission**: All passes share one `wgpu::CommandEncoder`, submitted via `queue.submit(once(encoder.finish()))` at the end.

### 6.4 GPU Resource Inventory (per ViewportRenderer instance)

| Resource | Format | Usage | Notes |
|----------|--------|-------|-------|
| HDR scene texture | `Rgba16Float` | `RENDER_ATTACHMENT + TEXTURE_BINDING` | Created on first resize; recreated on size change |
| Depth texture | `Depth32Float` | `RENDER_ATTACHMENT + TEXTURE_BINDING + COPY_SRC` | Shared across all passes |
| Depth staging buffer | `u8[256]` | `COPY_DST + MAP_READ` | 256 bytes (wgpu alignment); used for 1-pixel brush depth readback |
| Tonemap pipeline | — | `RenderPipeline` | Created once, reused on resize (bind group recreated, pipeline is stable) |
| Tonemap bind group | — | `BindGroup` | Recreated on every resize (references HDR texture view) |
| Tonemap params buffer | `u32[4]` | `UNIFORM + COPY_DST` | Mode selection: `[tonemap_mode, 0, 0, 0]` |

### 6.5 RenderMode Enum

```rust
pub enum RenderMode {
    EnginePBR,     // Default — full engine renderer (astraweave-render)
    FastPreview,   // Legacy label — cube placeholders, simple lighting
                   // NOTE: FastPreview no longer has its own shader path.
                   // The enum value is kept for API compatibility but behavior
                   // is now identical to EnginePBR (engine adapter is always active).
}
```

The `FastPreview` variant is retained in the enum but the rendering code does not branch on it for the scene pass — `EngineRenderAdapter` is always used when initialized. The distinction is whether the `engine_adapter` field is `Some` (normal operation) or `None` (headless/CI fallback, which clears to a dark color).

### 6.6 EngineRenderAdapter State

`tools/aw_editor/src/viewport/engine_adapter.rs`

| Field | Type | Purpose |
|-------|------|---------|
| `renderer` | `astraweave_render::Renderer` | The engine renderer instance |
| `initialized` | `bool` | Guard for is_initialized() queries |
| `terrain_model_names` | `Vec<String>` | Tracks `"terrain_chunk_N"` names for cleanup |
| `scatter_model_names` | `Vec<String>` | Tracks `"scatter_{key}"` names for cleanup |
| `terrain_total_triangles` | `usize` | Scene stats (triangles) |
| `terrain_total_indices` | `usize` | Scene stats (indices) |
| `scatter_placement_count` | `usize` | Scene stats (instance count) |
| `scatter_draw_call_count` | `u32` | Scene stats (one draw call per mesh type) |
| `weather_active` | `bool` | Whether weather system is running |
| `water_enabled` | `bool` | Whether WaterRenderer is attached |

Model naming conventions (important — model name collisions = wrong render output):
- Entity models: `"entity_default_cubes"` (no mesh), `"entity_mesh_{path_sanitized}"` (with mesh, `/\\.` → `_`)
- Terrain chunks: `"terrain_chunk_0"`, `"terrain_chunk_1"`, ...
- Scatter groups: `"scatter_{mesh_key}"`

### 6.7 Pose → Transform Mapping

`astraweave_core::Pose` fields consumed by `engine_adapter.rs::feed_entities()`:

```
Pose.use_float_pos  →  selects between float_x/float_z vs pos.x/pos.y (integer grid)
Pose.float_x        →  world X position
Pose.float_z        →  world Z position (mapped from Y in 2D integer coords)
Pose.height         →  world Y position
Pose.scale          →  uniform scale X
Pose.scale_y        →  per-axis scale Y  [ADDED in Fix 27 campaign — Fix 9]
Pose.scale_z        →  per-axis scale Z  [ADDED in Fix 27 campaign — Fix 9]
Pose.rotation       →  Y-axis rotation (radians)
Pose.rotation_x     →  X-axis rotation (radians)
Pose.rotation_z     →  Z-axis rotation (radians)
```

Team color mapping (when not selected):
- team.id == 0: green `[0.2, 0.8, 0.3, 1.0]`
- team.id == 1: blue `[0.3, 0.6, 1.0, 1.0]`
- team.id == 2: red `[1.0, 0.3, 0.2, 1.0]`
- team.id other: gray `[0.6, 0.6, 0.7, 1.0]`
- no team: white `[1.0, 1.0, 1.0, 1.0]`

---

## 7. Unsafe Code Inventory

### Verified Crates (Miri + Kani validated)

| Crate | Primary Unsafe Locations | Purpose |
|-------|------------------------|---------|
| `astraweave-ecs` | blob_vec.rs, archetype.rs, entity_allocator.rs, sparse_set.rs, parallel.rs, system_param.rs, command_buffer.rs | Type-erased component storage, raw memory alloc/dealloc, pointer arithmetic, drop function pointers |
| `astraweave-math` | simd_vec.rs, simd_mat.rs | SIMD intrinsics (auto-vectorization-assist) |
| `astraweave-core` | ecs_bridge.rs, ecs_events.rs | Entity mapping, event bridging |
| `astraweave-sdk` | lib.rs (C ABI functions) | FFI boundary: null checks, buffer overflow protection |

**Kani Proof Files**:
- `astraweave-ecs/src/blob_vec_kani.rs` — BlobVec invariants
- `astraweave-ecs/src/entity_allocator_kani.rs` — Generational index correctness
- `astraweave-core/src/schema_kani.rs` — Schema verification
- `astraweave-math/src/simd_vec_kani.rs` — SIMD operation correctness
- `astraweave-sdk/src/lib_kani.rs` — FFI safety (buffer overflow, null pointer)

### Other Crates with Unsafe

`astraweave-ai` (async_task.rs — custom RawWaker VTable), plus minor unsafe in: physics, render, asset, audio, fluids, scripting. These are not Miri/Kani validated but are lower-risk (mostly FFI to external libs like Rapier3D, wgpu, rodio).

---

## 8. Test Infrastructure

### Test Coverage by Crate

**46 crates** have `#[cfg(test)]` modules. **56 crates** have criterion benchmark harnesses.

| Tier | Crates | Status |
|------|--------|--------|
| **Formally Verified** | ecs, math, core, sdk | Miri (977 tests, 0 UB) + Kani proofs |
| **A+ Grade** | fluids (2,404 tests) | Benchmark caliber |
| **A Grade** | physics/core (110+), environment (55+) | Strong coverage |
| **B+ Grade** | vehicle (50+), gravity (30+) | Good, missing edge cases |
| **B Grade** | cloth (25+), ragdoll (33+) | Missing stress tests |
| **C-D Grade** | destruction (17), projectile (21), spatial_hash (8), async_scheduler (4) | Active improvement |

### CI Workflows

| Workflow | Schedule | Crates | Timeout |
|----------|----------|--------|---------|
| `miri.yml` | Weekly (Sat 2 AM UTC) | ecs (120m), core (90m), physics (90m), ai (60m) | Per-crate |
| `kani.yml` | Weekly (Sun 3 AM UTC) | ecs (120m), math (60m), sdk (60m), core (90m) | Per-crate |

### Editor Tests
`aw_editor`: **6,100+ tests** covering command system, gizmo operations, panels, viewport, scene serialization, prefabs, behavior graph, terrain integration, asset browser.

---

## 9. Known Issues & Exclusions

### Build Exclusions

| Crate/Example | Issue |
|--------------|-------|
| `ui_controls_demo`, `debug_overlay` | egui/winit version drift — won't compile |
| `astraweave-author`, `rhai_authoring` | Rhai Sync trait errors |
| `astraweave-llm`, `llm_toolcall` | Excluded from standard builds |
| ECS fuzz targets | Must be built separately (`astraweave-ecs/fuzz` excluded) |

### Production Safety
- **Zero `.unwrap()` in production code** — all confined to `#[cfg(test)]` modules
- Build/CLI tools have low-risk `.unwrap()` in non-runtime paths
- Windows: 16 MB stack size (configured in `.cargo/config.toml`) for large State structs

### Active Work (as of 2026-04-04)
- Fix 27 Unified Pipeline Campaign — structural phase complete (astraweave-render non-optional, entity rendering through engine); deeper unification (shadow/IBL/post-processing alignment) ongoing
- Editor behavioral correctness audit remediation — 37 fixes across 47 commits shipped

### Known Behavioral Issues (post-audit, not yet fixed)

| ID | Location | Issue |
|----|----------|-------|
| M-21 | `gizmo/scale.rs:56-57` | Scale gizmo UP-only — `mouse_delta.length()` always positive; can't downscale via drag |
| C-5 | `command.rs` | 9 operations bypass undo stack (AddComponent, RemoveComponent, ComponentDataChanged, MaterialPropertyChanged, MaterialTextureChanged still missing command classes) — **now resolved as of 2026-04-04 audit remediation** — verify against current code |
| Mutex poison | `viewport/widget.rs` (×16) | `if let Ok(mut renderer) = renderer.lock()` with no else branch — viewport freeze on any renderer panic |
| TerrainVertex | `types.rs`, `engine_adapter.rs` | 96-byte editor → 36-byte engine conversion is lossy; biome blend weights collapsed to dominant biome |
| Scatter mesh | `engine_adapter.rs:524` | Scatter placements always use cube placeholder; actual glTF loading not yet wired for scatter |

---

## 10. Quick Reference: Where to Find Things

| Need | Location |
|------|----------|
| AI orchestration | `astraweave-ai/src/{orchestrator,tool_sandbox,core_loop,arbiter}.rs` |
| WorldSnapshot definition | `astraweave-core/src/` (grep: `pub struct WorldSnapshot`) |
| ECS internals | `astraweave-ecs/src/{archetype,blob_vec,entity_allocator,events,parallel}.rs` |
| Rendering pipeline (engine) | `astraweave-render/src/{graph,renderer,hdr_pipeline,clustered,shadow_csm}.rs` |
| Engine renderer public API | `astraweave-render/src/lib.rs` (re-exports section) |
| Physics engine | `astraweave-physics/src/{character_controller,spatial_hash}.rs` |
| Combat system | `astraweave-gameplay/src/combat_physics.rs` |
| SIMD math | `astraweave-math/src/{simd_vec,simd_mat,simd_quat,simd_movement}.rs` |
| Terrain generation | `astraweave-terrain/src/{voxel_mesh,biome_pack,biome,scatter,blueprint_zone,climate,biome_lookup,biome_parameters,biome_param_blending,world_archetypes}.rs` |
| Blend import | `crates/astraweave-blend/src/{decomposer,texture_processor,importer}.rs` |
| Editor entry points | `tools/aw_editor/src/{main,lib,command,ui/mod}.rs` |
| Viewport module structure | `tools/aw_editor/src/viewport/mod.rs` |
| Unified render loop | `tools/aw_editor/src/viewport/renderer.rs` — `ViewportRenderer::render()` |
| Engine bridge | `tools/aw_editor/src/viewport/engine_adapter.rs` — `EngineRenderAdapter` |
| Entity → instance conversion | `tools/aw_editor/src/viewport/engine_adapter.rs:160-290` — `feed_entities()` |
| Terrain upload to engine | `tools/aw_editor/src/viewport/engine_adapter.rs:396-463` — `upload_terrain_chunks()` |
| Scatter upload to engine | `tools/aw_editor/src/viewport/engine_adapter.rs:485-573` — `upload_scatter_placements()` |
| Camera→engine conversion | `tools/aw_editor/src/viewport/camera.rs:567` — `to_engine_camera()` |
| TerrainVertex format | `tools/aw_editor/src/viewport/types.rs:17-63` |
| Shared viewport types | `tools/aw_editor/src/viewport/types.rs` — `SceneLight`, `GltfSkeleton`, `GltfAnimationClip`, `WeatherKind`, material constants |
| HDR tonemap shader | `tools/aw_editor/src/viewport/shaders/tonemap.wgsl` |
| Grid shader | `tools/aw_editor/src/viewport/shaders/grid.wgsl` |
| Gizmo shader | `tools/aw_editor/src/viewport/shaders/gizmo.wgsl` |
| Build config | `.cargo/config.toml`, root `Cargo.toml` |
| Kani proofs | `astraweave-{ecs,math,core,sdk}/src/*_kani.rs` |
| CI workflows | `.github/workflows/{miri,kani}.yml` |
| Project status | `docs/current/PROJECT_STATUS.md` |
| Behavioral audit | `docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md` |
| Fix 27 campaign plan | `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` |
| Benchmarks | `docs/current/MASTER_BENCHMARK_REPORT.md` |
| Coverage | `docs/current/MASTER_COVERAGE_REPORT.md` |

---

## 11. Recent Changes (v0.4.0 → v0.5.0, 2026-04-04)

### Fix 27 — Unified Rendering Pipeline

**Deleted** (no longer in codebase):
- `tools/aw_editor/src/viewport/entity_renderer.rs` (~3,600 LOC)
- `tools/aw_editor/src/viewport/mipmap_generator.rs`
- `tools/aw_editor/src/viewport/shaders/entity.wgsl`
- `tools/aw_editor/src/viewport/shaders/shadow.wgsl`
- `tools/aw_editor/src/viewport/shaders/brdf_lut.wgsl`
- `tools/aw_editor/src/viewport/shaders/mipmap_blit.wgsl`
- `tools/aw_editor/src/tab_viewer.rs`

**Changed**:
- `tools/aw_editor/Cargo.toml`: `astraweave-render` is now a required dependency (was `optional = true`)
- `tools/aw_editor/src/viewport/engine_adapter.rs`: Expanded from ~568 LOC to ~740 LOC — `feed_entities()`, `upload_terrain_chunks()`, `upload_scatter_placements()` added
- `tools/aw_editor/src/viewport/types.rs`: `SceneLight` and `GltfSkeleton`/`GltfAnimationClip` types moved here from `entity_renderer.rs`
- `astraweave-render/src/renderer.rs` (engine): 4-cascade CSM shadows replacing single-cascade
- `astraweave-render/src/renderer.rs` (engine): IBL prefiltered cubemap infrastructure added
- `astraweave-render/src/renderer.rs` (engine): Khronos PBR Neutral tonemapper added alongside ACES/Reinhard
- `astraweave_core::Pose`: Added `scale_y`, `scale_z` fields for per-axis scale

**37 additional behavioral fixes** across `command.rs`, `gizmo/`, `panels/`, `scene_serialization.rs`, `widget.rs`, and others — see `docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md` for the full list.
