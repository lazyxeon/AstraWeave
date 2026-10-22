//! Wave 2 Mutation Remediation Tests — biome_transition.rs
//!
//! Pins every numeric constant in BiomeVisuals::for_biome() across 8 biomes,
//! exercises EasingFunction::apply() exact formulas, TransitionConfig defaults,
//! TransitionEffect state-machine, lerp boundaries, and to_sky_config conversion.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::biome_transition::{
    BiomeVisuals, EasingFunction, TransitionConfig, TransitionEffect,
};
use astraweave_terrain::biome::BiomeType;
use glam::Vec3;

// ═══════════════════════════════════════════════════════════════════════
// Helper: assert a Vec3 matches exact components
// ═══════════════════════════════════════════════════════════════════════
fn assert_vec3(v: Vec3, x: f32, y: f32, z: f32, label: &str) {
    assert!(
        (v.x - x).abs() < 1e-6 && (v.y - y).abs() < 1e-6 && (v.z - z).abs() < 1e-6,
        "{label}: expected ({x}, {y}, {z}), got ({}, {}, {})",
        v.x,
        v.y,
        v.z
    );
}

// ═══════════════════════════════════════════════════════════════════════
// TransitionConfig defaults
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn transition_config_default_duration() {
    assert_eq!(TransitionConfig::default().duration, 2.0);
}
#[test]
fn transition_config_default_easing() {
    assert_eq!(
        TransitionConfig::default().easing,
        EasingFunction::SmoothStep
    );
}
#[test]
fn transition_config_default_blend_fog() {
    assert!(TransitionConfig::default().blend_fog);
}
#[test]
fn transition_config_default_blend_ambient() {
    assert!(TransitionConfig::default().blend_ambient);
}
#[test]
fn transition_config_default_apply_tint() {
    assert!(!TransitionConfig::default().apply_tint);
}
#[test]
fn transition_config_default_tint_alpha() {
    assert_eq!(TransitionConfig::default().tint_alpha, 0.15);
}

// ═══════════════════════════════════════════════════════════════════════
// EasingFunction::apply() — exact math for every variant
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn easing_linear_at_quarter() {
    assert!((EasingFunction::Linear.apply(0.25) - 0.25).abs() < 1e-6);
}
#[test]
fn easing_linear_at_three_quarters() {
    assert!((EasingFunction::Linear.apply(0.75) - 0.75).abs() < 1e-6);
}
#[test]
fn easing_smoothstep_at_quarter() {
    // t*t*(3 - 2*t) at t=0.25 = 0.0625 * 2.5 = 0.15625
    let expected = 0.25_f32 * 0.25 * (3.0 - 2.0 * 0.25);
    assert!((EasingFunction::SmoothStep.apply(0.25) - expected).abs() < 1e-6);
}
#[test]
fn easing_smoothstep_at_three_quarters() {
    let expected = 0.75_f32 * 0.75 * (3.0 - 2.0 * 0.75);
    assert!((EasingFunction::SmoothStep.apply(0.75) - expected).abs() < 1e-6);
}
#[test]
fn easing_smootherstep_at_half() {
    // t^3*(t*(6t-15)+10) at 0.5 = 0.125*(0.5*(3-15)+10) = 0.125*(−6+10) = 0.5
    let t = 0.5_f32;
    let expected = t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    assert!((EasingFunction::SmootherStep.apply(0.5) - expected).abs() < 1e-6);
}
#[test]
fn easing_smootherstep_at_quarter() {
    let t = 0.25_f32;
    let expected = t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    assert!((EasingFunction::SmootherStep.apply(0.25) - expected).abs() < 1e-6);
}
#[test]
fn easing_ease_in_at_half() {
    // t*t at 0.5 = 0.25
    assert!((EasingFunction::EaseIn.apply(0.5) - 0.25).abs() < 1e-6);
}
#[test]
fn easing_ease_in_at_quarter() {
    assert!((EasingFunction::EaseIn.apply(0.25) - 0.0625).abs() < 1e-6);
}
#[test]
fn easing_ease_out_at_half() {
    // 1-(1-t)^2 at 0.5 = 1 - 0.25 = 0.75
    assert!((EasingFunction::EaseOut.apply(0.5) - 0.75).abs() < 1e-6);
}
#[test]
fn easing_ease_out_at_quarter() {
    // 1 - 0.75^2 = 1 - 0.5625 = 0.4375
    assert!((EasingFunction::EaseOut.apply(0.25) - 0.4375).abs() < 1e-6);
}
#[test]
fn easing_ease_in_out_below_half() {
    // 2*t*t at 0.25 = 0.125
    assert!((EasingFunction::EaseInOut.apply(0.25) - 0.125).abs() < 1e-6);
}
#[test]
fn easing_ease_in_out_at_half() {
    // 2*0.5*0.5 = 0.5
    assert!((EasingFunction::EaseInOut.apply(0.5) - 0.5).abs() < 1e-6);
}
#[test]
fn easing_ease_in_out_above_half() {
    // 1 - (-2*0.75 + 2)^2 / 2 = 1 - 0.5^2 / 2 = 1 - 0.125 = 0.875
    assert!((EasingFunction::EaseInOut.apply(0.75) - 0.875).abs() < 1e-6);
}
#[test]
fn easing_clamps_negative() {
    assert!((EasingFunction::SmoothStep.apply(-1.0) - 0.0).abs() < 1e-6);
}
#[test]
fn easing_clamps_above_one() {
    assert!((EasingFunction::SmoothStep.apply(2.0) - 1.0).abs() < 1e-6);
}
#[test]
fn easing_all_zero_at_zero() {
    for e in [
        EasingFunction::Linear,
        EasingFunction::SmoothStep,
        EasingFunction::SmootherStep,
        EasingFunction::EaseIn,
        EasingFunction::EaseOut,
        EasingFunction::EaseInOut,
    ] {
        assert!((e.apply(0.0)).abs() < 1e-6, "{:?} at 0.0", e);
    }
}
#[test]
fn easing_all_one_at_one() {
    for e in [
        EasingFunction::Linear,
        EasingFunction::SmoothStep,
        EasingFunction::SmootherStep,
        EasingFunction::EaseIn,
        EasingFunction::EaseOut,
        EasingFunction::EaseInOut,
    ] {
        assert!((e.apply(1.0) - 1.0).abs() < 1e-6, "{:?} at 1.0", e);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeVisuals::default() — pin every field
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn visuals_default_fog_color() {
    assert_vec3(
        BiomeVisuals::default().fog_color,
        0.7,
        0.75,
        0.8,
        "fog_color",
    );
}
#[test]
fn visuals_default_fog_density() {
    assert_eq!(BiomeVisuals::default().fog_density, 0.001);
}
#[test]
fn visuals_default_fog_start() {
    assert_eq!(BiomeVisuals::default().fog_start, 50.0);
}
#[test]
fn visuals_default_fog_end() {
    assert_eq!(BiomeVisuals::default().fog_end, 500.0);
}
#[test]
fn visuals_default_ambient_color() {
    assert_vec3(
        BiomeVisuals::default().ambient_color,
        0.4,
        0.45,
        0.5,
        "ambient_color",
    );
}
#[test]
fn visuals_default_ambient_intensity() {
    assert_eq!(BiomeVisuals::default().ambient_intensity, 0.3);
}
#[test]
fn visuals_default_sky_colors() {
    let d = BiomeVisuals::default();
    assert_vec3(d.sky_day_top, 0.3, 0.6, 1.0, "sky_day_top");
    assert_vec3(d.sky_day_horizon, 0.8, 0.9, 1.0, "sky_day_horizon");
    assert_vec3(d.sky_sunset_top, 0.8, 0.4, 0.2, "sky_sunset_top");
    assert_vec3(d.sky_sunset_horizon, 1.0, 0.6, 0.3, "sky_sunset_horizon");
    assert_vec3(d.sky_night_top, 0.0, 0.0, 0.1, "sky_night_top");
    assert_vec3(d.sky_night_horizon, 0.1, 0.1, 0.2, "sky_night_horizon");
}
#[test]
fn visuals_default_water_colors() {
    let d = BiomeVisuals::default();
    assert_vec3(d.water_deep, 0.02, 0.08, 0.2, "water_deep");
    assert_vec3(d.water_shallow, 0.1, 0.4, 0.5, "water_shallow");
    assert_vec3(d.water_foam, 0.95, 0.98, 1.0, "water_foam");
}
#[test]
fn visuals_default_cloud_weather() {
    let d = BiomeVisuals::default();
    assert_eq!(d.cloud_coverage, 0.3);
    assert_eq!(d.cloud_speed, 0.02);
    assert_eq!(d.weather_particle_density, 1.0);
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeVisuals::for_biome() — Forest (18 fields)
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn visuals_forest_fog() {
    let v = BiomeVisuals::for_biome(BiomeType::Forest);
    assert_vec3(v.fog_color, 0.4, 0.5, 0.35, "forest.fog_color");
    assert_eq!(v.fog_density, 0.003);
    assert_eq!(v.fog_start, 30.0);
    assert_eq!(v.fog_end, 300.0);
}
#[test]
fn visuals_forest_ambient() {
    let v = BiomeVisuals::for_biome(BiomeType::Forest);
    assert_vec3(v.ambient_color, 0.3, 0.4, 0.25, "forest.ambient_color");
    assert_eq!(v.ambient_intensity, 0.25);
}
#[test]
fn visuals_forest_sky() {
    let v = BiomeVisuals::for_biome(BiomeType::Forest);
    assert_vec3(v.sky_day_top, 0.25, 0.55, 0.85, "forest.sky_day_top");
    assert_vec3(v.sky_day_horizon, 0.6, 0.8, 0.7, "forest.sky_day_horizon");
    assert_vec3(v.sky_sunset_top, 0.6, 0.35, 0.2, "forest.sky_sunset_top");
    assert_vec3(
        v.sky_sunset_horizon,
        0.85,
        0.55,
        0.3,
        "forest.sky_sunset_horizon",
    );
    assert_vec3(v.sky_night_top, 0.0, 0.02, 0.08, "forest.sky_night_top");
    assert_vec3(
        v.sky_night_horizon,
        0.05,
        0.08,
        0.12,
        "forest.sky_night_horizon",
    );
}
#[test]
fn visuals_forest_water() {
    let v = BiomeVisuals::for_biome(BiomeType::Forest);
    assert_vec3(v.water_deep, 0.03, 0.1, 0.12, "forest.water_deep");
    assert_vec3(v.water_shallow, 0.08, 0.3, 0.25, "forest.water_shallow");
    assert_vec3(v.water_foam, 0.85, 0.9, 0.8, "forest.water_foam");
}
#[test]
fn visuals_forest_cloud_weather() {
    let v = BiomeVisuals::for_biome(BiomeType::Forest);
    assert_eq!(v.cloud_coverage, 0.5);
    assert_eq!(v.cloud_speed, 0.01);
    assert_eq!(v.weather_particle_density, 0.6);
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeVisuals::for_biome() — Desert (18 fields)
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn visuals_desert_fog() {
    let v = BiomeVisuals::for_biome(BiomeType::Desert);
    assert_vec3(v.fog_color, 0.9, 0.85, 0.7, "desert.fog_color");
    assert_eq!(v.fog_density, 0.0005);
    assert_eq!(v.fog_start, 100.0);
    assert_eq!(v.fog_end, 1000.0);
}
#[test]
fn visuals_desert_ambient() {
    let v = BiomeVisuals::for_biome(BiomeType::Desert);
    assert_vec3(v.ambient_color, 0.6, 0.55, 0.4, "desert.ambient_color");
    assert_eq!(v.ambient_intensity, 0.4);
}
#[test]
fn visuals_desert_sky() {
    let v = BiomeVisuals::for_biome(BiomeType::Desert);
    assert_vec3(v.sky_day_top, 0.35, 0.6, 0.95, "desert.sky_day_top");
    assert_vec3(v.sky_day_horizon, 0.95, 0.9, 0.8, "desert.sky_day_horizon");
    assert_vec3(v.sky_sunset_top, 0.9, 0.45, 0.15, "desert.sky_sunset_top");
    assert_vec3(
        v.sky_sunset_horizon,
        1.0,
        0.7,
        0.35,
        "desert.sky_sunset_horizon",
    );
    assert_vec3(v.sky_night_top, 0.02, 0.0, 0.12, "desert.sky_night_top");
    assert_vec3(
        v.sky_night_horizon,
        0.12,
        0.08,
        0.18,
        "desert.sky_night_horizon",
    );
}
#[test]
fn visuals_desert_water() {
    let v = BiomeVisuals::for_biome(BiomeType::Desert);
    assert_vec3(v.water_deep, 0.05, 0.12, 0.18, "desert.water_deep");
    assert_vec3(v.water_shallow, 0.15, 0.35, 0.3, "desert.water_shallow");
    assert_vec3(v.water_foam, 0.95, 0.92, 0.85, "desert.water_foam");
}
#[test]
fn visuals_desert_cloud_weather() {
    let v = BiomeVisuals::for_biome(BiomeType::Desert);
    assert_eq!(v.cloud_coverage, 0.1);
    assert_eq!(v.cloud_speed, 0.03);
    assert_eq!(v.weather_particle_density, 1.5);
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeVisuals::for_biome() — Grassland (18 fields)
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn visuals_grassland_fog() {
    let v = BiomeVisuals::for_biome(BiomeType::Grassland);
    assert_vec3(v.fog_color, 0.7, 0.8, 0.85, "grassland.fog_color");
    assert_eq!(v.fog_density, 0.001);
    assert_eq!(v.fog_start, 80.0);
    assert_eq!(v.fog_end, 600.0);
}
#[test]
fn visuals_grassland_ambient() {
    let v = BiomeVisuals::for_biome(BiomeType::Grassland);
    assert_vec3(v.ambient_color, 0.5, 0.55, 0.5, "grassland.ambient_color");
    assert_eq!(v.ambient_intensity, 0.35);
}
#[test]
fn visuals_grassland_sky() {
    let v = BiomeVisuals::for_biome(BiomeType::Grassland);
    assert_vec3(v.sky_day_top, 0.3, 0.6, 1.0, "grassland.sky_day_top");
    assert_vec3(
        v.sky_day_horizon,
        0.8,
        0.9,
        1.0,
        "grassland.sky_day_horizon",
    );
    assert_vec3(v.sky_sunset_top, 0.8, 0.4, 0.2, "grassland.sky_sunset_top");
    assert_vec3(
        v.sky_sunset_horizon,
        1.0,
        0.6,
        0.3,
        "grassland.sky_sunset_horizon",
    );
    assert_vec3(v.sky_night_top, 0.0, 0.0, 0.1, "grassland.sky_night_top");
    assert_vec3(
        v.sky_night_horizon,
        0.1,
        0.1,
        0.2,
        "grassland.sky_night_horizon",
    );
}
#[test]
fn visuals_grassland_water() {
    let v = BiomeVisuals::for_biome(BiomeType::Grassland);
    assert_vec3(v.water_deep, 0.02, 0.08, 0.2, "grassland.water_deep");
    assert_vec3(v.water_shallow, 0.1, 0.4, 0.5, "grassland.water_shallow");
    assert_vec3(v.water_foam, 0.95, 0.98, 1.0, "grassland.water_foam");
}
#[test]
fn visuals_grassland_cloud_weather() {
    let v = BiomeVisuals::for_biome(BiomeType::Grassland);
    assert_eq!(v.cloud_coverage, 0.4);
    assert_eq!(v.cloud_speed, 0.025);
    assert_eq!(v.weather_particle_density, 1.0);
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeVisuals::for_biome() — Mountain (18 fields)
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn visuals_mountain_fog() {
    let v = BiomeVisuals::for_biome(BiomeType::Mountain);
    assert_vec3(v.fog_color, 0.75, 0.8, 0.9, "mountain.fog_color");
    assert_eq!(v.fog_density, 0.002);
    assert_eq!(v.fog_start, 50.0);
    assert_eq!(v.fog_end, 400.0);
}
#[test]
fn visuals_mountain_ambient() {
    let v = BiomeVisuals::for_biome(BiomeType::Mountain);
    assert_vec3(v.ambient_color, 0.45, 0.5, 0.6, "mountain.ambient_color");
    assert_eq!(v.ambient_intensity, 0.3);
}
#[test]
fn visuals_mountain_sky() {
    let v = BiomeVisuals::for_biome(BiomeType::Mountain);
    assert_vec3(v.sky_day_top, 0.2, 0.5, 0.95, "mountain.sky_day_top");
    assert_vec3(
        v.sky_day_horizon,
        0.7,
        0.82,
        0.95,
        "mountain.sky_day_horizon",
    );
    assert_vec3(v.sky_sunset_top, 0.7, 0.35, 0.25, "mountain.sky_sunset_top");
    assert_vec3(
        v.sky_sunset_horizon,
        0.95,
        0.6,
        0.4,
        "mountain.sky_sunset_horizon",
    );
    assert_vec3(v.sky_night_top, 0.0, 0.0, 0.12, "mountain.sky_night_top");
    assert_vec3(
        v.sky_night_horizon,
        0.08,
        0.08,
        0.2,
        "mountain.sky_night_horizon",
    );
}
#[test]
fn visuals_mountain_water() {
    let v = BiomeVisuals::for_biome(BiomeType::Mountain);
    assert_vec3(v.water_deep, 0.01, 0.06, 0.2, "mountain.water_deep");
    assert_vec3(v.water_shallow, 0.05, 0.3, 0.5, "mountain.water_shallow");
    assert_vec3(v.water_foam, 0.98, 1.0, 1.0, "mountain.water_foam");
}
#[test]
fn visuals_mountain_cloud_weather() {
    let v = BiomeVisuals::for_biome(BiomeType::Mountain);
    assert_eq!(v.cloud_coverage, 0.6);
    assert_eq!(v.cloud_speed, 0.04);
    assert_eq!(v.weather_particle_density, 1.2);
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeVisuals::for_biome() — Tundra (18 fields)
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn visuals_tundra_fog() {
    let v = BiomeVisuals::for_biome(BiomeType::Tundra);
    assert_vec3(v.fog_color, 0.85, 0.9, 0.95, "tundra.fog_color");
    assert_eq!(v.fog_density, 0.002);
    assert_eq!(v.fog_start, 40.0);
    assert_eq!(v.fog_end, 350.0);
}
#[test]
fn visuals_tundra_ambient() {
    let v = BiomeVisuals::for_biome(BiomeType::Tundra);
    assert_vec3(v.ambient_color, 0.5, 0.55, 0.65, "tundra.ambient_color");
    assert_eq!(v.ambient_intensity, 0.35);
}
#[test]
fn visuals_tundra_sky() {
    let v = BiomeVisuals::for_biome(BiomeType::Tundra);
    assert_vec3(v.sky_day_top, 0.4, 0.65, 0.95, "tundra.sky_day_top");
    assert_vec3(v.sky_day_horizon, 0.85, 0.92, 1.0, "tundra.sky_day_horizon");
    assert_vec3(v.sky_sunset_top, 0.75, 0.4, 0.3, "tundra.sky_sunset_top");
    assert_vec3(
        v.sky_sunset_horizon,
        1.0,
        0.65,
        0.45,
        "tundra.sky_sunset_horizon",
    );
    assert_vec3(v.sky_night_top, 0.0, 0.01, 0.1, "tundra.sky_night_top");
    assert_vec3(
        v.sky_night_horizon,
        0.1,
        0.12,
        0.22,
        "tundra.sky_night_horizon",
    );
}
#[test]
fn visuals_tundra_water() {
    let v = BiomeVisuals::for_biome(BiomeType::Tundra);
    assert_vec3(v.water_deep, 0.02, 0.1, 0.25, "tundra.water_deep");
    assert_vec3(v.water_shallow, 0.12, 0.45, 0.55, "tundra.water_shallow");
    assert_vec3(v.water_foam, 1.0, 1.0, 1.0, "tundra.water_foam");
}
#[test]
fn visuals_tundra_cloud_weather() {
    let v = BiomeVisuals::for_biome(BiomeType::Tundra);
    assert_eq!(v.cloud_coverage, 0.55);
    assert_eq!(v.cloud_speed, 0.03);
    assert_eq!(v.weather_particle_density, 1.3);
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeVisuals::for_biome() — Swamp (18 fields)
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn visuals_swamp_fog() {
    let v = BiomeVisuals::for_biome(BiomeType::Swamp);
    assert_vec3(v.fog_color, 0.35, 0.4, 0.3, "swamp.fog_color");
    assert_eq!(v.fog_density, 0.005);
    assert_eq!(v.fog_start, 20.0);
    assert_eq!(v.fog_end, 150.0);
}
#[test]
fn visuals_swamp_ambient() {
    let v = BiomeVisuals::for_biome(BiomeType::Swamp);
    assert_vec3(v.ambient_color, 0.25, 0.3, 0.2, "swamp.ambient_color");
    assert_eq!(v.ambient_intensity, 0.2);
}
#[test]
fn visuals_swamp_sky() {
    let v = BiomeVisuals::for_biome(BiomeType::Swamp);
    assert_vec3(v.sky_day_top, 0.25, 0.45, 0.65, "swamp.sky_day_top");
    assert_vec3(v.sky_day_horizon, 0.55, 0.6, 0.5, "swamp.sky_day_horizon");
    assert_vec3(v.sky_sunset_top, 0.55, 0.3, 0.2, "swamp.sky_sunset_top");
    assert_vec3(
        v.sky_sunset_horizon,
        0.75,
        0.45,
        0.25,
        "swamp.sky_sunset_horizon",
    );
    assert_vec3(v.sky_night_top, 0.0, 0.02, 0.05, "swamp.sky_night_top");
    assert_vec3(
        v.sky_night_horizon,
        0.05,
        0.06,
        0.08,
        "swamp.sky_night_horizon",
    );
}
#[test]
fn visuals_swamp_water() {
    let v = BiomeVisuals::for_biome(BiomeType::Swamp);
    assert_vec3(v.water_deep, 0.04, 0.08, 0.04, "swamp.water_deep");
    assert_vec3(v.water_shallow, 0.12, 0.2, 0.1, "swamp.water_shallow");
    assert_vec3(v.water_foam, 0.6, 0.65, 0.5, "swamp.water_foam");
}
#[test]
fn visuals_swamp_cloud_weather() {
    let v = BiomeVisuals::for_biome(BiomeType::Swamp);
    assert_eq!(v.cloud_coverage, 0.7);
    assert_eq!(v.cloud_speed, 0.01);
    assert_eq!(v.weather_particle_density, 0.8);
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeVisuals::for_biome() — Beach (18 fields)
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn visuals_beach_fog() {
    let v = BiomeVisuals::for_biome(BiomeType::Beach);
    assert_vec3(v.fog_color, 0.75, 0.85, 0.9, "beach.fog_color");
    assert_eq!(v.fog_density, 0.0008);
    assert_eq!(v.fog_start, 100.0);
    assert_eq!(v.fog_end, 800.0);
}
#[test]
fn visuals_beach_ambient() {
    let v = BiomeVisuals::for_biome(BiomeType::Beach);
    assert_vec3(v.ambient_color, 0.55, 0.6, 0.65, "beach.ambient_color");
    assert_eq!(v.ambient_intensity, 0.4);
}
#[test]
fn visuals_beach_sky() {
    let v = BiomeVisuals::for_biome(BiomeType::Beach);
    assert_vec3(v.sky_day_top, 0.3, 0.65, 1.0, "beach.sky_day_top");
    assert_vec3(v.sky_day_horizon, 0.85, 0.93, 1.0, "beach.sky_day_horizon");
    assert_vec3(v.sky_sunset_top, 0.85, 0.45, 0.15, "beach.sky_sunset_top");
    assert_vec3(
        v.sky_sunset_horizon,
        1.0,
        0.7,
        0.35,
        "beach.sky_sunset_horizon",
    );
    assert_vec3(v.sky_night_top, 0.0, 0.01, 0.1, "beach.sky_night_top");
    assert_vec3(
        v.sky_night_horizon,
        0.08,
        0.1,
        0.2,
        "beach.sky_night_horizon",
    );
}
#[test]
fn visuals_beach_water() {
    let v = BiomeVisuals::for_biome(BiomeType::Beach);
    assert_vec3(v.water_deep, 0.0, 0.05, 0.25, "beach.water_deep");
    assert_vec3(v.water_shallow, 0.05, 0.45, 0.6, "beach.water_shallow");
    assert_vec3(v.water_foam, 1.0, 1.0, 1.0, "beach.water_foam");
}
#[test]
fn visuals_beach_cloud_weather() {
    let v = BiomeVisuals::for_biome(BiomeType::Beach);
    assert_eq!(v.cloud_coverage, 0.25);
    assert_eq!(v.cloud_speed, 0.02);
    assert_eq!(v.weather_particle_density, 1.0);
}

// ═══════════════════════════════════════════════════════════════════════
// BiomeVisuals::for_biome() — River (18 fields)
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn visuals_river_fog() {
    let v = BiomeVisuals::for_biome(BiomeType::River);
    assert_vec3(v.fog_color, 0.65, 0.75, 0.8, "river.fog_color");
    assert_eq!(v.fog_density, 0.0015);
    assert_eq!(v.fog_start, 60.0);
    assert_eq!(v.fog_end, 400.0);
}
#[test]
fn visuals_river_ambient() {
    let v = BiomeVisuals::for_biome(BiomeType::River);
    assert_vec3(v.ambient_color, 0.45, 0.5, 0.55, "river.ambient_color");
    assert_eq!(v.ambient_intensity, 0.3);
}
#[test]
fn visuals_river_sky() {
    let v = BiomeVisuals::for_biome(BiomeType::River);
    assert_vec3(v.sky_day_top, 0.3, 0.6, 0.95, "river.sky_day_top");
    assert_vec3(v.sky_day_horizon, 0.75, 0.85, 0.95, "river.sky_day_horizon");
    assert_vec3(v.sky_sunset_top, 0.75, 0.4, 0.2, "river.sky_sunset_top");
    assert_vec3(
        v.sky_sunset_horizon,
        0.95,
        0.6,
        0.3,
        "river.sky_sunset_horizon",
    );
    assert_vec3(v.sky_night_top, 0.0, 0.01, 0.08, "river.sky_night_top");
    assert_vec3(
        v.sky_night_horizon,
        0.08,
        0.1,
        0.18,
        "river.sky_night_horizon",
    );
}
#[test]
fn visuals_river_water() {
    let v = BiomeVisuals::for_biome(BiomeType::River);
    assert_vec3(v.water_deep, 0.02, 0.1, 0.18, "river.water_deep");
    assert_vec3(v.water_shallow, 0.08, 0.38, 0.4, "river.water_shallow");
    assert_vec3(v.water_foam, 0.9, 0.95, 0.92, "river.water_foam");
}
#[test]
fn visuals_river_cloud_weather() {
    let v = BiomeVisuals::for_biome(BiomeType::River);
    assert_eq!(v.cloud_coverage, 0.35);
    assert_eq!(v.cloud_speed, 0.02);
    assert_eq!(v.weather_particle_density, 1.0);
}

// ═══════════════════════════════════════════════════════════════════════
// TransitionEffect state machine
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn effect_default_not_active() {
    let e = TransitionEffect::default();
    assert!(!e.is_active());
    assert!(e.from_biome().is_none());
    assert!(e.to_biome().is_none());
    assert_eq!(e.raw_progress(), 0.0);
}
#[test]
fn effect_start_none_from_uses_target() {
    let mut e = TransitionEffect::default();
    e.start(None, BiomeType::Mountain);
    assert!(e.is_active());
    assert_eq!(e.from_biome(), Some(BiomeType::Mountain));
    assert_eq!(e.to_biome(), Some(BiomeType::Mountain));
}
#[test]
fn effect_blend_factor_uses_easing() {
    let cfg = TransitionConfig {
        duration: 1.0,
        easing: EasingFunction::EaseIn,
        ..Default::default()
    };
    let mut e = TransitionEffect::new(cfg);
    e.start(Some(BiomeType::Forest), BiomeType::Desert);
    e.update(0.5);
    // EaseIn at 0.5 → 0.25
    assert!((e.blend_factor() - 0.25).abs() < 0.01);
}
#[test]
fn effect_update_rate_inversely_proportional_to_duration() {
    let cfg = TransitionConfig {
        duration: 4.0,
        easing: EasingFunction::Linear,
        ..Default::default()
    };
    let mut e = TransitionEffect::new(cfg);
    e.start(Some(BiomeType::Beach), BiomeType::River);
    e.update(1.0); // 1s out of 4s = 25%
    assert!((e.raw_progress() - 0.25).abs() < 0.01);
}
#[test]
fn effect_completes_at_full_duration() {
    let cfg = TransitionConfig {
        duration: 2.0,
        easing: EasingFunction::Linear,
        ..Default::default()
    };
    let mut e = TransitionEffect::new(cfg);
    e.start(Some(BiomeType::Tundra), BiomeType::Swamp);
    e.update(2.0);
    assert!(!e.is_active());
    assert_eq!(e.raw_progress(), 1.0);
    // After completion, from_biome should be the target
    assert_eq!(e.from_biome(), Some(BiomeType::Swamp));
}
#[test]
fn effect_cancel_reverts_progress() {
    let mut e = TransitionEffect::default();
    e.start(Some(BiomeType::Grassland), BiomeType::Mountain);
    e.update(0.5);
    e.cancel();
    assert!(!e.is_active());
    assert_eq!(e.raw_progress(), 0.0);
    assert_eq!(e.to_biome(), e.from_biome());
}
#[test]
fn effect_complete_snaps_forward() {
    let mut e = TransitionEffect::default();
    e.start(Some(BiomeType::Desert), BiomeType::Forest);
    e.update(0.1);
    e.complete();
    assert!(!e.is_active());
    assert_eq!(e.raw_progress(), 1.0);
    assert_eq!(e.from_biome(), Some(BiomeType::Forest));
}
#[test]
fn effect_tint_alpha_inactive_is_zero() {
    let cfg = TransitionConfig {
        apply_tint: true,
        tint_alpha: 0.3,
        ..Default::default()
    };
    let e = TransitionEffect::new(cfg);
    assert_eq!(e.tint_alpha(), 0.0);
}
#[test]
fn effect_tint_alpha_bell_curve_midpoint() {
    let cfg = TransitionConfig {
        duration: 1.0,
        easing: EasingFunction::Linear,
        apply_tint: true,
        tint_alpha: 0.2,
        ..Default::default()
    };
    let mut e = TransitionEffect::new(cfg);
    e.start(Some(BiomeType::Forest), BiomeType::Mountain);
    e.update(0.5);
    // peak * 4 * 0.5 * 0.5 = 0.2 * 1.0 = 0.2
    assert!((e.tint_alpha() - 0.2).abs() < 0.01);
}
#[test]
fn effect_tint_alpha_quarter_point() {
    let cfg = TransitionConfig {
        duration: 1.0,
        easing: EasingFunction::Linear,
        apply_tint: true,
        tint_alpha: 0.2,
        ..Default::default()
    };
    let mut e = TransitionEffect::new(cfg);
    e.start(Some(BiomeType::Forest), BiomeType::Mountain);
    e.update(0.25);
    // 0.2 * 4 * 0.25 * 0.75 = 0.2 * 0.75 = 0.15
    assert!((e.tint_alpha() - 0.15).abs() < 0.01);
}
#[test]
fn effect_tint_disabled_always_zero() {
    let cfg = TransitionConfig {
        apply_tint: false,
        tint_alpha: 1.0,
        ..Default::default()
    };
    let mut e = TransitionEffect::new(cfg);
    e.start(Some(BiomeType::Forest), BiomeType::Desert);
    e.update(0.5);
    assert_eq!(e.tint_alpha(), 0.0);
}
#[test]
fn effect_tint_color_is_average_ambient() {
    let mut e = TransitionEffect::default();
    e.start(Some(BiomeType::Forest), BiomeType::Desert);
    let forest = BiomeVisuals::for_biome(BiomeType::Forest);
    let desert = BiomeVisuals::for_biome(BiomeType::Desert);
    let expected = (forest.ambient_color + desert.ambient_color) * 0.5;
    let got = e.tint_color();
    assert!((got - expected).length() < 1e-5);
}

// ═══════════════════════════════════════════════════════════════════════
// to_sky_config
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn to_sky_config_maps_all_fields() {
    let v = BiomeVisuals::for_biome(BiomeType::Mountain);
    let c = v.to_sky_config();
    assert_eq!(c.day_color_top, v.sky_day_top);
    assert_eq!(c.day_color_horizon, v.sky_day_horizon);
    assert_eq!(c.sunset_color_top, v.sky_sunset_top);
    assert_eq!(c.sunset_color_horizon, v.sky_sunset_horizon);
    assert_eq!(c.night_color_top, v.sky_night_top);
    assert_eq!(c.night_color_horizon, v.sky_night_horizon);
    assert_eq!(c.cloud_coverage, v.cloud_coverage);
    assert_eq!(c.cloud_speed, v.cloud_speed);
}
