//! Mutation-killing tests for astraweave-scene.
//! These tests are designed to catch subtle mutations like boundary condition changes,
//! arithmetic operator substitutions, and conditional logic inversions.

use crate::*;
use glam::{Mat4, Quat, Vec3};
use std::f32::consts::PI;

// ============================================================================
// Transform Boundary Tests - Catch mutations in transform operations
// ============================================================================

#[cfg(test)]
mod transform_mutation_tests {
    use super::*;

    #[test]
    fn test_transform_new_preserves_exact_values() {
        let translation = Vec3::new(1.23456, -7.89012, 3.45678);
        let rotation = Quat::from_xyzw(0.1, 0.2, 0.3, 0.9).normalize();
        let scale = Vec3::new(0.5, 2.0, 1.5);

        let t = Transform::new(translation, rotation, scale);

        // Exact value preservation - catches field swaps
        assert_eq!(t.translation.x, 1.23456);
        assert_eq!(t.translation.y, -7.89012);
        assert_eq!(t.translation.z, 3.45678);
        assert_eq!(t.scale.x, 0.5);
        assert_eq!(t.scale.y, 2.0);
        assert_eq!(t.scale.z, 1.5);
    }

    #[test]
    fn test_identity_returns_correct_defaults() {
        let t = Transform::identity();

        // Ensure identity actually returns identity values
        assert_eq!(t.translation, Vec3::ZERO);
        assert_eq!(t.rotation, Quat::IDENTITY);
        assert_eq!(t.scale, Vec3::ONE);

        // Explicit checks for each component
        assert_eq!(t.translation.x, 0.0);
        assert_eq!(t.translation.y, 0.0);
        assert_eq!(t.translation.z, 0.0);
        assert_eq!(t.scale.x, 1.0);
        assert_eq!(t.scale.y, 1.0);
        assert_eq!(t.scale.z, 1.0);
    }

    #[test]
    fn test_from_translation_only_sets_translation() {
        let t = Transform::from_translation(Vec3::new(5.0, 10.0, 15.0));

        // Translation should be set
        assert_eq!(t.translation, Vec3::new(5.0, 10.0, 15.0));

        // Rotation and scale should be default
        assert_eq!(t.rotation, Quat::IDENTITY);
        assert_eq!(t.scale, Vec3::ONE);
    }

    #[test]
    fn test_from_rotation_only_sets_rotation() {
        let rotation = Quat::from_rotation_y(PI / 4.0);
        let t = Transform::from_rotation(rotation);

        // Rotation should be set
        assert!((t.rotation.dot(rotation) - 1.0).abs() < 0.0001);

        // Translation and scale should be default
        assert_eq!(t.translation, Vec3::ZERO);
        assert_eq!(t.scale, Vec3::ONE);
    }

    #[test]
    fn test_from_scale_creates_uniform_scale() {
        let t = Transform::from_scale(3.0);

        // Scale should be uniform
        assert_eq!(t.scale.x, 3.0);
        assert_eq!(t.scale.y, 3.0);
        assert_eq!(t.scale.z, 3.0);

        // Translation and rotation should be default
        assert_eq!(t.translation, Vec3::ZERO);
        assert_eq!(t.rotation, Quat::IDENTITY);
    }

    #[test]
    fn test_from_scale_vec_creates_non_uniform_scale() {
        let t = Transform::from_scale_vec(Vec3::new(1.0, 2.0, 3.0));

        // Each axis should be different
        assert_eq!(t.scale.x, 1.0);
        assert_eq!(t.scale.y, 2.0);
        assert_eq!(t.scale.z, 3.0);

        // Not uniform
        assert!(t.scale.x != t.scale.y);
        assert!(t.scale.y != t.scale.z);
    }

    #[test]
    fn test_is_identity_true_only_for_identity() {
        let identity = Transform::identity();
        assert!(identity.is_identity());

        // Any deviation should return false
        let mut t = Transform::identity();
        t.translation.x = 0.001;
        assert!(!t.is_identity());

        let mut t = Transform::identity();
        t.scale.y = 1.001;
        assert!(!t.is_identity());
    }

    #[test]
    fn test_is_uniform_scale_epsilon_handling() {
        // Exactly uniform
        let t = Transform::from_scale(2.0);
        assert!(t.is_uniform_scale());

        // Non-uniform
        let t = Transform::from_scale_vec(Vec3::new(2.0, 2.1, 2.0));
        assert!(!t.is_uniform_scale());

        // Very close but not equal (within epsilon)
        let t = Transform::from_scale_vec(Vec3::new(2.0, 2.0 + f32::EPSILON / 2.0, 2.0));
        assert!(t.is_uniform_scale());
    }

    #[test]
    fn test_uniform_scale_average_calculation() {
        let t = Transform::from_scale_vec(Vec3::new(1.0, 2.0, 3.0));
        let avg = t.uniform_scale();

        // Average of 1, 2, 3 = 2
        assert!((avg - 2.0).abs() < 0.0001);
    }

    #[test]
    fn test_forward_direction() {
        // Identity rotation: forward is -Z
        let t = Transform::identity();
        let fwd = t.forward();
        assert!((fwd - Vec3::NEG_Z).length() < 0.0001);

        // 90° rotation around Y: forward becomes -X
        let t = Transform::from_rotation(Quat::from_rotation_y(PI / 2.0));
        let fwd = t.forward();
        assert!((fwd - Vec3::NEG_X).length() < 0.001);
    }

    #[test]
    fn test_right_direction() {
        // Identity rotation: right is +X
        let t = Transform::identity();
        let right = t.right();
        assert!((right - Vec3::X).length() < 0.0001);
    }

    #[test]
    fn test_up_direction() {
        // Identity rotation: up is +Y
        let t = Transform::identity();
        let up = t.up();
        assert!((up - Vec3::Y).length() < 0.0001);
    }

    #[test]
    fn test_inverse_reverses_transform() {
        let t = Transform::new(
            Vec3::new(10.0, 20.0, 30.0),
            Quat::from_rotation_z(PI / 6.0),
            Vec3::new(2.0, 2.0, 2.0),
        );

        let inv = t.inverse();

        // Applying transform then inverse should return to original point
        let point = Vec3::new(1.0, 1.0, 1.0);
        let transformed = t.transform_point(point);
        let back = inv.transform_point(transformed);

        assert!((back - point).length() < 0.001);
    }

    #[test]
    fn test_transform_point_applies_scale_rotation_translation() {
        let t = Transform {
            translation: Vec3::new(10.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::new(2.0, 2.0, 2.0),
        };

        // Point (1, 0, 0) scaled by 2 = (2, 0, 0), then translated by (10, 0, 0) = (12, 0, 0)
        let result = t.transform_point(Vec3::new(1.0, 0.0, 0.0));
        assert!((result - Vec3::new(12.0, 0.0, 0.0)).length() < 0.0001);
    }

    #[test]
    fn test_transform_direction_ignores_translation() {
        let t = Transform {
            translation: Vec3::new(100.0, 100.0, 100.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };

        // Direction should not be affected by translation
        let result = t.transform_direction(Vec3::X);
        assert!((result - Vec3::X).length() < 0.0001);
    }

    #[test]
    fn test_lerp_at_boundaries() {
        let a = Transform::from_translation(Vec3::ZERO);
        let b = Transform::from_translation(Vec3::new(10.0, 10.0, 10.0));

        // t=0 should return a
        let result = a.lerp(&b, 0.0);
        assert!((result.translation - a.translation).length() < 0.0001);

        // t=1 should return b
        let result = a.lerp(&b, 1.0);
        assert!((result.translation - b.translation).length() < 0.0001);

        // t=0.5 should be midpoint
        let result = a.lerp(&b, 0.5);
        assert!((result.translation - Vec3::new(5.0, 5.0, 5.0)).length() < 0.0001);
    }
}

// ============================================================================
// Node Mutation Tests - Catch mutations in tree operations
// ============================================================================

#[cfg(test)]
mod node_mutation_tests {
    use super::*;

    #[test]
    fn test_node_new_creates_correct_name() {
        let node = Node::new("test_name");
        assert_eq!(node.name, "test_name");
        assert!(!node.name.is_empty());
        assert!(node.name != "other_name");
    }

    #[test]
    fn test_node_with_transform_sets_both() {
        let transform = Transform::from_translation(Vec3::new(5.0, 5.0, 5.0));
        let node = Node::with_transform("transformed", transform);

        assert_eq!(node.name, "transformed");
        assert_eq!(node.transform.translation, Vec3::new(5.0, 5.0, 5.0));
    }

    #[test]
    fn test_has_children_boundary() {
        let mut node = Node::new("parent");

        // No children
        assert!(!node.has_children());

        // One child
        node.add_child(Node::new("child"));
        assert!(node.has_children());

        // Multiple children
        node.add_child(Node::new("child2"));
        assert!(node.has_children());
    }

    #[test]
    fn test_child_count_accuracy() {
        let mut node = Node::new("parent");

        assert_eq!(node.child_count(), 0);

        node.add_child(Node::new("c1"));
        assert_eq!(node.child_count(), 1);

        node.add_child(Node::new("c2"));
        assert_eq!(node.child_count(), 2);

        node.add_child(Node::new("c3"));
        assert_eq!(node.child_count(), 3);
    }

    #[test]
    fn test_descendant_count_recursive() {
        let mut root = Node::new("root");

        // No descendants
        assert_eq!(root.descendant_count(), 0);

        // Add one child
        root.add_child(Node::new("c1"));
        assert_eq!(root.descendant_count(), 1);

        // Add grandchild
        root.children[0].add_child(Node::new("gc1"));
        assert_eq!(root.descendant_count(), 2);

        // Add another child with grandchildren
        let mut c2 = Node::new("c2");
        c2.add_child(Node::new("gc2"));
        c2.add_child(Node::new("gc3"));
        root.add_child(c2);
        assert_eq!(root.descendant_count(), 5);
    }

    #[test]
    fn test_is_leaf_boundary() {
        let mut node = Node::new("node");

        // Initially a leaf
        assert!(node.is_leaf());

        // After adding child, not a leaf
        node.add_child(Node::new("child"));
        assert!(!node.is_leaf());
    }

    #[test]
    fn test_find_child_exact_match() {
        let mut parent = Node::new("parent");
        parent.add_child(Node::new("alpha"));
        parent.add_child(Node::new("beta"));
        parent.add_child(Node::new("gamma"));

        // Exact match
        assert!(parent.find_child("alpha").is_some());
        assert!(parent.find_child("beta").is_some());

        // No match
        assert!(parent.find_child("delta").is_none());
        assert!(parent.find_child("Alpha").is_none()); // Case sensitive
        assert!(parent.find_child("").is_none());
    }

    #[test]
    fn test_find_child_mut_allows_modification() {
        let mut parent = Node::new("parent");
        parent.add_child(Node::new("child"));

        if let Some(child) = parent.find_child_mut("child") {
            child.transform.translation = Vec3::new(1.0, 2.0, 3.0);
        }

        // Verify modification
        let child = parent.find_child("child").unwrap();
        assert_eq!(child.transform.translation, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_find_descendant_recursive_search() {
        let mut root = Node::new("root");
        let mut c1 = Node::new("c1");
        c1.add_child(Node::new("gc1"));
        c1.children[0].add_child(Node::new("ggc1"));
        root.add_child(c1);

        // Find at each level
        assert!(root.find_descendant("c1").is_some());
        assert!(root.find_descendant("gc1").is_some());
        assert!(root.find_descendant("ggc1").is_some());

        // Not found
        assert!(root.find_descendant("nonexistent").is_none());
    }

    #[test]
    fn test_depth_calculation() {
        let mut root = Node::new("root");

        // Leaf node has depth 0
        assert_eq!(root.depth(), 0);

        // Add one level
        root.add_child(Node::new("c1"));
        assert_eq!(root.depth(), 1);

        // Add second level
        root.children[0].add_child(Node::new("gc1"));
        assert_eq!(root.depth(), 2);

        // Add third level
        root.children[0].children[0].add_child(Node::new("ggc1"));
        assert_eq!(root.depth(), 3);
    }

    #[test]
    fn test_node_display_format() {
        let leaf = Node::new("leaf");
        assert!(format!("{}", leaf).contains("leaf"));

        let mut parent = Node::new("parent");
        parent.add_child(Node::new("c1"));
        parent.add_child(Node::new("c2"));
        let display = format!("{}", parent);
        assert!(display.contains("parent"));
        assert!(display.contains("2 children"));
    }
}

// ============================================================================
// Scene Mutation Tests - Catch mutations in scene graph operations
// ============================================================================

#[cfg(test)]
mod scene_mutation_tests {
    use super::*;

    #[test]
    fn test_scene_new_creates_root() {
        let scene = Scene::new();
        assert_eq!(scene.root.name, "root");
        assert!(scene.root.children.is_empty());
    }

    #[test]
    fn test_scene_with_root_uses_custom_root() {
        let custom_root = Node::new("custom_root");
        let scene = Scene::with_root(custom_root);

        assert_eq!(scene.root.name, "custom_root");
    }

    #[test]
    fn test_node_count_includes_root() {
        let scene = Scene::new();

        // Just root
        assert_eq!(scene.node_count(), 1);
    }

    #[test]
    fn test_node_count_recursive() {
        let mut scene = Scene::new();
        scene.root.add_child(Node::new("c1"));
        scene.root.add_child(Node::new("c2"));
        scene.root.children[0].add_child(Node::new("gc1"));

        // root + 2 children + 1 grandchild = 4
        assert_eq!(scene.node_count(), 4);
    }

    #[test]
    fn test_is_empty_boundary() {
        let scene = Scene::new();
        assert!(scene.is_empty());

        let mut scene = Scene::new();
        scene.root.add_child(Node::new("child"));
        assert!(!scene.is_empty());
    }

    #[test]
    fn test_scene_depth() {
        let mut scene = Scene::new();
        assert_eq!(scene.depth(), 0);

        scene.root.add_child(Node::new("c1"));
        assert_eq!(scene.depth(), 1);

        scene.root.children[0].add_child(Node::new("gc1"));
        assert_eq!(scene.depth(), 2);
    }

    #[test]
    fn test_find_node_finds_root() {
        let scene = Scene::new();

        let found = scene.find_node("root");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "root");
    }

    #[test]
    fn test_find_node_finds_descendants() {
        let mut scene = Scene::new();
        scene.root.add_child(Node::new("child"));
        scene.root.children[0].add_child(Node::new("grandchild"));

        assert!(scene.find_node("child").is_some());
        assert!(scene.find_node("grandchild").is_some());
        assert!(scene.find_node("nonexistent").is_none());
    }

    #[test]
    fn test_traverse_visits_all_nodes() {
        let mut scene = Scene::new();
        scene.root.add_child(Node::new("c1"));
        scene.root.add_child(Node::new("c2"));

        let mut visited = Vec::new();
        scene.traverse(&mut |node, _| {
            visited.push(node.name.clone());
        });

        assert_eq!(visited.len(), 3);
        assert!(visited.contains(&"root".to_string()));
        assert!(visited.contains(&"c1".to_string()));
        assert!(visited.contains(&"c2".to_string()));
    }

    #[test]
    fn test_traverse_computes_world_transforms() {
        let mut scene = Scene::new();

        // Root at origin
        // Child translated by (10, 0, 0)
        let mut child = Node::new("child");
        child.transform.translation = Vec3::new(10.0, 0.0, 0.0);
        scene.root.add_child(child);

        let mut child_world = Mat4::IDENTITY;
        scene.traverse(&mut |node, world| {
            if node.name == "child" {
                child_world = world;
            }
        });

        // Child's world position should be (10, 0, 0)
        let (_, _, translation) = child_world.to_scale_rotation_translation();
        assert!((translation - Vec3::new(10.0, 0.0, 0.0)).length() < 0.0001);
    }

    #[test]
    fn test_traverse_with_path_tracks_path() {
        let mut scene = Scene::new();
        let mut c1 = Node::new("c1");
        c1.add_child(Node::new("gc1"));
        scene.root.add_child(c1);

        let mut gc1_path = Vec::new();
        scene.traverse_with_path(&mut |node, _, path| {
            if node.name == "gc1" {
                gc1_path = path.iter().map(|s| s.to_string()).collect();
            }
        });

        assert_eq!(gc1_path, vec!["root", "c1", "gc1"]);
    }

    /// Catches `* → +` in traverse_with_path world matrix computation (L344):
    /// `parent * n.transform.matrix()` must use matrix multiplication, not addition.
    /// Uses a parent with ROTATION so that multiplication and addition give different results.
    #[test]
    fn mutation_traverse_with_path_world_transform() {
        let mut scene = Scene::new();
        // Parent: rotated 90° around Y + translated (10, 0, 0)
        scene.root.transform = Transform::new(
            Vec3::new(10.0, 0.0, 0.0),
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::ONE,
        );

        let mut child = Node::new("child");
        child.transform.translation = Vec3::new(5.0, 0.0, 0.0);
        scene.root.add_child(child);

        let mut child_world_pos = Vec3::ZERO;
        scene.traverse_with_path(&mut |node, world_mat, _path| {
            if node.name == "child" {
                let (_, _, translation) = world_mat.to_scale_rotation_translation();
                child_world_pos = translation;
            }
        });

        // With matrix multiplication: child local (5,0,0) is rotated 90° Y by parent → (0,0,-5)
        // then translated by parent (10,0,0) → world (10, 0, -5).
        // With matrix addition: translation column would be (15, 0, 0) — completely wrong.
        assert!(
            (child_world_pos.x - 10.0).abs() < 0.01,
            "world x must be ~10.0, got {:?}",
            child_world_pos
        );
        assert!(
            (child_world_pos.z - (-5.0)).abs() < 0.01,
            "world z must be ~-5.0 (rotated), got {:?}",
            child_world_pos
        );
    }

    #[test]
    fn test_scene_display_format() {
        let mut scene = Scene::new();
        scene.root.add_child(Node::new("c1"));
        scene.root.add_child(Node::new("c2"));

        let display = format!("{}", scene);
        assert!(display.contains("Scene"));
        assert!(display.contains("3 nodes"));
        assert!(display.contains("depth=1"));
    }
}

// ============================================================================
// ECS Component Mutation Tests (when ecs feature is enabled)
// ============================================================================

#[cfg(all(test, feature = "ecs"))]
#[allow(unused_imports, clippy::field_reassign_with_default)]
mod ecs_mutation_tests {
    use super::*;
    use crate::ecs::*;

    #[test]
    fn test_playback_state_is_playing() {
        assert!(PlaybackState::Playing.is_playing());
        assert!(!PlaybackState::Paused.is_playing());
        assert!(!PlaybackState::Stopped.is_playing());
    }

    #[test]
    fn test_playback_state_is_paused() {
        assert!(!PlaybackState::Playing.is_paused());
        assert!(PlaybackState::Paused.is_paused());
        assert!(!PlaybackState::Stopped.is_paused());
    }

    #[test]
    fn test_playback_state_is_stopped() {
        assert!(!PlaybackState::Playing.is_stopped());
        assert!(!PlaybackState::Paused.is_stopped());
        assert!(PlaybackState::Stopped.is_stopped());
    }

    #[test]
    fn test_playback_state_is_active() {
        assert!(PlaybackState::Playing.is_active());
        assert!(PlaybackState::Paused.is_active());
        assert!(!PlaybackState::Stopped.is_active());
    }

    #[test]
    fn test_playback_state_all() {
        let all = PlaybackState::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&PlaybackState::Playing));
        assert!(all.contains(&PlaybackState::Paused));
        assert!(all.contains(&PlaybackState::Stopped));
    }

    #[test]
    fn test_animator_default() {
        let anim = CAnimator::default();
        assert_eq!(anim.clip_index, 0);
        assert_eq!(anim.time, 0.0);
        assert_eq!(anim.speed, 1.0);
        assert_eq!(anim.state, PlaybackState::Stopped);
        assert!(anim.looping);
    }

    #[test]
    fn test_animator_new() {
        let anim = CAnimator::new(5);
        assert_eq!(anim.clip_index, 5);
        assert_eq!(anim.state, PlaybackState::Stopped);
    }

    #[test]
    fn test_animator_play_sets_state() {
        let mut anim = CAnimator::default();
        assert_eq!(anim.state, PlaybackState::Stopped);

        anim.play();
        assert_eq!(anim.state, PlaybackState::Playing);
    }

    #[test]
    fn test_animator_pause_only_when_playing() {
        let mut anim = CAnimator::default();

        // Pause from stopped does nothing
        anim.pause();
        assert_eq!(anim.state, PlaybackState::Stopped);

        // Pause from playing works
        anim.play();
        anim.pause();
        assert_eq!(anim.state, PlaybackState::Paused);
    }

    #[test]
    fn test_animator_stop_resets_time() {
        let mut anim = CAnimator::default();
        anim.play();
        anim.time = 5.0;

        anim.stop();
        assert_eq!(anim.state, PlaybackState::Stopped);
        assert_eq!(anim.time, 0.0);
    }

    #[test]
    fn test_animator_toggle_pause() {
        let mut anim = CAnimator::default();

        // From stopped -> playing
        anim.toggle_pause();
        assert_eq!(anim.state, PlaybackState::Playing);

        // From playing -> paused
        anim.toggle_pause();
        assert_eq!(anim.state, PlaybackState::Paused);

        // From paused -> playing
        anim.toggle_pause();
        assert_eq!(anim.state, PlaybackState::Playing);
    }

    #[test]
    fn test_animator_with_speed() {
        let anim = CAnimator::new(0).with_speed(2.0);
        assert_eq!(anim.speed, 2.0);
    }

    #[test]
    fn test_animator_with_looping() {
        let anim = CAnimator::new(0).with_looping(false);
        assert!(!anim.looping);
    }

    #[test]
    fn test_animator_reset_clears_time() {
        let mut anim = CAnimator::default();
        anim.time = 10.0;

        anim.reset();
        assert_eq!(anim.time, 0.0);
    }
}

// ============================================================================
// Transform Matrix Tests - Ensure matrix operations are correct
// ============================================================================

#[cfg(test)]
mod matrix_mutation_tests {
    use super::*;

    #[test]
    fn test_matrix_order_is_scale_rotation_translation() {
        let t = Transform {
            translation: Vec3::new(5.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::new(2.0, 2.0, 2.0),
        };

        let m = t.matrix();

        // Transform point (1, 0, 0):
        // Scale: (2, 0, 0)
        // Rotate: (2, 0, 0) (identity)
        // Translate: (7, 0, 0)
        let result = m.transform_point3(Vec3::new(1.0, 0.0, 0.0));
        assert!((result.x - 7.0).abs() < 0.0001);
    }

    #[test]
    fn test_matrix_handles_negative_scale() {
        let t = Transform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::new(-1.0, 1.0, 1.0),
        };

        let m = t.matrix();
        let result = m.transform_point3(Vec3::new(1.0, 0.0, 0.0));

        // Negative X scale should flip X
        assert!((result.x - (-1.0)).abs() < 0.0001);
    }

    #[test]
    fn test_matrix_zero_scale() {
        let t = Transform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ZERO,
        };

        let m = t.matrix();
        let result = m.transform_point3(Vec3::new(100.0, 100.0, 100.0));

        // Zero scale collapses everything to origin
        assert!((result).length() < 0.0001);
    }
}

// ============================================================================
// Behavioral Correctness Tests - Scene Graph & Transform Invariants
// ============================================================================

#[cfg(test)]
mod behavioral_correctness_tests {
    use super::*;

    #[test]
    fn test_identity_transform_preserves_points() {
        // Behavioral: identity transform should not change any point
        let t = Transform::identity();
        let m = t.matrix();

        let test_points = [
            Vec3::ZERO,
            Vec3::ONE,
            Vec3::new(100.0, -50.0, 25.0),
            Vec3::new(-1.0, -1.0, -1.0),
        ];

        for point in test_points {
            let result = m.transform_point3(point);
            assert!(
                (result - point).length() < 0.0001,
                "Identity should preserve point {:?}",
                point
            );
        }
    }

    #[test]
    fn test_transform_matrix_determinant_positive_for_valid_scale() {
        // Behavioral: matrix with positive scale should have positive determinant
        let t = Transform {
            translation: Vec3::new(10.0, 20.0, 30.0),
            rotation: Quat::from_rotation_y(PI / 4.0),
            scale: Vec3::new(1.0, 2.0, 3.0),
        };

        let m = t.matrix();
        let det = m.determinant();

        assert!(
            det > 0.0,
            "Determinant should be positive for positive scales: {}",
            det
        );
    }

    #[test]
    fn test_transform_matrix_determinant_equals_scale_product() {
        // Behavioral: determinant should equal product of scales (for no rotation shear)
        let t = Transform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::new(2.0, 3.0, 4.0),
        };

        let m = t.matrix();
        let det = m.determinant();
        let expected = 2.0 * 3.0 * 4.0;

        assert!(
            (det - expected).abs() < 0.001,
            "Determinant should equal scale product: expected {}, got {}",
            expected,
            det
        );
    }

    #[test]
    fn test_rotation_preserves_length() {
        // Behavioral: pure rotation should preserve vector length
        let t = Transform {
            translation: Vec3::ZERO,
            rotation: Quat::from_rotation_y(PI / 3.0) * Quat::from_rotation_x(PI / 6.0),
            scale: Vec3::ONE,
        };

        let m = t.matrix();
        let original = Vec3::new(1.0, 2.0, 3.0);
        let original_len = original.length();

        let rotated = m.transform_vector3(original);
        let rotated_len = rotated.length();

        assert!(
            (original_len - rotated_len).abs() < 0.0001,
            "Rotation should preserve length: {} vs {}",
            original_len,
            rotated_len
        );
    }

    #[test]
    fn test_translation_only_changes_position() {
        // Behavioral: translation should only affect point position, not direction
        let t = Transform::from_translation(Vec3::new(100.0, 200.0, 300.0));
        let m = t.matrix();

        // Direction vector should be unchanged
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let transformed_dir = m.transform_vector3(dir);

        assert!(
            (transformed_dir - dir).length() < 0.0001,
            "Translation should not affect directions"
        );
    }

    #[test]
    fn test_uniform_scale_preserves_proportions() {
        // Behavioral: uniform scale should preserve proportions
        let t = Transform::from_scale(3.0);
        let m = t.matrix();

        let v = Vec3::new(1.0, 2.0, 3.0);
        let scaled = m.transform_vector3(v);

        // Ratios should be preserved
        assert!(
            (scaled.x / scaled.y - v.x / v.y).abs() < 0.0001,
            "X/Y ratio should be preserved"
        );
        assert!(
            (scaled.y / scaled.z - v.y / v.z).abs() < 0.0001,
            "Y/Z ratio should be preserved"
        );
    }

    #[test]
    fn test_transform_composition_is_associative() {
        // Behavioral: (A * B) * C == A * (B * C) for matrices
        let a = Transform::from_translation(Vec3::X * 5.0).matrix();
        let b = Transform::from_rotation(Quat::from_rotation_y(PI / 4.0)).matrix();
        let c = Transform::from_scale(2.0).matrix();

        let left = (a * b) * c;
        let right = a * (b * c);

        // Compare all elements
        for i in 0..4 {
            for j in 0..4 {
                let l = left.col(i)[j];
                let r = right.col(i)[j];
                assert!(
                    (l - r).abs() < 0.0001,
                    "Matrix multiplication should be associative at ({},{}): {} vs {}",
                    i,
                    j,
                    l,
                    r
                );
            }
        }
    }

    #[test]
    fn test_scale_by_one_is_identity() {
        // Behavioral: scaling by 1 should be equivalent to identity
        let t = Transform::from_scale(1.0);
        let identity = Transform::identity();

        let point = Vec3::new(7.0, 8.0, 9.0);
        let result_scaled = t.matrix().transform_point3(point);
        let result_identity = identity.matrix().transform_point3(point);

        assert!(
            (result_scaled - result_identity).length() < 0.0001,
            "Scale by 1 should equal identity transform"
        );
    }

    // =====================================================================
    // Additional mutation-killing tests — catches && → || and < → <=
    // =====================================================================

    /// Catches `&& → ||` in is_uniform_scale:
    /// With (2.0, 2.0, 999.0) the first condition `|x-y| < ε` is true,
    /// second condition `|y-z| < ε` is false. `true && false = false`,
    /// but `true || false = true`. We assert false.
    #[test]
    fn mutation_is_uniform_scale_and_vs_or() {
        // x == y but y != z
        let t = Transform::from_scale_vec(Vec3::new(2.0, 2.0, 999.0));
        assert!(!t.is_uniform_scale(), "x==y but y!=z must NOT be uniform");

        // Reverse asymmetry: x != y but y == z
        let t2 = Transform::from_scale_vec(Vec3::new(999.0, 2.0, 2.0));
        assert!(!t2.is_uniform_scale(), "x!=y but y==z must NOT be uniform");
    }

    /// Catches `< → <=` on line 114: `(x - y).abs() < EPSILON`
    /// If |x - y| == EPSILON exactly, `<` returns false but `<=` returns true.
    #[test]
    fn mutation_is_uniform_scale_epsilon_boundary_xy() {
        // Scale where |x - y| == EPSILON exactly, y == z
        let eps = f32::EPSILON;
        let t = Transform::from_scale_vec(Vec3::new(1.0, 1.0 + eps, 1.0 + eps));
        // |x - y| = eps — not strictly less than eps
        assert!(
            !t.is_uniform_scale(),
            "|x-y| == EPSILON must return false (< not <=)"
        );
    }

    /// Catches `< → <=` on line 115: `(y - z).abs() < EPSILON`
    /// If |y - z| == EPSILON exactly, `<` returns false but `<=` returns true.
    #[test]
    fn mutation_is_uniform_scale_epsilon_boundary_yz() {
        // Scale where x == y, |y - z| == EPSILON exactly
        let eps = f32::EPSILON;
        let t = Transform::from_scale_vec(Vec3::new(1.0, 1.0, 1.0 + eps));
        // |y - z| = eps — not strictly less than eps
        assert!(
            !t.is_uniform_scale(),
            "|y-z| == EPSILON must return false (< not <=)"
        );
    }
}

// ============================================================================
// World Partition Mutation Tests — AABB, Frustum, World Partition
// ============================================================================

#[cfg(test)]
mod world_partition_mutation_tests {
    use crate::world_partition::*;
    use glam::{Mat4, Vec3, Vec4};

    // ── AABB half_extents: catches `- → +` ──

    /// half_extents = (max - min) * 0.5. If mutated to (max + min) * 0.5, the result
    /// changes when min ≠ 0. With AABB(2,4,6 → 10,20,30):
    ///   correct: (4, 8, 12), mutant: (6, 12, 18)
    #[test]
    fn mutation_half_extents_minus_vs_plus() {
        let aabb = AABB::new(Vec3::new(2.0, 4.0, 6.0), Vec3::new(10.0, 20.0, 30.0));
        let half = aabb.half_extents();
        assert!(
            (half - Vec3::new(4.0, 8.0, 12.0)).length() < 0.0001,
            "half_extents must use subtraction, got {:?}",
            half
        );
    }

    // ── AABB from_center_half_extents: catches `- → +` and `+ → -` ──

    /// min = center - half_extents. If `- → +`, min becomes center + half_extents.
    /// max = center + half_extents. If `+ → -`, max becomes center - half_extents.
    #[test]
    fn mutation_from_center_half_extents_operators() {
        let center = Vec3::new(5.0, 10.0, 15.0);
        let half = Vec3::new(2.0, 3.0, 4.0);
        let aabb = AABB::from_center_half_extents(center, half);
        // min = (3, 7, 11), max = (7, 13, 19)
        assert!(
            (aabb.min - Vec3::new(3.0, 7.0, 11.0)).length() < 0.0001,
            "min must be center - half_extents, got {:?}",
            aabb.min
        );
        assert!(
            (aabb.max - Vec3::new(7.0, 13.0, 19.0)).length() < 0.0001,
            "max must be center + half_extents, got {:?}",
            aabb.max
        );
    }

    // ── AABB center: catches `+ → -` and `* → /` etc. ──

    #[test]
    fn mutation_aabb_center_formula() {
        let aabb = AABB::new(Vec3::new(2.0, 4.0, 6.0), Vec3::new(10.0, 20.0, 30.0));
        let center = aabb.center();
        // (2+10)/2=6, (4+20)/2=12, (6+30)/2=18
        assert!(
            (center - Vec3::new(6.0, 12.0, 18.0)).length() < 0.0001,
            "center must be (min+max)*0.5, got {:?}",
            center
        );
    }

    // ── AABB intersects: catches `&& → ||` per axis ──
    // Need boxes that overlap on all axes EXCEPT one specific axis.

    /// Boxes overlap on Y and Z but NOT on X. Tests `&&` between x-conditions and y-conditions.
    #[test]
    fn mutation_intersects_separated_on_x_only() {
        let a = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(5.0, 10.0, 10.0));
        let b = AABB::new(Vec3::new(10.0, 0.0, 0.0), Vec3::new(20.0, 10.0, 10.0));
        // Separate on X (a.max.x=5 < b.min.x=10), overlap on Y and Z.
        assert!(!a.intersects(&b), "Boxes separated on X must not intersect");
        assert!(
            !b.intersects(&a),
            "Boxes separated on X (reversed) must not intersect"
        );
    }

    /// Boxes overlap on X and Z but NOT on Y. Tests `&&` for y-conditions.
    #[test]
    fn mutation_intersects_separated_on_y_only() {
        let a = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(10.0, 5.0, 10.0));
        let b = AABB::new(Vec3::new(0.0, 10.0, 0.0), Vec3::new(10.0, 20.0, 10.0));
        // Separate on Y only.
        assert!(!a.intersects(&b), "Boxes separated on Y must not intersect");
    }

    /// Boxes overlap on X and Y but NOT on Z. Tests `&&` for z-conditions.
    #[test]
    fn mutation_intersects_separated_on_z_only() {
        let a = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(10.0, 10.0, 5.0));
        let b = AABB::new(Vec3::new(0.0, 0.0, 10.0), Vec3::new(10.0, 10.0, 20.0));
        // Separate on Z only.
        assert!(!a.intersects(&b), "Boxes separated on Z must not intersect");
    }

    /// Catches `<= → <` in intersects by testing touching boundaries per axis.
    #[test]
    fn mutation_intersects_touching_each_axis() {
        // Touch on X boundary: a.max.x == b.min.x
        let a = AABB::new(Vec3::ZERO, Vec3::new(10.0, 10.0, 10.0));
        let b_x = AABB::new(Vec3::new(10.0, 0.0, 0.0), Vec3::new(20.0, 10.0, 10.0));
        assert!(a.intersects(&b_x), "Touching on X must intersect");

        // Touch on Y boundary
        let b_y = AABB::new(Vec3::new(0.0, 10.0, 0.0), Vec3::new(10.0, 20.0, 10.0));
        assert!(a.intersects(&b_y), "Touching on Y must intersect");

        // Touch on Z boundary
        let b_z = AABB::new(Vec3::new(0.0, 0.0, 10.0), Vec3::new(10.0, 10.0, 20.0));
        assert!(a.intersects(&b_z), "Touching on Z must intersect");
    }

    /// Catches `>= → >` in intersects by testing reversed touching boundaries.
    #[test]
    fn mutation_intersects_touching_reversed() {
        // b.max.x == a.min.x
        let a = AABB::new(Vec3::new(10.0, 10.0, 10.0), Vec3::new(20.0, 20.0, 20.0));
        let b = AABB::new(Vec3::ZERO, Vec3::new(10.0, 10.0, 10.0));
        assert!(a.intersects(&b), "Reversed touching must intersect");
    }

    // ── AABB contains_point: catches `>= → >` and `<= → <` ──

    #[test]
    fn mutation_contains_point_boundaries() {
        let aabb = AABB::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(10.0, 10.0, 10.0));

        // Each axis boundary — catches >= → > and <= → <
        assert!(
            aabb.contains_point(Vec3::new(0.0, 5.0, 5.0)),
            "min.x boundary"
        );
        assert!(
            aabb.contains_point(Vec3::new(10.0, 5.0, 5.0)),
            "max.x boundary"
        );
        assert!(
            aabb.contains_point(Vec3::new(5.0, 0.0, 5.0)),
            "min.y boundary"
        );
        assert!(
            aabb.contains_point(Vec3::new(5.0, 10.0, 5.0)),
            "max.y boundary"
        );
        assert!(
            aabb.contains_point(Vec3::new(5.0, 5.0, 0.0)),
            "min.z boundary"
        );
        assert!(
            aabb.contains_point(Vec3::new(5.0, 5.0, 10.0)),
            "max.z boundary"
        );
    }

    // ── Frustum plane extraction: catches `+ → -` and `- → +` in from_view_projection ──

    /// Uses a known orthographic matrix to verify frustum planes point inward.
    /// We test that points JUST inside each plane pass AND points JUST outside fail.
    /// This catches any arithmetic mutation in plane extraction.
    #[test]
    fn mutation_frustum_planes_detect_inside_outside() {
        // Orthographic: visible region is x∈[-10,10], y∈[-10,10], z∈[-0.1, -100] (RH)
        let proj = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
        let frustum = Frustum::from_view_projection(proj);

        // Inside near center
        let inside_center =
            AABB::from_center_half_extents(Vec3::new(0.0, 0.0, -50.0), Vec3::splat(1.0));
        assert!(frustum.intersects_aabb(&inside_center), "center of frustum");

        // Outside left plane (x < -10)
        let outside_left =
            AABB::from_center_half_extents(Vec3::new(-20.0, 0.0, -50.0), Vec3::splat(1.0));
        assert!(
            !frustum.intersects_aabb(&outside_left),
            "outside left plane"
        );

        // Outside right plane (x > 10)
        let outside_right =
            AABB::from_center_half_extents(Vec3::new(20.0, 0.0, -50.0), Vec3::splat(1.0));
        assert!(
            !frustum.intersects_aabb(&outside_right),
            "outside right plane"
        );

        // Outside bottom plane (y < -10)
        let outside_bottom =
            AABB::from_center_half_extents(Vec3::new(0.0, -20.0, -50.0), Vec3::splat(1.0));
        assert!(
            !frustum.intersects_aabb(&outside_bottom),
            "outside bottom plane"
        );

        // Outside top plane (y > 10)
        let outside_top =
            AABB::from_center_half_extents(Vec3::new(0.0, 20.0, -50.0), Vec3::splat(1.0));
        assert!(!frustum.intersects_aabb(&outside_top), "outside top plane");

        // Outside far plane (z < -100 means further than far)
        let outside_far =
            AABB::from_center_half_extents(Vec3::new(0.0, 0.0, -200.0), Vec3::splat(1.0));
        assert!(!frustum.intersects_aabb(&outside_far), "outside far plane");
        // Note: near plane not tested here — the Gribb-Hartmann extraction formula
        // assumes OpenGL [-1,1] z range but glam::orthographic_rh uses [0,1] (wgpu).
        // Near plane is tested via the perspective frustum test below instead.
    }

    /// Individual frustum plane normals should point correctly.
    /// Tests with a perspective projection to cause different arithmetic paths.
    #[test]
    fn mutation_frustum_planes_perspective() {
        let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
        let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        let vp = proj * view;
        let frustum = Frustum::from_view_projection(vp);

        // Object directly ahead at z=-50 should be inside
        let ahead = AABB::from_center_half_extents(Vec3::new(0.0, 0.0, -50.0), Vec3::splat(1.0));
        assert!(frustum.intersects_aabb(&ahead), "ahead in perspective");

        // Object far to the left should be outside
        let far_left =
            AABB::from_center_half_extents(Vec3::new(-500.0, 0.0, -50.0), Vec3::splat(1.0));
        assert!(
            !frustum.intersects_aabb(&far_left),
            "far left in perspective"
        );

        // Object far to the right should be outside
        let far_right =
            AABB::from_center_half_extents(Vec3::new(500.0, 0.0, -50.0), Vec3::splat(1.0));
        assert!(
            !frustum.intersects_aabb(&far_right),
            "far right in perspective"
        );

        // Object far above should be outside
        let far_up = AABB::from_center_half_extents(Vec3::new(0.0, 500.0, -50.0), Vec3::splat(1.0));
        assert!(!frustum.intersects_aabb(&far_up), "far up in perspective");

        // Object behind camera should be outside (z > 0)
        let behind = AABB::from_center_half_extents(Vec3::new(0.0, 0.0, 50.0), Vec3::splat(1.0));
        assert!(
            !frustum.intersects_aabb(&behind),
            "behind camera in perspective"
        );
    }

    /// Catches `>= → <` in intersects_aabb positive vertex selection.
    /// Uses a box that is right at the frustum boundary on one axis — if the
    /// positive vertex selection is wrong, the box will be incorrectly culled.
    #[test]
    fn mutation_frustum_intersects_aabb_boundary_boxes() {
        let proj = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
        let frustum = Frustum::from_view_projection(proj);

        // Box that straddles the left boundary (partially inside)
        let straddle_left = AABB::new(Vec3::new(-12.0, -5.0, -50.0), Vec3::new(-8.0, 5.0, -40.0));
        assert!(
            frustum.intersects_aabb(&straddle_left),
            "Box straddling left boundary should intersect"
        );

        // Box that straddles the bottom boundary (partially inside)
        let straddle_bottom = AABB::new(Vec3::new(-5.0, -12.0, -50.0), Vec3::new(5.0, -8.0, -40.0));
        assert!(
            frustum.intersects_aabb(&straddle_bottom),
            "Box straddling bottom boundary should intersect"
        );
    }

    // ── cells_in_frustum: catches `+ → -` and `+ → *` in loop coordinates ──

    #[test]
    fn mutation_cells_in_frustum_loop_arithmetic() {
        // Wide orthographic frustum so all nearby cells are visible
        let proj = Mat4::orthographic_rh(-500.0, 500.0, -500.0, 500.0, 0.1, 500.0);
        let frustum = Frustum::from_view_projection(proj);

        // Camera at cell (2, 0, 3), radius covers 1 cell
        let camera_pos = Vec3::new(250.0, 50.0, 350.0); // Cell (2, 0, 3) for size=100
        let cell_size = 100.0;
        let cells = frustum.cells_in_frustum(camera_pos, cell_size, cell_size * 1.5);

        // Camera cell (2, 0, 3) must be present
        assert!(
            cells.contains(&GridCoord::new(2, 0, 3)),
            "Camera cell must be in frustum"
        );

        // Adjacent cells should be present (catches + → - in coordinate offsets)
        assert!(
            cells.contains(&GridCoord::new(3, 0, 3)),
            "Cell +x must be in frustum"
        );
        assert!(
            cells.contains(&GridCoord::new(1, 0, 3)),
            "Cell -x must be in frustum"
        );
        assert!(
            cells.contains(&GridCoord::new(2, 0, 4)),
            "Cell +z must be in frustum"
        );
        assert!(
            cells.contains(&GridCoord::new(2, 0, 2)),
            "Cell -z must be in frustum"
        );
        assert!(
            cells.contains(&GridCoord::new(2, 1, 3)),
            "Cell +y must be in frustum"
        );
        assert!(
            cells.contains(&GridCoord::new(2, -1, 3)),
            "Cell -y must be in frustum"
        );
    }

    // ── GridCoord::to_world_center: catches `+ → -` and `* → +` ──

    #[test]
    fn mutation_to_world_center_arithmetic() {
        // For cell (2, 3, -1) with size 100: center = (2.5*100, 3.5*100, -0.5*100) = (250, 350, -50)
        let coord = GridCoord::new(2, 3, -1);
        let center = coord.to_world_center(100.0);
        assert!(
            (center - Vec3::new(250.0, 350.0, -50.0)).length() < 0.01,
            "to_world_center arithmetic wrong, got {:?}",
            center
        );
    }

    // ── GridCoord::from_world_pos: catches `/ → *` ──

    #[test]
    fn mutation_from_world_pos_division() {
        let coord = GridCoord::from_world_pos(Vec3::new(250.0, 350.0, -50.0), 100.0);
        assert_eq!(coord.x, 2);
        assert_eq!(coord.y, 3);
        assert_eq!(coord.z, -1);
    }

    // ── GridCoord::manhattan_distance: catches `+ → -`, `- → +`, `.abs() → delete` ──

    #[test]
    fn mutation_manhattan_distance_components() {
        let a = GridCoord::new(1, 2, 3);
        let b = GridCoord::new(4, 1, 7);
        // |1-4| + |2-1| + |3-7| = 3 + 1 + 4 = 8
        assert_eq!(a.manhattan_distance(b), 8);

        // Negative coordinates
        let c = GridCoord::new(-3, -5, 2);
        let d = GridCoord::new(2, 1, -1);
        // |(-3)-2| + |(-5)-1| + |2-(-1)| = 5 + 6 + 3 = 14
        assert_eq!(c.manhattan_distance(d), 14);
    }

    // ── LRU eviction order: catches `push_front → push_back`, `pop_back → pop_front` ──

    #[test]
    fn mutation_lru_eviction_order() {
        let mut cache = LRUCache::new(3);
        cache.touch(GridCoord::new(1, 0, 0)); // Oldest
        cache.touch(GridCoord::new(2, 0, 0));
        cache.touch(GridCoord::new(3, 0, 0)); // Newest
        cache.touch(GridCoord::new(4, 0, 0)); // Should evict 1

        // Oldest (1) should be evicted, newest (4) should remain
        assert!(
            !cache.contains(GridCoord::new(1, 0, 0)),
            "oldest must be evicted"
        );
        assert!(
            cache.contains(GridCoord::new(4, 0, 0)),
            "newest must remain"
        );

        // LRU should be 2 (the oldest remaining)
        assert_eq!(cache.lru(), Some(GridCoord::new(2, 0, 0)));
    }

    /// Touch reorders: most-recently-touched should NOT be LRU.
    #[test]
    fn mutation_lru_touch_reorder_specifics() {
        let mut cache = LRUCache::new(5);
        cache.touch(GridCoord::new(1, 0, 0));
        cache.touch(GridCoord::new(2, 0, 0));
        cache.touch(GridCoord::new(3, 0, 0));
        // Order after touches: front=[3,2,1]=back
        assert_eq!(cache.lru(), Some(GridCoord::new(1, 0, 0)));

        // Touch 1 again — should move to front
        cache.touch(GridCoord::new(1, 0, 0));
        // Order: front=[1,3,2]=back → LRU is 2
        assert_eq!(cache.lru(), Some(GridCoord::new(2, 0, 0)));
    }

    // ── WorldPartition cells_in_radius: catches distance comparison ──

    #[test]
    fn mutation_cells_in_radius_distance_check() {
        let config = GridConfig {
            cell_size: 10.0,
            world_bounds: (-100.0, 100.0, -100.0, 100.0),
        };
        let partition = WorldPartition::new(config);

        // Center at (5, 0, 5) = cell (0, 0, 0). Radius = 12 (covers ~1 cell in each dir)
        let cells = partition.cells_in_radius(Vec3::new(5.0, 0.0, 5.0), 12.0);
        let center = GridCoord::new(0, 0, 0);
        assert!(cells.contains(&center), "center cell must be in radius");

        // Direct neighbor should be included (distance ~10 < 12)
        assert!(
            cells.contains(&GridCoord::new(1, 0, 0)),
            "adjacent cell should be in radius"
        );

        // Far cell should NOT be included (distance ~30 > 12)
        assert!(
            !cells.contains(&GridCoord::new(3, 0, 3)),
            "far cell should not be in radius"
        );
    }

    // ── Cell is_loaded / is_loading: catches state comparison ──

    #[test]
    fn mutation_cell_state_checks() {
        let mut cell = Cell::new(GridCoord::new(0, 0, 0), 100.0);

        assert!(!cell.is_loaded(), "default cell should not be loaded");
        assert!(!cell.is_loading(), "default cell should not be loading");

        cell.state = CellState::Loaded;
        assert!(cell.is_loaded());
        assert!(!cell.is_loading());

        cell.state = CellState::Loading;
        assert!(!cell.is_loaded());
        assert!(cell.is_loading());

        cell.state = CellState::Unloading;
        assert!(!cell.is_loaded());
        assert!(!cell.is_loading());
    }

    // ── WorldPartition assign/remove entity ──

    #[test]
    fn mutation_assign_entity_dedup() {
        let config = GridConfig::default();
        let mut partition = WorldPartition::new(config);

        let pos = Vec3::new(50.0, 0.0, 50.0);
        partition.assign_entity_to_cell(1, pos);
        partition.assign_entity_to_cell(1, pos); // Duplicate

        let coord = GridCoord::from_world_pos(pos, config.cell_size);
        let cell = partition.get_cell(coord).unwrap();
        assert_eq!(
            cell.entities.len(),
            1,
            "duplicate entity should not be added"
        );
    }

    #[test]
    fn mutation_remove_entity_clears_all_cells() {
        let config = GridConfig::default();
        let mut partition = WorldPartition::new(config);

        // Assign to multiple cells via bounds
        let bounds = AABB::new(Vec3::new(50.0, 0.0, 50.0), Vec3::new(150.0, 50.0, 150.0));
        partition.assign_entity_to_cells_by_bounds(42, bounds);

        // Should be in multiple cells
        let cell_count_before: usize = partition
            .cells
            .values()
            .filter(|c| c.entities.contains(&42))
            .count();
        assert!(cell_count_before >= 2, "entity should span multiple cells");

        // Remove
        partition.remove_entity(42);
        let cell_count_after: usize = partition
            .cells
            .values()
            .filter(|c| c.entities.contains(&42))
            .count();
        assert_eq!(
            cell_count_after, 0,
            "entity should be removed from all cells"
        );
    }

    // ── memory_usage_estimate: catches sizeof accumulation ──

    #[test]
    fn mutation_memory_usage_increases_with_entities() {
        let config = GridConfig::default();
        let mut partition = WorldPartition::new(config);

        partition.get_or_create_cell(GridCoord::new(0, 0, 0));
        let base_mem = partition.memory_usage_estimate();

        // Add entities to the cell
        let cell = partition.get_cell_mut(GridCoord::new(0, 0, 0)).unwrap();
        cell.entities.push(1);
        cell.entities.push(2);
        cell.entities.push(3);

        let with_entities_mem = partition.memory_usage_estimate();
        assert!(
            with_entities_mem > base_mem,
            "memory estimate must increase with entities: {} > {}",
            with_entities_mem,
            base_mem
        );
    }

    // ── Frustum plane normalization: catches /= → *= ──

    #[test]
    fn mutation_frustum_planes_normalized() {
        let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
        let frustum = Frustum::from_view_projection(vp);

        for (i, plane) in frustum.planes.iter().enumerate() {
            let normal_len = Vec3::new(plane.x, plane.y, plane.z).length();
            assert!(
                (normal_len - 1.0).abs() < 0.001,
                "Plane {} normal must be unit length, got {}",
                i,
                normal_len
            );
        }
    }

    // ── overlapping_cells: catches iteration bounds ──

    #[test]
    fn mutation_overlapping_cells_range() {
        // AABB spanning cells (-1,-1,-1) to (1,1,1) with cell_size=10
        let aabb = AABB::new(Vec3::new(-5.0, -5.0, -5.0), Vec3::new(15.0, 15.0, 15.0));
        let cells = aabb.overlapping_cells(10.0);

        // min_coord = from_world_pos(-5, -5, -5, 10) = (-1, -1, -1)
        // max_coord = from_world_pos(15, 15, 15, 10) = (1, 1, 1)
        // 3x3x3 = 27 cells
        assert_eq!(cells.len(), 27, "should span 3x3x3 cells");
        assert!(cells.contains(&GridCoord::new(-1, -1, -1)));
        assert!(cells.contains(&GridCoord::new(1, 1, 1)));
        assert!(cells.contains(&GridCoord::new(0, 0, 0)));
    }

    // ── Frustum plane extraction with rotated/translated camera ──
    // Catches all 25 `from_view_projection` arithmetic mutations (L198-238)
    // by ensuring the VP matrix has non-zero off-diagonal entries, so that
    // `+x` vs `-x` vs `*x` produce different plane coefficients.

    #[test]
    fn mutation_frustum_rotated_camera_planes() {
        // Non-trivial camera: offset and looking at an angle
        let view = Mat4::look_at_rh(
            Vec3::new(10.0, 5.0, 3.0), // eye
            Vec3::ZERO,                // target
            Vec3::Y,                   // up
        );
        let proj = Mat4::perspective_rh(
            std::f32::consts::FRAC_PI_3, // 60° FOV
            1.5,                         // non-unit aspect ratio
            0.1,
            100.0,
        );
        let vp = proj * view;
        let frustum = Frustum::from_view_projection(vp);

        // The camera is at (10,5,3) looking at origin. Origin should be inside.
        let at_origin = AABB::from_center_half_extents(Vec3::ZERO, Vec3::splat(0.5));
        assert!(
            frustum.intersects_aabb(&at_origin),
            "origin must be inside rotated frustum"
        );

        // Far behind the camera (opposite direction)
        let behind = AABB::from_center_half_extents(
            Vec3::new(50.0, 25.0, 15.0), // far behind camera
            Vec3::splat(0.5),
        );
        assert!(
            !frustum.intersects_aabb(&behind),
            "point behind camera must be outside"
        );

        // Far left of the view (perpendicular to view direction)
        // View direction: (0,0,0)-(10,5,3) = (-10,-5,-3) normalized
        // Left is perpendicular — use a point far to the "left" of the frustum
        let far_left =
            AABB::from_center_half_extents(Vec3::new(-100.0, 0.0, 100.0), Vec3::splat(0.5));
        assert!(
            !frustum.intersects_aabb(&far_left),
            "far left must be outside"
        );

        // Far right
        let far_right =
            AABB::from_center_half_extents(Vec3::new(100.0, 0.0, -100.0), Vec3::splat(0.5));
        assert!(
            !frustum.intersects_aabb(&far_right),
            "far right must be outside"
        );

        // Far above
        let far_up = AABB::from_center_half_extents(Vec3::new(5.0, 200.0, 0.0), Vec3::splat(0.5));
        assert!(
            !frustum.intersects_aabb(&far_up),
            "far above must be outside"
        );

        // Far below
        let far_down =
            AABB::from_center_half_extents(Vec3::new(5.0, -200.0, 0.0), Vec3::splat(0.5));
        assert!(
            !frustum.intersects_aabb(&far_down),
            "far below must be outside"
        );

        // Very far along view direction (beyond far plane)
        let beyond_far =
            AABB::from_center_half_extents(Vec3::new(-500.0, -250.0, -150.0), Vec3::splat(0.5));
        assert!(
            !frustum.intersects_aabb(&beyond_far),
            "beyond far plane must be outside"
        );
    }

    /// Verify each frustum plane's normal direction for a rotated camera.
    /// Each plane's normal should point roughly inward (toward the frustum interior).
    #[test]
    fn mutation_frustum_plane_normals_sign_check() {
        let view = Mat4::look_at_rh(Vec3::new(10.0, 5.0, 3.0), Vec3::ZERO, Vec3::Y);
        let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_3, 1.5, 0.1, 100.0);
        let vp = proj * view;
        let frustum = Frustum::from_view_projection(vp);

        // The center of the frustum should have positive distance to ALL planes.
        // Center point: midway between camera and target, roughly at (5, 2.5, 1.5)
        let center = Vec3::new(5.0, 2.5, 1.5);
        for (i, plane) in frustum.planes.iter().enumerate() {
            let normal = Vec3::new(plane.x, plane.y, plane.z);
            let dist = normal.dot(center) + plane.w;
            assert!(
                dist > -0.5, // slightly generous for rounding
                "plane {} must have positive signed distance to frustum center, got {}",
                i,
                dist
            );
        }
    }

    /// Use an asymmetric orthographic projection to create non-zero w_axis components.
    /// This catches mutations in the d-component (4th element) of each plane.
    #[test]
    fn mutation_frustum_asymmetric_ortho_planes() {
        // Asymmetric bounds: left != -right, bottom != -top
        let proj = Mat4::orthographic_rh(-5.0, 15.0, -3.0, 12.0, 0.1, 80.0);
        let frustum = Frustum::from_view_projection(proj);

        // Center of the visible region: x∈[-5,15] → cx=5, y∈[-3,12] → cy=4.5, z∈[-0.1,-80] → cz=-40
        let center = AABB::from_center_half_extents(Vec3::new(5.0, 4.5, -40.0), Vec3::splat(0.5));
        assert!(
            frustum.intersects_aabb(&center),
            "center of asymmetric ortho"
        );

        // Outside left (x < -5)
        let left = AABB::from_center_half_extents(Vec3::new(-20.0, 4.5, -40.0), Vec3::splat(0.5));
        assert!(
            !frustum.intersects_aabb(&left),
            "outside left of asymmetric ortho"
        );

        // Outside right (x > 15)
        let right = AABB::from_center_half_extents(Vec3::new(30.0, 4.5, -40.0), Vec3::splat(0.5));
        assert!(
            !frustum.intersects_aabb(&right),
            "outside right of asymmetric ortho"
        );

        // Outside bottom (y < -3)
        let bottom = AABB::from_center_half_extents(Vec3::new(5.0, -15.0, -40.0), Vec3::splat(0.5));
        assert!(
            !frustum.intersects_aabb(&bottom),
            "outside bottom of asymmetric ortho"
        );

        // Outside top (y > 12)
        let top = AABB::from_center_half_extents(Vec3::new(5.0, 25.0, -40.0), Vec3::splat(0.5));
        assert!(
            !frustum.intersects_aabb(&top),
            "outside top of asymmetric ortho"
        );

        // Outside far (z < -80)
        let far = AABB::from_center_half_extents(Vec3::new(5.0, 4.5, -150.0), Vec3::splat(0.5));
        assert!(
            !frustum.intersects_aabb(&far),
            "outside far of asymmetric ortho"
        );
    }

    /// Tilted camera test: non-Y up vector makes right.y ≠ 0, ensuring ALL view-projection
    /// matrix entries are non-zero. Verifies plane coefficients DIRECTLY by computing
    /// expected values from the VP matrix and comparing with Frustum output.
    /// This catches ALL from_view_projection arithmetic mutations.
    #[test]
    fn mutation_frustum_tilted_camera_all_entries_nonzero() {
        // Use a TILTED up vector so right.y ≠ 0
        let view = Mat4::look_at_rh(
            Vec3::new(5.0, 8.0, -2.0),
            Vec3::new(-3.0, 1.0, -10.0),
            Vec3::new(0.1, 0.95, 0.3),
        );
        let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, 1.33, 0.5, 200.0);
        let vp = proj * view;
        let frustum = Frustum::from_view_projection(vp);

        // Verify VP matrix has no zero entries
        let vp_cols = vp.to_cols_array();
        let zero_count = vp_cols.iter().filter(|v| v.abs() < 1e-6).count();
        assert!(
            zero_count <= 1,
            "VP matrix should have (nearly) all non-zero entries, found {} zeros",
            zero_count
        );

        // Manually compute expected planes using Gribb-Hartmann extraction
        // and compare with frustum.planes (which are normalized).
        // Left plane = row3 + row0
        let raw_left = Vec4::new(
            vp.x_axis.w + vp.x_axis.x,
            vp.y_axis.w + vp.y_axis.x,
            vp.z_axis.w + vp.z_axis.x,
            vp.w_axis.w + vp.w_axis.x,
        );
        // Right plane = row3 - row0
        let raw_right = Vec4::new(
            vp.x_axis.w - vp.x_axis.x,
            vp.y_axis.w - vp.y_axis.x,
            vp.z_axis.w - vp.z_axis.x,
            vp.w_axis.w - vp.w_axis.x,
        );
        // Bottom plane = row3 + row1
        let raw_bottom = Vec4::new(
            vp.x_axis.w + vp.x_axis.y,
            vp.y_axis.w + vp.y_axis.y,
            vp.z_axis.w + vp.z_axis.y,
            vp.w_axis.w + vp.w_axis.y,
        );
        // Top plane = row3 - row1
        let raw_top = Vec4::new(
            vp.x_axis.w - vp.x_axis.y,
            vp.y_axis.w - vp.y_axis.y,
            vp.z_axis.w - vp.z_axis.y,
            vp.w_axis.w - vp.w_axis.y,
        );
        // Near plane = row3 + row2
        let raw_near = Vec4::new(
            vp.x_axis.w + vp.x_axis.z,
            vp.y_axis.w + vp.y_axis.z,
            vp.z_axis.w + vp.z_axis.z,
            vp.w_axis.w + vp.w_axis.z,
        );
        // Far plane = row3 - row2
        let raw_far = Vec4::new(
            vp.x_axis.w - vp.x_axis.z,
            vp.y_axis.w - vp.y_axis.z,
            vp.z_axis.w - vp.z_axis.z,
            vp.w_axis.w - vp.w_axis.z,
        );

        let raw_planes = [raw_left, raw_right, raw_bottom, raw_top, raw_near, raw_far];
        let plane_names = ["left", "right", "bottom", "top", "near", "far"];

        for (i, (raw, name)) in raw_planes.iter().zip(plane_names.iter()).enumerate() {
            let normal_len = Vec3::new(raw.x, raw.y, raw.z).length();
            assert!(normal_len > 0.001, "plane {} normal length too small", name);

            let expected_normalized = *raw / normal_len;
            let actual = frustum.planes[i];

            // Compare each component
            assert!(
                (actual.x - expected_normalized.x).abs() < 0.001,
                "plane {} x mismatch: expected {}, got {}",
                name,
                expected_normalized.x,
                actual.x
            );
            assert!(
                (actual.y - expected_normalized.y).abs() < 0.001,
                "plane {} y mismatch: expected {}, got {}",
                name,
                expected_normalized.y,
                actual.y
            );
            assert!(
                (actual.z - expected_normalized.z).abs() < 0.001,
                "plane {} z mismatch: expected {}, got {}",
                name,
                expected_normalized.z,
                actual.z
            );
            assert!(
                (actual.w - expected_normalized.w).abs() < 0.001,
                "plane {} w mismatch: expected {}, got {}",
                name,
                expected_normalized.w,
                actual.w
            );
        }
    }

    // ── intersects_aabb boundary: catches `<` → `<=` at L278 ──

    #[test]
    fn mutation_intersects_aabb_exact_plane_boundary() {
        // Ortho frustum x∈[-10,10]
        let proj = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
        let frustum = Frustum::from_view_projection(proj);

        // Box whose positive vertex is EXACTLY on the left plane (dot + d = 0)
        // Left plane for symmetric ortho: normal=(1,0,0), d=10
        // A box at x∈[-11, -10], y∈[-1,1], z∈[-50,-48]
        // positive vertex = max.x=-10 (no, min.x for negative normal)
        // Actually for left plane normal.x > 0:
        //   positive vertex = (max.x, ...) = (-10, 1, -48)
        //   dot = 1*(-10) + 0*(1) + 0*(-48) = -10
        //   -10 + 10 = 0 → exactly on the boundary
        let on_boundary = AABB::new(Vec3::new(-11.0, -1.0, -50.0), Vec3::new(-10.0, 1.0, -48.0));
        // With `<`: 0 < 0 is false → NOT culled → intersects = true
        // With `<=`: 0 <= 0 is true → culled → intersects = false
        assert!(
            frustum.intersects_aabb(&on_boundary),
            "box touching plane boundary must intersect (< not <=)"
        );
    }

    // ── cells_in_frustum with non-zero camera and specific radius ──

    #[test]
    fn mutation_cells_in_frustum_nonzero_camera_coords() {
        // Use a large frustum that includes everything
        let proj = Mat4::orthographic_rh(-5000.0, 5000.0, -5000.0, 5000.0, 0.1, 5000.0);
        let frustum = Frustum::from_view_projection(proj);

        // Camera at cell (5, 3, 7) with cell_size=10, radius covers 2 cells
        let camera_pos = Vec3::new(55.0, 35.0, 75.0);
        let cell_size = 10.0;
        let cells = frustum.cells_in_frustum(camera_pos, cell_size, cell_size * 2.5);

        let camera_cell = GridCoord::new(5, 3, 7);
        assert!(
            cells.contains(&camera_cell),
            "camera cell (5,3,7) must be present"
        );

        // Asymmetric checks: (6,3,7) is different from (4,3,7) if + → -
        assert!(
            cells.contains(&GridCoord::new(6, 3, 7)),
            "cell +x must be present"
        );
        assert!(
            cells.contains(&GridCoord::new(4, 3, 7)),
            "cell -x must be present"
        );
        assert!(
            cells.contains(&GridCoord::new(5, 4, 7)),
            "cell +y must be present"
        );
        assert!(
            cells.contains(&GridCoord::new(5, 2, 7)),
            "cell -y must be present"
        );
        assert!(
            cells.contains(&GridCoord::new(5, 3, 8)),
            "cell +z must be present"
        );
        assert!(
            cells.contains(&GridCoord::new(5, 3, 6)),
            "cell -z must be present"
        );

        // Cell far away should NOT be present (catches radius / → %)
        assert!(
            !cells.contains(&GridCoord::new(50, 3, 7)),
            "far cell should not be present"
        );
    }

    #[test]
    fn mutation_cells_in_frustum_half_size_correct() {
        // Catches `cell_size * 0.5` → `cell_size + 0.5` and `cell_size / 0.5`.
        //
        // Strategy: build a TIGHT orthographic frustum so that adjacent cells
        // are OUTSIDE with correct half_size but INSIDE with inflated half_size.
        //
        // Camera at center of cell (0,0,0) = world (50,50,50), looking -Z.
        // Ortho half-widths = 30 → visible box ≈ [20,80]×[20,80]×[-10,49.9].
        // Cell (0,0,0) AABB [0,100]³ overlaps → INCLUDED.
        // Cell (1,0,0) AABB [100,200]×[0,100]×[0,100] → x-min=100 > 80 → EXCLUDED.
        //
        // With mutation * → +: half_size = 100+0.5 = 100.5
        //   Cell (1,0,0) AABB [49.5,250.5]... x-min=49.5 < 80 → WRONGLY INCLUDED.
        // With mutation * → /: half_size = 100/0.5 = 200
        //   Cell (1,0,0) AABB [-50,350]... clearly overlaps → WRONGLY INCLUDED.
        let cell_size = 100.0f32;
        let cam = Vec3::new(50.0, 50.0, 50.0);

        let view = Mat4::look_at_rh(cam, Vec3::new(50.0, 50.0, 0.0), Vec3::Y);
        let proj = Mat4::orthographic_rh(-30.0, 30.0, -30.0, 30.0, 0.1, 60.0);
        let vp = proj * view;
        let frustum = Frustum::from_view_projection(vp);

        // radius=200 → radius_cells=2 → loop includes cells up to ±2 from camera_cell
        let cells = frustum.cells_in_frustum(cam, cell_size, 200.0);

        let camera_cell = GridCoord::from_world_pos(cam, cell_size); // (0,0,0)
        assert!(
            cells.contains(&camera_cell),
            "camera cell {:?} must be in frustum",
            camera_cell
        );

        // Adjacent cells must NOT be included (frustum is too tight)
        let adjacent_x = GridCoord::new(1, 0, 0);
        let adjacent_neg_x = GridCoord::new(-1, 0, 0);
        assert!(
            !cells.contains(&adjacent_x),
            "cell (1,0,0) should be outside tight frustum (incorrect half_size?)"
        );
        assert!(
            !cells.contains(&adjacent_neg_x),
            "cell (-1,0,0) should be outside tight frustum (incorrect half_size?)"
        );
    }

    #[test]
    fn mutation_cells_in_frustum_radius_division() {
        // Specifically test that radius / cell_size uses / not %
        // radius=250, cell_size=100: 250/100=2.5 ceil=3 vs 250%100=50 ceil=50
        let proj = Mat4::orthographic_rh(-5000.0, 5000.0, -5000.0, 5000.0, 0.1, 5000.0);
        let frustum = Frustum::from_view_projection(proj);

        let cells = frustum.cells_in_frustum(Vec3::ZERO, 100.0, 250.0);
        // With /: radius_cells=3, loop range is 7×7×7=343 max
        // With %: radius_cells=50, loop range is 101×101×101 = giant
        assert!(
            cells.len() < 1000,
            "cells_in_frustum should use / not %, got {} cells",
            cells.len()
        );
    }

    // ── cells_in_radius: non-symmetric center, divergent radius ──

    #[test]
    fn mutation_cells_in_radius_asymmetric_center() {
        // Non-zero center so +dx and -dx produce different results
        let config = GridConfig {
            cell_size: 10.0,
            world_bounds: (-1000.0, 1000.0, -1000.0, 1000.0),
        };
        let partition = WorldPartition::new(config);

        // Center at (75, 0, 55) = cell (7, 0, 5). Radius = 15 covers neighbors.
        let cells = partition.cells_in_radius(Vec3::new(75.0, 0.0, 55.0), 15.0);
        let center = GridCoord::new(7, 0, 5);
        assert!(cells.contains(&center), "center cell must be present");

        // If +dx is mutated to -dx, cell (8,0,5) would become (6,0,5) and vice versa.
        // With non-zero center, (8,0,5) and (6,0,5) are DIFFERENT cells.
        assert!(
            cells.contains(&GridCoord::new(8, 0, 5)),
            "cell (8,0,5) must be present (+dx)"
        );
        assert!(
            cells.contains(&GridCoord::new(7, 0, 6)),
            "cell (7,0,6) must be present (+dz)"
        );
    }

    #[test]
    fn mutation_cells_in_radius_division_not_modulo() {
        // Radius and cell_size that produce DIFFERENT results with / vs %
        // radius=200, cell_size=100: 200/100=2 ceil=2 vs 200%100=0 ceil=0
        // With %: radius_cells=0, loop is just center cell (1 cell)
        // With /: radius_cells=2, loop covers 5×5=25 cells (filtered by distance)
        let config = GridConfig {
            cell_size: 100.0,
            world_bounds: (-10000.0, 10000.0, -10000.0, 10000.0),
        };
        let partition = WorldPartition::new(config);

        let cells = partition.cells_in_radius(Vec3::new(0.0, 0.0, 0.0), 200.0);
        // With correct /: should have multiple cells within 200 units
        // With %: radius_cells=0, only center cell
        assert!(
            cells.len() > 1,
            "cells_in_radius must use / not %, got {} cells (expected >1)",
            cells.len()
        );
    }

    // ── memory_usage_estimate: exact value verification ──

    #[test]
    fn mutation_memory_usage_exact_calculation() {
        let config = GridConfig::default();
        let mut partition = WorldPartition::new(config);

        // Create a cell with known entities and assets
        let coord = GridCoord::new(0, 0, 0);
        let cell = partition.get_or_create_cell(coord);
        cell.entities.push(1);
        cell.entities.push(2);
        cell.entities.push(3);
        cell.assets.push(crate::world_partition::AssetRef {
            path: "test.mesh".to_string(),
            asset_type: crate::world_partition::AssetType::Mesh,
        });
        cell.assets.push(crate::world_partition::AssetRef {
            path: "test.tex".to_string(),
            asset_type: crate::world_partition::AssetType::Texture,
        });

        let mem = partition.memory_usage_estimate();
        let cell_size = std::mem::size_of::<Cell>();
        let entity_size = std::mem::size_of::<u64>();
        let asset_size = std::mem::size_of::<crate::world_partition::AssetRef>();

        // Expected: cell_size + 3*entity_size + 2*asset_size
        let expected = cell_size + 3 * entity_size + 2 * asset_size;
        assert_eq!(
            mem, expected,
            "memory estimate must be cell_size({}) + 3*entity_size({}) + 2*asset_size({}) = {}, got {}",
            cell_size, entity_size, asset_size, expected, mem
        );

        // Also verify the formula uses multiplication, not addition
        // 3 * 8 = 24: with + it would be 3 + 8 = 11
        assert!(
            mem > cell_size + 20,
            "memory must account for entity sizes as multiplication"
        );
    }

    // ── components_of_type: catches iterator and filter mutations ──

    #[test]
    fn mutation_components_of_type_filters_correctly() {
        use crate::world_partition::{Cell, CellEntityBlueprint};
        use astraweave_asset::cell_loader::ComponentData as CellComponentData;

        let mut cell = Cell::new(GridCoord::new(0, 0, 0), 100.0);

        // Add entity blueprints with specific component types
        cell.entity_blueprints.push(CellEntityBlueprint {
            name: Some("entity_a".to_string()),
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
            components: vec![
                CellComponentData {
                    component_type: "MeshRenderer".to_string(),
                    data: "{}".to_string(),
                },
                CellComponentData {
                    component_type: "Collider".to_string(),
                    data: "{}".to_string(),
                },
            ],
        });
        cell.entity_blueprints.push(CellEntityBlueprint {
            name: Some("entity_b".to_string()),
            position: [1.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
            components: vec![CellComponentData {
                component_type: "MeshRenderer".to_string(),
                data: "{}".to_string(),
            }],
        });

        // Filter for "MeshRenderer" — should find 2
        let mesh_renderers: Vec<_> = cell.components_of_type("MeshRenderer").collect();
        assert_eq!(
            mesh_renderers.len(),
            2,
            "should find 2 MeshRenderer components, got {}",
            mesh_renderers.len()
        );

        // Filter for "Collider" — should find 1
        let colliders: Vec<_> = cell.components_of_type("Collider").collect();
        assert_eq!(colliders.len(), 1, "should find 1 Collider component");

        // Filter for nonexistent type — should find 0
        let scripts: Vec<_> = cell.components_of_type("Script").collect();
        assert_eq!(scripts.len(), 0, "should find 0 Script components");
    }
}

// ============================================================================
// Partitioned Scene Mutation Tests
// ============================================================================

#[cfg(test)]
mod partitioned_scene_mutation_tests {
    use crate::partitioned_scene::*;
    use crate::world_partition::GridCoord;
    use astraweave_asset::cell_loader::{CellData, EntityData};

    // ── entity_id generation: catches bitwise operation changes ──

    /// Entity ID = (coord.x << 40) | (coord.y << 20) | idx.
    /// Tests that different coords produce different IDs and that idx is encoded.
    #[test]
    fn mutation_entity_id_bitwise_operations() {
        let mut ps = PartitionedScene::new_default();
        let coord = GridCoord::new(1, 2, 0);

        let mut data = CellData::new([1, 2, 0]);
        data.add_entity(EntityData::new([0.0, 0.0, 0.0]).with_name("e0"));
        data.add_entity(EntityData::new([0.0, 0.0, 0.0]).with_name("e1"));

        ps.on_cell_loaded(coord, data);

        let entities = ps.query_entities_in_cell(coord).unwrap();
        assert_eq!(entities.len(), 2, "should have 2 entities");

        // Two entities in the same cell must have DIFFERENT IDs (idx differs)
        assert_ne!(entities[0], entities[1], "entities must have distinct IDs");

        // The entity IDs must encode the coord.x value in bits 40+
        let e0 = entities[0];
        let extracted_x = (e0 >> 40) as i32;
        assert_eq!(extracted_x, coord.x as i32, "x must be encoded at bits 40+");

        let extracted_y = ((e0 >> 20) & 0xFFFFF) as i32;
        assert_eq!(
            extracted_y, coord.y as i32,
            "y must be encoded at bits 20-39"
        );
    }

    /// Different cells must produce different entity IDs even for the same index.
    #[test]
    fn mutation_entity_id_different_cells() {
        let mut ps = PartitionedScene::new_default();

        let coord_a = GridCoord::new(1, 0, 0);
        let coord_b = GridCoord::new(2, 0, 0);

        let mut data_a = CellData::new([1, 0, 0]);
        data_a.add_entity(EntityData::new([0.0, 0.0, 0.0]).with_name("a"));
        let mut data_b = CellData::new([2, 0, 0]);
        data_b.add_entity(EntityData::new([0.0, 0.0, 0.0]).with_name("b"));

        ps.on_cell_loaded(coord_a, data_a);
        ps.on_cell_loaded(coord_b, data_b);

        let ea = ps.query_entities_in_cell(coord_a).unwrap();
        let eb = ps.query_entities_in_cell(coord_b).unwrap();
        assert_ne!(
            ea[0], eb[0],
            "different cells must produce different entity IDs"
        );
    }

    // ── move_entity_to_cell: catches old_cell removal and new_cell addition ──

    #[test]
    fn mutation_move_entity_removes_from_old_cell() {
        let mut ps = PartitionedScene::new_default();
        let a = GridCoord::new(0, 0, 0);
        let b = GridCoord::new(1, 0, 0);

        // Add entity to cell a
        ps.move_entity_to_cell(100, a);
        assert!(ps.query_entities_in_cell(a).unwrap().contains(&100));

        // Move to cell b
        ps.move_entity_to_cell(100, b);

        // Must be in b, NOT in a
        assert!(
            ps.query_entities_in_cell(b).unwrap().contains(&100),
            "entity must be in new cell"
        );
        let a_entities = ps.query_entities_in_cell(a);
        let in_a = a_entities.map(|v| v.contains(&100)).unwrap_or(false);
        assert!(!in_a, "entity must be removed from old cell");
    }

    // ── on_cell_unloaded: catches entity cleanup ──

    #[test]
    fn mutation_on_cell_unloaded_cleans_entity_map() {
        let mut ps = PartitionedScene::new_default();
        let coord = GridCoord::new(0, 0, 0);

        let mut data = CellData::new([0, 0, 0]);
        data.add_entity(EntityData::new([0.0, 0.0, 0.0]).with_name("e"));
        ps.on_cell_loaded(coord, data);

        let entity_id = ps.query_entities_in_cell(coord).unwrap()[0];
        assert!(
            ps.get_entity_cell(entity_id).is_some(),
            "entity should be tracked"
        );

        ps.on_cell_unloaded(coord);

        assert!(
            ps.get_entity_cell(entity_id).is_none(),
            "entity should be cleaned up"
        );
        assert!(
            ps.query_entities_in_cell(coord).is_none(),
            "cell entities should be removed"
        );
    }

    // ── drain_events returns and clears ──

    #[test]
    fn mutation_drain_events_returns_correct_events() {
        let mut ps = PartitionedScene::new_default();
        let coord = GridCoord::new(0, 0, 0);

        let mut data = CellData::new([0, 0, 0]);
        data.add_entity(EntityData::new([0.0, 0.0, 0.0]).with_name("e"));
        ps.on_cell_loaded(coord, data);

        let events = ps.drain_events();
        // Should have: EntitySpawned + CellLoaded = 2 events
        assert_eq!(events.len(), 2);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SceneEvent::CellLoaded(_))),
            "must have CellLoaded event"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SceneEvent::EntitySpawned(_, _))),
            "must have EntitySpawned event"
        );

        // Queue should be empty after drain
        assert!(ps.drain_events().is_empty());
    }
}

// ============================================================================
// ECS System Mutation Tests — comprehensive tests for ECS feature-gated code
// ============================================================================

#[cfg(all(test, feature = "ecs"))]
mod ecs_system_mutation_tests {
    use crate::ecs::*;
    use crate::Transform;
    use astraweave_ecs::World as EcsWorld;
    use glam::{Mat4, Quat, Vec3};

    // ── SceneGraph::attach / detach / reparent ──

    #[test]
    fn mutation_attach_creates_parent_and_children() {
        let mut world = EcsWorld::new();
        let parent = world.spawn();
        let child = world.spawn();

        world.insert(parent, CTransformLocal(Transform::identity()));
        world.insert(child, CTransformLocal(Transform::identity()));

        SceneGraph::attach(&mut world, child, parent);

        // Child must have CParent pointing to parent
        let p = world
            .get::<CParent>(child)
            .expect("child must have CParent");
        assert_eq!(p.0, parent, "CParent must point to parent entity");

        // Parent must have CChildren containing child
        let c = world
            .get::<CChildren>(parent)
            .expect("parent must have CChildren");
        assert!(c.0.contains(&child), "CChildren must contain child");

        // Child must be marked dirty
        assert!(
            world.get::<CDirtyTransform>(child).is_some(),
            "child must be marked dirty after attach"
        );
    }

    #[test]
    fn mutation_attach_deduplicates_children() {
        let mut world = EcsWorld::new();
        let parent = world.spawn();
        let child = world.spawn();

        world.insert(parent, CTransformLocal(Transform::identity()));
        world.insert(child, CTransformLocal(Transform::identity()));

        SceneGraph::attach(&mut world, child, parent);
        SceneGraph::attach(&mut world, child, parent); // Double attach

        let c = world.get::<CChildren>(parent).unwrap();
        let count = c.0.iter().filter(|&&e| e == child).count();
        assert_eq!(count, 1, "child must appear exactly once in children list");
    }

    #[test]
    fn mutation_detach_removes_parent_and_updates_children() {
        let mut world = EcsWorld::new();
        let parent = world.spawn();
        let child = world.spawn();

        world.insert(parent, CTransformLocal(Transform::identity()));
        world.insert(child, CTransformLocal(Transform::identity()));

        SceneGraph::attach(&mut world, child, parent);
        SceneGraph::detach(&mut world, child);

        // CParent should be removed
        assert!(
            world.get::<CParent>(child).is_none(),
            "CParent must be removed after detach"
        );

        // Parent's CChildren should not contain child
        if let Some(c) = world.get::<CChildren>(parent) {
            assert!(
                !c.0.contains(&child),
                "CChildren must not contain detached child"
            );
        }

        // Child should be dirty
        assert!(world.get::<CDirtyTransform>(child).is_some());
    }

    #[test]
    fn mutation_reparent_changes_parent() {
        let mut world = EcsWorld::new();
        let parent_a = world.spawn();
        let parent_b = world.spawn();
        let child = world.spawn();

        world.insert(parent_a, CTransformLocal(Transform::identity()));
        world.insert(parent_b, CTransformLocal(Transform::identity()));
        world.insert(child, CTransformLocal(Transform::identity()));

        SceneGraph::attach(&mut world, child, parent_a);
        SceneGraph::reparent(&mut world, child, parent_b);

        // Child should have parent_b
        let p = world.get::<CParent>(child).unwrap();
        assert_eq!(p.0, parent_b, "child must be reparented to parent_b");

        // parent_a should not have child
        if let Some(c) = world.get::<CChildren>(parent_a) {
            assert!(!c.0.contains(&child));
        }

        // parent_b should have child
        let c = world.get::<CChildren>(parent_b).unwrap();
        assert!(c.0.contains(&child));
    }

    // ── update_world_transforms ──

    #[test]
    fn mutation_update_world_transforms_root_entity() {
        let mut world = EcsWorld::new();
        let entity = world.spawn();

        let local = Transform::from_translation(Vec3::new(10.0, 20.0, 30.0));
        world.insert(entity, CTransformLocal(local));

        update_world_transforms(&mut world);

        let world_t = world
            .get::<CTransformWorld>(entity)
            .expect("must have CTransformWorld after update");
        let (_, _, translation) = world_t.0.to_scale_rotation_translation();
        assert!(
            (translation - Vec3::new(10.0, 20.0, 30.0)).length() < 0.001,
            "root world transform must equal local"
        );
    }

    #[test]
    fn mutation_update_world_transforms_hierarchy() {
        // Use rotation in parent so multiply vs add gives different results
        let mut world = EcsWorld::new();
        let parent = world.spawn();
        let child = world.spawn();

        // Parent: translate (10,0,0) + rotate 90° around Y
        let parent_transform = Transform {
            translation: Vec3::new(10.0, 0.0, 0.0),
            rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            scale: Vec3::ONE,
        };
        world.insert(parent, CTransformLocal(parent_transform));
        world.insert(
            child,
            CTransformLocal(Transform::from_translation(Vec3::new(5.0, 0.0, 0.0))),
        );

        SceneGraph::attach(&mut world, child, parent);
        update_world_transforms(&mut world);

        // Parent world position = (10, 0, 0)
        let parent_world = world.get::<CTransformWorld>(parent).unwrap();
        let (_, _, parent_pos) = parent_world.0.to_scale_rotation_translation();
        assert!(
            (parent_pos - Vec3::new(10.0, 0.0, 0.0)).length() < 0.001,
            "parent world position"
        );

        // With rotation: parent rotates 90° Y, so child local (5,0,0) becomes (0,0,-5) in parent space
        // Child world = parent_world * child_local = translate(10,0,0)*rotY(90°) * translate(5,0,0)
        // = (10 + 0, 0, 0 + -5) = (10, 0, -5)
        let child_world = world.get::<CTransformWorld>(child).unwrap();
        let (_, _, child_pos) = child_world.0.to_scale_rotation_translation();
        assert!(
            (child_pos - Vec3::new(10.0, 0.0, -5.0)).length() < 0.01,
            "child world position must account for parent rotation, got {:?}",
            child_pos
        );
    }

    #[test]
    fn mutation_update_world_transforms_clears_dirty() {
        let mut world = EcsWorld::new();
        let entity = world.spawn();
        world.insert(entity, CTransformLocal(Transform::identity()));
        world.insert(entity, CDirtyTransform);

        update_world_transforms(&mut world);

        assert!(
            world.get::<CDirtyTransform>(entity).is_none(),
            "dirty flag must be removed after update"
        );
    }

    #[test]
    fn mutation_update_world_transforms_scale_propagation() {
        let mut world = EcsWorld::new();
        let parent = world.spawn();
        let child = world.spawn();

        world.insert(parent, CTransformLocal(Transform::from_scale(2.0)));
        world.insert(child, CTransformLocal(Transform::from_scale(3.0)));

        SceneGraph::attach(&mut world, child, parent);
        update_world_transforms(&mut world);

        let child_world = world.get::<CTransformWorld>(child).unwrap();
        let (scale, _, _) = child_world.0.to_scale_rotation_translation();
        // 2.0 * 3.0 = 6.0
        assert!(
            (scale - Vec3::splat(6.0)).length() < 0.001,
            "child world scale must be 2*3=6, got {:?}",
            scale
        );
    }

    // ── sync_scene_to_renderer ──

    #[test]
    fn mutation_sync_renderer_includes_visible_entities() {
        let mut world = EcsWorld::new();
        let e1 = world.spawn();

        world.insert(e1, CTransformWorld(Mat4::from_translation(Vec3::X)));
        world.insert(e1, CMesh(1));
        world.insert(e1, CMaterial(0));
        world.insert(e1, CVisible(true));

        let instances = sync_scene_to_renderer(&mut world);
        assert_eq!(
            instances.len(),
            1,
            "visible entity with mesh+material must appear"
        );
        assert_eq!(instances[0].mesh_handle, 1);
        assert_eq!(instances[0].material_index, 0);
    }

    #[test]
    fn mutation_sync_renderer_excludes_invisible_entities() {
        let mut world = EcsWorld::new();
        let e1 = world.spawn();

        world.insert(e1, CTransformWorld(Mat4::IDENTITY));
        world.insert(e1, CMesh(1));
        world.insert(e1, CMaterial(0));
        world.insert(e1, CVisible(false));

        let instances = sync_scene_to_renderer(&mut world);
        assert!(
            instances.is_empty(),
            "invisible entity must not be rendered"
        );
    }

    #[test]
    fn mutation_sync_renderer_skips_no_mesh() {
        let mut world = EcsWorld::new();
        let e1 = world.spawn();

        world.insert(e1, CTransformWorld(Mat4::IDENTITY));
        // No CMesh
        world.insert(e1, CMaterial(0));

        let instances = sync_scene_to_renderer(&mut world);
        assert!(instances.is_empty(), "entity without mesh must be skipped");
    }

    #[test]
    fn mutation_sync_renderer_skips_no_material() {
        let mut world = EcsWorld::new();
        let e1 = world.spawn();

        world.insert(e1, CTransformWorld(Mat4::IDENTITY));
        world.insert(e1, CMesh(1));
        // No CMaterial

        let instances = sync_scene_to_renderer(&mut world);
        assert!(
            instances.is_empty(),
            "entity without material must be skipped"
        );
    }

    #[test]
    fn mutation_sync_renderer_default_visible() {
        let mut world = EcsWorld::new();
        let e1 = world.spawn();

        world.insert(e1, CTransformWorld(Mat4::IDENTITY));
        world.insert(e1, CMesh(1));
        world.insert(e1, CMaterial(0));
        // No CVisible — should default to visible

        let instances = sync_scene_to_renderer(&mut world);
        assert_eq!(
            instances.len(),
            1,
            "entity without CVisible defaults to visible"
        );
    }

    // ── update_animations ──

    #[test]
    fn mutation_update_animations_advances_time() {
        let mut world = EcsWorld::new();
        let entity = world.spawn();
        let mut animator = CAnimator::new(0);
        animator.play();
        world.insert(entity, animator);

        let clip_durations = [10.0]; // clip 0 is 10 seconds long
        update_animations(&mut world, 0.5, &clip_durations);

        let anim = world.get::<CAnimator>(entity).unwrap();
        assert!(
            (anim.time - 0.5).abs() < 0.001,
            "time must advance by dt*speed = 0.5, got {}",
            anim.time
        );
    }

    #[test]
    fn mutation_update_animations_respects_speed() {
        let mut world = EcsWorld::new();
        let entity = world.spawn();
        let mut animator = CAnimator::new(0).with_speed(2.0);
        animator.play();
        world.insert(entity, animator);

        let clip_durations = [10.0];
        update_animations(&mut world, 0.5, &clip_durations);

        let anim = world.get::<CAnimator>(entity).unwrap();
        // time = 0.5 * 2.0 = 1.0
        assert!(
            (anim.time - 1.0).abs() < 0.001,
            "time must be dt*speed = 1.0, got {}",
            anim.time
        );
    }

    #[test]
    fn mutation_update_animations_skips_paused() {
        let mut world = EcsWorld::new();
        let entity = world.spawn();
        let mut animator = CAnimator::new(0);
        animator.play();
        animator.pause();
        world.insert(entity, animator);

        let clip_durations = [10.0];
        update_animations(&mut world, 1.0, &clip_durations);

        let anim = world.get::<CAnimator>(entity).unwrap();
        assert_eq!(anim.time, 0.0, "paused animation must not advance");
    }

    #[test]
    fn mutation_update_animations_looping_wraps() {
        let mut world = EcsWorld::new();
        let entity = world.spawn();
        let mut animator = CAnimator::new(0).with_looping(true);
        animator.play();
        animator.time = 9.5;
        world.insert(entity, animator);

        let clip_durations = [10.0];
        update_animations(&mut world, 1.0, &clip_durations);

        let anim = world.get::<CAnimator>(entity).unwrap();
        // 9.5 + 1.0 = 10.5 → wraps to 0.5
        assert!(
            (anim.time - 0.5).abs() < 0.001,
            "looping animation must wrap: expected 0.5, got {}",
            anim.time
        );
        assert_eq!(
            anim.state,
            PlaybackState::Playing,
            "should still be playing"
        );
    }

    #[test]
    fn mutation_update_animations_non_looping_stops_at_end() {
        let mut world = EcsWorld::new();
        let entity = world.spawn();
        let mut animator = CAnimator::new(0).with_looping(false);
        animator.play();
        animator.time = 9.5;
        world.insert(entity, animator);

        let clip_durations = [10.0];
        update_animations(&mut world, 1.0, &clip_durations);

        let anim = world.get::<CAnimator>(entity).unwrap();
        assert!(
            (anim.time - 10.0).abs() < 0.001,
            "non-looping must clamp to duration"
        );
        assert_eq!(
            anim.state,
            PlaybackState::Stopped,
            "non-looping must stop at end"
        );
    }

    #[test]
    fn mutation_update_animations_marks_dirty_animation() {
        let mut world = EcsWorld::new();
        let entity = world.spawn();
        let mut animator = CAnimator::new(0);
        animator.play();
        world.insert(entity, animator);

        let clip_durations = [10.0];
        update_animations(&mut world, 0.1, &clip_durations);

        assert!(
            world.get::<CDirtyAnimation>(entity).is_some(),
            "playing animation must set CDirtyAnimation"
        );
    }

    // ── Animation boundary/edge-case tests for mutation hardening ──

    #[test]
    fn mutation_update_animations_looping_exact_boundary_no_wrap() {
        // Tests L862: `>` vs `>=` — when time EXACTLY equals clip_duration,
        // `>` won't wrap but `>=` would. Verifies the `>` semantics.
        let mut world = EcsWorld::new();
        let entity = world.spawn();
        let mut animator = CAnimator::new(0).with_looping(true);
        animator.play();
        animator.time = 9.0; // advance by 1.0 → exactly 10.0
        world.insert(entity, animator);

        let clip_durations = [10.0];
        update_animations(&mut world, 1.0, &clip_durations);

        let anim = world.get::<CAnimator>(entity).unwrap();
        // 9.0 + 1.0 = 10.0, which is NOT > 10.0, so should NOT wrap
        assert!(
            (anim.time - 10.0).abs() < 0.001,
            "time exactly at duration should NOT wrap (> not >=), got {}",
            anim.time
        );
    }

    #[test]
    fn mutation_update_animations_negative_speed_wraps() {
        // Tests L865: `< 0.0` — negative time should wrap via
        // `clip_duration + (time % clip_duration)` (L866)
        let mut world = EcsWorld::new();
        let entity = world.spawn();
        let mut animator = CAnimator::new(0).with_looping(true).with_speed(-1.0);
        animator.play();
        animator.time = 0.5; // advance by 1.0 * -1.0 → -0.5
        world.insert(entity, animator);

        let clip_durations = [10.0];
        update_animations(&mut world, 1.0, &clip_durations);

        let anim = world.get::<CAnimator>(entity).unwrap();
        // time = 0.5 + 1.0*(-1.0) = -0.5
        // -0.5 < 0.0 → wrap: 10.0 + (-0.5 % 10.0) = 10.0 + (-0.5) = 9.5
        assert!(
            (anim.time - 9.5).abs() < 0.01,
            "negative time should wrap to 9.5, got {}",
            anim.time
        );
    }

    #[test]
    fn mutation_update_animations_negative_speed_exact_zero_no_wrap() {
        // Tests L865: `<` vs `<=` — when time is exactly 0.0,
        // `<` won't wrap but `<=` would.
        let mut world = EcsWorld::new();
        let entity = world.spawn();
        let mut animator = CAnimator::new(0).with_looping(true).with_speed(-1.0);
        animator.play();
        animator.time = 1.0; // advance by 1.0 * -1.0 → 0.0
        world.insert(entity, animator);

        let clip_durations = [10.0];
        update_animations(&mut world, 1.0, &clip_durations);

        let anim = world.get::<CAnimator>(entity).unwrap();
        // time = 1.0 + 1.0*(-1.0) = 0.0
        // 0.0 < 0.0 is false → should NOT wrap
        assert!(
            anim.time.abs() < 0.001,
            "time exactly at 0.0 should NOT wrap (< not <=), got {}",
            anim.time
        );
    }

    // ── compute_poses_stub ──

    #[test]
    fn mutation_compute_poses_stub_marks_dirty_and_resizes() {
        let mut world = EcsWorld::new();
        let entity = world.spawn();

        let skeleton = CSkeleton {
            joint_count: 4,
            root_indices: vec![0],
            parent_indices: vec![None, Some(0), Some(1), Some(1)],
            inverse_bind_matrices: vec![Mat4::IDENTITY; 4],
            local_transforms: vec![Transform::identity(); 4],
        };
        world.insert(entity, skeleton);
        world.insert(entity, CJointMatrices::default());
        world.insert(entity, CDirtyAnimation);

        compute_poses_stub(&mut world);

        // Matrices should be resized to joint_count
        let matrices = world.get::<CJointMatrices>(entity).unwrap();
        assert_eq!(
            matrices.matrices.len(),
            4,
            "matrices must be resized to joint_count"
        );
        assert!(matrices.dirty, "matrices must be marked dirty");

        // CDirtyAnimation should be removed
        assert!(
            world.get::<CDirtyAnimation>(entity).is_none(),
            "CDirtyAnimation must be cleared"
        );
    }

    #[test]
    fn mutation_compute_poses_stub_skips_without_skeleton() {
        let mut world = EcsWorld::new();
        let entity = world.spawn();

        world.insert(entity, CJointMatrices::default());
        world.insert(entity, CDirtyAnimation);

        compute_poses_stub(&mut world);

        // Should not crash, CDirtyAnimation should remain (no skeleton → skip)
        // Actually the code removes CDirtyAnimation regardless because it's iterated
        // Wait, it iterates entities with CDirtyAnimation, then checks has_skeleton.
        // If !has_skeleton, it continues (skips this entity).
        // So CDirtyAnimation... let me check the code again.
        // The code does: for entity in entities { if !has_skeleton || !has_matrices { continue; } ... world.remove::<CDirtyAnimation>(entity); }
        // So if no skeleton, CDirtyAnimation is NOT removed.
        assert!(
            world.get::<CDirtyAnimation>(entity).is_some(),
            "CDirtyAnimation should remain when skeleton is missing"
        );
    }

    // ── CAnimator::normalized_time ──

    #[test]
    fn mutation_normalized_time_computation() {
        let mut animator = CAnimator::new(0);
        animator.time = 5.0;

        // 5.0 / 10.0 = 0.5
        assert!(
            (animator.normalized_time(10.0) - 0.5).abs() < 0.001,
            "normalized_time must be time/duration"
        );

        // Duration 0 → 0.0
        assert_eq!(
            animator.normalized_time(0.0),
            0.0,
            "zero duration must return 0"
        );

        // Negative duration → 0.0
        assert_eq!(
            animator.normalized_time(-1.0),
            0.0,
            "negative duration must return 0"
        );

        // Clamped to [0.0, 1.0]
        animator.time = 20.0;
        assert!(
            (animator.normalized_time(10.0) - 1.0).abs() < 0.001,
            "normalized_time must clamp to 1.0"
        );
    }

    // ── PlaybackState::name ──

    #[test]
    fn mutation_playback_state_name() {
        assert_eq!(PlaybackState::Playing.name(), "Playing");
        assert_eq!(PlaybackState::Paused.name(), "Paused");
        assert_eq!(PlaybackState::Stopped.name(), "Stopped");
    }

    // ── PlaybackState::is_active ──

    #[test]
    fn mutation_playback_is_active_uses_not_stopped() {
        // is_active = !is_stopped. If mutated to is_stopped, everything flips.
        assert!(PlaybackState::Playing.is_active());
        assert!(PlaybackState::Paused.is_active());
        assert!(!PlaybackState::Stopped.is_active());
    }

    // ── PlaybackState Display: catches fmt → Ok(Default::default()) ──

    #[test]
    fn mutation_playback_state_display() {
        let s = format!("{}", PlaybackState::Playing);
        assert!(!s.is_empty(), "Display must produce non-empty output");
        assert_eq!(s, "Playing");

        let s = format!("{}", PlaybackState::Paused);
        assert_eq!(s, "Paused");

        let s = format!("{}", PlaybackState::Stopped);
        assert_eq!(s, "Stopped");
    }

    // ── CAnimator::is_playing / is_paused / is_stopped: catches → true / → false ──

    #[test]
    fn mutation_animator_is_playing_delegates_correctly() {
        let mut a = CAnimator::new(0);
        assert!(!a.is_playing(), "stopped animator must not be is_playing");
        assert!(!a.is_paused(), "stopped animator must not be is_paused");
        assert!(a.is_stopped(), "stopped animator must be is_stopped");

        a.play();
        assert!(a.is_playing(), "playing animator must be is_playing");
        assert!(!a.is_paused(), "playing animator must not be is_paused");
        assert!(!a.is_stopped(), "playing animator must not be is_stopped");

        a.pause();
        assert!(!a.is_playing(), "paused animator must not be is_playing");
        assert!(a.is_paused(), "paused animator must be is_paused");
        assert!(!a.is_stopped(), "paused animator must not be is_stopped");
    }

    // ── CAnimator Display ──

    #[test]
    fn mutation_animator_display() {
        let anim = CAnimator::new(2).with_speed(1.5).with_looping(true);
        let display = format!("{}", anim);
        assert!(display.contains("clip=2"), "must show clip index");
        assert!(display.contains("speed=1.5"), "must show speed");
        assert!(display.contains("looping"), "must show looping");
    }

    #[test]
    fn mutation_animator_display_no_looping() {
        let anim = CAnimator::new(0).with_looping(false);
        let display = format!("{}", anim);
        assert!(
            !display.contains("looping"),
            "must not show looping when false"
        );
    }

    // ── CJointMatrices default ──

    #[test]
    fn mutation_joint_matrices_default() {
        let jm = CJointMatrices::default();
        assert!(jm.matrices.is_empty(), "default matrices must be empty");
        assert!(jm.dirty, "default dirty must be true");
    }

    // ── sync_bone_attachments ──

    #[test]
    fn mutation_sync_bone_attachments_sets_world_transform() {
        let mut world = EcsWorld::new();

        // Skeleton entity with joint matrices — use rotation+translation
        let skel_entity = world.spawn();
        let joint_mat = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_4)
            * Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
        world.insert(
            skel_entity,
            CJointMatrices {
                matrices: vec![joint_mat, Mat4::IDENTITY],
                dirty: false,
            },
        );

        // Attached entity referencing joint 0
        let attached = world.spawn();
        world.insert(
            attached,
            CParentBone {
                skeleton_entity: skel_entity,
                joint_index: 0,
            },
        );

        sync_bone_attachments(&mut world);

        // Attached entity's world transform should match joint 0
        let world_t = world
            .get::<CTransformWorld>(attached)
            .expect("must have world transform after sync");
        assert!(
            (world_t.0 - joint_mat).abs_diff_eq(Mat4::ZERO, 0.01),
            "attached entity must have joint world transform"
        );
    }

    #[test]
    fn mutation_sync_bone_attachments_out_of_range_index() {
        let mut world = EcsWorld::new();

        let skel_entity = world.spawn();
        world.insert(
            skel_entity,
            CJointMatrices {
                matrices: vec![Mat4::IDENTITY],
                dirty: false,
            },
        );

        let attached = world.spawn();
        world.insert(
            attached,
            CParentBone {
                skeleton_entity: skel_entity,
                joint_index: 999, // Far out of range
            },
        );

        // Should not crash
        sync_bone_attachments(&mut world);
        assert!(
            world.get::<CTransformWorld>(attached).is_none(),
            "out-of-range joint index should not set world transform"
        );
    }

    #[test]
    fn mutation_sync_bone_attachments_exact_boundary_index() {
        // Tests L935: `<` vs `<=` — joint_index == matrices.len() should NOT be valid
        let mut world = EcsWorld::new();

        let skel_entity = world.spawn();
        world.insert(
            skel_entity,
            CJointMatrices {
                matrices: vec![
                    Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)),
                    Mat4::IDENTITY,
                ],
                dirty: false,
            },
        );

        // Boundary: 2 matrices, joint_index=2 → should NOT access
        let attached = world.spawn();
        world.insert(
            attached,
            CParentBone {
                skeleton_entity: skel_entity,
                joint_index: 2, // Exactly == len
            },
        );

        sync_bone_attachments(&mut world);
        assert!(
            world.get::<CTransformWorld>(attached).is_none(),
            "joint_index == matrices.len() must NOT set transform (< not <=)"
        );
    }

    #[test]
    fn mutation_sync_bone_attachments_with_parent_local_transform() {
        // Tests L952: `parent_inv * joint_world_matrix` — multiply vs add
        // Entity has a CParent, so the local transform recomputation path is exercised.
        let mut world = EcsWorld::new();

        // Skeleton entity with joint matrices
        let skel_entity = world.spawn();
        let joint_mat = Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2)
            * Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0));
        world.insert(
            skel_entity,
            CJointMatrices {
                matrices: vec![joint_mat],
                dirty: false,
            },
        );

        // Parent entity with a non-trivial world transform (rotation)
        let parent_entity = world.spawn();
        let parent_world = Mat4::from_rotation_y(std::f32::consts::FRAC_PI_4);
        world.insert(parent_entity, CTransformWorld(parent_world));

        // Attached entity referencing joint 0, with parent
        let attached = world.spawn();
        world.insert(
            attached,
            CParentBone {
                skeleton_entity: skel_entity,
                joint_index: 0,
            },
        );
        world.insert(attached, CParent(parent_entity));

        sync_bone_attachments(&mut world);

        // World transform should equal the joint matrix
        let world_t = world
            .get::<CTransformWorld>(attached)
            .expect("must have world transform");
        assert!(
            (world_t.0 - joint_mat).abs_diff_eq(Mat4::ZERO, 0.01),
            "world transform must equal joint matrix"
        );

        // Local transform should be recomputed: parent_inv * joint_world
        let local_t = world
            .get::<CTransformLocal>(attached)
            .expect("must have local transform after bone sync with parent");
        let expected_local = parent_world.inverse() * joint_mat;
        let (exp_s, exp_r, exp_t) = expected_local.to_scale_rotation_translation();
        let local_mat = local_t.0.matrix();
        let (got_s, got_r, got_t) = local_mat.to_scale_rotation_translation();
        assert!(
            (got_t - exp_t).length() < 0.01,
            "local translation mismatch: expected {:?}, got {:?}",
            exp_t,
            got_t
        );
        assert!(
            (got_r - exp_r).length() < 0.01 || (got_r + exp_r).length() < 0.01,
            "local rotation mismatch: expected {:?}, got {:?}",
            exp_r,
            got_r
        );
    }
}
