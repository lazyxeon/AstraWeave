//! Camera convention contract tests — C.1 (Unified Camera campaign).
//!
//! These tests assert that the production runtime camera (post-C.3.C:
//! `astraweave_camera::FreeFly`, formerly the shim-aliased
//! `astraweave_render::camera::Camera`) complies with every non-deferred
//! convention documented in `docs/current/CAMERA_CONVENTIONS.md`. Failure
//! of any test in this file means a convention violation — either the code
//! drifted away from the documented convention, or the convention itself
//! needs revision (an Andrew design call, not an autonomous test
//! relaxation).
//!
//! Run with:
//!   cargo test --tests -p astraweave-render camera_conventions
//!
//! Section references in test names and docstrings cite
//! `docs/current/CAMERA_CONVENTIONS.md` §X.

use astraweave_camera::FreeFly as Camera;
use glam::{Mat4, Vec3, Vec4};

/// Reference camera with known fields for matrix comparisons.
fn reference_camera() -> Camera {
    Camera {
        position: Vec3::new(1.0, 2.0, 3.0),
        yaw: 0.7,
        pitch: 0.2,
        fovy: 60_f32.to_radians(),
        aspect: 16.0 / 9.0,
        znear: 0.1,
        zfar: 100.0,
    }
}

// ─────────────────────────────────────────────────────────────────────────
// §2.1 — FOV semantics: vertical FOV in radians, field name `fovy`
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn fovy_stores_radians() {
    // CAMERA_CONVENTIONS.md §2.1: `fovy` field stores radians. The
    // projection matrix construction passes `self.fovy` directly to
    // `Mat4::perspective_rh`, which itself expects radians. If a future
    // change wraps `fovy` in `.to_radians()` (storing degrees and
    // converting at projection time), the resulting matrix would diverge
    // from a manual construction that already used radians.
    let cam = Camera {
        fovy: 60_f32.to_radians(),
        ..reference_camera()
    };
    let cam_proj = cam.proj_matrix();
    let expected = Mat4::perspective_rh(
        60_f32.to_radians(),
        cam.aspect.max(0.01),
        cam.znear,
        cam.zfar,
    );
    assert_eq!(
        cam_proj, expected,
        "Camera::proj_matrix must consume fovy as radians (not degrees). \
         If fovy stored degrees, the manual perspective_rh with radians \
         would diverge. See CAMERA_CONVENTIONS.md §2.1."
    );
}

// ─────────────────────────────────────────────────────────────────────────
// §2.2 + §2.6 — Near/far plane handling: wgpu [0, 1] depth range via
// Mat4::perspective_rh (not perspective_rh_gl which gives [-1, 1]).
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn near_far_use_wgpu_zero_to_one_depth() {
    // CAMERA_CONVENTIONS.md §2.2 + §2.6: wgpu's depth convention is
    // [0, 1]. `Mat4::perspective_rh` maps view-space z = -near to NDC
    // z = 0 and view-space z = -far to NDC z = 1. Using
    // `perspective_rh_gl` instead would produce [-1, 1] depth and break
    // shadow mapping, depth-based picking, and CSM cascade extraction.
    let cam = Camera {
        fovy: 60_f32.to_radians(),
        aspect: 1.0,
        znear: 1.0,
        zfar: 100.0,
        ..reference_camera()
    };
    let proj = cam.proj_matrix();

    // Point at view-space z = -near should map to NDC z = 0.
    let near_view = Vec4::new(0.0, 0.0, -cam.znear, 1.0);
    let near_clip = proj * near_view;
    let near_ndc_z = near_clip.z / near_clip.w;
    assert!(
        (near_ndc_z - 0.0).abs() < 1e-4,
        "Near plane (view z = -{}) must map to NDC z = 0 under wgpu [0,1] depth; \
         got {}. Indicates use of perspective_rh_gl instead of perspective_rh. \
         See CAMERA_CONVENTIONS.md §2.2 + §2.6.",
        cam.znear,
        near_ndc_z
    );

    // Point at view-space z = -far should map to NDC z = 1.
    let far_view = Vec4::new(0.0, 0.0, -cam.zfar, 1.0);
    let far_clip = proj * far_view;
    let far_ndc_z = far_clip.z / far_clip.w;
    assert!(
        (far_ndc_z - 1.0).abs() < 1e-4,
        "Far plane (view z = -{}) must map to NDC z = 1 under wgpu [0,1] depth; \
         got {}. Indicates use of perspective_rh_gl instead of perspective_rh. \
         See CAMERA_CONVENTIONS.md §2.2 + §2.6.",
        cam.zfar,
        far_ndc_z
    );
}

// ─────────────────────────────────────────────────────────────────────────
// §2.3 — Aspect ratio handling: clamped via .max(0.01) at projection
// construction; zero or NaN aspect must not propagate to NaN matrix.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn aspect_floored_at_projection() {
    // CAMERA_CONVENTIONS.md §2.3: aspect is clamped to .max(0.01) at
    // projection construction time as defense-in-depth against
    // divide-by-zero from a zero-height window. A future change that
    // removes the .max(0.01) guard would let aspect = 0.0 propagate to
    // NaN in the matrix.
    let cam = Camera {
        aspect: 0.0,
        ..reference_camera()
    };
    let proj = cam.proj_matrix();
    assert!(
        !proj.is_nan(),
        "Camera::proj_matrix must clamp aspect via .max(0.01) so aspect = 0.0 \
         does not propagate NaN. See CAMERA_CONVENTIONS.md §2.3."
    );

    // NaN aspect should also be tolerated (.max(0.01) of NaN returns 0.01
    // in Rust's f32::max semantics, since NaN comparisons return false).
    let cam_nan = Camera {
        aspect: f32::NAN,
        ..reference_camera()
    };
    let proj_nan = cam_nan.proj_matrix();
    assert!(
        !proj_nan.is_nan(),
        "Camera::proj_matrix must tolerate aspect = NaN via .max(0.01) clamp. \
         See CAMERA_CONVENTIONS.md §2.3."
    );
}

// ─────────────────────────────────────────────────────────────────────────
// §2.4 — Coordinate handedness: right-handed, +Y up. View matrix
// construction must use Vec3::Y (positive), never -Vec3::Y.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn up_vector_is_positive_y() {
    // CAMERA_CONVENTIONS.md §2.4: view matrix uses Vec3::Y up. The
    // `-Vec3::Y` bug ("chunk-aligned rectangular voids in terrain") was
    // fixed in commit df7649287; tombstone comment at
    // astraweave-render/src/camera.rs:17-20.
    let cam = reference_camera();
    let cam_view = cam.view_matrix();
    let dir = Camera::dir(cam.yaw, cam.pitch);
    let expected = Mat4::look_to_rh(cam.position, dir, Vec3::Y);
    assert_eq!(
        cam_view, expected,
        "Camera::view_matrix must construct with Vec3::Y up. If -Vec3::Y is \
         used instead, the matrix differs and rendering produces inverted/voided \
         geometry. See CAMERA_CONVENTIONS.md §2.4 + astraweave-render/src/camera.rs:17-20 \
         tombstone."
    );
}

#[test]
fn negative_y_up_produces_different_view() {
    // CAMERA_CONVENTIONS.md §2.4 negative-test discriminator: this test
    // proves that the up-vector check above actually discriminates. If
    // we accidentally wrote `Mat4::look_to_rh(pos, dir, -Vec3::Y)` here,
    // we should get a different matrix than the canonical one.
    let cam = reference_camera();
    let canonical = cam.view_matrix();

    let dir = Camera::dir(cam.yaw, cam.pitch);
    let wrong_up = Mat4::look_to_rh(cam.position, dir, -Vec3::Y);
    assert_ne!(
        canonical, wrong_up,
        "Discriminator: -Vec3::Y up must produce a structurally different view \
         matrix from canonical +Vec3::Y up. If equal, our up-vector test is \
         vacuous. See CAMERA_CONVENTIONS.md §2.4."
    );
}

// ─────────────────────────────────────────────────────────────────────────
// §2.5 — View matrix construction style: both look_to_rh and look_at_rh
// acceptable; produce equivalent matrices for equivalent inputs.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn look_to_and_look_at_styles_equivalent() {
    // CAMERA_CONVENTIONS.md §2.5: glam's look_to_rh(eye, dir, up) and
    // look_at_rh(eye, eye + dir, up) produce identical matrices. This
    // permits FreeFly producers (direction-based) and Orbit producers
    // (target-based) to use whichever is more natural.
    let eye = Vec3::new(1.0, 2.0, 3.0);
    let dir = Vec3::new(0.5, -0.3, 0.8).normalize();
    let up = Vec3::Y;

    let via_look_to = Mat4::look_to_rh(eye, dir, up);
    let via_look_at = Mat4::look_at_rh(eye, eye + dir, up);

    // Compare element-wise with a small epsilon for floating-point noise.
    let cols_to = via_look_to.to_cols_array();
    let cols_at = via_look_at.to_cols_array();
    for (i, (a, b)) in cols_to.iter().zip(cols_at.iter()).enumerate() {
        assert!(
            (a - b).abs() < 1e-5,
            "look_to_rh and look_at_rh must produce equivalent matrices for \
             equivalent inputs; element {} differs: look_to={}, look_at={}. \
             See CAMERA_CONVENTIONS.md §2.5.",
            i,
            a,
            b
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// §2.8 — Yaw=0, pitch=0 forward direction: +X.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn yaw_zero_pitch_zero_forward_is_positive_x() {
    // CAMERA_CONVENTIONS.md §2.8: canonical direction at yaw=0,pitch=0 is
    // Vec3::X = (1, 0, 0). This is the engine's convention since the
    // df7649287 fix. Alternative conventions (-Z forward per Bevy/glTF,
    // -X forward per orbit-offset semantic) must convert at their
    // boundary.
    let dir = Camera::dir(0.0, 0.0);
    let delta = (dir - Vec3::X).length();
    assert!(
        delta < 1e-5,
        "Camera::dir(0.0, 0.0) must equal Vec3::X. Got {:?}, distance from \
         +X = {}. See CAMERA_CONVENTIONS.md §2.8.",
        dir,
        delta
    );
}

// ─────────────────────────────────────────────────────────────────────────
// §2.4 bench-mock-fix verification (C.1 Deliverable C)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn bench_mock_camera_uses_canonical_up_vector() {
    // CAMERA_CONVENTIONS.md §2.4: the bench mock at
    // astraweave-render/benches/camera_primitives_instancing.rs is a copy
    // of engine Camera's API for benchmarking. Before C.1 Deliverable C,
    // its view_matrix used `Mat4::look_to_rh(pos, dir, -Vec3::Y)` — the
    // pre-df7649287 bug. After the fix, it must use Vec3::Y.
    //
    // This test reads the bench source at compile time via include_str!
    // and asserts the canonical convention in the view_matrix body.
    // A future regression that reintroduces `-Vec3::Y` (or copy-pastes
    // from a stale source) fails this test.
    let bench_source = include_str!("../benches/camera_primitives_instancing.rs");

    // Locate the view_matrix function body. The bench mock's pattern is:
    //     pub fn view_matrix(&self) -> Mat4 {
    //         let dir = Self::dir(self.yaw, self.pitch);
    //         Mat4::look_to_rh(self.position, dir, <UP_VECTOR>)
    //     }
    let view_matrix_idx = bench_source
        .find("pub fn view_matrix")
        .expect("bench mock at astraweave-render/benches/camera_primitives_instancing.rs \
                 must define a view_matrix function for this test to verify. \
                 If the bench file was renamed or the function deleted, update \
                 this test or CAMERA_CONVENTIONS.md §3 row for #29.");
    let after_view_matrix = &bench_source[view_matrix_idx..];

    // Take the function body — up to the next `fn ` declaration.
    let next_fn_idx = after_view_matrix[1..]
        .find("\n    pub fn ")
        .or_else(|| after_view_matrix[1..].find("\nimpl "))
        .unwrap_or(after_view_matrix.len() - 1);
    let view_matrix_body = &after_view_matrix[..=next_fn_idx];

    assert!(
        !view_matrix_body.contains("-Vec3::Y"),
        "Bench mock at astraweave-render/benches/camera_primitives_instancing.rs \
         view_matrix must NOT use `-Vec3::Y` (the pre-df7649287 bug — causes \
         chunk-aligned rectangular voids in terrain). See CAMERA_CONVENTIONS.md \
         §2.4 and the tombstone at astraweave-render/src/camera.rs:17-20.\n\n\
         view_matrix body found:\n{}",
        view_matrix_body
    );

    assert!(
        view_matrix_body.contains("Vec3::Y"),
        "Bench mock at astraweave-render/benches/camera_primitives_instancing.rs \
         view_matrix must use `Vec3::Y` up. See CAMERA_CONVENTIONS.md §2.4.\n\n\
         view_matrix body found:\n{}",
        view_matrix_body
    );
}
