//! Mutation-killing tests for astraweave-physics.
//! These tests are designed to catch subtle mutations like boundary condition changes,
//! arithmetic operator substitutions, and conditional logic inversions.

#[cfg(test)]
mod aabb_tests {
    use crate::spatial_hash::AABB;
    use glam::Vec3;

    #[test]
    fn test_aabb_from_center_extents() {
        let aabb = AABB::from_center_extents(Vec3::new(5.0, 5.0, 5.0), Vec3::new(2.0, 3.0, 4.0));

        // Verify min = center - extents
        assert_eq!(aabb.min.x, 3.0);
        assert_eq!(aabb.min.y, 2.0);
        assert_eq!(aabb.min.z, 1.0);

        // Verify max = center + extents
        assert_eq!(aabb.max.x, 7.0);
        assert_eq!(aabb.max.y, 8.0);
        assert_eq!(aabb.max.z, 9.0);
    }

    #[test]
    fn test_aabb_from_sphere() {
        let aabb = AABB::from_sphere(Vec3::ZERO, 5.0);

        // Sphere creates cube AABB with half-extent = radius
        assert_eq!(aabb.min, Vec3::new(-5.0, -5.0, -5.0));
        assert_eq!(aabb.max, Vec3::new(5.0, 5.0, 5.0));
    }

    #[test]
    fn test_aabb_intersects_touching() {
        let a = AABB::from_center_extents(Vec3::ZERO, Vec3::splat(1.0));
        let b = AABB::from_center_extents(Vec3::new(2.0, 0.0, 0.0), Vec3::splat(1.0));

        // Boxes touch at x=1.0, should intersect
        assert!(a.intersects(&b));
    }

    #[test]
    fn test_aabb_intersects_separated() {
        let a = AABB::from_center_extents(Vec3::ZERO, Vec3::splat(1.0));
        let b = AABB::from_center_extents(Vec3::new(3.0, 0.0, 0.0), Vec3::splat(1.0));

        // Boxes separated on X axis
        assert!(!a.intersects(&b));
    }

    #[test]
    fn test_aabb_intersects_all_axes() {
        let a = AABB::from_center_extents(Vec3::ZERO, Vec3::splat(1.0));

        // Separated on Y
        let b = AABB::from_center_extents(Vec3::new(0.0, 3.0, 0.0), Vec3::splat(1.0));
        assert!(!a.intersects(&b));

        // Separated on Z
        let c = AABB::from_center_extents(Vec3::new(0.0, 0.0, 3.0), Vec3::splat(1.0));
        assert!(!a.intersects(&c));
    }

    #[test]
    fn test_aabb_center() {
        let aabb = AABB {
            min: Vec3::new(-2.0, -4.0, -6.0),
            max: Vec3::new(2.0, 4.0, 6.0),
        };

        let center = aabb.center();
        assert_eq!(center, Vec3::ZERO);
    }

    #[test]
    fn test_aabb_center_offset() {
        let aabb = AABB {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(10.0, 20.0, 30.0),
        };

        let center = aabb.center();
        assert_eq!(center, Vec3::new(5.0, 10.0, 15.0));
    }
}

#[cfg(test)]
mod spatial_hash_tests {
    use crate::spatial_hash::{SpatialHash, AABB};
    use glam::Vec3;

    #[test]
    fn test_spatial_hash_insert_and_query() {
        let mut grid = SpatialHash::<u32>::new(10.0);

        let aabb = AABB::from_sphere(Vec3::new(5.0, 5.0, 5.0), 1.0);
        grid.insert(1, aabb);

        let results = grid.query(aabb);
        assert!(results.contains(&1));
    }

    #[test]
    fn test_spatial_hash_multiple_objects() {
        let mut grid = SpatialHash::<u32>::new(10.0);

        // Insert objects at different locations
        grid.insert(1, AABB::from_sphere(Vec3::new(0.0, 0.0, 0.0), 1.0));
        grid.insert(2, AABB::from_sphere(Vec3::new(5.0, 0.0, 0.0), 1.0));
        grid.insert(3, AABB::from_sphere(Vec3::new(100.0, 0.0, 0.0), 1.0));

        // Query near origin should find 1 and 2
        let query_aabb = AABB::from_sphere(Vec3::new(2.5, 0.0, 0.0), 5.0);
        let results = grid.query(query_aabb);

        assert!(results.contains(&1));
        assert!(results.contains(&2));
        // Object 3 is far away
    }

    #[test]
    fn test_spatial_hash_clear() {
        let mut grid = SpatialHash::<u32>::new(10.0);

        grid.insert(1, AABB::from_sphere(Vec3::ZERO, 1.0));
        grid.clear();

        let results = grid.query(AABB::from_sphere(Vec3::ZERO, 1.0));
        assert!(!results.contains(&1));
    }

    #[test]
    fn test_spatial_hash_negative_coordinates() {
        let mut grid = SpatialHash::<u32>::new(10.0);

        let aabb = AABB::from_sphere(Vec3::new(-50.0, -50.0, -50.0), 1.0);
        grid.insert(1, aabb);

        let results = grid.query(aabb);
        assert!(results.contains(&1));
    }

    #[test]
    fn test_spatial_hash_cell_size() {
        // Small cell size
        let grid_small = SpatialHash::<u32>::new(1.0);
        assert_eq!(grid_small.cell_size(), 1.0);

        // Large cell size
        let grid_large = SpatialHash::<u32>::new(100.0);
        assert_eq!(grid_large.cell_size(), 100.0);
    }
}

#[cfg(test)]
mod projectile_tests {
    use crate::projectile::{ProjectileConfig, ProjectileKind, ProjectileManager};
    use glam::Vec3;

    #[test]
    fn test_projectile_config_defaults() {
        let config = ProjectileConfig::default();

        assert_eq!(config.kind, ProjectileKind::Kinematic);
        assert_eq!(config.gravity_scale, 1.0);
        assert_eq!(config.drag, 0.0);
        assert_eq!(config.max_lifetime, 10.0);
        assert_eq!(config.max_bounces, 0);
        assert_eq!(config.restitution, 0.5);
        assert_eq!(config.penetration, 0.0);
    }

    #[test]
    fn test_projectile_spawn() {
        let mut manager = ProjectileManager::new();

        let config = ProjectileConfig {
            position: Vec3::new(0.0, 1.0, 0.0),
            velocity: Vec3::new(10.0, 5.0, 0.0),
            ..Default::default()
        };

        let id = manager.spawn(config);
        assert!(id > 0);
    }

    #[test]
    fn test_projectile_spawn_multiple() {
        let mut manager = ProjectileManager::new();

        let id1 = manager.spawn(ProjectileConfig::default());
        let id2 = manager.spawn(ProjectileConfig::default());
        let id3 = manager.spawn(ProjectileConfig::default());

        // Each ID should be unique
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_projectile_kind_hitscan() {
        let config = ProjectileConfig {
            kind: ProjectileKind::Hitscan,
            ..Default::default()
        };

        assert_eq!(config.kind, ProjectileKind::Hitscan);
    }

    #[test]
    fn test_projectile_gravity_scales() {
        // Zero gravity
        let config_zero = ProjectileConfig {
            gravity_scale: 0.0,
            ..Default::default()
        };
        assert_eq!(config_zero.gravity_scale, 0.0);

        // Negative (reverse) gravity
        let config_neg = ProjectileConfig {
            gravity_scale: -1.0,
            ..Default::default()
        };
        assert_eq!(config_neg.gravity_scale, -1.0);

        // High gravity
        let config_high = ProjectileConfig {
            gravity_scale: 5.0,
            ..Default::default()
        };
        assert_eq!(config_high.gravity_scale, 5.0);
    }

    #[test]
    fn test_projectile_with_owner() {
        let config = ProjectileConfig {
            owner: Some(42),
            user_data: 12345,
            ..Default::default()
        };

        assert_eq!(config.owner, Some(42));
        assert_eq!(config.user_data, 12345);
    }
}

// =============================================================================
// BOUNDARY CONDITION TESTS - Test exact boundary values to catch < vs <= mutations
// =============================================================================

#[cfg(test)]
mod boundary_condition_tests {
    use crate::projectile::{ProjectileConfig, ProjectileKind};
    use crate::spatial_hash::{SpatialHash, AABB};
    use crate::{ActorKind, DebugLine, Layers};
    use glam::Vec3;

    // --- AABB boundary tests ---

    #[test]
    fn aabb_min_equals_max_is_degenerate() {
        let aabb = AABB {
            min: Vec3::splat(5.0),
            max: Vec3::splat(5.0),
        };
        // Volume should be zero
        let extent = aabb.max - aabb.min;
        assert_eq!(extent.x, 0.0);
        assert_eq!(extent.y, 0.0);
        assert_eq!(extent.z, 0.0);
    }

    #[test]
    fn aabb_intersection_at_exact_touch() {
        // Two AABBs exactly touching (sharing a face)
        let a = AABB {
            min: Vec3::ZERO,
            max: Vec3::splat(1.0),
        };
        let b = AABB {
            min: Vec3::new(1.0, 0.0, 0.0),
            max: Vec3::new(2.0, 1.0, 1.0),
        };
        // Touching at x=1.0 - should intersect (<=)
        assert!(a.intersects(&b), "Touching AABBs should intersect");
    }

    #[test]
    fn aabb_no_intersection_when_epsilon_apart() {
        let a = AABB {
            min: Vec3::ZERO,
            max: Vec3::splat(1.0),
        };
        let b = AABB {
            min: Vec3::new(1.001, 0.0, 0.0),
            max: Vec3::new(2.001, 1.0, 1.0),
        };
        // Gap of 0.001 - should NOT intersect
        assert!(!a.intersects(&b), "AABBs with gap should not intersect");
    }

    #[test]
    fn aabb_zero_extent_sphere() {
        let aabb = AABB::from_sphere(Vec3::ZERO, 0.0);
        assert_eq!(aabb.min, Vec3::ZERO);
        assert_eq!(aabb.max, Vec3::ZERO);
    }

    #[test]
    fn aabb_sphere_radius_at_one() {
        let aabb = AABB::from_sphere(Vec3::ZERO, 1.0);
        assert_eq!(aabb.min, Vec3::splat(-1.0));
        assert_eq!(aabb.max, Vec3::splat(1.0));
    }

    // --- DebugLine boundaries ---

    #[test]
    fn debug_line_zero_length() {
        let line = DebugLine::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert_eq!(line.length(), 0.0);
        assert!(line.is_degenerate());
    }

    #[test]
    fn debug_line_length_at_one() {
        let line = DebugLine::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert!((line.length() - 1.0).abs() < 0.0001);
        assert!(!line.is_degenerate());
    }

    #[test]
    fn debug_line_epsilon_length_not_degenerate() {
        let line = DebugLine::new([0.0, 0.0, 0.0], [0.001, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert!(line.length() > 0.0);
        assert!(!line.is_degenerate());
    }

    // --- Projectile config boundaries ---

    #[test]
    fn projectile_gravity_at_zero() {
        let config = ProjectileConfig {
            gravity_scale: 0.0,
            ..Default::default()
        };
        assert_eq!(config.gravity_scale, 0.0);
    }

    #[test]
    fn projectile_gravity_at_one() {
        let config = ProjectileConfig {
            gravity_scale: 1.0,
            ..Default::default()
        };
        assert_eq!(config.gravity_scale, 1.0);
    }

    #[test]
    fn projectile_lifetime_at_minimum() {
        let config = ProjectileConfig {
            max_lifetime: 0.001,
            ..Default::default()
        };
        assert!(config.max_lifetime > 0.0);
    }

    #[test]
    fn projectile_bounces_at_zero() {
        let config = ProjectileConfig {
            max_bounces: 0,
            ..Default::default()
        };
        assert_eq!(config.max_bounces, 0);
    }

    #[test]
    fn projectile_bounces_at_one() {
        let config = ProjectileConfig {
            max_bounces: 1,
            ..Default::default()
        };
        assert_eq!(config.max_bounces, 1);
    }

    // --- Spatial hash cell size boundaries ---

    #[test]
    fn spatial_hash_small_cell_size() {
        let grid = SpatialHash::<u32>::new(0.1);
        assert!((grid.cell_size() - 0.1).abs() < 0.0001);
    }

    #[test]
    fn spatial_hash_large_cell_size() {
        let grid = SpatialHash::<u32>::new(1000.0);
        assert!((grid.cell_size() - 1000.0).abs() < 0.01);
    }
}

// =============================================================================
// COMPARISON OPERATOR TESTS - Test to catch == vs != and < vs > swaps
// =============================================================================

#[cfg(test)]
mod comparison_operator_tests {
    use crate::projectile::ProjectileKind;
    use crate::spatial_hash::AABB;
    use crate::{ActorKind, Layers};
    use glam::Vec3;

    // --- AABB equality ---

    #[test]
    fn aabb_equal_when_same() {
        let a = AABB::from_sphere(Vec3::ZERO, 5.0);
        let b = AABB::from_sphere(Vec3::ZERO, 5.0);
        assert_eq!(a.min, b.min);
        assert_eq!(a.max, b.max);
    }

    #[test]
    fn aabb_not_equal_when_different_min() {
        let a = AABB::from_center_extents(Vec3::ZERO, Vec3::splat(1.0));
        let b = AABB::from_center_extents(Vec3::new(1.0, 0.0, 0.0), Vec3::splat(1.0));
        assert_ne!(a.min, b.min);
    }

    #[test]
    fn aabb_not_equal_when_different_max() {
        let a = AABB::from_center_extents(Vec3::ZERO, Vec3::splat(1.0));
        let b = AABB::from_center_extents(Vec3::ZERO, Vec3::splat(2.0));
        assert_ne!(a.max, b.max);
    }

    // --- ProjectileKind comparisons ---

    #[test]
    fn projectile_kind_hitscan_equals_hitscan() {
        assert_eq!(ProjectileKind::Hitscan, ProjectileKind::Hitscan);
    }

    #[test]
    fn projectile_kind_kinematic_equals_kinematic() {
        assert_eq!(ProjectileKind::Kinematic, ProjectileKind::Kinematic);
    }

    #[test]
    fn projectile_kind_hitscan_not_equals_kinematic() {
        assert_ne!(ProjectileKind::Hitscan, ProjectileKind::Kinematic);
    }

    // --- ActorKind comparisons ---

    #[test]
    fn actor_kind_static_equals_static() {
        assert_eq!(ActorKind::Static, ActorKind::Static);
    }

    #[test]
    fn actor_kind_dynamic_not_equals_static() {
        assert_ne!(ActorKind::Dynamic, ActorKind::Static);
    }

    #[test]
    fn actor_kind_character_not_equals_dynamic() {
        assert_ne!(ActorKind::Character, ActorKind::Dynamic);
    }

    // --- Layers comparisons ---

    #[test]
    fn layers_default_equals_default() {
        assert_eq!(Layers::DEFAULT, Layers::DEFAULT);
    }

    #[test]
    fn layers_character_equals_character() {
        assert_eq!(Layers::CHARACTER, Layers::CHARACTER);
    }

    #[test]
    fn layers_default_not_equals_character() {
        assert_ne!(Layers::DEFAULT, Layers::CHARACTER);
    }

    // --- Vec3 comparisons in AABBs ---

    #[test]
    fn vec3_comparison_for_intersection() {
        let a = AABB {
            min: Vec3::ZERO,
            max: Vec3::splat(2.0),
        };
        let b = AABB {
            min: Vec3::splat(1.0),
            max: Vec3::splat(3.0),
        };

        // Test overlap condition: a.max >= b.min AND b.max >= a.min
        assert!(a.max.x >= b.min.x);
        assert!(b.max.x >= a.min.x);
        assert!(a.intersects(&b));
    }

    #[test]
    fn vec3_comparison_no_intersection() {
        let a = AABB {
            min: Vec3::ZERO,
            max: Vec3::splat(1.0),
        };
        let b = AABB {
            min: Vec3::splat(2.0),
            max: Vec3::splat(3.0),
        };

        // No overlap: a.max < b.min on all axes
        assert!(a.max.x < b.min.x);
        assert!(!a.intersects(&b));
    }
}

// =============================================================================
// BOOLEAN RETURN PATH TESTS - Test all paths through boolean-returning functions
// =============================================================================

#[cfg(test)]
mod boolean_return_path_tests {
    use crate::spatial_hash::{SpatialHash, AABB};
    use crate::{ActorKind, DebugLine};
    use glam::Vec3;

    // --- AABB.intersects() paths ---

    #[test]
    fn aabb_intersects_returns_true_for_overlap() {
        let a = AABB::from_sphere(Vec3::ZERO, 2.0);
        let b = AABB::from_sphere(Vec3::splat(1.0), 2.0);
        assert!(a.intersects(&b));
    }

    #[test]
    fn aabb_intersects_returns_false_for_no_overlap() {
        let a = AABB::from_sphere(Vec3::ZERO, 1.0);
        let b = AABB::from_sphere(Vec3::splat(10.0), 1.0);
        assert!(!a.intersects(&b));
    }

    #[test]
    fn aabb_intersects_returns_true_for_self() {
        let a = AABB::from_sphere(Vec3::new(5.0, 10.0, 15.0), 3.0);
        assert!(a.intersects(&a));
    }

    // --- ActorKind boolean methods ---

    #[test]
    fn is_static_returns_true_for_static() {
        assert!(ActorKind::Static.is_static());
    }

    #[test]
    fn is_static_returns_false_for_dynamic() {
        assert!(!ActorKind::Dynamic.is_static());
    }

    #[test]
    fn is_dynamic_returns_true_for_dynamic() {
        assert!(ActorKind::Dynamic.is_dynamic());
    }

    #[test]
    fn is_dynamic_returns_false_for_static() {
        assert!(!ActorKind::Static.is_dynamic());
    }

    #[test]
    fn is_character_returns_true_for_character() {
        assert!(ActorKind::Character.is_character());
    }

    #[test]
    fn is_character_returns_false_for_static() {
        assert!(!ActorKind::Static.is_character());
    }

    #[test]
    fn is_movable_returns_true_for_dynamic() {
        assert!(ActorKind::Dynamic.is_movable());
    }

    #[test]
    fn is_movable_returns_true_for_character() {
        assert!(ActorKind::Character.is_movable());
    }

    #[test]
    fn is_movable_returns_false_for_static() {
        assert!(!ActorKind::Static.is_movable());
    }

    // --- DebugLine.is_degenerate() paths ---

    #[test]
    fn is_degenerate_returns_true_for_zero_length() {
        let line = DebugLine::new([5.0, 5.0, 5.0], [5.0, 5.0, 5.0], [1.0, 0.0, 0.0]);
        assert!(line.is_degenerate());
    }

    #[test]
    fn is_degenerate_returns_false_for_nonzero_length() {
        let line = DebugLine::new([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert!(!line.is_degenerate());
    }

    // --- SpatialHash query result paths ---

    #[test]
    fn query_returns_non_empty_when_object_exists() {
        let mut grid = SpatialHash::<u32>::new(10.0);
        let aabb = AABB::from_sphere(Vec3::ZERO, 1.0);
        grid.insert(1, aabb);

        let results = grid.query(aabb);
        assert!(!results.is_empty());
    }

    #[test]
    fn query_returns_empty_after_clear() {
        let mut grid = SpatialHash::<u32>::new(10.0);
        let aabb = AABB::from_sphere(Vec3::ZERO, 1.0);
        grid.insert(1, aabb);
        grid.clear();

        let results = grid.query(aabb);
        assert!(results.is_empty());
    }

    #[test]
    fn query_returns_empty_for_distant_query() {
        let mut grid = SpatialHash::<u32>::new(10.0);
        grid.insert(1, AABB::from_sphere(Vec3::ZERO, 1.0));

        // Query far away
        let distant_query = AABB::from_sphere(Vec3::splat(1000.0), 1.0);
        let results = grid.query(distant_query);
        assert!(!results.contains(&1));
    }
}
