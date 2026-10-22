//! Golden Test: Animated Pose at Fixed Frame
//!
//! Phase 2 Task 5 (Phase E): Validates that animation sampling produces
//! deterministic, reproducible results at fixed time.

mod test_utils;

use astraweave_render::{
    compute_joint_matrices, skin_vertex_cpu, AnimationChannel, AnimationClip, ChannelData,
    Interpolation, Joint, Skeleton, Transform,
};
use glam::{Mat4, Quat, Vec3};
use std::f32::consts::PI;

/// Create test skeleton for animation
fn create_animation_test_skeleton() -> Skeleton {
    Skeleton {
        root_indices: vec![0],
        joints: vec![
            Joint {
                name: "root".to_string(),
                parent_index: None,
                inverse_bind_matrix: Mat4::IDENTITY,
                local_transform: Transform::default(),
            },
            Joint {
                name: "child1".to_string(),
                parent_index: Some(0),
                inverse_bind_matrix: Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)),
                local_transform: Transform {
                    translation: Vec3::new(0.0, 1.0, 0.0),
                    ..Default::default()
                },
            },
            Joint {
                name: "child2".to_string(),
                parent_index: Some(1),
                inverse_bind_matrix: Mat4::from_translation(Vec3::new(0.0, -2.0, 0.0)),
                local_transform: Transform {
                    translation: Vec3::new(0.0, 1.0, 0.0),
                    ..Default::default()
                },
            },
        ],
    }
}

/// Create test animation: rotate child1 from 0° to 90° over 2 seconds
fn create_test_animation_clip() -> AnimationClip {
    AnimationClip {
        name: "test_rotation".to_string(),
        duration: 2.0,
        channels: vec![AnimationChannel {
            target_joint_index: 1, // Animate child1
            interpolation: Interpolation::Linear,
            times: vec![0.0, 1.0, 2.0],
            data: ChannelData::Rotation(vec![
                Quat::IDENTITY,                  // 0° at t=0
                Quat::from_rotation_z(PI / 4.0), // 45° at t=1
                Quat::from_rotation_z(PI / 2.0), // 90° at t=2
            ]),
        }],
    }
}

/// Golden baseline: Sample animation at t=1.0 (exact keyframe)
#[test]
fn test_animated_pose_keyframe_t1() {
    let skeleton = create_animation_test_skeleton();
    let clip = create_test_animation_clip();

    // Sample at exact keyframe (t=1.0)
    let local_poses = clip.sample(1.0, &skeleton);

    // Joint 0 (root) should be at bind pose
    assert_eq!(local_poses[0].translation, Vec3::ZERO);
    assert_eq!(local_poses[0].rotation, Quat::IDENTITY);

    // Joint 1 should have 45° rotation
    let expected_rotation = Quat::from_rotation_z(PI / 4.0);
    let rotation_diff = (local_poses[1].rotation.dot(expected_rotation) - 1.0).abs();
    assert!(
        rotation_diff < 1e-5,
        "Joint 1 rotation should be 45° at t=1.0"
    );

    // Verify translation is preserved
    assert_eq!(local_poses[1].translation, Vec3::new(0.0, 1.0, 0.0));
}

/// Golden baseline: Sample animation at t=0.5 (interpolated)
#[test]
fn test_animated_pose_interpolated_t0_5() {
    let skeleton = create_animation_test_skeleton();
    let clip = create_test_animation_clip();

    // Sample at interpolated time (t=0.5, midpoint between 0 and 1)
    let local_poses = clip.sample(0.5, &skeleton);

    // Joint 1 should have 22.5° rotation (linear interpolation of quaternions = slerp)
    let expected_rotation = Quat::IDENTITY.slerp(Quat::from_rotation_z(PI / 4.0), 0.5);
    let rotation_diff = (local_poses[1].rotation.dot(expected_rotation) - 1.0).abs();
    assert!(
        rotation_diff < 1e-4,
        "Joint 1 should be interpolated at t=0.5"
    );
}

/// Golden baseline: Deterministic sampling (repeat)
#[test]
fn test_animated_pose_determinism() {
    let skeleton = create_animation_test_skeleton();
    let clip = create_test_animation_clip();

    // Sample same time twice
    let poses1 = clip.sample(1.5, &skeleton);
    let poses2 = clip.sample(1.5, &skeleton);

    // Should be bit-exact
    assert_eq!(poses1.len(), poses2.len());
    for i in 0..poses1.len() {
        let diff_t = (poses1[i].translation - poses2[i].translation).length();
        let diff_r = (poses1[i].rotation.dot(poses2[i].rotation) - 1.0).abs();
        let diff_s = (poses1[i].scale - poses2[i].scale).length();

        assert!(diff_t < 1e-7, "Translation {} differs", i);
        assert!(diff_r < 1e-7, "Rotation {} differs", i);
        assert!(diff_s < 1e-7, "Scale {} differs", i);
    }
}

/// Golden baseline: Compute joint matrices from animated pose
#[test]
fn test_animated_pose_joint_matrices_t1() {
    let skeleton = create_animation_test_skeleton();
    let clip = create_test_animation_clip();

    // Sample at t=1.0 (45° rotation)
    let local_poses = clip.sample(1.0, &skeleton);

    // Compute joint matrices
    let joint_matrices = compute_joint_matrices(&skeleton, &local_poses).unwrap();

    assert_eq!(joint_matrices.len(), 3);

    // Verify joint 0 (root) is identity
    let diff0 = (joint_matrices[0] - Mat4::IDENTITY).abs();
    let max_diff0 = diff0
        .to_cols_array()
        .iter()
        .fold(0.0f32, |acc, &x| acc.max(x));
    assert!(
        max_diff0 < 1e-5,
        "Root joint should be identity in world space"
    );

    // Joint 1 should have rotation applied
    // Verify by checking that the matrix is NOT identity
    let diff1 = (joint_matrices[1] - Mat4::IDENTITY).abs();
    let max_diff1 = diff1
        .to_cols_array()
        .iter()
        .fold(0.0f32, |acc, &x| acc.max(x));
    assert!(max_diff1 > 0.1, "Joint 1 should have visible rotation");
}

/// Golden baseline: Vertex skinning with animated pose
#[test]
fn test_animated_pose_vertex_skinning_t1() {
    let skeleton = create_animation_test_skeleton();
    let clip = create_test_animation_clip();

    let local_poses = clip.sample(1.0, &skeleton);
    let joint_matrices = compute_joint_matrices(&skeleton, &local_poses).unwrap();

    // Vertex ABOVE joint 1 position - at (0, 2, 0) in mesh space
    // Joint 1 is at (0, 1, 0), so this vertex is +1 unit above it
    let position = Vec3::new(0.0, 2.0, 0.0);
    let normal = Vec3::Y;
    let joints: [u16; 4] = [1, 0, 0, 0];
    let weights = [1.0, 0.0, 0.0, 0.0];

    let (skinned_pos, skinned_normal) =
        skin_vertex_cpu(position, normal, joints, weights, &joint_matrices);

    // Inverse bind transforms (0, 2, 0) to (0, 1, 0) in joint local space
    // 45° rotation around Z at origin: (0, 1, 0) → (-sin(45°), cos(45°), 0) = (-0.707, 0.707, 0)
    // Joint 1 world transform adds translation (0, 1, 0) back
    let expected_x = -0.707; // -sin(45°)
    let expected_y = 0.707 + 1.0; // cos(45°) + translation

    let error_x = (skinned_pos.x - expected_x).abs();
    let error_y = (skinned_pos.y - expected_y).abs();

    assert!(
        error_x < 0.05,
        "Skinned X should be ~{}, got {}",
        expected_x,
        skinned_pos.x
    );
    assert!(
        error_y < 0.05,
        "Skinned Y should be ~{}, got {}",
        expected_y,
        skinned_pos.y
    );
    assert!(skinned_pos.z.abs() < 0.01, "Skinned Z should be ~0");

    // Normal should also be rotated
    assert!(
        (skinned_normal.length() - 1.0).abs() < 1e-5,
        "Normal should be normalized"
    );
}

/// Golden baseline: Full animation cycle sampling
#[test]
fn test_animated_pose_full_cycle() {
    let skeleton = create_animation_test_skeleton();
    let clip = create_test_animation_clip();

    // Sample at multiple time points
    let times = vec![0.0, 0.5, 1.0, 1.5, 2.0];
    let mut rotation_angles = Vec::new();

    for &t in &times {
        let poses = clip.sample(t, &skeleton);
        let rotation = poses[1].rotation;

        // Extract Z-axis rotation angle
        let (axis, angle) = rotation.to_axis_angle();
        let z_angle = if axis.z > 0.0 { angle } else { -angle };
        rotation_angles.push(z_angle);
    }

    // Verify monotonic increase
    for i in 1..rotation_angles.len() {
        assert!(
            rotation_angles[i] >= rotation_angles[i - 1],
            "Rotation should increase monotonically: {} < {} at index {}",
            rotation_angles[i],
            rotation_angles[i - 1],
            i
        );
    }

    // Verify start and end angles
    assert!(rotation_angles[0].abs() < 0.01, "Should start at 0°");
    assert!(
        (rotation_angles[4] - PI / 2.0).abs() < 0.01,
        "Should end at 90°"
    );
}

/// Golden baseline: Clamping at animation end
#[test]
fn test_animated_pose_clamping_beyond_duration() {
    let skeleton = create_animation_test_skeleton();
    let clip = create_test_animation_clip();

    // Sample beyond duration (t=3.0, duration=2.0)
    let poses_beyond = clip.sample(3.0, &skeleton);
    let poses_end = clip.sample(2.0, &skeleton);

    // Should clamp to end pose
    for i in 0..poses_beyond.len() {
        let diff_t = (poses_beyond[i].translation - poses_end[i].translation).length();
        let diff_r = (poses_beyond[i].rotation.dot(poses_end[i].rotation) - 1.0).abs();

        assert!(diff_t < 1e-5, "Translation should clamp to end");
        assert!(diff_r < 1e-5, "Rotation should clamp to end");
    }
}

/// Golden baseline: Negative time clamping
#[test]
fn test_animated_pose_clamping_negative_time() {
    let skeleton = create_animation_test_skeleton();
    let clip = create_test_animation_clip();

    // Sample at negative time
    let poses_neg = clip.sample(-1.0, &skeleton);
    let poses_start = clip.sample(0.0, &skeleton);

    // Should clamp to start pose
    for i in 0..poses_neg.len() {
        let diff_t = (poses_neg[i].translation - poses_start[i].translation).length();
        let diff_r = (poses_neg[i].rotation.dot(poses_start[i].rotation) - 1.0).abs();

        assert!(
            diff_t < 1e-5,
            "Translation should clamp to start at negative time"
        );
        assert!(
            diff_r < 1e-5,
            "Rotation should clamp to start at negative time"
        );
    }
}

/// Golden baseline: Hierarchical propagation verification
#[test]
fn test_animated_pose_hierarchical_propagation() {
    let skeleton = create_animation_test_skeleton();
    let clip = create_test_animation_clip();

    let local_poses = clip.sample(1.0, &skeleton);
    let joint_matrices = compute_joint_matrices(&skeleton, &local_poses).unwrap();

    // Joint 2 (child of joint 1) should inherit rotation
    // Check by verifying translation is rotated
    let joint2_world_pos = joint_matrices[2].w_axis.truncate();

    // Without rotation: child2 would be at (0, 2, 0)
    // With 45° rotation on joint1: child2 is rotated around joint1's origin
    // Expected position calculation:
    // joint1 at (0, 1, 0) rotated 45° = (-0.707, 0.707, 0)
    // joint2 local (0, 1, 0) from joint1 = rotate(0, 1, 0) around joint1

    // The exact position depends on rotation composition
    // Just verify it's NOT at (0, 2, 0) (unrotated)
    let unrotated_pos = Vec3::new(0.0, 2.0, 0.0);
    let distance_from_unrotated = (joint2_world_pos - unrotated_pos).length();

    assert!(
        distance_from_unrotated > 0.1,
        "Joint 2 should be affected by parent rotation. Got: {:?}",
        joint2_world_pos
    );
}
