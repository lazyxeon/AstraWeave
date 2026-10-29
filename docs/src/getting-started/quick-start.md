# Quick Start

Install the pinned toolchain, build `hello_companion`, and run the default classical planner.

## Prerequisites

- **Rust**: 1.89.0, pinned in `rust-toolchain.toml`. `rustup` selects that toolchain in this repository.
- **Platform**: Linux, macOS, or Windows. Replay hashes are compared on all three. That is not a claim that every windowed example has been run on all three.
- **GPU**: not required for the default `hello_companion` run. That binary is headless. Windowed examples need a Vulkan, Metal, or Direct3D 12 device that wgpu can open.
- **A local model**: not required for the default run. `--bt`, `--utility`, and `--ensemble` need the `llm` feature and do not open a socket. `--llm` and `--hybrid` need `llm,ollama` and talk to loopback Ollama.

## Installation

### 1. Clone the repository

```bash
git clone https://github.com/lazyxeon/AstraWeave.git
cd AstraWeave
rustup show
```

`rustup show` should report the active toolchain as 1.89.0 inside this directory.

### 2. System packages (Linux, windowed builds)

Headless `hello_companion` does not need these. Windowed crates (`aw_editor`, `physics_demo3d`, `fluids_demo`, `aravath_greybox`) link against the usual X11, Wayland, ALSA, and Vulkan development packages:

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config cmake ninja-build \
  libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev libxcb1-dev \
  libxcb-randr0-dev libxcb-xfixes0-dev libxcb-shape0-dev libxcb-xkb-dev \
  libgl1-mesa-dev libegl1-mesa-dev wayland-protocols libwayland-dev \
  libasound2-dev libpulse-dev libudev-dev mesa-vulkan-drivers vulkan-tools
```

### 3. Build the companion demo

```bash
cargo build -p hello_companion --release
```

## Run hello_companion

```bash
cargo run -p hello_companion --release
```

### What the default run does

The default features are `fast-alloc` only. The process prints a banner, selects **Classical (`RuleOrchestrator`)**, builds a small grid world, proposes a plan, and calls `validate_and_execute`.

A default run does not panic. The smoke `Throw` is aimed at the midpoint, and the obstacle column blocks that throw. `validate_and_execute` returns `EngineError::LosBlocked`. The example prints `⚠️  Execution failed: line of sight blocked. Continuing...` and keeps going. That line is the expected output. The literal `LosBlocked` is not in `examples/hello_companion`.

`setup_world` inserts a vertical obstacle column, then spawns a player, a companion, and an enemy. Perception uses `PerceptionConfig { los_max: 12 }`.

The seven modes, and the features they need, are on the [Hello Companion](../examples/hello-companion.md) page. A default run does not enable all seven. `AIArbiter` is not on the deterministic tick.

## Editor and greybox

```bash
cargo xtask fetch-assets
cargo editor
cargo run -p aravath_runtime --bin aravath_greybox --release
```

`cargo editor` is the alias in `.cargo/config.toml`. It runs `aw_editor`.

## Next steps

- [Hello Companion](../examples/hello-companion.md) for the mode flags
- [Engine status](https://github.com/lazyxeon/AstraWeave/blob/main/docs/current/ENGINE_STATUS.md) for measurements and the replay list
- [Working examples](../examples/index.md)
- [AI systems](../core-systems/ai/index.md)

## Troubleshooting

- `rustc --version` inside the repo should be 1.89.0. The pin is `rust-toolchain.toml`, not "1.89.0 or any newer".
- Graphics errors apply to windowed binaries. The default companion demo does not open a window.
- There is no `unified_showcase` package. See [Rendering showcase](../examples/unified-showcase.md).
