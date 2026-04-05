# AstraWeave Architecture Map

> **Generated**: 2026-04-04 | **Version**: 0.4.0 | **Rust**: 1.89.0
> Living document — used by all agents as the primary architectural reference.

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
| `aw_editor` | astract, core, ecs, profiling, render (opt), author, asset, audio, behavior, dialogue, quests, nav, observability, physics, security, terrain, asset-pipeline |

### 2.2 Leaf Crates (zero workspace deps)
`math`, `nav`, `materials`, `cinematics`, `asset-pipeline`, `input`, `pcg`, `fluids`

### 2.3 Architectural Anomalies

1. **`terrain` → `gameplay`** (reverse dependency): World-gen crate imports gameplay biome types. Should flow the other way. Blast radius: changing gameplay biome types breaks terrain generation.
2. **`render` → `aw_asset_cli`** (production → tool): A runtime rendering crate depending on a CLI tool crate. Unusual dependency direction.
3. **`npc` does NOT depend on `ai` or `behavior`**: NPC crate depends on physics/audio/gameplay instead. NPC behavior logic is decoupled from the AI orchestration layer.
4. **`core` → `behavior`**: Core foundation depends on behavior trees (behavioral is part of the core abstraction).

---

## 3. Public API Surface

### Core

**astraweave-ecs**
- Types: `World`, `Entity`, `EntityAllocator`, `CommandBuffer`, `Events`, `EventReader`, `Query`, `Query2`, `Query2Mut`, `Rng`, `ParallelSchedule`, `TypeRegistry`
- Traits: `Component`, `Resource`, `SystemParam`
- Stages: `PRE_SIMULATION`, `PERCEPTION`, `SIMULATION`, `AI_PLANNING`, `PHYSICS`, `POST_SIMULATION`
- Modules: archetype, blob_vec, command_buffer, component_meta, entity_allocator, events, parallel, rng, sparse_set, type_registry

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

**astraweave-materials**
- Enum: `Node` { Texture2D, Constant3, Constant1, Multiply, Add, MetallicRoughness, Clearcoat, Anisotropy, Transmission, NormalMap }
- Material graph node system (Phase 2 foundation)

### Physics

**astraweave-physics**
- Types: `PhysicsWorld`, `CharacterController`, `SpatialHash`, `ProjectileManager`, `GravityManager`, `Ragdoll`, `Vehicle`, `ClothManager`, `DestructionManager`, `EnvironmentManager`
- Wraps Rapier3D 0.22, re-exports all Rapier types
- Features: async-physics, profiling, ecs

**astraweave-fluids**
- 40+ modules: SPH, boundary, caustics, foam, god_rays, LOD, multi-phase, turbulence, underwater, waterfall
- Types: `WaterBuildingManager`, `WaterEffectsManager`, `CausticsSystem`, `WaterQualityPreset`
- Grade: A+ (2,404 tests)

### World & Scene

**astraweave-scene**
- Types: `Transform`, `Node`, `Scene`, `SceneError`
- Modules: gpu_resource_manager, partitioned_scene, streaming, world_partition

**astraweave-terrain**
- Types: `Biome`, `BiomeType`, `ChunkManager`, `TerrainChunk`, `Heightmap`, `VoxelGrid`, `WorldConfig`, `ZoneRegistry`
- 25+ modules: erosion, marching cubes, LOD, noise, scatter, biome blending, blueprint zones

**astraweave-nav**
- Types: `NavMesh`, `NavTri`, `Triangle`, `Aabb`
- A* pathfinding, navmesh baking, dirty region tracking

### Gameplay

**astraweave-gameplay**
- 20+ modules: combat, combat_physics, crafting, dialogue, quests, items, stats, biome, harvesting, weaving, weave_portals
- All modules re-exported via `pub use *`

**astraweave-audio**
- Types: `AudioEngine`, `DialoguePlayer`, `VoiceBank`, `EmitterId`, `ListenerPose`
- 4-bus mixer (master, music, SFX, voice), 3D spatial audio, TTS adapter

**astraweave-ui**
- Types: `HudManager`, `MenuManager`, `AccessibilitySettings`, `GamepadManager`
- Modules: accessibility, gamepad, hud, layer, menu, panels, persistence, state
- egui-based, colorblind modes, gamepad support, TOML persistence

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

### 5.2 Render Pipeline
```
Scene (Transform, Mesh, Material)
  → RenderGraph DAG [astraweave-render/src/graph.rs]
  → Depth pre-pass → GBuffer fill (deferred)
  → Clustered lighting / MegaLights [clustered.rs, clustered_megalights.rs]
  → Shadow CSM + point shadows [shadow_csm.rs, shadow_point.rs]
  → SSAO/GTAO → SSR → SSGI [gtao.rs, ssr.rs, ssgi.rs]
  → Bloom → Auto-exposure → Tonemapping (ACES/AgX) [bloom.rs, auto_exposure.rs, hdr_pipeline.rs]
  → TAA → Final output [taa.rs]
  → Post-process chain (motion blur, DoF, vignette) [post.rs]
```

### 5.3 Physics Pipeline
```
Forces / character input
  → PhysicsWorld.step() [astraweave-physics, wraps Rapier3D]
  → SpatialHash broad-phase [spatial_hash.rs]
  → Rapier3D narrow-phase collision
  → CollisionEvent / ContactForceEvent → Events<T>
  → CharacterController resolution [character_controller.rs]
  → Subsystems: Ragdoll, Vehicle, Cloth, Destruction, Fluids
```

### 5.4 Asset Pipeline
```
Source files (.blend, .gltf, textures)
  → BlendImporter / decomposer [astraweave-blend/src/decomposer.rs]
  → Texture processor (HDR→PNG, thumbnails) [texture_processor.rs]
  → BiomePack bridge [astraweave-terrain/src/biome_pack.rs]
  → MaterialManager (TOML → GPU D2 arrays) [astraweave-render]
  → Runtime: cell_loader, mesh_obj/mesh_gltf, texture_streaming
```

### 5.5 Editor Pipeline
```
User input (mouse, keyboard, gamepad)
  → egui event handling [aw_editor/src/main.rs]
  → Command system [aw_editor/src/command.rs]
  → Undo/Redo stack
  → Scene state mutation [scene_serialization.rs]
  → Viewport camera [viewport/camera.rs]
  → Entity/Gizmo/Grid/Physics rendering [viewport/*.rs]
  → Panel updates (asset browser, terrain, profiler, etc.) [panels/*.rs]
```

---

## 6. Unsafe Code Inventory

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

**Miri limitation**: Physics crate's Rapier3D FFI cannot be fully analyzed by Miri (documented with graceful fallback in CI).

---

## 7. Test Infrastructure

### Test Coverage by Crate

**46 crates** have `#[cfg(test)]` modules. **56 crates** have criterion benchmark harnesses.

| Tier | Crates | Status |
|------|--------|--------|
| **Formally Verified** | ecs, math, core, sdk | Miri (977 tests, 0 UB) + Kani proofs |
| **A+ Grade** | fluids (2,404 tests) | Benchmark caliber |
| **A Grade** | physics/core (110+), environment (55+) | Strong coverage |
| **B+ Grade** | vehicle (50+), gravity (30+) | Good, missing edge cases |
| **B Grade** | cloth (25+), ragdoll (33+) | Missing stress tests |
| **C-D Grade** | destruction (17), projectile (21), spatial_hash (8), async_scheduler (4) | Active improvement (Phase 8.8) |

### CI Workflows

| Workflow | Schedule | Crates | Timeout |
|----------|----------|--------|---------|
| `miri.yml` | Weekly (Sat 2 AM UTC) | ecs (120m), core (90m), physics (90m), ai (60m) | Per-crate |
| `kani.yml` | Weekly (Sun 3 AM UTC) | ecs (120m), math (60m), sdk (60m), core (90m) | Per-crate |

### Editor Tests
`aw_editor`: **6,100+ tests** covering command system, gizmo operations, panels, viewport, scene serialization, prefabs, behavior graph, terrain integration, asset browser.

---

## 8. Known Issues & Exclusions

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
- Phase 8.8: Physics Robustness Upgrade (spatial hash, async scheduler, projectile)
- Editor viewport rendering pipeline (post-processing chain, TAA, velocity buffer)
- In-Game UI Framework at 72% (Week 4 of 5)

---

## 9. Quick Reference: Where to Find Things

| Need | Location |
|------|----------|
| AI orchestration | `astraweave-ai/src/{orchestrator,tool_sandbox,core_loop,arbiter}.rs` |
| WorldSnapshot definition | `astraweave-core/src/` (exact file varies — grep for `pub struct WorldSnapshot`) |
| ECS internals | `astraweave-ecs/src/{archetype,blob_vec,entity_allocator,events,parallel}.rs` |
| Rendering pipeline | `astraweave-render/src/{graph,renderer,hdr_pipeline,clustered,shadow_csm}.rs` |
| Physics engine | `astraweave-physics/src/{character_controller,spatial_hash}.rs` |
| Combat system | `astraweave-gameplay/src/combat_physics.rs` |
| SIMD math | `astraweave-math/src/{simd_vec,simd_mat,simd_quat,simd_movement}.rs` |
| Terrain generation | `astraweave-terrain/src/{voxel_mesh,biome_pack,biome,scatter,blueprint_zone}.rs` |
| Blend import | `crates/astraweave-blend/src/{decomposer,texture_processor,importer}.rs` |
| Editor | `tools/aw_editor/src/{main,lib,command,ui/mod}.rs` |
| Viewport | `tools/aw_editor/src/viewport/{mod,renderer,camera,gizmo_renderer}.rs` |
| Build config | `.cargo/config.toml`, root `Cargo.toml` |
| Kani proofs | `astraweave-{ecs,math,core,sdk}/src/*_kani.rs` |
| CI workflows | `.github/workflows/{miri,kani}.yml` |
| Project status | `docs/current/PROJECT_STATUS.md` |
| Benchmarks | `docs/current/MASTER_BENCHMARK_REPORT.md` |
| Coverage | `docs/current/MASTER_COVERAGE_REPORT.md` |
