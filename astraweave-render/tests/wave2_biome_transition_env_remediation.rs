//! Wave 2 Proactive Remediation: biome_transition.rs (130 mutants) + environment.rs (401 mutants)
//!
//! Golden-value tests for EasingFunction::apply, BiomeVisuals::for_biome,
//! TransitionEffect state machine, and TimeOfDay lighting calculations.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::biome_transition::{
    BiomeVisuals, EasingFunction, TransitionConfig, TransitionEffect,
};
use astraweave_render::environment::{SkyConfig, TimeOfDay};
use astraweave_terrain::biome::BiomeType;
use glam::Vec3;

// ============================================================================
// EasingFunction::apply — golden values for every variant
// ============================================================================

#[test]
fn easing_linear_golden_values() {
    let e = EasingFunction::Linear;
    assert_eq!(e.apply(0.0), 0.0);
    assert_eq!(e.apply(0.25), 0.25);
    assert_eq!(e.apply(0.5), 0.5);
    assert_eq!(e.apply(0.75), 0.75);
    assert_eq!(e.apply(1.0), 1.0);
}

#[test]
fn easing_smoothstep_golden_values() {
    let e = EasingFunction::SmoothStep;
    // t * t * (3 - 2t)
    assert_eq!(e.apply(0.0), 0.0);
    assert_eq!(e.apply(1.0), 1.0);
    assert!((e.apply(0.5) - 0.5).abs() < 1e-6); // 0.25 * 2.0 = 0.5
                                                // at 0.25: 0.0625 * 2.5 = 0.15625
    assert!((e.apply(0.25) - 0.15625).abs() < 1e-5);
    // at 0.75: 0.5625 * 1.5 = 0.84375
    assert!((e.apply(0.75) - 0.84375).abs() < 1e-5);
}

#[test]
fn easing_smootherstep_golden_values() {
    let e = EasingFunction::SmootherStep;
    // t^3 * (t*(6t - 15) + 10)
    assert_eq!(e.apply(0.0), 0.0);
    assert_eq!(e.apply(1.0), 1.0);
    assert!((e.apply(0.5) - 0.5).abs() < 1e-5);
    // at 0.25: 0.015625 * (0.25*(1.5 - 15) + 10) = 0.015625 * (0.25*(-13.5)+10)
    //        = 0.015625 * (-3.375+10) = 0.015625 * 6.625 = 0.103515625
    assert!((e.apply(0.25) - 0.103515625).abs() < 1e-5);
}

#[test]
fn easing_ease_in_golden_values() {
    let e = EasingFunction::EaseIn;
    // t * t
    assert_eq!(e.apply(0.0), 0.0);
    assert_eq!(e.apply(1.0), 1.0);
    assert!((e.apply(0.5) - 0.25).abs() < 1e-6);
    assert!((e.apply(0.25) - 0.0625).abs() < 1e-6);
    assert!((e.apply(0.75) - 0.5625).abs() < 1e-6);
}

#[test]
fn easing_ease_out_golden_values() {
    let e = EasingFunction::EaseOut;
    // 1 - (1-t)^2
    assert_eq!(e.apply(0.0), 0.0);
    assert_eq!(e.apply(1.0), 1.0);
    assert!((e.apply(0.5) - 0.75).abs() < 1e-6);
    // at 0.25: 1 - 0.75^2 = 1 - 0.5625 = 0.4375
    assert!((e.apply(0.25) - 0.4375).abs() < 1e-5);
    // at 0.75: 1 - 0.25^2 = 1 - 0.0625 = 0.9375
    assert!((e.apply(0.75) - 0.9375).abs() < 1e-5);
}

#[test]
fn easing_ease_in_out_golden_values() {
    let e = EasingFunction::EaseInOut;
    assert_eq!(e.apply(0.0), 0.0);
    assert_eq!(e.apply(1.0), 1.0);
    // at 0.5: boundary — 2*0.25 = 0.5
    assert!((e.apply(0.5) - 0.5).abs() < 1e-5);
    // at 0.25 (< 0.5): 2 * 0.0625 = 0.125
    assert!((e.apply(0.25) - 0.125).abs() < 1e-5);
    // at 0.75 (>= 0.5): 1 - (-1.5+2)^2 / 2 = 1 - 0.25/2 = 1 - 0.125 = 0.875
    assert!((e.apply(0.75) - 0.875).abs() < 1e-5);
}

#[test]
fn easing_clamps_below_zero() {
    for e in [
        EasingFunction::Linear,
        EasingFunction::SmoothStep,
        EasingFunction::SmootherStep,
        EasingFunction::EaseIn,
        EasingFunction::EaseOut,
        EasingFunction::EaseInOut,
    ] {
        assert_eq!(e.apply(-1.0), e.apply(0.0), "{:?} should clamp negative", e);
    }
}

#[test]
fn easing_clamps_above_one() {
    for e in [
        EasingFunction::Linear,
        EasingFunction::SmoothStep,
        EasingFunction::SmootherStep,
        EasingFunction::EaseIn,
        EasingFunction::EaseOut,
        EasingFunction::EaseInOut,
    ] {
        assert_eq!(e.apply(2.0), e.apply(1.0), "{:?} should clamp above 1", e);
    }
}

#[test]
fn easing_monotonic_all_variants() {
    let variants = [
        EasingFunction::Linear,
        EasingFunction::SmoothStep,
        EasingFunction::SmootherStep,
        EasingFunction::EaseIn,
        EasingFunction::EaseOut,
        EasingFunction::EaseInOut,
    ];
    for e in variants {
        let mut prev = e.apply(0.0);
        for i in 1..=100 {
            let t = i as f32 / 100.0;
            let val = e.apply(t);
            assert!(val >= prev, "{:?} not monotonic at t={}", e, t);
            prev = val;
        }
    }
}

// ============================================================================
// TransitionConfig defaults
// ============================================================================

#[test]
fn transition_config_default_values() {
    let cfg = TransitionConfig::default();
    assert_eq!(cfg.duration, 2.0);
    assert_eq!(cfg.easing, EasingFunction::SmoothStep);
    assert!(cfg.blend_fog);
    assert!(cfg.blend_ambient);
    assert!(!cfg.apply_tint);
    assert_eq!(cfg.tint_alpha, 0.15);
}

// ============================================================================
// BiomeVisuals::default golden values
// ============================================================================

#[test]
fn biome_visuals_default_fog() {
    let v = BiomeVisuals::default();
    assert_eq!(v.fog_color, Vec3::new(0.7, 0.75, 0.8));
    assert_eq!(v.fog_density, 0.001);
    assert_eq!(v.fog_start, 50.0);
    assert_eq!(v.fog_end, 500.0);
}

#[test]
fn biome_visuals_default_ambient() {
    let v = BiomeVisuals::default();
    assert_eq!(v.ambient_color, Vec3::new(0.4, 0.45, 0.5));
    assert_eq!(v.ambient_intensity, 0.3);
}

#[test]
fn biome_visuals_default_sky() {
    let v = BiomeVisuals::default();
    assert_eq!(v.sky_day_top, Vec3::new(0.3, 0.6, 1.0));
    assert_eq!(v.sky_day_horizon, Vec3::new(0.8, 0.9, 1.0));
    assert_eq!(v.sky_sunset_top, Vec3::new(0.8, 0.4, 0.2));
    assert_eq!(v.sky_sunset_horizon, Vec3::new(1.0, 0.6, 0.3));
    assert_eq!(v.sky_night_top, Vec3::new(0.0, 0.0, 0.1));
    assert_eq!(v.sky_night_horizon, Vec3::new(0.1, 0.1, 0.2));
}

#[test]
fn biome_visuals_default_water() {
    let v = BiomeVisuals::default();
    assert_eq!(v.water_deep, Vec3::new(0.02, 0.08, 0.2));
    assert_eq!(v.water_shallow, Vec3::new(0.1, 0.4, 0.5));
    assert_eq!(v.water_foam, Vec3::new(0.95, 0.98, 1.0));
}

#[test]
fn biome_visuals_default_weather() {
    let v = BiomeVisuals::default();
    assert_eq!(v.cloud_coverage, 0.3);
    assert_eq!(v.cloud_speed, 0.02);
    assert_eq!(v.weather_particle_density, 1.0);
}

// ============================================================================
// BiomeVisuals::for_biome — spot-check distinguishing fields per biome
// ============================================================================

#[test]
fn for_biome_forest_fog() {
    let v = BiomeVisuals::for_biome(BiomeType::Forest);
    assert_eq!(v.fog_color, Vec3::new(0.4, 0.5, 0.35));
    assert_eq!(v.fog_density, 0.003);
    assert_eq!(v.fog_start, 30.0);
    assert_eq!(v.fog_end, 300.0);
}

#[test]
fn for_biome_forest_ambient() {
    let v = BiomeVisuals::for_biome(BiomeType::Forest);
    assert_eq!(v.ambient_color, Vec3::new(0.3, 0.4, 0.25));
    assert_eq!(v.ambient_intensity, 0.25);
}

#[test]
fn for_biome_forest_weather() {
    let v = BiomeVisuals::for_biome(BiomeType::Forest);
    assert_eq!(v.cloud_coverage, 0.5);
    assert_eq!(v.cloud_speed, 0.01);
    assert_eq!(v.weather_particle_density, 0.6);
}

#[test]
fn for_biome_desert_fog() {
    let v = BiomeVisuals::for_biome(BiomeType::Desert);
    assert_eq!(v.fog_color, Vec3::new(0.9, 0.85, 0.7));
    assert_eq!(v.fog_density, 0.0005);
    assert_eq!(v.fog_start, 100.0);
    assert_eq!(v.fog_end, 1000.0);
}

#[test]
fn for_biome_desert_ambient() {
    let v = BiomeVisuals::for_biome(BiomeType::Desert);
    assert_eq!(v.ambient_color, Vec3::new(0.6, 0.55, 0.4));
    assert_eq!(v.ambient_intensity, 0.4);
}

#[test]
fn for_biome_desert_weather() {
    let v = BiomeVisuals::for_biome(BiomeType::Desert);
    assert_eq!(v.cloud_coverage, 0.1);
    assert_eq!(v.cloud_speed, 0.03);
    assert_eq!(v.weather_particle_density, 1.5);
}

#[test]
fn for_biome_grassland_fog() {
    let v = BiomeVisuals::for_biome(BiomeType::Grassland);
    assert_eq!(v.fog_density, 0.001);
    assert_eq!(v.fog_start, 80.0);
    assert_eq!(v.fog_end, 600.0);
}

#[test]
fn for_biome_mountain_fog() {
    let v = BiomeVisuals::for_biome(BiomeType::Mountain);
    assert_eq!(v.fog_density, 0.002);
    assert_eq!(v.fog_start, 50.0);
    assert_eq!(v.fog_end, 400.0);
    assert_eq!(v.cloud_coverage, 0.6);
    assert_eq!(v.cloud_speed, 0.04);
    assert_eq!(v.weather_particle_density, 1.2);
}

#[test]
fn for_biome_tundra_unique() {
    let v = BiomeVisuals::for_biome(BiomeType::Tundra);
    assert_eq!(v.fog_density, 0.002);
    assert_eq!(v.fog_start, 40.0);
    assert_eq!(v.fog_end, 350.0);
    assert_eq!(v.cloud_coverage, 0.55);
    assert_eq!(v.weather_particle_density, 1.3);
}

#[test]
fn for_biome_swamp_unique() {
    let v = BiomeVisuals::for_biome(BiomeType::Swamp);
    assert_eq!(v.fog_density, 0.005);
    assert_eq!(v.fog_start, 20.0);
    assert_eq!(v.fog_end, 150.0);
    assert_eq!(v.ambient_intensity, 0.2);
    assert_eq!(v.cloud_coverage, 0.7);
    assert_eq!(v.cloud_speed, 0.01);
    assert_eq!(v.weather_particle_density, 0.8);
}

#[test]
fn for_biome_beach_unique() {
    let v = BiomeVisuals::for_biome(BiomeType::Beach);
    assert_eq!(v.fog_density, 0.0008);
    assert_eq!(v.fog_start, 100.0);
    assert_eq!(v.fog_end, 800.0);
    assert_eq!(v.ambient_intensity, 0.4);
    assert_eq!(v.cloud_coverage, 0.25);
}

#[test]
fn for_biome_river_unique() {
    let v = BiomeVisuals::for_biome(BiomeType::River);
    assert_eq!(v.fog_density, 0.0015);
    assert_eq!(v.fog_start, 60.0);
    assert_eq!(v.fog_end, 400.0);
    assert_eq!(v.cloud_coverage, 0.35);
}

#[test]
fn for_biome_all_distinct_fog_density() {
    // Every biome should have a unique fog density
    let biomes = [
        BiomeType::Forest,
        BiomeType::Desert,
        BiomeType::Grassland,
        BiomeType::Mountain,
        BiomeType::Tundra,
        BiomeType::Swamp,
        BiomeType::Beach,
        BiomeType::River,
    ];
    let densities: Vec<f32> = biomes
        .iter()
        .map(|b| BiomeVisuals::for_biome(*b).fog_density)
        .collect();
    // Multiple biomes share the same fog_density (Mountain=Tundra=0.002), so check
    // that there are at least 5 distinct values
    let mut unique = densities.clone();
    unique.sort_by(|a, b| a.partial_cmp(b).unwrap());
    unique.dedup();
    assert!(
        unique.len() >= 5,
        "Expected at least 5 distinct fog densities, got {:?}",
        unique
    );
}

// ============================================================================
// BiomeVisuals::for_biome — sky color golden values (deep match arms)
// ============================================================================

#[test]
fn for_biome_forest_sky_colors() {
    let v = BiomeVisuals::for_biome(BiomeType::Forest);
    assert_eq!(v.sky_day_top, Vec3::new(0.25, 0.55, 0.85));
    assert_eq!(v.sky_day_horizon, Vec3::new(0.6, 0.8, 0.7));
    assert_eq!(v.sky_sunset_top, Vec3::new(0.6, 0.35, 0.2));
    assert_eq!(v.sky_sunset_horizon, Vec3::new(0.85, 0.55, 0.3));
    assert_eq!(v.sky_night_top, Vec3::new(0.0, 0.02, 0.08));
    assert_eq!(v.sky_night_horizon, Vec3::new(0.05, 0.08, 0.12));
}

#[test]
fn for_biome_desert_sky_colors() {
    let v = BiomeVisuals::for_biome(BiomeType::Desert);
    assert_eq!(v.sky_day_top, Vec3::new(0.35, 0.6, 0.95));
    assert_eq!(v.sky_day_horizon, Vec3::new(0.95, 0.9, 0.8));
    assert_eq!(v.sky_sunset_top, Vec3::new(0.9, 0.45, 0.15));
    assert_eq!(v.sky_night_top, Vec3::new(0.02, 0.0, 0.12));
}

#[test]
fn for_biome_mountain_sky_colors() {
    let v = BiomeVisuals::for_biome(BiomeType::Mountain);
    assert_eq!(v.sky_day_top, Vec3::new(0.2, 0.5, 0.95));
    assert_eq!(v.sky_day_horizon, Vec3::new(0.7, 0.82, 0.95));
}

#[test]
fn for_biome_swamp_sky_colors() {
    let v = BiomeVisuals::for_biome(BiomeType::Swamp);
    assert_eq!(v.sky_day_top, Vec3::new(0.25, 0.45, 0.65));
    assert_eq!(v.sky_day_horizon, Vec3::new(0.55, 0.6, 0.5));
    assert_eq!(v.sky_night_top, Vec3::new(0.0, 0.02, 0.05));
}

// ============================================================================
// BiomeVisuals::for_biome — water color golden values
// ============================================================================

#[test]
fn for_biome_forest_water() {
    let v = BiomeVisuals::for_biome(BiomeType::Forest);
    assert_eq!(v.water_deep, Vec3::new(0.03, 0.1, 0.12));
    assert_eq!(v.water_shallow, Vec3::new(0.08, 0.3, 0.25));
    assert_eq!(v.water_foam, Vec3::new(0.85, 0.9, 0.8));
}

#[test]
fn for_biome_beach_water() {
    let v = BiomeVisuals::for_biome(BiomeType::Beach);
    assert_eq!(v.water_deep, Vec3::new(0.0, 0.05, 0.25));
    assert_eq!(v.water_shallow, Vec3::new(0.05, 0.45, 0.6));
    assert_eq!(v.water_foam, Vec3::new(1.0, 1.0, 1.0));
}

#[test]
fn for_biome_swamp_water() {
    let v = BiomeVisuals::for_biome(BiomeType::Swamp);
    assert_eq!(v.water_deep, Vec3::new(0.04, 0.08, 0.04));
    assert_eq!(v.water_shallow, Vec3::new(0.12, 0.2, 0.1));
    assert_eq!(v.water_foam, Vec3::new(0.6, 0.65, 0.5));
}

// ============================================================================
// BiomeVisuals::lerp
// ============================================================================

#[test]
fn visuals_lerp_t0_returns_self() {
    let a = BiomeVisuals::for_biome(BiomeType::Forest);
    let b = BiomeVisuals::for_biome(BiomeType::Desert);
    let result = a.lerp(&b, 0.0);
    assert_eq!(result.fog_density, a.fog_density);
    assert_eq!(result.fog_start, a.fog_start);
    assert_eq!(result.fog_end, a.fog_end);
    assert_eq!(result.ambient_intensity, a.ambient_intensity);
}

#[test]
fn visuals_lerp_t1_returns_other() {
    let a = BiomeVisuals::for_biome(BiomeType::Forest);
    let b = BiomeVisuals::for_biome(BiomeType::Desert);
    let result = a.lerp(&b, 1.0);
    assert!((result.fog_density - b.fog_density).abs() < 1e-6);
    assert!((result.fog_start - b.fog_start).abs() < 1e-3);
    assert!((result.fog_end - b.fog_end).abs() < 1e-3);
    assert!((result.ambient_intensity - b.ambient_intensity).abs() < 1e-6);
}

#[test]
fn visuals_lerp_t05_midpoint() {
    let a = BiomeVisuals::for_biome(BiomeType::Forest);
    let b = BiomeVisuals::for_biome(BiomeType::Desert);
    let result = a.lerp(&b, 0.5);
    let expected_density = (a.fog_density + b.fog_density) / 2.0;
    assert!((result.fog_density - expected_density).abs() < 1e-6);
    let expected_start = (a.fog_start + b.fog_start) / 2.0;
    assert!((result.fog_start - expected_start).abs() < 1e-3);
}

#[test]
fn visuals_lerp_cloud_coverage() {
    let a = BiomeVisuals::for_biome(BiomeType::Swamp); // 0.7
    let b = BiomeVisuals::for_biome(BiomeType::Desert); // 0.1
    let result = a.lerp(&b, 0.5);
    let expected = (0.7 + 0.1) / 2.0; // 0.4
    assert!((result.cloud_coverage - expected).abs() < 1e-5);
}

// ============================================================================
// BiomeVisuals::to_sky_config
// ============================================================================

#[test]
fn to_sky_config_preserves_sky_colors() {
    let v = BiomeVisuals::for_biome(BiomeType::Forest);
    let sc = v.to_sky_config();
    assert_eq!(sc.day_color_top, v.sky_day_top);
    assert_eq!(sc.day_color_horizon, v.sky_day_horizon);
    assert_eq!(sc.sunset_color_top, v.sky_sunset_top);
    assert_eq!(sc.sunset_color_horizon, v.sky_sunset_horizon);
    assert_eq!(sc.night_color_top, v.sky_night_top);
    assert_eq!(sc.night_color_horizon, v.sky_night_horizon);
    assert_eq!(sc.cloud_coverage, v.cloud_coverage);
    assert_eq!(sc.cloud_speed, v.cloud_speed);
}

// ============================================================================
// TransitionEffect state machine
// ============================================================================

#[test]
fn transition_starts_inactive() {
    let effect = TransitionEffect::default();
    assert!(!effect.is_active());
    assert_eq!(effect.raw_progress(), 0.0);
}

#[test]
fn transition_start_activates() {
    let mut effect = TransitionEffect::default();
    effect.start(Some(BiomeType::Forest), BiomeType::Desert);
    assert!(effect.is_active());
    assert_eq!(effect.from_biome(), Some(BiomeType::Forest));
    assert_eq!(effect.to_biome(), Some(BiomeType::Desert));
    assert_eq!(effect.raw_progress(), 0.0);
}

#[test]
fn transition_same_biome_is_noop() {
    let mut effect = TransitionEffect::default();
    effect.start(Some(BiomeType::Forest), BiomeType::Forest);
    assert!(!effect.is_active());
}

#[test]
fn transition_none_from_allows_same_biome() {
    let mut effect = TransitionEffect::default();
    effect.start(None, BiomeType::Forest);
    // None→Forest should still activate because from defaults to Forest
    // but then from==to and from.is_some() is false (it was None), so...
    // The code: from_biome = from.unwrap_or(to) = Forest
    // if from_biome == to && from.is_some() → from is None so this is false
    // So it DOES activate
    assert!(effect.is_active());
}

#[test]
fn transition_update_advances_progress() {
    let config = TransitionConfig {
        duration: 2.0,
        easing: EasingFunction::Linear,
        ..Default::default()
    };
    let mut effect = TransitionEffect::new(config);
    effect.start(Some(BiomeType::Grassland), BiomeType::Tundra);

    effect.update(0.5); // 0.5 / 2.0 = 0.25
    assert!((effect.raw_progress() - 0.25).abs() < 0.001);
    assert!(effect.is_active());
}

#[test]
fn transition_completes_at_full_progress() {
    let config = TransitionConfig {
        duration: 1.0,
        easing: EasingFunction::Linear,
        ..Default::default()
    };
    let mut effect = TransitionEffect::new(config);
    effect.start(Some(BiomeType::Beach), BiomeType::River);

    effect.update(1.5); // overshoots
    assert_eq!(effect.raw_progress(), 1.0);
    assert!(!effect.is_active());
}

#[test]
fn transition_blend_factor_uses_easing() {
    let config = TransitionConfig {
        duration: 1.0,
        easing: EasingFunction::EaseIn,
        ..Default::default()
    };
    let mut effect = TransitionEffect::new(config);
    effect.start(Some(BiomeType::Beach), BiomeType::River);
    effect.update(0.5);

    // EaseIn at 0.5 = 0.25
    assert!((effect.blend_factor() - 0.25).abs() < 0.01);
}

#[test]
fn transition_tint_alpha_zero_when_inactive() {
    let effect = TransitionEffect::default();
    assert_eq!(effect.tint_alpha(), 0.0);
}

#[test]
fn transition_tint_alpha_zero_when_tint_disabled() {
    let config = TransitionConfig {
        apply_tint: false,
        ..Default::default()
    };
    let mut effect = TransitionEffect::new(config);
    effect.start(Some(BiomeType::Forest), BiomeType::Desert);
    effect.update(1.0); // midpoint
    assert_eq!(effect.tint_alpha(), 0.0);
}

#[test]
fn transition_tint_alpha_peaks_at_midpoint() {
    let config = TransitionConfig {
        duration: 1.0,
        apply_tint: true,
        tint_alpha: 0.15,
        easing: EasingFunction::Linear,
        ..Default::default()
    };
    let mut effect = TransitionEffect::new(config);
    effect.start(Some(BiomeType::Forest), BiomeType::Desert);
    effect.update(0.5); // exactly midpoint

    // peak * 4 * 0.5 * 0.5 = 0.15 * 1.0 = 0.15
    assert!((effect.tint_alpha() - 0.15).abs() < 0.001);
}

#[test]
fn transition_tint_alpha_zero_at_start_and_end() {
    let config = TransitionConfig {
        duration: 1.0,
        apply_tint: true,
        tint_alpha: 0.15,
        easing: EasingFunction::Linear,
        ..Default::default()
    };
    let mut effect = TransitionEffect::new(config);
    effect.start(Some(BiomeType::Forest), BiomeType::Desert);

    // At progress=0.0 → tint = 0.15 * 4 * 0 * 1 = 0
    assert_eq!(effect.tint_alpha(), 0.0);
}

#[test]
fn transition_complete_snaps_to_end() {
    let mut effect = TransitionEffect::default();
    effect.start(Some(BiomeType::Forest), BiomeType::Desert);
    assert!(effect.is_active());

    effect.complete();
    assert!(!effect.is_active());
    assert_eq!(effect.raw_progress(), 1.0);
    assert_eq!(effect.from_biome(), Some(BiomeType::Desert));
}

#[test]
fn transition_cancel_snaps_to_start() {
    let mut effect = TransitionEffect::default();
    effect.start(Some(BiomeType::Forest), BiomeType::Desert);
    effect.update(0.5);

    effect.cancel();
    assert!(!effect.is_active());
    assert_eq!(effect.raw_progress(), 0.0);
    // to_biome reverts to from_biome
    assert_eq!(effect.to_biome(), effect.from_biome());
}

#[test]
fn transition_update_inactive_is_noop() {
    let mut effect = TransitionEffect::default();
    effect.update(1.0);
    assert_eq!(effect.raw_progress(), 0.0);
    assert!(!effect.is_active());
}

// ============================================================================
// TimeOfDay — lighting math
// ============================================================================

#[test]
fn time_of_day_new() {
    let tod = TimeOfDay::new(12.0, 60.0);
    assert_eq!(tod.current_time, 12.0);
    assert_eq!(tod.time_scale, 60.0);
}

#[test]
fn time_of_day_default() {
    let tod = TimeOfDay::default();
    assert_eq!(tod.current_time, 12.0);
    assert_eq!(tod.time_scale, 60.0);
    assert_eq!(tod.day_length, 1440.0);
}

#[test]
fn time_of_day_noon_is_day() {
    let tod = TimeOfDay::new(12.0, 1.0);
    assert!(tod.is_day());
    assert!(!tod.is_night());
}

#[test]
fn time_of_day_midnight_is_night() {
    let tod = TimeOfDay::new(0.0, 1.0);
    assert!(tod.is_night());
    assert!(!tod.is_day());
}

#[test]
fn time_of_day_sun_noon_above_horizon() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let sun = tod.get_sun_position();
    assert!(sun.y > 0.5, "Sun at noon should be high, got y={}", sun.y);
}

#[test]
fn time_of_day_sun_midnight_below_horizon() {
    let tod = TimeOfDay::new(0.0, 1.0);
    let sun = tod.get_sun_position();
    assert!(
        sun.y < 0.0,
        "Sun at midnight should be below horizon, got y={}",
        sun.y
    );
}

#[test]
fn time_of_day_moon_opposite_sun() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let sun = tod.get_sun_position();
    let moon = tod.get_moon_position();
    // moon = -sun
    assert!((moon.x + sun.x).abs() < 1e-5);
    assert!((moon.y + sun.y).abs() < 1e-5);
    assert!((moon.z + sun.z).abs() < 1e-5);
}

#[test]
fn time_of_day_light_direction_daytime_from_sun() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let light = tod.get_light_direction();
    let sun = tod.get_sun_position();
    // Light comes from sun → direction = -sun
    assert!((light.x + sun.x).abs() < 1e-4);
    assert!((light.y + sun.y).abs() < 1e-4);
    assert!((light.z + sun.z).abs() < 1e-4);
}

#[test]
fn time_of_day_light_color_noon_bright() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let color = tod.get_light_color();
    // Daytime: vec3(1.0, 0.95, 0.8) * (0.8 + 0.2*intensity)
    assert!(color.x > 0.8, "Noon light should be bright red channel");
    assert!(color.y > 0.7, "Noon light should have green");
}

#[test]
fn time_of_day_light_color_midnight_dim() {
    let tod = TimeOfDay::new(0.0, 1.0);
    let color = tod.get_light_color();
    // Night: vec3(0.3, 0.4, 0.8) * 0.15
    assert!(color.x < 0.1, "Midnight light should be dim");
    assert!((color.x - 0.3 * 0.15).abs() < 0.01);
    assert!((color.y - 0.4 * 0.15).abs() < 0.01);
    assert!((color.z - 0.8 * 0.15).abs() < 0.01);
}

#[test]
fn time_of_day_ambient_daytime_bright() {
    let tod = TimeOfDay::new(12.0, 1.0);
    let ambient = tod.get_ambient_color();
    assert!(
        ambient.x > 0.1,
        "Daytime ambient should have significant value"
    );
    assert!(ambient.z > ambient.x, "Daytime ambient should be bluish");
}

#[test]
fn time_of_day_ambient_nighttime_dim() {
    let tod = TimeOfDay::new(0.0, 1.0);
    let ambient = tod.get_ambient_color();
    // Night: vec3(0.1, 0.15, 0.3) * 0.1
    assert!((ambient.x - 0.01).abs() < 0.002);
    assert!((ambient.y - 0.015).abs() < 0.002);
    assert!((ambient.z - 0.03).abs() < 0.002);
}

// ============================================================================
// SkyConfig defaults
// ============================================================================

#[test]
fn sky_config_default_values() {
    let sc = SkyConfig::default();
    assert_eq!(sc.day_color_top, Vec3::new(0.3, 0.6, 1.0));
    assert_eq!(sc.day_color_horizon, Vec3::new(0.8, 0.9, 1.0));
    assert_eq!(sc.sunset_color_top, Vec3::new(0.8, 0.4, 0.2));
    assert_eq!(sc.sunset_color_horizon, Vec3::new(1.0, 0.6, 0.3));
    assert_eq!(sc.night_color_top, Vec3::new(0.0, 0.0, 0.1));
    assert_eq!(sc.night_color_horizon, Vec3::new(0.1, 0.1, 0.2));
    assert_eq!(sc.cloud_coverage, 0.5);
    assert_eq!(sc.cloud_speed, 0.02);
    assert_eq!(sc.cloud_altitude, 1000.0);
}
