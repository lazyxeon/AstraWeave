# Astract Integration Guide

Real-world workflows, patterns, and best practices.

---

## Table of Contents

1. [Common Workflows](#common-workflows)
2. [Architecture Patterns](#architecture-patterns)
3. [Performance Optimization](#performance-optimization)
4. [Error Handling](#error-handling)
5. [Testing Strategies](#testing-strategies)
6. [Production Checklist](#production-checklist)

---

## Common Workflows

### 1. Analytics Dashboard

**Goal**: Multi-chart dashboard with real-time updates.

**Components**:
- LineChart (time series)
- BarChart (categories)
- ScatterPlot (correlations)

**Architecture**:
```rust
struct AnalyticsDashboard {
    // Data sources
    time_series: Vec<Vec<Point>>,
    categorical: Vec<BarGroup>,
    clusters: Vec<PointCluster>,
    
    // Charts (stateless)
    // Created fresh each frame
}

impl AnalyticsDashboard {
    fn show(&mut self, ui: &mut Ui) {
        // Layout: 2×2 grid
        ui.columns(2, |cols| {
            // Top-left: Line chart
            let mut line = LineChart::new("Metrics Over Time");
            for (i, series) in self.time_series.iter().enumerate() {
                line.add_series(
                    format!("Metric {}", i),
                    series.clone(),
                    COLORS[i],
                );
            }
            line.show(&mut cols[0]);
            
            // Top-right: Bar chart
            let mut bar = BarChart::new("Category Breakdown");
            for group in &self.categorical {
                bar.add_group(group.clone());
            }
            bar.show(&mut cols[1]);
        });
        
        ui.columns(2, |cols| {
            // Bottom-left: Scatter
            let mut scatter = ScatterPlot::new("Correlation");
            for cluster in &self.clusters {
                scatter.add_cluster(cluster.clone());
            }
            scatter.show(&mut cols[0]);
            
            // Bottom-right: Controls
            cols[1].label("Update Rate:");
            // Add sliders, buttons, etc.
        });
    }
}
```

**Best Practices**:
- ✅ Recreate charts each frame (minimal overhead)
- ✅ Cache data, not charts
- ✅ Use columns for grid layout
- ✅ Consistent color palette
- ⚠️ Limit to 5-6 series per chart
- ⚠️ Update data on timer, not every frame

---

### 2. Visual AI Editor

**Goal**: Node-based behavior tree with live preview.

**Components**:
- NodeGraph (main editor)
- TreeView (node palette)
- ColorPicker (node colors)

**Architecture**:
```rust
struct AIEditor {
    graph: NodeGraph,
    palette: TreeView,
    selected_color: ColorPicker,
    
    // State
    selected_node: Option<usize>,
    next_id: usize,
}

impl AIEditor {
    fn new() -> Self {
        // Build palette
        let mut palette = TreeView::new();
        let root = palette.add_node(TreeNode::new(1, "Nodes").with_icon("📦"));
        
        let logic = palette.add_child(root, TreeNode::new(2, "Logic").with_icon("📁"));
        palette.add_child(logic, TreeNode::new(3, "Sequence").with_icon("▶️"));
        palette.add_child(logic, TreeNode::new(4, "Selector").with_icon("🔀"));
        
        let actions = palette.add_child(root, TreeNode::new(5, "Actions").with_icon("📁"));
        palette.add_child(actions, TreeNode::new(6, "Attack").with_icon("⚔️"));
        palette.add_child(actions, TreeNode::new(7, "Move").with_icon("🚶"));
        
        Self {
            graph: NodeGraph::new(),
            palette,
            selected_color: ColorPicker::new().with_color(Color32::from_rgb(100, 100, 255)),
            selected_node: None,
            next_id: 1,
        }
    }
    
    fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Left: Palette (200px)
            ui.vertical(|ui| {
                ui.set_width(200.0);
                ui.heading("Node Palette");
                
                if let Some(clicked) = self.palette.show(ui) {
                    self.add_node_from_palette(clicked);
                }
            });
            
            ui.separator();
            
            // Right: Graph + inspector
            ui.vertical(|ui| {
                // Graph (takes remaining space)
                ui.heading("Behavior Tree");
                
                if let Some(clicked) = self.graph.show(ui) {
                    self.selected_node = Some(clicked);
                }
                
                // Inspector (bottom panel)
                if let Some(id) = self.selected_node {
                    ui.separator();
                    ui.heading(format!("Node {} Properties", id));
                    
                    ui.label("Color:");
                    self.selected_color.show(ui);
                    
                    if ui.button("Apply Color").clicked() {
                        // Update node color
                    }
                    
                    if ui.button("Delete Node").clicked() {
                        // Remove from graph
                        self.selected_node = None;
                    }
                }
            });
        });
    }
    
    fn add_node_from_palette(&mut self, palette_id: usize) {
        let (label, ports) = match palette_id {
            3 => ("Sequence", vec![
                Port::new(0, "In", PortType::Exec),
                Port::new(0, "Out 1", PortType::Exec),
                Port::new(1, "Out 2", PortType::Exec),
            ]),
            4 => ("Selector", vec![
                Port::new(0, "In", PortType::Exec),
                Port::new(0, "Option 1", PortType::Bool),
                Port::new(1, "Option 2", PortType::Bool),
            ]),
            6 => ("Attack", vec![
                Port::new(0, "Execute", PortType::Exec),
                Port::new(0, "Target", PortType::Object),
            ]),
            7 => ("Move", vec![
                Port::new(0, "Execute", PortType::Exec),
                Port::new(0, "Destination", PortType::Object),
            ]),
            _ => return,
        };
        
        let mut node = GraphNode::new(self.next_id, label.to_string());
        for port in ports {
            if port.index == 0 {
                node.add_input(port);
            } else {
                node.add_output(port);
            }
        }
        
        self.graph.add_node(node.with_position(400.0, 200.0));
        self.next_id += 1;
    }
}
```

**Best Practices**:
- ✅ Use TreeView as palette
- ✅ Track selected node for inspector
- ✅ Horizontal layout for palette + graph
- ✅ Color-code port types
- ⚠️ Validate connections (type matching)
- ⚠️ Implement drag-to-connect UX

---

### 3. Theme Customizer

**Goal**: Live theme editor with preview.

**Components**:
- ColorPicker (multiple colors)
- RangeSlider (opacity, spacing)
- Animated preview (Tween/Spring)

**Architecture**:
```rust
struct ThemeEditor {
    // Colors
    bg_picker: ColorPicker,
    fg_picker: ColorPicker,
    accent_picker: ColorPicker,
    
    // Adjustments
    opacity: RangeSlider,
    spacing: RangeSlider,
    
    // Preview animation
    preview_pos: Spring,
}

impl ThemeEditor {
    fn new() -> Self {
        Self {
            bg_picker: ColorPicker::new().with_color(Color32::from_gray(30)),
            fg_picker: ColorPicker::new().with_color(Color32::WHITE),
            accent_picker: ColorPicker::new().with_color(Color32::from_rgb(0, 120, 255)),
            
            opacity: RangeSlider::new(0.0, 1.0).with_min(0.8).with_max(1.0),
            spacing: RangeSlider::new(0.0, 20.0).with_min(4.0).with_max(12.0),
            
            preview_pos: Spring::with_params(0.0, SpringParams::smooth()),
        }
    }
    
    fn show(&mut self, ui: &mut Ui, dt: f32) {
        ui.horizontal(|ui| {
            // Left: Controls
            ui.vertical(|ui| {
                ui.set_width(250.0);
                ui.heading("Theme Colors");
                
                ui.label("Background:");
                self.bg_picker.show(ui);
                
                ui.label("Foreground:");
                self.fg_picker.show(ui);
                
                ui.label("Accent:");
                self.accent_picker.show(ui);
                
                ui.separator();
                ui.heading("Adjustments");
                
                ui.label("Opacity:");
                self.opacity.show(ui);
                
                ui.label("Spacing:");
                self.spacing.show(ui);
                
                ui.separator();
                
                if ui.button("Export Theme").clicked() {
                    self.export_theme();
                }
                
                if ui.button("Reset").clicked() {
                    *self = Self::new();
                }
            });
            
            ui.separator();
            
            // Right: Live preview
            ui.vertical(|ui| {
                ui.heading("Preview");
                
                // Update animation
                self.preview_pos.update(dt);
                let hover_offset = self.preview_pos.position() as f32;
                
                // Apply theme
                let mut style = (*ui.ctx().style()).clone();
                style.visuals.window_fill = self.bg_picker.color();
                style.visuals.override_text_color = Some(self.fg_picker.color());
                style.visuals.widgets.active.bg_fill = self.accent_picker.color();
                style.spacing.item_spacing = egui::vec2(
                    self.spacing.min_value() as f32,
                    self.spacing.max_value() as f32,
                );
                
                ui.ctx().set_style(style);
                
                // Preview widgets
                Frame::window(&ui.style())
                    .show(ui, |ui| {
                        ui.heading("Sample Window");
                        ui.label("This is sample text");
                        
                        if ui.button("Sample Button").hovered() {
                            self.preview_pos.set_target(10.0);
                        } else if self.preview_pos.position() > 1.0 {
                            self.preview_pos.set_target(0.0);
                        }
                        
                        // Use hover offset for spring effect
                        ui.add_space(hover_offset);
                        
                        ui.separator();
                        ui.checkbox(&mut true, "Checkbox");
                        ui.radio(true, "Radio");
                    });
            });
        });
    }
    
    fn export_theme(&self) {
        // Serialize to TOML/JSON
    }
}
```

**Best Practices**:
- ✅ Live preview updates immediately
- ✅ Reset button for defaults
- ✅ Export to persistent format
- ✅ Use Spring for hover effects
- ⚠️ Restore original style after preview
- ⚠️ Validate color contrast (accessibility)

---

## Architecture Patterns

### Pattern 1: Stateless Charts

**When**: Charts, static visualizations

**Why**: Minimal state, easy to reason about

```rust
// ✅ GOOD: Stateless
fn show_chart(ui: &mut Ui, data: &[Point]) {
    let mut chart = LineChart::new("Title");
    chart.add_series("Data", data.to_vec(), Color32::BLUE);
    chart.show(ui);
}
```

```rust
// ❌ BAD: Unnecessary state
struct ChartState {
    chart: LineChart,  // Recreated every frame anyway!
}
```

---

### Pattern 2: Stateful Widgets

**When**: ColorPicker, RangeSlider, TreeView (user interaction)

**Why**: Preserve state across frames

```rust
// ✅ GOOD: Stateful in app struct
struct App {
    picker: ColorPicker,
    range: RangeSlider,
}

impl App {
    fn show(&mut self, ui: &mut Ui) {
        self.picker.show(ui);
        self.range.show(ui);
        
        // Use values
        let color = self.picker.color();
        let min = self.range.min_value();
    }
}
```

```rust
// ❌ BAD: Recreating stateful widgets
fn show(ui: &mut Ui) {
    let mut picker = ColorPicker::new();  // Lost state!
    picker.show(ui);
}
```

---

### Pattern 3: Animation Coordination

**When**: Multiple simultaneous animations

**Why**: Centralized control, synchronization

```rust
// ✅ GOOD: AnimationController
struct AnimatedPanel {
    controller: AnimationController,
}

impl AnimatedPanel {
    fn new() -> Self {
        let mut ctrl = AnimationController::new();
        ctrl.add_animation("x", Box::new(Tween::new(0.0, 100.0, 2.0)));
        ctrl.add_animation("y", Box::new(Tween::new(0.0, 50.0, 2.0)));
        ctrl.add_animation("alpha", Box::new(Tween::new(0.0, 1.0, 1.5)));
        
        Self { controller: ctrl }
    }
    
    fn show(&mut self, ui: &mut Ui, dt: f32) {
        self.controller.update_all(dt);
        
        let x = self.controller.get_value::<f32>("x").unwrap();
        let y = self.controller.get_value::<f32>("y").unwrap();
        let alpha = self.controller.get_value::<f32>("alpha").unwrap();
        
        // Render at (x, y) with alpha
    }
}
```

```rust
// ❌ BAD: Scattered animations
struct AnimatedPanel {
    x_tween: Tween<f32>,
    y_tween: Tween<f32>,
    alpha_tween: Tween<f32>,
}

// Must update/check each separately (error-prone)
```

---

### Pattern 4: Data-Driven UI

**When**: Dynamic widget creation

**Why**: Flexible, data-driven

```rust
#[derive(Clone)]
enum ChartConfig {
    Line { series: Vec<(String, Vec<Point>, Color32)> },
    Bar { groups: Vec<BarGroup> },
    Scatter { clusters: Vec<PointCluster> },
}

fn show_chart(ui: &mut Ui, config: &ChartConfig) {
    match config {
        ChartConfig::Line { series } => {
            let mut chart = LineChart::new("Data");
            for (name, points, color) in series {
                chart.add_series(name.clone(), points.clone(), *color);
            }
            chart.show(ui);
        }
        ChartConfig::Bar { groups } => {
            let mut chart = BarChart::new("Data");
            for group in groups {
                chart.add_group(group.clone());
            }
            chart.show(ui);
        }
        ChartConfig::Scatter { clusters } => {
            let mut scatter = ScatterPlot::new("Data");
            for cluster in clusters {
                scatter.add_cluster(cluster.clone());
            }
            scatter.show(ui);
        }
    }
}
```

---

## Performance Optimization

### 1. Data Caching

```rust
// ✅ GOOD: Cache processed data
struct Dashboard {
    raw_data: Vec<Record>,
    cached_points: Vec<Point>,  // Preprocessed
    last_update: Instant,
}

impl Dashboard {
    fn update_cache(&mut self) {
        if self.last_update.elapsed() > Duration::from_secs(1) {
            self.cached_points = self.raw_data.iter()
                .map(|r| (r.time, r.value))
                .collect();
            self.last_update = Instant::now();
        }
    }
    
    fn show(&mut self, ui: &mut Ui) {
        self.update_cache();
        
        let mut chart = LineChart::new("Title");
        chart.add_series("Data", self.cached_points.clone(), Color32::BLUE);
        chart.show(ui);
    }
}
```

```rust
// ❌ BAD: Reprocessing every frame
fn show(ui: &mut Ui, raw_data: &[Record]) {
    let points: Vec<Point> = raw_data.iter()  // Allocates every frame!
        .map(|r| (r.time, r.value))
        .collect();
    
    let mut chart = LineChart::new("Title");
    chart.add_series("Data", points, Color32::BLUE);
    chart.show(ui);
}
```

---

### 2. Animation Throttling

```rust
// ✅ GOOD: Throttle animation updates
struct App {
    spring: Spring,
    last_update: Instant,
}

impl App {
    fn show(&mut self, ui: &mut Ui) {
        let dt = self.last_update.elapsed().as_secs_f32();
        
        if dt > 0.016 {  // 60 FPS max
            self.spring.update(dt);
            self.last_update = Instant::now();
        }
        
        let pos = self.spring.position();
        // Render
    }
}
```

---

### 3. Conditional Rendering

```rust
// ✅ GOOD: Only render visible charts
struct Dashboard {
    charts: Vec<ChartConfig>,
    visible_tab: usize,
}

impl Dashboard {
    fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            for i in 0..self.charts.len() {
                if ui.selectable_label(self.visible_tab == i, format!("Chart {}", i)).clicked() {
                    self.visible_tab = i;
                }
            }
        });
        
        // Only render active tab
        show_chart(ui, &self.charts[self.visible_tab]);
    }
}
```

---

### 4. Point Decimation

```rust
// ✅ GOOD: Reduce points for large datasets
fn decimate_points(points: &[Point], max_points: usize) -> Vec<Point> {
    if points.len() <= max_points {
        return points.to_vec();
    }
    
    let step = points.len() / max_points;
    points.iter().step_by(step).copied().collect()
}

fn show_chart(ui: &mut Ui, points: &[Point]) {
    let decimated = decimate_points(points, 500);  // Max 500 points
    
    let mut chart = LineChart::new("Title");
    chart.add_series("Data", decimated, Color32::BLUE);
    chart.show(ui);
}
```

---

## Error Handling

### 1. Graceful Degradation

```rust
// ✅ GOOD: Fallback on error
fn show_data_chart(ui: &mut Ui, data: &Result<Vec<Point>, Error>) {
    match data {
        Ok(points) => {
            let mut chart = LineChart::new("Live Data");
            chart.add_series("Data", points.clone(), Color32::GREEN);
            chart.show(ui);
        }
        Err(e) => {
            ui.colored_label(Color32::RED, format!("Error: {}", e));
            ui.label("Using cached data...");
            
            // Show last known good data
            let mut chart = LineChart::new("Cached Data");
            chart.add_series("Data", get_cached_data(), Color32::GRAY);
            chart.show(ui);
        }
    }
}
```

---

### 2. Input Validation

```rust
// ✅ GOOD: Validate before creating chart
fn show_chart(ui: &mut Ui, points: &[Point]) {
    if points.is_empty() {
        ui.label("No data available");
        return;
    }
    
    if points.len() > 10_000 {
        ui.colored_label(Color32::YELLOW, "⚠ Large dataset, rendering may be slow");
    }
    
    let mut chart = LineChart::new("Data");
    chart.add_series("Series", points.to_vec(), Color32::BLUE);
    chart.show(ui);
}
```

---

## Testing Strategies

### 1. Unit Tests for Data Processing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_point_decimation() {
        let points: Vec<Point> = (0..1000).map(|i| (i as f64, i as f64)).collect();
        let decimated = decimate_points(&points, 100);
        
        assert_eq!(decimated.len(), 100);
        assert_eq!(decimated[0], (0.0, 0.0));
        assert_eq!(decimated[99].0, 990.0);  // Approximately
    }
    
    #[test]
    fn test_data_caching() {
        let mut dashboard = Dashboard::new();
        dashboard.raw_data = vec![Record { time: 0.0, value: 1.0 }];
        
        dashboard.update_cache();
        assert_eq!(dashboard.cached_points.len(), 1);
        
        // Should not update immediately
        dashboard.raw_data.push(Record { time: 1.0, value: 2.0 });
        dashboard.update_cache();
        assert_eq!(dashboard.cached_points.len(), 1);  // Still 1 (cached)
    }
}
```

---

### 2. Integration Tests with egui_kittest

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use egui_kittest::Harness;
    
    #[test]
    fn test_dashboard_rendering() {
        let mut harness = Harness::new(|ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut dashboard = Dashboard::new();
                dashboard.show(ui);
            });
        });
        
        harness.run();
        
        // Verify no panics
        assert!(harness.ctx().memory(|mem| mem.everything_is_visible()));
    }
}
```

---

### 3. Visual Regression Tests

```rust
#[test]
fn test_chart_appearance() {
    let mut harness = Harness::new(|ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut chart = LineChart::new("Test");
            chart.add_series("Data", vec![(0.0, 0.0), (1.0, 1.0)], Color32::BLUE);
            chart.show(ui);
        });
    });
    
    let snapshot = harness.snapshot();
    
    // Compare with reference image
    assert_snapshot_matches("chart_baseline.png", snapshot);
}
```

---

## Production Checklist

### Pre-Launch

- [ ] **Performance**: Profile with 10,000+ data points
- [ ] **Error Handling**: Graceful degradation on all errors
- [ ] **Accessibility**: Color contrast >4.5:1
- [ ] **Responsiveness**: Works on 1024×768 minimum
- [ ] **Documentation**: All public APIs documented
- [ ] **Tests**: >80% coverage on data processing
- [ ] **Visual Tests**: Snapshot tests for critical UI

### Optimization

- [ ] **Data**: Cache processed points (update on timer)
- [ ] **Rendering**: Decimate points >500
- [ ] **Animation**: Throttle to 60 FPS max
- [ ] **Memory**: Reuse allocations where possible
- [ ] **State**: Minimize clones, use references

### User Experience

- [ ] **Loading**: Show spinner while loading
- [ ] **Empty State**: Graceful "No data" message
- [ ] **Errors**: User-friendly error messages
- [ ] **Tooltips**: Hover info on all interactive elements
- [ ] **Keyboard**: Tab navigation works
- [ ] **Undo/Redo**: For NodeGraph editing

### Security

- [ ] **Input**: Validate all user data
- [ ] **Sanitization**: Escape labels/titles
- [ ] **Limits**: Cap data points/nodes
- [ ] **Permissions**: Read-only modes

---

## Common Pitfalls

### 1. Recreating Stateful Widgets

```rust
// ❌ WRONG
fn show(ui: &mut Ui) {
    let mut picker = ColorPicker::new();  // State lost every frame!
    picker.show(ui);
}

// ✅ CORRECT
struct App {
    picker: ColorPicker,  // State preserved
}
```

---

### 2. Cloning Large Datasets

```rust
// ❌ WRONG
chart.add_series("Data", huge_vec.clone(), Color32::BLUE);  // Expensive clone!

// ✅ CORRECT
let decimated = decimate_points(&huge_vec, 500);
chart.add_series("Data", decimated, Color32::BLUE);
```

---

### 3. Missing dt Updates

```rust
// ❌ WRONG
tween.update(0.016);  // Hardcoded, not actual frame time!

// ✅ CORRECT
let dt = frame_time.elapsed().as_secs_f32();
tween.update(dt);
```

---

### 4. Ignoring is_finished()

```rust
// ❌ WRONG
tween.update(dt);
let value = tween.value();  // Stuck at end value forever!

// ✅ CORRECT
tween.update(dt);
if tween.is_finished() {
    tween.reset();
    tween.play();
}
let value = tween.value();
```

---

## Cross-References

- [API Reference](./API_REFERENCE.md) - Method signatures
- [Widget Catalog](./WIDGET_CATALOG.md) - Widget guide
- [Method Reference](./METHOD_REFERENCE.md) - Quick lookup
- [Getting Started](./GETTING_STARTED.md) - Installation
- [Tutorials](./GETTING_STARTED.md#next-steps) - Step-by-step

---

## Real-World Examples

See `examples/astract_gallery/` for complete implementations:

- **charts_tab.rs** - Analytics dashboard pattern
- **graphs_tab.rs** - Visual AI editor pattern
- **advanced_tab.rs** - Theme editor pattern
- **animation_tab.rs** - Animation coordination pattern

---

**Build production-ready Astract applications! 🚀**
