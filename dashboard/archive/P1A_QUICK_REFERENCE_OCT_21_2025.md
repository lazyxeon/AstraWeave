# P1-A Quick Reference - October 21, 2025

**Status**: Ready for implementation  
**Phase**: P1-A Gap Analysis COMPLETE  
**Next**: Begin test implementation

---

## At a Glance

| Metric | Current | Target | Gap | Time |
|--------|---------|--------|-----|------|
| **P1-A Average Coverage** | 60.71% | 76.68% | +15.97pp | 11-17h |
| **astraweave-ecs** | 70.03% | 70% | ✅ Accept | 0h |
| **astraweave-ai** | 46.83% | 80% | +33.17pp | 5-8h |
| **astraweave-core** | 65.27% | 80% | +14.73pp | 6.5-9h |
| **Tests Added** | 162 | +61-71 | 223-233 total | - |

---

## Recommended Strategy: Scenario 2 (AI + Core)

**Why**:
- ✅ AI has largest gap (46.83% → 80%)
- ✅ Core has foundational schemas (WorldSnapshot, PlanIntent)
- ✅ ECS already "good" tier (70% acceptable)
- ✅ Time efficient (11-17h vs 13-20h for +3pp)

**Outcome**: 76.68% average (exceeds 75% P1-A target)

---

## Implementation Timeline

### Week 1: astraweave-ai (5-8 hours)

**Day 1-2**: orchestrator_extended_tests.rs (12 tests, 3-4h)  
**Day 3**: ecs_ai_plugin.rs expansion (6-8 tests, 2-3h)  
**Day 4**: tool_sandbox + core_loop (8-13 tests, 2.5-4h)  
**Target**: 80% coverage, 35-45 tests

### Week 2: astraweave-core (6.5-9 hours)

**Day 1-2**: schema_tests.rs (12 tests, 2.5-3.5h) 🎯 HIGHEST VALUE  
**Day 3**: validation.rs expansion (9 tests, 2-2.5h)  
**Day 4**: Small files (16 tests, 2-3h) - OPTIONAL  
**Minimum**: 76-79% coverage, 36 tests (4.5-6h)  
**Target**: 80-83% coverage, 52 tests (6.5-9h)

---

## Test Priorities by File

### astraweave-ai (35 tests specified)

| Priority | File | Tests | Time | Impact |
|----------|------|-------|------|--------|
| **P1** | orchestrator.rs | +10-15 | 3-4h | Highest |
| **P1** | ecs_ai_plugin.rs | +6-8 | 2-3h | High |
| **P2** | tool_sandbox.rs | +5-8 | 1.5-2h | Medium |
| **P3** | core_loop.rs | +3-5 | 1-2h | Low |
| **Defer** | LLM feature-gated | 0 | 0h | (1,502 lines) |

**Documentation**: `AI_GAP_ANALYSIS_OCT_21_2025.md` (6,000 words, 35 test specs)

---

### astraweave-core (37 tests specified)

| Priority | File/Phase | Tests | Time | Impact |
|----------|------------|-------|------|--------|
| **P1** | schema_tests.rs | +12 | 2.5-3.5h | Highest (+25-30pp) |
| **P2** | validation.rs | +9 | 2-2.5h | High (+10-15pp) |
| **P3** | Small files | +16 | 2-3h | Low (+5-10pp) |

**Documentation**: `CORE_GAP_ANALYSIS_OCT_21_2025.md` (5,000 words, 37 test specs)

---

## Key Commands

### Validate AI Changes
```powershell
# Run tests
cargo test -p astraweave-ai

# Measure coverage
cargo tarpaulin -p astraweave-ai --lib --tests --out Html --output-dir coverage/ai_final/

# Check quality
cargo fmt -p astraweave-ai
cargo clippy -p astraweave-ai --all-features -- -D warnings
```

### Validate Core Changes
```powershell
# Run tests
cargo test -p astraweave-core

# Measure coverage
cargo tarpaulin -p astraweave-core --lib --tests --out Html --output-dir coverage/core_final/

# Check quality
cargo fmt -p astraweave-core
cargo clippy -p astraweave-core --all-features -- -D warnings
```

### Validate P1-A Overall
```powershell
# Run all P1-A tests
cargo test -p astraweave-ecs -p astraweave-ai -p astraweave-core

# Measure combined (manual average from individual reports)
```

---

## Success Criteria

### Minimum Success (AI only - Scenario 1)
- ✅ astraweave-ai: 80% coverage (35 tests)
- ✅ P1-A average: 71.77%
- ✅ Time: 5-8 hours

### Target Success (AI + Core - Scenario 2) ⭐
- ✅ astraweave-ai: 80% coverage (35-45 tests)
- ✅ astraweave-core: 80% coverage (36-52 tests)
- ✅ P1-A average: 76.68%
- ✅ Time: 11-17 hours

### Stretch Success (All three - Scenario 3)
- ✅ All crates: 80% coverage
- ✅ P1-A average: 80%
- ✅ Time: 13-20 hours

---

## Risk Summary

| Risk | Mitigation | Impact |
|------|-----------|--------|
| **Time overrun** | Accept Core 76-79% (skip Phase 3) | Medium |
| **Feature-gated confusion** | Clear separation in docs, focus on core | Low |
| **Test complexity** | Start simple, add incrementally | Medium |

---

## Documentation References

**Gap Analyses** (Created today):
1. `AI_GAP_ANALYSIS_OCT_21_2025.md` - 6,000 words, 35 test specs
2. `CORE_GAP_ANALYSIS_OCT_21_2025.md` - 5,000 words, 37 test specs

**Implementation Plan**:
3. `P1A_IMPLEMENTATION_PLAN_OCT_21_2025.md` - Comprehensive roadmap

**Baseline Measurements**:
4. `P1A_CRATES_MEASUREMENT_COMPLETE_OCT_21_2025.md` - Current state

**Strategic Context**:
5. `COMPLETE_CODEBASE_COVERAGE_ANALYSIS_OCT_21_2025.md` - 47-crate roadmap
6. `EXECUTIVE_SUMMARY_CODEBASE_COVERAGE_OCT_21_2025.md` - High-level overview

---

## Next Actions

### Immediate
1. ✅ Gap analyses complete (AI + Core)
2. ✅ Implementation plan complete
3. ⏳ **User approval for Scenario 2**
4. ⏳ Begin Week 1 Day 1: orchestrator_extended_tests.rs

### This Week (AI)
- [ ] Create `astraweave-ai/tests/orchestrator_extended_tests.rs`
- [ ] Write 12 tests (RuleOrchestrator, UtilityOrchestrator, GoapOrchestrator)
- [ ] Expand ecs_ai_plugin.rs inline tests (+6-8)
- [ ] Expand tool_sandbox.rs + core_loop.rs (+8-13)
- [ ] Run tarpaulin, verify 80% coverage
- [ ] Create AI completion report

### Next Week (Core)
- [ ] Create `astraweave-core/tests/schema_tests.rs`
- [ ] Write 12 tests (WorldSnapshot, ActionStep, etc.)
- [ ] Expand validation.rs inline tests (+9)
- [ ] Optional: Small file tests (+16)
- [ ] Run tarpaulin, verify 76-83% coverage
- [ ] Create Core completion report
- [ ] Create P1-A campaign summary

---

## Quick Wins Identified

### AI Quick Wins (orchestrator.rs)
- `test_rule_orchestrator_no_enemies()` - Edge case for empty enemies
- `test_goap_no_valid_plan()` - Error handling for impossible goals
- `test_utility_morale_effect()` - Morale scoring verification

### Core Quick Wins (schema_tests.rs)
- `test_world_snapshot_creation()` - Basic data structure validation
- `test_action_step_move_to()` - Enum variant pattern matching
- `test_companion_state_default()` - Default values verification

**Estimated Time for Quick Wins**: 1-2 hours (6-9 tests)  
**Estimated Coverage Gain**: +5-8pp combined

---

## P1-A vs P0 Comparison

| Metric | P0 | P1-A Current | P1-A Target | Delta |
|--------|----|--------------|-----------------|-------|
| **Avg Coverage** | 86.85% | 60.71% | 76.68% | -10.17pp |
| **Tests** | 301 | 162 | 223-233 | +61-71 |
| **Time Spent** | 11.5h | 0.5h | 11.5-17.5h | +11-17h |
| **Crates** | 5 | 3 | 3 | Same |

**Insight**: P1-A has lower baseline (60.71% vs 86.85%) but still achievable with focused effort.

---

## Campaign Progress

| Phase | Crates | Status | Coverage | Time |
|-------|--------|--------|----------|------|
| **P0** | 5 | ✅ COMPLETE | 86.85% | 11.5h |
| **P1-A** | 3 | ⏳ IN PROGRESS | 60.71% → 76.68% | 11-17h |
| **P1-B** | 4 | ❓ PENDING | Unknown | TBD |
| **P1-C** | 5 | ❓ PENDING | Unknown | TBD |
| **P1-D** | 3 | ❓ PENDING | Unknown | TBD |
| **P2** | 12 | ❓ PENDING | Unknown | TBD |
| **P3** | 15 | ❓ PENDING | Unknown | TBD |
| **TOTAL** | **47** | **17% measured** | **76.59%*** | **12-28.5h** |

*Combined P0+P1-A average where measured

---

## Frequently Asked Questions

**Q: Why skip ECS if it's below target?**  
A: 70% is "good" tier (60-70% industry standard). Only 5-15pp below target, not worth 2-3 hours when AI/Core have larger gaps.

**Q: Why defer LLM feature-gated code?**  
A: 1,502 lines (49% of AI crate) saves 8-12 hours. Can be addressed in Phase 2 when LLM integration is more stable.

**Q: What if Core takes longer than 6.5-9h?**  
A: Accept 76-79% with Phases 1-2 only (4.5-6h). Still exceeds 75% target.

**Q: When do we move to P1-B?**  
A: After P1-A campaign complete (AI + Core at 80%, documentation done, ~2 weeks).

**Q: How do we measure P1-A average coverage?**  
A: Manual average of 3 final crate coverages: `(70.03 + 80 + 80) / 3 = 76.68%`

---

**Status**: Ready for implementation 🚀  
**Next**: User approval, then begin orchestrator_extended_tests.rs
