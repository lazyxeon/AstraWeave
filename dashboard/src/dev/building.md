# Building from Source

This guide covers how to build AstraWeave from source on all supported platforms.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Platform-Specific Instructions](#platform-specific-instructions)
- [Build Configurations](#build-configurations)
- [Troubleshooting](#troubleshooting)
- [Next Steps](#next-steps)

## Prerequisites

### Required Software

#### Rust Toolchain

AstraWeave requires Rust 1.75.0 or later.

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Or on Windows, download from: https://rustup.rs/

# Verify installation
rustc --version
cargo --version
```

```admonish tip
Keep Rust updated for the latest performance improvements and bug fixes:
\`\`\`bash
rustup update
\`\`\`
```

#### Git

```bash
# Linux (Debian/Ubuntu)
sudo apt install git

# macOS
brew install git

# Windows
# Download from https://git-scm.com/
```

### GPU Requirements

AstraWeave requires a modern GPU with support for:

- **Vulkan 1.2+** (Linux, Windows)
- **DirectX 12** (Windows)
- **Metal** (macOS)

Supported GPUs:
- NVIDIA: GTX 1060 or newer
- AMD: RX 580 or newer
- Intel: Arc series or newer integrated graphics

```admonish warning
Older GPUs may work but are not officially supported and may have reduced performance.
```

## Platform-Specific Instructions

### Windows

#### System Dependencies

1. **Visual Studio Build Tools** (required for linking):
   - Download [Visual Studio 2022 Build Tools](https://visualstudio.microsoft.com/downloads/)
   - Select "Desktop development with C++" workload
   - Minimum components:
     - MSVC v143 build tools
     - Windows 10/11 SDK

2. **Vulkan SDK** (optional, for Vulkan backend):
   - Download from [LunarG](https://vulkan.lunarg.com/)
   - Install with default options

#### Building on Windows

```powershell
# Clone the repository
git clone https://github.com/verdentlabs/astraweave.git
cd astraweave

# Build (release mode recommended)
cargo build --release

# Run tests
cargo test --release

# Run an example
cargo run --release --example basic_companion
```

```admonish note
Windows Defender may flag the first build as suspicious. This is normal for newly compiled executables.
```

#### Windows-Specific Issues

**Long Path Support**: Enable long paths if you encounter path length errors:

```powershell
# Run as Administrator
New-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" `
  -Name "LongPathsEnabled" -Value 1 -PropertyType DWORD -Force
```

### Linux

#### System Dependencies

**Debian/Ubuntu**:

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  pkg-config \
  libx11-dev \
  libxcursor-dev \
  libxrandr-dev \
  libxi-dev \
  libasound2-dev \
  libudev-dev \
  vulkan-tools \
  libvulkan-dev
```

**Fedora/RHEL**:

```bash
sudo dnf install -y \
  gcc gcc-c++ \
  pkg-config \
  libX11-devel \
  libXcursor-devel \
  libXrandr-devel \
  libXi-devel \
  alsa-lib-devel \
  systemd-devel \
  vulkan-tools \
  vulkan-loader-devel
```

**Arch Linux**:

```bash
sudo pacman -S --needed \
  base-devel \
  libx11 \
  libxcursor \
  libxrandr \
  libxi \
  alsa-lib \
  vulkan-tools \
  vulkan-headers
```

#### Building on Linux

```bash
# Clone the repository
git clone https://github.com/verdentlabs/astraweave.git
cd astraweave

# Build
cargo build --release

# Run tests
cargo test --release

# Run an example
cargo run --release --example basic_companion
```

#### GPU Drivers

**NVIDIA**:
```bash
# Ubuntu
sudo ubuntu-drivers autoinstall

# Or manually install latest driver
sudo apt install nvidia-driver-535
```

**AMD**:
```bash
# Mesa drivers (usually pre-installed)
sudo apt install mesa-vulkan-drivers
```

**Intel**:
```bash
sudo apt install mesa-vulkan-drivers intel-media-va-driver
```

Verify Vulkan support:
```bash
vulkaninfo | grep "deviceName"
```

### macOS

#### System Dependencies

1. **Xcode Command Line Tools**:
   ```bash
   xcode-select --install
   ```

2. **Homebrew** (recommended):
   ```bash
   /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
   ```

#### Building on macOS

```bash
# Clone the repository
git clone https://github.com/verdentlabs/astraweave.git
cd astraweave

# Build
cargo build --release

# Run tests
cargo test --release

# Run an example
cargo run --release --example basic_companion
```

```admonish warning
macOS uses Metal as the graphics backend. Ensure your system is macOS 10.15+ with Metal-compatible GPU.
```

#### Apple Silicon (M1/M2/M3)

AstraWeave fully supports Apple Silicon:

```bash
# Verify you're using the ARM toolchain
rustc --version --verbose | grep host
# Should show: aarch64-apple-darwin

# Build optimized for Apple Silicon
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## Build Configurations

### Release vs Debug

**Debug build** (fast compilation, slow runtime):
```bash
cargo build
# Binary: target/debug/
```

**Release build** (slow compilation, fast runtime):
```bash
cargo build --release
# Binary: target/release/
```

```admonish tip
Always use release builds for performance testing and gameplay. Debug builds can be 10-100x slower.
```

### Feature Flags

AstraWeave uses Cargo features for optional functionality:

```bash
# Build with all features
cargo build --all-features

# Build with specific features
cargo build --features "ai,physics,fluids"

# Build without default features
cargo build --no-default-features --features "minimal"
```

Available features:
- `ai` - AI companion systems (default)
- `physics` - Physics simulation
- `fluids` - Fluid dynamics
- `editor` - Level editor tools
- `networking` - Multiplayer support
- `serialization` - Save/load support

### Optimization Profiles

Custom profiles are defined in `Cargo.toml`:

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1

[profile.dev]
opt-level = 1

[profile.bench]
inherits = "release"
lto = "fat"
```

Build with specific profile:
```bash
cargo build --profile bench
```

### Cross-Compilation

#### Targeting Different Platforms

```bash
# Install target
rustup target add x86_64-pc-windows-gnu

# Build for target
cargo build --target x86_64-pc-windows-gnu --release
```

Common targets:
- `x86_64-pc-windows-msvc` - Windows (MSVC)
- `x86_64-unknown-linux-gnu` - Linux
- `x86_64-apple-darwin` - macOS (Intel)
- `aarch64-apple-darwin` - macOS (Apple Silicon)

## Troubleshooting

### Common Build Errors

#### "linker not found"

**Windows**: Install Visual Studio Build Tools

**Linux**:
```bash
sudo apt install build-essential
```

**macOS**:
```bash
xcode-select --install
```

#### "could not find native library 'vulkan'"

**Windows**: Install Vulkan SDK

**Linux**:
```bash
sudo apt install libvulkan-dev
```

**macOS**: Not needed (uses Metal)

#### Out of Memory During Build

Reduce parallel compilation:
```bash
cargo build --release -j 2
```

Or increase swap space on Linux:
```bash
sudo fallocate -l 8G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

#### "failed to run custom build command"

Clean the build cache:
```bash
cargo clean
cargo build --release
```

### Performance Issues

#### Slow Compilation

Use the faster linker:

**Linux** (lld):
```bash
sudo apt install lld
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"
cargo build --release
```

**macOS** (zld):
```bash
brew install michaeleisel/zld/zld
export RUSTFLAGS="-C link-arg=-fuse-ld=/usr/local/bin/zld"
cargo build --release
```

**Windows** (lld-link, included with MSVC):
```powershell
$env:RUSTFLAGS="-C link-arg=/DEBUG:NONE"
cargo build --release
```

Enable sccache for cached builds:
```bash
cargo install sccache
export RUSTC_WRAPPER=sccache
```

#### Runtime Performance

Verify release mode:
```bash
# Should show /release/ in path
cargo run --release -- --version
```

Check GPU usage:
```bash
# Windows
nvidia-smi

# Linux
watch -n 1 nvidia-smi
```

### Debugging Build Issues

Enable verbose output:
```bash
cargo build --release --verbose
```

Check dependencies:
```bash
cargo tree
```

Update dependencies:
```bash
cargo update
```

## Next Steps

After successfully building AstraWeave:

1. **Run the test suite**: See [Testing Guide](testing.md)
2. **Try the examples**: `cargo run --release --example <name>`
3. **Read the tutorials**: [Getting Started](../getting-started/index.md)
4. **Join the community**: [Discord](https://discord.gg/astraweave)

```admonish success
Congratulations! You've successfully built AstraWeave from source. You're ready to start developing AI-native games.
```
