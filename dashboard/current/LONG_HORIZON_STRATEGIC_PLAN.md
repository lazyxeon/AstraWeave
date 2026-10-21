# AstraWeave: Long-Horizon Strategic Implementation Plan (12 Months)

**Version**: 1.0  
**Timeline**: Months 1-12  
**Goal**: Transform from "compiles cleanly" to "production-ready AI-native game engine"  
**Owner**: Core Team + Community  
**Status**: 🟢 Ready to Execute

---

## Executive Summary

This plan outlines a **12-month transformation** of AstraWeave, organized into 3 strategic phases:

1. **Phase A: Foundation Hardening** (Months 1-3) - Eliminate critical blockers
2. **Phase B: Performance & Scale** (Months 4-6) - Achieve production performance targets
3. **Phase C: Production Polish** (Months 7-12) - Ship-quality engine with comprehensive tooling

### Key Principles

- **Foundation First**: Robustness before optimization, correctness before features
- **Measurable Progress**: Every phase has quantitative acceptance criteria
- **Incremental Delivery**: Ship improvements weekly, don't wait for perfection
- **Community Engagement**: Document wins publicly, celebrate milestones
- **Risk Management**: Multiple fallback strategies, never block on single approach

### Success Metrics (End of Month 12)

| Metric | Current | Target | Priority |
|--------|---------|--------|----------|
| `.unwrap()` in Core | 50+ | 0 | 🔴 Critical |
| Test Coverage (Core) | ~30% | 70%+ | 🔴 Critical |
| ECS Throughput | Unknown | 500+ entities @ 60fps | 🟠 High |
| LLM Quality Score | Unknown | 95%+ valid plans | 🟠 High |
| Integration Tests | 0 | 20+ | 🟠 High |
| Frame Time (p95) | Unknown | <16.67ms | 🟡 Medium |
| Production Uptime | Unknown | 24+ hours | 🟡 Medium |

---

## Phase A: Foundation Hardening (Months 1-3)

**Goal**: Eliminate critical blockers preventing production deployment  
**Focus**: Robustness, correctness, testing infrastructure  
**Theme**: "Make it work reliably before making it fast"

### Month 1: Critical Blockers Resolution

#### Week 1: Immediate Actions (See IMMEDIATE_ACTIONS_IMPLEMENTATION_PLAN.md)
- ✅ GPU skinning pipeline descriptor
- ✅ Combat physics attack sweep
- ✅ `.unwrap()` audit complete
- ✅ Performance baselines documented

#### Week 2-3: Core Error Handling (astraweave-ecs, astraweave-core)

**Deliverable**: Zero `.unwrap()` in core ECS and world simulation

**Implementation Steps**:

1. **Define Error Types** (Days 1-2):
```rust
// astraweave-ecs/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum EcsError {
    #[error("Entity {0:?} not found")]
    EntityNotFound(Entity),
    
    #[error("Component {0} not registered")]
    ComponentNotRegistered(String),
    
    #[error("System {0} failed: {1}")]
    SystemExecutionFailed(String, Box<dyn std::error::Error>),
    
    #[error("Resource {0} not found")]
    ResourceNotFound(String),
    
    #[error("Archetype storage full (max: {0})")]
    ArchetypeStorageFull(usize),
}

pub type EcsResult<T> = Result<T, EcsError>;
```

2. **Replace `.unwrap()` in Query System** (Days 3-7):
```rust
// Before
let component = self.components.get(&entity).unwrap();

// After
let component = self.components.get(&entity)
    .ok_or(EcsError::EntityNotFound(entity))?;
```

**Target Files**:
- `astraweave-ecs/src/query.rs` (8 instances)
- `astraweave-ecs/src/archetype.rs` (6 instances)
- `astraweave-ecs/src/system_param.rs` (4 instances)
- `astraweave-ecs/src/events.rs` (2 instances)

3. **Add Error Handling Tests** (Days 8-10):
```rust
#[test]
fn test_query_missing_entity_returns_error() {
    let world = World::new();
    let invalid_entity = Entity::from_bits(999);
    
    let result: EcsResult<&Position> = world.get_component(invalid_entity);
    
    assert!(result.is_err());
    match result {
        Err(EcsError::EntityNotFound(e)) => assert_eq!(e, invalid_entity),
        _ => panic!("Expected EntityNotFound error"),
    }
}
```

**Acceptance Criteria**:
- [ ] Zero `.unwrap()` in `astraweave-ecs/src/` (excluding tests)
- [ ] All public APIs return `EcsResult<T>` where appropriate
- [ ] 10+ error handling tests added
- [ ] Existing tests still pass (no regressions)
- [ ] Documentation updated with error handling examples

#### Week 4: LLM Error Handling & Evaluation Harness

**Deliverable 1**: Robust error handling in LLM integration

**Implementation**:
```rust
// astraweave-llm/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("LLM request timeout after {0:?}")]
    Timeout(Duration),
    
    #[error("Invalid JSON response: {0}")]
    InvalidJson(String),
    
    #[error("Disallowed tool: {0}")]
    DisallowedTool(String),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("Plan validation failed: {0}")]
    ValidationFailed(String),
}

pub type LlmResult<T> = Result<T, LlmError>;
```

**Target Files**:
- `astraweave-llm/src/client.rs` (5 instances)
- `astraweave-llm/src/parse.rs` (4 instances)
- `astraweave-llm/src/planning.rs` (4 instances)

**Deliverable 2**: LLM evaluation harness

**File**: `astraweave-llm-eval/src/lib.rs`

**Implementation**:
```rust
pub struct EvaluationHarness {
    test_scenarios: Vec<TestScenario>,
    scoring_fn: Box<dyn Fn(&PlanIntent) -> f32>,
}

pub struct TestScenario {
    pub name: String,
    pub world_snapshot: WorldSnapshot,
    pub expected_plan: ExpectedPlan,
    pub tool_registry: ToolRegistry,
}

pub struct EvaluationResult {
    pub scenario_name: String,
    pub plan_valid: bool,
    pub goal_achieved: bool,
    pub safety_score: f32,  // 0.0-1.0
    pub coherence_score: f32, // 0.0-1.0
    pub overall_score: f32,   // 0.0-100.0
}

impl EvaluationHarness {
    pub async fn run_evaluation(&self, client: &dyn LlmClient) -> Vec<EvaluationResult> {
        let mut results = Vec::new();
        
        for scenario in &self.test_scenarios {
            let plan = plan_from_llm(client, &scenario.world_snapshot, &scenario.tool_registry).await;
            
            let result = EvaluationResult {
                scenario_name: scenario.name.clone(),
                plan_valid: self.validate_plan(&plan, &scenario.tool_registry),
                goal_achieved: self.check_goal(&plan, &scenario.expected_plan),
                safety_score: self.score_safety(&plan),
                coherence_score: self.score_coherence(&plan),
                overall_score: self.calculate_overall_score(&plan, scenario),
            };
            
            results.push(result);
        }
        
        results
    }
    
    fn validate_plan(&self, plan: &PlanIntent, registry: &ToolRegistry) -> bool {
        // Check all tools are in allowlist
        plan.steps.iter().all(|step| {
            registry.tools.iter().any(|tool| tool.name == step.tool_name())
        })
    }
    
    fn check_goal(&self, plan: &PlanIntent, expected: &ExpectedPlan) -> bool {
        // Check if plan achieves expected goal
        // (Implementation depends on goal representation)
        true // Simplified
    }
    
    fn score_safety(&self, plan: &PlanIntent) -> f32 {
        // Penalize dangerous actions (e.g., attack allies, move into hazards)
        let mut score = 1.0;
        
        for step in &plan.steps {
            if self.is_dangerous(step) {
                score -= 0.2;
            }
        }
        
        score.max(0.0)
    }
    
    fn score_coherence(&self, plan: &PlanIntent) -> f32 {
        // Check logical consistency (e.g., MoveTo before Interact)
        let mut score = 1.0;
        
        // Simplified: Check if actions are in sensible order
        // (Full implementation would use dependency graph)
        
        score
    }
    
    fn calculate_overall_score(&self, plan: &PlanIntent, scenario: &TestScenario) -> f32 {
        let valid = self.validate_plan(plan, &scenario.tool_registry) as u8 as f32;
        let goal = self.check_goal(plan, &scenario.expected_plan) as u8 as f32;
        let safety = self.score_safety(plan);
        let coherence = self.score_coherence(plan);
        
        // Weighted average
        (valid * 40.0 + goal * 30.0 + safety * 15.0 + coherence * 15.0)
    }
}
```

**Test Scenarios** (20+ scenarios):
```rust
// Combat scenario
TestScenario {
    name: "combat_engage_enemy".to_string(),
    world_snapshot: WorldSnapshot {
        player: PlayerState { pos: IVec2 { x: 0, y: 0 }, health: 100 },
        companions: vec![],
        enemies: vec![EnemyState { pos: IVec2 { x: 5, y: 0 }, health: 50 }],
        obstacles: vec![],
    },
    expected_plan: ExpectedPlan {
        primary_goal: Goal::Eliminate(Enemy(0)),
        required_actions: vec!["MoveTo", "Attack"],
        forbidden_actions: vec!["Flee"],
    },
    tool_registry: create_combat_registry(),
},

// Exploration scenario
TestScenario {
    name: "explore_unknown_area".to_string(),
    world_snapshot: WorldSnapshot {
        player: PlayerState { pos: IVec2 { x: 0, y: 0 }, health: 100 },
        companions: vec![CompanionState { pos: IVec2 { x: 1, y: 0 } }],
        enemies: vec![],
        obstacles: vec![],
        pois: vec![Poi { pos: IVec2 { x: 10, y: 10 }, kind: "treasure" }],
    },
    expected_plan: ExpectedPlan {
        primary_goal: Goal::Investigate(Poi(0)),
        required_actions: vec!["MoveTo"],
        forbidden_actions: vec!["Attack"],
    },
    tool_registry: create_exploration_registry(),
},

// Stealth scenario
// Support scenario
// Puzzle scenario
// ... (18 more scenarios)
```

**Acceptance Criteria**:
- [ ] Evaluation harness runs 20+ scenarios in <30s
- [ ] Quality baseline established (e.g., "MockLlm: 98% valid, 85% goal achieved")
- [ ] CI integration (runs on every PR to `astraweave-llm`)
- [ ] Quality report artifact generated (markdown table with scores)

---

### Month 2: Testing Infrastructure

#### Week 5-6: Skeletal Animation Integration Tests (4/4)

**Deliverable**: Complete animation pipeline validation

**Test 1: CPU vs GPU Parity**
```rust
#[test]
fn test_cpu_gpu_skinning_parity() {
    let skeleton = create_test_skeleton(10); // 10 bones
    let mesh = create_test_mesh(500); // 500 vertices
    let animation = create_test_animation();
    
    // CPU skinning
    let cpu_output = cpu_skin_vertices(&mesh, &skeleton, &animation);
    
    // GPU skinning
    let (device, queue) = create_test_device();
    let gpu_output = gpu_skin_vertices(&device, &queue, &mesh, &skeleton, &animation);
    
    // Compare outputs (with epsilon tolerance for float precision)
    for (cpu_vert, gpu_vert) in cpu_output.iter().zip(gpu_output.iter()) {
        assert_vec3_near(cpu_vert.position, gpu_vert.position, 0.001);
        assert_vec3_near(cpu_vert.normal, gpu_vert.normal, 0.001);
    }
}
```

**Test 2: Animation Determinism**
```rust
#[test]
fn test_animation_determinism() {
    let skeleton = create_test_skeleton(10);
    let mesh = create_test_mesh(500);
    let animation = create_test_animation();
    
    // Run animation 100 times
    let mut outputs = Vec::new();
    for _ in 0..100 {
        let output = animate_skeleton(&skeleton, &animation, 1.0 /* time */);
        outputs.push(output);
    }
    
    // All outputs should be identical
    let first = &outputs[0];
    for output in &outputs[1..] {
        assert_eq!(first, output, "Animation is non-deterministic!");
    }
}
```

**Test 3: Scene Graph Integration**
```rust
#[test]
fn test_scene_graph_hierarchy() {
    // Create hierarchy: Root -> Spine -> Arm -> Hand
    let mut scene = SceneGraph::new();
    let root = scene.add_node("Root", Transform::identity());
    let spine = scene.add_node("Spine", Transform::translation(0.0, 1.0, 0.0));
    let arm = scene.add_node("Arm", Transform::translation(0.5, 0.0, 0.0));
    let hand = scene.add_node("Hand", Transform::translation(0.3, 0.0, 0.0));
    
    scene.set_parent(spine, root);
    scene.set_parent(arm, spine);
    scene.set_parent(hand, arm);
    
    // Rotate spine by 90°
    scene.set_local_rotation(spine, Quat::from_rotation_y(PI / 2.0));
    
    // Compute world transforms
    scene.update_world_transforms();
    
    // Hand's world position should reflect spine rotation
    let hand_world = scene.get_world_transform(hand);
    assert_vec3_near(hand_world.translation, Vec3::new(-0.3, 1.0, 0.5), 0.001);
}
```

**Test 4: Performance Validation**
```rust
#[test]
fn test_gpu_skinning_performance() {
    let skeleton = create_test_skeleton(100); // 100 bones (realistic)
    let mesh = create_test_mesh(10_000); // 10K vertices
    let animation = create_test_animation();
    
    let (device, queue) = create_test_device();
    
    // Benchmark GPU skinning
    let start = Instant::now();
    for _ in 0..100 {
        gpu_skin_vertices(&device, &queue, &mesh, &skeleton, &animation);
    }
    let gpu_time = start.elapsed() / 100;
    
    // Benchmark CPU skinning
    let start = Instant::now();
    for _ in 0..100 {
        cpu_skin_vertices(&mesh, &skeleton, &animation);
    }
    let cpu_time = start.elapsed() / 100;
    
    // GPU should be >5x faster for 10K vertices
    assert!(cpu_time > gpu_time * 5, 
            "GPU skinning not significantly faster: CPU={:?}, GPU={:?}", 
            cpu_time, gpu_time);
}
```

**Acceptance Criteria**:
- [ ] 4/4 tests implemented and passing
- [ ] Tests run in CI on every PR
- [ ] Performance test documents GPU advantage (>5x speedup)
- [ ] Documentation updated with test results

#### Week 7-8: Cross-System Integration Tests (10+ tests)

**Test Infrastructure**:
```rust
// astraweave-integration-tests/src/lib.rs
pub struct IntegrationTestHarness {
    world: World,
    renderer: Renderer,
    physics: PhysicsWorld,
    ai_service: AIService,
}

impl IntegrationTestHarness {
    pub fn new() -> Self {
        // Create headless renderer for CI
        let (device, queue) = create_headless_device();
        let renderer = Renderer::new(&device, &queue, 800, 600);
        
        Self {
            world: World::new(),
            renderer,
            physics: PhysicsWorld::new(),
            ai_service: AIService::with_mock_llm(),
        }
    }
    
    pub fn spawn_ai_agent(&mut self, position: Vec3) -> Entity {
        let entity = self.world.spawn_empty();
        self.world.insert(entity, Transform::from_translation(position));
        self.world.insert(entity, AIAgent::new());
        self.world.insert(entity, RigidBody::dynamic());
        entity
    }
    
    pub fn run_frame(&mut self) -> anyhow::Result<()> {
        // Full frame: ECS tick + physics step + render
        self.world.tick(1.0 / 60.0)?;
        self.physics.step(1.0 / 60.0)?;
        self.renderer.render(&self.world, &self.physics)?;
        Ok(())
    }
}
```

**Integration Test 1: AI + Physics + Navigation**
```rust
#[test]
fn test_ai_plans_valid_path_with_physics() -> anyhow::Result<()> {
    let mut harness = IntegrationTestHarness::new();
    
    // Spawn agent at (0, 0, 0)
    let agent = harness.spawn_ai_agent(Vec3::ZERO);
    
    // Create obstacle at (2, 0, 0)
    harness.spawn_obstacle(Vec3::new(2.0, 0.0, 0.0));
    
    // Set goal: Move to (5, 0, 0) - requires pathfinding around obstacle
    harness.set_goal(agent, Goal::MoveTo(Vec3::new(5.0, 0.0, 0.0)));
    
    // Run simulation for 5 seconds
    for _ in 0..300 { // 300 frames @ 60Hz = 5s
        harness.run_frame()?;
    }
    
    // Agent should reach goal
    let final_pos = harness.get_position(agent)?;
    assert!(final_pos.distance(Vec3::new(5.0, 0.0, 0.0)) < 0.5, 
            "Agent failed to reach goal: {:?}", final_pos);
    
    // Agent should not have collided with obstacle
    let collision_count = harness.get_collision_count(agent)?;
    assert_eq!(collision_count, 0, "Agent collided with obstacle");
    
    Ok(())
}
```

**Integration Test 2: Rendering + Skeletal Animation + Materials**
```rust
#[test]
fn test_animated_character_with_hot_reload() -> anyhow::Result<()> {
    let mut harness = IntegrationTestHarness::new();
    
    // Spawn animated character with PBR material
    let character = harness.spawn_character("knight.glb", "metal_armor.mat");
    
    // Start animation
    harness.play_animation(character, "idle_loop")?;
    
    // Render 10 frames
    for _ in 0..10 {
        harness.run_frame()?;
    }
    let frame1 = harness.get_frame_buffer();
    
    // Hot-reload material (change albedo color)
    harness.update_material("metal_armor.mat", |mat| {
        mat.albedo_color = Vec3::new(1.0, 0.0, 0.0); // Red
    })?;
    
    // Render 10 more frames
    for _ in 0..10 {
        harness.run_frame()?;
    }
    let frame2 = harness.get_frame_buffer();
    
    // Material change should be visible (different frame buffers)
    assert_ne!(frame1, frame2, "Material hot-reload had no effect");
    
    // Animation should continue smoothly (no flicker/reset)
    let animation_time = harness.get_animation_time(character)?;
    assert!(animation_time > 0.3, "Animation did not progress during hot-reload");
    
    Ok(())
}
```

**Integration Test 3: LLM + Tool Sandbox + World Simulation**
```rust
#[test]
fn test_llm_plan_validation_and_execution() -> anyhow::Result<()> {
    let mut harness = IntegrationTestHarness::new();
    
    // Spawn AI agent
    let agent = harness.spawn_ai_agent(Vec3::ZERO);
    
    // Spawn enemy at (5, 0, 0)
    let enemy = harness.spawn_enemy(Vec3::new(5.0, 0.0, 0.0));
    
    // Agent generates plan to attack enemy
    harness.run_ai_planning(agent)?;
    
    // Get generated plan
    let plan = harness.get_current_plan(agent)?;
    
    // Validate plan against tool sandbox
    let validation_result = harness.validate_plan(&plan)?;
    assert!(validation_result.is_valid(), "Plan validation failed: {:?}", validation_result);
    
    // Execute plan
    for _ in 0..600 { // 10 seconds
        harness.run_frame()?;
        
        if harness.is_enemy_defeated(enemy)? {
            break;
        }
    }
    
    // Enemy should be defeated
    assert!(harness.is_enemy_defeated(enemy)?, "Agent failed to defeat enemy");
    
    Ok(())
}
```

**Integration Test 4-10**: (Abbreviated)
- Network replication + persistence + ECS
- Asset loading + hot-reload + rendering
- Physics simulation + determinism + replay
- Multi-agent coordination + AI planning
- Combat system + damage + animation triggers
- Navmesh + dynamic obstacles + re-pathing
- UI + editor + live material preview

**Acceptance Criteria**:
- [ ] 10+ integration tests implemented
- [ ] Tests run in CI (headless rendering)
- [ ] All tests pass with <2% flakiness
- [ ] Test execution time <10 minutes total

---

### Month 3: Quality Gates & Documentation

#### Week 9-10: CI Enhancement & Quality Gates

**Deliverable**: Comprehensive CI pipeline with quality enforcement

**GitHub Actions Workflow**: `.github/workflows/quality-gates.yml`

```yaml
name: Quality Gates

on:
  pull_request:
    branches: [ main, develop ]
  push:
    branches: [ main, develop ]

jobs:
  compile-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Check compilation
        run: cargo check --workspace --all-features
      
      - name: Check no unwrap in core
        run: |
          unwrap_count=$(grep -r "\.unwrap()" astraweave-ecs/src astraweave-core/src | wc -l)
          if [ "$unwrap_count" -gt 0 ]; then
            echo "ERROR: Found $unwrap_count .unwrap() calls in core crates"
            exit 1
          fi
  
  test-coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin
      
      - name: Run tests with coverage
        run: cargo tarpaulin --out Xml --output-dir ./coverage
      
      - name: Check coverage threshold
        run: |
          coverage=$(grep -oP 'line-rate="\K[^"]+' coverage/cobertura.xml | head -1)
          if (( $(echo "$coverage < 0.70" | bc -l) )); then
            echo "ERROR: Coverage $coverage is below 70% threshold"
            exit 1
          fi
  
  integration-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run integration tests
        run: cargo test -p astraweave-integration-tests --release
  
  llm-quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run LLM evaluation harness
        run: cargo test -p astraweave-llm-eval --release
      
      - name: Check quality score
        run: |
          score=$(cargo run -p astraweave-llm-eval -- --json | jq '.overall_score')
          if (( $(echo "$score < 90.0" | bc -l) )); then
            echo "ERROR: LLM quality score $score is below 90% threshold"
            exit 1
          fi
  
  performance-regression:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run benchmarks
        run: cargo bench --workspace -- --save-baseline current
      
      - name: Compare with main branch
        run: |
          git fetch origin main
          git checkout origin/main
          cargo bench --workspace -- --save-baseline main
          git checkout -
          
          # Use critcmp to compare baselines
          cargo install critcmp
          critcmp main current || true
```

**Acceptance Criteria**:
- [ ] CI enforces zero `.unwrap()` in core crates
- [ ] Test coverage threshold enforced (70%+)
- [ ] Integration tests run on every PR
- [ ] LLM quality score tracked
- [ ] Performance regression detected automatically

#### Week 11-12: Documentation Sprint

**Deliverable**: Comprehensive production-ready documentation

**Documentation Structure**:
```
docs/
├── GETTING_STARTED.md (Updated)
├── ARCHITECTURE.md (Updated)
├── API_REFERENCE.md (Auto-generated from rustdoc)
├── production/
│   ├── DEPLOYMENT_GUIDE.md
│   ├── ERROR_HANDLING.md
│   ├── PERFORMANCE_TUNING.md
│   └── TROUBLESHOOTING.md
├── development/
│   ├── CONTRIBUTING.md (Updated)
│   ├── TESTING_GUIDE.md
│   ├── DEBUGGING_GUIDE.md
│   └── CODE_STYLE.md
└── examples/
    ├── HELLO_COMPANION.md (Updated)
    ├── ADVANCED_AI_PLANNING.md (New)
    └── CUSTOM_MATERIALS.md (New)
```

**Key Documentation Updates**:

1. **ERROR_HANDLING.md**: Document all error types and recovery strategies
2. **PERFORMANCE_TUNING.md**: Performance baselines and optimization guides
3. **TESTING_GUIDE.md**: How to write/run unit, integration, and benchmark tests
4. **DEPLOYMENT_GUIDE.md**: Production deployment checklist

**Acceptance Criteria**:
- [ ] 90%+ API coverage in rustdoc
- [ ] All production guides complete
- [ ] Code examples tested and working
- [ ] Migration guide from old error handling

---

### Phase A Success Metrics

**Quantitative**:
- ✅ 0 `.unwrap()` in core crates (ecs, ai, physics, nav)
- ✅ 70%+ test coverage on core systems
- ✅ 14/14 integration tests passing
- ✅ LLM quality score: 95%+ (20+ scenarios)
- ✅ CI pipeline enforces quality gates

**Qualitative**:
- ✅ Team confidence in production deployment
- ✅ Clear error messages aid debugging
- ✅ Comprehensive testing prevents regressions
- ✅ Documentation enables community contributions

---

## Phase B: Performance & Scale (Months 4-6)

**Goal**: Achieve production performance targets and scale to realistic game scenarios  
**Focus**: Optimization, parallelization, memory management  
**Theme**: "Make it fast without breaking determinism"

### Month 4: Performance Profiling & Baseline Optimization

#### Week 13-14: Tracy Integration & Profiling

**Deliverable**: Real-time profiling with Tracy

**Implementation**:
```rust
// Add to Cargo.toml
[dependencies]
tracy-client = { version = "0.17", optional = true }

[features]
profiling = ["tracy-client"]
```

**Instrumentation**:
```rust
use tracy_client::span;

pub fn ecs_tick(world: &mut World, dt: f32) {
    let _span = span!("ecs_tick");
    
    {
        let _span = span!("stage_perception");
        world.run_stage(SystemStage::PERCEPTION);
    }
    
    {
        let _span = span!("stage_ai_planning");
        world.run_stage(SystemStage::AI_PLANNING);
    }
    
    {
        let _span = span!("stage_physics");
        world.run_stage(SystemStage::PHYSICS);
    }
    
    // ... etc
}
```

**Profiling Example**:
```rust
// examples/profiling_demo/src/main.rs
fn main() {
    // Initialize Tracy
    tracy_client::Client::start();
    
    // Create large world for stress test
    let mut world = World::new();
    for i in 0..1000 {
        world.spawn_entity_with_ai(i);
    }
    
    // Run 1000 frames while profiling
    for frame in 0..1000 {
        tracy_client::frame_mark(); // Mark frame boundary
        world.tick(1.0 / 60.0);
    }
}
```

**Run**:
```bash
# Build with profiling enabled
cargo build --release --features profiling -p profiling_demo

# Run Tracy server (download from https://github.com/wolfpld/tracy)
.\tracy.exe

# Run profiling demo
.\target\release\profiling_demo.exe
```

**Analysis**:
1. Identify top 10 hotspots (functions consuming >5% frame time)
2. Document findings in `docs/performance/PROFILING_REPORT.md`
3. Create optimization backlog (prioritized by impact)

**Acceptance Criteria**:
- [ ] Tracy integration working in examples
- [ ] Profiling recording captured (1000 frames)
- [ ] Top 10 hotspots documented
- [ ] Optimization backlog created

#### Week 15-16: Low-Hanging Fruit Optimizations

**Target**: 20-30% improvement in hot paths

**Optimization 1: ECS Query Iteration**
```rust
// Before: BTreeMap iteration (cache-unfriendly)
for (entity, components) in self.archetypes.iter() {
    // Process...
}

// After: Packed array iteration (cache-friendly)
impl Archetype {
    pub fn iter_packed(&self) -> impl Iterator<Item = (&Entity, &Components)> {
        // Store entities and components in separate contiguous arrays
        self.entities.iter().zip(self.components.iter())
    }
}
```

**Optimization 2: Material Batching**
```rust
// Before: One draw call per entity
for entity in visible_entities {
    renderer.draw(entity.mesh, entity.material);
}

// After: Batch by material
let batches = group_by_material(visible_entities);
for (material, instances) in batches {
    renderer.draw_instanced(material, instances);
}
```

**Optimization 3: Perception Snapshot Caching**
```rust
pub struct PerceptionCache {
    last_snapshot: Option<WorldSnapshot>,
    last_world_hash: u64,
}

impl PerceptionCache {
    pub fn get_or_build(&mut self, world: &World) -> &WorldSnapshot {
        let world_hash = world.compute_hash(); // Hash entity positions/states
        
        if Some(world_hash) == self.last_world_hash.as_ref().copied() {
            // World unchanged, reuse snapshot
            return self.last_snapshot.as_ref().unwrap();
        }
        
        // World changed, rebuild snapshot
        self.last_snapshot = Some(build_world_snapshot(world));
        self.last_world_hash = Some(world_hash);
        self.last_snapshot.as_ref().unwrap()
    }
}
```

**Acceptance Criteria**:
- [ ] 3+ optimizations implemented
- [ ] Benchmarks show 20-30% improvement
- [ ] No regressions in functionality
- [ ] Determinism preserved (capture/replay tests pass)

---

### Month 5: Parallel Systems & Material Batching

#### Week 17-19: Parallel ECS Scheduling

**Deliverable**: Multi-threaded system execution with determinism

**Implementation**:
```rust
use rayon::prelude::*;

pub struct ParallelSystemStage {
    systems: Vec<Box<dyn System>>,
    dependencies: DependencyGraph,
}

impl ParallelSystemStage {
    pub fn run(&mut self, world: &mut World) {
        // Build execution batches (systems with no dependencies can run in parallel)
        let batches = self.dependencies.build_batches();
        
        for batch in batches {
            // Run systems in batch concurrently
            batch.par_iter_mut().for_each(|system| {
                system.run(world);
            });
        }
    }
}

// Parallel query iteration
impl<T: Component> Query<T> {
    pub fn par_iter(&self) -> impl ParallelIterator<Item = &T> {
        // Partition entities into chunks
        let chunk_size = (self.len() + num_cpus::get() - 1) / num_cpus::get();
        
        // CRITICAL: Sort entities first (determinism)
        let mut entities: Vec<_> = self.entities.iter().collect();
        entities.sort_by_key(|e| e.id());
        
        entities.par_chunks(chunk_size).flat_map(|chunk| {
            chunk.iter().map(|e| self.get(*e).unwrap())
        })
    }
}
```

**Determinism Validation**:
```rust
#[test]
fn test_parallel_systems_determinism() {
    // Run simulation 10 times with parallel systems
    let mut results = Vec::new();
    for _ in 0..10 {
        let mut world = create_test_world(1000);
        for _ in 0..100 {
            world.tick_parallel(1.0 / 60.0);
        }
        results.push(world.compute_hash());
    }
    
    // All runs should produce identical world state
    let first = results[0];
    for result in &results[1..] {
        assert_eq!(*result, first, "Parallel systems are non-deterministic!");
    }
}
```

**Performance Target**:
- Single-threaded: 100 entities @ 60fps
- 4-threaded: 400 entities @ 60fps (4x improvement)
- 8-threaded: 600 entities @ 60fps (6x improvement, not perfect scaling)

**Acceptance Criteria**:
- [ ] Parallel scheduling implemented with dependency graph
- [ ] Determinism preserved (10/10 test runs identical)
- [ ] 2-3x throughput improvement on 4-core CPU
- [ ] Feature flag `parallel-ecs` (opt-in)

#### Week 20: Material Batching & Texture Streaming

**Deliverable**: Optimized rendering pipeline

**Implementation** (see Month 5 section of strategic analysis)

**Acceptance Criteria**:
- [ ] Material instancing reduces draw calls by 5-10x
- [ ] Texture streaming keeps GPU memory <4GB
- [ ] 30-50% frame time reduction

---

### Month 6: LLM Batch Optimization & RAG Foundation

#### Week 21-22: LLM Batch Inference Tuning

**Current State**: Batch size 16, 4 workers, dynamic batching enabled

**Optimizations**:
1. **Increase Batch Size**: 16 → 32 (better GPU utilization)
2. **Adaptive Worker Scaling**: Scale workers based on queue depth
3. **Prompt Compression**: Reduce token count by 20-30%

**Implementation**:
```rust
// Adaptive worker scaling
impl BatchInferenceEngine {
    pub fn adjust_worker_count(&mut self) {
        let queue_depth = self.request_queue.read().await.len();
        
        let optimal_workers = if queue_depth > 100 {
            8 // High load: More workers
        } else if queue_depth > 50 {
            6 // Medium load
        } else {
            4 // Low load: Default
        };
        
        if optimal_workers != self.active_workers {
            self.scale_workers(optimal_workers).await;
        }
    }
}

// Prompt compression
impl PromptOptimizer {
    pub fn compress_prompt(&self, prompt: &str) -> String {
        let mut compressed = prompt.to_string();
        
        // Remove redundant whitespace
        compressed = compressed.split_whitespace().collect::<Vec<_>>().join(" ");
        
        // Abbreviate common terms
        compressed = compressed.replace("The player is at position", "Player pos:");
        compressed = compressed.replace("The enemy is at position", "Enemy pos:");
        
        // Remove verbose descriptions (keep essential info only)
        compressed = self.remove_fluff(&compressed);
        
        compressed
    }
}
```

**Performance Targets**:
- Batch throughput: 10 plans/sec → 20 plans/sec
- Single-request latency: 500ms → 400ms (prompt compression)
- Token usage: -20% (compression)

**Acceptance Criteria**:
- [ ] Batch throughput doubled
- [ ] Latency reduced by 20%
- [ ] Prompt compression functional (quality preserved)

#### Week 23-24: RAG System Foundation

**Deliverable**: Vector storage and retrieval infrastructure

**Implementation** (see MP-3 in strategic analysis)

**Acceptance Criteria**:
- [ ] Vector storage working (qdrant or hnswlib)
- [ ] Retrieval <50ms for top-5 memories
- [ ] Memory indexing integrated with ECS

---

### Phase B Success Metrics

**Quantitative**:
- ✅ 500+ entities @ 60fps (5x improvement from 100)
- ✅ Frame time p95 <16.67ms
- ✅ Material instancing 5-10x draw call reduction
- ✅ LLM batch throughput 20 plans/sec (2x improvement)
- ✅ Texture streaming <4GB GPU memory

**Qualitative**:
- ✅ Realistic game scenarios perform smoothly
- ✅ Profiling reveals no obvious bottlenecks
- ✅ Parallel systems maintain determinism
- ✅ RAG foundation ready for gameplay integration

---

## Phase C: Production Polish (Months 7-12)

**Goal**: Ship-quality engine with comprehensive tooling and community readiness  
**Focus**: Feature completeness, developer experience, cross-platform validation  
**Theme**: "Make it delightful to use"

### Month 7-8: RAG Memory System & Persona Tuning

#### Week 25-28: Long-Term Memory Integration

**Deliverable**: Fully functional RAG system with ECS integration

**Implementation** (see MP-3 in strategic analysis for detailed code)

**Features**:
1. **Semantic memory retrieval**: Given current situation, retrieve relevant past experiences
2. **Memory consolidation**: Summarize old memories to save tokens
3. **Persona-specific memories**: Each agent has unique memory store

**ECS Integration**:
```rust
pub fn memory_retrieval_system(
    query: Query<(&AIAgent, &PerceptionState, &mut MemoryContext)>,
    rag_service: Res<RagService>,
) {
    for (agent, perception, mut context) in query.iter_mut() {
        // Query RAG for relevant memories
        let current_situation = perception.last_snapshot.summary();
        let relevant_memories = rag_service.retrieve(
            agent.persona_id,
            &current_situation,
            5, // Top 5 memories
        );
        
        // Inject into context for LLM
        context.recent_memories = relevant_memories;
    }
}
```

**Evaluation**:
```rust
#[test]
fn test_rag_improves_llm_quality() {
    // Run evaluation harness with and without RAG
    let baseline_score = run_eval_without_rag(); // e.g., 85%
    let rag_score = run_eval_with_rag();         // e.g., 92%
    
    assert!(rag_score > baseline_score + 5.0, 
            "RAG did not improve quality by >5%");
}
```

**Acceptance Criteria**:
- [ ] RAG system integrated with AI planning
- [ ] Memory retrieval <50ms
- [ ] LLM quality score improves by >5% (A/B tested)
- [ ] Memory consolidation reduces token usage by 80%

---

### Month 9-10: Editor Stability & Workflow Polish

#### Week 29-32: Editor Production-Readiness

**Deliverable**: Stable editor with complete feature set

**Implementation**:

1. **Performance Panel** (Complete):
```rust
impl MaterialInspector {
    fn render_performance_panel(&mut self, ui: &mut egui::Ui, ctx: &EditorContext) {
        ui.heading("Performance Metrics");
        
        // Frame time graph (last 120 frames = 2 seconds @ 60fps)
        Plot::new("frame_time")
            .height(200.0)
            .show(ui, |plot_ui| {
                let points: Vec<_> = self.frame_times.iter()
                    .enumerate()
                    .map(|(i, &t)| [i as f64, t as f64])
                    .collect();
                plot_ui.line(Line::new(points));
            });
        
        // Real-time metrics
        ui.horizontal(|ui| {
            ui.label("FPS:");
            ui.label(format!("{:.1}", ctx.metrics.fps));
            
            ui.separator();
            
            ui.label("Frame Time:");
            ui.label(format!("{:.2}ms", ctx.metrics.frame_time_ms));
        });
        
        // Subsystem breakdown
        ui.collapsing("Subsystem Breakdown", |ui| {
            ui.label(format!("ECS Tick: {:.2}ms", ctx.metrics.ecs_time_ms));
            ui.label(format!("Rendering: {:.2}ms", ctx.metrics.render_time_ms));
            ui.label(format!("Physics: {:.2}ms", ctx.metrics.physics_time_ms));
            ui.label(format!("AI Planning: {:.2}ms", ctx.metrics.ai_time_ms));
        });
    }
}
```

2. **Undo/Redo System**:
```rust
pub struct TransactionManager {
    undo_stack: Vec<Transaction>,
    redo_stack: Vec<Transaction>,
    max_history: usize,
}

pub enum Transaction {
    SetMaterialProperty { material_id: u32, property: String, old_value: Value, new_value: Value },
    MoveEntity { entity: Entity, old_pos: Vec3, new_pos: Vec3 },
    DeleteEntity { entity: Entity, state: EntityState },
    // ... etc
}

impl TransactionManager {
    pub fn execute(&mut self, transaction: Transaction, world: &mut World) {
        transaction.apply(world);
        self.undo_stack.push(transaction);
        self.redo_stack.clear(); // Clear redo on new action
        
        // Limit history size
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }
    
    pub fn undo(&mut self, world: &mut World) -> Option<()> {
        let transaction = self.undo_stack.pop()?;
        transaction.revert(world);
        self.redo_stack.push(transaction);
        Some(())
    }
    
    pub fn redo(&mut self, world: &mut World) -> Option<()> {
        let transaction = self.redo_stack.pop()?;
        transaction.apply(world);
        self.undo_stack.push(transaction);
        Some(())
    }
}
```

3. **Visual Debugging**:
```rust
// Draw physics colliders in editor
pub fn debug_draw_colliders(
    render_context: &mut RenderContext,
    physics: &PhysicsWorld,
) {
    for (handle, collider) in physics.colliders().iter() {
        let shape = collider.shape();
        let position = collider.position();
        
        match shape.shape_type() {
            ShapeType::Ball => {
                render_context.draw_sphere(position, shape.as_ball().radius(), Color::GREEN);
            },
            ShapeType::Cuboid => {
                render_context.draw_box(position, shape.as_cuboid().half_extents(), Color::BLUE);
            },
            ShapeType::Capsule => {
                render_context.draw_capsule(position, shape.as_capsule(), Color::YELLOW);
            },
            _ => {},
        }
    }
}

// Draw navmesh in editor
pub fn debug_draw_navmesh(
    render_context: &mut RenderContext,
    navmesh: &NavMesh,
) {
    for triangle in navmesh.triangles() {
        render_context.draw_triangle(triangle.vertices(), Color::CYAN.with_alpha(0.3));
    }
    
    for edge in navmesh.portal_edges() {
        render_context.draw_line(edge.start, edge.end, Color::RED);
    }
}
```

**Stability Testing**:
```rust
#[test]
#[ignore] // Long-running test
fn test_editor_8_hour_session() {
    let mut editor = Editor::new();
    
    // Simulate 8 hours of editing (28,800 seconds @ 60fps)
    for _ in 0..(8 * 60 * 60 * 60) {
        // Perform random edits
        if rand::random::<f32>() < 0.1 {
            editor.modify_material(random_material());
        }
        
        if rand::random::<f32>() < 0.05 {
            editor.move_entity(random_entity(), random_position());
        }
        
        editor.render_frame();
        
        // Check for memory leaks
        if editor.frame_count() % 1000 == 0 {
            let memory_usage = get_memory_usage();
            assert!(memory_usage < 8 * 1024 * 1024 * 1024, // 8GB limit
                    "Memory leak detected: {}GB", memory_usage / (1024*1024*1024));
        }
    }
    
    // Editor should still be responsive
    assert!(editor.is_responsive(), "Editor became unresponsive");
}
```

**Acceptance Criteria**:
- [ ] Performance panel complete with real-time graphs
- [ ] Undo/redo system functional (20+ operations)
- [ ] Visual debugging for physics colliders and navmesh
- [ ] 8-hour stability test passes (no crashes, <8GB RAM)

---

### Month 11-12: Cross-Platform Validation & Community Launch

#### Week 33-36: Platform-Specific Testing

**Deliverable**: Verified builds on Windows, Linux, macOS

**CI Matrix**:
```yaml
strategy:
  matrix:
    os: [ubuntu-22.04, windows-2022, macos-13]
    rust: [stable, nightly]

steps:
  - name: Install dependencies (Linux)
    if: matrix.os == 'ubuntu-22.04'
    run: |
      sudo apt-get update
      sudo apt-get install -y libasound2-dev libudev-dev pkg-config
  
  - name: Build
    run: cargo build --release --all-features
  
  - name: Test
    run: cargo test --release --all-features
  
  - name: Run examples (headless)
    run: |
      cargo run --release -p hello_companion
      # ... other examples
```

**Platform-Specific Issues**:
- **Linux**: Audio backend (ALSA vs PulseAudio)
- **macOS**: Metal shader compatibility (vs Vulkan)
- **Windows**: DirectX 12 vs Vulkan performance

**Acceptance Criteria**:
- [ ] CI passes on all 3 platforms
- [ ] Platform-specific issues documented
- [ ] Performance parity ±10% across platforms

#### Week 37-40: Documentation & Community Launch

**Deliverable**: Production-ready documentation and launch materials

**Documentation Updates**:
1. **Tutorial Series**: "Build Your First AI Game in 30 Minutes"
2. **Video Walkthroughs**: YouTube series (5-10 episodes)
3. **Case Study**: "How We Built Veilweaver Demo"
4. **API Showcase**: Interactive examples on website

**Community Launch**:
1. **Blog Post**: "AstraWeave v1.0: Production-Ready AI-Native Game Engine"
2. **Reddit/HN**: Post on r/rust_gamedev, r/gamedev, Hacker News
3. **Twitter/X**: Launch thread with screenshots/videos
4. **Discord**: Create community server for support

**Acceptance Criteria**:
- [ ] 5+ tutorial docs published
- [ ] 3+ video walkthroughs on YouTube
- [ ] Case study published
- [ ] Community channels active

---

### Phase C Success Metrics

**Quantitative**:
- ✅ RAG system improves LLM quality by >5%
- ✅ Editor survives 8-hour stress test
- ✅ CI passes on Windows, Linux, macOS
- ✅ Documentation coverage 90%+
- ✅ 100+ community members (Discord)

**Qualitative**:
- ✅ Positive community feedback
- ✅ First external contribution merged
- ✅ Demo game (Veilweaver) showcases capabilities
- ✅ Team proud to present at conferences

---

## 12-Month Milestone Summary

| Phase | Months | Key Deliverables | Success Metrics |
|-------|--------|------------------|-----------------|
| **Phase A: Foundation** | 1-3 | Zero unwrap, testing infrastructure, LLM eval | 0 unwrap (core), 70% coverage, 95% LLM quality |
| **Phase B: Performance** | 4-6 | Parallel ECS, material batching, RAG foundation | 500 entities @ 60fps, <16.67ms p95 |
| **Phase C: Polish** | 7-12 | RAG integration, editor stability, community launch | Production-ready, cross-platform, 100+ users |

---

## Risk Management

### High-Risk Areas

1. **Parallel ECS Determinism**: Mitigation = Extensive testing, feature flag, conservative rollout
2. **LLM Quality Degradation**: Mitigation = Evaluation harness, fallback strategies, A/B testing
3. **Cross-Platform Issues**: Mitigation = Early CI integration, platform-specific testing, community beta
4. **Community Adoption**: Mitigation = High-quality docs, video tutorials, responsive support

### Contingency Plans

- **Parallel ECS Fails**: Keep single-threaded, optimize hot paths instead
- **RAG Doesn't Improve Quality**: Simplify to in-context learning only
- **Editor Stability Issues**: Release as beta, iterate with community feedback
- **Timeline Slippage**: Prioritize Phase A/B (foundation + performance), defer Phase C polish

---

## Success Celebration

### Quarterly Milestones

**Q1 (Month 3)**: Foundation Complete Party 🎉
- Blog post: "AstraWeave Foundation Hardening: 70% Test Coverage, Zero Unwrap"
- Team retrospective and lessons learned
- Celebrate first external PR

**Q2 (Month 6)**: Performance Victory Lap 🏁
- Blog post: "AstraWeave Performance: 500 Entities @ 60fps with Parallel ECS"
- Tracy profiling case study
- Community demo: Large-scale battle simulation

**Q3 (Month 9)**: Feature Showcase 🚀
- Blog post: "AstraWeave RAG: AI Agents with Long-Term Memory"
- Video: "Building an AI-Driven RPG in AstraWeave"
- Conference talk submission (GDC, Rust Conf)

**Q4 (Month 12)**: Production Launch 🎊
- Blog post: "AstraWeave v1.0: Production-Ready AI-Native Game Engine"
- Community launch on all platforms
- Release party (virtual or in-person)

---

## Next Actions

### This Week (Immediate)
1. Review this plan with team/stakeholders
2. Create GitHub project board with all tasks
3. Assign owners for Phase A deliverables
4. Schedule weekly sync meetings

### This Month (Phase A Start)
1. Execute Week 1 immediate actions (see IMMEDIATE_ACTIONS_IMPLEMENTATION_PLAN.md)
2. Begin `.unwrap()` replacement in core crates
3. Implement LLM evaluation harness
4. Set up enhanced CI pipeline

### This Quarter (Phase A Complete)
1. Achieve all Phase A success metrics
2. Document wins and lessons learned
3. Celebrate with team and community
4. Plan Phase B kickoff

---

**Document Status**: ✅ Complete and Ready for Execution  
**Owner**: Core Team  
**Review Schedule**: Monthly (end of each phase)  
**Next Review**: 2025-11-08 (End of Month 1)  
**Last Updated**: 2025-10-08
