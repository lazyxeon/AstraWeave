//! Integer column world for the forest greybox.
//!
//! Heights, surface materials, and scatter are functions of
//! `(FOREST_WORLD_SEED, x, z)`. The arithmetic is wrapping integers.
//! Species and yaw come from [`astraweave_core::sim::SimRng`]. Nothing in
//! this module iterates a `HashMap`, and nothing calls libm.

use astraweave_core::sim::{SimRng, SIM_SYSTEM_FOREST_SCATTER};
use glam::Vec3;
use rand::RngCore;

use crate::voxel_data::{Voxel, VoxelGrid, CHUNK_SIZE};

/// Greybox world seed. The Aravath opening and the replay hash both use it.
pub const FOREST_WORLD_SEED: u64 = 0xA2A0_7E11;

pub const MAT_STONE: u16 = 1;
pub const MAT_DIRT: u16 = 2;
pub const MAT_GRASS: u16 = 3;
pub const MAT_SAND: u16 = 4;
pub const MAT_WOOD: u16 = 5;

/// Opening ring around the western spawn. Palms stay outside it.
pub const OPENING_MIN: i32 = -96;
pub const OPENING_MAX: i32 = 128;

const CELL: i32 = 16;
const STRIDE: i32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColumnBiome {
    Grass,
    Sand,
    Rock,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ForestProp {
    Oak = 1,
    Pine = 2,
    Palm = 3,
    Rock = 4,
    Grass = 5,
    Bush = 6,
}

impl ForestProp {
    pub fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            1 => Self::Oak,
            2 => Self::Pine,
            3 => Self::Palm,
            4 => Self::Rock,
            5 => Self::Grass,
            6 => Self::Bush,
            _ => return None,
        })
    }

    pub fn scale_milli(self) -> u16 {
        match self {
            Self::Oak | Self::Pine => 3200,
            Self::Palm => 3000,
            Self::Rock => 2400,
            Self::Bush => 2200,
            Self::Grass => 1600,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PropPlacement {
    pub kind: ForestProp,
    pub x: i32,
    pub z: i32,
    pub yaw_quanta: u8,
    pub scale_milli: u16,
    pub surface_y: i32,
}

impl PropPlacement {
    /// Render yaw. The hashed value is [`Self::yaw_quanta`], not this float.
    pub fn yaw_radians(self) -> f32 {
        const STEP: f32 = std::f32::consts::TAU / 256.0;
        self.yaw_quanta as f32 * STEP
    }
}

pub fn on_yaqar_road(x: i32, z: i32) -> bool {
    (z - 16).abs() <= 1 && x >= 8
}

pub fn in_opening(x: i32, z: i32) -> bool {
    (OPENING_MIN..OPENING_MAX).contains(&x) && (OPENING_MIN..OPENING_MAX).contains(&z)
}

pub fn biome_at(x: i32, z: i32) -> ColumnBiome {
    if in_opening(x, z) {
        return ColumnBiome::Grass;
    }
    match mix(FOREST_WORLD_SEED ^ 0xB10, x, z) % 7 {
        0 | 1 => ColumnBiome::Sand,
        2 => ColumnBiome::Rock,
        _ => ColumnBiome::Grass,
    }
}

pub fn surface_height(x: i32, z: i32) -> i32 {
    let cell_x = x.div_euclid(CELL);
    let cell_z = z.div_euclid(CELL);
    let tx = fade(x.rem_euclid(CELL));
    let tz = fade(z.rem_euclid(CELL));
    let h00 = corner_height(cell_x, cell_z);
    let h10 = corner_height(cell_x + 1, cell_z);
    let h01 = corner_height(cell_x, cell_z + 1);
    let h11 = corner_height(cell_x + 1, cell_z + 1);
    let h0 = lerp(h00, h10, tx);
    let h1 = lerp(h01, h11, tx);
    lerp(h0, h1, tz).clamp(1, 24)
}

pub fn column_material(y: i32, height: i32, biome: ColumnBiome, road: bool) -> u16 {
    if road && y >= height.saturating_sub(1) {
        return MAT_DIRT;
    }
    match biome {
        ColumnBiome::Sand => {
            if y >= height.saturating_sub(2) {
                MAT_SAND
            } else {
                MAT_STONE
            }
        }
        ColumnBiome::Rock => MAT_STONE,
        ColumnBiome::Grass => {
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

pub fn write_column(grid: &mut VoxelGrid, x: i32, z: i32) {
    let height = surface_height(x, z);
    let biome = biome_at(x, z);
    let road = on_yaqar_road(x, z);
    for y in 0..=height {
        let material = column_material(y, height, biome, road);
        grid.set_voxel(
            Vec3::new(x as f32, y as f32, z as f32),
            Voxel::new(1.0, material),
        );
    }
    if biome == ColumnBiome::Grass
        && !road
        && mix(FOREST_WORLD_SEED ^ 0x10C, x, z) % 53 == 0
        && x.rem_euclid(CHUNK_SIZE) + 3 < CHUNK_SIZE
    {
        for step in 0..3 {
            grid.set_voxel(
                Vec3::new((x + step) as f32, (height + 1) as f32, z as f32),
                Voxel::new(1.0, MAT_WOOD),
            );
        }
    }
}

pub fn prop_at(x: i32, z: i32) -> Option<PropPlacement> {
    if on_yaqar_road(x, z) {
        return None;
    }
    if (x.rem_euclid(STRIDE) != 2) || (z.rem_euclid(STRIDE) != 2) {
        return None;
    }
    let mut rng = SimRng::new(
        FOREST_WORLD_SEED,
        cell_tick(x, z),
        SIM_SYSTEM_FOREST_SCATTER,
    );
    let roll = rng.next_u32() % 20;
    let yaw_quanta = (rng.next_u32() & 255) as u8;
    let biome = biome_at(x, z);
    let kind = match biome {
        ColumnBiome::Grass => match roll {
            0..=4 => ForestProp::Grass,
            5..=7 => ForestProp::Bush,
            8 => ForestProp::Rock,
            9..=12 => ForestProp::Oak,
            13..=16 => ForestProp::Pine,
            _ => return None,
        },
        ColumnBiome::Sand => match roll {
            0..=6 => ForestProp::Palm,
            7..=10 => ForestProp::Rock,
            _ => return None,
        },
        ColumnBiome::Rock => match roll {
            0..=8 => ForestProp::Rock,
            9..=12 => ForestProp::Pine,
            _ => return None,
        },
    };
    if in_opening(x, z) && kind == ForestProp::Palm {
        return None;
    }
    Some(PropPlacement {
        kind,
        x,
        z,
        yaw_quanta,
        scale_milli: kind.scale_milli(),
        surface_y: surface_height(x, z),
    })
}

pub fn scatter_chunk(origin_x: i32, origin_z: i32) -> Vec<PropPlacement> {
    let mut spots = Vec::new();
    let mut x = 2;
    while x < CHUNK_SIZE - 2 {
        let mut z = 2;
        while z < CHUNK_SIZE - 2 {
            if let Some(prop) = prop_at(origin_x + x, origin_z + z) {
                spots.push(prop);
            }
            z += STRIDE;
        }
        x += STRIDE;
    }
    spots
}

/// Keep a prop for the camera at integer metres `(cam_x, cam_z)`.
/// Far grass and bushes drop first. Distant trees keep every other yaw.
pub fn lod_keeps(kind: ForestProp, cam_x: i32, cam_z: i32, x: i32, z: i32, yaw_quanta: u8) -> bool {
    let dx = (x - cam_x) as i64;
    let dz = (z - cam_z) as i64;
    let dist_sq = dx * dx + dz * dz;
    match kind {
        ForestProp::Grass => dist_sq <= 32 * 32,
        ForestProp::Bush => dist_sq <= 64 * 64,
        ForestProp::Rock => dist_sq <= 96 * 96,
        ForestProp::Palm => dist_sq <= 80 * 80,
        ForestProp::Oak | ForestProp::Pine => {
            if dist_sq <= 56 * 56 {
                true
            } else if dist_sq <= 140 * 140 {
                yaw_quanta % 2 == 0
            } else {
                false
            }
        }
    }
}

/// Little-endian record of three chunks, including a negative coordinate and
/// a road chunk. Sorted by construction. This is the `worldgen_columns` case.
pub fn worldgen_record() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"AWCOL1");
    out.extend_from_slice(&FOREST_WORLD_SEED.to_le_bytes());
    for (cx, cz) in [(-1i32, 2i32), (0, 0), (8, 0)] {
        out.extend_from_slice(&cx.to_le_bytes());
        out.extend_from_slice(&cz.to_le_bytes());
        for lx in 0..CHUNK_SIZE {
            for lz in 0..CHUNK_SIZE {
                let x = cx * CHUNK_SIZE + lx;
                let z = cz * CHUNK_SIZE + lz;
                let height = surface_height(x, z);
                let material = column_material(height, height, biome_at(x, z), on_yaqar_road(x, z));
                out.extend_from_slice(&(height as i16).to_le_bytes());
                out.extend_from_slice(&material.to_le_bytes());
                if let Some(prop) = prop_at(x, z) {
                    out.push(prop.kind as u8);
                    out.push(prop.yaw_quanta);
                    out.extend_from_slice(&prop.scale_milli.to_le_bytes());
                } else {
                    out.push(0);
                    out.push(0);
                    out.extend_from_slice(&0u16.to_le_bytes());
                }
            }
        }
    }
    out
}

fn cell_tick(x: i32, z: i32) -> u64 {
    ((x as u32 as u64) << 32) | (z as u32 as u64)
}

fn mix(seed: u64, x: i32, z: i32) -> u32 {
    let mut n = seed
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((x as u32 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add((z as u32 as u64).wrapping_mul(0x94D0_49BB_1331_11EB));
    n = (n ^ (n >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (n ^ (n >> 31)) as u32
}

fn corner_height(cell_x: i32, cell_z: i32) -> i32 {
    let fine = (mix(FOREST_WORLD_SEED, cell_x, cell_z) % 7) as i32;
    let broad = (mix(
        FOREST_WORLD_SEED ^ 0xBEE5,
        cell_x.div_euclid(4),
        cell_z.div_euclid(4),
    ) % 5) as i32;
    // West of the spawn sits a little higher: cell_x is negative on that side.
    let west = if cell_x < 0 { 2 } else { 0 };
    6 + fine + broad + west
}

fn fade(t: i32) -> i32 {
    let t = (t as i64) * 1024 / CELL as i64;
    let faded = t * t * (3 * 1024 - 2 * t) / (1024 * 1024);
    faded as i32
}

fn lerp(a: i32, b: i32, t: i32) -> i32 {
    a + ((b - a) * t) / 1024
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_is_stable_and_covers_a_negative_chunk() {
        assert_eq!(worldgen_record(), worldgen_record());
        assert!(worldgen_record()
            .windows(4)
            .any(|w| w == (-1i32).to_le_bytes()));
    }

    #[test]
    fn opening_is_forest_and_the_road_is_empty_dirt() {
        assert_eq!(biome_at(16, 20), ColumnBiome::Grass);
        assert!(on_yaqar_road(20, 16));
        assert_eq!(
            column_material(
                surface_height(20, 16),
                surface_height(20, 16),
                biome_at(20, 16),
                true
            ),
            MAT_DIRT
        );
        assert!(prop_at(20, 16).is_none());
        let mut trees = 0;
        for cx in -2..3 {
            for cz in -2..3 {
                for prop in scatter_chunk(cx * CHUNK_SIZE, cz * CHUNK_SIZE) {
                    assert_ne!(prop.kind, ForestProp::Palm);
                    if matches!(prop.kind, ForestProp::Oak | ForestProp::Pine) {
                        trees += 1;
                    }
                }
            }
        }
        assert!(trees > 20, "opening should read as a forest, trees={trees}");
        let mut palms = 0;
        let mut x = 202;
        while x < 400 {
            let mut z = 202;
            while z < 400 {
                if prop_at(x, z).is_some_and(|prop| prop.kind == ForestProp::Palm) {
                    palms += 1;
                }
                z += STRIDE;
            }
            x += STRIDE;
        }
        assert!(palms > 0, "palms stay available outside the opening");
    }

    #[test]
    fn view_ring_lod_keeps_the_forest_under_budget() {
        let started = std::time::Instant::now();
        let mut total = 0;
        let mut kept = 0;
        for cx in -3..=3 {
            for cz in -3..=3 {
                for prop in scatter_chunk(cx * CHUNK_SIZE, cz * CHUNK_SIZE) {
                    total += 1;
                    if lod_keeps(prop.kind, 16, 16, prop.x, prop.z, prop.yaw_quanta) {
                        kept += 1;
                    }
                }
            }
        }
        let elapsed = started.elapsed();
        eprintln!("forest scatter total={total} lod_kept={kept} cpu={elapsed:?}");
        assert!(total > 400, "forest should be dense, total={total}");
        assert!(
            kept < 900,
            "LOD should keep a 4060-class frame on the opening ring, kept={kept}"
        );
        assert!(elapsed.as_millis() < 50, "column gen took {elapsed:?}");
    }

    #[test]
    fn yaw_comes_from_simrng_and_lod_culls_far_grass() {
        let prop = prop_at(18, 22).expect("a stride cell in the opening");
        let again = prop_at(18, 22).unwrap();
        assert_eq!(prop.yaw_quanta, again.yaw_quanta);
        assert!(!lod_keeps(ForestProp::Grass, 0, 0, 400, 0, 0));
        assert!(lod_keeps(ForestProp::Oak, 0, 0, 10, 0, 1));
        assert!(!lod_keeps(ForestProp::Oak, 0, 0, 100, 0, 1));
        assert!(lod_keeps(ForestProp::Oak, 0, 0, 100, 0, 2));
    }
}
