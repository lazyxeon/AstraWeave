# Test Coverage Baseline Analysis - October 20, 2025

**Objective**: Establish comprehensive test coverage exceeding industry standards (target: 90-95% for critical crates)  
**Tool**: cargo-tarpaulin 0.33.0  
**Date**: October 20, 2025  
**Status**: ⚠️  **BASELINE ESTABLISHED** - Significant gaps identified

---

## Executive Summary

Baseline coverage analysis reveals **significant testing gaps** across core engine crates:

- **astraweave-core**: 35.34% (965/2,731 lines) - ⚠️ BELOW TARGET
- **astraweave-ai**: 31.96% (789/2,469 lines) - ⚠️ BELOW TARGET
- **astraweave-physics**: 11.17% (161/1,441 lines) - 🔴 CRITICAL GAP
- **astraweave-nav**: 5.27% (72/1,367 lines) - 🔴 CRITICAL GAP

**Overall**: ~23% average coverage (1,987/8,008 lines across 4 critical crates)

**Industry Benchmarks**:
- Mature projects: 80-90% coverage
- Critical systems: 90-95% coverage
- Safety-critical: 95%+ coverage

**AstraWeave Target**: 90-95% for critical crates (AI-native, deterministic game engine)

**Gap**: **-67 to -72 percentage points** below target

---

## Detailed Baseline Metrics

### astraweave-core (35.34% coverage)

**Total**: 965/2,731 lines covered

**Well-Tested Modules** (>50% coverage):
- ✅ `tool_vocabulary.rs`: 573/574 (99.83%) - **EXCELLENT**
- ✅ `ecs_adapter.rs`: 46/54 (85.19%) - **GOOD**
- ✅ `world.rs`: 40/50 (80.00%) - **GOOD**
- ✅ `validation.rs`: 30/197 (15.23%) - ⚠️ **NEEDS WORK**
- ✅ `tools.rs`: 30/121 (24.79%) - ⚠️ **NEEDS WORK**

**Critical Gaps** (0% coverage):
- 🔴 `orchestrator.rs`: 0/27 (0%) - **AI PLANNING CORE**
- 🔴 `tool_sandbox.rs`: 0/8 (0%) in ai crate - **SECURITY CRITICAL**
- 🔴 `perception.rs`: 0/24 (0%) - **AI INPUT**
- 🔴 `schema.rs`: 0/19 (0%) - **DATA STRUCTURES**
- 🔴 `async_task.rs`: 0/51 (0%) in ai crate - **CONCURRENCY**

**Medium Gaps** (10-50% coverage):
- ⚠️ `capture_replay.rs`: 20/24 (83.33%) - GOOD
- ⚠️ `ecs_components.rs`: 4/10 (40.00%) - NEEDS IMPROVEMENT
- ⚠️ `ecs_bridge.rs`: 5/31 (16.13%) - NEEDS IMPROVEMENT
- ⚠️ `ecs_events.rs`: 10/16 (62.50%) - GOOD

---

### astraweave-ai (31.96% coverage)

**Total**: 789/2,469 lines covered

**Critical Gaps** (AI planning, orchestration):
- 🔴 `orchestrator.rs`: 0/27 (0%) - **CORE AI LOGIC**
- 🔴 `async_task.rs`: 0/51 (0%) - **LLM INTEGRATION**
- 🔴 `tool_sandbox.rs`: 0/8 (0%) - **SECURITY SANDBOX**

**Note**: Most astraweave-ai code is in other crates. Need to analyze:
- `astraweave-behavior` (GOAP, behavior trees)
- `astraweave-llm` (LLM integration)
- `astraweave-coordination` (multi-agent)

---

### astraweave-physics (11.17% coverage)

**Total**: 161/1,441 lines covered

**Critical Gaps**:
- 🔴 `spatial_hash.rs`: 0/59 (0%) - **COLLISION OPTIMIZATION**
- 🔴 `async_scheduler.rs`: 0/8 (0%) - **ASYNC PHYSICS**
- 🔴 All character controller code: **UNTESTED**
- 🔴 All rigid body code: **UNTESTED**
- 🔴 All raycast code: **UNTESTED**

**Impact**: Physics is **gameplay-critical** and **determinism-sensitive**. 11% coverage is unacceptable.

---

### astraweave-nav (5.27% coverage)

**Total**: 72/1,367 lines covered

**Critical Gaps**:
- 🔴 Pathfinding (A*): **UNTESTED**
- 🔴 Navmesh generation: **UNTESTED**
- 🔴 Portal graphs: **UNTESTED**
- 🔴 Dynamic obstacle avoidance: **UNTESTED**

**Impact**: Navigation is **AI-critical**. 5% coverage means AI agents can't be trusted to move correctly.

---

### astraweave-ecs (analyzed as part of core)

**Partial Coverage** (from astraweave-core run):
- `archetype.rs`: 59/88 (67.05%) - GOOD
- `lib.rs`: 82/156 (52.56%) - FAIR
- `entity_allocator.rs`: 20/64 (31.25%) - NEEDS WORK
- `sparse_set.rs`: 18/103 (17.48%) - NEEDS WORK
- `system_param.rs`: 10/74 (13.51%) - NEEDS WORK
- `blob_vec.rs`: 0/67 (0%) - **CRITICAL GAP**
- `command_buffer.rs`: 0/17 (0%) - **CRITICAL GAP**
- `events.rs`: 0/54 (0%) - **CRITICAL GAP**
- `rng.rs`: 0/15 (0%) - **DETERMINISM CRITICAL**

---

## Risk Assessment

### 🔴 CRITICAL RISKS (0-20% coverage)

**1. Physics Subsystem (11% coverage)**
- **Risk**: Collision bugs, determinism breaks, performance regressions
- **Impact**: Gameplay bugs, multiplayer desyncs, AI navigation failures
- **Priority**: **P0 - IMMEDIATE**

**2. Navigation Subsystem (5% coverage)**
- **Risk**: AI pathfinding failures, stuck agents, crashes
- **Impact**: AI agents unusable, game unplayable
- **Priority**: **P0 - IMMEDIATE**

**3. AI Orchestration (0% coverage)**
- **Risk**: AI planning failures, LLM integration bugs, tool sandbox bypasses
- **Impact**: AI agents non-functional, security vulnerabilities
- **Priority**: **P0 - IMMEDIATE**

**4. ECS Core Systems (0% for blob_vec, command_buffer, events, rng)**
- **Risk**: Memory corruption, entity lifecycle bugs, determinism breaks
- **Impact**: Engine crashes, save/load failures, multiplayer desyncs
- **Priority**: **P0 - IMMEDIATE**

### ⚠️  MEDIUM RISKS (20-50% coverage)

**5. ECS System Parameters (13% coverage)**
- **Risk**: Query bugs, component access issues
- **Impact**: System execution failures, performance issues
- **Priority**: **P1 - HIGH**

**6. Validation System (15% coverage)**
- **Risk**: Tool usage bugs, action validation bypasses
- **Impact**: AI cheating, game rule violations
- **Priority**: **P1 - HIGH**

### ✅ LOW RISKS (50%+ coverage)

**7. Tool Vocabulary (99% coverage)**
- **Status**: ✅ **EXCELLENT** - Well-tested, production-ready
- **Priority**: **P3 - MAINTAIN**

**8. ECS Adapter (85% coverage)**
- **Status**: ✅ **GOOD** - Integration tested, minor gaps
- **Priority**: **P3 - MAINTAIN**

---

## Coverage Targets by Crate

### Critical Crates (Target: 90-95%)

| Crate | Current | Target | Gap | Priority |
|-------|---------|--------|-----|----------|
| **astraweave-core** | 35% | 90% | -55% | P1 |
| **astraweave-ai** | 32% | 95% | -63% | P0 |
| **astraweave-physics** | 11% | 95% | -84% | P0 |
| **astraweave-nav** | 5% | 95% | -90% | P0 |
| **astraweave-ecs** | ~50% | 90% | -40% | P1 |
| **astraweave-behavior** | ??? | 90% | ??? | P0 |

### Supporting Crates (Target: 80-85%)

| Crate | Current | Target | Gap | Priority |
|-------|---------|--------|-----|----------|
| **astraweave-audio** | ??? | 80% | ??? | P2 |
| **astraweave-render** | ??? | 75% | ??? | P3 |
| **astraweave-scene** | ??? | 80% | ??? | P2 |
| **astraweave-terrain** | ??? | 75% | ??? | P3 |

---

## Estimated Effort

### High-Priority Crates (P0-P1)

**astraweave-physics** (11% → 95%):
- **Gap**: 1,280 lines untested
- **Estimated Tests Needed**: 80-100 tests
- **Time**: 8-10 hours
- **Focus**: Spatial hash, character controller, raycast, determinism

**astraweave-nav** (5% → 95%):
- **Gap**: 1,295 lines untested
- **Estimated Tests Needed**: 60-80 tests
- **Time**: 6-8 hours
- **Focus**: A* pathfinding, navmesh, portal graphs, obstacle avoidance

**astraweave-ai** (32% → 95%):
- **Gap**: 1,680 lines untested
- **Estimated Tests Needed**: 100-120 tests
- **Time**: 10-12 hours
- **Focus**: Orchestrator, async tasks, tool sandbox, behavior trees, GOAP

**astraweave-ecs** (50% → 90%):
- **Gap**: 400 lines untested (estimated)
- **Estimated Tests Needed**: 40-50 tests
- **Time**: 4-5 hours
- **Focus**: blob_vec, command_buffer, events, rng, system_param

**astraweave-core** (35% → 90%):
- **Gap**: 1,766 lines untested
- **Estimated Tests Needed**: 90-110 tests
- **Time**: 9-11 hours
- **Focus**: Perception, schema, tools, validation

**Total High-Priority**:
- **Lines**: ~5,421 untested lines
- **Tests**: 370-460 new tests
- **Time**: **37-46 hours** (5-6 working days)

### Medium-Priority Crates (P2)

**astraweave-audio**, **astraweave-scene**, **astraweave-asset**:
- **Time**: 10-15 hours (2 days)

### Low-Priority Crates (P3)

**astraweave-render**, **astraweave-terrain**, **UI crates**:
- **Time**: 8-12 hours (1-2 days)

**Grand Total**: **55-73 hours** (7-9 working days for full coverage)

---

## Implementation Plan

### Phase 1: Critical Systems (P0) - 3-4 days

**Day 1-2: Physics & Navigation** (14-18 hours)
1. ✅ Spatial hash tests (collision detection, 99.96% reduction validated)
2. ✅ Character controller tests (movement, jumping, ground detection)
3. ✅ Raycast tests (line of sight, attack sweep, terrain queries)
4. ✅ A* pathfinding tests (optimal paths, dynamic obstacles)
5. ✅ Navmesh tests (generation, portals, walkable surfaces)
6. ✅ Determinism tests (fixed RNG, reproducible results)

**Day 3-4: AI Orchestration & ECS** (14-17 hours)
7. ✅ Orchestrator tests (plan generation, tool selection, validation)
8. ✅ Async task tests (LLM requests, timeout handling, cancellation)
9. ✅ Tool sandbox tests (permission checks, resource limits, security)
10. ✅ Behavior tree tests (tick logic, node execution, state management)
11. ✅ GOAP tests (goal planning, action selection, preconditions)
12. ✅ ECS core tests (blob_vec, command_buffer, events, rng)

### Phase 2: High-Priority Systems (P1) - 2 days

**Day 5-6: Core Systems** (9-11 hours)
13. ✅ Perception tests (WorldSnapshot building, filtering, caching)
14. ✅ Schema tests (ActionStep serialization, JSON validation)
15. ✅ Tools tests (execution, validation, effects)
16. ✅ System param tests (queries, resource access, mutability)

### Phase 3: Supporting Systems (P2-P3) - 2-3 days

**Day 7-9: Audio, Scene, Render** (18-27 hours)
17. ✅ Audio tests (spatial audio, mixer, crossfades)
18. ✅ Scene tests (cell loading, streaming, async)
19. ✅ Render tests (materials, GPU skinning, instancing)

---

## Success Criteria

### Minimum Acceptable (Week 1)
- ✅ Physics: 90%+ coverage (spatial hash, character controller, raycast)
- ✅ Navigation: 90%+ coverage (A*, navmesh, portals)
- ✅ AI Core: 85%+ coverage (orchestrator, behavior trees, GOAP)
- ✅ ECS: 80%+ coverage (blob_vec, command_buffer, events)
- **Overall Critical Crates**: 85%+ average

### Target (Week 2)
- ✅ All critical crates: 90-95% coverage
- ✅ Supporting crates: 80-85% coverage
- ✅ Integration tests: Full pipeline validated
- ✅ CI integration: Tarpaulin in GitHub Actions
- **Overall**: 90%+ average across critical crates

### Industry-Leading (Week 3)
- ✅ All crates: 90%+ coverage
- ✅ Property-based tests: Fuzz high-risk code
- ✅ Mutation testing: Validate test quality
- ✅ Coverage trending: Track over time
- **Overall**: 95%+ average (safety-critical quality)

---

## Next Steps

### Immediate (Today)
1. ⏳ **Start Phase 1**: Physics & Navigation tests
2. ⏳ Create test harness for spatial hash (Week 8 implementation)
3. ⏳ Add character controller tests (movement, collision)
4. ⏳ Add A* pathfinding tests (optimal paths, obstacles)

### This Week
5. ⏳ Complete Phase 1 (physics, nav, AI, ECS)
6. ⏳ Generate coverage reports after each day
7. ⏳ Track progress toward 90% target
8. ⏳ Document patterns in TESTING_STRATEGIES.md

### Next Week
9. ⏳ Complete Phase 2 (core systems)
10. ⏳ Add Phase 3 (supporting systems)
11. ⏳ Integrate tarpaulin into CI/CD
12. ⏳ Create coverage badge for README

---

## Technical Notes

### Tarpaulin Limitations

**Known Issues**:
- ❌ Concurrency tests fail (World not `Send` due to TypeRegistry function pointers)
- ⚠️ Some async code hard to measure (futures, tokio tasks)
- ⚠️ GPU code can't be tested (rendering, compute shaders)

**Workarounds**:
- Exclude concurrency tests for now (fix `Send` issue separately)
- Test async code synchronously where possible
- Use integration tests for GPU pipelines (validate outputs, not internal code)

### Test Organization

**Current Structure**:
```
astraweave-physics/
├─ src/
│  ├─ lib.rs (no tests currently)
│  ├─ spatial_hash.rs (0/59 lines covered)
│  ├─ character_controller.rs (untested)
│  └─ raycast.rs (untested)
└─ tests/
   └─ (NONE - need to create)
```

**Target Structure**:
```
astraweave-physics/
├─ src/
│  ├─ lib.rs (unit tests for public API)
│  ├─ spatial_hash.rs (unit tests for grid logic)
│  ├─ character_controller.rs (unit tests for movement)
│  └─ raycast.rs (unit tests for collision)
└─ tests/
   ├─ integration_tests.rs (full pipeline)
   ├─ determinism_tests.rs (reproducibility)
   └─ benchmarks.rs (performance validation)
```

---

## Baseline Report Files

**HTML Reports Generated**:
- `coverage/baseline/core/tarpaulin-report.html` - astraweave-core (35.34%)
- `coverage/baseline/ai/tarpaulin-report.html` - astraweave-ai (31.96%)
- `coverage/baseline/physics/tarpaulin-report.html` - astraweave-physics (11.17%)
- `coverage/baseline/nav/tarpaulin-report.html` - astraweave-nav (5.27%)

**Command to View**:
```powershell
# Open HTML reports in browser
start coverage/baseline/core/tarpaulin-report.html
start coverage/baseline/physics/tarpaulin-report.html
```

---

## Conclusion

**Current State**: **23% average coverage** across critical crates (4/4 analyzed)

**Industry Benchmark**: 80-90% for mature projects, 90-95% for critical systems

**AstraWeave Target**: 90-95% (AI-native, deterministic game engine)

**Gap**: **-67 to -72 percentage points** below target

**Priority**: 🔴 **CRITICAL** - Physics (11%), Navigation (5%), AI (0% for orchestrator) are **unacceptably low**

**Recommendation**: **Start Phase 1 immediately** - Focus on physics, navigation, and AI orchestration (3-4 days of work)

**Expected Outcome**: 90%+ coverage for critical crates within 7-9 working days

---

*Report Generated*: October 20, 2025  
*Tool*: cargo-tarpaulin 0.33.0  
*Baseline Established*: YES ✅  
*Next Action*: Start Phase 1 (Physics & Navigation tests)
