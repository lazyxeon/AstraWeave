//! Wave 2 Golden-Value Projection Tests: clustered.rs bin_lights_cpu
//!
//! These tests verify EXACT cluster assignments using manually computed
//! golden values.  They target specific arithmetic mutations in the
//! projection / pixel-space math (lines 59-66 first pass, 112-117 second pass).
//!
//! MISSED mutations targeted:
//!   Line 59:35  `* fx`     → `/ fx`     (NDC x-projection)
//!   Line 60:35  `* fy`     → `+ fy`     (NDC y-projection)
//!   Line 62:25  `* width`  → `+ width`  (pixel-x from NDC)
//!   Line 65:57  `* 0.5`    → `+ 0.5`    (radius pixel x, doubles spread)
//!   Line 65:57  `* 0.5`    → `/ 0.5`    (radius pixel x, quadruples spread)

use astraweave_render::clustered::{bin_lights_cpu, ClusterDims, CpuLight};
use glam::Vec3;
use std::collections::BTreeSet;

const SCREEN: (u32, u32) = (640, 480);
const NEAR: f32 = 0.1;
const FAR: f32 = 100.0;
const FOV_Y: f32 = std::f32::consts::FRAC_PI_3; // 60°

// ===================== Helpers =====================

/// Which X columns (ix) have count > 0.
fn hit_x_columns(counts: &[u32], dims: &ClusterDims) -> BTreeSet<u32> {
    let mut set = BTreeSet::new();
    for iz in 0..dims.z {
        for iy in 0..dims.y {
            for ix in 0..dims.x {
                let ci = (ix + iy * dims.x + iz * dims.x * dims.y) as usize;
                if counts[ci] > 0 {
                    set.insert(ix);
                }
            }
        }
    }
    set
}

/// Which Y rows (iy) have count > 0.
fn hit_y_rows(counts: &[u32], dims: &ClusterDims) -> BTreeSet<u32> {
    let mut set = BTreeSet::new();
    for iz in 0..dims.z {
        for iy in 0..dims.y {
            for ix in 0..dims.x {
                let ci = (ix + iy * dims.x + iz * dims.x * dims.y) as usize;
                if counts[ci] > 0 {
                    set.insert(iy);
                }
            }
        }
    }
    set
}

/// Which Z slices (iz) have count > 0.
fn hit_z_slices(counts: &[u32], dims: &ClusterDims) -> BTreeSet<u32> {
    let mut set = BTreeSet::new();
    for iz in 0..dims.z {
        for iy in 0..dims.y {
            for ix in 0..dims.x {
                let ci = (ix + iy * dims.x + iz * dims.x * dims.y) as usize;
                if counts[ci] > 0 {
                    set.insert(iz);
                }
            }
        }
    }
    set
}

// ================================================================
// GROUP 1: X-projection golden values (catches lines 59, 62, 65)
// ================================================================
//
// Light at (5, 0, 10), r=0.5   dims 16×4×4
//   fx = 1.29904, tile_w = 40
//   ndc_x = (5/10)*1.29904 = 0.64952
//   px = (0.82476)*640 = 527.85
//   rpx_x = 0.06495 * 320 = 20.78
//   min_px=507  max_px=549  → ix 12..13
//
// With mutations:
//   *fx→/fx   : px≈443  → ix 10..11   (column 12 NOT hit)
//   *width→+w : px≈641  → ix 15       (column 12 NOT hit)
//   *0.5→+0.5 : rpx_x≈42 → ix 12..14 (column 14 HIT)
//   *0.5→/0.5 : rpx_x≈83 → ix 11..15 (column 11 HIT)

#[test]
fn golden_x_column_12_13_exact() {
    let dims = ClusterDims { x: 16, y: 4, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(5.0, 0.0, 10.0),
        radius: 0.5,
    }];
    let (counts, _, _) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let cols = hit_x_columns(&counts, &dims);
    assert!(
        cols.contains(&12) || cols.contains(&13),
        "Light at x=5 z=10 must hit column 12 or 13, got {:?}",
        cols,
    );
}

#[test]
fn golden_x_not_in_mutation_columns() {
    let dims = ClusterDims { x: 16, y: 4, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(5.0, 0.0, 10.0),
        radius: 0.5,
    }];
    let (counts, _, _) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let cols = hit_x_columns(&counts, &dims);
    // *fx→/fx shifts projection left to columns 10-11
    assert!(
        !cols.contains(&10),
        "*fx→/fx mutation shifts to col 10; got {:?}",
        cols
    );
    assert!(
        !cols.contains(&11),
        "*fx→/fx mutation adds col 11; got {:?}",
        cols
    );
    // *width→+width shifts projection far-right to col 15
    assert!(
        !cols.contains(&15),
        "*width→+width mutation shifts to 15; got {:?}",
        cols
    );
    // *0.5→+0.5 doubles rpx_x, expanding to col 14
    assert!(
        !cols.contains(&14),
        "*0.5→+0.5 mutation expands to 14; got {:?}",
        cols
    );
    // Column 0 is far from the projection
    assert!(
        !cols.contains(&0),
        "Should not hit column 0; got {:?}",
        cols
    );
}

#[test]
fn golden_x_small_radius_tight_span() {
    // r=0.5 at z=10 → rpx_x ≈ 21 px ≈ 0.5 tiles → ≤2 columns
    let dims = ClusterDims { x: 16, y: 4, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(5.0, 0.0, 10.0),
        radius: 0.5,
    }];
    let (counts, _, _) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let cols = hit_x_columns(&counts, &dims);
    assert!(
        cols.len() <= 2,
        "r=0.5 at z=10 should span ≤2 columns, got {:?}",
        cols
    );
}

// ================================================================
// GROUP 2: Y-projection golden values (catches line 60)
// ================================================================
//
// Light at (0, 4, 10), r=0.5   dims 4×16×4
//   fy = 1.73205, tile_h = 30
//   ndc_y = (4/10)*1.73205 = 0.69282
//   py = (0.84641)*480 = 406.28
//   rpx_y = 0.05 * 1.73205 * 240 = 20.78
//   min_py=385  max_py=428  → iy 12..14
//
// With mutations:
//   *fy→+fy : ndc_y=2.13, py=752 → iy clamped to 15 (row 12 NOT hit)

#[test]
fn golden_y_row_12_14_exact() {
    let dims = ClusterDims { x: 4, y: 16, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 4.0, 10.0),
        radius: 0.5,
    }];
    let (counts, _, _) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let rows = hit_y_rows(&counts, &dims);
    assert!(
        rows.contains(&12) || rows.contains(&13),
        "Light at y=4 z=10 must hit row 12 or 13, got {:?}",
        rows,
    );
}

#[test]
fn golden_y_not_in_mutation_rows() {
    let dims = ClusterDims { x: 4, y: 16, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 4.0, 10.0),
        radius: 0.5,
    }];
    let (counts, _, _) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let rows = hit_y_rows(&counts, &dims);
    // *fy→+fy shifts to clamped row 15
    assert!(
        !rows.contains(&15),
        "*fy→+fy mutation clamps to row 15; got {:?}",
        rows,
    );
    // Far from projection
    assert!(!rows.contains(&0), "Should not hit row 0; got {:?}", rows);
    assert!(!rows.contains(&1), "Should not hit row 1; got {:?}", rows);
}

#[test]
fn golden_y_row_span_bounded() {
    let dims = ClusterDims { x: 4, y: 16, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 4.0, 10.0),
        radius: 0.5,
    }];
    let (counts, _, _) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let rows = hit_y_rows(&counts, &dims);
    assert!(
        rows.len() <= 4,
        "r=0.5 at z=10 should span ≤4 rows, got {:?}",
        rows
    );
}

// ================================================================
// GROUP 3: Negative X position (mirror test)
// ================================================================
//
// Light at (-5, 0, 10), r=0.5   dims 16×4×4
//   ndc_x = -0.64952, px = 112.15
//   min_px=91, max_px=133 → ix 2..3
//
// With *fx→/fx : px≈197 → ix 4..5 (column 2 NOT hit)

#[test]
fn golden_negative_x_column_2_3() {
    let dims = ClusterDims { x: 16, y: 4, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(-5.0, 0.0, 10.0),
        radius: 0.5,
    }];
    let (counts, _, _) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let cols = hit_x_columns(&counts, &dims);
    assert!(
        cols.contains(&2) || cols.contains(&3),
        "Light at x=-5 z=10 must hit column 2 or 3, got {:?}",
        cols,
    );
    // Should not appear in the right half
    assert!(
        !cols.contains(&6),
        "Should not hit column 6; got {:?}",
        cols
    );
    assert!(
        !cols.contains(&12),
        "Should not hit column 12; got {:?}",
        cols
    );
}

#[test]
fn golden_negative_x_not_in_mutation_shifted() {
    let dims = ClusterDims { x: 16, y: 4, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(-5.0, 0.0, 10.0),
        radius: 0.5,
    }];
    let (counts, _, _) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let cols = hit_x_columns(&counts, &dims);
    // *fx→/fx : shifts right to columns 4-5
    assert!(
        !cols.contains(&5),
        "Mutation /fx would shift to col 5; got {:?}",
        cols
    );
}

// ================================================================
// GROUP 4: Index correctness (catches second-pass mutations)
// ================================================================

#[test]
fn golden_two_lights_separate_indices() {
    // Light 0 at (5, 0, 10) → columns 12-13 (16-col grid)
    // Light 1 at (-5, 0, 30) → column 6
    // Z-slices: L0 in iz=0, L1 in iz=1 — no overlap anywhere.
    let dims = ClusterDims { x: 16, y: 4, z: 4 };
    let lights = vec![
        CpuLight {
            pos: Vec3::new(5.0, 0.0, 10.0),
            radius: 0.5,
        },
        CpuLight {
            pos: Vec3::new(-5.0, 0.0, 30.0),
            radius: 0.5,
        },
    ];
    let (counts, indices, offsets) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);

    let total: u32 = counts.iter().sum();
    assert!(total >= 2, "Both lights should be binned, total={}", total);
    assert!(indices.contains(&0), "Light 0 must appear in indices");
    assert!(indices.contains(&1), "Light 1 must appear in indices");

    // Clusters in columns 12-13 should ONLY contain light 0.
    let mut found_light_0 = false;
    for iz in 0..dims.z {
        for iy in 0..dims.y {
            for ix in 12..=13u32 {
                let ci = (ix + iy * dims.x + iz * dims.x * dims.y) as usize;
                if counts[ci] > 0 {
                    found_light_0 = true;
                    let start = offsets[ci] as usize;
                    let end = start + counts[ci] as usize;
                    for &li in &indices[start..end] {
                        assert_eq!(
                            li, 0,
                            "Columns 12-13 should only contain light 0, found {}",
                            li,
                        );
                    }
                }
            }
        }
    }
    assert!(found_light_0, "Light 0 must have hits in columns 12-13");
}

#[test]
fn golden_index_validity_all_clusters() {
    let dims = ClusterDims { x: 16, y: 16, z: 4 };
    let lights = vec![
        CpuLight {
            pos: Vec3::new(3.0, 2.0, 10.0),
            radius: 1.0,
        },
        CpuLight {
            pos: Vec3::new(-4.0, -3.0, 20.0),
            radius: 2.0,
        },
        CpuLight {
            pos: Vec3::new(0.0, 0.0, 50.0),
            radius: 5.0,
        },
    ];
    let (counts, indices, offsets) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let n = (dims.x * dims.y * dims.z) as usize;

    for ci in 0..n {
        let start = offsets[ci] as usize;
        let count = counts[ci] as usize;
        let end = start + count;
        assert!(end <= indices.len(), "OOB for cluster {}", ci);
        for &li in &indices[start..end] {
            assert!(
                li < lights.len() as u32,
                "Bad light index {} in cluster {}",
                li,
                ci
            );
        }
    }
}

// ================================================================
// GROUP 5: Symmetry / cross-validation
// ================================================================

#[test]
fn golden_symmetric_lights_same_column_count() {
    let dims = ClusterDims { x: 16, y: 4, z: 4 };
    let left = vec![CpuLight {
        pos: Vec3::new(-3.0, 0.0, 15.0),
        radius: 1.0,
    }];
    let right = vec![CpuLight {
        pos: Vec3::new(3.0, 0.0, 15.0),
        radius: 1.0,
    }];
    let (c_l, _, _) = bin_lights_cpu(&left, dims, SCREEN, NEAR, FAR, FOV_Y);
    let (c_r, _, _) = bin_lights_cpu(&right, dims, SCREEN, NEAR, FAR, FOV_Y);
    let cols_l = hit_x_columns(&c_l, &dims);
    let cols_r = hit_x_columns(&c_r, &dims);
    assert_eq!(
        cols_l.len(),
        cols_r.len(),
        "Symmetric lights same column count: left {:?} vs right {:?}",
        cols_l,
        cols_r,
    );
}

#[test]
fn golden_center_light_symmetric_columns() {
    let dims = ClusterDims { x: 16, y: 4, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 10.0),
        radius: 1.0,
    }];
    let (counts, _, _) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let cols = hit_x_columns(&counts, &dims);
    // Center of 16 columns: pixel 320 / tile_w 40 = column 8.
    // A centered light should hit columns near 7-8.
    let min_col = *cols.iter().next().unwrap();
    let max_col = *cols.iter().last().unwrap();
    let center = (min_col + max_col) as f32 / 2.0;
    assert!(
        (center - 7.5).abs() < 1.5,
        "Center light should cluster near col 7-8, got {:?} (center={})",
        cols,
        center,
    );
}

// ================================================================
// GROUP 6: Pixel-space radius sensitivity
// ================================================================

#[test]
fn golden_radius_determines_column_spread() {
    let dims = ClusterDims { x: 16, y: 4, z: 4 };
    let small = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 10.0),
        radius: 0.5,
    }];
    let large = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 10.0),
        radius: 3.0,
    }];
    let (c_s, _, _) = bin_lights_cpu(&small, dims, SCREEN, NEAR, FAR, FOV_Y);
    let (c_l, _, _) = bin_lights_cpu(&large, dims, SCREEN, NEAR, FAR, FOV_Y);
    let cols_s = hit_x_columns(&c_s, &dims);
    let cols_l = hit_x_columns(&c_l, &dims);
    assert!(
        cols_l.len() > cols_s.len(),
        "Larger radius more columns: small={:?} large={:?}",
        cols_s,
        cols_l,
    );
}

#[test]
fn golden_radius_proportional_spread() {
    // Doubling radius doubles pixel spread → more columns.
    let dims = ClusterDims { x: 16, y: 4, z: 4 };
    let r2 = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 20.0),
        radius: 2.0,
    }];
    let r4 = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 20.0),
        radius: 4.0,
    }];
    let (c2, _, _) = bin_lights_cpu(&r2, dims, SCREEN, NEAR, FAR, FOV_Y);
    let (c4, _, _) = bin_lights_cpu(&r4, dims, SCREEN, NEAR, FAR, FOV_Y);
    let cols_r2 = hit_x_columns(&c2, &dims);
    let cols_r4 = hit_x_columns(&c4, &dims);
    assert!(
        cols_r4.len() >= cols_r2.len() + 1,
        "Double radius should add columns: r2={:?} r4={:?}",
        cols_r2,
        cols_r4,
    );
}

#[test]
fn golden_large_radius_precise_span() {
    // Light at (0, 0, 5), r=3: rpx_x ≈ 249 px
    // px=320, min_px≈71, max_px≈570 → ix 1..14
    // Mutation *0.5→+0.5 → rpx_x≈499, ix 0..15 (adds column 0)
    let dims = ClusterDims { x: 16, y: 4, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 5.0),
        radius: 3.0,
    }];
    let (counts, _, _) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let cols = hit_x_columns(&counts, &dims);
    assert!(
        cols.len() >= 10,
        "Large close light ≥10 columns, got {:?}",
        cols
    );
    let min_col = *cols.iter().next().unwrap();
    assert!(
        min_col >= 1,
        "Column 0 should not be hit with correct radius math, got {:?}",
        cols
    );
}

// ================================================================
// GROUP 7: Known-value z-slice assertions
// ================================================================

#[test]
fn golden_z_slice_near_plane_light() {
    // z=1, r=0.5: zmin=0.5, zmax=1.5
    // iz0 = ((0.4)/99.9)*4 ≈ 0.016 → 0
    // iz1 = ((1.4)/99.9)*4 ≈ 0.056 → 0
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 1.0),
        radius: 0.5,
    }];
    let (counts, _, _) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let slices = hit_z_slices(&counts, &dims);
    assert!(
        slices.contains(&0),
        "Near light should be in z-slice 0, got {:?}",
        slices
    );
    assert!(
        !slices.contains(&3),
        "Near light NOT in z-slice 3, got {:?}",
        slices
    );
}

#[test]
fn golden_z_slice_far_plane_light() {
    // z=90, r=5: zmin=85, zmax=95
    // iz0 = ((84.9)/99.9)*4 ≈ 3.40 → 3
    // iz1 = ((94.9)/99.9)*4 ≈ 3.80 → 3
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 90.0),
        radius: 5.0,
    }];
    let (counts, _, _) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let slices = hit_z_slices(&counts, &dims);
    assert!(
        slices.contains(&3),
        "Far light should be in z-slice 3, got {:?}",
        slices
    );
    assert!(
        !slices.contains(&0),
        "Far light NOT in z-slice 0, got {:?}",
        slices
    );
}

#[test]
fn golden_z_slice_mid_range_light() {
    // z=50, r=0.5: zmin=49.5, zmax=50.5 (NEAR=0.1, FAR=100, 4 z-slices).
    // LOGARITHMIC slicing (clustered.rs): iz = floor(ln(z/near)/ln(far/near) * 4)
    // iz0 = floor(ln(495)/ln(1000)*4) = floor(3.59) = 3
    // iz1 = floor(ln(505)/ln(1000)*4) = floor(3.60) = 3  -> slice 3
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 50.0),
        radius: 0.5,
    }];
    let (counts, _, _) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let slices = hit_z_slices(&counts, &dims);
    assert!(
        slices.contains(&3),
        "Mid-range light z=50 in log z-slice 3, got {:?}",
        slices,
    );
    assert!(!slices.contains(&0), "Not in z-slice 0; got {:?}", slices);
}

// ================================================================
// GROUP 8: Quadrant validation
// ================================================================

#[test]
fn golden_positive_x_in_right_half() {
    // Positive x projects to right side (columns ≥8 in 16-col grid).
    let dims = ClusterDims { x: 16, y: 4, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(3.0, 0.0, 10.0),
        radius: 0.5,
    }];
    let (counts, _, _) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let cols = hit_x_columns(&counts, &dims);
    let max_col = *cols.iter().last().unwrap();
    assert!(
        max_col >= 8,
        "Positive x light should be in right half (≥8), got {:?}",
        cols
    );
}

#[test]
fn golden_negative_x_in_left_half() {
    // Negative x projects to left side (columns ≤7 in 16-col grid).
    let dims = ClusterDims { x: 16, y: 4, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(-3.0, 0.0, 10.0),
        radius: 0.5,
    }];
    let (counts, _, _) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let cols = hit_x_columns(&counts, &dims);
    let min_col = *cols.iter().next().unwrap();
    assert!(
        min_col <= 7,
        "Negative x light should be in left half (≤7), got {:?}",
        cols
    );
}
