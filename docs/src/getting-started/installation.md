# Installation Guide

This guide covers detailed installation instructions for AstraWeave on different platforms, including all dependencies and troubleshooting common issues.

## System Requirements

### Minimum Requirements
- **CPU**: x64 processor with SSE2 support
- **Memory**: 4GB RAM (8GB+ recommended for AI models)
- **GPU**: Vulkan 1.0 compatible graphics card
- **Storage**: 2GB free space (more for AI models)
- **Rust**: 1.89.0+ (managed via rust-toolchain.toml)

### Recommended Requirements  
- **CPU**: Multi-core x64 processor (4+ cores)
- **Memory**: 16GB+ RAM for development and multiple AI models
- **GPU**: Modern Vulkan 1.2+ compatible GPU with 2GB+ VRAM
- **Storage**: SSD with 10GB+ free space

## Platform-Specific Installation

### Linux

#### Ubuntu/Debian
```bash
# Update package lists
sudo apt-get update

# Install build essentials
sudo apt-get install -y build-essential pkg-config cmake ninja-build

# Install graphics dependencies
sudo apt-get install -y libx11-dev libxi-dev libxcursor-dev libxrandr-dev \
  libxinerama-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-xcb-dev \
  libxcb1-dev libxcb-randr0-dev libxcb-xfixes0-dev libxcb-shape0-dev \
  libxcb-xkb-dev

# Install rendering dependencies
sudo apt-get install -y libgl1-mesa-dev libegl1-mesa-dev wayland-protocols \
  libwayland-dev mesa-vulkan-drivers vulkan-tools

# Install audio dependencies  
sudo apt-get install -y libasound2-dev libpulse-dev

# Install additional system dependencies
sudo apt-get install -y libudev-dev
```

#### Arch Linux
```bash
# Install base development tools
sudo pacman -S base-devel cmake ninja

# Install graphics and audio
sudo pacman -S vulkan-devel mesa alsa-lib libpulse wayland wayland-protocols

# Install X11 dependencies
sudo pacman -S libx11 libxcb libxrandr libxinerama libxcursor libxi
```

#### Fedora/RHEL
```bash
# Install development tools
sudo dnf groupinstall "Development Tools"
sudo dnf install cmake ninja-build pkg-config

# Install graphics dependencies
sudo dnf install libX11-devel libXi-devel libXcursor-devel libXrandr-devel \
  libXinerama-devel libxkbcommon-devel libxkbcommon-x11-devel

# Install Vulkan and Mesa
sudo dnf install vulkan-devel mesa-dri-drivers

# Install audio
sudo dnf install alsa-lib-devel pulseaudio-libs-devel
```

### macOS

#### Prerequisites
First, install Xcode Command Line Tools:
```bash
xcode-select --install
```

#### Using Homebrew (Recommended)
```bash
# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install cmake ninja pkg-config

# For Intel Macs, ensure MoltenVK is installed
brew install molten-vk
```

#### Manual Installation
- Download and install Xcode from the App Store
- Install CMake from [cmake.org](https://cmake.org/download/)
- Ensure MoltenVK is available for Vulkan support

### Windows

#### Using Visual Studio (Recommended)
1. Install Visual Studio 2019 or later with C++ build tools
2. Install Git for Windows
3. Install CMake (either standalone or via Visual Studio Installer)

#### Using MSYS2/MinGW
```bash
# Install MSYS2 from https://www.msys2.org/
# Then in MSYS2 terminal:
pacman -S mingw-w64-x86_64-cmake mingw-w64-x86_64-ninja
pacman -S mingw-w64-x86_64-vulkan-devel
```

## Rust Installation

AstraWeave uses a specific Rust version defined in `rust-toolchain.toml`. The installation process will automatically use the correct version.

### Install Rust
```bash
# Install rustup (Rust installer)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow the prompts, then restart your terminal or run:
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### Rust Components
The following components will be installed automatically when needed:
- `cargo` - Package manager and build tool
- `clippy` - Linter for catching common mistakes
- `rustfmt` - Code formatter
- `rust-analyzer` - Language server for IDE support

## Clone and Build

### 1. Clone the Repository
```bash
git clone https://github.com/lazyxeon/AstraWeave.git
cd AstraWeave
```

### 2. Verify Rust Toolchain
The correct Rust version will be installed automatically:
```bash
# This will show the version from rust-toolchain.toml
rustc --version
```

### 3. Build Core Components
Start with the stable, working components:
```bash
cargo build -p astraweave-core -p astraweave-ai -p astraweave-physics \
            -p astraweave-nav -p astraweave-render -p hello_companion
```

### 4. Run Tests
Verify the installation with tests:
```bash
cargo test -p astraweave-input
```

### 5. Run Example
Test the installation:
```bash
cargo run -p hello_companion --release
```

## Verification

### Check GPU Support
```bash
# Linux: Check Vulkan
vulkaninfo | grep "deviceName"

# macOS: Check Metal
system_profiler SPDisplaysDataType

# Windows: Use dxdiag or GPU-Z
```

### Check Audio
```bash
# Linux: Test audio devices
aplay -l

# macOS: Check audio
system_profiler SPAudioDataType  

# Windows: Check audio devices in Device Manager
```

## Development Environment Setup

### IDE Recommendations

#### VS Code (Recommended)
Install these extensions:
- `rust-analyzer` - Rust language support
- `CodeLLDB` - Debugging support
- `Even Better TOML` - TOML file support
- `Error Lens` - Inline error display

#### Other IDEs
- **CLion**: Has good Rust support with the Rust plugin
- **Vim/Neovim**: Use with rust-analyzer LSP
- **Emacs**: Use with rust-analyzer and rustic-mode

### Performance Considerations

#### Release Builds
For better performance during development:
```bash
# Always use release mode for examples
cargo run -p hello_companion --release

# Build in release mode
cargo build --release
```

#### Parallel Compilation
Speed up builds by using multiple CPU cores:
```bash
# Set in ~/.cargo/config.toml
[build]
jobs = 4  # or number of CPU cores
```

#### Target Directory
Use a shared target directory to reduce disk usage:
```bash
# Set CARGO_TARGET_DIR environment variable
export CARGO_TARGET_DIR=/path/to/shared/target
```

## Troubleshooting

### Common Build Errors

#### "linker not found"
- **Linux**: Install `build-essential` or `gcc`
- **macOS**: Install Xcode Command Line Tools
- **Windows**: Install Visual Studio with C++ tools

#### Vulkan errors
- **Linux**: Install `mesa-vulkan-drivers` and `vulkan-tools`
- **macOS**: Ensure MoltenVK is installed
- **Windows**: Update graphics drivers

#### Audio errors
- **Linux**: Install `libasound2-dev` and `libpulse-dev`
- **macOS**: Usually works out of the box
- **Windows**: Ensure Windows Audio service is running

### Performance Issues

#### Slow Compilation
- Use `cargo build --release` for better runtime performance
- Consider using `sccache` to cache compilation results
- Increase parallel build jobs in Cargo config

#### Runtime Performance
- Always use `--release` flag for examples and demos
- Ensure GPU drivers are up to date
- Check system has adequate RAM (4GB minimum)

### Platform-Specific Issues

#### Linux Wayland vs X11
AstraWeave supports both Wayland and X11:
```bash
# Force X11 if needed
export WAYLAND_DISPLAY=""

# Force Wayland if needed  
export DISPLAY=""
```

#### macOS Code Signing
For distribution on macOS, you may need to sign binaries:
```bash
codesign --force --deep --sign - target/release/hello_companion
```

#### Windows Antivirus
Some antivirus software may flag Rust binaries. Add exclusions for:
- The project directory
- `~/.cargo` directory
- `target/` build directory

## Next Steps

With AstraWeave installed:
1. Run through the [Quick Start Guide](./quick-start.md)
2. Explore [Working Examples](../examples/index.md)
3. Read about [Architecture](../architecture/overview.md)
4. Build [Your First Game](../game-dev/first-game.md)

For ongoing development, see the [Contributing Guide](../dev/contributing.md) and [Building from Source](../dev/building.md).