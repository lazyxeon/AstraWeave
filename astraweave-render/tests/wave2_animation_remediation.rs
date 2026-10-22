//! Wave 2 proactive remediation tests for animation.rs (93 mutants, 12 existing tests).
//!
//! Targets:
//!   - Transform::lerp boundary values (t=0, t=1, t=0.5)
//!   - AnimationState update edge cases (speed, reverse, paused, zero dt)
//!   - AnimationState play/pause/stop/restart state transitions
//!   - find_keyframes exact-match boundaries, all positions
//!   - skin_vertex_cpu with multiple joints, zero weights, out-of-bounds
//!   - JointPalette overflow / empty / from_matrices
//!   - compute_joint_matrices deep hierarchy, inverse bind

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::animation::{
    compute_joint_matrices, skin_vertex_cpu, AnimationChannel, AnimationClip, AnimationState,
    ChannelData, Interpolation, Joint, JointMatrixGPU, JointPalette, Skeleton, Transform,
    MAX_JOINTS,
};
use glam::{Mat4, Quat, Vec3};

// ══════════════════════════════════════════════════════════════════════════════
// Transform::lerp
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn lerp_t0_returns_self() {
    let a = Transform {
        translation: Vec3::new(1.0, 2.0, 3.0),
        rotation: Quat::from_rotation_y(0.5),
        scale: Vec3::new(2.0, 2.0, 2.0),
    };
    let b = Transform {
        translation: Vec3::new(10.0, 20.0, 30.0),
        rotation: Quat::from_rotation_y(1.5),
        scale: Vec3::new(4.0, 4.0, 4.0),
    };
    let r = a.lerp(&b, 0.0);
    assert!((r.translation - a.translation).length() < 1e-6);
    assert!((r.rotation.dot(a.rotation)).abs() > 0.9999);
    assert!((r.scale - a.scale).length() < 1e-6);
}

#[test]
fn lerp_t1_returns_other() {
    let a = Transform::default();
    let b = Transform {
        translation: Vec3::new(10.0, 20.0, 30.0),
        rotation: Quat::from_rotation_z(1.0),
        scale: Vec3::new(3.0, 3.0, 3.0),
    };
    let r = a.lerp(&b, 1.0);
    assert!((r.translation - b.translation).length() < 1e-5);
    assert!((r.rotation.dot(b.rotation)).abs() > 0.9999);
    assert!((r.scale - b.scale).length() < 1e-5);
}

#[test]
fn lerp_t05_is_midpoint() {
    let a = Transform {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };
    let b = Transform {
        translation: Vec3::new(10.0, 0.0, 0.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::new(3.0, 3.0, 3.0),
    };
    let r = a.lerp(&b, 0.5);
    assert!((r.translation.x - 5.0).abs() < 1e-5);
    assert!((r.scale.x - 2.0).abs() < 1e-5);
}

#[test]
fn lerp_identity_to_identity_is_identity() {
    let id = Transform::default();
    let r = id.lerp(&id, 0.5);
    assert!((r.translation - Vec3::ZERO).length() < 1e-6);
    assert!((r.rotation.dot(Quat::IDENTITY)).abs() > 0.9999);
    assert!((r.scale - Vec3::ONE).length() < 1e-6);
}

// ══════════════════════════════════════════════════════════════════════════════
// Transform::to_matrix
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn to_matrix_identity_is_mat4_identity() {
    let t = Transform::default();
    let m = t.to_matrix();
    for i in 0..4 {
        for j in 0..4 {
            let expected = if i == j { 1.0 } else { 0.0 };
            assert!(
                (m.col(i)[j] - expected).abs() < 1e-6,
                "Identity transform matrix at ({i},{j}) = {} ≠ {expected}",
                m.col(i)[j]
            );
        }
    }
}

#[test]
fn to_matrix_translation_in_w_axis() {
    let t = Transform {
        translation: Vec3::new(5.0, -3.0, 7.0),
        ..Default::default()
    };
    let m = t.to_matrix();
    assert!((m.w_axis.x - 5.0).abs() < 1e-6);
    assert!((m.w_axis.y + 3.0).abs() < 1e-6);
    assert!((m.w_axis.z - 7.0).abs() < 1e-6);
}

#[test]
fn to_matrix_scale_on_diagonal() {
    let t = Transform {
        scale: Vec3::new(2.0, 3.0, 4.0),
        ..Default::default()
    };
    let m = t.to_matrix();
    assert!((m.x_axis.x - 2.0).abs() < 1e-6);
    assert!((m.y_axis.y - 3.0).abs() < 1e-6);
    assert!((m.z_axis.z - 4.0).abs() < 1e-6);
}

// ══════════════════════════════════════════════════════════════════════════════
// AnimationState update — edge cases
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn update_paused_no_time_advance() {
    let mut s = AnimationState {
        playing: false,
        time: 0.5,
        speed: 1.0,
        ..Default::default()
    };
    s.update(1.0, 10.0);
    assert_eq!(s.time, 0.5, "paused → time unchanged");
}

#[test]
fn update_zero_dt_no_change() {
    let mut s = AnimationState {
        playing: true,
        time: 0.5,
        speed: 1.0,
        ..Default::default()
    };
    s.update(0.0, 10.0);
    assert_eq!(s.time, 0.5);
}

#[test]
fn update_speed_multiplies_dt() {
    let mut s = AnimationState {
        playing: true,
        time: 0.0,
        speed: 2.0,
        looping: false,
        ..Default::default()
    };
    s.update(0.5, 10.0);
    assert!(
        (s.time - 1.0).abs() < 1e-6,
        "speed=2 × dt=0.5 → time=1.0, got {}",
        s.time
    );
}

#[test]
fn update_negative_speed_goes_backward() {
    let mut s = AnimationState {
        playing: true,
        time: 5.0,
        speed: -1.0,
        looping: true,
        ..Default::default()
    };
    s.update(1.0, 10.0);
    assert!(
        (s.time - 4.0).abs() < 1e-6,
        "time should decrease with negative speed"
    );
}

#[test]
fn update_looping_wraps_around() {
    let mut s = AnimationState {
        playing: true,
        time: 9.5,
        speed: 1.0,
        looping: true,
        ..Default::default()
    };
    s.update(1.0, 10.0); // 9.5 + 1.0 = 10.5 → wraps to 0.5
    assert!(
        (s.time - 0.5).abs() < 1e-3,
        "looping wrap: expected ~0.5, got {}",
        s.time
    );
    assert!(s.playing, "looping should stay playing");
}

#[test]
fn update_non_looping_clamps_and_stops() {
    let mut s = AnimationState {
        playing: true,
        time: 9.5,
        speed: 1.0,
        looping: false,
        ..Default::default()
    };
    s.update(1.0, 10.0);
    assert_eq!(s.time, 10.0, "clamped to duration");
    assert!(!s.playing, "should stop at end");
}

#[test]
fn update_looping_negative_wraps() {
    let mut s = AnimationState {
        playing: true,
        time: 0.5,
        speed: -1.0,
        looping: true,
        ..Default::default()
    };
    s.update(1.0, 10.0); // 0.5 - 1.0 = -0.5 → wraps to 9.5
    assert!(
        (s.time - 9.5).abs() < 0.1,
        "negative wrap: expected ~9.5, got {}",
        s.time
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// AnimationState play/pause/stop/restart state transitions
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn play_sets_playing_true() {
    let mut s = AnimationState::default();
    assert!(!s.playing);
    s.play();
    assert!(s.playing);
}

#[test]
fn pause_sets_playing_false() {
    let mut s = AnimationState {
        playing: true,
        ..Default::default()
    };
    s.pause();
    assert!(!s.playing);
}

#[test]
fn pause_preserves_time() {
    let mut s = AnimationState {
        playing: true,
        time: 3.5,
        ..Default::default()
    };
    s.pause();
    assert_eq!(s.time, 3.5, "pause should keep current time");
}

#[test]
fn stop_resets_time_and_stops() {
    let mut s = AnimationState {
        playing: true,
        time: 5.0,
        ..Default::default()
    };
    s.stop();
    assert!(!s.playing);
    assert_eq!(s.time, 0.0);
}

#[test]
fn restart_resets_time_and_plays() {
    let mut s = AnimationState {
        playing: false,
        time: 5.0,
        ..Default::default()
    };
    s.restart();
    assert!(s.playing);
    assert_eq!(s.time, 0.0);
}

// ══════════════════════════════════════════════════════════════════════════════
// AnimationClip::sample — channel interpolation
// ══════════════════════════════════════════════════════════════════════════════

fn single_joint_skeleton() -> Skeleton {
    Skeleton {
        joints: vec![Joint {
            name: "root".into(),
            parent_index: None,
            inverse_bind_matrix: Mat4::IDENTITY,
            local_transform: Transform::default(),
        }],
        root_indices: vec![0],
    }
}

fn two_joint_skeleton() -> Skeleton {
    Skeleton {
        joints: vec![
            Joint {
                name: "root".into(),
                parent_index: None,
                inverse_bind_matrix: Mat4::IDENTITY,
                local_transform: Transform::default(),
            },
            Joint {
                name: "child".into(),
                parent_index: Some(0),
                inverse_bind_matrix: Mat4::IDENTITY,
                local_transform: Transform::default(),
            },
        ],
        root_indices: vec![0],
    }
}

#[test]
fn sample_translation_linear_midpoint() {
    let skel = single_joint_skeleton();
    let clip = AnimationClip {
        name: "move".into(),
        duration: 2.0,
        channels: vec![AnimationChannel {
            target_joint_index: 0,
            times: vec![0.0, 2.0],
            data: ChannelData::Translation(vec![Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)]),
            interpolation: Interpolation::Linear,
        }],
    };
    let t = clip.sample(1.0, &skel);
    assert!(
        (t[0].translation.x - 5.0).abs() < 1e-5,
        "midpoint should be 5.0"
    );
}

#[test]
fn sample_translation_step_no_interpolation() {
    let skel = single_joint_skeleton();
    let clip = AnimationClip {
        name: "step".into(),
        duration: 2.0,
        channels: vec![AnimationChannel {
            target_joint_index: 0,
            times: vec![0.0, 1.0, 2.0],
            data: ChannelData::Translation(vec![
                Vec3::ZERO,
                Vec3::new(10.0, 0.0, 0.0),
                Vec3::new(20.0, 0.0, 0.0),
            ]),
            interpolation: Interpolation::Step,
        }],
    };
    let t = clip.sample(0.5, &skel);
    // Step: should return value at idx0 (0.0)
    assert_eq!(
        t[0].translation,
        Vec3::ZERO,
        "step interp should use keyframe[0]"
    );
    let t2 = clip.sample(1.5, &skel);
    assert_eq!(
        t2[0].translation,
        Vec3::new(10.0, 0.0, 0.0),
        "step should jump to keyframe[1]"
    );
}

#[test]
fn sample_rotation_linear_slerp() {
    let skel = single_joint_skeleton();
    let q_start = Quat::IDENTITY;
    let q_end = Quat::from_rotation_z(std::f32::consts::PI);
    let clip = AnimationClip {
        name: "rot".into(),
        duration: 1.0,
        channels: vec![AnimationChannel {
            target_joint_index: 0,
            times: vec![0.0, 1.0],
            data: ChannelData::Rotation(vec![q_start, q_end]),
            interpolation: Interpolation::Linear,
        }],
    };
    let t = clip.sample(0.5, &skel);
    // At t=0.5, rotation should be ~90 degrees
    let expected = q_start.slerp(q_end, 0.5);
    assert!((t[0].rotation.dot(expected)).abs() > 0.999);
}

#[test]
fn sample_scale_linear() {
    let skel = single_joint_skeleton();
    let clip = AnimationClip {
        name: "scale".into(),
        duration: 1.0,
        channels: vec![AnimationChannel {
            target_joint_index: 0,
            times: vec![0.0, 1.0],
            data: ChannelData::Scale(vec![Vec3::ONE, Vec3::new(3.0, 3.0, 3.0)]),
            interpolation: Interpolation::Linear,
        }],
    };
    let t = clip.sample(0.5, &skel);
    assert!((t[0].scale.x - 2.0).abs() < 1e-5);
}

#[test]
fn sample_out_of_range_joint_index_ignored() {
    let skel = single_joint_skeleton();
    let clip = AnimationClip {
        name: "bad".into(),
        duration: 1.0,
        channels: vec![AnimationChannel {
            target_joint_index: 99, // Invalid
            times: vec![0.0, 1.0],
            data: ChannelData::Translation(vec![Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)]),
            interpolation: Interpolation::Linear,
        }],
    };
    // Should not panic
    let t = clip.sample(0.5, &skel);
    assert_eq!(t.len(), 1);
    assert_eq!(
        t[0].translation,
        Vec3::ZERO,
        "invalid joint → fallback to bind pose"
    );
}

#[test]
fn sample_multiple_channels_compose() {
    let skel = single_joint_skeleton();
    let clip = AnimationClip {
        name: "compose".into(),
        duration: 1.0,
        channels: vec![
            AnimationChannel {
                target_joint_index: 0,
                times: vec![0.0, 1.0],
                data: ChannelData::Translation(vec![Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0)]),
                interpolation: Interpolation::Linear,
            },
            AnimationChannel {
                target_joint_index: 0,
                times: vec![0.0, 1.0],
                data: ChannelData::Scale(vec![Vec3::ONE, Vec3::new(2.0, 2.0, 2.0)]),
                interpolation: Interpolation::Linear,
            },
        ],
    };
    let t = clip.sample(0.5, &skel);
    assert!(
        (t[0].translation.x - 2.5).abs() < 1e-5,
        "translation channel at 0.5"
    );
    assert!((t[0].scale.x - 1.5).abs() < 1e-5, "scale channel at 0.5");
}

// ══════════════════════════════════════════════════════════════════════════════
// skin_vertex_cpu — additional edge cases
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn skin_all_zero_weights_returns_zero() {
    let (pos, norm) = skin_vertex_cpu(
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::Y,
        [0, 0, 0, 0],
        [0.0, 0.0, 0.0, 0.0],
        &[Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0))],
    );
    assert_eq!(pos, Vec3::ZERO, "all zero weights → zero position");
    assert_eq!(norm, Vec3::ZERO, "all zero weights → zero normal");
}

#[test]
fn skin_four_joints_equally_weighted() {
    let joints = [0u16, 1, 2, 3];
    let weights = [0.25, 0.25, 0.25, 0.25];
    let matrices = vec![
        Mat4::from_translation(Vec3::new(4.0, 0.0, 0.0)),
        Mat4::from_translation(Vec3::new(0.0, 4.0, 0.0)),
        Mat4::from_translation(Vec3::new(0.0, 0.0, 4.0)),
        Mat4::from_translation(Vec3::new(-4.0, 0.0, 0.0)),
    ];
    let (pos, _) = skin_vertex_cpu(Vec3::ZERO, Vec3::Y, joints, weights, &matrices);
    // Average of translations: (4+0+0-4)/4=0, (0+4+0+0)/4=1, (0+0+4+0)/4=1
    assert!((pos.x - 0.0).abs() < 1e-5);
    assert!((pos.y - 1.0).abs() < 1e-5);
    assert!((pos.z - 1.0).abs() < 1e-5);
}

#[test]
fn skin_out_of_bounds_joint_skipped() {
    let (pos, _) = skin_vertex_cpu(
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::Y,
        [99, 0, 0, 0], // joint 99 out of bounds
        [0.5, 0.5, 0.0, 0.0],
        &[Mat4::from_translation(Vec3::new(2.0, 0.0, 0.0))],
    );
    // Only joint 0 contributes (weight 0.5): 0.5 * (1+2, 0, 0) = (1.5, 0, 0)
    assert!((pos.x - 1.5).abs() < 1e-5);
}

#[test]
fn skin_rotation_transforms_normal() {
    let rot = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let (_, norm) = skin_vertex_cpu(
        Vec3::ZERO,
        Vec3::X,
        [0, 0, 0, 0],
        [1.0, 0.0, 0.0, 0.0],
        &[rot],
    );
    // Normal X rotated 90° around Z → Y
    assert!((norm.x).abs() < 1e-5);
    assert!((norm.y - 1.0).abs() < 1e-5);
}

#[test]
fn skin_scaling_matrix_on_position() {
    let scale = Mat4::from_scale(Vec3::new(2.0, 3.0, 4.0));
    let (pos, _) = skin_vertex_cpu(
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::Y,
        [0, 0, 0, 0],
        [1.0, 0.0, 0.0, 0.0],
        &[scale],
    );
    assert!((pos.x - 2.0).abs() < 1e-5);
    assert!((pos.y - 3.0).abs() < 1e-5);
    assert!((pos.z - 4.0).abs() < 1e-5);
}

// ══════════════════════════════════════════════════════════════════════════════
// JointPalette
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn joint_palette_default_zero_count() {
    let p = JointPalette::default();
    assert_eq!(p.joint_count, 0);
}

#[test]
fn joint_palette_default_joints_are_identity() {
    let p = JointPalette::default();
    let id = Mat4::IDENTITY.to_cols_array_2d();
    for i in 0..MAX_JOINTS {
        assert_eq!(
            p.joints[i].matrix, id,
            "default joint {i} should be identity"
        );
    }
}

#[test]
fn joint_palette_from_empty() {
    let p = JointPalette::from_matrices(&[]);
    assert_eq!(p.joint_count, 0);
}

#[test]
fn joint_palette_from_matrices_stores_correctly() {
    let m = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
    let p = JointPalette::from_matrices(&[m, Mat4::IDENTITY]);
    assert_eq!(p.joint_count, 2);
    assert_eq!(p.joints[0].matrix, m.to_cols_array_2d());
    assert_eq!(p.joints[1].matrix, Mat4::IDENTITY.to_cols_array_2d());
}

#[test]
fn joint_palette_caps_at_max_joints() {
    let matrices: Vec<Mat4> = (0..MAX_JOINTS + 50)
        .map(|i| Mat4::from_translation(Vec3::new(i as f32, 0.0, 0.0)))
        .collect();
    let p = JointPalette::from_matrices(&matrices);
    assert_eq!(p.joint_count, MAX_JOINTS as u32);
}

// ══════════════════════════════════════════════════════════════════════════════
// JointMatrixGPU
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn joint_matrix_gpu_from_mat4() {
    let m = Mat4::from_translation(Vec3::new(7.0, 8.0, 9.0));
    let gpu: JointMatrixGPU = m.into();
    assert_eq!(gpu.matrix, m.to_cols_array_2d());
}

// ══════════════════════════════════════════════════════════════════════════════
// compute_joint_matrices — deeper hierarchies
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn joint_matrices_three_level_chain() {
    let skeleton = Skeleton {
        joints: vec![
            Joint {
                name: "root".into(),
                parent_index: None,
                inverse_bind_matrix: Mat4::IDENTITY,
                local_transform: Transform::default(),
            },
            Joint {
                name: "mid".into(),
                parent_index: Some(0),
                inverse_bind_matrix: Mat4::IDENTITY,
                local_transform: Transform::default(),
            },
            Joint {
                name: "tip".into(),
                parent_index: Some(1),
                inverse_bind_matrix: Mat4::IDENTITY,
                local_transform: Transform::default(),
            },
        ],
        root_indices: vec![0],
    };

    let transforms = vec![
        Transform {
            translation: Vec3::new(1.0, 0.0, 0.0),
            ..Default::default()
        },
        Transform {
            translation: Vec3::new(2.0, 0.0, 0.0),
            ..Default::default()
        },
        Transform {
            translation: Vec3::new(3.0, 0.0, 0.0),
            ..Default::default()
        },
    ];

    let mats = compute_joint_matrices(&skeleton, &transforms).unwrap();
    assert_eq!(mats.len(), 3);
    // root: (1,0,0), mid: (1+2,0,0)=(3,0,0), tip: (3+3,0,0)=(6,0,0)
    assert!((mats[0].w_axis.x - 1.0).abs() < 1e-6);
    assert!((mats[1].w_axis.x - 3.0).abs() < 1e-6);
    assert!((mats[2].w_axis.x - 6.0).abs() < 1e-6);
}

#[test]
fn joint_matrices_with_inverse_bind() {
    let skeleton = Skeleton {
        joints: vec![Joint {
            name: "root".into(),
            parent_index: None,
            inverse_bind_matrix: Mat4::from_translation(Vec3::new(-1.0, 0.0, 0.0)),
            local_transform: Transform::default(),
        }],
        root_indices: vec![0],
    };

    let transforms = vec![Transform {
        translation: Vec3::new(5.0, 0.0, 0.0),
        ..Default::default()
    }];

    let mats = compute_joint_matrices(&skeleton, &transforms).unwrap();
    // world_matrix = translate(5,0,0), skinning = world * inverse_bind = translate(5-1,0,0)
    assert!((mats[0].w_axis.x - 4.0).abs() < 1e-6);
}

#[test]
fn joint_matrices_with_rotation() {
    let skeleton = Skeleton {
        joints: vec![
            Joint {
                name: "root".into(),
                parent_index: None,
                inverse_bind_matrix: Mat4::IDENTITY,
                local_transform: Transform::default(),
            },
            Joint {
                name: "child".into(),
                parent_index: Some(0),
                inverse_bind_matrix: Mat4::IDENTITY,
                local_transform: Transform::default(),
            },
        ],
        root_indices: vec![0],
    };

    let transforms = vec![
        Transform {
            rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            ..Default::default()
        },
        Transform {
            translation: Vec3::new(1.0, 0.0, 0.0),
            ..Default::default()
        },
    ];

    let mats = compute_joint_matrices(&skeleton, &transforms).unwrap();
    // Child translation (1,0,0) in parent's rotated frame (90° Z) → (0,1,0)
    assert!((mats[1].w_axis.x).abs() < 1e-5);
    assert!((mats[1].w_axis.y - 1.0).abs() < 1e-5);
}

// ══════════════════════════════════════════════════════════════════════════════
// AnimationClip::sample — edge cases with find_keyframes
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn sample_at_time_zero() {
    let skel = single_joint_skeleton();
    let clip = AnimationClip {
        name: "t".into(),
        duration: 1.0,
        channels: vec![AnimationChannel {
            target_joint_index: 0,
            times: vec![0.0, 1.0],
            data: ChannelData::Translation(vec![
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
            ]),
            interpolation: Interpolation::Linear,
        }],
    };
    let t = clip.sample(0.0, &skel);
    assert!((t[0].translation.x - 1.0).abs() < 1e-5);
}

#[test]
fn sample_at_duration() {
    let skel = single_joint_skeleton();
    let clip = AnimationClip {
        name: "t".into(),
        duration: 1.0,
        channels: vec![AnimationChannel {
            target_joint_index: 0,
            times: vec![0.0, 1.0],
            data: ChannelData::Translation(vec![
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
            ]),
            interpolation: Interpolation::Linear,
        }],
    };
    let t = clip.sample(1.0, &skel);
    assert!((t[0].translation.x - 2.0).abs() < 1e-5);
}

#[test]
fn sample_before_first_keyframe() {
    let skel = single_joint_skeleton();
    let clip = AnimationClip {
        name: "t".into(),
        duration: 2.0,
        channels: vec![AnimationChannel {
            target_joint_index: 0,
            times: vec![0.5, 1.5],
            data: ChannelData::Translation(vec![
                Vec3::new(3.0, 0.0, 0.0),
                Vec3::new(7.0, 0.0, 0.0),
            ]),
            interpolation: Interpolation::Linear,
        }],
    };
    let t = clip.sample(0.0, &skel);
    // Before first keyframe → use first value
    assert!((t[0].translation.x - 3.0).abs() < 1e-5);
}

#[test]
fn sample_after_last_keyframe() {
    let skel = single_joint_skeleton();
    let clip = AnimationClip {
        name: "t".into(),
        duration: 5.0,
        channels: vec![AnimationChannel {
            target_joint_index: 0,
            times: vec![0.0, 1.0],
            data: ChannelData::Translation(vec![
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(9.0, 0.0, 0.0),
            ]),
            interpolation: Interpolation::Linear,
        }],
    };
    let t = clip.sample(3.0, &skel);
    // After last keyframe → use last value
    assert!((t[0].translation.x - 9.0).abs() < 1e-5);
}

#[test]
fn sample_empty_channels() {
    let skel = single_joint_skeleton();
    let clip = AnimationClip {
        name: "empty".into(),
        duration: 1.0,
        channels: vec![],
    };
    let t = clip.sample(0.5, &skel);
    // Should return default (bind pose)
    assert_eq!(t[0].translation, Vec3::ZERO);
}

#[test]
fn sample_multi_joint_independent() {
    let skel = two_joint_skeleton();
    let clip = AnimationClip {
        name: "multi".into(),
        duration: 1.0,
        channels: vec![
            AnimationChannel {
                target_joint_index: 0,
                times: vec![0.0, 1.0],
                data: ChannelData::Translation(vec![Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)]),
                interpolation: Interpolation::Linear,
            },
            AnimationChannel {
                target_joint_index: 1,
                times: vec![0.0, 1.0],
                data: ChannelData::Translation(vec![Vec3::ZERO, Vec3::new(0.0, 20.0, 0.0)]),
                interpolation: Interpolation::Linear,
            },
        ],
    };
    let t = clip.sample(0.5, &skel);
    assert!((t[0].translation.x - 5.0).abs() < 1e-5);
    assert!((t[1].translation.y - 10.0).abs() < 1e-5);
}

// ══════════════════════════════════════════════════════════════════════════════
// MAX_JOINTS constant
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn max_joints_is_256() {
    assert_eq!(MAX_JOINTS, 256);
}
