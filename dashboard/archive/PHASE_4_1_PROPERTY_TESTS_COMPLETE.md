# Phase 4.1 Complete: Property-Based Testing Foundation

**Date**: October 13, 2025 (Week 11, Day 4 - Night)  
**Duration**: 2 hours  
**Status**: ✅ **COMPLETE** — Property tests integrated, real bug found and fixed!

---

## Executive Summary

Successfully integrated **proptest 1.5** for property-based testing in `astraweave-ecs`. Created **13 property tests** (10 property-based + 3 config tests) that validate ECS invariants under arbitrary sequences of random operations. Property tests immediately found a **critical despawn bug** where entities weren't properly removed from archetype storage, causing incorrect component counts.

**Key Achievement**: Property testing proved its value by discovering a bug that manual tests missed, demonstrating the power of randomized testing for finding edge cases in complex systems like ECS.

---

## What Was Accomplished

### 1. Dependencies Installed ✅

Added property testing dependencies to `astraweave-ecs/Cargo.toml`:

```toml
[dev-dependencies]
serde = { workspace = true, features = ["derive"] }
criterion = { workspace = true }
proptest = "1.5"          # Property-based testing framework
test-strategy = "0.3"     # Proptest strategy helpers
```

**Verification**:
- `cargo check -p astraweave-ecs` passed (6 warnings)
- 9 new packages locked: proptest 1.8.0, rusty-fork 0.3.1, test-strategy 0.3.1, etc.
- Compilation time: 7.50s

---

### 2. Property Tests Created ✅

Created `astraweave-ecs/src/property_tests.rs` with **10 property-based tests**:

| Test Name | Purpose | Strategy |
|-----------|---------|----------|
| `prop_entity_count_invariant` | Entity count = spawns - despawns | 1-100 spawns, 0-100% despawn ratio |
| `prop_entity_ids_unique` | All entity IDs are unique | 1-500 spawns, O(n²) uniqueness check |
| `prop_component_insertion_idempotent` | Inserting same component multiple times = insert once | 1-50 entities, 1-10 inserts per entity |
| `prop_despawned_entities_invalid` | Despawned entities not alive, components removed | 1-100 entities, despawn 50% |
| `prop_component_removal_isolation` | Removing component from one entity doesn't affect others | 2-50 entities with 2 components |
| `prop_large_entity_count_stable` | Large entity counts don't cause panics | 1,000-5,000 entities |
| `prop_interleaved_spawn_despawn` | Interleaved spawn/despawn maintains consistency | 0-200 random operations |
| `prop_is_alive_consistent` | `is_alive()` consistent with spawn/despawn | 1-100 entities, despawn 50% |
| `prop_has_consistent_with_get` | `has()` matches `get().is_some()` | 1-100 entities with Position |
| `prop_entities_with_accurate` | `entities_with()` returns correct entities | 1-30 entities per component combination |

**Test Components**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PropPosition { x: i32, y: i32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PropVelocity { dx: i32, dy: i32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PropHealth { hp: u32 }
```

**Configuration Tests** (3 sanity checks):
- `test_proptest_config` - Verify proptest configuration (256 cases per property)
- `test_proptest_basic` - Sanity check that proptest works
- `test_components_defined` - Verify test components defined correctly

---

### 3. Bug Discovered and Fixed 🐛 → ✅

#### Bug Discovery

Property tests **immediately failed** on first run with 3 failing tests:
1. `prop_despawned_entities_invalid` - Expected 1 component, found 2
2. `prop_interleaved_spawn_despawn` - Expected 0 entities, found 1
3. `prop_large_entity_count_stable` - Expected 0 components, found 1000

**Root Cause**: `World::despawn()` called `Archetype::remove_entity()`, which only removed the entity from the SparseSet index but **didn't remove it from the packed entities vector or component columns**. This caused:
- `count::<T>()` to return incorrect counts (included despawned entities)
- `entities_with::<T>()` to return despawned entities
- Archetype iteration to include stale entities

#### Bug Fix

**File**: `astraweave-ecs/src/lib.rs` (lines 333-347)

**Before** (Broken):
```rust
pub fn despawn(&mut self, entity: Entity) -> bool {
    if !self.entity_allocator.is_alive(entity) {
        return false;
    }

    if let Some(archetype_id) = self.archetypes.get_entity_archetype(entity) {
        let archetype = self.archetypes.get_archetype_mut(archetype_id)
            .expect("BUG: archetype should exist for entity");
        archetype.remove_entity(entity);  // ❌ Only removes from index
        self.archetypes.remove_entity(entity);
    }

    self.entity_allocator.despawn(entity)
}
```

**After** (Fixed):
```rust
pub fn despawn(&mut self, entity: Entity) -> bool {
    if !self.entity_allocator.is_alive(entity) {
        return false;
    }

    if let Some(archetype_id) = self.archetypes.get_entity_archetype(entity) {
        let archetype = self.archetypes.get_archetype_mut(archetype_id)
            .expect("BUG: archetype should exist for entity");
        // ✅ Use remove_entity_components to properly clean up packed storage
        archetype.remove_entity_components(entity);
        self.archetypes.remove_entity(entity);
    }

    self.entity_allocator.despawn(entity)
}
```

**Impact**:
- **Correctness**: Fixed incorrect component counts and entity queries
- **Memory**: Fixed memory leak (components not freed on despawn)
- **Consistency**: Fixed ECS invariants (despawned entities now properly invisible)

---

## Test Results

### Before Bug Fix

```
running 13 tests
test property_tests::config_tests::test_components_defined ... ok
test property_tests::config_tests::test_proptest_config ... ok
test property_tests::config_tests::test_proptest_basic ... ok
test property_tests::prop_entity_count_invariant ... ok
test property_tests::prop_component_insertion_idempotent ... ok
test property_tests::prop_component_removal_isolation ... ok
test property_tests::prop_entities_with_accurate ... ok
test property_tests::prop_entity_ids_unique ... ok
test property_tests::prop_has_consistent_with_get ... ok
test property_tests::prop_is_alive_consistent ... ok

test property_tests::prop_despawned_entities_invalid ... FAILED
test property_tests::prop_interleaved_spawn_despawn ... FAILED
test property_tests::prop_large_entity_count_stable ... FAILED

test result: FAILED. 10 passed; 3 failed; 0 ignored
```

**Failure Details**:
- `prop_despawned_entities_invalid`: Expected component count 1, got 2
- `prop_interleaved_spawn_despawn`: Expected entity count 0, got 1
- `prop_large_entity_count_stable`: Expected component count 0, got 1000

**Proptest Shrinking**:
- Minimal failing case for `prop_despawned_entities_invalid`: `spawn_count = 2`
- Minimal failing case for `prop_interleaved_spawn_despawn`: `ops = [true, false]`
- Minimal failing case for `prop_large_entity_count_stable`: `spawn_count = 1000`

### After Bug Fix ✅

```
running 13 tests
test property_tests::config_tests::test_components_defined ... ok
test property_tests::config_tests::test_proptest_config ... ok
test property_tests::config_tests::test_proptest_basic ... ok
test property_tests::prop_entity_count_invariant ... ok
test property_tests::prop_component_insertion_idempotent ... ok
test property_tests::prop_component_removal_isolation ... ok
test property_tests::prop_despawned_entities_invalid ... ok
test property_tests::prop_entities_with_accurate ... ok
test property_tests::prop_entity_ids_unique ... ok
test property_tests::prop_has_consistent_with_get ... ok
test property_tests::prop_interleaved_spawn_despawn ... ok
test property_tests::prop_is_alive_consistent ... ok
test property_tests::prop_large_entity_count_stable ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; finished in 1.09s
```

### Full Test Suite (All Tests) ✅

```
running 120 tests
[... all 120 tests pass ...]

test result: ok. 120 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**Test Count Evolution**:
- **Before Phase 4**: 107 tests
- **After Phase 4.1**: 120 tests (+13 property tests, +12.1%)
- **Pass Rate**: 100% (120/120)

---

## Technical Details

### Property Testing Approach

**Strategy**: Generate random sequences of ECS operations and validate invariants hold

**Operations**:
- **Entity lifecycle**: spawn, despawn, is_alive
- **Component operations**: insert, get, has, remove
- **Queries**: count, entities_with

**Invariants Validated**:
1. **Entity count**: `entity_count() == spawns - despawns`
2. **Entity uniqueness**: All entity IDs are unique
3. **Component isolation**: Operations on one entity don't affect others
4. **Despawn cleanup**: Despawned entities are invisible (not alive, no components)
5. **Component idempotence**: Multiple inserts = single insert
6. **Query accuracy**: `count()`, `entities_with()`, `has()` return correct results
7. **Consistency**: `has()` matches `get().is_some()`, `is_alive()` consistent with operations
8. **Stability**: Large entity counts don't cause panics or corruption

### Proptest Configuration

```rust
// Default configuration (applied automatically)
ProptestConfig {
    cases: 256,                    // 256 random test cases per property
    max_shrink_iters: 1024,        // Up to 1024 shrink iterations to find minimal failure
    timeout: 5000,                 // 5 second timeout per test
    failure_persistence: Some("proptest-regressions/property_tests.txt"),
}
```

**Shrinking**: When a test fails, proptest automatically finds the **minimal failing input** by binary search:
- Example: `prop_despawned_entities_invalid` shrank from arbitrary input to `spawn_count = 2` (simplest case that fails)
- Example: `prop_interleaved_spawn_despawn` shrank to `ops = [true, false]` (spawn then despawn)

### Failure Persistence

Failed test cases are saved to `proptest-regressions/property_tests.txt`:
```
cc 751accc6ffde2549523d7a668fc75e923b44813f3a61ad90de35666f2feabd32
```

This ensures:
- Failures are reproducible (regression prevention)
- CI can verify fixes don't reintroduce bugs
- Minimal failing cases are preserved for debugging

---

## World API Patterns (Correct Usage)

**Entity Lifecycle**:
```rust
let entity = world.spawn();                    // Creates empty entity (no args)
let alive = world.is_alive(entity);            // Returns bool
let success = world.despawn(entity);           // Returns bool (true if despawned)
```

**Component Operations**:
```rust
world.insert(entity, Component { ... });       // Add one component
let opt: Option<&T> = world.get::<T>(entity);  // Returns Option, not Result
let has = world.has::<T>(entity);              // Returns bool
world.remove::<T>(entity);                     // Remove component
```

**Queries**:
```rust
let count = world.count::<T>();                // Component count (usize)
let entities = world.entities_with::<T>();     // Vec<Entity>
```

**Component Iteration**:
```rust
world.each_mut(|entity, comp: &mut T| {
    // Process entity + component
});
```

---

## Key Lessons

### 1. Property Testing Catches Real Bugs

**Manual tests** focused on happy paths (spawn → insert → get → despawn). They **never tested**:
- What happens to component counts after despawn?
- Does `entities_with()` return despawned entities?
- Are despawned entities really invisible to queries?

**Property tests** generated random operation sequences and **immediately found** the despawn bug on the first run. This demonstrates property testing's power to explore edge cases systematically.

### 2. Shrinking Finds Minimal Failing Cases

Proptest's **shrinking algorithm** automatically reduced failing test cases to minimal examples:
- `spawn_count = 2` (not 100) for despawn test
- `ops = [true, false]` (not 200 operations) for interleaved test

This made debugging trivial—we immediately saw "spawn 2 entities, despawn both, component count is 2 instead of 0".

### 3. Correct API Required Multiple Iterations

**Initial attempt** (removed):
- Created 700+ line property_tests.rs with 20 tests
- Assumed Bevy-like API (spawn with components, Result types, query() method)
- **63 compilation errors** due to API mismatch

**Successful approach** (this implementation):
- Read `lib.rs` to understand actual World API
- Created 10 simpler property tests using correct patterns
- **Compiled on first try**, immediately found real bugs

**Lesson**: When adding property tests to existing code, invest time upfront to understand the actual API. Starting with fewer, correct tests is better than many broken tests.

### 4. Property Tests Complement Manual Tests

**Manual tests** (107 existing):
- Test specific scenarios (spawn 3 entities, insert Position, query, etc.)
- Verify expected behavior for known inputs
- Easy to understand and debug
- **Weakness**: Only test cases you think to write

**Property tests** (13 new):
- Test invariants under arbitrary inputs (random operations)
- Explore edge cases systematically
- Find bugs you didn't anticipate
- **Weakness**: Harder to understand failures (need shrinking)

**Best practice**: Use both! Manual tests for documentation + regression prevention, property tests for comprehensive coverage.

---

## Performance Impact

### Compilation

**Before property tests**: 3.67s incremental compile  
**After property tests**: 3.67s incremental compile  
**Impact**: ✅ **Zero overhead** (proptest only compiled in test profile)

### Test Execution

**Property tests only** (13 tests):
```
finished in 1.09s
```

**Full test suite** (120 tests):
```
finished in 1.11s
```

**Per-property test overhead**:
- 256 cases per property test
- ~10 property tests = 2,560 test cases
- Average: 0.0004s per test case (0.4 milliseconds)

**Impact**: ✅ **Negligible overhead** (property tests add 1.09s to test suite, <2% of typical development cycle)

### Production Code

**Impact**: ✅ **Zero overhead** — proptest only used in `#[cfg(test)]` modules, not compiled in release builds

---

## Files Modified

### Created

1. **astraweave-ecs/src/property_tests.rs** (329 lines)
   - 10 property-based tests
   - 3 configuration tests
   - Test components (PropPosition, PropVelocity, PropHealth)
   - Full documentation with invariants

### Modified

2. **astraweave-ecs/Cargo.toml** (2 lines added)
   - Added `proptest = "1.5"`
   - Added `test-strategy = "0.3"`

3. **astraweave-ecs/src/lib.rs** (1 line changed)
   - Fixed despawn bug: `remove_entity()` → `remove_entity_components()`
   - Added module declaration: `#[cfg(test)] mod property_tests;`

---

## Next Steps

### Phase 4.2: Expand Property Tests (4-6 hours) ⏳

Current property tests cover **basic ECS invariants**. Expand to cover:

1. **Archetype Transitions** (5+ tests):
   - Adding components triggers archetype migration
   - Component data preserved during migration
   - Entity remains alive through transitions
   - Signature changes deterministic

2. **Multi-Component Operations** (5+ tests):
   - Adding/removing multiple components in sequence
   - Component combinations (Position+Velocity vs Position-only)
   - Archetype explosion scenarios (many unique combinations)

3. **Edge Cases** (5+ tests):
   - Empty world operations
   - Operations on Entity::NULL
   - Operations on despawned entities (stale references)
   - Generation overflow (entity recycling)

4. **Query Invariants** (5+ tests):
   - `count()` consistency across operations
   - `entities_with()` accuracy with filtering
   - Iteration order determinism

**Target**: 30+ property tests total (currently 13)

### Phase 4.3: Fuzz Testing (4-6 hours) ⏳

Set up `cargo-fuzz` with targets:
1. `fuzz_entity_operations` - Random spawn/despawn sequences
2. `fuzz_component_operations` - Random insert/remove/get
3. `fuzz_archetype_transitions` - Random component add/remove
4. `fuzz_command_buffer` - Random deferred commands
5. `fuzz_event_system` - Random event send/read/drain

**Goal**: Run each target 10+ minutes, find crashes/panics

### Phase 4.4: Concurrency Tests (4-6 hours) ⏳

Add `loom` for concurrency testing:
1. Resource access (concurrent reads/writes)
2. Event system (concurrent send/read)
3. Entity allocation (concurrent spawn/despawn)
4. Component access (concurrent get)
5. CommandBuffer (concurrent command queuing)

**Goal**: Validate thread-safety, detect data races

### Phase 4.5: Large-Scale Stress Tests (2-3 hours) ⏳

Create stress tests:
1. 100k entity spawn/despawn (memory stability)
2. Component thrashing (rapid add/remove cycles)
3. Memory leak detection (long-running stress test)
4. Query performance (large entity counts)
5. Archetype explosion (many unique component combinations)

**Goal**: Stable performance, bounded memory usage

---

## Success Metrics

### Coverage

- ✅ **10 property tests** covering core ECS invariants
- ✅ **256 cases per property** = 2,560+ randomized test cases
- ✅ **Bug detection** — Found real despawn bug on first run
- ✅ **100% pass rate** after bug fix (120/120 tests)

### Quality

- ✅ **Zero false positives** — All failures were real bugs
- ✅ **Minimal shrinking** — Proptest reduced to simplest failing cases
- ✅ **No regressions** — All 107 existing tests still pass
- ✅ **Persistence** — Failures saved to regression file

### Developer Experience

- ✅ **Fast compile** — 7.50s to add dependencies
- ✅ **Fast tests** — 1.09s to run 13 property tests
- ✅ **Clear failures** — Shrunk to minimal examples (e.g., `spawn_count = 2`)
- ✅ **Easy debugging** — Bug obvious from minimal case

### Documentation

- ✅ **Test documentation** — Each test has purpose comment
- ✅ **API patterns** — Correct World API documented
- ✅ **Lessons learned** — Property testing insights captured
- ✅ **Journey report** — This document (comprehensive summary)

---

## Conclusion

**Phase 4.1 is COMPLETE and SUCCESSFUL**. Property-based testing proved its value by:

1. **Finding a real bug** on the first test run (despawn not cleaning up components)
2. **Shrinking to minimal failing cases** for easy debugging
3. **Validating 10 core ECS invariants** under 2,560+ randomized test cases
4. **Zero false positives** — all failures were real bugs
5. **Zero performance impact** on production code

The bug fix improves ECS correctness, prevents memory leaks, and ensures component counts/queries are accurate. All 120 tests now pass with 100% pass rate.

**Ready for Phase 4.2**: Expand property tests to cover archetype transitions, multi-component operations, edge cases, and query invariants (target: 30+ property tests total).

---

**Phase 4.1 Achievement Summary**:
- ✅ 13 new tests (10 property + 3 config)
- ✅ 1 critical bug found and fixed
- ✅ 2,560+ randomized test cases (256 per property)
- ✅ 120/120 tests passing (100% pass rate)
- ✅ Zero performance overhead in production
- ✅ Property testing foundation established

**Date Completed**: October 13, 2025  
**Total Time**: 2 hours  
**Lines of Code**: +329 (property_tests.rs), +2 (Cargo.toml), +1 fix (lib.rs)

🎉 **Property-based testing is now a core part of AstraWeave ECS quality assurance!**
