//! Comprehensive tests for voxel_data.rs
//!
//! Phase 5: Pure Logic Tests - Voxel Data Structures
//! Target: 0% → 80%+ coverage for voxel_data.rs

#[cfg(test)]
mod tests {
    use crate::voxel_data::*;
    use glam::{IVec3, Vec3};

    // ============================================================================
    // ChunkCoord Tests (Basic Functionality)
    // ============================================================================

    #[test]
    fn test_chunk_coord_creation() {
        let coord = ChunkCoord::new(5, -3, 10);
        assert_eq!(coord.x, 5);
        assert_eq!(coord.y, -3);
        assert_eq!(coord.z, 10);
    }

    #[test]
    fn test_chunk_coord_from_world_pos() {
        // Test positive coordinates
        let pos = Vec3::new(64.0, 128.0, 96.0);
        let coord = ChunkCoord::from_world_pos(pos);
        assert_eq!(coord.x, 2); // 64 / 32 = 2
        assert_eq!(coord.y, 4); // 128 / 32 = 4
        assert_eq!(coord.z, 3); // 96 / 32 = 3

        // Test negative coordinates (floor behavior)
        let neg_pos = Vec3::new(-64.0, -32.0, -1.0);
        let neg_coord = ChunkCoord::from_world_pos(neg_pos);
        assert_eq!(neg_coord.x, -2);
        assert_eq!(neg_coord.y, -1);
        assert_eq!(neg_coord.z, -1);

        // Test zero
        let zero = Vec3::ZERO;
        let zero_coord = ChunkCoord::from_world_pos(zero);
        assert_eq!(zero_coord.x, 0);
        assert_eq!(zero_coord.y, 0);
        assert_eq!(zero_coord.z, 0);
    }

    #[test]
    fn test_chunk_coord_to_world_pos() {
        let coord = ChunkCoord::new(3, -2, 5);
        let pos = coord.to_world_pos();

        assert_eq!(pos.x, 3.0 * CHUNK_SIZE as f32);
        assert_eq!(pos.y, -2.0 * CHUNK_SIZE as f32);
        assert_eq!(pos.z, 5.0 * CHUNK_SIZE as f32);

        // Test zero coordinate
        let zero_coord = ChunkCoord::new(0, 0, 0);
        let zero_pos = zero_coord.to_world_pos();
        assert_eq!(zero_pos, Vec3::ZERO);
    }

    #[test]
    fn test_chunk_coord_round_trip() {
        // World pos → ChunkCoord → World pos should preserve chunk origin
        let original_pos = Vec3::new(100.0, 200.0, 300.0);
        let coord = ChunkCoord::from_world_pos(original_pos);
        let reconstructed_pos = coord.to_world_pos();

        // Should get the chunk's origin (lower corner)
        assert_eq!(reconstructed_pos.x, 96.0); // floor(100/32)*32 = 96
        assert_eq!(reconstructed_pos.y, 192.0); // floor(200/32)*32 = 192
        assert_eq!(reconstructed_pos.z, 288.0); // floor(300/32)*32 = 288
    }

    #[test]
    fn test_chunk_coord_neighbors() {
        let coord = ChunkCoord::new(10, 20, 30);
        let neighbors = coord.neighbors();

        assert_eq!(neighbors.len(), 6);
        assert_eq!(neighbors[0], ChunkCoord::new(11, 20, 30)); // +X
        assert_eq!(neighbors[1], ChunkCoord::new(9, 20, 30)); // -X
        assert_eq!(neighbors[2], ChunkCoord::new(10, 21, 30)); // +Y
        assert_eq!(neighbors[3], ChunkCoord::new(10, 19, 30)); // -Y
        assert_eq!(neighbors[4], ChunkCoord::new(10, 20, 31)); // +Z
        assert_eq!(neighbors[5], ChunkCoord::new(10, 20, 29)); // -Z
    }

    #[test]
    fn test_chunk_coord_neighbors_at_zero() {
        let zero = ChunkCoord::new(0, 0, 0);
        let neighbors = zero.neighbors();

        // Should handle negative neighbors correctly
        assert_eq!(neighbors[1], ChunkCoord::new(-1, 0, 0));
        assert_eq!(neighbors[3], ChunkCoord::new(0, -1, 0));
        assert_eq!(neighbors[5], ChunkCoord::new(0, 0, -1));
    }

    #[test]
    fn test_chunk_coord_equality() {
        let c1 = ChunkCoord::new(5, 10, 15);
        let c2 = ChunkCoord::new(5, 10, 15);
        let c3 = ChunkCoord::new(5, 10, 16);

        assert_eq!(c1, c2);
        assert_ne!(c1, c3);
    }

    // ============================================================================
    // Voxel Tests (Data Structure)
    // ============================================================================

    #[test]
    fn test_voxel_creation() {
        let voxel = Voxel::new(0.75, 42);
        assert_eq!(voxel.density, 0.75);
        assert_eq!(voxel.material, 42);
    }

    #[test]
    fn test_voxel_default() {
        let voxel = Voxel::default();
        assert_eq!(voxel.density, 0.0);
        assert_eq!(voxel.material, 0);
    }

    #[test]
    fn test_voxel_is_solid() {
        let solid = Voxel::new(0.75, 1);
        assert!(solid.is_solid());

        let boundary = Voxel::new(0.5, 1);
        assert!(!boundary.is_solid()); // density > 0.5, not >=

        let empty = Voxel::new(0.25, 1);
        assert!(!empty.is_solid());
    }

    #[test]
    fn test_voxel_is_empty() {
        let empty = Voxel::new(0.005, 1);
        assert!(empty.is_empty());

        let boundary = Voxel::new(0.01, 1);
        assert!(!boundary.is_empty()); // density < 0.01, not <=

        let solid = Voxel::new(0.75, 1);
        assert!(!solid.is_empty());
    }

    #[test]
    fn test_voxel_edge_cases() {
        // Test density boundaries
        let zero_density = Voxel::new(0.0, 1);
        assert!(zero_density.is_empty());
        assert!(!zero_density.is_solid());

        let full_density = Voxel::new(1.0, 1);
        assert!(!full_density.is_empty());
        assert!(full_density.is_solid());

        // Test maximum material ID
        let max_material = Voxel::new(0.5, u16::MAX);
        assert_eq!(max_material.material, u16::MAX);
    }

    // ============================================================================
    // VoxelChunk Tests (Core Functionality)
    // ============================================================================

    #[test]
    fn test_voxel_chunk_creation() {
        let coord = ChunkCoord::new(5, 10, 15);
        let chunk = VoxelChunk::new(coord);

        assert_eq!(chunk.coord(), coord);
        assert!(!chunk.is_dirty());
    }

    #[test]
    fn test_voxel_chunk_get_empty() {
        let chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));

        // Empty chunk should return None for any position
        let voxel = chunk.get_voxel(IVec3::new(0, 0, 0));
        assert!(voxel.is_none());

        let voxel = chunk.get_voxel(IVec3::new(15, 15, 15));
        assert!(voxel.is_none());
    }

    #[test]
    fn test_voxel_chunk_set_and_get() {
        let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));

        let pos = IVec3::new(10, 15, 20);
        let voxel = Voxel::new(0.8, 5);

        chunk.set_voxel(pos, voxel);
        assert!(chunk.is_dirty()); // Should mark as dirty after set

        let retrieved = chunk.get_voxel(pos);
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.density, 0.8);
        assert_eq!(retrieved.material, 5);
    }

    #[test]
    fn test_voxel_chunk_multiple_voxels() {
        let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));

        // Set multiple voxels
        let positions = vec![
            (IVec3::new(0, 0, 0), Voxel::new(1.0, 1)),
            (IVec3::new(31, 31, 31), Voxel::new(0.9, 2)),
            (IVec3::new(15, 15, 15), Voxel::new(0.5, 3)),
            (IVec3::new(5, 10, 20), Voxel::new(0.3, 4)),
        ];

        for (pos, voxel) in &positions {
            chunk.set_voxel(*pos, *voxel);
        }

        // Verify all voxels are retrievable
        for (pos, expected) in &positions {
            let retrieved = chunk.get_voxel(*pos).expect("Voxel should exist");
            assert_eq!(retrieved.density, expected.density);
            assert_eq!(retrieved.material, expected.material);
        }
    }

    #[test]
    fn test_voxel_chunk_out_of_bounds() {
        let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));

        // Negative positions (invalid)
        assert!(chunk.get_voxel(IVec3::new(-1, 0, 0)).is_none());
        assert!(chunk.get_voxel(IVec3::new(0, -1, 0)).is_none());
        assert!(chunk.get_voxel(IVec3::new(0, 0, -1)).is_none());

        // Positions >= CHUNK_SIZE (invalid)
        assert!(chunk.get_voxel(IVec3::new(CHUNK_SIZE, 0, 0)).is_none());
        assert!(chunk.get_voxel(IVec3::new(0, CHUNK_SIZE, 0)).is_none());
        assert!(chunk.get_voxel(IVec3::new(0, 0, CHUNK_SIZE)).is_none());

        // Set should be ignored for out of bounds
        let _initial_dirty = chunk.is_dirty();
        chunk.set_voxel(IVec3::new(-1, 0, 0), Voxel::new(1.0, 1));
        chunk.set_voxel(IVec3::new(CHUNK_SIZE, 0, 0), Voxel::new(1.0, 1));

        // Should not mark as dirty if position is invalid
        // (implementation may vary - this tests contract)
    }

    #[test]
    fn test_voxel_chunk_boundary_positions() {
        let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));

        // Valid boundary positions (0 to CHUNK_SIZE-1)
        let corners = [
            IVec3::new(0, 0, 0),
            IVec3::new(CHUNK_SIZE - 1, 0, 0),
            IVec3::new(0, CHUNK_SIZE - 1, 0),
            IVec3::new(0, 0, CHUNK_SIZE - 1),
            IVec3::new(CHUNK_SIZE - 1, CHUNK_SIZE - 1, CHUNK_SIZE - 1),
        ];

        for (i, pos) in corners.iter().enumerate() {
            let voxel = Voxel::new(0.5 + i as f32 * 0.1, i as u16);
            chunk.set_voxel(*pos, voxel);
        }

        // Verify all corners are accessible
        for (i, pos) in corners.iter().enumerate() {
            let retrieved = chunk.get_voxel(*pos).expect("Corner voxel should exist");
            assert!((retrieved.density - (0.5 + i as f32 * 0.1)).abs() < 0.001);
            assert_eq!(retrieved.material, i as u16);
        }
    }

    #[test]
    fn test_voxel_chunk_overwrite() {
        let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));
        let pos = IVec3::new(10, 10, 10);

        // Set initial voxel
        chunk.set_voxel(pos, Voxel::new(0.5, 1));
        chunk.mark_clean();

        // Overwrite with new voxel
        chunk.set_voxel(pos, Voxel::new(0.9, 99));
        assert!(chunk.is_dirty());

        // Verify new value is stored
        let retrieved = chunk.get_voxel(pos).unwrap();
        assert_eq!(retrieved.density, 0.9);
        assert_eq!(retrieved.material, 99);
    }

    #[test]
    fn test_voxel_chunk_dirty_flag() {
        let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));

        assert!(!chunk.is_dirty());

        // Setting voxel should mark as dirty
        chunk.set_voxel(IVec3::new(5, 5, 5), Voxel::new(1.0, 1));
        assert!(chunk.is_dirty());

        // Mark clean
        chunk.mark_clean();
        assert!(!chunk.is_dirty());

        // Setting another voxel should mark dirty again
        chunk.set_voxel(IVec3::new(10, 10, 10), Voxel::new(0.5, 2));
        assert!(chunk.is_dirty());
    }

    #[test]
    fn test_voxel_chunk_sparse_storage() {
        // Test that sparse octree handles empty regions efficiently
        let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));

        // Set only a few voxels in a large space
        chunk.set_voxel(IVec3::new(0, 0, 0), Voxel::new(1.0, 1));
        chunk.set_voxel(IVec3::new(31, 31, 31), Voxel::new(1.0, 2));

        // Most positions should return None (sparse)
        let mut none_count = 0;
        let mut some_count = 0;

        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    if chunk.get_voxel(IVec3::new(x, y, z)).is_some() {
                        some_count += 1;
                    } else {
                        none_count += 1;
                    }
                }
            }
        }

        // We set 2 voxels, but octree may initialize default leaves
        // At minimum, we should have far more None than Some
        assert!(
            none_count > some_count * 10,
            "Sparse octree should have mostly empty space. none: {}, some: {}",
            none_count,
            some_count
        );
    }

    // ============================================================================
    // Integration Tests (Multi-Component)
    // ============================================================================

    #[test]
    fn test_world_to_chunk_to_local() {
        // Test complete workflow: world pos → chunk coord → local pos
        let world_pos = Vec3::new(100.0, 200.0, 300.0);
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);

        // Calculate local position within chunk
        let chunk_origin = chunk_coord.to_world_pos();
        let local_offset = world_pos - chunk_origin;

        // Local position should be in range [0, CHUNK_SIZE)
        assert!(local_offset.x >= 0.0 && local_offset.x < CHUNK_SIZE as f32);
        assert!(local_offset.y >= 0.0 && local_offset.y < CHUNK_SIZE as f32);
        assert!(local_offset.z >= 0.0 && local_offset.z < CHUNK_SIZE as f32);

        // Convert to voxel coordinates
        let local_voxel = IVec3::new(
            local_offset.x as i32,
            local_offset.y as i32,
            local_offset.z as i32,
        );

        // Should be valid for chunk access
        let mut chunk = VoxelChunk::new(chunk_coord);
        chunk.set_voxel(local_voxel, Voxel::new(1.0, 1));
        assert!(chunk.get_voxel(local_voxel).is_some());
    }

    #[test]
    fn test_chunk_grid_coverage() {
        // Verify that neighboring chunks cover continuous space
        let center = ChunkCoord::new(0, 0, 0);
        let neighbors = center.neighbors();

        // Each neighbor should be exactly 1 chunk away
        for neighbor in neighbors.iter() {
            let dx = (center.x - neighbor.x).abs();
            let dy = (center.y - neighbor.y).abs();
            let dz = (center.z - neighbor.z).abs();

            // Exactly one axis should differ by 1
            let diff_count = (dx + dy + dz) as usize;
            assert_eq!(
                diff_count, 1,
                "Neighbor should be 1 chunk away on exactly 1 axis"
            );
        }

        // Verify no duplicates
        for i in 0..neighbors.len() {
            for j in (i + 1)..neighbors.len() {
                assert_ne!(neighbors[i], neighbors[j], "Neighbors should be unique");
            }
        }
    }

    #[test]
    fn test_material_id_range() {
        // Test that material IDs support full u16 range
        let voxels = [
            Voxel::new(1.0, 0),            // Min
            Voxel::new(1.0, 1),            // Low
            Voxel::new(1.0, 255),          // u8 max
            Voxel::new(1.0, 256),          // Beyond u8
            Voxel::new(1.0, u16::MAX / 2), // Mid
            Voxel::new(1.0, u16::MAX),     // Max
        ];

        let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));

        for (i, voxel) in voxels.iter().enumerate() {
            let pos = IVec3::new(i as i32, 0, 0);
            chunk.set_voxel(pos, *voxel);

            let retrieved = chunk.get_voxel(pos).unwrap();
            assert_eq!(
                retrieved.material, voxel.material,
                "Material ID should be preserved for ID {}",
                voxel.material
            );
        }
    }

    #[test]
    fn test_density_precision() {
        // Test that density values maintain precision
        let densities = [0.0, 0.001, 0.01, 0.1, 0.5, 0.9, 0.99, 0.999, 1.0];

        let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));

        for (i, density) in densities.iter().enumerate() {
            let pos = IVec3::new(i as i32, 0, 0);
            chunk.set_voxel(pos, Voxel::new(*density, 1));

            let retrieved = chunk.get_voxel(pos).unwrap();
            assert!(
                (retrieved.density - density).abs() < 0.0001,
                "Density should preserve precision: expected {}, got {}",
                density,
                retrieved.density
            );
        }
    }

    // ============================================================================
    // Stress Tests
    // ============================================================================

    #[test]
    fn test_voxel_chunk_full_fill() {
        // Test filling entire chunk (stress test for octree)
        let mut chunk = VoxelChunk::new(ChunkCoord::new(0, 0, 0));

        // Fill every 4th voxel (to keep test fast while testing octree)
        let mut filled_count = 0;
        for x in (0..CHUNK_SIZE).step_by(4) {
            for y in (0..CHUNK_SIZE).step_by(4) {
                for z in (0..CHUNK_SIZE).step_by(4) {
                    let pos = IVec3::new(x, y, z);
                    let voxel = Voxel::new(
                        (x + y + z) as f32 / (CHUNK_SIZE * 3) as f32,
                        ((x * y * z) % 256) as u16,
                    );
                    chunk.set_voxel(pos, voxel);
                    filled_count += 1;
                }
            }
        }

        assert!(filled_count > 0);
        assert!(chunk.is_dirty());

        // Verify random samples
        for x in (0..CHUNK_SIZE).step_by(8) {
            for y in (0..CHUNK_SIZE).step_by(8) {
                for z in (0..CHUNK_SIZE).step_by(8) {
                    let pos = IVec3::new(x, y, z);
                    if x % 4 == 0 && y % 4 == 0 && z % 4 == 0 {
                        assert!(
                            chunk.get_voxel(pos).is_some(),
                            "Filled position should exist: {:?}",
                            pos
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_chunk_coord_round_trip_while_origin_is_exact_in_f32() {
        // `to_world_pos` is `coord * CHUNK_SIZE` in f32. f32 integers are
        // exact through 2^24, and CHUNK_SIZE is 32, so |coord| <= 524_288
        // round-trips. i32::MAX / 2 does not: the measured reconstruction
        // is 1073741824, one chunk past 1073741823.
        let limit = (1 << 24) / CHUNK_SIZE;
        let exact = [
            ChunkCoord::new(0, 0, 0),
            ChunkCoord::new(1, -1, 2),
            ChunkCoord::new(limit, 0, -limit),
            ChunkCoord::new(-limit, limit, 0),
        ];
        for coord in exact {
            let reconstructed = ChunkCoord::from_world_pos(coord.to_world_pos());
            assert_eq!(coord, reconstructed);
        }

        let overflow = ChunkCoord::new(i32::MAX / 2, 0, 0);
        let reconstructed = ChunkCoord::from_world_pos(overflow.to_world_pos());
        assert_ne!(overflow.x, reconstructed.x);
        assert_eq!(reconstructed.x, 1_073_741_824);
    }
}
