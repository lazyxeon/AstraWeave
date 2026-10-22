//! Wave 2 – Golden-value tests for biome_transition.rs (130 mutants)
//!
//! Targets: EasingFunction::apply (48 mutants), TransitionEffect state machine,
//!          BiomeVisuals::for_biome field pinning, tint_alpha bell curve,
//!          lerp helper, TransitionConfig defaults.
//!
//! Strategy: Pin EXACT easing outputs at t=0, 0.25, 0.5, 0.75, 1.0 for all 6
//! functions. Pin BiomeVisuals::for_biome key values per biome type. Test
//! TransitionEffect state transitions with golden blend factors.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::biome_transition::{
    BiomeVisuals, EasingFunction, TransitionConfig, TransitionEffect,
};
use astraweave_terrain::biome::BiomeType;
use glam::Vec3;

// ============================================================================
// EasingFunction::apply — golden values at key points
// ============================================================================

// Linear(t) = t
#[test]
fn easing_linear_golden() {
    let e = EasingFunction::Linear;
    assert_eq!(e.apply(0.0), 0.0);
    assert_eq!(e.apply(0.25), 0.25);
    assert_eq!(e.apply(0.5), 0.5);
    assert_eq!(e.apply(0.75), 0.75);
    assert_eq!(e.apply(1.0), 1.0);
}

// SmoothStep(t) = t² * (3 - 2t)
// SS(0.25) = 0.0625 * 2.5 = 0.15625
// SS(0.5) = 0.25 * 2.0 = 0.5
// SS(0.75) = 0.5625 * 1.5 = 0.84375
#[test]
fn easing_smoothstep_golden() {
    let e = EasingFunction::SmoothStep;
    assert!((e.apply(0.0)).abs() < 1e-6);
    assert!(
        (e.apply(0.25) - 0.15625).abs() < 1e-5,
        "SS(0.25) = {}",
        e.apply(0.25)
    );
    assert!((e.apply(0.5) - 0.5).abs() < 1e-6);
    assert!(
        (e.apply(0.75) - 0.84375).abs() < 1e-5,
        "SS(0.75) = {}",
        e.apply(0.75)
    );
    assert!((e.apply(1.0) - 1.0).abs() < 1e-6);
}

// SmootherStep(t) = t³ * (t*(6t-15) + 10)
// SMS(0.25) = 0.015625 * (0.25*(1.5-15)+10) = 0.015625 * (0.25*-13.5+10) = 0.015625 * 6.625 = 0.103516
// SMS(0.5) = 0.125 * (0.5*(3-15)+10) = 0.125 * 4 = 0.5
// SMS(0.75) = 0.421875 * (0.75*(4.5-15)+10) = 0.421875 * (0.75*-10.5+10) = 0.421875 * 2.125 = 0.896484
#[test]
fn easing_smoother_step_golden() {
    let e = EasingFunction::SmootherStep;
    assert!((e.apply(0.0)).abs() < 1e-6);
    assert!(
        (e.apply(0.25) - 0.103516).abs() < 0.001,
        "SMS(0.25) = {}",
        e.apply(0.25)
    );
    assert!((e.apply(0.5) - 0.5).abs() < 1e-5);
    assert!(
        (e.apply(0.75) - 0.896484).abs() < 0.001,
        "SMS(0.75) = {}",
        e.apply(0.75)
    );
    assert!((e.apply(1.0) - 1.0).abs() < 1e-6);
}

// EaseIn(t) = t²
#[test]
fn easing_ease_in_golden() {
    let e = EasingFunction::EaseIn;
    assert_eq!(e.apply(0.0), 0.0);
    assert!((e.apply(0.25) - 0.0625).abs() < 1e-6);
    assert!((e.apply(0.5) - 0.25).abs() < 1e-6);
    assert!((e.apply(0.75) - 0.5625).abs() < 1e-6);
    assert!((e.apply(1.0) - 1.0).abs() < 1e-6);
}

// EaseOut(t) = 1 - (1-t)²
// EO(0.25) = 1 - 0.5625 = 0.4375
// EO(0.5) = 1 - 0.25 = 0.75
// EO(0.75) = 1 - 0.0625 = 0.9375
#[test]
fn easing_ease_out_golden() {
    let e = EasingFunction::EaseOut;
    assert!((e.apply(0.0)).abs() < 1e-6);
    assert!(
        (e.apply(0.25) - 0.4375).abs() < 1e-5,
        "EO(0.25) = {}",
        e.apply(0.25)
    );
    assert!((e.apply(0.5) - 0.75).abs() < 1e-5);
    assert!((e.apply(0.75) - 0.9375).abs() < 1e-5);
    assert!((e.apply(1.0) - 1.0).abs() < 1e-6);
}

// EaseInOut: t<0.5 → 2t², else → 1-(-2t+2)²/2
// EIO(0.25) = 2 * 0.0625 = 0.125
// EIO(0.5) = 1 - (-1+2)²/2 = 1 - 0.5 = 0.5 (falls in else branch)
// EIO(0.75) = 1 - (-1.5+2)²/2 = 1 - 0.125 = 0.875
#[test]
fn easing_ease_in_out_golden() {
    let e = EasingFunction::EaseInOut;
    assert!((e.apply(0.0)).abs() < 1e-6);
    assert!(
        (e.apply(0.25) - 0.125).abs() < 1e-5,
        "EIO(0.25) = {}",
        e.apply(0.25)
    );
    assert!((e.apply(0.5) - 0.5).abs() < 1e-5);
    assert!(
        (e.apply(0.75) - 0.875).abs() < 1e-5,
        "EIO(0.75) = {}",
        e.apply(0.75)
    );
    assert!((e.apply(1.0) - 1.0).abs() < 1e-6);
}

// ============================================================================
// EasingFunction::apply — clamping & symmetry
// ============================================================================

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
        assert_eq!(e.apply(-1.0), e.apply(0.0), "{:?} should clamp t<0", e);
        assert_eq!(e.apply(-100.0), e.apply(0.0), "{:?} far negative", e);
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
        assert_eq!(e.apply(2.0), e.apply(1.0), "{:?} should clamp t>1", e);
        assert_eq!(e.apply(100.0), e.apply(1.0), "{:?} far positive", e);
    }
}

#[test]
fn easing_all_start_at_zero_end_at_one() {
    for e in [
        EasingFunction::Linear,
        EasingFunction::SmoothStep,
        EasingFunction::SmootherStep,
        EasingFunction::EaseIn,
        EasingFunction::EaseOut,
        EasingFunction::EaseInOut,
    ] {
        assert!((e.apply(0.0)).abs() < 1e-6, "{:?} f(0) != 0", e);
        assert!((e.apply(1.0) - 1.0).abs() < 1e-6, "{:?} f(1) != 1", e);
    }
}

#[test]
fn easing_monotonically_increasing() {
    for e in [
        EasingFunction::Linear,
        EasingFunction::SmoothStep,
        EasingFunction::SmootherStep,
        EasingFunction::EaseIn,
        EasingFunction::EaseOut,
        EasingFunction::EaseInOut,
    ] {
        let mut prev = e.apply(0.0);
        for i in 1..=100 {
            let t = i as f32 / 100.0;
            let val = e.apply(t);
            assert!(
                val >= prev - 1e-6,
                "{:?} not monotonic at t={}: {} < {}",
                e,
                t,
                val,
                prev
            );
            prev = val;
        }
    }
}

#[test]
fn easing_smooth_and_smoother_equal_at_midpoint() {
    let ss = EasingFunction::SmoothStep.apply(0.5);
    let sms = EasingFunction::SmootherStep.apply(0.5);
    assert!((ss - 0.5).abs() < 1e-6, "SmoothStep(0.5) should be 0.5");
    assert!((sms - 0.5).abs() < 1e-6, "SmootherStep(0.5) should be 0.5");
}

#[test]
fn ease_in_out_symmetric_around_half() {
    let e = EasingFunction::EaseInOut;
    // f(0.25) + f(0.75) should equal 1.0 for symmetric ease
    let low = e.apply(0.25);
    let high = e.apply(0.75);
    assert!(
        (low + high - 1.0).abs() < 1e-5,
        "EaseInOut should be symmetric: f(0.25)={} + f(0.75)={} = {}",
        low,
        high,
        low + high
    );
}

#[test]
fn ease_in_slower_start_than_linear() {
    let ei = EasingFunction::EaseIn.apply(0.25);
    let lin = EasingFunction::Linear.apply(0.25);
    assert!(
        ei < lin,
        "EaseIn should be slower at start: {} >= {}",
        ei,
        lin
    );
}

#[test]
fn ease_out_faster_start_than_linear() {
    let eo = EasingFunction::EaseOut.apply(0.25);
    let lin = EasingFunction::Linear.apply(0.25);
    assert!(
        eo > lin,
        "EaseOut should be faster at start: {} <= {}",
        eo,
        lin
    );
}

// ============================================================================
// BiomeVisuals::for_biome — pin unique values per biome
// ============================================================================

#[test]
fn forest_fog_denser_than_desert() {
    let f = BiomeVisuals::for_biome(BiomeType::Forest);
    let d = BiomeVisuals::for_biome(BiomeType::Desert);
    assert!(
        f.fog_density > d.fog_density,
        "Forest fog ({}) should be denser than desert ({})",
        f.fog_density,
        d.fog_density
    );
}

#[test]
fn desert_fog_start_farther_than_forest() {
    let f = BiomeVisuals::for_biome(BiomeType::Forest);
    let d = BiomeVisuals::for_biome(BiomeType::Desert);
    assert!(
        d.fog_start > f.fog_start,
        "Desert fog_start ({}) > forest ({})",
        d.fog_start,
        f.fog_start
    );
}

#[test]
fn swamp_has_densest_fog() {
    let biomes = [
        BiomeType::Forest,
        BiomeType::Desert,
        BiomeType::Grassland,
        BiomeType::Mountain,
        BiomeType::Tundra,
        BiomeType::Beach,
        BiomeType::River,
    ];
    let swamp = BiomeVisuals::for_biome(BiomeType::Swamp);
    for b in biomes {
        let v = BiomeVisuals::for_biome(b);
        assert!(
            swamp.fog_density >= v.fog_density,
            "Swamp fog ({}) should be >= {:?} ({})",
            swamp.fog_density,
            b,
            v.fog_density
        );
    }
}

#[test]
fn all_biomes_have_positive_fog_density() {
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
    for b in biomes {
        let v = BiomeVisuals::for_biome(b);
        assert!(
            v.fog_density > 0.0,
            "{:?} fog_density should be positive",
            b
        );
        assert!(v.fog_start > 0.0, "{:?} fog_start should be positive", b);
        assert!(
            v.fog_end > v.fog_start,
            "{:?} fog_end should exceed fog_start",
            b
        );
        assert!(
            v.ambient_intensity > 0.0,
            "{:?} ambient_intensity should be positive",
            b
        );
    }
}

#[test]
fn forest_golden_fog_color() {
    let f = BiomeVisuals::for_biome(BiomeType::Forest);
    assert_eq!(f.fog_color, Vec3::new(0.4, 0.5, 0.35));
    assert_eq!(f.fog_density, 0.003);
    assert_eq!(f.fog_start, 30.0);
    assert_eq!(f.fog_end, 300.0);
}

#[test]
fn desert_golden_ambient() {
    let d = BiomeVisuals::for_biome(BiomeType::Desert);
    assert_eq!(d.ambient_color, Vec3::new(0.6, 0.55, 0.4));
    assert_eq!(d.ambient_intensity, 0.4);
}

#[test]
fn mountain_golden_cloud_coverage() {
    let m = BiomeVisuals::for_biome(BiomeType::Mountain);
    assert_eq!(m.cloud_coverage, 0.6);
    assert_eq!(m.cloud_speed, 0.04);
}

#[test]
fn beach_weather_density_default() {
    let b = BiomeVisuals::for_biome(BiomeType::Beach);
    assert_eq!(b.weather_particle_density, 1.0);
    assert_eq!(b.cloud_coverage, 0.25); // Usually sunny
}

// ============================================================================
// TransitionConfig defaults
// ============================================================================

#[test]
fn transition_config_defaults_golden() {
    let c = TransitionConfig::default();
    assert_eq!(c.duration, 2.0);
    assert_eq!(c.easing, EasingFunction::SmoothStep);
    assert!(c.blend_fog);
    assert!(c.blend_ambient);
    assert!(!c.apply_tint);
    assert_eq!(c.tint_alpha, 0.15);
}

// ============================================================================
// TransitionEffect — state machine
// ============================================================================

#[test]
fn transition_starts_inactive() {
    let te = TransitionEffect::new(TransitionConfig::default());
    assert!(!te.is_active());
    assert_eq!(te.raw_progress(), 0.0);
}

#[test]
fn transition_start_activates() {
    let mut te = TransitionEffect::new(TransitionConfig::default());
    te.start(Some(BiomeType::Forest), BiomeType::Desert);
    assert!(te.is_active());
    assert_eq!(te.raw_progress(), 0.0);
    assert_eq!(te.from_biome(), Some(BiomeType::Forest));
    assert_eq!(te.to_biome(), Some(BiomeType::Desert));
}

#[test]
fn transition_same_biome_noop() {
    let mut te = TransitionEffect::new(TransitionConfig::default());
    te.start(Some(BiomeType::Forest), BiomeType::Forest);
    assert!(!te.is_active(), "Same from/to should not activate");
}

#[test]
fn transition_none_from_uses_to() {
    let mut te = TransitionEffect::new(TransitionConfig::default());
    te.start(None, BiomeType::Desert);
    // from=None → defaults to Desert, but Desert == Desert would be noop?
    // Actually: from.unwrap_or(to) → Desert. from_biome == to && from.is_some() → false!
    // So it activates.
    assert!(
        te.is_active(),
        "None from should still activate (from.is_some() is false)"
    );
}

#[test]
fn transition_update_advances_progress() {
    let mut te = TransitionEffect::new(TransitionConfig {
        duration: 2.0,
        ..TransitionConfig::default()
    });
    te.start(Some(BiomeType::Forest), BiomeType::Desert);
    te.update(0.5); // 0.5s / 2.0s = 25% progress
    assert!(
        (te.raw_progress() - 0.25).abs() < 0.001,
        "Progress: {}",
        te.raw_progress()
    );
    assert!(te.is_active());
}

#[test]
fn transition_completes_at_duration() {
    let mut te = TransitionEffect::new(TransitionConfig {
        duration: 1.0,
        ..TransitionConfig::default()
    });
    te.start(Some(BiomeType::Forest), BiomeType::Desert);
    te.update(1.0); // Full duration
    assert!(!te.is_active(), "Should be complete");
    assert_eq!(te.raw_progress(), 1.0);
}

#[test]
fn transition_overshoots_clamped_to_one() {
    let mut te = TransitionEffect::new(TransitionConfig {
        duration: 1.0,
        ..TransitionConfig::default()
    });
    te.start(Some(BiomeType::Forest), BiomeType::Desert);
    te.update(5.0); // Way past duration
    assert_eq!(te.raw_progress(), 1.0);
    assert!(!te.is_active());
}

#[test]
fn transition_blend_factor_uses_easing() {
    let mut te = TransitionEffect::new(TransitionConfig {
        duration: 2.0,
        easing: EasingFunction::EaseIn,
        ..TransitionConfig::default()
    });
    te.start(Some(BiomeType::Forest), BiomeType::Desert);
    te.update(1.0); // 50% raw progress
    assert!((te.raw_progress() - 0.5).abs() < 0.001);
    // blend_factor should be EaseIn(0.5) = 0.25
    assert!(
        (te.blend_factor() - 0.25).abs() < 0.001,
        "Blend factor should be EaseIn(0.5) = 0.25, got {}",
        te.blend_factor()
    );
}

// ============================================================================
// TransitionEffect::tint_alpha — bell curve
// ============================================================================

#[test]
fn tint_alpha_zero_when_not_active() {
    let te = TransitionEffect::new(TransitionConfig {
        apply_tint: true,
        ..TransitionConfig::default()
    });
    assert_eq!(te.tint_alpha(), 0.0);
}

#[test]
fn tint_alpha_zero_when_tint_disabled() {
    let mut te = TransitionEffect::new(TransitionConfig {
        apply_tint: false,
        ..TransitionConfig::default()
    });
    te.start(Some(BiomeType::Forest), BiomeType::Desert);
    te.update(0.5);
    assert_eq!(te.tint_alpha(), 0.0, "Tint disabled → always 0");
}

#[test]
fn tint_alpha_peaks_at_midpoint() {
    let mut te = TransitionEffect::new(TransitionConfig {
        duration: 2.0,
        apply_tint: true,
        tint_alpha: 0.15,
        ..TransitionConfig::default()
    });
    te.start(Some(BiomeType::Forest), BiomeType::Desert);
    te.update(1.0); // t = 0.5 (midpoint)
                    // Formula: peak * 4 * t * (1-t) = 0.15 * 4 * 0.5 * 0.5 = 0.15
    assert!(
        (te.tint_alpha() - 0.15).abs() < 0.001,
        "Peak tint_alpha at midpoint should be {}, got {}",
        0.15,
        te.tint_alpha()
    );
}

#[test]
fn tint_alpha_bell_curve_quarter() {
    let mut te = TransitionEffect::new(TransitionConfig {
        duration: 4.0,
        apply_tint: true,
        tint_alpha: 0.2,
        ..TransitionConfig::default()
    });
    te.start(Some(BiomeType::Forest), BiomeType::Desert);
    te.update(1.0); // t = 0.25
                    // Formula: 0.2 * 4 * 0.25 * 0.75 = 0.2 * 0.75 = 0.15
    let expected = 0.2 * 4.0 * 0.25 * 0.75;
    assert!(
        (te.tint_alpha() - expected).abs() < 0.001,
        "tint_alpha at t=0.25: expected {}, got {}",
        expected,
        te.tint_alpha()
    );
}

// ============================================================================
// TransitionEffect::tint_color
// ============================================================================

#[test]
fn tint_color_is_average_of_ambient_colors() {
    let mut te = TransitionEffect::new(TransitionConfig::default());
    te.start(Some(BiomeType::Forest), BiomeType::Desert);
    let color = te.tint_color();
    let forest = BiomeVisuals::for_biome(BiomeType::Forest);
    let desert = BiomeVisuals::for_biome(BiomeType::Desert);
    let expected = (forest.ambient_color + desert.ambient_color) * 0.5;
    assert!(
        (color - expected).length() < 0.001,
        "Tint color should be avg of ambient: {:?} vs {:?}",
        color,
        expected
    );
}

// ============================================================================
// TransitionEffect::complete / cancel
// ============================================================================

#[test]
fn complete_finishes_instantly() {
    let mut te = TransitionEffect::new(TransitionConfig::default());
    te.start(Some(BiomeType::Forest), BiomeType::Desert);
    te.update(0.1); // Partial
    te.complete();
    assert!(!te.is_active());
    assert_eq!(te.raw_progress(), 1.0);
    // from_biome should now be to_biome
    assert_eq!(te.from_biome(), Some(BiomeType::Desert));
}

#[test]
fn cancel_snaps_back() {
    let mut te = TransitionEffect::new(TransitionConfig::default());
    te.start(Some(BiomeType::Forest), BiomeType::Desert);
    te.update(0.5);
    te.cancel();
    assert!(!te.is_active());
    assert_eq!(te.raw_progress(), 0.0);
    // to_biome should snap back to from_biome
    assert_eq!(te.to_biome(), Some(BiomeType::Forest));
}

// ============================================================================
// BiomeVisuals::lerp
// ============================================================================

#[test]
fn lerp_at_zero_returns_from() {
    let a = BiomeVisuals::for_biome(BiomeType::Forest);
    let b = BiomeVisuals::for_biome(BiomeType::Desert);
    let result = a.lerp(&b, 0.0);
    assert_eq!(result.fog_density, a.fog_density);
    assert_eq!(result.fog_start, a.fog_start);
    assert_eq!(result.ambient_intensity, a.ambient_intensity);
}

#[test]
fn lerp_at_one_returns_to() {
    let a = BiomeVisuals::for_biome(BiomeType::Forest);
    let b = BiomeVisuals::for_biome(BiomeType::Desert);
    let result = a.lerp(&b, 1.0);
    assert!((result.fog_density - b.fog_density).abs() < 1e-6);
    assert!((result.fog_start - b.fog_start).abs() < 1e-4);
    assert!((result.ambient_intensity - b.ambient_intensity).abs() < 1e-6);
}

#[test]
fn lerp_at_half_is_midpoint() {
    let a = BiomeVisuals::for_biome(BiomeType::Forest);
    let b = BiomeVisuals::for_biome(BiomeType::Desert);
    let result = a.lerp(&b, 0.5);
    let expected_density = (a.fog_density + b.fog_density) / 2.0;
    assert!(
        (result.fog_density - expected_density).abs() < 1e-6,
        "Midpoint fog_density: {} vs {}",
        result.fog_density,
        expected_density
    );
}
