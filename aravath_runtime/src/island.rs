//! Small solid island. Not the 10 km fill in `hybrid_voxel_demo`.

use astraweave_terrain::{Voxel, VoxelGrid};
use glam::Vec3;

pub const MAT_STONE: u16 = 1;
pub const MAT_DIRT: u16 = 2;
pub const MAT_GRASS: u16 = 3;
pub const MAT_SAND: u16 = 4;
pub const MAT_WOOD: u16 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Biome {
    Grass,
    Sand,
    Rock,
}

pub fn biome_at(x: i32, z: i32) -> Biome {
    let n = (x as f32 * 0.012).sin() * 0.65 + (z as f32 * 0.015).cos() * 0.45;
    if n > 0.55 {
        Biome::Sand
    } else if n < -0.42 {
        Biome::Rock
    } else {
        Biome::Grass
    }
}

pub fn column_material(y: i32, height: i32, biome: Biome) -> u16 {
    match biome {
        Biome::Sand => {
            if y >= height.saturating_sub(2) {
                MAT_SAND
            } else {
                MAT_STONE
            }
        }
        Biome::Rock => MAT_STONE,
        Biome::Grass => {
            if y >= height {
                MAT_GRASS
            } else if y >= height.saturating_sub(3) {
                MAT_DIRT
            } else {
                MAT_STONE
            }
        }
    }
}

/// Columns of solid voxels. `extent` is the X/Z size in voxels, starting at the origin.
pub fn fill_island(extent: i32) -> VoxelGrid {
    let mut grid = VoxelGrid::new();
    for x in 0..extent {
        for z in 0..extent {
            let h = surface_height(x, z);
            let biome = biome_at(x, z);
            for y in 0..=h {
                grid.set_voxel(
                    Vec3::new(x as f32, y as f32, z as f32),
                    Voxel::new(1.0, column_material(y, h, biome)),
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
    let broad = (x as f32 * 0.02).sin() * (z as f32 * 0.018).cos();
    (8.0 + 2.0 * wave + 2.5 * broad).floor() as i32
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn biomes_include_grass_sand_and_rock() {
        let mut kinds = HashSet::new();
        for x in -240..240 {
            for z in (-240..240).step_by(6) {
                kinds.insert(biome_at(x, z));
            }
        }
        assert!(kinds.contains(&Biome::Grass));
        assert!(kinds.contains(&Biome::Sand));
        assert!(kinds.contains(&Biome::Rock));
    }

    #[test]
    fn grass_columns_stack_three_materials() {
        assert_eq!(column_material(10, 10, Biome::Grass), MAT_GRASS);
        assert_eq!(column_material(8, 10, Biome::Grass), MAT_DIRT);
        assert_eq!(column_material(2, 10, Biome::Grass), MAT_STONE);
        assert_eq!(column_material(10, 10, Biome::Sand), MAT_SAND);
        assert_eq!(column_material(4, 10, Biome::Sand), MAT_STONE);
        assert_eq!(column_material(10, 10, Biome::Rock), MAT_STONE);
    }
}
