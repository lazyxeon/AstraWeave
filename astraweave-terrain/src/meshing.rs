//! Isosurface Generation using Dual Contouring
//!
//! This module implements the Dual Contouring algorithm to convert voxel data
//! into smooth polygon meshes. Dual Contouring is preferred over Marching Cubes
//! because it:
//! - Preserves sharp features better
//! - Produces fewer artifacts
//! - Generates more uniform triangles
//! - Handles hermite data (density + gradient)

use crate::marching_cubes_tables::{EDGE_ENDPOINTS, MC_EDGE_TABLE, MC_TRI_TABLE};
use crate::voxel_data::{ChunkCoord, Voxel, VoxelChunk, CHUNK_SIZE};
use glam::{IVec3, Vec3};
use std::collections::{HashMap, HashSet};

/// Vertex data for mesh generation
#[derive(Debug, Clone, Copy)]
pub struct MeshVertex {
    /// Position in world space
    pub position: Vec3,
    /// Normal vector
    pub normal: Vec3,
    /// Material ID
    pub material: u16,
}

/// Generated mesh data from voxel chunk
#[derive(Debug, Clone)]
pub struct ChunkMesh {
    /// Chunk coordinate
    pub coord: ChunkCoord,
    /// Vertex data
    pub vertices: Vec<MeshVertex>,
    /// Index buffer (triangles)
    pub indices: Vec<u32>,
}

impl ChunkMesh {
    /// Create an empty mesh
    pub fn empty(coord: ChunkCoord) -> Self {
        Self {
            coord,
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Check if mesh is empty
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Get memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.vertices.len() * std::mem::size_of::<MeshVertex>()
            + self.indices.len() * std::mem::size_of::<u32>()
    }

    /// Add vertical skirt geometry along chunk boundary edges to hide LOD seam cracks.
    ///
    /// For each mesh edge where both vertices lie on the same chunk boundary face,
    /// creates a quad (2 triangles) dropping the edge downward by `skirt_depth`.
    /// This fills any gaps between adjacent chunks at different LOD levels.
    ///
    /// Skirts are generated for all 6 faces (±X, ±Y, ±Z). Typical `skirt_depth`
    /// values of 2.0–4.0 world units hide cracks without visible artifacts.
    pub fn add_skirts(&mut self, skirt_depth: f32, chunk_origin: Vec3) {
        if self.vertices.is_empty() || skirt_depth <= 0.0 {
            return;
        }

        let chunk_extent = CHUNK_SIZE as f32;
        // Boundary detection tolerance: dual-contouring vertices sit at cell centers
        // (~0.5 to ~CHUNK_SIZE-1.5 in local space), so a 1.5 tolerance catches them.
        let eps = 1.5;

        // For each vertex, determine which boundary faces it belongs to.
        // Faces: 0=x_min, 1=x_max, 2=y_min, 3=y_max, 4=z_min, 5=z_max
        let face_membership: Vec<[bool; 6]> = self
            .vertices
            .iter()
            .map(|v| {
                let local = v.position - chunk_origin;
                [
                    local.x < eps,
                    local.x > chunk_extent - eps - 1.0,
                    local.y < eps,
                    local.y > chunk_extent - eps - 1.0,
                    local.z < eps,
                    local.z > chunk_extent - eps - 1.0,
                ]
            })
            .collect();

        let mut skirt_vertices: Vec<MeshVertex> = Vec::new();
        let mut skirt_indices: Vec<u32> = Vec::new();
        let base = self.vertices.len() as u32;
        let mut processed_edges: HashSet<(u32, u32)> = HashSet::new();

        let drop_offset = Vec3::new(0.0, -skirt_depth, 0.0);

        let num_triangles = self.indices.len() / 3;
        for tri in 0..num_triangles {
            let idx = [
                self.indices[tri * 3] as usize,
                self.indices[tri * 3 + 1] as usize,
                self.indices[tri * 3 + 2] as usize,
            ];

            for &(a, b) in &[(idx[0], idx[1]), (idx[1], idx[2]), (idx[2], idx[0])] {
                let edge_key = if a < b {
                    (a as u32, b as u32)
                } else {
                    (b as u32, a as u32)
                };
                if processed_edges.contains(&edge_key) {
                    continue;
                }

                // Check if both vertices share any boundary face
                let mut is_boundary_edge = false;
                for face in 0..6 {
                    if face_membership[a][face] && face_membership[b][face] {
                        is_boundary_edge = true;
                        break;
                    }
                }

                if is_boundary_edge {
                    processed_edges.insert(edge_key);

                    let skirt_idx_a = base + skirt_vertices.len() as u32;
                    skirt_vertices.push(MeshVertex {
                        position: self.vertices[a].position + drop_offset,
                        normal: self.vertices[a].normal,
                        material: self.vertices[a].material,
                    });

                    let skirt_idx_b = base + skirt_vertices.len() as u32;
                    skirt_vertices.push(MeshVertex {
                        position: self.vertices[b].position + drop_offset,
                        normal: self.vertices[b].normal,
                        material: self.vertices[b].material,
                    });

                    // Skirt quad: original edge (a, b) connected to dropped edge
                    skirt_indices.push(a as u32);
                    skirt_indices.push(b as u32);
                    skirt_indices.push(skirt_idx_a);

                    skirt_indices.push(skirt_idx_a);
                    skirt_indices.push(b as u32);
                    skirt_indices.push(skirt_idx_b);
                }
            }
        }

        self.vertices.extend(skirt_vertices);
        self.indices.extend(skirt_indices);
    }
}

/// Edge key for vertex deduplication
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EdgeKey {
    min: IVec3,
    max: IVec3,
}

impl EdgeKey {
    fn new(p1: IVec3, p2: IVec3) -> Self {
        if p1.x < p2.x
            || (p1.x == p2.x && p1.y < p2.y)
            || (p1.x == p2.x && p1.y == p2.y && p1.z < p2.z)
        {
            Self { min: p1, max: p2 }
        } else {
            Self { min: p2, max: p1 }
        }
    }
}

/// Dual Contouring mesh generator
#[derive(Clone)]
pub struct DualContouring {
    /// Vertex cache for deduplication, keyed by the crossed cell edge.
    vertex_cache: HashMap<EdgeKey, u32>,
    /// Edge intersection cache
    edge_cache: HashMap<EdgeKey, Vec3>,
}

impl DualContouring {
    /// Create a new Dual Contouring generator
    pub fn new() -> Self {
        Self {
            vertex_cache: HashMap::new(),
            edge_cache: HashMap::new(),
        }
    }

    /// Generate mesh from voxel chunk
    pub fn generate_mesh(&mut self, chunk: &VoxelChunk) -> ChunkMesh {
        self.vertex_cache.clear();
        self.edge_cache.clear();

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Process each cell in the chunk
        for z in 0..CHUNK_SIZE - 1 {
            for y in 0..CHUNK_SIZE - 1 {
                for x in 0..CHUNK_SIZE - 1 {
                    let cell_pos = IVec3::new(x, y, z);
                    self.process_cell(chunk, cell_pos, &mut vertices, &mut indices);
                }
            }
        }

        let chunk_origin = chunk.coord().to_world_pos();
        let mut mesh = ChunkMesh {
            coord: chunk.coord(),
            vertices,
            indices,
        };

        // Add skirt geometry to hide LOD seam cracks between chunks
        mesh.add_skirts(2.0, chunk_origin);
        mesh
    }

    /// Process a single cell (8 voxels forming a cube)
    fn process_cell(
        &mut self,
        chunk: &VoxelChunk,
        cell_pos: IVec3,
        vertices: &mut Vec<MeshVertex>,
        indices: &mut Vec<u32>,
    ) {
        // Get the 8 corner voxels
        let corners = self.get_cell_corners(chunk, cell_pos);

        // Calculate cell configuration (which corners are solid)
        let mut config = 0u8;
        for (i, corner) in corners.iter().enumerate() {
            if corner.is_some_and(|v| v.is_solid()) {
                config |= 1 << i;
            }
        }

        // Skip if all corners are the same (no surface)
        if config == 0 || config == 255 {
            return;
        }

        let material = corners
            .iter()
            .flatten()
            .find(|v| v.is_solid())
            .map(|v| v.material)
            .unwrap_or(0);

        self.generate_cell_triangles(chunk, cell_pos, config, material, vertices, indices);
    }

    /// Bourke marching-cubes corner order. Must match `EDGE_ENDPOINTS` and
    /// `MC_EDGE_TABLE` / `MC_TRI_TABLE`.
    fn get_cell_corners(&self, chunk: &VoxelChunk, cell_pos: IVec3) -> [Option<Voxel>; 8] {
        [
            chunk.get_voxel(cell_pos + IVec3::new(0, 0, 0)),
            chunk.get_voxel(cell_pos + IVec3::new(1, 0, 0)),
            chunk.get_voxel(cell_pos + IVec3::new(1, 0, 1)),
            chunk.get_voxel(cell_pos + IVec3::new(0, 0, 1)),
            chunk.get_voxel(cell_pos + IVec3::new(0, 1, 0)),
            chunk.get_voxel(cell_pos + IVec3::new(1, 1, 0)),
            chunk.get_voxel(cell_pos + IVec3::new(1, 1, 1)),
            chunk.get_voxel(cell_pos + IVec3::new(0, 1, 1)),
        ]
    }

    /// Compute vertex normal using central differences
    fn compute_vertex_normal(&self, chunk: &VoxelChunk, cell_pos: IVec3) -> Vec3 {
        let dx = self.sample_density(chunk, cell_pos + IVec3::new(1, 0, 0))
            - self.sample_density(chunk, cell_pos - IVec3::new(1, 0, 0));
        let dy = self.sample_density(chunk, cell_pos + IVec3::new(0, 1, 0))
            - self.sample_density(chunk, cell_pos - IVec3::new(0, 1, 0));
        let dz = self.sample_density(chunk, cell_pos + IVec3::new(0, 0, 1))
            - self.sample_density(chunk, cell_pos - IVec3::new(0, 0, 1));

        let gradient = Vec3::new(dx, dy, dz);
        if gradient.length_squared() > 0.001 {
            gradient.normalize()
        } else {
            Vec3::Y // Default up
        }
    }

    /// Sample density at position
    fn sample_density(&self, chunk: &VoxelChunk, pos: IVec3) -> f32 {
        chunk.get_voxel(pos).map(|v| v.density).unwrap_or(0.0)
    }

    /// Bourke corner offsets. Index matches [`Self::get_cell_corners`].
    const BOURKE_CORNERS: [IVec3; 8] = [
        IVec3::new(0, 0, 0),
        IVec3::new(1, 0, 0),
        IVec3::new(1, 0, 1),
        IVec3::new(0, 0, 1),
        IVec3::new(0, 1, 0),
        IVec3::new(1, 1, 0),
        IVec3::new(1, 1, 1),
        IVec3::new(0, 1, 1),
    ];

    fn edge_vertex(
        &mut self,
        chunk: &VoxelChunk,
        cell_pos: IVec3,
        edge_idx: usize,
        material: u16,
        vertices: &mut Vec<MeshVertex>,
    ) -> u32 {
        let (c0, c1) = EDGE_ENDPOINTS[edge_idx];
        let p0 = cell_pos + Self::BOURKE_CORNERS[c0];
        let p1 = cell_pos + Self::BOURKE_CORNERS[c1];
        let key = EdgeKey::new(p0, p1);
        if let Some(&index) = self.vertex_cache.get(&key) {
            return index;
        }
        let d0 = self.sample_density(chunk, p0);
        let d1 = self.sample_density(chunk, p1);
        let t = if (d1 - d0).abs() > 0.001 {
            ((0.5 - d0) / (d1 - d0)).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let local = p0.as_vec3() + (p1.as_vec3() - p0.as_vec3()) * t;
        let world = chunk.coord().to_world_pos() + local;
        let index = vertices.len() as u32;
        vertices.push(MeshVertex {
            position: world,
            normal: self.compute_vertex_normal(chunk, cell_pos),
            material,
        });
        self.vertex_cache.insert(key, index);
        index
    }

    /// Generate triangles for a cell based on configuration.
    /// Uses the marching-cubes lookup tables. One vertex is stored per crossed edge.
    fn generate_cell_triangles(
        &mut self,
        chunk: &VoxelChunk,
        cell_pos: IVec3,
        config: u8,
        material: u16,
        vertices: &mut Vec<MeshVertex>,
        indices: &mut Vec<u32>,
    ) {
        let edge_flags = MC_EDGE_TABLE[config as usize];
        if edge_flags == 0 {
            return;
        }

        let mut edge_vertices = [None; 12];
        for (edge_idx, slot) in edge_vertices.iter_mut().enumerate() {
            if (edge_flags & (1 << edge_idx)) != 0 {
                *slot = Some(self.edge_vertex(chunk, cell_pos, edge_idx, material, vertices));
            }
        }

        // Generate triangles using the triangle table
        let tri_config = MC_TRI_TABLE[config as usize];
        let mut i = 0;

        while i < 15 && tri_config[i] != -1 {
            // Each triangle uses 3 edge indices
            let e0 = tri_config[i] as usize;
            let e1 = tri_config[i + 1] as usize;
            let e2 = tri_config[i + 2] as usize;

            // Get vertex indices from edge vertices
            if let (Some(v0), Some(v1), Some(v2)) =
                (edge_vertices[e0], edge_vertices[e1], edge_vertices[e2])
            {
                // Add triangle (counter-clockwise winding)
                indices.push(v0);
                indices.push(v1);
                indices.push(v2);
            }

            i += 3;
        }
    }
}

impl Default for DualContouring {
    fn default() -> Self {
        Self::new()
    }
}

/// Async mesh generator for background processing
pub struct AsyncMeshGenerator {
    generator: DualContouring,
}

impl AsyncMeshGenerator {
    /// Create a new async mesh generator
    pub fn new() -> Self {
        Self {
            generator: DualContouring::new(),
        }
    }

    /// Generate mesh asynchronously
    pub async fn generate_mesh_async(&mut self, chunk: VoxelChunk) -> ChunkMesh {
        // In a real implementation, this would use tokio::spawn
        // For now, we just call the sync version
        self.generator.generate_mesh(&chunk)
    }

    /// Generate multiple meshes in parallel
    pub async fn generate_meshes_parallel(&mut self, chunks: Vec<VoxelChunk>) -> Vec<ChunkMesh> {
        // Use rayon for parallel processing
        use rayon::prelude::*;

        chunks
            .into_par_iter()
            .map(|chunk| {
                let mut gen = DualContouring::new();
                gen.generate_mesh(&chunk)
            })
            .collect()
    }
}

impl Default for AsyncMeshGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// LOD (Level of Detail) configuration
#[derive(Debug, Clone, Copy)]
pub struct LodConfig {
    /// Distance thresholds for each LOD level (fallback when no view params)
    pub distances: [f32; 4],
    /// Simplification factors for each LOD level
    pub simplification: [f32; 4],
    /// World-space geometric error introduced at each LOD level.
    /// Used with screen-space error metric when view params are available.
    pub geometric_errors: [f32; 4],
    /// Maximum acceptable screen-space pixel error. LOD increases when the
    /// projected pixel error of the *next* coarser level exceeds this budget.
    pub pixel_error_budget: f32,
}

impl Default for LodConfig {
    fn default() -> Self {
        Self {
            distances: [100.0, 250.0, 500.0, 1000.0],
            simplification: [1.0, 0.5, 0.25, 0.125],
            geometric_errors: [0.0, 0.5, 1.5, 4.0],
            pixel_error_budget: 2.0,
        }
    }
}

/// Screen-space inputs for [`LodMeshGenerator`]. Distance thresholds are
/// used when the caller does not have a view.
#[derive(Debug, Clone, Copy)]
pub struct ViewParams {
    /// Vertical field of view in radians.
    pub fov_y: f32,
    /// Viewport height in pixels.
    pub screen_height: f32,
}

impl ViewParams {
    pub fn new(fov_y: f32, screen_height: f32) -> Self {
        Self {
            fov_y,
            screen_height,
        }
    }
}

fn compute_pixel_error(geometric_error: f32, distance: f32, view: &ViewParams) -> f32 {
    let denom = distance * 2.0 * (view.fov_y * 0.5).tan();
    if denom < 1e-6 {
        return f32::MAX;
    }
    (geometric_error * view.screen_height) / denom
}

/// LOD mesh generator
pub struct LodMeshGenerator {
    config: LodConfig,
    generators: Vec<DualContouring>,
}

impl LodMeshGenerator {
    /// Create a new LOD mesh generator
    pub fn new(config: LodConfig) -> Self {
        Self {
            config,
            generators: vec![DualContouring::new(); 4],
        }
    }

    /// Generate mesh with appropriate LOD based on distance and optional view params.
    ///
    /// When `view` is `Some`, uses screen-space pixel error to select LOD level.
    /// When `view` is `None`, falls back to fixed distance thresholds.
    pub fn generate_mesh_lod(
        &mut self,
        chunk: &VoxelChunk,
        distance: f32,
        view: Option<&ViewParams>,
    ) -> ChunkMesh {
        let lod_level = self.select_lod_level(distance, view);
        self.generators[lod_level].generate_mesh(chunk)
    }

    /// Select LOD level based on screen-space error (preferred) or distance (fallback).
    fn select_lod_level(&self, distance: f32, view: Option<&ViewParams>) -> usize {
        if let Some(vp) = view {
            // Screen-space error: walk from coarsest LOD toward finest.
            // Pick the coarsest level whose pixel error is within budget.
            for level in (0..4).rev() {
                let geo_err = self.config.geometric_errors[level];
                let px = compute_pixel_error(geo_err, distance, vp);
                if px <= self.config.pixel_error_budget {
                    return level;
                }
            }
            0 // All levels exceed budget — use finest
        } else {
            for (i, &threshold) in self.config.distances.iter().enumerate() {
                if distance < threshold {
                    return i;
                }
            }
            3 // Furthest LOD
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel_data::Voxel;

    #[test]
    fn test_dual_contouring_empty_chunk() {
        let coord = ChunkCoord::new(0, 0, 0);
        let chunk = VoxelChunk::new(coord);

        let mut dc = DualContouring::new();
        let mesh = dc.generate_mesh(&chunk);

        assert!(mesh.is_empty());
    }

    #[test]
    fn test_dual_contouring_single_voxel() {
        let coord = ChunkCoord::new(0, 0, 0);
        let mut chunk = VoxelChunk::new(coord);

        // Set a single solid voxel
        chunk.set_voxel(IVec3::new(5, 5, 5), Voxel::new(1.0, 1));

        let mut dc = DualContouring::new();
        let mesh = dc.generate_mesh(&chunk);

        // Should generate some vertices
        assert!(!mesh.vertices.is_empty());
    }

    #[test]
    fn test_mesh_vertex_creation() {
        let vertex = MeshVertex {
            position: Vec3::new(1.0, 2.0, 3.0),
            normal: Vec3::Y,
            material: 5,
        };

        assert_eq!(vertex.position.x, 1.0);
        assert_eq!(vertex.normal, Vec3::Y);
        assert_eq!(vertex.material, 5);
    }

    #[test]
    fn test_lod_selection() {
        let config = LodConfig::default();
        let lod_gen = LodMeshGenerator::new(config);

        assert_eq!(lod_gen.select_lod_level(50.0, None), 0);
        assert_eq!(lod_gen.select_lod_level(200.0, None), 1);
        assert_eq!(lod_gen.select_lod_level(400.0, None), 2);
        assert_eq!(lod_gen.select_lod_level(1500.0, None), 3);
    }

    #[test]
    fn edge_midpoint_is_stored_and_two_runs_match_bits() {
        let coord = ChunkCoord::new(0, 0, 0);
        let mut chunk = VoxelChunk::new(coord);
        // Corner (3,0,0) is not the origin of cell (2,0,0). The old cache
        // keyed vertices by cell position and looked them up with
        // `edge_key.min`, so this edge's midpoint never became a vertex.
        chunk.set_voxel(IVec3::new(3, 0, 0), Voxel::new(1.0, 1));

        let mut first = DualContouring::new();
        let mut second = DualContouring::new();
        let a = first.generate_mesh(&chunk);
        let b = second.generate_mesh(&chunk);
        assert!(!a.indices.is_empty());
        assert_eq!(a.indices, b.indices);
        assert_eq!(a.vertices.len(), b.vertices.len());
        for (left, right) in a.vertices.iter().zip(b.vertices.iter()) {
            assert_eq!(left.position.x.to_bits(), right.position.x.to_bits());
            assert_eq!(left.position.y.to_bits(), right.position.y.to_bits());
            assert_eq!(left.position.z.to_bits(), right.position.z.to_bits());
            assert_eq!(left.material, right.material);
        }
        let midpoint = a.vertices.iter().any(|vertex| {
            vertex.position.x.to_bits() == 3.0f32.to_bits()
                && vertex.position.y.to_bits() == 0.0f32.to_bits()
                && vertex.position.z.to_bits() == 0.5f32.to_bits()
        });
        assert!(
            midpoint,
            "expected the crossed edge midpoint (3, 0, 0.5), got {:?}",
            a.vertices
                .iter()
                .map(|vertex| vertex.position)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_edge_key_ordering() {
        let p1 = IVec3::new(0, 0, 0);
        let p2 = IVec3::new(1, 0, 0);

        let key1 = EdgeKey::new(p1, p2);
        let key2 = EdgeKey::new(p2, p1);

        assert_eq!(key1, key2);
    }
}
