//! Integration tests for AIArbiter
//!
//! Tests the full GOAP+LLM hybrid arbiter pattern with:
//! - Mode transitions (GOAP → ExecutingLLM → GOAP)
//! - Non-blocking LLM polling
//! - Plan execution and exhaustion
//! - Error handling and fallbacks
//! - Metrics tracking

#[cfg(feature = "llm_orchestrator")]
mod arbiter_integration_tests {
    use anyhow::{anyhow, Result};
    use astraweave_ai::{AIArbiter, AIControlMode, LlmExecutor, Orchestrator, OrchestratorAsync};
    use astraweave_core::{
        ActionStep, CompanionState, EnemyState, IVec2, PlanIntent, PlayerState, WorldSnapshot,
    };
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    // ========================================================================
    // Mock Orchestrators
    // ========================================================================

    /// Mock GOAP orchestrator that returns predictable plans
    struct MockGoap {
        action_to_return: ActionStep,
    }

    impl Orchestrator for MockGoap {
        fn propose_plan(&self, _snap: &WorldSnapshot) -> PlanIntent {
            PlanIntent {
                plan_id: "mock-goap".into(),
                steps: vec![self.action_to_return.clone()],
            }
        }
    }

    /// Mock BehaviorTree orchestrator (fallback)
    struct MockBT {
        action_to_return: ActionStep,
    }

    impl Orchestrator for MockBT {
        fn propose_plan(&self, _snap: &WorldSnapshot) -> PlanIntent {
            PlanIntent {
                plan_id: "mock-bt".into(),
                steps: vec![self.action_to_return.clone()],
            }
        }
    }

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

    // ========================================================================
    // Test Helpers
    // ========================================================================

    fn create_test_snapshot() -> WorldSnapshot {
        WorldSnapshot {
            t: 1.234,
            player: PlayerState {
                hp: 80,

                pos: IVec2 { x: 0, y: 0 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 10,
                cooldowns: BTreeMap::new(),
                morale: 0.5,
                pos: IVec2 { x: 0, y: 0 },
            },
            enemies: vec![EnemyState {
                id: 2,
                pos: IVec2 { x: 5, y: 0 },
                hp: 50,
                cover: "low".into(),
                last_seen: 0.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        }
    }

    fn create_mock_llm_plan(steps: usize) -> PlanIntent {
        let mut plan_steps = Vec::new();
        for i in 0..steps {
            plan_steps.push(ActionStep::MoveTo {
                speed: None,
                x: i as i32,
                y: 0,
            });
        }

        PlanIntent {
            plan_id: format!("mock-llm-{}", steps),
            steps: plan_steps,
        }
    }

    // ========================================================================
    // Test 1: Initial Mode is GOAP
    // ========================================================================

    #[tokio::test]
    async fn test_arbiter_starts_in_goap_mode() {
        // Setup mock orchestrators
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::MoveTo {
                speed: None,
                x: 1,
                y: 0,
            },
        });

        let bt = Box::new(MockBT {
            action_to_return: ActionStep::Wait { duration: 1.0 },
        });

        // Create mock LLM executor (won't be called in this test)
        let mock_llm = Arc::new(MockLlmOrch::new());
        let runtime = tokio::runtime::Handle::current();
        let llm_executor = LlmExecutor::new(mock_llm, runtime);

        // Create arbiter
        let arbiter = AIArbiter::with_single_executor(llm_executor, goap, bt);

        // Verify initial mode
        assert_eq!(arbiter.mode(), AIControlMode::GOAP);
        assert!(!arbiter.is_llm_active());
        assert!(arbiter.current_plan().is_none());

        // Verify metrics
        let (transitions, requests, successes, failures, goap_actions, llm_steps) =
            arbiter.metrics();
        assert_eq!(transitions, 0);
        assert_eq!(requests, 0);
        assert_eq!(successes, 0);
        assert_eq!(failures, 0);
        assert_eq!(goap_actions, 0);
        assert_eq!(llm_steps, 0);
    }

    // ========================================================================
    // Test 2: GOAP Returns Actions Instantly
    // ========================================================================

    #[tokio::test]
    async fn test_goap_returns_instant_actions() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::MoveTo {
                speed: None,
                x: 2,
                y: 3,
            },
        });

        let bt = Box::new(MockBT {
            action_to_return: ActionStep::Wait { duration: 1.0 },
        });

        let mock_llm = Arc::new(MockLlmOrch::new());
        let runtime = tokio::runtime::Handle::current();
        let llm_executor = LlmExecutor::new(mock_llm, runtime);

        let mut arbiter = AIArbiter::with_single_executor(llm_executor, goap, bt);

        let snap = create_test_snapshot();

        // Call update() - should return GOAP action instantly
        let action = arbiter.update(&snap);

        // Verify action
        match action {
            ActionStep::MoveTo { x, y, .. } => {
                assert_eq!(x, 2);
                assert_eq!(y, 3);
            }
            _ => panic!("Expected MoveTo action"),
        }

        // Verify mode is still GOAP
        assert_eq!(arbiter.mode(), AIControlMode::GOAP);

        // Verify metrics
        let (_, _, _, _, goap_actions, _) = arbiter.metrics();
        assert_eq!(goap_actions, 1);
    }

    // ========================================================================
    // Test 3: LLM Request Spawned After Cooldown
    // ========================================================================

    #[tokio::test]
    async fn test_llm_request_spawned_after_cooldown() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::MoveTo {
                speed: None,
                x: 1,
                y: 0,
            },
        });

        let bt = Box::new(MockBT {
            action_to_return: ActionStep::Wait { duration: 1.0 },
        });

        // Mock LLM that will return a plan after delay
        let mock_llm = Arc::new(
            MockLlmOrch::new()
                .with_plan(create_mock_llm_plan(3))
                .with_delay(50), // 50ms delay
        );

        let runtime = tokio::runtime::Handle::current();
        let llm_executor = LlmExecutor::new(mock_llm, runtime);

        let mut arbiter =
            AIArbiter::with_single_executor(llm_executor, goap, bt).with_llm_cooldown(0.0); // No cooldown for test

        let snap = create_test_snapshot();

        // First update: should spawn LLM request
        let action1 = arbiter.update(&snap);
        assert!(matches!(action1, ActionStep::MoveTo { .. }));
        assert_eq!(arbiter.mode(), AIControlMode::GOAP);

        // Verify LLM request was spawned
        let (_, requests, _, _, _, _) = arbiter.metrics();
        assert_eq!(requests, 1);
        assert!(arbiter.is_llm_active());

        // Second update (before LLM completes): should still be in GOAP mode
        let action2 = arbiter.update(&snap);
        assert!(matches!(action2, ActionStep::MoveTo { .. }));
        assert_eq!(arbiter.mode(), AIControlMode::GOAP);

        // Wait for LLM to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Third update (after LLM completes): should transition to ExecutingLLM
        let action3 = arbiter.update(&snap);
        assert!(matches!(action3, ActionStep::MoveTo { .. }));
        assert!(matches!(
            arbiter.mode(),
            AIControlMode::ExecutingLLM { step_index: 1 }
        ));

        // Verify metrics
        let (transitions, _, successes, _, _, _) = arbiter.metrics();
        assert_eq!(transitions, 1); // GOAP → ExecutingLLM
        assert_eq!(successes, 1);
    }

    // ========================================================================
    // Test 4: Plan Execution and Exhaustion
    // ========================================================================

    #[tokio::test]
    async fn test_plan_execution_and_exhaustion() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Wait { duration: 0.5 },
        });

        let bt = Box::new(MockBT {
            action_to_return: ActionStep::Wait { duration: 1.0 },
        });

        let mock_llm = Arc::new(MockLlmOrch::new().with_plan(create_mock_llm_plan(3)));

        let runtime = tokio::runtime::Handle::current();
        let llm_executor = LlmExecutor::new(mock_llm.clone(), runtime);

        let mut arbiter =
            AIArbiter::with_single_executor(llm_executor, goap, bt).with_llm_cooldown(0.0);

        let snap = create_test_snapshot();

        // Manually transition to ExecutingLLM by providing a plan
        let plan = create_mock_llm_plan(3);
        arbiter.transition_to_llm(plan);

        // Verify mode
        assert_eq!(
            arbiter.mode(),
            AIControlMode::ExecutingLLM { step_index: 0 }
        );

        // Execute step 0
        let action0 = arbiter.update(&snap);
        assert!(matches!(action0, ActionStep::MoveTo { x: 0, .. }));
        assert_eq!(
            arbiter.mode(),
            AIControlMode::ExecutingLLM { step_index: 1 }
        );

        // Execute step 1
        let action1 = arbiter.update(&snap);
        assert!(matches!(action1, ActionStep::MoveTo { x: 1, .. }));
        assert_eq!(
            arbiter.mode(),
            AIControlMode::ExecutingLLM { step_index: 2 }
        );

        // Execute step 2 (last step)
        let action2 = arbiter.update(&snap);
        assert!(matches!(action2, ActionStep::MoveTo { x: 2, .. }));

        // Should transition back to GOAP after last step
        assert_eq!(arbiter.mode(), AIControlMode::GOAP);

        // Verify metrics
        let (transitions, _, _, _, _, llm_steps) = arbiter.metrics();
        assert_eq!(transitions, 2); // Manual transition + auto transition back
        assert_eq!(llm_steps, 3);
    }

    // ========================================================================
    // Test 5: LLM Failure Falls Back to GOAP
    // ========================================================================

    #[tokio::test]
    async fn test_llm_failure_fallback() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::MoveTo {
                speed: None,
                x: 5,
                y: 5,
            },
        });

        let bt = Box::new(MockBT {
            action_to_return: ActionStep::Wait { duration: 1.0 },
        });

        // Mock LLM configured to fail (no plan set)
        let mock_llm = Arc::new(MockLlmOrch::new().with_delay(50));

        let runtime = tokio::runtime::Handle::current();
        let llm_executor = LlmExecutor::new(mock_llm, runtime);

        let mut arbiter =
            AIArbiter::with_single_executor(llm_executor, goap, bt).with_llm_cooldown(0.0);

        let snap = create_test_snapshot();

        // First update: spawn LLM request
        let _ = arbiter.update(&snap);
        assert!(arbiter.is_llm_active());

        // Wait for LLM to "complete" (fail)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Second update: should detect failure, stay in GOAP, and spawn new request
        let action = arbiter.update(&snap);
        assert!(matches!(action, ActionStep::MoveTo { x: 5, y: 5, .. }));
        assert_eq!(arbiter.mode(), AIControlMode::GOAP);

        // Verify metrics
        // Note: After detecting failure, maybe_request_llm spawns a new request
        // So we get: 1 initial + 1 after failure = 2 total requests
        let (_, requests, successes, failures, _, _) = arbiter.metrics();
        assert_eq!(requests, 2); // Initial request + retry after failure
        assert_eq!(successes, 0);
        assert_eq!(failures, 1);
    }

    // ========================================================================
    // Test 6: Cooldown Prevents Redundant Requests
    // ========================================================================

    #[tokio::test]
    async fn test_cooldown_prevents_redundant_requests() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Wait { duration: 1.0 },
        });

        let bt = Box::new(MockBT {
            action_to_return: ActionStep::Wait { duration: 1.0 },
        });

        let mock_llm = Arc::new(
            MockLlmOrch::new()
                .with_plan(create_mock_llm_plan(1)) // 1-step plan (executes quickly)
                .with_delay(50), // Fast completion
        );

        let runtime = tokio::runtime::Handle::current();
        let llm_executor = LlmExecutor::new(mock_llm, runtime);

        let mut arbiter =
            AIArbiter::with_single_executor(llm_executor, goap, bt).with_llm_cooldown(5.0); // 5s cooldown

        let mut snap = create_test_snapshot();

        // First update at t=0: should spawn LLM request
        snap.t = 0.0;
        let _ = arbiter.update(&snap);
        let (_, requests1, _, _, _, _) = arbiter.metrics();
        assert_eq!(requests1, 1);

        // Wait for LLM to complete and transition to ExecutingLLM
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Execute the 1-step plan (transitions back to GOAP)
        snap.t = 0.1;
        let _ = arbiter.update(&snap);
        assert_eq!(arbiter.mode(), AIControlMode::GOAP); // Back in GOAP after plan exhaustion

        // Second update at t=1: should NOT spawn (cooldown not expired: 1.0 - 0.0 = 1.0 < 5.0)
        snap.t = 1.0;
        let _ = arbiter.update(&snap);
        let (_, requests2, _, _, _, _) = arbiter.metrics();
        assert_eq!(requests2, 1); // Same as before

        // Third update at t=6: should spawn new request (cooldown expired: 6.0 - 0.0 = 6.0 > 5.0)
        snap.t = 6.0;
        let _ = arbiter.update(&snap);
        let (_, requests3, _, _, _, _) = arbiter.metrics();
        assert_eq!(requests3, 2); // New request
    }

    // ========================================================================
    // Test 7: Mode Display Formatting
    // ========================================================================

    #[test]
    fn test_mode_display_formatting() {
        assert_eq!(format!("{}", AIControlMode::GOAP), "GOAP");
        assert_eq!(
            format!("{}", AIControlMode::ExecutingLLM { step_index: 5 }),
            "ExecutingLLM[step 5]"
        );
        assert_eq!(format!("{}", AIControlMode::BehaviorTree), "BehaviorTree");
    }

    // ========================================================================
    // Test 8: Empty GOAP Plan Falls Back to BehaviorTree
    // ========================================================================

    #[tokio::test]
    async fn test_empty_goap_plan_fallback() {
        // Mock GOAP that returns empty plan
        struct EmptyGoap;
        impl Orchestrator for EmptyGoap {
            fn propose_plan(&self, _snap: &WorldSnapshot) -> PlanIntent {
                PlanIntent {
                    plan_id: "empty".into(),
                    steps: vec![],
                }
            }
        }

        let goap = Box::new(EmptyGoap);
        let bt = Box::new(MockBT {
            action_to_return: ActionStep::Scan { radius: 10.0 },
        });

        let mock_llm = Arc::new(MockLlmOrch::new());
        let runtime = tokio::runtime::Handle::current();
        let llm_executor = LlmExecutor::new(mock_llm, runtime);

        let mut arbiter = AIArbiter::with_single_executor(llm_executor, goap, bt);

        let snap = create_test_snapshot();

        // Update should fall back to BehaviorTree
        let action = arbiter.update(&snap);
        assert!(matches!(action, ActionStep::Scan { radius: 10.0 }));
        assert_eq!(arbiter.mode(), AIControlMode::BehaviorTree);
    }

    // ========================================================================
    // Test 9: Concurrent Updates Don't Break State
    // ========================================================================

    #[tokio::test]
    async fn test_concurrent_updates() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Wait { duration: 0.1 },
        });

        let bt = Box::new(MockBT {
            action_to_return: ActionStep::Wait { duration: 1.0 },
        });

        let mock_llm = Arc::new(MockLlmOrch::new().with_plan(create_mock_llm_plan(5)));

        let runtime = tokio::runtime::Handle::current();
        let llm_executor = LlmExecutor::new(mock_llm, runtime);

        let mut arbiter =
            AIArbiter::with_single_executor(llm_executor, goap, bt).with_llm_cooldown(0.0);

        let snap = create_test_snapshot();

        // Rapid-fire updates (simulating high frame rate)
        for _ in 0..100 {
            let action = arbiter.update(&snap);
            // All updates should return valid actions (no panics)
            assert!(matches!(
                action,
                ActionStep::Wait { .. } | ActionStep::MoveTo { .. }
            ));
        }

        // Verify arbiter still functional
        let final_mode = arbiter.mode();
        assert!(matches!(
            final_mode,
            AIControlMode::GOAP | AIControlMode::ExecutingLLM { .. } | AIControlMode::BehaviorTree
        ));
    }

    // ========================================================================
    // Test 10: Metrics Accuracy
    // ========================================================================

    #[tokio::test]
    async fn test_metrics_accuracy() {
        let goap = Box::new(MockGoap {
            action_to_return: ActionStep::Wait { duration: 1.0 },
        });

        let bt = Box::new(MockBT {
            action_to_return: ActionStep::Wait { duration: 1.0 },
        });

        let mock_llm = Arc::new(
            MockLlmOrch::new()
                .with_plan(create_mock_llm_plan(4))
                .with_delay(50),
        );

        let runtime = tokio::runtime::Handle::current();
        let llm_executor = LlmExecutor::new(mock_llm.clone(), runtime);

        let mut arbiter =
            AIArbiter::with_single_executor(llm_executor, goap, bt).with_llm_cooldown(0.0);

        let snap = create_test_snapshot();

        // Scenario: 3 GOAP actions → LLM completes → 4 LLM steps → back to GOAP

        // Phase 1: GOAP actions (LLM request spawned on first update)
        for _ in 0..3 {
            let _ = arbiter.update(&snap);
        }

        // Wait for LLM
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Phase 2: LLM plan execution (4 steps)
        for _ in 0..4 {
            let _ = arbiter.update(&snap);
        }

        // Verify final state: back in GOAP
        assert_eq!(arbiter.mode(), AIControlMode::GOAP);

        // Verify metrics
        let (transitions, requests, successes, failures, goap_actions, llm_steps) =
            arbiter.metrics();

        assert_eq!(requests, 1); // One LLM request
        assert_eq!(successes, 1); // One successful plan
        assert_eq!(failures, 0); // No failures
        assert_eq!(transitions, 2); // GOAP → ExecutingLLM → GOAP
        assert_eq!(goap_actions, 3); // Three GOAP actions before LLM
        assert_eq!(llm_steps, 4); // Four LLM steps executed
    }
}
