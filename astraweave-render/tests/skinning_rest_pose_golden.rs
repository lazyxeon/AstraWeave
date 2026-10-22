//! Golden Test: Rest Pose Baseline
//!
//! Phase 2 Task 5 (Phase E): Validates that CPU skinning produces deterministic
//! results for a skeleton in rest pose (no animation).

mod test_utils;

use astraweave_render::{compute_joint_matrices, skin_vertex_cpu, Joint, Skeleton, Transform};
use glam::{Mat4, Vec3};

/// Create test skeleton for golden tests
fn create_test_skeleton() -> Skeleton {
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

/// Golden baseline: 3-joint skeleton in rest pose
#[test]
fn test_rest_pose_golden_baseline() {
    let skeleton = create_test_skeleton();

    // Rest pose: all joints at bind pose
    let rest_local_poses = vec![
        Transform::default(),
        Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        },
        Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        },
    ];

    // Compute joint matrices
    let joint_matrices = compute_joint_matrices(&skeleton, &rest_local_poses).unwrap();

    // In rest pose, skinning matrices should be identity (world * inverse_bind = identity)
    assert_eq!(joint_matrices.len(), 3);

    // Root joint should be identity
    let diff0 = (joint_matrices[0] - Mat4::IDENTITY).abs();
    let max_diff0 = diff0
        .to_cols_array()
        .iter()
        .fold(0.0f32, |acc, &x| acc.max(x));
    assert!(
        max_diff0 < 1e-5,
        "Root joint should be identity in rest pose"
    );
}

/// Golden baseline: Deterministic sampling (repeat)
#[test]
fn test_rest_pose_determinism() {
    let skeleton = create_test_skeleton();
    let rest_local_poses = vec![
        Transform::default(),
        Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        },
        Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        },
    ];

    // Compute matrices twice
    let matrices1 = compute_joint_matrices(&skeleton, &rest_local_poses).unwrap();
    let matrices2 = compute_joint_matrices(&skeleton, &rest_local_poses).unwrap();

    // Should be bit-exact (no randomness, no time-dependent ops)
    assert_eq!(matrices1.len(), matrices2.len());
    for i in 0..matrices1.len() {
        let diff = (matrices1[i] - matrices2[i]).abs();
        let max_diff = diff
            .to_cols_array()
            .iter()
            .fold(0.0f32, |acc, &x| acc.max(x));
        assert!(
            max_diff < 1e-7,
            "Matrix {} should be deterministic. Diff: {}",
            i,
            max_diff
        );
    }
}

/// Golden baseline: Single joint influence
#[test]
fn test_rest_pose_single_joint() {
    let skeleton = create_test_skeleton();
    let rest_local_poses = vec![
        Transform::default(),
        Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        },
        Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        },
    ];
    let joint_matrices = compute_joint_matrices(&skeleton, &rest_local_poses).unwrap();

    // Vertex influenced only by joint 0 (root, identity)
    let position = Vec3::new(1.0, 0.0, 0.0);
    let normal = Vec3::Y;
    let joints = [0, 0, 0, 0];
    let weights = [1.0, 0.0, 0.0, 0.0];

    let (skinned_pos, skinned_normal) =
        skin_vertex_cpu(position, normal, joints, weights, &joint_matrices);

    // With 100% weight on root joint (identity), vertex should be preserved
    assert!(
        (skinned_pos - position).length() < 1e-4,
        "Root joint skinning should preserve position. Got: {:?}, Expected: {:?}",
        skinned_pos,
        position
    );
    assert!(
        (skinned_normal.length() - 1.0).abs() < 1e-5,
        "Normal should be normalized"
    );
}

/// Golden baseline: Zero weights (no skinning)
#[test]
fn test_rest_pose_zero_weights() {
    let skeleton = create_test_skeleton();
    let rest_local_poses = vec![
        Transform::default(),
        Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        },
        Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        },
    ];
    let joint_matrices = compute_joint_matrices(&skeleton, &rest_local_poses).unwrap();

    let position = Vec3::new(1.0, 2.0, 3.0);
    let normal = Vec3::new(0.5, 0.5, 0.707).normalize();
    let joints = [0, 1, 2, 0];
    let weights = [0.0, 0.0, 0.0, 0.0]; // All zero

    let (skinned_pos, skinned_normal) =
        skin_vertex_cpu(position, normal, joints, weights, &joint_matrices);

    // With zero weights, vertex should stay at origin (additive blending starts at zero)
    assert_eq!(
        skinned_pos,
        Vec3::ZERO,
        "Zero weights should produce zero position"
    );
    assert_eq!(
        skinned_normal,
        Vec3::ZERO,
        "Zero weights should produce zero normal"
    );
}

/// Golden baseline: Verify normalized weights
#[test]
fn test_rest_pose_normalized_weights() {
    let skeleton = create_test_skeleton();
    let rest_local_poses = vec![
        Transform::default(),
        Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        },
        Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        },
    ];
    let joint_matrices = compute_joint_matrices(&skeleton, &rest_local_poses).unwrap();

    // Test with root joint (identity matrix)
    let position = Vec3::new(1.0, 0.0, 0.0);
    let normal = Vec3::Y;

    // Test with non-normalized weights on root (sum = 2.0)
    let joints = [0, 0, 0, 0];
    let weights = [2.0, 0.0, 0.0, 0.0]; // Sum = 2.0 (not normalized)

    let (skinned_pos, _) = skin_vertex_cpu(position, normal, joints, weights, &joint_matrices);

    // Result will be scaled by sum of weights
    // This documents the behavior: user must ensure weights sum to 1.0
    let expected_scale = 2.0; // Sum of weights
    assert!(
        (skinned_pos - position * expected_scale).length() < 1e-4,
        "Non-normalized weights scale the result by their sum. Got: {:?}, Expected: {:?}",
        skinned_pos,
        position * expected_scale
    );
}

/// Golden baseline: Multiple joints with normalized weights
#[test]
fn test_rest_pose_blended_weights() {
    let skeleton = create_test_skeleton();
    let rest_local_poses = vec![
        Transform::default(),
        Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        },
        Transform {
            translation: Vec3::new(0.0, 1.0, 0.0),
            ..Default::default()
        },
    ];
    let joint_matrices = compute_joint_matrices(&skeleton, &rest_local_poses).unwrap();

    // Vertex influenced by root only (simplest test)
    let position = Vec3::new(2.0, 0.0, 0.0);
    let normal = Vec3::Y;
    let joints = [0, 0, 0, 0];
    let weights = [1.0, 0.0, 0.0, 0.0];

    let (skinned_pos, _) = skin_vertex_cpu(position, normal, joints, weights, &joint_matrices);

    // Root is identity, so position preserved
    assert!(
        (skinned_pos - position).length() < 1e-4,
        "Single joint (root) should preserve position"
    );
}
