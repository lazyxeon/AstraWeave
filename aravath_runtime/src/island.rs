//! Small solid island. Not the 10 km fill in `hybrid_voxel_demo`.

use astraweave_terrain::{Voxel, VoxelGrid};
use glam::Vec3;

/// Columns of solid voxels. `extent` is the X/Z size in voxels, starting at the origin.
pub fn fill_island(extent: i32) -> VoxelGrid {
    let mut grid = VoxelGrid::new();
    for x in 0..extent {
        for z in 0..extent {
            let h = surface_height(x, z);
            for y in 0..=h {
                let material = if y < h { 1 } else { 3 };
                grid.set_voxel(
                    Vec3::new(x as f32, y as f32, z as f32),
                    Voxel::new(1.0, material),
                );
            }
        }
    }
    grid
}

/// A few 32³ chunks. 48 voxels spans chunk 0 and chunk 1 on X and Z.
pub fn small() -> VoxelGrid {
    fill_island(48)
}

pub fn surface_height(x: i32, z: i32) -> i32 {
    let wave = (x as f32 * 0.15).sin() * (z as f32 * 0.15).cos();
    (8.0 + 2.0 * wave).floor() as i32
}

/// Axis-aligned solid box, inclusive, for occlusion tests.
pub fn solid_box(min: [i32; 3], max: [i32; 3]) -> VoxelGrid {
    let mut grid = VoxelGrid::new();
    for x in min[0]..=max[0] {
        for y in min[1]..=max[1] {
            for z in min[2]..=max[2] {
                grid.set_voxel(Vec3::new(x as f32, y as f32, z as f32), Voxel::new(1.0, 1));
            }
        }
    }
    grid
}
