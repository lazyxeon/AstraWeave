# Getting Started with Astract

**Astract** is a React-style declarative UI framework for [egui](https://github.com/emilk/egui), providing a productive way to build interactive UIs with Rust.

This guide will help you get up and running with Astract in minutes.

---

## Table of Contents

1. [Installation](#installation)
2. [Your First Widget](#your-first-widget)
3. [Basic RSX Syntax](#basic-rsx-syntax)
4. [Running the Examples](#running-the-examples)
5. [Next Steps](#next-steps)

---

## Installation

### Prerequisites

- **Rust 1.70+** (latest stable recommended)
- **Cargo** (included with Rust)

### Adding Astract to Your Project

Add Astract to your `Cargo.toml`:

```toml
[dependencies]
astract = { git = "https://github.com/lazyxeon/AstraWeave-AI-Native-Gaming-Engine", package = "astract" }
eframe = "0.32"  # For desktop apps
```

Or if you've cloned the AstraWeave repository:

```toml
[dependencies]
astract = { path = "../../crates/astract" }
eframe = "0.32"
```

### Quick Install Script

```bash
# Clone the repository
git clone https://github.com/lazyxeon/AstraWeave-AI-Native-Gaming-Engine
cd AstraWeave-AI-Native-Gaming-Engine

# Build the framework
cargo build -p astract

# Run the gallery example
cargo run -p astract_gallery --release
```

---

## Your First Widget

Let's create a simple counter app to demonstrate Astract's core concepts.

### Step 1: Create a New Binary

```bash
cargo new --bin my_astract_app
cd my_astract_app
```

### Step 2: Add Dependencies

Edit `Cargo.toml`:

```toml
[dependencies]
astract = { git = "https://github.com/lazyxeon/AstraWeave-AI-Native-Gaming-Engine", package = "astract" }
eframe = "0.32"
```

### Step 3: Write Your First App

Edit `src/main.rs`:

```rust
use astract::prelude::egui::*;
use eframe;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_title("My First Astract App"),
        ..Default::default()
    };

    eframe::run_native(
        "Counter",
        options,
        Box::new(|_cc| Ok(Box::new(CounterApp::default()))),
    )
}

#[derive(Default)]
struct CounterApp {
    count: i32,
}

impl eframe::App for CounterApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Counter Example");
            ui.add_space(20.0);
            
            ui.horizontal(|ui| {
                if ui.button("➖ Decrement").clicked() {
                    self.count -= 1;
                }
                
                ui.label(format!("Count: {}", self.count));
                
                if ui.button("➕ Increment").clicked() {
                    self.count += 1;
                }
            });
            
            ui.add_space(10.0);
            
            if ui.button("🔄 Reset").clicked() {
                self.count = 0;
            }
        });
    }
}
```

### Step 4: Run Your App

```bash
cargo run --release
```

You should see a window with increment/decrement buttons!

---

## Basic RSX Syntax

Astract supports **RSX** (Rust Syntax Extension) for declarative UI:

### Traditional egui Code

```rust
ui.horizontal(|ui| {
    ui.label("Name:");
    ui.text_edit_singleline(&mut name);
});

ui.horizontal(|ui| {
    ui.label("Age:");
    ui.add(egui::Slider::new(&mut age, 0..=120));
});

if ui.button("Submit").clicked() {
    submit();
}
```

### With Astract RSX

```rust
use astract::rsx;

rsx! {
    <horizontal>
        <label text="Name:" />
        <text_edit_singleline value={&mut name} />
    </horizontal>
    
    <horizontal>
        <label text="Age:" />
        <slider value={&mut age} range={0..=120} />
    </horizontal>
    
    <button text="Submit" on_click={submit} />
}
```

**Benefits**:
- More concise and readable
- Easier to spot UI structure
- Less nesting depth
- Familiar to React/JSX developers

---

## Running the Examples

Astract includes several examples demonstrating different features:

### Example Gallery (Comprehensive)

**The best way to see all widgets in action!**

```bash
cargo run -p astract_gallery --release
```

**Features**:
- **Charts Tab**: LineChart, BarChart, ScatterPlot demos
- **Advanced Tab**: ColorPicker, TreeView, RangeSlider
- **Graphs Tab**: NodeGraph examples (behavior trees, shader graphs, dialogue)
- **Animation Tab**: Tween, Spring, EasingFunction demos

### Individual Examples

```bash
# Animation demos (from aw_editor)
cargo run -p aw_editor --release
# Click "Animation" panel to see spring physics, tweens, easing

# Counter component (RSX demo)
cargo run -p astract --example counter_component

# Hello companion (AI-native game engine demo)
cargo run -p hello_companion --release
```

---

## Core Concepts

### 1. Widget System

Astract provides stateful widgets that manage their own state:

```rust
use astract::advanced::ColorPicker;

// Create a color picker
let mut picker = ColorPicker::new()
    .with_color(Color32::RED)
    .show_alpha(true);

// Show it in UI
picker.show(ui);

// Get the selected color
let color = picker.color();
```

### 2. Charts

Visualize data with professional charts:

```rust
use astract::charts::LineChart;

let mut chart = LineChart::new("Performance");

// Add data series (tuples of (f64, f64))
chart.add_series(
    "FPS",
    vec![(0.0, 60.0), (1.0, 58.0), (2.0, 62.0)],
    Color32::GREEN
);

chart.show(ui);
```

### 3. Node Graphs

Create visual node editors for AI, shaders, or dialogue:

```rust
use astract::graph::{NodeGraph, GraphNode, Port, PortType};

let mut graph = NodeGraph::new();

// Create nodes
let mut start = GraphNode::new(1, "Start");
start.add_output(Port::new(0, "Out", PortType::Exec));
let start_id = graph.add_node(start);

let mut action = GraphNode::new(2, "Action");
action.add_input(Port::new(0, "In", PortType::Exec));
let action_id = graph.add_node(action);

// Connect them
graph.add_edge(start_id, 0, action_id, 0);

graph.show(ui);
```

### 4. Animations

Smooth, physics-based animations:

```rust
use astract::animation::{Tween, Spring, SpringParams, EasingFunction};

// Tween animation
let mut tween = Tween::new(0.0f32, 100.0f32, 2.0)
    .with_easing(EasingFunction::ElasticOut);
tween.play();

// Update every frame
tween.update(dt);
let value = tween.value();

// Spring physics
let mut spring = Spring::with_params(0.5, SpringParams::bouncy());
spring.set_target(1.0);

spring.update(dt);
let position = spring.position();
```

---

## Project Structure

A typical Astract project looks like:

```
my_app/
├── Cargo.toml
└── src/
    ├── main.rs          # App entry point
    ├── ui/
    │   ├── mod.rs
    │   ├── charts.rs    # Chart components
    │   ├── dialogs.rs   # Dialog components
    │   └── widgets.rs   # Custom widgets
    └── app.rs           # Main app state
```

### Recommended Structure

```rust
// src/main.rs
use eframe;
mod app;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "My App",
        Default::default(),
        Box::new(|_| Ok(Box::new(app::MyApp::default()))),
    )
}

// src/app.rs
use astract::prelude::egui::*;

pub struct MyApp {
    // App state
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // UI code
    }
}
```

---

## Common Patterns

### Pattern 1: Tabbed Interface

```rust
#[derive(Default)]
struct MyApp {
    selected_tab: Tab,
}

#[derive(Default, PartialEq)]
enum Tab {
    #[default]
    Dashboard,
    Settings,
    About,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.selected_tab, Tab::Dashboard, "Dashboard");
                ui.selectable_value(&mut self.selected_tab, Tab::Settings, "Settings");
                ui.selectable_value(&mut self.selected_tab, Tab::About, "About");
            });
        });
        
        CentralPanel::default().show(ctx, |ui| {
            match self.selected_tab {
                Tab::Dashboard => self.show_dashboard(ui),
                Tab::Settings => self.show_settings(ui),
                Tab::About => self.show_about(ui),
            }
        });
    }
}
```

### Pattern 2: Stateful Widgets

```rust
use astract::advanced::{ColorPicker, TreeView, RangeSlider};

struct MyApp {
    color_picker: ColorPicker,
    tree: TreeView,
    range: RangeSlider,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            color_picker: ColorPicker::new()
                .with_color(Color32::BLUE),
            tree: create_tree(),
            range: RangeSlider::new(0.0, 100.0)
                .with_min(25.0)
                .with_max(75.0),
        }
    }
}

impl MyApp {
    fn show_ui(&mut self, ui: &mut Ui) {
        self.color_picker.show(ui);
        let color = self.color_picker.color();
        
        self.tree.show(ui);
        
        self.range.show(ui);
        let min = self.range.min_value();
        let max = self.range.max_value();
    }
}
```

### Pattern 3: Animation Integration

```rust
use astract::animation::{Tween, AnimationState, EasingFunction};

struct MyApp {
    position_tween: Tween<f32>,
    time: f32,
}

impl Default for MyApp {
    fn default() -> Self {
        let mut tween = Tween::new(0.0, 200.0, 2.0)
            .with_easing(EasingFunction::SineInOut);
        tween.play();
        
        Self {
            position_tween: tween,
            time: 0.0,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Get delta time
        let dt = ctx.input(|i| i.stable_dt);
        self.time += dt;
        
        // Update animation
        self.position_tween.update(dt);
        let x = self.position_tween.value();
        
        // Use animated value
        CentralPanel::default().show(ctx, |ui| {
            let pos = Pos2::new(x, 100.0);
            ui.painter().circle_filled(pos, 10.0, Color32::BLUE);
        });
        
        // Request repaint for smooth animation
        ctx.request_repaint();
    }
}
```

---

## Best Practices

### 1. State Management

✅ **DO**: Store widget state in your app struct
```rust
struct App {
    picker: ColorPicker,  // Stores its own color
}
```

❌ **DON'T**: Try to pass mutable references to widgets
```rust
// This won't work - ColorPicker doesn't take &mut Color32
ColorPicker::new(&mut self.color)  // ❌ Wrong!
```

### 2. Performance

✅ **DO**: Use `ctx.request_repaint()` for animations
```rust
if animation_playing {
    ctx.request_repaint();
}
```

❌ **DON'T**: Call `request_repaint()` unconditionally
```rust
ctx.request_repaint();  // ❌ Wastes CPU when idle
```

### 3. Charts

✅ **DO**: Use `(f64, f64)` tuples for points
```rust
let points = vec![(0.0, 1.0), (1.0, 2.0)];
chart.add_series("Data", points, Color32::BLUE);
```

❌ **DON'T**: Create custom point structs
```rust
struct Point { x: f64, y: f64 }  // ❌ Not compatible
```

### 4. Namespacing

✅ **DO**: Import from `astract::prelude::egui::*`
```rust
use astract::prelude::egui::*;
use astract::charts::LineChart;
```

❌ **DON'T**: Mix direct egui imports
```rust
use egui::*;  // ❌ May conflict with astract::prelude
```

---

## Troubleshooting

### Build Errors

**Problem**: `error: cannot find type Ui`
```
Solution: Add `use astract::prelude::egui::*;`
```

**Problem**: `error: trait Widget is not implemented for ColorPicker`
```
Solution: Use .show() method, not ui.add():
    picker.show(ui);  // ✅ Correct
    ui.add(picker);   // ❌ Wrong
```

**Problem**: `error: expected Point, found tuple`
```
Solution: Point IS a tuple - use (f64, f64):
    vec![(0.0, 1.0), (1.0, 2.0)]  // ✅ Correct
    vec![Point { x: 0.0, y: 1.0 }]  // ❌ Wrong
```

### Runtime Issues

**Problem**: Widget doesn't respond to input
```
Solution: Ensure widget is stored in app state, not created every frame:
    struct App { picker: ColorPicker }  // ✅ Correct
    
    fn show(&mut self, ui: &mut Ui) {
        let mut picker = ColorPicker::new();  // ❌ Wrong - recreated every frame!
    }
```

**Problem**: Animation doesn't smooth
```
Solution: Call ctx.request_repaint() every frame:
    tween.update(dt);
    ctx.request_repaint();  // ✅ Request next frame
```

---

## Next Steps

Now that you've got the basics, explore:

1. **[Charts Tutorial](./CHARTS_TUTORIAL.md)** - LineChart, BarChart, ScatterPlot
2. **[Advanced Widgets Tutorial](./ADVANCED_WIDGETS_TUTORIAL.md)** - ColorPicker, TreeView, RangeSlider
3. **[NodeGraph Tutorial](./NODEGRAPH_TUTORIAL.md)** - Behavior trees, shader graphs
4. **[Animation Tutorial](./ANIMATION_TUTORIAL.md)** - Tweens, springs, easing
5. **[API Reference](./API_REFERENCE.md)** - Complete API documentation
6. **[Gallery Example](../examples/astract_gallery/)** - See all widgets in action

### Community & Support

- **GitHub**: [AstraWeave Repository](https://github.com/lazyxeon/AstraWeave-AI-Native-Gaming-Engine)
- **Issues**: Report bugs or request features
- **Examples**: Check `examples/` directory for more code

### Contributing

Astract is part of the AstraWeave AI-Native Game Engine project. Contributions are welcome!

---

**Happy coding with Astract! 🎨**
