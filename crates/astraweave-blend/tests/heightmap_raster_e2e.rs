//! End-to-end integration tests for heightmap rasterization.
//!
//! Tests the flow: terrain mesh → rasterization → bilinear sampling → FixedPlacement extraction.

use astraweave_blend::heightmap_raster::{
    is_terrain_name, rasterize_terrain_meshes, ExtractedTerrainMesh, FixedPlacement,
    TerrainBounds,
};

// ============================================================================
// Helpers
// ============================================================================

/// Create a flat terrain mesh (two triangles forming a quad) at known coordinates.
fn make_flat_quad(name: &str, min_x: f32, min_z: f32, max_x: f32, max_z: f32, height: f32) -> ExtractedTerrainMesh {
    let vertices = vec![
        [min_x, height, min_z],
        [max_x, height, min_z],
        [max_x, height, max_z],
        [min_x, height, max_z],
    ];
    let faces = vec![[0, 1, 2], [0, 2, 3]];
    let bounds = TerrainBounds::from_vertices(&vertices).unwrap();
    ExtractedTerrainMesh {
        name: name.to_string(),
        vertices,
        faces,
        bounds,
    }
}

/// Create a sloped terrain mesh: height increases linearly from min_z to max_z.
fn make_sloped_quad(
    name: &str,
    min_x: f32, min_z: f32, max_x: f32, max_z: f32,
    h_low: f32, h_high: f32,
) -> ExtractedTerrainMesh {
    let vertices = vec![
        [min_x, h_low, min_z],
        [max_x, h_low, min_z],
        [max_x, h_high, max_z],
        [min_x, h_high, max_z],
    ];
    let faces = vec![[0, 1, 2], [0, 2, 3]];
    let bounds = TerrainBounds::from_vertices(&vertices).unwrap();
    ExtractedTerrainMesh {
        name: name.to_string(),
        vertices,
        faces,
        bounds,
    }
}

// ============================================================================
// Test: single flat mesh → rasterized heightmap → verify all values equal
// ============================================================================

#[test]
fn test_flat_mesh_rasterization() {
    let mesh = make_flat_quad("Terrain", 0.0, 0.0, 100.0, 100.0, 5.0);
    let result = rasterize_terrain_meshes(&[mesh], 16).unwrap();

    assert_eq!(result.resolution, 16);
    assert!((result.world_min[0] - 0.0).abs() < 1e-3);
    assert!((result.world_max[0] - 100.0).abs() < 1e-3);
    assert!((result.min_height - 5.0).abs() < 0.5);
    assert!((result.max_height - 5.0).abs() < 0.5);

    // All interior heights should be close to 5.0
    for &h in &result.data {
        // Holes (NEG_INFINITY) may exist at edges; check non-hole values
        if h > f32::NEG_INFINITY {
            assert!(
                (h - 5.0).abs() < 1.0,
                "Rasterized height {} is far from expected 5.0",
                h
            );
        }
    }
}

// ============================================================================
// Test: sloped mesh → bilinear sampling → verify linear gradient
// ============================================================================

#[test]
fn test_sloped_mesh_bilinear_sampling() {
    let mesh = make_sloped_quad("Landscape_01", 0.0, 0.0, 100.0, 100.0, 0.0, 50.0);
    let result = rasterize_terrain_meshes(&[mesh], 32).unwrap();

    // Sample near the low end (z ≈ 0)
    let s_low = result.sample(50.0, 5.0);
    // Sample near the high end (z ≈ 100)
    let s_high = result.sample(50.0, 95.0);

    // Height should increase from low to high
    assert!(
        s_high > s_low,
        "Expected height at z=95 ({}) > height at z=5 ({})",
        s_high,
        s_low
    );

    // The midpoint should be approximately half
    let s_mid = result.sample(50.0, 50.0);
    assert!(
        (s_mid - 25.0).abs() < 10.0,
        "Midpoint sample {} expected ~25.0",
        s_mid
    );
}

// ============================================================================
// Test: multi-tile stitching → verify seam averaging
// ============================================================================

#[test]
fn test_multi_tile_seam_stitching() {
    // Two adjacent terrain tiles meeting at x=100
    let tile_a = make_flat_quad("Ground_A", 0.0, 0.0, 100.0, 100.0, 10.0);
    let tile_b = make_flat_quad("Ground_B", 100.0, 0.0, 200.0, 100.0, 20.0);

    let result = rasterize_terrain_meshes(&[tile_a, tile_b], 32).unwrap();

    // The combined bounds should cover 0..200 in X
    assert!(result.world_min[0] <= 0.1);
    assert!(result.world_max[0] >= 199.9);

    // Sample in the middle of tile A (should be ~10)
    let s_a = result.sample(50.0, 50.0);
    assert!(
        (s_a - 10.0).abs() < 2.0,
        "Tile A sample {} expected ~10.0",
        s_a
    );

    // Sample in the middle of tile B (should be ~20)
    let s_b = result.sample(150.0, 50.0);
    assert!(
        (s_b - 20.0).abs() < 2.0,
        "Tile B sample {} expected ~20.0",
        s_b
    );
}

// ============================================================================
// Test: terrain name identification
// ============================================================================

#[test]
fn test_is_terrain_name() {
    // Positive cases
    assert!(is_terrain_name("Terrain"));
    assert!(is_terrain_name("terrain_01"));
    assert!(is_terrain_name("Ground"));
    assert!(is_terrain_name("Main_Landscape"));
    assert!(is_terrain_name("FLOOR_MESH"));
    assert!(is_terrain_name("landscape.001"));

    // Negative cases
    assert!(!is_terrain_name("Tree_01"));
    assert!(!is_terrain_name("Rock_Large"));
    assert!(!is_terrain_name("Bush"));
    assert!(!is_terrain_name("Camera"));
    assert!(!is_terrain_name("Light"));
}

// ============================================================================
// Test: fixed placement serialization round-trip
// ============================================================================

#[test]
fn test_fixed_placement_serde_roundtrip() {
    let placement = FixedPlacement {
        position: [10.5, 3.2, -7.8],
        rotation: [0.0, 1.5707, 0.0],
        scale: [1.0, 1.0, 1.0],
        mesh_path: "meshes/pine_tree.glb".into(),
        category: "vegetation".into(),
        name: "Pine_Tree.003".into(),
    };

    let json = serde_json::to_string_pretty(&placement).unwrap();
    let deserialized: FixedPlacement = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.name, "Pine_Tree.003");
    assert_eq!(deserialized.mesh_path, "meshes/pine_tree.glb");
    assert!((deserialized.position[0] - 10.5).abs() < 1e-6);
    assert!((deserialized.rotation[1] - 1.5707).abs() < 1e-4);
}

// ============================================================================
// Test: rasterized heightmap footprint area
// ============================================================================

#[test]
fn test_rasterized_heightmap_footprint_area() {
    let mesh = make_flat_quad("Terrain", 0.0, 0.0, 200.0, 150.0, 1.0);
    let result = rasterize_terrain_meshes(&[mesh], 8).unwrap();

    let area = result.footprint_area();
    // 200 × 150 = 30000
    assert!(
        (area - 30000.0).abs() < 100.0,
        "Footprint area {} expected ~30000.0",
        area
    );
}

// ============================================================================
// Test: terrain bounds computation
// ============================================================================

#[test]
fn test_terrain_bounds_from_vertices() {
    let verts = vec![
        [0.0, 5.0, 0.0],
        [100.0, 10.0, 0.0],
        [100.0, 15.0, 100.0],
        [0.0, 20.0, 100.0],
    ];
    let bounds = TerrainBounds::from_vertices(&verts).unwrap();
    assert!((bounds.min[0] - 0.0).abs() < 1e-6);
    assert!((bounds.min[1] - 5.0).abs() < 1e-6);
    assert!((bounds.min[2] - 0.0).abs() < 1e-6);
    assert!((bounds.max[0] - 100.0).abs() < 1e-6);
    assert!((bounds.max[1] - 20.0).abs() < 1e-6);
    assert!((bounds.max[2] - 100.0).abs() < 1e-6);
    assert!((bounds.width() - 100.0).abs() < 1e-6);
    assert!((bounds.depth() - 100.0).abs() < 1e-6);
    assert!((bounds.footprint_area() - 10000.0).abs() < 1e-3);
}

#[test]
fn test_terrain_bounds_empty() {
    let result = TerrainBounds::from_vertices(&[]);
    assert!(result.is_none());
}

// ============================================================================
// Test: rasterization with zero extent errors
// ============================================================================

#[test]
fn test_rasterize_empty_mesh_list() {
    let result = rasterize_terrain_meshes(&[], 16);
    assert!(result.is_err());
}

#[test]
fn test_rasterize_invalid_resolution() {
    let mesh = make_flat_quad("Terrain", 0.0, 0.0, 100.0, 100.0, 5.0);
    let result = rasterize_terrain_meshes(&[mesh], 1);
    assert!(result.is_err());
}
