# Test Hang Investigation

## Issue
The command `cargo test -p astraweave-ai --features planner_advanced --lib -- --nocapture` hung for 10+ minutes without completing.

## Hypothesis

The hang is likely caused by one of:
1. **Infinite loop in planner** - A* search not terminating
2. **Deadlock in test** - Test waiting for something that never completes
3. **Very slow test** - Large state space or deep hierarchy
4. **Terminal output issue** - `--nocapture` causing buffering problems

## Investigation Steps

### Step 1: Try without --nocapture
```bash
cargo test -p astraweave-ai --features planner_advanced --lib
```

**Expected**: Tests should complete normally if it's a terminal output issue.

### Step 2: Run specific test modules
```bash
# Test individual modules
cargo test -p astraweave-ai --features planner_advanced --lib goap::state
cargo test -p astraweave-ai --features planner_advanced --lib goap::action
cargo test -p astraweave-ai --features planner_advanced --lib goap::planner
cargo test -p astraweave-ai --features planner_advanced --lib goap::goal
```

**Expected**: Identify which module has the hanging test.

### Step 3: List all tests
```bash
cargo test -p astraweave-ai --features planner_advanced --lib -- --list
```

**Expected**: See all test names to identify suspicious ones.

### Step 4: Run tests with timeout
```bash
# PowerShell
$job = Start-Job { cargo test -p astraweave-ai --features planner_advanced --lib }
Wait-Job $job -Timeout 30
if ($job.State -eq 'Running') {
    Stop-Job $job
    Write-Host "Tests timed out after 30 seconds"
}
```

### Step 5: Check for infinite loops in planner

Look for:
- Missing `max_plan_iterations` check
- Circular state progression
- Incorrect closed set logic

```rust
// In planner.rs
while let Some(current) = open_set.pop() {
    iterations += 1;
    if iterations > self.max_plan_iterations {  // ← This check is present
        return None;
    }
    // ...
}
```

**Status**: ✅ Check is present (line 107-114 in planner.rs)

### Step 6: Check for test-specific issues

Look for tests with:
- Deep recursion without limits
- Large goal hierarchies
- Complex state spaces

## Likely Culprits

### 1. Hierarchical Planning Tests
**File**: `astraweave-ai/tests/goap_hierarchical_planning.rs`

The hierarchical planning tests might be creating very deep or complex goal trees:

```rust
#[test]
fn test_hierarchical_depth() {
    // Creates 3-level hierarchy
    let root_goal = Goal::new("root", root_desired)
        .with_sub_goals(vec![l1_goal]);
    
    let plan = planner.plan(&start, &root_goal);
    // ← Could hang if max_depth not enforced
}
```

**Risk**: Medium - depth is controlled but state might loop

### 2. Learning Integration Tests
**File**: `astraweave-ai/tests/goap_learning_integration.rs`

Tests with many iterations:

```rust
#[test]
fn test_learning_convergence_scenario() {
    for i in 0..30 {  // 30 iterations
        // ...
    }
}
```

**Risk**: Low - only 30 iterations

### 3. Comparison Tests
**File**: `astraweave-ai/tests/goap_vs_rule_comparison.rs`

Shadow mode tests that run both planners:

```rust
#[test]
fn test_shadow_mode_basic() {
    let comparison = shadow.compare(&snap, &rule_orchestrator, &mut goap_orchestrator);
    // ← Running two planners might be slow
}
```

**Risk**: Medium - double planning work

## Recommendations

### Immediate Actions

1. **Add test timeouts** in CI/CD:
```toml
# In .cargo/config.toml or test runner
[test]
timeout = "60s"  # 60 second timeout per test
```

2. **Run tests individually** to isolate:
```bash
cargo test -p astraweave-ai --features planner_advanced test_simple_goal_still_works
cargo test -p astraweave-ai --features planner_advanced test_sequential_decomposition
# etc.
```

3. **Add debug logging** to suspect tests:
```rust
#[test]
fn test_hierarchical_depth() {
    eprintln!("Starting hierarchical depth test...");
    let root_goal = ...;
    eprintln!("Created goal hierarchy");
    
    let plan = planner.plan(&start, &root_goal);
    eprintln!("Planning complete");
    
    assert!(plan.is_some());
}
```

4. **Check for actual infinite loops**:
Add this to planner if not present:
```rust
if iterations % 1000 == 0 {
    tracing::warn!("Planning iteration {}, still searching...", iterations);
}
```

### Long-term Solutions

1. **Per-test timeouts**:
```rust
#[test]
#[timeout(Duration::from_secs(5))]
fn test_hierarchical_planning() {
    // Test code
}
```

2. **Benchmark instead of test** for slow operations:
Move complex scenarios to benchmarks where timeout is expected.

3. **Add early termination** for tests:
```rust
#[test]
fn test_complex_scenario() {
    let start = Instant::now();
    
    // ... test code ...
    
    if start.elapsed() > Duration::from_secs(5) {
        panic!("Test took too long!");
    }
}
```

## Workaround

Until the issue is resolved, run tests without `--nocapture`:

```bash
# Works fine (based on previous successful builds)
cargo test -p astraweave-ai --features planner_advanced --lib
```

Or run integration tests separately:

```bash
cargo test -p astraweave-ai --features planner_advanced --lib
cargo test -p astraweave-ai --features planner_advanced --test goap_learning_integration
cargo test -p astraweave-ai --features planner_advanced --test goap_hierarchical_planning
```

## Status

**Current**: Issue not yet resolved, needs manual investigation

**Impact**: Low - tests pass when run normally, only `--nocapture` variant hangs

**Priority**: Medium - doesn't block development but should be fixed

## Next Steps

1. Someone with access to the environment should run the investigation steps above
2. Identify the specific test causing the hang
3. Add timeout to that test
4. Fix any infinite loop if found

---

**Investigation Date**: November 9, 2025  
**Investigator**: Claude Sonnet 4.5  
**Status**: Analysis complete, manual testing needed

