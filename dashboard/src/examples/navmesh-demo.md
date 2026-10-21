# Navmesh Demo Walkthrough

The `navmesh_demo` example demonstrates AstraWeave's **navigation mesh system** - baking walkable surfaces from geometry and finding optimal paths with slope constraints.

## Running the Example

```bash
cargo run -p navmesh_demo --release
```

## What It Demonstrates

- **NavMesh baking**: Converting triangle soup into walkable navigation data
- **Slope filtering**: Excluding steep surfaces from navigation
- **A* pathfinding**: Finding optimal routes across the mesh
- **Path visualization**: Rendering waypoints in 3D space

## Controls

| Key | Action |
|-----|--------|
| `W/A/S/D` | Move camera |
| `Space` | Camera up |
| `Shift` | Camera down |
| Right-click + drag | Look around |
| `Esc` | Exit |

## Expected Behavior

When you run the demo:

1. **Yellow spheres**: Triangle centers of the navigation mesh
2. **Green spheres**: Path waypoints from start to goal
3. **Scene geometry**: Flat area + ramp + elevated plateau

The path flows from bottom-left (-3.5, 0, -3.5) up the ramp to the plateau (11.5, 0.8, 0).

## Code Walkthrough

### 1. Define Walkable Geometry

```rust
let tris = vec![
    // Main floor (two triangles forming a quad)
    tri(vec3(-4.0, 0.0, -4.0), vec3(4.0, 0.0, -4.0), vec3(4.0, 0.0, 4.0)),
    tri(vec3(-4.0, 0.0, -4.0), vec3(4.0, 0.0, 4.0), vec3(-4.0, 0.0, 4.0)),
    
    // Ramp going up (two triangles)
    tri(vec3(4.0, 0.0, -1.0), vec3(8.0, 0.8, -1.0), vec3(8.0, 0.8, 1.0)),
    tri(vec3(4.0, 0.0, -1.0), vec3(8.0, 0.8, 1.0), vec3(4.0, 0.0, 1.0)),
    
    // Elevated plateau (two triangles)
    tri(vec3(8.0, 0.8, -1.0), vec3(12.0, 0.8, -1.0), vec3(12.0, 0.8, 1.0)),
    tri(vec3(8.0, 0.8, -1.0), vec3(12.0, 0.8, 1.0), vec3(8.0, 0.8, 1.0)),
];
```

The geometry describes:
- **Floor**: 8×8 meter flat area at Y=0
- **Ramp**: Inclined surface from Y=0 to Y=0.8
- **Plateau**: Elevated area at Y=0.8

### 2. Bake the NavMesh

```rust
let nav = NavMesh::bake(
    &tris,  // Triangle geometry
    0.4,    // Agent radius (meters)
    50.0,   // Max walkable slope (degrees)
);
```

The baking process:
1. **Voxelizes** triangles into a 3D grid
2. **Filters** steep surfaces (>50° rejected)
3. **Shrinks** walkable area by agent radius (0.4m)
4. **Generates** navigation polygon data

### 3. Find a Path

```rust
let start = vec3(-3.5, 0.0, -3.5);  // Bottom-left of floor
let goal = vec3(11.5, 0.8, 0.0);     // Center of plateau

let path = nav.find_path(start, goal);
```

The `find_path` function:
1. **Projects** start/goal onto nearest navmesh triangle
2. **Runs A*** across triangle adjacency graph
3. **Returns** waypoint list (`Vec<Vec3>`)

### 4. Visualize Results

```rust
// Show triangle centers (yellow)
for t in &nav.tris {
    instances.push(Instance::from_pos_scale_color(
        t.center + vec3(0.0, 0.05, 0.0),  // Slightly above surface
        vec3(0.1, 0.1, 0.1),              // Small sphere
        [0.7, 0.7, 0.3, 1.0],             // Yellow
    ));
}

// Show path waypoints (green)
for p in &path {
    instances.push(Instance::from_pos_scale_color(
        *p + vec3(0.0, 0.08, 0.0),
        vec3(0.12, 0.12, 0.12),
        [0.2, 1.0, 0.4, 1.0],             // Green
    ));
}
```

## NavMesh Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      NavMesh                            │
├─────────────────────────────────────────────────────────┤
│  Input: Vec<Triangle>                                   │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ Voxelization │→ │ Slope Filter │→ │ Region Build │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
│                                                         │
│  Output: tris: Vec<NavTri>, adjacency: HashMap          │
├─────────────────────────────────────────────────────────┤
│  find_path(start, goal) → Vec\<Vec3\>                   │
│  ├─ Project points to mesh                              │
│  ├─ A* search on triangle graph                         │
│  └─ String-pull path smoothing                          │
└─────────────────────────────────────────────────────────┘
```

## Slope Filtering

The `max_slope` parameter (50° in this example) determines walkability:

| Surface Angle | Walkable? (50° max) |
|---------------|---------------------|
| 0° (flat)     | ✅ Yes |
| 30° (gentle)  | ✅ Yes |
| 50° (steep)   | ✅ Yes (boundary) |
| 60° (cliff)   | ❌ No |
| 90° (wall)    | ❌ No |

Steep surfaces are excluded from the navigation graph.

## Agent Radius

The `agent_radius` (0.4m) shrinks walkable areas:

```
Before shrinking:        After shrinking:
┌────────────────┐      ┌────────────────┐
│                │      │   ┌────────┐   │
│  Walkable      │  →   │   │Walkable│   │
│                │      │   └────────┘   │
└────────────────┘      └────────────────┘
                           ↑ 0.4m margin
```

This prevents agents from clipping through walls.

## Key Types

### Triangle

```rust
pub struct Triangle {
    pub a: Vec3,  // First vertex
    pub b: Vec3,  // Second vertex
    pub c: Vec3,  // Third vertex
}
```

### NavTri (Internal)

```rust
pub struct NavTri {
    pub center: Vec3,           // Triangle centroid
    pub normal: Vec3,           // Surface normal
    pub neighbors: Vec<usize>,  // Adjacent triangle indices
}
```

## Performance Notes

NavMesh baking is an offline process:
- **Bake time**: ~10ms for simple geometry, seconds for complex levels
- **Runtime pathfinding**: <1ms for typical paths
- **Memory**: ~100 bytes per navigation triangle

For large worlds, consider:
- Pre-baking navmeshes at build time
- Hierarchical navigation (coarse + fine meshes)
- Path caching for frequently-used routes

## Related Examples

- [Physics Demo](./physics-demo.md) - Rapier3D integration
- [Hello Companion](./hello-companion.md) - AI using navigation
- [Unified Showcase](./unified-showcase.md) - Rendering pipeline

## Troubleshooting

### Path returns empty
- Check that start/goal are near the navmesh surface
- Verify the mesh is connected (no isolated islands)

### Agent walks through walls
- Increase `agent_radius` parameter
- Ensure wall geometry is included in navmesh input

### Steep ramps not walkable
- Increase `max_slope` angle (up to 89°)

## Source Location

- **Example**: `examples/navmesh_demo/src/main.rs` (196 lines)
- **NavMesh**: `astraweave-nav/src/navmesh.rs`
- **Pathfinding**: `astraweave-nav/src/pathfinder.rs`
