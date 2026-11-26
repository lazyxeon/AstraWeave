---
layout: default
title: Scene Subsystem
---

# Scene Management (astraweave-scene)

AstraWeave's scene system provides a hierarchical transform graph, node tree, world partitioning for large open worlds, async cell streaming, and GPU resource lifecycle management. The crate has **210 tests**.

## Modules

| Module | Description |
|--------|-------------|
| `lib.rs` | Transform, Node, Scene core types |
| `world_partition` | Grid-based spatial partitioning |
| `streaming` | Async cell streaming with distance-based loading |
| `gpu_resource_manager` | GPU resource lifecycle (buffers, textures) per cell |
| `partitioned_scene` | Partitioned scene integration |
| `error` | `SceneError` / `SceneResult` types |
| `mutation_tests` | Mutation testing module |

## Feature Flags

| Feature | Description |
|---------|-------------|
| `ecs` | ECS system integration types |
| `world-partition` | Spatial partitioning and streaming |

## Core Types

### Transform

Translation, rotation, and scale applied hierarchically. Supports matrix conversion, interpolation, composition, and inverse:

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Transform {
    pub translation: Vec3,   // NOT "position"
    pub rotation: Quat,
    pub scale: Vec3,
}
```

**Constructors:**

| Method | Description |
|--------|-------------|
| `Transform::new(translation, rotation, scale)` | Full constructor |
| `Transform::identity()` | Identity transform |
| `Transform::from_translation(v)` | Translation only |
| `Transform::from_rotation(q)` | Rotation only |
| `Transform::from_scale(f32)` | Uniform scale |
| `Transform::from_scale_vec(Vec3)` | Non-uniform scale |

**Operations:**

| Method | Description |
|--------|-------------|
| `matrix() → Mat4` | 4×4 transformation matrix |
| `inverse() → Transform` | Inverse transform |
| `transform_point(Vec3) → Vec3` | Local → world point |
| `transform_direction(Vec3) → Vec3` | Local → world direction (ignores translation) |
| `lerp(&other, t) → Transform` | Linear interpolation (slerp for rotation) |
| `forward() → Vec3` | Local negative-Z direction |
| `right() → Vec3` | Local positive-X direction |
| `up() → Vec3` | Local positive-Y direction |
| `is_identity() → bool` | Identity check |
| `is_uniform_scale() → bool` | Uniform scale check |

### Node

Scene graph nodes with parent-child relationships. Children are stored inline as `Vec<Node>`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Node {
    pub name: String,
    pub transform: Transform,
    pub children: Vec<Node>,   // NOT Vec<NodeId>
}
```

| Method | Description |
|--------|-------------|
| `Node::new(name)` | Create named node |
| `Node::with_transform(name, transform)` | Create with transform |
| `add_child(Node)` | Append child node |
| `find_child(name) → Option<&Node>` | Non-recursive lookup |
| `find_descendant(name) → Option<&Node>` | Recursive depth-first search |
| `descendant_count() → usize` | Total descendants (recursive) |
| `depth() → usize` | Subtree depth |
| `is_leaf() → bool` | True if no children |

### Scene

Top-level container managing the node hierarchy:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Scene {
    pub root: Node,   // Single root node, NOT NodeId
}
```

| Method | Description |
|--------|-------------|
| `Scene::new()` | Default root named "root" |
| `Scene::with_root(node)` | Custom root node |
| `node_count() → usize` | Total nodes including root |
| `find_node(name) → Option<&Node>` | Searches entire tree |
| `traverse(fn(&Node, Mat4))` | Depth-first with world transforms |
| `traverse_with_path(fn(&Node, Mat4, &[&str]))` | Depth-first with name path tracking |
| `depth() → usize` | Maximum graph depth |
| `is_empty() → bool` | True if root has no children |

## World Partitioning

For large open worlds, the scene system divides space into a 3D grid of cells that stream in/out asynchronously:

```
WorldPartition Architecture
├── Grid (HashMap<GridCoord, Cell>)
│   └── Cell
│       ├── Entities (Vec<Entity>)
│       ├── Assets (Vec<AssetRef>)
│       └── State (Unloaded/Loading/Loaded)
└── WorldPartitionManager
    ├── Active Cells (camera distance-based)
    ├── LRU Cache (recently unloaded cells)
    └── Async Loader (tokio tasks)
```

### GridCoord

3D signed integer coordinates for cell addressing:

```rust
pub struct GridCoord { pub x: i32, pub y: i32, pub z: i32 }

// World position → grid coordinate
let coord = GridCoord::from_world_pos(position, cell_size);

// Grid coordinate → world center
let center = coord.to_world_center(cell_size);

// Get 26 neighbors (3D) or 8 neighbors (2D)
let neighbors = coord.neighbors_3d();
let flat_neighbors = coord.neighbors_2d();
```

### GridConfig

```rust
let config = GridConfig {
    cell_size: 100.0,                                  // meters per cell
    world_bounds: (-5000.0, 5000.0, -5000.0, 5000.0),  // 10 km × 10 km
};
let mut partition = WorldPartition::new(config);
```

## Async Cell Streaming

Distance-based cell loading managed by `WorldPartitionManager`:

```rust
use astraweave_scene::streaming::{WorldPartitionManager, StreamingConfig};

let config = StreamingConfig {
    max_active_cells: 25,        // 5×5 grid around camera
    lru_cache_size: 5,           // recently unloaded cache
    streaming_radius: 500.0,     // meters
    max_concurrent_loads: 4,     // parallel tokio tasks
};

let mut manager = WorldPartitionManager::new(partition, config);

// Add event listener for load/unload events
manager.add_event_listener(|event| match event {
    StreamingEvent::CellLoaded(coord) => println!("Loaded {coord:?}"),
    StreamingEvent::CellUnloaded(coord) => println!("Unloaded {coord:?}"),
    _ => {}
});

// Update streaming based on camera position (async)
manager.update(camera_position).await?;
```

### Streaming Events

| Event | Description |
|-------|-------------|
| `CellLoadStarted(GridCoord)` | Loading task spawned |
| `CellLoaded(GridCoord)` | Cell fully loaded into memory |
| `CellLoadFailed(GridCoord, String)` | Load error with message |
| `CellUnloadStarted(GridCoord)` | Unload initiated |
| `CellUnloaded(GridCoord)` | Cell removed from active set |

### Streaming Metrics

```rust
pub struct StreamingMetrics {
    pub active_cells: usize,
    pub loading_cells: usize,
    pub loaded_cells: usize,
    pub cached_cells: usize,
    pub memory_usage_bytes: usize,
    pub total_loads: u64,
    pub total_unloads: u64,
    pub failed_loads: u64,
}
```

## GPU Resource Management

Per-cell GPU resource lifecycle (buffers, textures) with memory budget enforcement:

```rust
pub struct CellGpuResources {
    pub coord: GridCoord,
    pub vertex_buffers: HashMap<AssetId, Buffer>,
    pub index_buffers: HashMap<AssetId, Buffer>,
    pub textures: HashMap<AssetId, Texture>,
    pub texture_sizes: HashMap<AssetId, usize>,
    pub memory_usage: usize,
}
```

The GPU resource manager tracks per-cell VRAM usage, uploads vertex/index buffers and textures on cell load, and releases them on cell unload to keep memory bounded.

## Integration

| System | Integration |
|--------|-------------|
| **ECS** | Nodes map to entities with Transform components |
| **Physics** | Cell boundaries define physics simulation regions |
| **Rendering** | Only loaded cells are submitted to the render queue |
| **AI** | WorldSnapshot filters entities by loaded cells |
| **Terrain** | Terrain cells align with world partition grid |
| **Asset** | `CellMetadata` and `ComponentData` from `astraweave-asset` feed cell loading |

[← Back to Home](index.html) · [Architecture](architecture.html) · [Rendering](rendering.html)
