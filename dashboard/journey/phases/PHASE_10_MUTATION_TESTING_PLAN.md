# Phase 10: Mutation Testing - Comprehensive Test Quality Validation

**Version**: 1.1  
**Date**: January 20, 2026  
**Status**: 🎯 IN PROGRESS - First P0 crate EXCEPTIONAL (94.37%), second crate running  
**Previous Phase**: Phase 9 (Bulletproof Validation Complete - 94.57% coverage, 25/25 crates)

**Progress**: 1/12 P0 crates complete ✅ (astraweave-math 94.37%), 1/12 in progress 🎯 (astraweave-nav 295 mutants)

---

## Mission Statement

**Objective**: Validate that AstraWeave's comprehensive test suite (2,189+ tests, 94.57% coverage) effectively catches bugs through **mutation testing** - the gold standard for test quality measurement.

**Why Mutation Testing?**
- **Code coverage measures what code is executed**, not whether tests detect bugs
- **Mutation testing measures test effectiveness** by deliberately introducing bugs (mutants)
- **Industry insight**: 80% coverage doesn't guarantee good tests; mutation testing does
- **AstraWeave's challenge**: Prove that 94.57% coverage translates to 80%+ mutation score (world-class)

---

## What is Mutation Testing?

### Core Concept

**Mutation testing** introduces small, deliberate bugs (mutations) into source code, then runs the test suite to see if tests detect the bugs.

**Example**:
```rust
// Original code
fn add(a: i32, b: i32) -> i32 {
    a + b  // This is correct
}

// Mutant 1: Changed + to -
fn add(a: i32, b: i32) -> i32 {
    a - b  // BUG! Tests should fail here
}

// Mutant 2: Changed return to 0
fn add(a: i32, b: i32) -> i32 {
    0  // BUG! Tests should fail here
}
```

**Test Quality Assessment**:
- ✅ **Mutant Killed**: Test suite detects bug (test fails) → Good test!
- ❌ **Mutant Survived**: Bug goes undetected (test passes) → Weak test!
- ⏱️ **Mutant Timeout**: Test runs forever → Infinite loop introduced

### Mutation Score

**Formula**: `Mutation Score = Killed Mutants / Total Mutants`

**Industry Benchmarks**:
- **60-70%**: Typical (average test quality)
- **70-80%**: Good (above-average test quality)
- **80-90%**: Excellent (world-class test quality)
- **90%+**: Exceptional (rare, requires extreme test discipline)

**AstraWeave Target**: **80%+ mutation score** (world-class, matching our 94.57% coverage excellence)

---

## cargo-mutants Tool

### Why cargo-mutants?

**Selected Tool**: cargo-mutants v26.1.2  
**Alternatives Considered**: mutagen (experimental), llvm-based (complex)

**Advantages**:
1. **Mature & Active**: 2+ years development, 500+ stars, active maintenance
2. **Zero Configuration**: Works out-of-box with cargo test
3. **Clear Output**: Generates mutation testing report with survived mutants highlighted
4. **Performance**: Parallel mutation testing (multi-core utilization)
5. **Integration**: Works with existing test infrastructure (no code changes needed)

### How cargo-mutants Works

**Process** (per crate):
1. **Analyze Source**: Identify mutation points (operators, returns, literals)
2. **Generate Mutants**: Create variations (e.g., `+` → `-`, `true` → `false`)
3. **Test Each Mutant**: Run `cargo test -p <crate>` with mutated code
4. **Classify Results**:
   - ✅ **Killed**: Test failed (mutant detected)
   - ❌ **Survived**: Test passed (mutant undetected, **weak spot!**)
   - ⏱️ **Timeout**: Test hung (infinite loop mutant)
   - ⚠️ **Build Failed**: Mutant created invalid Rust (skipped)

**Output**: `mutants.out/` directory with detailed report

---

## Phase 10 Roadmap

### Scope: 25 Crates (3 Priority Tiers)

**Estimated Duration**: 6-10 hours (varies by crate complexity and test count)

### Priority Order

**Tier 1: P0 Critical Infrastructure (12 crates)**
1. astraweave-core (269 tests, 95.24% coverage) — **2-3 hours**
2. astraweave-ecs (213 tests, 96.88% coverage) — **2-3 hours**
3. astraweave-physics (355 tests, 96.68% coverage) — **3-4 hours**
4. astraweave-math (34 tests, 98.07% coverage) — **30 min**
5. astraweave-render (369 tests, ~85% coverage) — **4-6 hours** (GPU complexity)
6. astraweave-asset (156 tests, 98.20% coverage) — **1-2 hours**
7. astraweave-terrain (~265 tests, 89.32% coverage) — **2-3 hours**
8. astraweave-gameplay (231 tests, 95.06% coverage) — **2-3 hours**
9. astraweave-audio (81 tests, 91.42% coverage) — **1-2 hours**
10. astraweave-nav (65 tests, 94.66% coverage) — **1 hour**
11. astraweave-scene (81 tests, 83.21% coverage) — **1-2 hours**
12. astraweave-ui (206 tests, 86.74% coverage) — **2-3 hours**

**Tier 2: P1 Important Support (5 crates)**
1. astraweave-ai (103 tests, 87.42% coverage) — **1-2 hours**
2. astraweave-cinematics (2 tests, 99.44% coverage) — **15 min**
3. astraweave-weaving (64 tests, 94.26% coverage) — **1 hour**
4. astraweave-materials (3 tests, 90.11% coverage) — **15 min**
5. aw_editor (71 tests, ~95% coverage) — **1-2 hours**

**Tier 3: P2 Support Systems (8 crates)**
1. astraweave-embeddings (30 tests, 98.23% coverage) — **30 min**
2. astraweave-memory (81 tests, 97.16% coverage) — **1 hour**
3. astraweave-behavior (57 tests, 96.65% coverage) — **1 hour**
4. astraweave-input (59 tests, 95.45% coverage) — **1 hour**
5. astraweave-pcg (19 tests, 93.46% coverage) — **30 min**
6. astraweave-scripting (~30 tests, 88.04% coverage) — **1 hour**
7. astraweave-security (38 tests, 79.18% coverage) — **1 hour**
8. astraweave-llm (587 tests, 78.40% coverage) — **2-3 hours** (largest test suite)

### Execution Strategy

**Phase 10A: P0 Critical Path (Days 1-2)**
- Focus: 12 P0 crates (highest priority)
- Goal: Validate critical infrastructure test quality
- Time: 18-30 hours estimated
- Success: 80%+ average mutation score across P0 tier

**Phase 10B: P1 Important (Day 3)**
- Focus: 5 P1 crates
- Goal: Validate AI, cinematics, editor test quality
- Time: 4-6 hours estimated
- Success: 75%+ average mutation score across P1 tier

**Phase 10C: P2 Support (Day 4)**
- Focus: 8 P2 crates
- Goal: Validate support systems test quality
- Time: 7-10 hours estimated
- Success: 70%+ average mutation score across P2 tier

**Phase 10D: Analysis & Remediation (Day 5)**
- Focus: Survived mutants analysis
- Goal: Identify weak test spots, add targeted tests
- Time: 4-6 hours estimated
- Success: Fix top 10 most critical survived mutants

---

## Mutation Testing Commands

### Basic Mutation Test (Single Crate)

```powershell
# Test a single crate (generates mutants.out/ report)
cargo mutants --package astraweave-math

# With parallel execution (faster, uses all CPU cores)
cargo mutants --package astraweave-math --jobs 8

# With timeout (prevent infinite loop mutants from blocking)
cargo mutants --package astraweave-math --timeout 60
```

### Filtered Mutation Test (Specific Files)

```powershell
# Test only specific file (e.g., core.rs)
cargo mutants --package astraweave-core --file src/core.rs

# Test only specific function
cargo mutants --package astraweave-ecs --file src/archetype.rs --regex "fn spawn_entity"
```

### Report Generation

```powershell
# Generate detailed mutation report
cargo mutants --package astraweave-math --output mutants.out/math/

# Generate JSON report (for automation)
cargo mutants --package astraweave-math --json > math_mutants.json
```

### Analysis Commands

```powershell
# View survived mutants (weak test spots)
Get-Content mutants.out/math/survived.txt

# Count mutation score
$killed = (Get-Content mutants.out/math/killed.txt).Count
$survived = (Get-Content mutants.out/math/survived.txt).Count
$score = [math]::Round(100 * $killed / ($killed + $survived), 2)
Write-Host "Mutation Score: $score%"
```

---

## Success Criteria

### Phase 10 Overall

✅ **All 25 crates mutation tested** (100% production codebase coverage)  
✅ **Overall mutation score ≥ 75%** (exceeds industry "good" threshold)  
✅ **P0 mutation score ≥ 80%** (world-class critical infrastructure)  
✅ **Top 10 survived mutants analyzed** (weak spots documented)  
✅ **Comprehensive mutation testing report** (per-crate scores, analysis, remediation plan)

### Per-Crate Success Criteria

**P0 (Critical)**:
- ✅ Mutation score ≥ 80% (world-class)
- ✅ Zero timeout mutants (no infinite loops)
- ✅ All survived mutants documented with remediation plan

**P1 (Important)**:
- ✅ Mutation score ≥ 75% (excellent)
- ✅ Critical survived mutants fixed (if any)

**P2 (Support)**:
- ✅ Mutation score ≥ 70% (good)
- ✅ Major survived mutants documented

---

## Expected Challenges

### Known Challenges from Industry Experience

1. **GPU Code (astraweave-render)**:
   - **Issue**: GPU shaders/tests may timeout or fail to mutate
   - **Mitigation**: Use `--timeout 120`, exclude GPU-only code with `--exclude`

2. **Async Code (astraweave-llm, astraweave-scene)**:
   - **Issue**: Async mutants may introduce race conditions or timeouts
   - **Mitigation**: Increase timeout, use `--test-threads=1` if needed

3. **Large Test Suites (astraweave-llm: 587 tests)**:
   - **Issue**: Mutation testing takes hours (each mutant reruns ALL tests)
   - **Mitigation**: Use `--jobs` for parallelism, run overnight if needed

4. **Integration Tests (cross-crate)**:
   - **Issue**: Mutants in crate A may break tests in crate B
   - **Mitigation**: Run per-crate mutation tests (isolate to unit/lib tests)

5. **Flaky Tests (rare)**:
   - **Issue**: Non-deterministic tests may randomly kill/survive mutants
   - **Mitigation**: Re-run flaky mutants, fix test determinism first

---

## Deliverables

### Phase 10 Reports (5 documents)

1. **PHASE_10_MUTATION_TESTING_PLAN.md** (this document) — ✅ COMPLETE
   - Roadmap, tool selection, commands, success criteria

2. **PHASE_10A_P0_MUTATION_RESULTS.md** (pending) — 🎯 NEXT
   - P0 crate mutation scores (12 crates)
   - Survived mutants analysis
   - Weak test spots identification
   - Remediation recommendations

3. **PHASE_10B_P1_MUTATION_RESULTS.md** (pending)
   - P1 crate mutation scores (5 crates)
   - AI/cinematics/editor test quality validation

4. **PHASE_10C_P2_MUTATION_RESULTS.md** (pending)
   - P2 crate mutation scores (8 crates)
   - LLM/security/embeddings test quality validation

5. **PHASE_10_MUTATION_TESTING_COMPLETE.md** (pending)
   - Overall mutation score (25/25 crates)
   - Top 10 survived mutants (critical weak spots)
   - Test quality assessment (excellent/good/needs work)
   - Comparison to industry (75% → AstraWeave X%)
   - Remediation plan (add targeted tests for survived mutants)

---

## Automation Integration (Future)

### CI/CD Integration

**Recommended Setup** (after Phase 10 validation):
```yaml
# .github/workflows/mutation-testing.yml
name: Mutation Testing
on:
  push:
    branches: [main]
  schedule:
    - cron: '0 2 * * 0'  # Weekly Sunday 2AM

jobs:
  mutants:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install cargo-mutants
      - run: cargo mutants --all --output mutants/ --json > mutation_report.json
      - run: |
          SCORE=$(jq '.mutation_score' mutation_report.json)
          if (( $(echo "$SCORE < 75" | bc -l) )); then
            echo "❌ Mutation score $SCORE% below 75% threshold!"
            exit 1
          fi
```

**Benefits**:
- Automatic test quality regression detection
- Prevents merging code with weak tests
- Weekly validation of test suite effectiveness

---

## Timeline

### Phase 10A: P0 Critical Path (Days 1-2)

**Day 1 (6-8 hours)**:
1. astraweave-math (30 min)
2. astraweave-nav (1 hour)
3. astraweave-core (2-3 hours)
4. astraweave-ecs (2-3 hours)

**Day 2 (8-10 hours)**:
1. astraweave-physics (3-4 hours)
2. astraweave-asset (1-2 hours)
3. astraweave-gameplay (2-3 hours)
4. astraweave-audio (1-2 hours)

**Day 3 (8-10 hours)**:
1. astraweave-terrain (2-3 hours)
2. astraweave-scene (1-2 hours)
3. astraweave-ui (2-3 hours)
4. astraweave-render (4-6 hours) — **Most complex**

### Phase 10B: P1 Important (Day 4, 4-6 hours)

1. astraweave-cinematics (15 min)
2. astraweave-materials (15 min)
3. astraweave-weaving (1 hour)
4. astraweave-ai (1-2 hours)
5. aw_editor (1-2 hours)

### Phase 10C: P2 Support (Day 5, 7-10 hours)

1. astraweave-embeddings (30 min)
2. astraweave-pcg (30 min)
3. astraweave-memory (1 hour)
4. astraweave-behavior (1 hour)
5. astraweave-input (1 hour)
6. astraweave-scripting (1 hour)
7. astraweave-security (1 hour)
8. astraweave-llm (2-3 hours) — **Largest test suite**

### Phase 10D: Analysis & Remediation (Day 6, 4-6 hours)

1. Aggregate mutation scores (1 hour)
2. Analyze top 10 survived mutants (2 hours)
3. Add targeted tests for critical weak spots (2-3 hours)
4. Create comprehensive report (1 hour)

**Total Estimated Time**: 29-40 hours (6 days @ 5-7 hours/day)

---

## Current Status

**cargo-mutants Installation**: ✅ COMPLETE (v26.1.2)  
**Mutation Testing Plan**: ✅ COMPLETE (this document)  
**Next Step**: Run mutation test on astraweave-math (30 min, smallest P0 crate)

**Command to Start**:
```powershell
cargo mutants --package astraweave-math --timeout 60 --jobs 8 --output mutants.out/math/
```

**Expected Output**:
```
Mutants tested: 50-100 (estimated based on crate size)
Killed: 40-85 (80-85% expected given 98.07% coverage)
Survived: 5-15 (15-20% expected, will analyze)
Timeout: 0-2 (minimal expected in math crate)
Mutation Score: 80-85% (target: ≥80%)
```

---

**Status**: 🎯 READY TO START Phase 10A Day 1  
**Prepared by**: GitHub Copilot (AI-orchestrated development)  
**Date**: January 20, 2026  
**Version**: 1.0.0
