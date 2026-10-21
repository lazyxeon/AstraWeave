# Zero Coverage Expansion Plan: Comprehensive Test Coverage Strategy

**Date**: October 18, 2025  
**Objective**: Add test coverage to 60+ files currently at 0% coverage  
**Scope**: ~1,500 lines across 15+ crates  
**Timeline**: 3-4 weeks (phased approach)

---

## 📊 Executive Summary

### Current State
- **Uncovered Files**: 60+ files with 0% coverage
- **Total Uncovered Lines**: ~1,500 lines (estimated)
- **Affected Crates**: 15+ core and supporting crates
- **Impact**: Critical engine functionality untested

### Strategic Approach
**4-Tier Prioritization** based on criticality and impact:
1. **Tier 1 (Week 1)**: Core ECS + AI (626 lines) - **CRITICAL**
2. **Tier 2 (Week 2)**: Math + Render + Physics (222 lines) - **HIGH**
3. **Tier 3 (Week 3+)**: LLM + Specialized (406 lines) - **MEDIUM**
4. **Tier 4 (Future)**: CLI + Networking (210 lines) - **LOW/DEFER**

### Success Metrics
- **Phase 1 Target**: 60-80% coverage for Tier 1 crates
- **Phase 2 Target**: 70-90% coverage for Tier 2 crates
- **Phase 3 Target**: 40-60% coverage for Tier 3 crates
- **Overall Goal**: 50%+ coverage for all core crates (Tier 1-2)

---

## 🎯 Tier 1: Critical Core Engine (Week 1) — HIGHEST PRIORITY

**Target**: 626 lines across 4 crates  
**Estimated Effort**: 30-40 hours (1 week @ 5-6 hours/day)  
**Velocity Assumption**: 15-20 lines/hour (higher for core modules with clear APIs)

### 1.1 astraweave-ecs (449 lines) — **START HERE**

**Why First**: Foundation of entire engine, used by all examples, high visibility

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `lib.rs` | 70 | P0 | 4 hours | 70-80% |
| `system_param.rs` | 74 | P0 | 4 hours | 70-80% |
| `events.rs` | 54 | P0 | 3 hours | 70-80% |
| `sparse_set.rs` | 61 | P1 | 3 hours | 60-70% |
| `blob_vec.rs` | 46 | P1 | 2.5 hours | 60-70% |
| `entity_allocator.rs` | 28 | P1 | 2 hours | 70-80% |
| `command_buffer.rs` | 17 | P2 | 1.5 hours | 60-70% |
| `archetype.rs` | 17 | P2 | 1.5 hours | 60-70% |
| `type_registry.rs` | 17 | P2 | 1.5 hours | 60-70% |
| `rng.rs` | 12 | P3 | 1 hour | 80-90% |
| **Total** | **449** | | **24-25 hours** | **70% avg** |

**Test Patterns**:
```rust
// Entity lifecycle tests
#[test]
fn test_entity_spawn_despawn_cycle() {
    let mut world = World::new();
    let entity = world.spawn().insert(Position { x: 0.0, y: 0.0 }).id();
    assert!(world.get::<Position>(entity).is_some());
    world.despawn(entity);
    assert!(world.get::<Position>(entity).is_none());
}

// System parameter tests
#[test]
fn test_query_iteration() {
    let mut world = World::new();
    world.spawn().insert(Position { x: 1.0, y: 2.0 });
    let mut query = world.query::<&Position>();
    let count = query.iter(&world).count();
    assert_eq!(count, 1);
}

// Event tests
#[test]
fn test_event_send_receive() {
    let mut world = World::new();
    let mut events = world.get_resource_mut::<Events<TestEvent>>().unwrap();
    events.send(TestEvent { value: 42 });
    let received = events.drain().collect::<Vec<_>>();
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].value, 42);
}
```

**Key Test Areas**:
1. **Entity Lifecycle**: spawn, despawn, hierarchy, generation
2. **Component Access**: insert, remove, query, mutation
3. **System Execution**: ordering, parameters, queries
4. **Event Flow**: send, receive, drain, ordering
5. **Archetypes**: creation, migration, query matching
6. **Sparse Sets**: insertion, removal, iteration
7. **Command Buffers**: deferred operations, ordering
8. **Type Registry**: registration, lookup, trait objects

### 1.2 astraweave-ai (86 lines)

**Why Important**: AI-native engine core, orchestration logic

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `async_task.rs` | 51 | P0 | 3 hours | 70-80% |
| `orchestrator.rs` | 27 | P0 | 2 hours | 70-80% |
| `tool_sandbox.rs` | 8 | P1 | 1 hour | 80-90% |
| **Total** | **86** | | **6 hours** | **75% avg** |

**Test Patterns**:
```rust
// Orchestrator tests
#[tokio::test]
async fn test_orchestrator_plan_generation() {
    let mut world = World::new();
    let snap = create_test_snapshot();
    let orchestrator = GOAPOrchestrator::new();
    let plan = orchestrator.plan(&mut world, &snap).await.unwrap();
    assert!(!plan.steps.is_empty());
    assert!(plan.steps[0].tool_name.len() > 0);
}

// Async task tests
#[tokio::test]
async fn test_async_task_completion() {
    let task = AsyncTask::new(async { Ok(42) });
    let result = task.await.unwrap();
    assert_eq!(result, 42);
}

// Tool sandbox tests
#[test]
fn test_tool_sandbox_validation() {
    let sandbox = ToolSandbox::new();
    let result = sandbox.validate_tool("MoveTo", &json!({"x": 10, "y": 20}));
    assert!(result.is_ok());
}
```

### 1.3 astraweave-core (16 lines)

**Why Important**: Core data structures shared across crates

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `ecs_events.rs` | 16 | P1 | 1.5 hours | 80-90% |
| **Total** | **16** | | **1.5 hours** | **85% avg** |

### 1.4 astraweave-behavior (8 lines)

**Why Important**: AI behavior systems (GOAP, behavior trees)

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `goap.rs` | 6 | P1 | 1 hour | 80-90% |
| `lib.rs` | 2 | P2 | 0.5 hours | 90-100% |
| **Total** | **8** | | **1.5 hours** | **85% avg** |

### 1.5 astraweave-physics (67 lines)

**Why Important**: Spatial hash optimization (Week 8), async scheduler

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `spatial_hash.rs` | 59 | P0 | 3 hours | 70-80% |
| `async_scheduler.rs` | 8 | P1 | 1 hour | 80-90% |
| **Total** | **67** | | **4 hours** | **75% avg** |

**Test Patterns**:
```rust
// Spatial hash tests
#[test]
fn test_spatial_hash_insertion_and_query() {
    let mut hash = SpatialHash::new(10.0); // 10x10 cell size
    hash.insert(EntityId(1), Vec2::new(5.0, 5.0));
    hash.insert(EntityId(2), Vec2::new(15.0, 15.0));
    
    let nearby = hash.query_radius(Vec2::new(6.0, 6.0), 5.0);
    assert!(nearby.contains(&EntityId(1)));
    assert!(!nearby.contains(&EntityId(2)));
}

// Async scheduler tests
#[tokio::test]
async fn test_async_physics_step() {
    let mut scheduler = AsyncPhysicsScheduler::new();
    scheduler.schedule_step(Duration::from_millis(16)).await;
    assert!(scheduler.last_step_duration() < Duration::from_millis(17));
}
```

**Tier 1 Summary**:
- **Total Lines**: 626 lines
- **Estimated Time**: 37-38 hours (1 week @ 5-6 hrs/day)
- **Target Coverage**: 70-80% average
- **Expected Coverage Gain**: ~470 lines covered

---

## 🎯 Tier 2: Important Subsystems (Week 2) — HIGH PRIORITY

**Target**: 222 lines across 5 crates  
**Estimated Effort**: 12-15 hours  
**Velocity Assumption**: 15-18 lines/hour

### 2.1 astraweave-math (84 lines)

**Why Important**: Performance-critical SIMD operations (Week 8 optimizations)

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `simd_vec.rs` | 49 | P0 | 3 hours | 80-90% |
| `simd_quat.rs` | 20 | P1 | 1.5 hours | 70-80% |
| `simd_mat.rs` | 15 | P1 | 1.5 hours | 70-80% |
| **Total** | **84** | | **6 hours** | **80% avg** |

**Test Patterns**:
```rust
// SIMD vector tests
#[test]
fn test_simd_vec_operations() {
    let v1 = SimdVec3::new(1.0, 2.0, 3.0);
    let v2 = SimdVec3::new(4.0, 5.0, 6.0);
    let sum = v1 + v2;
    assert_eq!(sum.x, 5.0);
    assert_eq!(sum.y, 7.0);
    assert_eq!(sum.z, 9.0);
}

// SIMD batch processing (Week 8 pattern)
#[test]
fn test_simd_batch_transform() {
    let positions = vec![Vec3::ZERO; 1000];
    let velocities = vec![Vec3::X; 1000];
    let dt = 0.016;
    
    let mut result = positions.clone();
    update_positions_simd(&mut result, &velocities, dt);
    
    assert!((result[0].x - 0.016).abs() < 1e-6);
}
```

### 2.2 astraweave-render (82 lines)

**Why Important**: Phase 8 game engine readiness (rendering completion)

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `graph.rs` | 36 | P0 | 2.5 hours | 60-70% |
| `renderer.rs` | 17 | P0 | 1.5 hours | 70-80% |
| `culling.rs` | 10 | P1 | 1 hour | 70-80% |
| `types.rs` | 9 | P1 | 1 hour | 80-90% |
| `overlay.rs` | 4 | P2 | 0.5 hours | 80-90% |
| `instancing.rs` | 3 | P2 | 0.5 hours | 90-100% |
| `culling_node.rs` | 3 | P2 | 0.5 hours | 90-100% |
| **Total** | **82** | | **7.5 hours** | **75% avg** |

### 2.3 astraweave-audio (40 lines)

**Why Important**: Phase 8 Priority 4 (production audio)

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `dialogue_runtime.rs` | 40 | P1 | 2.5 hours | 60-70% |
| **Total** | **40** | | **2.5 hours** | **65% avg** |

### 2.4 astraweave-scene (12 lines)

**Why Important**: World streaming, async cell loading

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `lib.rs` | 11 | P1 | 1 hour | 70-80% |
| `streaming.rs` | 1 | P2 | 0.5 hours | 100% |
| **Total** | **12** | | **1.5 hours** | **75% avg** |

### 2.5 astraweave-input (4 lines)

**Why Easy**: Small, clear API surface

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `manager.rs` | 4 | P2 | 0.5 hours | 90-100% |
| **Total** | **4** | | **0.5 hours** | **95% avg** |

**Tier 2 Summary**:
- **Total Lines**: 222 lines
- **Estimated Time**: 18 hours
- **Target Coverage**: 75-80% average
- **Expected Coverage Gain**: ~170 lines covered

---

## 🎯 Tier 3: Specialized Systems (Week 3+) — MEDIUM PRIORITY

**Target**: 406 lines across 8 crates  
**Estimated Effort**: 22-28 hours  
**Velocity Assumption**: 15-18 lines/hour

### 3.1 astraweave-llm (336 lines)

**Why Complex**: Many dependencies (Ollama, circuit breaker, retry logic)

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `production_hardening.rs` | 127 | P1 | 7 hours | 50-60% |
| `phi3.rs` | 43 | P1 | 2.5 hours | 60-70% |
| `cache/lru.rs` | 44 | P1 | 2.5 hours | 60-70% |
| `retry.rs` | 22 | P2 | 1.5 hours | 70-80% |
| `circuit_breaker.rs` | 21 | P2 | 1.5 hours | 70-80% |
| `tool_guard.rs` | 17 | P2 | 1 hour | 70-80% |
| `phi3_ollama.rs` | 7 | P2 | 1 hour | 80-90% |
| `hermes2pro_ollama.rs` | 7 | P2 | 1 hour | 80-90% |
| `cache/key.rs` | 5 | P3 | 0.5 hours | 80-90% |
| **Total** | **336** | | **18.5 hours** | **65% avg** |

**Challenge**: Requires Ollama mocking or integration tests

### 3.2 astraweave-asset (27 lines)

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `lib.rs` | 16 | P1 | 1.5 hours | 70-80% |
| `cell_loader.rs` | 11 | P1 | 1 hour | 70-80% |
| **Total** | **27** | | **2.5 hours** | **75% avg** |

### 3.3 astraweave-npc (16 lines)

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `runtime.rs` | 16 | P2 | 1.5 hours | 70-80% |
| **Total** | **16** | | **1.5 hours** | **75% avg** |

### 3.4 astraweave-pcg (12 lines)

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `seed_rng.rs` | 12 | P2 | 1 hour | 80-90% |
| **Total** | **12** | | **1 hour** | **85% avg** |

### 3.5 astraweave-asset-pipeline (10 lines)

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `validator.rs` | 10 | P2 | 1 hour | 80-90% |
| **Total** | **10** | | **1 hour** | **85% avg** |

### 3.6 astraweave-weaving (10 lines)

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `intents.rs` | 10 | P2 | 1 hour | 80-90% |
| **Total** | **10** | | **1 hour** | **85% avg** |

### 3.7 astraweave-rag (8 lines)

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `lib.rs` | 8 | P2 | 1 hour | 80-90% |
| **Total** | **8** | | **1 hour** | **85% avg** |

### 3.8 astraweave-memory (5 lines)

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `storage.rs` | 5 | P3 | 0.5 hours | 90-100% |
| **Total** | **5** | | **0.5 hours** | **95% avg** |

### 3.9 Small Modules (7 lines total)

| File | Lines | Priority | Estimated Time | Target Coverage |
|------|-------|----------|----------------|-----------------|
| `astraweave-embeddings/utils.rs` | 5 | P3 | 0.5 hours | 90-100% |
| `astraweave-gameplay/dialogue.rs` | 2 | P3 | 0.5 hours | 100% |
| `astraweave-ui/hud.rs` | 2 | P3 | 0.5 hours | 100% |
| `astraweave-gameplay/biome.rs` | 1 | P3 | 0.5 hours | 100% |
| `astraweave-prompts/compat.rs` | 2 | P3 | 0.5 hours | 100% |
| `astraweave-prompts/template.rs` | 5 | P3 | 0.5 hours | 90-100% |
| `astraweave-llm-eval/lib.rs` | 3 | P3 | 0.5 hours | 90-100% |
| **Total** | **20** | | **3.5 hours** | **95% avg** |

**Tier 3 Summary**:
- **Total Lines**: 444 lines
- **Estimated Time**: 29.5 hours
- **Target Coverage**: 70% average
- **Expected Coverage Gain**: ~310 lines covered

---

## 🎯 Tier 4: Deferred Systems (Future) — LOW PRIORITY

**Target**: 210 lines  
**Recommendation**: **DEFER** to future sprints

### 4.1 tools/astraweave-assets/main.rs (177 lines)

**Why Defer**: CLI tool, integration tests more appropriate, UI testing complex

### 4.2 Networking (18 lines)

| File | Lines | Reason to Defer |
|------|-------|-----------------|
| `net/aw-net-proto/lib.rs` | 10 | Networking not in Phase 8-9 roadmap |
| `net/aw-net-server/main.rs` | 8 | Server tests require integration setup |
| **Total** | **18** | **Phase 10 (optional)** |

### 4.3 Persistence (2 lines)

| File | Lines | Reason to Defer |
|------|-------|-----------------|
| `persistence/aw-save/lib.rs` | 2 | Save system is Phase 8 Priority 3, not yet implemented |
| **Total** | **2** | **Phase 8.3 (2-3 weeks out)** |

**Tier 4 Summary**: **DEFER** — Not critical for current Phase 8 roadmap

---

## 📅 Implementation Timeline

### Week 1: Tier 1 — Core Engine (Oct 18-25, 2025)
**Target**: 626 lines, 70-80% coverage

| Day | Focus | Lines | Hours | Deliverables |
|-----|-------|-------|-------|--------------|
| **Mon** | ECS: lib.rs, system_param.rs | 144 | 8h | Entity lifecycle, system param tests |
| **Tue** | ECS: events.rs, sparse_set.rs | 115 | 6h | Event flow, sparse set tests |
| **Wed** | ECS: blob_vec, entity_allocator | 74 | 4.5h | Blob vec, allocator tests |
| **Thu** | ECS: archetype, command_buffer, rng | 46 | 4.5h | Archetype, command buffer tests |
| **Fri** | AI: orchestrator, async_task, tool_sandbox | 86 | 6h | Orchestrator, async task tests |
| **Sat** | Physics: spatial_hash, async_scheduler | 67 | 4h | Spatial hash tests (Week 8 validation) |
| **Sun** | Core/Behavior: ecs_events, goap | 24 | 2.5h | Core events, GOAP tests |

**Week 1 Milestones**:
- ✅ 70%+ coverage for astraweave-ecs
- ✅ 75%+ coverage for astraweave-ai
- ✅ 75%+ coverage for astraweave-physics (spatial hash)
- ✅ 85%+ coverage for astraweave-core, astraweave-behavior

### Week 2: Tier 2 — Subsystems (Oct 25 - Nov 1, 2025)
**Target**: 222 lines, 75-80% coverage

| Day | Focus | Lines | Hours | Deliverables |
|-----|-------|-------|-------|--------------|
| **Mon** | Math: simd_vec.rs | 49 | 3h | SIMD vector tests (Week 8 patterns) |
| **Tue** | Math: simd_quat, simd_mat | 35 | 3h | SIMD quaternion, matrix tests |
| **Wed** | Render: graph.rs, renderer.rs | 53 | 4h | Render graph, renderer tests |
| **Thu** | Render: culling, types, overlay | 29 | 3h | Culling, render types tests |
| **Fri** | Audio: dialogue_runtime.rs | 40 | 2.5h | Dialogue runtime tests |
| **Sat** | Scene: lib, streaming | 12 | 1.5h | Scene streaming tests |
| **Sun** | Input: manager | 4 | 0.5h | Input manager tests |

**Week 2 Milestones**:
- ✅ 80%+ coverage for astraweave-math (SIMD)
- ✅ 75%+ coverage for astraweave-render
- ✅ 65%+ coverage for astraweave-audio
- ✅ 75%+ coverage for astraweave-scene
- ✅ 95%+ coverage for astraweave-input

### Week 3: Tier 3 — Specialized (Nov 1-8, 2025)
**Target**: 444 lines, 70% coverage

| Day | Focus | Lines | Hours | Deliverables |
|-----|-------|-------|-------|--------------|
| **Mon** | LLM: production_hardening (part 1) | 64 | 3.5h | Hardening tests (retry, timeout) |
| **Tue** | LLM: production_hardening (part 2) | 63 | 3.5h | Hardening tests (circuit breaker) |
| **Wed** | LLM: phi3, cache/lru | 87 | 5h | Phi-3, LRU cache tests |
| **Thu** | LLM: retry, circuit_breaker, tool_guard | 60 | 4h | Retry, CB, tool guard tests |
| **Fri** | LLM: ollama clients, cache/key | 19 | 2.5h | Ollama mocks, cache key tests |
| **Sat** | Asset: lib, cell_loader | 27 | 2.5h | Asset loading tests |
| **Sun** | Misc: npc, pcg, pipeline, weaving, rag, memory | 71 | 5h | Small module tests |

**Week 3 Milestones**:
- ✅ 65%+ coverage for astraweave-llm (complex mocking)
- ✅ 75%+ coverage for astraweave-asset
- ✅ 80%+ coverage for small modules (npc, pcg, pipeline, etc.)

### Week 4 (Optional): Polish & Edge Cases (Nov 8-15, 2025)
**Target**: Increase coverage to 80%+ on Tier 1-2

| Focus | Activities |
|-------|------------|
| **Coverage gaps** | Address remaining uncovered branches in Tier 1-2 |
| **Integration tests** | End-to-end tests for ECS + AI + Physics |
| **Documentation** | Update test coverage docs, celebrate milestones |
| **CI integration** | Add coverage thresholds to CI pipeline |

---

## 📊 Expected Outcomes

### Coverage Projections

**After Week 1 (Tier 1 Complete)**:
| Crate | Current | Target | Lines Covered |
|-------|---------|--------|---------------|
| astraweave-ecs | 0% (0/449) | 70% | ~314 lines |
| astraweave-ai | 0% (0/86) | 75% | ~65 lines |
| astraweave-physics | ~5% | 75% | ~50 lines |
| astraweave-core | 0% (0/16) | 85% | ~14 lines |
| astraweave-behavior | 0% (0/8) | 85% | ~7 lines |
| **Total** | **0/626** | **71%** | **~450 lines** |

**After Week 2 (Tier 2 Complete)**:
| Crate | Current | Target | Lines Covered |
|-------|---------|--------|---------------|
| astraweave-math | 0% (0/84) | 80% | ~67 lines |
| astraweave-render | ~2% | 75% | ~62 lines |
| astraweave-audio | 0% (0/40) | 65% | ~26 lines |
| astraweave-scene | 0% (0/12) | 75% | ~9 lines |
| astraweave-input | 0% (0/4) | 95% | ~4 lines |
| **Total** | **0/222** | **76%** | **~168 lines** |

**After Week 3 (Tier 3 Complete)**:
| Crate | Current | Target | Lines Covered |
|-------|---------|--------|---------------|
| astraweave-llm | 0% (0/336) | 65% | ~218 lines |
| astraweave-asset | 0% (0/27) | 75% | ~20 lines |
| Small modules | 0% (0/81) | 85% | ~69 lines |
| **Total** | **0/444** | **69%** | **~307 lines** |

**Overall After 3 Weeks**:
- **Total Lines Covered**: ~925 lines (out of 1,292 target)
- **Overall Coverage Gain**: From 0% → 72% average across Tier 1-3
- **Test Count**: +250-300 tests (estimated)

---

## 🎓 Testing Strategies by Crate Type

### Strategy 1: ECS & Core Systems
**Pattern**: Unit tests for isolated components, integration tests for system interactions

```rust
// Unit test: Component storage
#[test]
fn test_component_insert_remove() {
    let mut storage = ComponentStorage::<Position>::new();
    let entity = EntityId(42);
    storage.insert(entity, Position { x: 1.0, y: 2.0 });
    assert!(storage.get(entity).is_some());
    storage.remove(entity);
    assert!(storage.get(entity).is_none());
}

// Integration test: Full ECS cycle
#[test]
fn test_ecs_full_cycle() {
    let mut world = World::new();
    world.add_system(SystemStage::SIMULATION, movement_system);
    
    let entity = world.spawn()
        .insert(Position { x: 0.0, y: 0.0 })
        .insert(Velocity { x: 1.0, y: 0.0 })
        .id();
    
    world.tick(0.016);
    
    let pos = world.get::<Position>(entity).unwrap();
    assert!((pos.x - 0.016).abs() < 1e-6);
}
```

### Strategy 2: AI & Orchestration
**Pattern**: Mock WorldSnapshot, verify PlanIntent generation

```rust
#[tokio::test]
async fn test_goap_planner_generates_valid_plan() {
    let mut world = World::new();
    let snap = WorldSnapshot {
        t: 0.0,
        player: PlayerState { pos: IVec2::new(10, 10), health: 100 },
        me: CompanionState { pos: IVec2::new(5, 5), ammo: 10, morale: 0.8, cooldowns: BTreeMap::new() },
        enemies: vec![EnemyState { id: 1, pos: IVec2::new(8, 8), health: 50 }],
        pois: vec![],
        obstacles: vec![],
        objective: Some("defend".to_string()),
    };
    
    let planner = GOAPPlanner::new();
    let plan = planner.plan(&mut world, &snap).await.unwrap();
    
    assert!(!plan.steps.is_empty());
    assert_eq!(plan.steps[0].tool_name, "TakeCover");
}
```

### Strategy 3: Math & SIMD
**Pattern**: Property-based testing, batch processing validation

```rust
#[test]
fn test_simd_vec_addition_commutative() {
    use proptest::prelude::*;
    
    proptest!(|(x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32)| {
        let v1 = SimdVec3::new(x1, y1, z1);
        let v2 = SimdVec3::new(x2, y2, z2);
        
        let sum1 = v1 + v2;
        let sum2 = v2 + v1;
        
        prop_assert!((sum1.x - sum2.x).abs() < 1e-5);
        prop_assert!((sum1.y - sum2.y).abs() < 1e-5);
        prop_assert!((sum1.z - sum2.z).abs() < 1e-5);
    });
}
```

### Strategy 4: Rendering & Graphics
**Pattern**: Mock wgpu resources, test render graph construction

```rust
#[test]
fn test_render_graph_node_ordering() {
    let mut graph = RenderGraph::new();
    graph.add_node("shadow", shadow_pass_node());
    graph.add_node("main", main_pass_node());
    graph.add_edge("shadow", "main"); // shadow depends on main
    
    let order = graph.topological_sort().unwrap();
    assert_eq!(order[0], "shadow");
    assert_eq!(order[1], "main");
}
```

### Strategy 5: LLM & Async Systems
**Pattern**: Mock Ollama client, test error handling and retry logic

```rust
#[tokio::test]
async fn test_llm_retry_on_timeout() {
    let mut mock_client = MockOllamaClient::new();
    mock_client.expect_generate()
        .times(3)
        .returning(|_| Err(OllamaError::Timeout));
    
    let llm = Phi3Ollama::new_with_client(mock_client);
    let result = llm.generate_with_retry("test prompt", 3).await;
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().retry_count, 3);
}
```

---

## 🚀 Quick Start: Week 1 Day 1

### Immediate Action: ECS lib.rs Testing

**Step 1**: Read the ECS API (5 minutes)
```bash
cat crates/astraweave-ecs/src/lib.rs
```

**Step 2**: Create test file (2 minutes)
```bash
# File: crates/astraweave-ecs/tests/ecs_core_tests.rs
```

**Step 3**: Add basic entity lifecycle test (10 minutes)
```rust
use astraweave_ecs::*;

#[derive(Debug, Clone, Copy)]
struct Position { x: f32, y: f32 }

#[test]
fn test_entity_spawn_and_query() {
    let mut world = World::new();
    let entity = world.spawn().insert(Position { x: 1.0, y: 2.0 }).id();
    
    let pos = world.get::<Position>(entity).unwrap();
    assert_eq!(pos.x, 1.0);
    assert_eq!(pos.y, 2.0);
}

#[test]
fn test_entity_despawn() {
    let mut world = World::new();
    let entity = world.spawn().insert(Position { x: 0.0, y: 0.0 }).id();
    assert!(world.get::<Position>(entity).is_some());
    
    world.despawn(entity);
    assert!(world.get::<Position>(entity).is_none());
}
```

**Step 4**: Run tests (2 minutes)
```bash
cargo test -p astraweave-ecs --test ecs_core_tests -- --nocapture
```

**Step 5**: Check coverage (5 minutes)
```bash
cargo tarpaulin -p astraweave-ecs --out Stdout | grep "lib.rs"
```

**Expected Result**: 10-15% coverage on lib.rs (7-10 lines covered)

**Repeat**: Add 2-3 tests every 30 minutes, verify coverage incrementally

---

## 📚 Resources & References

### Existing Test Patterns
- **astraweave-assets**: Option B session tests (polyhaven_api_tests.rs, lib_api_tests.rs)
- **hello_companion**: Phase 6 AI integration tests
- **profiling_demo**: Week 8 performance tests

### Documentation
- **OPTION_B_FINAL_REPORT.md**: Test patterns, ROI analysis
- **PHASE_6_COMPLETION_SUMMARY.md**: AI testing patterns
- **WEEK_8_FINAL_SUMMARY.md**: Performance testing patterns

### Tools
- **cargo tarpaulin**: Coverage measurement
- **mockito**: HTTP mocking (for LLM tests)
- **proptest**: Property-based testing (for SIMD)
- **tokio-test**: Async testing utilities

---

## ⚠️ Risk Mitigation

### Risk 1: Scope Creep
**Mitigation**: Strict tier prioritization, accept 60-80% coverage (not 100%)

### Risk 2: API Drift
**Mitigation**: Read actual API before writing tests, compile frequently

### Risk 3: Mock Complexity
**Mitigation**: Start with unit tests, add integration tests only when needed

### Risk 4: Time Overrun
**Mitigation**: Weekly checkpoints, adjust scope if velocity < 15 lines/hour

### Risk 5: Flaky Tests
**Mitigation**: Use deterministic RNG, avoid timing-dependent assertions

---

## 🎯 Success Criteria

### Week 1 Completion
- [ ] astraweave-ecs: ≥70% coverage (314+ lines)
- [ ] astraweave-ai: ≥75% coverage (65+ lines)
- [ ] astraweave-physics: ≥75% coverage (50+ lines)
- [ ] All tests passing (100% pass rate)
- [ ] Zero compilation errors

### Week 2 Completion
- [ ] astraweave-math: ≥80% coverage (67+ lines)
- [ ] astraweave-render: ≥75% coverage (62+ lines)
- [ ] All Tier 1-2 crates ≥70% coverage
- [ ] +150-200 tests added

### Week 3 Completion
- [ ] astraweave-llm: ≥65% coverage (218+ lines)
- [ ] All Tier 1-3 crates ≥60% coverage
- [ ] +250-300 tests total across all tiers

### Overall Success
- [ ] **Zero coverage → 70%+ average** for core crates (Tier 1-2)
- [ ] **+925 lines covered** across 1,292 target lines
- [ ] **100% test pass rate** maintained throughout
- [ ] **Documentation updated** with test patterns and coverage reports

---

## 📊 Tracking & Reporting

### Daily Reports
Create `WEEK_X_DAY_Y_COVERAGE_REPORT.md` with:
- Lines covered today
- Tests added (count)
- Coverage % by crate
- Blockers/issues encountered
- Next day plan

### Weekly Summaries
Create `WEEK_X_COVERAGE_SUMMARY.md` with:
- Total lines covered (cumulative)
- Coverage % by tier
- Test count growth
- Velocity analysis (lines/hour)
- Lessons learned
- Week +1 plan adjustments

---

**Next Action**: Review this plan, confirm approach, then proceed with Week 1 Day 1 (ECS lib.rs testing) ✅
