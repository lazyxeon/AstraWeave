//! Batch 9: Extended PBR materials — MaterialGpuExtended constructors, defaults, feature flags
//! Mutation-resistant integration tests targeting:
//!   - MaterialGpuExtended: Default values, size (256B), Pod/Zeroable
//!   - car_paint(): clearcoat flag, metallic, roughness
//!   - brushed_metal(): anisotropy flag, metallic=1.0
//!   - skin(): subsurface flag, scale, radius
//!   - velvet(): sheen flag, sheen_color, roughness
//!   - glass(): transmission flag, ior, attenuation
//!   - has_feature/enable_feature/disable_feature: bitfield ops
//!   - Feature flag constants: bitmask values

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use glam::Vec3;

use astraweave_render::material_extended::{
    MaterialGpuExtended, MATERIAL_FLAG_ANISOTROPY, MATERIAL_FLAG_CLEARCOAT, MATERIAL_FLAG_SHEEN,
    MATERIAL_FLAG_SUBSURFACE, MATERIAL_FLAG_TRANSMISSION,
};

// ═══════════════════════════════════════════════════════════════════════════════
//  Flag constants
// ═══════════════════════════════════════════════════════════════════════════════

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
fn flags_are_disjoint_powers_of_two() {
    let all = [
        MATERIAL_FLAG_CLEARCOAT,
        MATERIAL_FLAG_ANISOTROPY,
        MATERIAL_FLAG_SUBSURFACE,
        MATERIAL_FLAG_SHEEN,
        MATERIAL_FLAG_TRANSMISSION,
    ];
    // Each flag should have exactly one bit set
    for &f in &all {
        assert_eq!(f.count_ones(), 1, "flag {f:#x} not a power of 2");
    }
    // OR of all should have 5 bits set (all disjoint)
    let combined = all.iter().fold(0u32, |a, &b| a | b);
    assert_eq!(combined.count_ones(), 5);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  MaterialGpuExtended default
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn default_size_256_bytes() {
    assert_eq!(std::mem::size_of::<MaterialGpuExtended>(), 256);
}

#[test]
fn default_flags_zero() {
    let m = MaterialGpuExtended::default();
    assert_eq!(m.flags, 0);
}

#[test]
fn default_base_color_white() {
    let m = MaterialGpuExtended::default();
    assert_eq!(m.base_color_factor, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn default_metallic_zero() {
    let m = MaterialGpuExtended::default();
    assert!((m.metallic_factor - 0.0).abs() < 1e-6);
}

#[test]
fn default_roughness_0_5() {
    let m = MaterialGpuExtended::default();
    assert!((m.roughness_factor - 0.5).abs() < 1e-6);
}

#[test]
fn default_occlusion_strength_1() {
    let m = MaterialGpuExtended::default();
    assert!((m.occlusion_strength - 1.0).abs() < 1e-6);
}

#[test]
fn default_emissive_zero() {
    let m = MaterialGpuExtended::default();
    assert_eq!(m.emissive_factor, [0.0, 0.0, 0.0]);
}

#[test]
fn default_clearcoat_strength_zero() {
    let m = MaterialGpuExtended::default();
    assert!((m.clearcoat_strength - 0.0).abs() < 1e-6);
}

#[test]
fn default_clearcoat_roughness_0_03() {
    let m = MaterialGpuExtended::default();
    assert!((m.clearcoat_roughness - 0.03).abs() < 1e-4);
}

#[test]
fn default_anisotropy_strength_zero() {
    let m = MaterialGpuExtended::default();
    assert!((m.anisotropy_strength - 0.0).abs() < 1e-6);
}

#[test]
fn default_subsurface_color_white() {
    let m = MaterialGpuExtended::default();
    assert_eq!(m.subsurface_color, [1.0, 1.0, 1.0]);
}

#[test]
fn default_subsurface_scale_zero() {
    let m = MaterialGpuExtended::default();
    assert!((m.subsurface_scale - 0.0).abs() < 1e-6);
}

#[test]
fn default_subsurface_radius_1() {
    let m = MaterialGpuExtended::default();
    assert!((m.subsurface_radius - 1.0).abs() < 1e-6);
}

#[test]
fn default_sheen_color_zero() {
    let m = MaterialGpuExtended::default();
    assert_eq!(m.sheen_color, [0.0, 0.0, 0.0]);
}

#[test]
fn default_sheen_roughness_0_5() {
    let m = MaterialGpuExtended::default();
    assert!((m.sheen_roughness - 0.5).abs() < 1e-6);
}

#[test]
fn default_transmission_zero() {
    let m = MaterialGpuExtended::default();
    assert!((m.transmission_factor - 0.0).abs() < 1e-6);
}

#[test]
fn default_ior_1_5() {
    let m = MaterialGpuExtended::default();
    assert!((m.ior - 1.5).abs() < 1e-4);
}

#[test]
fn default_attenuation_color_white() {
    let m = MaterialGpuExtended::default();
    assert_eq!(m.attenuation_color, [1.0, 1.0, 1.0]);
}

#[test]
fn default_attenuation_distance_1() {
    let m = MaterialGpuExtended::default();
    assert!((m.attenuation_distance - 1.0).abs() < 1e-6);
}

#[test]
fn default_no_features_active() {
    let m = MaterialGpuExtended::default();
    assert!(!m.has_feature(MATERIAL_FLAG_CLEARCOAT));
    assert!(!m.has_feature(MATERIAL_FLAG_ANISOTROPY));
    assert!(!m.has_feature(MATERIAL_FLAG_SUBSURFACE));
    assert!(!m.has_feature(MATERIAL_FLAG_SHEEN));
    assert!(!m.has_feature(MATERIAL_FLAG_TRANSMISSION));
}

// ═══════════════════════════════════════════════════════════════════════════════
//  car_paint()
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn car_paint_has_clearcoat_flag() {
    let m = MaterialGpuExtended::car_paint(Vec3::new(0.8, 0.0, 0.0), 0.9, 0.3);
    assert!(m.has_feature(MATERIAL_FLAG_CLEARCOAT));
}

#[test]
fn car_paint_no_other_flags() {
    let m = MaterialGpuExtended::car_paint(Vec3::new(0.8, 0.0, 0.0), 0.9, 0.3);
    assert!(!m.has_feature(MATERIAL_FLAG_ANISOTROPY));
    assert!(!m.has_feature(MATERIAL_FLAG_SUBSURFACE));
    assert!(!m.has_feature(MATERIAL_FLAG_SHEEN));
    assert!(!m.has_feature(MATERIAL_FLAG_TRANSMISSION));
}

#[test]
fn car_paint_base_color() {
    let m = MaterialGpuExtended::car_paint(Vec3::new(0.8, 0.1, 0.2), 0.9, 0.3);
    assert!((m.base_color_factor[0] - 0.8).abs() < 1e-6);
    assert!((m.base_color_factor[1] - 0.1).abs() < 1e-6);
    assert!((m.base_color_factor[2] - 0.2).abs() < 1e-6);
    assert!((m.base_color_factor[3] - 1.0).abs() < 1e-6); // alpha always 1
}

#[test]
fn car_paint_metallic_and_roughness() {
    let m = MaterialGpuExtended::car_paint(Vec3::ONE, 0.95, 0.2);
    assert!((m.metallic_factor - 0.95).abs() < 1e-6);
    assert!((m.roughness_factor - 0.2).abs() < 1e-6);
}

#[test]
fn car_paint_clearcoat_strength_1() {
    let m = MaterialGpuExtended::car_paint(Vec3::ONE, 1.0, 0.1);
    assert!((m.clearcoat_strength - 1.0).abs() < 1e-6);
}

#[test]
fn car_paint_clearcoat_roughness_0_05() {
    let m = MaterialGpuExtended::car_paint(Vec3::ONE, 1.0, 0.1);
    assert!((m.clearcoat_roughness - 0.05).abs() < 1e-4);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  brushed_metal()
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn brushed_metal_has_anisotropy_flag() {
    let m = MaterialGpuExtended::brushed_metal(Vec3::new(0.8, 0.8, 0.9), 0.4, 0.7, 0.0);
    assert!(m.has_feature(MATERIAL_FLAG_ANISOTROPY));
}

#[test]
fn brushed_metal_metallic_1() {
    let m = MaterialGpuExtended::brushed_metal(Vec3::ONE, 0.4, 0.7, 0.0);
    assert!((m.metallic_factor - 1.0).abs() < 1e-6);
}

#[test]
fn brushed_metal_anisotropy_values() {
    let m = MaterialGpuExtended::brushed_metal(Vec3::ONE, 0.3, 0.8, 1.57);
    assert!((m.anisotropy_strength - 0.8).abs() < 1e-6);
    assert!((m.anisotropy_rotation - 1.57).abs() < 1e-4);
    assert!((m.roughness_factor - 0.3).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  skin()
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn skin_has_subsurface_flag() {
    let m = MaterialGpuExtended::skin(Vec3::new(0.9, 0.7, 0.6), Vec3::new(1.0, 0.2, 0.1), 2.0, 0.5);
    assert!(m.has_feature(MATERIAL_FLAG_SUBSURFACE));
}

#[test]
fn skin_subsurface_tint_stored() {
    let tint = Vec3::new(1.0, 0.2, 0.1);
    let m = MaterialGpuExtended::skin(Vec3::ONE, tint, 2.0, 0.5);
    assert!((m.subsurface_color[0] - 1.0).abs() < 1e-6);
    assert!((m.subsurface_color[1] - 0.2).abs() < 1e-6);
    assert!((m.subsurface_color[2] - 0.1).abs() < 1e-6);
}

#[test]
fn skin_radius_and_scale() {
    let m = MaterialGpuExtended::skin(Vec3::ONE, Vec3::ONE, 3.0, 0.8);
    assert!((m.subsurface_radius - 3.0).abs() < 1e-6);
    assert!((m.subsurface_scale - 0.8).abs() < 1e-6);
}

#[test]
fn skin_not_metallic() {
    let m = MaterialGpuExtended::skin(Vec3::ONE, Vec3::ONE, 1.0, 1.0);
    assert!((m.metallic_factor - 0.0).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  velvet()
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn velvet_has_sheen_flag() {
    let m = MaterialGpuExtended::velvet(Vec3::new(0.3, 0.0, 0.5), Vec3::new(1.0, 0.8, 0.9), 0.4);
    assert!(m.has_feature(MATERIAL_FLAG_SHEEN));
}

#[test]
fn velvet_sheen_color_stored() {
    let m = MaterialGpuExtended::velvet(Vec3::ONE, Vec3::new(0.5, 0.6, 0.7), 0.3);
    assert!((m.sheen_color[0] - 0.5).abs() < 1e-6);
    assert!((m.sheen_color[1] - 0.6).abs() < 1e-6);
    assert!((m.sheen_color[2] - 0.7).abs() < 1e-6);
}

#[test]
fn velvet_sheen_roughness() {
    let m = MaterialGpuExtended::velvet(Vec3::ONE, Vec3::ONE, 0.35);
    assert!((m.sheen_roughness - 0.35).abs() < 1e-6);
}

#[test]
fn velvet_roughness_0_8() {
    let m = MaterialGpuExtended::velvet(Vec3::ONE, Vec3::ONE, 0.5);
    assert!((m.roughness_factor - 0.8).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  glass()
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn glass_has_transmission_flag() {
    let m = MaterialGpuExtended::glass(Vec3::ONE, 0.0, 1.0, 1.5, Vec3::ONE, 10.0);
    assert!(m.has_feature(MATERIAL_FLAG_TRANSMISSION));
}

#[test]
fn glass_ior_stored() {
    let m = MaterialGpuExtended::glass(Vec3::ONE, 0.05, 0.95, 1.45, Vec3::ONE, 5.0);
    assert!((m.ior - 1.45).abs() < 1e-4);
}

#[test]
fn glass_transmission_factor() {
    let m = MaterialGpuExtended::glass(Vec3::ONE, 0.0, 0.8, 1.5, Vec3::ONE, 10.0);
    assert!((m.transmission_factor - 0.8).abs() < 1e-6);
}

#[test]
fn glass_attenuation_values() {
    let m = MaterialGpuExtended::glass(Vec3::ONE, 0.0, 1.0, 1.5, Vec3::new(0.9, 0.95, 1.0), 7.5);
    assert!((m.attenuation_color[0] - 0.9).abs() < 1e-6);
    assert!((m.attenuation_distance - 7.5).abs() < 1e-6);
}

#[test]
fn glass_not_metallic() {
    let m = MaterialGpuExtended::glass(Vec3::ONE, 0.0, 1.0, 1.5, Vec3::ONE, 10.0);
    assert!((m.metallic_factor - 0.0).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Feature flag API
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn enable_feature_sets_bit() {
    let mut m = MaterialGpuExtended::default();
    m.enable_feature(MATERIAL_FLAG_CLEARCOAT);
    assert!(m.has_feature(MATERIAL_FLAG_CLEARCOAT));
}

#[test]
fn enable_multiple_features() {
    let mut m = MaterialGpuExtended::default();
    m.enable_feature(MATERIAL_FLAG_CLEARCOAT);
    m.enable_feature(MATERIAL_FLAG_SHEEN);
    assert!(m.has_feature(MATERIAL_FLAG_CLEARCOAT));
    assert!(m.has_feature(MATERIAL_FLAG_SHEEN));
    assert!(!m.has_feature(MATERIAL_FLAG_ANISOTROPY));
}

#[test]
fn disable_feature_clears_bit() {
    let mut m = MaterialGpuExtended::default();
    m.enable_feature(MATERIAL_FLAG_CLEARCOAT);
    m.enable_feature(MATERIAL_FLAG_SHEEN);
    m.disable_feature(MATERIAL_FLAG_CLEARCOAT);
    assert!(!m.has_feature(MATERIAL_FLAG_CLEARCOAT));
    assert!(m.has_feature(MATERIAL_FLAG_SHEEN)); // other flag untouched
}

#[test]
fn disable_already_disabled_is_noop() {
    let mut m = MaterialGpuExtended::default();
    m.disable_feature(MATERIAL_FLAG_TRANSMISSION);
    assert!(!m.has_feature(MATERIAL_FLAG_TRANSMISSION));
    assert_eq!(m.flags, 0);
}

#[test]
fn enable_all_flags() {
    let mut m = MaterialGpuExtended::default();
    m.enable_feature(MATERIAL_FLAG_CLEARCOAT);
    m.enable_feature(MATERIAL_FLAG_ANISOTROPY);
    m.enable_feature(MATERIAL_FLAG_SUBSURFACE);
    m.enable_feature(MATERIAL_FLAG_SHEEN);
    m.enable_feature(MATERIAL_FLAG_TRANSMISSION);
    assert_eq!(m.flags, 0x1F);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Pod / Zeroable
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn material_extended_pod_roundtrip() {
    let m = MaterialGpuExtended::car_paint(Vec3::new(0.8, 0.0, 0.1), 0.9, 0.2);
    let bytes = bytemuck::bytes_of(&m);
    assert_eq!(bytes.len(), 256);
    let back: &MaterialGpuExtended = bytemuck::from_bytes(bytes);
    assert_eq!(back.flags, m.flags);
    assert_eq!(back.base_color_factor, m.base_color_factor);
    assert_eq!(back.clearcoat_strength, m.clearcoat_strength);
}

#[test]
fn material_extended_zeroed() {
    let m: MaterialGpuExtended = bytemuck::Zeroable::zeroed();
    // Everything zero including base_color (unlike Default which has white base)
    assert_eq!(m.base_color_factor, [0.0; 4]);
    assert_eq!(m.flags, 0);
}
