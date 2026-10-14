//! Wave 2 – Golden-value tests for material_extended.rs (86 mutants)
//!
//! Targets: MaterialGpuExtended factory methods (car_paint, brushed_metal,
//!          skin, velvet, glass), MaterialDefinitionExtended::to_gpu flag
//!          computation, feature flag bit operations, TOML serde defaults.
//!
//! Strategy: Pin EXACT field values and flags for every factory method.
//! Pin to_gpu's flag computation boundaries (>0.0, abs()>0.001, fold/max).

use astraweave_render::material_extended::{
    MaterialDefinitionExtended, MaterialGpuExtended, MATERIAL_FLAG_ANISOTROPY,
    MATERIAL_FLAG_CLEARCOAT, MATERIAL_FLAG_SHEEN, MATERIAL_FLAG_SUBSURFACE,
    MATERIAL_FLAG_TRANSMISSION,
};
use glam::Vec3;

// ============================================================================
// Factory methods — pin every non-default field
// ============================================================================

#[test]
fn car_paint_field_values_golden() {
    let m = MaterialGpuExtended::car_paint(Vec3::new(0.8, 0.1, 0.2), 0.9, 0.3);
    // base_color_factor channels in order, alpha = 1.0
    assert_eq!(m.base_color_factor, [0.8, 0.1, 0.2, 1.0]);
    assert_eq!(m.metallic_factor, 0.9);
    assert_eq!(m.roughness_factor, 0.3);
    assert_eq!(m.clearcoat_strength, 1.0);
    assert_eq!(m.clearcoat_roughness, 0.05);
    assert_eq!(m.flags, MATERIAL_FLAG_CLEARCOAT);
    // Other features should be off
    assert!(!m.has_feature(MATERIAL_FLAG_ANISOTROPY));
    assert!(!m.has_feature(MATERIAL_FLAG_SUBSURFACE));
    assert!(!m.has_feature(MATERIAL_FLAG_SHEEN));
    assert!(!m.has_feature(MATERIAL_FLAG_TRANSMISSION));
}

#[test]
fn car_paint_base_color_xyz_order() {
    // Ensure x→[0], y→[1], z→[2] — catches channel swaps
    let m = MaterialGpuExtended::car_paint(Vec3::new(0.1, 0.2, 0.3), 0.5, 0.5);
    assert_eq!(m.base_color_factor[0], 0.1, "x → [0]");
    assert_eq!(m.base_color_factor[1], 0.2, "y → [1]");
    assert_eq!(m.base_color_factor[2], 0.3, "z → [2]");
    assert_eq!(m.base_color_factor[3], 1.0, "alpha → 1.0");
}

#[test]
fn brushed_metal_field_values_golden() {
    let m = MaterialGpuExtended::brushed_metal(Vec3::new(0.9, 0.9, 0.9), 0.4, 0.8, 1.57);
    assert_eq!(m.base_color_factor, [0.9, 0.9, 0.9, 1.0]);
    assert_eq!(m.metallic_factor, 1.0); // Always 1.0 for metal
    assert_eq!(m.roughness_factor, 0.4);
    assert_eq!(m.anisotropy_strength, 0.8);
    assert_eq!(m.anisotropy_rotation, 1.57);
    assert_eq!(m.flags, MATERIAL_FLAG_ANISOTROPY);
}

#[test]
fn brushed_metal_metallic_always_one() {
    // Even if caller passes different params, metallic should be 1.0
    let m = MaterialGpuExtended::brushed_metal(Vec3::ZERO, 0.2, 0.5, 0.0);
    assert_eq!(m.metallic_factor, 1.0);
}

#[test]
fn skin_field_values_golden() {
    let m = MaterialGpuExtended::skin(
        Vec3::new(0.95, 0.80, 0.70),
        Vec3::new(0.9, 0.3, 0.3),
        1.5,
        0.7,
    );
    assert_eq!(m.base_color_factor, [0.95, 0.80, 0.70, 1.0]);
    assert_eq!(m.metallic_factor, 0.0); // Skin is non-metallic
    assert_eq!(m.roughness_factor, 0.5);
    assert_eq!(m.subsurface_color, [0.9, 0.3, 0.3]);
    assert_eq!(m.subsurface_radius, 1.5);
    assert_eq!(m.subsurface_scale, 0.7);
    assert_eq!(m.flags, MATERIAL_FLAG_SUBSURFACE);
}

#[test]
fn skin_subsurface_color_xyz_order() {
    let m = MaterialGpuExtended::skin(Vec3::ONE, Vec3::new(0.1, 0.2, 0.3), 1.0, 1.0);
    assert_eq!(m.subsurface_color[0], 0.1, "x → [0]");
    assert_eq!(m.subsurface_color[1], 0.2, "y → [1]");
    assert_eq!(m.subsurface_color[2], 0.3, "z → [2]");
}

#[test]
fn skin_radius_vs_scale_not_swapped() {
    let m = MaterialGpuExtended::skin(Vec3::ONE, Vec3::ONE, 2.0, 0.5);
    assert_eq!(m.subsurface_radius, 2.0, "3rd param is radius");
    assert_eq!(m.subsurface_scale, 0.5, "4th param is scale");
}

#[test]
fn velvet_field_values_golden() {
    let m = MaterialGpuExtended::velvet(Vec3::new(0.5, 0.0, 0.1), Vec3::new(1.0, 0.8, 0.9), 0.3);
    assert_eq!(m.base_color_factor, [0.5, 0.0, 0.1, 1.0]);
    assert_eq!(m.metallic_factor, 0.0);
    assert_eq!(m.roughness_factor, 0.8); // Velvet is rough
    assert_eq!(m.sheen_color, [1.0, 0.8, 0.9]);
    assert_eq!(m.sheen_roughness, 0.3);
    assert_eq!(m.flags, MATERIAL_FLAG_SHEEN);
}

#[test]
fn velvet_sheen_color_xyz_order() {
    let m = MaterialGpuExtended::velvet(Vec3::ONE, Vec3::new(0.1, 0.2, 0.3), 0.5);
    assert_eq!(m.sheen_color[0], 0.1);
    assert_eq!(m.sheen_color[1], 0.2);
    assert_eq!(m.sheen_color[2], 0.3);
}

#[test]
fn glass_field_values_golden() {
    let m = MaterialGpuExtended::glass(
        Vec3::new(0.9, 0.95, 1.0), // tint
        0.05,                      // roughness
        0.95,                      // transmission
        1.5,                       // ior
        Vec3::new(0.9, 1.0, 0.9),  // attenuation_color
        10.0,                      // attenuation_distance
    );
    assert_eq!(m.base_color_factor, [0.9, 0.95, 1.0, 1.0]);
    assert_eq!(m.metallic_factor, 0.0);
    assert_eq!(m.roughness_factor, 0.05);
    assert_eq!(m.transmission_factor, 0.95);
    assert_eq!(m.ior, 1.5);
    assert_eq!(m.attenuation_color, [0.9, 1.0, 0.9]);
    assert_eq!(m.attenuation_distance, 10.0);
    assert_eq!(m.flags, MATERIAL_FLAG_TRANSMISSION);
}

#[test]
fn glass_param_order_not_swapped() {
    // Ensure roughness, transmission, ior, attenuation_dist are in correct slots
    let m = MaterialGpuExtended::glass(Vec3::ONE, 0.1, 0.2, 0.3, Vec3::ONE, 0.4);
    assert_eq!(m.roughness_factor, 0.1, "2nd param = roughness");
    assert_eq!(m.transmission_factor, 0.2, "3rd param = transmission");
    assert_eq!(m.ior, 0.3, "4th param = ior");
    assert_eq!(
        m.attenuation_distance, 0.4,
        "6th param = attenuation_distance"
    );
}

// ============================================================================
// Feature flag bitfield operations
// ============================================================================

#[test]
fn flag_constants_are_distinct_powers_of_two() {
    let flags = [
        MATERIAL_FLAG_CLEARCOAT,
        MATERIAL_FLAG_ANISOTROPY,
        MATERIAL_FLAG_SUBSURFACE,
        MATERIAL_FLAG_SHEEN,
        MATERIAL_FLAG_TRANSMISSION,
    ];
    // Each should be a distinct power of 2
    for &f in &flags {
        assert!(f.is_power_of_two(), "Flag 0x{:02X} must be power of 2", f);
    }
    // All should be distinct
    for i in 0..flags.len() {
        for j in (i + 1)..flags.len() {
            assert_ne!(flags[i], flags[j], "Flags must be distinct");
        }
    }
}

#[test]
fn flag_constants_golden_values() {
    assert_eq!(MATERIAL_FLAG_CLEARCOAT, 0x01);
    assert_eq!(MATERIAL_FLAG_ANISOTROPY, 0x02);
    assert_eq!(MATERIAL_FLAG_SUBSURFACE, 0x04);
    assert_eq!(MATERIAL_FLAG_SHEEN, 0x08);
    assert_eq!(MATERIAL_FLAG_TRANSMISSION, 0x10);
}

#[test]
fn has_feature_checks_exact_bit() {
    let m = MaterialGpuExtended {
        flags: MATERIAL_FLAG_CLEARCOAT | MATERIAL_FLAG_SHEEN, // 0x01 | 0x08 = 0x09
        ..Default::default()
    };
    assert!(m.has_feature(MATERIAL_FLAG_CLEARCOAT));
    assert!(m.has_feature(MATERIAL_FLAG_SHEEN));
    assert!(!m.has_feature(MATERIAL_FLAG_ANISOTROPY));
    assert!(!m.has_feature(MATERIAL_FLAG_SUBSURFACE));
    assert!(!m.has_feature(MATERIAL_FLAG_TRANSMISSION));
}

#[test]
fn enable_feature_sets_bit_without_clearing_others() {
    let mut m = MaterialGpuExtended::default();
    m.enable_feature(MATERIAL_FLAG_CLEARCOAT);
    m.enable_feature(MATERIAL_FLAG_SHEEN);
    assert_eq!(m.flags, 0x09); // 0x01 | 0x08
    assert!(m.has_feature(MATERIAL_FLAG_CLEARCOAT));
    assert!(m.has_feature(MATERIAL_FLAG_SHEEN));
}

#[test]
fn disable_feature_clears_only_target_bit() {
    let mut m = MaterialGpuExtended {
        flags: 0x1F, // All 5 flags set
        ..Default::default()
    };
    m.disable_feature(MATERIAL_FLAG_ANISOTROPY);
    assert_eq!(m.flags, 0x1D); // 0x1F & !0x02
    assert!(m.has_feature(MATERIAL_FLAG_CLEARCOAT)); // still set
    assert!(!m.has_feature(MATERIAL_FLAG_ANISOTROPY)); // cleared
    assert!(m.has_feature(MATERIAL_FLAG_SUBSURFACE)); // still set
}

#[test]
fn enable_already_enabled_is_idempotent() {
    let mut m = MaterialGpuExtended::default();
    m.enable_feature(MATERIAL_FLAG_SHEEN);
    m.enable_feature(MATERIAL_FLAG_SHEEN);
    assert_eq!(m.flags, MATERIAL_FLAG_SHEEN);
}

#[test]
fn disable_already_disabled_is_idempotent() {
    let mut m = MaterialGpuExtended::default();
    m.disable_feature(MATERIAL_FLAG_SHEEN); // Wasn't set
    assert_eq!(m.flags, 0);
}

// ============================================================================
// MaterialDefinitionExtended::to_gpu — flag computation boundaries
// ============================================================================

fn make_def(overrides: impl FnOnce(&mut MaterialDefinitionExtended)) -> MaterialDefinitionExtended {
    let mut d = MaterialDefinitionExtended {
        name: "test".into(),
        albedo: None,
        normal: None,
        orm: None,
        base_color_factor: [1.0, 1.0, 1.0, 1.0],
        metallic_factor: 0.0,
        roughness_factor: 0.5,
        occlusion_strength: 1.0,
        emissive_factor: [0.0, 0.0, 0.0],
        clearcoat_strength: 0.0,
        clearcoat_roughness: 0.03,
        clearcoat_normal: None,
        anisotropy_strength: 0.0,
        anisotropy_rotation: 0.0,
        subsurface_color: [1.0, 1.0, 1.0],
        subsurface_scale: 0.0,
        subsurface_radius: 1.0,
        thickness_map: None,
        sheen_color: [0.0, 0.0, 0.0],
        sheen_roughness: 0.5,
        transmission_factor: 0.0,
        ior: 1.5,
        attenuation_color: [1.0, 1.0, 1.0],
        attenuation_distance: 1.0,
    };
    overrides(&mut d);
    d
}

#[test]
fn to_gpu_no_features_flags_zero() {
    let d = make_def(|_| {});
    let g = d.to_gpu(0, 0, 0, 0, 0);
    assert_eq!(g.flags, 0, "All defaults → no flags");
}

#[test]
fn to_gpu_clearcoat_flag_when_strength_positive() {
    let d = make_def(|d| d.clearcoat_strength = 0.5);
    let g = d.to_gpu(0, 0, 0, 0, 0);
    assert!(g.has_feature(MATERIAL_FLAG_CLEARCOAT));
}

#[test]
fn to_gpu_clearcoat_flag_off_when_strength_zero() {
    let d = make_def(|d| d.clearcoat_strength = 0.0);
    let g = d.to_gpu(0, 0, 0, 0, 0);
    assert!(!g.has_feature(MATERIAL_FLAG_CLEARCOAT));
}

#[test]
fn to_gpu_anisotropy_flag_when_abs_above_threshold() {
    // Threshold is 0.001
    let d = make_def(|d| d.anisotropy_strength = 0.01);
    let g = d.to_gpu(0, 0, 0, 0, 0);
    assert!(g.has_feature(MATERIAL_FLAG_ANISOTROPY));
}

#[test]
fn to_gpu_anisotropy_flag_also_for_negative() {
    // abs() allows negative anisotropy to set the flag
    let d = make_def(|d| d.anisotropy_strength = -0.5);
    let g = d.to_gpu(0, 0, 0, 0, 0);
    assert!(
        g.has_feature(MATERIAL_FLAG_ANISOTROPY),
        "Negative anisotropy should set flag via abs()"
    );
}

#[test]
fn to_gpu_anisotropy_flag_off_when_near_zero() {
    let d = make_def(|d| d.anisotropy_strength = 0.0005); // Below 0.001 threshold
    let g = d.to_gpu(0, 0, 0, 0, 0);
    assert!(
        !g.has_feature(MATERIAL_FLAG_ANISOTROPY),
        "Below threshold → no flag"
    );
}

#[test]
fn to_gpu_subsurface_flag_when_scale_positive() {
    let d = make_def(|d| d.subsurface_scale = 0.1);
    let g = d.to_gpu(0, 0, 0, 0, 0);
    assert!(g.has_feature(MATERIAL_FLAG_SUBSURFACE));
}

#[test]
fn to_gpu_subsurface_flag_off_when_scale_zero() {
    let d = make_def(|d| d.subsurface_scale = 0.0);
    let g = d.to_gpu(0, 0, 0, 0, 0);
    assert!(!g.has_feature(MATERIAL_FLAG_SUBSURFACE));
}

#[test]
fn to_gpu_sheen_flag_when_any_channel_positive() {
    // sheen_max uses fold(max) over all 3 channels
    let d = make_def(|d| d.sheen_color = [0.0, 0.0, 0.1]);
    let g = d.to_gpu(0, 0, 0, 0, 0);
    assert!(g.has_feature(MATERIAL_FLAG_SHEEN));
}

#[test]
fn to_gpu_sheen_flag_off_when_all_channels_zero() {
    let d = make_def(|d| d.sheen_color = [0.0, 0.0, 0.0]);
    let g = d.to_gpu(0, 0, 0, 0, 0);
    assert!(!g.has_feature(MATERIAL_FLAG_SHEEN));
}

#[test]
fn to_gpu_sheen_flag_first_channel_only() {
    let d = make_def(|d| d.sheen_color = [0.5, 0.0, 0.0]);
    let g = d.to_gpu(0, 0, 0, 0, 0);
    assert!(
        g.has_feature(MATERIAL_FLAG_SHEEN),
        "First channel alone should set flag"
    );
}

#[test]
fn to_gpu_sheen_flag_second_channel_only() {
    let d = make_def(|d| d.sheen_color = [0.0, 0.5, 0.0]);
    let g = d.to_gpu(0, 0, 0, 0, 0);
    assert!(
        g.has_feature(MATERIAL_FLAG_SHEEN),
        "Second channel alone should set flag"
    );
}

#[test]
fn to_gpu_transmission_flag_when_factor_positive() {
    let d = make_def(|d| d.transmission_factor = 0.5);
    let g = d.to_gpu(0, 0, 0, 0, 0);
    assert!(g.has_feature(MATERIAL_FLAG_TRANSMISSION));
}

#[test]
fn to_gpu_transmission_flag_off_when_factor_zero() {
    let d = make_def(|d| d.transmission_factor = 0.0);
    let g = d.to_gpu(0, 0, 0, 0, 0);
    assert!(!g.has_feature(MATERIAL_FLAG_TRANSMISSION));
}

#[test]
fn to_gpu_all_features_combined() {
    let d = make_def(|d| {
        d.clearcoat_strength = 1.0;
        d.anisotropy_strength = 0.5;
        d.subsurface_scale = 0.3;
        d.sheen_color = [1.0, 0.0, 0.0];
        d.transmission_factor = 0.8;
    });
    let g = d.to_gpu(0, 0, 0, 0, 0);
    let all_flags = MATERIAL_FLAG_CLEARCOAT
        | MATERIAL_FLAG_ANISOTROPY
        | MATERIAL_FLAG_SUBSURFACE
        | MATERIAL_FLAG_SHEEN
        | MATERIAL_FLAG_TRANSMISSION;
    assert_eq!(
        g.flags, all_flags,
        "All features enabled → 0x{:02X}",
        all_flags
    );
}

// ============================================================================
// to_gpu — field passthrough verification
// ============================================================================

#[test]
fn to_gpu_passes_texture_indices() {
    let d = make_def(|_| {});
    let g = d.to_gpu(10, 20, 30, 40, 50);
    assert_eq!(g.albedo_index, 10);
    assert_eq!(g.normal_index, 20);
    assert_eq!(g.orm_index, 30);
    assert_eq!(g.clearcoat_normal_index, 40);
    assert_eq!(g.thickness_index, 50);
}

#[test]
fn to_gpu_passes_all_scalar_fields() {
    let d = make_def(|d| {
        d.metallic_factor = 0.7;
        d.roughness_factor = 0.3;
        d.occlusion_strength = 0.8;
        d.clearcoat_strength = 0.5;
        d.clearcoat_roughness = 0.1;
        d.anisotropy_strength = 0.6;
        d.anisotropy_rotation = 1.0;
        d.subsurface_scale = 0.4;
        d.subsurface_radius = 2.0;
        d.sheen_roughness = 0.2;
        d.transmission_factor = 0.9;
        d.ior = 1.33;
        d.attenuation_distance = 5.0;
    });
    let g = d.to_gpu(0, 0, 0, 0, 0);

    assert_eq!(g.metallic_factor, 0.7);
    assert_eq!(g.roughness_factor, 0.3);
    assert_eq!(g.occlusion_strength, 0.8);
    assert_eq!(g.clearcoat_strength, 0.5);
    assert_eq!(g.clearcoat_roughness, 0.1);
    assert_eq!(g.anisotropy_strength, 0.6);
    assert_eq!(g.anisotropy_rotation, 1.0);
    assert_eq!(g.subsurface_scale, 0.4);
    assert_eq!(g.subsurface_radius, 2.0);
    assert_eq!(g.sheen_roughness, 0.2);
    assert_eq!(g.transmission_factor, 0.9);
    assert_eq!(g.ior, 1.33);
    assert_eq!(g.attenuation_distance, 5.0);
}

#[test]
fn to_gpu_passes_array_fields() {
    let d = make_def(|d| {
        d.base_color_factor = [0.1, 0.2, 0.3, 0.4];
        d.emissive_factor = [10.0, 20.0, 30.0];
        d.subsurface_color = [0.5, 0.6, 0.7];
        d.sheen_color = [0.8, 0.9, 1.0];
        d.attenuation_color = [0.11, 0.22, 0.33];
    });
    let g = d.to_gpu(0, 0, 0, 0, 0);

    assert_eq!(g.base_color_factor, [0.1, 0.2, 0.3, 0.4]);
    assert_eq!(g.emissive_factor, [10.0, 20.0, 30.0]);
    assert_eq!(g.subsurface_color, [0.5, 0.6, 0.7]);
    assert_eq!(g.sheen_color, [0.8, 0.9, 1.0]);
    assert_eq!(g.attenuation_color, [0.11, 0.22, 0.33]);
}

// ============================================================================
// TOML serde round-trip with defaults
// ============================================================================

#[test]
fn toml_minimal_definition_uses_defaults() {
    let toml_str = r#"name = "minimal""#;
    let d: MaterialDefinitionExtended = toml::from_str(toml_str).unwrap();
    assert_eq!(d.name, "minimal");
    // Check serde defaults
    assert_eq!(d.base_color_factor, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(d.metallic_factor, 0.0);
    assert_eq!(d.roughness_factor, 0.5);
    assert_eq!(d.occlusion_strength, 1.0);
    assert_eq!(d.emissive_factor, [0.0, 0.0, 0.0]);
    assert_eq!(d.clearcoat_strength, 0.0);
    assert_eq!(d.clearcoat_roughness, 0.03);
    assert_eq!(d.anisotropy_strength, 0.0);
    assert_eq!(d.subsurface_color, [1.0, 1.0, 1.0]);
    assert_eq!(d.subsurface_scale, 0.0);
    assert_eq!(d.subsurface_radius, 1.0);
    assert_eq!(d.sheen_color, [0.0, 0.0, 0.0]);
    assert_eq!(d.sheen_roughness, 0.5);
    assert_eq!(d.transmission_factor, 0.0);
    assert_eq!(d.ior, 1.5);
    assert_eq!(d.attenuation_color, [1.0, 1.0, 1.0]);
    assert_eq!(d.attenuation_distance, 1.0);
}

#[test]
fn toml_roundtrip_preserves_all_fields() {
    let original = make_def(|d| {
        d.name = "roundtrip_test".into();
        d.metallic_factor = 0.7;
        d.clearcoat_strength = 0.5;
        d.anisotropy_strength = 0.3;
        d.subsurface_scale = 0.2;
        d.sheen_color = [0.1, 0.2, 0.3];
        d.transmission_factor = 0.8;
    });
    let toml_str = toml::to_string(&original).unwrap();
    let restored: MaterialDefinitionExtended = toml::from_str(&toml_str).unwrap();

    assert_eq!(restored.name, "roundtrip_test");
    assert_eq!(restored.metallic_factor, 0.7);
    assert_eq!(restored.clearcoat_strength, 0.5);
    assert_eq!(restored.anisotropy_strength, 0.3);
    assert_eq!(restored.subsurface_scale, 0.2);
    assert_eq!(restored.sheen_color, [0.1, 0.2, 0.3]);
    assert_eq!(restored.transmission_factor, 0.8);
}

// ============================================================================
// Default values — golden
// ============================================================================

#[test]
fn default_material_gpu_extended_golden() {
    let m = MaterialGpuExtended::default();
    assert_eq!(m.flags, 0);
    assert_eq!(m.base_color_factor, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(m.metallic_factor, 0.0);
    assert_eq!(m.roughness_factor, 0.5);
    assert_eq!(m.occlusion_strength, 1.0);
    assert_eq!(m.emissive_factor, [0.0, 0.0, 0.0]);
    assert_eq!(m.clearcoat_strength, 0.0);
    assert_eq!(m.clearcoat_roughness, 0.03);
    assert_eq!(m.anisotropy_strength, 0.0);
    assert_eq!(m.subsurface_color, [1.0, 1.0, 1.0]);
    assert_eq!(m.subsurface_scale, 0.0);
    assert_eq!(m.subsurface_radius, 1.0);
    assert_eq!(m.sheen_color, [0.0, 0.0, 0.0]);
    assert_eq!(m.sheen_roughness, 0.5);
    assert_eq!(m.transmission_factor, 0.0);
    assert_eq!(m.ior, 1.5);
    assert_eq!(m.attenuation_color, [1.0, 1.0, 1.0]);
    assert_eq!(m.attenuation_distance, 1.0);
}

#[test]
fn material_struct_size_256_bytes() {
    assert_eq!(std::mem::size_of::<MaterialGpuExtended>(), 256);
}

#[test]
fn material_struct_align_16_bytes() {
    assert_eq!(std::mem::align_of::<MaterialGpuExtended>(), 16);
}
