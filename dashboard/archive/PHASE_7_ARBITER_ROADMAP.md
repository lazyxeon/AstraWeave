# Phase 7: GOAP+Hermes Hybrid Arbiter - Implementation Roadmap

**Date**: October 15, 2025  
**Status**: IN PROGRESS  
**Objective**: Implement hot-swappable AI control where GOAP provides instant tactical responses while Hermes generates strategic plans asynchronously in the background.

---

## Executive Summary

**Problem**: Hermes 2 Pro has 21s latency (despite 38% optimization), creating unacceptable delays for real-time AI control.

**Solution**: Invert priority - GOAP maintains instant control (5-30 µs) while Hermes plans asynchronously (0-21s). Smooth transition when LLM plan ready.

**Expected Outcome**:
- ✅ Zero user-facing latency (GOAP always responds instantly)
- ✅ Strategic depth when Hermes plan completes
- ✅ Graceful degradation (GOAP fallback on LLM failure)
- ✅ Production-ready with comprehensive testing

---

## Phase Overview

| Phase | Description | Duration | Lines of Code | Status |
|-------|-------------|----------|---------------|--------|
| **Phase 1** | AsyncTask & LlmExecutor Infrastructure | 2-3 hours | ~200 LOC | ⏳ NEXT |
| **Phase 2** | AIArbiter Core Implementation | 3-4 hours | ~400 LOC | 📋 PLANNED |
| **Phase 3** | GOAP Integration & Optimization | 1-2 hours | ~100 LOC | 📋 PLANNED |
| **Phase 4** | hello_companion Integration | 1-2 hours | ~150 LOC | 📋 PLANNED |
| **Phase 5** | Testing & Validation | 2-3 hours | ~300 LOC | 📋 PLANNED |
| **Phase 6** | Benchmarking & Performance | 1-2 hours | ~150 LOC | 📋 PLANNED |
| **Phase 7** | Documentation & Polish | 1-2 hours | ~200 LOC | 📋 PLANNED |

**Total Estimate**: 11-18 hours (1.5-2.5 days)  
**Total Code**: ~1,500 lines (well-tested, documented)

---

## Detailed Implementation Plan

---

### **PHASE 1: AsyncTask & LlmExecutor Infrastructure** ⏳ NEXT

**Duration**: 2-3 hours  
**Dependencies**: None  
**Output**: Foundation for non-blocking LLM execution

#### 1.1 Create AsyncTask Wrapper

**File**: `astraweave-ai/src/async_task.rs` (NEW)

**Requirements**:
1. Generic wrapper around `tokio::task::JoinHandle<T>`
2. Non-blocking `try_recv()` method (returns `Option<Result<T>>`)
3. Timeout support (optional `Duration` parameter)
4. Error handling without panics (`.unwrap()` forbidden)
5. Feature-gated behind `llm_orchestrator` (requires tokio)

**API Design**:
```rust
pub struct AsyncTask<T> {
    handle: tokio::task::JoinHandle<T>,
    started_at: std::time::Instant,
    timeout: Option<std::time::Duration>,
}

impl<T> AsyncTask<T> {
    pub fn new(handle: JoinHandle<T>) -> Self;
    pub fn with_timeout(handle: JoinHandle<T>, timeout: Duration) -> Self;
    pub fn try_recv(&mut self) -> Option<Result<T>>;
    pub fn is_finished(&self) -> bool;
    pub fn elapsed(&self) -> Duration;
}
```

**Success Criteria**:
- ✅ Compiles with `cargo check -p astraweave-ai --features llm_orchestrator`
- ✅ `try_recv()` returns `None` when task pending, `Some(Ok(T))` when complete
- ✅ No blocking calls (verified via code review)
- ✅ Timeout detection works (unit test)

**Testing**:
```rust
#[test]
fn test_async_task_pending() {
    // Spawn long-running task
    // Verify try_recv() returns None
}

#[test]
fn test_async_task_complete() {
    // Spawn instant task
    // Verify try_recv() returns Some(Ok(value))
}

#[test]
fn test_async_task_timeout() {
    // Spawn task with short timeout
    // Verify timeout detection
}
```

#### 1.2 Create LlmExecutor

**File**: `astraweave-ai/src/llm_executor.rs` (NEW)

**Requirements**:
1. Wrap existing `dyn OrchestratorAsync` (from Phase 6/7)
2. Store shared `tokio::runtime::Handle` (avoid creating new runtime)
3. Provide `generate_plan_async()` using `spawn_blocking` (CPU-bound work)
4. Provide `generate_plan_sync()` for testing (block_on wrapper)
5. Handle `WorldSnapshot` cloning (async task owns data)
6. Integrate with existing Phase 7 fallback system

**API Design**:
```rust
pub struct LlmExecutor {
    orchestrator: Arc<dyn OrchestratorAsync + Send + Sync>,
    runtime: tokio::runtime::Handle,
}

impl LlmExecutor {
    pub fn new(
        orchestrator: Arc<dyn OrchestratorAsync + Send + Sync>,
        runtime: tokio::runtime::Handle,
    ) -> Self;
    
    pub fn generate_plan_async(
        &self,
        snap: WorldSnapshot,
    ) -> AsyncTask<PlanIntent>;
    
    pub fn generate_plan_sync(
        &self,
        snap: &WorldSnapshot,
    ) -> Result<PlanIntent>;
}
```

**Implementation Notes**:
- Use `Arc` to share orchestrator across async boundary
- Clone `WorldSnapshot` before moving into async task
- Use `spawn_blocking` for LLM inference (CPU-bound, not I/O-bound)
- Propagate errors via `Result` in `AsyncTask` payload

**Success Criteria**:
- ✅ Compiles with `cargo check -p astraweave-ai --features llm_orchestrator`
- ✅ `generate_plan_async()` returns immediately (<1 ms)
- ✅ Async task completes in background (verify with println!)
- ✅ Sync path works for testing

**Testing**:
```rust
#[test]
fn test_llm_executor_async_returns_immediately() {
    // Create executor with mock orchestrator
    // Call generate_plan_async()
    // Verify returns in <1ms
}

#[tokio::test]
async fn test_llm_executor_async_completion() {
    // Create executor with fast mock orchestrator
    // Call generate_plan_async()
    // Poll until complete
    // Verify plan received
}
```

#### 1.3 Update astraweave-ai Module Exports

**File**: `astraweave-ai/src/lib.rs`

**Changes**:
```rust
#[cfg(feature = "llm_orchestrator")]
pub mod async_task;
#[cfg(feature = "llm_orchestrator")]
pub mod llm_executor;

#[cfg(feature = "llm_orchestrator")]
pub use async_task::AsyncTask;
#[cfg(feature = "llm_orchestrator")]
pub use llm_executor::LlmExecutor;
```

**Success Criteria**:
- ✅ Exports visible when `llm_orchestrator` feature enabled
- ✅ No warnings about unused modules
- ✅ Documentation builds: `cargo doc -p astraweave-ai --features llm_orchestrator`

#### 1.4 Phase 1 Validation

**Commands**:
```powershell
# Compile check
cargo check -p astraweave-ai --features llm_orchestrator

# Run tests
cargo test -p astraweave-ai --features llm_orchestrator -- async_task
cargo test -p astraweave-ai --features llm_orchestrator -- llm_executor

# Verify no warnings
cargo clippy -p astraweave-ai --features llm_orchestrator -- -D warnings
```

**Deliverables**:
- ✅ `astraweave-ai/src/async_task.rs` (~100 LOC + tests)
- ✅ `astraweave-ai/src/llm_executor.rs` (~100 LOC + tests)
- ✅ Updated `astraweave-ai/src/lib.rs`
- ✅ All tests passing
- ✅ Zero compilation warnings

---

### **PHASE 2: AIArbiter Core Implementation** 📋 PLANNED

**Duration**: 3-4 hours  
**Dependencies**: Phase 1 complete  
**Output**: Hot-swappable AI controller

#### 2.1 Define AIArbiter Types

**File**: `astraweave-ai/src/ai_arbiter.rs` (NEW)

**Enums & Structs**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AIControlMode {
    GOAP,                       // Tactical control (instant)
    ExecutingLLM { step: usize }, // Executing Hermes plan step N
    BehaviorTree,               // Fallback if GOAP unavailable
}

pub struct AIArbiter {
    // Active controller
    active_mode: AIControlMode,
    
    // AI systems
    goap: Box<dyn Orchestrator>,
    behavior_tree: Option<Box<dyn Orchestrator>>,
    llm_executor: Option<LlmExecutor>,
    
    // LLM state tracking
    llm_plan_pending: Option<AsyncTask<PlanIntent>>,
    llm_plan_active: Option<PlanIntent>,
    llm_plan_step: usize,
    llm_last_request: Instant,
    
    // Configuration
    llm_replan_interval: Duration,
    enable_llm: bool,
}
```

**Constructor**:
```rust
impl AIArbiter {
    pub fn new(
        goap: Box<dyn Orchestrator>,
        behavior_tree: Option<Box<dyn Orchestrator>>,
        llm_executor: Option<LlmExecutor>,
    ) -> Self;
    
    pub fn with_replan_interval(mut self, interval: Duration) -> Self;
    pub fn with_llm_enabled(mut self, enabled: bool) -> Self;
}
```

#### 2.2 Implement Core Update Loop

**Method**: `AIArbiter::update()`

**Logic Flow**:
```rust
pub fn update(&mut self, snap: &WorldSnapshot) -> ActionStep {
    // 1. Poll for LLM plan completion (non-blocking)
    self.poll_llm_result();
    
    // 2. Select action based on mode
    let action = match self.active_mode {
        AIControlMode::GOAP => self.execute_goap(snap),
        AIControlMode::ExecutingLLM { step } => self.execute_llm_step(step, snap),
        AIControlMode::BehaviorTree => self.execute_bt(snap),
    };
    
    // 3. Request new LLM plan if appropriate
    self.maybe_request_llm(snap);
    
    action
}
```

**Helper Methods**:
```rust
fn poll_llm_result(&mut self) {
    if let Some(ref mut task) = self.llm_plan_pending {
        if let Some(result) = task.try_recv() {
            match result {
                Ok(plan) => self.transition_to_llm(plan),
                Err(e) => {
                    tracing::warn!("LLM planning failed: {}", e);
                    self.transition_to_goap();
                }
            }
            self.llm_plan_pending = None;
        }
    }
}

fn maybe_request_llm(&mut self, snap: &WorldSnapshot) {
    if !self.enable_llm { return; }
    if self.llm_plan_pending.is_some() { return; }
    
    let elapsed = self.llm_last_request.elapsed();
    if elapsed >= self.llm_replan_interval {
        if let Some(ref executor) = self.llm_executor {
            let task = executor.generate_plan_async(snap.clone());
            self.llm_plan_pending = Some(task);
            self.llm_last_request = Instant::now();
            tracing::info!("Requesting new LLM plan");
        }
    }
}
```

#### 2.3 Implement Mode Transitions

**Transition to LLM**:
```rust
fn transition_to_llm(&mut self, plan: PlanIntent) {
    if plan.steps.is_empty() {
        tracing::warn!("LLM returned empty plan, staying in GOAP mode");
        return;
    }
    
    tracing::info!(
        "Hermes plan ready ({} steps), taking control: {}",
        plan.steps.len(),
        plan.plan_id
    );
    
    self.llm_plan_active = Some(plan);
    self.llm_plan_step = 0;
    self.active_mode = AIControlMode::ExecutingLLM { step: 0 };
}

fn transition_to_goap(&mut self) {
    tracing::info!("Transitioning to GOAP control");
    self.llm_plan_active = None;
    self.llm_plan_step = 0;
    self.active_mode = AIControlMode::GOAP;
}
```

#### 2.4 Implement Execution Methods

**GOAP Execution**:
```rust
fn execute_goap(&self, snap: &WorldSnapshot) -> ActionStep {
    self.goap.propose_plan(snap)
        .steps
        .into_iter()
        .next()
        .unwrap_or_else(|| ActionStep::Idle)
}
```

**LLM Execution**:
```rust
fn execute_llm_step(&mut self, step: usize, snap: &WorldSnapshot) -> ActionStep {
    if let Some(ref plan) = self.llm_plan_active {
        if step < plan.steps.len() {
            let action = plan.steps[step].clone();
            
            // Validate action is still appropriate
            if self.is_action_valid(&action, snap) {
                self.llm_plan_step += 1;
                if self.llm_plan_step >= plan.steps.len() {
                    tracing::info!("LLM plan exhausted, GOAP resuming control");
                    self.transition_to_goap();
                } else {
                    self.active_mode = AIControlMode::ExecutingLLM { 
                        step: self.llm_plan_step 
                    };
                }
                return action;
            } else {
                tracing::warn!("LLM action no longer valid, reverting to GOAP");
                self.transition_to_goap();
            }
        }
    }
    
    // Fallback to GOAP
    self.transition_to_goap();
    self.execute_goap(snap)
}

fn is_action_valid(&self, action: &ActionStep, snap: &WorldSnapshot) -> bool {
    // TODO: Integrate with tool_sandbox validation
    // For now, basic checks
    true
}
```

#### 2.5 Phase 2 Validation

**Commands**:
```powershell
cargo check -p astraweave-ai --features "ai-goap,llm_orchestrator"
cargo clippy -p astraweave-ai --features "ai-goap,llm_orchestrator" -- -D warnings
```

**Deliverables**:
- ✅ `astraweave-ai/src/ai_arbiter.rs` (~400 LOC)
- ✅ Updated `astraweave-ai/src/lib.rs` exports
- ✅ Zero compilation errors/warnings

---

### **PHASE 3: GOAP Integration & Optimization** 📋 PLANNED

**Duration**: 1-2 hours  
**Dependencies**: Phase 2 complete  
**Output**: Fast GOAP tactical controller

#### 3.1 Enhance GoapOrchestrator

**File**: `astraweave-ai/src/orchestrator.rs`

**Add Helper Method**:
```rust
impl GoapOrchestrator {
    /// Fast path: return single best action instead of full plan
    pub fn next_action(&self, snap: &WorldSnapshot) -> ActionStep {
        let plan = self.propose_plan(snap);
        plan.steps.into_iter().next().unwrap_or(ActionStep::Idle)
    }
}
```

**Optimization**: Ensure single-step planning (don't compute full N-step plan if only need immediate action)

#### 3.2 Update AIArbiter to Use Fast Path

**File**: `astraweave-ai/src/ai_arbiter.rs`

**Change**:
```rust
fn execute_goap(&self, snap: &WorldSnapshot) -> ActionStep {
    // Use next_action() fast path if available
    if let Some(goap) = self.goap.as_any().downcast_ref::<GoapOrchestrator>() {
        goap.next_action(snap)
    } else {
        self.goap.propose_plan(snap)
            .steps
            .into_iter()
            .next()
            .unwrap_or(ActionStep::Idle)
    }
}
```

#### 3.3 Phase 3 Validation

**Benchmark**:
```powershell
cargo bench -p astraweave-ai --bench ai_core_loop -- goap
```

**Success Criteria**:
- ✅ GOAP execution <100 µs (verify in benchmark output)
- ✅ No performance regression vs Phase 6 baseline

**Deliverables**:
- ✅ Enhanced GOAP orchestrator (~50 LOC)
- ✅ Updated arbiter GOAP execution (~50 LOC)

---

### **PHASE 4: hello_companion Integration** 📋 PLANNED

**Duration**: 1-2 hours  
**Dependencies**: Phase 3 complete  
**Output**: Working demo with `--arbiter` mode

#### 4.1 Add Arbiter CLI Flag

**File**: `examples/hello_companion/src/main.rs`

**Add to AIMode enum**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AIMode {
    Classical,
    #[cfg(feature = "llm")]
    BehaviorTree,
    #[cfg(feature = "llm")]
    Utility,
    #[cfg(feature = "ollama")]
    LLM,
    #[cfg(feature = "ollama")]
    Hybrid,
    #[cfg(feature = "llm")]
    Ensemble,
    #[cfg(all(feature = "llm", feature = "ai-goap"))]
    Arbiter,  // NEW
}
```

**Add CLI parsing**:
```rust
let ai_mode = if args.contains(&"--arbiter".to_string()) {
    #[cfg(all(feature = "llm", feature = "ai-goap"))]
    { AIMode::Arbiter }
    #[cfg(not(all(feature = "llm", feature = "ai-goap")))]
    {
        eprintln!("--arbiter requires llm and ai-goap features");
        std::process::exit(1);
    }
} else if args.contains(&"--llm".to_string()) {
    // ... existing logic
}
```

#### 4.2 Create Arbiter Initialization

**Add function**:
```rust
#[cfg(all(feature = "llm", feature = "ai-goap"))]
fn create_arbiter(rt: &tokio::runtime::Runtime) -> AIArbiter {
    use astraweave_ai::{AIArbiter, LlmExecutor, GoapOrchestrator};
    use astraweave_llm::hermes2pro_ollama::Hermes2ProOllama;
    use std::sync::Arc;
    
    // GOAP planner (always available, instant)
    let goap = Box::new(GoapOrchestrator);
    
    // LLM executor (async, 13-21s)
    let client = Hermes2ProOllama::new(
        "http://127.0.0.1:11434".to_string(),
        "adrienbrault/nous-hermes2pro:Q4_K_M".to_string(),
    )
    .with_temperature(0.5)
    .with_max_tokens(256);  // Optimized from Phase 7
    
    let registry = astraweave_core::default_tool_registry();
    let orchestrator = Arc::new(astraweave_llm::FallbackOrchestrator::new(
        client,
        registry,
    ));
    
    let llm_executor = LlmExecutor::new(orchestrator, rt.handle().clone());
    
    AIArbiter::new(goap, None, Some(llm_executor))
        .with_replan_interval(Duration::from_secs(30))
        .with_llm_enabled(true)
}
```

#### 4.3 Update Game Loop

**Modify main loop**:
```rust
#[cfg(all(feature = "llm", feature = "ai-goap"))]
if ai_mode == AIMode::Arbiter {
    let mut arbiter = create_arbiter(&rt);
    
    loop {
        let snap = build_snapshot(&world, cfg.perception_range);
        
        // AIArbiter returns ActionStep instantly (GOAP or LLM)
        let action = arbiter.update(&snap);
        
        // Validate and execute
        match validate_and_execute(&mut world, snap.me.pos, &action, &vcfg) {
            Ok(outcome) => {
                println!("✅ Executed: {:?} -> {:?}", action, outcome);
            }
            Err(e) => {
                println!("❌ Validation failed: {}", e);
            }
        }
        
        // Step simulation
        step(&mut world, &cfg);
        
        // Exit conditions...
    }
}
```

#### 4.4 Add Debug Logging

**In AIArbiter transitions**:
```rust
// In transition_to_llm()
println!("[AIArbiter] Hermes plan ready ({} steps), taking control", plan.steps.len());

// In transition_to_goap()
println!("[AIArbiter] GOAP resuming control");

// In update() GOAP path
if self.active_mode == AIControlMode::GOAP {
    println!("[AIArbiter] GOAP providing instant action");
}
```

#### 4.5 Phase 4 Validation

**Commands**:
```powershell
# Compile check
cargo check -p hello_companion --features "llm,ollama,ai-goap"

# Run arbiter demo
cargo run -p hello_companion --release --features "llm,ollama,ai-goap" -- --arbiter

# Verify console output shows:
# - "GOAP providing instant action" (initial 0-21s)
# - "Hermes plan ready (N steps), taking control" (when LLM completes)
# - "Executing step X: ACTION" (LLM execution)
# - "GOAP resuming control" (after plan exhausted)
```

**Success Criteria**:
- ✅ Demo runs without crashes
- ✅ Console shows mode transitions
- ✅ No user-facing latency (instant actions every frame)
- ✅ Hermes plan executes when ready

**Deliverables**:
- ✅ Updated `hello_companion/src/main.rs` (~150 LOC)
- ✅ Arbiter mode working end-to-end

---

### **PHASE 5: Testing & Validation** 📋 PLANNED

**Duration**: 2-3 hours  
**Dependencies**: Phase 4 complete  
**Output**: Comprehensive test coverage

#### 5.1 Create Arbiter Unit Tests

**File**: `astraweave-ai/tests/arbiter_tests.rs` (NEW)

**Test Cases**:

**Test 1: GOAP-Only Mode**
```rust
#[test]
fn test_goap_only_control() {
    let goap = Box::new(GoapOrchestrator);
    let mut arbiter = AIArbiter::new(goap, None, None);
    
    let snap = create_test_snapshot();
    
    for _ in 0..100 {
        let action = arbiter.update(&snap);
        assert_ne!(action, ActionStep::Idle, "GOAP should always plan");
    }
    
    assert_eq!(arbiter.active_mode, AIControlMode::GOAP);
}
```

**Test 2: LLM Transition**
```rust
#[tokio::test]
async fn test_llm_transition() {
    let goap = Box::new(GoapOrchestrator);
    let mock_llm = create_mock_llm_executor();
    let mut arbiter = AIArbiter::new(goap, None, Some(mock_llm))
        .with_replan_interval(Duration::from_millis(100));
    
    let snap = create_test_snapshot();
    
    // Initially GOAP
    let action1 = arbiter.update(&snap);
    assert_eq!(arbiter.active_mode, AIControlMode::GOAP);
    
    // Wait for LLM plan
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Next update should transition to LLM
    let action2 = arbiter.update(&snap);
    assert!(matches!(arbiter.active_mode, AIControlMode::ExecutingLLM { .. }));
}
```

**Test 3: Plan Exhaustion**
```rust
#[test]
fn test_plan_exhaustion() {
    let goap = Box::new(GoapOrchestrator);
    let mut arbiter = AIArbiter::new(goap, None, None);
    
    // Inject short LLM plan (2 steps)
    let short_plan = PlanIntent {
        plan_id: "test-plan".into(),
        steps: vec![
            ActionStep::MoveTo { x: 10, y: 10, speed: None },
            ActionStep::TakeCover { position: None },
        ],
    };
    
    arbiter.transition_to_llm(short_plan);
    
    let snap = create_test_snapshot();
    
    // Execute both steps
    let action1 = arbiter.update(&snap);
    assert!(matches!(arbiter.active_mode, AIControlMode::ExecutingLLM { step: 1 }));
    
    let action2 = arbiter.update(&snap);
    assert_eq!(arbiter.active_mode, AIControlMode::GOAP, "Should revert to GOAP after plan exhausted");
}
```

**Test 4: Async Task Completion**
```rust
#[tokio::test]
async fn test_async_task_non_blocking() {
    let (tx, rx) = tokio::sync::oneshot::channel();
    
    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        tx.send(42).unwrap();
        42
    });
    
    let mut task = AsyncTask::new(handle);
    
    // Should return None immediately (task still running)
    assert!(task.try_recv().is_none());
    
    // Wait for completion
    tokio::time::sleep(Duration::from_millis(150)).await;
    
    // Should return Some(Ok(42))
    match task.try_recv() {
        Some(Ok(value)) => assert_eq!(value, 42),
        _ => panic!("Expected completed task"),
    }
}
```

**Test 5: Error Handling**
```rust
#[test]
fn test_llm_failure_reverts_to_goap() {
    let goap = Box::new(GoapOrchestrator);
    let failing_llm = create_failing_llm_executor();
    let mut arbiter = AIArbiter::new(goap, None, Some(failing_llm));
    
    // Simulate LLM failure
    arbiter.llm_plan_pending = Some(create_failing_task());
    
    let snap = create_test_snapshot();
    let action = arbiter.update(&snap);
    
    // Should revert to GOAP on failure
    assert_eq!(arbiter.active_mode, AIControlMode::GOAP);
    assert!(arbiter.llm_plan_pending.is_none());
}
```

#### 5.2 Create Mock Helpers

**File**: `astraweave-ai/tests/common/mod.rs`

**Mock Implementations**:
```rust
pub fn create_test_snapshot() -> WorldSnapshot {
    WorldSnapshot {
        t: 0.0,
        me: CompanionState { /* ... */ },
        player: PlayerState { /* ... */ },
        enemies: vec![/* ... */],
        pois: vec![],
        obstacles: vec![],
        objective: Some("extract".into()),
    }
}

pub fn create_mock_llm_executor() -> LlmExecutor {
    // Create mock that returns plan after delay
}

pub fn create_failing_llm_executor() -> LlmExecutor {
    // Create mock that always fails
}

pub fn create_failing_task() -> AsyncTask<PlanIntent> {
    // Create task that returns error
}
```

#### 5.3 Phase 5 Validation

**Commands**:
```powershell
# Run all arbiter tests
cargo test -p astraweave-ai --test arbiter_tests --features "ai-goap,llm_orchestrator"

# Run with output
cargo test -p astraweave-ai --test arbiter_tests --features "ai-goap,llm_orchestrator" -- --nocapture

# Verify coverage
cargo test -p astraweave-ai --features "ai-goap,llm_orchestrator"
```

**Success Criteria**:
- ✅ All tests passing (5/5 minimum)
- ✅ No test flakiness (run 10 times, all pass)
- ✅ Tests run fast (<5s total)

**Deliverables**:
- ✅ `astraweave-ai/tests/arbiter_tests.rs` (~300 LOC)
- ✅ `astraweave-ai/tests/common/mod.rs` (~100 LOC)
- ✅ 100% test pass rate

---

### **PHASE 6: Benchmarking & Performance** 📋 PLANNED

**Duration**: 1-2 hours  
**Dependencies**: Phase 5 complete  
**Output**: Performance validation

#### 6.1 Create Arbiter Benchmarks

**File**: `astraweave-ai/benches/arbiter_bench.rs` (NEW)

**Benchmark 1: GOAP Update**
```rust
fn bench_arbiter_goap_update(c: &mut Criterion) {
    let goap = Box::new(GoapOrchestrator);
    let mut arbiter = AIArbiter::new(goap, None, None);
    let snapshot = create_benchmark_snapshot();
    
    c.bench_function("arbiter_goap_update", |b| {
        b.iter(|| {
            black_box(arbiter.update(&snapshot))
        });
    });
}
```

**Benchmark 2: Mode Transition Overhead**
```rust
fn bench_arbiter_transition_to_llm(c: &mut Criterion) {
    let goap = Box::new(GoapOrchestrator);
    let mut arbiter = AIArbiter::new(goap, None, None);
    let plan = create_benchmark_plan();
    
    c.bench_function("arbiter_transition_to_llm", |b| {
        b.iter(|| {
            arbiter.transition_to_llm(black_box(plan.clone()));
            arbiter.transition_to_goap();  // Reset for next iteration
        });
    });
}
```

**Benchmark 3: LLM Execution Path**
```rust
fn bench_arbiter_llm_execution(c: &mut Criterion) {
    let goap = Box::new(GoapOrchestrator);
    let mut arbiter = AIArbiter::new(goap, None, None);
    let plan = create_benchmark_plan();
    arbiter.transition_to_llm(plan);
    
    let snapshot = create_benchmark_snapshot();
    
    c.bench_function("arbiter_llm_execution", |b| {
        b.iter(|| {
            black_box(arbiter.execute_llm_step(0, &snapshot))
        });
    });
}
```

**Benchmark 4: Poll Overhead**
```rust
fn bench_arbiter_poll_pending(c: &mut Criterion) {
    let goap = Box::new(GoapOrchestrator);
    let mut arbiter = AIArbiter::new(goap, None, None);
    
    // Set up pending task (never completes)
    arbiter.llm_plan_pending = Some(create_pending_task());
    
    c.bench_function("arbiter_poll_pending", |b| {
        b.iter(|| {
            black_box(arbiter.poll_llm_result())
        });
    });
}
```

#### 6.2 Add Criterion Configuration

**File**: `astraweave-ai/Cargo.toml`

**Update benches section**:
```toml
[[bench]]
name = "arbiter_bench"
harness = false
required-features = ["ai-goap", "llm_orchestrator"]
```

#### 6.3 Phase 6 Validation

**Commands**:
```powershell
# Run benchmarks
cargo bench -p astraweave-ai --bench arbiter_bench --features "ai-goap,llm_orchestrator"

# Compare against baselines
cargo bench -p astraweave-ai --bench ai_core_loop --features "ai-goap"
```

**Performance Targets**:
- ✅ `arbiter_goap_update`: <100 µs (matches Phase 6 GOAP baseline)
- ✅ `arbiter_transition_to_llm`: <10 µs (negligible overhead)
- ✅ `arbiter_llm_execution`: <5 µs (vector indexing + clone)
- ✅ `arbiter_poll_pending`: <1 µs (JoinHandle::is_finished check)

**Success Criteria**:
- ✅ All benchmarks meet targets
- ✅ No performance regression vs Phase 6 baseline
- ✅ Benchmark output saved to `target/criterion/arbiter_*`

**Deliverables**:
- ✅ `astraweave-ai/benches/arbiter_bench.rs` (~150 LOC)
- ✅ Performance validation complete

---

### **PHASE 7: Documentation & Polish** 📋 PLANNED

**Duration**: 1-2 hours  
**Dependencies**: Phase 6 complete  
**Output**: Production-ready documentation

#### 7.1 Create Implementation Report

**File**: `PHASE_7_ARBITER_IMPLEMENTATION.md` (NEW)

**Sections**:
1. **Executive Summary**
   - Problem statement (21s latency)
   - Solution overview (GOAP-first arbiter)
   - Key achievements

2. **Architecture**
   - Component diagram
   - Data flow (GOAP → LLM → GOAP)
   - Mode transition state machine

3. **API Reference**
   - `AIArbiter` public API
   - `LlmExecutor` public API
   - `AsyncTask` public API

4. **Usage Guide**
   ```rust
   // Basic setup
   let arbiter = AIArbiter::new(goap, None, llm_executor)
       .with_replan_interval(Duration::from_secs(30));
   
   // Game loop
   loop {
       let action = arbiter.update(&snapshot);  // Instant response
       execute(action);
   }
   ```

5. **Performance Characteristics**
   - GOAP latency: 5-30 µs
   - LLM latency: 13-21s (background, non-blocking)
   - Mode transition overhead: <10 µs
   - Memory overhead: ~2 KB (plan storage)

6. **Testing**
   - Unit test coverage: 5 tests
   - Benchmark results
   - Manual validation procedure

7. **Integration Points**
   - Existing Phase 6/7 code reuse
   - Feature gating strategy
   - Dependency requirements

8. **Design Decisions**
   - Why GOAP-first vs LLM-first
   - Why async vs sync
   - Why tokio vs manual threading

9. **Future Enhancements**
   - State-based replanning (defer to Phase 9)
   - LLM plan caching (defer to Phase 9)
   - Multi-agent coordination (defer to Phase 9)

**Target Length**: 5,000-8,000 words

#### 7.2 Update hello_companion README

**File**: `examples/hello_companion/README.md`

**Add Arbiter Section**:
```markdown
## Arbiter Mode (NEW - Phase 7)

**What it does**: GOAP provides instant tactical control while Hermes generates strategic plans asynchronously.

**Usage**:
```bash
cargo run -p hello_companion --release --features "llm,ollama,ai-goap" -- --arbiter
```

**Expected Output**:
```
[AIArbiter] GOAP providing instant action
[AIArbiter] GOAP providing instant action
...
[AIArbiter] Hermes plan ready (3 steps), taking control
[AIArbiter] Executing step 1: MoveTo
[AIArbiter] Executing step 2: TakeCover
[AIArbiter] Executing step 3: Attack
[AIArbiter] GOAP resuming control
```

**Performance**:
- User-facing latency: 5-30 µs (GOAP instant response)
- Strategic planning: 13-21s (background, non-blocking)
- Mode transitions: <10 µs overhead

**Configuration**:
- Replan interval: 30 seconds (configurable)
- LLM timeout: 60 seconds (inherited from Phase 7)
- Fallback: GOAP always available
```

#### 7.3 Update Main README

**File**: `README.md`

**Add to AI Features Section**:
```markdown
### AI Arbiter (Phase 7 - October 2025)

Hot-swappable AI control system combining:
- **GOAP**: Instant tactical responses (5-30 µs)
- **Hermes 2 Pro**: Strategic planning (async, 13-21s)
- **Seamless Transitions**: Zero user-facing latency

**Key Benefits**:
- ✅ No waiting for LLM responses (GOAP maintains control)
- ✅ Strategic depth when Hermes plan ready
- ✅ Graceful degradation on LLM failure
- ✅ Production-ready with comprehensive testing

See `PHASE_7_ARBITER_IMPLEMENTATION.md` for details.
```

#### 7.4 Update Copilot Instructions

**File**: `.github/copilot-instructions.md`

**Add to Phase 7 Section**:
```markdown
- ✅ **Phase 7: AI Arbiter COMPLETE** (Oct 15, 2025)
   - **GOAP+Hermes hybrid control** (instant tactical + async strategic)
   - **AsyncTask infrastructure** (non-blocking LLM execution)
   - **Zero user-facing latency** (GOAP always responds instantly)
   - **Seamless mode transitions** (GOAP → LLM → GOAP)
   - **5 unit tests + 4 benchmarks** (100% pass rate, <100 µs GOAP)
   - **hello_companion --arbiter mode** (working end-to-end demo)
   - **Documentation**: PHASE_7_ARBITER_IMPLEMENTATION.md
```

#### 7.5 Create Quick Reference Card

**File**: `ARBITER_QUICK_REFERENCE.md` (NEW)

**Content**:
```markdown
# AI Arbiter Quick Reference

## One-Line Summary
GOAP provides instant AI responses while Hermes plans strategically in the background.

## When to Use
- Real-time games requiring <100 µs AI response
- Turn-based games wanting zero-latency feel
- Any scenario where 13-21s LLM latency is unacceptable

## Quick Start
```rust
use astraweave_ai::{AIArbiter, GoapOrchestrator, LlmExecutor};

let arbiter = AIArbiter::new(
    Box::new(GoapOrchestrator),
    None,
    Some(llm_executor),
)
.with_replan_interval(Duration::from_secs(30));

// In game loop
let action = arbiter.update(&snapshot);  // Always instant!
```

## Performance
- GOAP: 5-30 µs
- LLM: 13-21s (async, non-blocking)
- Transitions: <10 µs

## Files
- Core: `astraweave-ai/src/ai_arbiter.rs`
- Async: `astraweave-ai/src/llm_executor.rs`
- Demo: `examples/hello_companion/src/main.rs`
- Tests: `astraweave-ai/tests/arbiter_tests.rs`
```

#### 7.6 Phase 7 Final Validation

**Commands**:
```powershell
# Full workspace check
cargo check -p astraweave-ai --features "ai-goap,llm_orchestrator"
cargo check -p hello_companion --features "llm,ollama,ai-goap"

# All tests
cargo test -p astraweave-ai --features "ai-goap,llm_orchestrator"

# All benchmarks
cargo bench -p astraweave-ai --features "ai-goap,llm_orchestrator"

# Demo run
cargo run -p hello_companion --release --features "llm,ollama,ai-goap" -- --arbiter

# Documentation build
cargo doc -p astraweave-ai --features "ai-goap,llm_orchestrator" --no-deps
```

**Success Criteria**:
- ✅ All commands succeed with zero errors
- ✅ Documentation renders correctly in browser
- ✅ Demo shows expected mode transitions
- ✅ README updates are clear and accurate

**Deliverables**:
- ✅ `PHASE_7_ARBITER_IMPLEMENTATION.md` (~8,000 words)
- ✅ `ARBITER_QUICK_REFERENCE.md` (~500 words)
- ✅ Updated `README.md`, `hello_companion/README.md`
- ✅ Updated `.github/copilot-instructions.md`

---

## Success Metrics Summary

**Functional**:
- ✅ `hello_companion --arbiter` runs with zero user-facing latency
- ✅ Console shows mode transitions (GOAP → LLM → GOAP)
- ✅ GOAP maintains control during 0-21s LLM planning
- ✅ Hermes plan executes when ready
- ✅ Smooth transition back to GOAP after plan exhausted

**Performance**:
- ✅ GOAP update: <100 µs
- ✅ Async LLM request: non-blocking (<1 ms)
- ✅ Mode transitions: <10 µs overhead
- ✅ No regression vs Phase 6 baseline

**Quality**:
- ✅ All tests passing: 5+ unit tests
- ✅ All benchmarks passing: 4+ benchmarks
- ✅ Zero compilation errors/warnings
- ✅ Documentation complete (3 docs, 10,000+ words)

**Code Volume**:
- Core: ~700 LOC (async_task, llm_executor, ai_arbiter)
- Tests: ~400 LOC
- Benchmarks: ~150 LOC
- Integration: ~150 LOC (hello_companion)
- Docs: ~10,000 words
- **Total**: ~1,500 LOC, well-tested and documented

---

## Risk Mitigation

**Risk 1: Tokio Runtime Availability**
- **Mitigation**: Feature-gate arbiter behind `llm_orchestrator`, share runtime from caller
- **Fallback**: Provide compile error with helpful message if features missing

**Risk 2: Plan Validation Overhead**
- **Mitigation**: Reuse existing Phase 7 tool_sandbox validation
- **Fallback**: Basic validity checks if sandbox unavailable

**Risk 3: State Drift (LLM plan becomes stale)**
- **Mitigation**: Track snapshot hash, discard plan if world changed significantly
- **Fallback**: Always validate action before execution, revert to GOAP on failure

**Risk 4: Performance Regression**
- **Mitigation**: Comprehensive benchmarking in Phase 6
- **Fallback**: Optimize hot path (GOAP execution) if needed

**Risk 5: Feature Gating Complexity**
- **Mitigation**: Clear `#[cfg]` attributes, helpful compile errors
- **Fallback**: Document required features in error messages

---

## Timeline Estimate (Detailed)

**Day 1 (6-8 hours)**:
- Phase 1: AsyncTask + LlmExecutor (2-3 hours)
- Phase 2: AIArbiter Core (3-4 hours)
- Phase 3: GOAP Integration (1-2 hours)

**Day 2 (5-10 hours)**:
- Phase 4: hello_companion Integration (1-2 hours)
- Phase 5: Testing & Validation (2-3 hours)
- Phase 6: Benchmarking & Performance (1-2 hours)
- Phase 7: Documentation & Polish (1-2 hours)

**Total**: 11-18 hours (1.5-2.5 days of focused work)

**Optimistic**: 11 hours (if async primitives straightforward)
**Realistic**: 14 hours (with debugging and iteration)
**Conservative**: 18 hours (with unexpected issues)

---

## Validation Commands (Comprehensive)

```powershell
# ═══════════════════════════════════════════════════════════════
# PHASE 1: AsyncTask & LlmExecutor
# ═══════════════════════════════════════════════════════════════
cargo check -p astraweave-ai --features llm_orchestrator
cargo test -p astraweave-ai --features llm_orchestrator -- async_task
cargo test -p astraweave-ai --features llm_orchestrator -- llm_executor
cargo clippy -p astraweave-ai --features llm_orchestrator -- -D warnings

# ═══════════════════════════════════════════════════════════════
# PHASE 2: AIArbiter Core
# ═══════════════════════════════════════════════════════════════
cargo check -p astraweave-ai --features "ai-goap,llm_orchestrator"
cargo clippy -p astraweave-ai --features "ai-goap,llm_orchestrator" -- -D warnings

# ═══════════════════════════════════════════════════════════════
# PHASE 3: GOAP Integration
# ═══════════════════════════════════════════════════════════════
cargo bench -p astraweave-ai --bench ai_core_loop -- goap

# ═══════════════════════════════════════════════════════════════
# PHASE 4: hello_companion Integration
# ═══════════════════════════════════════════════════════════════
cargo check -p hello_companion --features "llm,ollama,ai-goap"
cargo run -p hello_companion --release --features "llm,ollama,ai-goap" -- --arbiter

# ═══════════════════════════════════════════════════════════════
# PHASE 5: Testing & Validation
# ═══════════════════════════════════════════════════════════════
cargo test -p astraweave-ai --test arbiter_tests --features "ai-goap,llm_orchestrator"
cargo test -p astraweave-ai --features "ai-goap,llm_orchestrator" -- --nocapture

# ═══════════════════════════════════════════════════════════════
# PHASE 6: Benchmarking & Performance
# ═══════════════════════════════════════════════════════════════
cargo bench -p astraweave-ai --bench arbiter_bench --features "ai-goap,llm_orchestrator"

# ═══════════════════════════════════════════════════════════════
# PHASE 7: Documentation & Polish
# ═══════════════════════════════════════════════════════════════
cargo doc -p astraweave-ai --features "ai-goap,llm_orchestrator" --no-deps
cargo test -p astraweave-ai --features "ai-goap,llm_orchestrator"
cargo run -p hello_companion --release --features "llm,ollama,ai-goap" -- --arbiter
```

---

## Expected Console Output (hello_companion --arbiter)

```
╔════════════════════════════════════════════════════════════╗
║   AstraWeave AI Companion Demo - Arbiter Mode             ║
╚════════════════════════════════════════════════════════════╝

🎯 AI Mode: Arbiter (GOAP + Hermes 2 Pro Hybrid)
⚡ GOAP Latency: 5-30 µs (instant tactical control)
🧠 Hermes Latency: 13-21s (async strategic planning)

[AIArbiter] Starting in GOAP mode
[AIArbiter] GOAP providing instant action
✅ Executed: MoveTo(10, 10) -> Success
[AIArbiter] GOAP providing instant action
✅ Executed: TakeCover -> Success
[AIArbiter] Requesting new LLM plan
[AIArbiter] GOAP providing instant action
✅ Executed: Approach(target_id=3, distance=5.0) -> Success
... (15-20 GOAP actions during Hermes planning)
[AIArbiter] Hermes plan ready (3 steps), taking control: plan-1729015234
[AIArbiter] Executing step 1: MoveTo(15, 15)
✅ Executed: MoveTo(15, 15) -> Success
[AIArbiter] Executing step 2: ThrowSmoke(12, 12)
✅ Executed: ThrowSmoke(12, 12) -> Success
[AIArbiter] Executing step 3: Attack(target_id=3)
✅ Executed: Attack(target_id=3) -> Success
[AIArbiter] LLM plan exhausted, GOAP resuming control
[AIArbiter] GOAP providing instant action
✅ Executed: Retreat(target_id=3, distance=10.0) -> Success
```

---

## Phase Completion Checklist

**Phase 1: AsyncTask & LlmExecutor**
- [ ] `astraweave-ai/src/async_task.rs` created (~100 LOC)
- [ ] `astraweave-ai/src/llm_executor.rs` created (~100 LOC)
- [ ] Unit tests passing (3+ tests)
- [ ] Zero compilation warnings
- [ ] Documentation comments added

**Phase 2: AIArbiter Core**
- [ ] `astraweave-ai/src/ai_arbiter.rs` created (~400 LOC)
- [ ] `AIControlMode` enum defined
- [ ] `update()` method implemented
- [ ] Mode transitions implemented
- [ ] Zero compilation warnings

**Phase 3: GOAP Integration**
- [ ] `GoapOrchestrator::next_action()` added
- [ ] AIArbiter uses fast path
- [ ] Benchmark confirms <100 µs
- [ ] No performance regression

**Phase 4: hello_companion Integration**
- [ ] `--arbiter` CLI flag added
- [ ] `create_arbiter()` function implemented
- [ ] Game loop updated
- [ ] Debug logging added
- [ ] Demo runs successfully

**Phase 5: Testing & Validation**
- [ ] `arbiter_tests.rs` created (~300 LOC)
- [ ] 5+ unit tests passing
- [ ] Mock helpers created
- [ ] No test flakiness (10 runs)
- [ ] Tests run fast (<5s)

**Phase 6: Benchmarking & Performance**
- [ ] `arbiter_bench.rs` created (~150 LOC)
- [ ] 4+ benchmarks implemented
- [ ] All performance targets met
- [ ] Benchmark results documented

**Phase 7: Documentation & Polish**
- [ ] `PHASE_7_ARBITER_IMPLEMENTATION.md` created (~8,000 words)
- [ ] `ARBITER_QUICK_REFERENCE.md` created (~500 words)
- [ ] README updates complete
- [ ] Copilot instructions updated
- [ ] All validation commands succeed

---

## Next Steps After Completion

**Immediate (Phase 8)**:
- Integrate arbiter into `unified_showcase` demo
- Add arbiter metrics to Phase 7 telemetry system
- Update benchmark comparison tables

**Short-Term (Phase 9)**:
- State-based replanning (replan on enemy phase change)
- LLM plan caching (semantic similarity matching)
- Multi-agent coordination (arbiter per companion)

**Long-Term (Phase 10)**:
- Streaming LLM responses (partial plan execution)
- Hybrid GOAP+LLM planning (LLM suggests goals, GOAP executes)
- Advanced plan validation (predictive state checking)

---

**END OF ROADMAP**

**Status**: Ready to begin Phase 1  
**Next Action**: Create `astraweave-ai/src/async_task.rs`  
**Estimated Time to First Working Demo**: 6-8 hours

