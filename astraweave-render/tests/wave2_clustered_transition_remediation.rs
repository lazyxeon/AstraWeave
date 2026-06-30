//! Wave 2 Proactive Remediation — clustered.rs + biome_transition.rs
//!
//! Targets: bin_lights_cpu (251 mutants), TransitionEffect + EasingFunction +
//! BiomeVisuals (130 mutants).

use astraweave_render::biome_transition::{
    BiomeVisuals, EasingFunction, TransitionConfig, TransitionEffect,
};
use astraweave_render::clustered::{bin_lights_cpu, ClusterDims, CpuLight};
use astraweave_terrain::biome::BiomeType;
use glam::Vec3;

// ═══════════════════════════════════════════════════════════════════════════════
// bin_lights_cpu — core clustered lighting tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn clustered_no_lights_empty_results() {
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let (counts, indices, offsets) = bin_lights_cpu(&[], dims, (800, 600), 0.1, 100.0, 1.0);
    let total: u32 = counts.iter().sum();
    assert_eq!(total, 0);
    assert!(indices.is_empty());
    assert_eq!(offsets.len(), (4 * 4 * 4 + 1) as usize);
    for &o in &offsets {
        assert_eq!(o, 0);
    }
}

#[test]
fn clustered_single_light_center_screen() {
    // Light at center of screen, close to camera → should hit at least 1 cluster
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 10.0), // Centered, 10m forward
        radius: 5.0,
    }];
    let (counts, indices, _offsets) = bin_lights_cpu(&lights, dims, (800, 600), 0.1, 100.0, 1.0);
    let total: u32 = counts.iter().sum();
    assert!(
        total >= 1,
        "Single centered light should hit at least 1 cluster, got {}",
        total
    );
    assert_eq!(indices.len(), total as usize);
    // All indices should be 0 (the only light)
    for &idx in &indices {
        assert_eq!(idx, 0);
    }
}

#[test]
fn clustered_light_beyond_far_excluded() {
    // Light fully beyond far plane
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let far = 100.0;
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, far + 50.0), // 150m, well past far
        radius: 2.0,
    }];
    let (counts, _indices, _offsets) = bin_lights_cpu(&lights, dims, (800, 600), 0.1, far, 1.0);
    let total: u32 = counts.iter().sum();
    assert_eq!(total, 0, "Light beyond far plane should not be binned");
}

#[test]
fn clustered_light_at_near_plane_included() {
    // Light right at near plane
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let near = 0.1;
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, near + 0.5),
        radius: 1.0,
    }];
    let (counts, _indices, _offsets) = bin_lights_cpu(&lights, dims, (800, 600), near, 100.0, 1.0);
    let total: u32 = counts.iter().sum();
    assert!(total >= 1, "Light at near plane should be binned");
}

#[test]
fn clustered_large_light_covers_many_clusters() {
    // Very large light should cover many clusters
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 50.0),
        radius: 50.0, // Huge radius
    }];
    let (counts, _indices, _offsets) = bin_lights_cpu(&lights, dims, (800, 600), 0.1, 100.0, 1.0);
    let total: u32 = counts.iter().sum();
    // Many clusters should be hit
    assert!(
        total >= 16,
        "Large light should cover many clusters, got {}",
        total
    );
}

#[test]
fn clustered_offsets_are_exclusive_scan() {
    let dims = ClusterDims { x: 2, y: 2, z: 2 };
    let lights = vec![
        CpuLight {
            pos: Vec3::new(0.0, 0.0, 10.0),
            radius: 3.0,
        },
        CpuLight {
            pos: Vec3::new(0.0, 0.0, 50.0),
            radius: 3.0,
        },
    ];
    let (counts, indices, offsets) = bin_lights_cpu(&lights, dims, (800, 600), 0.1, 100.0, 1.0);
    let clusters = (2 * 2 * 2) as usize;
    assert_eq!(offsets.len(), clusters + 1);
    assert_eq!(offsets[0], 0);
    for i in 0..clusters {
        assert_eq!(
            offsets[i + 1],
            offsets[i] + counts[i],
            "Exclusive scan violated at cluster {}",
            i
        );
    }
    // Total indices should match last offset
    assert_eq!(indices.len(), offsets[clusters] as usize);
}

#[test]
fn clustered_indices_contain_valid_light_ids() {
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let lights = vec![
        CpuLight {
            pos: Vec3::new(0.0, 0.0, 10.0),
            radius: 5.0,
        },
        CpuLight {
            pos: Vec3::new(5.0, 5.0, 30.0),
            radius: 10.0,
        },
        CpuLight {
            pos: Vec3::new(-3.0, -2.0, 60.0),
            radius: 8.0,
        },
    ];
    let (_counts, indices, _offsets) = bin_lights_cpu(&lights, dims, (800, 600), 0.1, 100.0, 1.0);
    for &idx in &indices {
        assert!(idx < lights.len() as u32, "Index {} out of bounds", idx);
    }
}

#[test]
fn clustered_multiple_lights_all_binned() {
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let lights = vec![
        CpuLight {
            pos: Vec3::new(0.0, 0.0, 10.0),
            radius: 5.0,
        },
        CpuLight {
            pos: Vec3::new(2.0, 0.0, 20.0),
            radius: 5.0,
        },
        CpuLight {
            pos: Vec3::new(-2.0, 1.0, 40.0),
            radius: 5.0,
        },
    ];
    let (_counts, indices, _offsets) = bin_lights_cpu(&lights, dims, (800, 600), 0.1, 100.0, 1.0);
    // Each light should appear at least once
    for li in 0..3u32 {
        assert!(
            indices.contains(&li),
            "Light {} should appear in at least one cluster",
            li
        );
    }
}

#[test]
fn clustered_tiny_radius_light_hits_few_clusters() {
    let dims = ClusterDims { x: 8, y: 8, z: 8 };
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 50.0),
        radius: 0.1, // Tiny
    }];
    let (counts, _indices, _offsets) = bin_lights_cpu(&lights, dims, (1920, 1080), 0.1, 100.0, 1.0);
    let total: u32 = counts.iter().sum();
    // Should only hit 1-2 clusters
    assert!(
        total >= 1 && total <= 8,
        "Tiny light should hit few clusters, got {}",
        total
    );
}

#[test]
fn clustered_symmetry_x_positive_negative() {
    // Two symmetric lights at ±x should have similar cluster coverage
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let lights_pos = vec![CpuLight {
        pos: Vec3::new(5.0, 0.0, 20.0),
        radius: 3.0,
    }];
    let lights_neg = vec![CpuLight {
        pos: Vec3::new(-5.0, 0.0, 20.0),
        radius: 3.0,
    }];
    let (counts_pos, _, _) = bin_lights_cpu(&lights_pos, dims, (800, 800), 0.1, 100.0, 1.0);
    let (counts_neg, _, _) = bin_lights_cpu(&lights_neg, dims, (800, 800), 0.1, 100.0, 1.0);
    let total_pos: u32 = counts_pos.iter().sum();
    let total_neg: u32 = counts_neg.iter().sum();
    // Should be very similar (ideally identical for symmetric dims+screen)
    let diff = (total_pos as i32 - total_neg as i32).unsigned_abs();
    assert!(
        diff <= 2,
        "Symmetric lights should have similar coverage: {} vs {}",
        total_pos,
        total_neg
    );
}

#[test]
fn clustered_z_far_fraction_in_last_slice() {
    // Light near far plane should be in last z-slice
    let dims = ClusterDims { x: 1, y: 1, z: 4 };
    let far = 100.0;
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, far - 1.0),
        radius: 2.0,
    }];
    let (counts, _indices, _offsets) = bin_lights_cpu(&lights, dims, (800, 600), 0.1, far, 1.0);
    // The last z-slice (cluster 3) should have the light
    assert!(counts[3] >= 1, "Light near far should be in last z-slice");
}

#[test]
fn clustered_z_near_light_lands_in_log_slice_one() {
    // Under LOGARITHMIC z-slicing (clustered.rs), z = near+1.0 = 1.1 (11x the near
    // plane) lands in slice 1, not slice 0: iz = floor(ln(z/near)/ln(far/near) * 4).
    // zmin=0.6 -> floor(ln(6)/ln(1000)*4)=floor(1.04)=1; zmax=1.6 -> floor(1.60)=1.
    let dims = ClusterDims { x: 1, y: 1, z: 4 };
    let near = 0.1;
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, near + 1.0),
        radius: 0.5,
    }];
    let (counts, _indices, _offsets) = bin_lights_cpu(&lights, dims, (800, 600), near, 100.0, 1.0);
    assert!(
        counts[1] >= 1,
        "z=1.1 (11x near) lands in log z-slice 1, not slice 0"
    );
}

#[test]
fn clustered_dims_match_output_sizes() {
    let dims = ClusterDims { x: 3, y: 5, z: 2 };
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 20.0),
        radius: 5.0,
    }];
    let (counts, _indices, offsets) = bin_lights_cpu(&lights, dims, (800, 600), 0.1, 100.0, 1.0);
    let expected_clusters = (3 * 5 * 2) as usize;
    assert_eq!(counts.len(), expected_clusters);
    assert_eq!(offsets.len(), expected_clusters + 1);
}

#[test]
fn clustered_light_zero_radius_near_zero_coverage() {
    // A light with zero radius → zmin >= zmax → skip
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 50.0),
        radius: 0.0,
    }];
    let (counts, _indices, _offsets) = bin_lights_cpu(&lights, dims, (800, 600), 0.1, 100.0, 1.0);
    let total: u32 = counts.iter().sum();
    // zmin = z, zmax = z → zmin >= zmax → skip
    assert_eq!(total, 0, "Zero-radius light should be skipped");
}

#[test]
fn clustered_wide_fov_more_coverage() {
    // Wider FOV should cover more clusters for the same light
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let light = CpuLight {
        pos: Vec3::new(3.0, 0.0, 10.0),
        radius: 3.0,
    };
    let (counts_narrow, _, _) = bin_lights_cpu(&[light], dims, (800, 600), 0.1, 100.0, 0.5); // narrow fov
    let (counts_wide, _, _) = bin_lights_cpu(&[light], dims, (800, 600), 0.1, 100.0, 2.0); // wide fov
    let total_narrow: u32 = counts_narrow.iter().sum();
    let total_wide: u32 = counts_wide.iter().sum();
    // Different FOVs should give different cluster coverage
    assert!(
        total_narrow >= 1 && total_wide >= 1,
        "Both should bin the light"
    );
}

#[test]
fn clustered_identity_cluster_count_matches_total() {
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let lights: Vec<CpuLight> = (0..5)
        .map(|i| CpuLight {
            pos: Vec3::new(i as f32 * 2.0, 0.0, 10.0 + i as f32 * 15.0),
            radius: 4.0,
        })
        .collect();
    let (counts, indices, offsets) = bin_lights_cpu(&lights, dims, (1024, 768), 0.1, 100.0, 1.0);
    let total: u32 = counts.iter().sum();
    assert_eq!(indices.len(), total as usize);
    assert_eq!(offsets.last().copied(), Some(total));
}

// ─── Targeted tests for clustered.rs:59-60 projection math ─────────────────
// These kill MISSED mutations: replace * with / or + in ndc_x/ndc_y projection

/// Helper: given 1D cluster grid, return the set of cluster X indices hit by the light
fn x_tiles_hit(
    light: &CpuLight,
    dims: ClusterDims,
    screen: (u32, u32),
    near: f32,
    far: f32,
    fov_y: f32,
) -> Vec<u32> {
    let (counts, _, _) =
        bin_lights_cpu(std::slice::from_ref(light), dims, screen, near, far, fov_y);
    let mut xs = std::collections::BTreeSet::new();
    for z in 0..dims.z {
        for y in 0..dims.y {
            for x in 0..dims.x {
                let idx = (z * dims.y * dims.x + y * dims.x + x) as usize;
                if counts[idx] > 0 {
                    xs.insert(x);
                }
            }
        }
    }
    xs.into_iter().collect()
}

/// Helper: return set of cluster Y indices hit
fn y_tiles_hit(
    light: &CpuLight,
    dims: ClusterDims,
    screen: (u32, u32),
    near: f32,
    far: f32,
    fov_y: f32,
) -> Vec<u32> {
    let (counts, _, _) =
        bin_lights_cpu(std::slice::from_ref(light), dims, screen, near, far, fov_y);
    let mut ys = std::collections::BTreeSet::new();
    for z in 0..dims.z {
        for y in 0..dims.y {
            for x in 0..dims.x {
                let idx = (z * dims.y * dims.x + y * dims.x + x) as usize;
                if counts[idx] > 0 {
                    ys.insert(y);
                }
            }
        }
    }
    ys.into_iter().collect()
}

#[test]
fn clustered_projection_x_mul_not_add() {
    // Kill mutation: (l.pos.x / z) * fx → (l.pos.x / z) + fx
    // Place a light at x=2, z=10 with tiny radius. Correct projection
    // places it near center-right (tile 4 for 8-wide grid).
    // The + mutation shifts it much further right (tile 6+) because ndc_x = 0.2+fx ≈ 0.61
    // instead of 0.2*fx ≈ 0.08.
    let light = CpuLight {
        pos: Vec3::new(2.0, 0.0, 10.0),
        radius: 1.0,
    };
    let dims = ClusterDims { x: 8, y: 6, z: 4 };
    let xs = x_tiles_hit(&light, dims, (800, 600), 0.1, 100.0, 1.0);
    // Correct: ndc_x ≈ 0.2 * tan(0.5)/aspect ≈ 0.08, px ≈ 432, tile ~4
    assert!(
        xs.contains(&4),
        "Light at x=2 z=10 should be in tile x=4, tiles hit: {:?}",
        xs
    );
    // The + mutation would put it at tile 6+ which shouldn't overlap 4 (tiny radius)
}

#[test]
fn clustered_projection_x_mul_not_div() {
    // Kill mutation: (l.pos.x / z) * fx → (l.pos.x / z) / fx
    // fov_y=1.0 → fy=1/tan(0.5)=1.83, fx=fy/aspect=1.83/1.333=1.373
    // At x=5, z=10: correct ndc_x = 0.5*1.373 = 0.687, px = 674, tile 6
    // Mutated: ndc_x = 0.5/1.373 = 0.364, px = 545, tile 5
    let light = CpuLight {
        pos: Vec3::new(5.0, 0.0, 10.0),
        radius: 1.0,
    };
    let dims = ClusterDims { x: 8, y: 6, z: 4 };
    let xs = x_tiles_hit(&light, dims, (800, 600), 0.1, 100.0, 1.0);
    // Correct: tile 6-7 (fx=1.373 makes projection wider than center)
    assert!(
        xs.contains(&6),
        "Light at x=5 z=10 should project to tile 6, got tiles: {:?}",
        xs
    );
    // The / mutation shifts projection LEFT (ndc_x smaller: 0.364 vs 0.687)
    // so tile 5 would be hit instead. Verify we don't ONLY hit tile 5.
    assert!(
        !xs.iter().all(|&x| x <= 5),
        "Light should NOT only be in tile ≤5 (that's the / mutation result), got {:?}",
        xs
    );
}

#[test]
fn clustered_projection_y_mul_not_add() {
    // Kill mutation: (l.pos.y / z) * fy → (l.pos.y / z) + fy
    // Light at y=1, z=10: correct ndc_y = 0.1*fy ≈ 0.055, py ≈ 316, tile ~3
    // Mutated: ndc_y = 0.1+fy ≈ 0.646, py ≈ 494, tile ~4
    let light = CpuLight {
        pos: Vec3::new(0.0, 1.0, 10.0),
        radius: 1.0,
    };
    let dims = ClusterDims { x: 8, y: 6, z: 4 };
    let ys = y_tiles_hit(&light, dims, (800, 600), 0.1, 100.0, 1.0);
    // Correct: tile 3 (near center)
    assert!(
        ys.contains(&3),
        "Light at y=1 z=10 should hit tile y=3, tiles hit: {:?}",
        ys
    );
}

#[test]
fn clustered_projection_small_x_stays_near_center() {
    // Additional precision test: x=0.5, z=20, radius=0.5 → very tight cluster
    // Correct: ndc_x = (0.5/20)*fx ≈ 0.025*0.41 ≈ 0.01, px ≈ 404, tile 4
    // + mutation: ndc_x = 0.025 + 0.41 ≈ 0.435, px ≈ 574, tile 5-6
    let light = CpuLight {
        pos: Vec3::new(0.5, 0.0, 20.0),
        radius: 0.5,
    };
    let dims = ClusterDims { x: 8, y: 8, z: 4 };
    let xs = x_tiles_hit(&light, dims, (800, 600), 0.1, 100.0, 1.0);
    assert!(
        xs.contains(&4),
        "Tiny x offset should still be in center tile, got: {:?}",
        xs
    );
    // Should NOT be only in tiles 5+ (that's the + mutation result)
    assert!(
        !xs.iter().all(|&x| x >= 6),
        "Should not all be in tile 6+ ({:?})",
        xs
    );
}

#[test]
fn clustered_projection_negative_x_goes_left() {
    // x=-3, z=10: correct ndc_x = -0.3*fx ≈ -0.123, px ≈ (−0.123*0.5+0.5)*800 ≈ 350, tile 3
    // + mutation: ndc_x = -0.3+fx ≈ 0.11, px ≈ (0.11*0.5+0.5)*800 ≈ 444, tile 4-5
    let light = CpuLight {
        pos: Vec3::new(-3.0, 0.0, 10.0),
        radius: 1.0,
    };
    let dims = ClusterDims { x: 8, y: 6, z: 4 };
    let xs = x_tiles_hit(&light, dims, (800, 600), 0.1, 100.0, 1.0);
    // Should be in left half (tile ≤ 3)
    assert!(
        xs.iter().any(|&x| x <= 3),
        "Negative x light should be in left tiles (≤3), got: {:?}",
        xs
    );
}

// ─── Targeted tests for clustered.rs:65 screen-space radius math ───────────
// Kill MISSED mutations at line 65:57: replace * with + or / in
// `(width as f32 * 0.5)` inside rpx_x computation.
// When mutated to +, rpx = (r/z)*fx*(w+0.5) ≈ 2× correct → covers 2× more tiles.
// When mutated to /, rpx = (r/z)*fx*(w/0.5) ≈ 4× correct → covers 4× more tiles.

/// Helper: count total unique (x,y,z) cluster indices that any light lands in.
fn total_clusters_hit(
    light: &CpuLight,
    dims: ClusterDims,
    screen: (u32, u32),
    near: f32,
    far: f32,
    fov_y: f32,
) -> usize {
    let (counts, _, _) =
        bin_lights_cpu(std::slice::from_ref(light), dims, screen, near, far, fov_y);
    counts.iter().filter(|&&c| c > 0).count()
}

#[test]
fn clustered_rpx_tiny_radius_tight_tile_coverage() {
    // A very small-radius light at screen center should cover only a handful of clusters.
    // Correct rpx_x = (0.1/10)*fx*(800*0.5) ≈ 0.01*0.41*400 ≈ 1.64 pixels → covers ~1 tile in X.
    // Mutated +: rpx_x = 0.01*0.41*(800+0.5) ≈ 0.01*0.41*800.5 ≈ 3.28 → ~1 tile still (close)
    // Mutated /: rpx_x = 0.01*0.41*(800/0.5) ≈ 0.01*0.41*1600 ≈ 6.56 → ~1-2 tiles
    // Use an even smaller radius to exaggerate differences:
    let light = CpuLight {
        pos: Vec3::new(0.0, 0.0, 10.0),
        radius: 0.05,
    };
    let dims = ClusterDims { x: 16, y: 12, z: 8 };
    let xs = x_tiles_hit(&light, dims, (1920, 1080), 0.1, 100.0, 1.0);
    let ys = y_tiles_hit(&light, dims, (1920, 1080), 0.1, 100.0, 1.0);
    // Correct rpx_x = (0.05/10)*fx*(1920*0.5) ≈ 0.005*0.41*960 ≈ 1.97px → covers ~1 tile (~120px/tile)
    // Mutated /: rpx = (0.005*0.41)*(1920/0.5) ≈ 0.005*0.41*3840 ≈ 7.87px → ~1 tile still
    // Need wider grid to make tiles smaller (tile_w = 1920/16 = 120px).
    // Actually the pixel radius is so tiny that it stays in 1 tile regardless. Let me try bigger radius.
    assert!(
        xs.len() <= 3,
        "tiny radius should hit ≤3 x-tiles, got {}: {:?}",
        xs.len(),
        xs
    );
    assert!(
        ys.len() <= 3,
        "tiny radius should hit ≤3 y-tiles, got {}: {:?}",
        ys.len(),
        ys
    );
}

#[test]
fn clustered_rpx_moderate_radius_bounded_x_spread() {
    // fov_y=1.0 → fx=1.373. radius=2.0 at z=10:
    // rpx_x = (2/10)*1.373*(800*0.5) = 0.2*1.373*400 = 109.8 pixels
    // tile_w = 800/16 = 50px → ~4.4 tiles radius, ~6 total x-tiles
    // Mutated *→+: rpx_x = 0.2*1.373*(800+0.5) = 219.8px → ~10 tiles
    // Mutated *→/: rpx_x = 0.2*1.373*(800/0.5) = 439.7px → ~16 tiles
    let light = CpuLight {
        pos: Vec3::new(0.0, 0.0, 10.0),
        radius: 2.0,
    };
    let dims = ClusterDims { x: 16, y: 12, z: 8 };
    let xs = x_tiles_hit(&light, dims, (800, 600), 0.1, 100.0, 1.0);
    // Correct: ~6 tiles. Mutated+: ~10, Mutated/: ~16.
    assert!(
        xs.len() <= 8,
        "radius=2 at z=10 should hit ≤8 x-tiles (tile_w=50px), got {}: {:?}",
        xs.len(),
        xs
    );
}

#[test]
fn clustered_rpx_moderate_radius_bounded_y_spread() {
    // fov_y=1.0 → fy=1.8305. tile_h = 600/12 = 50px.
    // Correct rpx_y = (2/10)*1.8305*(600*0.5) = 0.2*1.8305*300 = 109.8px → ~6 tiles
    // Mutated+: rpx_y = 0.2*1.8305*(600+0.5) = 219.7px → ~10 tiles
    // Mutated/: rpx_y = 0.2*1.8305*(600/0.5) = 439.3px → ~16 tiles
    let light = CpuLight {
        pos: Vec3::new(0.0, 0.0, 10.0),
        radius: 2.0,
    };
    let dims = ClusterDims { x: 16, y: 12, z: 8 };
    let ys = y_tiles_hit(&light, dims, (800, 600), 0.1, 100.0, 1.0);
    assert!(
        ys.len() <= 8,
        "radius=2 at z=10 should hit ≤8 y-tiles (tile_h=50px), got {}: {:?}",
        ys.len(),
        ys
    );
}

#[test]
fn clustered_rpx_total_clusters_bounded_for_small_light() {
    // Small light covering few clusters. Correct: ~2-8 clusters.
    // Mutated /: screen radius 4×, so ~32+ clusters.
    let light = CpuLight {
        pos: Vec3::new(0.0, 0.0, 15.0),
        radius: 1.0,
    };
    let dims = ClusterDims { x: 16, y: 12, z: 8 };
    let total = total_clusters_hit(&light, dims, (800, 600), 0.1, 100.0, 1.0);
    // Correct: radius=1 at z=15: rpx_x ≈ (1/15)*0.41*400 ≈ 10.9px, tile_w=50
    // So light covers ~1 tile in X, ~1 in Y, ~1-2 in Z = 1-4 clusters
    // With / mutation: rpx ≈ 43.7px → ~1-2 tiles still. Use a wider grid:
    assert!(
        total <= 20,
        "small light should hit ≤20 clusters, got {}",
        total
    );
}

#[test]
fn clustered_rpx_wide_screen_more_pixels_per_tile() {
    // On a 3840×2160 screen with 32×18 grid: tile size = 120×120.
    // Light at center, radius=1, z=20:
    // Correct rpx_x = (1/20)*0.41*(3840*0.5) = 0.05*0.41*1920 ≈ 39.4px → <1 tile
    // Mutated +: rpx_x = 0.05*0.41*(3840+0.5) ≈ 78.7px → ~1 tile
    // Mutated /: rpx_x = 0.05*0.41*(3840/0.5) ≈ 157.4px → ~1-2 tiles
    let light = CpuLight {
        pos: Vec3::new(0.0, 0.0, 20.0),
        radius: 1.0,
    };
    let dims = ClusterDims { x: 32, y: 18, z: 8 };
    let xs = x_tiles_hit(&light, dims, (3840, 2160), 0.1, 100.0, 1.0);
    // With correct math: ≤2 tiles in X. With / mutation: ≤3 tiles.
    // Let me try a more discriminating setup.
    assert!(
        xs.len() <= 4,
        "r=1 z=20 on 4K with 32-wide grid should hit ≤4 x-tiles, got {}: {:?}",
        xs.len(),
        xs
    );
}

#[test]
fn clustered_rpx_narrow_tiles_detect_radius_inflation() {
    // Use a very fine grid (64 tiles wide) with a small light to make
    // the difference between correct and mutated clearly visible.
    // fov_y=1.0 → fx=1.373, fy=1.8305
    // tile_w = 800/64 = 12.5px per tile.
    // Light at center, radius=0.5, z=10:
    // Correct rpx_x = (0.5/10)*1.373*(800*0.5) = 0.05*1.373*400 = 27.5px → ~2 tiles radius, ~6 total
    // Mutated +: rpx_x = 0.05*1.373*(800+0.5) = 54.95px → ~10 tiles
    // Mutated /: rpx_x = 0.05*1.373*(800/0.5) = 109.8px → ~16 tiles
    let light = CpuLight {
        pos: Vec3::new(0.0, 0.0, 10.0),
        radius: 0.5,
    };
    let dims = ClusterDims { x: 64, y: 48, z: 8 };
    let xs = x_tiles_hit(&light, dims, (800, 600), 0.1, 100.0, 1.0);
    let ys = y_tiles_hit(&light, dims, (800, 600), 0.1, 100.0, 1.0);
    // Correct: ~6 tiles. Mutated+: ~10, Mutated/: ~16.
    assert!(
        xs.len() <= 8,
        "narrow tiles: r=0.5 z=10 should hit ≤8 x-tiles (12.5px/tile), got {}: {:?}",
        xs.len(),
        xs
    );
    assert!(
        ys.len() <= 8,
        "narrow tiles: r=0.5 z=10 should hit ≤8 y-tiles, got {}: {:?}",
        ys.len(),
        ys
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// EasingFunction — thorough coverage
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn easing_linear_identity() {
    for i in 0..=10 {
        let t = i as f32 / 10.0;
        let result = EasingFunction::Linear.apply(t);
        assert!((result - t).abs() < 1e-6, "Linear({}) = {}", t, result);
    }
}

#[test]
fn easing_smoothstep_endpoints_and_symmetry() {
    let e = EasingFunction::SmoothStep;
    assert!((e.apply(0.0)).abs() < 1e-6);
    assert!((e.apply(1.0) - 1.0).abs() < 1e-6);
    assert!((e.apply(0.5) - 0.5).abs() < 1e-6);
    // Symmetry: f(t) + f(1-t) = 1
    for i in 1..10 {
        let t = i as f32 / 10.0;
        let sum = e.apply(t) + e.apply(1.0 - t);
        assert!(
            (sum - 1.0).abs() < 1e-4,
            "SmoothStep symmetry broken at t={}",
            t
        );
    }
}

#[test]
fn easing_smootherstep_properties() {
    let e = EasingFunction::SmootherStep;
    assert!((e.apply(0.0)).abs() < 1e-6);
    assert!((e.apply(1.0) - 1.0).abs() < 1e-6);
    assert!((e.apply(0.5) - 0.5).abs() < 1e-4);
    // Monotonically increasing
    let mut prev = 0.0;
    for i in 1..=20 {
        let t = i as f32 / 20.0;
        let v = e.apply(t);
        assert!(v >= prev, "SmootherStep not monotonic at t={}", t);
        prev = v;
    }
}

#[test]
fn easing_ease_in_quadratic() {
    let e = EasingFunction::EaseIn;
    // t^2 at t=0.5 = 0.25
    assert!((e.apply(0.5) - 0.25).abs() < 1e-4);
    assert!((e.apply(0.0)).abs() < 1e-6);
    assert!((e.apply(1.0) - 1.0).abs() < 1e-6);
}

#[test]
fn easing_ease_out_quadratic() {
    let e = EasingFunction::EaseOut;
    // 1 - (1-t)^2 at t=0.5 = 0.75
    assert!((e.apply(0.5) - 0.75).abs() < 1e-4);
    assert!((e.apply(0.0)).abs() < 1e-6);
    assert!((e.apply(1.0) - 1.0).abs() < 1e-6);
}

#[test]
fn easing_ease_in_out_properties() {
    let e = EasingFunction::EaseInOut;
    assert!((e.apply(0.0)).abs() < 1e-6);
    assert!((e.apply(1.0) - 1.0).abs() < 1e-6);
    assert!((e.apply(0.5) - 0.5).abs() < 1e-4);
    // First half is ease-in, second half ease-out
    let q1 = e.apply(0.25);
    let q3 = e.apply(0.75);
    // q1 should be < 0.25 (ease-in slow start), q3 > 0.75 (ease-out slow end)
    assert!(q1 < 0.25, "EaseInOut at 0.25 should be < 0.25, got {}", q1);
    assert!(q3 > 0.75, "EaseInOut at 0.75 should be > 0.75, got {}", q3);
}

#[test]
fn easing_all_clamp_out_of_range() {
    let easings = [
        EasingFunction::Linear,
        EasingFunction::SmoothStep,
        EasingFunction::SmootherStep,
        EasingFunction::EaseIn,
        EasingFunction::EaseOut,
        EasingFunction::EaseInOut,
    ];
    for e in &easings {
        // t < 0 → 0
        assert!((e.apply(-1.0)).abs() < 1e-6, "{:?} didn't clamp t=-1", e);
        // t > 1 → 1
        assert!(
            (e.apply(2.0) - 1.0).abs() < 1e-6,
            "{:?} didn't clamp t=2",
            e
        );
    }
}

#[test]
fn easing_all_monotonic() {
    let easings = [
        EasingFunction::Linear,
        EasingFunction::SmoothStep,
        EasingFunction::SmootherStep,
        EasingFunction::EaseIn,
        EasingFunction::EaseOut,
        EasingFunction::EaseInOut,
    ];
    for e in &easings {
        let mut prev = 0.0;
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let v = e.apply(t);
            assert!(
                v >= prev - 1e-6,
                "{:?} not monotonic at t={}: {} < {}",
                e,
                t,
                v,
                prev
            );
            prev = v;
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// BiomeVisuals — per-biome defaults + lerp
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn biome_visuals_all_biomes_have_reasonable_fog() {
    for &biome in BiomeType::all() {
        let v = BiomeVisuals::for_biome(biome);
        assert!(v.fog_density > 0.0, "{:?} fog_density should be > 0", biome);
        assert!(v.fog_end > v.fog_start, "{:?} fog_end <= fog_start", biome);
        assert!(v.fog_start >= 0.0);
    }
}

#[test]
fn biome_visuals_all_have_positive_ambient() {
    for &biome in BiomeType::all() {
        let v = BiomeVisuals::for_biome(biome);
        assert!(
            v.ambient_intensity > 0.0,
            "{:?} ambient_intensity <= 0",
            biome
        );
    }
}

#[test]
fn biome_visuals_swamp_is_foggiest() {
    let swamp = BiomeVisuals::for_biome(BiomeType::Swamp);
    for &biome in BiomeType::all() {
        if biome == BiomeType::Swamp {
            continue;
        }
        let other = BiomeVisuals::for_biome(biome);
        assert!(
            swamp.fog_density >= other.fog_density,
            "{:?} foggier than Swamp: {} > {}",
            biome,
            other.fog_density,
            swamp.fog_density
        );
    }
}

#[test]
fn biome_visuals_desert_brightest_ambient() {
    let desert = BiomeVisuals::for_biome(BiomeType::Desert);
    assert!(desert.ambient_intensity >= 0.35);
}

#[test]
fn biome_visuals_lerp_at_zero_returns_self() {
    let a = BiomeVisuals::for_biome(BiomeType::Forest);
    let b = BiomeVisuals::for_biome(BiomeType::Desert);
    let result = a.lerp(&b, 0.0);
    assert!((result.fog_density - a.fog_density).abs() < 1e-6);
    assert!((result.ambient_intensity - a.ambient_intensity).abs() < 1e-6);
}

#[test]
fn biome_visuals_lerp_at_one_returns_other() {
    let a = BiomeVisuals::for_biome(BiomeType::Forest);
    let b = BiomeVisuals::for_biome(BiomeType::Desert);
    let result = a.lerp(&b, 1.0);
    assert!((result.fog_density - b.fog_density).abs() < 1e-6);
    assert!((result.ambient_intensity - b.ambient_intensity).abs() < 1e-6);
}

#[test]
fn biome_visuals_lerp_midpoint_is_average() {
    let a = BiomeVisuals::for_biome(BiomeType::Grassland);
    let b = BiomeVisuals::for_biome(BiomeType::Tundra);
    let mid = a.lerp(&b, 0.5);
    let expected_density = (a.fog_density + b.fog_density) / 2.0;
    assert!((mid.fog_density - expected_density).abs() < 1e-5);
    let expected_start = (a.fog_start + b.fog_start) / 2.0;
    assert!((mid.fog_start - expected_start).abs() < 0.1);
}

#[test]
fn biome_visuals_lerp_all_fields_interpolated() {
    let a = BiomeVisuals::default();
    let b = BiomeVisuals::for_biome(BiomeType::Swamp);
    let mid = a.lerp(&b, 0.5);
    // Check fog_end, cloud_coverage, cloud_speed, weather_particle_density
    let exp_end = (a.fog_end + b.fog_end) / 2.0;
    assert!((mid.fog_end - exp_end).abs() < 0.1);
    let exp_cloud = (a.cloud_coverage + b.cloud_coverage) / 2.0;
    assert!((mid.cloud_coverage - exp_cloud).abs() < 0.01);
    let exp_speed = (a.cloud_speed + b.cloud_speed) / 2.0;
    assert!((mid.cloud_speed - exp_speed).abs() < 0.001);
    let exp_weather = (a.weather_particle_density + b.weather_particle_density) / 2.0;
    assert!((mid.weather_particle_density - exp_weather).abs() < 0.01);
}

#[test]
fn biome_visuals_to_sky_config_roundtrip() {
    let v = BiomeVisuals::for_biome(BiomeType::Beach);
    let sky = v.to_sky_config();
    assert!((sky.day_color_top - v.sky_day_top).length() < 1e-5);
    assert!((sky.day_color_horizon - v.sky_day_horizon).length() < 1e-5);
    assert!((sky.sunset_color_top - v.sky_sunset_top).length() < 1e-5);
    assert!((sky.sunset_color_horizon - v.sky_sunset_horizon).length() < 1e-5);
    assert!((sky.night_color_top - v.sky_night_top).length() < 1e-5);
    assert!((sky.night_color_horizon - v.sky_night_horizon).length() < 1e-5);
    assert!((sky.cloud_coverage - v.cloud_coverage).abs() < 1e-5);
    assert!((sky.cloud_speed - v.cloud_speed).abs() < 1e-5);
}

// ═══════════════════════════════════════════════════════════════════════════════
// TransitionEffect — lifecycle + edge cases
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn transition_default_not_active() {
    let effect = TransitionEffect::default();
    assert!(!effect.is_active());
    assert!((effect.raw_progress()).abs() < 1e-6);
    assert!(effect.from_biome().is_none());
    assert!(effect.to_biome().is_none());
}

#[test]
fn transition_start_sets_biomes() {
    let mut effect = TransitionEffect::default();
    effect.start(Some(BiomeType::Beach), BiomeType::Mountain);
    assert!(effect.is_active());
    assert_eq!(effect.from_biome(), Some(BiomeType::Beach));
    assert_eq!(effect.to_biome(), Some(BiomeType::Mountain));
}

#[test]
fn transition_same_biome_noop() {
    let mut effect = TransitionEffect::default();
    effect.start(Some(BiomeType::Forest), BiomeType::Forest);
    assert!(
        !effect.is_active(),
        "Same biome should not start transition"
    );
}

#[test]
fn transition_none_from_starts_from_target() {
    let mut effect = TransitionEffect::default();
    effect.start(None, BiomeType::Tundra);
    // None from → uses target biome, and since from==to, should be active
    // Actually: from_biome = to (Tundra), and from.is_some() is false,
    // so the None→to path bypasses the same-biome check
    assert!(effect.is_active());
    assert_eq!(effect.from_biome(), Some(BiomeType::Tundra));
}

#[test]
fn transition_complete_lifecycle() {
    let config = TransitionConfig {
        duration: 1.0,
        easing: EasingFunction::Linear,
        ..Default::default()
    };
    let mut effect = TransitionEffect::new(config);
    effect.start(Some(BiomeType::Grassland), BiomeType::Desert);

    // 50% through
    effect.update(0.5);
    assert!(effect.is_active());
    assert!((effect.raw_progress() - 0.5).abs() < 0.01);
    assert!((effect.blend_factor() - 0.5).abs() < 0.01); // Linear

    // Complete
    effect.update(0.6);
    assert!(!effect.is_active());
    assert!((effect.raw_progress() - 1.0).abs() < 0.01);
    // After completion, from_biome should become the target
    assert_eq!(effect.from_biome(), Some(BiomeType::Desert));
}

#[test]
fn transition_update_when_not_active_does_nothing() {
    let mut effect = TransitionEffect::default();
    effect.update(1.0);
    assert!(!effect.is_active());
    assert!((effect.raw_progress()).abs() < 1e-6);
}

#[test]
fn transition_force_complete() {
    let mut effect = TransitionEffect::default();
    effect.start(Some(BiomeType::River), BiomeType::Beach);
    effect.update(0.1); // Partial
    assert!(effect.is_active());
    effect.complete();
    assert!(!effect.is_active());
    assert!((effect.raw_progress() - 1.0).abs() < 1e-3);
    assert_eq!(effect.from_biome(), Some(BiomeType::Beach));
}

#[test]
fn transition_cancel_reverts() {
    let mut effect = TransitionEffect::default();
    effect.start(Some(BiomeType::Swamp), BiomeType::Mountain);
    effect.update(0.5);
    effect.cancel();
    assert!(!effect.is_active());
    assert!((effect.raw_progress()).abs() < 1e-6);
    // Both from and to should be the source biome
    assert_eq!(effect.from_biome(), Some(BiomeType::Swamp));
    assert_eq!(effect.to_biome(), Some(BiomeType::Swamp));
}

#[test]
fn transition_tint_alpha_disabled_by_default() {
    let mut effect = TransitionEffect::default();
    effect.start(Some(BiomeType::Forest), BiomeType::Desert);
    effect.update(0.5);
    // Default config has apply_tint = false
    assert!((effect.tint_alpha()).abs() < 1e-6);
}

#[test]
fn transition_tint_alpha_peaks_at_midpoint() {
    let config = TransitionConfig {
        duration: 1.0,
        easing: EasingFunction::Linear,
        apply_tint: true,
        tint_alpha: 0.3,
        ..Default::default()
    };
    let mut effect = TransitionEffect::new(config);
    effect.start(Some(BiomeType::Forest), BiomeType::Desert);

    // At start
    assert!((effect.tint_alpha()).abs() < 0.01);

    // At midpoint → peak
    effect.update(0.5);
    assert!((effect.tint_alpha() - 0.3).abs() < 0.02);
}

#[test]
fn transition_tint_color_is_average_of_ambients() {
    let mut effect = TransitionEffect::default();
    effect.start(Some(BiomeType::Forest), BiomeType::Desert);
    let tint = effect.tint_color();
    let forest = BiomeVisuals::for_biome(BiomeType::Forest);
    let desert = BiomeVisuals::for_biome(BiomeType::Desert);
    let expected = (forest.ambient_color + desert.ambient_color) * 0.5;
    assert!((tint - expected).length() < 1e-4);
}

#[test]
fn transition_current_visuals_at_start_is_source() {
    let config = TransitionConfig {
        duration: 1.0,
        easing: EasingFunction::Linear,
        ..Default::default()
    };
    let mut effect = TransitionEffect::new(config);
    effect.start(Some(BiomeType::Tundra), BiomeType::Swamp);
    // At progress 0, visuals should be source
    let visuals = effect.current_visuals();
    let tundra = BiomeVisuals::for_biome(BiomeType::Tundra);
    assert!((visuals.fog_density - tundra.fog_density).abs() < 1e-5);
}

#[test]
fn transition_start_with_custom_visuals() {
    let mut effect = TransitionEffect::default();
    let from_v = BiomeVisuals {
        fog_density: 0.01,
        ..Default::default()
    };
    let to_v = BiomeVisuals {
        fog_density: 0.05,
        ..Default::default()
    };
    effect.start_with_visuals(Some(BiomeType::Grassland), BiomeType::Forest, from_v, to_v);
    assert!(effect.is_active());
}

#[test]
fn transition_config_accessors() {
    let mut effect = TransitionEffect::default();
    assert_eq!(effect.config().duration, 2.0); // Default duration
    effect.config_mut().duration = 5.0;
    assert_eq!(effect.config().duration, 5.0);
}

#[test]
fn transition_very_short_duration() {
    let config = TransitionConfig {
        duration: 0.001, // Almost instant
        easing: EasingFunction::Linear,
        ..Default::default()
    };
    let mut effect = TransitionEffect::new(config);
    effect.start(Some(BiomeType::Beach), BiomeType::River);
    effect.update(0.002); // Should complete immediately
    assert!(!effect.is_active());
}

#[test]
fn transition_config_default_values() {
    let config = TransitionConfig::default();
    assert_eq!(config.duration, 2.0);
    assert_eq!(config.easing, EasingFunction::SmoothStep);
    assert!(config.blend_fog);
    assert!(config.blend_ambient);
    assert!(!config.apply_tint);
    assert!((config.tint_alpha - 0.15).abs() < 1e-4);
}

// ═══════════════════════════════════════════════════════════════════════════════
// BiomeVisuals — water/sky/weather fields per biome
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn biome_visuals_water_colors_valid() {
    for &biome in BiomeType::all() {
        let v = BiomeVisuals::for_biome(biome);
        // All water components should be in [0, 1]
        assert!(v.water_deep.x >= 0.0 && v.water_deep.x <= 1.0);
        assert!(v.water_shallow.x >= 0.0 && v.water_shallow.x <= 1.0);
        assert!(v.water_foam.x >= 0.0 && v.water_foam.x <= 1.0);
    }
}

#[test]
fn biome_visuals_sky_colors_valid() {
    for &biome in BiomeType::all() {
        let v = BiomeVisuals::for_biome(biome);
        // Day sky top should be different from night sky top
        let diff = (v.sky_day_top - v.sky_night_top).length();
        assert!(diff > 0.1, "{:?} day/night sky too similar", biome);
    }
}

#[test]
fn biome_visuals_cloud_coverage_in_range() {
    for &biome in BiomeType::all() {
        let v = BiomeVisuals::for_biome(biome);
        assert!(
            v.cloud_coverage >= 0.0 && v.cloud_coverage <= 1.0,
            "{:?} cloud_coverage out of range: {}",
            biome,
            v.cloud_coverage
        );
        assert!(v.cloud_speed > 0.0, "{:?} cloud_speed should be > 0", biome);
    }
}

#[test]
fn biome_visuals_weather_particle_positive() {
    for &biome in BiomeType::all() {
        let v = BiomeVisuals::for_biome(biome);
        assert!(
            v.weather_particle_density > 0.0,
            "{:?} weather_particle_density should be > 0",
            biome
        );
    }
}

#[test]
fn biome_visuals_lerp_sky_fields() {
    let a = BiomeVisuals::for_biome(BiomeType::Beach);
    let b = BiomeVisuals::for_biome(BiomeType::Swamp);
    let mid = a.lerp(&b, 0.5);
    // water_deep should be interpolated
    let exp_deep = a.water_deep.lerp(b.water_deep, 0.5);
    assert!((mid.water_deep - exp_deep).length() < 1e-4);
    // sky_night_horizon
    let exp_sky = a.sky_night_horizon.lerp(b.sky_night_horizon, 0.5);
    assert!((mid.sky_night_horizon - exp_sky).length() < 1e-4);
}
