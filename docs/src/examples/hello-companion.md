# Hello Companion

`examples/hello_companion` is the headless AI showcase. The package name is `hello_companion`.

```bash
cargo run -p hello_companion --release
```

A default run uses `RuleOrchestrator`. It does not panic, and it does not print `LosBlocked`. That error string is not in this example.

## What runs

Without the `visual` feature, `main` parses flags and calls `run_single_demo`.

`setup_world` builds an `astraweave_core::World`, inserts a vertical obstacle column at x = 6, and spawns a player at `(2, 2)`, a companion at `(2, 3)`, and an enemy at `(12, 2)`. The snapshot objective is `extract`. `PerceptionConfig.los_max` is 12.

The selected orchestrator returns a `PlanIntent`. `validate_and_execute` runs it. On `Err`, the example prints the error and continues. It then steps the ECS app with `run_fixed` and prints positions.

This example is not the cross-OS replay job. Printout is not a golden hash.

## Modes

`AIMode` lists seven modes. The crate default features are `fast-alloc` only, so a plain `cargo run` compiles Classical and nothing else.

| Command | Mode | Feature flag |
| --- | --- | --- |
| `cargo run -p hello_companion --release` | Classical, `RuleOrchestrator` | none |
| `--features llm -- --bt` | Behavior tree | `llm` |
| `--features llm -- --utility` | Utility | `llm` |
| `--features llm,ollama -- --llm` | LLM (`Qwen3Ollama`) | `llm,ollama` |
| `--features llm,ollama -- --hybrid` | LLM, then classical fallback | `llm,ollama` |
| `--features llm -- --ensemble` | Ensemble | `llm` |
| `--features llm_orchestrator -- --arbiter` | `GoapOrchestrator` (`legacy_goap_plan`) | `llm_orchestrator` |
| `--features visual` | `visual_demo::run_visual_demo` | `visual` (implies `ollama`) |

Put `--features` before `--`. Cargo consumes the features; the example consumes the flag after `--`.

When the `ollama` feature is on and no mode flag is passed, `select_ai_mode` selects Hybrid. When `llm` is on and `ollama` is off, it selects Behavior Tree. A flag whose feature is missing prints a warning and falls back to Classical.

## Arbiter flag

`--arbiter` does not construct `AIArbiter`. With `llm_orchestrator` it prints that the model is not on this tick and calls `GoapOrchestrator::propose_plan`. `GoapOrchestrator` is the name kept for `htn::legacy_goap_plan`. It is always compiled. It is not the ECS planning system.

`AIArbiter` is a different type. It compiles only behind `astraweave-ai`'s `llm_orchestrator` feature, and it is not called from `sys_ai_planning`. See [AI Arbiter](../core-systems/ai/arbiter.md).

## Models

LLM modes use `astraweave_llm::qwen3_ollama::DEFAULT_QWEN_INSTRUCT_MODEL`, which is `qwen3.5:4b`.

`SystemOrchestratorConfig::default` in `astraweave-ai` uses `phi3:medium` when `OLLAMA_MODEL` is unset. That is the orchestrator default. It is not what `hello_companion`'s Qwen client constant names.

Ollama stays on loopback. A non-loopback URL is rejected before a client stores a key. See the engine status page for the URL rule.

## Visual demo

`--features visual` replaces `main` with the windowed demo. That build needs a GPU and the asset fetch used by other windowed binaries (`cargo xtask fetch-assets`).

## Related

- [Examples index](./index.md)
- [AI system](../core-systems/ai/index.md)
- [Engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md)
