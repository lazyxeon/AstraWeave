//! Wave 2 Mutation Remediation Tests for astraweave-terrain
//!
//! These tests target specific arithmetic and logical mutations that
//! survive the existing test suite. Each test pins down exact computed
//! values to kill mutations in:
//! - heightmap.rs: bilinear interpolation, normals, smooth, vertex/index gen
//! - erosion.rs: thermal erosion arithmetic, neighbor iteration
//! - climate.rs: Whittaker classification boundaries, fBm accumulation
//! - biome_blending.rs: PackedBiomeBlend normalization, dominant biome
//! - texture_splatting.rs: SplatRule::evaluate, TriplanarWeights, SplatWeights
//! - lib.rs: WorldConfig defaults, WorldGenerator pipeline

use astraweave_terrain::*;
use glam::{Vec2, Vec3};

// ============================================================================
// HEIGHTMAP: Bilinear Interpolation — pin down fractional-coordinate math
// ============================================================================

#[test]
fn bilinear_interp_quarter_x() {
    // 3×3 grid, sample at u=0.25, v=0.0
    // row0: [0, 10, 20]  → h00=0, h10=10, fx=0.25
    // h0 = 0*(1-0.25) + 10*0.25 = 2.5
    let data = vec![0.0, 10.0, 20.0, 0.0, 10.0, 20.0, 0.0, 10.0, 20.0];
    let hm = Heightmap::from_data(data, 3).unwrap();
    let v = hm.sample_bilinear(0.25, 0.0);
    assert!((v - 2.5).abs() < 1e-4, "Expected 2.5, got {v}");
}

#[test]
fn bilinear_interp_quarter_z() {
    // 3×3 grid, sample at u=0.0, v=0.25
    // col0: [0, 100, 200]  → h00=0, h01=100, fz=0.25
    // h0 = 0, h1 = 100 → result = 0*(1-0.25) + 100*0.25 = 25.0
    let data = vec![0.0, 0.0, 0.0, 100.0, 100.0, 100.0, 200.0, 200.0, 200.0];
    let hm = Heightmap::from_data(data, 3).unwrap();
    let v = hm.sample_bilinear(0.0, 0.25);
    assert!((v - 25.0).abs() < 1e-4, "Expected 25.0, got {v}");
}

#[test]
fn bilinear_interp_0_75_0_75() {
    // 3×3 grid: row-major
    //  [10, 20, 30]
    //  [40, 50, 60]
    //  [70, 80, 90]
    // sample_bilinear(0.75, 0.75):
    //   x=0.75, z=0.75 → x0=0,x1=1,z0=0,z1=1, fx=0.75, fz=0.75
    //   h00=10, h10=20, h01=40, h11=50
    //   h0 = 10*(1-0.75) + 20*0.75 = 2.5+15 = 17.5
    //   h1 = 40*0.25 + 50*0.75 = 10+37.5 = 47.5
    //   result = 17.5*0.25 + 47.5*0.75 = 4.375+35.625 = 40.0
    let data = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0];
    let hm = Heightmap::from_data(data, 3).unwrap();
    let v = hm.sample_bilinear(0.75, 0.75);
    assert!((v - 40.0).abs() < 1e-3, "Expected 40.0, got {v}");
}

#[test]
fn bilinear_interp_asymmetric_grid() {
    // 3×3 grid with distinct corner values
    //  [1, 2, 3]
    //  [4, 5, 6]
    //  [7, 8, 9]
    // At (0.5, 0.5): h00=1,h10=2,h01=4,h11=5
    //   h0 = 1*0.5 + 2*0.5 = 1.5
    //   h1 = 4*0.5 + 5*0.5 = 4.5
    //   result = 1.5*0.5 + 4.5*0.5 = 3.0
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
    let hm = Heightmap::from_data(data, 3).unwrap();
    let v = hm.sample_bilinear(0.5, 0.5);
    assert!((v - 3.0).abs() < 1e-4, "Expected 3.0, got {v}");
}

#[test]
fn bilinear_clamp_below_zero() {
    // Negative coords should be clamped to 0
    let data = vec![42.0, 0.0, 0.0, 0.0];
    let hm = Heightmap::from_data(data, 2).unwrap();
    let v = hm.sample_bilinear(-5.0, -5.0);
    assert!(
        (v - 42.0).abs() < 1e-4,
        "Negative coords should clamp to corner: {v}"
    );
}

// ============================================================================
// HEIGHTMAP: calculate_normal — verify exact dx/dz and normalization
// ============================================================================

#[test]
fn normal_flat_surface_points_up() {
    // Uniform height → normal should be (0, 1, 0)
    let data = vec![5.0; 9];
    let hm = Heightmap::from_data(data, 3).unwrap();
    let n = hm.calculate_normal(1, 1, 1.0);
    assert!((n.x).abs() < 1e-4, "x: {}", n.x);
    assert!((n.y - 1.0).abs() < 1e-4, "y: {}", n.y);
    assert!((n.z).abs() < 1e-4, "z: {}", n.z);
}

#[test]
fn normal_slope_in_x() {
    // Ramp: row = [0, 10, 20] in x-direction
    // At (1,1): left=0, right=20, dx = (20-0)/(2*1) = 10
    // Normal = normalize(-10, 1, 0)
    let data = vec![
        0.0, 10.0, 20.0, // row 0
        0.0, 10.0, 20.0, // row 1
        0.0, 10.0, 20.0, // row 2
    ];
    let hm = Heightmap::from_data(data, 3).unwrap();
    let n = hm.calculate_normal(1, 1, 1.0);

    // dx = (20-0)/(2*1) = 10, normal.x = -10 pre-normalize
    assert!(
        n.x < -0.5,
        "Normal should point against +X slope: x={}",
        n.x
    );
    assert!(n.y > 0.0, "Normal should have positive Y: y={}", n.y);
    assert!(
        n.z.abs() < 1e-4,
        "No Z component for pure X slope: z={}",
        n.z
    );

    // Verify exact normalized values: normalize(-10, 1, 0) = (-10, 1, 0) / sqrt(101)
    let mag = (10.0f32 * 10.0 + 1.0).sqrt();
    let expected_x = -10.0 / mag;
    let expected_y = 1.0 / mag;
    assert!(
        (n.x - expected_x).abs() < 1e-4,
        "x: {} vs {}",
        n.x,
        expected_x
    );
    assert!(
        (n.y - expected_y).abs() < 1e-4,
        "y: {} vs {}",
        n.y,
        expected_y
    );
}

#[test]
fn normal_slope_in_z() {
    // Ramp in z-direction: each row increases by 10
    let data = vec![
        0.0, 0.0, 0.0, // row 0 (z=0)
        10.0, 10.0, 10.0, // row 1 (z=1)
        20.0, 20.0, 20.0, // row 2 (z=2)
    ];
    let hm = Heightmap::from_data(data, 3).unwrap();
    let n = hm.calculate_normal(1, 1, 1.0);

    // dz = (20-0)/(2*1) = 10, normal.z = -10 pre-normalize
    assert!(
        n.z < -0.5,
        "Normal should point against +Z slope: z={}",
        n.z
    );
    assert!(n.x.abs() < 1e-4, "No X component: x={}", n.x);
}

#[test]
fn normal_with_scale_factor() {
    // Same as normal_slope_in_x but with scale=2.0
    // dx = (20-0)/(2*2) = 5, normal = normalize(-5, 1, 0)
    let data = vec![0.0, 10.0, 20.0, 0.0, 10.0, 20.0, 0.0, 10.0, 20.0];
    let hm = Heightmap::from_data(data, 3).unwrap();
    let n = hm.calculate_normal(1, 1, 2.0);

    let mag = (5.0f32 * 5.0 + 1.0).sqrt();
    let expected_x = -5.0 / mag;
    let expected_y = 1.0 / mag;
    assert!(
        (n.x - expected_x).abs() < 1e-4,
        "x with scale: {} vs {}",
        n.x,
        expected_x
    );
    assert!(
        (n.y - expected_y).abs() < 1e-4,
        "y with scale: {} vs {}",
        n.y,
        expected_y
    );
}

#[test]
fn normal_at_edge_uses_self_for_boundary() {
    // At (0,0), left = get_height(0,0), right = get_height(1,0)
    // up = get_height(0,0), down = get_height(0,1)
    // So boundary uses self as neighbor
    let data = vec![10.0, 20.0, 30.0, 40.0];
    let hm = Heightmap::from_data(data, 2).unwrap();
    let n = hm.calculate_normal(0, 0, 1.0);
    // left=10 (self), right=20 → dx = (20-10)/2 = 5
    // up=10 (self), down=30 → dz = (30-10)/2 = 10
    // normal = normalize(-5, 1, -10)
    let mag = (25.0 + 1.0 + 100.0f32).sqrt();
    assert!((n.x - (-5.0 / mag)).abs() < 1e-4);
    assert!((n.y - (1.0 / mag)).abs() < 1e-4);
    assert!((n.z - (-10.0 / mag)).abs() < 1e-4);
}

// ============================================================================
// HEIGHTMAP: smooth — verify kernel math
// ============================================================================

#[test]
fn smooth_single_iteration_center_value() {
    // 3×3 grid, all zeros except center=8.0
    // smooth kernel at (1,1):
    //   sum = left(0) + right(0) + up(0) + down(0) + center(8)*4 = 32
    //   new_center = 32 / 8 = 4.0
    let mut data = vec![0.0; 9];
    data[4] = 8.0; // center at (1,1)
    let mut hm = Heightmap::from_data(data, 3).unwrap();
    hm.smooth(1);
    let center = hm.get_height(1, 1);
    assert!(
        (center - 4.0).abs() < 1e-4,
        "After 1 smooth iteration, center should be 4.0, got {center}"
    );
}

#[test]
fn smooth_preserves_uniform() {
    // Uniform heightmap should be unchanged by smoothing
    let data = vec![5.0; 25]; // 5×5
    let mut hm = Heightmap::from_data(data, 5).unwrap();
    hm.smooth(3);
    for z in 0..5 {
        for x in 0..5 {
            assert!((hm.get_height(x, z) - 5.0).abs() < 1e-4);
        }
    }
}

#[test]
fn smooth_updates_min_max() {
    // Peak at center, smooth should reduce max
    let mut data = vec![0.0; 25];
    data[12] = 100.0; // center of 5×5
    let mut hm = Heightmap::from_data(data, 5).unwrap();
    hm.smooth(1);
    assert!(
        hm.max_height() < 100.0,
        "Max should decrease after smoothing"
    );
    assert!(hm.min_height() >= 0.0, "Min should stay non-negative");
}

// ============================================================================
// HEIGHTMAP: generate_vertices — verify exact world positions
// ============================================================================

#[test]
fn vertices_3x3_exact_positions() {
    // 3×3, chunk_size=2.0, offset=origin → step = 2.0/(3-1) = 1.0
    let data = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0];
    let hm = Heightmap::from_data(data, 3).unwrap();
    let verts = hm.generate_vertices(2.0, Vec3::ZERO);
    assert_eq!(verts.len(), 9);

    // (x=0,z=0) → world (0, 10, 0)
    assert!((verts[0].x - 0.0).abs() < 1e-4);
    assert!((verts[0].y - 10.0).abs() < 1e-4);
    assert!((verts[0].z - 0.0).abs() < 1e-4);

    // (x=1,z=0) → world (1, 20, 0)
    assert!((verts[1].x - 1.0).abs() < 1e-4);
    assert!((verts[1].y - 20.0).abs() < 1e-4);

    // (x=0,z=2) → world (0, 70, 2)
    assert!((verts[6].x - 0.0).abs() < 1e-4);
    assert!((verts[6].y - 70.0).abs() < 1e-4);
    assert!((verts[6].z - 2.0).abs() < 1e-4);

    // (x=2,z=2) → world (2, 90, 2)
    assert!((verts[8].x - 2.0).abs() < 1e-4);
    assert!((verts[8].y - 90.0).abs() < 1e-4);
    assert!((verts[8].z - 2.0).abs() < 1e-4);
}

#[test]
fn vertices_with_offset() {
    let data = vec![5.0; 4];
    let hm = Heightmap::from_data(data, 2).unwrap();
    let offset = Vec3::new(100.0, 0.0, 200.0);
    let verts = hm.generate_vertices(10.0, offset);
    assert_eq!(verts.len(), 4);

    // step = 10.0/(2-1) = 10.0
    // (0,0) → world (100, 5, 200)
    assert!((verts[0].x - 100.0).abs() < 1e-4);
    assert!((verts[0].y - 5.0).abs() < 1e-4);
    assert!((verts[0].z - 200.0).abs() < 1e-4);

    // (1,1) → world (110, 5, 210)
    assert!((verts[3].x - 110.0).abs() < 1e-4);
    assert!((verts[3].z - 210.0).abs() < 1e-4);
}

// ============================================================================
// HEIGHTMAP: generate_indices — verify exact triangle winding
// ============================================================================

#[test]
fn indices_2x2_exact() {
    // 2×2 grid has 1 quad → 2 triangles → 6 indices
    let data = vec![0.0; 4];
    let hm = Heightmap::from_data(data, 2).unwrap();
    let idx = hm.generate_indices();
    assert_eq!(idx.len(), 6);
    // Quad at (x=0,z=0): base = 0*2+0 = 0, resolution = 2
    // tri1: 0, 1, 2
    // tri2: 1, 3, 2
    assert_eq!(idx[0], 0);
    assert_eq!(idx[1], 1);
    assert_eq!(idx[2], 2); // base + resolution
    assert_eq!(idx[3], 1);
    assert_eq!(idx[4], 3); // base + resolution + 1
    assert_eq!(idx[5], 2);
}

#[test]
fn indices_3x3_count_and_quad_pattern() {
    // 3×3 grid: 2×2 quads → 4 quads → 24 indices
    let data = vec![0.0; 9];
    let hm = Heightmap::from_data(data, 3).unwrap();
    let idx = hm.generate_indices();
    assert_eq!(idx.len(), 24);

    // Check first quad (0,0): base=0, res=3
    assert_eq!(idx[0], 0);
    assert_eq!(idx[1], 1);
    assert_eq!(idx[2], 3); // base + resolution
    assert_eq!(idx[3], 1);
    assert_eq!(idx[4], 4); // base + resolution + 1
    assert_eq!(idx[5], 3);

    // Check second quad (1,0): base=1
    assert_eq!(idx[6], 1);
    assert_eq!(idx[7], 2);
    assert_eq!(idx[8], 4); // 1 + 3
    assert_eq!(idx[9], 2);
    assert_eq!(idx[10], 5); // 1 + 3 + 1
    assert_eq!(idx[11], 4);
}

#[test]
fn indices_4x4_total_count() {
    // 4×4 grid: 3×3 quads → 9 quads → 54 indices
    let data = vec![0.0; 16];
    let hm = Heightmap::from_data(data, 4).unwrap();
    let idx = hm.generate_indices();
    assert_eq!(idx.len(), 54);
}

// ============================================================================
// HEIGHTMAP: get_height_at_index — verify fallback behavior
// ============================================================================

#[test]
fn get_height_at_index_valid() {
    let data = vec![10.0, 20.0, 30.0, 40.0];
    let hm = Heightmap::from_data(data, 2).unwrap();
    assert_eq!(hm.get_height_at_index(0), 10.0);
    assert_eq!(hm.get_height_at_index(1), 20.0);
    assert_eq!(hm.get_height_at_index(3), 40.0);
}

#[test]
fn get_height_at_index_out_of_bounds_returns_zero() {
    let data = vec![10.0, 20.0, 30.0, 40.0];
    let hm = Heightmap::from_data(data, 2).unwrap();
    assert_eq!(hm.get_height_at_index(100), 0.0);
}

// ============================================================================
// HEIGHTMAP: recalculate_bounds — empty data edge case
// ============================================================================

#[test]
fn recalculate_bounds_empty_data() {
    let config = HeightmapConfig {
        resolution: 2,
        ..Default::default()
    };
    let mut hm = Heightmap::new(config).unwrap();
    // Hack: we can't make data empty through the API, but we can set all to same value
    hm.recalculate_bounds();
    assert_eq!(hm.min_height(), 0.0);
    assert_eq!(hm.max_height(), 0.0);
}

// ============================================================================
// EROSION: thermal erosion — pin down arithmetic
// ============================================================================

#[test]
fn thermal_erosion_lowers_spike() {
    // Create a 5×5 heightmap with a central spike
    let mut data = vec![0.0; 25];
    data[12] = 50.0; // center at (2,2)
    let mut hm = Heightmap::from_data(data, 5).unwrap();

    let center_before = hm.get_height(2, 2);
    assert_eq!(center_before, 50.0);

    astraweave_terrain::erosion::apply_thermal_erosion(&mut hm, 5, 30.0).unwrap();

    let center_after = hm.get_height(2, 2);
    assert!(
        center_after < center_before,
        "Spike should erode: {} >= {}",
        center_after,
        center_before
    );

    // Neighbors should gain material
    let n_right = hm.get_height(3, 2);
    assert!(
        n_right > 0.0,
        "Right neighbor should receive material: {n_right}"
    );
}

#[test]
fn thermal_erosion_preserves_flat() {
    // Flat heightmap should be unchanged
    let data = vec![10.0; 25];
    let mut hm = Heightmap::from_data(data, 5).unwrap();
    astraweave_terrain::erosion::apply_thermal_erosion(&mut hm, 10, 30.0).unwrap();
    for z in 0..5 {
        for x in 0..5 {
            assert!(
                (hm.get_height(x, z) - 10.0).abs() < 1e-3,
                "Flat terrain should stay flat at ({x},{z}): {}",
                hm.get_height(x, z)
            );
        }
    }
}

#[test]
fn thermal_erosion_total_mass_conservation() {
    // Total height sum should be approximately conserved
    let mut data = vec![0.0; 25];
    data[12] = 100.0;
    let total_before: f32 = data.iter().sum();
    let mut hm = Heightmap::from_data(data, 5).unwrap();
    astraweave_terrain::erosion::apply_thermal_erosion(&mut hm, 3, 20.0).unwrap();
    let total_after: f32 = hm.data().iter().sum();
    // Material should be approximately conserved (redistribution doesn't create/destroy)
    assert!(
        (total_after - total_before).abs() < 0.1 * total_before.abs().max(1.0),
        "Mass not conserved: before={total_before}, after={total_after}"
    );
}

// ============================================================================
// EROSION: hydraulic erosion — verify it modifies heightmap
// ============================================================================

#[test]
fn hydraulic_erosion_delegates_to_heightmap() {
    // Verify the delegation: erosion::apply_hydraulic_erosion calls heightmap.apply_hydraulic_erosion
    // Use a larger grid with more variation for the internal hydraulic to have effect
    let mut data = vec![0.0; 1024]; // 32×32
    for x in 0..32u32 {
        for z in 0..32u32 {
            data[(z * 32 + x) as usize] =
                ((x as f32 - 16.0).powi(2) + (z as f32 - 16.0).powi(2)).sqrt() * 3.0;
        }
    }
    let mut hm = Heightmap::from_data(data.clone(), 32).unwrap();
    let result = astraweave_terrain::erosion::apply_hydraulic_erosion(&mut hm, 1.0);
    assert!(result.is_ok(), "Hydraulic erosion should succeed");
    // The function is a simple delegate — just verify it returns Ok
}

// ============================================================================
// CLIMATE: Whittaker boundaries — exact boundary classification
// ============================================================================

#[test]
fn whittaker_tundra_low_temperature() {
    // t < 0.2 → Tundra regardless of moisture
    assert_eq!(
        astraweave_terrain::climate::utils::classify_whittaker_biome(0.1, 0.0),
        BiomeType::Tundra
    );
    assert_eq!(
        astraweave_terrain::climate::utils::classify_whittaker_biome(0.1, 0.9),
        BiomeType::Tundra
    );
    assert_eq!(
        astraweave_terrain::climate::utils::classify_whittaker_biome(0.19, 0.5),
        BiomeType::Tundra
    );
}

#[test]
fn whittaker_tundra_cool_dry() {
    // t < 0.4 && m < 0.3 → Tundra
    assert_eq!(
        astraweave_terrain::climate::utils::classify_whittaker_biome(0.3, 0.2),
        BiomeType::Tundra
    );
}

#[test]
fn whittaker_desert_cool_arid() {
    // t < 0.6 && m < 0.2 → Desert
    assert_eq!(
        astraweave_terrain::climate::utils::classify_whittaker_biome(0.5, 0.1),
        BiomeType::Desert
    );
}

#[test]
fn whittaker_desert_hot_semiarid() {
    // t > 0.7 && m < 0.4 → Desert
    assert_eq!(
        astraweave_terrain::climate::utils::classify_whittaker_biome(0.8, 0.3),
        BiomeType::Desert
    );
}

#[test]
fn whittaker_swamp_very_wet() {
    // m > 0.8 → Swamp
    assert_eq!(
        astraweave_terrain::climate::utils::classify_whittaker_biome(0.5, 0.9),
        BiomeType::Swamp
    );
}

#[test]
fn whittaker_forest_warm_wet() {
    // t > 0.6 && m > 0.6 → Forest
    assert_eq!(
        astraweave_terrain::climate::utils::classify_whittaker_biome(0.7, 0.7),
        BiomeType::Forest
    );
}

#[test]
fn whittaker_forest_moderate() {
    // t > 0.4 && m > 0.4 → Forest
    assert_eq!(
        astraweave_terrain::climate::utils::classify_whittaker_biome(0.5, 0.5),
        BiomeType::Forest
    );
}

#[test]
fn whittaker_grassland_fallthrough() {
    // None of the above → Grassland
    // t=0.45, m=0.35 → doesn't match tundra/desert/swamp/forest
    assert_eq!(
        astraweave_terrain::climate::utils::classify_whittaker_biome(0.45, 0.35),
        BiomeType::Grassland
    );
}

#[test]
fn climate_deterministic_same_seed() {
    let config = ClimateConfig::default();
    let c1 = ClimateMap::new(&config, 42);
    let c2 = ClimateMap::new(&config, 42);

    let (t1, m1) = c1.sample_climate(500.0, 500.0, 25.0);
    let (t2, m2) = c2.sample_climate(500.0, 500.0, 25.0);

    assert_eq!(t1, t2);
    assert_eq!(m1, m2);
}

#[test]
fn climate_different_seeds_differ() {
    let config = ClimateConfig::default();
    let c1 = ClimateMap::new(&config, 1);
    let c2 = ClimateMap::new(&config, 99999);

    let (t1, m1) = c1.sample_climate(1000.0, 1000.0, 10.0);
    let (t2, m2) = c2.sample_climate(1000.0, 1000.0, 10.0);

    // Very unlikely to be identical with different seeds
    assert!(
        t1 != t2 || m1 != m2,
        "Different seeds should produce different climate"
    );
}

#[test]
fn climate_height_gradient_reduces_temperature() {
    let config = ClimateConfig::default();
    let climate = ClimateMap::new(&config, 42);

    let (t_low, _) = climate.sample_climate(100.0, 100.0, 0.0);
    let (t_high, _) = climate.sample_climate(100.0, 100.0, 200.0);

    assert!(
        t_high < t_low,
        "Higher elevation should be cooler: t_low={t_low}, t_high={t_high}"
    );
}

// ============================================================================
// BIOME BLENDING: PackedBiomeBlend normalization and dominant biome
// ============================================================================

#[test]
fn packed_blend_single_biome_full_weight() {
    let weights = vec![BiomeWeight {
        biome: BiomeType::Desert,
        weight: 5.0,
    }];
    let packed = PackedBiomeBlend::from_weights(&weights);
    assert!(
        (packed.weights[0] - 1.0).abs() < 1e-4,
        "Single biome should normalize to 1.0"
    );
    assert_eq!(packed.dominant_biome(), BiomeType::Desert);
}

#[test]
fn packed_blend_two_biomes_proportional() {
    let weights = vec![
        BiomeWeight {
            biome: BiomeType::Grassland,
            weight: 3.0,
        },
        BiomeWeight {
            biome: BiomeType::Forest,
            weight: 1.0,
        },
    ];
    let packed = PackedBiomeBlend::from_weights(&weights);
    let sum: f32 = packed.weights.iter().sum();
    assert!(
        (sum - 1.0).abs() < 1e-4,
        "Weights should sum to 1.0, got {sum}"
    );
    assert!(
        (packed.weights[0] - 0.75).abs() < 1e-4,
        "3:1 ratio → 0.75, got {}",
        packed.weights[0]
    );
    assert!(
        (packed.weights[1] - 0.25).abs() < 1e-4,
        "3:1 ratio → 0.25, got {}",
        packed.weights[1]
    );
}

#[test]
fn packed_blend_zero_weights_fallback() {
    let weights: Vec<BiomeWeight> = vec![];
    let packed = PackedBiomeBlend::from_weights(&weights);
    assert!(
        (packed.weights[0] - 1.0).abs() < 1e-4,
        "Empty input should fallback to 1.0"
    );
    assert_eq!(
        packed.dominant_biome(),
        BiomeType::Grassland,
        "Empty should default to Grassland"
    );
}

#[test]
fn packed_blend_tiny_weights_culled() {
    let weights = vec![
        BiomeWeight {
            biome: BiomeType::Forest,
            weight: 1.0,
        },
        BiomeWeight {
            biome: BiomeType::Desert,
            weight: 0.0005,
        }, // below 0.001 threshold
    ];
    let packed = PackedBiomeBlend::from_weights(&weights);
    // Desert should be filtered out
    assert!(
        (packed.weights[0] - 1.0).abs() < 1e-3,
        "Tiny weight should be culled"
    );
}

#[test]
fn packed_blend_five_biomes_drops_lowest() {
    let weights = vec![
        BiomeWeight {
            biome: BiomeType::Grassland,
            weight: 0.5,
        },
        BiomeWeight {
            biome: BiomeType::Forest,
            weight: 0.3,
        },
        BiomeWeight {
            biome: BiomeType::Mountain,
            weight: 0.2,
        },
        BiomeWeight {
            biome: BiomeType::Desert,
            weight: 0.1,
        },
        BiomeWeight {
            biome: BiomeType::Tundra,
            weight: 0.05,
        },
    ];
    let packed = PackedBiomeBlend::from_weights(&weights);
    // Only top 4 should survive
    let non_zero: usize = packed.weights.iter().filter(|&&w| w > 0.0).count();
    assert!(non_zero <= 4, "At most 4 biomes: got {non_zero}");
}

// ============================================================================
// TEXTURE SPLATTING: SplatRule::evaluate — pin down weight arithmetic
// ============================================================================

#[test]
fn splat_rule_perfect_match() {
    let rule = SplatRule::grass();
    // height=50 in [0,100], slope=15 in [0,30] → base weight 1.0
    let w = rule.evaluate(50.0, 15.0);
    assert!(
        (w - 1.0).abs() < 1e-4,
        "Perfect match should give weight 1.0, got {w}"
    );
}

#[test]
fn splat_rule_below_min_height_falloff() {
    let rule = SplatRule::grass(); // min_height=0, height_falloff=0.02
                                   // height=-10 → penalty = (0 - (-10)) * 0.02 = 0.2 → weight = 1.0 * (1 - 0.2) = 0.8
    let w = rule.evaluate(-10.0, 15.0);
    assert!((w - 0.8).abs() < 1e-4, "Expected 0.8, got {w}");
}

#[test]
fn splat_rule_above_max_height_falloff() {
    let rule = SplatRule::grass(); // max_height=100, height_falloff=0.02
                                   // height=130 → penalty = (130-100)*0.02 = 0.6 → weight = 1.0*(1-0.6) = 0.4
    let w = rule.evaluate(130.0, 15.0);
    assert!((w - 0.4).abs() < 1e-4, "Expected 0.4, got {w}");
}

#[test]
fn splat_rule_above_max_slope_falloff() {
    let rule = SplatRule::grass(); // max_slope=30, slope_falloff=0.05
                                   // slope=40 → penalty = (40-30)*0.05 = 0.5 → weight = 1.0*(1-0.5) = 0.5
    let w = rule.evaluate(50.0, 40.0);
    assert!((w - 0.5).abs() < 1e-4, "Expected 0.5, got {w}");
}

#[test]
fn splat_rule_extreme_height_clamps_to_zero() {
    let rule = SplatRule::grass(); // height_falloff=0.02
                                   // height=200 → penalty = (200-100)*0.02 = 2.0 → max(0, 1-2) = 0.0
    let w = rule.evaluate(200.0, 15.0);
    assert!(
        w.abs() < 1e-4,
        "Far outside range should clamp to 0, got {w}"
    );
}

// ============================================================================
// TEXTURE SPLATTING: TriplanarWeights — exact sharpness math
// ============================================================================

#[test]
fn triplanar_pure_y_up() {
    let tw = TriplanarWeights::from_normal(Vec3::Y, 4.0);
    // abs_normal = (0, 1, 0)
    // x^4=0, y^4=1, z^4=0, sum=1
    assert!(
        (tw.y - 1.0).abs() < 1e-4,
        "Pure Y-up should give y=1.0, got {}",
        tw.y
    );
    assert!(tw.x < 1e-4);
    assert!(tw.z < 1e-4);
}

#[test]
fn triplanar_pure_x_facing() {
    let tw = TriplanarWeights::from_normal(Vec3::X, 4.0);
    assert!((tw.x - 1.0).abs() < 1e-4);
    assert!(tw.y < 1e-4);
}

#[test]
fn triplanar_equal_xy_sharpness_1() {
    // (0.707, 0.707, 0) with sharpness=1 → equal component → 50/50
    let n = Vec3::new(1.0, 1.0, 0.0).normalize();
    let tw = TriplanarWeights::from_normal(n, 1.0);
    assert!(
        (tw.x - tw.y).abs() < 0.05,
        "Equal components should give equal weights: x={}, y={}",
        tw.x,
        tw.y
    );
    assert!((tw.x + tw.y - 1.0).abs() < 0.05);
}

#[test]
fn triplanar_should_use_triplanar_steep() {
    // Steep surface (mostly X-facing) → y weight is low
    let tw = TriplanarWeights::from_normal(Vec3::X, 4.0);
    assert!(
        tw.should_use_triplanar(0.5),
        "Vertical cliff should use triplanar"
    );
}

#[test]
fn triplanar_should_not_use_triplanar_flat() {
    let tw = TriplanarWeights::from_normal(Vec3::Y, 4.0);
    assert!(
        !tw.should_use_triplanar(0.5),
        "Flat surface should not use triplanar"
    );
}

// ============================================================================
// TEXTURE SPLATTING: SplatWeights — get_weight, dominant_layer
// ============================================================================

#[test]
fn splat_weights_get_weight_all_layers() {
    let weights = SplatWeights::from_weights(&[0.1, 0.2, 0.3, 0.4, 0.0, 0.0, 0.0, 0.0]);
    // Normalized: total=1.0, so values stay as-is
    assert!((weights.get_weight(0) - 0.1).abs() < 1e-3);
    assert!((weights.get_weight(1) - 0.2).abs() < 1e-3);
    assert!((weights.get_weight(2) - 0.3).abs() < 1e-3);
    assert!((weights.get_weight(3) - 0.4).abs() < 1e-3);
}

#[test]
fn splat_weights_dominant_layer_correct() {
    let weights = SplatWeights::from_weights(&[0.1, 0.1, 0.8]);
    assert_eq!(weights.dominant_layer(), 2, "Layer 2 should dominate");
}

#[test]
fn splat_weights_out_of_range_returns_zero() {
    let weights = SplatWeights::default();
    assert_eq!(weights.get_weight(99), 0.0);
}

#[test]
fn splat_weights_extended_layers_normalization() {
    let weights = SplatWeights::from_weights(&[1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]);
    // 8 equal weights → each = 0.125
    for i in 0..8 {
        assert!(
            (weights.get_weight(i) - 0.125).abs() < 1e-3,
            "Layer {i}: {} != 0.125",
            weights.get_weight(i)
        );
    }
}

// ============================================================================
// TEXTURE SPLATTING: SplatMapGenerator — rule-based material assignment
// ============================================================================

#[test]
fn splat_generator_beach_is_sand() {
    let config = SplatConfig::default();
    let gen = SplatMapGenerator::with_default_rules(config, 42);
    let w = gen.calculate_weights(2.0, Vec3::Y);
    assert_eq!(
        w.dominant_layer(),
        1,
        "Beach height should give sand (layer 1)"
    );
}

#[test]
fn splat_generator_high_flat_is_snow() {
    let config = SplatConfig::default();
    let gen = SplatMapGenerator::with_default_rules(config, 42);
    let w = gen.calculate_weights(180.0, Vec3::Y);
    assert_eq!(
        w.dominant_layer(),
        4,
        "High flat terrain should give snow (layer 4)"
    );
}

#[test]
fn splat_generator_steep_is_rock() {
    let config = SplatConfig::default();
    let gen = SplatMapGenerator::with_default_rules(config, 42);
    let n = Vec3::new(0.3, 0.3, 0.9).normalize();
    let w = gen.calculate_weights(50.0, n);
    assert_eq!(
        w.dominant_layer(),
        7,
        "Steep slope should give rock (layer 7)"
    );
}

// ============================================================================
// BIOME BLEND CONFIG: verify defaults
// ============================================================================

#[test]
fn biome_blend_config_defaults() {
    let cfg = BiomeBlendConfig::default();
    assert!((cfg.blend_radius - 64.0).abs() < 1e-4);
    assert!((cfg.falloff_power - 2.0).abs() < 1e-4);
    assert!((cfg.edge_noise_scale - 0.02).abs() < 1e-4);
    assert!((cfg.edge_noise_amplitude - 16.0).abs() < 1e-4);
    assert!((cfg.min_weight_threshold - 0.01).abs() < 1e-4);
    assert!(cfg.height_blend_enabled);
    assert!((cfg.height_blend_factor - 0.3).abs() < 1e-4);
}

// ============================================================================
// SPLAT CONFIG: verify defaults
// ============================================================================

#[test]
fn splat_config_defaults() {
    let cfg = SplatConfig::default();
    assert!(cfg.triplanar_enabled);
    assert!((cfg.triplanar_slope_threshold - 45.0).abs() < 1e-4);
    assert!(cfg.height_blending_enabled);
    assert!((cfg.height_blend_contrast - 8.0).abs() < 1e-4);
    assert!((cfg.rock_slope_threshold - 35.0).abs() < 1e-4);
    assert!(cfg.detail_normal_enabled);
    assert!((cfg.detail_uv_scale - 16.0).abs() < 1e-4);
    assert!((cfg.snow_height_threshold - 150.0).abs() < 1e-4);
    assert!((cfg.snow_slope_fade - 30.0).abs() < 1e-4);
}

// ============================================================================
// TERRAIN MATERIAL: verify presets exact values
// ============================================================================

#[test]
fn terrain_material_grass_preset() {
    let g = TerrainMaterial::grass();
    assert_eq!(g.id, 0);
    assert_eq!(g.name, "Grass");
    assert!((g.uv_scale - 4.0).abs() < 1e-4);
    assert!((g.blend_sharpness - 2.0).abs() < 1e-4);
    assert!((g.triplanar_sharpness - 4.0).abs() < 1e-4);
}

#[test]
fn terrain_material_rock_preset() {
    let r = TerrainMaterial::rock();
    assert_eq!(r.id, 1);
    assert_eq!(r.name, "Rock");
    assert!((r.uv_scale - 2.0).abs() < 1e-4);
    assert!((r.blend_sharpness - 4.0).abs() < 1e-4);
    assert!((r.triplanar_sharpness - 8.0).abs() < 1e-4);
}

#[test]
fn terrain_material_sand_preset() {
    let s = TerrainMaterial::sand();
    assert_eq!(s.id, 2);
    assert_eq!(s.name, "Sand");
    assert!((s.uv_scale - 8.0).abs() < 1e-4);
}

#[test]
fn terrain_material_snow_preset() {
    let s = TerrainMaterial::snow();
    assert_eq!(s.id, 3);
    assert_eq!(s.name, "Snow");
    assert!((s.uv_scale - 6.0).abs() < 1e-4);
}

#[test]
fn terrain_material_dirt_preset() {
    let d = TerrainMaterial::dirt();
    assert_eq!(d.id, 4);
    assert_eq!(d.name, "Dirt");
    assert!((d.uv_scale - 4.0).abs() < 1e-4);
    assert!((d.blend_sharpness - 2.5).abs() < 1e-4);
}

// ============================================================================
// SPLAT RULE: verify preset exact values
// ============================================================================

#[test]
fn splat_rule_grass_preset_values() {
    let r = SplatRule::grass();
    assert_eq!(r.material_id, 0);
    assert!((r.min_height - 0.0).abs() < 1e-4);
    assert!((r.max_height - 100.0).abs() < 1e-4);
    assert!((r.min_slope - 0.0).abs() < 1e-4);
    assert!((r.max_slope - 30.0).abs() < 1e-4);
    assert_eq!(r.priority, 10);
    assert!((r.weight - 1.0).abs() < 1e-4);
    assert!((r.height_falloff - 0.02).abs() < 1e-4);
    assert!((r.slope_falloff - 0.05).abs() < 1e-4);
}

#[test]
fn splat_rule_rock_preset_values() {
    let r = SplatRule::rock();
    assert_eq!(r.material_id, 7);
    assert!((r.min_slope - 35.0).abs() < 1e-4);
    assert!((r.max_slope - 90.0).abs() < 1e-4);
    assert_eq!(r.priority, 20);
    assert!((r.slope_falloff - 0.1).abs() < 1e-4);
}

#[test]
fn splat_rule_sand_preset_values() {
    let r = SplatRule::sand();
    assert_eq!(r.material_id, 1);
    assert!((r.min_height - (-5.0)).abs() < 1e-4);
    assert!((r.max_height - 8.0).abs() < 1e-4);
    assert_eq!(r.priority, 15);
    assert!((r.weight - 2.0).abs() < 1e-4);
    assert!((r.height_falloff - 0.3).abs() < 1e-4);
}

#[test]
fn splat_rule_snow_preset_values() {
    let r = SplatRule::snow();
    assert_eq!(r.material_id, 4);
    assert!((r.min_height - 120.0).abs() < 1e-4);
    assert_eq!(r.priority, 25);
    assert!((r.slope_falloff - 0.08).abs() < 1e-4);
}

// ============================================================================
// LIB.RS: WorldConfig defaults
// ============================================================================

#[test]
fn world_config_default_seed() {
    let cfg = WorldConfig::default();
    assert_eq!(cfg.seed, 12345);
}

#[test]
fn world_config_default_chunk_size() {
    let cfg = WorldConfig::default();
    assert!((cfg.chunk_size - 256.0).abs() < 1e-4);
}

#[test]
fn world_config_default_resolution() {
    let cfg = WorldConfig::default();
    assert_eq!(cfg.heightmap_resolution, 128);
}

#[test]
fn world_config_default_biomes_count() {
    let cfg = WorldConfig::default();
    assert_eq!(cfg.biomes.len(), 4);
}

#[test]
fn world_config_default_biome_types() {
    let cfg = WorldConfig::default();
    let types: Vec<BiomeType> = cfg.biomes.iter().map(|b| b.biome_type).collect();
    assert!(types.contains(&BiomeType::Grassland));
    assert!(types.contains(&BiomeType::Desert));
    assert!(types.contains(&BiomeType::Forest));
    assert!(types.contains(&BiomeType::Mountain));
}

// ============================================================================
// LIB.RS: WorldGenerator creation and chunk pipeline
// ============================================================================

#[test]
fn world_generator_stores_config() {
    let mut cfg = WorldConfig::default();
    cfg.seed = 99999;
    let gen = WorldGenerator::new(cfg);
    assert_eq!(gen.config().seed, 99999);
    assert!((gen.config().chunk_size - 256.0).abs() < 1e-4);
}

#[test]
fn world_generator_chunk_has_correct_id() {
    let cfg = WorldConfig::default();
    let gen = WorldGenerator::new(cfg);
    let id = ChunkId::new(3, -2);
    let chunk = gen.generate_chunk(id).unwrap();
    assert_eq!(chunk.id(), id);
}

#[test]
fn world_generator_chunk_heightmap_populated() {
    let cfg = WorldConfig::default();
    let gen = WorldGenerator::new(cfg);
    let chunk = gen.generate_chunk(ChunkId::new(0, 0)).unwrap();
    let hm = chunk.heightmap();
    // Noise-generated heightmap should have non-zero variation
    assert!(
        hm.max_height() > hm.min_height(),
        "Heightmap should have variation"
    );
}

#[test]
fn world_generator_register_and_get() {
    let cfg = WorldConfig::default();
    let mut gen = WorldGenerator::new(cfg);
    let id = ChunkId::new(1, 1);
    gen.generate_and_register_chunk(id).unwrap();
    assert!(
        gen.get_chunk(id).is_some(),
        "Registered chunk should be retrievable"
    );
}

#[test]
fn world_generator_unregistered_chunk_is_none() {
    let cfg = WorldConfig::default();
    let gen = WorldGenerator::new(cfg);
    assert!(gen.get_chunk(ChunkId::new(99, 99)).is_none());
}

#[test]
fn world_generator_streaming_loads_chunks() {
    let cfg = WorldConfig::default();
    let mut gen = WorldGenerator::new(cfg);
    let loaded = gen.stream_chunks(Vec3::ZERO, 1).unwrap();
    assert!(
        !loaded.is_empty(),
        "stream_chunks should load at least one chunk"
    );
}

// ============================================================================
// HEIGHTMAP: from_data — min/max correctness with negative values
// ============================================================================

#[test]
fn from_data_negative_min_max() {
    let data = vec![-10.0, -5.0, 0.0, 5.0];
    let hm = Heightmap::from_data(data, 2).unwrap();
    assert_eq!(hm.min_height(), -10.0);
    assert_eq!(hm.max_height(), 5.0);
}

#[test]
fn from_data_all_negative() {
    let data = vec![-100.0, -50.0, -30.0, -10.0];
    let hm = Heightmap::from_data(data, 2).unwrap();
    assert_eq!(hm.min_height(), -100.0);
    assert_eq!(hm.max_height(), -10.0);
}

// ============================================================================
// HEIGHTMAP CONFIG: exact defaults
// ============================================================================

#[test]
fn heightmap_config_exact_defaults() {
    let cfg = HeightmapConfig::default();
    assert_eq!(cfg.resolution, 128);
    assert!((cfg.min_height - 0.0).abs() < 1e-6);
    assert!((cfg.max_height - 100.0).abs() < 1e-6);
    assert!((cfg.height_scale - 1.0).abs() < 1e-6);
}

// ============================================================================
// CLIMATE CONFIG: exact defaults
// ============================================================================

#[test]
fn climate_config_height_gradient_sign() {
    let cfg = ClimateConfig::default();
    assert!(
        cfg.temperature_height_gradient < 0.0,
        "Temperature should decrease with height"
    );
    assert!(
        cfg.temperature_latitude_gradient > 0.0,
        "Latitude gradient should be positive"
    );
    assert!(
        cfg.moisture_distance_falloff > 0.0,
        "Moisture falloff should be positive"
    );
}

#[test]
fn climate_config_layer_values() {
    let cfg = ClimateConfig::default();
    // Temperature layer
    assert!(cfg.temperature.scale > 0.0);
    assert!(cfg.temperature.amplitude > 0.0);
    assert!(cfg.temperature.octaves >= 1);
    assert!(cfg.temperature.persistence > 0.0 && cfg.temperature.persistence < 1.0);
    assert!(cfg.temperature.lacunarity > 1.0);

    // Moisture layer
    assert!(cfg.moisture.scale > 0.0);
    assert!(cfg.moisture.amplitude > 0.0);
    assert!(cfg.moisture.octaves >= 1);
    assert!(cfg.moisture.persistence > 0.0 && cfg.moisture.persistence < 1.0);
    assert!(cfg.moisture.lacunarity > 1.0);
}

// ============================================================================
// CLIMATE: generate_climate_preview and biome_classification_map
// ============================================================================

#[test]
fn climate_preview_correct_size() {
    let config = ClimateConfig::default();
    let climate = ClimateMap::new(&config, 42);
    let (temps, moist) =
        astraweave_terrain::climate::utils::generate_climate_preview(&climate, 8, 100.0);
    assert_eq!(temps.len(), 64); // 8*8
    assert_eq!(moist.len(), 64);
}

#[test]
fn biome_classification_map_correct_size() {
    let config = ClimateConfig::default();
    let climate = ClimateMap::new(&config, 42);
    let biomes =
        astraweave_terrain::climate::utils::generate_biome_classification_map(&climate, 8, 100.0);
    assert_eq!(biomes.len(), 64);
}

#[test]
fn climate_stats_temperature_range() {
    let config = ClimateConfig::default();
    let climate = ClimateMap::new(&config, 42);
    let stats = astraweave_terrain::climate::utils::calculate_climate_stats(
        &climate, 0.0, 1000.0, 0.0, 1000.0, 10,
    );
    assert!(stats.temperature_min <= stats.temperature_avg);
    assert!(stats.temperature_avg <= stats.temperature_max);
    assert!(stats.moisture_min <= stats.moisture_avg);
    assert!(stats.moisture_avg <= stats.moisture_max);
}

// ============================================================================
// BIOME BLENDING: height modification function
// ============================================================================

#[test]
fn biome_blender_beach_preferred_at_low_height() {
    let config = BiomeBlendConfig::default();
    let blender = BiomeBlender::new(config, 42);
    let center = Vec2::ZERO;

    let neighbors = vec![
        (Vec2::new(5.0, 0.0), BiomeType::Beach),
        (Vec2::new(5.0, 5.0), BiomeType::Grassland),
    ];

    // At low height=2, beach should be preferred
    let blend_low = blender.calculate_blend_weights(center, 2.0, &neighbors);
    // At high height=100, beach should be suppressed
    let blend_high = blender.calculate_blend_weights(center, 100.0, &neighbors);

    let beach_low = blend_low
        .weights
        .iter()
        .zip(blend_low.biome_ids.iter())
        .find(|(_, &id)| id == BiomeType::Beach as u8)
        .map(|(w, _)| *w)
        .unwrap_or(0.0);
    let beach_high = blend_high
        .weights
        .iter()
        .zip(blend_high.biome_ids.iter())
        .find(|(_, &id)| id == BiomeType::Beach as u8)
        .map(|(w, _)| *w)
        .unwrap_or(0.0);

    assert!(
        beach_low >= beach_high,
        "Beach should be more prominent at low height: low={beach_low}, high={beach_high}"
    );
}

// ============================================================================
// MAX_BLEND_BIOMES constant
// ============================================================================

#[test]
fn max_blend_biomes_is_4() {
    assert_eq!(astraweave_terrain::biome_blending::MAX_BLEND_BIOMES, 4);
}

// ============================================================================
// MAX_SPLAT_LAYERS constant
// ============================================================================

#[test]
fn max_splat_layers_is_8() {
    assert_eq!(MAX_SPLAT_LAYERS, 8);
}
