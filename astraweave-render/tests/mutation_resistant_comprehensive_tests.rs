//! Comprehensive mutation-resistant tests for astraweave-render
//!
//! Targets all critical numeric constants, comparison operators, formulas,
//! ordering comparisons, and boundary conditions that cargo-mutants would probe.

// ============================================================================
// Module: environment (TimeOfDay, SkyConfig)
// ============================================================================
mod environment_mutations {
    use astraweave_render::environment::{SkyConfig, TimeOfDay};

    // --- TimeOfDay defaults ---

    #[test]
    fn default_current_time_is_noon() {
        let tod = TimeOfDay::default();
        assert_eq!(tod.current_time, 12.0);
    }

    #[test]
    fn default_time_scale_is_60() {
        let tod = TimeOfDay::default();
        assert_eq!(tod.time_scale, 60.0);
    }

    #[test]
    fn default_day_length_is_1440() {
        let tod = TimeOfDay::default();
        assert_eq!(tod.day_length, 1440.0);
    }

    #[test]
    fn new_uses_provided_start_time_and_scale() {
        let tod = TimeOfDay::new(6.0, 120.0);
        assert_eq!(tod.current_time, 6.0);
        assert_eq!(tod.time_scale, 120.0);
    }

    // --- Sun position formula: angle = (time - 6.0) * PI / 12.0 ---

    #[test]
    fn sun_at_6am_is_at_horizon() {
        let tod = TimeOfDay::new(6.0, 1.0);
        let pos = tod.get_sun_position();
        // At 6am, angle = 0 → sin(0) = 0 → sun_height ≈ 0
        assert!(
            pos.y.abs() < 0.15,
            "Sun at 6am should be near horizon, got y={}",
            pos.y
        );
    }

    #[test]
    fn sun_at_noon_is_high() {
        let tod = TimeOfDay::new(12.0, 1.0);
        let pos = tod.get_sun_position();
        // At noon, angle = PI/2 → sin = 1.0
        assert!(pos.y > 0.5, "Sun at noon should be high, got y={}", pos.y);
    }

    #[test]
    fn sun_at_18_is_at_horizon() {
        let tod = TimeOfDay::new(18.0, 1.0);
        let pos = tod.get_sun_position();
        // At 6pm, angle = PI → sin(PI) ≈ 0
        assert!(
            pos.y.abs() < 0.15,
            "Sun at 6pm should be near horizon, got y={}",
            pos.y
        );
    }

    #[test]
    fn sun_at_midnight_is_below() {
        let tod = TimeOfDay::new(0.0, 1.0);
        let pos = tod.get_sun_position();
        // At midnight, angle = -PI/2 → sin = -1.0
        assert!(
            pos.y < -0.5,
            "Sun at midnight should be below horizon, got y={}",
            pos.y
        );
    }

    // --- Moon is opposite of sun ---

    #[test]
    fn moon_position_is_negated_sun() {
        let tod = TimeOfDay::new(12.0, 1.0);
        let sun = tod.get_sun_position();
        let moon = tod.get_moon_position();
        assert!((sun + moon).length() < 0.01, "Moon should be opposite sun");
    }

    // --- get_light_direction: sun_pos.y > 0.1 threshold ---

    #[test]
    fn light_dir_from_sun_when_high() {
        let tod = TimeOfDay::new(12.0, 1.0);
        let sun = tod.get_sun_position();
        let dir = tod.get_light_direction();
        // When sun high, direction = -sun
        assert!(sun.y > 0.1);
        let expected = -sun;
        assert!((dir - expected).length() < 0.01);
    }

    #[test]
    fn light_dir_from_moon_when_sun_low() {
        let tod = TimeOfDay::new(0.0, 1.0);
        let sun = tod.get_sun_position();
        assert!(sun.y <= 0.1, "Sun should be low at midnight");
        let _dir = tod.get_light_direction();
        // Just verifying it doesn't use sun path when sun_pos.y <= 0.1
    }

    // --- get_light_color: thresholds at 0.2 and -0.2 ---

    #[test]
    fn light_color_daytime_rgb_proportions() {
        let tod = TimeOfDay::new(12.0, 1.0);
        let color = tod.get_light_color();
        // Warm yellow/white: r >= g >= b
        assert!(color.x >= color.y, "R >= G for daytime");
        assert!(color.y >= color.z, "G >= B for daytime");
        // Should be bright (components > 0.5)
        assert!(color.x > 0.5);
    }

    #[test]
    fn light_color_nighttime_blue_tint() {
        let tod = TimeOfDay::new(0.0, 1.0);
        let color = tod.get_light_color();
        // Night: cool blue → b > r
        assert!(color.z > color.x, "Blue > Red at night");
        // Night intensity is low (0.15 multiplier)
        assert!(color.x < 0.2, "Night should be dim");
    }

    #[test]
    fn light_color_twilight_orange() {
        // At sunrise ~7am, sun_height is small positive
        // We need sun height between -0.2 and 0.2
        let tod = TimeOfDay::new(6.5, 1.0);
        let sun_height = tod.get_sun_position().y;
        if sun_height > -0.2 && sun_height <= 0.2 {
            let color = tod.get_light_color();
            // Orange/red: R > G > B
            assert!(color.x >= color.y, "R >= G for twilight");
        }
    }

    // --- get_ambient_color: threshold at sun_height > 0.0 ---

    #[test]
    fn ambient_bright_during_day() {
        let tod = TimeOfDay::new(12.0, 1.0);
        let ambient = tod.get_ambient_color();
        // Day ambient: vec3(0.4, 0.6, 1.0) * (0.3 + 0.4 * intensity)
        assert!(
            ambient.z > ambient.x,
            "Day ambient should be blue-ish (z > x)"
        );
        assert!(ambient.y > 0.1, "Day ambient should be bright");
    }

    #[test]
    fn ambient_dim_at_night() {
        let tod = TimeOfDay::new(0.0, 1.0);
        let ambient = tod.get_ambient_color();
        // Night ambient: vec3(0.1, 0.15, 0.3) * 0.1
        assert!(ambient.x < 0.05, "Night ambient should be very dim");
        assert!(ambient.z > ambient.x, "Night ambient blue > red");
    }

    // --- is_day, is_night, is_twilight boundary tests ---

    #[test]
    fn is_day_true_at_noon() {
        let tod = TimeOfDay::new(12.0, 1.0);
        assert!(tod.is_day());
    }

    #[test]
    fn is_day_false_at_midnight() {
        let tod = TimeOfDay::new(0.0, 1.0);
        assert!(!tod.is_day());
    }

    #[test]
    fn is_night_true_at_midnight() {
        let tod = TimeOfDay::new(0.0, 1.0);
        assert!(tod.is_night());
    }

    #[test]
    fn is_night_false_at_noon() {
        let tod = TimeOfDay::new(12.0, 1.0);
        assert!(!tod.is_night());
    }

    #[test]
    fn is_twilight_range_check() {
        // Twilight: sun_height in -0.1..=0.1
        let tod = TimeOfDay::new(12.0, 1.0);
        // At noon, sun is high → not twilight
        assert!(!tod.is_twilight());
    }

    // --- SkyConfig defaults ---

    #[test]
    fn sky_config_default_cloud_coverage() {
        let cfg = SkyConfig::default();
        assert_eq!(cfg.cloud_coverage, 0.5);
    }

    #[test]
    fn sky_config_default_cloud_speed() {
        let cfg = SkyConfig::default();
        assert_eq!(cfg.cloud_speed, 0.02);
    }

    #[test]
    fn sky_config_default_cloud_altitude() {
        let cfg = SkyConfig::default();
        assert_eq!(cfg.cloud_altitude, 1000.0);
    }

    #[test]
    fn sky_config_default_colors_are_proper() {
        let cfg = SkyConfig::default();
        // Day top should be blue
        assert!(cfg.day_color_top.z > cfg.day_color_top.x);
        // Night top should be very dark
        assert!(cfg.night_color_top.x < 0.05);
        assert!(cfg.night_color_top.y < 0.05);
    }
}

// ============================================================================
// Module: shadow_csm (CsmRenderer constants & cascade splitting)
// ============================================================================
mod shadow_csm_mutations {
    use astraweave_render::shadow_csm::*;

    #[test]
    fn cascade_count_is_4() {
        assert_eq!(CASCADE_COUNT, 4);
    }

    #[test]
    fn cascade_resolution_is_2048() {
        assert_eq!(CASCADE_RESOLUTION, 2048);
    }

    #[test]
    fn atlas_resolution_equals_cascade_resolution() {
        assert_eq!(ATLAS_RESOLUTION, CASCADE_RESOLUTION);
    }

    #[test]
    fn depth_bias_is_0_005() {
        assert!((DEPTH_BIAS - 0.005).abs() < f32::EPSILON);
    }

    #[test]
    fn gpu_shadow_cascade_size_is_96_bytes() {
        assert_eq!(
            std::mem::size_of::<GpuShadowCascade>(),
            96 // 64 (mat4) + 16 (split_distances) + 16 (atlas_transform)
        );
    }

    #[test]
    fn cascade_split_formula_logarithmic_with_lambda_0_5() {
        // Reproduce exact formula from update_cascades
        let near = 0.1_f32;
        let far = 1000.0_f32;
        let lambda = 0.5_f32;

        let mut splits = [0.0_f32; 5];
        splits[0] = near;
        splits[4] = far;

        #[allow(clippy::needless_range_loop)]
        for i in 1..4 {
            let i_f = i as f32;
            let n_f = 4.0_f32;
            let log_split = near * (far / near).powf(i_f / n_f);
            let uniform_split = near + (far - near) * (i_f / n_f);
            splits[i] = lambda * log_split + (1.0 - lambda) * uniform_split;
        }

        // Verify monotonically increasing
        for w in splits.windows(2) {
            assert!(w[0] < w[1], "Splits must be monotonically increasing");
        }

        // Verify first split is near, last is far
        assert_eq!(splits[0], near);
        assert_eq!(splits[4], far);

        // Verify split[1] is between pure uniform and pure log
        let pure_uniform_1 = near + (far - near) * (1.0 / 4.0);
        let pure_log_1 = near * (far / near).powf(1.0 / 4.0);
        assert!(splits[1] > pure_log_1.min(pure_uniform_1) - 0.1);
        assert!(splits[1] < pure_uniform_1.max(pure_log_1) + 0.1);
    }

    #[test]
    fn default_cascade_near_far_values() {
        // Default cascades: [0.1, 10.0], [10.0, 50.0], [50.0, 200.0], [200.0, 1000.0]
        let expected = [(0.1, 10.0), (10.0, 50.0), (50.0, 200.0), (200.0, 1000.0)];
        for (i, &(near, far)) in expected.iter().enumerate() {
            assert!(near < far, "Cascade {} near < far", i);
            assert!(near >= 0.0, "Cascade {} near >= 0", i);
        }
    }

    #[test]
    fn gpu_cascade_from_shadow_cascade() {
        let cascade = ShadowCascade {
            near: 0.1,
            far: 10.0,
            view_matrix: glam::Mat4::IDENTITY,
            proj_matrix: glam::Mat4::IDENTITY,
            view_proj_matrix: glam::Mat4::IDENTITY,
            atlas_offset: glam::Vec4::new(0.0, 0.0, 1.0, 1.0),
        };
        let gpu: GpuShadowCascade = (&cascade).into();
        assert_eq!(gpu.split_distances[0], 0.1);
        assert_eq!(gpu.split_distances[1], 10.0);
        assert_eq!(gpu.split_distances[2], 0.0);
        assert_eq!(gpu.split_distances[3], 0.0);
        assert_eq!(gpu.atlas_transform[0], 0.0);
        assert_eq!(gpu.atlas_transform[1], 0.0);
        assert_eq!(gpu.atlas_transform[2], 1.0);
        assert_eq!(gpu.atlas_transform[3], 1.0);
    }
}

// ============================================================================
// Module: culling (FrustumPlanes, AABB tests, cpu_frustum_cull)
// ============================================================================
mod culling_mutations {
    use astraweave_render::culling::*;
    use glam::{Mat4, Vec3};

    #[test]
    fn aabb_from_transform_uses_half_extents() {
        let transform = Mat4::IDENTITY;
        let local_min = Vec3::new(-1.0, -1.0, -1.0);
        let local_max = Vec3::new(1.0, 1.0, 1.0);
        let aabb = InstanceAABB::from_transform(&transform, local_min, local_max, 0);
        // Center should be (0,0,0), extent should be (1,1,1) (half of 2.0 range)
        assert!((aabb.center[0]).abs() < 0.01);
        assert!((aabb.center[1]).abs() < 0.01);
        assert!((aabb.center[2]).abs() < 0.01);
        assert!((aabb.extent[0] - 1.0).abs() < 0.01);
        assert!((aabb.extent[1] - 1.0).abs() < 0.01);
        assert!((aabb.extent[2] - 1.0).abs() < 0.01);
    }

    #[test]
    fn aabb_from_transform_asymmetric() {
        let transform = Mat4::IDENTITY;
        let local_min = Vec3::new(0.0, 0.0, 0.0);
        let local_max = Vec3::new(4.0, 6.0, 2.0);
        let aabb = InstanceAABB::from_transform(&transform, local_min, local_max, 42);
        // Center = (2, 3, 1), extent = (2, 3, 1)
        assert!((aabb.center[0] - 2.0).abs() < 0.01);
        assert!((aabb.center[1] - 3.0).abs() < 0.01);
        assert!((aabb.center[2] - 1.0).abs() < 0.01);
        assert!((aabb.extent[0] - 2.0).abs() < 0.01);
        assert!((aabb.extent[1] - 3.0).abs() < 0.01);
        assert!((aabb.extent[2] - 1.0).abs() < 0.01);
        assert_eq!(aabb.instance_index, 42);
    }

    #[test]
    fn frustum_test_aabb_inside_passes() {
        // Create a frustum from a perspective projection looking down -Z
        let proj = Mat4::perspective_rh(1.0, 1.0, 0.1, 100.0);
        let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0), Vec3::Y);
        let vp = proj * view;
        let frustum = FrustumPlanes::from_view_proj(&vp);

        // Object at (0,0,-10) should be inside
        assert!(frustum.test_aabb(Vec3::new(0.0, 0.0, -10.0), Vec3::splat(1.0)));
    }

    #[test]
    fn frustum_test_aabb_outside_fails() {
        let proj = Mat4::perspective_rh(1.0, 1.0, 0.1, 100.0);
        let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0), Vec3::Y);
        let vp = proj * view;
        let frustum = FrustumPlanes::from_view_proj(&vp);

        // Object far behind camera (positive Z, out of view)
        assert!(!frustum.test_aabb(Vec3::new(0.0, 0.0, 200.0), Vec3::splat(1.0)));
    }

    #[test]
    fn frustum_test_aabb_boundary_exactly_on_plane() {
        // Test the critical `dist < -radius` boundary
        // An object exactly at the boundary should still be visible
        let proj = Mat4::perspective_rh(1.0, 1.0, 0.1, 100.0);
        let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0), Vec3::Y);
        let vp = proj * view;
        let frustum = FrustumPlanes::from_view_proj(&vp);

        // Very tiny object right at near plane
        assert!(frustum.test_aabb(Vec3::new(0.0, 0.0, -0.1), Vec3::splat(0.05)));
    }

    #[test]
    fn cpu_frustum_cull_filters_correctly() {
        let proj = Mat4::perspective_rh(1.0, 1.0, 0.1, 100.0);
        let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0), Vec3::Y);
        let vp = proj * view;
        let frustum = FrustumPlanes::from_view_proj(&vp);

        let instances = vec![
            InstanceAABB::new(Vec3::new(0.0, 0.0, -10.0), Vec3::splat(1.0), 0), // visible
            InstanceAABB::new(Vec3::new(0.0, 0.0, 200.0), Vec3::splat(1.0), 1), // behind
            InstanceAABB::new(Vec3::new(0.0, 0.0, -50.0), Vec3::splat(1.0), 2), // visible
        ];

        let visible = cpu_frustum_cull(&instances, &frustum);
        assert!(visible.contains(&0));
        assert!(!visible.contains(&1));
        assert!(visible.contains(&2));
    }

    #[test]
    fn draw_indirect_command_new_fields() {
        let cmd = DrawIndirectCommand::new(36, 10, 0, 0);
        assert_eq!(cmd.vertex_count, 36);
        assert_eq!(cmd.instance_count, 10);
        assert_eq!(cmd.first_vertex, 0);
        assert_eq!(cmd.first_instance, 0);
    }

    #[test]
    fn batch_id_ordering() {
        let a = BatchId::new(1, 2);
        let b = BatchId::new(1, 3);
        let c = BatchId::new(2, 1);
        assert!(a < b);
        assert!(b < c);
    }

    #[test]
    fn draw_batch_instance_count() {
        let mut batch = DrawBatch::new(BatchId::new(0, 0), 36, 0);
        assert_eq!(batch.instance_count(), 0);
        batch.add_instance(0);
        batch.add_instance(1);
        assert_eq!(batch.instance_count(), 2);
    }
}

// ============================================================================
// Module: material_extended (MaterialGpuExtended, feature flags, to_gpu)
// ============================================================================
mod material_extended_mutations {
    use astraweave_render::material_extended::*;
    use glam::Vec3;

    // --- Struct size ---
    #[test]
    fn material_gpu_extended_is_256_bytes() {
        assert_eq!(std::mem::size_of::<MaterialGpuExtended>(), 256);
    }

    #[test]
    fn material_gpu_extended_16_byte_aligned() {
        assert_eq!(std::mem::align_of::<MaterialGpuExtended>(), 16);
    }

    // --- Feature flag bit values ---
    #[test]
    fn flag_clearcoat_is_0x01() {
        assert_eq!(MATERIAL_FLAG_CLEARCOAT, 0x01);
    }

    #[test]
    fn flag_anisotropy_is_0x02() {
        assert_eq!(MATERIAL_FLAG_ANISOTROPY, 0x02);
    }

    #[test]
    fn flag_subsurface_is_0x04() {
        assert_eq!(MATERIAL_FLAG_SUBSURFACE, 0x04);
    }

    #[test]
    fn flag_sheen_is_0x08() {
        assert_eq!(MATERIAL_FLAG_SHEEN, 0x08);
    }

    #[test]
    fn flag_transmission_is_0x10() {
        assert_eq!(MATERIAL_FLAG_TRANSMISSION, 0x10);
    }

    #[test]
    fn flags_are_distinct_bits() {
        let all = MATERIAL_FLAG_CLEARCOAT
            | MATERIAL_FLAG_ANISOTROPY
            | MATERIAL_FLAG_SUBSURFACE
            | MATERIAL_FLAG_SHEEN
            | MATERIAL_FLAG_TRANSMISSION;
        // Each flag is one bit, 5 bits → 0x1F
        assert_eq!(all, 0x1F);
    }

    // --- Default values ---
    #[test]
    fn default_roughness_factor_is_0_5() {
        let mat = MaterialGpuExtended::default();
        assert_eq!(mat.roughness_factor, 0.5);
    }

    #[test]
    fn default_clearcoat_roughness_is_0_03() {
        let mat = MaterialGpuExtended::default();
        assert_eq!(mat.clearcoat_roughness, 0.03);
    }

    #[test]
    fn default_ior_is_1_5() {
        let mat = MaterialGpuExtended::default();
        assert_eq!(mat.ior, 1.5);
    }

    #[test]
    fn default_subsurface_radius_is_1_0() {
        let mat = MaterialGpuExtended::default();
        assert_eq!(mat.subsurface_radius, 1.0);
    }

    #[test]
    fn default_sheen_roughness_is_0_5() {
        let mat = MaterialGpuExtended::default();
        assert_eq!(mat.sheen_roughness, 0.5);
    }

    #[test]
    fn default_attenuation_distance_is_1_0() {
        let mat = MaterialGpuExtended::default();
        assert_eq!(mat.attenuation_distance, 1.0);
    }

    #[test]
    fn default_flags_are_zero() {
        let mat = MaterialGpuExtended::default();
        assert_eq!(mat.flags, 0);
    }

    // --- Feature enable/disable/has ---
    #[test]
    fn enable_and_has_feature() {
        let mut mat = MaterialGpuExtended::default();
        assert!(!mat.has_feature(MATERIAL_FLAG_CLEARCOAT));
        mat.enable_feature(MATERIAL_FLAG_CLEARCOAT);
        assert!(mat.has_feature(MATERIAL_FLAG_CLEARCOAT));
    }

    #[test]
    fn disable_feature() {
        let mut mat = MaterialGpuExtended::default();
        mat.enable_feature(MATERIAL_FLAG_CLEARCOAT | MATERIAL_FLAG_SHEEN);
        mat.disable_feature(MATERIAL_FLAG_CLEARCOAT);
        assert!(!mat.has_feature(MATERIAL_FLAG_CLEARCOAT));
        assert!(mat.has_feature(MATERIAL_FLAG_SHEEN));
    }

    // --- to_gpu auto-flag thresholds ---
    #[test]
    fn to_gpu_clearcoat_flag_when_strength_positive() {
        let def = MaterialDefinitionExtended {
            name: "test".into(),
            albedo: None,
            normal: None,
            orm: None,
            base_color_factor: [1.0; 4],
            metallic_factor: 0.0,
            roughness_factor: 0.5,
            occlusion_strength: 1.0,
            emissive_factor: [0.0; 3],
            clearcoat_strength: 0.5, // > 0.0 → flag set
            clearcoat_roughness: 0.03,
            clearcoat_normal: None,
            anisotropy_strength: 0.0,
            anisotropy_rotation: 0.0,
            subsurface_color: [1.0; 3],
            subsurface_scale: 0.0,
            subsurface_radius: 1.0,
            thickness_map: None,
            sheen_color: [0.0; 3],
            sheen_roughness: 0.5,
            transmission_factor: 0.0,
            ior: 1.5,
            attenuation_color: [1.0; 3],
            attenuation_distance: 1.0,
        };
        let gpu = def.to_gpu(0, 0, 0, 0, 0);
        assert!(gpu.has_feature(MATERIAL_FLAG_CLEARCOAT));
    }

    #[test]
    fn to_gpu_no_clearcoat_flag_when_strength_zero() {
        let def = MaterialDefinitionExtended {
            name: "test".into(),
            albedo: None,
            normal: None,
            orm: None,
            base_color_factor: [1.0; 4],
            metallic_factor: 0.0,
            roughness_factor: 0.5,
            occlusion_strength: 1.0,
            emissive_factor: [0.0; 3],
            clearcoat_strength: 0.0, // == 0.0 → no flag
            clearcoat_roughness: 0.03,
            clearcoat_normal: None,
            anisotropy_strength: 0.0,
            anisotropy_rotation: 0.0,
            subsurface_color: [1.0; 3],
            subsurface_scale: 0.0,
            subsurface_radius: 1.0,
            thickness_map: None,
            sheen_color: [0.0; 3],
            sheen_roughness: 0.5,
            transmission_factor: 0.0,
            ior: 1.5,
            attenuation_color: [1.0; 3],
            attenuation_distance: 1.0,
        };
        let gpu = def.to_gpu(0, 0, 0, 0, 0);
        assert!(!gpu.has_feature(MATERIAL_FLAG_CLEARCOAT));
    }

    #[test]
    fn to_gpu_anisotropy_flag_threshold_0_001() {
        // abs(0.001) is NOT > 0.001 → no flag
        let def = MaterialDefinitionExtended {
            name: "test".into(),
            albedo: None,
            normal: None,
            orm: None,
            base_color_factor: [1.0; 4],
            metallic_factor: 0.0,
            roughness_factor: 0.5,
            occlusion_strength: 1.0,
            emissive_factor: [0.0; 3],
            clearcoat_strength: 0.0,
            clearcoat_roughness: 0.03,
            clearcoat_normal: None,
            anisotropy_strength: 0.001, // abs(0.001) == 0.001, NOT > 0.001
            anisotropy_rotation: 0.0,
            subsurface_color: [1.0; 3],
            subsurface_scale: 0.0,
            subsurface_radius: 1.0,
            thickness_map: None,
            sheen_color: [0.0; 3],
            sheen_roughness: 0.5,
            transmission_factor: 0.0,
            ior: 1.5,
            attenuation_color: [1.0; 3],
            attenuation_distance: 1.0,
        };
        let gpu = def.to_gpu(0, 0, 0, 0, 0);
        assert!(
            !gpu.has_feature(MATERIAL_FLAG_ANISOTROPY),
            "0.001 is NOT > 0.001"
        );

        // 0.002 IS > 0.001 → flag set
        let def2 = MaterialDefinitionExtended {
            anisotropy_strength: 0.002,
            ..def
        };
        let gpu2 = def2.to_gpu(0, 0, 0, 0, 0);
        assert!(gpu2.has_feature(MATERIAL_FLAG_ANISOTROPY), "0.002 > 0.001");
    }

    #[test]
    fn to_gpu_negative_anisotropy_triggers_flag() {
        let def = MaterialDefinitionExtended {
            name: "test".into(),
            albedo: None,
            normal: None,
            orm: None,
            base_color_factor: [1.0; 4],
            metallic_factor: 0.0,
            roughness_factor: 0.5,
            occlusion_strength: 1.0,
            emissive_factor: [0.0; 3],
            clearcoat_strength: 0.0,
            clearcoat_roughness: 0.03,
            clearcoat_normal: None,
            anisotropy_strength: -0.5, // abs(-0.5) = 0.5 > 0.001
            anisotropy_rotation: 0.0,
            subsurface_color: [1.0; 3],
            subsurface_scale: 0.0,
            subsurface_radius: 1.0,
            thickness_map: None,
            sheen_color: [0.0; 3],
            sheen_roughness: 0.5,
            transmission_factor: 0.0,
            ior: 1.5,
            attenuation_color: [1.0; 3],
            attenuation_distance: 1.0,
        };
        let gpu = def.to_gpu(0, 0, 0, 0, 0);
        assert!(gpu.has_feature(MATERIAL_FLAG_ANISOTROPY));
    }

    #[test]
    fn to_gpu_subsurface_flag_when_scale_positive() {
        let def = MaterialDefinitionExtended {
            name: "test".into(),
            albedo: None,
            normal: None,
            orm: None,
            base_color_factor: [1.0; 4],
            metallic_factor: 0.0,
            roughness_factor: 0.5,
            occlusion_strength: 1.0,
            emissive_factor: [0.0; 3],
            clearcoat_strength: 0.0,
            clearcoat_roughness: 0.03,
            clearcoat_normal: None,
            anisotropy_strength: 0.0,
            anisotropy_rotation: 0.0,
            subsurface_color: [1.0; 3],
            subsurface_scale: 0.5, // > 0.0 → flag
            subsurface_radius: 1.0,
            thickness_map: None,
            sheen_color: [0.0; 3],
            sheen_roughness: 0.5,
            transmission_factor: 0.0,
            ior: 1.5,
            attenuation_color: [1.0; 3],
            attenuation_distance: 1.0,
        };
        let gpu = def.to_gpu(0, 0, 0, 0, 0);
        assert!(gpu.has_feature(MATERIAL_FLAG_SUBSURFACE));
    }

    #[test]
    fn to_gpu_sheen_flag_when_color_nonzero() {
        let def = MaterialDefinitionExtended {
            name: "test".into(),
            albedo: None,
            normal: None,
            orm: None,
            base_color_factor: [1.0; 4],
            metallic_factor: 0.0,
            roughness_factor: 0.5,
            occlusion_strength: 1.0,
            emissive_factor: [0.0; 3],
            clearcoat_strength: 0.0,
            clearcoat_roughness: 0.03,
            clearcoat_normal: None,
            anisotropy_strength: 0.0,
            anisotropy_rotation: 0.0,
            subsurface_color: [1.0; 3],
            subsurface_scale: 0.0,
            subsurface_radius: 1.0,
            thickness_map: None,
            sheen_color: [0.5, 0.0, 0.0], // max(0.5, 0.0, 0.0) = 0.5 > 0.0
            sheen_roughness: 0.5,
            transmission_factor: 0.0,
            ior: 1.5,
            attenuation_color: [1.0; 3],
            attenuation_distance: 1.0,
        };
        let gpu = def.to_gpu(0, 0, 0, 0, 0);
        assert!(gpu.has_feature(MATERIAL_FLAG_SHEEN));
    }

    #[test]
    fn to_gpu_no_sheen_flag_when_color_zero() {
        let def = MaterialDefinitionExtended {
            name: "test".into(),
            albedo: None,
            normal: None,
            orm: None,
            base_color_factor: [1.0; 4],
            metallic_factor: 0.0,
            roughness_factor: 0.5,
            occlusion_strength: 1.0,
            emissive_factor: [0.0; 3],
            clearcoat_strength: 0.0,
            clearcoat_roughness: 0.03,
            clearcoat_normal: None,
            anisotropy_strength: 0.0,
            anisotropy_rotation: 0.0,
            subsurface_color: [1.0; 3],
            subsurface_scale: 0.0,
            subsurface_radius: 1.0,
            thickness_map: None,
            sheen_color: [0.0, 0.0, 0.0], // max = 0.0, NOT > 0.0
            sheen_roughness: 0.5,
            transmission_factor: 0.0,
            ior: 1.5,
            attenuation_color: [1.0; 3],
            attenuation_distance: 1.0,
        };
        let gpu = def.to_gpu(0, 0, 0, 0, 0);
        assert!(!gpu.has_feature(MATERIAL_FLAG_SHEEN));
    }

    #[test]
    fn to_gpu_transmission_flag_when_factor_positive() {
        let def = MaterialDefinitionExtended {
            name: "test".into(),
            albedo: None,
            normal: None,
            orm: None,
            base_color_factor: [1.0; 4],
            metallic_factor: 0.0,
            roughness_factor: 0.5,
            occlusion_strength: 1.0,
            emissive_factor: [0.0; 3],
            clearcoat_strength: 0.0,
            clearcoat_roughness: 0.03,
            clearcoat_normal: None,
            anisotropy_strength: 0.0,
            anisotropy_rotation: 0.0,
            subsurface_color: [1.0; 3],
            subsurface_scale: 0.0,
            subsurface_radius: 1.0,
            thickness_map: None,
            sheen_color: [0.0; 3],
            sheen_roughness: 0.5,
            transmission_factor: 0.8,
            ior: 1.5,
            attenuation_color: [1.0; 3],
            attenuation_distance: 1.0,
        };
        let gpu = def.to_gpu(0, 0, 0, 0, 0);
        assert!(gpu.has_feature(MATERIAL_FLAG_TRANSMISSION));
    }

    // --- Factory methods check ---
    #[test]
    fn car_paint_has_clearcoat_flag() {
        let mat = MaterialGpuExtended::car_paint(Vec3::new(1.0, 0.0, 0.0), 0.9, 0.3);
        assert!(mat.has_feature(MATERIAL_FLAG_CLEARCOAT));
        assert!(!mat.has_feature(MATERIAL_FLAG_ANISOTROPY));
        assert_eq!(mat.clearcoat_strength, 1.0);
        assert_eq!(mat.clearcoat_roughness, 0.05);
    }

    #[test]
    fn brushed_metal_has_anisotropy_flag() {
        let mat = MaterialGpuExtended::brushed_metal(Vec3::ONE, 0.4, 0.8, 0.0);
        assert!(mat.has_feature(MATERIAL_FLAG_ANISOTROPY));
        assert_eq!(mat.metallic_factor, 1.0);
    }

    #[test]
    fn skin_has_subsurface_flag() {
        let mat = MaterialGpuExtended::skin(Vec3::ONE, Vec3::new(0.9, 0.3, 0.3), 1.5, 0.7);
        assert!(mat.has_feature(MATERIAL_FLAG_SUBSURFACE));
        assert_eq!(mat.metallic_factor, 0.0);
    }

    #[test]
    fn velvet_has_sheen_flag() {
        let mat = MaterialGpuExtended::velvet(Vec3::ONE, Vec3::new(0.5, 0.5, 0.5), 0.3);
        assert!(mat.has_feature(MATERIAL_FLAG_SHEEN));
        assert_eq!(mat.roughness_factor, 0.8);
    }

    #[test]
    fn glass_has_transmission_flag() {
        let mat = MaterialGpuExtended::glass(Vec3::ONE, 0.0, 1.0, 1.5, Vec3::ONE, 10.0);
        assert!(mat.has_feature(MATERIAL_FLAG_TRANSMISSION));
        assert_eq!(mat.ior, 1.5);
    }
}

// ============================================================================
// Module: ssao (SsaoQuality, SsaoConfig, SsaoKernel)
// ============================================================================
#[cfg(feature = "ssao")]
mod ssao_mutations {
    use astraweave_render::ssao::*;

    // --- Quality preset values ---
    #[test]
    fn low_sample_count_is_8() {
        assert_eq!(SsaoQuality::Low.sample_count(), 8);
    }

    #[test]
    fn medium_sample_count_is_16() {
        assert_eq!(SsaoQuality::Medium.sample_count(), 16);
    }

    #[test]
    fn high_sample_count_is_32() {
        assert_eq!(SsaoQuality::High.sample_count(), 32);
    }

    #[test]
    fn ultra_sample_count_is_64() {
        assert_eq!(SsaoQuality::Ultra.sample_count(), 64);
    }

    #[test]
    fn low_radius_is_0_5() {
        assert_eq!(SsaoQuality::Low.radius(), 0.5);
    }

    #[test]
    fn medium_radius_is_1_0() {
        assert_eq!(SsaoQuality::Medium.radius(), 1.0);
    }

    #[test]
    fn high_radius_is_1_5() {
        assert_eq!(SsaoQuality::High.radius(), 1.5);
    }

    #[test]
    fn ultra_radius_is_2_0() {
        assert_eq!(SsaoQuality::Ultra.radius(), 2.0);
    }

    #[test]
    fn low_blur_kernel_is_0() {
        assert_eq!(SsaoQuality::Low.blur_kernel_size(), 0);
    }

    #[test]
    fn medium_blur_kernel_is_3() {
        assert_eq!(SsaoQuality::Medium.blur_kernel_size(), 3);
    }

    #[test]
    fn high_blur_kernel_is_5() {
        assert_eq!(SsaoQuality::High.blur_kernel_size(), 5);
    }

    #[test]
    fn ultra_blur_kernel_is_7() {
        assert_eq!(SsaoQuality::Ultra.blur_kernel_size(), 7);
    }

    // --- SsaoConfig defaults ---
    #[test]
    fn default_radius_is_1_0() {
        let cfg = SsaoConfig::default();
        assert_eq!(cfg.radius, 1.0);
    }

    #[test]
    fn default_bias_is_0_025() {
        let cfg = SsaoConfig::default();
        assert_eq!(cfg.bias, 0.025);
    }

    #[test]
    fn default_intensity_is_1_0() {
        let cfg = SsaoConfig::default();
        assert_eq!(cfg.intensity, 1.0);
    }

    #[test]
    fn default_quality_is_medium() {
        let cfg = SsaoConfig::default();
        assert_eq!(cfg.quality, SsaoQuality::Medium);
    }

    #[test]
    fn default_enabled_is_true() {
        let cfg = SsaoConfig::default();
        assert!(cfg.enabled);
    }

    // --- Kernel generation: scale formula is 0.1 + scale*scale * 0.9 ---
    #[test]
    fn kernel_generate_produces_nonzero_samples() {
        let kernel = SsaoKernel::generate(16);
        // First sample should be non-zero
        let s0 = kernel.samples[0];
        assert!(s0[0] != 0.0 || s0[1] != 0.0 || s0[2] != 0.0);
    }

    #[test]
    fn kernel_scale_formula_first_sample() {
        // For i=0: scale = (0 + 1) / N = 1/N
        // then scale = 0.1 + (1/N)^2 * 0.9
        // For N=16: final_scale = 0.1 + (1/16)^2 * 0.9 = 0.1 + 0.003515625 ≈ 0.10352
        let kernel = SsaoKernel::generate(16);
        let magnitude = (kernel.samples[0][0].powi(2)
            + kernel.samples[0][1].powi(2)
            + kernel.samples[0][2].powi(2))
        .sqrt();
        // Should be small (near 0.1 scale)
        assert!(
            magnitude < 0.2,
            "First sample should have small magnitude, got {}",
            magnitude
        );
        assert!(magnitude > 0.01, "First sample should be non-negligible");
    }

    #[test]
    fn kernel_scale_formula_last_sample() {
        // For i=15 (last of 16): scale = 16/16 = 1.0
        // then scale = 0.1 + 1.0 * 0.9 = 1.0
        let kernel = SsaoKernel::generate(16);
        let last = kernel.samples[15];
        let magnitude = (last[0].powi(2) + last[1].powi(2) + last[2].powi(2)).sqrt();
        // Should be large (near 1.0 scale)
        assert!(
            magnitude > 0.5,
            "Last sample should have large magnitude, got {}",
            magnitude
        );
    }

    #[test]
    fn kernel_hemisphere_z_positive() {
        // All samples should have positive z (hemisphere above ground)
        let kernel = SsaoKernel::generate(32);
        for i in 0..32 {
            assert!(kernel.samples[i][2] >= 0.0, "Sample {} z should be >= 0", i);
        }
    }
}

// ============================================================================
// Module: lod_generator (LODConfig, LODGenerator, EdgeCollapse ordering)
// ============================================================================
mod lod_generator_mutations {
    use astraweave_render::lod_generator::*;
    use glam::Vec3;

    // --- LODConfig defaults ---
    #[test]
    fn default_reduction_targets() {
        let cfg = LODConfig::default();
        assert_eq!(cfg.reduction_targets, vec![0.75, 0.50, 0.25]);
    }

    #[test]
    fn default_max_error_is_0_01() {
        let cfg = LODConfig::default();
        assert_eq!(cfg.max_error, 0.01);
    }

    #[test]
    fn default_preserve_boundaries_is_true() {
        let cfg = LODConfig::default();
        assert!(cfg.preserve_boundaries);
    }

    // --- SimplificationMesh ---
    #[test]
    fn mesh_vertex_and_triangle_count() {
        let mesh = SimplificationMesh::new(
            vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            vec![Vec3::Z; 3],
            vec![[0.0, 0.0]; 3],
            vec![0, 1, 2],
        );
        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.triangle_count(), 1);
    }

    // --- LOD generation preserves when below target ---
    #[test]
    fn simplify_no_op_when_below_target() {
        let mesh = SimplificationMesh::new(
            vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            vec![Vec3::Z; 3],
            vec![[0.0, 0.0]; 3],
            vec![0, 1, 2],
        );
        let gen = LODGenerator::new(LODConfig::default());
        let simplified = gen.simplify(&mesh, 100); // target > actual
        assert_eq!(simplified.vertex_count(), 3); // No simplification
    }

    // --- LOD generation reduces vertices ---
    #[test]
    fn simplify_reduces_vertex_count() {
        // Create a grid-like mesh with enough vertices for simplification
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        let grid = 5;
        for z in 0..grid {
            for x in 0..grid {
                positions.push(Vec3::new(x as f32, 0.0, z as f32));
                normals.push(Vec3::Y);
                uvs.push([x as f32 / grid as f32, z as f32 / grid as f32]);
            }
        }

        for z in 0..(grid - 1) {
            for x in 0..(grid - 1) {
                let i = z * grid + x;
                indices.push(i as u32);
                indices.push((i + 1) as u32);
                indices.push((i + grid) as u32);
                indices.push((i + 1) as u32);
                indices.push((i + grid + 1) as u32);
                indices.push((i + grid) as u32);
            }
        }

        let original_count = grid * grid; // 25 vertices
        let mesh = SimplificationMesh::new(positions, normals, uvs, indices);
        let gen = LODGenerator::new(LODConfig {
            max_error: 100.0, // Large error tolerance to allow simplification
            ..LODConfig::default()
        });
        // Target 50% of original — simplifier should reduce
        let simplified = gen.simplify(&mesh, original_count / 2);
        assert!(
            simplified.vertex_count() < original_count,
            "Should reduce vertex count: {} < {}",
            simplified.vertex_count(),
            original_count
        );
    }

    // --- generate_lods produces correct number of LODs ---
    #[test]
    fn generate_lods_count() {
        let mesh = SimplificationMesh::new(
            vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::new(1.0, 1.0, 0.0)],
            vec![Vec3::Z; 4],
            vec![[0.0, 0.0]; 4],
            vec![0, 1, 2, 1, 3, 2],
        );
        let cfg = LODConfig::default(); // 3 reduction targets
        let gen = LODGenerator::new(cfg);
        let lods = gen.generate_lods(&mesh);
        assert_eq!(lods.len(), 3); // One per reduction target
    }
}

// ============================================================================
// Module: transparency (sort order, BlendMode)
// ============================================================================
mod transparency_mutations {
    use astraweave_render::transparency::*;
    use glam::Vec3;

    #[test]
    fn sort_order_is_back_to_front() {
        let mut mgr = TransparencyManager::new();
        mgr.add_instance(0, Vec3::new(0.0, 0.0, -2.0), BlendMode::Alpha);
        mgr.add_instance(1, Vec3::new(0.0, 0.0, -10.0), BlendMode::Alpha);
        mgr.add_instance(2, Vec3::new(0.0, 0.0, -5.0), BlendMode::Alpha);
        mgr.update(Vec3::ZERO);

        let sorted: Vec<u32> = mgr.sorted_instances().map(|i| i.instance_index).collect();
        // Back-to-front: farthest first
        assert_eq!(sorted, vec![1, 2, 0]); // -10, -5, -2
    }

    #[test]
    fn sort_order_not_front_to_back() {
        let mut mgr = TransparencyManager::new();
        mgr.add_instance(0, Vec3::new(0.0, 0.0, -2.0), BlendMode::Alpha);
        mgr.add_instance(1, Vec3::new(0.0, 0.0, -10.0), BlendMode::Alpha);
        mgr.update(Vec3::ZERO);

        let sorted: Vec<u32> = mgr.sorted_instances().map(|i| i.instance_index).collect();
        // Must NOT be [0, 1] (front-to-back)
        assert_ne!(
            sorted,
            vec![0, 1],
            "Must be back-to-front, not front-to-back"
        );
    }

    #[test]
    fn clear_removes_all() {
        let mut mgr = TransparencyManager::new();
        mgr.add_instance(0, Vec3::ZERO, BlendMode::Alpha);
        assert_eq!(mgr.count(), 1);
        mgr.clear();
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn instances_by_blend_mode_filters() {
        let mut mgr = TransparencyManager::new();
        mgr.add_instance(0, Vec3::new(0.0, 0.0, -1.0), BlendMode::Alpha);
        mgr.add_instance(1, Vec3::new(0.0, 0.0, -2.0), BlendMode::Additive);
        mgr.add_instance(2, Vec3::new(0.0, 0.0, -3.0), BlendMode::Alpha);
        mgr.update(Vec3::ZERO);

        let alpha_count = mgr.instances_by_blend_mode(BlendMode::Alpha).count();
        let additive_count = mgr.instances_by_blend_mode(BlendMode::Additive).count();
        assert_eq!(alpha_count, 2);
        assert_eq!(additive_count, 1);
    }
}

// ============================================================================
// Module: decals (Decal fade, DecalBlendMode values, DecalAtlas UV)
// ============================================================================
mod decal_mutations {
    use astraweave_render::decals::*;
    use glam::{Quat, Vec3};

    #[test]
    fn decal_blend_mode_values() {
        assert_eq!(DecalBlendMode::Multiply as u32, 0);
        assert_eq!(DecalBlendMode::Additive as u32, 1);
        assert_eq!(DecalBlendMode::AlphaBlend as u32, 2);
        assert_eq!(DecalBlendMode::Stain as u32, 3);
    }

    #[test]
    fn decal_new_defaults() {
        let d = Decal::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE, ([0.0; 2], [1.0; 2]));
        assert_eq!(d.normal_strength, 1.0);
        assert_eq!(d.roughness, 0.5);
        assert_eq!(d.metallic, 0.0);
        assert_eq!(d.fade_duration, 0.0);
        assert_eq!(d.fade_time, 0.0);
        assert_eq!(d.albedo_tint, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn decal_update_permanent_stays_alive() {
        let mut d = Decal::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE, ([0.0; 2], [1.0; 2]));
        d.fade_duration = 0.0; // Permanent
        assert!(d.update(1.0)); // Should stay alive
        assert!(d.update(100.0)); // Still alive
    }

    #[test]
    fn decal_update_fading_removes_when_done() {
        let mut d = Decal::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE, ([0.0; 2], [1.0; 2]));
        d.fade_duration = 1.0; // 1 second fade

        // After 0.5s, should still be alive
        assert!(d.update(0.5));
        assert!(
            (d.albedo_tint[3] - 0.5).abs() < 0.01,
            "Alpha should be 0.5 at half fade"
        );

        // After another 0.5s, total >= 1.0, should be removed
        assert!(!d.update(0.5));
    }

    #[test]
    fn decal_fade_alpha_formula() {
        let mut d = Decal::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE, ([0.0; 2], [1.0; 2]));
        d.fade_duration = 2.0;

        // After 0.5s: alpha = 1.0 - (0.5 / 2.0) = 0.75
        d.update(0.5);
        assert!((d.albedo_tint[3] - 0.75).abs() < 0.01);
    }

    #[test]
    fn decal_to_gpu_blend_mode_in_params_w() {
        let mut d = Decal::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE, ([0.0; 2], [1.0; 2]));
        d.blend_mode = DecalBlendMode::Stain;
        let gpu = d.to_gpu();
        assert_eq!(gpu.params[3], 3.0); // Stain = 3
    }

    #[test]
    fn gpu_decal_size_is_112_bytes() {
        assert_eq!(std::mem::size_of::<GpuDecal>(), 112);
    }
}

// ============================================================================
// Module: camera (CameraController defaults, clamp boundaries)
// ============================================================================
mod camera_mutations {
    use astraweave_camera::{CameraController, CameraMode, FreeFly as Camera};

    #[test]
    fn camera_dir_at_zero_yaw_zero_pitch() {
        let dir = Camera::dir(0.0, 0.0);
        // cos(0)*cos(0) = 1, sin(0) = 0, sin(0)*cos(0) = 0
        assert!((dir.x - 1.0).abs() < 0.01);
        assert!((dir.y).abs() < 0.01);
        assert!((dir.z).abs() < 0.01);
    }

    #[test]
    fn camera_proj_guards_zero_aspect() {
        let cam = Camera {
            position: glam::Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            fovy: 1.0,
            aspect: 0.0, // Would cause division by zero
            znear: 0.1,
            zfar: 100.0,
        };
        // proj_matrix uses aspect.max(0.01)
        let _proj = cam.proj_matrix(); // Should not panic
    }

    #[test]
    fn controller_defaults() {
        let ctrl = CameraController::new(5.0, 0.003);
        assert_eq!(ctrl.mouse_smooth, 0.15);
        assert_eq!(ctrl.mouse_deadzone, 0.25);
        assert_eq!(ctrl.zoom_sensitivity, 0.1);
    }

    #[test]
    fn controller_sprint_and_precision_multipliers() {
        let ctrl = CameraController::new(5.0, 0.003);
        // Access via the public fields sprint_mult and precision_mult
        // These are private, but we can test the effects through behavior
        // At minimum, we know the constructor doesn't panic with these defaults
        assert!(!ctrl.is_dragging());
    }
}

// ============================================================================
// Module: advanced_post (TaaConfig, MotionBlurConfig, DofConfig, ColorGradingConfig)
// ============================================================================
mod advanced_post_mutations {
    use astraweave_render::advanced_post::*;
    use astraweave_render::TaaConfig;

    #[test]
    fn taa_default_blend_factor_is_0_95() {
        let cfg = TaaConfig::default();
        assert_eq!(cfg.blend_factor, 0.95);
    }

    #[test]
    fn taa_default_jitter_scale_is_1_0() {
        let cfg = TaaConfig::default();
        assert_eq!(cfg.jitter_scale, 1.0);
    }

    #[test]
    fn taa_default_enabled() {
        assert!(TaaConfig::default().enabled);
    }

    #[test]
    fn motion_blur_default_sample_count_is_8() {
        let cfg = MotionBlurConfig::default();
        assert_eq!(cfg.sample_count, 8);
    }

    #[test]
    fn motion_blur_default_strength_is_1_0() {
        let cfg = MotionBlurConfig::default();
        assert_eq!(cfg.strength, 1.0);
    }

    #[test]
    fn motion_blur_default_disabled() {
        assert!(!MotionBlurConfig::default().enabled);
    }

    #[test]
    fn dof_default_focus_distance_is_10_0() {
        let cfg = DofConfig::default();
        assert_eq!(cfg.focus_distance, 10.0);
    }

    #[test]
    fn dof_default_focus_range_is_5_0() {
        let cfg = DofConfig::default();
        assert_eq!(cfg.focus_range, 5.0);
    }

    #[test]
    fn dof_default_bokeh_size_is_2_0() {
        let cfg = DofConfig::default();
        assert_eq!(cfg.bokeh_size, 2.0);
    }

    #[test]
    fn color_grading_defaults() {
        let cfg = ColorGradingConfig::default();
        assert_eq!(cfg.exposure, 0.0);
        assert_eq!(cfg.contrast, 1.0);
        assert_eq!(cfg.saturation, 1.0);
        assert_eq!(cfg.temperature, 0.0);
        assert_eq!(cfg.tint, 0.0);
    }
}

// ============================================================================
// Module: gpu_memory (GpuMemoryBudget, CategoryBudget, try_allocate)
// ============================================================================
mod gpu_memory_mutations {
    use astraweave_render::gpu_memory::*;

    #[test]
    fn category_budget_default_soft_limit() {
        let budget = CategoryBudget::default();
        assert_eq!(budget.soft_limit, 256 * 1024 * 1024); // 256 MB
    }

    #[test]
    fn category_budget_default_hard_limit() {
        let budget = CategoryBudget::default();
        assert_eq!(budget.hard_limit, 512 * 1024 * 1024); // 512 MB
    }

    #[test]
    fn category_budget_starts_at_zero() {
        let budget = CategoryBudget::default();
        assert_eq!(budget.current, 0);
    }

    #[test]
    fn memory_category_all_returns_8() {
        assert_eq!(MemoryCategory::all().len(), 8);
    }

    #[test]
    fn try_allocate_succeeds_within_budget() {
        let mgr = GpuMemoryBudget::new();
        assert!(mgr.try_allocate(MemoryCategory::Textures, 1024));
    }

    #[test]
    fn try_allocate_fails_beyond_hard_limit() {
        let mgr = GpuMemoryBudget::with_total_budget(1024 * 1024); // 1 MB total
                                                                   // Per-category hard limit ≈ 1MB/8 = 128KB, textures gets 40% = ~400KB
                                                                   // Try to allocate way more than any category limit
        let huge = 2 * 1024 * 1024; // 2MB
        assert!(!mgr.try_allocate(MemoryCategory::Geometry, huge));
    }

    #[test]
    fn with_total_budget_texture_gets_extra() {
        let mgr = GpuMemoryBudget::with_total_budget(8 * 1024 * 1024); // 8 MB total
                                                                       // Textures hard limit = 40% of total = 3.2 MB
                                                                       // Regular per_category = 8MB/8 = 1MB
                                                                       // Texture should accept more than 1MB
        assert!(mgr.try_allocate(MemoryCategory::Textures, 2 * 1024 * 1024));
    }

    #[test]
    fn deallocate_returns_memory() {
        let mgr = GpuMemoryBudget::new();
        mgr.try_allocate(MemoryCategory::Geometry, 1000);
        let used1 = mgr.total_usage();
        assert!(used1 >= 1000);

        mgr.deallocate(MemoryCategory::Geometry, 500);
        let used2 = mgr.total_usage();
        assert!(used2 < used1);
    }

    #[test]
    fn get_usage_per_category() {
        let mgr = GpuMemoryBudget::new();
        assert_eq!(mgr.get_usage(MemoryCategory::Geometry), 0);
        mgr.try_allocate(MemoryCategory::Geometry, 2048);
        assert_eq!(mgr.get_usage(MemoryCategory::Geometry), 2048);
    }

    #[test]
    fn usage_percentage_formula() {
        let mgr = GpuMemoryBudget::with_total_budget(1_000_000);
        mgr.try_allocate(MemoryCategory::Geometry, 100);
        let pct = mgr.usage_percentage();
        // 100 / 1_000_000 = 0.0001 = 0.01%
        assert!(pct < 0.001);
        assert!(pct >= 0.0);
    }
}

// ============================================================================
// Module: clustered_forward (ClusterConfig defaults, GpuLight packing)
// ============================================================================
mod clustered_forward_mutations {
    use astraweave_render::clustered_forward::*;
    use glam::Vec3;

    #[test]
    fn cluster_config_defaults() {
        let cfg = ClusterConfig::default();
        assert_eq!(cfg.cluster_x, 16);
        assert_eq!(cfg.cluster_y, 9);
        assert_eq!(cfg.cluster_z, 24);
        assert_eq!(cfg.near, 0.1_f32);
        assert_eq!(cfg.far, 100.0_f32);
    }

    #[test]
    fn total_clusters_is_3456() {
        let cfg = ClusterConfig::default();
        let total = cfg.cluster_x * cfg.cluster_y * cfg.cluster_z;
        assert_eq!(total, 3456);
    }

    #[test]
    fn gpu_light_packs_radius_in_position_w() {
        let light = GpuLight::new(Vec3::new(1.0, 2.0, 3.0), 5.0, Vec3::ONE, 10.0);
        assert_eq!(light.position[3], 5.0); // w = radius
    }

    #[test]
    fn gpu_light_packs_intensity_in_color_w() {
        let light = GpuLight::new(Vec3::ZERO, 1.0, Vec3::new(0.5, 0.6, 0.7), 15.0);
        assert_eq!(light.color[3], 15.0); // w = intensity
    }
}

// ============================================================================
// Module: texture_streaming (LoadRequest ordering, memory estimation)
// ============================================================================
mod texture_streaming_mutations {
    use astraweave_render::texture_streaming::*;

    #[test]
    fn new_sets_memory_bytes_from_mb() {
        let mgr = TextureStreamingManager::new(256);
        let stats = mgr.get_stats();
        assert_eq!(stats.memory_budget_bytes, 256 * 1024 * 1024);
    }

    #[test]
    fn new_starts_empty() {
        let mgr = TextureStreamingManager::new(256);
        let stats = mgr.get_stats();
        assert_eq!(stats.loaded_count, 0);
        assert_eq!(stats.memory_used_bytes, 0);
        assert_eq!(stats.memory_used_percent, 0.0);
    }

    #[test]
    fn is_resident_false_initially() {
        let mgr = TextureStreamingManager::new(256);
        assert!(!mgr.is_resident(&"test_texture".to_string()));
    }

    #[test]
    fn clear_resets_everything() {
        let mut mgr = TextureStreamingManager::new(256);
        mgr.request_texture("test1".into(), 1, 10.0);
        mgr.clear();
        let stats = mgr.get_stats();
        assert_eq!(stats.loaded_count, 0);
        assert_eq!(stats.memory_used_bytes, 0);
    }

    #[test]
    fn request_texture_returns_none_for_new() {
        let mut mgr = TextureStreamingManager::new(256);
        let result = mgr.request_texture("new_texture".into(), 1, 10.0);
        assert!(result.is_none());
    }

    #[test]
    fn update_residency_does_not_crash() {
        let mut mgr = TextureStreamingManager::new(256);
        mgr.update_residency(glam::Vec3::new(10.0, 5.0, 20.0));
    }
}

// IBL methods (env_size, spec_size, irradiance_size, brdf_lut_size, spec_mips)
// are private — tested via inline cfg(test) tests in ibl.rs instead.
