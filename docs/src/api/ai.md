# AI API Reference

> **Crate**: `astraweave-ai`  
> **Manifest version**: `0.1.0` (`astraweave-ai/Cargo.toml`). The workspace package version is `0.4.0`. This crate does not set `publish`.

## Quick Links

- [Source](https://github.com/lazyxeon/AstraWeave/tree/main/astraweave-ai)
- [AI Core Concepts](../core-systems/ai/index.md)
- [Arbiter System](../core-systems/ai/arbiter.md)

---

## Core Modules

### core_loop

`astraweave-ai/src/core_loop.rs` exports `PlannerMode`, `CAiController`, and `dispatch_planner`. There is no `CoreLoop` type. `WorldSnapshot`, `PlanIntent`, and `ActionStep` live in `astraweave-core`.

`PlannerMode` is `Rule`, `BehaviorTree`, `GOAP`, and `Htn`. The behavior-tree import in this file is `cfg(feature = "ai-bt")`.

### orchestrator

```rust
use astraweave_ai::orchestrator::Orchestrator;
use astraweave_core::{PlanIntent, WorldSnapshot};

pub trait Orchestrator {
    fn propose_plan(&self, snap: &WorldSnapshot) -> PlanIntent;
}
```

Types in `orchestrator.rs`: `RuleOrchestrator`, `UtilityOrchestrator`, `GoapOrchestrator`, and `LlmOrchestrator<C>`. There is no `BehaviorTreeOrchestrator` and no `HybridOrchestrator`.

`OrchestratorAsync::plan` is `async fn plan(&self, snap: WorldSnapshot, budget_ms: u32) -> Result<PlanIntent>`.

### tool_sandbox

There is no `ToolSandbox` struct and no `ToolResult` enum. The function in `astraweave-ai/src/tool_sandbox.rs` is:

```text
validate_tool_action(agent_id, verb, world, context, target_pos) -> Result<()>
```

`verb` is `ToolVerb`. `world` is `&WorldSnapshot`. `context` is `&ValidationContext`.

---

## Feature-Gated Types

### AIArbiter (requires `llm_orchestrator`)

```rust
#[cfg(feature = "llm_orchestrator")]
fn arbiter_from_new() {
    use std::sync::Arc;

    use astraweave_ai::orchestrator::{Orchestrator, RuleOrchestrator};
    use astraweave_ai::{AIArbiter, LlmExecutor};

    let runtime = tokio::runtime::Handle::current();
    let strategic_executor = LlmExecutor::new(Arc::new(RuleOrchestrator::new()), runtime.clone());
    let fast_executor = LlmExecutor::new(Arc::new(RuleOrchestrator::new()), runtime);
    let goap: Box<dyn Orchestrator> = Box::new(RuleOrchestrator::new());
    let bt: Box<dyn Orchestrator> = Box::new(RuleOrchestrator::new());
    let _built = AIArbiter::new(strategic_executor, Some(fast_executor), goap, bt);
}
```

```rust
#[cfg(feature = "llm_orchestrator")]
fn arbiter_from_single_executor(snapshot: &astraweave_core::WorldSnapshot) {
    use std::sync::Arc;

    use astraweave_ai::orchestrator::{Orchestrator, RuleOrchestrator};
    use astraweave_ai::{AIArbiter, LlmExecutor};

    let runtime = tokio::runtime::Handle::current();
    let strategic_executor = LlmExecutor::new(Arc::new(RuleOrchestrator::new()), runtime);
    let goap: Box<dyn Orchestrator> = Box::new(RuleOrchestrator::new());
    let bt: Box<dyn Orchestrator> = Box::new(RuleOrchestrator::new());
    let mut arbiter = AIArbiter::with_single_executor(strategic_executor, goap, bt);
    let action = arbiter.update(snapshot);
    let _ = action;
}
```

`update` takes `&mut self`. `new` takes `Option<LlmExecutor>` as the second argument. The type is not on the sim tick. See [AI Arbiter](../core-systems/ai/arbiter.md).

---

### goap

The `goap` module compiles only with `planner_advanced`. The planner struct in `planner.rs` is `AdvancedGOAP`. `astraweave-ai` has no `GoapPlanner` type. `astraweave-behavior` defines `GoapPlanner` in `astraweave-behavior/src/goap.rs`. `Goal::new` takes a name and a `BTreeMap<String, StateValue>`.

`GoapOrchestrator` in `orchestrator.rs` is a different type. It is always compiled, and `propose_plan` calls `htn::legacy_goap_plan`. The deterministic tick uses `RuleOrchestrator`, not either of these.

---

### veilweaver_slice

The feature `veilweaver_slice` compiles `astraweave_ai::veilweaver` and exports `VeilweaverCompanionOrchestrator`. `FateThread`, `Prophecy`, and `ThreadWeaver` are not types in that module. Weaving on the sim path is `astraweave-weaving`, described on the engine status page. The identifier is not the player-facing game name.

---

## WorldSnapshot

Defined in `astraweave-core/src/schema.rs`:

```rust
pub struct WorldSnapshot {
    pub t: f32,
    pub player: PlayerState,
    pub me: CompanionState,
    pub enemies: Vec<EnemyState>,
    pub pois: Vec<Poi>,
    pub obstacles: Vec<IVec2>,
    pub objective: Option<String>,
    pub hazards: [WeaveHazard; MAX_WEAVE_HAZARDS],
    pub hazard_count: u8,
}

pub struct CompanionState {
    pub ammo: i32,
    pub cooldowns: BTreeMap<String, f32>,
    pub morale: f32,
    pub pos: IVec2,
}
```

---

## PlanIntent

```rust
pub struct PlanIntent {
    pub plan_id: String,
    pub steps: Vec<ActionStep>,
}
```

`ActionStep` is the enum in the same file. Two variants the crate's own example constructs:

- `MoveTo { x: i32, y: i32, speed: Option<MovementSpeed> }`
- `Attack { target_id: Entity }`

`Wait { duration }` and `Interact { object }` are not those variants. Read `schema.rs` for the rest.

---

## Orchestrator Implementations

| Orchestrator | In `orchestrator.rs` |
|--------------|----------------------|
| `RuleOrchestrator` | always compiled |
| `UtilityOrchestrator` | always compiled |
| `GoapOrchestrator` | always compiled. Calls `htn::legacy_goap_plan` |
| `LlmOrchestrator<C>` | feature `llm_orchestrator` |

This page does not repeat a latency table. The numbers in the crate's module docs are not a dated capture.

---

## Feature Flags

`default = []` in `astraweave-ai/Cargo.toml`.

| Feature | What it turns on |
|---------|------------------|
| `llm_orchestrator` | `astraweave-llm` with `ollama`, plus `tokio` and `futures` |
| `ai-goap` | optional `astraweave-behavior` |
| `ai-bt` | optional `astraweave-behavior` |
| `planner_advanced` | advanced GOAP module |
| `rag` | declared, empty |
| `profiling` | Tracy client |
| `alloc-counter` | allocation counter |
| `fast-alloc` | `astraweave-alloc` |
| `veilweaver_slice` | behavior crate and observability |
| `decision-trace` | `decision_trace` module |

There is no `goap`, `behavior_tree`, or `utility` feature.

---

## See Also

- [AI Core System](../core-systems/ai/index.md)
- [Arbiter System](../core-systems/ai/arbiter.md)
- [GOAP Guide](../core-systems/ai/planning.md)
- [Behavior Trees](../core-systems/ai/behavior-trees.md)
- [LLM Integration](../core-systems/ai/index.md)
