# AI API Reference

> **Crate**: `astraweave-ai`  
> **Coverage**: ~75%  
> **Tests**: 400+

The AI system provides the core perception-reasoning-planning-action loop, orchestration, and tool validation for AI-native game development.

## Quick Links

- [rustdoc](https://docs.rs/astraweave-ai) (when published)
- [Source](https://github.com/astraweave/astraweave/tree/main/astraweave-ai)
- [AI Core Concepts](../core-systems/ai-core.md)
- [Arbiter System](../core-systems/ai/arbiter.md)

---

## Core Modules

### core_loop

The fundamental perception → reasoning → planning → action cycle.

```rust
use astraweave_ai::core_loop::{CoreLoop, WorldSnapshot, PlanIntent, ActionStep};

// Build perception
let snapshot = WorldSnapshot {
    t: game_time,
    player: player_state,
    me: companion_state,
    enemies: detected_enemies,
    pois: points_of_interest,
    obstacles: obstacle_positions,
    objective: current_objective,
};

// Execute AI cycle
let plan = orchestrator.plan(&mut world, &snapshot)?;

// Execute first action
if let Some(action) = plan.steps.first() {
    execute_action(action);
}
```

**Key Types**:
- `WorldSnapshot` - Filtered world state for AI perception
- `PlanIntent` - Validated action sequence from AI
- `ActionStep` - Individual executable action

---

### orchestrator

Trait-based AI planning abstraction.

```rust
use astraweave_ai::orchestrator::Orchestrator;

pub trait Orchestrator: Send + Sync {
    fn plan(&self, world: &mut World, snap: &WorldSnapshot) -> Result<PlanIntent>;
}

// Implementations provided:
// - RuleOrchestrator (classical if-then rules)
// - GoapOrchestrator (goal-oriented planning)
// - LlmOrchestrator (LLM-based reasoning, feature-gated)
// - HybridOrchestrator (GOAP + LLM)
```

---

### tool_sandbox

Secure action validation preventing impossible/cheating actions.

```rust
use astraweave_ai::tool_sandbox::{ToolSandbox, ToolResult};

let sandbox = ToolSandbox::new();

// Validate action before execution
match sandbox.validate(&action, &world_state) {
    ToolResult::Valid => execute(action),
    ToolResult::Invalid(reason) => {
        log::warn!("AI attempted invalid action: {}", reason);
    }
}
```

**Validation Examples**:
- Movement range limits
- Line-of-sight requirements
- Resource availability checks
- Cooldown enforcement

---

## Feature-Gated Types

### AIArbiter (requires `llm_orchestrator`)

Hybrid GOAP+LLM system with zero user-facing latency.

```rust
#[cfg(feature = "llm_orchestrator")]
use astraweave_ai::{AIArbiter, LlmExecutor, GoapOrchestrator, RuleOrchestrator};

let mut arbiter = AIArbiter::new(llm_executor, goap, bt);

// Game loop - returns instantly (101.7 ns)
let action = arbiter.update(&snapshot);
```

**Performance**: 101.7 ns GOAP control, 575 ns LLM polling

See [Arbiter System](../core-systems/ai/arbiter.md) for full documentation.

---

### goap

Goal-Oriented Action Planning implementation.

```rust
use astraweave_ai::goap::{GoapPlanner, Goal, Action, WorldState};

let mut planner = GoapPlanner::new();

// Define goals
planner.add_goal(Goal::new("kill_enemy")
    .with_precondition("enemy_visible", true)
    .with_effect("enemy_dead", true));

// Define actions
planner.add_action(Action::new("attack")
    .with_precondition("has_weapon", true)
    .with_precondition("in_range", true)
    .with_effect("enemy_dead", true)
    .with_cost(1.0));

// Plan
let plan = planner.plan(&current_state, &goal_state)?;
```

---

### veilweaver

Integration for the Veilweaver game mechanics (fate-weaving).

```rust
use astraweave_ai::veilweaver::{FateThread, Prophecy, ThreadWeaver};

let mut weaver = ThreadWeaver::new();
let thread = weaver.create_thread(prophecy);

// Weave fate during gameplay
weaver.weave(&mut world, thread)?;
```

---

## WorldSnapshot

The AI's view of the world (filtered for perception).

```rust
pub struct WorldSnapshot {
    pub t: f32,                        // Current game time
    pub player: PlayerState,           // Player information
    pub me: CompanionState,            // This AI's state
    pub enemies: Vec<EnemyState>,      // Detected enemies
    pub pois: Vec<Poi>,                // Points of interest
    pub obstacles: Vec<IVec2>,         // Obstacle positions
    pub objective: Option<String>,     // Current objective
}

pub struct CompanionState {
    pub pos: IVec2,                    // Position
    pub ammo: i32,                     // Ammunition
    pub cooldowns: BTreeMap<String, f32>, // Ability cooldowns
    pub morale: f32,                   // Morale level
}
```

---

## PlanIntent

Validated action sequence from AI reasoning.

```rust
pub struct PlanIntent {
    pub plan_id: String,               // Unique plan identifier
    pub steps: Vec<ActionStep>,        // Ordered actions
}

pub enum ActionStep {
    MoveTo { x: i32, y: i32 },
    Attack { target: u32, stance: String },
    TakeCover { position: Option<(i32, i32)> },
    UseAbility { ability: String, target: Option<u32> },
    Wait { duration: f32 },
    Interact { object: u32 },
}
```

---

## Orchestrator Implementations

| Orchestrator | Latency | Use Case |
|--------------|---------|----------|
| `RuleOrchestrator` | ~100 ns | Simple if-then logic |
| `GoapOrchestrator` | 3-50 µs | Goal-oriented planning |
| `BehaviorTreeOrchestrator` | ~200 ns | Behavior trees |
| `UtilityOrchestrator` | ~500 ns | Utility-based scoring |
| `LlmOrchestrator` | 13-21s | Deep reasoning (async) |
| `HybridOrchestrator` | ~100 ns | GOAP + async LLM |

---

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `llm_orchestrator` | LLM-based planning | ❌ |
| `goap` | Goal-oriented planning | ✅ |
| `behavior_tree` | BT integration | ✅ |
| `utility` | Utility AI | ❌ |

```toml
[dependencies]
astraweave-ai = { version = "0.4", features = ["llm_orchestrator"] }
```

---

## Performance

| Operation | Latency | Notes |
|-----------|---------|-------|
| WorldSnapshot build | ~500 ns | Perception gathering |
| GOAP planning | 3-50 µs | Depends on action space |
| Tool validation | ~100 ns | Per action |
| Full AI cycle | ~5 µs | Typical case |

---

## See Also

- [AI Core System](../core-systems/ai-core.md)
- [Arbiter System](../core-systems/ai/arbiter.md)
- [GOAP Guide](../core-systems/goap.md)
- [Behavior Trees](../core-systems/behavior-trees.md)
- [LLM Integration](./llm.md)
