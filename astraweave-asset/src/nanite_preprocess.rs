//! Nanite-inspired meshlet preprocessing pipeline
//!
//! This module provides functionality to convert standard meshes into meshlet-based
//! representations with LOD hierarchies for efficient virtualized geometry rendering.

use anyhow::{Context, Result};
use glam::Vec3;
use meshopt::VertexDataAdapter;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

type SimplifiedMesh = (
    Vec<[f32; 3]>,
    Vec<[f32; 3]>,
    Vec<[f32; 4]>,
    Vec<[f32; 2]>,
    Vec<u32>,
);

/// Maximum vertices per meshlet (typical range: 64-128)
pub const MAX_MESHLET_VERTICES: usize = 64;

/// Maximum triangles per meshlet (typical range: 64-128)
pub const MAX_MESHLET_TRIANGLES: usize = 124;

/// Axis-Aligned Bounding Box
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_points(points: &[[f32; 3]]) -> Self {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);

        for p in points {
            let point = Vec3::from_array(*p);
            min = min.min(point);
            max = max.max(point);
        }

        Self { min, max }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    pub fn diagonal(&self) -> f32 {
        (self.max - self.min).length()
    }

    pub fn contains(&self, point: Vec3) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }

    pub fn merge(&self, other: &AABB) -> AABB {
        AABB {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}

/// Bounding cone for backface culling
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundingCone {
    /// Cone apex position
    pub apex: Vec3,
    /// Cone axis direction (normalized)
    pub axis: Vec3,
    /// Cone cutoff angle (cosine of half-angle)
    pub cutoff: f32,
}

impl BoundingCone {
    /// Create a bounding cone from a set of triangles
    pub fn from_triangles(positions: &[[f32; 3]], normals: &[[f32; 3]], indices: &[u32]) -> Self {
        if indices.is_empty() {
            return Self {
                apex: Vec3::ZERO,
                axis: Vec3::Z,
                cutoff: -1.0,
            };
        }

        // Compute average normal
        let mut avg_normal = Vec3::ZERO;
        for idx in indices.chunks_exact(3) {
            let n0 = Vec3::from_array(normals[idx[0] as usize]);
            let n1 = Vec3::from_array(normals[idx[1] as usize]);
            let n2 = Vec3::from_array(normals[idx[2] as usize]);
            avg_normal += n0 + n1 + n2;
        }
        avg_normal = avg_normal.normalize_or_zero();

        // Compute centroid as apex
        let mut centroid = Vec3::ZERO;
        let mut count = 0;
        for &idx in indices {
            centroid += Vec3::from_array(positions[idx as usize]);
            count += 1;
        }
        centroid /= count as f32;

        // Compute cone angle (find maximum deviation from average normal)
        let mut min_dot = 1.0f32;
        for idx in indices.chunks_exact(3) {
            let n0 = Vec3::from_array(normals[idx[0] as usize]);
            let n1 = Vec3::from_array(normals[idx[1] as usize]);
            let n2 = Vec3::from_array(normals[idx[2] as usize]);

            min_dot = min_dot.min(avg_normal.dot(n0));
            min_dot = min_dot.min(avg_normal.dot(n1));
            min_dot = min_dot.min(avg_normal.dot(n2));
        }

        Self {
            apex: centroid,
            axis: avg_normal,
            cutoff: min_dot.max(-1.0),
        }
    }

    /// Test if the cone is backfacing relative to a view direction
    pub fn is_backfacing(&self, view_dir: Vec3) -> bool {
        self.axis.dot(view_dir) < self.cutoff
    }
}

/// A meshlet: a cluster of triangles with associated metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meshlet {
    /// Local vertex indices (indices into the meshlet's vertex list)
    pub vertices: Vec<u32>,

    /// Triangle indices (triplets of indices into the vertices array)
    pub indices: Vec<u8>,

    /// Bounding box for frustum culling
    pub bounds: AABB,

    /// Bounding cone for backface culling
    pub cone: BoundingCone,

    /// LOD level (0 = highest detail)
    pub lod_level: u32,

    /// LOD error metric (screen-space error threshold)
    pub lod_error: f32,

    /// Parent meshlet index in LOD hierarchy (None for LOD 0)
    pub parent_index: Option<usize>,
}

impl Meshlet {
    /// Create a new meshlet from vertex and index data
    pub fn new(
        vertices: Vec<u32>,
        indices: Vec<u8>,
        positions: &[[f32; 3]],
        normals: &[[f32; 3]],
        lod_level: u32,
    ) -> Self {
        // Compute bounds from vertex positions
        let vertex_positions: Vec<[f32; 3]> = vertices
            .iter()
            .map(|&idx| positions[idx as usize])
            .collect();
        let bounds = AABB::from_points(&vertex_positions);

        // Compute bounding cone from triangles
        let triangle_indices: Vec<u32> = indices
            .chunks_exact(3)
            .flat_map(|tri| {
                vec![
                    vertices[tri[0] as usize],
                    vertices[tri[1] as usize],
                    vertices[tri[2] as usize],
                ]
            })
            .collect();
        let cone = BoundingCone::from_triangles(positions, normals, &triangle_indices);

        Self {
            vertices,
            indices,
            bounds,
            cone,
            lod_level,
            lod_error: 0.0,
            parent_index: None,
        }
    }

    /// Get the number of triangles in this meshlet
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Get the number of vertices in this meshlet
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }
}

/// Complete meshlet hierarchy for a mesh
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshletHierarchy {
    /// All meshlets across all LOD levels
    pub meshlets: Vec<Meshlet>,

    /// Original mesh vertex data
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub tangents: Vec<[f32; 4]>,
    pub uvs: Vec<[f32; 2]>,

    /// Number of LOD levels
    pub lod_count: u32,

    /// Meshlet indices for each LOD level
    pub lod_ranges: Vec<std::ops::Range<usize>>,
}

/// Quadric error metric for mesh simplification
#[derive(Debug, Clone, Copy)]
struct QuadricError {
    /// Quadric matrix (symmetric 4x4)
    q: [[f64; 4]; 4],
}

impl QuadricError {
    fn new() -> Self {
        Self { q: [[0.0; 4]; 4] }
    }

    /// Create quadric from a plane equation (a, b, c, d) where ax + by + cz + d = 0
    fn from_plane(a: f64, b: f64, c: f64, d: f64) -> Self {
        let mut q = [[0.0; 4]; 4];
        q[0][0] = a * a;
        q[0][1] = a * b;
        q[0][2] = a * c;
        q[0][3] = a * d;
        q[1][0] = a * b;
        q[1][1] = b * b;
        q[1][2] = b * c;
        q[1][3] = b * d;
        q[2][0] = a * c;
        q[2][1] = b * c;
        q[2][2] = c * c;
        q[2][3] = c * d;
        q[3][0] = a * d;
        q[3][1] = b * d;
        q[3][2] = c * d;
        q[3][3] = d * d;
        Self { q }
    }

    /// Create quadric from a triangle
    fn from_triangle(p0: Vec3, p1: Vec3, p2: Vec3) -> Self {
        let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();
        let d = -normal.dot(p0);
        Self::from_plane(normal.x as f64, normal.y as f64, normal.z as f64, d as f64)
    }

    /// Add another quadric
    fn add(&self, other: &QuadricError) -> Self {
        let mut result = Self::new();
        for i in 0..4 {
            for j in 0..4 {
                result.q[i][j] = self.q[i][j] + other.q[i][j];
            }
        }
        result
    }

    /// Compute error for a vertex position
    fn error(&self, v: Vec3) -> f64 {
        let x = v.x as f64;
        let y = v.y as f64;
        let z = v.z as f64;
        let w = 1.0;

        let v_vec = [x, y, z, w];
        let mut result = 0.0;

        for i in 0..4 {
            for j in 0..4 {
                result += v_vec[i] * self.q[i][j] * v_vec[j];
            }
        }

        result
    }
}

/// Generate meshlets from a mesh using meshopt's optimized clustering algorithm
pub fn generate_meshlets(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    _tangents: &[[f32; 4]],
    _uvs: &[[f32; 2]],
    indices: &[u32],
) -> Result<Vec<Meshlet>> {
    if indices.len() % 3 != 0 {
        anyhow::bail!("Index count must be a multiple of 3");
    }

    if indices.is_empty() {
        return Ok(Vec::new());
    }

    // Create vertex data adapter for meshopt (positions as raw bytes, zero-copy)
    let position_bytes: &[u8] = bytemuck::cast_slice(positions);
    let vertex_adapter = VertexDataAdapter::new(
        position_bytes,
        std::mem::size_of::<[f32; 3]>(),
        0,
    )
    .context("Failed to create VertexDataAdapter for meshlet generation")?;

    // Use meshopt's optimized meshlet builder (topology-aware spatial clustering)
    let meshopt_meshlets = meshopt::clusterize::build_meshlets(
        indices,
        &vertex_adapter,
        MAX_MESHLET_VERTICES,
        MAX_MESHLET_TRIANGLES,
        0.5, // cone_weight: balance cluster compactness vs backface-culling efficiency
    );

    // Convert meshopt meshlets to our Meshlet format
    let mut meshlets = Vec::with_capacity(meshopt_meshlets.len());
    for i in 0..meshopt_meshlets.len() {
        let m = meshopt_meshlets.get(i);
        let meshlet = Meshlet::new(
            m.vertices.to_vec(),
            m.triangles.to_vec(),
            positions,
            normals,
            0, // LOD 0
        );
        meshlets.push(meshlet);
    }

    Ok(meshlets)
}

/// Generate LOD hierarchy using mesh simplification
pub fn generate_lod_hierarchy(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    tangents: &[[f32; 4]],
    uvs: &[[f32; 2]],
    indices: &[u32],
    lod_count: u32,
) -> Result<MeshletHierarchy> {
    // Generate LOD 0 (highest detail)
    let lod0_meshlets = generate_meshlets(positions, normals, tangents, uvs, indices)?;

    let mut all_meshlets = lod0_meshlets;
    let mut lod_ranges: Vec<std::ops::Range<usize>> = vec![std::ops::Range {
        start: 0,
        end: all_meshlets.len(),
    }];

    // Generate simplified LODs
    let mut current_positions = positions.to_vec();
    let mut current_normals = normals.to_vec();
    let mut current_tangents = tangents.to_vec();
    let mut current_uvs = uvs.to_vec();
    let mut current_indices = indices.to_vec();

    for lod_level in 1..lod_count {
        // Simplify mesh (reduce triangle count by ~50%)
        let target_triangle_count = (current_indices.len() / 3).max(1) / 2;

        let (
            simplified_positions,
            simplified_normals,
            simplified_tangents,
            simplified_uvs,
            simplified_indices,
        ) = simplify_mesh(
            &current_positions,
            &current_normals,
            &current_tangents,
            &current_uvs,
            &current_indices,
            target_triangle_count,
        )?;

        // Generate meshlets for this LOD
        let mut lod_meshlets = generate_meshlets(
            &simplified_positions,
            &simplified_normals,
            &simplified_tangents,
            &simplified_uvs,
            &simplified_indices,
        )?;

        // Set LOD level and compute error metrics
        for meshlet in &mut lod_meshlets {
            meshlet.lod_level = lod_level;
            meshlet.lod_error = compute_lod_error(&meshlet.bounds, lod_level);
        }

        let start = all_meshlets.len();
        all_meshlets.extend(lod_meshlets);
        let end = all_meshlets.len();
        lod_ranges.push(start..end);

        // Update for next iteration
        current_positions = simplified_positions;
        current_normals = simplified_normals;
        current_tangents = simplified_tangents;
        current_uvs = simplified_uvs;
        current_indices = simplified_indices;

        // Stop if we've simplified too much
        if current_indices.len() < 12 {
            break;
        }
    }

    Ok(MeshletHierarchy {
        meshlets: all_meshlets,
        positions: positions.to_vec(),
        normals: normals.to_vec(),
        tangents: tangents.to_vec(),
        uvs: uvs.to_vec(),
        lod_count: lod_ranges.len() as u32,
        lod_ranges,
    })
}

/// Edge for collapse priority queue
#[derive(Debug, Clone)]
struct EdgeCollapse {
    v0: usize,
    v1: usize,
    error: f64,
    optimal_pos: Vec3,
}

impl PartialEq for EdgeCollapse {
    fn eq(&self, other: &Self) -> bool {
        self.error == other.error
    }
}

impl Eq for EdgeCollapse {}

impl PartialOrd for EdgeCollapse {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EdgeCollapse {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse ordering for min-heap (lower error = higher priority)
        other
            .error
            .partial_cmp(&self.error)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Simplify a mesh using quadric error metrics with edge collapse
fn simplify_mesh(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    tangents: &[[f32; 4]],
    uvs: &[[f32; 2]],
    indices: &[u32],
    target_triangle_count: usize,
) -> Result<SimplifiedMesh> {
    use std::collections::{BinaryHeap, HashMap, HashSet};

    let current_triangle_count = indices.len() / 3;
    if current_triangle_count <= target_triangle_count {
        return Ok((
            positions.to_vec(),
            normals.to_vec(),
            tangents.to_vec(),
            uvs.to_vec(),
            indices.to_vec(),
        ));
    }

    // PHASE 1: Compute quadric error matrix for each vertex
    let mut vertex_quadrics: Vec<QuadricError> = vec![QuadricError::new(); positions.len()];

    for tri in indices.chunks_exact(3) {
        let p0 = Vec3::from_array(positions[tri[0] as usize]);
        let p1 = Vec3::from_array(positions[tri[1] as usize]);
        let p2 = Vec3::from_array(positions[tri[2] as usize]);

        let tri_quadric = QuadricError::from_triangle(p0, p1, p2);

        // Accumulate quadric for each vertex
        vertex_quadrics[tri[0] as usize] = vertex_quadrics[tri[0] as usize].add(&tri_quadric);
        vertex_quadrics[tri[1] as usize] = vertex_quadrics[tri[1] as usize].add(&tri_quadric);
        vertex_quadrics[tri[2] as usize] = vertex_quadrics[tri[2] as usize].add(&tri_quadric);
    }

    // PHASE 2: Build edge connectivity graph
    let mut edges: HashSet<(usize, usize)> = HashSet::new();
    let mut vertex_faces: HashMap<usize, HashSet<usize>> = HashMap::new();

    for (face_idx, tri) in indices.chunks_exact(3).enumerate() {
        let v0 = tri[0] as usize;
        let v1 = tri[1] as usize;
        let v2 = tri[2] as usize;

        // Add edges (canonical ordering)
        edges.insert((v0.min(v1), v0.max(v1)));
        edges.insert((v1.min(v2), v1.max(v2)));
        edges.insert((v2.min(v0), v2.max(v0)));

        // Track which faces use each vertex
        vertex_faces.entry(v0).or_default().insert(face_idx);
        vertex_faces.entry(v1).or_default().insert(face_idx);
        vertex_faces.entry(v2).or_default().insert(face_idx);
    }

    // PHASE 3: Build priority queue of edge collapses
    let mut collapse_heap: BinaryHeap<EdgeCollapse> = BinaryHeap::new();

    for &(v0, v1) in &edges {
        let combined_quadric = vertex_quadrics[v0].add(&vertex_quadrics[v1]);

        // Find optimal position for collapsed vertex
        // Simple heuristic: midpoint (full QEF solution would solve for optimal pos)
        let p0 = Vec3::from_array(positions[v0]);
        let p1 = Vec3::from_array(positions[v1]);
        let optimal_pos = (p0 + p1) * 0.5;

        let error = combined_quadric.error(optimal_pos);

        collapse_heap.push(EdgeCollapse {
            v0,
            v1,
            error,
            optimal_pos,
        });
    }

    // PHASE 4: Perform edge collapses until target reached
    let mut collapsed_vertices: HashMap<usize, usize> = HashMap::new(); // Maps old -> new vertex index
    let mut removed_faces: HashSet<usize> = HashSet::new();
    let mut new_positions = positions.to_vec();
    let new_normals = normals.to_vec();
    let new_tangents = tangents.to_vec();
    let new_uvs = uvs.to_vec();

    let target_face_count = target_triangle_count;
    let mut current_face_count = current_triangle_count;

    while current_face_count > target_face_count {
        // Safe: checked !is_empty() in loop condition, but use if let for extra safety
        let Some(collapse) = collapse_heap.pop() else {
            break; // Heap exhausted
        };

        // Skip if vertices already collapsed
        let v0 = *collapsed_vertices.get(&collapse.v0).unwrap_or(&collapse.v0);
        let v1 = *collapsed_vertices.get(&collapse.v1).unwrap_or(&collapse.v1);

        if v0 == v1 {
            continue; // Already collapsed
        }

        // Check if collapse is valid (doesn't create degenerate geometry)
        if let Some(v0_faces) = vertex_faces.get(&v0) {
            if let Some(v1_faces) = vertex_faces.get(&v1) {
                // Count shared faces (these will be removed)
                let shared_faces: Vec<_> = v0_faces.intersection(v1_faces).copied().collect();

                if shared_faces.is_empty() {
                    continue; // No shared faces, invalid collapse
                }

                // Perform collapse: v1 -> v0
                collapsed_vertices.insert(v1, v0);
                new_positions[v0] = collapse.optimal_pos.to_array();

                // Update quadric
                vertex_quadrics[v0] = vertex_quadrics[v0].add(&vertex_quadrics[v1]);

                // Remove shared faces
                for &face_idx in &shared_faces {
                    removed_faces.insert(face_idx);
                    current_face_count -= 1;
                }

                // Update vertex_faces (transfer v1's faces to v0, except shared ones)
                if let Some(v1_faces_owned) = vertex_faces.remove(&v1) {
                    let v0_faces_mut = vertex_faces.entry(v0).or_default();
                    for face_idx in v1_faces_owned {
                        if !shared_faces.contains(&face_idx) {
                            v0_faces_mut.insert(face_idx);
                        }
                    }
                }
            }
        }

        if current_face_count <= target_face_count {
            break;
        }
    }

    // PHASE 5: Rebuild index buffer with collapsed vertices
    let mut new_indices = Vec::new();

    for (face_idx, tri) in indices.chunks_exact(3).enumerate() {
        if removed_faces.contains(&face_idx) {
            continue; // Skip removed faces
        }

        let v0 = *collapsed_vertices
            .get(&(tri[0] as usize))
            .unwrap_or(&(tri[0] as usize)) as u32;
        let v1 = *collapsed_vertices
            .get(&(tri[1] as usize))
            .unwrap_or(&(tri[1] as usize)) as u32;
        let v2 = *collapsed_vertices
            .get(&(tri[2] as usize))
            .unwrap_or(&(tri[2] as usize)) as u32;

        // Skip degenerate triangles
        if v0 == v1 || v1 == v2 || v2 == v0 {
            continue;
        }

        new_indices.push(v0);
        new_indices.push(v1);
        new_indices.push(v2);
    }

    // Ensure we have at least one triangle
    if new_indices.is_empty() && !indices.is_empty() {
        new_indices.extend_from_slice(&indices[0..3.min(indices.len())]);
    }

    Ok((
        new_positions,
        new_normals,
        new_tangents,
        new_uvs,
        new_indices,
    ))
}

/// Compute LOD error metric based on bounds and LOD level
fn compute_lod_error(bounds: &AABB, lod_level: u32) -> f32 {
    // Error increases with LOD level and object size
    let size = bounds.diagonal();
    size * (lod_level as f32 + 1.0) * 0.1
}

/// Async preprocessing pipeline for meshlet generation
pub async fn preprocess_mesh_async(
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    tangents: Vec<[f32; 4]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
    lod_count: u32,
) -> Result<MeshletHierarchy> {
    // Wrap data in Arc for sharing across threads
    let positions = Arc::new(positions);
    let normals = Arc::new(normals);
    let tangents = Arc::new(tangents);
    let uvs = Arc::new(uvs);
    let indices = Arc::new(indices);

    // Spawn blocking task for CPU-intensive work
    let hierarchy = tokio::task::spawn_blocking(move || {
        generate_lod_hierarchy(&positions, &normals, &tangents, &uvs, &indices, lod_count)
    })
    .await
    .context("Failed to spawn blocking task")??;

    Ok(hierarchy)
}

/// Save meshlet hierarchy to file (RON format)
pub fn save_meshlet_hierarchy(hierarchy: &MeshletHierarchy, path: &std::path::Path) -> Result<()> {
    let ron_string = ron::ser::to_string_pretty(hierarchy, ron::ser::PrettyConfig::default())
        .context("Failed to serialize meshlet hierarchy")?;
    std::fs::write(path, ron_string).context("Failed to write meshlet hierarchy file")?;
    Ok(())
}

/// Load meshlet hierarchy from file (RON format)
pub fn load_meshlet_hierarchy(path: &std::path::Path) -> Result<MeshletHierarchy> {
    let ron_string =
        std::fs::read_to_string(path).context("Failed to read meshlet hierarchy file")?;
    let hierarchy: MeshletHierarchy =
        ron::from_str(&ron_string).context("Failed to deserialize meshlet hierarchy")?;
    Ok(hierarchy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_creation() {
        let points = vec![[0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [-1.0, -1.0, -1.0]];
        let aabb = AABB::from_points(&points);

        assert_eq!(aabb.min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(aabb.max, Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(aabb.center(), Vec3::ZERO);
    }

    #[test]
    fn test_aabb_contains() {
        let aabb = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));

        assert!(aabb.contains(Vec3::ZERO));
        assert!(aabb.contains(Vec3::new(0.5, 0.5, 0.5)));
        assert!(!aabb.contains(Vec3::new(2.0, 0.0, 0.0)));
    }

    #[test]
    fn test_aabb_merge() {
        let aabb1 = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let aabb2 = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(0.5, 0.5, 0.5));

        let merged = aabb1.merge(&aabb2);
        assert_eq!(merged.min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(merged.max, Vec3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_aabb_diagonal() {
        let aabb = AABB::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));
        let diagonal = aabb.diagonal();
        assert!((diagonal - 1.732).abs() < 0.01); // sqrt(3)
    }

    #[test]
    fn test_meshlet_generation() {
        // Create a simple quad (2 triangles)
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let normals = vec![
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ];
        let tangents = vec![
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
        ];
        let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let indices = vec![0, 1, 2, 0, 2, 3];

        let meshlets = generate_meshlets(&positions, &normals, &tangents, &uvs, &indices).unwrap();

        assert!(!meshlets.is_empty());
        assert_eq!(meshlets[0].triangle_count(), 2);
        assert_eq!(meshlets[0].vertex_count(), 4);
    }

    #[test]
    fn test_meshlet_generation_large_mesh() {
        // Create a larger mesh to test clustering
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut tangents = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        // Create a grid of quads
        for y in 0..10 {
            for x in 0..10 {
                let base_idx = positions.len() as u32;

                // Add 4 vertices for this quad
                positions.push([x as f32, y as f32, 0.0]);
                positions.push([(x + 1) as f32, y as f32, 0.0]);
                positions.push([(x + 1) as f32, (y + 1) as f32, 0.0]);
                positions.push([x as f32, (y + 1) as f32, 0.0]);

                for _ in 0..4 {
                    normals.push([0.0, 0.0, 1.0]);
                    tangents.push([1.0, 0.0, 0.0, 1.0]);
                    uvs.push([0.0, 0.0]);
                }

                // Add 2 triangles
                indices.extend_from_slice(&[
                    base_idx,
                    base_idx + 1,
                    base_idx + 2,
                    base_idx,
                    base_idx + 2,
                    base_idx + 3,
                ]);
            }
        }

        let meshlets = generate_meshlets(&positions, &normals, &tangents, &uvs, &indices).unwrap();

        assert!(!meshlets.is_empty());

        // Verify all triangles are accounted for
        let total_triangles: usize = meshlets.iter().map(|m| m.triangle_count()).sum();
        assert_eq!(total_triangles, 200); // 10x10 grid = 100 quads = 200 triangles
    }

    #[test]
    fn test_bounding_cone() {
        let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]];
        let normals = vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
        let indices = vec![0, 1, 2];

        let cone = BoundingCone::from_triangles(&positions, &normals, &indices);

        // Cone should point in +Z direction
        assert!(cone.axis.z > 0.9);

        // Should be backfacing when viewed from -Z
        assert!(cone.is_backfacing(Vec3::new(0.0, 0.0, -1.0)));

        // Should not be backfacing when viewed from +Z
        assert!(!cone.is_backfacing(Vec3::new(0.0, 0.0, 1.0)));
    }

    #[test]
    fn test_lod_hierarchy_generation() {
        // Create a simple mesh
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let normals = vec![
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ];
        let tangents = vec![
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
        ];
        let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let indices = vec![0, 1, 2, 0, 2, 3];

        let hierarchy =
            generate_lod_hierarchy(&positions, &normals, &tangents, &uvs, &indices, 3).unwrap();

        assert!(hierarchy.lod_count > 0);
        assert!(!hierarchy.meshlets.is_empty());
        assert_eq!(hierarchy.lod_ranges.len(), hierarchy.lod_count as usize);

        // Verify LOD 0 exists
        assert!(!hierarchy.lod_ranges[0].is_empty());
    }

    #[test]
    fn test_quadric_error() {
        let q1 = QuadricError::from_plane(1.0, 0.0, 0.0, 0.0);
        let q2 = QuadricError::from_plane(0.0, 1.0, 0.0, 0.0);

        let combined = q1.add(&q2);

        // Error at origin should be 0
        let error = combined.error(Vec3::ZERO);
        assert!(error.abs() < 0.001);

        // Error should increase with distance
        let error_far = combined.error(Vec3::new(1.0, 1.0, 0.0));
        assert!(error_far > error);
    }

    #[test]
    fn test_lod_error_computation() {
        let aabb = AABB::new(Vec3::ZERO, Vec3::new(10.0, 10.0, 10.0));

        let error_lod0 = compute_lod_error(&aabb, 0);
        let error_lod1 = compute_lod_error(&aabb, 1);
        let error_lod2 = compute_lod_error(&aabb, 2);

        // Error should increase with LOD level
        assert!(error_lod1 > error_lod0);
        assert!(error_lod2 > error_lod1);
    }

    #[test]
    fn test_meshlet_serialization() {
        let meshlet = Meshlet {
            vertices: vec![0, 1, 2, 3],
            indices: vec![0, 1, 2, 0, 2, 3],
            bounds: AABB::new(Vec3::ZERO, Vec3::ONE),
            cone: BoundingCone {
                apex: Vec3::ZERO,
                axis: Vec3::Z,
                cutoff: 0.5,
            },
            lod_level: 0,
            lod_error: 0.1,
            parent_index: None,
        };

        // Test serialization
        let serialized = ron::ser::to_string(&meshlet).unwrap();
        let deserialized: Meshlet = ron::from_str(&serialized).unwrap();

        assert_eq!(meshlet.vertices, deserialized.vertices);
        assert_eq!(meshlet.indices, deserialized.indices);
        assert_eq!(meshlet.lod_level, deserialized.lod_level);
    }
}
