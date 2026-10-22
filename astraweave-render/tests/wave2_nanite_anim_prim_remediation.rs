//! Wave 2 Render Sweep — Nanite Visibility, Animation, Primitives Proactive Remediation
//!
//! Targets: nanite_visibility.rs (126 mutants), animation.rs (93 mutants),
//! primitives.rs (123 mutants), clustered_forward.rs (134 mutants — GpuLight only)

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::animation::{
    compute_joint_matrices, AnimationChannel, AnimationClip, AnimationState, ChannelData,
    Interpolation, Joint, JointPalette, Skeleton, Transform,
};
use astraweave_render::clustered_forward::{ClusterConfig, GpuLight};
#[cfg(feature = "nanite")]
use astraweave_render::nanite_visibility::{Frustum, LODSelector};
use astraweave_render::primitives;
use glam::{Mat4, Quat, Vec3};

// ═════════════════════════════════════════════════════════════════════════
// Frustum (nanite feature-gated)
// ═════════════════════════════════════════════════════════════════════════

#[cfg(feature = "nanite")]
fn make_frustum() -> Frustum {
    let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::Y);
    let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 100.0);
    Frustum::from_matrix(proj * view)
}

#[cfg(feature = "nanite")]
#[test]
fn frustum_origin_aabb_visible() {
    let f = make_frustum();
    assert!(
        f.test_aabb(Vec3::splat(-1.0), Vec3::splat(1.0)),
        "AABB at origin should be visible from camera at z=10"
    );
}

#[cfg(feature = "nanite")]
#[test]
fn frustum_far_behind_aabb_invisible() {
    let f = make_frustum();
    assert!(
        !f.test_aabb(Vec3::new(-1.0, -1.0, 200.0), Vec3::new(1.0, 1.0, 202.0)),
        "AABB far behind camera should not be visible"
    );
}

#[cfg(feature = "nanite")]
#[test]
fn frustum_far_left_aabb_invisible() {
    let f = make_frustum();
    assert!(
        !f.test_aabb(Vec3::new(500.0, -0.5, -0.5), Vec3::new(501.0, 0.5, 0.5)),
        "AABB far to the right should not be visible"
    );
}

#[cfg(feature = "nanite")]
#[test]
fn frustum_sphere_at_origin_visible() {
    let f = make_frustum();
    assert!(f.test_sphere(Vec3::ZERO, 1.0));
}

#[cfg(feature = "nanite")]
#[test]
fn frustum_sphere_behind_invisible() {
    let f = make_frustum();
    assert!(
        !f.test_sphere(Vec3::new(0.0, 0.0, 500.0), 1.0),
        "Sphere far behind camera should not be visible"
    );
}

#[cfg(feature = "nanite")]
#[test]
fn frustum_large_sphere_always_visible() {
    let f = make_frustum();
    assert!(
        f.test_sphere(Vec3::ZERO, 1000.0),
        "Very large sphere should be visible regardless"
    );
}

#[cfg(feature = "nanite")]
#[test]
fn frustum_sphere_at_edge_visible() {
    let f = make_frustum();
    // sphere near the frustum edge — large radius should make it visible
    assert!(f.test_sphere(Vec3::new(5.0, 0.0, 0.0), 5.0));
}

// ═════════════════════════════════════════════════════════════════════════
// LODSelector (nanite feature-gated)
// ═════════════════════════════════════════════════════════════════════════

#[cfg(feature = "nanite")]
#[test]
fn lod_selector_new_defaults() {
    let sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_4);
    assert!((sel.screen_height - 1080.0).abs() < 0.01);
    assert!((sel.fov - std::f32::consts::FRAC_PI_4).abs() < 0.01);
    assert!((sel.lod_bias - 1.0).abs() < 0.01);
}

#[cfg(feature = "nanite")]
#[test]
fn lod_selector_close_object_highest_detail() {
    let sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_4);
    let lod = sel.select_lod(
        Vec3::ZERO,               // bounds center
        1.0,                      // bounds radius
        2.0,                      // lod_error
        Vec3::new(0.0, 0.0, 2.0), // camera very close
        3,                        // max_lod
    );
    assert_eq!(lod, 0, "Close object should use highest detail (LOD 0)");
}

#[cfg(feature = "nanite")]
#[test]
fn lod_selector_distant_object_lower_detail() {
    let sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_4);
    let lod_near = sel.select_lod(Vec3::ZERO, 1.0, 2.0, Vec3::new(0.0, 0.0, 5.0), 3);
    let lod_far = sel.select_lod(Vec3::ZERO, 1.0, 2.0, Vec3::new(0.0, 0.0, 500.0), 3);
    assert!(
        lod_far >= lod_near,
        "Far object should have same or lower detail LOD"
    );
}

#[cfg(feature = "nanite")]
#[test]
fn lod_selector_max_lod_capped() {
    let sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_4);
    let lod = sel.select_lod(
        Vec3::ZERO,
        0.001,
        0.1,
        Vec3::new(0.0, 0.0, 10000.0), // very far
        2,                            // max_lod = 2
    );
    assert!(lod <= 2, "LOD should not exceed max_lod={}", lod);
}

// ═════════════════════════════════════════════════════════════════════════
// Transform
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn transform_default_is_identity() {
    let t = Transform::default();
    assert_eq!(t.translation, Vec3::ZERO);
    assert_eq!(t.rotation, Quat::IDENTITY);
    assert_eq!(t.scale, Vec3::ONE);
}

#[test]
fn transform_to_matrix_identity() {
    let t = Transform::default();
    let m = t.to_matrix();
    let expected = Mat4::IDENTITY;
    for i in 0..16 {
        assert!(
            (m.to_cols_array()[i] - expected.to_cols_array()[i]).abs() < 0.001,
            "Identity transform matrix mismatch at index {i}"
        );
    }
}

#[test]
fn transform_to_matrix_translation() {
    let t = Transform {
        translation: Vec3::new(1.0, 2.0, 3.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };
    let m = t.to_matrix();
    // Last column should be the translation
    let cols = m.to_cols_array_2d();
    assert!((cols[3][0] - 1.0).abs() < 0.001);
    assert!((cols[3][1] - 2.0).abs() < 0.001);
    assert!((cols[3][2] - 3.0).abs() < 0.001);
}

#[test]
fn transform_lerp_midpoint() {
    let a = Transform {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };
    let b = Transform {
        translation: Vec3::new(2.0, 4.0, 6.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::new(2.0, 2.0, 2.0),
    };
    let mid = a.lerp(&b, 0.5);
    assert!((mid.translation.x - 1.0).abs() < 0.01);
    assert!((mid.translation.y - 2.0).abs() < 0.01);
    assert!((mid.scale.x - 1.5).abs() < 0.01);
}

#[test]
fn transform_lerp_at_zero() {
    let a = Transform {
        translation: Vec3::X,
        ..Transform::default()
    };
    let b = Transform {
        translation: Vec3::Y,
        ..Transform::default()
    };
    let result = a.lerp(&b, 0.0);
    assert!((result.translation - Vec3::X).length() < 0.01);
}

#[test]
fn transform_lerp_at_one() {
    let a = Transform {
        translation: Vec3::X,
        ..Transform::default()
    };
    let b = Transform {
        translation: Vec3::Y,
        ..Transform::default()
    };
    let result = a.lerp(&b, 1.0);
    assert!((result.translation - Vec3::Y).length() < 0.01);
}

// ═════════════════════════════════════════════════════════════════════════
// AnimationState
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn animation_state_default_not_playing() {
    let state = AnimationState::default();
    assert!(!state.playing);
    assert!((state.time - 0.0).abs() < 0.001);
    assert!((state.speed - 1.0).abs() < 0.001);
    assert!(state.looping);
}

#[test]
fn animation_state_play_and_pause() {
    let mut state = AnimationState::default();
    state.play();
    assert!(state.playing);
    state.pause();
    assert!(!state.playing);
}

#[test]
fn animation_state_stop_resets_time() {
    let mut state = AnimationState::default();
    state.play();
    state.update(1.0, 5.0);
    assert!(state.time > 0.0);
    state.stop();
    assert!(!state.playing);
    assert!((state.time - 0.0).abs() < 0.001);
}

#[test]
fn animation_state_restart() {
    let mut state = AnimationState::default();
    state.play();
    state.update(2.0, 5.0);
    state.restart();
    assert!(state.playing);
    assert!((state.time - 0.0).abs() < 0.001);
}

#[test]
fn animation_state_update_advances_time() {
    let mut state = AnimationState::default();
    state.play();
    state.update(0.5, 5.0);
    assert!((state.time - 0.5).abs() < 0.01);
}

#[test]
fn animation_state_update_not_playing_no_advance() {
    let mut state = AnimationState::default();
    // Not playing — should not advance
    state.update(1.0, 5.0);
    assert!((state.time - 0.0).abs() < 0.001);
}

#[test]
fn animation_state_looping_wraps() {
    let mut state = AnimationState::default();
    state.play();
    state.looping = true;
    state.update(7.0, 5.0); // 7.0 mod 5.0 = 2.0
    assert!(
        (state.time - 2.0).abs() < 0.01,
        "Looping should wrap: {}",
        state.time
    );
}

#[test]
fn animation_state_non_looping_clamps() {
    let mut state = AnimationState::default();
    state.play();
    state.looping = false;
    state.update(7.0, 5.0);
    assert!(
        (state.time - 5.0).abs() < 0.01,
        "Non-looping should clamp to duration"
    );
    assert!(!state.playing, "Should stop playing at end");
}

#[test]
fn animation_state_speed_multiplier() {
    let mut state = AnimationState::default();
    state.play();
    state.speed = 2.0;
    state.update(1.0, 10.0);
    assert!(
        (state.time - 2.0).abs() < 0.01,
        "Speed 2x should advance 2.0 in 1s"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// AnimationClip: sample
// ═════════════════════════════════════════════════════════════════════════

fn make_simple_skeleton() -> Skeleton {
    Skeleton {
        joints: vec![
            Joint {
                name: "root".to_string(),
                parent_index: None,
                inverse_bind_matrix: Mat4::IDENTITY,
                local_transform: Transform::default(),
            },
            Joint {
                name: "child".to_string(),
                parent_index: Some(0),
                inverse_bind_matrix: Mat4::IDENTITY,
                local_transform: Transform {
                    translation: Vec3::new(0.0, 1.0, 0.0),
                    ..Transform::default()
                },
            },
        ],
        root_indices: vec![0],
    }
}

fn make_simple_clip() -> AnimationClip {
    AnimationClip {
        name: "walk".to_string(),
        duration: 1.0,
        channels: vec![AnimationChannel {
            target_joint_index: 0,
            times: vec![0.0, 0.5, 1.0],
            data: ChannelData::Translation(vec![
                Vec3::ZERO,
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
            ]),
            interpolation: Interpolation::Linear,
        }],
    }
}

#[test]
fn animation_clip_sample_at_start() {
    let skeleton = make_simple_skeleton();
    let clip = make_simple_clip();
    let transforms = clip.sample(0.0, &skeleton);
    assert_eq!(transforms.len(), 2);
    assert!((transforms[0].translation - Vec3::ZERO).length() < 0.01);
}

#[test]
fn animation_clip_sample_at_midpoint() {
    let skeleton = make_simple_skeleton();
    let clip = make_simple_clip();
    let transforms = clip.sample(0.25, &skeleton);
    // Linear interpolation between t=0 (0,0,0) and t=0.5 (1,0,0) at t=0.25 → (0.5,0,0)
    assert!(
        (transforms[0].translation.x - 0.5).abs() < 0.01,
        "Expected x=0.5, got {}",
        transforms[0].translation.x
    );
}

#[test]
fn animation_clip_sample_at_end() {
    let skeleton = make_simple_skeleton();
    let clip = make_simple_clip();
    let transforms = clip.sample(1.0, &skeleton);
    assert!((transforms[0].translation.x - 2.0).abs() < 0.01);
}

#[test]
fn animation_clip_sample_step_interpolation() {
    let skeleton = make_simple_skeleton();
    let clip = AnimationClip {
        name: "step".to_string(),
        duration: 1.0,
        channels: vec![AnimationChannel {
            target_joint_index: 0,
            times: vec![0.0, 0.5, 1.0],
            data: ChannelData::Translation(vec![
                Vec3::ZERO,
                Vec3::new(5.0, 0.0, 0.0),
                Vec3::new(10.0, 0.0, 0.0),
            ]),
            interpolation: Interpolation::Step,
        }],
    };
    // Step interpolation at t=0.25 should still be the first keyframe value
    let transforms = clip.sample(0.25, &skeleton);
    assert!(
        (transforms[0].translation.x - 0.0).abs() < 0.01,
        "Step should hold first keyframe, got {}",
        transforms[0].translation.x
    );
}

#[test]
fn animation_clip_sample_rotation() {
    let skeleton = make_simple_skeleton();
    let q0 = Quat::IDENTITY;
    let q1 = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let clip = AnimationClip {
        name: "rotate".to_string(),
        duration: 1.0,
        channels: vec![AnimationChannel {
            target_joint_index: 0,
            times: vec![0.0, 1.0],
            data: ChannelData::Rotation(vec![q0, q1]),
            interpolation: Interpolation::Linear,
        }],
    };
    let transforms = clip.sample(0.5, &skeleton);
    // Slerp at t=0.5 should be half rotation (PI/4)
    let expected_angle = std::f32::consts::FRAC_PI_4;
    let (axis, angle) = transforms[0].rotation.to_axis_angle();
    assert!(
        (angle - expected_angle).abs() < 0.05,
        "Expected angle ~{expected_angle}, got {angle}"
    );
    // Axis should be Z
    assert!(
        (axis.z.abs() - 1.0).abs() < 0.01,
        "Rotation axis should be Z"
    );
}

#[test]
fn animation_clip_sample_scale() {
    let skeleton = make_simple_skeleton();
    let clip = AnimationClip {
        name: "scale".to_string(),
        duration: 1.0,
        channels: vec![AnimationChannel {
            target_joint_index: 0,
            times: vec![0.0, 1.0],
            data: ChannelData::Scale(vec![Vec3::ONE, Vec3::new(3.0, 3.0, 3.0)]),
            interpolation: Interpolation::Linear,
        }],
    };
    let transforms = clip.sample(0.5, &skeleton);
    assert!(
        (transforms[0].scale.x - 2.0).abs() < 0.01,
        "Expected scale 2.0 at midpoint"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// compute_joint_matrices
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn compute_joint_matrices_identity() {
    let skeleton = make_simple_skeleton();
    let transforms = vec![Transform::default(); 2];
    let matrices = compute_joint_matrices(&skeleton, &transforms).unwrap();
    assert_eq!(matrices.len(), 2);
    // Root should be identity (or very close)
    let root = matrices[0];
    for i in 0..16 {
        assert!(
            (root.to_cols_array()[i] - Mat4::IDENTITY.to_cols_array()[i]).abs() < 0.01,
            "Root matrix should be identity at index {i}"
        );
    }
}

#[test]
fn compute_joint_matrices_respects_hierarchy() {
    let skeleton = make_simple_skeleton();
    let transforms = vec![
        Transform {
            translation: Vec3::new(1.0, 0.0, 0.0),
            ..Transform::default()
        },
        Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            ..Transform::default()
        },
    ];
    let matrices = compute_joint_matrices(&skeleton, &transforms).unwrap();
    // Parent (root) translates by (1,0,0)
    // Child translates by (0,2,0) in LOCAL space
    // Child world position should be (1,2,0)
    let child_world = matrices[1].to_scale_rotation_translation().2;
    assert!(
        (child_world.x - 1.0).abs() < 0.01,
        "Child world X should be 1.0, got {}",
        child_world.x
    );
    assert!(
        (child_world.y - 2.0).abs() < 0.01,
        "Child world Y should be 2.0, got {}",
        child_world.y
    );
}

// ═════════════════════════════════════════════════════════════════════════
// JointPalette
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn joint_palette_from_matrices() {
    let matrices = vec![Mat4::IDENTITY, Mat4::from_translation(Vec3::X)];
    let palette = JointPalette::from_matrices(&matrices);
    assert_eq!(palette.joint_count, 2);
}

#[test]
fn joint_palette_from_empty_matrices() {
    let palette = JointPalette::from_matrices(&[]);
    assert_eq!(palette.joint_count, 0);
}

// ═════════════════════════════════════════════════════════════════════════
// Primitives
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cube_has_valid_geometry() {
    let (vertices, indices) = primitives::cube();
    assert!(!vertices.is_empty(), "Cube should have vertices");
    assert!(!indices.is_empty(), "Cube should have indices");
    assert_eq!(indices.len() % 3, 0, "Indices should be triangle list");
    // A cube should have 36 indices (12 triangles × 3)
    assert_eq!(indices.len(), 36, "Cube should have 36 indices");
    // All indices should be valid
    for &idx in &indices {
        assert!(
            (idx as usize) < vertices.len(),
            "Index {} out of bounds for {} vertices",
            idx,
            vertices.len()
        );
    }
}

#[test]
fn plane_has_valid_geometry() {
    let (vertices, indices) = primitives::plane();
    assert!(!vertices.is_empty());
    assert!(!indices.is_empty());
    assert_eq!(indices.len() % 3, 0);
    // A simple plane = 2 triangles = 6 indices
    assert_eq!(indices.len(), 6);
    for &idx in &indices {
        assert!((idx as usize) < vertices.len());
    }
}

#[test]
fn sphere_has_valid_geometry() {
    let (vertices, indices) = primitives::sphere(16, 16, 1.0);
    assert!(!vertices.is_empty());
    assert!(!indices.is_empty());
    assert_eq!(indices.len() % 3, 0);
    for &idx in &indices {
        assert!(
            (idx as usize) < vertices.len(),
            "Sphere index {} out of bounds for {} vertices",
            idx,
            vertices.len()
        );
    }
}

#[test]
fn sphere_radius_correct() {
    let (vertices, _) = primitives::sphere(8, 8, 5.0);
    for v in &vertices {
        let dist = (v.position[0].powi(2) + v.position[1].powi(2) + v.position[2].powi(2)).sqrt();
        assert!(
            (dist - 5.0).abs() < 0.01,
            "Sphere vertex should be at radius 5.0, got {dist}"
        );
    }
}

#[test]
fn sphere_normals_unit_length() {
    let (vertices, _) = primitives::sphere(8, 8, 1.0);
    for v in &vertices {
        let len = (v.normal[0].powi(2) + v.normal[1].powi(2) + v.normal[2].powi(2)).sqrt();
        assert!(
            (len - 1.0).abs() < 0.01,
            "Normal should be unit length, got {len}"
        );
    }
}

#[test]
fn sphere_more_stacks_more_vertices() {
    let (v1, _) = primitives::sphere(4, 4, 1.0);
    let (v2, _) = primitives::sphere(8, 8, 1.0);
    assert!(
        v2.len() > v1.len(),
        "More stacks/slices should produce more vertices"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// GpuLight
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn gpu_light_new() {
    let light = GpuLight::new(
        Vec3::new(1.0, 2.0, 3.0),
        10.0,
        Vec3::new(1.0, 0.5, 0.0),
        2.0,
    );
    // position = [x, y, z, radius]
    assert!((light.position[0] - 1.0).abs() < 0.01);
    assert!((light.position[1] - 2.0).abs() < 0.01);
    assert!((light.position[2] - 3.0).abs() < 0.01);
    assert!((light.position[3] - 10.0).abs() < 0.01, "w = radius");
    // color = [r, g, b, intensity]
    assert!((light.color[0] - 1.0).abs() < 0.01);
    assert!((light.color[3] - 2.0).abs() < 0.01, "w = intensity");
}

// ═════════════════════════════════════════════════════════════════════════
// ClusterConfig defaults
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cluster_config_default_reasonable() {
    let config = ClusterConfig::default();
    assert!(config.cluster_x > 0, "Should have positive X slices");
    assert!(config.cluster_y > 0, "Should have positive Y slices");
    assert!(config.cluster_z > 0, "Should have positive Z slices");
    assert!(config.near > 0.0, "Near plane should be positive");
    assert!(config.far > config.near, "Far should exceed near");
}
