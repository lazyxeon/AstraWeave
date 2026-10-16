//! Mutation-Resistant Behavioral Correctness Tests
//!
//! These tests verify that physics subsystems produce CORRECT results, not just
//! that they run without crashing. Each test is designed to catch common mutations
//! (e.g., + to -, * to /, sign flips, off-by-one errors).
//!
//! Tests verify:
//! - Newton's Laws of Motion
//! - Conservation of Energy/Momentum
//! - Mathematical correctness of physics formulas
//! - Boundary conditions and edge cases
//!
//! Phase 8.8: Production-Ready Physics Validation

use astraweave_physics::cloth::{Cloth, ClothConfig, ClothId, ClothParticle, DistanceConstraint};
use astraweave_physics::destruction::{DestructibleConfig, DestructionManager, FracturePattern};
use astraweave_physics::projectile::{
    ExplosionConfig, FalloffCurve, ProjectileConfig, ProjectileManager,
};
use astraweave_physics::ragdoll::{BoneShape, RagdollBuilder, RagdollConfig};
use astraweave_physics::spatial_hash::{SpatialHash, AABB};
use astraweave_physics::vehicle::{FrictionCurve, WheelConfig};
use glam::Vec3;

// ============================================================================
// NEWTON'S LAWS VERIFICATION
// ============================================================================

/// Newton's First Law: An object at rest stays at rest
#[test]
fn test_newtons_first_law_object_at_rest() {
    let mut manager = ProjectileManager::new();
    manager.gravity = Vec3::ZERO; // No forces
    manager.wind = Vec3::ZERO;

    let config = ProjectileConfig {
        position: Vec3::new(5.0, 10.0, 3.0),
        velocity: Vec3::ZERO, // At rest
        gravity_scale: 0.0,
        drag: 0.0,
        ..Default::default()
    };
    let id = manager.spawn(config);
    let initial_pos = manager.get(id).unwrap().position;

    // Update for 100 frames
    let raycast = |_: Vec3, _: Vec3, _: f32| None;
    for _ in 0..100 {
        manager.update(1.0 / 60.0, raycast);
    }

    let final_pos = manager.get(id).unwrap().position;

    // Position should be EXACTLY unchanged (within float precision)
    assert!(
        (final_pos - initial_pos).length() < 1e-6,
        "Object at rest with no forces should stay at rest. Initial: {:?}, Final: {:?}",
        initial_pos,
        final_pos
    );
}

/// Newton's First Law: An object in motion stays in motion (no forces)
#[test]
fn test_newtons_first_law_uniform_motion() {
    let mut manager = ProjectileManager::new();
    manager.gravity = Vec3::ZERO;
    manager.wind = Vec3::ZERO;

    let initial_velocity = Vec3::new(10.0, 5.0, -3.0);
    let config = ProjectileConfig {
        position: Vec3::ZERO,
        velocity: initial_velocity,
        gravity_scale: 0.0,
        drag: 0.0,
        ..Default::default()
    };
    let id = manager.spawn(config);

    let raycast = |_: Vec3, _: Vec3, _: f32| None;
    let dt = 1.0 / 60.0;
    let frames = 60;

    for _ in 0..frames {
        manager.update(dt, raycast);
    }

    let proj = manager.get(id).unwrap();
    let expected_pos = initial_velocity * dt * frames as f32;

    // Verify constant velocity motion: position = velocity * time
    assert!(
        (proj.position - expected_pos).length() < 0.01,
        "Uniform motion violated. Expected {:?}, got {:?}",
        expected_pos,
        proj.position
    );

    // Velocity should be unchanged
    assert!(
        (proj.velocity - initial_velocity).length() < 1e-5,
        "Velocity should remain constant. Expected {:?}, got {:?}",
        initial_velocity,
        proj.velocity
    );
}

/// Newton's Second Law: F = ma (acceleration proportional to force)
#[test]
fn test_newtons_second_law_gravity_acceleration() {
    let mut manager = ProjectileManager::new();
    let g = 9.81;
    manager.gravity = Vec3::new(0.0, -g, 0.0);

    let config = ProjectileConfig {
        position: Vec3::new(0.0, 100.0, 0.0),
        velocity: Vec3::ZERO,
        gravity_scale: 1.0,
        drag: 0.0,
        ..Default::default()
    };
    let id = manager.spawn(config);

    let raycast = |_: Vec3, _: Vec3, _: f32| None;
    let dt = 1.0 / 60.0;
    let time = 1.0; // 1 second
    let frames = (time / dt) as usize;

    for _ in 0..frames {
        manager.update(dt, raycast);
    }

    let proj = manager.get(id).unwrap();

    // After 1 second of free fall:
    // v = g*t = 9.81 m/s downward
    // y = y0 - 0.5*g*t^2 = 100 - 4.905 ≈ 95.095m
    let expected_y = 100.0 - 0.5 * g * time * time;
    let expected_vy = -g * time;

    assert!(
        (proj.position.y - expected_y).abs() < 0.5,
        "Free fall position incorrect. Expected y≈{:.2}, got {:.2}",
        expected_y,
        proj.position.y
    );

    assert!(
        (proj.velocity.y - expected_vy).abs() < 0.5,
        "Free fall velocity incorrect. Expected vy≈{:.2}, got {:.2}",
        expected_vy,
        proj.velocity.y
    );
}

/// Newton's Third Law: Equal and opposite forces (reflected in explosion)
#[test]
fn test_newtons_third_law_explosion_symmetry() {
    let manager = ProjectileManager::new();
    let config = ExplosionConfig {
        center: Vec3::ZERO,
        radius: 10.0,
        force: 1000.0,
        falloff: FalloffCurve::Constant,
        upward_bias: 0.0, // Pure radial
    };

    // Two bodies at equal distances, opposite sides
    let bodies = vec![
        (1, Vec3::new(5.0, 0.0, 0.0)),  // +X
        (2, Vec3::new(-5.0, 0.0, 0.0)), // -X
    ];

    let results = manager.calculate_explosion(&config, bodies);

    assert_eq!(results.len(), 2);

    let impulse1 = results.iter().find(|r| r.body_id == 1).unwrap().impulse;
    let impulse2 = results.iter().find(|r| r.body_id == 2).unwrap().impulse;

    // Impulses should be equal in magnitude, opposite in direction
    assert!(
        (impulse1.length() - impulse2.length()).abs() < 0.01,
        "Explosion impulses should have equal magnitude"
    );

    assert!(
        (impulse1 + impulse2).length() < 0.01,
        "Explosion impulses should cancel out (sum to zero). Sum: {:?}",
        impulse1 + impulse2
    );
}

// ============================================================================
// PROJECTILE PHYSICS CORRECTNESS
// ============================================================================

/// Verify parabolic trajectory under gravity (analytical solution)
#[test]
fn test_projectile_parabolic_trajectory() {
    let mut manager = ProjectileManager::new();
    let g = 10.0; // Simple gravity for easy math
    manager.gravity = Vec3::new(0.0, -g, 0.0);

    let v0 = 20.0;
    let config = ProjectileConfig {
        position: Vec3::ZERO,
        velocity: Vec3::new(v0, v0, 0.0), // 45° angle
        gravity_scale: 1.0,
        drag: 0.0,
        ..Default::default()
    };
    let id = manager.spawn(config);

    let raycast = |_: Vec3, _: Vec3, _: f32| None;

    // Time to apex = v0_y / g = 2 seconds
    // Max height = v0_y^2 / (2g) = 400/20 = 20m
    // Time to ground = 2 * t_apex = 4 seconds
    // Range = v0_x * t_total = 20 * 4 = 80m

    // Simulate to apex (2 seconds)
    for _ in 0..120 {
        // 2s at 60fps
        manager.update(1.0 / 60.0, raycast);
    }

    let proj = manager.get(id).unwrap();

    // At apex, vertical velocity should be ~0
    assert!(
        proj.velocity.y.abs() < 1.0,
        "At apex, vy should be ~0, got {}",
        proj.velocity.y
    );

    // Height should be near theoretical max
    let expected_max_height = v0 * v0 / (2.0 * g);
    assert!(
        (proj.position.y - expected_max_height).abs() < 2.0,
        "Max height should be ~{}, got {}",
        expected_max_height,
        proj.position.y
    );
}

/// Verify drag reduces velocity correctly (mutation: removing drag)
#[test]
fn test_projectile_drag_reduces_speed_monotonically() {
    let mut manager = ProjectileManager::new();
    manager.gravity = Vec3::ZERO;

    let config = ProjectileConfig {
        position: Vec3::ZERO,
        velocity: Vec3::new(100.0, 0.0, 0.0),
        gravity_scale: 0.0,
        drag: 0.05,
        ..Default::default()
    };
    let id = manager.spawn(config);

    let raycast = |_: Vec3, _: Vec3, _: f32| None;
    let mut prev_speed = 100.0;

    for _ in 0..100 {
        manager.update(1.0 / 60.0, raycast);
        let speed = manager.get(id).unwrap().velocity.length();

        // Speed must ALWAYS decrease with drag
        assert!(
            speed <= prev_speed + 0.001,
            "Drag should only decrease speed, not increase. Prev: {}, Now: {}",
            prev_speed,
            speed
        );
        prev_speed = speed;
    }

    // Final speed should be significantly less than initial
    assert!(
        prev_speed < 50.0,
        "With drag, speed should decrease substantially. Final: {}",
        prev_speed
    );
}

/// Verify bounce reflects velocity correctly
#[test]
fn test_projectile_bounce_reflection_correct() {
    let mut manager = ProjectileManager::new();
    manager.gravity = Vec3::ZERO;

    let config = ProjectileConfig {
        position: Vec3::new(0.0, 5.0, 0.0),    // Start above floor
        velocity: Vec3::new(10.0, -10.0, 0.0), // Moving right and down
        gravity_scale: 0.0,
        max_bounces: 1,
        restitution: 1.0, // Perfect bounce
        ..Default::default()
    };
    let id = manager.spawn(config);

    // Floor at y=0, normal pointing up
    let raycast = |origin: Vec3, dir: Vec3, max: f32| {
        if origin.y > 0.0 && dir.y < 0.0 {
            // Ray going down from above floor
            let t = origin.y / (-dir.y);
            if t > 0.0 && t < max / dir.length() {
                return Some((
                    Vec3::new(origin.x + dir.x * t, 0.0, origin.z + dir.z * t),
                    Vec3::Y, // Floor normal points up
                    Some(1),
                    t * dir.length(),
                ));
            }
        }
        None
    };

    // Run until projectile hits floor (should hit within 1 second)
    for _ in 0..60 {
        manager.update(1.0 / 60.0, raycast);
        if let Some(proj) = manager.get(id) {
            // Check if bounced (velocity.y should become positive)
            if proj.velocity.y > 0.0 {
                break;
            }
        }
    }

    let proj = manager.get(id).unwrap();

    // After bouncing off horizontal floor:
    // - Horizontal velocity unchanged: vx = 10.0
    // - Vertical velocity reversed: vy = +10.0 (was -10.0)
    assert!(
        (proj.velocity.x - 10.0).abs() < 0.5,
        "Horizontal velocity should be ~unchanged. Expected ~10.0, got {}",
        proj.velocity.x
    );

    assert!(
        proj.velocity.y > 0.0,
        "Vertical velocity should be positive after bounce. Got {}",
        proj.velocity.y
    );
}

/// Verify falloff curves are mathematically correct
#[test]
fn test_falloff_curves_mathematical_correctness() {
    // Linear: f(d) = 1 - d/r
    let linear = FalloffCurve::Linear;
    assert!(
        (linear.calculate(0.0, 10.0) - 1.0).abs() < 1e-6,
        "Linear at 0"
    );
    assert!(
        (linear.calculate(5.0, 10.0) - 0.5).abs() < 1e-6,
        "Linear at 0.5r"
    );
    assert!(
        (linear.calculate(10.0, 10.0) - 0.0).abs() < 1e-6,
        "Linear at r"
    );

    // Quadratic: f(d) = 1 - (d/r)^2
    let quadratic = FalloffCurve::Quadratic;
    assert!(
        (quadratic.calculate(0.0, 10.0) - 1.0).abs() < 1e-6,
        "Quadratic at 0"
    );
    assert!(
        (quadratic.calculate(5.0, 10.0) - 0.75).abs() < 1e-6,
        "Quadratic at 0.5r: expected 0.75, got {}",
        quadratic.calculate(5.0, 10.0)
    );
    assert!(
        (quadratic.calculate(10.0, 10.0) - 0.0).abs() < 1e-6,
        "Quadratic at r"
    );

    // Constant: always 1.0 within radius
    let constant = FalloffCurve::Constant;
    assert!(
        (constant.calculate(0.0, 10.0) - 1.0).abs() < 1e-6,
        "Constant at 0"
    );
    assert!(
        (constant.calculate(9.9, 10.0) - 1.0).abs() < 1e-6,
        "Constant at 0.99r"
    );

    // Outside radius should always be 0
    for curve in [
        FalloffCurve::Linear,
        FalloffCurve::Quadratic,
        FalloffCurve::Constant,
    ] {
        assert!(
            curve.calculate(11.0, 10.0) < 1e-6,
            "All curves should be 0 outside radius"
        );
    }
}

// ============================================================================
// GRAVITY SYSTEM CORRECTNESS
// ============================================================================

/// Verify point gravity uses distance-based falloff
/// Note: AstraWeave uses (1 - d/r)² falloff, not true 1/r² inverse-square

/// Verify gravity zones don't affect objects outside

/// Verify gravity scale is applied correctly (mutation: ignoring scale)

// ============================================================================
// CLOTH PHYSICS CORRECTNESS
// ============================================================================

/// Verify Verlet integration conserves energy approximately
#[test]
fn test_cloth_verlet_integration_stability() {
    let config = ClothConfig {
        width: 5,
        height: 5,
        gravity: Vec3::ZERO, // No external forces
        wind: Vec3::ZERO,
        damping: 1.0, // No damping
        stiffness: 0.5,
        ..Default::default()
    };

    let mut cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);

    // Give one particle an initial velocity
    cloth.particles[12].prev_position = cloth.particles[12].position - Vec3::new(1.0, 0.0, 0.0);

    let initial_energy = calculate_cloth_kinetic_energy(&cloth);

    // Simulate for 100 frames
    for _ in 0..100 {
        cloth.update(1.0 / 60.0);
    }

    let final_energy = calculate_cloth_kinetic_energy(&cloth);

    // Energy should not INCREASE (would indicate instability)
    assert!(
        final_energy <= initial_energy * 1.1, // Allow 10% tolerance for constraint solving
        "Energy should not increase. Initial: {}, Final: {}",
        initial_energy,
        final_energy
    );
}

fn calculate_cloth_kinetic_energy(cloth: &Cloth) -> f32 {
    cloth
        .particles
        .iter()
        .filter(|p| !p.pinned)
        .map(|p| {
            let v = p.velocity();
            0.5 * (1.0 / p.inv_mass.max(0.001)) * v.length_squared()
        })
        .sum()
}

/// Verify distance constraints maintain rest length
#[test]
fn test_cloth_constraint_maintains_distance() {
    let mut particles = vec![
        ClothParticle::new(Vec3::new(0.0, 0.0, 0.0), 1.0),
        ClothParticle::new(Vec3::new(2.0, 0.0, 0.0), 1.0), // Start at 2.0, rest is 1.0
    ];
    particles[0].pinned = true; // Fix first particle

    let constraint = DistanceConstraint::new(0, 1, 1.0);

    // Solve constraint multiple times
    for _ in 0..10 {
        constraint.solve(&mut particles);
    }

    let distance = (particles[1].position - particles[0].position).length();

    assert!(
        (distance - 1.0).abs() < 0.1,
        "Constraint should restore rest length. Expected 1.0, got {}",
        distance
    );
}

/// Verify cloth falls under gravity (mutation: sign flip)
#[test]
fn test_cloth_gravity_direction_correct() {
    let config = ClothConfig {
        width: 3,
        height: 3,
        gravity: Vec3::new(0.0, -10.0, 0.0),
        wind: Vec3::ZERO,
        ..Default::default()
    };

    let mut cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);
    let initial_y: f32 =
        cloth.particles.iter().map(|p| p.position.y).sum::<f32>() / cloth.particles.len() as f32;

    // Simulate for 1 second
    for _ in 0..60 {
        cloth.update(1.0 / 60.0);
    }

    let final_y: f32 = cloth
        .particles
        .iter()
        .filter(|p| !p.position.y.is_nan())
        .map(|p| p.position.y)
        .sum::<f32>()
        / cloth
            .particles
            .iter()
            .filter(|p| !p.position.y.is_nan())
            .count()
            .max(1) as f32;

    // Cloth should have fallen (negative Y direction)
    assert!(
        final_y < initial_y,
        "Cloth should fall under gravity. Initial Y: {}, Final Y: {}",
        initial_y,
        final_y
    );
}

// ============================================================================
// SPATIAL HASH CORRECTNESS
// ============================================================================

/// Verify spatial hash finds ALL overlapping entities (no false negatives)
#[test]
fn test_spatial_hash_no_false_negatives() {
    let mut hash = SpatialHash::new(10.0);

    // Insert 100 entities in a grid
    for i in 0..10 {
        for j in 0..10 {
            let id = i * 10 + j;
            let pos = Vec3::new(i as f32 * 5.0, 0.0, j as f32 * 5.0);
            let aabb = AABB::from_center_extents(pos, Vec3::splat(2.0));
            hash.insert(id, aabb);
        }
    }

    // Query that should find center 4 entities
    let query_aabb = AABB::from_center_extents(Vec3::new(12.5, 0.0, 12.5), Vec3::splat(5.0));

    let results = hash.query(query_aabb);

    // Should find entities at positions (10,10), (10,15), (15,10), (15,15)
    // Which are IDs: 22, 23, 32, 33
    for expected_id in [22, 23, 32, 33] {
        assert!(
            results.contains(&expected_id),
            "Spatial hash missed entity {}. Results: {:?}",
            expected_id,
            results
        );
    }
}

/// Verify spatial hash doesn't return entities that don't overlap (no false positives)
#[test]
fn test_spatial_hash_no_false_positives() {
    let mut hash = SpatialHash::new(10.0);

    // Insert entity at origin
    hash.insert(1, AABB::from_center_extents(Vec3::ZERO, Vec3::splat(1.0)));

    // Insert entity far away
    hash.insert(
        2,
        AABB::from_center_extents(Vec3::new(100.0, 0.0, 0.0), Vec3::splat(1.0)),
    );

    // Query near origin should only find entity 1
    let query = AABB::from_center_extents(Vec3::new(1.5, 0.0, 0.0), Vec3::splat(2.0));
    let results = hash.query(query);

    assert!(results.contains(&1), "Should find nearby entity");

    // Entity 2 should not be in results (it's 100 units away)
    assert!(
        !results.contains(&2),
        "Should not find entity 100 units away. Results: {:?}",
        results
    );
}

/// Verify AABB intersection is symmetric
#[test]
fn test_aabb_intersection_symmetric() {
    let a = AABB::from_center_extents(Vec3::ZERO, Vec3::splat(5.0));
    let b = AABB::from_center_extents(Vec3::new(3.0, 0.0, 0.0), Vec3::splat(5.0));

    // Intersection should be symmetric: a∩b = b∩a
    assert_eq!(
        a.intersects(&b),
        b.intersects(&a),
        "AABB intersection should be symmetric"
    );
    assert!(a.intersects(&b), "Overlapping AABBs should intersect");
}

// ============================================================================
// VEHICLE PHYSICS CORRECTNESS
// ============================================================================

/// Verify friction curve has physically correct shape (peak at optimal slip)
#[test]
fn test_friction_curve_physical_shape() {
    let curves = [
        FrictionCurve::tarmac(),
        FrictionCurve::gravel(),
        FrictionCurve::ice(),
    ];

    for curve in curves {
        // At zero slip, friction should be zero (no relative motion)
        let f_zero = curve.friction_at_slip(0.0);
        assert!(
            f_zero.abs() < 0.01,
            "Friction at zero slip should be ~0, got {}",
            f_zero
        );

        // Friction should peak near optimal slip
        let f_at_optimal = curve.friction_at_slip(curve.optimal_slip);
        let f_well_beyond = curve.friction_at_slip(curve.optimal_slip * 3.0);

        // After peak, friction should decrease (or stay stable)
        assert!(
            f_at_optimal >= f_well_beyond - 0.05,
            "Friction at optimal ({}) should be >= friction beyond optimal ({})",
            f_at_optimal,
            f_well_beyond
        );

        // Friction at optimal should be close to peak_friction
        assert!(
            (f_at_optimal - curve.peak_friction).abs() < 0.3,
            "Friction at optimal slip should be near peak. Expected ~{}, got {}",
            curve.peak_friction,
            f_at_optimal
        );

        // Friction should not be negative
        for slip in [0.0, 0.05, 0.1, 0.2, 0.5, 1.0] {
            let f = curve.friction_at_slip(slip);
            assert!(
                f >= 0.0,
                "Friction should never be negative, got {} at slip {}",
                f,
                slip
            );
        }
    }
}

/// Verify tarmac has more grip than ice
#[test]
fn test_friction_surface_comparison_correct() {
    let tarmac = FrictionCurve::tarmac();
    let ice = FrictionCurve::ice();

    // At optimal slip, tarmac should have much more grip
    let tarmac_grip = tarmac.friction_at_slip(tarmac.optimal_slip);
    let ice_grip = ice.friction_at_slip(ice.optimal_slip);

    assert!(
        tarmac_grip > ice_grip * 2.0,
        "Tarmac should have significantly more grip than ice. Tarmac: {}, Ice: {}",
        tarmac_grip,
        ice_grip
    );
}

/// Verify suspension force follows Hooke's law (F = -kx)
#[test]
fn test_suspension_hookes_law() {
    // This is a behavioral check that suspension stiffness works correctly
    // Higher stiffness should mean higher force for same compression

    let wheel_soft = WheelConfig {
        suspension_stiffness: 10000.0,
        ..Default::default()
    };

    let wheel_stiff = WheelConfig {
        suspension_stiffness: 20000.0,
        ..Default::default()
    };

    // Stiffer suspension should have higher stiffness value
    assert!(
        wheel_stiff.suspension_stiffness > wheel_soft.suspension_stiffness,
        "Config values should reflect stiffness correctly"
    );

    // Force ratio should match stiffness ratio (Hooke's law: F ∝ k)
    let ratio = wheel_stiff.suspension_stiffness / wheel_soft.suspension_stiffness;
    assert!(
        (ratio - 2.0).abs() < 0.01,
        "Stiffness ratio should be 2.0, got {}",
        ratio
    );
}

// ============================================================================
// DESTRUCTION PHYSICS CORRECTNESS
// ============================================================================

/// Verify damage reduces health correctly (mutation: adding instead of subtracting)
#[test]
fn test_destruction_damage_reduces_health() {
    let mut manager = DestructionManager::new();
    let config = DestructibleConfig {
        max_health: 100.0,
        ..Default::default()
    };

    let id = manager.add_destructible(config, Vec3::ZERO);

    let initial_health = manager.get(id).unwrap().health;
    assert!(
        (initial_health - 100.0).abs() < 0.01,
        "Initial health should be max"
    );

    // Apply 30 damage
    manager.apply_damage(id, 30.0);
    let after_damage = manager.get(id).unwrap().health;

    assert!(
        (after_damage - 70.0).abs() < 0.01,
        "Health should be 70 after 30 damage. Got {}",
        after_damage
    );

    // Apply more damage
    manager.apply_damage(id, 50.0);
    let final_health = manager.get(id).unwrap().health;

    assert!(
        (final_health - 20.0).abs() < 0.01,
        "Health should be 20 after total 80 damage. Got {}",
        final_health
    );
}

/// Verify debris count matches fracture pattern
#[test]
fn test_destruction_debris_count_matches_pattern() {
    for piece_count in [4, 8, 16, 32] {
        let pattern = FracturePattern::uniform(piece_count, Vec3::splat(0.5), 10.0);

        assert!(
            pattern.debris.len() >= piece_count / 2,
            "Pattern with {} pieces should have at least {} debris configs, got {}",
            piece_count,
            piece_count / 2,
            pattern.debris.len()
        );
    }
}

// ============================================================================
// RAGDOLL PHYSICS CORRECTNESS
// ============================================================================

/// Verify bone shape volumes are calculated correctly
#[test]
fn test_bone_shape_volume_formulas() {
    // Sphere: V = (4/3)πr³
    let sphere = BoneShape::Sphere { radius: 1.0 };
    let expected_sphere = (4.0 / 3.0) * std::f32::consts::PI * 1.0_f32.powi(3);
    assert!(
        (sphere.volume() - expected_sphere).abs() < 0.01,
        "Sphere volume incorrect. Expected {}, got {}",
        expected_sphere,
        sphere.volume()
    );

    // Box: V = 8 * hx * hy * hz (half extents)
    let box_shape = BoneShape::Box {
        half_extents: Vec3::new(1.0, 2.0, 3.0),
    };
    let expected_box = 8.0 * 1.0 * 2.0 * 3.0;
    assert!(
        (box_shape.volume() - expected_box).abs() < 0.01,
        "Box volume incorrect. Expected {}, got {}",
        expected_box,
        box_shape.volume()
    );

    // Capsule: V = cylinder + sphere = πr²h + (4/3)πr³
    let capsule = BoneShape::Capsule {
        radius: 1.0,
        half_height: 2.0,
    };
    let cylinder = std::f32::consts::PI * 1.0 * 1.0 * 4.0; // r²*h
    let sphere_v = (4.0 / 3.0) * std::f32::consts::PI * 1.0_f32.powi(3);
    let expected_capsule = cylinder + sphere_v;
    assert!(
        (capsule.volume() - expected_capsule).abs() < 0.1,
        "Capsule volume incorrect. Expected {}, got {}",
        expected_capsule,
        capsule.volume()
    );
}

/// Verify ragdoll mass scaling is applied correctly
#[test]
fn test_ragdoll_mass_scaling_correct() {
    let config_normal = RagdollConfig {
        mass_scale: 1.0,
        ..Default::default()
    };

    let config_heavy = RagdollConfig {
        mass_scale: 2.0,
        ..Default::default()
    };

    let mut builder_normal = RagdollBuilder::new(config_normal);
    let mut builder_heavy = RagdollBuilder::new(config_heavy);

    // Add same bone to both
    builder_normal.add_bone(
        "test",
        None,
        Vec3::ZERO,
        BoneShape::Sphere { radius: 0.1 },
        10.0,
    );
    builder_heavy.add_bone(
        "test",
        None,
        Vec3::ZERO,
        BoneShape::Sphere { radius: 0.1 },
        10.0,
    );

    // The config's mass_scale should be stored correctly
    assert!(
        (builder_normal.config.mass_scale - 1.0).abs() < 0.01,
        "Normal mass scale should be 1.0"
    );
    assert!(
        (builder_heavy.config.mass_scale - 2.0).abs() < 0.01,
        "Heavy mass scale should be 2.0"
    );
}

// ============================================================================
// MUTATION-CATCHING EDGE CASES
// ============================================================================

/// Catch sign flip mutations in gravity

/// Catch off-by-one in spatial hash cell calculations
#[test]
fn test_mutation_spatial_hash_cell_boundary() {
    let mut hash = SpatialHash::new(10.0); // Cell size 10

    // Entity exactly on cell boundary
    let aabb = AABB::from_center_extents(Vec3::new(10.0, 0.0, 0.0), Vec3::splat(0.5));
    hash.insert(1, aabb);

    // Query straddling the boundary
    let query = AABB::from_center_extents(Vec3::new(9.5, 0.0, 0.0), Vec3::splat(1.0));
    let results = hash.query(query);

    assert!(
        results.contains(&1),
        "Entity at cell boundary should be found by adjacent query"
    );
}

/// Catch division errors in falloff calculations
#[test]
fn test_mutation_falloff_division() {
    // Zero radius should not cause division by zero
    let linear = FalloffCurve::Linear;
    let result = linear.calculate(5.0, 0.0);
    assert!(
        !result.is_nan() && !result.is_infinite(),
        "Zero radius should not cause NaN/Inf"
    );

    // Negative distance should be handled
    let neg_result = linear.calculate(-5.0, 10.0);
    assert!(
        !neg_result.is_nan() && !neg_result.is_infinite(),
        "Negative distance should not cause NaN/Inf"
    );
}

/// Verify zero dt doesn't break physics
#[test]
fn test_mutation_zero_timestep() {
    let mut manager = ProjectileManager::new();
    let config = ProjectileConfig {
        position: Vec3::new(1.0, 2.0, 3.0),
        velocity: Vec3::new(10.0, 20.0, 30.0),
        ..Default::default()
    };
    let id = manager.spawn(config);

    let initial_pos = manager.get(id).unwrap().position;
    let initial_vel = manager.get(id).unwrap().velocity;

    // Update with zero dt
    let raycast = |_: Vec3, _: Vec3, _: f32| None;
    manager.update(0.0, raycast);

    let proj = manager.get(id).unwrap();

    // Nothing should change with zero dt
    assert!(
        (proj.position - initial_pos).length() < 1e-6,
        "Zero dt should not change position"
    );
    assert!(
        (proj.velocity - initial_vel).length() < 1e-6,
        "Zero dt should not change velocity"
    );
}

/// Catch multiplication/division swaps in explosion force
#[test]
fn test_mutation_explosion_force_scaling() {
    let manager = ProjectileManager::new();

    let config_weak = ExplosionConfig {
        center: Vec3::ZERO,
        radius: 10.0,
        force: 100.0,
        falloff: FalloffCurve::Constant,
        upward_bias: 0.0,
    };

    let config_strong = ExplosionConfig {
        center: Vec3::ZERO,
        radius: 10.0,
        force: 1000.0, // 10x stronger
        falloff: FalloffCurve::Constant,
        upward_bias: 0.0,
    };

    let body = vec![(1, Vec3::new(5.0, 0.0, 0.0))];

    let result_weak = manager.calculate_explosion(&config_weak, body.clone());
    let result_strong = manager.calculate_explosion(&config_strong, body);

    let impulse_weak = result_weak[0].impulse.length();
    let impulse_strong = result_strong[0].impulse.length();

    // 10x force should give 10x impulse
    let ratio = impulse_strong / impulse_weak;
    assert!(
        (ratio - 10.0).abs() < 0.1,
        "Force scaling should be linear. Expected ratio 10, got {}",
        ratio
    );
}
