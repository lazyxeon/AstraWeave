# Physics API Reference

> **Crate**: `astraweave-physics`  
> **Coverage**: ~82%  
> **Tests**: 657+

Physics simulation built on Rapier3D with character controllers, vehicles, destruction, and deterministic replay support.

## Quick Links

- [rustdoc](https://docs.rs/astraweave-physics) (when published)
- [Source](https://github.com/astraweave/astraweave/tree/main/astraweave-physics)
- [Physics System Guide](../core-systems/physics.md)

---

## Core Types

### PhysicsWorld

Central physics simulation container.

```rust
use astraweave_physics::{PhysicsWorld, RigidBodyDesc, ColliderDesc};

let mut physics = PhysicsWorld::new();

// Create rigid body
let body = physics.create_rigid_body(RigidBodyDesc {
    position: Vec3::new(0.0, 5.0, 0.0),
    body_type: RigidBodyType::Dynamic,
    mass: 1.0,
    ..Default::default()
});

// Add collider
physics.add_collider(body, ColliderDesc::sphere(0.5));

// Step simulation
physics.step(1.0 / 60.0);
```

---

### CharacterController

Physics-based character movement with slopes, stairs, and ground detection.

```rust
use astraweave_physics::CharacterController;

let mut controller = CharacterController::new(CharacterConfig {
    height: 1.8,
    radius: 0.3,
    step_height: 0.3,
    max_slope: 45.0,
    ..Default::default()
});

// Move character
let movement = controller.move_character(
    &physics,
    desired_velocity,
    delta_time,
);

// Check ground state
if controller.is_grounded() {
    // Can jump
}
```

---

### Vehicle

Vehicle physics with tire model and suspension.

```rust
use astraweave_physics::vehicle::{Vehicle, VehicleConfig, WheelConfig};

let vehicle = Vehicle::new(VehicleConfig {
    chassis_mass: 1500.0,
    wheels: vec![
        WheelConfig { position: Vec3::new(-0.8, 0.0, 1.5), ..Default::default() },
        WheelConfig { position: Vec3::new(0.8, 0.0, 1.5), ..Default::default() },
        WheelConfig { position: Vec3::new(-0.8, 0.0, -1.5), ..Default::default() },
        WheelConfig { position: Vec3::new(0.8, 0.0, -1.5), ..Default::default() },
    ],
    ..Default::default()
});

// Apply input
vehicle.set_steering(steering_angle);
vehicle.set_throttle(throttle);
vehicle.set_brake(brake);
```

---

### SpatialHash

Efficient broad-phase collision detection.

```rust
use astraweave_physics::SpatialHash;

let mut spatial = SpatialHash::new(10.0); // Cell size

// Insert objects
spatial.insert(entity_a, aabb_a);
spatial.insert(entity_b, aabb_b);

// Query nearby objects
let nearby = spatial.query_sphere(position, radius);

// Query potential collisions
let pairs = spatial.get_collision_pairs();
```

**Performance**: 99.96% collision check reduction vs brute force

---

### Ragdoll

Ragdoll physics with joint constraints.

```rust
use astraweave_physics::ragdoll::{Ragdoll, RagdollConfig, BoneConfig};

let ragdoll = Ragdoll::new(RagdollConfig {
    bones: vec![
        BoneConfig { name: "spine", parent: None, .. },
        BoneConfig { name: "head", parent: Some(0), .. },
        BoneConfig { name: "arm_l", parent: Some(0), .. },
        // ...
    ],
    ..Default::default()
});

// Activate ragdoll on character death
ragdoll.activate(&mut physics, character_pose);
```

---

### Cloth

Cloth simulation with wind and collision.

```rust
use astraweave_physics::cloth::{Cloth, ClothConfig};

let cloth = Cloth::new(ClothConfig {
    width: 10,
    height: 10,
    particle_mass: 0.1,
    stiffness: 1000.0,
    damping: 0.1,
    ..Default::default()
});

// Pin corners
cloth.pin_particle(0, 0);
cloth.pin_particle(9, 0);

// Apply wind
cloth.apply_force(wind_force);
```

---

### Destruction

Destructible objects with fracturing.

```rust
use astraweave_physics::destruction::{Destructible, FractureConfig};

let destructible = Destructible::new(FractureConfig {
    health: 100.0,
    fracture_threshold: 50.0,
    piece_count: 8,
    ..Default::default()
});

// Apply damage
let fragments = destructible.apply_damage(damage, hit_point)?;

// Spawn fragment entities
for fragment in fragments {
    commands.spawn(fragment);
}
```

---

### Projectile

Projectile physics with penetration and ricochet.

```rust
use astraweave_physics::projectile::{Projectile, ProjectileConfig};

let projectile = Projectile::new(ProjectileConfig {
    velocity: Vec3::new(0.0, 0.0, 500.0),
    mass: 0.01,
    drag: 0.1,
    gravity_scale: 1.0,
    penetration_power: 50.0,
    ..Default::default()
});

// Step projectile
let hit = projectile.step(&physics, delta_time);
if let Some(hit_info) = hit {
    // Handle impact
}
```

---

## Collision Detection

### Raycasting

```rust
use astraweave_physics::RaycastResult;

// Single ray
if let Some(hit) = physics.raycast(origin, direction, max_distance) {
    println!("Hit at {:?}, normal: {:?}", hit.point, hit.normal);
}

// Ray with filter
let hit = physics.raycast_filtered(origin, direction, max_distance, |entity| {
    !entity.has::<Ghost>()
});
```

### Shape Casts

```rust
// Sphere cast
let hit = physics.spherecast(origin, radius, direction, max_distance);

// Box cast
let hit = physics.boxcast(origin, half_extents, direction, max_distance);
```

### Overlap Queries

```rust
// Get all entities in sphere
let entities = physics.overlap_sphere(center, radius);

// Get all entities in box
let entities = physics.overlap_box(center, half_extents, rotation);
```

---

## Collision Shapes

| Shape | Constructor | Use Case |
|-------|-------------|----------|
| Sphere | `ColliderDesc::sphere(radius)` | Characters, projectiles |
| Box | `ColliderDesc::cuboid(hx, hy, hz)` | Crates, buildings |
| Capsule | `ColliderDesc::capsule(half_height, radius)` | Characters |
| Cylinder | `ColliderDesc::cylinder(half_height, radius)` | Pillars |
| Convex Hull | `ColliderDesc::convex_hull(points)` | Complex objects |
| Trimesh | `ColliderDesc::trimesh(vertices, indices)` | Static geometry |
| Heightfield | `ColliderDesc::heightfield(heights, scale)` | Terrain |

---

## Collision Groups

```rust
use astraweave_physics::{CollisionGroups, Group};

// Define groups
const PLAYER: Group = Group::GROUP_1;
const ENEMY: Group = Group::GROUP_2;
const PROJECTILE: Group = Group::GROUP_3;
const ENVIRONMENT: Group = Group::GROUP_4;

// Player collides with enemies and environment
let player_groups = CollisionGroups::new(
    PLAYER,
    ENEMY | ENVIRONMENT,
);

// Projectile collides with everything
let projectile_groups = CollisionGroups::new(
    PROJECTILE,
    PLAYER | ENEMY | ENVIRONMENT,
);
```

---

## Determinism

AstraWeave physics is fully deterministic for replay and networking:

```rust
use astraweave_physics::PhysicsWorld;

// Same seed = same results
let mut physics_a = PhysicsWorld::with_seed(12345);
let mut physics_b = PhysicsWorld::with_seed(12345);

// Run simulation
for _ in 0..1000 {
    physics_a.step(1.0 / 60.0);
    physics_b.step(1.0 / 60.0);
}

// Results are bit-identical
assert_eq!(physics_a.checksum(), physics_b.checksum());
```

---

## Performance

| Operation | Latency | Notes |
|-----------|---------|-------|
| Rigid body step | 2.97 µs | Single body |
| Full physics tick | 6.52 µs | 100 bodies |
| Character move | 114 ns | Controller step |
| Raycast | ~500 ns | Typical scene |
| Spatial hash query | ~50 ns | Per cell |

---

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `debug-render` | Physics visualization | ❌ |
| `simd` | SIMD acceleration | ✅ |
| `parallel` | Parallel simulation | ✅ |
| `serialize` | State serialization | ❌ |

---

## See Also

- [Physics System Guide](../core-systems/physics.md)
- [Character Controller](../core-systems/character-controller.md)
- [Fluids System](./fluids.md)
- [Terrain Integration](../core-systems/terrain.md#physics-integration)
