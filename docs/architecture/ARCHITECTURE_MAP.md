# AstraWeave Architecture Map

> **Generated**: 2026-05-07 | **Last reconciled**: 2026-06-29 | **Version**: 0.7.4 | **Rust**: 1.89.0
> Living document — used by all agents as the primary architectural reference.
> **0.7.2 update (2026-06-10)**: Engine-health-audit reconciliation — workspace member count corrected 143 → **130** (root Cargo.toml members verified identical to `cargo metadata --no-deps`); `astraweave-camera` (Unified Camera C.2, `52b9e711c`, 2026-05-18) added to §1/§2.1 and to `astraweave-render`'s dep row; examples count corrected to 59; §0 editor/render trace rows updated for Multi-Tool Sub-phase 3/4 closeouts (SP5 in flight) and Render Parity P.1–P.7 closure.
> **0.7.1 update (2026-06-10)**: Net-Trio-Remediation reconciliation — the standalone-trio HMAC-vs-XOR signature mismatch is RESOLVED (canonical HMAC-SHA256 signing enforced end-to-end, kick-by-default). Updated the net_ecs subsystem row (§0), the known-issues silent-failure row (§4.3), the documentation-hazards row (§7.2), the §8.8 network data-flow diagram, and §14 open-question 17. See `net_ecs.md` §6/§7 and `docs/audits/net_trio_signature_remediation_findings_2026-06.md`.
> **0.7.0 update**: Reconciled against the 13 per-subsystem architecture traces under `docs/architecture/`. Every traced subsystem now linked at section level. Workspace-wide structural axioms (§7.7 wrapped-component resource identity, Fix-27 dual-pipeline lesson, silent-failure policy, "wired beats tested" taxonomy) crystallised into their own section. Dormant-code taxonomy and documentation-hazard inventory consolidated from per-trace evidence. **0.6.0 update (2026-05-07)**: Full cartography audit — impostor infrastructure catalogued, astraweave-alloc added, workspace count verified at 143 members, viewport module inventory updated, Regional Archetype Variation campaign status integrated.
> **0.5.0 update (2026-04-04)**: Reflects Fix 27 Unified Pipeline Campaign — EntityRenderer deleted, astraweave-render is now non-optional in aw_editor.

---

## 0. Trace Index (read these first when modifying a traced subsystem)

| Subsystem | Trace | Status (per trace) |
|---|---|---|
| Terrain Material System | [`terrain_materials.md`](terrain_materials.md) | Canonical 32-layer pipeline active; legacy 8-layer `texture_splatting` test-only; simple `terrain.rs` path transitional (1 caller) |
| Render Pipeline + Material System + Shader Infrastructure | [`render_pipeline_material_system_shader_infrastructure.md`](render_pipeline_material_system_shader_infrastructure.md) | Active workzone — Multi-Tool Sub-phases 3/4 closed (2026-05-14 / 2026-06-06), SP5 in flight; Editor-Engine Render Parity P.1–P.7 closed 2026-05-17 (bit-identical parity harness); 123 source files + 71 WGSL shaders + editor viewport |
| Physics | [`physics.md`](physics.md) | Active core (`PhysicsWorld` + `CharacterController` + Rapier broadphase) with feature-gated and dormant subsystems; `SpatialHash` module dormant despite doc-comment advertising |
| Persistence (aw-save + persistence-ecs) | [`persistence_ecs.md`](persistence_ecs.md) | `aw-save` production-grade; `astraweave-persistence-ecs` has working roundtrip but `auto_save_system` and replay event apply are TODO stubs; declared by stress-test but unused |
| Net (snapshot-based server) | [`net.md`](net.md) | Active; 2D grid `IVec2` model; JSON over WebSocket; coexists with `astraweave-net-ecs` (different model entirely) |
| Net-ECS + standalone matchmaking | [`net_ecs.md`](net_ecs.md) | Standalone server/client production-style; ECS Plugin layer **dormant** (zero production consumers despite declared dep); the former HMAC-vs-XOR sig mismatch is **RESOLVED** — HMAC-SHA256 signing now enforced end-to-end, kick-by-default (Net-Trio-Remediation; `net_ecs.md` §6/§7) |
| Input | [`input.md`](input.md) | Pure facade over winit + gilrs; declared by `astraweave-gameplay` and `astraweave-ui` but neither imports the crate; editor reinvents the entire input domain in a 2,511-LoC panel that doesn't drain its action queue |
| Fluids | [`fluids.md`](fluids.md) | **Dormant/deprecated for runtime engine (post-W.1).** ~24.2K LoC after W.1 (2026-06-20) removed the SPH/voxel solver + `simd_ops.rs` (−58.8K LoC); only `examples/` consume it. Now the deprecated PBD remnant + retained F.4 GPU-particle accent substrate (`FluidSystem` / `WaterEffectsManager`; `PcisphSystem`/`UnifiedSolver`/`ResearchFluidSystem` absent). The live water system is the view-side successor — [`water.md`](water.md) |
| ECS substrate + Math/Core/SDK | [`ecs_math_core_sdk_foundation.md`](ecs_math_core_sdk_foundation.md) | Active foundation; **two `World` types** and **two `Entity` types** coexist (legacy `core::World` bridged into ECS via `build_app`); `ParallelSchedule` removed 2026-04-18 |
| Audio | [`audio.md`](audio.md) | Active rodio facade; **5 buses** (not 4 per stale lib.rs docstring); not an ECS Resource (rodio chain is `!Send`); 10+ editor panel knobs are no-op forward-design |
| Animation System | [`animation.md`](animation.md) | Phase 2 Task 5 complete; **four parallel type families** (`render::Skeleton`/`asset::Skeleton`/`scene::CSkeleton`/`editor::GltfSkeleton`) — no shared types, no `From` impls; `MAX_JOINTS = 256` hard-coded in two places; CubicSpline falls back to Linear/Slerp |
| AI Pipeline (8 subsystem traces) | [`ai_pipeline.md`](ai_pipeline.md) | Active foundation — engine's first-class citizen. 12,700+ agents @ 60 FPS validated. **Two GOAP implementations**, **runtime LLM default still phi3:medium despite Qwen3 doc-comments**, hardening surface (~15K LoC) shelf-stocked but not in line, Memory/Coordination/RAG subsystems dormant |
| aw_editor (Visual Editor) | [`aw_editor.md`](aw_editor.md) | Mid-campaign. 216 `.rs` files / 224,584+ LoC; god-struct `EditorApp` (123 fields); §7.7 wrapped-component trap surfaced at 4 layers; Multi-Tool Sub-phases 3 (incl. Real-Fix.A–E) and 4 COMPLETE, Sub-phase 5 in flight (5.A/5.B landed 2026-06-06; 5.C closeout + Mediator Removal + SP6 pending) |

The 5-prompt trace toolkit lives at `_meta/` (`ARCHITECTURE_TRACE_TEMPLATE.md`, `TRACE_PROMPT_TEMPLATE.md`, `TRACE_VERIFICATION_PROMPT_TEMPLATE.md`, `DEEP_TRACE_INVESTIGATION_TEMPLATE.md`, `SUBSYSTEM_TRACE_EXPANSION_PROMPT_TEMPLATE.md`).

---

## 1. Workspace Overview

- **Total workspace members**: 130 (root Cargo.toml members list, verified identical to `cargo metadata --no-deps` 2026-06-10; the prior 143 figure was a stale over-count)
- **Production crates**: ~51 (core engine infrastructure, incl. `astraweave-camera` added 2026-05-18)
- **Examples**: 59
- **Tools**: 12 (`aw_editor`, `aw_asset_cli`, `astraweave-assets`, `aw_debug`, `aw_build`, `aw_texture_gen`, `aw_headless`, `ollama_probe`, `asset_signing`, `aw_release`, `aw_demo_builder`, `aw_save_cli`)
- **Crates/** subdirectory**: 5 (`astraweave-blend`, `astraweave-alloc`, `astraweave-persistence-player`, `astract`, `astract/astract-macro`)
- **Networking sub-crates**: 3 (`aw-net-proto`, `aw-net-server`, `aw-net-client`)
- **Persistence**: `aw-save`, `aw_save_cli`, `astraweave-persistence-ecs`, `astraweave-persistence-player` (the last is a **disjoint** player-profile crate, not part of the ECS-world-save chain — see `persistence_ecs.md` §6)
- **Resolver**: 2
- **Build profiles**: dev (opt-level 0, deps opt-level 2), release-fast (no LTO), release (thin LTO)
- **Excludes**: `astraweave-ecs/fuzz` (must be built separately)

---

## 2. Crate Dependency Graph

### 2.1 Domain Groupings

**Core (foundation — minimal deps).** Trace: [`ecs_math_core_sdk_foundation.md`](ecs_math_core_sdk_foundation.md)

| Crate | Workspace Deps |
|-------|---------------|
| `astraweave-math` | _(none)_ |
| `astraweave-ecs` | profiling |
| `astraweave-core` | ecs, behavior |
| `astraweave-sdk` | core |
| `astraweave-alloc` | _(optional: mimalloc)_ |

> **Reconciliation note (v0.7.0)**: The foundation trace clarifies a structural fact the prior map under-stated: **two `World` types** (`astraweave_ecs::World` archetype substrate vs. `astraweave_core::World` HashMap-per-component legacy struct) and **two `Entity` types** (generational `{id, generation}` vs bare `u32`) coexist and are bridged inside an `astraweave_ecs::App` by `astraweave_core::ecs_adapter::build_app`. See `ecs_math_core_sdk_foundation.md` §1 (status note) and §6 (conflict map).

**AI (orchestration, planning, LLM).** Trace: [`ai_pipeline.md`](ai_pipeline.md) (covers 8 crates with subsystem traces in §13)

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

**Rendering & Assets.** Trace: [`render_pipeline_material_system_shader_infrastructure.md`](render_pipeline_material_system_shader_infrastructure.md), [`terrain_materials.md`](terrain_materials.md), [`animation.md`](animation.md)

| Crate | Workspace Deps |
|-------|---------------|
| `astraweave-materials` | _(none)_ |
| `astraweave-cinematics` | _(none)_ |
| `astraweave-camera` | _(none — glam/winit/optional serde only; canonical `Projection`/`RenderView`/`CameraProducer` types, created Unified Camera C.2 `52b9e711c` 2026-05-18)_ |
| `astraweave-render` | core, materials, cinematics, terrain, profiling, asset, **camera** (non-optional, Cargo.toml:45), **aw_asset_cli** (tool — unusual direction, verified 2026-05-13 still present at Cargo.toml:60) |
| `astraweave-asset` | blend |
| `astraweave-asset-pipeline` | _(none)_ |
| `astraweave-blend` | _(in crates/)_ |

**Impostor Infrastructure (April 16, 2026):**
- **Render modules**: `impostor_bake` (offline/lazy atlas bake), `impostor_lod3` (billboard sampling shader), `impostor_pass` (reusable draw helper)
- **Editor modules**: `impostor_registry` (content-hashed disk cache), `impostor_wiring` (scatter-to-bake bridge)
- **Feature**: `impostor-bake` (in `aw_editor` default features as of April 2026)
- **CLI**: `astraweave-render/src/bin/aw_impostor_bake.rs` (feature-gated, offline atlas generator)

**Physics & World.** Trace: [`physics.md`](physics.md), [`fluids.md`](fluids.md)

| Crate | Workspace Deps |
|-------|---------------|
| `astraweave-nav` | _(none)_ |
| `astraweave-physics` | profiling, ecs (opt), scene (opt) |
| `astraweave-scene` | ecs, asset |
| `astraweave-terrain` | core, **gameplay** (reverse dep — verified 2026-05-13 still present at Cargo.toml:14) |
| `astraweave-fluids` | _(none)_ |

**Gameplay.** Traces touching this domain: [`input.md`](input.md), [`audio.md`](audio.md), [`animation.md`](animation.md)

| Crate | Workspace Deps |
|-------|---------------|
| `astraweave-input` | _(none)_ |
| `astraweave-pcg` | _(none)_ |
| `astraweave-gameplay` | core, physics, nav, ecs, input, scene |
| `astraweave-quests` | llm, rag, context, prompts |
| `astraweave-weaving` | pcg |
| `astraweave-audio` | gameplay |
| `astraweave-ui` | input, gameplay, cinematics |

> **Reconciliation note (v0.7.0)**: `astraweave-gameplay` and `astraweave-ui` both declare `astraweave-input` as a workspace dep, but **neither imports it** — verified by `input.md` §4 and `§11`. The single actual consumer of `astraweave-input` is `examples/ui_controls_demo`, and that demo's logic does not even read the `InputManager` state (it `match`es raw `KeyCode` directly). This is a declared-but-unused-dep pattern; see §5 below.

**Networking.** Traces: [`net.md`](net.md), [`net_ecs.md`](net_ecs.md), [`persistence_ecs.md`](persistence_ecs.md)

| Crate | Workspace Deps |
|-------|---------------|
| `astraweave-net` | core |
| `astraweave-net-ecs` | aw-net-proto, ecs, core |
| `astraweave-persistence-ecs` | aw-save, ecs, core, memory |

> **Reconciliation note (v0.7.0)**: `net_ecs.md` documents that **`astraweave-stress-test` declares both `astraweave-net-ecs` AND `astraweave-persistence-ecs`** (`Cargo.toml:20-21`) but `grep -rn "use astraweave_net_ecs\|use astraweave_persistence_ecs" astraweave-stress-test/` returns no matches. Both deps were added in the same commit (`08befc6ec`, 2025-10-01, "phase 6 implementation"), which also created both crates. They have never been imported. Also: `astraweave-persistence-ecs/Cargo.toml:20` declares `astraweave-memory` as a workspace dep that is never `use`d anywhere in `astraweave-persistence-ecs/src/`. See `persistence_ecs.md` §6 and §11.

**Tools.** Trace: [`aw_editor.md`](aw_editor.md)

| Crate | Workspace Deps |
|-------|---------------|
| `aw_editor` | astract, core, ecs, profiling, **render (REQUIRED, non-optional)**, author, asset, audio, behavior, dialogue, quests, nav, observability, physics, security, terrain, asset-pipeline |

> **Reconciliation note (v0.7.0)**: `aw_editor.md` §1 verified the dep count as **18 direct `astraweave-*` path deps** (not 24+ as the prior map's `EditorApp` description hinted). The editor has 12 Cargo features, 41 `PanelType` variants, 49 panel files, 123 fields on the `EditorApp` god struct, and 216 `.rs` files at ~224,584 LoC. **`astraweave-input` is conspicuously NOT in the editor's dep list** — the editor reimplements the entire input domain in `panels/input_bindings_panel.rs` (2,511 LoC, 13 types). See `input.md` §6 and `aw_editor.md` §6.

### 2.2 Leaf Crates (zero workspace deps)
`math`, `nav`, `materials`, `cinematics`, `asset-pipeline`, `input`, `pcg`, `fluids`

### 2.3 Architectural Anomalies

1. **`terrain` → `gameplay`** (reverse dependency): World-gen crate imports gameplay biome types. Should flow the other way. Verified 2026-05-13 still present. Blast radius: changing gameplay biome types breaks terrain generation.
2. **`render` → `aw_asset_cli`** (production → tool): A runtime rendering crate depending on a CLI tool crate. Verified 2026-05-13 still present at `astraweave-render/Cargo.toml:60`. Unusual dependency direction.
3. **`npc` does NOT depend on `ai` or `behavior`**: NPC crate depends on physics/audio/gameplay instead. NPC behavior logic is decoupled from the AI orchestration layer. Confirmed by `ai_pipeline.md` §13.3 — NPC is "a fully isolated AI subsystem with its own parallel vocabulary (`NpcAction`/`NpcPlan`/`NpcWorldView`/`LlmAdapter`/`CommandSink`), zero imports of canonical AI types, and direct physics/audio integration."
4. **`core` → `behavior`**: Core foundation depends on behavior trees (behavioral is part of the core abstraction).
5. **`aw_editor` → `render` is REQUIRED** (Fix 27): Previously optional (`#[cfg(feature = "astraweave-render")]`), now always compiled. ~30 `#[cfg]` guards removed. Build time increased ~45s but editor always has engine-grade PBR rendering.
6. **`astraweave-stress-test` declared-but-unused deps**: declares both `astraweave-net-ecs` and `astraweave-persistence-ecs`, imports neither. (Surfaced by `net_ecs.md` §1 and `persistence_ecs.md` §6.)
7. **`astraweave-persistence-ecs` declared-but-unused dep on `astraweave-memory`**: dep declared at `Cargo.toml:20`, zero `use astraweave_memory` in `astraweave-persistence-ecs/src/`. (Surfaced by `persistence_ecs.md` §6.)
8. **`astraweave-gameplay` and `astraweave-ui` declared-but-unused deps on `astraweave-input`**: both Cargo.tomls pull in the crate; neither `src/` directory imports it. (Surfaced by `input.md` §4 and §11.)

There are no dependency cycles in the workspace graph as of 2026-05-13. The "reverse" `terrain → gameplay` dep is a directional anomaly (flows backwards conceptually) but not a cycle.

---

## 3. Public API Surface

### Core (Foundation Trace: [`ecs_math_core_sdk_foundation.md`](ecs_math_core_sdk_foundation.md))

**astraweave-ecs**
- Types: `World`, `Entity` (`{id: u32, generation: u32}`), `EntityAllocator`, `CommandBuffer`, `Events`, `EventReader`, `Query`, `Query2`, `Query2Mut`, `Rng`, `TypeRegistry`
- Traits: `Component`, `Resource`, `SystemParam`
- Stages: `PRE_SIMULATION`, `PERCEPTION`, `SIMULATION`, `SYNC`, `AI_PLANNING`, `PHYSICS`, `POST_SIMULATION`, `PRESENTATION` — executed **deterministically on a single thread per tick**.
- Modules: archetype, blob_vec, command_buffer, component_meta, entity_allocator, events, rng, sparse_set, type_registry
- **Note**: `ParallelSchedule` was removed 2026-04-18 (commit `617c14de8`). See `docs/audits/parallel_schedule_removal_2026-04-18.md` and the trace at `ecs_math_core_sdk_foundation.md` §7 (Decision: Deterministic single-threaded ECS).
- **Note (v0.7.0)**: An orphan source file `astraweave-ecs/src/lib_new.rs` (17 lines) is present but not declared in any `pub mod` list. See `ecs_math_core_sdk_foundation.md` §11.

**astraweave-math**
- SIMD batch ops: Vec3/Vec4 dot/cross/normalize, Mat4 multiply/transpose, Quat multiply/normalize/slerp (1.7-2.5× over scalar glam per Week 5-6 measurements)
- Module: simd_vec, simd_mat, simd_quat, simd_movement
- Function: `enable_flush_to_zero()` (sets MXCSR FTZ+DAZ on x86_64)

**astraweave-sdk**
- Types: `Version`, `AWVersion`, `SdkError`
- Trait: `GameAdapter`
- C ABI: `aw_version()`, `aw_world_create()`, `aw_world_destroy()`, `aw_world_tick()`, `aw_world_snapshot_json()`, `aw_world_submit_intent_json()`, plus `aw_world_set_snapshot_callback`, `aw_world_set_delta_callback`
- **Reconciliation note (v0.7.0)**: Per `ecs_math_core_sdk_foundation.md` §11, the SDK has exactly **one in-tree consumer** (`examples/sdk_c_harness`) — the prior map's "otherwise out-of-tree audience" framing is accurate but the in-tree harness exists for exercising `#[link]` + `extern "C"` binding.

**astraweave-alloc**
- Macro: `setup_global_allocator!()` (opt-in mimalloc replacement)
- Feature: `fast-alloc` (disabled by default; binaries opt in)
- Re-export: `MiMalloc` (when `fast-alloc` enabled)
- Purpose: Zero-cost opt-in allocator swap for binaries (library crates unaffected)

**astraweave-core**
- Types: `WorldSnapshot`, `CompanionState`, `PlayerState`, `EnemyState`, `Poi`, `PlanIntent`, `ActionStep`
- **Plus the legacy `astraweave_core::World`** (HashMap-per-component) and its bridge `ecs_adapter::build_app` (mints an `astraweave_ecs::App`, mirrors legacy entities into ECS components, inserts the legacy `World` as a resource, registers `sys_sim`/`sys_move`/`sys_bridge_sync`/`sys_sync_to_legacy`/`sys_refresh_los`)
- Two `Entity` types coexist: `astraweave_core::Entity = u32` (bare numeric) vs. `astraweave_ecs::Entity { id, generation }` (generational). `EntityBridge` is the only safe conversion.
- Bridges ECS ↔ AI via WorldSnapshot serialization

### AI (Trace: [`ai_pipeline.md`](ai_pipeline.md) — 8 crates, ~85K LoC)

**astraweave-ai**
- Types: `AIArbiter`, `AIControlMode` (3 variants: `GOAP`/`ExecutingLLM { step_index }`/`BehaviorTree`), `LlmExecutor`, `AsyncTask`, `VeilweaverCompanionOrchestrator`
- Trait: `Orchestrator` (sync) + `OrchestratorAsync` (async)
- Function: `build_app_with_ai()`
- Modules: core_loop (defines `PlannerMode` enum: Rule/BehaviorTree/GOAP), orchestrator, tool_sandbox, ai_arbiter, llm_executor, goap (advanced — feature-gated `planner_advanced`, 22 files / 16.7K LoC)
- **Reconciliation note (v0.7.0)**: `astraweave-ai` lacks `#![forbid(unsafe_code)]` at lib.rs:1 (unlike its 7 sibling AI crates), but workspace grep verified zero unsafe blocks in the crate. See `ai_pipeline.md` §11 (decisional question).
- **Reconciliation note (v0.7.0)**: Per `ai_pipeline.md` §6, runtime LLM model default is **`phi3:medium`** (`orchestrator.rs:488-490`) despite doc-comments at `ai_arbiter.rs:1` referring to "GOAP+Qwen3 Hybrid Control System". CLAUDE.md's "GOAP+Qwen3 Hybrid" reflects target state, not the runtime default. Set `OLLAMA_MODEL=qwen3:8b` to get documented behavior.

**astraweave-behavior**
- Enum: `BehaviorNode` { Sequence, Selector, Action, Condition, Decorator, Parallel } (`#[non_exhaustive]`)
- Modules: ecs, goap (canonical: `WorldState` is `BTreeMap<u32, bool>` with interned u32 keys; `GoapAction`, `GoapGoal`, `GoapPlanner`), goap_cache (LRU), interner
- **Reconciliation note (v0.7.0)**: This is the **canonical** GOAP implementation. The **advanced** GOAP at `astraweave-ai/src/goap/` (feature `planner_advanced`) is a parallel implementation with no shared Rust types — see `ai_pipeline.md` §6 and §13.6 (Advanced GOAP subsystem trace).

### Rendering (Trace: [`render_pipeline_material_system_shader_infrastructure.md`](render_pipeline_material_system_shader_infrastructure.md))

**astraweave-render**
- Types: `Renderer` (7,809-line aggregator), `Camera`, `CameraController`, `Texture`, `Vertex`, `Mesh`, `MaterialManager`, `MaterialLibrary` (canonical 32-layer terrain), `BindlessMaterialSystem`, `Skeleton`, `AnimationClip`, `JointPalette`, `JointPaletteManager` (pooled SSBO with dynamic offsets), `GBuffer`
- **Impostor types**: `ImpostorBaker`, `ImpostorAtlasSpec`, `ImpostorAtlasSidecar`, `ImpostorPass`, `Lod3Resources`, `Lod3InstanceRaw`, `SpeciesRowGpu`
- 123 source files (~78K LoC) + 71 WGSL shaders
- Modules: clustered lighting, MegaLights (GPU light culling), CSM shadows (per-cascade uniform buffers — see `render_pipeline_material_system_shader_infrastructure.md` §7 Decision: race avoidance), bloom, TAA, SSAO/GTAO, SSGI, SSR, Lumen GI, VXGI (alt path), volumetric fog, god rays, atmosphere, auto-exposure, particle system, terrain materials (32-layer canonical via `terrain_material_manager.rs`), weather, **impostor_bake**, **impostor_lod3**, **impostor_pass**
- Features: PBR, deferred, forward+, lumen GI, GPU particles, decals, water, **impostor-bake**, **skinning-gpu** (GPU skinning), **terrain-splat-arrays**, **textures**, **gpu-particles**, **nanite**
- **Key signature change (Real-Fix.A 2026-05-07)**: `Renderer::draw_into(view, depth_view, encoder)` now accepts `Option<&wgpu::TextureView>` for depth. Editor passes its own depth target. Resolves §7.7 wrapped-component identity at depth-target layer. See `render_pipeline_material_system_shader_infrastructure.md` §6 (Trap) and §7 (Decision).
- Key methods used by editor: `Renderer::new_from_device()`, `draw_into(target, depth_view, encoder)`, `update_camera()`, `add_model()`, `clear_model()`, `has_model()`, `update_instances()`, `create_mesh_from_cpu_mesh()`, `create_mesh_from_arrays()`, `create_mesh_from_full_arrays()`, `set_sky_config()`, `set_weather()`, `set_water_renderer()`, `scene_environment_mut()`, `time_of_day_mut()`, `shadows_enabled()`, `set_shadows_enabled()`, `install_impostor_pass()`, `remove_impostor_pass()`, `impostor_pass_mut()`, `current_view_proj()`, `update_all_impostor_cameras()`
- Key re-exports: `WeatherKind` (via `effects::WeatherKind`), `SkyConfig`, `WaterRenderer`, `TerrainVertex`, `Instance`
- **Reconciliation note (v0.7.0)**: `render_pipeline_material_system_shader_infrastructure.md` §6 documents **three material binding paths coexist intentionally**: `MaterialManager` (TOML-driven legacy at `material.rs:949`), `MaterialLibrary` (canonical 32-layer terrain at `material_library.rs`), `BindlessMaterialSystem` (bindless modern at `material_bindless.rs`). Also notes `types::Material` vs `material_library::Material` are intentionally non-collidable at the crate root (the latter is NOT re-exported per `lib.rs:251-253`).

**astraweave-materials**
- Enum: `Node` { Texture2D, Constant3, Constant1, Multiply, Add, MetallicRoughness, Clearcoat, Anisotropy, Transmission, NormalMap } (`#[non_exhaustive]`)
- Material graph node system (Phase 2 foundation). Single-file crate. Used by `astraweave-render` as `MaterialPackage` import.

### Physics (Trace: [`physics.md`](physics.md))

**astraweave-physics**
- Types: `PhysicsWorld`, `CharacterController`, `SpatialHash` (**dormant** — see below), `ProjectileManager`, `GravityManager`, `Ragdoll`, `Vehicle`, `ClothManager`, `DestructionManager`, `EnvironmentManager`
- Type used by editor viewport: `DebugLine` (debug geometry for physics wireframes, brush cursor, zone overlays)
- Wraps Rapier3D 0.22, re-exports all Rapier types
- Features (verified 2026-05-12 per trace v1.2 — **5 features, not 3**): `async-physics`, `profiling`, `ecs` (`PhysicsPlugin` for `astraweave-ecs`), `alloc-counter` (bench-only — `alloc_measure` bench `required-features`), `fast-alloc` (bench-only — mimalloc allocator swap)
- **Reconciliation note (v0.7.0 — SpatialHash dormancy)**: The crate-level doc-comment at `lib.rs:25-26` advertises `SpatialHash` as the broadphase with "99.96% pair reduction vs brute-force." Actual broadphase is Rapier's `DefaultBroadPhase` (`lib.rs:907`). `SpatialHash` (1,038 LoC) is dormant in production; only test-file consumers (4 test files / 33 `#[test]` attributes; zero benches). See `physics.md` §6 (Trap) and §11.
- **Reconciliation note (v0.7.0 — stub functions)**: `process_destructible_hits` (`lib.rs:1587`) has zero callers workspace-wide (`physics.md` §11). `add_water_aabb` (`lib.rs:1449`) is a no-op stub (function body `{}`, all params underscored). `add_destructible_box`'s `_health`/`_break_impulse` params are ignored.
- **Reconciliation note (v0.7.0)**: `PhysicsWorld` is `Send + Sync` (auto-derived; compile-time-proven by `astraweave-scripting/src/lib.rs:148, 336, 453, 506` using `World::get_resource<PhysicsWorld>` which requires `Send + Sync`). The prior trace claim "Send but NOT Sync" was wrong. See `physics.md` §8 Invariant 18.

**astraweave-fluids**
- 19 source files (~24.2K LoC) after W.1 (2026-06-20) removed the SPH/voxel solver + `simd_ops.rs` (−58.8K LoC). Largest single file now: `editor.rs` at 5,823 LoC; second: `lib.rs` at 4,068 LoC. (`simd_ops.rs` — formerly 39,554 LoC — deleted in W.1.)
- Post-W.1, the SPH/voxel solver is removed; the remaining surfaces are `FluidSystem` (lib.rs PBD remnant) and `WaterEffectsManager` (visual coordinator). `PcisphSystem` (formerly standalone PCISPH) and `simd_ops.rs` were deleted in W.1 (2026-06-20); `UnifiedSolver` was deleted earlier with `unified_solver.rs`; `ResearchFluidSystem` was never a type — `research.rs` is a wgpu-free config/particle module.
- Types: `WaterBuildingManager`, `WaterEffectsManager`, `CausticsSystem`, `WaterQualityPreset` (Low/Medium/High/Ultra/Custom)
- **Reconciliation note (v0.7.0 — runtime dormancy)**: `fluids.md` §1 verified 2026-05-12: the **only** workspace consumer of `astraweave-fluids` outside the crate itself is `examples/fluids_demo/src/main.rs:18-21`. **No production game-loop crate** (`astraweave-render`, `astraweave-gameplay`, `astraweave-physics`, `astraweave-scene`, `astraweave-terrain`, `astraweave-ecs`) depends on `astraweave-fluids`. The audit doc `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md` rates current state at "Grade B (Good for games, insufficient for research)." This was the single largest dormant-code reservoir in the workspace at ~84.5K LoC at v0.7.0; W.1 (2026-06-20) cut it to ~24.2K LoC by removing the SPH/voxel solver.
- **Reconciliation note (v0.7.0 — `editor.rs` not wired)**: The 5,823-LoC `editor.rs` is forward-design infrastructure. Verified 2026-05-12: `tools/aw_editor/Cargo.toml` does not declare `astraweave-fluids` as a dependency; workspace grep for `use astraweave_fluids` inside `tools/aw_editor` returns zero matches. See `fluids.md` §5 file map editor.rs row.
- **Reconciliation note (v0.7.0 — no `forbid(unsafe_code)`)**: Unlike sibling engine crates, `astraweave-fluids/src/lib.rs:1` does NOT declare `#![forbid(unsafe_code)]`. Only two unsafe occurrences crate-wide: `unsafe impl bytemuck::Pod for DebugVertex {}` + `unsafe impl bytemuck::Zeroable for DebugVertex {}` at `debug_viz.rs:479-480`. No `unsafe { ... }` blocks.

### World & Scene

**astraweave-scene** (Trace: [`animation.md`](animation.md) for the ECS animation surface)
- Types: `Transform`, `Node`, `Scene`, `SceneError`
- Modules: gpu_resource_manager, partitioned_scene, streaming, world_partition
- Animation ECS components: `CSkeleton`, `CSkinnedMesh`, `CAnimator`, `CJointMatrices`, `CParentBone` + systems `update_animations`, `compute_poses_stub`, `update_bone_attachments`
- **Reconciliation note (v0.7.0)**: `animation.md` §6 documents `compute_poses_stub` (literal name "_stub"): the function checks for ECS component presence but does not write a full pose. The canonical hierarchical-pose computation is render-side `compute_joint_matrices` (`astraweave-render/src/animation.rs:274-336`). See `animation.md` §11 (decisional question: when does it stop being a stub?).

**astraweave-terrain** (Trace: [`terrain_materials.md`](terrain_materials.md) for the 32-layer material pipeline)
- Types: `Biome`, `BiomeType`, `ChunkManager`, `TerrainChunk`, `Heightmap`, `VoxelGrid`, `WorldConfig`, `ZoneRegistry`
- Climate-field architecture (Phase 1.6-F D.1): `ClimateMap`, `ClimateSample` (real-world units: temperature_c, moisture_mm, continentalness), `WorldArchetype` (climate envelope: means + variances + latitude_temperature_drop_c), `ClimateMap::sample(x, z, elevation) → ClimateSample` per-vertex API.
- Whittaker biome lookup (Phase 1.6-F D.2): `BiomeId` enum with 19 fixed variants (11 terrestrial Whittaker + 5 aquatic + 3 elevation overlays); `lookup_biome(temp_c, moisture_mm, elevation_m) → BiomeId` deterministic four-layer-ordered classifier.
- Per-biome parameter system (Phase 1.6-F D.3): `BiomeParameters` (mountains_amplitude, ridge_strength, runevision_config, erosion_preset, scatter_density, scatter_species_set, surface_color_palette); `BiomeParameters::for_biome(BiomeId)` total over all 19 variants. Replaces legacy `BiomeNoisePreset` (removed D.3c).
- Scattered-convolution biome blending (Phase 1.6-F D.4, NoisePosti.ng-style): `BiomeParamBlendConfig`, `BlendedBiomeParams`, `blend_biome_parameters()` — N jittered samples per vertex with distance-weighted parameter blending and dominant-biome assignment, position-quantized for shared-edge invariance.
- World archetype catalog (Phase 1.6-F D.5): `WorldArchetypeId` enum (6 variants: Continental Temperate, Equatorial Tropical, Boreal/Subarctic, Mediterranean, Desert, Custom); `world_archetypes::all()`, `display_name()`, `description()`, `default_archetype()` for editor consumption.
- 30+ modules: erosion (`advanced_erosion`, `runevision_erosion`), marching cubes, LOD, noise (`noise_gen`, `perlin_gradient`), scatter, climate (`climate`), biome lookup (`biome_lookup`), per-biome parameters (`biome_parameters`), biome param blending (`biome_param_blending`), legacy splat blending (`biome_blending`), world archetypes (`world_archetypes`), blueprint zones, plus the dormant `texture_splatting.rs` (8-layer legacy, test-only — see `terrain_materials.md` §5 and §11).
- Phase 1.6-F (Terrain Generation Quality Campaign) CLOSED VIA ARCHITECTURAL PIVOT 2026-04-29; D.1-D.5 deliverables preserved as within-region machinery for new Phase 1.X (Regional Archetype Variation) campaign at `docs/current/REGIONAL_ARCHETYPE_VARIATION_CAMPAIGN.md`.
- **Reconciliation note (v0.7.0 — material pipeline)**: Per `terrain_materials.md`, the canonical terrain material rendering pipeline lives in `astraweave-render` (32-layer canonical material library), not in `astraweave-terrain` (where the legacy 8-layer `texture_splatting` is test-only). The "biome layer" in `astraweave-terrain` (`elevation_biome.rs`, `biome*.rs`) is **semantically distinct** from the "material layer" in `astraweave-render` (`material_library.rs`, `terrain_material_manager.rs`). Conflating them is the dominant cognitive trap in the terrain/render boundary.

**astraweave-nav**
- Types: `NavMesh`, `NavTri`, `Triangle`, `Aabb`
- A* pathfinding, navmesh baking, dirty region tracking

### Gameplay (Traces touching this domain: [`audio.md`](audio.md), [`animation.md`](animation.md), [`input.md`](input.md))

**astraweave-gameplay**
- 20+ modules: combat, combat_physics, crafting, dialogue, quests, items, stats, biome, harvesting, weaving, weave_portals

**astraweave-audio** (Trace: [`audio.md`](audio.md))
- Types: `AudioEngine`, `DialoguePlayer`, `VoiceBank`, `EmitterId` (u64, **no allocator, no sentinel** — collisions silently merge `SpatialSink`s, see `audio.md` §6 trap), `ListenerPose`
- **5 buses** (not 4 per stale lib.rs:8 docstring — verified by `audio.md` §1 status note): master, music, ambient, voice, SFX
- 3D spatial audio via `SpatialSink` ear-position panning, TTS adapter trait (single-method synth-to-file)
- **Reconciliation note (v0.7.0)**: `AudioEngine` is `!Send + !Sync` (because rodio's `OutputStream` wraps `cpal::Stream` which carries `NotSendSyncAcrossAllPlatforms`). Consequence: cannot be an ECS Resource, cannot be wrapped in `Arc<RwLock<_>>` for cross-thread sharing. The crate has zero `astraweave-ecs` dependency. The editor `AudioPanel` exposes 10+ knobs (HRTF, Doppler, distance model, reverb, crossfade duration, shuffle/loop) whose `AudioAction` variants have **bodyless or comment-only match arms** in the bridge — forward-design UI placeholders. See `audio.md` §6 and §11.

**astraweave-ui**
- Types: `HudManager`, `MenuManager`, `AccessibilitySettings`, `GamepadManager`
- Modules: accessibility, gamepad, hud, layer, menu, panels, persistence, state

---

## 4. Workspace-Wide Structural Axioms

These are not properties of any one subsystem — they're cross-cutting rules that the traces have surfaced as load-bearing. CLAUDE.md elevated several of these from "candidate corollary" to "structural axiom" during recent campaigns.

### 4.1 §7.7 Wrapped-Component Resource Identity (structural axiom)

**Source**: `aw_editor.md` §6 (Multi-Tool Architecture campaign), `render_pipeline_material_system_shader_infrastructure.md` §6 (canonical anti-pattern), CLAUDE.md (the elevated rule).

**The rule**: When component A wraps component B and both manage state of the same logical role (depth target, terrain chunk map, splat buffer, material library, input action queue, audio bus state, animation clip library), **A's reads do not reflect B's writes** unless the wrapper explicitly delegates.

**Confirmed instances (Editor Multi-Tool Architecture Sub-phase 3, 4 layers / 4 fixes)**:

| Layer | Date | Mechanism | Fix |
|---|---|---|---|
| Depth target | 2026-05-07 Round 5 | Editor's `read_depth_at_pixel` sampled local depth texture; engine's `render_to_texture` wrote to its own internal depth | Real-Fix.A `0f569d212` — `Option<&wgpu::TextureView>` depth_view parameter on `Renderer::draw_into` |
| Mesh data | 2026-05-07 Round 6 | Initial chunk upload → live `Renderer::terrain_forward.chunks`; incremental brush update → legacy `terrain_clusters` Vec (no rendering path read it) | Real-Fix.B `eaaa53433` — shared `upload_or_update_terrain_chunk_forward` helper |
| Texture-data attribute set | 2026-05-08 Round 7 | Paint mutated `vertex.material_ids`/`material_weights`; `build_chunk_splat_maps` read only `vertex.biome_weights_0/1` | Real-Fix.C `ded9a0457` — Option C unified `TerrainVertex` into single canonical `material_*` attribute set |
| UI/renderer capacity | 2026-05-08 Round 8 | UI offers 22 materials; splat-build caps at 8; renderer texture array allocates 8 slots | Real-Fix.D 2026-05-08 commit `7067cc03d` — canonical 32-layer material library (`MAX_TERRAIN_LAYERS = 32`, `NUM_SPLAT_MAPS = 8`). See `terrain_materials.md`. |

**Workspace-wide recurrence (non-rendering boundaries)** — same shape at multiple layers:

| Instance | Files | Source trace |
|---|---|---|
| Editor `input_bindings_panel.rs` (2,511 LoC) reinventing astraweave-input | `tools/aw_editor/src/panels/input_bindings_panel.rs` vs `astraweave-input` (editor doesn't depend on it) | `input.md` §6 |
| Editor `AudioPanel` UI knobs (10+ no-op `AudioAction` variants) | `tools/aw_editor/src/audio_bridge.rs:165-205` vs `astraweave-audio::AudioEngine` | `audio.md` §6 |
| Fluids `editor.rs` (5,823 LoC) forward-design surface not wired to `tools/aw_editor` | `astraweave-fluids/src/editor.rs` vs `tools/aw_editor/Cargo.toml` (no dep) | `fluids.md` §5 |
| Four parallel `Skeleton`/`Joint`/`Transform`/`AnimationClip`/`Interpolation` type families | render / asset / scene-CSkeleton / editor-Gltf* | `animation.md` §6 |

**How to apply**: Before adding any wrapper-layer resource, check whether the wrapped layer holds the same logical resource — if so, the wrapper must **delegate**, not duplicate. See `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` §7.7 for the full forensic trail.

### 4.2 No Second Implementation of an Existing Logical System (Fix-27 lesson)

**Source**: `render_pipeline_material_system_shader_infrastructure.md` §7 (Fix 27 campaign), CLAUDE.md Scope Discipline.

**The rule**: Never build a second implementation of a logical system that already exists — rendering path, vertex format, material pipeline, scheduler, tonemap chain, scene serializer.

**Confirmed dual-implementation events** (each cost multi-week cleanup):
- Fix-27 campaign: Editor's `FastPreview` PBR shader + CSM + BRDF LUT + IBL + glTF loader + tonemap (~4,000 LoC) had **diverged across 12 dimensions** from `astraweave-render::Renderer`. Resolved by deletion. `RenderMode::FastPreview` enum residue remains at `tools/aw_editor/src/viewport/engine_adapter.rs:25-28`.
- Dual TerrainVertex: 96-byte editor format with `material_ids[4]`/`material_weights[4]` vs 36-byte engine format with single `biome_id`. Resolved by `terrain_materials.md` §7 unification (Real-Fix.C). `TerrainVertex::to_engine_vertex()` adapter retained but bench-only (`terrain_materials.md` §5).
- Editor-local shadow-quality-layout vs engine-side shadow uniforms. Reconciled into the per-cascade uniform buffer pattern (`renderer.rs:752-757`).
- Dual `RagPipeline` structs: canonical at `astraweave-rag/src/pipeline.rs:21-51` (1693 LoC); orphan at `astraweave-ai/src/rag/pipeline.rs:115` (~360 LoC) — the inner orphan **never compiles** because `pub mod rag` is not declared in `astraweave-ai/src/lib.rs`. See `ai_pipeline.md` §11.

**How to apply**: Before adding any such system, run `rg 'struct <Name>\|trait <Name>'` workspace-wide. If a peer implementation exists, extend it or surface the conflict.

### 4.3 No Silent Failures on User-Facing Result Operations

**Source**: CLAUDE.md Error Handling Policy, `aw_editor.md` §6, `persistence_ecs.md` §6.

**The rule**: Never discard `Result` on user-facing fallible operations (asset I/O, GPU state, file ops, prefab/scene reload, watcher creation). Use `?.context("…")` or a named recovery function. `let _ =` and `.ok()` are forbidden on such calls.

**Why**: Behavioral Correctness Audit (`EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md`) found **12 CRITICAL + 18 HIGH** editor-audit findings where features silently no-op'd. Documented load-bearing silent-failure shapes that survived:

| Shape | Location | Source |
|---|---|---|
| `auto_save_system` body is comment-only TODO | `astraweave-persistence-ecs/src/lib.rs:72-75` | `persistence_ecs.md` §6 |
| `replay_system` advances tick but never applies events | `astraweave-persistence-ecs/src/lib.rs:96` | `persistence_ecs.md` §6 |
| `apply_delta` silent no-op on tick mismatch | `astraweave-net/src/lib.rs:404-406` | `net.md` §6 |
| `process_destructible_hits` no-op stub | `astraweave-physics/src/lib.rs:1587` | `physics.md` §6 |
| `add_water_aabb` no-op stub (function body `{}`) | `astraweave-physics/src/lib.rs:1449` | `physics.md` §6 |
| `compute_poses_stub` checks components but doesn't write poses | `astraweave-scene/src/lib.rs:941` | `animation.md` §6 |
| ~~HMAC server verification always fails (16-byte sig vs 32-byte HMAC-SHA256 OutputSize), failure only `warn!`~~ **RESOLVED** (`561b20957`+`79424389e`+`066cd6cfd`; refute `eb9977b88`) — client+server unified on canonical HMAC-SHA256 (`aw-net-proto`), server verifies FIRST and kicks by default; `sign16`/`SessionKey`/`session_key_hint` deleted | `net/aw-net-server/src/lib.rs` (`on_client_msg{,_tls}`), `net/aw-net-proto/src/lib.rs` | `net_ecs.md` §6/§7; `docs/audits/net_trio_signature_remediation_findings_2026-06.md` |
| `deserialize_ecs_world` on empty blob returns `Ok(())` silently | `astraweave-persistence-ecs/src/lib.rs:447-450` | `persistence_ecs.md` §6 |
| `load_bindings` collapses every error mode to `None` | `astraweave-input/src/save.rs:16-19` | `input.md` §6 |
| `safe_llm_invoke` has zero workspace callers (4096-char limit unreachable) | `astraweave-llm/src/llm_adapter.rs` (file labeled "stub") | `ai_pipeline.md` §6, §13.7 |
| `LLM_CACHE_SIM_THRESH < 1.0` causes test pollution; off by default | `astraweave-llm/src/lib.rs:57-67` | `ai_pipeline.md` §6 |
| Editor `pending_actions` queues on 25 panels but only 7 `take_*_actions` drain functions exist | `tools/aw_editor/src/panels/*.rs` | `aw_editor.md` §6, §11 |

**How to apply**: Run `rg 'let _ = .*[Rr]esult\|let _ = .*\?'` and `rg '\.ok\(\)' --type rust` periodically. Treat each match as a finding requiring justification.

### 4.4 Wired Beats Tested (the dormant-code taxonomy)

**Source**: CLAUDE.md Key Lesson #8, surfaced by every trace's §11. See §5 below for the consolidated inventory.

**The rule**: A subsystem with passing tests and zero production callers is dormant code, not a feature. Tests are necessary but not sufficient — the Integration Completeness checklist (CLAUDE.md) requires (1) production caller, (2) all registration surfaces touched, (3) every UI/API-exposed config field is read, (4) architecture trace current.

---

## 5. Dormant-Code Inventory (workspace-wide)

Reconciled from per-subsystem trace §11 / §13 sections. Each entry: subsystem, LoC scale, evidence.

### 5.1 In-design-but-tested (passes tests, zero production callers)

| Subsystem | LoC | Evidence | Source trace |
|---|---|---|---|
| **Fluids (entire crate)** | ~24.2K (post-W.1; was ~84.5K) | `examples/` are the only workspace consumers; the SPH/voxel solver was removed in W.1 (2026-06-20). No game-loop crate depends on `astraweave-fluids`. | `fluids.md` §1, §11 |
| **Memory pipeline (main)** | ~11K | Zero in-engine production consumers in `astraweave-ai`, `astraweave-behavior`, `astraweave-render`. Only `astraweave-persona` uses the legacy `persona::*` types. | `ai_pipeline.md` §13.1, §11 |
| **Coordination crate** | ~5.3K | Zero workspace consumers (no examples, no tools, no game-loop crates). Explicit `#[allow(dead_code)]` "reserved for future..." markers on 7+ struct/field locations. Three commented-out `pub mod` declarations whose source files never existed. | `ai_pipeline.md` §13.5, §6 |
| **Advanced GOAP** | ~16.7K | 22 files, feature `planner_advanced`. Zero production constructors outside tests + benches + three disabled CLI bins (`bin.disabled/{analyze-plan,validate-goals,visualize-plan}.rs`). | `ai_pipeline.md` §13.6, §11 |
| **LLM Production Hardening** | ~15K | 16 files. Only `parse_llm_response` (and `FallbackOrchestrator::generate_plan`) is actively wired in production. `ProductionHardeningLayer` has no production constructor. The runtime `AIArbiter`/`LlmExecutor` path bypasses the entire hardening surface — no rate limiting, no circuit breaking, no backpressure, no A/B routing, no retry, no telemetry, no ToolGuard, no 4-tier fallback. | `ai_pipeline.md` §13.7, §11 |
| **RAG stack** (`astraweave-rag` + `astraweave-embeddings` + `astraweave-context`) | ~12.3K | Foundation primitives active (`TokenCounter::new` instantiated by `RagPipeline::new` AND `ConversationHistory::new`); `RagPipeline` composite **dormant**: held as a field in 5 LLM-enhanced consumer crates (Director, Quests, Persona, Dialogue, Coordination), all themselves dormant. Plus advertised HNSW vector index is actually a linear scan. | `ai_pipeline.md` §13.8, §11 |
| **Dialogue LLM layer (`llm_dialogue.rs`)** | ~2.9K | 60% of crate. Zero external consumers. The basic `DialogueGraph`/`DialogueRunner`/`toml_loader` path IS actively production-wired into `veilweaver_slice_runtime` and the editor, but the LLM-enhanced layer is dormant. | `ai_pipeline.md` §13.4 |
| **NPC** (isolated subsystem) | ~1.7K | Fully isolated AI subsystem with its own parallel vocabulary (`NpcAction`/`NpcPlan`/`NpcWorldView`/`LlmAdapter`/`CommandSink`). Zero imports of canonical AI types. Direct physics/audio integration rather than ECS-loop participation. Single direct consumer in `astraweave-npc` itself. | `ai_pipeline.md` §13.3 |
| **`astraweave-net-ecs` ECS Plugin layer** | (small) | Working tests, **no production consumer**. The Cargo-declared dep in `astraweave-stress-test/Cargo.toml:20` is not imported in any source file. Async helpers `connect_to_server` / `start_network_server` use `Codec::Bincode` (incompatible with standalone server's `Codec::PostcardLz4`). | `net_ecs.md` §1, §6 |
| **`astraweave-persistence-ecs` ECS layer (the Plugin parts)** | (medium) | Working roundtrip serialization but `auto_save_system` body is comment-only TODO; replay event apply is TODO; `CPersistenceManager::save_game` hardcodes inventory + zero companions; `calculate_world_hash` covers only 4 of 10 components; `SerializedWorld.world_tick` always 0. No production consumer outside crate's own tests. | `persistence_ecs.md` §1, §6 |

### 5.2 Dormant scaffolding (module exists, body is TODO comments)

| Location | Trace |
|---|---|
| `astraweave-persistence-ecs::auto_save_system` body | `persistence_ecs.md` §6 |
| `astraweave-persistence-ecs::replay_system` event apply | `persistence_ecs.md` §6 |
| `astraweave-llm::llm_adapter::safe_llm_invoke` (zero callers; `MAX_PROMPT_LENGTH = 4096` unreachable) | `ai_pipeline.md` §6, §11, §13.7 |
| `astraweave-scene::compute_poses_stub` (checks components but doesn't write poses) | `animation.md` §6 |
| `astraweave-physics::process_destructible_hits` (no-op with `#[allow(dead_code)]`) | `physics.md` §6, §8 (invariant 19) |
| `astraweave-physics::add_water_aabb` (no-op stub) | `physics.md` §6, §8 (invariant 17) |
| `astraweave-physics::add_destructible_box` `_health`/`_break_impulse` params (ignored, underscored) | `physics.md` §6 |
| `astraweave-coordination` 3 commented-out `pub mod` declarations (source files never created) | `ai_pipeline.md` §6 |

### 5.3 Orphan source (file on disk, not declared as a module)

| File | Trace |
|---|---|
| `astraweave-net-ecs/src/lib_temp.rs` (436-line near-duplicate of `lib.rs`) | `net_ecs.md` §5, §6 |
| `archive/temp_files/temp/temp_lib.rs` (third copy of net-ecs lib.rs surface) | `net_ecs.md` §6 |
| `astraweave-ai/src/rag/` (second `RagPipeline` struct never declared as module) | `ai_pipeline.md` §11 (parallel-implementation drift) |
| `astraweave-ai/src/persona/` (orphan persona manager files) | `ai_pipeline.md` §11 |
| `astraweave-ecs/src/lib_new.rs` (17-line stub never wired) | `ecs_math_core_sdk_foundation.md` §6, §11 |

### 5.4 Declared-but-unused Cargo deps (listed in Cargo.toml, zero `use` statements)

| Consumer | Declared dep | Trace |
|---|---|---|
| `astraweave-stress-test` | `astraweave-net-ecs` | `net_ecs.md` §1, §6 |
| `astraweave-stress-test` | `astraweave-persistence-ecs` | `persistence_ecs.md` §6 |
| `astraweave-persistence-ecs` | `astraweave-memory` | `persistence_ecs.md` §6 |
| `astraweave-gameplay` | `astraweave-input` | `input.md` §4, §11 |
| `astraweave-ui` | `astraweave-input` | `input.md` §4, §11 |
| `examples/veilweaver_demo` | `astraweave-audio` | `audio.md` §6, §11 |

### 5.5 Dormant feature flags (gate zero `#[cfg(feature = "X")]` sites)

Editor `editor-graphs`, `editor-materials`, `editor-terrain`, `editor-nav`, `editor-sim`, `editor-full` — verified 2026-05-12 in `aw_editor.md` §6: `grep -rn "editor-graphs\|editor-materials\|editor-terrain\|editor-nav\|editor-sim\|editor-full"` across `tools/aw_editor/` returns **no `.rs` file hits**. The flags exist only in `Cargo.toml`.

### 5.6 Aspirational-doc-only types (referenced in `docs/src/` but not in any `pub use`)

Covered in detail in §7 (Documentation Hazards) below. Origin: commit `28bc94f21` (2025-09-08, "Create comprehensive bespoke wiki with 51-section documentation structure (#34)") authored by GitHub Copilot bot.

---

## 6. Integration Seams

### 6.1 Shared Types (High Blast Radius)

| Type | Defined In | Consumed By | Risk | Source trace |
|------|-----------|-------------|------|---|
| `WorldSnapshot` | astraweave-core | ai, llm, director, memory, examples | **CRITICAL** — field names hard-coded in LLM prompts at three serializers (`prompts.rs:209+`, `compression.rs:139+`, `prompt_template.rs:227-251`); renaming breaks both Rust AND AI behavior; no build-time check ties Rust field names to JSON output keys | `ecs_math_core_sdk_foundation.md` §8 (Invariant 8), `ai_pipeline.md` §6 |
| `CompanionState` | astraweave-core | ai, llm, examples | HIGH — LLM prompt dependency | `ai_pipeline.md` §6 |
| `PlanIntent` / `ActionStep` | astraweave-core | ai, tool_sandbox, examples | HIGH — only `validate_and_execute` actually mutates `World` for AI; this is the **engine-side sandbox boundary**, where AI proposes but engine disposes | `ai_pipeline.md` §6, §7 (Decision 7) |
| `Entity` (ECS) | astraweave-ecs | core, ai, scene, gameplay, physics, net-ecs, persistence-ecs, editor | HIGH — different from legacy `core::Entity = u32`; `EntityBridge` is the only safe conversion point | `ecs_math_core_sdk_foundation.md` §6 |
| `World` (ECS substrate) | astraweave-ecs | nearly everything | CRITICAL — coexists with legacy `astraweave_core::World`; both alive simultaneously and bridged in `build_app` | `ecs_math_core_sdk_foundation.md` §1, §6 |
| `BehaviorNode` | astraweave-behavior | core, ai, editor | MEDIUM — `#[non_exhaustive]`; adding a variant requires touching every dispatch site per Integration Completeness #2 | `ai_pipeline.md` §6 |
| `BiomeType` | astraweave-gameplay | terrain, render, scatter | HIGH — terrain reverse-dep flows backwards through this type | `terrain_materials.md` §4 |
| `Transform` / `Node` | astraweave-scene | render, editor, physics | HIGH — `Transform` is also one of three coexisting Transform types in the animation type families (render/asset/scene) | `animation.md` §6 |
| `NavMesh` | astraweave-nav | ai, gameplay, terrain | MEDIUM | `physics.md` §4 |
| `PhysicsWorld` | astraweave-physics | gameplay, npc, editor, scripting | MEDIUM — `Send + Sync` (verified 2026-05-12 — auto-derived); production consumers take `&mut PhysicsWorld` directly via `World::get_resource_mut` | `physics.md` §8 (Invariant 18) |
| `AudioEngine` | astraweave-audio | editor (via bridge), demos, NPC | MEDIUM — **`!Send + !Sync`** (rodio/cpal `OutputStream` chain); cannot be ECS Resource; consumers must hold directly | `audio.md` §7 |
| `Skeleton`/`Joint`/`Transform`/`AnimationClip`/`AnimationChannel`/`Interpolation`/`ChannelData` (four parallel families) | render / asset / scene-CSkeleton / editor-Gltf* | each crate independently | HIGH — no `From`/`Into` impls between any pair; conversions are ad-hoc at call sites | `animation.md` §6 |
| `TerrainVertex` (two formats) | render (legacy single-`biome_id`, 36 B) vs editor (canonical 96 B with `material_ids[4]`/`material_weights[4]`) | each independently | HIGH — `TerrainVertex::to_engine_vertex()` adapter is bench-only after Real-Fix.C unification; lossy collapse retained for legacy `terrain.rs` path | `terrain_materials.md` §5, §11 |
| `Material` (collision) | `types::Material` (re-exported at `astraweave-render::lib.rs:147`) vs `material_library::Material` (intentionally NOT re-exported per `lib.rs:251-253`) | render-internal | LOW — collision is documented and prevented by non-re-export comment | `render_pipeline_material_system_shader_infrastructure.md` §6 |
| `Renderer::draw_into` signature (Real-Fix.A) | `astraweave_render::Renderer` | aw_editor `EngineRenderAdapter` | HIGH — added `Option<&wgpu::TextureView>` depth_view 2026-05-07 (commit `0f569d212`); editor passes its own depth target so engine terrain writes depth into it; resolves §7.7 at depth-target layer | `render_pipeline_material_system_shader_infrastructure.md` §7 (Decision), `aw_editor.md` §6 |
| `EntityState` (collision) | `astraweave-net::EntityState { pos: IVec2, hp, team, ammo }` vs `astraweave-net-ecs::EntityState { position: Vec3, health }` | each independently | MEDIUM — two networking subsystems with disjoint data models; neither imports the other | `net.md` §6, `net_ecs.md` §6 |
| `Snapshot` / `ReplayEvent` (collisions) | `astraweave-net::Snapshot`, `aw-net-proto::ServerToClient::Snapshot`, `astraweave-net-ecs::NetworkSnapshot`, plus `astraweave-net::ReplayEvent` vs `astraweave-persistence-ecs::ReplayEvent` (different fields) | each independently | MEDIUM — qualify imports | `net.md` §6, `net_ecs.md` §6, `persistence_ecs.md` §6 |

### 6.2 Cross-Crate Trait Implementations

| Trait | Defined In | Implemented By |
|-------|-----------|----------------|
| `Component` (blanket) | ecs | Any `T: 'static + Send + Sync` |
| `Resource` (blanket) | ecs | Any `T: 'static + Send + Sync` |
| `Orchestrator` / `OrchestratorAsync` | ai | GOAP (canonical), BT, LLM orchestrators, RuleOrchestrator, VeilweaverCompanionOrchestrator, `astraweave-ai::goap::GOAPOrchestrator` adapter (advanced GOAP) |
| `GameAdapter` | sdk | User game implementations |
| `Plugin` | ecs | `PersistencePlugin`, `NetworkClientPlugin`, `NetworkServerPlugin`, `PhysicsPlugin` (feature `ecs`), `AiPlanningPlugin` |
| `EditorPlugin` | aw_editor (separate from `ecs::Plugin`!) | Editor extension types via `PluginManager` |
| `Panel` | aw_editor (`panels/mod.rs`) | 49 panel structs |
| `ActiveTool` | aw_editor (Multi-Tool Architecture campaign) | Brush tools |

### 6.3 Event Channels

- `astraweave-ecs::Events<T>` — typed event bus, used across physics (CollisionEvent, ContactForceEvent), gameplay, AI
- `astraweave-core::ecs_events` — bridge events between ECS and legacy `World`, e.g. `MovedEvent`. **Distinct type** from `astraweave_ecs::Events<E>` — see `ecs_math_core_sdk_foundation.md` §6.
- `astraweave-physics` crossbeam channels: `collision_recv`, `contact_force_recv` — **unbounded; drain every tick** or they accumulate forever (`physics.md` Appendix A item 8).

### 6.4 System Stage Registration

All systems register into the 8-stage pipeline via `app.add_system(SystemStage::X, fn)`. **Registering on an unknown stage name silently drops the system in release builds** (debug builds log a warning). See `ecs_math_core_sdk_foundation.md` §6 trap. Cross-crate system registration happens in examples and the editor, where multiple crates' systems are wired into a single `World`.

### 6.5 Editor↔Engine Rendering Boundary (Fix 27 + Multi-Tool Architecture Sub-phase 3)

The most critical integration seam after the Fix 27 campaign. All data flowing from the editor to the engine renderer:

| Data | Editor Type | Engine Type | Conversion | File |
|------|-------------|-------------|------------|------|
| Camera | `OrbitCamera` | `astraweave_render::camera::Camera` | `OrbitCamera::to_engine_camera()` — copies yaw, pitch, fov, aspect, znear, zfar | `viewport/camera.rs:567` |
| Entity pose | `astraweave_core::World::pose()` → `Pose` (with `float_x`, `float_z`, `height`, `scale`, `scale_y`, `scale_z`, `rotation`, `rotation_x`, `rotation_z`) | `astraweave_render::Instance` with `glam::Mat4` transform | Constructed via `Mat4::from_scale_rotation_translation(Vec3(scale, scale_y, scale_z), Quat::from_euler(XYZ, rot_x, rot, rot_z), Vec3(x, h, z))` | `engine_adapter.rs:160-213` |
| Entity selection | `Vec<astraweave_core::Entity>` | Per-instance `color: [f32; 4]` (orange `[1.0, 0.6, 0.2, 1.0]`) | Applied when `selected_entities.contains(&entity)` | `engine_adapter.rs:195-202` |
| Terrain chunks | `Vec<(Vec<TerrainVertex>, Vec<u32>)>` — editor `TerrainVertex` is 96 bytes (pos+norm+uv + `material_ids[4]` + `material_weights[4]`, post Real-Fix.C unification) | `astraweave_render::TerrainVertex` — 36 bytes (pos+norm+uv+biome_id) — **legacy path** | `TerrainVertex::to_engine_vertex()` — bench-only after Real-Fix.C; production splat-bake path routes through `terrain_splat_builder.rs` → `terrain_material_manager.rs` | `engine_adapter.rs:396-453`, `viewport/types.rs:35-63`, `terrain_materials.md` §5 |
| Scatter placements | `Vec<ScatterPlacement>` (pos+scale+rot+tint+mesh_key) | `astraweave_render::Instance` per placement | `Mat4::from_scale_rotation_translation(splat(p.scale), Quat::from_rotation_y(p.rotation), p.position)` | `engine_adapter.rs:485-573` |
| Fog parameters | `TerrainFogParams` | `astraweave_render::scene_environment::SceneEnvironment` | `env.visuals.fog_density`, `env.visuals.fog_color` | `engine_adapter.rs:630-638` |
| Lighting parameters | `TerrainLightingParams` | `SceneEnvironment` | `env.visuals.ambient_color`, `env.visuals.ambient_intensity` | `engine_adapter.rs:641-646` |
| Sky config | `astraweave_render::SkyConfig` | `astraweave_render::SkyConfig` | Direct pass-through | `engine_adapter.rs:597-605` |
| Weather | `WeatherKind` (editor enum, 6 variants) | `astraweave_render::WeatherKind` (engine enum) | Passed directly after mapping from 11-type `world_panel` weather via `WeatherKind::from_world_panel()` | `engine_adapter.rs:607-610`, `viewport/types.rs:154-165` |
| Water style | `WaterStyle` { Ocean, River, Lake, Swamp } | `astraweave_render::WaterRenderer` | Color presets applied during `WaterRenderer::new()` then `renderer.set_water_renderer()` | `engine_adapter.rs:649-686` |
| **Depth target (Real-Fix.A)** | Editor's own `wgpu::TextureView` | Passed into `Renderer::draw_into(view, depth_view, encoder)` as `Option<&wgpu::TextureView>` | Editor depth view shared with engine terrain depth writes | `viewport/renderer.rs`, `render_pipeline_material_system_shader_infrastructure.md` §6 |

**Format mismatch (legacy path)**: Editor `TerrainVertex` is 96 bytes; engine `terrain.rs` is 36 bytes. The `to_engine_vertex()` conversion is lossy — biome blend information (4 slots) is collapsed to the single dominant biome, and per-vertex `material_ids` are discarded. **This was reclassified post-Real-Fix.C**: the canonical render path (32-layer splat material) bypasses this adapter entirely. The legacy path is preserved only for `examples/weaving_playground`. See `terrain_materials.md` §5.

---

## 7. Documentation Hazards (workspace-wide consolidated inventory)

Per-trace `§6` sections surfaced a coordinated set of doc-vs-code drift. The canonical origin (for most aspirational-only docs) is commit **`28bc94f21`** (2025-09-08, "Create comprehensive bespoke wiki with 51-section documentation structure (#34)") authored by **GitHub Copilot bot**. The commit added ~80 doc files in one sweep with no corresponding code changes.

Treat `docs/src/` content as **historical/aspirational** unless cross-validated against actual `pub use` re-exports in the relevant crate's `lib.rs`. The architecture traces in `docs/architecture/` are the falsification mechanism.

### 7.1 Aspirational-doc-only types (referenced in `docs/src/`, absent from `pub use` re-exports)

| Doc file | Claimed types | Reality | Source trace |
|---|---|---|---|
| `docs/src/core-systems/audio.md` + `docs/src/api/audio.md` | `AudioConfig`, `AudioBackend`, `AudioListener`, `SpatialSound`, `AttenuationModel`, `ReverbZone`, `AudioOcclusion`, `MusicManager`, `MusicLayer`, `SfxManager`, `SoundPool`, `AudioMixer`, `Bus` | **None exist** in `astraweave-audio/src/lib.rs` re-exports. The actual API is ~30 lines of `lib.rs` + the source it re-exports (`AudioEngine`, `DialoguePlayer`, `VoiceBank`, `EmitterId`, `ListenerPose`). | `audio.md` §6 |
| `docs/src/core-systems/input.md` | `InputSystem`, `InputConfig`, `ActionMap`, `BindingRecorder`, `BindingProfile`, `ContextPriority`, `InputBuffer`, `InputPredictor`, `InputRecorder`, `mapping::ActionMap`, `rebinding::BindingRecorder`, `replay::InputRecorder`, `buffer::InputBuffer`, `device::MouseButton` | **None exist** in `astraweave-input/src/lib.rs` re-exports. Actual API is `Action`, `Binding`, `BindingSet`, `InputManager`, `Axis2`. | `input.md` §6 |
| `docs/src/core-systems/networking.md` | Claims **QUIC (via Quinn)** transport with "UDP-based reliable/unreliable messaging," 0-RTT, multiplexed. Plus submodules `replication`, `state`, `delta`, `serialization`, `prediction` and types `Server`/`ServerConfig`/`Client`/`ClientConfig`/`ClientEvent`. | Actual `astraweave-net` uses **WebSocket over TCP** via `tokio-tungstenite` (`Cargo.toml:18-19`, `lib.rs:596`). **No `quinn` dependency exists**. Architectural-class mismatch (TCP+WS vs UDP+QUIC), not just type-name mismatch. None of the submodules exist. | `net.md` §6 |

### 7.2 Doc-comment migration drift (target state ahead of runtime wiring)

| Claim | Doc location | Reality | Source trace |
|---|---|---|---|
| AI runtime model is Qwen3 | `astraweave-ai/src/ai_arbiter.rs:1` doc-comment ("GOAP+Qwen3 Hybrid Control System"); CLAUDE.md ("GOAP+Qwen3 Hybrid") | `astraweave-ai/src/orchestrator.rs:488-490` defaults `OLLAMA_MODEL` to `"phi3:medium"` (`unwrap_or_else(\|_\| "phi3:medium".to_string())`). Three Ollama clients (Phi3, Hermes2Pro, Qwen3) coexist. Set `OLLAMA_MODEL=qwen3:8b` to get documented behavior. | `ai_pipeline.md` §6, §11 |
| ~~Networking uses HMAC signatures with XOR `sign16` as MVP~~ **RESOLVED** | `net/README.md` (updated W.4) | Client and server are now unified on canonical HMAC-SHA256 signing (`aw-net-proto`); the client signs and the server verifies (constant-time) over `input_frame_sig_payload`, kicking by default on failure. `sign16` deleted. Net-Trio-Remediation `561b20957`+`79424389e`+`066cd6cfd`. | `net_ecs.md` §6/§7 |
| HNSW vector index for embeddings | `astraweave-embeddings/src/lib.rs:9` advertises HNSW with `hnsw_rs` dependency declared, feature default-on | Actual `VectorStore::search` is a **linear scan over a DashMap**. | CLAUDE.md, `ai_pipeline.md` §13.8 |
| `SpatialHash` is the physics broadphase | `astraweave-physics/src/lib.rs:25-26` doc-comment advertises "99.96% pair reduction" | Actual broadphase is Rapier's `DefaultBroadPhase`. `SpatialHash` (1,038 LoC) is dormant. | `physics.md` §1, §6 |
| Audio crate is "4-bus mixer (master, music, SFX, voice)" | `astraweave-audio/src/lib.rs:8` docstring | Actual `AudioEngine` has **5** buses (ambient added separately). Doc string drifted; engine grew the 5th bus in commit `745c100a8` alongside biome materials. | `audio.md` §6 |
| `auto_save_system` runs autosaves | `astraweave-persistence-ecs::PersistencePlugin` registers it | Body is comment-only TODO; produces zero saves. | `persistence_ecs.md` §6 |
| `replay_system` applies events | Same | Advances `current_tick` counter only; never applies the `events` field. | `persistence_ecs.md` §6 |
| `RagPipeline` supports HNSW + memory consolidation | `astraweave-rag/src/lib.rs` | HNSW is linear-scan; `VectorStoreWrapper::get_all_memories` returns empty Vec making consolidation a no-op. | `ai_pipeline.md` §13.8 |
| `compute_poses` is the ECS pose-compute system | `astraweave-scene/src/lib.rs` | Function is named `compute_poses_stub`; checks for components but doesn't write poses. Canonical compute is render-side `compute_joint_matrices`. | `animation.md` §6, §11 |
| AudioPanel HRTF/Doppler/distance-model/reverb knobs work | `tools/aw_editor/src/panels/audio_panel.rs` | 10+ `AudioAction` variants have bodyless or comment-only match arms in `audio_bridge.rs:165-205`. Forward-design UI placeholders. | `audio.md` §6, §11 |
| `pan_mode` enum changes spatial audio behavior | `astraweave-audio::AudioEngine` | Field is stored and updated but **never read** by any other method in the crate. Spatial sinks already created continue using rodio's spatial panning regardless. | `audio.md` §6 |

### 7.3 Other documentation hazards

- **5 `.awsv` files + `index.json` checked into `persistence/aw-save/`** with 2026-03-13 timestamps. Per `persistence_ecs.md` §11, these were committed in `c9ed24c0c` (commit title "Add input, materials, and PCG scans; implement save file structure" — unrelated to file content). Not referenced by name in any test loader; incidental test-run outputs. Plus `astraweave-persistence-ecs/savegame.bin` (2 bytes) at the crate root.
- **`docs/src/` content generally**: Per `audio.md` §6 and `input.md` §6, the `28bc94f21` sweep was bulk AI-generated structural placeholders, not a record of prior implementation. The architecture traces in `docs/architecture/` are the active falsification mechanism.

When a doc-comment describes desired behavior, treat it as a hypothesis to verify against the code path, not as ground truth.

---

## 8. Data Flow Paths

### 8.1 AI Loop (Trace: [`ai_pipeline.md`](ai_pipeline.md) §2.1)

```
World state
  → build_ai_snapshots() [PERCEPTION stage, astraweave-ai/src/core_loop.rs]
  → WorldSnapshot [astraweave-core]
  → AIArbiter.update() [astraweave-ai/src/ai_arbiter.rs]
  → mode decision: GOAP | BehaviorTree | ExecutingLLM { step_index }
  → Orchestrator.plan() → PlanIntent [astraweave-core]
  → tool_sandbox validation [astraweave-ai/src/tool_sandbox.rs]
  → engine-side validate_and_execute [astraweave-core/src/validation.rs]
  → ActionStep execution → World mutation (only mutation point for AI)
```

Three control-mode enums coexist (`PlannerMode` for per-entity dispatch on `CAiController`; `AIControlMode` for arbiter state; `FallbackTier` for LLM degradation). 4-tier fallback chain (`FullLlm` → `SimplifiedLlm` → `Heuristic` → `Emergency`) but the runtime path goes through `AIArbiter` and bypasses most production-hardening primitives. See `ai_pipeline.md` §13.7 for the dormancy detail.

### 8.2 Render Pipeline (Trace: [`render_pipeline_material_system_shader_infrastructure.md`](render_pipeline_material_system_shader_infrastructure.md) §2)

```
Scene (Transform, Mesh, Material)
  → Renderer::render() OR Renderer::draw_into(view, depth_view, encoder)
    [astraweave-render/src/renderer.rs]
  → Stage R0: Surface acquire
  → Stage R1: GPU profiler + staging ring begin_frame
  → Stage R2: Clustered light bin [clustered.rs, clustered_forward.rs]
  → Stage R3: Shadow cascades (CSM, per-cascade uniform buffers to avoid queue.write_buffer race)
              [shadow_csm.rs, shadow_quality.rs, shaders/shadow_sampling.wgsl]
  → Stage R4: Main scene pass (forward+ clustered)
              [renderer.rs inline SHADER_SRC; bind groups 0=camera, 1=material, 2=shadow, 3=PBR textures]
  → Stage R5: Post-FX chain (HDR, bloom, GTAO, SSGI, SSR, TAA, DOF, motion blur, tonemap)
              [hdr_pipeline.rs orchestrator]
  → Stage R6: Composite + present [post_pipeline + hdr_blit_pipeline]
```

A declarative `frame_graph.rs` DAG scaffolding exists for topology validation but **pass nodes delegate command recording back to `Renderer` methods** — frame graph is not yet command-driving (`frame_graph.rs:18-25` migration status comment). See `render_pipeline_material_system_shader_infrastructure.md` §6 trap.

### 8.3 Terrain Material Pipeline (Trace: [`terrain_materials.md`](terrain_materials.md) §2)

```
[CPU authoring / editor: TerrainVertex with material_ids[4], material_weights[4]]
  → tools/aw_editor/src/viewport/types.rs
  → build_chunk_splat_maps(vertices, w, h)
  → tools/aw_editor/src/viewport/terrain_splat_builder.rs
  → set_chunk_splat_forward(chunk, splats, dims)
  → astraweave-render/src/terrain_material_manager.rs (GPU upload + binding)
  → draw_chunk_forward(...)
  → astraweave-render/shaders/pbr_terrain_vs.wgsl (vertex pass)
  → astraweave-render/shaders/pbr_terrain.wgsl (fragment material blend)
  → Final shaded terrain pixel
```

The canonical 32-layer pipeline is structurally clean. Legacy 8-layer `texture_splatting.rs` and simple-biome-id `terrain.rs` coexist but are dormant. See `terrain_materials.md` §5 status definitions.

### 8.4 Animation Pipeline (Trace: [`animation.md`](animation.md) §2)

```
[A: glTF asset file]
  → astraweave_asset::load_skeleton / load_animations (asset-side Skeleton/Joint/Transform/AnimationClip with float arrays)
  → [B: Conversion to runtime types — caller site, no canonical conversion function]
  → astraweave_render::animation types (glam-based Skeleton/Joint/Transform/AnimationClip)
  → [C: ECS component spawn] CSkeleton + CAnimator + CJointMatrices + CParentBone [astraweave-scene]
  → update_animations(world, dt, clip_durations) [per-tick]
  → AnimationClip::sample(time, skeleton) → Vec<Transform>
  → compute_joint_matrices(skeleton, local_transforms) → Vec<Mat4>
  → CPU path: skin_vertex_cpu(...) [deterministic CI-safe default]
  → GPU path: JointPaletteManager::upload_matrices → SKINNING_GPU_SHADER [feature skinning-gpu]
```

Four parallel type families with no shared types and no `From` impls. `MAX_JOINTS = 256` hard-coded in two places. CubicSpline falls back to Linear/Slerp. See `animation.md` §6 and §11.

### 8.5 Physics Pipeline (Trace: [`physics.md`](physics.md) §2)

```
Forces / character input
  → PhysicsWorld.step() [astraweave-physics, wraps Rapier3D 0.22]
  → step_internal:
    → apply_buoyancy_forces() (BEFORE Rapier step — per-body forces from buoyancy_bodies HashMap)
    → pipeline.step() — Rapier-native integration + DefaultBroadPhase + narrowphase + solver + CCD
    → query_pipeline.update(&self.colliders) (AFTER Rapier step — "Week 2 Day 3 fix")
  → CollisionEvent / ContactForceEvent → crossbeam channels (UNBOUNDED — drain every tick)
  → CharacterController resolution via control_character(id, desired_move, dt) [character_controller.rs]
  → Subsystems (independent, mostly not wired into step): Ragdoll, Vehicle, Cloth, Destruction, Fluids
```

`SpatialHash` is advertised in lib.rs doc-comment but **dormant** — production broadphase is Rapier's `DefaultBroadPhase`. See `physics.md` §6 trap.

### 8.6 Asset Pipeline

```
Source files (.blend, .gltf, textures)
  → BlendImporter / decomposer [astraweave-blend/src/decomposer.rs]
  → Texture processor (HDR→PNG, thumbnails) [texture_processor.rs]
  → BiomePack bridge [astraweave-terrain/src/biome_pack.rs]
  → MaterialManager (TOML → GPU D2 arrays) [astraweave-render — three coexisting paths]
  → Runtime: cell_loader, mesh_obj/mesh_gltf, texture_streaming
```

### 8.7 Persistence Pipeline (Trace: [`persistence_ecs.md`](persistence_ecs.md) §2)

```
ECS World
  → serialize_ecs_world(&world) (persistence-ecs/lib.rs:278-366)
  → 10 separate Query<C> passes for entity discovery (CPos/CHealth/CTeam/CAmmo/CCooldowns/CDesiredPos/CAiAgent/CPersona/CMemory, plus CLegacyId)
  → SerializedEntity { entity_raw: u64, plus 10 Option<C> fields }
  → SerializedWorld { entities, world_tick: 0 (HARDCODED — TODO) }
  → postcard::to_allocvec → Vec<u8> ECS blob
  → CPersistenceManager::save_game(slot, world_tick, world_hash, ecs_blob)
    → SaveBundleV2 (with hardcoded inventory: credits=1000, items=[], companions=[])
  → SaveManager::save → write_awsv(path, &bundle)
    → postcard → lz4 → CRC32 → atomic write (tmp + sync_all + rename)
  → On-disk .awsv file
```

The `aw-save` layer (file format) is production-grade. The `astraweave-persistence-ecs` layer (ECS Plugin) is partly stub. See `persistence_ecs.md` §6 and §11.

### 8.8 Network Pipeline (Traces: [`net.md`](net.md) §2 and [`net_ecs.md`](net_ecs.md) §2)

Two coexisting subsystems with **disjoint data models, wire formats, and integration patterns**. Neither imports the other.

**`astraweave-net` (snapshot-based)**:

```
World (grid-based, IVec2 positions)
  → build_snapshot(world, tick, seq) → Snapshot
  → 60 Hz fixed tick on GameServer; broadcast cadence: full every 60 ticks, delta every 3 ticks
  → filter_snapshot_for_viewer(head, &interest, viewer) [4 Interest impls: Full/RadiusTeam/Fov/FovLos]
  → diff_snapshots(base, head, FullInterest, viewer) → Delta
  → JSON over WebSocket text frames (Msg enum with #[serde(tag="type")])
```

**`astraweave-net-ecs` + standalone trio (`aw-net-{proto,client,server}`)**:

```
ECS World (Vec3 positions)
  → server_snapshot_system OR standalone server build_snapshot
  → ClientToServer/ServerToClient enums via Codec::PostcardLz4 (standalone) or Codec::Bincode (ECS Plugin)
  → WebSocket over TLS (wss://) [standalone] or plain TCP (ws://) [ECS Plugin]
  → Matchmaking: room cap 4, tick_hz 30 (hardcoded); shared 32-byte SigningKey (AW_SHARED_KEY / --shared-key-hex / dev_default)
  → InputFrame { seq, tick_ms, input_blob, sig: [u8; 32] }
    → Client signs HMAC-SHA256 over input_frame_sig_payload; Server verifies FIRST (constant-time), kicks by default (Close 1008) per SignatureFailurePolicy
  → Token-bucket rate limit: 8 tokens/sec refill, 60-bucket capacity, 1 cost/msg
```

### 8.9 Editor Command Flow (Trace: [`aw_editor.md`](aw_editor.md) §2)

```
User input (mouse, keyboard, gamepad via winit → eframe)
  → EditorApp::update(ctx, frame) [main.rs:9119-9120]
  → tick_audio_subsystem + tick_animation_subsystem + tick_movement_scripts (play mode only)
  → process_pending_hotkeys + process_file_watcher_events + auto-save check
  → MenuBar / StatusBar / Dialogs
  → DockArea::new(&mut dock_state).show(ctx, &mut tab_viewer)
  → Per-panel ui() dispatch via egui_dock::TabViewer (41 PanelType variants → 49 panels)
  → Panel emits PanelEvent values onto a queue + may push EditorCommand onto UndoStack
  → Viewport: ViewportRenderer.render() via EngineRenderAdapter::render_to_texture
    → astraweave_render::Renderer::draw_into(view, depth_view, encoder)
  → Panel updates (asset browser, terrain, profiler, etc.)
```

`UndoStack` is bounded (default 100 commands). 9 entity-creation operations were flagged in Behavioral Correctness Audit as bypassing the undo stack — Wave-3 mutation remediation closed several. 18 `pending_actions` queues exist on panels but only 7 `take_*_actions` drain functions are defined — most user button presses queue events that no code ever reads.

---

## 9. Editor Viewport Architecture (Unified Pipeline — Fix 27)

(Largely unchanged from v0.6.0 — see `aw_editor.md` for the architectural campaigns currently in flight.)

### 9.1 Module Structure

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
| `tools/aw_editor/src/viewport/terrain_splat.rs` | Terrain material splat pipeline (superseded by `Renderer::terrain_forward` per `engine_adapter.rs:6-10`) | ~medium |
| `tools/aw_editor/src/viewport/terrain_splat_builder.rs` | Splat texture builder | ~medium |
| `tools/aw_editor/src/viewport/terrain_biome_placeholder.rs` | Biome placeholder rendering | ~small |
| `tools/aw_editor/src/viewport/impostor_registry.rs` | Content-hashed impostor atlas cache | ~340 |
| `tools/aw_editor/src/viewport/impostor_wiring.rs` | Scatter-to-bake bridge helpers | ~280 |
| ~~`tools/aw_editor/src/viewport/shaders/tonemap.wgsl`~~ | **DELETED 2026-05-17** (Render Parity P.3, `e09703538`) — editor now tonemaps through the engine's shared ACES `post_pipeline`; multi-operator authoring (PBR Neutral/Reinhard UI) removed per P.0 Q3 | — |
| `tools/aw_editor/src/viewport/shaders/grid.wgsl` | Floor grid with axes, anti-aliased lines | ~small |
| `tools/aw_editor/src/viewport/shaders/gizmo.wgsl` | Transform handle geometry | ~small |

**Files DELETED in the Fix 27 campaign** (per `aw_editor.md` §5 and Section 12 below):

| Deleted File | What It Was |
|-------------|------------|
| `tools/aw_editor/src/viewport/entity_renderer.rs` | ~3,600 LOC — editor's own PBR shader, shadow system, BRDF LUT, IBL computation, glTF/texture pipeline, material uniforms. The "FastPreview" path. |
| `tools/aw_editor/src/viewport/mipmap_generator.rs` | GPU mipmap generation using compute shaders |
| `tools/aw_editor/src/viewport/shaders/entity.wgsl` | Editor's own PBR WGSL shader (full Cook-Torrance BRDF) |
| `tools/aw_editor/src/viewport/shaders/shadow.wgsl` | Editor's own shadow map WGSL shader |
| `tools/aw_editor/src/viewport/shaders/brdf_lut.wgsl` | BRDF integration LUT compute shader |
| `tools/aw_editor/src/viewport/shaders/mipmap_blit.wgsl` | GPU mipmap generation shader |
| `tools/aw_editor/src/tab_viewer.rs` | Tab viewer logic (moved/unified into `tab_viewer/mod.rs` 8,185-line module) |

### 9.2 Frame Rendering Sequence

`ViewportRenderer::render()` in `tools/aw_editor/src/viewport/renderer.rs`:

```
Frame Start
  ├─ [Size check] resize() → Depth32Float, Rgba16Float HDR, Tonemap pipeline, Depth staging buffer
  ├─ [PASS 1: Engine Scene — HDR target] adapter.render_to_texture(hdr_view, encoder)
  │     → Renderer::draw_into(hdr_view, Some(depth_view), encoder)   [Real-Fix.A: depth_view threaded]
  ├─ [PASS 2: Grid Overlay — HDR target] grid_renderer.render(...)
  ├─ [PASS 3: Physics/Debug Lines — HDR target] PhysicsDebugRenderer (merges all line arrays)
  ├─ [PASS 4: HDR → LDR Blit — LDR target] Tonemap pipeline (3-vertex full-screen triangle)
  └─ [PASS 5: Gizmo Overlays — LDR target] After tonemap for crisp LDR overlays
```

**Command submission**: All passes share one `wgpu::CommandEncoder`, submitted via `queue.submit(once(encoder.finish()))` at the end.

(Sections 9.3-9.7 GPU resource inventory, RenderMode enum, EngineRenderAdapter state, model naming conventions, Pose → Transform mapping unchanged from v0.6.0 — see `aw_editor.md` for any newer reorganization.)

---

## 10. Unsafe Code Inventory (Trace: [`ecs_math_core_sdk_foundation.md`](ecs_math_core_sdk_foundation.md) §8)

### Verified Crates (Miri + Kani validated)

| Crate | Primary Unsafe Locations | Purpose |
|-------|------------------------|---------|
| `astraweave-ecs` | blob_vec.rs, archetype.rs, entity_allocator.rs, sparse_set.rs, parallel.rs (residual), system_param.rs, command_buffer.rs | Type-erased component storage, raw memory alloc/dealloc, pointer arithmetic, drop function pointers |
| `astraweave-math` | simd_vec.rs, simd_mat.rs, simd_quat.rs | SIMD intrinsics (`#[target_feature]`-gated; release fallthrough to glam scalar) |
| `astraweave-core` | ecs_bridge.rs, ecs_events.rs | Entity mapping, event bridging |
| `astraweave-sdk` | lib.rs (C ABI functions) | FFI boundary: null checks, buffer overflow protection, every entry `unsafe extern "C"` or `extern "C"` |

**Kani Proof Files**:
- `astraweave-ecs/src/blob_vec_kani.rs` — BlobVec invariants
- `astraweave-ecs/src/entity_allocator_kani.rs` — Generational index correctness
- `astraweave-core/src/schema_kani.rs` — Schema verification (`IVec2` numerical properties, `WorldSnapshot` helper correctness — does NOT cover field-name stability)
- `astraweave-math/src/simd_vec_kani.rs` — SIMD operation correctness
- `astraweave-sdk/src/lib_kani.rs` — FFI safety (buffer overflow, null pointer)

### Other Crates with Unsafe

| Crate | Unsafe Locations | Notes |
|---|---|---|
| `astraweave-ai` | `async_task.rs` — custom RawWaker VTable | Crate-level `forbid(unsafe_code)` is NOT declared (`ai_pipeline.md` §8 Invariant 15; §11). |
| `astraweave-fluids` | `debug_viz.rs:479-480` — 2× `unsafe impl bytemuck::{Pod,Zeroable}` for `DebugVertex` | No `unsafe {…}` blocks. No `forbid(unsafe_code)`. |
| `astraweave-physics`, `astraweave-input`, `astraweave-net`, `astraweave-audio`, `astraweave-llm`, `astraweave-behavior`, `astraweave-memory`, `astraweave-director`, `astraweave-coordination`, `astraweave-npc`, `astraweave-dialogue`, `astraweave-persistence-ecs`, `aw-save`, `astraweave-net-ecs`, `aw-net-proto`, `astraweave-scene` | _(none)_ — `#![forbid(unsafe_code)]` declared at crate root | Unsafe lives in deps (Rapier3D, wgpu, rodio, tokio-rustls, etc.) |
| `astraweave-render` | `#![deny(unsafe_code)]` at `lib.rs:1` — crate is **unsafe-free** | Any wgpu unsafe lives inside wgpu |
| `aw_editor` | NOT `#![forbid(unsafe_code)]` — raw-window-handle, wgpu raw resources, Windows-specific HWND access at `main.rs:417` | Cannot forbid unsafe |

---

## 11. Test Infrastructure

### Test Coverage by Crate

**46+ crates** have `#[cfg(test)]` modules. **56+ crates** have criterion benchmark harnesses.

| Tier | Crates | Status |
|------|--------|--------|
| **Formally Verified** | ecs, math, core, sdk | Miri (1,059 tests, 0 UB) + Kani proofs |
| **A+ Grade** | fluids (2,404 tests / 600+ inline + integration suite) | Benchmark caliber — but the crate is **dormant in production** (`fluids.md` §11) |
| **A Grade** | physics (110+ in core, 20 integration files, 10 benches), environment (55+), net (377), persistence-ecs+aw-save combined (179), audio (~80 inline + 14 integration files / 7,063 LoC) | Strong coverage |
| **B+ Grade** | vehicle (50+), gravity (30+), animation (13+29 inline + 5 dedicated integration files / 1,921 LoC + Wave 2) | Good, missing edge cases |
| **B Grade** | cloth (25+), ragdoll (33+) | Missing stress tests |
| **C-D Grade** | destruction (17), projectile (21), spatial_hash (8 — see `physics.md` §11 for actual surface), async_scheduler (4) | Active improvement |

### CI Workflows

| Workflow | Schedule | Crates | Timeout |
|----------|----------|--------|---------|
| `miri.yml` | Weekly (Sat 2 AM UTC) | ecs (120m), core (90m), physics (90m), ai (60m) | Per-crate |
| `kani.yml` | Weekly (Sun 3 AM UTC) | ecs (120m), math (60m), sdk (60m), core (90m) | Per-crate |
| `net-tests.yml` | Push to main + every PR | `astraweave-net` (`--all-features --verbose`) | Per-crate |
| `sanitizers.yml` | (per workflow) | P1 array includes `astraweave-net`, `astraweave-asset`, `astraweave-memory`, `astraweave-context` | Per-crate |
| `aw-editor-tests.yml` + `editor-ci.yml` | (per workflow) | `aw_editor` | Per-crate |

**Crates NOT in any workflow** (per per-trace audits): `astraweave-persistence-ecs` (per `persistence_ecs.md` §10), `astraweave-input` Miri/Kani (per `input.md` §10 — present only in `ci.yml` and `benchmark-dashboard.yml`), `astraweave-audio` Miri/Kani/mutation/coverage (per `audio.md` §10 — only `ci.yml`).

### Editor Tests

`aw_editor`: **9,425 `#[test]` annotations** total (4,103 inline + 5,322 in `tests/`; live count 2026-06-10, superseding the ~9,397 of 2026-05-12 per `aw_editor.md` §10). 66 files in `tools/aw_editor/tests/` including 46+ `wave2_*` suites and 16 `mutation_resistant_*.rs` per-subsystem files.

### Fuzz Targets (off-CI)

- `astraweave-net/fuzz/fuzz_targets/`: 4 cargo-fuzz targets (`fuzz_delta_compression`, `fuzz_interest_management`, `fuzz_packet_parsing`, `fuzz_snapshot_serialization`). Not in any workflow as of `a2474c5b7` (`net.md` §10).
- `astraweave-ecs/fuzz`: excluded from workspace.

---

## 12. Known Issues & Exclusions

### Build Exclusions

| Crate/Example | Issue |
|--------------|-------|
| `ui_controls_demo`, `debug_overlay` | egui/winit version drift — won't compile |
| `astraweave-author`, `rhai_authoring` | Rhai Sync trait errors |
| `astraweave-llm`, `llm_toolcall` | Excluded from standard builds |
| ECS fuzz targets | Must be built separately (`astraweave-ecs/fuzz` excluded) |

### Production Safety
- **Zero `.unwrap()` in production code (engine runtime crates)** — all confined to `#[cfg(test)]` modules
- Build/CLI tools (`aw_build`, `aw_demo_builder`) have low-risk `.unwrap()` in non-runtime paths
- Editor: per `aw_editor.md` §8 Invariant 19, **~12 `.unwrap()` calls remain before the first `#[cfg(test)]` boundary** across `tools/aw_editor/src/` (down from audit baseline 110+). Distribution: `mutation_tests.rs` (8 — itself a test-harness module), `terrain_integration.rs` (3), `runtime.rs` (1). Zero in `main.rs`.
- Windows: 16 MB stack size (configured in `.cargo/config.toml`) for large State structs

### Active Work (as of 2026-05-13)
- **Editor Multi-Tool Architecture Campaign Sub-phase 3** (Mediator Brush diagnostic, Round 8 closure) — `aw_editor.md` §1 Owner notes. §7.7 wrapped-component resource identity trap surfaced at 4 layers; Real-Fix.A/B/C landed, Real-Fix.D pending. Per the campaign doc Status header, a CLAUDE.md amendment cycle is pending to elevate §7.7 from candidate corollary to first-class top-level Edit.
- **Fix 27 Unified Pipeline Campaign** — structurally complete (astraweave-render non-optional, entity rendering through engine, ~4,000 LoC deleted); deeper unification (shadow/IBL/post-processing alignment) ongoing
- **Editor behavioral correctness audit remediation** — 37 fixes across 47 commits shipped; per-audit open items still pending Andrew-gate (`aw_editor.md` §11)
- **Architecture trace campaign** — 13 of N subsystems traced (this document); per CLAUDE.md, traces are part of the production contract

---

## 13. Quick Reference: Where to Find Things

| Need | Location |
|------|----------|
| **Per-subsystem trace** | `docs/architecture/<system>.md` — see §0 trace index above |
| **Trace toolkit prompts** | `docs/architecture/_meta/` (5 templates) |
| AI orchestration | `astraweave-ai/src/{orchestrator,tool_sandbox,core_loop,ai_arbiter,llm_executor}.rs` |
| WorldSnapshot definition | `astraweave-core/src/schema.rs:270` (`pub struct WorldSnapshot`) |
| ECS internals | `astraweave-ecs/src/{archetype,blob_vec,entity_allocator,events,parallel}.rs` |
| Rendering pipeline (engine) | `astraweave-render/src/{renderer,frame_graph,graph,hdr_pipeline,clustered,shadow_csm}.rs` |
| Engine renderer public API | `astraweave-render/src/lib.rs` (re-exports section) — but check `lib.rs:251-253` for explicit non-re-export of `material_library::Material` |
| Physics engine | `astraweave-physics/src/{lib (PhysicsWorld + CharacterController),character_controller,spatial_hash (DORMANT)}.rs` |
| Combat system | `astraweave-gameplay/src/combat_physics.rs` |
| SIMD math | `astraweave-math/src/{simd_vec,simd_mat,simd_quat,simd_movement}.rs` |
| Terrain generation | `astraweave-terrain/src/{voxel_mesh,biome_pack,biome,scatter,blueprint_zone,climate,biome_lookup,biome_parameters,biome_param_blending,world_archetypes}.rs` |
| Terrain material system (canonical 32-layer) | `astraweave-render/src/{terrain_material,terrain_material_manager,material_library}.rs` + `astraweave-render/shaders/{pbr_terrain_vs,pbr_terrain}.wgsl` + `tools/aw_editor/src/viewport/{types,terrain_splat_builder}.rs` |
| Animation runtime (canonical) | `astraweave-render/src/{animation,skinning_gpu}.rs` |
| ECS animation components | `astraweave-scene/src/lib.rs:415-` (`CSkeleton`, `CAnimator`, `CJointMatrices`, `CParentBone`) |
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
| CI workflows | `.github/workflows/{miri,kani,net-tests,sanitizers,aw-editor-tests,editor-ci}.yml` |
| Project status | `docs/current/PROJECT_STATUS.md` |
| Behavioral audit | `docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md` |
| Fix 27 campaign plan | `docs/current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` |
| Multi-Tool Architecture campaign | `docs/current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md` |
| Fluids enhancement roadmap | `docs/current/FLUIDS_RESEARCH_GRADE_ENHANCEMENT_PLAN.md` |
| Benchmarks | `docs/current/MASTER_BENCHMARK_REPORT.md` |
| Coverage | `docs/current/MASTER_COVERAGE_REPORT.md` |

---

## 14. Workspace-Wide Open Questions (surfaced by per-trace §11)

These are decisional questions that cross subsystem boundaries or affect the broader architecture. Each cites the trace where it lives. **Andrew-gate decisions pending.**

1. **Long-term plan for legacy `astraweave-core::World`?** Bridged to ECS via `build_app`; consumed by AI orchestrators, SDK, `validate_and_execute`, capture_replay, editor. Retiring it requires coordinated changes across all consumers. No retirement campaign identified. `ecs_math_core_sdk_foundation.md` §11.

2. **Two GOAP implementations — consolidation roadmap?** Canonical `astraweave-behavior::goap` vs advanced `astraweave-ai::goap` (16.7K LoC, feature `planner_advanced`, zero production constructors). `ai_pipeline.md` §11, §13.6.

3. **Runtime LLM model: phi3:medium (default) vs Qwen3 (doc-comments).** One-line change to remove the `"phi3:medium"` default at `orchestrator.rs:488-490`. Three Ollama clients coexist. `ai_pipeline.md` §6, §11.

4. **Runtime AI path bypasses entire LLM hardening surface (~15K LoC).** `LlmExecutor` consumes `Arc<dyn LlmExecutor>` directly without wrapping in `FallbackOrchestrator` or `ProductionHardeningLayer`. Smallest production-wiring step is making `LlmExecutor::new` construct a `FallbackOrchestrator`. `ai_pipeline.md` §11, §13.7.

5. **§7.7 wrapped-component resource identity trap — preventive instrumentation?** Four confirmed instances during Editor Multi-Tool Architecture Sub-phase 3. CLAUDE.md amendment cycle pending — potentially elevating the rule from "candidate corollary appended to Edit 2" to a first-class top-level Edit. `render_pipeline_material_system_shader_infrastructure.md` §11.

6. **Multiple `WorldSnapshot` JSON serializers with different key conventions.** Three serializers in `astraweave-llm/src/{prompts,compression,prompt_template}.rs` use different translation rules (`pos`→`position`, `hp`→`health`, `pois`→`points_of_interest` in `prompts.rs` but raw names in others). Any LLM trained against one encoder is implicitly bound to that encoder's keys. No build-time check ties Rust field names to JSON output keys. `ecs_math_core_sdk_foundation.md` §11.

7. **Frame graph migration: when does it move from scaffold to command-driver?** Currently delegates GPU recording back to `Renderer` methods. `render_pipeline_material_system_shader_infrastructure.md` §11.

8. **Lumen GI vs VXGI — coexisting or eventual single choice?** Both are fully implemented in `astraweave-render/src/{lumen,distance_field,surface_cache,final_gather}.rs` vs `gi/{vxgi,voxelization_pipeline}.rs`. `render_pipeline_material_system_shader_infrastructure.md` §11.

9. **`RenderMode::FastPreview` long-term disposition.** Per `FIX27_UNIFIED_PIPELINE_CAMPAIGN.md` was targeted for elimination but the enum variant remains as transitional fallback. `render_pipeline_material_system_shader_infrastructure.md` §11.

10. **Three material binding paths — eventual consolidation?** `MaterialManager` (TOML-driven) / `MaterialLibrary` (canonical 32-layer terrain) / `BindlessMaterialSystem` (bindless modern). `render_pipeline_material_system_shader_infrastructure.md` §11.

11. **Memory subsystem dormancy — production-wire, prune, or rebrand?** ~11K LoC; zero in-engine production consumers. `ai_pipeline.md` §11, §13.1.

12. **Fluids subsystem dormancy — superseded by the view-side water successor (W.1/W.2, 2026-06-20).** W.1 removed the SPH/voxel solver (−58.8K LoC); the crate is now ~24.2K LoC (deprecated PBD remnant + F.4 accent substrate). The live water system is `astraweave-water` + render `water.rs` (see `water.md`). Remaining question: prune or rebrand the deprecated remnant. `fluids.md` §11.

13. **Four parallel `Skeleton`/`Joint`/`Transform`/`AnimationClip` type families — unify into a shared `astraweave-animation` crate?** `animation.md` §11.

14. **`MAX_JOINTS = 256` — permanent limit or scale with hardware?** Hard-coded in Rust (`animation.rs:358`) and WGSL (`skinning_gpu.rs:261`); kept in sync manually; surplus joints silently drop. `animation.md` §11.

15. **CubicSpline interpolation — implement or remove the variant?** Render-side and asset-side both have the variant but fall back to Linear/Slerp. glTF cubic-spline clips lose fidelity. `animation.md` §11.

16. **Co-existence of two networking subsystems (`astraweave-net` vs `astraweave-net-ecs`+standalone trio).** Disjoint data models (`IVec2` vs `Vec3`), disjoint wire formats (JSON vs PostcardLz4/Bincode), disjoint integration patterns. Are these intended to coexist long-term? `net.md` §11.

17. **Standalone server signature mismatch — RESOLVED** (Net-Trio-Remediation `561b20957`+`79424389e`+`066cd6cfd`; refute `eb9977b88`). Client and server now sign/verify canonical HMAC-SHA256 (`aw-net-proto`) over `input_frame_sig_payload`; server verifies FIRST and kicks by default (`SignatureFailurePolicy::Kick`). `sign16`/`SessionKey`/`session_key_hint` deleted; `InputFrame.sig` is `[u8; 32]`; 104-test regression net. Deliberate residual boundaries (out of scope, not defects): no replay protection; server→client unsigned. `net_ecs.md` §6/§7/§11; `docs/audits/net_trio_signature_remediation_findings_2026-06.md`.

18. **`astraweave-persistence-ecs` Plugin layer disposition.** `auto_save_system` body is comment-only TODO; `replay_system` doesn't apply events; `CPersistenceManager::save_game` hardcodes inventory; `calculate_world_hash` covers only 4 of 10 components; `SerializedWorld.world_tick` always 0. No production consumer. `persistence_ecs.md` §11.

19. **`SpatialHash` physics module — wire in or remove?** Advertised as broadphase in lib.rs doc-comment; actual broadphase is Rapier's `DefaultBroadPhase`. 1,038 LoC dormant. `physics.md` §11.

20. **Editor god-struct refactor.** `EditorApp` 55+ → 123 fields (verified 2026-05-12). Audit §1.1 recommends extraction into `SceneManager`, `SelectionService`, `AssetService`, `ToolService`. No campaign launched. `aw_editor.md` §11.

21. **`astraweave-input` declared-but-unused deps in `astraweave-gameplay` and `astraweave-ui`.** Both Cargo.tomls pull the crate; neither imports it. The dep additions left no source-file `use astraweave_input` in git history. `input.md` §11.

22. **Editor's parallel input/audio/animation/fluids surfaces.** `input_bindings_panel.rs` (2,511 LoC, 13 types, no `astraweave-input` dep), `AudioPanel` 10+ no-op `AudioAction` variants, `astraweave-fluids::editor.rs` (5,823 LoC forward-design not wired to `tools/aw_editor`). Should the editor be migrated to use the canonical crates' types? `input.md` §6, `audio.md` §6, `fluids.md` §11, `aw_editor.md` §6.

23. **Aspirational documentation tree disposition** (commit `28bc94f21`, 80+ files in `docs/src/`). Delete, rewrite to match actual API, or retain as roadmap for a future rewrite? `audio.md` §11, `input.md` §11.

---

## Revision History

### v0.7.4 (2026-06-29)
D.2.B-Prop-Final miri-count propagation (doc-truth campaign): the §"Formally Verified" tier row corrected **Miri 977 → 1,059** (current count, per `CLAIMS_REGISTRY.md` `miri-tests` VERIFIED-AT-HEAD; supersedes the stale-low 977). Single factual sync; no other content changed.

### v0.7.3 (2026-06-25)
D.2.A.1 W.1-contamination correction (doc-truth campaign), driven by `docs/campaigns/doc-truth/D_RESUME_0_RECON.md`:
- **astraweave-fluids fluids surfaces** (§0 trace row, §5 source-file/solver bullets + dormant-LoC table, §11 open-question 12): W.1 (2026-06-20, `1a57fdd41`) deleted the SPH/voxel solver + `simd_ops.rs` (−58.8K LoC). Corrected from "84.5K LoC / 35 files / `simd_ops.rs` 39,554 / `PcisphSystem` / five solver surfaces" to **~24.2K LoC / 19 files / `FluidSystem` + `WaterEffectsManager` only** (`PcisphSystem`/`UnifiedSolver`/`ResearchFluidSystem` absent) — the deprecated PBD remnant + F.4 GPU-particle accent substrate. The live water system is the view-side successor (`astraweave-water` + render `water.rs`); see `water.md`.
- Present-tense corrections of deleted code on the canonical cross-crate map; registry source-of-truth is `CLAIMS_REGISTRY.md` (`fluids-loc` / `water-facade-loc` / `water-surface-loc` / `water-system`). These were the only broad docs W.2 Phase 2 left fully contaminated.

### v0.7.2 (2026-06-10)
Engine-health-audit reconciliation pass (doc-only), driven by the 2026-06-10 full workspace audit:
- §1: total workspace members corrected **143 → 130** — the root Cargo.toml `[workspace].members` list contains 130 entries (128 unique paths; `astraweave-npc` and `astraweave-security` are each listed twice) and `cargo metadata --no-deps` resolves exactly 130 packages (the two extras are auto-included path-dependency packages). Examples count corrected to **59**.
- §2.1: added `astraweave-camera` (Unified Camera C.2, `52b9e711c`, 2026-05-18) to the Rendering & Assets table; appended `camera` to `astraweave-render`'s workspace-deps row (non-optional, astraweave-render/Cargo.toml:45). Other production consumers: `tools/aw_editor` + 13 examples.
- §0: render-pipeline trace row updated (Multi-Tool SP3/SP4 closed, SP5 in flight; Render Parity P.1–P.7 closed 2026-05-17); aw_editor trace row updated (Real-Fix.A–E all landed; SP3/SP4 complete; SP5 in flight per commits `85786bf70`/`3cdb23239`).
- Companion edits in the same pass: README engine-health section rewritten against this map + the live audit; `workspace_map.html` counts re-reconciled; MASTER_COVERAGE_REPORT bumped v5.3.0; MASTER_ROADMAP bumped v1.51; PROJECT_STATUS refreshed; CLAUDE.md known-build-issues + networking-hazard rows corrected.

### v0.7.1 (2026-06-10)
Net-Trio-Remediation reconciliation pass (doc-only). The standalone matchmaking trio's signature defect — client signed with the XOR `sign16` (16-byte tag) while the server verified HMAC-SHA256 (32-byte tag), so every verification failed and the server only `warn!`ed — is now FIXED and ENFORCED in code. Reconciled five surfaces here against the landed change:
- §0 Trace Index: net_ecs row now notes the mismatch is RESOLVED (HMAC-SHA256 enforced, kick-by-default).
- §4.3 Silent-Failure inventory: the "HMAC server verification always fails / `warn!`-only" row marked RESOLVED with the commit ledger; retargeted from `aw-net-server/src/main.rs` to `src/lib.rs` (`on_client_msg{,_tls}`) + `aw-net-proto/src/lib.rs`.
- §7.2 Doc-comment drift: the "XOR `sign16` as MVP" README row marked RESOLVED (README updated in W.4).
- §8.8 Network data-flow diagram: replaced the `sign16`/`[u8;16]`/"always fails — warn! only" lines with the enforced HMAC flow (`[u8; 32]` sig, `input_frame_sig_payload`, verify-FIRST, kick-by-default); removed `session_key`/`session_hint` from the matchmaking line.
- §14 open-question 17: marked RESOLVED with the commit ledger; recorded the deliberate residual boundaries (no replay protection; S2C unsigned).
- The two-coexisting-networking-subsystems open question (§14 #16) is left intact — that is the Dormant Surface Disposition's scope, not this workflow's.
- **Commit ledger**: `561b20957` (W.1 canonical HMAC; stub deleted), `79424389e` (W.2.a client adopts), `066cd6cfd` (W.2.b server verifies + policy + lib/bin split), `9a3fc94e3` (W.2.b-fix1), `7029d7d7f`/`a2b494942`/`0e702738e`/`68a9a1936` (W.3.1-3.4 test families), `420a6f61b` (W.5.1), `2955cd14c` (W.5.2), `eb9977b88` (W.5.3 refute). Authoritative input: `net_ecs.md` v1.3 (2026-06-10); audit `docs/audits/net_trio_signature_remediation_findings_2026-06.md`.

### v0.7.0 (2026-05-13)
Reconciliation pass against the 13 per-subsystem architecture traces under `docs/architecture/`:
- Added §0 Trace Index linking every traced subsystem at section level.
- Added §4 Workspace-Wide Structural Axioms — promoted the §7.7 wrapped-component resource identity rule, Fix-27 no-second-implementation lesson, silent-failure policy, and wired-beats-tested taxonomy.
- Added §5 Dormant-Code Inventory consolidating the per-trace §11 evidence across six categories (~200K LoC total).
- Added §7 Documentation Hazards consolidating the cross-trace doc-vs-code drift inventory (aspirational `docs/src/` types, QUIC-vs-WebSocket architectural mismatch, Qwen3-vs-phi3 runtime default, HMAC-vs-XOR sig mismatch, HNSW-vs-linear-scan, SpatialHash dormancy, 4-bus-vs-5-bus audio docstring, `compute_poses_stub`).
- Added §14 Workspace-Wide Open Questions surfacing 23 cross-cutting decisional questions from per-trace §11 sections.
- Updated §2 dependency tables to cite the relevant trace for each domain.
- Verified `terrain → gameplay` and `render → aw_asset_cli` dependency anomalies still hold against current Cargo.toml.
- Added new anomalies to §2.3: declared-but-unused-dep patterns surfaced by traces (`stress-test`'s net-ecs+persistence-ecs deps, `persistence-ecs`'s memory dep, `gameplay`/`ui`'s input dep).
- Updated §3 public-API surface with trace-cited corrections: `astraweave-physics` has **5 features** not 3 (`physics.md` §1); `astraweave-fluids` is **dormant** outside `examples/fluids_demo` (`fluids.md` §1); `AudioEngine` has **5 buses** not 4 (`audio.md` §1); `astraweave-net-ecs` ECS Plugin layer is **dormant** (`net_ecs.md` §1); LLM runtime default is `phi3:medium` despite Qwen3 doc-comments (`ai_pipeline.md` §6); editor's `astraweave-render` dep is **required, non-optional** post-Fix-27.
- Updated §6 Integration Seams with risk levels reflecting trace §6 conflict maps: `Renderer::draw_into` signature change (Real-Fix.A) added as HIGH-risk seam; four-parallel-Skeleton-type-families flagged as HIGH; `EntityState` collision between `net` and `net-ecs` flagged.
- Updated §10 Unsafe Code Inventory: `astraweave-ai` lacks `forbid(unsafe_code)` but has zero unsafe blocks; `astraweave-fluids` has 2 bytemuck `unsafe impl` lines but no `unsafe {}` blocks; sibling AI crates and most physics-and-below crates declare `forbid`.
- Preserved §1, §8, §9, §11, §12, §13 content from v0.6.0 where still accurate. Edited surgically rather than wholesale reorganizing.
- **Authoritative inputs**: `terrain_materials.md` v1.1 (2026-05-10), `render_pipeline_material_system_shader_infrastructure.md` v1.0 (2026-05-10), `physics.md` v1.2 (2026-05-12), `persistence_ecs.md` v1.2 (2026-05-12), `net.md` v1.2 (2026-05-12), `net_ecs.md` v1.2 (2026-05-12), `input.md` v1.2 (2026-05-12), `fluids.md` v1.2 (2026-05-12), `ecs_math_core_sdk_foundation.md` v1.2 (2026-05-10), `audio.md` v1.2 (2026-05-12), `animation.md` v1.0 (2026-05-10), `ai_pipeline.md` v1.11 (2026-05-12), `aw_editor.md` v1.2 (2026-05-12).

### v0.6.0 (2026-05-07)
Full cartography audit — impostor infrastructure catalogued, astraweave-alloc added, workspace count verified at 143 members, viewport module inventory updated, Regional Archetype Variation campaign status integrated.

**Workspace Growth (+3 members):**
- `astraweave-alloc` (in `crates/`) — Opt-in mimalloc global allocator replacement
- `astraweave-optimization` — LLM optimization experiments
- `crates/astract` + `crates/astract/astract-macro` — Attribute macro system

**Impostor LOD3 System (April 16, 2026):**
- Render Crate: `impostor_bake.rs` (57 tests), `impostor_lod3.rs` (5 GPU tests), `impostor_pass.rs` (8 tests), `bin/aw_impostor_bake.rs` (CLI)
- Editor Viewport: `impostor_registry.rs` (8 tests), `impostor_wiring.rs` (4 tests), `engine_adapter.rs` (13 tests)
- Total New Tests: 57 (impostor family)

**ECS Architecture — ParallelSchedule Removal** (April 18, 2026):
- `astraweave-ecs` is now **deterministic single-threaded** per tick
- Removed `ParallelSchedule`, `SystemAccess`, `SystemDescriptor`
- Audit: `docs/audits/parallel_schedule_removal_2026-04-18.md`
- Parallelism moved to subsystem level (rayon, tokio, GPU compute)

**Editor Default Features Changed:**
- `impostor-bake`, `terrain-splat-arrays`, `fast-alloc` — NOW in default features
- Opt-out: `--no-default-features --features editor-core`

### v0.5.0 (2026-04-04)
Reflects Fix 27 Unified Pipeline Campaign — EntityRenderer deleted, astraweave-render is now non-optional in aw_editor.

**Deleted** (no longer in codebase):
- `tools/aw_editor/src/viewport/entity_renderer.rs` (~3,600 LOC)
- `tools/aw_editor/src/viewport/mipmap_generator.rs`
- `tools/aw_editor/src/viewport/shaders/{entity,shadow,brdf_lut,mipmap_blit}.wgsl`
- `tools/aw_editor/src/tab_viewer.rs`

**Changed**:
- `tools/aw_editor/Cargo.toml`: `astraweave-render` is now a required dependency
- `tools/aw_editor/src/viewport/engine_adapter.rs`: Expanded ~568 → ~740 LOC
- `astraweave-render/src/renderer.rs`: 4-cascade CSM shadows, IBL prefiltered cubemap, Khronos PBR Neutral tonemapper
- `astraweave_core::Pose`: Added `scale_y`, `scale_z` fields for per-axis scale

**37 additional behavioral fixes** — see `docs/current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md`.
