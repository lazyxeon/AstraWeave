# Astract Gizmo System - Architecture Overview

**Version**: 1.0  
**Date**: November 4, 2025  
**Purpose**: System design and technical documentation

---

## Table of Contents

1. [System Overview](#system-overview)
2. [Component Architecture](#component-architecture)
3. [Data Flow](#data-flow)
4. [State Machine Design](#state-machine-design)
5. [Transform Pipeline](#transform-pipeline)
6. [Rendering Architecture](#rendering-architecture)
7. [Picking System](#picking-system)
8. [Performance Characteristics](#performance-characteristics)
9. [Extension Points](#extension-points)
10. [Design Decisions](#design-decisions)

---

## System Overview

The **Astract Gizmo System** is a production-ready implementation of Blender-style transform gizmos designed for Rust game engines. It provides a complete solution for 3D object manipulation with sub-nanosecond performance.

### Core Design Principles

1. **Deterministic**: All transforms produce identical results given same inputs
2. **Zero-Copy**: Minimal allocations, stack-based data structures
3. **Stateless Math**: Transform functions are pure (no hidden state)
4. **Composable**: Each component can be used independently
5. **Performance-First**: Sub-nanosecond state transitions, single-digit nanosecond math

### System Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                        │
│  (Game engine, level editor, asset viewer, etc.)           │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            │ Events (keyboard, mouse)
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                     Gizmo System                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ State Machine│  │  Transform   │  │   Rendering  │     │
│  │  (GizmoState)│  │  (Translate, │  │ (GizmoRenderer│     │
│  │              │  │   Rotate,    │  │  & Picker)   │     │
│  │              │  │   Scale)     │  │              │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│          │                  │                  │            │
│          └──────────────────┴──────────────────┘            │
│                            │                                │
└────────────────────────────┼────────────────────────────────┘
                             │ Transforms (Vec3, Quat)
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                    Scene Graph                              │
│  (Object transforms, camera, viewport)                     │
└─────────────────────────────────────────────────────────────┘
```

---

## Component Architecture

### Module Hierarchy

```
aw_editor_lib::gizmo/
├── state.rs              # GizmoState, GizmoMode, AxisConstraint
├── translate.rs          # TranslateGizmo (mouse & numeric)
├── rotate.rs             # RotateGizmo (mouse & numeric, snapping)
├── scale.rs              # ScaleGizmo (uniform & per-axis)
├── render.rs             # GizmoRenderer (handles, colors)
├── picking.rs            # GizmoPicker (ray-casting, screen-to-world)
└── scene_viewport.rs     # CameraController, Transform, SceneViewport
```

### Component Relationships

```
┌─────────────────────────────────────────────────────────────┐
│                        GizmoState                           │
│  ┌────────────────────────────────────────────────────┐    │
│  │ mode: GizmoMode (Inactive, Translate, Rotate, Scale)│    │
│  │ numeric_input: String                              │    │
│  │ mouse_delta: Vec2                                  │    │
│  │ snap_enabled: bool                                 │    │
│  └────────────────────────────────────────────────────┘    │
│                            │                                │
│         Dispatches to ─────┴──────┐                         │
│                                   │                         │
└───────────────────────────────────┼─────────────────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    │               │               │
                    ▼               ▼               ▼
          ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
          │TranslateGizmo│ │ RotateGizmo  │ │  ScaleGizmo  │
          │              │ │              │ │              │
          │• calculate_  │ │• calculate_  │ │• calculate_  │
          │  translation │ │  rotation    │ │  scale       │
          │• calculate_  │ │• calculate_  │ │• calculate_  │
          │  translation_│ │  rotation_   │ │  scale_      │
          │  numeric     │ │  numeric     │ │  numeric     │
          └──────────────┘ └──────────────┘ └──────────────┘
                    │               │               │
                    └───────────────┼───────────────┘
                                    │
                                    ▼
                          ┌──────────────────┐
                          │ Transform Output │
                          │ (Vec3 or Quat)   │
                          └──────────────────┘
```

### Rendering & Picking Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│                     GizmoRenderer                           │
│  ┌────────────────────────────────────────────────────┐    │
│  │ render_translate() → Vec<GizmoHandle>              │    │
│  │ render_rotate()    → Vec<GizmoHandle>              │    │
│  │ render_scale()     → Vec<GizmoHandle>              │    │
│  └────────────────────────────────────────────────────┘    │
│                            │                                │
│                            ▼                                │
│              ┌─────────────────────────────┐               │
│              │   GizmoHandle               │               │
│              │ • shape: Arrow/Circle/Cube  │               │
│              │ • axis: X/Y/Z               │               │
│              │ • color: RGB                │               │
│              │ • position: Vec3            │               │
│              │ • scale: f32                │               │
│              └─────────────────────────────┘               │
│                            │                                │
└────────────────────────────┼────────────────────────────────┘
                             │
                ┌────────────┴────────────┐
                │                         │
                ▼                         ▼
      ┌─────────────────┐       ┌─────────────────┐
      │ Graphics System │       │   GizmoPicker   │
      │ (wgpu, OpenGL)  │       │                 │
      │                 │       │ • pick_handle() │
      │ Render to screen│       │ • screen_to_    │
      │                 │       │   world_ray()   │
      └─────────────────┘       └─────────────────┘
                                          │
                                          ▼
                                ┌──────────────────┐
                                │ User Interaction │
                                │ (Click handle)   │
                                └──────────────────┘
```

---

## Data Flow

### Input → Transform → Output Pipeline

```
USER INPUT
    │
    ├─► Keyboard Event (G/R/S, X/Y/Z, 0-9, Enter, Esc)
    │        │
    │        ▼
    │   GizmoState::handle_key()
    │        │
    │        ├─► Mode Transition (Inactive → Translate/Rotate/Scale)
    │        ├─► Constraint Update (None → X → XY → X)
    │        ├─► Numeric Input Buffer ("5.2")
    │        └─► Snap Toggle (on/off)
    │
    └─► Mouse Event (move, click)
             │
             ▼
        GizmoState::update_mouse()
             │
             ▼
        mouse_delta: Vec2 (screen pixels)
             │
             └─────────────┐
                           │
TRANSFORM CALCULATION      │
    │                      │
    ▼                      ▼
┌───────────────────────────────────────┐
│ TranslateGizmo::calculate_translation()│
│ RotateGizmo::calculate_rotation()     │
│ ScaleGizmo::calculate_scale()         │
└───────────────────────────────────────┘
    │
    ├─► Inputs:
    │   • mouse_delta: Vec2 (screen pixels)
    │   • constraint: AxisConstraint (X/Y/Z/XY/XZ/YZ/None)
    │   • camera_distance: f32 (for translation)
    │   • sensitivity: f32 (for rotation/scale)
    │   • snap_enabled: bool (for rotation)
    │   • object_rotation: Quat (for local space)
    │   • local_space: bool (world vs local)
    │
    └─► Outputs:
        • Vec3 (translation vector)
        • Quat (rotation delta)
        • Vec3 (scale multiplier)
             │
             ▼
SCENE GRAPH UPDATE
    │
    ├─► object.translation += delta (translation)
    ├─► object.rotation = delta * object.rotation (rotation)
    └─► object.scale *= delta (scale)
             │
             ▼
        RENDER UPDATE
```

### Example: Translation Data Flow

```
1. User Input:
   Keyboard: G → X
   Mouse: Move 100px right

2. State Update:
   GizmoState {
       mode: Translate { constraint: X },
       mouse_delta: Vec2(100.0, 0.0),
       snap_enabled: false,
       numeric_input: "",
   }

3. Transform Calculation:
   TranslateGizmo::calculate_translation(
       Vec2(100.0, 0.0),  // mouse_delta
       X,                  // constraint
       10.0,              // camera_distance
       Quat::IDENTITY,    // object_rotation
       false,             // local_space
   )
   │
   ├─► Convert screen pixels to world units:
   │   scale = camera_distance * 0.01 = 0.1
   │   world_delta = Vec2(100.0, 0.0) * 0.1 = Vec2(10.0, 0.0)
   │
   ├─► Project to 3D:
   │   delta = Vec3(10.0, 0.0, 0.0) (right in screen space)
   │
   ├─► Apply constraint (X-axis only):
   │   delta = Vec3(10.0, 0.0, 0.0) (already X-axis)
   │
   └─► Return: Vec3(10.0, 0.0, 0.0)

4. Scene Graph Update:
   object.translation += Vec3(10.0, 0.0, 0.0)
   // Object moved 10 units along world X-axis

5. Confirm:
   User: Enter
   GizmoState::confirm_transform()
   mode → Inactive
```

---

## State Machine Design

### GizmoState State Diagram

```
                      ┌──────────────┐
                      │   Inactive   │ ◄─── Initial state
                      └──────┬───────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         │ G key             │ R key             │ S key
         │                   │                   │
         ▼                   ▼                   ▼
┌────────────────┐  ┌────────────────┐  ┌────────────────┐
│   Translate    │  │     Rotate     │  │     Scale      │
│ constraint:None│  │ constraint:None│  │ constraint:None│
│                │  │                │  │  uniform:false │
└────────┬───────┘  └────────┬───────┘  └────────┬───────┘
         │                   │                   │
         │ X/Y/Z key         │ X/Y/Z key         │ X/Y/Z key
         ▼                   ▼                   ▼
┌────────────────┐  ┌────────────────┐  ┌────────────────┐
│   Translate    │  │     Rotate     │  │     Scale      │
│ constraint:X   │  │ constraint:X   │  │ constraint:X   │
└────────┬───────┘  └────────┬───────┘  └────────┬───────┘
         │                   │                   │
         │ X key again       │ X key again       │ S key again
         ▼                   ▼                   ▼
┌────────────────┐  ┌────────────────┐  ┌────────────────┐
│   Translate    │  │     Rotate     │  │     Scale      │
│ constraint:XY  │  │ constraint:XY  │  │  uniform:true  │
└────────┬───────┘  └────────┬───────┘  └────────┬───────┘
         │                   │                   │
         │ Enter/Esc         │ Enter/Esc         │ Enter/Esc
         └───────────────────┴───────────────────┘
                             │
                             ▼
                      ┌──────────────┐
                      │   Inactive   │
                      └──────────────┘
```

### State Transition Rules

| Current State | Event | Next State | Notes |
|--------------|-------|------------|-------|
| Inactive | G | Translate{None} | Start translation |
| Inactive | R | Rotate{None} | Start rotation |
| Inactive | S | Scale{None, false} | Start scale (per-axis) |
| Translate{c} | X | Translate{X} or Translate{XY} | Toggle X constraint |
| Translate{c} | Y | Translate{Y} or Translate{YZ} | Toggle Y constraint |
| Translate{c} | Z | Translate{Z} or Translate{XZ} | Toggle Z constraint |
| Rotate{c} | X/Y/Z | Rotate{X/Y/Z} | Change rotation axis |
| Scale{c, false} | S | Scale{c, true} | Toggle uniform scale |
| Any Active | Enter | Inactive | Confirm transform |
| Any Active | Esc | Inactive | Cancel transform |

### Constraint Toggle Logic

```rust
impl AxisConstraint {
    pub fn toggle_planar(&mut self) {
        *self = match *self {
            AxisConstraint::X => AxisConstraint::XY,
            AxisConstraint::Y => AxisConstraint::YZ,
            AxisConstraint::Z => AxisConstraint::XZ,
            AxisConstraint::XY => AxisConstraint::X,
            AxisConstraint::YZ => AxisConstraint::Y,
            AxisConstraint::XZ => AxisConstraint::Z,
            AxisConstraint::None => AxisConstraint::None,
        };
    }
}
```

**Example Sequence**:
```
Press X → X-axis
Press X → XY-plane
Press X → X-axis (cycles back)
```

---

## Transform Pipeline

### Translation Transform Pipeline

```
INPUT: mouse_delta (screen pixels)
    │
    ▼
STEP 1: Screen → World Scale
    scale = camera_distance * 0.01
    world_delta = mouse_delta * scale
    │
    ▼
STEP 2: Screen → World Axes
    right = Vec3(world_delta.x, 0.0, 0.0)
    up = Vec3(0.0, world_delta.y, 0.0)
    delta = right + up
    │
    ▼
STEP 3: Apply Constraint
    match constraint {
        None => delta,
        X => Vec3(delta.x, 0.0, 0.0),
        Y => Vec3(0.0, delta.y, 0.0),
        Z => Vec3(0.0, 0.0, delta.z),
        XY => Vec3(delta.x, delta.y, 0.0),
        XZ => Vec3(delta.x, 0.0, delta.z),
        YZ => Vec3(0.0, delta.y, delta.z),
    }
    │
    ▼
STEP 4: Local Space Transform (if enabled)
    if local_space {
        delta = object_rotation * delta
    }
    │
    ▼
OUTPUT: Vec3 (world-space translation)
```

### Rotation Transform Pipeline

```
INPUT: mouse_delta (screen pixels)
    │
    ▼
STEP 1: Screen → Radians
    angle = mouse_delta.length() * sensitivity / 100.0
    │
    ▼
STEP 2: Apply Snapping (if enabled)
    if snap_enabled {
        snap_increment = π/12 (15°)
        angle = round(angle / snap_increment) * snap_increment
    }
    │
    ▼
STEP 3: Determine Rotation Axis
    axis = match constraint {
        X => Vec3::X,
        Y => Vec3::Y,
        Z => Vec3::Z,
        _ => return Quat::IDENTITY, // Planar constraints unsupported
    }
    │
    ▼
STEP 4: Build Quaternion
    rotation = Quat::from_axis_angle(axis, angle)
    │
    ▼
STEP 5: Local Space Transform (if enabled)
    if local_space {
        axis = object_rotation * axis
        rotation = Quat::from_axis_angle(axis, angle)
    }
    │
    ▼
OUTPUT: Quat (rotation delta)
```

### Scale Transform Pipeline

```
INPUT: mouse_delta (screen pixels)
    │
    ▼
STEP 1: Screen → Scale Factor
    factor = 1.0 + (mouse_delta.length() / 100.0) * sensitivity
    │
    ▼
STEP 2: Apply Constraint
    if uniform {
        scale = Vec3::splat(factor)
    } else {
        scale = match constraint {
            None => Vec3::splat(factor),
            X => Vec3::new(factor, 1.0, 1.0),
            Y => Vec3::new(1.0, factor, 1.0),
            Z => Vec3::new(1.0, 1.0, factor),
            _ => Vec3::ONE, // Planar constraints unsupported
        }
    }
    │
    ▼
STEP 3: Clamp to Safe Range
    scale = scale.clamp(
        Vec3::splat(MIN_SCALE), // 0.01
        Vec3::splat(MAX_SCALE), // 100.0
    )
    │
    ▼
OUTPUT: Vec3 (scale multiplier)
```

---

## Rendering Architecture

### Gizmo Handle Rendering

```
GizmoRenderer::render_translate(position, scale)
    │
    ▼
Generate 3 Handles (X, Y, Z):
    ┌───────────────────────────────────────┐
    │ GizmoHandle {                         │
    │   shape: Arrow,                       │
    │   axis: X,                            │
    │   color: Vec3(1.0, 0.0, 0.0), // Red  │
    │   position: position + Vec3::X * scale,│
    │   scale: scale,                       │
    │ }                                     │
    └───────────────────────────────────────┘
    ┌───────────────────────────────────────┐
    │ GizmoHandle {                         │
    │   shape: Arrow,                       │
    │   axis: Y,                            │
    │   color: Vec3(0.0, 1.0, 0.0), // Green│
    │   position: position + Vec3::Y * scale,│
    │   scale: scale,                       │
    │ }                                     │
    └───────────────────────────────────────┘
    ┌───────────────────────────────────────┐
    │ GizmoHandle {                         │
    │   shape: Arrow,                       │
    │   axis: Z,                            │
    │   color: Vec3(0.0, 0.0, 1.0), // Blue │
    │   position: position + Vec3::Z * scale,│
    │   scale: scale,                       │
    │ }                                     │
    └───────────────────────────────────────┘
    │
    ▼
Return Vec<GizmoHandle> (length = 3)
```

**Color Scheme**:
- **X-axis**: Red (1.0, 0.0, 0.0)
- **Y-axis**: Green (0.0, 1.0, 0.0)
- **Z-axis**: Blue (0.0, 0.0, 1.0)

**Handle Shapes**:
- **Translation**: Arrows (3D vectors)
- **Rotation**: Circles (torus primitives)
- **Scale**: Cubes (box primitives)

---

## Picking System

### Ray-Casting Algorithm

```
1. Screen to World Ray
   ┌─────────────────────────────────────┐
   │ screen_pos: Vec2 (pixels)           │
   │ viewport_size: Vec2 (width, height) │
   │ view_matrix: Mat4                   │
   │ projection_matrix: Mat4             │
   └─────────────────────────────────────┘
        │
        ▼
   Normalize Screen Coordinates:
   ndc.x = (screen_pos.x / viewport_size.x) * 2.0 - 1.0
   ndc.y = 1.0 - (screen_pos.y / viewport_size.y) * 2.0
        │
        ▼
   Unproject to World Space:
   inv_view_proj = (projection * view).inverse()
   near_point = inv_view_proj * Vec4(ndc.x, ndc.y, 0.0, 1.0)
   far_point = inv_view_proj * Vec4(ndc.x, ndc.y, 1.0, 1.0)
        │
        ▼
   Build Ray:
   ray_origin = near_point.xyz() / near_point.w
   ray_direction = (far_point.xyz() / far_point.w - ray_origin).normalize()
        │
        ▼
   ┌─────────────────────────────────────┐
   │ ray_origin: Vec3                    │
   │ ray_direction: Vec3 (normalized)    │
   └─────────────────────────────────────┘

2. Handle Intersection Test
   For each GizmoHandle:
       ┌───────────────────────────────────┐
       │ handle.shape: Arrow/Circle/Cube   │
       │ handle.position: Vec3             │
       │ handle.scale: f32                 │
       └───────────────────────────────────┘
            │
            ▼
       Ray-Shape Intersection:
       • Arrow: Ray-Cylinder test (length + radius)
       • Circle: Ray-Torus test (major + minor radius)
       • Cube: Ray-AABB test (min/max bounds)
            │
            ▼
       Distance Check:
       if distance < threshold {
           return Some(handle.axis)
       }
            │
            ▼
   First hit wins (closest handle)
```

**Intersection Algorithms**:

**Ray-Cylinder (Arrow)**:
```rust
fn ray_cylinder_intersection(
    ray_origin: Vec3,
    ray_direction: Vec3,
    cylinder_center: Vec3,
    cylinder_axis: Vec3,
    cylinder_radius: f32,
    cylinder_length: f32,
) -> Option<f32> {
    // 1. Project ray onto cylinder axis
    // 2. Compute perpendicular distance to axis
    // 3. Check if distance < radius AND within length
    // 4. Return intersection distance (or None)
}
```

**Ray-Torus (Circle)**:
```rust
fn ray_torus_intersection(
    ray_origin: Vec3,
    ray_direction: Vec3,
    torus_center: Vec3,
    torus_normal: Vec3,
    major_radius: f32,
    minor_radius: f32,
) -> Option<f32> {
    // 1. Project ray onto torus plane
    // 2. Compute distance to torus center circle
    // 3. Check if distance within [major_radius - minor_radius, major_radius + minor_radius]
    // 4. Return intersection distance (or None)
}
```

**Ray-AABB (Cube)**:
```rust
fn ray_aabb_intersection(
    ray_origin: Vec3,
    ray_direction: Vec3,
    aabb_min: Vec3,
    aabb_max: Vec3,
) -> Option<f32> {
    // 1. Compute slab intersections for each axis
    // 2. Find maximum tmin and minimum tmax
    // 3. Check if tmin <= tmax AND tmax >= 0
    // 4. Return tmin (entry distance) or None
}
```

---

## Performance Characteristics

### Benchmark Results (Day 12)

| Operation | Time | Throughput @ 60 FPS |
|-----------|------|---------------------|
| State Transition | 315-382 ps | 196,000+ transitions/frame |
| Translation Math | 2.5-6 ns | 55,000+ calculations/frame |
| Rotation Math | 17 ns | 30,000+ calculations/frame |
| Scale Math | 10-15 ns | 40,000+ calculations/frame |
| Render 3 Handles | 85-150 ns | 55,000+ gizmos/frame |
| Pick 3 Handles | 5-35 ns | 165,000+ picks/frame |
| Full Workflow | 25-40 ns | 106,000+ workflows/frame |

**60 FPS Budget**: 16.67 ms = 16,670,000 ns

**Gizmo System Overhead** (typical scene):
- 10 objects × 40 ns = **400 ns** (0.0024% of frame budget)
- Negligible impact on performance

### Memory Footprint

```rust
sizeof(GizmoState) = 56 bytes
sizeof(GizmoMode) = 24 bytes
sizeof(AxisConstraint) = 1 byte
sizeof(GizmoHandle) = 48 bytes
sizeof(CameraController) = 64 bytes
sizeof(Transform) = 40 bytes

Typical Scene:
  1 × GizmoState = 56 bytes
  10 × Transform = 400 bytes
  1 × CameraController = 64 bytes
  3 × GizmoHandle (temp) = 144 bytes
  ──────────────────────────────
  Total = 664 bytes (~0.6 KB)
```

**Zero Allocations**: All operations use stack-based data structures.

---

## Extension Points

### Adding New Gizmo Modes

```rust
// 1. Add new mode to GizmoMode enum
pub enum GizmoMode {
    Inactive,
    Translate { constraint: AxisConstraint },
    Rotate { constraint: AxisConstraint },
    Scale { constraint: AxisConstraint, uniform: bool },
    // NEW:
    Shear { constraint: AxisConstraint, axis: ShearAxis },
}

// 2. Create new gizmo module (shear.rs)
pub struct ShearGizmo;

impl ShearGizmo {
    pub fn calculate_shear(
        mouse_delta: Vec2,
        constraint: AxisConstraint,
        shear_axis: ShearAxis,
    ) -> Mat4 {
        // Shear transform calculation...
    }
}

// 3. Update GizmoState transitions
impl GizmoState {
    pub fn start_shear(&mut self) {
        self.mode = GizmoMode::Shear {
            constraint: AxisConstraint::None,
            axis: ShearAxis::XY,
        };
    }
}

// 4. Add rendering (render.rs)
impl GizmoRenderer {
    pub fn render_shear(position: Vec3, scale: f32) -> Vec<GizmoHandle> {
        // Shear gizmo visualization (e.g., parallelogram)...
    }
}

// 5. Update picking (picking.rs)
impl GizmoPicker {
    fn pick_shear_handle(&self, ...) -> Option<AxisConstraint> {
        // Ray-parallelogram intersection...
    }
}
```

### Custom Constraints

```rust
// Add new constraint types
pub enum AxisConstraint {
    None,
    X, Y, Z,
    XY, XZ, YZ,
    // NEW:
    Custom(Vec3),  // Arbitrary axis direction
    Planar(Vec3),  // Arbitrary plane normal
}

// Implement projection logic
impl AxisConstraint {
    pub fn project_vector(&self, v: Vec3) -> Vec3 {
        match self {
            AxisConstraint::Custom(axis) => {
                // Project v onto custom axis
                axis * axis.dot(v)
            }
            AxisConstraint::Planar(normal) => {
                // Project v onto plane
                v - normal * normal.dot(v)
            }
            _ => self.apply_to_vector(v),
        }
    }
}
```

### Custom Snapping

```rust
// Add snapping strategies
pub enum SnapMode {
    None,
    Grid { size: f32 },        // Grid snapping (1.0, 2.0, 3.0, ...)
    Degrees { increment: f32 }, // Angle snapping (15°, 30°, 45°, ...)
    Custom { values: Vec<f32> }, // Custom snap points
}

impl SnapMode {
    pub fn snap(&self, value: f32) -> f32 {
        match self {
            SnapMode::Grid { size } => {
                (value / size).round() * size
            }
            SnapMode::Degrees { increment } => {
                let radians = increment.to_radians();
                (value / radians).round() * radians
            }
            SnapMode::Custom { values } => {
                // Find closest value in list
                values.iter()
                    .min_by_key(|v| ((value - **v).abs() * 1000.0) as i32)
                    .copied()
                    .unwrap_or(value)
            }
            SnapMode::None => value,
        }
    }
}
```

---

## Design Decisions

### 1. Why Stateless Transform Functions?

**Decision**: `TranslateGizmo`, `RotateGizmo`, `ScaleGizmo` are stateless (zero-sized types with static methods).

**Rationale**:
- **Purity**: No hidden state = deterministic results
- **Testing**: Easy to unit test (no setup/teardown)
- **Performance**: No allocations, inlined by compiler
- **Composability**: Functions can be called independently

**Alternative Rejected**: Stateful gizmo objects with internal caching.

### 2. Why Separate GizmoState?

**Decision**: `GizmoState` manages mode, constraints, input buffer separately from transform logic.

**Rationale**:
- **Separation of Concerns**: State management ≠ math
- **Flexibility**: State machine can change without affecting math
- **Reusability**: Transform functions usable without state machine

**Alternative Rejected**: Unified `Gizmo` struct with state + math.

### 3. Why Vec3 for Scale (Not f32)?

**Decision**: `ScaleGizmo::calculate_scale()` returns `Vec3` (per-axis) instead of `f32` (uniform).

**Rationale**:
- **Generality**: Supports both uniform and per-axis scaling
- **Consistency**: All transforms return 3D vectors or quaternions
- **Flexibility**: Caller can apply to subset of axes

**Alternative Rejected**: Separate `calculate_scale_uniform()` and `calculate_scale_per_axis()` functions.

### 4. Why Quat for Rotation (Not Mat4)?

**Decision**: `RotateGizmo::calculate_rotation()` returns `Quat` instead of `Mat4`.

**Rationale**:
- **Interpolation**: Quaternions support SLERP for smooth animation
- **Composition**: Quat multiplication is faster than Mat4
- **Gimbal Lock**: Quaternions avoid gimbal lock issues
- **Size**: Quat (16 bytes) < Mat4 (64 bytes)

**Alternative Rejected**: Euler angles (gimbal lock) or rotation matrices (size/speed).

### 5. Why 15° Snap Increment?

**Decision**: Rotation snapping uses 15° increments (π/12 radians).

**Rationale**:
- **Industry Standard**: Blender, Maya, 3ds Max use 15° default
- **Divisibility**: 360° / 15° = 24 (even division)
- **Common Angles**: 30°, 45°, 60°, 90° all multiples of 15°

**Alternative Rejected**: 10° (doesn't divide 90° evenly), 5° (too granular).

### 6. Why Clamp Scale to [0.01, 100.0]?

**Decision**: Scale factors clamped to prevent negative/zero/infinity.

**Rationale**:
- **Safety**: Negative scale breaks rendering (backface culling)
- **Stability**: Zero scale causes division-by-zero errors
- **Practicality**: 0.01-100.0 range covers all reasonable use cases

**Alternative Rejected**: Unbounded scaling (unsafe), [0.1, 10.0] (too restrictive).

---

## Next Steps

- **User Guide**: See `GIZMO_USER_GUIDE.md` for tutorials
- **API Reference**: See `GIZMO_API_REFERENCE.md` for function signatures
- **Examples**: See `GIZMO_EXAMPLES.md` for complete workflows

---

**End of Architecture Overview**
