---
layout: default
title: Development Setup
---

# Development Setup

## Prerequisites

| Requirement | Version | Purpose |
|-------------|---------|---------|
| **Rust** | 1.89.0+ (pinned via `rust-toolchain.toml`) | Compiler and cargo |
| **Git** | Any | Version control |
| **Ollama** | Latest (optional) | LLM features |
| **Tracy** | 0.11+ (optional) | Performance profiling |

## Initial Setup

```bash
# Clone the repository
git clone https://github.com/lazyxeon/AstraWeave.git
cd AstraWeave

# First build (15-45 min — wgpu + graphics dependencies)
cargo build

# Verify with workspace check
cargo check --workspace
```

## Running Tests

```bash
# All working crates
cargo test --workspace

# Specific crate (library tests)
cargo test -p astraweave-ecs --lib
cargo test -p astraweave-physics --lib
cargo test -p astraweave-fluids --lib

# With async-physics feature
cargo test -p astraweave-physics --features async-physics --lib

# Integration tests
cargo test -p astraweave-scene --tests
cargo test -p astraweave-prompts --tests
```

## Running Benchmarks

```bash
cargo bench -p astraweave-core
cargo bench -p astraweave-physics
cargo bench -p astraweave-ai
cargo bench -p astraweave-ecs
cargo bench -p astraweave-math
cargo bench -p astraweave-sdk
```

## Running Examples

```bash
# AI companion demo (6 operational modes)
cargo run -p hello_companion --release

# Unified showcase (all systems integrated)
cargo run -p unified_showcase --release

# Individual demos
cargo run -p physics_demo3d --release
cargo run -p terrain_demo --release
cargo run -p fluids_demo --release
cargo run -p weaving_playground --release
```

59 example packages are available. See [Crate Index](crates.html) for the full list.

## LLM Setup (Optional)

To use LLM-powered AI features:

1. Install [Ollama](https://ollama.ai)
2. Pull a model. The runtime default is `phi3:medium`; Qwen3-8B is opt-in via `OLLAMA_MODEL`:
   ```bash
   ollama pull phi3:medium      # runtime default
   ollama pull qwen3:8b         # opt-in: set OLLAMA_MODEL=qwen3:8b
   ```
3. Run with LLM features:
   ```bash
   cargo run -p hello_companion --release --features llm_orchestrator
   ```

## Makefile Targets

The project includes a Makefile with convenience targets:

```bash
make build-core       # Build core crates only
make check-all        # Workspace check
make test-all         # Test all working crates
make clippy-all       # Full linting
make gfx-check        # Renderer + demo compile check
make quickstart       # Setup, build, run example
make dev              # format + lint + test
make ci               # Clean build + test + lint + audit
```

## Development Workflow

1. Make changes in one crate at a time
2. **Quick check**: `cargo check -p <crate>` (mandatory after every change)
3. **Fix errors**: All compilation errors must be resolved immediately
4. **Test**: `cargo test -p <crate> --lib`
5. **Format**: `cargo fmt --all`
6. **Lint**: `cargo clippy -p <crate> --all-features -- -D warnings`
7. **Integration**: Run `hello_companion` or `unified_showcase`

## Formal Verification

```bash
# Miri (undefined behavior detection — 977 tests, 0 UB)
cargo +nightly miri test -p astraweave-ecs --lib -- --test-threads=1
cargo +nightly miri test -p astraweave-core --lib -- --test-threads=1
cargo +nightly miri test -p astraweave-math --lib -- --test-threads=1
cargo +nightly miri test -p astraweave-sdk --lib -- --test-threads=1

# Kani (formal proofs)
cargo kani --package astraweave-sdk
cargo kani --package astraweave-ecs
cargo kani --package astraweave-math
```

## Platform Notes

### Windows

The workspace configures a 16 MB stack size for MSVC and GNU targets (in `.cargo/config.toml`) to prevent stack overflow from large `State` structs in release builds:

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "link-args=/STACK:16777216"]
```

## Project Structure

```
AstraWeave/
├── astraweave-ai/          # AI orchestration (GOAP + BT + LLM)
├── astraweave-ai-gen/      # AI generation utilities
├── astraweave-asset/       # Asset loading and management
├── astraweave-asset-pipeline/ # Asset build pipeline
├── astraweave-audio/       # Spatial audio (rodio 0.17)
├── astraweave-behavior/    # Behavior tree engine
├── astraweave-cinematics/  # Cutscene system
├── astraweave-context/     # Context management
├── astraweave-coordination/# Multi-agent coordination
├── astraweave-core/        # Shared types and schema
├── astraweave-dialogue/    # Dialogue system
├── astraweave-director/    # AI director
├── astraweave-ecs/         # Entity Component System
├── astraweave-embeddings/  # Text embeddings
├── astraweave-fluids/      # SPH fluid simulation
├── astraweave-gameplay/    # Combat, crafting, quests
├── astraweave-input/       # Input handling (gilrs 0.10)
├── astraweave-ipc/         # Inter-process communication
├── astraweave-materials/   # Material system
├── astraweave-math/        # SIMD math (glam 0.30)
├── astraweave-nav/         # Navmesh pathfinding
├── astraweave-physics/     # Physics (rapier3d 0.22)
├── astraweave-render/      # GPU rendering (wgpu 25.0.2)
├── astraweave-scene/       # Scene graph and streaming
├── astraweave-sdk/         # C ABI (cbindgen 0.29)
├── astraweave-terrain/     # Procedural terrain
├── astraweave-ui/          # In-game UI (egui 0.32)
├── astraweave-weaving/     # Veilweaver fate mechanic
├── examples/               # 59 runnable example packages
├── tools/                  # Editor and CLI tools
├── docs/                   # Internal documentation
├── gh-pages/               # This documentation site
└── scripts/                # Build and setup scripts
```

The workspace contains **128 packages** total (69 non-example + 59 examples).

## Build Timings

| Build Type | Duration |
|------------|----------|
| First build (cold) | 15–45 min |
| Core incremental | 8–15 sec |
| Full workspace check | 2–4 min |

[← Back to Home](index.html)
