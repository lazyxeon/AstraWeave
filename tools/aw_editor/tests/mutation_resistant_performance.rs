//! Mutation-resistant tests for Performance Panel system
//!
//! These tests target boundary conditions, comparison operators, and boolean return paths
//! to achieve high mutation testing kill rates (90%+).

use aw_editor_lib::panels::{
    AlertSeverity, GpuStats, MemoryStats, MetricUnit, PerfAlert, PerfCategory, PerfMetric,
    PerformanceAction, SubsystemTiming,
};

// ============================================================================
// PERF CATEGORY TESTS
// ============================================================================

mod perf_category_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(PerfCategory::all().len(), 10);
    }

    // Test all variant name() uniqueness
    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = PerfCategory::all().iter().map(|c| c.name()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    // Test all variant icon() uniqueness
    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = PerfCategory::all().iter().map(|c| c.icon()).collect();
        let unique: std::collections::HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test each variant name
    #[test]
    fn test_frame_name() {
        assert_eq!(PerfCategory::Frame.name(), "Frame");
    }

    #[test]
    fn test_cpu_name() {
        assert_eq!(PerfCategory::Cpu.name(), "CPU");
    }

    #[test]
    fn test_gpu_name() {
        assert_eq!(PerfCategory::Gpu.name(), "GPU");
    }

    #[test]
    fn test_memory_name() {
        assert_eq!(PerfCategory::Memory.name(), "Memory");
    }

    #[test]
    fn test_physics_name() {
        assert_eq!(PerfCategory::Physics.name(), "Physics");
    }

    #[test]
    fn test_ai_name() {
        assert_eq!(PerfCategory::Ai.name(), "AI");
    }

    #[test]
    fn test_rendering_name() {
        assert_eq!(PerfCategory::Rendering.name(), "Rendering");
    }

    #[test]
    fn test_audio_name() {
        assert_eq!(PerfCategory::Audio.name(), "Audio");
    }

    #[test]
    fn test_network_name() {
        assert_eq!(PerfCategory::Network.name(), "Network");
    }

    #[test]
    fn test_scripting_name() {
        assert_eq!(PerfCategory::Scripting.name(), "Scripting");
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", PerfCategory::Frame);
        assert!(display.contains("[Img]"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", PerfCategory::Cpu);
        assert!(display.contains("CPU"));
    }
}

// ============================================================================
// METRIC UNIT TESTS
// ============================================================================

mod metric_unit_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(MetricUnit::all().len(), 11);
    }

    // Test is_time_unit()
    #[test]
    fn test_milliseconds_is_time_unit() {
        assert!(MetricUnit::Milliseconds.is_time_unit());
    }

    #[test]
    fn test_microseconds_is_time_unit() {
        assert!(MetricUnit::Microseconds.is_time_unit());
    }

    #[test]
    fn test_nanoseconds_is_time_unit() {
        assert!(MetricUnit::Nanoseconds.is_time_unit());
    }

    #[test]
    fn test_percent_is_not_time_unit() {
        assert!(!MetricUnit::Percent.is_time_unit());
    }

    #[test]
    fn test_bytes_is_not_time_unit() {
        assert!(!MetricUnit::Bytes.is_time_unit());
    }

    #[test]
    fn test_fps_is_not_time_unit() {
        assert!(!MetricUnit::Fps.is_time_unit());
    }

    // Test is_memory_unit()
    #[test]
    fn test_bytes_is_memory_unit() {
        assert!(MetricUnit::Bytes.is_memory_unit());
    }

    #[test]
    fn test_kilobytes_is_memory_unit() {
        assert!(MetricUnit::Kilobytes.is_memory_unit());
    }

    #[test]
    fn test_megabytes_is_memory_unit() {
        assert!(MetricUnit::Megabytes.is_memory_unit());
    }

    #[test]
    fn test_gigabytes_is_memory_unit() {
        assert!(MetricUnit::Gigabytes.is_memory_unit());
    }

    #[test]
    fn test_milliseconds_is_not_memory_unit() {
        assert!(!MetricUnit::Milliseconds.is_memory_unit());
    }

    #[test]
    fn test_percent_is_not_memory_unit() {
        assert!(!MetricUnit::Percent.is_memory_unit());
    }

    #[test]
    fn test_count_is_not_memory_unit() {
        assert!(!MetricUnit::Count.is_memory_unit());
    }

    // Test suffix()
    #[test]
    fn test_milliseconds_suffix() {
        assert_eq!(MetricUnit::Milliseconds.suffix(), "ms");
    }

    #[test]
    fn test_microseconds_suffix() {
        assert_eq!(MetricUnit::Microseconds.suffix(), "µs");
    }

    #[test]
    fn test_percent_suffix() {
        assert_eq!(MetricUnit::Percent.suffix(), "%");
    }

    #[test]
    fn test_megabytes_suffix() {
        assert_eq!(MetricUnit::Megabytes.suffix(), "MB");
    }

    #[test]
    fn test_fps_suffix() {
        assert_eq!(MetricUnit::Fps.suffix(), "FPS");
    }

    #[test]
    fn test_count_suffix_is_empty() {
        assert_eq!(MetricUnit::Count.suffix(), "");
    }

    // Test format()
    #[test]
    fn test_format_fps() {
        assert_eq!(MetricUnit::Fps.format(60.0), "60FPS");
    }

    #[test]
    fn test_format_count() {
        assert_eq!(MetricUnit::Count.format(1234.0), "1234");
    }

    #[test]
    fn test_format_percent() {
        assert_eq!(MetricUnit::Percent.format(50.5), "50.5%");
    }

    #[test]
    fn test_format_milliseconds() {
        assert_eq!(MetricUnit::Milliseconds.format(16.67), "16.67 ms");
    }

    #[test]
    fn test_format_megabytes() {
        assert_eq!(MetricUnit::Megabytes.format(512.0), "512.0 MB");
    }

    // Test all names unique
    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = MetricUnit::all().iter().map(|u| u.name()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }
}

// ============================================================================
// ALERT SEVERITY TESTS
// ============================================================================

mod alert_severity_tests {
    use super::*;

    #[test]
    fn test_all_count() {
        assert_eq!(AlertSeverity::all().len(), 3);
    }

    // Test is_serious()
    #[test]
    fn test_info_is_not_serious() {
        assert!(!AlertSeverity::Info.is_serious());
    }

    #[test]
    fn test_warning_is_serious() {
        assert!(AlertSeverity::Warning.is_serious());
    }

    #[test]
    fn test_critical_is_serious() {
        assert!(AlertSeverity::Critical.is_serious());
    }

    // Test name()
    #[test]
    fn test_info_name() {
        assert_eq!(AlertSeverity::Info.name(), "Info");
    }

    #[test]
    fn test_warning_name() {
        assert_eq!(AlertSeverity::Warning.name(), "Warning");
    }

    #[test]
    fn test_critical_name() {
        assert_eq!(AlertSeverity::Critical.name(), "Critical");
    }

    // Test icon()
    #[test]
    fn test_info_icon() {
        assert_eq!(AlertSeverity::Info.icon(), "ℹ️");
    }

    #[test]
    fn test_warning_icon() {
        assert_eq!(AlertSeverity::Warning.icon(), "[!]");
    }

    #[test]
    fn test_critical_icon() {
        assert_eq!(AlertSeverity::Critical.icon(), "[R]");
    }

    // Test all names unique
    #[test]
    fn test_all_names_unique() {
        let names: Vec<&str> = AlertSeverity::all().iter().map(|s| s.name()).collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique.len());
    }

    // Test all icons unique
    #[test]
    fn test_all_icons_unique() {
        let icons: Vec<&str> = AlertSeverity::all().iter().map(|s| s.icon()).collect();
        let unique: std::collections::HashSet<_> = icons.iter().collect();
        assert_eq!(icons.len(), unique.len());
    }

    // Test Display trait
    #[test]
    fn test_display_contains_icon() {
        let display = format!("{}", AlertSeverity::Warning);
        assert!(display.contains("[!]"));
    }

    #[test]
    fn test_display_contains_name() {
        let display = format!("{}", AlertSeverity::Critical);
        assert!(display.contains("Critical"));
    }
}

// ============================================================================
// PERF METRIC TESTS
// ============================================================================

mod perf_metric_tests {
    use super::*;

    fn make_metric() -> PerfMetric {
        PerfMetric::new("Test", PerfCategory::Frame, MetricUnit::Milliseconds)
    }

    fn make_metric_with_values(values: &[f64]) -> PerfMetric {
        let mut metric = make_metric();
        for &v in values {
            metric.push(v);
        }
        metric
    }

    #[test]
    fn test_new_value_is_zero() {
        let metric = make_metric();
        assert_eq!(metric.value, 0.0);
    }

    #[test]
    fn test_new_history_is_empty() {
        let metric = make_metric();
        assert!(metric.history.is_empty());
    }

    #[test]
    fn test_new_budget_is_none() {
        let metric = make_metric();
        assert!(metric.budget.is_none());
    }

    // Test with_budget()
    #[test]
    fn test_with_budget_sets_budget() {
        let metric = make_metric().with_budget(16.67);
        assert_eq!(metric.budget, Some(16.67));
    }

    // Test push()
    #[test]
    fn test_push_updates_value() {
        let mut metric = make_metric();
        metric.push(10.0);
        assert_eq!(metric.value, 10.0);
    }

    #[test]
    fn test_push_adds_to_history() {
        let mut metric = make_metric();
        metric.push(10.0);
        metric.push(20.0);
        assert_eq!(metric.history.len(), 2);
    }

    // Test average()
    #[test]
    fn test_average_empty_is_zero() {
        let metric = make_metric();
        assert_eq!(metric.average(), 0.0);
    }

    #[test]
    fn test_average_single_value() {
        let metric = make_metric_with_values(&[10.0]);
        assert_eq!(metric.average(), 10.0);
    }

    #[test]
    fn test_average_multiple_values() {
        let metric = make_metric_with_values(&[10.0, 20.0, 30.0]);
        assert_eq!(metric.average(), 20.0);
    }

    // Test min()
    #[test]
    fn test_min_single_value() {
        let metric = make_metric_with_values(&[10.0]);
        assert_eq!(metric.min(), 10.0);
    }

    #[test]
    fn test_min_multiple_values() {
        let metric = make_metric_with_values(&[10.0, 5.0, 20.0]);
        assert_eq!(metric.min(), 5.0);
    }

    // Test max()
    #[test]
    fn test_max_single_value() {
        let metric = make_metric_with_values(&[10.0]);
        assert_eq!(metric.max(), 10.0);
    }

    #[test]
    fn test_max_multiple_values() {
        let metric = make_metric_with_values(&[10.0, 30.0, 20.0]);
        assert_eq!(metric.max(), 30.0);
    }

    // Test is_over_budget()
    #[test]
    fn test_is_over_budget_no_budget() {
        let mut metric = make_metric();
        metric.push(100.0);
        assert!(!metric.is_over_budget());
    }

    #[test]
    fn test_is_over_budget_under() {
        let mut metric = make_metric().with_budget(20.0);
        metric.push(10.0);
        assert!(!metric.is_over_budget());
    }

    #[test]
    fn test_is_over_budget_at_exact() {
        let mut metric = make_metric().with_budget(20.0);
        metric.push(20.0);
        assert!(!metric.is_over_budget());
    }

    #[test]
    fn test_is_over_budget_over() {
        let mut metric = make_metric().with_budget(20.0);
        metric.push(21.0);
        assert!(metric.is_over_budget());
    }

    // Test budget_percent()
    #[test]
    fn test_budget_percent_no_budget() {
        let mut metric = make_metric();
        metric.push(10.0);
        assert!(metric.budget_percent().is_none());
    }

    #[test]
    fn test_budget_percent_half() {
        let mut metric = make_metric().with_budget(20.0);
        metric.push(10.0);
        assert_eq!(metric.budget_percent(), Some(50.0));
    }

    #[test]
    fn test_budget_percent_full() {
        let mut metric = make_metric().with_budget(20.0);
        metric.push(20.0);
        assert_eq!(metric.budget_percent(), Some(100.0));
    }

    #[test]
    fn test_budget_percent_over() {
        let mut metric = make_metric().with_budget(20.0);
        metric.push(30.0);
        assert_eq!(metric.budget_percent(), Some(150.0));
    }
}

// ============================================================================
// SUBSYSTEM TIMING TESTS
// ============================================================================

mod subsystem_timing_tests {
    use super::*;

    fn make_timing() -> SubsystemTiming {
        SubsystemTiming::new("Physics", 2.0)
    }

    #[test]
    fn test_new_time_is_zero() {
        let timing = make_timing();
        assert_eq!(timing.time_ms, 0.0);
    }

    #[test]
    fn test_new_call_count_is_zero() {
        let timing = make_timing();
        assert_eq!(timing.call_count, 0);
    }

    #[test]
    fn test_new_budget_is_set() {
        let timing = make_timing();
        assert_eq!(timing.budget_ms, 2.0);
    }

    #[test]
    fn test_new_history_is_empty() {
        let timing = make_timing();
        assert!(timing.history.is_empty());
    }

    // Test push()
    #[test]
    fn test_push_updates_time() {
        let mut timing = make_timing();
        timing.push(1.5, 10);
        assert_eq!(timing.time_ms, 1.5);
    }

    #[test]
    fn test_push_updates_call_count() {
        let mut timing = make_timing();
        timing.push(1.5, 10);
        assert_eq!(timing.call_count, 10);
    }

    #[test]
    fn test_push_adds_to_history() {
        let mut timing = make_timing();
        timing.push(1.0, 5);
        timing.push(2.0, 6);
        assert_eq!(timing.history.len(), 2);
    }

    // Test average()
    #[test]
    fn test_average_empty_is_zero() {
        let timing = make_timing();
        assert_eq!(timing.average(), 0.0);
    }

    #[test]
    fn test_average_single_value() {
        let mut timing = make_timing();
        timing.push(2.5, 10);
        assert_eq!(timing.average(), 2.5);
    }

    #[test]
    fn test_average_multiple_values() {
        let mut timing = make_timing();
        timing.push(1.0, 5);
        timing.push(2.0, 5);
        timing.push(3.0, 5);
        assert_eq!(timing.average(), 2.0);
    }

    // Test is_over_budget()
    #[test]
    fn test_is_over_budget_under() {
        let mut timing = make_timing();
        timing.push(1.5, 10);
        assert!(!timing.is_over_budget());
    }

    #[test]
    fn test_is_over_budget_at_exact() {
        let mut timing = make_timing();
        timing.push(2.0, 10);
        assert!(!timing.is_over_budget());
    }

    #[test]
    fn test_is_over_budget_over() {
        let mut timing = make_timing();
        timing.push(2.5, 10);
        assert!(timing.is_over_budget());
    }

    // Test budget_percent()
    #[test]
    fn test_budget_percent_zero_budget() {
        let mut timing = SubsystemTiming::new("Test", 0.0);
        timing.push(1.0, 5);
        assert_eq!(timing.budget_percent(), 0.0);
    }

    #[test]
    fn test_budget_percent_half() {
        let mut timing = make_timing();
        timing.push(1.0, 5);
        assert_eq!(timing.budget_percent(), 50.0);
    }

    #[test]
    fn test_budget_percent_full() {
        let mut timing = make_timing();
        timing.push(2.0, 5);
        assert_eq!(timing.budget_percent(), 100.0);
    }

    #[test]
    fn test_budget_percent_over() {
        let mut timing = make_timing();
        timing.push(3.0, 5);
        assert_eq!(timing.budget_percent(), 150.0);
    }
}

// ============================================================================
// MEMORY STATS TESTS
// ============================================================================

mod memory_stats_tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let stats = MemoryStats::default();
        assert_eq!(stats.total_allocated_mb, 0.0);
        assert_eq!(stats.heap_used_mb, 0.0);
        assert_eq!(stats.gpu_used_mb, 0.0);
    }

    // Test heap_usage_percent()
    #[test]
    fn test_heap_usage_percent_zero_committed() {
        let stats = MemoryStats {
            heap_used_mb: 100.0,
            heap_committed_mb: 0.0,
            ..Default::default()
        };
        assert_eq!(stats.heap_usage_percent(), 0.0);
    }

    #[test]
    fn test_heap_usage_percent_half() {
        let stats = MemoryStats {
            heap_used_mb: 50.0,
            heap_committed_mb: 100.0,
            ..Default::default()
        };
        assert_eq!(stats.heap_usage_percent(), 50.0);
    }

    #[test]
    fn test_heap_usage_percent_full() {
        let stats = MemoryStats {
            heap_used_mb: 100.0,
            heap_committed_mb: 100.0,
            ..Default::default()
        };
        assert_eq!(stats.heap_usage_percent(), 100.0);
    }

    // Test gpu_usage_percent()
    #[test]
    fn test_gpu_usage_percent_zero_available() {
        let stats = MemoryStats {
            gpu_used_mb: 1000.0,
            gpu_available_mb: 0.0,
            ..Default::default()
        };
        assert_eq!(stats.gpu_usage_percent(), 0.0);
    }

    #[test]
    fn test_gpu_usage_percent_half() {
        let stats = MemoryStats {
            gpu_used_mb: 4000.0,
            gpu_available_mb: 8000.0,
            ..Default::default()
        };
        assert_eq!(stats.gpu_usage_percent(), 50.0);
    }

    #[test]
    fn test_gpu_usage_percent_full() {
        let stats = MemoryStats {
            gpu_used_mb: 8000.0,
            gpu_available_mb: 8000.0,
            ..Default::default()
        };
        assert_eq!(stats.gpu_usage_percent(), 100.0);
    }
}

// ============================================================================
// GPU STATS TESTS
// ============================================================================

mod gpu_stats_tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let stats = GpuStats::default();
        assert_eq!(stats.frame_time_ms, 0.0);
        assert_eq!(stats.vertex_count, 0);
        assert_eq!(stats.triangle_count, 0);
        assert_eq!(stats.draw_calls, 0);
    }

    #[test]
    fn test_custom_values() {
        let stats = GpuStats {
            frame_time_ms: 16.67,
            vertex_count: 1_000_000,
            triangle_count: 333_333,
            draw_calls: 500,
            texture_bindings: 100,
            shader_switches: 25,
            buffer_uploads_kb: 1024.0,
            render_targets: 4,
            compute_dispatches: 10,
        };
        assert_eq!(stats.frame_time_ms, 16.67);
        assert_eq!(stats.vertex_count, 1_000_000);
        assert_eq!(stats.draw_calls, 500);
    }
}

// ============================================================================
// PERF ALERT TESTS
// ============================================================================

mod perf_alert_tests {
    use super::*;

    fn make_alert() -> PerfAlert {
        PerfAlert::new(
            AlertSeverity::Warning,
            PerfCategory::Frame,
            "Frame spike detected",
        )
    }

    #[test]
    fn test_new_sets_severity() {
        let alert = make_alert();
        assert_eq!(alert.severity, AlertSeverity::Warning);
    }

    #[test]
    fn test_new_sets_category() {
        let alert = make_alert();
        assert_eq!(alert.category, PerfCategory::Frame);
    }

    #[test]
    fn test_new_sets_message() {
        let alert = make_alert();
        assert_eq!(alert.message, "Frame spike detected");
    }

    #[test]
    fn test_age_secs_is_positive() {
        let alert = make_alert();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(alert.age_secs() > 0.0);
    }

    #[test]
    fn test_age_secs_increases_over_time() {
        let alert = make_alert();
        let age1 = alert.age_secs();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let age2 = alert.age_secs();
        assert!(age2 > age1);
    }
}

// ============================================================================
// PERFORMANCE ACTION TESTS
// ============================================================================

mod performance_action_tests {
    use super::*;

    // Test Display trait
    #[test]
    fn test_display_toggle_subsystems() {
        let action = PerformanceAction::ToggleSubsystems(true);
        let display = format!("{}", action);
        assert!(display.contains("subsystems"));
        assert!(display.contains("true"));
    }

    #[test]
    fn test_display_set_target_fps() {
        let action = PerformanceAction::SetTargetFps(144.0);
        let display = format!("{}", action);
        assert!(display.contains("144"));
    }

    #[test]
    fn test_display_reset_statistics() {
        let action = PerformanceAction::ResetStatistics;
        let display = format!("{}", action);
        assert!(display.contains("Reset"));
    }

    #[test]
    fn test_display_export_report() {
        let action = PerformanceAction::ExportReport;
        let display = format!("{}", action);
        assert!(display.contains("Export"));
    }

    #[test]
    fn test_display_subsystem_budget() {
        let action = PerformanceAction::SetSubsystemBudget("Physics".into(), 2.5);
        let display = format!("{}", action);
        assert!(display.contains("Physics"));
        assert!(display.contains("2.5"));
    }

    #[test]
    fn test_display_save_recording() {
        let action = PerformanceAction::SaveRecording("profile.json".into());
        let display = format!("{}", action);
        assert!(display.contains("profile.json"));
    }
}
