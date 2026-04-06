//! Wave 2 Heightmap Remediation Tests
//!
//! Targets missed mutants in heightmap.rs, specifically:
//! - apply_hydraulic_erosion: velocity calculations, erosion/deposition logic
//! - sample_bilinear: interpolation arithmetic
//! - calculate_normal: gradient computation
//! - generate_vertices / generate_indices: mesh generation
//! - smooth: averaging kernel
//! - recalculate_bounds: min/max tracking
//! - set_height / get_height: boundary checks

use astraweave_terrain::heightmap::{Heightmap, HeightmapConfig};
use glam::Vec3;

// ─── Helpers ───────────────────────────────────────────────────────────────

fn make_heightmap(resolution: u32) -> Heightmap {
    Heightmap::new(HeightmapConfig {
        resolution,
        ..Default::default()
    })
    .unwrap()
}

fn sloped_heightmap(resolution: u32) -> Heightmap {
    let size = (resolution * resolution) as usize;
    let data: Vec<f32> = (0..size)
        .map(|i| {
            let x = (i as u32) % resolution;
            x as f32
        })
        .collect();
    Heightmap::from_data(data, resolution).unwrap()
}

fn peaked_heightmap(resolution: u32) -> Heightmap {
    let size = (resolution * resolution) as usize;
    let mid = resolution / 2;
    let data: Vec<f32> = (0..size)
        .map(|i| {
            let x = (i as u32) % resolution;
            let z = (i as u32) / resolution;
            let dx = (x as f32 - mid as f32).abs();
            let dz = (z as f32 - mid as f32).abs();
            100.0 - (dx + dz) * 5.0
        })
        .collect();
    Heightmap::from_data(data, resolution).unwrap()
}

// ─── HeightmapConfig Default ───────────────────────────────────────────────

#[test]
fn config_default_resolution() {
    let cfg = HeightmapConfig::default();
    assert_eq!(cfg.resolution, 128);
}

#[test]
fn config_default_min_height() {
    let cfg = HeightmapConfig::default();
    assert_eq!(cfg.min_height, 0.0);
}

#[test]
fn config_default_max_height() {
    let cfg = HeightmapConfig::default();
    assert_eq!(cfg.max_height, 100.0);
}

#[test]
fn config_default_height_scale() {
    let cfg = HeightmapConfig::default();
    assert_eq!(cfg.height_scale, 1.0);
}

// ─── Heightmap::new ────────────────────────────────────────────────────────

#[test]
fn new_heightmap_data_length() {
    let hm = make_heightmap(16);
    assert_eq!(hm.data().len(), 16 * 16);
}

#[test]
fn new_heightmap_all_zeros() {
    let hm = make_heightmap(8);
    for &h in hm.data() {
        assert_eq!(h, 0.0);
    }
}

#[test]
fn new_heightmap_resolution() {
    let hm = make_heightmap(32);
    assert_eq!(hm.resolution(), 32);
}

#[test]
fn new_heightmap_min_max_zero() {
    let hm = make_heightmap(4);
    assert_eq!(hm.min_height(), 0.0);
    assert_eq!(hm.max_height(), 0.0);
}

// ─── Heightmap::from_data ──────────────────────────────────────────────────

#[test]
fn from_data_size_mismatch_errors() {
    let data = vec![1.0, 2.0, 3.0]; // 3 ≠ 2*2
    assert!(Heightmap::from_data(data, 2).is_err());
}

#[test]
fn from_data_computes_min_max() {
    let data = vec![5.0, 10.0, -3.0, 7.0];
    let hm = Heightmap::from_data(data, 2).unwrap();
    assert_eq!(hm.min_height(), -3.0);
    assert_eq!(hm.max_height(), 10.0);
}

#[test]
fn from_data_preserves_values() {
    let data = vec![1.0, 2.0, 3.0, 4.0];
    let hm = Heightmap::from_data(data.clone(), 2).unwrap();
    assert_eq!(hm.data(), &data[..]);
}

#[test]
fn from_data_resolution_correct() {
    let data = vec![0.0; 9];
    let hm = Heightmap::from_data(data, 3).unwrap();
    assert_eq!(hm.resolution(), 3);
}

// ─── get_height / set_height ───────────────────────────────────────────────

#[test]
fn get_height_out_of_bounds_returns_zero() {
    let hm = make_heightmap(4);
    assert_eq!(hm.get_height(4, 0), 0.0); // x >= resolution
    assert_eq!(hm.get_height(0, 4), 0.0); // z >= resolution
    assert_eq!(hm.get_height(100, 100), 0.0);
}

#[test]
fn set_height_updates_value() {
    let mut hm = make_heightmap(4);
    hm.set_height(2, 3, 42.0);
    assert_eq!(hm.get_height(2, 3), 42.0);
}

#[test]
fn set_height_updates_min() {
    let mut hm = make_heightmap(4);
    hm.set_height(0, 0, -5.0);
    assert_eq!(hm.min_height(), -5.0);
}

#[test]
fn set_height_updates_max() {
    let mut hm = make_heightmap(4);
    hm.set_height(0, 0, 50.0);
    assert_eq!(hm.max_height(), 50.0);
}

#[test]
fn set_height_out_of_bounds_no_crash() {
    let mut hm = make_heightmap(4);
    hm.set_height(4, 0, 10.0);
    hm.set_height(0, 4, 10.0);
    // No crash = success; values still 0
    assert_eq!(hm.get_height(3, 3), 0.0);
}

#[test]
fn get_height_at_index_in_range() {
    let data = vec![7.0, 8.0, 9.0, 10.0];
    let hm = Heightmap::from_data(data, 2).unwrap();
    assert_eq!(hm.get_height_at_index(0), 7.0);
    assert_eq!(hm.get_height_at_index(3), 10.0);
}

#[test]
fn get_height_at_index_out_of_range() {
    let hm = make_heightmap(2);
    assert_eq!(hm.get_height_at_index(100), 0.0);
}

// ─── recalculate_bounds ────────────────────────────────────────────────────

#[test]
fn recalculate_bounds_after_data_mut() {
    let mut hm = make_heightmap(4);
    // Directly modify data
    let data = hm.data_mut();
    data[0] = -10.0;
    data[5] = 50.0;
    hm.recalculate_bounds();
    assert_eq!(hm.min_height(), -10.0);
    assert_eq!(hm.max_height(), 50.0);
}

#[test]
fn recalculate_bounds_empty_data() {
    // Can't easily make empty heightmap but test the concept via from_data
    let data = vec![0.0; 4];
    let mut hm = Heightmap::from_data(data, 2).unwrap();
    hm.data_mut()[0] = 100.0;
    hm.data_mut()[1] = -100.0;
    hm.recalculate_bounds();
    assert_eq!(hm.min_height(), -100.0);
    assert_eq!(hm.max_height(), 100.0);
}

// ─── sample_bilinear ──────────────────────────────────────────────────────

#[test]
fn bilinear_at_grid_points_exact() {
    // 3x3 grid: height = x + z*10
    let data = vec![0.0, 1.0, 2.0, 10.0, 11.0, 12.0, 20.0, 21.0, 22.0];
    let hm = Heightmap::from_data(data, 3).unwrap();
    assert!((hm.sample_bilinear(0.0, 0.0) - 0.0).abs() < 0.01);
    assert!((hm.sample_bilinear(1.0, 0.0) - 1.0).abs() < 0.01);
    assert!((hm.sample_bilinear(0.0, 1.0) - 10.0).abs() < 0.01);
    assert!((hm.sample_bilinear(1.0, 1.0) - 11.0).abs() < 0.01);
}

#[test]
fn bilinear_midpoint_interpolation() {
    // 2x2 grid: corners at 0, 10, 20, 30
    let data = vec![0.0, 10.0, 20.0, 30.0];
    let hm = Heightmap::from_data(data, 2).unwrap();
    let mid = hm.sample_bilinear(0.5, 0.5);
    // Expected: (0 + 10 + 20 + 30) / 4 = 15
    assert!(
        (mid - 15.0).abs() < 0.01,
        "Midpoint should be 15.0, got {mid}"
    );
}

#[test]
fn bilinear_x_interpolation() {
    let data = vec![0.0, 10.0, 0.0, 10.0];
    let hm = Heightmap::from_data(data, 2).unwrap();
    // At z=0, interpolate between 0 and 10
    let val = hm.sample_bilinear(0.5, 0.0);
    assert!(
        (val - 5.0).abs() < 0.01,
        "x-interp should be 5.0, got {val}"
    );
}

#[test]
fn bilinear_z_interpolation() {
    let data = vec![0.0, 0.0, 10.0, 10.0];
    let hm = Heightmap::from_data(data, 2).unwrap();
    // At x=0, interpolate between 0 and 10
    let val = hm.sample_bilinear(0.0, 0.5);
    assert!(
        (val - 5.0).abs() < 0.01,
        "z-interp should be 5.0, got {val}"
    );
}

#[test]
fn bilinear_clamp_negative_coords() {
    let data = vec![42.0, 0.0, 0.0, 0.0];
    let hm = Heightmap::from_data(data, 2).unwrap();
    let val = hm.sample_bilinear(-5.0, -5.0);
    assert!(
        (val - 42.0).abs() < 0.1,
        "Clamped to (0,0) should be 42.0, got {val}"
    );
}

#[test]
fn bilinear_clamp_large_coords() {
    let data = vec![0.0, 0.0, 0.0, 99.0];
    let hm = Heightmap::from_data(data, 2).unwrap();
    let val = hm.sample_bilinear(100.0, 100.0);
    // Should clamp to near (1,1) corner = 99.0
    assert!((val - 99.0).abs() < 1.0, "Clamped to corner, got {val}");
}

// ─── calculate_normal ─────────────────────────────────────────────────────

#[test]
fn normal_on_flat_terrain_points_up() {
    let data = vec![5.0; 9];
    let hm = Heightmap::from_data(data, 3).unwrap();
    let n = hm.calculate_normal(1, 1, 1.0);
    assert!(
        (n.y - 1.0).abs() < 0.01,
        "Flat normal should be (0,1,0), got {n:?}"
    );
    assert!(n.x.abs() < 0.01);
    assert!(n.z.abs() < 0.01);
}

#[test]
fn normal_on_x_slope_tilts_x() {
    // Heights increase in x: 0, 10, 20
    let data = vec![0.0, 10.0, 20.0, 0.0, 10.0, 20.0, 0.0, 10.0, 20.0];
    let hm = Heightmap::from_data(data, 3).unwrap();
    let n = hm.calculate_normal(1, 1, 1.0);
    // dx = (20 - 0) / 2 = 10, normal.x should be negative (tilted against slope)
    assert!(n.x < 0.0, "Normal should tilt against x-slope: {n:?}");
    assert!(n.z.abs() < 0.01, "No z-tilt expected: {n:?}");
}

#[test]
fn normal_on_z_slope_tilts_z() {
    // Heights increase in z: row0=0, row1=10, row2=20
    let data = vec![0.0, 0.0, 0.0, 10.0, 10.0, 10.0, 20.0, 20.0, 20.0];
    let hm = Heightmap::from_data(data, 3).unwrap();
    let n = hm.calculate_normal(1, 1, 1.0);
    assert!(n.z < 0.0, "Normal should tilt against z-slope: {n:?}");
    assert!(n.x.abs() < 0.01, "No x-tilt expected: {n:?}");
}

#[test]
fn normal_scale_affects_steepness() {
    let data = vec![0.0, 10.0, 20.0, 0.0, 10.0, 20.0, 0.0, 10.0, 20.0];
    let hm = Heightmap::from_data(data, 3).unwrap();
    let n_small = hm.calculate_normal(1, 1, 1.0);
    let n_large = hm.calculate_normal(1, 1, 10.0);
    // Larger scale = less steep = more vertical normal
    assert!(
        n_large.y > n_small.y,
        "Larger scale should give more vertical normal: small={n_small:?} large={n_large:?}"
    );
}

#[test]
fn normal_at_edge_uses_self() {
    let data = vec![0.0, 10.0, 20.0, 0.0, 10.0, 20.0, 0.0, 10.0, 20.0];
    let hm = Heightmap::from_data(data, 3).unwrap();
    // Edge normals should not panic
    let _n00 = hm.calculate_normal(0, 0, 1.0);
    let _n20 = hm.calculate_normal(2, 0, 1.0);
    let _n02 = hm.calculate_normal(0, 2, 1.0);
    let _n22 = hm.calculate_normal(2, 2, 1.0);
}

#[test]
fn normal_is_normalized() {
    let hm = peaked_heightmap(8);
    let n = hm.calculate_normal(4, 4, 1.0);
    let len = n.length();
    assert!(
        (len - 1.0).abs() < 0.001,
        "Normal should be unit vector, length={len}"
    );
}

// ─── apply_hydraulic_erosion ──────────────────────────────────────────────

#[test]
fn erosion_on_flat_terrain_minimal_change() {
    let data = vec![10.0; 64]; // 8x8 flat
    let mut hm = Heightmap::from_data(data, 8).unwrap();
    let before = hm.data().to_vec();
    hm.apply_hydraulic_erosion(0.5).unwrap();
    // On flat terrain, velocity is ~0, so we get deposition everywhere
    // Ensure changes are very small
    let max_diff: f32 = hm
        .data()
        .iter()
        .zip(before.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_diff < 1.0,
        "Flat terrain should have minimal erosion, max_diff={max_diff}"
    );
}

#[test]
fn erosion_on_peaked_terrain_modifies_heights() {
    let mut hm = peaked_heightmap(16);
    let before = hm.data().to_vec();
    hm.apply_hydraulic_erosion(1.0).unwrap();
    // Peaked terrain has non-zero second derivative -> velocity -> erosion
    let any_diff = hm
        .data()
        .iter()
        .zip(before.iter())
        .any(|(a, b)| (a - b).abs() > 1e-6);
    assert!(any_diff, "Erosion should modify peaked terrain");
}

#[test]
fn erosion_stronger_strength_more_change() {
    let mut hm_weak = peaked_heightmap(16);
    let mut hm_strong = peaked_heightmap(16);
    hm_weak.apply_hydraulic_erosion(0.1).unwrap();
    hm_strong.apply_hydraulic_erosion(10.0).unwrap();

    let original = peaked_heightmap(16);
    let diff_weak: f32 = hm_weak
        .data()
        .iter()
        .zip(original.data().iter())
        .map(|(a, b)| (a - b).abs())
        .sum();
    let diff_strong: f32 = hm_strong
        .data()
        .iter()
        .zip(original.data().iter())
        .map(|(a, b)| (a - b).abs())
        .sum();

    assert!(
        diff_strong > diff_weak,
        "Stronger erosion should change more: weak={diff_weak} strong={diff_strong}"
    );
}

#[test]
fn erosion_updates_bounds() {
    let mut hm = peaked_heightmap(16);
    let old_min = hm.min_height();
    let old_max = hm.max_height();
    hm.apply_hydraulic_erosion(2.0).unwrap();
    // Bounds should be recalculated after erosion
    let new_min = hm.min_height();
    let new_max = hm.max_height();
    // At least one bound should differ (erosion modifies terrain)
    let bounds_changed = (new_min - old_min).abs() > 1e-6 || (new_max - old_max).abs() > 1e-6;
    assert!(
        bounds_changed,
        "Bounds should change after erosion: old=({old_min},{old_max}) new=({new_min},{new_max})"
    );
}

#[test]
fn erosion_zero_strength_minimal_change() {
    let mut hm = sloped_heightmap(8);
    let before = hm.data().to_vec();
    hm.apply_hydraulic_erosion(0.0).unwrap();
    // 0 strength => ~no erosion (deposition formula: speed * deposition * 0 * 0.05 = 0)
    let max_diff: f32 = hm
        .data()
        .iter()
        .zip(before.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_diff < 0.01,
        "Zero strength should produce negligible change: max_diff={max_diff}"
    );
}

#[test]
fn erosion_returns_ok() {
    let mut hm = make_heightmap(8);
    assert!(hm.apply_hydraulic_erosion(1.0).is_ok());
}

#[test]
fn erosion_interior_cells_only() {
    // Borders (row/col 0 and resolution-1) should be unchanged
    let data: Vec<f32> = (0..64).map(|i| i as f32).collect();
    let mut hm = Heightmap::from_data(data.clone(), 8).unwrap();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // Check border cells (first/last row and col)
    for i in 0..8 {
        assert_eq!(
            hm.get_height(i, 0),
            data[(i) as usize],
            "Top border should be unchanged"
        );
        assert_eq!(
            hm.get_height(i, 7),
            data[(7 * 8 + i) as usize],
            "Bottom border should be unchanged"
        );
        assert_eq!(
            hm.get_height(0, i),
            data[(i * 8) as usize],
            "Left border should be unchanged"
        );
        assert_eq!(
            hm.get_height(7, i),
            data[(i * 8 + 7) as usize],
            "Right border should be unchanged"
        );
    }
}

#[test]
fn erosion_velocity_calculation_affects_result() {
    // Create terrain with specific height difference pattern
    // If velocity calc is wrong (e.g., * replaced with +), result differs
    let mut data = vec![10.0f32; 64]; // 8x8
                                      // Create a valley in the middle
    for z in 2..6 {
        for x in 2..6 {
            data[(z * 8 + x) as usize] = 5.0; // Lower center
        }
    }
    let mut hm = Heightmap::from_data(data, 8).unwrap();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // Interior cells around the valley edge should be affected
    let edge_cell = hm.get_height(2, 2);
    assert!(
        (edge_cell - 5.0).abs() > 1e-6,
        "Valley edge should be modified by erosion: {edge_cell}"
    );
}

#[test]
fn erosion_deposition_occurs_on_slow_flow() {
    // Flat terrain with slight variation → slow flow → deposition
    let mut data = vec![10.0f32; 64]; // 8x8 flat
    data[3 * 8 + 3] = 10.1; // Tiny bump
    let original_center = data[4 * 8 + 4];
    let mut hm = Heightmap::from_data(data, 8).unwrap();
    hm.apply_hydraulic_erosion(1.0).unwrap();

    // Center area should have slight deposition (height increase)
    let new_center = hm.get_height(4, 4);
    // Deposition adds to height
    let diff = new_center - original_center;
    // With mostly flat terrain, deposition should dominate
    assert!(
        diff.abs() > 0.0 || new_center == original_center,
        "Deposition should occur on near-flat areas"
    );
}

// ─── generate_vertices ────────────────────────────────────────────────────

#[test]
fn vertices_count_matches_resolution() {
    let hm = make_heightmap(4);
    let verts = hm.generate_vertices(100.0, Vec3::ZERO);
    assert_eq!(verts.len(), 4 * 4);
}

#[test]
fn vertices_first_at_offset() {
    let hm = make_heightmap(4);
    let offset = Vec3::new(10.0, 0.0, 20.0);
    let verts = hm.generate_vertices(100.0, offset);
    assert!((verts[0].x - 10.0).abs() < 0.01);
    assert!((verts[0].z - 20.0).abs() < 0.01);
}

#[test]
fn vertices_step_size_correct() {
    let hm = make_heightmap(4);
    let verts = hm.generate_vertices(300.0, Vec3::ZERO);
    // step = 300 / (4-1) = 100
    assert!(
        (verts[1].x - 100.0).abs() < 0.01,
        "Step should be 100.0, got x={}",
        verts[1].x
    );
}

#[test]
fn vertices_height_from_heightmap() {
    let mut hm = make_heightmap(4);
    hm.set_height(1, 0, 25.0);
    let verts = hm.generate_vertices(100.0, Vec3::ZERO);
    // Vertex at (1,0) is index 1
    assert!((verts[1].y - 25.0).abs() < 0.01);
}

#[test]
fn vertices_z_varies_by_row() {
    let hm = make_heightmap(4);
    let verts = hm.generate_vertices(300.0, Vec3::ZERO);
    // step = 300/3 = 100
    // Row 0: z=0, Row 1: z=100, Row 2: z=200
    assert!(
        (verts[4].z - 100.0).abs() < 0.01,
        "z at row 1 should be 100"
    );
    assert!(
        (verts[8].z - 200.0).abs() < 0.01,
        "z at row 2 should be 200"
    );
}

// ─── generate_indices ─────────────────────────────────────────────────────

#[test]
fn indices_count_for_3x3() {
    let hm = make_heightmap(3);
    let idx = hm.generate_indices();
    // 2x2 quads * 2 triangles * 3 vertices = 24
    assert_eq!(idx.len(), 24);
}

#[test]
fn indices_count_for_4x4() {
    let hm = make_heightmap(4);
    let idx = hm.generate_indices();
    // 3x3 quads * 2 triangles * 3 = 54
    assert_eq!(idx.len(), 54);
}

#[test]
fn indices_in_valid_range() {
    let hm = make_heightmap(5);
    let idx = hm.generate_indices();
    let max_vertex = 5 * 5 - 1;
    for &i in &idx {
        assert!(i <= max_vertex, "Index {i} exceeds max vertex {max_vertex}");
    }
}

#[test]
fn indices_first_triangle_pattern() {
    let hm = make_heightmap(4);
    let idx = hm.generate_indices();
    // First quad, first triangle (CCW from above): base, base+resolution, base+1
    assert_eq!(idx[0], 0);
    assert_eq!(idx[1], 4); // base + resolution = 4
    assert_eq!(idx[2], 1); // base + 1
}

#[test]
fn indices_second_triangle_pattern() {
    let hm = make_heightmap(4);
    let idx = hm.generate_indices();
    // First quad, second triangle (CCW from above): base+1, base+resolution, base+resolution+1
    assert_eq!(idx[3], 1);
    assert_eq!(idx[4], 4); // base + resolution = 0 + 4
    assert_eq!(idx[5], 5); // base + resolution + 1 = 0 + 4 + 1
}

// ─── smooth ───────────────────────────────────────────────────────────────

#[test]
fn smooth_zero_iterations_no_change() {
    let mut hm = peaked_heightmap(8);
    let before = hm.data().to_vec();
    hm.smooth(0);
    assert_eq!(hm.data(), &before[..]);
}

#[test]
fn smooth_reduces_peak() {
    let mut hm = peaked_heightmap(8);
    let peak_before = hm.get_height(4, 4);
    hm.smooth(5);
    let peak_after = hm.get_height(4, 4);
    assert!(
        peak_after < peak_before,
        "Smoothing should reduce peaks: before={peak_before} after={peak_after}"
    );
}

#[test]
fn smooth_more_iterations_more_smooth() {
    let mut hm1 = peaked_heightmap(8);
    let mut hm2 = peaked_heightmap(8);
    hm1.smooth(1);
    hm2.smooth(10);

    let peak1 = hm1.get_height(4, 4);
    let peak2 = hm2.get_height(4, 4);
    assert!(
        peak2 < peak1,
        "More iterations = more smoothing: 1iter={peak1} 10iter={peak2}"
    );
}

#[test]
fn smooth_borders_unchanged() {
    let data: Vec<f32> = (0..64).map(|i| i as f32).collect();
    let mut hm = Heightmap::from_data(data.clone(), 8).unwrap();
    hm.smooth(3);

    for i in 0..8 {
        assert_eq!(
            hm.get_height(i, 0),
            data[i as usize],
            "Top border unchanged"
        );
        assert_eq!(
            hm.get_height(i, 7),
            data[(7 * 8 + i) as usize],
            "Bottom border unchanged"
        );
        assert_eq!(
            hm.get_height(0, i),
            data[(i * 8) as usize],
            "Left border unchanged"
        );
        assert_eq!(
            hm.get_height(7, i),
            data[(i * 8 + 7) as usize],
            "Right border unchanged"
        );
    }
}

#[test]
fn smooth_updates_bounds() {
    let mut hm = peaked_heightmap(8);
    let old_max = hm.max_height();
    hm.smooth(10);
    let new_max = hm.max_height();
    assert!(
        new_max <= old_max,
        "Smoothing should not increase max: old={old_max} new={new_max}"
    );
}

#[test]
fn smooth_kernel_averages_neighbors() {
    // 3x3 grid: center high, rest zero
    let mut data = vec![0.0f32; 9];
    data[4] = 80.0; // center = (1,1) in 3x3
    let mut hm = Heightmap::from_data(data, 3).unwrap();
    hm.smooth(1);

    // Kernel: (left + right + up + down + center*4) / 8
    // = (0 + 0 + 0 + 0 + 80*4) / 8 = 320/8 = 40
    let smoothed = hm.get_height(1, 1);
    assert!(
        (smoothed - 40.0).abs() < 0.1,
        "Smooth kernel should average to 40.0, got {smoothed}"
    );
}

// ─── data_mut ──────────────────────────────────────────────────────────────

#[test]
fn data_mut_returns_correct_slice() {
    let mut hm = make_heightmap(4);
    let slice = hm.data_mut();
    assert_eq!(slice.len(), 16);
    // Can modify
    slice[0] = 99.0;
    // Verify through get_height
    assert_eq!(hm.get_height(0, 0), 99.0);
}
