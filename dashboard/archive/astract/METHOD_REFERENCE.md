# Astract Method Reference

Alphabetical quick reference for all public methods.

---

## A

### `add_animation(id: &str, animation: Box<dyn Animation>)` - AnimationController
Add named animation to controller.

**Returns**: `&mut Self` (builder pattern)

**Example**:
```rust
ctrl.add_animation("x", Box::new(tween));
```

---

### `add_child(parent_id: usize, node: TreeNode) -> usize` - TreeView
Add child node to parent in tree.

**Returns**: Child node ID

**Example**:
```rust
let child_id = tree.add_child(parent, TreeNode::new(2, "child"));
```

---

### `add_cluster(cluster: PointCluster)` - ScatterPlot
Add point cluster to scatter plot.

**Returns**: `&mut Self` (builder pattern)

**Example**:
```rust
scatter.add_cluster(PointCluster::new("A", points, Color32::RED));
```

---

### `add_edge(source_id: usize, source_port: usize, target_id: usize, target_port: usize)` - NodeGraph
Connect two nodes by ports.

**Returns**: `&mut Self` (builder pattern)

**Example**:
```rust
graph.add_edge(1, 0, 2, 0);
```

---

### `add_group(group: BarGroup)` - BarChart
Add bar group to chart.

**Returns**: `&mut Self` (builder pattern)

**Example**:
```rust
chart.add_group(BarGroup { category: "Q1", bars: vec![...] });
```

---

### `add_input(port: Port)` - GraphNode
Add input port to node.

**Returns**: `&mut Self` (builder pattern)

**Example**:
```rust
node.add_input(Port::new(0, "In", PortType::Exec));
```

---

### `add_node(node: GraphNode) -> usize` - NodeGraph
Add node to graph.

**Returns**: Node ID

**Example**:
```rust
let id = graph.add_node(GraphNode::new(1, "Start"));
```

---

### `add_node(node: TreeNode) -> usize` - TreeView
Add root node to tree.

**Returns**: Node ID

**Example**:
```rust
let id = tree.add_node(TreeNode::new(1, "Root"));
```

---

### `add_output(port: Port)` - GraphNode
Add output port to node.

**Returns**: `&mut Self` (builder pattern)

**Example**:
```rust
node.add_output(Port::new(0, "Out", PortType::Number));
```

---

### `add_series(name: String, points: Vec<Point>, color: Color32)` - LineChart
Add line series to chart.

**Returns**: `&mut Self` (builder pattern)

**Example**:
```rust
chart.add_series("FPS", vec![(0.0, 60.0), (1.0, 58.0)], Color32::GREEN);
```

---

## C

### `color() -> Color32` - ColorPicker
Get current color.

**Returns**: Current RGBA color

**Example**:
```rust
let c = picker.color();
```

---

## G

### `get_value<T: Animation>(&self, id: &str) -> Option<T::Output>` - AnimationController
Get animation value by ID.

**Returns**: Animation value if exists and playing

**Example**:
```rust
let x = ctrl.get_value::<Tween<f32>>("x")?;
```

---

## I

### `is_finished(&self, id: &str) -> bool` - AnimationController
Check if animation is finished.

**Returns**: True if animation complete

**Example**:
```rust
if ctrl.is_finished("x") { /* restart */ }
```

---

### `is_finished(&self) -> bool` - Tween
Check if tween finished.

**Returns**: True if complete

**Example**:
```rust
if tween.is_finished() { tween.reset(); }
```

---

## M

### `max_value() -> f64` - RangeSlider
Get max handle value.

**Returns**: Current max value

**Example**:
```rust
let max = range.max_value();
```

---

### `min_value() -> f64` - RangeSlider
Get min handle value.

**Returns**: Current min value

**Example**:
```rust
let min = range.min_value();
```

---

## N

### `new(id: usize, label: String) -> Self` - GraphNode
Create graph node.

**Returns**: New GraphNode

**Example**:
```rust
let node = GraphNode::new(1, "Start");
```

---

### `new(id: usize, label: String) -> Self` - TreeNode
Create tree node.

**Returns**: New TreeNode

**Example**:
```rust
let node = TreeNode::new(1, "Root");
```

---

### `new(index: usize, label: String, port_type: PortType) -> Self` - Port
Create port.

**Returns**: New Port

**Example**:
```rust
let port = Port::new(0, "In", PortType::Exec);
```

---

### `new(name: String, points: Vec<Point>, color: Color32) -> Self` - PointCluster
Create point cluster.

**Returns**: New PointCluster

**Example**:
```rust
let cluster = PointCluster::new("A", points, Color32::RED);
```

---

### `new(range_min: f64, range_max: f64) -> Self` - RangeSlider
Create range slider.

**Returns**: New RangeSlider

**Example**:
```rust
let slider = RangeSlider::new(0.0, 100.0);
```

---

### `new(start: T, end: T, duration: f32) -> Self` - Tween
Create tween animation.

**Returns**: New Tween<T>

**Example**:
```rust
let tween = Tween::new(0.0, 100.0, 2.0);
```

---

### `new(title: String) -> Self` - BarChart
Create bar chart.

**Returns**: New BarChart

**Example**:
```rust
let chart = BarChart::new("Sales");
```

---

### `new(title: String) -> Self` - LineChart
Create line chart.

**Returns**: New LineChart

**Example**:
```rust
let chart = LineChart::new("FPS");
```

---

### `new(title: String) -> Self` - ScatterPlot
Create scatter plot.

**Returns**: New ScatterPlot

**Example**:
```rust
let scatter = ScatterPlot::new("Clusters");
```

---

### `new() -> Self` - AnimationController
Create animation controller.

**Returns**: New AnimationController

**Example**:
```rust
let ctrl = AnimationController::new();
```

---

### `new() -> Self` - ColorPicker
Create color picker.

**Returns**: New ColorPicker with black

**Example**:
```rust
let picker = ColorPicker::new();
```

---

### `new() -> Self` - NodeGraph
Create node graph.

**Returns**: New NodeGraph

**Example**:
```rust
let graph = NodeGraph::new();
```

---

### `new() -> Self` - TreeView
Create tree view.

**Returns**: New TreeView

**Example**:
```rust
let tree = TreeView::new();
```

---

## P

### `pause()` - Tween
Pause animation.

**Returns**: `&mut Self`

**Example**:
```rust
tween.pause();
```

---

### `play(&mut self, id: &str)` - AnimationController
Play animation by ID.

**Example**:
```rust
ctrl.play("x");
```

---

### `play()` - Tween
Start/resume animation.

**Returns**: `&mut Self`

**Example**:
```rust
tween.play();
```

---

### `play_all(&mut self)` - AnimationController
Play all animations.

**Example**:
```rust
ctrl.play_all();
```

---

### `position() -> f64` - Spring
Get current position.

**Returns**: Position value

**Example**:
```rust
let pos = spring.position();
```

---

## R

### `reset()` - Tween
Reset to start, stop playing.

**Returns**: `&mut Self`

**Example**:
```rust
tween.reset();
```

---

## S

### `set_target(&mut self, target: f64)` - Spring
Set target position.

**Example**:
```rust
spring.set_target(100.0);
```

---

### `show(&mut self, ui: &mut Ui)` - BarChart
Render bar chart.

**Example**:
```rust
chart.show(ui);
```

---

### `show(&mut self, ui: &mut Ui)` - ColorPicker
Render color picker.

**Example**:
```rust
picker.show(ui);
```

---

### `show(&mut self, ui: &mut Ui)` - LineChart
Render line chart.

**Example**:
```rust
chart.show(ui);
```

---

### `show(&mut self, ui: &mut Ui)` - RangeSlider
Render range slider.

**Example**:
```rust
slider.show(ui);
```

---

### `show(&mut self, ui: &mut Ui)` - ScatterPlot
Render scatter plot.

**Example**:
```rust
scatter.show(ui);
```

---

### `show(&mut self, ui: &mut Ui) -> Option<usize>` - NodeGraph
Render node graph, return clicked node ID.

**Returns**: Some(id) if node clicked

**Example**:
```rust
if let Some(id) = graph.show(ui) { /* ... */ }
```

---

### `show(&mut self, ui: &mut Ui) -> Option<usize>` - TreeView
Render tree view, return clicked node ID.

**Returns**: Some(id) if node clicked

**Example**:
```rust
if let Some(id) = tree.show(ui) { /* ... */ }
```

---

### `show_alpha(show: bool)` - ColorPicker
Toggle alpha slider.

**Returns**: `&mut Self` (builder)

**Example**:
```rust
picker.show_alpha(true);
```

---

### `smooth() -> SpringParams` - SpringParams
Create smooth spring preset.

**Returns**: SpringParams (stiffness: 100, damping: 1.0)

**Example**:
```rust
let spring = Spring::with_params(0.0, SpringParams::smooth());
```

---

### `state(&self) -> AnimationState` - Tween
Get animation state.

**Returns**: Playing, Paused, or Stopped

**Example**:
```rust
match tween.state() {
    AnimationState::Playing => { /* ... */ },
    _ => { /* ... */ },
}
```

---

### `stiff() -> SpringParams` - SpringParams
Create stiff spring preset.

**Returns**: SpringParams (stiffness: 500, damping: 1.5)

**Example**:
```rust
let spring = Spring::with_params(0.0, SpringParams::stiff());
```

---

## U

### `update(&mut self, dt: f32)` - Spring
Update spring physics.

**Example**:
```rust
spring.update(0.016); // 60 FPS
```

---

### `update(&mut self, dt: f32)` - Tween
Update tween progress.

**Example**:
```rust
tween.update(0.016); // 60 FPS
```

---

### `update_all(&mut self, dt: f32)` - AnimationController
Update all animations.

**Example**:
```rust
ctrl.update_all(0.016); // 60 FPS
```

---

## V

### `value(&self) -> T` - Tween
Get current interpolated value.

**Returns**: Interpolated value of type T

**Example**:
```rust
let x = tween.value();
```

---

### `velocity() -> f64` - Spring
Get current velocity.

**Returns**: Velocity value

**Example**:
```rust
let vel = spring.velocity();
```

---

## W

### `with_color(color: Color32)` - ColorPicker
Set initial color.

**Returns**: `&mut Self` (builder)

**Example**:
```rust
let picker = ColorPicker::new().with_color(Color32::RED);
```

---

### `with_easing(easing: EasingFunction)` - Tween
Set easing function.

**Returns**: `&mut Self` (builder)

**Example**:
```rust
let tween = Tween::new(0.0, 100.0, 2.0)
    .with_easing(EasingFunction::ElasticOut);
```

---

### `with_icon(icon: &str)` - TreeNode
Set node icon (emoji).

**Returns**: Self (builder)

**Example**:
```rust
let node = TreeNode::new(1, "File").with_icon("📄");
```

---

### `with_max(max: f64)` - RangeSlider
Set initial max value.

**Returns**: `&mut Self` (builder)

**Example**:
```rust
let slider = RangeSlider::new(0.0, 100.0).with_max(75.0);
```

---

### `with_min(min: f64)` - RangeSlider
Set initial min value.

**Returns**: `&mut Self` (builder)

**Example**:
```rust
let slider = RangeSlider::new(0.0, 100.0).with_min(25.0);
```

---

### `with_params(initial: f64, params: SpringParams) -> Self` - Spring
Create spring with custom parameters.

**Returns**: New Spring

**Example**:
```rust
let spring = Spring::with_params(0.0, SpringParams::smooth());
```

---

### `with_position(x: f64, y: f64)` - GraphNode
Set node position.

**Returns**: Self (builder)

**Example**:
```rust
let node = GraphNode::new(1, "Start").with_position(50.0, 100.0);
```

---

## Method Index by Type

### AnimationController
- `new()` - Create controller
- `add_animation(id, animation)` - Add named animation
- `play(id)` - Play by ID
- `play_all()` - Play all
- `update_all(dt)` - Update all
- `get_value<T>(id)` - Get value
- `is_finished(id)` - Check if done

### BarChart
- `new(title)` - Create chart
- `add_group(group)` - Add bar group
- `show(ui)` - Render

### ColorPicker
- `new()` - Create picker
- `with_color(color)` - Set color
- `show_alpha(show)` - Toggle alpha
- `show(ui)` - Render
- `color()` - Get color

### GraphNode
- `new(id, label)` - Create node
- `add_input(port)` - Add input
- `add_output(port)` - Add output
- `with_position(x, y)` - Set position

### LineChart
- `new(title)` - Create chart
- `add_series(name, points, color)` - Add line
- `show(ui)` - Render

### NodeGraph
- `new()` - Create graph
- `add_node(node)` - Add node
- `add_edge(src, tgt)` - Connect nodes
- `show(ui)` - Render (returns clicked ID)

### Port
- `new(index, label, type)` - Create port

### PointCluster
- `new(name, points, color)` - Create cluster

### RangeSlider
- `new(min, max)` - Create slider
- `with_min(min)` - Set initial min
- `with_max(max)` - Set initial max
- `show(ui)` - Render
- `min_value()` - Get min
- `max_value()` - Get max

### ScatterPlot
- `new(title)` - Create plot
- `add_cluster(cluster)` - Add cluster
- `show(ui)` - Render

### Spring
- `with_params(initial, params)` - Create spring
- `set_target(target)` - Set target
- `update(dt)` - Update physics
- `position()` - Get position
- `velocity()` - Get velocity

### SpringParams
- `smooth()` - Smooth preset
- `bouncy()` - Bouncy preset
- `stiff()` - Stiff preset

### TreeNode
- `new(id, label)` - Create node
- `with_icon(icon)` - Set icon

### TreeView
- `new()` - Create tree
- `add_node(node)` - Add root
- `add_child(parent, node)` - Add child
- `show(ui)` - Render (returns clicked ID)

### Tween<T>
- `new(start, end, duration)` - Create tween
- `with_easing(func)` - Set easing
- `play()` - Start/resume
- `pause()` - Pause
- `reset()` - Reset
- `update(dt)` - Update
- `value()` - Get value
- `is_finished()` - Check done
- `state()` - Get state

---

## Common Patterns

### Builder Pattern
```rust
ColorPicker::new()
    .with_color(Color32::RED)
    .show_alpha(true);

RangeSlider::new(0.0, 100.0)
    .with_min(25.0)
    .with_max(75.0);

Tween::new(0.0, 100.0, 2.0)
    .with_easing(EasingFunction::ElasticOut);
```

### Stateful Widgets (show + getters)
```rust
// Render
picker.show(ui);
range.show(ui);

// Get values
let color = picker.color();
let min = range.min_value();
let max = range.max_value();
```

### Interactive Widgets (show returns Option<usize>)
```rust
if let Some(id) = graph.show(ui) {
    println!("Node {} clicked", id);
}

if let Some(id) = tree.show(ui) {
    load_file(id);
}
```

### Animation Loop
```rust
// Every frame
tween.update(dt);
spring.update(dt);
ctrl.update_all(dt);

// Get values
let x = tween.value();
let pos = spring.position();
let y = ctrl.get_value::<Tween<f32>>("y")?;
```

---

## See Also

- [API Reference](./API_REFERENCE.md) - Detailed method docs
- [Widget Catalog](./WIDGET_CATALOG.md) - Visual guide
- [Tutorials](./GETTING_STARTED.md#next-steps) - Step-by-step guides

---

**Quick alphabetical lookup for all Astract methods!**
