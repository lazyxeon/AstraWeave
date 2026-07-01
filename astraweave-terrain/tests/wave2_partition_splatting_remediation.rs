//! Wave 2 Remediation Tests — partition_integration.rs (99 mutants) + texture_splatting.rs (152 mutants)
//!
//! Targets: shards 17-18, 20-21
//! Focus: PartitionCoord arithmetic, VoxelPartitionConfig defaults, chunk mapping,
//!        TerrainMaterial presets, SplatWeights normalization/dominant_layer,
//!        SplatRule evaluation, TriplanarWeights, SplatMapGenerator.

use astraweave_terrain::*;
use glam::{Vec3, Vec4};

// ============================================================================
// PartitionCoord
// ============================================================================

#[test]
fn partition_coord_new() {
    let p = PartitionCoord::new(3, -2, 5);
    assert_eq!(p.x, 3);
    assert_eq!(p.y, -2);
    assert_eq!(p.z, 5);
}

#[test]
fn partition_coord_from_world_pos_positive() {
    let pos = Vec3::new(300.0, 128.0, 500.0);
    let p = PartitionCoord::from_world_pos(pos, 256.0);
    // 300/256 = 1.17 → floor = 1, 128/256 = 0.5 → floor = 0, 500/256 = 1.95 → floor = 1
    assert_eq!(p.x, 1);
    assert_eq!(p.y, 0);
    assert_eq!(p.z, 1);
}

#[test]
fn partition_coord_from_world_pos_negative() {
    let pos = Vec3::new(-100.0, -50.0, -300.0);
    let p = PartitionCoord::from_world_pos(pos, 256.0);
    // -100/256 = -0.39 → floor = -1
    assert_eq!(p.x, -1);
    assert_eq!(p.y, -1);
    assert_eq!(p.z, -2);
}

#[test]
fn partition_coord_to_world_center() {
    let p = PartitionCoord::new(0, 0, 0);
    let center = p.to_world_center(256.0);
    // (0 + 0.5) * 256 = 128
    assert!((center.x - 128.0).abs() < 1e-4);
    assert!((center.y - 128.0).abs() < 1e-4);
    assert!((center.z - 128.0).abs() < 1e-4);
}

#[test]
fn partition_coord_to_world_center_nonzero() {
    let p = PartitionCoord::new(2, 1, -1);
    let center = p.to_world_center(256.0);
    assert!((center.x - 640.0).abs() < 1e-4); // (2 + 0.5) * 256 = 640
    assert!((center.y - 384.0).abs() < 1e-4); // (1 + 0.5) * 256 = 384
    assert!((center.z - (-128.0)).abs() < 1e-4); // (-1 + 0.5) * 256 = -128
}

#[test]
fn partition_coord_to_world_min() {
    let p = PartitionCoord::new(1, 0, 2);
    let min = p.to_world_min(256.0);
    assert!((min.x - 256.0).abs() < 1e-4);
    assert!((min.y - 0.0).abs() < 1e-4);
    assert!((min.z - 512.0).abs() < 1e-4);
}

#[test]
fn partition_coord_get_voxel_chunks_count() {
    let p = PartitionCoord::new(0, 0, 0);
    let chunks = p.get_voxel_chunks(256.0);
    // 256 / 32 = 8 per axis → 8^3 = 512
    assert_eq!(chunks.len(), 512);
}

#[test]
fn partition_coord_get_voxel_chunks_base_offset() {
    let p = PartitionCoord::new(1, 0, 0);
    let chunks = p.get_voxel_chunks(256.0);
    // Base = 1 * 8 = 8
    assert!(chunks.iter().any(|c| c.x == 8 && c.y == 0 && c.z == 0));
    // Should NOT have chunks starting at 0
    assert!(!chunks.iter().any(|c| c.x == 0));
}

#[test]
fn partition_coord_from_chunk_coord() {
    let chunk = ChunkCoord::new(16, 8, 24);
    let partition: PartitionCoord = chunk.into();
    // 16/8=2, 8/8=1, 24/8=3
    assert_eq!(partition.x, 2);
    assert_eq!(partition.y, 1);
    assert_eq!(partition.z, 3);
}

#[test]
fn partition_coord_from_chunk_coord_negative() {
    let chunk = ChunkCoord::new(-1, -9, -16);
    let partition: PartitionCoord = chunk.into();
    // div_euclid(-1, 8) = -1, div_euclid(-9, 8) = -2, div_euclid(-16, 8) = -2
    assert_eq!(partition.x, -1);
    assert_eq!(partition.y, -2);
    assert_eq!(partition.z, -2);
}

// ============================================================================
// VoxelPartitionConfig
// ============================================================================

#[test]
fn voxel_partition_config_defaults() {
    let config = VoxelPartitionConfig::default();
    assert!((config.cell_size - 256.0).abs() < 1e-4);
    assert_eq!(config.memory_budget, 500_000_000);
    assert!(config.auto_mesh);
    assert!((config.lod_distances[0] - 100.0).abs() < 1e-4);
    assert!((config.lod_distances[1] - 250.0).abs() < 1e-4);
    assert!((config.lod_distances[2] - 500.0).abs() < 1e-4);
    assert!((config.lod_distances[3] - 1000.0).abs() < 1e-4);
}

// ============================================================================
// VoxelPartitionManager
// ============================================================================

#[tokio::test]
async fn partition_manager_new_empty() {
    let mgr = VoxelPartitionManager::new(VoxelPartitionConfig::default());
    assert_eq!(mgr.get_stats().active_cells, 0);
    assert_eq!(mgr.get_stats().loaded_chunks, 0);
    assert_eq!(mgr.get_stats().meshed_chunks, 0);
    assert!(mgr.get_active_cells().is_empty());
}

#[tokio::test]
async fn partition_manager_with_cell_size() {
    let mgr = VoxelPartitionManager::with_cell_size(128.0);
    assert!(mgr.get_active_cells().is_empty());
}

#[tokio::test]
async fn partition_manager_with_memory_budget() {
    let mgr = VoxelPartitionManager::with_memory_budget(1_000_000);
    assert_eq!(mgr.get_stats().loaded_chunks, 0);
}

#[tokio::test]
async fn partition_manager_activate_cell() {
    let mut mgr = VoxelPartitionManager::new(VoxelPartitionConfig::default());
    let cell = PartitionCoord::new(0, 0, 0);
    let loaded = mgr.activate_cell(cell).await.unwrap();
    assert!(!loaded.is_empty());
    assert!(mgr.get_active_cells().contains(&cell));
}

#[tokio::test]
async fn partition_manager_duplicate_activate() {
    let mut mgr = VoxelPartitionManager::new(VoxelPartitionConfig::default());
    let cell = PartitionCoord::new(0, 0, 0);
    mgr.activate_cell(cell).await.unwrap();
    // Second activation should return empty (already active)
    let result = mgr.activate_cell(cell).await.unwrap();
    assert!(
        result.is_empty(),
        "Duplicate activation should return empty"
    );
}

#[tokio::test]
async fn partition_manager_deactivate_cell() {
    let mut mgr = VoxelPartitionManager::new(VoxelPartitionConfig::default());
    let cell = PartitionCoord::new(0, 0, 0);
    mgr.activate_cell(cell).await.unwrap();
    let unloaded = mgr.deactivate_cell(cell).await.unwrap();
    assert!(!unloaded.is_empty());
    assert!(!mgr.get_active_cells().contains(&cell));
}

#[tokio::test]
async fn partition_manager_deactivate_inactive() {
    let mut mgr = VoxelPartitionManager::new(VoxelPartitionConfig::default());
    let cell = PartitionCoord::new(99, 99, 99);
    let result = mgr.deactivate_cell(cell).await.unwrap();
    assert!(
        result.is_empty(),
        "Deactivating inactive cell returns empty"
    );
}

#[tokio::test]
async fn partition_manager_events() {
    let mut mgr = VoxelPartitionManager::new(VoxelPartitionConfig::default());
    let cell = PartitionCoord::new(0, 0, 0);
    mgr.activate_cell(cell).await.unwrap();
    let events = mgr.drain_events();
    assert!(!events.is_empty(), "Should have activation events");
}

#[tokio::test]
async fn partition_manager_is_chunk_loaded() {
    let mut mgr = VoxelPartitionManager::new(VoxelPartitionConfig::default());
    let cell = PartitionCoord::new(0, 0, 0);
    mgr.activate_cell(cell).await.unwrap();

    let loaded = mgr.is_chunk_loaded(ChunkCoord::new(0, 0, 0)).await;
    assert!(loaded, "Chunk in activated cell should be loaded");
}

// ============================================================================
// TerrainMaterial presets (value pinning for mutation detection)
// ============================================================================

#[test]
fn terrain_material_default_values() {
    let mat = TerrainMaterial::default();
    assert_eq!(mat.id, 0);
    assert_eq!(mat.name, "Default");
    assert!((mat.uv_scale - 1.0).abs() < 1e-6);
    assert!((mat.blend_sharpness - 2.0).abs() < 1e-6);
    assert!((mat.triplanar_sharpness - 4.0).abs() < 1e-6);
}

#[test]
fn terrain_material_grass_preset() {
    let mat = TerrainMaterial::grass();
    assert_eq!(mat.id, 0);
    assert_eq!(mat.name, "Grass");
    assert!((mat.uv_scale - 4.0).abs() < 1e-6);
    assert!((mat.blend_sharpness - 2.0).abs() < 1e-6);
    assert!((mat.triplanar_sharpness - 4.0).abs() < 1e-6);
}

#[test]
fn terrain_material_rock_preset() {
    let mat = TerrainMaterial::rock();
    assert_eq!(mat.id, 1);
    assert_eq!(mat.name, "Rock");
    assert!((mat.uv_scale - 2.0).abs() < 1e-6);
    assert!((mat.blend_sharpness - 4.0).abs() < 1e-6);
    assert!((mat.triplanar_sharpness - 8.0).abs() < 1e-6);
}

#[test]
fn terrain_material_sand_preset() {
    let mat = TerrainMaterial::sand();
    assert_eq!(mat.id, 2);
    assert_eq!(mat.name, "Sand");
    assert!((mat.uv_scale - 8.0).abs() < 1e-6);
    assert!((mat.blend_sharpness - 1.5).abs() < 1e-6);
}

#[test]
fn terrain_material_snow_preset() {
    let mat = TerrainMaterial::snow();
    assert_eq!(mat.id, 3);
    assert_eq!(mat.name, "Snow");
    assert!((mat.uv_scale - 6.0).abs() < 1e-6);
    assert!((mat.blend_sharpness - 1.0).abs() < 1e-6);
}

#[test]
fn terrain_material_dirt_preset() {
    let mat = TerrainMaterial::dirt();
    assert_eq!(mat.id, 4);
    assert_eq!(mat.name, "Dirt");
    assert!((mat.uv_scale - 4.0).abs() < 1e-6);
    assert!((mat.blend_sharpness - 2.5).abs() < 1e-6);
}

// ============================================================================
// SplatWeights
// ============================================================================

#[test]
fn splat_weights_from_weights_normalization() {
    let sw = SplatWeights::from_weights(&[0.5, 0.3, 0.2]);
    let total = sw.weights_0.x
        + sw.weights_0.y
        + sw.weights_0.z
        + sw.weights_0.w
        + sw.weights_1.x
        + sw.weights_1.y
        + sw.weights_1.z
        + sw.weights_1.w;
    assert!(
        (total - 1.0).abs() < 0.001,
        "Weights should sum to 1.0, got {total}"
    );
}

#[test]
fn splat_weights_from_weights_zero_fallback() {
    let sw = SplatWeights::from_weights(&[0.0, 0.0, 0.0]);
    // All zero → documented fallback: weights_0.y = 1.0 (layer 1 / sand),
    // texture_splatting.rs:169-170 ("avoids grass bleed in non-grassland biomes").
    assert!(
        (sw.weights_0.y - 1.0).abs() < 1e-6,
        "Zero weights fallback to layer 1 (sand)"
    );
}

#[test]
fn splat_weights_from_weights_8_layers() {
    let w = [0.1, 0.1, 0.1, 0.1, 0.2, 0.2, 0.1, 0.1];
    let sw = SplatWeights::from_weights(&w);
    let total = sw.weights_0.x
        + sw.weights_0.y
        + sw.weights_0.z
        + sw.weights_0.w
        + sw.weights_1.x
        + sw.weights_1.y
        + sw.weights_1.z
        + sw.weights_1.w;
    assert!((total - 1.0).abs() < 0.001);
    // Layers 4 and 5 had highest raw weight, verify they're in weights_1.x and .y
    assert!(sw.weights_1.x > sw.weights_0.x);
}

#[test]
fn splat_weights_get_weight_all_indices() {
    let sw = SplatWeights::from_weights(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    // After normalization, each should be index_value / 36.0
    for i in 0..8 {
        let w = sw.get_weight(i);
        assert!(w > 0.0, "Layer {i} weight should be > 0");
    }
    // Out of bounds → 0
    assert_eq!(sw.get_weight(8), 0.0);
    assert_eq!(sw.get_weight(100), 0.0);
}

#[test]
fn splat_weights_dominant_layer() {
    let sw = SplatWeights::from_weights(&[0.1, 0.6, 0.3]);
    assert_eq!(sw.dominant_layer(), 1, "Layer 1 should be dominant");
}

#[test]
fn splat_weights_dominant_layer_in_second_half() {
    let sw = SplatWeights::from_weights(&[0.0, 0.0, 0.0, 0.0, 0.1, 0.1, 0.1, 0.7]);
    assert_eq!(sw.dominant_layer(), 7, "Layer 7 should be dominant");
}

// ============================================================================
// SplatConfig
// ============================================================================

#[test]
fn splat_config_defaults() {
    let config = SplatConfig::default();
    assert!(config.triplanar_enabled);
    assert!((config.triplanar_slope_threshold - 45.0).abs() < 1e-4);
    assert!(config.height_blending_enabled);
    assert!((config.height_blend_contrast - 8.0).abs() < 1e-4);
    assert!((config.rock_slope_threshold - 35.0).abs() < 1e-4);
    assert!(config.detail_normal_enabled);
    assert!((config.detail_uv_scale - 16.0).abs() < 1e-4);
    assert!((config.snow_height_threshold - 150.0).abs() < 1e-4);
    assert!((config.snow_slope_fade - 30.0).abs() < 1e-4);
}

// ============================================================================
// SplatRule
// ============================================================================

#[test]
fn splat_rule_default_values() {
    let r = SplatRule::default();
    assert_eq!(r.material_id, 0);
    assert_eq!(r.min_height, f32::MIN);
    assert_eq!(r.max_height, f32::MAX);
    assert!((r.min_slope - 0.0).abs() < 1e-6);
    assert!((r.max_slope - 90.0).abs() < 1e-4);
    assert_eq!(r.priority, 0);
    assert!((r.weight - 1.0).abs() < 1e-6);
    assert!((r.height_falloff - 0.01).abs() < 1e-6);
    assert!((r.slope_falloff - 0.05).abs() < 1e-6);
}

#[test]
fn splat_rule_grass_preset() {
    let r = SplatRule::grass();
    assert_eq!(r.material_id, 0);
    assert!((r.min_height - 0.0).abs() < 1e-6);
    assert!((r.max_height - 100.0).abs() < 1e-4);
    assert!((r.min_slope - 0.0).abs() < 1e-6);
    assert!((r.max_slope - 30.0).abs() < 1e-4);
    assert_eq!(r.priority, 10);
}

#[test]
fn splat_rule_rock_preset() {
    let r = SplatRule::rock();
    assert_eq!(r.material_id, 7);
    assert!((r.min_slope - 35.0).abs() < 1e-4);
    assert!((r.max_slope - 90.0).abs() < 1e-4);
    assert_eq!(r.priority, 20);
}

#[test]
fn splat_rule_sand_preset() {
    let r = SplatRule::sand();
    assert_eq!(r.material_id, 1);
    assert!((r.min_height - (-5.0)).abs() < 1e-4);
    assert!((r.max_height - 8.0).abs() < 1e-4);
    assert_eq!(r.priority, 15);
    assert!((r.weight - 2.0).abs() < 1e-6, "Sand base weight = 2.0");
}

#[test]
fn splat_rule_snow_preset() {
    let r = SplatRule::snow();
    assert_eq!(r.material_id, 4);
    assert!((r.min_height - 120.0).abs() < 1e-4);
    assert_eq!(r.max_height, f32::MAX);
    assert_eq!(r.priority, 25);
}

#[test]
fn splat_rule_evaluate_in_range() {
    let r = SplatRule::grass();
    let w = r.evaluate(50.0, 15.0); // Perfect grass conditions
    assert!(w > 0.9, "In-range grass should have weight > 0.9, got {w}");
}

#[test]
fn splat_rule_evaluate_above_max_height() {
    let r = SplatRule::grass(); // max_height = 100.0
    let w_in = r.evaluate(50.0, 15.0);
    let w_above = r.evaluate(150.0, 15.0);
    assert!(w_above < w_in, "Above max_height should reduce weight");
}

#[test]
fn splat_rule_evaluate_below_min_height() {
    let r = SplatRule::sand(); // min_height = -5.0
    let w_in = r.evaluate(2.0, 10.0);
    let w_below = r.evaluate(-50.0, 10.0);
    assert!(w_below < w_in, "Below min_height should reduce weight");
}

#[test]
fn splat_rule_evaluate_above_max_slope() {
    let r = SplatRule::grass(); // max_slope = 30.0
    let w_flat = r.evaluate(50.0, 10.0);
    let w_steep = r.evaluate(50.0, 60.0);
    assert!(w_steep < w_flat, "Above max_slope should reduce weight");
}

#[test]
fn splat_rule_evaluate_below_min_slope() {
    let r = SplatRule::rock(); // min_slope = 35.0
    let w_steep = r.evaluate(50.0, 60.0);
    let w_flat = r.evaluate(50.0, 5.0);
    assert!(w_flat < w_steep, "Below min_slope should reduce weight");
}

#[test]
fn splat_rule_evaluate_far_out_zero() {
    let r = SplatRule::grass(); // max_height=100, height_falloff=0.02
                                // At height 200: (200-100)*0.02 = 2.0. 1.0 - 2.0 = -1.0 → max(0.0) = 0.0
    let w = r.evaluate(200.0, 15.0);
    assert!((w - 0.0).abs() < 1e-6, "Far outside range should be 0");
}

// ============================================================================
// TriplanarWeights
// ============================================================================

#[test]
fn triplanar_weights_flat_surface() {
    let tw = TriplanarWeights::from_normal(Vec3::Y, 4.0);
    assert!(tw.y > 0.99, "Flat surface should have Y-dominant weight");
    assert!(tw.x < 0.01);
    assert!(tw.z < 0.01);
}

#[test]
fn triplanar_weights_vertical_cliff_x() {
    let tw = TriplanarWeights::from_normal(Vec3::X, 4.0);
    assert!(tw.x > 0.99, "X-facing cliff should have X-dominant weight");
}

#[test]
fn triplanar_weights_vertical_cliff_z() {
    let tw = TriplanarWeights::from_normal(Vec3::Z, 4.0);
    assert!(tw.z > 0.99, "Z-facing cliff should have Z-dominant weight");
}

#[test]
fn triplanar_weights_45deg_slope() {
    let normal = Vec3::new(0.707, 0.707, 0.0).normalize();
    let tw = TriplanarWeights::from_normal(normal, 4.0);
    assert!(
        (tw.x - tw.y).abs() < 0.05,
        "45° slope should have ~equal X and Y"
    );
}

#[test]
fn triplanar_weights_sum_to_one() {
    let tw = TriplanarWeights::from_normal(Vec3::new(1.0, 2.0, 3.0).normalize(), 4.0);
    let total = tw.x + tw.y + tw.z;
    assert!(
        (total - 1.0).abs() < 0.01,
        "Weights should sum to ~1.0, got {total}"
    );
}

#[test]
fn triplanar_should_use_triplanar_flat() {
    let tw = TriplanarWeights::from_normal(Vec3::Y, 4.0);
    // Flat surface → Y weight high → should NOT use triplanar
    assert!(!tw.should_use_triplanar(0.5));
}

#[test]
fn triplanar_should_use_triplanar_steep() {
    let tw = TriplanarWeights::from_normal(Vec3::X, 4.0);
    // Steep cliff → Y weight low → SHOULD use triplanar
    assert!(tw.should_use_triplanar(0.5));
}

// ============================================================================
// SplatMapGenerator
// ============================================================================

#[test]
fn splat_generator_with_default_rules_grass() {
    let gen = SplatMapGenerator::with_default_rules(SplatConfig::default(), 42);
    let w = gen.calculate_weights(50.0, Vec3::Y);
    assert_eq!(
        w.dominant_layer(),
        0,
        "Flat lowland should be grass (layer 0)"
    );
}

#[test]
fn splat_generator_with_default_rules_rock() {
    let gen = SplatMapGenerator::with_default_rules(SplatConfig::default(), 42);
    let steep_normal = Vec3::new(0.3, 0.3, 0.9).normalize();
    let w = gen.calculate_weights(50.0, steep_normal);
    assert_eq!(
        w.dominant_layer(),
        7,
        "Steep slope should be rock (layer 7)"
    );
}

#[test]
fn splat_generator_with_default_rules_sand() {
    let gen = SplatMapGenerator::with_default_rules(SplatConfig::default(), 42);
    let w = gen.calculate_weights(2.0, Vec3::Y);
    assert_eq!(
        w.dominant_layer(),
        1,
        "Low flat area should be sand (layer 1)"
    );
}

#[test]
fn splat_generator_with_default_rules_snow() {
    let gen = SplatMapGenerator::with_default_rules(SplatConfig::default(), 42);
    let w = gen.calculate_weights(180.0, Vec3::Y);
    assert_eq!(
        w.dominant_layer(),
        4,
        "High flat area should be snow (layer 4)"
    );
}

#[test]
fn splat_generator_add_rule_sorts_by_priority() {
    let mut gen = SplatMapGenerator::new(SplatConfig::default(), 42);
    gen.add_rule(SplatRule {
        priority: 5,
        ..SplatRule::default()
    });
    gen.add_rule(SplatRule {
        priority: 20,
        ..SplatRule::default()
    });
    gen.add_rule(SplatRule {
        priority: 10,
        ..SplatRule::default()
    });
    // After sorting by priority (descending), weights calculation should work
    let w = gen.calculate_weights(50.0, Vec3::Y);
    let total = w.weights_0.x
        + w.weights_0.y
        + w.weights_0.z
        + w.weights_0.w
        + w.weights_1.x
        + w.weights_1.y
        + w.weights_1.z
        + w.weights_1.w;
    assert!(
        (total - 1.0).abs() < 0.01,
        "Normalized total should be ~1.0"
    );
}

#[test]
fn splat_generator_generate_splat_map_size() {
    let gen = SplatMapGenerator::with_default_rules(SplatConfig::default(), 42);
    let heights = vec![50.0, 60.0, 70.0, 80.0];
    let normals = vec![Vec3::Y; 4];
    let map = gen.generate_splat_map(&heights, &normals, 2);
    assert_eq!(map.len(), 4);
}

#[test]
fn splat_generator_generate_splat_map_normalized() {
    let gen = SplatMapGenerator::with_default_rules(SplatConfig::default(), 42);
    let heights = vec![0.0, 50.0, 130.0, 200.0];
    let normals = vec![Vec3::Y, Vec3::Y, Vec3::Y, Vec3::Y];
    let map = gen.generate_splat_map(&heights, &normals, 2);

    for (i, sw) in map.iter().enumerate() {
        let total = sw.weights_0.x
            + sw.weights_0.y
            + sw.weights_0.z
            + sw.weights_0.w
            + sw.weights_1.x
            + sw.weights_1.y
            + sw.weights_1.z
            + sw.weights_1.w;
        assert!(
            (total - 1.0).abs() < 0.01,
            "Splat map entry {i} weights should sum to ~1.0, got {total}"
        );
    }
}

#[test]
fn splat_generator_missing_normal_uses_y_up() {
    let gen = SplatMapGenerator::with_default_rules(SplatConfig::default(), 42);
    let heights = vec![50.0, 60.0];
    let normals = vec![Vec3::Y]; // Only 1 normal for 2 heights
    let map = gen.generate_splat_map(&heights, &normals, 2);
    assert_eq!(map.len(), 2, "Should generate weights for all heights");
}

// ============================================================================
// TerrainSplatVertex
// ============================================================================

#[test]
fn terrain_splat_vertex_creation() {
    let splat = SplatWeights::from_weights(&[1.0]);
    let vtx = TerrainSplatVertex::new(
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::Y,
        glam::Vec2::new(0.5, 0.5),
        splat,
        4.0,
    );
    assert!((vtx.position.x - 1.0).abs() < 1e-6);
    assert!((vtx.normal.y - 1.0).abs() < 1e-6);
    assert!(vtx.triplanar.y > 0.9, "Y-up normal → high triplanar Y");
}
