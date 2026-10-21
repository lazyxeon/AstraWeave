# P1-A Implementation Plan - October 21, 2025

**Phase**: P1-A Improvement Campaign  
**Crates**: astraweave-ecs, astraweave-ai, astraweave-core  
**Current Coverage**: 60.71% average  
**Target Coverage**: 75-85% average  
**Estimated Time**: 5-13 hours

---

## Executive Summary

P1-A contains the **foundational crates** for AstraWeave's AI-native architecture:
- **astraweave-ecs**: ECS engine (70.03% - near target)
- **astraweave-ai**: AI orchestration (46.83% - needs work)
- **astraweave-core**: Core schemas (65.27% - needs work)

**Gap vs Target**: -14.29pp to -24.29pp (depending on 75% or 85% target)

**Strategic Decision**: Focus on **astraweave-ai** (largest gap, highest value) and **astraweave-core** (foundational schemas). Accept ECS at 70% (already "good" tier).

---

## Current State Summary

| Crate | Coverage | Tests | Lines | Test Density | Gap | Priority |
|-------|----------|-------|-------|--------------|-----|----------|
| **astraweave-ecs** | 70.03% | 136 | 734 | 5.4 lines/test | -5 to -15pp | **P3 LOW** |
| **astraweave-ai** | 46.83% | 11 | 331 | 30.1 lines/test | **-28 to -38pp** | **P1 HIGH** |
| **astraweave-core** | 65.27% | 15 | 1,146 | 76.4 lines/test | -10 to -20pp | **P2 MEDIUM** |
| **AVERAGE** | **60.71%** | **162** | **2,211** | **13.7 lines/test** | **-14 to -24pp** | - |

**Key Insight**: Test density is acceptable for ECS (5.4) but too coarse for AI (30.1) and Core (76.4). Need more granular tests.

---

## Improvement Scenarios

### Scenario 1: AI-Only (Minimum Viable)

**Approach**: Improve only astraweave-ai to 80%, leave others as-is

**Effort**: 5-8 hours (AI gap analysis complete)

**Outcome**:
- astraweave-ecs: 70.03% (no change)
- astraweave-ai: 46.83% → **80%** (+33.17pp)
- astraweave-core: 65.27% (no change)
- **Average: 71.77%** (+11.06pp)

**Status**: ✅ **Exceeds 70% industry standard minimum**

---

### Scenario 2: AI + Core (Recommended)

**Approach**: Improve AI to 80% AND Core to 80%, leave ECS as-is

**Effort**: 11-17 hours (5-8h AI + 6.5-9h Core)

**Outcome**:
- astraweave-ecs: 70.03% (no change)
- astraweave-ai: 46.83% → **80%** (+33.17pp)
- astraweave-core: 65.27% → **80%** (+14.73pp)
- **Average: 76.68%** (+15.97pp)

**Status**: ✅ **Exceeds 75% P1-A target**

---

### Scenario 3: All Three (Stretch Goal)

**Approach**: Improve AI to 80%, Core to 80%, AND ECS to 80%

**Effort**: 13-20 hours (5-8h AI + 6.5-9h Core + 2-3h ECS)

**Outcome**:
- astraweave-ecs: 70.03% → **80%** (+9.97pp)
- astraweave-ai: 46.83% → **80%** (+33.17pp)
- astraweave-core: 65.27% → **80%** (+14.73pp)
- **Average: 80%** (+19.29pp)

**Status**: ✅ **Meets 80% excellent tier**

---

## Recommended Strategy: Scenario 2 (AI + Core)

**Rationale**:
1. ✅ AI has largest gap (46.83% → 80% = +33pp)
2. ✅ Core has foundational data structures (WorldSnapshot, PlanIntent, ActionStep)
3. ✅ ECS already "good" tier (70% is acceptable for infrastructure)
4. ✅ Time efficient (11-17h vs 13-20h for marginal +3.32pp)

**Time**: 11-17 hours  
**Tests**: +61-71 tests (AI +24-34, Core +37)  
**Coverage**: 76.68% average (exceeds 75% target)

---

## Detailed Plans by Crate

### 1. astraweave-ai (5-8 hours)

**Current**: 46.83%, 11 tests  
**Target**: 80%  
**Gap**: +33.17pp  
**Tests Needed**: +24-34 tests

**📄 Full Analysis**: `AI_GAP_ANALYSIS_OCT_21_2025.md` (6,000 words)

#### Priority 1: orchestrator.rs (3-4 hours)
- **Tests**: +10-15 tests
- **Categories**: RuleOrchestrator (3), UtilityOrchestrator (4), GoapOrchestrator (3), Config (2)
- **File**: Create `tests/orchestrator_extended_tests.rs`

#### Priority 2: ecs_ai_plugin.rs (2-3 hours)
- **Tests**: +6-8 tests
- **Categories**: Plugin registration (2), build_app_with_ai (3), System functions (2)
- **File**: Expand inline tests in `src/ecs_ai_plugin.rs`

#### Priority 3: tool_sandbox.rs (1.5-2 hours)
- **Tests**: +5-8 tests
- **Categories**: Action types (3), Error cases (3), ValidationContext (2)
- **File**: Expand inline tests in `src/tool_sandbox.rs`

#### Priority 4: core_loop.rs (1-2 hours)
- **Tests**: +3-5 tests
- **Categories**: CAiController (2), dispatch_planner (2)
- **File**: Expand inline tests in `src/core_loop.rs`

#### Deferred: Feature-Gated LLM Code
- **Lines**: 1,502 (49% of codebase)
- **Files**: ai_arbiter.rs, llm_executor.rs, async_task.rs
- **Reason**: Saves 8-12 hours, already measured separately
- **Future**: Phase 2 or later

---

### 2. astraweave-core (6.5-9 hours)

**Current**: 65.27%, 15 tests  
**Target**: 80%  
**Gap**: +14.73pp  
**Tests Needed**: +37 tests (revised from +15-25)

**📄 Full Analysis**: `CORE_GAP_ANALYSIS_OCT_21_2025.md` (5,000 words)

#### Phase 1: schema_tests.rs (2.5-3.5 hours) 🎯
- **Tests**: +12 tests
- **Categories**: WorldSnapshot (3), CompanionState (2), PlanIntent (2), ActionStep (5)
- **File**: Create `tests/schema_tests.rs`
- **Impact**: +25-30pp (largest gain)

#### Phase 2: validation.rs Expansion (2-2.5 hours)
- **Tests**: +9 tests
- **Categories**: Tool validation (3), Error types (3), Complex scenarios (3)
- **File**: Expand existing `src/validation.rs #[cfg(test)] mod tests`
- **Impact**: +10-15pp

#### Phase 3: Small File Tests (2-3 hours)
- **Tests**: +16 tests
- **Files**: tools.rs (4), perception.rs (3), world.rs (2), ecs_* (5), capture_replay.rs (2)
- **Impact**: +5-10pp

**Note**: Core estimate revised from 3-5h to 6.5-9h due to:
- schema.rs larger than expected (426 lines, 0% coverage)
- 11 files have ZERO tests (73% of files untested)

**Minimum Viable**: Phases 1-2 only (76-79% coverage, 4.5-6h)

---

### 3. astraweave-ecs (DEFERRED - Optional)

**Current**: 70.03%, 136 tests  
**Target**: 80% (stretch)  
**Gap**: +9.97pp  
**Tests Needed**: +20-30 tests (estimated)  
**Time**: 2-3 hours

**Rationale for Deferring**:
- Already at "good" tier (60-70% is industry standard)
- Only 5-15pp below P1-A target (marginal improvement)
- ECS has 136 tests already (comprehensive suite)
- Time better spent on AI/Core (larger gaps)

**If Pursued** (Scenario 3):
- Focus on error handling, edge cases
- Add integration tests (multi-system scenarios)
- Test archetype transitions, entity lifecycle

---

## Implementation Sequence

### Week 1: astraweave-ai (5-8 hours)

**Day 1-2: orchestrator_extended_tests.rs** (3-4h)
```bash
# Create new test file
touch astraweave-ai/tests/orchestrator_extended_tests.rs

# Write 12 tests (RuleOrchestrator, UtilityOrchestrator, GoapOrchestrator)

# Validate
cargo test -p astraweave-ai --test orchestrator_extended_tests
cargo tarpaulin -p astraweave-ai --lib --tests
```

**Day 3: ecs_ai_plugin.rs expansion** (2-3h)
```bash
# Add 6-8 tests to existing inline tests

# Validate
cargo test -p astraweave-ai
cargo tarpaulin -p astraweave-ai --lib
```

**Day 4: tool_sandbox.rs + core_loop.rs** (2.5-4h)
```bash
# Add 8-13 tests total (5-8 + 3-5)

# Final validation
cargo test -p astraweave-ai
cargo tarpaulin -p astraweave-ai --lib --tests --out Html --output-dir coverage/ai_final/
```

**Target**: 80% coverage, 35-45 tests, 5-8 hours

---

### Week 2: astraweave-core (6.5-9 hours)

**Day 1-2: schema_tests.rs** (2.5-3.5h) 🎯
```bash
# Create new test file
touch astraweave-core/tests/schema_tests.rs

# Write 12 tests (WorldSnapshot, CompanionState, PlanIntent, ActionStep)

# Validate
cargo test -p astraweave-core --test schema_tests
cargo tarpaulin -p astraweave-core --lib --tests
```

**Day 3: validation.rs expansion** (2-2.5h)
```bash
# Add 9 tests to existing inline tests

# Validate
cargo test -p astraweave-core
cargo tarpaulin -p astraweave-core --lib --tests
```

**Day 4: Small file tests** (2-3h) - OPTIONAL
```bash
# Add 16 tests across 7 files
# tools.rs (4), perception.rs (3), world.rs (2), ecs_* (5), capture_replay.rs (2)

# Final validation
cargo test -p astraweave-core
cargo tarpaulin -p astraweave-core --lib --tests --out Html --output-dir coverage/core_final/
```

**Minimum Target**: 76-79% coverage, 36 tests, 4.5-6 hours  
**Stretch Target**: 80-83% coverage, 52 tests, 6.5-9 hours

---

## Timeline Estimates

| Scenario | Crates | Time | Coverage | Tests Added | Status |
|----------|--------|------|----------|-------------|--------|
| **Baseline** | - | - | 60.71% | 0 | Current |
| **Scenario 1** | AI only | 5-8h | **71.77%** | +24-34 | ✅ Min viable |
| **Scenario 2** | AI + Core | **11-17h** | **76.68%** | **+61-71** | ✅ **RECOMMENDED** |
| **Scenario 3** | All three | 13-20h | 80% | +81-101 | Stretch goal |

**Recommended**: Scenario 2 (11-17 hours over 2 weeks)

---

## Success Criteria

### Minimum Success (Scenario 1)
- ✅ astraweave-ai: 80% coverage
- ✅ P1-A average: 71.77% (exceeds 70% industry standard)
- ✅ Time: 5-8 hours (under budget)

### Target Success (Scenario 2) ⭐
- ✅ astraweave-ai: 80% coverage
- ✅ astraweave-core: 80% coverage
- ✅ P1-A average: 76.68% (exceeds 75% target)
- ✅ Time: 11-17 hours (reasonable)

### Stretch Success (Scenario 3)
- ✅ All three crates: 80% coverage
- ✅ P1-A average: 80% (excellent tier)
- ✅ Time: 13-20 hours (high effort)

---

## Risk Mitigation

### Risk 1: Time Overrun ⚠️
**Issue**: Core revised from 3-5h to 6.5-9h (schema complexity)

**Mitigation**:
- Accept Core at 76-79% with Phases 1-2 only (4.5-6h)
- Skip Phase 3 small files if needed
- Total: 9.5-14h (vs 11-17h estimate)

**Impact**: Still exceeds 75% target with buffer

---

### Risk 2: Feature-Gated Code Confusion ⚠️
**Issue**: AI has 49% feature-gated code excluded from baseline

**Mitigation**:
- Clear separation in gap analysis
- Focus only on core orchestrator logic
- Document deferred work (1,502 lines for Phase 2)

**Impact**: Low (already handled in analysis)

---

### Risk 3: Test Complexity ⚠️
**Issue**: Some tests may be harder than estimated

**Mitigation**:
- Start with simplest tests (data structure creation)
- Add complexity incrementally
- Accept lower test count if complex (e.g., 30 tests instead of 35)

**Impact**: Medium (may reduce final coverage by 2-5pp)

---

## Quality Gates

### Per-Crate Gates

**astraweave-ai**:
- ✅ Minimum: 35 tests, 78% coverage
- ✅ Target: 40 tests, 80% coverage
- ✅ No new warnings
- ✅ All tests pass

**astraweave-core**:
- ✅ Minimum: 36 tests, 76% coverage (Phases 1-2)
- ✅ Target: 52 tests, 80% coverage (All phases)
- ✅ No new warnings
- ✅ All tests pass

### Overall P1-A Gates

- ✅ Minimum: 71.77% average (Scenario 1)
- ✅ Target: 76.68% average (Scenario 2)
- ✅ Stretch: 80% average (Scenario 3)
- ✅ Zero test failures
- ✅ Zero clippy errors

---

## Documentation Requirements

### Per-Crate Completion Reports

**Template** (already created for P0):
```markdown
# [Crate Name] Test Improvement Complete - [Date]

## Coverage Summary
- Before: X%
- After: Y%
- Improvement: +Zpp

## Tests Added
- Category 1: N tests
- Category 2: M tests
- Total: K tests

## Time Spent
- Estimated: Ah
- Actual: Bh
- Variance: Ch

## Lessons Learned
- [Lesson 1]
- [Lesson 2]

## Next Steps
- [Next crate or phase]
```

### P1-A Campaign Summary

**Final Report** (to be created):
- `P1A_CAMPAIGN_COMPLETE_OCT_21_2025.md`
- All three crates summarized
- Comparison to P0 campaign
- Lessons learned
- Transition to P1-B

---

## Next Steps

### Immediate (Today)
1. ✅ Complete Core gap analysis ← **DONE**
2. ✅ Create implementation plan ← **DONE**
3. ⏳ User approval for Scenario 2
4. ⏳ Begin Week 1 Day 1: orchestrator_extended_tests.rs

### This Week (Week 1 - AI)
- [ ] Day 1-2: orchestrator_extended_tests.rs (12 tests)
- [ ] Day 3: ecs_ai_plugin.rs expansion (6-8 tests)
- [ ] Day 4: tool_sandbox + core_loop (8-13 tests)
- [ ] Day 5: AI completion report + tarpaulin HTML

### Next Week (Week 2 - Core)
- [ ] Day 1-2: schema_tests.rs (12 tests) 🎯
- [ ] Day 3: validation.rs expansion (9 tests)
- [ ] Day 4: Small file tests (16 tests) - OPTIONAL
- [ ] Day 5: Core completion report + P1-A summary

---

## Reference Documents

**Created Today** (October 21, 2025):
1. `COMPLETE_CODEBASE_COVERAGE_ANALYSIS_OCT_21_2025.md` - Overall 47-crate roadmap
2. `P1A_CRATES_MEASUREMENT_COMPLETE_OCT_21_2025.md` - P1-A baseline measurements
3. `EXECUTIVE_SUMMARY_CODEBASE_COVERAGE_OCT_21_2025.md` - High-level overview
4. `AI_GAP_ANALYSIS_OCT_21_2025.md` - Detailed AI test plan (35 specs)
5. `CORE_GAP_ANALYSIS_OCT_21_2025.md` - Detailed Core test plan (37 specs)
6. `P1A_IMPLEMENTATION_PLAN_OCT_21_2025.md` - This document

**P0 Campaign Reference**:
- `P0_CAMPAIGN_COMPLETE.md` - P0 completion summary (86.85% average)
- 5 individual crate completion reports

**Strategic Planning**:
- `COMPREHENSIVE_STRATEGIC_ANALYSIS.md` - 12-month roadmap
- `LONG_HORIZON_STRATEGIC_PLAN.md` - Phased approach
- `IMPLEMENTATION_PLANS_INDEX.md` - Navigation guide

---

## Summary

**Current State**: P1-A at 60.71% average (8/47 crates measured)

**Target State**: P1-A at 76.68% average (exceeds 75% target)

**Recommended Approach**: Scenario 2 (AI + Core improvement)
- ✅ astraweave-ai: 46.83% → 80% (+33.17pp, 5-8h)
- ✅ astraweave-core: 65.27% → 80% (+14.73pp, 6.5-9h)
- ⚠️ astraweave-ecs: 70.03% (no change, already good)

**Timeline**: 11-17 hours over 2 weeks

**Tests**: +61-71 tests (AI +24-34, Core +37)

**Status**: Ready for implementation 🚀

---

**End of Implementation Plan** | **Next**: User approval for Scenario 2
