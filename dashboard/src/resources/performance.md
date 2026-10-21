# Performance Tips

Quick reference for optimizing your AstraWeave games. For in-depth performance optimization, see the [Performance Guide](../dev/performance.md).

## Quick Wins

### Build Configuration

Always use release builds for performance testing:

```bash
cargo run --release -p your_game
```

### Common Optimizations

| Issue | Solution |
|-------|----------|
| Low FPS | Use release builds, reduce entity count |
| High memory | Limit AI memory buffers, use asset streaming |
| AI lag | Increase planning interval, reduce perception range |
| Render stutter | Enable frustum culling, use LODs |

### AI Performance

```rust
ai_config.planning_interval = Duration::from_millis(500);
ai_config.perception_range = 15.0;
ai_config.max_concurrent_llm_requests = 2;
```

### ECS Batching

Process entities in batches:

```rust
fn process_entities(query: Query<(&Transform, &mut Velocity)>) {
    query.par_iter_mut().for_each(|(transform, mut velocity)| {
        velocity.0 = calculate_velocity(transform);
    });
}
```

## Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Frame time | < 16.67ms | 60 FPS |
| AI tick | < 5ms | Planning + execution |
| Physics step | < 4ms | Collision + dynamics |
| Render | < 8ms | Draw calls + GPU |

## Profiling Tools

- **Tracy**: Real-time frame profiler
- **Cargo flamegraph**: CPU profiling
- **RenderDoc**: GPU debugging

```bash
cargo install cargo-flamegraph
cargo flamegraph --release -p your_game
```

## See Also

- [Performance Guide](../dev/performance.md) - Complete optimization guide
- [Best Practices](best-practices.md) - Architecture patterns
- [Configuration](../reference/configuration.md) - Performance settings
- [Troubleshooting](troubleshooting.md) - Common issues
