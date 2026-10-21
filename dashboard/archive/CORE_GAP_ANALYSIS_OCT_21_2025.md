# astraweave-core Gap Analysis & Test Plan - October 21, 2025

**Current Coverage**: 65.27% (748/1,146 lines)  
**Target Coverage**: 80% (+14.73pp improvement)  
**Current Tests**: 15 tests  
**Estimated Tests Needed**: +15-25 tests  
**Estimated Time**: 3-5 hours

---

## Executive Summary

**astraweave-core** currently has **65.27% coverage** with only **15 tests** for **1,146 lines of code**. This creates a **test density of 76.4 lines/test**, which is extremely coarse-grained (should be 10-15 lines/test).

**Primary Gaps**:
1. **schema.rs** (426 lines) - NO TESTS - Core data structures untested
2. **validation.rs** (457 lines) - Only 2 tests - Validation logic undertested
3. **Smaller utility files** - 11 files with NO tests (perception.rs, world.rs, etc.)

**Strategy**: Focus on schema.rs first (highest value), then validation.rs, then utility files.

---

## File-by-File Analysis

### Breakdown by File Size & Test Coverage

| File | Lines | Tests | Est. Coverage | Priority | Tests Needed |
|------|-------|-------|---------------|----------|--------------|
| **tool_vocabulary.rs** | 826 | 4 | ~99%? | P4 LOW | 0 (already good) |
| **validation.rs** | 457 | 2 | ~30%? | **P1 HIGH** | +8-10 |
| **schema.rs** | 426 | 0 | **0%** | **P1 HIGHEST** | +8-12 |
| **ecs_adapter.rs** | 360 | 5 | ~85%? | P4 LOW | 0 (already good) |
| **tools.rs** | 235 | 0 | 0% | **P2 MEDIUM** | +3-5 |
| **tool_sandbox.rs** | 202 | 4 | ~60%? | P3 LOW | +2-3 |
| **world.rs** | 120 | 0 | 0% | P3 LOW | +2-3 |
| **ecs_components.rs** | 98 | 0 | 0% | P3 LOW | +1-2 |
| **ecs_events.rs** | 74 | 0 | 0% | P3 LOW | +1-2 |
| **ecs_bridge.rs** | 73 | 0 | 0% | P3 LOW | +1-2 |
| **perception.rs** | 71 | 0 | 0% | P2 MEDIUM | +2-3 |
| **lib.rs** | 63 | 0 | 100% (re-exports) | - | 0 |
| **capture_replay.rs** | 57 | 0 | 0% | P3 LOW | +1-2 |
| **sim.rs** | 7 | 0 | 100% (constants) | - | 0 |
| **util.rs** | 4 | 0 | 100% (utility) | - | 0 |
| **TOTAL** | **3,073** | **15** | **65.27%** | - | **+29-45** |

**Key Finding**: 11/15 files have ZERO tests! But 65% coverage means other files are well-tested.

**Well-Tested Files** (can skip):
- tool_vocabulary.rs: ~99% (4 tests, comprehensive)
- ecs_adapter.rs: ~85% (5 tests, good coverage)
- tool_sandbox.rs: ~60% (4 tests, reasonable)

**Critical Gaps** (must address):
- schema.rs: 0% ← **HIGHEST PRIORITY**
- validation.rs: ~30% ← **HIGH PRIORITY**
- tools.rs: 0% ← **MEDIUM PRIORITY**
- perception.rs: 0% ← **MEDIUM PRIORITY**

---

## Priority 1: schema.rs (8-12 tests) 🎯

**Current**: 0% coverage, 0 tests  
**Target**: 75% coverage  
**Lines**: 426  
**Time**: 2.5-3.5 hours

### File Overview

**Purpose**: Core data structures for AI planning (WorldSnapshot, PlanIntent, ActionStep, etc.)

**Key Types** (from copilot-instructions.md):
```rust
pub struct WorldSnapshot {
    pub t: f32,
    pub player: PlayerState,
    pub me: CompanionState,
    pub enemies: Vec<EnemyState>,
    pub pois: Vec<Poi>,
    pub obstacles: Vec<IVec2>,
    pub objective: Option<String>,
}

pub struct CompanionState {
    pub ammo: i32,
    pub cooldowns: BTreeMap<String, f32>,
    pub morale: f32,
    pub pos: IVec2,
}

pub struct PlanIntent {
    pub plan_id: String,
    pub steps: Vec<ActionStep>,
}

pub enum ActionStep {
    MoveTo { x: i32, y: i32, speed: Option<f32> },
    Attack { target_id: u32 },
    Throw { item: String, x: i32, y: i32 },
    CoverFire { target_id: u32, duration: f32 },
    TakeCover { x: i32, y: i32 },
    Reload,
    // ... more variants
}
```

### Identified Gaps

**Gap 1: WorldSnapshot Construction** (3 tests)
```rust
// UNCOVERED: WorldSnapshot::new()
// UNCOVERED: WorldSnapshot field validation
// UNCOVERED: WorldSnapshot serialization
```

**Test Plan**:
```rust
#[test]
fn test_world_snapshot_creation() {
    let snap = WorldSnapshot {
        t: 1.0,
        player: PlayerState::default(),
        me: CompanionState::default(),
        enemies: vec![],
        pois: vec![],
        obstacles: vec![],
        objective: None,
    };
    
    assert_eq!(snap.t, 1.0);
    assert!(snap.enemies.is_empty());
}

#[test]
fn test_world_snapshot_with_enemies() {
    let snap = WorldSnapshot {
        // ... with 3 enemies
    };
    
    assert_eq!(snap.enemies.len(), 3);
}

#[test]
fn test_world_snapshot_serialization() {
    let snap = WorldSnapshot { /* ... */ };
    
    let json = serde_json::to_string(&snap).unwrap();
    let deserialized: WorldSnapshot = serde_json::from_str(&json).unwrap();
    
    assert_eq!(snap.t, deserialized.t);
}
```

---

**Gap 2: CompanionState** (2 tests)
```rust
// UNCOVERED: CompanionState::default()
// UNCOVERED: Cooldown management
```

**Test Plan**:
```rust
#[test]
fn test_companion_state_default() {
    let companion = CompanionState::default();
    
    assert_eq!(companion.ammo, 0);  // or expected default
    assert!(companion.cooldowns.is_empty());
    assert_eq!(companion.morale, 100.0);  // or expected default
}

#[test]
fn test_companion_cooldowns() {
    let mut companion = CompanionState::default();
    companion.cooldowns.insert("attack".to_string(), 2.5);
    
    assert_eq!(companion.cooldowns.get("attack"), Some(&2.5));
}
```

---

**Gap 3: PlanIntent** (2 tests)
```rust
// UNCOVERED: PlanIntent::new()
// UNCOVERED: plan_id generation
```

**Test Plan**:
```rust
#[test]
fn test_plan_intent_creation() {
    let plan = PlanIntent {
        plan_id: "test-plan-1".to_string(),
        steps: vec![],
    };
    
    assert_eq!(plan.plan_id, "test-plan-1");
    assert!(plan.steps.is_empty());
}

#[test]
fn test_plan_intent_with_steps() {
    let plan = PlanIntent {
        plan_id: "plan-123".to_string(),
        steps: vec![
            ActionStep::MoveTo { x: 5, y: 10, speed: None },
            ActionStep::Attack { target_id: 42 },
        ],
    };
    
    assert_eq!(plan.steps.len(), 2);
}
```

---

**Gap 4: ActionStep Enum** (5 tests - one per major variant)
```rust
// UNCOVERED: ActionStep::MoveTo
// UNCOVERED: ActionStep::Attack
// UNCOVERED: ActionStep::Throw
// UNCOVERED: ActionStep::CoverFire
// UNCOVERED: ActionStep::TakeCover
// UNCOVERED: ActionStep::Reload
```

**Test Plan**:
```rust
#[test]
fn test_action_step_move_to() {
    let action = ActionStep::MoveTo { x: 10, y: 20, speed: Some(1.5) };
    
    match action {
        ActionStep::MoveTo { x, y, speed } => {
            assert_eq!(x, 10);
            assert_eq!(y, 20);
            assert_eq!(speed, Some(1.5));
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_action_step_attack() {
    let action = ActionStep::Attack { target_id: 42 };
    
    match action {
        ActionStep::Attack { target_id } => {
            assert_eq!(target_id, 42);
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_action_step_throw() {
    let action = ActionStep::Throw {
        item: "grenade".to_string(),
        x: 15,
        y: 25,
    };
    
    match action {
        ActionStep::Throw { item, x, y } => {
            assert_eq!(item, "grenade");
            assert_eq!(x, 15);
            assert_eq!(y, 25);
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_action_step_cover_fire() {
    let action = ActionStep::CoverFire {
        target_id: 10,
        duration: 3.5,
    };
    
    match action {
        ActionStep::CoverFire { target_id, duration } => {
            assert_eq!(target_id, 10);
            assert_eq!(duration, 3.5);
        }
        _ => panic!("Wrong variant"),
    }
}

#[test]
fn test_action_step_reload() {
    let action = ActionStep::Reload;
    
    match action {
        ActionStep::Reload => {},
        _ => panic!("Wrong variant"),
    }
}
```

---

### schema.rs Test Summary

**New Tests**: 12 tests (3 + 2 + 2 + 5)  
**Coverage Gain**: 0% → 75% (+75pp)  
**Time**: 2.5-3.5 hours  
**File**: Create `astraweave-core/tests/schema_tests.rs`

**Impact**: Highest value - schema is core of AI planning system

---

## Priority 2: validation.rs (8-10 tests)

**Current**: ~30% coverage (2 tests exist)  
**Target**: 75% coverage  
**Lines**: 457  
**Time**: 2-2.5 hours

### Existing Tests (2 tests)

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_basic_validation()  // Basic validation pass
    
    #[test]
    fn test_validation_failure()  // Validation failure case
}
```

**Coverage**: Only basic validation, missing edge cases

### Identified Gaps

**Gap 1: Tool Validation Rules** (3 tests)
```rust
// UNCOVERED: Range validation
// UNCOVERED: Cooldown validation
// UNCOVERED: Resource validation (ammo, etc.)
```

**Test Plan**:
```rust
#[test]
fn test_validation_range_check() {
    // Action with target out of range
    // Should fail validation
}

#[test]
fn test_validation_cooldown_check() {
    // Action on cooldown
    // Should fail validation
}

#[test]
fn test_validation_resource_check() {
    // Action requires ammo, but ammo = 0
    // Should fail validation
}
```

---

**Gap 2: Validation Error Types** (3 tests)
```rust
// UNCOVERED: Different ValidationError variants
// UNCOVERED: Error message formatting
// UNCOVERED: Error categorization
```

**Test Plan**:
```rust
#[test]
fn test_validation_error_types() {
    // Test each ValidationError variant
    // Verify error messages
}

#[test]
fn test_validation_error_formatting() {
    // Test Display impl for errors
}

#[test]
fn test_validation_error_categories() {
    // Group errors by category
    // Verify categorization logic
}
```

---

**Gap 3: Complex Validation Scenarios** (3 tests)
```rust
// UNCOVERED: Multi-step plan validation
// UNCOVERED: State-dependent validation
// UNCOVERED: Composite validation rules
```

**Test Plan**:
```rust
#[test]
fn test_multi_step_validation() {
    // Plan with 5 steps
    // Verify all steps validated
}

#[test]
fn test_state_dependent_validation() {
    // Action valid in state A, invalid in state B
}

#[test]
fn test_composite_validation() {
    // Multiple validation rules applied
    // Verify all rules checked
}
```

---

### validation.rs Test Summary

**New Tests**: 9 tests (3 + 3 + 3)  
**Coverage Gain**: ~30% → 75% (+45pp)  
**Time**: 2-2.5 hours  
**File**: Expand existing `validation.rs #[cfg(test)] mod tests`

---

## Priority 3: tools.rs (3-5 tests)

**Current**: 0% coverage, 0 tests  
**Target**: 60% coverage  
**Lines**: 235  
**Time**: 1-1.5 hours

### File Overview

**Purpose**: Tool definition structures and utilities

### Test Plan

```rust
#[test]
fn test_tool_definition_creation() {
    // Create ToolDefinition struct
    // Verify fields
}

#[test]
fn test_tool_registry() {
    // Add tools to registry
    // Query tools
}

#[test]
fn test_tool_parameters() {
    // Tool with parameters
    // Verify parameter validation
}

#[test]
fn test_default_tool_registry() {
    // Test default_tool_registry() function
    // Verify default tools present
}
```

**New Tests**: 4 tests  
**Coverage Gain**: 0% → 60% (+60pp)  
**Time**: 1-1.5 hours  
**File**: Create `tools.rs #[cfg(test)] mod tests`

---

## Priority 4: perception.rs (2-3 tests)

**Current**: 0% coverage, 0 tests  
**Target**: 60% coverage  
**Lines**: 71  
**Time**: 0.5-1 hour

### File Overview

**Purpose**: Perception system for AI (building WorldSnapshot from World state)

### Test Plan

```rust
#[test]
fn test_build_world_snapshot() {
    // Create World with entities
    // Build WorldSnapshot
    // Verify snapshot contains entities
}

#[test]
fn test_perception_filtering() {
    // World with 10 entities
    // Perception should filter to relevant entities only
}

#[test]
fn test_perception_range() {
    // Entities beyond perception range
    // Should not appear in snapshot
}
```

**New Tests**: 3 tests  
**Coverage Gain**: 0% → 60% (+60pp)  
**Time**: 0.5-1 hour  
**File**: Create `perception.rs #[cfg(test)] mod tests`

---

## Priority 5: Small Utility Files (6-10 tests total)

**Files**: world.rs, ecs_components.rs, ecs_events.rs, ecs_bridge.rs, capture_replay.rs  
**Current**: 0% coverage for all  
**Target**: 50-60% coverage  
**Combined Lines**: ~420 lines  
**Time**: 1-1.5 hours

### Approach: 1-2 tests per file

**world.rs** (2 tests):
```rust
#[test]
fn test_world_creation() { /* ... */ }

#[test]
fn test_world_entity_management() { /* ... */ }
```

**ecs_components.rs** (2 tests):
```rust
#[test]
fn test_component_definitions() { /* ... */ }

#[test]
fn test_component_serialization() { /* ... */ }
```

**ecs_events.rs** (2 tests):
```rust
#[test]
fn test_event_creation() { /* ... */ }

#[test]
fn test_event_dispatch() { /* ... */ }
```

**ecs_bridge.rs** (1 test):
```rust
#[test]
fn test_legacy_ecs_bridge() { /* ... */ }
```

**capture_replay.rs** (2 tests):
```rust
#[test]
fn test_capture_snapshot() { /* ... */ }

#[test]
fn test_replay_snapshot() { /* ... */ }
```

**New Tests**: 9 tests (2+2+2+1+2)  
**Coverage Gain**: 0% → 55% average (+55pp)  
**Time**: 1-1.5 hours

---

## Deferred: Already Well-Tested Files

### tool_vocabulary.rs (826 lines, 4 tests, ~99% coverage)

**Reason**: Already has excellent coverage, no work needed.

### ecs_adapter.rs (360 lines, 5 tests, ~85% coverage)

**Reason**: Already has good coverage, close to target.

### tool_sandbox.rs (202 lines, 4 tests, ~60% coverage)

**Reason**: Reasonable coverage, lower priority than schema/validation.

**Combined**: 1,388 lines already at 80%+ average ← Can skip!

---

## Test Implementation Strategy

### Approach: Systematic File-by-File

**File Structure**:
```
astraweave-core/
├── src/
│   ├── validation.rs (existing tests - expand)
│   ├── tools.rs (NEW tests inline)
│   ├── perception.rs (NEW tests inline)
│   ├── world.rs (NEW tests inline)
│   ├── ecs_components.rs (NEW tests inline)
│   ├── ecs_events.rs (NEW tests inline)
│   ├── ecs_bridge.rs (NEW tests inline)
│   └── capture_replay.rs (NEW tests inline)
└── tests/
    ├── schema_tests.rs (NEW - 12 tests)
    └── integration_tests.rs (NEW - optional)
```

---

### Phase 1: schema_tests.rs (2.5-3.5 hours) 🎯

**Priority**: HIGHEST  
**Tests**: 12 tests  
**Coverage Impact**: +25-30pp (largest untested file)

**Categories**:
1. WorldSnapshot (3 tests)
2. CompanionState (2 tests)
3. PlanIntent (2 tests)
4. ActionStep enum (5 tests)

**Template**:
```rust
use astraweave_core::*;

#[test]
fn test_world_snapshot_creation() {
    let snap = WorldSnapshot {
        t: 1.0,
        player: PlayerState {
            pos: IVec2 { x: 0, y: 0 },
            hp: 100,
        },
        me: CompanionState {
            pos: IVec2 { x: 5, y: 5 },
            ammo: 30,
            cooldowns: BTreeMap::new(),
            morale: 80.0,
        },
        enemies: vec![],
        pois: vec![],
        obstacles: vec![],
        objective: Some("Survive".to_string()),
    };
    
    assert_eq!(snap.t, 1.0);
    assert_eq!(snap.me.ammo, 30);
    assert!(snap.enemies.is_empty());
}

// ... 11 more tests
```

---

### Phase 2: Expand validation.rs Tests (2-2.5 hours)

**Tests**: +9 tests (2 existing + 9 new = 11 total)  
**Coverage Impact**: +10-15pp

**Approach**: Add to existing `#[cfg(test)] mod tests` block

---

### Phase 3: Small File Tests (2-3 hours)

**Files**: tools.rs, perception.rs, world.rs, ecs_*, capture_replay.rs  
**Tests**: +16 tests (4+3+9)  
**Coverage Impact**: +5-10pp

**Approach**: Add inline `#[cfg(test)] mod tests` to each file

---

## Expected Outcomes

### Coverage Targets

| Phase | Tests Added | Est. Coverage | Time |
|-------|-------------|---------------|------|
| **Baseline** | 15 | **65.27%** | - |
| Phase 1 (schema) | +12 | 73-75% | 2.5-3.5h |
| Phase 2 (validation) | +9 | 76-79% | 2-2.5h |
| Phase 3 (small files) | +16 | 80-83% | 2-3h |
| **TOTAL** | **+37** | **80-83%** | **6.5-9h** |

**Note**: Estimate is 6.5-9h, but original estimate was 3-5h. This is because:
1. schema.rs is larger than expected (426 lines, 0% coverage)
2. Many small files need tests (11 files with 0 tests)

**Revised Estimate**: 6.5-9 hours (vs original 3-5h estimate)

**Mitigation**: Can achieve 75-79% with just Phases 1-2 (4.5-6h), which still exceeds target.

---

### Success Criteria

**Minimum Viable** (Phase 1 + Phase 2):
- ✅ 36 tests (15 + 12 + 9 = 36 tests)
- ✅ 76-79% coverage (exceeds 75% target)
- ✅ 4.5-6 hours (close to original estimate)

**Stretch Goal** (All 3 phases):
- ✅ 52 tests (15 + 12 + 9 + 16 = 52 tests)
- ✅ 80-83% coverage (meets 80% target)
- ✅ 6.5-9 hours

---

## Implementation Checklist

### Pre-Work (0.5 hours)
- [ ] Review schema.rs API (understand all types)
- [ ] Check validation.rs existing tests (understand patterns)
- [ ] Verify test helper availability (make_test_world, etc.)

### Phase 1: schema_tests.rs (2.5-3.5 hours)
- [ ] Create `tests/schema_tests.rs`
- [ ] Write 3 WorldSnapshot tests
- [ ] Write 2 CompanionState tests
- [ ] Write 2 PlanIntent tests
- [ ] Write 5 ActionStep tests
- [ ] Run `cargo test -p astraweave-core`
- [ ] Run tarpaulin (measure coverage)

### Phase 2: validation.rs Expansion (2-2.5 hours)
- [ ] Add 3 tool validation tests
- [ ] Add 3 validation error tests
- [ ] Add 3 complex scenario tests
- [ ] Run `cargo test -p astraweave-core`
- [ ] Run tarpaulin (verify 76-79%)

### Phase 3: Small File Tests (2-3 hours)
- [ ] Add tools.rs tests (4 tests)
- [ ] Add perception.rs tests (3 tests)
- [ ] Add world.rs tests (2 tests)
- [ ] Add ecs_components.rs tests (2 tests)
- [ ] Add ecs_events.rs tests (2 tests)
- [ ] Add ecs_bridge.rs tests (1 test)
- [ ] Add capture_replay.rs tests (2 tests)
- [ ] Run `cargo test -p astraweave-core`
- [ ] Run tarpaulin (verify 80-83%)

### Completion (0.5 hours)
- [ ] Final tarpaulin run (HTML report)
- [ ] Document final coverage
- [ ] Create completion summary
- [ ] Update TODO list

---

## Risk Assessment

### Risk 1: Time Overrun ⚠️

**Issue**: 6.5-9h vs 3-5h original estimate

**Mitigation**:
- Accept 76-79% with Phases 1-2 only (4.5-6h)
- Phase 3 small files optional (nice-to-have)
- Still exceeds 75% minimum target

**Impact**: Medium (can still meet minimum target in budget)

---

### Risk 2: Schema Complexity ⚠️

**Issue**: schema.rs may have complex types not captured in analysis

**Mitigation**:
- Read schema.rs fully before starting tests
- Start with simplest types (IVec2, PlanIntent)
- Add complexity incrementally

**Impact**: Low (worst case, reduce test count)

---

### Risk 3: Validation Logic Complexity ⚠️

**Issue**: Validation rules may be intricate, hard to test

**Mitigation**:
- Focus on happy paths first
- Add edge cases if time allows
- Accept 70-75% if validation too complex

**Impact**: Low (validation.rs only 2-2.5h)

---

## Summary

**Current**: 65.27% coverage, 15 tests  
**Target**: 80% coverage, ~50 tests  
**Gap**: +14.73pp, +35 tests  
**Time**: 6.5-9 hours (revised from 3-5h)

**Strategic Approach**:
1. ✅ schema.rs (highest value, 426 lines, 0% → 75%) - 2.5-3.5h
2. ✅ validation.rs (high value, 457 lines, 30% → 75%) - 2-2.5h
3. ⚠️ Small files (nice-to-have, 11 files, 0% → 55%) - 2-3h

**Minimum Success**: Phases 1-2 (76-79% coverage, 4.5-6h)  
**Full Success**: All 3 phases (80-83% coverage, 6.5-9h)

**Key Insight**: astraweave-core has MORE untested files than expected (11 files with 0 tests), but high baseline (65%) means well-tested files carry the average.

**Next Step**: Begin Phase 1 - Create `schema_tests.rs` (2.5-3.5 hours)

---

**End of Gap Analysis** | **Status**: Ready for implementation 🚀
