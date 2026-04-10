//! Blueprint Panel — 2D Top-Down Zone Editor
//!
//! Provides a canvas-based polygon zone editor for defining terrain generation zones:
//! - **Pan + Zoom Canvas**: egui Painter with world-to-screen transform, grid overlay
//! - **Polygon Drawing Tools**: PlaceNode, MoveNode, DeleteNode, ConnectNodes, Select
//! - **Zone Inspector Sidebar**: name, priority, source selector, placement mode, blend margin
//! - **Zone Rendering**: translucent filled polygons with outlines, draggable vertices
//! - **Undo/Redo**: Built-in command stack for all zone mutations
//! - **Actions**: AddZone, RemoveZone, GenerateZone, GenerateAll, ClearGeneration

use crate::panels::Panel;
use egui::{Color32, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use glam::Vec2 as GVec2;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::info;

// ============================================================================
// BLUEPRINT TOOL — Active drawing/editing tool
// ============================================================================

/// Active tool in the blueprint canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlueprintTool {
    /// Click to add vertex to current polygon
    #[default]
    PlaceNode,
    /// Drag existing vertex to reposition
    MoveNode,
    /// Click vertex or edge to remove
    DeleteNode,
    /// Auto-close polygon when clicking first vertex
    ConnectNodes,
    /// Click a zone to select it
    Select,
}

impl BlueprintTool {
    pub fn name(&self) -> &'static str {
        match self {
            Self::PlaceNode => "Place Node",
            Self::MoveNode => "Move Node",
            Self::DeleteNode => "Delete Node",
            Self::ConnectNodes => "Connect Nodes",
            Self::Select => "Select Zone",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::PlaceNode => "[+]",
            Self::MoveNode => "[<>]",
            Self::DeleteNode => "[-]",
            Self::ConnectNodes => "[O]",
            Self::Select => "[S]",
        }
    }

    pub fn all() -> &'static [BlueprintTool] {
        &[
            Self::PlaceNode,
            Self::MoveNode,
            Self::DeleteNode,
            Self::ConnectNodes,
            Self::Select,
        ]
    }
}

// ============================================================================
// BLUEPRINT ACTION — Actions emitted by the panel
// ============================================================================

/// Actions the panel needs the host (main.rs) to execute.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum BlueprintAction {
    /// Generate scatter for a specific zone
    GenerateZone { zone_index: usize },
    /// Generate scatter for all zones
    GenerateAll,
    /// Clear previously generated scatter
    ClearGeneration,
    /// Save zones to file
    SaveZones,
    /// Load zones from file
    LoadZones,
}

// ============================================================================
// BLUEPRINT COMMAND — Undo/Redo
// ============================================================================

/// An undoable command for zone mutations.
#[derive(Debug, Clone)]
enum BlueprintCommand {
    AddZone {
        zone: ZoneState,
    },
    RemoveZone {
        index: usize,
        zone: ZoneState,
    },
    AddVertex {
        zone_index: usize,
        vertex: GVec2,
    },
    RemoveVertex {
        zone_index: usize,
        vertex_index: usize,
        vertex: GVec2,
    },
    MoveVertex {
        zone_index: usize,
        vertex_index: usize,
        old_pos: GVec2,
        new_pos: GVec2,
    },
    UpdateZoneName {
        zone_index: usize,
        old_name: String,
        new_name: String,
    },
    UpdateZonePriority {
        zone_index: usize,
        old_priority: i32,
        new_priority: i32,
    },
    UpdateZoneSource {
        zone_index: usize,
        old_source: ZoneSourceState,
        new_source: ZoneSourceState,
    },
    UpdateBlendMargin {
        zone_index: usize,
        old_margin: f32,
        new_margin: f32,
    },
}

/// Internal undo stack (separate from the ECS-world undo stack).
struct BlueprintUndoStack {
    undo: VecDeque<BlueprintCommand>,
    redo: Vec<BlueprintCommand>,
    capacity: usize,
}

impl BlueprintUndoStack {
    fn new(capacity: usize) -> Self {
        Self {
            undo: VecDeque::with_capacity(capacity),
            redo: Vec::new(),
            capacity,
        }
    }

    fn push(&mut self, cmd: BlueprintCommand) {
        if self.undo.len() >= self.capacity {
            self.undo.pop_front();
        }
        self.undo.push_back(cmd);
        self.redo.clear();
    }

    fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

// ============================================================================
// ZONE STATE — Panel-local zone representation
// ============================================================================

/// Source type for a zone (panel-side mirror of terrain's ZoneSource).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ZoneSourceState {
    BiomePreset(String),
    BlendScene { pack_path: String, replica: bool },
}

impl Default for ZoneSourceState {
    fn default() -> Self {
        Self::BiomePreset("Grassland".into())
    }
}

/// Panel-local zone state (lightweight mirror of BlueprintZone).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneState {
    pub name: String,
    pub vertices: Vec<GVec2>,
    pub source: ZoneSourceState,
    pub priority: i32,
    pub enabled: bool,
    pub blend_margin: f32,
    /// Manual scene scale override (None = auto, Some(ratio) = user-controlled).
    pub scene_scale_override: Option<f32>,
}

impl Default for ZoneState {
    fn default() -> Self {
        Self {
            name: "New Zone".into(),
            vertices: Vec::new(),
            source: ZoneSourceState::default(),
            priority: 0,
            enabled: true,
            blend_margin: 8.0,
            scene_scale_override: None,
        }
    }
}

impl ZoneState {
    fn color(&self) -> Color32 {
        match &self.source {
            ZoneSourceState::BiomePreset(_) => Color32::from_rgba_premultiplied(50, 180, 80, 60),
            ZoneSourceState::BlendScene { .. } => {
                Color32::from_rgba_premultiplied(60, 120, 220, 60)
            }
        }
    }

    fn outline_color(&self) -> Color32 {
        match &self.source {
            ZoneSourceState::BiomePreset(_) => Color32::from_rgb(80, 220, 100),
            ZoneSourceState::BlendScene { .. } => Color32::from_rgb(80, 150, 255),
        }
    }
}

// ============================================================================
// CAMERA / TRANSFORM
// ============================================================================

/// 2D camera for the blueprint canvas (pan + zoom).
#[derive(Debug, Clone)]
struct CanvasCamera {
    /// World position at the center of the canvas.
    center: GVec2,
    /// Pixels per world unit.
    zoom: f32,
}

impl Default for CanvasCamera {
    fn default() -> Self {
        Self {
            center: GVec2::ZERO,
            zoom: 4.0,
        }
    }
}

impl CanvasCamera {
    fn world_to_screen(&self, world: GVec2, canvas_center: Pos2) -> Pos2 {
        let dx = (world.x - self.center.x) * self.zoom;
        let dz = (world.y - self.center.y) * self.zoom;
        Pos2::new(canvas_center.x + dx, canvas_center.y - dz) // Y flipped
    }

    fn screen_to_world(&self, screen: Pos2, canvas_center: Pos2) -> GVec2 {
        let dx = (screen.x - canvas_center.x) / self.zoom;
        let dz = -(screen.y - canvas_center.y) / self.zoom; // Y flipped
        GVec2::new(self.center.x + dx, self.center.y + dz)
    }
}

// ============================================================================
// BLUEPRINT PANEL
// ============================================================================

/// The 2D top-down blueprint zone editor panel.
pub struct BlueprintPanel {
    /// Active drawing tool.
    tool: BlueprintTool,
    /// Camera (pan/zoom) state.
    camera: CanvasCamera,
    /// All zones being edited.
    zones: Vec<ZoneState>,
    /// Index of the currently selected zone (None = no selection).
    selected_zone: Option<usize>,
    /// Vertices being placed for a new zone (before closing).
    pending_vertices: Vec<GVec2>,
    /// Drag state: (zone_index, vertex_index, original_pos).
    drag_state: Option<(usize, usize, GVec2)>,
    /// Undo/redo stack.
    undo_stack: BlueprintUndoStack,
    /// Actions queued for the host.
    pending_actions: Vec<BlueprintAction>,
    /// Show grid overlay.
    show_grid: bool,
    /// Grid spacing in world units.
    grid_spacing: f32,
    /// Next zone ID counter.
    next_zone_id: u64,
    /// Vertex hit radius in screen pixels.
    vertex_hit_radius: f32,
}

impl Default for BlueprintPanel {
    fn default() -> Self {
        Self {
            tool: BlueprintTool::default(),
            camera: CanvasCamera::default(),
            zones: Vec::new(),
            selected_zone: None,
            pending_vertices: Vec::new(),
            drag_state: None,
            undo_stack: BlueprintUndoStack::new(200),
            pending_actions: Vec::new(),
            show_grid: true,
            grid_spacing: 64.0,
            next_zone_id: 1,
            vertex_hit_radius: 10.0,
        }
    }
}

impl BlueprintPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Take any queued actions (drain).
    pub fn take_actions(&mut self) -> Vec<BlueprintAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Get immutable access to zones (for generation / serialization).
    pub fn zones(&self) -> &[ZoneState] {
        &self.zones
    }

    /// Replace all zones (e.g., after loading from file).
    pub fn set_zones(&mut self, zones: Vec<ZoneState>) {
        self.zones = zones;
        self.selected_zone = None;
        self.pending_vertices.clear();
    }

    /// Add a single zone (e.g., from blend import "Create Replica Zone" flow).
    pub fn add_zone(&mut self, zone: ZoneState) {
        self.zones.push(zone);
        self.selected_zone = Some(self.zones.len() - 1);
    }

    // ---- undo / redo helpers ----

    fn execute_cmd(&mut self, cmd: BlueprintCommand) {
        self.apply_command(&cmd, false);
        self.undo_stack.push(cmd);
    }

    fn undo(&mut self) {
        // If actively placing nodes, pop the last pending vertex first.
        // This gives per-node undo during the PlaceNode workflow.
        if self.tool == BlueprintTool::PlaceNode && !self.pending_vertices.is_empty() {
            self.pending_vertices.pop();
            return;
        }
        if let Some(cmd) = self.undo_stack.undo.pop_back() {
            self.apply_command(&cmd, true);
            self.undo_stack.redo.push(cmd);
        }
    }

    fn redo(&mut self) {
        if let Some(cmd) = self.undo_stack.redo.pop() {
            self.apply_command(&cmd, false);
            self.undo_stack.undo.push_back(cmd);
        }
    }

    fn apply_command(&mut self, cmd: &BlueprintCommand, reverse: bool) {
        match cmd {
            BlueprintCommand::AddZone { zone } => {
                if reverse {
                    self.zones.pop();
                } else {
                    self.zones.push(zone.clone());
                    self.selected_zone = Some(self.zones.len() - 1);
                }
            }
            BlueprintCommand::RemoveZone { index, zone } => {
                if reverse {
                    let idx = (*index).min(self.zones.len());
                    self.zones.insert(idx, zone.clone());
                } else if *index < self.zones.len() {
                    self.zones.remove(*index);
                    if self.selected_zone == Some(*index) {
                        self.selected_zone = None;
                    }
                }
            }
            BlueprintCommand::AddVertex { zone_index, vertex } => {
                if reverse {
                    if let Some(z) = self.zones.get_mut(*zone_index) {
                        z.vertices.pop();
                    }
                } else if let Some(z) = self.zones.get_mut(*zone_index) {
                    z.vertices.push(*vertex);
                }
            }
            BlueprintCommand::RemoveVertex {
                zone_index,
                vertex_index,
                vertex,
            } => {
                if reverse {
                    if let Some(z) = self.zones.get_mut(*zone_index) {
                        let idx = (*vertex_index).min(z.vertices.len());
                        z.vertices.insert(idx, *vertex);
                    }
                } else if let Some(z) = self.zones.get_mut(*zone_index) {
                    if *vertex_index < z.vertices.len() {
                        z.vertices.remove(*vertex_index);
                    }
                }
            }
            BlueprintCommand::MoveVertex {
                zone_index,
                vertex_index,
                old_pos,
                new_pos,
            } => {
                if let Some(z) = self.zones.get_mut(*zone_index) {
                    if let Some(v) = z.vertices.get_mut(*vertex_index) {
                        *v = if reverse { *old_pos } else { *new_pos };
                    }
                }
            }
            BlueprintCommand::UpdateZoneName {
                zone_index,
                old_name,
                new_name,
            } => {
                if let Some(z) = self.zones.get_mut(*zone_index) {
                    z.name = if reverse {
                        old_name.clone()
                    } else {
                        new_name.clone()
                    };
                }
            }
            BlueprintCommand::UpdateZonePriority {
                zone_index,
                old_priority,
                new_priority,
            } => {
                if let Some(z) = self.zones.get_mut(*zone_index) {
                    z.priority = if reverse {
                        *old_priority
                    } else {
                        *new_priority
                    };
                }
            }
            BlueprintCommand::UpdateZoneSource {
                zone_index,
                old_source,
                new_source,
            } => {
                if let Some(z) = self.zones.get_mut(*zone_index) {
                    z.source = if reverse {
                        old_source.clone()
                    } else {
                        new_source.clone()
                    };
                }
            }
            BlueprintCommand::UpdateBlendMargin {
                zone_index,
                old_margin,
                new_margin,
            } => {
                if let Some(z) = self.zones.get_mut(*zone_index) {
                    z.blend_margin = if reverse { *old_margin } else { *new_margin };
                }
            }
        }
    }

    // ---- canvas drawing ----

    fn draw_grid(&self, painter: &egui::Painter, canvas_rect: Rect, canvas_center: Pos2) {
        if !self.show_grid {
            return;
        }
        let grid_color = Color32::from_rgba_premultiplied(60, 60, 60, 40);
        let axis_color = Color32::from_rgba_premultiplied(120, 120, 120, 80);

        // Visible world range
        let tl = self
            .camera
            .screen_to_world(canvas_rect.left_top(), canvas_center);
        let br = self
            .camera
            .screen_to_world(canvas_rect.right_bottom(), canvas_center);
        let min_x = tl.x.min(br.x);
        let max_x = tl.x.max(br.x);
        let min_z = tl.y.min(br.y);
        let max_z = tl.y.max(br.y);

        let spacing = self.grid_spacing;
        let start_x = (min_x / spacing).floor() as i64;
        let end_x = (max_x / spacing).ceil() as i64;
        let start_z = (min_z / spacing).floor() as i64;
        let end_z = (max_z / spacing).ceil() as i64;

        // Vertical lines
        for ix in start_x..=end_x {
            let wx = ix as f32 * spacing;
            let top = self
                .camera
                .world_to_screen(GVec2::new(wx, max_z), canvas_center);
            let bot = self
                .camera
                .world_to_screen(GVec2::new(wx, min_z), canvas_center);
            let color = if ix == 0 { axis_color } else { grid_color };
            painter.line_segment([top, bot], Stroke::new(1.0, color));
        }
        // Horizontal lines
        for iz in start_z..=end_z {
            let wz = iz as f32 * spacing;
            let left = self
                .camera
                .world_to_screen(GVec2::new(min_x, wz), canvas_center);
            let right = self
                .camera
                .world_to_screen(GVec2::new(max_x, wz), canvas_center);
            let color = if iz == 0 { axis_color } else { grid_color };
            painter.line_segment([left, right], Stroke::new(1.0, color));
        }
    }

    fn draw_zones(&self, painter: &egui::Painter, canvas_center: Pos2) {
        for (i, zone) in self.zones.iter().enumerate() {
            if zone.vertices.len() < 2 {
                continue;
            }
            let is_selected = self.selected_zone == Some(i);

            // Convert to screen coords
            let screen_pts: Vec<Pos2> = zone
                .vertices
                .iter()
                .map(|v| self.camera.world_to_screen(*v, canvas_center))
                .collect();

            // Filled polygon (convex mesh — for concave we'd need triangulation,
            // but egui's PathShape handles simple polygons).
            if zone.vertices.len() >= 3 {
                let fill = zone.color();
                let outline_w = if is_selected { 3.0 } else { 1.5 };
                let outline_c = if is_selected {
                    Color32::YELLOW
                } else {
                    zone.outline_color()
                };
                painter.add(egui::Shape::convex_polygon(
                    screen_pts.clone(),
                    fill,
                    Stroke::new(outline_w, outline_c),
                ));
            } else {
                // 2 vertices: just a line segment
                painter.line_segment(
                    [screen_pts[0], screen_pts[1]],
                    Stroke::new(2.0, zone.outline_color()),
                );
            }

            // Vertex dots
            for (vi, sp) in screen_pts.iter().enumerate() {
                let radius = if is_selected { 5.0 } else { 3.5 };
                let dot_color = if is_selected && Some(vi) == self.drag_state.as_ref().map(|d| d.1)
                {
                    Color32::WHITE
                } else {
                    zone.outline_color()
                };
                painter.circle_filled(*sp, radius, dot_color);
            }

            // Centroid label
            if zone.vertices.len() >= 3 {
                let cx: f32 =
                    zone.vertices.iter().map(|v| v.x).sum::<f32>() / zone.vertices.len() as f32;
                let cz: f32 =
                    zone.vertices.iter().map(|v| v.y).sum::<f32>() / zone.vertices.len() as f32;
                let sp = self
                    .camera
                    .world_to_screen(GVec2::new(cx, cz), canvas_center);
                painter.text(
                    sp,
                    egui::Align2::CENTER_CENTER,
                    &zone.name,
                    egui::FontId::proportional(12.0),
                    Color32::WHITE,
                );
            }
        }
    }

    fn draw_pending(&self, painter: &egui::Painter, canvas_center: Pos2) {
        if self.pending_vertices.is_empty() {
            return;
        }
        let pts: Vec<Pos2> = self
            .pending_vertices
            .iter()
            .map(|v| self.camera.world_to_screen(*v, canvas_center))
            .collect();

        for pair in pts.windows(2) {
            painter.line_segment(
                [pair[0], pair[1]],
                Stroke::new(2.0, Color32::from_rgb(255, 200, 60)),
            );
        }
        for p in &pts {
            painter.circle_filled(*p, 4.0, Color32::from_rgb(255, 220, 80));
        }
    }

    // ---- hit testing ----

    fn hit_vertex(&self, screen_pos: Pos2, canvas_center: Pos2) -> Option<(usize, usize)> {
        let r2 = self.vertex_hit_radius * self.vertex_hit_radius;
        for (zi, zone) in self.zones.iter().enumerate() {
            for (vi, v) in zone.vertices.iter().enumerate() {
                let sp = self.camera.world_to_screen(*v, canvas_center);
                let d2 = (sp.x - screen_pos.x).powi(2) + (sp.y - screen_pos.y).powi(2);
                if d2 <= r2 {
                    return Some((zi, vi));
                }
            }
        }
        None
    }

    fn hit_zone(&self, world_pos: GVec2) -> Option<usize> {
        // Simple point-in-polygon (ray casting) on each zone
        for (i, zone) in self.zones.iter().enumerate().rev() {
            if zone.vertices.len() < 3 {
                continue;
            }
            if point_in_polygon_simple(world_pos, &zone.vertices) {
                return Some(i);
            }
        }
        None
    }

    // ---- canvas input handling ----

    fn handle_canvas_input(&mut self, response: &egui::Response, canvas_center: Pos2) {
        // Pan with middle-mouse or right-drag
        if response.dragged_by(egui::PointerButton::Middle)
            || response.dragged_by(egui::PointerButton::Secondary)
        {
            let delta = response.drag_delta();
            self.camera.center.x -= delta.x / self.camera.zoom;
            self.camera.center.y += delta.y / self.camera.zoom; // Y flipped
        }

        // Zoom with scroll
        let scroll_delta = response.ctx.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 && response.hovered() {
            let factor = (scroll_delta * 0.005).exp();
            self.camera.zoom = (self.camera.zoom * factor).clamp(0.1, 100.0);
        }

        // Tool-specific left-click
        if response.clicked_by(egui::PointerButton::Primary) {
            if let Some(pos) = response.interact_pointer_pos() {
                let world = self.camera.screen_to_world(pos, canvas_center);
                match self.tool {
                    BlueprintTool::PlaceNode => self.tool_place_node(world),
                    BlueprintTool::DeleteNode => self.tool_delete_node(pos, canvas_center),
                    BlueprintTool::ConnectNodes => self.tool_connect_nodes(pos, canvas_center),
                    BlueprintTool::Select => {
                        // Simple click (no drag) selects zone
                        self.selected_zone = self.hit_zone(world);
                    }
                    BlueprintTool::MoveNode => {} // handled via drag below
                }
            }
        }

        // Drag vertices — works in both MoveNode and Select tools.
        // In Select mode, a vertex hit takes priority over zone selection,
        // giving intuitive direct manipulation.
        if self.tool == BlueprintTool::MoveNode || self.tool == BlueprintTool::Select {
            if response.drag_started_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    if let Some((zi, vi)) = self.hit_vertex(pos, canvas_center) {
                        let orig = self.zones[zi].vertices[vi];
                        self.drag_state = Some((zi, vi, orig));
                        // Also select the zone being dragged
                        self.selected_zone = Some(zi);
                    }
                }
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                if let (Some((zi, vi, _)), Some(pos)) =
                    (self.drag_state, response.interact_pointer_pos())
                {
                    let world = self.camera.screen_to_world(pos, canvas_center);
                    if let Some(z) = self.zones.get_mut(zi) {
                        if let Some(v) = z.vertices.get_mut(vi) {
                            *v = world;
                        }
                    }
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                if let Some((zi, vi, old_pos)) = self.drag_state.take() {
                    if let Some(z) = self.zones.get(zi) {
                        if let Some(&new_pos) = z.vertices.get(vi) {
                            if (new_pos - old_pos).length() > 0.01 {
                                self.undo_stack.push(BlueprintCommand::MoveVertex {
                                    zone_index: zi,
                                    vertex_index: vi,
                                    old_pos,
                                    new_pos,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    fn tool_place_node(&mut self, world: GVec2) {
        self.pending_vertices.push(world);
    }

    fn tool_delete_node(&mut self, screen_pos: Pos2, canvas_center: Pos2) {
        if let Some((zi, vi)) = self.hit_vertex(screen_pos, canvas_center) {
            let vertex = self.zones[zi].vertices[vi];
            self.execute_cmd(BlueprintCommand::RemoveVertex {
                zone_index: zi,
                vertex_index: vi,
                vertex,
            });
        }
    }

    fn tool_connect_nodes(&mut self, screen_pos: Pos2, canvas_center: Pos2) {
        if self.pending_vertices.len() >= 3 {
            // Check if clicking near first vertex
            let first_sp = self
                .camera
                .world_to_screen(self.pending_vertices[0], canvas_center);
            let d2 = (first_sp.x - screen_pos.x).powi(2) + (first_sp.y - screen_pos.y).powi(2);
            if d2 <= self.vertex_hit_radius * self.vertex_hit_radius {
                // Close polygon — create zone
                let zone = ZoneState {
                    name: format!("Zone {}", self.next_zone_id),
                    vertices: std::mem::take(&mut self.pending_vertices),
                    ..Default::default()
                };
                self.next_zone_id += 1;
                self.execute_cmd(BlueprintCommand::AddZone { zone });
                return;
            }
        }
        // Otherwise, just add vertex
        if let Some(pos) = Some(self.camera.screen_to_world(screen_pos, canvas_center)) {
            self.pending_vertices.push(pos);
        }
    }

    // ---- sidebar ----

    fn show_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            for tool in BlueprintTool::all() {
                let selected = self.tool == *tool;
                if ui
                    .selectable_label(selected, format!("{} {}", tool.icon(), tool.name()))
                    .clicked()
                {
                    self.tool = *tool;
                    if *tool != BlueprintTool::PlaceNode && *tool != BlueprintTool::ConnectNodes {
                        self.pending_vertices.clear();
                    }
                }
            }
            ui.separator();
            // Undo is available when there are pending vertices OR undo stack entries
            let can_undo = !self.pending_vertices.is_empty() || self.undo_stack.can_undo();
            if ui
                .add_enabled(can_undo, egui::Button::new("Undo"))
                .clicked()
            {
                self.undo();
            }
            if ui
                .add_enabled(self.undo_stack.can_redo(), egui::Button::new("Redo"))
                .clicked()
            {
                self.redo();
            }
        });
    }

    fn show_zone_list(&mut self, ui: &mut Ui) {
        ui.heading("Zones");
        let mut to_remove: Option<usize> = None;
        let zone_count = self.zones.len();
        for i in 0..zone_count {
            let is_selected = self.selected_zone == Some(i);
            ui.horizontal(|ui| {
                let label = format!(
                    "{} {} [pri:{}]",
                    if self.zones[i].enabled { "[v]" } else { "[ ]" },
                    self.zones[i].name,
                    self.zones[i].priority,
                );
                if ui.selectable_label(is_selected, label).clicked() {
                    self.selected_zone = Some(i);
                }
                if ui.small_button("X").clicked() {
                    to_remove = Some(i);
                }
            });
        }
        if let Some(idx) = to_remove {
            let zone = self.zones[idx].clone();
            self.execute_cmd(BlueprintCommand::RemoveZone { index: idx, zone });
        }

        ui.separator();

        // Finalize pending vertices as a new zone
        if !self.pending_vertices.is_empty() {
            let count = self.pending_vertices.len();
            ui.label(format!("Pending: {} vertices", count));
            if count >= 3 && ui.button("Close Polygon").clicked() {
                let zone_name = format!("Zone {}", self.next_zone_id);
                info!(zone = %zone_name, vertices = count, "Blueprint: created zone");
                let zone = ZoneState {
                    name: zone_name,
                    vertices: std::mem::take(&mut self.pending_vertices),
                    ..Default::default()
                };
                self.next_zone_id += 1;
                self.execute_cmd(BlueprintCommand::AddZone { zone });
            }
            if ui.button("Cancel").clicked() {
                self.pending_vertices.clear();
            }
        } else if ui.button("New Zone (Place Nodes)").clicked() {
            self.tool = BlueprintTool::PlaceNode;
        }
    }

    fn show_zone_inspector(&mut self, ui: &mut Ui) {
        let idx = match self.selected_zone {
            Some(i) if i < self.zones.len() => i,
            _ => {
                ui.label("No zone selected");
                return;
            }
        };

        ui.heading("Zone Inspector");
        ui.separator();

        // Name
        let old_name = self.zones[idx].name.clone();
        let mut name = old_name.clone();
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut name);
        });
        if name != old_name {
            info!(zone_index = idx, new_name = %name, "Blueprint: renamed zone");
            self.execute_cmd(BlueprintCommand::UpdateZoneName {
                zone_index: idx,
                old_name,
                new_name: name,
            });
        }

        // Priority
        let old_pri = self.zones[idx].priority;
        let mut pri = old_pri;
        ui.horizontal(|ui| {
            ui.label("Priority:");
            ui.add(egui::DragValue::new(&mut pri).range(-100..=100));
        });
        if pri != old_pri {
            self.execute_cmd(BlueprintCommand::UpdateZonePriority {
                zone_index: idx,
                old_priority: old_pri,
                new_priority: pri,
            });
        }

        // Enabled toggle
        let mut enabled = self.zones[idx].enabled;
        ui.checkbox(&mut enabled, "Enabled");
        self.zones[idx].enabled = enabled;

        // Blend margin
        ui.separator();
        let old_margin = self.zones[idx].blend_margin;
        let mut margin = old_margin;
        ui.horizontal(|ui| {
            ui.label("Blend Margin:");
            ui.add(egui::Slider::new(&mut margin, 0.0..=64.0).text("units"));
        });
        if (margin - old_margin).abs() > 0.01 {
            self.execute_cmd(BlueprintCommand::UpdateBlendMargin {
                zone_index: idx,
                old_margin,
                new_margin: margin,
            });
        }

        // Scene Scale override (only for BlendScene zones)
        if matches!(self.zones[idx].source, ZoneSourceState::BlendScene { .. }) {
            ui.separator();
            ui.label(RichText::new("Scene Scale").strong());
            let mut use_auto = self.zones[idx].scene_scale_override.is_none();
            ui.checkbox(&mut use_auto, "Auto scale");
            if use_auto && self.zones[idx].scene_scale_override.is_some() {
                self.zones[idx].scene_scale_override = None;
            } else if !use_auto && self.zones[idx].scene_scale_override.is_none() {
                self.zones[idx].scene_scale_override = Some(1.0);
            }
            if let Some(ref mut scale) = self.zones[idx].scene_scale_override {
                ui.horizontal(|ui| {
                    ui.label("Scale:");
                    ui.add(
                        egui::Slider::new(scale, 0.25..=4.0)
                            .logarithmic(true)
                            .text("x"),
                    );
                });
                ui.label(
                    RichText::new(format!(
                        "1.0x = exact replica, <1.0 = compress, >1.0 = expand"
                    ))
                    .weak()
                    .size(10.0),
                );
            }
        }

        // Source type selector
        ui.separator();
        ui.label(RichText::new("Source").strong());
        let is_biome = matches!(self.zones[idx].source, ZoneSourceState::BiomePreset(_));
        ui.horizontal(|ui| {
            if ui.selectable_label(is_biome, "Biome Preset").clicked() && !is_biome {
                let old = self.zones[idx].source.clone();
                self.execute_cmd(BlueprintCommand::UpdateZoneSource {
                    zone_index: idx,
                    old_source: old,
                    new_source: ZoneSourceState::BiomePreset("Grassland".into()),
                });
            }
            if ui.selectable_label(!is_biome, "Blend Scene").clicked() && is_biome {
                let old = self.zones[idx].source.clone();
                self.execute_cmd(BlueprintCommand::UpdateZoneSource {
                    zone_index: idx,
                    old_source: old,
                    new_source: ZoneSourceState::BlendScene {
                        pack_path: String::new(),
                        replica: true,
                    },
                });
            }
        });

        // Source-specific UI
        match &self.zones[idx].source {
            ZoneSourceState::BiomePreset(biome) => {
                let biome = biome.clone();
                let all_options = crate::terrain_integration::cached_biome_options();
                ui.horizontal(|ui| {
                    ui.label("Biome:");
                    egui::ComboBox::from_id_salt("biome_select")
                        .selected_text(&biome)
                        .show_ui(ui, |ui| {
                            for opt in &all_options {
                                let is_selected = opt.display == biome || opt.value == biome;
                                if ui.selectable_label(is_selected, &opt.display).clicked()
                                    && !is_selected
                                {
                                    let old = ZoneSourceState::BiomePreset(biome.clone());
                                    // Pack options use "pack:path" format → BlendScene source
                                    let new =
                                        if let Some(pack_path) = opt.value.strip_prefix("pack:") {
                                            ZoneSourceState::BlendScene {
                                                pack_path: pack_path.to_string(),
                                                replica: false,
                                            }
                                        } else {
                                            ZoneSourceState::BiomePreset(opt.display.clone())
                                        };
                                    self.zones[idx].source = new.clone();
                                    self.undo_stack.push(BlueprintCommand::UpdateZoneSource {
                                        zone_index: idx,
                                        old_source: old,
                                        new_source: new,
                                    });
                                }
                            }
                        });
                });
            }
            ZoneSourceState::BlendScene { pack_path, replica } => {
                let pack_path = pack_path.clone();
                let replica = *replica;

                // Show biome/pack selector — user can switch to built-in or another pack
                let all_options = crate::terrain_integration::cached_biome_options();
                let current_display = all_options
                    .iter()
                    .find(|o| {
                        o.value
                            .strip_prefix("pack:")
                            .is_some_and(|p| p == pack_path)
                    })
                    .map(|o| o.display.clone())
                    .unwrap_or_else(|| {
                        std::path::Path::new(&pack_path)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Pack")
                            .to_string()
                    });
                ui.horizontal(|ui| {
                    ui.label("Source:");
                    egui::ComboBox::from_id_salt("blend_biome_select")
                        .selected_text(&current_display)
                        .show_ui(ui, |ui| {
                            for opt in &all_options {
                                let is_selected = opt
                                    .value
                                    .strip_prefix("pack:")
                                    .is_some_and(|p| p == pack_path);
                                if ui.selectable_label(is_selected, &opt.display).clicked()
                                    && !is_selected
                                {
                                    let old = self.zones[idx].source.clone();
                                    let new = if let Some(pp) = opt.value.strip_prefix("pack:") {
                                        ZoneSourceState::BlendScene {
                                            pack_path: pp.to_string(),
                                            replica,
                                        }
                                    } else {
                                        ZoneSourceState::BiomePreset(opt.display.clone())
                                    };
                                    self.zones[idx].source = new.clone();
                                    self.undo_stack.push(BlueprintCommand::UpdateZoneSource {
                                        zone_index: idx,
                                        old_source: old,
                                        new_source: new,
                                    });
                                }
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Mode:");
                    let mut is_replica = replica;
                    ui.selectable_value(&mut is_replica, true, "Replica");
                    ui.selectable_value(&mut is_replica, false, "Inspired");
                    if is_replica != replica {
                        let old = self.zones[idx].source.clone();
                        self.zones[idx].source = ZoneSourceState::BlendScene {
                            pack_path: pack_path.clone(),
                            replica: is_replica,
                        };
                        self.undo_stack.push(BlueprintCommand::UpdateZoneSource {
                            zone_index: idx,
                            old_source: old,
                            new_source: self.zones[idx].source.clone(),
                        });
                    }
                });
            }
        }

        // Vertex list
        ui.separator();
        ui.label(RichText::new(format!("Vertices ({})", self.zones[idx].vertices.len())).strong());
        let verts = self.zones[idx].vertices.clone();
        for (vi, v) in verts.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("  #{}: ({:.1}, {:.1})", vi, v.x, v.y));
            });
        }

        // Generate button
        ui.separator();
        if ui.button("Generate Zone").clicked() {
            info!(zone_index = idx, "Blueprint: generating zone");
            self.pending_actions
                .push(BlueprintAction::GenerateZone { zone_index: idx });
        }
    }

    fn show_actions_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.zones.is_empty(), egui::Button::new("Generate All"))
                .clicked()
            {
                info!(
                    zone_count = self.zones.len(),
                    "Blueprint: generating all zones"
                );
                self.pending_actions.push(BlueprintAction::GenerateAll);
            }
            if ui.button("Clear Generation").clicked() {
                info!("Blueprint: clearing generation");
                self.pending_actions.push(BlueprintAction::ClearGeneration);
            }
            ui.separator();
            if ui.button("Save Zones").clicked() {
                info!("Blueprint: saving zones");
                self.pending_actions.push(BlueprintAction::SaveZones);
            }
            if ui.button("Load Zones").clicked() {
                info!("Blueprint: loading zones");
                self.pending_actions.push(BlueprintAction::LoadZones);
            }
            ui.separator();
            ui.checkbox(&mut self.show_grid, "Grid");
            ui.add(
                egui::DragValue::new(&mut self.grid_spacing)
                    .range(8.0..=512.0)
                    .prefix("Grid: "),
            );
            ui.label(format!("Zoom: {:.1}x", self.camera.zoom));
        });
    }
}

// ============================================================================
// Panel trait impl
// ============================================================================

impl Panel for BlueprintPanel {
    fn name(&self) -> &str {
        "Blueprint"
    }

    fn show(&mut self, ui: &mut Ui) {
        // Toolbar at top
        self.show_toolbar(ui);
        ui.separator();

        // Actions bar
        self.show_actions_bar(ui);
        ui.separator();

        // Split: left = canvas, right = inspector
        let available = ui.available_size();
        let inspector_width = 260.0_f32.min(available.x * 0.35);
        let canvas_width = available.x - inspector_width - 8.0;

        ui.horizontal(|ui| {
            // Canvas area
            let (response, painter) = ui.allocate_painter(
                Vec2::new(canvas_width, available.y),
                Sense::click_and_drag(),
            );

            let canvas_rect = response.rect;
            let canvas_center = canvas_rect.center();

            // Draw background
            painter.rect_filled(canvas_rect, 0.0, Color32::from_rgb(30, 30, 35));

            // Draw grid
            self.draw_grid(&painter, canvas_rect, canvas_center);

            // Draw zones
            self.draw_zones(&painter, canvas_center);

            // Draw pending vertices
            self.draw_pending(&painter, canvas_center);

            // Handle input
            self.handle_canvas_input(&response, canvas_center);

            ui.separator();

            // Inspector sidebar
            ui.vertical(|ui| {
                ui.set_width(inspector_width);
                self.show_zone_list(ui);
                ui.separator();
                self.show_zone_inspector(ui);
            });
        });
    }
}

// ============================================================================
// Point-in-polygon (simple ray cast)
// ============================================================================

fn point_in_polygon_simple(point: GVec2, vertices: &[GVec2]) -> bool {
    let n = vertices.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let vi = vertices[i];
        let vj = vertices[j];

        if ((vi.y > point.y) != (vj.y > point.y))
            && (point.x < (vj.x - vi.x) * (point.y - vi.y) / (vj.y - vi.y) + vi.x)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_camera_roundtrip() {
        let cam = CanvasCamera {
            center: GVec2::new(100.0, 200.0),
            zoom: 2.0,
        };
        let canvas_center = Pos2::new(400.0, 300.0);
        let world = GVec2::new(150.0, 250.0);
        let screen = cam.world_to_screen(world, canvas_center);
        let back = cam.screen_to_world(screen, canvas_center);
        assert!((back.x - world.x).abs() < 0.01);
        assert!((back.y - world.y).abs() < 0.01);
    }

    #[test]
    fn test_canvas_camera_origin() {
        let cam = CanvasCamera::default();
        let center = Pos2::new(400.0, 300.0);
        let screen = cam.world_to_screen(GVec2::ZERO, center);
        assert!((screen.x - 400.0).abs() < 0.01);
        assert!((screen.y - 300.0).abs() < 0.01);
    }

    #[test]
    fn test_point_in_polygon_simple_square() {
        let verts = vec![
            GVec2::new(0.0, 0.0),
            GVec2::new(10.0, 0.0),
            GVec2::new(10.0, 10.0),
            GVec2::new(0.0, 10.0),
        ];
        assert!(point_in_polygon_simple(GVec2::new(5.0, 5.0), &verts));
        assert!(!point_in_polygon_simple(GVec2::new(15.0, 5.0), &verts));
    }

    #[test]
    fn test_point_in_polygon_simple_triangle() {
        let verts = vec![
            GVec2::new(0.0, 0.0),
            GVec2::new(20.0, 0.0),
            GVec2::new(10.0, 20.0),
        ];
        assert!(point_in_polygon_simple(GVec2::new(10.0, 5.0), &verts));
        assert!(!point_in_polygon_simple(GVec2::new(0.0, 20.0), &verts));
    }

    #[test]
    fn test_zone_creation_and_undo() {
        let mut panel = BlueprintPanel::new();
        assert_eq!(panel.zones.len(), 0);

        let zone = ZoneState {
            name: "Test".into(),
            vertices: vec![
                GVec2::new(0.0, 0.0),
                GVec2::new(10.0, 0.0),
                GVec2::new(10.0, 10.0),
            ],
            ..Default::default()
        };
        panel.execute_cmd(BlueprintCommand::AddZone { zone });
        assert_eq!(panel.zones.len(), 1);
        assert_eq!(panel.zones[0].name, "Test");

        panel.undo();
        assert_eq!(panel.zones.len(), 0);

        panel.redo();
        assert_eq!(panel.zones.len(), 1);
        assert_eq!(panel.zones[0].name, "Test");
    }

    #[test]
    fn test_zone_remove_and_undo() {
        let mut panel = BlueprintPanel::new();
        let zone = ZoneState {
            name: "ToRemove".into(),
            vertices: vec![GVec2::ZERO, GVec2::new(1.0, 0.0), GVec2::new(0.0, 1.0)],
            ..Default::default()
        };
        panel.execute_cmd(BlueprintCommand::AddZone { zone: zone.clone() });
        assert_eq!(panel.zones.len(), 1);

        panel.execute_cmd(BlueprintCommand::RemoveZone { index: 0, zone });
        assert_eq!(panel.zones.len(), 0);

        panel.undo();
        assert_eq!(panel.zones.len(), 1);
        assert_eq!(panel.zones[0].name, "ToRemove");
    }

    #[test]
    fn test_vertex_add_remove_undo() {
        let mut panel = BlueprintPanel::new();
        let zone = ZoneState {
            name: "Base".into(),
            vertices: vec![GVec2::ZERO, GVec2::new(5.0, 0.0), GVec2::new(5.0, 5.0)],
            ..Default::default()
        };
        panel.execute_cmd(BlueprintCommand::AddZone { zone });

        // Add vertex
        panel.execute_cmd(BlueprintCommand::AddVertex {
            zone_index: 0,
            vertex: GVec2::new(0.0, 5.0),
        });
        assert_eq!(panel.zones[0].vertices.len(), 4);

        // Undo add
        panel.undo();
        assert_eq!(panel.zones[0].vertices.len(), 3);

        // Redo add
        panel.redo();
        assert_eq!(panel.zones[0].vertices.len(), 4);

        // Remove vertex
        panel.execute_cmd(BlueprintCommand::RemoveVertex {
            zone_index: 0,
            vertex_index: 1,
            vertex: GVec2::new(5.0, 0.0),
        });
        assert_eq!(panel.zones[0].vertices.len(), 3);

        // Undo remove
        panel.undo();
        assert_eq!(panel.zones[0].vertices.len(), 4);
    }

    #[test]
    fn test_move_vertex_undo() {
        let mut panel = BlueprintPanel::new();
        let zone = ZoneState {
            name: "MoveTest".into(),
            vertices: vec![GVec2::ZERO, GVec2::new(10.0, 0.0), GVec2::new(5.0, 10.0)],
            ..Default::default()
        };
        panel.execute_cmd(BlueprintCommand::AddZone { zone });

        let old = GVec2::new(10.0, 0.0);
        let new = GVec2::new(12.0, 3.0);
        panel.execute_cmd(BlueprintCommand::MoveVertex {
            zone_index: 0,
            vertex_index: 1,
            old_pos: old,
            new_pos: new,
        });
        assert!((panel.zones[0].vertices[1] - new).length() < 0.01);

        panel.undo();
        assert!((panel.zones[0].vertices[1] - old).length() < 0.01);
    }

    #[test]
    fn test_update_name_undo() {
        let mut panel = BlueprintPanel::new();
        let zone = ZoneState::default();
        panel.execute_cmd(BlueprintCommand::AddZone { zone });

        panel.execute_cmd(BlueprintCommand::UpdateZoneName {
            zone_index: 0,
            old_name: "New Zone".into(),
            new_name: "Forest Edge".into(),
        });
        assert_eq!(panel.zones[0].name, "Forest Edge");

        panel.undo();
        assert_eq!(panel.zones[0].name, "New Zone");
    }

    #[test]
    fn test_update_priority_undo() {
        let mut panel = BlueprintPanel::new();
        let zone = ZoneState::default();
        panel.execute_cmd(BlueprintCommand::AddZone { zone });

        panel.execute_cmd(BlueprintCommand::UpdateZonePriority {
            zone_index: 0,
            old_priority: 0,
            new_priority: 5,
        });
        assert_eq!(panel.zones[0].priority, 5);

        panel.undo();
        assert_eq!(panel.zones[0].priority, 0);
    }

    #[test]
    fn test_update_source_undo() {
        let mut panel = BlueprintPanel::new();
        let zone = ZoneState::default();
        panel.execute_cmd(BlueprintCommand::AddZone { zone });

        let old = ZoneSourceState::BiomePreset("Grassland".into());
        let new = ZoneSourceState::BlendScene {
            pack_path: "pack/forest".into(),
            replica: true,
        };
        panel.execute_cmd(BlueprintCommand::UpdateZoneSource {
            zone_index: 0,
            old_source: old.clone(),
            new_source: new.clone(),
        });
        assert_eq!(panel.zones[0].source, new);

        panel.undo();
        assert_eq!(panel.zones[0].source, old);
    }

    #[test]
    fn test_blend_margin_undo() {
        let mut panel = BlueprintPanel::new();
        let zone = ZoneState::default();
        panel.execute_cmd(BlueprintCommand::AddZone { zone });

        panel.execute_cmd(BlueprintCommand::UpdateBlendMargin {
            zone_index: 0,
            old_margin: 8.0,
            new_margin: 20.0,
        });
        assert!((panel.zones[0].blend_margin - 20.0).abs() < 0.01);

        panel.undo();
        assert!((panel.zones[0].blend_margin - 8.0).abs() < 0.01);
    }

    #[test]
    fn test_zone_colors_by_source() {
        let biome_zone = ZoneState {
            source: ZoneSourceState::BiomePreset("Forest".into()),
            ..Default::default()
        };
        let blend_zone = ZoneState {
            source: ZoneSourceState::BlendScene {
                pack_path: "x".into(),
                replica: false,
            },
            ..Default::default()
        };
        // Just verify they produce different colors (green vs blue)
        assert_ne!(biome_zone.color(), blend_zone.color());
        assert_ne!(biome_zone.outline_color(), blend_zone.outline_color());
    }

    #[test]
    fn test_tool_enum_coverage() {
        assert_eq!(BlueprintTool::all().len(), 5);
        for tool in BlueprintTool::all() {
            assert!(!tool.name().is_empty());
            assert!(!tool.icon().is_empty());
        }
    }

    #[test]
    fn test_hit_zone() {
        let mut panel = BlueprintPanel::new();
        let zone = ZoneState {
            name: "HitTest".into(),
            vertices: vec![
                GVec2::new(0.0, 0.0),
                GVec2::new(100.0, 0.0),
                GVec2::new(100.0, 100.0),
                GVec2::new(0.0, 100.0),
            ],
            ..Default::default()
        };
        panel.execute_cmd(BlueprintCommand::AddZone { zone });

        assert_eq!(panel.hit_zone(GVec2::new(50.0, 50.0)), Some(0));
        assert_eq!(panel.hit_zone(GVec2::new(200.0, 200.0)), None);
    }

    #[test]
    fn test_take_actions() {
        let mut panel = BlueprintPanel::new();
        panel.pending_actions.push(BlueprintAction::GenerateAll);
        panel.pending_actions.push(BlueprintAction::SaveZones);

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(panel.take_actions().is_empty());
    }

    #[test]
    fn test_set_zones() {
        let mut panel = BlueprintPanel::new();
        panel.selected_zone = Some(0);
        panel.pending_vertices.push(GVec2::new(1.0, 2.0));

        let zones = vec![ZoneState::default(), ZoneState::default()];
        panel.set_zones(zones);
        assert_eq!(panel.zones.len(), 2);
        assert!(panel.selected_zone.is_none());
        assert!(panel.pending_vertices.is_empty());
    }
}
