# AI API Reference

> **Crate**: `astraweave-ai`  
> **Coverage**: ~75%  
> **Tests**: 400+

The AI system provides the core perception-reasoning-planning-action loop, orchestration, and tool validation for AI-native game development.

## Quick Links

- [rustdoc](https://docs.rs/astraweave-ai) (when published)
- [Source](https://github.com/lazyxeon/AstraWeave/tree/main/astraweave-ai)
- [AI Core Concepts](../core-systems/ai/index.md)
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
use astraweave_ai::AIArbiter;

let arbiter = AIArbiter::new(strategic_executor, Some(fast_executor), goap, bt);
let arbiter = AIArbiter::with_single_executor(strategic_executor, goap, bt);
let action = arbiter.update(&snapshot);
```

`new` takes `Option<LlmExecutor>` as the second argument. Nanosecond timings are not repeated here. The type is not on the sim tick. See [AI Arbiter](../core-systems/ai/arbiter.md).

---

### goap

The `goap` module compiles only with `planner_advanced`. The planner struct in `planner.rs` is `AdvancedGOAP`. There is no `GoapPlanner` type. `Goal::new` takes a name and a `BTreeMap<String, StateValue>`.

`GoapOrchestrator` in `orchestrator.rs` is a different type. It is always compiled, and `propose_plan` calls `htn::legacy_goap_plan`. The deterministic tick uses `RuleOrchestrator`, not either of these.

---

### veilweaver_slice

The feature `veilweaver_slice` compiles `astraweave_ai::veilweaver` and exports `VeilweaverCompanionOrchestrator`. `FateThread`, `Prophecy`, and `ThreadWeaver` are not types in that module. Weaving on the sim path is `astraweave-weaving`, described on the engine status page. The identifier is not the player-facing game name.

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

- [AI Core System](../core-systems/ai/index.md)
- [Arbiter System](../core-systems/ai/arbiter.md)
- [GOAP Guide](../core-systems/ai/planning.md)
- [Behavior Trees](../core-systems/ai/behavior-trees.md)
- [LLM Integration](../core-systems/ai/index.md)
