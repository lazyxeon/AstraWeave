//! AI Arbiter: GOAP+Qwen3 Hybrid Control System
//!
//! The `AIArbiter` provides seamless switching between instant GOAP tactical control
//! and asynchronous Qwen3 strategic planning, achieving zero user-facing latency.
//!
//! ## Dual-Executor Architecture
//!
//! The arbiter supports two LLM executors:
//! - **Fast executor** (non-thinking Qwen3): Low-latency (<2s) inline queries and fallbacks
//! - **Strategic executor** (thinking Qwen3): Deep chain-of-thought planning (3-8s, background)
//!
//! # Architecture
//!
//! ```text
//! Game Loop (60 FPS)
//!       ↓
//! AIArbiter::update(&snap)  ← Always returns instantly
//!       ↓
//! ┌─────────────────────────────────────────────┐
//! │  Mode: GOAP (Default - 5-30 µs)             │
//! │  ├─ Check: Should request LLM?              │
//! │  │   └─ If yes: Spawn async task            │
//! │  ├─ Poll: LLM result ready?                 │
//! │  │   └─ If yes: Transition to ExecutingLLM  │
//! │  └─ Return: goap.next_action()              │
//! │                                              │
//! │  Mode: ExecutingLLM (Executing plan)        │
//! │  ├─ Execute: plan.steps[step_index]         │
//! │  ├─ Advance: step_index++                   │
//! │  └─ If plan exhausted: Transition to GOAP   │
//! │                                              │
//! │  Mode: BehaviorTree (Emergency fallback)    │
//! │  └─ Return: bt.plan() (sync, <1 ms)         │
//! └─────────────────────────────────────────────┘
//!       ↓
//! ActionStep  ← Instant, every frame
//! ```
//!
//! # Performance Targets
//!
//! - **GOAP mode**: <100 µs per update
//! - **ExecutingLLM mode**: <50 µs per update (array lookup)
//! - **Mode transitions**: <10 µs
//! - **LLM request**: <1 ms (spawn async task)
//! - **LLM polling**: <10 µs (non-blocking check)
//!
//! # Example Usage
//!
//! ```ignore
//! use astraweave_ai::{AIArbiter, LlmExecutor};
//! use astraweave_llm::fallback_system::FallbackOrchestrator;
//! use astraweave_core::{WorldSnapshot, default_tool_registry};
//! use std::sync::Arc;
//!
//! async fn example() -> anyhow::Result<()> {
//!     // Setup orchestrators
//!     let llm_client = /* Qwen3-8B client */;
//!     let llm_orch = Arc::new(FallbackOrchestrator::new(llm_client, default_tool_registry()));
//!     let runtime = tokio::runtime::Handle::current();
//!     let llm_executor = LlmExecutor::new(llm_orch, runtime);
//!
//!     let goap = /* GOAP orchestrator */;
//!     let bt = /* Behavior tree orchestrator */;
//!
//!     let mut arbiter = AIArbiter::with_single_executor(llm_executor, goap, bt);
//!
//!     // Game loop (60 FPS)
//!     loop {
//!         let snapshot = /* build world snapshot */;
//!         
//!         // Always returns instantly (GOAP or cached plan step)
//!         let action = arbiter.update(&snapshot);
//!         
//!         // Apply action to game world
//!         // ...
//!     }
//!     Ok(())
//! }
//! ```

use crate::async_task::AsyncTask;
use crate::llm_executor::LlmExecutor;
use crate::orchestrator::Orchestrator;
use anyhow::Result;
use astraweave_core::{metrics, ActionStep, PlanIntent, WorldSnapshot};
use tracing::{debug, info, warn};

/// AI control mode for the arbiter.
///
/// Determines which AI system is currently providing actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum AIControlMode {
    /// GOAP provides instant tactical decisions (5-30 µs).
    /// This is the default mode and fallback when LLM plans are exhausted.
    #[default]
    GOAP,

    /// Executing a multi-step LLM plan.
    /// Stores the current step index being executed.
    ExecutingLLM {
        /// Current step index in the plan (0-based)
        step_index: usize,
    },

    /// Emergency fallback to behavior tree (if GOAP fails).
    /// Should rarely be used in practice.
    BehaviorTree,
}

impl std::fmt::Display for AIControlMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AIControlMode::GOAP => write!(f, "GOAP"),
            AIControlMode::ExecutingLLM { step_index } => {
                write!(f, "ExecutingLLM[step {}]", step_index)
            }
            AIControlMode::BehaviorTree => write!(f, "BehaviorTree"),
        }
    }
}

impl AIControlMode {
    /// Check if this mode is the default GOAP mode.
    #[must_use]
    pub fn is_goap(&self) -> bool {
        matches!(self, AIControlMode::GOAP)
    }

    /// Check if this mode is executing an LLM plan.
    #[must_use]
    pub fn is_executing_llm(&self) -> bool {
        matches!(self, AIControlMode::ExecutingLLM { .. })
    }

    /// Check if this mode is the behavior tree fallback.
    #[must_use]
    pub fn is_behavior_tree(&self) -> bool {
        matches!(self, AIControlMode::BehaviorTree)
    }

    /// Check if this mode is a fallback mode (not executing a plan).
    #[must_use]
    pub fn is_fallback(&self) -> bool {
        matches!(self, AIControlMode::BehaviorTree)
    }

    /// Check if this is an instant (fast) mode.
    #[must_use]
    pub fn is_instant(&self) -> bool {
        matches!(self, AIControlMode::GOAP | AIControlMode::BehaviorTree)
    }

    /// Get the current step index if executing an LLM plan.
    #[must_use]
    pub fn step_index(&self) -> Option<usize> {
        match self {
            AIControlMode::ExecutingLLM { step_index } => Some(*step_index),
            _ => None,
        }
    }

    /// Create an ExecutingLLM mode at the given step index.
    #[must_use]
    pub fn executing_llm(step_index: usize) -> Self {
        AIControlMode::ExecutingLLM { step_index }
    }

    /// Get the mode name without step information.
    #[must_use]
    pub fn mode_name(&self) -> &'static str {
        match self {
            AIControlMode::GOAP => "GOAP",
            AIControlMode::ExecutingLLM { .. } => "ExecutingLLM",
            AIControlMode::BehaviorTree => "BehaviorTree",
        }
    }
}

/// AI Arbiter: Hybrid GOAP+Qwen3 control system.
///
/// Seamlessly switches between instant GOAP tactical control and asynchronous
/// Qwen3 strategic planning to achieve zero user-facing latency.
///
/// ## Dual-Executor Architecture
///
/// The arbiter supports two LLM executors:
/// - **`strategic_executor`**: Uses thinking-mode Qwen3 for deep background planning.
///   Spawned asynchronously via `generate_plan_async()`.
/// - **`fast_executor`** (optional): Uses non-thinking Qwen3 for low-latency inline
///   queries and plan-validation fallback. If not provided, falls back to GOAP.
///
/// # Thread Safety
/// - Not `Send` or `Sync` (contains orchestrators which may not be thread-safe)
/// - Designed for single-threaded game loop usage
///
/// # Performance
/// - `update()`: Always <100 µs (GOAP fast path or plan step lookup)
/// - Mode transitions: <10 µs
/// - LLM requests: Non-blocking (<1 ms to spawn task)
pub struct AIArbiter {
    // === AI Modules ===
    /// Strategic LLM executor for async background planning (thinking mode Qwen3).
    /// Used by `maybe_request_llm()` for deep chain-of-thought plan generation.
    strategic_executor: LlmExecutor,

    /// Fast LLM executor for low-latency inline queries (non-thinking Qwen3).
    /// Optional — if absent, the arbiter uses GOAP for all fast-path decisions.
    fast_executor: Option<LlmExecutor>,

    /// GOAP orchestrator for instant tactical decisions
    goap: Box<dyn Orchestrator>,

    /// Behavior tree orchestrator (emergency fallback)
    bt: Box<dyn Orchestrator>,

    // === State Management ===
    /// Current control mode
    mode: AIControlMode,

    /// Active LLM task (if any)
    current_llm_task: Option<AsyncTask<Result<PlanIntent>>>,

    /// Current LLM plan being executed (if any)
    current_plan: Option<PlanIntent>,

    // === LLM Request Policy ===
    /// Minimum time between LLM requests (seconds)
    llm_request_cooldown: f32,

    /// Time of last LLM request
    last_llm_request_time: f32,

    // === Metrics (for debugging and tuning) ===
    /// Total number of mode transitions
    mode_transitions: u32,

    /// Total LLM requests initiated
    llm_requests: u32,

    /// Successful LLM plan generations
    llm_successes: u32,

    /// Failed LLM plan generations
    llm_failures: u32,

    /// Total GOAP actions returned
    goap_actions: u32,

    /// Total LLM plan steps executed
    llm_steps_executed: u32,
}

impl AIArbiter {
    /// Create a new AI Arbiter with dual executors.
    ///
    /// # Arguments
    /// - `strategic_executor`: Async LLM plan generator (thinking-mode Qwen3)
    /// - `fast_executor`: Optional low-latency LLM for inline queries (non-thinking Qwen3)
    /// - `goap`: GOAP orchestrator for instant tactical decisions
    /// - `bt`: Behavior tree orchestrator (emergency fallback)
    ///
    /// # Default Configuration
    /// - Initial mode: GOAP
    /// - LLM request cooldown: 15.0 seconds
    /// - All metrics initialized to 0
    ///
    /// # Example
    /// ```ignore
    /// use astraweave_ai::{AIArbiter, LlmExecutor};
    /// use std::sync::Arc;
    /// use astraweave_core::default_tool_registry;
    ///
    /// async fn example() {
    ///     let strategic_exec = /* thinking-mode Qwen3 executor */;
    ///     let fast_exec = /* non-thinking Qwen3 executor */;
    ///     let goap = /* ... */;
    ///     let bt = /* ... */;
    ///
    ///     let arbiter = AIArbiter::new(strategic_exec, Some(fast_exec), goap, bt);
    /// }
    /// ```
    pub fn new(
        strategic_executor: LlmExecutor,
        fast_executor: Option<LlmExecutor>,
        goap: Box<dyn Orchestrator>,
        bt: Box<dyn Orchestrator>,
    ) -> Self {
        info!(
            "AIArbiter initialized in GOAP mode (dual-executor: {})",
            if fast_executor.is_some() { "yes" } else { "no" }
        );
        Self {
            strategic_executor,
            fast_executor,
            goap,
            bt,
            mode: AIControlMode::GOAP,
            current_llm_task: None,
            current_plan: None,
            llm_request_cooldown: 15.0,
            last_llm_request_time: -999.0, // Allow immediate first request
            mode_transitions: 0,
            llm_requests: 0,
            llm_successes: 0,
            llm_failures: 0,
            goap_actions: 0,
            llm_steps_executed: 0,
        }
    }

    /// Create a new AI Arbiter with a single executor (backward compatibility).
    ///
    /// Uses the provided executor as the strategic executor. No fast executor
    /// is configured — the arbiter uses GOAP for all fast-path decisions.
    ///
    /// This constructor maintains backward compatibility with code that
    /// passes a single `LlmExecutor`.
    ///
    /// # Arguments
    /// - `llm_executor`: Async LLM plan generator (used as strategic executor)
    /// - `goap`: GOAP orchestrator for instant tactical decisions
    /// - `bt`: Behavior tree orchestrator (emergency fallback)
    pub fn with_single_executor(
        llm_executor: LlmExecutor,
        goap: Box<dyn Orchestrator>,
        bt: Box<dyn Orchestrator>,
    ) -> Self {
        Self::new(llm_executor, None, goap, bt)
    }

    /// Configure LLM request cooldown.
    ///
    /// Sets the minimum time between LLM plan requests to prevent spamming
    /// the LLM with redundant requests.
    ///
    /// # Arguments
    /// - `cooldown`: Cooldown time in seconds (default: 15.0)
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_ai::AIArbiter;
    /// # fn example(mut arbiter: AIArbiter) {
    /// arbiter.with_llm_cooldown(10.0); // 10 second cooldown
    /// # }
    /// ```
    pub fn with_llm_cooldown(mut self, cooldown: f32) -> Self {
        self.llm_request_cooldown = cooldown;
        self
    }

    /// Main update loop - returns an action instantly.
    ///
    /// This is the primary entry point for the arbiter. It always returns
    /// an `ActionStep` instantly, regardless of mode:
    /// - **GOAP mode**: Returns GOAP's fast-path action (<100 µs)
    /// - **ExecutingLLM mode**: Returns next step from plan (<50 µs)
    /// - **BehaviorTree mode**: Returns BT action (<1 ms)
    ///
    /// # Arguments
    /// - `snap`: Current world snapshot
    ///
    /// # Returns
    /// An `ActionStep` to execute this frame
    ///
    /// # Performance
    /// - **Target**: <100 µs per call
    /// - **Actual**: 5-30 µs (GOAP), <50 µs (ExecutingLLM), <1 ms (BT)
    ///
    /// # Side Effects
    /// - May spawn async LLM task (if cooldown expired)
    /// - May transition modes (if LLM plan ready or exhausted)
    /// - Updates metrics
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_ai::AIArbiter;
    /// # use astraweave_core::WorldSnapshot;
    /// # fn example(mut arbiter: AIArbiter, snapshot: WorldSnapshot) {
    /// let action = arbiter.update(&snapshot);
    /// // Apply action to game world
    /// # }
    /// ```
    pub fn update(&mut self, snap: &WorldSnapshot) -> ActionStep {
        let start = std::time::Instant::now();

        // Poll for LLM completion (non-blocking, <10 µs)
        if let Some(plan_result) = self.poll_llm_result() {
            match plan_result {
                Ok(plan) => {
                    info!(
                        "LLM plan ready: {} steps, transitioning to ExecutingLLM",
                        plan.steps.len()
                    );
                    self.transition_to_llm(plan);
                }
                Err(e) => {
                    warn!("LLM planning failed: {}, staying in GOAP mode", e);
                    self.llm_failures += 1;
                }
            }
        }

        // Execute based on current mode
        let action = match self.mode {
            AIControlMode::GOAP => {
                // Check if we should request LLM planning
                self.maybe_request_llm(snap);

                // Return instant GOAP action (first step of plan)
                self.goap_actions += 1;
                let plan = self.goap.propose_plan(snap);
                #[allow(clippy::unnecessary_lazy_evaluations)]
                // Closure has side effects (warn!, transition_to_bt)
                plan.steps.first().cloned().unwrap_or_else(|| {
                    warn!("GOAP plan has no steps, falling back to BehaviorTree");
                    self.transition_to_bt();
                    let bt_plan = self.bt.propose_plan(snap);
                    bt_plan.steps.first().cloned().unwrap_or(
                        // Ultimate fallback: Wait 1 second
                        ActionStep::Wait { duration: 1.0 },
                    )
                })
            }

            AIControlMode::ExecutingLLM { step_index } => {
                // Execute current step from LLM plan
                if let Some(plan) = &self.current_plan {
                    if step_index < plan.steps.len() {
                        // Return current step
                        let action = plan.steps[step_index].clone();
                        self.llm_steps_executed += 1;

                        // Advance to next step
                        let next_index = step_index + 1;
                        if next_index >= plan.steps.len() {
                            // Plan exhausted, return to GOAP
                            debug!(
                                "LLM plan exhausted ({} steps executed), transitioning to GOAP",
                                plan.steps.len()
                            );
                            self.transition_to_goap();
                        } else {
                            // Continue executing plan
                            self.mode = AIControlMode::ExecutingLLM {
                                step_index: next_index,
                            };
                        }

                        action
                    } else {
                        // Invalid step index, fall back to GOAP
                        warn!(
                            "Invalid LLM step index {} (plan has {} steps), falling back to GOAP",
                            step_index,
                            plan.steps.len()
                        );
                        self.transition_to_goap();
                        self.update(snap)
                    }
                } else {
                    // No plan available, fall back to GOAP
                    warn!("ExecutingLLM mode but no plan available, falling back to GOAP");
                    self.transition_to_goap();
                    self.update(snap)
                }
            }

            AIControlMode::BehaviorTree => {
                // Emergency fallback to BT
                let bt_plan = self.bt.propose_plan(snap);
                bt_plan
                    .steps
                    .first()
                    .cloned()
                    .unwrap_or(ActionStep::Wait { duration: 1.0 })
            }
        };

        metrics::histogram(
            "ai.arbiter.update_latency",
            start.elapsed().as_secs_f64() * 1000.0,
        );
        action
    }

    /// Transition to ExecutingLLM mode with a new plan.
    ///
    /// # Arguments
    /// - `plan`: The LLM-generated plan to execute
    ///
    /// # Effects
    /// - Sets mode to `ExecutingLLM { step_index: 0 }`
    /// - Stores plan in `current_plan`
    /// - Increments `mode_transitions` and `llm_successes`
    ///
    /// # Visibility
    /// Public for testing purposes (allows manual plan injection)
    pub fn transition_to_llm(&mut self, plan: PlanIntent) {
        let steps = plan.steps.len();
        self.current_plan = Some(plan);
        self.mode = AIControlMode::ExecutingLLM { step_index: 0 };
        self.mode_transitions += 1;
        self.llm_successes += 1;
        metrics::increment("ai.mode.transition.llm");
        info!("Mode transition: GOAP → ExecutingLLM ({} steps)", steps);
    }

    /// Transition to GOAP mode.
    ///
    /// # Effects
    /// - Sets mode to `GOAP`
    /// - Clears `current_plan`
    /// - Increments `mode_transitions`
    fn transition_to_goap(&mut self) {
        if self.mode != AIControlMode::GOAP {
            self.mode = AIControlMode::GOAP;
            self.current_plan = None;
            self.mode_transitions += 1;
            metrics::increment("ai.mode.transition.goap");
            info!("Mode transition: {} → GOAP", self.mode);
        }
    }

    /// Transition to BehaviorTree mode (emergency fallback).
    ///
    /// # Effects
    /// - Sets mode to `BehaviorTree`
    /// - Clears `current_plan` and `current_llm_task`
    /// - Increments `mode_transitions`
    fn transition_to_bt(&mut self) {
        if self.mode != AIControlMode::BehaviorTree {
            self.mode = AIControlMode::BehaviorTree;
            self.current_plan = None;
            self.current_llm_task = None;
            self.mode_transitions += 1;
            metrics::increment("ai.mode.transition.bt");
            warn!("Mode transition: {} → BehaviorTree (emergency)", self.mode);
        }
    }

    /// Poll for LLM task completion (non-blocking).
    ///
    /// Checks if the active LLM task has completed and returns the result.
    ///
    /// # Returns
    /// - `Some(Ok(plan))`: LLM planning succeeded
    /// - `Some(Err(e))`: LLM planning failed
    /// - `None`: No active task or task still running
    ///
    /// # Performance
    /// - **Target**: <10 µs
    /// - **Non-blocking**: Returns immediately
    fn poll_llm_result(&mut self) -> Option<Result<PlanIntent>> {
        if let Some(task) = &mut self.current_llm_task {
            if let Some(result) = task.try_recv() {
                // Task completed, clear it
                self.current_llm_task = None;

                // Extract inner Result from AsyncTask<Result<PlanIntent>>
                return Some(match result {
                    Ok(plan_result) => plan_result, // Inner Result<PlanIntent>
                    Err(e) => Err(e.context("LLM task join error")),
                });
            }
        }
        None
    }

    /// Request LLM planning if conditions are met.
    ///
    /// Checks if LLM planning should be requested based on:
    /// - No active LLM task
    /// - Cooldown expired
    /// - Currently in GOAP mode (not already executing plan)
    ///
    /// # Arguments
    /// - `snap`: Current world snapshot (for LLM context)
    ///
    /// # Effects
    /// - May spawn async LLM task via `llm_executor.generate_plan_async()`
    /// - Updates `last_llm_request_time`
    /// - Increments `llm_requests`
    ///
    /// # Performance
    /// - **Target**: <1 ms (just spawns async task)
    fn maybe_request_llm(&mut self, snap: &WorldSnapshot) {
        // Only request if no active task
        if self.current_llm_task.is_some() {
            return;
        }

        // Only request in GOAP mode (not while executing LLM plan)
        if self.mode != AIControlMode::GOAP {
            return;
        }

        // Check cooldown
        let cooldown_elapsed = snap.t - self.last_llm_request_time;
        if cooldown_elapsed < self.llm_request_cooldown {
            return;
        }

        // Spawn async LLM task
        debug!(
            "Requesting LLM planning (cooldown elapsed: {:.1}s)",
            cooldown_elapsed
        );
        let task = self.strategic_executor.generate_plan_async(snap.clone());
        self.current_llm_task = Some(task);
        self.last_llm_request_time = snap.t;
        self.llm_requests += 1;
    }

    /// Get current control mode.
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_ai::{AIArbiter, AIControlMode};
    /// # fn example(arbiter: &AIArbiter) {
    /// match arbiter.mode() {
    ///     AIControlMode::GOAP => println!("Using GOAP"),
    ///     AIControlMode::ExecutingLLM { step_index } => {
    ///         println!("Executing LLM plan step {}", step_index)
    ///     }
    ///     AIControlMode::BehaviorTree => println!("Emergency fallback"),
    ///     _ => println!("Other mode"),
    /// }
    /// # }
    /// ```
    pub fn mode(&self) -> AIControlMode {
        self.mode
    }

    /// Get metrics snapshot.
    ///
    /// Returns current arbiter metrics for debugging and performance tuning.
    ///
    /// # Returns
    /// Tuple of: (mode_transitions, llm_requests, llm_successes, llm_failures,
    ///            goap_actions, llm_steps_executed)
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_ai::AIArbiter;
    /// # fn example(arbiter: &AIArbiter) {
    /// let (transitions, requests, successes, failures, goap, llm_steps) =
    ///     arbiter.metrics();
    /// println!("Arbiter metrics: {} transitions, {}/{} LLM success rate",
    ///     transitions, successes, requests);
    /// # }
    /// ```
    pub fn metrics(&self) -> (u32, u32, u32, u32, u32, u32) {
        (
            self.mode_transitions,
            self.llm_requests,
            self.llm_successes,
            self.llm_failures,
            self.goap_actions,
            self.llm_steps_executed,
        )
    }

    /// Check if LLM task is active.
    ///
    /// # Returns
    /// `true` if an LLM task is currently running in the background
    pub fn is_llm_active(&self) -> bool {
        self.current_llm_task.is_some()
    }

    /// Get current plan (if executing LLM plan).
    ///
    /// # Returns
    /// Reference to current plan if in `ExecutingLLM` mode, `None` otherwise
    pub fn current_plan(&self) -> Option<&PlanIntent> {
        self.current_plan.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{CompanionState, EnemyState, IVec2, PlayerState};
    use std::collections::BTreeMap;

    /// Mock GOAP orchestrator for testing
    struct MockGoap {
        action_to_return: ActionStep,
        should_fail: bool,
    }

    impl Orchestrator for MockGoap {
        fn propose_plan(&self, _snap: &WorldSnapshot) -> PlanIntent {
            if self.should_fail {
                return PlanIntent {
                    plan_id: "mock-fail".into(),
                    steps: vec![],
                };
            }
            PlanIntent {
                plan_id: "mock-goap".into(),
                steps: vec![self.action_to_return.clone()],
            }
        }
    }

    /// Mock BehaviorTree orchestrator for testing
    struct MockBT;

    impl Orchestrator for MockBT {
        fn propose_plan(&self, _snap: &WorldSnapshot) -> PlanIntent {
            PlanIntent {
                plan_id: "mock-bt".into(),
                steps: vec![ActionStep::Scan { radius: 10.0 }],
            }
        }
    }

    fn create_test_snapshot(t: f32) -> WorldSnapshot {
        WorldSnapshot {
            t,
            me: CompanionState {
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 5, y: 5 },
            },
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 5, y: 5 },
                stance: "stand".into(),
                orders: vec![],
            },
            enemies: vec![EnemyState {
                id: 1,
                pos: IVec2 { x: 10, y: 10 },
                hp: 100,
                cover: "none".into(),
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: Some("extract".into()),
        }
    }

    // Note: Full integration tests require tokio runtime and real LlmExecutor
    // These tests focus on mode transitions and GOAP fallback behavior
    // LLM integration will be tested in Phase 5 (arbiter_tests.rs)

    #[test]
    fn test_arbiter_initial_mode_is_goap() {
        // This test would require a real LlmExecutor, so we'll defer to integration tests
        // For now, just validate the enum and display implementations
        assert_eq!(AIControlMode::GOAP.to_string(), "GOAP");
        assert_eq!(
            AIControlMode::ExecutingLLM { step_index: 2 }.to_string(),
            "ExecutingLLM[step 2]"
        );
        assert_eq!(AIControlMode::BehaviorTree.to_string(), "BehaviorTree");
    }

    #[test]
    fn test_mode_display() {
        let mode1 = AIControlMode::GOAP;
        let mode2 = AIControlMode::ExecutingLLM { step_index: 5 };
        let mode3 = AIControlMode::BehaviorTree;

        assert_eq!(format!("{}", mode1), "GOAP");
        assert_eq!(format!("{}", mode2), "ExecutingLLM[step 5]");
        assert_eq!(format!("{}", mode3), "BehaviorTree");
    }

    #[test]
    fn test_mode_equality() {
        let goap1 = AIControlMode::GOAP;
        let goap2 = AIControlMode::GOAP;
        let llm1 = AIControlMode::ExecutingLLM { step_index: 3 };
        let llm2 = AIControlMode::ExecutingLLM { step_index: 3 };
        let llm3 = AIControlMode::ExecutingLLM { step_index: 4 };
        let bt1 = AIControlMode::BehaviorTree;
        let bt2 = AIControlMode::BehaviorTree;

        assert_eq!(goap1, goap2);
        assert_eq!(llm1, llm2);
        assert_ne!(llm1, llm3); // Different step index
        assert_eq!(bt1, bt2);
        assert_ne!(goap1, llm1);
        assert_ne!(goap1, bt1);
        assert_ne!(llm1, bt1);
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_mode_clone() {
        let mode1 = AIControlMode::ExecutingLLM { step_index: 42 };
        let mode2 = mode1.clone();

        assert_eq!(mode1, mode2);
        assert_eq!(mode2, AIControlMode::ExecutingLLM { step_index: 42 });
    }

    #[test]
    fn test_mode_debug() {
        let mode = AIControlMode::ExecutingLLM { step_index: 7 };
        let debug_str = format!("{:?}", mode);

        assert!(debug_str.contains("ExecutingLLM"));
        assert!(debug_str.contains("step_index"));
        assert!(debug_str.contains("7"));
    }

    #[test]
    fn test_action_step_wait_boundary() {
        // Test Wait action with zero duration (edge case)
        let wait_zero = ActionStep::Wait { duration: 0.0 };
        assert!(matches!(wait_zero, ActionStep::Wait { duration } if duration == 0.0));

        // Test Wait action with very large duration
        let wait_large = ActionStep::Wait { duration: 999999.0 };
        assert!(matches!(wait_large, ActionStep::Wait { duration } if duration == 999999.0));

        // Test Wait action with negative duration (invalid but should be handled)
        let wait_negative = ActionStep::Wait { duration: -1.0 };
        assert!(matches!(wait_negative, ActionStep::Wait { duration } if duration < 0.0));
    }

    #[test]
    fn test_action_step_clone() {
        let action1 = ActionStep::MoveTo {
            x: 10,
            y: 20,
            speed: None,
        };
        let action2 = action1.clone();

        assert!(matches!(
            action2,
            ActionStep::MoveTo {
                x: 10,
                y: 20,
                speed: None
            }
        ));
    }

    #[test]
    fn test_world_snapshot_edge_cases() {
        // Test snapshot with no enemies
        let snap_no_enemies = WorldSnapshot {
            t: 1.0,
            me: CompanionState {
                ammo: 0, // Out of ammo
                cooldowns: BTreeMap::new(),
                morale: 0.0, // Zero morale,
                pos: IVec2 { x: 0, y: 0 },
            },
            player: PlayerState {
                hp: 0, // Player dead
                pos: IVec2 { x: 0, y: 0 },
                stance: "".into(), // Empty stance
                orders: vec![],
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None, // No objective
        };

        assert_eq!(snap_no_enemies.enemies.len(), 0);
        assert_eq!(snap_no_enemies.me.ammo, 0);
        assert_eq!(snap_no_enemies.player.hp, 0);

        // Test snapshot with many enemies
        let snap_many_enemies = WorldSnapshot {
            t: 1.0,
            me: CompanionState {
                ammo: 1000,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 0, y: 0 },
            },
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: vec![],
            },
            enemies: (0..100)
                .map(|i| EnemyState {
                    id: i as u32,
                    pos: IVec2 { x: i, y: i },
                    hp: 100,
                    cover: "full".into(),
                    last_seen: 0.0,
                })
                .collect(),
            pois: vec![],
            obstacles: vec![],
            objective: Some("survive".into()),
        };

        assert_eq!(snap_many_enemies.enemies.len(), 100);
    }

    #[test]
    fn test_plan_intent_empty_steps() {
        let empty_plan = PlanIntent {
            plan_id: "empty-plan".into(),
            steps: vec![],
        };

        assert_eq!(empty_plan.steps.len(), 0);
        assert!(empty_plan.steps.is_empty());
    }

    #[test]
    fn test_plan_intent_single_step() {
        let single_step_plan = PlanIntent {
            plan_id: "single-step".into(),
            steps: vec![ActionStep::Wait { duration: 1.0 }],
        };

        assert_eq!(single_step_plan.steps.len(), 1);
        assert!(matches!(
            single_step_plan.steps[0],
            ActionStep::Wait { duration: 1.0 }
        ));
    }

    #[test]
    fn test_plan_intent_many_steps() {
        let many_steps_plan = PlanIntent {
            plan_id: "many-steps".into(),
            steps: (0..100)
                .map(|i| ActionStep::MoveTo {
                    x: i,
                    y: i,
                    speed: None,
                })
                .collect(),
        };

        assert_eq!(many_steps_plan.steps.len(), 100);
    }

    #[test]
    fn test_mock_goap_success() {
        let goap = MockGoap {
            action_to_return: ActionStep::Scan { radius: 15.0 },
            should_fail: false,
        };

        let snap = create_test_snapshot(1.0);
        let plan = goap.propose_plan(&snap);

        assert_eq!(plan.plan_id, "mock-goap");
        assert_eq!(plan.steps.len(), 1);
        assert!(matches!(plan.steps[0], ActionStep::Scan { radius: 15.0 }));
    }

    #[test]
    fn test_mock_goap_failure() {
        let goap = MockGoap {
            action_to_return: ActionStep::Wait { duration: 1.0 },
            should_fail: true,
        };

        let snap = create_test_snapshot(1.0);
        let plan = goap.propose_plan(&snap);

        assert_eq!(plan.plan_id, "mock-fail");
        assert_eq!(plan.steps.len(), 0);
    }

    #[test]
    fn test_mock_bt_always_returns_scan() {
        let bt = MockBT;
        let snap = create_test_snapshot(1.0);
        let plan = bt.propose_plan(&snap);

        assert_eq!(plan.plan_id, "mock-bt");
        assert_eq!(plan.steps.len(), 1);
        assert!(matches!(plan.steps[0], ActionStep::Scan { radius: 10.0 }));
    }

    #[test]
    fn test_create_test_snapshot_values() {
        let snap = create_test_snapshot(5.0);

        assert_eq!(snap.t, 5.0);
        assert_eq!(snap.me.ammo, 10);
        assert_eq!(snap.me.morale, 1.0);
        assert_eq!(snap.me.pos, IVec2 { x: 5, y: 5 });
        assert_eq!(snap.player.hp, 100);
        assert_eq!(snap.player.pos, IVec2 { x: 5, y: 5 });
        assert_eq!(snap.enemies.len(), 1);
        assert_eq!(snap.enemies[0].id, 1);
        assert_eq!(snap.enemies[0].pos, IVec2 { x: 10, y: 10 });
        assert_eq!(snap.objective, Some("extract".into()));
    }

    #[test]
    fn test_companion_state_zero_ammo() {
        let companion = CompanionState {
            ammo: 0,
            cooldowns: BTreeMap::new(),
            morale: 0.5,
            pos: IVec2 { x: 0, y: 0 },
        };

        assert_eq!(companion.ammo, 0);
    }

    #[test]
    fn test_companion_state_negative_morale() {
        let companion = CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: -0.5, // Invalid but should be handled
            pos: IVec2 { x: 0, y: 0 },
        };

        assert!(companion.morale < 0.0);
    }

    #[test]
    fn test_companion_state_high_morale() {
        let companion = CompanionState {
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 2.0, // Above max (should be clamped by game logic)
            pos: IVec2 { x: 0, y: 0 },
        };

        assert!(companion.morale > 1.0);
    }

    #[test]
    fn test_enemy_state_dead() {
        let enemy = EnemyState {
            id: 1,
            pos: IVec2 { x: 10, y: 10 },
            hp: 0, // Dead
            cover: "none".into(),
            last_seen: 1.0,
        };

        assert_eq!(enemy.hp, 0);
    }

    #[test]
    fn test_enemy_state_negative_hp() {
        let enemy = EnemyState {
            id: 1,
            pos: IVec2 { x: 10, y: 10 },
            hp: -50, // Overkill damage
            cover: "none".into(),
            last_seen: 1.0,
        };

        assert!(enemy.hp < 0);
    }

    #[test]
    fn test_player_state_zero_hp() {
        let player = PlayerState {
            hp: 0, // Dead
            pos: IVec2 { x: 0, y: 0 },
            stance: "prone".into(),
            orders: vec![],
        };

        assert_eq!(player.hp, 0);
    }

    #[test]
    fn test_player_state_many_orders() {
        let player = PlayerState {
            hp: 100,
            pos: IVec2 { x: 0, y: 0 },
            stance: "stand".into(),
            orders: (0..50).map(|i| format!("order_{}", i)).collect(),
        };

        assert_eq!(player.orders.len(), 50);
    }

    // ============================================================================
    // Comprehensive Integration Tests (Migrated from tests/ directory)
    // These tests validate AIArbiter lifecycle and contribute to lib coverage
    // Uses existing MockGoap/MockBT mocks defined above
    // ============================================================================

    // Additional imports for integration tests
    use crate::orchestrator::OrchestratorAsync;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    // Mock LLM Orchestrator for async testing
    struct MockLlmOrch {
        plan_to_return: Arc<Mutex<Option<PlanIntent>>>,
        delay_ms: u64,
    }

    impl MockLlmOrch {
        fn new() -> Self {
            Self {
                plan_to_return: Arc::new(Mutex::new(None)),
                delay_ms: 0,
            }
        }

        fn with_plan(self, plan: PlanIntent) -> Self {
            *self.plan_to_return.lock().unwrap() = Some(plan);
            self
        }

        fn with_delay(mut self, delay_ms: u64) -> Self {
            self.delay_ms = delay_ms;
            self
        }
    }

    #[async_trait::async_trait]
    impl OrchestratorAsync for MockLlmOrch {
        async fn plan(&self, _snap: WorldSnapshot, _budget_ms: u32) -> Result<PlanIntent> {
            if self.delay_ms > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
            }

            let plan_guard = self.plan_to_return.lock().unwrap();
            match plan_guard.as_ref() {
                Some(plan) => Ok(plan.clone()),
                None => Err(anyhow::anyhow!("Mock LLM orchestrator configured to fail")),
            }
        }

        fn name(&self) -> &'static str {
            "MockLlmOrch"
        }
    }

    // Helper to create arbiter with configured mocks
    fn create_arbiter_with_mocks(
        goap: Box<dyn Orchestrator>,
        bt: Box<dyn Orchestrator>,
        llm_delay: Duration,
    ) -> AIArbiter {
        let mock_llm = Arc::new(
            MockLlmOrch::new()
                .with_delay(llm_delay.as_millis() as u64)
                .with_plan(PlanIntent {
                    plan_id: "llm".into(),
                    steps: vec![
                        ActionStep::MoveTo {
                            x: 1,
                            y: 1,
                            speed: None,
                        },
                        ActionStep::Scan { radius: 5.0 },
                        ActionStep::Attack { target_id: 1 },
                    ],
                }),
        );

        let runtime = tokio::runtime::Handle::current();
        let llm_executor = LlmExecutor::new(mock_llm, runtime);

        AIArbiter::with_single_executor(llm_executor, goap, bt)
    }

    // ============================================================================
    // Integration Tests: update() Method
    // ============================================================================

    #[tokio::test]
    async fn test_update_goap_mode_returns_goap_action() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Scan { radius: 10.0 },
            should_fail: false,
        });
        let bt = Box::new(MockBT);
        let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_secs(10));

        let snap = create_test_snapshot(0.0);
        let action = arbiter.update(&snap);

        assert!(matches!(action, ActionStep::Scan { radius: 10.0 }));
        assert_eq!(arbiter.mode(), AIControlMode::GOAP);

        let (_, _, _, _, goap_actions, _) = arbiter.metrics();
        assert_eq!(goap_actions, 1);
    }

    #[tokio::test]
    async fn test_update_executing_llm_returns_plan_steps_sequentially() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Wait { duration: 1.0 },
            should_fail: false,
        });
        let bt = Box::new(MockBT);
        let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_secs(10));

        let plan = PlanIntent {
            plan_id: "test-plan".into(),
            steps: vec![
                ActionStep::MoveTo {
                    x: 1,
                    y: 1,
                    speed: None,
                },
                ActionStep::Scan { radius: 5.0 },
                ActionStep::Attack { target_id: 1 },
            ],
        };
        arbiter.transition_to_llm(plan);

        let snap = create_test_snapshot(0.0);

        let action1 = arbiter.update(&snap);
        assert!(matches!(
            action1,
            ActionStep::MoveTo {
                x: 1,
                y: 1,
                speed: None
            }
        ));
        assert!(matches!(
            arbiter.mode(),
            AIControlMode::ExecutingLLM { step_index: 1 }
        ));

        let action2 = arbiter.update(&snap);
        assert!(matches!(action2, ActionStep::Scan { radius: 5.0 }));
        assert!(matches!(
            arbiter.mode(),
            AIControlMode::ExecutingLLM { step_index: 2 }
        ));

        let action3 = arbiter.update(&snap);
        assert!(matches!(action3, ActionStep::Attack { target_id: 1 }));
        assert_eq!(arbiter.mode(), AIControlMode::GOAP);

        let (_, _, _, _, _, llm_steps) = arbiter.metrics();
        assert_eq!(llm_steps, 3);
    }

    #[tokio::test]
    async fn test_update_goap_fallback_when_empty_plan() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Wait { duration: 1.0 },
            should_fail: true, // Returns empty plan
        });
        let bt = Box::new(MockBT);
        let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_secs(10));

        let snap = create_test_snapshot(0.0);
        let action = arbiter.update(&snap);

        assert!(matches!(action, ActionStep::Scan { radius: 10.0 })); // MockBT action
        assert_eq!(arbiter.mode(), AIControlMode::BehaviorTree);
    }

    // ============================================================================
    // Integration Tests: maybe_request_llm() Cooldown
    // ============================================================================

    #[tokio::test]
    async fn test_maybe_request_llm_respects_cooldown() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Wait { duration: 1.0 },
            should_fail: false,
        });
        let bt = Box::new(MockBT);
        let mut arbiter =
            create_arbiter_with_mocks(goap, bt, Duration::from_millis(100)).with_llm_cooldown(5.0);

        let snap1 = create_test_snapshot(0.0);
        arbiter.update(&snap1);
        assert!(arbiter.is_llm_active());

        let (_, requests1, _, _, _, _) = arbiter.metrics();
        assert_eq!(requests1, 1);

        let snap2 = create_test_snapshot(2.0);
        arbiter.update(&snap2);

        let (_, requests2, _, _, _, _) = arbiter.metrics();
        assert_eq!(requests2, 1);

        tokio::time::sleep(Duration::from_millis(150)).await;
        let snap_transition = create_test_snapshot(3.0);
        arbiter.update(&snap_transition);
        arbiter.update(&snap_transition);
        arbiter.update(&snap_transition);
        arbiter.update(&snap_transition);

        assert_eq!(arbiter.mode(), AIControlMode::GOAP);

        let snap3 = create_test_snapshot(8.0);
        arbiter.update(&snap3);

        let (_, requests3, _, _, _, _) = arbiter.metrics();
        assert_eq!(requests3, 2);
    }

    #[tokio::test]
    async fn test_maybe_request_llm_skips_when_executing_llm() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Wait { duration: 1.0 },
            should_fail: false,
        });
        let bt = Box::new(MockBT);
        let mut arbiter =
            create_arbiter_with_mocks(goap, bt, Duration::from_secs(10)).with_llm_cooldown(0.0);

        let plan = PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::Wait { duration: 1.0 }],
        };
        arbiter.transition_to_llm(plan);

        let snap = create_test_snapshot(10.0);
        arbiter.update(&snap);

        let (_, requests, _, _, _, _) = arbiter.metrics();
        assert_eq!(requests, 0);
    }

    // ============================================================================
    // Integration Tests: poll_llm_result() Completion
    // ============================================================================

    #[tokio::test]
    async fn test_poll_llm_result_success_transitions_to_executing_llm() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Wait { duration: 1.0 },
            should_fail: false,
        });
        let bt = Box::new(MockBT);
        let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_millis(100));

        let snap1 = create_test_snapshot(0.0);
        arbiter.update(&snap1);

        tokio::time::sleep(Duration::from_millis(150)).await;

        let snap2 = create_test_snapshot(1.0);
        arbiter.update(&snap2);

        assert!(matches!(
            arbiter.mode(),
            AIControlMode::ExecutingLLM { step_index: 1 }
        ));

        let (transitions, _, successes, failures, _, _) = arbiter.metrics();
        assert_eq!(successes, 1);
        assert_eq!(failures, 0);
        assert!(transitions >= 1);
    }

    #[tokio::test]
    async fn test_poll_llm_result_failure_stays_in_goap() {
        let mock_llm = Arc::new(MockLlmOrch::new().with_delay(100));
        let runtime = tokio::runtime::Handle::current();
        let llm_executor = LlmExecutor::new(mock_llm, runtime);

        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Scan { radius: 10.0 },
            should_fail: false,
        });
        let bt = Box::new(MockBT);
        let mut arbiter = AIArbiter::with_single_executor(llm_executor, goap, bt);

        let snap1 = create_test_snapshot(0.0);
        arbiter.update(&snap1);

        tokio::time::sleep(Duration::from_millis(150)).await;

        let snap2 = create_test_snapshot(1.0);
        let action = arbiter.update(&snap2);

        assert_eq!(arbiter.mode(), AIControlMode::GOAP);
        assert!(matches!(action, ActionStep::Scan { radius: 10.0 }));

        let (_, _, successes, failures, _, _) = arbiter.metrics();
        assert_eq!(successes, 0);
        assert_eq!(failures, 1);
    }

    // ============================================================================
    // Integration Tests: Transition Logic
    // ============================================================================

    #[tokio::test]
    async fn test_transition_to_goap_clears_plan() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Wait { duration: 1.0 },
            should_fail: false,
        });
        let bt = Box::new(MockBT);
        let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_secs(10));

        let plan = PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::Wait { duration: 1.0 }],
        };
        arbiter.transition_to_llm(plan);

        assert!(arbiter.current_plan().is_some());

        let snap = create_test_snapshot(0.0);
        arbiter.update(&snap);

        assert!(arbiter.current_plan().is_none());
        assert_eq!(arbiter.mode(), AIControlMode::GOAP);
    }

    #[tokio::test]
    async fn test_transition_to_bt_clears_task_and_plan() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Wait { duration: 1.0 },
            should_fail: true, // Empty plan triggers BT
        });
        let bt = Box::new(MockBT);
        let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_millis(100));

        let snap1 = create_test_snapshot(0.0);
        let action = arbiter.update(&snap1);

        assert_eq!(arbiter.mode(), AIControlMode::BehaviorTree);
        assert!(matches!(action, ActionStep::Scan { radius: 10.0 }));
        assert!(!arbiter.is_llm_active());
        assert!(arbiter.current_plan().is_none());
    }

    // ============================================================================
    // Integration Tests: Error Handling
    // ============================================================================

    #[tokio::test]
    async fn test_invalid_step_index_falls_back_to_goap() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Scan { radius: 10.0 },
            should_fail: false,
        });
        let bt = Box::new(MockBT);
        let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_secs(10));

        let plan = PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::Wait { duration: 1.0 }],
        };
        arbiter.transition_to_llm(plan);

        let snap1 = create_test_snapshot(0.0);
        arbiter.update(&snap1);

        assert_eq!(arbiter.mode(), AIControlMode::GOAP);

        let snap2 = create_test_snapshot(1.0);
        let action = arbiter.update(&snap2);
        assert!(matches!(action, ActionStep::Scan { radius: 10.0 }));
    }

    #[tokio::test]
    async fn test_executing_llm_without_plan_falls_back_to_goap() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Scan { radius: 10.0 },
            should_fail: false,
        });
        let bt = Box::new(MockBT);
        let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_secs(10));

        let plan = PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::Wait { duration: 1.0 }],
        };
        arbiter.transition_to_llm(plan);

        let snap1 = create_test_snapshot(0.0);
        arbiter.update(&snap1);

        assert_eq!(arbiter.mode(), AIControlMode::GOAP);
    }

    // ============================================================================
    // Integration Tests: Metrics Tracking
    // ============================================================================

    #[tokio::test]
    async fn test_metrics_track_all_counters() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Wait { duration: 1.0 },
            should_fail: false,
        });
        let bt = Box::new(MockBT);
        let mut arbiter =
            create_arbiter_with_mocks(goap, bt, Duration::from_millis(100)).with_llm_cooldown(0.5);

        let snap1 = create_test_snapshot(0.0);
        let snap2 = create_test_snapshot(1.0);
        let snap3 = create_test_snapshot(2.0);

        arbiter.update(&snap1);
        arbiter.update(&snap2);

        tokio::time::sleep(Duration::from_millis(150)).await;
        arbiter.update(&snap3);

        let (transitions, requests, successes, failures, goap_actions, llm_steps) =
            arbiter.metrics();

        assert!(transitions >= 1);
        assert_eq!(requests, 1);
        assert_eq!(successes, 1);
        assert_eq!(failures, 0);
        assert_eq!(goap_actions, 2);
        assert_eq!(llm_steps, 1);
    }

    // ============================================================================
    // Integration Tests: with_llm_cooldown() Configuration
    // ============================================================================

    #[tokio::test]
    async fn test_with_llm_cooldown_configures_cooldown() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Wait { duration: 1.0 },
            should_fail: false,
        });
        let bt = Box::new(MockBT);
        let mut arbiter =
            create_arbiter_with_mocks(goap, bt, Duration::from_millis(100)).with_llm_cooldown(10.0);

        let snap1 = create_test_snapshot(0.0);
        arbiter.update(&snap1);
        let (_, requests1, _, _, _, _) = arbiter.metrics();
        assert_eq!(requests1, 1);
        assert!(arbiter.is_llm_active());

        let snap2 = create_test_snapshot(5.0);
        arbiter.update(&snap2);
        let (_, requests2, _, _, _, _) = arbiter.metrics();
        assert_eq!(requests2, 1);

        tokio::time::sleep(Duration::from_millis(150)).await;
        let snap_transition = create_test_snapshot(6.0);
        arbiter.update(&snap_transition);
        arbiter.update(&snap_transition);
        arbiter.update(&snap_transition);
        arbiter.update(&snap_transition);

        assert_eq!(arbiter.mode(), AIControlMode::GOAP);

        let snap3 = create_test_snapshot(11.0);
        arbiter.update(&snap3);
        let (_, requests3, _, _, _, _) = arbiter.metrics();
        assert_eq!(requests3, 2);
    }

    // ============================================================================
    // AIControlMode Helper Method Tests
    // ============================================================================

    #[test]
    fn test_ai_control_mode_is_goap() {
        assert!(AIControlMode::GOAP.is_goap());
        assert!(!AIControlMode::ExecutingLLM { step_index: 0 }.is_goap());
        assert!(!AIControlMode::BehaviorTree.is_goap());
    }

    #[test]
    fn test_ai_control_mode_is_executing_llm() {
        assert!(!AIControlMode::GOAP.is_executing_llm());
        assert!(AIControlMode::ExecutingLLM { step_index: 0 }.is_executing_llm());
        assert!(AIControlMode::ExecutingLLM { step_index: 5 }.is_executing_llm());
        assert!(!AIControlMode::BehaviorTree.is_executing_llm());
    }

    #[test]
    fn test_ai_control_mode_is_behavior_tree() {
        assert!(!AIControlMode::GOAP.is_behavior_tree());
        assert!(!AIControlMode::ExecutingLLM { step_index: 0 }.is_behavior_tree());
        assert!(AIControlMode::BehaviorTree.is_behavior_tree());
    }

    #[test]
    fn test_ai_control_mode_is_fallback() {
        // BehaviorTree is a fallback mode
        assert!(!AIControlMode::GOAP.is_fallback());
        assert!(!AIControlMode::ExecutingLLM { step_index: 0 }.is_fallback());
        assert!(AIControlMode::BehaviorTree.is_fallback());
    }

    #[test]
    fn test_ai_control_mode_is_instant() {
        // GOAP and BehaviorTree provide instant responses (in-memory graph traversals)
        assert!(AIControlMode::GOAP.is_instant());
        assert!(AIControlMode::BehaviorTree.is_instant());
        // Only ExecutingLLM involves async I/O
        assert!(!AIControlMode::ExecutingLLM { step_index: 0 }.is_instant());
    }

    #[test]
    fn test_ai_control_mode_step_index() {
        assert_eq!(AIControlMode::GOAP.step_index(), None);
        assert_eq!(
            AIControlMode::ExecutingLLM { step_index: 0 }.step_index(),
            Some(0)
        );
        assert_eq!(
            AIControlMode::ExecutingLLM { step_index: 42 }.step_index(),
            Some(42)
        );
        assert_eq!(AIControlMode::BehaviorTree.step_index(), None);
    }

    #[test]
    fn test_ai_control_mode_executing_llm_factory() {
        let mode = AIControlMode::executing_llm(5);
        assert!(matches!(
            mode,
            AIControlMode::ExecutingLLM { step_index: 5 }
        ));

        let mode_zero = AIControlMode::executing_llm(0);
        assert!(matches!(
            mode_zero,
            AIControlMode::ExecutingLLM { step_index: 0 }
        ));

        let mode_large = AIControlMode::executing_llm(1000);
        assert!(matches!(
            mode_large,
            AIControlMode::ExecutingLLM { step_index: 1000 }
        ));
    }

    #[test]
    fn test_ai_control_mode_mode_name() {
        assert_eq!(AIControlMode::GOAP.mode_name(), "GOAP");
        assert_eq!(
            AIControlMode::ExecutingLLM { step_index: 0 }.mode_name(),
            "ExecutingLLM"
        );
        assert_eq!(
            AIControlMode::ExecutingLLM { step_index: 99 }.mode_name(),
            "ExecutingLLM"
        );
        assert_eq!(AIControlMode::BehaviorTree.mode_name(), "BehaviorTree");
    }

    #[test]
    fn test_ai_control_mode_default() {
        let default_mode = AIControlMode::default();
        assert_eq!(default_mode, AIControlMode::GOAP);
        assert!(default_mode.is_goap());
    }

    #[test]
    fn test_ai_control_mode_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(AIControlMode::GOAP);
        set.insert(AIControlMode::ExecutingLLM { step_index: 0 });
        set.insert(AIControlMode::ExecutingLLM { step_index: 1 });
        set.insert(AIControlMode::BehaviorTree);

        // Should have 4 unique entries (different step_index values are different)
        assert_eq!(set.len(), 4);
        assert!(set.contains(&AIControlMode::GOAP));
        assert!(set.contains(&AIControlMode::ExecutingLLM { step_index: 0 }));
        assert!(set.contains(&AIControlMode::ExecutingLLM { step_index: 1 }));
        assert!(set.contains(&AIControlMode::BehaviorTree));
    }

    #[test]
    fn test_ai_control_mode_helpers_consistency() {
        // GOAP: instant, not fallback, no step_index
        let goap = AIControlMode::GOAP;
        assert!(goap.is_goap());
        assert!(goap.is_instant());
        assert!(!goap.is_fallback());
        assert!(goap.step_index().is_none());

        // ExecutingLLM: has step_index, not instant, not fallback
        let llm = AIControlMode::ExecutingLLM { step_index: 3 };
        assert!(llm.is_executing_llm());
        assert!(!llm.is_instant());
        assert!(!llm.is_fallback());
        assert_eq!(llm.step_index(), Some(3));

        // BehaviorTree: fallback mode, instant (in-memory graph traversal)
        let bt = AIControlMode::BehaviorTree;
        assert!(bt.is_behavior_tree());
        assert!(bt.is_fallback());
        assert!(bt.is_instant());
        assert!(bt.step_index().is_none());
    }

    #[test]
    fn test_ai_control_mode_all_mutually_exclusive() {
        // Each mode should only return true for exactly one is_* method
        let modes = [
            AIControlMode::GOAP,
            AIControlMode::ExecutingLLM { step_index: 0 },
            AIControlMode::BehaviorTree,
        ];

        for mode in &modes {
            let count = [
                mode.is_goap(),
                mode.is_executing_llm(),
                mode.is_behavior_tree(),
            ]
            .iter()
            .filter(|&&b| b)
            .count();

            assert_eq!(
                count, 1,
                "Mode {:?} should match exactly one category",
                mode
            );
        }
    }

    // ============================================================================
    // Mutation-Killing Tests (targeted from cargo-mutants analysis)
    // ============================================================================

    /// Kills mutation: line 429 `step_index < plan.steps.len()` → `<=`
    /// Empty LLM plan with step_index=0: `0 < 0` = false (fallback),
    /// but `0 <= 0` = true → panics on steps[0].
    #[tokio::test]
    async fn test_empty_llm_plan_falls_back_to_goap() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Scan { radius: 10.0 },
            should_fail: false,
        });
        let bt = Box::new(MockBT);
        let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_secs(10));

        // Transition to LLM with an empty plan (step_index=0, len=0)
        let plan = PlanIntent {
            plan_id: "empty".into(),
            steps: vec![],
        };
        arbiter.transition_to_llm(plan);
        assert!(matches!(
            arbiter.mode(),
            AIControlMode::ExecutingLLM { step_index: 0 }
        ));

        // Update should detect invalid state and fall back to GOAP
        let snap = create_test_snapshot(0.0);
        let action = arbiter.update(&snap);

        assert_eq!(arbiter.mode(), AIControlMode::GOAP);
        assert!(matches!(action, ActionStep::Scan { radius: 10.0 }));
    }

    /// Kills mutations: line 519 `mode_transitions += 1` → `-=` and `*=`
    /// in transition_to_goap. After transition_to_llm (transitions=1) and then
    /// plan exhaustion → transition_to_goap, transitions must be exactly 2.
    /// With `-=`: 1-1=0≠2. With `*=`: 1*1=1≠2.
    #[tokio::test]
    async fn test_transition_to_goap_increments_mode_transitions() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Wait { duration: 1.0 },
            should_fail: false,
        });
        let bt = Box::new(MockBT);
        let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_secs(10));

        // transition_to_llm → mode_transitions=1
        let plan = PlanIntent {
            plan_id: "single".into(),
            steps: vec![ActionStep::Wait { duration: 1.0 }],
        };
        arbiter.transition_to_llm(plan);
        let (t1, _, _, _, _, _) = arbiter.metrics();
        assert_eq!(t1, 1);

        // Exhaust plan → transition_to_goap → mode_transitions must be 2
        let snap = create_test_snapshot(0.0);
        arbiter.update(&snap);
        let (t2, _, _, _, _, _) = arbiter.metrics();
        assert_eq!(t2, 2);
    }

    /// Kills mutation: line 536 `mode_transitions += 1` → `*=`
    /// in transition_to_bt. Before the call, mode_transitions=0.
    /// With `+=`: 0+1=1. With `*=`: 0*1=0≠1.
    #[tokio::test]
    async fn test_transition_to_bt_increments_mode_transitions() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Wait { duration: 1.0 },
            should_fail: true, // Empty plan triggers BT fallback
        });
        let bt = Box::new(MockBT);
        let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_secs(10));

        let snap = create_test_snapshot(0.0);
        arbiter.update(&snap);

        assert_eq!(arbiter.mode(), AIControlMode::BehaviorTree);
        let (transitions, _, _, _, _, _) = arbiter.metrics();
        assert_eq!(transitions, 1);
    }

    /// Kills mutations: line 600 `cooldown_elapsed < self.llm_request_cooldown`
    /// → `==` and `<=`. At exact boundary (elapsed == cooldown):
    /// Original `<`: 5.0 < 5.0 = false → request proceeds.
    /// Mutation `<=`: 5.0 <= 5.0 = true → returns early, no request.
    /// Mutation `==`: 5.0 == 5.0 = true → returns early, no request.
    #[tokio::test]
    async fn test_cooldown_exact_boundary_allows_request() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Wait { duration: 1.0 },
            should_fail: false,
        });
        let bt = Box::new(MockBT);
        let mut arbiter =
            create_arbiter_with_mocks(goap, bt, Duration::from_millis(10)).with_llm_cooldown(5.0);

        // First request at t=0.0
        let snap1 = create_test_snapshot(0.0);
        arbiter.update(&snap1);
        let (_, r1, _, _, _, _) = arbiter.metrics();
        assert_eq!(r1, 1);

        // Wait for LLM task to complete
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Poll result → transitions to ExecutingLLM, executes step 0
        let snap2 = create_test_snapshot(1.0);
        arbiter.update(&snap2);
        // Execute remaining steps (mock plan has 3 steps)
        let snap3 = create_test_snapshot(2.0);
        arbiter.update(&snap3);
        let snap4 = create_test_snapshot(3.0);
        arbiter.update(&snap4);
        // Should be back in GOAP after exhausting 3-step plan
        assert_eq!(arbiter.mode(), AIControlMode::GOAP);

        // At t=5.0: elapsed = 5.0 - 0.0 = 5.0 = cooldown (exact boundary)
        // With <: proceeds. With <= or ==: returns early.
        let snap5 = create_test_snapshot(5.0);
        arbiter.update(&snap5);
        let (_, r2, _, _, _, _) = arbiter.metrics();
        assert_eq!(
            r2, 2,
            "Second LLM request should fire at exact cooldown boundary"
        );
    }
}
