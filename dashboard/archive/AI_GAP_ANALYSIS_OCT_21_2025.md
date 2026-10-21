# astraweave-ai Gap Analysis & Test Plan - October 21, 2025

**Current Coverage**: 46.83% (155/331 lines)  
**Target Coverage**: 80% (+33.17pp improvement)  
**Current Tests**: 11 tests  
**Estimated Tests Needed**: +24-34 tests  
**Estimated Time**: 5-8 hours

---

## Executive Summary

**astraweave-ai** currently has **46.83% coverage** with only **11 tests** for **331 lines of code**. This creates a **test density of 30.1 lines/test**, which is too coarse-grained (should be 10-15 lines/test).

**Primary Gaps**:
1. **Orchestrator implementations** - Multiple AI planning strategies undertested
2. **ECS integration** - Plugin and system registration undertested  
3. **Async/LLM features** - Feature-gated code paths (ai_arbiter, llm_executor, async_task)

**Strategy**: Focus on core orchestrator logic first (highest value), then ECS integration, defer LLM features (feature-gated, lower priority).

---

## File-by-File Analysis

### Breakdown by File Size

| File | Lines | Est. Current Coverage | Priority | Tests Needed |
|------|-------|----------------------|----------|--------------|
| **ai_arbiter.rs** | 632 | <10%? (feature-gated) | P3 LOW | 0 (defer) |
| **orchestrator.rs** | 499 | ~50%? (5 tests exist) | **P1 HIGH** | +10-15 |
| **llm_executor.rs** | 450 | <10%? (feature-gated) | P3 LOW | 0 (defer) |
| **async_task.rs** | 420 | <10%? (feature-gated) | P3 LOW | 0 (defer) |
| **tool_sandbox.rs** | 382 | ~40%? (4 tests exist) | **P2 MEDIUM** | +5-8 |
| **core_loop.rs** | 371 | ~60%? (6 tests exist) | **P2 MEDIUM** | +3-5 |
| **ecs_ai_plugin.rs** | 298 | ~20%? (1 test exists) | **P1 HIGH** | +6-8 |
| **lib.rs** | 21 | 100% (re-exports) | - | 0 |
| **TOTAL** | **3,073** | **46.83%** | - | **+24-36** |

**Note**: Total lines (3,073) >> measured lines (331) because feature-gated code (`#[cfg(feature = "llm_orchestrator")]`) is excluded from baseline measurement.

**Measured Files** (331 lines):
- orchestrator.rs: ~160-180 lines (non-LLM parts)
- tool_sandbox.rs: ~60-80 lines
- core_loop.rs: ~50-70 lines
- ecs_ai_plugin.rs: ~40-60 lines

---

## Priority 1: orchestrator.rs (10-15 tests) 🎯

**Current**: ~50% coverage (5 tests exist)  
**Target**: 85% coverage  
**Lines**: ~160-180 (measured, non-LLM)  
**Time**: 3-4 hours

### Existing Tests (5 tests in orchestrator.rs)

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_rule_orchestrator_smoke_plan()  // RuleOrchestrator with smoke
    
    #[test]
    fn test_rule_orchestrator_no_smoke()    // RuleOrchestrator without smoke
    
    #[test]
    fn test_utility_orchestrator_basic()    // UtilityOrchestrator basic
    
    #[test]
    fn test_goap_orchestrator_basic()       // GoapOrchestrator basic
    
    #[test]
    fn test_system_orchestrator_config()    // SystemOrchestratorConfig
}
```

**Coverage**: Basic happy paths for each orchestrator type

### Identified Gaps

**Gap 1: RuleOrchestrator Edge Cases** (3 tests needed)
```rust
// UNCOVERED: No enemies in snapshot
// UNCOVERED: Multiple enemies (target selection)
// UNCOVERED: Player position edge cases
```

**Test Plan**:
```rust
#[test]
fn test_rule_orchestrator_no_enemies() {
    // WorldSnapshot with empty enemies vec
    // Should return empty PlanIntent (fallback)
}

#[test]
fn test_rule_orchestrator_multiple_enemies() {
    // WorldSnapshot with 3+ enemies
    // Verify picks closest enemy
}

#[test]
fn test_rule_orchestrator_edge_positions() {
    // Enemy at same position as companion
    // Verify doesn't crash, produces valid plan
}
```

---

**Gap 2: UtilityOrchestrator Scoring** (4 tests needed)
```rust
// UNCOVERED: Utility scoring calculations
// UNCOVERED: Multiple action options ranking
// UNCOVERED: Morale effects on decisions
// UNCOVERED: Distance calculations
```

**Test Plan**:
```rust
#[test]
fn test_utility_scoring_attack() {
    // Close enemy, high ammo → should score attack highest
}

#[test]
fn test_utility_scoring_defend() {
    // Low ammo, high morale → should prefer defensive actions
}

#[test]
fn test_utility_morale_effect() {
    // Low morale (<30) → should avoid aggressive actions
}

#[test]
fn test_utility_distance_weighting() {
    // Far enemies → lower attack utility
    // Close enemies → higher attack utility
}
```

---

**Gap 3: GoapOrchestrator Planning** (3 tests needed)
```rust
// UNCOVERED: GOAP graph search edge cases
// UNCOVERED: No valid plan scenarios
// UNCOVERED: Cost calculations
```

**Test Plan**:
```rust
#[test]
fn test_goap_no_valid_plan() {
    // Impossible goal state
    // Should return fallback plan
}

#[test]
fn test_goap_cost_optimization() {
    // Multiple paths to goal
    // Verify picks lowest-cost path
}

#[test]
fn test_goap_state_transitions() {
    // Complex state changes
    // Verify correct action sequencing
}
```

---

**Gap 4: SystemOrchestratorConfig** (2 tests needed)
```rust
// UNCOVERED: make_system_orchestrator with different modes
// UNCOVERED: PlannerMode enum coverage
```

**Test Plan**:
```rust
#[test]
fn test_make_system_orchestrator_all_modes() {
    // Classical, BehaviorTree, Utility, GOAP modes
    // Verify correct orchestrator type returned
}

#[test]
fn test_planner_mode_serialization() {
    // Verify PlannerMode enum values
    // Test Display/Debug traits
}
```

---

### orchestrator.rs Test Summary

**New Tests**: 12 tests (3 + 4 + 3 + 2)  
**Coverage Gain**: ~50% → 85% (+35pp)  
**Time**: 3-4 hours  
**File**: Create `astraweave-ai/tests/orchestrator_extended_tests.rs` (keep existing inline tests)

---

## Priority 2: ecs_ai_plugin.rs (6-8 tests) 🎯

**Current**: ~20% coverage (1 test exists)  
**Target**: 75% coverage  
**Lines**: ~40-60 (measured)  
**Time**: 2-3 hours

### Existing Test (1 test in ecs_ai_plugin.rs)

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_ai_plugin_builds() {
        // Basic smoke test that plugin can be created
    }
}
```

**Coverage**: Plugin instantiation only

### Identified Gaps

**Gap 1: Plugin Registration** (2 tests)
```rust
// UNCOVERED: AiPlanningPlugin::name()
// UNCOVERED: AiPlanningPlugin::setup()
```

**Test Plan**:
```rust
#[test]
fn test_ai_plugin_name() {
    let plugin = AiPlanningPlugin;
    assert_eq!(plugin.name(), "AiPlanningPlugin");
}

#[test]
fn test_ai_plugin_setup() {
    let mut app = ecs::App::default();
    let plugin = AiPlanningPlugin;
    
    plugin.setup(&mut app);
    
    // Verify systems registered
    // Verify stages configured
}
```

---

**Gap 2: build_app_with_ai Function** (3 tests)
```rust
// UNCOVERED: build_app_with_ai core logic
// UNCOVERED: System registration order
// UNCOVERED: Legacy World integration
```

**Test Plan**:
```rust
#[test]
fn test_build_app_with_ai_systems() {
    let legacy_world = astraweave_core::World::new();
    let app = build_app_with_ai(legacy_world, 0.016);
    
    // Verify AI planning systems registered
    // Verify system stages correct
}

#[test]
fn test_build_app_with_ai_timestep() {
    let world = astraweave_core::World::new();
    let app = build_app_with_ai(world, 0.033); // 30 FPS
    
    // Verify dt passed correctly
}

#[test]
fn test_build_app_with_legacy_world() {
    let mut world = astraweave_core::World::new();
    // Populate world with entities
    
    let app = build_app_with_ai(world, 0.016);
    
    // Verify world data accessible in ECS
}
```

---

**Gap 3: System Functions** (2 tests)
```rust
// UNCOVERED: AI planning system execution
// UNCOVERED: Component queries
```

**Test Plan**:
```rust
#[test]
fn test_ai_planning_system_execution() {
    // Create app with AI
    // Spawn entity with AI component
    // Run one tick
    // Verify AI planning executed
}

#[test]
fn test_ai_component_queries() {
    // Test component access patterns
    // Verify queries work correctly
}
```

---

### ecs_ai_plugin.rs Test Summary

**New Tests**: 7 tests (2 + 3 + 2)  
**Coverage Gain**: ~20% → 75% (+55pp)  
**Time**: 2-3 hours  
**File**: Expand existing `ecs_ai_plugin.rs #[cfg(test)] mod tests`

---

## Priority 3: tool_sandbox.rs (5-8 tests)

**Current**: ~40% coverage (4 tests exist)  
**Target**: 75% coverage  
**Lines**: ~60-80 (measured)  
**Time**: 1.5-2 hours

### Existing Tests (4 tests in tool_sandbox.rs)

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_validate_move_to_valid()        // Valid MoveTo action
    
    #[test]
    fn test_validate_move_to_obstacle()     // MoveTo blocked by obstacle
    
    #[test]
    fn test_validate_throw_out_of_range()   // Throw too far
    
    #[test]
    fn test_validate_attack_valid()         // Valid Attack action
}
```

**Coverage**: Basic validation for 3 action types (MoveTo, Throw, Attack)

### Identified Gaps

**Gap 1: Additional Action Types** (3 tests)
```rust
// UNCOVERED: CoverFire validation
// UNCOVERED: TakeCover validation
// UNCOVERED: Reload validation
```

**Test Plan**:
```rust
#[test]
fn test_validate_cover_fire() {
    // Valid CoverFire action
    // Verify target exists, in range
}

#[test]
fn test_validate_take_cover() {
    // Valid TakeCover position
    // Verify position is valid cover spot
}

#[test]
fn test_validate_reload() {
    // Valid Reload action
    // Verify cooldown constraints
}
```

---

**Gap 2: Error Cases** (3 tests)
```rust
// UNCOVERED: ToolError variants
// UNCOVERED: ValidationCategory edge cases
// UNCOVERED: ToolVerb enum coverage
```

**Test Plan**:
```rust
#[test]
fn test_tool_error_variants() {
    // Test all ToolError types
    // Verify error messages
}

#[test]
fn test_validation_categories() {
    // Test Range, Cooldown, Resource, Legal categories
    // Verify categorization logic
}

#[test]
fn test_tool_verb_coverage() {
    // Test all ToolVerb enum variants
    // Verify Display impl
}
```

---

**Gap 3: ValidationContext** (2 tests)
```rust
// UNCOVERED: ValidationContext builder
// UNCOVERED: Complex validation scenarios
```

**Test Plan**:
```rust
#[test]
fn test_validation_context_creation() {
    // Build ValidationContext with various inputs
    // Verify all fields populated correctly
}

#[test]
fn test_multi_constraint_validation() {
    // Action that violates multiple constraints
    // Verify all violations reported
}
```

---

### tool_sandbox.rs Test Summary

**New Tests**: 8 tests (3 + 3 + 2)  
**Coverage Gain**: ~40% → 75% (+35pp)  
**Time**: 1.5-2 hours  
**File**: Expand existing `tool_sandbox.rs #[cfg(test)] mod tests`

---

## Priority 4: core_loop.rs (3-5 tests)

**Current**: ~60% coverage (6 tests exist)  
**Target**: 80% coverage  
**Lines**: ~50-70 (measured)  
**Time**: 1-2 hours

### Existing Tests (6 tests in core_loop.rs)

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_planner_mode_classical()        // Classical mode
    
    #[test]
    fn test_planner_mode_behavior_tree()    // BehaviorTree mode
    
    #[test]
    fn test_planner_mode_utility()          // Utility mode
    
    #[test]
    fn test_planner_mode_goap()             // GOAP mode
    
    #[test]
    fn test_planner_mode_hybrid()           // Hybrid mode
    
    #[test]
    fn test_planner_mode_ensemble()         // Ensemble mode
}
```

**Coverage**: dispatch_planner with different PlannerMode variants

### Identified Gaps

**Gap 1: CAiController** (2 tests)
```rust
// UNCOVERED: CAiController struct usage
// UNCOVERED: Controller state management
```

**Test Plan**:
```rust
#[test]
fn test_cai_controller_creation() {
    // Create CAiController with various configs
    // Verify fields initialized correctly
}

#[test]
fn test_cai_controller_state_transitions() {
    // Test controller state changes
    // Verify mode switching
}
```

---

**Gap 2: dispatch_planner Edge Cases** (2 tests)
```rust
// UNCOVERED: Invalid WorldSnapshot handling
// UNCOVERED: Error propagation
```

**Test Plan**:
```rust
#[test]
fn test_dispatch_planner_empty_snapshot() {
    // Empty WorldSnapshot
    // Verify graceful handling
}

#[test]
fn test_dispatch_planner_error_handling() {
    // Orchestrator that returns error
    // Verify error propagates correctly
}
```

---

### core_loop.rs Test Summary

**New Tests**: 4 tests (2 + 2)  
**Coverage Gain**: ~60% → 80% (+20pp)  
**Time**: 1-2 hours  
**File**: Expand existing `core_loop.rs #[cfg(test)] mod tests`

---

## Deferred: Feature-Gated Code (LLM Integration)

### Why Defer?

**Files**:
- ai_arbiter.rs (632 lines)
- llm_executor.rs (450 lines)
- async_task.rs (420 lines)

**Total Deferred**: ~1,502 lines (49% of total code!)

**Reasons to Defer**:
1. ✅ **Feature-gated**: `#[cfg(feature = "llm_orchestrator")]` - not counted in baseline
2. ✅ **Lower priority**: LLM integration is advanced feature, not core AI
3. ✅ **Complex setup**: Requires tokio runtime, async test infrastructure
4. ✅ **Time-intensive**: Would add 8-12 hours for 10-15% coverage gain in measured code

**Current Coverage Impact**: These files don't count toward 46.83% baseline (excluded by cargo tarpaulin when feature disabled)

**Future Work**: Phase 2 (after P1-A complete) can add LLM feature tests if needed

---

## Test Implementation Strategy

### Approach: Incremental Test Files

**File Structure**:
```
astraweave-ai/
├── src/
│   ├── orchestrator.rs (existing inline tests - keep)
│   ├── tool_sandbox.rs (existing inline tests - expand)
│   ├── core_loop.rs (existing inline tests - expand)
│   └── ecs_ai_plugin.rs (existing inline tests - expand)
└── tests/
    ├── orchestrator_extended_tests.rs (NEW - 12 tests)
    └── integration_tests.rs (NEW - 5 tests, optional)
```

**Rationale**:
- Keep existing inline tests (already working, don't break them)
- Add extended tests as separate file (easier to review)
- Integration tests optional (if time allows)

---

### Phase 1: orchestrator_extended_tests.rs (3-4 hours) 🎯

**Priority**: HIGHEST  
**Tests**: 12 tests  
**Coverage Impact**: +25-30pp (biggest file, most gaps)

**Test Categories**:
1. RuleOrchestrator edge cases (3 tests)
2. UtilityOrchestrator scoring (4 tests)
3. GoapOrchestrator planning (3 tests)
4. SystemOrchestratorConfig (2 tests)

**Template**:
```rust
#[cfg(test)]
mod orchestrator_extended {
    use super::*;
    use astraweave_core::*;
    
    fn make_test_snapshot() -> WorldSnapshot {
        // Helper to build WorldSnapshot for tests
        WorldSnapshot {
            t: 1.0,
            player: PlayerState { /* ... */ },
            me: CompanionState { /* ... */ },
            enemies: vec![/* ... */],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        }
    }
    
    #[test]
    fn test_rule_orchestrator_no_enemies() {
        let snap = make_test_snapshot();  // with empty enemies
        let orch = RuleOrchestrator;
        let plan = orch.propose_plan(&snap);
        assert!(plan.steps.is_empty());  // fallback plan
    }
    
    // ... 11 more tests
}
```

---

### Phase 2: Expand Inline Tests (2-3 hours)

**Files**:
1. ecs_ai_plugin.rs (+6 tests)
2. tool_sandbox.rs (+8 tests)
3. core_loop.rs (+4 tests)

**Total**: +18 tests  
**Coverage Impact**: +5-10pp

**Approach**: Add tests to existing `#[cfg(test)] mod tests` blocks

---

### Phase 3: Integration Tests (1-2 hours, OPTIONAL)

**File**: `tests/integration_tests.rs`  
**Tests**: 5 comprehensive integration tests  
**Coverage Impact**: +2-5pp (mostly validating existing code)

**Test Ideas**:
```rust
#[test]
fn test_full_ai_pipeline() {
    // World → Snapshot → Orchestrator → PlanIntent → Validation
}

#[test]
fn test_multi_agent_planning() {
    // Multiple AI agents with different orchestrators
}

#[test]
fn test_ai_ecs_integration() {
    // Full ECS cycle with AI planning plugin
}

#[test]
fn test_error_recovery() {
    // AI errors don't crash engine
}

#[test]
fn test_performance_budget() {
    // Planning completes within time budget
}
```

**Priority**: LOW (only if Phases 1-2 complete ahead of schedule)

---

## Expected Outcomes

### Coverage Targets

| Phase | Tests Added | Est. Coverage | Time |
|-------|-------------|---------------|------|
| **Baseline** | 11 | **46.83%** | - |
| Phase 1 (orchestrator) | +12 | 70-75% | 3-4h |
| Phase 2 (inline tests) | +18 | 78-82% | 2-3h |
| Phase 3 (integration) [OPTIONAL] | +5 | 82-85% | 1-2h |
| **TOTAL** | **+35** | **80-85%** | **5-8h** |

### Success Criteria

**Minimum Viable** (Phase 1 + Phase 2):
- ✅ 30 tests (11 + 12 + 18 = 41 tests)
- ✅ 78-82% coverage (exceeds 75% target)
- ✅ 5-7 hours (within 5-8h estimate)

**Stretch Goal** (All 3 phases):
- ✅ 35 tests (41 + 5 = 46 tests)
- ✅ 82-85% coverage (exceeds 80% target)
- ✅ 6-9 hours

---

## Implementation Checklist

### Pre-Work (0.5 hours)
- [ ] Review astraweave-core::WorldSnapshot API (avoid P0 mistakes)
- [ ] Check astraweave-behavior::GOAP API (for GOAP tests)
- [ ] Verify test helper patterns (reuse from other crates)

### Phase 1: orchestrator_extended_tests.rs (3-4 hours)
- [ ] Create `tests/orchestrator_extended_tests.rs`
- [ ] Add test helper functions (make_test_snapshot, etc.)
- [ ] Write 3 RuleOrchestrator tests
- [ ] Write 4 UtilityOrchestrator tests
- [ ] Write 3 GoapOrchestrator tests
- [ ] Write 2 SystemOrchestratorConfig tests
- [ ] Run `cargo test -p astraweave-ai` (verify all pass)
- [ ] Run tarpaulin (measure coverage gain)

### Phase 2: Inline Test Expansion (2-3 hours)
- [ ] Expand ecs_ai_plugin.rs tests (+6 tests)
- [ ] Expand tool_sandbox.rs tests (+8 tests)
- [ ] Expand core_loop.rs tests (+4 tests)
- [ ] Run `cargo test -p astraweave-ai` (verify all pass)
- [ ] Run tarpaulin (verify 78-82% coverage)

### Phase 3: Integration Tests (1-2 hours, OPTIONAL)
- [ ] Create `tests/integration_tests.rs`
- [ ] Write 5 integration tests
- [ ] Run `cargo test -p astraweave-ai` (verify all pass)
- [ ] Run tarpaulin (verify 82-85% coverage)

### Completion (0.5 hours)
- [ ] Final tarpaulin run (HTML report)
- [ ] Document final coverage metrics
- [ ] Create completion summary
- [ ] Update TODO list

---

## Risk Assessment

### Risk 1: API Uncertainty ⚠️

**Issue**: Exact API for astraweave-behavior GOAP may differ from assumptions

**Mitigation**: 
- Check actual GOAP API before writing tests
- Use grep/read_file to verify method signatures
- Start with simple tests, add complexity after validation

**Impact**: Low (only affects 3 GOAP tests)

---

### Risk 2: ECS Test Complexity ⚠️

**Issue**: ECS integration tests may need complex setup

**Mitigation**:
- Start with simple plugin instantiation tests
- Use existing example code as reference
- Accept lower coverage if ECS tests too complex

**Impact**: Medium (affects 6-8 tests)

---

### Risk 3: Time Overrun ⚠️

**Issue**: 5-8 hour estimate may be optimistic

**Mitigation**:
- Prioritize Phase 1 (highest value)
- Accept 75-78% if Phase 2 takes too long
- Skip Phase 3 if needed (integration tests are optional)

**Impact**: Low (can still meet 75% minimum target with Phase 1+2)

---

## Summary

**Current**: 46.83% coverage, 11 tests  
**Target**: 80% coverage, ~40-45 tests  
**Gap**: +33.17pp, +30-35 tests  
**Time**: 5-8 hours  

**Strategic Approach**:
1. ✅ Focus on core orchestrator logic (highest value, 3-4h)
2. ✅ Expand inline tests for coverage gaps (2-3h)
3. ✅ Skip LLM feature-gated code (deferred to Phase 2)
4. ⚠️ Integration tests optional (only if ahead of schedule)

**Success Definition**: 
- Minimum: 75%+ coverage, 35-40 tests, 5-7 hours
- Target: 80%+ coverage, 40-45 tests, 5-8 hours
- Stretch: 85%+ coverage, 45+ tests, 6-9 hours

**Next Step**: Begin Phase 1 - Create `orchestrator_extended_tests.rs` (3-4 hours)

---

**End of Gap Analysis** | **Status**: Ready for implementation 🚀
