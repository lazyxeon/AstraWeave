# Animation Tutorial

Create smooth, physics-based animations with Astract's animation system.

---

## Table of Contents

1. [Overview](#overview)
2. [Tween Animations](#tween-animations)
3. [Spring Physics](#spring-physics)
4. [Easing Functions](#easing-functions)
5. [Animation Controller](#animation-controller)
6. [Real-World Examples](#real-world-examples)

---

## Overview

Astract provides professional animation tools:

- **Tween** - Value interpolation with 15 easing functions
- **Spring** - Physics-based animations with natural motion
- **EasingFunction** - Mathematical curves for smooth transitions
- **AnimationController** - Manage multiple animations

### Core Concepts

```rust
use astract::animation::{
    Tween,              // Generic interpolation
    Spring,             // Physics simulation
    SpringParams,       // Spring configuration presets
    EasingFunction,     // 15 easing curves
    AnimationController,// Multi-animation manager
    AnimationState,     // Playing, Paused, Stopped
};
```

---

## Tween Animations

Interpolate values over time with easing functions.

### Basic Tween

```rust
use astract::animation::{Tween, EasingFunction};

struct MyApp {
    position_tween: Tween<f32>,
    time: f32,
}

impl Default for MyApp {
    fn default() -> Self {
        // Create tween: from → to, duration
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
        
        // Update tween
        self.position_tween.update(dt);
        
        // Get current value
        let x = self.position_tween.value();
        
        // Use animated value
        CentralPanel::default().show(ctx, |ui| {
            let painter = ui.painter();
            let pos = Pos2::new(x, 100.0);
            painter.circle_filled(pos, 10.0, Color32::BLUE);
        });
        
        // Request next frame
        ctx.request_repaint();
    }
}
```

### Controlling Playback

```rust
// Play/pause/stop controls
if tween.state() == AnimationState::Playing {
    ui.label("Playing...");
    if ui.button("⏸ Pause").clicked() {
        tween.pause();
    }
}

if tween.state() == AnimationState::Paused {
    ui.label("Paused");
    if ui.button("▶ Resume").clicked() {
        tween.play();
    }
}

// Reset to beginning
if ui.button("⏮ Reset").clicked() {
    tween.reset();
    tween.play();
}

// Check if finished
if tween.is_finished() {
    ui.label("✅ Animation complete!");
}
```

### Looping Animations

```rust
struct LoopingAnimation {
    bounce_tween: Tween<f32>,
}

impl LoopingAnimation {
    fn new() -> Self {
        let mut tween = Tween::new(0.0, 100.0, 1.0)
            .with_easing(EasingFunction::ElasticOut);
        tween.play();
        
        Self { bounce_tween: tween }
    }
    
    fn update(&mut self, dt: f32) {
        self.bounce_tween.update(dt);
        
        // Auto-restart when finished
        if self.bounce_tween.is_finished() {
            self.bounce_tween.reset();
            self.bounce_tween.play();
        }
    }
}
```

### Color Tweening

```rust
use astract::animation::Tween;
use egui::Color32;

// Tween can interpolate any type that implements Linear trait
struct ColorAnimation {
    color_tween: Tween<Color32>,
}

impl ColorAnimation {
    fn new() -> Self {
        let mut tween = Tween::new(
            Color32::RED,
            Color32::BLUE,
            3.0,  // 3 seconds
        ).with_easing(EasingFunction::SineInOut);
        tween.play();
        
        Self { color_tween: tween }
    }
    
    fn show(&mut self, ui: &mut Ui, dt: f32) {
        self.color_tween.update(dt);
        
        let color = self.color_tween.value();
        
        // Draw colored rectangle
        let (response, painter) = ui.allocate_painter(
            Vec2::new(200.0, 100.0),
            Sense::hover(),
        );
        painter.rect_filled(response.rect, 0.0, color);
        
        // Loop
        if self.color_tween.is_finished() {
            self.color_tween.reset();
            self.color_tween.play();
        }
    }
}
```

---

## Spring Physics

Physics-based animations with natural motion.

### Basic Spring

```rust
use astract::animation::{Spring, SpringParams};

struct SpringDemo {
    spring: Spring,
    target: f32,
}

impl Default for SpringDemo {
    fn default() -> Self {
        Self {
            spring: Spring::with_params(
                0.5,  // Initial position
                SpringParams::smooth(),  // Preset
            ),
            target: 0.5,
        }
    }
}

impl SpringDemo {
    fn show(&mut self, ui: &mut Ui, dt: f32) {
        // Update target with slider
        ui.horizontal(|ui| {
            ui.label("Target:");
            if ui.add(Slider::new(&mut self.target, 0.0..=1.0)).changed() {
                self.spring.set_target(self.target);
            }
        });
        
        // Update physics
        self.spring.update(dt);
        
        // Get current position
        let pos = self.spring.position();
        let velocity = self.spring.velocity();
        
        // Display info
        ui.label(format!("Position: {:.3}", pos));
        ui.label(format!("Velocity: {:.3}", velocity));
        
        // Visual indicator
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), 50.0),
            Sense::hover(),
        );
        
        let x = response.rect.min.x + pos * response.rect.width();
        let y = response.rect.center().y;
        painter.circle_filled(
            Pos2::new(x, y),
            10.0,
            Color32::BLUE,
        );
    }
}
```

### Spring Presets

```rust
// Smooth spring (gentle, no overshoot)
let spring = Spring::with_params(0.0, SpringParams::smooth());
// stiffness: 100.0, damping: 1.0 (critically damped)

// Bouncy spring (overshoot and oscillation)
let spring = Spring::with_params(0.0, SpringParams::bouncy());
// stiffness: 300.0, damping: 0.5 (underdamped)

// Stiff spring (fast response)
let spring = Spring::with_params(0.0, SpringParams::stiff());
// stiffness: 500.0, damping: 1.5 (slightly overdamped)

// Custom parameters
let spring = Spring::with_params(
    0.0,
    SpringParams {
        stiffness: 200.0,  // Higher = faster response
        damping: 0.8,       // 0.0 = oscillate forever
                           // 1.0 = critically damped
                           // >1.0 = overdamped (slow)
    },
);
```

### Interactive Spring

```rust
struct InteractiveSpring {
    x_spring: Spring,
    y_spring: Spring,
    mouse_pos: Pos2,
}

impl InteractiveSpring {
    fn new() -> Self {
        Self {
            x_spring: Spring::with_params(0.5, SpringParams::smooth()),
            y_spring: Spring::with_params(0.5, SpringParams::smooth()),
            mouse_pos: Pos2::ZERO,
        }
    }
    
    fn show(&mut self, ui: &mut Ui, dt: f32) {
        // Get mouse position (normalized 0-1)
        if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
            let rect = ui.available_rect_before_wrap();
            let norm_x = (pos.x - rect.min.x) / rect.width();
            let norm_y = (pos.y - rect.min.y) / rect.height();
            
            self.x_spring.set_target(norm_x as f64);
            self.y_spring.set_target(norm_y as f64);
        }
        
        // Update springs
        self.x_spring.update(dt);
        self.y_spring.update(dt);
        
        // Draw follower circle
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), 300.0),
            Sense::hover(),
        );
        
        let rect = response.rect;
        let x = rect.min.x + self.x_spring.position() as f32 * rect.width();
        let y = rect.min.y + self.y_spring.position() as f32 * rect.height();
        
        painter.circle_filled(
            Pos2::new(x, y),
            15.0,
            Color32::from_rgb(70, 130, 180),
        );
        
        ui.label("Move mouse to see spring follow!");
    }
}
```

---

## Easing Functions

15 mathematical curves for smooth motion.

### Available Functions

```rust
pub enum EasingFunction {
    Linear,           // Constant speed
    SineIn,           // Slow start
    SineOut,          // Slow end
    SineInOut,        // Slow start & end
    QuadIn,           // Accelerate (x²)
    QuadOut,          // Decelerate (x²)
    QuadInOut,        // Accel then decel (x²)
    CubicIn,          // Strong accel (x³)
    CubicOut,         // Strong decel (x³)
    CubicInOut,       // Strong accel/decel (x³)
    ElasticIn,        // Elastic pull-back
    ElasticOut,       // Elastic overshoot
    BounceOut,        // Bouncing ball landing
    BackIn,           // Pull back before motion
    BackOut,          // Overshoot then settle
}
```

### Visual Comparison

```rust
struct EasingComparison {
    tweens: Vec<(String, Tween<f32>)>,
    time: f32,
}

impl EasingComparison {
    fn new() -> Self {
        let easings = vec![
            ("Linear", EasingFunction::Linear),
            ("SineIn", EasingFunction::SineIn),
            ("SineOut", EasingFunction::SineOut),
            ("QuadIn", EasingFunction::QuadIn),
            ("QuadOut", EasingFunction::QuadOut),
            ("ElasticOut", EasingFunction::ElasticOut),
            ("BounceOut", EasingFunction::BounceOut),
            ("BackOut", EasingFunction::BackOut),
        ];
        
        let tweens = easings
            .into_iter()
            .map(|(name, easing)| {
                let mut tween = Tween::new(0.0, 200.0, 2.0)
                    .with_easing(easing);
                tween.play();
                (name.to_string(), tween)
            })
            .collect();
        
        Self { tweens, time: 0.0 }
    }
    
    fn show(&mut self, ui: &mut Ui, dt: f32) {
        ui.heading("Easing Function Comparison");
        
        // Update all tweens
        for (_, tween) in &mut self.tweens {
            tween.update(dt);
        }
        
        // Draw side-by-side
        for (name, tween) in &self.tweens {
            ui.horizontal(|ui| {
                ui.label(format!("{:12}", name));
                
                let x = tween.value();
                let (response, painter) = ui.allocate_painter(
                    Vec2::new(220.0, 30.0),
                    Sense::hover(),
                );
                
                let rect = response.rect;
                painter.rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(1.0, Color32::GRAY),
                );
                
                let circle_x = rect.min.x + x;
                let circle_y = rect.center().y;
                painter.circle_filled(
                    Pos2::new(circle_x, circle_y),
                    8.0,
                    Color32::BLUE,
                );
            });
        }
        
        // Restart button
        if ui.button("🔄 Restart All").clicked() {
            for (_, tween) in &mut self.tweens {
                tween.reset();
                tween.play();
            }
        }
    }
}
```

---

## Animation Controller

Manage multiple animations simultaneously.

### Basic Usage

```rust
use astract::animation::AnimationController;

struct MultiAnimation {
    controller: AnimationController,
}

impl MultiAnimation {
    fn new() -> Self {
        let mut controller = AnimationController::new();
        
        // Add animations with string IDs
        controller.add_animation(
            "position",
            Tween::new(0.0, 100.0, 2.0)
                .with_easing(EasingFunction::SineInOut),
        );
        
        controller.add_animation(
            "rotation",
            Tween::new(0.0f32, 360.0f32, 3.0)
                .with_easing(EasingFunction::Linear),
        );
        
        controller.add_animation(
            "scale",
            Tween::new(1.0, 2.0, 1.5)
                .with_easing(EasingFunction::ElasticOut),
        );
        
        // Start all
        controller.play_all();
        
        Self { controller }
    }
    
    fn update(&mut self, dt: f32) {
        self.controller.update_all(dt);
    }
    
    fn show(&self, ui: &mut Ui) {
        // Get animated values
        let pos = self.controller.get_value::<f32>("position").unwrap();
        let rot = self.controller.get_value::<f32>("rotation").unwrap();
        let scale = self.controller.get_value::<f32>("scale").unwrap();
        
        ui.label(format!("Position: {:.1}", pos));
        ui.label(format!("Rotation: {:.1}°", rot));
        ui.label(format!("Scale: {:.2}×", scale));
    }
}
```

### Sequencing Animations

```rust
struct SequencedAnimation {
    controller: AnimationController,
    current_step: usize,
}

impl SequencedAnimation {
    fn new() -> Self {
        let mut controller = AnimationController::new();
        
        // Add animations (don't start yet)
        controller.add_animation(
            "step1",
            Tween::new(0.0, 100.0, 1.0),
        );
        controller.add_animation(
            "step2",
            Tween::new(100.0, 200.0, 1.0),
        );
        controller.add_animation(
            "step3",
            Tween::new(200.0, 0.0, 1.0),
        );
        
        // Start first step
        controller.play("step1");
        
        Self {
            controller,
            current_step: 1,
        }
    }
    
    fn update(&mut self, dt: f32) {
        self.controller.update_all(dt);
        
        // Check for step completion
        match self.current_step {
            1 if self.controller.is_finished("step1") => {
                self.controller.play("step2");
                self.current_step = 2;
            }
            2 if self.controller.is_finished("step2") => {
                self.controller.play("step3");
                self.current_step = 3;
            }
            3 if self.controller.is_finished("step3") => {
                // Sequence complete!
                self.current_step = 0;
            }
            _ => {}
        }
    }
}
```

---

## Real-World Examples

### Loading Screen Animation

```rust
struct LoadingScreen {
    spinner_tween: Tween<f32>,
    progress_tween: Tween<f32>,
    pulse_tween: Tween<f32>,
}

impl LoadingScreen {
    fn new() -> Self {
        let mut spinner = Tween::new(0.0, 360.0, 2.0)
            .with_easing(EasingFunction::Linear);
        spinner.play();
        
        let mut progress = Tween::new(0.0, 100.0, 5.0)
            .with_easing(EasingFunction::SineInOut);
        progress.play();
        
        let mut pulse = Tween::new(0.8, 1.2, 0.8)
            .with_easing(EasingFunction::SineInOut);
        pulse.play();
        
        Self {
            spinner_tween: spinner,
            progress_tween: progress,
            pulse_tween: pulse,
        }
    }
    
    fn show(&mut self, ui: &mut Ui, dt: f32) {
        // Update animations
        self.spinner_tween.update(dt);
        self.progress_tween.update(dt);
        self.pulse_tween.update(dt);
        
        // Loop spinner and pulse
        if self.spinner_tween.is_finished() {
            self.spinner_tween.reset();
            self.spinner_tween.play();
        }
        if self.pulse_tween.is_finished() {
            self.pulse_tween.reset();
            self.pulse_tween.play();
        }
        
        let rotation = self.spinner_tween.value();
        let progress = self.progress_tween.value();
        let pulse = self.pulse_tween.value();
        
        // Center content
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            
            // Spinning loader
            let (response, painter) = ui.allocate_painter(
                Vec2::new(50.0, 50.0),
                Sense::hover(),
            );
            
            let center = response.rect.center();
            let radius = 20.0 * pulse;
            
            painter.circle_stroke(
                center,
                radius,
                Stroke::new(3.0, Color32::BLUE),
            );
            
            // Rotating arc
            // (Simplified - actual implementation would use painter.arc)
            
            ui.add_space(20.0);
            
            // Progress bar
            ui.add(ProgressBar::new(progress / 100.0));
            
            ui.label(format!("Loading... {:.0}%", progress));
        });
    }
}
```

### Button Hover Animation

```rust
struct AnimatedButton {
    hover_spring: Spring,
    is_hovered: bool,
    label: String,
}

impl AnimatedButton {
    fn new(label: impl Into<String>) -> Self {
        Self {
            hover_spring: Spring::with_params(0.0, SpringParams::bouncy()),
            is_hovered: false,
            label: label.into(),
        }
    }
    
    fn show(&mut self, ui: &mut Ui, dt: f32) -> Response {
        // Update spring
        self.hover_spring.update(dt);
        
        // Target: 1.0 when hovered, 0.0 when not
        let target = if self.is_hovered { 1.0 } else { 0.0 };
        self.hover_spring.set_target(target);
        
        let hover_amount = self.hover_spring.position() as f32;
        
        // Calculate animated properties
        let base_height = 30.0;
        let hover_height = 35.0;
        let height = base_height + (hover_height - base_height) * hover_amount;
        
        let base_color = Color32::from_rgb(70, 130, 180);
        let hover_color = Color32::from_rgb(100, 160, 210);
        let color = Color32::from_rgb(
            (base_color.r() as f32 + (hover_color.r() as f32 - base_color.r() as f32) * hover_amount) as u8,
            (base_color.g() as f32 + (hover_color.g() as f32 - base_color.g() as f32) * hover_amount) as u8,
            (base_color.b() as f32 + (hover_color.b() as f32 - base_color.b() as f32) * hover_amount) as u8,
        );
        
        // Draw button
        let button = Button::new(&self.label)
            .fill(color)
            .min_size(Vec2::new(100.0, height));
        
        let response = ui.add(button);
        
        // Update hover state
        self.is_hovered = response.hovered();
        
        response
    }
}
```

### Notification Popup

```rust
struct Notification {
    slide_tween: Tween<f32>,
    fade_tween: Tween<f32>,
    message: String,
}

impl Notification {
    fn show(message: impl Into<String>) -> Self {
        let mut slide = Tween::new(-300.0, 0.0, 0.5)
            .with_easing(EasingFunction::BackOut);
        slide.play();
        
        let mut fade = Tween::new(0.0, 1.0, 0.5)
            .with_easing(EasingFunction::SineIn);
        fade.play();
        
        Self {
            slide_tween: slide,
            fade_tween: fade,
            message: message.into(),
        }
    }
    
    fn show_ui(&mut self, ctx: &Context, dt: f32) {
        // Update animations
        self.slide_tween.update(dt);
        self.fade_tween.update(dt);
        
        let x_offset = self.slide_tween.value();
        let alpha = self.fade_tween.value() as f32;
        
        // Top-right popup
        TopBottomPanel::top("notification")
            .show(ctx, |ui| {
                // Apply horizontal offset
                ui.add_space(x_offset.abs());
                
                // Semi-transparent background
                let bg_color = Color32::from_rgba_premultiplied(
                    50, 50, 50,
                    (200.0 * alpha) as u8,
                );
                
                Frame::none()
                    .fill(bg_color)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.colored_label(
                            Color32::WHITE,
                            &self.message,
                        );
                    });
            });
    }
}
```

---

## Best Practices

### 1. Delta Time

✅ **DO**: Use proper delta time
```rust
let dt = ctx.input(|i| i.stable_dt);
tween.update(dt);
```

❌ **DON'T**: Use fixed timestep
```rust
tween.update(0.016);  // ❌ Assumes 60 FPS
```

### 2. Request Repaint

✅ **DO**: Request repaint during animations
```rust
if tween.state() == AnimationState::Playing {
    ctx.request_repaint();
}
```

❌ **DON'T**: Always repaint
```rust
ctx.request_repaint();  // ❌ Wastes CPU when idle
```

### 3. Easing Selection

✅ **DO**: Match easing to use case
```rust
// UI elements
EasingFunction::SineInOut     // Smooth, professional
EasingFunction::QuadOut       // Snappy buttons

// Game objects
EasingFunction::ElasticOut    // Bouncy, playful
EasingFunction::BounceOut     // Falling objects

// Loading screens
EasingFunction::Linear        // Spinners
```

---

## Next Steps

- **[Gallery Example](../../examples/astract_gallery/)** - See all animations in action
- **[API Reference](./API_REFERENCE.md)** - Complete animation API
- **[Getting Started](./GETTING_STARTED.md)** - Integrate into your app

---

**Animate everything! 🎬**
