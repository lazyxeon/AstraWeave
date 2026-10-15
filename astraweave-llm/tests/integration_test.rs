use astraweave_core::{
    CompanionState, Constraints, EnemyState, IVec2, PlayerState, ToolRegistry, ToolSpec,
    WorldSnapshot,
};
use astraweave_llm::{parse_llm_plan, plan_from_llm, MockLlm, PlanSource};

/// Integration test for end-to-end LLM workflow
#[tokio::test]
async fn test_llm_integration_workflow() {
    let world_snapshot = create_complex_scenario();
    let tool_registry = create_comprehensive_registry();

    // Test that MockLlm produces valid output
    let client = MockLlm;
    let plan_source = plan_from_llm(&client, &world_snapshot, &tool_registry).await;
    let plan = match plan_source {
        PlanSource::Llm(p) => p,
        PlanSource::Fallback { .. } => panic!("Expected LLM plan"),
        _ => panic!("Unexpected PlanSource variant"),
    };

    // Verify plan structure
    assert!(!plan.plan_id.is_empty());
    assert!(!plan.steps.is_empty());

    // Verify all steps are valid according to registry
    for step in &plan.steps {
        match step {
            astraweave_core::ActionStep::MoveTo { .. } => {
                assert!(tool_registry.tools.iter().any(|t| t.name == "MoveTo"));
            }
            astraweave_core::ActionStep::ThrowSmoke { .. } => {
                assert!(tool_registry.tools.iter().any(|t| t.name == "ThrowSmoke"));
            }
            astraweave_core::ActionStep::Attack { .. } => {
                assert!(tool_registry.tools.iter().any(|t| t.name == "Attack"));
            }
            astraweave_core::ActionStep::Revive { .. } => {
                assert!(tool_registry.tools.iter().any(|t| t.name == "Revive"));
            }
            _ => {} // Accept all other valid actions from Phase 7 expansion
        }
    }

    // Test that the plan can be serialized back to JSON
    let json_output = serde_json::to_string(&plan).expect("Plan should be serializable");

    // Test that serialized plan can be parsed back
    let reparsed_plan =
        parse_llm_plan(&json_output, &tool_registry).expect("Serialized plan should be parsable");

    assert_eq!(plan.plan_id, reparsed_plan.plan_id);
    assert_eq!(plan.steps.len(), reparsed_plan.steps.len());
}

/// Test that the prompt generation includes all necessary context
#[test]
fn test_prompt_generation_comprehensive() {
    let world_snapshot = create_complex_scenario();
    let tool_registry = create_comprehensive_registry();

    let prompt = astraweave_llm::build_prompt(&world_snapshot, &tool_registry);

    // Verify prompt contains world state
    assert!(prompt.contains("85")); // Player HP
    assert!(prompt.contains("25")); // Companion ammo
    assert!(prompt.contains("enemies")); // Enemy references - should be plural
    assert!(prompt.contains("extract")); // Objective

    // Verify prompt contains tool specifications (Phase 7: PascalCase tools)
    assert!(prompt.contains("MoveTo"));
    assert!(prompt.contains("ThrowSmoke"));
    assert!(prompt.contains("Attack"));
    assert!(prompt.contains("Revive"));

    // Verify prompt contains constraints info
    assert!(prompt.contains("engine will validate"));
    assert!(prompt.contains("cooldown"));
    assert!(prompt.contains("LOS"));

    // Verify JSON schema is present
    assert!(prompt.contains("JSON"));
    assert!(prompt.contains("plan_id"));
    assert!(prompt.contains("steps"));
}

/// Test error handling for various invalid scenarios
#[tokio::test]
async fn test_error_handling_scenarios() {
    // Clear global cache to prevent cross-test pollution
    #[cfg(feature = "llm_cache")]
    astraweave_llm::clear_global_cache();

    // Use unique world snapshot with distinctive objective to ensure unique cache key
    // This prevents cache collisions with parallel tests that use the same scenario
    let mut world_snapshot = create_complex_scenario();
    world_snapshot.t = 999.99; // Unique timestamp
    world_snapshot.objective = Some("ERROR_HANDLING_TEST_UNIQUE_OBJECTIVE_12345".to_string());
    let tool_registry = create_comprehensive_registry();

    // Test with client that returns invalid JSON
    struct BadJsonClient;

    #[async_trait::async_trait]
    impl astraweave_llm::LlmClient for BadJsonClient {
        async fn complete(&self, _prompt: &str) -> anyhow::Result<String> {
            Ok("This is not JSON at all!".to_string())
        }
    }

    let bad_client = BadJsonClient;
    let plan_source = plan_from_llm(&bad_client, &world_snapshot, &tool_registry).await;
    // Should fallback to heuristic or emergency plan (Phase 7 multi-tier fallback)
    match plan_source {
        PlanSource::Llm(_) => panic!("Expected fallback from BadJsonClient"),
        PlanSource::Fallback { plan, .. } => {
            // Phase 7: Plans use UUID-based IDs
            assert!(
                plan.plan_id.starts_with("heuristic-") || plan.plan_id.starts_with("emergency-"),
                "Expected heuristic or emergency plan, got: {}",
                plan.plan_id
            );
        }
        _ => panic!("Unexpected PlanSource variant"),
    }

    // Clear cache again before second test case to prevent race conditions with parallel tests
    #[cfg(feature = "llm_cache")]
    astraweave_llm::clear_global_cache();

    // Test with client that returns JSON with disallowed tools
    struct DisallowedToolClient;

    #[async_trait::async_trait]
    impl astraweave_llm::LlmClient for DisallowedToolClient {
        async fn complete(&self, _prompt: &str) -> anyhow::Result<String> {
            Ok(
                r#"{"plan_id": "bad", "steps": [{"act": "Hack", "target": "mainframe"}]}"#
                    .to_string(),
            )
        }
    }

    let disallowed_client = DisallowedToolClient;
    let plan_source = plan_from_llm(&disallowed_client, &world_snapshot, &tool_registry).await;
    // Should fallback to heuristic or emergency plan (Phase 7)
    match &plan_source {
        PlanSource::Llm(_) => panic!("Expected fallback from DisallowedToolClient"),
        PlanSource::Fallback { plan, .. } => {
            // Phase 7: Plans use UUID-based IDs
            assert!(
                plan.plan_id.starts_with("heuristic-") || plan.plan_id.starts_with("emergency-"),
                "Expected heuristic or emergency plan, got: {}",
                plan.plan_id
            );
        }
        _ => panic!("Unexpected PlanSource variant"),
    }
}

/// Test validation with different tool registry configurations
#[test]
fn test_tool_registry_validation() {
    // Test with minimal registry (PascalCase to match action_step_to_tool_name)
    let minimal_registry = ToolRegistry {
        tools: vec![ToolSpec {
            name: "MoveTo".into(),
            args: [("x", "i32"), ("y", "i32")]
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }],
        constraints: Constraints {
            enforce_cooldowns: false,
            enforce_los: false,
            enforce_stamina: false,
        },
    };

    let valid_json = r#"{"plan_id": "minimal", "steps": [{"act": "MoveTo", "x": 5, "y": 5}]}"#;
    let result = parse_llm_plan(valid_json, &minimal_registry);
    assert!(result.is_ok());

    // Test that unregistered tools are rejected
    let invalid_json =
        r#"{"plan_id": "invalid", "steps": [{"act": "Throw", "item": "grenade", "x": 5, "y": 5}]}"#;
    let result = parse_llm_plan(invalid_json, &minimal_registry);
    assert!(result.is_err());
}

fn create_complex_scenario() -> WorldSnapshot {
    WorldSnapshot {
        t: 42.5,
        player: PlayerState {
            hp: 85,
            pos: IVec2 { x: 10, y: 15 },
            stance: "crouch".into(),
            orders: vec!["hold_position".into(), "watch_six".into()],
        },
        me: CompanionState {
            ammo: 25,
            cooldowns: Default::default(),
            morale: 0.8,
            pos: IVec2 { x: 12, y: 15 },
        },
        enemies: vec![
            EnemyState {
                id: 201,
                pos: IVec2 { x: 25, y: 20 },
                hp: 90,
                cover: "heavy".into(),
                last_seen: 2.0,
            },
            EnemyState {
                id: 202,
                pos: IVec2 { x: 18, y: 12 },
                hp: 45,
                cover: "light".into(),
                last_seen: 0.5,
            },
            EnemyState {
                id: 203,
                pos: IVec2 { x: 30, y: 25 },
                hp: 75,
                cover: "none".into(),
                last_seen: 5.0,
            },
        ],
        pois: vec![
            astraweave_core::Poi {
                k: "extract_zone".into(),
                pos: IVec2 { x: 50, y: 50 },
            },
            astraweave_core::Poi {
                k: "ammo_resupply".into(),
                pos: IVec2 { x: 15, y: 8 },
            },
            astraweave_core::Poi {
                k: "high_ground".into(),
                pos: IVec2 { x: 20, y: 25 },
            },
        ],
        obstacles: vec![],
        objective: Some("Reach extraction zone while eliminating hostiles".into()),
    
        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}

fn create_comprehensive_registry() -> ToolRegistry {
    ToolRegistry {
        tools: vec![
            // Phase 7: Matches MockLlm output and action_step_to_tool_name mapping (PascalCase)
            ToolSpec {
                name: "MoveTo".into(),
                args: [("x", "i32"), ("y", "i32")]
                    .into_iter()
                    .map(|(k, v)| (k.into(), v.into()))
                    .collect(),
            },
            ToolSpec {
                name: "ThrowSmoke".into(),
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
                name: "Revive".into(),
                args: [("ally_id", "u32")]
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

/// Test obstacle handling with single obstacle blocking direct path
#[tokio::test]
async fn test_obstacle_single_blocking_path() {
    let world_snapshot = create_scenario_with_single_obstacle();
    let tool_registry = create_comprehensive_registry();

    let client = MockLlm;
    let plan_source = plan_from_llm(&client, &world_snapshot, &tool_registry).await;
    let plan = match plan_source {
        PlanSource::Llm(p) => p,
        PlanSource::Fallback { .. } => panic!("Expected LLM plan"),
        _ => panic!("Unexpected PlanSource variant"),
    };

    // Plan should include movement steps that navigate around the obstacle
    assert!(!plan.steps.is_empty());
    assert!(plan
        .steps
        .iter()
        .any(|step| matches!(step, astraweave_core::ActionStep::MoveTo { .. })));
}

/// Test obstacle handling with multiple obstacles creating a maze-like environment
#[tokio::test]
async fn test_obstacle_multiple_complex_layout() {
    let world_snapshot = create_scenario_with_multiple_obstacles();
    let tool_registry = create_comprehensive_registry();

    let client = MockLlm;
    let plan_source = plan_from_llm(&client, &world_snapshot, &tool_registry).await;
    let plan = match plan_source {
        PlanSource::Llm(p) => p,
        PlanSource::Fallback { .. } => panic!("Expected LLM plan"),
        _ => panic!("Unexpected PlanSource variant"),
    };

    // Plan should be generated despite complex obstacle layout
    assert!(!plan.steps.is_empty());
}

/// Test obstacle handling when obstacles surround the companion
#[tokio::test]
async fn test_obstacle_surrounded_companion() {
    let world_snapshot = create_scenario_with_surrounding_obstacles();
    let tool_registry = create_comprehensive_registry();

    let client = MockLlm;
    let plan_source = plan_from_llm(&client, &world_snapshot, &tool_registry).await;
    let plan = match plan_source {
        PlanSource::Llm(p) => p,
        PlanSource::Fallback { .. } => panic!("Expected LLM plan"),
        _ => panic!("Unexpected PlanSource variant"),
    };

    // Plan should still be generated even with companion in confined space
    assert!(!plan.steps.is_empty());
}

/// Test obstacle handling with obstacles at edge positions
#[tokio::test]
async fn test_obstacle_edge_positions() {
    let world_snapshot = create_scenario_with_edge_obstacles();
    let tool_registry = create_comprehensive_registry();

    let client = MockLlm;
    let plan_source = plan_from_llm(&client, &world_snapshot, &tool_registry).await;
    let plan = match plan_source {
        PlanSource::Llm(p) => p,
        PlanSource::Fallback { .. } => panic!("Expected LLM plan"),
        _ => panic!("Unexpected PlanSource variant"),
    };

    // Plan should handle edge cases properly
    assert!(!plan.steps.is_empty());
}

/// Test prompt generation includes obstacle information
#[test]
fn test_prompt_includes_obstacles() {
    let world_snapshot = create_scenario_with_single_obstacle();
    let tool_registry = create_comprehensive_registry();

    let prompt = astraweave_llm::build_prompt(&world_snapshot, &tool_registry);

    // Verify prompt contains obstacle information
    assert!(prompt.contains("obstacles"));
    assert!(prompt.contains("5")); // Should contain obstacle coordinates
    assert!(prompt.contains("10"));
}

/// Test that obstacle scenarios don't break JSON parsing
#[test]
fn test_obstacle_scenario_json_parsing() {
    let _world_snapshot = create_scenario_with_multiple_obstacles();
    let tool_registry = create_comprehensive_registry();

    let json_output =
        r#"{"plan_id": "obstacle-test", "steps": [{"act": "MoveTo", "x": 15, "y": 20}]}"#;

    let result = parse_llm_plan(json_output, &tool_registry);
    assert!(result.is_ok());
}
fn create_scenario_with_single_obstacle() -> WorldSnapshot {
    WorldSnapshot {
        t: 10.0,
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 0, y: 0 },
            stance: "stand".into(),
            orders: vec![],
        },
        me: CompanionState {
            ammo: 10,
            cooldowns: Default::default(),
            morale: 1.0,
            pos: IVec2 { x: 0, y: 0 },
        },
        enemies: vec![EnemyState {
            id: 1,
            pos: IVec2 { x: 10, y: 0 },
            hp: 100,
            cover: "none".into(),
            last_seen: 0.0,
        }],
        pois: vec![],
        obstacles: vec![IVec2 { x: 5, y: 0 }], // Single obstacle blocking direct path
        objective: Some("Reach enemy position".into()),
    
        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}

fn create_scenario_with_multiple_obstacles() -> WorldSnapshot {
    WorldSnapshot {
        t: 15.0,
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 0, y: 0 },
            stance: "stand".into(),
            orders: vec![],
        },
        me: CompanionState {
            ammo: 10,
            cooldowns: Default::default(),
            morale: 1.0,
            pos: IVec2 { x: 0, y: 0 },
        },
        enemies: vec![EnemyState {
            id: 1,
            pos: IVec2 { x: 20, y: 20 },
            hp: 100,
            cover: "none".into(),
            last_seen: 0.0,
        }],
        pois: vec![],
        obstacles: vec![
            IVec2 { x: 5, y: 0 },   // Horizontal barrier
            IVec2 { x: 5, y: 5 },   // Vertical extension
            IVec2 { x: 10, y: 10 }, // Diagonal blocker
            IVec2 { x: 15, y: 15 }, // Near target
            IVec2 { x: 0, y: 10 },  // Side blocker
        ],
        objective: Some("Navigate complex obstacle field".into()),
    
        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}

fn create_scenario_with_surrounding_obstacles() -> WorldSnapshot {
    WorldSnapshot {
        t: 20.0,
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 10, y: 10 },
            stance: "stand".into(),
            orders: vec![],
        },
        me: CompanionState {
            ammo: 10,
            cooldowns: Default::default(),
            morale: 1.0,
            pos: IVec2 { x: 10, y: 10 }, // Companion at center
        },
        enemies: vec![EnemyState {
            id: 1,
            pos: IVec2 { x: 25, y: 25 },
            hp: 100,
            cover: "none".into(),
            last_seen: 0.0,
        }],
        pois: vec![],
        obstacles: vec![
            IVec2 { x: 9, y: 10 },  // Left
            IVec2 { x: 11, y: 10 }, // Right
            IVec2 { x: 10, y: 9 },  // Up
            IVec2 { x: 10, y: 11 }, // Down
            IVec2 { x: 9, y: 9 },   // Diagonal
            IVec2 { x: 11, y: 11 }, // Diagonal
        ],
        objective: Some("Escape confined space".into()),
    
        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}

fn create_scenario_with_edge_obstacles() -> WorldSnapshot {
    WorldSnapshot {
        t: 25.0,
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 0, y: 0 },
            stance: "stand".into(),
            orders: vec![],
        },
        me: CompanionState {
            ammo: 10,
            cooldowns: Default::default(),
            morale: 1.0,
            pos: IVec2 { x: 0, y: 0 },
        },
        enemies: vec![EnemyState {
            id: 1,
            pos: IVec2 { x: 50, y: 50 },
            hp: 100,
            cover: "none".into(),
            last_seen: 0.0,
        }],
        pois: vec![],
        obstacles: vec![
            IVec2 { x: 0, y: 1 },   // Adjacent to companion
            IVec2 { x: 49, y: 50 }, // Adjacent to enemy
            IVec2 { x: 25, y: 25 }, // Center obstacle
            IVec2 { x: 0, y: 50 },  // Corner position
            IVec2 { x: 50, y: 0 },  // Opposite corner
        ],
        objective: Some("Handle edge case obstacles".into()),
    
        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}
