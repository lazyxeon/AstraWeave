# Troubleshooting Examples

This guide helps resolve common issues when running AstraWeave examples.

## Working Examples

The following examples are verified to build and run:

| Example | Command | Notes |
|---------|---------|-------|
| hello_companion | `cargo run -p hello_companion --release` | Expected panic (demonstrates tool validation) |
| fluids_demo | `cargo run -p fluids_demo --release` | Interactive PCISPH fluid simulation |
| unified_showcase | `cargo run -p unified_showcase --release` | wgpu rendering with shadows, terrain |
| physics_demo3d | `cargo run -p physics_demo3d --release` | Rapier3D physics with character controller |
| navmesh_demo | `cargo run -p navmesh_demo --release` | NavMesh baking and A* pathfinding |
| audio_spatial_demo | `cargo run -p audio_spatial_demo --release` | 3D spatial audio |
| adaptive_boss | `cargo run -p adaptive_boss --release` | Director AI for boss encounters |
| ipc_loopback | `cargo run -p ipc_loopback --release` | IPC communication test |

## Known Compilation Issues

Some examples have compilation issues due to API evolution:

| Example | Issue | Status |
|---------|-------|--------|
| debug_overlay | egui 0.32 API changes | Needs update |
| ui_controls_demo | egui/winit version mismatches | Needs update |
| visual_3d | winit 0.30 migration | Needs update |
| rhai_authoring | Rhai sync/send traits | Needs refactor |
| npc_town_demo | Multiple API mismatches | Needs update |

## Build Aliases

Use these workspace aliases for safer builds:

```bash
cargo build -p astraweave-core -p astraweave-ecs -p astraweave-math -p astraweave-ai
cargo check --workspace
cargo clippy --workspace --all-features -- -D warnings
```

## Common Errors

### "No suitable graphics adapter"

```
thread 'main' panicked at 'No suitable graphics adapter found'
```

**Solution**: Update graphics drivers or use software rendering:
```bash
export WGPU_BACKEND=gl
cargo run -p hello_companion
```

### "AI model not found"

```
Error: Could not load AI model
```

**Solution**: Examples use mock AI by default. This panic is expected when no LLM is configured.

### Vulkan Not Found (Linux)

```bash
sudo apt-get install mesa-vulkan-drivers vulkan-tools
```

### Missing Build Tools (Windows)

Install Visual Studio with C++ build tools.

## Getting Help

- [Full Troubleshooting Guide](../resources/troubleshooting.md)
- [FAQ](../resources/faq.md)
- [Community](../resources/community.md)
