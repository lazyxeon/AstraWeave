# astraweave-ai

AI orchestration and planning layer for AstraWeave.

## Overview

Implements the engine's AI-native architecture: the `Orchestrator` trait abstracts planning backends (rule-based, behavior-tree, LLM, or hybrid), while the core loop drives the perception → reasoning → planning → action pipeline.

## Key Types

| Type | Description |
|------|-------------|
| `Orchestrator` | Trait abstracting AI planning backends |
| `AiPlanningPlugin` | ECS plugin for AI system registration |
| `AIArbiter` | GOAP + Qwen-backed LLM hybrid controller |
| `LlmExecutor` | Async LLM task execution |

## Modules

- **`orchestrator`** — `Orchestrator` trait
- **`core_loop`** — Perception → planning → action pipeline
- **`ecs_ai_plugin`** — ECS integration (`build_app_with_ai()`)
- **`tool_sandbox`** — Runtime action plan validation
- **`goap`** — Goal-Oriented Action Planning (feature-gated)
- **`ai_arbiter`** — GOAP + LLM hybrid control mode switching

## Feature Flags

| Feature | Description |
|---------|-------------|
| `llm_orchestrator` | LLM executor and async task infrastructure |
| `veilweaver_slice` | Veilweaver companion orchestrator |
| `planner_advanced` | GOAP planner with caching and visualization |

## Performance

- GOAP planning: 1.01 µs (cache hit), 47.2 µs (cache miss)
- Behavior trees: 57–253 ns/tick
- Arbiter cycle: 313.7 ns
- Validated: **12,700+ agents @ 60 FPS**

## License

MIT
