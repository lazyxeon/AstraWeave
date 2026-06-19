#![allow(unused_imports, clippy::field_reassign_with_default)]
//! Comprehensive mutation-resistant tests for astraweave-physics.
//!
//! Targets every numeric constant, boundary condition, arithmetic operator,
//! and comparison operator that cargo-mutants can perturb. Organized by module
//! priority (P1 = highest surviving mutant count).

use astraweave_physics::*;
use glam::Vec3;
use std::f32::consts::{PI, SQRT_2};

// ============================================================================
// P1: CharacterController (~50 estimated surviving mutants)
// ============================================================================

mod character_controller_mutations {
    use super::*;

    #[test]
    fn volume_formula_cylinder_component() {
        // volume = PI * r^2 * (height - 2*radius) + (4/3) * PI * r^3
        let cc = CharacterController::new(0.5, 2.0);
        let cylinder_h = 2.0 - 2.0 * 0.5; // = 1.0
        let cylinder_vol = PI * 0.5 * 0.5 * cylinder_h;
        let sphere_vol = (4.0 / 3.0) * PI * 0.5_f32.powi(3);
        let expected = cylinder_vol + sphere_vol;
        assert!(
            (cc.volume() - expected).abs() < 1e-5,
            "Volume mismatch: got {} expected {}",
            cc.volume(),
            expected
        );
    }

    #[test]
    fn volume_sphere_dominant() {
        // When height == 2*radius, cylinder_height = 0, pure sphere
        let cc = CharacterController::new(1.0, 2.0);
        let sphere_vol = (4.0 / 3.0) * PI * 1.0_f32.powi(3);
        assert!(
            (cc.volume() - sphere_vol).abs() < 1e-5,
            "Sphere-only volume: got {} expected {}",
            cc.volume(),
            sphere_vol
        );
    }

    #[test]
    fn volume_tall_capsule() {
        let cc = CharacterController::new(0.3, 3.0);
        let cyl_h = 3.0 - 2.0 * 0.3;
        let cyl = PI * 0.3 * 0.3 * cyl_h;
        let sph = (4.0 / 3.0) * PI * 0.3_f32.powi(3);
        assert!((cc.volume() - (cyl + sph)).abs() < 1e-5);
    }

    #[test]
    fn is_falling_threshold_negative_001() {
        let mut cc = CharacterController::new(0.5, 2.0);
        cc.vertical_velocity = -0.02; // below -0.01 → falling
        assert!(cc.is_falling());

        cc.vertical_velocity = -0.01; // exactly at threshold → NOT falling
        assert!(!cc.is_falling());

        cc.vertical_velocity = -0.005; // above -0.01 → NOT falling
        assert!(!cc.is_falling());
    }

    #[test]
    fn is_rising_threshold_positive_001() {
        let mut cc = CharacterController::new(0.5, 2.0);
        cc.vertical_velocity = 0.02; // above 0.01 → rising
        assert!(cc.is_rising());

        cc.vertical_velocity = 0.01; // exactly at threshold → NOT rising
        assert!(!cc.is_rising());

        cc.vertical_velocity = 0.005; // below 0.01 → NOT rising
        assert!(!cc.is_rising());
    }

    #[test]
    fn is_falling_and_rising_mutually_exclusive() {
        let mut cc = CharacterController::new(0.5, 2.0);
        cc.vertical_velocity = 0.0;
        assert!(!cc.is_falling());
        assert!(!cc.is_rising());
    }

    #[test]
    fn coyote_time_defaults() {
        let cc = CharacterController::new(0.5, 2.0);
        assert_eq!(cc.coyote_time_limit, 0.15);
        assert_eq!(cc.jump_buffer_limit, 0.15);
    }

    #[test]
    fn has_coyote_time_boundary() {
        let mut cc = CharacterController::new(0.5, 2.0);
        cc.time_since_grounded = 0.14; // within limit
        assert!(cc.has_coyote_time());

        cc.time_since_grounded = 0.15; // at limit → NOT (< not <=)
        assert!(!cc.has_coyote_time());

        cc.time_since_grounded = 0.16; // past limit
        assert!(!cc.has_coyote_time());
    }

    #[test]
    fn has_buffered_jump_boundary() {
        let mut cc = CharacterController::new(0.5, 2.0);
        cc.jump_buffer_timer = 0.01;
        assert!(cc.has_buffered_jump());

        cc.jump_buffer_timer = 0.0; // exactly 0 → NOT (> 0.0)
        assert!(!cc.has_buffered_jump());

        cc.jump_buffer_timer = -0.01; // negative
        assert!(!cc.has_buffered_jump());
    }

    #[test]
    fn can_jump_when_grounded() {
        let cc = CharacterController::new(0.5, 2.0);
        assert!(cc.is_grounded());
        assert!(cc.can_jump());
    }

    #[test]
    fn max_climb_angle_deg_to_rad() {
        let cc = CharacterController::new(0.5, 2.0);
        let expected = 45.0_f32.to_radians();
        assert!((cc.max_climb_angle_rad() - expected).abs() < 1e-6);
    }

    #[test]
    fn constructor_defaults() {
        let cc = CharacterController::new(0.4, 1.8);
        assert_eq!(cc.radius, 0.4);
        assert_eq!(cc.height, 1.8);
        assert_eq!(cc.max_climb_angle_deg, 45.0);
        assert_eq!(cc.max_step, 0.3);
        assert_eq!(cc.vertical_velocity, 0.0);
        assert_eq!(cc.gravity_scale, 1.0);
        assert_eq!(cc.time_since_grounded, 0.0);
        assert_eq!(cc.jump_buffer_timer, 0.0);
        assert_eq!(cc.pending_jump_velocity, 0.0);
    }

    #[test]
    fn reset_clears_all_state() {
        let mut cc = CharacterController::new(0.5, 2.0);
        cc.vertical_velocity = 5.0;
        cc.time_since_grounded = 1.0;
        cc.jump_buffer_timer = 0.5;
        cc.pending_jump_velocity = 3.0;

        cc.reset();

        assert_eq!(cc.vertical_velocity, 0.0);
        assert_eq!(cc.time_since_grounded, 0.0);
        assert_eq!(cc.jump_buffer_timer, 0.0);
        assert_eq!(cc.pending_jump_velocity, 0.0);
        assert!(cc.is_grounded());
    }
}

// ============================================================================
// P2: Vehicle Physics (~40 estimated surviving mutants)
// ============================================================================

mod vehicle_friction_curve_mutations {
    use super::*;
    use astraweave_physics::vehicle::*;
    #[allow(unused_imports)]
    #[test]
    fn friction_at_zero_slip_returns_zero() {
        let fc = FrictionCurve::default();
        assert_eq!(fc.friction_at_slip(0.0), 0.0);
    }

    #[test]
    fn friction_at_very_small_slip_returns_zero() {
        let fc = FrictionCurve::default();
        // Below 0.001 threshold
        assert_eq!(fc.friction_at_slip(0.0005), 0.0);
    }

    #[test]
    fn friction_at_optimal_slip_near_peak() {
        let fc = FrictionCurve::default();
        // At optimal_slip (0.08), x = 1.0, should be near peak_friction
        let f = fc.friction_at_slip(0.08);
        // rising: peak * (1.0 - exp(-stiffness * 1.0)) = 1.2 * (1 - exp(-10))
        let expected = 1.2 * (1.0 - (-10.0_f32).exp());
        assert!(
            (f - expected).abs() < 0.01,
            "At optimal slip: got {} expected {}",
            f,
            expected
        );
    }

    #[test]
    fn friction_negative_slip_uses_abs() {
        let fc = FrictionCurve::default();
        let positive = fc.friction_at_slip(0.05);
        let negative = fc.friction_at_slip(-0.05);
        assert!(
            (positive - negative).abs() < 1e-6,
            "Negative slip should give same friction as positive"
        );
    }

    #[test]
    fn friction_beyond_optimal_decays() {
        let fc = FrictionCurve::default();
        // Past optimal: x > 1.0, enters falling portion
        let at_opt = fc.friction_at_slip(0.08);
        let past_opt = fc.friction_at_slip(0.16); // x = 2.0
        assert!(
            past_opt < at_opt,
            "Friction should decay past optimal slip: {} should be < {}",
            past_opt,
            at_opt
        );
    }

    #[test]
    fn friction_far_past_optimal_approaches_sliding() {
        let fc = FrictionCurve::default();
        // At x = 1.0 + 0.5 = 1.5 optimal (decay = min((0.5)*2, 1) = 1.0)
        // So peak - (peak - slide) * 1.0 = slide
        let f = fc.friction_at_slip(0.08 * 1.5);
        // decay = ((1.5-1.0)*2.0).min(1.0) = 1.0
        let expected = 1.2 - (1.2 - 0.8) * 1.0; // = 0.8
        assert!(
            (f - expected).abs() < 0.01,
            "Far past optimal: got {} expected {}",
            f,
            expected
        );
    }

    #[test]
    fn friction_presets_differ() {
        let tarmac = FrictionCurve::tarmac();
        let gravel = FrictionCurve::gravel();
        let ice = FrictionCurve::ice();
        let mud = FrictionCurve::mud();

        assert!(tarmac.peak_friction > gravel.peak_friction);
        assert!(gravel.peak_friction > mud.peak_friction);
        assert!(mud.peak_friction > ice.peak_friction);
    }

    #[test]
    fn tarmac_defaults() {
        let t = FrictionCurve::tarmac();
        assert_eq!(t.optimal_slip, 0.08);
        assert_eq!(t.peak_friction, 1.2);
        assert_eq!(t.sliding_friction, 0.9);
        assert_eq!(t.stiffness, 12.0);
    }

    #[test]
    fn gravel_defaults() {
        let g = FrictionCurve::gravel();
        assert_eq!(g.optimal_slip, 0.15);
        assert_eq!(g.peak_friction, 0.8);
        assert_eq!(g.sliding_friction, 0.6);
        assert_eq!(g.stiffness, 6.0);
    }

    #[test]
    fn ice_defaults() {
        let i = FrictionCurve::ice();
        assert_eq!(i.optimal_slip, 0.05);
        assert_eq!(i.peak_friction, 0.3);
        assert_eq!(i.sliding_friction, 0.15);
        assert_eq!(i.stiffness, 20.0);
    }

    #[test]
    fn mud_defaults() {
        let m = FrictionCurve::mud();
        assert_eq!(m.optimal_slip, 0.2);
        assert_eq!(m.peak_friction, 0.5);
        assert_eq!(m.sliding_friction, 0.4);
        assert_eq!(m.stiffness, 4.0);
    }

    #[test]
    fn engine_torque_at_rpm_below_idle_is_zero() {
        let engine = EngineConfig::default();
        assert_eq!(engine.torque_at_rpm(0.0), 0.0);
        assert_eq!(engine.torque_at_rpm(700.0), 0.0); // below idle_rpm=800
    }

    #[test]
    fn engine_torque_at_rpm_above_max_is_zero() {
        let engine = EngineConfig::default();
        assert_eq!(engine.torque_at_rpm(7001.0), 0.0); // above max_rpm=7000
        assert_eq!(engine.torque_at_rpm(10000.0), 0.0);
    }

    #[test]
    fn engine_torque_at_max_torque_rpm() {
        let engine = EngineConfig::default();
        // At max_torque_rpm, normalized = 1.0, so torque = max_torque * (1-(1-1)^2) = max_torque
        let t = engine.torque_at_rpm(engine.max_torque_rpm);
        assert!(
            (t - engine.max_torque).abs() < 0.1,
            "At peak RPM: got {} expected {}",
            t,
            engine.max_torque
        );
    }

    #[test]
    fn engine_torque_rising_portion() {
        let engine = EngineConfig::default();
        // Midpoint between idle and max_torque_rpm
        let mid_rpm = (engine.idle_rpm + engine.max_torque_rpm) / 2.0;
        let t = engine.torque_at_rpm(mid_rpm);
        assert!(t > 0.0, "Torque should be positive between idle and peak");
        assert!(
            t < engine.max_torque,
            "Torque should be below peak between idle and peak"
        );
    }

    #[test]
    fn engine_torque_falling_portion() {
        let engine = EngineConfig::default();
        // Midpoint between max_torque_rpm and max_rpm
        let mid_rpm = (engine.max_torque_rpm + engine.max_rpm) / 2.0;
        let t = engine.torque_at_rpm(mid_rpm);
        assert!(t > 0.0, "Torque still positive after peak");
        assert!(t < engine.max_torque, "Torque should decay after peak RPM");
    }

    #[test]
    fn engine_config_defaults() {
        let e = EngineConfig::default();
        assert_eq!(e.max_torque, 400.0);
        assert_eq!(e.max_torque_rpm, 4500.0);
        assert_eq!(e.max_rpm, 7000.0);
        assert_eq!(e.idle_rpm, 800.0);
        assert_eq!(e.engine_braking, 0.3);
    }

    #[test]
    fn transmission_effective_ratio_neutral() {
        let t = TransmissionConfig::default();
        assert_eq!(t.effective_ratio(0), 0.0);
    }

    #[test]
    fn transmission_effective_ratio_reverse() {
        let t = TransmissionConfig::default();
        let ratio = t.effective_ratio(-1);
        assert_eq!(ratio, t.reverse_ratio * t.final_drive);
    }

    #[test]
    fn transmission_effective_ratio_first_gear() {
        let t = TransmissionConfig::default();
        let ratio = t.effective_ratio(1);
        assert_eq!(ratio, t.gear_ratios[0] * t.final_drive);
    }

    #[test]
    fn transmission_effective_ratio_out_of_range() {
        let t = TransmissionConfig::default();
        // gear_ratios has 6 entries, gear 7 → unwrap_or(1.0)
        let ratio = t.effective_ratio(7);
        assert_eq!(ratio, 1.0 * t.final_drive);
    }

    #[test]
    fn transmission_defaults() {
        let t = TransmissionConfig::default();
        assert_eq!(t.gear_ratios, vec![3.5, 2.1, 1.4, 1.0, 0.8, 0.65]);
        assert_eq!(t.reverse_ratio, -3.2);
        assert_eq!(t.final_drive, 3.7);
        assert_eq!(t.shift_time, 0.2);
        assert_eq!(t.num_gears(), 6);
    }

    #[test]
    fn wheel_config_defaults() {
        let w = WheelConfig::default();
        assert_eq!(w.radius, 0.35);
        assert_eq!(w.width, 0.25);
        assert!(!w.steerable);
        assert!(!w.driven);
        assert_eq!(w.suspension_rest_length, 0.3);
        assert_eq!(w.suspension_stiffness, 35000.0);
        assert_eq!(w.suspension_damping, 4500.0);
        assert_eq!(w.suspension_max_compression, 0.1);
        assert_eq!(w.suspension_max_extension, 0.2);
    }

    #[test]
    fn wheel_config_front_left_is_steerable_not_driven() {
        let w = WheelConfig::front_left(Vec3::new(-0.8, 0.0, 1.2));
        assert!(w.steerable);
        assert!(!w.driven);
        assert_eq!(w.position_id, WheelPosition::FrontLeft);
    }

    #[test]
    fn wheel_config_rear_left_is_driven_not_steerable() {
        let w = WheelConfig::rear_left(Vec3::new(-0.8, 0.0, -1.2));
        assert!(!w.steerable);
        assert!(w.driven);
        assert_eq!(w.position_id, WheelPosition::RearLeft);
    }

    #[test]
    fn vehicle_speed_conversions() {
        let config = VehicleConfig::default();
        let mut v = Vehicle::new(1, 1, config);
        v.speed = 10.0; // 10 m/s
        assert!((v.speed_kmh() - 36.0).abs() < 0.1); // 10 * 3.6
        assert!((v.speed_mph() - 22.37).abs() < 0.1); // 10 * 2.237
    }

    #[test]
    fn vehicle_shift_mechanics() {
        let config = VehicleConfig::default();
        let mut v = Vehicle::new(1, 1, config);
        assert_eq!(v.current_gear, 1);

        v.shift_up();
        assert_eq!(v.current_gear, 2);
        assert!(v.is_shifting());
        assert!(v.shift_timer > 0.0);

        // Can't shift while already shifting
        let gear_before = v.current_gear;
        v.shift_up();
        assert_eq!(v.current_gear, gear_before);
    }

    #[test]
    fn vehicle_shift_down_limits() {
        let config = VehicleConfig::default();
        let mut v = Vehicle::new(1, 1, config);
        v.shift_down(); // 1 → 0 (neutral)
        assert_eq!(v.current_gear, 0);

        v.shift_timer = 0.0; // clear shift timer
        v.shift_down(); // 0 → -1 (reverse)
        assert_eq!(v.current_gear, -1);

        v.shift_timer = 0.0;
        v.shift_down(); // -1 → can't go below
        assert_eq!(v.current_gear, -1);
    }

    #[test]
    fn vehicle_airborne_detection() {
        let config = VehicleConfig::default();
        let v = Vehicle::new(1, 1, config);
        // All wheels not grounded by default
        assert!(v.is_airborne());
        assert_eq!(v.grounded_wheels(), 0);
    }

    #[test]
    fn vehicle_config_defaults() {
        let c = VehicleConfig::default();
        assert_eq!(c.mass, 1500.0);
        assert_eq!(c.wheels.len(), 4);
        assert_eq!(c.drag_coefficient, 0.35);
        assert_eq!(c.frontal_area, 2.2);
        assert_eq!(c.max_steering_angle, 0.6);
        assert_eq!(c.brake_force, 15000.0);
        assert_eq!(c.handbrake_multiplier, 2.0);
    }
}

// ============================================================================
// P3: Cloth Simulation (~20 estimated surviving mutants)
// ============================================================================

mod cloth_simulation_mutations {
    use super::*;
    use astraweave_physics::cloth::*;
    #[allow(unused_imports)]
    #[test]
    fn cloth_particle_verlet_integration() {
        let mut p = ClothParticle::new(Vec3::ZERO, 1.0);
        // Move particle so velocity = (1,0,0)
        p.prev_position = Vec3::new(-1.0, 0.0, 0.0);

        let dt = 1.0 / 60.0;
        let damping = 0.98;
        // Apply gravity
        p.apply_force(Vec3::new(0.0, -9.81, 0.0)); // force / mass = accel

        p.integrate(dt, damping);

        // Verlet: new_pos = pos + velocity*damping + accel * dt * dt
        // velocity = (0,0,0) - (-1,0,0) = (1,0,0)
        // accel = force * inv_mass = (0, -9.81, 0) * 1.0
        let expected_x = 0.0 + 1.0 * damping + 0.0; // no x accel
        let expected_y = 0.0 + 0.0 * damping + (-9.81) * dt * dt; // dt*dt not dt
        assert!(
            (p.position.x - expected_x).abs() < 1e-4,
            "X: got {} expected {}",
            p.position.x,
            expected_x
        );
        assert!(
            (p.position.y - expected_y).abs() < 1e-4,
            "Y: got {} expected {}",
            p.position.y,
            expected_y
        );
    }

    #[test]
    fn cloth_particle_pinned_doesnt_move() {
        let mut p = ClothParticle::new(Vec3::ZERO, 1.0);
        p.pinned = true;
        p.apply_force(Vec3::new(100.0, 100.0, 100.0));
        p.integrate(1.0 / 60.0, 0.98);
        assert_eq!(p.position, Vec3::ZERO);
    }

    #[test]
    fn cloth_particle_velocity() {
        let mut p = ClothParticle::new(Vec3::new(1.0, 0.0, 0.0), 1.0);
        p.prev_position = Vec3::ZERO;
        assert_eq!(p.velocity(), Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn cloth_particle_inv_mass() {
        let p = ClothParticle::new(Vec3::ZERO, 2.0);
        assert!((p.inv_mass - 0.5).abs() < 1e-6);
    }

    #[test]
    fn distance_constraint_stiffness_half_split() {
        // correction = delta * diff * 0.5 * stiffness
        // Then distributed by inv_mass ratio: w1/(w1+w2) and w2/(w2+w2)
        let mut particles = vec![
            ClothParticle::new(Vec3::ZERO, 1.0),
            ClothParticle::new(Vec3::new(2.0, 0.0, 0.0), 1.0),
        ];
        let mut c = DistanceConstraint::new(0, 1, 1.0); // rest=1.0, actual=2.0
        c.stiffness = 1.0;

        c.solve(&mut particles);

        // delta = (2,0,0), current_length=2.0, diff = (2-1)/2 = 0.5
        // correction = (2,0,0) * 0.5 * 0.5 * 1.0 = (0.5, 0, 0)
        // w1=1, w2=1, total=2  →  each gets 0.5 of correction
        // p0 += (0.5,0,0) * 0.5 = (0.25, 0, 0)
        // p1 -= (0.5,0,0) * 0.5 = (1.75, 0, 0)
        assert!(
            (particles[0].position.x - 0.25).abs() < 1e-4,
            "P0 should move to 0.25, got {}",
            particles[0].position.x
        );
        assert!(
            (particles[1].position.x - 1.75).abs() < 1e-4,
            "P1 should move to 1.75, got {}",
            particles[1].position.x
        );
        // Verify the 0.5 factor: if it were 1.0 instead, each would move by 0.5
        // This catches mutations that change the 0.5 constant
        assert!(
            particles[0].position.x < 0.3,
            "0.5 factor should limit movement"
        );
    }

    #[test]
    fn distance_constraint_threshold() {
        // Length below 0.0001 → no correction
        let mut particles = vec![
            ClothParticle::new(Vec3::ZERO, 1.0),
            ClothParticle::new(Vec3::new(0.00005, 0.0, 0.0), 1.0),
        ];
        let mut c = DistanceConstraint::new(0, 1, 0.0);
        c.stiffness = 1.0;

        c.solve(&mut particles);

        // Should be unchanged because distance < 0.0001
        assert_eq!(particles[0].position, Vec3::ZERO);
    }

    #[test]
    fn distance_constraint_pinned_particle_doesnt_move() {
        let mut particles = vec![
            ClothParticle::new(Vec3::ZERO, 1.0),
            ClothParticle::new(Vec3::new(3.0, 0.0, 0.0), 1.0),
        ];
        particles[0].pinned = true;
        particles[0].inv_mass = 0.0;

        let mut c = DistanceConstraint::new(0, 1, 1.0);
        c.stiffness = 1.0;
        c.solve(&mut particles);

        // p0 should NOT move (pinned)
        assert_eq!(particles[0].position, Vec3::ZERO, "Pinned particle moved!");
        // p1 absorbs correction proportional to its weight ratio
        // w1=0 (pinned), w2=1, total=1 → p2 gets full correction
        // delta=(3,0,0), current=3.0, diff=(3-1)/3=0.667, correction=(3,0,0)*0.667*0.5*1.0=(1.0,0,0)
        // p1 -= (1.0,0,0) * (1/1) = moves to (2.0, 0, 0)
        assert!(
            (particles[1].position.x - 2.0).abs() < 1e-4,
            "p1 should move toward rest length, got {}",
            particles[1].position.x
        );
        // p1 moved closer to rest length but not all the way (due to 0.5 factor)
        assert!(
            particles[1].position.x < 3.0,
            "p1 should have moved toward p0"
        );
        assert!(
            particles[1].position.x > 1.0,
            "p1 should not overshoot rest length in one step"
        );
    }

    #[test]
    fn cloth_shear_constraint_uses_sqrt2() {
        let config = ClothConfig {
            width: 3,
            height: 3,
            spacing: 1.0,
            stiffness: 0.8,
            ..Default::default()
        };
        let cloth = Cloth::new(ClothId(0), config, Vec3::ZERO);

        // Find a shear (diagonal) constraint — rest length should be spacing * SQRT_2
        let shear = cloth
            .constraints
            .iter()
            .find(|c| (c.rest_length - SQRT_2).abs() < 0.01);
        assert!(
            shear.is_some(),
            "Should have shear constraints with rest_length = SQRT_2"
        );
        // Shear stiffness = config.stiffness * 0.5
        let s = shear.unwrap();
        assert!(
            (s.stiffness - 0.8 * 0.5).abs() < 1e-4,
            "Shear stiffness should be 0.5 * config.stiffness"
        );
    }

    #[test]
    fn cloth_bend_constraint_uses_2x_spacing() {
        let config = ClothConfig {
            width: 4,
            height: 4,
            spacing: 0.5,
            stiffness: 0.8,
            ..Default::default()
        };
        let cloth = Cloth::new(ClothId(0), config, Vec3::ZERO);

        // Bend constraints have rest_length = spacing * 2.0
        let bend = cloth
            .constraints
            .iter()
            .find(|c| (c.rest_length - 1.0).abs() < 0.01);
        assert!(
            bend.is_some(),
            "Should have bend constraints with rest_length = 2*spacing"
        );
        let b = bend.unwrap();
        assert!(
            (b.stiffness - 0.8 * 0.3).abs() < 1e-4,
            "Bend stiffness should be 0.3 * config.stiffness"
        );
    }

    #[test]
    fn cloth_config_defaults() {
        let c = ClothConfig::default();
        assert_eq!(c.width, 20);
        assert_eq!(c.height, 20);
        assert_eq!(c.spacing, 0.1);
        assert_eq!(c.particle_mass, 0.1);
        assert_eq!(c.stiffness, 0.8);
        assert_eq!(c.damping, 0.98);
        assert_eq!(c.solver_iterations, 3);
        assert_eq!(c.gravity, Vec3::new(0.0, -9.81, 0.0));
        assert_eq!(c.wind, Vec3::ZERO);
        assert_eq!(c.air_resistance, 0.01);
    }

    #[test]
    fn cloth_particle_count() {
        let config = ClothConfig {
            width: 5,
            height: 3,
            ..Default::default()
        };
        let cloth = Cloth::new(ClothId(0), config, Vec3::ZERO);
        assert_eq!(cloth.particles.len(), 15); // 5*3
    }

    #[test]
    fn cloth_pin_top_edge() {
        let config = ClothConfig {
            width: 5,
            height: 3,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(0), config, Vec3::ZERO);
        cloth.pin_top_edge();

        for x in 0..5 {
            assert!(
                cloth.particles[x].pinned,
                "Top edge particle {} not pinned",
                x
            );
            assert_eq!(cloth.particles[x].inv_mass, 0.0);
        }
        // Non-top particles should not be pinned
        assert!(!cloth.particles[5].pinned);
    }

    #[test]
    fn cloth_collider_sphere_penetration() {
        let mut p = ClothParticle::new(Vec3::new(0.5, 0.0, 0.0), 1.0);
        p.prev_position = p.position; // zero velocity

        let collider = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 1.0,
        };

        collider.resolve_collision(&mut p, 0.0);

        // Particle at 0.5 is inside sphere of radius 1.0
        // Should be pushed outward to radius 1.0
        assert!(
            (p.position.length() - 1.0).abs() < 0.01,
            "Particle should be pushed to sphere surface, got {}",
            p.position.length()
        );
    }

    #[test]
    fn cloth_collider_plane() {
        let mut p = ClothParticle::new(Vec3::new(0.0, -0.5, 0.0), 1.0);
        p.prev_position = p.position;

        let collider = ClothCollider::Plane {
            point: Vec3::ZERO,
            normal: Vec3::Y,
        };

        collider.resolve_collision(&mut p, 0.0);

        // Particle at y=-0.5 is below plane at y=0
        // Should be pushed up to y=0
        assert!(
            p.position.y >= -0.01,
            "Particle should be pushed above plane, got y={}",
            p.position.y
        );
    }
}

// ============================================================================
// P4: Environment / Wind (~25 estimated surviving mutants)
// ============================================================================

mod environment_wind_mutations {
    use super::*;
    use astraweave_physics::environment::*;
    #[allow(unused_imports)]
    #[test]
    fn wind_force_formula_exact() {
        // F = 0.5 * 1.225 * speed^2 * drag * area
        let config = WindZoneConfig {
            shape: WindZoneShape::Global,
            wind_type: WindType::Directional,
            direction: Vec3::new(1.0, 0.0, 0.0),
            strength: 10.0,
            falloff: 0.0, // no falloff
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);

        let drag = 1.5;
        let area = 2.0;
        let force = zone.wind_force_at(Vec3::ZERO, drag, area);

        let expected = 0.5 * 1.225 * 10.0 * 10.0 * drag * area;
        assert!(
            (force.x - expected).abs() < 0.1,
            "Wind force: got {} expected {}",
            force.x,
            expected
        );
    }

    #[test]
    fn wind_force_inactive_zone_returns_zero() {
        let config = WindZoneConfig {
            active: false,
            strength: 100.0,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);
        let force = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        assert_eq!(force, Vec3::ZERO);
    }

    #[test]
    fn wind_force_outside_zone_returns_zero() {
        let config = WindZoneConfig {
            shape: WindZoneShape::Sphere { radius: 5.0 },
            position: Vec3::ZERO,
            strength: 100.0,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);
        let force = zone.wind_force_at(Vec3::new(100.0, 0.0, 0.0), 1.0, 1.0);
        assert_eq!(force, Vec3::ZERO);
    }

    #[test]
    fn vortex_wind_at_center_pure_updraft() {
        let config = WindZoneConfig {
            shape: WindZoneShape::Global,
            wind_type: WindType::Vortex {
                tangential_speed: 10.0,
                inward_pull: 5.0,
                updraft: 3.0,
            },
            strength: 10.0,
            position: Vec3::ZERO,
            falloff: 0.0,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);

        // At center (dist < 0.1), only updraft applies
        let force = zone.wind_force_at(Vec3::new(0.01, 0.0, 0.01), 1.0, 1.0);
        // The updraft value is 3.0, and the wind formula applies F = 0.5*1.225*v^2*drag*area
        // So there should be a Y component
        assert!(
            force.y > 0.0,
            "Vortex at center should have upward force, got {:?}",
            force
        );
    }

    #[test]
    fn wind_speed_below_threshold_returns_zero() {
        let config = WindZoneConfig {
            shape: WindZoneShape::Global,
            wind_type: WindType::Directional,
            direction: Vec3::X,
            strength: 0.001, // very small → effective velocity < 0.01
            falloff: 0.0,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);
        let force = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        assert_eq!(force, Vec3::ZERO, "Sub-threshold wind should be zero");
    }

    #[test]
    fn falloff_no_falloff() {
        let config = WindZoneConfig {
            shape: WindZoneShape::Sphere { radius: 10.0 },
            position: Vec3::ZERO,
            falloff: 0.0, // disabled
            strength: 10.0,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);
        // Force at center and near edge should be same (no falloff)
        let f_center = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        let f_edge = zone.wind_force_at(Vec3::new(9.0, 0.0, 0.0), 1.0, 1.0);
        assert!(
            (f_center.x - f_edge.x).abs() < 0.1,
            "With falloff=0, force should be same everywhere"
        );
    }

    #[test]
    fn gust_event_envelope_attack_phase() {
        let g = GustEvent::new(Vec3::X, 100.0, 1.0);
        // At t=0, elapsed=0, attack = (0*4).min(1) = 0
        assert_eq!(g.current_strength(), 0.0);
    }

    #[test]
    fn gust_event_envelope_peak() {
        let mut g = GustEvent::new(Vec3::X, 100.0, 1.0);
        g.elapsed = 0.5; // t=0.5, attack=(2.0).min(1)=1, release=((0.5)*4).min(1)=1
        let s = g.current_strength();
        assert!(
            (s - 100.0).abs() < 1.0,
            "Mid-gust should be near peak: got {}",
            s
        );
    }

    #[test]
    fn gust_event_finished() {
        let mut g = GustEvent::new(Vec3::X, 100.0, 1.0);
        g.elapsed = 1.0;
        assert!(g.is_finished());
        assert_eq!(g.current_strength(), 0.0);
    }

    #[test]
    fn gust_event_smoothness_zero_is_constant() {
        let mut g = GustEvent::new(Vec3::X, 100.0, 1.0);
        g.smoothness = 0.0;
        g.elapsed = 0.1;
        assert_eq!(
            g.current_strength(),
            100.0,
            "Smoothness=0 → constant strength"
        );
    }

    #[test]
    fn environment_manager_wind_zone_lifecycle() {
        let mut mgr = EnvironmentManager::new();
        let id = mgr.add_wind_zone(WindZoneConfig::default());
        assert!(mgr.get_wind_zone(id).is_some());
        assert!(mgr.remove_wind_zone(id));
        assert!(mgr.get_wind_zone(id).is_none());
    }
}

// ============================================================================
// P5: Destruction System (~15 estimated surviving mutants)
// ============================================================================

mod destruction_mutations {
    use super::*;
    use astraweave_physics::destruction::*;
    #[allow(unused_imports)]
    #[test]
    fn fracture_uniform_piece_count_uses_cbrt() {
        let fp = FracturePattern::uniform(27, Vec3::splat(3.0), 27.0);
        // pieces_per_axis = cbrt(27).ceil() = 3
        // total pieces = 3*3*3 = 27
        assert_eq!(fp.debris.len(), 27);
    }

    #[test]
    fn fracture_uniform_piece_count_rounds_up() {
        let fp = FracturePattern::uniform(10, Vec3::splat(3.0), 10.0);
        // pieces_per_axis = cbrt(10).ceil() = ceil(2.154) = 3
        // The loop caps at piece_count via `if debris.len() >= piece_count { break; }`
        // So we get exactly 10, not 3*3*3=27
        assert_eq!(fp.debris.len(), 10);
        // Verify cbrt().ceil() determines grid subdivision (3, not 2)
        // With axis=3, piece_size = half_extents * 2 / 3 = 2.0
        let piece_size = Vec3::splat(3.0) * 2.0 / 3.0;
        if let DebrisShape::Box { half_extents } = fp.debris[0].shape {
            let expected = piece_size * 0.4;
            assert!((half_extents - expected).length() < 0.01);
        }
    }

    #[test]
    fn fracture_uniform_debris_half_extents_use_04_factor() {
        let fp = FracturePattern::uniform(8, Vec3::splat(2.0), 8.0);
        // pieces_per_axis = cbrt(8) = 2
        // piece_size = half_extents * 2.0 / 2 = half_extents
        // debris half_extents = piece_size * 0.4
        let piece_size = Vec3::splat(2.0) * 2.0 / 2.0; // = (2.0, 2.0, 2.0)
        let expected_he = piece_size * 0.4; // = (0.8, 0.8, 0.8)
        if let DebrisShape::Box { half_extents } = fp.debris[0].shape {
            assert!(
                (half_extents - expected_he).length() < 0.01,
                "Debris half_extents: got {:?} expected {:?}",
                half_extents,
                expected_he
            );
        } else {
            panic!("Expected Box debris shape");
        }
    }

    #[test]
    fn fracture_uniform_mass_distribution() {
        let total_mass = 100.0;
        let fp = FracturePattern::uniform(8, Vec3::splat(1.0), total_mass);
        let piece_mass = total_mass / fp.debris.len() as f32;
        for d in &fp.debris {
            assert!(
                (d.mass - piece_mass).abs() < 0.01,
                "Each debris mass should be {}. got {}",
                piece_mass,
                d.mass
            );
        }
    }

    #[test]
    fn fracture_radial_golden_angle() {
        let fp = FracturePattern::radial(10, 5.0, 10.0);
        assert_eq!(fp.debris.len(), 10);
        // Each piece should have velocity_factor = 1.5 (radial)
        for d in &fp.debris {
            assert_eq!(
                d.velocity_factor, 1.5,
                "Radial velocity_factor should be 1.5"
            );
        }
    }

    #[test]
    fn debris_config_defaults() {
        let d = DebrisConfig::default();
        assert_eq!(d.velocity_factor, 1.0);
        assert_eq!(d.angular_velocity_factor, 0.5);
        assert_eq!(d.lifetime, 10.0);
    }

    #[test]
    fn destructible_apply_damage_reduces_health() {
        let config = DestructibleConfig::default();
        let mut d = Destructible::new(DestructibleId(1), config, Vec3::ZERO);
        assert_eq!(d.health, 100.0);

        d.apply_damage(30.0);
        assert_eq!(d.health, 70.0);
        assert_eq!(d.state, DestructibleState::Intact); // 70% > 50%
    }

    #[test]
    fn destructible_damage_threshold_50_percent() {
        let config = DestructibleConfig::default();
        let mut d = Destructible::new(DestructibleId(1), config, Vec3::ZERO);

        d.apply_damage(51.0); // health = 49, < 50% of 100
        assert_eq!(d.state, DestructibleState::Damaged);
    }

    #[test]
    fn destructible_fatal_damage() {
        let config = DestructibleConfig::default();
        let mut d = Destructible::new(DestructibleId(1), config, Vec3::ZERO);

        d.apply_damage(100.0);
        assert_eq!(d.health, 0.0);
        assert_eq!(d.state, DestructibleState::Destroying);
    }

    #[test]
    fn destructible_health_clamped_at_zero() {
        let config = DestructibleConfig::default();
        let mut d = Destructible::new(DestructibleId(1), config, Vec3::ZERO);

        d.apply_damage(200.0); // overkill
        assert_eq!(d.health, 0.0); // .max(0.0) prevents negative
    }

    #[test]
    fn destructible_no_damage_when_destroying() {
        let config = DestructibleConfig::default();
        let mut d = Destructible::new(DestructibleId(1), config, Vec3::ZERO);
        d.state = DestructibleState::Destroying;

        d.apply_damage(50.0);
        assert_eq!(d.health, 100.0); // should be unchanged
    }

    #[test]
    fn destructible_force_trigger() {
        let config = DestructibleConfig {
            trigger: DestructionTrigger::Force { threshold: 500.0 },
            ..Default::default()
        };
        let mut d = Destructible::new(DestructibleId(1), config, Vec3::ZERO);

        d.apply_force(400.0);
        assert_eq!(d.state, DestructibleState::Intact);

        d.apply_force(100.0); // accumulated = 500
        assert_eq!(d.state, DestructibleState::Destroying);
    }

    #[test]
    fn destructible_health_trigger_with_damage_threshold() {
        let config = DestructibleConfig {
            trigger: DestructionTrigger::Health,
            damage_threshold: 10.0,
            force_to_damage: 0.1,
            max_health: 100.0,
            ..Default::default()
        };
        let mut d = Destructible::new(DestructibleId(1), config, Vec3::ZERO);

        // Force below damage_threshold → no damage
        d.apply_force(5.0);
        assert_eq!(d.health, 100.0);

        // Force above threshold → damage = (force - threshold) * conversion
        d.apply_force(110.0); // damage = (110-10)*0.1 = 10.0
        assert!(
            (d.health - 90.0).abs() < 0.01,
            "Health should be ~90, got {}",
            d.health
        );
    }

    #[test]
    fn destructible_collision_trigger() {
        let config = DestructibleConfig {
            trigger: DestructionTrigger::Collision,
            ..Default::default()
        };
        let mut d = Destructible::new(DestructibleId(1), config, Vec3::ZERO);

        d.on_collision(1.0); // any collision → destroying
        assert_eq!(d.state, DestructibleState::Destroying);
    }

    #[test]
    fn destructible_health_percent() {
        let config = DestructibleConfig::default();
        let mut d = Destructible::new(DestructibleId(1), config, Vec3::ZERO);
        assert_eq!(d.health_percent(), 1.0);
        d.apply_damage(25.0);
        assert!((d.health_percent() - 0.75).abs() < 0.01);
    }

    #[test]
    fn destructible_should_spawn_debris_only_when_destroying() {
        let config = DestructibleConfig::default();
        let mut d = Destructible::new(DestructibleId(1), config, Vec3::ZERO);
        assert!(!d.should_spawn_debris());

        d.state = DestructibleState::Destroying;
        assert!(d.should_spawn_debris());

        d.complete_destruction();
        assert!(!d.should_spawn_debris());
        assert!(d.is_destroyed());
    }

    #[test]
    fn destructible_reset_frame() {
        let config = DestructibleConfig::default();
        let mut d = Destructible::new(DestructibleId(1), config, Vec3::ZERO);
        d.accumulated_force = 123.0;
        d.reset_frame();
        assert_eq!(d.accumulated_force, 0.0);
    }

    #[test]
    fn destruction_trigger_default_is_force_1000() {
        let t = DestructionTrigger::default();
        assert_eq!(t, DestructionTrigger::Force { threshold: 1000.0 });
    }

    #[test]
    fn destructible_config_defaults() {
        let c = DestructibleConfig::default();
        assert_eq!(c.max_health, 100.0);
        assert_eq!(c.damage_threshold, 10.0);
        assert_eq!(c.force_to_damage, 0.1);
        assert_eq!(c.destruction_force, 5.0);
    }
}

// ============================================================================
// P6: Gravity System (~10 estimated surviving mutants)
// ============================================================================

mod gravity_system_mutations {
    use super::*;
    use astraweave_physics::gravity::*;
    #[allow(unused_imports)]
    #[test]
    fn point_gravity_falloff_formula_quadratic() {
        let shape = GravityZoneShape::Point {
            center: Vec3::ZERO,
            radius: 100.0,
            strength: 100.0,
        };

        // At distance 50 from center (50% of radius):
        // falloff = 1.0 - (50/100).min(1.0) = 0.5
        // force = strength * falloff * falloff = 100 * 0.25 = 25
        let gravity = shape
            .get_gravity(Vec3::new(50.0, 0.0, 0.0), Vec3::ZERO)
            .unwrap();
        let magnitude = gravity.length();
        assert!(
            (magnitude - 25.0).abs() < 0.5,
            "Point gravity magnitude: got {} expected ~25",
            magnitude
        );
        // Direction should be toward center (negative X)
        assert!(gravity.x < 0.0, "Should pull toward center");
    }

    #[test]
    fn point_gravity_at_center_is_zero() {
        let shape = GravityZoneShape::Point {
            center: Vec3::ZERO,
            radius: 100.0,
            strength: 100.0,
        };

        // At center (distance < 0.001), returns ZERO
        let gravity = shape.get_gravity(Vec3::ZERO, Vec3::ZERO).unwrap();
        assert_eq!(gravity, Vec3::ZERO);
    }

    #[test]
    fn point_gravity_at_edge_is_zero() {
        let shape = GravityZoneShape::Point {
            center: Vec3::ZERO,
            radius: 10.0,
            strength: 100.0,
        };

        // At exactly at radius: falloff = 1.0 - (10/10).min(1.0) = 0.0 → force = 0
        let gravity = shape
            .get_gravity(Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO)
            .unwrap();
        assert!(
            gravity.length() < 0.01,
            "At edge, force should be ~0, got {:?}",
            gravity
        );
    }

    #[test]
    fn point_gravity_beyond_radius_clamped() {
        let shape = GravityZoneShape::Point {
            center: Vec3::ZERO,
            radius: 10.0,
            strength: 100.0,
        };

        // Beyond radius: contains() returns false → get_gravity returns None
        let gravity = shape.get_gravity(Vec3::new(20.0, 0.0, 0.0), Vec3::ZERO);
        assert!(
            gravity.is_none(),
            "Beyond radius should return None, not Some"
        );

        // At exactly the boundary (distance² == radius²), should still be inside
        let at_edge = shape.get_gravity(Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO);
        assert!(at_edge.is_some(), "At exact radius should be Some (<=)");
        // But force should be zero since falloff = (1 - 1.0)² = 0
        assert!(
            at_edge.unwrap().length() < 0.01,
            "Force at edge should be ~0"
        );
    }

    #[test]
    fn point_gravity_distance_guard_at_0001() {
        let shape = GravityZoneShape::Point {
            center: Vec3::ZERO,
            radius: 100.0,
            strength: 100.0,
        };

        // At distance 0.0005 (< 0.001), should return ZERO
        let gravity = shape
            .get_gravity(Vec3::new(0.0005, 0.0, 0.0), Vec3::ZERO)
            .unwrap();
        assert_eq!(gravity, Vec3::ZERO);
    }

    #[test]
    fn box_zone_contains_boundary() {
        let shape = GravityZoneShape::Box {
            min: Vec3::new(-5.0, -5.0, -5.0),
            max: Vec3::new(5.0, 5.0, 5.0),
        };
        assert!(shape.contains(Vec3::new(5.0, 5.0, 5.0))); // at max → inside (<=)
        assert!(shape.contains(Vec3::new(-5.0, -5.0, -5.0))); // at min → inside (>=)
        assert!(!shape.contains(Vec3::new(5.01, 0.0, 0.0))); // just outside
    }

    #[test]
    fn sphere_zone_contains_boundary() {
        let shape = GravityZoneShape::Sphere {
            center: Vec3::ZERO,
            radius: 10.0,
        };
        assert!(shape.contains(Vec3::ZERO)); // center
                                             // At exactly radius: distance_squared = 100, radius^2 = 100 → contains (<=)
        assert!(shape.contains(Vec3::new(10.0, 0.0, 0.0)));
        assert!(!shape.contains(Vec3::new(10.01, 0.0, 0.0)));
    }

    #[test]
    fn gravity_manager_zone_id_increments() {
        let mut mgr = GravityManager::new(Vec3::new(0.0, -9.81, 0.0));
        let id1 = mgr.add_zone(GravityZone::default());
        let id2 = mgr.add_zone(GravityZone::default());
        assert_eq!(id2, id1 + 1);
    }

    #[test]
    fn gravity_manager_default_is_earth() {
        let mgr = GravityManager::default();
        assert!((mgr.global_gravity.y - (-9.81)).abs() < 0.01);
    }

    #[test]
    fn body_gravity_defaults() {
        let s = BodyGravitySettings::default();
        assert_eq!(s.scale, 1.0);
        assert!(s.custom_direction.is_none());
        assert!(!s.ignore_zones);
    }

    #[test]
    fn gravity_zone_default() {
        let z = GravityZone::default();
        assert_eq!(z.id, 0);
        assert_eq!(z.gravity, Vec3::ZERO);
        assert_eq!(z.priority, 0);
        assert!(z.active);
        assert!(z.name.is_none());
    }
}

// ============================================================================
// P7: PhysicsConfig & BuoyancyData (~8 estimated surviving mutants)
// ============================================================================

mod physics_config_mutations {
    use super::*;

    #[test]
    fn physics_config_defaults() {
        let c = PhysicsConfig::default();
        assert_eq!(c.gravity, Vec3::new(0.0, -9.81, 0.0));
        assert!(!c.ccd_enabled);
        assert_eq!(c.max_ccd_substeps, 1);
        assert!((c.time_step - 1.0 / 60.0).abs() < 1e-6);
        assert_eq!(c.water_level, f32::NEG_INFINITY);
        assert_eq!(c.fluid_density, 1000.0);
    }

    #[test]
    fn is_earth_gravity_exact() {
        let c = PhysicsConfig::default();
        assert!(c.is_earth_gravity());
    }

    #[test]
    fn is_earth_gravity_threshold_01_y() {
        let mut c = PhysicsConfig::default();
        c.gravity = Vec3::new(0.0, -9.81 + 0.09, 0.0); // within 0.1
        assert!(c.is_earth_gravity());

        c.gravity = Vec3::new(0.0, -9.81 + 0.11, 0.0); // outside 0.1
        assert!(!c.is_earth_gravity());
    }

    #[test]
    fn is_earth_gravity_threshold_001_xz() {
        let mut c = PhysicsConfig::default();
        c.gravity = Vec3::new(0.009, -9.81, 0.009); // within 0.01
        assert!(c.is_earth_gravity());

        c.gravity = Vec3::new(0.011, -9.81, 0.0); // outside 0.01
        assert!(!c.is_earth_gravity());

        c.gravity = Vec3::new(0.0, -9.81, 0.011); // z outside 0.01
        assert!(!c.is_earth_gravity());
    }

    #[test]
    fn is_zero_gravity_threshold_1e6() {
        let c = PhysicsConfig {
            gravity: Vec3::ZERO,
            ..Default::default()
        };
        assert!(c.is_zero_gravity());

        let c2 = PhysicsConfig {
            gravity: Vec3::new(0.0001, 0.0, 0.0), // length_squared = 1e-8 < 1e-6
            ..Default::default()
        };
        assert!(c2.is_zero_gravity());

        let c3 = PhysicsConfig {
            gravity: Vec3::new(0.01, 0.0, 0.0), // length_squared = 1e-4 > 1e-6
            ..Default::default()
        };
        assert!(!c3.is_zero_gravity());
    }

    #[test]
    fn gravity_magnitude() {
        let c = PhysicsConfig::default();
        assert!((c.gravity_magnitude() - 9.81).abs() < 0.01);
    }

    #[test]
    fn target_fps() {
        let c = PhysicsConfig::default();
        assert!((c.target_fps() - 60.0).abs() < 0.1);
    }

    #[test]
    fn has_water_default_is_false() {
        let c = PhysicsConfig::default();
        assert!(!c.has_water()); // NEG_INFINITY is not finite
    }

    #[test]
    fn has_water_with_level() {
        let c = PhysicsConfig::default().with_water(5.0, 1025.0);
        assert!(c.has_water());
        assert_eq!(c.water_level, 5.0);
        assert_eq!(c.fluid_density, 1025.0);
    }

    #[test]
    fn buoyancy_data_force_formula() {
        let b = BuoyancyData::new(2.0, 1.0);
        // buoyancy_force = volume * fluid_density * 9.81
        let f = b.buoyancy_force(1000.0);
        let expected = 2.0 * 1000.0 * 9.81;
        assert!(
            (f - expected).abs() < 0.01,
            "Buoyancy force: got {} expected {}",
            f,
            expected
        );
    }

    #[test]
    fn buoyancy_data_drag_formula() {
        let b = BuoyancyData::new(1.0, 2.0);
        // drag_force = 0.5 * drag * velocity * velocity
        let d = b.drag_force(10.0);
        let expected = 0.5 * 2.0 * 10.0 * 10.0;
        assert!(
            (d - expected).abs() < 0.01,
            "Drag force: got {} expected {}",
            d,
            expected
        );
    }

    #[test]
    fn buoyancy_data_validity() {
        assert!(BuoyancyData::new(1.0, 0.0).is_valid());
        assert!(!BuoyancyData::new(0.0, 0.0).is_valid());
        assert!(!BuoyancyData::new(-1.0, 0.0).is_valid());
    }

    #[test]
    fn buoyancy_data_with_volume() {
        let b = BuoyancyData::with_volume(5.0);
        assert_eq!(b.volume, 5.0);
        assert_eq!(b.drag, 0.0);
    }

    #[test]
    fn physics_config_builders() {
        let c = PhysicsConfig::new()
            .with_gravity(Vec3::new(0.0, -3.72, 0.0))
            .with_ccd(4)
            .with_time_step(1.0 / 120.0);
        assert_eq!(c.gravity, Vec3::new(0.0, -3.72, 0.0));
        assert!(c.ccd_enabled);
        assert_eq!(c.max_ccd_substeps, 4);
        assert!((c.time_step - 1.0 / 120.0).abs() < 1e-6);
    }
}

// ============================================================================
// P8: Projectile & Falloff Curves (~8 estimated surviving mutants)
// ============================================================================

mod projectile_mutations {
    use super::*;
    use astraweave_physics::projectile::*;
    #[allow(unused_imports)]
    #[test]
    fn falloff_linear() {
        assert_eq!(FalloffCurve::Linear.calculate(0.0, 10.0), 1.0); // at center
        assert!((FalloffCurve::Linear.calculate(5.0, 10.0) - 0.5).abs() < 0.01); // midpoint
        assert_eq!(FalloffCurve::Linear.calculate(10.0, 10.0), 0.0); // at edge
        assert_eq!(FalloffCurve::Linear.calculate(15.0, 10.0), 0.0); // beyond
    }

    #[test]
    fn falloff_quadratic() {
        // 1 - t^2
        assert_eq!(FalloffCurve::Quadratic.calculate(0.0, 10.0), 1.0);
        let mid = FalloffCurve::Quadratic.calculate(5.0, 10.0);
        assert!((mid - 0.75).abs() < 0.01); // 1 - 0.25
        assert_eq!(FalloffCurve::Quadratic.calculate(10.0, 10.0), 0.0);
    }

    #[test]
    fn falloff_exponential() {
        // e^(-3t)
        assert_eq!(FalloffCurve::Exponential.calculate(0.0, 10.0), 1.0);
        let mid = FalloffCurve::Exponential.calculate(5.0, 10.0);
        let expected = (-1.5_f32).exp(); // t=0.5, -3*0.5=-1.5
        assert!((mid - expected).abs() < 0.01);
        assert_eq!(FalloffCurve::Exponential.calculate(10.0, 10.0), 0.0); // past edge
    }

    #[test]
    fn falloff_constant() {
        assert_eq!(FalloffCurve::Constant.calculate(0.0, 10.0), 1.0);
        assert_eq!(FalloffCurve::Constant.calculate(5.0, 10.0), 1.0);
        assert_eq!(FalloffCurve::Constant.calculate(9.99, 10.0), 1.0);
        assert_eq!(FalloffCurve::Constant.calculate(10.0, 10.0), 0.0); // at/beyond radius
    }

    #[test]
    fn falloff_zero_radius_returns_zero_at_boundary() {
        // When radius=0, distance(0) >= radius(0) is true → returns 0.0
        // The `radius <= 0.0 → 1.0` branch is unreachable because
        // `distance >= radius` check comes first (distance is always >= 0)
        assert_eq!(FalloffCurve::Linear.calculate(0.0, 0.0), 0.0);
        assert_eq!(FalloffCurve::Linear.calculate(5.0, 0.0), 0.0);
    }

    #[test]
    fn falloff_at_exact_radius_returns_zero() {
        // distance >= radius → 0.0 (tests the >= boundary)
        assert_eq!(FalloffCurve::Linear.calculate(10.0, 10.0), 0.0);
        assert_eq!(FalloffCurve::Quadratic.calculate(10.0, 10.0), 0.0);
        assert_eq!(FalloffCurve::Exponential.calculate(10.0, 10.0), 0.0);
        assert_eq!(FalloffCurve::Constant.calculate(10.0, 10.0), 0.0);
    }

    #[test]
    fn falloff_just_inside_radius_is_nonzero() {
        // distance < radius → nonzero result
        assert!(FalloffCurve::Linear.calculate(9.99, 10.0) > 0.0);
        assert!(FalloffCurve::Quadratic.calculate(9.99, 10.0) > 0.0);
        assert!(FalloffCurve::Constant.calculate(9.99, 10.0) > 0.0);
    }

    #[test]
    fn projectile_manager_spawn_and_get() {
        let mut mgr = ProjectileManager::new();
        let id = mgr.spawn(ProjectileConfig::default());
        assert!(mgr.get(id).is_some());
        assert_eq!(mgr.count(), 1);
    }

    #[test]
    fn projectile_manager_despawn() {
        let mut mgr = ProjectileManager::new();
        let id = mgr.spawn(ProjectileConfig::default());
        assert!(mgr.despawn(id));
        assert!(!mgr.despawn(id)); // already removed
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn projectile_config_defaults() {
        let c = ProjectileConfig::default();
        assert_eq!(c.kind, ProjectileKind::Kinematic);
        assert_eq!(c.gravity_scale, 1.0);
        assert_eq!(c.drag, 0.0);
        assert_eq!(c.radius, 0.05);
        assert_eq!(c.max_lifetime, 10.0);
        assert_eq!(c.max_bounces, 0);
        assert_eq!(c.restitution, 0.5);
        assert_eq!(c.penetration, 0.0);
        assert!(c.owner.is_none());
        assert_eq!(c.user_data, 0);
    }

    #[test]
    fn explosion_config_defaults() {
        let e = ExplosionConfig::default();
        assert_eq!(e.radius, 5.0);
        assert_eq!(e.force, 1000.0);
        assert_eq!(e.upward_bias, 0.3);
        assert_eq!(e.falloff, FalloffCurve::Linear);
    }

    #[test]
    fn explosion_calculation_within_radius() {
        let mgr = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::ZERO,
            radius: 10.0,
            force: 1000.0,
            falloff: FalloffCurve::Linear,
            upward_bias: 0.0,
        };

        let bodies = vec![(1u64, Vec3::new(5.0, 0.0, 0.0))];
        let results = mgr.calculate_explosion(&config, bodies);

        assert_eq!(results.len(), 1);
        let r = &results[0];
        assert_eq!(r.body_id, 1);
        // Distance 5.0, linear falloff = 1 - 5/10 = 0.5
        assert!((r.falloff_multiplier - 0.5).abs() < 0.01);
        // Force = 1000 * 0.5 = 500
        assert!((r.impulse.length() - 500.0).abs() < 10.0);
    }

    #[test]
    fn explosion_calculation_beyond_radius() {
        let mgr = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::ZERO,
            radius: 5.0,
            force: 1000.0,
            falloff: FalloffCurve::Linear,
            upward_bias: 0.0,
        };

        let bodies = vec![(1u64, Vec3::new(10.0, 0.0, 0.0))];
        let results = mgr.calculate_explosion(&config, bodies);
        assert_eq!(results.len(), 0, "Beyond radius should not be affected");
    }

    #[test]
    fn explosion_upward_bias() {
        let mgr = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::ZERO,
            radius: 10.0,
            force: 1000.0,
            falloff: FalloffCurve::Constant,
            upward_bias: 1.0, // pure upward
        };

        let bodies = vec![(1u64, Vec3::new(5.0, 0.0, 0.0))];
        let results = mgr.calculate_explosion(&config, bodies);

        let r = &results[0];
        // With upward_bias=1.0: biased_dir = (radial*(1-1) + Y*1).normalize() = Y
        assert!(
            r.impulse.y > r.impulse.x.abs(),
            "Upward bias should make Y dominant: {:?}",
            r.impulse
        );
    }

    #[test]
    fn projectile_manager_default_gravity() {
        let mgr = ProjectileManager::new();
        assert_eq!(mgr.gravity, Vec3::new(0.0, -9.81, 0.0));
        assert_eq!(mgr.wind, Vec3::ZERO);
    }

    #[test]
    fn projectile_lifetime_despawn() {
        let mut mgr = ProjectileManager::new();
        let id = mgr.spawn(ProjectileConfig {
            max_lifetime: 0.01,
            velocity: Vec3::new(1.0, 0.0, 0.0),
            ..Default::default()
        });

        // Update with dt larger than lifetime
        let no_hit = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };
        mgr.update(0.02, no_hit);

        // Should have been despawned
        assert!(mgr.get(id).is_none());
    }
}

// ============================================================================
// Misc: ActorKind, DebugLine, JointType
// ============================================================================

mod misc_types_mutations {
    use super::*;

    #[test]
    fn actor_kind_names() {
        assert_eq!(ActorKind::Static.name(), "Static");
        assert_eq!(ActorKind::Dynamic.name(), "Dynamic");
        assert_eq!(ActorKind::Character.name(), "Character");
        assert_eq!(ActorKind::Other.name(), "Other");
    }

    #[test]
    fn actor_kind_predicates() {
        // Positive assertions
        assert!(ActorKind::Static.is_static());
        assert!(ActorKind::Dynamic.is_dynamic());
        assert!(ActorKind::Character.is_character());
        assert!(ActorKind::Other.is_other());

        // Negative assertions for is_static (kills mutation "replace is_static with true")
        assert!(!ActorKind::Dynamic.is_static());
        assert!(!ActorKind::Character.is_static());
        assert!(!ActorKind::Other.is_static());

        // Negative assertions for is_dynamic (kills mutation "replace is_dynamic with true")
        assert!(!ActorKind::Static.is_dynamic());
        assert!(!ActorKind::Character.is_dynamic());
        assert!(!ActorKind::Other.is_dynamic());

        // Negative assertions for is_character (kills mutation "replace is_character with true")
        assert!(!ActorKind::Static.is_character());
        assert!(!ActorKind::Dynamic.is_character());
        assert!(!ActorKind::Other.is_character());

        // Negative assertions for is_other (kills mutation "replace is_other with true")
        assert!(!ActorKind::Static.is_other());
        assert!(!ActorKind::Dynamic.is_other());
        assert!(!ActorKind::Character.is_other());
    }

    #[test]
    fn actor_kind_movable() {
        assert!(!ActorKind::Static.is_movable());
        assert!(ActorKind::Dynamic.is_movable());
        assert!(ActorKind::Character.is_movable());
        assert!(!ActorKind::Other.is_movable());
    }

    #[test]
    fn actor_kind_all() {
        let all = ActorKind::all();
        assert_eq!(all.len(), 4);
        assert_eq!(all[0], ActorKind::Static);
        assert_eq!(all[1], ActorKind::Dynamic);
        assert_eq!(all[2], ActorKind::Character);
        assert_eq!(all[3], ActorKind::Other);
    }

    #[test]
    fn debug_line_length() {
        let line = DebugLine::new([0.0, 0.0, 0.0], [3.0, 4.0, 0.0], [1.0, 0.0, 0.0]);
        assert!((line.length() - 5.0).abs() < 1e-4); // 3-4-5 triangle
    }

    /// Test that catches mutation "replace - with + in dy calculation"
    /// By using non-zero start Y coordinate, we can detect the sign error.
    /// start=[1,2,3], end=[4,6,3] → dx=3, dy=4, dz=0 → length=5
    /// With mutation: dy=6+2=8 → length=sqrt(9+64+0)=8.54 ≠ 5
    #[test]
    fn debug_line_length_nonzero_start_y() {
        let line = DebugLine::new([1.0, 2.0, 3.0], [4.0, 6.0, 3.0], [1.0, 0.0, 0.0]);
        // dx = 4-1 = 3, dy = 6-2 = 4, dz = 3-3 = 0 → length = 5.0
        assert!(
            (line.length() - 5.0).abs() < 1e-4,
            "length={}, expected 5.0",
            line.length()
        );
    }

    /// Additional mutation killer: negative Y direction
    /// start=[0,10,0], end=[0,2,0] → dy=-8, length=8
    /// With mutation: dy=2+10=12 → length=12 ≠ 8
    #[test]
    fn debug_line_length_negative_y_direction() {
        let line = DebugLine::new([0.0, 10.0, 0.0], [0.0, 2.0, 0.0], [1.0, 0.0, 0.0]);
        // dy = 2 - 10 = -8, length = 8.0
        assert!(
            (line.length() - 8.0).abs() < 1e-4,
            "length={}, expected 8.0",
            line.length()
        );
    }

    /// Test with all non-zero coordinates to catch any axis mutation
    #[test]
    fn debug_line_length_all_nonzero() {
        let line = DebugLine::new([1.0, 2.0, 3.0], [2.0, 4.0, 5.0], [1.0, 0.0, 0.0]);
        // dx = 1, dy = 2, dz = 2 → length = sqrt(1+4+4) = 3.0
        assert!(
            (line.length() - 3.0).abs() < 1e-4,
            "length={}, expected 3.0",
            line.length()
        );
    }

    /// Mutation killer: catches `*` → `+` mutations in the squared distance formula.
    /// Uses deltas that are NOT 0 or 2 (where x*x == x+x), ensuring all square terms
    /// give different results when mutated.
    /// dx=3, dy=4, dz=5: correct=sqrt(9+16+25)=sqrt(50)=7.07
    /// With dx*dx→dx+dx: sqrt(6+16+25)=sqrt(47)=6.86 ≠ 7.07
    /// With dy*dy→dy+dy: sqrt(9+8+25)=sqrt(42)=6.48 ≠ 7.07
    /// With dz*dz→dz+dz: sqrt(9+16+10)=sqrt(35)=5.92 ≠ 7.07
    #[test]
    fn debug_line_length_multiply_mutation_killer() {
        let line = DebugLine::new([0.0, 0.0, 0.0], [3.0, 4.0, 5.0], [1.0, 0.0, 0.0]);
        // dx=3, dy=4, dz=5 → length = sqrt(9+16+25) = sqrt(50) = 7.0710678...
        let expected = 50.0_f32.sqrt();
        assert!(
            (line.length() - expected).abs() < 1e-4,
            "length={}, expected {}",
            line.length(),
            expected
        );
    }

    #[test]
    fn debug_line_length_squared() {
        let line = DebugLine::new([0.0, 0.0, 0.0], [3.0, 4.0, 0.0], [1.0, 0.0, 0.0]);
        assert!((line.length_squared() - 25.0).abs() < 1e-4);
    }

    /// Mutation killer: catches `+` → `-` and `*` → `+` mutations in length_squared.
    /// Uses deltas that are NOT 0 or 2, ensuring all squared terms differ when mutated.
    /// dx=3, dy=4, dz=5: correct = 9+16+25 = 50
    /// With first +→-: 9-16+25 = 18 ≠ 50
    /// With dx*dx→dx+dx: 6+16+25 = 47 ≠ 50
    #[test]
    fn debug_line_length_squared_mutation_killer() {
        let line = DebugLine::new([0.0, 0.0, 0.0], [3.0, 4.0, 5.0], [1.0, 0.0, 0.0]);
        // dx=3, dy=4, dz=5 → length_squared = 9+16+25 = 50
        assert!(
            (line.length_squared() - 50.0).abs() < 1e-4,
            "length_squared={}, expected 50.0",
            line.length_squared()
        );
    }

    /// Additional tests for length_squared to catch all arithmetic mutations
    #[test]
    fn debug_line_length_squared_all_axis_mutations() {
        // Use different values on all axes to catch any axis-specific mutation
        let line = DebugLine::new([1.0, 2.0, 3.0], [4.0, 6.0, 8.0], [1.0, 0.0, 0.0]);
        // dx=3, dy=4, dz=5 → length_squared = 9+16+25 = 50
        assert!(
            (line.length_squared() - 50.0).abs() < 1e-4,
            "length_squared={}, expected 50.0",
            line.length_squared()
        );
    }

    #[test]
    fn debug_line_midpoint() {
        let line = DebugLine::new([0.0, 0.0, 0.0], [10.0, 20.0, 30.0], [1.0, 1.0, 1.0]);
        let mid = line.midpoint();
        assert!((mid[0] - 5.0).abs() < 1e-4);
        assert!((mid[1] - 10.0).abs() < 1e-4);
        assert!((mid[2] - 15.0).abs() < 1e-4);
    }

    #[test]
    fn debug_line_direction() {
        let line = DebugLine::new([1.0, 2.0, 3.0], [4.0, 6.0, 9.0], [1.0, 0.0, 0.0]);
        let dir = line.direction();
        assert!((dir[0] - 3.0).abs() < 1e-4);
        assert!((dir[1] - 4.0).abs() < 1e-4);
        assert!((dir[2] - 6.0).abs() < 1e-4);
    }

    #[test]
    fn debug_line_degenerate() {
        let line = DebugLine::new([1.0, 2.0, 3.0], [1.0, 2.0, 3.0], [1.0, 0.0, 0.0]);
        assert!(line.is_degenerate());

        let line2 = DebugLine::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert!(!line2.is_degenerate());
    }

    /// Mutation killer: tests is_degenerate boundary at 1e-10 threshold.
    /// NOTE: Due to f32 precision, (1e-5)^2 lands 1 ULP below 1e-10 (0x2edbe6fe vs 0x2edbe6ff),
    /// and the next f32 up squares to 2 ULPs above. The threshold value 1e-10 falls in a gap
    /// between consecutive f32 squares, making `<` vs `<=` observationally equivalent.
    /// We test both sides of the gap to catch `<` → `>`, `<` → `==`, and threshold mutations.
    #[test]
    fn debug_line_degenerate_boundary() {
        let dx = 1e-5_f32;
        // f32 precision: (1e-5)^2 = 0x2edbe6fe < 1e-10 = 0x2edbe6ff
        let line = DebugLine::new([0.0, 0.0, 0.0], [dx, 0.0, 0.0], [1.0, 0.0, 0.0]);
        let len_sq = line.length_squared();
        assert!(
            len_sq < 1e-10_f32,
            "f32 precision: (1e-5)^2 = {} should be < 1e-10 (1 ULP gap)",
            len_sq
        );
        assert!(
            line.is_degenerate(),
            "length_squared < threshold → degenerate"
        );

        // Next f32 value up from dx: its square jumps above threshold
        let dx_up = f32::from_bits(dx.to_bits() + 1);
        let line_up = DebugLine::new([0.0, 0.0, 0.0], [dx_up, 0.0, 0.0], [1.0, 0.0, 0.0]);
        let len_sq_up = line_up.length_squared();
        assert!(
            len_sq_up > 1e-10_f32,
            "1 ULP above sqrt(threshold): {} should be > 1e-10",
            len_sq_up
        );
        assert!(
            !line_up.is_degenerate(),
            "length_squared > threshold → NOT degenerate"
        );

        // 0.9× factor: well below threshold → degenerate (0.81 * 1e-10 ≈ 8.1e-11)
        let smaller = DebugLine::new([0.0, 0.0, 0.0], [dx * 0.9, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert!(smaller.is_degenerate(), "0.9x should be degenerate");

        // 1.1× factor: well above threshold → NOT degenerate (1.21 * 1e-10 ≈ 1.21e-10)
        let larger = DebugLine::new([0.0, 0.0, 0.0], [dx * 1.1, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert!(!larger.is_degenerate(), "1.1x should NOT be degenerate");
    }

    #[test]
    fn debug_line_color_helpers() {
        let r = DebugLine::red([0.0; 3], [1.0; 3]);
        assert_eq!(r.color, [1.0, 0.0, 0.0]);

        let g = DebugLine::green([0.0; 3], [1.0; 3]);
        assert_eq!(g.color, [0.0, 1.0, 0.0]);

        let b = DebugLine::blue([0.0; 3], [1.0; 3]);
        assert_eq!(b.color, [0.0, 0.0, 1.0]);

        let w = DebugLine::white([0.0; 3], [1.0; 3]);
        assert_eq!(w.color, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn debug_line_from_vec3() {
        let line = DebugLine::from_vec3(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(4.0, 5.0, 6.0),
            [1.0, 0.0, 0.0],
        );
        assert_eq!(line.start, [1.0, 2.0, 3.0]);
        assert_eq!(line.end, [4.0, 5.0, 6.0]);
    }

    #[test]
    fn joint_type_predicates() {
        assert!(JointType::Fixed.is_fixed());
        assert!(JointType::revolute_y().is_revolute());
        assert!(JointType::prismatic_y().is_prismatic());
        assert!(JointType::Spherical.is_spherical());
    }

    #[test]
    fn joint_type_degrees_of_freedom() {
        assert_eq!(JointType::Fixed.degrees_of_freedom(), 0);
        assert_eq!(JointType::revolute_y().degrees_of_freedom(), 1);
        assert_eq!(JointType::prismatic_y().degrees_of_freedom(), 1);
        assert_eq!(JointType::Spherical.degrees_of_freedom(), 3);
    }

    #[test]
    fn joint_type_axes() {
        assert!(JointType::Fixed.axis().is_none());
        assert_eq!(JointType::revolute_y().axis(), Some(Vec3::Y));
        assert_eq!(JointType::revolute_x().axis(), Some(Vec3::X));
        assert_eq!(JointType::revolute_z().axis(), Some(Vec3::Z));
        assert_eq!(JointType::prismatic_y().axis(), Some(Vec3::Y));
        assert!(JointType::Spherical.axis().is_none());
    }

    #[test]
    fn joint_type_has_limits() {
        assert!(!JointType::Fixed.has_limits());
        assert!(!JointType::revolute_y().has_limits());
        let limited = JointType::Revolute {
            axis: Vec3::Y,
            limits: Some((-1.0, 1.0)),
        };
        assert!(limited.has_limits());
        assert_eq!(limited.limits(), Some((-1.0, 1.0)));
    }

    #[test]
    fn joint_type_rotational_vs_linear() {
        assert!(JointType::revolute_y().is_rotational());
        assert!(JointType::Spherical.is_rotational());
        assert!(!JointType::Fixed.is_rotational());
        assert!(!JointType::prismatic_y().is_rotational());

        assert!(JointType::prismatic_y().is_linear());
        assert!(!JointType::revolute_y().is_linear());
    }

    #[test]
    fn joint_id_validity() {
        assert!(!JointId::invalid().is_valid());
        assert!(JointId::new(1).is_valid());
        assert_eq!(JointId::invalid().raw(), 0);
        assert_eq!(JointId::new(42).raw(), 42);
    }

    #[test]
    fn char_state_names() {
        assert_eq!(CharState::Grounded.name(), "Grounded");
        assert!(CharState::Grounded.is_grounded());
        assert_eq!(CharState::all().len(), 1);
    }

    #[test]
    fn layers_bitflags() {
        let default = Layers::DEFAULT;
        let character = Layers::CHARACTER;
        assert_ne!(default, character);
        let combined = default | character;
        assert!(combined.contains(Layers::DEFAULT));
        assert!(combined.contains(Layers::CHARACTER));
    }
}
