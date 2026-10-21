# Phase 5: Tooling & Designer Enablement - Implementation Plan

## Vision
Provide designers with comprehensive tools to author, validate, debug, and optimize GOAP goals and plans. Transform the GOAP system from a powerful backend into a complete designer-facing platform.

## Current State (Post-Phase 4)
✅ Hierarchical goal system with TOML authoring  
✅ Multi-goal scheduling and plan stitching  
✅ Designer guide with examples  
❌ No validation tooling for TOML files  
❌ No visualization of plans or hierarchies  
❌ No debugging or analysis tools  
❌ No performance profiling utilities  

## Goals & Deliverables

### 1. Goal Validation CLI Tool
**What**: Command-line validator for goal TOML files

**Features**:
- [ ] Schema validation (correct structure, required fields)
- [ ] Semantic validation (achievable goals, valid state variables)
- [ ] Circular dependency detection
- [ ] Performance estimates (hierarchy depth, complexity)
- [ ] Pretty error messages with line numbers
- [ ] Batch validation of entire directories

**Deliverable**: `cargo run --bin validate-goals -- path/to/goals/`

### 2. Plan Visualization Tools
**What**: Text-based and exportable visualizations of plans

**Features**:
- [ ] Plan tree ASCII art rendering
- [ ] Goal hierarchy visualization
- [ ] Action sequence timeline
- [ ] State transition diagram
- [ ] Export to DOT/GraphViz format
- [ ] HTML report generation

**Deliverable**: `PlanVisualizer` module + CLI tool

### 3. Analysis & Profiling Tools
**What**: Tools to measure and optimize plan quality

**Features**:
- [ ] Plan quality metrics (cost, risk, length)
- [ ] Action success rate reports
- [ ] Bottleneck identification
- [ ] Comparison tool (before/after optimization)
- [ ] Learning progress visualization
- [ ] Execution time profiling

**Deliverable**: `PlanAnalyzer` module + reports

### 4. Interactive Debugging Utilities
**What**: Runtime debugging aids for developers and designers

**Features**:
- [ ] Step-by-step plan execution simulator
- [ ] State diff viewer (before/after actions)
- [ ] Goal satisfaction checker
- [ ] "Why did planner choose X?" explainer
- [ ] Conflict analyzer with suggestions
- [ ] Replan trigger detector

**Deliverable**: `DebugTools` module

### 5. Goal Templates & Generator
**What**: Tools to quickly create common goal patterns

**Features**:
- [ ] Goal template library (20+ patterns)
- [ ] Interactive goal generator CLI
- [ ] Pattern matching for common scenarios
- [ ] Template composition tools
- [ ] Preset configurations (easy/medium/hard)

**Deliverable**: Expanded template library + generator

### 6. Documentation & Tutorials
**What**: Enhanced docs with interactive examples

**Features**:
- [ ] Workflow tutorials (5+ step-by-step guides)
- [ ] Video-style ASCII animations
- [ ] Troubleshooting decision tree
- [ ] Performance optimization guide
- [ ] Case studies from example templates
- [ ] FAQ with common issues

**Deliverable**: Enhanced documentation suite

## Implementation Order

### Week 1: Validation & Analysis Foundation
1. Create `goal_validator` module with schema validation
2. Implement semantic validation (circular deps, achievability)
3. Build `plan_analyzer` module with quality metrics
4. Add comprehensive error reporting
5. **Checkpoint**: Can validate all example goals and get quality reports

### Week 2: Visualization Tools
1. Create `plan_visualizer` module with ASCII rendering
2. Implement goal hierarchy visualization
3. Add DOT/GraphViz export
4. Build HTML report generator
5. **Checkpoint**: Can visualize example plans in multiple formats

### Week 3: Debugging & Profiling
1. Create `debug_tools` module with step simulator
2. Implement state diff viewer
3. Add "why this action?" explainer
4. Build conflict analyzer
5. **Checkpoint**: Can debug plan execution issues

### Week 4: Templates & Polish
1. Expand template library (20+ templates)
2. Create interactive goal generator
3. Write workflow tutorials
4. Performance optimization guide
5. **Checkpoint**: Complete designer workflow demonstrated

## Technical Design

### Goal Validator Architecture

```rust
pub struct GoalValidator {
    schema_rules: Vec<Box<dyn ValidationRule>>,
    semantic_rules: Vec<Box<dyn SemanticRule>>,
}

pub trait ValidationRule {
    fn validate(&self, goal: &GoalDefinition) -> Result<(), ValidationError>;
}

pub enum ValidationError {
    SchemaError { field: String, message: String, line: usize },
    SemanticError { message: String, suggestion: Option<String> },
    Warning { message: String },
}
```

### Plan Visualizer Architecture

```rust
pub struct PlanVisualizer {
    format: VisualizationFormat,
}

pub enum VisualizationFormat {
    AsciiTree,
    AsciiTimeline,
    Dot,
    Html,
    Json,
}

impl PlanVisualizer {
    pub fn visualize_plan(&self, plan: &[String], actions: &[Box<dyn Action>]) -> String;
    pub fn visualize_hierarchy(&self, goal: &Goal) -> String;
    pub fn visualize_execution(&self, tracker: &PlanExecutionTracker) -> String;
}
```

### Plan Analyzer Architecture

```rust
pub struct PlanAnalyzer;

pub struct PlanMetrics {
    pub total_cost: f32,
    pub total_risk: f32,
    pub action_count: usize,
    pub estimated_duration: f32,
    pub bottlenecks: Vec<String>,
    pub success_probability: f32,
}

impl PlanAnalyzer {
    pub fn analyze(&self, plan: &[String], actions: &[Box<dyn Action>], history: &ActionHistory) -> PlanMetrics;
    pub fn compare(&self, plan1: &PlanMetrics, plan2: &PlanMetrics) -> ComparisonReport;
    pub fn suggest_optimizations(&self, metrics: &PlanMetrics) -> Vec<Suggestion>;
}
```

### Debug Tools Architecture

```rust
pub struct PlanDebugger {
    current_step: usize,
    state_history: Vec<WorldState>,
}

impl PlanDebugger {
    pub fn new(plan: Vec<String>, start_state: WorldState) -> Self;
    pub fn step_forward(&mut self) -> Result<(), String>;
    pub fn step_backward(&mut self);
    pub fn get_state_diff(&self) -> StateDiff;
    pub fn explain_action(&self, action_name: &str) -> Explanation;
    pub fn check_goal_progress(&self, goal: &Goal) -> ProgressReport;
}
```

## Validation Rules

### Schema Validation Rules
1. **RequiredFieldsRule**: Ensures name, desired_state present
2. **TypeValidationRule**: Checks field types match schema
3. **RangeValidationRule**: Validates numeric ranges
4. **DecompositionStrategyRule**: Checks valid strategy values
5. **DepthLimitRule**: Ensures max_depth is reasonable

### Semantic Validation Rules
1. **CircularDependencyRule**: Detects sub-goal loops
2. **AchievabilityRule**: Checks if goal is theoretically achievable
3. **StateVariableRule**: Warns about unknown state variables
4. **ConflictingGoalsRule**: Detects contradictory sub-goals
5. **ComplexityRule**: Warns about excessive complexity

## Visualization Examples

### ASCII Plan Tree
```
Plan for "escort_player" (3 actions, cost: 5.0, risk: 0.2)
├─ scan (cost: 1.0, risk: 0.05)
├─ move_to (cost: 2.0, risk: 0.1)
└─ attack (cost: 2.0, risk: 0.05)
```

### Goal Hierarchy
```
[SEQUENTIAL] escort_player (priority: 10.0, deadline: 300s)
  │
  ├─ [SEQUENTIAL] clear_path (priority: 9.0)
  │  ├─ scout_route (priority: 8.0)
  │  └─ eliminate_threats (priority: 9.0)
  │
  ├─ stay_close_to_player (priority: 8.0)
  │
  └─ reach_extraction (priority: 10.0)
```

### Execution Timeline
```
Time | Action        | State Changes              | Success
-----|---------------|----------------------------|--------
0.0  | scan          | area_scanned=true          | ✓
1.0  | move_to       | at_location=true, x=10     | ✓
3.0  | attack        | enemy_defeated=true        | ✓
```

## CLI Tools

### 1. validate-goals
```bash
# Validate single goal
cargo run --bin validate-goals -- escort_mission.toml

# Validate directory
cargo run --bin validate-goals -- examples/goal_templates/

# Strict mode (warnings as errors)
cargo run --bin validate-goals -- --strict goals/

# Output format
cargo run --bin validate-goals -- --format json goals/
```

### 2. visualize-plan
```bash
# Visualize plan from goal
cargo run --bin visualize-plan -- --goal escort.toml --state world.json

# Output formats
cargo run --bin visualize-plan -- --format ascii goals/escort.toml
cargo run --bin visualize-plan -- --format dot goals/escort.toml > plan.dot
cargo run --bin visualize-plan -- --format html goals/escort.toml > plan.html
```

### 3. analyze-plan
```bash
# Analyze plan quality
cargo run --bin analyze-plan -- --goal escort.toml --history history.json

# Compare two plans
cargo run --bin analyze-plan -- --compare before.toml after.toml

# Generate report
cargo run --bin analyze-plan -- --report goals/ > report.html
```

### 4. debug-plan
```bash
# Interactive debugger
cargo run --bin debug-plan -- --goal escort.toml --state world.json

# Commands in debugger:
# > step        - Execute next action
# > back        - Undo last action
# > state       - Show current state
# > diff        - Show state changes
# > explain     - Explain why action was chosen
# > goal        - Check goal progress
# > quit        - Exit
```

### 5. generate-goal
```bash
# Interactive generator
cargo run --bin generate-goal

# From template
cargo run --bin generate-goal -- --template escort --output my_escort.toml

# List templates
cargo run --bin generate-goal -- --list
```

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Validation Speed** | <100ms for typical goal | CLI timing |
| **Error Detection Rate** | ≥95% of common errors | Manual testing |
| **Visualization Quality** | Readable ASCII for 50+ action plans | Visual inspection |
| **Tool Adoption** | ≥3 tools used per designer session | User feedback |
| **Time to Fix Error** | <5min with tool vs >15min without | User study |
| **Template Coverage** | 20+ common scenarios | Template count |

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Tools too complex** | Low adoption | User testing, simplify UX |
| **Validation too strict** | False positives | Configurable severity levels |
| **Performance issues** | Slow validation | Optimize hot paths, caching |
| **Visualization unreadable** | Confusing output | Multiple formats, user feedback |
| **Tool maintenance burden** | Tech debt | Good abstractions, tests |

## Dependencies
- Phase 4 goal authoring system
- `toml` for parsing (already present)
- `serde` for serialization
- `colored` for terminal output (optional)
- `graphviz` bindings for DOT export (optional)

## Documentation Deliverables
1. **`docs/tooling_user_guide.md`** - How to use each tool
2. **`docs/workflow_tutorials.md`** - Step-by-step workflows
3. **`docs/validation_rules.md`** - What each rule checks
4. **`docs/optimization_guide.md`** - Performance tuning
5. **`docs/PHASE5_COMPLETE.md`** - Completion summary

## Testing Strategy
1. Unit tests for each validation rule (≥20 tests)
2. Integration tests for CLI tools (≥10 tests)
3. Visual regression tests for visualizations
4. Performance tests for large goals/plans
5. User acceptance testing with designers

## Next Steps (Immediate)
1. ✅ Create this plan document
2. 🔄 Create `goal_validator` module with basic schema validation
3. 🔄 Implement validation rules
4. 🔄 Create `validate-goals` CLI tool
5. 🔄 Add comprehensive error messages
6. 🔄 Write tests for validator

---

**Phase 5 Kick-off**: Ready to begin implementation! 🛠️

**Estimated Completion**: 4 weeks (accelerated to hours in this session)

**Feature Flag**: `planner_advanced` (already active)

