# Test Coverage Gap Analysis & Implementation Plan
**Date**: October 20, 2025  
**Baseline Complete**: 10 crates analyzed  
**Average Coverage**: 17.88% (3,387/18,184 lines)  
**Target**: 90-95% for critical crates

---

## Executive Summary

Baseline coverage analysis reveals **critical gaps** across all engine subsystems. The 17.88% average coverage is **72 percentage points below industry standards** for production game engines.

**Priority Classification**:
- **P0 (CRITICAL)**: 5 crates with <20% coverage - **AI/physics/nav failures imminent**
- **P1 (HIGH)**: 3 crates with 20-40% coverage - **Determinism/security risks**
- **P2 (MEDIUM)**: 2 crates with >40% coverage - **Incremental improvements**

**Estimated Effort**: 55-73 hours (7-9 working days) to reach 90% targets

---

## Detailed Gap Analysis by Priority

### 🔴 P0: CRITICAL RISKS (0-20% Coverage)

#### 1. astraweave-audio (1.76% coverage - 34/1,930 lines)

**Current State**: **WORST COVERAGE** - Spatial audio completely untested

**Critical Gaps**:
- **Spatial audio system**: 0% coverage (position-based attenuation, 3D audio)
- **Audio mixer**: 0% coverage (4-bus system, crossfading, volume control)
- **Audio occlusion**: 0% coverage (raycast-based audio blocking)
- **Reverb zones**: 0% coverage (environmental audio effects)
- **Audio streaming**: 0% coverage (async audio loading, memory management)

**Risk Assessment**:
- **Impact**: Audio bugs cause immersion breaks, crashes, memory leaks
- **Likelihood**: HIGH - Complex 3D audio math untested
- **Severity**: P0 - Audio is user-facing, bugs are immediately noticeable

**Test Requirements**:
- ✅ Basic playback (play, pause, stop, volume)
- ✅ Spatial attenuation (distance falloff, listener position)
- ✅ Mixer functionality (bus routing, crossfades, master volume)
- ✅ Occlusion tests (raycast blocking, material absorption)
- ✅ Reverb zones (room size, material properties, wet/dry mix)
- ✅ Stress tests (100+ simultaneous sounds, memory limits)

**Estimated Effort**: 10-12 hours (60-80 tests)

---

#### 2. astraweave-nav (5.27% coverage - 72/1,367 lines)

**Current State**: **CRITICAL GAP** - Pathfinding algorithms completely untested

**Critical Gaps**:
- **A* pathfinding**: 0% coverage (optimal path generation, heuristics)
- **Navmesh generation**: 0% coverage (walkable surface detection, triangulation)
- **Portal graphs**: 0% coverage (inter-region pathfinding, door logic)
- **Dynamic obstacles**: 0% coverage (real-time path updates, avoidance)
- **NavQuery API**: 0% coverage (path requests, caching, fallbacks)

**Risk Assessment**:
- **Impact**: AI agents can't navigate, get stuck, crash game
- **Likelihood**: HIGH - Complex graph algorithms, edge cases
- **Severity**: P0 - Navigation is **AI-critical**, engine unusable without it

**Test Requirements**:
- ✅ A* correctness (optimal paths, no infinite loops)
- ✅ Navmesh generation (various terrain types, holes, slopes)
- ✅ Portal graphs (multi-room navigation, door states)
- ✅ Dynamic obstacles (moving entities, path recalculation)
- ✅ Edge cases (unreachable targets, narrow passages, cliffs)
- ✅ Performance (1,000+ path requests/sec, caching effectiveness)

**Estimated Effort**: 8-10 hours (60-80 tests)

---

####3. astraweave-physics (11.17% coverage - 161/1,441 lines)

**Current State**: **CRITICAL GAP** - Collision detection untested

**Critical Gaps**:
- **Spatial hash** (`spatial_hash.rs`): 0/59 lines (0%) - **Week 8 optimization untested!**
- **Character controller** (`character_controller.rs`): 0% - Movement, jumping, ground detection
- **Raycast** (`raycast.rs`): 0% - Line of sight, attack sweep, terrain queries
- **Rigid body** (`rigid_body.rs`): 0% - Physics simulation, forces, collisions
- **Async scheduler** (`async_scheduler.rs`): 0/8 lines (0%) - Concurrent physics
- **Deterministic RNG** (`rng.rs`): 0/15 lines (0%) - **Multiplayer-critical!**

**Risk Assessment**:
- **Impact**: Collision bugs, determinism breaks, multiplayer desyncs
- **Likelihood**: HIGH - Complex collision math, floating-point precision
- **Severity**: P0 - Physics is **gameplay-critical** and **determinism-sensitive**

**Test Requirements**:
- ✅ Spatial hash (grid insertion, query, 99.96% collision reduction validation)
- ✅ Character controller (movement, jumping, slope handling, edge cases)
- ✅ Raycast (accuracy, edge cases, performance)
- ✅ Rigid body (forces, collisions, stacking)
- ✅ Determinism (fixed RNG, reproducible results, bit-identical across runs)
- ✅ Async scheduler (concurrent physics, race conditions)

**Estimated Effort**: 10-12 hours (80-100 tests)

---

#### 4. astraweave-behavior (12.62% coverage - 215/1,703 lines)

**Current State**: **SIGNIFICANT GAPS** - GOAP/BT partially tested

**Critical Gaps**:
- **GOAP planner**: ~30% coverage (goal selection, action chaining, cost calculation)
- **Behavior trees**: ~40% coverage (node execution, state management, tick logic)
- **Utility AI**: 0% coverage (scoring, action selection, curves)
- **Action preconditions**: 0% coverage (world state validation, failure handling)
- **Goal prioritization**: 0% coverage (dynamic goal weights, re-planning)

**Risk Assessment**:
- **Impact**: AI agents make poor decisions, get stuck, crash
- **Likelihood**: HIGH - Complex state machines, edge cases
- **Severity**: P0 - Behavior is **AI-critical**, drives agent intelligence

**Test Requirements**:
- ✅ GOAP planning (optimal action sequences, cost minimization)
- ✅ Behavior tree execution (node types, success/failure propagation)
- ✅ Utility AI (scoring functions, action selection, tie-breaking)
- ✅ Preconditions (validation, early exit, error handling)
- ✅ Goal prioritization (dynamic weights, interrupts, replanning)

**Estimated Effort**: 8-10 hours (60-80 tests)

---

#### 5. astraweave-math (13.24% coverage - 189/1,428 lines)

**Current State**: **SIMD untested** - Week 8 optimizations at risk

**Critical Gaps**:
- **SIMD movement** (`simd_movement.rs`): 0% - **2.08× speedup untested!**
- **SIMD vector ops** (`simd_vec.rs`): ~20% - Batch processing, auto-vectorization
- **SIMD matrix ops** (`simd_mat.rs`): ~10% - Transformations, projections
- **SIMD quaternions** (`simd_quat.rs`): ~5% - Rotations, slerp

**Risk Assessment**:
- **Impact**: Performance regressions, SIMD bugs, numerical errors
- **Likelihood**: MEDIUM - glam handles most SIMD, but custom code at risk
- **Severity**: P1 - Math is **performance-critical**, errors cascade

**Test Requirements**:
- ✅ SIMD movement (batch processing, correctness, 2.08× speedup validation)
- ✅ SIMD vector ops (add, mul, dot, cross, normalization)
- ✅ SIMD matrix ops (transformations, inverse, determinant)
- ✅ SIMD quaternions (multiply, slerp, axis-angle)
- ✅ Numerical accuracy (floating-point precision, edge cases)

**Estimated Effort**: 6-8 hours (50-70 tests)

---

### ⚠️  P1: HIGH RISKS (20-40% Coverage)

#### 6. astraweave-gameplay (18.05% coverage - 448/2,482 lines)

**Current State**: **COMBAT PHYSICS PARTIALLY TESTED**

**Critical Gaps**:
- **Attack sweep** (`combat_physics.rs`): ~30% - Raycast cone, parry, iframes
- **Damage calculation**: 0% - Critical hits, armor, resistances
- **Status effects**: 0% - Buffs, debuffs, stacking logic
- **Combat events**: 0% - Hit registration, feedback, sound triggers

**Risk Assessment**:
- **Impact**: Combat feels broken, unfair, unresponsive
- **Likelihood**: MEDIUM - Combat physics tested in Week 1, but gaps remain
- **Severity**: P1 - Combat is **gameplay-critical**, bugs ruin player experience

**Test Requirements**:
- ✅ Attack sweep (cone detection, parry mechanics, iframes)
- ✅ Damage calculation (crits, armor, resistances, edge cases)
- ✅ Status effects (application, stacking, expiration, cleanse)
- ✅ Combat events (hit feedback, sound triggers, particle effects)

**Estimated Effort**: 6-8 hours (50-70 tests)

---

#### 7. astraweave-ai (31.96% coverage - 789/2,469 lines)

**Current State**: **ORCHESTRATOR UNTESTED** - Core AI loop at risk

**Critical Gaps**:
- **Orchestrator** (`orchestrator.rs`): 0/27 lines (0%) - **AI planning core!**
- **Async task** (`async_task.rs`): 0/51 lines (0%) - LLM integration
- **Tool sandbox** (`tool_sandbox.rs`): 0/8 lines (0%) - Security critical
- **AI core loop** (`core_loop.rs`): ~40% - Perception → Planning → Action
- **WorldSnapshot** (`perception.rs`): 0/24 lines (0%) - AI input

**Risk Assessment**:
- **Impact**: AI agents non-functional, planning failures, security bypasses
- **Likelihood**: HIGH - Complex async code, LLM integration, security
- **Severity**: P0 - AI orchestration is **engine-critical**, AstraWeave's core value prop

**Test Requirements**:
- ✅ Orchestrator (plan generation, tool selection, validation)
- ✅ Async tasks (LLM requests, timeout handling, cancellation)
- ✅ Tool sandbox (permission checks, resource limits, injection attacks)
- ✅ AI core loop (full pipeline, perception → action, error handling)
- ✅ WorldSnapshot (filtering, caching, correctness)

**Estimated Effort**: 10-12 hours (80-100 tests)

---

#### 8. astraweave-ecs (31.48% coverage - 514/1,633 lines)

**Current State**: **CORE SYSTEMS UNTESTED** - Determinism at risk

**Critical Gaps**:
- **blob_vec.rs**: 0/67 lines (0%) - Component storage, memory layout
- **command_buffer.rs**: 0/17 lines (0%) - Deferred operations, ordering
- **events.rs**: 0/54 lines (0%) - Event dispatch, ordering, cleanup
- **rng.rs**: 0/15 lines (0%) - **Deterministic RNG critical for multiplayer!**
- **system_param.rs**: 10/74 lines (13.51%) - Queries, resource access

**Risk Assessment**:
- **Impact**: Entity lifecycle bugs, memory corruption, determinism breaks
- **Likelihood**: HIGH - Core ECS infrastructure, complex invariants
- **Severity**: P0 - ECS is **engine foundation**, bugs cascade everywhere

**Test Requirements**:
- ✅ blob_vec (insert, remove, iteration, memory safety)
- ✅ command_buffer (deferred ops, ordering, correctness)
- ✅ events (dispatch, reader iteration, cleanup)
- ✅ rng (fixed seed, reproducibility, bit-identical results)
- ✅ system_param (queries, mutable/immutable access, conflicts)

**Estimated Effort**: 6-8 hours (40-50 tests)

---

### ✅ P2: MEDIUM RISKS (40%+ Coverage)

#### 9. astraweave-core (35.34% coverage - 965/2,731 lines)

**Current State**: **TOOL VOCABULARY EXCELLENT** (99.8%), other gaps

**Strengths**:
- ✅ Tool vocabulary (99.8% coverage) - Schema generation, tool metadata
- ✅ ECS adapter (85.2% coverage) - Integration with ECS systems
- ✅ Capture/replay (83.3% coverage) - Deterministic replay

**Critical Gaps**:
- **Perception** (`perception.rs`): 0/24 lines (0%) - WorldSnapshot building
- **Schema** (`schema.rs`): 0/19 lines (0%) - JSON schema validation
- **Tools** (`tools.rs`): 30/121 lines (24.8%) - Tool implementations
- **Validation** (`validation.rs`): 30/197 lines (15.2%) - Action validation

**Risk Assessment**:
- **Impact**: AI input/output bugs, validation bypasses, tool failures
- **Likelihood**: MEDIUM - Some coverage exists, but critical paths untested
- **Severity**: P1 - Core systems affect AI reliability

**Test Requirements**:
- ✅ Perception (WorldSnapshot correctness, filtering, edge cases)
- ✅ Schema (JSON validation, error handling, versioning)
- ✅ Tools (execution, effects, rollback, error handling)
- ✅ Validation (rule checking, preconditions, security)

**Estimated Effort**: 8-10 hours (70-90 tests)

---

## Implementation Plan

### Week 1: P0 Critical Crates (Days 1-5)

**Goal**: Fix most dangerous gaps (audio, nav, physics, behavior, math)

**Day 1-2: Audio + Navigation** (18-22 hours total)
- **Audio** (10-12h): Spatial audio, mixer, occlusion, reverb, stress tests
- **Navigation** (8-10h): A*, navmesh, portals, obstacles, edge cases

**Day 3-4: Physics + Behavior** (18-22 hours total)
- **Physics** (10-12h): Spatial hash, character controller, raycast, determinism
- **Behavior** (8-10h): GOAP, behavior trees, utility AI, preconditions

**Day 5: Math** (6-8 hours)
- **Math**: SIMD movement, vector/matrix ops, numerical accuracy

**Week 1 Target**: All P0 crates at 85-90% coverage

---

### Week 2: P1 High-Priority Crates (Days 6-9)

**Goal**: Fix high-risk gaps (gameplay, AI, ECS, core)

**Day 6: Gameplay + AI** (16-20 hours total)
- **Gameplay** (6-8h): Combat physics, damage calc, status effects
- **AI** (10-12h): Orchestrator, async tasks, tool sandbox, core loop

**Day 7: ECS + Core** (14-18 hours total)
- **ECS** (6-8h): blob_vec, command_buffer, events, rng, system_param
- **Core** (8-10h): Perception, schema, tools, validation

**Week 2 Target**: All P1 crates at 80-85% coverage

---

### Week 3 (Optional): Refinement & Integration (Days 10-12)

**Goal**: Polish, integration tests, CI setup

**Day 10: Integration Tests** (6-8 hours)
- Full pipeline tests (ECS → Perception → Planning → Physics → ECS)
- Multi-agent scenarios (100+ agents, complex interactions)
- Determinism validation (3+ runs, bit-identical results)

**Day 11: CI Integration** (4-6 hours)
- Add tarpaulin to GitHub Actions
- Coverage trending (track over time)
- Badge for README (90%+ coverage)

**Day 12: Documentation** (4-6 hours)
- TESTING_STRATEGIES.md (patterns, best practices)
- Update developer guides (how to run tests, add new tests)
- Maintenance procedures (coverage reviews, test updates)

**Week 3 Target**: 90-95% coverage across all critical crates, CI integrated

---

## Test Writing Patterns

### Pattern 1: Unit Tests (Module-Level)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_hash_insert() {
        let mut hash = SpatialHash::new(10.0);
        let entity = Entity::from_raw(1);
        let pos = Vec3::new(5.0, 0.0, 5.0);
        
        hash.insert(entity, pos);
        
        let results = hash.query_radius(pos, 5.0);
        assert!(results.contains(&entity));
    }

    #[test]
    fn test_spatial_hash_collision_reduction() {
        // Validate Week 8 optimization: 99.96% collision reduction
        let mut hash = SpatialHash::new(10.0);
        for i in 0..1000 {
            hash.insert(Entity::from_raw(i), Vec3::new(i as f32, 0.0, 0.0));
        }
        
        let checks_before = 1000 * 999 / 2; // O(n^2) = 499,500
        let checks_after = hash.collision_check_count();
        let reduction = 1.0 - (checks_after as f32 / checks_before as f32);
        
        assert!(reduction >= 0.9996, "Expected 99.96% reduction, got {:.2}%", reduction * 100.0);
    }
}
```

### Pattern 2: Integration Tests (Cross-Module)

```rust
// tests/integration_tests.rs
#[test]
fn test_ai_core_loop_full_pipeline() {
    let mut world = World::new();
    let mut ai = AIOrchestrator::new();
    
    // Setup: Create agent + obstacles
    let agent = world.spawn((Position(Vec3::ZERO), AIAgent::default()));
    let obstacle = world.spawn((Position(Vec3::new(5.0, 0.0, 0.0)), Obstacle));
    
    // Perception: Build WorldSnapshot
    let snap = build_world_snapshot(&world, agent);
    assert_eq!(snap.obstacles.len(), 1);
    
    // Planning: Generate PlanIntent
    let plan = ai.plan(&world, &snap).unwrap();
    assert!(!plan.steps.is_empty());
    
    // Physics: Apply actions
    apply_plan_to_world(&mut world, agent, &plan);
    
    // Validation: Check movement
    let new_pos = world.get::<Position>(agent).unwrap();
    assert_ne!(new_pos.0, Vec3::ZERO, "Agent should have moved");
}
```

### Pattern 3: Determinism Tests

```rust
#[test]
fn test_physics_determinism() {
    let mut rng1 = FixedRng::seed_from_u64(12345);
    let mut rng2 = FixedRng::seed_from_u64(12345);
    
    // Run 1
    let mut world1 = World::new();
    let entity1 = world1.spawn((Position(Vec3::ZERO), Velocity(Vec3::X)));
    for _ in 0..100 {
        physics_tick(&mut world1, &mut rng1, 1.0/60.0);
    }
    let pos1 = world1.get::<Position>(entity1).unwrap().0;
    
    // Run 2
    let mut world2 = World::new();
    let entity2 = world2.spawn((Position(Vec3::ZERO), Velocity(Vec3::X)));
    for _ in 0..100 {
        physics_tick(&mut world2, &mut rng2, 1.0/60.0);
    }
    let pos2 = world2.get::<Position>(entity2).unwrap().0;
    
    // Bit-identical results
    assert_eq!(pos1, pos2, "Physics must be deterministic");
}
```

### Pattern 4: Property-Based Tests (Fuzzing)

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_astar_correctness(
        start in any::<(i32, i32)>(),
        goal in any::<(i32, i32)>(),
    ) {
        let navmesh = Navmesh::new();
        let path = astar(&navmesh, start, goal);
        
        if let Some(p) = path {
            // Path must start at start
            assert_eq!(p.first(), Some(&start));
            // Path must end at goal
            assert_eq!(p.last(), Some(&goal));
            // Path must be continuous (no jumps)
            for window in p.windows(2) {
                let dist = manhattan_distance(window[0], window[1]);
                assert!(dist <= 1, "Path has gap: {:?}", window);
            }
        }
    }
}
```

---

## Success Criteria

### Minimum Acceptable (Week 1 Complete)
- ✅ **P0 crates**: 85-90% coverage (audio, nav, physics, behavior, math)
- ✅ **Critical paths tested**: Spatial hash, A*, GOAP, SIMD movement
- ✅ **Zero regression**: All existing tests still passing
- ✅ **Determinism validated**: Fixed RNG, reproducible results

### Target (Week 2 Complete)
- ✅ **All critical crates**: 90-95% coverage (P0 + P1)
- ✅ **Integration tests**: Full AI pipeline validated
- ✅ **Performance validated**: Benchmarks confirm Week 8 optimizations
- ✅ **Documentation**: TESTING_STRATEGIES.md complete

### Industry-Leading (Week 3 Complete)
- ✅ **All crates**: 90%+ coverage (including examples)
- ✅ **CI integrated**: Tarpaulin in GitHub Actions, coverage trending
- ✅ **Coverage badge**: README shows 90%+ badge
- ✅ **Maintenance plan**: Regular reviews, test updates

---

## Risk Mitigation

### Risk 1: Test Writing Takes Longer Than Estimated
**Mitigation**: Prioritize P0 crates first. If Week 1 runs over, defer P1 crates to Week 2. Minimum viable is 85% P0 coverage.

### Risk 2: Tests Reveal Critical Bugs
**Mitigation**: Expected! Fix bugs as discovered. Budget 20% extra time for bug fixes (~10-15 hours).

### Risk 3: Compilation Issues Block Coverage
**Mitigation**: Use `--lib` flag to skip integration tests if needed. Focus on library code first.

### Risk 4: Tarpaulin Performance Issues
**Mitigation**: Run per-crate coverage (faster). Use `--timeout 600` for slow crates. Consider cargo-llvm-cov as backup.

---

## Next Actions

**Immediate** (Today):
1. ✅ Review this gap analysis with stakeholders
2. ⏳ **Start Week 1 Day 1**: Audio + Navigation tests
3. ⏳ Create test harness for spatial audio (position-based attenuation)
4. ⏳ Add A* pathfinding tests (optimal paths, edge cases)

**This Week**:
5. ⏳ Complete Week 1 (P0 crates to 85-90%)
6. ⏳ Generate daily coverage reports (track progress)
7. ⏳ Document patterns in TESTING_STRATEGIES.md
8. ⏳ Celebrate reaching 50% average coverage milestone!

---

*Analysis Complete*: October 20, 2025  
*Baseline Data*: 10 crates, 18,184 lines analyzed  
*Next Step*: Begin Week 1 Day 1 (Audio + Navigation tests)  
*Target Completion*: 7-9 working days to 90% coverage
