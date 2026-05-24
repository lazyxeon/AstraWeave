//! OrbitCamera `CameraProducer` contract tests (Unified Camera sub-phase C.4).
//!
//! Verifies that `OrbitCamera` correctly implements the `CameraProducer`
//! trait from `astraweave-camera`, per `CAMERA_CONVENTIONS.md` §2.9. These
//! are the producer-contract closure proofs for C.4 — analogous to the
//! `astraweave-camera/tests/canonical_types.rs` tests that anchor `FreeFly`.
//!
//! The five tests verify:
//!
//! 1. **Trait implementation** (compile-time): `OrbitCamera: CameraProducer`
//!    is satisfiable; the trait bound is structural and stays stable.
//! 2. **`RenderView::position` source-of-truth**: equals
//!    `OrbitCamera::position()`. Anchors C.4's design Decision 2 (reuse
//!    the existing position method — no inline spherical-to-cartesian).
//! 3. **`RenderView::view` world-relative**: equals
//!    `OrbitCamera::view_matrix()`. Anchors that the trait method is the
//!    world-relative variant (per C.4 Decision 1).
//! 4. **`RenderView::view` camera-relative**: equals
//!    `OrbitCamera::view_matrix_relative()` when the camera-relative
//!    concrete method is used. Anchors that the concrete-only method is
//!    the camera-relative variant (per C.4 Decision 1).
//! 5. **FOV boundary conversion**: `OrbitCamera::fov` stores degrees per
//!    the editor's historical convention; the `RenderView::fovy` returned
//!    by `to_render_view` equals `fov.to_radians()`. Anchors C.4's design
//!    Decision 3 (field stays as degrees; conversion happens at the
//!    producer boundary; structural rename deferred to C.4.B).

use astraweave_camera::CameraProducer;
use aw_editor_lib::viewport::OrbitCamera;
use glam::Vec3;

/// Fixture: an OrbitCamera at a known state for deterministic assertions.
/// Matches the parity harness's modest-position fixture (focal_point at
/// origin, distance=25, yaw=π/4, pitch=π/6) — the campaign-wide reference
/// fixture for camera-related tests.
fn fixture_camera() -> OrbitCamera {
    OrbitCamera::new(
        Vec3::ZERO,
        25.0,
        std::f32::consts::PI / 4.0,
        std::f32::consts::PI / 6.0,
    )
}

/// Test 1 — trait implementation (compile-time check).
///
/// If this function compiles, `OrbitCamera: CameraProducer` holds. The
/// `_dyn` binding forces the trait to be object-safe at the call site
/// (the trait *is* object-safe per its single-method design, but pinning
/// it in a test makes regressions explicit).
#[test]
fn orbit_camera_implements_camera_producer() {
    let camera = fixture_camera();
    let _dyn: &dyn CameraProducer = &camera;
    // Produce a RenderView via the trait method — if the impl is absent
    // or the signature drifts, this fails to compile.
    let _view = CameraProducer::to_render_view(&camera);
}

/// Test 2 — `RenderView::position` equals `OrbitCamera::position()`.
///
/// Anchors Decision 2: the trait impl reuses the existing position
/// method rather than inlining the spherical-to-cartesian math. A future
/// change to either path is caught here.
#[test]
fn orbit_camera_to_render_view_position_matches_position_method() {
    let camera = fixture_camera();
    let view = camera.to_render_view();
    let expected = camera.position();
    assert_eq!(
        view.position, expected,
        "RenderView::position must equal OrbitCamera::position() (Decision 2 — single source of truth)"
    );
}

/// Test 3 — `RenderView::view` (from trait method) equals
/// `OrbitCamera::view_matrix()` (world-space).
///
/// Anchors Decision 1: the trait method is the world-relative variant.
#[test]
fn orbit_camera_to_render_view_uses_world_view_matrix() {
    let camera = fixture_camera();
    let view = camera.to_render_view();
    let expected = camera.view_matrix();
    assert_eq!(
        view.view, expected,
        "trait method `to_render_view` must use the world-space view matrix (Decision 1)"
    );
}

/// Test 4 — `RenderView::view` (from concrete-only method) equals
/// `OrbitCamera::view_matrix_relative()` (camera-relative).
///
/// Anchors Decision 1: the concrete-only `to_render_view_camera_relative`
/// is the camera-relative variant, mirroring FreeFly's pattern from C.3.A.
#[test]
fn orbit_camera_to_render_view_camera_relative_uses_relative_view_matrix() {
    let camera = fixture_camera();
    let view = camera.to_render_view_camera_relative();
    let expected = camera.view_matrix_relative();
    assert_eq!(
        view.view, expected,
        "concrete method `to_render_view_camera_relative` must use the camera-relative view matrix (Decision 1)"
    );
}

/// Test 5 — FOV boundary conversion: `OrbitCamera::fov` stores degrees;
/// `RenderView::fovy` returned by `to_render_view` stores radians.
///
/// Anchors Decision 3: the structural field stays as `fov: f32` in
/// degrees per the editor's historical convention; the conversion to
/// radians happens at the `Projection`/`RenderView` boundary. The
/// structural rename `fov: degrees` → `fovy: radians` is deferred to
/// sub-phase C.4.B.
///
/// Fixture-default `OrbitCamera::fov` is 60.0 degrees (per
/// `OrbitCamera::default`).
#[test]
fn orbit_camera_fov_converts_degrees_to_radians_at_render_view_boundary() {
    let camera = fixture_camera();
    let view = camera.to_render_view();
    let expected_fovy_radians = 60.0_f32.to_radians();
    assert!(
        (view.fovy - expected_fovy_radians).abs() < 1e-6,
        "RenderView::fovy must be OrbitCamera::fov.to_radians() — got {} radians, expected {} radians (60° converted)",
        view.fovy,
        expected_fovy_radians,
    );
    // Symmetric assertion on the camera-relative variant — same boundary
    // discipline applies regardless of which producer method the caller uses.
    let view_relative = camera.to_render_view_camera_relative();
    assert!(
        (view_relative.fovy - expected_fovy_radians).abs() < 1e-6,
        "to_render_view_camera_relative must apply the same fovy conversion"
    );
}
