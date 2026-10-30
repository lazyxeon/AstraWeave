---
layout: default
title: Crate Index
---

# Crate Index

AstraWeave consists of **130 workspace packages** <!-- Source: CLAIMS_REGISTRY.md#workspace-members --> (~51 production crates plus tools and 59 examples) with **~39,900 test markers** <!-- Source: CLAIMS_REGISTRY.md#test-markers-total --> across the workspace.

## Tier 1: Core Engine

| Crate | Description | Tests | Coverage |
|-------|-------------|------:|----------|
| **astraweave-ecs** | Archetype-based ECS with deterministic stages | 330+ | 90.57% |
| **astraweave-core** | Shared types: WorldSnapshot, PlanIntent, ToolRegistry | 505+ | 89.73% |
| **astraweave-math** | SIMD-accelerated vector/matrix/quaternion ops | 109 | 72.13% |
| **astraweave-camera** | Canonical camera types: `FreeFly` producer, `RenderView` upload contract, `CameraProducer` trait | 19 | — |
| **astraweave-physics** | Rapier3D wrapper: rigid bodies, spatial hash, ragdoll, cloth | 1,244+ | 34.38% |
| **astraweave-render** | wgpu 25.0.2 renderer: PBR, shadows, post-FX, animation | 806 | 48.04% |
| **astraweave-scene** | Scene graph, transforms, world partitioning, streaming | 210 | 55.42% |
| **astraweave-sdk** | C ABI for non-Rust embedding (cbindgen 0.29) | 70 | 68.24% |

## Tier 2: AI & Behavior

| Crate | Description | Tests | Coverage |
|-------|-------------|------:|----------|
| **astraweave-ai** | AI orchestration: GOAP, BT, LLM, hybrid arbiter (7 modes) | 100+ | 74.21% |
| **astraweave-behavior** | Behavior trees and GOAP planner | 233 | 85.81% |
| **astraweave-npc** | NPC behavior, emotion system, schedules | 108+ | — |
| **astraweave-director** | AI director, boss encounters, minion management | 180+ | — |
| **astraweave-coordination** | Multi-agent coordination, squad formations | 94+ | — |
| **astraweave-persona** | NPC personality and dialogue personas | 244+ | — |

## Tier 3: Gameplay & Content

| Crate | Description | Tests | Coverage |
|-------|-------------|------:|----------|
| **astraweave-gameplay** | Combat physics, crafting, quests, items, biomes | 471 | 50.46% |
| **astraweave-weaving** | Veilweaver fate-weaving mechanic | 407 | 94.34% |
| **astraweave-nav** | Navmesh baking and A* pathfinding | 216 | 93.11% |
| **astraweave-terrain** | Procedural terrain: voxel/polygon, marching cubes, climate | 2,536 | 44.11% |
| **astraweave-pcg** | Procedural content generation | 133 | 99.32% |
| **astraweave-fluids** | PBD fluid simulation | 738 <!-- Source: CLAIMS_REGISTRY.md#fluids-test-markers --> | 97.24% |
| **astraweave-materials** | Material definitions and shader cache | 282 | 98.55% |
| **astraweave-quests** | Quest system, objectives, rewards | 218+ | — |
| **astraweave-dialogue** | Dialogue trees, branching conversation | 291+ | Build error (Rhai Sync) |

## Tier 4: UI, Input & Presentation

| Crate | Description | Tests | Coverage |
|-------|-------------|------:|----------|
| **astraweave-ui** | egui menus, HUD, accessibility, gamepad | 331 | 73.84% |
| **astraweave-input** | Key/mouse/gamepad bindings and action mapping | 481 | 95.62% |
| **astraweave-audio** | rodio 0.17 spatial audio, dialogue, TTS | 239 | 24.34% |
| **astraweave-cinematics** | Timeline sequencer, camera/audio/FX tracks | 476 | 99.54% |
| **astract** | React-style declarative UI widgets | 168+ | — |
| **astract-macro** | Proc-macro crate for astract | 7+ | — |

## Tier 5: LLM & Intelligence

| Crate | Description | Tests | Coverage |
|-------|-------------|------:|----------|
| **astraweave-llm** | LLM client abstraction (Ollama, streaming) | 714+ | Excluded (ext. API) |
| **astraweave-llm-eval** | LLM evaluation harness | 43+ | — |
| **astraweave-prompts** | Prompt templates and engineering | 1,931 | 95.66% |
| **astraweave-embeddings** | Text embeddings client | 331 | 97.84% |
| **astraweave-rag** | Retrieval-augmented generation | 364 | 19.85% |
| **astraweave-memory** | Long-term memory for NPCs | 945 | 94.47% |
| **astraweave-context** | Context window management | 424 | 25.71% |
| **astraweave-optimization** | LLM performance optimization | 60+ | — |

## Tier 6: Networking & Persistence

| Crate | Description | Tests |
|-------|-------------|------:|
| **astraweave-net** | Networking core | 255+ |
| **astraweave-net-ecs** | ECS networking integration | 35+ |
| **aw-net-client** | Network client | 0 |
| **aw-net-server** | Network server | 0 |
| **aw-net-proto** | Network protocol definitions | 53+ |
| **aw-save** | Versioned, checksummed save files | 44+ |
| **astraweave-persistence-ecs** | ECS persistence integration | 160 |
| **astraweave-persistence-player** | Player profile persistence | 91+ |

## Tier 7: Assets & Content Pipeline

| Crate | Description | Tests |
|-------|-------------|------:|
| **astraweave-asset** | Asset loading and management | 431+ (build error) |
| **astraweave-asset-pipeline** | Asset build pipeline | 61+ |
| **astraweave-assets** | Asset definitions and registry | 137+ |
| **astraweave-blend** | Animation blending and transitions | 2,242+ (build error) |

## Tier 8: Infrastructure & Tools

| Crate | Description | Tests |
|-------|-------------|------:|
| **aw_editor** | Level/encounter editor (GUI) | 9,427 <!-- Source: CLAIMS_REGISTRY.md#editor-test-markers --> |
| **aw_asset_cli** | Asset pipeline CLI | — |
| **aw_build** | Build automation | 0 |
| **aw_debug** | Debug utilities | — |
| **aw_headless** | Headless engine runner | — |
| **aw_demo_builder** | Demo builder CLI | 0 |
| **aw_release** | Release automation | 0 |
| **aw_save_cli** | Save file CLI tool | 0 |
| **aw_texture_gen** | Texture generator | 0 |
| **astraweave-profiling** | Tracy profiling integration | 41+ |
| **astraweave-observability** | Metrics and logging | 105+ |
| **astraweave-security** | Sandboxing and validation | 701 |
| **astraweave-secrets** | Encryption and key management | 54+ |
| **astraweave-scripting** | Rhai scripting integration | 179 |
| **astraweave-stress-test** | Stress testing framework | 34+ |
| **astraweave-ipc** | Inter-process communication | 57+ |
| **astraweave-steam** | Steam platform integration | 30+ (build error) |
| **astraweave-ai-gen** | AI generation utilities | 0 |

## Tier 9: Game-Specific

| Crate | Description | Tests |
|-------|-------------|------:|
| **veilweaver_slice_loader** | Veilweaver slice asset loader | 0 |
| **veilweaver_slice_runtime** | Veilweaver slice runtime logic | 460+ |
| **astraweave-author** | Rhai-based authoring tools | Build error (Sync) |

## Examples (59 packages)

All examples are runnable via `cargo run -p <name> --release`:

| Category | Examples |
|----------|----------|
| **AI/Companion** | `hello_companion`, `core_loop_bt_demo`, `core_loop_goap_demo`, `ecs_ai_demo`, `ecs_ai_showcase`, `orchestrator_async_tick`, `adaptive_boss`, `phase_director` |
| **LLM** | `llm_comprehensive_demo`, `llm_integration`, `llm_streaming_demo`, `llm_toolcall`, `ollama_probe`, `ollama_probe_example`, `phi3_demo`, `persona_loader` |
| **Rendering** | `visual_3d`, `shadow_csm_demo`, `skinning_demo`, `cutscene_render_demo`, `renderer_integration_test`, `debug_overlay` |
| **Physics** | `physics_demo3d`, `fluids_demo`, `nav_physics_bridge` |
| **Terrain/World** | `terrain_demo`, `world_partition_demo`, `biome_gpu_demo`, `biome_showcase`, `biome_transition_demo`, `biome_weather_demo`, `greybox_generator` |
| **UI** | `ui_controls_demo`, `ui_menu_demo`, `debug_toolkit_demo` |
| **Audio** | `audio_spatial_demo`, `dialogue_audio_cli`, `dialogue_voice_demo` |
| **Gameplay** | `combat_physics_demo`, `crafting_combat_demo`, `veilweaver_demo`, `veilweaver_quest_demo`, `weaving_playground`, `weaving_pcg_demo`, `quest_dialogue_demo`, `npc_town_demo` |
| **Cinematics** | `cinematics_timeline_demo` |
| **Networking** | `coop_client`, `coop_server`, `ipc_loopback` |
| **Persistence** | `save_integration`, `companion_profile` |
| **Tools** | `asset_signing`, `scripting_advanced_demo`, `rhai_authoring`, `profiling_demo`, `advanced_content_demo` |
| **Showcase** | `unified_showcase` |

## Known Build Issues

| Crate | Issue | Workaround |
|-------|-------|------------|
| `astraweave-dialogue` | Rhai Sync trait error | Excluded from standard builds |
| `astraweave-author` | Rhai Sync trait error | Excluded from standard builds |
| `astraweave-asset` | Build error | Under investigation |
| `astraweave-blend` | Build error | Under investigation |
| `astraweave-steam` | Build error | Platform-specific dependencies |
| `astraweave-llm` | Requires external API keys | Excluded from CI |
| `debug_overlay` | egui/winit version drift | Under investigation |
| `ui_controls_demo` | egui/winit version drift | Under investigation |

[← Back to Home](index.html)
