# Quick Start

Get up and running with AstraWeave in minutes! This guide will help you install the engine, run your first AI companion, and understand the basic concepts.

## Prerequisites

- **Rust**: 1.89.0+ (managed automatically via `rust-toolchain.toml`)
- **Platform**: Linux, macOS, or Windows
- **GPU**: Vulkan-compatible graphics card
- **Memory**: 4GB+ RAM recommended for AI models

## Installation

### 1. Clone the Repository

```bash
git clone https://github.com/lazyxeon/AstraWeave.git
cd AstraWeave
```

### 2. System Dependencies (Linux)

If you're on Linux, install the required system packages:

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config cmake ninja-build \
  libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev libxcb1-dev \
  libxcb-randr0-dev libxcb-xfixes0-dev libxcb-shape0-dev libxcb-xkb-dev \
  libgl1-mesa-dev libegl1-mesa-dev wayland-protocols libwayland-dev \
  libasound2-dev libpulse-dev libudev-dev mesa-vulkan-drivers vulkan-tools
```

### 3. Build Core Components

Build the stable, working core components:

```bash
cargo build -p astraweave-core -p astraweave-ai -p astraweave-physics \
            -p astraweave-nav -p astraweave-render -p hello_companion
```

This typically takes 8-15 seconds after initial dependency download.

## Your First AI Companion

Let's run the most basic example to see AstraWeave in action:

```bash
cargo run -p hello_companion --release
```

### What You'll See

The demo will show:
1. **AI Perception**: The companion perceives the world state
2. **Planning**: AI generates a plan using its understanding
3. **Tool Validation**: The engine validates what the AI wants to do
4. **Expected Panic**: The demo will panic with "LosBlocked" - this is expected behavior demonstrating the validation system

### Example Output

```
[INFO] AI Companion initialized
[INFO] Perception snapshot captured: 1 entities
[INFO] Planning phase: generating intent for companion
[INFO] Generated plan: MoveTo(target_position)
[INFO] Validating tool usage: MovementTool
[ERROR] Validation failed: LosBlocked
thread 'main' panicked at 'LOS validation failed'
```

**This panic is intentional!** It demonstrates AstraWeave's core principle: *the AI can only do what the engine validates as possible*.

## Understanding What Happened

The hello_companion example showcases AstraWeave's fundamental architecture:

1. **Fixed-Tick Simulation**: The world runs at deterministic 60Hz
2. **AI Perception**: AI agents receive structured world snapshots
3. **Planning Layer**: AI generates intentions using LLM-based planning
4. **Tool Validation**: Engine validates every AI action before execution
5. **Safety First**: Invalid actions are rejected, maintaining game integrity

## Next Steps

Now that you've seen the core loop in action:

- **Learn the Architecture**: Read [AI-Native Design](../architecture/ai-native.md)
- **Build Your First Game**: Follow [Building Your First Game](../game-dev/first-game.md)  
- **Explore More Examples**: Check out [Working Examples](../examples/index.md)
- **Dive Deeper**: Study [Core Systems](../core-systems/ai/index.md)

## Troubleshooting

### Build Errors

If you encounter build errors:
- Make sure you have the correct Rust version (check `rust-toolchain.toml`)
- Install system dependencies for your platform
- Some examples have known compilation issues - stick to the working core components listed above

### Runtime Issues

- **Graphics errors**: Ensure you have Vulkan drivers installed
- **Audio errors**: Install audio system dependencies (ALSA/PulseAudio on Linux)
- **Permission errors**: Make sure your user can access graphics and audio devices

For more help, see [Troubleshooting](../resources/troubleshooting.md).

---

**🎉 Congratulations!** You've successfully run your first AstraWeave AI companion. The engine validated the AI's actions and maintained world integrity, just as it should.