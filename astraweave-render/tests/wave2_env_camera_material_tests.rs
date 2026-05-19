//! Wave 2 Mutation Remediation Tests — environment, camera, advanced_post, material
//!
//! Pins SkyConfig defaults, TimeOfDay sun/moon/light math, Camera & CameraController,
//! TaaConfig/MotionBlurConfig/DofConfig/ColorGradingConfig defaults, BloomConfig,
//! MaterialGpuExtended defaults and factory methods, material flags.

use astraweave_render::material::MaterialGpu;
use astraweave_render::material_extended::{
    MaterialGpuExtended, MATERIAL_FLAG_ANISOTROPY, MATERIAL_FLAG_CLEARCOAT, MATERIAL_FLAG_SHEEN,
    MATERIAL_FLAG_SUBSURFACE, MATERIAL_FLAG_TRANSMISSION,
};
use astraweave_camera::{CameraController, FreeFly as Camera};
use astraweave_render::{
    ColorGradingConfig, DofConfig, MotionBlurConfig, SkyConfig,
    TaaConfig, TimeOfDay,
};
use glam::{vec3, Vec3};

// ═══════════════════════════════════════════════════════════════════════
// SkyConfig defaults
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn sky_config_default_day_color_top() {
    let c = SkyConfig::default();
    assert_eq!(c.day_color_top, vec3(0.3, 0.6, 1.0));
}
#[test]
fn sky_config_default_day_color_horizon() {
    assert_eq!(SkyConfig::default().day_color_horizon, vec3(0.8, 0.9, 1.0));
}
#[test]
fn sky_config_default_sunset_color_top() {
    assert_eq!(SkyConfig::default().sunset_color_top, vec3(0.8, 0.4, 0.2));
}
#[test]
fn sky_config_default_sunset_color_horizon() {
    assert_eq!(
        SkyConfig::default().sunset_color_horizon,
        vec3(1.0, 0.6, 0.3)
    );
}
#[test]
fn sky_config_default_night_color_top() {
    assert_eq!(SkyConfig::default().night_color_top, vec3(0.0, 0.0, 0.1));
}
#[test]
fn sky_config_default_night_color_horizon() {
    assert_eq!(
        SkyConfig::default().night_color_horizon,
        vec3(0.1, 0.1, 0.2)
    );
}
#[test]
fn sky_config_default_cloud_coverage() {
    assert_eq!(SkyConfig::default().cloud_coverage, 0.5);
}
#[test]
fn sky_config_default_cloud_speed() {
    assert_eq!(SkyConfig::default().cloud_speed, 0.02);
}
#[test]
fn sky_config_default_cloud_altitude() {
    assert_eq!(SkyConfig::default().cloud_altitude, 1000.0);
}

// ═══════════════════════════════════════════════════════════════════════
// TimeOfDay defaults and constructors
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn time_of_day_default_current_time() {
    assert_eq!(TimeOfDay::default().current_time, 12.0);
}
#[test]
fn time_of_day_default_time_scale() {
    assert_eq!(TimeOfDay::default().time_scale, 60.0);
}
#[test]
fn time_of_day_default_day_length() {
    assert_eq!(TimeOfDay::default().day_length, 1440.0);
}
#[test]
fn time_of_day_new_sets_fields() {
    let tod = TimeOfDay::new(6.0, 120.0);
    assert_eq!(tod.current_time, 6.0);
    assert_eq!(tod.time_scale, 120.0);
    assert_eq!(tod.day_length, 1440.0); // always 1440
}

// ═══════════════════════════════════════════════════════════════════════
// TimeOfDay sun position math
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn sun_position_noon_is_up() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let pos = tod.get_sun_position();
    assert!(pos.y > 0.5, "Noon sun should be high: {:?}", pos);
}
#[test]
fn sun_position_midnight_is_down() {
    let tod = TimeOfDay::new(0.0, 1.0);
    let pos = tod.get_sun_position();
    assert!(pos.y < -0.5, "Midnight sun should be below: {:?}", pos);
}
#[test]
fn sun_position_6am_near_horizon() {
    let tod = TimeOfDay::new(6.0, 1.0);
    let pos = tod.get_sun_position();
    // sin(0)=0, so y should be near zero
    assert!(pos.y.abs() < 0.1, "6AM sun near horizon: {:?}", pos);
}
#[test]
fn sun_position_18_near_horizon() {
    let tod = TimeOfDay::new(18.0, 1.0);
    let pos = tod.get_sun_position();
    assert!(pos.y.abs() < 0.1, "6PM sun near horizon: {:?}", pos);
}
#[test]
fn sun_position_is_normalized() {
    for h in [0.0, 3.0, 6.0, 9.0, 12.0, 15.0, 18.0, 21.0] {
        let tod = TimeOfDay::new(h, 1.0);
        let pos = tod.get_sun_position();
        assert!(
            (pos.length() - 1.0).abs() < 0.01,
            "Not unit at h={h}: {:?}",
            pos
        );
    }
}
#[test]
fn moon_is_opposite_sun() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let sun = tod.get_sun_position();
    let moon = tod.get_moon_position();
    assert!((sun + moon).length() < 0.01, "Sun + Moon should be ~0");
}

// ═══════════════════════════════════════════════════════════════════════
// TimeOfDay light direction
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn light_direction_day_from_sun() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let sun = tod.get_sun_position();
    let dir = tod.get_light_direction();
    // Light comes from sun: dir = -sun
    assert!((dir + sun).length() < 0.01);
}
#[test]
fn light_direction_night_from_moon() {
    let tod = TimeOfDay::new(0.0, 1.0);
    let moon = tod.get_moon_position();
    let dir = tod.get_light_direction();
    assert!((dir + moon).length() < 0.01);
}

// ═══════════════════════════════════════════════════════════════════════
// TimeOfDay light color branching
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn light_color_noon_warm_white() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let color = tod.get_light_color();
    // Daytime when sun_height > 0.2, base (1.0, 0.95, 0.8)
    assert!(color.x > 0.8);
    assert!(color.y > 0.75);
}
#[test]
fn light_color_midnight_cool_blue() {
    let tod = TimeOfDay::new(0.0, 1.0);
    let color = tod.get_light_color();
    // Night: (0.3, 0.4, 0.8) * 0.15
    assert!(color.x < 0.1);
    assert!(color.z > color.x);
}

// ═══════════════════════════════════════════════════════════════════════
// TimeOfDay is_day / is_night / is_twilight
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn is_day_at_noon() {
    assert!(TimeOfDay::new(12.0, 1.0).is_day());
}
#[test]
fn is_not_day_at_midnight() {
    assert!(!TimeOfDay::new(0.0, 1.0).is_day());
}
#[test]
fn is_night_at_midnight() {
    assert!(TimeOfDay::new(0.0, 1.0).is_night());
}
#[test]
fn is_not_night_at_noon() {
    assert!(!TimeOfDay::new(12.0, 1.0).is_night());
}

// ═══════════════════════════════════════════════════════════════════════
// Camera
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn camera_vp_not_identity() {
    let cam = Camera {
        position: Vec3::new(0.0, 5.0, -10.0),
        yaw: 0.0,
        pitch: 0.0,
        fovy: 1.0,
        aspect: 1.0,
        znear: 0.1,
        zfar: 100.0,
    };
    let vp = cam.vp();
    // VP should not be identity for a non-trivial camera
    assert!(vp != glam::Mat4::IDENTITY);
}
#[test]
fn camera_proj_aspect_affects_output() {
    let cam1 = Camera {
        position: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        fovy: 1.0,
        aspect: 1.0,
        znear: 0.1,
        zfar: 100.0,
    };
    let cam2 = Camera {
        position: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        fovy: 1.0,
        aspect: 2.0,
        znear: 0.1,
        zfar: 100.0,
    };
    assert!(cam1.proj_matrix() != cam2.proj_matrix());
}

// ═══════════════════════════════════════════════════════════════════════
// CameraController defaults
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn camera_controller_new_speed() {
    let cc = CameraController::new(5.0, 0.01);
    assert_eq!(cc.speed, 5.0);
}
#[test]
fn camera_controller_new_sensitivity() {
    let cc = CameraController::new(5.0, 0.01);
    assert_eq!(cc.sensitivity, 0.01);
}
#[test]
fn camera_controller_new_orbit_distance() {
    let cc = CameraController::new(5.0, 0.01);
    assert_eq!(cc.orbit_distance, 5.0);
}
#[test]
fn camera_controller_new_zoom_sensitivity() {
    let cc = CameraController::new(5.0, 0.01);
    assert_eq!(cc.zoom_sensitivity, 0.1);
}
#[test]
fn camera_controller_new_mouse_smooth() {
    let cc = CameraController::new(5.0, 0.01);
    assert_eq!(cc.mouse_smooth, 0.15);
}
#[test]
fn camera_controller_new_mouse_deadzone() {
    let cc = CameraController::new(5.0, 0.01);
    assert_eq!(cc.mouse_deadzone, 0.25);
}

// ═══════════════════════════════════════════════════════════════════════
// TaaConfig defaults
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn taa_config_default_enabled() {
    assert!(TaaConfig::default().enabled);
}
#[test]
fn taa_config_default_blend_factor() {
    assert_eq!(TaaConfig::default().blend_factor, 0.95);
}
#[test]
fn taa_config_default_jitter_scale() {
    assert_eq!(TaaConfig::default().jitter_scale, 1.0);
}

// ═══════════════════════════════════════════════════════════════════════
// MotionBlurConfig defaults
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn motion_blur_default_disabled() {
    assert!(!MotionBlurConfig::default().enabled);
}
#[test]
fn motion_blur_default_sample_count() {
    assert_eq!(MotionBlurConfig::default().sample_count, 8);
}
#[test]
fn motion_blur_default_strength() {
    assert_eq!(MotionBlurConfig::default().strength, 1.0);
}

// ═══════════════════════════════════════════════════════════════════════
// DofConfig defaults
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn dof_default_disabled() {
    assert!(!DofConfig::default().enabled);
}
#[test]
fn dof_default_focus_distance() {
    assert_eq!(DofConfig::default().focus_distance, 10.0);
}
#[test]
fn dof_default_focus_range() {
    assert_eq!(DofConfig::default().focus_range, 5.0);
}
#[test]
fn dof_default_bokeh_size() {
    assert_eq!(DofConfig::default().bokeh_size, 2.0);
}

// ═══════════════════════════════════════════════════════════════════════
// ColorGradingConfig defaults
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn color_grading_default_enabled() {
    assert!(ColorGradingConfig::default().enabled);
}
#[test]
fn color_grading_default_exposure() {
    assert_eq!(ColorGradingConfig::default().exposure, 0.0);
}
#[test]
fn color_grading_default_contrast() {
    assert_eq!(ColorGradingConfig::default().contrast, 1.0);
}
#[test]
fn color_grading_default_saturation() {
    assert_eq!(ColorGradingConfig::default().saturation, 1.0);
}
#[test]
fn color_grading_default_temperature() {
    assert_eq!(ColorGradingConfig::default().temperature, 0.0);
}
#[test]
fn color_grading_default_tint() {
    assert_eq!(ColorGradingConfig::default().tint, 0.0);
}

// ═══════════════════════════════════════════════════════════════════════
// Material flags (exact bit values)
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn material_flag_clearcoat() {
    assert_eq!(MATERIAL_FLAG_CLEARCOAT, 0x01);
}
#[test]
fn material_flag_anisotropy() {
    assert_eq!(MATERIAL_FLAG_ANISOTROPY, 0x02);
}
#[test]
fn material_flag_subsurface() {
    assert_eq!(MATERIAL_FLAG_SUBSURFACE, 0x04);
}
#[test]
fn material_flag_sheen() {
    assert_eq!(MATERIAL_FLAG_SHEEN, 0x08);
}
#[test]
fn material_flag_transmission() {
    assert_eq!(MATERIAL_FLAG_TRANSMISSION, 0x10);
}

// ═══════════════════════════════════════════════════════════════════════
// MaterialGpuExtended defaults
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn mat_ext_default_base_color_factor() {
    let m = MaterialGpuExtended::default();
    assert_eq!(m.base_color_factor, [1.0, 1.0, 1.0, 1.0]);
}
#[test]
fn mat_ext_default_metallic_factor() {
    assert_eq!(MaterialGpuExtended::default().metallic_factor, 0.0);
}
#[test]
fn mat_ext_default_roughness_factor() {
    assert_eq!(MaterialGpuExtended::default().roughness_factor, 0.5);
}
#[test]
fn mat_ext_default_occlusion_strength() {
    assert_eq!(MaterialGpuExtended::default().occlusion_strength, 1.0);
}
#[test]
fn mat_ext_default_emissive_factor() {
    assert_eq!(
        MaterialGpuExtended::default().emissive_factor,
        [0.0, 0.0, 0.0]
    );
}
#[test]
fn mat_ext_default_clearcoat_strength() {
    assert_eq!(MaterialGpuExtended::default().clearcoat_strength, 0.0);
}
#[test]
fn mat_ext_default_clearcoat_roughness() {
    assert_eq!(MaterialGpuExtended::default().clearcoat_roughness, 0.03);
}
#[test]
fn mat_ext_default_anisotropy_strength() {
    assert_eq!(MaterialGpuExtended::default().anisotropy_strength, 0.0);
}
#[test]
fn mat_ext_default_anisotropy_rotation() {
    assert_eq!(MaterialGpuExtended::default().anisotropy_rotation, 0.0);
}
#[test]
fn mat_ext_default_subsurface_color() {
    assert_eq!(
        MaterialGpuExtended::default().subsurface_color,
        [1.0, 1.0, 1.0]
    );
}
#[test]
fn mat_ext_default_subsurface_scale() {
    assert_eq!(MaterialGpuExtended::default().subsurface_scale, 0.0);
}
#[test]
fn mat_ext_default_subsurface_radius() {
    assert_eq!(MaterialGpuExtended::default().subsurface_radius, 1.0);
}
#[test]
fn mat_ext_default_sheen_color() {
    assert_eq!(MaterialGpuExtended::default().sheen_color, [0.0, 0.0, 0.0]);
}
#[test]
fn mat_ext_default_sheen_roughness() {
    assert_eq!(MaterialGpuExtended::default().sheen_roughness, 0.5);
}
#[test]
fn mat_ext_default_transmission_factor() {
    assert_eq!(MaterialGpuExtended::default().transmission_factor, 0.0);
}
#[test]
fn mat_ext_default_ior() {
    assert_eq!(MaterialGpuExtended::default().ior, 1.5);
}
#[test]
fn mat_ext_default_attenuation_color() {
    assert_eq!(
        MaterialGpuExtended::default().attenuation_color,
        [1.0, 1.0, 1.0]
    );
}
#[test]
fn mat_ext_default_attenuation_distance() {
    assert_eq!(MaterialGpuExtended::default().attenuation_distance, 1.0);
}

// ═══════════════════════════════════════════════════════════════════════
// MaterialGpuExtended factory methods
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn mat_ext_car_paint_clearcoat() {
    let m = MaterialGpuExtended::car_paint(vec3(1.0, 0.0, 0.0), 0.9, 0.2);
    assert_eq!(m.clearcoat_strength, 1.0);
    assert_eq!(m.clearcoat_roughness, 0.05);
    assert_eq!(m.metallic_factor, 0.9);
    assert_eq!(m.roughness_factor, 0.2);
}
#[test]
fn mat_ext_brushed_metal_metallic() {
    let m = MaterialGpuExtended::brushed_metal(vec3(0.8, 0.8, 0.8), 0.3, 0.7, 0.0);
    assert_eq!(m.metallic_factor, 1.0);
    assert_eq!(m.anisotropy_strength, 0.7);
    assert_eq!(m.anisotropy_rotation, 0.0);
}
#[test]
fn mat_ext_skin_subsurface() {
    let m = MaterialGpuExtended::skin(vec3(0.8, 0.6, 0.5), vec3(0.9, 0.4, 0.3), 2.0, 0.8);
    assert_eq!(m.metallic_factor, 0.0);
    assert_eq!(m.roughness_factor, 0.5);
    assert_eq!(m.subsurface_color, [0.9, 0.4, 0.3]);
    assert_eq!(m.subsurface_radius, 2.0);
    assert_eq!(m.subsurface_scale, 0.8);
}
#[test]
fn mat_ext_velvet_sheen() {
    let m = MaterialGpuExtended::velvet(vec3(0.5, 0.0, 0.5), vec3(0.8, 0.2, 0.8), 0.6);
    assert_eq!(m.metallic_factor, 0.0);
    assert_eq!(m.roughness_factor, 0.8);
    assert_eq!(m.sheen_color, [0.8, 0.2, 0.8]);
    assert_eq!(m.sheen_roughness, 0.6);
}
#[test]
fn mat_ext_glass_transmission() {
    let m = MaterialGpuExtended::glass(
        vec3(1.0, 1.0, 1.0),
        0.05,
        0.9,
        1.5,
        vec3(0.9, 0.9, 1.0),
        0.5,
    );
    assert_eq!(m.transmission_factor, 0.9);
    assert_eq!(m.ior, 1.5);
    assert_eq!(m.attenuation_color, [0.9, 0.9, 1.0]);
    assert_eq!(m.attenuation_distance, 0.5);
}

// ═══════════════════════════════════════════════════════════════════════
// MaterialGpuExtended::has_feature auto-flags
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn mat_ext_clearcoat_flag() {
    let m = MaterialGpuExtended::car_paint(vec3(1.0, 0.0, 0.0), 0.9, 0.2);
    assert!(m.has_feature(MATERIAL_FLAG_CLEARCOAT));
}
#[test]
fn mat_ext_anisotropy_flag() {
    let m = MaterialGpuExtended::brushed_metal(vec3(0.8, 0.8, 0.8), 0.3, 0.7, 0.0);
    assert!(m.has_feature(MATERIAL_FLAG_ANISOTROPY));
}
#[test]
fn mat_ext_subsurface_flag() {
    let m = MaterialGpuExtended::skin(vec3(0.8, 0.6, 0.5), vec3(0.9, 0.4, 0.3), 2.0, 0.8);
    assert!(m.has_feature(MATERIAL_FLAG_SUBSURFACE));
}
#[test]
fn mat_ext_sheen_flag() {
    let m = MaterialGpuExtended::velvet(vec3(0.5, 0.0, 0.5), vec3(0.8, 0.2, 0.8), 0.6);
    assert!(m.has_feature(MATERIAL_FLAG_SHEEN));
}
#[test]
fn mat_ext_transmission_flag() {
    let m = MaterialGpuExtended::glass(
        vec3(1.0, 1.0, 1.0),
        0.05,
        0.9,
        1.5,
        vec3(0.9, 0.9, 1.0),
        0.5,
    );
    assert!(m.has_feature(MATERIAL_FLAG_TRANSMISSION));
}
#[test]
fn mat_ext_default_no_flags() {
    let m = MaterialGpuExtended::default();
    assert!(!m.has_feature(MATERIAL_FLAG_CLEARCOAT));
    assert!(!m.has_feature(MATERIAL_FLAG_ANISOTROPY));
    assert!(!m.has_feature(MATERIAL_FLAG_SUBSURFACE));
    assert!(!m.has_feature(MATERIAL_FLAG_SHEEN));
    assert!(!m.has_feature(MATERIAL_FLAG_TRANSMISSION));
}

// ═══════════════════════════════════════════════════════════════════════
// MaterialGpu::neutral
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn material_gpu_neutral_factors() {
    let m = MaterialGpu::neutral(0);
    // factors: metallic=0, roughness=0.5, ao=1, alpha=1
    assert_eq!(m.factors[0], 0.0); // metallic
    assert_eq!(m.factors[1], 0.5); // roughness
    assert_eq!(m.factors[2], 1.0); // ao
    assert_eq!(m.factors[3], 1.0); // alpha
}
#[test]
fn material_gpu_neutral_tiling() {
    let m = MaterialGpu::neutral(0);
    assert_eq!(m.tiling_triplanar[0], 1.0); // tiling x
    assert_eq!(m.tiling_triplanar[1], 1.0); // tiling y
    assert_eq!(m.tiling_triplanar[2], 16.0); // triplanar scale
}
#[test]
fn material_gpu_neutral_flags_zero() {
    assert_eq!(MaterialGpu::neutral(0).flags, 0);
}
