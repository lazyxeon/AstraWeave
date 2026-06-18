# AI-Native Architecture Validation Test Suite

**Date Created**: October 13, 2025  
**Purpose**: Validate AstraWeave's AI-native architecture with tests  
**Scope**: 15 comprehensive tests across 5 phases  
**Status**: ✅ Test structure created, implementation in progress

---

## Test Philosophy

This test suite validates **actual implementations**, not aspirational features. We test:
- ✅ RuleOrchestrator (simple tactical AI)
- ✅ Tool Validation Framework (anti-cheat)
- ✅ GOAP Planner (cached planning)
- ✅ Behavior Trees (high-throughput AI)
- ✅ Core AI Loop (Perception → Planning → Action)

We **do NOT test** unimplemented features (LLM orchestrator, companion learning, etc.).

---

## Test Organization

### **Phase 1: Perception & WorldSnapshot** (`perception_tests.rs`)
Tests snapshot creation, distribution, and accuracy for multi-agent AI.

**Tests**:
1. `test_snapshot_accuracy` - Property-based testing (snapshot reflects world state)
2. `test_snapshot_throughput` - Benchmark 100/500/1000 agents receiving snapshots
3. `test_perception_stress` - Sustained perception over 1000 frames (<10% degradation)

**Success Criteria**:
- ✅ 1000 agents receive snapshots in <5ms
- ✅ Snapshot data matches world state (100% accuracy)
- ✅ <10% performance degradation over 1000 frames

---

### **Phase 2: Tool Validation Sandbox** (`tool_validation_tests.rs`)
Tests anti-cheat validation, performance, and concurrency safety.

**Tests**:
4. `test_anti_cheat_validation` - 100% invalid actions rejected (cooldowns, LOS, resources)
5. `test_validation_performance` - >100k checks/sec (benchmark ValidationContext)
6. `test_concurrency_safety` - Thread-safe cooldown/resource tracking

**Success Criteria**:
- ✅ 100% of cheating attempts rejected
- ✅ >100,000 validations/sec (10 µs per check)
- ✅ No race conditions under concurrent access

---

### **Phase 3: Planner Performance** (`planner_tests.rs`)
Tests RuleOrchestrator, GOAP, and Behavior Tree performance.

**Tests**:
7. `test_rule_orchestrator_correctness` - Validate smoke + advance tactical logic
8. `test_goap_scalability` - 676 agents planning in <10ms (cache validation)
9. `test_behavior_tree_scalability` - 66K agents possible (validate claim)
10. `test_planner_mode_switching` - Hot-swap between Rule/BT/GOAP without panics

**Success Criteria**:
- ✅ RuleOrchestrator produces expected plans (smoke + advance)
- ✅ 676 agents plan in <10ms average (with caching)
- ✅ Behavior Trees achieve 57-253 ns/agent (66K agents @ 60 FPS)
- ✅ Mode switching completes without errors

---

### **Phase 4: Integrated AI Loop** (`integration_tests.rs`)
Tests full Perception → Planning → Action loop under realistic conditions.

**Tests**:
11. `test_full_ai_loop_60fps` - 676 agents, full loop, <16.67ms per frame
12. `test_boss_ai_stress` - Single agent with complex GOAP under combat load
13. `test_multi_agent_coordination` - Squad tactics (covering fire, retreat, etc.)

**Success Criteria**:
- ✅ 95% of frames complete within 16.67ms budget (60 FPS)
- ✅ Boss AI maintains <10ms planning time under stress
- ✅ Squad coordination produces valid tactical plans

---

### **Phase 5: Determinism & Stability** (`determinism_tests.rs`)
Tests replay determinism and long-term stability.

**Tests**:
14. `test_deterministic_planning` - Hash-based replay verification (100% match)
15. `test_memory_stability` - 1-hour marathon test (no leaks, no OOM)

**Success Criteria**:
- ✅ 100% hash match across replays (deterministic planning)
- ✅ Stable memory usage over 1-hour run (<5% growth)
- ✅ No panics, no crashes, no resource exhaustion

---

## Running Tests

### Run All AI-Native Tests
```powershell
cargo test -p astraweave-ai --test perception_tests
cargo test -p astraweave-ai --test tool_validation_tests
cargo test -p astraweave-ai --test planner_tests
cargo test -p astraweave-ai --test integration_tests
cargo test -p astraweave-ai --test determinism_tests
```

### Run Specific Test
```powershell
cargo test -p astraweave-ai --test planner_tests test_goap_scalability -- --nocapture
```

### Run with Release Optimizations (for performance tests)
```powershell
cargo test -p astraweave-ai --release --test integration_tests test_full_ai_loop_60fps
```

### Run Long-Duration Tests
```powershell
# 1-hour marathon test (warning: takes 1 hour!)
cargo test -p astraweave-ai --release --test determinism_tests test_memory_stability -- --nocapture --ignored
```

---

## Expected Timings

| Test File | Tests | Est. Duration | Notes |
|-----------|-------|---------------|-------|
| `perception_tests.rs` | 3 | ~2 minutes | Includes 1000-frame stress test |
| `tool_validation_tests.rs` | 3 | ~30 seconds | Fast validation checks |
| `planner_tests.rs` | 4 | ~1 minute | Includes 676-agent benchmark |
| `integration_tests.rs` | 3 | ~5 minutes | Full AI loop validation |
| `determinism_tests.rs` | 2 | ~10 seconds* | *Marathon test is 1 hour (ignored) |

**Total**: ~9 minutes for standard tests, +1 hour for marathon test

---

## Performance Targets (from BASELINE_METRICS.md)

### Current Baselines
- **AI Core Loop**: 2.10 µs (full loop) = **7,936 agents @ 60 FPS**
- **Behavior Trees**: 57-253 ns/agent = **66,000 agents @ 60 FPS**
- **GOAP Planning**: 5.4-31.7 µs per plan (with caching)
- **ECS Tick**: <1 ns/entity for 50-entity world

### Test Targets (Conservative)
- **Perception**: 1000 agents in <5ms (5 µs/agent)
- **Tool Validation**: >100k checks/sec (10 µs/check)
- **GOAP at Scale**: 676 agents in <10ms average (14.8 µs/agent with caching)
- **Full AI Loop**: 676 agents in <16.67ms (24.6 µs/agent)
- **Boss AI**: Single complex agent in <10ms

---

## CI Integration

Add to `.github/workflows/ci.yml`:
```yaml
- name: AI-Native Validation Tests
  run: |
    cargo test -p astraweave-ai --test perception_tests --release
    cargo test -p astraweave-ai --test tool_validation_tests --release
    cargo test -p astraweave-ai --test planner_tests --release
    cargo test -p astraweave-ai --test integration_tests --release
    cargo test -p astraweave-ai --test determinism_tests --release
```

---

## Implementation Status

- ✅ Test structure created (this README)
- ⏳ Phase 1: Perception tests (in progress)
- ⏳ Phase 2: Tool validation tests (pending)
- ⏳ Phase 3: Planner tests (pending)
- ⏳ Phase 4: Integration tests (pending)
- ⏳ Phase 5: Determinism tests (pending)

**Next**: Implement `perception_tests.rs`

---

**Version**: 1.0.0  
**Author**: AstraWeave AI (GitHub Copilot)  
**License**: MIT
