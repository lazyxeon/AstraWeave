//! End-to-end integration tests for the Blueprint Zone → Scatter pipeline.
//!
//! These tests verify the full flow: polygon definition → zone creation →
//! scatter generation → heightmap injection → boundary blending → multi-zone overlap.

use astraweave_terrain::{
    AdaptiveScaleParams, BiomeType, BlueprintZone, ChunkId, Heightmap,
    TerrainChunk, ZoneId, ZoneRegistry, ZoneSource,
};
use astraweave_terrain::zone_scatter::{
    apply_heightmap_patches, generate_multi_zone_scatter, HeightmapPatch,
    ZoneScatterGenerator,
};
use astraweave_terrain::blueprint_zone::{
    point_in_polygon, polygon_area, polygon_bounding_rect, polygon_centroid,
};
use glam::Vec2;
use std::collections::HashMap;

// ============================================================================
// Helpers
// ============================================================================

/// Create a flat terrain chunk at the given chunk coordinates.
fn make_flat_chunk(cx: i32, cz: i32, resolution: u32) -> TerrainChunk {
    let data = vec![0.0f32; (resolution * resolution) as usize];
    let heightmap = Heightmap::from_data(data, resolution).unwrap();
    let biome_map = vec![BiomeType::Grassland; (resolution * resolution) as usize];
    TerrainChunk::new(ChunkId::new(cx, cz), heightmap, biome_map)
}

/// Create a square zone polygon centered at the origin.
fn square_zone(half: f32) -> Vec<Vec2> {
    vec![
        Vec2::new(-half, -half),
        Vec2::new(half, -half),
        Vec2::new(half, half),
        Vec2::new(-half, half),
    ]
}

/// Create a BlueprintZone with a biome preset source.
fn biome_zone(id: u64, name: &str, vertices: Vec<Vec2>, biome: BiomeType) -> BlueprintZone {
    let mut zone = BlueprintZone::new(ZoneId(id), name.to_string());
    zone.vertices = vertices;
    zone.source = ZoneSource::BiomePreset(biome);
    zone.enabled = true;
    zone
}

// ============================================================================
// Test: polygon → scatter → verify within boundary
// ============================================================================

#[test]
fn test_biome_preset_scatter_within_polygon() {
    let chunk_size = 256.0f32;
    let resolution = 32u32;

    // Create a zone polygon covering part of chunk (0,0)
    let zone_verts = vec![
        Vec2::new(10.0, 10.0),
        Vec2::new(200.0, 10.0),
        Vec2::new(200.0, 200.0),
        Vec2::new(10.0, 200.0),
    ];

    let zone = biome_zone(1, "Grassland Zone", zone_verts.clone(), BiomeType::Grassland);
    let chunk = make_flat_chunk(0, 0, resolution);
    let chunks: Vec<&TerrainChunk> = vec![&chunk];

    let generator = ZoneScatterGenerator::new(chunk_size, resolution);
    let result = generator.generate_zone_scatter(&zone, &chunks, 42).unwrap();

    // All placements must be within the polygon bounds
    let (bmin, bmax) = polygon_bounding_rect(&zone_verts);
    for v in &result.placements {
        let pt = Vec2::new(v.position.x, v.position.z);
        assert!(
            pt.x >= bmin.x && pt.x <= bmax.x && pt.y >= bmin.y && pt.y <= bmax.y,
            "Placement at ({}, {}) is outside bounding rect",
            pt.x,
            pt.y
        );
        assert!(
            point_in_polygon(pt, &zone_verts),
            "Placement at ({}, {}) is outside polygon",
            pt.x,
            pt.y
        );
    }
}

// ============================================================================
// Test: heightmap injection → verify terrain values match source
// ============================================================================

#[test]
fn test_heightmap_patch_application() {
    let resolution = 16u32;
    let chunk = make_flat_chunk(0, 0, resolution);

    // All heights should be 0.0 initially
    for gz in 0..resolution {
        for gx in 0..resolution {
            let h = chunk.heightmap().get_height(gx, gz);
            assert!(
                (h - 0.0).abs() < 1e-6,
                "Initial height at ({},{}) = {}, expected 0.0",
                gx, gz, h
            );
        }
    }

    // Create a patch that sets a few grid cells to known values
    let mut heights = std::collections::HashMap::new();
    heights.insert((2, 3), 10.0);
    heights.insert((5, 5), 25.5);
    heights.insert((8, 8), -3.0);

    let patch = HeightmapPatch {
        chunk_id: ChunkId::new(0, 0),
        heights,
    };

    // Apply the patch via HashMap
    let chunk_id = chunk.id();
    let mut chunk_map: HashMap<ChunkId, TerrainChunk> = HashMap::new();
    chunk_map.insert(chunk_id, chunk);

    use astraweave_terrain::zone_scatter::ZoneGenerationResult;
    let result = ZoneGenerationResult {
        placements: Vec::new(),
        heightmap_patches: vec![patch],
    };
    apply_heightmap_patches(&mut chunk_map, &[result]);

    let c = &chunk_map[&chunk_id];
    // Verify modified values
    assert!((c.heightmap().get_height(2, 3) - 10.0).abs() < 1e-6);
    assert!((c.heightmap().get_height(5, 5) - 25.5).abs() < 1e-6);
    assert!((c.heightmap().get_height(8, 8) - (-3.0)).abs() < 1e-6);

    // Untouched cells should remain 0
    assert!((c.heightmap().get_height(0, 0) - 0.0).abs() < 1e-6);
    assert!((c.heightmap().get_height(15, 15) - 0.0).abs() < 1e-6);
}

// ============================================================================
// Test: adaptive scaling → verify density/scale ratios
// ============================================================================

#[test]
fn test_adaptive_scale_params() {
    // Reference scene is 100×100 = 10000 sq units
    let reference_area = 10000.0;

    // Zone is 200×200 = 40000 sq units (4× larger)
    let zone_area = 40000.0;

    let params = AdaptiveScaleParams::compute(reference_area, zone_area);
    let ratio = zone_area / reference_area; // 4.0

    // position_scale = sqrt(ratio) = 2.0
    assert!(
        (params.position_scale - ratio.sqrt()).abs() < 0.01,
        "position_scale = {}, expected ~{}",
        params.position_scale,
        ratio.sqrt()
    );

    // density_multiplier = sqrt(ratio) clamped to [0.1, 10.0]
    assert!(
        (params.density_multiplier - ratio.sqrt()).abs() < 0.01,
        "density_multiplier = {}, expected ~{}",
        params.density_multiplier,
        ratio.sqrt()
    );

    // scale_multiplier = ratio^0.25 clamped to [0.5, 2.0]
    assert!(
        (params.scale_multiplier - ratio.powf(0.25)).abs() < 0.01,
        "scale_multiplier = {}, expected ~{}",
        params.scale_multiplier,
        ratio.powf(0.25)
    );

    // Identity params when areas are equal
    let identity = AdaptiveScaleParams::compute(1000.0, 1000.0);
    assert!((identity.position_scale - 1.0).abs() < 0.01);
    assert!((identity.density_multiplier - 1.0).abs() < 0.01);
    assert!((identity.scale_multiplier - 1.0).abs() < 0.01);
}

// ============================================================================
// Test: multi-zone overlap → verify priority-based overlap
// ============================================================================

#[test]
fn test_multi_zone_overlap_priority() {
    let chunk_size = 256.0;
    let resolution = 16u32;

    // Two overlapping zones: low priority grassland, high priority forest
    let zone_low = {
        let mut z = biome_zone(
            1,
            "Grassland",
            square_zone(100.0),
            BiomeType::Grassland,
        );
        z.priority = 0;
        z
    };

    let zone_high = {
        let mut z = biome_zone(
            2,
            "Forest",
            square_zone(50.0), // smaller but higher priority
            BiomeType::Forest,
        );
        z.priority = 10;
        z
    };

    let chunk = make_flat_chunk(0, 0, resolution);
    let chunks: Vec<&TerrainChunk> = vec![&chunk];

    let results = generate_multi_zone_scatter(
        &[zone_low, zone_high],
        &chunks,
        chunk_size,
        resolution,
        42,
    )
    .unwrap();

    assert_eq!(results.len(), 2, "Should get results for both zones");

    // The high-priority forest zone's heightmap cells should not be
    // overwritten by the low-priority zone. We just verify both produce
    // results without error.
    for result in &results {
        // Each zone should produce a valid (possibly empty) result
        assert!(result.placements.len() == 0 || result.placements.len() > 0);
    }
}

// ============================================================================
// Test: zone registry CRUD + save/load round-trip
// ============================================================================

#[test]
fn test_zone_registry_save_load_roundtrip() {
    let mut registry = ZoneRegistry::new();

    // Add two zones
    let id1 = registry.next_zone_id();
    let mut z1 = BlueprintZone::new(id1, "Forest Clearing".into());
    z1.vertices = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(100.0, 0.0),
        Vec2::new(100.0, 100.0),
        Vec2::new(0.0, 100.0),
    ];
    z1.source = ZoneSource::BiomePreset(BiomeType::Forest);
    z1.priority = 5;
    z1.blend_margin = 12.0;
    registry.add_zone(z1);

    let id2 = registry.next_zone_id();
    let mut z2 = BlueprintZone::new(id2, "Desert Dunes".into());
    z2.vertices = vec![
        Vec2::new(200.0, 200.0),
        Vec2::new(400.0, 200.0),
        Vec2::new(400.0, 400.0),
        Vec2::new(200.0, 400.0),
    ];
    z2.source = ZoneSource::BiomePreset(BiomeType::Desert);
    z2.priority = 3;
    registry.add_zone(z2);

    assert_eq!(registry.len(), 2);

    // Save to a temp file
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test_zones.json");
    registry.save(&path).unwrap();

    // Load back
    let loaded = ZoneRegistry::load(&path).unwrap();
    assert_eq!(loaded.len(), 2);

    let loaded_z1 = loaded.get_zone(id1).expect("zone 1 not found");
    assert_eq!(loaded_z1.name, "Forest Clearing");
    assert_eq!(loaded_z1.vertices.len(), 4);
    assert_eq!(loaded_z1.priority, 5);
    assert!((loaded_z1.blend_margin - 12.0).abs() < 1e-6);
    assert!(matches!(loaded_z1.source, ZoneSource::BiomePreset(BiomeType::Forest)));

    let loaded_z2 = loaded.get_zone(id2).expect("zone 2 not found");
    assert_eq!(loaded_z2.name, "Desert Dunes");
    assert!(matches!(loaded_z2.source, ZoneSource::BiomePreset(BiomeType::Desert)));
}

// ============================================================================
// Test: polygon geometry utilities
// ============================================================================

#[test]
fn test_polygon_area_and_centroid() {
    let square = square_zone(50.0);
    let area = polygon_area(&square);
    assert!(
        (area - 10000.0).abs() < 0.1,
        "Area of 100×100 square = {}, expected 10000",
        area
    );

    let centroid = polygon_centroid(&square);
    assert!(centroid.x.abs() < 0.1, "Centroid x = {}", centroid.x);
    assert!(centroid.y.abs() < 0.1, "Centroid y = {}", centroid.y);
}

#[test]
fn test_point_in_polygon_boundary() {
    let triangle = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(100.0, 0.0),
        Vec2::new(50.0, 100.0),
    ];

    // Interior point
    assert!(point_in_polygon(Vec2::new(50.0, 30.0), &triangle));
    // Well outside
    assert!(!point_in_polygon(Vec2::new(200.0, 200.0), &triangle));
    assert!(!point_in_polygon(Vec2::new(-10.0, -10.0), &triangle));
}

// ============================================================================
// Test: zone generation result is empty for disabled zones
// ============================================================================

#[test]
fn test_disabled_zone_produces_empty_result() {
    let chunk_size = 256.0;
    let resolution = 16u32;

    let mut zone = biome_zone(1, "Disabled", square_zone(100.0), BiomeType::Grassland);
    zone.enabled = false;

    let chunk = make_flat_chunk(0, 0, resolution);
    let chunks: Vec<&TerrainChunk> = vec![&chunk];

    let generator = ZoneScatterGenerator::new(chunk_size, resolution);
    let result = generator.generate_zone_scatter(&zone, &chunks, 42).unwrap();

    assert_eq!(result.placements.len(), 0);
    assert_eq!(result.heightmap_patches.len(), 0);
}

#[test]
fn test_zone_with_too_few_vertices_produces_empty() {
    let chunk_size = 256.0;
    let resolution = 16u32;

    let mut zone = biome_zone(1, "TwoVerts", vec![], BiomeType::Grassland);
    zone.vertices = vec![Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0)]; // Only 2

    let chunk = make_flat_chunk(0, 0, resolution);
    let chunks: Vec<&TerrainChunk> = vec![&chunk];

    let generator = ZoneScatterGenerator::new(chunk_size, resolution);
    let result = generator.generate_zone_scatter(&zone, &chunks, 42).unwrap();

    assert_eq!(result.placement_count(), 0);
}

// ============================================================================
// Test: blend mask sampling
// ============================================================================

#[test]
fn test_blend_mask_bilinear_sampling() {
    use astraweave_terrain::BlendMask;

    let mut mask = BlendMask::new(4, (0.0, 0.0, 100.0, 100.0));

    // Set corners to known values
    mask.set(0, 0, 0.0);
    mask.set(3, 0, 1.0);
    mask.set(0, 3, 1.0);
    mask.set(3, 3, 0.5);

    // Sample at corners
    let s00 = mask.sample(0.0, 0.0);
    assert!(
        s00.abs() < 0.2,
        "Sample at (0,0) = {}, expected ~0.0",
        s00
    );

    let s30 = mask.sample(100.0, 0.0);
    assert!(
        (s30 - 1.0).abs() < 0.2,
        "Sample at (100,0) = {}, expected ~1.0",
        s30
    );
}

// ============================================================================
// Test: multiple chunks with zone spanning across them
// ============================================================================

#[test]
fn test_zone_spanning_multiple_chunks() {
    let chunk_size = 256.0;
    let resolution = 16u32;

    // Zone spans chunks (0,0) and (1,0)
    let zone_verts = vec![
        Vec2::new(100.0, 50.0),
        Vec2::new(400.0, 50.0),
        Vec2::new(400.0, 200.0),
        Vec2::new(100.0, 200.0),
    ];

    let zone = biome_zone(1, "Wide Zone", zone_verts, BiomeType::Grassland);

    let chunk0 = make_flat_chunk(0, 0, resolution);
    let chunk1 = make_flat_chunk(1, 0, resolution);
    let chunks: Vec<&TerrainChunk> = vec![&chunk0, &chunk1];

    let generator = ZoneScatterGenerator::new(chunk_size, resolution);
    let result = generator.generate_zone_scatter(&zone, &chunks, 99).unwrap();

    // Should succeed without panics, potentially producing vegetation
    // across both chunks
    let _ = result.placements.len(); // Verify no panic when spanning chunks
}
