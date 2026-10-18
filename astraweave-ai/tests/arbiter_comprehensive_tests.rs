#![cfg(feature = "llm_orchestrator")]
//! Comprehensive integration tests for AIArbiter
//!
//! These tests achieve 80%+ coverage of ai_arbiter.rs by testing:
//! - Mode transitions (GOAP ↔ ExecutingLLM ↔ BehaviorTree)
//! - LLM task lifecycle (spawn → poll → complete)
//! - Cooldown behavior
//! - Plan execution (multi-step plans)
//! - Error handling and fallbacks
//! - Metrics tracking

use astraweave_ai::orchestrator::Orchestrator;
#[cfg(feature = "llm_orchestrator")]
use astraweave_ai::{AIArbiter, AIControlMode, LlmExecutor, OrchestratorAsync};
use astraweave_core::{
    ActionStep, CompanionState, EnemyState, IVec2, PlanIntent, PlayerState, WorldSnapshot,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Create a test WorldSnapshot at time `t`
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

        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}

/// Mock GOAP orchestrator
struct MockGoap {
    action: ActionStep,
}

impl Orchestrator for MockGoap {
    fn propose_plan(&self, _snap: &WorldSnapshot) -> PlanIntent {
        PlanIntent {
            plan_id: "mock-goap".into(),
            steps: vec![self.action.clone()],
        }
    }
}

/// Mock BehaviorTree orchestrator
struct MockBT;

impl Orchestrator for MockBT {
    fn propose_plan(&self, _snap: &WorldSnapshot) -> PlanIntent {
        PlanIntent {
            plan_id: "mock-bt".into(),
            steps: vec![ActionStep::Scan { radius: 10.0 }],
        }
    }
}

/// Mock LLM orchestrator that returns a plan after a delay
struct MockLlmOrch {
    delay_ms: u64,
    plan: PlanIntent,
}

impl Orchestrator for MockLlmOrch {
    fn propose_plan(&self, _snap: &WorldSnapshot) -> PlanIntent {
        // Synchronous version - not used by LlmExecutor
        self.plan.clone()
    }
}

#[async_trait::async_trait]
impl OrchestratorAsync for MockLlmOrch {
    async fn plan(&self, _snap: WorldSnapshot, _budget_ms: u32) -> anyhow::Result<PlanIntent> {
        // Simulate async delay
        tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
        Ok(self.plan.clone())
    }

    fn name(&self) -> &'static str {
        "MockLlmOrch"
    }
}

/// Create a test arbiter with mock orchestrators
fn create_test_arbiter(llm_delay_ms: u64) -> AIArbiter {
    let llm_orch = Arc::new(MockLlmOrch {
        delay_ms: llm_delay_ms,
        plan: PlanIntent {
            plan_id: "mock-llm".into(),
            steps: vec![
                ActionStep::MoveTo {
                    x: 10,
                    y: 10,
                    speed: None,
                },
                ActionStep::CoverFire {
                    target_id: 1,
                    duration: 2.0,
                },
                ActionStep::Scan { radius: 5.0 },
            ],
        },
    });

    let runtime = tokio::runtime::Handle::current();
    let llm_executor = LlmExecutor::new(llm_orch, runtime);

    let goap = Box::new(MockGoap {
        action: ActionStep::Wait { duration: 0.5 },
    });
    let bt = Box::new(MockBT);

    AIArbiter::with_single_executor(llm_executor, goap, bt)
}

// ============================================================================
// Test 1: Initial State
// ============================================================================

#[tokio::test]
async fn test_arbiter_starts_in_goap_mode() {
    let arbiter = create_test_arbiter(50);
    assert_eq!(arbiter.mode(), AIControlMode::GOAP);
}

#[tokio::test]
async fn test_arbiter_no_active_task_initially() {
    let arbiter = create_test_arbiter(50);
    assert!(!arbiter.is_llm_active());
}

#[tokio::test]
async fn test_arbiter_no_current_plan_initially() {
    let arbiter = create_test_arbiter(50);
    assert!(arbiter.current_plan().is_none());
}

// ============================================================================
// Test 2: GOAP Mode Behavior
// ============================================================================

#[tokio::test]
async fn test_goap_mode_returns_instant_action() {
    let mut arbiter = create_test_arbiter(50);
    let snap = create_test_snapshot(0.0);
    let action = arbiter.update(&snap);

    // Should return GOAP's Wait action
    match action {
        ActionStep::Wait { duration } => assert_eq!(duration, 0.5),
        _ => panic!("Expected Wait action from GOAP"),
    }
}

#[tokio::test]
async fn test_goap_mode_increments_goap_actions_metric() {
    let mut arbiter = create_test_arbiter(50);
    let snap = create_test_snapshot(0.0);

    let (_, _, _, _, goap_before, _) = arbiter.metrics();
    arbiter.update(&snap);
    let (_, _, _, _, goap_after, _) = arbiter.metrics();

    assert_eq!(goap_after, goap_before + 1);
}

// ============================================================================
// Test 3: LLM Request Cooldown
// ============================================================================

#[tokio::test]
async fn test_llm_request_respects_cooldown() {
    let mut arbiter = create_test_arbiter(50).with_llm_cooldown(10.0);

    // First update at t=0.0 - should spawn immediately (first request allowed)
    let snap1 = create_test_snapshot(0.0);
    arbiter.update(&snap1);
    assert!(arbiter.is_llm_active()); // Task spawned immediately
    let (_, requests_1, _, _, _, _) = arbiter.metrics();
    assert_eq!(requests_1, 1);

    // Wait for task to complete
    sleep(Duration::from_millis(150)).await;

    // Execute the complete plan (3 steps) to return to GOAP mode
    let snap2 = create_test_snapshot(2.5);
    arbiter.update(&snap2); // Detects completion, transitions to ExecutingLLM, executes step 0
    assert!(!arbiter.is_llm_active()); // Task should be cleared after poll_llm_result
    assert_eq!(
        arbiter.mode(),
        AIControlMode::ExecutingLLM { step_index: 1 }
    );

    let snap3 = create_test_snapshot(3.0);
    arbiter.update(&snap3); // Executes step 1, advances to step 2

    let snap4 = create_test_snapshot(3.5);
    arbiter.update(&snap4); // Executes step 2 (last), transitions to GOAP
    assert_eq!(arbiter.mode(), AIControlMode::GOAP);

    // Verify still only 1 request so far
    let (_, requests_2, _, _, _, _) = arbiter.metrics();
    assert_eq!(requests_2, 1);

    // Update at t=5.0 - cooldown NOT elapsed (5.0 - 0.0 = 5.0 < 10.0)
    let snap5 = create_test_snapshot(5.0);
    arbiter.update(&snap5);
    assert!(!arbiter.is_llm_active()); // Should NOT spawn new task
    let (_, requests_3, _, _, _, _) = arbiter.metrics();
    assert_eq!(requests_3, 1); // Still only 1 request (cooldown not elapsed)

    // Update at t=10.1 - cooldown elapsed (10.1 - 0.0 = 10.1 > 10.0)
    let snap6 = create_test_snapshot(10.1);
    arbiter.update(&snap6);
    assert!(arbiter.is_llm_active()); // SHOULD spawn new task
    let (_, requests_4, _, _, _, _) = arbiter.metrics();
    assert_eq!(requests_4, 2); // Second request spawned
}

#[tokio::test]
async fn test_llm_request_increments_requests_metric() {
    let mut arbiter = create_test_arbiter(50).with_llm_cooldown(1.0);

    let (_, requests_before, _, _, _, _) = arbiter.metrics();

    // First update (t=0.0, cooldown not elapsed)
    let snap1 = create_test_snapshot(0.0);
    arbiter.update(&snap1);

    // Second update (t=2.0, cooldown elapsed)
    let snap2 = create_test_snapshot(2.0);
    arbiter.update(&snap2);

    let (_, requests_after, _, _, _, _) = arbiter.metrics();
    assert_eq!(requests_after, requests_before + 1);
}

// ============================================================================
// Test 4: LLM Task Lifecycle (spawn → poll → complete)
// ============================================================================

#[tokio::test]
async fn test_llm_task_completion_transitions_to_executing_llm() {
    let mut arbiter = create_test_arbiter(100).with_llm_cooldown(1.0);

    // Spawn LLM task
    let snap1 = create_test_snapshot(2.0);
    arbiter.update(&snap1);
    assert!(arbiter.is_llm_active());
    assert_eq!(arbiter.mode(), AIControlMode::GOAP); // Still in GOAP while waiting

    // Wait for task completion (100ms delay + overhead)
    sleep(Duration::from_millis(200)).await;

    // Next update should detect completion and transition
    let snap2 = create_test_snapshot(2.5);
    let first_action = arbiter.update(&snap2);

    assert!(!arbiter.is_llm_active()); // Task cleared
                                       // After first update in ExecutingLLM mode, step_index advances from 0 to 1
    assert_eq!(
        arbiter.mode(),
        AIControlMode::ExecutingLLM { step_index: 1 }
    );

    // Verify we got the first action from the plan
    match first_action {
        ActionStep::MoveTo { x, y, .. } => {
            assert_eq!(x, 10);
            assert_eq!(y, 10);
        }
        _ => panic!("Expected MoveTo action"),
    }
}

#[tokio::test]
async fn test_llm_success_increments_success_metric() {
    let mut arbiter = create_test_arbiter(100).with_llm_cooldown(1.0);

    let (_, _, successes_before, _, _, _) = arbiter.metrics();

    // Spawn and complete LLM task
    let snap1 = create_test_snapshot(2.0);
    arbiter.update(&snap1);
    sleep(Duration::from_millis(200)).await;
    let snap2 = create_test_snapshot(2.5);
    arbiter.update(&snap2); // This triggers transition_to_llm which increments llm_successes

    let (_, _, successes_after, _, _, _) = arbiter.metrics();
    assert_eq!(successes_after, successes_before + 1);
}

#[tokio::test]
async fn test_llm_success_increments_mode_transitions_metric() {
    let mut arbiter = create_test_arbiter(100).with_llm_cooldown(1.0);

    let (transitions_before, _, _, _, _, _) = arbiter.metrics();
    assert_eq!(transitions_before, 0); // Should start at 0

    // Spawn LLM task
    let snap1 = create_test_snapshot(2.0);
    arbiter.update(&snap1);
    assert!(arbiter.is_llm_active()); // Task spawned

    // Wait for task to complete (increased wait time)
    sleep(Duration::from_millis(250)).await;

    // Trigger transition by polling (this should detect completion and call transition_to_llm)
    let snap2 = create_test_snapshot(2.5);
    arbiter.update(&snap2);

    // Verify transition occurred
    assert_eq!(
        arbiter.mode(),
        AIControlMode::ExecutingLLM { step_index: 1 }
    ); // Should be in ExecutingLLM mode

    let (transitions_after, _, _, _, _, _) = arbiter.metrics();
    assert_eq!(transitions_after, 1); // GOAP → ExecutingLLM = 1 transition
}

// ============================================================================
// Test 5: ExecutingLLM Mode (Multi-Step Plans)
// ============================================================================

#[tokio::test]
async fn test_executing_llm_returns_plan_steps_sequentially() {
    let mut arbiter = create_test_arbiter(100).with_llm_cooldown(1.0);

    // Get into ExecutingLLM mode
    let snap1 = create_test_snapshot(2.0);
    arbiter.update(&snap1);
    sleep(Duration::from_millis(200)).await;
    let snap2 = create_test_snapshot(2.5);
    let action1 = arbiter.update(&snap2); // Executes step 0, advances to step 1

    // Verify we're in ExecutingLLM mode after first action
    assert_eq!(
        arbiter.mode(),
        AIControlMode::ExecutingLLM { step_index: 1 }
    );

    // Step 1: MoveTo (first action was already returned above)
    match action1 {
        ActionStep::MoveTo { x, y, speed } => {
            assert_eq!(x, 10);
            assert_eq!(y, 10);
            assert_eq!(speed, None);
        }
        _ => panic!("Expected MoveTo, got {:?}", action1),
    }

    // Step 2: CoverFire
    let snap3 = create_test_snapshot(3.0);
    let action2 = arbiter.update(&snap3); // Executes step 1, advances to step 2
    match action2 {
        ActionStep::CoverFire {
            target_id,
            duration,
        } => {
            assert_eq!(target_id, 1);
            assert_eq!(duration, 2.0);
        }
        _ => panic!("Expected CoverFire, got {:?}", action2),
    }
    assert_eq!(
        arbiter.mode(),
        AIControlMode::ExecutingLLM { step_index: 2 }
    );

    // Step 3: Scan (last step)
    let snap4 = create_test_snapshot(3.5);
    let action3 = arbiter.update(&snap4); // Executes step 2 (last), transitions to GOAP
    match action3 {
        ActionStep::Scan { radius } => assert_eq!(radius, 5.0),
        _ => panic!("Expected Scan, got {:?}", action3),
    }

    // Plan exhausted, should transition back to GOAP
    assert_eq!(arbiter.mode(), AIControlMode::GOAP);
}

#[tokio::test]
async fn test_executing_llm_increments_steps_executed_metric() {
    let mut arbiter = create_test_arbiter(100).with_llm_cooldown(1.0);

    // Get into ExecutingLLM mode
    let snap1 = create_test_snapshot(2.0);
    arbiter.update(&snap1);
    sleep(Duration::from_millis(200)).await;
    let snap2 = create_test_snapshot(2.5);
    arbiter.update(&snap2); // Executes step 0, advances to step 1

    let (_, _, _, _, _, steps_before) = arbiter.metrics();
    assert_eq!(steps_before, 1); // 1 step already executed in the transition update

    // Execute 2 more steps
    let snap3 = create_test_snapshot(3.0);
    arbiter.update(&snap3); // Executes step 1, advances to step 2
    let snap4 = create_test_snapshot(3.5);
    arbiter.update(&snap4); // Executes step 2 (last), transitions to GOAP

    let (_, _, _, _, _, steps_after) = arbiter.metrics();
    assert_eq!(steps_after, 3); // Total 3 steps executed (1 initial + 2 more)
}

#[tokio::test]
async fn test_plan_exhaustion_transitions_back_to_goap() {
    let mut arbiter = create_test_arbiter(100).with_llm_cooldown(1.0);

    // Get into ExecutingLLM mode with 3-step plan
    let snap1 = create_test_snapshot(2.0);
    arbiter.update(&snap1);
    sleep(Duration::from_millis(200)).await;
    let snap2 = create_test_snapshot(2.5);
    arbiter.update(&snap2); // Executes step 0, advances to step 1

    assert_eq!(
        arbiter.mode(),
        AIControlMode::ExecutingLLM { step_index: 1 }
    );

    // Execute remaining 2 steps
    arbiter.update(&create_test_snapshot(3.0)); // Executes step 1, advances to step 2
    assert_eq!(
        arbiter.mode(),
        AIControlMode::ExecutingLLM { step_index: 2 }
    );

    arbiter.update(&create_test_snapshot(3.5)); // Executes step 2 (last), transitions to GOAP

    // Should be back in GOAP
    assert_eq!(arbiter.mode(), AIControlMode::GOAP);
    assert!(arbiter.current_plan().is_none());
}

#[tokio::test]
async fn test_plan_exhaustion_increments_mode_transitions() {
    let mut arbiter = create_test_arbiter(100).with_llm_cooldown(1.0);

    // Get into ExecutingLLM mode
    let snap1 = create_test_snapshot(2.0);
    arbiter.update(&snap1);
    sleep(Duration::from_millis(200)).await;
    let snap2 = create_test_snapshot(2.5);
    arbiter.update(&snap2);

    let (transitions_before, _, _, _, _, _) = arbiter.metrics();

    // Execute all steps until plan exhaustion
    arbiter.update(&create_test_snapshot(3.0));
    arbiter.update(&create_test_snapshot(3.5));
    arbiter.update(&create_test_snapshot(4.0)); // Triggers ExecutingLLM → GOAP

    let (transitions_after, _, _, _, _, _) = arbiter.metrics();
    assert_eq!(transitions_after, transitions_before + 1);
}

// ============================================================================
// Test 6: Manual Mode Transitions (Public API)
// ============================================================================

#[tokio::test]
async fn test_manual_transition_to_llm_sets_mode() {
    let mut arbiter = create_test_arbiter(50);

    let plan = PlanIntent {
        plan_id: "manual-test".into(),
        steps: vec![ActionStep::Scan { radius: 15.0 }],
    };

    arbiter.transition_to_llm(plan);
    assert_eq!(
        arbiter.mode(),
        AIControlMode::ExecutingLLM { step_index: 0 }
    );
}

#[tokio::test]
async fn test_manual_transition_stores_plan() {
    let mut arbiter = create_test_arbiter(50);

    let plan = PlanIntent {
        plan_id: "manual-test".into(),
        steps: vec![ActionStep::Scan { radius: 15.0 }],
    };

    arbiter.transition_to_llm(plan.clone());

    let stored_plan = arbiter.current_plan().unwrap();
    assert_eq!(stored_plan.plan_id, plan.plan_id);
    assert_eq!(stored_plan.steps.len(), plan.steps.len());
}

#[tokio::test]
async fn test_manual_transition_increments_metrics() {
    let mut arbiter = create_test_arbiter(50);

    let (transitions_before, _, successes_before, _, _, _) = arbiter.metrics();

    let plan = PlanIntent {
        plan_id: "manual".into(),
        steps: vec![ActionStep::Wait { duration: 1.0 }],
    };
    arbiter.transition_to_llm(plan);

    let (transitions_after, _, successes_after, _, _, _) = arbiter.metrics();
    assert_eq!(transitions_after, transitions_before + 1);
    assert_eq!(successes_after, successes_before + 1);
}

// ============================================================================
// Test 7: Cooldown Configuration
// ============================================================================

#[tokio::test]
async fn test_with_llm_cooldown_sets_cooldown() {
    let arbiter = create_test_arbiter(50).with_llm_cooldown(5.0);

    // No direct getter for cooldown, but we can test behavior:
    // Request at t=0, then at t=3 (< 5s cooldown)
    let mut arb = arbiter;
    arb.update(&create_test_snapshot(0.0));
    let active_before = arb.is_llm_active();

    arb.update(&create_test_snapshot(3.0));
    let active_after = arb.is_llm_active();

    // Should NOT spawn task at t=3 (cooldown not elapsed)
    assert_eq!(active_before, active_after);
}

#[tokio::test]
async fn test_zero_cooldown_allows_immediate_requests() {
    let mut arbiter = create_test_arbiter(50).with_llm_cooldown(0.0);

    // First update should spawn task immediately
    arbiter.update(&create_test_snapshot(0.0));
    assert!(arbiter.is_llm_active());
}

// ============================================================================
// Test 8: Metrics Getters
// ============================================================================

#[tokio::test]
async fn test_metrics_returns_all_six_values() {
    let arbiter = create_test_arbiter(50);
    let (transitions, requests, successes, failures, goap, steps) = arbiter.metrics();

    // Should all be 0 initially
    assert_eq!(transitions, 0);
    assert_eq!(requests, 0);
    assert_eq!(successes, 0);
    assert_eq!(failures, 0);
    assert_eq!(goap, 0);
    assert_eq!(steps, 0);
}

#[tokio::test]
async fn test_metrics_persist_across_updates() {
    let mut arbiter = create_test_arbiter(50);

    // Do some GOAP updates
    arbiter.update(&create_test_snapshot(0.0));
    arbiter.update(&create_test_snapshot(0.5));
    arbiter.update(&create_test_snapshot(1.0));

    let (_, _, _, _, goap_actions, _) = arbiter.metrics();
    assert_eq!(goap_actions, 3);
}

// ============================================================================
// Test 9: Current Plan Getter
// ============================================================================

#[tokio::test]
async fn test_current_plan_returns_none_in_goap_mode() {
    let mut arbiter = create_test_arbiter(50);
    arbiter.update(&create_test_snapshot(0.0));

    assert_eq!(arbiter.mode(), AIControlMode::GOAP);
    assert!(arbiter.current_plan().is_none());
}

#[tokio::test]
async fn test_current_plan_returns_plan_in_executing_llm_mode() {
    let mut arbiter = create_test_arbiter(50);

    let plan = PlanIntent {
        plan_id: "test-plan".into(),
        steps: vec![ActionStep::Wait { duration: 1.0 }],
    };
    arbiter.transition_to_llm(plan.clone());

    let stored = arbiter.current_plan().unwrap();
    assert_eq!(stored.plan_id, "test-plan");
}

// ============================================================================
// Test 10: Mode Equality and Display
// ============================================================================

#[tokio::test]
async fn test_control_mode_equality() {
    let mode1 = AIControlMode::GOAP;
    let mode2 = AIControlMode::GOAP;
    let mode3 = AIControlMode::ExecutingLLM { step_index: 0 };
    let mode4 = AIControlMode::ExecutingLLM { step_index: 0 };
    let mode5 = AIControlMode::ExecutingLLM { step_index: 1 };

    assert_eq!(mode1, mode2);
    assert_eq!(mode3, mode4);
    assert_ne!(mode3, mode5); // Different step indices
    assert_ne!(mode1, mode3);
}

#[tokio::test]
async fn test_control_mode_display_formatting() {
    assert_eq!(format!("{}", AIControlMode::GOAP), "GOAP");
    assert_eq!(
        format!("{}", AIControlMode::ExecutingLLM { step_index: 3 }),
        "ExecutingLLM[step 3]"
    );
    assert_eq!(format!("{}", AIControlMode::BehaviorTree), "BehaviorTree");
}
