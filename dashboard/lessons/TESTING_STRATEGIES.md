# Testing Strategies: Validation Approaches

**Context**: This document captures testing and validation lessons learned during 40+ days of building AstraWeave, with focus on Week 2-3 testing sprints.

---

## Core Principles

### 1. Risk-Based Testing Priority ✅

**Pattern**: Test high-risk code first (not everything equally)

**Why it works**:
- Limited time (can't test everything)
- Focus on impact (critical bugs vs minor issues)
- Diminishing returns (95% coverage usually enough)

**Risk Priority**:
```
1. HIGH RISK (MUST TEST)
   - AI planning (correctness critical)
   - Determinism (multiplayer requirement)
   - API boundaries (easy to break)
   - Physics (gameplay-critical)

2. MEDIUM RISK (SHOULD TEST)
   - ECS iteration (performance-sensitive)
   - Serialization (save/load correctness)
   - Input handling (user-facing)

3. LOW RISK (CAN SKIP)
   - Trivial getters/setters
   - Pure data structs
   - Debug logging
```

**Evidence**:
- **Week 2**: 111 tests written (focused on high-risk)
- **Week 3**: 9 integration tests (cross-module validation)
- **95.5% coverage** achieved (not 100%, good enough)

---

### 2. Integration Tests > Unit Tests (Complex Systems) ✅

**Pattern**: Test full pipelines (not isolated functions) for complex systems

**Why it works**:
- Catches interaction bugs (unit tests miss these)
- Validates real workflows (not synthetic scenarios)
- Higher confidence (end-to-end validation)

**Evidence**:
- **Week 3 Day 2**: 9 integration tests (ECS → Perception → Planning → Physics → ECS feedback)
- **100% passing** (6,000 agent-frames validated)
- **Determinism proven** (3 runs, bit-identical results)

**Example**:
```rust
// ❌ WEAK: Unit test (isolated function)
#[test]
fn test_goap_planner() {
    let planner = GoapPlanner::new();
    let result = planner.plan(&state);
    assert!(result.is_ok());
}

// ✅ STRONG: Integration test (full pipeline)
#[test]
fn test_ai_core_loop_integration() {
    // 1. Setup ECS world
    let mut world = World::new();
    world.spawn((Position::default(), Velocity::default()));
    
    // 2. Build WorldSnapshot (perception)
    let snap = build_snapshot(&world);
    
    // 3. AI planning
    let plan = orchestrator.plan(&mut world, &snap)?;
    
    // 4. Execute actions (physics)
    execute_plan(&mut world, &plan)?;
    
    // 5. Validate ECS feedback
    let pos = world.get::<Position>(entity)?;
    assert_eq!(pos.x, expected_x);  // End-to-end validation
}
```

---

### 3. Determinism Validation (Multi-Frame Tests) ✅

**Pattern**: Run same scenario 3+ times, assert bit-identical results

**Why it works**:
- Proves multiplayer readiness (no desyncs)
- Enables replay system (perfect reproduction)
- Catches non-deterministic bugs (RNG, timing, floating-point)

**Evidence**:
- **Phase 3**: Determinism validation complete
- **Week 3**: 3 runs, bit-identical results (6,000 agent-frames)
- **6.48M validation checks/sec** (anti-cheat proven)

**Example**:
```rust
// Determinism validation (Week 3)
#[test]
fn test_deterministic_simulation() {
    let seed = 12345;
    
    // Run 1
    let mut world1 = create_world(seed);
    for _ in 0..60 {
        tick_systems(&mut world1);
    }
    let state1 = serialize_world(&world1);
    
    // Run 2 (same seed)
    let mut world2 = create_world(seed);
    for _ in 0..60 {
        tick_systems(&mut world2);
    }
    let state2 = serialize_world(&world2);
    
    // Run 3 (same seed)
    let mut world3 = create_world(seed);
    for _ in 0..60 {
        tick_systems(&mut world3);
    }
    let state3 = serialize_world(&world3);
    
    // Assert bit-identical (not "close enough")
    assert_eq!(state1, state2);
    assert_eq!(state2, state3);
}
```

---

### 4. Benchmark = Validation (Performance Tests) ✅

**Pattern**: Benchmarks are tests (not just performance measurement)

**Why it works**:
- Validates performance goals (not just "it works")
- Catches regressions (compare to baseline)
- Enables capacity planning (extrapolate from benchmarks)

**Evidence**:
- **Week 3**: 11 benchmarks validated 46-65% AI improvements
- **Week 8**: Benchmarks caught ECS regression (+18.77%)
- **Threshold validation**: scripts/check_benchmark_thresholds.ps1

**Example**:
```rust
// Benchmark with validation (Week 3)
fn bench_ai_planning(c: &mut Criterion) {
    let world = setup_world();
    let snap = build_snapshot(&world);
    
    c.bench_function("ai_planning", |b| {
        b.iter(|| {
            let plan = orchestrator.plan(&world, &snap);
            black_box(plan);
        });
    });
}

// Threshold validation (scripts/check_benchmark_thresholds.ps1)
# Expected: <1 µs
# Actual: 87-202 ns
# Status: PASS (10× better than threshold)
```

**Threshold Enforcement**:
```powershell
# scripts/check_benchmark_thresholds.ps1
$thresholds = @{
    "ai_planning" = 1000  # 1 µs
    "ecs_tick" = 500      # 500 µs
    "physics_step" = 3000 # 3 ms
}

foreach ($bench in $results) {
    if ($bench.time -gt $thresholds[$bench.name]) {
        Write-Error "❌ $($bench.name) over budget"
        exit 1
    }
}
```

---

## Testing Patterns

### 5. Zero-Warning Policy (Week 3+) ✅

**Pattern**: Treat warnings as errors (fix immediately)

**Why it works**:
- Prevents technical debt (warnings → errors eventually)
- Cleaner code (no noise in CI)
- Catches real issues (unused variables often bugs)

**Evidence**:
- **Week 3 Day 1**: Fixed 14 accumulated warnings (0.2h)
- **18-day zero-warning streak** (Week 3-4, Phase 8.1)
- **Current**: 242 tests, ZERO warnings

**How to apply**:
```bash
# Enforce in CI
cargo clippy --all-features -- -D warnings

# Local development (fail on warnings)
export RUSTFLAGS="-D warnings"
cargo check

# Weekly sweep (find new warnings)
cargo clippy --workspace --all-features -- -W clippy::all
```

---

### 6. Test Naming Convention (Descriptive > Terse) ✅

**Pattern**: Name tests by scenario (not function name)

**Why it works**:
- Clear failure messages (know what broke)
- Self-documenting (test is specification)
- Easy to debug (understand intent immediately)

**Example**:
```rust
// ❌ WEAK: Terse name
#[test]
fn test_planner() { ... }

// ✅ STRONG: Descriptive name
#[test]
fn test_goap_planner_generates_valid_moveto_action_for_reachable_poi() { ... }

// When this fails:
// ❌ "test_planner failed" (no context)
// ✅ "test_goap_planner_generates_valid_moveto_action_for_reachable_poi failed" (clear issue)
```

---

### 7. Property-Based Testing (Fuzz Inputs) ✅

**Pattern**: Generate random inputs, assert invariants hold

**Why it works**:
- Catches edge cases (manually-written tests miss these)
- Validates assumptions (are invariants really true?)
- Higher confidence (tested 1,000+ scenarios, not 10)

**Example**:
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_spatial_hash_never_loses_entities(
        positions in prop::collection::vec(
            (0.0..1000.0, 0.0..1000.0),
            0..1000
        )
    ) {
        let mut hash = SpatialHash::new(10.0);
        
        // Insert all entities
        for (i, &(x, y)) in positions.iter().enumerate() {
            hash.insert(EntityId(i as u64), Vec2::new(x, y));
        }
        
        // Query entire map
        let found = hash.query_all();
        
        // Invariant: All entities must be found
        prop_assert_eq!(found.len(), positions.len());
    }
}

// Runs 100+ random scenarios (default proptest config)
```

---

### 8. Test Fixtures (Reusable Setup) ✅

**Pattern**: Extract common setup to helper functions

**Why it works**:
- DRY (don't repeat setup)
- Consistent state (all tests use same baseline)
- Easy to modify (change once, affects all tests)

**Example**:
```rust
// Test fixture (Week 3 pattern)
fn create_test_world() -> (World, EntityId, EntityId) {
    let mut world = World::new();
    
    let player = world.spawn((
        Position::new(0.0, 0.0),
        Velocity::default(),
        Health::new(100),
    ));
    
    let enemy = world.spawn((
        Position::new(10.0, 10.0),
        Velocity::default(),
        Health::new(50),
    ));
    
    (world, player, enemy)
}

// Tests use fixture
#[test]
fn test_ai_planning() {
    let (mut world, player, enemy) = create_test_world();
    // ... test code
}

#[test]
fn test_physics_collision() {
    let (mut world, player, enemy) = create_test_world();
    // ... test code
}
```

---

## Validation Strategies

### 9. Manual Validation (User Acceptance Testing) ✅

**Pattern**: Run examples manually, document expected behavior

**Why it works**:
- Catches UX issues (tests don't)
- Validates feel (not just correctness)
- Build confidence (see it working)

**Evidence**:
- **Phase 8.1 Week 2**: 27 automated tests + user acceptance testing
- **UI_MENU_DEMO_TEST_REPORT.md**: Manual validation (7/7 pass)
- **Week 3 Day 5**: Tooltip demos validated manually

**Example** (UI_MENU_DEMO_TEST_REPORT.md):
```markdown
## Manual Validation Checklist

1. ✅ Main menu appears on startup
2. ✅ "New Game" button navigates to gameplay
3. ✅ "Settings" button opens settings menu
4. ✅ Graphics settings persist after restart
5. ✅ ESC key opens pause menu during gameplay
6. ✅ Audio sliders adjust volume smoothly
7. ✅ "Quit" button exits cleanly

**Status**: 7/7 PASS
```

---

### 10. Regression Test on Bug Fixes ✅

**Pattern**: Write test that reproduces bug, then fix bug

**Why it works**:
- Prevents regression (bug won't come back)
- Validates fix (test fails before fix, passes after)
- Documents bug (test is specification)

**Example**:
```rust
// Week 3 ActionStep bug (pattern matching required)
#[test]
fn test_actionstep_pattern_matching_required() {
    let action = ActionStep::MoveTo {
        target_pos: IVec2::new(10, 10),
        distance: 5.0,
    };
    
    // ❌ WRONG: Field access (this was the bug)
    // let pos = action.target_pos;  // Doesn't compile
    
    // ✅ CORRECT: Pattern matching
    if let ActionStep::MoveTo { target_pos, .. } = action {
        assert_eq!(target_pos, IVec2::new(10, 10));
    } else {
        panic!("Expected MoveTo variant");
    }
}
```

---

### 11. Smoke Tests (Quick Validation) ✅

**Pattern**: Run fast tests before full test suite

**Why it works**:
- Fast feedback (catch obvious breaks)
- Fail early (don't wait 5 min for full suite)
- Developer-friendly (run frequently)

**Example**:
```bash
# Smoke tests (fast, <10 seconds)
cargo test -p astraweave-core --lib
cargo test -p astraweave-ai test_core_loop

# Full suite (slow, 1-2 minutes)
cargo test --workspace

# CI pipeline (2 stages)
1. Smoke tests (fast fail)
2. Full suite (thorough validation)
```

---

### 12. Coverage Metrics (But Don't Worship 100%) ✅

**Pattern**: Track coverage, but don't aim for 100%

**Why it works**:
- Diminishing returns (95% → 100% is expensive)
- Some code untestable (unsafe, debug logging)
- Focus on critical paths (not everything)

**Evidence**:
- **Week 2**: 95.5% coverage achieved (stopped there)
- **Week 3**: Added 9 integration tests (higher value than 4.5% more unit tests)

**How to apply**:
```bash
# Install tarpaulin (coverage tool)
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage/

# Set target (not 100%)
# Target: 90-95% for critical crates
# Target: 70-80% for UI/rendering (harder to test)
```

---

## AI Testing

### 13. Mock LLM for Unit Tests (Fast) ✅

**Pattern**: Use MockLLM for fast unit tests (not real LLM)

**Why it works**:
- Fast (no network calls)
- Deterministic (same output every time)
- CI-friendly (no external dependencies)

**Example**:
```rust
use astraweave_ai::test_utils::MockLlmOrch;

#[test]
fn test_arbiter_transitions_to_executing_llm() {
    let mock_llm = Arc::new(MockLlmOrch::new_with_plan(
        mock_plan()  // Return this plan
    ));
    
    let executor = LlmExecutor::new(mock_llm, tool_registry);
    let mut arbiter = AIArbiter::new(executor);
    
    // Test logic (fast, no real LLM)
    arbiter.update(&world, &snap)?;
    assert!(matches!(arbiter.mode(), AIControlMode::ExecutingLLM { .. }));
}
```

---

### 14. Real LLM for Integration Tests (Slow) ✅

**Pattern**: Use real LLM for end-to-end validation (not mocks)

**Why it works**:
- Catches real bugs (mocks hide issues)
- Validates production setup (same as prod)
- Builds confidence (not synthetic)

**Evidence**:
- **Phase 7**: Live Ollama validation caught case sensitivity bug (0% → 75-85% success)
- **hello_companion --demo-all**: End-to-end validation with Hermes 2 Pro

**Example**:
```rust
#[tokio::test]
#[ignore]  // Slow test, run manually
async fn test_llm_integration_with_hermes2pro() {
    let client = OllamaClient::new("http://localhost:11434");
    let model = "adrienbrault/nous-hermes2pro:Q4_K_M";
    
    let llm = Hermes2ProOllama::new(client, model.into());
    let executor = LlmExecutor::new(Arc::new(llm), tool_registry);
    
    // Real LLM request (slow, but thorough)
    let result = executor.request_plan(&world, &snap).await?;
    
    // Validate production behavior
    assert!(result.is_ok());
    assert_eq!(result.unwrap().steps.len(), 3);
}
```

---

### 15. Test with Real Data (Not Synthetic) ✅

**Pattern**: Use realistic entity counts, positions, velocities

**Why it works**:
- Finds performance issues (synthetic data hides these)
- Validates edge cases (real data is messy)
- Builds confidence (production-like scenarios)

**Example**:
```rust
// ❌ WEAK: Synthetic data (too clean)
#[test]
fn test_spatial_hash_performance() {
    let positions = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 10.0),
        Vec2::new(20.0, 20.0),
    ];
    // ...
}

// ✅ STRONG: Realistic data (messy, clustered)
#[test]
fn test_spatial_hash_performance_realistic() {
    let mut rng = ChaCha8Rng::seed_from_u64(12345);
    
    // 1,000 entities with realistic distribution
    let positions: Vec<Vec2> = (0..1000)
        .map(|_| {
            // Clustered (like real gameplay)
            let cluster = rng.gen_range(0..5);
            let base = Vec2::new(cluster as f32 * 50.0, cluster as f32 * 50.0);
            base + Vec2::new(rng.gen_range(-25.0..25.0), rng.gen_range(-25.0..25.0))
        })
        .collect();
    
    // Test with realistic data
    // ...
}
```

---

## CI/CD Testing

### 16. Fail Fast (Don't Continue After Errors) ✅

**Pattern**: Stop CI pipeline on first failure (don't continue)

**Why it works**:
- Faster feedback (don't wait for all stages)
- Clearer errors (not buried in logs)
- Saves CI time (don't run tests if build fails)

**Example** (.github/workflows/ci.yml):
```yaml
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v3
      
      - name: Check
        run: cargo check --workspace
        # If this fails, stop here (don't run tests)
  
  test:
    needs: check  # Only run if check succeeds
    runs-on: ubuntu-latest
    steps:
      - name: Test
        run: cargo test --workspace
```

---

### 17. Cache Dependencies (Fast CI) ✅

**Pattern**: Cache Rust dependencies in CI (not re-download)

**Why it works**:
- 10× faster builds (5 min → 30 sec)
- Reduces CI load (less network traffic)
- Faster feedback (shorter wait times)

**Example** (.github/workflows/ci.yml):
```yaml
- name: Cache Cargo
  uses: actions/cache@v3
  with:
    path: |
      ~/.cargo/bin/
      ~/.cargo/registry/index/
      ~/.cargo/registry/cache/
      ~/.cargo/git/db/
      target/
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

---

## Test Organization

### 18. Separate Unit/Integration Tests ✅

**Pattern**: Unit tests in `src/`, integration tests in `tests/`

**Why it works**:
- Clear separation (unit vs integration)
- Faster unit tests (no module linking)
- Parallel execution (integration tests run separately)

**Project structure**:
```
astraweave-ai/
├─ src/
│  ├─ lib.rs
│  ├─ core_loop.rs
│  └─ core_loop.rs  // Unit tests here
│     #[cfg(test)]
│     mod tests { ... }
├─ tests/
│  ├─ perception_tests.rs     // Integration test
│  ├─ planner_tests.rs        // Integration test
│  └─ integration_tests.rs    // Integration test
└─ benches/
   └─ ai_benchmarks.rs         // Benchmarks
```

---

### 19. Test Organization by Feature ✅

**Pattern**: Group tests by feature (not by type)

**Why it works**:
- Easy to find (tests near code)
- Feature-focused (test one thing)
- Refactoring-friendly (move code, move tests)

**Example**:
```
// ❌ WEAK: Organized by type
tests/
├─ unit_tests.rs (50 tests, unrelated)
├─ integration_tests.rs (30 tests, unrelated)
└─ benchmarks.rs (20 benchmarks, unrelated)

// ✅ STRONG: Organized by feature
tests/
├─ perception_tests.rs (WorldSnapshot tests)
├─ planner_tests.rs (AI planning tests)
├─ physics_tests.rs (Physics integration tests)
└─ determinism_tests.rs (Replay system tests)
```

---

## Conclusion

**Key Insight**: Test what matters, not everything equally

The testing strategies that worked best:
1. ✅ **Risk-based priority** (test high-risk code first)
2. ✅ **Integration > unit** (for complex systems)
3. ✅ **Determinism validation** (multi-frame tests)
4. ✅ **Benchmark = validation** (performance tests)
5. ✅ **Zero-warning policy** (fix immediately)
6. ✅ **Property-based testing** (fuzz inputs)
7. ✅ **Manual UAT** (see it working)
8. ✅ **Regression tests** (prevent bug recurrence)
9. ✅ **Real data** (not synthetic)
10. ✅ **Fast CI** (cache dependencies)

**Evidence**: 242 tests passing (100% pass rate), 18-day zero-warning streak, 95.5% coverage, determinism validated

**Next**: See `WHAT_WORKED.md` for process patterns and `PERFORMANCE_PATTERNS.md` for optimization lessons

---

*Last Updated*: January 2026 (October 20, 2025)  
*Extracted from*: Week 2-3 testing sprints, 242 tests, validation reports
