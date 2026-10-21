# CLI Tools

AstraWeave provides command-line tools for development, asset processing, debugging, and deployment.

## Overview

| Tool | Purpose |
|------|---------|
| `aw_build` | Build and package games |
| `aw_asset_cli` | Asset processing and optimization |
| `aw_debug` | Runtime debugging and inspection |
| `aw_editor` | Visual editor (GUI) |
| `aw_headless` | Headless server for testing |
| `aw_save_cli` | Save file management |

## aw_build

Build and package games for distribution.

### Installation

```bash
cargo install --path tools/aw_build
```

### Usage

```bash
# Build for current platform
aw_build

# Build release version
aw_build --release

# Build for specific platform
aw_build --target windows
aw_build --target linux
aw_build --target macos
aw_build --target web

# Full package with assets
aw_build --release --package

# Custom output directory
aw_build --release --output dist/
```

### Configuration

Create `aw_build.toml` in your project root:

```toml
[package]
name = "my-game"
version = "1.0.0"
author = "Developer Name"
description = "An awesome game"

[build]
features = ["release-optimizations"]
target_dir = "target"

[assets]
source = "assets"
compress = true
strip_unused = true

[platforms.windows]
icon = "assets/icons/game.ico"
installer = true

[platforms.linux]
appimage = true

[platforms.macos]
bundle = true
icon = "assets/icons/game.icns"

[platforms.web]
wasm_opt = true
```

### Build Profiles

```bash
# Debug build (fast compile, slow runtime)
aw_build --profile debug

# Release build (slow compile, optimized)
aw_build --profile release

# Distribution build (maximum optimization)
aw_build --profile dist
```

## aw_asset_cli

Process and optimize game assets.

### Installation

```bash
cargo install --path tools/aw_asset_cli
```

### Commands

#### Import Assets

```bash
# Import a single asset
aw_asset_cli import models/character.fbx

# Import with options
aw_asset_cli import models/character.fbx --format gltf --optimize

# Batch import directory
aw_asset_cli import-dir raw_assets/ --recursive
```

#### Optimize Assets

```bash
# Optimize textures
aw_asset_cli optimize textures/ --compress --generate-mipmaps

# Optimize meshes
aw_asset_cli optimize-mesh models/ --simplify 0.5

# Full optimization pass
aw_asset_cli optimize-all assets/
```

#### Pack Assets

```bash
# Create asset bundle
aw_asset_cli pack assets/ --output game.pak

# Pack with compression
aw_asset_cli pack assets/ --output game.pak --compression zstd

# Pack specific types
aw_asset_cli pack assets/textures/ --output textures.pak --type texture
```

#### Validate Assets

```bash
# Check asset integrity
aw_asset_cli validate assets/

# Check specific asset
aw_asset_cli validate models/character.gltf --verbose

# Validate bundle
aw_asset_cli validate-bundle game.pak
```

### Texture Processing

```bash
# Convert to KTX2 format
aw_asset_cli texture convert input.png --format ktx2

# Generate mipmaps
aw_asset_cli texture mipmaps input.png

# Compress textures
aw_asset_cli texture compress textures/ --format bc7

# Create texture atlas
aw_asset_cli texture atlas sprites/*.png --output atlas.png --json atlas.json
```

### Mesh Processing

```bash
# Simplify mesh
aw_asset_cli mesh simplify model.gltf --ratio 0.5 --output model_lod1.gltf

# Generate LODs
aw_asset_cli mesh lod model.gltf --levels 3

# Optimize vertex cache
aw_asset_cli mesh optimize model.gltf --vertex-cache --output optimized.gltf
```

## aw_debug

Runtime debugging and inspection tools.

### Installation

```bash
cargo install --path tools/aw_debug
```

### Attach to Running Game

```bash
# Connect to local game
aw_debug attach

# Connect to specific port
aw_debug attach --port 9999

# Connect to remote game
aw_debug attach --host 192.168.1.100 --port 9999
```

### Commands (Interactive Mode)

```
> help                    # Show available commands
> entities                # List all entities
> entity 42               # Inspect entity 42
> components Player       # Find entities with Player component
> resource GameState      # Inspect resource
> systems                 # List running systems
> perf                    # Show performance stats
> mem                     # Show memory usage
> spawn Enemy 10.0 0.0 5.0  # Spawn entity at position
> despawn 42              # Despawn entity
> pause                   # Pause game
> resume                  # Resume game
> step                    # Step one frame
> quit                    # Disconnect
```

### Scripted Commands

```bash
# Run commands from file
aw_debug attach --script debug_commands.txt

# Run single command
aw_debug attach --exec "entities | grep Player"

# Dump state to file
aw_debug attach --exec "dump world.json"
```

### Profiling

```bash
# Start profiling
aw_debug attach --profile

# Capture Tracy trace
aw_debug attach --tracy --output trace.tracy

# Memory profiling
aw_debug attach --memory-profile --output memory.json
```

## aw_editor

Visual editor for scene creation and game development.

### Installation

```bash
cargo install --path tools/aw_editor
```

### Launch

```bash
# Open editor
aw_editor

# Open specific project
aw_editor --project path/to/project

# Open specific scene
aw_editor --project myproject --scene levels/main.scene
```

### Features

- Scene hierarchy view
- Component inspector
- Asset browser
- Behavior tree editor
- Material editor
- Play-in-editor

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+S | Save scene |
| Ctrl+Shift+S | Save all |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z | Redo |
| F5 | Play |
| Shift+F5 | Stop |
| F | Focus selected |
| Delete | Delete selected |

## aw_headless

Run games without rendering for testing and CI.

### Installation

```bash
cargo install --path tools/aw_headless
```

### Usage

```bash
# Run game headlessly
aw_headless --project path/to/game

# Run for specific duration
aw_headless --project game --duration 60

# Run specific test scene
aw_headless --project game --scene test_level.scene

# Run with custom config
aw_headless --project game --config test_config.toml
```

### Testing

```bash
# Run integration tests
aw_headless --project game --test integration

# Run performance benchmark
aw_headless --project game --benchmark --output results.json

# Run AI behavior tests
aw_headless --project game --test ai-behaviors --verbose
```

### CI Integration

```yaml
# GitHub Actions example
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: cargo build --release
      - name: Headless Tests
        run: aw_headless --project . --test all --timeout 300
```

## aw_save_cli

Manage game save files.

### Installation

```bash
cargo install --path tools/aw_save_cli
```

### Commands

```bash
# List saves
aw_save_cli list

# Inspect save
aw_save_cli inspect save_001.dat

# Validate save
aw_save_cli validate save_001.dat

# Export to JSON
aw_save_cli export save_001.dat --format json --output save.json

# Import from JSON
aw_save_cli import save.json --output save_new.dat

# Migrate save to new version
aw_save_cli migrate save_001.dat --to-version 2
```

### Save File Locations

| Platform | Default Location |
|----------|-----------------|
| Windows | `%APPDATA%/GameName/saves/` |
| Linux | `~/.local/share/GameName/saves/` |
| macOS | `~/Library/Application Support/GameName/saves/` |

## Common Workflows

### Development Build

```bash
# Quick iteration
cargo build
cargo run

# With asset hot-reload
ASTRAWEAVE_HOT_RELOAD=1 cargo run
```

### Release Build

```bash
# Build optimized release
aw_build --release

# Process assets
aw_asset_cli optimize-all assets/

# Package for distribution
aw_build --release --package
```

### CI/CD Pipeline

```bash
# Install tools
cargo install --path tools/aw_build
cargo install --path tools/aw_asset_cli
cargo install --path tools/aw_headless

# Build
aw_build --release

# Test
aw_headless --test all

# Package
aw_build --release --package --output artifacts/
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `ASTRAWEAVE_LOG` | Log level (trace, debug, info, warn, error) |
| `ASTRAWEAVE_ASSETS` | Asset directory path |
| `ASTRAWEAVE_CONFIG` | Config file path |
| `ASTRAWEAVE_HOT_RELOAD` | Enable hot-reload (1/0) |
| `ASTRAWEAVE_DEBUG_PORT` | Debug server port |

## Related Documentation

- [Building](../dev/building.md) - Build instructions
- [Configuration](configuration.md) - Configuration options
- [Testing](../dev/testing.md) - Testing guide
- [Crates Reference](crates.md) - API documentation
