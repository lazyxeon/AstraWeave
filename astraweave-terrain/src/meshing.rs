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
use std::collections::HashMap;

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
    /// Vertex cache for deduplication
    vertex_cache: HashMap<IVec3, u32>,
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

        ChunkMesh {
            coord: chunk.coord(),
            vertices,
            indices,
        }
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

        // Find surface crossing edges and compute vertex position
        let vertex_pos = self.compute_vertex_position(chunk, cell_pos, &corners);
        let vertex_normal = self.compute_vertex_normal(chunk, cell_pos);
        let material = corners
            .iter()
            .flatten()
            .find(|v| v.is_solid())
            .map(|v| v.material)
            .unwrap_or(0);

        // Add vertex to cache
        let vertex_index = vertices.len() as u32;
        self.vertex_cache.insert(cell_pos, vertex_index);

        let world_pos = chunk.coord().to_world_pos() + vertex_pos;
        vertices.push(MeshVertex {
            position: world_pos,
            normal: vertex_normal,
            material,
        });

        // Generate triangles for this cell
        self.generate_cell_triangles(cell_pos, config, indices);
    }

    /// Get the 8 corner voxels of a cell
    fn get_cell_corners(&self, chunk: &VoxelChunk, cell_pos: IVec3) -> [Option<Voxel>; 8] {
        [
            chunk.get_voxel(cell_pos + IVec3::new(0, 0, 0)),
            chunk.get_voxel(cell_pos + IVec3::new(1, 0, 0)),
            chunk.get_voxel(cell_pos + IVec3::new(1, 1, 0)),
            chunk.get_voxel(cell_pos + IVec3::new(0, 1, 0)),
            chunk.get_voxel(cell_pos + IVec3::new(0, 0, 1)),
            chunk.get_voxel(cell_pos + IVec3::new(1, 0, 1)),
            chunk.get_voxel(cell_pos + IVec3::new(1, 1, 1)),
            chunk.get_voxel(cell_pos + IVec3::new(0, 1, 1)),
        ]
    }

    /// Compute vertex position using QEF (Quadratic Error Function) minimization
    /// Simplified version: use average of edge intersections
    fn compute_vertex_position(
        &mut self,
        _chunk: &VoxelChunk,
        cell_pos: IVec3,
        corners: &[Option<Voxel>; 8],
    ) -> Vec3 {
        let mut sum = Vec3::ZERO;
        let mut count = 0;

        // Check all 12 edges of the cube
        let edges = [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0), // Bottom face
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4), // Top face
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7), // Vertical edges
        ];

        for (i, j) in edges.iter() {
            if let (Some(v1), Some(v2)) = (corners[*i], corners[*j]) {
                // Check if edge crosses surface (one solid, one empty)
                if v1.is_solid() != v2.is_solid() {
                    let p1 = self.corner_offset(*i);
                    let p2 = self.corner_offset(*j);

                    // Linear interpolation based on density
                    let t = if (v1.density - v2.density).abs() > 0.001 {
                        (0.5 - v1.density) / (v2.density - v1.density)
                    } else {
                        0.5
                    };
                    let t = t.clamp(0.0, 1.0);

                    let intersection = p1 + (p2 - p1) * t;
                    sum += intersection;
                    count += 1;
                }
            }
        }

        if count > 0 {
            cell_pos.as_vec3() + sum / count as f32
        } else {
            // Fallback to cell center
            cell_pos.as_vec3() + Vec3::splat(0.5)
        }
    }

    /// Get offset for corner index (0-7)
    fn corner_offset(&self, index: usize) -> Vec3 {
        let x = if index & 1 != 0 { 1.0 } else { 0.0 };
        let y = if index & 2 != 0 { 1.0 } else { 0.0 };
        let z = if index & 4 != 0 { 1.0 } else { 0.0 };
        Vec3::new(x, y, z)
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

    /// Generate triangles for a cell based on configuration
    /// Uses proper Marching Cubes lookup tables for watertight meshes
    fn generate_cell_triangles(&self, cell_pos: IVec3, config: u8, indices: &mut Vec<u32>) {
        // Get the edge table value for this configuration
        let edge_flags = MC_EDGE_TABLE[config as usize];

        // If no edges have vertices, skip this cell
        if edge_flags == 0 {
            return;
        }

        // Build list of edge vertices (up to 12 edges)
        let mut edge_vertices = [None; 12];
        for edge_idx in 0..12 {
            if (edge_flags & (1 << edge_idx)) != 0 {
                // This edge has a vertex on the isosurface
                let (c0_idx, c1_idx) = EDGE_ENDPOINTS[edge_idx];

                // Compute corner positions
                let corner_offsets = [
                    IVec3::new(0, 0, 0), // 0
                    IVec3::new(1, 0, 0), // 1
                    IVec3::new(1, 0, 1), // 2
                    IVec3::new(0, 0, 1), // 3
                    IVec3::new(0, 1, 0), // 4
                    IVec3::new(1, 1, 0), // 5
                    IVec3::new(1, 1, 1), // 6
                    IVec3::new(0, 1, 1), // 7
                ];

                let p0 = cell_pos + corner_offsets[c0_idx];
                let p1 = cell_pos + corner_offsets[c1_idx];
                let edge_key = EdgeKey::new(p0, p1);

                // Look up the vertex index (should already exist from process_cell)
                edge_vertices[edge_idx] = self.vertex_cache.get(&edge_key.min).copied();
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
        view: Option<&crate::lod_manager::ViewParams>,
    ) -> ChunkMesh {
        let lod_level = self.select_lod_level(distance, view);
        self.generators[lod_level].generate_mesh(chunk)
    }

    /// Select LOD level based on screen-space error (preferred) or distance (fallback).
    fn select_lod_level(
        &self,
        distance: f32,
        view: Option<&crate::lod_manager::ViewParams>,
    ) -> usize {
        if let Some(vp) = view {
            // Screen-space error: walk from coarsest LOD toward finest.
            // Pick the coarsest level whose pixel error is within budget.
            for level in (0..4).rev() {
                let geo_err = self.config.geometric_errors[level];
                let px = crate::lod_manager::compute_pixel_error(geo_err, distance, vp);
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
    fn test_edge_key_ordering() {
        let p1 = IVec3::new(0, 0, 0);
        let p2 = IVec3::new(1, 0, 0);

        let key1 = EdgeKey::new(p1, p2);
        let key2 = EdgeKey::new(p2, p1);

        assert_eq!(key1, key2);
    }
}
