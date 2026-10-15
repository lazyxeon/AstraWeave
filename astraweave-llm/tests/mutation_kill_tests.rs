//! Targeted mutation-kill tests for astraweave-llm
//!
//! Covers missed mutations in:
//! - lib.rs utility functions (parse_llm_plan, sanitize_plan, extract_json_*, fallback_heuristic_plan)
//! - cache module (find_similar, is_empty, LRU put/keys, similarity functions)

#![allow(unused_imports, clippy::bool_comparison)]

use astraweave_core::{
    ActionStep, CompanionState, Constraints, EnemyState, IVec2, PlayerState, ToolRegistry,
    ToolSpec, WorldSnapshot,
};
use astraweave_llm::cache::similarity::{
    extract_key_tokens, jaccard_similarity, prompt_similarity, tokenize,
};
use astraweave_llm::cache::{CacheDecision, CachedPlan, PromptCache};
use astraweave_llm::{fallback_heuristic_plan, parse_llm_plan, sanitize_plan};
use std::collections::BTreeMap;
use std::time::Instant;

// ═══════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════

fn full_registry() -> ToolRegistry {
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
                name: "Throw".into(),
                args: [("item", "string"), ("x", "i32"), ("y", "i32")]
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
                name: "Revive".into(),
                args: [("ally_id", "u32")]
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

fn snap_with_enemies_and_objective() -> WorldSnapshot {
    WorldSnapshot {
        t: 1.0,
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 10, y: 10 },
            stance: "stand".into(),
            orders: vec![],
        },
        me: CompanionState {
            ammo: 30,
            cooldowns: BTreeMap::new(),
            morale: 0.9,
            pos: IVec2 { x: 3, y: 2 },
        },
        enemies: vec![EnemyState {
            id: 42,
            pos: IVec2 { x: 20, y: 20 },
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

fn snap_no_enemies_no_objective() -> WorldSnapshot {
    WorldSnapshot {
        t: 0.0,
        player: PlayerState {
            hp: 100,
            pos: IVec2 { x: 1, y: 1 },
            stance: "stand".into(),
            orders: vec![],
        },
        me: CompanionState {
            ammo: 30,
            cooldowns: BTreeMap::new(),
            morale: 0.8,
            pos: IVec2 { x: 1, y: 1 },
        },
        enemies: vec![],
        pois: vec![],
        obstacles: vec![],
        objective: None,
    
        hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
        hazard_count: 0,
    }
}

fn make_cached_plan(plan_id: &str) -> CachedPlan {
    CachedPlan {
        plan: astraweave_core::PlanIntent {
            plan_id: plan_id.to_string(),
            steps: vec![],
        },
        created_at: Instant::now(),
        tokens_saved: 100,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// FALLBACK HEURISTIC PLAN TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod fallback_heuristic_tests {
    use super::*;

    #[test]
    fn heuristic_with_extract_objective_and_far_player_produces_moveto() {
        let snap = snap_with_enemies_and_objective();
        let reg = full_registry();
        let plan = fallback_heuristic_plan(&snap, &reg);
        assert_eq!(plan.plan_id, "heuristic-fallback");
        // Player at (10,10), companion at (3,2), dist = 7+8 = 15 > 3 → MoveTo
        let has_move = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::MoveTo { .. }));
        assert!(has_move, "expected MoveTo when dist > 3");
    }

    #[test]
    fn heuristic_with_extract_objective_and_close_player_no_moveto() {
        let mut snap = snap_with_enemies_and_objective();
        // Place companion right next to player so dist <= 3
        snap.me.pos = IVec2 { x: 10, y: 10 };
        let reg = full_registry();
        let plan = fallback_heuristic_plan(&snap, &reg);
        let has_move = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::MoveTo { .. }));
        assert!(!has_move, "no MoveTo when dist <= 3");
    }

    #[test]
    fn heuristic_moveto_targets_player_position() {
        let snap = snap_with_enemies_and_objective();
        let reg = full_registry();
        let plan = fallback_heuristic_plan(&snap, &reg);
        for step in &plan.steps {
            if let ActionStep::MoveTo { x, y, .. } = step {
                assert_eq!(*x, snap.player.pos.x);
                assert_eq!(*y, snap.player.pos.y);
            }
        }
    }

    #[test]
    fn heuristic_non_extract_objective_no_moveto() {
        let mut snap = snap_with_enemies_and_objective();
        snap.objective = Some("defend".into());
        let reg = full_registry();
        let plan = fallback_heuristic_plan(&snap, &reg);
        let has_move = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::MoveTo { .. }));
        assert!(!has_move, "non-extract objective should not trigger MoveTo");
    }

    #[test]
    fn heuristic_enemies_nearby_produces_coverfire() {
        let snap = snap_with_enemies_and_objective();
        let reg = full_registry();
        let plan = fallback_heuristic_plan(&snap, &reg);
        let has_cover = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::CoverFire { .. }));
        assert!(has_cover, "expected CoverFire when enemies present");
    }

    #[test]
    fn heuristic_coverfire_targets_first_enemy_with_duration_2() {
        let snap = snap_with_enemies_and_objective();
        let reg = full_registry();
        let plan = fallback_heuristic_plan(&snap, &reg);
        for step in &plan.steps {
            if let ActionStep::CoverFire {
                target_id,
                duration,
            } = step
            {
                assert_eq!(*target_id, 42);
                assert!((duration - 2.0).abs() < f32::EPSILON);
            }
        }
    }

    #[test]
    fn heuristic_no_enemies_no_coverfire() {
        let snap = snap_no_enemies_no_objective();
        let reg = full_registry();
        let plan = fallback_heuristic_plan(&snap, &reg);
        let has_cover = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::CoverFire { .. }));
        assert!(!has_cover, "no CoverFire without enemies");
    }

    #[test]
    fn heuristic_no_moveto_tool_in_registry_skips_moveto() {
        let snap = snap_with_enemies_and_objective();
        let reg = ToolRegistry {
            tools: vec![ToolSpec {
                name: "CoverFire".into(),
                args: Default::default(),
            }],
            constraints: Constraints {
                enforce_cooldowns: false,
                enforce_los: false,
                enforce_stamina: false,
            },
        };
        let plan = fallback_heuristic_plan(&snap, &reg);
        let has_move = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::MoveTo { .. }));
        assert!(!has_move, "no MoveTo when tool not in registry");
    }

    #[test]
    fn heuristic_no_coverfire_tool_in_registry_skips_coverfire() {
        let snap = snap_with_enemies_and_objective();
        let reg = ToolRegistry {
            tools: vec![ToolSpec {
                name: "MoveTo".into(),
                args: Default::default(),
            }],
            constraints: Constraints {
                enforce_cooldowns: false,
                enforce_los: false,
                enforce_stamina: false,
            },
        };
        let plan = fallback_heuristic_plan(&snap, &reg);
        let has_cover = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::CoverFire { .. }));
        assert!(!has_cover, "no CoverFire when tool not in registry");
    }

    #[test]
    fn heuristic_no_objective_no_moveto() {
        let mut snap = snap_with_enemies_and_objective();
        snap.objective = None;
        let reg = full_registry();
        let plan = fallback_heuristic_plan(&snap, &reg);
        let has_move = plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::MoveTo { .. }));
        assert!(!has_move, "no objective means no MoveTo");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SANITIZE PLAN TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod sanitize_plan_tests {
    use super::*;

    #[test]
    fn sanitize_retains_valid_moveto() {
        let reg = full_registry();
        let snap = snap_with_enemies_and_objective();
        let mut plan = astraweave_core::PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::MoveTo {
                x: 5,
                y: 5,
                speed: None,
            }],
        };
        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert_eq!(plan.steps.len(), 1);
    }

    #[test]
    fn sanitize_removes_moveto_out_of_bounds() {
        let reg = full_registry();
        let snap = snap_with_enemies_and_objective();
        let mut plan = astraweave_core::PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::MoveTo {
                x: 200,
                y: 5,
                speed: None,
            }],
        };
        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert!(
            plan.steps.is_empty(),
            "out-of-bounds MoveTo should be removed"
        );
    }

    #[test]
    fn sanitize_removes_moveto_y_out_of_bounds() {
        let reg = full_registry();
        let snap = snap_with_enemies_and_objective();
        let mut plan = astraweave_core::PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::MoveTo {
                x: 5,
                y: -200,
                speed: None,
            }],
        };
        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn sanitize_removes_moveto_without_registry_entry() {
        let snap = snap_with_enemies_and_objective();
        let empty_reg = ToolRegistry {
            tools: vec![],
            constraints: Constraints {
                enforce_cooldowns: false,
                enforce_los: false,
                enforce_stamina: false,
            },
        };
        let mut plan = astraweave_core::PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::MoveTo {
                x: 5,
                y: 5,
                speed: None,
            }],
        };
        sanitize_plan(&mut plan, &snap, &empty_reg).unwrap();
        assert!(
            plan.steps.is_empty(),
            "MoveTo without registry entry removed"
        );
    }

    #[test]
    fn sanitize_retains_valid_throw_smoke() {
        let reg = full_registry();
        let snap = snap_with_enemies_and_objective();
        let mut plan = astraweave_core::PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::Throw {
                item: "smoke".into(),
                x: 1,
                y: 1,
            }],
        };
        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert_eq!(plan.steps.len(), 1);
    }

    #[test]
    fn sanitize_removes_throw_with_invalid_item() {
        let reg = full_registry();
        let snap = snap_with_enemies_and_objective();
        let mut plan = astraweave_core::PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::Throw {
                item: "nuke".into(),
                x: 1,
                y: 1,
            }],
        };
        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert!(
            plan.steps.is_empty(),
            "invalid throw item should be removed"
        );
    }

    #[test]
    fn sanitize_removes_throw_out_of_bounds() {
        let reg = full_registry();
        let snap = snap_with_enemies_and_objective();
        let mut plan = astraweave_core::PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::Throw {
                item: "smoke".into(),
                x: 150,
                y: 1,
            }],
        };
        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn sanitize_retains_valid_coverfire() {
        let reg = full_registry();
        let snap = snap_with_enemies_and_objective();
        let mut plan = astraweave_core::PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::CoverFire {
                target_id: 42,
                duration: 3.0,
            }],
        };
        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert_eq!(plan.steps.len(), 1);
    }

    #[test]
    fn sanitize_removes_coverfire_invalid_target() {
        let reg = full_registry();
        let snap = snap_with_enemies_and_objective();
        let mut plan = astraweave_core::PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::CoverFire {
                target_id: 999,
                duration: 3.0,
            }],
        };
        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert!(plan.steps.is_empty(), "target_id not in enemies → removed");
    }

    #[test]
    fn sanitize_removes_coverfire_zero_duration() {
        let reg = full_registry();
        let snap = snap_with_enemies_and_objective();
        let mut plan = astraweave_core::PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::CoverFire {
                target_id: 42,
                duration: 0.0,
            }],
        };
        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert!(plan.steps.is_empty(), "duration 0 → removed");
    }

    #[test]
    fn sanitize_removes_coverfire_excessive_duration() {
        let reg = full_registry();
        let snap = snap_with_enemies_and_objective();
        let mut plan = astraweave_core::PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::CoverFire {
                target_id: 42,
                duration: 11.0,
            }],
        };
        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert!(plan.steps.is_empty(), "duration > 10 → removed");
    }

    #[test]
    fn sanitize_removes_revive_without_registry_entry() {
        let snap = snap_with_enemies_and_objective();
        let no_revive_reg = ToolRegistry {
            tools: vec![ToolSpec {
                name: "MoveTo".into(),
                args: Default::default(),
            }],
            constraints: Constraints {
                enforce_cooldowns: false,
                enforce_los: false,
                enforce_stamina: false,
            },
        };
        let mut plan = astraweave_core::PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::Revive { ally_id: 1 }],
        };
        sanitize_plan(&mut plan, &snap, &no_revive_reg).unwrap();
        assert!(
            plan.steps.is_empty(),
            "Revive without registry entry → removed"
        );
    }

    #[test]
    fn sanitize_retains_revive_with_registry_entry() {
        let reg = full_registry();
        let snap = snap_with_enemies_and_objective();
        let mut plan = astraweave_core::PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::Revive { ally_id: 1 }],
        };
        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert_eq!(plan.steps.len(), 1);
    }

    #[test]
    fn sanitize_boundary_coord_100_retained() {
        let reg = full_registry();
        let snap = snap_with_enemies_and_objective();
        let mut plan = astraweave_core::PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::MoveTo {
                x: 100,
                y: -100,
                speed: None,
            }],
        };
        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert_eq!(plan.steps.len(), 1, "coords at boundary ±100 should stay");
    }

    #[test]
    fn sanitize_boundary_coord_101_removed() {
        let reg = full_registry();
        let snap = snap_with_enemies_and_objective();
        let mut plan = astraweave_core::PlanIntent {
            plan_id: "test".into(),
            steps: vec![ActionStep::MoveTo {
                x: 101,
                y: 0,
                speed: None,
            }],
        };
        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert!(plan.steps.is_empty(), "coords 101 exceed bound");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PARSE LLM PLAN TESTS (extract_json_object, strip_code_fences, etc.)
// ═══════════════════════════════════════════════════════════════════════════

mod parse_plan_tests {
    use super::*;

    #[test]
    fn parse_plan_from_json_with_code_fence() {
        let reg = full_registry();
        let input = r#"```json
{"plan_id": "fenced", "steps": [{"act":"MoveTo","x":1,"y":2}]}
```"#;
        let plan = parse_llm_plan(input, &reg).unwrap();
        assert_eq!(plan.plan_id, "fenced");
        assert_eq!(plan.steps.len(), 1);
    }

    #[test]
    fn parse_plan_from_bare_fenced_block() {
        let reg = full_registry();
        let input = r#"```
{"plan_id": "bare", "steps": []}
```"#;
        let plan = parse_llm_plan(input, &reg).unwrap();
        assert_eq!(plan.plan_id, "bare");
    }

    #[test]
    fn parse_plan_from_embedded_json() {
        let reg = full_registry();
        let input = r#"Here is my plan: {"plan_id":"embed","steps":[]} and that's it."#;
        let plan = parse_llm_plan(input, &reg).unwrap();
        assert_eq!(plan.plan_id, "embed");
    }

    #[test]
    fn parse_plan_with_escaped_quotes_in_json() {
        let reg = full_registry();
        let input = r#"{"plan_id":"esc\"test","steps":[]}"#;
        // This JSON has escaped quotes inside a value — extract_json_object handles escape tracking
        let result = parse_llm_plan(input, &reg);
        // Should either parse or fail gracefully, never panic
        let _ = result;
    }

    #[test]
    fn parse_plan_fuzzy_key_planid() {
        let reg = full_registry();
        let input = r#"{"planId":"fuzzy-match","steps":[]}"#;
        let plan = parse_llm_plan(input, &reg).unwrap();
        assert_eq!(plan.plan_id, "fuzzy-match");
    }

    #[test]
    fn parse_plan_fuzzy_key_plan_number() {
        let reg = full_registry();
        let input = r#"{"plan_number":"fuzzy-num","steps":[]}"#;
        let plan = parse_llm_plan(input, &reg).unwrap();
        assert_eq!(plan.plan_id, "fuzzy-num");
    }

    #[test]
    fn parse_plan_rejects_no_json() {
        let reg = full_registry();
        let result = parse_llm_plan("no json here", &reg);
        assert!(result.is_err());
    }

    #[test]
    fn parse_plan_validates_against_registry() {
        // MoveTo without "MoveTo" in registry should fail validation
        let empty_reg = ToolRegistry {
            tools: vec![],
            constraints: Constraints {
                enforce_cooldowns: false,
                enforce_los: false,
                enforce_stamina: false,
            },
        };
        let input = r#"{"plan_id":"bad","steps":[{"act":"MoveTo","x":1,"y":2}]}"#;
        let result = parse_llm_plan(input, &empty_reg);
        assert!(
            result.is_err(),
            "validate_plan should reject MoveTo not in registry"
        );
    }

    #[test]
    fn parse_plan_nested_json_objects() {
        let reg = full_registry();
        // Text before and nested braces
        let input = r#"Sure! {"plan_id":"nested","steps":[{"act":"MoveTo","x":1,"y":2}]}"#;
        let plan = parse_llm_plan(input, &reg).unwrap();
        assert_eq!(plan.plan_id, "nested");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CACHE MODULE TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod cache_tests {
    use super::*;
    use astraweave_llm::cache::key::PromptKey;

    #[test]
    fn prompt_cache_is_empty_when_new() {
        let cache = PromptCache::new(10);
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn prompt_cache_not_empty_after_put() {
        let cache = PromptCache::new(10);
        let key = PromptKey::new("hello world", "model-a", 0.7, &["MoveTo"]);
        cache.put(key, make_cached_plan("p1"));
        assert!(!cache.is_empty());
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn prompt_cache_exact_hit() {
        let cache = PromptCache::new(10);
        let key = PromptKey::new("attack the enemy", "model-a", 0.7, &["MoveTo"]);
        cache.put(key.clone(), make_cached_plan("exact-hit"));
        let result = cache.get(&key);
        assert!(result.is_some());
        let (plan, decision) = result.unwrap();
        assert_eq!(plan.plan.plan_id, "exact-hit");
        assert_eq!(decision, CacheDecision::HitExact);
    }

    #[test]
    fn prompt_cache_miss() {
        let cache = PromptCache::new(10);
        let key = PromptKey::new("attack the enemy", "model-a", 0.7, &["MoveTo"]);
        let result = cache.get(&key);
        assert!(result.is_none());
    }

    #[test]
    fn prompt_cache_similarity_hit() {
        // Use a very low threshold so similarity match triggers
        let cache = PromptCache::with_similarity_threshold(10, 0.3);
        let key1 = PromptKey::new(
            "move companion to enemy position attack",
            "model-a",
            0.7,
            &["MoveTo"],
        );
        cache.put(key1, make_cached_plan("similar-plan"));

        // Different but overlapping prompt (same model + temperature)
        let key2 = PromptKey::new(
            "move companion towards enemy position for attack",
            "model-a",
            0.7,
            &["MoveTo"],
        );
        let result = cache.get(&key2);
        assert!(result.is_some(), "similarity match should trigger");
        let (_, decision) = result.unwrap();
        // Should be HitSimilar, not HitExact
        assert!(
            matches!(decision, CacheDecision::HitSimilar(_)),
            "expected HitSimilar, got {:?}",
            decision
        );
    }

    #[test]
    fn prompt_cache_similarity_skips_different_model() {
        let cache = PromptCache::with_similarity_threshold(10, 0.01);
        let key1 = PromptKey::new("move to enemy", "model-a", 0.7, &["MoveTo"]);
        cache.put(key1, make_cached_plan("plan-a"));

        // Same prompt but different model — should NOT match via similarity
        let key2 = PromptKey::new("move to enemy", "model-b", 0.7, &["MoveTo"]);
        let result = cache.get(&key2);
        assert!(
            result.is_none(),
            "different model should not similarity match"
        );
    }

    #[test]
    fn prompt_cache_similarity_skips_distant_temperature() {
        let cache = PromptCache::with_similarity_threshold(10, 0.01);
        let key1 = PromptKey::new("move to enemy", "model-a", 0.7, &["MoveTo"]);
        cache.put(key1, make_cached_plan("plan-a"));

        // Same prompt but temperature diff > 0.1
        let key2 = PromptKey::new("move to enemy", "model-a", 1.0, &["MoveTo"]);
        let result = cache.get(&key2);
        assert!(result.is_none(), "temperature diff > 0.1 should not match");
    }

    #[test]
    fn prompt_cache_eviction_increments_counter() {
        let cache = PromptCache::new(2);
        let k1 = PromptKey::new("prompt one", "m", 0.5, &[]);
        let k2 = PromptKey::new("prompt two", "m", 0.5, &[]);
        let k3 = PromptKey::new("prompt three", "m", 0.5, &[]);
        cache.put(k1, make_cached_plan("p1"));
        cache.put(k2, make_cached_plan("p2"));
        cache.put(k3, make_cached_plan("p3")); // evicts LRU
        let stats = cache.stats();
        assert!(stats.evictions > 0, "eviction counter should increment");
        assert_eq!(stats.size, 2);
    }

    #[test]
    fn prompt_cache_clear_resets_stats() {
        let cache = PromptCache::new(10);
        let key = PromptKey::new("test", "model", 0.5, &[]);
        cache.put(key.clone(), make_cached_plan("p"));
        cache.get(&key);
        cache.clear();
        assert!(cache.is_empty());
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn prompt_cache_stats_hit_rate() {
        let cache = PromptCache::new(10);
        let key = PromptKey::new("test", "model", 0.5, &[]);
        cache.put(key.clone(), make_cached_plan("p"));
        cache.get(&key); // hit
        cache.get(&key); // hit
        let missing = PromptKey::new("missing", "model", 0.5, &[]);
        cache.get(&missing); // miss
        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        // hit_rate = 2/3 * 100 = 66
        assert_eq!(stats.hit_rate, 66);
    }

    #[test]
    fn prompt_key_equality_ignores_normalized_prompt() {
        let k1 = PromptKey::new("hello world test", "m", 0.5, &["a"]);
        let k2 = PromptKey::new("hello world test", "m", 0.5, &["a"]);
        assert_eq!(k1, k2);
    }

    #[test]
    fn prompt_key_hash_consistency() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let k1 = PromptKey::new("hello", "m", 0.5, &["a"]);
        let k2 = PromptKey::new("hello", "m", 0.5, &["a"]);
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        k1.hash(&mut h1);
        k2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// LRU CACHE TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod lru_tests {
    use astraweave_llm::cache::LruCache;

    #[test]
    fn lru_put_returns_false_no_eviction() {
        let cache = LruCache::new(5);
        assert!(!cache.put("a", 1));
    }

    #[test]
    fn lru_put_returns_true_on_eviction() {
        let cache = LruCache::new(2);
        cache.put("a", 1);
        cache.put("b", 2);
        let evicted = cache.put("c", 3);
        assert!(evicted, "should evict when at capacity");
    }

    #[test]
    fn lru_put_update_existing_no_eviction() {
        let cache = LruCache::new(2);
        cache.put("a", 1);
        cache.put("b", 2);
        let evicted = cache.put("a", 10); // update existing
        assert!(!evicted, "updating existing key should not evict");
        assert_eq!(cache.get(&"a"), Some(10));
    }

    #[test]
    fn lru_keys_returns_all_keys() {
        let cache = LruCache::new(5);
        cache.put("x", 1);
        cache.put("y", 2);
        cache.put("z", 3);
        let mut keys = cache.keys();
        keys.sort();
        assert_eq!(keys, vec!["x", "y", "z"]);
    }

    #[test]
    fn lru_is_empty_true_when_new() {
        let cache: LruCache<String, i32> = LruCache::new(5);
        assert!(cache.is_empty());
    }

    #[test]
    fn lru_is_empty_false_after_put() {
        let cache = LruCache::new(5);
        cache.put("a", 1);
        assert!(!cache.is_empty());
    }

    #[test]
    fn lru_evicts_least_recently_used() {
        let cache = LruCache::new(2);
        cache.put("a", 1);
        cache.put("b", 2);
        cache.get(&"a"); // touch "a" so "b" is LRU
        cache.put("c", 3); // should evict "b"
        assert_eq!(cache.get(&"a"), Some(1));
        assert_eq!(cache.get(&"b"), None, "b should have been evicted as LRU");
        assert_eq!(cache.get(&"c"), Some(3));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// SIMILARITY FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod similarity_tests {
    use super::*;

    #[test]
    fn jaccard_identical_sets() {
        let tokens = vec!["attack", "enemy"];
        assert_eq!(jaccard_similarity(&tokens, &tokens), 1.0);
    }

    #[test]
    fn jaccard_disjoint_sets() {
        let a = vec!["attack", "enemy"];
        let b = vec!["defend", "ally"];
        assert_eq!(jaccard_similarity(&a, &b), 0.0);
    }

    #[test]
    fn jaccard_both_empty() {
        let empty: Vec<&str> = vec![];
        assert_eq!(jaccard_similarity(&empty, &empty), 1.0);
    }

    #[test]
    fn jaccard_one_empty() {
        let a = vec!["x"];
        let empty: Vec<&str> = vec![];
        assert_eq!(jaccard_similarity(&a, &empty), 0.0);
        assert_eq!(jaccard_similarity(&empty, &a), 0.0);
    }

    #[test]
    fn jaccard_partial_overlap() {
        let a = vec!["attack", "enemy", "position"];
        let b = vec!["attack", "enemy", "cover"];
        // intersection=2, union=4 → 0.5
        let sim = jaccard_similarity(&a, &b);
        assert!((sim - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn tokenize_splits_whitespace() {
        let tokens = tokenize("hello world");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn tokenize_lowercases() {
        let tokens = tokenize("Hello WORLD");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn tokenize_splits_punctuation() {
        let tokens = tokenize("move,attack.defend");
        assert_eq!(tokens, vec!["move", "attack", "defend"]);
    }

    #[test]
    fn extract_key_tokens_filters_stopwords() {
        let tokens = extract_key_tokens("move to the enemy position");
        // "to", "the" are stopwords; "move" len=4, "enemy" len=5, "position" len=8
        assert!(tokens.contains(&"move".to_string()));
        assert!(tokens.contains(&"enemy".to_string()));
        assert!(tokens.contains(&"position".to_string()));
        assert!(!tokens.contains(&"the".to_string()));
        assert!(!tokens.contains(&"to".to_string()));
    }

    #[test]
    fn extract_key_tokens_filters_short_tokens() {
        let tokens = extract_key_tokens("go to xy");
        // "go" len=2, "to" stopword, "xy" len=2 → all filtered
        assert!(tokens.is_empty());
    }

    #[test]
    fn prompt_similarity_identical() {
        let sim = prompt_similarity("attack enemy position", "attack enemy position");
        assert_eq!(sim, 1.0);
    }

    #[test]
    fn prompt_similarity_partial() {
        let sim = prompt_similarity("attack enemy position", "attack enemy cover");
        assert!(sim > 0.0 && sim < 1.0);
    }
}
