---
layout: default
title: Navigation Subsystem
---

# Navigation (astraweave-nav)

AstraWeave provides navmesh-based pathfinding with A* search, dynamic region invalidation, and partial rebaking. The crate has **216 tests**.

## Features

- **NavMesh baking**: Automatic generation from world triangles with step height and slope filtering
- **A* pathfinding**: Optimal path search on triangulated meshes
- **Dynamic invalidation**: Mark regions dirty and rebake only affected areas
- **Partial rebaking**: Incremental navmesh updates as the world changes
- **Rich triangle API**: Per-triangle normals, centers, areas, perimeters, degenerate detection
- **AABB queries**: Spatial containment and intersection tests

## Core Types

### NavMesh

```rust
use astraweave_nav::{NavMesh, Triangle};

// Bake a navmesh from triangles
let mesh = NavMesh::bake(&triangles, max_step, max_slope_deg);

// Find a path from start to goal
let path = mesh.find_path(start_pos, goal_pos);
```

### Triangle

The fundamental navmesh primitive with full geometric analysis:

```rust
pub struct Triangle {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
}
```

Methods: `new()`, `center()`, `normal()`, `area()`, `is_degenerate()`, `perimeter()`, `edge_lengths()`, `from_vertices()`.

### NavTri

A baked navigation triangle with connectivity information:

```rust
pub struct NavTri {
    pub idx: usize,
    pub verts: [Vec3; 3],
    pub normal: Vec3,
    pub center: Vec3,
    pub neighbors: Vec<usize>,
}
```

### Aabb

Axis-aligned bounding box for spatial queries:

```rust
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}
```

Methods: `contains()`, `intersects()`, `merge()`, `from_triangle()`, `center()`, `size()`, `volume()`, `expand()`.

### NavMesh

The baked navigation mesh built from world triangles:

```rust
use astraweave_nav::{NavMesh, Triangle};

let triangles = vec![
    Triangle::new(Vec3::ZERO, Vec3::X, Vec3::Z),
    // ... world geometry
];

// Bake with step height and slope constraints
let mesh = NavMesh::bake(&triangles, max_step, max_slope_deg);
```

## Pathfinding

A* search on the triangulated navmesh:

```rust
// Find the optimal path between two world-space points
let path: Vec<Vec3> = mesh.find_path(start_pos, goal_pos);
```

The pathfinder returns a sequence of waypoints through traversable navmesh triangles.

## Dynamic Region Invalidation

When the world changes (doors opening, bridges collapsing, terrain deformation), mark affected regions for rebaking:

```rust
// Mark a region as needing rebake
mesh.invalidate_region(affected_aabb);

// Check if any regions need reconstruction
if mesh.needs_rebake() {
    // Rebuild only the dirty portions
    mesh.rebake_dirty_regions(&updated_triangles);

    // Or do a partial targeted rebake
    mesh.partial_rebake(&new_triangles_in_region);
}
```

This avoids the cost of a full navmesh rebuild when only small portions of the world change.

## Integration with AI

AI agents use navigation through the WorldSnapshot perception pipeline:

```
PERCEPTION stage → Build WorldSnapshot (include obstacle/nav data)
    ↓
AI_PLANNING stage → Orchestrator queries obstacles for path validity
    ↓
ActionStep::MoveTo → Character controller follows planned waypoints
    ↓
PHYSICS stage → Character controller executes movement
```

The `WorldSnapshot.obstacles` field provides AI agents with obstacle positions for navigation-aware planning. `ActionStep::MoveTo` references world-space coordinates that correspond to navigable areas.

## Test Coverage

- **216 tests** covering baking, pathfinding, invalidation, edge cases, and stress scenarios

[← Back to Home](index.html) · [Architecture](architecture.html) · [Physics](physics.html)
