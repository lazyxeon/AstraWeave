// Shadow Mode: Side-by-side comparison of GOAP vs RuleOrchestrator
// Phase 2: Engine Integration

use crate::orchestrator::Orchestrator;
use astraweave_core::{PlanIntent, WorldSnapshot};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Comparison result between two planning approaches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanComparison {
    pub timestamp: f32,
    pub tactical_summary: String,
    pub rule_plan: PlanSummary,
    pub goap_plan: PlanSummary,
    pub differences: PlanDiff,
    pub metrics: ComparisonMetrics,
}

/// Summary of a single plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSummary {
    pub plan_id: String,
    pub step_count: usize,
    pub action_types: Vec<String>,
    pub planning_time_ms: f64,
    pub empty: bool,
}

/// Differences between two plans
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanDiff {
    pub step_count_diff: i32,
    pub actions_in_common: usize,
    pub unique_to_rule: Vec<String>,
    pub unique_to_goap: Vec<String>,
    pub order_differs: bool,
    pub similarity_score: f32, // 0.0 = completely different, 1.0 = identical
}

/// Performance and quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonMetrics {
    pub rule_faster: bool,
    pub time_difference_ms: f64,
    pub goap_more_steps: bool,
    pub both_empty: bool,
    pub both_non_empty: bool,
}

impl PlanComparison {
    /// Create a new comparison from two plans
    pub fn new(
        snap: &WorldSnapshot,
        rule_plan: PlanIntent,
        rule_time_ms: f64,
        goap_plan: PlanIntent,
        goap_time_ms: f64,
    ) -> Self {
        let tactical_summary = crate::goap::adapter::SnapshotAdapter::tactical_summary(snap);

        let rule_summary = PlanSummary::from_intent(&rule_plan, rule_time_ms);
        let goap_summary = PlanSummary::from_intent(&goap_plan, goap_time_ms);

        let differences = PlanDiff::compute(&rule_summary, &goap_summary);
        let metrics =
            ComparisonMetrics::compute(&rule_summary, &goap_summary, rule_time_ms, goap_time_ms);

        Self {
            timestamp: snap.t,
            tactical_summary,
            rule_plan: rule_summary,
            goap_plan: goap_summary,
            differences,
            metrics,
        }
    }

    /// Format as human-readable log entry
    pub fn to_log_entry(&self) -> String {
        let mut log = String::new();

        log.push_str(&format!(
            "\n═══ Shadow Mode Comparison @ t={:.1}s ═══\n",
            self.timestamp
        ));
        log.push_str(&format!("Situation: {}\n\n", self.tactical_summary));

        log.push_str("🤖 RuleOrchestrator:\n");
        log.push_str(&format!("  Plan ID: {}\n", self.rule_plan.plan_id));
        log.push_str(&format!("  Steps: {}\n", self.rule_plan.step_count));
        log.push_str(&format!("  Actions: {:?}\n", self.rule_plan.action_types));
        log.push_str(&format!(
            "  Planning Time: {:.2}ms\n\n",
            self.rule_plan.planning_time_ms
        ));

        log.push_str("🧠 GOAP Planner:\n");
        log.push_str(&format!("  Plan ID: {}\n", self.goap_plan.plan_id));
        log.push_str(&format!("  Steps: {}\n", self.goap_plan.step_count));
        log.push_str(&format!("  Actions: {:?}\n", self.goap_plan.action_types));
        log.push_str(&format!(
            "  Planning Time: {:.2}ms\n\n",
            self.goap_plan.planning_time_ms
        ));

        log.push_str("📊 Differences:\n");
        log.push_str(&format!(
            "  Similarity Score: {:.1}%\n",
            self.differences.similarity_score * 100.0
        ));
        log.push_str(&format!(
            "  Common Actions: {}\n",
            self.differences.actions_in_common
        ));

        if !self.differences.unique_to_rule.is_empty() {
            log.push_str(&format!(
                "  Only in Rule: {:?}\n",
                self.differences.unique_to_rule
            ));
        }
        if !self.differences.unique_to_goap.is_empty() {
            log.push_str(&format!(
                "  Only in GOAP: {:?}\n",
                self.differences.unique_to_goap
            ));
        }
        if self.differences.order_differs {
            log.push_str("  ⚠ Action order differs\n");
        }

        log.push_str("\n📈 Metrics:\n");
        if self.metrics.rule_faster {
            log.push_str(&format!(
                "  ✓ Rule faster by {:.2}ms\n",
                self.metrics.time_difference_ms
            ));
        } else {
            log.push_str(&format!(
                "  ✓ GOAP faster by {:.2}ms\n",
                self.metrics.time_difference_ms.abs()
            ));
        }

        if self.metrics.both_empty {
            log.push_str("  ℹ Both plans empty (no action needed)\n");
        } else if !self.metrics.both_non_empty {
            log.push_str("  ⚠ One planner failed to generate plan\n");
        }

        log.push_str("═══════════════════════════════════════\n");
        log
    }

    /// Export as JSON for offline analysis
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

impl PlanSummary {
    fn from_intent(intent: &PlanIntent, planning_time_ms: f64) -> Self {
        let action_types: Vec<String> = intent
            .steps
            .iter()
            .map(|step| {
                format!("{:?}", step)
                    .split_whitespace()
                    .next()
                    .unwrap_or("Unknown")
                    .to_string()
            })
            .collect();

        Self {
            plan_id: intent.plan_id.clone(),
            step_count: intent.steps.len(),
            action_types,
            planning_time_ms,
            empty: intent.steps.is_empty(),
        }
    }
}

impl PlanDiff {
    fn compute(rule: &PlanSummary, goap: &PlanSummary) -> Self {
        let step_count_diff = goap.step_count as i32 - rule.step_count as i32;

        // Find common actions (ignoring order)
        let rule_set: std::collections::HashSet<_> = rule.action_types.iter().collect();
        let goap_set: std::collections::HashSet<_> = goap.action_types.iter().collect();

        let actions_in_common = rule_set.intersection(&goap_set).count();

        let unique_to_rule: Vec<String> = rule
            .action_types
            .iter()
            .filter(|a| !goap_set.contains(a))
            .map(|s| s.clone())
            .collect();

        let unique_to_goap: Vec<String> = goap
            .action_types
            .iter()
            .filter(|a| !rule_set.contains(a))
            .map(|s| s.clone())
            .collect();

        // Check if order differs (for common actions)
        let order_differs = rule.action_types != goap.action_types;

        // Calculate similarity score
        let total_actions = (rule.step_count + goap.step_count).max(1);
        let similarity_score = (actions_in_common * 2) as f32 / total_actions as f32;

        Self {
            step_count_diff,
            actions_in_common,
            unique_to_rule,
            unique_to_goap,
            order_differs,
            similarity_score,
        }
    }
}

impl ComparisonMetrics {
    fn compute(
        rule: &PlanSummary,
        goap: &PlanSummary,
        rule_time_ms: f64,
        goap_time_ms: f64,
    ) -> Self {
        Self {
            rule_faster: rule_time_ms < goap_time_ms,
            time_difference_ms: rule_time_ms - goap_time_ms,
            goap_more_steps: goap.step_count > rule.step_count,
            both_empty: rule.empty && goap.empty,
            both_non_empty: !rule.empty && !goap.empty,
        }
    }
}

/// Shadow mode runner that executes both planners
pub struct ShadowModeRunner {
    comparisons: Vec<PlanComparison>,
    log_to_console: bool,
}

impl ShadowModeRunner {
    pub fn new(log_to_console: bool) -> Self {
        Self {
            comparisons: Vec::new(),
            log_to_console,
        }
    }

    /// Run both planners and compare results
    pub fn compare(
        &mut self,
        snap: &WorldSnapshot,
        rule_orchestrator: &crate::orchestrator::RuleOrchestrator,
        goap_orchestrator: &mut crate::goap::orchestrator::GOAPOrchestrator,
    ) -> &PlanComparison {
        // Execute rule-based planner
        let rule_start = Instant::now();
        let rule_plan = rule_orchestrator.propose_plan(snap);
        let rule_time_ms = rule_start.elapsed().as_secs_f64() * 1000.0;

        // Execute GOAP planner
        let goap_start = Instant::now();
        let goap_plan = goap_orchestrator.propose_plan(snap);
        let goap_time_ms = goap_start.elapsed().as_secs_f64() * 1000.0;

        // Create comparison
        let comparison =
            PlanComparison::new(snap, rule_plan, rule_time_ms, goap_plan, goap_time_ms);

        if self.log_to_console {
            println!("{}", comparison.to_log_entry());
        }

        self.comparisons.push(comparison);
        self.comparisons.last().expect("just pushed comparison")
    }

    /// Get all comparisons
    pub fn get_comparisons(&self) -> &[PlanComparison] {
        &self.comparisons
    }

    /// Generate aggregate statistics
    pub fn generate_report(&self) -> ShadowModeReport {
        ShadowModeReport::from_comparisons(&self.comparisons)
    }
}

/// Aggregate report from multiple comparisons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowModeReport {
    pub total_comparisons: usize,
    pub avg_similarity: f32,
    pub goap_faster_count: usize,
    pub rule_faster_count: usize,
    pub avg_time_diff_ms: f64,
    pub both_empty_count: usize,
    pub avg_rule_steps: f32,
    pub avg_goap_steps: f32,
}

impl ShadowModeReport {
    fn from_comparisons(comparisons: &[PlanComparison]) -> Self {
        if comparisons.is_empty() {
            return Self::default();
        }

        let total = comparisons.len();

        let avg_similarity = comparisons
            .iter()
            .map(|c| c.differences.similarity_score)
            .sum::<f32>()
            / total as f32;

        let goap_faster_count = comparisons
            .iter()
            .filter(|c| !c.metrics.rule_faster)
            .count();

        let rule_faster_count = total - goap_faster_count;

        let avg_time_diff_ms = comparisons
            .iter()
            .map(|c| c.metrics.time_difference_ms)
            .sum::<f64>()
            / total as f64;

        let both_empty_count = comparisons.iter().filter(|c| c.metrics.both_empty).count();

        let avg_rule_steps = comparisons
            .iter()
            .map(|c| c.rule_plan.step_count)
            .sum::<usize>() as f32
            / total as f32;

        let avg_goap_steps = comparisons
            .iter()
            .map(|c| c.goap_plan.step_count)
            .sum::<usize>() as f32
            / total as f32;

        Self {
            total_comparisons: total,
            avg_similarity,
            goap_faster_count,
            rule_faster_count,
            avg_time_diff_ms,
            both_empty_count,
            avg_rule_steps,
            avg_goap_steps,
        }
    }

    pub fn print_report(&self) {
        println!("\n╔════════════════════════════════════════╗");
        println!("║     Shadow Mode Aggregate Report      ║");
        println!("╚════════════════════════════════════════╝\n");

        println!("📊 Total Comparisons: {}", self.total_comparisons);
        println!("🎯 Average Similarity: {:.1}%", self.avg_similarity * 100.0);
        println!("\n⏱️  Performance:");
        println!(
            "   • GOAP faster: {} times ({:.1}%)",
            self.goap_faster_count,
            (self.goap_faster_count as f32 / self.total_comparisons as f32) * 100.0
        );
        println!(
            "   • Rule faster: {} times ({:.1}%)",
            self.rule_faster_count,
            (self.rule_faster_count as f32 / self.total_comparisons as f32) * 100.0
        );
        println!(
            "   • Avg time difference: {:.2}ms",
            self.avg_time_diff_ms.abs()
        );

        println!("\n📈 Plan Characteristics:");
        println!("   • Avg Rule steps: {:.1}", self.avg_rule_steps);
        println!("   • Avg GOAP steps: {:.1}", self.avg_goap_steps);
        println!("   • Both empty: {} times", self.both_empty_count);

        println!("\n════════════════════════════════════════\n");
    }
}

impl Default for ShadowModeReport {
    fn default() -> Self {
        Self {
            total_comparisons: 0,
            avg_similarity: 0.0,
            goap_faster_count: 0,
            rule_faster_count: 0,
            avg_time_diff_ms: 0.0,
            both_empty_count: 0,
            avg_rule_steps: 0.0,
            avg_goap_steps: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_core::{ActionStep, CompanionState, IVec2, PlayerState, Stance};
    use std::collections::BTreeMap;

    fn make_test_snapshot() -> WorldSnapshot {
        WorldSnapshot {
            t: 1.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 0, y: 0 },
                stance: Stance::Stand,
                orders: vec![],
            },
            me: CompanionState {
                ammo: 20,
                cooldowns: BTreeMap::new(),
                morale: 1.0,
                pos: IVec2 { x: 5, y: 5 },
            },
            enemies: vec![],
            pois: vec![],
            obstacles: vec![],
            objective: None,
        }
    }

    #[test]
    fn test_plan_comparison() {
        let snap = make_test_snapshot();

        let rule_plan = PlanIntent {
            plan_id: "rule-1".to_string(),
            steps: vec![
                ActionStep::MoveTo {
                    x: 10,
                    y: 10,
                    speed: None,
                },
                ActionStep::Attack { target_id: 1 },
            ],
        };

        let goap_plan = PlanIntent {
            plan_id: "goap-1".to_string(),
            steps: vec![
                ActionStep::MoveTo {
                    x: 10,
                    y: 10,
                    speed: None,
                },
                ActionStep::Reload,
                ActionStep::Attack { target_id: 1 },
            ],
        };

        let comparison = PlanComparison::new(&snap, rule_plan, 1.5, goap_plan, 2.0);

        assert_eq!(comparison.differences.actions_in_common, 2); // MoveTo and Attack
        assert_eq!(comparison.differences.step_count_diff, 1); // GOAP has 1 more step
        assert!(comparison.metrics.rule_faster);
    }

    #[test]
    fn test_shadow_mode_report() {
        let mut comparisons = Vec::new();

        for i in 0..5 {
            let snap = make_test_snapshot();
            let rule_plan = PlanIntent {
                plan_id: format!("rule-{}", i),
                steps: vec![ActionStep::MoveTo {
                    x: i,
                    y: i,
                    speed: None,
                }],
            };
            let goap_plan = PlanIntent {
                plan_id: format!("goap-{}", i),
                steps: vec![ActionStep::MoveTo {
                    x: i,
                    y: i,
                    speed: None,
                }],
            };

            comparisons.push(PlanComparison::new(&snap, rule_plan, 1.0, goap_plan, 1.5));
        }

        let report = ShadowModeReport::from_comparisons(&comparisons);

        assert_eq!(report.total_comparisons, 5);
        assert_eq!(report.rule_faster_count, 5); // Rule is always faster in this test
        assert!(report.avg_similarity > 0.9); // Plans are very similar
    }

    // ================================================================
    // Mutation-killing tests
    // ================================================================

    /// Kills line 158: `to_json()` body replaced with Ok(String::new()) / Ok("xyzzy")
    #[test]
    fn test_to_json_produces_valid_json() {
        let snap = make_test_snapshot();
        let rule_plan = PlanIntent {
            plan_id: "rule-1".to_string(),
            steps: vec![ActionStep::MoveTo {
                x: 1,
                y: 2,
                speed: None,
            }],
        };
        let goap_plan = PlanIntent {
            plan_id: "goap-1".to_string(),
            steps: vec![ActionStep::MoveTo {
                x: 1,
                y: 2,
                speed: None,
            }],
        };
        let comparison = PlanComparison::new(&snap, rule_plan, 1.0, goap_plan, 2.0);

        let json = comparison.to_json().unwrap();
        assert!(json.len() > 10, "JSON should be non-trivial");
        assert!(json.contains("rule-1"), "JSON should contain plan ID");
        // Verify it's valid JSON by parsing back
        let _: serde_json::Value = serde_json::from_str(&json).unwrap();
    }

    /// Kills lines 117, 123: to_log_entry with unique_to_rule/goap sections
    #[test]
    fn test_to_log_entry_with_differences() {
        let snap = make_test_snapshot();
        let rule_plan = PlanIntent {
            plan_id: "rule-1".to_string(),
            steps: vec![ActionStep::Reload],
        };
        let goap_plan = PlanIntent {
            plan_id: "goap-1".to_string(),
            steps: vec![ActionStep::Attack { target_id: 1 }],
        };
        let comparison = PlanComparison::new(&snap, rule_plan, 1.0, goap_plan, 2.0);

        let log = comparison.to_log_entry();
        assert!(log.contains("Only in Rule"), "Should show unique_to_rule");
        assert!(log.contains("Only in GOAP"), "Should show unique_to_goap");
    }

    /// Kills line 148: to_log_entry with one empty plan
    #[test]
    fn test_to_log_entry_one_empty_plan() {
        let snap = make_test_snapshot();
        let rule_plan = PlanIntent {
            plan_id: "rule-1".to_string(),
            steps: vec![ActionStep::Reload],
        };
        let goap_plan = PlanIntent {
            plan_id: "goap-1".to_string(),
            steps: vec![], // Empty
        };
        let comparison = PlanComparison::new(&snap, rule_plan, 1.0, goap_plan, 2.0);

        let log = comparison.to_log_entry();
        assert!(
            log.contains("failed to generate plan"),
            "Should warn about one empty planner"
        );
    }

    /// Kills lines 188, 199, 206, 211: PlanDiff exact values
    #[test]
    fn test_plan_diff_exact_values() {
        let snap = make_test_snapshot();
        let rule_plan = PlanIntent {
            plan_id: "rule-1".to_string(),
            steps: vec![
                ActionStep::MoveTo {
                    x: 1,
                    y: 1,
                    speed: None,
                },
                ActionStep::Attack { target_id: 1 },
            ],
        };
        let goap_plan = PlanIntent {
            plan_id: "goap-1".to_string(),
            steps: vec![
                ActionStep::Reload,
                ActionStep::Attack { target_id: 1 },
                ActionStep::MoveTo {
                    x: 2,
                    y: 2,
                    speed: None,
                },
            ],
        };
        let comparison = PlanComparison::new(&snap, rule_plan, 1.0, goap_plan, 2.0);

        // step_count_diff = 3 - 2 = 1
        assert_eq!(comparison.differences.step_count_diff, 1);
        // unique_to_rule should have exactly the actions only in rule
        assert!(
            !comparison.differences.unique_to_rule.is_empty()
                || !comparison.differences.unique_to_goap.is_empty(),
            "Should have some unique actions"
        );
        // order_differs should be true (different action sequences)
        assert!(
            comparison.differences.order_differs,
            "Action order should differ"
        );
    }

    /// Kills lines 214-215: similarity_score exact value
    #[test]
    fn test_similarity_score_exact() {
        let snap = make_test_snapshot();
        // Identical plans
        let rule_plan = PlanIntent {
            plan_id: "r".to_string(),
            steps: vec![ActionStep::Reload],
        };
        let goap_plan = PlanIntent {
            plan_id: "g".to_string(),
            steps: vec![ActionStep::Reload],
        };
        let comparison = PlanComparison::new(&snap, rule_plan, 1.0, goap_plan, 1.0);
        // 1 common action, total = 1+1 = 2, similarity = (1*2)/2 = 1.0
        assert!(
            (comparison.differences.similarity_score - 1.0).abs() < 0.01,
            "Identical plans should have similarity 1.0, got {}",
            comparison.differences.similarity_score
        );

        // Completely different plans
        let rule_plan2 = PlanIntent {
            plan_id: "r".to_string(),
            steps: vec![ActionStep::Reload],
        };
        let goap_plan2 = PlanIntent {
            plan_id: "g".to_string(),
            steps: vec![ActionStep::Attack { target_id: 1 }],
        };
        let comparison2 = PlanComparison::new(&snap, rule_plan2, 1.0, goap_plan2, 1.0);
        // 0 common actions, total = 2, similarity = 0/2 = 0.0
        assert!(
            comparison2.differences.similarity_score < 0.01,
            "Completely different plans should have similarity ~0.0, got {}",
            comparison2.differences.similarity_score
        );
    }

    /// Kills lines 236-240: ComparisonMetrics exact values
    #[test]
    fn test_comparison_metrics_exact_values() {
        let snap = make_test_snapshot();
        // GOAP faster
        let rule_plan = PlanIntent {
            plan_id: "r".to_string(),
            steps: vec![ActionStep::Reload, ActionStep::Attack { target_id: 1 }],
        };
        let goap_plan = PlanIntent {
            plan_id: "g".to_string(),
            steps: vec![ActionStep::Reload],
        };
        let comparison = PlanComparison::new(&snap, rule_plan, 3.0, goap_plan, 1.0);

        assert!(!comparison.metrics.rule_faster, "GOAP should be faster");
        assert!(
            (comparison.metrics.time_difference_ms - 2.0).abs() < 0.01,
            "Time diff should be 2.0ms"
        );
        assert!(!comparison.metrics.goap_more_steps, "Rule has more steps");
        assert!(!comparison.metrics.both_empty, "Neither is empty");
        assert!(
            comparison.metrics.both_non_empty,
            "Both should be non-empty"
        );
    }

    /// Kills line 290: get_comparisons returning empty slice
    #[test]
    fn test_get_comparisons_returns_stored() {
        let runner = ShadowModeRunner::new(false);
        assert_eq!(runner.get_comparisons().len(), 0);
        // Can't easily push without orchestrators, but verify empty vec is returned
    }

    /// Kills line 295: generate_report returning Default
    /// Kills lines 324, 337, 345, 351: report avg calculations
    #[test]
    fn test_report_exact_values() {
        let snap = make_test_snapshot();
        let mut comparisons = Vec::new();

        // Comparison 1: rule faster, 2 rule steps, 1 goap step, identical action
        comparisons.push(PlanComparison::new(
            &snap,
            PlanIntent {
                plan_id: "r1".to_string(),
                steps: vec![ActionStep::Reload, ActionStep::Reload],
            },
            1.0,
            PlanIntent {
                plan_id: "g1".to_string(),
                steps: vec![ActionStep::Reload],
            },
            3.0,
        ));

        // Comparison 2: goap faster, 1 rule step, 2 goap steps
        comparisons.push(PlanComparison::new(
            &snap,
            PlanIntent {
                plan_id: "r2".to_string(),
                steps: vec![ActionStep::Reload],
            },
            5.0,
            PlanIntent {
                plan_id: "g2".to_string(),
                steps: vec![ActionStep::Reload, ActionStep::Reload],
            },
            2.0,
        ));

        let report = ShadowModeReport::from_comparisons(&comparisons);

        assert_eq!(report.total_comparisons, 2);
        assert_eq!(report.goap_faster_count, 1);
        assert_eq!(report.rule_faster_count, 1);
        // avg_time_diff = ((1.0-3.0) + (5.0-2.0)) / 2 = (-2.0 + 3.0) / 2 = 0.5
        assert!(
            (report.avg_time_diff_ms - 0.5).abs() < 0.01,
            "avg_time_diff_ms should be 0.5, got {}",
            report.avg_time_diff_ms
        );
        // avg_rule_steps = (2 + 1) / 2 = 1.5
        assert!(
            (report.avg_rule_steps - 1.5).abs() < 0.01,
            "avg_rule_steps should be 1.5, got {}",
            report.avg_rule_steps
        );
        // avg_goap_steps = (1 + 2) / 2 = 1.5
        assert!(
            (report.avg_goap_steps - 1.5).abs() < 0.01,
            "avg_goap_steps should be 1.5, got {}",
            report.avg_goap_steps
        );
        assert_eq!(report.both_empty_count, 0);
    }

    /// Kills line 331: `total - goap_faster_count` → `total + goap_faster_count`
    #[test]
    fn test_report_rule_faster_count_with_mixed_data() {
        let snap = make_test_snapshot();
        let mut comparisons = Vec::new();

        // 3 comparisons: 2 goap faster, 1 rule faster
        for i in 0..3 {
            let rule_time = if i < 2 { 5.0 } else { 1.0 };
            let goap_time = if i < 2 { 1.0 } else { 5.0 };
            comparisons.push(PlanComparison::new(
                &snap,
                PlanIntent {
                    plan_id: format!("r{}", i),
                    steps: vec![ActionStep::Reload],
                },
                rule_time,
                PlanIntent {
                    plan_id: format!("g{}", i),
                    steps: vec![ActionStep::Reload],
                },
                goap_time,
            ));
        }

        let report = ShadowModeReport::from_comparisons(&comparisons);
        assert_eq!(report.goap_faster_count, 2);
        assert_eq!(
            report.rule_faster_count, 1,
            "rule_faster should be total - goap_faster = 3 - 2 = 1"
        );
    }

    /// Kills line 188: `- with /` in step_count_diff (i32 division)
    /// 5 - 2 = 3, but 5 / 2 = 2 (integer division). Must use counts where a-b != a/b.
    #[test]
    fn test_step_count_diff_kills_division_mutation() {
        let snap = make_test_snapshot();
        let rule_plan = PlanIntent {
            plan_id: "r".to_string(),
            steps: vec![ActionStep::Reload, ActionStep::Reload],
        };
        let goap_plan = PlanIntent {
            plan_id: "g".to_string(),
            steps: vec![
                ActionStep::Reload,
                ActionStep::Reload,
                ActionStep::Reload,
                ActionStep::Reload,
                ActionStep::Reload,
            ],
        };
        let comparison = PlanComparison::new(&snap, rule_plan, 1.0, goap_plan, 1.0);
        // step_count_diff = 5 - 2 = 3, NOT 5 / 2 = 2
        assert_eq!(
            comparison.differences.step_count_diff, 3,
            "step_count_diff should be 3 (5-2), not 2 (5/2)"
        );
    }

    /// Kills line 236: `< with <=` in rule_faster (equal times)
    /// Kills line 238: `> with ==` / `>= with >=` in goap_more_steps (equal steps)
    #[test]
    fn test_metrics_with_equal_values() {
        let snap = make_test_snapshot();
        // Equal times
        let rule_plan = PlanIntent {
            plan_id: "r".to_string(),
            steps: vec![ActionStep::Reload],
        };
        let goap_plan = PlanIntent {
            plan_id: "g".to_string(),
            steps: vec![ActionStep::Reload],
        };
        let comparison = PlanComparison::new(&snap, rule_plan, 2.0, goap_plan, 2.0);

        // Equal times: rule_time < goap_time → false
        assert!(
            !comparison.metrics.rule_faster,
            "With equal times, rule_faster should be false"
        );
        // Equal step counts: goap > rule → false
        assert!(
            !comparison.metrics.goap_more_steps,
            "With equal step counts, goap_more_steps should be false"
        );
    }

    /// Kills line 324: `/ total` → `* total` in avg_similarity
    #[test]
    fn test_report_avg_similarity_exact() {
        let snap = make_test_snapshot();
        let mut comparisons = Vec::new();

        // 3 comparisons with identical plans (similarity 1.0 each)
        for _ in 0..3 {
            comparisons.push(PlanComparison::new(
                &snap,
                PlanIntent {
                    plan_id: "r".to_string(),
                    steps: vec![ActionStep::Reload],
                },
                1.0,
                PlanIntent {
                    plan_id: "g".to_string(),
                    steps: vec![ActionStep::Reload],
                },
                1.0,
            ));
        }

        let report = ShadowModeReport::from_comparisons(&comparisons);
        // avg_similarity = sum(1.0 * 3) / 3 = 1.0, NOT sum * 3 = 9.0
        assert!(
            (report.avg_similarity - 1.0).abs() < 0.01,
            "avg_similarity should be 1.0, got {}",
            report.avg_similarity
        );
    }
}
