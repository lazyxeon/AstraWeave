# FAQ

Frequently asked questions about AstraWeave, the AI-native game engine.

## General

### What is AstraWeave?

AstraWeave is an AI-native game engine built in Rust. Unlike traditional engines that add AI as an afterthought, AstraWeave places intelligent agents at the core of its architecture. AI companions, enemies, and NPCs use the same validated game systems as players, ensuring fair and emergent gameplay.

### How is AstraWeave different from Unity or Unreal?

| Feature | AstraWeave | Unity/Unreal |
|---------|------------|--------------|
| AI Architecture | AI-first, tool-validated | AI as addon/plugin |
| LLM Integration | Native, first-class | Third-party plugins |
| Determinism | Guaranteed by design | Optional, complex |
| Language | Rust (safe, fast) | C#/C++ |
| Multiplayer AI | Same validation for AI & players | Separate systems |

### Is AstraWeave production-ready?

AstraWeave is in active development (pre-1.0). Core systems are functional and tested, but APIs may change. We recommend it for:
- Research and prototyping
- Indie game development
- Learning AI game development
- Contributing to an open-source engine

### What platforms does AstraWeave support?

- **Windows**: Full support (primary development platform)
- **Linux**: Full support
- **macOS**: Supported (Apple Silicon native)
- **WebAssembly**: Experimental (rendering only)
- **Consoles**: Planned for post-1.0

## AI System

### Which LLMs does AstraWeave support?

AstraWeave uses Ollama for local LLM inference. Recommended models:

| Model | Use Case | VRAM Required |
|-------|----------|---------------|
| `hermes2-pro-mistral` | Tool calling, dialogue | 8GB |
| `phi3:mini` | Lightweight inference | 4GB |
| `llama3:8b` | General purpose | 8GB |
| `mistral:7b` | Fast inference | 8GB |

### Can I use cloud LLMs like GPT-4 or Claude?

Not currently. AstraWeave is designed for local inference to ensure:
- Deterministic behavior
- Low latency (8ms budget per tick)
- Privacy (no data leaves the machine)
- Offline gameplay

Cloud support may be added post-1.0 for specific use cases.

### How does AI validation work?

All AI actions go through the Tool Validation System:

```rust
let tool_call = ToolCall {
    tool: "move_to",
    params: json!({ "target": [10.0, 0.0, 5.0] }),
};

match validator.validate(&tool_call, &world_state) {
    ToolResult::Success(action) => execute(action),
    ToolResult::Blocked(reason) => handle_failure(reason),
}
```

This prevents AI from cheating - it can only perform actions that would be valid for a player.

### Why does my AI companion not respond?

Common causes:
1. **Ollama not running**: Start with `ollama serve`
2. **Model not pulled**: Run `ollama pull hermes2-pro-mistral`
3. **Port conflict**: Check Ollama is on port 11434
4. **Timeout**: AI has an 8ms budget per tick; complex queries may timeout

## Performance

### What are the minimum system requirements?

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 4 cores, 2.5 GHz | 8 cores, 3.5 GHz |
| RAM | 8 GB | 16 GB |
| GPU | GTX 1060 / RX 580 | RTX 3070 / RX 6800 |
| VRAM | 4 GB | 8 GB (for LLM) |
| Storage | 10 GB | 50 GB SSD |

### Why is my game running slowly?

1. **Debug builds**: Always use `--release` for playable performance
2. **LLM inference**: Check Ollama is GPU-accelerated
3. **Physics overhead**: Reduce collision complexity
4. **AI agents**: Limit concurrent planning operations

### How do I profile my game?

AstraWeave integrates with Tracy profiler:

```bash
cargo run -p your_game --release --features profiling
```

See [Performance Optimization](../dev/performance.md) for details.

## ECS Architecture

### Why does AstraWeave use its own ECS?

AstraWeave's ECS is designed for:
- **Deterministic iteration**: Same order every tick
- **Generational entities**: Safe entity references
- **AI integration**: Perception and planning systems
- **Networking**: State synchronization

It's inspired by Bevy but optimized for AI-native gameplay.

### How do I create entities?

```rust
let entity = world.spawn((
    Transform::from_xyz(0.0, 1.0, 0.0),
    RigidBody::Dynamic,
    Collider::sphere(0.5),
    Health { current: 100.0, max: 100.0 },
));
```

### How do I query entities?

```rust
fn damage_system(
    mut query: Query<(&mut Health, &Transform), With<Enemy>>,
) {
    for (mut health, transform) in query.iter_mut() {
        if in_danger_zone(transform) {
            health.current -= 10.0;
        }
    }
}
```

## Building and Development

### How do I build from source?

```bash
git clone https://github.com/lazyxeon/AstraWeave
cd AstraWeave
cargo build --release
```

### Why is the first build so slow?

AstraWeave has many dependencies. First builds compile:
- wgpu (graphics)
- rapier (physics)
- rodio (audio)
- tokio (async runtime)

Subsequent builds are much faster due to incremental compilation.

### How do I run the examples?

```bash
cargo run -p hello_companion --release
cargo run -p physics_demo3d --release
cargo run -p unified_showcase --release
```

### How do I contribute?

See [Contributing Guide](../dev/contributing.md). In brief:
1. Fork the repository
2. Create a feature branch
3. Make changes with tests
4. Submit a pull request

## Troubleshooting

### "LosBlocked" error in hello_companion

This is expected behavior! The example demonstrates that AI cannot perform invalid actions. The companion tries to move but line-of-sight is blocked, proving the validation system works.

### Shader compilation errors

Ensure your GPU driver is up to date. WGPU requires:
- Vulkan 1.1+ on Windows/Linux
- Metal on macOS
- WebGPU on browsers

### "Entity not found" panic

This usually means an entity was despawned while still referenced. Use generational entities:

```rust
if world.get::<Health>(entity).is_some() {
    // Entity still exists
}
```

### Audio not playing

Check audio device availability and ensure the audio example works:

```bash
cargo run -p audio_spatial_demo --release
```

## Licensing

### What license is AstraWeave under?

AstraWeave uses a dual license:
- **Apache 2.0**: For most use cases
- **MIT**: Alternative option

Commercial use is permitted under both licenses.

### Can I use AstraWeave for commercial games?

Yes! Both Apache 2.0 and MIT licenses permit commercial use. Attribution is appreciated but not required.

## Getting Help

### Where can I get help?

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: Questions and community chat
- **Documentation**: This site
- **Examples**: `examples/` directory in the repository

### How do I report a bug?

Open a GitHub issue with:
1. AstraWeave version
2. Operating system
3. Steps to reproduce
4. Expected vs actual behavior
5. Error messages or logs
