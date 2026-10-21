# Navigation API Reference

> **Crate**: `astraweave-nav`  
> **Coverage**: ~78%  
> **Tests**: 200+

Navmesh-based pathfinding with A*, portal graphs, and dynamic obstacle avoidance.

## Quick Links

- [rustdoc](https://docs.rs/astraweave-nav) (when published)
- [Source](https://github.com/astraweave/astraweave/tree/main/astraweave-nav)
- [Navigation Guide](../core-systems/navigation.md)

---

## Core Types

### NavMesh

Navigation mesh for pathfinding.

```rust
use astraweave_nav::{NavMesh, NavMeshConfig};

let config = NavMeshConfig {
    cell_size: 0.3,
    cell_height: 0.2,
    agent_height: 2.0,
    agent_radius: 0.6,
    max_slope: 45.0,
    ..Default::default()
};

// Build from geometry
let navmesh = NavMesh::build(&vertices, &indices, config)?;

// Query path
let path = navmesh.find_path(start, goal)?;
```

---

### NavAgent

Agent with steering behaviors.

```rust
use astraweave_nav::{NavAgent, AgentConfig};

let mut agent = NavAgent::new(AgentConfig {
    speed: 5.0,
    acceleration: 10.0,
    radius: 0.5,
    ..Default::default()
});

// Set destination
agent.set_destination(target_position);

// Update each frame
agent.update(delta_time, &navmesh);

// Get movement vector
let velocity = agent.velocity();
```

---

### PortalGraph

Hierarchical pathfinding for large worlds.

```rust
use astraweave_nav::PortalGraph;

let mut graph = PortalGraph::new();

// Add regions
graph.add_region("forest", forest_navmesh);
graph.add_region("castle", castle_navmesh);

// Connect regions via portals
graph.add_portal("forest", "castle", portal_polygon);

// Find cross-region path
let path = graph.find_path(start, goal)?;
```

---

### ObstacleAvoidance

Dynamic obstacle avoidance using velocity obstacles.

```rust
use astraweave_nav::ObstacleAvoidance;

let mut avoidance = ObstacleAvoidance::new();

// Add moving obstacles
for other in nearby_agents {
    avoidance.add_obstacle(other.position, other.velocity, other.radius);
}

// Compute safe velocity
let safe_velocity = avoidance.compute(
    agent.position,
    agent.desired_velocity,
    agent.radius,
    delta_time,
);
```

---

## Pathfinding

### A* Search

```rust
use astraweave_nav::astar::{astar_search, PathResult};

let result = astar_search(&navmesh, start, goal);

match result {
    PathResult::Found(path) => {
        for waypoint in path {
            // Follow path
        }
    }
    PathResult::Partial(path) => {
        // Closest reachable point
    }
    PathResult::NotFound => {
        // No path exists
    }
}
```

### Path Smoothing

```rust
use astraweave_nav::path_smoothing::smooth_path;

let raw_path = navmesh.find_path(start, goal)?;
let smooth = smooth_path(&raw_path, &navmesh);

// Smooth path has fewer waypoints
assert!(smooth.len() <= raw_path.len());
```

---

## Crowd Simulation

### CrowdManager

Large-scale agent coordination.

```rust
use astraweave_nav::crowd::{CrowdManager, CrowdAgent};

let mut crowd = CrowdManager::new(&navmesh, max_agents);

// Add agents
let agent_id = crowd.add_agent(CrowdAgent {
    position: spawn_point,
    target: destination,
    speed: 5.0,
    ..Default::default()
});

// Update all agents
crowd.update(delta_time);

// Get agent state
let agent = crowd.get_agent(agent_id);
```

---

## Performance

| Operation | Latency | Notes |
|-----------|---------|-------|
| A* pathfind | 50-500 µs | Depends on distance |
| NavMesh query | ~1 µs | Point location |
| Agent update | ~100 ns | Per agent |
| Crowd update (100) | ~50 µs | All agents |

---

## See Also

- [Navigation Guide](../core-systems/navigation.md)
- [AI Integration](../core-systems/ai-core.md)
- [Terrain Integration](../core-systems/terrain.md#navigation)
