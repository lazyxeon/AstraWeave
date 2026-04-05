//! Mutation-resistant comprehensive tests for astraweave-llm-eval.
//!
//! Targets: ScenarioType enum variants, ScoringWeights defaults, Scenario construction,
//! EvaluationSuite construction, ScenarioResult serde, EvaluationResults serde,
//! TypeStats serde.

use astraweave_llm_eval::*;
use std::collections::HashMap;

// =========================================================================
// ScenarioType — enum variants, equality, hash, clone, copy, serde
// =========================================================================

#[test]
fn scenario_type_all_variants_distinct() {
    let variants = [
        ScenarioType::Combat,
        ScenarioType::Exploration,
        ScenarioType::Stealth,
        ScenarioType::Support,
        ScenarioType::Puzzle,
    ];
    for (i, a) in variants.iter().enumerate() {
        for (j, b) in variants.iter().enumerate() {
            if i == j {
                assert_eq!(a, b, "Same index must be equal");
            } else {
                assert_ne!(a, b, "Different indices must differ: {} vs {}", i, j);
            }
        }
    }
}

#[test]
fn scenario_type_copy_semantics() {
    let a = ScenarioType::Combat;
    let b = a; // Copy
    assert_eq!(a, b);
}

#[test]
fn scenario_type_clone_semantics() {
    let a = ScenarioType::Exploration;
    #[allow(clippy::clone_on_copy)]
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn scenario_type_hash_all_variants() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(ScenarioType::Combat);
    set.insert(ScenarioType::Exploration);
    set.insert(ScenarioType::Stealth);
    set.insert(ScenarioType::Support);
    set.insert(ScenarioType::Puzzle);
    assert_eq!(set.len(), 5);
}

#[test]
fn scenario_type_hash_duplicates() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(ScenarioType::Combat);
    set.insert(ScenarioType::Combat);
    assert_eq!(set.len(), 1);
}

#[test]
fn scenario_type_serde_roundtrip_all_variants() {
    let variants = [
        ScenarioType::Combat,
        ScenarioType::Exploration,
        ScenarioType::Stealth,
        ScenarioType::Support,
        ScenarioType::Puzzle,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).unwrap();
        let deserialized: ScenarioType = serde_json::from_str(&json).unwrap();
        assert_eq!(*v, deserialized);
    }
}

#[test]
fn scenario_type_debug_format() {
    let s = format!("{:?}", ScenarioType::Combat);
    assert!(s.contains("Combat"));
}

// =========================================================================
// ScoringWeights — default values, serde
// =========================================================================

#[test]
fn scoring_weights_default_validity() {
    let w = ScoringWeights::default();
    assert!((w.validity - 0.40).abs() < 1e-10);
}

#[test]
fn scoring_weights_default_goal_achievement() {
    let w = ScoringWeights::default();
    assert!((w.goal_achievement - 0.30).abs() < 1e-10);
}

#[test]
fn scoring_weights_default_safety() {
    let w = ScoringWeights::default();
    assert!((w.safety - 0.15).abs() < 1e-10);
}

#[test]
fn scoring_weights_default_coherence() {
    let w = ScoringWeights::default();
    assert!((w.coherence - 0.15).abs() < 1e-10);
}

#[test]
fn scoring_weights_default_sum_to_one() {
    let w = ScoringWeights::default();
    let total = w.validity + w.goal_achievement + w.safety + w.coherence;
    assert!(
        (total - 1.0).abs() < 1e-10,
        "Weights must sum to 1.0, got {}",
        total
    );
}

#[test]
fn scoring_weights_serde_roundtrip() {
    let w = ScoringWeights::default();
    let json = serde_json::to_string(&w).unwrap();
    let w2: ScoringWeights = serde_json::from_str(&json).unwrap();
    assert!((w.validity - w2.validity).abs() < 1e-15);
    assert!((w.goal_achievement - w2.goal_achievement).abs() < 1e-15);
    assert!((w.safety - w2.safety).abs() < 1e-15);
    assert!((w.coherence - w2.coherence).abs() < 1e-15);
}

#[test]
fn scoring_weights_custom_serde_roundtrip() {
    let w = ScoringWeights {
        validity: 0.50,
        goal_achievement: 0.20,
        safety: 0.20,
        coherence: 0.10,
    };
    let json = serde_json::to_string(&w).unwrap();
    let w2: ScoringWeights = serde_json::from_str(&json).unwrap();
    assert!((w.validity - w2.validity).abs() < 1e-15);
    assert!((w.goal_achievement - w2.goal_achievement).abs() < 1e-15);
}

#[test]
fn scoring_weights_clone() {
    let w = ScoringWeights::default();
    let c = w.clone();
    assert!((w.validity - c.validity).abs() < 1e-15);
    assert!((w.safety - c.safety).abs() < 1e-15);
}

// =========================================================================
// Scenario — construction, field access, serde
// =========================================================================

fn make_scenario(id: &str, stype: ScenarioType) -> Scenario {
    Scenario {
        id: id.to_string(),
        scenario_type: stype,
        description: "test scenario".to_string(),
        prompt: "test prompt".to_string(),
        expected_actions: vec!["MoveTo".to_string()],
        forbidden_actions: vec!["Throw".to_string()],
    }
}

#[test]
fn scenario_field_access() {
    let s = make_scenario("s1", ScenarioType::Combat);
    assert_eq!(s.id, "s1");
    assert_eq!(s.scenario_type, ScenarioType::Combat);
    assert_eq!(s.description, "test scenario");
    assert_eq!(s.prompt, "test prompt");
    assert_eq!(s.expected_actions.len(), 1);
    assert_eq!(s.expected_actions[0], "MoveTo");
    assert_eq!(s.forbidden_actions.len(), 1);
    assert_eq!(s.forbidden_actions[0], "Throw");
}

#[test]
fn scenario_serde_roundtrip() {
    let s = make_scenario("s2", ScenarioType::Stealth);
    let json = serde_json::to_string(&s).unwrap();
    let s2: Scenario = serde_json::from_str(&json).unwrap();
    assert_eq!(s.id, s2.id);
    assert_eq!(s.scenario_type, s2.scenario_type);
    assert_eq!(s.expected_actions, s2.expected_actions);
    assert_eq!(s.forbidden_actions, s2.forbidden_actions);
}

#[test]
fn scenario_clone() {
    let s = make_scenario("clone_test", ScenarioType::Support);
    let c = s.clone();
    assert_eq!(s.id, c.id);
    assert_eq!(s.scenario_type, c.scenario_type);
}

#[test]
fn scenario_empty_actions() {
    let s = Scenario {
        id: "empty".to_string(),
        scenario_type: ScenarioType::Puzzle,
        description: String::new(),
        prompt: String::new(),
        expected_actions: vec![],
        forbidden_actions: vec![],
    };
    assert!(s.expected_actions.is_empty());
    assert!(s.forbidden_actions.is_empty());
}

// =========================================================================
// EvaluationSuite — default(), new()
// =========================================================================

#[test]
fn evaluation_suite_default_has_five_scenarios() {
    let suite = EvaluationSuite::default();
    assert_eq!(suite.scenarios.len(), 5);
}

#[test]
fn evaluation_suite_default_threshold() {
    let suite = EvaluationSuite::default();
    assert!((suite.passing_threshold - 0.70).abs() < 1e-10);
}

#[test]
fn evaluation_suite_default_weights_are_default() {
    let suite = EvaluationSuite::default();
    assert!((suite.weights.validity - 0.40).abs() < 1e-10);
    assert!((suite.weights.goal_achievement - 0.30).abs() < 1e-10);
    assert!((suite.weights.safety - 0.15).abs() < 1e-10);
    assert!((suite.weights.coherence - 0.15).abs() < 1e-10);
}

#[test]
fn evaluation_suite_default_scenario_ids() {
    let suite = EvaluationSuite::default();
    let ids: Vec<&str> = suite.scenarios.iter().map(|s| s.id.as_str()).collect();
    assert!(ids.contains(&"combat_basic"));
    assert!(ids.contains(&"combat_grenade"));
    assert!(ids.contains(&"exploration"));
    assert!(ids.contains(&"stealth"));
    assert!(ids.contains(&"support"));
}

#[test]
fn evaluation_suite_default_scenario_types() {
    let suite = EvaluationSuite::default();
    let types: Vec<ScenarioType> = suite.scenarios.iter().map(|s| s.scenario_type).collect();
    assert_eq!(
        types.iter().filter(|t| **t == ScenarioType::Combat).count(),
        2
    );
    assert_eq!(
        types
            .iter()
            .filter(|t| **t == ScenarioType::Exploration)
            .count(),
        1
    );
    assert_eq!(
        types
            .iter()
            .filter(|t| **t == ScenarioType::Stealth)
            .count(),
        1
    );
    assert_eq!(
        types
            .iter()
            .filter(|t| **t == ScenarioType::Support)
            .count(),
        1
    );
}

#[test]
fn evaluation_suite_new_custom_scenarios() {
    let scenarios = vec![make_scenario("custom1", ScenarioType::Puzzle)];
    let suite = EvaluationSuite::new(scenarios);
    assert_eq!(suite.scenarios.len(), 1);
    assert_eq!(suite.scenarios[0].id, "custom1");
}

#[test]
fn evaluation_suite_new_uses_default_weights() {
    let suite = EvaluationSuite::new(vec![]);
    assert!((suite.weights.validity - 0.40).abs() < 1e-10);
}

#[test]
fn evaluation_suite_new_uses_default_threshold() {
    let suite = EvaluationSuite::new(vec![]);
    assert!((suite.passing_threshold - 0.70).abs() < 1e-10);
}

#[test]
fn evaluation_suite_new_empty() {
    let suite = EvaluationSuite::new(vec![]);
    assert_eq!(suite.scenarios.len(), 0);
}

// =========================================================================
// Default scenarios — expected_actions, forbidden_actions
// =========================================================================

#[test]
fn default_combat_basic_expected_actions() {
    let suite = EvaluationSuite::default();
    let combat = suite
        .scenarios
        .iter()
        .find(|s| s.id == "combat_basic")
        .unwrap();
    assert!(combat.expected_actions.contains(&"MoveTo".to_string()));
    assert!(combat.expected_actions.contains(&"CoverFire".to_string()));
    assert!(combat.forbidden_actions.is_empty());
}

#[test]
fn default_combat_grenade_expected_actions() {
    let suite = EvaluationSuite::default();
    let grenade = suite
        .scenarios
        .iter()
        .find(|s| s.id == "combat_grenade")
        .unwrap();
    assert!(grenade.expected_actions.contains(&"Throw".to_string()));
    assert!(grenade.forbidden_actions.is_empty());
}

#[test]
fn default_exploration_forbidden_actions() {
    let suite = EvaluationSuite::default();
    let explore = suite
        .scenarios
        .iter()
        .find(|s| s.id == "exploration")
        .unwrap();
    assert!(explore.expected_actions.contains(&"MoveTo".to_string()));
    assert!(explore.forbidden_actions.contains(&"CoverFire".to_string()));
}

#[test]
fn default_stealth_forbidden_actions() {
    let suite = EvaluationSuite::default();
    let stealth = suite.scenarios.iter().find(|s| s.id == "stealth").unwrap();
    assert!(stealth.expected_actions.contains(&"MoveTo".to_string()));
    assert!(stealth.forbidden_actions.contains(&"CoverFire".to_string()));
    assert!(stealth.forbidden_actions.contains(&"Throw".to_string()));
}

#[test]
fn default_support_expected_actions() {
    let suite = EvaluationSuite::default();
    let support = suite.scenarios.iter().find(|s| s.id == "support").unwrap();
    assert!(support.expected_actions.contains(&"MoveTo".to_string()));
    assert!(support.expected_actions.contains(&"Revive".to_string()));
    assert!(support.forbidden_actions.contains(&"CoverFire".to_string()));
}

// =========================================================================
// ScenarioResult — serde roundtrip
// =========================================================================

#[test]
fn scenario_result_serde_roundtrip() {
    let r = ScenarioResult {
        scenario_id: "test".to_string(),
        scenario_type: ScenarioType::Combat,
        validity_score: 1.0,
        goal_score: 0.75,
        safety_score: 1.0,
        coherence_score: 0.5,
        overall_score: 0.80,
        elapsed_ms: 42,
        raw_response: "{\"test\": true}".to_string(),
        parsed_plan: None,
        errors: vec!["err1".to_string()],
    };
    let json = serde_json::to_string(&r).unwrap();
    let r2: ScenarioResult = serde_json::from_str(&json).unwrap();
    assert_eq!(r.scenario_id, r2.scenario_id);
    assert_eq!(r.scenario_type, r2.scenario_type);
    assert!((r.validity_score - r2.validity_score).abs() < 1e-15);
    assert!((r.goal_score - r2.goal_score).abs() < 1e-15);
    assert!((r.safety_score - r2.safety_score).abs() < 1e-15);
    assert!((r.coherence_score - r2.coherence_score).abs() < 1e-15);
    assert!((r.overall_score - r2.overall_score).abs() < 1e-15);
    assert_eq!(r.elapsed_ms, r2.elapsed_ms);
    assert_eq!(r.raw_response, r2.raw_response);
    assert_eq!(r.errors, r2.errors);
}

#[test]
fn scenario_result_all_zeros() {
    let r = ScenarioResult {
        scenario_id: String::new(),
        scenario_type: ScenarioType::Puzzle,
        validity_score: 0.0,
        goal_score: 0.0,
        safety_score: 0.0,
        coherence_score: 0.0,
        overall_score: 0.0,
        elapsed_ms: 0,
        raw_response: String::new(),
        parsed_plan: None,
        errors: vec![],
    };
    assert_eq!(r.validity_score, 0.0);
    assert_eq!(r.goal_score, 0.0);
    assert_eq!(r.safety_score, 0.0);
    assert_eq!(r.coherence_score, 0.0);
    assert_eq!(r.overall_score, 0.0);
    assert_eq!(r.elapsed_ms, 0);
}

#[test]
fn scenario_result_clone() {
    let r = ScenarioResult {
        scenario_id: "clone_test".to_string(),
        scenario_type: ScenarioType::Exploration,
        validity_score: 0.9,
        goal_score: 0.8,
        safety_score: 1.0,
        coherence_score: 0.7,
        overall_score: 0.85,
        elapsed_ms: 100,
        raw_response: "resp".to_string(),
        parsed_plan: None,
        errors: vec![],
    };
    let c = r.clone();
    assert_eq!(c.scenario_id, "clone_test");
    assert!((c.overall_score - 0.85).abs() < 1e-15);
}

// =========================================================================
// TypeStats — serde roundtrip
// =========================================================================

#[test]
fn type_stats_serde_roundtrip() {
    let ts = TypeStats {
        count: 10,
        avg_validity: 0.95,
        avg_goal: 0.80,
        avg_safety: 1.0,
        avg_overall: 0.90,
    };
    let json = serde_json::to_string(&ts).unwrap();
    let ts2: TypeStats = serde_json::from_str(&json).unwrap();
    assert_eq!(ts.count, ts2.count);
    assert!((ts.avg_validity - ts2.avg_validity).abs() < 1e-15);
    assert!((ts.avg_goal - ts2.avg_goal).abs() < 1e-15);
    assert!((ts.avg_safety - ts2.avg_safety).abs() < 1e-15);
    assert!((ts.avg_overall - ts2.avg_overall).abs() < 1e-15);
}

#[test]
fn type_stats_clone() {
    let ts = TypeStats {
        count: 3,
        avg_validity: 0.5,
        avg_goal: 0.5,
        avg_safety: 0.5,
        avg_overall: 0.5,
    };
    let c = ts.clone();
    assert_eq!(c.count, 3);
}

// =========================================================================
// EvaluationResults — serde roundtrip
// =========================================================================

#[test]
fn evaluation_results_serde_roundtrip() {
    let mut results_by_type = HashMap::new();
    results_by_type.insert(
        ScenarioType::Combat,
        TypeStats {
            count: 2,
            avg_validity: 1.0,
            avg_goal: 0.75,
            avg_safety: 1.0,
            avg_overall: 0.90,
        },
    );

    let r = EvaluationResults {
        total_scenarios: 5,
        passed: 4,
        failed: 1,
        avg_validity: 0.90,
        avg_goal_achievement: 0.80,
        avg_safety: 1.0,
        avg_coherence: 0.70,
        overall_score: 0.85,
        total_elapsed_ms: 500,
        results_by_type,
        scenario_results: vec![],
    };
    let json = serde_json::to_string(&r).unwrap();
    let r2: EvaluationResults = serde_json::from_str(&json).unwrap();
    assert_eq!(r.total_scenarios, r2.total_scenarios);
    assert_eq!(r.passed, r2.passed);
    assert_eq!(r.failed, r2.failed);
    assert!((r.avg_validity - r2.avg_validity).abs() < 1e-15);
    assert!((r.avg_goal_achievement - r2.avg_goal_achievement).abs() < 1e-15);
    assert!((r.avg_safety - r2.avg_safety).abs() < 1e-15);
    assert!((r.avg_coherence - r2.avg_coherence).abs() < 1e-15);
    assert!((r.overall_score - r2.overall_score).abs() < 1e-15);
    assert_eq!(r.total_elapsed_ms, r2.total_elapsed_ms);
    assert_eq!(r.scenario_results.len(), 0);
}

#[test]
fn evaluation_results_clone() {
    let r = EvaluationResults {
        total_scenarios: 2,
        passed: 1,
        failed: 1,
        avg_validity: 0.5,
        avg_goal_achievement: 0.5,
        avg_safety: 0.5,
        avg_coherence: 0.5,
        overall_score: 0.5,
        total_elapsed_ms: 100,
        results_by_type: HashMap::new(),
        scenario_results: vec![],
    };
    let c = r.clone();
    assert_eq!(c.total_scenarios, 2);
    assert_eq!(c.passed, 1);
    assert_eq!(c.failed, 1);
}

#[test]
fn evaluation_results_with_scenario_results() {
    let sr = ScenarioResult {
        scenario_id: "sr1".to_string(),
        scenario_type: ScenarioType::Combat,
        validity_score: 1.0,
        goal_score: 1.0,
        safety_score: 1.0,
        coherence_score: 1.0,
        overall_score: 1.0,
        elapsed_ms: 10,
        raw_response: "{}".to_string(),
        parsed_plan: None,
        errors: vec![],
    };
    let r = EvaluationResults {
        total_scenarios: 1,
        passed: 1,
        failed: 0,
        avg_validity: 1.0,
        avg_goal_achievement: 1.0,
        avg_safety: 1.0,
        avg_coherence: 1.0,
        overall_score: 1.0,
        total_elapsed_ms: 10,
        results_by_type: HashMap::new(),
        scenario_results: vec![sr],
    };
    let json = serde_json::to_string(&r).unwrap();
    let r2: EvaluationResults = serde_json::from_str(&json).unwrap();
    assert_eq!(r2.scenario_results.len(), 1);
    assert_eq!(r2.scenario_results[0].scenario_id, "sr1");
}

// =========================================================================
// Edge cases
// =========================================================================

#[test]
fn scoring_weights_all_zero_is_valid_struct() {
    let w = ScoringWeights {
        validity: 0.0,
        goal_achievement: 0.0,
        safety: 0.0,
        coherence: 0.0,
    };
    assert_eq!(w.validity, 0.0);
}

#[test]
fn scoring_weights_all_one_quarter() {
    let w = ScoringWeights {
        validity: 0.25,
        goal_achievement: 0.25,
        safety: 0.25,
        coherence: 0.25,
    };
    let total = w.validity + w.goal_achievement + w.safety + w.coherence;
    assert!((total - 1.0).abs() < 1e-15);
}
// ========================================================================
// MUTATION KILL TESTS — target scoring function misses via evaluate()
// ========================================================================
mod mutation_kill_tests {
    use super::*;
    use astraweave_llm::LlmClient;
    use std::sync::Arc;

    /// Mock LLM returning 2 ThrowSmoke steps (non-validated types bypass tool registry check)
    struct SmokePlanMock;

    #[async_trait::async_trait]
    impl LlmClient for SmokePlanMock {
        async fn complete(&self, _prompt: &str) -> anyhow::Result<String> {
            Ok(r#"{"plan_id":"test","steps":[
                {"act":"ThrowSmoke","x":1,"y":2},
                {"act":"Attack","target_id":1}
            ]}"#
            .into())
        }
    }

    fn make_scenario(id: &str, stype: ScenarioType) -> Scenario {
        Scenario {
            id: id.into(),
            scenario_type: stype,
            description: "test".into(),
            prompt: "test".into(),
            expected_actions: vec![],
            forbidden_actions: vec![],
        }
    }

    /// Kills: score_coherence → 1.0 (line 388), == → != (line 426)
    /// Also: evaluate_scenario overall_score arithmetic (lines 304-307)
    /// And: evaluate >= → < (line 163), evaluate - → + (line 203)
    #[tokio::test]
    async fn evaluate_single_scenario_exact_scores() {
        // SmokePlanMock: [ThrowSmoke, Attack]
        // These pass validation catch-all (not MoveTo/Throw/CoverFire/Revive)
        // validity = 1.0 (parsed OK)
        // goal = 0.0 (no expected actions in match arms for ThrowSmoke/Attack)
        // safety = 1.0 (no forbidden actions in match arms)
        // coherence: 1 pair: ThrowSmoke→Attack, not same type, not MoveTo→CoverFire
        //   coherence_points=0, total_checks=1 → (0/1 + 1)/2 = 0.5
        // overall = 1.0*0.4 + 0.0*0.3 + 1.0*0.15 + 0.5*0.15 = 0.625
        let suite = EvaluationSuite::new(vec![make_scenario("s1", ScenarioType::Combat)]);
        let results = suite.evaluate(Arc::new(SmokePlanMock)).await;

        assert_eq!(results.total_scenarios, 1);

        let r = &results.scenario_results[0];
        assert!((r.validity_score - 1.0).abs() < 0.01, "validity");
        assert!((r.safety_score - 1.0).abs() < 0.01, "safety");
        assert!(
            (r.coherence_score - 0.5).abs() < 0.01,
            "coherence should be 0.5, got {}",
            r.coherence_score
        );

        let expected_overall = 1.0 * 0.4 + 0.0 * 0.3 + 1.0 * 0.15 + 0.5 * 0.15;
        assert!(
            (r.overall_score - expected_overall).abs() < 0.01,
            "overall should be {}, got {}",
            expected_overall,
            r.overall_score
        );

        // 0.625 < threshold 0.70, so failed=1, passed=0
        assert_eq!(results.passed, 0);
        assert_eq!(results.failed, 1);
    }

    /// Kills: average → 1.0 (line 438), average / → %,* (line 441)
    /// Also: evaluate == → != (line 183), delete ! (line 186)
    #[tokio::test]
    async fn evaluate_multiple_scenarios_averages() {
        let suite = EvaluationSuite::new(vec![
            make_scenario("m1", ScenarioType::Combat),
            make_scenario("m2", ScenarioType::Exploration),
        ]);
        let results = suite.evaluate(Arc::new(SmokePlanMock)).await;

        assert_eq!(results.total_scenarios, 2);
        // Both scenarios produce same scores → averages = individual scores
        assert!((results.avg_validity - 1.0).abs() < 0.01);
        assert!((results.avg_safety - 1.0).abs() < 0.01);
        assert!(
            (results.avg_coherence - 0.5).abs() < 0.01,
            "avg_coherence should be 0.5, got {}",
            results.avg_coherence
        );

        // results_by_type should have entries for Combat and Exploration
        assert!(results.results_by_type.contains_key(&ScenarioType::Combat));
        assert!(results
            .results_by_type
            .contains_key(&ScenarioType::Exploration));
        assert!(!results.results_by_type.contains_key(&ScenarioType::Stealth));
    }

    /// Kills: score_goal_achievement → 1.0 (line 332)
    /// With empty steps, score_goal_achievement returns 0.0.
    /// Mutation to 1.0 would change goal_score from 0.0 to 1.0.
    #[tokio::test]
    async fn goal_score_zero_when_no_matching_actions() {
        let suite = EvaluationSuite::new(vec![make_scenario("g1", ScenarioType::Combat)]);
        let results = suite.evaluate(Arc::new(SmokePlanMock)).await;
        let r = &results.scenario_results[0];
        assert!(
            (r.goal_score - 0.0).abs() < 0.01,
            "goal should be 0.0, got {}",
            r.goal_score
        );
    }
}
