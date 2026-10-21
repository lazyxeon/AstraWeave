# Charts Tutorial

Learn how to create beautiful, interactive charts with Astract's charting system.

---

## Table of Contents

1. [Overview](#overview)
2. [LineChart](#linechart)
3. [BarChart](#barchart)
4. [ScatterPlot](#scatterplot)
5. [Real-World Examples](#real-world-examples)
6. [Best Practices](#best-practices)

---

## Overview

Astract provides three powerful chart types:

- **LineChart** - Time-series data, trends, continuous metrics
- **BarChart** - Categorical comparisons, grouped data
- **ScatterPlot** - Point clusters, correlation analysis

All charts share common features:
- Interactive tooltips
- Customizable colors
- Responsive sizing
- Smooth rendering

### Point Type

**Critical**: Astract uses `(f64, f64)` **tuples** for chart points:

```rust
pub type Point = (f64, f64);

// Correct ✅
let points = vec![(0.0, 1.0), (1.0, 2.0), (2.0, 1.5)];

// Wrong ❌
struct Point { x: f64, y: f64 }  // Don't create custom structs!
```

---

## LineChart

Perfect for visualizing trends over time or continuous data.

### Basic Example

```rust
use astract::prelude::egui::*;
use astract::charts::LineChart;

fn show_line_chart(ui: &mut Ui) {
    let mut chart = LineChart::new("Performance Over Time");
    
    // Add data series (name, points, color)
    chart.add_series(
        "FPS",
        vec![
            (0.0, 60.0),
            (1.0, 58.0),
            (2.0, 62.0),
            (3.0, 59.0),
        ],
        Color32::GREEN,
    );
    
    // Display chart
    chart.show(ui);
}
```

### Multiple Series

Compare multiple datasets on one chart:

```rust
fn show_multi_series(ui: &mut Ui) {
    let mut chart = LineChart::new("CPU vs GPU Usage");
    
    // CPU usage (blue)
    chart.add_series(
        "CPU",
        vec![(0.0, 45.0), (1.0, 52.0), (2.0, 48.0), (3.0, 50.0)],
        Color32::from_rgb(70, 130, 180),  // Steel blue
    );
    
    // GPU usage (orange)
    chart.add_series(
        "GPU",
        vec![(0.0, 60.0), (1.0, 65.0), (2.0, 62.0), (3.0, 68.0)],
        Color32::from_rgb(255, 140, 0),   // Dark orange
    );
    
    chart.show(ui);
}
```

### Mathematical Functions

Visualize mathematical functions:

```rust
fn show_sine_wave(ui: &mut Ui) {
    let mut chart = LineChart::new("Sine Wave");
    
    // Generate sine wave data
    let points: Vec<(f64, f64)> = (0..100)
        .map(|i| {
            let x = i as f64 * 0.1;
            let y = (x * 2.0).sin();
            (x, y)
        })
        .collect();
    
    chart.add_series("sin(2x)", points, Color32::BLUE);
    chart.show(ui);
}
```

### Interactive Controls

Add sliders to control chart parameters:

```rust
struct ChartApp {
    point_count: usize,
}

impl ChartApp {
    fn show(&mut self, ui: &mut Ui) {
        // Slider to control data density
        ui.horizontal(|ui| {
            ui.label("Points:");
            ui.add(Slider::new(&mut self.point_count, 10..=200));
        });
        
        // Generate data based on slider
        let points: Vec<(f64, f64)> = (0..self.point_count)
            .map(|i| {
                let x = i as f64 / self.point_count as f64 * 10.0;
                let y = (x * 2.0).sin();
                (x, y)
            })
            .collect();
        
        let mut chart = LineChart::new("Adjustable Sine Wave");
        chart.add_series("sin(x)", points, Color32::BLUE);
        chart.show(ui);
    }
}
```

### Real-Time Data

Create live updating charts:

```rust
struct LiveChart {
    history: Vec<(f64, f64)>,
    time: f64,
}

impl LiveChart {
    fn update(&mut self, dt: f32) {
        self.time += dt as f64;
        
        // Add new data point
        let value = (self.time * 2.0).sin() * 50.0 + 50.0;
        self.history.push((self.time, value));
        
        // Keep last 100 points
        if self.history.len() > 100 {
            self.history.remove(0);
        }
    }
    
    fn show(&mut self, ui: &mut Ui) {
        let mut chart = LineChart::new("Live Data Stream");
        chart.add_series("Value", self.history.clone(), Color32::GREEN);
        chart.show(ui);
    }
}
```

---

## BarChart

Ideal for comparing categories or grouped data.

### Basic Example

```rust
use astract::charts::{BarChart, BarGroup, Bar};

fn show_bar_chart(ui: &mut Ui) {
    let mut chart = BarChart::new("Quarterly Sales");
    
    // Create a group of bars for Q1
    let q1 = BarGroup {
        category: "Q1".to_string(),
        bars: vec![
            Bar {
                label: "Product A".to_string(),
                value: 65.0,
                color: Color32::from_rgb(255, 99, 71),  // Tomato
            },
            Bar {
                label: "Product B".to_string(),
                value: 45.0,
                color: Color32::from_rgb(70, 130, 180), // Steel blue
            },
        ],
    };
    
    chart.add_group(q1);
    chart.show(ui);
}
```

### Multiple Categories

Compare across multiple periods:

```rust
fn show_grouped_bars(ui: &mut Ui) {
    let mut chart = BarChart::new("Sales by Quarter");
    
    // Q1 data
    chart.add_group(BarGroup {
        category: "Q1".to_string(),
        bars: vec![
            Bar { label: "A".into(), value: 65.0, color: Color32::RED },
            Bar { label: "B".into(), value: 45.0, color: Color32::BLUE },
            Bar { label: "C".into(), value: 55.0, color: Color32::GREEN },
        ],
    });
    
    // Q2 data
    chart.add_group(BarGroup {
        category: "Q2".to_string(),
        bars: vec![
            Bar { label: "A".into(), value: 70.0, color: Color32::RED },
            Bar { label: "B".into(), value: 50.0, color: Color32::BLUE },
            Bar { label: "C".into(), value: 60.0, color: Color32::GREEN },
        ],
    });
    
    // Q3 data
    chart.add_group(BarGroup {
        category: "Q3".to_string(),
        bars: vec![
            Bar { label: "A".into(), value: 75.0, color: Color32::RED },
            Bar { label: "B".into(), value: 55.0, color: Color32::BLUE },
            Bar { label: "C".into(), value: 65.0, color: Color32::GREEN },
        ],
    });
    
    chart.show(ui);
}
```

### Interactive Bars

Control bar values with sliders:

```rust
struct InteractiveBarChart {
    value_a: f64,
    value_b: f64,
    value_c: f64,
}

impl InteractiveBarChart {
    fn show(&mut self, ui: &mut Ui) {
        ui.heading("Adjust Values:");
        
        // Sliders for each bar
        ui.horizontal(|ui| {
            ui.label("A:");
            ui.add(Slider::new(&mut self.value_a, 0.0..=100.0));
        });
        ui.horizontal(|ui| {
            ui.label("B:");
            ui.add(Slider::new(&mut self.value_b, 0.0..=100.0));
        });
        ui.horizontal(|ui| {
            ui.label("C:");
            ui.add(Slider::new(&mut self.value_c, 0.0..=100.0));
        });
        
        // Create chart with current values
        let mut chart = BarChart::new("Interactive Bars");
        chart.add_group(BarGroup {
            category: "Values".to_string(),
            bars: vec![
                Bar {
                    label: "Product A".into(),
                    value: self.value_a,
                    color: Color32::from_rgb(255, 99, 71),
                },
                Bar {
                    label: "Product B".into(),
                    value: self.value_b,
                    color: Color32::from_rgb(70, 130, 180),
                },
                Bar {
                    label: "Product C".into(),
                    value: self.value_c,
                    color: Color32::from_rgb(144, 238, 144),
                },
            ],
        });
        
        chart.show(ui);
    }
}
```

---

## ScatterPlot

Visualize point clusters and correlations.

### Basic Example

```rust
use astract::charts::{ScatterPlot, PointCluster};

fn show_scatter(ui: &mut Ui) {
    let mut scatter = ScatterPlot::new("Data Clusters");
    
    // Create a cluster of points
    let points = vec![
        (1.0, 2.0),
        (2.0, 3.5),
        (3.0, 3.0),
        (4.0, 5.0),
    ];
    
    let cluster = PointCluster::new(
        "Group A",
        points,
        Color32::BLUE,
    );
    
    scatter.add_cluster(cluster);
    scatter.show(ui);
}
```

### Multiple Clusters

Visualize different groups:

```rust
fn show_multi_cluster(ui: &mut Ui) {
    let mut scatter = ScatterPlot::new("Customer Segments");
    
    // High-value customers (red)
    let high_value = vec![
        (50.0, 80.0),
        (55.0, 85.0),
        (60.0, 90.0),
        (58.0, 88.0),
    ];
    scatter.add_cluster(PointCluster::new(
        "High Value",
        high_value,
        Color32::from_rgb(255, 0, 0),
    ));
    
    // Mid-value customers (blue)
    let mid_value = vec![
        (30.0, 50.0),
        (35.0, 55.0),
        (32.0, 52.0),
        (38.0, 58.0),
    ];
    scatter.add_cluster(PointCluster::new(
        "Mid Value",
        mid_value,
        Color32::from_rgb(0, 0, 255),
    ));
    
    // Low-value customers (green)
    let low_value = vec![
        (10.0, 20.0),
        (15.0, 25.0),
        (12.0, 22.0),
        (18.0, 28.0),
    ];
    scatter.add_cluster(PointCluster::new(
        "Low Value",
        low_value,
        Color32::from_rgb(0, 255, 0),
    ));
    
    scatter.show(ui);
}
```

### Random Data Generation

Create scatter plots from random data:

```rust
use rand::Rng;

fn generate_cluster(
    center_x: f64,
    center_y: f64,
    count: usize,
    spread: f64,
) -> Vec<(f64, f64)> {
    let mut rng = rand::thread_rng();
    
    (0..count)
        .map(|_| {
            let x = center_x + rng.gen_range(-spread..spread);
            let y = center_y + rng.gen_range(-spread..spread);
            (x, y)
        })
        .collect()
}

fn show_random_scatter(ui: &mut Ui) {
    let mut scatter = ScatterPlot::new("Random Clusters");
    
    // Cluster 1
    let cluster1 = generate_cluster(20.0, 30.0, 50, 10.0);
    scatter.add_cluster(PointCluster::new(
        "Cluster 1",
        cluster1,
        Color32::RED,
    ));
    
    // Cluster 2
    let cluster2 = generate_cluster(50.0, 60.0, 50, 10.0);
    scatter.add_cluster(PointCluster::new(
        "Cluster 2",
        cluster2,
        Color32::BLUE,
    ));
    
    scatter.show(ui);
}
```

---

## Real-World Examples

### Game Performance Dashboard

```rust
struct PerformanceDashboard {
    fps_history: Vec<(f64, f64)>,
    memory_history: Vec<(f64, f64)>,
    time: f64,
}

impl PerformanceDashboard {
    fn update(&mut self, fps: f64, memory_mb: f64, dt: f32) {
        self.time += dt as f64;
        
        self.fps_history.push((self.time, fps));
        self.memory_history.push((self.time, memory_mb));
        
        // Keep last 60 seconds
        let cutoff = self.time - 60.0;
        self.fps_history.retain(|(t, _)| *t >= cutoff);
        self.memory_history.retain(|(t, _)| *t >= cutoff);
    }
    
    fn show(&self, ui: &mut Ui) {
        ui.heading("Performance Dashboard");
        
        // FPS chart
        let mut fps_chart = LineChart::new("FPS (Last 60s)");
        fps_chart.add_series(
            "FPS",
            self.fps_history.clone(),
            Color32::GREEN,
        );
        fps_chart.show(ui);
        
        ui.add_space(10.0);
        
        // Memory chart
        let mut mem_chart = LineChart::new("Memory Usage (MB)");
        mem_chart.add_series(
            "Memory",
            self.memory_history.clone(),
            Color32::BLUE,
        );
        mem_chart.show(ui);
    }
}
```

### Sales Analytics

```rust
struct SalesAnalytics {
    quarterly_sales: Vec<BarGroup>,
}

impl SalesAnalytics {
    fn new() -> Self {
        Self {
            quarterly_sales: vec![
                BarGroup {
                    category: "Q1 2024".into(),
                    bars: vec![
                        Bar { label: "North".into(), value: 125000.0, color: Color32::RED },
                        Bar { label: "South".into(), value: 98000.0, color: Color32::BLUE },
                        Bar { label: "East".into(), value: 110000.0, color: Color32::GREEN },
                        Bar { label: "West".into(), value: 87000.0, color: Color32::YELLOW },
                    ],
                },
                BarGroup {
                    category: "Q2 2024".into(),
                    bars: vec![
                        Bar { label: "North".into(), value: 135000.0, color: Color32::RED },
                        Bar { label: "South".into(), value: 105000.0, color: Color32::BLUE },
                        Bar { label: "East".into(), value: 120000.0, color: Color32::GREEN },
                        Bar { label: "West".into(), value: 92000.0, color: Color32::YELLOW },
                    ],
                },
            ],
        }
    }
    
    fn show(&self, ui: &mut Ui) {
        ui.heading("Regional Sales Performance");
        
        let mut chart = BarChart::new("Sales by Region ($)");
        for group in &self.quarterly_sales {
            chart.add_group(group.clone());
        }
        chart.show(ui);
    }
}
```

### Machine Learning Results

```rust
struct MLVisualization {
    training_loss: Vec<(f64, f64)>,
    validation_loss: Vec<(f64, f64)>,
    clusters: Vec<PointCluster>,
}

impl MLVisualization {
    fn show(&self, ui: &mut Ui) {
        ui.heading("Training Progress");
        
        // Loss curves
        let mut loss_chart = LineChart::new("Loss Over Time");
        loss_chart.add_series(
            "Training Loss",
            self.training_loss.clone(),
            Color32::BLUE,
        );
        loss_chart.add_series(
            "Validation Loss",
            self.validation_loss.clone(),
            Color32::RED,
        );
        loss_chart.show(ui);
        
        ui.add_space(20.0);
        ui.heading("Feature Space Clustering");
        
        // Cluster visualization
        let mut scatter = ScatterPlot::new("Learned Clusters");
        for cluster in &self.clusters {
            scatter.add_cluster(cluster.clone());
        }
        scatter.show(ui);
    }
}
```

---

## Best Practices

### 1. Data Preparation

✅ **DO**: Use f64 tuples for points
```rust
let points: Vec<(f64, f64)> = vec![(0.0, 1.0), (1.0, 2.0)];
```

❌ **DON'T**: Create custom point structs
```rust
struct Point { x: f64, y: f64 }  // ❌ Won't work
```

### 2. Color Selection

✅ **DO**: Use distinct colors for multiple series
```rust
chart.add_series("A", data_a, Color32::RED);
chart.add_series("B", data_b, Color32::BLUE);
chart.add_series("C", data_c, Color32::GREEN);
```

❌ **DON'T**: Use similar colors
```rust
chart.add_series("A", data_a, Color32::from_rgb(100, 0, 0));
chart.add_series("B", data_b, Color32::from_rgb(110, 0, 0));  // ❌ Too similar!
```

### 3. Performance

✅ **DO**: Limit data points for real-time charts
```rust
if history.len() > 1000 {
    history.drain(0..500);  // Remove old data
}
```

❌ **DON'T**: Let data grow unbounded
```rust
history.push(new_point);  // ❌ Memory leak!
```

### 4. Naming

✅ **DO**: Use clear, descriptive titles
```rust
LineChart::new("CPU Usage Over Last Hour (%)")
```

❌ **DON'T**: Use vague titles
```rust
LineChart::new("Chart")  // ❌ Not helpful
```

---

## Common Pitfalls

### Pitfall 1: Wrong Point Type

```rust
// ❌ WRONG: Custom struct
struct Point { x: f64, y: f64 }
let points = vec![Point { x: 0.0, y: 1.0 }];

// ✅ CORRECT: Use tuples
let points = vec![(0.0, 1.0), (1.0, 2.0)];
```

### Pitfall 2: Modifying Chart After show()

```rust
// ❌ WRONG: Can't modify after .show()
let mut chart = LineChart::new("Title");
chart.show(ui);
chart.add_series("Data", points, Color32::RED);  // ❌ Too late!

// ✅ CORRECT: Add all series before .show()
let mut chart = LineChart::new("Title");
chart.add_series("Data", points, Color32::RED);
chart.show(ui);  // ✅ Show after setup
```

### Pitfall 3: Forgetting to Clone Data

```rust
// ❌ WRONG: Moves self.data
chart.add_series("Data", self.data, Color32::RED);
// self.data is now gone!

// ✅ CORRECT: Clone if you need to keep it
chart.add_series("Data", self.data.clone(), Color32::RED);
// self.data still available
```

---

## Next Steps

- **[Advanced Widgets Tutorial](./ADVANCED_WIDGETS_TUTORIAL.md)** - ColorPicker, TreeView, RangeSlider
- **[NodeGraph Tutorial](./NODEGRAPH_TUTORIAL.md)** - Visual node editors
- **[Animation Tutorial](./ANIMATION_TUTORIAL.md)** - Smooth transitions
- **[Gallery Example](../../examples/astract_gallery/)** - See all charts in action

---

**Ready to create beautiful charts! 📊**
