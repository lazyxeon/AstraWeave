//! Integration tests for LLM orchestration
//!
//! Tests the full pipeline using PUBLIC APIs only
//! Validates end-to-end behavior with real mock LLM client and caching

use astraweave_core::{
    ActionStep, CompanionState, Constraints, EnemyState, IVec2, PlayerState, ToolRegistry,
    ToolSpec, WorldSnapshot,
};
use astraweave_llm::telemetry::LlmTelemetry;
use astraweave_llm::{
    build_prompt, fallback_heuristic_plan, parse_llm_plan, plan_from_llm, AlwaysErrMock, LlmClient,
    PlanSource,
};
use std::sync::Arc;

// ============================================================================
// Helper: Create Test Fixtures
// ============================================================================

fn create_test_registry() -> ToolRegistry {
    // Tool names MUST be PascalCase to match ActionStep enum variants
    // and FallbackOrchestrator's simplified_tools list
    ToolRegistry {
        tools: vec![
            ToolSpec {
                name: "MoveTo".into(),
                args: [("x", "i32"), ("y", "i32")]
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            },
            ToolSpec {
                name: "Attack".into(),
                args: [("target_id", "u32")]
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            },
            ToolSpec {
                name: "Throw".into(), // Legacy alias
                args: [("item", "enum[smoke,grenade]"), ("x", "i32"), ("y", "i32")]
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            },
            ToolSpec {
                name: "CoverFire".into(),
                args: [("target_id", "u32"), ("duration", "f32")]
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            },
            ToolSpec {
                name: "Scan".into(),
                args: [("radius", "f32")]
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            },
            ToolSpec {
                name: "Wait".into(),
                args: [("duration", "f32")]
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            },
        ],
        constraints: Constraints {
            enforce_cooldowns: true,
            enforce_los: true,
            enforce_stamina: true,
        },
    }
}

fn create_test_world_snapshot() -> WorldSnapshot {
    WorldSnapshot {
        t: 1.0,
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 2, y: 2 },
            stance: "stand".into(),
            orders: vec![],
        },
        me: CompanionState {
            ammo: 30,
            cooldowns: Default::default(),
            morale: 0.9,
            pos: IVec2 { x: 3, y: 2 },
        },
        enemies: vec![EnemyState {
            id: 99,
            pos: IVec2 { x: 12, y: 2 },
            hp: 60,
            cover: "low".into(),
            last_seen: 1.0,
        }],
        pois: vec![],
        obstacles: vec![],
        objective: Some("extract".into()),
    
        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}

// Custom mock LLM that returns valid plans
// NOTE: Must only use tools that are in FallbackOrchestrator's simplified_tools list:
// MoveTo, ThrowSmoke, ThrowExplosive, AoEAttack, TakeCover, Attack, Approach, Retreat,
// MarkTarget, Distract, Reload, Scan, Wait, Block, Heal
struct ValidPlanMock;

#[async_trait::async_trait]
impl LlmClient for ValidPlanMock {
    async fn complete(&self, _prompt: &str) -> anyhow::Result<String> {
        Ok(r#"{
            "plan_id": "integration-test-plan",
            "steps": [
                {"act": "MoveTo", "x": 5, "y": 5},
                {"act": "Attack", "target_id": 99}
            ]
        }"#
        .to_string())
    }
}

// ============================================================================
// Test 1: End-to-End Success Path (Valid LLM Response)
// ============================================================================

#[tokio::test]
async fn test_end_to_end_valid_llm_response() {
    let snap = create_test_world_snapshot();
    let reg = create_test_registry();
    let client = ValidPlanMock;

    let result = plan_from_llm(&client, &snap, &reg).await;

    match result {
        PlanSource::Llm(plan) => {
            assert_eq!(plan.plan_id, "integration-test-plan");
            assert_eq!(plan.steps.len(), 2);

            // Verify steps are correct
            match &plan.steps[0] {
                ActionStep::MoveTo { x, y, speed: _ } => {
                    assert_eq!(*x, 5);
                    assert_eq!(*y, 5);
                }
                _ => panic!("Expected MoveTo as first step"),
            }

            match &plan.steps[1] {
                ActionStep::Attack { target_id } => {
                    assert_eq!(*target_id, 99);
                }
                _ => panic!("Expected Attack as second step"),
            }
        }
        PlanSource::Fallback { .. } => panic!("Expected LLM plan, got fallback"),
        _ => panic!("Unexpected PlanSource variant"),
    }
}

// ============================================================================
// Test 2: Fallback on LLM Failure
// ============================================================================

#[tokio::test]
async fn test_fallback_on_llm_failure() {
    // Clear global cache to prevent cross-test pollution
    #[cfg(feature = "llm_cache")]
    astraweave_llm::clear_global_cache();

    // Use unique snapshot to avoid cache collisions with parallel tests
    let mut snap = create_test_world_snapshot();
    snap.t = 888.88; // Unique timestamp
    snap.objective = Some("FALLBACK_TEST_UNIQUE_888".to_string());
    let reg = create_test_registry();
    let client = AlwaysErrMock;

    let result = plan_from_llm(&client, &snap, &reg).await;

    match result {
        PlanSource::Llm(_) => panic!("Expected fallback, got LLM plan"),
        PlanSource::Fallback { plan, reason } => {
            // Phase 7: Plans use UUID-based IDs
            assert!(
                plan.plan_id.starts_with("heuristic-") || plan.plan_id.starts_with("emergency-"),
                "Expected heuristic or emergency plan, got: {}",
                plan.plan_id
            );
            assert!(reason.contains("tier") || reason.contains("attempts"));
        }
        _ => panic!("Unexpected PlanSource variant"),
    }
}

// ============================================================================
// Test 3: Parse Valid Plan
// ============================================================================

#[test]
fn test_parse_valid_plan() {
    let reg = create_test_registry();
    let json = r#"{
        "plan_id": "parse-test",
        "steps": [
            {"act": "MoveTo", "x": 10, "y": 20},
            {"act": "Throw", "item": "smoke", "x": 15, "y": 25}
        ]
    }"#;

    let result = parse_llm_plan(json, &reg);
    assert!(
        result.is_ok(),
        "Failed to parse valid plan: {:?}",
        result.err()
    );

    let plan = result.unwrap();
    assert_eq!(plan.plan_id, "parse-test");
    assert_eq!(plan.steps.len(), 2);
}

// ============================================================================
// Test 4: Fallback Heuristic Plan
// ============================================================================

#[test]
fn test_fallback_heuristic_plan() {
    let snap = create_test_world_snapshot();
    let reg = create_test_registry();

    let plan = fallback_heuristic_plan(&snap, &reg);

    assert_eq!(plan.plan_id, "heuristic-fallback");
    assert!(!plan.steps.is_empty(), "Fallback plan should have steps");

    // Fallback heuristic generates at least one step (actual count varies by game state)
    assert!(!plan.steps.is_empty(), "Should have at least 1 step");
}

// ============================================================================
// Test 5: Telemetry Tracking
// ============================================================================

#[test]
fn test_telemetry_tracking() {
    let telemetry = LlmTelemetry::new();

    // Record operations
    telemetry.record_request();
    telemetry.record_success();
    telemetry.record_cache_hit();

    telemetry.record_request();
    telemetry.record_error();
    telemetry.record_cache_miss();

    telemetry.record_retry();
    telemetry.record_circuit_open();

    // Validate counters
    let snapshot = telemetry.snapshot();

    assert_eq!(snapshot.requests_total, 2);
    assert_eq!(snapshot.requests_success, 1);
    assert_eq!(snapshot.requests_error, 1);
    assert_eq!(snapshot.cache_hits, 1);
    assert_eq!(snapshot.cache_misses, 1);
    assert_eq!(snapshot.retries_attempted, 1);
    assert_eq!(snapshot.circuit_breaker_open, 1);
}

// ============================================================================
// Test 6: Build Prompt Contains Required Elements
// ============================================================================

#[test]
fn test_build_prompt_structure() {
    let snap = create_test_world_snapshot();
    let reg = create_test_registry();

    let prompt = build_prompt(&snap, &reg);

    // Verify prompt contains expected elements
    // Note: Tool names are PascalCase to match ActionStep enum variants
    assert!(prompt.contains("AI game companion planner"));
    assert!(prompt.contains("MoveTo"));
    assert!(prompt.contains("Throw"));
    assert!(prompt.contains("Attack"));
    assert!(prompt.contains("Return ONLY JSON"));
    assert!(prompt.contains("plan_id"));
    assert!(prompt.contains("steps"));
}

// ============================================================================
// Test 7: Invalid JSON Returns Error
// ============================================================================

#[test]
fn test_parse_invalid_json_fails() {
    let reg = create_test_registry();
    let invalid_json = "not valid json";

    let result = parse_llm_plan(invalid_json, &reg);
    assert!(result.is_err(), "Should fail on invalid JSON");
}

// ============================================================================
// Test 8: Disallowed Tool Returns Error
// ============================================================================

#[test]
fn test_parse_plan_with_disallowed_tool() {
    let mut reg = create_test_registry();
    // Remove MoveTo tool (PascalCase to match registry)
    reg.tools.retain(|t| t.name != "MoveTo");

    let json = r#"{
        "plan_id": "test",
        "steps": [{"act": "MoveTo", "x": 5, "y": 5}]
    }"#;

    let result = parse_llm_plan(json, &reg);
    assert!(result.is_err(), "Should fail with disallowed tool");
    assert!(result.unwrap_err().to_string().contains("disallowed"));
}

// ============================================================================
// Test 9: Concurrent Telemetry Updates (Thread Safety)
// ============================================================================

#[test]
fn test_telemetry_thread_safety() {
    use std::thread;

    let telemetry = Arc::new(LlmTelemetry::new());
    let mut handles = vec![];

    // Spawn 10 threads, each recording 100 requests
    for _ in 0..10 {
        let telemetry_clone = Arc::clone(&telemetry);
        let handle = thread::spawn(move || {
            for _ in 0..100 {
                telemetry_clone.record_request();
                telemetry_clone.record_success();
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify total count
    let snapshot = telemetry.snapshot();
    assert_eq!(
        snapshot.requests_total, 1000,
        "Should have 1000 total requests"
    );
    assert_eq!(
        snapshot.requests_success, 1000,
        "Should have 1000 successful requests"
    );
}

// ============================================================================
// Test 10: Multiple LLM Calls (Cache Miss Simulation)
// ============================================================================

#[tokio::test]
async fn test_multiple_llm_calls() {
    let snap = create_test_world_snapshot();
    let reg = create_test_registry();
    let client = ValidPlanMock;

    // Call 3 times (first call = cache miss, subsequent = cache hits if caching enabled)
    for i in 0..3 {
        let result = plan_from_llm(&client, &snap, &reg).await;

        match result {
            PlanSource::Llm(plan) => {
                assert_eq!(plan.plan_id, "integration-test-plan");
                println!("Call {}: LLM plan received", i + 1);
            }
            PlanSource::Fallback { .. } => {
                panic!("Call {}: Expected LLM plan, got fallback", i + 1);
            }
            _ => panic!("Unexpected PlanSource variant"),
        }
    }
}
