//! Visual node graph widget for behavior trees.
//!
//! Renders `BehaviorGraphDocument` nodes as colored boxes with ports and Bezier wires,
//! supporting drag-to-move, drag-to-connect, selection, and context menus.

use egui::{
    self, epaint::CubicBezierShape, Color32, CursorIcon, Id, Pos2, Rect, RichText, Sense, Stroke,
    Vec2,
};

use super::document::{
    BehaviorGraphDocument, BehaviorGraphNodeKind, DecoratorKind, DecoratorNode, NodeId,
    NodePosition,
};

// ────────────────────── Constants ──────────────────────

const NODE_WIDTH: f32 = 160.0;
const NODE_HEADER_HEIGHT: f32 = 28.0;
const NODE_BODY_HEIGHT: f32 = 24.0;
const NODE_CORNER_RADIUS: f32 = 6.0;
const PORT_RADIUS: f32 = 6.0;
const PORT_HIT_RADIUS: f32 = 10.0;
const WIRE_THICKNESS: f32 = 2.5;
const GRID_SPACING: f32 = 20.0;
const GRID_COLOR: Color32 = Color32::from_rgba_premultiplied(40, 40, 45, 60);
const ROOT_BADGE_COLOR: Color32 = Color32::from_rgb(255, 200, 50);

// ────────────────────── Node colors ──────────────────────

fn node_header_color(kind: &BehaviorGraphNodeKind) -> Color32 {
    match kind {
        BehaviorGraphNodeKind::Action { .. } => Color32::from_rgb(50, 100, 180), // Blue
        BehaviorGraphNodeKind::Condition { .. } => Color32::from_rgb(50, 150, 80), // Green
        BehaviorGraphNodeKind::Sequence { .. } => Color32::from_rgb(200, 120, 40), // Orange
        BehaviorGraphNodeKind::Selector { .. } => Color32::from_rgb(140, 70, 170), // Purple
        BehaviorGraphNodeKind::Parallel { .. } => Color32::from_rgb(170, 60, 60), // Red
        BehaviorGraphNodeKind::Decorator(_) => Color32::from_rgb(180, 160, 50),  // Yellow
    }
}

const NODE_BODY_COLOR: Color32 = Color32::from_rgb(45, 45, 50);
const NODE_BODY_SELECTED: Color32 = Color32::from_rgb(55, 55, 65);
const SELECTION_OUTLINE: Color32 = Color32::from_rgb(80, 160, 255);
const PORT_COLOR: Color32 = Color32::from_rgb(180, 180, 190);
const PORT_HOVER_COLOR: Color32 = Color32::from_rgb(100, 200, 255);
const WIRE_COLOR: Color32 = Color32::from_rgb(160, 160, 170);
const WIRE_ACTIVE_COLOR: Color32 = Color32::from_rgb(100, 200, 255);

// ────────────────────── Port types ──────────────────────

/// Which port on a node (input = top, output = bottom).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PortKind {
    /// Input port (top of node). Receives connection from parent.
    Input,
    /// Output port (bottom of node). Sends connection to children.
    /// Index gives position for composites with multiple outputs.
    Output(usize),
}

/// A reference to a specific port on a specific node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PortRef {
    node_id: NodeId,
    port: PortKind,
}

/// Node and port under the pointer for one interaction pass.
struct PointerHit {
    node: Option<NodeId>,
    port: Option<PortRef>,
}

// ────────────────────── Interaction state ──────────────────────

/// Tracks in-progress drag-to-connect wire.
#[derive(Debug, Clone, Copy)]
struct PendingWire {
    from: PortRef,
    /// Current mouse position (endpoint of the in-progress wire).
    mouse_pos: Pos2,
}

/// Tracks node drag state.
#[derive(Debug, Clone, Copy)]
struct NodeDrag {
    node_id: NodeId,
    /// Offset from node origin to mouse at drag start.
    offset: Vec2,
}

/// Context menu state
#[derive(Debug, Clone)]
enum ContextMenu {
    /// Right-clicked on empty canvas
    Canvas { pos: Pos2 },
    /// Right-clicked on a node
    Node { node_id: NodeId },
}

// ────────────────────── Main widget ──────────────────────

/// Visual node graph editor widget.
pub struct NodeGraphWidget {
    /// Currently selected node.
    pub selected_node: Option<NodeId>,
    /// Pan offset (scrolling the canvas).
    pan_offset: Vec2,
    /// In-progress wire connection.
    pending_wire: Option<PendingWire>,
    /// In-progress node drag.
    node_drag: Option<NodeDrag>,
    /// Context menu state.
    context_menu: Option<ContextMenu>,
    /// Zoom level (future use).
    _zoom: f32,
    /// Status message for user feedback.
    pub status: Option<String>,
}

impl Default for NodeGraphWidget {
    fn default() -> Self {
        Self {
            selected_node: None,
            pan_offset: Vec2::ZERO,
            pending_wire: None,
            node_drag: None,
            context_menu: None,
            _zoom: 1.0,
            status: None,
        }
    }
}

impl NodeGraphWidget {
    /// Show the full node graph for the given document.
    ///
    /// Returns true if the document was modified (dirty).
    pub fn show(&mut self, ui: &mut egui::Ui, doc: &mut BehaviorGraphDocument) -> bool {
        let mut changed = false;

        // Allocate a large interactive area for the canvas
        let (canvas_rect, canvas_response) =
            ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());

        // Handle canvas panning (middle-mouse drag or right-drag on empty space)
        if canvas_response.dragged_by(egui::PointerButton::Middle) {
            self.pan_offset += canvas_response.drag_delta();
        }

        let painter = ui.painter_at(canvas_rect);

        // Clip to canvas
        painter.rect_filled(canvas_rect, 0.0, Color32::from_rgb(30, 30, 35));

        // Draw grid
        self.draw_grid(&painter, canvas_rect);

        // Build node layout info: mapping from NodeId → screen Rect
        let node_rects = self.compute_node_rects(doc, canvas_rect);

        // Draw wires first (behind nodes)
        self.draw_wires(&painter, doc, &node_rects, canvas_rect);

        // Draw pending wire (drag-to-connect in progress)
        if let Some(ref wire) = self.pending_wire {
            let start = self.port_screen_pos(wire.from, doc, &node_rects, canvas_rect);
            if let Some(start) = start {
                draw_bezier_wire(&painter, start, wire.mouse_pos, WIRE_ACTIVE_COLOR);
            }
        }

        // Draw nodes
        let mut click_hit_node = None;
        let mut port_hit: Option<PortRef> = None;

        for node in doc.nodes() {
            if let Some(&node_rect) = node_rects.get(&node.id) {
                let is_selected = self.selected_node == Some(node.id);
                let is_root = node.id == doc.root_id();

                self.draw_node(&painter, node, node_rect, is_selected, is_root);

                // Check port hover/click
                let input_pos = port_pos_input(node_rect);
                let output_positions = self.output_port_positions(node, node_rect);

                if let Some(mouse) = ui.ctx().pointer_hover_pos() {
                    // Input port hover
                    if (mouse - input_pos).length() < PORT_HIT_RADIUS && canvas_rect.contains(mouse)
                    {
                        let pref = PortRef {
                            node_id: node.id,
                            port: PortKind::Input,
                        };
                        painter.circle_filled(input_pos, PORT_RADIUS, PORT_HOVER_COLOR);
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                        port_hit = Some(pref);
                    }
                    // Output port hover
                    for (i, &opos) in output_positions.iter().enumerate() {
                        if (mouse - opos).length() < PORT_HIT_RADIUS && canvas_rect.contains(mouse)
                        {
                            let pref = PortRef {
                                node_id: node.id,
                                port: PortKind::Output(i),
                            };
                            painter.circle_filled(opos, PORT_RADIUS, PORT_HOVER_COLOR);
                            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                            port_hit = Some(pref);
                        }
                    }
                }

                // Node body click detection
                if let Some(mouse) = canvas_response.interact_pointer_pos() {
                    if node_rect.contains(mouse) {
                        click_hit_node = Some(node.id);
                    }
                }
            }
        }

        // Handle interactions
        changed |= self.handle_interactions(
            ui,
            doc,
            &canvas_response,
            canvas_rect,
            &node_rects,
            PointerHit {
                node: click_hit_node,
                port: port_hit,
            },
        );

        // Context menu
        changed |= self.handle_context_menu(ui, doc, &canvas_response, canvas_rect);

        changed
    }

    // ────────────────── Grid ──────────────────

    fn draw_grid(&self, painter: &egui::Painter, rect: Rect) {
        let offset = self.pan_offset;
        let start_x = rect.min.x + (offset.x % GRID_SPACING);
        let start_y = rect.min.y + (offset.y % GRID_SPACING);

        let mut x = start_x;
        while x < rect.max.x {
            painter.line_segment(
                [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                Stroke::new(1.0, GRID_COLOR),
            );
            x += GRID_SPACING;
        }
        let mut y = start_y;
        while y < rect.max.y {
            painter.line_segment(
                [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                Stroke::new(1.0, GRID_COLOR),
            );
            y += GRID_SPACING;
        }
    }

    // ────────────────── Node rects ──────────────────

    fn compute_node_rects(
        &self,
        doc: &BehaviorGraphDocument,
        canvas_rect: Rect,
    ) -> std::collections::HashMap<NodeId, Rect> {
        let mut rects = std::collections::HashMap::new();
        let origin = canvas_rect.left_top().to_vec2() + self.pan_offset;

        for node in doc.nodes() {
            let top_left = Pos2::new(node.position.x + origin.x, node.position.y + origin.y);
            let height = NODE_HEADER_HEIGHT + NODE_BODY_HEIGHT;
            let rect = Rect::from_min_size(top_left, Vec2::new(NODE_WIDTH, height));
            rects.insert(node.id, rect);
        }
        rects
    }

    // ────────────────── Draw node ──────────────────

    fn draw_node(
        &self,
        painter: &egui::Painter,
        node: &super::document::BehaviorGraphNode,
        rect: Rect,
        is_selected: bool,
        is_root: bool,
    ) {
        let header_rect = Rect::from_min_size(rect.min, Vec2::new(NODE_WIDTH, NODE_HEADER_HEIGHT));
        let body_rect = Rect::from_min_size(
            Pos2::new(rect.min.x, rect.min.y + NODE_HEADER_HEIGHT),
            Vec2::new(NODE_WIDTH, NODE_BODY_HEIGHT),
        );

        // Selection outline (drawn first, slightly larger)
        if is_selected {
            painter.rect_filled(
                rect.expand(2.0),
                NODE_CORNER_RADIUS + 2.0,
                SELECTION_OUTLINE,
            );
        }

        // Header background
        let header_color = node_header_color(&node.kind);
        painter.rect_filled(header_rect, NODE_CORNER_RADIUS, header_color);
        // Flatten bottom corners of header by drawing a small rect
        let join_rect = Rect::from_min_size(
            Pos2::new(header_rect.min.x, header_rect.max.y - NODE_CORNER_RADIUS),
            Vec2::new(NODE_WIDTH, NODE_CORNER_RADIUS),
        );
        painter.rect_filled(join_rect, 0.0, header_color);

        // Body background
        let body_color = if is_selected {
            NODE_BODY_SELECTED
        } else {
            NODE_BODY_COLOR
        };
        painter.rect_filled(body_rect, 0.0, body_color);
        // Round bottom corners
        let bottom_rect = Rect::from_min_size(
            Pos2::new(body_rect.min.x, body_rect.max.y - NODE_CORNER_RADIUS),
            Vec2::new(NODE_WIDTH, NODE_CORNER_RADIUS),
        );
        painter.rect_filled(bottom_rect, NODE_CORNER_RADIUS, body_color);

        // Root badge
        if is_root {
            let badge_pos = Pos2::new(rect.max.x - 8.0, rect.min.y + 8.0);
            painter.circle_filled(badge_pos, 5.0, ROOT_BADGE_COLOR);
        }

        // Header text: icon + label
        let icon = node.kind.icon();
        let header_text = format!("{} {}", icon, node.label);
        painter.text(
            Pos2::new(header_rect.min.x + 8.0, header_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &header_text,
            egui::FontId::proportional(12.0),
            Color32::WHITE,
        );

        // Body text: kind-specific info
        let body_text = match &node.kind {
            BehaviorGraphNodeKind::Action { name } => format!("⚡ {}", name),
            BehaviorGraphNodeKind::Condition { name } => format!("❓ {}", name),
            BehaviorGraphNodeKind::Sequence { children } => {
                format!("→ {} children", children.len())
            }
            BehaviorGraphNodeKind::Selector { children } => {
                format!("⑂ {} children", children.len())
            }
            BehaviorGraphNodeKind::Parallel {
                children,
                success_threshold,
            } => {
                format!("∥ {} / {}", success_threshold, children.len())
            }
            BehaviorGraphNodeKind::Decorator(d) => format!("◇ {}", d.decorator),
        };
        painter.text(
            Pos2::new(body_rect.min.x + 8.0, body_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &body_text,
            egui::FontId::proportional(11.0),
            Color32::from_rgb(180, 180, 190),
        );

        // Input port (top center) — not for root node
        let input_pos = port_pos_input(rect);
        painter.circle_filled(input_pos, PORT_RADIUS, PORT_COLOR);
        painter.circle_stroke(
            input_pos,
            PORT_RADIUS,
            Stroke::new(1.0, Color32::from_rgb(80, 80, 90)),
        );

        // Output ports (bottom) — only for nodes that can have children
        if node.kind.supports_children() {
            let positions = self.output_port_positions(node, rect);
            for pos in &positions {
                painter.circle_filled(*pos, PORT_RADIUS, PORT_COLOR);
                painter.circle_stroke(
                    *pos,
                    PORT_RADIUS,
                    Stroke::new(1.0, Color32::from_rgb(80, 80, 90)),
                );
            }
        }
    }

    // ────────────────── Port positions ──────────────────

    fn output_port_positions(
        &self,
        node: &super::document::BehaviorGraphNode,
        rect: Rect,
    ) -> Vec<Pos2> {
        if !node.kind.supports_children() {
            return Vec::new();
        }
        // Single output port at bottom center
        vec![Pos2::new(rect.center().x, rect.max.y)]
    }

    fn port_screen_pos(
        &self,
        port_ref: PortRef,
        doc: &BehaviorGraphDocument,
        node_rects: &std::collections::HashMap<NodeId, Rect>,
        _canvas_rect: Rect,
    ) -> Option<Pos2> {
        let rect = node_rects.get(&port_ref.node_id)?;
        match port_ref.port {
            PortKind::Input => Some(port_pos_input(*rect)),
            PortKind::Output(_) => {
                let node = doc.node(port_ref.node_id)?;
                let positions = self.output_port_positions(node, *rect);
                positions.first().copied()
            }
        }
    }

    // ────────────────── Wires ──────────────────

    fn draw_wires(
        &self,
        painter: &egui::Painter,
        doc: &BehaviorGraphDocument,
        node_rects: &std::collections::HashMap<NodeId, Rect>,
        _canvas_rect: Rect,
    ) {
        for node in doc.nodes() {
            let children: Vec<NodeId> = match &node.kind {
                BehaviorGraphNodeKind::Sequence { children }
                | BehaviorGraphNodeKind::Selector { children }
                | BehaviorGraphNodeKind::Parallel { children, .. } => children.clone(),
                BehaviorGraphNodeKind::Decorator(d) => d.child.into_iter().collect(),
                _ => Vec::new(),
            };

            if let Some(&parent_rect) = node_rects.get(&node.id) {
                let out_pos = Pos2::new(parent_rect.center().x, parent_rect.max.y);
                for child_id in &children {
                    if let Some(&child_rect) = node_rects.get(child_id) {
                        let in_pos = port_pos_input(child_rect);
                        draw_bezier_wire(painter, out_pos, in_pos, WIRE_COLOR);
                    }
                }
            }
        }
    }

    // ────────────────── Interactions ──────────────────

    fn handle_interactions(
        &mut self,
        ui: &egui::Ui,
        doc: &mut BehaviorGraphDocument,
        response: &egui::Response,
        canvas_rect: Rect,
        node_rects: &std::collections::HashMap<NodeId, Rect>,
        hit: PointerHit,
    ) -> bool {
        let click_hit_node = hit.node;
        let port_hit = hit.port;
        let mut changed = false;

        // Left-click on node → select
        if response.clicked() {
            if let Some(node_id) = click_hit_node {
                self.selected_node = Some(node_id);
            } else {
                // Clicked empty canvas → deselect
                self.selected_node = None;
            }
        }

        // Left-drag start
        if response.drag_started_by(egui::PointerButton::Primary) {
            if let Some(mouse) = response.interact_pointer_pos() {
                // Check if starting on a port
                if let Some(port_ref) = &port_hit {
                    // Only start wire from output ports
                    if matches!(port_ref.port, PortKind::Output(_)) {
                        self.pending_wire = Some(PendingWire {
                            from: *port_ref,
                            mouse_pos: mouse,
                        });
                    }
                } else if let Some(node_id) = click_hit_node {
                    // Start node drag
                    if let Some(&rect) = node_rects.get(&node_id) {
                        self.node_drag = Some(NodeDrag {
                            node_id,
                            offset: rect.min - mouse,
                        });
                        self.selected_node = Some(node_id);
                    }
                }
            }
        }

        // Drag in progress
        if response.dragged_by(egui::PointerButton::Primary) {
            if let Some(mouse) = ui.ctx().pointer_hover_pos() {
                // Update pending wire
                if let Some(ref mut wire) = self.pending_wire {
                    wire.mouse_pos = mouse;
                }
                // Update node drag
                if let Some(ref drag) = self.node_drag {
                    let new_pos = mouse + drag.offset;
                    let origin = canvas_rect.left_top().to_vec2() + self.pan_offset;
                    let doc_x = new_pos.x - origin.x;
                    let doc_y = new_pos.y - origin.y;
                    if let Some(node) = doc.node_mut(drag.node_id) {
                        node.position = NodePosition { x: doc_x, y: doc_y };
                        changed = true;
                    }
                }
            }
        }

        // Drag released
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            // Complete pending wire
            if let Some(wire) = self.pending_wire.take() {
                if let Some(target_port) = &port_hit {
                    if matches!(target_port.port, PortKind::Input)
                        && target_port.node_id != wire.from.node_id
                    {
                        // Connect: wire.from (output of parent) → target_port (input of child)
                        changed |= self.connect_nodes(doc, wire.from.node_id, target_port.node_id);
                    }
                }
            }
            self.node_drag = None;
        }

        // Delete key
        if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
            if let Some(node_id) = self.selected_node {
                if node_id != doc.root_id() {
                    if doc.remove_node(node_id).is_ok() {
                        self.selected_node = Some(doc.root_id());
                        self.status = Some("Node deleted".into());
                        changed = true;
                    }
                } else {
                    self.status = Some("Cannot delete root node".into());
                }
            }
        }

        changed
    }

    fn connect_nodes(
        &mut self,
        doc: &mut BehaviorGraphDocument,
        parent_id: NodeId,
        child_id: NodeId,
    ) -> bool {
        // Verify parent supports children and child exists
        let parent = match doc.node(parent_id) {
            Some(n) => n,
            None => return false,
        };
        if !parent.kind.supports_children() {
            self.status = Some("Node cannot have children".into());
            return false;
        }

        // Check if already connected
        let children = match &parent.kind {
            BehaviorGraphNodeKind::Sequence { children }
            | BehaviorGraphNodeKind::Selector { children }
            | BehaviorGraphNodeKind::Parallel { children, .. } => children.clone(),
            BehaviorGraphNodeKind::Decorator(d) => d.child.into_iter().collect(),
            _ => Vec::new(),
        };
        if children.contains(&child_id) {
            self.status = Some("Already connected".into());
            return false;
        }

        // For decorators, check if already has a child
        if let Some(node) = doc.node(parent_id) {
            if let BehaviorGraphNodeKind::Decorator(d) = &node.kind {
                if d.child.is_some() {
                    self.status = Some("Decorator already has a child".into());
                    return false;
                }
            }
        }

        // First, remove child from any existing parent
        self.disconnect_child_from_all_parents(doc, child_id);

        // Add child to parent
        if let Some(node) = doc.node_mut(parent_id) {
            match &mut node.kind {
                BehaviorGraphNodeKind::Sequence { children }
                | BehaviorGraphNodeKind::Selector { children }
                | BehaviorGraphNodeKind::Parallel { children, .. } => {
                    children.push(child_id);
                }
                BehaviorGraphNodeKind::Decorator(d) => {
                    d.child = Some(child_id);
                }
                _ => return false,
            }
        }

        self.status = Some("Connected".into());
        true
    }

    fn disconnect_child_from_all_parents(&self, doc: &mut BehaviorGraphDocument, child_id: NodeId) {
        // Iterate all nodes and remove child_id from their children lists
        let node_ids: Vec<NodeId> = doc.nodes().iter().map(|n| n.id).collect();
        for nid in node_ids {
            if let Some(node) = doc.node_mut(nid) {
                match &mut node.kind {
                    BehaviorGraphNodeKind::Sequence { children }
                    | BehaviorGraphNodeKind::Selector { children }
                    | BehaviorGraphNodeKind::Parallel { children, .. } => {
                        children.retain(|&c| c != child_id);
                    }
                    BehaviorGraphNodeKind::Decorator(d) => {
                        if d.child == Some(child_id) {
                            d.child = None;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // ────────────────── Context menu ──────────────────

    fn handle_context_menu(
        &mut self,
        ui: &egui::Ui,
        doc: &mut BehaviorGraphDocument,
        response: &egui::Response,
        canvas_rect: Rect,
    ) -> bool {
        let mut changed = false;

        // Open context menu on right-click
        if response.secondary_clicked() {
            if let Some(mouse) = response.interact_pointer_pos() {
                // Check if right-clicked on a node
                let origin = canvas_rect.left_top().to_vec2() + self.pan_offset;
                let mut found_node = None;
                for node in doc.nodes() {
                    let top_left =
                        Pos2::new(node.position.x + origin.x, node.position.y + origin.y);
                    let height = NODE_HEADER_HEIGHT + NODE_BODY_HEIGHT;
                    let rect = Rect::from_min_size(top_left, Vec2::new(NODE_WIDTH, height));
                    if rect.contains(mouse) {
                        found_node = Some(node.id);
                        break;
                    }
                }
                self.context_menu = if let Some(node_id) = found_node {
                    self.selected_node = Some(node_id);
                    Some(ContextMenu::Node { node_id })
                } else {
                    Some(ContextMenu::Canvas { pos: mouse })
                };
            }
        }

        // Render context menu
        if let Some(ref menu) = self.context_menu.clone() {
            let menu_id = Id::new("node_graph_context_menu");
            let mut close = false;

            egui::Area::new(menu_id)
                .fixed_pos(match menu {
                    ContextMenu::Canvas { pos } => *pos,
                    ContextMenu::Node { node_id } => {
                        let origin = canvas_rect.left_top().to_vec2() + self.pan_offset;
                        doc.node(*node_id)
                            .map(|n| {
                                Pos2::new(
                                    n.position.x + origin.x + NODE_WIDTH,
                                    n.position.y + origin.y,
                                )
                            })
                            .unwrap_or(canvas_rect.center())
                    }
                })
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(40, 40, 48))
                        .corner_radius(4.0)
                        .inner_margin(4.0)
                        .stroke(Stroke::new(1.0, Color32::from_rgb(70, 70, 80)))
                        .show(ui, |ui| match menu {
                            ContextMenu::Canvas { pos } => {
                                ui.label(RichText::new("Add Node").strong());
                                ui.separator();
                                let add_pos = self.screen_to_doc(*pos, canvas_rect);
                                if ui.button("⚡ Action").clicked() {
                                    let id = doc.add_node(
                                        "New Action",
                                        BehaviorGraphNodeKind::Action {
                                            name: "action".into(),
                                        },
                                    );
                                    if let Some(n) = doc.node_mut(id) {
                                        n.position = add_pos;
                                    }
                                    self.selected_node = Some(id);
                                    changed = true;
                                    close = true;
                                }
                                if ui.button("❓ Condition").clicked() {
                                    let id = doc.add_node(
                                        "New Condition",
                                        BehaviorGraphNodeKind::Condition {
                                            name: "condition".into(),
                                        },
                                    );
                                    if let Some(n) = doc.node_mut(id) {
                                        n.position = add_pos;
                                    }
                                    self.selected_node = Some(id);
                                    changed = true;
                                    close = true;
                                }
                                if ui.button("→ Sequence").clicked() {
                                    let id = doc.add_node(
                                        "New Sequence",
                                        BehaviorGraphNodeKind::Sequence {
                                            children: Vec::new(),
                                        },
                                    );
                                    if let Some(n) = doc.node_mut(id) {
                                        n.position = add_pos;
                                    }
                                    self.selected_node = Some(id);
                                    changed = true;
                                    close = true;
                                }
                                if ui.button("⑂ Selector").clicked() {
                                    let id = doc.add_node(
                                        "New Selector",
                                        BehaviorGraphNodeKind::Selector {
                                            children: Vec::new(),
                                        },
                                    );
                                    if let Some(n) = doc.node_mut(id) {
                                        n.position = add_pos;
                                    }
                                    self.selected_node = Some(id);
                                    changed = true;
                                    close = true;
                                }
                                if ui.button("∥ Parallel").clicked() {
                                    let id = doc.add_node(
                                        "New Parallel",
                                        BehaviorGraphNodeKind::Parallel {
                                            children: Vec::new(),
                                            success_threshold: 1,
                                        },
                                    );
                                    if let Some(n) = doc.node_mut(id) {
                                        n.position = add_pos;
                                    }
                                    self.selected_node = Some(id);
                                    changed = true;
                                    close = true;
                                }
                                if ui.button("◇ Decorator (Inverter)").clicked() {
                                    let id = doc.add_node(
                                        "New Inverter",
                                        BehaviorGraphNodeKind::Decorator(DecoratorNode::new(
                                            DecoratorKind::Inverter,
                                        )),
                                    );
                                    if let Some(n) = doc.node_mut(id) {
                                        n.position = add_pos;
                                    }
                                    self.selected_node = Some(id);
                                    changed = true;
                                    close = true;
                                }
                            }
                            ContextMenu::Node { node_id } => {
                                let is_root = *node_id == doc.root_id();
                                ui.label(RichText::new("Node").strong());
                                ui.separator();
                                if ui.button("🎯 Set as Root").clicked() {
                                    if doc.set_root(*node_id).is_ok() {
                                        self.status = Some("Root changed".into());
                                        changed = true;
                                    }
                                    close = true;
                                }
                                if ui.button("✂ Disconnect from Parent").clicked() {
                                    self.disconnect_child_from_all_parents(doc, *node_id);
                                    self.status = Some("Disconnected".into());
                                    changed = true;
                                    close = true;
                                }
                                if !is_root {
                                    if ui.button("🗑 Delete Node").clicked() {
                                        if doc.remove_node(*node_id).is_ok() {
                                            self.selected_node = Some(doc.root_id());
                                            self.status = Some("Node deleted".into());
                                            changed = true;
                                        }
                                        close = true;
                                    }
                                }
                            }
                        });
                });

            // Close menu on any click elsewhere
            if close || ui.input(|i| i.pointer.any_pressed()) {
                // Delay close by one frame so the button click registers
                if close {
                    self.context_menu = None;
                }
            }
            // Close on Escape
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.context_menu = None;
            }
        }

        changed
    }

    // ────────────────── Auto-layout ──────────────────

    /// Arrange nodes in a top-down tree layout starting from the root.
    /// Call this when loading a graph with all positions at (0,0).
    pub fn auto_layout(&self, doc: &mut BehaviorGraphDocument) {
        let root_id = doc.root_id();
        let mut visited = std::collections::HashSet::new();
        let mut column_widths: Vec<f32> = Vec::new();
        self.compute_subtree_width(doc, root_id, &mut visited, &mut column_widths, 0);

        visited.clear();
        self.layout_subtree(doc, root_id, &mut visited, 0, 0.0);
    }

    fn compute_subtree_width(
        &self,
        doc: &BehaviorGraphDocument,
        node_id: NodeId,
        visited: &mut std::collections::HashSet<NodeId>,
        column_widths: &mut Vec<f32>,
        _depth: usize,
    ) -> f32 {
        if !visited.insert(node_id) {
            return NODE_WIDTH + 20.0;
        }
        let children: Vec<NodeId> = doc
            .node(node_id)
            .map(|n| match &n.kind {
                BehaviorGraphNodeKind::Sequence { children }
                | BehaviorGraphNodeKind::Selector { children }
                | BehaviorGraphNodeKind::Parallel { children, .. } => children.clone(),
                BehaviorGraphNodeKind::Decorator(d) => d.child.into_iter().collect(),
                _ => Vec::new(),
            })
            .unwrap_or_default();

        if children.is_empty() {
            return NODE_WIDTH + 20.0;
        }

        let total: f32 = children
            .iter()
            .map(|&cid| self.compute_subtree_width(doc, cid, visited, column_widths, _depth + 1))
            .sum();
        total.max(NODE_WIDTH + 20.0)
    }

    fn layout_subtree(
        &self,
        doc: &mut BehaviorGraphDocument,
        node_id: NodeId,
        visited: &mut std::collections::HashSet<NodeId>,
        depth: usize,
        x_center: f32,
    ) {
        if !visited.insert(node_id) {
            return;
        }

        let y = depth as f32 * 100.0 + 50.0;
        let x = x_center - NODE_WIDTH / 2.0;

        if let Some(node) = doc.node_mut(node_id) {
            node.position = NodePosition { x, y };
        }

        let children: Vec<NodeId> = doc
            .node(node_id)
            .map(|n| match &n.kind {
                BehaviorGraphNodeKind::Sequence { children }
                | BehaviorGraphNodeKind::Selector { children }
                | BehaviorGraphNodeKind::Parallel { children, .. } => children.clone(),
                BehaviorGraphNodeKind::Decorator(d) => d.child.into_iter().collect(),
                _ => Vec::new(),
            })
            .unwrap_or_default();

        if children.is_empty() {
            return;
        }

        // Compute widths for each child subtree
        let mut child_widths: Vec<f32> = Vec::new();
        let mut temp_visited = visited.clone();
        for &cid in &children {
            let w =
                self.compute_subtree_width(doc, cid, &mut temp_visited, &mut Vec::new(), depth + 1);
            child_widths.push(w);
        }

        let total_width: f32 = child_widths.iter().sum();
        let mut cx = x_center - total_width / 2.0;

        for (i, &cid) in children.iter().enumerate() {
            let w = child_widths[i];
            let child_center = cx + w / 2.0;
            self.layout_subtree(doc, cid, visited, depth + 1, child_center);
            cx += w;
        }
    }

    // ────────────────── Helpers ──────────────────

    fn screen_to_doc(&self, screen_pos: Pos2, canvas_rect: Rect) -> NodePosition {
        let origin = canvas_rect.left_top().to_vec2() + self.pan_offset;
        NodePosition {
            x: screen_pos.x - origin.x,
            y: screen_pos.y - origin.y,
        }
    }

    /// Check if any node positions are all at default (0,0) — meaning auto-layout is needed.
    pub fn needs_auto_layout(&self, doc: &BehaviorGraphDocument) -> bool {
        if doc.nodes().len() <= 1 {
            return false;
        }
        doc.nodes()
            .iter()
            .all(|n| n.position.x == 0.0 && n.position.y == 0.0)
    }
}

// ────────────────── Free functions ──────────────────

fn port_pos_input(rect: Rect) -> Pos2 {
    Pos2::new(rect.center().x, rect.min.y)
}

fn draw_bezier_wire(painter: &egui::Painter, from: Pos2, to: Pos2, color: Color32) {
    let dy = (to.y - from.y).abs().max(40.0) * 0.5;
    let cp1 = Pos2::new(from.x, from.y + dy);
    let cp2 = Pos2::new(to.x, to.y - dy);
    let bezier = CubicBezierShape::from_points_stroke(
        [from, cp1, cp2, to],
        false,
        Color32::TRANSPARENT,
        Stroke::new(WIRE_THICKNESS, color),
    );
    painter.add(bezier);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_graph_widget_default() {
        let widget = NodeGraphWidget::default();
        assert!(widget.selected_node.is_none());
        assert!(widget.pending_wire.is_none());
        assert!(widget.node_drag.is_none());
        assert!(widget.context_menu.is_none());
    }

    #[test]
    fn test_screen_to_doc_conversion() {
        let widget = NodeGraphWidget::default();
        let canvas = Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(800.0, 600.0));
        let doc_pos = widget.screen_to_doc(Pos2::new(200.0, 150.0), canvas);
        assert!((doc_pos.x - 100.0).abs() < 0.01);
        assert!((doc_pos.y - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_needs_auto_layout_single_node() {
        let widget = NodeGraphWidget::default();
        let doc = BehaviorGraphDocument::new_default();
        assert!(!widget.needs_auto_layout(&doc));
    }

    #[test]
    fn test_needs_auto_layout_multiple_at_origin() {
        let widget = NodeGraphWidget::default();
        let mut doc = BehaviorGraphDocument::new_default();
        doc.add_node(
            "test",
            BehaviorGraphNodeKind::Action {
                name: "test".into(),
            },
        );
        assert!(widget.needs_auto_layout(&doc));
    }

    #[test]
    fn test_port_pos_input() {
        let rect = Rect::from_min_size(
            Pos2::new(100.0, 200.0),
            Vec2::new(NODE_WIDTH, NODE_HEADER_HEIGHT + NODE_BODY_HEIGHT),
        );
        let pos = port_pos_input(rect);
        assert!((pos.x - (100.0 + NODE_WIDTH / 2.0)).abs() < 0.01);
        assert!((pos.y - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_node_header_colors_unique() {
        let colors = [
            node_header_color(&BehaviorGraphNodeKind::Action { name: "a".into() }),
            node_header_color(&BehaviorGraphNodeKind::Condition { name: "c".into() }),
            node_header_color(&BehaviorGraphNodeKind::Sequence { children: vec![] }),
            node_header_color(&BehaviorGraphNodeKind::Selector { children: vec![] }),
            node_header_color(&BehaviorGraphNodeKind::Parallel {
                children: vec![],
                success_threshold: 1,
            }),
            node_header_color(&BehaviorGraphNodeKind::Decorator(DecoratorNode::new(
                DecoratorKind::Inverter,
            ))),
        ];
        // All colors should be different
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(
                    colors[i], colors[j],
                    "Colors at {} and {} should differ",
                    i, j
                );
            }
        }
    }

    #[test]
    fn test_auto_layout_positions_nodes() {
        let widget = NodeGraphWidget::default();
        let mut doc = BehaviorGraphDocument::new_default();
        let root = doc.root_id();
        // Make root a sequence so it can have children
        if let Some(node) = doc.node_mut(root) {
            node.kind = BehaviorGraphNodeKind::Sequence {
                children: Vec::new(),
            };
        }
        let child = doc
            .add_child_node(
                root,
                "child",
                BehaviorGraphNodeKind::Action { name: "act".into() },
            )
            .unwrap();

        widget.auto_layout(&mut doc);

        let root_node = doc.node(root).unwrap();
        let child_node = doc.node(child).unwrap();

        // Root should be at depth 0, child at depth 1
        assert!(
            root_node.position.y < child_node.position.y,
            "Child should be below root"
        );
    }

    #[test]
    fn test_connect_nodes_basic() {
        let mut widget = NodeGraphWidget::default();
        let mut doc = BehaviorGraphDocument::new_default();
        let root = doc.root_id();
        // Make root a sequence
        if let Some(node) = doc.node_mut(root) {
            node.kind = BehaviorGraphNodeKind::Sequence {
                children: Vec::new(),
            };
        }
        let child = doc.add_node(
            "child",
            BehaviorGraphNodeKind::Action { name: "act".into() },
        );

        let connected = widget.connect_nodes(&mut doc, root, child);
        assert!(connected);

        // Verify child is now in root's children
        if let Some(node) = doc.node(root) {
            if let BehaviorGraphNodeKind::Sequence { children } = &node.kind {
                assert!(children.contains(&child));
            } else {
                panic!("Expected Sequence");
            }
        }
    }

    #[test]
    fn test_connect_duplicate_prevented() {
        let mut widget = NodeGraphWidget::default();
        let mut doc = BehaviorGraphDocument::new_default();
        let root = doc.root_id();
        if let Some(node) = doc.node_mut(root) {
            node.kind = BehaviorGraphNodeKind::Sequence {
                children: Vec::new(),
            };
        }
        let child = doc.add_node(
            "child",
            BehaviorGraphNodeKind::Action { name: "act".into() },
        );

        widget.connect_nodes(&mut doc, root, child);
        let second = widget.connect_nodes(&mut doc, root, child);
        assert!(!second, "Duplicate connection should be rejected");
    }

    #[test]
    fn test_connect_leaf_rejected() {
        let mut widget = NodeGraphWidget::default();
        let mut doc = BehaviorGraphDocument::new_default();
        let root = doc.root_id(); // root is Action by default
        let child = doc.add_node(
            "child",
            BehaviorGraphNodeKind::Action { name: "act".into() },
        );

        let connected = widget.connect_nodes(&mut doc, root, child);
        assert!(!connected, "Action node should not accept children");
    }

    #[test]
    fn test_disconnect_child() {
        let mut widget = NodeGraphWidget::default();
        let mut doc = BehaviorGraphDocument::new_default();
        let root = doc.root_id();
        if let Some(node) = doc.node_mut(root) {
            node.kind = BehaviorGraphNodeKind::Sequence {
                children: Vec::new(),
            };
        }
        let child = doc.add_node(
            "child",
            BehaviorGraphNodeKind::Action { name: "act".into() },
        );
        widget.connect_nodes(&mut doc, root, child);

        widget.disconnect_child_from_all_parents(&mut doc, child);

        if let Some(node) = doc.node(root) {
            if let BehaviorGraphNodeKind::Sequence { children } = &node.kind {
                assert!(!children.contains(&child), "Child should be disconnected");
            }
        }
    }
}
