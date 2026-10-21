# Phase 4: Hierarchical & Multi-Goal Expansion - Implementation Plan

## Vision
Enable GOAP to handle complex, multi-layered objectives by supporting:
1. **Hierarchical Goals (HTN-style)**: Break down high-level goals into sub-goals recursively
2. **Multi-Goal Scheduling**: Handle concurrent goals with dynamic priorities and deadlines
3. **Plan Stitching**: Combine sub-plans into coherent action sequences
4. **Designer Authoring**: TOML/RON templates for defining goal hierarchies

## Current State (Post-Phase 3)
✅ Basic GOAP planner with A* search  
✅ Learning and persistence system  
✅ Single-goal planning with priority  
❌ Sub-goals are defined in `Goal` struct but unused  
❌ Multi-goal planning exists but doesn't respect deadlines or hierarchies  
❌ No plan stitching or recursive decomposition  
❌ No designer-facing goal templates  

## Goals & Deliverables

### 1. Enable Hierarchical Goal Decomposition
**What**: Implement HTN-style recursive planning where high-level goals decompose into sub-goals

**Implementation Steps**:
- [ ] Add `decompose_goal()` method to identify when a goal should be broken down
- [ ] Implement recursive planner that attempts sub-goals before direct planning
- [ ] Add plan stitching to combine sub-plans into a coherent sequence
- [ ] Handle sub-goal failure and fallback strategies
- [ ] Add depth limits to prevent infinite recursion

**Acceptance Criteria**:
- Can plan for "Secure Area" → ["Clear Enemies", "Establish Perimeter"]
- Sub-goal failures trigger re-planning or parent goal failure
- Maximum recursion depth enforced (e.g., 5 levels)

### 2. Enhanced Multi-Goal Scheduling
**What**: Improve existing multi-goal planning with deadline awareness and priority-based interleaving

**Implementation Steps**:
- [ ] Enhance `Goal` with urgency calculation based on deadline proximity
- [ ] Implement goal scheduler that orders goals by urgency × priority
- [ ] Add plan interleaving for time-critical goals
- [ ] Support goal preemption (abandon current plan for urgent goal)
- [ ] Add goal satisfaction checking and auto-removal

**Acceptance Criteria**:
- Urgent low-priority goals can preempt non-urgent high-priority goals
- Goals with missed deadlines are deprioritized or cancelled
- Can handle 5+ concurrent goals without performance degradation

### 3. Plan Stitching & Validation
**What**: Combine multiple sub-plans into a single executable plan with conflict detection

**Implementation Steps**:
- [ ] Implement plan validator that checks for state conflicts
- [ ] Add plan merging logic (sequential, interleaved, parallel)
- [ ] Detect and resolve resource conflicts (e.g., can't heal and attack simultaneously)
- [ ] Add plan optimization pass to remove redundant actions
- [ ] Support partial plan execution and resume points

**Acceptance Criteria**:
- Conflicting actions (e.g., "MoveTo(A)" then "MoveTo(B)") are detected
- Sub-plans are merged into efficient sequences
- Can identify safe resume points after plan interruption

### 4. Goal Authoring Templates
**What**: Designer-friendly TOML/RON schemas for defining goal hierarchies

**Implementation Steps**:
- [ ] Design goal definition schema (TOML format)
- [ ] Implement goal loader/parser with validation
- [ ] Create example templates (escort, defend, assault, patrol)
- [ ] Add goal library for common tactical patterns
- [ ] Document authoring workflow

**Acceptance Criteria**:
- Designers can define goals in TOML without code changes
- Goal loader validates structure and reports errors
- 5+ example templates covering common scenarios

### 5. Comprehensive Testing
**What**: Validate hierarchical planning, multi-goal scheduling, and plan stitching

**Implementation Steps**:
- [ ] Unit tests for goal decomposition logic
- [ ] Integration tests for hierarchical planning scenarios
- [ ] Multi-goal scheduling tests with deadlines
- [ ] Plan stitching tests with conflict detection
- [ ] End-to-end scenario tests (protect + pursue)

**Acceptance Criteria**:
- All new tests passing
- Code coverage ≥80% for new modules
- Performance: hierarchical planning ≤10ms for 3-level hierarchies

## Technical Design

### Goal Hierarchy Structure
```rust
pub struct Goal {
    pub desired_state: WorldState,
    pub priority: f32,
    pub deadline: Option<f32>,
    pub sub_goals: Vec<Goal>,  // ← Currently unused, will enable
    pub decomposition_strategy: DecompositionStrategy,
}

pub enum DecompositionStrategy {
    Sequential,      // Sub-goals must be achieved in order
    Parallel,        // Sub-goals can be pursued simultaneously
    AnyOf,           // Any one sub-goal satisfies parent
    AllOf,           // All sub-goals must be satisfied
}
```

### Hierarchical Planner Flow
```
1. Check if goal is satisfied → Done
2. Check if goal has sub-goals:
   a. YES → Decompose and recursively plan for each sub-goal
            Stitch sub-plans together based on strategy
   b. NO  → Use standard A* planning
3. If sub-goal planning fails → Try direct planning
4. If all fail → Return failure
```

### Multi-Goal Scheduler
```rust
pub struct GoalScheduler {
    active_goals: Vec<Goal>,
    current_plan: Option<Vec<String>>,
    last_replan_time: f32,
}

impl GoalScheduler {
    pub fn add_goal(&mut self, goal: Goal);
    pub fn update(&mut self, current_time: f32, world: &WorldState) -> Option<Vec<String>>;
    fn calculate_urgency(&self, goal: &Goal, current_time: f32) -> f32;
    fn should_replan(&self, world: &WorldState) -> bool;
}
```

### Plan Stitcher
```rust
pub struct PlanStitcher;

impl PlanStitcher {
    pub fn merge_sequential(plans: Vec<Vec<String>>) -> Result<Vec<String>, StitchError>;
    pub fn merge_interleaved(plans: Vec<Vec<String>>, priorities: Vec<f32>) -> Result<Vec<String>, StitchError>;
    pub fn detect_conflicts(plan: &[String], state: &WorldState) -> Vec<Conflict>;
    pub fn optimize(plan: Vec<String>) -> Vec<String>;
}
```

### Goal Definition Schema (TOML)
```toml
[goal.secure_area]
name = "Secure Area"
priority = 8.0
deadline_seconds = 60.0
decomposition = "sequential"

[goal.secure_area.desired_state]
enemies_in_area = 0
perimeter_established = true

[[goal.secure_area.sub_goals]]
name = "Clear Enemies"
priority = 9.0
[goal.secure_area.sub_goals.desired_state]
enemies_in_area = 0

[[goal.secure_area.sub_goals]]
name = "Establish Perimeter"
priority = 7.0
[goal.secure_area.sub_goals.desired_state]
perimeter_established = true
```

## Implementation Order

### Week 1: Goal Decomposition Core
1. Add `DecompositionStrategy` enum to `goal.rs`
2. Implement `Goal::should_decompose()` and `Goal::decompose()`
3. Add recursive planning to `AdvancedGOAP`
4. Unit tests for decomposition logic
5. **Checkpoint**: Can decompose 2-level goal hierarchy

### Week 2: Plan Stitching
1. Create `plan_stitcher.rs` module
2. Implement sequential plan merging
3. Add conflict detection
4. Implement plan optimization
5. **Checkpoint**: Can merge 3 sub-plans into coherent sequence

### Week 3: Multi-Goal Scheduling
1. Create `goal_scheduler.rs` module
2. Implement urgency calculation with deadlines
3. Add goal preemption logic
4. Implement goal satisfaction checking
5. **Checkpoint**: Can schedule 5+ concurrent goals

### Week 4: Authoring & Testing
1. Design TOML goal schema
2. Implement goal loader in `goal_authoring.rs`
3. Create 5+ example goal templates
4. Comprehensive integration tests
5. **Checkpoint**: Designer can author and load custom goals

## Test Scenarios

### Scenario 1: Hierarchical Escort Mission
```
Goal: "Escort Player to Extraction"
├─ Sub-Goal: "Clear Path" (sequential)
│  ├─ "Eliminate Threats Ahead"
│  └─ "Mark Safe Route"
└─ Sub-Goal: "Stay Close to Player"
   └─ "Maintain Distance ≤ 5 units"
```

**Expected Behavior**:
- Agent clears enemies sequentially along path
- Periodically moves to stay near player
- If player deviates, re-plans path clearing

### Scenario 2: Concurrent Protect + Pursue
```
Goal A: "Protect Player" (priority 9, no deadline)
Goal B: "Pursue Enemy Leader" (priority 7, deadline 30s)
```

**Expected Behavior**:
- Initially pursues leader (deadline urgency)
- If player endangered, switches to protect
- If deadline near-miss and player safe, resumes pursuit

### Scenario 3: Nested Tactical Goals
```
Goal: "Assault Position"
├─ "Suppress Enemy" (parallel)
├─ "Flank Route"
│  ├─ "Find Cover Path"
│  └─ "Move to Flank"
└─ "Execute Attack"
```

**Expected Behavior**:
- Plans suppressing fire while flanking
- Finds safe flanking route
- Coordinates final assault

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Decomposition Depth** | ≤5 levels | Max recursion in tests |
| **Planning Time (3-level)** | ≤10ms | Benchmark hierarchical plan |
| **Goal Scheduling Overhead** | ≤2ms for 10 goals | Scheduler update time |
| **Plan Stitching Success** | ≥90% | % of stitches without conflicts |
| **Designer Authoring** | <30min to create complex goal | User testing |
| **Test Coverage** | ≥80% | Cargo tarpaulin |

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Exponential plan explosion** | High planning time | Limit decomposition depth, add pruning |
| **Conflicting sub-goals** | Invalid plans | Comprehensive conflict detection |
| **Designer complexity** | Low adoption | Extensive examples, validation errors |
| **Goal oscillation** | Thrashing between goals | Add hysteresis to urgency calculation |
| **Plan stitching bugs** | Execution failures | Extensive integration tests |

## Dependencies
- Phase 3 learning system (for sub-goal cost estimation)
- `goal.rs` enhancements (decomposition strategies)
- New modules: `plan_stitcher.rs`, `goal_scheduler.rs`, `goal_authoring.rs`
- TOML parsing library (already in dependencies)

## Documentation Deliverables
1. **`docs/hierarchical_goals_guide.md`** - Designer guide to goal hierarchies
2. **`examples/goal_templates/`** - 5+ TOML example templates
3. **`docs/PHASE4_COMPLETE.md`** - Completion summary
4. Inline code documentation for new APIs

## Validation Plan
1. Unit tests for each new module (≥20 tests)
2. Integration tests for hierarchical scenarios (≥5 scenarios)
3. Performance benchmarks (decomposition, stitching, scheduling)
4. Designer usability testing with TOML templates

## Next Steps (Immediate)
1. ✅ Create this plan document
2. 🔄 Add `DecompositionStrategy` enum to `goal.rs`
3. 🔄 Implement basic `Goal::should_decompose()` logic
4. 🔄 Add recursive planning scaffold to `planner.rs`
5. 🔄 Create `plan_stitcher.rs` with sequential merge
6. 🔄 Write first hierarchical planning test

---

**Phase 4 Kick-off**: Ready to begin implementation! 🚀

**Estimated Completion**: 4 weeks (accelerated to hours in this session)

**Feature Flag**: `planner_advanced` (already active)

