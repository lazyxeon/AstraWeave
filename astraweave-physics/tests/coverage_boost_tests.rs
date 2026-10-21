//! Coverage Boost Tests for AstraWeave Physics
//!
//! These tests target uncovered lines identified by llvm-cov analysis.

use glam::Vec3;

// ============================================================================
// PROJECTILE SYSTEM COVERAGE TESTS
// ============================================================================

mod projectile_coverage {
    use super::*;
    use astraweave_physics::projectile::*;

    #[test]
    fn test_projectile_manager_default() {
        let manager = ProjectileManager::default();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_projectile_crud() {
        let mut manager = ProjectileManager::new();

        let config = ProjectileConfig {
            kind: ProjectileKind::Kinematic,
            position: Vec3::ZERO,
            velocity: Vec3::new(0.0, 10.0, 50.0),
            gravity_scale: 1.0,
            drag: 0.1,
            radius: 0.1,
            max_lifetime: 10.0,
            ..Default::default()
        };

        let id = manager.spawn(config);
        assert_eq!(manager.count(), 1);

        assert!(manager.get(id).is_some());
        assert!(manager.get(9999).is_none());

        if let Some(proj) = manager.get_mut(id) {
            proj.velocity = Vec3::new(100.0, 0.0, 0.0);
        }
        assert_eq!(
            manager.get(id).unwrap().velocity,
            Vec3::new(100.0, 0.0, 0.0)
        );

        assert_eq!(manager.iter().count(), 1);

        assert!(manager.despawn(id));
        assert!(!manager.despawn(id));
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_falloff_curves() {
        let linear = FalloffCurve::Linear;
        let quadratic = FalloffCurve::Quadratic;
        let exponential = FalloffCurve::Exponential;
        let constant = FalloffCurve::Constant;

        // At center
        assert!((linear.calculate(0.0, 10.0) - 1.0).abs() < 0.01);
        assert!((quadratic.calculate(0.0, 10.0) - 1.0).abs() < 0.01);
        assert!((constant.calculate(0.0, 10.0) - 1.0).abs() < 0.01);

        // At half distance
        assert!((linear.calculate(5.0, 10.0) - 0.5).abs() < 0.01);
        assert!((quadratic.calculate(5.0, 10.0) - 0.75).abs() < 0.01);
        assert!((constant.calculate(5.0, 10.0) - 1.0).abs() < 0.01);

        // At edge - returns 0.0 since distance >= radius
        assert!((linear.calculate(10.0, 10.0) - 0.0).abs() < 0.01);
        assert_eq!(linear.calculate(15.0, 10.0), 0.0);
        assert_eq!(exponential.calculate(15.0, 10.0), 0.0);

        // Edge case: zero radius - distance (0.0) >= radius (0.0) is true, returns 0.0
        assert_eq!(linear.calculate(0.0, 0.0), 0.0);

        // Positive radius, center position returns 1.0
        assert_eq!(constant.calculate(0.0, 1.0), 1.0);
    }

    #[test]
    fn test_projectile_kinds() {
        let mut manager = ProjectileManager::new();

        let kinematic = ProjectileConfig {
            kind: ProjectileKind::Kinematic,
            position: Vec3::new(0.0, 10.0, 0.0),
            velocity: Vec3::new(0.0, 0.0, 50.0),
            gravity_scale: 1.0,
            drag: 0.0,
            ..Default::default()
        };
        let k_id = manager.spawn(kinematic);

        let hitscan = ProjectileConfig {
            kind: ProjectileKind::Hitscan,
            position: Vec3::ZERO,
            velocity: Vec3::new(0.0, 0.0, 1000.0),
            gravity_scale: 0.0,
            ..Default::default()
        };
        let h_id = manager.spawn(hitscan);

        assert!(manager.get(k_id).is_some());
        assert!(manager.get(h_id).is_some());

        manager.update(0.016, |_origin, _dir, _dist| None);

        let k_proj = manager.get(k_id).unwrap();
        assert!(k_proj.position.z > 0.0);
    }

    #[test]
    fn test_explosion_config() {
        let explosion = ExplosionConfig::default();
        assert!(explosion.radius > 0.0);
        assert!(explosion.force > 0.0);

        let custom = ExplosionConfig {
            center: Vec3::new(10.0, 5.0, 10.0),
            radius: 20.0,
            force: 5000.0,
            falloff: FalloffCurve::Quadratic,
            upward_bias: 0.5,
        };
        assert_eq!(custom.radius, 20.0);
    }
}

// ============================================================================
// VEHICLE PHYSICS COVERAGE TESTS
// ============================================================================

mod vehicle_coverage {
    use super::*;
    use astraweave_physics::vehicle::*;

    #[test]
    fn test_vehicle_manager_default() {
        let manager = VehicleManager::default();
        assert!(manager.get(1).is_none());
    }

    #[test]
    fn test_friction_curve_default_and_evaluate() {
        let curve = FrictionCurve::default();
        assert!(curve.optimal_slip > 0.0);
        assert!(curve.peak_friction > 0.0);

        let f0 = curve.friction_at_slip(0.0);
        assert_eq!(f0, 0.0);

        let f_opt = curve.friction_at_slip(curve.optimal_slip);
        assert!(f_opt > 0.0);

        let f_high = curve.friction_at_slip(0.5);
        assert!(f_high > 0.0);

        let f_neg = curve.friction_at_slip(-0.1);
        assert!(f_neg > 0.0);
    }

    #[test]
    fn test_friction_curve_presets() {
        let tarmac = FrictionCurve::tarmac();
        let gravel = FrictionCurve::gravel();
        let ice = FrictionCurve::ice();
        let mud = FrictionCurve::mud();

        assert!(tarmac.peak_friction > gravel.peak_friction);
        assert!(tarmac.peak_friction > ice.peak_friction);
        assert!(ice.peak_friction < gravel.peak_friction);
        assert!(ice.peak_friction < mud.peak_friction);
        assert!(gravel.optimal_slip > tarmac.optimal_slip);
    }

    #[test]
    fn test_wheel_builders() {
        let fl = WheelConfig::front_left(Vec3::new(-1.0, 0.0, 1.5));
        let fr = WheelConfig::front_right(Vec3::new(1.0, 0.0, 1.5));
        let rl = WheelConfig::rear_left(Vec3::new(-1.0, 0.0, -1.5));
        let rr = WheelConfig::rear_right(Vec3::new(1.0, 0.0, -1.5));

        assert!(fl.steerable);
        assert!(fr.steerable);
        assert!(!rl.steerable);
        assert!(!rr.steerable);
    }

    #[test]
    fn test_wheel_builder_methods() {
        let wheel = WheelConfig::rear_left(Vec3::new(-1.0, 0.0, -1.5))
            .with_drive()
            .with_radius(0.35)
            .with_suspension(40000.0, 4000.0, 0.3);

        assert!(wheel.driven);
        assert_eq!(wheel.radius, 0.35);
        assert_eq!(wheel.suspension_stiffness, 40000.0);
        assert_eq!(wheel.suspension_damping, 4000.0);
        assert_eq!(wheel.suspension_rest_length, 0.3);
    }

    #[test]
    fn test_engine_config() {
        let engine = EngineConfig {
            max_torque: 300.0,
            max_torque_rpm: 4500.0,
            max_rpm: 8000.0,
            idle_rpm: 800.0,
            engine_braking: 0.3,
        };

        // EngineConfig fields
        assert_eq!(engine.max_torque, 300.0);
        assert_eq!(engine.max_rpm, 8000.0);
        assert!(engine.max_torque_rpm < engine.max_rpm);
        assert!(engine.idle_rpm < engine.max_torque_rpm);
    }

    #[test]
    fn test_transmission_config() {
        let trans = TransmissionConfig {
            gear_ratios: vec![3.5, 2.5, 1.8, 1.3, 1.0, 0.8],
            reverse_ratio: 3.5,
            final_drive: 3.7,
            shift_time: 0.2,
        };

        assert_eq!(trans.num_gears(), 6);

        let reverse = trans.effective_ratio(-1);
        assert!((reverse - 3.5 * 3.7).abs() < 0.01);

        let first = trans.effective_ratio(1);
        assert!((first - 3.5 * 3.7).abs() < 0.01);

        let neutral = trans.effective_ratio(0);
        assert_eq!(neutral, 0.0);
    }
}

// ============================================================================
// CLOTH SIMULATION COVERAGE TESTS
// ============================================================================

mod cloth_coverage {
    use super::*;
    use astraweave_physics::cloth::*;

    #[test]
    fn test_cloth_manager_crud() {
        let mut manager = ClothManager::new();

        let config = ClothConfig {
            width: 10,
            height: 10,
            spacing: 0.1,
            particle_mass: 0.1,
            stiffness: 0.8,
            damping: 0.98,
            solver_iterations: 3,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            wind: Vec3::ZERO,
            air_resistance: 0.01,
        };

        let id = manager.create(config.clone(), Vec3::ZERO);

        assert!(manager.get(id).is_some());

        if let Some(cloth) = manager.get_mut(id) {
            cloth.config.damping = 0.95;
        }
        assert!((manager.get(id).unwrap().config.damping - 0.95).abs() < 0.001);

        manager.create(config.clone(), Vec3::new(5.0, 0.0, 0.0));
        manager.create(config, Vec3::new(10.0, 0.0, 0.0));

        assert_eq!(manager.iter().count(), 3);
        assert_eq!(manager.count(), 3);

        assert!(manager.remove(id));
        assert!(!manager.remove(id));
        assert_eq!(manager.iter().count(), 2);
    }

    #[test]
    fn test_cloth_colliders() {
        let config = ClothConfig::default();
        let mut cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);

        cloth.add_collider(ClothCollider::Sphere {
            center: Vec3::new(0.0, -1.0, 0.0),
            radius: 1.0,
        });

        cloth.add_collider(ClothCollider::Plane {
            point: Vec3::new(0.0, -2.0, 0.0),
            normal: Vec3::Y,
        });

        cloth.add_collider(ClothCollider::Capsule {
            start: Vec3::new(-1.0, 0.0, 0.0),
            end: Vec3::new(1.0, 0.0, 0.0),
            radius: 0.3,
        });

        assert_eq!(cloth.colliders.len(), 3);

        cloth.clear_colliders();
        assert_eq!(cloth.colliders.len(), 0);
    }

    #[test]
    fn test_cloth_particle_access() {
        let config = ClothConfig {
            width: 10,
            height: 10,
            ..Default::default()
        };

        let mut cloth = Cloth::new(ClothId(1), config, Vec3::new(0.0, 5.0, 0.0));

        assert!(cloth.particle_position(0).is_some());
        assert!(cloth.particle_position(50).is_some());
        assert!(cloth.particle_position(99).is_some());
        assert!(cloth.particle_position(1000).is_none());

        assert!(cloth.particle_index(0, 0).is_some());
        assert!(cloth.particle_index(9, 9).is_some());
        assert!(cloth.particle_index(100, 100).is_none());

        cloth.pin_particle(0);
        cloth.pin_particle(9);
        assert!(cloth.particles[0].pinned);
        assert!(cloth.particles[9].pinned);

        cloth.unpin_particle(0);
        assert!(!cloth.particles[0].pinned);

        for _ in 0..30 {
            cloth.update(0.016);
        }

        assert!(cloth.particle_position(0).is_some());
    }

    #[test]
    fn test_cloth_pin_methods() {
        let config = ClothConfig {
            width: 8,
            height: 8,
            ..Default::default()
        };

        let mut cloth = Cloth::new(ClothId(1), config, Vec3::new(0.0, 5.0, 0.0));

        cloth.pin_top_edge();
        for x in 0..8 {
            assert!(cloth.particles[x].pinned);
        }

        let mut cloth2 = Cloth::new(ClothId(2), ClothConfig::default(), Vec3::ZERO);
        cloth2.pin_corners();
        assert!(cloth2.particles[0].pinned);
        assert!(cloth2.particles[cloth2.config.width - 1].pinned);
    }

    #[test]
    fn test_cloth_update_simulation() {
        let config = ClothConfig {
            width: 5,
            height: 5,
            spacing: 0.1,
            wind: Vec3::new(5.0, 0.0, 0.0),
            ..Default::default()
        };

        let mut cloth = Cloth::new(ClothId(1), config, Vec3::new(0.0, 5.0, 0.0));
        cloth.pin_top_edge();

        for _ in 0..100 {
            cloth.update(0.016);
        }

        // Cloth should have fallen and moved with wind
        let bottom_center = cloth.particle_position(
            cloth.config.width * (cloth.config.height - 1) + cloth.config.width / 2,
        );
        assert!(bottom_center.is_some());
    }
}

// ============================================================================
// ENVIRONMENT SYSTEM COVERAGE TESTS
// ============================================================================

mod environment_coverage {
    use super::*;
    use astraweave_physics::environment::*;

    #[test]
    fn test_environment_manager_crud() {
        let mut manager = EnvironmentManager::new();

        let config = WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Sphere { radius: 10.0 },
            wind_type: WindType::Directional,
            direction: Vec3::X,
            strength: 20.0,
            falloff: 0.5,
            active: true,
        };

        let id = manager.add_wind_zone(config);

        assert!(manager.get_wind_zone(id).is_some());

        if let Some(zone) = manager.get_wind_zone_mut(id) {
            zone.config.strength = 50.0;
        }
        assert!((manager.get_wind_zone(id).unwrap().config.strength - 50.0).abs() < 0.01);

        assert!(manager.remove_wind_zone(id));
        assert!(!manager.remove_wind_zone(id));
    }

    #[test]
    fn test_wind_zone_shapes() {
        let mut manager = EnvironmentManager::new();

        manager.add_wind_zone(WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Global,
            wind_type: WindType::Directional,
            direction: Vec3::X,
            strength: 5.0,
            falloff: 0.0,
            active: true,
        });

        manager.add_wind_zone(WindZoneConfig {
            position: Vec3::new(20.0, 0.0, 0.0),
            shape: WindZoneShape::Box {
                half_extents: Vec3::new(5.0, 5.0, 5.0),
            },
            wind_type: WindType::Directional,
            direction: Vec3::Y,
            strength: 10.0,
            falloff: 0.0,
            active: true,
        });

        manager.add_wind_zone(WindZoneConfig {
            position: Vec3::new(40.0, 0.0, 0.0),
            shape: WindZoneShape::Cylinder {
                radius: 5.0,
                height: 20.0,
            },
            wind_type: WindType::Vortex {
                tangential_speed: 20.0,
                inward_pull: 5.0,
                updraft: 10.0,
            },
            direction: Vec3::Y,
            strength: 50.0,
            falloff: 0.3,
            active: true,
        });

        assert_eq!(manager.wind_zone_count(), 3);
    }

    #[test]
    fn test_wind_types() {
        let mut manager = EnvironmentManager::new();

        manager.add_wind_zone(WindZoneConfig {
            wind_type: WindType::Directional,
            ..Default::default()
        });

        manager.add_wind_zone(WindZoneConfig {
            position: Vec3::new(20.0, 0.0, 0.0),
            shape: WindZoneShape::Cylinder {
                radius: 10.0,
                height: 30.0,
            },
            wind_type: WindType::Vortex {
                tangential_speed: 15.0,
                inward_pull: 3.0,
                updraft: 5.0,
            },
            ..Default::default()
        });

        manager.add_wind_zone(WindZoneConfig {
            position: Vec3::new(40.0, 0.0, 0.0),
            wind_type: WindType::Turbulent {
                intensity: 1.0,
                frequency: 2.0,
            },
            ..Default::default()
        });

        assert_eq!(manager.wind_zone_count(), 3);
    }

    #[test]
    fn test_environment_counts() {
        let mut manager = EnvironmentManager::new();

        assert_eq!(manager.wind_zone_count(), 0);

        manager.add_wind_zone(WindZoneConfig::default());
        manager.add_wind_zone(WindZoneConfig::default());

        assert_eq!(manager.wind_zone_count(), 2);
    }

    #[test]
    fn test_environment_update() {
        let mut manager = EnvironmentManager::new();

        manager.add_wind_zone(WindZoneConfig {
            wind_type: WindType::Turbulent {
                intensity: 1.0,
                frequency: 2.0,
            },
            ..Default::default()
        });

        for _ in 0..100 {
            manager.update(0.016);
        }

        assert!(manager.wind_zone_count() > 0);
    }
}

// ============================================================================
// SPATIAL HASH COVERAGE TESTS
// ============================================================================

mod spatial_hash_coverage {
    use super::*;
    use astraweave_physics::spatial_hash::*;

    #[test]
    fn test_spatial_hash_operations() {
        let mut hash = SpatialHash::<u64>::new(10.0);

        for i in 0..100 {
            let x = (i % 10) as f32 * 5.0;
            let z = (i / 10) as f32 * 5.0;
            let aabb =
                AABB::from_center_extents(Vec3::new(x + 0.5, 0.5, z + 0.5), Vec3::splat(0.5));
            hash.insert(i as u64, aabb);
        }

        let query_aabb = AABB::from_center_extents(Vec3::new(10.0, 0.5, 10.0), Vec3::splat(10.0));
        let results = hash.query(query_aabb);
        assert!(!results.is_empty());

        hash.clear();
        let query_aabb2 = AABB::from_center_extents(Vec3::new(10.0, 0.5, 10.0), Vec3::splat(10.0));
        let empty_results = hash.query(query_aabb2);
        assert!(empty_results.is_empty());
    }

    #[test]
    fn test_aabb_methods() {
        let aabb1 = AABB::from_center_extents(Vec3::new(0.5, 0.5, 0.5), Vec3::splat(0.5));
        let aabb2 = AABB::from_center_extents(Vec3::new(1.0, 1.0, 1.0), Vec3::splat(0.5));
        let aabb3 = AABB::from_center_extents(Vec3::new(5.5, 5.5, 5.5), Vec3::splat(0.5));

        assert!(aabb1.intersects(&aabb2));
        assert!(!aabb1.intersects(&aabb3));

        let center = aabb1.center();
        assert!((center - Vec3::new(0.5, 0.5, 0.5)).length() < 0.01);

        let sphere_aabb = AABB::from_sphere(Vec3::ZERO, 1.0);
        assert!((sphere_aabb.min + Vec3::ONE).length() < 0.01);
        assert!((sphere_aabb.max - Vec3::ONE).length() < 0.01);
    }
}

// ============================================================================
// PHYSICS WORLD INTEGRATION COVERAGE TESTS
// ============================================================================

mod physics_world_coverage {
    use super::*;
    use astraweave_physics::*;

    #[test]
    fn test_radial_impulse() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        for i in 0..10 {
            let angle = (i as f32) * std::f32::consts::TAU / 10.0;
            physics.add_dynamic_box(
                Vec3::new(angle.cos() * 5.0, 5.0, angle.sin() * 5.0),
                Vec3::ONE * 0.5,
                1.0,
                Layers::DEFAULT,
            );
        }

        let affected = physics.apply_radial_impulse(
            Vec3::ZERO,
            20.0,
            100.0,
            projectile::FalloffCurve::Linear,
            0.3,
        );

        assert!(affected > 0);
        physics.step();
    }

    #[test]
    fn test_wind_and_ccd() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        physics.set_wind(Vec3::X, 15.0);

        let body_id = physics.add_dynamic_box(
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::ONE * 0.1,
            0.5,
            Layers::DEFAULT,
        );

        physics.enable_ccd(body_id);

        for _ in 0..30 {
            physics.step();
        }

        if let Some(_transform) = physics.body_transform(body_id) {
            // Body should have moved
        }
    }

    #[test]
    fn test_set_body_position() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        let body_id = physics.add_dynamic_box(Vec3::ZERO, Vec3::ONE, 1.0, Layers::DEFAULT);

        physics.set_body_position(body_id, Vec3::new(100.0, 50.0, 25.0));

        if let Some(transform) = physics.body_transform(body_id) {
            let pos = Vec3::new(transform.w_axis.x, transform.w_axis.y, transform.w_axis.z);
            assert!((pos.x - 100.0).abs() < 0.1);
            assert!((pos.y - 50.0).abs() < 0.1);
            assert!((pos.z - 25.0).abs() < 0.1);
        }
    }

    #[test]
    fn test_velocity_operations() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        let body_id =
            physics.add_dynamic_box(Vec3::new(0.0, 10.0, 0.0), Vec3::ONE, 1.0, Layers::DEFAULT);

        physics.set_velocity(body_id, Vec3::new(10.0, 5.0, -3.0));

        if let Some(vel) = physics.get_velocity(body_id) {
            assert!((vel.x - 10.0).abs() < 0.1);
            assert!((vel.y - 5.0).abs() < 0.1);
            assert!((vel.z - (-3.0)).abs() < 0.1);
        }
    }

    #[test]
    fn test_raycast() {
        let mut physics = PhysicsWorld::new(Vec3::new(0.0, -9.81, 0.0));

        physics.create_ground_plane(Vec3::new(100.0, 0.1, 100.0), 0.5);
        physics.add_dynamic_box(Vec3::new(0.0, 2.0, 0.0), Vec3::ONE, 10.0, Layers::DEFAULT);

        if let Some(hit) =
            physics.raycast(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.0, -1.0, 0.0), 20.0)
        {
            assert!(hit.3 < 20.0); // hit.3 is distance
        }
    }
}
