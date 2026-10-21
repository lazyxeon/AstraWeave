@AGENTS.md

# Claude Code

The import above is required. Claude Code does not load `AGENTS.md` when this file exists.

The field manual that used to live here is [`docs/current/AGENT_FIELD_MANUAL.md`](docs/current/AGENT_FIELD_MANUAL.md). `AGENTS.md` wins if they disagree.

`aw_trace_sync` rewrites only the table between the markers below. Do not edit that table by hand.

<!-- TRACE-TABLE:START -->
| Trace | Subsystem |
|---|---|
| `docs/architecture/ai_pipeline.md` | AI Pipeline (with 8 subsystem traces) |
| `docs/architecture/animation.md` | Animation System |
| `docs/architecture/asset.md` | Asset + Asset-Pipeline — loading, cell loader, Nanite preprocess, validator/texture/mesh |
| `docs/architecture/audio.md` | Audio |
| `docs/architecture/aw_editor.md` | aw_editor (Visual Editor) |
| `docs/architecture/camera.md` | Camera — freefly, projection, render-view, parity producer |
| `docs/architecture/causality.md` | Typed causal events, parent links, and baked consequence rules |
| `docs/architecture/cinematics.md` | Cinematics — timeline / sequencer |
| `docs/architecture/combat.md` | Q32.32 damage dealt and received, data-driven schools, AI knowledge mask |
| `docs/architecture/destruction.md` | Command log, support graph, fracture, terrain deltas, and debris. Sim state is Fixed64. |
| `docs/architecture/ecs_math_core_sdk_foundation.md` | ECS substrate + Math/Core/SDK |
| `docs/architecture/fluids.md` | Fluids |
| `docs/architecture/gameplay.md` | Gameplay — combat, crafting, Veilweaver slice, in-crate weave bridge |
| `docs/architecture/input.md` | Input |
| `docs/architecture/nav.md` | Navigation / Pathfinding — navmesh (resolves the two-`nav`-crate conflict) |
| `docs/architecture/net.md` | Net (snapshot-based game server) |
| `docs/architecture/net_ecs.md` | Net-ECS + standalone matchmaking |
| `docs/architecture/pcg.md` | PCG — WFC, layout, encounters, seeded RNG |
| `docs/architecture/persistence_ecs.md` | Persistence (aw-save + persistence-ecs) |
| `docs/architecture/physics.md` | Physics (Rapier3D wrapping + subsystems) |
| `docs/architecture/quests.md` | Quests — components, systems, LLM + terrain quests |
| `docs/architecture/render_pipeline_material_system_shader_infrastructure.md` | Render Pipeline + Material System + Shader Infrastructure |
| `docs/architecture/scene.md` | Scene — scene graph, world partition, cell streaming, GPU resource manager |
| `docs/architecture/scripting.md` | Sandboxed Rhai gameplay scripts, the mod loader, and tick-boundary reload |
| `docs/architecture/security.md` | Security + Secrets — script sandbox, anticheat, signatures, path validation, keyring |
| `docs/architecture/terrain.md` | Terrain System — voxel meshing, biomes, noise, scatter, streaming (complements `terrain_materials.md`, which is the material-splat slice) |
| `docs/architecture/terrain_materials.md` | Terrain Material System (canonical reference example) |
| `docs/architecture/ui.md` | UI — HUD, menus, panels (egui) |
| `docs/architecture/veil.md` | Deterministic Q32.32 fate-scar field sampled by volumetric fog and AI perception |
| `docs/architecture/water.md` | Water Successor — `WaterQuery` facade + render water surface + weave-response (part/freeze/raise/FreezeWater) + F.4 accents |
| `docs/architecture/weaving.md` | Weaving — modifier graph, casts, and the anchor slice (`astraweave-weaving`) |
<!-- TRACE-TABLE:END -->
