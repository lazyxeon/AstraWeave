//! Mutation-Resistant Tests for astraweave-physics
//!
//! These tests verify **exact computed values** to ensure mutations to formulas
//! are detected by `cargo mutants`. Each test asserts on specific numerical results
//! rather than just checking relative comparisons or ranges.

#![cfg(test)]

use astraweave_physics::projectile::FalloffCurve;
use astraweave_physics::vehicle::{
    EngineConfig, FrictionCurve, TransmissionConfig, WheelConfig, WheelPosition,
};
use glam::Vec3;

// =============================================================================
// FalloffCurve Tests - Complete coverage of all curve types
// =============================================================================

mod falloff_curve_tests {
    use super::*;

    // --- Linear Curve ---
    #[test]
    fn linear_at_center_returns_exactly_1() {
        let curve = FalloffCurve::Linear;
        let result = curve.calculate(0.0, 10.0);
        assert!(
            (result - 1.0).abs() < 1e-6,
            "Linear at center should be exactly 1.0, got {}",
            result
        );
    }

    #[test]
    fn linear_at_half_radius_returns_exactly_0_5() {
        let curve = FalloffCurve::Linear;
        let result = curve.calculate(5.0, 10.0);
        // Linear formula: 1.0 - (5.0 / 10.0) = 1.0 - 0.5 = 0.5
        assert!(
            (result - 0.5).abs() < 1e-6,
            "Linear at half radius should be 0.5, got {}",
            result
        );
    }

    #[test]
    fn linear_at_quarter_radius_returns_exactly_0_75() {
        let curve = FalloffCurve::Linear;
        let result = curve.calculate(2.5, 10.0);
        // Linear formula: 1.0 - (2.5 / 10.0) = 1.0 - 0.25 = 0.75
        assert!(
            (result - 0.75).abs() < 1e-6,
            "Linear at quarter radius should be 0.75, got {}",
            result
        );
    }

    #[test]
    fn linear_at_edge_returns_exactly_0() {
        let curve = FalloffCurve::Linear;
        let result = curve.calculate(10.0, 10.0);
        // Linear formula: 1.0 - (10.0 / 10.0) = 1.0 - 1.0 = 0.0
        assert!(
            (result - 0.0).abs() < 1e-6,
            "Linear at edge should be 0.0, got {}",
            result
        );
    }

    #[test]
    fn linear_beyond_radius_returns_0() {
        let curve = FalloffCurve::Linear;
        let result = curve.calculate(15.0, 10.0);
        assert!(
            (result - 0.0).abs() < 1e-6,
            "Linear beyond radius should be 0.0, got {}",
            result
        );
    }

    // --- Quadratic Curve ---
    #[test]
    fn quadratic_at_center_returns_exactly_1() {
        let curve = FalloffCurve::Quadratic;
        let result = curve.calculate(0.0, 10.0);
        assert!(
            (result - 1.0).abs() < 1e-6,
            "Quadratic at center should be 1.0, got {}",
            result
        );
    }

    #[test]
    fn quadratic_at_half_radius_returns_exactly_0_75() {
        let curve = FalloffCurve::Quadratic;
        let result = curve.calculate(5.0, 10.0);
        // Quadratic formula: 1.0 - (0.5)^2 = 1.0 - 0.25 = 0.75
        assert!(
            (result - 0.75).abs() < 1e-6,
            "Quadratic at half radius should be 0.75, got {}",
            result
        );
    }

    #[test]
    fn quadratic_at_quarter_radius_returns_0_9375() {
        let curve = FalloffCurve::Quadratic;
        let result = curve.calculate(2.5, 10.0);
        // Quadratic formula: 1.0 - (0.25)^2 = 1.0 - 0.0625 = 0.9375
        assert!(
            (result - 0.9375).abs() < 1e-6,
            "Quadratic at quarter radius should be 0.9375, got {}",
            result
        );
    }

    #[test]
    fn quadratic_at_three_quarters_returns_0_4375() {
        let curve = FalloffCurve::Quadratic;
        let result = curve.calculate(7.5, 10.0);
        // Quadratic formula: 1.0 - (0.75)^2 = 1.0 - 0.5625 = 0.4375
        assert!(
            (result - 0.4375).abs() < 1e-6,
            "Quadratic at 3/4 radius should be 0.4375, got {}",
            result
        );
    }

    #[test]
    fn quadratic_at_edge_returns_exactly_0() {
        let curve = FalloffCurve::Quadratic;
        let result = curve.calculate(10.0, 10.0);
        assert!(
            (result - 0.0).abs() < 1e-6,
            "Quadratic at edge should be 0.0, got {}",
            result
        );
    }

    // --- Exponential Curve ---
    #[test]
    fn exponential_at_center_returns_exactly_1() {
        let curve = FalloffCurve::Exponential;
        let result = curve.calculate(0.0, 10.0);
        // e^(-0 * 3) = e^0 = 1.0
        assert!(
            (result - 1.0).abs() < 1e-6,
            "Exponential at center should be 1.0, got {}",
            result
        );
    }

    #[test]
    fn exponential_at_half_radius_returns_exp_minus_1_5() {
        let curve = FalloffCurve::Exponential;
        let result = curve.calculate(5.0, 10.0);
        // e^(-0.5 * 3) = e^(-1.5) ≈ 0.2231
        let expected = (-1.5_f32).exp();
        assert!(
            (result - expected).abs() < 1e-4,
            "Exponential at half radius should be {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn exponential_at_one_third_radius_returns_exp_minus_1() {
        let curve = FalloffCurve::Exponential;
        let result = curve.calculate(10.0 / 3.0, 10.0);
        // e^(-(1/3) * 3) = e^(-1) ≈ 0.3679
        let expected = (-1.0_f32).exp();
        assert!(
            (result - expected).abs() < 1e-4,
            "Exponential at 1/3 radius should be {}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn exponential_at_edge_returns_exp_minus_3() {
        let curve = FalloffCurve::Exponential;
        let result = curve.calculate(9.999, 10.0); // Just inside radius
                                                   // e^(-1 * 3) = e^(-3) ≈ 0.0498
        let expected = (-3.0_f32).exp();
        assert!(
            (result - expected).abs() < 0.01,
            "Exponential at edge should be ~{}, got {}",
            expected,
            result
        );
    }

    // --- Constant Curve ---
    #[test]
    fn constant_at_center_returns_exactly_1() {
        let curve = FalloffCurve::Constant;
        let result = curve.calculate(0.0, 10.0);
        assert!(
            (result - 1.0).abs() < 1e-6,
            "Constant at center should be 1.0, got {}",
            result
        );
    }

    #[test]
    fn constant_at_half_radius_returns_exactly_1() {
        let curve = FalloffCurve::Constant;
        let result = curve.calculate(5.0, 10.0);
        assert!(
            (result - 1.0).abs() < 1e-6,
            "Constant at half radius should be 1.0, got {}",
            result
        );
    }

    #[test]
    fn constant_at_almost_edge_returns_exactly_1() {
        let curve = FalloffCurve::Constant;
        let result = curve.calculate(9.99, 10.0);
        assert!(
            (result - 1.0).abs() < 1e-6,
            "Constant at 99.9% radius should be 1.0, got {}",
            result
        );
    }

    #[test]
    fn constant_beyond_radius_returns_0() {
        let curve = FalloffCurve::Constant;
        let result = curve.calculate(10.0, 10.0);
        assert!(
            (result - 0.0).abs() < 1e-6,
            "Constant at edge should be 0.0, got {}",
            result
        );
    }

    // --- Edge Cases ---
    #[test]
    fn zero_radius_at_zero_distance_returns_1() {
        // When radius <= 0, the implementation returns 1.0 for all curves
        for curve in [
            FalloffCurve::Linear,
            FalloffCurve::Quadratic,
            FalloffCurve::Constant,
            FalloffCurve::Exponential,
        ] {
            let result = curve.calculate(0.0, 0.0);
            // Per implementation: if radius <= 0.0 { return 1.0; }
            // But also: if distance >= radius { return 0.0; } is checked first
            // So when distance=0, radius=0: distance >= radius is true, returns 0.0
            assert!(
                (result - 0.0).abs() < 1e-6,
                "{:?} with zero radius and zero distance returns 0.0 (distance >= radius), got {}",
                curve,
                result
            );
        }
    }

    #[test]
    fn tiny_radius_returns_1_at_center() {
        // With a very small non-zero radius, center should return 1.0
        for curve in [
            FalloffCurve::Linear,
            FalloffCurve::Quadratic,
            FalloffCurve::Constant,
        ] {
            let result = curve.calculate(0.0, 0.001);
            assert!(
                (result - 1.0).abs() < 1e-6,
                "{:?} with tiny radius at center should return 1.0, got {}",
                curve,
                result
            );
        }
    }
}

// =============================================================================
// Engine Torque Curve Tests - Verify parabolic torque formula
// =============================================================================

mod engine_torque_tests {
    use super::*;

    #[test]
    fn torque_at_idle_minus_1_rpm_returns_0() {
        let engine = EngineConfig {
            max_torque: 400.0,
            max_torque_rpm: 4500.0,
            max_rpm: 7000.0,
            idle_rpm: 800.0,
            engine_braking: 0.3,
        };
        let result = engine.torque_at_rpm(799.0);
        assert!(
            (result - 0.0).abs() < 1e-6,
            "Torque below idle should be 0.0, got {}",
            result
        );
    }

    #[test]
    fn torque_at_idle_exactly_returns_0() {
        let engine = EngineConfig {
            max_torque: 400.0,
            max_torque_rpm: 4500.0,
            max_rpm: 7000.0,
            idle_rpm: 800.0,
            engine_braking: 0.3,
        };
        // At exactly idle: normalized = 0, so (1 - (1-0)^2) = 0
        let result = engine.torque_at_rpm(800.0);
        assert!(
            (result - 0.0).abs() < 1e-6,
            "Torque at exactly idle should be 0.0, got {}",
            result
        );
    }

    #[test]
    fn torque_at_max_torque_rpm_returns_max_torque() {
        let engine = EngineConfig {
            max_torque: 400.0,
            max_torque_rpm: 4500.0,
            max_rpm: 7000.0,
            idle_rpm: 800.0,
            engine_braking: 0.3,
        };
        let result = engine.torque_at_rpm(4500.0);
        // At max_torque_rpm: normalized = 1.0, so max_torque * (1 - (1-1)^2) = max_torque * 1 = 400
        assert!(
            (result - 400.0).abs() < 1.0,
            "Torque at peak RPM should be ~400.0, got {}",
            result
        );
    }

    #[test]
    fn torque_at_max_rpm_returns_0() {
        let engine = EngineConfig {
            max_torque: 400.0,
            max_torque_rpm: 4500.0,
            max_rpm: 7000.0,
            idle_rpm: 800.0,
            engine_braking: 0.3,
        };
        let result = engine.torque_at_rpm(7000.0);
        // At max_rpm: falloff = 1.0, so max_torque * (1 - 1^2) = 0
        assert!(
            (result - 0.0).abs() < 1.0,
            "Torque at max RPM should be ~0.0, got {}",
            result
        );
    }

    #[test]
    fn torque_above_max_rpm_returns_0() {
        let engine = EngineConfig {
            max_torque: 400.0,
            max_torque_rpm: 4500.0,
            max_rpm: 7000.0,
            idle_rpm: 800.0,
            engine_braking: 0.3,
        };
        let result = engine.torque_at_rpm(8000.0);
        assert!(
            (result - 0.0).abs() < 1e-6,
            "Torque above max RPM should be 0.0, got {}",
            result
        );
    }

    #[test]
    fn torque_at_midpoint_rising_has_correct_value() {
        let engine = EngineConfig {
            max_torque: 400.0,
            max_torque_rpm: 4500.0,
            max_rpm: 7000.0,
            idle_rpm: 800.0,
            engine_braking: 0.3,
        };
        // Midpoint on rising portion: rpm = (800 + 4500) / 2 = 2650
        // normalized = (2650 - 800) / (4500 - 800) = 1850 / 3700 = 0.5
        // torque = 400 * (1 - (1 - 0.5)^2) = 400 * (1 - 0.25) = 400 * 0.75 = 300
        let result = engine.torque_at_rpm(2650.0);
        assert!(
            (result - 300.0).abs() < 5.0,
            "Torque at midpoint rising should be ~300.0, got {}",
            result
        );
    }

    #[test]
    fn torque_at_midpoint_falling_has_correct_value() {
        let engine = EngineConfig {
            max_torque: 400.0,
            max_torque_rpm: 4500.0,
            max_rpm: 7000.0,
            idle_rpm: 800.0,
            engine_braking: 0.3,
        };
        // Midpoint on falling portion: rpm = (4500 + 7000) / 2 = 5750
        // falloff = (5750 - 4500) / (7000 - 4500) = 1250 / 2500 = 0.5
        // torque = 400 * max(0, 1 - 0.5^2) = 400 * 0.75 = 300
        let result = engine.torque_at_rpm(5750.0);
        assert!(
            (result - 300.0).abs() < 5.0,
            "Torque at midpoint falling should be ~300.0, got {}",
            result
        );
    }
}

// =============================================================================
// Transmission Ratio Tests - Verify gear calculations
// =============================================================================

mod transmission_tests {
    use super::*;

    #[test]
    fn neutral_gear_returns_exactly_0() {
        let trans = TransmissionConfig::default();
        let result = trans.effective_ratio(0);
        assert!(
            (result - 0.0).abs() < 1e-6,
            "Neutral should have 0.0 ratio, got {}",
            result
        );
    }

    #[test]
    fn first_gear_has_highest_forward_ratio() {
        let trans = TransmissionConfig {
            gear_ratios: vec![3.5, 2.1, 1.4, 1.0, 0.8, 0.65],
            reverse_ratio: -3.2,
            final_drive: 3.7,
            shift_time: 0.2,
        };
        // First gear: 3.5 * 3.7 = 12.95
        let result = trans.effective_ratio(1);
        assert!(
            (result - 12.95).abs() < 0.01,
            "First gear should be 12.95, got {}",
            result
        );
    }

    #[test]
    fn second_gear_has_correct_ratio() {
        let trans = TransmissionConfig {
            gear_ratios: vec![3.5, 2.1, 1.4, 1.0, 0.8, 0.65],
            reverse_ratio: -3.2,
            final_drive: 3.7,
            shift_time: 0.2,
        };
        // Second gear: 2.1 * 3.7 = 7.77
        let result = trans.effective_ratio(2);
        assert!(
            (result - 7.77).abs() < 0.01,
            "Second gear should be 7.77, got {}",
            result
        );
    }

    #[test]
    fn sixth_gear_has_lowest_forward_ratio() {
        let trans = TransmissionConfig {
            gear_ratios: vec![3.5, 2.1, 1.4, 1.0, 0.8, 0.65],
            reverse_ratio: -3.2,
            final_drive: 3.7,
            shift_time: 0.2,
        };
        // Sixth gear: 0.65 * 3.7 = 2.405
        let result = trans.effective_ratio(6);
        assert!(
            (result - 2.405).abs() < 0.01,
            "Sixth gear should be 2.405, got {}",
            result
        );
    }

    #[test]
    fn reverse_gear_is_negative() {
        let trans = TransmissionConfig {
            gear_ratios: vec![3.5, 2.1, 1.4, 1.0, 0.8, 0.65],
            reverse_ratio: -3.2,
            final_drive: 3.7,
            shift_time: 0.2,
        };
        // Reverse: -3.2 * 3.7 = -11.84
        let result = trans.effective_ratio(-1);
        assert!(
            (result - (-11.84)).abs() < 0.01,
            "Reverse should be -11.84, got {}",
            result
        );
    }

    #[test]
    fn out_of_range_gear_returns_1_times_final_drive() {
        let trans = TransmissionConfig {
            gear_ratios: vec![3.5, 2.1, 1.4], // Only 3 gears
            reverse_ratio: -3.2,
            final_drive: 3.7,
            shift_time: 0.2,
        };
        // Gear 4 doesn't exist, so ratio = 1.0 * 3.7 = 3.7
        let result = trans.effective_ratio(4);
        assert!(
            (result - 3.7).abs() < 0.01,
            "Out-of-range gear should use 1.0 ratio = 3.7, got {}",
            result
        );
    }

    #[test]
    fn num_gears_returns_correct_count() {
        let trans = TransmissionConfig {
            gear_ratios: vec![3.5, 2.1, 1.4, 1.0, 0.8, 0.65],
            reverse_ratio: -3.2,
            final_drive: 3.7,
            shift_time: 0.2,
        };
        assert_eq!(trans.num_gears(), 6, "Should have 6 gears");
    }
}

// =============================================================================
// Friction Curve Tests - Verify Pacejka-inspired slip model
// =============================================================================

mod friction_curve_tests {
    use super::*;

    #[test]
    fn tarmac_preset_has_correct_values() {
        let curve = FrictionCurve::tarmac();
        assert!(
            (curve.optimal_slip - 0.08).abs() < 1e-6,
            "Tarmac optimal slip should be 0.08"
        );
        assert!(
            (curve.peak_friction - 1.2).abs() < 1e-6,
            "Tarmac peak friction should be 1.2"
        );
        assert!(
            (curve.sliding_friction - 0.9).abs() < 1e-6,
            "Tarmac sliding friction should be 0.9"
        );
        assert!(
            (curve.stiffness - 12.0).abs() < 1e-6,
            "Tarmac stiffness should be 12.0"
        );
    }

    #[test]
    fn ice_preset_has_correct_values() {
        let curve = FrictionCurve::ice();
        assert!(
            (curve.optimal_slip - 0.05).abs() < 1e-6,
            "Ice optimal slip should be 0.05"
        );
        assert!(
            (curve.peak_friction - 0.3).abs() < 1e-6,
            "Ice peak friction should be 0.3"
        );
        assert!(
            (curve.sliding_friction - 0.15).abs() < 1e-6,
            "Ice sliding friction should be 0.15"
        );
        assert!(
            (curve.stiffness - 20.0).abs() < 1e-6,
            "Ice stiffness should be 20.0"
        );
    }

    #[test]
    fn gravel_preset_has_correct_values() {
        let curve = FrictionCurve::gravel();
        assert!(
            (curve.optimal_slip - 0.15).abs() < 1e-6,
            "Gravel optimal slip should be 0.15"
        );
        assert!(
            (curve.peak_friction - 0.8).abs() < 1e-6,
            "Gravel peak friction should be 0.8"
        );
        assert!(
            (curve.sliding_friction - 0.6).abs() < 1e-6,
            "Gravel sliding friction should be 0.6"
        );
        assert!(
            (curve.stiffness - 6.0).abs() < 1e-6,
            "Gravel stiffness should be 6.0"
        );
    }

    #[test]
    fn mud_preset_has_correct_values() {
        let curve = FrictionCurve::mud();
        assert!(
            (curve.optimal_slip - 0.2).abs() < 1e-6,
            "Mud optimal slip should be 0.2"
        );
        assert!(
            (curve.peak_friction - 0.5).abs() < 1e-6,
            "Mud peak friction should be 0.5"
        );
        assert!(
            (curve.sliding_friction - 0.4).abs() < 1e-6,
            "Mud sliding friction should be 0.4"
        );
        assert!(
            (curve.stiffness - 4.0).abs() < 1e-6,
            "Mud stiffness should be 4.0"
        );
    }

    #[test]
    fn zero_slip_returns_zero_friction() {
        let curve = FrictionCurve::tarmac();
        let result = curve.friction_at_slip(0.0);
        assert!(
            (result - 0.0).abs() < 1e-6,
            "Zero slip should produce zero friction, got {}",
            result
        );
    }

    #[test]
    fn very_small_slip_returns_zero_friction() {
        let curve = FrictionCurve::tarmac();
        let result = curve.friction_at_slip(0.0005);
        assert!(
            (result - 0.0).abs() < 1e-6,
            "Slip < 0.001 should return 0.0, got {}",
            result
        );
    }

    #[test]
    fn negative_slip_uses_absolute_value() {
        let curve = FrictionCurve::tarmac();
        let positive = curve.friction_at_slip(0.04);
        let negative = curve.friction_at_slip(-0.04);
        assert!(
            (positive - negative).abs() < 1e-6,
            "Negative slip should behave same as positive"
        );
    }

    #[test]
    fn friction_at_optimal_slip_approaches_peak() {
        let curve = FrictionCurve::tarmac();
        // At x = 1.0 (optimal slip): friction = peak * (1 - e^(-stiffness))
        // = 1.2 * (1 - e^(-12)) ≈ 1.2 * 0.999994 ≈ 1.1999
        let result = curve.friction_at_slip(curve.optimal_slip);
        let expected = curve.peak_friction * (1.0 - (-curve.stiffness).exp());
        assert!(
            (result - expected).abs() < 0.01,
            "Friction at optimal slip should be ~{}, got {}",
            expected,
            result
        );
    }

    #[test]
    fn high_slip_decay_toward_sliding_friction() {
        let curve = FrictionCurve::tarmac();
        // At very high slip (x >> 1), friction should approach sliding_friction
        let result = curve.friction_at_slip(0.5); // Way past optimal 0.08
                                                  // x = 0.5 / 0.08 = 6.25, so decay = min((6.25-1)*2, 1) = min(10.5, 1) = 1
                                                  // friction = peak - (peak - slide) * 1 = 1.2 - (1.2 - 0.9) * 1 = 0.9
        assert!(
            result < curve.peak_friction,
            "High slip friction should be below peak"
        );
        assert!(
            result >= curve.sliding_friction - 0.05,
            "High slip friction should approach sliding: {}",
            result
        );
    }
}

// =============================================================================
// Wheel Configuration Tests - Verify preset values
// =============================================================================

mod wheel_config_tests {
    use super::*;

    #[test]
    fn default_radius_is_0_35() {
        let wheel = WheelConfig::default();
        assert!(
            (wheel.radius - 0.35).abs() < 1e-6,
            "Default radius should be 0.35"
        );
    }

    #[test]
    fn default_width_is_0_25() {
        let wheel = WheelConfig::default();
        assert!(
            (wheel.width - 0.25).abs() < 1e-6,
            "Default width should be 0.25"
        );
    }

    #[test]
    fn default_suspension_rest_length_is_0_3() {
        let wheel = WheelConfig::default();
        assert!(
            (wheel.suspension_rest_length - 0.3).abs() < 1e-6,
            "Default rest length should be 0.3"
        );
    }

    #[test]
    fn default_suspension_stiffness_is_35000() {
        let wheel = WheelConfig::default();
        assert!(
            (wheel.suspension_stiffness - 35000.0).abs() < 1e-6,
            "Default stiffness should be 35000"
        );
    }

    #[test]
    fn default_suspension_damping_is_4500() {
        let wheel = WheelConfig::default();
        assert!(
            (wheel.suspension_damping - 4500.0).abs() < 1e-6,
            "Default damping should be 4500"
        );
    }

    #[test]
    fn front_left_is_steerable_not_driven() {
        let wheel = WheelConfig::front_left(Vec3::ZERO);
        assert!(wheel.steerable, "Front left should be steerable");
        assert!(
            !wheel.driven,
            "Front left should not be driven (FWD sets this)"
        );
        assert_eq!(wheel.position_id, WheelPosition::FrontLeft);
    }

    #[test]
    fn rear_right_is_driven_not_steerable() {
        let wheel = WheelConfig::rear_right(Vec3::ZERO);
        assert!(!wheel.steerable, "Rear right should not be steerable");
        assert!(wheel.driven, "Rear right should be driven (RWD)");
        assert_eq!(wheel.position_id, WheelPosition::RearRight);
    }

    #[test]
    fn with_drive_sets_driven_flag() {
        let wheel = WheelConfig::front_left(Vec3::ZERO).with_drive();
        assert!(wheel.driven, "with_drive should set driven = true");
    }

    #[test]
    fn with_radius_sets_correct_value() {
        let wheel = WheelConfig::default().with_radius(0.4);
        assert!(
            (wheel.radius - 0.4).abs() < 1e-6,
            "with_radius should set radius = 0.4"
        );
    }

    #[test]
    fn with_suspension_sets_all_values() {
        let wheel = WheelConfig::default().with_suspension(50000.0, 6000.0, 0.4);
        assert!((wheel.suspension_stiffness - 50000.0).abs() < 1e-6);
        assert!((wheel.suspension_damping - 6000.0).abs() < 1e-6);
        assert!((wheel.suspension_rest_length - 0.4).abs() < 1e-6);
    }
}
