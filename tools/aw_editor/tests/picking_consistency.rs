//! OrbitCamera picking-consistency tests (Unified Camera sub-phase C.4).
//!
//! Verifies the post-C.4 fix for the picking-vs-depth VP mismatch
//! identified in C.0 audit §3.2: `ray_from_screen` and
//! `unproject_depth_to_world` produce consistent results at any camera
//! world position (modest or large).
//!
//! The pre-fix divergence was a **float-precision issue**, not a
//! coord-space mismatch: both functions are mathematically equivalent
//! (since clip-space output is invariant across world-space vs
//! camera-relative VP pipelines), but `ray_from_screen` inverted the
//! absolute VP whose translation column grows with `|position()|`,
//! losing precision at large camera world positions. The post-C.4 fix
//! migrates `ray_from_screen` to invert the camera-relative VP (whose
//! translations are near zero) and then translate the origin to world
//! space — matching the precision-stable discipline of
//! `unproject_depth_to_world`.
//!
//! Two tests:
//!
//! 1. **Modest camera position** — focal_point at origin (the parity
//!    harness fixture). Both functions should agree to within tight
//!    float epsilon both pre- and post-fix; this test is a baseline
//!    regression guard.
//! 2. **Large camera position** — focal_point far from origin. This
//!    is where the pre-fix divergence manifested. Post-fix, both
//!    functions must agree to within float epsilon.

use aw_editor_lib::viewport::OrbitCamera;
use glam::Vec3;

const VP_WIDTH: u32 = 1920;
const VP_HEIGHT: u32 = 1080;

/// Build an OrbitCamera with a custom focal_point at modest distance.
fn camera_at(focal_point: Vec3) -> OrbitCamera {
    OrbitCamera::new(
        focal_point,
        25.0,
        std::f32::consts::PI / 4.0,
        std::f32::consts::PI / 6.0,
    )
}

/// At a given pixel, the ray's origin (near plane in world space) must
/// equal `unproject_depth_to_world(px, py, 0.0)`. Both compute the same
/// world-space point from the same NDC at depth=0.
///
/// NDC convention: `ray_from_screen` uses `screen_pos / viewport_size`
/// (no half-pixel offset); `unproject_depth_to_world` uses
/// `(px + 0.5) / vp_width`. To make them produce identical NDC, we feed
/// `ray_from_screen` the pixel-center coordinate `(px + 0.5, py + 0.5)`.
fn assert_picking_consistency_at(camera: &OrbitCamera, px: u32, py: u32) {
    let viewport = egui::vec2(VP_WIDTH as f32, VP_HEIGHT as f32);
    let pixel_center = egui::pos2(px as f32 + 0.5, py as f32 + 0.5);

    let ray = camera.ray_from_screen(pixel_center, viewport);
    let unproj_near = camera.unproject_depth_to_world(
        px as f32,
        py as f32,
        VP_WIDTH as f32,
        VP_HEIGHT as f32,
        0.0, // depth=0 → near plane
    );

    let delta = (ray.origin - unproj_near).length();
    // Picking-consistency budget: 1e-3 world units. The two functions
    // share the same precision-stable inversion path post-C.4; the
    // remaining delta is from intermediate float rounding in
    // `Mat4::inverse` and `project_point3`. 1mm at the near plane is
    // well within picking tolerance.
    assert!(
        delta < 1e-3,
        "ray_from_screen.origin and unproject_depth_to_world(depth=0) must agree at world position {:?}; |delta| = {} world units",
        camera.position(),
        delta,
    );

    // Symmetric assertion at the far plane (depth=1): the world point
    // along the ray at the far plane must equal
    // `unproject_depth_to_world(px, py, 1.0)`. Computed by following the
    // ray's direction to wherever the far plane lies. We use the
    // unprojected far-plane point as the truth; the ray's NDC-far point
    // is `inv_vp_rel * (ndc_x, ndc_y, 1.0) + position`, which is
    // equivalent to what `unproject_depth_to_world(_, _, 1.0)` computes.
    let unproj_far = camera.unproject_depth_to_world(
        px as f32,
        py as f32,
        VP_WIDTH as f32,
        VP_HEIGHT as f32,
        1.0, // depth=1 → far plane
    );

    // Verify `unproj_far` lies along the ray (within tight float
    // epsilon). The signed distance from the ray's line is
    // `||(p - origin) - direction · ((p - origin) · direction)||`.
    let to_far = unproj_far - ray.origin;
    let along = ray.direction.dot(to_far);
    let perp = to_far - ray.direction * along;
    let perpendicular_distance = perp.length();
    // Relative tolerance: at large camera positions, the far-plane
    // world point's coordinates are large; an absolute epsilon would
    // be over-strict. Tolerance scales with the magnitude of `to_far`.
    let tolerance = (to_far.length() * 1e-4).max(1e-3);
    assert!(
        perpendicular_distance < tolerance,
        "unproject_depth_to_world(depth=1) must lie on the ray from ray_from_screen at world position {:?}; perpendicular = {} world units (tolerance {})",
        camera.position(),
        perpendicular_distance,
        tolerance,
    );
}

/// Test 1 — modest camera position (parity harness fixture).
///
/// focal_point at origin, distance=25. Baseline regression guard:
/// both pre- and post-C.4 the functions agreed here within tight
/// float epsilon. This test catches a future change that breaks
/// the modest-position case.
#[test]
fn ray_from_screen_and_unproject_depth_to_world_agree_at_modest_camera_position() {
    let camera = camera_at(Vec3::ZERO);
    let center_x = VP_WIDTH / 2;
    let center_y = VP_HEIGHT / 2;
    assert_picking_consistency_at(&camera, center_x, center_y);

    // A few non-center pixels exercise off-axis NDC mapping.
    assert_picking_consistency_at(&camera, VP_WIDTH / 4, VP_HEIGHT / 4);
    assert_picking_consistency_at(&camera, (VP_WIDTH * 3) / 4, (VP_HEIGHT * 3) / 4);
}

/// Test 2 — large camera position (post-C.4 fix verification).
///
/// focal_point far from origin (≈14km diagonal). Pre-C.4,
/// `ray_from_screen` inverted the absolute VP whose translation
/// column carries `|cam_pos|`-magnitude entries, losing precision
/// here. Post-C.4, both functions invert the camera-relative VP
/// (precision-stable) and add `position()`, so they agree.
///
/// If this test fails on the central pixel, the C.4 fix has
/// regressed (e.g., a future "simplification" reintroduced the
/// absolute-VP path in `ray_from_screen`).
#[test]
fn ray_from_screen_and_unproject_depth_to_world_agree_at_large_camera_position() {
    let camera = camera_at(Vec3::new(10_000.0, 0.0, 10_000.0));
    let center_x = VP_WIDTH / 2;
    let center_y = VP_HEIGHT / 2;
    assert_picking_consistency_at(&camera, center_x, center_y);
    assert_picking_consistency_at(&camera, VP_WIDTH / 4, VP_HEIGHT / 4);
}
