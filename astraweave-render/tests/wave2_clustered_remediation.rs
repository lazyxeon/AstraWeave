//! Wave 2 Proactive Remediation: clustered.rs (251 mutants)
//!
//! Tests for CPU light binning: bin_lights_cpu invariants, edge cases,
//! and golden-value cluster assignments.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::clustered::{bin_lights_cpu, ClusterDims, CpuLight};
use glam::Vec3;

const DIMS: ClusterDims = ClusterDims { x: 4, y: 4, z: 4 };
const SCREEN: (u32, u32) = (640, 480);
const NEAR: f32 = 0.1;
const FAR: f32 = 100.0;
const FOV_Y: f32 = std::f32::consts::FRAC_PI_3; // 60°

// ============================================================================
// Basic invariants
// ============================================================================

#[test]
fn empty_lights_all_zero() {
    let (counts, indices, offsets) = bin_lights_cpu(&[], DIMS, SCREEN, NEAR, FAR, FOV_Y);
    let total_clusters = (DIMS.x * DIMS.y * DIMS.z) as usize;
    assert_eq!(counts.len(), total_clusters);
    assert!(counts.iter().all(|&c| c == 0));
    assert!(indices.is_empty());
    assert_eq!(offsets.len(), total_clusters + 1);
    assert!(offsets.iter().all(|&o| o == 0));
}

#[test]
fn offsets_are_exclusive_prefix_sum() {
    let lights = vec![
        CpuLight {
            pos: Vec3::new(0.0, 0.0, 10.0),
            radius: 5.0,
        },
        CpuLight {
            pos: Vec3::new(3.0, 3.0, 20.0),
            radius: 3.0,
        },
    ];
    let (counts, _indices, offsets) = bin_lights_cpu(&lights, DIMS, SCREEN, NEAR, FAR, FOV_Y);

    // offsets[0] must be 0
    assert_eq!(offsets[0], 0);

    // offsets[i+1] = offsets[i] + counts[i]
    for i in 0..counts.len() {
        assert_eq!(
            offsets[i + 1],
            offsets[i] + counts[i],
            "Prefix sum mismatch at cluster {}",
            i
        );
    }

    // Total indices = last offset
    let total: u32 = counts.iter().sum();
    assert_eq!(offsets[counts.len()], total);
}

#[test]
fn total_indices_equals_sum_of_counts() {
    let lights = vec![
        CpuLight {
            pos: Vec3::new(0.0, 0.0, 5.0),
            radius: 2.0,
        },
        CpuLight {
            pos: Vec3::new(-2.0, 1.0, 15.0),
            radius: 4.0,
        },
        CpuLight {
            pos: Vec3::new(1.0, -1.0, 50.0),
            radius: 10.0,
        },
    ];
    let (counts, indices, _offsets) = bin_lights_cpu(&lights, DIMS, SCREEN, NEAR, FAR, FOV_Y);
    let total: u32 = counts.iter().sum();
    assert_eq!(indices.len(), total as usize);
}

// ============================================================================
// Single light placement
// ============================================================================

#[test]
fn single_light_at_center_hits_at_least_one_cluster() {
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 10.0),
        radius: 1.0,
    }];
    let (counts, _indices, _offsets) = bin_lights_cpu(&lights, DIMS, SCREEN, NEAR, FAR, FOV_Y);
    let total: u32 = counts.iter().sum();
    assert!(
        total >= 1,
        "Center light should hit at least 1 cluster, got {}",
        total
    );
}

#[test]
fn large_radius_light_hits_many_clusters() {
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 10.0),
        radius: 50.0, // Very large
    }];
    let (counts, _indices, _offsets) = bin_lights_cpu(&lights, DIMS, SCREEN, NEAR, FAR, FOV_Y);
    let hit_clusters: usize = counts.iter().filter(|&&c| c > 0).count();
    let total_clusters = (DIMS.x * DIMS.y * DIMS.z) as usize;
    // A massive light should cover most/all clusters
    assert!(
        hit_clusters > total_clusters / 2,
        "Large light should hit >50% of clusters, got {}/{}",
        hit_clusters,
        total_clusters
    );
}

#[test]
fn small_radius_light_hits_few_clusters() {
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 50.0), // Far away
        radius: 0.5,                    // Small
    }];
    let (counts, _indices, _offsets) = bin_lights_cpu(&lights, DIMS, SCREEN, NEAR, FAR, FOV_Y);
    let hit_clusters: usize = counts.iter().filter(|&&c| c > 0).count();
    // Small distant light should only hit a few clusters
    assert!(
        hit_clusters <= 8,
        "Small distant light should hit ≤8 clusters, got {}",
        hit_clusters
    );
}

// ============================================================================
// Rejection / boundary cases
// ============================================================================

#[test]
fn light_beyond_far_plane_is_skipped() {
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 200.0), // Way beyond far=100
        radius: 2.0,
    }];
    let (counts, indices, _offsets) = bin_lights_cpu(&lights, DIMS, SCREEN, NEAR, FAR, FOV_Y);
    let total: u32 = counts.iter().sum();
    assert_eq!(total, 0, "Light beyond far plane should be skipped");
    assert!(indices.is_empty());
}

#[test]
fn light_at_far_boundary_may_be_included() {
    // z - radius > far → skipped. z + radius = 102, z - radius = 98 < 100 → included
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 100.0),
        radius: 2.0,
    }];
    let (counts, _indices, _offsets) = bin_lights_cpu(&lights, DIMS, SCREEN, NEAR, FAR, FOV_Y);
    let total: u32 = counts.iter().sum();
    // z=100 - radius=2 = 98 < far=100, so should be included
    assert!(
        total >= 1,
        "Light at far boundary with radius reaching inside should be included"
    );
}

#[test]
fn light_just_beyond_far_plus_radius() {
    // z - radius > far → skipped
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 103.0),
        radius: 2.0, // z - r = 101 > 100
    }];
    let (counts, _indices, _offsets) = bin_lights_cpu(&lights, DIMS, SCREEN, NEAR, FAR, FOV_Y);
    let total: u32 = counts.iter().sum();
    assert_eq!(
        total, 0,
        "Light with z-radius > far should be fully rejected"
    );
}

#[test]
fn light_near_plane_included() {
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 0.5),
        radius: 1.0,
    }];
    let (counts, _indices, _offsets) = bin_lights_cpu(&lights, DIMS, SCREEN, NEAR, FAR, FOV_Y);
    let total: u32 = counts.iter().sum();
    assert!(total >= 1, "Light near the near plane should be included");
}

// ============================================================================
// Multiple lights — indices contain correct light IDs
// ============================================================================

#[test]
fn indices_contain_correct_light_ids() {
    let lights = vec![
        CpuLight {
            pos: Vec3::new(0.0, 0.0, 10.0),
            radius: 1.0,
        },
        CpuLight {
            pos: Vec3::new(5.0, 5.0, 30.0),
            radius: 2.0,
        },
    ];
    let (_counts, indices, _offsets) = bin_lights_cpu(&lights, DIMS, SCREEN, NEAR, FAR, FOV_Y);

    // All indices should be valid light IDs (0 or 1)
    for &idx in &indices {
        assert!(idx < lights.len() as u32, "Index {} out of range", idx);
    }
}

#[test]
fn each_light_appears_in_indices() {
    let lights = vec![
        CpuLight {
            pos: Vec3::new(0.0, 0.0, 10.0),
            radius: 5.0,
        },
        CpuLight {
            pos: Vec3::new(-3.0, 2.0, 20.0),
            radius: 3.0,
        },
        CpuLight {
            pos: Vec3::new(2.0, -1.0, 40.0),
            radius: 8.0,
        },
    ];
    let (_counts, indices, _offsets) = bin_lights_cpu(&lights, DIMS, SCREEN, NEAR, FAR, FOV_Y);

    // Each light should appear at least once
    for li in 0..lights.len() {
        assert!(
            indices.contains(&(li as u32)),
            "Light {} should appear in indices",
            li
        );
    }
}

// ============================================================================
// Different cluster dimensions
// ============================================================================

#[test]
fn single_cluster_captures_everything() {
    let dims = ClusterDims { x: 1, y: 1, z: 1 };
    let lights = vec![
        CpuLight {
            pos: Vec3::new(0.0, 0.0, 10.0),
            radius: 1.0,
        },
        CpuLight {
            pos: Vec3::new(3.0, -2.0, 50.0),
            radius: 5.0,
        },
    ];
    let (counts, indices, _offsets) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    assert_eq!(counts.len(), 1);
    // Both lights should be in cluster 0
    assert_eq!(counts[0] as usize, indices.len());
    assert!(indices.contains(&0));
    assert!(indices.contains(&1));
}

#[test]
fn high_resolution_clusters_still_work() {
    let dims = ClusterDims {
        x: 16,
        y: 16,
        z: 16,
    };
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 10.0),
        radius: 3.0,
    }];
    let (counts, indices, offsets) = bin_lights_cpu(&lights, dims, SCREEN, NEAR, FAR, FOV_Y);
    let total_clusters = (dims.x * dims.y * dims.z) as usize;
    assert_eq!(counts.len(), total_clusters);
    assert_eq!(offsets.len(), total_clusters + 1);
    let total: u32 = counts.iter().sum();
    assert_eq!(indices.len(), total as usize);
    assert!(total >= 1);
}

// ============================================================================
// Determinism
// ============================================================================

#[test]
fn binning_is_deterministic() {
    let lights = vec![
        CpuLight {
            pos: Vec3::new(1.0, 2.0, 15.0),
            radius: 3.0,
        },
        CpuLight {
            pos: Vec3::new(-4.0, 0.5, 30.0),
            radius: 5.0,
        },
    ];
    let (c1, i1, o1) = bin_lights_cpu(&lights, DIMS, SCREEN, NEAR, FAR, FOV_Y);
    let (c2, i2, o2) = bin_lights_cpu(&lights, DIMS, SCREEN, NEAR, FAR, FOV_Y);
    assert_eq!(c1, c2);
    assert_eq!(i1, i2);
    assert_eq!(o1, o2);
}

// ============================================================================
// Screen size edge cases
// ============================================================================

#[test]
fn zero_screen_size_doesnt_panic() {
    // width.max(1) and height.max(1) protect against zero
    let lights = vec![CpuLight {
        pos: Vec3::new(0.0, 0.0, 10.0),
        radius: 1.0,
    }];
    let (counts, _indices, _offsets) = bin_lights_cpu(&lights, DIMS, (0, 0), NEAR, FAR, FOV_Y);
    // Should not panic; counts is valid
    assert_eq!(counts.len(), (DIMS.x * DIMS.y * DIMS.z) as usize);
}
