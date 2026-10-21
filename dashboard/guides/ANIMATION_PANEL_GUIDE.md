# Animation Panel User Guide

**AstraWeave Editor - Animation Tools**  
**Version:** 0.1.0  
**Last Updated:** November 18, 2025

---

## Overview

The Animation Panel provides professional-grade animation tools using the **astract animation library**. It includes:

- **Tween System** - Smooth interpolation for any property type
- **Spring Physics** - Realistic motion with bounce and damping
- **11 Easing Functions** - Industry-standard timing curves
- **Interactive Demos** - Real-time visualization and testing

**Status:** ✅ Functional (wired into the editor)

---

## Quick Start

### Opening the Panel

1. Launch AstraWeave Editor
2. Click **Window** → **Animation** (or press `F9` if mapped)
3. The Animation panel appears with three demonstration sections

### Panel Sections

```
┌─ Animation Panel ────────────────────┐
│                                      │
│ 🎬 Tween Demo (ElasticOut)          │
│    [Bouncing ball visualization]     │
│    [🔄 Restart Bounce]               │
│                                      │
│ 🎨 Color Tween (SineInOut)          │
│    [Color morphing bar]              │
│    [🔄 Restart Color]                │
│                                      │
│ 🎯 Spring Physics (Bouncy)          │
│    [Interactive mouse tracking]      │
│                                      │
│ ☑ Show Easing Comparison            │
│    [Easing function curves chart]    │
│    [🔄 Restart All Easings]          │
│                                      │
└──────────────────────────────────────┘
```

---

## Features

### 1. Tween System

**What is a Tween?**
Tweens (short for "in-betweens") smoothly interpolate between two values over time.

**Supported Types:**
- `f32` - Numeric values (position, rotation, scale)
- `Color32` - RGBA colors
- `Vec2`, `Vec3` - 2D/3D vectors (with component-wise interpolation)

**Example Usage:**
```rust
use astract::animation::{Tween, EasingFunction};

// Create position tween: 0.0 → 200.0 over 2 seconds
let mut tween = Tween::new(0.0f32, 200.0f32, 2.0)
    .with_easing(EasingFunction::ElasticOut);

tween.play(); // Start animation

// In update loop:
tween.update(delta_time);
let current_value = tween.value(); // Get interpolated value

// Check status:
if tween.is_complete() {
    println!("Animation finished!");
}

// Restart from beginning:
tween.restart();
```

**Controls:**
- Click **🔄 Restart Bounce** to replay the bouncing ball animation
- Click **🔄 Restart Color** to replay the color morphing

---

### 2. Spring Physics

**What is a Spring?**
Springs simulate realistic motion with inertia, velocity, and damping (like a bouncy ball or door closer).

**Spring Types:**
```rust
SpringParams::smooth()   // Gentle, smooth motion (no overshoot)
SpringParams::bouncy()   // Energetic bounce (overshoots target)
SpringParams::tight()    // Quick snap to target (minimal bounce)
```

**Example Usage:**
```rust
use astract::animation::{Spring, SpringParams};

// Create spring with bouncy physics
let mut spring = Spring::with_params(0.0, SpringParams::bouncy());

// Set target position
spring.set_target(1.0);

// In update loop:
spring.update(delta_time);
let current_position = spring.position();
let current_velocity = spring.velocity();
```

**Interactive Demo:**
- Move your mouse in the spring physics area
- The yellow circle follows your mouse with spring physics
- The white outline shows the target position
- Notice the bounce and settling behavior

---

### 3. Easing Functions

**What are Easing Functions?**
Easing functions control the rate of change during interpolation (timing curves).

**Available Easings (11 total):**

| Function | Description | Use Case |
|----------|-------------|----------|
| **Linear** | Constant speed | Mechanical motion |
| **QuadIn** | Accelerate in | Gravity, falling |
| **QuadOut** | Decelerate out | Braking, stopping |
| **CubicIn** | Strong acceleration | Dramatic entrances |
| **CubicOut** | Strong deceleration | Dramatic exits |
| **SineIn** | Gentle acceleration | Smooth starts |
| **SineOut** | Gentle deceleration | Smooth stops |
| **ExpoIn** | Very fast acceleration | Explosions |
| **ExpoOut** | Very fast deceleration | Impact effects |
| **ElasticOut** | Elastic bounce | Spring effects |
| **BounceOut** | Multiple bounces | Ball physics |

**Visualization:**
- Check ☑ **Show Easing Comparison** to see all 11 easing functions simultaneously
- Each function is color-coded and labeled
- The graph shows how each easing interpolates 0 → 1 over 2 seconds
- Click **🔄 Restart All Easings** to replay the comparison

---

## Use Cases in Game Development

### Character Animation
```rust
// Smooth camera zoom
let zoom_tween = Tween::new(1.0, 3.0, 0.5)
    .with_easing(EasingFunction::QuadOut);

// Character jump arc
let jump_tween = Tween::new(0.0, 2.0, 0.8)
    .with_easing(EasingFunction::QuadIn); // Up
// Use QuadOut for landing
```

### UI Animations
```rust
// Menu slide-in
let menu_tween = Tween::new(-200.0, 0.0, 0.3)
    .with_easing(EasingFunction::CubicOut);

// Button color pulse
let pulse_tween = Tween::new(Color32::GRAY, Color32::WHITE, 0.5)
    .with_easing(EasingFunction::SineInOut);
```

### Physics & Feel
```rust
// Recoil recovery (gun kick)
let recoil_spring = Spring::with_params(0.0, SpringParams::tight());

// Camera shake (impact)
let shake_spring = Spring::with_params(0.0, SpringParams::bouncy());

// Door closing (realistic physics)
let door_spring = Spring::with_params(0.0, SpringParams::smooth());
```

---

## Advanced Topics

### Custom Easing Functions

The `astract` library supports custom easing functions via closures:

```rust
let custom_easing = |t: f32| -> f32 {
    // Your custom curve (t is 0.0 to 1.0)
    t * t * (3.0 - 2.0 * t) // Smoothstep
};

let tween = Tween::new(0.0, 100.0, 2.0)
    .with_custom_easing(custom_easing);
```

### Chaining Animations

Sequence multiple tweens for complex motion:

```rust
let tween1 = Tween::new(0.0, 50.0, 1.0);
let tween2 = Tween::new(50.0, 100.0, 1.0);

// In update loop:
if tween1.is_complete() {
    tween2.play();
}
```

### Looping Animations

Create continuous animations:

```rust
let mut tween = Tween::new(0.0, 360.0, 2.0)
    .with_easing(EasingFunction::Linear);

// In update loop:
tween.update(dt);
if tween.is_complete() {
    tween.restart(); // Loop forever
}
```

---

## API Reference

### Tween<T>

```rust
// Create tween
pub fn new(start: T, end: T, duration: f32) -> Self

// Configuration
pub fn with_easing(self, easing: EasingFunction) -> Self
pub fn with_custom_easing<F>(self, func: F) -> Self

// Playback control
pub fn play(&mut self)
pub fn pause(&mut self)
pub fn restart(&mut self)

// Update (call every frame)
pub fn update(&mut self, delta_time: f32)

// Query state
pub fn value(&self) -> T        // Current interpolated value
pub fn progress(&self) -> f32   // 0.0 to 1.0
pub fn is_running(&self) -> bool
pub fn is_paused(&self) -> bool
pub fn is_complete(&self) -> bool
```

### Spring

```rust
// Create spring
pub fn new(initial_position: f32) -> Self
pub fn with_params(initial_position: f32, params: SpringParams) -> Self

// Control
pub fn set_target(&mut self, target: f32)
pub fn set_position(&mut self, position: f32)
pub fn set_velocity(&mut self, velocity: f32)

// Update (call every frame)
pub fn update(&mut self, delta_time: f32)

// Query state
pub fn position(&self) -> f32
pub fn velocity(&self) -> f32
pub fn target(&self) -> f32
```

### SpringParams

```rust
// Presets
SpringParams::smooth()   // frequency: 1.0, damping: 1.0
SpringParams::bouncy()   // frequency: 3.0, damping: 0.3
SpringParams::tight()    // frequency: 5.0, damping: 0.8

// Custom
SpringParams {
    frequency: f32,  // Speed (higher = faster)
    damping: f32,    // Bounce (lower = more bounce, 1.0 = critical)
}
```

---

## Tips & Best Practices

### Performance
- ✅ Tweens are lightweight (12-16 bytes per instance)
- ✅ Springs are fast (2-4 multiplications per update)
- ✅ Safe to have 100+ active animations simultaneously

### Timing
- Use **dt = 0.016** (60 FPS) or **dt = 0.008** (120 FPS)
- Always call `update(dt)` every frame for smooth animation
- Pause tweens when not visible to save CPU

### Easing Selection
- **UI animations:** QuadOut, CubicOut (fast entry, smooth stop)
- **Character movement:** SineInOut (smooth acceleration + deceleration)
- **Impact effects:** ElasticOut, BounceOut (dramatic feel)
- **Mechanical motion:** Linear (constant speed)

### Spring Tuning
- **Smooth UI:** `frequency: 2.0, damping: 1.0` (no overshoot)
- **Game feel:** `frequency: 4.0, damping: 0.5` (slight bounce)
- **Cartoon physics:** `frequency: 5.0, damping: 0.2` (exaggerated bounce)

---

## Troubleshooting

### Tween not animating
- ✅ Call `tween.play()` to start
- ✅ Call `tween.update(dt)` every frame
- ✅ Check `tween.is_running()` to verify state

### Spring oscillating forever
- ✅ Increase damping (closer to 1.0 = less bounce)
- ✅ Use `SpringParams::smooth()` for no overshoot
- ✅ Check delta_time isn't too large (should be ~0.016)

### Animations feel sluggish
- ✅ Decrease tween duration (try 0.2-0.5 seconds)
- ✅ Use faster easing (ExpoOut, QuadOut)
- ✅ Increase spring frequency (try 5.0+)

---

## Testing

The Animation Panel includes 4 automated tests:

```bash
# Run animation panel tests
cargo test -p aw_editor animation::tests
```

**Tests:**
- `test_animation_panel_creation` - Verifies 11 easing functions created
- `test_bounce_tween_running` - Validates tween auto-start
- `test_color_tween_running` - Validates color tween auto-start
- `test_spring_initial_position` - Verifies spring initialization

---

## Related Documentation

- **astract Animation Library:** Full API documentation
- **EDITOR_USER_GUIDE.md:** Main editor reference
- **EDITOR_STATUS_REPORT.md:** Feature completion status

---

## Keyboard Shortcuts

**While Animation Panel is focused:**
- `Space` - Pause/resume all animations (if implemented)
- `R` - Restart all animations
- `Esc` - Close panel

**Global shortcuts** (work anywhere in editor):
- `F9` - Toggle Animation Panel (if mapped)

---

## Conclusion

The Animation Panel is a tool for testing and demonstrating animation systems. It showcases the capabilities of the `astract` animation library with interactive, real-time visualizations.

**Use it to:**
- Test easing functions for your game
- Experiment with spring physics parameters
- Learn animation timing and feel
- Prototype UI animations before coding

---

**Guide Version:** 1.0  
**Panel Status:** ✅ Functional (wired into the editor)  
**Test Coverage:** 4 automated tests (all passing)
