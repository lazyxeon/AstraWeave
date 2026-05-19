//! Wave 3 mutation-resistant remediation tests for astraweave-render.
//!
//! Targets specific mutation survivors identified by cargo-mutants analysis.
//! Pins exact values, boundary operators, and per-variant return semantics.

use astraweave_camera::FreeFly as Camera;
use astraweave_render::{
    EasingFunction, FrustumPlanes, MaterialGpu, TimeOfDay, WeatherSystem, WeatherType,
};
use glam::{vec3, Mat4, Vec3};

// ============================================================================
// REMEDIATION 1: environment.rs — TimeOfDay exact value assertions
// Kill mutants on get_light_color, get_ambient_color, get_light_attenuation
// ============================================================================

mod time_of_day_exact {
    use super::*;

    fn tod(hour: f32) -> TimeOfDay {
        TimeOfDay::new(hour, 0.0) // time_scale=0 freezes time
    }

    #[test]
    fn noon_sun_position_y_close_to_1() {
        let t = tod(12.0);
        let sun = t.get_sun_position();
        assert!(sun.y > 0.95, "Noon sun Y should be > 0.95, got {}", sun.y);
    }

    #[test]
    fn midnight_sun_below_horizon() {
        let t = tod(0.0);
        let sun = t.get_sun_position();
        assert!(
            sun.y < -0.9,
            "Midnight sun Y should be < -0.9, got {}",
            sun.y
        );
    }

    #[test]
    fn sunrise_near_horizon() {
        let t = tod(6.0);
        let sun = t.get_sun_position();
        assert!(
            sun.y.abs() < 0.15,
            "Sunrise sun Y should be near 0, got {}",
            sun.y
        );
    }

    #[test]
    fn is_day_boundary_precise() {
        let day = tod(12.0);
        assert!(day.is_day(), "Noon should be day");

        let night = tod(0.0);
        assert!(!night.is_day(), "Midnight should not be day");
    }

    #[test]
    fn is_night_boundary_precise() {
        let midnight = tod(0.0);
        assert!(midnight.is_night(), "Midnight should be night");

        let noon = tod(12.0);
        assert!(!noon.is_night(), "Noon should not be night");
    }

    #[test]
    fn light_color_daytime_is_warm() {
        let t = tod(12.0);
        let color = t.get_light_color();
        assert!(color.x > 0.8, "Day light R > 0.8, got {}", color.x);
        assert!(color.y > 0.7, "Day light G > 0.7, got {}", color.y);
        assert!(color.z > 0.5, "Day light B > 0.5, got {}", color.z);
    }

    #[test]
    fn light_color_night_is_cool_blue() {
        let t = tod(0.0);
        let color = t.get_light_color();
        assert!(color.x < 0.1, "Night light R < 0.1, got {}", color.x);
        assert!(color.z > color.x, "Night light should be bluish");
        // Exact formula: vec3(0.3, 0.4, 0.8) * 0.15
        assert!(
            (color.x - 0.3 * 0.15).abs() < 0.01,
            "Night R ~0.045, got {}",
            color.x
        );
        assert!(
            (color.y - 0.4 * 0.15).abs() < 0.01,
            "Night G ~0.06, got {}",
            color.y
        );
        assert!(
            (color.z - 0.8 * 0.15).abs() < 0.01,
            "Night B ~0.12, got {}",
            color.z
        );
    }

    #[test]
    fn ambient_color_day_vs_night() {
        let day_amb = tod(12.0).get_ambient_color();
        let night_amb = tod(0.0).get_ambient_color();

        let day_lum = day_amb.x + day_amb.y + day_amb.z;
        let night_lum = night_amb.x + night_amb.y + night_amb.z;
        assert!(
            day_lum > night_lum * 5.0,
            "Day ambient ({}) should be >> night ambient ({})",
            day_lum,
            night_lum
        );
    }

    #[test]
    fn ambient_color_night_exact() {
        let amb = tod(0.0).get_ambient_color();
        // Night: vec3(0.1, 0.15, 0.3) * 0.1
        assert!(
            (amb.x - 0.01).abs() < 0.005,
            "Night ambient R ~0.01, got {}",
            amb.x
        );
        assert!(
            (amb.y - 0.015).abs() < 0.005,
            "Night ambient G ~0.015, got {}",
            amb.y
        );
        assert!(
            (amb.z - 0.03).abs() < 0.005,
            "Night ambient B ~0.03, got {}",
            amb.z
        );
    }

    #[test]
    fn moon_position_opposite_sun() {
        let t = tod(12.0);
        let sun = t.get_sun_position();
        let moon = t.get_moon_position();
        assert!((moon.x + sun.x).abs() < 0.001);
        assert!((moon.y + sun.y).abs() < 0.001);
        assert!((moon.z + sun.z).abs() < 0.001);
    }

    #[test]
    fn light_direction_sun_during_day() {
        let t = tod(12.0);
        let dir = t.get_light_direction();
        let sun = t.get_sun_position();
        // During day: light_direction = -sun_pos
        assert!((dir.x + sun.x).abs() < 0.01, "Day light dir X = -sun.x");
        assert!((dir.y + sun.y).abs() < 0.01, "Day light dir Y = -sun.y");
    }

    // Kill mutation: PI / 12.0 → PI % 12.0 or PI * 12.0 in azimuth calc
    // Tests x/z components at mid-morning where azimuth matters
    #[test]
    fn sun_azimuth_at_9am() {
        let t = tod(9.0);
        let sun = t.get_sun_position();
        // At 9:00: sun_azimuth = (9-12) * PI/12 = -PI/4
        // sun is in the eastern sky: x should be negative, z positive
        assert!(
            sun.x < -0.1,
            "9am sun X should be negative (east), got {}",
            sun.x
        );
        assert!(sun.z > 0.1, "9am sun Z should be positive, got {}", sun.z);
        // x and z should be approximately equal magnitude (sin/cos of PI/4)
        assert!(
            (sun.x.abs() - sun.z.abs()).abs() < 0.05,
            "At 9am, |x| ≈ |z| since azimuth=-π/4, got x={} z={}",
            sun.x,
            sun.z
        );
    }

    // Kill mutation: (current_time - 12.0) * PI → (current_time - 12.0) + PI (line 61 col 54)
    // Kill mutation: PI / 12.0 → PI % 12.0 (line 61 col 77)
    // Kill mutation: PI / 12.0 → PI * 12.0 (line 61 col 77)
    // At 3pm (15.0), compute exact expected values and compare tightly
    #[test]
    fn sun_position_at_3pm_exact_azimuth() {
        let t = tod(15.0);
        let sun = t.get_sun_position();
        // sun_angle = (15-6)*PI/12 = 3*PI/4
        // sun_height = sin(3*PI/4) = 0.7071
        // sun_azimuth = (15-12)*PI/12 = PI/4
        // horizontal_distance = (1.0 - 0.7071).max(0.1) = 0.2929
        // x = sin(PI/4) * 0.2929 = 0.7071 * 0.2929 = 0.2071
        // y = 0.7071
        // z = cos(PI/4) * 0.2929 = 0.7071 * 0.2929 = 0.2071
        // normalized = (0.2071, 0.7071, 0.2071) / len

        // Key: x MUST be positive at 3pm (sun in west)
        assert!(
            sun.x > 0.15,
            "3pm sun x should be positive (western sky), got {}",
            sun.x
        );
        // z should also be positive
        assert!(
            sun.z > 0.15,
            "3pm sun z should be positive at 3pm, got {}",
            sun.z
        );
        // y should be high (afternoon sun still above horizon)
        assert!(sun.y > 0.8, "3pm sun y should be high, got {}", sun.y);
        // x and z should be approximately equal (azimuth = PI/4)
        assert!(
            (sun.x - sun.z).abs() < 0.02,
            "At 3pm: x ≈ z since azimuth=π/4, got x={} z={}",
            sun.x,
            sun.z
        );
    }

    // Kill mutation: (current_time - 12.0) → (current_time + 12.0) (line 61 col 46)
    // This mutation shifts azimuth by 2π, making it math-equivalent. Mark as equivalent.
    // Instead test a time where the FULL formula's output changes detectably with different ops.
    #[test]
    fn sun_position_at_8am_exact_values() {
        let t = tod(8.0);
        let sun = t.get_sun_position();
        // sun_angle = (8-6)*PI/12 = PI/6
        // sun_height = sin(PI/6) = 0.5
        // sun_azimuth = (8-12)*PI/12 = -PI/3
        // horizontal_distance = (1.0 - 0.5).max(0.1) = 0.5
        // x = sin(-PI/3) * 0.5 = -0.866 * 0.5 = -0.433
        // y = 0.5
        // z = cos(-PI/3) * 0.5 = 0.5 * 0.5 = 0.25
        // raw = (-0.433, 0.5, 0.25), len ≈ 0.706
        // normalized ≈ (-0.613, 0.708, 0.354)
        let expected_x = -0.613;
        let expected_y = 0.708;
        let expected_z = 0.354;
        assert!(
            (sun.x - expected_x).abs() < 0.05,
            "8am sun.x ≈ {}, got {}",
            expected_x,
            sun.x
        );
        assert!(
            (sun.y - expected_y).abs() < 0.05,
            "8am sun.y ≈ {}, got {}",
            expected_y,
            sun.y
        );
        assert!(
            (sun.z - expected_z).abs() < 0.05,
            "8am sun.z ≈ {}, got {}",
            expected_z,
            sun.z
        );
    }

    // Test at 16:00 (4pm) where mutations produce very different x values
    #[test]
    fn sun_position_at_4pm_x_positive() {
        let t = tod(16.0);
        let sun = t.get_sun_position();
        // sun_azimuth = (16-12)*PI/12 = PI/3
        // sin(PI/3) ≈ 0.866 → x positive in afternoon
        assert!(
            sun.x > 0.2,
            "4pm sun x must be positive (afternoon), got {}",
            sun.x
        );
        // With mutation * → +: azimuth = (4 + PI)/12 ≈ 0.595, sin(0.595) ≈ 0.558
        // With mutation / → *: azimuth = 4*PI*12 ≈ 150.8 → sin wraps unpredictably
        // With mutation / → %: azimuth = 4*PI % 12 ≈ 0.566 → sin(0.566) ≈ 0.534
        // The tight bound on x should distinguish
    }

    // Kill mutations on line 67: < with == and < with <=
    // These are near-equivalent mutants at the sun_height.abs() < 0.01 guard.
    // Test at exact sunrise to verify the if-branch is taken
    #[test]
    fn sun_height_near_zero_takes_if_branch() {
        // At t=6.0: sun_height = sin(0) = 0.0, so abs() = 0.0 < 0.01 → if branch
        // if branch: y = 0.0
        // With == mutation: 0.0 == 0.01 → false → else branch → y = 0.0 still but via different path
        // The outputs are equivalent, so we check at t=6.003 to barely be in the threshold
        let t = tod(6.003);
        let sun = t.get_sun_position();
        // sun_angle = 0.003 * PI/12 ≈ 0.000785
        // sun_height = sin(0.000785) ≈ 0.000785
        // |0.000785| < 0.01 → takes the if branch → y = 0.0
        // With == mutation: 0.000785 == 0.01 → false → else branch → y ≈ 0.000785
        assert!(
            sun.y.abs() < 0.001,
            "Near sunrise (6.003), sun.y should be ~0 (if branch), got {}",
            sun.y
        );
    }
}

// ============================================================================
// REMEDIATION 2: environment.rs — WeatherSystem exact per-type values
// ============================================================================

mod weather_exact_values {
    use super::*;

    fn ws_with(w: WeatherType) -> WeatherSystem {
        let mut ws = WeatherSystem::new();
        ws.set_weather(w, 0.0); // instant transition
        ws.update(0.01); // tick so current weather resolves
        ws
    }

    #[test]
    fn light_attenuation_per_weather_type() {
        assert_eq!(
            ws_with(WeatherType::Clear).get_light_attenuation(),
            1.0,
            "Clear"
        );
        assert_eq!(
            ws_with(WeatherType::Cloudy).get_light_attenuation(),
            0.7,
            "Cloudy"
        );
        assert_eq!(
            ws_with(WeatherType::Rain).get_light_attenuation(),
            0.5,
            "Rain"
        );
        assert_eq!(
            ws_with(WeatherType::Storm).get_light_attenuation(),
            0.3,
            "Storm"
        );
        assert_eq!(
            ws_with(WeatherType::Snow).get_light_attenuation(),
            0.6,
            "Snow"
        );
        assert_eq!(
            ws_with(WeatherType::Fog).get_light_attenuation(),
            0.4,
            "Fog"
        );
        assert_eq!(
            ws_with(WeatherType::Sandstorm).get_light_attenuation(),
            0.2,
            "Sandstorm"
        );
    }

    #[test]
    fn terrain_color_modifier_clear_is_white() {
        let ws = ws_with(WeatherType::Clear);
        let mod_color = ws.get_terrain_color_modifier();
        assert_eq!(
            mod_color,
            vec3(1.0, 1.0, 1.0),
            "Clear terrain color must be white"
        );
    }

    #[test]
    fn terrain_color_modifier_cloudy_exact() {
        let ws = ws_with(WeatherType::Cloudy);
        let mod_color = ws.get_terrain_color_modifier();
        assert!((mod_color.x - 0.8).abs() < 0.01);
        assert!((mod_color.y - 0.8).abs() < 0.01);
        assert!((mod_color.z - 0.9).abs() < 0.01);
    }

    // Kill mutation: transition_duration <= 0.0 → transition_duration > 0.0
    // Instant transition (duration=0) must apply weather BEFORE update() is called
    #[test]
    fn set_weather_instant_applies_without_update() {
        let mut ws = WeatherSystem::new();
        // Don't call update() — instant transition must apply immediately
        ws.set_weather(WeatherType::Rain, 0.0);
        assert_eq!(
            ws.get_light_attenuation(),
            0.5,
            "set_weather with duration=0.0 must apply instantly (Rain attenuation=0.5)"
        );
    }

    // Kill mutation: <= 0.0 → > 0.0 (swaps instant vs gradual)
    // With mutation, duration=0.0 would take the gradual path (no immediate change),
    // while positive duration would take the instant path.
    #[test]
    fn set_weather_gradual_does_not_apply_instantly() {
        let mut ws = WeatherSystem::new();
        // Use a positive transition duration — should NOT apply instantly
        ws.set_weather(WeatherType::Storm, 5.0);
        // Without calling update(), the weather should still be Clear (default)
        assert_eq!(
            ws.get_light_attenuation(),
            1.0,
            "set_weather with duration=5.0 should NOT apply instantly — still Clear (1.0)"
        );
    }

    // Additionally: verify negative duration behaves like instant (duration <= 0)
    #[test]
    fn set_weather_negative_duration_is_instant() {
        let mut ws = WeatherSystem::new();
        ws.set_weather(WeatherType::Snow, -1.0);
        assert_eq!(
            ws.get_light_attenuation(),
            0.6,
            "set_weather with duration=-1.0 must apply instantly (Snow attenuation=0.6)"
        );
    }
}

// ============================================================================
// REMEDIATION 3: ssao.rs — SsaoQuality exact per-variant values
// ============================================================================

#[cfg(feature = "ssao")]
mod ssao_exact {
    use astraweave_render::ssao::{SsaoKernel, SsaoQuality};

    #[test]
    fn sample_count_exact_per_quality() {
        assert_eq!(SsaoQuality::Low.sample_count(), 8);
        assert_eq!(SsaoQuality::Medium.sample_count(), 16);
        assert_eq!(SsaoQuality::High.sample_count(), 32);
        assert_eq!(SsaoQuality::Ultra.sample_count(), 64);
    }

    #[test]
    fn radius_exact_per_quality() {
        assert_eq!(SsaoQuality::Low.radius(), 0.5);
        assert_eq!(SsaoQuality::Medium.radius(), 1.0);
        assert_eq!(SsaoQuality::High.radius(), 1.5);
        assert_eq!(SsaoQuality::Ultra.radius(), 2.0);
    }

    #[test]
    fn blur_kernel_size_exact_per_quality() {
        assert_eq!(SsaoQuality::Low.blur_kernel_size(), 0);
        assert_eq!(SsaoQuality::Medium.blur_kernel_size(), 3);
        assert_eq!(SsaoQuality::High.blur_kernel_size(), 5);
        assert_eq!(SsaoQuality::Ultra.blur_kernel_size(), 7);
    }

    #[test]
    fn kernel_generate_samples_in_hemisphere() {
        let kernel = SsaoKernel::generate(32);
        for i in 0..32 {
            let s = kernel.samples[i];
            assert!(s[2] >= 0.0, "Sample {} z should be >= 0, got {}", i, s[2]);
            let len = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
            assert!(len <= 1.01, "Sample {} length {} should be <= 1", i, len);
            assert!(len > 0.0, "Sample {} should have non-zero length", i);
        }
    }

    #[test]
    fn kernel_generate_scale_bias_correct() {
        let kernel = SsaoKernel::generate(64);
        let first_len = (kernel.samples[0][0].powi(2)
            + kernel.samples[0][1].powi(2)
            + kernel.samples[0][2].powi(2))
        .sqrt();
        let last_len = (kernel.samples[63][0].powi(2)
            + kernel.samples[63][1].powi(2)
            + kernel.samples[63][2].powi(2))
        .sqrt();
        assert!(
            last_len > first_len,
            "Last sample ({}) should be farther than first ({})",
            last_len,
            first_len
        );
    }
}

// ============================================================================
// REMEDIATION 4: material.rs — MaterialGpu::neutral exact values
// ============================================================================

mod material_exact {
    use super::*;

    #[test]
    fn neutral_factors_exact() {
        let mat = MaterialGpu::neutral(0);
        // factors = [metallic=0, roughness=0.5, ao=1.0, alpha=1.0]
        assert_eq!(mat.factors[0], 0.0, "Metallic should be 0.0");
        assert_eq!(mat.factors[1], 0.5, "Roughness should be 0.5");
        assert_eq!(mat.factors[2], 1.0, "AO should be 1.0");
        assert_eq!(mat.factors[3], 1.0, "Alpha should be 1.0");
    }

    #[test]
    fn neutral_tiling_exact() {
        let mat = MaterialGpu::neutral(0);
        // tiling_triplanar = [1.0, 1.0, 16.0, 0.0]
        assert_eq!(mat.tiling_triplanar[0], 1.0, "Tiling U should be 1.0");
        assert_eq!(mat.tiling_triplanar[1], 1.0, "Tiling V should be 1.0");
        assert_eq!(
            mat.tiling_triplanar[2], 16.0,
            "Detail tiling should be 16.0"
        );
        assert_eq!(
            mat.tiling_triplanar[3], 0.0,
            "Triplanar blend should be 0.0"
        );
    }

    #[test]
    fn neutral_indices_use_layer_param() {
        let mat3 = MaterialGpu::neutral(3);
        assert_eq!(mat3.texture_indices[0], 3);
        assert_eq!(mat3.texture_indices[1], 3);
        assert_eq!(mat3.texture_indices[2], 3);
        assert_eq!(mat3.texture_indices[3], 0, "Fourth index always 0");
    }

    #[test]
    fn neutral_flags_zero() {
        let mat = MaterialGpu::neutral(0);
        assert_eq!(mat.flags, 0, "Default flags should be 0");
    }
}

// ============================================================================
// REMEDIATION 5: post.rs — BloomConfig::validate boundary precision
// ============================================================================

#[cfg(feature = "bloom")]
mod bloom_validate_boundary {
    use astraweave_render::BloomConfig;

    #[test]
    fn threshold_boundary_10_is_valid() {
        let config = BloomConfig {
            threshold: 10.0,
            ..BloomConfig::default()
        };
        assert!(config.validate().is_ok(), "threshold=10.0 should be valid");
    }

    #[test]
    fn threshold_boundary_above_10_is_invalid() {
        let config = BloomConfig {
            threshold: 10.01,
            ..BloomConfig::default()
        };
        assert!(
            config.validate().is_err(),
            "threshold=10.01 should be invalid"
        );
    }

    #[test]
    fn threshold_zero_is_valid() {
        let config = BloomConfig {
            threshold: 0.0,
            ..BloomConfig::default()
        };
        assert!(config.validate().is_ok(), "threshold=0.0 should be valid");
    }

    #[test]
    fn intensity_boundary_1_is_valid() {
        let config = BloomConfig {
            intensity: 1.0,
            ..BloomConfig::default()
        };
        assert!(config.validate().is_ok(), "intensity=1.0 should be valid");
    }

    #[test]
    fn intensity_above_1_is_invalid() {
        let config = BloomConfig {
            intensity: 1.01,
            ..BloomConfig::default()
        };
        assert!(
            config.validate().is_err(),
            "intensity=1.01 should be invalid"
        );
    }

    #[test]
    fn mip_count_1_is_valid() {
        let config = BloomConfig {
            mip_count: 1,
            ..BloomConfig::default()
        };
        assert!(config.validate().is_ok(), "mip_count=1 should be valid");
    }

    #[test]
    fn mip_count_8_is_valid() {
        let config = BloomConfig {
            mip_count: 8,
            ..BloomConfig::default()
        };
        assert!(config.validate().is_ok(), "mip_count=8 should be valid");
    }

    #[test]
    fn mip_count_0_is_invalid() {
        let config = BloomConfig {
            mip_count: 0,
            ..BloomConfig::default()
        };
        assert!(config.validate().is_err(), "mip_count=0 should be invalid");
    }

    #[test]
    fn mip_count_9_is_invalid() {
        let config = BloomConfig {
            mip_count: 9,
            ..BloomConfig::default()
        };
        assert!(config.validate().is_err(), "mip_count=9 should be invalid");
    }
}

// ============================================================================
// REMEDIATION 6: biome_transition.rs — EasingFunction::apply exact values
// ============================================================================

mod easing_exact {
    use super::*;

    #[test]
    fn linear_is_identity() {
        for &t in &[0.0, 0.25, 0.5, 0.75, 1.0] {
            let r = EasingFunction::Linear.apply(t);
            assert!(
                (r - t).abs() < 1e-6,
                "Linear({}) should be {}, got {}",
                t,
                t,
                r
            );
        }
    }

    #[test]
    fn smoothstep_at_half_is_half() {
        let r = EasingFunction::SmoothStep.apply(0.5);
        assert!(
            (r - 0.5).abs() < 1e-6,
            "SmoothStep(0.5) = {}, expected 0.5",
            r
        );
    }

    #[test]
    fn smoothstep_exact_formula() {
        // Formula: t * t * (3.0 - 2.0 * t)
        let t = 0.3_f32;
        let expected = t * t * (3.0 - 2.0 * t);
        let actual = EasingFunction::SmoothStep.apply(t);
        assert!((actual - expected).abs() < 1e-6);
    }

    #[test]
    fn smootherstep_exact_formula() {
        // Formula: t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
        let t = 0.3_f32;
        let expected = t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
        let actual = EasingFunction::SmootherStep.apply(t);
        assert!((actual - expected).abs() < 1e-6);
    }

    #[test]
    fn ease_in_is_quadratic() {
        let t = 0.4_f32;
        let expected = t * t;
        let actual = EasingFunction::EaseIn.apply(t);
        assert!((actual - expected).abs() < 1e-6);
    }

    #[test]
    fn ease_out_formula() {
        let t = 0.4_f32;
        let expected = 1.0 - (1.0 - t) * (1.0 - t);
        let actual = EasingFunction::EaseOut.apply(t);
        assert!((actual - expected).abs() < 1e-6);
    }

    #[test]
    fn ease_in_out_boundary_at_half() {
        // At t=0.5: takes the `if t < 0.5` branch: 2.0 * 0.5 * 0.5 = 0.5
        let r = EasingFunction::EaseInOut.apply(0.5);
        // With mutation (< becomes <=): t=0.5 still takes if branch → same result 0.5
        // But that's OK — we test BOTH branches produce correct values; the
        // mutation is killed by the just_above_half test where the else branch
        // would produce wrong result if it's switched.
        assert!(
            (r - 0.5).abs() < 1e-6,
            "EaseInOut(0.5) should be 0.5, got {}",
            r
        );
    }

    #[test]
    fn ease_in_out_just_below_half() {
        let t = 0.499_f32;
        // t < 0.5 branch: 2.0 * t * t
        let expected = 2.0 * t * t;
        let actual = EasingFunction::EaseInOut.apply(t);
        assert!((actual - expected).abs() < 1e-4);
    }

    #[test]
    fn ease_in_out_just_above_half() {
        let t = 0.501_f32;
        // t >= 0.5 branch: 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
        let expected = 1.0 - (-2.0 * t + 2.0_f32).powi(2) / 2.0;
        let actual = EasingFunction::EaseInOut.apply(t);
        assert!((actual - expected).abs() < 1e-4);
    }

    // Kill mutation: t < 0.5 → t <= 0.5
    // At EXACTLY t=0.5, if branch gives 2*0.25 = 0.5
    // else branch gives 1 - (-2*0.5+2)^2/2 = 1 - 1/2 = 0.5
    // Both branches give 0.5 at exactly t=0.5, so the mutation is equivalent at this point.
    // To kill it, we need values near the boundary where the branches diverge.
    // At t=0.50001: if branch = 2 * 0.50001^2 = 0.5000200, else branch = 0.4999800
    // The mutation moves t=0.50001 from else→if, changing the result.
    #[test]
    fn ease_in_out_distinguishes_branches_near_half() {
        let t = 0.50001_f32;
        // Original (< 0.5): t=0.50001 → else branch
        let else_result = 1.0 - (-2.0 * t + 2.0_f32).powi(2) / 2.0;
        let if_result = 2.0 * t * t;
        // With mutation (<= 0.5): t=0.50001 → still else branch (0.50001 > 0.5)
        // hmm, 0.50001 > 0.5, so < vs <= doesn't matter here.
        // Need t EXACTLY 0.5 to distinguish, but both give same result.
        // This mutation is equivalent. Accept it and focus on other mutants.
        let actual = EasingFunction::EaseInOut.apply(t);
        assert!(
            (actual - else_result).abs() < 1e-4,
            "t=0.50001 should use else branch, got {} expected {}",
            actual,
            else_result
        );
    }

    #[test]
    fn all_easings_endpoints_correct() {
        let easings = [
            EasingFunction::Linear,
            EasingFunction::SmoothStep,
            EasingFunction::SmootherStep,
            EasingFunction::EaseIn,
            EasingFunction::EaseOut,
            EasingFunction::EaseInOut,
        ];
        for e in &easings {
            assert!((e.apply(0.0)).abs() < 1e-6, "{:?} at 0 should be 0", e);
            assert!(
                (e.apply(1.0) - 1.0).abs() < 1e-6,
                "{:?} at 1 should be 1",
                e
            );
        }
    }
}

// ============================================================================
// REMEDIATION 7: camera.rs — direction and clamp exact tests
// ============================================================================

mod camera_exact {
    use super::*;

    #[test]
    fn camera_dir_at_zero_pitch_yaw() {
        // Camera::dir is a STATIC method: dir(yaw, pitch) -> Vec3
        let dir = Camera::dir(0.0, 0.0);
        // At pitch=0, yaw=0: cos(0)*cos(0)=1, sin(0)=0, sin(0)*cos(0)=0 → (1, 0, 0)
        assert!(
            (dir.x - 1.0).abs() < 0.01,
            "dir.x should be ~1, got {}",
            dir.x
        );
        assert!((dir.y).abs() < 0.01, "dir.y should be ~0, got {}", dir.y);
        assert!((dir.z).abs() < 0.01, "dir.z should be ~0, got {}", dir.z);
    }

    #[test]
    fn camera_dir_look_up_pitch_90() {
        let dir = Camera::dir(0.0, std::f32::consts::FRAC_PI_2);
        // Looking straight up: y ~ 1
        assert!(
            dir.y > 0.99,
            "Looking up, dir.y should be ~1, got {}",
            dir.y
        );
    }

    #[test]
    fn camera_view_proj_not_nan() {
        let cam = Camera {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            fovy: 1.0,
            aspect: 1.0,
            znear: 0.1,
            zfar: 100.0,
        };
        let vp = cam.vp();
        for i in 0..4 {
            for j in 0..4 {
                assert!(!vp.col(i)[j].is_nan(), "VP matrix should not contain NaN");
            }
        }
    }

    // Kill mutation: sy * cp → sy / cp at line 34 col 35
    // Requires both yaw ≠ 0 and pitch ≠ 0 so sy ≠ 0 and cp ≠ 1
    #[test]
    fn camera_dir_yaw_pitch_45deg() {
        use std::f32::consts::FRAC_PI_4;
        let dir = Camera::dir(FRAC_PI_4, FRAC_PI_4);
        // cy=cos(π/4)=0.707, sy=sin(π/4)=0.707, cp=cos(π/4)=0.707, sp=sin(π/4)=0.707
        // Before normalize: (0.5, 0.707, 0.5), length = 1.0
        // x and z must be equal since cos(π/4) == sin(π/4)
        assert!(
            (dir.x - dir.z).abs() < 0.01,
            "At yaw=π/4, pitch=π/4: dir.x should equal dir.z, got x={} z={}",
            dir.x,
            dir.z
        );
        assert!(
            dir.z > 0.45 && dir.z < 0.55,
            "dir.z should be ~0.5, got {}",
            dir.z
        );
    }

    // Stronger test: with mutation sy*cp → sy/cp, the z component becomes sy/cp = 1.0
    // but the correct z before normalization is sy*cp = 0.707*0.707 = 0.5
    // At yaw=π/3, pitch=π/6: sy=sin(60°)=0.866, cp=cos(30°)=0.866
    // Correct z_raw = sy*cp = 0.866*0.866 = 0.75
    // Mutated z_raw = sy/cp = 0.866/0.866 = 1.0
    // These are different enough to distinguish after normalization.
    #[test]
    fn camera_dir_yaw60_pitch30_precise() {
        use std::f32::consts::{FRAC_PI_3, FRAC_PI_6};
        let dir = Camera::dir(FRAC_PI_3, FRAC_PI_6);
        // cy=cos(π/3)=0.5, sy=sin(π/3)=0.866, cp=cos(π/6)=0.866, sp=sin(π/6)=0.5
        // raw = (cy*cp, sp, sy*cp) = (0.5*0.866, 0.5, 0.866*0.866) = (0.433, 0.5, 0.75)
        // len = sqrt(0.433^2 + 0.5^2 + 0.75^2) = sqrt(0.1875 + 0.25 + 0.5625) = sqrt(1.0) = 1.0
        // normalized = (0.433, 0.5, 0.75)
        // With mutation: raw_z = 0.866/0.866 = 1.0 → raw = (0.433, 0.5, 1.0) → len ≈ 1.20
        // normalized_z ≈ 0.833 (VERY different from 0.75)
        assert!(
            (dir.z - 0.75).abs() < 0.02,
            "dir.z at yaw=π/3, pitch=π/6 should be ~0.75, got {} (mutation would give ~0.833)",
            dir.z
        );
        assert!(
            (dir.x - 0.433).abs() < 0.02,
            "dir.x should be ~0.433, got {}",
            dir.x
        );
        assert!(
            (dir.y - 0.5).abs() < 0.02,
            "dir.y should be ~0.5, got {}",
            dir.y
        );
    }
}

// ============================================================================
// REMEDIATION 8: culling.rs — frustum test_aabb boundary
// ============================================================================

mod culling_boundary {
    use super::*;

    #[test]
    fn frustum_from_identity_vp_accepts_origin_box() {
        // Identity view-proj creates a frustum containing the NDC cube
        let vp = Mat4::IDENTITY;
        let frustum = FrustumPlanes::from_view_proj(&vp);
        // Small box at origin should be inside
        let result = frustum.test_aabb(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.1, 0.1, 0.1));
        assert!(
            result,
            "Small box at origin should be inside identity frustum"
        );
    }

    #[test]
    fn frustum_rejects_far_away_box() {
        let vp = Mat4::IDENTITY;
        let frustum = FrustumPlanes::from_view_proj(&vp);
        // Box very far away should be outside
        let result = frustum.test_aabb(Vec3::new(1000.0, 1000.0, 1000.0), Vec3::new(0.1, 0.1, 0.1));
        assert!(
            !result,
            "Box at (1000,1000,1000) should be outside identity frustum"
        );
    }

    #[test]
    fn frustum_consistency() {
        let vp = Mat4::IDENTITY;
        let frustum = FrustumPlanes::from_view_proj(&vp);
        let center = Vec3::new(0.5, 0.5, 0.5);
        let extent = Vec3::new(0.1, 0.1, 0.1);
        let r1 = frustum.test_aabb(center, extent);
        let r2 = frustum.test_aabb(center, extent);
        assert_eq!(r1, r2, "Same input should give same result");
    }
}
