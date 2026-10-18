use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Statistics for a specific action's execution history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStats {
    pub executions: u32,
    pub successes: u32,
    pub failures: u32,
    pub avg_duration: f32,
}

impl ActionStats {
    pub fn new() -> Self {
        Self {
            executions: 0,
            successes: 0,
            failures: 0,
            avg_duration: 0.0,
        }
    }

    /// Success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f32 {
        if self.executions == 0 {
            0.5 // Unknown performance, assume neutral
        } else {
            self.successes as f32 / self.executions as f32
        }
    }

    /// Failure rate (0.0 to 1.0)
    pub fn failure_rate(&self) -> f32 {
        1.0 - self.success_rate()
    }

    /// Reliability score (higher is more reliable)
    /// Factors in both success rate and sample size
    pub fn reliability_score(&self) -> f32 {
        let confidence_factor = (self.executions as f32 / 20.0).min(1.0);
        self.success_rate() * confidence_factor
    }
}

impl Default for ActionStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Action execution history for learning and adaptation
/// Tracks success/failure rates and execution times
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionHistory {
    stats: HashMap<String, ActionStats>,
}

impl ActionHistory {
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }

    /// Record a successful action execution
    pub fn record_success(&mut self, action_name: &str, duration: f32) {
        let stats = self.stats.entry(action_name.to_string()).or_default();

        stats.executions += 1;
        stats.successes += 1;

        // Update rolling average duration
        if stats.executions == 1 {
            stats.avg_duration = duration;
        } else {
            let prev_total = stats.avg_duration * (stats.executions - 1) as f32;
            stats.avg_duration = (prev_total + duration) / stats.executions as f32;
        }
    }

    /// Record a failed action execution
    pub fn record_failure(&mut self, action_name: &str) {
        let stats = self.stats.entry(action_name.to_string()).or_default();

        stats.executions += 1;
        stats.failures += 1;
    }

    /// Get statistics for a specific action
    pub fn get_action_stats(&self, action_name: &str) -> Option<&ActionStats> {
        self.stats.get(action_name)
    }

    /// Get mutable statistics for a specific action
    pub fn get_action_stats_mut(&mut self, action_name: &str) -> Option<&mut ActionStats> {
        self.stats.get_mut(action_name)
    }

    /// Get all action names with recorded history
    pub fn action_names(&self) -> Vec<String> {
        self.stats.keys().cloned().collect()
    }

    /// Clear all history (for testing or resets)
    pub fn clear(&mut self) {
        self.stats.clear();
    }

    /// Total number of actions tracked
    pub fn len(&self) -> usize {
        self.stats.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stats.is_empty()
    }

    /// Reset stats for an action (useful for testing or manual intervention)
    pub fn reset_action(&mut self, action_name: &str) {
        self.stats.remove(action_name);
    }

    /// Get total number of executions across all actions
    pub fn total_executions(&self) -> u32 {
        self.stats.values().map(|s| s.executions).sum()
    }

    /// Prune actions with very low execution counts (noise reduction)
    pub fn prune_noise(&mut self, min_executions: u32) {
        self.stats
            .retain(|_, stats| stats.executions >= min_executions);
    }

    /// Merge another history into this one
    /// Useful for sharing learning across entities
    pub fn merge(&mut self, other: &ActionHistory) {
        for (action_name, other_stats) in &other.stats {
            let stats = self.stats.entry(action_name.clone()).or_default();

            // Merge statistics
            let total_exec = stats.executions + other_stats.executions;
            if total_exec > 0 {
                stats.avg_duration = (stats.avg_duration * stats.executions as f32
                    + other_stats.avg_duration * other_stats.executions as f32)
                    / total_exec as f32;
            }

            stats.executions += other_stats.executions;
            stats.successes += other_stats.successes;
            stats.failures += other_stats.failures;
        }
    }

    /// Prune old or unreliable data
    /// Keeps only the N most executed actions
    pub fn prune(&mut self, keep_top_n: usize) {
        if self.stats.len() <= keep_top_n {
            return;
        }

        let mut actions: Vec<_> = self.stats.iter().collect();
        actions.sort_by(|a, b| b.1.executions.cmp(&a.1.executions));

        let keep_names: Vec<_> = actions
            .iter()
            .take(keep_top_n)
            .map(|(k, _)| k.to_string())
            .collect();
        self.stats.retain(|k, _| keep_names.contains(k));
    }
}

impl Default for ActionHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_stats_success_rate() {
        let mut stats = ActionStats::new();
        assert_eq!(stats.success_rate(), 0.5); // Default for unknown

        stats.executions = 10;
        stats.successes = 7;
        stats.failures = 3;

        assert_eq!(stats.success_rate(), 0.7);
        assert_eq!(stats.failure_rate(), 0.3);
    }

    #[test]
    fn test_record_success() {
        let mut history = ActionHistory::new();

        history.record_success("attack", 1.5);
        history.record_success("attack", 2.5);

        let stats = history.get_action_stats("attack").unwrap();
        assert_eq!(stats.executions, 2);
        assert_eq!(stats.successes, 2);
        assert_eq!(stats.failures, 0);
        assert_eq!(stats.avg_duration, 2.0);
    }

    #[test]
    fn test_record_failure() {
        let mut history = ActionHistory::new();

        history.record_success("move", 1.0);
        history.record_failure("move");
        history.record_failure("move");

        let stats = history.get_action_stats("move").unwrap();
        assert_eq!(stats.executions, 3);
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.failures, 2);
        assert!((stats.success_rate() - 0.333333).abs() < 0.01);
    }

    #[test]
    fn test_rolling_average_duration() {
        let mut history = ActionHistory::new();

        history.record_success("heal", 1.0);
        history.record_success("heal", 2.0);
        history.record_success("heal", 3.0);

        let stats = history.get_action_stats("heal").unwrap();
        assert_eq!(stats.avg_duration, 2.0);
    }

    #[test]
    fn test_merge_histories() {
        let mut history1 = ActionHistory::new();
        history1.record_success("attack", 1.0);
        history1.record_success("attack", 2.0);

        let mut history2 = ActionHistory::new();
        history2.record_success("attack", 3.0);
        history2.record_failure("attack");

        history1.merge(&history2);

        let stats = history1.get_action_stats("attack").unwrap();
        assert_eq!(stats.executions, 4);
        assert_eq!(stats.successes, 3);
        assert_eq!(stats.failures, 1);
        assert_eq!(stats.success_rate(), 0.75);
    }

    #[test]
    fn test_prune() {
        let mut history = ActionHistory::new();

        history.record_success("action1", 1.0);
        history.record_success("action1", 1.0);
        history.record_success("action1", 1.0);

        history.record_success("action2", 1.0);
        history.record_success("action2", 1.0);

        history.record_success("action3", 1.0);

        assert_eq!(history.len(), 3);

        history.prune(2);

        assert_eq!(history.len(), 2);
        assert!(history.get_action_stats("action1").is_some());
        assert!(history.get_action_stats("action2").is_some());
        assert!(history.get_action_stats("action3").is_none());
    }

    #[test]
    fn test_reliability_score() {
        let mut stats = ActionStats::new();

        // Low confidence (few executions)
        stats.executions = 2;
        stats.successes = 2;
        assert!(stats.reliability_score() < 0.2);

        // High confidence (many executions)
        stats.executions = 20;
        stats.successes = 18;
        assert!(stats.reliability_score() > 0.85);
    }

    // ── Mutation-killing tests ──

    #[test]
    fn test_reliability_score_exact_arithmetic() {
        // Kills: /20.0 → /1.0, .min(1.0) → .max(1.0), etc.
        let mut stats = ActionStats::new();

        // exec=10, succ=10 → confidence=(10/20).min(1.0)=0.5, sr=1.0, rel=0.5
        stats.executions = 10;
        stats.successes = 10;
        assert_eq!(stats.reliability_score(), 0.5);

        // exec=20, succ=20 → confidence=(20/20).min(1.0)=1.0, sr=1.0, rel=1.0
        stats.executions = 20;
        stats.successes = 20;
        assert_eq!(stats.reliability_score(), 1.0);

        // exec=40, succ=20 → confidence=(40/20).min(1.0)=1.0, sr=0.5, rel=0.5
        stats.executions = 40;
        stats.successes = 20;
        assert_eq!(stats.reliability_score(), 0.5);

        // exec=0 → sr=0.5, confidence=0.0, rel=0.0
        stats.executions = 0;
        stats.successes = 0;
        assert_eq!(stats.reliability_score(), 0.0);
    }

    #[test]
    fn test_success_rate_default_neutral() {
        // Kills: default 0.5 → 0.0 or 1.0
        let stats = ActionStats::new();
        assert_eq!(stats.success_rate(), 0.5);
    }

    #[test]
    fn test_failure_rate_complement() {
        // Kills: 1.0 - success_rate → success_rate or 0.0 - success_rate
        let mut stats = ActionStats::new();
        stats.executions = 10;
        stats.successes = 7;
        stats.failures = 3;
        assert_eq!(stats.failure_rate(), 0.3);
        // Also verify sr + fr = 1.0
        assert_eq!(stats.success_rate() + stats.failure_rate(), 1.0);
    }

    #[test]
    fn test_rolling_average_first_execution_path() {
        // Kills: ==1 → ==0 branch check, and first-exec direct assignment
        let mut history = ActionHistory::new();
        history.record_success("a", 5.0);

        let stats = history.get_action_stats("a").unwrap();
        assert_eq!(stats.executions, 1);
        assert_eq!(stats.avg_duration, 5.0); // First exec: direct assignment

        // Second execution: rolling average
        history.record_success("a", 3.0);
        let stats = history.get_action_stats("a").unwrap();
        assert_eq!(stats.executions, 2);
        // Rolling avg = (5.0 * 1 + 3.0) / 2 = 4.0
        assert_eq!(stats.avg_duration, 4.0);
    }

    #[test]
    fn test_rolling_average_arithmetic() {
        // Kills: (exec-1) → (exec+1), division errors
        let mut history = ActionHistory::new();
        history.record_success("x", 10.0); // avg=10
        history.record_success("x", 20.0); // avg = (10*1 + 20)/2 = 15
        history.record_success("x", 30.0); // avg = (15*2 + 30)/3 = 20

        let stats = history.get_action_stats("x").unwrap();
        assert_eq!(stats.avg_duration, 20.0);
    }

    #[test]
    fn test_total_executions() {
        let mut history = ActionHistory::new();
        history.record_success("a", 1.0);
        history.record_success("a", 1.0);
        history.record_success("a", 1.0);
        history.record_success("b", 1.0);
        history.record_failure("b");

        assert_eq!(history.total_executions(), 5);
    }

    #[test]
    fn test_prune_noise_threshold() {
        // Kills: >= → >
        let mut history = ActionHistory::new();
        // "a" has 3 executions, "b" has 2
        history.record_success("a", 1.0);
        history.record_success("a", 1.0);
        history.record_success("a", 1.0);
        history.record_success("b", 1.0);
        history.record_success("b", 1.0);

        // prune_noise(3) → retain where exec >= 3 → only "a"
        history.prune_noise(3);
        assert_eq!(history.len(), 1);
        assert!(history.get_action_stats("a").is_some());
        assert!(history.get_action_stats("b").is_none());

        // prune_noise(2) on a fresh set → both retained
        let mut h2 = ActionHistory::new();
        h2.record_success("x", 1.0);
        h2.record_success("x", 1.0);
        h2.record_success("y", 1.0);
        h2.record_success("y", 1.0);
        h2.prune_noise(2);
        assert_eq!(h2.len(), 2);
    }

    #[test]
    fn test_reset_action() {
        let mut history = ActionHistory::new();
        history.record_success("x", 1.0);
        assert!(history.get_action_stats("x").is_some());

        history.reset_action("x");
        assert!(history.get_action_stats("x").is_none());
    }

    #[test]
    fn test_clear_is_empty_len() {
        let mut history = ActionHistory::new();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);

        history.record_success("a", 1.0);
        assert!(!history.is_empty());
        assert_eq!(history.len(), 1);

        history.clear();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_action_names() {
        let mut history = ActionHistory::new();
        history.record_success("alpha", 1.0);
        history.record_success("beta", 1.0);
        history.record_failure("gamma");

        let names = history.action_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));
        assert!(names.contains(&"gamma".to_string()));
    }

    #[test]
    fn test_merge_weighted_duration_exact() {
        // Kills: dur*exec + other_dur*other_exec / total arithmetic mutations
        let mut h1 = ActionHistory::new();
        h1.record_success("act", 3.0); // exec=1, avg=3.0
        h1.record_success("act", 3.0); // exec=2, avg=3.0

        let mut h2 = ActionHistory::new();
        h2.record_success("act", 2.0); // exec=1, avg=2.0
        h2.record_success("act", 2.0); // exec=2, avg=2.0
        h2.record_success("act", 2.0); // exec=3, avg=2.0

        h1.merge(&h2);
        let stats = h1.get_action_stats("act").unwrap();
        assert_eq!(stats.executions, 5);
        assert_eq!(stats.successes, 5);
        // Weighted avg: (3.0*2 + 2.0*3) / 5 = 12/5 = 2.4
        assert!((stats.avg_duration - 2.4).abs() < 1e-6);
    }

    #[test]
    fn test_merge_new_action() {
        // Merging an action that doesn't exist in self yet
        let mut h1 = ActionHistory::new();
        let mut h2 = ActionHistory::new();
        h2.record_success("new_act", 5.0);

        h1.merge(&h2);
        let stats = h1.get_action_stats("new_act").unwrap();
        assert_eq!(stats.executions, 1);
        assert_eq!(stats.successes, 1);
    }

    #[test]
    fn test_record_failure_increments() {
        let mut history = ActionHistory::new();
        history.record_failure("x");
        history.record_failure("x");

        let stats = history.get_action_stats("x").unwrap();
        assert_eq!(stats.executions, 2);
        assert_eq!(stats.failures, 2);
        assert_eq!(stats.successes, 0);
    }

    // ── Round 2 mutation-killing tests ──

    #[test]
    fn test_get_action_stats_mut_returns_mutable_ref() {
        // Kills: get_action_stats_mut → None / Some(Box::leak(...))
        let mut history = ActionHistory::new();
        history.record_success("heal", 2.0);

        // Should return Some with correct data
        let stats = history.get_action_stats_mut("heal").unwrap();
        assert_eq!(stats.executions, 1);
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.avg_duration, 2.0);

        // Mutate through the reference
        stats.executions = 10;
        stats.successes = 8;

        // Verify mutation persisted
        let stats2 = history.get_action_stats("heal").unwrap();
        assert_eq!(stats2.executions, 10);
        assert_eq!(stats2.successes, 8);
    }

    #[test]
    fn test_get_action_stats_mut_none_for_missing() {
        let mut history = ActionHistory::new();
        assert!(history.get_action_stats_mut("nonexistent").is_none());
    }
}
