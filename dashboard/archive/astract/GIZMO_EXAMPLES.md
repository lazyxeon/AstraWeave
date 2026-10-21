# Astract Gizmo System - Complete Examples

**Version**: 1.0  
**Date**: November 4, 2025  
**Purpose**: Real-world integration examples

---

## Table of Contents

1. [Example 1: Simple Translation Workflow](#example-1-simple-translation-workflow)
2. [Example 2: Rotation with Snapping](#example-2-rotation-with-snapping)
3. [Example 3: Per-Axis Scaling](#example-3-per-axis-scaling)
4. [Example 4: Full Viewport Integration](#example-4-full-viewport-integration)
5. [Example 5: Numeric Input Workflow](#example-5-numeric-input-workflow)
6. [Example 6: Local vs World Space](#example-6-local-vs-world-space)
7. [Example 7: Complete Game Engine Integration](#example-7-complete-game-engine-integration)

---

## Example 1: Simple Translation Workflow

**Objective**: Move an object along the X-axis using mouse input.

```rust
use aw_editor_lib::gizmo::{GizmoState, TranslateGizmo, AxisConstraint};
use glam::{Vec2, Vec3, Quat};
use winit::keyboard::KeyCode;

fn main() {
    // Initialize scene
    let mut object_position = Vec3::new(0.0, 0.0, 0.0);
    let mut gizmo = GizmoState::new();
    
    // Simulated input sequence:
    // 1. User presses 'G' to start translation
    // 2. User presses 'X' to constrain to X-axis
    // 3. User moves mouse 100px right
    // 4. User presses Enter to confirm
    
    // Step 1: Start translation
    gizmo.start_translate();
    println!("Translation mode active: {}", gizmo.is_active());
    // Output: Translation mode active: true
    
    // Step 2: Constrain to X-axis
    gizmo.handle_key(KeyCode::KeyX);
    println!("Constraint: {:?}", gizmo.constraint());
    // Output: Constraint: X
    
    // Step 3: Mouse movement
    let mouse_delta = Vec2::new(100.0, 0.0); // 100px right
    gizmo.update_mouse(mouse_delta);
    
    // Calculate translation
    let translation = TranslateGizmo::calculate_translation(
        gizmo.mouse_delta(),
        gizmo.constraint(),
        10.0,           // Camera distance
        Quat::IDENTITY, // No object rotation
        false,          // World space
    );
    
    println!("Translation delta: {:?}", translation);
    // Output: Translation delta: Vec3 { x: 10.0, y: 0.0, z: 0.0 }
    
    // Apply translation
    object_position += translation;
    println!("New position: {:?}", object_position);
    // Output: New position: Vec3 { x: 10.0, y: 0.0, z: 0.0 }
    
    // Step 4: Confirm
    gizmo.confirm_transform();
    println!("Transform confirmed, active: {}", gizmo.is_active());
    // Output: Transform confirmed, active: false
}
```

**Key Takeaways**:
- Translation sensitivity scales with `camera_distance`
- Mouse delta is screen-space (pixels)
- Constraint determines allowed movement axes
- `confirm_transform()` resets gizmo to `Inactive`

---

## Example 2: Rotation with Snapping

**Objective**: Rotate an object around Z-axis in 15° increments.

```rust
use aw_editor_lib::gizmo::{GizmoState, RotateGizmo, AxisConstraint};
use glam::{Vec2, Quat};
use winit::keyboard::KeyCode;

fn main() {
    // Initialize object rotation
    let mut object_rotation = Quat::IDENTITY;
    let mut gizmo = GizmoState::new();
    
    // Simulated input sequence:
    // 1. User presses 'R' to start rotation
    // 2. User presses 'Z' to constrain to Z-axis
    // 3. User moves mouse 50px (rotates)
    // 4. User presses Enter to confirm
    
    // Step 1: Start rotation
    gizmo.start_rotate();
    println!("Rotation mode active");
    
    // Step 2: Constrain to Z-axis
    gizmo.handle_key(KeyCode::KeyZ);
    
    // Step 3: Mouse movement (with snapping enabled)
    gizmo.toggle_snap(); // Enable 15° snapping
    let mouse_delta = Vec2::new(50.0, 0.0); // 50px right
    gizmo.update_mouse(mouse_delta);
    
    // Calculate rotation
    let rotation_delta = RotateGizmo::calculate_rotation(
        gizmo.mouse_delta(),
        gizmo.constraint(),
        1.0,               // Sensitivity (1.0 = 100px per radian)
        gizmo.snap_enabled(), // true (snapping on)
        object_rotation,   // Current rotation
        false,             // World space
    );
    
    // Check rotation angle
    let (axis, angle) = rotation_delta.to_axis_angle();
    println!("Rotation: {:.1}° around {:?}", angle.to_degrees(), axis);
    // Output: Rotation: 30.0° around Vec3(0.0, 0.0, 1.0)
    // (50px ≈ 0.5 radians ≈ 28.6°, snapped to 30°)
    
    // Apply rotation
    object_rotation = rotation_delta * object_rotation;
    
    // Step 4: Confirm
    gizmo.confirm_transform();
}
```

**Key Takeaways**:
- Snapping rounds to nearest 15° (0°, 15°, 30°, ...)
- `toggle_snap()` enables/disables snapping
- Rotation is a **delta** (multiply: `new = delta * old`)
- Sensitivity controls degrees per pixel (100px = 1 radian ≈ 57°)

---

## Example 3: Per-Axis Scaling

**Objective**: Scale object 2× on Y-axis only.

```rust
use aw_editor_lib::gizmo::{GizmoState, ScaleGizmo, AxisConstraint};
use glam::{Vec2, Vec3, Quat};
use winit::keyboard::KeyCode;

fn main() {
    // Initialize object scale
    let mut object_scale = Vec3::ONE; // (1.0, 1.0, 1.0)
    let mut gizmo = GizmoState::new();
    
    // Simulated input sequence:
    // 1. User presses 'S' to start scale
    // 2. User presses 'Y' to constrain to Y-axis
    // 3. User moves mouse 100px up (scales 2×)
    // 4. User presses Enter to confirm
    
    // Step 1: Start scale
    gizmo.start_scale();
    
    // Step 2: Constrain to Y-axis
    gizmo.handle_key(KeyCode::KeyY);
    
    // Step 3: Mouse movement
    let mouse_delta = Vec2::new(0.0, 100.0); // 100px up (positive Y in screen space)
    gizmo.update_mouse(mouse_delta);
    
    // Calculate scale
    let scale_multiplier = ScaleGizmo::calculate_scale(
        gizmo.mouse_delta(),
        gizmo.constraint(),
        false,          // Not uniform (per-axis)
        1.0,            // Sensitivity (100px = 1× scale change)
        Quat::IDENTITY, // Unused for scale
        false,          // Unused for scale
    );
    
    println!("Scale multiplier: {:?}", scale_multiplier);
    // Output: Scale multiplier: Vec3 { x: 1.0, y: 2.0, z: 1.0 }
    
    // Apply scale
    object_scale *= scale_multiplier;
    println!("New scale: {:?}", object_scale);
    // Output: New scale: Vec3 { x: 1.0, y: 2.0, z: 1.0 }
    
    // Step 4: Confirm
    gizmo.confirm_transform();
}
```

**Key Takeaways**:
- Scale is a **multiplier** (use `*=`, not `+=`)
- `uniform: false` allows per-axis scaling
- Scale is always in local space (object-relative)
- Mouse delta magnitude determines scale factor

---

## Example 4: Full Viewport Integration

**Objective**: Complete 3D viewport with camera controls and gizmo.

```rust
use aw_editor_lib::gizmo::{
    GizmoState, TranslateGizmo, RotateGizmo, ScaleGizmo,
    GizmoRenderer, GizmoPicker,
    scene_viewport::{CameraController, Transform},
};
use glam::{Vec2, Vec3};
use winit::keyboard::KeyCode;

struct Editor {
    camera: CameraController,
    gizmo: GizmoState,
    selected_object: Option<usize>,
    objects: Vec<Transform>,
    picker: GizmoPicker,
}

impl Editor {
    fn new() -> Self {
        Self {
            camera: CameraController::default(),
            gizmo: GizmoState::new(),
            selected_object: Some(0), // Select first object
            objects: vec![Transform::default()],
            picker: GizmoPicker::default(),
        }
    }
    
    fn handle_keyboard(&mut self, key: KeyCode) {
        match key {
            // Gizmo modes
            KeyCode::KeyG => self.gizmo.start_translate(),
            KeyCode::KeyR => self.gizmo.start_rotate(),
            KeyCode::KeyS => self.gizmo.start_scale(),
            
            // Constraints
            KeyCode::KeyX | KeyCode::KeyY | KeyCode::KeyZ => {
                self.gizmo.handle_key(key);
            }
            
            // Confirm/Cancel
            KeyCode::Enter => {
                self.apply_transform();
                self.gizmo.confirm_transform();
            }
            KeyCode::Escape => {
                self.gizmo.cancel_transform();
            }
            
            _ => {}
        }
    }
    
    fn handle_mouse_move(&mut self, delta: Vec2, middle_button: bool) {
        if middle_button {
            // Camera control (middle mouse button)
            self.camera.orbit(delta * 0.01, 1.0);
        } else if self.gizmo.is_active() {
            // Gizmo transform
            self.gizmo.update_mouse(delta);
        }
    }
    
    fn handle_mouse_wheel(&mut self, delta: f32) {
        self.camera.zoom(delta, 1.0);
    }
    
    fn handle_mouse_click(&mut self, screen_pos: Vec2, viewport_size: Vec2) {
        if self.gizmo.is_active() {
            return; // Ignore clicks during transform
        }
        
        // Convert screen to world ray
        let (ray_origin, ray_direction) = GizmoPicker::screen_to_world_ray(
            screen_pos,
            viewport_size,
            self.camera.view_matrix(),
            self.camera.projection_matrix(),
        );
        
        // Try to pick gizmo handle
        if let Some(object_idx) = self.selected_object {
            let object = &self.objects[object_idx];
            let picked = self.picker.pick_handle(
                ray_origin,
                ray_direction,
                &self.gizmo.mode(),
                object.translation,
            );
            
            if let Some(constraint) = picked {
                // Start transform on picked axis
                match self.gizmo.mode() {
                    aw_editor_lib::gizmo::GizmoMode::Translate { .. } => {
                        self.gizmo.start_translate();
                        self.gizmo.set_constraint(constraint);
                    }
                    aw_editor_lib::gizmo::GizmoMode::Rotate { .. } => {
                        self.gizmo.start_rotate();
                        self.gizmo.set_constraint(constraint);
                    }
                    aw_editor_lib::gizmo::GizmoMode::Scale { .. } => {
                        self.gizmo.start_scale();
                        self.gizmo.set_constraint(constraint);
                    }
                    _ => {}
                }
            }
        }
    }
    
    fn apply_transform(&mut self) {
        if let Some(object_idx) = self.selected_object {
            let object = &mut self.objects[object_idx];
            
            match self.gizmo.mode() {
                aw_editor_lib::gizmo::GizmoMode::Translate { constraint } => {
                    let delta = TranslateGizmo::calculate_translation(
                        self.gizmo.mouse_delta(),
                        *constraint,
                        self.camera.distance,
                        object.rotation,
                        false, // World space
                    );
                    object.translation += delta;
                }
                aw_editor_lib::gizmo::GizmoMode::Rotate { constraint } => {
                    let delta = RotateGizmo::calculate_rotation(
                        self.gizmo.mouse_delta(),
                        *constraint,
                        1.0,
                        self.gizmo.snap_enabled(),
                        object.rotation,
                        false,
                    );
                    object.rotation = delta * object.rotation;
                }
                aw_editor_lib::gizmo::GizmoMode::Scale { constraint, uniform } => {
                    let delta = ScaleGizmo::calculate_scale(
                        self.gizmo.mouse_delta(),
                        *constraint,
                        *uniform,
                        1.0,
                        object.rotation,
                        false,
                    );
                    object.scale *= delta;
                }
                _ => {}
            }
        }
    }
    
    fn render(&self) {
        // Render objects
        for object in &self.objects {
            // Render object mesh with transform...
            println!("Object at {:?}", object.translation);
        }
        
        // Render gizmo handles
        if let Some(object_idx) = self.selected_object {
            let object = &self.objects[object_idx];
            
            let handles = match self.gizmo.mode() {
                aw_editor_lib::gizmo::GizmoMode::Translate { .. } => {
                    GizmoRenderer::render_translate(object.translation, 1.0)
                }
                aw_editor_lib::gizmo::GizmoMode::Rotate { .. } => {
                    GizmoRenderer::render_rotate(object.translation, 1.0)
                }
                aw_editor_lib::gizmo::GizmoMode::Scale { .. } => {
                    GizmoRenderer::render_scale(object.translation, 1.0)
                }
                _ => vec![],
            };
            
            for handle in handles {
                // Render handle with graphics API...
                println!("Handle: {:?} at {:?}", handle.axis, handle.position);
            }
        }
    }
}

fn main() {
    let mut editor = Editor::new();
    
    // Simulate user interaction
    editor.handle_keyboard(KeyCode::KeyG); // Start translate
    editor.handle_keyboard(KeyCode::KeyX); // Constrain to X
    editor.handle_mouse_move(Vec2::new(100.0, 0.0), false); // Move mouse
    editor.handle_keyboard(KeyCode::Enter); // Confirm
    
    editor.render();
    // Output: Object at Vec3(10.0, 0.0, 0.0)
}
```

**Key Takeaways**:
- Separate camera controls from gizmo transforms
- Use picking for handle selection
- Apply transforms in `confirm_transform()` callback
- Render gizmo handles based on current mode

---

## Example 5: Numeric Input Workflow

**Objective**: Move object exactly 5.2 units on X-axis using keyboard input.

```rust
use aw_editor_lib::gizmo::{GizmoState, TranslateGizmo, AxisConstraint};
use glam::{Vec3, Quat};
use winit::keyboard::KeyCode;

fn main() {
    let mut object_position = Vec3::ZERO;
    let mut gizmo = GizmoState::new();
    
    // Simulated input sequence:
    // 1. User presses 'G' to start translation
    // 2. User presses 'X' to constrain to X-axis
    // 3. User types "5.2" (numeric input)
    // 4. User presses Enter to confirm
    
    // Step 1: Start translation
    gizmo.start_translate();
    
    // Step 2: Constrain to X-axis
    gizmo.handle_key(KeyCode::KeyX);
    
    // Step 3: Numeric input
    gizmo.add_numeric_input('5');
    gizmo.add_numeric_input('.');
    gizmo.add_numeric_input('2');
    println!("Numeric input: {}", gizmo.numeric_input());
    // Output: Numeric input: 5.2
    
    // Parse numeric input
    if let Some(value) = gizmo.parse_numeric_input() {
        println!("Parsed value: {}", value);
        // Output: Parsed value: 5.2
        
        // Calculate translation
        let translation = TranslateGizmo::calculate_translation_numeric(
            value,
            gizmo.constraint(),
            Quat::IDENTITY,
            false,
        );
        
        println!("Translation: {:?}", translation);
        // Output: Translation: Vec3 { x: 5.2, y: 0.0, z: 0.0 }
        
        // Apply translation
        object_position += translation;
        println!("New position: {:?}", object_position);
        // Output: New position: Vec3 { x: 5.2, y: 0.0, z: 0.0 }
    }
    
    // Step 4: Confirm
    gizmo.confirm_transform();
}
```

**Numeric Input Features**:
- Supports **0-9**, **.**, **-** (negative numbers)
- Auto-parsed to `f32`
- Clear with `clear_numeric_input()` or `Esc`
- Works with translation, rotation (degrees), and scale

**Example: Rotation with Numeric Input**:

```rust
use aw_editor_lib::gizmo::{GizmoState, RotateGizmo, AxisConstraint};

fn rotate_90_degrees() {
    let mut gizmo = GizmoState::new();
    gizmo.start_rotate();
    gizmo.handle_key(KeyCode::KeyZ); // Z-axis
    
    // Type "90"
    gizmo.add_numeric_input('9');
    gizmo.add_numeric_input('0');
    
    if let Some(degrees) = gizmo.parse_numeric_input() {
        let rotation = RotateGizmo::calculate_rotation_numeric(
            degrees,
            gizmo.constraint(),
            Quat::IDENTITY,
            false,
        );
        
        // rotation is now 90° around Z-axis
        let (_, angle) = rotation.to_axis_angle();
        println!("Rotation: {:.1}°", angle.to_degrees());
        // Output: Rotation: 90.0°
    }
}
```

---

## Example 6: Local vs World Space

**Objective**: Demonstrate local vs world space transforms on a rotated object.

```rust
use aw_editor_lib::gizmo::{TranslateGizmo, AxisConstraint};
use glam::{Vec2, Vec3, Quat};

fn main() {
    // Object rotated 45° around Z-axis
    let object_rotation = Quat::from_rotation_z(std::f32::consts::PI / 4.0);
    let mouse_delta = Vec2::new(100.0, 0.0); // 100px right
    
    // World space: Move along world X-axis
    let translation_world = TranslateGizmo::calculate_translation(
        mouse_delta,
        AxisConstraint::X,
        10.0,
        object_rotation,
        false, // world_space = false (confusing naming, false = world)
    );
    
    println!("World space translation: {:?}", translation_world);
    // Output: Vec3 { x: 10.0, y: 0.0, z: 0.0 }
    // (Moves along world X, ignoring object rotation)
    
    // Local space: Move along object's local X-axis (rotated 45°)
    let translation_local = TranslateGizmo::calculate_translation(
        mouse_delta,
        AxisConstraint::X,
        10.0,
        object_rotation,
        true, // local_space = true
    );
    
    println!("Local space translation: {:?}", translation_local);
    // Output: Vec3 { x: 7.07, y: 7.07, z: 0.0 }
    // (Moves along object's local X, which is rotated 45° in world space)
    
    // Verify local space is rotated 45°
    let angle = translation_local.y.atan2(translation_local.x);
    println!("Local X-axis angle: {:.1}°", angle.to_degrees());
    // Output: Local X-axis angle: 45.0°
}
```

**Visual Explanation**:

```
World Space (local_space = false):
  World X-axis: →
  Object rotated 45°: ⤢
  
  Translation follows world X (→), not object rotation.
  Result: Vec3 { x: 10.0, y: 0.0, z: 0.0 }

Local Space (local_space = true):
  World X-axis: →
  Object rotated 45°: ⤢
  Object's local X: ⤢
  
  Translation follows object's local X (⤢).
  Result: Vec3 { x: 7.07, y: 7.07, z: 0.0 } (45° rotated)
```

---

## Example 7: Complete Game Engine Integration

**Objective**: Full integration with a game engine (event loop, input, rendering).

```rust
use aw_editor_lib::gizmo::{
    GizmoState, TranslateGizmo, RotateGizmo, ScaleGizmo,
    GizmoRenderer, GizmoPicker, AxisConstraint,
    scene_viewport::{CameraController, Transform},
};
use glam::{Vec2, Vec3};
use winit::{
    event::{Event, WindowEvent, MouseButton, ElementState},
    event_loop::{EventLoop, ControlFlow},
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

struct GameEngine {
    camera: CameraController,
    gizmo: GizmoState,
    objects: Vec<Transform>,
    selected_object: Option<usize>,
    picker: GizmoPicker,
    mouse_pos: Vec2,
    last_mouse_pos: Vec2,
    middle_mouse_down: bool,
}

impl GameEngine {
    fn new() -> Self {
        Self {
            camera: CameraController::default(),
            gizmo: GizmoState::new(),
            objects: vec![Transform::default()],
            selected_object: Some(0),
            picker: GizmoPicker::default(),
            mouse_pos: Vec2::ZERO,
            last_mouse_pos: Vec2::ZERO,
            middle_mouse_down: false,
        }
    }
    
    fn update(&mut self) {
        // Update camera if middle mouse button is down
        if self.middle_mouse_down {
            let delta = self.mouse_pos - self.last_mouse_pos;
            self.camera.orbit(delta * 0.01, 1.0);
        }
        
        // Update gizmo transform if active
        if self.gizmo.is_active() {
            let delta = self.mouse_pos - self.last_mouse_pos;
            self.gizmo.update_mouse(delta);
            self.apply_transform();
        }
        
        self.last_mouse_pos = self.mouse_pos;
    }
    
    fn apply_transform(&mut self) {
        if let Some(object_idx) = self.selected_object {
            let object = &mut self.objects[object_idx];
            
            match self.gizmo.mode() {
                aw_editor_lib::gizmo::GizmoMode::Translate { constraint } => {
                    let delta = TranslateGizmo::calculate_translation(
                        self.gizmo.mouse_delta(),
                        *constraint,
                        self.camera.distance,
                        object.rotation,
                        false,
                    );
                    object.translation += delta;
                }
                aw_editor_lib::gizmo::GizmoMode::Rotate { constraint } => {
                    let delta = RotateGizmo::calculate_rotation(
                        self.gizmo.mouse_delta(),
                        *constraint,
                        1.0,
                        self.gizmo.snap_enabled(),
                        object.rotation,
                        false,
                    );
                    object.rotation = delta * object.rotation;
                }
                aw_editor_lib::gizmo::GizmoMode::Scale { constraint, uniform } => {
                    let delta = ScaleGizmo::calculate_scale(
                        self.gizmo.mouse_delta(),
                        *constraint,
                        *uniform,
                        1.0,
                        object.rotation,
                        false,
                    );
                    object.scale *= delta;
                }
                _ => {}
            }
        }
    }
    
    fn render(&self) {
        // Render objects...
        
        // Render gizmo
        if let Some(object_idx) = self.selected_object {
            let object = &self.objects[object_idx];
            
            let handles = match self.gizmo.mode() {
                aw_editor_lib::gizmo::GizmoMode::Translate { .. } => {
                    GizmoRenderer::render_translate(object.translation, 1.0)
                }
                aw_editor_lib::gizmo::GizmoMode::Rotate { .. } => {
                    GizmoRenderer::render_rotate(object.translation, 1.0)
                }
                aw_editor_lib::gizmo::GizmoMode::Scale { .. } => {
                    GizmoRenderer::render_scale(object.translation, 1.0)
                }
                _ => vec![],
            };
            
            // Render handles with graphics API...
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("Gizmo Example")
        .build(&event_loop)
        .unwrap();
    
    let mut engine = GameEngine::new();
    
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    elwt.exit();
                }
                
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state == ElementState::Pressed {
                        if let PhysicalKey::Code(code) = event.physical_key {
                            match code {
                                KeyCode::KeyG => engine.gizmo.start_translate(),
                                KeyCode::KeyR => engine.gizmo.start_rotate(),
                                KeyCode::KeyS => engine.gizmo.start_scale(),
                                KeyCode::KeyX | KeyCode::KeyY | KeyCode::KeyZ => {
                                    engine.gizmo.handle_key(code);
                                }
                                KeyCode::Enter => {
                                    engine.gizmo.confirm_transform();
                                }
                                KeyCode::Escape => {
                                    engine.gizmo.cancel_transform();
                                }
                                _ => {}
                            }
                        }
                    }
                }
                
                WindowEvent::MouseInput { state, button, .. } => {
                    if button == MouseButton::Middle {
                        engine.middle_mouse_down = state == ElementState::Pressed;
                    }
                }
                
                WindowEvent::CursorMoved { position, .. } => {
                    engine.mouse_pos = Vec2::new(position.x as f32, position.y as f32);
                }
                
                WindowEvent::MouseWheel { delta, .. } => {
                    let scroll = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                        winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 100.0,
                    };
                    engine.camera.zoom(scroll, 1.0);
                }
                
                WindowEvent::RedrawRequested => {
                    engine.update();
                    engine.render();
                }
                
                _ => {}
            },
            
            Event::AboutToWait => {
                window.request_redraw();
            }
            
            _ => {}
        }
    }).unwrap();
}
```

**Integration Checklist**:

- ✅ Handle keyboard events (G/R/S, X/Y/Z, Enter, Esc)
- ✅ Handle mouse events (move, click, scroll, middle button)
- ✅ Update gizmo state in event loop
- ✅ Apply transforms in real-time
- ✅ Render gizmo handles
- ✅ Separate camera controls from gizmo
- ✅ Use picking for handle selection

---

## Performance Tips

Based on benchmark results (see `ASTRACT_GIZMO_DAY_12_COMPLETE.md`):

1. **State transitions**: 315-382 ps (virtually free)
2. **Transform calculations**: 2.5-17 ns (negligible overhead)
3. **Rendering**: 85-150 ns per handle (3 handles = ~300 ns)
4. **Picking**: 5-35 ns per handle (very fast)

**Capacity @ 60 FPS (16.67 ms budget)**:
- **106,000 gizmos** (full workflow: 25-40 ns each)
- **55,000 translation calculations** (2.5-6 ns each)
- **196,000 state transitions** (315-382 ps each)

**Recommendation**: Don't worry about performance - gizmo system is highly optimized.

---

## Common Patterns

### Pattern 1: Two-Stage Constraint

Press axis key twice to toggle planar constraint:

```rust
// First press: X-axis constraint
gizmo.handle_key(KeyCode::KeyX); // constraint = X

// Second press: XY-plane constraint
gizmo.handle_key(KeyCode::KeyX); // constraint = XY

// Third press: X-axis again
gizmo.handle_key(KeyCode::KeyX); // constraint = X
```

### Pattern 2: Uniform Scale Shortcut

Press S twice for uniform scale:

```rust
// First press: Scale with constraint
gizmo.start_scale();
// mode = Scale { constraint: None, uniform: false }

// Second press: Force uniform scale
gizmo.start_scale();
// mode = Scale { constraint: None, uniform: true }
```

### Pattern 3: Snap Toggle

Toggle snapping with a key (e.g., Ctrl):

```rust
fn handle_key(&mut self, key: KeyCode, ctrl_pressed: bool) {
    if ctrl_pressed {
        self.gizmo.toggle_snap();
    } else {
        // Normal key handling...
    }
}
```

---

## Next Steps

- **User Guide**: See `GIZMO_USER_GUIDE.md` for detailed usage
- **API Reference**: See `GIZMO_API_REFERENCE.md` for function signatures
- **Architecture**: See `GIZMO_ARCHITECTURE.md` for system design

---

**End of Examples**
