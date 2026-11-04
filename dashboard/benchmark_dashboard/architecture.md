---
layout: default
title: Architecture
---

# Architecture Overview

AstraWeave is an AI-native game engine organized as a Rust workspace of **128 packages** (69 engine/tool crates + 59 example crates) with clear dependency boundaries.

## Dependency Graph

```
                    astraweave-sdk (C ABI – cbindgen)
                         │
                    astraweave-core (shared types, schema)
                    ╱    │    ╲        ╲          ╲
          astraweave-ai  │  astraweave-gameplay  astraweave-weaving
              │     astraweave-ecs     │
              │          │             │
    astraweave-behavior  │   astraweave-physics ── astraweave-fluids
                         │         │
                   astraweave-nav   │
                         │         │
                   astraweave-scene astraweave-terrain
                    ╱         ╲    │
          astraweave-render  astraweave-audio
                    │                │
              astraweave-ui    astraweave-dialogue
                    │
              astraweave-input
```

## AI-Native Loop

Every NPC in AstraWeave runs through a validated 4-stage loop every frame:

### 1. Perception (`SystemStage::PERCEPTION`)

Build a `WorldSnapshot` for each AI agent, containing filtered world state:

```rust
pub struct WorldSnapshot {
    pub t: f32,
    pub player: PlayerState,    // hp, pos, stance, orders
    pub me: CompanionState,     // ammo, cooldowns, morale, pos
    pub enemies: Vec<EnemyState>,
    pub pois: Vec<Poi>,
    pub obstacles: Vec<IVec2>,
    pub objective: Option<String>,
}
```

### 2. Reasoning (`SystemStage::AI_PLANNING`)

The `Orchestrator` trait dispatches to the configured AI backend:

```rust
pub trait Orchestrator {
    fn propose_plan(&self, snap: &WorldSnapshot) -> PlanIntent;
}

pub trait OrchestratorAsync {
    async fn plan(&self, snap: WorldSnapshot, budget_ms: u32) -> Result<PlanIntent>;
}
```

| Mode | Backend | Latency | Use Case |
|------|---------|---------|----------|
| Classical | `RuleOrchestrator` | ~ns | Rule-based patrol, guard, combat |
| BehaviorTree | `BehaviorGraph` | 3.19 µs/1K nodes | Hierarchical decision trees |
| Utility | `UtilityOrchestrator` | 460 ns | Dynamic priority scoring |
| GOAP | `GOAPPlanner` | 3.46–3.56 ns (next_action) | Strategic goal planning |
| LLM | Ollama (default `phi3:medium`; Qwen3-8B opt-in) | 2–8 s | Creative, emergent behavior |
| Hybrid | LLM + fallback | 2–8 s with fallback | LLM with graceful degradation |
| Arbiter | `AIArbiter` (GOAP + LLM) | 314 ns–8 s | GOAP for tactics, LLM for strategy |

### 3. Planning

The orchestrator produces a `PlanIntent` containing a sequence of `ActionStep` values. Each step is a strongly-typed enum variant covering 30+ action types across movement, combat, tactical, equipment, and support categories:

```rust
pub struct PlanIntent {
    pub plan_id: String,
    pub steps: Vec<ActionStep>,
}

pub enum ActionStep {
    MoveTo { x: i32, y: i32, speed: Option<MovementSpeed> },
    Attack { target_id: Entity },
    TakeCover { position: Option<IVec2> },
    ThrowSmoke { x: i32, y: i32 },
    Heal { target_id: Option<Entity> },
    CoverFire { target_id: Entity, duration: f32 },
    // ... 30+ variants total
}
```

### 4. Action (Tool Sandbox)

The `ToolRegistry` validates every `ActionStep` against permitted actions, constraints, and cooldowns. Invalid actions are rejected — **no AI can "cheat"**. Validation throughput: **6.48M checks/sec**.

## ECS System Stages

Deterministic execution order, 60 Hz fixed tick:

| # | Stage | Purpose | Key Systems |
|---|-------|---------|-------------|
| 1 | `PRE_SIMULATION` | Setup, initialization | Resource loading, state init |
| 2 | `PERCEPTION` | Build WorldSnapshots | AI sensor updates, visibility |
| 3 | `SIMULATION` | Game logic | Cooldowns, state machines, combat |
| 4 | `AI_PLANNING` | Generate PlanIntents | Orchestrator dispatch, arbiter |
| 5 | `PHYSICS` | Forces, collisions | Rapier3D step, spatial hash |
| 6 | `POST_SIMULATION` | Cleanup, constraints | Constraint resolution, GC |
| 7 | `PRESENTATION` | Output | Rendering, audio, UI updates |

Systems are registered per-stage and execute in deterministic order within each stage. The `AiPlanningPlugin` provides `build_app_with_ai()` for automatic AI system registration.

## Memory Safety — Miri Validation

All unsafe code has been validated with Miri (symbolic alignment check, strict provenance):

| Crate | Tests | Unsafe Patterns Validated |
|-------|-------|--------------------------|
| astraweave-ecs | 419 | BlobVec, SparseSet, EntityAllocator, SystemParam |
| astraweave-math | 109 | SIMD intrinsics (SSE2), scalar fallback, edge cases |
| astraweave-core | 503 | Entity::from_raw, capture/replay, schema validation |
| astraweave-sdk | 28 | C ABI FFI, raw pointer handling, handle lifecycle |
| **Total** | **1,059** | **0 undefined behavior** |

Miri flags: `-Zmiri-symbolic-alignment-check -Zmiri-strict-provenance`

## Formal Verification — Kani Proofs

In addition to Miri, critical code paths are formally verified with Kani:

- **69+ proof harnesses** across ECS, Math, Core, and SDK crates
- Proofs cover: entity allocation soundness, SIMD correctness, FFI safety
- All harnesses pass with zero violations
- CI: `.github/workflows/kani.yml`

## Mutation Testing

Test quality validated through `cargo mutants`:

- **Wave 1**: 767 manually-targeted mutation tests across 7 crates
- **Wave 2**: 1,261+ automated mutants (astraweave-prompts: 792 mutants, 100% kill rate)
- Ongoing: terrain, render, editor mutation campaigns

## Determinism

AstraWeave provides deterministic simulation:

- Bit-identical replay across runs
- Position tolerance: < 0.0001
- 100-frame replay validated (1.67 seconds @ 60 FPS)
- 5-run consistency validated (exceeds 3-run target)
- 100 seeds tested (comprehensive RNG validation)
- Fixed-timestep ECS with deterministic system ordering

## Workspace Organization

| Tier | Count | Purpose |
|------|-------|---------|
| **Core Engine** | 12 crates | ECS, Core, Math, Physics, Render, Scene, Nav, Audio, Input, Gameplay, UI, SDK |
| **AI & Intelligence** | 12 crates | AI, Behavior, LLM, Prompts, Embeddings, RAG, Memory, Context, Persona, Coordination, Optimization, LLM-Eval |
| **World & Content** | 6 crates | Terrain, PCG, Weaving, Fluids, Materials, Cinematics |
| **Networking & Persistence** | 8 crates | Net-core, Net-ECS, Client, Server, Proto, Save, Persistence-ECS, Persistence-Player |
| **Tools** | 12 crates | Editor, Asset CLI, Build, Debug, Headless, Release, Profiling, Observability, and more |
| **Examples** | 59 crates | Demos covering all engine subsystems |

[← Back to Home](index.html)
