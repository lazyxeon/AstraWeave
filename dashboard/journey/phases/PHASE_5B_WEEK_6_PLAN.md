# Phase 5B - Week 6: `astraweave-ecs` Testing Plan

**Crate**: `astraweave-ecs` (Archetype-based Entity Component System)  
**Estimated Duration**: 6-8 hours  
**Target Tests**: 60-80  
**Target Coverage**: 80-90%  
**Priority**: ⭐⭐⭐⭐⭐ (Engine foundation)

---

## Why astraweave-ecs for Week 6?

### Rationale

**1. Testability**: Pure Rust, no external dependencies
- No GPU context required (unlike astraweave-render)
- No LLM mocking needed (unlike astraweave-ai)
- No audio files needed (unlike astraweave-audio)
- Straightforward unit testing with high coverage potential

**2. Coverage Potential**: 80-90% achievable
- Well-defined API surface (queries, systems, events)
- Deterministic behavior (reproducible tests)
- Previous weeks average: 90.6% coverage

**3. Time Budget**: Fits remaining 15.6h comfortably
- Estimated: 6-8h
- Buffer: 7.6-9.6h remaining for Week 7
- Safe margin for Week 7 final crate

**4. Strategic Value**: ECS is engine foundation
- All other systems depend on ECS (AI, physics, rendering)
- High impact if bugs are found
- Core to AI-native architecture

**5. Test Contribution**: 60-80 tests hits 555 target
- Current: 507/555 (91%)
- Week 6: +60-80 → 567-587 total
- **Exceeds 555 target by 12-32 tests!**

---

## Scope Analysis

### Core Components

**1. Archetype Management** (`archetype.rs`)
- Entity storage by component signature
- Component addition/removal
- Query matching

**2. World Management** (`world.rs`)
- Entity spawn/despawn
- Component get/set
- System execution

**3. System Parameters** (`system_param.rs`)
- Query types (Query, QueryMut)
- Resource access (Res, ResMut)
- Event handling

**4. Events** (`events.rs`)
- Event queue management
- Event reader/writer
- Frame clearing

**5. System Stages** (`system_stage.rs`)
- Ordered execution (PRE_SIMULATION → PRESENTATION)
- System registration
- Deterministic ordering

---

## Estimated Test Breakdown

### Day 1: Baseline + Core Entity/Component Tests (1.5h)
**Target**: 15-20 tests

**Focus**:
- Measure existing coverage baseline
- Entity spawn/despawn
- Component add/remove/get/set
- Multiple component types
- Entity existence queries

**Example Tests**:
```rust
#[test]
fn test_spawn_entity() { ... }

#[test]
fn test_add_component() { ... }

#[test]
fn test_remove_component() { ... }

#[test]
fn test_multiple_components() { ... }

#[test]
fn test_despawn_entity() { ... }
```

---

### Day 2: Query System Tests (2h)
**Target**: 20-25 tests

**Focus**:
- Query matching (With, Without)
- QueryMut (mutable access)
- Multiple queries
- Query iteration order
- Empty query results

**Example Tests**:
```rust
#[test]
fn test_query_with_filter() { ... }

#[test]
fn test_query_mut() { ... }

#[test]
fn test_multiple_queries() { ... }

#[test]
fn test_query_empty() { ... }

#[test]
fn test_query_iteration_order() { ... }
```

---

### Day 3: System & Event Tests (1.5h)
**Target**: 15-20 tests

**Focus**:
- System registration
- System execution order
- Event sending/receiving
- Event queue clearing
- Multiple events per frame

**Example Tests**:
```rust
#[test]
fn test_register_system() { ... }

#[test]
fn test_system_execution_order() { ... }

#[test]
fn test_send_receive_event() { ... }

#[test]
fn test_event_queue_clearing() { ... }

#[test]
fn test_multiple_events() { ... }
```

---

### Day 4: Stress & Edge Cases (1.5h)
**Target**: 15-20 tests

**Focus**:
- 1,000+ entities
- 100+ components per entity
- Rapid spawn/despawn cycles
- Query performance under load
- System ordering edge cases

**Example Tests**:
```rust
#[test]
fn test_stress_many_entities() { ... }

#[test]
fn test_stress_many_components() { ... }

#[test]
fn test_stress_rapid_spawn_despawn() { ... }

#[test]
fn test_edge_empty_world() { ... }

#[test]
fn test_edge_system_ordering_conflict() { ... }
```

---

### Day 5: Documentation & Summary (0.5h)
**Target**: 0 new tests, comprehensive report

**Focus**:
- Create Week 6 completion report
- Update Phase 5B status (607/555 tests, 6/7 crates)
- Grade: ⭐⭐⭐⭐⭐ A+ (expected)

---

## Success Criteria

| Metric | Target | Stretch Goal |
|--------|--------|--------------|
| **Tests** | 60 | 80 |
| **Coverage** | 80% | 90% |
| **Time** | 6-8h | <6h |
| **Pass Rate** | 100% | 100% |
| **Benchmarks** | 5 | 10 |

**Grade Target**: ⭐⭐⭐⭐⭐ A+

---

## Patterns to Apply (from Weeks 1-5)

### 1. Helper Functions (Week 2)
```rust
fn spawn_entity_with_components(world: &mut World, ...) -> EntityId { ... }
fn query_count<T: Component>(world: &World) -> usize { ... }
```

### 2. Stress Test Thresholds (Week 3)
- 1,000+ entities (scalability)
- 100+ components (memory stress)
- 1,000+ queries (lookup performance)

### 3. Edge Case Categories (Week 4)
- Empty state (no entities, no systems)
- Boundary conditions (single entity, zero components)
- Error paths (invalid queries, missing components)

### 4. Documentation Quality (Week 5)
- Comprehensive file headers
- Helper function docstrings
- Section markers for organization

### 5. Sub-File Organization (Week 5)
```rust
// Day 1: Core Entity Tests
#[test]
fn test_spawn_entity() { ... }

// Day 2: Query System Tests
#[test]
fn test_query_with_filter() { ... }
```

---

## Expected Challenges

### 1. System Ordering Complexity
**Challenge**: Testing system execution order with side effects
**Solution**: Use mock components with counters to track execution order

### 2. Query Type Safety
**Challenge**: Compile-time query validation means fewer runtime tests
**Solution**: Focus on runtime behavior (iteration order, filtering)

### 3. Event Queue Lifetime
**Challenge**: Events cleared per frame, hard to test persistence
**Solution**: Test frame boundaries explicitly (before/after clear)

### 4. Archetype Switching
**Challenge**: Component add/remove causes entity to move archetypes
**Solution**: Test component presence after add/remove operations

---

## Risk Mitigation

### Risk 1: Complex API Surface
**Probability**: Medium  
**Impact**: Could take 10h instead of 6-8h  
**Mitigation**: Focus on core paths first (entity, component, query), defer advanced features if time runs out

### Risk 2: Low Existing Coverage
**Probability**: Low (ECS is mature)  
**Impact**: More tests needed to reach 80% target  
**Mitigation**: Measure baseline on Day 1, adjust targets if <50% existing

### Risk 3: Test Complexity
**Probability**: Medium  
**Impact**: Tests harder to write/debug than expected  
**Mitigation**: Apply Week 2-5 helper function patterns, keep tests simple

---

## Week 7 Preview

After Week 6 completes with 60-80 tests (567-587 total), we'll have **12-32 tests over 555 target**.

**Week 7 Options** (pick 1-2 crates to use remaining 7.6-9.6h):

### Option 1: `astraweave-render` (50-60 tests, 6-7h)
**Pros**: High user-facing impact, material system testable  
**Cons**: GPU tests complex, lower coverage ceiling (60-75%)

### Option 2: `astraweave-physics` (40-50 tests, 5-6h)
**Pros**: Critical for gameplay, collision detection testable  
**Cons**: Rapier3D integration testing tricky

### Option 3: `astraweave-gameplay` (30-40 tests, 4-5h)
**Pros**: Combat physics pure Rust, easy to test  
**Cons**: Smaller crate, less strategic impact

**Recommended**: Combination approach
- Start with smaller crate (gameplay/physics)
- If time remains, add renderer tests
- Goal: Use full 7.6-9.6h buffer

---

## Success Prediction

**Confidence**: 🟢 **HIGH** (85-90%)

**Reasons**:
1. Pure Rust (no external dependencies)
2. Well-defined API (no ambiguity)
3. 5/5 A+ track record (proven execution)
4. 1.4× efficiency maintained (16h buffer)
5. Testability lessons from Weeks 1-5 apply directly

**Expected Outcome**: 60-80 tests, 80-90% coverage, 6-8h, ⭐⭐⭐⭐⭐ A+

---

## Timeline

| Day | Focus | Hours | Tests | Status |
|-----|-------|-------|-------|--------|
| **Day 1** | Baseline + Entity/Component | 1.5h | 15-20 | 📅 Planned |
| **Day 2** | Query System | 2h | 20-25 | 📅 Planned |
| **Day 3** | Systems & Events | 1.5h | 15-20 | 📅 Planned |
| **Day 4** | Stress & Edge | 1.5h | 15-20 | 📅 Planned |
| **Day 5** | Documentation | 0.5h | 0 | 📅 Planned |
| **TOTAL** | Week 6 | **7h** | **65-85** | 🎯 Target |

**Buffer**: 8.6h remaining for Week 7 after Week 6

---

## Celebration Goals 🎉

If Week 6 succeeds:
- ✅ 6/7 crates complete (86%)
- ✅ 567-587 tests (exceeds 555 target!)
- ✅ 6/6 A+ grades (100% success rate)
- ✅ 36.4h/45h invested (81%)
- ✅ 8.6h buffer for Week 7 final sprint

**Phase 5B on track for 100% completion with 6-7 A+ grades!**

---

**Next Action**: Start Week 6 Day 1 - Measure astraweave-ecs baseline and create core entity/component tests

**Command**: 
```powershell
# Measure baseline coverage
cargo llvm-cov --lib -p astraweave-ecs --summary-only

# Count existing tests
cargo test -p astraweave-ecs --lib -- --list
```

---

**Document Status**: ✅ COMPLETE  
**Week 6 Start**: Ready to begin  
**Confidence**: 🟢 HIGH (85-90%)
