# AIArbiter Quick Reference

**5-minute guide to GOAP+Hermes Hybrid AI**

---

## TL;DR

```rust
// 1. Create arbiter (one-time setup)
let mut arbiter = AIArbiter::new(llm_executor, goap, bt);

// 2. Update every frame
let action = arbiter.update(&snapshot);  // 101.7 ns

// 3. Execute action
execute_action(action);
```

**Result**: Instant tactical AI (GOAP) + deep strategic planning (LLM) with zero user-facing latency.

---

## Quick Start

### Installation

```toml
[dependencies]
astraweave-ai = { version = "0.1", features = ["llm_orchestrator"] }
tokio = { version = "1", features = ["rt", "sync", "time"] }
```

### Basic Usage

```rust
use astraweave_ai::{AIArbiter, LlmExecutor, GoapOrchestrator, RuleOrchestrator};
use astraweave_llm::LlmOrchestrator;
use std::sync::Arc;

// Initialize LLM orchestrator (shared across agents)
let llm_orch = Arc::new(LlmOrchestrator::new(/* config */));
let runtime = tokio::runtime::Handle::current();
let llm_executor = LlmExecutor::new(llm_orch, runtime);

// Create orchestrators
let goap = Box::new(GoapOrchestrator::new());
let bt = Box::new(RuleOrchestrator);

// Create arbiter
let mut arbiter = AIArbiter::new(llm_executor, goap, bt);

// Game loop
loop {
    let snapshot = build_world_snapshot(/* ... */);
    let action = arbiter.update(&snapshot);  // 101.7 ns in GOAP mode
    execute_action(action);
}
```

---

## API Reference

### AIArbiter

**Constructor**:
```rust
pub fn new(
    llm_executor: LlmExecutor,
    goap: Box<dyn Orchestrator>,
    bt: Box<dyn Orchestrator>,
) -> Self
```

**Core Methods**:
```rust
// Main control loop (call every frame)
pub fn update(&mut self, snap: &WorldSnapshot) -> ActionStep

// Get current mode
pub fn mode(&self) -> AIControlMode

// Check if LLM is active
pub fn is_llm_active(&self) -> bool

// Get metrics
pub fn metrics(&self) -> (
    usize,  // mode_transitions
    usize,  // llm_requests
    usize,  // llm_successes
    usize,  // llm_failures
    usize,  // goap_actions
    usize,  // llm_steps_executed
)
```

**Configuration**:
```rust
// Set LLM request cooldown (default: 15s)
pub fn with_llm_cooldown(self, cooldown: f32) -> Self
```

**Testing Methods** (public for testing only):
```rust
// Manually inject LLM plan
pub fn transition_to_llm(&mut self, plan: PlanIntent)
```

### AIControlMode

```rust
pub enum AIControlMode {
    GOAP,                          // Fast tactical mode (101.7 ns)
    ExecutingLLM { step_index: usize },  // Executing LLM plan (575 ns)
    BehaviorTree,                  // Emergency fallback
}
```

**Display**:
```rust
println!("{}", AIControlMode::GOAP);  // "GOAP"
println!("{}", AIControlMode::ExecutingLLM { step_index: 5 });  // "ExecutingLLM[step 5]"
```

### LlmExecutor

```rust
pub struct LlmExecutor { /* ... */ }

impl LlmExecutor {
    // Create executor with async orchestrator
    pub fn new(
        orchestrator: Arc<dyn OrchestratorAsync>,
        runtime: tokio::runtime::Handle,
    ) -> Self
    
    // Spawn async LLM planning (non-blocking)
    pub fn generate_plan_async(&self, snap: WorldSnapshot) -> AsyncTask<PlanIntent>
}
```

---

## Common Patterns

### Pattern 1: Basic Agent

```rust
pub struct AIAgent {
    arbiter: AIArbiter,
}

impl AIAgent {
    pub fn new(llm_executor: LlmExecutor) -> Self {
        let goap = Box::new(GoapOrchestrator::new());
        let bt = Box::new(RuleOrchestrator);
        let arbiter = AIArbiter::new(llm_executor, goap, bt);
        Self { arbiter }
    }
    
    pub fn update(&mut self, snap: &WorldSnapshot) -> ActionStep {
        self.arbiter.update(snap)
    }
}
```

### Pattern 2: Shared LLM Executor

```rust
// Create once, clone for each agent
let llm_orch = Arc::new(LlmOrchestrator::new(/* ... */));
let runtime = tokio::runtime::Handle::current();
let base_executor = LlmExecutor::new(llm_orch, runtime);

// Create 100 agents with same LLM
let agents: Vec<AIAgent> = (0..100)
    .map(|_| AIAgent::new(base_executor.clone()))
    .collect();
```

### Pattern 3: Custom Cooldown

```rust
// Aggressive (check LLM every 5 seconds)
let arbiter = AIArbiter::new(executor, goap, bt)
    .with_llm_cooldown(5.0);

// Passive (check LLM every 30 seconds)
let arbiter = AIArbiter::new(executor, goap, bt)
    .with_llm_cooldown(30.0);

// Immediate (no cooldown, spam LLM every frame if no active request)
let arbiter = AIArbiter::new(executor, goap, bt)
    .with_llm_cooldown(0.0);
```

### Pattern 4: Metrics Monitoring

```rust
// In game loop
let (transitions, requests, successes, failures, goap_actions, llm_steps) = 
    arbiter.metrics();

if requests > 0 {
    let success_rate = 100.0 * successes as f64 / requests as f64;
    println!("LLM success rate: {:.1}%", success_rate);
    
    if success_rate < 50.0 {
        eprintln!("WARNING: LLM failing frequently!");
    }
}

// Log mode changes
if transitions > last_transitions {
    println!("Mode changed to: {}", arbiter.mode());
    last_transitions = transitions;
}
```

### Pattern 5: Mode-Specific Logic

```rust
match arbiter.mode() {
    AIControlMode::GOAP => {
        // Using fast tactical AI
        ui.show_status("Tactical Mode");
    }
    AIControlMode::ExecutingLLM { step_index } => {
        // Executing LLM plan
        ui.show_status(&format!("Strategic Mode (step {})", step_index));
        ui.show_indicator("LLM Active", Color::GREEN);
    }
    AIControlMode::BehaviorTree => {
        // Emergency fallback
        ui.show_warning("Fallback Mode");
    }
}
```

---

## Performance Guide

### Benchmarks (Typical Results)

| Operation | Latency | Notes |
|-----------|---------|-------|
| GOAP update | 101.7 ns | Instant tactical decision |
| ExecutingLLM update | 575 ns | Plan step execution |
| LLM poll | 104.7 ns | Non-blocking check |
| Mode transition | 221.9 ns | Switch GOAP ↔ ExecutingLLM |

### Capacity Analysis

**1,000 agents @ 60 FPS**:
- Cost: 1,000 × 101.7 ns = **101.7 µs** (0.6% of 16.67 ms frame)
- **Verdict**: Easily achievable

**10,000 agents @ 60 FPS**:
- Cost: 10,000 × 101.7 ns = **1.017 ms** (6.1% of frame)
- **Verdict**: Feasible with staggered updates

### Optimization Tips

**1. Stagger Agent Updates**:
```rust
// Update 1/10th of agents each frame (10-frame rotation)
let agents_this_frame = agents.iter_mut()
    .skip(frame_count % 10)
    .step_by(10);

for agent in agents_this_frame {
    agent.update(&snapshot);
}
```

**2. Adjust Cooldown Based on Workload**:
```rust
// High agent count: increase cooldown
let cooldown = if agent_count > 1000 { 30.0 } else { 15.0 };
let arbiter = AIArbiter::new(executor, goap, bt)
    .with_llm_cooldown(cooldown);
```

**3. Profile with Tracy**:
```rust
#[cfg(feature = "profiling")]
astraweave_profiling::span!("AI::Arbiter::update");
```

---

## Troubleshooting

### Problem: LLM Never Completes

**Symptoms**:
- `is_llm_active()` always returns true
- Never transitions to ExecutingLLM mode
- Metrics show `llm_requests > 0` but `llm_successes = 0`

**Causes**:
1. LLM model not loaded (Ollama not running)
2. LLM request timeout (default: 60s)
3. LLM parsing errors (invalid JSON)

**Solutions**:
```bash
# Check Ollama is running
ollama list

# Test LLM directly
ollama run adrienbrault/nous-hermes2pro:Q4_K_M

# Check logs for errors
RUST_LOG=debug cargo run
```

### Problem: High LLM Failure Rate

**Symptoms**:
- Metrics show `llm_failures` > 50% of `llm_requests`
- Frequent mode transitions (GOAP → ExecutingLLM → GOAP)

**Causes**:
1. LLM model quality (try larger model)
2. LLM prompt issues (bad few-shot examples)
3. Network issues (cloud LLM)

**Solutions**:
```rust
// 1. Increase cooldown (reduce failure rate impact)
let arbiter = AIArbiter::new(executor, goap, bt)
    .with_llm_cooldown(30.0);

// 2. Check failure reasons in logs
if let Err(e) = llm_result {
    eprintln!("LLM failure: {:?}", e);
}

// 3. Use better model
let llm_orch = LlmOrchestrator::new_with_model(
    "adrienbrault/nous-hermes2pro:Q8_0"  // Larger, more accurate
);
```

### Problem: Excessive LLM Requests

**Symptoms**:
- Metrics show `llm_requests` growing very fast
- LLM API costs increasing
- Network bandwidth high

**Causes**:
1. Cooldown too low (or 0)
2. Many agents with independent arbiters

**Solutions**:
```rust
// 1. Increase cooldown
let arbiter = AIArbiter::new(executor, goap, bt)
    .with_llm_cooldown(20.0);  // Minimum 20s between requests

// 2. Share LLM plans across agents
struct SharedLlmPlanner {
    last_plan: Arc<Mutex<Option<PlanIntent>>>,
    last_request_time: f32,
}

// 3. Monitor request rate
if arbiter.metrics().1 > MAX_REQUESTS_PER_MINUTE {
    eprintln!("WARNING: LLM request rate too high!");
}
```

### Problem: Arbiter "Stuck" in ExecutingLLM

**Symptoms**:
- Agent executes same action repeatedly
- Never returns to GOAP mode
- `step_index` doesn't advance

**Causes**:
1. Plan has duplicate steps
2. Step execution logic broken
3. Plan too long (100+ steps)

**Solutions**:
```rust
// 1. Validate plan length
if plan.steps.len() > 50 {
    eprintln!("WARNING: LLM plan too long ({} steps)", plan.steps.len());
}

// 2. Add plan timeout
struct ArbiterWithTimeout {
    arbiter: AIArbiter,
    plan_start_time: f32,
    plan_timeout: f32,  // e.g., 30 seconds
}

impl ArbiterWithTimeout {
    fn update(&mut self, snap: &WorldSnapshot) -> ActionStep {
        // Check timeout
        if matches!(self.arbiter.mode(), AIControlMode::ExecutingLLM { .. }) {
            if snap.t - self.plan_start_time > self.plan_timeout {
                // Force back to GOAP
                eprintln!("Plan timeout, returning to GOAP");
                // (manually transition - requires making transition_to_goap public)
            }
        }
        
        self.arbiter.update(snap)
    }
}
```

---

## Examples

### hello_companion Demo

**Location**: `examples/hello_companion/`

**Run**:
```bash
# GOAP-only mode
cargo run -p hello_companion --release

# Arbiter mode (GOAP + Hermes 2 Pro)
cargo run -p hello_companion --release --features llm_orchestrator -- --arbiter
```

**Expected Output**:
```
[INFO] AIArbiter initialized in GOAP mode
Frame 0: MoveTo { x: 5, y: 5 } (GOAP)
Frame 1: TakeCover { position: Some((3, 2)) } (GOAP)
Frame 2: MoveTo { x: 5, y: 5 } (GOAP)
[INFO] LLM plan ready: 3 steps, transitioning to ExecutingLLM
Frame 3: MoveTo { x: 4, y: 0 } (ExecutingLLM[step 1])
Frame 4: TakeCover { position: Some((4, 1)) } (ExecutingLLM[step 2])
Frame 5: Attack { target: 1, stance: "crouched" } (ExecutingLLM[step 3])
[INFO] LLM plan exhausted, transitioning to GOAP
Frame 6: MoveTo { x: 5, y: 5 } (GOAP)
```

---

## Testing

### Run Tests

```bash
# Unit tests (AsyncTask, LlmExecutor, AIArbiter)
cargo test -p astraweave-ai --lib

# Integration tests (10 comprehensive scenarios)
cargo test -p astraweave-ai --test arbiter_tests --features llm_orchestrator

# Benchmarks (5 performance tests)
cargo bench -p astraweave-ai --bench arbiter_bench --features llm_orchestrator
```

### Test Your Integration

```rust
#[test]
fn test_arbiter_integration() {
    // Create minimal arbiter
    let llm_orch = Arc::new(MockLlmOrch::new());  // Mock for testing
    let runtime = tokio::runtime::Handle::current();
    let llm_executor = LlmExecutor::new(llm_orch, runtime);
    let goap = Box::new(GoapOrchestrator::new());
    let bt = Box::new(RuleOrchestrator);
    let mut arbiter = AIArbiter::new(llm_executor, goap, bt);
    
    // Create test snapshot
    let snap = WorldSnapshot { /* ... */ };
    
    // Test basic update
    let action = arbiter.update(&snap);
    assert!(matches!(action, ActionStep::MoveTo { .. }));
    
    // Verify metrics
    let (_, _, _, _, goap_actions, _) = arbiter.metrics();
    assert_eq!(goap_actions, 1);
}
```

---

## Additional Resources

- **Full Documentation**: `docs/ARBITER_IMPLEMENTATION.md`
- **Phase Reports**: Root directory (`PHASE_7_ARBITER_PHASE_*_COMPLETE.md`)
- **Source Code**: `astraweave-ai/src/ai_arbiter.rs`
- **Tests**: `astraweave-ai/tests/arbiter_tests.rs`
- **Benchmarks**: `astraweave-ai/benches/arbiter_bench.rs`

---

## Key Takeaways

✅ **Performance**: 101.7 ns GOAP update (982× faster than target)  
✅ **Scalability**: 1,000 agents = 0.6% frame budget  
✅ **Zero Latency**: LLM runs in background, users never wait  
✅ **Production Ready**: 10/10 tests, 5/5 benchmarks passing

**Remember**: The arbiter sacrifices **100 nanoseconds** to gain **20 seconds of LLM reasoning**. That's a 200,000,000× time trade-off in your favor.

---

**Quick Reference Version**: 1.0.0  
**Date**: January 15, 2025  
**For**: AstraWeave AI-Native Gaming Engine
