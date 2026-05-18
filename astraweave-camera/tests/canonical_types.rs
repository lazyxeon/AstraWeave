//! Contract tests for astraweave-camera canonical types.
//!
//! These tests assert that [`Projection`] and [`RenderView`] comply with the
//! conventions documented in `docs/current/CAMERA_CONVENTIONS.md`. Failure
//! means a convention violation in the type — either the code drifted, or
//! the convention itself needs revision (an Andrew design call, not an
//! autonomous test relaxation).
//!
//! Producer-specific tests (FreeFly's +X forward direction at yaw=0,
//! OrbitCamera's orbit-offset semantic, etc.) are out of C.2 scope — they
//! land in C.3/C.4 alongside the producer migrations.
//!
//! Run with:
//!   cargo test --tests -p astraweave-camera

use astraweave_camera::{Projection, RenderView};
use glam::{Mat4, Vec3, Vec4};

// ─────────────────────────────────────────────────────────────────────────
// Projection contract tests
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn projection_stores_radians_per_section_2_1() {
    // CAMERA_CONVENTIONS.md §2.1: fovy stores radians.
    let p = Projection::perspective(60_f32.to_radians(), 16.0 / 9.0, 0.1, 100.0);
    assert!(
        (p.fovy - 60_f32.to_radians()).abs() < 1e-6,
        "Projection::fovy must store radians per §2.1; got {}",
        p.fovy
    );
}

#[test]
fn projection_uses_perspective_rh_per_section_2_6() {
    // CAMERA_CONVENTIONS.md §2.6: Mat4::perspective_rh only.
    let p = Projection::perspective(60_f32.to_radians(), 1.0, 1.0, 100.0);
    let expected = Mat4::perspective_rh(60_f32.to_radians(), 1.0_f32.max(0.01), 1.0, 100.0);
    assert_eq!(
        p.matrix, expected,
        "Projection::matrix must use Mat4::perspective_rh per §2.6"
    );
}

#[test]
fn projection_depth_range_is_zero_to_one_per_section_2_2() {
    // CAMERA_CONVENTIONS.md §2.2: wgpu [0, 1] depth range.
    let p = Projection::perspective(60_f32.to_radians(), 1.0, 1.0, 100.0);

    // View-space z = -near should map to NDC z = 0.
    let near_view = Vec4::new(0.0, 0.0, -p.znear, 1.0);
    let near_clip = p.matrix * near_view;
    let near_ndc_z = near_clip.z / near_clip.w;
    assert!(
        (near_ndc_z - 0.0).abs() < 1e-4,
        "Near plane must map to NDC z=0 per §2.2; got {}",
        near_ndc_z
    );

    // View-space z = -far should map to NDC z = 1.
    let far_view = Vec4::new(0.0, 0.0, -p.zfar, 1.0);
    let far_clip = p.matrix * far_view;
    let far_ndc_z = far_clip.z / far_clip.w;
    assert!(
        (far_ndc_z - 1.0).abs() < 1e-4,
        "Far plane must map to NDC z=1 per §2.2; got {}",
        far_ndc_z
    );
}

#[test]
fn projection_aspect_is_floored_per_section_2_3() {
    // CAMERA_CONVENTIONS.md §2.3: aspect clamped via .max(0.01) at matrix construction.
    let p = Projection::perspective(60_f32.to_radians(), 0.0, 0.1, 100.0);
    assert!(
        !p.matrix.is_nan(),
        "Projection::matrix must not be NaN when aspect=0.0 per §2.3"
    );

    let p_nan = Projection::perspective(60_f32.to_radians(), f32::NAN, 0.1, 100.0);
    assert!(
        !p_nan.matrix.is_nan(),
        "Projection::matrix must not be NaN when aspect=NaN per §2.3 (.max(0.01) clamp; NaN.max(0.01) = 0.01)"
    );
}

#[test]
fn projection_preserves_aspect_pre_floor() {
    // The stored aspect field preserves the input value (pre-floor); only
    // matrix construction floors it. This lets callers debug-inspect what
    // was passed.
    let p = Projection::perspective(60_f32.to_radians(), 0.0, 0.1, 100.0);
    assert_eq!(
        p.aspect, 0.0,
        "Projection::aspect stores the pre-floor input value; only matrix construction floors it"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// RenderView contract tests
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn render_view_precomputes_view_proj() {
    // RenderView carries view_proj = projection.matrix * view, precomputed.
    let projection = Projection::perspective(60_f32.to_radians(), 1.0, 0.1, 100.0);
    let view = Mat4::look_to_rh(Vec3::ZERO, Vec3::X, Vec3::Y);
    let rv = RenderView::new(view, &projection, Vec3::ZERO, Vec3::X);

    let expected_view_proj = projection.matrix * view;
    assert_eq!(
        rv.view_proj, expected_view_proj,
        "RenderView::view_proj must equal projection.matrix * view"
    );
}

#[test]
fn render_view_precomputes_inverses() {
    let projection = Projection::perspective(60_f32.to_radians(), 1.0, 0.1, 100.0);
    let view = Mat4::look_to_rh(Vec3::new(1.0, 2.0, 3.0), Vec3::X, Vec3::Y);
    let rv = RenderView::new(view, &projection, Vec3::new(1.0, 2.0, 3.0), Vec3::X);

    // inverse_view * view should be identity (within float epsilon).
    let identity_via_view = rv.inverse_view * rv.view;
    for col in 0..4 {
        for row in 0..4 {
            let expected = if col == row { 1.0 } else { 0.0 };
            assert!(
                (identity_via_view.col(col)[row] - expected).abs() < 1e-4,
                "inverse_view * view should be identity; element [{}][{}] = {}",
                col,
                row,
                identity_via_view.col(col)[row]
            );
        }
    }

    // inverse_view_proj * view_proj should be identity.
    let identity_via_vp = rv.inverse_view_proj * rv.view_proj;
    for col in 0..4 {
        for row in 0..4 {
            let expected = if col == row { 1.0 } else { 0.0 };
            assert!(
                (identity_via_vp.col(col)[row] - expected).abs() < 1e-3,
                "inverse_view_proj * view_proj should be identity; element [{}][{}] = {}",
                col,
                row,
                identity_via_vp.col(col)[row]
            );
        }
    }
}

#[test]
fn render_view_mirrors_projection_parameters() {
    let projection = Projection::perspective(45_f32.to_radians(), 1.5, 0.5, 200.0);
    let view = Mat4::IDENTITY;
    let rv = RenderView::new(view, &projection, Vec3::ZERO, Vec3::X);

    assert_eq!(rv.fovy, projection.fovy);
    assert_eq!(rv.aspect, projection.aspect);
    assert_eq!(rv.znear, projection.znear);
    assert_eq!(rv.zfar, projection.zfar);
    assert_eq!(rv.projection, projection.matrix);
}

#[test]
fn render_view_position_extractable_from_inverse_view() {
    // Per RenderView doc comment: position == inverse_view.col(3).xyz.
    let projection = Projection::perspective(60_f32.to_radians(), 1.0, 0.1, 100.0);
    let position = Vec3::new(5.0, 10.0, -3.0);
    let view = Mat4::look_to_rh(position, Vec3::X, Vec3::Y);
    let rv = RenderView::new(view, &projection, position, Vec3::X);

    let extracted = rv.inverse_view.col(3).truncate();
    assert!(
        (extracted - rv.position).length() < 1e-4,
        "RenderView::position should match inverse_view.col(3).xyz; got position={:?}, extracted={:?}",
        rv.position,
        extracted
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Cross-type compliance — Projection feeds RenderView correctly
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn render_view_built_from_canonical_projection_satisfies_depth_range() {
    // RenderView built from canonical Projection inherits §2.2 depth range.
    // Proves the cross-type composition preserves convention guarantees.
    let projection = Projection::perspective(60_f32.to_radians(), 1.0, 1.0, 100.0);
    let view = Mat4::IDENTITY;
    let rv = RenderView::new(view, &projection, Vec3::ZERO, Vec3::X);

    let near_view = Vec4::new(0.0, 0.0, -projection.znear, 1.0);
    let near_clip = rv.projection * near_view;
    let near_ndc_z = near_clip.z / near_clip.w;
    assert!(
        (near_ndc_z - 0.0).abs() < 1e-4,
        "RenderView built from canonical Projection must inherit §2.2 depth (near→NDC z=0); got {}",
        near_ndc_z
    );

    let far_view = Vec4::new(0.0, 0.0, -projection.zfar, 1.0);
    let far_clip = rv.projection * far_view;
    let far_ndc_z = far_clip.z / far_clip.w;
    assert!(
        (far_ndc_z - 1.0).abs() < 1e-4,
        "RenderView built from canonical Projection must inherit §2.2 depth (far→NDC z=1); got {}",
        far_ndc_z
    );
}
