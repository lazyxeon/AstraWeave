# Physics Demo Walkthrough

The `physics_demo3d` example showcases AstraWeave's **Rapier3D-based physics system** with character controllers, destructible objects, water buoyancy, wind forces, and real-time rendering.

## Running the Example

```bash
cargo run -p physics_demo3d --release
```

## What It Demonstrates

- **Character controller**: Physics-driven movement with climb detection
- **Destructible objects**: Boxes that break under force
- **Water simulation**: Buoyancy and drag in water volumes
- **Wind forces**: Environmental wind affecting dynamic objects
- **Collision layers**: Filtering which objects collide
- **Real-time rendering**: wgpu-based visualization

## Controls

### Camera

| Key | Action |
|-----|--------|
| `W/A/S/D` | Move camera |
| `Space` | Camera up |
| `Shift` | Camera down |
| Right-click + drag | Look around |
| `Esc` | Exit |

### Character

| Key | Action |
|-----|--------|
| `I/J/K/L` | Move character (forward/left/back/right) |
| `C` | Attempt climb (hold near climbable surface) |

### Physics

| Key | Action |
|-----|--------|
| `F` | Spawn dynamic box at (0, 4, 0) |
| `N` | Spawn destructible box |
| `M` | Break last destructible |
| `B` | Spawn ragdoll placeholder |
| `T` | Toggle wind on/off |
| `G` | Toggle water volume on/off |

## Expected Behavior

When you run the demo:

1. **Initial scene**: Ground plane, wall, character (green), water volume, destructible box
2. **Press `J/L`**: Character slides left/right with physics
3. **Press `F`**: Box drops and bounces
4. **Press `T`**: Wind pushes boxes in the wind direction
5. **Press `G`**: Water volume toggles, affecting buoyancy
6. **Press `M`**: Destructible box shatters into fragments

## Code Walkthrough

### 1. Physics World Setup

```rust
let mut phys = PhysicsWorld::new(vec3(0.0, -9.81, 0.0));
let _ground = phys.create_ground_plane(vec3(100.0, 0.0, 100.0), 1.0);
```

- Creates a physics world with Earth gravity (9.81 m/s²)
- Adds a ground plane at Y=0

### 2. Static Geometry

```rust
let _wall = phys.add_static_trimesh(
    &[
        vec3(5.0, 0.0, 0.0),
        vec3(5.0, 3.0, 0.0),
        // ... vertices
    ],
    &[[0, 1, 2], [3, 2, 1]],  // Triangle indices
    Layers::CHARACTER | Layers::DEFAULT,
);
```

Static meshes (walls, floors) don't move but participate in collisions. The `Layers` flags control which objects can collide.

### 3. Character Controller

```rust
let char_id = phys.add_character(
    vec3(-2.0, 1.0, 0.0),  // Start position
    vec3(0.4, 0.9, 0.4),   // Capsule half-extents
);
```

The character controller:
- Handles walking on slopes (up to 50° by default)
- Provides ground detection and step climbing
- Prevents tunneling through walls

### 4. Character Movement

```rust
// In the game loop:
let desired = vec3(self.move_dir.x, 0.0, self.move_dir.z);
self.phys.control_character(self.char_id, desired, dt, self.climb_try);
```

The controller converts desired velocity into physics-respecting movement.

### 5. Destructible Objects

```rust
let id = phys.add_destructible_box(
    vec3(-1.0, 1.0, 2.0),   // Position
    vec3(0.4, 0.4, 0.4),    // Half-size
    3.0,                     // Mass (kg)
    50.0,                    // Break force threshold
    12.0,                    // Fragment mass
);
```

When break force is exceeded:
```rust
self.phys.break_destructible(id);  // Shatters into fragments
```

### 6. Water Volume

```rust
phys.add_water_aabb(
    vec3(-2.0, 0.0, -2.0),  // Min corner
    vec3(2.0, 1.2, 2.0),    // Max corner
    1000.0,                  // Water density (kg/m³)
    0.8,                     // Drag coefficient
);
```

Objects inside the volume experience:
- **Buoyancy**: Upward force proportional to submerged volume
- **Drag**: Velocity-dependent resistance

### 7. Wind Forces

```rust
self.phys.set_wind(
    vec3(1.0, 0.0, 0.2).normalize(),  // Direction
    8.0,                               // Strength
);
```

Wind applies continuous force to dynamic bodies.

### 8. Rendering Loop

```rust
fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
    // Update physics
    self.phys.step();
    
    // Sync render instances from physics transforms
    self.instances.clear();
    for (handle, _body) in self.phys.bodies.iter() {
        if let Some(m) = self.phys.body_transform(id) {
            self.instances.push(Instance { transform: m, ... });
        }
    }
    
    // Render
    renderer.update_instances(&self.instances);
    renderer.render()?;
}
```

## Physics Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    PhysicsWorld                         │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │ Rigid Bodies│  │ Colliders   │  │ Constraints     │ │
│  │ (dynamic)   │  │ (shapes)    │  │ (joints)        │ │
│  └─────────────┘  └─────────────┘  └─────────────────┘ │
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │ Character   │  │ Water       │  │ Wind/Forces     │ │
│  │ Controller  │  │ Volumes     │  │                 │ │
│  └─────────────┘  └─────────────┘  └─────────────────┘ │
│                                                         │
│  Backend: Rapier3D 0.22                                 │
└─────────────────────────────────────────────────────────┘
```

## Collision Layers

AstraWeave uses bitflag layers to control collisions:

```rust
pub struct Layers: u32 {
    const DEFAULT   = 0b0001;  // General objects
    const CHARACTER = 0b0010;  // Player/NPC capsules
    const PROJECTILE= 0b0100;  // Bullets, arrows
    const TRIGGER   = 0b1000;  // Non-solid triggers
}
```

Objects only collide if their layer masks overlap.

## Performance Notes

The physics demo runs at 60 FPS with:
- ~50 dynamic bodies
- Character controller
- Water volume intersection tests
- Wind force application

For larger simulations, consider:
- Reducing simulation substeps
- Using simpler collision shapes
- Spatial partitioning for many bodies

## Related Examples

- [Fluids Demo](./fluids-demo.md) - Particle-based fluid simulation
- [Navmesh Demo](./navmesh-demo.md) - Pathfinding on terrain
- [Unified Showcase](./unified-showcase.md) - Rendering pipeline

## Troubleshooting

### Character falls through floor
Ensure the ground plane is created before spawning the character.

### Objects don't collide
Check that collision layers overlap between the objects.

### Low framerate
Reduce the number of dynamic bodies or increase the physics timestep.

## Source Location

- **Example**: `examples/physics_demo3d/src/main.rs` (301 lines)
- **Physics**: `astraweave-physics/src/lib.rs`
- **Character Controller**: `astraweave-physics/src/character_controller.rs`
