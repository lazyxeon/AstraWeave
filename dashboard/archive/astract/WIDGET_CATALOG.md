# Astract Widget Catalog

Visual guide to all Astract widgets with use cases and examples.

---

## Overview

Astract provides **10 production-ready widgets** across 4 categories:

1. **Charts** (3 widgets) - Data visualization
2. **Graphs** (1 widget) - Visual programming
3. **Advanced Widgets** (3 widgets) - Interactive controls
4. **Animation** (3 systems) - Motion and transitions

---

## Charts Widgets

### 📈 LineChart

**Visual Description**: Multiple colored lines on X/Y axes with legend, grid, tooltips.

**Best For**:
- Time-series data (FPS over time, stock prices)
- Performance monitoring (CPU, GPU, memory usage)
- Continuous metrics (temperature, speed, health)
- Trend analysis (sales trends, user growth)

**When to Use**:
- ✅ Data points connected by lines
- ✅ Showing trends over time
- ✅ Comparing multiple series
- ✅ Continuous data

**When NOT to Use**:
- ❌ Categorical data (use BarChart)
- ❌ Discrete points (use ScatterPlot)
- ❌ Part-of-whole (use pie chart)

**Code Signature**:
```rust
let mut chart = LineChart::new("Title");
chart.add_series("Series 1", vec![(x, y), ...], Color32::BLUE);
chart.show(ui);
```

**Real-World Example**:
```rust
// Game performance monitor
let mut fps_chart = LineChart::new("FPS (Last 60 seconds)");
fps_chart.add_series("FPS", fps_history, Color32::GREEN);
fps_chart.add_series("Target", target_line, Color32::from_gray(128));
fps_chart.show(ui);
```

**Strengths**:
- Clear trend visualization
- Multiple series comparison
- Tooltip shows exact values
- Auto-scales to data

**Limitations**:
- Cluttered with >5 series
- Poor for categorical data
- Requires sorted X values for best results

---

### 📊 BarChart

**Visual Description**: Grouped vertical bars with category labels, legend, grid.

**Best For**:
- Categorical comparisons (sales by region, scores by team)
- Grouped data (quarterly revenue by product)
- Discrete measurements (bug counts by severity)
- Before/after comparisons

**When to Use**:
- ✅ Comparing categories
- ✅ Discrete data points
- ✅ Grouped comparisons
- ✅ Part-of-whole within groups

**When NOT to Use**:
- ❌ Continuous time series (use LineChart)
- ❌ Correlation analysis (use ScatterPlot)
- ❌ >8 groups (cluttered)

**Code Signature**:
```rust
let mut chart = BarChart::new("Title");
chart.add_group(BarGroup {
    category: "Q1",
    bars: vec![Bar { label: "A", value: 100.0, color: Color32::RED }],
});
chart.show(ui);
```

**Real-World Example**:
```rust
// Sales analytics
let mut chart = BarChart::new("Regional Sales (2024)");
for quarter in ["Q1", "Q2", "Q3", "Q4"] {
    chart.add_group(BarGroup {
        category: quarter.into(),
        bars: vec![
            Bar { label: "North".into(), value: north_sales[quarter], color: RED },
            Bar { label: "South".into(), value: south_sales[quarter], color: BLUE },
            Bar { label: "East".into(), value: east_sales[quarter], color: GREEN },
            Bar { label: "West".into(), value: west_sales[quarter], color: YELLOW },
        ],
    });
}
chart.show(ui);
```

**Strengths**:
- Clear category comparison
- Easy to read values
- Good for grouped data
- Color-coded for clarity

**Limitations**:
- Limited to ~8 groups max
- Takes more space than lines
- Not good for trends

---

### 🔵 ScatterPlot

**Visual Description**: Colored point clusters on X/Y axes with legend, grid, tooltips.

**Best For**:
- Correlation analysis (height vs weight, price vs sales)
- Cluster visualization (customer segments, data groups)
- Outlier detection (anomaly detection)
- Distribution patterns (random data, normal distribution)

**When to Use**:
- ✅ Correlation between variables
- ✅ Cluster identification
- ✅ Outlier detection
- ✅ Distribution analysis

**When NOT to Use**:
- ❌ Connected data points (use LineChart)
- ❌ Categorical comparison (use BarChart)
- ❌ Time series (use LineChart)

**Code Signature**:
```rust
let mut scatter = ScatterPlot::new("Title");
scatter.add_cluster(PointCluster::new("Cluster 1", vec![(x, y), ...], Color32::BLUE));
scatter.show(ui);
```

**Real-World Example**:
```rust
// Machine learning cluster visualization
let mut scatter = ScatterPlot::new("Feature Space");
scatter.add_cluster(PointCluster::new("Class A", class_a_points, Color32::RED));
scatter.add_cluster(PointCluster::new("Class B", class_b_points, Color32::BLUE));
scatter.add_cluster(PointCluster::new("Class C", class_c_points, Color32::GREEN));
scatter.show(ui);
```

**Strengths**:
- Clear cluster visualization
- Easy correlation spotting
- Outlier identification
- Multiple groups support

**Limitations**:
- No connection between points
- Cluttered with >1000 points
- Tooltip only on hover

---

## Graph Widget

### 🔗 NodeGraph

**Visual Description**: Visual node editor with boxes, colored ports, connection lines.

**Best For**:
- Behavior trees (AI decision-making)
- Shader graphs (material editors)
- Dialogue systems (branching conversations)
- State machines (game states, workflows)
- Data flow (processing pipelines)

**When to Use**:
- ✅ Visual programming needed
- ✅ Non-programmers editing logic
- ✅ Complex decision trees
- ✅ Data flow visualization

**When NOT to Use**:
- ❌ Simple linear logic (use code)
- ❌ Text-based editing preferred
- ❌ Very large graphs (>100 nodes)

**Code Signature**:
```rust
let mut graph = NodeGraph::new();
let node1_id = graph.add_node(GraphNode::new(1, "Start"));
let node2_id = graph.add_node(GraphNode::new(2, "Action"));
graph.add_edge(node1_id, 0, node2_id, 0);
if let Some(clicked) = graph.show(ui) { /* ... */ }
```

**Real-World Example**:
```rust
// AI behavior tree
let mut graph = NodeGraph::new();

// Root sequence
let mut root = GraphNode::new(1, "Root Sequence");
root.add_output(Port::new(0, "1", PortType::Exec));
root.add_output(Port::new(1, "2", PortType::Exec));
let root_id = graph.add_node(root.with_position(50.0, 100.0));

// Check health condition
let mut check = GraphNode::new(2, "Health > 50%");
check.add_input(Port::new(0, "In", PortType::Exec));
check.add_output(Port::new(0, "Pass", PortType::Bool));
let check_id = graph.add_node(check.with_position(250.0, 100.0));

// Attack action
let mut attack = GraphNode::new(3, "Attack");
attack.add_input(Port::new(0, "Do", PortType::Exec));
let attack_id = graph.add_node(attack.with_position(450.0, 100.0));

graph.add_edge(root_id, 0, check_id, 0);
graph.add_edge(root_id, 1, attack_id, 0);
```

**Port Types**:
- **Exec** (white) - Execution flow, control order
- **Bool** (red) - True/false conditions
- **Number** (green) - Numeric data (damage, health, etc.)
- **String** (blue) - Text data (names, messages)
- **Object** (yellow) - Complex data (entities, vectors)

**Strengths**:
- Visual, intuitive editing
- Clear data flow
- Non-programmer friendly
- Easy to understand logic

**Limitations**:
- Takes screen space
- Performance with >50 nodes
- Requires layout management

---

## Advanced Widgets

### 🎨 ColorPicker

**Visual Description**: RGB/HSV color selector with alpha slider, preview square.

**Best For**:
- Theme editors (background, foreground, accent colors)
- Material editors (diffuse, specular, emission)
- Drawing apps (brush color, fill color)
- UI customization (user preferences)

**When to Use**:
- ✅ User selects colors
- ✅ Theme customization
- ✅ Visual design tools
- ✅ Color preview needed

**When NOT to Use**:
- ❌ Predefined color set (use buttons)
- ❌ Simple on/off (use checkbox)
- ❌ Limited space (use dropdown)

**Code Signature**:
```rust
// In app state
picker: ColorPicker::new().with_color(Color32::RED).show_alpha(true)

// In update
picker.show(ui);
let color = picker.color();
```

**Real-World Example**:
```rust
struct ThemeEditor {
    bg_picker: ColorPicker,
    fg_picker: ColorPicker,
    accent_picker: ColorPicker,
}

impl ThemeEditor {
    fn show(&mut self, ui: &mut Ui) {
        ui.label("Background:");
        self.bg_picker.show(ui);
        
        ui.label("Foreground:");
        self.fg_picker.show(ui);
        
        ui.label("Accent:");
        self.accent_picker.show(ui);
        
        // Preview
        Frame::none()
            .fill(self.bg_picker.color())
            .show(ui, |ui| {
                ui.visuals_mut().override_text_color = Some(self.fg_picker.color());
                ui.label("Preview text");
            });
    }
}
```

**Strengths**:
- Full RGBA control
- Real-time preview
- Familiar HSV interface
- Precise value input

**Limitations**:
- Takes vertical space (~200px)
- Overwhelming for beginners
- Alpha channel optional

---

### 📁 TreeView

**Visual Description**: Hierarchical file/folder structure with expand/collapse arrows, icons.

**Best For**:
- File browsers (project files, asset explorer)
- Scene hierarchies (game objects, UI elements)
- Menu systems (nested categories)
- Organizational charts (company structure)

**When to Use**:
- ✅ Hierarchical data
- ✅ Parent-child relationships
- ✅ Expandable sections
- ✅ Navigation trees

**When NOT to Use**:
- ❌ Flat lists (use ListView)
- ❌ >500 nodes (performance)
- ❌ No hierarchy (use list)

**Code Signature**:
```rust
let mut tree = TreeView::new();
let root = tree.add_node(TreeNode::new(1, "Root").with_icon("📁"));
tree.add_child(root, TreeNode::new(2, "Child").with_icon("📄"));
if let Some(clicked) = tree.show(ui) { /* ... */ }
```

**Real-World Example**:
```rust
// File system browser
let mut tree = TreeView::new();

let project = tree.add_node(TreeNode::new(1, "my_project").with_icon("📁"));

let src = tree.add_child(project, TreeNode::new(2, "src").with_icon("📁"));
tree.add_child(src, TreeNode::new(3, "main.rs").with_icon("🦀"));
tree.add_child(src, TreeNode::new(4, "lib.rs").with_icon("🦀"));

let ui_folder = tree.add_child(src, TreeNode::new(5, "ui").with_icon("📁"));
tree.add_child(ui_folder, TreeNode::new(6, "mod.rs").with_icon("🦀"));
tree.add_child(ui_folder, TreeNode::new(7, "widgets.rs").with_icon("🦀"));

tree.add_child(project, TreeNode::new(8, "Cargo.toml").with_icon("⚙️"));
tree.add_child(project, TreeNode::new(9, "README.md").with_icon("📖"));

if let Some(clicked_id) = tree.show(ui) {
    load_file(clicked_id);
}
```

**Strengths**:
- Clear hierarchy
- Familiar interface
- Icons for visual clarity
- Click handling built-in

**Limitations**:
- Vertical space hungry
- Performance with deep trees
- No drag-and-drop (custom needed)

---

### 🎚️ RangeSlider

**Visual Description**: Dual-handle slider with min/max values, visual range highlight.

**Best For**:
- Price filters (min-max price range)
- Date ranges (start-end date selection)
- Value filters (age range, score range)
- Parameter bounds (simulation min/max)

**When to Use**:
- ✅ Range selection needed
- ✅ Min AND max values
- ✅ Continuous ranges
- ✅ Visual feedback important

**When NOT to Use**:
- ❌ Single value (use Slider)
- ❌ Discrete steps (use dropdown)
- ❌ Boolean choice (use checkbox)

**Code Signature**:
```rust
// In app state
range: RangeSlider::new(0.0, 100.0).with_min(25.0).with_max(75.0)

// In update
range.show(ui);
let min = range.min_value();
let max = range.max_value();
```

**Real-World Example**:
```rust
// E-commerce price filter
struct ProductFilter {
    price_range: RangeSlider,
}

impl ProductFilter {
    fn new() -> Self {
        Self {
            price_range: RangeSlider::new(0.0, 10000.0)
                .with_min(500.0)
                .with_max(5000.0),
        }
    }
    
    fn show(&mut self, ui: &mut Ui) {
        ui.label("Price Range:");
        self.price_range.show(ui);
        
        let min = self.price_range.min_value();
        let max = self.price_range.max_value();
        ui.label(format!("${:.0} - ${:.0}", min, max));
        
        let count = count_products_in_range(min, max);
        ui.label(format!("{} products found", count));
        
        if ui.button("Apply Filter").clicked() {
            apply_filter(min, max);
        }
    }
}
```

**Strengths**:
- Intuitive dual-handle control
- Visual range feedback
- Precise value control
- Real-time updates

**Limitations**:
- Requires horizontal space (~200px)
- Two handles can collide
- Not for discrete values

---

## Animation Systems

### 🎬 Tween<T>

**Visual Description**: Value interpolation with 15 easing curves (linear, sine, elastic, bounce, etc.).

**Best For**:
- UI animations (panel slide, fade in/out)
- Position tweening (move object A → B)
- Color transitions (red → blue)
- Size changes (scale 1× → 2×)
- Rotation (0° → 360°)

**When to Use**:
- ✅ A → B interpolation needed
- ✅ Specific duration required
- ✅ Easing function control
- ✅ Known start/end values

**When NOT to Use**:
- ❌ Physics-based motion (use Spring)
- ❌ User-controlled (use direct input)
- ❌ Infinite loops (use modulo math)

**Code Signature**:
```rust
let mut tween = Tween::new(0.0, 100.0, 2.0)
    .with_easing(EasingFunction::ElasticOut);
tween.play();

// Every frame
tween.update(dt);
let value = tween.value();
```

**Easing Guide**:
- **Linear** - Loading bars, constant motion
- **SineInOut** - Smooth UI panels, modals
- **QuadOut** - Button clicks, snappy responses
- **ElasticOut** - Bouncy game UI, playful effects
- **BounceOut** - Falling objects, drop animations
- **BackOut** - Overshoot effects, emphasis

**Real-World Example**:
```rust
// Loading screen spinner
struct LoadingScreen {
    rotation: Tween<f32>,
}

impl LoadingScreen {
    fn new() -> Self {
        let mut tween = Tween::new(0.0, 360.0, 2.0)
            .with_easing(EasingFunction::Linear);
        tween.play();
        
        Self { rotation: tween }
    }
    
    fn show(&mut self, ui: &mut Ui, dt: f32) {
        self.rotation.update(dt);
        let angle = self.rotation.value();
        
        // Loop
        if self.rotation.is_finished() {
            self.rotation.reset();
            self.rotation.play();
        }
        
        // Draw rotating spinner at angle
    }
}
```

**Strengths**:
- 15 easing functions
- Generic over types (f32, Color32, Pos2)
- Play/pause/reset control
- Precise timing

**Limitations**:
- Fixed start/end (not dynamic target)
- Requires manual looping
- Not physics-based

---

### 🌊 Spring

**Visual Description**: Physics-based animation with mass, stiffness, damping (like a real spring).

**Best For**:
- Following targets (camera follow, cursor follower)
- Natural motion (UI elements settling)
- Responsive controls (smooth input lag)
- Interactive elements (button hover, drag-and-drop)

**When to Use**:
- ✅ Natural, physics-based motion
- ✅ Dynamic target changes
- ✅ Smooth following behavior
- ✅ Overshoot/settle effects

**When NOT to Use**:
- ❌ Precise timing needed (use Tween)
- ❌ Fixed duration required (use Tween)
- ❌ No physics desired (use Tween)

**Code Signature**:
```rust
let mut spring = Spring::with_params(0.5, SpringParams::smooth());
spring.set_target(1.0);

// Every frame
spring.update(dt);
let pos = spring.position();
let vel = spring.velocity();
```

**Presets**:
- **smooth()** - No overshoot, gentle (UI panels, fades)
- **bouncy()** - Overshoot, oscillate (game UI, playful)
- **stiff()** - Fast response (cursor followers, responsive controls)

**Real-World Example**:
```rust
// Mouse follower
struct Follower {
    x_spring: Spring,
    y_spring: Spring,
}

impl Follower {
    fn new() -> Self {
        Self {
            x_spring: Spring::with_params(0.0, SpringParams::smooth()),
            y_spring: Spring::with_params(0.0, SpringParams::smooth()),
        }
    }
    
    fn update(&mut self, mouse_x: f64, mouse_y: f64, dt: f32) {
        self.x_spring.set_target(mouse_x);
        self.y_spring.set_target(mouse_y);
        
        self.x_spring.update(dt);
        self.y_spring.update(dt);
        
        let x = self.x_spring.position();
        let y = self.y_spring.position();
        
        // Draw circle at (x, y) following mouse
    }
}
```

**Strengths**:
- Natural, physics-based
- Dynamic target changes
- Velocity-aware
- Customizable parameters

**Limitations**:
- No precise duration
- Requires parameter tuning
- Overdamped can be slow

---

### 🎭 AnimationController

**Visual Description**: Multi-animation manager for coordinated sequences.

**Best For**:
- Complex sequences (step1 → step2 → step3)
- Coordinated animations (x + y + rotation together)
- State machines (idle → walk → run)
- Cutscenes (multiple simultaneous animations)

**When to Use**:
- ✅ Multiple animations together
- ✅ Sequencing required
- ✅ Named animation IDs
- ✅ Coordinated control

**When NOT to Use**:
- ❌ Single animation (use Tween/Spring directly)
- ❌ Simple cases (over-engineering)
- ❌ No coordination needed

**Code Signature**:
```rust
let mut controller = AnimationController::new();
controller.add_animation("x", Tween::new(0.0, 100.0, 2.0));
controller.add_animation("y", Tween::new(0.0, 50.0, 1.5));
controller.play_all();

// Every frame
controller.update_all(dt);
let x = controller.get_value::<f32>("x").unwrap();
let y = controller.get_value::<f32>("y").unwrap();
```

**Real-World Example**:
```rust
// Character animation
struct Character {
    animations: AnimationController,
}

impl Character {
    fn new() -> Self {
        let mut ctrl = AnimationController::new();
        
        ctrl.add_animation("walk_x", Tween::new(0.0, 100.0, 3.0));
        ctrl.add_animation("walk_y", Tween::new(0.0, 0.0, 3.0));
        ctrl.add_animation("bob", Tween::new(0.0, 5.0, 0.5).with_easing(EasingFunction::SineInOut));
        
        Self { animations: ctrl }
    }
    
    fn walk(&mut self) {
        self.animations.play_all();
    }
    
    fn update(&mut self, dt: f32) {
        self.animations.update_all(dt);
        
        let x = self.animations.get_value::<f32>("walk_x").unwrap();
        let y = self.animations.get_value::<f32>("walk_y").unwrap();
        let bob = self.animations.get_value::<f32>("bob").unwrap();
        
        // Position character at (x, y + bob)
        
        // Loop bob animation
        if self.animations.is_finished("bob") {
            self.animations.play("bob");
        }
    }
}
```

**Strengths**:
- Multiple animations together
- Named access (string IDs)
- Centralized control
- Sequencing support

**Limitations**:
- Overhead for single animations
- String IDs (not type-safe)
- Requires boilerplate

---

## Widget Comparison Matrix

| Widget | Category | Input | Output | State | Space | Complexity |
|--------|----------|-------|--------|-------|-------|------------|
| LineChart | Chart | Points | Visual | Stateless | Medium | Low |
| BarChart | Chart | Groups | Visual | Stateless | Medium | Low |
| ScatterPlot | Chart | Clusters | Visual | Stateless | Medium | Low |
| NodeGraph | Graph | Nodes/Edges | Visual + Click | Stateful | Large | High |
| ColorPicker | Widget | None | Color | Stateful | Medium | Medium |
| TreeView | Widget | Nodes | Click | Stateful | Medium | Medium |
| RangeSlider | Widget | None | Min/Max | Stateful | Small | Low |
| Tween | Animation | Start/End | Value | Stateful | N/A | Low |
| Spring | Animation | Target | Position | Stateful | N/A | Medium |
| AnimationController | Animation | Animations | Values | Stateful | N/A | Medium |

---

## Use Case Index

### By Industry

**Game Development**:
- NodeGraph → AI behavior trees
- Spring → Camera follow, smooth movement
- Tween → UI animations, health bars
- LineChart → Performance monitoring

**Business/Analytics**:
- LineChart → Sales trends, KPIs
- BarChart → Regional comparisons, quarterly reports
- ScatterPlot → Customer segmentation, correlation analysis

**Creative Tools**:
- ColorPicker → Theme editors, drawing apps
- TreeView → Asset browsers, scene hierarchies
- NodeGraph → Shader editors, material editors

**Web Apps**:
- RangeSlider → Price filters, date ranges
- Tween → Page transitions, loading animations
- TreeView → Navigation menus, file browsers

### By Task

**Data Visualization**:
1. LineChart (trends)
2. BarChart (categories)
3. ScatterPlot (correlation)

**Visual Programming**:
1. NodeGraph (logic editing)

**User Input**:
1. ColorPicker (color selection)
2. RangeSlider (range selection)
3. TreeView (hierarchy navigation)

**Motion & Polish**:
1. Tween (precise timing)
2. Spring (natural motion)
3. AnimationController (coordination)

---

## Quick Selection Guide

**"I need to..."**

→ **Show trends over time** → LineChart  
→ **Compare categories** → BarChart  
→ **Find correlations** → ScatterPlot  
→ **Build visual logic** → NodeGraph  
→ **Let user pick colors** → ColorPicker  
→ **Show file structure** → TreeView  
→ **Filter by range** → RangeSlider  
→ **Animate A to B** → Tween  
→ **Follow a target** → Spring  
→ **Coordinate animations** → AnimationController

---

## See Also

- [API Reference](./API_REFERENCE.md) - Complete method documentation
- [Getting Started](./GETTING_STARTED.md) - Installation and first app
- [Tutorials](./GETTING_STARTED.md#next-steps) - Detailed guides
- [Gallery Example](../../examples/astract_gallery/) - See all widgets live

---

**Choose the right widget for the job! 🎨**
