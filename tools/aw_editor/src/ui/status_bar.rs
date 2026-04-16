use crate::command::UndoStack;
use crate::editor_mode::EditorMode;
use crate::entity_manager::SelectionSet;
use crate::gizmo::snapping::SnappingConfig;
use crate::gizmo::state::GizmoMode;
use crate::ui::progress::ProgressManager;
use egui::{Align, Layout, Ui};
use std::time::Instant;

/// System resource usage information
#[derive(Clone, Copy, Debug, Default)]
pub struct ResourceUsage {
    /// Memory used in bytes
    pub memory_used: u64,
    /// Total available memory in bytes
    pub memory_total: u64,
    /// GPU memory used in bytes
    pub gpu_memory_used: u64,
    /// GPU memory total in bytes
    pub gpu_memory_total: u64,
    /// GPU utilization percentage (0-100)
    pub gpu_utilization: f32,
}

impl ResourceUsage {
    /// Create new resource usage with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Memory usage as a percentage (0-100)
    pub fn memory_percent(&self) -> f32 {
        if self.memory_total > 0 {
            (self.memory_used as f64 / self.memory_total as f64 * 100.0) as f32
        } else {
            0.0
        }
    }

    /// GPU memory usage as a percentage (0-100)
    pub fn gpu_memory_percent(&self) -> f32 {
        if self.gpu_memory_total > 0 {
            (self.gpu_memory_used as f64 / self.gpu_memory_total as f64 * 100.0) as f32
        } else {
            0.0
        }
    }

    /// Format bytes to human-readable string
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}

/// Background task summary for compact display
#[derive(Clone, Debug)]
pub struct BackgroundTaskSummary {
    /// Number of active tasks
    pub active_count: usize,
    /// Total progress across all tasks (0.0-1.0)
    pub overall_progress: f32,
    /// Name of the primary task (first/most recent)
    pub primary_task: Option<String>,
    /// Time when the first task started
    pub started_at: Option<Instant>,
}

impl BackgroundTaskSummary {
    /// Create empty summary
    pub fn empty() -> Self {
        Self {
            active_count: 0,
            overall_progress: 0.0,
            primary_task: None,
            started_at: None,
        }
    }

    /// Check if there are active tasks
    pub fn is_active(&self) -> bool {
        self.active_count > 0
    }
}

/// Status bar component for the bottom of the editor
///
/// Shows:
/// - Editor mode (Edit/Play/Paused)
/// - Current gizmo mode (Translate/Rotate/Scale)
/// - Selection count
/// - Undo/redo state
/// - FPS counter
/// - Snap settings
/// - Active progress tasks (Week 6)
/// - Memory/GPU usage indicators (Week 6 Day 5)
pub struct StatusBar;

impl StatusBar {
    /// Render the enhanced status bar with progress and resource usage (Week 6 Day 5)
    #[allow(clippy::too_many_arguments)]
    pub fn show_enhanced(
        ui: &mut Ui,
        editor_mode: &EditorMode,
        gizmo_mode: &GizmoMode,
        selection: &SelectionSet,
        undo_stack: &UndoStack,
        snap_config: &SnappingConfig,
        fps: f32,
        is_dirty: bool,
        entity_count: usize,
        scene_path: Option<&str>,
        progress_manager: &mut ProgressManager,
        resource_usage: &ResourceUsage,
    ) {
        // First show compact progress bar if there are active tasks
        if progress_manager.has_active_tasks() {
            progress_manager.show_compact(ui);
            ui.add_space(2.0);
        }

        // Then show the main status bar with resource indicators
        Self::show_main(
            ui,
            editor_mode,
            gizmo_mode,
            selection,
            undo_stack,
            snap_config,
            fps,
            is_dirty,
            entity_count,
            scene_path,
            resource_usage,
        );
    }

    /// Render the status bar with progress support (Week 6)
    #[allow(clippy::too_many_arguments)]
    pub fn show_with_progress(
        ui: &mut Ui,
        editor_mode: &EditorMode,
        gizmo_mode: &GizmoMode,
        selection: &SelectionSet,
        undo_stack: &UndoStack,
        snap_config: &SnappingConfig,
        fps: f32,
        is_dirty: bool,
        entity_count: usize,
        scene_path: Option<&str>,
        progress_manager: &mut ProgressManager,
    ) {
        // First show compact progress bar if there are active tasks
        if progress_manager.has_active_tasks() {
            progress_manager.show_compact(ui);
            ui.add_space(2.0);
        }

        // Then show the regular status bar
        Self::show(
            ui,
            editor_mode,
            gizmo_mode,
            selection,
            undo_stack,
            snap_config,
            fps,
            is_dirty,
            entity_count,
            scene_path,
        );
    }

    /// Internal method with resource usage display
    #[allow(clippy::too_many_arguments)]
    fn show_main(
        ui: &mut Ui,
        editor_mode: &EditorMode,
        gizmo_mode: &GizmoMode,
        selection: &SelectionSet,
        undo_stack: &UndoStack,
        snap_config: &SnappingConfig,
        fps: f32,
        is_dirty: bool,
        entity_count: usize,
        scene_path: Option<&str>,
        resource_usage: &ResourceUsage,
    ) {
        ui.horizontal(|ui| {
            if is_dirty {
                ui.label(
                    egui::RichText::new("*")
                        .color(egui::Color32::from_rgb(255, 100, 100))
                        .strong(),
                )
                .on_hover_text("Unsaved changes - Press Ctrl+S to save");
            }

            Self::show_editor_mode(ui, editor_mode);
            ui.separator();

            Self::show_gizmo_mode(ui, gizmo_mode);
            ui.separator();

            Self::show_selection(ui, selection);
            ui.separator();

            Self::show_undo_redo(ui, undo_stack);
            ui.separator();

            ui.label(format!("Entities: {}", entity_count))
                .on_hover_text(
                    "ECS entities in the active scene.\n\n\
                     Note: procedurally-scattered vegetation and terrain \
                     chunks are rendered through a separate GPU pipeline \
                     and are NOT counted here. This is expected — \
                     they're not selectable as scene entities.",
                );
            ui.separator();

            if let Some(path) = scene_path {
                ui.label(path)
                    .on_hover_text(format!("Scene file: {}", path));
            } else {
                ui.label("Untitled")
                    .on_hover_text("Scene not saved - Press Ctrl+S to save");
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                Self::show_fps(ui, fps);
                ui.separator();

                // Show resource usage indicators (Week 6 Day 5)
                Self::show_resource_usage(ui, resource_usage);
                ui.separator();

                Self::show_snap_settings(ui, snap_config);
            });
        });
    }

    /// Render the status bar
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ui: &mut Ui,
        editor_mode: &EditorMode,
        gizmo_mode: &GizmoMode,
        selection: &SelectionSet,
        undo_stack: &UndoStack,
        snap_config: &SnappingConfig,
        fps: f32,
        is_dirty: bool,
        entity_count: usize,
        scene_path: Option<&str>,
    ) {
        ui.horizontal(|ui| {
            if is_dirty {
                ui.label(
                    egui::RichText::new("*")
                        .color(egui::Color32::from_rgb(255, 100, 100))
                        .strong(),
                )
                .on_hover_text("Unsaved changes - Press Ctrl+S to save");
            }

            Self::show_editor_mode(ui, editor_mode);
            ui.separator();

            Self::show_gizmo_mode(ui, gizmo_mode);
            ui.separator();

            Self::show_selection(ui, selection);
            ui.separator();

            Self::show_undo_redo(ui, undo_stack);
            ui.separator();

            ui.label(format!("Entities: {}", entity_count))
                .on_hover_text(
                    "ECS entities in the active scene.\n\n\
                     Note: procedurally-scattered vegetation and terrain \
                     chunks are rendered through a separate GPU pipeline \
                     and are NOT counted here. This is expected — \
                     they're not selectable as scene entities.",
                );
            ui.separator();

            if let Some(path) = scene_path {
                ui.label(path)
                    .on_hover_text(format!("Scene file: {}", path));
            } else {
                ui.label("Untitled")
                    .on_hover_text("Scene not saved - Press Ctrl+S to save");
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                Self::show_fps(ui, fps);
                ui.separator();

                Self::show_snap_settings(ui, snap_config);
            });
        });
    }

    fn show_editor_mode(ui: &mut Ui, mode: &EditorMode) {
        let status_label = egui::RichText::new(mode.status_text())
            .color(mode.status_color())
            .strong();

        let hotkey_text = match mode {
            EditorMode::Edit => "Press F5 to Play",
            EditorMode::Play => "F6 to Pause, F7 to Stop",
            EditorMode::Paused => "F5 to Resume, F7 to Stop",
        };

        ui.label(status_label).on_hover_text(hotkey_text);
    }

    fn show_gizmo_mode(ui: &mut Ui, mode: &GizmoMode) {
        let (text, hotkey) = match mode {
            GizmoMode::Inactive => ("Inactive", "ESC"),
            GizmoMode::Translate { .. } => ("Translate", "G"),
            GizmoMode::Rotate { .. } => ("Rotate", "R"),
            GizmoMode::Scale { .. } => ("Scale", "S"),
        };

        ui.label(format!("{} ({})", text, hotkey))
            .on_hover_text(format!("Press {} to switch to {} mode", hotkey, text));
    }

    fn show_selection(ui: &mut Ui, selection: &SelectionSet) {
        let count = selection.count();

        if count == 0 {
            ui.label("Nothing selected");
        } else if count == 1 {
            ui.label("1 entity selected");
        } else {
            ui.label(format!("{} entities selected", count))
                .on_hover_text("Use Ctrl+Click to toggle selection, Shift+Click for range");
        }
    }

    fn show_undo_redo(ui: &mut Ui, undo_stack: &UndoStack) {
        let undo_count = undo_stack.undo_count();
        let redo_count = undo_stack.redo_count();

        if undo_count > 0 {
            let desc = undo_stack.undo_description().unwrap_or_default();
            ui.label(format!("Undo ({}): {}", undo_count, desc))
                .on_hover_text("Ctrl+Z to undo");
        } else {
            ui.label("Undo (0)")
                .on_hover_text("Make some changes to enable undo");
        }

        ui.add_space(8.0);

        if redo_count > 0 {
            let desc = undo_stack.redo_description().unwrap_or_default();
            ui.label(format!("Redo ({}): {}", redo_count, desc))
                .on_hover_text("Ctrl+Y to redo");
        } else {
            ui.label("Redo (0)")
                .on_hover_text("Undo something to enable redo");
        }
    }

    fn show_snap_settings(ui: &mut Ui, snap: &SnappingConfig) {
        if snap.grid_enabled {
            ui.label(format!("Grid: {:.1}u", snap.grid_size))
                .on_hover_text("Grid snapping enabled - Hold Ctrl to snap positions");
        }

        if snap.angle_enabled {
            ui.label(format!("Angle: {:.0}°", snap.angle_increment))
                .on_hover_text("Angle snapping enabled - Hold Ctrl to snap rotations");
        }

        if !snap.grid_enabled && !snap.angle_enabled {
            ui.label("Snap: OFF")
                .on_hover_text("Press S to toggle snapping");
        }
    }

    /// Show resource usage indicators (Week 6 Day 5)
    fn show_resource_usage(ui: &mut Ui, usage: &ResourceUsage) {
        // Memory usage indicator
        let mem_percent = usage.memory_percent();
        let mem_color = if mem_percent < 50.0 {
            egui::Color32::GREEN
        } else if mem_percent < 80.0 {
            egui::Color32::YELLOW
        } else {
            egui::Color32::RED
        };

        if usage.memory_total > 0 {
            ui.colored_label(
                mem_color,
                format!("Mem: {}", ResourceUsage::format_bytes(usage.memory_used)),
            )
            .on_hover_text(format!(
                "Memory: {} / {} ({:.1}%)",
                ResourceUsage::format_bytes(usage.memory_used),
                ResourceUsage::format_bytes(usage.memory_total),
                mem_percent
            ));
        }

        // GPU memory indicator
        let gpu_mem_percent = usage.gpu_memory_percent();
        let gpu_mem_color = if gpu_mem_percent < 50.0 {
            egui::Color32::GREEN
        } else if gpu_mem_percent < 80.0 {
            egui::Color32::YELLOW
        } else {
            egui::Color32::RED
        };

        if usage.gpu_memory_total > 0 {
            ui.add_space(4.0);
            ui.colored_label(
                gpu_mem_color,
                format!(
                    "GPU: {}",
                    ResourceUsage::format_bytes(usage.gpu_memory_used)
                ),
            )
            .on_hover_text(format!(
                "GPU Memory: {} / {} ({:.1}%)",
                ResourceUsage::format_bytes(usage.gpu_memory_used),
                ResourceUsage::format_bytes(usage.gpu_memory_total),
                gpu_mem_percent
            ));
        }

        // GPU utilization indicator
        if usage.gpu_utilization > 0.0 {
            let gpu_util_color = if usage.gpu_utilization < 50.0 {
                egui::Color32::GREEN
            } else if usage.gpu_utilization < 80.0 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::RED
            };

            ui.add_space(4.0);
            ui.colored_label(
                gpu_util_color,
                format!("GPU: {:.0}%", usage.gpu_utilization),
            )
            .on_hover_text("GPU utilization percentage");
        }
    }

    fn show_fps(ui: &mut Ui, fps: f32) {
        let color = if fps >= 55.0 {
            egui::Color32::GREEN
        } else if fps >= 30.0 {
            egui::Color32::YELLOW
        } else {
            egui::Color32::RED
        };

        ui.colored_label(color, format!("FPS: {:.0}", fps))
            .on_hover_text("Target: 60 FPS for smooth editing");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::UndoStack;
    use crate::entity_manager::SelectionSet;
    use crate::gizmo::snapping::SnappingConfig;
    use crate::gizmo::{state::GizmoMode, AxisConstraint};

    #[test]
    fn test_status_bar_creation() {
        let selection = SelectionSet::new();
        let undo_stack = UndoStack::new(100);
        let gizmo_mode = GizmoMode::Translate {
            constraint: AxisConstraint::None,
        };
        let snap_config = SnappingConfig::default();

        assert_eq!(selection.count(), 0);
        assert!(!undo_stack.can_undo());
        assert!(!undo_stack.can_redo());

        assert_eq!(
            gizmo_mode,
            GizmoMode::Translate {
                constraint: AxisConstraint::None
            }
        );
        assert!(snap_config.grid_enabled);
    }

    // Week 6 Day 5 Tests: Resource Usage

    #[test]
    fn test_resource_usage_new() {
        let usage = ResourceUsage::new();
        assert_eq!(usage.memory_used, 0);
        assert_eq!(usage.memory_total, 0);
        assert_eq!(usage.gpu_memory_used, 0);
        assert_eq!(usage.gpu_memory_total, 0);
        assert_eq!(usage.gpu_utilization, 0.0);
    }

    #[test]
    fn test_resource_usage_memory_percent() {
        let mut usage = ResourceUsage::new();
        usage.memory_used = 500;
        usage.memory_total = 1000;
        assert!((usage.memory_percent() - 50.0).abs() < 0.001);

        // Test zero total (avoid divide by zero)
        usage.memory_total = 0;
        assert_eq!(usage.memory_percent(), 0.0);
    }

    #[test]
    fn test_resource_usage_gpu_memory_percent() {
        let mut usage = ResourceUsage::new();
        usage.gpu_memory_used = 2 * 1024 * 1024 * 1024; // 2 GB
        usage.gpu_memory_total = 8 * 1024 * 1024 * 1024; // 8 GB
        assert!((usage.gpu_memory_percent() - 25.0).abs() < 0.001);

        // Test zero total
        usage.gpu_memory_total = 0;
        assert_eq!(usage.gpu_memory_percent(), 0.0);
    }

    #[test]
    fn test_format_bytes_bytes() {
        assert_eq!(ResourceUsage::format_bytes(0), "0 B");
        assert_eq!(ResourceUsage::format_bytes(512), "512 B");
        assert_eq!(ResourceUsage::format_bytes(1023), "1023 B");
    }

    #[test]
    fn test_format_bytes_kilobytes() {
        assert_eq!(ResourceUsage::format_bytes(1024), "1.0 KB");
        assert_eq!(ResourceUsage::format_bytes(1536), "1.5 KB");
        assert_eq!(ResourceUsage::format_bytes(10 * 1024), "10.0 KB");
    }

    #[test]
    fn test_format_bytes_megabytes() {
        assert_eq!(ResourceUsage::format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(ResourceUsage::format_bytes(256 * 1024 * 1024), "256.0 MB");
    }

    #[test]
    fn test_format_bytes_gigabytes() {
        assert_eq!(ResourceUsage::format_bytes(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(
            ResourceUsage::format_bytes(4 * 1024 * 1024 * 1024),
            "4.0 GB"
        );
        assert_eq!(
            ResourceUsage::format_bytes(16 * 1024 * 1024 * 1024),
            "16.0 GB"
        );
    }

    #[test]
    fn test_background_task_summary_empty() {
        let summary = BackgroundTaskSummary::empty();
        assert_eq!(summary.active_count, 0);
        assert_eq!(summary.overall_progress, 0.0);
        assert!(summary.primary_task.is_none());
        assert!(summary.started_at.is_none());
        assert!(!summary.is_active());
    }

    #[test]
    fn test_background_task_summary_active() {
        let summary = BackgroundTaskSummary {
            active_count: 2,
            overall_progress: 0.5,
            primary_task: Some("Loading assets".to_string()),
            started_at: Some(Instant::now()),
        };
        assert!(summary.is_active());
        assert_eq!(summary.active_count, 2);
        assert_eq!(summary.primary_task.as_deref(), Some("Loading assets"));
    }
}
