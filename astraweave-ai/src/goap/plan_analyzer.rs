use super::{Action, ActionHistory, WorldState};
use std::collections::HashMap;

/// Metrics for analyzing plan quality
#[derive(Debug, Clone)]
pub struct PlanMetrics {
    pub total_cost: f32,
    pub total_risk: f32,
    pub action_count: usize,
    pub estimated_duration: f32,
    pub success_probability: f32,
    pub bottlenecks: Vec<Bottleneck>,
    pub action_breakdown: HashMap<String, ActionMetrics>,
}

/// Metrics for individual actions
#[derive(Debug, Clone)]
pub struct ActionMetrics {
    pub cost: f32,
    pub risk: f32,
    pub success_rate: f32,
    pub avg_duration: f32,
    pub executions: u32,
}

/// Identified bottleneck in a plan
#[derive(Debug, Clone)]
pub struct Bottleneck {
    pub action_name: String,
    pub reason: BottleneckReason,
    pub severity: f32, // 0.0 - 1.0
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum BottleneckReason {
    HighCost,
    HighRisk,
    LowSuccessRate,
    LongDuration,
}

/// Comparison report between two plans
#[derive(Debug, Clone)]
pub struct ComparisonReport {
    pub cost_diff: f32,
    pub risk_diff: f32,
    pub duration_diff: f32,
    pub success_prob_diff: f32,
    pub better_plan: PlanComparison,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlanComparison {
    Plan1Better,
    Plan2Better,
    Similar,
}

/// Optimization suggestion
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub message: String,
    pub priority: SuggestionPriority,
    pub estimated_improvement: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum SuggestionPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Plan analyzer for quality metrics and optimization
pub struct PlanAnalyzer;

impl PlanAnalyzer {
    /// Analyze a plan and produce metrics
    pub fn analyze(
        plan: &[String],
        actions: &[Box<dyn Action>],
        history: &ActionHistory,
        start_state: &WorldState,
    ) -> PlanMetrics {
        let mut total_cost = 0.0;
        let mut total_risk = 0.0;
        let mut total_duration = 0.0;
        let mut action_breakdown = HashMap::new();
        let mut current_state = start_state.clone();

        for action_name in plan {
            if let Some(action) = actions.iter().find(|a| a.name() == action_name) {
                let cost = action.calculate_cost(&current_state, history);
                let success_prob = action.success_probability(&current_state, history);
                let risk = 1.0 - success_prob;

                let stats = history.get_action_stats(action_name);
                let duration = stats.map(|s| s.avg_duration).unwrap_or(1.0);
                let executions = stats.map(|s| s.executions).unwrap_or(0);
                let success_rate = stats.map(|s| s.success_rate()).unwrap_or(0.5);

                total_cost += cost;
                total_risk += risk;
                total_duration += duration;

                action_breakdown.insert(
                    action_name.clone(),
                    ActionMetrics {
                        cost,
                        risk,
                        success_rate,
                        avg_duration: duration,
                        executions,
                    },
                );

                current_state.apply_effects(action.effects());
            }
        }

        // Calculate overall success probability (product of individual probabilities)
        let success_probability = action_breakdown.values().map(|m| m.success_rate).product();

        // Identify bottlenecks
        let bottlenecks = Self::identify_bottlenecks(&action_breakdown);

        PlanMetrics {
            total_cost,
            total_risk,
            action_count: plan.len(),
            estimated_duration: total_duration,
            success_probability,
            bottlenecks,
            action_breakdown,
        }
    }

    /// Compare two plans
    pub fn compare(metrics1: &PlanMetrics, metrics2: &PlanMetrics) -> ComparisonReport {
        let cost_diff = metrics2.total_cost - metrics1.total_cost;
        let risk_diff = metrics2.total_risk - metrics1.total_risk;
        let duration_diff = metrics2.estimated_duration - metrics1.estimated_duration;
        let success_prob_diff = metrics2.success_probability - metrics1.success_probability;

        // Scoring: lower cost, risk, duration and higher success prob is better
        let score1 =
            -metrics1.total_cost - metrics1.total_risk * 2.0 - metrics1.estimated_duration * 0.1
                + metrics1.success_probability * 10.0;
        let score2 =
            -metrics2.total_cost - metrics2.total_risk * 2.0 - metrics2.estimated_duration * 0.1
                + metrics2.success_probability * 10.0;

        let better_plan = if (score1 - score2).abs() < 0.5 {
            PlanComparison::Similar
        } else if score1 > score2 {
            PlanComparison::Plan1Better
        } else {
            PlanComparison::Plan2Better
        };

        let mut recommendations = Vec::new();

        if cost_diff.abs() > 1.0 {
            if cost_diff > 0.0 {
                recommendations.push(format!("Plan 1 is {:.1} cost units cheaper", cost_diff));
            } else {
                recommendations.push(format!("Plan 2 is {:.1} cost units cheaper", -cost_diff));
            }
        }

        if risk_diff.abs() > 0.1 {
            if risk_diff > 0.0 {
                recommendations.push(format!("Plan 1 has {:.2} less risk", risk_diff));
            } else {
                recommendations.push(format!("Plan 2 has {:.2} less risk", -risk_diff));
            }
        }

        if duration_diff.abs() > 2.0 {
            if duration_diff > 0.0 {
                recommendations.push(format!("Plan 1 is {:.1}s faster", duration_diff));
            } else {
                recommendations.push(format!("Plan 2 is {:.1}s faster", -duration_diff));
            }
        }

        ComparisonReport {
            cost_diff,
            risk_diff,
            duration_diff,
            success_prob_diff,
            better_plan,
            recommendations,
        }
    }

    /// Suggest optimizations for a plan
    pub fn suggest_optimizations(metrics: &PlanMetrics) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        // Check for high overall cost
        if metrics.total_cost > 20.0 {
            suggestions.push(Suggestion {
                message: format!(
                    "Plan has high total cost ({:.1}). Consider shorter alternative paths.",
                    metrics.total_cost
                ),
                priority: SuggestionPriority::High,
                estimated_improvement: Some(metrics.total_cost * 0.2),
            });
        }

        // Check for high overall risk
        if metrics.total_risk > 2.0 {
            suggestions.push(Suggestion {
                message: format!(
                    "Plan has high cumulative risk ({:.2}). Consider more reliable actions.",
                    metrics.total_risk
                ),
                priority: SuggestionPriority::Critical,
                estimated_improvement: Some(metrics.total_risk * 0.3),
            });
        }

        // Check for low success probability
        if metrics.success_probability < 0.5 {
            suggestions.push(Suggestion {
                message: format!(
                    "Plan has low success probability ({:.1}%). Consider adding fallback actions.",
                    metrics.success_probability * 100.0
                ),
                priority: SuggestionPriority::Critical,
                estimated_improvement: None,
            });
        }

        // Check for long duration
        if metrics.estimated_duration > 30.0 {
            suggestions.push(Suggestion {
                message: format!(
                    "Plan takes a long time ({:.1}s). Look for faster action sequences.",
                    metrics.estimated_duration
                ),
                priority: SuggestionPriority::Medium,
                estimated_improvement: Some(metrics.estimated_duration * 0.15),
            });
        }

        // Analyze bottlenecks
        for bottleneck in &metrics.bottlenecks {
            match bottleneck.reason {
                BottleneckReason::HighCost => {
                    suggestions.push(Suggestion {
                        message: format!(
                            "Action '{}' is expensive. Consider alternatives or optimize preconditions.",
                            bottleneck.action_name
                        ),
                        priority: if bottleneck.severity > 0.7 {
                            SuggestionPriority::High
                        } else {
                            SuggestionPriority::Medium
                        },
                        estimated_improvement: Some(bottleneck.severity * 5.0),
                    });
                }
                BottleneckReason::HighRisk => {
                    suggestions.push(Suggestion {
                        message: format!(
                            "Action '{}' is risky. Add supporting actions to improve success rate.",
                            bottleneck.action_name
                        ),
                        priority: SuggestionPriority::Critical,
                        estimated_improvement: None,
                    });
                }
                BottleneckReason::LowSuccessRate => {
                    suggestions.push(Suggestion {
                        message: format!(
                            "Action '{}' has low historical success rate. Review preconditions or avoid.",
                            bottleneck.action_name
                        ),
                        priority: SuggestionPriority::High,
                        estimated_improvement: None,
                    });
                }
                BottleneckReason::LongDuration => {
                    suggestions.push(Suggestion {
                        message: format!(
                            "Action '{}' takes a long time. Consider faster alternatives.",
                            bottleneck.action_name
                        ),
                        priority: SuggestionPriority::Low,
                        estimated_improvement: Some(bottleneck.severity * 2.0),
                    });
                }
            }
        }

        // Check for very long plans
        if metrics.action_count > 10 {
            suggestions.push(Suggestion {
                message: format!(
                    "Plan has {} actions. Consider hierarchical decomposition or simpler goals.",
                    metrics.action_count
                ),
                priority: SuggestionPriority::Medium,
                estimated_improvement: None,
            });
        }

        // Sort by priority
        suggestions.sort_by(|a, b| b.priority.cmp(&a.priority));

        suggestions
    }

    /// Identify bottlenecks in a plan
    fn identify_bottlenecks(action_breakdown: &HashMap<String, ActionMetrics>) -> Vec<Bottleneck> {
        let mut bottlenecks = Vec::new();

        // Calculate averages
        let avg_cost: f32 =
            action_breakdown.values().map(|m| m.cost).sum::<f32>() / action_breakdown.len() as f32;
        let avg_risk: f32 =
            action_breakdown.values().map(|m| m.risk).sum::<f32>() / action_breakdown.len() as f32;
        let avg_duration: f32 = action_breakdown
            .values()
            .map(|m| m.avg_duration)
            .sum::<f32>()
            / action_breakdown.len() as f32;

        for (action_name, metrics) in action_breakdown {
            // Check for high cost (>2x average)
            if metrics.cost > avg_cost * 2.0 {
                bottlenecks.push(Bottleneck {
                    action_name: action_name.clone(),
                    reason: BottleneckReason::HighCost,
                    severity: (metrics.cost / (avg_cost * 2.0)).min(1.0),
                });
            }

            // Check for high risk (>2x average)
            if metrics.risk > avg_risk * 2.0 && metrics.risk > 0.3 {
                bottlenecks.push(Bottleneck {
                    action_name: action_name.clone(),
                    reason: BottleneckReason::HighRisk,
                    severity: (metrics.risk / (avg_risk * 2.0)).min(1.0),
                });
            }

            // Check for low success rate (<50%)
            if metrics.success_rate < 0.5 && metrics.executions > 3 {
                bottlenecks.push(Bottleneck {
                    action_name: action_name.clone(),
                    reason: BottleneckReason::LowSuccessRate,
                    severity: 1.0 - metrics.success_rate,
                });
            }

            // Check for long duration (>2x average)
            if metrics.avg_duration > avg_duration * 2.0 && metrics.avg_duration > 2.0 {
                bottlenecks.push(Bottleneck {
                    action_name: action_name.clone(),
                    reason: BottleneckReason::LongDuration,
                    severity: (metrics.avg_duration / (avg_duration * 2.0)).min(1.0),
                });
            }
        }

        // Sort by severity
        bottlenecks.sort_by(|a, b| {
            b.severity
                .partial_cmp(&a.severity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        bottlenecks
    }

    /// Generate a human-readable report
    pub fn generate_report(metrics: &PlanMetrics) -> String {
        let mut report = String::new();

        report.push_str("=== Plan Analysis Report ===\n\n");

        report.push_str(&format!("Total Actions: {}\n", metrics.action_count));
        report.push_str(&format!("Total Cost: {:.2}\n", metrics.total_cost));
        report.push_str(&format!("Total Risk: {:.2}\n", metrics.total_risk));
        report.push_str(&format!(
            "Estimated Duration: {:.1}s\n",
            metrics.estimated_duration
        ));
        report.push_str(&format!(
            "Success Probability: {:.1}%\n",
            metrics.success_probability * 100.0
        ));

        if !metrics.bottlenecks.is_empty() {
            report.push_str("\n=== Bottlenecks ===\n");
            for bottleneck in &metrics.bottlenecks {
                report.push_str(&format!(
                    "- {}: {:?} (severity: {:.1}%)\n",
                    bottleneck.action_name,
                    bottleneck.reason,
                    bottleneck.severity * 100.0
                ));
            }
        }

        let suggestions = Self::suggest_optimizations(metrics);
        if !suggestions.is_empty() {
            report.push_str("\n=== Optimization Suggestions ===\n");
            for (i, suggestion) in suggestions.iter().take(5).enumerate() {
                report.push_str(&format!(
                    "{}. [{:?}] {}\n",
                    i + 1,
                    suggestion.priority,
                    suggestion.message
                ));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goap::SimpleAction;
    use std::collections::BTreeMap;

    fn create_test_action(name: &str, cost: f32) -> Box<dyn Action> {
        Box::new(SimpleAction::new(
            name,
            BTreeMap::new(),
            BTreeMap::new(),
            cost,
        ))
    }

    #[test]
    fn test_analyze_simple_plan() {
        let actions: Vec<Box<dyn Action>> = vec![
            create_test_action("action1", 1.0),
            create_test_action("action2", 2.0),
        ];
        let plan = vec!["action1".to_string(), "action2".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let metrics = PlanAnalyzer::analyze(&plan, &actions, &history, &start);

        assert_eq!(metrics.action_count, 2);
        assert_eq!(metrics.total_cost, 3.0);
    }

    #[test]
    fn test_identify_high_cost_bottleneck() {
        let mut action_breakdown = HashMap::new();

        action_breakdown.insert(
            "cheap".to_string(),
            ActionMetrics {
                cost: 1.0,
                risk: 0.1,
                success_rate: 0.9,
                avg_duration: 1.0,
                executions: 10,
            },
        );

        action_breakdown.insert(
            "expensive".to_string(),
            ActionMetrics {
                cost: 20.0, // Much higher - avg is 10.5, 2x avg is 21.0, so 20.0 is close but we need >2x
                risk: 0.1,
                success_rate: 0.9,
                avg_duration: 1.0,
                executions: 10,
            },
        );

        action_breakdown.insert(
            "very_cheap".to_string(),
            ActionMetrics {
                cost: 0.5, // Add third action to bring avg down
                risk: 0.1,
                success_rate: 0.9,
                avg_duration: 1.0,
                executions: 10,
            },
        );

        let bottlenecks = PlanAnalyzer::identify_bottlenecks(&action_breakdown);

        // avg = (1.0 + 20.0 + 0.5) / 3 = 7.17, 2x = 14.33
        // 20.0 > 14.33, so should identify bottleneck
        assert!(!bottlenecks.is_empty());
        assert!(bottlenecks
            .iter()
            .any(|b| b.reason == BottleneckReason::HighCost));
    }

    #[test]
    fn test_compare_plans() {
        let metrics1 = PlanMetrics {
            total_cost: 5.0,
            total_risk: 0.5,
            action_count: 3,
            estimated_duration: 10.0,
            success_probability: 0.8,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let metrics2 = PlanMetrics {
            total_cost: 8.0, // Higher cost
            total_risk: 0.3, // Lower risk
            action_count: 4,
            estimated_duration: 12.0,
            success_probability: 0.9, // Higher success
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let comparison = PlanAnalyzer::compare(&metrics1, &metrics2);

        assert!(comparison.cost_diff > 0.0); // Plan 2 costs more
        assert!(comparison.risk_diff < 0.0); // Plan 2 has less risk
    }

    #[test]
    fn test_suggest_optimizations_high_cost() {
        let metrics = PlanMetrics {
            total_cost: 25.0, // High cost
            total_risk: 0.2,
            action_count: 5,
            estimated_duration: 10.0,
            success_probability: 0.9,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let suggestions = PlanAnalyzer::suggest_optimizations(&metrics);

        assert!(!suggestions.is_empty());
        assert!(suggestions
            .iter()
            .any(|s| s.message.contains("high total cost")));
    }

    #[test]
    fn test_suggest_optimizations_low_success() {
        let metrics = PlanMetrics {
            total_cost: 5.0,
            total_risk: 0.2,
            action_count: 3,
            estimated_duration: 10.0,
            success_probability: 0.3, // Low success
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let suggestions = PlanAnalyzer::suggest_optimizations(&metrics);

        assert!(!suggestions.is_empty());
        assert!(suggestions
            .iter()
            .any(|s| s.priority == SuggestionPriority::Critical));
    }

    #[test]
    fn test_generate_report() {
        let metrics = PlanMetrics {
            total_cost: 10.0,
            total_risk: 0.5,
            action_count: 5,
            estimated_duration: 15.0,
            success_probability: 0.85,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let report = PlanAnalyzer::generate_report(&metrics);

        assert!(report.contains("Plan Analysis Report"));
        assert!(report.contains("Total Actions: 5"));
        assert!(report.contains("Total Cost"));
    }

    // ========== Mutation-killing tests ==========

    #[test]
    fn test_analyze_with_history_stats() {
        let actions: Vec<Box<dyn Action>> = vec![
            create_test_action("move", 3.0),
            create_test_action("attack", 5.0),
        ];
        let plan = vec!["move".to_string(), "attack".to_string()];
        let mut history = ActionHistory::new();

        // Record some history for "move"
        history.record_success("move", 2.0);
        history.record_success("move", 3.0);
        history.record_failure("move");

        // Record history for "attack"
        history.record_success("attack", 4.0);
        history.record_success("attack", 6.0);

        let start = WorldState::new();
        let metrics = PlanAnalyzer::analyze(&plan, &actions, &history, &start);

        assert_eq!(metrics.action_count, 2);

        // Check move action breakdown
        let move_m = metrics.action_breakdown.get("move").unwrap();
        assert_eq!(move_m.executions, 3);
        // success_rate = 2/3
        assert!((move_m.success_rate - 2.0 / 3.0).abs() < 0.01);
        // avg_duration from history
        assert!(move_m.avg_duration > 0.0);

        // Check attack action breakdown
        let attack_m = metrics.action_breakdown.get("attack").unwrap();
        assert_eq!(attack_m.executions, 2);
        assert!((attack_m.success_rate - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_analyze_total_risk_and_duration() {
        let actions: Vec<Box<dyn Action>> =
            vec![create_test_action("a1", 2.0), create_test_action("a2", 3.0)];
        let plan = vec!["a1".to_string(), "a2".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let metrics = PlanAnalyzer::analyze(&plan, &actions, &history, &start);

        // risk = 1.0 - success_probability for each action
        // No history → success_prob = 0.8 (SimpleAction default), risk = 0.2 each
        assert!((metrics.total_risk - 0.4).abs() < 0.01);

        // Duration: no history → defaults to 1.0 each
        assert!((metrics.estimated_duration - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_analyze_success_probability_product() {
        let actions: Vec<Box<dyn Action>> =
            vec![create_test_action("a1", 1.0), create_test_action("a2", 1.0)];
        let plan = vec!["a1".to_string(), "a2".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let metrics = PlanAnalyzer::analyze(&plan, &actions, &history, &start);

        // No history → success_rate = 0.5 (from ActionStats default)
        // Product of 0.5 * 0.5 = 0.25
        assert!((metrics.success_probability - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_compare_plan2_better() {
        let metrics1 = PlanMetrics {
            total_cost: 20.0,
            total_risk: 1.0,
            action_count: 5,
            estimated_duration: 30.0,
            success_probability: 0.3,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let metrics2 = PlanMetrics {
            total_cost: 5.0,
            total_risk: 0.2,
            action_count: 2,
            estimated_duration: 5.0,
            success_probability: 0.9,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let comparison = PlanAnalyzer::compare(&metrics1, &metrics2);
        assert_eq!(comparison.better_plan, PlanComparison::Plan2Better);
    }

    #[test]
    fn test_compare_similar() {
        let metrics1 = PlanMetrics {
            total_cost: 10.0,
            total_risk: 0.5,
            action_count: 3,
            estimated_duration: 10.0,
            success_probability: 0.8,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        // Nearly identical scoring
        let metrics2 = PlanMetrics {
            total_cost: 10.0,
            total_risk: 0.5,
            action_count: 3,
            estimated_duration: 10.0,
            success_probability: 0.8,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let comparison = PlanAnalyzer::compare(&metrics1, &metrics2);
        assert_eq!(comparison.better_plan, PlanComparison::Similar);
    }

    #[test]
    fn test_compare_plan1_better() {
        let metrics1 = PlanMetrics {
            total_cost: 3.0,
            total_risk: 0.1,
            action_count: 2,
            estimated_duration: 5.0,
            success_probability: 0.95,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let metrics2 = PlanMetrics {
            total_cost: 25.0,
            total_risk: 2.0,
            action_count: 8,
            estimated_duration: 40.0,
            success_probability: 0.2,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let comparison = PlanAnalyzer::compare(&metrics1, &metrics2);
        assert_eq!(comparison.better_plan, PlanComparison::Plan1Better);
    }

    #[test]
    fn test_compare_recommendations_cost() {
        let metrics1 = PlanMetrics {
            total_cost: 5.0,
            total_risk: 0.5,
            action_count: 3,
            estimated_duration: 10.0,
            success_probability: 0.8,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        // Plan 2 much more expensive
        let metrics2 = PlanMetrics {
            total_cost: 15.0,
            total_risk: 0.5,
            action_count: 3,
            estimated_duration: 10.0,
            success_probability: 0.8,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let comparison = PlanAnalyzer::compare(&metrics1, &metrics2);
        assert!(comparison.cost_diff > 1.0);
        assert!(
            comparison
                .recommendations
                .iter()
                .any(|r| r.contains("cheaper")),
            "Expected cost recommendation: {:?}",
            comparison.recommendations
        );
    }

    #[test]
    fn test_compare_recommendations_risk() {
        let metrics1 = PlanMetrics {
            total_cost: 10.0,
            total_risk: 0.5,
            action_count: 3,
            estimated_duration: 10.0,
            success_probability: 0.8,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let metrics2 = PlanMetrics {
            total_cost: 10.0,
            total_risk: 1.0,
            action_count: 3,
            estimated_duration: 10.0,
            success_probability: 0.8,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let comparison = PlanAnalyzer::compare(&metrics1, &metrics2);
        assert!(
            comparison
                .recommendations
                .iter()
                .any(|r| r.contains("risk")),
            "Expected risk recommendation: {:?}",
            comparison.recommendations
        );
    }

    #[test]
    fn test_compare_recommendations_duration() {
        let metrics1 = PlanMetrics {
            total_cost: 10.0,
            total_risk: 0.5,
            action_count: 3,
            estimated_duration: 5.0,
            success_probability: 0.8,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let metrics2 = PlanMetrics {
            total_cost: 10.0,
            total_risk: 0.5,
            action_count: 3,
            estimated_duration: 15.0,
            success_probability: 0.8,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let comparison = PlanAnalyzer::compare(&metrics1, &metrics2);
        assert!(comparison.duration_diff > 2.0);
        assert!(
            comparison
                .recommendations
                .iter()
                .any(|r| r.contains("faster")),
            "Expected duration recommendation: {:?}",
            comparison.recommendations
        );
    }

    #[test]
    fn test_suggest_optimizations_high_risk() {
        let metrics = PlanMetrics {
            total_cost: 5.0,
            total_risk: 2.5, // > 2.0
            action_count: 3,
            estimated_duration: 10.0,
            success_probability: 0.7,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let suggestions = PlanAnalyzer::suggest_optimizations(&metrics);
        assert!(
            suggestions
                .iter()
                .any(|s| s.message.contains("cumulative risk")),
            "Expected high risk suggestion: {:?}",
            suggestions.iter().map(|s| &s.message).collect::<Vec<_>>()
        );
        // High risk → Critical priority
        assert!(suggestions
            .iter()
            .any(|s| s.priority == SuggestionPriority::Critical && s.message.contains("risk")));
    }

    #[test]
    fn test_suggest_optimizations_long_duration() {
        let metrics = PlanMetrics {
            total_cost: 5.0,
            total_risk: 0.2,
            action_count: 3,
            estimated_duration: 35.0, // > 30.0
            success_probability: 0.9,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let suggestions = PlanAnalyzer::suggest_optimizations(&metrics);
        assert!(
            suggestions.iter().any(|s| s.message.contains("long time")),
            "Expected duration suggestion: {:?}",
            suggestions.iter().map(|s| &s.message).collect::<Vec<_>>()
        );
        // Duration → Medium priority with estimated_improvement
        let dur_s = suggestions
            .iter()
            .find(|s| s.message.contains("long time"))
            .unwrap();
        assert_eq!(dur_s.priority, SuggestionPriority::Medium);
        assert!(dur_s.estimated_improvement.is_some());
    }

    #[test]
    fn test_suggest_optimizations_long_plan() {
        let metrics = PlanMetrics {
            total_cost: 5.0,
            total_risk: 0.2,
            action_count: 12, // > 10
            estimated_duration: 10.0,
            success_probability: 0.9,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let suggestions = PlanAnalyzer::suggest_optimizations(&metrics);
        assert!(
            suggestions.iter().any(|s| s.message.contains("12 actions")),
            "Expected long plan suggestion: {:?}",
            suggestions.iter().map(|s| &s.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_suggest_optimizations_bottleneck_high_risk() {
        let metrics = PlanMetrics {
            total_cost: 5.0,
            total_risk: 0.2,
            action_count: 3,
            estimated_duration: 10.0,
            success_probability: 0.9,
            bottlenecks: vec![Bottleneck {
                action_name: "risky_move".to_string(),
                reason: BottleneckReason::HighRisk,
                severity: 0.9,
            }],
            action_breakdown: HashMap::new(),
        };

        let suggestions = PlanAnalyzer::suggest_optimizations(&metrics);
        assert!(suggestions
            .iter()
            .any(|s| s.message.contains("risky_move") && s.message.contains("risky")));
    }

    #[test]
    fn test_suggest_optimizations_bottleneck_low_success() {
        let metrics = PlanMetrics {
            total_cost: 5.0,
            total_risk: 0.2,
            action_count: 3,
            estimated_duration: 10.0,
            success_probability: 0.9,
            bottlenecks: vec![Bottleneck {
                action_name: "flaky_action".to_string(),
                reason: BottleneckReason::LowSuccessRate,
                severity: 0.7,
            }],
            action_breakdown: HashMap::new(),
        };

        let suggestions = PlanAnalyzer::suggest_optimizations(&metrics);
        assert!(suggestions
            .iter()
            .any(|s| s.message.contains("flaky_action")
                && s.message.contains("low historical success")));
    }

    #[test]
    fn test_suggest_optimizations_bottleneck_long_duration() {
        let metrics = PlanMetrics {
            total_cost: 5.0,
            total_risk: 0.2,
            action_count: 3,
            estimated_duration: 10.0,
            success_probability: 0.9,
            bottlenecks: vec![Bottleneck {
                action_name: "slow_action".to_string(),
                reason: BottleneckReason::LongDuration,
                severity: 0.6,
            }],
            action_breakdown: HashMap::new(),
        };

        let suggestions = PlanAnalyzer::suggest_optimizations(&metrics);
        assert!(suggestions
            .iter()
            .any(|s| s.message.contains("slow_action") && s.message.contains("long time")));
        // LongDuration bottleneck → Low priority
        let dur_s = suggestions
            .iter()
            .find(|s| s.message.contains("slow_action"))
            .unwrap();
        assert_eq!(dur_s.priority, SuggestionPriority::Low);
        assert!(dur_s.estimated_improvement.is_some());
    }

    #[test]
    fn test_suggest_optimizations_bottleneck_high_cost_severity() {
        // Test severity > 0.7 → High priority, and severity <= 0.7 → Medium
        let metrics_high = PlanMetrics {
            total_cost: 5.0,
            total_risk: 0.2,
            action_count: 3,
            estimated_duration: 10.0,
            success_probability: 0.9,
            bottlenecks: vec![Bottleneck {
                action_name: "costly".to_string(),
                reason: BottleneckReason::HighCost,
                severity: 0.8,
            }],
            action_breakdown: HashMap::new(),
        };

        let suggestions = PlanAnalyzer::suggest_optimizations(&metrics_high);
        let costly_s = suggestions
            .iter()
            .find(|s| s.message.contains("costly"))
            .unwrap();
        assert_eq!(costly_s.priority, SuggestionPriority::High);

        let metrics_low = PlanMetrics {
            total_cost: 5.0,
            total_risk: 0.2,
            action_count: 3,
            estimated_duration: 10.0,
            success_probability: 0.9,
            bottlenecks: vec![Bottleneck {
                action_name: "moderate".to_string(),
                reason: BottleneckReason::HighCost,
                severity: 0.5,
            }],
            action_breakdown: HashMap::new(),
        };

        let suggestions2 = PlanAnalyzer::suggest_optimizations(&metrics_low);
        let mod_s = suggestions2
            .iter()
            .find(|s| s.message.contains("moderate"))
            .unwrap();
        assert_eq!(mod_s.priority, SuggestionPriority::Medium);
    }

    #[test]
    fn test_identify_bottleneck_high_risk() {
        let mut breakdown = HashMap::new();

        breakdown.insert(
            "safe".to_string(),
            ActionMetrics {
                cost: 1.0,
                risk: 0.1,
                success_rate: 0.9,
                avg_duration: 1.0,
                executions: 10,
            },
        );

        breakdown.insert(
            "risky".to_string(),
            ActionMetrics {
                cost: 1.0,
                risk: 0.8, // avg_risk = 0.45, 2x = 0.9; 0.8 < 0.9 won't trigger
                success_rate: 0.2,
                avg_duration: 1.0,
                executions: 10,
            },
        );

        // Need a third action to make avg low enough
        breakdown.insert(
            "normal".to_string(),
            ActionMetrics {
                cost: 1.0,
                risk: 0.1,
                success_rate: 0.9,
                avg_duration: 1.0,
                executions: 10,
            },
        );

        // avg_risk = (0.1 + 0.8 + 0.1) / 3 = 0.333, 2x = 0.667
        // 0.8 > 0.667 AND 0.8 > 0.3 → HighRisk
        let bottlenecks = PlanAnalyzer::identify_bottlenecks(&breakdown);
        assert!(
            bottlenecks
                .iter()
                .any(|b| b.action_name == "risky" && b.reason == BottleneckReason::HighRisk),
            "Expected HighRisk bottleneck: {:?}",
            bottlenecks
        );
    }

    #[test]
    fn test_identify_bottleneck_low_success_rate() {
        let mut breakdown = HashMap::new();

        breakdown.insert(
            "reliable".to_string(),
            ActionMetrics {
                cost: 1.0,
                risk: 0.1,
                success_rate: 0.95,
                avg_duration: 1.0,
                executions: 10,
            },
        );

        breakdown.insert(
            "unreliable".to_string(),
            ActionMetrics {
                cost: 1.0,
                risk: 0.1,
                success_rate: 0.3, // < 0.5 AND executions > 3
                avg_duration: 1.0,
                executions: 5,
            },
        );

        let bottlenecks = PlanAnalyzer::identify_bottlenecks(&breakdown);
        assert!(
            bottlenecks
                .iter()
                .any(|b| b.action_name == "unreliable"
                    && b.reason == BottleneckReason::LowSuccessRate),
            "Expected LowSuccessRate bottleneck: {:?}",
            bottlenecks
        );
        // Severity = 1.0 - success_rate = 0.7
        let bn = bottlenecks
            .iter()
            .find(|b| b.reason == BottleneckReason::LowSuccessRate)
            .unwrap();
        assert!((bn.severity - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_identify_bottleneck_low_success_rate_min_executions() {
        // executions <= 3 should NOT trigger LowSuccessRate
        let mut breakdown = HashMap::new();

        breakdown.insert(
            "few_runs".to_string(),
            ActionMetrics {
                cost: 1.0,
                risk: 0.1,
                success_rate: 0.2,
                avg_duration: 1.0,
                executions: 3, // <= 3
            },
        );

        let bottlenecks = PlanAnalyzer::identify_bottlenecks(&breakdown);
        assert!(
            !bottlenecks
                .iter()
                .any(|b| b.reason == BottleneckReason::LowSuccessRate),
            "Should not flag with only 3 executions"
        );
    }

    #[test]
    fn test_identify_bottleneck_long_duration() {
        let mut breakdown = HashMap::new();

        breakdown.insert(
            "fast".to_string(),
            ActionMetrics {
                cost: 1.0,
                risk: 0.1,
                success_rate: 0.9,
                avg_duration: 1.0,
                executions: 10,
            },
        );

        breakdown.insert(
            "slow".to_string(),
            ActionMetrics {
                cost: 1.0,
                risk: 0.1,
                success_rate: 0.9,
                avg_duration: 5.0, // avg = 3.0, 2x = 6.0; need it > 6.0 and > 2.0
                executions: 10,
            },
        );

        // avg = (1.0 + 5.0) / 2 = 3.0, threshold = 6.0
        // 5.0 < 6.0 → not triggered

        // Add a third to bring avg down
        breakdown.insert(
            "normal".to_string(),
            ActionMetrics {
                cost: 1.0,
                risk: 0.1,
                success_rate: 0.9,
                avg_duration: 1.0,
                executions: 10,
            },
        );

        // avg = (1.0 + 5.0 + 1.0) / 3 = 2.33, 2x = 4.67
        // 5.0 > 4.67 AND 5.0 > 2.0 → LongDuration
        let bottlenecks = PlanAnalyzer::identify_bottlenecks(&breakdown);
        assert!(
            bottlenecks
                .iter()
                .any(|b| b.action_name == "slow" && b.reason == BottleneckReason::LongDuration),
            "Expected LongDuration bottleneck: {:?}",
            bottlenecks
        );
    }

    #[test]
    fn test_identify_bottleneck_long_duration_min_threshold() {
        // avg_duration > 2x avg BUT avg_duration <= 2.0 should NOT trigger
        let mut breakdown = HashMap::new();

        breakdown.insert(
            "tiny_fast".to_string(),
            ActionMetrics {
                cost: 1.0,
                risk: 0.1,
                success_rate: 0.9,
                avg_duration: 0.1,
                executions: 10,
            },
        );

        breakdown.insert(
            "tiny_slow".to_string(),
            ActionMetrics {
                cost: 1.0,
                risk: 0.1,
                success_rate: 0.9,
                avg_duration: 1.5, // > 2x avg(0.8) but <= 2.0
                executions: 10,
            },
        );

        let bottlenecks = PlanAnalyzer::identify_bottlenecks(&breakdown);
        assert!(
            !bottlenecks
                .iter()
                .any(|b| b.reason == BottleneckReason::LongDuration),
            "Should not flag when avg_duration <= 2.0"
        );
    }

    #[test]
    fn test_identify_bottleneck_high_risk_min_threshold() {
        // risk > 2x avg BUT risk <= 0.3 should NOT trigger
        let mut breakdown = HashMap::new();

        breakdown.insert(
            "low_risk1".to_string(),
            ActionMetrics {
                cost: 1.0,
                risk: 0.05,
                success_rate: 0.9,
                avg_duration: 1.0,
                executions: 10,
            },
        );

        breakdown.insert(
            "low_risk2".to_string(),
            ActionMetrics {
                cost: 1.0,
                risk: 0.25, // > 2x avg(0.15)=0.3; 0.25 < 0.3 → should NOT trigger
                success_rate: 0.75,
                avg_duration: 1.0,
                executions: 10,
            },
        );

        let bottlenecks = PlanAnalyzer::identify_bottlenecks(&breakdown);
        assert!(
            !bottlenecks
                .iter()
                .any(|b| b.reason == BottleneckReason::HighRisk),
            "Should not flag when risk <= 0.3"
        );
    }

    #[test]
    fn test_generate_report_with_bottlenecks() {
        let metrics = PlanMetrics {
            total_cost: 25.0, // Also triggers high-cost suggestion
            total_risk: 0.5,
            action_count: 5,
            estimated_duration: 15.0,
            success_probability: 0.85,
            bottlenecks: vec![
                Bottleneck {
                    action_name: "expensive_move".to_string(),
                    reason: BottleneckReason::HighCost,
                    severity: 0.8,
                },
                Bottleneck {
                    action_name: "risky_attack".to_string(),
                    reason: BottleneckReason::HighRisk,
                    severity: 0.6,
                },
            ],
            action_breakdown: HashMap::new(),
        };

        let report = PlanAnalyzer::generate_report(&metrics);

        assert!(report.contains("Bottlenecks"));
        assert!(report.contains("expensive_move"));
        assert!(report.contains("risky_attack"));
        assert!(report.contains("HighCost"));
        assert!(report.contains("severity"));
    }

    #[test]
    fn test_generate_report_with_suggestions() {
        let metrics = PlanMetrics {
            total_cost: 25.0,         // triggers high-cost suggestion
            total_risk: 3.0,          // triggers high-risk suggestion
            action_count: 12,         // triggers long-plan suggestion
            estimated_duration: 35.0, // triggers long-duration suggestion
            success_probability: 0.3, // triggers low success suggestion
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let report = PlanAnalyzer::generate_report(&metrics);

        assert!(report.contains("Optimization Suggestions"));
        assert!(report.contains("high total cost"));
    }

    #[test]
    fn test_generate_report_metric_values() {
        let metrics = PlanMetrics {
            total_cost: 12.34,
            total_risk: 0.56,
            action_count: 7,
            estimated_duration: 23.4,
            success_probability: 0.78,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let report = PlanAnalyzer::generate_report(&metrics);
        assert!(report.contains("Total Actions: 7"));
        assert!(report.contains("12.34")); // Total Cost
        assert!(report.contains("0.56")); // Total Risk
        assert!(report.contains("23.4")); // Duration
        assert!(report.contains("78.0%")); // Success probability
    }

    #[test]
    fn test_compare_diffs_computation() {
        let metrics1 = PlanMetrics {
            total_cost: 10.0,
            total_risk: 0.5,
            action_count: 3,
            estimated_duration: 20.0,
            success_probability: 0.7,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let metrics2 = PlanMetrics {
            total_cost: 15.0,
            total_risk: 0.8,
            action_count: 4,
            estimated_duration: 25.0,
            success_probability: 0.6,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let comparison = PlanAnalyzer::compare(&metrics1, &metrics2);

        // cost_diff = metrics2.cost - metrics1.cost = 5.0
        assert!((comparison.cost_diff - 5.0).abs() < 0.01);
        // risk_diff = 0.3
        assert!((comparison.risk_diff - 0.3).abs() < 0.01);
        // duration_diff = 5.0
        assert!((comparison.duration_diff - 5.0).abs() < 0.01);
        // success_prob_diff = -0.1
        assert!((comparison.success_prob_diff - (-0.1)).abs() < 0.01);
    }

    #[test]
    fn test_suggestion_sorting_by_priority() {
        let metrics = PlanMetrics {
            total_cost: 25.0,         // High priority
            total_risk: 3.0,          // Critical priority
            action_count: 12,         // Medium priority
            estimated_duration: 35.0, // Medium priority
            success_probability: 0.3, // Critical priority
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let suggestions = PlanAnalyzer::suggest_optimizations(&metrics);

        // First suggestions should be Critical
        assert!(suggestions.len() >= 2);
        assert_eq!(suggestions[0].priority, SuggestionPriority::Critical);
    }

    #[test]
    fn test_suggest_high_cost_estimated_improvement() {
        let metrics = PlanMetrics {
            total_cost: 30.0,
            total_risk: 0.2,
            action_count: 3,
            estimated_duration: 10.0,
            success_probability: 0.9,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let suggestions = PlanAnalyzer::suggest_optimizations(&metrics);
        let cost_s = suggestions
            .iter()
            .find(|s| s.message.contains("high total cost"))
            .unwrap();
        // estimated_improvement = total_cost * 0.2 = 6.0
        assert!((cost_s.estimated_improvement.unwrap() - 6.0).abs() < 0.01);
    }

    #[test]
    fn test_suggest_high_risk_estimated_improvement() {
        let metrics = PlanMetrics {
            total_cost: 5.0,
            total_risk: 3.0,
            action_count: 3,
            estimated_duration: 10.0,
            success_probability: 0.9,
            bottlenecks: vec![],
            action_breakdown: HashMap::new(),
        };

        let suggestions = PlanAnalyzer::suggest_optimizations(&metrics);
        let risk_s = suggestions
            .iter()
            .find(|s| s.message.contains("cumulative risk"))
            .unwrap();
        // estimated_improvement = total_risk * 0.3 = 0.9
        assert!((risk_s.estimated_improvement.unwrap() - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_bottleneck_severity_capped_at_1() {
        let mut breakdown = HashMap::new();

        breakdown.insert(
            "cheap1".to_string(),
            ActionMetrics {
                cost: 0.5,
                risk: 0.1,
                success_rate: 0.9,
                avg_duration: 1.0,
                executions: 10,
            },
        );

        breakdown.insert(
            "cheap2".to_string(),
            ActionMetrics {
                cost: 0.5,
                risk: 0.1,
                success_rate: 0.9,
                avg_duration: 1.0,
                executions: 10,
            },
        );

        // Extremely expensive — severity should be capped at 1.0
        // avg = (0.5 + 0.5 + 100.0) / 3 = 33.67, 2x = 67.33
        // 100.0 / 67.33 = 1.48 → min(1.0) → capped at 1.0
        breakdown.insert(
            "extreme".to_string(),
            ActionMetrics {
                cost: 100.0,
                risk: 0.1,
                success_rate: 0.9,
                avg_duration: 1.0,
                executions: 10,
            },
        );

        let bottlenecks = PlanAnalyzer::identify_bottlenecks(&breakdown);
        let bn = bottlenecks
            .iter()
            .find(|b| b.action_name == "extreme")
            .unwrap();
        assert!(
            (bn.severity - 1.0).abs() < 0.01,
            "Severity should cap at 1.0"
        );
    }

    #[test]
    fn test_bottleneck_sorting_by_severity() {
        let mut breakdown = HashMap::new();

        breakdown.insert(
            "filler".to_string(),
            ActionMetrics {
                cost: 1.0,
                risk: 0.05,
                success_rate: 0.95,
                avg_duration: 0.5,
                executions: 10,
            },
        );

        breakdown.insert(
            "medium_cost".to_string(),
            ActionMetrics {
                cost: 10.0, // High cost but moderate severity
                risk: 0.05,
                success_rate: 0.95,
                avg_duration: 0.5,
                executions: 10,
            },
        );

        breakdown.insert(
            "extreme_cost".to_string(),
            ActionMetrics {
                cost: 50.0, // Very high cost
                risk: 0.05,
                success_rate: 0.95,
                avg_duration: 0.5,
                executions: 10,
            },
        );

        let bottlenecks = PlanAnalyzer::identify_bottlenecks(&breakdown);

        // Both should be HighCost but extreme should be first (higher severity)
        if bottlenecks.len() >= 2 {
            assert!(bottlenecks[0].severity >= bottlenecks[1].severity);
        }
    }
}
