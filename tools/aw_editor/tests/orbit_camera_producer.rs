//! OrbitCamera `CameraProducer` contract tests (Unified Camera
//! sub-phases C.4 + C.4.B).
//!
//! Verifies that `OrbitCamera` correctly implements the `CameraProducer`
//! trait from `astraweave-camera`, per `CAMERA_CONVENTIONS.md` §2.9. These
//! are the producer-contract closure proofs for C.4 — analogous to the
//! `astraweave-camera/tests/canonical_types.rs` tests that anchor `FreeFly`.
//!
//! Sub-phase C.4.B extended this file with three new tests anchoring the
//! `fov: degrees` → `fovy: radians` field rename (the deferred field
//! migration from C.4 Decision 3) and one backward-compat test for the
//! serde shadow type's legacy-`fov` handling.
//!
//! The tests verify:
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
//! 5. **FOV producer-boundary value**: `RenderView::fovy` returned by
//!    `to_render_view` equals 60° in radians for the default fixture.
//!    Post-C.4.B, the field already stores radians, so the producer
//!    boundary no longer converts — this test verifies the value is
//!    correct, not the conversion path. Decision 3 of C.4 (field stays
//!    as degrees) is now closed by C.4.B's rename.
//!
//! C.4.B additions:
//!
//! 6. **`fovy` field stores radians** (C.4.B Decision 1): a default
//!    OrbitCamera's `fovy` field equals `60_f32.to_radians()` directly
//!    — no degree storage, no degree → radians conversion at the
//!    producer boundary.
//! 7. **`set_fov(degrees)` API boundary** (C.4.B Decision 1): the
//!    setter takes degrees per the UI convention and stores radians
//!    internally; verifies the boundary conversion happens at the
//!    setter, not at consumer sites.
//! 8. **`fov_degrees()` getter** (C.4.B Decision 1): returns the FOV in
//!    degrees for UI binding read paths.
//! 9. **Backward-compat deserialization** (C.4.B Decision 2): legacy
//!    saved files with `fov: 60.0` (degrees, pre-C.4.B field name)
//!    deserialize into `OrbitCamera.fovy = 60.to_radians()` via the
//!    `OrbitCameraSerde` shadow type's `From` implementation.

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

/// Test 5 — `RenderView::fovy` value: the producer emits the canonical
/// 60° fixture FOV in radians.
///
/// Post-C.4.B the field already stores radians per
/// `CAMERA_CONVENTIONS.md` §2.1; the producer boundary no longer
/// converts (the body of `to_render_view` reads `self.fovy` directly).
/// This test verifies the value is correct; the field-storage units are
/// verified by `fovy_field_stores_radians_post_c_4_b` below.
///
/// Fixture-default `OrbitCamera::fovy` is `60_f32.to_radians()` (per
/// `OrbitCamera::default`).
#[test]
fn orbit_camera_to_render_view_emits_fovy_in_radians() {
    let camera = fixture_camera();
    let view = camera.to_render_view();
    let expected_fovy_radians = 60.0_f32.to_radians();
    assert!(
        (view.fovy - expected_fovy_radians).abs() < 1e-6,
        "RenderView::fovy must be 60° in radians for the default fixture — got {} radians, expected {} radians",
        view.fovy,
        expected_fovy_radians,
    );
    // Symmetric assertion on the camera-relative variant — same FOV
    // value regardless of which producer method the caller uses.
    let view_relative = camera.to_render_view_camera_relative();
    assert!(
        (view_relative.fovy - expected_fovy_radians).abs() < 1e-6,
        "to_render_view_camera_relative must emit the same fovy"
    );
}

// ─── C.4.B additions ────────────────────────────────────────────────────

/// Test 6 (C.4.B) — `OrbitCamera.fovy` field stores radians directly.
///
/// Per `CAMERA_CONVENTIONS.md` §2.1 and the sub-phase C.4.B field
/// rename, `OrbitCamera.fovy` stores radians (not degrees as the
/// pre-C.4.B `fov` field did). A default OrbitCamera's `fovy` equals
/// `60_f32.to_radians()` directly — no conversion needed at any
/// consumer site.
#[test]
fn fovy_field_stores_radians_post_c_4_b() {
    let cam = fixture_camera();
    let expected_radians = 60_f32.to_radians();
    assert!(
        (cam.fovy() - expected_radians).abs() < 1e-6,
        "OrbitCamera.fovy should store radians per CAMERA_CONVENTIONS.md §2.1 (post-C.4.B); \
         got {} radians, expected {} radians",
        cam.fovy(),
        expected_radians,
    );
}

/// Test 7 (C.4.B) — `set_fov(degrees)` API takes degrees per
/// Decision 1's boundary convention.
///
/// The setter accepts a degree value (matching the editor's UI slider
/// boundary), converts internally to radians, and stores in
/// `OrbitCamera.fovy`. This anchors that the API surface stays in
/// degrees while internal storage is canonical radians.
#[test]
fn set_fov_takes_degrees_per_boundary_convention() {
    let mut cam = fixture_camera();
    cam.set_fov(90.0);
    let expected_radians = 90_f32.to_radians();
    assert!(
        (cam.fovy() - expected_radians).abs() < 1e-6,
        "set_fov(90.0) should set fovy to 90° in radians; got {} radians, expected {} radians",
        cam.fovy(),
        expected_radians,
    );
    // Verify the value flows correctly through the producer pipeline.
    let view = cam.to_render_view();
    assert!(
        (view.fovy - expected_radians).abs() < 1e-6,
        "post-setter value must propagate through to_render_view"
    );
}

/// Test 8 (C.4.B) — `fov_degrees()` getter returns degrees per the UI
/// binding read path.
///
/// Symmetric to `set_fov`: the UI reads through `fov_degrees()` and gets
/// degrees, never seeing the internal radian representation.
#[test]
fn fov_degrees_getter_returns_degrees() {
    let mut cam = fixture_camera();
    cam.set_fov(75.0);
    let degrees = cam.fov_degrees();
    assert!(
        (degrees - 75.0).abs() < 1e-4,
        "fov_degrees() should return the FOV in degrees; got {} degrees (radians stored: {})",
        degrees,
        cam.fovy(),
    );
}

/// Test 9 (C.4.B) — backward-compat deserialization: legacy `fov`
/// field (degrees) deserializes into `fovy` (radians) via the
/// `OrbitCameraSerde` shadow type per Decision 2.
///
/// Pre-C.4.B saved `.editor_preferences.json` files have
/// `"fov": 60.0` (degrees). Post-C.4.B, those files must continue to
/// deserialize correctly: the legacy value is interpreted as degrees
/// and converted to radians at deserialization. New files emit `fovy`
/// (radians) directly; legacy files are migrated forward on first save.
#[test]
fn deserializes_legacy_fov_field_as_degrees() {
    let legacy_json = r#"{
        "focal_point": [0.0, 0.0, 0.0],
        "distance": 25.0,
        "yaw": 0.7853982,
        "pitch": 0.5235988,
        "fov": 60.0,
        "aspect": 1.7777778,
        "near": 0.5,
        "far": 5000.0,
        "min_distance": 0.02,
        "max_distance": 20000.0,
        "min_pitch": -1.5607963,
        "max_pitch": 1.5607963,
        "zoom_target": 25.0,
        "focal_point_target": [0.0, 0.0, 0.0],
        "pitch_target": 0.5235988,
        "yaw_target": 0.7853982
    }"#;
    let cam: OrbitCamera =
        serde_json::from_str(legacy_json).expect("legacy fov field should deserialize");
    let expected = 60_f32.to_radians();
    assert!(
        (cam.fovy() - expected).abs() < 1e-6,
        "Legacy fov: 60.0 (degrees) should deserialize to fovy = 60.to_radians(); got {} radians",
        cam.fovy(),
    );
}

/// Test 10 (C.4.B) — canonical `fovy` field deserialization: post-C.4.B
/// saved files use `fovy` (radians) directly; the shadow type prefers
/// the canonical value when both are present (per `From` impl ordering).
#[test]
fn deserializes_canonical_fovy_field_as_radians() {
    let canonical_radians = 75_f32.to_radians();
    let canonical_json = format!(
        r#"{{
            "focal_point": [0.0, 0.0, 0.0],
            "distance": 25.0,
            "yaw": 0.7853982,
            "pitch": 0.5235988,
            "fovy": {},
            "aspect": 1.7777778,
            "near": 0.5,
            "far": 5000.0,
            "min_distance": 0.02,
            "max_distance": 20000.0,
            "min_pitch": -1.5607963,
            "max_pitch": 1.5607963,
            "zoom_target": 25.0,
            "focal_point_target": [0.0, 0.0, 0.0],
            "pitch_target": 0.5235988,
            "yaw_target": 0.7853982
        }}"#,
        canonical_radians
    );
    let cam: OrbitCamera =
        serde_json::from_str(&canonical_json).expect("canonical fovy field should deserialize");
    assert!(
        (cam.fovy() - canonical_radians).abs() < 1e-6,
        "Canonical fovy field should deserialize directly as radians; got {} radians, expected {}",
        cam.fovy(),
        canonical_radians,
    );
}
