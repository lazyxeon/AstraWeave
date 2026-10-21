# AI Arbiter System

> **Status**: Production Ready  
> **Crate**: `astraweave-ai` (requires `llm_orchestrator` feature)  
> **Documentation**: See also [Complete Implementation Guide](../../archive/completion_reports/ARBITER_IMPLEMENTATION.md)

The AIArbiter is a hybrid AI control system that combines instant tactical decisions (GOAP) with deep strategic reasoning (LLM), achieving **zero user-facing latency** while maintaining LLM-level intelligence.

## The Problem

Traditional game AI faces a dilemma:

| Approach | Latency | Intelligence |
|----------|---------|--------------|
| **Fast AI** (GOAP, BT) | ~100 ns | Limited reasoning |
| **Smart AI** (LLM) | 13-21 seconds | Deep understanding |

Players either wait 20 seconds for smart AI or get immediate but shallow responses.

## The Solution

The arbiter provides **zero user-facing latency** by:

1. **Instant GOAP control** - Returns tactical actions in 101.7 ns
2. **Background LLM planning** - Generates strategic plans asynchronously
3. **Seamless transitions** - Switches to LLM plans when ready
4. **Non-blocking polling** - Checks LLM completion in 104.7 ns

---

## Performance

| Operation | Latency | Target | Speedup |
|-----------|---------|--------|---------|
| GOAP update | 101.7 ns | 100 µs | 982× |
| LLM polling | 575 ns | 50 µs | 86× |
| Mode transition | 221.9 ns | 10 µs | 45× |
| Full 3-step cycle | 313.7 ns | — | — |

### Scalability

| Agents | Overhead | Frame Budget | Status |
|--------|----------|--------------|--------|
| 1,000 | 101.7 µs | 0.6% | ✅ |
| 10,000 | 1.02 ms | 6.1% | ✅ |
| 50,000 | 5.09 ms | 30.5% | ⚠️ |

---

## Quick Start

```rust
use astraweave_ai::{AIArbiter, LlmExecutor, GoapOrchestrator, RuleOrchestrator};
use std::sync::Arc;

// Create arbiter
let llm_orch = Arc::new(LlmOrchestrator::new(/* config */));
let runtime = tokio::runtime::Handle::current();
let llm_executor = LlmExecutor::new(llm_orch, runtime);

let goap = Box::new(GoapOrchestrator::new());
let bt = Box::new(RuleOrchestrator);

let mut arbiter = AIArbiter::new(llm_executor, goap, bt);

// Game loop
loop {
    let snapshot = build_world_snapshot(/* ... */);
    let action = arbiter.update(&snapshot);  // 101.7 ns
    execute_action(action);
}
```

---

## Architecture

### Three-Tier Control System

```
┌─────────────────────────────────────────────────────┐
│                   AIArbiter                         │
│  (Orchestration Layer - 101.7 ns overhead)          │
└────┬────────────────────┬────────────────────┬──────┘
     │                    │                    │
     ▼                    ▼                    ▼
┌──────────┐      ┌──────────────┐     ┌──────────┐
│   GOAP   │      │  LLM (Ollama) │     │    BT    │
│ (3-5 ns) │      │ (13-21s async)│     │ Fallback │
└──────────┘      └──────────────┘     └──────────┘
```

### Mode State Machine

```
        ┌──────────────┐
        │     GOAP     │ ◄─────────┐
        │ (Instant AI) │           │
        └───────┬──────┘           │
                │                  │
                │ LLM ready        │ Plan exhausted
                │                  │
        ┌───────▼──────────┐       │
        │   ExecutingLLM   │───────┘
        │  (Step-by-step)  │
        └──────────────────┘
                │
                │ Empty plan
                ▼
        ┌──────────────┐
        │ BehaviorTree │
        │  (Fallback)  │
        └──────────────┘
```

---

## API Reference

### AIArbiter

```rust
pub struct AIArbiter { /* ... */ }

impl AIArbiter {
    /// Create new arbiter in GOAP mode
    pub fn new(
        llm_executor: LlmExecutor,
        goap: Box<dyn Orchestrator>,
        bt: Box<dyn Orchestrator>,
    ) -> Self;
    
    /// Set LLM request cooldown (default: 15s)
    pub fn with_llm_cooldown(self, cooldown: f32) -> Self;
    
    /// Main control loop - call every frame
    pub fn update(&mut self, snap: &WorldSnapshot) -> ActionStep;
    
    /// Get current mode
    pub fn mode(&self) -> AIControlMode;
    
    /// Check if LLM task is active
    pub fn is_llm_active(&self) -> bool;
    
    /// Get performance metrics
    pub fn metrics(&self) -> (
        usize,  // mode_transitions
        usize,  // llm_requests
        usize,  // llm_successes
        usize,  // llm_failures
        usize,  // goap_actions
        usize,  // llm_steps_executed
    );
}
```

### AIControlMode

```rust
pub enum AIControlMode {
    GOAP,                              // Fast tactical mode
    ExecutingLLM { step_index: usize }, // Executing LLM plan
    BehaviorTree,                      // Emergency fallback
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
    pub fn update(&mut self, snap: &WorldSnapshot) -> ActionStep {
        self.arbiter.update(snap)
    }
}
```

### Pattern 2: Shared LLM Executor

```rust
// Create once, clone for each agent
let base_executor = LlmExecutor::new(llm_orch, runtime);

let agents: Vec<AIAgent> = (0..100)
    .map(|_| AIAgent::new(base_executor.clone()))
    .collect();
```

### Pattern 3: Custom Cooldown

```rust
// Aggressive (more LLM requests)
let arbiter = AIArbiter::new(executor, goap, bt)
    .with_llm_cooldown(5.0);

// Passive (fewer LLM requests)
let arbiter = AIArbiter::new(executor, goap, bt)
    .with_llm_cooldown(30.0);
```

### Pattern 4: Metrics Monitoring

```rust
let (transitions, requests, successes, failures, goap_actions, llm_steps) = 
    arbiter.metrics();

let success_rate = 100.0 * successes as f64 / requests as f64;
if success_rate < 50.0 {
    warn!("LLM success rate low: {:.1}%", success_rate);
}
```

### Pattern 5: Mode-Specific Logic

```rust
match arbiter.mode() {
    AIControlMode::GOAP => {
        ui.show_status("Tactical Mode");
    }
    AIControlMode::ExecutingLLM { step_index } => {
        ui.show_status(&format!("Strategic Step {}", step_index));
        ui.show_indicator("LLM Active");
    }
    AIControlMode::BehaviorTree => {
        ui.show_warning("Fallback Mode");
    }
}
```

---

## Cooldown Configuration

The LLM cooldown controls how frequently the arbiter requests new strategic plans:

| Cooldown | Use Case |
|----------|----------|
| 5s | Aggressive - Frequent strategic updates |
| 15s | Default - Balanced performance |
| 30s | Passive - Reduce LLM costs |
| 0s | Immediate - Testing only |

```rust
let arbiter = AIArbiter::new(executor, goap, bt)
    .with_llm_cooldown(15.0);  // Default
```

---

## Troubleshooting

### LLM Never Completes

**Symptoms**: `is_llm_active()` always true, never transitions to ExecutingLLM

**Causes**:
1. Ollama not running
2. Model not loaded
3. Network issues

**Fix**:
```bash
# Verify Ollama is running
ollama list

# Test model directly (runtime default is phi3:medium; OLLAMA_MODEL overrides it)
ollama run phi3:medium
```

### High Failure Rate

**Symptoms**: `llm_failures` > 50% of requests

**Causes**:
1. Model quality issues
2. Bad prompts
3. Timeout too short

**Fix**:
```rust
// Increase cooldown to reduce impact
let arbiter = AIArbiter::new(executor, goap, bt)
    .with_llm_cooldown(30.0);
```

### Stuck in ExecutingLLM

**Symptoms**: Same action repeated, `step_index` doesn't advance

**Causes**:
1. Plan has duplicate steps
2. Plan too long

**Fix**: Validate plan length before execution:
```rust
if plan.steps.len() > 50 {
    warn!("LLM plan too long: {} steps", plan.steps.len());
}
```

---

## Running the Demo

```bash
# GOAP-only mode
cargo run -p hello_companion --release

# Arbiter mode (GOAP + LLM; runtime default model phi3:medium via Ollama)
cargo run -p hello_companion --release --features llm_orchestrator -- --arbiter
```

**Expected output**:
```
Frame 0: MoveTo { x: 5, y: 5 } (GOAP)
Frame 1: TakeCover { position: Some((3, 2)) } (GOAP)
[INFO] LLM plan ready: 3 steps
Frame 3: MoveTo { x: 4, y: 0 } (ExecutingLLM[step 1])
Frame 4: TakeCover { position: Some((4, 1)) } (ExecutingLLM[step 2])
Frame 5: Attack { target: 1 } (ExecutingLLM[step 3])
[INFO] Plan exhausted, returning to GOAP
Frame 6: MoveTo { x: 5, y: 5 } (GOAP)
```

---

## See Also

- [Complete Implementation Guide](../../archive/completion_reports/ARBITER_IMPLEMENTATION.md) - 8,000+ word deep dive
- [Quick Reference](../../archive/completion_reports/ARBITER_QUICK_REFERENCE.md) - 5-minute API guide
- [AI Core Loop](./ai-core.md) - Perception-Reasoning-Planning-Action
- [GOAP System](./goap.md) - Goal-oriented action planning
- [Behavior Trees](./behavior-trees.md) - Behavior tree integration
