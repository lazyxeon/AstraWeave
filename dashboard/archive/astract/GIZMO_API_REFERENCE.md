# Astract Gizmo System - API Reference

**Version**: 1.0  
**Date**: November 4, 2025  
**Crate**: `aw_editor_lib`

---

## Table of Contents

1. [Module Overview](#module-overview)
2. [Core Types](#core-types)
3. [State Machine](#state-machine)
4. [Transform Functions](#transform-functions)
5. [Rendering](#rendering)
6. [Picking](#picking)
7. [Scene Viewport](#scene-viewport)
8. [Type Aliases & Constants](#type-aliases--constants)

---

## Module Overview

### Gizmo Module Structure

```rust
pub mod gizmo {
    pub mod state;             // GizmoState, GizmoMode, AxisConstraint
    pub mod translate;         // TranslateGizmo
    pub mod rotate;            // RotateGizmo
    pub mod scale;             // ScaleGizmo
    pub mod render;            // GizmoRenderer
    pub mod picking;           // GizmoPicker
    pub mod scene_viewport;    // CameraController, Transform, SceneViewport
}
```

### Quick Import

```rust
use aw_editor_lib::gizmo::{
    // State machine
    GizmoState, GizmoMode, AxisConstraint,
    
    // Transform functions
    TranslateGizmo, RotateGizmo, ScaleGizmo,
    
    // Rendering & picking
    GizmoRenderer, GizmoPicker,
    
    // Viewport integration
    scene_viewport::{CameraController, Transform, SceneViewport},
};
```

---

## Core Types

### AxisConstraint

Constrains transforms to specific axes or planes.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisConstraint {
    None,  // Free movement (all axes)
    X,     // Lock to X-axis
    Y,     // Lock to Y-axis
    Z,     // Lock to Z-axis
    XY,    // Lock to XY plane (Z fixed)
    XZ,    // Lock to XZ plane (Y fixed)
    YZ,    // Lock to YZ plane (X fixed)
}
```

**Methods**:

```rust
impl AxisConstraint {
    /// Toggle between axis and planar constraint
    /// X → X → XY, Y → Y → XZ, Z → Z → YZ
    pub fn toggle_planar(&mut self);
    
    /// Get axis vector for single-axis constraints
    /// X → Vec3::X, Y → Vec3::Y, Z → Vec3::Z, others → Vec3::ZERO
    pub fn axis_vector(&self) -> Vec3;
    
    /// Get constraint name as string
    pub fn name(&self) -> &'static str;
}
```

**Examples**:

```rust
// Toggle constraint
let mut constraint = AxisConstraint::X;
constraint.toggle_planar(); // Now XY
constraint.toggle_planar(); // Now X again

// Get axis vector
let axis = AxisConstraint::Y.axis_vector(); // Vec3::Y

// Get name
let name = AxisConstraint::XZ.name(); // "XZ"
```

### GizmoMode

Current gizmo operating mode.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum GizmoMode {
    Inactive,
    Translate { constraint: AxisConstraint },
    Rotate { constraint: AxisConstraint },
    Scale { constraint: AxisConstraint, uniform: bool },
}
```

**Variants**:

- **`Inactive`**: No active transform
- **`Translate`**: Translation mode with axis constraint
- **`Rotate`**: Rotation mode with axis constraint
- **`Scale`**: Scale mode with axis constraint and uniform flag

**Examples**:

```rust
// Check mode
match gizmo.mode() {
    GizmoMode::Inactive => println!("No transform active"),
    GizmoMode::Translate { constraint } => {
        println!("Translating on {:?}", constraint);
    }
    GizmoMode::Rotate { constraint } => {
        println!("Rotating around {:?}", constraint);
    }
    GizmoMode::Scale { constraint, uniform } => {
        println!("Scaling {:?} (uniform: {})", constraint, uniform);
    }
}
```

---

## State Machine

### GizmoState

Main state machine for gizmo operations.

```rust
pub struct GizmoState {
    mode: GizmoMode,
    numeric_input: String,
    mouse_delta: Vec2,
    snap_enabled: bool,
}
```

**Constructor**:

```rust
impl GizmoState {
    /// Create new inactive gizmo state
    pub fn new() -> Self;
}
```

**Mode Transitions**:

```rust
impl GizmoState {
    /// Start translation (G key)
    pub fn start_translate(&mut self);
    
    /// Start rotation (R key)
    pub fn start_rotate(&mut self);
    
    /// Start scale (S key)
    pub fn start_scale(&mut self);
    
    /// Confirm current transform (Enter key)
    pub fn confirm_transform(&mut self);
    
    /// Cancel current transform (Esc key)
    pub fn cancel_transform(&mut self);
}
```

**Constraint Handling**:

```rust
impl GizmoState {
    /// Handle keyboard input for constraints (X/Y/Z keys)
    pub fn handle_key(&mut self, key: winit::keyboard::KeyCode);
    
    /// Set specific constraint
    pub fn set_constraint(&mut self, constraint: AxisConstraint);
    
    /// Get current constraint
    pub fn constraint(&self) -> AxisConstraint;
}
```

**Mouse Handling**:

```rust
impl GizmoState {
    /// Update mouse position delta
    pub fn update_mouse(&mut self, delta: Vec2);
    
    /// Get current mouse delta
    pub fn mouse_delta(&self) -> Vec2;
}
```

**Numeric Input**:

```rust
impl GizmoState {
    /// Add character to numeric input (0-9, ., -)
    pub fn add_numeric_input(&mut self, c: char);
    
    /// Parse numeric input to f32
    pub fn parse_numeric_input(&self) -> Option<f32>;
    
    /// Clear numeric input buffer
    pub fn clear_numeric_input(&mut self);
    
    /// Get current numeric input string
    pub fn numeric_input(&self) -> &str;
}
```

**Queries**:

```rust
impl GizmoState {
    /// Check if any transform is active
    pub fn is_active(&self) -> bool;
    
    /// Get current mode
    pub fn mode(&self) -> &GizmoMode;
    
    /// Check if snap is enabled
    pub fn snap_enabled(&self) -> bool;
    
    /// Toggle snap on/off
    pub fn toggle_snap(&mut self);
}
```

**Complete Example**:

```rust
use aw_editor_lib::gizmo::GizmoState;
use winit::keyboard::KeyCode;

let mut gizmo = GizmoState::new();

// User presses G
gizmo.start_translate();
assert!(gizmo.is_active());

// User presses X
gizmo.handle_key(KeyCode::KeyX);
// (Constraint now X-axis)

// User types "5"
gizmo.add_numeric_input('5');
assert_eq!(gizmo.numeric_input(), "5");

// User presses Enter
gizmo.confirm_transform();
assert!(!gizmo.is_active()); // Transform confirmed, inactive
```

---

## Transform Functions

### TranslateGizmo

Calculate translation vectors from mouse or numeric input.

```rust
pub struct TranslateGizmo;
```

**Methods**:

```rust
impl TranslateGizmo {
    /// Calculate translation from mouse delta
    ///
    /// # Arguments
    /// - `mouse_delta`: Screen-space mouse movement (pixels)
    /// - `constraint`: Axis/plane constraint
    /// - `camera_distance`: Distance from camera (affects sensitivity)
    /// - `object_rotation`: Object's current rotation (for local space)
    /// - `local_space`: Use object-local coordinates (vs world)
    ///
    /// # Returns
    /// World-space translation vector
    pub fn calculate_translation(
        mouse_delta: Vec2,
        constraint: AxisConstraint,
        camera_distance: f32,
        object_rotation: Quat,
        local_space: bool,
    ) -> Vec3;
    
    /// Calculate translation from numeric input
    ///
    /// # Arguments
    /// - `value`: Numeric input value (units)
    /// - `constraint`: Axis/plane constraint
    /// - `object_rotation`: Object's rotation (for local space)
    /// - `local_space`: Use object-local coordinates
    ///
    /// # Returns
    /// World-space translation vector
    pub fn calculate_translation_numeric(
        value: f32,
        constraint: AxisConstraint,
        object_rotation: Quat,
        local_space: bool,
    ) -> Vec3;
}
```

**Examples**:

```rust
use aw_editor_lib::gizmo::{TranslateGizmo, AxisConstraint};
use glam::{Vec2, Quat};

// Mouse-based translation
let delta = TranslateGizmo::calculate_translation(
    Vec2::new(100.0, 0.0),  // 100px right
    AxisConstraint::X,       // X-axis only
    10.0,                    // 10 units from camera
    Quat::IDENTITY,          // No object rotation
    false,                   // World space
);
// Result: Vec3 { x: ~10.0, y: 0.0, z: 0.0 }

// Numeric translation
let delta = TranslateGizmo::calculate_translation_numeric(
    5.2,                     // Move 5.2 units
    AxisConstraint::Y,       // Y-axis only
    Quat::IDENTITY,
    false,
);
// Result: Vec3 { x: 0.0, y: 5.2, z: 0.0 }

// Planar translation (XY plane)
let delta = TranslateGizmo::calculate_translation(
    Vec2::new(50.0, 50.0),  // Diagonal mouse movement
    AxisConstraint::XY,      // XY plane (Z locked)
    15.0,
    Quat::IDENTITY,
    false,
);
// Result: Vec3 { x: ~5.0, y: ~5.0, z: 0.0 }
```

### RotateGizmo

Calculate rotation quaternions from mouse or numeric input.

```rust
pub struct RotateGizmo;
```

**Methods**:

```rust
impl RotateGizmo {
    /// Calculate rotation from mouse delta
    ///
    /// # Arguments
    /// - `mouse_delta`: Screen-space mouse movement (pixels)
    /// - `constraint`: Rotation axis (X/Y/Z only, planes ignored)
    /// - `sensitivity`: Rotation speed (100px = sensitivity radians)
    /// - `snap_enabled`: Enable 15° snapping
    /// - `object_rotation`: Object's rotation (for local space)
    /// - `local_space`: Use object-local coordinates
    ///
    /// # Returns
    /// Rotation quaternion (delta, not absolute)
    pub fn calculate_rotation(
        mouse_delta: Vec2,
        constraint: AxisConstraint,
        sensitivity: f32,
        snap_enabled: bool,
        object_rotation: Quat,
        local_space: bool,
    ) -> Quat;
    
    /// Calculate rotation from numeric input (degrees)
    ///
    /// # Arguments
    /// - `degrees`: Rotation angle in degrees
    /// - `constraint`: Rotation axis (X/Y/Z only)
    /// - `object_rotation`: Object's rotation (for local space)
    /// - `local_space`: Use object-local coordinates
    ///
    /// # Returns
    /// Rotation quaternion
    pub fn calculate_rotation_numeric(
        degrees: f32,
        constraint: AxisConstraint,
        object_rotation: Quat,
        local_space: bool,
    ) -> Quat;
}
```

**Snapping**:
- 15° increments (π/12 radians)
- Snap threshold: 0.01 radians (~0.57°)

**Examples**:

```rust
use aw_editor_lib::gizmo::{RotateGizmo, AxisConstraint};
use glam::{Vec2, Quat};

// Mouse-based rotation (smooth)
let rotation = RotateGizmo::calculate_rotation(
    Vec2::new(100.0, 0.0),  // 100px mouse movement
    AxisConstraint::Z,       // Rotate around Z-axis
    1.0,                     // Default sensitivity
    false,                   // No snapping
    Quat::IDENTITY,
    false,
);
// Result: ~1 radian rotation around Z

// Mouse-based rotation (snapped)
let rotation = RotateGizmo::calculate_rotation(
    Vec2::new(100.0, 0.0),
    AxisConstraint::Z,
    1.0,
    true,  // Snap enabled
    Quat::IDENTITY,
    false,
);
// Result: 0°, 15°, 30°, 45°, 60°, 75°, or 90° (snapped to nearest 15°)

// Numeric rotation
let rotation = RotateGizmo::calculate_rotation_numeric(
    90.0,                    // 90 degrees
    AxisConstraint::X,       // Around X-axis
    Quat::IDENTITY,
    false,
);
// Result: 90° rotation around X

// Apply to object
object.rotation = rotation * object.rotation;
```

### ScaleGizmo

Calculate scale multipliers from mouse or numeric input.

```rust
pub struct ScaleGizmo;
```

**Methods**:

```rust
impl ScaleGizmo {
    /// Calculate scale from mouse delta
    ///
    /// # Arguments
    /// - `mouse_delta`: Screen-space mouse movement (pixels)
    /// - `constraint`: Scale axis (X/Y/Z or None for uniform)
    /// - `uniform`: Force uniform scaling (all axes)
    /// - `sensitivity`: Scale speed (100px = sensitivity× scale)
    /// - `_object_rotation`: Unused (scale doesn't rotate axes)
    /// - `_local_space`: Unused (scale is always local)
    ///
    /// # Returns
    /// Scale multiplier vector (clamped to [0.01, 100.0])
    pub fn calculate_scale(
        mouse_delta: Vec2,
        constraint: AxisConstraint,
        uniform: bool,
        sensitivity: f32,
        _object_rotation: Quat,
        _local_space: bool,
    ) -> Vec3;
    
    /// Calculate scale from numeric input
    ///
    /// # Arguments
    /// - `value`: Scale multiplier (1.0 = no change)
    /// - `constraint`: Scale axis
    /// - `uniform`: Force uniform scaling
    ///
    /// # Returns
    /// Scale multiplier vector (clamped to [0.01, 100.0])
    pub fn calculate_scale_numeric(
        value: f32,
        constraint: AxisConstraint,
        uniform: bool,
    ) -> Vec3;
}
```

**Constants**:
```rust
const MIN_SCALE: f32 = 0.01;   // Minimum scale (1%)
const MAX_SCALE: f32 = 100.0;  // Maximum scale (100×)
```

**Examples**:

```rust
use aw_editor_lib::gizmo::{ScaleGizmo, AxisConstraint};
use glam::{Vec2, Vec3, Quat};

// Uniform scaling (all axes)
let scale = ScaleGizmo::calculate_scale(
    Vec2::new(100.0, 0.0),  // 100px mouse movement
    AxisConstraint::None,
    true,                    // Uniform
    1.0,
    Quat::IDENTITY,
    false,
);
// Result: Vec3::splat(2.0) = 2× on all axes

// Per-axis scaling
let scale = ScaleGizmo::calculate_scale(
    Vec2::new(100.0, 0.0),
    AxisConstraint::X,       // X-axis only
    false,                   // Not uniform
    1.0,
    Quat::IDENTITY,
    false,
);
// Result: Vec3::new(2.0, 1.0, 1.0) = 2× on X only

// Numeric scale
let scale = ScaleGizmo::calculate_scale_numeric(
    3.0,                     // 3× scale
    AxisConstraint::Y,
    false,
);
// Result: Vec3::new(1.0, 3.0, 1.0) = 3× on Y only

// Safe clamping (prevents negative/infinite scale)
let scale = ScaleGizmo::calculate_scale_numeric(
    0.001,                   // Too small
    AxisConstraint::X,
    false,
);
// Result: Vec3::new(0.01, 1.0, 1.0) = Clamped to minimum

// Apply to object
object.scale *= scale;
```

---

## Rendering

### GizmoRenderer

Render 3D gizmo handles.

```rust
pub struct GizmoRenderer;
```

**Methods**:

```rust
impl GizmoRenderer {
    /// Render translation gizmo (3 arrows: X=red, Y=green, Z=blue)
    pub fn render_translate(
        position: Vec3,
        scale: f32,
    ) -> Vec<GizmoHandle>;
    
    /// Render rotation gizmo (3 circles: X=red, Y=green, Z=blue)
    pub fn render_rotate(
        position: Vec3,
        scale: f32,
    ) -> Vec<GizmoHandle>;
    
    /// Render scale gizmo (3 cubes: X=red, Y=green, Z=blue)
    pub fn render_scale(
        position: Vec3,
        scale: f32,
    ) -> Vec<GizmoHandle>;
}
```

**GizmoHandle Type**:

```rust
pub struct GizmoHandle {
    pub shape: GizmoShape,     // Arrow, Circle, or Cube
    pub axis: AxisConstraint,  // Which axis (X/Y/Z)
    pub color: Vec3,           // RGB color (0.0-1.0)
    pub position: Vec3,        // World position
    pub scale: f32,            // Handle size
}

pub enum GizmoShape {
    Arrow,   // Translation handle
    Circle,  // Rotation handle
    Cube,    // Scale handle
}
```

**Examples**:

```rust
use aw_editor_lib::gizmo::GizmoRenderer;
use glam::Vec3;

// Render translation gizmo
let handles = GizmoRenderer::render_translate(
    Vec3::new(0.0, 0.0, 0.0),  // Origin
    1.0,                        // Standard size
);
// Returns 3 handles: X (red), Y (green), Z (blue) arrows

// Render rotation gizmo
let handles = GizmoRenderer::render_rotate(
    Vec3::new(5.0, 2.0, 0.0),  // Custom position
    0.5,                        // Half size
);
// Returns 3 handles: X (red), Y (green), Z (blue) circles

// Render scale gizmo
let handles = GizmoRenderer::render_scale(
    Vec3::ZERO,
    2.0,                        // Double size
);
// Returns 3 handles: X (red), Y (green), Z (blue) cubes

// Iterate over handles
for handle in handles {
    println!("Axis: {:?}, Color: {:?}, Shape: {:?}",
        handle.axis, handle.color, handle.shape);
    // Render handle with graphics API...
}
```

---

## Picking

### GizmoPicker

Ray-picking for gizmo handle selection.

```rust
#[derive(Default)]
pub struct GizmoPicker {
    threshold: f32,  // Hit detection threshold
}
```

**Constructor**:

```rust
impl GizmoPicker {
    /// Create default picker (threshold = 0.1)
    pub fn default() -> Self;
}
```

**Methods**:

```rust
impl GizmoPicker {
    /// Pick gizmo handle from screen ray
    ///
    /// # Arguments
    /// - `ray_origin`: Ray start position (camera)
    /// - `ray_direction`: Ray direction (normalized)
    /// - `mode`: Current gizmo mode (determines handle type)
    /// - `gizmo_position`: Gizmo center position
    ///
    /// # Returns
    /// Picked axis constraint, or None if no hit
    pub fn pick_handle(
        &self,
        ray_origin: Vec3,
        ray_direction: Vec3,
        mode: &GizmoMode,
        gizmo_position: Vec3,
    ) -> Option<AxisConstraint>;
    
    /// Screen to world ray conversion
    ///
    /// # Arguments
    /// - `screen_pos`: Screen coordinates (pixels, origin top-left)
    /// - `viewport_size`: Viewport dimensions (width, height)
    /// - `view_matrix`: Camera view matrix
    /// - `projection_matrix`: Camera projection matrix
    ///
    /// # Returns
    /// (ray_origin, ray_direction) in world space
    pub fn screen_to_world_ray(
        screen_pos: Vec2,
        viewport_size: Vec2,
        view_matrix: Mat4,
        projection_matrix: Mat4,
    ) -> (Vec3, Vec3);
}
```

**Picking Algorithms**:
- **Arrow**: Ray-cylinder intersection
- **Circle**: Ray-torus intersection
- **Cube**: Ray-box intersection

**Examples**:

```rust
use aw_editor_lib::gizmo::{GizmoPicker, GizmoMode, AxisConstraint};
use glam::{Vec2, Vec3, Mat4};

let picker = GizmoPicker::default();

// Get mouse ray
let (ray_origin, ray_direction) = GizmoPicker::screen_to_world_ray(
    Vec2::new(640.0, 360.0),  // Mouse position (screen)
    Vec2::new(1280.0, 720.0), // Viewport size
    view_matrix,               // Camera view
    projection_matrix,         // Camera projection
);

// Pick gizmo handle
let mode = GizmoMode::Translate { constraint: AxisConstraint::None };
let picked = picker.pick_handle(
    ray_origin,
    ray_direction,
    &mode,
    Vec3::ZERO,  // Gizmo at origin
);

match picked {
    Some(AxisConstraint::X) => println!("Picked X-axis"),
    Some(AxisConstraint::Y) => println!("Picked Y-axis"),
    Some(AxisConstraint::Z) => println!("Picked Z-axis"),
    None => println!("No hit"),
    _ => {}
}
```

---

## Scene Viewport

### CameraController

Orbit, pan, and zoom camera controller.

```rust
pub struct CameraController {
    pub target: Vec3,     // Look-at target
    pub yaw: f32,         // Horizontal rotation (radians)
    pub pitch: f32,       // Vertical rotation (radians, clamped ±85°)
    pub distance: f32,    // Distance from target
    pub fov: f32,         // Field of view (radians, default π/4)
    pub aspect: f32,      // Aspect ratio (width/height)
    pub near: f32,        // Near clip plane
    pub far: f32,         // Far clip plane
}
```

**Constructor**:

```rust
impl Default for CameraController {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            distance: 10.0,
            fov: std::f32::consts::PI / 4.0,  // 45°
            aspect: 16.0 / 9.0,
            near: 0.1,
            far: 1000.0,
        }
    }
}
```

**Methods**:

```rust
impl CameraController {
    /// Orbit camera around target
    pub fn orbit(&mut self, delta: Vec2, sensitivity: f32);
    
    /// Pan camera (move target)
    pub fn pan(&mut self, delta: Vec2, sensitivity: f32);
    
    /// Zoom camera (change distance)
    pub fn zoom(&mut self, delta: f32, sensitivity: f32);
    
    /// Get view matrix
    pub fn view_matrix(&self) -> Mat4;
    
    /// Get projection matrix
    pub fn projection_matrix(&self) -> Mat4;
    
    /// Get combined view-projection matrix
    pub fn view_projection_matrix(&self) -> Mat4;
    
    /// Get camera position in world space
    pub fn position(&self) -> Vec3;
}
```

**Examples**:

```rust
use aw_editor_lib::gizmo::scene_viewport::CameraController;
use glam::Vec2;

let mut camera = CameraController::default();

// Orbit by 0.1 radians
camera.orbit(Vec2::new(0.1, 0.05), 1.0);

// Pan 10 pixels
camera.pan(Vec2::new(10.0, 5.0), 1.0);

// Zoom in
camera.zoom(-1.0, 1.0);

// Get matrices
let view = camera.view_matrix();
let proj = camera.projection_matrix();
let vp = camera.view_projection_matrix();
```

### Transform

Object transform representation.

```rust
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}
```

**Methods**:

```rust
impl Transform {
    /// Get transform matrix (TRS order)
    pub fn matrix(&self) -> Mat4;
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}
```

### SceneViewport

Complete viewport with camera and objects.

```rust
pub struct SceneViewport {
    pub camera: CameraController,
    pub transforms: Vec<Transform>,
}
```

**Methods**:

```rust
impl SceneViewport {
    /// Create new viewport
    pub fn new() -> Self;
    
    /// Add object to scene
    pub fn add_object(&mut self, transform: Transform);
    
    /// Get object transform (mutable)
    pub fn get_object_mut(&mut self, index: usize) -> Option<&mut Transform>;
}
```

---

## Type Aliases & Constants

### Common Types

```rust
use glam::{Vec2, Vec3, Quat, Mat4};
use winit::keyboard::KeyCode;
```

### Performance Constants

From benchmarking (see `ASTRACT_GIZMO_BENCHMARKS.md`):

```rust
// State transition time: 315-382 picoseconds
// Translation math: 2.5-6 nanoseconds
// Rotation math: 17 nanoseconds
// Scale math: 10-15 nanoseconds
// Rendering: 85-150 nanoseconds
// Picking: 5-35 nanoseconds
// Full workflow: 25-40 nanoseconds

// 60 FPS capacity
const GIZMOS_PER_FRAME_60FPS: usize = 106_000;
```

### Scale Limits

```rust
const MIN_SCALE: f32 = 0.01;   // 1% of original
const MAX_SCALE: f32 = 100.0;  // 100× original
```

### Rotation Snap

```rust
const SNAP_INCREMENT: f32 = std::f32::consts::PI / 12.0;  // 15°
const SNAP_THRESHOLD: f32 = 0.01;  // ~0.57°
```

---

## Error Handling

**All gizmo functions are infallible** - they return sensible defaults on invalid input:

- **Invalid mouse delta** → `Vec3::ZERO` or `Quat::IDENTITY`
- **Invalid constraint** → Uses `None` constraint
- **Scale out of bounds** → Clamped to `[0.01, 100.0]`
- **Planar constraint on rotation** → Returns `Quat::IDENTITY`

**No panic conditions exist** in the gizmo system.

---

## Thread Safety

All types are **`Send + Sync`** (can be used across threads):

- `GizmoState` - Safe (no interior mutability)
- `TranslateGizmo`, `RotateGizmo`, `ScaleGizmo` - Safe (stateless)
- `GizmoRenderer` - Safe (stateless)
- `GizmoPicker` - Safe (immutable threshold)
- `CameraController` - Safe (POD data)
- `Transform` - Safe (POD data)

**Note**: `GizmoState` requires `&mut` for mutations, preventing concurrent modification.

---

## Next Steps

- **User Guide**: See `GIZMO_USER_GUIDE.md` for tutorials
- **Examples**: See `GIZMO_EXAMPLES.md` for complete workflows
- **Architecture**: See `GIZMO_ARCHITECTURE.md` for system design
- **Benchmarks**: See `ASTRACT_GIZMO_DAY_12_COMPLETE.md` for performance data

---

**End of API Reference**
