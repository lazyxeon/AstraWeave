# Phase 3: Learning & Persistence - Implementation Plan

## Vision
Enable the GOAP AI to learn from experience, improving planning quality across play sessions through persistent action history and adaptive cost/risk calculations.

**Duration**: Weeks 6-8 (Accelerated - targeting single session)

**Status**: 🚀 **STARTING**

---

## Phase 3 Goals

### Primary Objectives
1. **Persistent Learning**: `ActionHistory` survives game sessions via save system
2. **Closed Learning Loop**: Execution outcomes → Telemetry → History → Improved plans
3. **Tunable Parameters**: Cost/risk weights configurable via TOML
4. **Adaptive Probabilities**: Success rate estimates improve with experience
5. **Retention Policies**: Clear rules for what data persists and for how long

### Success Metrics
- ≥10% improvement in success probability for repeated encounter types over 30 minutes
- Action history persists across save/load cycles without corruption
- Config changes take effect without code recompilation
- Learning converges within 10-15 repetitions of similar scenarios

---

## Architecture Overview

### Current State (End of Phase 2)
```
GameWorld → WorldSnapshot → GOAPOrchestrator → PlanIntent → validate_and_execute()
                                ↓
                          AdvancedGOAP (uses ActionHistory for costs)
                                ↑
                          ActionHistory (in-memory only)
```

**Gap**: Execution outcomes don't feed back to `ActionHistory`, and history is lost on restart.

### Target State (Phase 3)
```
GameWorld → WorldSnapshot → GOAPOrchestrator → PlanIntent → validate_and_execute()
     ↓                           ↓                               ↓
ExecutionOutcome ----→ TelemetryCollector ----→ ActionHistory → AdvancedGOAP
     ↑                                                ↓
     |                                          SaveSystem
     |                                          (persistence)
     ↓
LearningConfig (TOML) ----→ Cost/Risk tuning
```

**Key Additions**:
1. **ExecutionOutcome** events from `validate_and_execute`
2. **Learning feedback** from telemetry to history
3. **Persistence layer** via `astraweave-memory` (or custom JSON/bincode)
4. **Config system** for tuning parameters

---

## Implementation Tasks

### Task 1: ActionHistory Persistence Schema ✅ In Progress

**Goal**: Define serialization format and save/load interface for `ActionHistory`.

**Subtasks**:
1. Add `serde` derive to `ActionHistory` and `ActionStats`
2. Create `HistoryPersistence` module with save/load functions
3. Define JSON schema for human-readable inspection
4. Add optional bincode for compact binary format
5. Implement checksum validation to detect corruption

**Files**:
- `astraweave-ai/src/goap/history.rs` (update)
- `astraweave-ai/src/goap/persistence.rs` (new)

**Schema Design**:
```rust
#[derive(Serialize, Deserialize)]
pub struct PersistedHistory {
    version: u32,
    timestamp: u64,
    checksum: u64,
    history: ActionHistory,
}
```

**Integration Points**:
- Save: On game save or periodic autosave
- Load: On game load, merge with existing history
- Fallback: If load fails, start with empty history (graceful degradation)

---

### Task 2: TOML Configuration System

**Goal**: Expose tuning parameters for designers/players without recompilation.

**Config Schema** (`config/goap_learning.toml`):
```toml
[learning]
enabled = true
initial_success_rate = 0.75
min_success_rate = 0.1
max_success_rate = 0.95

[learning.smoothing]
method = "ewma"  # "ewma" or "bayesian"
ewma_alpha = 0.2
bayesian_prior_successes = 3
bayesian_prior_failures = 1

[cost_tuning]
base_cost_multiplier = 1.0
risk_weight = 5.0
health_critical_threshold = 30
health_wounded_threshold = 60
ammo_critical_threshold = 5
ammo_low_threshold = 10

[persistence]
enabled = true
save_interval_seconds = 60
retention_days = 30
max_entries_per_action = 1000
prune_threshold = 10000
```

**Implementation**:
- Use `serde` + `toml` crate
- Load on startup, watch for file changes (optional hot-reload)
- Validate ranges (e.g., success rates must be 0.0-1.0)
- Provide defaults if config missing

**Files**:
- `astraweave-ai/src/goap/config.rs` (new)
- `config/goap_learning.toml` (new)

---

### Task 3: Execution Outcome Tracking

**Goal**: Capture results of executed actions and feed them into the learning system.

**Execution Flow**:
```rust
// In validate_and_execute or after action execution
let outcome = ExecutionOutcome {
    action_name: "attack".to_string(),
    success: true,
    duration: Duration::from_millis(150),
    timestamp: game_time,
    context: Some(context_snapshot),
};

telemetry_collector.record_outcome(outcome);
// → Telemetry updates ActionHistory
// → ActionHistory influences future planning costs
```

**New Types**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutcome {
    pub action_name: String,
    pub success: bool,
    pub duration: Duration,
    pub timestamp: f32,
    pub context: Option<CompactWorldState>, // Optional: for context-aware learning
}
```

**Integration**:
- Hook into `validate_and_execute` (astraweave-core)
- Emit `ExecutionOutcome` events
- `GOAPOrchestrator` listens and updates its `ActionHistory`
- Telemetry system also records for analytics

**Files**:
- `astraweave-ai/src/goap/telemetry.rs` (update)
- `astraweave-ai/src/goap/learning.rs` (new - learning loop logic)
- `astraweave-core/src/validation.rs` (update - emit outcomes)

---

### Task 4: Adaptive Success Probability

**Goal**: Implement smoothing algorithms so success probability estimates improve with experience.

**Current**: `ActionHistory` tracks raw successes/failures, returns simple rate.

**Enhancement**: Add temporal smoothing for better estimates.

#### Option A: EWMA (Exponentially Weighted Moving Average)
```rust
impl ActionStats {
    pub fn success_probability_ewma(&self, alpha: f32) -> f32 {
        // Recent data weighted more heavily
        let raw_rate = self.success_rate();
        self.ewma_estimate.unwrap_or(raw_rate) * (1.0 - alpha) + raw_rate * alpha
    }
}
```

**Pros**: Simple, fast, adapts quickly to recent changes
**Cons**: No confidence intervals

#### Option B: Bayesian Estimation
```rust
impl ActionStats {
    pub fn success_probability_bayesian(&self, prior_successes: u32, prior_failures: u32) -> f32 {
        let posterior_successes = self.successes + prior_successes;
        let posterior_failures = self.failures + prior_failures;
        posterior_successes as f32 / (posterior_successes + posterior_failures) as f32
    }
}
```

**Pros**: Handles sparse data well, confidence intervals available
**Cons**: Slightly more complex

**Implementation**: Support both, selected via config.

**Files**:
- `astraweave-ai/src/goap/history.rs` (update)
- `astraweave-ai/src/goap/learning.rs` (smoothing algorithms)

---

### Task 5: Retention Policies

**Goal**: Define what data persists, for how long, and pruning strategies.

**Policies**:

1. **Action History Retention**
   - Keep last 1000 executions per action (configurable)
   - Prune entries older than 30 days (configurable)
   - Retain aggregate stats indefinitely (successes, failures, avg duration)

2. **Telemetry Retention**
   - In-memory ring buffer: 1000 events (current)
   - Persistent log: Last 7 days of detailed events (optional)
   - Aggregate metrics: Indefinite retention

3. **Pruning Strategy**
   - On save: Prune old entries before serialization
   - On load: Validate and prune if over threshold
   - Runtime: Periodic pruning every 5 minutes (if enabled)

**Implementation**:
```rust
impl ActionHistory {
    pub fn prune(&mut self, max_age_days: u32, max_entries_per_action: usize) {
        let cutoff_time = current_time() - Duration::from_days(max_age_days);
        for stats in self.stats.values_mut() {
            stats.prune_old_entries(cutoff_time, max_entries_per_action);
        }
    }
}
```

**Files**:
- `astraweave-ai/src/goap/history.rs` (update)
- `astraweave-ai/src/goap/persistence.rs` (pruning logic)

---

### Task 6: Integration Testing

**Goal**: Validate that learning actually improves planning over time.

**Test Scenarios**:

1. **Learning Convergence Test**
   - Run same scenario 20 times
   - Verify success probability increases from initial ~75% to ~90%+
   - Confirm costs adjust based on observed outcomes

2. **Persistence Test**
   - Build ActionHistory with 100 action executions
   - Save to disk
   - Load in new instance
   - Verify all stats match

3. **Config Validation Test**
   - Load config with various parameter values
   - Verify range validation (reject invalid values)
   - Confirm defaults applied when config missing

4. **Pruning Test**
   - Create history with 2000 entries, mix of old and recent
   - Apply pruning with 1000 entry limit
   - Verify oldest entries removed, recent retained

5. **Smoothing Algorithm Test**
   - Compare EWMA vs Bayesian vs raw rate
   - Verify EWMA adapts faster to recent changes
   - Verify Bayesian handles sparse data better

**Files**:
- `astraweave-ai/tests/goap_learning_tests.rs` (new)
- `astraweave-ai/src/goap/learning.rs` (test utilities)

---

## Implementation Order

### Week 1 (Days 1-2): Foundation
1. ✅ Task 1.1: Add serde to ActionHistory/ActionStats
2. ✅ Task 1.2: Create persistence module with save/load
3. ✅ Task 2.1: Define TOML config schema
4. ✅ Task 2.2: Implement config loading with validation

### Week 1 (Days 3-4): Learning Loop
5. ✅ Task 3.1: Define ExecutionOutcome type
6. ✅ Task 3.2: Hook telemetry to ActionHistory updates
7. ✅ Task 4.1: Implement EWMA smoothing
8. ✅ Task 4.2: Implement Bayesian smoothing

### Week 2 (Days 1-2): Retention & Polish
9. ✅ Task 5.1: Implement pruning logic
10. ✅ Task 5.2: Add retention policy enforcement
11. ✅ Task 1.3: Add checksum validation
12. ✅ Task 2.3: Add config hot-reload (optional)

### Week 2 (Days 3-4): Testing & Documentation
13. ✅ Task 6: Integration tests (all scenarios)
14. ✅ Task 7: Documentation (config guide, learning system explanation)
15. ✅ Phase 3 completion report

---

## Technical Decisions

### Persistence Format: JSON + optional bincode
**Rationale**: JSON for human readability and debugging, bincode for production performance.

### Smoothing Default: EWMA with α=0.2
**Rationale**: Simple, fast, good balance between stability and adaptation. Bayesian available for sparse data scenarios.

### Retention Default: 30 days, 1000 entries/action
**Rationale**: Balances learning effectiveness with disk space. Configurable for different use cases.

### Config Location: `config/goap_learning.toml`
**Rationale**: Separate from code, easily modified by designers. Standard TOML format.

---

## Success Criteria

### Learning Effectiveness
- [ ] Success probability improves ≥10% over 30 minutes of play in repeated scenarios
- [ ] Costs adapt based on observed failures (risky actions become more expensive)
- [ ] Plans generated after learning are more reliable than initial plans

### Persistence Robustness
- [ ] ActionHistory survives save/load cycle with 100% fidelity
- [ ] Checksum detects corrupted data
- [ ] Graceful fallback if load fails (empty history, no crash)

### Configuration Flexibility
- [ ] All tuning parameters accessible via TOML
- [ ] Invalid configs rejected with clear error messages
- [ ] Defaults provide good out-of-box experience

### Performance
- [ ] Save operation < 50ms for typical history (1000 entries)
- [ ] Load operation < 100ms
- [ ] Pruning operation < 10ms
- [ ] Learning overhead < 1ms per action execution

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| **History corruption** | High - AI stops learning | Checksum validation, backup on save |
| **Config typos break AI** | Medium - Bad planning | Validation with fallback to defaults |
| **Learning instability** | Medium - Oscillating costs | Smoothing algorithms, min/max bounds |
| **Disk space growth** | Low - Retention policies | Aggressive pruning, configurable limits |
| **Performance regression** | Low - Save/load overhead | Async save, lazy load, pruning |

---

## Deliverables

1. **Persistence Module** (`goap/persistence.rs`)
   - Save/load ActionHistory with checksum validation
   - JSON + bincode support
   - Pruning and retention enforcement

2. **Config System** (`goap/config.rs`, `config/goap_learning.toml`)
   - TOML-based tuning parameters
   - Validation and defaults
   - (Optional) Hot-reload

3. **Learning Loop** (`goap/learning.rs`)
   - ExecutionOutcome tracking
   - Telemetry → ActionHistory feedback
   - EWMA and Bayesian smoothing

4. **Integration Tests** (`tests/goap_learning_tests.rs`)
   - Learning convergence
   - Persistence validation
   - Config validation
   - Pruning correctness

5. **Documentation**
   - `docs/phase3_learning_persistence.md` - Technical details
   - `docs/goap_config_guide.md` - Designer-facing config reference
   - `docs/phase3_completion_report.md` - Final report

---

## Next Actions

**Immediate**: Begin Task 1 - Add serde to ActionHistory and create persistence module.

**Follow-up**: Once persistence is working, implement TOML config system for tuning.

**Validation**: After core loop is closed, run learning convergence tests to validate improvement.

---

**Phase 3 Status**: 🚀 **STARTING** - Persistence design in progress

**Feature Flag**: `planner_advanced` (continue using existing flag)

**Target Completion**: End of Week 2 (or single accelerated session)

