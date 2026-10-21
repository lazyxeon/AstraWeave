# Weaving System Test Implementation Plan

**Date**: January 2025  
**Target**: Increase test coverage from 9.47% (21 tests) to 80%+ (96+ tests)  
**Effort Estimate**: 6-8 hours (0.5-1 day)  
**Priority**: **P0 BLOCKER** - Must complete before Week 1 greybox work begins

---

## Executive Summary

The `astraweave-weaving` crate currently has **21 unit tests** covering basic adjudication, intent proposal, and pattern detection. This represents approximately **9.47% test coverage** (138/1457 lines covered per prior metrics).

**Critical Gap**: The core fate-weaving mechanics that define Veilweaver's identity are undertested:
- ❌ Thread manipulation (snip, splice, knot)
- ❌ Timeline branching and merging
- ❌ Causality validation
- ❌ Anchor stabilization sequences
- ❌ Storm choice determinism

**This plan** provides step-by-step instructions to implement **75 additional tests** across 5 categories, bringing total coverage to **96+ tests (80%+)**.

---

## Test Categories & Prioritization

### Category 1: Anchor Stabilization (15 tests) - **P0 CRITICAL**

**Why P0**: Anchor stabilization is the core tutorial mechanic (Z1 Frayed Causeway) and boss fight mechanic (Z4 arena anchors). Bugs here break the entire vertical slice.

**File**: `astraweave-weaving/tests/anchor_stabilization_tests.rs` (new)

#### Tests to Implement:

1. **`test_single_anchor_basic_repair`**
   - **Setup**: Single unstable anchor, player has 1 Echo Shard
   - **Action**: Apply stabilization pulse
   - **Assert**: Anchor stabilized, Echo Shard consumed, event emitted

2. **`test_single_anchor_insufficient_resources`**
   - **Setup**: Single unstable anchor, player has 0 Echo Shards
   - **Action**: Attempt stabilization pulse
   - **Assert**: Anchor remains unstable, no resource consumed, error returned

3. **`test_multi_anchor_sequence`**
   - **Setup**: 3 unstable anchors, player has 3 Echo Shards
   - **Action**: Stabilize in sequence (anchor_a → anchor_b → anchor_c)
   - **Assert**: All anchors stabilized, activation_order tracked (0, 1, 2)

4. **`test_multi_anchor_parallel_attempt`**
   - **Setup**: 2 unstable anchors, player has 2 Echo Shards
   - **Action**: Attempt simultaneous stabilization (single frame)
   - **Assert**: Only one anchor processed per frame (deterministic ordering)

5. **`test_anchor_interruption_mid_stabilization`**
   - **Setup**: Anchor at 50% stability, player channeling pulse
   - **Action**: Enemy attack interrupts channeling
   - **Assert**: Anchor reverts to unstable, partial progress lost

6. **`test_anchor_echo_cost_validation`**
   - **Setup**: Anchor requires 2 Echo Shards (boss arena variant)
   - **Action**: Apply pulse with only 1 Echo Shard
   - **Assert**: Pulse fails, anchor unchanged, clear error message

7. **`test_anchor_rupture_from_boss_attack`**
   - **Setup**: Stable anchor in boss arena
   - **Action**: Boss casts `Anchor Rupture` (see OATHBOUND_WARDEN_ENCOUNTER.md)
   - **Assert**: Anchor destabilized, stability drops to 20%, repair required

8. **`test_anchor_repair_under_combat_pressure`**
   - **Setup**: Unstable anchor, 3 enemies within 5 m
   - **Action**: Player attempts repair while taking damage
   - **Assert**: Repair succeeds if not interrupted, enemies do not block repair

9. **`test_anchor_respawn_after_rupture`**
   - **Setup**: Anchor completely destroyed (0% stability)
   - **Action**: Wait 30 seconds (respawn timer)
   - **Assert**: Anchor respawns at 50% stability, repair available

10. **`test_anchor_stability_thresholds`**
    - **Setup**: Anchor at various stability levels (10%, 50%, 90%)
    - **Action**: Apply single pulse to each
    - **Assert**: Stability increases by fixed amount (e.g., +40%), caps at 100%

11. **`test_anchor_highlight_radius_collision`**
    - **Setup**: Player at various distances from anchor (1 m, 2.5 m, 4 m)
    - **Action**: Query highlight visibility
    - **Assert**: Highlight visible within 2.5 m radius, invisible beyond

12. **`test_anchor_metadata_parsing_from_cells`**
    - **Setup**: Cell with `WeaveAnchor` component data
    - **Action**: Parse cell metadata
    - **Assert**: Anchor spec extracted correctly (position, ID, stability)

13. **`test_anchor_sequence_tracking`**
    - **Setup**: Tutorial state with 5 anchors
    - **Action**: Stabilize in order: 3 → 1 → 4 → 2 → 5
    - **Assert**: `anchor_sequence` = ["anchor_3", "anchor_1", "anchor_4", "anchor_2", "anchor_5"]

14. **`test_anchor_failsafe_reset_logic`**
    - **Setup**: Tutorial anchor at 15% stability (below 20% threshold)
    - **Action**: Trigger failsafe check
    - **Assert**: Anchor reset to 50% stability, player notified

15. **`test_anchor_completion_event_emission`**
    - **Setup**: Final tutorial anchor stabilized
    - **Action**: Complete stabilization
    - **Assert**: `AnchorStabilizedEvent` emitted with correct anchor_id

**Estimated Time**: 2.5-3 hours (12 minutes per test average)

---

### Category 2: Thread Manipulation (20 tests) - **P1 HIGH**

**Why P1**: Thread operations (snip, splice, knot) are advanced weaving mechanics. Not required for basic tutorial but critical for demonstrating fate-weaving uniqueness.

**File**: `astraweave-weaving/tests/thread_manipulation_tests.rs` (new)

#### Tests to Implement:

1. **`test_thread_snip_basic`**
   - **Action**: Snip single thread at midpoint
   - **Assert**: Thread split into two segments, causality chain broken

2. **`test_thread_snip_budget_check`**
   - **Setup**: Budget remaining = 5, snip cost = 10
   - **Action**: Attempt snip
   - **Assert**: Snip denied, budget unchanged

3. **`test_thread_splice_reconnect`**
   - **Setup**: Two thread segments (A, B)
   - **Action**: Splice A → B
   - **Assert**: Single continuous thread, causality restored

4. **`test_thread_splice_mismatched_types`**
   - **Setup**: Temporal thread + spatial thread
   - **Action**: Attempt splice
   - **Assert**: Splice denied (incompatible thread types)

5. **`test_thread_knot_creation`**
   - **Action**: Create knot at thread intersection
   - **Assert**: Knot node created, thread paths converge

6. **`test_thread_knot_stability`**
   - **Setup**: Knot with 3 threads converging
   - **Action**: Apply tension to one thread
   - **Assert**: Knot stability decreases, may rupture if threshold exceeded

7. **`test_timeline_branching_storm_choice`**
   - **Setup**: Loom Crossroads choice trigger
   - **Action**: Select "Stabilize" option
   - **Assert**: Timeline branches, flag set, separate event stream created

8. **`test_timeline_merging_post_choice`**
   - **Setup**: Two timeline branches (stabilize/redirect)
   - **Action**: Boss defeat (both branches converge)
   - **Assert**: Timelines merge, single outcome state validated

9. **`test_causality_validation_no_paradox`**
   - **Setup**: Event sequence: A → B → C
   - **Action**: Attempt to snip thread B (breaks causality)
   - **Assert**: Snip denied, causality violation detected

10. **`test_causality_paradox_detection`**
    - **Setup**: Circular timeline loop (A → B → C → A)
    - **Action**: Detect paradox
    - **Assert**: Paradox flagged, loop broken at weakest link

11. **`test_budget_constraints_during_combat`**
    - **Setup**: Combat active, budget per tick = 20
    - **Action**: Attempt expensive weave (cost = 30)
    - **Assert**: Weave queued for next tick or denied

12. **`test_multi_thread_simultaneous_manipulation`**
    - **Setup**: 3 threads requiring simultaneous splice
    - **Action**: Execute multi-splice operation
    - **Assert**: All threads connected, single atomic operation

13. **`test_thread_stability_decay_over_time`**
    - **Setup**: Thread with 100% stability
    - **Action**: Wait 60 seconds (no maintenance)
    - **Assert**: Stability decays to 80%, repair prompt appears

14. **`test_thread_repair_resource_costs`**
    - **Setup**: Damaged thread (50% stability), 1 Echo Shard available
    - **Action**: Repair thread
    - **Assert**: Stability restored to 100%, Echo Shard consumed

15. **`test_thread_fracture_from_cleave_combo`**
    - **Setup**: Boss uses Cleave Combo (see OATHBOUND_WARDEN_ENCOUNTER.md)
    - **Action**: Last hit lands on player near thread anchor
    - **Assert**: Thread fractures, stability drops to 60%

16. **`test_thread_tension_accumulation`**
    - **Setup**: Multiple weave actions in short duration
    - **Action**: Perform 5 weaves in 10 seconds
    - **Assert**: Thread tension increases, repair cost +20%

17. **`test_thread_resonance_patterns`**
    - **Setup**: 3 threads in proximity (< 5 m apart)
    - **Action**: Stabilize one thread
    - **Assert**: Adjacent threads gain +10% stability (resonance effect)

18. **`test_thread_anchor_binding`**
    - **Setup**: Thread and anchor at same position
    - **Action**: Bind thread to anchor
    - **Assert**: Thread stability locked to anchor stability

19. **`test_thread_visibility_occlusion`**
    - **Setup**: Thread behind solid geometry
    - **Action**: Query thread visibility
    - **Assert**: Thread occluded, shader applies fade effect

20. **`test_thread_determinism_fixed_seed`**
    - **Setup**: RNG seed = 12345
    - **Action**: Perform sequence of 10 weave operations
    - **Assert**: Run 3 times, all operations produce identical results

**Estimated Time**: 3-3.5 hours (10 minutes per test average)

---

### Category 3: Pattern Detection Edge Cases (15 tests) - **P2 MEDIUM**

**Why P2**: Pattern detection drives emergent weaving behavior. Existing 7 tests cover basics; these tests validate edge cases and performance under load.

**File**: `astraweave-weaving/tests/pattern_detection_edge_tests.rs` (new)

#### Tests to Implement:

1. **`test_low_health_cluster_boundary_conditions`**
   - **Setup**: 3 entities at exactly 25% health (threshold)
   - **Action**: Detect low health cluster
   - **Assert**: Pattern detected with PatternStrength::Moderate

2. **`test_resource_scarcity_gradual_depletion`**
   - **Setup**: Resource count: 100 → 50 → 10 → 0 over 60 seconds
   - **Action**: Monitor scarcity detection
   - **Assert**: Pattern strength increases: Weak → Moderate → Strong → Extreme

3. **`test_faction_conflict_multi_faction`**
   - **Setup**: 3 factions (A, B, C), each with 5 entities
   - **Action**: Detect faction conflict
   - **Assert**: All pairwise conflicts detected (A-B, A-C, B-C)

4. **`test_combat_intensity_spike`**
   - **Setup**: Combat intensity 10% → 90% in single frame
   - **Action**: Detect intensity change
   - **Assert**: Spike detected, emergency weave prompt triggered

5. **`test_multi_pattern_simultaneous_detection`**
   - **Setup**: Low health cluster + resource scarcity + faction conflict
   - **Action**: Run pattern detection
   - **Assert**: All 3 patterns detected, prioritized by strength

6. **`test_pattern_detection_performance_100_entities`**
   - **Setup**: World with 100 entities, 5 pattern detectors
   - **Action**: Run single detection pass
   - **Assert**: Completes in < 1 ms (60 FPS budget)

7. **`test_pattern_detection_performance_1000_entities`**
   - **Setup**: World with 1000 entities, 5 pattern detectors
   - **Action**: Run single detection pass
   - **Assert**: Completes in < 5 ms (warning threshold)

8. **`test_pattern_strength_threshold_exact_match`**
   - **Setup**: Pattern with strength = 0.50 (threshold)
   - **Action**: Query pattern above/below threshold
   - **Assert**: Threshold = 0.50 included in "above" category

9. **`test_pattern_cooldown_interactions`**
   - **Setup**: Pattern detector with 10-tick cooldown
   - **Action**: Detect pattern, wait 5 ticks, attempt re-detection
   - **Assert**: Re-detection blocked, cooldown remaining = 5

10. **`test_pattern_priority_conflicts`**
    - **Setup**: Two patterns with equal strength (0.75)
    - **Action**: Resolve priority
    - **Assert**: Deterministic tie-breaking (alphabetical by pattern ID)

11. **`test_pattern_metadata_serialization`**
    - **Setup**: Pattern with complex metadata map
    - **Action**: Serialize to JSON, deserialize
    - **Assert**: Metadata preserved exactly

12. **`test_pattern_detector_configuration`**
    - **Setup**: PatternDetector with custom thresholds
    - **Action**: Load from TOML config
    - **Assert**: All thresholds applied correctly

13. **`test_pattern_signal_propagation`**
    - **Setup**: Pattern detected at origin (0, 0, 0)
    - **Action**: Propagate signal to entities within 10 m radius
    - **Assert**: All entities within radius receive signal, others do not

14. **`test_pattern_filtering_by_range`**
    - **Setup**: 10 entities at various distances (2 m, 5 m, 10 m, 20 m)
    - **Action**: Detect patterns within 8 m range
    - **Assert**: Only entities ≤ 8 m considered, others ignored

15. **`test_pattern_determinism_fixed_seed`**
    - **Setup**: RNG seed = 54321
    - **Action**: Run pattern detection 10 times
    - **Assert**: All runs produce identical pattern set and strengths

**Estimated Time**: 2-2.5 hours (9 minutes per test average)

---

### Category 4: Integration Tests (15 tests) - **P1 HIGH**

**Why P1**: Integration tests validate cross-system interactions critical for vertical slice gameplay. Must pass before Week 2 mechanics implementation.

**File**: `astraweave-weaving/tests/integration_tests.rs` (new)

#### Tests to Implement:

1. **`test_z1_tutorial_progression_sequence`**
   - **Setup**: Z1 Frayed Causeway with tutorial anchors
   - **Action**: Walk through tutorial sequence (tut_start → stabilize → tut_success)
   - **Assert**: All triggers activate, anchors stabilize, bridge repairs

2. **`test_companion_ai_weave_support_action`**
   - **Setup**: Aria companion, unstable anchor, Echo Charge = 2
   - **Action**: Aria executes `CastStabilityPulse` GOAP action
   - **Assert**: Anchor stabilized, Echo Charge -= 1, telemetry event emitted

3. **`test_boss_anchor_targeting_logic`**
   - **Setup**: Oathbound Warden Phase 2, 2 stable arena anchors
   - **Action**: Boss selects anchor to rupture
   - **Assert**: `Anchor Rupture` targets deterministically (never same anchor twice)

4. **`test_storm_choice_determinism_3_runs`**
   - **Setup**: Loom Crossroads choice prompt, RNG seed = 99999
   - **Action**: Run 3 playthroughs, always choose "Stabilize"
   - **Assert**: All 3 runs produce identical timeline branch and boss modifiers

5. **`test_echo_grove_combat_encounter`**
   - **Setup**: Z2 Echo Grove, 4 Rift Stalkers + 1 Sentinel
   - **Action**: Complete combat, deploy barrier anchors
   - **Assert**: All enemies defeated, barriers created, Echo Dash unlock granted

6. **`test_loom_crossroads_choice_branching`**
   - **Setup**: Z3 choice prompt
   - **Action**: Select "Redirect" option
   - **Assert**: Timeline branches, storm_redirect flag set, boss arena modifiers change

7. **`test_oathbound_warden_adaptive_abilities`**
   - **Setup**: Boss Phase 3, player dealt 40% ranged damage
   - **Action**: Boss enters Directive Override
   - **Assert**: `AntiRangedField` selected, boss event emitted

8. **`test_post_run_recap_metric_capture`**
   - **Setup**: Complete 30-minute playthrough
   - **Action**: View post-run recap
   - **Assert**: Metrics displayed: anchors repaired, combo count, adaptive unlock

9. **`test_full_30_minute_playthrough_smoke`**
   - **Setup**: Clean slate, Z0 → Z4 full sequence
   - **Action**: Automated playthrough (simulated inputs)
   - **Assert**: No crashes, all major triggers activate, boss defeated

10. **`test_cell_streaming_metadata_refresh`**
    - **Setup**: World partition with 6 cells
    - **Action**: Stream Z1 → Z2 → Z3 (sequential loading)
    - **Assert**: Metadata refreshes on each load, anchors extracted correctly

11. **`test_trigger_activation_sequences`**
    - **Setup**: Z1 with 3 triggers (tut_start, tut_anchor_hold, tut_success)
    - **Action**: Activate in sequence
    - **Assert**: `TriggerVolumeEvent` emitted for each, tutorial state updated

12. **`test_anchor_repair_during_boss_phases`**
    - **Setup**: Boss Phase 2 (Fulcrum Shift), anchor ruptured
    - **Action**: Player repairs anchor while dodging boss attacks
    - **Assert**: Anchor stabilized, boss cannot re-rupture same anchor

13. **`test_multi_system_coordination_ai_weaving_combat`**
    - **Setup**: Combat encounter with Aria companion, unstable anchors
    - **Action**: Player engages enemies, Aria stabilizes anchors
    - **Assert**: Combat completes, anchors stable, synergy bonus applied

14. **`test_telemetry_event_ordering`**
    - **Setup**: Sequence of events (combat start, anchor stabilize, boss phase)
    - **Action**: Record telemetry events
    - **Assert**: Events ordered correctly by timestamp, no duplicates

15. **`test_dialogue_node_branching_validation`**
    - **Setup**: Storm choice "Stabilize" selected, adaptive unlock "Threadbind Riposte"
    - **Action**: Trigger outro dialogue
    - **Assert**: Dialogue nodes match branch (n11_stable with Threadbind banter)

**Estimated Time**: 2.5-3 hours (12 minutes per test average)

---

### Category 5: Determinism Tests (10 tests) - **P0 CRITICAL**

**Why P0**: Determinism is a core engine requirement (see Phase 3 Gap 2 completion, Option 3 validation). Weaving must maintain 100% deterministic behavior for multiplayer and replay.

**File**: `astraweave-weaving/tests/determinism_tests.rs` (new)

#### Tests to Implement:

1. **`test_fixed_seed_replay_3_runs`**
   - **Setup**: RNG seed = 11111
   - **Action**: Run weaving sequence 3 times (10 operations per run)
   - **Assert**: All 3 runs produce bit-identical results (hash comparison)

2. **`test_fixed_seed_replay_100_operations`**
   - **Setup**: RNG seed = 22222
   - **Action**: Run 100 weaving operations
   - **Assert**: Replay produces identical state, no drift

3. **`test_multiplayer_state_synchronization`**
   - **Setup**: 2 simulated clients, shared world state
   - **Action**: Client A performs weave, sync to Client B
   - **Assert**: Both clients have identical weave state

4. **`test_event_ordering_guarantees`**
   - **Setup**: Multiple weave events in single frame
   - **Action**: Process events
   - **Assert**: Events processed in deterministic order (entity ID sort)

5. **`test_storm_choice_branch_consistency`**
   - **Setup**: RNG seed = 33333
   - **Action**: Run 3 playthroughs, choose "Stabilize" each time
   - **Assert**: All 3 runs produce identical boss modifiers and timeline states

6. **`test_boss_adaptive_ability_determinism`**
   - **Setup**: Fixed player damage pattern (60% melee, 40% ranged)
   - **Action**: Boss Phase 3 adaptive selection
   - **Assert**: Always selects `CounterShockAura` (deterministic decision tree)

7. **`test_companion_goap_plan_determinism`**
   - **Setup**: World state with 3 goals active, RNG seed = 44444
   - **Action**: Aria plans action sequence
   - **Assert**: 3 runs produce identical action sequence

8. **`test_weaving_pattern_detection_determinism`**
   - **Setup**: 50 entities, 5 pattern detectors, RNG seed = 55555
   - **Action**: Run pattern detection
   - **Assert**: All runs detect identical patterns with identical strengths

9. **`test_tutorial_sequence_determinism`**
   - **Setup**: Z1 tutorial, RNG seed = 66666
   - **Action**: Complete tutorial 3 times
   - **Assert**: Identical anchor stabilization order and timing

10. **`test_echo_shard_collection_determinism`**
    - **Setup**: 5 Echo Shards in world, RNG seed = 77777
    - **Action**: Collect shards in various orders
    - **Assert**: Final resource count identical across 3 runs

**Estimated Time**: 1.5-2 hours (12 minutes per test average)

---

## Implementation Strategy

### Phase 1: Test Fixtures (30 minutes)

Create shared test utilities to reduce code duplication:

**File**: `astraweave-weaving/tests/common/mod.rs`

```rust
use astraweave_ecs::World;
use astraweave_weaving::*;

/// Create a mock world with basic entities
pub fn create_test_world(entity_count: usize) -> World {
    let mut world = World::new();
    // Add entities, components
    world
}

/// Create a mock weave anchor spec
pub fn create_test_anchor(anchor_id: &str, stability: f32) -> WeaveAnchorSpec {
    WeaveAnchorSpec {
        cell: GridCoord(100, 0, 0),
        position: [0.0, 0.0, 0.0],
        anchor_id: anchor_id.to_string(),
        anchor_type: Some("structural".to_string()),
        stability: Some(stability.to_string()),
        echo_cost: Some(1.0),
    }
}

/// Create a mock tutorial state
pub fn create_test_tutorial_state() -> WeaveTutorialState {
    WeaveTutorialState::default()
}

/// Run deterministic test (3 identical runs)
pub fn assert_deterministic_behavior<F>(seed: u64, test_fn: F)
where
    F: Fn(&mut World) -> Vec<u8>, // Returns hash of final state
{
    let results: Vec<Vec<u8>> = (0..3)
        .map(|_| {
            let mut world = create_test_world_with_seed(seed);
            test_fn(&mut world)
        })
        .collect();

    assert_eq!(results[0], results[1], "Run 1 != Run 2");
    assert_eq!(results[1], results[2], "Run 2 != Run 3");
}

/// Create world with fixed RNG seed
pub fn create_test_world_with_seed(seed: u64) -> World {
    let mut world = World::new();
    // Initialize RNG with seed
    world
}
```

### Phase 2: Sequential Implementation (6-8 hours)

**Hour 1**: Category 5 (Determinism) - P0 Critical
- Implement 10 determinism tests
- Use `assert_deterministic_behavior` helper
- Validate fixed seed replay works correctly

**Hour 2**: Category 1 (Anchor Stabilization Part 1) - P0 Critical
- Implement tests 1-8 (basic repair, resources, sequencing)
- Use `create_test_anchor` helper

**Hour 3**: Category 1 (Anchor Stabilization Part 2) - P0 Critical
- Implement tests 9-15 (respawn, metadata, events)
- Validate integration with tutorial system

**Hour 4**: Category 4 (Integration Part 1) - P1 High
- Implement tests 1-8 (tutorial, companion, boss, storm choice)
- Use full vertical slice fixtures

**Hour 5**: Category 4 (Integration Part 2) - P1 High
- Implement tests 9-15 (recap, streaming, triggers, multi-system)
- Smoke test 30-minute playthrough

**Hour 6**: Category 2 (Thread Manipulation Part 1) - P1 High
- Implement tests 1-10 (snip, splice, knot, timeline branching)
- Use mock thread state

**Hour 7**: Category 2 (Thread Manipulation Part 2) - P1 High
- Implement tests 11-20 (budget, decay, fracture, determinism)
- Validate causality checks

**Hour 8**: Category 3 (Pattern Detection Edge Cases) - P2 Medium
- Implement all 15 tests (boundary conditions, performance, filtering)
- Run performance benchmarks (100, 1000 entities)

### Phase 3: Validation (30 minutes)

1. **Run full test suite**:
   ```powershell
   cargo test -p astraweave-weaving --lib
   ```
   - Target: 96/96 tests passing (21 existing + 75 new)

2. **Generate coverage report**:
   ```powershell
   cargo tarpaulin -p astraweave-weaving --out Lcov
   ```
   - Target: ≥80% line coverage

3. **Validate integration**:
   ```powershell
   cargo test -p veilweaver_slice_runtime --test integration_tests
   ```
   - Ensure new tests integrate with runtime harness

4. **Document results**:
   - Update `FOUNDATION_AUDIT_REPORT.md` with coverage metrics
   - Create `WEAVING_TEST_COMPLETION.md` with summary

---

## Success Criteria

### Completion Checklist:
- [ ] **75 new tests implemented** across 5 categories
- [ ] **96+ total tests passing** (21 existing + 75 new)
- [ ] **≥80% line coverage** validated via tarpaulin
- [ ] **All P0/P1 tests passing** (determinism + anchor stabilization + integration)
- [ ] **Test fixtures created** (`common/mod.rs` with 5+ helper functions)
- [ ] **Performance validated** (pattern detection < 5 ms @ 1000 entities)
- [ ] **Documentation updated** (completion report, coverage metrics)

### Quality Gates:
- ✅ **No test flakiness**: All tests pass 10 consecutive runs
- ✅ **Determinism validated**: Fixed seed tests pass 100% reliably
- ✅ **Integration validated**: Full 30-minute playthrough smoke test passes
- ✅ **Performance validated**: No tests exceed 60 FPS budget (16.67 ms)

---

## Risks & Mitigation

### Risk 1: Missing API Surface
- **Problem**: Test implementation reveals missing methods in `astraweave-weaving` API
- **Mitigation**: Extend API as needed (e.g., add `Anchor::set_stability()` if missing)
- **Impact**: +1-2 hours implementation time

### Risk 2: Test Fixture Complexity
- **Problem**: Creating realistic world state for integration tests is time-consuming
- **Mitigation**: Use simplified mock states, focus on behavior over realism
- **Impact**: Accept some integration tests as "smoke tests" rather than comprehensive

### Risk 3: Performance Bottlenecks
- **Problem**: Tests reveal performance issues (e.g., pattern detection > 5 ms)
- **Mitigation**: Document bottleneck, defer optimization to Week 4+ (not a blocker)
- **Impact**: Week 4 optimization work may be required

### Risk 4: Determinism Edge Cases
- **Problem**: Some operations are non-deterministic (e.g., HashMap iteration order)
- **Mitigation**: Use `BTreeMap` consistently, sort entity IDs before processing
- **Impact**: May require refactoring some `astraweave-weaving` internals

---

## Post-Completion Actions

### Immediate (Day 2 Evening):
1. **Update `FOUNDATION_AUDIT_REPORT.md`**:
   - Change weaving system status from ⚠️ to ✅
   - Update coverage metrics (9.47% → 80%+)
   - Mark P0 blocker as resolved

2. **Create completion report**:
   - **File**: `WEAVING_TEST_COMPLETION.md`
   - Document: tests implemented, coverage achieved, issues found
   - Include: performance metrics, determinism validation results

3. **Notify stakeholders**:
   - Foundation audit complete, weaving system validated
   - Ready to begin Week 1 greybox work (Day 3)

### Week 1 Integration:
1. **Run tests during greybox work**:
   - Validate metadata parsing from `.ron` scene descriptors
   - Ensure tutorial triggers integrate correctly

2. **Expand tests as needed**:
   - Add edge cases discovered during greybox implementation
   - Target: Maintain ≥80% coverage throughout vertical slice development

### Week 2+ Validation:
1. **Use tests to validate mechanics**:
   - Week 2: Run anchor stabilization tests during tutorial implementation
   - Week 3: Run integration tests during companion AI implementation
   - Week 4: Run boss tests during Oathbound Warden implementation

2. **Determinism checks**:
   - Run determinism test suite before each milestone
   - Validate 3-run consistency at end of each week

---

## Appendix A: Test Naming Conventions

**Format**: `test_<component>_<scenario>_<expected_behavior>`

**Examples**:
- ✅ `test_anchor_single_basic_repair`
- ✅ `test_thread_snip_budget_check`
- ✅ `test_pattern_detection_performance_1000_entities`
- ❌ `test_stuff` (too vague)
- ❌ `anchor_test` (missing `test_` prefix)

**Components**:
- `anchor` - Anchor stabilization
- `thread` - Thread manipulation
- `pattern` - Pattern detection
- `integration` - Cross-system tests
- `determinism` - Fixed seed replay

**Scenarios**:
- `basic` - Happy path
- `insufficient_resources` - Error condition
- `multi` - Multiple entities/operations
- `performance` - Performance validation
- `determinism` - Fixed seed validation

---

## Appendix B: Example Test Implementation

```rust
// File: astraweave-weaving/tests/anchor_stabilization_tests.rs

use astraweave_weaving::*;
mod common;

#[test]
fn test_anchor_single_basic_repair() {
    // Setup: Single unstable anchor, player has 1 Echo Shard
    let mut world = common::create_test_world(1);
    let anchor = common::create_test_anchor("anchor_a", 30.0); // 30% stability
    let player_echo_shards = 1;

    // Action: Apply stabilization pulse
    let result = apply_stabilization_pulse(&mut world, &anchor, player_echo_shards);

    // Assert: Anchor stabilized, Echo Shard consumed, event emitted
    assert!(result.is_ok());
    let anchor_state = world.get_anchor_state("anchor_a").unwrap();
    assert_eq!(anchor_state.stability, 100.0, "Anchor should be fully stabilized");
    assert_eq!(world.get_player_echo_shards(), 0, "Echo Shard should be consumed");
    
    let events = world.get_events::<AnchorStabilizedEvent>();
    assert_eq!(events.len(), 1, "Should emit AnchorStabilizedEvent");
    assert_eq!(events[0].anchor_id, "anchor_a");
}

#[test]
fn test_determinism_fixed_seed_replay_3_runs() {
    common::assert_deterministic_behavior(11111, |world| {
        // Perform 10 weaving operations
        for i in 0..10 {
            perform_weave_operation(world, i);
        }
        
        // Return hash of final state
        compute_world_state_hash(world)
    });
    // Helper automatically validates 3 runs produce identical hashes
}
```

---

**Report Prepared By**: AstraWeave Copilot (AI Orchestration)  
**Next Action**: Begin Phase 1 (Test Fixtures) - Day 1 Morning  
**Estimated Completion**: Day 2 Evening (6-8 hours total)
