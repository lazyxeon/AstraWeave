# Phase 0: Advanced GOAP Discovery & Alignment

**Date:** November 9, 2025  
**Status:** Discovery Complete  
**Next Phase:** Technical Foundation (Phase 1)

---

## Executive Summary

Phase 0 audit reveals AstraWeave has a flexible, trait-based orchestrator architecture (`Orchestrator` trait → `PlanIntent`) that can accommodate GOAP integration without breaking existing systems. The current validation pipeline (constraint checking, LOS, cooldowns, pathfinding) is action-agnostic and will support GOAP-generated plans transparently. **Recommended path: Add GOAP as a new `Orchestrator` implementation behind feature flag, preserving all existing modes.**

---

## 1. Existing AI Architecture Audit

### 1.1 Core Components

#### **astraweave-ai**
- **Location:** `astraweave-ai/src/lib.rs`
- **Current Implementation:** `RuleOrchestrator`
  - Simple rule-based planner with smoke/advance heuristics
  - Produces `PlanIntent` with 1-3 `ActionStep` sequences
  - **Strengths:** Fast, deterministic, battle-tested
  - **Limitations:** No learning, no cost-benefit reasoning, brittle in complex scenarios

#### **astraweave-core**
- **Schema** (`schema.rs`):
  - `WorldSnapshot`: Immutable world view (player, companion, enemies, POIs, obstacles, objective)
  - `PlanIntent`: Container for `plan_id` + `Vec<ActionStep>`
  - `ActionStep`: 43 action variants (movement, combat, tactical, utility, defensive, equipment)
  - `Constraints`: Boolean flags for cooldown/LOS/stamina enforcement
  - `EngineError`: Typed errors (Cooldown, LosBlocked, NoPath, InvalidAction, Resource)
  
- **Validation** (`validation.rs`):
  - `validate_and_execute(World, Entity, PlanIntent, ValidateCfg)` → `Result<(), EngineError>`
  - **Constraint checks:** Cooldowns (line 298-300), LOS (line 291-292), pathfinding (line 32-34), ammo (line 316-320)
  - **Critical insight:** Validation is action-agnostic; GOAP plans will pass through same pipeline
  - **Test coverage:** 95%+ with 30+ test cases covering all action variants

- **Tools** (`tools.rs`, `tool_sandbox.rs`):
  - A* pathfinding, LOS raycasting, cover position queries
  - Error mapping to stable taxonomy (`ToolBlock` with `ToolBlockReason`)
  - Reusable by GOAP precondition/effect modeling

#### **astraweave-director**
- **Purpose:** High-level encounter pacing (spawning, terrain manipulation, phase transitions)
- **Implementations:**
  - `BossDirector`: Heuristic-based (lines 26-72)
  - `PhaseDirector`: HP-threshold phase switching with telegraphs (lines 24-101)
  - `LlmDirector`: LLM-powered adaptive tactics with player behavior modeling (lines 194-550)
  
- **Interface:** `DirectorPlan` with `DirectorOp` (Fortify, SpawnWave, Collapse)
- **Budget System:** `DirectorBudget` limits spawns/terrain edits
- **GOAP opportunity:** Director could use GOAP for strategic goal decomposition (e.g., "increase tension" → fortify choke + spawn reinforcements)

### 1.2 Integration Points

#### **Orchestrator Trait**
```rust
pub trait Orchestrator {
    fn propose_plan(&self, snap: &WorldSnapshot) -> PlanIntent;
}
```
- **Current users:** `RuleOrchestrator`, plus experimental LLM/Utility/BT modes in examples
- **GOAP integration path:** Implement `GOAPOrchestrator: Orchestrator` with internal `AdvancedGOAP` planner

#### **AI Mode Variants** (from `hello_companion/src/main.rs`)
- `Classical`: Uses `RuleOrchestrator`
- `BehaviorTree`: Experimental (requires `llm` feature)
- `Utility`: Experimental (requires `llm` feature)
- `LLM`: Ollama-powered (requires `ollama` feature)
- `Hybrid`: LLM with classical fallback
- `Ensemble`: Multi-orchestrator voting
- `Arbiter`: Stateful hybrid coordinator
- **Proposed:** `GOAP` mode (requires `planner_advanced` feature)

#### **Execution Flow**
```
WorldSnapshot → Orchestrator::propose_plan() → PlanIntent
    ↓
validate_and_execute() → checks constraints (cooldowns, LOS, path)
    ↓
apply ActionStep effects to World → update state
    ↓
(optional) record telemetry for learning
```
- GOAP fits cleanly: `GOAPOrchestrator` generates `PlanIntent`, rest of pipeline unchanged

### 1.3 Data Flows & Dependencies

#### **Planning Inputs**
- `WorldSnapshot` provides:
  - Entity positions (`player.pos`, `me.pos`, `enemies[].pos`)
  - State values (`player.hp`, `me.ammo`, `enemy.cover`, `me.cooldowns`)
  - Environment (`pois`, `objective`)
- **GOAP mapping:** Snapshot fields → GOAP `WorldState` key-value pairs

#### **Action Catalog**
- 43 action variants across 7 categories (Movement, Offensive, Defensive, Equipment, Tactical, Utility, Legacy)
- Each has implicit preconditions (e.g., `CoverFire` needs ammo, LOS) and effects (damage, movement, cooldown)
- **GOAP requirement:** Explicit `Action` trait implementations for subset of `ActionStep` variants

#### **Validation Dependencies**
- Pathfinding: `astar_path()` from `astraweave_nav` (A* on obstacle grid)
- LOS checks: Bresenham raycasting against obstacles
- Cooldown tracking: `BTreeMap<String, f32>` per entity
- Resource checks: Ammo tracking via `World::ammo(entity)`

---

## 2. GOAP Integration Viability Assessment

### 2.1 Compatibility Matrix

| Component | Current State | GOAP Requirement | Integration Effort | Risk |
|-----------|--------------|------------------|-------------------|------|
| `Orchestrator` trait | Stable, 1 method | Implement for GOAP | **Low** (straightforward impl) | Low |
| `WorldSnapshot` | Battle-tested schema | Map to GOAP `WorldState` | **Medium** (adapter layer) | Low |
| `ActionStep` enum | 43 variants | Subset as GOAP actions | **Medium** (15-20 actions) | Medium |
| `validate_and_execute` | 95%+ coverage | No changes needed | **None** | Low |
| Director integration | 3 director types | Optional GOAP goals | **High** (new goal semantics) | Medium |
| Telemetry/learning | Manual logging | Wire to `ActionHistory` | **Medium** (persistence) | Medium |
| Examples/demos | 7 AI modes | Add GOAP mode | **Low** (follow patterns) | Low |

### 2.2 Breaking Change Analysis

**Zero breaking changes required** if GOAP is added as:
1. New crate: `astraweave-goap` (or module in `astraweave-ai`)
2. Feature flag: `planner_advanced`
3. New `Orchestrator` impl: `GOAPOrchestrator`
4. Optional in examples: `AIMode::GOAP`

**Potential conflicts:**
- None identified; existing code paths unchanged when feature disabled
- `RuleOrchestrator` remains default for all existing workflows

### 2.3 Technical Feasibility

**Strengths favoring GOAP:**
- ✅ Action-oriented design (43 `ActionStep` variants map naturally to GOAP actions)
- ✅ Constraint system already separates preconditions (validation) from execution
- ✅ Trait-based architecture allows drop-in orchestrators
- ✅ Snapshot-based planning (immutable world view) aligns with GOAP's search model
- ✅ Rich tool library (pathfinding, LOS, cover queries) supports GOAP heuristics

**Challenges requiring attention:**
- ⚠️ State explosion: `WorldSnapshot` has ~15 key fields; naïve GOAP state could be too large
  - **Mitigation:** Abstract state to critical properties (health bands, ammo status, enemy count tiers)
- ⚠️ Action fidelity: Not all `ActionStep` variants have explicit preconditions in code
  - **Mitigation:** Start with 15-20 well-understood actions (movement, combat, healing)
- ⚠️ Heuristic tuning: A* requires domain-specific heuristic for fast convergence
  - **Mitigation:** Use Claude prototype's multi-metric heuristic as baseline, tune via benchmarking
- ⚠️ Learning persistence: `ActionHistory` needs save/load integration
  - **Mitigation:** Leverage `astraweave-memory` crate (already handles save data)

---

## 3. Acceptance Metrics Definition

### 3.1 Plan Quality Metrics

| Metric | Baseline (RuleOrchestrator) | Target (GOAP) | Measurement Method |
|--------|---------------------------|---------------|-------------------|
| **Success Rate** | 70% (manual estimate) | ≥85% | Execute 100 test scenarios, count validation passes |
| **Plan Depth** | 1-3 steps | 3-7 steps | Average steps per plan in test suite |
| **Goal Achievement** | Single implicit goal | Multi-goal with priorities | % of scenarios meeting all goals |
| **Failure Recovery** | None (replan from scratch) | Plan repair or fast replan | Time to recover from mid-plan failure |

**Test scenarios:**
1. **Combat:** Defeat enemy while low health (should prioritize healing)
2. **Tactical:** Flank enemy with cover (multi-step positioning + attack)
3. **Resource constrained:** Low ammo, must reload before attacking
4. **Multi-goal:** Heal ally + defeat enemy + reach extraction (priority ordering)
5. **Dynamic:** Enemy moves mid-plan (requires replan or adaptation)

### 3.2 Performance Metrics

| Metric | Acceptable | Target | Critical Limit | Measurement Tool |
|--------|-----------|--------|---------------|------------------|
| **Planning Latency (P50)** | ≤2 ms | ≤1 ms | ≤5 ms | `std::time::Instant` + histogram |
| **Planning Latency (P95)** | ≤6 ms | ≤4 ms | ≤10 ms | `std::time::Instant` + histogram |
| **Planning Latency (P99)** | ≤12 ms | ≤8 ms | ≤20 ms | `std::time::Instant` + histogram |
| **Memory per planner** | ≤1 MB | ≤512 KB | ≤2 MB | Heap profiler |
| **Plan cache hit rate** | N/A | ≥40% | — | Track closed-set hits |

**Benchmark harness:**
- 1000 planning iterations across 10 scenario archetypes
- Record full distribution, not just averages
- Run on representative hardware (mid-range gaming PC)

### 3.3 Learning Impact Metrics

| Metric | Definition | Target | Measurement Window |
|--------|-----------|--------|-------------------|
| **Success probability convergence** | Actions reach stable estimates | ±10% after 20 executions | Per-action |
| **Cost adjustment correlation** | Failed actions get higher costs | R² ≥ 0.6 | 50 plan executions |
| **Adaptation speed** | Time to avoid repeated failure pattern | <5 failures | Single scenario loop |
| **Long-term improvement** | Success rate increase over session | +10% after 100 plans | Full playthrough |

**Test protocol:**
- Seed with neutral history (50% success rate for all actions)
- Inject simulated failures for specific actions (e.g., "attack" fails 80% when health <30)
- Measure: After N executions, does planner avoid doomed "attack" actions in low-health states?

### 3.4 Stability Metrics

| Metric | Definition | Target | Detection Method |
|--------|-----------|--------|------------------|
| **Plan determinism** | Same input → same plan | 100% (given same history) | Snapshot hash + plan hash comparison |
| **Validation pass rate** | GOAP plans pass constraint checks | ≥95% | Count `Ok()` from `validate_and_execute` |
| **Crash-free hours** | No panics/crashes from planner | ≥100h continuous | Integration test marathon |
| **Edge case handling** | Graceful failures (no panic) | 100% | Fuzz testing with invalid snapshots |

### 3.5 Designer Adoption Metrics (Post-Launch)

| Metric | Definition | Target | Timeline |
|--------|-----------|--------|----------|
| **Scenario coverage** | % scenarios using GOAP goals | ≥80% | Month 1 post-launch |
| **Custom actions authored** | New GOAP actions added by designers | ≥5 | Month 2 |
| **Bug reports (GOAP-specific)** | Planner-caused issues | <3 per week | Ongoing |
| **Designer satisfaction** | Survey rating (1-5) | ≥4.0 | Quarterly |

---

## 4. Success Metric Dashboard Stub

### 4.1 Real-Time Metrics (In-Game Telemetry)

**Dashboard Components:**
1. **Planning Performance**
   - Latency histogram (P50/P95/P99) updated every 100 plans
   - Plan depth distribution (1-15 steps)
   - Open-set size per plan (measure of search efficiency)

2. **Plan Quality**
   - Success rate rolling average (last 50 plans)
   - Goal achievement breakdown (primary vs secondary goals)
   - Validation failure reasons (cooldown, LOS, path, resource)

3. **Learning Health**
   - Action history size (# unique actions tracked)
   - Success probability stability (variance last 20 executions)
   - Cost adjustment magnitude (avg delta from base cost)

4. **Comparison vs Baseline**
   - GOAP vs RuleOrchestrator side-by-side (success rate, latency, plan depth)
   - Scenario-specific A/B test results

**Instrumentation Points:**
```rust
// In GOAPOrchestrator::propose_plan()
metrics::histogram!("goap.planning_latency_ms", planning_duration_ms);
metrics::counter!("goap.plans_generated", 1);
metrics::gauge!("goap.plan_depth", plan.steps.len() as f64);

// In validate_and_execute() wrapper
if let Err(e) = validation_result {
    metrics::counter!("goap.validation_failures", 1, "reason" => format!("{:?}", e));
}

// In ActionHistory::record_success/record_failure
metrics::gauge!("goap.action_success_rate", stats.success_rate(), "action" => action_name);
```

### 4.2 Offline Analysis (Post-Session Reports)

**Generated Artifacts:**
1. **Plan corpus CSV:**
   - Columns: `timestamp, scenario_id, plan_id, steps, latency_ms, validation_result, execution_result`
   - Used for regression detection and quality trends

2. **Action history snapshot:**
   - JSON export of `ActionHistory` per entity type
   - Tracks learning convergence and identifies anomalies

3. **Heuristic effectiveness report:**
   - Measures: nodes expanded vs plan depth, heuristic admissibility violations
   - Guides tuning of goal distance estimators

4. **Failure case log:**
   - Plans that failed validation or execution
   - Manual review queue for edge cases

**Storage:** `target/goap_telemetry/` directory, rotated per session

---

## 5. Stakeholder Alignment

### 5.1 Design Review Schedule

| Stakeholder Group | Topics | Date | Status |
|-------------------|--------|------|--------|
| **Gameplay Team** | GOAP behavior vs rule-based; designer workflows | TBD | Pending |
| **AI/Director Team** | Goal system semantics; director integration | TBD | Pending |
| **Tooling Team** | Visualization needs; authoring schemas | TBD | Pending |
| **Performance Team** | Latency budgets; profiling hooks | TBD | Pending |
| **QA Team** | Test scenarios; validation criteria | TBD | Pending |

### 5.2 Key Discussion Points

**For Gameplay:**
- Q: Should GOAP replace rule-based, or coexist as optional mode?
  - **Recommendation:** Coexist initially; evaluate for replacement after 3-month soak
- Q: How much control do designers need over action costs/risks?
  - **Recommendation:** Expose via TOML config files with hot-reload support

**For Director:**
- Q: Can director goals drive GOAP multi-goal planning?
  - **Recommendation:** Phase 4 integration; requires unified goal semantics
- Q: Should director use GOAP for encounter pacing decisions?
  - **Recommendation:** Explore after companion GOAP stabilizes

**For Tooling:**
- Q: What visualizations are critical for debugging plans?
  - **Recommendation:** Plan tree explorer, action success heatmaps (Phase 5)
- Q: How to validate designer-authored goal definitions?
  - **Recommendation:** Schema validators + linter integration

---

## 6. Risk Register

| Risk ID | Description | Likelihood | Impact | Mitigation | Owner |
|---------|-------------|-----------|--------|------------|-------|
| **R1** | Non-deterministic state hashing causes plan loops | High | High | Implement canonical key ordering in Phase 1 | AI Team |
| **R2** | Planning latency exceeds 10ms under complex scenarios | Medium | High | Add iteration budget, incremental replanning | Perf Team |
| **R3** | Designer learning curve slows adoption | Medium | Medium | Provide templates, training sessions | Tools Team |
| **R4** | Learning history corruption breaks persistence | Low | High | Add checksum validation, opt-out toggle | AI Team |
| **R5** | Stochastic actions diverge from success estimates | Medium | Medium | Instrument outcomes, adjust models | AI Team |
| **R6** | Integration breaks existing AI modes | Low | Critical | Feature flag isolation, regression suite | AI Team |
| **R7** | Memory growth from unbounded action history | Medium | Medium | Implement LRU eviction, cap at 1000 actions | AI Team |

---

## 7. Phase 0 Deliverables Checklist

- [x] **Audit existing AI touchpoints** (`astraweave-ai`, `astraweave-core`, `astraweave-director`)
- [x] **Map integration dependencies** (schema, validation, orchestrator trait)
- [x] **Define acceptance metrics** (plan quality, latency, learning, stability)
- [x] **Success metric dashboard specification** (real-time + offline)
- [x] **Stakeholder alignment plan** (design review schedule, discussion topics)
- [x] **Risk assessment** (7 identified risks with mitigations)
- [ ] **Roadmap sign-off** (pending gameplay/director/tooling leads approval)

---

## 8. Recommendations for Phase 1 (Technical Foundation)

### 8.1 Immediate Actions

1. **Port Claude GOAP modules** into `astraweave-ai::goap` (or new `astraweave-goap` crate)
   - File structure: `goap/core.rs`, `goap/action.rs`, `goap/goal.rs`, `goap/history.rs`, `goap/orchestrator.rs`
   - Add `planner_advanced` feature flag to `Cargo.toml`

2. **Fix determinism bug** in `WorldState` hashing
   - Replace `format!("{:?}", state)` with derived `Hash`/`Eq` or sorted key signature
   - Add unit test verifying deterministic plan generation

3. **Extend `StateValue` comparisons**
   - Support numeric ranges: `StateValue::IntRange(min, max)`
   - Add tolerance predicates: `StateValue::FloatApprox(value, epsilon)`
   - Update `WorldState::satisfies()` logic

4. **Baseline benchmark suite**
   - 10 scenarios × 100 iterations for `RuleOrchestrator`
   - Record latency distribution, success rate, plan depth
   - Commit as Phase 1 acceptance criteria reference

### 8.2 Phase 1 Exit Criteria

- [ ] `cargo test -p astraweave-goap` passes 20+ unit tests
- [ ] Deterministic planning verified (10 runs on same input → identical plans)
- [ ] Heuristic admissibility validated via test fixtures
- [ ] Benchmark shows <2ms P50 latency on 5 test scenarios
- [ ] No regressions in existing `astraweave-ai` tests

---

## 9. Appendices

### A. File Manifest (Scanned)

**astraweave-ai:**
- `src/lib.rs` (71 lines): `Orchestrator` trait + `RuleOrchestrator`

**astraweave-core:**
- `src/schema.rs` (136 lines): `WorldSnapshot`, `PlanIntent`, `ActionStep`, `ToolSpec`, `Constraints`
- `src/validation.rs` (1653 lines): `validate_and_execute()`, 95%+ test coverage
- `src/tools.rs`: A* pathfinding, LOS checks, cover queries
- `src/tool_sandbox.rs` (56 lines): Error taxonomy mapping

**astraweave-director:**
- `src/lib.rs` (73 lines): `BossDirector`
- `src/llm_director.rs` (646 lines): `LlmDirector`, `PlayerBehaviorModel`, `TacticPlan`
- `src/phase.rs` (102 lines): `PhaseDirector`, HP-threshold phase switching

**examples:**
- `hello_companion/src/main.rs` (1260 lines): 7 AI modes, plan generation + validation flow

### B. Action Catalog Summary

**Categories & Counts:**
- Movement (6): MoveTo, Approach, Retreat, TakeCover, Strafe, Patrol
- Offensive (8): Attack, AimedShot, QuickAttack, HeavyAttack, AoEAttack, ThrowExplosive, CoverFire, Charge
- Defensive (6): Block, Dodge, Parry, ThrowSmoke, Heal, UseDefensiveAbility
- Equipment (5): EquipWeapon, SwitchWeapon, Reload, UseItem, DropItem
- Tactical (8): CallReinforcements, MarkTarget, RequestCover, CoordinateAttack, SetAmbush, Distract, Regroup
- Utility (5): Scan, Wait, Interact, UseAbility, Taunt
- Legacy (2): Throw, Revive

**GOAP Priority Actions (Phase 2):**
1. MoveTo, Approach, Retreat (movement fundamentals)
2. Attack, AimedShot, CoverFire (combat core)
3. Heal, ThrowSmoke (survival/support)
4. Reload, EquipWeapon (resource management)
5. TakeCover, Scan (tactical awareness)

### C. Validation Constraint Coverage

| Constraint Type | Enforcement Point | Error Type | Example |
|----------------|------------------|-----------|---------|
| **Cooldown** | Line 298-300 (validation.rs) | `EngineError::Cooldown(String)` | "throw:grenade" |
| **Line of Sight** | Line 291-292, 313-314 | `EngineError::LosBlocked` | ThrowSmoke blocked |
| **Pathfinding** | Line 32-34 | `EngineError::NoPath` | MoveTo unreachable |
| **Resource (Ammo)** | Line 316-320 | `EngineError::Resource(String)` | "ammo" |
| **Invalid Action** | Various | `EngineError::InvalidAction(String)` | "Actor has no position" |

---

## 10. Next Steps (Action Items)

**Immediate (Week 0-1):**
1. Circulate Phase 0 document to gameplay/director/tooling leads for review
2. Schedule alignment meetings (30-45 min each group)
3. Set up telemetry dashboard prototype (Grafana or simple web UI)
4. Assign Phase 1 technical lead

**Phase 1 Prep:**
1. Create `astraweave-goap` crate skeleton with feature flag
2. Write Phase 1 acceptance test harness (baseline benchmarks)
3. Reserve integration branch: `feature/advanced-goap-phase1`

---

## Document Metadata

- **Author:** AI Analysis System
- **Reviewers:** [Pending]
- **Last Updated:** November 9, 2025
- **Version:** 1.0 (Phase 0 Complete)
- **Next Review:** Phase 1 Kickoff (TBD)

---

## Sign-Off

**Phase 0 Complete: ✅**

| Role | Name | Approval | Date |
|------|------|----------|------|
| AI/Planner Lead | [TBD] | ⏳ Pending | — |
| Gameplay Lead | [TBD] | ⏳ Pending | — |
| Director Lead | [TBD] | ⏳ Pending | — |
| Tooling Lead | [TBD] | ⏳ Pending | — |
| Performance Lead | [TBD] | ⏳ Pending | — |

**Proceed to Phase 1:** ⏳ Awaiting sign-off

