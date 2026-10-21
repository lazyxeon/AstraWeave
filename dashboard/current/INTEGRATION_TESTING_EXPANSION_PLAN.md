# AstraWeave Integration Testing Expansion Plan

**Version**: 1.1.0  
**Date**: January 2025  
**Updated**: January 2025  
**Priority**: HIGH (User-specified: integration testing first)  
**Status**: ✅ PHASE 1 IMPLEMENTED

---

## Executive Summary

This document outlines the plan to expand integration testing across AstraWeave's critical subsystems. Integration tests validate that components work together correctly, catching issues that unit tests miss.

### Implementation Status

| Phase | Status | Tests Created |
|-------|--------|---------------|
| Phase 1: Core Pipeline | ✅ Complete | 22 tests |
| Phase 2: Persistence | ✅ Existing | (aw-save tests) |
| Phase 3: Editor | 🟡 Planned | - |
| Phase 4: Cross-Platform | ✅ Complete | CI workflow |

### Artifacts Created

| File | Location | Tests |
|------|----------|-------|
| ECS Pipeline | `astraweave-ecs/tests/full_pipeline_integration.rs` | 6 |
| Network Sync | `astraweave-net/tests/integration/snapshot_sync_tests.rs` | 6 |
| LLM Fallback | `astraweave-llm/tests/fallback_chain_integration.rs` | 10 |
| CI Workflow | `.github/workflows/integration-tests.yml` | - |

### Why Integration Testing First?

1. **Real-world validation**: Unit tests mock dependencies; integration tests use real components
2. **Interface contracts**: Catches breaking changes between crates
3. **Performance under load**: Tests realistic scenarios, not micro-benchmarks
4. **Determinism proof**: Validates multiplayer/replay correctness

---

## Current State

### Existing Integration Test Files

```
tests/
├── astraweave-ai/tests/
│   ├── goap_integration.rs           # GOAP planner + world state
│   ├── behavior_tree_integration.rs  # BT + ECS integration
│   └── arbiter_tests.rs              # GOAP + LLM arbiter
├── astraweave-net/tests/
│   └── integration/
│       └── packet_loss_tests.rs      # Network reliability
├── astraweave-physics/tests/
│   └── character_controller_tests.rs # Physics + ECS
├── tools/aw_editor/tests/
│   ├── integration_tests.rs          # Editor + world state
│   ├── comprehensive_smoke_tests.rs  # Full editor flow
│   └── workflow_project_lifecycle.rs # Project save/load
└── ...
```

### Gap Analysis

| Subsystem | Unit Tests | Integration Tests | Gap | Status |
|-----------|------------|-------------------|-----|--------|
| ECS + AI | ✅ 100+ | ✅ 6 | Implemented | ✅ Complete |
| Physics + ECS | ✅ 50+ | ⚠️ 2 | Need collision + component sync | 🟡 Planned |
| Net + ECS | ✅ 30+ | ✅ 6 | Implemented | ✅ Complete |
| LLM + AI | ✅ 60+ | ✅ 10 | Implemented | ✅ Complete |
| Audio + Scene | ✅ 40+ | ❌ 0 | No integration tests | 🟡 Planned |
| Asset + Render | ✅ 100+ | ✅ Existing | Has extensive tests | ✅ Complete |
| Save/Load | ✅ 20+ | ✅ 3+ | Has aw-save tests | ✅ Complete |

---

## Phase 1: Core Pipeline Integration (Week 1-2)

### 1.1 ECS → AI → Physics Pipeline

**File**: `astraweave-ecs/tests/full_pipeline_integration.rs`

```rust
//! Integration test: ECS → AI → Physics → Rendering
//! 
//! Validates the complete game loop works end-to-end:
//! 1. Spawn entities with components (ECS)
//! 2. Run AI decision making (AI orchestrators)
//! 3. Apply physics forces (Physics)
//! 4. Update transforms (Back to ECS)

#[test]
fn test_ecs_ai_physics_loop() {
    // TODO: Implement
    // - Spawn 10 AI agents with Position, Velocity, AIController
    // - Run AI tick to generate movement intents
    // - Run physics tick to apply forces
    // - Verify positions updated correctly
    // - Verify determinism (3 runs, same seed = same result)
}

#[test]
fn test_perception_to_action_flow() {
    // TODO: Implement
    // - Create WorldSnapshot from ECS world
    // - Pass to GOAP orchestrator
    // - Verify PlanIntent is valid
    // - Apply first ActionStep to world
    // - Verify world state changed as expected
}

#[test]
fn test_1000_agents_at_60fps() {
    // TODO: Implement
    // - Spawn 1000 agents
    // - Run 60 ticks
    // - Verify total time < 1 second (60 FPS budget)
    // - Verify no memory growth (leak detection)
}
```

### 1.2 Network Snapshot Integration

**File**: `astraweave-net/tests/integration/snapshot_sync_tests.rs`

```rust
//! Integration test: ECS world → Network snapshot → Delta → Reconstruct

#[test]
fn test_snapshot_roundtrip() {
    // TODO: Implement
    // - Create ECS world with 100 entities
    // - Convert to Snapshot
    // - Serialize/deserialize
    // - Verify all entity data preserved
}

#[test]
fn test_delta_compression_accuracy() {
    // TODO: Implement
    // - Create two world states (tick 0, tick 60)
    // - Generate delta
    // - Apply delta to tick 0 snapshot
    // - Verify result equals tick 60
}

#[test]
fn test_interest_management_filtering() {
    // TODO: Implement
    // - Create world with 3 teams, 50 entities each
    // - Apply RadiusTeamInterest filter
    // - Verify only visible entities included
}
```

### 1.3 LLM Fallback Chain

**File**: `astraweave-llm/tests/fallback_integration.rs`

```rust
//! Integration test: LLM → Hermes → Fallback → Heuristics

#[test]
fn test_full_fallback_chain() {
    // TODO: Implement
    // - Configure 4-tier fallback (LLM → Fast LLM → Heuristic → Emergency)
    // - Simulate LLM timeout
    // - Verify graceful degradation
    // - Verify action still produced
}

#[test]
fn test_streaming_parser_with_real_chunks() {
    // TODO: Implement
    // - Feed realistic LLM output chunks
    // - Parse incrementally
    // - Verify partial plans available
}
```

---

## Phase 2: Persistence Integration (Week 2-3)

### 2.1 Save/Load Full World

**File**: `persistence/aw-save/tests/full_world_save_tests.rs`

```rust
//! Integration test: Full ECS world save/load cycle

#[test]
fn test_save_load_complex_world() {
    // TODO: Implement
    // - Create world with:
    //   - 500 entities
    //   - 10 different component types
    //   - Hierarchy relationships
    //   - AI state (current plan, cooldowns)
    // - Save to file
    // - Load into new world
    // - Verify bitwise equality
}

#[test]
fn test_save_load_determinism() {
    // TODO: Implement
    // - Run simulation for 100 ticks
    // - Save world
    // - Load world
    // - Run 100 more ticks
    // - Verify matches unsaved run
}

#[test]
fn test_version_migration() {
    // TODO: Implement
    // - Load save from older version
    // - Verify migration successful
    // - Verify gameplay unaffected
}
```

### 2.2 Asset Pipeline Integration

**File**: `astraweave-asset/tests/pipeline_integration.rs`

```rust
//! Integration test: Asset loading → Hot reload → Scene update

#[test]
fn test_gltf_to_scene_pipeline() {
    // TODO: Implement
    // - Load glTF file through asset pipeline
    // - Verify mesh data extracted
    // - Verify textures loaded
    // - Verify material bindings correct
}

#[test]
fn test_hot_reload_assets() {
    // TODO: Implement
    // - Load asset
    // - Modify source file
    // - Trigger reload
    // - Verify scene updated
}
```

---

## Phase 3: Editor Integration (Week 3-4)

### 3.1 Editor-Engine Round Trip

**File**: `tools/aw_editor/tests/engine_integration.rs`

```rust
//! Integration test: Editor commands → Engine state → UI update

#[test]
fn test_select_transform_undo_redo() {
    // TODO: Implement
    // - Select entity in scene
    // - Transform via gizmo
    // - Undo
    // - Verify position restored
    // - Redo
    // - Verify transform reapplied
}

#[test]
fn test_prefab_spawn_with_children() {
    // TODO: Implement
    // - Load prefab with hierarchy
    // - Spawn into scene
    // - Verify all children spawned
    // - Verify component data correct
}

#[test]
fn test_play_mode_transition() {
    // TODO: Implement
    // - Enter play mode
    // - Simulate AI + physics
    // - Exit play mode
    // - Verify editor state unchanged
}
```

---

## Phase 4: Cross-Platform Integration (Week 4)

### 4.1 Determinism Across Platforms

**File**: `tests/cross_platform_determinism.rs`

```rust
//! Integration test: Same seed = same result across platforms

#[test]
fn test_physics_determinism() {
    // Run on Linux, Windows, macOS
    // Verify identical results
}

#[test]
fn test_rng_determinism() {
    // Verify seeded RNG produces identical sequences
}
```

---

## CI Integration

### New Integration Test Workflow

**File**: `.github/workflows/integration-tests.yml`

```yaml
name: Integration Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]
  schedule:
    - cron: '0 4 * * *'  # Nightly

jobs:
  integration-core:
    name: Core Pipeline Integration
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: dtolnay/rust-toolchain@stable
      - name: Run ECS + AI + Physics integration
        run: cargo test -p astraweave-ecs --test full_pipeline_integration
      - name: Run Network integration
        run: cargo test -p astraweave-net --test snapshot_sync_tests
      
  integration-persistence:
    name: Persistence Integration
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: dtolnay/rust-toolchain@stable
      - name: Run Save/Load integration
        run: cargo test -p aw-save --test full_world_save_tests
        
  integration-determinism:
    name: Determinism Integration
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v5
      - uses: dtolnay/rust-toolchain@stable
      - name: Run determinism tests
        run: cargo test --workspace -- determinism
```

---

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Integration test files | 8 | 25+ |
| Integration tests | ~30 | 150+ |
| Cross-crate coverage | 3 paths | 12+ paths |
| Platform combinations | 1 | 3 (Linux, Windows, macOS) |
| CI integration test time | 0 | <10 min |

---

## Priority Order (Based on Risk)

1. **ECS → AI → Physics** (Critical path, highest usage)
2. **Network Snapshot Sync** (Multiplayer correctness)
3. **Save/Load Roundtrip** (User data integrity)
4. **LLM Fallback Chain** (Graceful degradation)
5. **Asset Pipeline** (Content pipeline reliability)
6. **Editor Commands** (Developer experience)
7. **Cross-Platform** (Platform parity)

---

## Timeline

| Week | Focus | Deliverables |
|------|-------|--------------|
| 1 | Core pipeline | 5 ECS+AI+Physics integration tests |
| 2 | Network + LLM | 5 Network + 3 LLM integration tests |
| 3 | Persistence | 5 Save/Load + 3 Asset integration tests |
| 4 | Editor + Platform | 5 Editor + 3 Cross-platform tests |

**Total New Tests**: 29 integration tests
**Estimated Time**: 4 weeks (part-time), 2 weeks (full-time)

---

## Appendix: Test Naming Convention

```rust
// Pattern: test_{subsystems}_{scenario}
fn test_ecs_ai_simple_movement() {}      // Basic case
fn test_ecs_ai_1000_agents() {}          // Scale case  
fn test_ecs_ai_determinism_3_runs() {}   // Property case
fn test_net_snapshot_corrupted_data() {} // Error case
```

---

## Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-01 | Copilot | Initial creation |
