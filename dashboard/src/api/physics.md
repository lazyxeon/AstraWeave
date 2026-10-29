# Physics API Reference

> **Crate**: `astraweave-physics`  
> **Manifest version**: `0.1.0`

`PhysicsWorld` wraps Rapier. `broad_phase` is `DefaultBroadPhase`. `RigidBodyBuilder`, `ColliderBuilder`, `RigidBodySet`, and `ColliderSet` are re-exported from `rapier3d::prelude`.

There is no `RigidBodyDesc`, `ColliderDesc`, or `CharacterConfig` type, and no `rigid_body`, `collider`, or `character_controller` module.

## PhysicsWorld

```rust
use astraweave_physics::PhysicsWorld;
use glam::vec3;

let mut physics = PhysicsWorld::new(vec3(0.0, -9.81, 0.0));
let _ground = physics.create_ground_plane(vec3(50.0, 0.0, 50.0), 1.0);
let _box = physics.add_dynamic_box(
    vec3(0.0, 2.0, 0.0),
    vec3(0.5, 0.5, 0.5),
    1.0,
    astraweave_physics::Layers::DEFAULT,
);
physics.step();
```

`new` takes `Vec3` gravity. `step(&mut self)` takes no `dt`. `create_ground_plane(&mut self, half: Vec3, friction: f32) -> BodyId`. `add_dynamic_box(&mut self, pos, half, mass, groups) -> BodyId`.

`raycast(&self, origin, direction, max_distance) -> Option<(Vec3, Vec3, Option<BodyId>, f32)>`.

## CharacterController

```rust
use astraweave_physics::CharacterController;

let controller = CharacterController::new(0.3, 1.8);
let _grounded = controller.is_grounded();
```

`new(radius: f32, height: f32)`. The initial `state` is `CharState::Grounded`, so `is_grounded` is true on a fresh controller. There is no `move_character` method on this struct.

## Modules

Public modules: `spatial_hash`, `async_scheduler`, `ecs`, `projectile`, `ragdoll`, `vehicle`, `environment`, `cloth`, `deformed`, `traversal`.

`SpatialHash::new(cell_size: f32)` and `insert` exist. `PhysicsWorld::step` does not call that helper. The step uses Rapier's broadphase.

## Features

`default = []`. Declared features: `async-physics`, `profiling`, `ecs`, `alloc-counter`, `fast-alloc`. There is no `debug-render`, `simd`, `parallel`, or `serialize` feature.

## See Also

- [Physics System Guide](../core-systems/physics.md)
- [Fluids System](./fluids.md)
- [Terrain](../core-systems/terrain.md)
