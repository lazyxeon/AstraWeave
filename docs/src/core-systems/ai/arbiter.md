# AI Arbiter

This page is the guide to `AIArbiter` and to the `--arbiter` flag on `hello_companion`. They are different.

## Run the flag that exists today

From the repository root:

```bash
cargo run -p hello_companion --release --features llm_orchestrator -- --arbiter
```

`examples/hello_companion/src/main.rs`, arm `AIMode::Arbiter`, prints `The model is not on this tick. The sim validates the proposed steps.` and calls `generate_goap_plan`. That function builds `GoapOrchestrator` and calls `propose_plan`. `GoapOrchestrator` lives in `astraweave-ai/src/orchestrator.rs` and forwards to `htn::legacy_goap_plan`. It does not construct `AIArbiter`.

Without the feature, the same flag prints `Arbiter mode requires --features llm_orchestrator` and falls back to Classical.

A default run, no flag and no extra features, is `RuleOrchestrator`:

```bash
cargo run -p hello_companion --release
```

Walk that path on [Hello Companion](../../examples/hello-companion.md).

## Where the type lives

`astraweave-ai/src/ai_arbiter.rs` is compiled only with feature `llm_orchestrator` (`astraweave-ai/src/lib.rs`). The feature is off by default. `sys_ai_planning` does not call `AIArbiter`. The tick uses `RuleOrchestrator`.

Issue **#334** is open: delete `AIArbiter` once benches have a deterministic replacement. Issue **#383** is the port onto the deterministic HTN tick. The root README lists both under work that is not on `main` as finished work. The [engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md) page says the same.

Comments on `update` name microsecond targets. Those comments are not a dated capture. This page does not repeat nanosecond tables.

## Construct it

`AIArbiter::new` takes four arguments. The second is `Option<LlmExecutor>`.

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
    let mut arbiter = AIArbiter::with_single_executor(strategic_executor, goap, bt)
        .with_llm_cooldown(15.0);
    let action = arbiter.update(snapshot);
    let mode = arbiter.mode();
    let _ = (action, mode);
}
```

Signatures in `astraweave-ai/src/ai_arbiter.rs`:

- `new(strategic_executor, fast_executor, goap, bt) -> Self` with `fast_executor: Option<LlmExecutor>` and the last two `Box<dyn Orchestrator>`
- `with_single_executor(llm_executor, goap, bt)` calls `new(llm_executor, None, goap, bt)`
- `with_llm_cooldown(mut self, cooldown: f32) -> Self` stores the seconds. The field starts at `15.0`
- `update(&mut self, snap: &WorldSnapshot) -> ActionStep`
- `mode(&self) -> AIControlMode`

`LlmExecutor::new` (`astraweave-ai/src/llm_executor.rs`) takes `Arc<dyn OrchestratorAsync + Send + Sync>` and a `tokio::runtime::Handle`. Building that executor means an orchestrator and a runtime. The hello_companion flag does not do that.

`AIControlMode` is:

- `GOAP` (default)
- `ExecutingLLM { step_index: usize }`
- `BehaviorTree`

## What `update` does

`update` polls an in-flight LLM task if one exists. `Ok(plan)` moves to `ExecutingLLM`. `Err` increments `llm_failures` and stays in GOAP. In `GOAP` it may call `maybe_request_llm`, then `self.goap.propose_plan(snap)`, and returns the first step. An empty GOAP plan warns and asks `self.bt`. The fast executor field is stored and marked `dead_code` in this file. The comment says dedicated fast-path dispatch is not wired.

Doc comments on the struct say Qwen3. `SystemOrchestratorConfig::default` in `astraweave-ai/src/orchestrator.rs` uses `phi3:medium` when `OLLAMA_MODEL` is unset. `hello_companion` LLM modes use `DEFAULT_QWEN_INSTRUCT_MODEL` (`qwen3.5:4b`).

## Check that the type builds

```bash
cargo check -p astraweave-ai --features llm_orchestrator
```

That compiles the module. It does not put the type on the sim tick, and it does not run the hello_companion flag.

`GoapOrchestrator` (always compiled) and `astraweave_ai::goap::AdvancedGOAP` (feature `planner_advanced` only) are different types. `astraweave-ai` has no `GoapPlanner`. `astraweave-behavior` defines `GoapPlanner` in `astraweave-behavior/src/goap.rs`.

## Related

- [Hello Companion](../../examples/hello-companion.md)
- [Planning](./planning.md)
- [Behavior trees](./behavior-trees.md)
- [AI system](./index.md)
