# Feature Flags Reference

**AstraWeave Game Engine Cargo Features**

This document lists all Cargo feature flags across the workspace.

---

## Core Engine Features

### `nanite`
- **Description**: Virtualized geometry system with GPU-driven rendering
- **Default**: Disabled
- **Impact**: +15% GPU usage, -40% draw calls for complex meshes, LOD streaming
- **Enable**: `cargo build --features nanite`
- **Crates**: astraweave-render, astraweave-asset

### `llm_orchestrator`
- **Description**: LLM-based AI orchestration and planning
- **Default**: Disabled
- **Impact**: +50MB binary size, requires LLM API access
- **Enable**: `cargo build --features llm_orchestrator`
- **Crates**: astraweave-ai, astraweave-llm

---

## Rendering Features

### `postfx`
- **Description**: Post-processing effects (bloom, tonemapping)
- **Default**: Enabled
- **Crates**: astraweave-render

### `megalights`
- **Description**: GPU light culling for 100k+ dynamic lights
- **Default**: Disabled
- **Impact**: +5ms frame time for 10k lights
- **Crates**: astraweave-render

### `ibl`
- **Description**: Image-based lighting with environment maps
- **Default**: Disabled
- **Crates**: astraweave-render

### `msaa`
- **Description**: Multi-sample anti-aliasing
- **Default**: Disabled
- **Impact**: +40% GPU memory for 4x MSAA
- **Options**: 2x, 4x, 8x
- **Crates**: astraweave-render

### `deferred`
- **Description**: Deferred rendering pipeline (G-buffer)
- **Default**: Disabled (uses forward rendering)
- **Crates**: astraweave-render

### `advanced-post`
- **Description**: TAA, motion blur, depth-of-field
- **Default**: Disabled
- **Impact**: +10ms frame time
- **Crates**: astraweave-render

---

## Development & Testing

### `gpu-tests`
- **Description**: Enable GPU-dependent integration tests
- **Default**: Disabled (CI uses headless)
- **Enable**: `cargo test --features gpu-tests`
- **Crates**: astraweave-render (tests)

### `profiling`
- **Description**: Tracy profiler integration
- **Default**: Disabled
- **Impact**: +10% CPU overhead
- **Enable**: `cargo build --features profiling`
- **Requires**: Tracy viewer application
- **Crates**: astraweave-profiling

---

## Platform & Build

### `professional-compression`
- **Description**: Use libktx-rs for high-quality texture compression
- **Default**: Disabled (uses placeholder encoder)
- **Impact**: Requires native libKTX build
- **Crates**: tools/aw_asset_cli

---

## Feature Matrix

| Feature | Default | Dev | CI | Staging | Prod | Binary Size |
|---------|---------|-----|----|---------| -----|-------------|
| `postfx` | ✓ | ✓ | ✓ | ✓ | ✓ | +2MB |
| `nanite` | ✗ | ✓ | ✗ | ✓ | ✓ | +5MB |
| `megalights` | ✗ | ✓ | ✗ | ✓ | ✓ | +1MB |
| `llm_orchestrator` | ✗ | ✓ | ✗ | ✓ | ✓ | +50MB |
| `msaa` | ✗ | ✓ | ✗ | ✓ | ✓ | +500KB |
| `profiling` | ✗ | ✓ | ✗ | ✗ | ✗ | +3MB |
| `gpu-tests` | ✗ | ✗ | ✗ | ✗ | ✗ | N/A |

---

## Common Feature Combinations

### Minimal Build (fast iteration)
```bash
cargo build --no-default-features
```
**Use case**: Fastest compile times, core functionality only

### Default Build (balanced)
```bash
cargo build
```
**Includes**: postfx
**Use case**: Standard development

### Full Build (all features)
```bash
cargo build --features "nanite,megalights,ibl,msaa,advanced-post,llm_orchestrator"
```
**Use case**: Testing, showcase demos

### Production Build
```bash
cargo build --release --features "nanite,megalights,ibl,msaa"
```
**Excludes**: profiling, gpu-tests

---

## Usage Examples

### Enable specific features
```bash
cargo build --features "nanite,megalights"
```

### Disable default features
```bash
cargo build --no-default-features
```

### Run tests with GPU
```bash
cargo test --features gpu-tests
```

### Profile with Tracy
```bash
cargo build --features profiling
cargo run --features profiling --example profiling_demo
# Open Tracy viewer to connect
```

---

**Last Updated:** 2025-11-13  
**Version:** 0.4.0
