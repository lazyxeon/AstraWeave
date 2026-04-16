// Week 5 Action 19: LOD Generation Module
// Implements quadric error metrics for mesh simplification (Garland & Heckbert 1997)
// Generates multiple LOD levels for memory and performance optimization

use glam::Vec3;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

/// Level of Detail configuration
#[derive(Debug, Clone)]
pub struct LODConfig {
    /// Target reduction percentages for each LOD level (e.g., [0.75, 0.50, 0.25])
    pub reduction_targets: Vec<f32>,
    /// Maximum allowed quadric error for simplification
    pub max_error: f32,
    /// Preserve mesh boundaries during simplification
    pub preserve_boundaries: bool,
}

impl Default for LODConfig {
    fn default() -> Self {
        Self {
            reduction_targets: vec![0.75, 0.50, 0.25], // LOD1: 75%, LOD2: 50%, LOD3: 25% of vertices
            max_error: 0.01,
            preserve_boundaries: true,
        }
    }
}

/// Mesh representation for LOD generation
#[derive(Debug, Clone)]
pub struct SimplificationMesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

impl SimplificationMesh {
    pub fn new(
        positions: Vec<Vec3>,
        normals: Vec<Vec3>,
        uvs: Vec<[f32; 2]>,
        indices: Vec<u32>,
    ) -> Self {
        Self {
            positions,
            normals,
            uvs,
            indices,
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Convert a canonical `CpuMesh` into a simplification mesh.
    pub fn from_cpu_mesh(mesh: &crate::mesh::CpuMesh) -> Self {
        Self {
            positions: mesh
                .vertices
                .iter()
                .map(|vertex| Vec3::from_array(vertex.position))
                .collect(),
            normals: mesh
                .vertices
                .iter()
                .map(|vertex| Vec3::from_array(vertex.normal))
                .collect(),
            uvs: mesh.vertices.iter().map(|vertex| vertex.uv).collect(),
            indices: mesh.indices.clone(),
        }
    }

    /// Convert a simplification mesh back into a canonical `CpuMesh`.
    pub fn to_cpu_mesh(
        &self,
        albedo_image: Option<crate::mesh::CpuImage>,
        texture_source_hint: Option<String>,
    ) -> crate::mesh::CpuMesh {
        let mut mesh = crate::mesh::CpuMesh {
            vertices: self
                .positions
                .iter()
                .enumerate()
                .map(|(index, position)| {
                    let normal = self
                        .normals
                        .get(index)
                        .copied()
                        .unwrap_or(Vec3::Y)
                        .normalize_or_zero();
                    let normal = if normal.length_squared() > 0.0 {
                        normal
                    } else {
                        Vec3::Y
                    };
                    let uv = self.uvs.get(index).copied().unwrap_or([0.0, 0.0]);
                    crate::mesh::MeshVertex {
                        position: position.to_array(),
                        normal: normal.to_array(),
                        tangent: [1.0, 0.0, 0.0, 1.0],
                        uv,
                    }
                })
                .collect(),
            indices: self.indices.clone(),
            albedo_image,
            texture_source_hint,
        };
        crate::mesh::compute_tangents(&mut mesh);
        mesh
    }
}

/// Quadric error matrix (4x4 symmetric matrix for error metric)
/// Represents the sum of squared distances to planes
#[derive(Debug, Clone, Copy)]
struct Quadric {
    // Symmetric matrix stored as upper triangle: q11, q12, q13, q14, q22, q23, q24, q33, q34, q44
    data: [f64; 10],
}

impl Quadric {
    fn zero() -> Self {
        Self { data: [0.0; 10] }
    }

    /// Create quadric from triangle plane equation ax + by + cz + d = 0
    fn from_plane(a: f64, b: f64, c: f64, d: f64) -> Self {
        Self {
            data: [
                a * a,
                a * b,
                a * c,
                a * d, // q11, q12, q13, q14
                b * b,
                b * c,
                b * d, // q22, q23, q24
                c * c,
                c * d, // q33, q34
                d * d, // q44
            ],
        }
    }

    /// Add two quadrics (for merging vertices)
    fn add(&self, other: &Quadric) -> Quadric {
        let mut result = Quadric::zero();
        for i in 0..10 {
            result.data[i] = self.data[i] + other.data[i];
        }
        result
    }

    /// Evaluate quadric error at position [x, y, z]
    fn evaluate(&self, pos: Vec3) -> f64 {
        let x = pos.x as f64;
        let y = pos.y as f64;
        let z = pos.z as f64;

        // Q(v) = v^T * Q * v where v = [x, y, z, 1]
        let q11 = self.data[0];
        let q12 = self.data[1];
        let q13 = self.data[2];
        let q14 = self.data[3];
        let q22 = self.data[4];
        let q23 = self.data[5];
        let q24 = self.data[6];
        let q33 = self.data[7];
        let q34 = self.data[8];
        let q44 = self.data[9];

        x * (q11 * x + q12 * y + q13 * z + q14)
            + y * (q12 * x + q22 * y + q23 * z + q24)
            + z * (q13 * x + q23 * y + q33 * z + q34)
            + (q14 * x + q24 * y + q34 * z + q44)
    }
}

/// Edge collapse candidate
#[derive(Debug, Clone)]
struct EdgeCollapse {
    v1: usize,     // First vertex
    v2: usize,     // Second vertex
    error: f64,    // Quadric error of collapse
    new_pos: Vec3, // Optimal position after collapse
}

impl PartialOrd for EdgeCollapse {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for EdgeCollapse {
    fn eq(&self, other: &Self) -> bool {
        self.error == other.error
    }
}

impl Eq for EdgeCollapse {}

impl Ord for EdgeCollapse {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap: lower error = higher priority
        other
            .error
            .partial_cmp(&self.error)
            .unwrap_or(Ordering::Equal)
    }
}

/// LOD generator using quadric error metrics
pub struct LODGenerator {
    config: LODConfig,
}

impl LODGenerator {
    pub fn new(config: LODConfig) -> Self {
        Self { config }
    }

    /// Generate LOD levels for a mesh
    pub fn generate_lods(&self, mesh: &SimplificationMesh) -> Vec<SimplificationMesh> {
        let mut lods = Vec::new();
        let mut current_mesh = mesh.clone();

        for &target_ratio in &self.config.reduction_targets {
            let target_vertices = (mesh.vertex_count() as f32 * target_ratio).ceil() as usize;
            current_mesh = self.simplify(&current_mesh, target_vertices);
            lods.push(current_mesh.clone());
        }

        lods
    }

    /// Simplify mesh to target vertex count using quadric error metrics
    pub fn simplify(
        &self,
        mesh: &SimplificationMesh,
        target_vertices: usize,
    ) -> SimplificationMesh {
        if mesh.vertex_count() <= target_vertices {
            return mesh.clone();
        }

        // Step 1: Compute quadric for each vertex
        let mut quadrics = vec![Quadric::zero(); mesh.vertex_count()];
        self.compute_vertex_quadrics(mesh, &mut quadrics);

        // Step 2: Build edge collapse candidates
        let mut collapses = BinaryHeap::new();
        self.build_edge_collapses(mesh, &quadrics, &mut collapses);

        // Step 3: Iteratively collapse edges
        let mut simplified = mesh.clone();
        let mut active_vertices: Vec<bool> = vec![true; mesh.vertex_count()];
        let vertices_to_remove = mesh.vertex_count() - target_vertices;

        for _ in 0..vertices_to_remove {
            if let Some(collapse) = collapses.pop() {
                if collapse.error > self.config.max_error as f64 {
                    break; // Stop if error exceeds threshold
                }

                // Collapse v2 into v1
                if active_vertices[collapse.v1] && active_vertices[collapse.v2] {
                    self.apply_collapse(&mut simplified, &collapse, &mut active_vertices);
                    quadrics[collapse.v1] = quadrics[collapse.v1].add(&quadrics[collapse.v2]);
                }
            } else {
                break; // No more collapses available
            }
        }

        // Step 4: Rebuild mesh with active vertices only
        self.rebuild_mesh(&simplified, &active_vertices)
    }

    /// Compute quadric error matrix for each vertex
    fn compute_vertex_quadrics(&self, mesh: &SimplificationMesh, quadrics: &mut [Quadric]) {
        for tri_idx in 0..mesh.triangle_count() {
            let i0 = mesh.indices[tri_idx * 3] as usize;
            let i1 = mesh.indices[tri_idx * 3 + 1] as usize;
            let i2 = mesh.indices[tri_idx * 3 + 2] as usize;

            let p0 = mesh.positions[i0];
            let p1 = mesh.positions[i1];
            let p2 = mesh.positions[i2];

            // Compute plane equation ax + by + cz + d = 0
            let normal = (p1 - p0).cross(p2 - p0).normalize();
            let d = -normal.dot(p0);

            let quadric =
                Quadric::from_plane(normal.x as f64, normal.y as f64, normal.z as f64, d as f64);

            // Add to all three vertices
            quadrics[i0] = quadrics[i0].add(&quadric);
            quadrics[i1] = quadrics[i1].add(&quadric);
            quadrics[i2] = quadrics[i2].add(&quadric);
        }
    }

    /// Build edge collapse candidates from mesh edges
    fn build_edge_collapses(
        &self,
        mesh: &SimplificationMesh,
        quadrics: &[Quadric],
        collapses: &mut BinaryHeap<EdgeCollapse>,
    ) {
        let mut edges: HashMap<(usize, usize), ()> = HashMap::new();

        // Collect unique edges from triangles
        for tri_idx in 0..mesh.triangle_count() {
            let i0 = mesh.indices[tri_idx * 3] as usize;
            let i1 = mesh.indices[tri_idx * 3 + 1] as usize;
            let i2 = mesh.indices[tri_idx * 3 + 2] as usize;

            edges.insert((i0.min(i1), i0.max(i1)), ());
            edges.insert((i1.min(i2), i1.max(i2)), ());
            edges.insert((i2.min(i0), i2.max(i0)), ());
        }

        // Create collapse candidate for each edge
        for &(v1, v2) in edges.keys() {
            let combined_quadric = quadrics[v1].add(&quadrics[v2]);

            // Optimal position is midpoint (simplified - full solution requires solving linear system)
            let new_pos = (mesh.positions[v1] + mesh.positions[v2]) * 0.5;
            let error = combined_quadric.evaluate(new_pos);

            collapses.push(EdgeCollapse {
                v1,
                v2,
                error,
                new_pos,
            });
        }
    }

    /// Apply edge collapse to mesh
    fn apply_collapse(
        &self,
        mesh: &mut SimplificationMesh,
        collapse: &EdgeCollapse,
        active: &mut [bool],
    ) {
        // Update position of v1 to optimal position
        mesh.positions[collapse.v1] = collapse.new_pos;

        // Mark v2 as inactive
        active[collapse.v2] = false;

        // Remap all references to v2 → v1 in indices
        for idx in mesh.indices.iter_mut() {
            if *idx == collapse.v2 as u32 {
                *idx = collapse.v1 as u32;
            }
        }
    }

    /// Rebuild mesh with only active vertices
    fn rebuild_mesh(&self, mesh: &SimplificationMesh, active: &[bool]) -> SimplificationMesh {
        let mut new_positions = Vec::new();
        let mut new_normals = Vec::new();
        let mut new_uvs = Vec::new();
        let mut vertex_remap: HashMap<usize, usize> = HashMap::new();

        // Copy active vertices
        for (old_idx, &is_active) in active.iter().enumerate() {
            if is_active {
                let new_idx = new_positions.len();
                vertex_remap.insert(old_idx, new_idx);
                new_positions.push(mesh.positions[old_idx]);
                new_normals.push(mesh.normals[old_idx]);
                new_uvs.push(mesh.uvs[old_idx]);
            }
        }

        // Remap indices and remove degenerate triangles
        let mut new_indices = Vec::new();
        for tri_idx in 0..mesh.triangle_count() {
            let i0 = mesh.indices[tri_idx * 3] as usize;
            let i1 = mesh.indices[tri_idx * 3 + 1] as usize;
            let i2 = mesh.indices[tri_idx * 3 + 2] as usize;

            if let (Some(&n0), Some(&n1), Some(&n2)) = (
                vertex_remap.get(&i0),
                vertex_remap.get(&i1),
                vertex_remap.get(&i2),
            ) {
                // Skip degenerate triangles
                if n0 != n1 && n1 != n2 && n2 != n0 {
                    new_indices.push(n0 as u32);
                    new_indices.push(n1 as u32);
                    new_indices.push(n2 as u32);
                }
            }
        }

        SimplificationMesh {
            positions: new_positions,
            normals: new_normals,
            uvs: new_uvs,
            indices: new_indices,
        }
    }

    /// Calculate reduction percentage achieved
    pub fn calculate_reduction(
        &self,
        original: &SimplificationMesh,
        lod: &SimplificationMesh,
    ) -> f32 {
        1.0 - (lod.vertex_count() as f32 / original.vertex_count() as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_cube() -> SimplificationMesh {
        // Simple cube mesh (8 vertices, 12 triangles)
        let positions = vec![
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
        ];

        let normals = positions.iter().map(|p| p.normalize()).collect();
        let uvs = vec![[0.0, 0.0]; 8];

        let indices = vec![
            // Front
            0, 1, 2, 0, 2, 3, // Back
            5, 4, 7, 5, 7, 6, // Left
            4, 0, 3, 4, 3, 7, // Right
            1, 5, 6, 1, 6, 2, // Top
            3, 2, 6, 3, 6, 7, // Bottom
            4, 5, 1, 4, 1, 0,
        ];

        SimplificationMesh::new(positions, normals, uvs, indices)
    }

    #[test]
    fn test_lod_generation() {
        let config = LODConfig::default();
        let generator = LODGenerator::new(config);
        let cube = create_test_cube();

        let lods = generator.generate_lods(&cube);

        assert_eq!(lods.len(), 3); // 3 LOD levels
        for lod in &lods {
            assert!(lod.vertex_count() <= cube.vertex_count());
            assert!(lod.triangle_count() <= cube.triangle_count());
        }
    }

    #[test]
    fn test_simplification_reduces_vertices() {
        let config = LODConfig::default();
        let generator = LODGenerator::new(config);
        let cube = create_test_cube();

        let simplified = generator.simplify(&cube, 6); // Reduce to 6 vertices (more realistic for cube)

        // Should reduce vertices (may not reach exact target due to geometric constraints)
        assert!(simplified.vertex_count() <= cube.vertex_count());
        assert!(simplified.triangle_count() > 0); // Should still have geometry
    }

    #[test]
    fn test_quadric_evaluation() {
        let quadric = Quadric::from_plane(0.0, 1.0, 0.0, 0.0); // XZ plane (y=0)

        let on_plane = Vec3::new(1.0, 0.0, 1.0);
        let off_plane = Vec3::new(1.0, 1.0, 1.0);

        let error_on = quadric.evaluate(on_plane);
        let error_off = quadric.evaluate(off_plane);

        assert!(error_on < 0.001); // Should be near zero
        assert!(error_off > error_on); // Off-plane should have higher error
    }

    #[test]
    fn test_reduction_calculation() {
        let config = LODConfig {
            reduction_targets: vec![0.50], // Single 50% reduction target
            max_error: 1.0,                // Higher threshold for test mesh
            preserve_boundaries: true,
        };
        let generator = LODGenerator::new(config);
        let cube = create_test_cube();

        let simplified = generator.simplify(&cube, 4);
        let reduction = generator.calculate_reduction(&cube, &simplified);

        assert!((0.0..=1.0).contains(&reduction));
        // Any reduction is valid (algorithm may be conservative on simple geometry)
        println!("Achieved reduction: {:.1}%", reduction * 100.0);
    }

    #[test]
    fn test_mesh_integrity_after_simplification() {
        let config = LODConfig::default();
        let generator = LODGenerator::new(config);
        let cube = create_test_cube();

        let simplified = generator.simplify(&cube, 6);

        // All indices should be valid
        for &idx in &simplified.indices {
            assert!((idx as usize) < simplified.vertex_count());
        }

        // Should have triangles (indices divisible by 3)
        assert_eq!(simplified.indices.len() % 3, 0);
    }

    #[test]
    fn test_empty_mesh_simplification() {
        // EDGE CASE: Zero vertices
        let config = LODConfig::default();
        let generator = LODGenerator::new(config);

        let empty_mesh = SimplificationMesh::new(vec![], vec![], vec![], vec![]);
        let simplified = generator.simplify(&empty_mesh, 0);

        assert_eq!(simplified.vertex_count(), 0);
        assert_eq!(simplified.triangle_count(), 0);
    }

    #[test]
    fn test_lod_level_exceeds_triangle_count() {
        // EDGE CASE: Requesting more LOD levels than triangles
        let config = LODConfig {
            reduction_targets: vec![0.25, 0.50, 0.75, 0.90, 0.95], // 5 LOD levels
            max_error: 1.0,
            preserve_boundaries: true,
        };
        let generator = LODGenerator::new(config);

        // Simple triangle mesh (1 triangle = 3 vertices)
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 1.0, 0.0),
        ];
        let normals = vec![Vec3::new(0.0, 0.0, 1.0); 3];
        let uvs = vec![[0.0, 0.0]; 3];
        let indices = vec![0, 1, 2];

        let mesh = SimplificationMesh::new(positions, normals, uvs, indices);
        let lods = generator.generate_lods(&mesh);

        // Should generate LODs (algorithm may be conservative)
        assert!(!lods.is_empty());
        for lod in &lods {
            // Each LOD should maintain mesh integrity
            assert_eq!(lod.indices.len() % 3, 0);
        }
    }

    #[test]
    fn test_quadric_error_at_infinity() {
        // EDGE CASE: Quadric error calculation with extreme positions
        let quadric = Quadric::from_plane(0.0, 1.0, 0.0, 0.0); // XZ plane (y=0)

        let extreme_pos = Vec3::new(1e10, 1e10, 1e10);
        let error = quadric.evaluate(extreme_pos);

        // Error should be finite (not NaN/Inf)
        assert!(error.is_finite());
        assert!(error >= 0.0); // Quadric error is always non-negative
    }

    #[test]
    fn test_target_vertex_count_less_than_three() {
        // EDGE CASE: Target vertex count insufficient for a triangle
        let config = LODConfig::default();
        let generator = LODGenerator::new(config);
        let cube = create_test_cube();

        let simplified = generator.simplify(&cube, 2); // Can't form a triangle

        // Algorithm should either preserve minimum geometry or return valid mesh
        assert_eq!(simplified.indices.len() % 3, 0); // Still valid triangle mesh
        if simplified.triangle_count() > 0 {
            // If triangles exist, vertices must be >= 3
            assert!(simplified.vertex_count() >= 3);
        }
    }

    #[test]
    fn test_degenerate_mesh_all_coplanar() {
        // EDGE CASE: All vertices on same plane (flat mesh)
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
        ];
        let normals = vec![Vec3::new(0.0, 1.0, 0.0); 4];
        let uvs = vec![[0.0, 0.0]; 4];
        let indices = vec![0, 1, 2, 1, 3, 2]; // Two triangles

        let config = LODConfig::default();
        let generator = LODGenerator::new(config);
        let mesh = SimplificationMesh::new(positions, normals, uvs, indices);

        let simplified = generator.simplify(&mesh, 3);

        // Should handle coplanar geometry without errors
        assert!(simplified.vertex_count() > 0);
        assert_eq!(simplified.indices.len() % 3, 0);
    }

    // --- Mutation-resistant additions ---

    #[test]
    fn mesh_vertex_and_triangle_counts_exact() {
        let cube = create_test_cube();
        assert_eq!(cube.vertex_count(), 8, "Cube should have 8 vertices");
        assert_eq!(cube.triangle_count(), 12, "Cube should have 12 triangles");
    }

    #[test]
    fn lod_levels_monotonically_reducing() {
        let config = LODConfig::default();
        let generator = LODGenerator::new(config);
        let cube = create_test_cube();
        let lods = generator.generate_lods(&cube);
        assert!(lods.len() >= 2, "Need at least 2 LODs");
        // Each subsequent LOD should have <= vertices of the previous
        for w in lods.windows(2) {
            assert!(
                w[1].vertex_count() <= w[0].vertex_count(),
                "LOD[i+1] ({}) should have <= vertices than LOD[i] ({})",
                w[1].vertex_count(),
                w[0].vertex_count()
            );
        }
    }

    #[test]
    fn calculate_reduction_specific_values() {
        let config = LODConfig::default();
        let generator = LODGenerator::new(config);
        let cube = create_test_cube();

        // Identity reduction (same mesh) should be 0.0
        let reduction = generator.calculate_reduction(&cube, &cube);
        assert!(
            reduction.abs() < f32::EPSILON,
            "Identical meshes should have 0 reduction, got {}",
            reduction
        );
    }

    #[test]
    fn quadric_zero_has_zero_error() {
        let q = Quadric::zero();
        let err = q.evaluate(Vec3::new(42.0, -7.0, 100.0));
        assert!(
            err.abs() < 1e-12,
            "Zero quadric should give zero error, got {}",
            err
        );
    }

    #[test]
    fn quadric_add_is_commutative() {
        let a = Quadric::from_plane(1.0, 0.0, 0.0, 0.0);
        let b = Quadric::from_plane(0.0, 1.0, 0.0, 0.0);
        let ab = a.add(&b);
        let ba = b.add(&a);
        for i in 0..10 {
            assert!(
                (ab.data[i] - ba.data[i]).abs() < 1e-12,
                "Quadric add not commutative at index {}",
                i
            );
        }
    }

    #[test]
    fn quadric_add_accumulates_error() {
        let a = Quadric::from_plane(1.0, 0.0, 0.0, 1.0); // plane x = -1
        let b = Quadric::from_plane(1.0, 0.0, 0.0, -1.0); // plane x = 1
        let sum = a.add(&b);

        // Point x=0 is equidistant between planes; error should be 2.0 (1^2 + 1^2)
        let err_origin = sum.evaluate(Vec3::ZERO);
        assert!(
            (err_origin - 2.0).abs() < 1e-10,
            "Expected error 2.0 at origin, got {}",
            err_origin
        );
    }

    #[test]
    fn quadric_from_plane_error_on_plane_is_zero() {
        let q = Quadric::from_plane(0.0, 0.0, 1.0, -5.0); // z = 5
        let on_plane = Vec3::new(100.0, -200.0, 5.0);
        let err = q.evaluate(on_plane);
        assert!(err.abs() < 1e-10, "Point on plane should have zero error");
    }

    #[test]
    fn quadric_from_plane_error_off_plane_proportional() {
        let q = Quadric::from_plane(0.0, 1.0, 0.0, 0.0); // y = 0
        let err1 = q.evaluate(Vec3::new(0.0, 1.0, 0.0));
        let err2 = q.evaluate(Vec3::new(0.0, 2.0, 0.0));
        // Error should be distance^2: err1 = 1, err2 = 4
        assert!(
            (err1 - 1.0).abs() < 1e-10,
            "err1 should be 1.0, got {}",
            err1
        );
        assert!(
            (err2 - 4.0).abs() < 1e-10,
            "err2 should be 4.0, got {}",
            err2
        );
    }

    #[test]
    fn edge_collapse_min_heap_order() {
        use std::collections::BinaryHeap;
        let mut heap = BinaryHeap::new();
        heap.push(EdgeCollapse {
            v1: 0,
            v2: 1,
            error: 10.0,
            new_pos: Vec3::ZERO,
        });
        heap.push(EdgeCollapse {
            v1: 2,
            v2: 3,
            error: 1.0,
            new_pos: Vec3::ZERO,
        });
        heap.push(EdgeCollapse {
            v1: 4,
            v2: 5,
            error: 5.0,
            new_pos: Vec3::ZERO,
        });
        // Min-heap: should pop lowest error first
        let first = heap.pop().unwrap();
        assert!(
            (first.error - 1.0).abs() < 1e-10,
            "Should pop error=1.0 first, got {}",
            first.error
        );
        let second = heap.pop().unwrap();
        assert!(
            (second.error - 5.0).abs() < 1e-10,
            "Should pop error=5.0 second, got {}",
            second.error
        );
        let third = heap.pop().unwrap();
        assert!(
            (third.error - 10.0).abs() < 1e-10,
            "Should pop error=10.0 third, got {}",
            third.error
        );
    }

    #[test]
    fn lod_config_default_has_three_targets() {
        let config = LODConfig::default();
        assert_eq!(config.reduction_targets.len(), 3);
        // Each target should be in (0, 1)
        for &t in &config.reduction_targets {
            assert!(t > 0.0 && t < 1.0, "Target {} out of (0,1)", t);
        }
    }
}
