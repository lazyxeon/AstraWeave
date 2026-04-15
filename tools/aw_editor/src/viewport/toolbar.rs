//! Viewport Toolbar
//!
//! Floating toolbar overlay for viewport controls and settings.
//! Provides quick access to common viewport operations.
//!
//! # Features
//!
//! - Shading mode toggle (wireframe/lit/unlit)
//! - Grid visibility toggle with type selector (infinite/crosshair)
//! - Snap-to-grid toggle and settings
//! - Performance stats display
//! - Camera bookmarks

/// Grid display type for viewport
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum GridType {
    /// Infinite ground plane grid with distance fading
    #[default]
    Infinite,

    /// Crosshair-style grid (XZ axis lines only, no full grid)
    Crosshair,

    /// No grid (same as show_grid = false, but separate option)
    None,
}

impl GridType {
    /// Get all grid types
    pub fn all() -> &'static [GridType] {
        &[GridType::Infinite, GridType::Crosshair, GridType::None]
    }

    /// Cycle to next grid type
    pub fn cycle(&self) -> Self {
        match self {
            GridType::Infinite => GridType::Crosshair,
            GridType::Crosshair => GridType::None,
            GridType::None => GridType::Infinite,
        }
    }

    /// Display name for UI
    pub fn name(&self) -> &'static str {
        match self {
            GridType::Infinite => "Infinite",
            GridType::Crosshair => "Crosshair",
            GridType::None => "None",
        }
    }

    /// Get icon for this grid type
    pub fn icon(&self) -> &'static str {
        match self {
            GridType::Infinite => "⊞",
            GridType::Crosshair => "✛",
            GridType::None => "⊠",
        }
    }

    /// Check if grid is visible with this type
    pub fn is_visible(&self) -> bool {
        !matches!(self, GridType::None)
    }
}

impl std::fmt::Display for GridType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

/// Viewport toolbar widget
///
/// Displays as floating panel over the viewport with professional controls.
#[derive(Debug, Clone)]
pub struct ViewportToolbar {
    /// Current shading mode
    pub shading_mode: ShadingMode,

    /// Grid visibility
    pub show_grid: bool,

    /// Grid display type (infinite plane vs crosshair)
    pub grid_type: GridType,

    /// Snap to grid enabled
    pub snap_enabled: bool,

    /// Grid snap size (meters)
    pub snap_size: f32,

    /// Angle snap enabled
    pub angle_snap_enabled: bool,

    /// Angle snap increment (degrees)
    pub angle_snap_degrees: f32,

    /// Show performance stats
    pub show_stats: bool,

    /// Performance stats (updated by viewport)
    pub stats: PerformanceStats,

    /// Current play state (mirrored from EditorMode)
    pub play_state: PlayState,

    /// Pending play action from toolbar buttons
    play_action: Option<PlayAction>,
}

/// Editor play state for toolbar display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayState {
    #[default]
    Editing,
    Playing,
    Paused,
}

/// Action requested by toolbar play controls
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayAction {
    Play,
    Pause,
    Stop,
    Step,
}

impl Default for ViewportToolbar {
    fn default() -> Self {
        Self {
            shading_mode: ShadingMode::Lit,
            show_grid: true,
            grid_type: GridType::Infinite,
            snap_enabled: false,
            snap_size: 1.0,
            angle_snap_enabled: true,
            angle_snap_degrees: 15.0,
            show_stats: true,
            stats: PerformanceStats::default(),
            play_state: PlayState::default(),
            play_action: None,
        }
    }
}

impl ViewportToolbar {
    /// Render toolbar UI
    ///
    /// Displays as floating panel at top-left of viewport.
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        viewport_rect: egui::Rect,
        camera: &mut super::camera::OrbitCamera,
    ) {
        // Helper: check if area needs repositioning (first frame or drifted outside viewport)
        let needs_reposition = |ctx: &egui::Context, id: egui::Id, vp: egui::Rect| -> bool {
            ctx.memory(|mem| match mem.area_rect(id) {
                None => true,
                Some(rect) => !vp.intersects(rect),
            })
        };

        // ── Toolbar: top-center of viewport ──
        let toolbar_id = egui::Id::new("viewport_toolbar_v2");
        // Approximate toolbar width ~600px; center it horizontally
        let toolbar_pos = egui::pos2(
            viewport_rect.center().x - 300.0,
            viewport_rect.top() + 10.0,
        );

        let mut toolbar_area = egui::Area::new(toolbar_id)
            .movable(true)
            .constrain_to(viewport_rect)
            .order(egui::Order::Foreground);
        if needs_reposition(ui.ctx(), toolbar_id, viewport_rect) {
            toolbar_area = toolbar_area.current_pos(toolbar_pos);
        } else {
            toolbar_area = toolbar_area.default_pos(toolbar_pos);
        }
        toolbar_area
            .show(ui.ctx(), |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_premultiplied(30, 30, 35, 230))
                    .corner_radius(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Shading Mode
                            egui::ComboBox::from_id_salt("shading_mode")
                                .selected_text(format!("{:?}", self.shading_mode))
                                .width(90.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.shading_mode,
                                        ShadingMode::Lit,
                                        "Lit",
                                    );
                                    ui.selectable_value(
                                        &mut self.shading_mode,
                                        ShadingMode::Unlit,
                                        "Unlit",
                                    );
                                    ui.selectable_value(
                                        &mut self.shading_mode,
                                        ShadingMode::Wireframe,
                                        "Wireframe",
                                    );
                                });

                            ui.separator();

                            ui.checkbox(&mut self.show_grid, "Grid");

                            // Grid type selector (only show when grid is visible)
                            if self.show_grid {
                                egui::ComboBox::from_id_salt("grid_type")
                                    .selected_text(self.grid_type.name())
                                    .width(80.0)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.grid_type,
                                            GridType::Infinite,
                                            "Infinite",
                                        );
                                        ui.selectable_value(
                                            &mut self.grid_type,
                                            GridType::Crosshair,
                                            "Crosshair",
                                        );
                                    });
                            }

                            ui.separator();

                            ui.checkbox(&mut self.snap_enabled, "Grid Snap");
                            if self.snap_enabled {
                                if ui
                                    .small_button("0.5")
                                    .on_hover_text("Grid size: 0.5 units")
                                    .clicked()
                                {
                                    self.snap_size = 0.5;
                                }
                                if ui
                                    .small_button("1.0")
                                    .on_hover_text("Grid size: 1.0 units")
                                    .clicked()
                                {
                                    self.snap_size = 1.0;
                                }
                                if ui
                                    .small_button("2.0")
                                    .on_hover_text("Grid size: 2.0 units")
                                    .clicked()
                                {
                                    self.snap_size = 2.0;
                                }
                                ui.add(
                                    egui::DragValue::new(&mut self.snap_size)
                                        .speed(0.1)
                                        .range(0.1..=10.0)
                                        .suffix("m"),
                                );
                            }

                            ui.separator();

                            ui.checkbox(&mut self.angle_snap_enabled, "Angle Snap");
                            if self.angle_snap_enabled {
                                if ui
                                    .small_button("15°")
                                    .on_hover_text("Angle snap: 15 degrees")
                                    .clicked()
                                {
                                    self.angle_snap_degrees = 15.0;
                                }
                                if ui
                                    .small_button("45°")
                                    .on_hover_text("Angle snap: 45 degrees")
                                    .clicked()
                                {
                                    self.angle_snap_degrees = 45.0;
                                }
                                if ui
                                    .small_button("90°")
                                    .on_hover_text("Angle snap: 90 degrees")
                                    .clicked()
                                {
                                    self.angle_snap_degrees = 90.0;
                                }
                            }

                            ui.separator();

                            ui.checkbox(&mut self.show_stats, "Stats");
                        });
                    });
            });

        // Performance stats panel (bottom-left, inside viewport)
        if self.show_stats {
            let stats_id = egui::Id::new("viewport_stats_v2");
            let stats_pos = viewport_rect.left_bottom() + egui::vec2(10.0, -200.0);

            let mut stats_area = egui::Area::new(stats_id)
                .movable(true)
                .constrain_to(viewport_rect)
                .order(egui::Order::Foreground);
            if needs_reposition(ui.ctx(), stats_id, viewport_rect) {
                stats_area = stats_area.current_pos(stats_pos);
            } else {
                stats_area = stats_area.default_pos(stats_pos);
            }
            stats_area
                .show(ui.ctx(), |ui| {
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgba_premultiplied(20, 20, 25, 200))
                        .corner_radius(4.0)
                        .inner_margin(6.0)
                        .show(ui, |ui| {
                            ui.style_mut().spacing.item_spacing = egui::vec2(4.0, 2.0);
                            ui.label(egui::RichText::new("Performance").strong());
                            ui.separator();
                            ui.label(format!("FPS: {:.1}", self.stats.fps));
                            ui.label(format!("Frame: {:.2}ms", self.stats.frame_time_ms));
                            ui.label(format!("Entities: {}", self.stats.entity_count));
                            ui.label(format!("Triangles: {}K", self.stats.triangle_count / 1000));
                            ui.label(format!(
                                "Scatter: {} inst / {} draws",
                                self.stats.scatter_instances, self.stats.scatter_draw_calls
                            ));
                            ui.label(format!("Memory: {:.1} MB", self.stats.memory_usage_mb));

                            if !self.stats.frame_time_history.is_empty() {
                                ui.separator();
                                let graph_height = 30.0;
                                let graph_width = 120.0;
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(graph_width, graph_height),
                                    egui::Sense::hover(),
                                );
                                let max_time = self
                                    .stats
                                    .frame_time_history
                                    .iter()
                                    .copied()
                                    .fold(16.67f32, f32::max);
                                let painter = ui.painter();
                                painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(30, 30, 40));
                                let target_line_y = rect.max.y - (16.67 / max_time) * graph_height;
                                painter.line_segment(
                                    [
                                        egui::pos2(rect.min.x, target_line_y),
                                        egui::pos2(rect.max.x, target_line_y),
                                    ],
                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)),
                                );
                                let history = &self.stats.frame_time_history;
                                let step = graph_width / 60.0;
                                for (i, &frame_time) in history.iter().enumerate() {
                                    let x = rect.min.x + (i as f32) * step;
                                    let h = (frame_time / max_time) * graph_height;
                                    let color = if frame_time <= 16.67 {
                                        egui::Color32::GREEN
                                    } else if frame_time <= 33.33 {
                                        egui::Color32::YELLOW
                                    } else {
                                        egui::Color32::RED
                                    };
                                    painter.line_segment(
                                        [egui::pos2(x, rect.max.y), egui::pos2(x, rect.max.y - h)],
                                        egui::Stroke::new(2.0, color),
                                    );
                                }
                            }
                        });
                });
        }

        // Camera & Selection info panel (top-right, below toolbar)
        {
            let info_id = egui::Id::new("viewport_info_v2");
            let info_pos = viewport_rect.right_top() + egui::vec2(-150.0, 10.0);

            let mut info_area = egui::Area::new(info_id)
                .movable(true)
                .constrain_to(viewport_rect)
                .order(egui::Order::Foreground);
            if needs_reposition(ui.ctx(), info_id, viewport_rect) {
                info_area = info_area.current_pos(info_pos);
            } else {
                info_area = info_area.default_pos(info_pos);
            }
            info_area
                .show(ui.ctx(), |ui| {
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgba_premultiplied(20, 20, 25, 200))
                        .corner_radius(4.0)
                        .inner_margin(6.0)
                        .show(ui, |ui| {
                            ui.style_mut().spacing.item_spacing = egui::vec2(4.0, 2.0);
                            let [x, y, z] = self.stats.camera_position;
                            ui.label(egui::RichText::new("Camera").strong());
                            ui.label(format!("X: {:.1}", x));
                            ui.label(format!("Y: {:.1}", y));
                            ui.label(format!("Z: {:.1}", z));
                            ui.separator();
                            let sel = self.stats.selection_count;
                            if sel == 0 {
                                ui.label("No selection");
                            } else if sel == 1 {
                                ui.label("1 selected");
                            } else {
                                ui.label(format!("{} selected", sel));
                            }
                            ui.separator();
                            if ui
                                .button("Reset Camera")
                                .on_hover_text("Reset camera to default position")
                                .clicked()
                            {
                                camera.reset_to_origin();
                            }
                        });
                });
        }

        // Play controls removed — redundant and non-functional.
    }

    /// Take the pending play action, if any
    pub fn take_play_action(&mut self) -> Option<PlayAction> {
        self.play_action.take()
    }
}

/// Shading mode for viewport rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShadingMode {
    /// Full lighting with shadows
    Lit,

    /// No lighting, flat colors
    Unlit,

    /// Wireframe overlay
    Wireframe,
}

impl ShadingMode {
    /// Get all shading modes
    pub fn all() -> &'static [ShadingMode] {
        &[ShadingMode::Lit, ShadingMode::Unlit, ShadingMode::Wireframe]
    }

    /// Display name for UI
    pub fn name(&self) -> &'static str {
        match self {
            ShadingMode::Lit => "Lit",
            ShadingMode::Unlit => "Unlit",
            ShadingMode::Wireframe => "Wireframe",
        }
    }

    /// Get icon for this shading mode
    pub fn icon(&self) -> &'static str {
        match self {
            ShadingMode::Lit => "[Lt]",
            ShadingMode::Unlit => "[Un]",
            ShadingMode::Wireframe => "[Wf]",
        }
    }

    /// Check if lighting is enabled
    pub fn has_lighting(&self) -> bool {
        matches!(self, ShadingMode::Lit)
    }

    /// Check if this mode shows wireframe
    pub fn is_wireframe(&self) -> bool {
        matches!(self, ShadingMode::Wireframe)
    }

    /// Cycle to next shading mode
    pub fn cycle(&self) -> Self {
        match self {
            ShadingMode::Lit => ShadingMode::Unlit,
            ShadingMode::Unlit => ShadingMode::Wireframe,
            ShadingMode::Wireframe => ShadingMode::Lit,
        }
    }

    pub fn to_u32(self) -> u32 {
        match self {
            ShadingMode::Lit => 0,
            ShadingMode::Unlit => 1,
            ShadingMode::Wireframe => 2,
        }
    }
}

impl std::fmt::Display for ShadingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

/// Performance statistics for viewport
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    /// Frames per second
    pub fps: f32,

    /// Frame time in milliseconds
    pub frame_time_ms: f32,

    /// Number of entities rendered
    pub entity_count: u32,

    /// Number of triangles rendered
    pub triangle_count: u32,

    /// Memory usage in megabytes
    pub memory_usage_mb: f32,

    /// Frame time history for graph (last 60 frames)
    pub frame_time_history: Vec<f32>,

    /// Camera position for HUD display
    pub camera_position: [f32; 3],

    /// Number of selected entities
    pub selection_count: usize,

    /// Total scatter instances uploaded to GPU
    pub scatter_instances: usize,

    /// Number of scatter draw calls (mesh_type × quadrants)
    pub scatter_draw_calls: u32,
}

impl Default for PerformanceStats {
    fn default() -> Self {
        Self {
            fps: 0.0,
            frame_time_ms: 0.0,
            entity_count: 0,
            triangle_count: 0,
            memory_usage_mb: 0.0,
            frame_time_history: Vec::with_capacity(60),
            camera_position: [0.0, 0.0, 0.0],
            selection_count: 0,
            scatter_instances: 0,
            scatter_draw_calls: 0,
        }
    }
}

impl PerformanceStats {
    pub fn push_frame_time(&mut self, frame_time_ms: f32) {
        if self.frame_time_history.len() >= 60 {
            self.frame_time_history.remove(0);
        }
        self.frame_time_history.push(frame_time_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_type_cycle() {
        let grid = GridType::Infinite;
        assert_eq!(grid.cycle(), GridType::Crosshair);
        assert_eq!(grid.cycle().cycle(), GridType::None);
        assert_eq!(grid.cycle().cycle().cycle(), GridType::Infinite);
    }

    #[test]
    fn test_grid_type_name() {
        assert_eq!(GridType::Infinite.name(), "Infinite");
        assert_eq!(GridType::Crosshair.name(), "Crosshair");
        assert_eq!(GridType::None.name(), "None");
    }

    #[test]
    fn test_shading_mode_values() {
        assert_eq!(ShadingMode::Lit.to_u32(), 0);
        assert_eq!(ShadingMode::Unlit.to_u32(), 1);
        assert_eq!(ShadingMode::Wireframe.to_u32(), 2);
    }

    #[test]
    fn test_toolbar_defaults() {
        let toolbar = ViewportToolbar::default();
        assert_eq!(toolbar.shading_mode, ShadingMode::Lit);
        assert!(toolbar.show_grid);
        assert_eq!(toolbar.grid_type, GridType::Infinite);
        assert!(!toolbar.snap_enabled);
        assert_eq!(toolbar.snap_size, 1.0);
        assert!(toolbar.angle_snap_enabled);
        assert_eq!(toolbar.angle_snap_degrees, 15.0);
        assert!(toolbar.show_stats);
    }

    #[test]
    fn test_performance_stats_default() {
        let stats = PerformanceStats::default();
        assert_eq!(stats.fps, 0.0);
        assert_eq!(stats.frame_time_history.capacity(), 60);
    }

    #[test]
    fn test_performance_stats_push() {
        let mut stats = PerformanceStats::default();
        for i in 0..70 {
            stats.push_frame_time(i as f32);
        }
        assert_eq!(stats.frame_time_history.len(), 60);
        assert_eq!(*stats.frame_time_history.last().unwrap(), 69.0);
        assert_eq!(stats.frame_time_history[0], 10.0);
    }

    #[test]
    fn test_grid_type_default() {
        assert_eq!(GridType::default(), GridType::Infinite);
    }

    // === GridType Display and helper tests ===

    #[test]
    fn test_grid_type_all() {
        let all = GridType::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&GridType::Infinite));
        assert!(all.contains(&GridType::Crosshair));
        assert!(all.contains(&GridType::None));
    }

    #[test]
    fn test_grid_type_display() {
        assert_eq!(format!("{}", GridType::Infinite), "⊞ Infinite");
        assert_eq!(format!("{}", GridType::Crosshair), "✛ Crosshair");
        assert_eq!(format!("{}", GridType::None), "⊠ None");
    }

    #[test]
    fn test_grid_type_icon() {
        assert_eq!(GridType::Infinite.icon(), "⊞");
        assert_eq!(GridType::Crosshair.icon(), "✛");
        assert_eq!(GridType::None.icon(), "⊠");
    }

    #[test]
    fn test_grid_type_is_visible() {
        assert!(GridType::Infinite.is_visible());
        assert!(GridType::Crosshair.is_visible());
        assert!(!GridType::None.is_visible());
    }

    #[test]
    fn test_grid_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(GridType::Infinite);
        set.insert(GridType::Crosshair);
        assert_eq!(set.len(), 2);
    }

    // === ShadingMode Display and helper tests ===

    #[test]
    fn test_shading_mode_all() {
        let all = ShadingMode::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&ShadingMode::Lit));
        assert!(all.contains(&ShadingMode::Unlit));
        assert!(all.contains(&ShadingMode::Wireframe));
    }

    #[test]
    fn test_shading_mode_display() {
        assert_eq!(format!("{}", ShadingMode::Lit), "[Lt] Lit");
        assert_eq!(format!("{}", ShadingMode::Unlit), "[Un] Unlit");
        assert_eq!(format!("{}", ShadingMode::Wireframe), "[Wf] Wireframe");
    }

    #[test]
    fn test_shading_mode_name() {
        assert_eq!(ShadingMode::Lit.name(), "Lit");
        assert_eq!(ShadingMode::Unlit.name(), "Unlit");
        assert_eq!(ShadingMode::Wireframe.name(), "Wireframe");
    }

    #[test]
    fn test_shading_mode_icon() {
        assert_eq!(ShadingMode::Lit.icon(), "[Lt]");
        assert_eq!(ShadingMode::Unlit.icon(), "[Un]");
        assert_eq!(ShadingMode::Wireframe.icon(), "[Wf]");
    }

    #[test]
    fn test_shading_mode_has_lighting() {
        assert!(ShadingMode::Lit.has_lighting());
        assert!(!ShadingMode::Unlit.has_lighting());
        assert!(!ShadingMode::Wireframe.has_lighting());
    }

    #[test]
    fn test_shading_mode_is_wireframe() {
        assert!(!ShadingMode::Lit.is_wireframe());
        assert!(!ShadingMode::Unlit.is_wireframe());
        assert!(ShadingMode::Wireframe.is_wireframe());
    }

    #[test]
    fn test_shading_mode_cycle() {
        assert_eq!(ShadingMode::Lit.cycle(), ShadingMode::Unlit);
        assert_eq!(ShadingMode::Unlit.cycle(), ShadingMode::Wireframe);
        assert_eq!(ShadingMode::Wireframe.cycle(), ShadingMode::Lit);
    }

    #[test]
    fn test_shading_mode_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ShadingMode::Lit);
        set.insert(ShadingMode::Wireframe);
        assert_eq!(set.len(), 2);
    }

    // ========== Play Controls Tests ==========

    #[test]
    fn test_play_state_default() {
        assert_eq!(PlayState::default(), PlayState::Editing);
    }

    #[test]
    fn test_play_action_variants() {
        let actions = [
            PlayAction::Play,
            PlayAction::Pause,
            PlayAction::Stop,
            PlayAction::Step,
        ];
        // Ensure all variants are distinct
        for (i, a) in actions.iter().enumerate() {
            for (j, b) in actions.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn test_toolbar_take_play_action() {
        let mut toolbar = ViewportToolbar::default();
        assert!(toolbar.take_play_action().is_none());

        toolbar.play_action = Some(PlayAction::Play);
        assert_eq!(toolbar.take_play_action(), Some(PlayAction::Play));
        // Should be None after take
        assert!(toolbar.take_play_action().is_none());
    }

    #[test]
    fn test_toolbar_play_state_sync() {
        let mut toolbar = ViewportToolbar::default();
        assert_eq!(toolbar.play_state, PlayState::Editing);

        toolbar.play_state = PlayState::Playing;
        assert_eq!(toolbar.play_state, PlayState::Playing);

        toolbar.play_state = PlayState::Paused;
        assert_eq!(toolbar.play_state, PlayState::Paused);
    }
}
