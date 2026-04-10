// tools/aw_editor/src/panels/performance_panel.rs - Comprehensive performance monitoring panel

use super::Panel;
use crate::runtime::RuntimeStats;
use astract::widgets::PerformanceBudgetWidget;
use egui::{Color32, RichText, Ui};
use std::collections::{HashMap, VecDeque};
use tracing::info;

// ═══════════════════════════════════════════════════════════════════════════════════
// PERFORMANCE CATEGORIES & METRICS
// ═══════════════════════════════════════════════════════════════════════════════════

/// Performance monitoring category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PerfCategory {
    Frame,
    Cpu,
    Gpu,
    Memory,
    Physics,
    Ai,
    Rendering,
    Audio,
    Network,
    Scripting,
}

impl std::fmt::Display for PerfCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl PerfCategory {
    pub fn name(&self) -> &'static str {
        match self {
            PerfCategory::Frame => "Frame",
            PerfCategory::Cpu => "CPU",
            PerfCategory::Gpu => "GPU",
            PerfCategory::Memory => "Memory",
            PerfCategory::Physics => "Physics",
            PerfCategory::Ai => "AI",
            PerfCategory::Rendering => "Rendering",
            PerfCategory::Audio => "Audio",
            PerfCategory::Network => "Network",
            PerfCategory::Scripting => "Scripting",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            PerfCategory::Frame => "[Img]",
            PerfCategory::Cpu => "[Wrn]",
            PerfCategory::Gpu => "[Gp]",
            PerfCategory::Memory => "[Save]",
            PerfCategory::Physics => "[Zap]",
            PerfCategory::Ai => "[Brain]",
            PerfCategory::Rendering => "[Art]",
            PerfCategory::Audio => "[Snd]",
            PerfCategory::Network => "[Net]",
            PerfCategory::Scripting => "📜",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            PerfCategory::Frame => Color32::from_rgb(100, 200, 255),
            PerfCategory::Cpu => Color32::from_rgb(255, 180, 100),
            PerfCategory::Gpu => Color32::from_rgb(180, 100, 255),
            PerfCategory::Memory => Color32::from_rgb(100, 255, 180),
            PerfCategory::Physics => Color32::from_rgb(255, 100, 100),
            PerfCategory::Ai => Color32::from_rgb(255, 200, 100),
            PerfCategory::Rendering => Color32::from_rgb(100, 180, 255),
            PerfCategory::Audio => Color32::from_rgb(200, 200, 100),
            PerfCategory::Network => Color32::from_rgb(150, 150, 255),
            PerfCategory::Scripting => Color32::from_rgb(200, 150, 200),
        }
    }

    pub fn all() -> &'static [PerfCategory] {
        &[
            PerfCategory::Frame,
            PerfCategory::Cpu,
            PerfCategory::Gpu,
            PerfCategory::Memory,
            PerfCategory::Physics,
            PerfCategory::Ai,
            PerfCategory::Rendering,
            PerfCategory::Audio,
            PerfCategory::Network,
            PerfCategory::Scripting,
        ]
    }
}

/// Individual performance metric
#[derive(Debug, Clone)]
pub struct PerfMetric {
    pub name: String,
    pub category: PerfCategory,
    pub value: f64,
    pub unit: MetricUnit,
    pub budget: Option<f64>,
    pub history: VecDeque<f64>,
}

impl PerfMetric {
    pub fn new(name: impl Into<String>, category: PerfCategory, unit: MetricUnit) -> Self {
        Self {
            name: name.into(),
            category,
            value: 0.0,
            unit,
            budget: None,
            history: VecDeque::with_capacity(120),
        }
    }

    pub fn with_budget(mut self, budget: f64) -> Self {
        self.budget = Some(budget);
        self
    }

    pub fn push(&mut self, value: f64) {
        self.value = value;
        self.history.push_back(value);
        if self.history.len() > 120 {
            self.history.pop_front();
        }
    }

    pub fn average(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        self.history.iter().sum::<f64>() / self.history.len() as f64
    }

    pub fn min(&self) -> f64 {
        self.history.iter().copied().fold(f64::MAX, f64::min)
    }

    pub fn max(&self) -> f64 {
        self.history.iter().copied().fold(f64::MIN, f64::max)
    }

    pub fn is_over_budget(&self) -> bool {
        self.budget.map(|b| self.value > b).unwrap_or(false)
    }

    pub fn budget_percent(&self) -> Option<f64> {
        self.budget.map(|b| (self.value / b) * 100.0)
    }
}

/// Metric unit for display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MetricUnit {
    Milliseconds,
    Microseconds,
    Nanoseconds,
    Percent,
    Bytes,
    Kilobytes,
    Megabytes,
    Gigabytes,
    Count,
    PerSecond,
    Fps,
}

impl std::fmt::Display for MetricUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.suffix())
    }
}

impl MetricUnit {
    pub fn all() -> &'static [MetricUnit] {
        &[
            MetricUnit::Milliseconds,
            MetricUnit::Microseconds,
            MetricUnit::Nanoseconds,
            MetricUnit::Percent,
            MetricUnit::Bytes,
            MetricUnit::Kilobytes,
            MetricUnit::Megabytes,
            MetricUnit::Gigabytes,
            MetricUnit::Count,
            MetricUnit::PerSecond,
            MetricUnit::Fps,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            MetricUnit::Milliseconds => "Milliseconds",
            MetricUnit::Microseconds => "Microseconds",
            MetricUnit::Nanoseconds => "Nanoseconds",
            MetricUnit::Percent => "Percent",
            MetricUnit::Bytes => "Bytes",
            MetricUnit::Kilobytes => "Kilobytes",
            MetricUnit::Megabytes => "Megabytes",
            MetricUnit::Gigabytes => "Gigabytes",
            MetricUnit::Count => "Count",
            MetricUnit::PerSecond => "Per Second",
            MetricUnit::Fps => "FPS",
        }
    }

    /// Returns true if this is a time unit
    pub fn is_time_unit(&self) -> bool {
        matches!(
            self,
            MetricUnit::Milliseconds | MetricUnit::Microseconds | MetricUnit::Nanoseconds
        )
    }

    /// Returns true if this is a memory unit
    pub fn is_memory_unit(&self) -> bool {
        matches!(
            self,
            MetricUnit::Bytes
                | MetricUnit::Kilobytes
                | MetricUnit::Megabytes
                | MetricUnit::Gigabytes
        )
    }

    pub fn suffix(&self) -> &'static str {
        match self {
            MetricUnit::Milliseconds => "ms",
            MetricUnit::Microseconds => "µs",
            MetricUnit::Nanoseconds => "ns",
            MetricUnit::Percent => "%",
            MetricUnit::Bytes => "B",
            MetricUnit::Kilobytes => "KB",
            MetricUnit::Megabytes => "MB",
            MetricUnit::Gigabytes => "GB",
            MetricUnit::Count => "",
            MetricUnit::PerSecond => "/s",
            MetricUnit::Fps => "FPS",
        }
    }

    pub fn format(&self, value: f64) -> String {
        match self {
            MetricUnit::Fps | MetricUnit::Count => format!("{:.0}{}", value, self.suffix()),
            MetricUnit::Percent => format!("{:.1}{}", value, self.suffix()),
            MetricUnit::Bytes
            | MetricUnit::Kilobytes
            | MetricUnit::Megabytes
            | MetricUnit::Gigabytes => {
                format!("{:.1} {}", value, self.suffix())
            }
            _ => format!("{:.2} {}", value, self.suffix()),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// SUBSYSTEM TIMING
// ═══════════════════════════════════════════════════════════════════════════════════

/// Timing data for a subsystem
#[derive(Debug, Clone, Default)]
pub struct SubsystemTiming {
    pub name: String,
    pub time_ms: f64,
    pub call_count: u32,
    pub budget_ms: f64,
    pub history: VecDeque<f64>,
}

impl SubsystemTiming {
    pub fn new(name: impl Into<String>, budget_ms: f64) -> Self {
        Self {
            name: name.into(),
            time_ms: 0.0,
            call_count: 0,
            budget_ms,
            history: VecDeque::with_capacity(60),
        }
    }

    pub fn push(&mut self, time_ms: f64, calls: u32) {
        self.time_ms = time_ms;
        self.call_count = calls;
        self.history.push_back(time_ms);
        if self.history.len() > 60 {
            self.history.pop_front();
        }
    }

    pub fn average(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        self.history.iter().sum::<f64>() / self.history.len() as f64
    }

    pub fn is_over_budget(&self) -> bool {
        self.time_ms > self.budget_ms
    }

    pub fn budget_percent(&self) -> f64 {
        if self.budget_ms > 0.0 {
            (self.time_ms / self.budget_ms) * 100.0
        } else {
            0.0
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// MEMORY STATS
// ═══════════════════════════════════════════════════════════════════════════════════

/// Memory usage statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub total_allocated_mb: f64,
    pub total_reserved_mb: f64,
    pub heap_used_mb: f64,
    pub heap_committed_mb: f64,
    pub gpu_used_mb: f64,
    pub gpu_available_mb: f64,
    pub allocations_per_frame: u32,
    pub deallocations_per_frame: u32,
    pub peak_usage_mb: f64,
}

impl MemoryStats {
    pub fn heap_usage_percent(&self) -> f64 {
        if self.heap_committed_mb > 0.0 {
            (self.heap_used_mb / self.heap_committed_mb) * 100.0
        } else {
            0.0
        }
    }

    pub fn gpu_usage_percent(&self) -> f64 {
        if self.gpu_available_mb > 0.0 {
            (self.gpu_used_mb / self.gpu_available_mb) * 100.0
        } else {
            0.0
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// GPU STATS
// ═══════════════════════════════════════════════════════════════════════════════════

/// GPU performance statistics
#[derive(Debug, Clone, Default)]
pub struct GpuStats {
    pub frame_time_ms: f64,
    pub vertex_count: u64,
    pub triangle_count: u64,
    pub draw_calls: u32,
    pub texture_bindings: u32,
    pub shader_switches: u32,
    pub buffer_uploads_kb: f64,
    pub render_targets: u32,
    pub compute_dispatches: u32,
}

// ═══════════════════════════════════════════════════════════════════════════════════
// PERFORMANCE ALERT
// ═══════════════════════════════════════════════════════════════════════════════════

/// Performance alert severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl AlertSeverity {
    pub fn all() -> &'static [AlertSeverity] {
        &[
            AlertSeverity::Info,
            AlertSeverity::Warning,
            AlertSeverity::Critical,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "Info",
            AlertSeverity::Warning => "Warning",
            AlertSeverity::Critical => "Critical",
        }
    }

    /// Returns true if this is a serious alert
    pub fn is_serious(&self) -> bool {
        matches!(self, AlertSeverity::Warning | AlertSeverity::Critical)
    }

    pub fn color(&self) -> Color32 {
        match self {
            AlertSeverity::Info => Color32::from_rgb(100, 180, 255),
            AlertSeverity::Warning => Color32::YELLOW,
            AlertSeverity::Critical => Color32::RED,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "ℹ️",
            AlertSeverity::Warning => "[!]",
            AlertSeverity::Critical => "[R]",
        }
    }
}

/// Performance alert
#[derive(Debug, Clone)]
pub struct PerfAlert {
    pub severity: AlertSeverity,
    pub category: PerfCategory,
    pub message: String,
    pub timestamp: std::time::Instant,
}

impl PerfAlert {
    pub fn new(
        severity: AlertSeverity,
        category: PerfCategory,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            category,
            message: message.into(),
            timestamp: std::time::Instant::now(),
        }
    }

    pub fn age_secs(&self) -> f32 {
        self.timestamp.elapsed().as_secs_f32()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// ACTION SYSTEM
// ═══════════════════════════════════════════════════════════════════════════════════

/// Actions that can be triggered from the performance panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PerformanceAction {
    // Display toggles
    ToggleSubsystems(bool),
    ToggleMemory(bool),
    ToggleGpu(bool),
    ToggleMetrics(bool),
    ToggleAlerts(bool),
    ToggleHistoryGraphs(bool),

    // Category selection
    SelectCategory(Option<PerfCategory>),

    // Frame rate settings
    SetTargetFps(f64),

    // Metrics operations
    ResetStatistics,
    ClearAlerts,
    ExportReport,

    // Subsystem control
    ToggleSubsystemProfiling(String, bool),
    SetSubsystemBudget(String, f64),

    // Memory operations
    TriggerGarbageCollection,
    SnapshotMemory,

    // Recording
    StartRecording,
    StopRecording,
    SaveRecording(String),
}

impl std::fmt::Display for PerformanceAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PerformanceAction::ToggleSubsystems(b) => write!(f, "Toggle subsystems: {}", b),
            PerformanceAction::ToggleMemory(b) => write!(f, "Toggle memory: {}", b),
            PerformanceAction::ToggleGpu(b) => write!(f, "Toggle GPU: {}", b),
            PerformanceAction::ToggleMetrics(b) => write!(f, "Toggle metrics: {}", b),
            PerformanceAction::ToggleAlerts(b) => write!(f, "Toggle alerts: {}", b),
            PerformanceAction::ToggleHistoryGraphs(b) => write!(f, "Toggle history graphs: {}", b),
            PerformanceAction::SelectCategory(cat) => write!(f, "Select category: {:?}", cat),
            PerformanceAction::SetTargetFps(fps) => write!(f, "Set target FPS: {:.0}", fps),
            PerformanceAction::ResetStatistics => write!(f, "Reset statistics"),
            PerformanceAction::ClearAlerts => write!(f, "Clear alerts"),
            PerformanceAction::ExportReport => write!(f, "Export report"),
            PerformanceAction::ToggleSubsystemProfiling(name, b) => {
                write!(f, "Toggle {} profiling: {}", name, b)
            }
            PerformanceAction::SetSubsystemBudget(name, budget) => {
                write!(f, "Set {} budget: {:.2}ms", name, budget)
            }
            PerformanceAction::TriggerGarbageCollection => write!(f, "Trigger GC"),
            PerformanceAction::SnapshotMemory => write!(f, "Snapshot memory"),
            PerformanceAction::StartRecording => write!(f, "Start recording"),
            PerformanceAction::StopRecording => write!(f, "Stop recording"),
            PerformanceAction::SaveRecording(path) => write!(f, "Save recording: {}", path),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// PERFORMANCE PANEL
// ═══════════════════════════════════════════════════════════════════════════════════

/// Comprehensive performance monitoring panel
pub struct PerformancePanel {
    // Core widget
    widget: PerformanceBudgetWidget,
    last_update: std::time::Instant,
    frame_count: u64,
    runtime_stats: Option<RuntimeStats>,

    // Metrics
    metrics: HashMap<String, PerfMetric>,

    // Subsystem timings
    subsystems: Vec<SubsystemTiming>,

    // Memory stats
    memory_stats: MemoryStats,

    // GPU stats
    gpu_stats: GpuStats,

    // Alerts
    alerts: VecDeque<PerfAlert>,
    max_alerts: usize,

    // Display options
    show_subsystems: bool,
    show_memory: bool,
    show_gpu: bool,
    show_metrics: bool,
    show_alerts: bool,
    show_history_graphs: bool,
    selected_category: Option<PerfCategory>,

    // Target frame rate
    target_fps: f64,
    target_frame_time_ms: f64,

    // Statistics
    total_frames: u64,
    frames_over_budget: u64,
    worst_frame_time_ms: f64,
    best_frame_time_ms: f64,
    session_start: std::time::Instant,

    // Action system
    actions: Vec<PerformanceAction>,
}

impl PerformancePanel {
    pub fn new() -> Self {
        let mut panel = Self {
            widget: PerformanceBudgetWidget::new(),
            last_update: std::time::Instant::now(),
            frame_count: 0,
            runtime_stats: None,
            metrics: HashMap::new(),
            subsystems: Vec::new(),
            memory_stats: MemoryStats::default(),
            gpu_stats: GpuStats::default(),
            alerts: VecDeque::with_capacity(50),
            max_alerts: 50,
            show_subsystems: true,
            show_memory: true,
            show_gpu: false,
            show_metrics: true,
            show_alerts: true,
            show_history_graphs: false,
            selected_category: None,
            target_fps: 60.0,
            target_frame_time_ms: 16.67,
            total_frames: 0,
            frames_over_budget: 0,
            worst_frame_time_ms: 0.0,
            best_frame_time_ms: f64::MAX,
            session_start: std::time::Instant::now(),
            actions: Vec::new(),
        };
        panel.init_default_metrics();
        panel.init_default_subsystems();
        panel
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Action System
    // ─────────────────────────────────────────────────────────────────────────────

    /// Queue an action for later processing
    pub fn queue_action(&mut self, action: PerformanceAction) {
        self.actions.push(action);
    }

    /// Check if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.actions.is_empty()
    }

    /// Get pending actions without consuming them
    pub fn pending_actions(&self) -> &[PerformanceAction] {
        &self.actions
    }

    /// Take all pending actions, clearing the queue
    pub fn take_actions(&mut self) -> Vec<PerformanceAction> {
        std::mem::take(&mut self.actions)
    }

    fn init_default_metrics(&mut self) {
        // Frame metrics
        self.metrics.insert(
            "frame_time".into(),
            PerfMetric::new("Frame Time", PerfCategory::Frame, MetricUnit::Milliseconds)
                .with_budget(16.67),
        );
        self.metrics.insert(
            "fps".into(),
            PerfMetric::new("FPS", PerfCategory::Frame, MetricUnit::Fps).with_budget(60.0),
        );

        // CPU metrics
        self.metrics.insert(
            "cpu_usage".into(),
            PerfMetric::new("CPU Usage", PerfCategory::Cpu, MetricUnit::Percent).with_budget(80.0),
        );
        self.metrics.insert(
            "thread_count".into(),
            PerfMetric::new("Active Threads", PerfCategory::Cpu, MetricUnit::Count),
        );

        // GPU metrics
        self.metrics.insert(
            "gpu_frame_time".into(),
            PerfMetric::new(
                "GPU Frame Time",
                PerfCategory::Gpu,
                MetricUnit::Milliseconds,
            )
            .with_budget(10.0),
        );
        self.metrics.insert(
            "draw_calls".into(),
            PerfMetric::new("Draw Calls", PerfCategory::Gpu, MetricUnit::Count).with_budget(2000.0),
        );

        // Memory metrics
        self.metrics.insert(
            "heap_used".into(),
            PerfMetric::new("Heap Used", PerfCategory::Memory, MetricUnit::Megabytes),
        );
        self.metrics.insert(
            "gpu_memory".into(),
            PerfMetric::new("GPU Memory", PerfCategory::Memory, MetricUnit::Megabytes),
        );

        // Physics metrics
        self.metrics.insert(
            "physics_step".into(),
            PerfMetric::new(
                "Physics Step",
                PerfCategory::Physics,
                MetricUnit::Milliseconds,
            )
            .with_budget(4.0),
        );
        self.metrics.insert(
            "collision_pairs".into(),
            PerfMetric::new("Collision Pairs", PerfCategory::Physics, MetricUnit::Count),
        );

        // AI metrics
        self.metrics.insert(
            "ai_update".into(),
            PerfMetric::new("AI Update", PerfCategory::Ai, MetricUnit::Milliseconds)
                .with_budget(2.0),
        );
        self.metrics.insert(
            "active_agents".into(),
            PerfMetric::new("Active Agents", PerfCategory::Ai, MetricUnit::Count),
        );

        // Rendering metrics
        self.metrics.insert(
            "render_time".into(),
            PerfMetric::new(
                "Render Time",
                PerfCategory::Rendering,
                MetricUnit::Milliseconds,
            )
            .with_budget(8.0),
        );
        self.metrics.insert(
            "triangles".into(),
            PerfMetric::new("Triangles", PerfCategory::Rendering, MetricUnit::Count),
        );
    }

    fn init_default_subsystems(&mut self) {
        self.subsystems = vec![
            SubsystemTiming::new("Input", 0.5),
            SubsystemTiming::new("Physics", 4.0),
            SubsystemTiming::new("AI", 2.0),
            SubsystemTiming::new("Animation", 1.5),
            SubsystemTiming::new("Audio", 1.0),
            SubsystemTiming::new("Rendering", 8.0),
            SubsystemTiming::new("UI", 1.0),
            SubsystemTiming::new("Scripting", 1.0),
        ];
    }

    /// Simulate frame timing data
    fn simulate_frame_timing(&mut self) {
        let elapsed_secs = self.last_update.elapsed().as_secs_f32();
        let base_time = 4.0;
        let variance = (elapsed_secs * 2.0).sin() * 1.5;
        let total_ms = base_time + variance;

        self.widget.update_from_frame_time(total_ms);
        self.update_metrics(total_ms as f64);
        self.simulate_subsystems(total_ms as f64);
        self.frame_count += 1;
    }

    fn update_metrics(&mut self, frame_time_ms: f64) {
        // Update frame metrics
        if let Some(metric) = self.metrics.get_mut("frame_time") {
            metric.push(frame_time_ms);
        }
        if let Some(metric) = self.metrics.get_mut("fps") {
            let fps = if frame_time_ms > 0.0 {
                1000.0 / frame_time_ms
            } else {
                0.0
            };
            metric.push(fps);
        }

        // Simulate other metrics
        let variance = (self.frame_count as f64 * 0.1).sin();

        if let Some(metric) = self.metrics.get_mut("cpu_usage") {
            metric.push(25.0 + variance * 10.0);
        }
        if let Some(metric) = self.metrics.get_mut("heap_used") {
            metric.push(256.0 + variance * 20.0);
        }
        if let Some(metric) = self.metrics.get_mut("draw_calls") {
            metric.push(800.0 + variance * 100.0);
        }
        if let Some(metric) = self.metrics.get_mut("triangles") {
            metric.push(150000.0 + variance * 20000.0);
        }

        // Update statistics
        self.total_frames += 1;
        if frame_time_ms > self.target_frame_time_ms {
            self.frames_over_budget += 1;
        }
        if frame_time_ms > self.worst_frame_time_ms {
            self.worst_frame_time_ms = frame_time_ms;
        }
        if frame_time_ms < self.best_frame_time_ms {
            self.best_frame_time_ms = frame_time_ms;
        }

        // Check for alerts
        self.check_alerts(frame_time_ms);
    }

    fn simulate_subsystems(&mut self, total_ms: f64) {
        // Distribute frame time across subsystems
        let distribution = [0.03, 0.25, 0.12, 0.10, 0.05, 0.35, 0.05, 0.05];

        for (i, subsystem) in self.subsystems.iter_mut().enumerate() {
            let ratio = distribution.get(i).copied().unwrap_or(0.1);
            let time = total_ms * ratio * (1.0 + (self.frame_count as f64 * 0.05).sin() * 0.2);
            subsystem.push(time, 1);
        }
    }

    fn check_alerts(&mut self, frame_time_ms: f64) {
        // Frame time alert
        if frame_time_ms > 33.33 {
            self.add_alert(PerfAlert::new(
                AlertSeverity::Critical,
                PerfCategory::Frame,
                format!("Frame time {:.1}ms (< 30 FPS)", frame_time_ms),
            ));
        } else if frame_time_ms > 20.0 {
            self.add_alert(PerfAlert::new(
                AlertSeverity::Warning,
                PerfCategory::Frame,
                format!("Frame time {:.1}ms (< 50 FPS)", frame_time_ms),
            ));
        }

        // Check subsystem budgets - collect alerts first to avoid borrow issues
        let subsystem_alerts: Vec<PerfAlert> = self
            .subsystems
            .iter()
            .filter(|s| s.is_over_budget())
            .map(|s| {
                PerfAlert::new(
                    AlertSeverity::Warning,
                    PerfCategory::Cpu,
                    format!(
                        "{} over budget: {:.1}ms / {:.1}ms",
                        s.name, s.time_ms, s.budget_ms
                    ),
                )
            })
            .collect();

        for alert in subsystem_alerts {
            self.add_alert(alert);
        }
    }

    fn add_alert(&mut self, alert: PerfAlert) {
        // Deduplicate recent alerts
        let dominated = self.alerts.iter().any(|a| {
            a.category == alert.category && a.message == alert.message && a.age_secs() < 2.0
        });

        if !dominated {
            info!(severity = ?alert.severity, category = ?alert.category, msg = %alert.message, "perf: alert triggered");
            self.alerts.push_front(alert);
            if self.alerts.len() > self.max_alerts {
                self.alerts.pop_back();
            }
        }
    }

    /// Feed live runtime stats
    pub fn push_runtime_stats(&mut self, stats: &RuntimeStats) {
        self.runtime_stats = Some(stats.clone());

        let frame_time = if stats.frame_time_ms > 0.0 {
            stats.frame_time_ms
        } else {
            16.0
        };
        self.widget.update_from_frame_time(frame_time);
        self.update_metrics(frame_time as f64);
    }

    /// Set actual editor frame time
    pub fn set_frame_time(&mut self, frame_time_ms: f32) {
        if self.runtime_stats.is_none() {
            self.widget.update_from_frame_time(frame_time_ms);
            self.update_metrics(frame_time_ms as f64);
            self.frame_count += 1;
        }
    }

    /// Clear runtime stats
    pub fn clear_runtime_stats(&mut self) {
        self.runtime_stats = None;
    }

    /// Get session uptime
    pub fn session_uptime(&self) -> std::time::Duration {
        self.session_start.elapsed()
    }

    /// Get frame budget hit rate
    pub fn budget_hit_rate(&self) -> f64 {
        if self.total_frames == 0 {
            return 100.0;
        }
        ((self.total_frames - self.frames_over_budget) as f64 / self.total_frames as f64) * 100.0
    }

    fn show_summary(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // FPS
            if let Some(metric) = self.metrics.get("fps") {
                let fps = metric.value;
                let color = if fps >= 60.0 {
                    Color32::from_rgb(100, 255, 100)
                } else if fps >= 30.0 {
                    Color32::YELLOW
                } else {
                    Color32::RED
                };
                ui.label(
                    RichText::new(format!("{:.0} FPS", fps))
                        .color(color)
                        .strong(),
                );
            }

            ui.separator();

            // Frame time
            if let Some(metric) = self.metrics.get("frame_time") {
                let ft = metric.value;
                let color = if ft <= 16.67 {
                    Color32::from_rgb(100, 255, 100)
                } else if ft <= 33.33 {
                    Color32::YELLOW
                } else {
                    Color32::RED
                };
                ui.label(RichText::new(format!("[Time] {:.2} ms", ft)).color(color));
            }

            ui.separator();

            // Budget hit rate
            let rate = self.budget_hit_rate();
            let color = if rate >= 99.0 {
                Color32::from_rgb(100, 255, 100)
            } else if rate >= 90.0 {
                Color32::YELLOW
            } else {
                Color32::RED
            };
            ui.label(RichText::new(format!("{:.1}% on budget", rate)).color(color));
        });
    }

    fn show_subsystems_section(&mut self, ui: &mut Ui) {
        ui.collapsing("Subsystem Breakdown", |ui| {
            let total_time: f64 = self.subsystems.iter().map(|s| s.time_ms).sum();

            for subsystem in &self.subsystems {
                let percent = if total_time > 0.0 {
                    (subsystem.time_ms / total_time) * 100.0
                } else {
                    0.0
                };

                let budget_pct = subsystem.budget_percent();
                let color = if budget_pct > 100.0 {
                    Color32::RED
                } else if budget_pct > 80.0 {
                    Color32::YELLOW
                } else {
                    Color32::from_rgb(100, 200, 100)
                };

                ui.horizontal(|ui| {
                    ui.label(format!("{:12}", subsystem.name));

                    // Progress bar
                    let bar_width = 100.0;
                    let filled = (budget_pct / 100.0).min(1.0) as f32 * bar_width;

                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(bar_width, 14.0), egui::Sense::hover());
                    let painter = ui.painter();

                    painter.rect_filled(rect, 2.0, Color32::from_gray(60));
                    let filled_rect =
                        egui::Rect::from_min_size(rect.min, egui::vec2(filled, rect.height()));
                    painter.rect_filled(filled_rect, 2.0, color);

                    ui.label(RichText::new(format!("{:.2} ms", subsystem.time_ms)).color(color));
                    ui.label(RichText::new(format!("({:.0}%)", percent)).weak());
                });
            }

            ui.separator();
            ui.label(format!("Total: {:.2} ms", total_time));
        });
    }

    fn show_memory_section(&self, ui: &mut Ui) {
        ui.collapsing("Memory", |ui| {
            ui.horizontal(|ui| {
                ui.label("Heap:");
                ui.label(format!(
                    "{:.1} MB / {:.1} MB ({:.0}%)",
                    self.memory_stats.heap_used_mb,
                    self.memory_stats.heap_committed_mb,
                    self.memory_stats.heap_usage_percent()
                ));
            });

            ui.horizontal(|ui| {
                ui.label("GPU:");
                ui.label(format!(
                    "{:.1} MB / {:.1} MB ({:.0}%)",
                    self.memory_stats.gpu_used_mb,
                    self.memory_stats.gpu_available_mb,
                    self.memory_stats.gpu_usage_percent()
                ));
            });

            ui.horizontal(|ui| {
                ui.label("Peak:");
                ui.label(format!("{:.1} MB", self.memory_stats.peak_usage_mb));
            });

            ui.horizontal(|ui| {
                ui.label("Allocs/frame:");
                ui.label(format!("{}", self.memory_stats.allocations_per_frame));
            });
        });
    }

    fn show_gpu_section(&self, ui: &mut Ui) {
        ui.collapsing("GPU Stats", |ui| {
            ui.label(format!(
                "Frame Time: {:.2} ms",
                self.gpu_stats.frame_time_ms
            ));
            ui.label(format!("Draw Calls: {}", self.gpu_stats.draw_calls));
            ui.label(format!("Triangles: {}", self.gpu_stats.triangle_count));
            ui.label(format!("Vertices: {}", self.gpu_stats.vertex_count));
            ui.label(format!(
                "Shader Switches: {}",
                self.gpu_stats.shader_switches
            ));
            ui.label(format!(
                "Texture Bindings: {}",
                self.gpu_stats.texture_bindings
            ));
            ui.label(format!(
                "Buffer Uploads: {:.1} KB",
                self.gpu_stats.buffer_uploads_kb
            ));
            ui.label(format!("Render Targets: {}", self.gpu_stats.render_targets));
            ui.label(format!(
                "Compute Dispatches: {}",
                self.gpu_stats.compute_dispatches
            ));
        });
    }

    fn show_alerts_section(&self, ui: &mut Ui) {
        if self.alerts.is_empty() {
            return;
        }

        ui.collapsing(format!("🔔 Alerts ({})", self.alerts.len()), |ui| {
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    for alert in self.alerts.iter().take(20) {
                        let age = alert.age_secs();
                        let alpha = if age < 5.0 {
                            255
                        } else {
                            (255.0 * (1.0 - (age - 5.0) / 10.0).max(0.0)) as u8
                        };

                        let color = Color32::from_rgba_unmultiplied(
                            alert.severity.color().r(),
                            alert.severity.color().g(),
                            alert.severity.color().b(),
                            alpha,
                        );

                        ui.horizontal(|ui| {
                            ui.label(RichText::new(alert.severity.icon()).color(color));
                            ui.label(RichText::new(&alert.message).color(color).small());
                            ui.label(RichText::new(format!("{:.0}s ago", age)).weak().small());
                        });
                    }
                });
        });
    }

    fn show_session_stats(&self, ui: &mut Ui) {
        ui.collapsing("📈 Session Statistics", |ui| {
            let uptime = self.session_uptime();
            ui.label(format!(
                "Session: {:02}:{:02}:{:02}",
                uptime.as_secs() / 3600,
                (uptime.as_secs() % 3600) / 60,
                uptime.as_secs() % 60
            ));
            ui.label(format!("Total Frames: {}", self.total_frames));
            ui.label(format!(
                "Over Budget: {} ({:.1}%)",
                self.frames_over_budget,
                if self.total_frames > 0 {
                    (self.frames_over_budget as f64 / self.total_frames as f64) * 100.0
                } else {
                    0.0
                }
            ));
            ui.label(format!(
                "Best Frame: {:.2} ms",
                if self.best_frame_time_ms < f64::MAX {
                    self.best_frame_time_ms
                } else {
                    0.0
                }
            ));
            ui.label(format!("Worst Frame: {:.2} ms", self.worst_frame_time_ms));

            if let Some(metric) = self.metrics.get("frame_time") {
                ui.label(format!("Avg Frame: {:.2} ms", metric.average()));
            }
        });
    }
}

impl Default for PerformancePanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for PerformancePanel {
    fn name(&self) -> &str {
        "Performance"
    }

    fn update(&mut self) {
        let now = std::time::Instant::now();
        if now.duration_since(self.last_update).as_millis() >= 16 {
            if self.runtime_stats.is_some() {
                self.frame_count += 1;
            } else {
                self.simulate_frame_timing();
            }
            self.last_update = now;
        }
    }

    fn show(&mut self, ui: &mut Ui) {
        ui.heading("Performance Monitor");
        ui.separator();

        // Summary bar
        self.show_summary(ui);

        ui.separator();

        // Display options toolbar
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_subsystems, "Subsystems");
            ui.checkbox(&mut self.show_memory, "Memory");
            ui.checkbox(&mut self.show_gpu, "GPU");
            ui.checkbox(&mut self.show_alerts, "Alerts");
        });

        ui.separator();

        // Main widget
        self.widget.show(ui);

        ui.add_space(10.0);

        // Runtime stats (if available)
        if let Some(stats) = &self.runtime_stats {
            ui.group(|ui| {
                ui.label(RichText::new("Runtime Metrics").strong());
                ui.label(format!("Frame Time: {:.2} ms", stats.frame_time_ms));
                ui.label(format!("FPS: {:.0}", stats.fps));
                ui.label(format!("Entities: {}", stats.entity_count));
                ui.label(format!("Tick #: {}", stats.tick_count));
                ui.label(format!("Fixed Steps: {}", stats.fixed_steps_last_tick));

                let color = if stats.frame_time_ms > 20.0 {
                    Color32::RED
                } else if stats.frame_time_ms > 16.7 {
                    Color32::YELLOW
                } else {
                    Color32::from_rgb(120, 220, 150)
                };

                let msg = if stats.frame_time_ms > 20.0 {
                    "Over budget (>20ms)"
                } else if stats.frame_time_ms > 16.7 {
                    "Near budget (16.7-20ms)"
                } else {
                    "Within 60 FPS budget"
                };

                ui.colored_label(color, msg);
            });
        }

        ui.add_space(5.0);

        // Subsystem breakdown
        if self.show_subsystems {
            self.show_subsystems_section(ui);
        }

        // Memory section
        if self.show_memory {
            self.show_memory_section(ui);
        }

        // GPU section
        if self.show_gpu {
            self.show_gpu_section(ui);
        }

        // Alerts
        if self.show_alerts {
            self.show_alerts_section(ui);
        }

        // Session stats
        self.show_session_stats(ui);

        ui.add_space(10.0);

        // Controls
        ui.horizontal(|ui| {
            if ui.button("Reset").clicked() {
                self.widget = PerformanceBudgetWidget::new();
                self.frame_count = 0;
                self.runtime_stats = None;
                self.alerts.clear();
                self.total_frames = 0;
                self.frames_over_budget = 0;
                self.worst_frame_time_ms = 0.0;
                self.best_frame_time_ms = f64::MAX;
                self.session_start = std::time::Instant::now();
                self.init_default_metrics();
                self.init_default_subsystems();
            }

            if ui.button("🧹 Clear Alerts").clicked() {
                self.alerts.clear();
            }
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // PERF CATEGORY TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_perf_category_name() {
        assert_eq!(PerfCategory::Frame.name(), "Frame");
        assert_eq!(PerfCategory::Cpu.name(), "CPU");
        assert_eq!(PerfCategory::Gpu.name(), "GPU");
        assert_eq!(PerfCategory::Memory.name(), "Memory");
        assert_eq!(PerfCategory::Physics.name(), "Physics");
        assert_eq!(PerfCategory::Ai.name(), "AI");
    }

    #[test]
    fn test_perf_category_icon() {
        assert!(!PerfCategory::Frame.icon().is_empty());
        assert!(!PerfCategory::Cpu.icon().is_empty());
        assert!(!PerfCategory::Gpu.icon().is_empty());
    }

    #[test]
    fn test_perf_category_color() {
        let color = PerfCategory::Frame.color();
        assert!(color.r() > 0 || color.g() > 0 || color.b() > 0);
    }

    #[test]
    fn test_perf_category_all() {
        let all = PerfCategory::all();
        assert_eq!(all.len(), 10);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PERF METRIC TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_perf_metric_creation() {
        let metric = PerfMetric::new("Test", PerfCategory::Frame, MetricUnit::Milliseconds);
        assert_eq!(metric.name, "Test");
        assert_eq!(metric.category, PerfCategory::Frame);
        assert_eq!(metric.value, 0.0);
        assert!(metric.budget.is_none());
    }

    #[test]
    fn test_perf_metric_with_budget() {
        let metric = PerfMetric::new("Test", PerfCategory::Frame, MetricUnit::Milliseconds)
            .with_budget(16.67);
        assert_eq!(metric.budget, Some(16.67));
    }

    #[test]
    fn test_perf_metric_push() {
        let mut metric = PerfMetric::new("Test", PerfCategory::Frame, MetricUnit::Milliseconds);
        metric.push(10.0);
        metric.push(20.0);
        metric.push(15.0);

        assert_eq!(metric.value, 15.0);
        assert_eq!(metric.history.len(), 3);
    }

    #[test]
    fn test_perf_metric_average() {
        let mut metric = PerfMetric::new("Test", PerfCategory::Frame, MetricUnit::Milliseconds);
        metric.push(10.0);
        metric.push(20.0);
        metric.push(30.0);

        assert_eq!(metric.average(), 20.0);
    }

    #[test]
    fn test_perf_metric_min_max() {
        let mut metric = PerfMetric::new("Test", PerfCategory::Frame, MetricUnit::Milliseconds);
        metric.push(10.0);
        metric.push(30.0);
        metric.push(20.0);

        assert_eq!(metric.min(), 10.0);
        assert_eq!(metric.max(), 30.0);
    }

    #[test]
    fn test_perf_metric_over_budget() {
        let mut metric = PerfMetric::new("Test", PerfCategory::Frame, MetricUnit::Milliseconds)
            .with_budget(16.67);

        metric.push(10.0);
        assert!(!metric.is_over_budget());

        metric.push(20.0);
        assert!(metric.is_over_budget());
    }

    #[test]
    fn test_perf_metric_budget_percent() {
        let mut metric = PerfMetric::new("Test", PerfCategory::Frame, MetricUnit::Milliseconds)
            .with_budget(20.0);
        metric.push(10.0);

        assert_eq!(metric.budget_percent(), Some(50.0));
    }

    #[test]
    fn test_perf_metric_history_limit() {
        let mut metric = PerfMetric::new("Test", PerfCategory::Frame, MetricUnit::Milliseconds);

        for i in 0..150 {
            metric.push(i as f64);
        }

        assert_eq!(metric.history.len(), 120);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // METRIC UNIT TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_metric_unit_suffix() {
        assert_eq!(MetricUnit::Milliseconds.suffix(), "ms");
        assert_eq!(MetricUnit::Microseconds.suffix(), "µs");
        assert_eq!(MetricUnit::Percent.suffix(), "%");
        assert_eq!(MetricUnit::Megabytes.suffix(), "MB");
        assert_eq!(MetricUnit::Fps.suffix(), "FPS");
    }

    #[test]
    fn test_metric_unit_format() {
        assert_eq!(MetricUnit::Milliseconds.format(16.67), "16.67 ms");
        assert_eq!(MetricUnit::Fps.format(60.0), "60FPS");
        assert_eq!(MetricUnit::Percent.format(75.5), "75.5%");
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SUBSYSTEM TIMING TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_subsystem_timing_creation() {
        let timing = SubsystemTiming::new("Physics", 4.0);
        assert_eq!(timing.name, "Physics");
        assert_eq!(timing.budget_ms, 4.0);
        assert_eq!(timing.time_ms, 0.0);
    }

    #[test]
    fn test_subsystem_timing_push() {
        let mut timing = SubsystemTiming::new("Physics", 4.0);
        timing.push(3.5, 1);

        assert_eq!(timing.time_ms, 3.5);
        assert_eq!(timing.call_count, 1);
        assert_eq!(timing.history.len(), 1);
    }

    #[test]
    fn test_subsystem_timing_average() {
        let mut timing = SubsystemTiming::new("Physics", 4.0);
        timing.push(2.0, 1);
        timing.push(4.0, 1);
        timing.push(6.0, 1);

        assert_eq!(timing.average(), 4.0);
    }

    #[test]
    fn test_subsystem_timing_over_budget() {
        let mut timing = SubsystemTiming::new("Physics", 4.0);

        timing.push(3.0, 1);
        assert!(!timing.is_over_budget());

        timing.push(5.0, 1);
        assert!(timing.is_over_budget());
    }

    #[test]
    fn test_subsystem_timing_budget_percent() {
        let mut timing = SubsystemTiming::new("Physics", 4.0);
        timing.push(2.0, 1);

        assert_eq!(timing.budget_percent(), 50.0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // MEMORY STATS TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_memory_stats_default() {
        let stats = MemoryStats::default();
        assert_eq!(stats.heap_used_mb, 0.0);
        assert_eq!(stats.gpu_used_mb, 0.0);
    }

    #[test]
    fn test_memory_stats_heap_usage_percent() {
        let stats = MemoryStats {
            heap_used_mb: 256.0,
            heap_committed_mb: 512.0,
            ..Default::default()
        };
        assert_eq!(stats.heap_usage_percent(), 50.0);
    }

    #[test]
    fn test_memory_stats_gpu_usage_percent() {
        let stats = MemoryStats {
            gpu_used_mb: 1024.0,
            gpu_available_mb: 4096.0,
            ..Default::default()
        };
        assert_eq!(stats.gpu_usage_percent(), 25.0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // GPU STATS TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_gpu_stats_default() {
        let stats = GpuStats::default();
        assert_eq!(stats.frame_time_ms, 0.0);
        assert_eq!(stats.draw_calls, 0);
        assert_eq!(stats.triangle_count, 0);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ALERT TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_alert_severity_color() {
        assert_ne!(AlertSeverity::Info.color(), AlertSeverity::Critical.color());
    }

    #[test]
    fn test_alert_severity_icon() {
        assert!(!AlertSeverity::Warning.icon().is_empty());
        assert!(!AlertSeverity::Critical.icon().is_empty());
    }

    #[test]
    fn test_perf_alert_creation() {
        let alert = PerfAlert::new(AlertSeverity::Warning, PerfCategory::Frame, "Test alert");
        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert_eq!(alert.category, PerfCategory::Frame);
        assert_eq!(alert.message, "Test alert");
    }

    #[test]
    fn test_perf_alert_age() {
        let alert = PerfAlert::new(AlertSeverity::Info, PerfCategory::Cpu, "Test");
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(alert.age_secs() >= 0.01);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PANEL TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_performance_panel_creation() {
        let panel = PerformancePanel::new();
        assert_eq!(panel.frame_count, 0);
        assert!(panel.runtime_stats.is_none());
        assert!(panel.show_subsystems);
        assert_eq!(panel.target_fps, 60.0);
    }

    #[test]
    fn test_performance_panel_default() {
        let panel: PerformancePanel = Default::default();
        assert_eq!(panel.target_fps, 60.0);
    }

    #[test]
    fn test_performance_panel_metrics_initialized() {
        let panel = PerformancePanel::new();
        assert!(panel.metrics.contains_key("frame_time"));
        assert!(panel.metrics.contains_key("fps"));
        assert!(panel.metrics.contains_key("cpu_usage"));
        assert!(panel.metrics.contains_key("draw_calls"));
    }

    #[test]
    fn test_performance_panel_subsystems_initialized() {
        let panel = PerformancePanel::new();
        assert!(!panel.subsystems.is_empty());

        let names: Vec<&str> = panel.subsystems.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Physics"));
        assert!(names.contains(&"Rendering"));
        assert!(names.contains(&"AI"));
    }

    #[test]
    fn test_performance_panel_set_frame_time() {
        let mut panel = PerformancePanel::new();
        panel.set_frame_time(16.67);

        assert_eq!(panel.frame_count, 1);
    }

    #[test]
    fn test_performance_panel_budget_hit_rate() {
        let mut panel = PerformancePanel::new();

        // All frames within budget
        for _ in 0..10 {
            panel.set_frame_time(10.0);
        }

        assert_eq!(panel.budget_hit_rate(), 100.0);
    }

    #[test]
    fn test_performance_panel_budget_hit_rate_with_misses() {
        let mut panel = PerformancePanel::new();

        // 5 good frames
        for _ in 0..5 {
            panel.set_frame_time(10.0);
        }
        // 5 bad frames
        for _ in 0..5 {
            panel.set_frame_time(20.0);
        }

        assert_eq!(panel.budget_hit_rate(), 50.0);
    }

    #[test]
    fn test_performance_panel_session_uptime() {
        let panel = PerformancePanel::new();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let uptime = panel.session_uptime();
        assert!(uptime.as_millis() >= 10);
    }

    #[test]
    fn test_performance_panel_clear_runtime_stats() {
        let mut panel = PerformancePanel::new();
        panel.runtime_stats = Some(RuntimeStats::default());

        panel.clear_runtime_stats();
        assert!(panel.runtime_stats.is_none());
    }

    #[test]
    fn test_performance_panel_worst_best_frame() {
        let mut panel = PerformancePanel::new();

        panel.set_frame_time(10.0);
        panel.set_frame_time(20.0);
        panel.set_frame_time(5.0);

        assert_eq!(panel.best_frame_time_ms, 5.0);
        assert_eq!(panel.worst_frame_time_ms, 20.0);
    }

    #[test]
    fn test_performance_panel_name() {
        let panel = PerformancePanel::new();
        assert_eq!(panel.name(), "Performance");
    }

    #[test]
    fn test_performance_panel_alerts() {
        let mut panel = PerformancePanel::new();

        // Trigger a critical alert by simulating a very slow frame
        panel.set_frame_time(50.0);

        // Should have at least one alert
        assert!(!panel.alerts.is_empty());
    }

    #[test]
    fn test_performance_panel_alert_deduplication() {
        let mut panel = PerformancePanel::new();

        // Same alert shouldn't be added twice within 2 seconds
        panel.add_alert(PerfAlert::new(
            AlertSeverity::Warning,
            PerfCategory::Frame,
            "Test",
        ));
        panel.add_alert(PerfAlert::new(
            AlertSeverity::Warning,
            PerfCategory::Frame,
            "Test",
        ));

        assert_eq!(panel.alerts.len(), 1);
    }

    // ========== PerfCategory Display Tests ==========

    #[test]
    fn test_perf_category_display() {
        for cat in PerfCategory::all() {
            let display = format!("{}", cat);
            assert!(display.contains(cat.name()));
        }
    }

    #[test]
    fn test_perf_category_hash() {
        use std::collections::HashSet;
        let set: HashSet<PerfCategory> = PerfCategory::all().iter().copied().collect();
        assert_eq!(set.len(), 10);
    }

    // ========== MetricUnit Tests ==========

    #[test]
    fn test_metric_unit_display() {
        for unit in MetricUnit::all() {
            let display = format!("{}", unit);
            assert_eq!(display, unit.suffix());
        }
    }

    #[test]
    fn test_metric_unit_all_variants() {
        let all = MetricUnit::all();
        assert_eq!(all.len(), 11);
    }

    #[test]
    fn test_metric_unit_hash() {
        use std::collections::HashSet;
        let set: HashSet<MetricUnit> = MetricUnit::all().iter().copied().collect();
        assert_eq!(set.len(), 11);
    }

    #[test]
    fn test_metric_unit_is_time_unit() {
        assert!(MetricUnit::Milliseconds.is_time_unit());
        assert!(MetricUnit::Microseconds.is_time_unit());
        assert!(MetricUnit::Nanoseconds.is_time_unit());
        assert!(!MetricUnit::Percent.is_time_unit());
        assert!(!MetricUnit::Megabytes.is_time_unit());
        assert!(!MetricUnit::Fps.is_time_unit());
    }

    #[test]
    fn test_metric_unit_is_memory_unit() {
        assert!(MetricUnit::Bytes.is_memory_unit());
        assert!(MetricUnit::Kilobytes.is_memory_unit());
        assert!(MetricUnit::Megabytes.is_memory_unit());
        assert!(MetricUnit::Gigabytes.is_memory_unit());
        assert!(!MetricUnit::Milliseconds.is_memory_unit());
        assert!(!MetricUnit::Percent.is_memory_unit());
    }

    #[test]
    fn test_metric_unit_name() {
        assert_eq!(MetricUnit::Milliseconds.name(), "Milliseconds");
        assert_eq!(MetricUnit::Fps.name(), "FPS");
        assert_eq!(MetricUnit::PerSecond.name(), "Per Second");
    }

    // ========== AlertSeverity Tests ==========

    #[test]
    fn test_alert_severity_display() {
        for severity in AlertSeverity::all() {
            let display = format!("{}", severity);
            assert!(display.contains(severity.name()));
        }
    }

    #[test]
    fn test_alert_severity_all_variants() {
        let all = AlertSeverity::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&AlertSeverity::Info));
        assert!(all.contains(&AlertSeverity::Warning));
        assert!(all.contains(&AlertSeverity::Critical));
    }

    #[test]
    fn test_alert_severity_hash() {
        use std::collections::HashSet;
        let set: HashSet<AlertSeverity> = AlertSeverity::all().iter().copied().collect();
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_alert_severity_is_serious() {
        assert!(!AlertSeverity::Info.is_serious());
        assert!(AlertSeverity::Warning.is_serious());
        assert!(AlertSeverity::Critical.is_serious());
    }

    #[test]
    fn test_alert_severity_name() {
        assert_eq!(AlertSeverity::Info.name(), "Info");
        assert_eq!(AlertSeverity::Warning.name(), "Warning");
        assert_eq!(AlertSeverity::Critical.name(), "Critical");
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // PerformanceAction Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_performance_action_display() {
        let action = PerformanceAction::SelectCategory(Some(PerfCategory::Rendering));
        let display = format!("{}", action);
        assert!(display.contains("Rendering"));
    }

    #[test]
    fn test_performance_action_display_all_variants() {
        let actions = vec![
            PerformanceAction::ToggleSubsystems(true),
            PerformanceAction::ToggleMemory(true),
            PerformanceAction::ToggleGpu(true),
            PerformanceAction::SelectCategory(Some(PerfCategory::Physics)),
            PerformanceAction::SetTargetFps(60.0),
            PerformanceAction::ResetStatistics,
            PerformanceAction::StartRecording,
            PerformanceAction::SaveRecording("trace.json".to_string()),
        ];

        for action in actions {
            let display = format!("{}", action);
            assert!(
                !display.is_empty(),
                "Display should not be empty for {:?}",
                action
            );
        }
    }

    #[test]
    fn test_performance_action_equality() {
        let action1 = PerformanceAction::SelectCategory(Some(PerfCategory::Audio));
        let action2 = PerformanceAction::SelectCategory(Some(PerfCategory::Audio));
        let action3 = PerformanceAction::SelectCategory(Some(PerfCategory::Ai));

        assert_eq!(action1, action2);
        assert_ne!(action1, action3);
    }

    #[test]
    fn test_performance_action_clone() {
        let action = PerformanceAction::SaveRecording("output.trace".to_string());
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }

    #[test]
    fn test_performance_panel_pending_actions_empty_by_default() {
        let panel = PerformancePanel::new();
        assert!(!panel.has_pending_actions());
        assert!(panel.pending_actions().is_empty());
    }

    #[test]
    fn test_performance_panel_queue_action() {
        let mut panel = PerformancePanel::new();
        panel.queue_action(PerformanceAction::ResetStatistics);
        assert!(panel.has_pending_actions());
        assert_eq!(panel.pending_actions().len(), 1);
    }

    #[test]
    fn test_performance_panel_take_actions() {
        let mut panel = PerformancePanel::new();
        panel.queue_action(PerformanceAction::ResetStatistics);
        panel.queue_action(PerformanceAction::ToggleGpu(true));

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());
        assert!(panel.pending_actions().is_empty());
    }

    #[test]
    fn test_performance_panel_action_order_preserved() {
        let mut panel = PerformancePanel::new();
        panel.queue_action(PerformanceAction::StartRecording);
        panel.queue_action(PerformanceAction::SelectCategory(Some(
            PerfCategory::Network,
        )));
        panel.queue_action(PerformanceAction::StopRecording);

        let actions = panel.take_actions();
        assert!(matches!(actions[0], PerformanceAction::StartRecording));
        assert!(matches!(actions[1], PerformanceAction::SelectCategory(_)));
        assert!(matches!(actions[2], PerformanceAction::StopRecording));
    }

    #[test]
    fn test_performance_action_display_toggles() {
        let actions = vec![
            PerformanceAction::ToggleSubsystems(true),
            PerformanceAction::ToggleMemory(false),
            PerformanceAction::ToggleGpu(true),
            PerformanceAction::ToggleMetrics(false),
            PerformanceAction::ToggleAlerts(true),
            PerformanceAction::ToggleHistoryGraphs(false),
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[0].contains("true"));
        assert!(displays[1].contains("false"));
    }

    #[test]
    fn test_performance_action_metrics_operations() {
        let actions = vec![
            PerformanceAction::ResetStatistics,
            PerformanceAction::ClearAlerts,
            PerformanceAction::ExportReport,
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[0].contains("Reset"));
        assert!(displays[1].contains("Clear"));
        assert!(displays[2].contains("report"));
    }

    #[test]
    fn test_performance_action_subsystem_control() {
        let actions = vec![
            PerformanceAction::ToggleSubsystemProfiling("Rendering".to_string(), true),
            PerformanceAction::ToggleSubsystemProfiling("Physics".to_string(), false),
            PerformanceAction::SetSubsystemBudget("AI".to_string(), 5.0),
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[0].contains("Rendering"));
        assert!(displays[2].contains("5"));
    }

    #[test]
    fn test_performance_action_memory_operations() {
        let actions = vec![
            PerformanceAction::TriggerGarbageCollection,
            PerformanceAction::SnapshotMemory,
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[0].contains("GC"));
        assert!(displays[1].contains("memory"));
    }

    #[test]
    fn test_performance_action_recording() {
        let actions = vec![
            PerformanceAction::StartRecording,
            PerformanceAction::StopRecording,
            PerformanceAction::SaveRecording("trace_output.json".to_string()),
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[0].contains("Start"));
        assert!(displays[1].contains("Stop"));
        assert!(displays[2].contains("trace_output"));
    }
}
