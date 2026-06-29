# AstraWeave: Master Strategic Roadmap

**Version**: 1.53  
**Last Updated**: June 29, 2026  
**Status**: Authoritative Source  
**Validation**: ✅ PASS — [Full Report](../current/ENGINE_VALIDATION_2026_01_13.md)

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Project Status](#project-status)
3. [Architecture Overview](#architecture-overview)
4. [Strategic Roadmap](#strategic-roadmap)
5. [Success Metrics](#success-metrics)
6. [Completed Phases](#completed-phases)
7. [Revision History](#revision-history)

---

## Executive Summary

### Mission

AstraWeave is an **AI-native game engine** where AI agents are first-class citizens, not bolted-on features. The entire codebase is developed iteratively by AI Agents (GitHub Copilot/Antigravity) with zero human-written code.

### Current State (June 10, 2026)

| Metric | Value |
|--------|-------|
| **Validation Status** | ✅ **A+ Grade (95/100)** |
| **Tests Passing** | **5,372/5,383 (99.8%)** (core validation suite) |
| **Compilation** | **17/17 core crates** (0 errors, 0 warnings) |
| **Miri Validation** | ✅ **1,059 tests, 0 UB** |
| **Kani Verification** | ✅ **69 proof harnesses, all passing** |
| Workspace Packages | **130** (59 core + 71 examples/tools; `cargo metadata --no-deps`, 2026-06-10 — adds `astraweave-alloc` and `astraweave-camera`) |
| Total Tests | **~39,900** `#[test]`/`#[tokio::test]` markers (live count 2026-06-10) <!-- Source: CLAIMS_REGISTRY.md#test-markers-total --> |
| Integration Tests | **~9,081** |
| Test Coverage (Overall) | **57.35%** whole-workspace line (`cargo llvm-cov --workspace`, 2026-06-29; supersedes the 59.3% 29-crate subset) |
| High-Coverage Crates (≥85%) | **14 of 28** in the prior 2026-02-25 subset run (whole-workspace per-crate not re-broken-out) |
| Frame Time (1K entities) | ~0.97 ms (system) / ~0.71 ms (mimalloc) — 2.70 ms was the Week-8 target |
| Agent Capacity | 12,700+ @ 60 FPS |
| **Determinism** | **100% validated** |
| **Memory Safety** | ✅ **Miri-verified** |
| **Mutation Testing** | Wave 1: 767 manual + Wave 2: 1,261 automated + Wave 3: 489 targeted + Wave 4: 411 fluids |

**Recent Achievements**:
- (Jun 9–10, 2026): **Net-Trio-Remediation W.1–W.5** — canonical HMAC-SHA256 sign/verify in `aw-net-proto` (XOR `sign16` stub deleted), server verify-first with `SignatureFailurePolicy::Kick` default, TLS-path coverage; trio 53 → 104 tests — [Findings](../audits/net_trio_signature_remediation_findings_2026-06.md)
- (Jun 6, 2026): **Editor Multi-Tool Sub-phase 4 COMPLETE** (dispatcher Pattern A regression net) + Sub-phase 5.A/5.B landed (RegionalArchetypePanel ActiveTool); SP5.C closeout pending — [Campaign](../current/EDITOR_MULTI_TOOL_ARCHITECTURE_CAMPAIGN.md)
- (Jun 2, 2026): **Terrain Asset Quality campaign closed** — texture VRAM baselined at 80 MiB/active pack (31% of 256 MB soft budget); BC7/KTX2 cook path flagged broken (runtime uploads uncompressed RGBA8) — [Outcome](../audits/terrain_asset_quality_outcome_2026-06.md)
- (Jun 1, 2026): **Unified Camera C.0–C.9 closed** — new `astraweave-camera` crate; 8 divergent camera codepaths consolidated into one `RenderView` upload contract; 3 parallel cinematics keyframe systems retired — [Outcome](../audits/unified_camera_outcome_2026-06.md)
- (May 17, 2026): **Editor-Engine Render Parity P.1–P.7 closed** — bit-identical (SHA-256) editor↔engine WYSIWYG parity, five seams structurally protected, public regression harness — [Outcome](../audits/editor_engine_render_parity_outcome_2026-05.md)
- (May 14, 2026): **Editor Multi-Tool Sub-phase 3 closed** — Mediator Brush Architecture complete (Real-Fix.A–E + cleanup chain); 8/8 brush modes operational; erosion chunk seams eliminated
- (Apr 17, 2026): **mimalloc allocator merge** — `astraweave-alloc` crate (`fast-alloc` feature); `ai.goap.plan` −52%, profiling_demo FPS +43% — [Experiment](../audits/mimalloc_experiment_2026-04-17.md)
- (Mar 25, 2026): Blueprint Zone Editor & Blend Scene Integration — 9 phases, polygon zone editor, Replica/Inspired placement, heightmap injection, 125+ tests — [Details](../current/ARCHITECTURE_REFERENCE.md#blueprint-zone-system)
- (Feb 25, 2026): Master Coverage Report v5.0 — full re-measurement of 28 crates via `cargo llvm-cov` — [Details](../current/MASTER_COVERAGE_REPORT.md)
- (Feb 2026): Wave 2 automated mutation testing — astraweave-prompts 100% kill rate (792 mutants)
- (Feb 3, 2026): Miri memory safety validation complete — 4 crates, 977 tests, zero undefined behavior — [Details](../current/MIRI_VALIDATION_REPORT.md)

### What AstraWeave Is

- ✅ A **working prototype** with solid foundations
- ✅ An **AI-native game engine** with unique architecture
- ✅ A **comprehensive experiment** proving AI can build complex systems
- ✅ **Memory-safe**: All unsafe code Miri-validated
- ✅ Approximately **3-12 months away** from production readiness

### What AstraWeave Is Not

- ❌ A fully production-ready game engine
- ❌ A feature-complete Unity/Unreal alternative
- ❌ Ready to ship commercial games today

---

## Project Status

### Core Capabilities ✅

| System | Status | Key Features |
|--------|--------|--------------|
| **ECS** | Production | Archetype-based, BlobVec storage, deterministic, 8-stage pipeline, 728 tests (330 lib + 398 integration), **Miri-validated**, **Kani-verified** |
| **AI** | Production | 7 planning modes (feature-gated), GOAP, Behavior Trees, LLM integration, 268 tests <!-- Source: CLAIMS_REGISTRY.md#ai-modes --> |
| **Rendering** | Production | wgpu 25, PBR/IBL, CSM shadows, post-processing, **Headless Support**, 806+ lib tests |
| **Physics** | Production | Rapier3D, character controller, spatial hashing, fluids (738 test markers), 1,244 tests <!-- Source: CLAIMS_REGISTRY.md#fluids-test-markers --> |
| **Audio** | Production | Spatial audio, 4-bus mixer, rodio backend, 239 tests |
| **Navigation** | Production | Navmesh, A*, portal graphs, 216 lib tests / 496 total |
| **SDK** | Production | C ABI FFI, **Miri-validated**, **Kani-verified** |
| **Prompts** | Production | Template engine, prompt library, 1,931 tests, 100% mutation kill rate |
| **Fluids** | In-design (zero consumers outside crate) | PBD solver, 738 test markers <!-- Source: CLAIMS_REGISTRY.md#fluids-test-markers --> |

### Recent Additions (February 2026)

| Feature | Location | Tests |
|---------|----------|-------|
| **✅ Blueprint Zone Editor** | `astraweave-terrain`, `astraweave-blend`, `aw_editor` | **125+** |
| **✅ Master Coverage Audit v5.0** | 28 crates measured via `cargo llvm-cov` | **~18,200** |
| **✅ Wave 2 Automated Mutation Testing** | prompts (792), render (339), editor (130) | **1,261+** |
| **✅ Miri Memory Safety Validation** | ecs, math, core, sdk | **1,059** |
| **✅ Kani Formal Verification** | ecs, math, core, sdk | **69 proofs** |
| **✅ Fluids System** | `astraweave-fluids` | **738** <!-- Source: CLAIMS_REGISTRY.md#fluids-test-markers --> |
| **✅ Prompt Engineering System** | `astraweave-prompts` | **1,931** |
| Physics Robustness (Phase 8.8) | `astraweave-physics` | 1,244+ |
| ECS BlobVec Optimization | `astraweave-ecs/src/archetype.rs` | 728 |
| Headless Renderer & GPU Verification | `astraweave-render/src/renderer.rs` | 806+ |
| AI-Orchestrated Dynamic Terrain | Multiple crates | 2,536 |
| Memory System | `astraweave-memory` | 945 |
| Editor (aw_editor) | `tools/aw_editor` | ~9,427 <!-- Source: CLAIMS_REGISTRY.md#editor-test-markers --> |

### Test Coverage by Priority Tier

> **Source**: [MASTER_COVERAGE_REPORT.md v5.0.0](../current/MASTER_COVERAGE_REPORT.md) (2026-02-25, LLVM source-based)

| Tier | Description | Weighted Line Coverage | Crates |
|------|-------------|----------------------|--------|
| P0 | Core Engine (ECS, AI, Render, Physics, Nav, etc.) | **55.4%** | 12 crates, 137,617 lines |
| P1 | Important Systems (Cinematics, PCG, Materials, etc.) | **58.9%** | 5 crates, 25,136 lines |
| P2 | Support & Infrastructure (Input, Memory, Security, etc.) | **73.9%** | 6 crates, 38,419 lines |

> **Note**: Coverage methodology changed in v5.0 from manual source-file analysis to `cargo llvm-cov --lib --summary-only`. Numbers include dependency code inflation and exclude integration tests. 14 of 28 measured crates exceed 85% line coverage. See coverage report for details.

---

## Architecture Overview

### Intelligence Loop

```
Perception → Reasoning → Planning → Action → Validation → Simulation
```

The core game loop is an **Intelligence Loop** where:

1. **Perception**: Agents see the world through WorldSnapshot
2. **Reasoning**: LLMs and Utility systems analyze state
3. **Planning**: GOAP and Behavior Trees formulate plans
4. **Action**: Plans execute via deterministic ECS commands
5. **Validation**: Tool sandbox validates all actions
6. **Simulation**: ECS updates world state

### ECS System Stages

| Stage | Order | Purpose |
|-------|-------|---------|
| PRE_SIMULATION | 1 | Setup, initialization |
| PERCEPTION | 2 | Build WorldSnapshots, update sensors |
| SIMULATION | 3 | Game logic, cooldowns, state |
| AI_PLANNING | 4 | Generate PlanIntents |
| PHYSICS | 5 | Apply forces, collisions |
| POST_SIMULATION | 6 | Cleanup, constraints |
| PRESENTATION | 7 | Rendering, audio, UI |

### AI Arbiter System

The **Arbiter** (`astraweave-ai/src/ai_arbiter.rs`) orchestrates the handshake between LLM strategic planning and GOAP tactical execution:

```
┌─────────────────────────────────────────────────────────────┐
│                      AI Arbiter                              │
├─────────────────────────────────────────────────────────────┤
│  WorldSnapshot → [LLM Strategic] ↔ [GOAP Tactical] → Action │
│                        ↓                  ↓                  │
│              Long-term Goals      Short-term Plans           │
│              (3-10 seconds)       (per-frame)                │
└─────────────────────────────────────────────────────────────┘
```

| Mode | Description | Latency |
|------|-------------|---------|
| **Classical** | Rule-based fallback | ~0.20ms |
| **BehaviorTree** | Hierarchical state machines | ~0.17ms |
| **Utility** | Weighted action scoring | ~0.46ms |
| **LLM** | Strategic planning (runtime default phi3:medium; model set via `OLLAMA_MODEL`) | ~3.5s |
| **Hybrid** | LLM strategy + GOAP execution | ~2.2s |
| **Ensemble** | Multi-mode weighted voting | ~2.4s |

Key features:
- **Async LLM polling**: Non-blocking (<10 µs overhead)
- **Automatic fallback**: LLM → GOAP → Utility → Classical
- **Metrics tracking**: Transition success rates, execution times

**Core Crates** (57):
- `astraweave-ecs`, `astraweave-ai`, `astraweave-core`
- `astraweave-render`, `astraweave-physics`, `astraweave-audio`
- `astraweave-nav`, `astraweave-behavior`, `astraweave-llm`
- `astraweave-security`, `astraweave-net`, `astraweave-scripting`
- `astraweave-fluids`, `astraweave-prompts`, `astraweave-memory`
- `astraweave-optimization`, `astraweave-observability`, `astraweave-profiling`
- ...and 39 more

**Examples/Tools** (71):
- `hello_companion`, `unified_showcase`, `aw_editor`
- Integration demos, test harnesses, CLI tools

---

## Strategic Roadmap

### Phase 8: Game Engine Readiness (In Progress)

**Objective**: Transform from "production-ready infrastructure" to "ship a game on it"

| Priority | Component | Timeline | Status |
|----------|-----------|----------|--------|
| 1 | In-Game UI Framework | 5 weeks | ✅ Complete (331 tests) |
| 2 | Complete Rendering Pipeline | 4-5 weeks | ✅ Complete (806+ tests) |
| 3 | Save/Load System | 2-3 weeks | ✅ Complete (160 tests) |
| 4 | Production Audio | 2-3 weeks | ✅ Complete (239 tests) |
| 8.8 | Physics Robustness Upgrade | 4 phases | 🟡 In Progress (1,244 tests) |

**Recent Additions (December 2025)**:
- `astraweave-ui/src/gamepad.rs` - PlayStation controller support (7 tests)
- `astraweave-ui/src/accessibility.rs` - Colorblind modes + high-contrast (12 tests)

### Phase 9: Distribution & Polish ✅

| Component | Timeline | Status |
|-----------|----------|--------|
| Scripting Runtime | 4 weeks | ✅ Complete (179 tests) |
| Asset Pipeline | 2 weeks | ✅ Complete (48 tests) |
| Build & Packaging | 2 weeks | ✅ Complete (aw_build + aw_release) |
| Profiling Tools | 1 week | ✅ Tracy integrated |

### Phase 10: Advanced Features ✅

| Component | Timeline | Status |
|-----------|----------|--------|
| Multiplayer Networking | 4-6 weeks | ✅ Complete (11 tests) |
| Global Illumination | 3-4 weeks | ✅ Complete (355 render tests, VXGI) |
| Steam Platform | 2 weeks | ✅ Complete (8 tests) |
| **AI-Orchestrated Terrain** | 2-3 weeks | ✅ Complete (2,536 tests) |

---

## Success Metrics

### Current vs Targets

| Metric | Current | 12-Month Target | Status |
|--------|---------|-----------------|--------|
| Test Coverage (Overall) | 57.35% (whole-workspace line) | 70%+ | 🟡 Below target (whole-workspace re-baseline 2026-06-29; supersedes 59.3% subset) |
| Test Coverage (P0 top-5) | 85-96% (ECS, Physics, Math, Nav, Behavior) | 90%+ | ✅ Met (top 5 P0 crates) |
| Total Tests | ~39,900 markers | 20,000+ | ✅ Exceeded <!-- Source: CLAIMS_REGISTRY.md#test-markers-total --> |
| Integration Tests | ~9,081 | 5,000+ | ✅ Exceeded |
| Mutation Kill Rate (prompts) | 100% (792 mutants) | 90%+ | ✅ Exceeded |
| `.unwrap()` in Core | 0 | 0 | ✅ Complete |
| Frame Time (1K entities) | ~0.97 ms (system) / ~0.71 ms (mimalloc) | <10ms | ✅ Exceeded (2.70 ms was the Week-8 target) |
| Agent Capacity | 12,700+ | 10,000+ | ✅ Exceeded |
| Kani Proofs | 69 | 50+ | ✅ Exceeded |
| LLM Quality Score | 75-85% | 95%+ | 🟠 In Progress |

### Performance Baselines

| Subsystem | Metric | Value |
|-----------|--------|-------|
| ECS | World creation | 25.8 ns |
| ECS | Entity spawn | 420 ns |
| AI Core Loop | Tick | 184 ns - 2.10 µs |
| GOAP | Cache hit | 1.01 µs |
| Behavior Trees | Tick | 57-253 ns |
| Physics | Full tick | 6.52 µs |
| Frame Time | Overall | ~0.97 ms (system) / ~0.71 ms (mimalloc) @ 1,000 entities — 2.70 ms was the Week-8 target |

---

## Completed Phases

### Phase 1-8: Rendering System ✅

**Completed**: November 2025 (~15 hours, 36/36 tasks)

Key achievements:
- PBR with Cook-Torrance BRDF
- Image-Based Lighting with environment maps
- Cascaded Shadow Maps (4 cascades, PCF)
- Post-processing (Bloom, SSAO, SSR, Motion Blur, DoF)
- GPU particle system with compute shaders
- Back-face culling (~40% performance improvement)

### Phase 2: AI Hardening ✅

**Completed**: November 2025

Key achievements:
- GOAP infinite loop prevention
- LLM client stabilization (history tracking)
- Streaming API (44.3× time-to-first-chunk improvement)
- Batch executor (6-8× throughput improvement)

### Phase 8.7: LLM Testing ✅

**Completed**: November 2025

Key achievements:
- 107+ tests added (100% pass rate)
- RAG integration with lifecycle management
- Persona system stabilization

### Phase 9.2: Scripting Foundation ✅

**Completed**: November 2025 (Sprint 2)

Key achievements:
- `astraweave-scripting` crate with Rhai v1.23
- `CScript` component for ECS integration
- Event system with `ScriptEvent` enum

### Phase 10.4: AI-Orchestrated Dynamic Terrain ✅

**Completed**: December 8, 2025

Key achievements:
- TerrainSolver for LLM-to-world coordinate translation
- TerrainModifier with time-sliced voxel operations
- NavMesh invalidation and partial rebaking
- Terrain persistence with compression
- LLM prompt templates for terrain generation
- Terrain-driven quest generation (20 feature types)
- API expansion (`apply_damage`, `set_position`)

### Blueprint Zone Editor & Blend Scene Integration ✅

**Completed**: March 25, 2026

Key achievements:
- Polygon-based Blueprint Zone data model with `ZoneRegistry` (CRUD, spatial queries, JSON persistence)
- `ZoneScatterGenerator` with Replica (1:1 fixed placement) and Inspired (procedural) modes
- `heightmap_raster` module for rasterizing `.blend` terrain meshes → heightmaps + fixed placements
- Boundary blending with smoothstep falloff + `BrushMode::ZoneBlend` manual paint brush
- `BlueprintPanel` — full 2D canvas editor with polygon tools, zone inspector, undo/redo
- `BlueprintOverlay` — 3D viewport wireframe projection via `DebugLine` renderer pass
- `BlendAssetScanner` for asset browser `.blend` discovery
- Adaptive scaling (density/scale/position) when zone area differs from source scene footprint
- System wiring: action dispatch, overlay sync, save/load with panel state roundtrip
- 125+ tests across 9 test suites (24 zone model + 16 scatter + 11 scatter e2e + 11 raster + 10 raster e2e + 9 biome pack + 17 panel + 7 overlay + 8 scanner)

---

## Revision History

> **IEEE/ACM-Compliant Format** (restructured 2026-01-18)

### Executive Summary

| Metric | Value |
|--------|-------|
| **Total Versions** | 22 |
| **Timeline** | Nov 22, 2025 → Mar 2026 (~4 months) |
| **Primary Authors** | AI Team |
| **Average Frequency** | ~3 days/version |

### Version Type Legend

| Symbol | Type | Description |
|--------|------|-------------|
| 🔷 | MAJOR | Phase completion, strategic milestones |
| 🔹 | MINOR | Feature additions, sprint completion |
| 🔸 | PATCH | Bug fixes, small updates |
| 🔍 | AUDIT | Validation, verification sessions |
| ⚠️ | FIX | Critical bug fixes, regressions |
| 📋 | PLAN | Documentation, planning updates |

### Impact Grade Legend

| Grade | Symbol | Criteria |
|-------|--------|----------|
| 🔴 | CRITICAL | Phase completion, >100 tests, strategic change |
| 🟡 | SIGNIFICANT | Sprint complete, >20 tests, milestone |
| 🟢 | INCREMENTAL | <20 tests, documentation, minor updates |
| ⚪ | ADMINISTRATIVE | Planning, documentation-only |

---

### Primary Revision Table

| Ver | Date | Type | Impact | Summary (≤80 chars) |
|-----|------|------|--------|---------------------|
| **1.53** | Jun 29 | 🔍 | 🟡 | D.3.2b doc-truth cleanup: fluids test markers corrected to 738 (post-W.1; rows :86/:91/:102) |
| **1.52** | Jun 29 | 🔍 | 🟡 | D-series Path-B.2: coverage 57.35% whole-workspace (was 59.3% subset); miri 1,059 |
| **1.49** | Mar 25 | 🔷 | 🟡 | Blueprint Zone Editor: 9 phases, polygon zones, Replica/Inspired, 125+ tests |
| **1.48** | Feb 27 | 🔷 | 🟡 | Blend Import Pipeline: 7 phases, BiomePack, editor panel, 97 tests |
| **1.47** | Feb 26 | 🔍 | 🟡 | Roadmap audit: corrected metrics, 128 pkgs, ~27K tests, v5.0 coverage |
| **1.46** | Feb 10 | 🔷 | 🔴 | Veilweaver vertical slice: 5 phases, 20 modules, 320 tests, zero unsafe |
| **1.45** | Feb 3 | 🔷 | 🟡 | Miri validation: 977 tests, 4 crates, zero UB |
| **1.44** | Jan 26 | 📋 | 🟢 | Revised Validation Plan v2.0: Sanitizers, CI workflow, validate.ps1 |
| **1.43** | Jan 26 | ⚠️ | 🔴 | ECS REGRESSION FIXED: BlobVec 52-68% faster, 10K+ entities restored |
| **1.42** | Dec 21 | 🔍 | 🟢 | Renderer production validation: Headless verification complete |
| **1.41** | Dec 20 | 🔹 | 🟡 | Renderer Headless: +365 tests, 42% coverage, CI-safe testing |
| **1.40** | Dec 8 | 🔷 | 🔴 | AI Terrain: 8 phases, TerrainSolver, NavMesh, 320+ tests |
| **1.39** | Dec 6 | 🔹 | 🟡 | Steam Platform: astraweave-steam, Steamworks SDK, achievements |
| **1.38** | Dec 6 | 🔍 | 🟡 | Phase 10 GI: VXGI complete, VxgiRenderer, 355 render tests |
| **1.37** | Dec 6 | 🔷 | 🟡 | Phase 10 Networking: astraweave-net/net-ecs, TLS/WebSocket/HMAC |
| **1.36** | Dec 6 | 🔷 | 🟡 | Phase 9 complete: Build/Packaging, release.yml Win/Mac/Linux |
| **1.35** | Dec 6 | 🔍 | 🟢 | Phase 9 audit: Scripting 12 tests, Asset Pipeline 48 tests |
| **1.34** | Dec 6 | 🔷 | 🟡 | Phase 8.4 Audio complete: #[serial] fix, 147 tests passing |
| **1.33** | Dec 5 | 🔷 | 🟡 | Phase 8.3 Save/Load: Debug thresholds fixed, 52 tests |
| **1.32** | Dec 5 | 🔷 | 🟡 | Phase 8.1 UI Framework: gamepad/accessibility, 153 tests |
| **1.31** | Dec 5 | 📋 | 🟢 | Docs audit: 82→121 crates, TLS/Security, docs reorg |
| **1.30** | Nov 23 | 🔹 | 🟢 | Phase 9.2 Sprint 2: Event system, API expansion |
| **1.29** | Nov 23 | 🔹 | 🟢 | Phase 9.2 started: astraweave-scripting crate |
| **1.28** | Nov 23 | 🔹 | 🟢 | Phase 8.7 Sprint 4: Full stack integration |
| **1.27** | Nov 22 | 🔹 | 🟢 | Phase 8.7 RAG integration complete |

---

### Statistical Summary

**By Type:**
| Type | Count | % |
|------|-------|---|
| 🔷 MAJOR | 9 | 40.9% |
| 🔹 MINOR | 6 | 27.3% |
| 📋 PLAN | 2 | 9.1% |
| 🔍 AUDIT | 3 | 13.6% |
| ⚠️ FIX | 1 | 4.5% |

**By Impact:**
| Impact | Count | % |
|--------|-------|---|
| 🔴 CRITICAL | 2 | 9.1% |
| 🟡 SIGNIFICANT | 11 | 50.0% |
| 🟢 INCREMENTAL | 8 | 36.4% |

---

### Key Milestones Timeline

```
Nov 22 ─── v1.27: Phase 8.7 RAG Integration
    │
Nov 23 ─── v1.28-1.30: Sprint 4, Scripting Foundation
    │
Dec 5 ─┬─ v1.32: Phase 8.1 UI Framework (153 tests)
       ├─ v1.33: Phase 8.3 Save/Load (52 tests)
       └─ v1.34: Phase 8.4 Audio (147 tests)
    │
Dec 6 ─┬─ v1.36: Phase 9 Build/Packaging
       ├─ v1.37: Phase 10 Networking (TLS/WS)
       └─ v1.39: Steam Platform Integration
    │
Dec 8 ─── v1.40: AI-Orchestrated Dynamic Terrain (2,536 tests)
    │
Dec 20-21 ─ v1.41-1.42: Renderer Headless (+365 tests)
    │
Jan 2026 ─ v1.43-1.44: ECS Regression Fix, Validation Plan v2.0
    │
Feb 3 ─── v1.45: Miri Validation (977 tests, 0 UB)
    │
Feb 10 ── v1.46: Veilweaver Vertical Slice (320 tests, 20 modules)
    │
Feb 26 ── v1.47: Roadmap Audit (128 pkgs, ~27K tests, coverage v5.0)
    │
Feb 27 ── v1.48: Blend Import Pipeline (7 phases, 97 tests)
    │
Mar 25 ── v1.49: Blueprint Zone Editor (9 phases, 125+ tests)
```

---

### Detailed Changelog (Critical Versions)

<details>
<summary><b>v1.47 (Feb 26, 2026) - ROADMAP ACCURACY AUDIT</b></summary>

**Impact**: 🟡 SIGNIFICANT  
**Type**: 🔍 AUDIT  
**Author**: AI Team

**Changes**:
- Full cross-reference audit of roadmap against actual codebase
- **Corrected workspace packages**: 121 → 128 (57 core + 71 examples/tools)
- **Corrected total tests**: 7,600+ → ~27,000+ measured / ~35,000 `#[test]` markers
- **Corrected integration tests**: 215 → ~9,081
- **Updated coverage methodology**: replaced stale 78%/94.71% with v5.0 LLVM-based measurements (59.3% weighted, 14 crates at 85%+)
- **Added missing crates**: astraweave-fluids (4,907 tests), astraweave-prompts (1,931 tests), astraweave-memory (945 tests), astraweave-optimization
- **Added missing milestones**: Wave 2 mutation testing, Coverage audit v5.0, Phase 8.8 Physics
- **Fixed Phase 8 status**: marked as In Progress (Phase 8.8 Physics Robustness ongoing)
- **Added Kani verification**: 69 proof harnesses across 4 crates
- **Updated per-crate test counts**: Nav 66 → 216 lib / 496 total, ECS 386 → 728 total, Render 369 → 806+ lib
- **Updated success metrics targets** to reflect current scale

</details>

<details>
<summary><b>v1.46 (Feb 2026) - VEILWEAVER VERTICAL SLICE</b></summary>

**Impact**: 🔴 CRITICAL  
**Type**: 🔷 MAJOR  
**Author**: AI Team

**Changes**:
- Complete 5-phase Veilweaver vertical slice (`veilweaver_slice_runtime` crate)
- `#![forbid(unsafe_code)]` — zero unsafe, headless-safe, no wgpu/egui deps
- **Phase 1**: Core game loop, zone registry (5 zones), ECS integration
- **Phase 2**: Dialogue system, cinematics player, storm choice branching
- **Phase 3**: Boss HUD (3-phase health bar), companion affinity meter, telemetry
- **Phase 4**: VFX descriptors (6 categories), audio specs (10+ cue types), palette
- **Phase 5**: Determinism validation (3-run hash consistency), perf budget tracker (p50/p95/p99), save/checkpoint, 30-min pacing simulation, edge case hardening
- **Edge case hardening**: NaN guards on all animation ticks, `.expect()` → guard clause, VecDeque for O(1) eviction, Default impls, `.first()` vs direct-index safety
- **20 source modules**, 6 integration test suites, 320 tests (265 unit + 55 integration)
- Clippy clean, fmt clean, zero warnings in crate
- New crate: `veilweaver_slice_runtime v0.1.0`

</details>

<details>
<summary><b>v1.43 (Jan 2026) - ECS REGRESSION FIX</b></summary>

**Impact**: 🔴 CRITICAL  
**Type**: ⚠️ FIX  
**Author**: AI Team

**Changes**:
- Fixed critical BlobVec performance regression from NASA-grade audit v5.52
- Root cause: HashMap allocation in Archetype::new() even in legacy mode
- Fix: Lazy initialization with Option<HashMap>
- Results: entity_spawn 52% faster, entity_despawn 68% faster
- 10K+ entities @ 60 FPS restored, all 220 ECS tests passing
- Grade: B- → A+

</details>

<details>
<summary><b>v1.40 (Dec 8, 2025) - AI-ORCHESTRATED DYNAMIC TERRAIN</b></summary>

**Impact**: 🔴 CRITICAL  
**Type**: 🔷 MAJOR  
**Author**: AI Team

**Changes**:
- Complete 8-phase implementation:
  - TerrainSolver for LLM-to-world coordinate translation
  - TerrainModifier with time-sliced voxel operations
  - NavMesh invalidation and partial rebaking
  - Terrain persistence with compression
  - LLM prompt templates for terrain generation
  - Terrain-driven quest generation (20 feature types)
  - API expansion (`apply_damage`, `set_position`)
- 320+ tests added

</details>

<details>
<summary>v1.45 (2026-02-03) — Miri Memory Safety Validation</summary>

**Impact**: 🟢 ENHANCEMENT  
**Type**: 🔷 MAJOR  
**Author**: AI Team

**Changes**:
- Comprehensive Miri validation across 4 crates with unsafe code
- 977 tests validated with zero undefined behavior detected
- Crates validated: astraweave-ecs (386 tests), astraweave-math (109 tests), astraweave-core (465 tests), astraweave-sdk (17 tests)
- Memory safety certification for: BlobVec, SparseSet, EntityAllocator, SIMD intrinsics, C ABI FFI
- Documentation: [MIRI_VALIDATION_REPORT.md](../current/MIRI_VALIDATION_REPORT.md), [ECS_MIRI_VALIDATION_REPORT.md](../current/ECS_MIRI_VALIDATION_REPORT.md)

</details>

---

### Issues Resolved During Restructuring

| ID | Issue | Resolution |
|----|-------|------------|
| VERBOSE-001 | Entries 50-200 words each | Summarized to ≤80 chars |
| FORMAT-001 | Missing type/impact classification | Added 6 types + 4 impact grades |
| STATS-001 | No statistical summary | Added type/impact distribution |
| TIMELINE-001 | No visual timeline | Added ASCII milestone visualization |

---

**Next Review Date**: 2026-07-10 (monthly cadence)  
**Revision History Format Version**: 2.0.0 (IEEE/ACM-compliant)  
**Last Restructured**: 2026-04-05

### v1.51 (2026-06-10) — Engine Health Audit + Six-Campaign Reconciliation

**Scope**: Full engine-health-status audit and roadmap reconciliation; records all campaigns completed since v1.50 (the header had remained at v1.49/2026-03-25 despite the v1.50 entry below — an internal inconsistency now corrected).

**Key Changes**:
- **Net-Trio-Remediation W.1–W.5** (Jun 9–10): HMAC-SHA256 input signing canonical and enforced end-to-end; XOR `sign16` deleted; `SignatureFailurePolicy::Kick` default; trio 53 → 104 tests
- **Editor Multi-Tool Architecture**: Sub-phase 3 closed (May 14, Real-Fix.A–E + cleanup chain), Sub-phase 4 closed (Jun 6); Sub-phase 5 in flight (5.A/5.B landed Jun 6)
- **Unified Camera C.0–C.9 closed** (Jun 1): new `astraweave-camera` crate; one `RenderView` upload contract; net LoC reduction
- **Editor-Engine Render Parity P.1–P.7 closed** (May 17): bit-identical WYSIWYG parity harness, five seams protected
- **Terrain Asset Quality closed** (Jun 2): VRAM baseline 80 MiB/active pack; BC7/KTX2 cook path flagged broken
- **mimalloc merge recorded** (Apr 17): `astraweave-alloc`, `fast-alloc` default-on in profiling_demo/hello_companion/editor
- **Counts reconciled**: workspace packages 128 → 130; test markers ~39,973 (live count); mutation waves 3+4 added to summary
- **Build health**: `cargo check --workspace` 0 errors; all former known-build-issue crates compile clean (live audit 2026-06-10)

**Documents**: [PROJECT_STATUS](../current/PROJECT_STATUS.md) | [Architecture Map v0.7.2](../architecture/ARCHITECTURE_MAP.md) | [Coverage Report v5.3.0](../current/MASTER_COVERAGE_REPORT.md) | campaign outcome docs under `docs/audits/`

---

### v1.50 (2026-04-05) — Editor Behavioral Correctness Audit + Unified Pipeline

**Scope**: Complete behavioral correctness audit and remediation of `aw_editor`, culminating in unified rendering pipeline.

**Key Changes**:
- **37 fixes** across 47 commits addressing 14 CRITICAL, 18 HIGH, 28 MEDIUM findings
- **Unified rendering pipeline** (Fix 27): eliminated dual FastPreview/EnginePBR architecture, deleted ~4,669 LOC of duplicate rendering code
- **SOTA rendering upgrades**: Disney BRDF + multi-scatter energy compensation (Turquin 2019), 4-cascade CSM shadows with frustum fitting, IBL prefiltered cubemap, Khronos PBR Neutral tonemapper, 3-channel DFG LUT, glTF MikkTSpace tangent loading
- **Undo system completion**: All 9 previously-bypassing operations now route through undo stack
- **Silent failure resolution**: 60 error-swallowing patterns identified, critical ones fixed with structured logging

**Documents**: [Audit Report](../current/EDITOR_BEHAVIORAL_CORRECTNESS_AUDIT.md) | [Architecture Map](../architecture/ARCHITECTURE_MAP.md) | [Campaign Plan](../current/FIX27_UNIFIED_PIPELINE_CAMPAIGN.md)

---

*See `.github/copilot-instructions.md` for maintenance protocol and enforcement rules.*