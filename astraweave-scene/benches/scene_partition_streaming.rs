// AstraWeave-Scene Comprehensive Benchmarks v5.35
//
// Benchmarks covering:
// - Transform: matrix computations, TRS operations
// - Scene Graph: node creation, traversal, hierarchy
// - World Partition: GridCoord, AABB, cell operations, frustum culling
// - LRU Cache: cache operations, eviction
// - GPU Resource Management: budget tracking, memory allocation
// - Streaming: cell entity management, spatial queries
// - Partitioned Scene: entity tracking, cell queries

use criterion::{criterion_group, BenchmarkId, Criterion, Throughput};
use glam::{Mat4, Quat, Vec3, Vec4};
use std::collections::HashMap;
use std::hint::black_box;

// ============================================================================
// MOCK TYPES (avoiding full crate dependencies for benchmark isolation)
// ============================================================================

/// Transform mock (from lib.rs)
#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    pub fn from_trs(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }
}

/// Node mock (from lib.rs)
#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub transform: Transform,
    pub children: Vec<Node>,
}

impl Node {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transform: Transform::default(),
            children: Vec::new(),
        }
    }
}

/// Scene mock (from lib.rs)
#[derive(Debug, Clone)]
pub struct Scene {
    pub root: Node,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            root: Node::new("root"),
        }
    }

    pub fn traverse<'a>(&'a self, f: &mut impl FnMut(&'a Node, Mat4)) {
        fn walk<'a>(n: &'a Node, parent: Mat4, f: &mut impl FnMut(&'a Node, Mat4)) {
            let world = parent * n.transform.matrix();
            f(n, world);
            for c in &n.children {
                walk(c, world, f);
            }
        }
        walk(&self.root, Mat4::IDENTITY, f);
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

/// GridCoord mock (from world_partition.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl GridCoord {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn from_world_pos(pos: Vec3, cell_size: f32) -> Self {
        Self {
            x: (pos.x / cell_size).floor() as i32,
            y: (pos.y / cell_size).floor() as i32,
            z: (pos.z / cell_size).floor() as i32,
        }
    }

    pub fn to_world_center(self, cell_size: f32) -> Vec3 {
        Vec3::new(
            (self.x as f32 + 0.5) * cell_size,
            (self.y as f32 + 0.5) * cell_size,
            (self.z as f32 + 0.5) * cell_size,
        )
    }

    pub fn neighbors_3d(self) -> Vec<GridCoord> {
        let mut neighbors = Vec::with_capacity(26);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue;
                    }
                    neighbors.push(GridCoord::new(self.x + dx, self.y + dy, self.z + dz));
                }
            }
        }
        neighbors
    }

    pub fn neighbors_2d(self) -> Vec<GridCoord> {
        let mut neighbors = Vec::with_capacity(8);
        for dx in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dz == 0 {
                    continue;
                }
                neighbors.push(GridCoord::new(self.x + dx, self.y, self.z + dz));
            }
        }
        neighbors
    }

    pub fn manhattan_distance(self, other: GridCoord) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs() + (self.z - other.z).abs()
    }
}

/// AABB mock (from world_partition.rs)
#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_center_half_extents(center: Vec3, half_extents: Vec3) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    pub fn overlapping_cells(&self, cell_size: f32) -> Vec<GridCoord> {
        let min_coord = GridCoord::from_world_pos(self.min, cell_size);
        let max_coord = GridCoord::from_world_pos(self.max, cell_size);

        let mut cells = Vec::new();
        for x in min_coord.x..=max_coord.x {
            for y in min_coord.y..=max_coord.y {
                for z in min_coord.z..=max_coord.z {
                    cells.push(GridCoord::new(x, y, z));
                }
            }
        }
        cells
    }
}

/// Frustum mock (from world_partition.rs)
#[derive(Debug, Clone)]
pub struct Frustum {
    pub planes: [Vec4; 6],
}

impl Frustum {
    pub fn from_view_projection(view_proj: Mat4) -> Self {
        let mut planes = [Vec4::ZERO; 6];

        // Left plane
        planes[0] = Vec4::new(
            view_proj.x_axis.w + view_proj.x_axis.x,
            view_proj.y_axis.w + view_proj.y_axis.x,
            view_proj.z_axis.w + view_proj.z_axis.x,
            view_proj.w_axis.w + view_proj.w_axis.x,
        );
        // Right plane
        planes[1] = Vec4::new(
            view_proj.x_axis.w - view_proj.x_axis.x,
            view_proj.y_axis.w - view_proj.y_axis.x,
            view_proj.z_axis.w - view_proj.z_axis.x,
            view_proj.w_axis.w - view_proj.w_axis.x,
        );
        // Bottom plane
        planes[2] = Vec4::new(
            view_proj.x_axis.w + view_proj.x_axis.y,
            view_proj.y_axis.w + view_proj.y_axis.y,
            view_proj.z_axis.w + view_proj.z_axis.y,
            view_proj.w_axis.w + view_proj.w_axis.y,
        );
        // Top plane
        planes[3] = Vec4::new(
            view_proj.x_axis.w - view_proj.x_axis.y,
            view_proj.y_axis.w - view_proj.y_axis.y,
            view_proj.z_axis.w - view_proj.z_axis.y,
            view_proj.w_axis.w - view_proj.w_axis.y,
        );
        // Near plane
        planes[4] = Vec4::new(
            view_proj.x_axis.w + view_proj.x_axis.z,
            view_proj.y_axis.w + view_proj.y_axis.z,
            view_proj.z_axis.w + view_proj.z_axis.z,
            view_proj.w_axis.w + view_proj.w_axis.z,
        );
        // Far plane
        planes[5] = Vec4::new(
            view_proj.x_axis.w - view_proj.x_axis.z,
            view_proj.y_axis.w - view_proj.y_axis.z,
            view_proj.z_axis.w - view_proj.z_axis.z,
            view_proj.w_axis.w - view_proj.w_axis.z,
        );

        // Normalize planes
        for plane in &mut planes {
            let length = Vec3::new(plane.x, plane.y, plane.z).length();
            if length > 0.0 {
                *plane /= length;
            }
        }

        Self { planes }
    }

    pub fn intersects_aabb(&self, aabb: &AABB) -> bool {
        for plane in &self.planes {
            let normal = Vec3::new(plane.x, plane.y, plane.z);
            let d = plane.w;

            let p = Vec3::new(
                if normal.x >= 0.0 {
                    aabb.max.x
                } else {
                    aabb.min.x
                },
                if normal.y >= 0.0 {
                    aabb.max.y
                } else {
                    aabb.min.y
                },
                if normal.z >= 0.0 {
                    aabb.max.z
                } else {
                    aabb.min.z
                },
            );

            if normal.dot(p) + d < 0.0 {
                return false;
            }
        }
        true
    }
}

/// LRU Cache mock (from world_partition.rs)
use std::collections::VecDeque;

#[derive(Debug)]
pub struct LRUCache {
    capacity: usize,
    queue: VecDeque<GridCoord>,
}

impl LRUCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            queue: VecDeque::with_capacity(capacity),
        }
    }

    pub fn touch(&mut self, coord: GridCoord) {
        if let Some(pos) = self.queue.iter().position(|&c| c == coord) {
            self.queue.remove(pos);
        }
        self.queue.push_front(coord);
        if self.queue.len() > self.capacity {
            self.queue.pop_back();
        }
    }

    pub fn contains(&self, coord: GridCoord) -> bool {
        self.queue.contains(&coord)
    }

    pub fn lru(&self) -> Option<GridCoord> {
        self.queue.back().copied()
    }

    pub fn remove(&mut self, coord: GridCoord) {
        if let Some(pos) = self.queue.iter().position(|&c| c == coord) {
            self.queue.remove(pos);
        }
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

/// Cell state mock (from world_partition.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Unloaded,
    Loading,
    Loaded,
    Unloading,
}

/// Cell mock (from world_partition.rs)
#[derive(Debug, Clone)]
pub struct Cell {
    pub coord: GridCoord,
    pub state: CellState,
    pub entities: Vec<u64>,
    pub bounds: AABB,
}

impl Cell {
    pub fn new(coord: GridCoord, cell_size: f32) -> Self {
        let center = coord.to_world_center(cell_size);
        let half_size = Vec3::splat(cell_size * 0.5);
        let bounds = AABB::from_center_half_extents(center, half_size);

        Self {
            coord,
            state: CellState::Unloaded,
            entities: Vec::new(),
            bounds,
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.state == CellState::Loaded
    }
}

/// WorldPartition mock (from world_partition.rs)
#[derive(Debug, Clone)]
pub struct GridConfig {
    pub cell_size: f32,
    pub world_bounds: (f32, f32, f32, f32),
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            cell_size: 100.0,
            world_bounds: (-5000.0, 5000.0, -5000.0, 5000.0),
        }
    }
}

#[derive(Debug)]
pub struct WorldPartition {
    pub config: GridConfig,
    pub cells: HashMap<GridCoord, Cell>,
}

impl WorldPartition {
    pub fn new(config: GridConfig) -> Self {
        Self {
            config,
            cells: HashMap::new(),
        }
    }

    pub fn get_or_create_cell(&mut self, coord: GridCoord) -> &mut Cell {
        let cell_size = self.config.cell_size;
        self.cells
            .entry(coord)
            .or_insert_with(|| Cell::new(coord, cell_size))
    }

    pub fn assign_entity_to_cell(&mut self, entity: u64, position: Vec3) {
        let coord = GridCoord::from_world_pos(position, self.config.cell_size);
        let cell = self.get_or_create_cell(coord);
        if !cell.entities.contains(&entity) {
            cell.entities.push(entity);
        }
    }

    pub fn cells_in_radius(&self, center: Vec3, radius: f32) -> Vec<GridCoord> {
        let center_coord = GridCoord::from_world_pos(center, self.config.cell_size);
        let radius_cells = (radius / self.config.cell_size).ceil() as i32;

        let mut cells = Vec::new();
        for dx in -radius_cells..=radius_cells {
            for dz in -radius_cells..=radius_cells {
                let coord = GridCoord::new(center_coord.x + dx, 0, center_coord.z + dz);
                let cell_center = coord.to_world_center(self.config.cell_size);
                let distance = (cell_center - center).length();
                if distance <= radius {
                    cells.push(coord);
                }
            }
        }
        cells
    }
}

/// GPU Resource Budget mock (from gpu_resource_manager.rs)
#[derive(Debug)]
pub struct GpuMemoryStats {
    pub total_allocated: usize,
    pub max_budget: usize,
    pub active_cells: usize,
    pub utilization: f32,
}

#[derive(Debug)]
pub struct CellGpuResources {
    pub coord: GridCoord,
    pub memory_usage: usize,
}

impl CellGpuResources {
    pub fn new(coord: GridCoord) -> Self {
        Self {
            coord,
            memory_usage: 0,
        }
    }
}

#[derive(Debug)]
pub struct GpuResourceBudget {
    pub max_memory_bytes: usize,
    pub current_usage: usize,
    pub cells: HashMap<GridCoord, CellGpuResources>,
}

impl GpuResourceBudget {
    pub fn new(max_memory_bytes: usize) -> Self {
        Self {
            max_memory_bytes,
            current_usage: 0,
            cells: HashMap::new(),
        }
    }

    pub fn can_allocate(&self, bytes: usize) -> bool {
        self.current_usage + bytes <= self.max_memory_bytes
    }

    pub fn get_or_create_cell(&mut self, coord: GridCoord) -> &mut CellGpuResources {
        self.cells
            .entry(coord)
            .or_insert_with(|| CellGpuResources::new(coord))
    }

    pub fn unload_cell(&mut self, coord: GridCoord) {
        if let Some(cell) = self.cells.get(&coord) {
            self.current_usage = self.current_usage.saturating_sub(cell.memory_usage);
        }
        self.cells.remove(&coord);
    }

    pub fn update_usage(&mut self) {
        self.current_usage = self.cells.values().map(|c| c.memory_usage).sum();
    }

    pub fn stats(&self) -> GpuMemoryStats {
        GpuMemoryStats {
            total_allocated: self.current_usage,
            max_budget: self.max_memory_bytes,
            active_cells: self.cells.len(),
            utilization: (self.current_usage as f32 / self.max_memory_bytes as f32) * 100.0,
        }
    }

    pub fn find_furthest_cell(&self, camera_pos: Vec3, cell_size: f32) -> Option<GridCoord> {
        let mut furthest: Option<(GridCoord, f32)> = None;

        for coord in self.cells.keys() {
            let cell_center = coord.to_world_center(cell_size);
            let distance = (cell_center - camera_pos).length();

            match furthest {
                None => furthest = Some((*coord, distance)),
                Some((_, max_dist)) if distance > max_dist => furthest = Some((*coord, distance)),
                _ => {}
            }
        }

        furthest.map(|(coord, _)| coord)
    }
}

/// CellEntities mock (from partitioned_scene.rs)
#[derive(Debug, Clone)]
pub struct CellEntities {
    pub cell: GridCoord,
    pub entities: Vec<u64>,
}

impl CellEntities {
    pub fn new(cell: GridCoord) -> Self {
        Self {
            cell,
            entities: Vec::new(),
        }
    }

    pub fn add_entity(&mut self, entity: u64) {
        if !self.entities.contains(&entity) {
            self.entities.push(entity);
        }
    }

    pub fn remove_entity(&mut self, entity: u64) {
        self.entities.retain(|&e| e != entity);
    }
}

// ============================================================================
// BENCHMARK GROUP 1: Transform Operations
// ============================================================================

fn bench_transform_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Transform");

    // Transform creation
    group.bench_function("default_creation", |b| {
        b.iter(|| black_box(Transform::default()))
    });

    group.bench_function("from_trs_creation", |b| {
        let t = Vec3::new(10.0, 20.0, 30.0);
        let r = Quat::from_rotation_y(std::f32::consts::PI / 4.0);
        let s = Vec3::new(2.0, 2.0, 2.0);
        b.iter(|| black_box(Transform::from_trs(t, r, s)))
    });

    // Matrix computation
    group.bench_function("matrix_identity", |b| {
        let t = Transform::default();
        b.iter(|| black_box(t.matrix()))
    });

    group.bench_function("matrix_translation_only", |b| {
        let t = Transform {
            translation: Vec3::new(100.0, 200.0, 300.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        b.iter(|| black_box(t.matrix()))
    });

    group.bench_function("matrix_full_trs", |b| {
        let t = Transform {
            translation: Vec3::new(10.0, 20.0, 30.0),
            rotation: Quat::from_rotation_y(std::f32::consts::PI / 4.0),
            scale: Vec3::new(2.0, 3.0, 4.0),
        };
        b.iter(|| black_box(t.matrix()))
    });

    // Matrix multiplication chain
    group.bench_function("matrix_chain_2", |b| {
        let t1 = Transform::from_trs(Vec3::X * 10.0, Quat::IDENTITY, Vec3::ONE);
        let t2 = Transform::from_trs(Vec3::Y * 5.0, Quat::from_rotation_z(0.5), Vec3::ONE);
        b.iter(|| {
            let m1 = t1.matrix();
            let m2 = t2.matrix();
            black_box(m1 * m2)
        })
    });

    group.bench_function("matrix_chain_5", |b| {
        let transforms: Vec<Transform> = (0..5)
            .map(|i| {
                Transform::from_trs(
                    Vec3::new(i as f32, 0.0, 0.0),
                    Quat::from_rotation_y(i as f32 * 0.1),
                    Vec3::ONE,
                )
            })
            .collect();
        b.iter(|| {
            let mut result = Mat4::IDENTITY;
            for t in &transforms {
                result *= t.matrix();
            }
            black_box(result)
        })
    });

    // Transform decomposition
    group.bench_function("matrix_decompose_trs", |b| {
        let t = Transform::from_trs(
            Vec3::new(10.0, 20.0, 30.0),
            Quat::from_rotation_y(1.0),
            Vec3::new(2.0, 2.0, 2.0),
        );
        let m = t.matrix();
        b.iter(|| black_box(m.to_scale_rotation_translation()))
    });

    group.finish();
}

// ============================================================================
// BENCHMARK GROUP 2: Scene Graph Operations
// ============================================================================

fn bench_scene_graph_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("SceneGraph");

    // Node creation
    group.bench_function("node_creation_str", |b| {
        b.iter(|| black_box(Node::new("test_node")))
    });

    group.bench_function("node_creation_string", |b| {
        b.iter(|| black_box(Node::new(String::from("test_node"))))
    });

    // Scene creation
    group.bench_function("scene_creation", |b| b.iter(|| black_box(Scene::new())));

    // Scene traversal - varying depths
    for depth in [1, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("traverse_linear_depth", depth),
            &depth,
            |b, &depth| {
                let mut scene = Scene::new();
                let mut current = &mut scene.root;
                for i in 0..depth {
                    current.children.push(Node::new(format!("level{}", i)));
                    current = &mut current.children[0];
                }
                b.iter(|| {
                    let mut count = 0;
                    scene.traverse(&mut |_node, _matrix| {
                        count += 1;
                    });
                    black_box(count)
                })
            },
        );
    }

    // Wide scene (many children per node)
    for width in [2, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("traverse_wide_children", width),
            &width,
            |b, &width| {
                let mut scene = Scene::new();
                for i in 0..width {
                    scene.root.children.push(Node::new(format!("child{}", i)));
                }
                b.iter(|| {
                    let mut count = 0;
                    scene.traverse(&mut |_node, _matrix| {
                        count += 1;
                    });
                    black_box(count)
                })
            },
        );
    }

    // Deep+wide combined
    group.bench_function("traverse_tree_3x3x3", |b| {
        let mut scene = Scene::new();
        for i in 0..3 {
            let mut level1 = Node::new(format!("l1_{}", i));
            for j in 0..3 {
                let mut level2 = Node::new(format!("l2_{}", j));
                for k in 0..3 {
                    level2.children.push(Node::new(format!("l3_{}", k)));
                }
                level1.children.push(level2);
            }
            scene.root.children.push(level1);
        }
        // Total nodes: 1 + 3 + 9 + 27 = 40
        b.iter(|| {
            let mut count = 0;
            scene.traverse(&mut |_node, _matrix| {
                count += 1;
            });
            black_box(count)
        })
    });

    // Traverse with transform accumulation
    group.bench_function("traverse_with_transform_extraction", |b| {
        let mut scene = Scene::new();
        scene.root.transform.translation = Vec3::new(10.0, 0.0, 0.0);
        for i in 0..5 {
            let mut child = Node::new(format!("child{}", i));
            child.transform.translation = Vec3::new(5.0, 0.0, 0.0);
            scene.root.children.push(child);
        }
        b.iter(|| {
            let mut positions = Vec::new();
            scene.traverse(&mut |_node, matrix| {
                let (_, _, t) = matrix.to_scale_rotation_translation();
                positions.push(t);
            });
            black_box(positions)
        })
    });

    group.finish();
}

// ============================================================================
// BENCHMARK GROUP 3: GridCoord Operations
// ============================================================================

fn bench_grid_coord_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("GridCoord");

    // Creation
    group.bench_function("new_creation", |b| {
        b.iter(|| black_box(GridCoord::new(1, 2, 3)))
    });

    group.bench_function("from_world_pos_origin", |b| {
        let pos = Vec3::ZERO;
        b.iter(|| black_box(GridCoord::from_world_pos(pos, 100.0)))
    });

    group.bench_function("from_world_pos_positive", |b| {
        let pos = Vec3::new(150.0, 50.0, 250.0);
        b.iter(|| black_box(GridCoord::from_world_pos(pos, 100.0)))
    });

    group.bench_function("from_world_pos_negative", |b| {
        let pos = Vec3::new(-150.0, -50.0, -250.0);
        b.iter(|| black_box(GridCoord::from_world_pos(pos, 100.0)))
    });

    // Conversion
    group.bench_function("to_world_center", |b| {
        let coord = GridCoord::new(5, 2, 8);
        b.iter(|| black_box(coord.to_world_center(100.0)))
    });

    // Neighbors
    group.bench_function("neighbors_3d_26", |b| {
        let coord = GridCoord::new(0, 0, 0);
        b.iter(|| black_box(coord.neighbors_3d()))
    });

    group.bench_function("neighbors_2d_8", |b| {
        let coord = GridCoord::new(0, 0, 0);
        b.iter(|| black_box(coord.neighbors_2d()))
    });

    // Distance
    group.bench_function("manhattan_distance", |bencher| {
        let a = GridCoord::new(0, 0, 0);
        let b = GridCoord::new(10, 20, 30);
        bencher.iter(|| black_box(a.manhattan_distance(b)))
    });

    // Hash operations (for HashMap lookups)
    group.bench_function("hash_lookup", |b| {
        let mut map: HashMap<GridCoord, u32> = HashMap::new();
        for x in -5..=5 {
            for z in -5..=5 {
                map.insert(GridCoord::new(x, 0, z), (x + z) as u32);
            }
        }
        let key = GridCoord::new(3, 0, 4);
        b.iter(|| black_box(map.get(&key)))
    });

    group.finish();
}

// ============================================================================
// BENCHMARK GROUP 4: AABB Operations
// ============================================================================

fn bench_aabb_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("AABB");

    // Creation
    group.bench_function("new_creation", |b| {
        let min = Vec3::new(0.0, 0.0, 0.0);
        let max = Vec3::new(10.0, 10.0, 10.0);
        b.iter(|| black_box(AABB::new(min, max)))
    });

    group.bench_function("from_center_half_extents", |b| {
        let center = Vec3::new(5.0, 5.0, 5.0);
        let half = Vec3::new(5.0, 5.0, 5.0);
        b.iter(|| black_box(AABB::from_center_half_extents(center, half)))
    });

    // Properties
    group.bench_function("center_computation", |b| {
        let aabb = AABB::new(Vec3::ZERO, Vec3::new(10.0, 20.0, 30.0));
        b.iter(|| black_box(aabb.center()))
    });

    group.bench_function("half_extents_computation", |b| {
        let aabb = AABB::new(Vec3::ZERO, Vec3::new(10.0, 20.0, 30.0));
        b.iter(|| black_box(aabb.half_extents()))
    });

    // Point containment
    group.bench_function("contains_point_inside", |b| {
        let aabb = AABB::new(Vec3::ZERO, Vec3::new(10.0, 10.0, 10.0));
        let point = Vec3::new(5.0, 5.0, 5.0);
        b.iter(|| black_box(aabb.contains_point(point)))
    });

    group.bench_function("contains_point_outside", |b| {
        let aabb = AABB::new(Vec3::ZERO, Vec3::new(10.0, 10.0, 10.0));
        let point = Vec3::new(15.0, 5.0, 5.0);
        b.iter(|| black_box(aabb.contains_point(point)))
    });

    // Intersection
    group.bench_function("intersects_overlapping", |bencher| {
        let a = AABB::new(Vec3::ZERO, Vec3::new(10.0, 10.0, 10.0));
        let b = AABB::new(Vec3::new(5.0, 5.0, 5.0), Vec3::new(15.0, 15.0, 15.0));
        bencher.iter(|| black_box(a.intersects(&b)))
    });

    group.bench_function("intersects_separate", |bencher| {
        let a = AABB::new(Vec3::ZERO, Vec3::new(10.0, 10.0, 10.0));
        let b = AABB::new(Vec3::new(20.0, 20.0, 20.0), Vec3::new(30.0, 30.0, 30.0));
        bencher.iter(|| black_box(a.intersects(&b)))
    });

    // Cell overlap
    for size in [1, 4, 8] {
        let extent = size as f32 * 100.0;
        group.bench_with_input(
            BenchmarkId::new("overlapping_cells", format!("{}x{}x{}", size, size, size)),
            &extent,
            |b, &extent| {
                let aabb = AABB::new(Vec3::ZERO, Vec3::splat(extent));
                b.iter(|| black_box(aabb.overlapping_cells(100.0)))
            },
        );
    }

    group.finish();
}

// ============================================================================
// BENCHMARK GROUP 5: Frustum Culling
// ============================================================================

fn bench_frustum_culling(c: &mut Criterion) {
    let mut group = c.benchmark_group("FrustumCulling");

    // Frustum creation
    group.bench_function("from_orthographic_matrix", |b| {
        let view_proj = Mat4::orthographic_rh(-100.0, 100.0, -100.0, 100.0, 0.1, 1000.0);
        b.iter(|| black_box(Frustum::from_view_projection(view_proj)))
    });

    group.bench_function("from_perspective_matrix", |b| {
        let proj = Mat4::perspective_rh(std::f32::consts::PI / 4.0, 16.0 / 9.0, 0.1, 1000.0);
        let view = Mat4::look_at_rh(Vec3::new(0.0, 10.0, 10.0), Vec3::ZERO, Vec3::Y);
        let view_proj = proj * view;
        b.iter(|| black_box(Frustum::from_view_projection(view_proj)))
    });

    // AABB intersection tests
    let view_proj = Mat4::orthographic_rh(-100.0, 100.0, -100.0, 100.0, 0.1, 1000.0);
    let frustum = Frustum::from_view_projection(view_proj);

    group.bench_function("intersects_aabb_inside", |b| {
        let aabb = AABB::new(Vec3::new(-10.0, -10.0, -10.0), Vec3::new(10.0, 10.0, 10.0));
        b.iter(|| black_box(frustum.intersects_aabb(&aabb)))
    });

    group.bench_function("intersects_aabb_outside", |b| {
        let aabb = AABB::new(
            Vec3::new(200.0, 200.0, 200.0),
            Vec3::new(300.0, 300.0, 300.0),
        );
        b.iter(|| black_box(frustum.intersects_aabb(&aabb)))
    });

    group.bench_function("intersects_aabb_partial", |b| {
        let aabb = AABB::new(Vec3::new(90.0, 0.0, 0.0), Vec3::new(110.0, 20.0, 20.0));
        b.iter(|| black_box(frustum.intersects_aabb(&aabb)))
    });

    // Batch culling
    for count in [10, 50, 100, 500] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("batch_cull_aabbs", count),
            &count,
            |b, &count| {
                let aabbs: Vec<AABB> = (0..count)
                    .map(|i| {
                        let offset = (i as f32 - count as f32 / 2.0) * 10.0;
                        AABB::new(
                            Vec3::new(offset, 0.0, 0.0),
                            Vec3::new(offset + 5.0, 5.0, 5.0),
                        )
                    })
                    .collect();
                b.iter(|| {
                    let visible: Vec<bool> = aabbs
                        .iter()
                        .map(|aabb| frustum.intersects_aabb(aabb))
                        .collect();
                    black_box(visible)
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// BENCHMARK GROUP 6: LRU Cache Operations
// ============================================================================

fn bench_lru_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("LRUCache");

    // Creation
    for capacity in [5, 25, 100] {
        group.bench_with_input(
            BenchmarkId::new("creation", capacity),
            &capacity,
            |b, &capacity| b.iter(|| black_box(LRUCache::new(capacity))),
        );
    }

    // Touch operations
    group.bench_function("touch_new_entry", |b| {
        let mut cache = LRUCache::new(10);
        let mut i = 0;
        b.iter(|| {
            cache.touch(GridCoord::new(i, 0, 0));
            i += 1;
            if i > 100 {
                i = 0;
                cache = LRUCache::new(10);
            }
        })
    });

    group.bench_function("touch_existing_entry", |b| {
        let mut cache = LRUCache::new(10);
        for i in 0..5 {
            cache.touch(GridCoord::new(i, 0, 0));
        }
        b.iter(|| {
            cache.touch(GridCoord::new(2, 0, 0));
            black_box(())
        })
    });

    group.bench_function("touch_with_eviction", |b| {
        let mut cache = LRUCache::new(5);
        for i in 0..5 {
            cache.touch(GridCoord::new(i, 0, 0));
        }
        let mut i = 100;
        b.iter(|| {
            cache.touch(GridCoord::new(i, 0, 0));
            i += 1;
        })
    });

    // Contains check
    group.bench_function("contains_present", |b| {
        let mut cache = LRUCache::new(10);
        for i in 0..10 {
            cache.touch(GridCoord::new(i, 0, 0));
        }
        let key = GridCoord::new(5, 0, 0);
        b.iter(|| black_box(cache.contains(key)))
    });

    group.bench_function("contains_absent", |b| {
        let mut cache = LRUCache::new(10);
        for i in 0..10 {
            cache.touch(GridCoord::new(i, 0, 0));
        }
        let key = GridCoord::new(99, 0, 0);
        b.iter(|| black_box(cache.contains(key)))
    });

    // LRU retrieval
    group.bench_function("lru_retrieval", |b| {
        let mut cache = LRUCache::new(10);
        for i in 0..10 {
            cache.touch(GridCoord::new(i, 0, 0));
        }
        b.iter(|| black_box(cache.lru()))
    });

    // Remove
    group.bench_function("remove_existing", |b| {
        b.iter_batched(
            || {
                let mut cache = LRUCache::new(10);
                for i in 0..10 {
                    cache.touch(GridCoord::new(i, 0, 0));
                }
                cache
            },
            |mut cache| {
                cache.remove(GridCoord::new(5, 0, 0));
                black_box(cache)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

// ============================================================================
// BENCHMARK GROUP 7: World Partition Operations
// ============================================================================

fn bench_world_partition_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("WorldPartition");

    // Creation
    group.bench_function("creation_default", |b| {
        b.iter(|| black_box(WorldPartition::new(GridConfig::default())))
    });

    // Cell operations
    group.bench_function("get_or_create_cell_new", |b| {
        b.iter_batched(
            || WorldPartition::new(GridConfig::default()),
            |mut partition| {
                let coord = GridCoord::new(5, 0, 5);
                partition.get_or_create_cell(coord);
                black_box(partition)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("get_or_create_cell_existing", |b| {
        let mut partition = WorldPartition::new(GridConfig::default());
        let coord = GridCoord::new(5, 0, 5);
        partition.get_or_create_cell(coord);
        b.iter(|| {
            partition.get_or_create_cell(coord);
            black_box(())
        })
    });

    // Entity assignment
    group.bench_function("assign_entity_single", |b| {
        let mut partition = WorldPartition::new(GridConfig::default());
        let mut entity_id = 0u64;
        b.iter(|| {
            partition.assign_entity_to_cell(entity_id, Vec3::new(150.0, 0.0, 250.0));
            entity_id += 1;
        })
    });

    // Cells in radius
    for radius in [100.0, 300.0, 500.0] {
        group.bench_with_input(
            BenchmarkId::new("cells_in_radius", format!("{}m", radius as i32)),
            &radius,
            |b, &radius| {
                let partition = WorldPartition::new(GridConfig::default());
                let center = Vec3::new(0.0, 0.0, 0.0);
                b.iter(|| black_box(partition.cells_in_radius(center, radius)))
            },
        );
    }

    // Pre-populated partition operations
    group.bench_function("cells_in_radius_populated", |b| {
        let mut partition = WorldPartition::new(GridConfig::default());
        // Populate with 100 cells
        for x in -5..=5 {
            for z in -5..=5 {
                partition.get_or_create_cell(GridCoord::new(x, 0, z));
            }
        }
        let center = Vec3::new(0.0, 0.0, 0.0);
        b.iter(|| black_box(partition.cells_in_radius(center, 300.0)))
    });

    group.finish();
}

// ============================================================================
// BENCHMARK GROUP 8: GPU Resource Budget Operations
// ============================================================================

fn bench_gpu_resource_budget(c: &mut Criterion) {
    let mut group = c.benchmark_group("GpuResourceBudget");

    // Creation
    group.bench_function("creation_500mb", |b| {
        b.iter(|| black_box(GpuResourceBudget::new(500 * 1024 * 1024)))
    });

    // Allocation check
    group.bench_function("can_allocate_yes", |b| {
        let budget = GpuResourceBudget::new(100 * 1024 * 1024);
        b.iter(|| black_box(budget.can_allocate(10 * 1024 * 1024)))
    });

    group.bench_function("can_allocate_no", |b| {
        let mut budget = GpuResourceBudget::new(100 * 1024 * 1024);
        budget.current_usage = 95 * 1024 * 1024;
        b.iter(|| black_box(budget.can_allocate(10 * 1024 * 1024)))
    });

    // Cell operations
    group.bench_function("get_or_create_cell", |b| {
        let mut budget = GpuResourceBudget::new(500 * 1024 * 1024);
        let mut x = 0;
        b.iter(|| {
            let coord = GridCoord::new(x, 0, 0);
            budget.get_or_create_cell(coord);
            x += 1;
        })
    });

    group.bench_function("unload_cell", |b| {
        b.iter_batched(
            || {
                let mut budget = GpuResourceBudget::new(500 * 1024 * 1024);
                for i in 0..10 {
                    let cell = budget.get_or_create_cell(GridCoord::new(i, 0, 0));
                    cell.memory_usage = 10 * 1024 * 1024;
                }
                budget.update_usage();
                budget
            },
            |mut budget| {
                budget.unload_cell(GridCoord::new(5, 0, 0));
                black_box(budget)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    // Stats computation
    group.bench_function("stats_computation", |b| {
        let mut budget = GpuResourceBudget::new(500 * 1024 * 1024);
        for i in 0..25 {
            let cell = budget.get_or_create_cell(GridCoord::new(i % 5, 0, i / 5));
            cell.memory_usage = 10 * 1024 * 1024;
        }
        budget.update_usage();
        b.iter(|| black_box(budget.stats()))
    });

    // Find furthest cell
    group.bench_function("find_furthest_cell_25", |b| {
        let mut budget = GpuResourceBudget::new(500 * 1024 * 1024);
        for x in -2..=2 {
            for z in -2..=2 {
                budget.get_or_create_cell(GridCoord::new(x, 0, z));
            }
        }
        let camera = Vec3::ZERO;
        b.iter(|| black_box(budget.find_furthest_cell(camera, 100.0)))
    });

    group.bench_function("find_furthest_cell_100", |b| {
        let mut budget = GpuResourceBudget::new(500 * 1024 * 1024);
        for x in -5..=5 {
            for z in -5..=5 {
                budget.get_or_create_cell(GridCoord::new(x, 0, z));
            }
        }
        let camera = Vec3::ZERO;
        b.iter(|| black_box(budget.find_furthest_cell(camera, 100.0)))
    });

    // Update usage
    group.bench_function("update_usage_25_cells", |b| {
        let mut budget = GpuResourceBudget::new(500 * 1024 * 1024);
        for i in 0..25 {
            let cell = budget.get_or_create_cell(GridCoord::new(i % 5, 0, i / 5));
            cell.memory_usage = 10 * 1024 * 1024;
        }
        b.iter(|| {
            budget.update_usage();
            black_box(budget.current_usage)
        })
    });

    group.finish();
}

// ============================================================================
// BENCHMARK GROUP 9: Cell Entity Management
// ============================================================================

fn bench_cell_entity_management(c: &mut Criterion) {
    let mut group = c.benchmark_group("CellEntities");

    // Creation
    group.bench_function("creation", |b| {
        let coord = GridCoord::new(0, 0, 0);
        b.iter(|| black_box(CellEntities::new(coord)))
    });

    // Add entity
    group.bench_function("add_entity_first", |b| {
        let mut ce = CellEntities::new(GridCoord::new(0, 0, 0));
        b.iter(|| {
            ce.add_entity(1);
            ce.entities.clear();
        })
    });

    group.bench_function("add_entity_dedup", |b| {
        let mut ce = CellEntities::new(GridCoord::new(0, 0, 0));
        ce.add_entity(1);
        b.iter(|| {
            ce.add_entity(1); // Already exists
            black_box(())
        })
    });

    // Add multiple entities
    for count in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("add_entities_batch", count),
            &count,
            |b, &count| {
                b.iter_batched(
                    || CellEntities::new(GridCoord::new(0, 0, 0)),
                    |mut ce| {
                        for i in 0..count {
                            ce.add_entity(i as u64);
                        }
                        black_box(ce)
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    // Remove entity
    group.bench_function("remove_entity_present", |b| {
        b.iter_batched(
            || {
                let mut ce = CellEntities::new(GridCoord::new(0, 0, 0));
                for i in 0..10 {
                    ce.add_entity(i);
                }
                ce
            },
            |mut ce| {
                ce.remove_entity(5);
                black_box(ce)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("remove_entity_absent", |b| {
        let mut ce = CellEntities::new(GridCoord::new(0, 0, 0));
        for i in 0..10 {
            ce.add_entity(i);
        }
        b.iter(|| {
            ce.remove_entity(999);
            black_box(())
        })
    });

    group.finish();
}

// ============================================================================
// BENCHMARK GROUP 10: Spatial Queries
// ============================================================================

fn bench_spatial_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("SpatialQueries");

    // Entity to cell mapping
    group.bench_function("entity_cell_lookup", |b| {
        let mut entity_cells: HashMap<u64, GridCoord> = HashMap::new();
        for i in 0..1000 {
            entity_cells.insert(i, GridCoord::new((i % 10) as i32, 0, (i / 10) as i32));
        }
        b.iter(|| black_box(entity_cells.get(&500)))
    });

    // Multi-cell query
    group.bench_function("query_entities_5_cells", |b| {
        let mut cell_entities: HashMap<GridCoord, CellEntities> = HashMap::new();
        for x in 0..5 {
            for z in 0..5 {
                let coord = GridCoord::new(x, 0, z);
                let mut ce = CellEntities::new(coord);
                for i in 0..20 {
                    ce.add_entity((x * 100 + z * 10 + i) as u64);
                }
                cell_entities.insert(coord, ce);
            }
        }
        let query_cells: Vec<GridCoord> = (0..5).map(|x| GridCoord::new(x, 0, 0)).collect();
        b.iter(|| {
            let mut entities = Vec::new();
            for coord in &query_cells {
                if let Some(ce) = cell_entities.get(coord) {
                    entities.extend_from_slice(&ce.entities);
                }
            }
            black_box(entities)
        })
    });

    // Radius-based entity query
    group.bench_function("query_radius_entities", |b| {
        let mut partition = WorldPartition::new(GridConfig::default());
        let mut cell_entities: HashMap<GridCoord, CellEntities> = HashMap::new();

        // Populate grid
        for x in -5..=5 {
            for z in -5..=5 {
                let coord = GridCoord::new(x, 0, z);
                partition.get_or_create_cell(coord);
                let mut ce = CellEntities::new(coord);
                for i in 0..10 {
                    ce.add_entity(((x + 5) * 1000 + (z + 5) * 100 + i) as u64);
                }
                cell_entities.insert(coord, ce);
            }
        }

        let center = Vec3::new(0.0, 0.0, 0.0);
        b.iter(|| {
            let cells = partition.cells_in_radius(center, 300.0);
            let mut entities = Vec::new();
            for coord in &cells {
                if let Some(ce) = cell_entities.get(coord) {
                    entities.extend_from_slice(&ce.entities);
                }
            }
            black_box(entities)
        })
    });

    group.finish();
}

// ============================================================================
// BENCHMARK GROUPS REGISTRATION
// ============================================================================

criterion_group!(
    transform_scene,
    bench_transform_operations,
    bench_scene_graph_operations
);
criterion_group!(
    grid_aabb,
    bench_grid_coord_operations,
    bench_aabb_operations
);
criterion_group!(
    frustum_lru,
    bench_frustum_culling,
    bench_lru_cache_operations
);
criterion_group!(
    partition_budget,
    bench_world_partition_operations,
    bench_gpu_resource_budget
);
criterion_group!(
    cells_queries,
    bench_cell_entity_management,
    bench_spatial_queries
);
