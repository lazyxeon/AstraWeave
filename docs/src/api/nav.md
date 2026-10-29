# Navigation API Reference

> **Crate**: `astraweave-nav`  
> **Manifest version**: `0.1.0`

`lib.rs` keeps `tiles` private and re-exports `NavTiles`, `NavScratch`, `NavChange`, and related notice helpers from that module. `NavMesh`, `Triangle`, `NavTri`, and `Aabb` are public in `lib.rs`.

There is no `NavMeshConfig`, `NavAgent`, `PortalGraph`, `ObstacleAvoidance`, or `CrowdManager` type, and no `navmesh`, `pathfinding`, `portal_graph`, or `agent` module.

## NavMesh

```rust
use astraweave_nav::{NavMesh, Triangle};
use glam::Vec3;

let tri = Triangle::new(Vec3::ZERO, Vec3::X, Vec3::Y);
let mesh = NavMesh::bake(&[tri], 0.5, 45.0);
let path = mesh.find_path(Vec3::ZERO, Vec3::X);
let _ = path;
```

`bake(tris, max_step, max_slope_deg) -> Self`. `find_path(&self, start: Vec3, goal: Vec3) -> Vec<Vec3>`. It does not return `Result`. `find_path` is not the hashed route.

## NavTiles

```rust
use astraweave_nav::{NavScratch, NavTiles};
use glam::Vec3;

let tiles = NavTiles::new(0.4, 45.0);
let mut scratch = NavScratch::new();
let count = tiles.query_path(&mut scratch, Vec3::ZERO, Vec3::X);
let _ = count;
```

`NavScratch::new` is a `const fn`. `query_path` returns `u16`. That call is the replayed path query.

## See Also

- [Navigation Guide](../core-systems/navigation.md)
- [AI Integration](../core-systems/ai/index.md)
- [Terrain](../core-systems/terrain.md)
