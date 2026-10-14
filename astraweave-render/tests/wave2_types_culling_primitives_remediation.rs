#![allow(unused, clippy::all)]
//! Wave 2 mutation-resistant remediation tests for types.rs, culling.rs, and primitives.rs
//!
//! Targets golden-value assertions for:
//!   - Instance::raw() normal_matrix computation (inverse().transpose())
//!   - cluster_index logarithmic z-slicing arithmetic
//!   - FrustumPlanes::from_view_proj Gribb-Hartmann sign extraction
//!   - FrustumPlanes::test_aabb per-component radius computation
//!   - InstanceAABB::from_transform corner transformation
//!   - primitives::sphere() trig math, vertex counts, radius scaling
//!   - primitives::cube() face normal direction signs
//!   - primitives::plane() y=0 constraint

use astraweave_render::culling::{
    batch_visible_instances, build_indirect_commands_cpu, cpu_frustum_cull, BatchId, DrawBatch,
    DrawIndirectCommand, FrustumPlanes, InstanceAABB,
};
use astraweave_render::primitives;
use astraweave_render::types::{
    cluster_index, ClusterDims, Instance, InstanceRaw, Material, Vertex,
};
use glam::{Mat4, Quat, Vec3};

// ══════════════════════════════════════════════════════════════════════════════
// Instance::raw() — normal_matrix golden values
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn instance_raw_identity_normal_matrix() {
    let inst = Instance {
        transform: Mat4::IDENTITY,
        color: [1.0; 4],
        material_id: 0,
    };
    let raw = inst.raw();
    // inverse(I).transpose() = I, so normal_matrix should be identity 3x3
    for i in 0..3 {
        for j in 0..3 {
            let expected = if i == j { 1.0 } else { 0.0 };
            assert!(
                (raw.normal_matrix[i][j] - expected).abs() < 1e-5,
                "normal_matrix[{i}][{j}] = {}, expected {expected}",
                raw.normal_matrix[i][j]
            );
        }
    }
}

#[test]
fn instance_raw_uniform_scale_normal_matrix() {
    // Uniform scale s → normal = (sI)^-T = (1/s)I
    let s = 3.0;
    let inst = Instance {
        transform: Mat4::from_scale(Vec3::splat(s)),
        color: [1.0; 4],
        material_id: 0,
    };
    let raw = inst.raw();
    let expected = 1.0 / s;
    for i in 0..3 {
        assert!(
            (raw.normal_matrix[i][i] - expected).abs() < 1e-4,
            "diagonal[{i}] = {}, expected {expected}",
            raw.normal_matrix[i][i]
        );
    }
}

#[test]
fn instance_raw_translation_doesnt_affect_normal() {
    let inst = Instance {
        transform: Mat4::from_translation(Vec3::new(100.0, 200.0, 300.0)),
        color: [1.0; 4],
        material_id: 0,
    };
    let raw = inst.raw();
    // Translation doesn't change normal matrix (still identity upper-left)
    for i in 0..3 {
        for j in 0..3 {
            let expected = if i == j { 1.0 } else { 0.0 };
            assert!(
                (raw.normal_matrix[i][j] - expected).abs() < 1e-4,
                "translation normal_matrix[{i}][{j}] = {}, expected {expected}",
                raw.normal_matrix[i][j]
            );
        }
    }
}

#[test]
fn instance_raw_rotation_normal_matrix_orthogonal() {
    // For rotation matrix R, normal = R^-T = R (since R^-1 = R^T for orthogonal)
    let rot = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let inst = Instance {
        transform: Mat4::from_quat(rot),
        color: [1.0; 4],
        material_id: 0,
    };
    let raw = inst.raw();
    // Rotation of 90° around Z: x-axis→y-axis, y-axis→-x-axis
    // normal_matrix[0] should be ≈ (0, 1, 0) (new x-axis direction)
    assert!((raw.normal_matrix[0][0]).abs() < 1e-4);
    assert!((raw.normal_matrix[0][1] - 1.0).abs() < 1e-4);
    assert!((raw.normal_matrix[0][2]).abs() < 1e-4);
}

#[test]
fn instance_raw_non_uniform_scale_normal_matrix() {
    // Non-uniform scale (2, 1, 1): normal = inverse(diag(2,1,1)).transpose() = diag(0.5, 1, 1)
    let inst = Instance {
        transform: Mat4::from_scale(Vec3::new(2.0, 1.0, 1.0)),
        color: [1.0; 4],
        material_id: 0,
    };
    let raw = inst.raw();
    assert!(
        (raw.normal_matrix[0][0] - 0.5).abs() < 1e-4,
        "x: {}",
        raw.normal_matrix[0][0]
    );
    assert!(
        (raw.normal_matrix[1][1] - 1.0).abs() < 1e-4,
        "y: {}",
        raw.normal_matrix[1][1]
    );
    assert!(
        (raw.normal_matrix[2][2] - 1.0).abs() < 1e-4,
        "z: {}",
        raw.normal_matrix[2][2]
    );
}

#[test]
fn instance_raw_model_matrix_stored_correctly() {
    let pos = Vec3::new(1.0, 2.0, 3.0);
    let scale = Vec3::new(4.0, 5.0, 6.0);
    let transform = Mat4::from_scale_rotation_translation(scale, Quat::IDENTITY, pos);
    let inst = Instance {
        transform,
        color: [0.5, 0.6, 0.7, 0.8],
        material_id: 99,
    };
    let raw = inst.raw();
    assert_eq!(raw.material_id, 99);
    assert_eq!(raw.color, [0.5, 0.6, 0.7, 0.8]);
    // Translation in w_axis (column-major: model[3])
    assert!((raw.model[3][0] - 1.0).abs() < 1e-5);
    assert!((raw.model[3][1] - 2.0).abs() < 1e-5);
    assert!((raw.model[3][2] - 3.0).abs() < 1e-5);
    // Scale on diagonal
    assert!((raw.model[0][0] - 4.0).abs() < 1e-5);
    assert!((raw.model[1][1] - 5.0).abs() < 1e-5);
    assert!((raw.model[2][2] - 6.0).abs() < 1e-5);
}

#[test]
fn instance_raw_padding_always_zero() {
    let inst = Instance {
        transform: Mat4::from_rotation_y(1.0) * Mat4::from_scale(Vec3::new(2.0, 3.0, 4.0)),
        color: [1.0; 4],
        material_id: 42,
    };
    let raw = inst.raw();
    assert_eq!(raw._padding, [0, 0, 0]);
}

#[test]
fn instance_from_pos_scale_color_material_id_zero() {
    let inst = Instance::from_pos_scale_color(Vec3::ONE, Vec3::ONE, [1.0; 4]);
    assert_eq!(inst.material_id, 0);
}

// ══════════════════════════════════════════════════════════════════════════════
// cluster_index — logarithmic z-slicing golden values
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn cluster_index_origin_near() {
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let idx = cluster_index(0, 0, 800, 800, 0.1, 0.1, 100.0, dims);
    // px=0,py=0 → sx=0,sy=0; depth=near → z_lin=0 → z_log=0
    assert_eq!(idx, 0);
}

#[test]
fn cluster_index_x_progression() {
    let dims = ClusterDims { x: 4, y: 1, z: 1 };
    // Each quarter of screen width maps to one x slice
    let i0 = cluster_index(0, 0, 400, 100, 0.1, 0.1, 100.0, dims);
    let i1 = cluster_index(100, 0, 400, 100, 0.1, 0.1, 100.0, dims);
    let i2 = cluster_index(200, 0, 400, 100, 0.1, 0.1, 100.0, dims);
    let i3 = cluster_index(300, 0, 400, 100, 0.1, 0.1, 100.0, dims);
    assert_eq!(i0, 0);
    assert_eq!(i1, 1);
    assert_eq!(i2, 2);
    assert_eq!(i3, 3);
}

#[test]
fn cluster_index_y_uses_stride_x() {
    let dims = ClusterDims { x: 4, y: 4, z: 1 };
    // sy=1 → index += dims.x = 4
    let i0 = cluster_index(0, 0, 400, 400, 0.1, 0.1, 100.0, dims);
    let i1 = cluster_index(0, 100, 400, 400, 0.1, 0.1, 100.0, dims);
    assert_eq!(i0, 0);
    assert_eq!(i1, 4); // sy=1 * dims.x=4
}

#[test]
fn cluster_index_z_uses_stride_xy() {
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    // z_log=1 → index += dims.x * dims.y = 16
    let near = 0.0;
    let far = 4.0;
    // depth in second quarter: z_lin = 0.375 → z_log = floor(0.375*4) = 1
    let idx = cluster_index(0, 0, 400, 400, 1.5, near, far, dims);
    assert_eq!(idx, 16); // 0 + 0 + 1*4*4
}

#[test]
fn cluster_index_max_clamp() {
    let dims = ClusterDims { x: 4, y: 4, z: 4 };
    let max_idx = dims.x * dims.y * dims.z;
    let idx = cluster_index(9999, 9999, 100, 100, 999.0, 0.1, 100.0, dims);
    assert!(idx < max_idx, "index {idx} >= max {max_idx}");
}

#[test]
fn cluster_index_depth_at_far_edge() {
    let dims = ClusterDims { x: 1, y: 1, z: 4 };
    let idx = cluster_index(0, 0, 100, 100, 99.9, 0.1, 100.0, dims);
    // z_lin ≈ 0.998 → z_log = floor(3.99) = 3
    assert_eq!(idx, 3);
}

// ══════════════════════════════════════════════════════════════════════════════
// FrustumPlanes — Gribb-Hartmann plane extraction & test_aabb
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn frustum_ortho_left_right_planes_oppose() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let f = FrustumPlanes::from_view_proj(&vp);
    // Left and right plane normals should point in opposite x directions
    let left_nx = f.planes[0][0];
    let right_nx = f.planes[1][0];
    assert!(
        left_nx * right_nx < 0.0,
        "left/right should oppose: {left_nx} vs {right_nx}"
    );
}

#[test]
fn frustum_ortho_bottom_top_planes_oppose() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let f = FrustumPlanes::from_view_proj(&vp);
    let bottom_ny = f.planes[2][1];
    let top_ny = f.planes[3][1];
    assert!(
        bottom_ny * top_ny < 0.0,
        "bottom/top should oppose: {bottom_ny} vs {top_ny}"
    );
}

#[test]
fn frustum_ortho_near_far_planes_oppose() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let f = FrustumPlanes::from_view_proj(&vp);
    let near_nz = f.planes[4][2];
    let far_nz = f.planes[5][2];
    assert!(
        near_nz * far_nz < 0.0,
        "near/far should oppose: {near_nz} vs {far_nz}"
    );
}

#[test]
fn frustum_perspective_planes_normalized() {
    let vp = Mat4::perspective_rh(1.0, 16.0 / 9.0, 0.1, 500.0);
    let f = FrustumPlanes::from_view_proj(&vp);
    for (i, plane) in f.planes.iter().enumerate() {
        let len = (plane[0] * plane[0] + plane[1] * plane[1] + plane[2] * plane[2]).sqrt();
        assert!((len - 1.0).abs() < 0.01, "plane {i} normal length={len}");
    }
}

#[test]
fn test_aabb_at_frustum_border_visible() {
    // AABB exactly at the edge should still be visible (inside, not outside)
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let f = FrustumPlanes::from_view_proj(&vp);
    // Place AABB right at x=9, extent=2 → overlaps boundary
    let visible = f.test_aabb(Vec3::new(9.0, 0.0, -50.0), Vec3::splat(2.0));
    assert!(visible, "AABB overlapping frustum border should be visible");
}

#[test]
fn test_aabb_just_outside_left() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let f = FrustumPlanes::from_view_proj(&vp);
    // AABB fully to the left: center at x=-15, extent=1 → entirely outside
    let visible = f.test_aabb(Vec3::new(-15.0, 0.0, -50.0), Vec3::splat(1.0));
    assert!(!visible, "AABB fully outside left should not be visible");
}

#[test]
fn test_aabb_just_outside_right() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let f = FrustumPlanes::from_view_proj(&vp);
    let visible = f.test_aabb(Vec3::new(15.0, 0.0, -50.0), Vec3::splat(1.0));
    assert!(!visible, "AABB fully outside right should not be visible");
}

#[test]
fn test_aabb_just_outside_top() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let f = FrustumPlanes::from_view_proj(&vp);
    let visible = f.test_aabb(Vec3::new(0.0, 15.0, -50.0), Vec3::splat(1.0));
    assert!(!visible, "AABB fully outside top should not be visible");
}

#[test]
fn test_aabb_just_outside_bottom() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let f = FrustumPlanes::from_view_proj(&vp);
    let visible = f.test_aabb(Vec3::new(0.0, -15.0, -50.0), Vec3::splat(1.0));
    assert!(!visible, "AABB fully outside bottom should not be visible");
}

#[test]
fn test_aabb_behind_camera_perspective() {
    // Perspective frustum has well-defined near plane
    let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0), Vec3::Y);
    let proj = Mat4::perspective_rh(1.0, 1.0, 0.1, 100.0);
    let vp = proj * view;
    let f = FrustumPlanes::from_view_proj(&vp);
    // Object behind camera (positive Z) should not be visible
    let visible = f.test_aabb(Vec3::new(0.0, 0.0, 50.0), Vec3::splat(1.0));
    assert!(!visible, "AABB behind camera should not be visible");
}

#[test]
fn test_aabb_beyond_far_plane() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let f = FrustumPlanes::from_view_proj(&vp);
    let visible = f.test_aabb(Vec3::new(0.0, 0.0, -200.0), Vec3::splat(1.0));
    assert!(!visible, "AABB beyond far plane should not be visible");
}

#[test]
fn test_aabb_radius_per_component_x() {
    // AABB is a thin slab extended only in X
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let f = FrustumPlanes::from_view_proj(&vp);
    // extent = (100, 0, 0): wide in X, zero in Y and Z
    // Center at x=9 — visually at margin. With x-extent=100, clearly inside.
    let visible = f.test_aabb(Vec3::new(9.0, 0.0, -50.0), Vec3::new(100.0, 0.0, 0.0));
    assert!(visible, "X extent should contribute to radius");
}

#[test]
fn test_aabb_zero_extent() {
    let vp = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0);
    let f = FrustumPlanes::from_view_proj(&vp);
    // Zero extent at origin should be visible
    let visible = f.test_aabb(Vec3::ZERO, Vec3::ZERO);
    assert!(visible, "Zero-extent AABB at origin should be visible");
}

// ══════════════════════════════════════════════════════════════════════════════
// InstanceAABB::from_transform — corner transformation
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn aabb_from_transform_identity() {
    let aabb = InstanceAABB::from_transform(
        &Mat4::IDENTITY,
        Vec3::new(-1.0, -2.0, -3.0),
        Vec3::new(1.0, 2.0, 3.0),
        7,
    );
    assert!((aabb.center[0]).abs() < 1e-4);
    assert!((aabb.center[1]).abs() < 1e-4);
    assert!((aabb.center[2]).abs() < 1e-4);
    assert!((aabb.extent[0] - 1.0).abs() < 1e-4);
    assert!((aabb.extent[1] - 2.0).abs() < 1e-4);
    assert!((aabb.extent[2] - 3.0).abs() < 1e-4);
    assert_eq!(aabb.instance_index, 7);
}

#[test]
fn aabb_from_transform_scale() {
    let aabb = InstanceAABB::from_transform(
        &Mat4::from_scale(Vec3::splat(2.0)),
        Vec3::splat(-1.0),
        Vec3::splat(1.0),
        0,
    );
    // Scale 2 → extent doubles from 1 to 2
    for i in 0..3 {
        assert!(
            (aabb.extent[i] - 2.0).abs() < 1e-3,
            "extent[{i}]={}",
            aabb.extent[i]
        );
    }
}

#[test]
fn aabb_from_transform_90_rotation_y() {
    // 90° rotation around Y swaps X and Z extents
    let aabb = InstanceAABB::from_transform(
        &Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2),
        Vec3::new(-2.0, -1.0, -3.0),
        Vec3::new(2.0, 1.0, 3.0),
        0,
    );
    // Original extent: (2, 1, 3). After 90° Y rotation: x-extent≈3, z-extent≈2
    assert!(
        (aabb.extent[0] - 3.0).abs() < 0.1,
        "rotated x extent: {}",
        aabb.extent[0]
    );
    assert!(
        (aabb.extent[1] - 1.0).abs() < 0.1,
        "y unchanged: {}",
        aabb.extent[1]
    );
    assert!(
        (aabb.extent[2] - 2.0).abs() < 0.1,
        "rotated z extent: {}",
        aabb.extent[2]
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// cpu_frustum_cull — edge cases
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn cpu_frustum_cull_all_visible() {
    let vp = Mat4::orthographic_rh(-100.0, 100.0, -100.0, 100.0, 0.1, 1000.0);
    let f = FrustumPlanes::from_view_proj(&vp);
    let instances: Vec<InstanceAABB> = (0..10)
        .map(|i| InstanceAABB::new(Vec3::new(0.0, 0.0, -(i as f32) * 10.0), Vec3::ONE, i))
        .collect();
    let visible = cpu_frustum_cull(&instances, &f);
    assert_eq!(visible.len(), 10);
}

#[test]
fn cpu_frustum_cull_none_visible() {
    let vp = Mat4::orthographic_rh(-1.0, 1.0, -1.0, 1.0, 0.1, 2.0);
    let f = FrustumPlanes::from_view_proj(&vp);
    let instances = vec![
        InstanceAABB::new(Vec3::new(100.0, 0.0, 0.0), Vec3::splat(0.1), 0),
        InstanceAABB::new(Vec3::new(-100.0, 0.0, 0.0), Vec3::splat(0.1), 1),
    ];
    let visible = cpu_frustum_cull(&instances, &f);
    assert!(visible.is_empty());
}

// ══════════════════════════════════════════════════════════════════════════════
// DrawBatch / batch_visible_instances — arithmetic
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn draw_batch_instance_count_is_len() {
    let mut batch = DrawBatch::new(BatchId::new(0, 0), 100, 0);
    for i in 0..5 {
        batch.add_instance(i);
    }
    assert_eq!(batch.instance_count(), 5);
    assert_eq!(batch.instances.len(), 5);
}

#[test]
fn build_indirect_commands_gpu_layout() {
    let mut b = DrawBatch::new(BatchId::new(0, 0), 36, 200);
    b.add_instance(0);
    b.add_instance(5);
    let cmds = build_indirect_commands_cpu(&[b]);
    assert_eq!(cmds[0].vertex_count, 36);
    assert_eq!(cmds[0].instance_count, 2);
    assert_eq!(cmds[0].first_vertex, 200);
    assert_eq!(cmds[0].first_instance, 0); // always 0
}

#[test]
fn batch_visible_instances_single_group() {
    let visible = vec![10, 20, 30];
    let batches = batch_visible_instances(&visible, |_| BatchId::new(5, 3), |_| (100, 0));
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].instance_count(), 3);
    assert_eq!(batches[0].instances, vec![10, 20, 30]);
    assert_eq!(batches[0].vertex_count, 100);
}

// ══════════════════════════════════════════════════════════════════════════════
// primitives::cube() — golden value checks
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn cube_vertex_count_and_index_count() {
    let (verts, indices) = primitives::cube();
    assert_eq!(verts.len(), 24); // 6 faces * 4 verts
    assert_eq!(indices.len(), 36); // 6 faces * 2 tris * 3
}

#[test]
fn cube_face_normals_sign_check() {
    let (verts, _) = primitives::cube();
    // Face 0 (+X): normal should be (1, 0, 0)
    assert_eq!(verts[0].normal, [1.0, 0.0, 0.0]);
    // Face 1 (-X): normal should be (-1, 0, 0)
    assert_eq!(verts[4].normal, [-1.0, 0.0, 0.0]);
    // Face 2 (+Y): normal should be (0, 1, 0)
    assert_eq!(verts[8].normal, [0.0, 1.0, 0.0]);
    // Face 3 (-Y): normal should be (0, -1, 0)
    assert_eq!(verts[12].normal, [0.0, -1.0, 0.0]);
    // Face 4 (+Z): normal should be (0, 0, 1)
    assert_eq!(verts[16].normal, [0.0, 0.0, 1.0]);
    // Face 5 (-Z): normal should be (0, 0, -1)
    assert_eq!(verts[20].normal, [0.0, 0.0, -1.0]);
}

#[test]
fn cube_all_positions_in_unit_range() {
    let (verts, _) = primitives::cube();
    for v in &verts {
        for i in 0..3 {
            assert!(
                v.position[i].abs() <= 1.0 + 1e-6,
                "position[{i}]={} exceeds ±1",
                v.position[i]
            );
        }
    }
}

#[test]
fn cube_index_winding_consistent() {
    let (verts, indices) = primitives::cube();
    // Each face's 2 triangles should reference only vertices from that face
    for face in 0..6 {
        let base = face * 4;
        let idx_offset = face * 6;
        for i in 0..6 {
            let idx = indices[idx_offset + i] as usize;
            assert!(
                idx >= base && idx < base + 4,
                "face {face}: index {idx} outside face range [{base}, {})",
                base + 4
            );
        }
    }
}

#[test]
fn cube_tangents_all_set() {
    let (verts, _) = primitives::cube();
    for (i, v) in verts.iter().enumerate() {
        assert_eq!(v.tangent, [1.0, 0.0, 0.0, 1.0], "vertex {i} tangent wrong");
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// primitives::plane() — golden value checks
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn plane_all_y_zero() {
    let (verts, _) = primitives::plane();
    for v in &verts {
        assert_eq!(v.position[1], 0.0);
    }
}

#[test]
fn plane_normal_is_up() {
    let (verts, _) = primitives::plane();
    for v in &verts {
        assert_eq!(v.normal, [0.0, 1.0, 0.0]);
    }
}

#[test]
fn plane_indices_golden() {
    let (_, indices) = primitives::plane();
    assert_eq!(indices, vec![0, 1, 2, 0, 2, 3]);
}

#[test]
fn plane_uv_corners() {
    let (verts, _) = primitives::plane();
    let uvs: Vec<[f32; 2]> = verts.iter().map(|v| v.uv).collect();
    assert!(uvs.contains(&[0.0, 0.0]));
    assert!(uvs.contains(&[1.0, 0.0]));
    assert!(uvs.contains(&[1.0, 1.0]));
    assert!(uvs.contains(&[0.0, 1.0]));
}

// ══════════════════════════════════════════════════════════════════════════════
// primitives::sphere() — trig and vertex golden values
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn sphere_vertex_count_formula() {
    let (verts, _) = primitives::sphere(8, 8, 1.0);
    assert_eq!(verts.len(), (8 + 1) * (8 + 1)); // (stacks+1)*(slices+1) = 81
}

#[test]
fn sphere_index_count_formula() {
    let (_, indices) = primitives::sphere(8, 8, 1.0);
    assert_eq!(indices.len(), (8 * 8 * 6) as usize); // 384
}

#[test]
fn sphere_poles_at_correct_position() {
    let (verts, _) = primitives::sphere(8, 8, 2.0);
    // Top pole: i=0 (phi=0) → y=radius, x=z=0
    assert!(
        (verts[0].position[1] - 2.0).abs() < 1e-4,
        "top pole y={}",
        verts[0].position[1]
    );
    assert!(
        verts[0].position[0].abs() < 1e-4,
        "top pole x={}",
        verts[0].position[0]
    );
    assert!(
        verts[0].position[2].abs() < 1e-4,
        "top pole z={}",
        verts[0].position[2]
    );

    // Bottom pole: i=stacks (last stack, j=0)
    let bottom = &verts[8 * (8 + 1)]; // i=8, j=0
    assert!(
        (bottom.position[1] + 2.0).abs() < 1e-4,
        "bottom pole y={}",
        bottom.position[1]
    );
}

#[test]
fn sphere_all_normals_unit_length() {
    let (verts, _) = primitives::sphere(10, 10, 3.0);
    for (i, v) in verts.iter().enumerate() {
        let len =
            (v.normal[0] * v.normal[0] + v.normal[1] * v.normal[1] + v.normal[2] * v.normal[2])
                .sqrt();
        assert!((len - 1.0).abs() < 1e-4, "vertex {i}: normal length={len}");
    }
}

#[test]
fn sphere_radius_scales_positions() {
    let (v1, _) = primitives::sphere(4, 4, 1.0);
    let (v2, _) = primitives::sphere(4, 4, 5.0);
    // Same vertex index, radius=5 should be 5× radius=1
    for i in 0..v1.len() {
        for j in 0..3 {
            let expected = v1[i].position[j] * 5.0;
            assert!(
                (v2[i].position[j] - expected).abs() < 1e-3,
                "vert {i} pos[{j}]: {} vs expected {}",
                v2[i].position[j],
                expected
            );
        }
    }
}

#[test]
fn sphere_clamped_below_3() {
    let (v1, i1) = primitives::sphere(1, 1, 1.0);
    let (v2, i2) = primitives::sphere(3, 3, 1.0);
    assert_eq!(v1.len(), v2.len());
    assert_eq!(i1.len(), i2.len());
}

#[test]
fn sphere_uvs_in_range() {
    let (verts, _) = primitives::sphere(8, 8, 1.0);
    for v in &verts {
        assert!(v.uv[0] >= 0.0 && v.uv[0] <= 1.0, "u={}", v.uv[0]);
        assert!(v.uv[1] >= 0.0 && v.uv[1] <= 1.0, "v={}", v.uv[1]);
    }
}

#[test]
fn sphere_equator_x_vertex() {
    // At equator (i=stacks/2), j=0 (theta=0): position should be (radius, 0, 0)
    let (verts, _) = primitives::sphere(8, 8, 1.0);
    // i=4 (equator), j=0: phi = PI/2, theta = 0
    // nx = sin(PI/2)*cos(0) = 1, ny = cos(PI/2) = 0, nz = sin(PI/2)*sin(0) = 0
    let equator_idx = 4 * (8 + 1); // i=4, j=0
    let v = &verts[equator_idx];
    assert!(
        (v.position[0] - 1.0).abs() < 1e-4,
        "equator x: {}",
        v.position[0]
    );
    assert!(v.position[1].abs() < 1e-4, "equator y: {}", v.position[1]);
    assert!(v.position[2].abs() < 1e-4, "equator z: {}", v.position[2]);
}

#[test]
fn sphere_no_degenerate_triangles() {
    let (_, indices) = primitives::sphere(6, 6, 1.0);
    for tri in indices.chunks(3) {
        assert_ne!(tri[0], tri[1], "degenerate: {:?}", tri);
        // Note: pole triangles may have two indices sharing a position (but different indices)
        // so we only check index equality, not position equality
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Material — basic sanity
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn material_clone_preserves_color() {
    let m = Material {
        color: [0.1, 0.2, 0.3, 0.4],
    };
    let m2 = m.clone();
    assert_eq!(m.color, m2.color);
}

// ══════════════════════════════════════════════════════════════════════════════
// Struct sizes (bytemuck Pod alignment)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn vertex_size_48_bytes() {
    assert_eq!(std::mem::size_of::<Vertex>(), 48);
}

#[test]
fn instance_aabb_size_32_bytes() {
    assert_eq!(std::mem::size_of::<InstanceAABB>(), 32);
}

#[test]
fn frustum_planes_size_96_bytes() {
    assert_eq!(std::mem::size_of::<FrustumPlanes>(), 96);
}

#[test]
fn draw_indirect_command_size_16_bytes() {
    assert_eq!(std::mem::size_of::<DrawIndirectCommand>(), 16);
}
