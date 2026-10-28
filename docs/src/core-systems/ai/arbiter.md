# AI Arbiter

`AIArbiter` is a feature-gated type in `astraweave-ai`. It is not the planner on the deterministic tick, and it is not what `hello_companion --arbiter` runs.

Current state, from the tree and the [engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md) page:

- The module `astraweave-ai/src/ai_arbiter.rs` is compiled only with the `llm_orchestrator` feature. That feature is off by default.
- `sys_ai_planning` does not call `AIArbiter`. The default tick uses `RuleOrchestrator`.
- Issue **#334** is open: delete `AIArbiter` once benches have a deterministic replacement. Issue **#383** is the port into the deterministic HTN tick. Neither is on `main` as finished work.
- Nanosecond tables (101.7 ns, 575 ns, 221.9 ns) are not on the engine status page. They are not repeated here. Comments on `update` state a target under 100 µs. That comment is not a dated capture.

## Constructors

`AIArbiter::new` takes four arguments. The second is `Option<LlmExecutor>`.

```rust
#[cfg(feature = "llm_orchestrator")]
use astraweave_ai::AIArbiter;

let arbiter = AIArbiter::new(strategic_executor, Some(fast_executor), goap, bt);
let arbiter = AIArbiter::with_single_executor(strategic_executor, goap, bt);
let arbiter = arbiter.with_llm_cooldown(15.0);
```

`with_single_executor` passes `None` for the fast executor. The cooldown field starts at 15 seconds. `update` takes a `WorldSnapshot` and returns an `ActionStep`.

`GoapOrchestrator` in `astraweave-ai/src/orchestrator.rs` is a different type. It is always compiled. `propose_plan` calls `htn::legacy_goap_plan`. The advanced `astraweave_ai::goap` module compiles only with `planner_advanced`.

## hello_companion

```bash
cargo run -p hello_companion --release --features llm_orchestrator -- --arbiter
```

That flag does not construct `AIArbiter`. The example prints that the model is not on the tick and calls `GoapOrchestrator`. See [Hello Companion](../../examples/hello-companion.md).

A default `cargo run -p hello_companion --release` is Classical `RuleOrchestrator` and does not panic.

## Model names

Doc comments on `AIArbiter` say Qwen3. `SystemOrchestratorConfig::default` uses `phi3:medium` when `OLLAMA_MODEL` is unset. The Qwen client constant `DEFAULT_QWEN_INSTRUCT_MODEL` is `qwen3.5:4b`.

## Related

- [AI system](./index.md)
- [Planning](./planning.md)
- [Behavior trees](./behavior-trees.md)
