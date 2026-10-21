# Physics System

The AstraWeave physics system provides comprehensive 3D physics simulation through integration with [Rapier](https://rapier.rs/), a high-performance physics engine written in Rust.

<!--
  Reconciliation note added 2026-05-15 (trace campaign).
  Source: ARCHITECTURE_MAP.md §7.2 doc-comment migration drift, physics.md §1, §6, §11.
  The lib.rs:25-26 doc-comment advertises SpatialHash as the broadphase with
  "99.96% pair reduction"; the actual broadphase is Rapier's DefaultBroadPhase.
  SpatialHash (1,038 LoC) is dormant in production — test-file-only consumers,
  zero benches. Tracked as Q19 in §14.
-->

```admonish warning title="Doc-comment drift: SpatialHash is not the broadphase"
The `astraweave-physics` crate-level doc-comment (`lib.rs:25-26`) advertises a
`SpatialHash` broadphase claiming "99.96% pair reduction vs brute-force." That
module exists (1,038 LoC) but is **dormant in production** — the actual broadphase
is Rapier's `DefaultBroadPhase` (`lib.rs:907`). `SpatialHash` is consumed only by
test files (4 test files / 33 `#[test]` attributes; zero benches). Tracked as Q19
in `ARCHITECTURE_MAP.md` §14. See [`physics.md`](https://github.com/lazyxeon/AstraWeave/blob/main/docs/architecture/physics.md) §6 for the full trap analysis.

Other documented stubs in the same crate: `process_destructible_hits` (no-op,
zero callers), `add_water_aabb` (no-op stub with `{}` body), `add_destructible_box`
ignores its `_health` / `_break_impulse` parameters.
```

## Overview

```admonish info
The physics system runs at a fixed 60Hz tick rate to ensure deterministic simulation.
```

Key features:

- **Rigid Body Dynamics** - Full 3D rigid body simulation
- **Character Controllers** - Player and NPC movement with collision
- **Collision Detection** - Broad and narrow phase collision
- **Spatial Queries** - Raycasting, shape casting, overlap tests
- **Joints & Constraints** - Connect bodies with various joint types
- **Continuous Collision Detection** - Prevents tunneling for fast objects

## Architecture

```mermaid
graph TB
    subgraph Physics Pipeline
        A[Collect Inputs] --> B[Broad Phase]
        B --> C[Narrow Phase]
        C --> D[Constraint Solver]
        D --> E[Position Integration]
        E --> F[Sync to ECS]
    end
    
    G[ECS World] --> A
    F --> G
```

## Core Components

### RigidBody

Represents a physics-simulated body:

```rust
use astraweave_physics::{RigidBody, RigidBodyType};

let body = RigidBody {
    body_type: RigidBodyType::Dynamic,
    mass: 1.0,
    linear_damping: 0.1,
    angular_damping: 0.1,
    gravity_scale: 1.0,
    ..Default::default()
};
```

Body types:
- `Dynamic` - Fully simulated, responds to forces
- `Static` - Immovable, used for environment
- `Kinematic` - Moved by code, pushes dynamic bodies

### Collider

Defines collision shapes:

```rust
use astraweave_physics::{Collider, ColliderShape};

let collider = Collider {
    shape: ColliderShape::Box { half_extents: Vec3::new(1.0, 1.0, 1.0) },
    friction: 0.5,
    restitution: 0.3,
    sensor: false,
    ..Default::default()
};
```

Available shapes:
- `Box` - Axis-aligned box
- `Sphere` - Perfect sphere
- `Capsule` - Cylinder with spherical caps
- `Cylinder` - Circular cylinder
- `ConvexHull` - Convex mesh
- `TriMesh` - Triangle mesh (static only)
- `HeightField` - Terrain heightmap

### CharacterController

Handles player/NPC movement:

```rust
use astraweave_physics::CharacterController;

let controller = CharacterController {
    height: 1.8,
    radius: 0.3,
    step_height: 0.3,
    max_slope: 45.0_f32.to_radians(),
    ..Default::default()
};
```

## Spatial Queries

### Raycasting

```rust
use astraweave_physics::{RaycastQuery, RaycastHit};

let query = RaycastQuery {
    origin: Vec3::new(0.0, 5.0, 0.0),
    direction: Vec3::NEG_Y,
    max_distance: 100.0,
    filter: CollisionFilter::default(),
};

if let Some(hit) = physics.raycast(&query) {
    println!("Hit entity {:?} at distance {}", hit.entity, hit.distance);
}
```

### Shape Casting

```rust
use astraweave_physics::{ShapeCastQuery, ColliderShape};

let query = ShapeCastQuery {
    shape: ColliderShape::Sphere { radius: 0.5 },
    origin: start_pos,
    direction: velocity.normalize(),
    max_distance: velocity.length(),
};

let hits = physics.shape_cast(&query);
```

### Overlap Tests

```rust
use astraweave_physics::OverlapQuery;

let query = OverlapQuery {
    shape: ColliderShape::Sphere { radius: 5.0 },
    position: explosion_center,
    filter: CollisionFilter::default(),
};

for entity in physics.overlap(&query) {
    apply_explosion_damage(entity);
}
```

## Collision Filtering

Control which objects can collide:

```rust
use astraweave_physics::{CollisionFilter, CollisionGroup};

let player_filter = CollisionFilter {
    membership: CollisionGroup::PLAYER,
    filter: CollisionGroup::WORLD | CollisionGroup::ENEMY | CollisionGroup::PROJECTILE,
};
```

## Performance Optimization

### Spatial Hash

The physics system uses a spatial hash for broad-phase acceleration:

```rust
use astraweave_physics::SpatialHash;

let spatial = SpatialHash::new(10.0);
spatial.insert(entity, aabb);

let nearby = spatial.query_aabb(&query_aabb);
```

### Async Scheduling

For large simulations, physics can run asynchronously:

```rust
use astraweave_physics::AsyncPhysicsScheduler;

let scheduler = AsyncPhysicsScheduler::new(4);
scheduler.step(&mut physics_world, delta_time).await;
```

## Integration with ECS

Physics components sync automatically with ECS transforms:

```rust
fn physics_sync_system(
    query: Query<(&mut Transform, &RigidBodyHandle)>,
    physics: Res<PhysicsWorld>,
) {
    for (mut transform, handle) in query.iter_mut() {
        if let Some(body) = physics.get_body(*handle) {
            transform.translation = body.position();
            transform.rotation = body.rotation();
        }
    }
}
```

## See Also

- [API Documentation](../api/index.md) - `astraweave_physics` API
- [Character Controller Tutorial](../game-dev/first-game.md)
- [Navigation System](./navigation.md) - Pathfinding integration
- [Deterministic Simulation](../architecture/deterministic.md)
