//! Node graph editor widget for visual scripting and data flow visualization.
//!
//! Provides a visual node graph editor with drag-and-drop nodes, connections between ports,
//! and force-directed auto-layout.
//!
//! # Example
//! ```
//! use astract::graph::{NodeGraph, GraphNode, Port, PortType};
//!
//! let mut graph = NodeGraph::new();
//!
//! // Add nodes
//! let mut start_node = GraphNode::new(1, "Start");
//! start_node.add_output(Port::new(0, "Out", PortType::Exec));
//! let start_id = graph.add_node(start_node);
//!
//! let mut action_node = GraphNode::new(2, "Action");
//! action_node.add_input(Port::new(0, "In", PortType::Exec));
//! action_node.add_output(Port::new(1, "Out", PortType::Exec));
//! let action_id = graph.add_node(action_node);
//!
//! // Connect nodes
//! graph.add_edge(start_id, 0, action_id, 0);
//!
//! // Show UI
//! // graph.show(ui);
//! ```

use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use std::collections::HashMap;

pub type NodeId = u64;

/// Port type for visual distinction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PortType {
    /// Execution flow (white)
    Exec,
    /// Boolean data (red)
    Bool,
    /// Numeric data (green)
    Number,
    /// String data (blue)
    String,
    /// Object reference (yellow)
    Object,
}

impl PortType {
    /// Get color for port type
    pub fn color(&self) -> Color32 {
        match self {
            PortType::Exec => Color32::WHITE,
            PortType::Bool => Color32::from_rgb(220, 50, 50),
            PortType::Number => Color32::from_rgb(100, 200, 100),
            PortType::String => Color32::from_rgb(100, 150, 255),
            PortType::Object => Color32::from_rgb(255, 200, 50),
        }
    }
}

/// Input/output port on a node
#[derive(Debug, Clone)]
pub struct Port {
    /// Port index (unique within node's input/output list)
    pub index: usize,
    /// Port label
    pub label: String,
    /// Port type
    pub port_type: PortType,
}

impl Port {
    /// Create a new port
    pub fn new(index: usize, label: impl Into<String>, port_type: PortType) -> Self {
        Self {
            index,
            label: label.into(),
            port_type,
        }
    }
}

/// Node in the graph
#[derive(Debug, Clone)]
pub struct GraphNode {
    id: NodeId,
    label: String,
    position: Pos2,
    inputs: Vec<Port>,
    outputs: Vec<Port>,
    size: Vec2,
    selected: bool,
}

impl GraphNode {
    /// Create a new node at origin
    pub fn new(id: NodeId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            position: Pos2::ZERO,
            inputs: Vec::new(),
            outputs: Vec::new(),
            size: Vec2::new(120.0, 60.0),
            selected: false,
        }
    }

    /// Set node position
    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.position = Pos2::new(x, y);
        self
    }

    /// Add an input port
    pub fn add_input(&mut self, port: Port) {
        self.inputs.push(port);
        self.update_size();
    }

    /// Add an output port
    pub fn add_output(&mut self, port: Port) {
        self.outputs.push(port);
        self.update_size();
    }

    /// Update node size based on port count
    fn update_size(&mut self) {
        let port_count = self.inputs.len().max(self.outputs.len());
        let min_height = 60.0;
        let port_height = 20.0;
        self.size.y = min_height + (port_count as f32 * port_height);
    }

    /// Get node ID
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Get node label
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Get input ports
    pub fn inputs(&self) -> &[Port] {
        &self.inputs
    }

    /// Get output ports
    pub fn outputs(&self) -> &[Port] {
        &self.outputs
    }

    /// Get node bounding rect
    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.position, self.size)
    }

    /// Get node X position
    pub fn x(&self) -> f32 {
        self.position.x
    }

    /// Get node Y position
    pub fn y(&self) -> f32 {
        self.position.y
    }

    /// Get port position (world coordinates)
    pub fn port_position(&self, is_output: bool, port_index: usize) -> Pos2 {
        let port_offset_y = 30.0 + (port_index as f32 * 20.0);
        let x = if is_output {
            self.position.x + self.size.x
        } else {
            self.position.x
        };
        Pos2::new(x, self.position.y + port_offset_y)
    }
}

/// Edge connecting two ports
#[derive(Debug, Clone, Copy)]
pub struct GraphEdge {
    source_node: NodeId,
    source_port: usize,
    target_node: NodeId,
    target_port: usize,
}

impl GraphEdge {
    /// Create a new edge
    pub fn new(
        source_node: NodeId,
        source_port: usize,
        target_node: NodeId,
        target_port: usize,
    ) -> Self {
        Self {
            source_node,
            source_port,
            target_node,
            target_port,
        }
    }

    /// Get source node ID
    pub fn source_node(&self) -> NodeId {
        self.source_node
    }

    /// Get source port index
    pub fn source_port(&self) -> usize {
        self.source_port
    }

    /// Get target node ID
    pub fn target_node(&self) -> NodeId {
        self.target_node
    }

    /// Get target port index
    pub fn target_port(&self) -> usize {
        self.target_port
    }
}

/// Node graph editor
pub struct NodeGraph {
    nodes: HashMap<NodeId, GraphNode>,
    edges: Vec<GraphEdge>,
    next_id: NodeId,
    pan_offset: Vec2,
    zoom: f32,
    #[allow(dead_code)]
    dragging_node: Option<NodeId>,
}

impl NodeGraph {
    /// Create a new empty node graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            next_id: 1,
            pan_offset: Vec2::ZERO,
            zoom: 1.0,
            dragging_node: None,
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: GraphNode) -> NodeId {
        let id = node.id();
        self.nodes.insert(id, node);
        self.next_id = self.next_id.max(id + 1);
        id
    }

    /// Add an edge between two ports
    pub fn add_edge(
        &mut self,
        source_node: NodeId,
        source_port: usize,
        target_node: NodeId,
        target_port: usize,
    ) {
        let edge = GraphEdge::new(source_node, source_port, target_node, target_port);
        self.edges.push(edge);
    }

    /// Get all nodes
    pub fn nodes(&self) -> &HashMap<NodeId, GraphNode> {
        &self.nodes
    }

    /// Get all edges
    pub fn edges(&self) -> &[GraphEdge] {
        &self.edges
    }

    /// Get a node by ID
    pub fn get_node(&self, id: NodeId) -> Option<&GraphNode> {
        self.nodes.get(&id)
    }

    /// Get a mutable node by ID
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut GraphNode> {
        self.nodes.get_mut(&id)
    }

    /// Clear all nodes and edges
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.next_id = 1;
    }

    /// Remove a node and its connected edges
    pub fn remove_node(&mut self, id: NodeId) -> Option<GraphNode> {
        // Remove all edges connected to this node
        self.edges
            .retain(|e| e.source_node != id && e.target_node != id);
        // Remove the node
        self.nodes.remove(&id)
    }

    /// Apply force-directed layout to arrange nodes automatically
    ///
    /// Uses spring forces (attraction) and repulsion forces to create a visually
    /// pleasing layout. Connected nodes are pulled together, unconnected nodes
    /// are pushed apart.
    ///
    /// # Example
    /// ```no_run
    /// # use astract::graph::{NodeGraph, GraphNode, Port, PortType, ForceDirectedParams};
    /// let mut graph = NodeGraph::new();
    /// // ... add nodes and edges ...
    /// graph.auto_layout(); // Use default params
    ///
    /// // Or with custom params
    /// let params = ForceDirectedParams {
    ///     k: 150.0,                // Larger optimal distance
    ///     max_iterations: 300,     // Fewer iterations for faster layout
    ///     ..Default::default()
    /// };
    /// graph.auto_layout_with_params(params);
    /// ```
    pub fn auto_layout(&mut self) {
        self.auto_layout_with_params(super::force_directed::ForceDirectedParams::default());
    }

    /// Apply force-directed layout with custom parameters
    pub fn auto_layout_with_params(&mut self, params: super::force_directed::ForceDirectedParams) {
        use super::force_directed::ForceDirectedLayout;

        if self.nodes.is_empty() {
            return;
        }

        // Collect node IDs and initial positions
        let node_ids: Vec<NodeId> = self.nodes.keys().copied().collect();
        let initial_positions: HashMap<NodeId, Vec2> = self
            .nodes
            .iter()
            .map(|(&id, node)| (id, node.position.to_vec2()))
            .collect();

        // Convert edges to (source_id, target_id) pairs
        let edges: Vec<(NodeId, NodeId)> = self
            .edges
            .iter()
            .map(|edge| (edge.source_node, edge.target_node))
            .collect();

        // Run layout algorithm
        let layout = ForceDirectedLayout::new(params);
        let new_positions = layout.layout(&node_ids, &initial_positions, &edges);

        // Apply new positions
        for (id, position) in new_positions {
            if let Some(node) = self.nodes.get_mut(&id) {
                node.position = Pos2::new(position.x, position.y);
            }
        }
    }

    /// Show the node graph UI
    pub fn show(&mut self, ui: &mut Ui) -> Option<NodeId> {
        let mut clicked_node = None;

        ui.vertical(|ui| {
            ui.label("Node Graph Editor");
            ui.separator();

            // Graph canvas
            let (response, painter) = ui.allocate_painter(
                Vec2::new(ui.available_width(), 400.0),
                Sense::click_and_drag(),
            );

            let canvas_rect = response.rect;

            // Background
            painter.rect_filled(canvas_rect, 0.0, Color32::from_gray(30));

            // Draw edges
            for edge in &self.edges {
                if let (Some(source), Some(target)) = (
                    self.nodes.get(&edge.source_node),
                    self.nodes.get(&edge.target_node),
                ) {
                    let source_pos = source.port_position(true, edge.source_port);
                    let target_pos = target.port_position(false, edge.target_port);

                    // Transform to screen coordinates
                    let source_screen =
                        canvas_rect.min + (source_pos.to_vec2() + self.pan_offset) * self.zoom;
                    let target_screen =
                        canvas_rect.min + (target_pos.to_vec2() + self.pan_offset) * self.zoom;

                    // Bezier curve for edges
                    let ctrl_offset = 50.0 * self.zoom;
                    let ctrl1 = Pos2::new(source_screen.x + ctrl_offset, source_screen.y);
                    let ctrl2 = Pos2::new(target_screen.x - ctrl_offset, target_screen.y);

                    // Draw cubic bezier (simplified with line segments)
                    let steps = 20;
                    for i in 0..steps {
                        let t1 = i as f32 / steps as f32;
                        let t2 = (i + 1) as f32 / steps as f32;
                        let p1 = cubic_bezier(source_screen, ctrl1, ctrl2, target_screen, t1);
                        let p2 = cubic_bezier(source_screen, ctrl1, ctrl2, target_screen, t2);
                        painter.line_segment([p1, p2], Stroke::new(2.0, Color32::from_gray(150)));
                    }
                }
            }

            // Draw nodes
            for node in self.nodes.values() {
                let node_rect_world = node.rect();
                let node_rect_screen = Rect::from_min_size(
                    canvas_rect.min + (node_rect_world.min.to_vec2() + self.pan_offset) * self.zoom,
                    node_rect_world.size() * self.zoom,
                );

                // Node background
                let bg_color = if node.selected {
                    Color32::from_rgb(80, 100, 120)
                } else {
                    Color32::from_rgb(60, 70, 80)
                };
                painter.rect_filled(node_rect_screen, 5.0, bg_color);
                painter.rect_stroke(
                    node_rect_screen,
                    5.0,
                    Stroke::new(1.0, Color32::from_gray(150)),
                    egui::epaint::StrokeKind::Middle,
                );

                // Node label
                painter.text(
                    node_rect_screen.center_top() + Vec2::new(0.0, 10.0),
                    egui::Align2::CENTER_TOP,
                    &node.label,
                    egui::FontId::proportional(12.0),
                    Color32::WHITE,
                );

                // Input ports (left side)
                for (i, port) in node.inputs.iter().enumerate() {
                    let port_world = node.port_position(false, i);
                    let port_screen =
                        canvas_rect.min + (port_world.to_vec2() + self.pan_offset) * self.zoom;
                    painter.circle_filled(port_screen, 4.0 * self.zoom, port.port_type.color());
                }

                // Output ports (right side)
                for (i, port) in node.outputs.iter().enumerate() {
                    let port_world = node.port_position(true, i);
                    let port_screen =
                        canvas_rect.min + (port_world.to_vec2() + self.pan_offset) * self.zoom;
                    painter.circle_filled(port_screen, 4.0 * self.zoom, port.port_type.color());
                }

                // Check if clicked
                if response.clicked() {
                    let click_pos = response.interact_pointer_pos().unwrap();
                    if node_rect_screen.contains(click_pos) {
                        clicked_node = Some(node.id);
                    }
                }
            }

            // Pan/drag interaction
            if response.dragged() {
                self.pan_offset += response.drag_delta() / self.zoom;
            }

            // Stats
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(format!("Nodes: {}", self.nodes.len()));
                ui.label(format!("Edges: {}", self.edges.len()));
                ui.label(format!("Zoom: {:.2}×", self.zoom));
            });
        });

        clicked_node
    }
}

impl Default for NodeGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper: Cubic Bezier curve
fn cubic_bezier(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, t: f32) -> Pos2 {
    let u = 1.0 - t;
    let tt = t * t;
    let uu = u * u;
    let uuu = uu * u;
    let ttt = tt * t;

    let x = uuu * p0.x + 3.0 * uu * t * p1.x + 3.0 * u * tt * p2.x + ttt * p3.x;
    let y = uuu * p0.y + 3.0 * uu * t * p1.y + 3.0 * u * tt * p2.y + ttt * p3.y;

    Pos2::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = GraphNode::new(1, "Test");
        assert_eq!(node.id(), 1);
        assert_eq!(node.label(), "Test");
        assert_eq!(node.inputs().len(), 0);
        assert_eq!(node.outputs().len(), 0);
    }

    #[test]
    fn test_node_with_ports() {
        let mut node = GraphNode::new(1, "Test");
        node.add_input(Port::new(0, "In", PortType::Exec));
        node.add_output(Port::new(0, "Out", PortType::Number));

        assert_eq!(node.inputs().len(), 1);
        assert_eq!(node.outputs().len(), 1);
        assert_eq!(node.inputs()[0].label, "In");
        assert_eq!(node.outputs()[0].port_type, PortType::Number);
    }

    #[test]
    fn test_graph_add_node() {
        let mut graph = NodeGraph::new();
        let node = GraphNode::new(1, "Test");
        let id = graph.add_node(node);

        assert_eq!(id, 1);
        assert_eq!(graph.nodes().len(), 1);
        assert!(graph.get_node(id).is_some());
    }

    #[test]
    fn test_graph_add_edge() {
        let mut graph = NodeGraph::new();
        let node1 = GraphNode::new(1, "Node1");
        let node2 = GraphNode::new(2, "Node2");

        graph.add_node(node1);
        graph.add_node(node2);
        graph.add_edge(1, 0, 2, 0);

        assert_eq!(graph.edges().len(), 1);
        assert_eq!(graph.edges()[0].source_node(), 1);
        assert_eq!(graph.edges()[0].target_node(), 2);
    }

    #[test]
    fn test_graph_clear() {
        let mut graph = NodeGraph::new();
        let node = GraphNode::new(1, "Test");
        graph.add_node(node);
        graph.add_edge(1, 0, 1, 0);

        graph.clear();

        assert_eq!(graph.nodes().len(), 0);
        assert_eq!(graph.edges().len(), 0);
    }

    #[test]
    fn test_port_type_colors() {
        assert_eq!(PortType::Exec.color(), Color32::WHITE);
        assert_eq!(PortType::Bool.color(), Color32::from_rgb(220, 50, 50));
        assert_eq!(PortType::Number.color(), Color32::from_rgb(100, 200, 100));
        assert_eq!(PortType::String.color(), Color32::from_rgb(100, 150, 255));
        assert_eq!(PortType::Object.color(), Color32::from_rgb(255, 200, 50));
    }

    #[test]
    fn test_node_position() {
        let node = GraphNode::new(1, "Test").with_position(100.0, 200.0);
        assert_eq!(node.position, Pos2::new(100.0, 200.0));
    }

    #[test]
    fn test_node_rect() {
        let node = GraphNode::new(1, "Test").with_position(100.0, 200.0);
        let rect = node.rect();
        assert_eq!(rect.min, Pos2::new(100.0, 200.0));
        assert_eq!(rect.size(), Vec2::new(120.0, 60.0));
    }

    #[test]
    fn test_port_position() {
        let mut node = GraphNode::new(1, "Test").with_position(100.0, 200.0);
        node.add_input(Port::new(0, "In", PortType::Exec));
        node.add_output(Port::new(0, "Out", PortType::Exec));

        let input_pos = node.port_position(false, 0);
        let output_pos = node.port_position(true, 0);

        assert_eq!(input_pos, Pos2::new(100.0, 230.0)); // Left side
        assert_eq!(output_pos, Pos2::new(220.0, 230.0)); // Right side (100 + 120)
    }
}
