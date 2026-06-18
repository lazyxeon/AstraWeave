---
layout: default
title: AI Subsystem
---

# AI Orchestration (astraweave-ai)

AstraWeave's AI system supports 7 reasoning modes — from nanosecond classical AI to creative LLM-driven behavior — unified behind the `Orchestrator` and `OrchestratorAsync` traits with a 4-tier fallback system.

## Architecture

```
WorldSnapshot ──► Orchestrator ──► PlanIntent ──► ToolRegistry ──► Validated Actions
                   ▲
        ┌──────────┼──────────────────────┐
        │          │          │           │
  RuleOrch    BehaviorTree  GOAP    phi3 LLM (def)
  (classical)  (3.19 µs)  (3.5 ns)    (2-8 s)
        │          │          │           │
        └──────────┼──────────┘           │
             UtilityOrch (460 ns)         │
                   │                      │
             AIArbiter (GOAP + LLM hybrid)
                   │
             Ensemble (all modes voting)
```

## AI Modes

The `hello_companion` example demonstrates all 7 modes:

| # | Mode | Backend | Latency | Use Case |
|---|------|---------|---------|----------|
| 1 | Classical | `RuleOrchestrator` | ~ns | Rule-based patrol, guard, combat |
| 2 | BehaviorTree | `BehaviorGraph` | 3.19 µs/1K nodes | Hierarchical decision trees |
| 3 | Utility | `UtilityOrchestrator` | 460 ns | Dynamic priority scoring with curves |
| 4 | LLM | Ollama (default `phi3:medium`; Qwen3-8B opt-in) | 2–8 s | Creative, emergent behavior |
| 5 | Hybrid | LLM + fallback chain | 2–8 s | LLM with graceful degradation |
| 6 | Ensemble | All modes voting | ~2.4 s | Maximum accuracy (multiple backends vote) |
| 7 | Arbiter | `AIArbiter` (GOAP + LLM) | 314 ns–8 s | GOAP for tactics, LLM for strategy |

## Core Types

### WorldSnapshot

The perception data structure provided to every AI agent each frame:

```rust
pub struct WorldSnapshot {
    pub t: f32,                        // simulation time
    pub player: PlayerState,           // player hp, pos, stance, orders
    pub me: CompanionState,            // snap.me.pos, snap.me.ammo, snap.me.morale
    pub enemies: Vec<EnemyState>,      // enemy id, pos, hp, cover, last_seen
    pub pois: Vec<Poi>,                // points of interest
    pub obstacles: Vec<IVec2>,         // obstacle positions
    pub objective: Option<String>,     // current mission objective
}

pub struct CompanionState {
    pub ammo: i32,
    pub cooldowns: BTreeMap<String, f32>,
    pub morale: f32,
    pub pos: IVec2,
}

pub struct PlayerState {
    pub hp: i32,
    pub pos: IVec2,
    pub stance: String,
    pub orders: Vec<String>,
}

pub struct EnemyState {
    pub id: Entity,
    pub pos: IVec2,
    pub hp: i32,
    pub cover: String,
    pub last_seen: f32,
}
```

### PlanIntent and ActionStep

```rust
pub struct PlanIntent {
    pub plan_id: String,
    pub steps: Vec<ActionStep>,
}

// 30+ strongly-typed action variants
pub enum ActionStep {
    // Movement
    MoveTo { x: i32, y: i32, speed: Option<MovementSpeed> },
    Approach { target_id: Entity, distance: f32 },
    Retreat { target_id: Entity, distance: f32 },
    TakeCover { position: Option<IVec2> },
    // Offensive
    Attack { target_id: Entity },
    CoverFire { target_id: Entity, duration: f32 },
    ThrowSmoke { x: i32, y: i32 },
    // Support
    Heal { target_id: Option<Entity> },
    // ... and more: equipment, tactical, special actions
}
```

### Orchestrator Traits

```rust
// Synchronous orchestration (classical, BT, utility, GOAP)
pub trait Orchestrator {
    fn propose_plan(&self, snap: &WorldSnapshot) -> PlanIntent;
}

// Asynchronous orchestration (LLM, hybrid, ensemble)
pub trait OrchestratorAsync {
    async fn plan(&self, snap: WorldSnapshot, budget_ms: u32) -> Result<PlanIntent>;
}
```

### AIArbiter (Hybrid Mode)

The arbiter manages GOAP ↔ LLM transitions, using GOAP for real-time tactical decisions and LLM for higher-level strategic planning:

```rust
// Feature-gated: requires `llm_orchestrator` feature
let arbiter = AIArbiter::new(
    llm_executor,                      // LLM backend
    goap_orchestrator,                 // GOAP backend (Box<dyn Orchestrator>)
    bt_orchestrator,                   // BT fallback (Box<dyn Orchestrator>)
);

match arbiter.mode() {
    AIControlMode::GOAP => { /* instant tactical decision */ }
    AIControlMode::ExecutingLLM { step_index } => { /* executing step N of LLM plan */ }
    AIControlMode::BehaviorTree => { /* fallback to behavior tree */ }
}
```

### 4-Tier Fallback System

When the primary AI backend fails, the system gracefully degrades:

| Tier | Strategy | Trigger |
|------|----------|---------|
| 1. **Full LLM** | Structured JSON plan with full WorldSnapshot | Default for LLM/Hybrid modes |
| 2. **Simplified LLM** | Reduced context, shorter prompt | Full LLM timeout or parse error |
| 3. **Heuristic** | Rule-based decision from snapshot analysis | LLM unavailable |
| 4. **Emergency** | Safe default action (take cover / hold position) | All backends fail |

## ECS Integration

The `AiPlanningPlugin` provides turnkey AI integration into the ECS loop:

```rust
use astraweave_ai::ecs_ai_plugin::build_app_with_ai;

// Registers AI systems in the correct stages:
//   PERCEPTION  → build_ai_snapshots
//   AI_PLANNING → orchestrator_tick
build_app_with_ai(&mut app);
```

## Tool Sandbox

All AI actions are validated by the `ToolRegistry` before execution:

- **30+ registered action types** across Movement, Combat, Tactical, Equipment, Support, and Special categories
- Cooldown enforcement prevents action spam
- Range and target validation prevents impossible actions
- Constraint checking ensures game-rule compliance
- **No AI can cheat** — every ActionStep passes through validation

## Behavior Trees

The `astraweave-behavior` crate provides composable behavior nodes:

```rust
use astraweave_behavior::{BehaviorGraph, BehaviorNode, BehaviorContext, BehaviorStatus};

let root = BehaviorNode::Selector(vec![
    BehaviorNode::Sequence(vec![
        BehaviorNode::Condition("check_threat".into()),
        BehaviorNode::Action("throw_smoke".into()),
    ]),
    BehaviorNode::Sequence(vec![
        BehaviorNode::Action("move_to_objective".into()),
    ]),
]);

let graph = BehaviorGraph::new(root);
let status = graph.tick(&BehaviorContext::new(snap));
```

Node types: `Selector`, `Sequence`, `Condition`, `Action`, `Inverter`, `Repeater`, `Parallel`, `Fallback`.

## Performance

| Metric | Value |
|--------|-------|
| GOAP next_action | 3.46–3.56 ns |
| Behavior tree (1K nodes) | 3.19 µs |
| Arbiter GOAP control | 101.7 ns |
| Arbiter LLM polling | 575.3 ns |
| Arbiter mode transition | 221.9 ns |
| Full arbiter cycle | 313.7 ns |
| Utility scoring | 460 ns |
| Classical rule evaluation | 0.20 ms (hello_companion) |
| LLM round-trip | 3,462 ms (hello_companion) |
| **Agents @ 60 FPS** | **12,700+** |
| Anti-cheat checks/sec | **6.48M** |
| Deterministic replay | **100%** bit-identical |

### Agent Capacity at 60 FPS (16.67 ms budget)

| Mode | Max Agents |
|------|------------|
| Arbiter GOAP control | 160,000 |
| Arbiter mode transition | 73,000 |
| Behavior tree | ~5,200 (1K node tree) |
| Full GOAP cycle | 51,000 |
| Arbiter LLM polling | 28,000 |
| GOAP next_action | >>1M |
| Hybrid (1,000 agents) | 0.6% frame budget |

## Test Coverage

- **astraweave-ai**: 268 tests
- **astraweave-behavior**: 233 tests
- **astraweave-core** (schema, tool sandbox): 505 tests

[← Back to Home](index.html) · [Architecture](architecture.html) · [ECS](ecs.html)
