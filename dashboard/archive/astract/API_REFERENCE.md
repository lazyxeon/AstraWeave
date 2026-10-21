# Astract API Reference

Complete API documentation for the Astract UI framework.

---

## Table of Contents

1. [Charts API](#charts-api)
2. [Graph API](#graph-api)
3. [Advanced Widgets API](#advanced-widgets-api)
4. [Animation API](#animation-api)
5. [Type Reference](#type-reference)

---

## Charts API

### LineChart

**Purpose**: Display time-series or continuous data as connected lines.

**Type Signature**:
```rust
pub struct LineChart {
    title: String,
    series: Vec<DataSeries>,
}

pub struct DataSeries {
    pub name: String,
    pub points: Vec<Point>,
    pub color: Color32,
}

pub type Point = (f64, f64);
```

**Methods**:

#### `new(title: impl Into<String>) -> Self`
Create a new line chart with the given title.

```rust
let chart = LineChart::new("Performance Over Time");
```

#### `add_series(name: impl Into<String>, points: Vec<Point>, color: Color32) -> &mut Self`
Add a data series to the chart.

**Parameters**:
- `name`: Series name for legend
- `points`: Vector of `(x, y)` coordinate tuples
- `color`: Line color

```rust
chart.add_series(
    "FPS",
    vec![(0.0, 60.0), (1.0, 58.0), (2.0, 62.0)],
    Color32::GREEN
);
```

#### `show(&self, ui: &mut Ui)`
Render the chart in the given UI context.

```rust
chart.show(ui);
```

**Example**:
```rust
let mut chart = LineChart::new("CPU Usage");
chart.add_series("Core 1", cpu1_data, Color32::RED);
chart.add_series("Core 2", cpu2_data, Color32::BLUE);
chart.show(ui);
```

---

### BarChart

**Purpose**: Compare categorical data with grouped bars.

**Type Signature**:
```rust
pub struct BarChart {
    title: String,
    groups: Vec<BarGroup>,
}

pub struct BarGroup {
    pub category: String,
    pub bars: Vec<Bar>,
}

pub struct Bar {
    pub label: String,
    pub value: f64,
    pub color: Color32,
}
```

**Methods**:

#### `new(title: impl Into<String>) -> Self`
Create a new bar chart.

```rust
let chart = BarChart::new("Sales by Region");
```

#### `add_group(&mut self, group: BarGroup) -> &mut Self`
Add a group of bars to the chart.

```rust
let group = BarGroup {
    category: "Q1".to_string(),
    bars: vec![
        Bar { label: "North".into(), value: 1000.0, color: Color32::RED },
        Bar { label: "South".into(), value: 800.0, color: Color32::BLUE },
    ],
};
chart.add_group(group);
```

#### `show(&self, ui: &mut Ui)`
Render the chart.

```rust
chart.show(ui);
```

**Example**:
```rust
let mut chart = BarChart::new("Quarterly Revenue");

chart.add_group(BarGroup {
    category: "Q1 2024".into(),
    bars: vec![
        Bar { label: "Product A".into(), value: 65000.0, color: Color32::from_rgb(255, 99, 71) },
        Bar { label: "Product B".into(), value: 45000.0, color: Color32::from_rgb(70, 130, 180) },
    ],
});

chart.show(ui);
```

---

### ScatterPlot

**Purpose**: Visualize point clusters and correlations.

**Type Signature**:
```rust
pub struct ScatterPlot {
    title: String,
    clusters: Vec<PointCluster>,
}

pub struct PointCluster {
    pub name: String,
    pub points: Vec<Point>,
    pub color: Color32,
}
```

**Methods**:

#### `new(title: impl Into<String>) -> Self`
Create a new scatter plot.

```rust
let scatter = ScatterPlot::new("Customer Segments");
```

#### `add_cluster(&mut self, cluster: PointCluster) -> &mut Self`
Add a cluster of points.

```rust
let cluster = PointCluster::new(
    "High Value",
    vec![(50.0, 80.0), (55.0, 85.0)],
    Color32::RED
);
scatter.add_cluster(cluster);
```

#### `show(&self, ui: &mut Ui)`
Render the scatter plot.

```rust
scatter.show(ui);
```

**Example**:
```rust
let mut scatter = ScatterPlot::new("Data Distribution");

scatter.add_cluster(PointCluster::new(
    "Group A",
    vec![(10.0, 20.0), (12.0, 22.0), (15.0, 25.0)],
    Color32::BLUE
));

scatter.add_cluster(PointCluster::new(
    "Group B",
    vec![(50.0, 60.0), (52.0, 62.0), (55.0, 65.0)],
    Color32::RED
));

scatter.show(ui);
```

---

## Graph API

### NodeGraph

**Purpose**: Visual node-based editor for behavior trees, shaders, dialogue.

**Type Signature**:
```rust
pub struct NodeGraph {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}
```

**Methods**:

#### `new() -> Self`
Create a new empty graph.

```rust
let graph = NodeGraph::new();
```

#### `add_node(&mut self, node: GraphNode) -> usize`
Add a node to the graph, returns node ID.

```rust
let node = GraphNode::new(1, "Start");
let node_id = graph.add_node(node);
```

#### `add_edge(&mut self, source_id: usize, source_port: usize, target_id: usize, target_port: usize)`
Connect two nodes.

**Parameters**:
- `source_id`: ID of source node
- `source_port`: Index of output port on source
- `target_id`: ID of target node
- `target_port`: Index of input port on target

```rust
graph.add_edge(start_id, 0, action_id, 0);
```

#### `show(&mut self, ui: &mut Ui) -> Option<usize>`
Render the graph, returns clicked node ID if any.

```rust
if let Some(clicked_id) = graph.show(ui) {
    println!("Node {} clicked", clicked_id);
}
```

---

### GraphNode

**Type Signature**:
```rust
pub struct GraphNode {
    id: usize,
    label: String,
    inputs: Vec<Port>,
    outputs: Vec<Port>,
    position: Option<(f32, f32)>,
}
```

**Methods**:

#### `new(id: usize, label: impl Into<String>) -> Self`
Create a new node.

```rust
let node = GraphNode::new(1, "Selector");
```

#### `add_input(&mut self, port: Port) -> &mut Self`
Add an input port.

```rust
node.add_input(Port::new(0, "In", PortType::Exec));
```

#### `add_output(&mut self, port: Port) -> &mut Self`
Add an output port.

```rust
node.add_output(Port::new(0, "Out", PortType::Exec));
```

#### `with_position(mut self, x: f32, y: f32) -> Self`
Set node position (builder pattern).

```rust
let node = GraphNode::new(1, "Start")
    .with_position(100.0, 50.0);
```

**Example**:
```rust
let mut node = GraphNode::new(1, "Attack");
node.add_input(Port::new(0, "Execute", PortType::Exec));
node.add_output(Port::new(0, "Done", PortType::Exec));
node.add_output(Port::new(1, "Damage", PortType::Number));
let node = node.with_position(250.0, 100.0);
```

---

### Port

**Type Signature**:
```rust
pub struct Port {
    pub index: usize,
    pub label: String,
    pub port_type: PortType,
}
```

**Methods**:

#### `new(index: usize, label: impl Into<String>, port_type: PortType) -> Self`
Create a new port.

```rust
let port = Port::new(0, "Output", PortType::Exec);
```

---

### PortType

**Type Signature**:
```rust
pub enum PortType {
    Exec,    // Execution flow (white)
    Bool,    // Boolean data (red)
    Number,  // Numeric data (green)
    String,  // String data (blue)
    Object,  // Object reference (yellow)
}
```

**Color Coding**:
- **Exec** (white): Control flow, execution order
- **Bool** (red): True/false values
- **Number** (green): Integers, floats, numeric data
- **String** (blue): Text data
- **Object** (yellow): Complex objects, references

---

## Advanced Widgets API

### ColorPicker

**Purpose**: RGBA color selection with preview.

**Type Signature**:
```rust
pub struct ColorPicker {
    color: Color32,
    show_alpha: bool,
}
```

**Methods**:

#### `new() -> Self`
Create a new color picker (default: white, alpha shown).

```rust
let picker = ColorPicker::new();
```

#### `with_color(mut self, color: Color32) -> Self`
Set initial color (builder pattern).

```rust
let picker = ColorPicker::new()
    .with_color(Color32::RED);
```

#### `show_alpha(mut self, show: bool) -> Self`
Enable/disable alpha channel (builder pattern).

```rust
let picker = ColorPicker::new()
    .with_color(Color32::BLUE)
    .show_alpha(false);  // RGB only
```

#### `show(&mut self, ui: &mut Ui)`
Render the color picker (updates internal state).

```rust
picker.show(ui);
```

#### `color(&self) -> Color32`
Get current selected color.

```rust
let selected = picker.color();
```

**Example**:
```rust
struct App {
    picker: ColorPicker,
}

impl Default for App {
    fn default() -> Self {
        Self {
            picker: ColorPicker::new()
                .with_color(Color32::from_rgb(70, 130, 180))
                .show_alpha(true),
        }
    }
}

impl App {
    fn show(&mut self, ui: &mut Ui) {
        self.picker.show(ui);
        let color = self.picker.color();
        
        // Use color
        ui.colored_label(color, "Preview text");
    }
}
```

---

### TreeView

**Purpose**: Hierarchical data visualization with expand/collapse.

**Type Signature**:
```rust
pub struct TreeView {
    nodes: Vec<TreeNode>,
    expanded: HashSet<usize>,
}

pub struct TreeNode {
    pub id: usize,
    pub label: String,
    pub icon: Option<String>,
    pub parent: Option<usize>,
}
```

**Methods**:

#### `new() -> Self`
Create a new empty tree.

```rust
let tree = TreeView::new();
```

#### `add_node(&mut self, node: TreeNode) -> usize`
Add a root-level node, returns node ID.

```rust
let root = tree.add_node(
    TreeNode::new(1, "Project").with_icon("📁")
);
```

#### `add_child(&mut self, parent_id: usize, node: TreeNode) -> usize`
Add a child node under a parent.

```rust
let child = tree.add_child(
    root,
    TreeNode::new(2, "src").with_icon("📁")
);
```

#### `show(&mut self, ui: &mut Ui) -> Option<usize>`
Render the tree, returns clicked node ID if any.

```rust
if let Some(clicked) = tree.show(ui) {
    println!("Clicked node: {}", clicked);
}
```

**Example**:
```rust
let mut tree = TreeView::new();

let root = tree.add_node(
    TreeNode::new(1, "my_project").with_icon("📁")
);

let src = tree.add_child(
    root,
    TreeNode::new(2, "src").with_icon("📁")
);

tree.add_child(src, TreeNode::new(3, "main.rs").with_icon("📄"));
tree.add_child(src, TreeNode::new(4, "lib.rs").with_icon("📄"));
tree.add_child(root, TreeNode::new(5, "Cargo.toml").with_icon("⚙️"));
```

---

### RangeSlider

**Purpose**: Dual-handle range selection.

**Type Signature**:
```rust
pub struct RangeSlider {
    range_min: f64,
    range_max: f64,
    current_min: f64,
    current_max: f64,
}
```

**Methods**:

#### `new(range_min: f64, range_max: f64) -> Self`
Create a new range slider with overall range.

```rust
let slider = RangeSlider::new(0.0, 100.0);
```

#### `with_min(mut self, min: f64) -> Self`
Set initial minimum value (builder pattern).

```rust
let slider = RangeSlider::new(0.0, 100.0)
    .with_min(25.0);
```

#### `with_max(mut self, max: f64) -> Self`
Set initial maximum value (builder pattern).

```rust
let slider = RangeSlider::new(0.0, 100.0)
    .with_min(25.0)
    .with_max(75.0);
```

#### `show(&mut self, ui: &mut Ui)`
Render the slider (updates internal state).

```rust
slider.show(ui);
```

#### `min_value(&self) -> f64`
Get current minimum value.

```rust
let min = slider.min_value();
```

#### `max_value(&self) -> f64`
Get current maximum value.

```rust
let max = slider.max_value();
```

**Example**:
```rust
struct App {
    price_range: RangeSlider,
}

impl Default for App {
    fn default() -> Self {
        Self {
            price_range: RangeSlider::new(0.0, 1000.0)
                .with_min(100.0)
                .with_max(500.0),
        }
    }
}

impl App {
    fn show(&mut self, ui: &mut Ui) {
        ui.label("Price Range:");
        self.price_range.show(ui);
        
        let min = self.price_range.min_value();
        let max = self.price_range.max_value();
        ui.label(format!("${:.0} - ${:.0}", min, max));
    }
}
```

---

## Animation API

### Tween<T>

**Purpose**: Interpolate values over time with easing.

**Type Signature**:
```rust
pub struct Tween<T: Linear> {
    start: T,
    end: T,
    duration: f32,
    elapsed: f32,
    easing: EasingFunction,
    state: AnimationState,
}
```

**Methods**:

#### `new(start: T, end: T, duration: f32) -> Self`
Create a new tween animation.

**Parameters**:
- `start`: Starting value
- `end`: Ending value
- `duration`: Duration in seconds

```rust
let tween = Tween::new(0.0, 100.0, 2.0);
```

#### `with_easing(mut self, easing: EasingFunction) -> Self`
Set easing function (builder pattern).

```rust
let tween = Tween::new(0.0, 100.0, 2.0)
    .with_easing(EasingFunction::ElasticOut);
```

#### `play(&mut self)`
Start/resume animation.

```rust
tween.play();
```

#### `pause(&mut self)`
Pause animation.

```rust
tween.pause();
```

#### `reset(&mut self)`
Reset to start.

```rust
tween.reset();
```

#### `update(&mut self, dt: f32)`
Update animation (call every frame).

**Parameters**:
- `dt`: Delta time in seconds

```rust
let dt = ctx.input(|i| i.stable_dt);
tween.update(dt);
```

#### `value(&self) -> T`
Get current interpolated value.

```rust
let current = tween.value();
```

#### `is_finished(&self) -> bool`
Check if animation completed.

```rust
if tween.is_finished() {
    tween.reset();
    tween.play();  // Loop
}
```

#### `state(&self) -> AnimationState`
Get current state (Playing, Paused, Stopped).

```rust
match tween.state() {
    AnimationState::Playing => { /* ... */ }
    AnimationState::Paused => { /* ... */ }
    AnimationState::Stopped => { /* ... */ }
}
```

**Example**:
```rust
struct App {
    position: Tween<f32>,
}

impl Default for App {
    fn default() -> Self {
        let mut tween = Tween::new(0.0, 200.0, 2.0)
            .with_easing(EasingFunction::SineInOut);
        tween.play();
        
        Self { position: tween }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let dt = ctx.input(|i| i.stable_dt);
        self.position.update(dt);
        
        let x = self.position.value();
        
        CentralPanel::default().show(ctx, |ui| {
            let painter = ui.painter();
            painter.circle_filled(
                Pos2::new(x, 100.0),
                10.0,
                Color32::BLUE
            );
        });
        
        ctx.request_repaint();
    }
}
```

---

### Spring

**Purpose**: Physics-based animation with natural motion.

**Type Signature**:
```rust
pub struct Spring {
    position: f64,
    velocity: f64,
    target: f64,
    params: SpringParams,
}

pub struct SpringParams {
    pub stiffness: f64,
    pub damping: f64,
}
```

**Methods**:

#### `new(initial: f64) -> Self`
Create a spring with default smooth parameters.

```rust
let spring = Spring::new(0.5);
```

#### `with_params(initial: f64, params: SpringParams) -> Self`
Create a spring with custom parameters.

```rust
let spring = Spring::with_params(0.0, SpringParams::bouncy());
```

#### `set_target(&mut self, target: f64)`
Set target position.

```rust
spring.set_target(1.0);
```

#### `update(&mut self, dt: f32)`
Update physics simulation.

```rust
spring.update(dt);
```

#### `position(&self) -> f64`
Get current position.

```rust
let pos = spring.position();
```

#### `velocity(&self) -> f64`
Get current velocity.

```rust
let vel = spring.velocity();
```

**SpringParams Presets**:

```rust
// Smooth (no overshoot)
SpringParams::smooth()  // stiffness: 100.0, damping: 1.0

// Bouncy (overshoot and oscillate)
SpringParams::bouncy()  // stiffness: 300.0, damping: 0.5

// Stiff (fast response)
SpringParams::stiff()   // stiffness: 500.0, damping: 1.5

// Custom
SpringParams {
    stiffness: 200.0,  // Higher = faster
    damping: 0.8,      // 0 = forever, 1 = critical, >1 = slow
}
```

**Example**:
```rust
struct App {
    spring: Spring,
    target: f32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            spring: Spring::with_params(0.5, SpringParams::smooth()),
            target: 0.5,
        }
    }
}

impl App {
    fn show(&mut self, ui: &mut Ui, dt: f32) {
        ui.horizontal(|ui| {
            ui.label("Target:");
            if ui.add(Slider::new(&mut self.target, 0.0..=1.0)).changed() {
                self.spring.set_target(self.target as f64);
            }
        });
        
        self.spring.update(dt);
        let pos = self.spring.position();
        
        ui.label(format!("Position: {:.3}", pos));
    }
}
```

---

### EasingFunction

**Purpose**: Mathematical curves for smooth interpolation.

**Type Signature**:
```rust
pub enum EasingFunction {
    Linear,
    SineIn,
    SineOut,
    SineInOut,
    QuadIn,
    QuadOut,
    QuadInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    ElasticIn,
    ElasticOut,
    BounceOut,
    BackIn,
    BackOut,
}
```

**Usage Guide**:

| Function | Use Case | Characteristics |
|----------|----------|-----------------|
| `Linear` | Loading bars, spinners | Constant speed |
| `SineIn` | Fade in | Slow start |
| `SineOut` | Fade out | Slow end |
| `SineInOut` | UI panels, modals | Smooth both ends |
| `QuadIn` | Accelerating objects | x² acceleration |
| `QuadOut` | Buttons, clicks | x² deceleration |
| `QuadInOut` | General UI motion | Balanced |
| `CubicIn` | Dramatic entries | Strong acceleration |
| `CubicOut` | Dramatic exits | Strong deceleration |
| `CubicInOut` | Emphasis | Very smooth |
| `ElasticIn` | Pull-back effect | Spring wind-up |
| `ElasticOut` | Bouncy UI | Overshoot + settle |
| `BounceOut` | Falling objects | Bounce on landing |
| `BackIn` | Anticipation | Pull back first |
| `BackOut` | Overshoot | Go past then settle |

---

### AnimationController

**Purpose**: Manage multiple animations simultaneously.

**Type Signature**:
```rust
pub struct AnimationController {
    animations: HashMap<String, Box<dyn Animation>>,
}
```

**Methods**:

#### `new() -> Self`
Create a new controller.

```rust
let controller = AnimationController::new();
```

#### `add_animation<T: Animation + 'static>(&mut self, id: impl Into<String>, animation: T)`
Add an animation with string ID.

```rust
controller.add_animation(
    "position",
    Tween::new(0.0, 100.0, 2.0)
);
```

#### `play(&mut self, id: &str)`
Start a specific animation.

```rust
controller.play("position");
```

#### `play_all(&mut self)`
Start all animations.

```rust
controller.play_all();
```

#### `pause(&mut self, id: &str)`
Pause a specific animation.

```rust
controller.pause("position");
```

#### `update_all(&mut self, dt: f32)`
Update all animations.

```rust
controller.update_all(dt);
```

#### `get_value<T: 'static>(&self, id: &str) -> Option<T>`
Get current value of an animation.

```rust
let pos = controller.get_value::<f32>("position").unwrap();
```

#### `is_finished(&self, id: &str) -> bool`
Check if animation completed.

```rust
if controller.is_finished("step1") {
    controller.play("step2");
}
```

**Example**:
```rust
struct App {
    controller: AnimationController,
}

impl App {
    fn new() -> Self {
        let mut controller = AnimationController::new();
        
        controller.add_animation(
            "x",
            Tween::new(0.0, 200.0, 2.0)
                .with_easing(EasingFunction::SineInOut)
        );
        
        controller.add_animation(
            "y",
            Tween::new(0.0, 100.0, 1.5)
                .with_easing(EasingFunction::ElasticOut)
        );
        
        controller.play_all();
        
        Self { controller }
    }
    
    fn update(&mut self, dt: f32) {
        self.controller.update_all(dt);
        
        let x = self.controller.get_value::<f32>("x").unwrap();
        let y = self.controller.get_value::<f32>("y").unwrap();
        
        // Use x, y for rendering
    }
}
```

---

## Type Reference

### AnimationState

```rust
pub enum AnimationState {
    Playing,
    Paused,
    Stopped,
}
```

**Usage**:
```rust
match tween.state() {
    AnimationState::Playing => ui.label("▶ Playing"),
    AnimationState::Paused => ui.label("⏸ Paused"),
    AnimationState::Stopped => ui.label("⏹ Stopped"),
}
```

---

### Linear Trait

**Purpose**: Types that can be interpolated linearly.

**Implemented For**:
- `f32`, `f64`
- `i32`, `u32`, `i64`, `u64`
- `Color32` (RGBA color blending)
- `Pos2`, `Vec2` (2D vectors)
- Custom types (implement trait)

**Example**:
```rust
// These all work with Tween<T>
let float_tween = Tween::new(0.0f32, 100.0f32, 2.0);
let color_tween = Tween::new(Color32::RED, Color32::BLUE, 3.0);
let pos_tween = Tween::new(Pos2::ZERO, Pos2::new(100.0, 100.0), 1.5);
```

---

## Quick Reference Table

### Charts

| Type | Method | Purpose |
|------|--------|---------|
| `LineChart::new(title)` | Constructor | Create line chart |
| `.add_series(name, points, color)` | Add data | Add line series |
| `.show(ui)` | Render | Display chart |
| `BarChart::new(title)` | Constructor | Create bar chart |
| `.add_group(group)` | Add data | Add bar group |
| `ScatterPlot::new(title)` | Constructor | Create scatter plot |
| `.add_cluster(cluster)` | Add data | Add point cluster |

### Graphs

| Type | Method | Purpose |
|------|--------|---------|
| `NodeGraph::new()` | Constructor | Create graph |
| `.add_node(node)` | Add node | Returns node ID |
| `.add_edge(src, sp, tgt, tp)` | Connect | Link nodes |
| `.show(ui)` | Render | Returns clicked ID |
| `GraphNode::new(id, label)` | Constructor | Create node |
| `.add_input(port)` | Add port | Input port |
| `.add_output(port)` | Add port | Output port |
| `.with_position(x, y)` | Position | Set coordinates |

### Advanced Widgets

| Type | Method | Purpose |
|------|--------|---------|
| `ColorPicker::new()` | Constructor | Create picker |
| `.with_color(color)` | Configure | Set initial color |
| `.show_alpha(bool)` | Configure | Enable alpha |
| `.show(ui)` | Render | Display picker |
| `.color()` | Getter | Get selected color |
| `TreeView::new()` | Constructor | Create tree |
| `.add_node(node)` | Add node | Root level |
| `.add_child(parent, node)` | Add node | Child level |
| `.show(ui)` | Render | Returns clicked ID |
| `RangeSlider::new(min, max)` | Constructor | Create slider |
| `.with_min(val)` | Configure | Set min value |
| `.with_max(val)` | Configure | Set max value |
| `.show(ui)` | Render | Display slider |
| `.min_value()` | Getter | Get min |
| `.max_value()` | Getter | Get max |

### Animations

| Type | Method | Purpose |
|------|--------|---------|
| `Tween::new(start, end, dur)` | Constructor | Create tween |
| `.with_easing(func)` | Configure | Set easing |
| `.play()` | Control | Start animation |
| `.pause()` | Control | Pause animation |
| `.update(dt)` | Update | Advance time |
| `.value()` | Getter | Get current value |
| `.is_finished()` | Check | Test completion |
| `Spring::new(initial)` | Constructor | Create spring |
| `.with_params(initial, params)` | Constructor | Custom params |
| `.set_target(target)` | Control | Set target |
| `.update(dt)` | Update | Physics step |
| `.position()` | Getter | Get position |
| `.velocity()` | Getter | Get velocity |
| `AnimationController::new()` | Constructor | Create controller |
| `.add_animation(id, anim)` | Add | Register animation |
| `.play(id)` | Control | Start one |
| `.play_all()` | Control | Start all |
| `.update_all(dt)` | Update | Update all |
| `.get_value<T>(id)` | Getter | Get value |

---

## Integration Patterns

### Pattern 1: Chart Dashboard

```rust
struct Dashboard {
    fps_chart: LineChart,
    memory_chart: LineChart,
    cpu_bars: BarChart,
}

impl Dashboard {
    fn update(&mut self, fps: f64, memory: f64, cpu: &[f64]) {
        // Update chart data
    }
    
    fn show(&self, ui: &mut Ui) {
        ui.columns(2, |cols| {
            self.fps_chart.show(&mut cols[0]);
            self.memory_chart.show(&mut cols[1]);
        });
        self.cpu_bars.show(ui);
    }
}
```

### Pattern 2: Animated UI

```rust
struct AnimatedPanel {
    controller: AnimationController,
    is_open: bool,
}

impl AnimatedPanel {
    fn toggle(&mut self) {
        self.is_open = !self.is_open;
        let target = if self.is_open { 1.0 } else { 0.0 };
        
        // Update all animations to new target
        if let Some(tween) = self.controller.get_mut("slide") {
            tween.set_target(target);
        }
    }
    
    fn show(&mut self, ui: &mut Ui, dt: f32) {
        self.controller.update_all(dt);
        
        let slide = self.controller.get_value::<f32>("slide").unwrap_or(0.0);
        
        // Apply animated offset
        ui.set_clip_rect(/* ... */);
    }
}
```

### Pattern 3: Node-Based AI

```rust
struct AIEditor {
    graph: NodeGraph,
    selected_node: Option<usize>,
}

impl AIEditor {
    fn show(&mut self, ui: &mut Ui) {
        // Toolbar
        ui.horizontal(|ui| {
            if ui.button("Add Selector").clicked() {
                let node = GraphNode::new(self.next_id(), "Selector")
                    .with_position(100.0, 100.0);
                self.graph.add_node(node);
            }
        });
        
        // Graph view
        if let Some(clicked) = self.graph.show(ui) {
            self.selected_node = Some(clicked);
        }
        
        // Properties panel
        if let Some(id) = self.selected_node {
            self.show_properties(ui, id);
        }
    }
}
```

---

## See Also

- [Getting Started](./GETTING_STARTED.md) - Installation and first app
- [Charts Tutorial](./CHARTS_TUTORIAL.md) - Detailed chart examples
- [Advanced Widgets Tutorial](./ADVANCED_WIDGETS_TUTORIAL.md) - Widget patterns
- [NodeGraph Tutorial](./NODEGRAPH_TUTORIAL.md) - Visual editors
- [Animation Tutorial](./ANIMATION_TUTORIAL.md) - Animation techniques
- [Gallery Example](../../examples/astract_gallery/) - All features demonstrated

---

**Complete API coverage for productive development! 📚**
