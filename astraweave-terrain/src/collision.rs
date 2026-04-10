//! Terrain collision mesh generation
//!
//! Converts terrain data (voxel meshes and heightmaps) into physics-compatible
//! collision geometry. Output format matches `PhysicsWorld::add_static_trimesh()`:
//! - `Vec<Vec3>` for vertex positions
//! - `Vec<[u32; 3]>` for triangle indices

use crate::chunk::TerrainChunk;
use crate::meshing::ChunkMesh;
use glam::Vec3;

/// Collision mesh data ready for physics system consumption.
///
/// Feed `vertices` and `triangles` directly into
/// `PhysicsWorld::add_static_trimesh(&vertices, &triangles, layers)`.
#[derive(Debug, Clone)]
pub struct CollisionMesh {
    /// Vertex positions in world space
    pub vertices: Vec<Vec3>,
    /// Triangle index triplets
    pub triangles: Vec<[u32; 3]>,
}

impl CollisionMesh {
    /// Check if the collision mesh has any geometry
    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    /// Number of triangles
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }
}

/// Extract collision mesh data from a voxel `ChunkMesh`.
///
/// Strips normals and material data, keeping only positions and triangle indices.
/// Skirt geometry (if present) is included — it extends slightly below the surface
/// which provides a small margin against penetration.
pub fn collision_mesh_from_chunk(mesh: &ChunkMesh) -> CollisionMesh {
    let vertices: Vec<Vec3> = mesh.vertices.iter().map(|v| v.position).collect();

    let num_triangles = mesh.indices.len() / 3;
    let triangles: Vec<[u32; 3]> = (0..num_triangles)
        .map(|i| {
            [
                mesh.indices[i * 3],
                mesh.indices[i * 3 + 1],
                mesh.indices[i * 3 + 2],
            ]
        })
        .collect();

    CollisionMesh {
        vertices,
        triangles,
    }
}

/// Generate a collision mesh from a heightmap-based `TerrainChunk`.
///
/// Creates a regular grid of triangles from the heightmap data.
/// Each heightmap cell produces 2 triangles (a quad split diagonally).
///
/// # Arguments
/// * `chunk` — The terrain chunk to generate collision for
/// * `chunk_size` — World-space size of the chunk (width and depth)
/// * `resolution_step` — Sample every Nth heightmap point (1 = full detail, 2 = half, etc.)
pub fn collision_mesh_from_heightmap(
    chunk: &TerrainChunk,
    chunk_size: f32,
    resolution_step: u32,
) -> CollisionMesh {
    let heightmap = chunk.heightmap();
    let resolution = heightmap.resolution();
    let data = heightmap.data();
    let step = resolution_step.max(1);

    let chunk_origin = chunk.id().to_world_pos(chunk_size);

    // Number of vertices along each axis after stepping
    let n = ((resolution - 1) / step) + 1;

    let mut vertices = Vec::with_capacity((n * n) as usize);
    let mut triangles = Vec::new();

    // Generate vertex grid
    for iz in 0..n {
        for ix in 0..n {
            let hx = (ix * step).min(resolution - 1);
            let hz = (iz * step).min(resolution - 1);
            let height = data[(hz * resolution + hx) as usize];

            let world_x = chunk_origin.x + (hx as f32 / (resolution - 1) as f32) * chunk_size;
            let world_z = chunk_origin.z + (hz as f32 / (resolution - 1) as f32) * chunk_size;

            vertices.push(Vec3::new(world_x, height, world_z));
        }
    }

    // Generate triangle indices (2 triangles per quad)
    for iz in 0..(n - 1) {
        for ix in 0..(n - 1) {
            let v00 = iz * n + ix;
            let v10 = iz * n + ix + 1;
            let v01 = (iz + 1) * n + ix;
            let v11 = (iz + 1) * n + ix + 1;

            // Triangle 1: v00, v10, v01
            triangles.push([v00, v10, v01]);
            // Triangle 2: v10, v11, v01
            triangles.push([v10, v11, v01]);
        }
    }

    CollisionMesh {
        vertices,
        triangles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heightmap::{Heightmap, HeightmapConfig};
    use crate::meshing::{ChunkMesh, MeshVertex};
    use crate::voxel_data::ChunkCoord;
    use crate::{BiomeType, ChunkId};

    #[test]
    fn test_collision_mesh_from_empty_chunk() {
        let mesh = ChunkMesh::empty(ChunkCoord::new(0, 0, 0));
        let collision = collision_mesh_from_chunk(&mesh);
        assert!(collision.is_empty());
        assert_eq!(collision.triangle_count(), 0);
    }

    #[test]
    fn test_collision_mesh_from_chunk_preserves_indices() {
        let mesh = ChunkMesh {
            coord: ChunkCoord::new(0, 0, 0),
            vertices: vec![
                MeshVertex {
                    position: Vec3::new(0.0, 0.0, 0.0),
                    normal: Vec3::Y,
                    material: 1,
                },
                MeshVertex {
                    position: Vec3::new(1.0, 0.0, 0.0),
                    normal: Vec3::Y,
                    material: 1,
                },
                MeshVertex {
                    position: Vec3::new(0.0, 0.0, 1.0),
                    normal: Vec3::Y,
                    material: 1,
                },
            ],
            indices: vec![0, 1, 2],
        };

        let collision = collision_mesh_from_chunk(&mesh);
        assert_eq!(collision.vertices.len(), 3);
        assert_eq!(collision.triangle_count(), 1);
        assert_eq!(collision.triangles[0], [0, 1, 2]);
    }

    #[test]
    fn test_collision_mesh_from_heightmap_basic() {
        // Create a flat 4x4 heightmap
        let data = vec![5.0_f32; 16];
        let heightmap = Heightmap::from_data(data, 4).unwrap();
        let biomes = vec![BiomeType::Grassland; 16];
        let chunk = TerrainChunk::new(ChunkId::new(0, 0), heightmap, biomes);

        let collision = collision_mesh_from_heightmap(&chunk, 100.0, 1);

        // 4x4 grid = 16 vertices, 3x3 quads = 18 triangles
        assert_eq!(collision.vertices.len(), 16);
        assert_eq!(collision.triangle_count(), 18);

        // All heights should be 5.0
        for v in &collision.vertices {
            assert!((v.y - 5.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_collision_mesh_from_heightmap_stepped() {
        // Create an 8x8 heightmap but sample every 2nd point
        let data = vec![10.0_f32; 64];
        let heightmap = Heightmap::from_data(data, 8).unwrap();
        let biomes = vec![BiomeType::Grassland; 64];
        let chunk = TerrainChunk::new(ChunkId::new(0, 0), heightmap, biomes);

        let collision = collision_mesh_from_heightmap(&chunk, 100.0, 2);

        // step=2 on 8-wide → indices 0,2,4,6 → 4 vertices per axis → 16 vertices
        assert_eq!(collision.vertices.len(), 16);
        // 3x3 quads = 18 triangles
        assert_eq!(collision.triangle_count(), 18);
    }

    #[test]
    fn test_collision_mesh_world_positions() {
        let data = vec![0.0_f32; 4]; // 2x2 heightmap
        let heightmap = Heightmap::from_data(data, 2).unwrap();
        let biomes = vec![BiomeType::Grassland; 4];
        let chunk = TerrainChunk::new(ChunkId::new(1, 2), heightmap, biomes);

        let collision = collision_mesh_from_heightmap(&chunk, 64.0, 1);

        // Chunk (1,2) at size 64 → origin (64, 0, 128)
        assert_eq!(collision.vertices.len(), 4);
        assert!((collision.vertices[0].x - 64.0).abs() < f32::EPSILON);
        assert!((collision.vertices[0].z - 128.0).abs() < f32::EPSILON);
    }
}
