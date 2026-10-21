# Advanced Widgets Tutorial

Master Astract's advanced interactive widgets for professional UIs.

---

## Table of Contents

1. [Overview](#overview)
2. [ColorPicker](#colorpicker)
3. [TreeView](#treeview)
4. [RangeSlider](#rangeslider)
5. [Stateful Widget Pattern](#stateful-widget-pattern)
6. [Real-World Examples](#real-world-examples)

---

## Overview

Astract's advanced widgets provide rich interactive experiences:

- **ColorPicker** - RGBA color selection with preview
- **TreeView** - Hierarchical data visualization
- **RangeSlider** - Dual-handle range selection

### Key Concept: Stateful Widgets

Unlike basic egui widgets, Astract's advanced widgets are **stateful**:

```rust
// ❌ WRONG: Don't create widgets every frame
fn show(ui: &mut Ui) {
    let picker = ColorPicker::new();  // ❌ Loses state!
    picker.show(ui);
}

// ✅ CORRECT: Store widgets in app state
struct App {
    picker: ColorPicker,  // ✅ Persistent state
}

impl App {
    fn show(&mut self, ui: &mut Ui) {
        self.picker.show(ui);  // ✅ Preserves state
    }
}
```

**Why Stateful?**
- Widgets remember selections (color, tree expansion, slider values)
- Interactions persist across frames
- Better performance (no recreation)

---

## ColorPicker

Professional color selection with alpha channel support.

### Basic Example

```rust
use astract::prelude::egui::*;
use astract::advanced::ColorPicker;

struct MyApp {
    picker: ColorPicker,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            picker: ColorPicker::new()
                .with_color(Color32::RED)
                .show_alpha(true),
        }
    }
}

impl MyApp {
    fn show(&mut self, ui: &mut Ui) {
        ui.heading("Color Picker");
        
        // Show the picker
        self.picker.show(ui);
        
        // Get the selected color
        let color = self.picker.color();
        
        // Use the color
        ui.label(format!("Selected: {:?}", color));
    }
}
```

### With Preview

Show a large preview of the selected color:

```rust
fn show_with_preview(&mut self, ui: &mut Ui) {
    ui.heading("Color Picker with Preview");
    
    // Color picker
    self.picker.show(ui);
    
    ui.add_space(10.0);
    
    // Large preview square
    let color = self.picker.color();
    let (response, painter) = ui.allocate_painter(
        Vec2::new(200.0, 100.0),
        Sense::hover()
    );
    painter.rect_filled(
        response.rect,
        0.0,
        color,
    );
    
    // Color info
    ui.label(format!(
        "RGB: ({}, {}, {})",
        color.r(), color.g(), color.b()
    ));
    ui.label(format!("Alpha: {}", color.a()));
}
```

### Themed Color Picker

Create a color picker for specific themes:

```rust
struct ThemeEditor {
    background_picker: ColorPicker,
    foreground_picker: ColorPicker,
    accent_picker: ColorPicker,
}

impl Default for ThemeEditor {
    fn default() -> Self {
        Self {
            background_picker: ColorPicker::new()
                .with_color(Color32::from_rgb(30, 30, 30))
                .show_alpha(false),
            foreground_picker: ColorPicker::new()
                .with_color(Color32::WHITE)
                .show_alpha(false),
            accent_picker: ColorPicker::new()
                .with_color(Color32::from_rgb(0, 120, 215))
                .show_alpha(false),
        }
    }
}

impl ThemeEditor {
    fn show(&mut self, ui: &mut Ui) {
        ui.heading("Theme Editor");
        
        ui.label("Background Color:");
        self.background_picker.show(ui);
        ui.add_space(10.0);
        
        ui.label("Foreground Color:");
        self.foreground_picker.show(ui);
        ui.add_space(10.0);
        
        ui.label("Accent Color:");
        self.accent_picker.show(ui);
        ui.add_space(10.0);
        
        // Preview theme
        let bg = self.background_picker.color();
        let fg = self.foreground_picker.color();
        let accent = self.accent_picker.color();
        
        ui.group(|ui| {
            ui.visuals_mut().override_text_color = Some(fg);
            ui.visuals_mut().widgets.inactive.bg_fill = bg;
            ui.label("This is how your theme looks!");
            if ui.button("Accent Button").clicked() {
                // ...
            }
        });
    }
}
```

---

## TreeView

Display hierarchical data with expand/collapse functionality.

### Basic Example

```rust
use astract::advanced::{TreeView, TreeNode};

struct FileExplorer {
    tree: TreeView,
}

impl FileExplorer {
    fn new() -> Self {
        let mut tree = TreeView::new();
        
        // Create root folder
        let root = tree.add_node(
            TreeNode::new(1, "Project")
                .with_icon("📁")
        );
        
        // Add children
        tree.add_child(
            root,
            TreeNode::new(2, "src")
                .with_icon("📁")
        );
        tree.add_child(
            root,
            TreeNode::new(3, "tests")
                .with_icon("📁")
        );
        tree.add_child(
            root,
            TreeNode::new(4, "Cargo.toml")
                .with_icon("📄")
        );
        
        Self { tree }
    }
    
    fn show(&mut self, ui: &mut Ui) {
        ui.heading("File Explorer");
        
        if let Some(clicked_id) = self.tree.show(ui) {
            ui.label(format!("Clicked node: {}", clicked_id));
        }
    }
}
```

### Nested Hierarchy

Create deeply nested structures:

```rust
fn create_project_tree() -> TreeView {
    let mut tree = TreeView::new();
    
    // Root
    let root = tree.add_node(
        TreeNode::new(1, "my_project").with_icon("📁")
    );
    
    // src/ folder
    let src = TreeNode::new(2, "src").with_icon("📁");
    let src_id = tree.add_child(root, src);
    
    // src/ files
    tree.add_child(src_id, TreeNode::new(3, "main.rs").with_icon("📄"));
    tree.add_child(src_id, TreeNode::new(4, "lib.rs").with_icon("📄"));
    
    // src/ui/ folder
    let ui_folder = TreeNode::new(5, "ui").with_icon("📁");
    let ui_id = tree.add_child(src_id, ui_folder);
    
    // src/ui/ files
    tree.add_child(ui_id, TreeNode::new(6, "mod.rs").with_icon("📄"));
    tree.add_child(ui_id, TreeNode::new(7, "widgets.rs").with_icon("📄"));
    
    // tests/ folder
    let tests = TreeNode::new(8, "tests").with_icon("📁");
    let tests_id = tree.add_child(root, tests);
    tree.add_child(tests_id, TreeNode::new(9, "integration.rs").with_icon("📄"));
    
    // Config files
    tree.add_child(root, TreeNode::new(10, "Cargo.toml").with_icon("⚙️"));
    tree.add_child(root, TreeNode::new(11, "README.md").with_icon("📖"));
    
    tree
}
```

### Interactive Selection

Handle node clicks and track selection:

```rust
struct InteractiveTree {
    tree: TreeView,
    selected_id: Option<usize>,
    selected_name: String,
}

impl InteractiveTree {
    fn show(&mut self, ui: &mut Ui) {
        ui.heading("Interactive Tree");
        
        // Show current selection
        if self.selected_id.is_some() {
            ui.label(format!("Selected: {}", self.selected_name));
            ui.add_space(5.0);
        }
        
        // Show tree and handle clicks
        if let Some(clicked_id) = self.tree.show(ui) {
            self.selected_id = Some(clicked_id);
            self.selected_name = format!("Node {}", clicked_id);
        }
        
        ui.add_space(10.0);
        
        // Actions based on selection
        if self.selected_id.is_some() {
            ui.horizontal(|ui| {
                if ui.button("✏️ Rename").clicked() {
                    // Rename logic
                }
                if ui.button("🗑️ Delete").clicked() {
                    // Delete logic
                }
            });
        }
    }
}
```

---

## RangeSlider

Select a range with dual handles.

### Basic Example

```rust
use astract::advanced::RangeSlider;

struct RangeFilter {
    range: RangeSlider,
}

impl Default for RangeFilter {
    fn default() -> Self {
        Self {
            range: RangeSlider::new(0.0, 100.0)
                .with_min(25.0)
                .with_max(75.0),
        }
    }
}

impl RangeFilter {
    fn show(&mut self, ui: &mut Ui) {
        ui.heading("Range Filter");
        
        // Show the slider
        self.range.show(ui);
        
        // Get current values
        let min = self.range.min_value();
        let max = self.range.max_value();
        
        ui.label(format!("Range: {:.1} - {:.1}", min, max));
    }
}
```

### Price Range Filter

Common e-commerce use case:

```rust
struct PriceFilter {
    price_range: RangeSlider,
}

impl Default for PriceFilter {
    fn default() -> Self {
        Self {
            price_range: RangeSlider::new(0.0, 1000.0)
                .with_min(100.0)
                .with_max(500.0),
        }
    }
}

impl PriceFilter {
    fn show(&mut self, ui: &mut Ui) {
        ui.heading("Price Filter");
        
        ui.label("Select price range:");
        self.price_range.show(ui);
        
        let min = self.price_range.min_value();
        let max = self.price_range.max_value();
        
        ui.label(format!("${:.0} - ${:.0}", min, max));
        
        // Show filtered count (example)
        let count = self.count_items_in_range(min, max);
        ui.label(format!("{} items match", count));
    }
    
    fn count_items_in_range(&self, min: f64, max: f64) -> usize {
        // Your filtering logic here
        42
    }
}
```

### With Visual Feedback

Show a visualization of the selected range:

```rust
fn show_with_visualization(&mut self, ui: &mut Ui) {
    ui.heading("Range Slider with Visualization");
    
    // Slider
    self.range.show(ui);
    
    ui.add_space(10.0);
    
    // Visual representation
    let min = self.range.min_value();
    let max = self.range.max_value();
    let range_min = 0.0;
    let range_max = 100.0;
    
    let (response, painter) = ui.allocate_painter(
        Vec2::new(ui.available_width(), 40.0),
        Sense::hover()
    );
    
    let rect = response.rect;
    
    // Background (unselected)
    painter.rect_filled(rect, 2.0, Color32::from_gray(50));
    
    // Selected range
    let min_x = rect.min.x + (min - range_min) / (range_max - range_min) * rect.width();
    let max_x = rect.min.x + (max - range_min) / (range_max - range_min) * rect.width();
    
    let selected_rect = Rect::from_min_max(
        Pos2::new(min_x, rect.min.y),
        Pos2::new(max_x, rect.max.y),
    );
    painter.rect_filled(selected_rect, 2.0, Color32::from_rgb(70, 130, 180));
    
    // Labels
    painter.text(
        Pos2::new(min_x, rect.center().y),
        Align2::CENTER_CENTER,
        format!("{:.0}", min),
        FontId::default(),
        Color32::WHITE,
    );
    painter.text(
        Pos2::new(max_x, rect.center().y),
        Align2::CENTER_CENTER,
        format!("{:.0}", max),
        FontId::default(),
        Color32::WHITE,
    );
}
```

---

## Stateful Widget Pattern

Understanding how Astract widgets work internally.

### The Pattern

```rust
// Widget stores its state
pub struct ColorPicker {
    color: Color32,
    show_alpha: bool,
    // ... internal state
}

impl ColorPicker {
    // Builder pattern for configuration
    pub fn new() -> Self { /* ... */ }
    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }
    pub fn show_alpha(mut self, show: bool) -> Self {
        self.show_alpha = show;
        self
    }
    
    // Display method (mutates state)
    pub fn show(&mut self, ui: &mut Ui) {
        // Update internal state based on user input
        // ...
    }
    
    // Getters for state
    pub fn color(&self) -> Color32 {
        self.color
    }
}
```

### Using the Pattern

```rust
struct App {
    // ✅ Store widget in app state
    picker: ColorPicker,
}

impl Default for App {
    fn default() -> Self {
        Self {
            // ✅ Configure once during initialization
            picker: ColorPicker::new()
                .with_color(Color32::BLUE)
                .show_alpha(true),
        }
    }
}

impl App {
    fn show(&mut self, ui: &mut Ui) {
        // ✅ Call .show() to display and handle interactions
        self.picker.show(ui);
        
        // ✅ Access state via getters
        let color = self.picker.color();
        
        // ✅ Widget state persists between frames
    }
}
```

### Why Not ui.add()?

```rust
// egui's built-in widgets use Widget trait
ui.add(egui::Slider::new(&mut value, 0.0..=1.0));  // ✅ Works

// Astract widgets DON'T implement Widget
ui.add(ColorPicker::new());  // ❌ Compile error!

// Instead, use .show() method
picker.show(ui);  // ✅ Correct
```

**Reason**: Stateful widgets need to persist across frames. The `Widget` trait is designed for **stateless** widgets that recreate every frame.

---

## Real-World Examples

### Complete Theme Editor

```rust
struct ThemeEditorApp {
    background_picker: ColorPicker,
    text_picker: ColorPicker,
    primary_picker: ColorPicker,
    secondary_picker: ColorPicker,
    theme_name: String,
}

impl Default for ThemeEditorApp {
    fn default() -> Self {
        Self {
            background_picker: ColorPicker::new()
                .with_color(Color32::from_rgb(18, 18, 18))
                .show_alpha(false),
            text_picker: ColorPicker::new()
                .with_color(Color32::WHITE)
                .show_alpha(false),
            primary_picker: ColorPicker::new()
                .with_color(Color32::from_rgb(0, 120, 215))
                .show_alpha(false),
            secondary_picker: ColorPicker::new()
                .with_color(Color32::from_rgb(104, 33, 122))
                .show_alpha(false),
            theme_name: "My Theme".to_string(),
        }
    }
}

impl ThemeEditorApp {
    fn show(&mut self, ui: &mut Ui) {
        ui.heading("Theme Editor");
        
        // Theme name
        ui.horizontal(|ui| {
            ui.label("Theme Name:");
            ui.text_edit_singleline(&mut self.theme_name);
        });
        
        ui.separator();
        
        // Color pickers in a grid
        ui.columns(2, |columns| {
            columns[0].label("Background:");
            columns[0].add_space(5.0);
            self.background_picker.show(&mut columns[0]);
            
            columns[1].label("Text:");
            columns[1].add_space(5.0);
            self.text_picker.show(&mut columns[1]);
        });
        
        ui.add_space(10.0);
        
        ui.columns(2, |columns| {
            columns[0].label("Primary:");
            columns[0].add_space(5.0);
            self.primary_picker.show(&mut columns[0]);
            
            columns[1].label("Secondary:");
            columns[1].add_space(5.0);
            self.secondary_picker.show(&mut columns[1]);
        });
        
        ui.separator();
        
        // Preview
        self.show_preview(ui);
        
        // Export button
        if ui.button("💾 Export Theme").clicked() {
            self.export_theme();
        }
    }
    
    fn show_preview(&self, ui: &mut Ui) {
        ui.heading("Preview");
        
        let bg = self.background_picker.color();
        let text = self.text_picker.color();
        let primary = self.primary_picker.color();
        
        Frame::none()
            .fill(bg)
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.visuals_mut().override_text_color = Some(text);
                ui.label("This is sample text in your theme");
                
                let button = Button::new("Primary Button")
                    .fill(primary);
                ui.add(button);
            });
    }
    
    fn export_theme(&self) {
        // Export theme to file or clipboard
        println!("Exporting theme: {}", self.theme_name);
    }
}
```

### File System Browser

```rust
struct FileBrowser {
    tree: TreeView,
    current_path: String,
    file_content: String,
}

impl FileBrowser {
    fn new() -> Self {
        let mut tree = TreeView::new();
        
        // Build file tree (simplified)
        let root = tree.add_node(
            TreeNode::new(1, "C:\\").with_icon("💻")
        );
        
        let users = TreeNode::new(2, "Users").with_icon("📁");
        let users_id = tree.add_child(root, users);
        
        let me = TreeNode::new(3, "Me").with_icon("👤");
        let me_id = tree.add_child(users_id, me);
        
        tree.add_child(me_id, TreeNode::new(4, "Documents").with_icon("📁"));
        tree.add_child(me_id, TreeNode::new(5, "Downloads").with_icon("📁"));
        tree.add_child(me_id, TreeNode::new(6, "Desktop").with_icon("📁"));
        
        Self {
            tree,
            current_path: String::new(),
            file_content: String::new(),
        }
    }
    
    fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Left: File tree
            ui.vertical(|ui| {
                ui.heading("Files");
                ui.set_width(200.0);
                
                if let Some(clicked_id) = self.tree.show(ui) {
                    self.load_file(clicked_id);
                }
            });
            
            ui.separator();
            
            // Right: File content
            ui.vertical(|ui| {
                ui.heading(&self.current_path);
                ui.add_space(5.0);
                
                ScrollArea::vertical()
                    .show(ui, |ui| {
                        ui.label(&self.file_content);
                    });
            });
        });
    }
    
    fn load_file(&mut self, file_id: usize) {
        self.current_path = format!("File {}", file_id);
        self.file_content = format!("Content of file {}", file_id);
    }
}
```

### Data Analysis Dashboard

```rust
struct DataFilterPanel {
    date_range: RangeSlider,
    price_range: RangeSlider,
    category_tree: TreeView,
}

impl Default for DataFilterPanel {
    fn default() -> Self {
        let mut tree = TreeView::new();
        let root = tree.add_node(TreeNode::new(1, "All Categories").with_icon("📊"));
        tree.add_child(root, TreeNode::new(2, "Electronics").with_icon("⚡"));
        tree.add_child(root, TreeNode::new(3, "Clothing").with_icon("👕"));
        tree.add_child(root, TreeNode::new(4, "Food").with_icon("🍔"));
        
        Self {
            date_range: RangeSlider::new(0.0, 365.0)
                .with_min(0.0)
                .with_max(90.0),
            price_range: RangeSlider::new(0.0, 10000.0)
                .with_min(100.0)
                .with_max(5000.0),
            category_tree: tree,
        }
    }
}

impl DataFilterPanel {
    fn show(&mut self, ui: &mut Ui) {
        ui.heading("Filters");
        
        // Date range
        ui.label("Date Range (days):");
        self.date_range.show(ui);
        let days_min = self.date_range.min_value() as i32;
        let days_max = self.date_range.max_value() as i32;
        ui.label(format!("{} - {} days ago", days_min, days_max));
        
        ui.add_space(10.0);
        
        // Price range
        ui.label("Price Range:");
        self.price_range.show(ui);
        let price_min = self.price_range.min_value();
        let price_max = self.price_range.max_value();
        ui.label(format!("${:.0} - ${:.0}", price_min, price_max));
        
        ui.add_space(10.0);
        
        // Category filter
        ui.label("Categories:");
        if let Some(category_id) = self.category_tree.show(ui) {
            println!("Selected category: {}", category_id);
        }
        
        ui.add_space(10.0);
        
        // Apply button
        if ui.button("🔍 Apply Filters").clicked() {
            self.apply_filters();
        }
    }
    
    fn apply_filters(&self) {
        println!("Applying filters...");
        // Filter logic here
    }
}
```

---

## Best Practices

### 1. Widget Lifecycle

✅ **DO**: Initialize widgets once
```rust
struct App {
    picker: ColorPicker,  // ✅ Created in Default::default()
}
```

❌ **DON'T**: Create widgets every frame
```rust
fn show(ui: &mut Ui) {
    let picker = ColorPicker::new();  // ❌ Recreated every frame!
}
```

### 2. State Access

✅ **DO**: Use getters after .show()
```rust
self.picker.show(ui);
let color = self.picker.color();  // ✅ Get updated state
```

❌ **DON'T**: Access before .show()
```rust
let color = self.picker.color();  // ❌ May be stale
self.picker.show(ui);
```

### 3. Configuration

✅ **DO**: Use builder pattern during initialization
```rust
ColorPicker::new()
    .with_color(Color32::RED)
    .show_alpha(true)
```

❌ **DON'T**: Try to reconfigure after creation
```rust
let mut picker = ColorPicker::new();
picker.set_color(Color32::RED);  // ❌ No such method!
```

---

## Next Steps

- **[NodeGraph Tutorial](./NODEGRAPH_TUTORIAL.md)** - Visual node editors
- **[Animation Tutorial](./ANIMATION_TUTORIAL.md)** - Smooth transitions
- **[Gallery Example](../../examples/astract_gallery/)** - See widgets in action

---

**Master advanced widgets! 🎨**
