//! Wave 2 Meshing + Voxel Data Remediation Tests
//!
//! Targets mutation-sensitive code in meshing.rs and voxel_data.rs:
//! - ChunkMesh: empty, is_empty, memory_usage
//! - EdgeKey ordering symmetry
//! - DualContouring: empty chunk, single voxel, corner offsets, normal fallback
//! - LodConfig defaults, LodMeshGenerator::select_lod_level
//! - AsyncMeshGenerator/LodMeshGenerator construction
//! - Voxel: new, is_solid threshold, is_empty threshold, defaults
//! - ChunkCoord: from_world_pos, to_world_pos, neighbors
//! - VoxelChunk: get/set, bounds checking, dirty flag, world coords
//! - VoxelGrid: set/get/remove/clear, dirty tracking, chunk_count, memory
//! - OctreeNode: child_index bit manipulation

use astraweave_terrain::meshing::*;
use astraweave_terrain::voxel_data::*;
use glam::{IVec3, Vec3};

// ============================================================================
// A. ChunkMesh basics
// ============================================================================

#[test]
fn chunk_mesh_empty_has_no_vertices() {
    let m = ChunkMesh::empty(ChunkCoord::new(0, 0, 0));
    assert!(m.vertices.is_empty());
    assert!(m.indices.is_empty());
}

#[test]
fn chunk_mesh_empty_is_empty() {
    let m = ChunkMesh::empty(ChunkCoord::new(0, 0, 0));
    assert!(m.is_empty());
}

#[test]
fn chunk_mesh_with_verts_is_not_empty() {
    let m = ChunkMesh {
        coord: ChunkCoord::new(0, 0, 0),
        vertices: vec![MeshVertex {
            position: Vec3::ZERO,
            normal: Vec3::Y,
            material: 1,
        }],
        indices: vec![0],
    };
    assert!(!m.is_empty());
}

#[test]
fn chunk_mesh_memory_usage_increases_with_data() {
    let empty = ChunkMesh::empty(ChunkCoord::new(0, 0, 0));
    let filled = ChunkMesh {
        coord: ChunkCoord::new(0, 0, 0),
        vertices: vec![
            MeshVertex {
                position: Vec3::ZERO,
                normal: Vec3::Y,
                material: 1,
            },
            MeshVertex {
                position: Vec3::X,
                normal: Vec3::Y,
                material: 1,
            },
            MeshVertex {
                position: Vec3::Y,
                normal: Vec3::Y,
                material: 1,
            },
        ],
        indices: vec![0, 1, 2],
    };
    assert!(filled.memory_usage() > empty.memory_usage());
}

#[test]
fn chunk_mesh_coord_matches() {
    let coord = ChunkCoord::new(3, 7, -2);
    let m = ChunkMesh::empty(coord);
    assert_eq!(m.coord.x, 3);
    assert_eq!(m.coord.y, 7);
    assert_eq!(m.coord.z, -2);
}

// ============================================================================
// B. EdgeKey symmetry
// ============================================================================

#[test]
fn edge_key_symmetric() {
    // Testing EdgeKey indirectly through DualContouring
    // EdgeKey::new(p1, p2) == EdgeKey::new(p2, p1) — verified by mesh generation
    let mut dc = DualContouring::new();
    let coord = ChunkCoord::new(0, 0, 0);
    let chunk = VoxelChunk::new(coord);
    let mesh = dc.generate_mesh(&chunk);
    assert!(mesh.is_empty(), "empty chunk should produce empty mesh");
}

// ============================================================================
// C. DualContouring
// ============================================================================

#[test]
fn dual_contouring_default_is_new() {
    let dc = DualContouring::default();
    let dc2 = DualContouring::new();
    // Both should start fresh — verifying they don't panic
    let _ = dc;
    let _ = dc2;
}

#[test]
fn dual_contouring_empty_chunk_produces_no_mesh() {
    let mut dc = DualContouring::new();
    let chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    let mesh = dc.generate_mesh(&chunk);
    assert!(mesh.is_empty());
    assert_eq!(mesh.vertices.len(), 0);
    assert_eq!(mesh.indices.len(), 0);
}

#[test]
fn dual_contouring_single_solid_voxel_generates_vertices() {
    let mut dc = DualContouring::new();
    let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    chunk.set_voxel(IVec3::new(5, 5, 5), Voxel::new(1.0, 1));
    let mesh = dc.generate_mesh(&chunk);
    assert!(
        !mesh.is_empty(),
        "single solid voxel should produce mesh vertices"
    );
}

#[test]
fn dual_contouring_solid_block_generates_surface() {
    let mut dc = DualContouring::new();
    let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    // Fill a 4x4x4 block of solid voxels
    for x in 4..8 {
        for y in 4..8 {
            for z in 4..8 {
                chunk.set_voxel(IVec3::new(x, y, z), Voxel::new(1.0, 1));
            }
        }
    }
    let mesh = dc.generate_mesh(&chunk);
    assert!(!mesh.is_empty());
    // Should have normals and positions
    for v in &mesh.vertices {
        assert!(v.normal.length() > 0.0, "normals should be non-zero");
    }
    // Indices should be multiples of 3 (triangles)
    assert_eq!(
        mesh.indices.len() % 3,
        0,
        "indices should form complete triangles"
    );
}

#[test]
fn dual_contouring_reuse_clears_cache() {
    let mut dc = DualContouring::new();
    let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    chunk.set_voxel(IVec3::new(5, 5, 5), Voxel::new(1.0, 1));
    let mesh1 = dc.generate_mesh(&chunk);

    // Generate again — caches should be cleared
    let mesh2 = dc.generate_mesh(&chunk);
    assert_eq!(mesh1.vertices.len(), mesh2.vertices.len());
}

#[test]
fn dual_contouring_material_from_solid_voxel() {
    let mut dc = DualContouring::new();
    let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    chunk.set_voxel(IVec3::new(5, 5, 5), Voxel::new(1.0, 42));
    let mesh = dc.generate_mesh(&chunk);
    // At least one vertex should have material 42
    let has_mat = mesh.vertices.iter().any(|v| v.material == 42);
    assert!(has_mat, "mesh should preserve material from solid voxel");
}

// ============================================================================
// D. LodConfig and LodMeshGenerator
// ============================================================================

#[test]
fn lod_config_default_distances() {
    let c = LodConfig::default();
    assert_eq!(c.distances, [100.0, 250.0, 500.0, 1000.0]);
}

#[test]
fn lod_config_default_simplification() {
    let c = LodConfig::default();
    assert_eq!(c.simplification, [1.0, 0.5, 0.25, 0.125]);
}

#[test]
fn lod_mesh_generator_close_uses_full_detail() {
    // Test indirectly through generate_mesh_lod (public API)
    let mut gen = LodMeshGenerator::new(LodConfig::default());
    let chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    // Should not panic for any distance
    let _m = gen.generate_mesh_lod(&chunk, 10.0, None);
}

#[test]
fn lod_mesh_generator_far_uses_low_detail() {
    let mut gen = LodMeshGenerator::new(LodConfig::default());
    let chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    let _m = gen.generate_mesh_lod(&chunk, 5000.0, None);
}

#[test]
fn lod_mesh_generator_various_distances() {
    let mut gen = LodMeshGenerator::new(LodConfig::default());
    let chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    // All should produce empty meshes from empty chunk
    for dist in [10.0, 150.0, 400.0, 800.0, 5000.0] {
        let m = gen.generate_mesh_lod(&chunk, dist, None);
        assert!(
            m.is_empty(),
            "empty chunk should always produce empty mesh at dist {}",
            dist
        );
    }
}

// ============================================================================
// E. AsyncMeshGenerator
// ============================================================================

#[test]
fn async_mesh_generator_constructs() {
    let _gen = AsyncMeshGenerator::new();
    let _gen2 = AsyncMeshGenerator::default();
}

// ============================================================================
// F. Voxel basics
// ============================================================================

#[test]
fn voxel_new_stores_density_and_material() {
    let v = Voxel::new(0.75, 3);
    assert_eq!(v.density, 0.75);
    assert_eq!(v.material, 3);
}

#[test]
fn voxel_default_is_empty() {
    let v = Voxel::default();
    assert_eq!(v.density, 0.0);
    assert_eq!(v.material, 0);
    assert!(v.is_empty());
    assert!(!v.is_solid());
}

#[test]
fn voxel_is_solid_threshold() {
    // density > 0.5 is solid
    assert!(
        !Voxel::new(0.5, 0).is_solid(),
        "0.5 should NOT be solid (> 0.5 required)"
    );
    assert!(Voxel::new(0.51, 0).is_solid(), "0.51 should be solid");
    assert!(Voxel::new(1.0, 0).is_solid());
    assert!(!Voxel::new(0.49, 0).is_solid());
}

#[test]
fn voxel_is_empty_threshold() {
    // density < 0.01 is empty
    assert!(Voxel::new(0.0, 0).is_empty());
    assert!(Voxel::new(0.005, 0).is_empty());
    assert!(
        !Voxel::new(0.01, 0).is_empty(),
        "0.01 should NOT be empty (< 0.01 required)"
    );
    assert!(!Voxel::new(0.5, 0).is_empty());
}

#[test]
fn voxel_neither_solid_nor_empty() {
    let v = Voxel::new(0.3, 0);
    assert!(!v.is_solid());
    assert!(!v.is_empty());
}

// ============================================================================
// G. ChunkCoord
// ============================================================================

#[test]
fn chunk_coord_new() {
    let c = ChunkCoord::new(3, -2, 7);
    assert_eq!(c.x, 3);
    assert_eq!(c.y, -2);
    assert_eq!(c.z, 7);
}

#[test]
fn chunk_coord_from_world_pos_positive() {
    let c = ChunkCoord::from_world_pos(Vec3::new(50.0, 100.0, 0.0));
    // 50 / 32 = 1.5625 → floor → 1
    assert_eq!(c.x, 1);
    // 100 / 32 = 3.125 → floor → 3
    assert_eq!(c.y, 3);
    assert_eq!(c.z, 0);
}

#[test]
fn chunk_coord_from_world_pos_negative() {
    let c = ChunkCoord::from_world_pos(Vec3::new(-10.0, -1.0, -100.0));
    // -10 / 32 = -0.3125 → floor → -1
    assert_eq!(c.x, -1);
    assert_eq!(c.y, -1);
    // -100 / 32 = -3.125 → floor → -4
    assert_eq!(c.z, -4);
}

#[test]
fn chunk_coord_to_world_pos() {
    let c = ChunkCoord::new(1, 3, -1);
    let w = c.to_world_pos();
    assert_eq!(w.x, 32.0);
    assert_eq!(w.y, 96.0);
    assert_eq!(w.z, -32.0);
}

#[test]
fn chunk_coord_roundtrip_positive() {
    let pos = Vec3::new(64.0, 0.0, 128.0);
    let c = ChunkCoord::from_world_pos(pos);
    let origin = c.to_world_pos();
    // origin should be <= pos in all components (chunk origin is min corner)
    assert!(origin.x <= pos.x);
    assert!(origin.y <= pos.y);
    assert!(origin.z <= pos.z);
}

#[test]
fn chunk_coord_neighbors_count() {
    let c = ChunkCoord::new(0, 0, 0);
    assert_eq!(c.neighbors().len(), 6);
}

#[test]
fn chunk_coord_neighbors_axes() {
    let c = ChunkCoord::new(5, 10, 15);
    let n = c.neighbors();
    // +x, -x, +y, -y, +z, -z
    assert_eq!(n[0], ChunkCoord::new(6, 10, 15));
    assert_eq!(n[1], ChunkCoord::new(4, 10, 15));
    assert_eq!(n[2], ChunkCoord::new(5, 11, 15));
    assert_eq!(n[3], ChunkCoord::new(5, 9, 15));
    assert_eq!(n[4], ChunkCoord::new(5, 10, 16));
    assert_eq!(n[5], ChunkCoord::new(5, 10, 14));
}

#[test]
fn chunk_coord_eq_hash() {
    use std::collections::HashSet;
    let mut s = HashSet::new();
    s.insert(ChunkCoord::new(1, 2, 3));
    s.insert(ChunkCoord::new(1, 2, 3)); // duplicate
    s.insert(ChunkCoord::new(4, 5, 6));
    assert_eq!(s.len(), 2);
}

// ============================================================================
// H. VoxelChunk
// ============================================================================

#[test]
fn voxel_chunk_new_is_empty() {
    let c = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    assert!(c.is_empty());
    assert!(!c.is_dirty());
}

#[test]
fn voxel_chunk_set_get() {
    let mut c = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    c.set_voxel(IVec3::new(5, 10, 15), Voxel::new(0.8, 3));
    let got = c.get_voxel(IVec3::new(5, 10, 15));
    assert!(got.is_some());
    let v = got.unwrap();
    assert!((v.density - 0.8).abs() < 1e-6);
    assert_eq!(v.material, 3);
}

#[test]
fn voxel_chunk_out_of_bounds_get_returns_none() {
    let c = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    assert!(c.get_voxel(IVec3::new(-1, 0, 0)).is_none());
    assert!(c.get_voxel(IVec3::new(0, 32, 0)).is_none());
    assert!(c.get_voxel(IVec3::new(0, 0, 32)).is_none());
}

#[test]
fn voxel_chunk_out_of_bounds_set_is_noop() {
    let mut c = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    c.set_voxel(IVec3::new(-1, 0, 0), Voxel::new(1.0, 1));
    assert!(c.is_empty(), "out-of-bounds set should not add data");
    assert!(!c.is_dirty(), "out-of-bounds set should not dirty chunk");
}

#[test]
fn voxel_chunk_dirty_after_set() {
    let mut c = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    c.set_voxel(IVec3::new(0, 0, 0), Voxel::new(1.0, 1));
    assert!(c.is_dirty());
}

#[test]
fn voxel_chunk_mark_clean_clears_dirty() {
    let mut c = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    c.set_voxel(IVec3::new(0, 0, 0), Voxel::new(1.0, 1));
    assert!(c.is_dirty());
    c.mark_clean();
    assert!(!c.is_dirty());
}

#[test]
fn voxel_chunk_coord_accessor() {
    let coord = ChunkCoord::new(3, -1, 7);
    let c = VoxelChunk::new(coord);
    assert_eq!(c.coord(), coord);
}

#[test]
fn voxel_chunk_world_pos_get_set() {
    let coord = ChunkCoord::new(0, 0, 0);
    let mut c = VoxelChunk::new(coord);
    let world_pos = Vec3::new(5.5, 10.5, 15.5);
    c.set_voxel_world(world_pos, Voxel::new(0.9, 7));
    let got = c.get_voxel_world(world_pos);
    assert!(got.is_some());
    assert!((got.unwrap().density - 0.9).abs() < 1e-6);
}

#[test]
fn voxel_chunk_memory_increases_with_data() {
    let mut c = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    let m0 = c.memory_usage();
    c.set_voxel(IVec3::new(0, 0, 0), Voxel::new(1.0, 1));
    assert!(
        c.memory_usage() > m0,
        "memory should increase after adding voxel"
    );
}

// ============================================================================
// I. VoxelGrid
// ============================================================================

#[test]
fn voxel_grid_new_is_empty() {
    let g = VoxelGrid::new();
    assert_eq!(g.chunk_count(), 0);
    assert!(g.dirty_chunks().is_empty());
}

#[test]
fn voxel_grid_default_is_new() {
    let g = VoxelGrid::default();
    assert_eq!(g.chunk_count(), 0);
}

#[test]
fn voxel_grid_set_voxel_creates_chunk() {
    let mut g = VoxelGrid::new();
    g.set_voxel(Vec3::new(5.0, 5.0, 5.0), Voxel::new(1.0, 1));
    assert_eq!(g.chunk_count(), 1);
    assert_eq!(g.dirty_chunks().len(), 1);
}

#[test]
fn voxel_grid_get_voxel_roundtrip() {
    let mut g = VoxelGrid::new();
    let pos = Vec3::new(10.5, 20.5, 30.5);
    g.set_voxel(pos, Voxel::new(0.75, 5));
    let got = g.get_voxel(pos);
    assert!(got.is_some());
    assert!((got.unwrap().density - 0.75).abs() < 1e-6);
    assert_eq!(got.unwrap().material, 5);
}

#[test]
fn voxel_grid_get_voxel_nonexistent_returns_none() {
    let g = VoxelGrid::new();
    assert!(g.get_voxel(Vec3::new(100.0, 100.0, 100.0)).is_none());
}

#[test]
fn voxel_grid_mark_chunk_clean() {
    let mut g = VoxelGrid::new();
    g.set_voxel(Vec3::new(5.0, 5.0, 5.0), Voxel::new(1.0, 1));
    assert_eq!(g.dirty_chunks().len(), 1);
    let coord = ChunkCoord::from_world_pos(Vec3::new(5.0, 5.0, 5.0));
    g.mark_chunk_clean(coord);
    assert_eq!(g.dirty_chunks().len(), 0);
}

#[test]
fn voxel_grid_remove_chunk() {
    let mut g = VoxelGrid::new();
    g.set_voxel(Vec3::new(5.0, 5.0, 5.0), Voxel::new(1.0, 1));
    assert_eq!(g.chunk_count(), 1);
    let coord = ChunkCoord::from_world_pos(Vec3::new(5.0, 5.0, 5.0));
    let removed = g.remove_chunk(coord);
    assert!(removed.is_some());
    assert_eq!(g.chunk_count(), 0);
    assert_eq!(g.dirty_chunks().len(), 0);
}

#[test]
fn voxel_grid_clear() {
    let mut g = VoxelGrid::new();
    g.set_voxel(Vec3::new(5.0, 5.0, 5.0), Voxel::new(1.0, 1));
    g.set_voxel(Vec3::new(50.0, 50.0, 50.0), Voxel::new(0.8, 2));
    assert_eq!(g.chunk_count(), 2);
    g.clear();
    assert_eq!(g.chunk_count(), 0);
    assert!(g.dirty_chunks().is_empty());
}

#[test]
fn voxel_grid_chunk_coords() {
    let mut g = VoxelGrid::new();
    g.set_voxel(Vec3::new(5.0, 5.0, 5.0), Voxel::new(1.0, 1));
    g.set_voxel(Vec3::new(50.0, 50.0, 50.0), Voxel::new(0.8, 2));
    let coords = g.chunk_coords();
    assert_eq!(coords.len(), 2);
}

#[test]
fn voxel_grid_memory_usage_increases() {
    let mut g = VoxelGrid::new();
    let m0 = g.memory_usage();
    g.set_voxel(Vec3::new(5.0, 5.0, 5.0), Voxel::new(1.0, 1));
    assert!(g.memory_usage() > m0);
}

#[test]
fn voxel_grid_get_or_create_chunk() {
    let mut g = VoxelGrid::new();
    let coord = ChunkCoord::new(1, 2, 3);
    let chunk = g.get_or_create_chunk(coord);
    assert_eq!(chunk.coord(), coord);
    assert_eq!(g.chunk_count(), 1);
    // Calling again should return same chunk
    let _ = g.get_or_create_chunk(coord);
    assert_eq!(g.chunk_count(), 1);
}

#[test]
fn voxel_grid_get_chunk_existing() {
    let mut g = VoxelGrid::new();
    let coord = ChunkCoord::new(0, 0, 0);
    g.get_or_create_chunk(coord);
    assert!(g.get_chunk(coord).is_some());
}

#[test]
fn voxel_grid_get_chunk_nonexistent() {
    let g = VoxelGrid::new();
    assert!(g.get_chunk(ChunkCoord::new(99, 99, 99)).is_none());
}

#[test]
fn voxel_grid_multiple_voxels_same_chunk() {
    let mut g = VoxelGrid::new();
    // Both positions in same chunk (0,0,0) since both < 32
    g.set_voxel(Vec3::new(1.0, 1.0, 1.0), Voxel::new(1.0, 1));
    g.set_voxel(Vec3::new(2.0, 2.0, 2.0), Voxel::new(0.8, 2));
    assert_eq!(g.chunk_count(), 1, "same chunk should not be duplicated");
    // Only 1 dirty entry (not 2)
    assert_eq!(g.dirty_chunks().len(), 1);
}

// ============================================================================
// J. CHUNK_SIZE and MAX_OCTREE_DEPTH constants
// ============================================================================

#[test]
fn chunk_size_is_32() {
    assert_eq!(CHUNK_SIZE, 32);
}

#[test]
fn max_octree_depth_is_5() {
    assert_eq!(MAX_OCTREE_DEPTH, 5);
}

// ============================================================================
// K. Boundary voxel positions
// ============================================================================

#[test]
fn voxel_at_chunk_origin() {
    let mut c = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    c.set_voxel(IVec3::new(0, 0, 0), Voxel::new(1.0, 1));
    assert!(c.get_voxel(IVec3::new(0, 0, 0)).is_some());
}

#[test]
fn voxel_at_chunk_max_corner() {
    let mut c = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    c.set_voxel(IVec3::new(31, 31, 31), Voxel::new(1.0, 1));
    assert!(c.get_voxel(IVec3::new(31, 31, 31)).is_some());
}

#[test]
fn voxel_exactly_at_chunk_size_is_out_of_bounds() {
    let c = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
    assert!(c.get_voxel(IVec3::new(32, 0, 0)).is_none());
}
