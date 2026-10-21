# Benchmark Dashboard

The benchmark dashboard shows real-time performance metrics for AstraWeave.

[View the Benchmark Dashboard](../benchmarks/)

## Dashboard Features

- **CI Performance Tracking**: Automated benchmarks on every commit
- **Historical Trends**: Performance over time
- **Regression Detection**: Automatic alerts for performance regressions

## Benchmark Categories

| Category | Description |
|----------|-------------|
| ECS | Entity-Component-System operations |
| AI | Planning, perception, behavior trees |
| Rendering | Draw calls, shaders, textures |
| Physics | Collision detection, simulation |

## Running Benchmarks Locally

```bash
cargo bench -p astraweave-ecs
cargo bench -p astraweave-ai
cargo bench -p astraweave-render
```

## See Also

- [Performance Optimization](./dev/performance.md)
- [Performance Tips](./resources/performance.md)
