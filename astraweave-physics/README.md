# astraweave-physics

Full-featured physics simulation for AstraWeave, wrapping **Rapier3D 0.22**.

## Overview

Provides rigid body dynamics, character controllers, broadphase spatial hashing, projectiles, ragdolls, vehicles, cloth, gravity zones, wind/water environments, and destructible objects.

## Key Types

| Type | Description |
|------|-------------|
| `PhysicsWorld` | Central simulation (bodies, colliders, joints, queries) |
| `CharacterController` | Kinematic movement with ground detection |
| `SpatialHash` | Dormant in-crate grid (advertised "99.96% pair reduction"); the active broadphase is Rapier's `DefaultBroadPhase`, not `SpatialHash` |
| `ProjectileManager` | Ballistic projectiles with penetration/explosions |
| `Ragdoll` / `RagdollBuilder` | Multi-bone ragdoll creation |
| `Vehicle` / `VehicleManager` | Vehicle dynamics with drivetrain |
| `ClothManager` | Particle-based cloth with distance constraints |
| `DestructionManager` | Fracture patterns and debris |
| `EnvironmentManager` | Wind zones, water volumes, buoyancy |
| `GravityManager` | Composable gravity zones |

## Feature Flags

| Feature | Description |
|---------|-------------|
| `async-physics` | 3-stage parallel pipeline via Rayon |
| `profiling` | Tracy integration |
| `ecs` | ECS system integration |

## Performance

- Character move: 114 ns
- Full physics tick: 6.52 µs
- Rigid body step: 2.97 µs
- Spatial hash: 3.77 ms (FxHashMap)

## License

MIT
