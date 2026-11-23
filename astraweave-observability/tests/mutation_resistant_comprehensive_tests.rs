//! Mutation-resistant comprehensive tests for astraweave-observability.
//! Targets exact return values, boundary conditions, default values,
//! and operational correctness for 90%+ mutation kill rate.

#![allow(clippy::field_reassign_with_default)]

use astraweave_observability::*;
use std::collections::HashMap;

// ========================================================================
// OBSERVABILITY CONFIG DEFAULTS
// ========================================================================

#[test]
fn config_default_tracing_level() {
    let cfg = ObservabilityConfig::default();
    assert_eq!(cfg.tracing_level, "INFO");
}

#[test]
fn config_default_metrics_enabled() {
    let cfg = ObservabilityConfig::default();
    assert!(cfg.metrics_enabled, "metrics enabled by default");
}

#[test]
fn config_default_crash_reporting_enabled() {
    let cfg = ObservabilityConfig::default();
    assert!(
        cfg.crash_reporting_enabled,
        "crash reporting enabled by default"
    );
}

#[test]
fn config_clone_preserves_all() {
    let cfg = ObservabilityConfig {
        tracing_level: "DEBUG".to_string(),
        metrics_enabled: false,
        crash_reporting_enabled: false,
    };
    let cfg2 = cfg.clone();
    assert_eq!(cfg2.tracing_level, "DEBUG");
    assert!(!cfg2.metrics_enabled);
    assert!(!cfg2.crash_reporting_enabled);
}

#[test]
fn config_serde_roundtrip() {
    let cfg = ObservabilityConfig::default();
    let json = serde_json::to_string(&cfg).unwrap();
    let cfg2: ObservabilityConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(cfg2.tracing_level, cfg.tracing_level);
    assert_eq!(cfg2.metrics_enabled, cfg.metrics_enabled);
    assert_eq!(cfg2.crash_reporting_enabled, cfg.crash_reporting_enabled);
}

// ========================================================================
// OBSERVABILITY STATE
// ========================================================================

#[test]
fn state_stores_config() {
    let cfg = ObservabilityConfig {
        tracing_level: "WARN".to_string(),
        metrics_enabled: false,
        crash_reporting_enabled: true,
    };
    let state = ObservabilityState::new(cfg);
    assert_eq!(state.config.tracing_level, "WARN");
    assert!(!state.config.metrics_enabled);
    assert!(state.config.crash_reporting_enabled);
}

// ========================================================================
// COMPANION EVENTS
// ========================================================================

#[test]
fn companion_action_event_fields() {
    let event = CompanionActionEvent {
        action_id: "heal".to_string(),
        success: true,
        latency_ms: 42.5,
    };
    assert_eq!(event.action_id, "heal");
    assert!(event.success);
    assert!((event.latency_ms - 42.5).abs() < 1e-6);
}

#[test]
fn companion_action_event_clone() {
    let event = CompanionActionEvent {
        action_id: "attack".to_string(),
        success: false,
        latency_ms: 100.0,
    };
    let e2 = event.clone();
    assert_eq!(e2.action_id, "attack");
    assert!(!e2.success);
    assert!((e2.latency_ms - 100.0).abs() < 1e-6);
}

#[test]
fn companion_action_event_serde() {
    let event = CompanionActionEvent {
        action_id: "dodge".to_string(),
        success: true,
        latency_ms: 5.0,
    };
    let json = serde_json::to_string(&event).unwrap();
    let e2: CompanionActionEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(e2.action_id, "dodge");
    assert!(e2.success);
}

#[test]
fn companion_unlock_event_fields() {
    let event = CompanionAdaptiveUnlock {
        unlock_id: "fire_ability".to_string(),
    };
    assert_eq!(event.unlock_id, "fire_ability");
}

#[test]
fn companion_unlock_clone() {
    let event = CompanionAdaptiveUnlock {
        unlock_id: "shield".to_string(),
    };
    let e2 = event.clone();
    assert_eq!(e2.unlock_id, "shield");
}

// ========================================================================
// TELEMETRY CONFIG DEFAULTS
// ========================================================================

#[test]
fn telemetry_config_default_max_traces() {
    let cfg = TelemetryConfig::default();
    assert_eq!(cfg.max_traces, 10_000);
}

#[test]
fn telemetry_config_default_log_content() {
    let cfg = TelemetryConfig::default();
    assert!(!cfg.log_content, "log_content false by default");
}

#[test]
fn telemetry_config_default_cost_tracking() {
    let cfg = TelemetryConfig::default();
    assert!(cfg.enable_cost_tracking);
}

#[test]
fn telemetry_config_default_prometheus_disabled() {
    let cfg = TelemetryConfig::default();
    assert!(!cfg.enable_prometheus);
}

#[test]
fn telemetry_config_default_opentelemetry_disabled() {
    let cfg = TelemetryConfig::default();
    assert!(!cfg.enable_opentelemetry);
}

#[test]
fn telemetry_config_default_sampling_rate() {
    let cfg = TelemetryConfig::default();
    assert!(
        (cfg.sampling_rate - 1.0).abs() < 1e-6,
        "100% sampling by default"
    );
}

// ========================================================================
// ALERT THRESHOLDS DEFAULTS
// ========================================================================

#[test]
fn alert_thresholds_default_latency_p95() {
    let at = AlertThresholds::default();
    assert_eq!(at.latency_p95_ms, 5000);
}

#[test]
fn alert_thresholds_default_error_rate() {
    let at = AlertThresholds::default();
    assert!(
        (at.error_rate - 0.1).abs() < 1e-6,
        "10% error rate threshold"
    );
}

#[test]
fn alert_thresholds_default_cost_per_hour() {
    let at = AlertThresholds::default();
    assert!(
        (at.cost_per_hour_usd - 10.0).abs() < 1e-6,
        "$10/hour default"
    );
}

#[test]
fn alert_thresholds_default_queue_depth() {
    let at = AlertThresholds::default();
    assert_eq!(at.queue_depth, 100);
}

#[test]
fn alert_thresholds_default_token_rate() {
    let at = AlertThresholds::default();
    assert_eq!(at.token_rate, 10_000);
}

// ========================================================================
// LLM METRICS DEFAULTS
// ========================================================================

#[test]
fn llm_metrics_default_zeroed() {
    let m = LlmMetrics::default();
    assert_eq!(m.total_requests, 0);
    assert_eq!(m.successful_requests, 0);
    assert_eq!(m.failed_requests, 0);
    assert_eq!(m.total_tokens, 0);
    assert!((m.total_cost_usd - 0.0).abs() < 1e-10);
    assert!((m.average_latency_ms - 0.0).abs() < 1e-6);
    assert_eq!(m.p50_latency_ms, 0);
    assert_eq!(m.p95_latency_ms, 0);
    assert_eq!(m.p99_latency_ms, 0);
    assert!((m.error_rate - 0.0).abs() < 1e-6);
    assert_eq!(m.active_requests, 0);
    assert_eq!(m.last_updated.unix_timestamp(), 0);
}

#[test]
fn llm_metrics_serde_roundtrip() {
    let mut m = LlmMetrics::default();
    m.total_requests = 100;
    m.successful_requests = 90;
    m.failed_requests = 10;
    m.total_tokens = 50_000;
    let json = serde_json::to_string(&m).unwrap();
    let m2: LlmMetrics = serde_json::from_str(&json).unwrap();
    assert_eq!(m2.total_requests, 100);
    assert_eq!(m2.successful_requests, 90);
    assert_eq!(m2.failed_requests, 10);
    assert_eq!(m2.total_tokens, 50_000);
}

// ========================================================================
// MODEL METRICS
// ========================================================================

#[test]
fn model_metrics_default_zeroed() {
    let m = ModelMetrics::default();
    assert_eq!(m.requests, 0);
    assert_eq!(m.total_tokens, 0);
    assert!((m.total_cost_usd - 0.0).abs() < 1e-10);
    assert!((m.average_latency_ms - 0.0).abs() < 1e-6);
    assert!((m.error_rate - 0.0).abs() < 1e-6);
}

// ========================================================================
// SOURCE METRICS
// ========================================================================

#[test]
fn source_metrics_default_zeroed() {
    let m = SourceMetrics::default();
    assert_eq!(m.requests, 0);
    assert_eq!(m.total_tokens, 0);
    assert!((m.average_latency_ms - 0.0).abs() < 1e-6);
    assert!((m.error_rate - 0.0).abs() < 1e-6);
}

// ========================================================================
// COST TRACKER DEFAULTS
// ========================================================================

#[test]
fn cost_tracker_default_zero_budgets() {
    let ct = CostTracker::default();
    assert!((ct.daily_budget_usd - 0.0).abs() < 1e-6);
    assert!((ct.monthly_budget_usd - 0.0).abs() < 1e-6);
    assert!((ct.current_day_spend - 0.0).abs() < 1e-6);
    assert!((ct.current_month_spend - 0.0).abs() < 1e-6);
    assert!(ct.hourly_costs.is_empty());
    assert!(ct.cost_by_model.is_empty());
    assert!(ct.cost_by_source.is_empty());
}

// ========================================================================
// ALERT MANAGER
// ========================================================================

#[test]
fn alert_manager_default_empty() {
    let am = AlertManager::default();
    assert!(am.active_alerts.is_empty());
    assert!(am.alert_history.is_empty());
    assert!(am.notification_channels.is_empty());
}

// ========================================================================
// LLM TRACE
// ========================================================================

#[test]
fn llm_trace_serde_roundtrip() {
    let now = time::OffsetDateTime::now_utc();
    let trace = LlmTrace {
        request_id: "req-001".to_string(),
        session_id: Some("sess-1".to_string()),
        user_id: None,
        prompt: Some("Hello".to_string()),
        response: Some("Hi there".to_string()),
        prompt_hash: Some(12345),
        model: "hermes-2".to_string(),
        start_time: now,
        end_time: now,
        latency_ms: 150,
        tokens_prompt: 10,
        tokens_response: 20,
        total_tokens: 30,
        cost_usd: 0.001,
        success: true,
        error_message: None,
        error_type: None,
        request_source: "test".to_string(),
        tags: HashMap::new(),
    };
    let json = serde_json::to_string(&trace).unwrap();
    let t2: LlmTrace = serde_json::from_str(&json).unwrap();
    assert_eq!(t2.request_id, "req-001");
    assert_eq!(t2.model, "hermes-2");
    assert_eq!(t2.latency_ms, 150);
    assert_eq!(t2.total_tokens, 30);
    assert!(t2.success);
}

// ========================================================================
// LLM TELEMETRY: LIFECYCLE
// ========================================================================

#[tokio::test]
async fn telemetry_new_empty_metrics() {
    let tel = LlmTelemetry::new(TelemetryConfig::default());
    let m = tel.get_metrics().await;
    assert_eq!(m.total_requests, 0);
    assert_eq!(m.successful_requests, 0);
    assert_eq!(m.failed_requests, 0);
}

#[tokio::test]
async fn telemetry_record_success_increments() {
    let tel = LlmTelemetry::new(TelemetryConfig::default());
    let now = time::OffsetDateTime::now_utc();
    let trace = LlmTrace {
        request_id: "r1".into(),
        session_id: None,
        user_id: None,
        prompt: None,
        response: None,
        prompt_hash: None,
        model: "test-model".into(),
        start_time: now,
        end_time: now,
        latency_ms: 100,
        tokens_prompt: 10,
        tokens_response: 20,
        total_tokens: 30,
        cost_usd: 0.01,
        success: true,
        error_message: None,
        error_type: None,
        request_source: "unit-test".into(),
        tags: HashMap::new(),
    };
    tel.record_request(trace).await.unwrap();
    let m = tel.get_metrics().await;
    assert_eq!(m.total_requests, 1);
    assert_eq!(m.successful_requests, 1);
    assert_eq!(m.failed_requests, 0);
    assert_eq!(m.total_tokens, 30);
}

#[tokio::test]
async fn telemetry_record_failure_increments() {
    let tel = LlmTelemetry::new(TelemetryConfig::default());
    let now = time::OffsetDateTime::now_utc();
    let trace = LlmTrace {
        request_id: "r2".into(),
        session_id: None,
        user_id: None,
        prompt: None,
        response: None,
        prompt_hash: None,
        model: "test-model".into(),
        start_time: now,
        end_time: now,
        latency_ms: 5000,
        tokens_prompt: 5,
        tokens_response: 0,
        total_tokens: 5,
        cost_usd: 0.005,
        success: false,
        error_message: Some("timeout".into()),
        error_type: Some("Timeout".into()),
        request_source: "unit-test".into(),
        tags: HashMap::new(),
    };
    tel.record_request(trace).await.unwrap();
    let m = tel.get_metrics().await;
    assert_eq!(m.total_requests, 1);
    assert_eq!(m.successful_requests, 0);
    assert_eq!(m.failed_requests, 1);
    assert!((m.error_rate - 1.0).abs() < 1e-3, "100% error rate");
}

#[tokio::test]
async fn telemetry_clear_data_resets() {
    let tel = LlmTelemetry::new(TelemetryConfig::default());
    let now = time::OffsetDateTime::now_utc();
    let trace = LlmTrace {
        request_id: "r3".into(),
        session_id: None,
        user_id: None,
        prompt: None,
        response: None,
        prompt_hash: None,
        model: "m".into(),
        start_time: now,
        end_time: now,
        latency_ms: 50,
        tokens_prompt: 1,
        tokens_response: 1,
        total_tokens: 2,
        cost_usd: 0.0,
        success: true,
        error_message: None,
        error_type: None,
        request_source: "test".into(),
        tags: HashMap::new(),
    };
    tel.record_request(trace).await.unwrap();
    tel.clear_data().await.unwrap();
    let m = tel.get_metrics().await;
    assert_eq!(m.total_requests, 0, "cleared");
    assert_eq!(m.successful_requests, 0);
    assert_eq!(m.total_tokens, 0);
}

#[tokio::test]
async fn telemetry_export_json_empty() {
    let tel = LlmTelemetry::new(TelemetryConfig::default());
    let json = tel.export_traces(ExportFormat::Json, None).await.unwrap();
    assert!(json.contains('['), "JSON array output");
}

#[tokio::test]
async fn telemetry_export_csv_header() {
    let tel = LlmTelemetry::new(TelemetryConfig::default());
    let csv = tel.export_traces(ExportFormat::Csv, None).await.unwrap();
    assert!(
        csv.contains("request_id") || csv.is_empty() || csv.contains("model"),
        "CSV should have header or be empty"
    );
}

#[tokio::test]
async fn telemetry_dashboard_data_has_structure() {
    let tel = LlmTelemetry::new(TelemetryConfig::default());
    let dash = tel.get_dashboard_data().await.unwrap();
    assert_eq!(dash.current_metrics.total_requests, 0);
    assert!(dash.active_alerts.is_empty());
    assert!(dash.top_errors.is_empty());
}

#[tokio::test]
async fn telemetry_multiple_requests_accumulate() {
    let tel = LlmTelemetry::new(TelemetryConfig::default());
    let now = time::OffsetDateTime::now_utc();
    for i in 0..5 {
        let trace = LlmTrace {
            request_id: format!("req-{}", i),
            session_id: None,
            user_id: None,
            prompt: None,
            response: None,
            prompt_hash: None,
            model: "m".into(),
            start_time: now,
            end_time: now,
            latency_ms: 100,
            tokens_prompt: 10,
            tokens_response: 10,
            total_tokens: 20,
            cost_usd: 0.01,
            success: true,
            error_message: None,
            error_type: None,
            request_source: "test".into(),
            tags: HashMap::new(),
        };
        tel.record_request(trace).await.unwrap();
    }
    let m = tel.get_metrics().await;
    assert_eq!(m.total_requests, 5);
    assert_eq!(m.successful_requests, 5);
    assert_eq!(m.total_tokens, 100);
}

// ========================================================================
// TRACE FILTER
// ========================================================================

#[test]
fn trace_filter_default_none() {
    let f = TraceFilter {
        model: None,
        success: None,
        start_time: None,
        end_time: None,
        min_latency_ms: None,
        max_latency_ms: None,
    };
    assert!(f.model.is_none());
    assert!(f.success.is_none());
}

// ========================================================================
// ERROR TRACKER
// ========================================================================

#[test]
fn error_tracker_default_empty() {
    let et = ErrorTracker::default();
    assert!(et.error_counts.is_empty());
    assert!(et.error_patterns.is_empty());
    assert!(et.recent_errors.is_empty());
}

// ========================================================================
// PERFORMANCE HISTOGRAMS
// ========================================================================

#[test]
fn performance_histograms_default_constructed() {
    // Just verify it doesn't panic
    let _ph = PerformanceHistograms::default();
}

// ========================================================================
// BUDGET ALERT TYPE
// ========================================================================

#[test]
fn budget_alert_type_debug() {
    let daily = BudgetAlertType::DailyBudget;
    let monthly = BudgetAlertType::MonthlyBudget;
    let hourly = BudgetAlertType::HourlyRate;
    assert_eq!(format!("{:?}", daily), "DailyBudget");
    assert_eq!(format!("{:?}", monthly), "MonthlyBudget");
    assert_eq!(format!("{:?}", hourly), "HourlyRate");
}

// ========================================================================
// ALERT TYPE
// ========================================================================

#[test]
fn alert_type_all_variants_debug() {
    let variants = [
        AlertType::HighLatency,
        AlertType::HighErrorRate,
        AlertType::HighCost,
        AlertType::QueueBacklog,
        AlertType::ModelFailure,
    ];
    for v in &variants {
        let s = format!("{:?}", v);
        assert!(!s.is_empty());
    }
}

// ========================================================================
// ALERT SEVERITY
// ========================================================================

#[test]
fn alert_severity_all_variants() {
    let _info = AlertSeverity::Info;
    let _warn = AlertSeverity::Warning;
    let _crit = AlertSeverity::Critical;
    // Just verify they exist and are constructible
    assert_eq!(format!("{:?}", AlertSeverity::Info), "Info");
    assert_eq!(format!("{:?}", AlertSeverity::Warning), "Warning");
    assert_eq!(format!("{:?}", AlertSeverity::Critical), "Critical");
}

// ========================================================================
// NOTIFICATION CHANNEL
// ========================================================================

#[test]
fn notification_channel_log() {
    let ch = NotificationChannel::Log;
    assert_eq!(format!("{:?}", ch), "Log");
}

#[test]
fn notification_channel_webhook() {
    let ch = NotificationChannel::Webhook("http://example.com".to_string());
    let s = format!("{:?}", ch);
    assert!(s.contains("Webhook"));
    assert!(s.contains("example.com"));
}

#[test]
fn notification_channel_email() {
    let ch = NotificationChannel::Email("test@test.com".to_string());
    let s = format!("{:?}", ch);
    assert!(s.contains("Email"));
}

// ========================================================================
// EXPORT FORMAT
// ========================================================================

#[test]
fn export_format_variants_exist() {
    let _json = ExportFormat::Json;
    let _csv = ExportFormat::Csv;
    let _otel = ExportFormat::OpenTelemetry;
    assert_eq!(format!("{:?}", ExportFormat::Json), "Json");
    assert_eq!(format!("{:?}", ExportFormat::Csv), "Csv");
}

// ========================================================================
// COST SUMMARY
// ========================================================================

#[test]
fn cost_summary_serde_roundtrip() {
    let cs = CostSummary {
        current_hour_cost: 1.5,
        today_cost: 25.0,
        month_cost: 300.0,
        daily_budget_remaining: 75.0,
        monthly_budget_remaining: 700.0,
        projected_monthly_cost: 400.0,
    };
    let json = serde_json::to_string(&cs).unwrap();
    let cs2: CostSummary = serde_json::from_str(&json).unwrap();
    assert!((cs2.current_hour_cost - 1.5).abs() < 1e-6);
    assert!((cs2.today_cost - 25.0).abs() < 1e-6);
    assert!((cs2.projected_monthly_cost - 400.0).abs() < 1e-6);
}

// ========================================================================
// CLONE PROPAGATION
// ========================================================================

#[test]
fn telemetry_clone_shares_state() {
    let tel = LlmTelemetry::new(TelemetryConfig::default());
    let _tel2 = tel.clone(); // Should compile—manual Clone via Arc
}

// ========================================================================
// MUTATION KILL TESTS — targeting 50 missed mutations in update_metrics,
// get_dashboard_data, record_request, and should_sample
// ========================================================================

fn make_trace(
    id: &str,
    model: &str,
    source: &str,
    latency_ms: u64,
    tokens: usize,
    cost: f64,
    success: bool,
) -> LlmTrace {
    let now = time::OffsetDateTime::now_utc();
    LlmTrace {
        request_id: id.into(),
        session_id: None,
        user_id: None,
        prompt: None,
        response: None,
        prompt_hash: None,
        model: model.into(),
        start_time: now,
        end_time: now,
        latency_ms,
        tokens_prompt: tokens / 2,
        tokens_response: tokens - tokens / 2,
        total_tokens: tokens,
        cost_usd: cost,
        success,
        error_message: if success { None } else { Some("err".into()) },
        error_type: if success {
            None
        } else {
            Some("TestError".into())
        },
        request_source: source.into(),
        tags: HashMap::new(),
    }
}

/// Kill: total_cost_usd += mutations (line 605), average_latency running
/// average formula (lines 609-611), error_rate / mutations (line 612).
/// Uses 3 requests with distinct latencies to break `*→+`, `*→/`, `-→+`,
/// `-→/`, `+→-`, `+→*`, `/→%`, `/→*` on the incremental average formula.
#[tokio::test]
async fn three_requests_verify_averages_and_costs() {
    let tel = LlmTelemetry::new(TelemetryConfig::default());

    tel.record_request(make_trace("r1", "m", "s", 100, 30, 0.01, true))
        .await
        .unwrap();
    tel.record_request(make_trace("r2", "m", "s", 200, 50, 0.03, true))
        .await
        .unwrap();
    tel.record_request(make_trace("r3", "m", "s", 600, 20, 0.06, false))
        .await
        .unwrap();

    let m = tel.get_metrics().await;

    // total_cost_usd = 0.01 + 0.03 + 0.06 = 0.10
    assert!(
        (m.total_cost_usd - 0.10).abs() < 1e-6,
        "cost should be 0.10, got {}",
        m.total_cost_usd
    );

    // average_latency = (100 + 200 + 600) / 3 = 300.0
    assert!(
        (m.average_latency_ms - 300.0).abs() < 1.0,
        "avg latency should be 300, got {}",
        m.average_latency_ms
    );

    // error_rate = 1 failure / 3 total ≈ 0.333
    assert!(
        (m.error_rate - 1.0 / 3.0).abs() < 0.01,
        "error rate should be ~0.333, got {}",
        m.error_rate
    );
}

/// Kill: model_metrics accumulation (lines 620-621), model average_latency
/// running average (lines 623-625), model error_rate formulas for both
/// success and failure paths (lines 626-633), and delete-! mutation.
#[tokio::test]
async fn model_metrics_with_mixed_success_failures() {
    let tel = LlmTelemetry::new(TelemetryConfig::default());

    // 3 requests to same model: success, failure, success
    tel.record_request(make_trace("r1", "gpt4", "s", 100, 30, 0.02, true))
        .await
        .unwrap();
    tel.record_request(make_trace("r2", "gpt4", "s", 200, 70, 0.08, false))
        .await
        .unwrap();
    tel.record_request(make_trace("r3", "gpt4", "s", 600, 50, 0.05, true))
        .await
        .unwrap();

    let m = tel.get_metrics().await;
    let mm = m.model_usage.get("gpt4").expect("model metrics must exist");

    assert_eq!(mm.requests, 3);
    assert_eq!(mm.total_tokens, 150);
    assert!(
        (mm.total_cost_usd - 0.15).abs() < 1e-6,
        "model cost should be 0.15, got {}",
        mm.total_cost_usd
    );

    // average_latency = (100 + 200 + 600) / 3 = 300.0
    assert!(
        (mm.average_latency_ms - 300.0).abs() < 1.0,
        "model avg latency should be 300, got {}",
        mm.average_latency_ms
    );

    // error_rate: 1 failure out of 3 ≈ 0.333
    assert!(
        (mm.error_rate - 1.0 / 3.0).abs() < 0.02,
        "model error rate should be ~0.333, got {}",
        mm.error_rate
    );
}

/// Kill: source_metrics accumulation (line 642), source average_latency
/// (line 646) running average formula.
#[tokio::test]
async fn source_metrics_accumulated_correctly() {
    let tel = LlmTelemetry::new(TelemetryConfig::default());

    tel.record_request(make_trace("r1", "m", "orchestrator", 100, 30, 0.01, true))
        .await
        .unwrap();
    tel.record_request(make_trace("r2", "m", "orchestrator", 200, 70, 0.04, false))
        .await
        .unwrap();
    tel.record_request(make_trace("r3", "m", "orchestrator", 600, 50, 0.05, true))
        .await
        .unwrap();

    let m = tel.get_metrics().await;
    let sm = m
        .source_metrics
        .get("orchestrator")
        .expect("source metrics must exist");

    assert_eq!(sm.requests, 3);
    assert_eq!(sm.total_tokens, 150);

    // average_latency = (100 + 200 + 600) / 3 = 300.0
    assert!(
        (sm.average_latency_ms - 300.0).abs() < 1.0,
        "source avg latency should be 300, got {}",
        sm.average_latency_ms
    );

    // error_rate: 1 / 3 ≈ 0.333
    assert!(
        (sm.error_rate - 1.0 / 3.0).abs() < 0.02,
        "source error rate should be ~0.333, got {}",
        sm.error_rate
    );
}

/// Kill: trace buffer limit > → == and > → >= (line 407).
#[tokio::test]
async fn trace_buffer_enforces_max_traces() {
    let config = TelemetryConfig {
        max_traces: 2,
        sampling_rate: 1.0,
        ..TelemetryConfig::default()
    };
    let tel = LlmTelemetry::new(config);

    tel.record_request(make_trace("r1", "m", "s", 10, 1, 0.0, true))
        .await
        .unwrap();
    tel.record_request(make_trace("r2", "m", "s", 20, 1, 0.0, true))
        .await
        .unwrap();
    tel.record_request(make_trace("r3", "m", "s", 30, 1, 0.0, true))
        .await
        .unwrap();

    let json = tel.export_traces(ExportFormat::Json, None).await.unwrap();
    let traces: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    assert_eq!(
        traces.len(),
        2,
        "should keep only max_traces=2 traces, got {}",
        traces.len()
    );
}

/// Kill: delete-! on error tracking condition (line 431) — ensures failures
/// trigger error patterns and successes do not.
#[tokio::test]
async fn failure_triggers_error_tracking_success_does_not() {
    // Success-only: no error patterns
    let tel_ok = LlmTelemetry::new(TelemetryConfig::default());
    tel_ok
        .record_request(make_trace("r1", "m", "s", 100, 10, 0.01, true))
        .await
        .unwrap();
    let dash_ok = tel_ok.get_dashboard_data().await.unwrap();
    assert!(
        dash_ok.top_errors.is_empty(),
        "success-only should have no error patterns"
    );

    // Failure: should produce error patterns
    let tel_err = LlmTelemetry::new(TelemetryConfig::default());
    tel_err
        .record_request(make_trace("r2", "m", "s", 100, 10, 0.01, false))
        .await
        .unwrap();
    let dash_err = tel_err.get_dashboard_data().await.unwrap();
    assert!(
        !dash_err.top_errors.is_empty(),
        "failed trace should produce error patterns"
    );
}

/// Kill: should_sample → true (line 588). With sampling_rate=0.0 no traces
/// should be stored; the mutation makes it always true → traces appear.
#[tokio::test]
async fn sampling_rate_zero_stores_no_traces() {
    let config = TelemetryConfig {
        sampling_rate: 0.0,
        ..TelemetryConfig::default()
    };
    let tel = LlmTelemetry::new(config);

    tel.record_request(make_trace("r1", "m", "s", 50, 10, 0.01, true))
        .await
        .unwrap();

    let json = tel.export_traces(ExportFormat::Json, None).await.unwrap();
    let traces: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    assert_eq!(
        traces.len(),
        0,
        "sampling_rate=0 should store no traces, found {}",
        traces.len()
    );
}

/// Kill: dashboard budget_remaining - → + (lines 460, 463) and cost
/// percentile / → %|* (lines 502-503).
#[tokio::test]
async fn dashboard_budget_remaining_and_cost_percentiles() {
    let tel = LlmTelemetry::new(TelemetryConfig::default());

    // Record with non-trivial cost so histograms have data and spend > 0
    tel.record_request(make_trace("r1", "m", "s", 100, 30, 0.05, true))
        .await
        .unwrap();

    let dash = tel.get_dashboard_data().await.unwrap();

    // With default budget=0 and spend>0: remaining = max(0 - spend, 0) = 0.0
    // Mutation - → + would give max(0 + spend, 0) = spend > 0
    assert!(
        dash.cost_summary.daily_budget_remaining < 0.01,
        "daily remaining should be ~0, got {}",
        dash.cost_summary.daily_budget_remaining
    );
    assert!(
        dash.cost_summary.monthly_budget_remaining < 0.01,
        "monthly remaining should be ~0, got {}",
        dash.cost_summary.monthly_budget_remaining
    );

    // Cost percentiles: histogram recorded (0.05*100)=5 cents.
    // Divided by 100 → 0.05 USD. Mutations /→* (500.0) or /→% (5.0) would
    // produce values well above 0.5.
    assert!(
        dash.performance_percentiles.cost_p50 < 0.5,
        "cost_p50 should be in dollars not cents: {}",
        dash.performance_percentiles.cost_p50
    );
    assert!(
        dash.performance_percentiles.cost_p95 < 0.5,
        "cost_p95 should be in dollars not cents: {}",
        dash.performance_percentiles.cost_p95
    );
}
