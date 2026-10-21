# Phase 5B Week 5: astraweave-input — Planning Document

**Crate**: `astraweave-input`  
**Start Date**: October 24, 2025  
**Target Duration**: 3 days (Oct 24-26)  
**Status**: 📋 Planning (Ready to start)

---

## Quick Summary

**Target**: 60 tests, 75-85% coverage, 10 hours  
**Strategy**: Apply Week 4's coverage breakthrough patterns  
**Goal**: Maintain 5/5 A+ streak

---

## Week 5 Objectives

### Primary Goals

1. **Test Coverage**: Achieve 75-85% coverage (target: 80%+)
2. **Test Count**: Add 60+ tests (unit, stress, edge, integration)
3. **Pass Rate**: Maintain 100% pass rate
4. **Time Efficiency**: Stay under 10 hours (aim for 6-7h, 30-40% buffer)
5. **Grade**: Achieve 5th consecutive A+ grade

### Success Criteria

| Criterion | Target | Status |
|-----------|--------|--------|
| Tests Added | ≥60 | 📅 Planned |
| Coverage | 75-85% | 📅 Planned |
| Pass Rate | 100% | 📅 Planned |
| Time Budget | ≤10h | 📅 Planned |
| Grade | A+ | 📅 Planned |

---

## Crate Overview: astraweave-input

**Purpose**: Cross-platform input handling for keyboard, mouse, gamepad

**Key Components**:
- Input mapping system (key bindings, rebinding)
- Gamepad support (detection, button mapping)
- Mouse input (position, buttons, wheel)
- Keyboard input (key states, modifiers)
- Input context system (game states with different bindings)

**Potential Challenges**:
1. Platform-specific behavior (Windows, Linux, macOS)
2. Gamepad detection (may need mock devices)
3. Key rebinding edge cases
4. Input event timing/ordering
5. Context switching validation

---

## Week 5 Day-by-Day Plan

### Day 1: Baseline + Unit Tests (2.5-3h)

**Focus**: Establish foundation with core unit tests

**Tasks**:
1. ✅ Run baseline coverage measurement
2. ✅ Identify existing tests (if any)
3. ✅ Create 15-20 unit tests:
   - Input mapping creation/modification
   - Key binding registration
   - Gamepad button mapping
   - Mouse input handling
   - Keyboard state management
   - Context creation/switching

**Expected Coverage**: ~50-60% (baseline)

**Deliverable**: `PHASE_5B_WEEK_5_DAY_1_BASELINE.md`

---

### Day 2: Stress Tests + Edge Cases (3-3.5h)

**Focus**: Validate performance and boundary conditions

**Tasks**:
1. ✅ Create 15-20 stress tests:
   - Rapid input events (1000+ per frame)
   - Many simultaneous inputs (10+ keys pressed)
   - Large binding tables (100+ bindings)
   - Context switching under load
   - Gamepad polling stress

2. ✅ Create 15-20 edge case tests:
   - Invalid key codes
   - Conflicting bindings
   - Missing gamepad devices
   - Context not found
   - Modifier key combinations
   - Mouse out-of-bounds

**Expected Coverage**: ~65-75%

**Deliverable**: `PHASE_5B_WEEK_5_DAY_2_COMPLETE.md`

---

### Day 3: Integration Tests + Benchmarks (2.5-3h)

**Focus**: Cross-component scenarios and performance validation

**Tasks**:
1. ✅ Create 10-15 integration tests:
   - Input pipeline (event → mapping → action)
   - Multi-context gameplay (menu → game → pause)
   - Gamepad + keyboard simultaneous
   - Key rebinding persistence
   - Input recording/playback (if supported)

2. ✅ Create 5-10 benchmarks:
   - Input event processing throughput
   - Binding lookup performance
   - Context switching speed
   - Gamepad polling overhead

3. ✅ Handle potential coverage plateaus:
   - **Apply Week 4 Pattern**: Generate test fixtures programmatically if needed
   - Config files for key bindings (TOML/JSON)
   - Mock gamepad device profiles

**Expected Coverage**: 75-85%+ (breakthrough if plateau detected)

**Deliverable**: `PHASE_5B_WEEK_5_DAY_3_COMPLETE.md`

---

### Day 4: Documentation + Week Summary (1-1.5h)

**Focus**: Consolidate achievements and prepare for Week 6

**Tasks**:
1. ✅ Create comprehensive week summary
2. ✅ Document patterns/lessons learned
3. ✅ Compare to Weeks 1-4
4. ✅ Update Phase 5B status
5. ✅ Create Week 6 plan (astraweave-weaving)

**Deliverable**: `PHASE_5B_WEEK_5_COMPLETE.md`

---

## Patterns to Apply from Week 4

### 1. Coverage Plateau Detection

**Pattern**:
```
If coverage unchanged for 2+ days:
  1. Run `cargo llvm-cov --show-missing -p astraweave-input`
  2. Identify uncovered line clusters
  3. Check for missing test data (config files, device profiles)
  4. Generate missing data programmatically
  5. Re-measure coverage (expect +10-20%)
```

**Application for Week 5**:
- Key binding config files (TOML/JSON)
- Gamepad device profiles (virtual devices)
- Input event sequences (recorded inputs)

### 2. Zero-Dependency Test Fixtures

**Pattern**: Generate test data in pure Rust (no external tools)

**Application for Week 5**:
```rust
// Example: Generate key binding config
fn generate_test_config(path: &Path) -> std::io::Result<()> {
    let config = r#"
[bindings]
move_forward = "W"
move_backward = "S"
jump = "Space"
fire = "Mouse1"
"#;
    std::fs::write(path, config)
}

// Example: Mock gamepad device
struct MockGamepad {
    buttons: [bool; 16],
    axes: [f32; 6],
}
```

### 3. Helper Functions for Test Clarity

**Pattern**: Extract common setup/assertions into helpers

**Application for Week 5**:
```rust
fn create_test_input_system() -> InputSystem { /* ... */ }
fn assert_binding_exists(system: &InputSystem, action: &str, key: &str) { /* ... */ }
fn simulate_key_press(system: &mut InputSystem, key: KeyCode) { /* ... */ }
```

### 4. Pragmatic Solutions Over Perfect

**Pattern**: "Good enough" solutions that pass tests > perfect solutions

**Application for Week 5**:
- Mock gamepad devices (don't need real hardware)
- Simple config files (don't need full asset pipeline)
- Synthetic input events (don't need OS-level input injection)

---

## Expected Challenges & Mitigation

### Challenge 1: Platform-Specific Code

**Issue**: Windows/Linux/macOS differences in input APIs

**Mitigation**:
- Focus on cross-platform abstraction layer
- Use `#[cfg(test)]` to mock platform-specific code
- Test common logic, accept platform differences

### Challenge 2: Gamepad Device Detection

**Issue**: CI/tests may not have physical gamepads

**Mitigation**:
- Create mock gamepad implementations
- Test gamepad API surface (not hardware)
- Use feature flags for optional gamepad tests

### Challenge 3: Input Event Timing

**Issue**: Frame-dependent input processing

**Mitigation**:
- Test event ordering (not exact timing)
- Use deterministic time stepping in tests
- Focus on state correctness, not timing

### Challenge 4: Key Rebinding Persistence

**Issue**: May require file I/O (config loading/saving)

**Mitigation**:
- Generate test config files programmatically (Week 4 pattern)
- Test serialization/deserialization separately
- Use temp directories for test configs

---

## Estimated Time Breakdown

| Day | Focus | Estimated Time | Buffer |
|-----|-------|----------------|--------|
| Day 1 | Baseline + Unit Tests | 2.5-3h | 0.5h |
| Day 2 | Stress + Edge Cases | 3-3.5h | 0.5h |
| Day 3 | Integration + Benchmarks | 2.5-3h | 0.5h |
| Day 4 | Documentation | 1-1.5h | 0.5h |
| **Total** | **All tasks** | **9-11h** | **2h buffer** |

**Target**: Complete in 7-8 hours (30-40% under budget)

---

## Success Metrics

### Week 5 Targets

| Metric | Minimum | Target | Stretch |
|--------|---------|--------|---------|
| **Tests** | 50 | 60 | 70 |
| **Coverage** | 75% | 80% | 85% |
| **Pass Rate** | 100% | 100% | 100% |
| **Time** | <10h | <8h | <7h |
| **Grade** | A | A+ | A+ |

### Phase 5B Overall (After Week 5)

| Metric | Current | After Week 5 | Progress |
|--------|---------|--------------|----------|
| **Tests** | 452/555 (81%) | 512/555 (92%) | +11% |
| **Crates** | 4/7 (57%) | 5/7 (71%) | +14% |
| **Time** | 25.9h/45h (58%) | 33-34h/45h (73-76%) | +15-18% |
| **A+ Rate** | 4/4 (100%) | 5/5 (100%) | Maintained |

---

## Key Documentation to Create

1. **Day 1**: `PHASE_5B_WEEK_5_DAY_1_BASELINE.md` (Baseline + unit tests)
2. **Day 2**: `PHASE_5B_WEEK_5_DAY_2_COMPLETE.md` (Stress + edge cases)
3. **Day 3**: `PHASE_5B_WEEK_5_DAY_3_COMPLETE.md` (Integration + benchmarks)
4. **Day 4**: `PHASE_5B_WEEK_5_COMPLETE.md` (Comprehensive week summary)
5. **Update**: `PHASE_5B_STATUS.md` (Mark Week 5 complete)

---

## Lessons from Week 4 to Remember

1. **Coverage plateaus indicate missing test data** (not bad tests)
2. **Generate test fixtures programmatically** (zero dependencies)
3. **Pragmatic solutions beat perfect solutions** (if tests pass)
4. **Test infrastructure has exponential ROI** (invest early)

---

## Week 6 Preview (astraweave-weaving)

**Target**: 75 tests, 75-85% coverage, 9 hours  
**Focus**: Fate-weaving system (Veilweaver game mechanic)  
**Timeline**: Oct 27-29 (3 days)

---

## Commands for Week 5

```powershell
# Baseline coverage
cargo llvm-cov --lib -p astraweave-input --summary-only

# Run tests
cargo test -p astraweave-input --lib

# Check for missing coverage
cargo llvm-cov --html -p astraweave-input --open --ignore-filename-regex 'tests/'

# Benchmarks (if applicable)
cargo bench -p astraweave-input --bench input_benchmarks
```

---

## Status

**Current**: ✅ Week 4 COMPLETE (A+ grade, 92.34% coverage, 97 tests)  
**Next**: 📋 Week 5 Planning COMPLETE (Ready to start Day 1)  
**Timeline**: Oct 24-26 (3 days planned)  
**Goal**: 5th consecutive A+ grade, maintain 100% A+ rate

---

## Ready to Start!

**Week 5 Day 1**: Baseline measurement + unit tests (2.5-3h)  
**First Task**: Run `cargo llvm-cov --lib -p astraweave-input --summary-only`  
**Expected**: Discover existing test baseline, identify coverage gaps

**Let's maintain the A+ streak!** 🚀

---

**Planning Version**: 1.0  
**Date**: October 23, 2025  
**Status**: ✅ Ready for Week 5 kickoff (Oct 24)
