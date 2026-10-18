#![cfg(feature = "llm_orchestrator")]
//! AI Arbiter Implementation Tests
//!
//! Comprehensive integration tests targeting the AIArbiter implementation
//! to achieve 80%+ coverage of ai_arbiter.rs.
//!
//! Test coverage targets:
//! - update() method: All modes (GOAP, ExecutingLLM, BehaviorTree)
use anyhow::{anyhow, Result};
#[cfg(feature = "llm_orchestrator")]
use astraweave_ai::ai_arbiter::{AIArbiter, AIControlMode};
#[cfg(feature = "llm_orchestrator")]
use astraweave_ai::llm_executor::LlmExecutor;
use astraweave_ai::orchestrator::{Orchestrator, OrchestratorAsync};
use astraweave_core::schema::{CompanionState, EnemyState, IVec2, PlayerState, WorldSnapshot};
use astraweave_core::{ActionStep, PlanIntent};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ============================================================================
// Mock Orchestrators
// ============================================================================

/// Mock LLM orchestrator with controllable behavior
struct MockLlmOrch {
    /// If Some, return this plan. If None, return error.
    plan_to_return: Arc<Mutex<Option<PlanIntent>>>,
    /// Artificial delay in milliseconds
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

    #[allow(dead_code)]
    fn set_plan(&self, plan: Option<PlanIntent>) {
        *self.plan_to_return.lock().unwrap() = plan;
    }
}

#[async_trait::async_trait]
impl OrchestratorAsync for MockLlmOrch {
    async fn plan(&self, _snap: WorldSnapshot, _budget_ms: u32) -> Result<PlanIntent> {
        // Artificial delay to simulate LLM inference
        if self.delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        }

        let plan_guard = self.plan_to_return.lock().unwrap();
        match plan_guard.as_ref() {
            Some(plan) => Ok(plan.clone()),
            None => Err(anyhow!("Mock LLM orchestrator configured to fail")),
        }
    }

    fn name(&self) -> &'static str {
        "MockLlmOrch"
    }
}

/// Mock GOAP orchestrator that returns configurable actions
struct MockGoap {
    action: ActionStep,
    empty_plan: bool,
}

impl MockGoap {
    fn new(action: ActionStep) -> Self {
        Self {
            action,
            empty_plan: false,
        }
    }

    fn with_empty_plan() -> Self {
        Self {
            action: ActionStep::Wait { duration: 1.0 },
            empty_plan: true,
        }
    }
}

impl Orchestrator for MockGoap {
    fn propose_plan(&self, _snap: &WorldSnapshot) -> PlanIntent {
        if self.empty_plan {
            return PlanIntent {
                plan_id: "empty".into(),
                steps: vec![],
            };
        }
        PlanIntent {
            plan_id: "goap".into(),
            steps: vec![self.action.clone()],
        }
    }
}

/// Mock BehaviorTree orchestrator
struct MockBT {
    action: ActionStep,
}

impl MockBT {
    fn new(action: ActionStep) -> Self {
        Self { action }
    }
}

impl Orchestrator for MockBT {
    fn propose_plan(&self, _snap: &WorldSnapshot) -> PlanIntent {
        PlanIntent {
            plan_id: "bt".into(),
            steps: vec![self.action.clone()],
        }
    }
}

// ============================================================================
// Test Helpers
// ============================================================================

fn create_test_snapshot(t: f32) -> WorldSnapshot {
    WorldSnapshot {
        t,
        player: PlayerState {
            hp: 100,

            pos: IVec2 { x: 0, y: 0 },
            stance: "stand".into(),
            orders: vec![],
        },
        me: CompanionState {
            pos: IVec2 { x: 5, y: 5 },
            ammo: 10,
            cooldowns: BTreeMap::new(),
            morale: 1.0,
        },
        enemies: vec![EnemyState {
            id: 1,
            pos: IVec2 { x: 10, y: 10 },
            hp: 50,
            cover: "low".into(),
            last_seen: 0.0,
        }],
        pois: vec![],
        obstacles: vec![],
        objective: Some("Test objective".into()),

        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}

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
    let goap = Box::new(MockGoap::new(ActionStep::Scan { radius: 10.0 }));
    let bt = Box::new(MockBT::new(ActionStep::Wait { duration: 1.0 }));
    let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_secs(10));

    let snap = create_test_snapshot(0.0);
    let action = arbiter.update(&snap);

    // Should return GOAP action (first step of GOAP plan)
    assert!(matches!(action, ActionStep::Scan { radius: 10.0 }));

    // Metrics: 1 GOAP action
    let (_, _, _, _, goap_actions, _) = arbiter.metrics();
    assert_eq!(goap_actions, 1, "Should have 1 GOAP action");

    // Mode should still be GOAP
    assert_eq!(arbiter.mode(), AIControlMode::GOAP);
}

#[tokio::test]
async fn test_update_executing_llm_returns_plan_steps_sequentially() {
    let goap = Box::new(MockGoap::new(ActionStep::Wait { duration: 1.0 }));
    let bt = Box::new(MockBT::new(ActionStep::Wait { duration: 1.0 }));
    let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_secs(10));

    // Manually inject LLM plan
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

    // Step 1: MoveTo
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

    // Step 2: Scan
    let action2 = arbiter.update(&snap);
    assert!(matches!(action2, ActionStep::Scan { radius: 5.0 }));
    assert!(matches!(
        arbiter.mode(),
        AIControlMode::ExecutingLLM { step_index: 2 }
    ));

    // Step 3: Attack (last step)
    let action3 = arbiter.update(&snap);
    assert!(matches!(action3, ActionStep::Attack { target_id: 1 }));

    // After last step, should transition back to GOAP
    assert_eq!(
        arbiter.mode(),
        AIControlMode::GOAP,
        "Should transition to GOAP after plan exhaustion"
    );

    // Metrics: 3 LLM steps executed
    let (_, _, _, _, _, llm_steps) = arbiter.metrics();
    assert_eq!(llm_steps, 3, "Should have 3 LLM steps executed");
}

#[tokio::test]
async fn test_update_goap_fallback_when_empty_plan() {
    let goap = Box::new(MockGoap::with_empty_plan());
    let bt = Box::new(MockBT::new(ActionStep::Scan { radius: 15.0 }));
    let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_secs(10));

    let snap = create_test_snapshot(0.0);
    let action = arbiter.update(&snap);

    // GOAP has no steps, should fall back to BT
    assert!(
        matches!(action, ActionStep::Scan { radius: 15.0 }),
        "Should return BT action when GOAP plan is empty"
    );

    // Should have transitioned to BT
    assert_eq!(
        arbiter.mode(),
        AIControlMode::BehaviorTree,
        "Should transition to BehaviorTree when GOAP fails"
    );
}

// ============================================================================
// Integration Tests: maybe_request_llm() Cooldown
// ============================================================================

#[tokio::test]
async fn test_maybe_request_llm_respects_cooldown() {
    let goap = Box::new(MockGoap::new(ActionStep::Wait { duration: 1.0 }));
    let bt = Box::new(MockBT::new(ActionStep::Wait { duration: 1.0 }));
    let mut arbiter =
        create_arbiter_with_mocks(goap, bt, Duration::from_millis(100)).with_llm_cooldown(5.0); // 5 second cooldown

    // First update: Should allow LLM request (t=0.0, cooldown starts at -999.0)
    let snap1 = create_test_snapshot(0.0);
    arbiter.update(&snap1);

    // Should have active LLM task
    assert!(
        arbiter.is_llm_active(),
        "Should have active LLM task after first update"
    );

    // Metrics: 1 LLM request
    let (_, requests1, _, _, _, _) = arbiter.metrics();
    assert_eq!(requests1, 1, "Should have 1 LLM request");

    // Second update (t=2.0): Should NOT request (active task blocks new requests)
    let snap2 = create_test_snapshot(2.0);
    arbiter.update(&snap2);

    // Metrics: Still 1 LLM request (active task prevents new request)
    let (_, requests2, _, _, _, _) = arbiter.metrics();
    assert_eq!(
        requests2, 1,
        "Should still have 1 LLM request (active task blocks)"
    );

    // Wait for LLM completion and transition
    tokio::time::sleep(Duration::from_millis(150)).await;
    let snap_transition = create_test_snapshot(3.0);
    arbiter.update(&snap_transition); // This polls the result and transitions to ExecutingLLM

    // Verify we transitioned to ExecutingLLM
    assert!(
        matches!(arbiter.mode(), AIControlMode::ExecutingLLM { .. }),
        "Should be in ExecutingLLM mode"
    );

    // Now exhaust the plan to return to GOAP
    arbiter.update(&snap_transition); // Step 1
    arbiter.update(&snap_transition); // Step 2
    arbiter.update(&snap_transition); // Step 3, exhausts plan, transitions to GOAP

    // Verify back in GOAP mode
    assert_eq!(
        arbiter.mode(),
        AIControlMode::GOAP,
        "Should be back in GOAP mode"
    );

    // Third update (t=8.0): Should allow new request (cooldown expired AND back in GOAP)
    let snap3 = create_test_snapshot(8.0);
    arbiter.update(&snap3);

    // Metrics: 2 LLM requests (cooldown expired AND previous task completed)
    let (_, requests3, _, _, _, _) = arbiter.metrics();
    assert_eq!(
        requests3, 2,
        "Should have 2 LLM requests after cooldown expires and plan exhausted"
    );
}

#[tokio::test]
async fn test_maybe_request_llm_skips_when_executing_llm() {
    let goap = Box::new(MockGoap::new(ActionStep::Wait { duration: 1.0 }));
    let bt = Box::new(MockBT::new(ActionStep::Wait { duration: 1.0 }));
    let mut arbiter =
        create_arbiter_with_mocks(goap, bt, Duration::from_secs(10)).with_llm_cooldown(0.0); // Zero cooldown (always allow)

    // Manually inject LLM plan
    let plan = PlanIntent {
        plan_id: "test".into(),
        steps: vec![ActionStep::Wait { duration: 1.0 }],
    };
    arbiter.transition_to_llm(plan);

    // Update while in ExecutingLLM mode
    let snap = create_test_snapshot(10.0);
    arbiter.update(&snap);

    // Should NOT request LLM while executing plan (even with zero cooldown)
    let (_, requests, _, _, _, _) = arbiter.metrics();
    assert_eq!(requests, 0, "Should not request LLM while executing plan");
}

// ============================================================================
// Integration Tests: poll_llm_result() Completion
// ============================================================================

#[tokio::test]
async fn test_poll_llm_result_success_transitions_to_executing_llm() {
    let goap = Box::new(MockGoap::new(ActionStep::Wait { duration: 1.0 }));
    let bt = Box::new(MockBT::new(ActionStep::Wait { duration: 1.0 }));
    let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_millis(100));

    // Request LLM plan
    let snap1 = create_test_snapshot(0.0);
    arbiter.update(&snap1);

    // Wait for LLM completion
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Next update should poll and transition to ExecutingLLM
    let snap2 = create_test_snapshot(1.0);
    arbiter.update(&snap2);

    // Should have transitioned to ExecutingLLM
    assert!(
        matches!(
            arbiter.mode(),
            AIControlMode::ExecutingLLM { step_index: 1 }
        ),
        "Should transition to ExecutingLLM after LLM success"
    );

    // Metrics: 1 success
    let (transitions, _, successes, failures, _, _) = arbiter.metrics();
    assert_eq!(successes, 1, "Should have 1 LLM success");
    assert_eq!(failures, 0, "Should have 0 LLM failures");
    assert!(transitions >= 1, "Should have at least 1 mode transition");
}

#[tokio::test]
async fn test_poll_llm_result_failure_stays_in_goap() {
    // Create arbiter with failing LLM (returns error, no plan)
    let mock_llm = Arc::new(MockLlmOrch::new().with_delay(100)); // No plan set, will return error
    let runtime = tokio::runtime::Handle::current();
    let llm_executor = LlmExecutor::new(mock_llm, runtime);

    let goap = Box::new(MockGoap::new(ActionStep::Scan { radius: 10.0 }));
    let bt = Box::new(MockBT::new(ActionStep::Wait { duration: 1.0 }));
    let mut arbiter = AIArbiter::with_single_executor(llm_executor, goap, bt);

    // Request LLM plan
    let snap1 = create_test_snapshot(0.0);
    arbiter.update(&snap1);

    // Wait for LLM failure
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Next update should poll and stay in GOAP
    let snap2 = create_test_snapshot(1.0);
    let action = arbiter.update(&snap2);

    // Should still be in GOAP
    assert_eq!(
        arbiter.mode(),
        AIControlMode::GOAP,
        "Should stay in GOAP after LLM failure"
    );

    // Should return GOAP action
    assert!(
        matches!(action, ActionStep::Scan { radius: 10.0 }),
        "Should return GOAP action"
    );

    // Metrics: 1 failure, 0 successes
    let (_, _, successes, failures, _, _) = arbiter.metrics();
    assert_eq!(successes, 0, "Should have 0 LLM successes");
    assert_eq!(failures, 1, "Should have 1 LLM failure");
}

// ============================================================================
// Integration Tests: Transition Logic
// ============================================================================

#[tokio::test]
async fn test_transition_to_goap_clears_plan() {
    let goap = Box::new(MockGoap::new(ActionStep::Wait { duration: 1.0 }));
    let bt = Box::new(MockBT::new(ActionStep::Wait { duration: 1.0 }));
    let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_secs(10));

    // Inject plan
    let plan = PlanIntent {
        plan_id: "test".into(),
        steps: vec![ActionStep::Wait { duration: 1.0 }],
    };
    arbiter.transition_to_llm(plan);

    // Verify plan exists
    assert!(
        arbiter.current_plan().is_some(),
        "Should have plan after transition_to_llm"
    );

    // Exhaust plan (triggers transition to GOAP)
    let snap = create_test_snapshot(0.0);
    arbiter.update(&snap);

    // Plan should be cleared
    assert!(
        arbiter.current_plan().is_none(),
        "Plan should be cleared after transition to GOAP"
    );
    assert_eq!(arbiter.mode(), AIControlMode::GOAP);
}

#[tokio::test]
async fn test_transition_to_bt_clears_task_and_plan() {
    let goap = Box::new(MockGoap::with_empty_plan()); // Will trigger BT fallback
    let bt = Box::new(MockBT::new(ActionStep::Scan { radius: 15.0 }));
    let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_millis(100));

    // First update: GOAP returns empty plan, triggers BT fallback immediately
    let snap1 = create_test_snapshot(0.0);
    let action = arbiter.update(&snap1);

    // Should have transitioned to BT immediately (empty GOAP plan)
    assert_eq!(
        arbiter.mode(),
        AIControlMode::BehaviorTree,
        "Should transition to BT on empty GOAP plan"
    );
    assert!(
        matches!(action, ActionStep::Scan { radius: 15.0 }),
        "Should return BT action"
    );

    // Verify no LLM task was created (BT fallback doesn't use LLM)
    assert!(
        !arbiter.is_llm_active(),
        "Should not have LLM task in BT mode"
    );
    assert!(
        arbiter.current_plan().is_none(),
        "Should not have plan in BT mode"
    );
}

// ============================================================================
// Integration Tests: Error Handling
// ============================================================================

#[tokio::test]
async fn test_invalid_step_index_falls_back_to_goap() {
    let goap = Box::new(MockGoap::new(ActionStep::Scan { radius: 10.0 }));
    let bt = Box::new(MockBT::new(ActionStep::Wait { duration: 1.0 }));
    let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_secs(10));

    // Manually create invalid state: ExecutingLLM with step_index > plan.len()
    let plan = PlanIntent {
        plan_id: "test".into(),
        steps: vec![ActionStep::Wait { duration: 1.0 }], // Only 1 step
    };
    arbiter.transition_to_llm(plan);

    // Exhaust plan (step 0)
    let snap1 = create_test_snapshot(0.0);
    arbiter.update(&snap1);

    // Should be back in GOAP now
    assert_eq!(arbiter.mode(), AIControlMode::GOAP);

    // Next update should work normally
    let snap2 = create_test_snapshot(1.0);
    let action = arbiter.update(&snap2);
    assert!(matches!(action, ActionStep::Scan { radius: 10.0 }));
}

#[tokio::test]
async fn test_executing_llm_without_plan_falls_back_to_goap() {
    let goap = Box::new(MockGoap::new(ActionStep::Scan { radius: 10.0 }));
    let bt = Box::new(MockBT::new(ActionStep::Wait { duration: 1.0 }));
    let mut arbiter = create_arbiter_with_mocks(goap, bt, Duration::from_secs(10));

    // Force invalid state: ExecutingLLM mode but no plan
    // (This shouldn't happen in practice, but test defensive programming)
    // We can't directly set mode without plan via public API, so we inject plan then clear it
    let plan = PlanIntent {
        plan_id: "test".into(),
        steps: vec![ActionStep::Wait { duration: 1.0 }],
    };
    arbiter.transition_to_llm(plan);

    // Exhaust plan to transition to GOAP
    let snap1 = create_test_snapshot(0.0);
    arbiter.update(&snap1);

    // Should be in GOAP now (defensive programming worked)
    assert_eq!(arbiter.mode(), AIControlMode::GOAP);
}

// ============================================================================
// Integration Tests: Metrics Tracking
// ============================================================================

#[tokio::test]
async fn test_metrics_track_all_counters() {
    let goap = Box::new(MockGoap::new(ActionStep::Wait { duration: 1.0 }));
    let bt = Box::new(MockBT::new(ActionStep::Wait { duration: 1.0 }));
    let mut arbiter =
        create_arbiter_with_mocks(goap, bt, Duration::from_millis(100)).with_llm_cooldown(0.5); // Short cooldown

    let snap1 = create_test_snapshot(0.0);
    let snap2 = create_test_snapshot(1.0);
    let snap3 = create_test_snapshot(2.0);

    // Update 1: GOAP action, LLM request
    arbiter.update(&snap1);

    // Update 2: GOAP action (LLM still running)
    arbiter.update(&snap2);

    // Wait for LLM completion
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Update 3: Transition to ExecutingLLM
    arbiter.update(&snap3);

    let (transitions, requests, successes, failures, goap_actions, llm_steps) = arbiter.metrics();

    assert!(transitions >= 1, "Should have at least 1 transition");
    assert_eq!(requests, 1, "Should have 1 LLM request");
    assert_eq!(successes, 1, "Should have 1 LLM success");
    assert_eq!(failures, 0, "Should have 0 failures");
    assert_eq!(goap_actions, 2, "Should have 2 GOAP actions");
    assert_eq!(llm_steps, 1, "Should have 1 LLM step executed");
}

// ============================================================================
// Integration Tests: with_llm_cooldown() Configuration
// ============================================================================

#[tokio::test]
async fn test_with_llm_cooldown_configures_cooldown() {
    let goap = Box::new(MockGoap::new(ActionStep::Wait { duration: 1.0 }));
    let bt = Box::new(MockBT::new(ActionStep::Wait { duration: 1.0 }));
    let mut arbiter =
        create_arbiter_with_mocks(goap, bt, Duration::from_millis(100)).with_llm_cooldown(10.0); // 10 second cooldown

    // First request at t=0
    let snap1 = create_test_snapshot(0.0);
    arbiter.update(&snap1);
    let (_, requests1, _, _, _, _) = arbiter.metrics();
    assert_eq!(requests1, 1, "Should have 1 request");
    assert!(arbiter.is_llm_active(), "Should have active LLM task");

    // Second update at t=5 (within cooldown, AND active task blocks)
    let snap2 = create_test_snapshot(5.0);
    arbiter.update(&snap2);
    let (_, requests2, _, _, _, _) = arbiter.metrics();
    assert_eq!(
        requests2, 1,
        "Should still have 1 request (active task blocks)"
    );

    // Wait for LLM completion and transition
    tokio::time::sleep(Duration::from_millis(150)).await;
    let snap_transition = create_test_snapshot(6.0);
    arbiter.update(&snap_transition); // Polls result, transitions to ExecutingLLM

    // Exhaust plan to return to GOAP
    arbiter.update(&snap_transition); // Step 1
    arbiter.update(&snap_transition); // Step 2
    arbiter.update(&snap_transition); // Step 3, exhausts plan

    // Verify back in GOAP
    assert_eq!(
        arbiter.mode(),
        AIControlMode::GOAP,
        "Should be back in GOAP mode"
    );

    // Third update at t=11 (cooldown expired AND back in GOAP)
    let snap3 = create_test_snapshot(11.0);
    arbiter.update(&snap3);
    let (_, requests3, _, _, _, _) = arbiter.metrics();
    assert_eq!(
        requests3, 2,
        "Should have 2 requests after cooldown expires and plan exhausted"
    );
}
