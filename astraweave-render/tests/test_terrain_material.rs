// Phase PBR-F: Comprehensive Unit Tests for Terrain Layering System
// Tests cover struct alignment, weight normalization, blending correctness, and factory methods

#[cfg(test)]
mod terrain_material_tests {
    use astraweave_render::{
        TerrainLayerDesc, TerrainLayerGpu, TerrainMaterialDesc, TerrainMaterialGpu,
    };
    use std::path::PathBuf;

    // ========================================================================
    // MEMORY LAYOUT AND ALIGNMENT TESTS
    // ========================================================================

    #[test]
    fn test_terrain_layer_size_and_alignment() {
        // Verify TerrainLayerGpu is exactly 64 bytes (16-byte aligned)
        assert_eq!(
            std::mem::size_of::<TerrainLayerGpu>(),
            64,
            "TerrainLayerGpu must be 64 bytes for GPU buffer compatibility"
        );
        assert_eq!(
            std::mem::align_of::<TerrainLayerGpu>(),
            16,
            "TerrainLayerGpu must be 16-byte aligned for UBO/SSBO"
        );
    }

    #[test]
    fn test_terrain_material_size_and_alignment() {
        // Verify TerrainMaterialGpu is exactly 576 bytes (8*64 + 64)
        assert_eq!(
            std::mem::size_of::<TerrainMaterialGpu>(),
            576,
            "TerrainMaterialGpu must be 576 bytes (8 layers + common params)"
        );
        assert_eq!(
            std::mem::align_of::<TerrainMaterialGpu>(),
            16,
            "TerrainMaterialGpu must be 16-byte aligned"
        );
    }

    #[test]
    fn test_pod_zeroable_terrain_layer() {
        // Verify TerrainLayerGpu implements Pod and Zeroable (can be created from bytes)
        let bytes = [0u8; 64];
        let layer: TerrainLayerGpu = bytemuck::cast(bytes);

        assert_eq!(layer.uv_scale, [0.0, 0.0]);
        assert_eq!(layer.blend_sharpness, 0.0);
        assert_eq!(layer.triplanar_power, 0.0);
    }

    #[test]
    fn test_pod_zeroable_terrain_material() {
        // Verify TerrainMaterialGpu implements Pod and Zeroable
        let bytes = [0u8; 576];
        let material: TerrainMaterialGpu = bytemuck::cast(bytes);

        assert_eq!(material.splat_uv_scale, 0.0);
        assert_eq!(material.triplanar_enabled, 0);
        assert_eq!(material.normal_blend_method, 0);
    }

    // ========================================================================
    // DEFAULT VALUES TESTS
    // ========================================================================

    #[test]
    fn test_default_terrain_layer() {
        let layer = TerrainLayerGpu::default();

        assert_eq!(layer.uv_scale, [1.0, 1.0]);
        assert_eq!(layer.height_range, [0.0, 100.0]);
        assert_eq!(layer.blend_sharpness, 0.5);
        assert_eq!(layer.triplanar_power, 4.0);
        assert_eq!(layer.material_factors, [0.0, 0.5]); // metallic=0, roughness=0.5
        assert_eq!(layer.texture_indices, [0, 0, 0, 0]);
    }

    #[test]
    fn test_default_terrain_material() {
        let material = TerrainMaterialGpu::default();

        assert_eq!(material.splat_map_index_0, 0);
        assert_eq!(material.splat_uv_scale, 1.0);
        assert_eq!(material.triplanar_enabled, 1); // Enabled by default
        assert_eq!(material.normal_blend_method, 1); // RNM by default
        assert_eq!(material.triplanar_slope_threshold, 45.0);
        assert_eq!(material.height_blend_enabled, 1);

        // Check all 8 layers initialized
        for i in 0..8 {
            assert_eq!(material.layers[i].uv_scale, [1.0, 1.0]);
        }
    }

    // ========================================================================
    // FACTORY METHODS TESTS
    // ========================================================================

    #[test]
    fn test_grassland_factory() {
        let grassland = TerrainMaterialDesc::grassland();

        assert_eq!(grassland.name, "grassland_terrain");
        assert_eq!(grassland.biome, "grassland");
        assert_eq!(grassland.layers.len(), 4);

        // Check layer names
        assert_eq!(grassland.layers[0].name, "grass");
        assert_eq!(grassland.layers[1].name, "dirt");
        assert_eq!(grassland.layers[2].name, "rock");
        assert_eq!(grassland.layers[3].name, "sparse_grass");

        // Check UV scales are reasonable
        assert_eq!(grassland.layers[0].uv_scale, [8.0, 8.0]);
        assert_eq!(grassland.layers[1].uv_scale, [6.0, 6.0]);

        // Check height ranges
        assert_eq!(grassland.layers[0].height_range, Some([0.0, 50.0]));
        assert_eq!(grassland.layers[2].height_range, Some([40.0, 100.0]));

        // Check triplanar settings
        assert!(grassland.triplanar_enabled);
        assert_eq!(grassland.triplanar_slope_threshold, 35.0);
    }

    #[test]
    fn test_desert_factory() {
        let desert = TerrainMaterialDesc::desert();

        assert_eq!(desert.name, "desert_terrain");
        assert_eq!(desert.biome, "desert");
        assert_eq!(desert.layers.len(), 4);

        assert_eq!(desert.layers[0].name, "sand");
        assert_eq!(desert.layers[1].name, "red_sand");
        assert_eq!(desert.layers[2].name, "desert_rock");
        assert_eq!(desert.layers[3].name, "cracked_ground");

        // Desert should have higher UV scales for sand detail
        assert_eq!(desert.layers[0].uv_scale, [12.0, 12.0]);

        // Desert rock should be rougher
        assert_eq!(desert.layers[2].roughness, 0.6);
    }

    #[test]
    fn test_forest_factory() {
        let forest = TerrainMaterialDesc::forest();

        assert_eq!(forest.name, "forest_terrain");
        assert_eq!(forest.biome, "forest");
        assert_eq!(forest.layers.len(), 4);

        assert_eq!(forest.layers[0].name, "moss");
        assert_eq!(forest.layers[1].name, "forest_dirt");
        assert_eq!(forest.layers[2].name, "forest_rock");
        assert_eq!(forest.layers[3].name, "leaf_litter");

        // Forest should have gentler triplanar threshold (less steep terrain)
        assert_eq!(forest.triplanar_slope_threshold, 30.0);
    }

    // ========================================================================
    // NORMAL BLEND METHOD TESTS
    // ========================================================================

    #[test]
    fn test_normal_blend_parsing() {
        let desc = TerrainMaterialDesc {
            normal_blend_method: "linear".to_string(),
            ..Default::default()
        };
        assert_eq!(desc.normal_blend_to_gpu(), 0);

        let desc = TerrainMaterialDesc {
            normal_blend_method: "rnm".to_string(),
            ..Default::default()
        };
        assert_eq!(desc.normal_blend_to_gpu(), 1);

        let desc = TerrainMaterialDesc {
            normal_blend_method: "udn".to_string(),
            ..Default::default()
        };
        assert_eq!(desc.normal_blend_to_gpu(), 2);

        let desc = TerrainMaterialDesc {
            normal_blend_method: "RNM".to_string(), // Case insensitive
            ..Default::default()
        };
        assert_eq!(desc.normal_blend_to_gpu(), 1);

        let desc = TerrainMaterialDesc {
            normal_blend_method: "invalid".to_string(),
            ..Default::default()
        };
        assert_eq!(desc.normal_blend_to_gpu(), 1); // Falls back to RNM
    }

    // ========================================================================
    // TO_GPU CONVERSION TESTS
    // ========================================================================

    #[test]
    fn test_to_gpu_conversion_basic() {
        let desc = TerrainMaterialDesc::default();

        // Mock texture resolver (returns sequential indices)
        use std::cell::Cell;
        let counter = Cell::new(0u32);
        let resolver = |_: &PathBuf| -> u32 {
            let val = counter.get();
            counter.set(val + 1);
            val
        };

        let gpu = desc.to_gpu(&resolver);

        // Check default values transferred
        assert_eq!(gpu.splat_uv_scale, 1.0);
        assert_eq!(gpu.triplanar_enabled, 1);
        assert_eq!(gpu.normal_blend_method, 1); // RNM
        assert_eq!(gpu.triplanar_slope_threshold, 45.0);
        assert_eq!(gpu.height_blend_enabled, 1);
    }

    #[test]
    fn test_to_gpu_conversion_grassland() {
        let desc = TerrainMaterialDesc::grassland();

        // Mock texture resolver that returns predictable indices
        use std::cell::Cell;
        let counter = Cell::new(0u32);
        let resolver = |_: &PathBuf| -> u32 {
            let idx = counter.get();
            counter.set(idx + 1);
            idx
        };

        let gpu = desc.to_gpu(&resolver);

        // Check grassland-specific settings
        assert_eq!(gpu.splat_uv_scale, 0.5);
        assert_eq!(gpu.triplanar_enabled, 1);
        assert_eq!(gpu.triplanar_slope_threshold, 35.0);

        // Check first layer (grass) properties
        let grass_layer = &gpu.layers[0];
        assert_eq!(grass_layer.uv_scale, [8.0, 8.0]);
        assert_eq!(grass_layer.blend_sharpness, 0.6);
        assert_eq!(grass_layer.triplanar_power, 3.0);
        assert_eq!(grass_layer.material_factors, [0.0, 0.9]); // metallic=0, roughness=0.9

        // Texture indices should be assigned sequentially
        assert!(grass_layer.texture_indices[0] < 100); // albedo index
        assert!(grass_layer.texture_indices[1] < 100); // normal index
        assert!(grass_layer.texture_indices[2] < 100); // orm index
        assert!(grass_layer.texture_indices[3] < 100); // height index
    }

    #[test]
    fn test_to_gpu_handles_missing_textures() {
        let mut desc = TerrainMaterialDesc::default();

        // Add single layer with only albedo
        desc.layers.push(TerrainLayerDesc {
            name: "test".to_string(),
            albedo: Some(PathBuf::from("test.png")),
            normal: None,
            orm: None,
            height: None,
            ..Default::default()
        });

        let resolver = |path: &PathBuf| -> u32 {
            if path.to_str().unwrap() == "test.png" {
                42
            } else {
                0
            }
        };

        let gpu = desc.to_gpu(&resolver);

        // Albedo should be set, others default to 0
        assert_eq!(gpu.layers[0].texture_indices[0], 42);
        assert_eq!(gpu.layers[0].texture_indices[1], 0);
        assert_eq!(gpu.layers[0].texture_indices[2], 0);
        assert_eq!(gpu.layers[0].texture_indices[3], 0);
    }

    // ========================================================================
    // LAYER WEIGHT NORMALIZATION TESTS
    // ========================================================================

    #[test]
    fn test_splat_weight_normalization_concept() {
        // This tests the mathematical concept that will be used in WGSL
        let weights = [0.5f32, 0.3, 0.2, 0.1];
        let sum = weights[0] + weights[1] + weights[2] + weights[3];
        assert!((sum - 1.1).abs() < 0.001); // Original sum is 1.1

        let normalized = weights.map(|w| w / sum);
        let new_sum = normalized[0] + normalized[1] + normalized[2] + normalized[3];
        assert!((new_sum - 1.0).abs() < 0.001); // Normalized sum is 1.0
    }

    #[test]
    fn test_height_based_weight_adjustment() {
        // Simulate height-based weight adjustment
        let base_weights = [0.4f32, 0.3, 0.2, 0.1];
        let heights = [0.5f32, 0.8, 0.3, 0.2]; // Layer 1 is highest
        let blend_sharpness = 0.5;

        // Height-adjusted weights (higher areas blend more)
        let adjusted = [
            base_weights[0] * (1.0 + heights[0] * blend_sharpness),
            base_weights[1] * (1.0 + heights[1] * blend_sharpness),
            base_weights[2] * (1.0 + heights[2] * blend_sharpness),
            base_weights[3] * (1.0 + heights[3] * blend_sharpness),
        ];

        // Normalize
        let sum = adjusted[0] + adjusted[1] + adjusted[2] + adjusted[3];
        let final_weights = adjusted.map(|w| w / sum);

        // Layer 1 (highest) should have gained weight
        assert!(final_weights[1] > base_weights[1]);

        // Sum should still be 1.0
        let total = final_weights[0] + final_weights[1] + final_weights[2] + final_weights[3];
        assert!((total - 1.0).abs() < 0.001);
    }

    // ========================================================================
    // TRIPLANAR BLEND FACTOR TESTS
    // ========================================================================

    #[test]
    fn test_triplanar_blend_factor_flat_surface() {
        // Flat surface (normal pointing up) should have low triplanar blend
        let flat_normal = [0.0f32, 1.0, 0.0];
        let threshold = 45.0f32.to_radians();

        // Dot product with up vector [0, 1, 0]
        let up_dot = (flat_normal[0] * 0.0 + flat_normal[1] * 1.0 + flat_normal[2] * 0.0).abs();
        assert!((up_dot - 1.0).abs() < 0.001); // Should be exactly 1.0

        // At 45° threshold, flat surface should use standard UV (not triplanar)
        let should_triplanar = up_dot < threshold.cos();
        assert!(!should_triplanar);
    }

    #[test]
    fn test_triplanar_blend_factor_steep_slope() {
        // Steep slope (45° or more) should have high triplanar blend
        let steep_normal = [0.707f32, 0.707, 0.0]; // Normalized 45° slope
        let threshold = 45.0f32.to_radians();

        let up_dot = (steep_normal[0] * 0.0 + steep_normal[1] * 1.0 + steep_normal[2] * 0.0).abs();
        assert!((up_dot - 0.707).abs() < 0.01);

        let should_triplanar = up_dot < threshold.cos();
        assert!(should_triplanar);
    }

    #[test]
    fn test_triplanar_blend_factor_vertical_cliff() {
        // Vertical cliff (normal horizontal) should use maximum triplanar
        let vertical_normal = [1.0f32, 0.0, 0.0];
        let threshold = 45.0f32.to_radians();

        let up_dot =
            (vertical_normal[0] * 0.0 + vertical_normal[1] * 1.0 + vertical_normal[2] * 0.0).abs();
        assert!(up_dot < 0.001); // Nearly 0

        let should_triplanar = up_dot < threshold.cos();
        assert!(should_triplanar);
    }

    // ========================================================================
    // UV SCALING TESTS
    // ========================================================================

    #[test]
    fn test_per_layer_uv_scaling() {
        let grassland = TerrainMaterialDesc::grassland();

        // Different layers should have different UV scales
        let scales: Vec<[f32; 2]> = grassland.layers.iter().map(|l| l.uv_scale).collect();

        assert_eq!(scales[0], [8.0, 8.0]); // grass
        assert_eq!(scales[1], [6.0, 6.0]); // dirt
        assert_eq!(scales[2], [4.0, 4.0]); // rock
        assert_eq!(scales[3], [10.0, 10.0]); // sparse grass

        // Verify they're all different (allows detail variation)
        assert_ne!(scales[0], scales[1]);
        assert_ne!(scales[1], scales[2]);
    }

    // ========================================================================
    // EDGE CASE TESTS
    // ========================================================================

    #[test]
    fn test_single_layer_fallback() {
        let mut desc = TerrainMaterialDesc::default();
        desc.layers.push(TerrainLayerDesc {
            name: "single".to_string(),
            ..Default::default()
        });

        let resolver = |_: &PathBuf| -> u32 { 0 };
        let gpu = desc.to_gpu(&resolver);

        // First layer should be initialized, others default
        assert_eq!(gpu.layers[1].uv_scale, [1.0, 1.0]);
        assert_eq!(gpu.layers[2].uv_scale, [1.0, 1.0]);
        assert_eq!(gpu.layers[3].uv_scale, [1.0, 1.0]);
    }

    #[test]
    fn test_zero_splat_uv_scale() {
        let desc = TerrainMaterialDesc {
            splat_uv_scale: 0.0,
            ..Default::default()
        };

        let resolver = |_: &PathBuf| -> u32 { 0 };
        let gpu = desc.to_gpu(&resolver);

        // Should preserve zero (edge case, not recommended but valid)
        assert_eq!(gpu.splat_uv_scale, 0.0);
    }

    #[test]
    fn test_extreme_triplanar_power() {
        let layer = TerrainLayerGpu {
            triplanar_power: 20.0, // Very high (sharp transitions)
            ..Default::default()
        };

        // Should be valid (pow will just make sharp falloff)
        assert!(layer.triplanar_power > 0.0);

        let layer = TerrainLayerGpu {
            triplanar_power: 0.5, // Very low (soft transitions)
            ..Default::default()
        };
        assert!(layer.triplanar_power > 0.0);
    }

    #[test]
    fn test_height_range_validation() {
        let grassland = TerrainMaterialDesc::grassland();

        for layer in &grassland.layers {
            if let Some(range) = layer.height_range {
                // Min should be less than max
                assert!(
                    range[0] < range[1],
                    "Layer {} has invalid height range",
                    layer.name
                );
            }
        }
    }

    // ========================================================================
    // SERIALIZATION TESTS (TOML)
    // ========================================================================

    #[test]
    fn test_terrain_material_toml_roundtrip() {
        let original = TerrainMaterialDesc::grassland();

        // Serialize to TOML
        let toml_string = toml::to_string(&original).expect("Failed to serialize to TOML");

        // Deserialize back
        let deserialized: TerrainMaterialDesc =
            toml::from_str(&toml_string).expect("Failed to deserialize from TOML");

        // Check key properties preserved
        assert_eq!(deserialized.name, original.name);
        assert_eq!(deserialized.biome, original.biome);
        assert_eq!(deserialized.layers.len(), original.layers.len());
        assert_eq!(
            deserialized.triplanar_slope_threshold,
            original.triplanar_slope_threshold
        );
    }

    #[test]
    fn test_terrain_layer_toml_defaults() {
        let toml_minimal = r#"
            name = "test_layer"
        "#;

        let layer: TerrainLayerDesc =
            toml::from_str(toml_minimal).expect("Failed to parse minimal TOML");

        // Check defaults applied
        assert_eq!(layer.uv_scale, [1.0, 1.0]);
        assert_eq!(layer.blend_sharpness, 0.5);
        assert_eq!(layer.triplanar_power, 4.0);
        assert_eq!(layer.roughness, 0.5);
        assert_eq!(layer.metallic, 0.0);
    }
}
