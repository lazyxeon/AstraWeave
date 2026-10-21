# Platform Support

AstraWeave targets multiple platforms with varying levels of support and feature availability.

## Support Tiers

| Tier | Definition | Guarantees |
|------|------------|------------|
| **Tier 1** | Fully supported, tested in CI | Full feature support, regular testing, priority bug fixes |
| **Tier 2** | Supported, community tested | Most features work, issues addressed on best-effort basis |
| **Tier 3** | Experimental | May work, no guarantees, community-maintained |

## Platform Matrix

### Desktop Platforms

| Platform | Tier | Rendering | AI/LLM | Audio | Networking |
|----------|------|-----------|--------|-------|------------|
| Windows 10/11 (x64) | Tier 1 | Vulkan, DX12 | Full | Full | Full |
| Linux (x64) | Tier 1 | Vulkan | Full | Full | Full |
| macOS (x64) | Tier 2 | Metal | Full | Full | Full |
| macOS (Apple Silicon) | Tier 2 | Metal | Full | Full | Full |

### Web Platforms

| Platform | Tier | Rendering | AI/LLM | Audio | Networking |
|----------|------|-----------|--------|-------|------------|
| WebAssembly | Tier 2 | WebGPU, WebGL2 | Remote only | Web Audio | WebSocket |

### Console Platforms

| Platform | Tier | Notes |
|----------|------|-------|
| PlayStation 5 | Planned | Post-1.0 |
| Xbox Series X/S | Planned | Post-1.0 |
| Nintendo Switch | Planned | Post-1.0 |

### Mobile Platforms

| Platform | Tier | Notes |
|----------|------|-------|
| Android | Tier 3 | Experimental |
| iOS | Tier 3 | Experimental |

## System Requirements

### Minimum Requirements

| Component | Windows | Linux | macOS |
|-----------|---------|-------|-------|
| OS | Windows 10 (1909+) | Ubuntu 20.04+ / Fedora 34+ | macOS 11+ |
| CPU | x64, 4 cores | x64, 4 cores | x64 / Apple M1+ |
| RAM | 8 GB | 8 GB | 8 GB |
| GPU | Vulkan 1.2 / DX12 | Vulkan 1.2 | Metal 2 |
| Storage | 2 GB | 2 GB | 2 GB |

### Recommended Requirements

| Component | Specification |
|-----------|---------------|
| CPU | 8+ cores, modern (2020+) |
| RAM | 16 GB+ |
| GPU | RTX 2060 / RX 5700 / M1 Pro+ |
| Storage | SSD, 10 GB+ |

### For AI/LLM Features

| Feature | Additional Requirements |
|---------|------------------------|
| Local LLM (Ollama) | 16 GB+ RAM, fast SSD |
| GPU-accelerated LLM | NVIDIA GPU with 8 GB+ VRAM, CUDA |
| Large context windows | 32 GB+ RAM |

## Graphics API Support

### Vulkan (Windows, Linux)

Primary rendering backend for Windows and Linux.

```toml
[dependencies]
astraweave-render = { version = "0.1", features = ["vulkan"] }
```

**Requirements**:
- Vulkan 1.2+
- Up-to-date GPU drivers

**Tested GPUs**:
- NVIDIA: GTX 1000+, RTX series
- AMD: RX 400+, RX 5000+, RX 6000+, RX 7000+
- Intel: UHD 600+, Arc series

### DirectX 12 (Windows)

Alternative backend for Windows.

```toml
[dependencies]
astraweave-render = { version = "0.1", features = ["dx12"] }
```

**Requirements**:
- Windows 10 1909+
- DirectX 12 capable GPU

### Metal (macOS)

Primary backend for macOS.

```toml
[dependencies]
astraweave-render = { version = "0.1", features = ["metal"] }
```

**Requirements**:
- macOS 11 (Big Sur)+
- Metal 2 capable GPU

### WebGPU (Web)

Modern web rendering API.

```toml
[dependencies]
astraweave-render = { version = "0.1", features = ["webgpu"] }
```

**Requirements**:
- Chrome 113+ / Firefox 115+ / Safari 17+
- WebGPU enabled

### WebGL2 (Web Fallback)

Fallback for browsers without WebGPU.

```toml
[dependencies]
astraweave-render = { version = "0.1", features = ["webgl2"] }
```

**Limitations**:
- Reduced shader capabilities
- Lower texture limits
- No compute shaders

## Feature Availability by Platform

### Core Features

| Feature | Windows | Linux | macOS | Web |
|---------|---------|-------|-------|-----|
| ECS | Yes | Yes | Yes | Yes |
| Physics | Yes | Yes | Yes | Yes |
| Audio | Yes | Yes | Yes | Yes |
| Input | Yes | Yes | Yes | Partial |
| Networking | Yes | Yes | Yes | WebSocket |

### Rendering Features

| Feature | Windows | Linux | macOS | Web |
|---------|---------|-------|-------|-----|
| PBR Materials | Yes | Yes | Yes | Yes |
| Shadow Mapping | Yes | Yes | Yes | Limited |
| Post-Processing | Yes | Yes | Yes | Limited |
| Skeletal Animation | Yes | Yes | Yes | Yes |
| Compute Shaders | Yes | Yes | Yes | WebGPU only |

### AI Features

| Feature | Windows | Linux | macOS | Web |
|---------|---------|-------|-------|-----|
| Behavior Trees | Yes | Yes | Yes | Yes |
| GOAP | Yes | Yes | Yes | Yes |
| Local LLM (Ollama) | Yes | Yes | Yes | No |
| Remote LLM | Yes | Yes | Yes | Yes |
| GPU LLM (CUDA) | Yes | Limited | No | No |

## Building for Platforms

### Windows

```bash
# Native build
cargo build --release

# Cross-compile from Linux
cargo build --release --target x86_64-pc-windows-gnu
```

### Linux

```bash
# Native build
cargo build --release

# With specific features
cargo build --release --features "vulkan,llm"
```

### macOS

```bash
# Native build
cargo build --release

# Universal binary (x64 + ARM)
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
lipo -create -output libgame.dylib \
  target/x86_64-apple-darwin/release/libgame.dylib \
  target/aarch64-apple-darwin/release/libgame.dylib
```

### WebAssembly

```bash
# Install wasm-pack
cargo install wasm-pack

# Build for web
wasm-pack build --target web --release

# Optimize WASM
wasm-opt -O3 -o game_opt.wasm game.wasm
```

## Platform-Specific Configuration

### Windows Configuration

```toml
# astraweave.toml
[platform.windows]
renderer = "dx12"  # or "vulkan"
vsync = true
fullscreen_mode = "borderless"

[platform.windows.audio]
backend = "wasapi"
```

### Linux Configuration

```toml
[platform.linux]
renderer = "vulkan"
vsync = true
wayland = true  # Enable Wayland support

[platform.linux.audio]
backend = "pulseaudio"  # or "alsa", "pipewire"
```

### macOS Configuration

```toml
[platform.macos]
renderer = "metal"
retina = true
native_fullscreen = true

[platform.macos.audio]
backend = "coreaudio"
```

### Web Configuration

```toml
[platform.web]
renderer = "webgpu"  # or "webgl2"
canvas_id = "game-canvas"

[platform.web.audio]
backend = "webaudio"
```

## Known Limitations

### Windows

- DX12 may require Windows Update for latest features
- Some anti-virus software may flag debug builds

### Linux

- Wayland compositor compatibility varies
- Audio backend may need manual selection

### macOS

- Notarization required for distribution
- Some features limited without Rosetta on Apple Silicon

### Web

- No local file system access
- LLM requires remote server
- Thread support limited (SharedArrayBuffer)
- Binary size considerations

## Troubleshooting Platform Issues

### Windows: DX12 Errors

```
Error: Failed to create DX12 device
```

**Solution**: Update GPU drivers and Windows.

### Linux: Vulkan Not Found

```
Error: Failed to load Vulkan library
```

**Solution**:
```bash
# Ubuntu/Debian
sudo apt install vulkan-tools mesa-vulkan-drivers

# Fedora
sudo dnf install vulkan-loader vulkan-tools
```

### macOS: Metal Errors

```
Error: Metal device not found
```

**Solution**: Ensure running on macOS 11+ with Metal-capable GPU.

### Web: WebGPU Not Supported

```
Error: WebGPU not available
```

**Solution**: Enable WebGPU in browser settings or use WebGL2 fallback:
```toml
[platform.web]
renderer = "webgl2"
```

## Platform Testing

### CI/CD Matrix

```yaml
# GitHub Actions example
strategy:
  matrix:
    os: [ubuntu-latest, windows-latest, macos-latest]
    include:
      - os: ubuntu-latest
        target: x86_64-unknown-linux-gnu
      - os: windows-latest
        target: x86_64-pc-windows-msvc
      - os: macos-latest
        target: x86_64-apple-darwin
```

### Platform-Specific Tests

```rust
#[cfg(target_os = "windows")]
#[test]
fn test_windows_specific() {
    // Windows-only test
}

#[cfg(target_os = "linux")]
#[test]
fn test_linux_specific() {
    // Linux-only test
}

#[cfg(target_arch = "wasm32")]
#[test]
fn test_wasm_specific() {
    // WebAssembly-only test
}
```

## Related Documentation

- [Configuration](configuration.md) - Platform configuration options
- [Building](../dev/building.md) - Build instructions
- [Troubleshooting](../resources/troubleshooting.md) - General troubleshooting
- [CLI Tools](cli-tools.md) - Command-line tools
