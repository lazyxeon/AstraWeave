# NodeGraph Tutorial

Create visual node-based editors for AI behavior trees, shader graphs, and dialogue systems.

---

## Table of Contents

1. [Overview](#overview)
2. [Core Concepts](#core-concepts)
3. [Behavior Trees](#behavior-trees)
4. [Shader Graphs](#shader-graphs)
5. [Dialogue Systems](#dialogue-systems)
6. [Advanced Features](#advanced-features)

---

## Overview

NodeGraph enables visual programming for:

- **Behavior Trees** - AI decision-making (game AI, NPCs)
- **Shader Graphs** - Visual shader authoring
- **Dialogue Systems** - Branching conversations
- **State Machines** - Game state flow
- **Data Flow Graphs** - Processing pipelines

### Key Components

```rust
use astract::graph::{
    NodeGraph,    // The graph container
    GraphNode,    // Individual nodes
    GraphEdge,    // Connections between nodes
    Port,         // Input/output ports
    PortType,     // Data type of ports
};
```

---

## Core Concepts

### Creating a Graph

```rust
use astract::prelude::egui::*;
use astract::graph::NodeGraph;

struct MyApp {
    graph: NodeGraph,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            graph: NodeGraph::new(),
        }
    }
}

impl MyApp {
    fn show(&mut self, ui: &mut Ui) {
        self.graph.show(ui);
    }
}
```

### Creating Nodes

```rust
use astract::graph::{GraphNode, Port, PortType};

// Create a node
let mut start_node = GraphNode::new(1, "Start");

// Add input ports
start_node.add_input(Port::new(
    0,                    // Port index
    "In",                 // Port label
    PortType::Exec,       // Port type
));

// Add output ports
start_node.add_output(Port::new(
    0,
    "Out",
    PortType::Exec,
));

// Optional: Set position
let start_node = start_node.with_position(100.0, 50.0);

// Add to graph
let start_id = graph.add_node(start_node);
```

### Port Types

```rust
pub enum PortType {
    Exec,    // Execution flow (white) - control flow
    Bool,    // Boolean data (red) - true/false
    Number,  // Numeric data (green) - f64, i32
    String,  // String data (blue) - text
    Object,  // Object reference (yellow) - complex data
}
```

### Connecting Nodes

```rust
// Connect output of node 1 to input of node 2
graph.add_edge(
    start_id,    // Source node ID
    0,           // Source port index (output)
    action_id,   // Target node ID
    0,           // Target port index (input)
);
```

---

## Behavior Trees

AI decision-making with hierarchical logic.

### Simple Behavior Tree

```rust
fn create_simple_behavior_tree() -> NodeGraph {
    let mut graph = NodeGraph::new();
    
    // 1. Start node
    let mut start = GraphNode::new(1, "Start");
    start.add_output(Port::new(0, "Out", PortType::Exec));
    let start = start.with_position(50.0, 100.0);
    let start_id = graph.add_node(start);
    
    // 2. Selector node (try branches in order)
    let mut selector = GraphNode::new(2, "Selector");
    selector.add_input(Port::new(0, "In", PortType::Exec));
    selector.add_output(Port::new(0, "Option 1", PortType::Exec));
    selector.add_output(Port::new(1, "Option 2", PortType::Exec));
    let selector = selector.with_position(200.0, 100.0);
    let selector_id = graph.add_node(selector);
    
    // 3. Action nodes
    let mut attack = GraphNode::new(3, "Attack");
    attack.add_input(Port::new(0, "Execute", PortType::Exec));
    let attack = attack.with_position(350.0, 50.0);
    let attack_id = graph.add_node(attack);
    
    let mut flee = GraphNode::new(4, "Flee");
    flee.add_input(Port::new(0, "Execute", PortType::Exec));
    let flee = flee.with_position(350.0, 150.0);
    let flee_id = graph.add_node(flee);
    
    // Connect nodes
    graph.add_edge(start_id, 0, selector_id, 0);
    graph.add_edge(selector_id, 0, attack_id, 0);
    graph.add_edge(selector_id, 1, flee_id, 0);
    
    graph
}
```

### Advanced Behavior Tree (Guards + Decorators)

```rust
fn create_advanced_behavior_tree() -> NodeGraph {
    let mut graph = NodeGraph::new();
    
    // 1. Root: Sequence (run all in order)
    let mut root = GraphNode::new(1, "Root Sequence");
    root.add_output(Port::new(0, "1", PortType::Exec));
    root.add_output(Port::new(1, "2", PortType::Exec));
    root.add_output(Port::new(2, "3", PortType::Exec));
    let root = root.with_position(50.0, 150.0);
    let root_id = graph.add_node(root);
    
    // 2. Check Health (condition guard)
    let mut check_health = GraphNode::new(2, "Health > 50%");
    check_health.add_input(Port::new(0, "In", PortType::Exec));
    check_health.add_output(Port::new(0, "Pass", PortType::Bool));
    let check_health = check_health.with_position(250.0, 50.0);
    let health_id = graph.add_node(check_health);
    
    // 3. Selector: Choose action
    let mut selector = GraphNode::new(3, "Action Selector");
    selector.add_input(Port::new(0, "In", PortType::Exec));
    selector.add_output(Port::new(0, "Aggressive", PortType::Exec));
    selector.add_output(Port::new(1, "Defensive", PortType::Exec));
    let selector = selector.with_position(250.0, 150.0);
    let selector_id = graph.add_node(selector);
    
    // 4. Attack action
    let mut attack = GraphNode::new(4, "Attack Enemy");
    attack.add_input(Port::new(0, "Do", PortType::Exec));
    let attack = attack.with_position(450.0, 100.0);
    let attack_id = graph.add_node(attack);
    
    // 5. Heal action
    let mut heal = GraphNode::new(5, "Heal Self");
    heal.add_input(Port::new(0, "Do", PortType::Exec));
    let heal = heal.with_position(450.0, 200.0);
    let heal_id = graph.add_node(heal);
    
    // 6. End
    let mut end = GraphNode::new(6, "End Turn");
    end.add_input(Port::new(0, "In", PortType::Exec));
    let end = end.with_position(250.0, 250.0);
    let end_id = graph.add_node(end);
    
    // Connect execution flow
    graph.add_edge(root_id, 0, health_id, 0);
    graph.add_edge(root_id, 1, selector_id, 0);
    graph.add_edge(root_id, 2, end_id, 0);
    
    // Connect selector outputs
    graph.add_edge(selector_id, 0, attack_id, 0);
    graph.add_edge(selector_id, 1, heal_id, 0);
    
    graph
}
```

---

## Shader Graphs

Visual shader authoring (material editor style).

### Basic Shader Graph

```rust
fn create_simple_shader_graph() -> NodeGraph {
    let mut graph = NodeGraph::new();
    
    // 1. Texture Sample
    let mut texture = GraphNode::new(1, "Texture");
    texture.add_output(Port::new(0, "RGB", PortType::Object));
    texture.add_output(Port::new(1, "A", PortType::Number));
    let texture = texture.with_position(50.0, 100.0);
    let tex_id = graph.add_node(texture);
    
    // 2. Color Multiply
    let mut multiply = GraphNode::new(2, "Multiply");
    multiply.add_input(Port::new(0, "A", PortType::Object));
    multiply.add_input(Port::new(1, "B", PortType::Object));
    multiply.add_output(Port::new(0, "Result", PortType::Object));
    let multiply = multiply.with_position(250.0, 100.0);
    let mult_id = graph.add_node(multiply);
    
    // 3. Tint Color
    let mut tint = GraphNode::new(3, "Tint Color");
    tint.add_output(Port::new(0, "Color", PortType::Object));
    let tint = tint.with_position(50.0, 200.0);
    let tint_id = graph.add_node(tint);
    
    // 4. Output
    let mut output = GraphNode::new(4, "Output");
    output.add_input(Port::new(0, "Color", PortType::Object));
    output.add_input(Port::new(1, "Alpha", PortType::Number));
    let output = output.with_position(450.0, 100.0);
    let out_id = graph.add_node(output);
    
    // Connect shader nodes
    graph.add_edge(tex_id, 0, mult_id, 0);     // Texture RGB → Multiply A
    graph.add_edge(tint_id, 0, mult_id, 1);    // Tint → Multiply B
    graph.add_edge(mult_id, 0, out_id, 0);     // Multiply Result → Output Color
    graph.add_edge(tex_id, 1, out_id, 1);      // Texture Alpha → Output Alpha
    
    graph
}
```

### Advanced Material Graph

```rust
fn create_pbr_material_graph() -> NodeGraph {
    let mut graph = NodeGraph::new();
    
    // Texture inputs
    let mut albedo_tex = GraphNode::new(1, "Albedo Texture");
    albedo_tex.add_output(Port::new(0, "RGB", PortType::Object));
    let albedo_tex = albedo_tex.with_position(50.0, 50.0);
    let albedo_id = graph.add_node(albedo_tex);
    
    let mut normal_tex = GraphNode::new(2, "Normal Map");
    normal_tex.add_output(Port::new(0, "Normal", PortType::Object));
    let normal_tex = normal_tex.with_position(50.0, 150.0);
    let normal_id = graph.add_node(normal_tex);
    
    let mut roughness_tex = GraphNode::new(3, "Roughness");
    roughness_tex.add_output(Port::new(0, "R", PortType::Number));
    let roughness_tex = roughness_tex.with_position(50.0, 250.0);
    let rough_id = graph.add_node(roughness_tex);
    
    let mut metallic_tex = GraphNode::new(4, "Metallic");
    metallic_tex.add_output(Port::new(0, "M", PortType::Number));
    let metallic_tex = metallic_tex.with_position(50.0, 350.0);
    let metal_id = graph.add_node(metallic_tex);
    
    // PBR Output
    let mut pbr_output = GraphNode::new(5, "PBR Output");
    pbr_output.add_input(Port::new(0, "Albedo", PortType::Object));
    pbr_output.add_input(Port::new(1, "Normal", PortType::Object));
    pbr_output.add_input(Port::new(2, "Roughness", PortType::Number));
    pbr_output.add_input(Port::new(3, "Metallic", PortType::Number));
    let pbr_output = pbr_output.with_position(400.0, 200.0);
    let output_id = graph.add_node(pbr_output);
    
    // Connect material channels
    graph.add_edge(albedo_id, 0, output_id, 0);
    graph.add_edge(normal_id, 0, output_id, 1);
    graph.add_edge(rough_id, 0, output_id, 2);
    graph.add_edge(metal_id, 0, output_id, 3);
    
    graph
}
```

---

## Dialogue Systems

Branching narrative and conversations.

### Simple Dialogue

```rust
fn create_simple_dialogue() -> NodeGraph {
    let mut graph = NodeGraph::new();
    
    // 1. Start
    let mut start = GraphNode::new(1, "Hello, traveler!");
    start.add_output(Port::new(0, "Continue", PortType::Exec));
    let start = start.with_position(50.0, 100.0);
    let start_id = graph.add_node(start);
    
    // 2. Player choice
    let mut choice = GraphNode::new(2, "[Player Choice]");
    choice.add_input(Port::new(0, "In", PortType::Exec));
    choice.add_output(Port::new(0, "\"Who are you?\"", PortType::Exec));
    choice.add_output(Port::new(1, "\"Goodbye\"", PortType::Exec));
    let choice = choice.with_position(250.0, 100.0);
    let choice_id = graph.add_node(choice);
    
    // 3. Response 1
    let mut response1 = GraphNode::new(3, "I'm a merchant.");
    response1.add_input(Port::new(0, "In", PortType::Exec));
    response1.add_output(Port::new(0, "Continue", PortType::Exec));
    let response1 = response1.with_position(450.0, 50.0);
    let resp1_id = graph.add_node(response1);
    
    // 4. Response 2
    let mut response2 = GraphNode::new(4, "Safe travels!");
    response2.add_input(Port::new(0, "In", PortType::Exec));
    let response2 = response2.with_position(450.0, 150.0);
    let resp2_id = graph.add_node(response2);
    
    // 5. Shop option
    let mut shop = GraphNode::new(5, "[Open Shop]");
    shop.add_input(Port::new(0, "In", PortType::Exec));
    let shop = shop.with_position(650.0, 50.0);
    let shop_id = graph.add_node(shop);
    
    // Connect dialogue flow
    graph.add_edge(start_id, 0, choice_id, 0);
    graph.add_edge(choice_id, 0, resp1_id, 0);
    graph.add_edge(choice_id, 1, resp2_id, 0);
    graph.add_edge(resp1_id, 0, shop_id, 0);
    
    graph
}
```

### Branching Quest Dialogue

```rust
fn create_quest_dialogue() -> NodeGraph {
    let mut graph = NodeGraph::new();
    
    // Quest giver introduction
    let mut intro = GraphNode::new(1, "NPC: I need help!");
    intro.add_output(Port::new(0, "Next", PortType::Exec));
    let intro = intro.with_position(50.0, 150.0);
    let intro_id = graph.add_node(intro);
    
    // Player response
    let mut response = GraphNode::new(2, "[Player]");
    response.add_input(Port::new(0, "In", PortType::Exec));
    response.add_output(Port::new(0, "\"What's wrong?\"", PortType::Exec));
    response.add_output(Port::new(1, "\"Not interested\"", PortType::Exec));
    let response = response.with_position(250.0, 150.0);
    let resp_id = graph.add_node(response);
    
    // Accept quest branch
    let mut accept = GraphNode::new(3, "Monsters attack!");
    accept.add_input(Port::new(0, "In", PortType::Exec));
    accept.add_output(Port::new(0, "Next", PortType::Exec));
    let accept = accept.with_position(450.0, 100.0);
    let accept_id = graph.add_node(accept);
    
    let mut quest_start = GraphNode::new(4, "[Quest Started]");
    quest_start.add_input(Port::new(0, "In", PortType::Exec));
    let quest_start = quest_start.with_position(650.0, 100.0);
    let quest_id = graph.add_node(quest_start);
    
    // Reject quest branch
    let mut reject = GraphNode::new(5, "Oh... okay.");
    reject.add_input(Port::new(0, "In", PortType::Exec));
    let reject = reject.with_position(450.0, 200.0);
    let reject_id = graph.add_node(reject);
    
    // Connect dialogue branches
    graph.add_edge(intro_id, 0, resp_id, 0);
    graph.add_edge(resp_id, 0, accept_id, 0);
    graph.add_edge(resp_id, 1, reject_id, 0);
    graph.add_edge(accept_id, 0, quest_id, 0);
    
    graph
}
```

---

## Advanced Features

### Dynamic Node Creation

```rust
struct GraphEditor {
    graph: NodeGraph,
    next_id: usize,
}

impl GraphEditor {
    fn add_custom_node(&mut self, label: &str, x: f32, y: f32) -> usize {
        let mut node = GraphNode::new(self.next_id, label);
        node.add_input(Port::new(0, "In", PortType::Exec));
        node.add_output(Port::new(0, "Out", PortType::Exec));
        let node = node.with_position(x, y);
        
        let id = self.graph.add_node(node);
        self.next_id += 1;
        id
    }
    
    fn show(&mut self, ui: &mut Ui) {
        // Toolbar
        ui.horizontal(|ui| {
            if ui.button("➕ Add Node").clicked() {
                self.add_custom_node("New Node", 100.0, 100.0);
            }
            if ui.button("🔗 Connect").clicked() {
                // Connection logic
            }
        });
        
        ui.separator();
        
        // Graph view
        self.graph.show(ui);
    }
}
```

### Graph Validation

```rust
fn validate_graph(graph: &NodeGraph) -> Result<(), String> {
    // Check for cycles
    if has_cycle(graph) {
        return Err("Graph contains cycles!".to_string());
    }
    
    // Check for disconnected nodes
    if has_disconnected_nodes(graph) {
        return Err("Graph has disconnected nodes!".to_string());
    }
    
    // Check port type compatibility
    for edge in graph.edges() {
        let src_port = edge.source_port_type();
        let dst_port = edge.target_port_type();
        
        if !ports_compatible(src_port, dst_port) {
            return Err(format!(
                "Incompatible port types: {:?} → {:?}",
                src_port, dst_port
            ));
        }
    }
    
    Ok(())
}
```

### Graph Serialization

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct GraphSave {
    nodes: Vec<NodeData>,
    edges: Vec<EdgeData>,
}

fn save_graph(graph: &NodeGraph, path: &str) -> std::io::Result<()> {
    let save_data = GraphSave {
        nodes: graph.nodes().map(|n| n.to_data()).collect(),
        edges: graph.edges().map(|e| e.to_data()).collect(),
    };
    
    let json = serde_json::to_string_pretty(&save_data)?;
    std::fs::write(path, json)?;
    Ok(())
}

fn load_graph(path: &str) -> std::io::Result<NodeGraph> {
    let json = std::fs::read_to_string(path)?;
    let save_data: GraphSave = serde_json::from_str(&json)?;
    
    let mut graph = NodeGraph::new();
    for node_data in save_data.nodes {
        graph.add_node(node_data.to_node());
    }
    for edge_data in save_data.edges {
        graph.add_edge(
            edge_data.source_id,
            edge_data.source_port,
            edge_data.target_id,
            edge_data.target_port,
        );
    }
    
    Ok(graph)
}
```

---

## Best Practices

### 1. Node ID Management

✅ **DO**: Use unique, sequential IDs
```rust
let mut next_id = 1;
let node1 = GraphNode::new(next_id, "Node 1");
next_id += 1;
let node2 = GraphNode::new(next_id, "Node 2");
```

❌ **DON'T**: Reuse IDs
```rust
let node1 = GraphNode::new(1, "Node 1");
let node2 = GraphNode::new(1, "Node 2");  // ❌ Duplicate ID!
```

### 2. Port Types

✅ **DO**: Use PortType::Exec for control flow
```rust
port.add_output(Port::new(0, "Execute", PortType::Exec));
```

❌ **DON'T**: Use Object for everything
```rust
port.add_output(Port::new(0, "Execute", PortType::Object));  // ❌ Wrong semantics
```

### 3. Layout

✅ **DO**: Space nodes clearly
```rust
node1.with_position(100.0, 100.0);
node2.with_position(300.0, 100.0);  // 200px horizontal gap
```

❌ **DON'T**: Overlap nodes
```rust
node1.with_position(100.0, 100.0);
node2.with_position(110.0, 105.0);  // ❌ Too close!
```

---

## Next Steps

- **[Animation Tutorial](./ANIMATION_TUTORIAL.md)** - Smooth transitions
- **[Gallery Example](../../examples/astract_gallery/)** - See graphs in action
- **[API Reference](./API_REFERENCE.md)** - Complete API docs

---

**Build powerful visual editors! 🔗**
