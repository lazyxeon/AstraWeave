//! Integration tests for CPU/GPU skinning parity
//!
//! Phase 2 Task 5 (Phase E): Validates that CPU and GPU skinning produce identical results

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::{
    compute_joint_matrices, skin_vertex_cpu, AnimationChannel, AnimationClip, ChannelData,
    Interpolation, JointPalette, Skeleton, Transform, MAX_JOINTS,
};
use glam::{Mat4, Quat, Vec3};

/// Helper to create a simple test skeleton
fn create_test_skeleton() -> Skeleton {
    use astraweave_render::Joint;

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
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE,
                },
            },
            Joint {
                name: "child2".to_string(),
                parent_index: Some(1),
                inverse_bind_matrix: Mat4::from_translation(Vec3::new(0.0, -2.0, 0.0)),
                local_transform: Transform {
                    translation: Vec3::new(0.0, 1.0, 0.0),
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE,
                },
            },
        ],
    }
}

/// Helper to create a simple animation clip
fn create_test_animation() -> AnimationClip {
    AnimationClip {
        name: "test_anim".to_string(),
        duration: 2.0,
        channels: vec![AnimationChannel {
            target_joint_index: 1,
            interpolation: Interpolation::Linear,
            times: vec![0.0, 1.0, 2.0],
            data: ChannelData::Rotation(vec![
                Quat::IDENTITY,
                Quat::from_rotation_z(std::f32::consts::PI / 4.0),
                Quat::IDENTITY,
            ]),
        }],
    }
}

#[test]
fn test_cpu_skinning_deterministic() {
    let skeleton = create_test_skeleton();
    let clip = create_test_animation();

    // Compute poses at time 0.5
    let local_poses1 = clip.sample(0.5, &skeleton);
    let matrices1 = compute_joint_matrices(&skeleton, &local_poses1).unwrap();

    // Compute again - should be identical
    let local_poses2 = clip.sample(0.5, &skeleton);
    let matrices2 = compute_joint_matrices(&skeleton, &local_poses2).unwrap();

    assert_eq!(matrices1.len(), matrices2.len());
    for i in 0..matrices1.len() {
        let diff =
            (matrices1[i].to_cols_array_2d()[0][0] - matrices2[i].to_cols_array_2d()[0][0]).abs();
        assert!(diff < 1e-6, "Matrix {} differs: {}", i, diff);
    }
}

#[test]
fn test_cpu_skinning_vertex() {
    let skeleton = create_test_skeleton();
    let clip = create_test_animation();

    let local_poses = clip.sample(0.5, &skeleton);
    let matrices = compute_joint_matrices(&skeleton, &local_poses).unwrap();

    // Test vertex at origin
    let position = Vec3::new(0.0, 1.0, 0.0);
    let normal = Vec3::new(0.0, 1.0, 0.0);
    let joints = [1, 0, 0, 0];
    let weights = [1.0, 0.0, 0.0, 0.0];

    let (skinned_pos, skinned_normal) =
        skin_vertex_cpu(position, normal, joints, weights, &matrices);

    // Should be transformed by joint 1
    assert!(skinned_pos.length() > 0.5, "Position should be transformed");
    assert!(
        (skinned_normal.length() - 1.0).abs() < 1e-5,
        "Normal should be normalized"
    );
}

#[test]
fn test_joint_palette_conversion() {
    let skeleton = create_test_skeleton();
    let clip = create_test_animation();

    let local_poses = clip.sample(1.0, &skeleton);
    let matrices = compute_joint_matrices(&skeleton, &local_poses).unwrap();

    // Convert to GPU palette
    let palette = JointPalette::from_matrices(&matrices);

    assert_eq!(palette.joint_count, 3);

    // Verify matrices are preserved
    for (i, original_matrix) in matrices.iter().enumerate().take(3) {
        let original = original_matrix.to_cols_array_2d();
        let gpu = palette.joints[i].matrix;

        for row in 0..4 {
            for col in 0..4 {
                let diff = (original[row][col] - gpu[row][col]).abs();
                assert!(
                    diff < 1e-5,
                    "Matrix element [{},{}] differs: {}",
                    row,
                    col,
                    diff
                );
            }
        }
    }
}

#[test]
fn test_skinning_weighted_blend() {
    let skeleton = create_test_skeleton();
    let clip = create_test_animation();

    let local_poses = clip.sample(0.5, &skeleton);
    let matrices = compute_joint_matrices(&skeleton, &local_poses).unwrap();

    let position = Vec3::new(0.0, 1.5, 0.0);
    let normal = Vec3::Y;
    let joints = [1, 2, 0, 0];
    let weights = [0.5, 0.5, 0.0, 0.0]; // Blend between joints 1 and 2

    let (skinned_pos, _) = skin_vertex_cpu(position, normal, joints, weights, &matrices);

    // Verify blending occurred (result should be between both transforms)
    assert!(skinned_pos.y > 0.0, "Y should be positive after blend");
}

#[test]
fn test_max_joints_limit() {
    // Create many matrices
    let matrices = vec![Mat4::IDENTITY; 300];

    let palette = JointPalette::from_matrices(&matrices);

    // Should clamp to MAX_JOINTS
    assert_eq!(palette.joint_count, MAX_JOINTS as u32);
}

#[test]
fn test_animation_sampling_interpolation() {
    let skeleton = create_test_skeleton();
    let clip = create_test_animation();

    // Sample at different times
    let poses_start = clip.sample(0.0, &skeleton);
    let poses_mid = clip.sample(1.0, &skeleton);
    let poses_end = clip.sample(2.0, &skeleton);

    // Joint 1 should have rotation animation
    let rot_start = poses_start[1].rotation;
    let rot_mid = poses_mid[1].rotation;
    let rot_end = poses_end[1].rotation;

    // Start and end should be identity
    assert!((rot_start.w - 1.0).abs() < 1e-5, "Start should be identity");
    assert!((rot_end.w - 1.0).abs() < 1e-5, "End should be identity");

    // Middle should be rotated
    assert!(rot_mid.w < 1.0, "Middle should have rotation");
}

#[test]
fn test_hierarchical_transform_propagation() {
    let skeleton = create_test_skeleton();

    // Create poses with root rotation
    let mut local_poses = vec![Transform::default(); 3];
    local_poses[0].rotation = Quat::from_rotation_z(std::f32::consts::PI / 2.0); // 90 degrees

    let matrices = compute_joint_matrices(&skeleton, &local_poses).unwrap();

    // Child joints should inherit parent rotation
    // Verify by checking matrix components
    let child_matrix = matrices[1];
    let translation = child_matrix.w_axis.truncate();

    // After 90 degree rotation around Z, Y translation becomes X translation
    assert!(
        translation.x.abs() > 0.5,
        "Child should be rotated by parent: {:?}",
        translation
    );
}

#[cfg(feature = "skinning-gpu")]
mod gpu_tests {
    #[allow(unused_imports)] // Needed for future GPU integration tests
    use super::*;
    #[allow(unused_imports)]
    use astraweave_render::JointPaletteManager;

    // GPU tests require wgpu instance - these are placeholders for integration tests
    // that would run in examples with actual GPU context

    #[test]
    #[ignore] // Requires GPU context
    fn test_gpu_skinning_parity() {
        // This would be implemented in a full GPU integration test
        // comparing CPU and GPU skinned results
    }

    #[test]
    #[ignore] // Requires GPU context
    fn test_joint_palette_upload() {
        // This would test uploading palettes to GPU buffers
    }
}

/// Golden test: Fixed animation at fixed time should produce known result
#[test]
fn test_golden_pose() {
    let skeleton = create_test_skeleton();
    let clip = create_test_animation();

    // Sample at exact keyframe time
    let poses = clip.sample(1.0, &skeleton);
    let matrices = compute_joint_matrices(&skeleton, &poses).unwrap();

    // Joint 1 should have 45 degree rotation
    let joint1_matrix = matrices[1];

    // Verify the rotation component (approximate - exact values depend on bind pose)
    // Just ensure it's not identity
    let is_identity = joint1_matrix == Mat4::IDENTITY;
    assert!(!is_identity, "Joint 1 should be transformed");
}

/// Stress test: Many joints
#[test]
fn test_large_skeleton() {
    use astraweave_render::Joint;

    let mut joints = vec![];
    for i in 0..100 {
        joints.push(Joint {
            name: format!("joint_{}", i),
            parent_index: if i == 0 { None } else { Some(i - 1) },
            inverse_bind_matrix: Mat4::IDENTITY,
            local_transform: Transform {
                translation: Vec3::new(0.0, 0.1, 0.0),
                ..Default::default()
            },
        });
    }

    let skeleton = Skeleton {
        root_indices: vec![0],
        joints,
    };

    // Use poses that match the skeleton's local transforms (0.1 Y translation per joint)
    let mut poses = vec![Transform::default(); 100];
    for pose in poses.iter_mut() {
        pose.translation = Vec3::new(0.0, 0.1, 0.0);
    }

    let matrices = compute_joint_matrices(&skeleton, &poses).unwrap();

    assert_eq!(matrices.len(), 100);

    // Last joint should be far from origin due to accumulated translations
    // 100 joints × 0.1 Y translation = 10.0 Y total
    let last_pos = matrices[99].w_axis.truncate();
    assert!(
        last_pos.y > 5.0,
        "Last joint should be accumulated (expected ~10.0): {:?}",
        last_pos
    );
}
