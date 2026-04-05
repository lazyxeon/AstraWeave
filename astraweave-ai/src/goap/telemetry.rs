// Telemetry hooks for GOAP plan quality metrics and execution outcomes
// Phase 2: Engine Integration

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Telemetry event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TelemetryEvent {
    /// A plan was generated
    PlanGenerated {
        plan_id: String,
        timestamp: f32,
        step_count: usize,
        planning_time_ms: f64,
        action_names: Vec<String>,
    },
    /// A plan step was executed
    StepExecuted {
        plan_id: String,
        action_name: String,
        success: bool,
        duration_ms: f64,
    },
    /// A plan was completed
    PlanCompleted {
        plan_id: String,
        total_duration_ms: f64,
        steps_executed: usize,
        steps_failed: usize,
    },
    /// A plan was abandoned
    PlanAbandoned {
        plan_id: String,
        reason: String,
        steps_completed: usize,
    },
    /// Planning failed
    PlanningFailed {
        timestamp: f32,
        goal_description: String,
        reason: String,
    },
}

/// Aggregate metrics for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMetrics {
    pub total_plans_generated: usize,
    pub total_plans_completed: usize,
    pub total_plans_abandoned: usize,
    pub total_planning_failures: usize,
    pub avg_planning_time_ms: f64,
    pub avg_steps_per_plan: f32,
    pub avg_plan_success_rate: f32,
    pub fastest_plan_ms: f64,
    pub slowest_plan_ms: f64,
}

impl Default for PlanMetrics {
    fn default() -> Self {
        Self {
            total_plans_generated: 0,
            total_plans_completed: 0,
            total_plans_abandoned: 0,
            total_planning_failures: 0,
            avg_planning_time_ms: 0.0,
            avg_steps_per_plan: 0.0,
            avg_plan_success_rate: 0.0,
            fastest_plan_ms: f64::MAX,
            slowest_plan_ms: 0.0,
        }
    }
}

/// Telemetry collector with ring buffer
pub struct TelemetryCollector {
    events: VecDeque<TelemetryEvent>,
    max_events: usize,
    metrics: PlanMetrics,
    current_plan_start: Option<Instant>,
}

impl TelemetryCollector {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(max_events),
            max_events,
            metrics: PlanMetrics::default(),
            current_plan_start: None,
        }
    }

    /// Record a telemetry event
    pub fn record(&mut self, event: TelemetryEvent) {
        // Update metrics based on event type
        match &event {
            TelemetryEvent::PlanGenerated {
                planning_time_ms,
                step_count,
                ..
            } => {
                self.metrics.total_plans_generated += 1;

                // Update averages
                let n = self.metrics.total_plans_generated as f64;
                self.metrics.avg_planning_time_ms =
                    (self.metrics.avg_planning_time_ms * (n - 1.0) + planning_time_ms) / n;

                let n = self.metrics.total_plans_generated as f32;
                self.metrics.avg_steps_per_plan =
                    (self.metrics.avg_steps_per_plan * (n - 1.0) + *step_count as f32) / n;

                // Update min/max
                if *planning_time_ms < self.metrics.fastest_plan_ms {
                    self.metrics.fastest_plan_ms = *planning_time_ms;
                }
                if *planning_time_ms > self.metrics.slowest_plan_ms {
                    self.metrics.slowest_plan_ms = *planning_time_ms;
                }

                self.current_plan_start = Some(Instant::now());
            }
            TelemetryEvent::PlanCompleted { .. } => {
                self.metrics.total_plans_completed += 1;
                self.update_success_rate();
                self.current_plan_start = None;
            }
            TelemetryEvent::PlanAbandoned { .. } => {
                self.metrics.total_plans_abandoned += 1;
                self.update_success_rate();
                self.current_plan_start = None;
            }
            TelemetryEvent::PlanningFailed { .. } => {
                self.metrics.total_planning_failures += 1;
            }
            _ => {}
        }

        // Add to ring buffer
        if self.events.len() >= self.max_events {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    fn update_success_rate(&mut self) {
        let total_attempted =
            self.metrics.total_plans_completed + self.metrics.total_plans_abandoned;
        if total_attempted > 0 {
            self.metrics.avg_plan_success_rate =
                self.metrics.total_plans_completed as f32 / total_attempted as f32;
        }
    }

    /// Get current metrics
    pub fn metrics(&self) -> &PlanMetrics {
        &self.metrics
    }

    /// Get all recorded events
    pub fn events(&self) -> &VecDeque<TelemetryEvent> {
        &self.events
    }

    /// Clear all events (keeps metrics)
    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    /// Reset everything
    pub fn reset(&mut self) {
        self.events.clear();
        self.metrics = PlanMetrics::default();
        self.current_plan_start = None;
    }

    /// Export events as JSON
    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.events)
    }

    /// Print metrics summary
    pub fn print_metrics(&self) {
        println!("\n╔════════════════════════════════════════╗");
        println!("║       GOAP Telemetry Metrics           ║");
        println!("╚════════════════════════════════════════╝\n");

        println!("📊 Plan Generation:");
        println!("   • Total plans: {}", self.metrics.total_plans_generated);
        println!("   • Completed: {}", self.metrics.total_plans_completed);
        println!("   • Abandoned: {}", self.metrics.total_plans_abandoned);
        println!("   • Failures: {}", self.metrics.total_planning_failures);
        println!(
            "   • Success rate: {:.1}%",
            self.metrics.avg_plan_success_rate * 100.0
        );

        println!("\n⏱️  Performance:");
        println!(
            "   • Avg planning time: {:.2}ms",
            self.metrics.avg_planning_time_ms
        );
        println!("   • Fastest: {:.2}ms", self.metrics.fastest_plan_ms);
        println!("   • Slowest: {:.2}ms", self.metrics.slowest_plan_ms);

        println!("\n📈 Plan Characteristics:");
        println!("   • Avg steps: {:.1}", self.metrics.avg_steps_per_plan);

        println!("\n════════════════════════════════════════\n");
    }
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new(1000) // Default: keep last 1000 events
    }
}

/// Helper to track execution of a single plan
pub struct PlanExecutionTracker {
    plan_id: String,
    start_time: Instant,
    steps_executed: usize,
    steps_failed: usize,
}

impl PlanExecutionTracker {
    pub fn new(plan_id: String) -> Self {
        Self {
            plan_id,
            start_time: Instant::now(),
            steps_executed: 0,
            steps_failed: 0,
        }
    }

    pub fn record_step(
        &mut self,
        action_name: String,
        success: bool,
        duration: Duration,
    ) -> TelemetryEvent {
        if success {
            self.steps_executed += 1;
        } else {
            self.steps_failed += 1;
        }

        TelemetryEvent::StepExecuted {
            plan_id: self.plan_id.clone(),
            action_name,
            success,
            duration_ms: duration.as_secs_f64() * 1000.0,
        }
    }

    pub fn complete(self) -> TelemetryEvent {
        let total_duration_ms = self.start_time.elapsed().as_secs_f64() * 1000.0;
        TelemetryEvent::PlanCompleted {
            plan_id: self.plan_id,
            total_duration_ms,
            steps_executed: self.steps_executed,
            steps_failed: self.steps_failed,
        }
    }

    pub fn abandon(self, reason: String) -> TelemetryEvent {
        TelemetryEvent::PlanAbandoned {
            plan_id: self.plan_id,
            reason,
            steps_completed: self.steps_executed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_collector() {
        let mut collector = TelemetryCollector::new(10);

        collector.record(TelemetryEvent::PlanGenerated {
            plan_id: "plan-1".to_string(),
            timestamp: 1.0,
            step_count: 3,
            planning_time_ms: 2.5,
            action_names: vec!["move".to_string(), "attack".to_string()],
        });

        collector.record(TelemetryEvent::PlanCompleted {
            plan_id: "plan-1".to_string(),
            total_duration_ms: 100.0,
            steps_executed: 3,
            steps_failed: 0,
        });

        assert_eq!(collector.metrics().total_plans_generated, 1);
        assert_eq!(collector.metrics().total_plans_completed, 1);
        assert_eq!(collector.metrics().avg_plan_success_rate, 1.0);
    }

    #[test]
    fn test_ring_buffer() {
        let mut collector = TelemetryCollector::new(3);

        for i in 0..5 {
            collector.record(TelemetryEvent::PlanGenerated {
                plan_id: format!("plan-{}", i),
                timestamp: i as f32,
                step_count: 1,
                planning_time_ms: 1.0,
                action_names: vec![],
            });
        }

        // Should only have last 3 events
        assert_eq!(collector.events().len(), 3);
        // But total count should be 5
        assert_eq!(collector.metrics().total_plans_generated, 5);
    }

    #[test]
    fn test_plan_execution_tracker() {
        let mut tracker = PlanExecutionTracker::new("test-plan".to_string());

        let event = tracker.record_step("move".to_string(), true, Duration::from_millis(50));

        match event {
            TelemetryEvent::StepExecuted { success, .. } => {
                assert!(success);
            }
            _ => panic!("Wrong event type"),
        }

        let complete_event = tracker.complete();
        match complete_event {
            TelemetryEvent::PlanCompleted { steps_executed, .. } => {
                assert_eq!(steps_executed, 1);
            }
            _ => panic!("Wrong event type"),
        }
    }

    // ── mutation-killing tests for telemetry metrics ──

    #[test]
    fn test_running_average_two_plans() {
        let mut collector = TelemetryCollector::new(100);

        // Plan 1: time=2.0ms, 3 steps
        collector.record(TelemetryEvent::PlanGenerated {
            plan_id: "p1".into(),
            timestamp: 0.0,
            step_count: 3,
            planning_time_ms: 2.0,
            action_names: vec![],
        });
        assert_eq!(collector.metrics().avg_planning_time_ms, 2.0);
        assert_eq!(collector.metrics().avg_steps_per_plan, 3.0);

        // Plan 2: time=4.0ms, 5 steps
        collector.record(TelemetryEvent::PlanGenerated {
            plan_id: "p2".into(),
            timestamp: 1.0,
            step_count: 5,
            planning_time_ms: 4.0,
            action_names: vec![],
        });
        // avg time = (2.0*1 + 4.0) / 2 = 3.0
        assert!(
            (collector.metrics().avg_planning_time_ms - 3.0).abs() < 0.001,
            "avg_planning_time_ms: {}",
            collector.metrics().avg_planning_time_ms
        );
        // avg steps = (3.0*1 + 5.0) / 2 = 4.0
        assert!(
            (collector.metrics().avg_steps_per_plan - 4.0).abs() < 0.001,
            "avg_steps_per_plan: {}",
            collector.metrics().avg_steps_per_plan
        );
    }

    #[test]
    fn test_fastest_and_slowest_tracking() {
        let mut collector = TelemetryCollector::new(100);

        collector.record(TelemetryEvent::PlanGenerated {
            plan_id: "p1".into(),
            timestamp: 0.0,
            step_count: 1,
            planning_time_ms: 5.0,
            action_names: vec![],
        });

        assert_eq!(collector.metrics().fastest_plan_ms, 5.0);
        assert_eq!(collector.metrics().slowest_plan_ms, 5.0);

        collector.record(TelemetryEvent::PlanGenerated {
            plan_id: "p2".into(),
            timestamp: 1.0,
            step_count: 1,
            planning_time_ms: 2.0,
            action_names: vec![],
        });

        assert_eq!(collector.metrics().fastest_plan_ms, 2.0);
        assert_eq!(collector.metrics().slowest_plan_ms, 5.0);

        collector.record(TelemetryEvent::PlanGenerated {
            plan_id: "p3".into(),
            timestamp: 2.0,
            step_count: 1,
            planning_time_ms: 8.0,
            action_names: vec![],
        });

        assert_eq!(collector.metrics().fastest_plan_ms, 2.0);
        assert_eq!(collector.metrics().slowest_plan_ms, 8.0);
    }

    #[test]
    fn test_plan_abandoned_increments_and_rate() {
        let mut collector = TelemetryCollector::new(100);

        // 2 completed + 1 abandoned → rate = 2/3 ≈ 0.6667
        collector.record(TelemetryEvent::PlanCompleted {
            plan_id: "p1".into(),
            total_duration_ms: 10.0,
            steps_executed: 1,
            steps_failed: 0,
        });
        collector.record(TelemetryEvent::PlanCompleted {
            plan_id: "p2".into(),
            total_duration_ms: 20.0,
            steps_executed: 2,
            steps_failed: 0,
        });
        collector.record(TelemetryEvent::PlanAbandoned {
            plan_id: "p3".into(),
            reason: "test".into(),
            steps_completed: 0,
        });

        assert_eq!(collector.metrics().total_plans_completed, 2);
        assert_eq!(collector.metrics().total_plans_abandoned, 1);
        let rate = collector.metrics().avg_plan_success_rate;
        assert!((rate - 2.0 / 3.0).abs() < 0.01, "success rate: {rate}");
    }

    #[test]
    fn test_planning_failed_increments() {
        let mut collector = TelemetryCollector::new(100);

        collector.record(TelemetryEvent::PlanningFailed {
            timestamp: 0.0,
            goal_description: "test".into(),
            reason: "no path".into(),
        });

        assert_eq!(collector.metrics().total_planning_failures, 1);
    }

    #[test]
    fn test_clear_events_preserves_metrics() {
        let mut collector = TelemetryCollector::new(100);
        collector.record(TelemetryEvent::PlanGenerated {
            plan_id: "p1".into(),
            timestamp: 0.0,
            step_count: 3,
            planning_time_ms: 5.0,
            action_names: vec![],
        });

        collector.clear_events();
        assert!(collector.events().is_empty());
        assert_eq!(collector.metrics().total_plans_generated, 1);
    }

    #[test]
    fn test_reset_clears_everything() {
        let mut collector = TelemetryCollector::new(100);
        collector.record(TelemetryEvent::PlanGenerated {
            plan_id: "p1".into(),
            timestamp: 0.0,
            step_count: 3,
            planning_time_ms: 5.0,
            action_names: vec![],
        });

        collector.reset();
        assert!(collector.events().is_empty());
        assert_eq!(collector.metrics().total_plans_generated, 0);
        assert_eq!(collector.metrics().avg_planning_time_ms, 0.0);
    }

    #[test]
    fn test_export_json() {
        let mut collector = TelemetryCollector::new(100);
        collector.record(TelemetryEvent::PlanGenerated {
            plan_id: "p1".into(),
            timestamp: 0.0,
            step_count: 1,
            planning_time_ms: 1.0,
            action_names: vec!["attack".into()],
        });

        let json = collector.export_json().unwrap();
        assert!(json.contains("p1"), "JSON should contain plan_id");
        assert!(json.contains("attack"), "JSON should contain action name");
        assert!(!json.is_empty());
    }

    #[test]
    fn test_tracker_failed_steps() {
        let mut tracker = PlanExecutionTracker::new("plan-fail".into());

        // Record 2 successes and 1 failure
        let _ = tracker.record_step("move".into(), true, Duration::from_millis(10));
        let _ = tracker.record_step("attack".into(), true, Duration::from_millis(20));
        let step_event = tracker.record_step("heal".into(), false, Duration::from_millis(50));

        // Verify duration_ms conversion
        match step_event {
            TelemetryEvent::StepExecuted {
                duration_ms,
                success,
                ..
            } => {
                assert!(!success);
                assert!((duration_ms - 50.0).abs() < 0.1);
            }
            _ => panic!("expected StepExecuted"),
        }

        let complete_event = tracker.complete();
        match complete_event {
            TelemetryEvent::PlanCompleted {
                steps_executed,
                steps_failed,
                total_duration_ms,
                ..
            } => {
                assert_eq!(steps_executed, 2);
                assert_eq!(steps_failed, 1);
                // total_duration_ms = start_time.elapsed() * 1000.0
                // Test runs in µs, so should be < 500ms
                // Kills * → + (would add ~1000), but / → * is still timing-dependent
                assert!(
                    total_duration_ms < 500.0,
                    "total_duration_ms should be small, got {total_duration_ms}"
                );
                assert!(total_duration_ms >= 0.0, "duration must be non-negative");
            }
            _ => panic!("expected PlanCompleted"),
        }
    }

    #[test]
    fn test_tracker_complete_duration_kills_mutations() {
        // Creates a tracker, sleeps briefly, then completes.
        // Ensures * 1000.0 is correct by asserting a range
        // that excludes + 1000.0 and / 1000.0 results
        let tracker = PlanExecutionTracker::new("duration-test".into());
        std::thread::sleep(Duration::from_millis(10));
        let event = tracker.complete();
        match event {
            TelemetryEvent::PlanCompleted {
                total_duration_ms, ..
            } => {
                // With * 1000: ~10ms (between 5 and 200)
                // With + 1000: ~1000ms (way above 200)
                // With / 1000: ~0.00001ms (way below 5)
                assert!(
                    total_duration_ms > 5.0,
                    "duration too small (/ mutation?): {total_duration_ms}"
                );
                assert!(
                    total_duration_ms < 200.0,
                    "duration too large (+ mutation?): {total_duration_ms}"
                );
            }
            _ => panic!("expected PlanCompleted"),
        }
    }

    #[test]
    fn test_tracker_abandon() {
        let mut tracker = PlanExecutionTracker::new("plan-abandon".into());
        let _ = tracker.record_step("move".into(), true, Duration::from_millis(10));

        let event = tracker.abandon("enemy spotted".into());
        match event {
            TelemetryEvent::PlanAbandoned {
                plan_id,
                reason,
                steps_completed,
            } => {
                assert_eq!(plan_id, "plan-abandon");
                assert_eq!(reason, "enemy spotted");
                assert_eq!(steps_completed, 1);
            }
            _ => panic!("expected PlanAbandoned"),
        }
    }

    #[test]
    fn test_success_rate_exact() {
        let mut collector = TelemetryCollector::new(100);

        // 1 completed, 0 abandoned → rate = 1.0
        collector.record(TelemetryEvent::PlanCompleted {
            plan_id: "p1".into(),
            total_duration_ms: 10.0,
            steps_executed: 1,
            steps_failed: 0,
        });
        assert_eq!(collector.metrics().avg_plan_success_rate, 1.0);

        // 1 completed, 1 abandoned → rate = 0.5
        collector.record(TelemetryEvent::PlanAbandoned {
            plan_id: "p2".into(),
            reason: "test".into(),
            steps_completed: 0,
        });
        assert!(
            (collector.metrics().avg_plan_success_rate - 0.5).abs() < 0.001,
            "rate: {}",
            collector.metrics().avg_plan_success_rate
        );
    }

    #[test]
    fn test_print_metrics_does_not_panic() {
        let collector = TelemetryCollector::new(100);
        collector.print_metrics();
    }
}
