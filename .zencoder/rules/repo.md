---
description: Repository Information Overview
alwaysApply: true
---
# **ALWAYS REFERENCE copilot-instructions.md FOR FULL ROLE AND RULES.**

# AstraWeave AI-Native Gaming Engine

## Summary

**AstraWeave** is a deterministic, ECS-based game engine in Rust where AI agents are first-class citizens. Built entirely through AI-generated iterative prompting, it spans 130 workspace members <!-- Source: CLAIMS_REGISTRY.md#workspace-members -->, wgpu 25.0.2 rendering, Rapier3D physics, AI orchestration (GOAP, behavior trees, local LLM via Ollama with `phi3:medium` as the runtime default), and extensive testing (96.9% determinism validation, 166+ tests passing).

## Repository Structure

### Core Engine
- **astraweave-ecs**: Archetype-based ECS with 8 deterministic system stages (incl. SYNC)
- **astraweave-ai**: AI orchestrator, core loop, tool sandbox, 7 planning modes (feature-gated) <!-- Source: CLAIMS_REGISTRY.md#ai-modes -->
- **astraweave-render**: wgpu 25.0.2 renderer with PBR materials, GPU skinning, LODs, mesh optimization
- **astraweave-physics**: Rapier3D integration, character controller; in-crate `SpatialHash` module is dormant (live broadphase is Rapier `DefaultBroadPhase`)
- **astraweave-nav**: Navmesh pathfinding with A* and portal graphs
- **astraweave-math**: SIMD-optimized vectors/matrices (2.08× speedup @ 10k entities)
- **astraweave-terrain**: Hybrid voxel/polygon with marching cubes mesh generation

### Supporting Systems
- **astraweave-audio**: Spatial audio with rodio backend
- **astraweave-dialogue**: Branching dialogue with audio mapping
- **astraweave-behavior**: Behavior trees and utility AI systems
- **astraweave-gameplay**: Combat physics, attack sweeps, damage calculation
- **astraweave-cinematics**: Timeline-based sequencing for camera/audio/FX
- **astraweave-persistence-ecs**: ECS world serialization and save/load
- **astraweave-scene**: World partitioning and async cell streaming

### Tools & Examples
- **tools/aw_editor**: 14-panel level/encounter editor (egui-wgpu)
- **tools/aw_asset_cli**: Asset pipeline and material management
- **tools/ollama_probe**: LLM connectivity validator (local LLM via Ollama)
- **examples/**: 40+ working examples (hello_companion, profiling_demo, unified_showcase, astract_gallery)

### Recent Additions (Phase 8.1)
- **Astract**: Declarative UI framework with animation system, 5 tutorials, 166/166 tests passing
- **UI Framework**: In-game menus, HUD, pause menu, settings, graphics/audio/controls UI

## Language & Runtime

**Language**: Rust  
**Version**: 1.89.0 (pinned via rust-toolchain.toml)  
**Edition**: 2021  
**Build System**: Cargo with workspace resolver v2  
**Platforms**: x86_64-unknown-linux-gnu, x86_64-pc-windows-msvc, x86_64-apple-darwin, aarch64-apple-darwin

## Dependencies

**Graphics**: wgpu 25.0.2, egui 0.32, winit 0.30, glam 0.30  
**Physics**: rapier3d 0.22, rand/rand_chacha 0.9 (deterministic RNG)  
**AI/LLM**: local LLM via Ollama (`phi3:medium` runtime default; Hermes/Qwen opt-in via `OLLAMA_MODEL`), rhai 1.23 (scripting)  
**Async**: tokio 1.x, tungstenite 0.28 (WebSocket)  
**Serialization**: serde 1, serde_json 1, toml 0.9, zip 6.0  
**Audio**: rodio 0.17 (spatial audio)  
**Security**: ed25519-dalek 2, sha2 0.10  
**Testing**: criterion 0.7, proptest (benchmarking, fuzz testing)  
**Profiling**: Tracy 0.11.1 (zero-overhead instrumentation)

## Build & Installation

```bash
# Automated setup
./scripts/bootstrap.sh    # Cross-platform

# Core build (2-5 min first build, 8-15s incremental)
cargo build -p astraweave-ecs -p astraweave-ai -p astraweave-physics -p astraweave-nav -p astraweave-render
cargo build --workspace  # All crates

# Development workflow
cargo fmt --all && cargo clippy --workspace --all-features -- -D warnings && cargo test --workspace
make dev                 # Comprehensive check
```

**Profiles**: dev (fast iteration), release (optimized runtime)

## Testing & Validation

**Framework**: Rust cargo test + criterion benchmarks  
**Test Coverage**: 96.9% determinism (31/32 tests), 166/166 Astract tests, 42/42 HUD tests  
**Benchmarks**: ECS (25.8 ns spawn), AI (184 ns – 2.10 µs), Physics (114 ns character move), Frame time (~0.97 ms system / ~0.71 ms mimalloc @ 1000 entities; 2.70 ms was the Week-8 target) <!-- Source: CLAIMS_REGISTRY.md#frame-time-1000-entities -->

**Run Tests**:
```bash
make test                              # All working crates
cargo test -p astraweave-ecs           # Specific crate
cargo bench -p astraweave-math         # Benchmarks (SIMD)
```

**Performance SLA**: 12,700+ agents @ 60 FPS with 100% determinism, 6.48M validation checks/sec

## Main Entry Points

**Working Examples**:
- `hello_companion`: All 7 AI modes (feature-gated) <!-- Source: CLAIMS_REGISTRY.md#ai-modes --> + local LLM via Ollama
- `profiling_demo`: Week 8 performance profiling with Tracy
- `astract_gallery`: UI framework showcase (Astract Gizmo complete)
- `unified_showcase`: Island + assets + rendering + physics

**Tools**:
- `tools/aw_editor`: Level editor with 14 UI panels
- `tools/aw_asset_cli`: Asset pipeline
- `tools/ollama_probe`: LLM connectivity verification

## Docker

**Status**: No official Docker images. Cross-compilation supported via rust-toolchain.toml.

```bash
cargo build --target x86_64-unknown-linux-gnu --release
```

## Key Achievements

- **Phase 6** ✅ (Oct 14): Hermes 2 Pro integration, 54 errors → 0
- **Week 8** ✅ (Oct 9-12): Performance sprint, 2.70 ms frame time (370 FPS, 84% headroom)
- **Phase 8.1** ✅ (Oct 14–Nov 3): UI framework weeks 1-3 + Astract Gizmo (animations, gallery, docs)
- **Option 3** ✅ (Nov 1): Determinism validation, 100% bit-identical replay, 31/32 tests
- **Integration Tests** ✅ (Oct 31): 800+ tests across 106 files, 10 integration paths validated

## Project Metadata

**Version**: 0.4.0  
**License**: MIT  
**Status**: Phase 6 complete, Phase 8 UI Framework in progress (dated Nov 2025; see `docs/current/PROJECT_STATUS.md` for current state)  
**Generation**: 100% AI-generated — 130 workspace members <!-- Source: CLAIMS_REGISTRY.md#workspace-members --> via iterative AI development  
**Updated**: November 10, 2025

## Documentation

- **Master Roadmap**: `docs/current/MASTER_ROADMAP.md` (Phase 8-10 planning)
- **Master Benchmark**: `docs/current/MASTER_BENCHMARK_REPORT.md` (Performance metrics)
- **Master Coverage**: `docs/current/MASTER_COVERAGE_REPORT.md` (Test coverage analysis)
- **Strategic Plans**: `docs/root-archive/` (Phase completion reports, validation summaries)
- **Setup Guide**: `docs/supplemental-docs/DEVELOPMENT_SETUP.md`
