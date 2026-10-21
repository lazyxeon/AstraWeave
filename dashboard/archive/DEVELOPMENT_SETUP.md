# AstraWeave Gaming Engine - Development Setup Guide

This guide helps you set up a complete development environment for the AstraWeave AI-native gaming engine.

## Quick Start (Recommended)

The fastest way to get started is using our automated bootstrap script:

```bash
# Clone the repository
git clone https://github.com/lazyxeon/AstraWeave-AI-Native-Gaming-Engine.git
cd AstraWeave-AI-Native-Gaming-Engine

# Run the bootstrap script
./scripts/bootstrap.sh
```

The script will:
- 🔍 Detect your platform (Linux, macOS, Windows/WSL)
- 📦 Install all required system dependencies
- 🦀 Set up the Rust toolchain (pinned to 1.89.0)
- 🔧 Install development tools (cargo-audit, cargo-deny, etc.)
- ✅ Validate your installation

## Manual Setup

If you prefer to set up manually or the bootstrap script doesn't work for your platform:

### Prerequisites

#### All Platforms
- **Rust 1.89.0** (managed automatically via `rust-toolchain.toml`)
- **Git** for version control
- **CMake** (3.15+) and **Ninja** build system

#### Linux (Ubuntu/Debian)
```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential pkg-config cmake ninja-build \
    libx11-dev libxi-dev libxcursor-dev libxrandr-dev libxinerama-dev \
    libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev \
    libxcb1-dev libxcb-randr0-dev libxcb-xfixes0-dev libxcb-shape0-dev libxcb-xkb-dev \
    libgl1-mesa-dev libegl1-mesa-dev wayland-protocols libwayland-dev \
    libasound2-dev libpulse-dev libudev-dev mesa-vulkan-drivers vulkan-tools
```

#### Linux (Fedora/RHEL)
```bash
sudo dnf install -y \
    gcc gcc-c++ pkg-config cmake ninja-build \
    libX11-devel libXi-devel libXcursor-devel libXrandr-devel libXinerama-devel \
    libxkbcommon-devel libxkbcommon-x11-devel libxcb-devel \
    mesa-libGL-devel mesa-libEGL-devel wayland-devel wayland-protocols-devel \
    alsa-lib-devel pulseaudio-libs-devel systemd-devel vulkan-devel vulkan-tools
```

#### Linux (Arch)
```bash
sudo pacman -S \
    base-devel pkg-config cmake ninja \
    libx11 libxi libxcursor libxrandr libxinerama \
    libxkbcommon libxkbcommon-x11 libxcb mesa wayland wayland-protocols \
    alsa-lib libpulse systemd vulkan-tools vulkan-icd-loader
```

#### macOS
```bash
# Install Homebrew if needed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install pkg-config cmake ninja
```

#### Windows
Use **Windows Subsystem for Linux (WSL)** with Ubuntu and follow the Linux instructions.

### Rust Installation

If you don't have Rust installed:

```bash
# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Source the environment
source ~/.cargo/env

# The project's rust-toolchain.toml will automatically use Rust 1.89.0
```

### Additional Tools

Install helpful development tools:

```bash
cargo install cargo-audit cargo-deny cargo-watch sccache
```

## Building the Project

### Core Components

Build the essential working components:

```bash
# Using the convenient alias
cargo build-core

# Or manually
cargo build -p astraweave-core -p astraweave-ai -p astraweave-physics \
            -p astraweave-nav -p astraweave-render -p hello_companion
```

### All Working Components

```bash
# Check all working components
cargo check-all

# Build all working components
cargo build-working
```

### Running Tests

```bash
# Run tests for all working components
cargo test-all

# Or run tests manually
cargo test --workspace --lib --exclude astraweave-author \
  --exclude visual_3d --exclude ui_controls_demo --exclude npc_town_demo \
  --exclude rhai_authoring --exclude cutscene_render_demo \
  --exclude weaving_playground --exclude combat_physics_demo \
  --exclude navmesh_demo --exclude physics_demo3d \
  --exclude debug_toolkit_demo --exclude aw_editor
```

## Development Workflow

### Quick Validation

To validate your setup is working:

```bash
# Validate with the bootstrap script
./scripts/bootstrap.sh --validate-only

# Or manually test core compilation
cargo check -p astraweave-core

# Test the hello_companion example
cargo run -p hello_companion --release
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Run linter
cargo clippy-all

# Security audit
cargo audit
cargo deny check
```

### Build Optimization

The project uses several optimizations:

- **sccache**: Compilation caching (automatically configured)
- **Release builds**: Optimized for performance
- **Target-specific optimizations**: Enabled for non-MSVC targets

### Development Tools

#### Useful Cargo Aliases
- `cargo build-core` - Build essential components
- `cargo build-working` - Build all working components
- `cargo check-all` - Check all working components
- `cargo test-all` - Test all working components
- `cargo clippy-all` - Lint all working components

#### File Watching
```bash
# Watch for changes and rebuild
cargo watch -x "check -p astraweave-core"

# Watch and run tests
cargo watch -x "test -p astraweave-core"
```

## Troubleshooting

### Common Issues

#### "alsa-sys build failed"
- **Solution**: Install ALSA development libraries
- **Ubuntu**: `sudo apt-get install libasound2-dev`
- **Fedora**: `sudo dnf install alsa-lib-devel`

#### "X11 libraries not found"
- **Solution**: Install X11 development packages
- **Ubuntu**: `sudo apt-get install libx11-dev libxi-dev libxcursor-dev`

#### "CMake not found"
- **Solution**: Install CMake
- **Ubuntu**: `sudo apt-get install cmake`
- **macOS**: `brew install cmake`

#### "Vulkan drivers missing"
- **Solution**: Install Vulkan support
- **Ubuntu**: `sudo apt-get install mesa-vulkan-drivers vulkan-tools`

#### Build fails on macOS
- **Solution**: Set OpenSSL environment variables:
  ```bash
  export OPENSSL_ROOT_DIR=$(brew --prefix openssl)
  export OPENSSL_LIB_DIR=$(brew --prefix openssl)/lib
  export OPENSSL_INCLUDE_DIR=$(brew --prefix openssl)/include
  ```

### Getting Help

If you encounter issues:

1. **Check the logs**: Build errors usually contain helpful information
2. **Validate setup**: Run `./scripts/bootstrap.sh --validate-only`
3. **Clean build**: `cargo clean && cargo build-core`
4. **Check system dependencies**: Ensure all packages from the prerequisites are installed
5. **Update toolchain**: `rustup update`

### Known Limitations

Some components have compilation issues and are excluded from workspace builds:
- `astraweave-author` (rhai sync/send trait issues)
- `visual_3d` and UI examples (API mismatches with egui/winit)
- Several examples missing `serde_json` dependency

These are actively being worked on and don't affect the core engine functionality.

## Contributing

After setting up your development environment:

1. Create a feature branch: `git checkout -b feature/my-feature`
2. Make your changes
3. Test thoroughly: `cargo test-all && cargo clippy-all`
4. Format code: `cargo fmt --all`
5. Run security checks: `cargo audit && cargo deny check`
6. Submit a pull request

## Performance Tips

### Build Performance
- Use `sccache` for compilation caching (installed by bootstrap script)
- Use `cargo build --release` for performance testing
- Consider `cargo build --release -j $(nproc)` for faster parallel builds

### Development Performance
- Use `cargo check` instead of `cargo build` for faster syntax checking
- Use `cargo watch` for automatic rebuilds during development
- Focus on individual packages: `cargo check -p astraweave-core`

## IDE Setup

### VS Code
Recommended extensions:
- **rust-analyzer**: Language server
- **CodeLLDB**: Debugging support
- **Better TOML**: Configuration file support
- **GitLens**: Git integration

### Other IDEs
The project includes standard Rust configuration files that work with:
- **CLion** (with Rust plugin)
- **Emacs** (with rust-mode)
- **Vim/Neovim** (with rust.vim and coc-rust-analyzer)

## Next Steps

Once your environment is set up:

1. **Explore the examples**: Start with `hello_companion`
2. **Read the documentation**: Check the `docs/` directory
3. **Understand the architecture**: Review `ONE_PAGE_OVERVIEW.md`
4. **Join the community**: Check `CONTRIBUTING.md` for community guidelines

Happy coding! 🦀🎮