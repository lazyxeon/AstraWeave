//! Mutation-resistant comprehensive tests for astraweave-math.
//! Targets exact return values, boundary conditions, operator swaps,
//! and mathematical identities for 90%+ mutation kill rate.

use astraweave_math::simd_movement::{update_positions_naive, update_positions_simd};
use astraweave_math::*;
use glam::{Mat4, Quat, Vec3};

// ========================================================================
// SIMD VEC: DOT PRODUCT
// ========================================================================

#[test]
fn dot_parallel_unit_vectors() {
    assert!((dot_simd(Vec3::X, Vec3::X) - 1.0).abs() < 1e-6);
    assert!((dot_simd(Vec3::Y, Vec3::Y) - 1.0).abs() < 1e-6);
    assert!((dot_simd(Vec3::Z, Vec3::Z) - 1.0).abs() < 1e-6);
}

#[test]
fn dot_orthogonal_is_zero() {
    assert!(dot_simd(Vec3::X, Vec3::Y).abs() < 1e-6);
    assert!(dot_simd(Vec3::Y, Vec3::Z).abs() < 1e-6);
    assert!(dot_simd(Vec3::X, Vec3::Z).abs() < 1e-6);
}

#[test]
fn dot_anti_parallel_is_negative_one() {
    assert!((dot_simd(Vec3::X, Vec3::NEG_X) - (-1.0)).abs() < 1e-6);
}

#[test]
fn dot_commutative() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, -5.0, 6.0);
    assert!((dot_simd(a, b) - dot_simd(b, a)).abs() < 1e-6);
}

#[test]
fn dot_known_value() {
    // (1,2,3)·(4,-5,6) = 4 + (-10) + 18 = 12
    let r = dot_simd(Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, -5.0, 6.0));
    assert!((r - 12.0).abs() < 1e-6);
}

#[test]
fn dot_zero_vector() {
    assert!(dot_simd(Vec3::ZERO, Vec3::new(1.0, 2.0, 3.0)).abs() < 1e-6);
}

// ========================================================================
// SIMD VEC: CROSS PRODUCT
// ========================================================================

#[test]
fn cross_basis_vectors() {
    let xy = cross_simd(Vec3::X, Vec3::Y);
    assert!((xy - Vec3::Z).length() < 1e-6, "X × Y = Z");
    let yz = cross_simd(Vec3::Y, Vec3::Z);
    assert!((yz - Vec3::X).length() < 1e-6, "Y × Z = X");
    let zx = cross_simd(Vec3::Z, Vec3::X);
    assert!((zx - Vec3::Y).length() < 1e-6, "Z × X = Y");
}

#[test]
fn cross_anticommutative() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, -5.0, 6.0);
    let ab = cross_simd(a, b);
    let ba = cross_simd(b, a);
    assert!((ab + ba).length() < 1e-5, "a×b = -(b×a)");
}

#[test]
fn cross_self_is_zero() {
    let v = Vec3::new(3.0, 7.0, -2.0);
    assert!(cross_simd(v, v).length() < 1e-6);
}

#[test]
fn cross_orthogonal_to_inputs() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, -5.0, 6.0);
    let c = cross_simd(a, b);
    assert!(dot_simd(c, a).abs() < 1e-4, "cross perpendicular to a");
    assert!(dot_simd(c, b).abs() < 1e-4, "cross perpendicular to b");
}

// ========================================================================
// SIMD VEC: NORMALIZE
// ========================================================================

#[test]
fn normalize_unit_vector_unchanged() {
    let n = normalize_simd(Vec3::X);
    assert!((n - Vec3::X).length() < 1e-6);
}

#[test]
fn normalize_produces_unit_length() {
    let v = Vec3::new(3.0, 4.0, 0.0);
    let n = normalize_simd(v);
    assert!((length_simd(n) - 1.0).abs() < 1e-6);
}

#[test]
fn normalize_preserves_direction() {
    let v = Vec3::new(3.0, 4.0, 0.0);
    let n = normalize_simd(v);
    assert!((n.x / n.y - 3.0 / 4.0).abs() < 1e-5, "ratio preserved");
}

#[test]
fn normalize_zero_returns_zero() {
    let n = normalize_simd(Vec3::ZERO);
    assert!(n.length() < 1e-8, "zero vector normalizes to zero");
}

#[test]
fn normalize_near_zero_boundary() {
    // threshold is len_sq < 1e-16 (i.e. length < 1e-8)
    let tiny = Vec3::new(1e-9, 0.0, 0.0); // length = 1e-9, len_sq = 1e-18 < 1e-16
    let n = normalize_simd(tiny);
    assert!(n.length() < 1e-6, "below threshold returns near-zero");
}

#[test]
fn normalize_above_threshold() {
    // length = 1e-7, len_sq = 1e-14 > 1e-16 threshold
    let v = Vec3::new(1e-7, 0.0, 0.0);
    let n = normalize_simd(v);
    assert!(
        (length_simd(n) - 1.0).abs() < 1e-3,
        "above threshold normalizes"
    );
}

#[test]
fn normalize_negative_components() {
    let v = Vec3::new(-3.0, -4.0, 0.0);
    let n = normalize_simd(v);
    assert!(n.x < 0.0, "preserves negative x sign");
    assert!(n.y < 0.0, "preserves negative y sign");
    assert!((length_simd(n) - 1.0).abs() < 1e-6);
}

// ========================================================================
// SIMD VEC: LENGTH
// ========================================================================

#[test]
fn length_zero() {
    assert!(length_simd(Vec3::ZERO).abs() < 1e-8);
}

#[test]
fn length_unit_axes() {
    assert!((length_simd(Vec3::X) - 1.0).abs() < 1e-6);
    assert!((length_simd(Vec3::Y) - 1.0).abs() < 1e-6);
    assert!((length_simd(Vec3::Z) - 1.0).abs() < 1e-6);
}

#[test]
fn length_3_4_5_triangle() {
    assert!((length_simd(Vec3::new(3.0, 4.0, 0.0)) - 5.0).abs() < 1e-6);
}

#[test]
fn length_squared_no_sqrt() {
    let v = Vec3::new(3.0, 4.0, 0.0);
    assert!((length_squared_simd(v) - 25.0).abs() < 1e-6);
}

#[test]
fn length_always_non_negative() {
    let v = Vec3::new(-5.0, -3.0, -1.0);
    assert!(length_simd(v) >= 0.0);
}

#[test]
fn length_squared_is_dot_self() {
    let v = Vec3::new(2.0, 3.0, 4.0);
    assert!((length_squared_simd(v) - dot_simd(v, v)).abs() < 1e-6);
}

// ========================================================================
// SIMD MAT: MULTIPLY
// ========================================================================

#[test]
fn mat_mul_identity_left() {
    let m = Mat4::from_scale(Vec3::new(2.0, 3.0, 4.0));
    let r = mul_simd(Mat4::IDENTITY, m);
    assert!((r - m).abs_diff_eq(Mat4::ZERO, 1e-6));
}

#[test]
fn mat_mul_identity_right() {
    let m = Mat4::from_scale(Vec3::new(2.0, 3.0, 4.0));
    let r = mul_simd(m, Mat4::IDENTITY);
    assert!((r - m).abs_diff_eq(Mat4::ZERO, 1e-6));
}

#[test]
fn mat_mul_scale_composes() {
    let a = Mat4::from_scale(Vec3::splat(2.0));
    let b = Mat4::from_scale(Vec3::splat(3.0));
    let r = mul_simd(a, b);
    let expected = Mat4::from_scale(Vec3::splat(6.0));
    assert!((r - expected).abs_diff_eq(Mat4::ZERO, 1e-5));
}

#[test]
fn mat_mul_not_commutative() {
    let a = Mat4::from_translation(Vec3::X);
    let b = Mat4::from_scale(Vec3::splat(2.0));
    let ab = mul_simd(a, b);
    let ba = mul_simd(b, a);
    // Translation * Scale ≠ Scale * Translation in general
    assert!(
        !(ab - ba).abs_diff_eq(Mat4::ZERO, 1e-3),
        "mul is not commutative"
    );
}

// ========================================================================
// SIMD MAT: TRANSPOSE
// ========================================================================

#[test]
fn transpose_identity_unchanged() {
    let t = transpose_simd(Mat4::IDENTITY);
    assert!((t - Mat4::IDENTITY).abs_diff_eq(Mat4::ZERO, 1e-6));
}

#[test]
fn transpose_involution() {
    let m = Mat4::from_cols(
        glam::Vec4::new(1.0, 2.0, 3.0, 4.0),
        glam::Vec4::new(5.0, 6.0, 7.0, 8.0),
        glam::Vec4::new(9.0, 10.0, 11.0, 12.0),
        glam::Vec4::new(13.0, 14.0, 15.0, 16.0),
    );
    let tt = transpose_simd(transpose_simd(m));
    assert!(
        (tt - m).abs_diff_eq(Mat4::ZERO, 1e-6),
        "transpose(transpose(M)) = M"
    );
}

// ========================================================================
// SIMD MAT: INVERSE
// ========================================================================

#[test]
fn inverse_identity() {
    let inv = inverse_simd(Mat4::IDENTITY);
    assert!((inv - Mat4::IDENTITY).abs_diff_eq(Mat4::ZERO, 1e-6));
}

#[test]
fn inverse_of_scale() {
    let s = Mat4::from_scale(Vec3::splat(2.0));
    let inv = inverse_simd(s);
    let expected = Mat4::from_scale(Vec3::splat(0.5));
    assert!((inv - expected).abs_diff_eq(Mat4::ZERO, 1e-5));
}

#[test]
fn mul_inverse_is_identity() {
    let m = Mat4::from_scale(Vec3::new(2.0, 3.0, 4.0));
    let inv = inverse_simd(m);
    let r = mul_simd(m, inv);
    assert!((r - Mat4::IDENTITY).abs_diff_eq(Mat4::ZERO, 1e-4));
}

// ========================================================================
// SIMD MAT: TRANSFORM POINT
// ========================================================================

#[test]
fn transform_point_identity() {
    let p = Vec3::new(1.0, 2.0, 3.0);
    let r = transform_point_simd(Mat4::IDENTITY, p);
    assert!((r - p).length() < 1e-6);
}

#[test]
fn transform_point_translation() {
    let m = Mat4::from_translation(Vec3::new(10.0, 20.0, 30.0));
    let r = transform_point_simd(m, Vec3::ZERO);
    assert!((r - Vec3::new(10.0, 20.0, 30.0)).length() < 1e-5);
}

#[test]
fn transform_point_scale() {
    let m = Mat4::from_scale(Vec3::splat(3.0));
    let r = transform_point_simd(m, Vec3::new(1.0, 2.0, 3.0));
    assert!((r - Vec3::new(3.0, 6.0, 9.0)).length() < 1e-5);
}

#[test]
fn transform_points_batch_preserves_count() {
    let m = Mat4::IDENTITY;
    let pts = vec![Vec3::X, Vec3::Y, Vec3::Z];
    let result = transform_points_batch(m, &pts);
    assert_eq!(result.len(), 3);
}

#[test]
fn transform_points_batch_empty() {
    let result = transform_points_batch(Mat4::IDENTITY, &[]);
    assert!(result.is_empty());
}

#[test]
fn transform_points_batch_correctness() {
    let m = Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0));
    let pts = vec![Vec3::ZERO, Vec3::Y];
    let result = transform_points_batch(m, &pts);
    assert!((result[0] - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-5);
    assert!((result[1] - Vec3::new(1.0, 1.0, 0.0)).length() < 1e-5);
}

// ========================================================================
// SIMD QUAT: MULTIPLY
// ========================================================================

#[test]
fn quat_mul_identity_left() {
    let q = Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_4);
    let r = mul_quat_simd(Quat::IDENTITY, q);
    assert!((r.x - q.x).abs() < 1e-5);
    assert!((r.y - q.y).abs() < 1e-5);
    assert!((r.z - q.z).abs() < 1e-5);
    assert!((r.w - q.w).abs() < 1e-5);
}

#[test]
fn quat_mul_identity_right() {
    let q = Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_4);
    let r = mul_quat_simd(q, Quat::IDENTITY);
    assert!((r.x - q.x).abs() < 1e-5);
    assert!((r.w - q.w).abs() < 1e-5);
}

#[test]
fn quat_mul_not_commutative() {
    let a = Quat::from_axis_angle(Vec3::X, std::f32::consts::FRAC_PI_4);
    let b = Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_4);
    let ab = mul_quat_simd(a, b);
    let ba = mul_quat_simd(b, a);
    let diff =
        (ab.x - ba.x).abs() + (ab.y - ba.y).abs() + (ab.z - ba.z).abs() + (ab.w - ba.w).abs();
    assert!(diff > 1e-3, "quaternion multiply is not commutative");
}

// ========================================================================
// SIMD QUAT: NORMALIZE
// ========================================================================

#[test]
fn quat_normalize_produces_unit() {
    let q = Quat::from_xyzw(1.0, 2.0, 3.0, 4.0);
    let n = normalize_quat_simd(q);
    let len = (n.x * n.x + n.y * n.y + n.z * n.z + n.w * n.w).sqrt();
    assert!((len - 1.0).abs() < 1e-5);
}

#[test]
fn quat_normalize_already_unit() {
    let q = Quat::IDENTITY;
    let n = normalize_quat_simd(q);
    assert!((n.w - 1.0).abs() < 1e-6);
    assert!(n.x.abs() < 1e-6);
}

#[test]
fn quat_normalize_batch_count() {
    let quats = vec![Quat::IDENTITY; 10];
    let result = normalize_batch(&quats);
    assert_eq!(result.len(), 10);
}

#[test]
fn quat_normalize_batch_all_unit() {
    let quats = vec![
        Quat::from_xyzw(1.0, 0.0, 0.0, 0.0),
        Quat::from_xyzw(0.0, 2.0, 0.0, 0.0),
        Quat::from_xyzw(0.0, 0.0, 3.0, 4.0),
    ];
    for n in normalize_batch(&quats) {
        let len = (n.x * n.x + n.y * n.y + n.z * n.z + n.w * n.w).sqrt();
        assert!((len - 1.0).abs() < 1e-5);
    }
}

// ========================================================================
// SIMD QUAT: DOT
// ========================================================================

#[test]
fn quat_dot_self_is_length_sq() {
    let q = Quat::from_xyzw(1.0, 2.0, 3.0, 4.0);
    let expected = 1.0 + 4.0 + 9.0 + 16.0; // 30.0
    assert!((dot_quat_simd(q, q) - expected).abs() < 1e-4);
}

#[test]
fn quat_dot_identity_self() {
    assert!((dot_quat_simd(Quat::IDENTITY, Quat::IDENTITY) - 1.0).abs() < 1e-6);
}

// ========================================================================
// SIMD QUAT: SLERP
// ========================================================================

#[test]
fn slerp_t0_returns_first() {
    let a = Quat::IDENTITY;
    let b = Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI);
    let r = slerp_simd(a, b, 0.0);
    assert!((r.w - a.w).abs() < 1e-5);
}

#[test]
fn slerp_t1_returns_second() {
    let a = Quat::IDENTITY;
    let b = Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2);
    let r = slerp_simd(a, b, 1.0);
    assert!((r.y - b.y).abs() < 1e-5);
    assert!((r.w - b.w).abs() < 1e-5);
}

#[test]
fn slerp_halfway_interpolation() {
    let a = Quat::IDENTITY;
    let b = Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2);
    let mid = slerp_simd(a, b, 0.5);
    let len = (mid.x * mid.x + mid.y * mid.y + mid.z * mid.z + mid.w * mid.w).sqrt();
    assert!((len - 1.0).abs() < 1e-5, "slerp produces unit quaternion");
}

#[test]
fn slerp_batch_count() {
    let pairs = vec![(Quat::IDENTITY, Quat::IDENTITY); 5];
    let result = slerp_batch(&pairs, 0.5);
    assert_eq!(result.len(), 5);
}

#[test]
fn slerp_batch_empty() {
    let result = slerp_batch(&[], 0.5);
    assert!(result.is_empty());
}

// ========================================================================
// SIMD MOVEMENT: UPDATE POSITIONS
// ========================================================================

#[test]
fn update_positions_basic() {
    let mut pos = vec![Vec3::ZERO];
    let vel = vec![Vec3::new(1.0, 0.0, 0.0)];
    update_positions_simd(&mut pos, &vel, 1.0);
    assert!((pos[0] - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-6);
}

#[test]
fn update_positions_dt_scaling() {
    let mut pos = vec![Vec3::ZERO];
    let vel = vec![Vec3::new(10.0, 0.0, 0.0)];
    update_positions_simd(&mut pos, &vel, 0.5);
    assert!((pos[0].x - 5.0).abs() < 1e-6, "pos += vel * dt");
}

#[test]
fn update_positions_zero_dt() {
    let mut pos = vec![Vec3::new(1.0, 2.0, 3.0)];
    let vel = vec![Vec3::new(100.0, 200.0, 300.0)];
    update_positions_simd(&mut pos, &vel, 0.0);
    assert!(
        (pos[0] - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-6,
        "zero dt no movement"
    );
}

#[test]
fn update_positions_negative_velocity() {
    let mut pos = vec![Vec3::new(10.0, 0.0, 0.0)];
    let vel = vec![Vec3::new(-5.0, 0.0, 0.0)];
    update_positions_simd(&mut pos, &vel, 1.0);
    assert!((pos[0].x - 5.0).abs() < 1e-6);
}

#[test]
fn update_positions_batch_exact_4() {
    // Exactly BATCH_SIZE=4, no remainder
    let mut pos = vec![Vec3::ZERO; 4];
    let vel = vec![Vec3::X, Vec3::Y, Vec3::Z, Vec3::new(1.0, 1.0, 1.0)];
    update_positions_simd(&mut pos, &vel, 2.0);
    assert!((pos[0] - Vec3::new(2.0, 0.0, 0.0)).length() < 1e-6);
    assert!((pos[1] - Vec3::new(0.0, 2.0, 0.0)).length() < 1e-6);
    assert!((pos[2] - Vec3::new(0.0, 0.0, 2.0)).length() < 1e-6);
    assert!((pos[3] - Vec3::new(2.0, 2.0, 2.0)).length() < 1e-6);
}

#[test]
fn update_positions_batch_plus_remainder() {
    // 5 entities: 1 batch of 4 + 1 remainder
    let mut pos = vec![Vec3::ZERO; 5];
    let vel = vec![Vec3::X; 5];
    update_positions_simd(&mut pos, &vel, 1.0);
    for p in &pos {
        assert!(
            (p.x - 1.0).abs() < 1e-6,
            "all 5 updated including remainder"
        );
    }
}

#[test]
fn update_positions_remainder_only() {
    // 3 entities: 0 full batches + 3 remainder
    let mut pos = vec![Vec3::ZERO; 3];
    let vel = vec![Vec3::Y; 3];
    update_positions_simd(&mut pos, &vel, 1.0);
    for p in &pos {
        assert!((p.y - 1.0).abs() < 1e-6);
    }
}

#[test]
fn update_positions_empty() {
    let mut pos: Vec<Vec3> = vec![];
    let vel: Vec<Vec3> = vec![];
    update_positions_simd(&mut pos, &vel, 1.0); // should not panic
}

#[test]
#[should_panic]
fn update_positions_mismatched_panics() {
    let mut pos = vec![Vec3::ZERO; 3];
    let vel = vec![Vec3::X; 5]; // mismatch!
    update_positions_simd(&mut pos, &vel, 1.0);
}

#[test]
fn update_positions_naive_matches_simd() {
    let mut pos_simd = vec![Vec3::new(1.0, 2.0, 3.0); 7];
    let mut pos_naive = pos_simd.clone();
    let vel = vec![Vec3::new(0.5, -0.3, 1.2); 7];
    update_positions_simd(&mut pos_simd, &vel, 0.016);
    update_positions_naive(&mut pos_naive, &vel, 0.016);
    for (s, n) in pos_simd.iter().zip(pos_naive.iter()) {
        assert!((s.x - n.x).abs() < 1e-5);
        assert!((s.y - n.y).abs() < 1e-5);
        assert!((s.z - n.z).abs() < 1e-5);
    }
}

#[test]
fn update_positions_large_batch() {
    let n = 100;
    let mut pos = vec![Vec3::ZERO; n];
    let vel: Vec<Vec3> = (0..n).map(|i| Vec3::new(i as f32, 0.0, 0.0)).collect();
    update_positions_simd(&mut pos, &vel, 1.0);
    for (i, p) in pos.iter().enumerate().take(n) {
        assert!((p.x - i as f32).abs() < 1e-4, "entity {} correct", i);
    }
}

// ========================================================================
// MATHEMATICAL IDENTITIES
// ========================================================================

#[test]
fn pythagorean_identity_dot() {
    // |a|² + |b|² >= 2*|a·b| (Cauchy-Schwarz related)
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, 5.0, 6.0);
    let lhs = length_squared_simd(a) * length_squared_simd(b);
    let rhs = dot_simd(a, b) * dot_simd(a, b);
    assert!(lhs >= rhs - 1e-3, "Cauchy-Schwarz: |a|²|b|² >= (a·b)²");
}

#[test]
fn cross_product_magnitude() {
    // |a × b|² + (a·b)² = |a|²|b|² (Lagrange identity)
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, 5.0, 6.0);
    let cross_mag_sq = length_squared_simd(cross_simd(a, b));
    let dot_sq = dot_simd(a, b) * dot_simd(a, b);
    let product_sq = length_squared_simd(a) * length_squared_simd(b);
    assert!(
        (cross_mag_sq + dot_sq - product_sq).abs() < 1e-2,
        "Lagrange identity"
    );
}

#[test]
fn normalize_idempotent() {
    let v = Vec3::new(3.0, 4.0, 12.0);
    let n1 = normalize_simd(v);
    let n2 = normalize_simd(n1);
    assert!((n1 - n2).length() < 1e-6, "normalizing twice = once");
}

#[test]
fn length_of_normalized_is_one() {
    let v = Vec3::new(7.0, -3.0, 11.0);
    let n = normalize_simd(v);
    assert!((length_simd(n) - 1.0).abs() < 1e-5);
}
