use astract::graph::{ForceDirectedParams, GraphEdge, GraphNode, NodeGraph, Port, PortType};
use egui::Ui;
use std::collections::HashMap;
use tracing::info;

/// Graph type for categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GraphType {
    BehaviorTree,
    Shader,
    Dialogue,
    StateMachine,
    Animation,
    Custom,
}

impl std::fmt::Display for GraphType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl GraphType {
    pub fn all() -> &'static [GraphType] {
        &[
            GraphType::BehaviorTree,
            GraphType::Shader,
            GraphType::Dialogue,
            GraphType::StateMachine,
            GraphType::Animation,
            GraphType::Custom,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            GraphType::BehaviorTree => "Behavior Tree",
            GraphType::Shader => "Shader Graph",
            GraphType::Dialogue => "Dialogue Graph",
            GraphType::StateMachine => "State Machine",
            GraphType::Animation => "Animation Graph",
            GraphType::Custom => "Custom Graph",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            GraphType::BehaviorTree => "[Brain]",
            GraphType::Shader => "[Art]",
            GraphType::Dialogue => "[Chat]",
            GraphType::StateMachine => "⚙️",
            GraphType::Animation => "[Anim]",
            GraphType::Custom => "[Chart]",
        }
    }
}

/// Node template for quick creation
#[derive(Debug, Clone)]
pub struct NodeTemplate {
    pub name: String,
    pub category: String,
    pub inputs: Vec<(String, PortType)>,
    pub outputs: Vec<(String, PortType)>,
    pub color: Option<egui::Color32>,
}

impl NodeTemplate {
    pub fn new(name: impl Into<String>, category: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            category: category.into(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            color: None,
        }
    }

    pub fn with_input(mut self, name: impl Into<String>, port_type: PortType) -> Self {
        self.inputs.push((name.into(), port_type));
        self
    }

    pub fn with_output(mut self, name: impl Into<String>, port_type: PortType) -> Self {
        self.outputs.push((name.into(), port_type));
        self
    }

    pub fn with_color(mut self, color: egui::Color32) -> Self {
        self.color = Some(color);
        self
    }

    /// Create a GraphNode from this template
    pub fn create_node(&self, id: u64, x: f32, y: f32) -> GraphNode {
        let mut node = GraphNode::new(id, &self.name).with_position(x, y);

        for (port_id, (name, port_type)) in self.inputs.iter().enumerate() {
            node.add_input(Port::new(port_id, name, *port_type));
        }

        for (port_id, (name, port_type)) in self.outputs.iter().enumerate() {
            node.add_output(Port::new(port_id + 100, name, *port_type));
        }

        node
    }
}

/// Graph statistics for display
#[derive(Debug, Clone, Default)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub input_ports: usize,
    pub output_ports: usize,
    pub unconnected_inputs: usize,
    pub unconnected_outputs: usize,
    pub max_depth: usize,
}

impl GraphStats {
    pub fn from_graph(graph: &NodeGraph) -> Self {
        let nodes = graph.nodes();
        let edges = graph.edges();

        let mut input_ports = 0;
        let mut output_ports = 0;

        for (_, node) in nodes.iter() {
            input_ports += node.inputs().len();
            output_ports += node.outputs().len();
        }

        // Count connected ports
        let connected_inputs: std::collections::HashSet<_> = edges
            .iter()
            .map(|e| (e.target_node(), e.target_port()))
            .collect();
        let connected_outputs: std::collections::HashSet<_> = edges
            .iter()
            .map(|e| (e.source_node(), e.source_port()))
            .collect();

        // Calculate max depth using BFS from root nodes (nodes with no incoming edges)
        let max_depth = Self::calculate_max_depth(nodes, edges);

        Self {
            node_count: nodes.len(),
            edge_count: edges.len(),
            input_ports,
            output_ports,
            unconnected_inputs: input_ports.saturating_sub(connected_inputs.len()),
            unconnected_outputs: output_ports.saturating_sub(connected_outputs.len()),
            max_depth,
        }
    }

    /// Calculate the maximum depth of the graph using BFS from root nodes
    fn calculate_max_depth(
        nodes: &std::collections::HashMap<u64, GraphNode>,
        edges: &[GraphEdge],
    ) -> usize {
        use std::collections::{HashMap, HashSet, VecDeque};

        if nodes.is_empty() {
            return 0;
        }

        // Build adjacency list (source -> targets)
        let mut children: HashMap<u64, Vec<u64>> = HashMap::new();
        let mut has_parent: HashSet<u64> = HashSet::new();

        for edge in edges {
            children
                .entry(edge.source_node())
                .or_default()
                .push(edge.target_node());
            has_parent.insert(edge.target_node());
        }

        // Find root nodes (nodes with no incoming edges)
        let roots: Vec<u64> = nodes
            .keys()
            .filter(|id| !has_parent.contains(id))
            .copied()
            .collect();

        // If no roots found (cyclic graph), use first node as root
        let start_nodes = if roots.is_empty() {
            nodes.keys().take(1).copied().collect()
        } else {
            roots
        };

        // BFS to find max depth
        let mut max_depth = 0;
        let mut visited: HashSet<u64> = HashSet::new();
        let mut queue: VecDeque<(u64, usize)> = VecDeque::new();

        for root in start_nodes {
            queue.push_back((root, 1));
            visited.insert(root);
        }

        while let Some((node_id, depth)) = queue.pop_front() {
            max_depth = max_depth.max(depth);

            if let Some(targets) = children.get(&node_id) {
                for &target in targets {
                    if visited.insert(target) {
                        queue.push_back((target, depth + 1));
                    }
                }
            }
        }

        max_depth
    }
}

/// Panel demonstrating graph visualization widgets for visual scripting and behavior trees
pub struct GraphPanel {
    // Graphs
    behavior_tree_graph: NodeGraph,
    shader_graph: NodeGraph,
    dialogue_graph: NodeGraph,

    // State
    initialized: bool,
    active_graph: GraphType,

    // View controls
    zoom_level: f32,
    show_grid: bool,
    snap_to_grid: bool,
    grid_size: f32,
    show_minimap: bool,

    // Search/filter
    search_query: String,
    filter_by_type: Option<PortType>,

    // Node templates
    templates: HashMap<GraphType, Vec<NodeTemplate>>,
    selected_template: Option<usize>,

    // Selection
    selected_nodes: Vec<u64>,
    clipboard: Option<Vec<GraphNode>>,

    // Stats
    show_stats: bool,

    // Next node ID for creation
    next_node_id: u64,
}

impl Default for GraphPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphPanel {
    pub fn new() -> Self {
        let mut panel = Self {
            behavior_tree_graph: NodeGraph::new(),
            shader_graph: NodeGraph::new(),
            dialogue_graph: NodeGraph::new(),
            initialized: false,
            active_graph: GraphType::BehaviorTree,
            zoom_level: 1.0,
            show_grid: true,
            snap_to_grid: false,
            grid_size: 20.0,
            show_minimap: false,
            search_query: String::new(),
            filter_by_type: None,
            templates: HashMap::new(),
            selected_template: None,
            selected_nodes: Vec::new(),
            clipboard: None,
            show_stats: true,
            next_node_id: 100,
        };
        panel.init_templates();
        panel
    }

    /// Initialize node templates for each graph type
    fn init_templates(&mut self) {
        // Behavior Tree templates
        self.templates.insert(
            GraphType::BehaviorTree,
            vec![
                NodeTemplate::new("Root", "Control").with_output("Out", PortType::Exec),
                NodeTemplate::new("Selector", "Control")
                    .with_input("In", PortType::Exec)
                    .with_output("A", PortType::Exec)
                    .with_output("B", PortType::Exec)
                    .with_output("C", PortType::Exec),
                NodeTemplate::new("Sequence", "Control")
                    .with_input("In", PortType::Exec)
                    .with_output("Step 1", PortType::Exec)
                    .with_output("Step 2", PortType::Exec),
                NodeTemplate::new("Condition", "Logic")
                    .with_input("In", PortType::Exec)
                    .with_input("Value", PortType::Bool)
                    .with_output("True", PortType::Exec)
                    .with_output("False", PortType::Exec),
                NodeTemplate::new("Action", "Leaf")
                    .with_input("In", PortType::Exec)
                    .with_output("Success", PortType::Bool),
                NodeTemplate::new("Wait", "Leaf")
                    .with_input("In", PortType::Exec)
                    .with_input("Duration", PortType::Number)
                    .with_output("Done", PortType::Exec),
            ],
        );

        // Shader templates
        self.templates.insert(
            GraphType::Shader,
            vec![
                NodeTemplate::new("Texture Sample", "Input")
                    .with_output("Color", PortType::Object)
                    .with_output("Alpha", PortType::Number),
                NodeTemplate::new("Color", "Input").with_output("RGB", PortType::Object),
                NodeTemplate::new("Float", "Input").with_output("Value", PortType::Number),
                NodeTemplate::new("Add", "Math")
                    .with_input("A", PortType::Number)
                    .with_input("B", PortType::Number)
                    .with_output("Result", PortType::Number),
                NodeTemplate::new("Multiply", "Math")
                    .with_input("A", PortType::Number)
                    .with_input("B", PortType::Number)
                    .with_output("Result", PortType::Number),
                NodeTemplate::new("Lerp", "Math")
                    .with_input("A", PortType::Object)
                    .with_input("B", PortType::Object)
                    .with_input("T", PortType::Number)
                    .with_output("Result", PortType::Object),
                NodeTemplate::new("Material Output", "Output")
                    .with_input("Base Color", PortType::Object)
                    .with_input("Normal", PortType::Object)
                    .with_input("Roughness", PortType::Number)
                    .with_input("Metallic", PortType::Number),
            ],
        );

        // Dialogue templates
        self.templates.insert(
            GraphType::Dialogue,
            vec![
                NodeTemplate::new("Start", "Control").with_output("Begin", PortType::Exec),
                NodeTemplate::new("End", "Control").with_input("Finish", PortType::Exec),
                NodeTemplate::new("Say", "Dialogue")
                    .with_input("In", PortType::Exec)
                    .with_input("Speaker", PortType::String)
                    .with_input("Text", PortType::String)
                    .with_output("Next", PortType::Exec),
                NodeTemplate::new("Choice", "Dialogue")
                    .with_input("In", PortType::Exec)
                    .with_output("Option 1", PortType::Exec)
                    .with_output("Option 2", PortType::Exec)
                    .with_output("Option 3", PortType::Exec),
                NodeTemplate::new("Condition", "Logic")
                    .with_input("In", PortType::Exec)
                    .with_input("Variable", PortType::String)
                    .with_output("True", PortType::Exec)
                    .with_output("False", PortType::Exec),
                NodeTemplate::new("Set Variable", "Logic")
                    .with_input("In", PortType::Exec)
                    .with_input("Variable", PortType::String)
                    .with_input("Value", PortType::String)
                    .with_output("Next", PortType::Exec),
            ],
        );
    }

    /// Get templates for current graph type
    pub fn current_templates(&self) -> &[NodeTemplate] {
        self.templates
            .get(&self.active_graph)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get the active graph
    fn active_graph_mut(&mut self) -> &mut NodeGraph {
        match self.active_graph {
            GraphType::BehaviorTree => &mut self.behavior_tree_graph,
            GraphType::Shader => &mut self.shader_graph,
            GraphType::Dialogue => &mut self.dialogue_graph,
            _ => &mut self.behavior_tree_graph,
        }
    }

    /// Get the active graph (immutable)
    fn active_graph_ref(&self) -> &NodeGraph {
        match self.active_graph {
            GraphType::BehaviorTree => &self.behavior_tree_graph,
            GraphType::Shader => &self.shader_graph,
            GraphType::Dialogue => &self.dialogue_graph,
            _ => &self.behavior_tree_graph,
        }
    }

    /// Add a node from template
    pub fn add_node_from_template(&mut self, template_index: usize, x: f32, y: f32) -> Option<u64> {
        let templates = self.current_templates().to_vec();
        if let Some(template) = templates.get(template_index) {
            let id = self.next_node_id;
            self.next_node_id += 1;

            let (final_x, final_y) = if self.snap_to_grid {
                (
                    (x / self.grid_size).round() * self.grid_size,
                    (y / self.grid_size).round() * self.grid_size,
                )
            } else {
                (x, y)
            };

            let node = template.create_node(id, final_x, final_y);
            self.active_graph_mut().add_node(node);
            Some(id)
        } else {
            None
        }
    }

    /// Delete selected nodes
    pub fn delete_selected(&mut self) {
        for &node_id in &self.selected_nodes.clone() {
            self.active_graph_mut().remove_node(node_id);
        }
        self.selected_nodes.clear();
    }

    /// Copy selected nodes to clipboard
    pub fn copy_selected(&mut self) {
        let graph = self.active_graph_ref();
        let copied: Vec<GraphNode> = self
            .selected_nodes
            .iter()
            .filter_map(|&id| graph.get_node(id).cloned())
            .collect();
        if !copied.is_empty() {
            self.clipboard = Some(copied);
        }
    }

    /// Paste nodes from clipboard
    pub fn paste(&mut self, offset_x: f32, offset_y: f32) {
        if let Some(nodes) = self.clipboard.clone() {
            self.selected_nodes.clear();
            for node in nodes {
                let new_id = self.next_node_id;
                self.next_node_id += 1;

                let mut new_node = GraphNode::new(new_id, node.label())
                    .with_position(node.x() + offset_x, node.y() + offset_y);

                for port in node.inputs() {
                    new_node.add_input(port.clone());
                }
                for port in node.outputs() {
                    new_node.add_output(port.clone());
                }

                self.active_graph_mut().add_node(new_node);
                self.selected_nodes.push(new_id);
            }
        }
    }

    /// Get statistics for current graph
    pub fn current_stats(&self) -> GraphStats {
        GraphStats::from_graph(self.active_graph_ref())
    }

    /// Filter nodes by search query
    fn node_matches_search(&self, node: &GraphNode) -> bool {
        if self.search_query.is_empty() {
            return true;
        }
        let query = self.search_query.to_lowercase();
        node.label().to_lowercase().contains(&query)
    }

    /// Initialize example graphs with nodes and edges
    fn init(&mut self) {
        if self.initialized {
            return;
        }

        // --- Behavior Tree Graph ---
        // AI behavior: Patrol → Detect Enemy → Attack
        let mut root = GraphNode::new(1, "Root").with_position(50.0, 50.0);
        root.add_output(Port::new(0, "Out", PortType::Exec));

        let mut selector = GraphNode::new(2, "Selector").with_position(200.0, 50.0);
        selector.add_input(Port::new(0, "In", PortType::Exec));
        selector.add_output(Port::new(1, "Option A", PortType::Exec));
        selector.add_output(Port::new(2, "Option B", PortType::Exec));

        let mut patrol = GraphNode::new(3, "Patrol").with_position(350.0, 20.0);
        patrol.add_input(Port::new(0, "In", PortType::Exec));
        patrol.add_output(Port::new(1, "Complete", PortType::Bool));

        let mut attack_seq = GraphNode::new(4, "Attack Sequence").with_position(350.0, 100.0);
        attack_seq.add_input(Port::new(0, "In", PortType::Exec));
        attack_seq.add_output(Port::new(1, "Target", PortType::Object));

        let mut detect = GraphNode::new(5, "Detect Enemy").with_position(500.0, 70.0);
        detect.add_input(Port::new(0, "Target", PortType::Object));
        detect.add_output(Port::new(1, "Found", PortType::Bool));

        self.behavior_tree_graph.add_node(root);
        self.behavior_tree_graph.add_node(selector);
        self.behavior_tree_graph.add_node(patrol);
        self.behavior_tree_graph.add_node(attack_seq);
        self.behavior_tree_graph.add_node(detect);

        // Connect behavior tree
        self.behavior_tree_graph.add_edge(1, 0, 2, 0); // Root → Selector
        self.behavior_tree_graph.add_edge(2, 1, 3, 0); // Selector → Patrol
        self.behavior_tree_graph.add_edge(2, 2, 4, 0); // Selector → Attack
        self.behavior_tree_graph.add_edge(4, 1, 5, 0); // Attack → Detect

        // --- Shader Graph ---
        // Texture → Color Adjust → Output
        let mut texture_in = GraphNode::new(11, "Texture Input").with_position(50.0, 50.0);
        texture_in.add_output(Port::new(0, "Color", PortType::Object));
        texture_in.add_output(Port::new(1, "UV", PortType::Number));

        let mut multiply = GraphNode::new(12, "Multiply").with_position(200.0, 50.0);
        multiply.add_input(Port::new(0, "A", PortType::Number));
        multiply.add_input(Port::new(1, "B", PortType::Number));
        multiply.add_output(Port::new(2, "Result", PortType::Number));

        let mut color_adjust = GraphNode::new(13, "Color Adjust").with_position(350.0, 50.0);
        color_adjust.add_input(Port::new(0, "Color", PortType::Object));
        color_adjust.add_input(Port::new(1, "Brightness", PortType::Number));
        color_adjust.add_output(Port::new(2, "Out", PortType::Object));

        let mut output = GraphNode::new(14, "Material Output").with_position(500.0, 50.0);
        output.add_input(Port::new(0, "Base Color", PortType::Object));

        self.shader_graph.add_node(texture_in);
        self.shader_graph.add_node(multiply);
        self.shader_graph.add_node(color_adjust);
        self.shader_graph.add_node(output);

        // Connect shader graph
        self.shader_graph.add_edge(11, 0, 13, 0); // Texture → Color Adjust (Color)
        self.shader_graph.add_edge(11, 1, 12, 0); // Texture → Multiply (UV)
        self.shader_graph.add_edge(12, 2, 13, 1); // Multiply → Color Adjust (Brightness)
        self.shader_graph.add_edge(13, 2, 14, 0); // Color Adjust → Output

        // --- Dialogue Graph ---
        // Branching conversation with choices
        let mut start_dlg = GraphNode::new(21, "Start").with_position(50.0, 50.0);
        start_dlg.add_output(Port::new(0, "Next", PortType::Exec));

        let mut greeting = GraphNode::new(22, "Greeting").with_position(200.0, 50.0);
        greeting.add_input(Port::new(0, "In", PortType::Exec));
        greeting.add_output(Port::new(1, "Option A", PortType::String));
        greeting.add_output(Port::new(2, "Option B", PortType::String));

        let mut friendly = GraphNode::new(23, "Friendly Response").with_position(350.0, 20.0);
        friendly.add_input(Port::new(0, "Choice", PortType::String));
        friendly.add_output(Port::new(1, "Next", PortType::Exec));

        let mut hostile = GraphNode::new(24, "Hostile Response").with_position(350.0, 100.0);
        hostile.add_input(Port::new(0, "Choice", PortType::String));
        hostile.add_output(Port::new(1, "Next", PortType::Exec));

        let mut end_dlg = GraphNode::new(25, "End").with_position(500.0, 60.0);
        end_dlg.add_input(Port::new(0, "In", PortType::Exec));

        self.dialogue_graph.add_node(start_dlg);
        self.dialogue_graph.add_node(greeting);
        self.dialogue_graph.add_node(friendly);
        self.dialogue_graph.add_node(hostile);
        self.dialogue_graph.add_node(end_dlg);

        // Connect dialogue graph
        self.dialogue_graph.add_edge(21, 0, 22, 0); // Start → Greeting
        self.dialogue_graph.add_edge(22, 1, 23, 0); // Greeting → Friendly (Option A)
        self.dialogue_graph.add_edge(22, 2, 24, 0); // Greeting → Hostile (Option B)
        self.dialogue_graph.add_edge(23, 1, 25, 0); // Friendly → End
        self.dialogue_graph.add_edge(24, 1, 25, 0); // Hostile → End

        self.initialized = true;
    }

    pub fn total_node_count(&self) -> usize {
        self.behavior_tree_graph.nodes().len()
            + self.shader_graph.nodes().len()
            + self.dialogue_graph.nodes().len()
    }

    pub fn show(&mut self, ui: &mut Ui) {
        self.init(); // Lazy init

        ui.heading("Graph Editor");
        ui.label("Visual scripting and node-based programming interface.");
        ui.separator();

        // ═══════════════════════════════════════════════════════════════════════════
        // GRAPH TYPE SELECTOR & MAIN TOOLBAR
        // ═══════════════════════════════════════════════════════════════════════════
        ui.horizontal(|ui| {
            ui.label("Active Graph:");
            for graph_type in [
                GraphType::BehaviorTree,
                GraphType::Shader,
                GraphType::Dialogue,
            ] {
                let selected = self.active_graph == graph_type;
                if ui
                    .selectable_label(
                        selected,
                        format!("{} {}", graph_type.icon(), graph_type.name()),
                    )
                    .clicked()
                {
                    self.active_graph = graph_type;
                    self.selected_nodes.clear();
                }
            }
        });

        ui.separator();

        // ═══════════════════════════════════════════════════════════════════════════
        // TOOLBAR
        // ═══════════════════════════════════════════════════════════════════════════
        ui.horizontal(|ui| {
            // View controls
            ui.label("Zoom:");
            if ui
                .add(egui::Slider::new(&mut self.zoom_level, 0.25..=4.0).step_by(0.25))
                .changed()
            {
                // Zoom applied to graph view (future integration)
            }

            ui.separator();

            ui.checkbox(&mut self.show_grid, "Grid");
            if self.show_grid {
                ui.checkbox(&mut self.snap_to_grid, "🧲 Snap");
            }

            ui.separator();

            ui.checkbox(&mut self.show_minimap, "🗺️ Minimap");
            ui.checkbox(&mut self.show_stats, "📈 Stats");
        });

        ui.horizontal(|ui| {
            // Edit controls
            if ui.button("Auto-Layout").clicked() {
                self.active_graph_mut().auto_layout();
            }
            if ui.button("Force Layout").clicked() {
                let params = ForceDirectedParams {
                    k: 150.0,
                    max_iterations: 300,
                    ..Default::default()
                };
                self.active_graph_mut().auto_layout_with_params(params);
            }

            ui.separator();

            let has_selection = !self.selected_nodes.is_empty();
            let has_clipboard = self.clipboard.is_some();

            if ui
                .add_enabled(has_selection, egui::Button::new("Copy"))
                .clicked()
            {
                self.copy_selected();
            }
            if ui
                .add_enabled(has_clipboard, egui::Button::new("Paste"))
                .clicked()
            {
                self.paste(50.0, 50.0);
            }
            if ui
                .add_enabled(has_selection, egui::Button::new("Delete"))
                .clicked()
            {
                info!(count = self.selected_nodes.len(), "graph: nodes deleted");
                self.delete_selected();
            }

            ui.separator();

            if ui.button("🔙 Reset Graph").clicked() {
                info!(graph = ?self.active_graph, "graph: reset");
                self.active_graph_mut().clear();
                self.initialized = false;
                self.selected_nodes.clear();
                self.init();
            }
        });

        ui.add_space(8.0);

        // ═══════════════════════════════════════════════════════════════════════════
        // SEARCH & FILTER
        // ═══════════════════════════════════════════════════════════════════════════
        ui.horizontal(|ui| {
            ui.label("[Srch]");
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("Search nodes...")
                    .desired_width(150.0),
            );

            if !self.search_query.is_empty() && ui.button("✕").clicked() {
                self.search_query.clear();
            }

            ui.separator();

            ui.label("Filter:");
            if ui
                .selectable_label(self.filter_by_type.is_none(), "All")
                .clicked()
            {
                self.filter_by_type = None;
            }
            for port_type in [
                PortType::Exec,
                PortType::Bool,
                PortType::Number,
                PortType::String,
                PortType::Object,
            ] {
                let is_selected = self.filter_by_type == Some(port_type);
                if ui
                    .selectable_label(is_selected, format!("{:?}", port_type))
                    .clicked()
                {
                    self.filter_by_type = Some(port_type);
                }
            }
        });

        ui.separator();

        // ═══════════════════════════════════════════════════════════════════════════
        // MAIN CONTENT: SIDEBAR + GRAPH VIEW
        // ═══════════════════════════════════════════════════════════════════════════
        ui.columns(2, |cols| {
            // LEFT SIDEBAR: Templates
            cols[0].set_max_width(200.0);
            cols[0].heading("Node Templates");

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(&mut cols[0], |ui| {
                    let templates = self.current_templates().to_vec();
                    for (idx, template) in templates.iter().enumerate() {
                        let is_selected = self.selected_template == Some(idx);

                        ui.horizontal(|ui| {
                            if ui.selectable_label(is_selected, &template.name).clicked() {
                                self.selected_template = Some(idx);
                            }
                            ui.label(egui::RichText::new(&template.category).small().weak());
                        });
                    }
                });

            cols[0].add_space(8.0);

            if let Some(idx) = self.selected_template {
                if cols[0].button("Add Selected Node").clicked() {
                    info!(template_idx = idx, graph = ?self.active_graph, "graph: node added from template");
                    // Add node at center of current view
                    self.add_node_from_template(idx, 250.0, 150.0);
                }
            }

            // Stats section
            if self.show_stats {
                cols[0].add_space(16.0);
                cols[0].separator();
                cols[0].heading("Graph Stats");

                let stats = self.current_stats();
                cols[0].label(format!("Nodes: {}", stats.node_count));
                cols[0].label(format!("Edges: {}", stats.edge_count));
                cols[0].label(format!("[Lt] Inputs: {}", stats.input_ports));
                cols[0].label(format!("[Rt] Outputs: {}", stats.output_ports));

                if stats.unconnected_inputs > 0 || stats.unconnected_outputs > 0 {
                    cols[0].add_space(4.0);
                    cols[0].label(
                        egui::RichText::new(format!(
                            "Unconnected: {} in, {} out",
                            stats.unconnected_inputs, stats.unconnected_outputs
                        ))
                        .color(egui::Color32::YELLOW),
                    );
                }
            }

            // RIGHT: Graph View
            cols[1].heading(format!(
                "{} {}",
                self.active_graph.icon(),
                self.active_graph.name()
            ));

            // Display current graph description
            let description = match self.active_graph {
                GraphType::BehaviorTree => "AI behavior: Patrol → Detect Enemy → Attack",
                GraphType::Shader => "Material nodes: Texture → Color Adjust → Output",
                GraphType::Dialogue => "NPC dialogue: Start → Greeting → Choices → End",
                _ => "",
            };
            cols[1].label(description);
            cols[1].add_space(4.0);

            // Show the graph
            match self.active_graph {
                GraphType::BehaviorTree => {
                    self.behavior_tree_graph.show(&mut cols[1]);
                }
                GraphType::Shader => {
                    self.shader_graph.show(&mut cols[1]);
                }
                GraphType::Dialogue => {
                    self.dialogue_graph.show(&mut cols[1]);
                }
                _ => {
                    cols[1].label("Graph type not yet implemented");
                }
            }

            // Port type legend
            cols[1].add_space(8.0);
            cols[1].horizontal(|ui| {
                ui.label(egui::RichText::new("Port Types:").small());
                ui.label(egui::RichText::new("⚪ Exec").small());
                ui.label(egui::RichText::new("[R] Bool").small());
                ui.label(egui::RichText::new("[G] Number").small());
                ui.label(egui::RichText::new("[B] String").small());
                ui.label(egui::RichText::new("[Y] Object").small());
            });
        });

        ui.add_space(16.0);

        // ═══════════════════════════════════════════════════════════════════════════
        // LEGACY COLLAPSING SECTIONS (for backwards compatibility and exploration)
        // ═══════════════════════════════════════════════════════════════════════════
        ui.collapsing("All Graphs (Collapsed View)", |ui| {
            ui.collapsing("Behavior Tree (AI Logic)", |ui| {
                ui.label("AI behavior tree for enemy NPCs");
                ui.horizontal(|ui| {
                    if ui.button("Auto-Layout").clicked() {
                        self.behavior_tree_graph.auto_layout();
                    }
                });
                self.behavior_tree_graph.show(ui);
            });

            ui.collapsing("Shader Graph (Material Nodes)", |ui| {
                ui.label("Material shader composition");
                ui.horizontal(|ui| {
                    if ui.button("Auto-Layout").clicked() {
                        self.shader_graph.auto_layout();
                    }
                });
                self.shader_graph.show(ui);
            });

            ui.collapsing("[Chat] Dialogue Graph (Branching Conversations)", |ui| {
                ui.label("NPC conversation flow");
                ui.horizontal(|ui| {
                    if ui.button("Auto-Layout").clicked() {
                        self.dialogue_graph.auto_layout();
                    }
                });
                self.dialogue_graph.show(ui);
            });
        });

        ui.add_space(8.0);

        ui.collapsing("About Graph Editor", |ui| {
            ui.label(egui::RichText::new("Features:").strong());
            ui.label("• Node graph editor with drag-and-drop nodes");
            ui.label("• Bezier curve connections between ports");
            ui.label("• Type-colored ports (Exec, Bool, Number, String, Object)");
            ui.label("• Force-directed auto-layout (spring forces + repulsion)");
            ui.label("• Template-based node creation");
            ui.label("• Copy/paste support for nodes");
            ui.label("• Search and filter functionality");
            ui.label("• Graph statistics display");
            ui.separator();
            ui.label(egui::RichText::new("Use Cases:").strong());
            ui.label("• Visual scripting (behavior trees, state machines)");
            ui.label("• Shader graph editors");
            ui.label("• Dialogue systems with branching paths");
            ui.label("• AI planning visualization");
            ui.label("• Animation blend trees");
            ui.label("• Data flow graphs (signal processing)");
            ui.separator();
            ui.label(egui::RichText::new("Keyboard Shortcuts:").strong());
            ui.label("• Ctrl+C: Copy selected nodes");
            ui.label("• Ctrl+V: Paste nodes");
            ui.label("• Delete: Remove selected nodes");
            ui.label("• Ctrl+A: Select all nodes");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // BASIC PANEL TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_graph_panel_creation() {
        let panel = GraphPanel::new();
        assert!(!panel.initialized);
        assert_eq!(panel.zoom_level, 1.0);
        assert!(panel.show_grid);
        assert!(!panel.snap_to_grid);
        assert_eq!(panel.grid_size, 20.0);
        assert!(!panel.show_minimap);
        assert!(panel.search_query.is_empty());
        assert!(panel.filter_by_type.is_none());
        assert!(panel.selected_nodes.is_empty());
        assert!(panel.clipboard.is_none());
        assert!(panel.show_stats);
    }

    #[test]
    fn test_graph_panel_initialization() {
        let mut panel = GraphPanel::new();
        panel.init();
        assert!(panel.initialized);

        // Behavior tree should have 5 nodes, 4 edges
        assert_eq!(panel.behavior_tree_graph.nodes().len(), 5);
        assert_eq!(panel.behavior_tree_graph.edges().len(), 4);

        // Shader graph should have 4 nodes, 4 edges
        assert_eq!(panel.shader_graph.nodes().len(), 4);
        assert_eq!(panel.shader_graph.edges().len(), 4);

        // Dialogue graph should have 5 nodes, 5 edges
        assert_eq!(panel.dialogue_graph.nodes().len(), 5);
        assert_eq!(panel.dialogue_graph.edges().len(), 5);
    }

    #[test]
    fn test_graph_panel_double_init_safe() {
        let mut panel = GraphPanel::new();
        panel.init();
        let nodes_count = panel.behavior_tree_graph.nodes().len();

        // Calling init again should not duplicate nodes
        panel.init();
        assert_eq!(panel.behavior_tree_graph.nodes().len(), nodes_count);
    }

    #[test]
    fn test_graph_panel_reset() {
        let mut panel = GraphPanel::new();
        panel.init();

        // Clear and re-init
        panel.behavior_tree_graph.clear();
        panel.initialized = false;
        panel.init();

        // Should still have 5 nodes
        assert_eq!(panel.behavior_tree_graph.nodes().len(), 5);
    }

    #[test]
    fn test_total_node_count() {
        let mut panel = GraphPanel::new();
        panel.init();
        // 5 + 4 + 5 = 14 total nodes
        assert_eq!(panel.total_node_count(), 14);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // GRAPH TYPE TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_graph_type_names() {
        assert_eq!(GraphType::BehaviorTree.name(), "Behavior Tree");
        assert_eq!(GraphType::Shader.name(), "Shader Graph");
        assert_eq!(GraphType::Dialogue.name(), "Dialogue Graph");
        assert_eq!(GraphType::StateMachine.name(), "State Machine");
        assert_eq!(GraphType::Animation.name(), "Animation Graph");
        assert_eq!(GraphType::Custom.name(), "Custom Graph");
    }

    #[test]
    fn test_graph_type_icons() {
        assert_eq!(GraphType::BehaviorTree.icon(), "[Brain]");
        assert_eq!(GraphType::Shader.icon(), "[Art]");
        assert_eq!(GraphType::Dialogue.icon(), "[Chat]");
        assert_eq!(GraphType::StateMachine.icon(), "⚙️");
        assert_eq!(GraphType::Animation.icon(), "[Anim]");
        assert_eq!(GraphType::Custom.icon(), "[Chart]");
    }

    #[test]
    fn test_active_graph_switching() {
        let mut panel = GraphPanel::new();
        panel.init();

        assert_eq!(panel.active_graph, GraphType::BehaviorTree);

        panel.active_graph = GraphType::Shader;
        assert_eq!(panel.active_graph_ref().nodes().len(), 4);

        panel.active_graph = GraphType::Dialogue;
        assert_eq!(panel.active_graph_ref().nodes().len(), 5);

        panel.active_graph = GraphType::BehaviorTree;
        assert_eq!(panel.active_graph_ref().nodes().len(), 5);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // NODE TEMPLATE TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_node_template_creation() {
        let template = NodeTemplate::new("Test", "Category");
        assert_eq!(template.name, "Test");
        assert_eq!(template.category, "Category");
        assert!(template.inputs.is_empty());
        assert!(template.outputs.is_empty());
    }

    #[test]
    fn test_node_template_with_ports() {
        let template = NodeTemplate::new("Add", "Math")
            .with_input("A", PortType::Number)
            .with_input("B", PortType::Number)
            .with_output("Result", PortType::Number);

        assert_eq!(template.inputs.len(), 2);
        assert_eq!(template.outputs.len(), 1);
        assert_eq!(template.inputs[0].0, "A");
        assert_eq!(template.inputs[1].0, "B");
        assert_eq!(template.outputs[0].0, "Result");
    }

    #[test]
    fn test_node_template_with_color() {
        let color = egui::Color32::RED;
        let template = NodeTemplate::new("Red Node", "Colored").with_color(color);

        assert_eq!(template.color, Some(color));
    }

    #[test]
    fn test_node_template_create_node() {
        let template = NodeTemplate::new("Math Add", "Math")
            .with_input("A", PortType::Number)
            .with_input("B", PortType::Number)
            .with_output("Result", PortType::Number);

        let node = template.create_node(42, 100.0, 200.0);

        assert_eq!(node.id(), 42);
        assert_eq!(node.label(), "Math Add");
        assert_eq!(node.x(), 100.0);
        assert_eq!(node.y(), 200.0);
        assert_eq!(node.inputs().len(), 2);
        assert_eq!(node.outputs().len(), 1);
    }

    #[test]
    fn test_templates_initialized() {
        let panel = GraphPanel::new();

        // Check behavior tree templates
        let bt_templates = panel.templates.get(&GraphType::BehaviorTree).unwrap();
        assert!(bt_templates.len() >= 6);

        // Check shader templates
        let shader_templates = panel.templates.get(&GraphType::Shader).unwrap();
        assert!(shader_templates.len() >= 7);

        // Check dialogue templates
        let dlg_templates = panel.templates.get(&GraphType::Dialogue).unwrap();
        assert!(dlg_templates.len() >= 6);
    }

    #[test]
    fn test_current_templates() {
        let mut panel = GraphPanel::new();

        panel.active_graph = GraphType::BehaviorTree;
        assert!(panel.current_templates().len() >= 6);

        panel.active_graph = GraphType::Shader;
        assert!(panel.current_templates().len() >= 7);

        panel.active_graph = GraphType::Dialogue;
        assert!(panel.current_templates().len() >= 6);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // GRAPH STATS TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_graph_stats_from_graph() {
        let mut graph = NodeGraph::new();

        let mut node1 = GraphNode::new(1, "Node1").with_position(0.0, 0.0);
        node1.add_input(Port::new(0, "In", PortType::Exec));
        node1.add_output(Port::new(1, "Out", PortType::Exec));

        let mut node2 = GraphNode::new(2, "Node2").with_position(100.0, 0.0);
        node2.add_input(Port::new(0, "In", PortType::Exec));
        node2.add_output(Port::new(1, "Out", PortType::Exec));

        graph.add_node(node1);
        graph.add_node(node2);
        graph.add_edge(1, 1, 2, 0); // Connect node1 out to node2 in

        let stats = GraphStats::from_graph(&graph);

        assert_eq!(stats.node_count, 2);
        assert_eq!(stats.edge_count, 1);
        assert_eq!(stats.input_ports, 2);
        assert_eq!(stats.output_ports, 2);
    }

    #[test]
    fn test_current_stats() {
        let mut panel = GraphPanel::new();
        panel.init();

        panel.active_graph = GraphType::BehaviorTree;
        let bt_stats = panel.current_stats();
        assert_eq!(bt_stats.node_count, 5);
        assert_eq!(bt_stats.edge_count, 4);

        panel.active_graph = GraphType::Shader;
        let shader_stats = panel.current_stats();
        assert_eq!(shader_stats.node_count, 4);
        assert_eq!(shader_stats.edge_count, 4);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // NODE MANIPULATION TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_add_node_from_template() {
        let mut panel = GraphPanel::new();
        panel.active_graph = GraphType::BehaviorTree;

        let initial_nodes = panel.active_graph_ref().nodes().len();

        // Add a node from first template
        let id = panel.add_node_from_template(0, 200.0, 100.0);
        assert!(id.is_some());
        assert_eq!(panel.active_graph_ref().nodes().len(), initial_nodes + 1);
    }

    #[test]
    fn test_add_node_with_snap_to_grid() {
        let mut panel = GraphPanel::new();
        panel.snap_to_grid = true;
        panel.grid_size = 20.0;

        let id = panel.add_node_from_template(0, 105.0, 45.0).unwrap();

        // Find the node and check position is snapped
        let node = panel.active_graph_ref().get_node(id).unwrap();
        assert_eq!(node.x(), 100.0); // 105 snaps to 100
        assert_eq!(node.y(), 40.0); // 45 snaps to 40
    }

    #[test]
    fn test_add_node_invalid_template() {
        let mut panel = GraphPanel::new();
        let id = panel.add_node_from_template(999, 0.0, 0.0);
        assert!(id.is_none());
    }

    #[test]
    fn test_delete_selected_nodes() {
        let mut panel = GraphPanel::new();
        panel.init();

        // Select first two nodes
        panel.selected_nodes = vec![1, 2];

        let initial = panel.behavior_tree_graph.nodes().len();
        panel.delete_selected();

        assert!(panel.selected_nodes.is_empty());
        assert_eq!(panel.behavior_tree_graph.nodes().len(), initial - 2);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CLIPBOARD TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_copy_selected_empty() {
        let mut panel = GraphPanel::new();
        panel.init();

        // Nothing selected
        panel.copy_selected();
        assert!(panel.clipboard.is_none());
    }

    #[test]
    fn test_copy_and_paste() {
        let mut panel = GraphPanel::new();
        panel.init();

        // Select node 1
        panel.selected_nodes = vec![1];
        panel.copy_selected();

        assert!(panel.clipboard.is_some());
        assert_eq!(panel.clipboard.as_ref().unwrap().len(), 1);

        // Paste with offset
        let initial = panel.behavior_tree_graph.nodes().len();
        panel.paste(50.0, 50.0);

        assert_eq!(panel.behavior_tree_graph.nodes().len(), initial + 1);
        assert_eq!(panel.selected_nodes.len(), 1); // Pasted node is selected
    }

    #[test]
    fn test_paste_empty_clipboard() {
        let mut panel = GraphPanel::new();
        panel.init();

        let initial = panel.behavior_tree_graph.nodes().len();
        panel.paste(0.0, 0.0);

        // Nothing should be pasted
        assert_eq!(panel.behavior_tree_graph.nodes().len(), initial);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SEARCH & FILTER TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_search_empty_query() {
        let panel = GraphPanel::new();
        let node = GraphNode::new(1, "Test Node").with_position(0.0, 0.0);

        assert!(panel.node_matches_search(&node));
    }

    #[test]
    fn test_search_matching() {
        let mut panel = GraphPanel::new();
        panel.search_query = "test".to_string();

        let matching = GraphNode::new(1, "Test Node").with_position(0.0, 0.0);
        let not_matching = GraphNode::new(2, "Other Node").with_position(0.0, 0.0);

        assert!(panel.node_matches_search(&matching));
        assert!(!panel.node_matches_search(&not_matching));
    }

    #[test]
    fn test_search_case_insensitive() {
        let mut panel = GraphPanel::new();
        panel.search_query = "TEST".to_string();

        let node = GraphNode::new(1, "test node").with_position(0.0, 0.0);
        assert!(panel.node_matches_search(&node));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // VIEW SETTINGS TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_zoom_level_default() {
        let panel = GraphPanel::new();
        assert_eq!(panel.zoom_level, 1.0);
    }

    #[test]
    fn test_grid_settings_default() {
        let panel = GraphPanel::new();
        assert!(panel.show_grid);
        assert!(!panel.snap_to_grid);
        assert_eq!(panel.grid_size, 20.0);
    }

    #[test]
    fn test_minimap_default() {
        let panel = GraphPanel::new();
        assert!(!panel.show_minimap);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DEFAULT TRAIT TESTS
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_default_trait() {
        let panel: GraphPanel = Default::default();
        assert!(!panel.initialized);
        assert_eq!(panel.active_graph, GraphType::BehaviorTree);
    }

    #[test]
    fn test_next_node_id_increments() {
        let mut panel = GraphPanel::new();
        let id1 = panel.add_node_from_template(0, 0.0, 0.0).unwrap();
        let id2 = panel.add_node_from_template(0, 50.0, 0.0).unwrap();

        assert!(id2 > id1);
    }

    // ===== GraphType Enum Tests =====

    #[test]
    fn test_graph_type_display() {
        for graph_type in GraphType::all() {
            let display = format!("{}", graph_type);
            assert!(
                display.contains(graph_type.name()),
                "Display should contain name"
            );
        }
    }

    #[test]
    fn test_graph_type_all_variants() {
        let variants = GraphType::all();
        assert_eq!(variants.len(), 6, "Expected 6 graph type variants");
        assert!(variants.contains(&GraphType::BehaviorTree));
        assert!(variants.contains(&GraphType::Shader));
        assert!(variants.contains(&GraphType::Dialogue));
        assert!(variants.contains(&GraphType::StateMachine));
    }

    #[test]
    fn test_graph_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for gt in GraphType::all() {
            set.insert(*gt);
        }
        assert_eq!(set.len(), GraphType::all().len());
    }

    #[test]
    fn test_graph_type_name() {
        assert_eq!(GraphType::BehaviorTree.name(), "Behavior Tree");
        assert_eq!(GraphType::Shader.name(), "Shader Graph");
        assert_eq!(GraphType::Dialogue.name(), "Dialogue Graph");
    }

    #[test]
    fn test_graph_type_icon() {
        for gt in GraphType::all() {
            let icon = gt.icon();
            assert!(!icon.is_empty(), "Icon should not be empty");
        }
    }
}
