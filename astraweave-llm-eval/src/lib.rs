//! LLM Evaluation Harness
//!
//! Automated testing framework for LLM plan generation with scoring metrics:
//! - **Validity** (40%): Plan parses and follows schema
//! - **Goal Achievement** (30%): Steps likely achieve the goal
//! - **Safety** (15%): No dangerous or impossible actions
//! - **Coherence** (15%): Steps are logically ordered
//!
//! # Example
//! ```ignore
//! use astraweave_llm_eval::{EvaluationSuite, ScenarioType};
//! use astraweave_llm::MockLlm;
//! use std::sync::Arc;
//!
//! async fn example() {
//!     let suite = EvaluationSuite::default();
//!     let client = Arc::new(MockLlm);
//!     let results = suite.evaluate(client).await;
//!
//!     println!("Overall score: {:.1}%", results.overall_score);
//! }
//! ```

use astraweave_core::{ActionStep, PlanIntent, ToolRegistry};
use astraweave_llm::LlmClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info};

/// Type of scenario (determines goal and expected actions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ScenarioType {
    Combat,
    Exploration,
    Stealth,
    Support,
    Puzzle,
}

/// A test scenario for LLM evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub scenario_type: ScenarioType,
    pub description: String,
    pub prompt: String,                 // Direct prompt for LLM
    pub expected_actions: Vec<String>,  // Expected action types (e.g., ["MoveTo", "CoverFire"])
    pub forbidden_actions: Vec<String>, // Actions that should never appear
}

/// Scoring weights for evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    pub validity: f64,         // 40% - Plan parses correctly
    pub goal_achievement: f64, // 30% - Steps achieve the goal
    pub safety: f64,           // 15% - No dangerous actions
    pub coherence: f64,        // 15% - Logical step ordering
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            validity: 0.40,
            goal_achievement: 0.30,
            safety: 0.15,
            coherence: 0.15,
        }
    }
}

/// Result of evaluating a single scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub scenario_id: String,
    pub scenario_type: ScenarioType,
    pub validity_score: f64,  // 0.0 - 1.0
    pub goal_score: f64,      // 0.0 - 1.0
    pub safety_score: f64,    // 0.0 - 1.0
    pub coherence_score: f64, // 0.0 - 1.0
    pub overall_score: f64,   // Weighted average
    pub elapsed_ms: u64,
    pub raw_response: String,
    pub parsed_plan: Option<PlanIntent>,
    pub errors: Vec<String>,
}

/// Aggregate results across all scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResults {
    pub total_scenarios: usize,
    pub passed: usize,
    pub failed: usize,
    pub avg_validity: f64,
    pub avg_goal_achievement: f64,
    pub avg_safety: f64,
    pub avg_coherence: f64,
    pub overall_score: f64,
    pub total_elapsed_ms: u64,
    pub results_by_type: std::collections::HashMap<ScenarioType, TypeStats>,
    pub scenario_results: Vec<ScenarioResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeStats {
    pub count: usize,
    pub avg_validity: f64,
    pub avg_goal: f64,
    pub avg_safety: f64,
    pub avg_overall: f64,
}

/// Evaluation suite with scenarios
pub struct EvaluationSuite {
    pub scenarios: Vec<Scenario>,
    pub weights: ScoringWeights,
    pub passing_threshold: f64, // Default: 0.70 (70%)
}

impl Default for EvaluationSuite {
    fn default() -> Self {
        Self {
            scenarios: Self::create_default_scenarios(),
            weights: ScoringWeights::default(),
            passing_threshold: 0.70,
        }
    }
}

impl EvaluationSuite {
    /// Create a new evaluation suite with custom scenarios
    pub fn new(scenarios: Vec<Scenario>) -> Self {
        Self {
            scenarios,
            weights: ScoringWeights::default(),
            passing_threshold: 0.70,
        }
    }

    /// Evaluate an LLM client against all scenarios
    pub async fn evaluate(&self, client: Arc<dyn LlmClient>) -> EvaluationResults {
        let start = Instant::now();
        let mut scenario_results = Vec::new();

        info!("Starting evaluation of {} scenarios", self.scenarios.len());

        for scenario in &self.scenarios {
            let result = self.evaluate_scenario(scenario, client.clone()).await;
            debug!(
                "Scenario '{}': {:.1}% overall",
                scenario.id,
                result.overall_score * 100.0
            );
            scenario_results.push(result);
        }

        let total_elapsed_ms = start.elapsed().as_millis() as u64;

        // Calculate aggregate stats
        let passed = scenario_results
            .iter()
            .filter(|r| r.overall_score >= self.passing_threshold)
            .count();

        let avg_validity = Self::average(&scenario_results, |r| r.validity_score);
        let avg_goal_achievement = Self::average(&scenario_results, |r| r.goal_score);
        let avg_safety = Self::average(&scenario_results, |r| r.safety_score);
        let avg_coherence = Self::average(&scenario_results, |r| r.coherence_score);
        let overall_score = Self::average(&scenario_results, |r| r.overall_score);

        // Stats by scenario type
        let mut results_by_type = std::collections::HashMap::new();
        for scenario_type in [
            ScenarioType::Combat,
            ScenarioType::Exploration,
            ScenarioType::Stealth,
            ScenarioType::Support,
            ScenarioType::Puzzle,
        ] {
            let type_results: Vec<_> = scenario_results
                .iter()
                .filter(|r| r.scenario_type == scenario_type)
                .collect();

            if !type_results.is_empty() {
                results_by_type.insert(
                    scenario_type,
                    TypeStats {
                        count: type_results.len(),
                        avg_validity: Self::average(&type_results, |r| r.validity_score),
                        avg_goal: Self::average(&type_results, |r| r.goal_score),
                        avg_safety: Self::average(&type_results, |r| r.safety_score),
                        avg_overall: Self::average(&type_results, |r| r.overall_score),
                    },
                );
            }
        }

        EvaluationResults {
            total_scenarios: self.scenarios.len(),
            passed,
            failed: self.scenarios.len() - passed,
            avg_validity,
            avg_goal_achievement,
            avg_safety,
            avg_coherence,
            overall_score,
            total_elapsed_ms,
            results_by_type,
            scenario_results,
        }
    }

    /// Evaluate a single scenario
    async fn evaluate_scenario(
        &self,
        scenario: &Scenario,
        client: Arc<dyn LlmClient>,
    ) -> ScenarioResult {
        let start = Instant::now();
        let mut errors = Vec::new();

        // Use the scenario's prompt directly
        let prompt = &scenario.prompt;

        // Get LLM response
        let raw_response = match client.complete(prompt).await {
            Ok(response) => response,
            Err(e) => {
                errors.push(format!("LLM error: {}", e));
                return ScenarioResult {
                    scenario_id: scenario.id.clone(),
                    scenario_type: scenario.scenario_type,
                    validity_score: 0.0,
                    goal_score: 0.0,
                    safety_score: 0.0,
                    coherence_score: 0.0,
                    overall_score: 0.0,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    raw_response: String::new(),
                    parsed_plan: None,
                    errors,
                };
            }
        };

        // Parse plan - use a permissive ToolRegistry for evaluation
        let tool_registry = ToolRegistry {
            tools: vec![
                astraweave_core::ToolSpec {
                    name: "move_to".to_string(),
                    args: std::collections::BTreeMap::new(),
                },
                astraweave_core::ToolSpec {
                    name: "throw".to_string(),
                    args: std::collections::BTreeMap::new(),
                },
                astraweave_core::ToolSpec {
                    name: "cover_fire".to_string(),
                    args: std::collections::BTreeMap::new(),
                },
                astraweave_core::ToolSpec {
                    name: "revive".to_string(),
                    args: std::collections::BTreeMap::new(),
                },
            ],
            constraints: astraweave_core::Constraints {
                enforce_cooldowns: false,
                enforce_los: false,
                enforce_stamina: false,
            },
        };

        let parsed_plan = match astraweave_llm::parse_llm_plan(&raw_response, &tool_registry) {
            Ok(plan) => Some(plan),
            Err(e) => {
                errors.push(format!("Parse error: {}", e));
                None
            }
        };

        // Score the plan
        let validity_score = if parsed_plan.is_some() { 1.0 } else { 0.0 };

        let goal_score = if let Some(ref plan) = parsed_plan {
            self.score_goal_achievement(&plan.steps, scenario)
        } else {
            0.0
        };

        let safety_score = if let Some(ref plan) = parsed_plan {
            self.score_safety(&plan.steps, scenario)
        } else {
            1.0 // No plan = no unsafe actions
        };

        let coherence_score = if let Some(ref plan) = parsed_plan {
            self.score_coherence(&plan.steps)
        } else {
            0.0
        };

        let overall_score = validity_score * self.weights.validity
            + goal_score * self.weights.goal_achievement
            + safety_score * self.weights.safety
            + coherence_score * self.weights.coherence;

        ScenarioResult {
            scenario_id: scenario.id.clone(),
            scenario_type: scenario.scenario_type,
            validity_score,
            goal_score,
            safety_score,
            coherence_score,
            overall_score,
            elapsed_ms: start.elapsed().as_millis() as u64,
            raw_response,
            parsed_plan,
            errors,
        }
    }

    /// Build a prompt from a scenario
    #[allow(dead_code)]
    fn build_prompt(&self, scenario: &Scenario) -> String {
        scenario.prompt.clone()
    }

    /// Score goal achievement (0.0 - 1.0)
    fn score_goal_achievement(&self, steps: &[ActionStep], scenario: &Scenario) -> f64 {
        if steps.is_empty() {
            return 0.0;
        }

        let mut score = 0.0;
        let expected = &scenario.expected_actions;

        // Check if expected actions appear in the plan
        for expected_action in expected {
            let action_present = steps.iter().any(|s| {
                matches!(
                    (expected_action.as_str(), s),
                    ("MoveTo", ActionStep::MoveTo { .. })
                        | ("Throw", ActionStep::Throw { .. })
                        | ("CoverFire", ActionStep::CoverFire { .. })
                        | ("Revive", ActionStep::Revive { .. })
                )
            });

            if action_present {
                score += 1.0 / expected.len() as f64;
            }
        }

        score.clamp(0.0, 1.0)
    }

    /// Score safety (0.0 - 1.0)
    fn score_safety(&self, steps: &[ActionStep], scenario: &Scenario) -> f64 {
        if steps.is_empty() {
            return 1.0;
        }

        let forbidden = &scenario.forbidden_actions;

        // Check for forbidden actions
        let violations = steps
            .iter()
            .filter(|s| {
                forbidden.iter().any(|f| {
                    matches!(
                        (f.as_str(), *s),
                        ("MoveTo", ActionStep::MoveTo { .. })
                            | ("Throw", ActionStep::Throw { .. })
                            | ("CoverFire", ActionStep::CoverFire { .. })
                            | ("Revive", ActionStep::Revive { .. })
                    )
                })
            })
            .count();

        if violations == 0 {
            1.0
        } else {
            (1.0 - (violations as f64 / steps.len() as f64)).clamp(0.0, 1.0)
        }
    }

    /// Score coherence (0.0 - 1.0)
    fn score_coherence(&self, steps: &[ActionStep]) -> f64 {
        if steps.is_empty() {
            return 0.0;
        }

        // Simple heuristic: check for logical patterns
        // - MoveTo before CoverFire (good)
        // - Multiple identical action types in a row (bad)

        let mut coherence_points = 0;
        let mut total_checks = 0;

        for i in 1..steps.len() {
            let prev = &steps[i - 1];
            let curr = &steps[i];

            total_checks += 1;

            // Good: MoveTo before CoverFire
            if matches!(prev, ActionStep::MoveTo { .. })
                && matches!(curr, ActionStep::CoverFire { .. })
            {
                coherence_points += 1;
            }

            // Bad: Identical consecutive action types (except MoveTo)
            let same_type = matches!(
                (prev, curr),
                (ActionStep::MoveTo { .. }, ActionStep::MoveTo { .. })
                    | (ActionStep::Throw { .. }, ActionStep::Throw { .. })
                    | (ActionStep::CoverFire { .. }, ActionStep::CoverFire { .. })
                    | (ActionStep::Revive { .. }, ActionStep::Revive { .. })
            );

            if same_type && !matches!(prev, ActionStep::MoveTo { .. }) {
                coherence_points -= 1;
            }
        }

        if total_checks == 0 {
            return 1.0;
        }

        ((coherence_points as f64 / total_checks as f64) + 1.0) / 2.0 // Normalize to 0-1
    }

    /// Helper to calculate averages
    fn average<T, F>(items: &[T], extractor: F) -> f64
    where
        F: Fn(&T) -> f64,
    {
        if items.is_empty() {
            return 0.0;
        }
        items.iter().map(extractor).sum::<f64>() / items.len() as f64
    }

    /// Create default test scenarios
    fn create_default_scenarios() -> Vec<Scenario> {
        vec![
            Scenario {
                id: "combat_basic".to_string(),
                scenario_type: ScenarioType::Combat,
                description: "Enemy visible. Engage tactically.".to_string(),
                prompt: "Enemy at (10, 5). You're at (3, 3). Generate a JSON plan with steps to engage:\n\
                         {\"plan_id\": \"id\", \"steps\": [{\"act\": \"MoveTo\", \"x\": 10, \"y\": 5}, {\"act\": \"CoverFire\", \"target_id\": 99, \"duration\": 2.0}]}".to_string(),
                expected_actions: vec!["MoveTo".to_string(), "CoverFire".to_string()],
                forbidden_actions: vec![],
            },
            Scenario {
                id: "combat_grenade".to_string(),
                scenario_type: ScenarioType::Combat,
                description: "Multiple enemies. Use smoke tactically.".to_string(),
                prompt: "Three enemies at (12, 8). You have smoke. Generate JSON plan:\n\
                         {\"plan_id\": \"id\", \"steps\": [{\"act\": \"Throw\", \"item\": \"smoke\", \"x\": 12, \"y\": 8}]}".to_string(),
                expected_actions: vec!["Throw".to_string()],
                forbidden_actions: vec![],
            },
            Scenario {
                id: "exploration".to_string(),
                scenario_type: ScenarioType::Exploration,
                description: "Navigate to waypoint.".to_string(),
                prompt: "Move to (15, 10). Generate JSON plan:\n\
                         {\"plan_id\": \"id\", \"steps\": [{\"act\": \"MoveTo\", \"x\": 15, \"y\": 10}]}".to_string(),
                expected_actions: vec!["MoveTo".to_string()],
                forbidden_actions: vec!["CoverFire".to_string()],
            },
            Scenario {
                id: "stealth".to_string(),
                scenario_type: ScenarioType::Stealth,
                description: "Avoid detection while moving.".to_string(),
                prompt: "Guard at (8, 4). Reach (15, 10) quietly. Generate JSON plan (MoveTo only):\n\
                         {\"plan_id\": \"id\", \"steps\": [{\"act\": \"MoveTo\", \"x\": 15, \"y\": 10}]}".to_string(),
                expected_actions: vec!["MoveTo".to_string()],
                forbidden_actions: vec!["CoverFire".to_string(), "Throw".to_string()],
            },
            Scenario {
                id: "support".to_string(),
                scenario_type: ScenarioType::Support,
                description: "Revive downed ally.".to_string(),
                prompt: "Ally down at (5, 5). Revive them. Generate JSON plan:\n\
                         {\"plan_id\": \"id\", \"steps\": [{\"act\": \"MoveTo\", \"x\": 5, \"y\": 5}, {\"act\": \"Revive\", \"ally_id\": 42}]}".to_string(),
                expected_actions: vec!["MoveTo".to_string(), "Revive".to_string()],
                forbidden_actions: vec!["CoverFire".to_string()],
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_llm::MockLlm;

    #[tokio::test]
    async fn test_evaluation_basic() {
        let suite = EvaluationSuite::default();
        let client = Arc::new(MockLlm);

        let results = suite.evaluate(client).await;

        assert!(results.total_scenarios > 0);
        assert!(results.overall_score > 0.0);
    }

    #[tokio::test]
    async fn test_scenario_scoring() {
        let suite = EvaluationSuite::default();
        let scenario = &suite.scenarios[0]; // Combat scenario

        let plan = PlanIntent {
            plan_id: "test".to_string(),
            steps: vec![
                ActionStep::MoveTo {
                    x: 10,
                    y: 5,
                    speed: None,
                },
                ActionStep::CoverFire {
                    target_id: 99,
                    duration: 2.0,
                },
            ],
        };

        let goal_score = suite.score_goal_achievement(&plan.steps, scenario);
        assert!(goal_score > 0.5); // Should score well for expected actions

        let safety_score = suite.score_safety(&plan.steps, scenario);
        assert_eq!(safety_score, 1.0); // No forbidden actions

        let coherence_score = suite.score_coherence(&plan.steps);
        assert!(coherence_score > 0.0); // MoveTo before CoverFire is coherent
    }
}
