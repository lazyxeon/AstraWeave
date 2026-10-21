# Aravath greybox contract

The voxel grid is the only ground. `DualContouring` in `astraweave-terrain` remeshes dirty chunks into one trimesh and one render mesh. Do not add a second mesher, and do not fill a 10 km voxel volume the way `examples/hybrid_voxel_demo` does.

`ThirdPersonCamera` is a `CameraProducer`. The trait method is world-relative. `to_render_view_camera_relative` is the opt-in large-world form, same split as `FreeFly`. Occlusion is a physics ray in `aravath_runtime`; the camera crate does not depend on Rapier.

`PhysicsWorld::remove_body` drops a body and its colliders. Chunk replacement is `remove_body` then `add_static_trimesh`. `break_destructible` calls `remove_body`.

Game code lives in `aravath_runtime`. It does not depend on `astraweave-weaving`.

## Phases

1. Stand and dig. Done when the headless tests pass: the body rests on the island, a dig drops it, a clean sync keeps body ids, and a wall shortens the camera distance.
2. Walk, run, and roll spend an f32 stamina pool on the motor.
3. One locomotion graph over `astraweave-render` clips, with a capsule stand-in if no clip is authored.
4. One timed attack, one hit during the active window, stagger, lock-on, one behavior-tree enemy.
5. Place with a 6-neighbor support check, an edit-log save through `aw-save`, and a nav dirty region. Do not call `CPersistenceManager::save_game`.

Streaming, day/night gameplay, weapon movesets, poise, thread tiers, and companions wait until this loop exists.

Marching-cubes note: `DualContouring` emits one vertex per crossed edge, and cell corners use Bourke order so they match `MC_TRI_TABLE`. The previous cache stored a vertex per cell and looked it up by a corner, so a solid chunk produced hundreds of vertices and a single triangle.
